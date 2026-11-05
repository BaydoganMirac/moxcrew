use patchfleet_core::{
    decisions, domain, project_memory as memory, shared_memory as shared, store::Store,
};
use serde_json::{json, Value};

fn command(mut p: Value) -> Value {
    p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    p
}
fn conn() -> Value {
    json!({"cloudUrl":"https://memory.example","workspaceId":"workspace:fixture","hostId":"host:fixture"})
}
fn memory_apply(events: &mut Vec<Value>, action: &str, p: Value) -> Value {
    let (next, r) = memory::apply(events, action, &command(p)).unwrap();
    events.extend(next);
    r
}
fn local_revision(events: &[Value], project: &Value) -> Value {
    memory::project(events).unwrap()["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()["revision"]
        .clone()
}
fn apply(events: &mut Vec<Value>, action: &str, p: &Value, c: &Value) -> Value {
    let (next, r) = shared::apply(events, action, p, c).unwrap();
    events.extend(next);
    r
}
fn fixture() -> (Vec<Value>, Value, Value, Value) {
    let mut events = Vec::new();
    let project = memory_apply(
        &mut events,
        "project_create",
        json!({"name":"Local private project"}),
    )["projectId"]
        .clone();
    let source=memory_apply(&mut events,"project_message",json!({"projectId":project,"expectedProjectRevision":1,"title":"Source private title","text":"PRIVATE_RAW_SOURCE_CANARY · /private/repository TOKEN_CANARY"}))["sourceId"].clone();
    let draft = draft(&mut events, &project, &source, "Accepted public title");
    let (next, _) = decisions::apply(
        &events,
        "project_decision_accept",
        &command(json!({"projectId":project,"decisionId":draft,"expectedDecisionRevision":1})),
    )
    .unwrap();
    events.extend(next);
    (events, project, source, draft)
}
fn draft(events: &mut Vec<Value>, project: &Value, source: &Value, title: &str) -> Value {
    let p = command(
        json!({"projectId":project,"expectedProjectRevision":local_revision(events,project),"title":title,"context":"Approved context","decision":"Approved decision","rationale":"Approved rationale","consequences":"Approved consequence","sources":[{"sourceId":source,"revision":1}]}),
    );
    let (next, r) = decisions::apply(events, "project_decision_propose", &p).unwrap();
    events.extend(next);
    r["decisionId"].clone()
}
fn projected(events: &[Value]) -> Value {
    domain::project_events(events).unwrap()["sharedMemory"].clone()
}
fn fake_receipt(op: &Value, revision: u64, role: &str, enabled: bool) -> Value {
    let p = &op["payload"];
    let shared = if p["type"] == "create_project" {
        json!(format!(
            "shared:{}",
            op["operationId"]
                .as_str()
                .unwrap()
                .strip_prefix("memory-op:")
                .unwrap()
        ))
    } else {
        p["sharedProjectId"].clone()
    };
    let reason = match p["type"].as_str().unwrap() {
        "create_project" => "MEMORY_PROJECT_CREATED",
        "link_reader" => "MEMORY_READER_LINKED",
        "publish" => "MEMORY_PUBLISHED",
        _ => "MEMORY_SHARING_UPDATED",
    };
    json!({"schemaVersion":1,"operationId":op["operationId"],"operationType":p["type"],"requestHash":shared::request_hash(op).unwrap(),"outcome":"applied","reasonCode":reason,"sharedProjectId":shared,"revision":revision,"role":role,"enabled":enabled,"completedAt":patchfleet_core::now()})
}
fn deliver(
    events: &mut Vec<Value>,
    project: &Value,
    c: &Value,
    revision: u64,
    role: &str,
    enabled: bool,
) -> Value {
    let (next, op) = shared::prepare_delivery(events, project.as_str().unwrap(), c).unwrap();
    events.extend(next);
    assert!(!op.is_null());
    let r = fake_receipt(&op, revision, role, enabled);
    events.push(shared::acknowledge(project.as_str().unwrap(), &op, &r).unwrap());
    domain::project_events(events).unwrap();
    op
}
fn setup(events: &mut Vec<Value>, project: &Value, c: &Value) -> Value {
    let r = apply(
        events,
        "memory_cloud_create",
        &command(
            json!({"projectId":project,"expectedProjectRevision":local_revision(events,project),"name":"Explicit shared alias"}),
        ),
        c,
    );
    assert_eq!(r["outcome"], "applied");
    deliver(events, project, c, 0, "publisher", true)
}
fn publish(project: &Value, decision: &Value, shared_revision: u64) -> Value {
    command(
        json!({"projectId":project,"decisionId":decision,"expectedDecisionRevision":2,"expectedSharedRevision":shared_revision,"consent":true}),
    )
}
fn supersede(events: &mut Vec<Value>, project: &Value, source: &Value, old: &Value) -> Value {
    let replacement = draft(events, project, source, "Replacement public title");
    let p = command(
        json!({"projectId":project,"decisionId":old,"expectedDecisionRevision":2,"replacementDecisionId":replacement,"expectedReplacementRevision":1}),
    );
    let (next, r) = decisions::apply(events, "project_decision_supersede", &p).unwrap();
    assert_eq!(r["outcome"], "applied");
    events.extend(next);
    replacement
}

#[test]
fn queued_consent_freezes_only_accepted_dto_and_acknowledgement_alone_publishes() {
    let (mut events, project, source, decision) = fixture();
    let c = conn();
    setup(&mut events, &project, &c);
    let p = publish(&project, &decision, 0);
    let r = apply(&mut events, "memory_cloud_publish", &p, &c);
    assert_eq!(r["reasonCode"], "MEMORY_OPERATION_QUEUED");
    let before = projected(&events);
    assert_eq!(before["mappings"][0]["sharedRevision"], 0);
    assert!(before["projects"][0]["pendingOperation"].is_object());
    let op = deliver(&mut events, &project, &c, 1, "publisher", true);
    let dto = &op["payload"]["decision"];
    assert_eq!(dto["decisionId"], decision);
    assert_eq!(dto["revision"], 2);
    assert!(dto["acceptedAt"].as_str().is_some());
    assert_eq!(dto["sources"], json!([{"sourceId":source,"revision":1}]));
    for denied in [
        "PRIVATE_RAW_SOURCE_CANARY",
        "/private/repository",
        "TOKEN_CANARY",
        "origin",
        "projectId",
        "credential",
    ] {
        assert!(!op.to_string().contains(denied), "{denied}");
    }
    assert_eq!(projected(&events)["mappings"][0]["sharedRevision"], 1);
    assert!(projected(&events)["projects"][0]["pendingOperation"].is_null());
    let n = events.len();
    assert_eq!(
        apply(&mut events, "memory_cloud_publish", &p, &Value::Null),
        r
    );
    assert_eq!(events.len(), n);
    let mut changed = p;
    changed["consent"] = json!(false);
    assert!(shared::apply(&events, "memory_cloud_publish", &changed, &c).is_err());
}

#[test]
fn every_owner_action_crash_prefix_recovers_one_queue_fact_and_one_original_receipt() {
    for action in [
        "memory_cloud_create",
        "memory_cloud_link",
        "memory_cloud_publish",
        "memory_cloud_sharing",
        "memory_cloud_sync",
    ] {
        let (mut base, project, _, decision) = fixture();
        let c = conn();
        if !["memory_cloud_create", "memory_cloud_link"].contains(&action) {
            setup(&mut base, &project, &c);
        }
        let p = match action {
            "memory_cloud_create" => command(
                json!({"projectId":project,"expectedProjectRevision":local_revision(&base,&project),"name":"Shared"}),
            ),
            "memory_cloud_link" => command(
                json!({"projectId":project,"expectedProjectRevision":local_revision(&base,&project),"sharedProjectId":format!("shared:{}",uuid::Uuid::new_v4())}),
            ),
            "memory_cloud_publish" => publish(&project, &decision, 0),
            "memory_cloud_sharing" => {
                command(json!({"projectId":project,"expectedSharedRevision":0,"enabled":false}))
            }
            _ => command(json!({"projectId":project})),
        };
        let (next, receipt) = shared::apply(&base, action, &p, &c).unwrap();
        assert_eq!(next.len(), 3);
        for prefix in 0..=3 {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::open(dir.path().into()).unwrap();
            let mut input = base.clone();
            input.extend_from_slice(&next[..prefix]);
            store.append(input).unwrap();
            store
                .transaction(|events| Ok((shared::recover(events)?, Value::Null)))
                .unwrap();
            let actual = store
                .transaction(|events| shared::apply(events, action, &p, &c))
                .unwrap();
            assert_eq!(
                actual["outcome"], receipt["outcome"],
                "{action} prefix {prefix}"
            );
            let events = store.read_events().unwrap();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["schemaVersion"] == 5
                        && e["payload"]["commandId"] == p["commandId"]
                        && [
                            "shared_memory.operation.queued",
                            "shared_memory.sync.queued"
                        ]
                        .contains(&e["type"].as_str().unwrap_or("")))
                    .count(),
                1
            );
            assert!(shared::recover(&events).unwrap().is_empty());
            assert_eq!(
                store.projections().unwrap(),
                domain::project_events(&events).unwrap()
            );
        }
    }
}

#[test]
fn first_attempt_rechecks_local_revision_but_uncertain_attempt_retries_original_dto() {
    for attempted in [false, true] {
        let (mut events, project, source, decision) = fixture();
        let c = conn();
        setup(&mut events, &project, &c);
        apply(
            &mut events,
            "memory_cloud_publish",
            &publish(&project, &decision, 0),
            &c,
        );
        let op = if attempted {
            let (next, op) =
                shared::prepare_delivery(&events, project.as_str().unwrap(), &c).unwrap();
            events.extend(next);
            Some(op)
        } else {
            None
        };
        supersede(&mut events, &project, &source, &decision);
        let (next, retry) =
            shared::prepare_delivery(&events, project.as_str().unwrap(), &c).unwrap();
        if attempted {
            assert!(next.is_empty());
            assert_eq!(retry, op.unwrap());
            assert_eq!(retry["payload"]["decision"]["decisionId"], decision);
            let r = fake_receipt(&retry, 1, "publisher", true);
            events.push(shared::acknowledge(project.as_str().unwrap(), &retry, &r).unwrap());
            assert_eq!(projected(&events)["projects"][0]["diverged"], true);
        } else {
            assert!(retry.is_null());
            assert_eq!(next[0]["type"], "shared_memory.operation.cancelled");
            events.extend(next);
            assert_eq!(
                projected(&events)["projects"][0]["lastErrorCode"],
                "STALE_DECISION_REVISION"
            );
        }
        domain::project_events(&events).unwrap();
    }
}

#[test]
fn connection_changes_never_replay_consent_and_explicit_remapping_retires_uncertain_slot() {
    let (mut events, project, _, decision) = fixture();
    let c = conn();
    setup(&mut events, &project, &c);
    apply(
        &mut events,
        "memory_cloud_publish",
        &publish(&project, &decision, 0),
        &c,
    );
    let (next, old) = shared::prepare_delivery(&events, project.as_str().unwrap(), &c).unwrap();
    events.extend(next);
    let mut other = c.clone();
    other["workspaceId"] = json!("workspace:other");
    other["hostId"] = json!("host:other");
    assert_eq!(
        shared::prepare_delivery(&events, project.as_str().unwrap(), &other)
            .unwrap_err()
            .0,
        "MEMORY_CONNECTION_CHANGED"
    );
    let r = apply(
        &mut events,
        "memory_cloud_sharing",
        &command(json!({"projectId":project,"expectedSharedRevision":0,"enabled":false})),
        &other,
    );
    assert_eq!(r["reasonCode"], "MEMORY_CONNECTION_CHANGED");
    let rev = local_revision(&events, &project);
    let r = apply(
        &mut events,
        "memory_cloud_link",
        &command(
            json!({"projectId":project,"expectedProjectRevision":rev,"sharedProjectId":format!("shared:{}",uuid::Uuid::new_v4())}),
        ),
        &other,
    );
    assert_eq!(r["outcome"], "applied");
    let p = projected(&events);
    let old = p["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["operation"]["operationId"] == old["operationId"])
        .unwrap();
    assert_eq!(old["state"], "unknown");
    assert_eq!(old["lastErrorCode"], "MEMORY_CONNECTION_CHANGED");
    let (next, new) = shared::prepare_delivery(&events, project.as_str().unwrap(), &other).unwrap();
    events.extend(next);
    assert_eq!(new["hostId"], "host:other");
    assert_ne!(new["operationId"], old["operation"]["operationId"]);
    domain::project_events(&events).unwrap();
}

#[test]
fn strict_protocol_hashes_scope_receipts_and_forged_facts_fail_closed() {
    let (mut events, project, _, decision) = fixture();
    let c = conn();
    let setup_op = setup(&mut events, &project, &c);
    assert_eq!(
        shared::request_hash(&json!({"b":{"z":2,"a":1},"a":[{"z":3,"a":2}]})).unwrap(),
        shared::request_hash(&json!({"a":[{"a":2,"z":3}],"b":{"a":1,"z":2}})).unwrap()
    );
    let p = publish(&project, &decision, 0);
    let (next, _) = shared::apply(&events, "memory_cloud_publish", &p, &c).unwrap();
    for i in 0..5 {
        let mut forged = next.clone();
        match i {
            0 => {
                forged[1]["payload"]["operation"]["payload"]["decision"]["context"] =
                    json!("forged")
            }
            1 => forged[1]["payload"]["operation"]["hostId"] = json!("host:other"),
            2 => {
                forged[1]["payload"]["operation"]["payload"]["decision"]["sources"][0]["text"] =
                    json!("raw")
            }
            3 => forged[0]["payload"]["actorId"] = json!("cloud-owner"),
            _ => forged[2]["payload"]["receipt"]["reasonCode"] = json!("MEMORY_PUBLISHED"),
        };
        let mut input = events.clone();
        input.extend(forged);
        assert!(domain::project_events(&input).is_err(), "{i}");
    }
    let mut r = fake_receipt(&setup_op, 0, "publisher", true);
    r["requestHash"] = json!("0".repeat(64));
    assert!(shared::validate_receipt(&r, &setup_op).is_err());
    let mut invalid = setup_op;
    invalid["payload"]["credential"] = json!("never");
    assert!(shared::validate_operation(&invalid).is_err());
    let mut input = events.clone();
    input.push(
        shared::acknowledge(
            project.as_str().unwrap(),
            &next[1]["payload"]["operation"],
            &fake_receipt(&next[1]["payload"]["operation"], 1, "publisher", true),
        )
        .unwrap(),
    );
    assert!(domain::project_events(&input).is_err());
}

#[test]
fn stale_scope_unpaired_and_pending_operations_reject_without_false_publication() {
    let (mut events, project, source, decision) = fixture();
    let c = conn();
    let p = command(
        json!({"projectId":project,"expectedProjectRevision":local_revision(&events,&project),"name":"Shared"}),
    );
    assert_eq!(
        apply(&mut events, "memory_cloud_create", &p, &Value::Null)["reasonCode"],
        "CLOUD_UNPAIRED"
    );
    setup(&mut events, &project, &c);
    let mut stale = publish(&project, &decision, 1);
    assert_eq!(
        apply(&mut events, "memory_cloud_publish", &stale, &c)["reasonCode"],
        "MEMORY_STALE_REVISION"
    );
    stale["expectedSharedRevision"] = json!(0);
    stale["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    assert_eq!(
        apply(&mut events, "memory_cloud_publish", &stale, &c)["outcome"],
        "applied"
    );
    assert_eq!(
        apply(
            &mut events,
            "memory_cloud_sharing",
            &command(json!({"projectId":project,"expectedSharedRevision":0,"enabled":false})),
            &c
        )["reasonCode"],
        "MEMORY_OPERATION_PENDING"
    );
    let second = memory_apply(
        &mut events,
        "project_create",
        json!({"name":"Other project"}),
    )["projectId"]
        .clone();
    let p = publish(&second, &decision, 0);
    assert_eq!(
        apply(&mut events, "memory_cloud_publish", &p, &c)["reasonCode"],
        "MEMORY_NOT_LINKED"
    );
    let p = memory_apply(
        &mut events,
        "project_source_edit",
        json!({"projectId":project,"sourceId":source,"expectedSourceRevision":1,"title":"Changed source","text":"Changed after acceptance"}),
    );
    assert_eq!(p["outcome"], "applied");
    let (next, op) = shared::prepare_delivery(&events, project.as_str().unwrap(), &c).unwrap();
    assert!(!op.is_null());
    events.extend(next);
    domain::project_events(&events).unwrap();
}

#[test]
fn reader_cache_is_complete_scoped_immutable_bounded_and_never_local_authority() {
    let (mut events, project, _, decision) = fixture();
    let c = conn();
    setup(&mut events, &project, &c);
    apply(
        &mut events,
        "memory_cloud_publish",
        &publish(&project, &decision, 0),
        &c,
    );
    let op = deliver(&mut events, &project, &c, 1, "publisher", true);
    let projection = projected(&events);
    let shared_id = projection["mappings"][0]["sharedProjectId"].clone();
    let dto = op["payload"]["decision"].clone();
    let at = patchfleet_core::now();
    let head = json!({"decisionId":decision,"revision":2,"title":dto["title"],"publishedAt":at});
    let manifest = json!({"revision":1,"decisions":[head]});
    let record = json!({"schemaVersion":1,"sharedProjectId":shared_id,"manifestRevision":1,"current":true,"publishedAt":at,"decision":dto});
    let empty = shared::empty_reader();
    assert!(shared::merge_reader(
        &empty,
        &projection,
        project.as_str().unwrap(),
        &c,
        &manifest,
        &[]
    )
    .is_err());
    let reader = shared::merge_reader(
        &empty,
        &projection,
        project.as_str().unwrap(),
        &c,
        &manifest,
        std::slice::from_ref(&record),
    )
    .unwrap();
    shared::validate_reader(&reader).unwrap();
    let page = shared::read_list(&projection, &reader, &c, &json!({"projectId":project})).unwrap();
    assert_eq!(page["total"], 1);
    assert_eq!(page["stale"], false);
    assert!(!page.to_string().contains("Approved decision"));
    let exact = shared::read_detail(
        &projection,
        &reader,
        &c,
        &json!({"projectId":project,"decisionId":decision,"revision":2}),
    )
    .unwrap();
    assert_eq!(exact["decision"], record["decision"]);
    let mut bad = record.clone();
    bad["decision"]["decision"] = json!("Changed immutable body");
    assert!(shared::merge_reader(
        &reader,
        &projection,
        project.as_str().unwrap(),
        &c,
        &manifest,
        &[bad]
    )
    .is_err());
    let reader = shared::merge_reader(
        &reader,
        &projection,
        project.as_str().unwrap(),
        &c,
        &json!({"revision":2,"decisions":[]}),
        &[],
    )
    .unwrap();
    let exact = shared::read_detail(
        &projection,
        &reader,
        &Value::Null,
        &json!({"projectId":project,"decisionId":decision,"revision":2}),
    )
    .unwrap();
    assert_eq!(exact["current"], false);
    assert_eq!(exact["offline"], true);
    assert_eq!(exact["stale"], true);
    let before = domain::project_events(&events).unwrap()["projectDecisions"].clone();
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    store.append(events.clone()).unwrap();
    store
        .commit_shared_cache(project.as_str().unwrap(), &c, &manifest, &[record])
        .unwrap();
    assert_eq!(store.projections().unwrap()["projectDecisions"], before);
    let revision = store.shared_metadata(&c).unwrap()["revision"].clone();
    let cache_path = dir.path().join("shared-memory-reader.json");
    let modified = std::fs::metadata(&cache_path).unwrap().modified().unwrap();
    store
        .read_shared_memory(&json!({"projectId":project,"commandId":"ignored"}), &c)
        .unwrap();
    assert_eq!(
        modified,
        std::fs::metadata(&cache_path).unwrap().modified().unwrap()
    );
    drop(store);
    let reopened = Store::open(dir.path().into()).unwrap();
    assert_eq!(reopened.shared_metadata(&c).unwrap()["revision"], revision);
    assert_eq!(
        reopened
            .read_shared_decision(
                &json!({"projectId":project,"decisionId":decision,"revision":2}),
                &c
            )
            .unwrap()["decision"],
        exact["decision"]
    );
}

fn shift(events: &mut [Value], days: i64) {
    for e in events {
        let t = chrono::DateTime::parse_from_rfc3339(e["recordedAt"].as_str().unwrap()).unwrap()
            + chrono::Duration::days(days);
        e["recordedAt"] = json!(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        if e["type"] == "shared_memory.command.requested" {
            for k in ["createdAt", "expiresAt"] {
                let t = chrono::DateTime::parse_from_rfc3339(e["payload"][k].as_str().unwrap())
                    .unwrap()
                    + chrono::Duration::days(days);
                e["payload"][k] = json!(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            }
        } else if e["type"] == "shared_memory.operation.queued" {
            for k in ["createdAt", "expiresAt"] {
                let t = chrono::DateTime::parse_from_rfc3339(
                    e["payload"]["operation"][k].as_str().unwrap(),
                )
                .unwrap()
                    + chrono::Duration::days(days);
                e["payload"]["operation"][k] =
                    json!(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            }
        } else if e["type"] == "shared_memory.command.receipted" {
            e["payload"]["receipt"]["completedAt"] = e["recordedAt"].clone();
        }
    }
}
#[test]
fn clock_rollback_and_expiry_do_not_change_durable_consent_or_uncertain_delivery() {
    let (base, project, _, _) = fixture();
    let c = conn();
    let p = command(
        json!({"projectId":project,"expectedProjectRevision":local_revision(&base,&project),"name":"Shared"}),
    );
    let (mut next, _) = shared::apply(&base, "memory_cloud_create", &p, &c).unwrap();
    shift(&mut next, 2);
    let mut future = base.clone();
    future.extend(next.clone());
    let mut retry = p.clone();
    for k in ["createdAt", "expiresAt"] {
        retry[k] = next[0]["payload"][k].clone();
    }
    assert_eq!(
        shared::apply(&future, "memory_cloud_create", &retry, &c)
            .unwrap()
            .1,
        next[2]["payload"]["receipt"]
    );
    let mut prefix = base.clone();
    prefix.extend_from_slice(&next[..2]);
    prefix.extend(shared::recover(&prefix).unwrap());
    domain::project_events(&prefix).unwrap();
    retry["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    assert_eq!(
        shared::apply(&base, "memory_cloud_create", &retry, &c)
            .unwrap_err()
            .0,
        "MEMORY_INVALID_TIME"
    );
    let (mut old, _) = shared::apply(&base, "memory_cloud_create", &p, &c).unwrap();
    shift(&mut old, -8);
    let mut expired = base.clone();
    expired.push(old[0].clone());
    let recovered = shared::recover(&expired).unwrap();
    assert_eq!(recovered[0]["payload"]["receipt"]["outcome"], "expired");
    let mut queued = base;
    queued.extend(old);
    let (next, op) = shared::prepare_delivery(&queued, project.as_str().unwrap(), &c).unwrap();
    assert!(op.is_null());
    assert_eq!(next[0]["payload"]["reasonCode"], "MEMORY_OPERATION_EXPIRED");
    queued.extend(next);
    domain::project_events(&queued).unwrap();
}

#[test]
fn unchanged_sharing_is_valid_and_rejected_setup_can_be_explicitly_retried() {
    let (mut events, project, _, _) = fixture();
    let c = conn();
    setup(&mut events, &project, &c);
    let receipt = apply(
        &mut events,
        "memory_cloud_sharing",
        &command(json!({"projectId":project,"expectedSharedRevision":0,"enabled":true})),
        &c,
    );
    assert_eq!(receipt["outcome"], "applied");
    deliver(&mut events, &project, &c, 0, "publisher", true);
    assert_eq!(projected(&events)["mappings"][0]["sharedRevision"], 0);
    let (mut fresh, project, _, _) = fixture();
    let p = command(
        json!({"projectId":project,"expectedProjectRevision":local_revision(&fresh,&project),"name":"Retry"}),
    );
    apply(&mut fresh, "memory_cloud_create", &p, &c);
    let (next, op) = shared::prepare_delivery(&fresh, project.as_str().unwrap(), &c).unwrap();
    fresh.extend(next);
    let mut r = fake_receipt(&op, 0, "publisher", true);
    r["outcome"] = json!("expired");
    r["reasonCode"] = json!("MEMORY_OPERATION_EXPIRED");
    r["completedAt"] = op["expiresAt"].clone();
    r["sharedProjectId"] = Value::Null;
    r["role"] = Value::Null;
    r["enabled"] = Value::Null;
    fresh.push(shared::acknowledge(project.as_str().unwrap(), &op, &r).unwrap());
    let retry = command(
        json!({"projectId":project,"expectedProjectRevision":local_revision(&fresh,&project),"name":"Retry"}),
    );
    assert_eq!(
        apply(&mut fresh, "memory_cloud_create", &retry, &c)["outcome"],
        "applied"
    );
}

#[test]
fn reader_cache_rejects_bad_pagination_extra_records_and_wrong_current_labels() {
    let (mut events, project, _, decision) = fixture();
    let c = conn();
    setup(&mut events, &project, &c);
    apply(
        &mut events,
        "memory_cloud_publish",
        &publish(&project, &decision, 0),
        &c,
    );
    let op = deliver(&mut events, &project, &c, 1, "publisher", true);
    let projection = projected(&events);
    let shared_id = projection["mappings"][0]["sharedProjectId"].clone();
    let at = patchfleet_core::now();
    let row = json!({"decisionId":decision,"revision":2,"title":op["payload"]["decision"]["title"],"publishedAt":at});
    let manifest = json!({"schemaVersion":1,"sharedProjectId":shared_id,"revision":1,"enabled":true,"offset":0,"limit":20,"total":1,"decisions":[row]});
    shared::validate_manifest(&manifest, &shared_id, 0, None).unwrap();
    for i in 0..4 {
        let mut bad = manifest.clone();
        match i {
            0 => bad["total"] = json!(65),
            1 => bad["decisions"] = json!([]),
            2 => bad["offset"] = json!(1),
            _ => bad["credential"] = json!("forged"),
        };
        assert!(shared::validate_manifest(&bad, &shared_id, 0, None).is_err());
    }
    let mut record = json!({"schemaVersion":1,"sharedProjectId":shared_id,"manifestRevision":1,"current":true,"publishedAt":at,"decision":op["payload"]["decision"]});
    let m = json!({"revision":1,"decisions":[]});
    assert!(shared::merge_reader(
        &shared::empty_reader(),
        &projection,
        project.as_str().unwrap(),
        &c,
        &m,
        std::slice::from_ref(&record)
    )
    .is_err());
    record["current"] = json!(false);
    let m = json!({"revision":1,"decisions":manifest["decisions"]});
    assert!(shared::merge_reader(
        &shared::empty_reader(),
        &projection,
        project.as_str().unwrap(),
        &c,
        &m,
        &[record]
    )
    .is_err());
}

#[test]
fn total_pending_queue_is_bounded_and_exact_attempt_is_retried_even_after_operation_expiry() {
    let mut events = Vec::new();
    let c = conn();
    for i in 0..33 {
        let project = memory_apply(
            &mut events,
            "project_create",
            json!({"name":format!("Project {i}")}),
        )["projectId"]
            .clone();
        let p = command(
            json!({"projectId":project,"expectedProjectRevision":1,"name":format!("Shared {i}")}),
        );
        let r = apply(&mut events, "memory_cloud_create", &p, &c);
        assert_eq!(
            r["reasonCode"],
            if i == 32 {
                "MEMORY_QUEUE_FULL"
            } else {
                "MEMORY_OPERATION_QUEUED"
            }
        );
    }
    assert_eq!(
        projected(&events)["operations"].as_array().unwrap().len(),
        32
    );
    let (base, project, _, _) = fixture();
    let p = command(
        json!({"projectId":project,"expectedProjectRevision":local_revision(&base,&project),"name":"Uncertain expired operation"}),
    );
    let (mut next, _) = shared::apply(&base, "memory_cloud_create", &p, &c).unwrap();
    let mut before = base.clone();
    before.extend(next.clone());
    let (attempt, _) = shared::prepare_delivery(&before, project.as_str().unwrap(), &c).unwrap();
    next.extend(attempt);
    shift(&mut next, -8);
    let op = next[1]["payload"]["operation"].clone();
    next.last_mut().unwrap()["payload"]["requestHash"] = json!(shared::request_hash(&op).unwrap());
    let mut old = base;
    old.extend(next);
    domain::project_events(&old).unwrap();
    let (add, retry) = shared::prepare_delivery(&old, project.as_str().unwrap(), &c).unwrap();
    assert!(add.is_empty());
    assert_eq!(retry, op);
}

#[test]
fn reader_budget_evicts_history_while_preserving_all_sixty_four_current_heads() {
    let (mut events, project, _, decision) = fixture();
    let c = conn();
    setup(&mut events, &project, &c);
    apply(
        &mut events,
        "memory_cloud_publish",
        &publish(&project, &decision, 0),
        &c,
    );
    let op = deliver(&mut events, &project, &c, 1, "publisher", true);
    let projection = projected(&events);
    let shared_id = projection["mappings"][0]["sharedProjectId"].clone();
    let mut dto = op["payload"]["decision"].clone();
    dto["decision"] = json!("界".repeat(20_000));
    for key in ["context", "rationale", "consequences"] {
        dto[key] = json!("界".repeat(10_000));
    }
    shared::validate_dto(&dto).unwrap();
    let at = patchfleet_core::now();
    let record = |current: bool| {
        let mut d = dto.clone();
        d["decisionId"] = json!(format!("decision:{}", uuid::Uuid::new_v4()));
        json!({"schemaVersion":1,"sharedProjectId":shared_id,"manifestRevision":1,"current":current,"publishedAt":at,"decision":d})
    };
    let head = |r: &Value| json!({"decisionId":r["decision"]["decisionId"],"revision":r["decision"]["revision"],"title":r["decision"]["title"],"publishedAt":r["publishedAt"]});
    let mut old = vec![record(true)];
    old.extend((0..99).map(|_| record(false)));
    let reader = json!({"schemaVersion":1,"revision":1,"projects":[{"projectId":project,"connection":c,"sharedProjectId":shared_id,"verifiedAt":at,"manifest":{"revision":1,"decisions":[head(&old[0])]},"records":old}]});
    shared::validate_reader(&reader).unwrap();
    let mut records: Vec<_> = (0..64)
        .map(|_| {
            let mut r = record(true);
            r["manifestRevision"] = json!(2);
            r
        })
        .collect();
    records.sort_by(|a, b| {
        a["decision"]["decisionId"]
            .as_str()
            .cmp(&b["decision"]["decisionId"].as_str())
    });
    let heads: Vec<_> = records.iter().map(head).collect();
    let next = shared::merge_reader(
        &reader,
        &projection,
        project.as_str().unwrap(),
        &c,
        &json!({"revision":2,"decisions":heads}),
        &records,
    )
    .unwrap();
    assert!(serde_json::to_vec(&next).unwrap().len() <= shared::MAX_READER_BYTES);
    let records = next["projects"][0]["records"].as_array().unwrap();
    assert_eq!(records.iter().filter(|r| r["current"] == true).count(), 64);
    assert!(records.len() <= 128);
    shared::validate_reader(&next).unwrap();
}

#[test]
fn connection_remapping_at_queue_cap_requires_an_actual_pending_slot_to_replace() {
    for retire_pending in [false, true] {
        let (mut events, project, _, decision) = fixture();
        let old_connection = conn();
        setup(&mut events, &project, &old_connection);
        let old_operation = if retire_pending {
            apply(
                &mut events,
                "memory_cloud_publish",
                &publish(&project, &decision, 0),
                &old_connection,
            );
            let (next, operation) =
                shared::prepare_delivery(&events, project.as_str().unwrap(), &old_connection)
                    .unwrap();
            events.extend(next);
            Some(operation["operationId"].clone())
        } else {
            None
        };
        let mut new_connection = old_connection.clone();
        new_connection["workspaceId"] = json!("workspace:new-account");
        new_connection["hostId"] = json!("host:new-account");
        for i in 0..(32 - usize::from(retire_pending)) {
            let other = memory_apply(
                &mut events,
                "project_create",
                json!({"name":format!("New queue project {i}")}),
            )["projectId"]
                .clone();
            let request = command(
                json!({"projectId":other,"expectedProjectRevision":1,"name":format!("New queue shared {i}")}),
            );
            assert_eq!(
                apply(
                    &mut events,
                    "memory_cloud_create",
                    &request,
                    &new_connection
                )["outcome"],
                "applied"
            );
        }
        let before = projected(&events);
        let pending_count = |projection: &Value| {
            projection["operations"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|op| ["queued", "attempted"].contains(&op["state"].as_str().unwrap()))
                .count()
        };
        assert_eq!(pending_count(&before), 32);
        let revision = local_revision(&events, &project);
        let request = command(
            json!({"projectId":project,"expectedProjectRevision":revision,"sharedProjectId":format!("shared:{}",uuid::Uuid::new_v4())}),
        );
        let receipt = apply(&mut events, "memory_cloud_link", &request, &new_connection);
        let after = projected(&events);
        assert_eq!(pending_count(&after), 32, "retire_pending={retire_pending}");
        if let Some(old_operation) = old_operation {
            assert_eq!(receipt["outcome"], "applied");
            let old = after["operations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|op| op["operation"]["operationId"] == old_operation)
                .unwrap();
            assert_eq!(old["state"], "unknown");
            assert_eq!(old["lastErrorCode"], "MEMORY_CONNECTION_CHANGED");
            assert_eq!(old["connection"], old_connection);
            assert_eq!(
                after["mappings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|m| m["projectId"] == project)
                    .unwrap()["connection"],
                new_connection
            );
        } else {
            assert_eq!(receipt["reasonCode"], "MEMORY_QUEUE_FULL");
            assert_eq!(receipt["outcome"], "rejected");
            assert_eq!(after, before);
        }
        let count = events.len();
        assert_eq!(
            apply(&mut events, "memory_cloud_link", &request, &new_connection),
            receipt
        );
        assert_eq!(events.len(), count);
    }
}

#[test]
fn matched_semantic_publish_rejections_end_delivery_and_allow_new_owner_operations() {
    for reason in [
        "MEMORY_CAPACITY_EXCEEDED",
        "MEMORY_CONFLICT",
        "MEMORY_STALE_REVISION",
    ] {
        let (mut events, project, source, decision) = fixture();
        let c = conn();
        setup(&mut events, &project, &c);
        let request = publish(&project, &decision, 0);
        let queued = apply(&mut events, "memory_cloud_publish", &request, &c);
        let (next, operation) =
            shared::prepare_delivery(&events, project.as_str().unwrap(), &c).unwrap();
        events.extend(next);
        let revision = u64::from(reason == "MEMORY_STALE_REVISION");
        let mut receipt = fake_receipt(&operation, revision, "publisher", true);
        receipt["outcome"] = json!("rejected");
        receipt["reasonCode"] = json!(reason);
        events.push(shared::acknowledge(project.as_str().unwrap(), &operation, &receipt).unwrap());
        let after = projected(&events);
        assert!(after["projects"][0]["pendingOperation"].is_null());
        assert_eq!(after["projects"][0]["lastErrorCode"], reason);
        assert_eq!(after["mappings"][0]["sharedRevision"], revision);
        let count = events.len();
        assert_eq!(
            apply(&mut events, "memory_cloud_publish", &request, &c),
            queued
        );
        assert_eq!(events.len(), count);
        let shared_id = after["mappings"][0]["sharedProjectId"].clone();
        events.push(shared::observed(
            project.as_str().unwrap(),
            &c,
            &shared_id,
            &json!(revision),
        ));
        apply(
            &mut events,
            "memory_cloud_sync",
            &command(json!({"projectId":project})),
            &c,
        );
        assert_eq!(projected(&events)["projects"][0]["lastErrorCode"], reason);
        let mut replacement_events = events.clone();
        let replacement = supersede(&mut replacement_events, &project, &source, &decision);
        assert_eq!(
            apply(
                &mut replacement_events,
                "memory_cloud_publish",
                &publish(&project, &replacement, revision),
                &c
            )["outcome"],
            "applied"
        );
        let sharing =
            command(json!({"projectId":project,"expectedSharedRevision":revision,"enabled":false}));
        assert_eq!(
            apply(&mut events, "memory_cloud_sharing", &sharing, &c)["outcome"],
            "applied"
        );
        assert!(projected(&events)["projects"][0]["lastErrorCode"].is_null());
    }
}

#[test]
fn rejected_receipts_match_reason_operation_scope_hash_and_server_revision() {
    let (mut events, project, _, decision) = fixture();
    let c = conn();
    setup(&mut events, &project, &c);
    apply(
        &mut events,
        "memory_cloud_publish",
        &publish(&project, &decision, 0),
        &c,
    );
    let (_, operation) = shared::prepare_delivery(&events, project.as_str().unwrap(), &c).unwrap();
    let mut receipt = fake_receipt(&operation, 0, "publisher", true);
    receipt["outcome"] = json!("rejected");
    receipt["reasonCode"] = json!("MEMORY_CAPACITY_EXCEEDED");
    shared::validate_receipt(&receipt, &operation).unwrap();
    for mutation in 0..7 {
        let mut bad = receipt.clone();
        match mutation {
            0 => bad["reasonCode"] = json!("MEMORY_PUBLISHED"),
            1 => bad["reasonCode"] = json!("MEMORY_UNKNOWN_REJECTION"),
            2 => bad["requestHash"] = json!("0".repeat(64)),
            3 => bad["role"] = json!("reader"),
            4 => bad["sharedProjectId"] = json!(format!("shared:{}", uuid::Uuid::new_v4())),
            5 => bad["revision"] = json!(1),
            _ => bad["enabled"] = json!(false),
        }
        assert!(
            shared::validate_receipt(&bad, &operation).is_err(),
            "mutation={mutation}"
        );
    }
    let mut sharing = operation.clone();
    sharing["payload"] = json!({"type":"sharing","sharedProjectId":operation["payload"]["sharedProjectId"],"expectedRevision":9_007_199_254_740_991u64,"enabled":true});
    let mut overflow = fake_receipt(&sharing, 9_007_199_254_740_991, "publisher", false);
    overflow["outcome"] = json!("rejected");
    overflow["reasonCode"] = json!("MEMORY_CONFLICT");
    shared::validate_receipt(&overflow, &sharing).unwrap();
    overflow["reasonCode"] = json!("MEMORY_CAPACITY_EXCEEDED");
    assert!(shared::validate_receipt(&overflow, &sharing).is_err());
    overflow["reasonCode"] = json!("MEMORY_CONFLICT");
    overflow["enabled"] = json!(true);
    assert!(shared::validate_receipt(&overflow, &sharing).is_err());
}
