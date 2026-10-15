use super::*;
use std::{path::Path, process::Command, time::Duration};

fn command(mut value: Value) -> Value {
    value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    value
}
async fn apply(host: &Host, action: &str, value: Value) -> Value {
    let result = host.action(action, command(value)).await.unwrap();
    assert_eq!(result["outcome"], "applied", "{action}: {result}");
    result
}
fn project_revision(host: &Host, id: &Value) -> Value {
    host.snapshot().unwrap()["projectMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *id)
        .unwrap()["revision"]
        .clone()
}
async fn decision(
    host: &Host,
    project: &Value,
    source: &Value,
    title: &str,
    accepted: bool,
) -> Value {
    let proposed = apply(host,"project_decision_propose",json!({"projectId":project,"expectedProjectRevision":project_revision(host,project),"title":title,"context":"Context","decision":"Use integer minor units","rationale":"Exact arithmetic","consequences":"Explicit conversions","sources":[{"sourceId":source,"revision":1}]})).await;
    if accepted {
        apply(host,"project_decision_accept",json!({"projectId":project,"decisionId":proposed["decisionId"],"expectedDecisionRevision":1})).await;
    }
    proposed["decisionId"].clone()
}
fn git(path: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap()
        .status
        .success());
}
struct Fixture {
    _directory: tempfile::TempDir,
    host: Arc<Host>,
    project: Value,
    other: Value,
    source: Value,
    work: Vec<Value>,
}
async fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join("main");
    let linked = directory.path().join("linked");
    std::fs::create_dir(&main).unwrap();
    git(&main, &["init", "--quiet"]);
    git(
        &main,
        &[
            "-c",
            "user.name=Memory test",
            "-c",
            "user.email=memory@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    git(
        &main,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            linked.to_str().unwrap(),
        ],
    );
    let missing = directory.path().join("uninstalled-provider");
    let controller = providers::Controller::with_programs(providers::Programs {
        codex: missing.clone(),
        claude: missing.clone(),
        gemini: missing,
        timeout: Duration::from_secs(3),
    });
    let host = Host::open_with_controller(directory.path().join("data"), controller).unwrap();
    let project =
        apply(&host, "project_create", json!({"name":"Billing"})).await["projectId"].clone();
    let other =
        apply(&host, "project_create", json!({"name":"Other project"})).await["projectId"].clone();
    let mut work = Vec::new();
    for (index, path) in [main, linked].iter().enumerate() {
        let workspace = apply(
            &host,
            "workspace_register",
            json!({"workingDirectory":path}),
        )
        .await["workspaceId"]
            .clone();
        apply(&host,"project_link_workspace",json!({"projectId":project,"workspaceId":workspace,"expectedProjectRevision":project_revision(&host,&project)})).await;
        work.push(apply(&host,"work_enqueue",json!({"workspaceId":workspace,"title":format!("Task {index}"),"instruction":"OWNER_INSTRUCTION_CANARY"})).await["workItemId"].clone());
    }
    let source = apply(&host,"project_message",json!({"projectId":project,"expectedProjectRevision":project_revision(&host,&project),"title":"Evidence","text":"RAW_SOURCE_CANARY"})).await["sourceId"].clone();
    Fixture {
        _directory: directory,
        host,
        project,
        other,
        source,
        work,
    }
}
async fn launch(f: &Fixture, index: usize) -> (Value, agent_memory::Ticket) {
    let mut target = f.host.snapshot().unwrap()["work"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["workItemId"] == f.work[index])
        .unwrap()
        .clone();
    let request = intent(
        "start_work",
        json!({"workItemId":target["workItemId"],"expectedItemRevision":target["revision"]}),
        &command(json!({})),
    );
    f.host.store.append(vec![event("command.requested",json!({"intent":request})),event("run.launching",json!({"intentId":request["intentId"],"workItemId":target["workItemId"],"expectedItemRevision":target["revision"],"ownerEpoch":f.host.epoch}))]).unwrap();
    target["runId"] = json!(format!("run:{}", request["intentId"].as_str().unwrap()));
    let ticket = f.host.prepare_agent_memory(&target).await.unwrap().unwrap();
    (target, ticket)
}
fn credentials(ticket: &agent_memory::Ticket) -> Value {
    serde_json::from_slice(&crate::store::read_private_file(&ticket.path).unwrap()).unwrap()
}
async fn rpc(ticket: &Value, tool: &str, args: Value) -> Value {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(4))
        .build()
        .unwrap()
        .post(ticket["endpoint"].as_str().unwrap())
        .bearer_auth(ticket["bearer"].as_str().unwrap())
        .json(&json!({"schemaVersion":1,"tool":tool,"arguments":args}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test]
async fn two_real_worktrees_share_current_accepted_memory_and_never_raw_sources() {
    let f = fixture().await;
    let first = decision(&f.host, &f.project, &f.source, "Money rule", true).await;
    let second = decision(&f.host, &f.project, &f.source, "Other rule", true).await;
    let draft = decision(&f.host, &f.project, &f.source, "Unapproved rule", false).await;
    let (target, a) = launch(&f, 0).await;
    let (_, b) = launch(&f, 1).await;
    let ca = credentials(&a);
    let cb = credentials(&b);
    let operation = f.host.operations.lock().await;
    let early = rpc(&ca, "decision_read", json!({"decisionId":first})).await;
    assert!(early.to_string().contains("AGENT_MEMORY_CONTEXT_REQUIRED"));
    let page = rpc(&ca, "project_context", json!({"limit":1})).await;
    drop(operation);
    assert_eq!(page["total"], 2, "{page}");
    assert_eq!(page["nextOffset"], 1);
    assert!(rpc(&ca, "decision_read", json!({"decisionId":first}))
        .await
        .to_string()
        .contains("CONTEXT_REQUIRED"));
    let completed = rpc(
        &ca,
        "project_context",
        json!({"offset":1,"limit":1,"revision":page["decisionRevision"]}),
    )
    .await;
    assert!(completed["nextOffset"].is_null());
    let whole = rpc(&cb, "project_context", json!({})).await;
    assert_eq!(whole["total"], 2);
    let exact = rpc(&ca, "decision_read", json!({"decisionId":first})).await;
    assert_eq!(
        exact["decision"]["decision"], "Use integer minor units",
        "{exact}"
    );
    assert_eq!(
        rpc(&cb, "decision_read", json!({"decisionId":first})).await,
        exact
    );
    assert!(!exact.to_string().contains("RAW_SOURCE_CANARY"));
    assert!(!exact.to_string().contains("origin"));
    assert!(!exact.to_string().contains("history"));
    assert!(rpc(&ca, "decision_read", json!({"decisionId":draft}))
        .await
        .to_string()
        .contains("DECISION_UNAVAILABLE"));
    assert!(rpc(&ca, "project_context", json!({"projectId":f.other}))
        .await
        .to_string()
        .contains("INVALID"));
    let found = rpc(&ca, "memory_search", json!({"query":"integer"})).await;
    assert!(found.to_string().contains(first.as_str().unwrap()));
    assert!(!found.to_string().contains("RAW_SOURCE_CANARY"));
    rpc(&ca, "memory_search", json!({"query":"SEARCH_QUERY_CANARY"})).await;
    let foreign_source = apply(&f.host,"project_message",json!({"projectId":f.other,"expectedProjectRevision":project_revision(&f.host,&f.other),"title":"Foreign evidence","text":"FOREIGN_SOURCE_CANARY"})).await["sourceId"].clone();
    let foreign = decision(
        &f.host,
        &f.other,
        &foreign_source,
        "Foreign accepted ADR",
        true,
    )
    .await;
    assert!(rpc(&ca, "decision_read", json!({"decisionId":foreign}))
        .await
        .to_string()
        .contains("DECISION_UNAVAILABLE"));
    apply(&f.host,"project_decision_supersede",json!({"projectId":f.project,"decisionId":second,"expectedDecisionRevision":2,"replacementDecisionId":draft,"expectedReplacementRevision":1})).await;
    assert!(rpc(&ca, "decision_read", json!({"decisionId":first}))
        .await
        .to_string()
        .contains("CONTEXT_REQUIRED"));
    let refreshed = rpc(&ca, "project_context", json!({})).await;
    assert_eq!(refreshed["stale"], true);
    assert_eq!(refreshed["total"], 2);
    assert!(rpc(&ca, "decision_read", json!({"decisionId":second}))
        .await
        .to_string()
        .contains("DECISION_UNAVAILABLE"));
    let snapshot = f.host.snapshot().unwrap();
    let context = snapshot["agentMemory"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["runId"] == target["runId"])
        .unwrap();
    assert_eq!(context["stale"], true);
    assert_eq!(context["manifestLoaded"], true);
    let facts = std::fs::read_to_string(f.host.data_dir.join("events.jsonl")).unwrap();
    assert!(!facts.contains(ca["bearer"].as_str().unwrap()));
    assert!(!facts.contains("memory_search"));
    assert!(!facts.contains("SEARCH_QUERY_CANARY"));
    for version in [1, 2] {
        let outgoing = crate::cloud::build_projection(&snapshot, version, 1, "install:test")
            .unwrap()
            .to_string();
        for forbidden in [
            "agentMemory",
            ca["bearer"].as_str().unwrap(),
            "SEARCH_QUERY_CANARY",
            "RAW_SOURCE_CANARY",
        ] {
            assert!(!outgoing.contains(forbidden));
        }
    }
    a.active.store(false, Ordering::SeqCst);
    assert!(rpc(&ca, "project_context", json!({}))
        .await
        .to_string()
        .contains("ACCESS_DENIED"));
    f.host.shutdown().await;
}

#[tokio::test]
async fn empty_manifest_is_valid_but_canonical_start_failure_and_shutdown_revoke() {
    let f = fixture().await;
    let (target, ticket) = launch(&f, 0).await;
    let credential = credentials(&ticket);
    let context = rpc(&credential, "project_context", json!({})).await;
    assert_eq!(context["total"], 0, "{context}");
    let id = target["runId"]
        .as_str()
        .unwrap()
        .strip_prefix("run:")
        .unwrap();
    f.host.store.append(vec![event("run.start_unknown",json!({"intentId":id,"workItemId":target["workItemId"],"expectedItemRevision":target["revision"],"ownerEpoch":f.host.epoch}))]).unwrap();
    assert!(rpc(&credential, "project_context", json!({}))
        .await
        .to_string()
        .contains("SCOPE_DENIED"));
    f.host.request_shutdown();
    assert!(!ticket.active.load(Ordering::SeqCst));
    f.host.shutdown().await;
}

#[tokio::test]
#[ignore = "requires an explicitly isolated installed Codex wrapper and built Rust MCP executable; never starts a model turn"]
async fn installed_codex_uses_real_rust_host_from_two_worktrees_without_inference() {
    let binary = PathBuf::from(
        std::env::var_os("PATCHFLEET_TEST_MEMORY_BINARY").expect("explicit built CLI"),
    );
    let codex = PathBuf::from(
        std::env::var_os("PATCHFLEET_TEST_CODEX").expect("explicit isolated provider wrapper"),
    );
    assert!(binary.is_absolute() && codex.is_absolute());
    let f = fixture().await;
    let accepted = decision(&f.host, &f.project, &f.source, "Money rule", true).await;
    let controller = providers::Controller::with_programs(providers::Programs {
        codex,
        claude: "unused".into(),
        gemini: "unused".into(),
        timeout: Duration::from_secs(15),
    });
    let mut leases = Vec::new();
    for index in 0..2 {
        let (target, ticket) = launch(&f, index).await;
        let credential = credentials(&ticket);
        let mut config = ticket.config.clone();
        config["mcp_servers"]["patchfleet_memory"]["command"] = json!(binary);
        let memory = providers::MemoryConfig {
            config,
            active: ticket.active.clone(),
        };
        let session = controller
            .prepare_with_memory(&target, &json!({"runId":target["runId"]}), Some(memory))
            .await
            .unwrap();
        assert!(!session.is_empty());
        assert!(
            !ticket.path.exists(),
            "first real preflight consumes bootstrap"
        );
        let exact = rpc(&credential, "decision_read", json!({"decisionId":accepted})).await;
        assert_eq!(
            exact["decision"]["decision"], "Use integer minor units",
            "{exact}"
        );
        let snapshot = f.host.snapshot().unwrap();
        assert!(snapshot["agentMemory"]["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["runId"] == target["runId"] && c["manifestLoaded"] == true));
        leases.push(ticket);
    }
    controller.shutdown().await;
    assert!(leases
        .iter()
        .all(|ticket| !ticket.active.load(Ordering::SeqCst)));
    f.host.shutdown().await;
}

#[tokio::test]
async fn team_role_and_retry_keep_original_project_after_folder_relink() {
    let mut f = fixture().await;
    let accepted = decision(&f.host, &f.project, &f.source, "Original team rule", true).await;
    let workspace = f.host.snapshot().unwrap()["workspaces"]["items"][0]["workspaceId"].clone();
    let input = json!({"name":"Memory team","goal":"Follow accepted decisions","workspaceId":workspace,"templateId":"product-feature","orchestratorPackId":"pack:orchestrator","workerPackIds":["pack:backend"],"settings":{"concurrency":1,"retryLimit":1,"timeoutMinutes":10,"failurePolicy":"continue"}});
    let team = f
        .host
        .action("team_create", command(input.clone()))
        .await
        .unwrap()["teamId"]
        .clone();
    apply(&f.host,"project_unlink_workspace",json!({"projectId":f.project,"workspaceId":workspace,"expectedProjectRevision":project_revision(&f.host,&f.project)})).await;
    // A team created while unlinked must stay unbound even after a later link.
    let mut unbound_input = input;
    unbound_input["name"] = json!("Unbound team");
    unbound_input["settings"]["retryLimit"] = json!(0);
    let unbound = f
        .host
        .action("team_create", command(unbound_input))
        .await
        .unwrap()["teamId"]
        .clone();
    apply(&f.host,"project_link_workspace",json!({"projectId":f.other,"workspaceId":workspace,"expectedProjectRevision":project_revision(&f.host,&f.other)})).await;
    let owner_sources = f
        .host
        .store
        .read_memory(&json!({"projectId":f.project}))
        .unwrap()["total"]
        .clone();
    for id in [&team, &unbound] {
        f.host
            .team_event("team.started", json!({"teamId":id}))
            .unwrap();
        f.host.advance_team(id).await.unwrap();
    }
    let find_work = |id: &Value| {
        f.host.snapshot().unwrap()["teams"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["teamId"] == *id)
            .unwrap()["tasks"][0]["workItemId"]
            .clone()
    };
    let role = find_work(&team);
    let unbound_role = find_work(&unbound);
    let cursor = f.host.store.event_cursor().unwrap();
    for id in [&team, &unbound] {
        f.host.advance_team(id).await.unwrap();
    }
    assert_eq!(
        f.host.store.event_cursor().unwrap(),
        cursor,
        "failed provider starts must not loop on automatic team advancement"
    );
    assert_eq!(
        f.host
            .store
            .subject_project("work", role.as_str().unwrap())
            .unwrap()
            .as_deref(),
        f.project.as_str()
    );
    assert!(f
        .host
        .store
        .subject_project("work", unbound_role.as_str().unwrap())
        .unwrap()
        .is_none());
    f.work.push(role);
    let (target, ticket) = launch(&f, 2).await;
    let credential = credentials(&ticket);
    assert_eq!(
        rpc(&credential, "project_context", json!({})).await["projectId"],
        f.project
    );
    assert_eq!(
        rpc(&credential, "decision_read", json!({"decisionId":accepted})).await["decision"]
            ["title"],
        "Original team rule"
    );
    let intent_id = target["runId"]
        .as_str()
        .unwrap()
        .strip_prefix("run:")
        .unwrap();
    let session = format!("session:{}", uuid::Uuid::new_v4());
    f.host.store.append(vec![
        event("run.prepared",json!({"intentId":intent_id,"workItemId":target["workItemId"],"providerId":"codex","providerSessionId":session,"expectedItemRevision":target["revision"],"ownerEpoch":f.host.epoch})),
        event("turn.requested",json!({"intentId":intent_id,"providerSessionId":session,"ownerEpoch":f.host.epoch})),
        event("run.started",json!({"intentId":intent_id,"expectedItemRevision":target["revision"],"ownerEpoch":f.host.epoch,"run":{"schemaVersion":1,"runId":target["runId"],"workItemId":target["workItemId"],"providerId":"codex","providerSessionId":session,"providerTurnId":"turn:test","ownerEpoch":f.host.epoch,"status":"running","revision":1,"startedAt":now(),"terminalAt":null}})),
        event("session.observed",json!({"providerId":"codex","providerSessionId":session,"status":"running","createdAt":now(),"lastObservedAt":now()})),
        event("session.terminal",json!({"providerId":"codex","providerSessionId":session,"status":"failed","lastObservedAt":now()})),
    ]).unwrap();
    assert!(rpc(&credential, "project_context", json!({}))
        .await
        .to_string()
        .contains("SCOPE_DENIED"));
    let after = f.host.advance_team(&team).await.unwrap();
    assert_eq!(after["tasks"][0]["attempt"], 2);
    let retry = after["tasks"][0]["workItemId"].clone();
    assert_ne!(retry, target["workItemId"]);
    assert_eq!(
        f.host
            .store
            .subject_project("work", retry.as_str().unwrap())
            .unwrap()
            .as_deref(),
        f.project.as_str()
    );
    f.work.push(retry);
    let (_, retry_ticket) = launch(&f, 3).await;
    assert_eq!(
        rpc(&credentials(&retry_ticket), "project_context", json!({})).await["projectId"],
        f.project
    );
    assert_eq!(
        f.host
            .store
            .read_memory(&json!({"projectId":f.project}))
            .unwrap()["total"],
        owner_sources,
        "generated role contracts must not become owner evidence"
    );
    f.host.shutdown().await;
}
