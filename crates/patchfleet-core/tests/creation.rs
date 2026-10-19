use patchfleet_core::Host;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn command(mut payload: Value) -> Value {
    let created = chrono::Utc::now();
    payload["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    payload["createdAt"] = json!(created.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    payload["expiresAt"] = json!((created + chrono::Duration::minutes(5))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    payload
}
async fn project(host: &Host, name: &str) -> Value {
    host.action("project_create", command(json!({"name":name})))
        .await
        .unwrap()["projectId"]
        .clone()
}
fn revision(host: &Host, project: &Value) -> Value {
    host.snapshot().unwrap()["projectMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()["revision"]
        .clone()
}
fn enqueue(host: &Host, project: &Value) -> Value {
    command(
        json!({"projectId":project,"expectedProjectRevision":revision(host,project),"abilityId":"D1","title":"A useful brief","instruction":"Turn my notes into a clear brief. PRIVATE_CREATION_CANARY"}),
    )
}
fn directory(host: &Host, work: &Value) -> PathBuf {
    PathBuf::from(
        host.snapshot().unwrap()["work"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["workItemId"] == *work)
            .unwrap()["workingDirectory"]
            .as_str()
            .unwrap(),
    )
}
fn selection(project: &Value, work: &Value, path: &str) -> Value {
    json!({"projectId":project,"workItemId":work,"path":path})
}

#[tokio::test]
async fn project_job_needs_no_git_and_replays_exact_scope_after_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let p = project(&host, "Holiday research").await;
    let request = enqueue(&host, &p);
    let receipt = host
        .action("project_work_enqueue", request.clone())
        .await
        .unwrap();
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(receipt["projectId"], p);
    assert_eq!(
        host.action("project_work_enqueue", request.clone())
            .await
            .unwrap(),
        receipt
    );
    let work = receipt["workItemId"].clone();
    let path = directory(&host, &work);
    assert!(path.join("outputs").is_dir());
    assert!(!path.join(".git").exists());
    assert!(
        patchfleet_core::providers::validate_workspace(path.to_str().unwrap())
            .await
            .is_err()
    );
    assert!(host.snapshot().unwrap()["projectJobs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|job| job["workItemId"] == work && job["projectId"] == p));
    let mut changed = request.clone();
    changed["abilityId"] = json!("R1");
    assert_eq!(
        host.action("project_work_enqueue", changed)
            .await
            .unwrap_err()
            .0,
        "IDEMPOTENCY_KEY_REUSED"
    );
    let mut stale = enqueue(&host, &p);
    stale["expectedProjectRevision"] = json!(0);
    assert_eq!(
        host.action("project_work_enqueue", stale)
            .await
            .unwrap_err()
            .0,
        "STALE_PROJECT_REVISION"
    );
    let empty = host
        .action(
            "project_results_read",
            json!({"projectId":p,"workItemId":work}),
        )
        .await
        .unwrap();
    assert_eq!(empty["workStatus"], "queued");
    assert_eq!(empty["items"], json!([]));
    let sources = host
        .action("project_memory_read", json!({"projectId":p}))
        .await
        .unwrap();
    assert!(sources.to_string().contains("PRIVATE_CREATION_CANARY"));
    for version in [1, 2] {
        let mut snapshot = host.snapshot().unwrap();
        snapshot["sequence"] = json!(1);
        snapshot["recordedAt"] = json!(patchfleet_core::now());
        snapshot["agentPacks"] = snapshot["packs"].clone();
        let outgoing =
            patchfleet_core::cloud::build_projection(&snapshot, version, 1, "install:test")
                .unwrap()
                .to_string();
        assert!(!outgoing.contains("PRIVATE_CREATION_CANARY"));
        assert!(!outgoing.contains("projectJobs"));
        assert!(!outgoing.contains(path.to_str().unwrap()));
    }
    host.shutdown().await;
    drop(host);
    let host = Host::open(data).unwrap();
    assert_eq!(
        host.action("project_work_enqueue", request).await.unwrap(),
        receipt
    );
    assert_eq!(directory(&host, &work), path);
    host.shutdown().await;
}

#[tokio::test]
async fn actual_results_export_without_overwrite_and_stay_project_scoped() {
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let a = project(&host, "A").await;
    let b = project(&host, "B").await;
    let receipt = host
        .action("project_work_enqueue", enqueue(&host, &a))
        .await
        .unwrap();
    let work = &receipt["workItemId"];
    let path = directory(&host, work);
    std::fs::write(path.join("outputs/brief.md"), "# Brief\nA real artifact.").unwrap();
    std::fs::write(path.join("private-input.txt"), "SHOULD_NOT_LIST").unwrap();
    let items = host
        .action(
            "project_results_read",
            json!({"projectId":a,"workItemId":work}),
        )
        .await
        .unwrap();
    assert_eq!(items["items"].as_array().unwrap().len(), 1);
    assert_eq!(items["items"][0]["path"], "outputs/brief.md");
    assert_eq!(items["items"][0]["textPreview"], true);
    assert_eq!(items["workStatus"], "queued");
    let read = selection(&a, work, "outputs/brief.md");
    assert_eq!(
        host.action("project_result_read", read.clone())
            .await
            .unwrap()["text"],
        "# Brief\nA real artifact."
    );
    assert!(host
        .action(
            "project_result_read",
            selection(&b, work, "outputs/brief.md")
        )
        .await
        .is_err());
    let target = temporary.path().join("brief.md");
    let mut export = command(read);
    export["targetPath"] = json!(target);
    let exported = host
        .action("project_result_export", export.clone())
        .await
        .unwrap();
    assert_eq!(exported["reasonCode"], "PROJECT_RESULT_EXPORTED");
    assert_eq!(
        host.action("project_result_export", export.clone())
            .await
            .unwrap(),
        exported
    );
    let mut collision = command(selection(&a, work, "outputs/brief.md"));
    collision["targetPath"] = json!(target);
    assert_eq!(
        host.action("project_result_export", collision)
            .await
            .unwrap_err()
            .0,
        "PROJECT_RESULT_EXPORT_EXISTS"
    );
    assert_eq!(
        std::fs::read(&target).unwrap(),
        std::fs::read(path.join("outputs/brief.md")).unwrap()
    );
    let mut protected = command(selection(&a, work, "outputs/brief.md"));
    protected["targetPath"] = json!(data.join("forged.json"));
    assert!(host
        .action("project_result_export", protected)
        .await
        .is_err());
    assert!(!data.join("forged.json").exists());
    host.shutdown().await;
    drop(host);
    let host = Host::open(data).unwrap();
    assert_eq!(
        host.action("project_result_export", export).await.unwrap(),
        exported
    );
    host.shutdown().await;
}

#[tokio::test]
async fn results_reject_traversal_unsupported_and_oversize_files() {
    let temporary = tempfile::tempdir().unwrap();
    let host = Host::open(temporary.path().join("data")).unwrap();
    let p = project(&host, "P").await;
    let receipt = host
        .action("project_work_enqueue", enqueue(&host, &p))
        .await
        .unwrap();
    let work = &receipt["workItemId"];
    let path = directory(&host, work);
    for bad in [
        "outputs/../events.jsonl",
        "../events.jsonl",
        "/etc/passwd",
        "outputs\\secret.txt",
        "outputs/./report.md",
        "outputs/.hidden.txt",
        "inputs/data.csv",
    ] {
        assert!(
            host.action("project_result_read", selection(&p, work, bad))
                .await
                .is_err(),
            "{bad}"
        );
    }
    std::fs::write(path.join("outputs/presentation.pptx"), b"not previewable").unwrap();
    std::fs::write(path.join("outputs/binary.txt"), [255, 254, 253]).unwrap();
    let large = std::fs::File::create(path.join("outputs/large.md")).unwrap();
    large.set_len(256 * 1024 + 1).unwrap();
    assert_eq!(
        host.action(
            "project_result_read",
            selection(&p, work, "outputs/presentation.pptx")
        )
        .await
        .unwrap_err()
        .0,
        "PROJECT_RESULT_PREVIEW_UNSUPPORTED"
    );
    assert!(host
        .action(
            "project_result_read",
            selection(&p, work, "outputs/binary.txt")
        )
        .await
        .is_err());
    assert_eq!(
        host.action(
            "project_result_read",
            selection(&p, work, "outputs/large.md")
        )
        .await
        .unwrap_err()
        .0,
        "PROJECT_RESULT_TOO_LARGE"
    );
    assert!(host
        .action(
            "project_result_read",
            selection(&p, work, "outputs/missing.md")
        )
        .await
        .is_err());
    for index in 0..205 {
        std::fs::write(path.join(format!("outputs/item-{index}.txt")), "x").unwrap();
    }
    let items = host
        .action(
            "project_results_read",
            json!({"projectId":p,"workItemId":work}),
        )
        .await
        .unwrap();
    assert_eq!(items["items"].as_array().unwrap().len(), 200);
    assert_eq!(items["truncated"], true);
    host.shutdown().await;
}

#[cfg(unix)]
#[tokio::test]
async fn symlinks_hardlinks_and_replaced_job_roots_cannot_expose_private_files() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().unwrap();
    let host = Host::open(temporary.path().join("data")).unwrap();
    let p = project(&host, "P").await;
    let receipt = host
        .action("project_work_enqueue", enqueue(&host, &p))
        .await
        .unwrap();
    let work = &receipt["workItemId"];
    let path = directory(&host, work);
    let outside = temporary.path().join("private");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "NEVER_EXPOSE").unwrap();
    symlink(outside.join("secret.txt"), path.join("outputs/linked.txt")).unwrap();
    symlink(&outside, path.join("outputs/subfolder")).unwrap();
    std::fs::hard_link(
        outside.join("secret.txt"),
        path.join("outputs/hardlink.txt"),
    )
    .unwrap();
    for bad in [
        "outputs/linked.txt",
        "outputs/subfolder/secret.txt",
        "outputs/hardlink.txt",
    ] {
        assert!(host
            .action("project_result_read", selection(&p, work, bad))
            .await
            .is_err());
    }
    let results = host
        .action(
            "project_results_read",
            json!({"projectId":p,"workItemId":work}),
        )
        .await
        .unwrap();
    assert_eq!(results["items"], json!([]));
    std::fs::rename(&path, path.with_extension("original")).unwrap();
    symlink(&outside, &path).unwrap();
    let snapshot = host.snapshot().unwrap();
    let job = snapshot["projectJobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|job| job["workItemId"] == *work)
        .unwrap();
    assert!(job["runtimeIssue"].is_string());
    assert!(host
        .action(
            "project_results_read",
            json!({"projectId":p,"workItemId":work})
        )
        .await
        .is_err());
    host.shutdown().await;
}

#[tokio::test]
async fn deleted_output_directory_keeps_job_identity_with_a_visible_issue() {
    let temporary = tempfile::tempdir().unwrap();
    let host = Host::open(temporary.path().join("data")).unwrap();
    let p = project(&host, "P").await;
    let receipt = host
        .action("project_work_enqueue", enqueue(&host, &p))
        .await
        .unwrap();
    let work = &receipt["workItemId"];
    std::fs::remove_dir(directory(&host, work).join("outputs")).unwrap();
    let snapshot = host.snapshot().unwrap();
    let job = snapshot["projectJobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|job| job["workItemId"] == *work)
        .unwrap();
    assert_eq!(job["projectId"], p);
    assert_eq!(job["runtimeIssue"], "PROJECT_JOB_DIRECTORY_UNAVAILABLE");
    assert!(host
        .action(
            "project_results_read",
            json!({"projectId":p,"workItemId":work})
        )
        .await
        .is_err());
    host.shutdown().await;
}

#[tokio::test]
async fn managed_retry_gets_a_new_folder_and_preserves_original_results() {
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let p = project(&host, "P").await;
    let receipt = host
        .action("project_work_enqueue", enqueue(&host, &p))
        .await
        .unwrap();
    let work = receipt["workItemId"].clone();
    let original = directory(&host, &work);
    std::fs::write(original.join("outputs/original.txt"), "Preserve me").unwrap();
    host.shutdown().await;
    drop(host);
    append_unknown_start(&data, &work);
    let host = Host::open(data).unwrap();
    let current = host.snapshot().unwrap()["work"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["workItemId"] == work)
        .unwrap()
        .clone();
    assert_eq!(current["status"], "blocked");
    let request = command(json!({"workItemId":work,"expectedItemRevision":current["revision"]}));
    let retried = host.action("work_retry", request.clone()).await.unwrap();
    assert_eq!(retried["outcome"], "applied");
    assert_ne!(retried["workItemId"], work);
    assert_eq!(host.action("work_retry", request).await.unwrap(), retried);
    let new_directory = directory(&host, &retried["workItemId"]);
    assert_ne!(new_directory, original);
    assert!(new_directory.join("outputs").is_dir());
    assert_eq!(
        std::fs::read_to_string(original.join("outputs/original.txt")).unwrap(),
        "Preserve me"
    );
    assert!(host.snapshot().unwrap()["projectJobs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|job| job["workItemId"] == retried["workItemId"] && job["projectId"] == p));
    host.shutdown().await;
}

fn append_unknown_start(data: &Path, work: &Value) {
    let store = patchfleet_core::store::Store::open(data.to_owned()).unwrap();
    let args = command(json!({}));
    let intent = json!({"schemaVersion":1,"intentId":args["commandId"],"idempotencyKey":args["commandId"],"type":"start_work","actorId":"local-owner","createdAt":args["createdAt"],"expiresAt":args["expiresAt"],"payload":{"workItemId":work,"expectedItemRevision":1}});
    let payload = json!({"intentId":args["commandId"],"workItemId":work,"expectedItemRevision":1,"ownerEpoch":uuid::Uuid::new_v4().to_string()});
    store
        .append(vec![
            patchfleet_core::event("command.requested", json!({"intent":intent})),
            patchfleet_core::event("run.launching", payload.clone()),
            patchfleet_core::event("run.start_unknown", payload),
        ])
        .unwrap();
}
