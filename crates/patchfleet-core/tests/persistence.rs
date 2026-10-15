use patchfleet_core::{
    domain::{project_events, validate_event, validate_workspace},
    event,
    store::{atomic_write, Store},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;
use std::sync::Arc;
use tempfile::tempdir;

fn legacy(mut bundle: Value) -> Value {
    bundle.as_object_mut().unwrap().remove("projectMemory");
    bundle.as_object_mut().unwrap().remove("projectDecisions");
    bundle.as_object_mut().unwrap().remove("sharedMemory");
    bundle
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/parity-v1.json")).unwrap()
}
fn cases() -> Vec<Value> {
    fixture()["cases"].as_array().unwrap().clone()
}
fn owner_request(command: &str) -> Value {
    let mut v = event(
        "owner.command.requested",
        json!({"commandId":command,"action":"pack_remove","payload":{"packId":"pack:fixture"}}),
    );
    v["schemaVersion"] = json!(2);
    v
}
fn owner_receipt(command: &str, result: Value) -> Value {
    let mut v = event(
        "owner.command.receipted",
        json!({"commandId":command,"result":result}),
    );
    v["schemaVersion"] = json!(2);
    v
}
fn log_bytes(events: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for v in events {
        serde_json::to_writer(&mut bytes, v).unwrap();
        bytes.push(b'\n');
    }
    bytes
}

#[test]
fn legacy_javascript_replay_matches_every_projection_and_prefix() {
    let cases = cases();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let events = case["events"].as_array().unwrap();
        assert_eq!(
            legacy(project_events(events).unwrap()),
            case["expected"],
            "complete {}",
            case["name"]
        );
        for (index, expected) in case["prefixSha256"].as_array().unwrap().iter().enumerate() {
            let value = legacy(project_events(&events[..=index]).unwrap());
            let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&value).unwrap()));
            assert_eq!(
                digest,
                expected.as_str().unwrap(),
                "{} prefix {}",
                case["name"],
                index + 1
            );
        }
    }
}

#[test]
fn canonical_log_reopens_and_rebuilds_all_five_projection_shapes() {
    for case in cases() {
        let dir = tempdir().unwrap();
        let events = case["events"].as_array().unwrap().clone();
        {
            let store = Store::open(dir.path().into()).unwrap();
            store.append(events.clone()).unwrap();
        }
        for file in [
            "observation.json",
            "work-items.json",
            "workspaces.json",
            "agent-packs.json",
            "agent-teams.json",
        ] {
            fs::write(dir.path().join(file), b"corrupt stale projection").unwrap();
        }
        let store = Store::open(dir.path().into()).unwrap();
        assert_eq!(store.read_events().unwrap(), events);
        assert_eq!(legacy(store.projections().unwrap()), case["expected"]);
        for (key, file) in [
            ("observation", "observation.json"),
            ("work", "work-items.json"),
            ("workspaces", "workspaces.json"),
            ("packs", "agent-packs.json"),
            ("teams", "agent-teams.json"),
        ] {
            let path = dir.path().join(file);
            if case["expected"][key].is_null() {
                assert!(!path.exists());
            } else {
                let v: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
                assert_eq!(v, case["expected"][key]);
            }
        }
    }
}

#[test]
fn incomplete_final_record_is_ignored_then_truncated_before_durable_append() {
    for tail in [
        b"{\"partial\":".as_slice(),
        b"{\"complete_but_no_newline\":true}",
        b"\xff\xfe",
    ] {
        let dir = tempdir().unwrap();
        let request = owner_request("cmd:before");
        let mut bytes = log_bytes(std::slice::from_ref(&request));
        bytes.extend(tail);
        fs::write(dir.path().join("events.jsonl"), &bytes).unwrap();
        let store = Store::open(dir.path().into()).unwrap();
        assert_eq!(store.read_events().unwrap(), vec![request.clone()]);
        // A read/open does not silently migrate or truncate canonical bytes.
        assert_eq!(fs::read(dir.path().join("events.jsonl")).unwrap(), bytes);
        store
            .append(vec![owner_receipt(
                "cmd:before",
                json!({"outcome":"failed","reasonCode":"OUTCOME_UNKNOWN"}),
            )])
            .unwrap();
        let final_bytes = fs::read(dir.path().join("events.jsonl")).unwrap();
        assert_eq!(final_bytes.last(), Some(&b'\n'));
        assert_eq!(store.read_events().unwrap().len(), 2);
        assert!(!String::from_utf8(final_bytes).unwrap().contains("partial"));
    }
}

#[test]
fn completed_corruption_unknown_schema_and_duplicate_identifiers_fail_closed() {
    let valid = owner_request("cmd:valid");
    let mut unknown = valid.clone();
    unknown["type"] = json!("future.unknown");
    let mut version = valid.clone();
    version["schemaVersion"] = json!(3);
    let mut extra = valid.clone();
    extra["payload"]["secret"] = json!("unexpected");
    for bad in [
        b"not-json\n".to_vec(),
        b"\n".to_vec(),
        b"\xff\n".to_vec(),
        log_bytes(&[unknown]),
        log_bytes(&[version]),
        log_bytes(&[extra]),
        log_bytes(&[valid.clone(), valid.clone()]),
    ] {
        let dir = tempdir().unwrap();
        let mut bytes = log_bytes(std::slice::from_ref(&valid));
        bytes.extend(bad);
        fs::write(dir.path().join("events.jsonl"), &bytes).unwrap();
        assert!(Store::open(dir.path().into()).is_err());
        assert_eq!(fs::read(dir.path().join("events.jsonl")).unwrap(), bytes);
    }
}

#[test]
fn semantic_corruption_rejected_before_append_changes_canonical_bytes() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let valid = owner_request("cmd:valid");
    store.append(vec![valid.clone()]).unwrap();
    let before = fs::read(dir.path().join("events.jsonl")).unwrap();
    let mut receipt = owner_receipt("cmd:missing", json!({"outcome":"failed"}));
    assert!(store.append(vec![receipt.clone()]).is_err());
    receipt["payload"]["commandId"] = json!("cmd:valid");
    store.append(vec![receipt.clone()]).unwrap();
    let receipted = fs::read(dir.path().join("events.jsonl")).unwrap();
    assert!(store.append(vec![receipt]).is_err());
    assert_eq!(
        fs::read(dir.path().join("events.jsonl")).unwrap(),
        receipted
    );
    assert!(store.append(vec![valid]).is_err());
    let orphan = cases()[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["type"] == "work.enqueued")
        .unwrap()
        .clone();
    assert!(project_events(&[orphan]).is_err());
    assert!(!before.is_empty());
}

#[test]
fn work_receipt_revision_and_owner_epoch_transition_are_checked() {
    let events = cases()[0]["events"].as_array().unwrap().clone();
    for kind in ["run.started", "run.interrupted", "command.receipted"] {
        let mut corrupt = events.clone();
        let target = corrupt.iter_mut().find(|e| e["type"] == kind).unwrap();
        if kind == "command.receipted" {
            target["payload"]["receipt"]["workProjectionRevision"] = json!(999);
        } else if kind == "run.started" {
            target["payload"]["ownerEpoch"] = json!("different-epoch");
        } else {
            target["payload"]["expectedRunRevision"] = json!(999);
        }
        assert!(project_events(&corrupt).is_err(), "{kind}");
    }
}

#[test]
fn transactions_are_serialized_and_return_results_after_persistence() {
    let dir = tempdir().unwrap();
    let store = Arc::new(Store::open(dir.path().into()).unwrap());
    let mut threads = Vec::new();
    for _ in 0..16 {
        let store = Arc::clone(&store);
        threads.push(std::thread::spawn(move || {
            store
                .transaction(|events| {
                    let count = events
                        .iter()
                        .filter(|e| e["type"] == "owner.command.requested")
                        .count()
                        + 1;
                    let id = format!("cmd:{}", uuid::Uuid::new_v4());
                    Ok((
                        vec![
                            owner_request(&id),
                            owner_receipt(&id, json!({"count":count})),
                        ],
                        json!(count),
                    ))
                })
                .unwrap()
                .as_u64()
                .unwrap()
        }));
    }
    let mut counts: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    counts.sort();
    assert_eq!(counts, (1..=16).collect::<Vec<_>>());
    assert_eq!(store.read_events().unwrap().len(), 32);
    drop(store);
    assert_eq!(
        Store::open(dir.path().into())
            .unwrap()
            .read_events()
            .unwrap()
            .len(),
        32
    );
}

#[test]
fn one_os_writer_lock_excludes_a_second_process_and_releases_on_drop() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    assert!(Store::open(dir.path().into()).is_err());
    let run = |expected: &str| {
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "writer_lock_child", "--nocapture"])
            .env("PATCHFLEET_TEST_LOCK_DIR", dir.path())
            .env("PATCHFLEET_TEST_LOCK_EXPECT", expected)
            .output()
            .unwrap()
    };
    let child = run("locked");
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    drop(store);
    let child = run("available");
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
}
#[test]
fn writer_lock_child() {
    let Ok(dir) = std::env::var("PATCHFLEET_TEST_LOCK_DIR") else {
        return;
    };
    let available = Store::open(dir.into()).is_ok();
    assert_eq!(
        available,
        std::env::var("PATCHFLEET_TEST_LOCK_EXPECT").unwrap() == "available"
    );
}

#[test]
fn atomic_replacement_retains_one_complete_target_and_no_temporary_files() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cache.json");
    atomic_write(&path, b"first").unwrap();
    atomic_write(&path, b"second").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"second");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}

#[test]
fn windows_workspace_records_replay_portably_and_names_sort_deterministically() {
    let workspace = json!({"schemaVersion":1,"workspaceId":"workspace:11111111-1111-4111-8111-111111111111","displayName":"Türkçe","workingDirectory":"C:\\Projects\\Patchfleet","createdAt":"2026-07-20T12:00:00.000Z","revision":1});
    validate_workspace(&workspace).unwrap();
    let mut unc = workspace.clone();
    unc["workingDirectory"] = json!("\\\\server\\share\\project");
    validate_workspace(&unc).unwrap();
    let mut relative = workspace;
    relative["workingDirectory"] = json!("C:relative");
    assert!(validate_workspace(&relative).is_err());
    let pack = cases()
        .into_iter()
        .find(|c| c["name"] == "pack-update-removal-reinstall")
        .unwrap()["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["type"] == "agent_pack.installed")
        .unwrap()["payload"]["pack"]
        .clone();
    let mut events = Vec::new();
    for (id, name) in [
        ("pack:a", "z"),
        ("pack:b", "A"),
        ("pack:c", "é"),
        ("pack:d", "a"),
    ] {
        let mut p = pack.clone();
        p["id"] = json!(id);
        p["name"] = json!(name);
        events.push(event("agent_pack.installed", json!({"pack":p})));
    }
    let projection = project_events(&events).unwrap();
    let names: Vec<_> = projection["packs"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["pack"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["A", "a", "z", "é"]);
}

#[test]
fn v2_local_commands_are_allowlisted_bounded_and_unique() {
    let request = owner_request("cmd:fixture");
    let receipt = owner_receipt("cmd:fixture", json!({"outcome":"applied"}));
    project_events(&[request.clone(), receipt.clone()]).unwrap();
    assert!(project_events(std::slice::from_ref(&receipt)).is_err());
    let mut duplicate = receipt.clone();
    duplicate["id"] = json!(uuid::Uuid::new_v4());
    assert!(project_events(&[request.clone(), receipt, duplicate]).is_err());
    let mut unsafe_action = request.clone();
    unsafe_action["payload"]["action"] = json!("arbitrary_shell");
    assert!(validate_event(&unsafe_action).is_err());
    let mut oversized = request;
    oversized["payload"]["payload"] = json!({"text":"x".repeat(128*1024)});
    assert!(validate_event(&oversized).is_err());
}

#[test]
fn v2_cloud_team_receipts_are_linked_to_the_original_sequence_and_intent() {
    let intent = json!({"schemaVersion":2,"intentId":"22222222-2222-4222-8222-222222222222","idempotencyKey":"cloud:key","type":"cancel_team","actorId":"cloud-owner","createdAt":"2026-07-20T12:00:00.000Z","expiresAt":"2026-07-20T12:01:00.000Z","payload":{"teamId":"team:11111111-1111-4111-8111-111111111111","expectedTeamRevision":1}});
    let receipt = json!({"schemaVersion":2,"intentId":intent["intentId"],"idempotencyKey":intent["idempotencyKey"],"commandType":"cancel_team","outcome":"rejected","reasonCode":"TEAM_NOT_FOUND","completedAt":"2026-07-20T12:00:01.000Z","teamId":intent["payload"]["teamId"],"teamRevision":0});
    let mut request = event(
        "cloud_team.command.requested",
        json!({"sequence":7,"intent":intent}),
    );
    request["schemaVersion"] = json!(2);
    let mut terminal = event(
        "cloud_team.command.receipted",
        json!({"sequence":7,"receipt":receipt}),
    );
    terminal["schemaVersion"] = json!(2);
    terminal["recordedAt"] = terminal["payload"]["receipt"]["completedAt"].clone();
    assert!(project_events(&[terminal.clone()]).is_err());
    project_events(&[request.clone(), terminal.clone()]).unwrap();
    let mut wrong = terminal.clone();
    wrong["payload"]["sequence"] = json!(8);
    assert!(project_events(&[request.clone(), wrong]).is_err());
    let mut wrong = terminal;
    wrong["payload"]["receipt"]["idempotencyKey"] = json!("other:key");
    assert!(project_events(&[request, wrong]).is_err());
}

#[cfg(windows)]
#[test]
fn windows_private_storage_has_protected_owner_and_system_acl() {
    use std::{os::windows::ffi::OsStrExt, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, GetNamedSecurityInfoW,
            SDDL_REVISION_1, SE_FILE_OBJECT,
        },
        Security::DACL_SECURITY_INFORMATION,
    };
    fn inspect(path: &std::path::Path) {
        let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut descriptor = null_mut();
        let mut dacl = null_mut();
        let status = unsafe {
            GetNamedSecurityInfoW(
                name.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut descriptor,
            )
        };
        assert_eq!(status, 0);
        assert!(!dacl.is_null());
        assert_eq!(
            unsafe { (*dacl).AceCount },
            2,
            "only current owner and SYSTEM grants"
        );
        let mut sddl = null_mut();
        let mut length = 0;
        assert_ne!(
            unsafe {
                ConvertSecurityDescriptorToStringSecurityDescriptorW(
                    descriptor,
                    SDDL_REVISION_1,
                    DACL_SECURITY_INFORMATION,
                    &mut sddl,
                    &mut length,
                )
            },
            0
        );
        let text = String::from_utf16(unsafe {
            std::slice::from_raw_parts(sddl, length.saturating_sub(1) as usize)
        })
        .unwrap();
        unsafe {
            LocalFree(sddl.cast());
            LocalFree(descriptor);
        }
        assert!(text.starts_with("D:P"), "protected ACL: {text}");
        assert!(text.contains(";;;SY)"), "SYSTEM access: {text}");
        assert!(
            text.contains(";;;S-1-"),
            "current process SID access: {text}"
        );
        for forbidden in [";;;WD)", ";;;BU)", ";;;AU)", ";;;BA)"] {
            assert!(!text.contains(forbidden), "unexpected grant: {text}");
        }
    }
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let request = owner_request("cmd:windows");
    store.append(vec![request]).unwrap();
    let cloud = dir.path().join("cloud.json");
    atomic_write(&cloud, b"{\"private\":true}\n").unwrap();
    for path in [
        dir.path().to_path_buf(),
        dir.path().join("writer.lock"),
        dir.path().join("events.jsonl"),
        cloud,
    ] {
        inspect(&path);
    }
}

#[test]
fn cached_reads_do_not_replay_or_rewrite_projection_files() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let case = cases()[0].clone();
    store
        .append(case["events"].as_array().unwrap().clone())
        .unwrap();
    let files = [
        "events.jsonl",
        "observation.json",
        "work-items.json",
        "workspaces.json",
    ];
    let before: Vec<_> = files
        .iter()
        .map(|f| {
            fs::metadata(dir.path().join(f))
                .unwrap()
                .modified()
                .unwrap()
        })
        .collect();
    for _ in 0..100 {
        assert_eq!(legacy(store.projections().unwrap()), case["expected"]);
        assert_eq!(
            store.read_events().unwrap(),
            case["events"].as_array().unwrap().clone()
        );
    }
    let after: Vec<_> = files
        .iter()
        .map(|f| {
            fs::metadata(dir.path().join(f))
                .unwrap()
                .modified()
                .unwrap()
        })
        .collect();
    assert_eq!(before, after);
    let projection_path = dir.path().join("work-items.json");
    fs::write(&projection_path, "broken-cache").unwrap();
    assert_eq!(legacy(store.projections().unwrap()), case["expected"]);
    assert_eq!(
        fs::read(&projection_path).unwrap(),
        b"broken-cache",
        "cached read must not repair disk"
    );
    store.recover().unwrap();
    let fixed: Value = serde_json::from_slice(&fs::read(projection_path).unwrap()).unwrap();
    assert_eq!(fixed, case["expected"]["work"]);
}

#[test]
fn projection_write_failure_does_not_hide_fsynced_events_or_receipts() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    fs::create_dir(dir.path().join("work-items.json")).unwrap();
    let case = cases()[0].clone();
    let events = case["events"].as_array().unwrap().clone();
    assert!(store.append(events.clone()).is_err());
    assert_eq!(store.read_events().unwrap(), events);
    assert_eq!(legacy(store.projections().unwrap()), case["expected"]);
    fs::remove_dir(dir.path().join("work-items.json")).unwrap();
    store.recover().unwrap();
    drop(store);
    assert_eq!(
        legacy(
            Store::open(dir.path().into())
                .unwrap()
                .projections()
                .unwrap()
        ),
        case["expected"]
    );
}

#[test]
fn unexpected_external_log_mutation_fails_closed_until_explicit_recovery() {
    use std::io::Write;
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    store.append(vec![owner_request("cmd:original")]).unwrap();
    let additions = log_bytes(&[owner_request("cmd:external")]);
    fs::OpenOptions::new()
        .append(true)
        .open(dir.path().join("events.jsonl"))
        .unwrap()
        .write_all(&additions)
        .unwrap();
    assert!(store.projections().is_err());
    assert!(store.read_events().is_err());
    assert!(store
        .append(vec![owner_request("cmd:must-not-append")])
        .is_err());
    store.recover().unwrap();
    assert_eq!(store.read_events().unwrap().len(), 2);
}

#[cfg(unix)]
#[test]
fn private_storage_rejects_links_without_changing_outside_files() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = tempdir().unwrap();
    let outside = root.path().join("outside.jsonl");
    fs::write(&outside, b"").unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o644)).unwrap();
    for name in [
        "events.jsonl",
        "writer.lock",
        "observation.json",
        "work-items.json",
        "workspaces.json",
        "agent-packs.json",
        "agent-teams.json",
    ] {
        let data = root.path().join(name.replace('.', "-"));
        fs::create_dir(&data).unwrap();
        symlink(&outside, data.join(name)).unwrap();
        assert!(Store::open(data).is_err(), "{name}");
        assert_eq!(fs::read(&outside).unwrap(), b"");
        assert_eq!(
            fs::metadata(&outside).unwrap().permissions().mode() & 0o777,
            0o644
        );
    }
    let linked_file = root.path().join("private-link.json");
    symlink(&outside, &linked_file).unwrap();
    assert!(patchfleet_core::store::read_private_file(&linked_file).is_err());
    assert!(atomic_write(&linked_file, b"overwrite").is_err());
    let outside_dir = root.path().join("outside-dir");
    fs::create_dir(&outside_dir).unwrap();
    fs::set_permissions(&outside_dir, fs::Permissions::from_mode(0o755)).unwrap();
    let linked_dir = root.path().join("directory-link");
    symlink(&outside_dir, &linked_dir).unwrap();
    assert!(Store::open(linked_dir).is_err());
    assert_eq!(
        fs::metadata(outside_dir).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(
        fs::metadata(outside).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

#[test]
#[ignore = "manual 52,000-event cached-read timing probe"]
fn cached_projection_large_log_probe() {
    let dir = tempdir().unwrap();
    let events:Vec<_>=(0..52_000).map(|_|event("provider.observed",json!({"providerId":"codex","state":"available","version":"1.2.3","capabilities":{"recentObservation":true,"explicitLiveStatus":true},"observedAt":"2026-07-20T12:00:00.000Z","sessionIds":[]}))).collect();
    fs::write(dir.path().join("events.jsonl"), log_bytes(&events)).unwrap();
    let boot = std::time::Instant::now();
    let store = Store::open(dir.path().into()).unwrap();
    let boot_time = boot.elapsed();
    let before = fs::metadata(dir.path().join("observation.json"))
        .unwrap()
        .modified()
        .unwrap();
    let reads = std::time::Instant::now();
    for _ in 0..1_000 {
        assert_eq!(
            store.projections().unwrap()["observation"]["observations"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    let read_time = reads.elapsed();
    assert_eq!(
        before,
        fs::metadata(dir.path().join("observation.json"))
            .unwrap()
            .modified()
            .unwrap()
    );
    eprintln!("52,000 events: open/replay={boot_time:?}, 1,000 cached projections={read_time:?}, average={:?}",read_time/1_000);
}

#[test]
fn replaced_writer_lock_cannot_leave_the_old_host_serving_or_writing() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let lock = dir.path().join("writer.lock");
    fs::rename(&lock, dir.path().join("detached-writer.lock")).unwrap();
    fs::write(&lock, b"new lock").unwrap();
    assert!(store.projections().is_err());
    assert!(store.read_events().is_err());
    assert!(store.append(vec![owner_request("cmd:stale-lock")]).is_err());
    assert!(store.recover().is_err());
    assert!(!dir.path().join("events.jsonl").exists());
}

#[test]
fn hard_links_cannot_share_private_state_across_distinct_writer_directories() {
    let root = tempdir().unwrap();
    let outside = root.path().join("outside.jsonl");
    fs::write(&outside, b"").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o644)).unwrap();
    }
    let before = fs::metadata(&outside).unwrap().permissions();
    for name in ["events.jsonl", "writer.lock", "work-items.json"] {
        for suffix in ["first", "second"] {
            let data = root
                .path()
                .join(format!("{}-{suffix}", name.replace('.', "-")));
            fs::create_dir(&data).unwrap();
            fs::hard_link(&outside, data.join(name)).unwrap();
            assert!(
                Store::open(data).is_err(),
                "shared {name} inode must never acquire an independent writer"
            );
            assert_eq!(fs::read(&outside).unwrap(), b"");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(&outside).unwrap().permissions().mode(),
                    before.mode()
                );
            }
            #[cfg(windows)]
            {
                assert_eq!(
                    fs::metadata(&outside).unwrap().permissions().readonly(),
                    before.readonly()
                );
            }
        }
    }
    let linked = root.path().join("hard-linked-private.json");
    fs::hard_link(&outside, &linked).unwrap();
    assert!(patchfleet_core::store::read_private_file(&linked).is_err());
    assert!(atomic_write(&linked, b"must not write").is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"");
}
