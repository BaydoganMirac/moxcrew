use patchfleet_core::{decisions, domain, extraction as ex, project_memory as memory};
use serde_json::{json, Value};

fn command(mut value: Value) -> Value {
    value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    value
}
fn memory_apply(events: &mut Vec<Value>, action: &str, value: Value) -> Value {
    let (next, r) = memory::apply(events, action, &command(value)).unwrap();
    events.extend(next);
    r
}
fn project(events: &mut Vec<Value>) -> String {
    memory_apply(
        events,
        "project_create",
        json!({"name":"Synthetic architecture"}),
    )["projectId"]
        .as_str()
        .unwrap()
        .into()
}
fn revision(events: &[Value], project: &str) -> Value {
    memory::project(events).unwrap()["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == project)
        .unwrap()["revision"]
        .clone()
}
fn message(events: &mut Vec<Value>, project: &str, text: &str) -> String {
    let revision = revision(events, project);
    memory_apply(events,"project_message",json!({"projectId":project,"title":"Owner source","text":text,"expectedProjectRevision":revision}))["sourceId"].as_str().unwrap().into()
}
fn apply(events: &mut Vec<Value>, action: &str, value: Value) -> Value {
    let (next, r) = ex::apply(events, action, &command(value)).unwrap();
    events.extend(next);
    r
}
fn configure(events: &mut Vec<Value>, project: &str, revision: u64, enabled: bool) -> Value {
    apply(
        events,
        "project_extraction_configure",
        json!({"projectId":project,"expectedPolicyRevision":revision,"enabled":enabled,"consent":enabled}),
    )
}
fn reconcile(events: &mut Vec<Value>) {
    events.extend(ex::reconcile(events).unwrap());
}
fn projection(events: &[Value]) -> Value {
    ex::project_validated(events).unwrap()
}
fn job(events: &[Value], source: &str) -> Value {
    projection(events)["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|j| j["sourceId"] == source)
        .unwrap()
        .clone()
}
fn start(events: &mut Vec<Value>, source: &str) -> Value {
    let id = job(events, source)["jobId"].as_str().unwrap().to_owned();
    let (next, r) = ex::start(events, &id, "synthetic-epoch").unwrap();
    events.extend(next);
    r
}
fn proposal(source: &str, decision: &str) -> Value {
    json!({"title":"Local authority","context":"Owner intent","decision":decision,"rationale":"Durable context","consequences":"Explicit acceptance","evidence":[{"sourceId":source,"revision":1,"excerpt":"Use the canonical writer"}],"relation":"new","relatedDecisionIds":[]})
}
fn finish(
    events: &mut Vec<Value>,
    source: &str,
    outcome: std::result::Result<Value, String>,
) -> Value {
    let j = job(events, source);
    let (next, r) = ex::finish(
        events,
        j["jobId"].as_str().unwrap(),
        j["attempt"].as_u64().unwrap(),
        "synthetic-epoch",
        outcome,
    )
    .unwrap();
    events.extend(next);
    r
}
fn fixture() -> (Vec<Value>, String, String) {
    let mut events = vec![];
    let project = project(&mut events);
    configure(&mut events, &project, 0, true);
    let source = message(
        &mut events,
        &project,
        "Use the canonical writer. PRIVATE_SOURCE_BODY_CANARY Türkçe karar kanıtı.",
    );
    reconcile(&mut events);
    (events, project, source)
}
fn edit(events: &mut Vec<Value>, project: &str, source: &str) {
    memory_apply(
        events,
        "project_source_edit",
        json!({"projectId":project,"sourceId":source,"expectedSourceRevision":1,"title":"Edited","text":"Changed source body"}),
    );
}

#[test]
fn consent_is_off_by_default_enable_never_backfills_and_commands_are_idempotent() {
    let mut events = vec![];
    let project = project(&mut events);
    let old = message(
        &mut events,
        &project,
        "Old source cannot silently cost inference",
    );
    assert!(projection(&events).is_null());
    assert!(ex::reconcile(&events).unwrap().is_empty());
    let denied = apply(
        &mut events,
        "project_extraction_configure",
        json!({"projectId":project,"expectedPolicyRevision":0,"enabled":true,"consent":false}),
    );
    assert_eq!(denied["reasonCode"], "EXTRACTION_CONSENT_REQUIRED");
    let payload = command(
        json!({"projectId":project,"expectedPolicyRevision":0,"enabled":true,"consent":true}),
    );
    let (next, receipt) = ex::apply(&events, "project_extraction_configure", &payload).unwrap();
    events.extend(next);
    assert_eq!(receipt["outcome"], "applied");
    let (next, again) = ex::apply(&events, "project_extraction_configure", &payload).unwrap();
    assert!(next.is_empty());
    assert_eq!(receipt, again);
    let mut reused = payload;
    reused["enabled"] = json!(false);
    assert!(ex::apply(&events, "project_extraction_configure", &reused).is_err());
    reconcile(&mut events);
    assert!(projection(&events)["jobs"].as_array().unwrap().is_empty());
    let queued = apply(
        &mut events,
        "project_extraction_enqueue",
        json!({"projectId":project,"sourceId":old,"expectedSourceRevision":1}),
    );
    assert_eq!(queued["outcome"], "applied");
    let new = message(&mut events, &project, "Yeni kaynak / new source");
    reconcile(&mut events);
    assert_eq!(job(&events, &new)["status"], "queued");
    let stale = configure(&mut events, &project, 0, false);
    assert_eq!(stale["reasonCode"], "EXTRACTION_STALE_POLICY");
}
#[test]
fn sourced_drafts_keep_explicit_generator_and_quotes_without_accepting_or_exporting_bodies() {
    let (mut events, project, source) = fixture();
    let started = start(&mut events, &source);
    assert!(started["input"]["source"]["text"]
        .as_str()
        .unwrap()
        .contains("PRIVATE_SOURCE_BODY_CANARY"));
    assert_eq!(started["input"]["extractorVersion"], ex::VERSION);
    assert!(serde_json::to_vec(&started["input"]).unwrap().len() <= ex::MAX_INPUT_BYTES);
    let r = finish(
        &mut events,
        &source,
        Ok(json!({"proposals":[proposal(&source,"Use Rust for the local writer")]})),
    );
    assert_eq!(r["job"]["status"], "completed");
    let decision = r["job"]["proposalIds"][0].as_str().unwrap();
    let ds = decisions::project_validated(&events).unwrap();
    assert_eq!(ds["decisions"][0]["status"], "proposed");
    let provenance = ex::provenance(&projection(&events), decision).unwrap();
    assert_eq!(provenance["generator"]["providerId"], "codex");
    assert_eq!(
        provenance["evidence"][0]["excerpt"],
        "Use the canonical writer"
    );
    let metadata = ex::metadata(&projection(&events)).to_string();
    for canary in [
        "PRIVATE_SOURCE_BODY_CANARY",
        "Use the canonical writer",
        "Use Rust for the local writer",
        "Owner source",
    ] {
        assert!(!metadata.contains(canary));
    }
    let mut future_projection = projection(&events);
    future_projection["jobs"][0]["rawProviderOutput"] = json!("FUTURE_FIELD_CANARY");
    assert!(!ex::metadata(&future_projection)
        .to_string()
        .contains("FUTURE_FIELD_CANARY"));
    edit(&mut events, &project, &source);
    assert_eq!(
        ex::provenance(&projection(&events), decision).unwrap(),
        provenance
    );
    let detail = decisions::read_detail(
        &decisions::project_validated(&events).unwrap(),
        &json!({"projectId":project,"decisionId":decision}),
    )
    .unwrap();
    assert_eq!(detail["sources"][0]["revision"], 1);
    assert!(detail["sources"][0]["text"]
        .as_str()
        .unwrap()
        .contains("PRIVATE_SOURCE_BODY_CANARY"));
    assert_eq!(ex::metadata(&Value::Null), Value::Null);
    assert_eq!(ex::provenance(&Value::Null, decision), None);
}
#[test]
fn no_adr_is_a_success_and_raw_provider_errors_are_never_durable() {
    let (mut events, project, source) = fixture();
    start(&mut events, &source);
    let r = finish(&mut events, &source, Ok(json!({"proposals":[]})));
    assert_eq!(r["job"]["reasonCode"], "EXTRACTION_NO_ADR");
    assert!(decisions::project_validated(&events).unwrap()["decisions"]
        .as_array()
        .unwrap()
        .is_empty());
    let next = message(
        &mut events,
        &project,
        "Sadece bir soru mu? / Is this a question?",
    );
    reconcile(&mut events);
    start(&mut events, &next);
    finish(
        &mut events,
        &next,
        Err("RAW_PROVIDER_SECRET_CANARY /private/path".into()),
    );
    assert!(!serde_json::to_string(&events)
        .unwrap()
        .contains("RAW_PROVIDER_SECRET_CANARY"));
    assert_eq!(
        job(&events, &next)["reasonCode"],
        "EXTRACTION_PROVIDER_FAILED"
    );
    assert!(ex::reconcile(&events).unwrap().is_empty());
}

#[test]
fn host_shutdown_signal_is_interrupted_and_requires_explicit_retry() {
    let (mut events, _, source) = fixture();
    start(&mut events, &source);
    let r = finish(&mut events, &source, Err("EXTRACTION_INTERRUPTED".into()));
    assert_eq!(r["job"]["status"], "interrupted");
    assert_eq!(r["job"]["reasonCode"], "EXTRACTION_INTERRUPTED");
    assert!(ex::reconcile(&events).unwrap().is_empty());
    assert!(ex::recover(&events).unwrap().is_empty());
}
#[test]
fn malformed_or_foreign_second_proposal_fails_the_whole_batch() {
    let (mut base, _, source) = fixture();
    start(&mut base, &source);
    let valid = proposal(&source, "Use Rust");
    let mut cases = vec![];
    let mut bad = valid.clone();
    bad["evidence"][0]["excerpt"] = json!("Invented unsupported quote");
    cases.push(bad);
    let mut bad = valid.clone();
    bad["evidence"][0]["revision"] = json!(2);
    cases.push(bad);
    let mut bad = valid.clone();
    bad["evidence"][0]["sourceId"] = json!(format!("source:{}", uuid::Uuid::new_v4()));
    cases.push(bad);
    let mut bad = valid.clone();
    bad["relation"] = json!("possible_conflict");
    bad["relatedDecisionIds"] = json!([format!("decision:{}", uuid::Uuid::new_v4())]);
    cases.push(bad);
    let mut bad = valid.clone();
    bad["tool"] = json!("project_decision_accept");
    cases.push(bad);
    let mut bad = valid.clone();
    bad["title"] = json!("🦀".repeat(81));
    cases.push(bad);
    let mut bad = valid.clone();
    bad["evidence"][0]["excerpt"] = json!("x".repeat(1001));
    cases.push(bad);
    let mut bad = valid.clone();
    bad["decision"] = json!("三".repeat(20_000));
    for key in ["context", "rationale", "consequences"] {
        bad[key] = json!("三".repeat(10_000));
    }
    cases.push(bad);
    for bad in cases {
        let mut events = base.clone();
        let before = events.len();
        let r = finish(&mut events, &source, Ok(json!({"proposals":[valid,bad]})));
        assert_eq!(r["job"]["reasonCode"], "EXTRACTION_INVALID_OUTPUT");
        assert!(events[before..].iter().all(|e| e["schemaVersion"] == 7));
        assert!(decisions::project_validated(&events).unwrap()["decisions"]
            .as_array()
            .unwrap()
            .is_empty());
    }
}
#[test]
fn source_edits_and_policy_generations_cancel_work_and_cannot_publish_stale_completion() {
    let (mut events, project, source) = fixture();
    start(&mut events, &source);
    edit(&mut events, &project, &source);
    let r = finish(
        &mut events,
        &source,
        Ok(json!({"proposals":[proposal(&source,"Stale")]})),
    );
    assert_eq!(r["job"]["reasonCode"], "EXTRACTION_SOURCE_CHANGED");
    let retry = apply(
        &mut events,
        "project_extraction_retry",
        json!({"projectId":project,"jobId":r["job"]["jobId"],"expectedAttempt":1}),
    );
    assert_eq!(retry["reasonCode"], "EXTRACTION_SOURCE_CHANGED");
    reconcile(&mut events);
    assert_eq!(job(&events, &source)["sourceRevision"], 2);
    start(&mut events, &source);
    configure(&mut events, &project, 1, false);
    configure(&mut events, &project, 2, true);
    reconcile(&mut events);
    assert_eq!(
        job(&events, &source)["reasonCode"],
        "EXTRACTION_POLICY_CHANGED"
    );
    assert!(ex::finish(
        &events,
        job(&events, &source)["jobId"].as_str().unwrap(),
        1,
        "synthetic-epoch",
        Ok(json!({"proposals":[]}))
    )
    .is_err());
    let current = job(&events, &source);
    let retried = apply(
        &mut events,
        "project_extraction_retry",
        json!({"projectId":project,"jobId":current["jobId"],"expectedAttempt":1}),
    );
    assert_eq!(retried["outcome"], "applied");
    let r = start(&mut events, &source);
    assert_eq!(r["job"]["policyRevision"], 3);
}
#[test]
fn restart_prefixes_never_leave_a_generated_draft_without_provenance_or_repeat_acknowledged_text() {
    let (mut base, project, source) = fixture();
    start(&mut base, &source);
    let output = json!({"proposals":[proposal(&source,"Use Rust"),proposal(&source,"Keep one canonical writer")]});
    let j = job(&base, &source);
    let (next, _) = ex::finish(
        &base,
        j["jobId"].as_str().unwrap(),
        1,
        "synthetic-epoch",
        Ok(output.clone()),
    )
    .unwrap();
    let mut incomplete = base.clone();
    incomplete.push(next[0].clone()); // Bound provenance, but no V4 command/receipt yet.
    let mut forged_completion = next.last().unwrap().clone();
    forged_completion["payload"]["proposalIds"] = json!([]);
    forged_completion["payload"]["reasonCode"] = json!("EXTRACTION_NO_ADR");
    incomplete.push(forged_completion);
    assert!(ex::project_validated(&incomplete).is_err());
    for count in 0..=next.len() {
        let mut events = base.clone();
        events.extend(next[..count].iter().cloned());
        domain::project_events(&events).unwrap();
        events.extend(decisions::recover(&events).unwrap());
        events.extend(ex::recover(&events).unwrap());
        let projection = projection(&events);
        let decisions = decisions::project_validated(&events).unwrap();
        for d in decisions["decisions"].as_array().unwrap() {
            assert!(ex::provenance(&projection, d["decisionId"].as_str().unwrap()).is_some());
        }
        let current = job(&events, &source);
        if current["status"] != "completed" {
            let receipt = apply(
                &mut events,
                "project_extraction_retry",
                json!({"projectId":project,"jobId":j["jobId"],"expectedAttempt":1}),
            );
            assert_eq!(receipt["outcome"], "applied");
            start(&mut events, &source);
            finish(&mut events, &source, Ok(output.clone()));
        }
        let ds = decisions::project_validated(&events).unwrap();
        assert_eq!(ds["decisions"].as_array().unwrap().len(), 2);
        assert!(ds["decisions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["status"] == "proposed"));
        assert!(ex::recover(&events).unwrap().is_empty());
    }
}
#[test]
fn explicit_attempt_limit_and_normalized_dedup_cover_rejected_decisions() {
    let (mut events, project, source) = fixture();
    start(&mut events, &source);
    finish(&mut events, &source, Err("EXTRACTION_TIMEOUT".into()));
    for attempt in 1..3 {
        let j = job(&events, &source);
        let r = apply(
            &mut events,
            "project_extraction_retry",
            json!({"projectId":project,"jobId":j["jobId"],"expectedAttempt":attempt}),
        );
        assert_eq!(r["outcome"], "applied");
        start(&mut events, &source);
        finish(
            &mut events,
            &source,
            Err("EXTRACTION_PROVIDER_FAILED".into()),
        );
    }
    let j = job(&events, &source);
    let r = apply(
        &mut events,
        "project_extraction_retry",
        json!({"projectId":project,"jobId":j["jobId"],"expectedAttempt":3}),
    );
    assert_eq!(r["reasonCode"], "EXTRACTION_ATTEMPT_LIMIT");
    let second = message(
        &mut events,
        &project,
        "Use the canonical writer. Another source",
    );
    reconcile(&mut events);
    start(&mut events, &second);
    let done = finish(
        &mut events,
        &second,
        Ok(json!({"proposals":[proposal(&second,"Use Rust")]})),
    );
    let decision = done["job"]["proposalIds"][0].clone();
    let (next, _) = decisions::apply(
        &events,
        "project_decision_reject",
        &command(json!({"projectId":project,"decisionId":decision,"expectedDecisionRevision":1})),
    )
    .unwrap();
    events.extend(next);
    let third = message(
        &mut events,
        &project,
        "Use the canonical writer. Third source",
    );
    reconcile(&mut events);
    start(&mut events, &third);
    let r = finish(
        &mut events,
        &third,
        Ok(json!({"proposals":[proposal(&third,"USE   RUST")]})),
    );
    assert_eq!(r["job"]["reasonCode"], "EXTRACTION_DUPLICATE");
    assert_eq!(r["job"]["skippedDuplicates"], 1);
    assert_eq!(
        decisions::project_validated(&events).unwrap()["decisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn pending_owner_commands_recover_without_inference_and_forged_facts_fail_closed() {
    let (mut events, project, source) = fixture();
    let payload = command(
        json!({"projectId":project,"expectedPolicyRevision":1,"enabled":false,"consent":false}),
    );
    let (next, _) = ex::apply(&events, "project_extraction_configure", &payload).unwrap();
    for count in 1..next.len() {
        let mut prefix = events.clone();
        prefix.extend(next[..count].iter().cloned());
        prefix.extend(ex::recover(&prefix).unwrap());
        assert_eq!(projection(&prefix)["settings"][0]["enabled"], false);
        assert!(ex::recover(&prefix).unwrap().is_empty());
    }
    let queued = events
        .iter()
        .find(|e| e["type"] == "extraction.job_queued")
        .unwrap()
        .clone();
    let mut forged = queued.clone();
    forged["payload"]["sourceRevision"] = json!(2);
    let mut invalid = events.clone();
    invalid.push(forged);
    assert!(ex::project_validated(&invalid).is_err());
    let mut wrong = queued;
    wrong["schemaVersion"] = json!(6);
    events.push(wrong);
    assert!(ex::project_validated(&events).is_err());
    assert!(source.starts_with("source:"));
}

#[test]
fn global_queue_capacity_is_bounded_and_waiting_new_sources_are_not_lost() {
    let mut events = vec![];
    let project = project(&mut events);
    configure(&mut events, &project, 0, true);
    for n in 0..35 {
        message(&mut events, &project, &format!("Future source {n}"));
    }
    reconcile(&mut events);
    let before = projection(&events);
    assert_eq!(before["jobs"].as_array().unwrap().len(), 32);
    assert!(ex::reconcile(&events).unwrap().is_empty());
    let source = before["jobs"][0]["sourceId"].as_str().unwrap();
    start(&mut events, source);
    finish(&mut events, source, Ok(json!({"proposals":[]})));
    reconcile(&mut events);
    let after = projection(&events);
    assert_eq!(after["jobs"].as_array().unwrap().len(), 33);
    assert_eq!(
        after["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| j["status"] == "queued")
            .count(),
        32
    );
    assert_eq!(
        ex::metadata(&after)["projects"][0]["recentJobs"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
}

#[test]
fn accepted_context_is_scoped_bounded_and_explicitly_partial_with_exact_revisions() {
    let (mut events, project, source) = fixture();
    for n in 0..21 {
        let p = command(
            json!({"projectId":project,"expectedProjectRevision":revision(&events,&project),"title":format!("Owner decision {n}"),"context":"","decision":format!("Use the canonical writer with rule {n}"),"rationale":"","consequences":"","sources":[{"sourceId":source,"revision":1}]}),
        );
        let (next, r) = decisions::apply(&events, "project_decision_propose", &p).unwrap();
        events.extend(next);
        let(next,_)=decisions::apply(&events,"project_decision_accept",&command(json!({"projectId":project,"decisionId":r["decisionId"],"expectedDecisionRevision":1}))).unwrap();
        events.extend(next);
    }
    let started = start(&mut events, &source);
    let input = &started["input"];
    assert_eq!(input["contextPartial"], true);
    assert_eq!(input["acceptedDecisions"].as_array().unwrap().len(), 20);
    assert!(input["acceptedDecisions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|d| d["revision"] == 2));
    assert!(serde_json::to_vec(input).unwrap().len() <= ex::MAX_INPUT_BYTES);
    let mut related = proposal(&source, "Another proposed rule");
    related["relation"] = json!("possible_conflict");
    related["relatedDecisionIds"] = input["acceptedDecisions"]
        .as_array()
        .unwrap()
        .iter()
        .take(9)
        .map(|d| d["decisionId"].clone())
        .collect();
    let r = finish(&mut events, &source, Ok(json!({"proposals":[related]})));
    assert_eq!(r["job"]["reasonCode"], "EXTRACTION_INVALID_OUTPUT");
}

#[test]
fn a_full_legacy_source_that_exceeds_json_input_budget_fails_visibly_without_truncation() {
    let mut events = vec![];
    let project = project(&mut events);
    configure(&mut events, &project, 0, true);
    let work_id = "work:synthetic-large-source";
    events.push(memory::bind(&project, "work", work_id));
    let at = patchfleet_core::now();
    let cmd = format!("cmd:{}", uuid::Uuid::new_v4());
    let work = json!({"schemaVersion":1,"workItemId":work_id,"title":"Legacy owner task","instruction":"\u{0001}".repeat(50_000),"providerId":"codex","workingDirectory":"/fixture/project","status":"queued","createdAt":at,"revision":1});
    let intent = json!({"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"type":"enqueue_work","actorId":"local-owner","createdAt":at,"expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"payload":{"workItem":work}});
    events.push(patchfleet_core::event(
        "command.requested",
        json!({"intent":intent}),
    ));
    events.push(patchfleet_core::event(
        "work.enqueued",
        json!({"intentId":cmd,"workItem":work}),
    ));
    events.push(patchfleet_core::event("command.receipted",json!({"receipt":{"schemaVersion":1,"intentId":cmd,"idempotencyKey":cmd,"commandType":"enqueue_work","outcome":"applied","reasonCode":"WORK_ENQUEUED","completedAt":at,"workProjectionRevision":1,"workItemId":work_id}})));
    domain::project_events(&events).unwrap();
    reconcile(&mut events);
    let projection = projection(&events);
    let source = projection["jobs"][0]["sourceId"].as_str().unwrap();
    let result = start(&mut events, source);
    assert_eq!(result["job"]["status"], "failed");
    assert_eq!(result["job"]["reasonCode"], "EXTRACTION_INPUT_TOO_LARGE");
    assert!(result["input"].is_null());
    assert!(ex::reconcile(&events).unwrap().is_empty());
    assert_eq!(
        memory::project(&events).unwrap()["sources"][0]["text"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .count(),
        50_000
    );
}
