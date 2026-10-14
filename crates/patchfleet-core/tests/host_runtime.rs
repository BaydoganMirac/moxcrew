use patchfleet_core::{now, Host};
use serde_json::{json, Value};
use std::process::Command;

fn command() -> Value {
    json!({"commandId":format!("cmd:{}",uuid::Uuid::new_v4()),"createdAt":now(),
        "expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true)})
}
fn git_workspace(path: &std::path::Path) {
    std::fs::create_dir_all(path).unwrap();
    assert!(Command::new("git")
        .arg("init")
        .arg("--quiet")
        .arg(path)
        .status()
        .unwrap()
        .success());
}

#[tokio::test]
async fn desktop_commands_replay_idempotently_and_enforce_revisions() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    git_workspace(&workspace);
    let data = directory.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let mut register = command();
    register["workingDirectory"] = json!(workspace);
    let r = host
        .action("workspace_register", register.clone())
        .await
        .unwrap();
    assert_eq!(r["outcome"], "applied");
    assert_eq!(
        host.action("workspace_register", register.clone())
            .await
            .unwrap(),
        r
    );
    let workspace_id = r["workspaceId"].clone();
    let mut enqueue = command();
    enqueue["workspaceId"] = workspace_id.clone();
    enqueue["title"] = json!("Keep integer cents");
    enqueue["instruction"] = json!("Use integer amounts. Private source canary.");
    let receipt = host.action("work_enqueue", enqueue.clone()).await.unwrap();
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(
        host.action("work_enqueue", enqueue.clone()).await.unwrap(),
        receipt
    );
    let mut changed = enqueue.clone();
    changed["title"] = json!("Different title");
    assert!(host.action("work_enqueue", changed).await.is_err());
    let mut remove = command();
    remove["workItemId"] = receipt["workItemId"].clone();
    remove["expectedItemRevision"] = json!(99);
    assert_eq!(
        host.action("work_remove", remove).await.unwrap()["reasonCode"],
        "STALE_ITEM_REVISION"
    );
    let mut team = command();
    team["name"] = json!("Feature team");
    team["goal"] = json!("A bounded feature");
    team["workspaceId"] = workspace_id;
    team["templateId"] = json!("product-feature");
    team["orchestratorPackId"] = json!("pack:orchestrator");
    team["workerPackIds"] = json!(["pack:backend", "pack:qa"]);
    team["settings"] =
        json!({"concurrency":1,"retryLimit":0,"timeoutMinutes":10,"failurePolicy":"stop"});
    let created = host.action("team_create", team.clone()).await.unwrap();
    assert_eq!(created["reasonCode"], "TEAM_CREATED");
    assert_eq!(
        host.action("team_create", team.clone()).await.unwrap(),
        created
    );
    let mut start = command();
    start["teamId"] = created["teamId"].clone();
    start["expectedTeamRevision"] = json!(99);
    assert!(host
        .action("team_start", start)
        .await
        .unwrap_err()
        .0
        .contains("STALE_TEAM_REVISION"));
    let snapshot = host.snapshot().unwrap();
    assert_eq!(snapshot["work"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["teams"]["items"].as_array().unwrap().len(), 1);
    assert!(!snapshot["cloud"].to_string().contains("credential"));
    host.shutdown().await;
    drop(host);
    let restored = Host::open(data).unwrap();
    assert_eq!(restored.snapshot().unwrap()["work"], snapshot["work"]);
    assert_eq!(restored.action("team_create", team).await.unwrap(), created);
    assert!(restored
        .action("arbitrary_shell", json!({"command":"anything"}))
        .await
        .is_err());
    restored.shutdown().await;
}

#[tokio::test]
async fn closure_and_legacy_directory_prevent_unowned_mutation() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("runtime.json"), "{}").unwrap();
    assert!(Host::open(directory.path().to_owned()).is_err());
    std::fs::remove_file(directory.path().join("runtime.json")).unwrap();
    let host = Host::open(directory.path().to_owned()).unwrap();
    assert!(Host::open(directory.path().to_owned()).is_err());
    host.shutdown().await;
    assert_eq!(
        host.action("observe", json!({})).await.unwrap_err().0,
        "HOST_CLOSING"
    );
}

#[tokio::test]
async fn malformed_ipc_and_forbidden_pack_changes_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(directory.path().to_owned()).unwrap();
    assert!(host.action("cloud_pair",json!({"cloudUrl":"https://example.com","pairingCode":"x","displayName":"Test","shell":"arbitrary"})).await.is_err());
    assert!(host
        .action("pack_remove", json!({"packId":"pack:backend"}))
        .await
        .is_err());
    assert!(host
        .action(
            "workspace_register",
            json!({"workingDirectory":"relative/path"})
        )
        .await
        .is_err());
    host.shutdown().await;
}

#[tokio::test]
async fn fresh_install_and_corrupt_optional_cloud_still_allow_local_work() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("cloud.json"),
        "malformed pairing record",
    )
    .unwrap();
    let host = Host::open(directory.path().to_owned()).unwrap();
    let snapshot = host.snapshot().unwrap();
    for key in ["work", "workspaces", "teams"] {
        assert!(snapshot[key]["items"].is_array(), "{key}");
    }
    assert!(snapshot["work"]["runs"].is_array());
    assert_eq!(snapshot["cloud"]["error"], true);
    assert_eq!(
        host.action("cloud_sync", json!({})).await.unwrap_err().0,
        "CLOUD_RECOVERY_REQUIRED"
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("cloud.json")).unwrap(),
        "malformed pairing record"
    );
    host.shutdown().await;
}

#[tokio::test]
async fn local_only_sync_keeps_canonical_state_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(directory.path().to_owned()).unwrap();
    let before = host.snapshot().unwrap();
    assert_eq!(
        host.action("cloud_sync", json!({})).await.unwrap(),
        json!({"kind":"unpaired"})
    );
    host.background_tick(false).await.unwrap();
    assert_eq!(host.snapshot().unwrap(), before);
    host.shutdown().await;
}

#[tokio::test]
async fn owner_commands_expire_with_a_durable_replayable_result() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(directory.path().to_owned()).unwrap();
    let command = json!({"packId":"pack:backend","commandId":format!("cmd:{}",uuid::Uuid::new_v4()),"createdAt":"2026-01-01T00:00:00.000Z","expiresAt":"2026-01-01T00:05:00.000Z"});
    let result = host.action("pack_remove", command.clone()).await.unwrap();
    assert_eq!(result["outcome"], "expired");
    assert_eq!(host.action("pack_remove", command).await.unwrap(), result);
    let bad = json!({"packId":"pack:backend","createdAt":"yesterday","expiresAt":"tomorrow"});
    assert!(host.action("pack_remove", bad).await.is_err());
    host.shutdown().await;
}

fn remove_last_receipt(data: &std::path::Path, kind: &str) {
    let path = data.join("events.jsonl");
    let bytes = std::fs::read_to_string(&path).unwrap();
    let mut events = bytes
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let index = events.iter().rposition(|e| e["type"] == kind).unwrap();
    events.remove(index);
    std::fs::write(
        path,
        events.iter().map(|e| format!("{e}\n")).collect::<String>(),
    )
    .unwrap();
}

#[tokio::test]
async fn workspace_applied_facts_recover_their_missing_receipts() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    git_workspace(&workspace);
    let data = directory.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let mut register = command();
    register["workingDirectory"] = json!(workspace);
    let original = host
        .action("workspace_register", register.clone())
        .await
        .unwrap();
    host.shutdown().await;
    drop(host);
    remove_last_receipt(&data, "workspace.command.receipted");
    let host = Host::open(data.clone()).unwrap();
    let recovered = host.action("workspace_register", register).await.unwrap();
    assert_eq!(recovered["intentId"], original["intentId"]);
    assert_eq!(recovered["reasonCode"], "WORKSPACE_REGISTERED");
    assert_eq!(recovered["outcome"], "applied");
    let mut remove = command();
    remove["workspaceId"] = original["workspaceId"].clone();
    remove["expectedWorkspaceRevision"] = json!(1);
    let removed = host
        .action("workspace_remove", remove.clone())
        .await
        .unwrap();
    host.shutdown().await;
    drop(host);
    remove_last_receipt(&data, "workspace.command.receipted");
    let host = Host::open(data).unwrap();
    let recovered = host.action("workspace_remove", remove).await.unwrap();
    assert_eq!(recovered["intentId"], removed["intentId"]);
    assert_eq!(recovered["reasonCode"], "WORKSPACE_REMOVED");
    assert_eq!(recovered["outcome"], "applied");
    assert!(host.snapshot().unwrap()["workspaces"]["items"]
        .as_array()
        .unwrap()
        .is_empty());
    host.shutdown().await;
}

#[tokio::test]
async fn queue_applied_fact_recovers_without_duplicate_work() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    git_workspace(&workspace);
    let data = directory.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let mut enqueue = command();
    enqueue["workingDirectory"] = json!(workspace);
    enqueue["title"] = json!("Recover queue");
    enqueue["instruction"] = json!("Persist before acknowledging.");
    let original = host.action("work_enqueue", enqueue.clone()).await.unwrap();
    host.shutdown().await;
    drop(host);
    remove_last_receipt(&data, "command.receipted");
    let host = Host::open(data).unwrap();
    let recovered = host.action("work_enqueue", enqueue).await.unwrap();
    assert_eq!(recovered["intentId"], original["intentId"]);
    assert_eq!(recovered["outcome"], "applied");
    assert_eq!(
        host.snapshot().unwrap()["work"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    host.shutdown().await;
}

#[tokio::test]
async fn removed_workspaces_block_draft_start_without_changing_receipt_replay() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    git_workspace(&workspace);
    let data = directory.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let mut register = command();
    register["workingDirectory"] = json!(workspace);
    let registered = host.action("workspace_register", register).await.unwrap();
    let mut team = command();
    team["name"] = json!("Obsolete draft");
    team["goal"] = json!("A bounded feature");
    team["workspaceId"] = registered["workspaceId"].clone();
    team["templateId"] = json!("product-feature");
    team["orchestratorPackId"] = json!("pack:orchestrator");
    team["workerPackIds"] = json!(["pack:backend", "pack:qa"]);
    team["settings"] =
        json!({"concurrency":1,"retryLimit":0,"timeoutMinutes":10,"failurePolicy":"stop"});
    let created = host.action("team_create", team).await.unwrap();
    let mut remove = command();
    remove["workspaceId"] = registered["workspaceId"].clone();
    remove["expectedWorkspaceRevision"] = json!(1);
    let removed = host
        .action("workspace_remove", remove.clone())
        .await
        .unwrap();
    let mut start = command();
    start["teamId"] = created["teamId"].clone();
    start["expectedTeamRevision"] = json!(1);
    assert_eq!(
        host.action("team_start", start).await.unwrap_err().0,
        "WORKSPACE_NOT_REGISTERED"
    );
    let snapshot = host.snapshot().unwrap();
    assert_eq!(snapshot["teams"]["items"][0]["status"], "draft");
    assert_eq!(
        snapshot["teams"]["items"][0]["runtimeIssue"],
        "WORKSPACE_NOT_REGISTERED"
    );
    assert_eq!(
        host.action("workspace_remove", remove.clone())
            .await
            .unwrap(),
        removed
    );
    host.shutdown().await;
    drop(host);

    // Recover a state produced by the previous start-before-validation behavior.
    let store = patchfleet_core::store::Store::open(data.clone()).unwrap();
    store
        .append(vec![patchfleet_core::event(
            "team.started",
            json!({"teamId":created["teamId"]}),
        )])
        .unwrap();
    drop(store);
    let host = Host::open(data).unwrap();
    assert_eq!(
        host.action("workspace_remove", remove).await.unwrap(),
        removed
    );
    let mut new_remove = command();
    new_remove["workspaceId"] = registered["workspaceId"].clone();
    new_remove["expectedWorkspaceRevision"] = json!(1);
    assert_eq!(
        host.action("workspace_remove", new_remove)
            .await
            .unwrap_err()
            .0,
        "WORKSPACE_IN_USE"
    );
    host.shutdown().await;
}
