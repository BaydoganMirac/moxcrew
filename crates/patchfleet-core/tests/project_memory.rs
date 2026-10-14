use patchfleet_core::{domain, event, now, project_memory as memory, store::Store};
use serde_json::{json, Value};
use tempfile::tempdir;

fn command(mut p: Value) -> Value {
    p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    p
}
fn apply(store: &Store, action: &str, p: Value) -> Value {
    store
        .transaction(|events| memory::apply(events, action, &p))
        .unwrap()
}
fn project_id(store: &Store, name: &str) -> String {
    apply(store, "project_create", command(json!({"name":name})))["projectId"]
        .as_str()
        .unwrap()
        .into()
}
fn revision(store: &Store, project: &str) -> Value {
    store.runtime_projections().unwrap()["projectMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == project)
        .unwrap()["revision"]
        .clone()
}
fn link(store: &Store, project: &str, workspace: &str, action: &str) -> Value {
    apply(
        store,
        action,
        command(
            json!({"projectId":project,"workspaceId":workspace,"expectedProjectRevision":revision(store,project)}),
        ),
    )
}
fn workspace_events(name: &str) -> (String, Vec<Value>) {
    let workspace = format!("workspace:{}", uuid::Uuid::new_v4());
    let cmd = format!("cmd:{}", uuid::Uuid::new_v4());
    let at = now();
    let w = json!({"schemaVersion":1,"workspaceId":workspace,"displayName":name,"workingDirectory":format!("/fixture/{}",uuid::Uuid::new_v4()),"createdAt":at,"revision":1});
    let intent = json!({"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"type":"register_workspace","actorId":"local-owner","createdAt":at,"expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"payload":{"workspace":w}});
    (
        workspace.clone(),
        vec![
            event("workspace.command.requested", json!({"intent":intent})),
            event(
                "workspace.registered",
                json!({"intentId":cmd,"workspace":w}),
            ),
            event(
                "workspace.command.receipted",
                json!({"receipt":{"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"commandType":"register_workspace","outcome":"applied","reasonCode":"WORKSPACE_REGISTERED","completedAt":at,"workspaceProjectionRevision":1,"workspaceId":workspace}}),
            ),
        ],
    )
}
fn register(store: &Store, name: &str) -> String {
    let (workspace, mut additions) = workspace_events(name);
    let count = store.projections().unwrap()["workspaces"]["revision"]
        .as_u64()
        .unwrap_or(0)
        + 1;
    additions[2]["payload"]["receipt"]["workspaceProjectionRevision"] = json!(count);
    store.append(additions).unwrap();
    workspace
}
fn work_events(work_id: &str, instruction: &str, revision: u64) -> Vec<Value> {
    let cmd = format!("cmd:{}", uuid::Uuid::new_v4());
    let at = now();
    let work = json!({"schemaVersion":1,"workItemId":work_id,"title":"Owner work","instruction":instruction,"providerId":"codex","workingDirectory":"/fixture/project","status":"queued","createdAt":at,"revision":1});
    let intent = json!({"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"type":"enqueue_work","actorId":"local-owner","createdAt":at,"expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"payload":{"workItem":work}});
    vec![
        event("command.requested", json!({"intent":intent})),
        event("work.enqueued", json!({"intentId":cmd,"workItem":work})),
        event(
            "command.receipted",
            json!({"receipt":{"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"commandType":"enqueue_work","outcome":"applied","reasonCode":"WORK_ENQUEUED","completedAt":at,"workProjectionRevision":revision,"workItemId":work_id}}),
        ),
    ]
}
fn page(store: &Store, project: &str) -> Value {
    store
        .read_memory(&json!({"projectId":project,"limit":20}))
        .unwrap()
}
fn message(store: &Store, project: &str, title: &str, text: &str) -> Value {
    apply(
        store,
        "project_message",
        command(
            json!({"projectId":project,"title":title,"text":text,"expectedProjectRevision":revision(store,project)}),
        ),
    )
}

#[test]
fn two_workspaces_share_one_project_and_competing_links_reject_without_mutation() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let a = project_id(&store, "A");
    let b = project_id(&store, "B");
    let w1 = register(&store, "First");
    let w2 = register(&store, "Second");
    assert_eq!(
        link(&store, &a, &w1, "project_link_workspace")["outcome"],
        "applied"
    );
    assert_eq!(
        link(&store, &a, &w2, "project_link_workspace")["outcome"],
        "applied"
    );
    let before = store.projections().unwrap()["projectMemory"].clone();
    let denied = link(&store, &b, &w1, "project_link_workspace");
    assert_eq!(denied["reasonCode"], "WORKSPACE_LINKED_TO_OTHER_PROJECT");
    assert_eq!(store.projections().unwrap()["projectMemory"], before);
    assert_eq!(store.workspace_project(&w1).unwrap(), Some(a.clone()));
    assert_eq!(store.workspace_project(&w2).unwrap(), Some(a));
}

#[test]
fn message_edits_keep_identity_origin_and_immutable_event_history_across_restart() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let project = project_id(&store, "UTF-16 memory");
    let first = command(
        json!({"projectId":project,"title":"First","text":"OWNER_FIRST_CANARY","expectedProjectRevision":revision(&store,&project)}),
    );
    let receipt = apply(&store, "project_message", first.clone());
    let prior = page(&store, &project)["sources"][0].clone();
    assert_eq!(apply(&store, "project_message", first.clone()), receipt);
    assert_eq!(page(&store, &project)["total"], 1);
    let edit = command(
        json!({"projectId":project,"sourceId":receipt["sourceId"],"title":"Updated","text":"OWNER_EDIT_CANARY","expectedSourceRevision":1}),
    );
    let edited = apply(&store, "project_source_edit", edit.clone());
    assert_eq!(edited["sourceRevision"], 2);
    let current = page(&store, &project)["sources"][0].clone();
    assert_eq!(current["origin"], prior["origin"]);
    assert_eq!(current["createdAt"], prior["createdAt"]);
    assert_eq!(current["sourceId"], prior["sourceId"]);
    assert_eq!(current["text"], "OWNER_EDIT_CANARY");
    let events = store.read_events().unwrap();
    assert!(events.iter().any(|e| e["type"] == "project.source.created"
        && e["payload"]["source"]["text"] == "OWNER_FIRST_CANARY"));
    drop(store);
    let restored = Store::open(dir.path().into()).unwrap();
    assert_eq!(page(&restored, &project)["sources"][0], current);
    assert_eq!(apply(&restored, "project_message", first), receipt);
    assert_eq!(apply(&restored, "project_source_edit", edit), edited);
}

#[test]
fn stale_and_cross_project_edits_leave_source_state_untouched() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let a = project_id(&store, "A");
    let b = project_id(&store, "B");
    let source = message(&store, &a, "Title", "Evidence")["sourceId"].clone();
    let before = store.projections().unwrap()["projectMemory"].clone();
    for (project, expected, reason) in [
        (&a, 2, "STALE_SOURCE_REVISION"),
        (&b, 1, "SOURCE_NOT_IN_PROJECT"),
    ] {
        let receipt = apply(
            &store,
            "project_source_edit",
            command(
                json!({"projectId":project,"sourceId":source,"title":"Tampered","text":"Denied","expectedSourceRevision":expected}),
            ),
        );
        assert_eq!(receipt["outcome"], "rejected");
        assert_eq!(receipt["reasonCode"], reason);
        assert_eq!(store.projections().unwrap()["projectMemory"], before);
    }
    let rejected = apply(
        &store,
        "project_message",
        command(json!({"projectId":a,"title":"Stale","text":"Denied","expectedProjectRevision":1})),
    );
    assert_eq!(rejected["reasonCode"], "STALE_PROJECT_REVISION");
    assert_eq!(store.projections().unwrap()["projectMemory"], before);
}

#[test]
fn explicit_subject_binding_captures_once_and_survives_workspace_relink() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let a = project_id(&store, "A");
    let b = project_id(&store, "B");
    let w = register(&store, "Shared");
    link(&store, &a, &w, "project_link_workspace");
    store
        .append(vec![memory::bind(&a, "work", "work:pending")])
        .unwrap();
    assert_eq!(
        page(&store, &a)["total"],
        0,
        "binding cannot fabricate owner input"
    );
    link(&store, &a, &w, "project_unlink_workspace");
    link(&store, &b, &w, "project_link_workspace");
    let work = work_events("work:pending", "Captured owner instruction", 1);
    let origin = work[1]["id"].clone();
    store.append(work).unwrap();
    let captured = page(&store, &a)["sources"][0].clone();
    assert_eq!(
        captured["sourceId"],
        format!("source:{}", origin.as_str().unwrap())
    );
    assert_eq!(captured["origin"]["eventId"], origin);
    assert_eq!(captured["origin"]["id"], "work:pending");
    assert_eq!(page(&store, &b)["total"], 0);
    assert_eq!(
        store.subject_project("work", "work:pending").unwrap(),
        Some(a.clone())
    );
    assert!(store
        .append(vec![memory::bind(&b, "work", "work:pending")])
        .is_err());
    let source = captured["sourceId"].clone();
    let denied = apply(
        &store,
        "project_source_edit",
        command(
            json!({"projectId":a,"sourceId":source,"title":"Edit","text":"No","expectedSourceRevision":1}),
        ),
    );
    assert_eq!(denied["reasonCode"], "SOURCE_IMMUTABLE");
    store
        .append(work_events(
            "work:generated-team",
            "Generated instructions must remain unbound",
            2,
        ))
        .unwrap();
    assert_eq!(page(&store, &a)["total"], 1);
    assert_eq!(page(&store, &b)["total"], 0);
}

#[test]
fn linked_workspace_removal_preserves_sources_and_subject_scope() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let project = project_id(&store, "Historic");
    let w = register(&store, "Removed");
    link(&store, &project, &w, "project_link_workspace");
    message(&store, &project, "Keep", "Historical input");
    store
        .append(vec![memory::bind(&project, "work", "work:historic")])
        .unwrap();
    let before = revision(&store, &project).as_u64().unwrap();
    let cmd = format!("cmd:{}", uuid::Uuid::new_v4());
    let at = now();
    let request = json!({"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"type":"remove_workspace","actorId":"local-owner","createdAt":at,"expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"payload":{"workspaceId":w,"expectedWorkspaceRevision":1}});
    store.append(vec![event("workspace.command.requested",json!({"intent":request})),event("workspace.removed",json!({"intentId":cmd,"workspaceId":w,"expectedWorkspaceRevision":1})),event("workspace.command.receipted",json!({"receipt":{"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"commandType":"remove_workspace","outcome":"applied","reasonCode":"WORKSPACE_REMOVED","completedAt":at,"workspaceProjectionRevision":2,"workspaceId":w}}))]).unwrap();
    assert_eq!(store.workspace_project(&w).unwrap(), None);
    assert_eq!(
        store.subject_project("work", "work:historic").unwrap(),
        Some(project.clone())
    );
    assert_eq!(revision(&store, &project), before + 1);
    assert_eq!(page(&store, &project)["total"], 1);
}

#[test]
fn every_pure_command_crash_prefix_recovers_once_and_receipts_remain_stable() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let project = project_id(&store, "Recovery");
    let a = register(&store, "A");
    let b = register(&store, "B");
    link(&store, &project, &a, "project_link_workspace");
    let source = message(&store, &project, "Original", "Message")["sourceId"].clone();
    let base = store.read_events().unwrap();
    let rev = revision(&store, &project);
    for (action, payload) in [
        ("project_create", json!({"name":"Recovered project"})),
        (
            "project_link_workspace",
            json!({"projectId":project,"workspaceId":b,"expectedProjectRevision":rev}),
        ),
        (
            "project_unlink_workspace",
            json!({"projectId":project,"workspaceId":a,"expectedProjectRevision":rev}),
        ),
        (
            "project_message",
            json!({"projectId":project,"title":"Recovered","text":"Recorded once","expectedProjectRevision":rev}),
        ),
        (
            "project_source_edit",
            json!({"projectId":project,"sourceId":source,"title":"Recovered edit","text":"Recorded once","expectedSourceRevision":1}),
        ),
    ] {
        let payload = command(payload);
        let (planned, _) = memory::apply(&base, action, &payload).unwrap();
        assert_eq!(planned.len(), 3);
        for prefix in 1..=3 {
            let mut events = base.clone();
            events.extend(planned[..prefix].iter().cloned());
            let recovered = memory::recover(&events).unwrap();
            events.extend(recovered);
            domain::project_events(&events).unwrap();
            assert!(memory::recover(&events).unwrap().is_empty());
            let (additions, receipt) = memory::apply(&events, action, &payload).unwrap();
            assert!(additions.is_empty());
            assert_eq!(receipt["outcome"], "applied");
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["type"] == "project.command.receipted"
                        && e["payload"]["receipt"]["commandId"] == payload["commandId"])
                    .count(),
                1
            );
        }
    }
}

#[test]
fn malformed_times_and_utf16_limits_fail_without_panics_or_writes() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let project = project_id(&store, "Limits");
    let before = store.read_events().unwrap();
    let bad = command(json!({"name":"Malformed","createdAt":"not-a-time"}));
    let result = std::panic::catch_unwind(|| memory::apply(&before, "project_create", &bad));
    assert!(result.is_ok());
    assert!(result.unwrap().is_err());
    let expected_revision = revision(&store, &project);
    for text in [
        "🦀".repeat(25_001),
        "control\u{0001}character".into(),
        " whitespace ".into(),
    ] {
        let result=store.transaction(|events|memory::apply(events,"project_message",&command(json!({"projectId":project,"title":"Limit","text":text,"expectedProjectRevision":expected_revision}))));
        assert!(result.is_err());
    }
    assert_eq!(store.read_events().unwrap(), before);
}

#[test]
fn keyword_pages_are_scoped_bounded_and_never_enter_snapshots_or_events() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    let a = project_id(&store, "A");
    let b = project_id(&store, "B");
    for i in 0..23 {
        message(
            &store,
            &a,
            &format!("Money {i}"),
            "OWNER_SEARCH_CANARY amount",
        );
    }
    message(&store, &b, "Money B", "OTHER_PROJECT_CANARY");
    let before = store.event_cursor().unwrap();
    let payload =
        command(json!({"projectId":a,"query":"owner_search_canary","limit":20,"offset":0}));
    let page = store.read_memory(&payload).unwrap();
    assert_eq!(page["total"], 23);
    assert_eq!(page["sources"].as_array().unwrap().len(), 20);
    assert_eq!(page["revision"], revision(&store, &a));
    let second = store
        .read_memory(&json!({"projectId":a,"query":"owner_search_canary","limit":20,"offset":20}))
        .unwrap();
    assert_eq!(second["sources"].as_array().unwrap().len(), 3);
    assert!(!second.to_string().contains("OTHER_PROJECT_CANARY"));
    assert_eq!(store.event_cursor().unwrap(), before);
    let snapshot = store.runtime_projections().unwrap();
    assert!(snapshot["projectMemory"].get("sources").is_none());
    assert!(snapshot["projectMemory"].get("bindings").is_none());
    assert!(!snapshot.to_string().contains("OWNER_SEARCH_CANARY"));
    assert!(store
        .read_memory(&json!({"projectId":a,"limit":21}))
        .is_err());
    assert!(store
        .read_memory(&json!({"projectId":a,"unexpected":"field"}))
        .is_err());
}

#[test]
fn version_isolation_and_forged_fact_provenance_fail_closed() {
    let (additions, _) =
        memory::apply(&[], "project_create", &command(json!({"name":"Strict"}))).unwrap();
    for version in [1, 2, 4] {
        let mut bad = additions[0].clone();
        bad["schemaVersion"] = json!(version);
        assert!(domain::validate_event(&bad).is_err());
    }
    let mut bad = additions.clone();
    bad[1]["payload"]["project"]["name"] = json!("Forged");
    assert!(domain::project_events(&bad).is_err());
    let mut bad = additions;
    bad[2]["payload"]["receipt"]["projectRevision"] = json!(999);
    assert!(domain::project_events(&bad).is_err());
}
