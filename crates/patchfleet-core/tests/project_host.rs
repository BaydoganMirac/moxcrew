use patchfleet_core::{now, Host};
use serde_json::{json, Value};

fn command(mut args: Value) -> Value {
    args["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    let created = chrono::Utc::now();
    args["createdAt"] = json!(created.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    args["expiresAt"] = json!((created + chrono::Duration::minutes(5))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    args
}
fn project(snapshot: &Value, id: &Value) -> Value {
    snapshot["projectMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *id)
        .unwrap()
        .clone()
}
async fn link(host: &Host, project_id: &Value, workspace_id: &Value, action: &str) {
    let p = project(&host.snapshot().unwrap(), project_id);
    let receipt = host.action(action, command(json!({"projectId":project_id,"workspaceId":workspace_id,"expectedProjectRevision":p["revision"]}))).await.unwrap();
    assert_eq!(receipt["outcome"], "applied");
}

#[tokio::test]
async fn owner_sources_keep_scope_across_relink_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let data = directory.path().join("data");
    let worktree = directory.path().join("repository");
    std::fs::create_dir_all(&worktree).unwrap();
    assert!(std::process::Command::new("git")
        .args(["init", "--quiet"])
        .arg(&worktree)
        .status()
        .unwrap()
        .success());
    let host = Host::open(data.clone()).unwrap();
    let workspace = host
        .action(
            "workspace_register",
            command(json!({"workingDirectory":worktree,"displayName":"Main worktree"})),
        )
        .await
        .unwrap()["workspaceId"]
        .clone();
    let a = host
        .action("project_create", command(json!({"name":"Billing"})))
        .await
        .unwrap()["projectId"]
        .clone();
    let b = host
        .action("project_create", command(json!({"name":"Other project"})))
        .await
        .unwrap()["projectId"]
        .clone();
    link(&host, &a, &workspace, "project_link_workspace").await;
    let p = project(&host.snapshot().unwrap(), &a);
    let message = command(
        json!({"projectId":a,"expectedProjectRevision":p["revision"],"title":"Money","text":"PRIVATE_MEMORY_CANARY · tutarlar kuruş olarak saklanır."}),
    );
    let receipt = host
        .action("project_message", message.clone())
        .await
        .unwrap();
    assert_eq!(
        host.action("project_message", message.clone())
            .await
            .unwrap(),
        receipt
    );
    let task = command(
        json!({"workspaceId":workspace,"title":"Amount schema","instruction":"PRIVATE_TASK_CANARY · integer minor units"}),
    );
    let queued = host.action("work_enqueue", task.clone()).await.unwrap();
    assert_eq!(host.action("work_enqueue", task).await.unwrap(), queued);
    let team_request = command(
        json!({"name":"Billing team","goal":"PRIVATE_TEAM_CANARY · check amount handling","workspaceId":workspace,"templateId":"release","orchestratorPackId":"pack:orchestrator","workerPackIds":["pack:release"],"settings":{"concurrency":1,"retryLimit":0,"timeoutMinutes":10,"failurePolicy":"stop"}}),
    );
    let team = host
        .action("team_create", team_request.clone())
        .await
        .unwrap();
    assert_eq!(
        host.action("team_create", team_request).await.unwrap(),
        team
    );
    link(&host, &a, &workspace, "project_unlink_workspace").await;
    link(&host, &b, &workspace, "project_link_workspace").await;
    let start = command(json!({"teamId":team["teamId"],"expectedTeamRevision":1}));
    host.action("team_start", start).await.unwrap();
    let snapshot = host.snapshot().unwrap();
    let t = snapshot["teams"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["teamId"] == team["teamId"])
        .unwrap();
    let question = &t["tasks"][0]["question"];
    assert!(question["questionId"].is_string());
    host.action("team_answer", command(json!({"teamId":team["teamId"],"questionId":question["questionId"],"answer":"PRIVATE_ANSWER_CANARY · local test target only","expectedTeamRevision":t["revision"]}))).await.unwrap();
    let current = host.snapshot().unwrap();
    let t = current["teams"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["teamId"] == team["teamId"])
        .unwrap();
    host.action("team_approve", command(json!({"teamId":team["teamId"],"taskId":t["tasks"][0]["taskId"],"decision":"approved","note":"PRIVATE_NOTE_CANARY · verify fixture only","expectedTeamRevision":t["revision"]}))).await.unwrap();
    let sources_a = host
        .action("project_memory_read", json!({"projectId":a}))
        .await
        .unwrap();
    let sources_b = host
        .action("project_memory_read", json!({"projectId":b}))
        .await
        .unwrap();
    assert_eq!(sources_a["total"], 5);
    assert_eq!(sources_b["total"], 0);
    let kinds = sources_a["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["kind"].as_str().unwrap())
        .collect::<Vec<_>>();
    for kind in [
        "message",
        "work_instruction",
        "team_goal",
        "owner_answer",
        "approval_note",
    ] {
        assert!(kinds.contains(&kind), "{kind}");
    }
    // Generated team work contains pack/task text but is not an owner source.
    assert!(
        host.snapshot().unwrap()["work"]["items"]
            .as_array()
            .unwrap()
            .len()
            > 1
    );
    let snapshot = host.snapshot().unwrap();
    assert!(snapshot["projectMemory"]["sources"].is_null());
    for version in [1, 2] {
        let mut cloud_input = snapshot.clone();
        cloud_input["sequence"] = json!(1);
        cloud_input["recordedAt"] = json!(now());
        cloud_input["agentPacks"] = cloud_input["packs"].clone();
        cloud_input["projectMemory"] = sources_a.clone();
        let outgoing =
            patchfleet_core::cloud::build_projection(&cloud_input, version, 1, "install:test")
                .unwrap()
                .to_string();
        assert!(!outgoing.contains("PRIVATE_"));
        assert!(!outgoing.contains(a.as_str().unwrap()));
        assert!(!outgoing.contains("projectMemory"));
    }
    let forged = command(
        json!({"projectId":b,"sourceId":receipt["sourceId"],"expectedSourceRevision":1,"title":"Forged","text":"Wrong project"}),
    );
    let forged_result = host.action("project_source_edit", forged).await;
    assert!(forged_result.is_err() || forged_result.unwrap()["outcome"] == "rejected");
    host.shutdown().await;
    drop(host);
    let restored = Host::open(data).unwrap();
    assert_eq!(
        restored
            .action("project_memory_read", json!({"projectId":a}))
            .await
            .unwrap(),
        sources_a
    );
    assert_eq!(
        restored.action("project_message", message).await.unwrap(),
        receipt
    );
    restored.shutdown().await;
}

fn append_unknown_start(data: &std::path::Path, work: &Value) {
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

#[tokio::test]
async fn task_retry_inherits_original_scope_including_no_project() {
    let directory = tempfile::tempdir().unwrap();
    let worktree = directory.path().join("repository");
    std::fs::create_dir_all(&worktree).unwrap();
    assert!(std::process::Command::new("git")
        .args(["init", "--quiet"])
        .arg(&worktree)
        .status()
        .unwrap()
        .success());
    let data = directory.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let w = host
        .action(
            "workspace_register",
            command(json!({"workingDirectory":worktree})),
        )
        .await
        .unwrap()["workspaceId"]
        .clone();
    let unbound = host
        .action(
            "work_enqueue",
            command(
                json!({"workspaceId":w,"title":"Old task","instruction":"Unbound owner source"}),
            ),
        )
        .await
        .unwrap()["workItemId"]
        .clone();
    let a = host
        .action(
            "project_create",
            command(json!({"name":"Original project"})),
        )
        .await
        .unwrap()["projectId"]
        .clone();
    let b = host
        .action("project_create", command(json!({"name":"New project"})))
        .await
        .unwrap()["projectId"]
        .clone();
    link(&host, &a, &w, "project_link_workspace").await;
    let bound = host.action("work_enqueue", command(json!({"workspaceId":w,"title":"Bound task","instruction":"Original project source"}))).await.unwrap()["workItemId"].clone();
    link(&host, &a, &w, "project_unlink_workspace").await;
    link(&host, &b, &w, "project_link_workspace").await;
    host.shutdown().await;
    drop(host);
    // Valid canonical failure prefixes, without starting any provider process.
    append_unknown_start(&data, &bound);
    append_unknown_start(&data, &unbound);
    let host = Host::open(data).unwrap();
    for id in [&bound, &unbound] {
        let snapshot = host.snapshot().unwrap();
        let item = snapshot["work"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["workItemId"] == *id)
            .unwrap();
        assert_eq!(item["status"], "blocked");
        let retry = command(json!({"workItemId":id,"expectedItemRevision":item["revision"]}));
        let receipt = host.action("work_retry", retry.clone()).await.unwrap();
        assert_eq!(receipt["outcome"], "applied");
        assert_eq!(host.action("work_retry", retry).await.unwrap(), receipt);
    }
    assert_eq!(
        host.action("project_memory_read", json!({"projectId":a}))
            .await
            .unwrap()["total"],
        2
    );
    assert_eq!(
        host.action("project_memory_read", json!({"projectId":b}))
            .await
            .unwrap()["total"],
        0
    );
    host.shutdown().await;
}
