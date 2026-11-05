//! Additional owner reads/imports reuse Studio's pinned resources and production jobs.
use super::*;
#[cfg(test)]
#[path = "studio_capabilities_tests.rs"]
mod tests;

pub(super) fn action_fields(action: &str) -> Option<&'static [&'static str]> {
    Some(match action {
        "project_resource_import_batch" => &["projectId", "expectedProjectRevision", "sourcePaths"],
        "project_resource_read" => &["projectId", "resourceId", "expectedSha256"],
        "project_resource_from_artifact" => &[
            "projectId",
            "expectedProjectRevision",
            "workItemId",
            "path",
            "expectedSha256",
        ],
        "project_resources_from_artifacts" => &["projectId", "expectedProjectRevision", "files"],
        "project_artifact_visual_read" => {
            &["projectId", "workItemId", "path", "expectedSha256", "page"]
        }
        "project_artifact_frame_read" => {
            &["projectId", "workItemId", "path", "expectedSha256", "time"]
        }
        "project_artifact_open" => &["projectId", "workItemId", "path", "expectedSha256"],
        "project_image_engine_set" => &["projectId", "url", "checkpoint"],
        "project_image_engine_remove" => &["projectId"],
        _ => return None,
    })
}
pub(super) fn is_read(action: &str) -> bool {
    matches!(
        action,
        "project_resource_read"
            | "project_artifact_visual_read"
            | "project_artifact_frame_read"
            | "project_artifact_open"
    )
}
pub(super) fn sensitive(action: &str) -> bool {
    matches!(
        action,
        "project_image_engine_set" | "project_image_engine_remove"
    )
}
fn resource_id(seed: &str) -> String {
    let digest = Sha256::digest(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    format!("resource:{}", uuid::Uuid::from_bytes(bytes))
}
fn stage(host: &Host, project: &str, resource_id: &str, name: &str, bytes: &[u8]) -> Result<Value> {
    require(bytes.len() as u64 <= FILE_LIMIT)?;
    let extension = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty() && s.len() <= 12 && s.bytes().all(|b| b.is_ascii_alphanumeric()))
        .unwrap_or("bin")
        .to_ascii_lowercase();
    let file_name = format!("source.{extension}");
    let directory = resource_directory(host.creation_data_dir(), project, resource_id)?;
    store::private_directory(&directory)?;
    if directory.join(&file_name).exists() {
        require(
            conversation::copied_bytes(&directory, Path::new(&file_name), FILE_LIMIT)? == bytes,
        )?;
    } else {
        store::atomic_write(&directory.join(&file_name), bytes)?;
    }
    Ok(
        json!({"resourceId":resource_id,"revision":1,"name":name.chars().take(200).collect::<String>(),"kind":kind(name),"bytes":bytes.len(),"sha256":digest(bytes),"fileName":file_name,"status":"available"}),
    )
}
pub(super) fn validate_event(e: &Value) -> Result<bool> {
    let p = &e["payload"];
    match e["type"].as_str().unwrap_or("") {
        "project_studio.resources_imported" => {
            exact(p, &["projectId", "commandId", "resources"], &[])?;
            id(&p["projectId"], "project:")?;
            id(&p["commandId"], "cmd:")?;
            let resources = p["resources"]
                .as_array()
                .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
            require(!resources.is_empty() && resources.len() <= 12)?;
            let mut ids = std::collections::BTreeSet::new();
            let mut total = 0;
            for r in resources {
                resource(r)?;
                require(
                    r["revision"] == 1
                        && r["status"] == "available"
                        && ids.insert(text(r, "resourceId")?),
                )?;
                total += r["bytes"].as_u64().unwrap();
            }
            require(total <= FILE_LIMIT)?;
        }
        "project_studio.resource_published" => {
            exact(p, &["projectId", "resource", "origin"], &[])?;
            id(&p["projectId"], "project:")?;
            resource(&p["resource"])?;
            let o = &p["origin"];
            exact(
                o,
                &[
                    "runId",
                    "workItemId",
                    "ownerEpoch",
                    "nonce",
                    "requestHash",
                    "source",
                    "sourceSha256",
                ],
                &[],
            )?;
            id(&o["workItemId"], "work:")?;
            for k in ["runId", "ownerEpoch", "nonce"] {
                bounded(&o[k], 128)?;
            }
            hash(&o["requestHash"])?;
            hash(&o["sourceSha256"])?;
            let source = creation::relative_result(bounded(&o["source"], 1024)?)?;
            require(
                source.starts_with("outputs")
                    && p["resource"]["sha256"] == o["sourceSha256"]
                    && p["resource"]["bytes"]
                        .as_u64()
                        .is_some_and(|n| n <= 64 * 1024)
                    && p["resource"]["revision"] == 1
                    && p["resource"]["status"] == "available",
            )?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
pub(super) fn replay(
    events: &[Value],
    e: &Value,
    resources: &mut BTreeMap<String, Value>,
) -> Result<bool> {
    let p = &e["payload"];
    let rows = match e["type"].as_str().unwrap_or("") {
        "project_studio.resources_imported" => {
            let request = events
                .iter()
                .find(|r| {
                    r["type"] == "owner.command.requested"
                        && r["payload"]["commandId"] == p["commandId"]
                })
                .ok_or_else(|| fail("PROJECT_STUDIO_MISSING_AUTHORITY"))?;
            let action = text(&request["payload"], "action")?;
            require(matches!(
                action,
                "project_resource_import_batch"
                    | "project_resource_from_artifact"
                    | "project_resources_from_artifacts"
            ))?;
            let c = owner_proof(events, p, action)?;
            let rows = array(p, "resources");
            if action == "project_resource_import_batch" {
                let paths = c["sourcePaths"]
                    .as_array()
                    .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
                require(paths.len() == rows.len())?;
                for (i, r) in rows.iter().enumerate() {
                    let source = Path::new(bounded(&paths[i], 4096)?);
                    require(source.is_absolute())?;
                    require(
                        r["resourceId"] == resource_id(&format!("{}:{i}", text(p, "commandId")?))
                            && r["name"]
                                == source
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .chars()
                                    .take(200)
                                    .collect::<String>(),
                    )?;
                }
            } else if action == "project_resources_from_artifacts" {
                let files = c["files"]
                    .as_array()
                    .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
                require(files.len() == rows.len())?;
                let memory = crate::project_memory::project(events)?;
                for (i, (file, row)) in files.iter().zip(rows).enumerate() {
                    exact(file, &["workItemId", "path", "expectedSha256"], &[])?;
                    require(
                        row["resourceId"] == resource_id(&format!("{}:{i}", text(p, "commandId")?))
                            && row["sha256"] == file["expectedSha256"],
                    )?;
                    require(array(&memory, "bindings").iter().any(|b| {
                        b["subjectKind"] == "work"
                            && b["subjectId"] == file["workItemId"]
                            && b["projectId"] == p["projectId"]
                    }))?;
                }
            } else {
                require(
                    rows.len() == 1
                        && rows[0]["resourceId"] == resource_id(text(p, "commandId")?)
                        && rows[0]["sha256"] == c["expectedSha256"],
                )?;
                require(
                    array(&crate::project_memory::project(events)?, "bindings")
                        .iter()
                        .any(|b| {
                            b["subjectKind"] == "work"
                                && b["subjectId"] == c["workItemId"]
                                && b["projectId"] == p["projectId"]
                        }),
                )?;
            }
            rows.to_vec()
        }
        "project_studio.resource_published" => {
            let o = &p["origin"];
            let work = domain::project_work(events)?;
            require(array(&work, "runs").iter().any(|r| {
                r["runId"] == o["runId"]
                    && r["workItemId"] == o["workItemId"]
                    && r["ownerEpoch"] == o["ownerEpoch"]
                    && r["status"] == "running"
            }))?;
            require(events.iter().any(|r| {
                r["type"] == "agent_memory.context_bound"
                    && r["payload"]["projectId"] == p["projectId"]
                    && r["payload"]["runId"] == o["runId"]
                    && r["payload"]["ownerEpoch"] == o["ownerEpoch"]
            }))?;
            require(
                p["resource"]["resourceId"]
                    == resource_id(&format!("{}:{}", text(o, "runId")?, text(o, "nonce")?)),
            )?;
            require(!events.iter().any(|r| {
                r["type"] == "project_studio.resource_published"
                    && r["payload"]["origin"]["runId"] == o["runId"]
                    && r["payload"]["origin"]["nonce"] == o["nonce"]
            }))?;
            vec![p["resource"].clone()]
        }
        _ => return Ok(false),
    };
    project_exists(events, &p["projectId"])?;
    for mut r in rows {
        r["projectId"] = p["projectId"].clone();
        require(
            resources
                .insert(text(&r, "resourceId")?.to_owned(), r)
                .is_none(),
        )?;
    }
    Ok(true)
}
pub(super) fn recovered(events: &[Value], request: &Value) -> Option<Value> {
    let p = &events.iter().find(|e| {
        e["type"] == "project_studio.resources_imported"
            && e["payload"]["commandId"] == request["commandId"]
            && e["payload"]["projectId"] == request["payload"]["projectId"]
    })?["payload"];
    Some(
        json!({"outcome":"applied","reasonCode":"PROJECT_RESOURCES_IMPORTED","commandId":p["commandId"],"projectId":p["projectId"],"resources":p["resources"]}),
    )
}
fn artifact_bytes(host: &Host, p: &Value) -> Result<Vec<u8>> {
    let (root, _) = host.creation_scope(p)?;
    let bytes = conversation::copied_bytes(
        &root,
        &creation::relative_result(text(p, "path")?)?,
        FILE_LIMIT,
    )?;
    if hash(&p["expectedSha256"])? != digest(&bytes) {
        return Err(fail("PROJECT_ARTIFACT_CHANGED"));
    }
    Ok(bytes)
}
pub(super) async fn action(host: &Host, action: &str, p: &Value) -> Result<Value> {
    host.studio_project(p)?;
    match action {
        "project_resource_import_batch"
        | "project_resource_from_artifact"
        | "project_resources_from_artifacts" => {
            require(p["expectedProjectRevision"].as_u64().is_some_and(|n| n > 0))?;
            let command = id(&p["commandId"], "cmd:")?;
            let mut frozen = Vec::new();
            let mut total = 0;
            if action == "project_resource_import_batch" {
                let paths = p["sourcePaths"]
                    .as_array()
                    .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
                require(!paths.is_empty() && paths.len() <= 12)?;
                let mut unique = std::collections::BTreeSet::new();
                for path in paths {
                    let path = Path::new(bounded(path, 4096)?);
                    require(path.is_absolute() && unique.insert(path.to_path_buf()))?;
                    let parent = std::fs::canonicalize(
                        path.parent()
                            .ok_or_else(|| fail("PROJECT_RESOURCE_PATH_INVALID"))?,
                    )
                    .map_err(|_| fail("PROJECT_RESOURCE_PATH_INVALID"))?;
                    let name = path
                        .file_name()
                        .ok_or_else(|| fail("PROJECT_RESOURCE_PATH_INVALID"))?;
                    let bytes =
                        conversation::copied_bytes(&parent, Path::new(name), FILE_LIMIT - total)?;
                    total += bytes.len() as u64;
                    frozen.push((name.to_string_lossy().into_owned(), bytes));
                }
            } else if action == "project_resources_from_artifacts" {
                let files = p["files"]
                    .as_array()
                    .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
                require(!files.is_empty() && files.len() <= 12)?;
                let mut unique = std::collections::BTreeSet::new();
                for file in files {
                    exact(file, &["workItemId", "path", "expectedSha256"], &[])?;
                    require(unique.insert((text(file, "workItemId")?, text(file, "path")?)))?;
                    let mut selection = file.clone();
                    selection["projectId"] = p["projectId"].clone();
                    let bytes = artifact_bytes(host, &selection)?;
                    total += bytes.len() as u64;
                    require(total <= FILE_LIMIT)?;
                    frozen.push((
                        Path::new(text(file, "path")?)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                        bytes,
                    ));
                }
            } else {
                let bytes = artifact_bytes(host, p)?;
                frozen.push((
                    Path::new(text(p, "path")?)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    bytes,
                ));
            }
            let mut resources = Vec::new();
            for (i, (name, bytes)) in frozen.iter().enumerate() {
                let seed = if action == "project_resource_from_artifact" {
                    command.to_owned()
                } else {
                    format!("{command}:{i}")
                };
                resources.push(stage(
                    host,
                    text(p, "projectId")?,
                    &resource_id(&seed),
                    name,
                    bytes,
                )?);
            }
            host.store.append(vec![fact(
                "project_studio.resources_imported",
                json!({"projectId":p["projectId"],"commandId":command,"resources":resources}),
            )])?;
            Ok(
                json!({"outcome":"applied","reasonCode":"PROJECT_RESOURCES_IMPORTED","commandId":command,"projectId":p["projectId"],"resources":resources}),
            )
        }
        "project_resource_read" => {
            let (resource, bytes) =
                host.studio_resource(text(p, "projectId")?, text(p, "resourceId")?)?;
            if hash(&p["expectedSha256"])? != resource["sha256"] {
                return Err(fail("PROJECT_RESOURCE_CHANGED"));
            }
            let preview = host
                .studio_preview(text(&resource, "fileName")?, bytes)
                .await?;
            Ok(
                json!({"reasonCode":"PROJECT_RESOURCE_READ","projectId":p["projectId"],"resourceId":p["resourceId"],"sha256":resource["sha256"],"resource":resource,"preview":preview}),
            )
        }
        "project_artifact_visual_read" | "project_artifact_frame_read" => {
            let bytes = artifact_bytes(host, p)?;
            let path = text(p, "path")?;
            let source = format!("inputs/{}", conversation::copied_name(0, Path::new(path)));
            let (op, args, file) = if action == "project_artifact_visual_read" {
                let page = p["page"]
                    .as_u64()
                    .filter(|n| (1..=100).contains(n))
                    .ok_or_else(|| fail("PROJECT_ARTIFACT_SECTION_INVALID"))?;
                require(matches!(kind(path), "presentation" | "document" | "pdf"))?;
                (
                    "documents.render",
                    json!({"source":source,"pages":[page]}),
                    format!("page-{page}.png"),
                )
            } else {
                require(kind(path) == "video")?;
                let time = p["time"]
                    .as_f64()
                    .filter(|n| n.is_finite() && *n >= 0.0)
                    .ok_or_else(|| fail("PROJECT_ARTIFACT_SECTION_INVALID"))?;
                (
                    "video.frame",
                    json!({"source":source,"time":time}),
                    "preview.png".into(),
                )
            };
            let output = production::run(
                host.creation_data_dir(),
                op,
                &args,
                &[(source, bytes)],
                None,
            )
            .await?;
            let png = output
                .files
                .iter()
                .find(|(name, _)| *name == file)
                .ok_or_else(|| fail("PROJECT_PREVIEW_UNAVAILABLE"))?;
            require(png.1.len() <= 2 * 1024 * 1024)?;
            let mut result = json!({"reasonCode":if action=="project_artifact_visual_read"{"PROJECT_ARTIFACT_VISUAL_READ"}else{"PROJECT_ARTIFACT_FRAME_READ"},"projectId":p["projectId"],"workItemId":p["workItemId"],"path":path,"sha256":p["expectedSha256"],"preview":{"kind":"image","pngBytes":png.1,"width":output.value["width"],"height":output.value["height"]}});
            let key = if action == "project_artifact_visual_read" {
                "page"
            } else {
                "time"
            };
            result[key] = p[key].clone();
            Ok(result)
        }
        "project_artifact_open" => {
            let bytes = artifact_bytes(host, p)?;
            let path = text(p, "path")?;
            require(
                Path::new(path)
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("html")),
            )?;
            // Only fixed offline interactive/browser outputs may be opened, never arbitrary agent HTML.
            std::str::from_utf8(&bytes).map_err(|_| fail("PROJECT_PREVIEW_ENCODING"))?;
            let state = project(&host.store.read_events()?)?;
            require(array(&state, "jobs").iter().any(|job| {
                job["projectId"] == p["projectId"]
                    && job["workItemId"] == p["workItemId"]
                    && job["status"] == "completed"
                    && matches!(
                        job["operation"].as_str(),
                        Some("interactive.create" | "browser.inspect" | "browser.draft")
                    )
                    && array(job, "artifacts").iter().any(|artifact| {
                        artifact["path"] == p["path"] && artifact["sha256"] == p["expectedSha256"]
                    })
            }))?;
            let directory = std::fs::canonicalize(host.creation_data_dir())?
                .join("artifact-previews")
                .join(hash(&p["expectedSha256"])?);
            store::private_directory(&directory)?;
            let open_path = directory.join("output.html");
            store::atomic_write(&open_path, &bytes)?;
            Ok(
                json!({"reasonCode":"PROJECT_ARTIFACT_OPEN","projectId":p["projectId"],"commandId":p["commandId"],"workItemId":p["workItemId"],"path":path,"sha256":p["expectedSha256"],"openPath":open_path.to_string_lossy()}),
            )
        }
        "project_image_engine_set" | "project_image_engine_remove" => {
            let engine = if action == "project_image_engine_set" {
                production::configure_image_engine(
                    host.creation_data_dir(),
                    text(p, "projectId")?,
                    text(p, "url")?,
                    text(p, "checkpoint")?,
                )?
            } else {
                production::remove_image_engine(host.creation_data_dir(), text(p, "projectId")?)?
            };
            Ok(
                json!({"outcome":"applied","reasonCode":"PROJECT_IMAGE_ENGINE_CONFIGURED","projectId":p["projectId"],"imageEngine":engine}),
            )
        }
        _ => Err(fail("PROJECT_STUDIO_ACTION_UNSUPPORTED")),
    }
}
pub(super) fn publish(host: &Host, binding: &agent_memory::Binding, args: &Value) -> Result<Value> {
    let events = host.store.read_events()?;
    let request_hash = digest(&serde_json::to_vec(args)?);
    if let Some(prior) = events.iter().find(|e| {
        e["type"] == "project_studio.resource_published"
            && e["payload"]["origin"]["runId"] == binding.run_id
            && e["payload"]["origin"]["nonce"] == args["nonce"]
    }) {
        require(
            prior["payload"]["projectId"] == binding.project_id
                && prior["payload"]["origin"]["requestHash"] == request_hash,
        )?;
        return Ok(
            json!({"reasonCode":"PROJECT_RESOURCE_PUBLISHED","resource":prior["payload"]["resource"]}),
        );
    }
    require(
        events
            .iter()
            .filter(|e| {
                e["type"] == "project_studio.resource_published"
                    && e["payload"]["origin"]["runId"] == binding.run_id
            })
            .count()
            < 32,
    )?;
    let source = text(args, "source")?;
    let relative = creation::relative_result(source)?;
    require(relative.starts_with("outputs"))?;
    let (root, _) = host.creation_scope(
        &json!({"projectId":binding.project_id,"workItemId":binding.work_item_id}),
    )?;
    let bytes = conversation::copied_bytes(&root, &relative, 64 * 1024)?;
    std::str::from_utf8(&bytes).map_err(|_| fail("PROJECT_RESOURCE_TEXT_REQUIRED"))?;
    let resource = stage(
        host,
        &binding.project_id,
        &resource_id(&format!("{}:{}", binding.run_id, text(args, "nonce")?)),
        "source.txt",
        &bytes,
    )?;
    host.checked_agent_scope(binding)?;
    host.store.append(vec![fact("project_studio.resource_published",json!({"projectId":binding.project_id,"resource":resource,"origin":{"runId":binding.run_id,"workItemId":binding.work_item_id,"ownerEpoch":binding.owner_epoch,"nonce":args["nonce"],"requestHash":request_hash,"source":source,"sourceSha256":digest(&bytes)}}))])?;
    Ok(json!({"reasonCode":"PROJECT_RESOURCE_PUBLISHED","resource":resource}))
}

pub(super) async fn validate_selection(
    host: &Host,
    path: &str,
    bytes: &[u8],
    section: &str,
) -> Result<()> {
    if section.is_empty() {
        return Ok(());
    }
    if let Some(region) = section.strip_prefix("region:") {
        require(kind(path) == "image")?;
        let values = region
            .split(',')
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| fail("PROJECT_ARTIFACT_SECTION_INVALID"))?;
        if values.len() != 4
            || values
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0)
            || values[2] <= 0.0
            || values[3] <= 0.0
            || values[0] + values[2] > 1.000001
            || values[1] + values[3] > 1.000001
        {
            return Err(fail("PROJECT_ARTIFACT_SECTION_INVALID"));
        }
        let canonical = format!(
            "region:{:.6},{:.6},{:.6},{:.6}",
            values[0], values[1], values[2], values[3]
        );
        if canonical != section {
            return Err(fail("PROJECT_ARTIFACT_SECTION_INVALID"));
        }
        let preview = host.studio_preview(path, bytes.to_vec()).await?;
        require(preview["kind"] == "image")?;
        return Ok(());
    }
    let preview = host.studio_preview(path, bytes.to_vec()).await?;
    if let Some(range) = section.strip_prefix("time:") {
        require(kind(path) == "video")?;
        let parts = range.split(':').collect::<Vec<_>>();
        require(parts.len() == 2)?;
        let start = parts[0]
            .parse::<f64>()
            .map_err(|_| fail("PROJECT_ARTIFACT_SECTION_INVALID"))?;
        let end = parts[1]
            .parse::<f64>()
            .map_err(|_| fail("PROJECT_ARTIFACT_SECTION_INVALID"))?;
        require(
            start.is_finite()
                && end.is_finite()
                && start >= 0.0
                && end > start
                && preview["durationSeconds"]
                    .as_f64()
                    .is_some_and(|d| end <= d)
                && format!("time:{start:.3}:{end:.3}") == section,
        )?;
        return Ok(());
    }
    if let Some(page) = section.strip_prefix("page:") {
        require(matches!(kind(path), "pdf" | "document" | "presentation"))?;
        let page = page
            .parse::<u64>()
            .map_err(|_| fail("PROJECT_ARTIFACT_SECTION_INVALID"))?;
        if page == 0 || page > 100 {
            return Err(fail("PROJECT_ARTIFACT_SECTION_INVALID"));
        }
        let source = format!("inputs/{}", conversation::copied_name(0, Path::new(path)));
        production::run(
            host.creation_data_dir(),
            "documents.render",
            &json!({"source":source,"pages":[page]}),
            &[(source, bytes.to_vec())],
            None,
        )
        .await?;
        return Ok(());
    }
    if !array(&preview, "pages").iter().any(|v| v["id"] == section) {
        return Err(fail("PROJECT_ARTIFACT_SECTION_INVALID"));
    }
    Ok(())
}
