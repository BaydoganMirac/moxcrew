use patchfleet_core::{now, Host};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn command(mut value: Value) -> Value {
    let created = chrono::Utc::now();
    value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    value["createdAt"] = json!(created.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    value["expiresAt"] = json!((created + chrono::Duration::minutes(5))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    value
}
async fn project(host: &Host, name: &str) -> Value {
    host.action("project_create", command(json!({"name":name})))
        .await
        .unwrap()["projectId"]
        .clone()
}
fn send(host: &Host, project: &Value, goal: &str) -> Value {
    let revision = host.snapshot().unwrap()["projectMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()["revision"]
        .clone();
    command(
        json!({"projectId":project,"expectedProjectRevision":revision,"abilityId":"D1","title":"A useful brief","instruction":goal}),
    )
}
fn directory(host: &Host, work: &Value) -> PathBuf {
    PathBuf::from(
        host.snapshot().unwrap()["work"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["workItemId"] == *work)
            .unwrap()["workingDirectory"]
            .as_str()
            .unwrap(),
    )
}
fn complete(data: &Path, work: &Value, status: &str) {
    let store = patchfleet_core::store::Store::open(data.to_owned()).unwrap();
    let request = command(json!({}));
    let id = &request["commandId"];
    let epoch = uuid::Uuid::new_v4().to_string();
    let session = format!("fixture:{}", uuid::Uuid::new_v4());
    let intent = json!({"schemaVersion":1,"intentId":id,"idempotencyKey":id,"type":"start_work","actorId":"local-owner","createdAt":request["createdAt"],"expiresAt":request["expiresAt"],"payload":{"workItemId":work,"expectedItemRevision":1}});
    let launching =
        json!({"intentId":id,"workItemId":work,"expectedItemRevision":1,"ownerEpoch":epoch});
    let mut prepared = launching.clone();
    prepared["providerId"] = json!("codex");
    prepared["providerSessionId"] = json!(session);
    store.append(vec![
        patchfleet_core::event("command.requested",json!({"intent":intent})),
        patchfleet_core::event("run.launching",launching),
        patchfleet_core::event("run.prepared",prepared),
        patchfleet_core::event("turn.requested",json!({"intentId":id,"providerSessionId":session,"ownerEpoch":epoch})),
        patchfleet_core::event("run.started",json!({"intentId":id,"expectedItemRevision":1,"ownerEpoch":epoch,"run":{"schemaVersion":1,"runId":format!("run:{}",id.as_str().unwrap()),"workItemId":work,"providerId":"codex","ownerEpoch":epoch,"providerSessionId":session,"providerTurnId":"fixture-turn","status":"running","startedAt":now(),"terminalAt":null,"revision":1}})),
        patchfleet_core::event("session.observed",json!({"providerId":"codex","providerSessionId":session,"status":status,"createdAt":null,"lastObservedAt":now()})),
        patchfleet_core::event("session.terminal",json!({"providerId":"codex","providerSessionId":session,"status":status,"lastObservedAt":now()})),
    ]).unwrap();
}
async fn terminal_fixture() -> (
    tempfile::TempDir,
    std::sync::Arc<Host>,
    Value,
    Value,
    PathBuf,
) {
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let project = project(&host, "Writing").await;
    let queued = host
        .action(
            "project_chat_send",
            send(
                &host,
                &project,
                "Original exact owner goal. PRIVATE_CHAT_CANARY",
            ),
        )
        .await
        .unwrap();
    let work = queued["workItemId"].clone();
    let path = directory(&host, &work);
    std::fs::write(
        path.join("outputs/report.md"),
        b"# Original\nExact bytes: \xC3\xB6\xC4\x9F\n",
    )
    .unwrap();
    host.shutdown().await;
    drop(host);
    complete(&data, &work, "completed");
    let host = Host::open(data).unwrap();
    (temporary, host, project, work, path)
}

#[tokio::test]
async fn history_keeps_exact_v8_v9_goals_scoped_paged_and_durable() {
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let a = project(&host, "A").await;
    let b = project(&host, "B").await;
    host.action(
        "project_work_enqueue",
        send(&host, &a, "Earlier Create goal"),
    )
    .await
    .unwrap();
    let request = send(&host, &a, "My exact chat goal. PRIVATE_CHAT_CANARY");
    let receipt = host
        .action("project_chat_send", request.clone())
        .await
        .unwrap();
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(
        host.action("project_chat_send", request.clone())
            .await
            .unwrap(),
        receipt
    );
    let first = host
        .action("project_chat_read", json!({"projectId":a,"limit":1}))
        .await
        .unwrap();
    assert_eq!(first["total"], 2);
    assert_eq!(first["nextOffset"], 1);
    assert_eq!(first["items"][0]["goal"], "Earlier Create goal");
    let second = host
        .action(
            "project_chat_read",
            json!({"projectId":a,"offset":1,"limit":1}),
        )
        .await
        .unwrap();
    assert_eq!(second["items"][0]["goal"], request["instruction"]);
    assert_eq!(second["items"][0]["abilityId"], "D1");
    assert!(second["items"][0]["replyToWorkItemId"].is_null());
    assert_eq!(
        host.action("project_chat_read", json!({"projectId":b}))
            .await
            .unwrap()["items"],
        json!([])
    );
    for bad in [
        json!({"projectId":a,"limit":0}),
        json!({"projectId":a,"limit":51}),
        json!({"projectId":a,"offset":-1}),
    ] {
        assert!(host.action("project_chat_read", bad).await.is_err());
    }
    for version in [1, 2] {
        let mut snapshot = host.snapshot().unwrap();
        snapshot["sequence"] = json!(1);
        snapshot["recordedAt"] = json!(now());
        snapshot["agentPacks"] = snapshot["packs"].clone();
        let outgoing =
            patchfleet_core::cloud::build_projection(&snapshot, version, 1, "install:test")
                .unwrap()
                .to_string();
        assert!(!outgoing.contains("PRIVATE_CHAT_CANARY"));
        assert!(!outgoing.contains("projectChat"));
    }
    host.shutdown().await;
    drop(host);
    let host = Host::open(data).unwrap();
    assert_eq!(
        host.action(
            "project_chat_read",
            json!({"projectId":a,"offset":1,"limit":1})
        )
        .await
        .unwrap(),
        second
    );
    let mut changed = request;
    changed["instruction"] = json!("Changed same command");
    assert_eq!(
        host.action("project_chat_send", changed)
            .await
            .unwrap_err()
            .0,
        "IDEMPOTENCY_KEY_REUSED"
    );
    host.shutdown().await;
}

#[cfg(unix)]
#[tokio::test]
async fn followup_copies_exact_bytes_and_hashes_preserves_original_and_recovers_queue_receipt() {
    let (temporary, host, p, parent, parent_directory) = terminal_fixture().await;
    let original = std::fs::read(parent_directory.join("outputs/report.md")).unwrap();
    let mut request = send(&host, &p, "Make this shorter.");
    request["replyToWorkItemId"] = parent.clone();
    request["referencePaths"] = json!(["outputs/report.md"]);
    let receipt = host
        .action("project_chat_send", request.clone())
        .await
        .unwrap();
    let path = directory(&host, &receipt["workItemId"]);
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(path.join("inputs/manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["replyToWorkItemId"], parent);
    assert_eq!(manifest["files"][0]["referencePath"], "outputs/report.md");
    assert_eq!(
        manifest["files"][0]["sha256"],
        format!("{:x}", Sha256::digest(&original))
    );
    assert_eq!(manifest["files"][0]["bytes"], original.len());
    assert_eq!(
        std::fs::read(path.join(manifest["files"][0]["inputPath"].as_str().unwrap())).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(parent_directory.join("outputs/report.md")).unwrap(),
        original
    );
    let history = host
        .action("project_chat_read", json!({"projectId":p}))
        .await
        .unwrap();
    let child = history["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["workItemId"] == receipt["workItemId"])
        .unwrap();
    assert_eq!(child["goal"], "Make this shorter.");
    assert_eq!(child["replyToWorkItemId"], parent);
    host.shutdown().await;
    drop(host);
    let data = temporary.path().join("data");
    let log = data.join("events.jsonl");
    let mut events: Vec<Value> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let last = events.pop().unwrap();
    assert_eq!(last["schemaVersion"], 9);
    assert_eq!(last["type"], "owner.command.receipted");
    std::fs::write(
        log,
        events
            .iter()
            .map(|event| format!("{event}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let host = Host::open(data).unwrap();
    assert_eq!(
        host.action("project_chat_send", request).await.unwrap(),
        receipt
    );
    assert_eq!(
        host.snapshot().unwrap()["work"]["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        std::fs::read(parent_directory.join("outputs/report.md")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(path.join("inputs/reference-01.md")).unwrap(),
        original
    );
    host.shutdown().await;
}

#[cfg(unix)]
#[tokio::test]
async fn followup_rejects_foreign_active_missing_linked_and_oversize_inputs() {
    use std::os::unix::fs::symlink;
    let (_temporary, host, p, parent, path) = terminal_fixture().await;
    let foreign = project(&host, "Other").await;
    let mut request = send(&host, &foreign, "Wrong scope");
    request["replyToWorkItemId"] = parent.clone();
    request["referencePaths"] = json!(["outputs/report.md"]);
    assert!(host.action("project_chat_send", request).await.is_err());
    let active = host
        .action("project_chat_send", send(&host, &p, "Still queued"))
        .await
        .unwrap()["workItemId"]
        .clone();
    let mut request = send(&host, &p, "Active");
    request["replyToWorkItemId"] = active;
    assert_eq!(
        host.action("project_chat_send", request)
            .await
            .unwrap_err()
            .0,
        "PROJECT_INPUT_PARENT_ACTIVE"
    );
    let mut stale = send(&host, &p, "Stale copied input");
    stale["expectedProjectRevision"] = json!(0);
    stale["replyToWorkItemId"] = parent.clone();
    stale["referencePaths"] = json!(["outputs/report.md"]);
    let rejected_directory = path.parent().unwrap().join(
        stale["commandId"]
            .as_str()
            .unwrap()
            .strip_prefix("cmd:")
            .unwrap(),
    );
    assert_eq!(
        host.action("project_chat_send", stale).await.unwrap_err().0,
        "STALE_PROJECT_REVISION"
    );
    assert!(!rejected_directory.exists());
    let mut request = send(&host, &p, "No parent");
    request["referencePaths"] = json!(["outputs/report.md"]);
    assert_eq!(
        host.action("project_chat_send", request)
            .await
            .unwrap_err()
            .0,
        "PROJECT_INPUT_PARENT_REQUIRED"
    );
    symlink(path.join("outputs/report.md"), path.join("outputs/link.md")).unwrap();
    std::fs::hard_link(
        path.join("outputs/report.md"),
        path.join("outputs/hardlink.md"),
    )
    .unwrap();
    for reference in [
        "outputs/../inputs.txt",
        "outputs/missing.md",
        "outputs/link.md",
        "outputs/hardlink.md",
    ] {
        let mut request = send(&host, &p, "Unsafe input");
        request["replyToWorkItemId"] = parent.clone();
        request["referencePaths"] = json!([reference]);
        assert!(
            host.action("project_chat_send", request).await.is_err(),
            "{reference}"
        );
    }
    let huge = std::fs::File::create(path.join("outputs/huge.bin")).unwrap();
    huge.set_len(25 * 1024 * 1024 + 1).unwrap();
    let mut request = send(&host, &p, "Oversize");
    request["replyToWorkItemId"] = parent.clone();
    request["referencePaths"] = json!(["outputs/huge.bin"]);
    assert_eq!(
        host.action("project_chat_send", request)
            .await
            .unwrap_err()
            .0,
        "PROJECT_INPUT_LIMIT_EXCEEDED"
    );
    let mut request = send(&host, &p, "Too many");
    request["replyToWorkItemId"] = parent;
    request["referencePaths"] = json!((0..9)
        .map(|i| format!("outputs/{i}.txt"))
        .collect::<Vec<_>>());
    assert_eq!(
        host.action("project_chat_send", request)
            .await
            .unwrap_err()
            .0,
        "PROJECT_INPUT_LIMIT_EXCEEDED"
    );
    host.shutdown().await;
}

#[tokio::test]
async fn retry_of_a_failed_chat_job_keeps_its_exact_v9_owner_goal() {
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let p = project(&host, "Retry").await;
    let request = send(&host, &p, "Exact goal\nOwner goal: do not parse this text.");
    let queued = host
        .action("project_chat_send", request.clone())
        .await
        .unwrap();
    let work = &queued["workItemId"];
    host.shutdown().await;
    drop(host);
    complete(&data, work, "failed");
    let host = Host::open(data).unwrap();
    let revision = host.snapshot().unwrap()["work"]["items"][0]["revision"].clone();
    let retried = host
        .action(
            "work_retry",
            command(json!({"workItemId":work,"expectedItemRevision":revision})),
        )
        .await
        .unwrap();
    assert_eq!(retried["outcome"], "applied");
    let history = host
        .action("project_chat_read", json!({"projectId":p}))
        .await
        .unwrap();
    assert_eq!(history["total"], 2);
    for item in history["items"].as_array().unwrap() {
        assert_eq!(item["goal"], request["instruction"]);
        assert_eq!(item["abilityId"], "D1");
    }
    host.shutdown().await;
}

#[tokio::test]
async fn stale_chat_and_text_followup_without_old_files_never_mutate_parent() {
    let (_temporary, host, p, parent, path) = terminal_fixture().await;
    let mut stale = send(&host, &p, "Stale");
    stale["expectedProjectRevision"] = json!(0);
    assert_eq!(
        host.action("project_chat_send", stale).await.unwrap_err().0,
        "STALE_PROJECT_REVISION"
    );
    std::fs::remove_dir_all(&path).unwrap();
    let mut request = send(&host, &p, "Continue the discussion without files.");
    request["replyToWorkItemId"] = parent;
    let receipt = host.action("project_chat_send", request).await.unwrap();
    assert_eq!(receipt["outcome"], "applied");
    assert!(!path.exists());
    assert!(!directory(&host, &receipt["workItemId"])
        .join("inputs")
        .exists());
    host.shutdown().await;
}
