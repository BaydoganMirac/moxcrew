use patchfleet_core::{decisions, domain, project_memory as memory, store::Store};
use serde_json::{json, Value};
use tempfile::tempdir;

fn command(mut p: Value) -> Value {
    p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    p
}
fn memory_apply(events: &mut Vec<Value>, action: &str, p: Value) -> Value {
    let (next, receipt) = memory::apply(events, action, &command(p)).unwrap();
    events.extend(next);
    receipt
}
fn memory_revision(events: &[Value], project: &str) -> Value {
    memory::project(events).unwrap()["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == project)
        .unwrap()["revision"]
        .clone()
}
fn create_project(events: &mut Vec<Value>, name: &str) -> String {
    memory_apply(events, "project_create", json!({"name":name}))["projectId"]
        .as_str()
        .unwrap()
        .into()
}
fn message(events: &mut Vec<Value>, project: &str, text: &str) -> Value {
    let rev = memory_revision(events, project);
    memory_apply(
        events,
        "project_message",
        json!({"projectId":project,"title":"Evidence","text":text,"expectedProjectRevision":rev}),
    )
}
fn edit(events: &mut Vec<Value>, project: &str, source: &str, revision: u64, text: &str) {
    let r = memory_apply(
        events,
        "project_source_edit",
        json!({"projectId":project,"sourceId":source,"expectedSourceRevision":revision,"title":"Changed evidence","text":text}),
    );
    assert_eq!(r["outcome"], "applied");
}
fn fixture() -> (Vec<Value>, String, String) {
    let mut events = Vec::new();
    let project = create_project(&mut events, "Architecture");
    let source = message(&mut events, &project, "Original owner source canary")["sourceId"]
        .as_str()
        .unwrap()
        .into();
    (events, project, source)
}
fn proposal(
    events: &[Value],
    project: &str,
    source: &str,
    source_revision: u64,
    title: &str,
) -> Value {
    command(
        json!({"projectId":project,"expectedProjectRevision":memory_revision(events,project),"title":title,"decision":"Use the local canonical writer","context":"Offline operation","rationale":"Durable evidence","consequences":"Single writer","sources":[{"sourceId":source,"revision":source_revision}]}),
    )
}
fn apply(events: &mut Vec<Value>, action: &str, p: &Value) -> Value {
    let (next, r) = decisions::apply(events, action, p).unwrap();
    events.extend(next);
    r
}
fn propose(events: &mut Vec<Value>, project: &str, source: &str, rev: u64, title: &str) -> Value {
    let p = proposal(events, project, source, rev, title);
    apply(events, "project_decision_propose", &p)
}
fn change(project: &str, decision: &Value, revision: u64) -> Value {
    command(json!({"projectId":project,"decisionId":decision,"expectedDecisionRevision":revision}))
}
fn detail(events: &[Value], project: &str, decision: &Value) -> Value {
    decisions::read_detail(
        &domain::project_events(events).unwrap()["projectDecisions"],
        &json!({"projectId":project,"decisionId":decision}),
    )
    .unwrap()
}
fn cache(events: &[Value]) -> Value {
    domain::project_events(events).unwrap()["projectDecisions"].clone()
}

#[test]
fn accepted_source_versions_survive_edits_restart_and_cache_rebuild() {
    let (mut events, project, source) = fixture();
    let source_rev = memory_revision(&events, &project);
    let proposed = propose(&mut events, &project, &source, 1, "Local authority");
    assert_eq!(proposed["reasonCode"], "PROJECT_DECISION_PROPOSED");
    assert_eq!(proposed["projectRevision"], 1);
    let accepted = apply(
        &mut events,
        "project_decision_accept",
        &change(&project, &proposed["decisionId"], 1),
    );
    assert_eq!(accepted["decisionRevision"], 2);
    assert_eq!(memory_revision(&events, &project), source_rev);
    edit(
        &mut events,
        &project,
        &source,
        1,
        "Edited source cannot rewrite a decision",
    );
    let original = detail(&events, &project, &proposed["decisionId"]);
    assert_eq!(
        original["sources"][0]["text"],
        "Original owner source canary"
    );
    assert_eq!(original["sources"][0]["revision"], 1);
    assert_eq!(
        original["decision"]["sources"][0]["origin"],
        original["sources"][0]["origin"]
    );
    assert_eq!(original["history"].as_array().unwrap().len(), 2);
    // Only source refs/provenance enter V4 facts; evidence text remains V3.
    for e in events.iter().filter(|e| e["schemaVersion"] == 4) {
        assert!(!e.to_string().contains("Original owner source canary"));
    }
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    store.append(events.clone()).unwrap();
    assert_eq!(store.read_decision(&json!({"projectId":project,"decisionId":proposed["decisionId"],"commandId":"ignored","createdAt":"ignored","expiresAt":"ignored"})).unwrap(),original);
    let runtime = store.runtime_projections().unwrap();
    assert_eq!(runtime["projectDecisions"].as_object().unwrap().len(), 3);
    assert!(runtime["projectDecisions"].get("decisions").is_none());
    assert!(!runtime.to_string().contains("Original owner source canary"));
    let expected = store.projections().unwrap();
    let cache_path = dir.path().join("project-decisions.json");
    let before = std::fs::metadata(&cache_path).unwrap().modified().unwrap();
    let count = store.event_cursor().unwrap().0;
    for _ in 0..3 {
        store.read_decisions(&json!({"projectId":project})).unwrap();
    }
    assert_eq!(
        before,
        std::fs::metadata(&cache_path).unwrap().modified().unwrap()
    );
    assert_eq!(count, store.event_cursor().unwrap().0);
    drop(store);
    std::fs::write(&cache_path, b"damaged disposable projection").unwrap();
    let reopened = Store::open(dir.path().into()).unwrap();
    assert_eq!(reopened.projections().unwrap(), expected);
    assert_eq!(
        reopened
            .read_decision(&json!({"projectId":project,"decisionId":proposed["decisionId"]}))
            .unwrap(),
        original
    );
}

#[test]
fn changed_sources_block_acceptance_but_rejection_remains_available() {
    let (mut events, project, source) = fixture();
    let proposed = propose(&mut events, &project, &source, 1, "Old evidence");
    edit(&mut events, &project, &source, 1, "Current evidence");
    let before = cache(&events);
    let rejected = apply(
        &mut events,
        "project_decision_accept",
        &change(&project, &proposed["decisionId"], 1),
    );
    assert_eq!(rejected["reasonCode"], "STALE_SOURCE_REVISION");
    assert_eq!(cache(&events), before);
    let rejected = apply(
        &mut events,
        "project_decision_reject",
        &change(&project, &proposed["decisionId"], 1),
    );
    assert_eq!(rejected["outcome"], "applied");
    assert_eq!(
        detail(&events, &project, &proposed["decisionId"])["decision"]["status"],
        "rejected"
    );
    let repeated = apply(
        &mut events,
        "project_decision_accept",
        &change(&project, &proposed["decisionId"], 2),
    );
    assert_eq!(repeated["reasonCode"], "DECISION_NOT_PROPOSED");
}

#[test]
fn supersession_accepts_replacement_atomically_and_stale_replacement_leaves_old_active() {
    let (mut events, project, source) = fixture();
    let old = propose(&mut events, &project, &source, 1, "Old");
    apply(
        &mut events,
        "project_decision_accept",
        &change(&project, &old["decisionId"], 1),
    );
    let replacement = propose(&mut events, &project, &source, 1, "Replacement");
    edit(&mut events, &project, &source, 1, "Revision two");
    let swap = |replacement: &Value| {
        command(
            json!({"projectId":project,"decisionId":old["decisionId"],"expectedDecisionRevision":2,"replacementDecisionId":replacement,"expectedReplacementRevision":1}),
        )
    };
    let before = cache(&events);
    let denied = apply(
        &mut events,
        "project_decision_supersede",
        &swap(&replacement["decisionId"]),
    );
    assert_eq!(denied["reasonCode"], "STALE_SOURCE_REVISION");
    assert_eq!(cache(&events), before);
    let current = propose(&mut events, &project, &source, 2, "Current replacement");
    let from = events.len();
    let accepted = apply(
        &mut events,
        "project_decision_supersede",
        &swap(&current["decisionId"]),
    );
    assert_eq!(
        events[from..]
            .iter()
            .filter(|e| e["type"] == "decision.superseded")
            .count(),
        1
    );
    assert_eq!(accepted["projectRevision"], 5);
    assert_eq!(accepted["decisionRevision"], 3);
    assert_eq!(accepted["replacementRevision"], 2);
    let predecessor = detail(&events, &project, &old["decisionId"]);
    let successor = detail(&events, &project, &current["decisionId"]);
    assert_eq!(predecessor["decision"]["status"], "superseded");
    assert_eq!(
        predecessor["decision"]["supersededByDecisionId"],
        current["decisionId"]
    );
    assert_eq!(successor["decision"]["status"], "accepted");
    assert_eq!(
        successor["decision"]["supersedesDecisionId"],
        old["decisionId"]
    );
    assert_eq!(predecessor["history"].as_array().unwrap().len(), 3);
    assert_eq!(successor["sources"][0]["text"], "Revision two");
    assert_eq!(
        predecessor["sources"][0]["text"],
        "Original owner source canary"
    );
    let meta = decisions::metadata(&cache(&events));
    assert_eq!(meta["projects"][0]["acceptedCount"], 1);
    assert_eq!(meta["projects"][0]["proposedCount"], 1);
}

#[test]
fn exact_retries_are_stable_and_changed_command_identity_is_rejected() {
    let (mut events, project, source) = fixture();
    let p = proposal(&events, &project, &source, 1, "Idempotence");
    let receipt = apply(&mut events, "project_decision_propose", &p);
    edit(&mut events, &project, &source, 1, "Later source");
    let count = events.len();
    assert_eq!(apply(&mut events, "project_decision_propose", &p), receipt);
    assert_eq!(events.len(), count);
    let mut changed = p.clone();
    changed["decision"] = json!("Changed content");
    assert_eq!(
        decisions::apply(&events, "project_decision_propose", &changed)
            .unwrap_err()
            .0,
        "IDEMPOTENCY_KEY_REUSED"
    );
    let mut changed = p.clone();
    changed["createdAt"] = json!("2026-01-01T00:00:00.000Z");
    assert!(decisions::apply(&events, "project_decision_propose", &changed).is_err());
    assert!(decisions::recover(&events).unwrap().is_empty());
}

#[test]
fn each_lifecycle_command_recovers_every_durable_prefix_without_repeating_facts() {
    for action in [
        "project_decision_propose",
        "project_decision_accept",
        "project_decision_reject",
        "project_decision_supersede",
    ] {
        let (mut base, project, source) = fixture();
        let payload = if action == "project_decision_propose" {
            proposal(&base, &project, &source, 1, "Prefix draft")
        } else {
            let draft = propose(&mut base, &project, &source, 1, "First draft");
            if action == "project_decision_supersede" {
                apply(
                    &mut base,
                    "project_decision_accept",
                    &change(&project, &draft["decisionId"], 1),
                );
                let replacement = propose(&mut base, &project, &source, 1, "Replacement");
                command(
                    json!({"projectId":project,"decisionId":draft["decisionId"],"expectedDecisionRevision":2,"replacementDecisionId":replacement["decisionId"],"expectedReplacementRevision":1}),
                )
            } else {
                change(&project, &draft["decisionId"], 1)
            }
        };
        let (next, expected) = decisions::apply(&base, action, &payload).unwrap();
        assert_eq!(next.len(), 3);
        for prefix in 0..=next.len() {
            let dir = tempdir().unwrap();
            let store = Store::open(dir.path().into()).unwrap();
            let mut input = base.clone();
            input.extend_from_slice(&next[..prefix]);
            store.append(input).unwrap();
            let recovered = decisions::recover(&store.read_events().unwrap()).unwrap();
            store.append(recovered).unwrap();
            let receipt = store
                .transaction(|events| decisions::apply(events, action, &payload))
                .unwrap();
            assert_eq!(receipt["outcome"], expected["outcome"]);
            assert_eq!(receipt["decisionRevision"], expected["decisionRevision"]);
            let full = store.read_events().unwrap();
            assert_eq!(
                full.iter()
                    .filter(|e| e["schemaVersion"] == 4
                        && e["type"] != "decision.command.requested"
                        && e["type"] != "decision.command.receipted"
                        && e["payload"]["commandId"] == payload["commandId"])
                    .count(),
                1,
                "{action} prefix {prefix}"
            );
            assert_eq!(
                store.projections().unwrap(),
                domain::project_events(&full).unwrap()
            );
            assert!(decisions::recover(&full).unwrap().is_empty());
        }
    }
}

fn shift_command_time(events: &mut [Value], days: i64) {
    for e in events {
        let t = chrono::DateTime::parse_from_rfc3339(e["recordedAt"].as_str().unwrap()).unwrap()
            + chrono::Duration::days(days);
        e["recordedAt"] = json!(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        if e["type"] == "decision.command.requested" {
            for key in ["createdAt", "expiresAt"] {
                let t = chrono::DateTime::parse_from_rfc3339(e["payload"][key].as_str().unwrap())
                    .unwrap()
                    + chrono::Duration::days(days);
                e["payload"][key] = json!(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            }
        } else if e["type"] == "decision.proposed" {
            for key in ["createdAt", "updatedAt"] {
                e["payload"]["decision"][key] = e["recordedAt"].clone();
            }
        } else if e["type"] == "decision.command.receipted" {
            e["payload"]["receipt"]["completedAt"] = e["recordedAt"].clone();
        }
    }
}
#[test]
fn clock_corrections_preserve_receipts_and_fact_prefixes_but_new_future_commands_fail() {
    let (base, project, source) = fixture();
    let p = proposal(&base, &project, &source, 1, "Clock");
    let (mut next, _) = decisions::apply(&base, "project_decision_propose", &p).unwrap();
    shift_command_time(&mut next, 2);
    let mut future = base.clone();
    future.extend(next.clone());
    domain::project_events(&future).unwrap();
    let mut retry = p.clone();
    for key in ["createdAt", "expiresAt"] {
        retry[key] = next[0]["payload"][key].clone();
    }
    assert_eq!(
        decisions::apply(&future, "project_decision_propose", &retry)
            .unwrap()
            .1,
        next[2]["payload"]["receipt"]
    );
    let mut prefix = base.clone();
    prefix.extend_from_slice(&next[..2]);
    let recovered = decisions::recover(&prefix).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0]["payload"]["receipt"]["outcome"], "applied");
    prefix.extend(recovered);
    domain::project_events(&prefix).unwrap();
    retry["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    assert_eq!(
        decisions::apply(&base, "project_decision_propose", &retry)
            .unwrap_err()
            .0,
        "INVALID_DECISION_TIME"
    );
    let mut pending = base;
    pending.push(next[0].clone());
    let recovered = decisions::recover(&pending).unwrap();
    assert_eq!(recovered[0]["type"], "decision.proposed");
    pending.extend(recovered);
    domain::project_events(&pending).unwrap();
}

#[test]
fn expired_request_does_not_create_fact_but_already_applied_fact_gets_applied_receipt() {
    let (base, project, source) = fixture();
    let p = proposal(&base, &project, &source, 1, "Expiry");
    let (mut next, _) = decisions::apply(&base, "project_decision_propose", &p).unwrap();
    shift_command_time(&mut next, -2);
    for prefix in 1..=2 {
        let mut input = base.clone();
        input.extend_from_slice(&next[..prefix]);
        let recovered = decisions::recover(&input).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(
            recovered[0]["payload"]["receipt"]["outcome"],
            if prefix == 1 { "expired" } else { "applied" }
        );
        input.extend(recovered);
        domain::project_events(&input).unwrap();
    }
}

#[test]
fn project_scope_is_enforced_for_sources_decisions_replacements_and_reads() {
    let (mut events, a, source) = fixture();
    let b = create_project(&mut events, "B");
    let b_source = message(&mut events, &b, "Project B secret")["sourceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let wrong = proposal(&events, &a, &b_source, 1, "Wrong source");
    assert_eq!(
        apply(&mut events, "project_decision_propose", &wrong)["reasonCode"],
        "SOURCE_PROJECT_MISMATCH"
    );
    let draft = propose(&mut events, &a, &source, 1, "A only");
    assert_eq!(
        apply(
            &mut events,
            "project_decision_accept",
            &change(&b, &draft["decisionId"], 1)
        )["reasonCode"],
        "DECISION_NOT_FOUND"
    );
    assert_eq!(
        decisions::read_detail(
            &cache(&events),
            &json!({"projectId":b,"decisionId":draft["decisionId"]})
        )
        .unwrap_err()
        .0,
        "DECISION_NOT_FOUND"
    );
    apply(
        &mut events,
        "project_decision_accept",
        &change(&a, &draft["decisionId"], 1),
    );
    let other = propose(&mut events, &b, &b_source, 1, "B draft");
    let swap = command(
        json!({"projectId":a,"decisionId":draft["decisionId"],"expectedDecisionRevision":2,"replacementDecisionId":other["decisionId"],"expectedReplacementRevision":1}),
    );
    assert_eq!(
        apply(&mut events, "project_decision_supersede", &swap)["reasonCode"],
        "REPLACEMENT_NOT_FOUND"
    );
    let page = decisions::read_list(
        &cache(&events),
        &json!({"projectId":a,"query":"Project B secret"}),
    )
    .unwrap();
    assert_eq!(page["total"], 0);
}

#[test]
fn forged_facts_and_receipts_unknown_versions_and_duplicate_records_fail_closed() {
    let (base, project, source) = fixture();
    let p = proposal(&base, &project, &source, 1, "Unforgeable");
    let (next, _) = decisions::apply(&base, "project_decision_propose", &p).unwrap();
    for mutation in 0..9 {
        let mut forged = next.clone();
        match mutation {
            0 => forged[1]["payload"]["decision"]["decision"] = json!("Forged content"),
            1 => forged[1]["payload"]["decision"]["sources"][0]["origin"]["id"] = json!("forged"),
            2 => forged[1]["payload"]["decision"]["sources"][0]["revision"] = json!(2),
            3 => forged[1]["payload"]["decision"]["status"] = json!("accepted"),
            4 => forged[2]["payload"]["receipt"]["projectRevision"] = json!(99),
            5 => forged[1]["schemaVersion"] = json!(3),
            6 => forged[1]["type"] = json!("decision.unknown"),
            7 => forged[0]["payload"]["actorId"] = json!("cloud-owner"),
            _ => forged[1]["payload"]["decision"]["secret"] = json!("arbitrary"),
        }
        let mut input = base.clone();
        input.extend(forged);
        assert!(
            domain::project_events(&input).is_err(),
            "mutation {mutation}"
        );
    }
    for indices in [
        vec![1],
        vec![2],
        vec![0, 2],
        vec![0, 1, 1],
        vec![0, 1, 2, 2],
        vec![0, 0],
    ] {
        let mut input = base.clone();
        input.extend(indices.into_iter().map(|i| next[i].clone()));
        assert!(domain::project_events(&input).is_err());
    }
    let mut input = base;
    input.extend(next);
    let p = change(
        &project,
        &input.last().unwrap()["payload"]["receipt"]["decisionId"],
        1,
    );
    let (mut transition, _) = decisions::apply(&input, "project_decision_accept", &p).unwrap();
    transition[1]["type"] = json!("decision.rejected");
    input.extend(transition);
    assert!(domain::project_events(&input).is_err());
}

#[test]
fn decision_reads_are_bounded_scoped_summaries_and_include_empty_projects() {
    let (mut events, project, source) = fixture();
    let empty = create_project(&mut events, "Empty");
    let empty_page = decisions::read_list(&cache(&events), &json!({"projectId":empty})).unwrap();
    assert_eq!(empty_page["revision"], 0);
    assert_eq!(empty_page["total"], 0);
    for i in 0..23 {
        let r = propose(&mut events, &project, &source, 1, &format!("Decision {i}"));
        if i == 0 {
            apply(
                &mut events,
                "project_decision_reject",
                &change(&project, &r["decisionId"], 1),
            );
        }
    }
    let projected = cache(&events);
    let before = events.clone();
    let page = decisions::read_list(
        &projected,
        &json!({"projectId":project,"limit":20,"query":" canonical "}),
    )
    .unwrap();
    assert_eq!(page["query"], "canonical");
    assert_eq!(page["total"], 23);
    assert_eq!(page["decisions"].as_array().unwrap().len(), 20);
    assert!(page["decisions"][0].get("decision").is_none());
    assert!(page["decisions"][0].get("sources").is_none());
    let page = decisions::read_list(
        &projected,
        &json!({"projectId":project,"offset":20,"limit":20,"status":"proposed"}),
    )
    .unwrap();
    assert_eq!(page["total"], 22);
    assert_eq!(page["decisions"].as_array().unwrap().len(), 2);
    for p in [
        json!({"projectId":project,"limit":21}),
        json!({"projectId":project,"offset":-1}),
        json!({"projectId":project,"status":"binding"}),
        json!({"projectId":project,"query":"bad\u{000b}"}),
        json!({"projectId":project,"unknown":true}),
    ] {
        assert!(decisions::read_list(&projected, &p).is_err());
    }
    assert_eq!(events, before);
}

#[test]
fn strict_text_utf16_optional_fields_and_time_boundaries_are_validated() {
    let (base, project, source) = fixture();
    let p = proposal(&base, &project, &source, 1, "Text");
    for mutation in 0..9 {
        let mut bad = p.clone();
        match mutation {
            0 => bad["title"] = json!("😀".repeat(81)),
            1 => bad["decision"] = json!(" "),
            2 => bad["context"] = json!("hidden\u{0000}"),
            3 => bad["sources"] = json!([]),
            4 => {
                bad["sources"] =
                    json!([{"sourceId":source,"revision":1},{"sourceId":source,"revision":1}])
            }
            5 => bad["createdAt"] = json!("bad time"),
            6 => {
                bad["createdAt"] = json!("2026-01-01T00:00:00.000Z");
                bad["expiresAt"] = json!("2026-01-01T00:05:00.001Z");
            }
            7 => bad["sources"][0]["revision"] = json!(0),
            _ => bad["unexpected"] = json!(true),
        }
        assert!(
            decisions::apply(&base, "project_decision_propose", &bad).is_err(),
            "mutation {mutation}"
        );
    }
    let mut valid = p;
    valid["title"] = json!("😀".repeat(80));
    valid["decision"] = json!("界".repeat(20_000));
    for key in ["context", "rationale", "consequences"] {
        valid.as_object_mut().unwrap().remove(key);
    }
    let (next, r) = decisions::apply(&base, "project_decision_propose", &valid).unwrap();
    assert_eq!(r["outcome"], "applied");
    assert_eq!(next[1]["payload"]["decision"]["context"], "");
}

#[test]
fn interleaved_pending_receipt_keeps_the_original_fact_revisions() {
    let (mut events, project, source) = fixture();
    let p = proposal(&events, &project, &source, 1, "Pending receipt");
    let (next, original) = decisions::apply(&events, "project_decision_propose", &p).unwrap();
    events.extend_from_slice(&next[..2]);
    let accepted = apply(
        &mut events,
        "project_decision_accept",
        &change(&project, &original["decisionId"], 1),
    );
    assert_eq!(accepted["projectRevision"], 2);
    events.extend(decisions::recover(&events).unwrap());
    let receipt = apply(&mut events, "project_decision_propose", &p);
    assert_eq!(receipt["decisionRevision"], 1);
    assert_eq!(receipt["projectRevision"], 1);
    assert_eq!(
        detail(&events, &project, &original["decisionId"])["decision"]["revision"],
        2
    );
    domain::project_events(&events).unwrap();
}

#[test]
fn bound_legacy_work_evidence_and_workspace_removal_use_the_exact_v3_timeline() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/parity-v1.json")).unwrap();
    let mut events = Vec::new();
    let project = create_project(&mut events, "Legacy-bound project");
    let work_case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "work-control-receipts-and-observation")
        .unwrap();
    let legacy = work_case["events"].as_array().unwrap();
    events.extend_from_slice(&legacy[..3]);
    let workspace = legacy[1]["payload"]["workspace"]["workspaceId"].clone();
    let rev = memory_revision(&events, &project);
    memory_apply(
        &mut events,
        "project_link_workspace",
        json!({"projectId":project,"workspaceId":workspace,"expectedProjectRevision":rev}),
    );
    events.push(memory::bind(&project, "work", "work-a"));
    events.extend_from_slice(&legacy[3..10]);
    let source = format!("source:{}", legacy[8]["id"].as_str().unwrap());
    let draft = propose(&mut events, &project, &source, 1, "Bound work decision");
    assert_eq!(draft["outcome"], "applied");
    let d = detail(&events, &project, &draft["decisionId"]);
    assert_eq!(d["sources"][0]["kind"], "work_instruction");
    assert_eq!(
        d["sources"][0]["text"],
        legacy[8]["payload"]["workItem"]["instruction"]
    );
    // Removal has a separate real V1 intent/receipt; source scope survives it.
    let ws_case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "workspace-receipts-and-removal")
        .unwrap();
    let mut removal = ws_case["events"].as_array().unwrap()[6..9].to_vec();
    removal[0]["payload"]["intent"]["payload"]["workspaceId"] = workspace.clone();
    removal[1]["payload"]["workspaceId"] = workspace.clone();
    removal[2]["payload"]["receipt"]["workspaceId"] = workspace;
    // The reference fixture had two registrations before removal; this log has one.
    removal[2]["payload"]["receipt"]["workspaceProjectionRevision"] = json!(2);
    events.extend(removal);
    domain::project_events(&events).unwrap();
    let fresh = proposal(&events, &project, &source, 1, "After removal");
    assert_eq!(
        apply(&mut events, "project_decision_propose", &fresh)["outcome"],
        "applied"
    );
    assert_eq!(
        apply(
            &mut events,
            "project_decision_accept",
            &change(&project, &draft["decisionId"], 1)
        )["outcome"],
        "applied"
    );
    assert_eq!(
        detail(&events, &project, &draft["decisionId"])["sources"],
        d["sources"]
    );
}

#[test]
fn proposals_cannot_reference_future_source_revisions_or_forge_atomic_supersession() {
    let (mut events, project, source) = fixture();
    let old = propose(&mut events, &project, &source, 1, "Old");
    apply(
        &mut events,
        "project_decision_accept",
        &change(&project, &old["decisionId"], 1),
    );
    let replacement = propose(&mut events, &project, &source, 1, "Replacement");
    let p = command(
        json!({"projectId":project,"decisionId":old["decisionId"],"expectedDecisionRevision":2,"replacementDecisionId":replacement["decisionId"],"expectedReplacementRevision":1}),
    );
    let (next, _) = decisions::apply(&events, "project_decision_supersede", &p).unwrap();
    let mut forged = next.clone();
    forged[1]["payload"]["replacementDecisionId"] = old["decisionId"].clone();
    let mut input = events.clone();
    input.extend(forged);
    assert!(domain::project_events(&input).is_err());
    let mut future = proposal(&events, &project, &source, 2, "Future citation");
    assert_eq!(
        apply(&mut events, "project_decision_propose", &future)["reasonCode"],
        "STALE_SOURCE_REVISION"
    );
    future["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    future["expectedProjectRevision"] = json!(1);
    assert_eq!(
        apply(&mut events, "project_decision_propose", &future)["reasonCode"],
        "STALE_PROJECT_REVISION"
    );
}
