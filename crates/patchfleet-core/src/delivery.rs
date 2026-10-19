//! Explicit same-Project, hash-pinned ZIP delivery; no new persisted artifact entity.
#[cfg(unix)]
use super::digest;
use super::{bounded, exact, hash, id};
#[cfg(unix)]
use crate::{conversation, work::text};
use crate::{creation, Error, Host, Result};
#[cfg(unix)]
use serde_json::json;
use serde_json::Value;
#[cfg(unix)]
use sha2::{Digest, Sha256};
use std::collections::HashSet;
#[cfg(unix)]
use std::io::Write;

#[cfg(unix)]
const FILE_BYTES: u64 = 25 * 1024 * 1024;
#[cfg(unix)]
const TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const FILE_COUNT: usize = 32;

fn invalid(code: &str) -> Error {
    Error(code.into())
}
fn limit() -> Error {
    invalid("PROJECT_DELIVERY_LIMIT_EXCEEDED")
}
pub(super) fn validate_request(p: &Value) -> Result<()> {
    id(&p["projectId"], "project:")?;
    bounded(&p["targetPath"], 32768)?;
    let files = p["files"]
        .as_array()
        .filter(|v| !v.is_empty() && v.len() <= FILE_COUNT)
        .ok_or_else(limit)?;
    let mut identities = HashSet::new();
    for file in files {
        exact(file, &["workItemId", "path", "expectedSha256"], &[])?;
        let work = id(&file["workItemId"], "work:")?;
        let path = bounded(&file["path"], 2048)?;
        creation::relative_result(path)?;
        hash(&file["expectedSha256"])?;
        if !identities.insert((work, path)) {
            return Err(invalid("PROJECT_DELIVERY_DUPLICATE"));
        }
    }
    Ok(())
}

pub(super) fn export(host: &Host, p: &Value) -> Result<Value> {
    validate_request(p)?;
    #[cfg(not(unix))]
    {
        let _ = host;
        Err(invalid("PROJECT_RESULTS_UNSUPPORTED_PLATFORM"))
    }
    #[cfg(unix)]
    {
        let mut entries = Vec::new();
        let mut files = Vec::new();
        let mut total = 0u64;
        for (index, selection) in p["files"].as_array().unwrap().iter().enumerate() {
            let (root, _) = host.creation_scope(
                &json!({"projectId":p["projectId"],"workItemId":selection["workItemId"]}),
            )?;
            let path = creation::relative_result(text(selection, "path")?)?;
            let bytes =
                conversation::copied_bytes(&root, &path, FILE_BYTES.min(TOTAL_BYTES - total))
                    .map_err(|error| {
                        if error.0 == "PROJECT_INPUT_LIMIT_EXCEEDED" {
                            limit()
                        } else {
                            error
                        }
                    })?;
            let sha = digest(&bytes);
            if selection["expectedSha256"] != sha {
                return Err(invalid("PROJECT_ARTIFACT_CHANGED"));
            }
            total += bytes.len() as u64;
            let name: String = path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .chars()
                .take(80)
                .map(|c| {
                    if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let alias = format!("files/{:02}-{name}", index + 1);
            files.push(json!({"workItemId":selection["workItemId"],"path":selection["path"],"archivePath":alias,"sha256":sha,"bytes":bytes.len()}));
            entries.push((alias, bytes));
        }
        let manifest = json!({"schemaVersion":1,"projectId":p["projectId"],"files":files});
        entries.push((
            "manifest.json".into(),
            serde_json::to_vec_pretty(&manifest)?,
        ));
        zip_size(&entries)?;
        // All source bytes and hashes have been checked before the target is created.
        let mut destination = host.checked_export_destination(text(p, "targetPath")?)?;
        let (bytes, sha) = write_zip(&mut destination, &entries)?;
        // Retain partial writes on error; this public path may already name a replacement.
        destination.flush()?;
        destination.sync_all()?;
        Ok(
            json!({"reasonCode":"PROJECT_DELIVERY_EXPORTED","projectId":p["projectId"],"fileCount":files.len(),"bytes":bytes,"sourceBytes":total,"archiveSha256":sha,"files":files}),
        )
    }
}

#[cfg(unix)]
fn zip_size(entries: &[(String, Vec<u8>)]) -> Result<u64> {
    if entries.len() > u16::MAX as usize {
        return Err(limit());
    }
    let mut size = 22u64;
    for (name, bytes) in entries {
        if name.len() > u16::MAX as usize || bytes.len() > u32::MAX as usize {
            return Err(limit());
        }
        size = size
            .checked_add(30 + 46 + 2 * name.len() as u64 + bytes.len() as u64)
            .ok_or_else(limit)?;
    }
    if size > u32::MAX as u64 {
        return Err(limit());
    }
    Ok(size)
}

#[cfg(unix)]
const fn crc_table() -> [u32; 256] {
    let mut table = [0; 256];
    let mut index = 0;
    while index < 256 {
        let mut crc = index as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[index] = crc;
        index += 1;
    }
    table
}
#[cfg(unix)]
fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = crc_table();
    !bytes.iter().fold(!0u32, |crc, byte| {
        (crc >> 8) ^ TABLE[((crc ^ *byte as u32) & 255) as usize]
    })
}
#[cfg(unix)]
fn u16le(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
#[cfg(unix)]
fn u32le(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

#[cfg(unix)]
fn write_zip(mut destination: impl Write, entries: &[(String, Vec<u8>)]) -> Result<(u64, String)> {
    zip_size(entries)?;
    let mut offset = 0u32;
    let mut sha = Sha256::new();
    let mut central = Vec::new();
    let mut write = |data: &[u8]| -> Result<()> {
        destination.write_all(data)?;
        sha.update(data);
        Ok(())
    };
    for (name, bytes) in entries {
        let crc = crc32(bytes);
        let mut local = Vec::with_capacity(30 + name.len());
        u32le(&mut local, 0x0403_4b50);
        for value in [20, 0x0800, 0, 0, 33] {
            u16le(&mut local, value);
        }
        u32le(&mut local, crc);
        u32le(&mut local, bytes.len() as u32);
        u32le(&mut local, bytes.len() as u32);
        u16le(&mut local, name.len() as u16);
        u16le(&mut local, 0);
        local.extend_from_slice(name.as_bytes());
        write(&local)?;
        write(bytes)?;

        u32le(&mut central, 0x0201_4b50);
        for value in [20, 20, 0x0800, 0, 0, 33] {
            u16le(&mut central, value);
        }
        u32le(&mut central, crc);
        u32le(&mut central, bytes.len() as u32);
        u32le(&mut central, bytes.len() as u32);
        u16le(&mut central, name.len() as u16);
        for _ in 0..4 {
            u16le(&mut central, 0);
        }
        u32le(&mut central, 0);
        u32le(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
        offset += local.len() as u32 + bytes.len() as u32;
    }
    let mut end = Vec::with_capacity(22);
    u32le(&mut end, 0x0605_4b50);
    for value in [0, 0, entries.len() as u16, entries.len() as u16] {
        u16le(&mut end, value);
    }
    u32le(&mut end, central.len() as u32);
    u32le(&mut end, offset);
    u16le(&mut end, 0);
    write(&central)?;
    write(&end)?;
    Ok((
        offset as u64 + central.len() as u64 + end.len() as u64,
        format!("{:x}", sha.finalize()),
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf, sync::Arc};

    fn command(mut p: Value) -> Value {
        p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
        let now = chrono::Utc::now();
        p["createdAt"] = json!(now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        p["expiresAt"] = json!((now + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        p
    }
    async fn job(host: &Host, project: &Value) -> (Value, PathBuf) {
        let revision = host.store.runtime_projections().unwrap()["projectMemory"]["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["projectId"] == *project)
            .unwrap()["revision"]
            .clone();
        let result = host.action("project_work_enqueue", command(json!({"projectId":project,"expectedProjectRevision":revision,"abilityId":"D1","title":"Delivery outputs","instruction":"Prepare actual useful outputs. PRIVATE_DELIVERY_PROMPT"}))).await.unwrap();
        let id = result["workItemId"].clone();
        let (root, _) = host
            .creation_scope(&json!({"projectId":project,"workItemId":id}))
            .unwrap();
        (id, root)
    }
    async fn fixture() -> (tempfile::TempDir, Arc<Host>, Value, Value, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let host = Host::open(tmp.path().join("data")).unwrap();
        let project = host
            .action(
                "project_create",
                command(json!({"name":"Delivery Project"})),
            )
            .await
            .unwrap()["projectId"]
            .clone();
        let (work, root) = job(&host, &project).await;
        (tmp, host, project, work, root)
    }
    fn selected(work: &Value, path: &str, bytes: &[u8]) -> Value {
        json!({"workItemId":work,"path":path,"expectedSha256":digest(bytes)})
    }
    fn le16(bytes: &[u8], position: usize) -> usize {
        u16::from_le_bytes(bytes[position..position + 2].try_into().unwrap()) as usize
    }
    fn le32(bytes: &[u8], position: usize) -> usize {
        u32::from_le_bytes(bytes[position..position + 4].try_into().unwrap()) as usize
    }
    fn read_store_zip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
        let end = bytes.len() - 22;
        assert_eq!(&bytes[end..end + 4], b"PK\x05\x06");
        assert_eq!(le16(bytes, end + 4), 0);
        assert_eq!(le16(bytes, end + 6), 0);
        let count = le16(bytes, end + 10);
        assert_eq!(le16(bytes, end + 8), count);
        let mut central = le32(bytes, end + 16);
        assert_eq!(central + le32(bytes, end + 12), end);
        let mut items = Vec::new();
        for _ in 0..count {
            assert_eq!(&bytes[central..central + 4], b"PK\x01\x02");
            assert_eq!(le16(bytes, central + 8), 0x0800);
            assert_eq!(le16(bytes, central + 10), 0);
            let local = le32(bytes, central + 42);
            assert_eq!(&bytes[local..local + 4], b"PK\x03\x04");
            let name_length = le16(bytes, central + 28);
            let name = String::from_utf8(bytes[central + 46..central + 46 + name_length].to_vec())
                .unwrap();
            assert_eq!(le16(bytes, local + 26), name_length);
            assert_eq!(
                &bytes[local + 30..local + 30 + name_length],
                name.as_bytes()
            );
            let size = le32(bytes, central + 24);
            assert_eq!(le32(bytes, central + 20), size);
            assert_eq!(le32(bytes, local + 22), size);
            let data = local + 30 + name_length + le16(bytes, local + 28);
            let content = bytes[data..data + size].to_vec();
            assert_eq!(le32(bytes, central + 16), crc32(&content) as usize);
            assert_eq!(le32(bytes, local + 14), crc32(&content) as usize);
            items.push((name, content));
            central += 46 + name_length + le16(bytes, central + 30) + le16(bytes, central + 32);
        }
        assert_eq!(central, end);
        items
    }
    #[test]
    fn store_zip_has_standard_crc_offsets_and_utf8_names() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        let entries = vec![
            (
                "files/01-Özet_猫.md".into(),
                b"Actual original bytes\0\xff".to_vec(),
            ),
            ("manifest.json".into(), b"{}".to_vec()),
        ];
        let mut output = Vec::new();
        let (bytes, hash) = write_zip(&mut output, &entries).unwrap();
        assert_eq!(bytes, output.len() as u64);
        assert_eq!(bytes, zip_size(&entries).unwrap());
        assert_eq!(hash, digest(&output));
        assert_eq!(read_store_zip(&output), entries);
        let invalid = vec![("x".repeat(u16::MAX as usize + 1), vec![])];
        let mut output = Vec::new();
        assert!(write_zip(&mut output, &invalid).is_err());
        assert!(output.is_empty());
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn host_exports_real_collision_free_bytes_manifest_and_replays_exact_receipt() {
        let (tmp, host, project, work, root) = fixture().await;
        fs::create_dir(root.join("outputs/charts")).unwrap();
        fs::write(root.join("outputs/Özet_猫.md"), b"Original document").unwrap();
        fs::write(
            root.join("outputs/charts/Özet_猫.md"),
            b"Different chart bytes",
        )
        .unwrap();
        let (second_work, second_root) = job(&host, &project).await;
        fs::write(second_root.join("outputs/Özet_猫.md"), b"Another mission").unwrap();
        let files = json!([
            selected(&work, "outputs/Özet_猫.md", b"Original document"),
            selected(&work, "outputs/charts/Özet_猫.md", b"Different chart bytes"),
            selected(&second_work, "outputs/Özet_猫.md", b"Another mission")
        ]);
        let target = tmp.path().join("delivery.zip");
        let request = command(json!({"projectId":project,"files":files,"targetPath":target}));
        let result = host
            .action("project_delivery_export", request.clone())
            .await
            .unwrap();
        assert_eq!(result["reasonCode"], "PROJECT_DELIVERY_EXPORTED");
        assert_eq!(result["fileCount"], 3);
        let bytes = fs::read(&target).unwrap();
        assert_eq!(result["bytes"], bytes.len());
        assert_eq!(result["archiveSha256"], digest(&bytes));
        let entries = read_store_zip(&bytes);
        assert_eq!(entries.len(), 4);
        let manifest: Value = serde_json::from_slice(&entries[3].1).unwrap();
        assert_eq!(manifest["schemaVersion"], 1);
        assert_eq!(manifest["projectId"], project);
        let aliases: HashSet<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["archivePath"].as_str().unwrap())
            .collect();
        assert_eq!(aliases.len(), 3);
        for (index, file) in manifest["files"].as_array().unwrap().iter().enumerate() {
            assert_eq!(file["workItemId"], files[index]["workItemId"]);
            assert_eq!(file["path"], files[index]["path"]);
            let (_, content) = entries
                .iter()
                .find(|(name, _)| *name == file["archivePath"])
                .unwrap();
            assert_eq!(file["sha256"], digest(content));
            assert_eq!(file["bytes"], content.len());
        }
        let manifest_text = serde_json::to_string(&manifest).unwrap();
        assert!(!manifest_text.contains("PRIVATE_DELIVERY_PROMPT"));
        assert!(!manifest_text.contains(tmp.path().to_str().unwrap()));
        assert_eq!(
            fs::read(root.join("outputs/Özet_猫.md")).unwrap(),
            b"Original document"
        );
        fs::write(
            root.join("outputs/Özet_猫.md"),
            b"Changed after acknowledged export",
        )
        .unwrap();
        assert_eq!(
            host.action("project_delivery_export", request.clone())
                .await
                .unwrap(),
            result
        );
        assert_eq!(fs::read(&target).unwrap(), bytes);
        let mut changed = request.clone();
        changed["targetPath"] = json!(tmp.path().join("changed.zip"));
        assert_eq!(
            host.action("project_delivery_export", changed)
                .await
                .unwrap_err()
                .0,
            "IDEMPOTENCY_KEY_REUSED"
        );
        host.shutdown().await;
        drop(host);
        let reopened = Host::open(tmp.path().join("data")).unwrap();
        assert_eq!(
            reopened
                .action("project_delivery_export", request)
                .await
                .unwrap(),
            result
        );
        assert_eq!(fs::read(&target).unwrap(), bytes);
        reopened.shutdown().await;
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn invalid_sources_scopes_and_selections_never_create_destination() {
        let (tmp, host, project, work, root) = fixture().await;
        fs::write(root.join("outputs/source.md"), b"Original").unwrap();
        let valid = selected(&work, "outputs/source.md", b"Original");
        let foreign = host
            .action("project_create", command(json!({"name":"Foreign"})))
            .await
            .unwrap()["projectId"]
            .clone();
        let (foreign_work, foreign_root) = job(&host, &foreign).await;
        fs::write(foreign_root.join("outputs/source.md"), b"Original").unwrap();
        fs::write(tmp.path().join("outside.md"), b"Original").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("outside.md"), root.join("outputs/link.md"))
            .unwrap();
        for case in [
            "hash",
            "missing",
            "missing-job",
            "foreign",
            "traversal",
            "symlink",
            "duplicate",
            "different-hash-duplicate",
            "empty",
            "count",
            "extra",
            "absolute",
            "parent-link",
        ] {
            let mut files = vec![valid.clone()];
            match case {
                "hash" => files[0]["expectedSha256"] = json!("0".repeat(64)),
                "missing" => files[0]["path"] = json!("outputs/absent.md"),
                "missing-job" => {
                    files[0]["workItemId"] = json!(format!("work:{}", uuid::Uuid::new_v4()))
                }
                "foreign" => files[0]["workItemId"] = foreign_work.clone(),
                "traversal" => files[0]["path"] = json!("outputs/../../outside.md"),
                "symlink" => files[0]["path"] = json!("outputs/link.md"),
                "duplicate" => files.push(valid.clone()),
                "different-hash-duplicate" => {
                    files.push(valid.clone());
                    files[1]["expectedSha256"] = json!("0".repeat(64));
                }
                "empty" => files.clear(),
                "count" => files = (0..33).map(|_| valid.clone()).collect(),
                "extra" => files[0]["absolutePath"] = json!("untrusted"),
                "absolute" => files[0]["path"] = json!(tmp.path().join("outside.md")),
                _ => {
                    std::os::unix::fs::symlink(tmp.path(), root.join("outputs/linked-parent"))
                        .unwrap();
                    files[0]["path"] = json!("outputs/linked-parent/outside.md");
                }
            }
            let target = tmp.path().join(format!("{case}.zip"));
            assert!(
                host.action(
                    "project_delivery_export",
                    command(json!({"projectId":project,"files":files,"targetPath":target}))
                )
                .await
                .is_err(),
                "{case}"
            );
            assert!(!target.exists(), "{case}");
        }
        host.shutdown().await;
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn checked_destination_and_limits_preserve_files_without_partial_source_failures() {
        let (tmp, host, project, work, root) = fixture().await;
        fs::write(root.join("outputs/source.md"), b"Original").unwrap();
        let files = json!([selected(&work, "outputs/source.md", b"Original")]);
        let existing = tmp.path().join("existing.zip");
        fs::write(&existing, b"Keep owner bytes").unwrap();
        let link = tmp.path().join("linked.zip");
        std::os::unix::fs::symlink(&existing, &link).unwrap();
        let internal = tmp.path().join("data/forbidden.zip");
        let internal_link = tmp.path().join("internal-link");
        std::os::unix::fs::symlink(tmp.path().join("data"), &internal_link).unwrap();
        for target in [
            existing.clone(),
            link,
            internal.clone(),
            internal_link.join("forbidden.zip"),
            tmp.path().join("missing/folder.zip"),
            PathBuf::from("relative.zip"),
            tmp.path().join("data/../escaped.zip"),
        ] {
            assert!(host
                .action(
                    "project_delivery_export",
                    command(json!({"projectId":project,"files":files,"targetPath":target}))
                )
                .await
                .is_err());
        }
        assert_eq!(fs::read(&existing).unwrap(), b"Keep owner bytes");
        assert!(!internal.exists());
        let large = root.join("outputs/large.bin");
        fs::File::create(&large)
            .unwrap()
            .set_len(FILE_BYTES + 1)
            .unwrap();
        let target = tmp.path().join("large.zip");
        let request = command(
            json!({"projectId":project,"files":[{"workItemId":work,"path":"outputs/large.bin","expectedSha256":"0".repeat(64)}],"targetPath":target}),
        );
        assert_eq!(
            host.action("project_delivery_export", request)
                .await
                .unwrap_err()
                .0,
            "PROJECT_DELIVERY_LIMIT_EXCEEDED"
        );
        assert!(!target.exists());
        let mut aggregate = Vec::new();
        for (index, size) in [FILE_BYTES, FILE_BYTES, 15 * 1024 * 1024]
            .into_iter()
            .enumerate()
        {
            let path = format!("outputs/part-{index}.bin");
            fs::File::create(root.join(&path))
                .unwrap()
                .set_len(size)
                .unwrap();
            let mut sha = Sha256::new();
            let chunk = [0u8; 64 * 1024];
            let mut remaining = size;
            while remaining > 0 {
                let amount = remaining.min(chunk.len() as u64);
                sha.update(&chunk[..amount as usize]);
                remaining -= amount;
            }
            aggregate.push(json!({"workItemId":work,"path":path,"expectedSha256":format!("{:x}",sha.finalize())}));
        }
        let target = tmp.path().join("aggregate.zip");
        assert_eq!(
            host.action(
                "project_delivery_export",
                command(json!({"projectId":project,"files":aggregate,"targetPath":target}))
            )
            .await
            .unwrap_err()
            .0,
            "PROJECT_DELIVERY_LIMIT_EXCEEDED"
        );
        assert!(!target.exists());
        host.shutdown().await;
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn export_crash_without_receipt_is_unknown_and_never_repeated() {
        let (tmp, host, project, work, root) = fixture().await;
        fs::write(root.join("outputs/source.md"), b"Original").unwrap();
        let target = tmp.path().join("crash.zip");
        let request = command(
            json!({"projectId":project,"files":[selected(&work,"outputs/source.md",b"Original")],"targetPath":target}),
        );
        host.action("project_delivery_export", request.clone())
            .await
            .unwrap();
        let bytes = fs::read(&target).unwrap();
        let mut prefix = host.store.read_events().unwrap();
        let outer = prefix.pop().unwrap();
        assert_eq!(outer["type"], "owner.command.receipted");
        assert_eq!(outer["payload"]["commandId"], request["commandId"]);
        host.shutdown().await;
        drop(host);
        fs::write(
            tmp.path().join("data/events.jsonl"),
            prefix.iter().map(|e| format!("{e}\n")).collect::<String>(),
        )
        .unwrap();
        let reopened = Host::open(tmp.path().join("data")).unwrap();
        let result = reopened
            .action("project_delivery_export", request.clone())
            .await
            .unwrap();
        assert_eq!(result["reasonCode"], "OWNER_COMMAND_OUTCOME_UNKNOWN");
        assert_eq!(
            reopened
                .action("project_delivery_export", request)
                .await
                .unwrap(),
            result
        );
        assert_eq!(fs::read(&target).unwrap(), bytes);
        reopened.shutdown().await;
    }
    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "writes a retained local ZIP for independent external-reader inspection"]
    async fn write_delivery_archive_fixture() {
        let (tmp, host, project, work, root) = fixture().await;
        fs::create_dir(root.join("outputs/charts")).unwrap();
        fs::write(root.join("outputs/Özet_猫.md"), b"Original Unicode report").unwrap();
        fs::write(root.join("outputs/charts/Özet_猫.md"), [0, 255, 17, 42]).unwrap();
        let target = tmp.path().join("delivery-fixture.zip");
        let result = host.action("project_delivery_export",command(json!({"projectId":project,"files":[selected(&work,"outputs/Özet_猫.md",b"Original Unicode report"),selected(&work,"outputs/charts/Özet_猫.md",&[0,255,17,42])],"targetPath":target}))).await.unwrap();
        assert_eq!(result["fileCount"], 2);
        host.shutdown().await;
        drop(host);
        let retained = tmp.keep();
        eprintln!(
            "Delivery ZIP fixture: {}",
            retained.join("delivery-fixture.zip").display()
        );
    }
}
