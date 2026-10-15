use patchfleet_core::{now, Host};
use serde_json::{json, Value};

fn command(mut args: Value) -> Value {
    args["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    let at = chrono::Utc::now();
    args["createdAt"] = json!(at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    args["expiresAt"] =
        json!((at + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    args
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
async fn propose(host: &Host, project: &Value, source: &Value, title: &str) -> Value {
    let args = command(
        json!({"projectId":project,"expectedProjectRevision":project_revision(host,project),
        "title":title,"context":"PRIVATE_CONTEXT_CANARY","decision":"PRIVATE_ADR_CANARY · integer minor units",
        "rationale":"PRIVATE_RATIONALE_CANARY","consequences":"PRIVATE_CONSEQUENCES_CANARY",
        "sources":[{"sourceId":source["sourceId"],"revision":source["revision"]}]}),
    );
    let receipt = host
        .action("project_decision_propose", args.clone())
        .await
        .unwrap();
    assert_eq!(receipt["outcome"], "applied", "{receipt}");
    assert_eq!(
        host.action("project_decision_propose", args).await.unwrap(),
        receipt
    );
    receipt
}

#[tokio::test]
async fn local_owner_decisions_survive_restart_with_original_evidence_and_no_cloud_upload() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(directory.path().to_owned()).unwrap();
    let project = host
        .action(
            "project_create",
            command(json!({"name":"Decision fixture"})),
        )
        .await
        .unwrap()["projectId"]
        .clone();
    host.action(
        "project_message",
        command(json!({"projectId":project,"expectedProjectRevision":1,
        "title":"Original source","text":"PRIVATE_SOURCE_CANARY · original evidence"})),
    )
    .await
    .unwrap();
    let page = host
        .action("project_memory_read", json!({"projectId":project}))
        .await
        .unwrap();
    let source = page["sources"][0].clone();
    let first = propose(&host, &project, &source, "Original ADR").await;
    let accept = command(
        json!({"projectId":project,"decisionId":first["decisionId"],"expectedDecisionRevision":1}),
    );
    let receipt = host
        .action("project_decision_accept", accept.clone())
        .await
        .unwrap();
    assert_eq!(receipt["outcome"], "applied");
    let stale = propose(&host, &project, &source, "Stale draft").await;
    host.action("project_source_edit", command(json!({"projectId":project,"sourceId":source["sourceId"],
        "expectedSourceRevision":1,"title":"Edited source","text":"PRIVATE_EDIT_CANARY · changed guidance"}))).await.unwrap();
    let stale_accept = host
        .action(
            "project_decision_accept",
            command(json!({"projectId":project,
        "decisionId":stale["decisionId"],"expectedDecisionRevision":1})),
        )
        .await
        .unwrap();
    assert_eq!(stale_accept["outcome"], "rejected");
    assert_eq!(stale_accept["reasonCode"], "STALE_SOURCE_REVISION");
    let rejected = host
        .action(
            "project_decision_reject",
            command(json!({"projectId":project,
        "decisionId":stale["decisionId"],"expectedDecisionRevision":1})),
        )
        .await
        .unwrap();
    assert_eq!(rejected["outcome"], "applied");
    let current_source = host
        .action("project_memory_read", json!({"projectId":project}))
        .await
        .unwrap()["sources"][0]
        .clone();
    let replacement = propose(&host, &project, &current_source, "Replacement ADR").await;
    let supersede = command(
        json!({"projectId":project,"decisionId":first["decisionId"],"expectedDecisionRevision":2,
        "replacementDecisionId":replacement["decisionId"],"expectedReplacementRevision":1}),
    );
    let replaced = host
        .action("project_decision_supersede", supersede.clone())
        .await
        .unwrap();
    assert_eq!(replaced["outcome"], "applied", "{replaced}");
    let detail_args = json!({"projectId":project,"decisionId":first["decisionId"]});
    let detail = host
        .action("project_decision_read", detail_args.clone())
        .await
        .unwrap();
    assert_eq!(detail["decision"]["status"], "superseded");
    assert_eq!(
        detail["decision"]["supersededByDecisionId"],
        replacement["decisionId"]
    );
    assert_eq!(detail["sources"][0]["title"], "Original source");
    assert_eq!(detail["sources"][0]["text"], source["text"]);
    assert_eq!(detail["history"].as_array().unwrap().len(), 3);
    let active = host
        .action(
            "project_decisions_read",
            json!({"projectId":project,"status":"accepted","query":"integer"}),
        )
        .await
        .unwrap();
    assert_eq!(active["total"], 1);
    assert_eq!(
        active["decisions"][0]["decisionId"],
        replacement["decisionId"]
    );
    assert!(active["decisions"][0]["decision"].is_null());
    let snapshot = host.snapshot().unwrap();
    assert_eq!(project_revision(&host, &project), 3);
    assert!(snapshot["projectDecisions"]["decisions"].is_null());
    assert!(!snapshot["projectDecisions"]
        .to_string()
        .contains("PRIVATE_"));
    for version in [1, 2] {
        let mut input = snapshot.clone();
        input["sequence"] = json!(1);
        input["recordedAt"] = json!(now());
        input["agentPacks"] = input["packs"].clone();
        input["projectDecisions"] = detail.clone();
        input["decisionPage"] = active.clone();
        let outgoing = patchfleet_core::cloud::build_projection(&input, version, 1, "install:test")
            .unwrap()
            .to_string();
        for denied in [
            "PRIVATE_",
            "projectDecisions",
            "decisionPage",
            project.as_str().unwrap(),
            first["decisionId"].as_str().unwrap(),
        ] {
            assert!(!outgoing.contains(denied), "{outgoing}");
        }
    }
    let other = host
        .action("project_create", command(json!({"name":"Other scope"})))
        .await
        .unwrap()["projectId"]
        .clone();
    assert!(host
        .action(
            "project_decision_read",
            json!({"projectId":other,"decisionId":first["decisionId"]})
        )
        .await
        .is_err());
    assert!(host
        .action(
            "project_decision_accept",
            json!({"projectId":project,"decisionId":replacement["decisionId"],
        "expectedDecisionRevision":2,"actorId":"forged"})
        )
        .await
        .is_err());
    let bytes_before = std::fs::read(directory.path().join("events.jsonl")).unwrap();
    for _ in 0..3 {
        host.action(
            "project_decisions_read",
            json!({"projectId":project,"query":"PRIVATE_"}),
        )
        .await
        .unwrap();
        host.action("project_decision_read", detail_args.clone())
            .await
            .unwrap();
    }
    assert_eq!(
        std::fs::read(directory.path().join("events.jsonl")).unwrap(),
        bytes_before
    );
    host.shutdown().await;
    drop(host);
    let restored = Host::open(directory.path().to_owned()).unwrap();
    assert_eq!(
        restored
            .action("project_decision_read", detail_args)
            .await
            .unwrap(),
        detail
    );
    assert_eq!(
        restored
            .action("project_decision_accept", accept)
            .await
            .unwrap(),
        receipt
    );
    assert_eq!(
        restored
            .action("project_decision_supersede", supersede)
            .await
            .unwrap(),
        replaced
    );
    assert_eq!(
        restored
            .action(
                "project_decisions_read",
                json!({"projectId":project,"status":"accepted","query":"integer"})
            )
            .await
            .unwrap(),
        active
    );
    restored.shutdown().await;
}

#[tokio::test]
async fn maximum_multilingual_adr_round_trips_and_unknown_fields_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(directory.path().to_owned()).unwrap();
    let project = host
        .action("project_create", command(json!({"name":"TR / EN / CJK"})))
        .await
        .unwrap()["projectId"]
        .clone();
    let source = host.action("project_message", command(json!({"projectId":project,"expectedProjectRevision":1,"title":"Scope","text":"Türkçe ve English source"}))).await.unwrap()["sourceId"].clone();
    let args = command(
        json!({"projectId":project,"expectedProjectRevision":2,"title":"多言語 ADR",
        "context":"甲".repeat(10_000),"decision":"乙".repeat(20_000),"rationale":"丙".repeat(10_000),"consequences":"丁".repeat(10_000),
        "sources":[{"sourceId":source,"revision":1}]}),
    );
    assert!(
        serde_json::to_vec(&args).unwrap().len()
            < Host::action_payload_limit("project_decision_propose")
    );
    let receipt = host
        .action("project_decision_propose", args.clone())
        .await
        .unwrap();
    assert_eq!(receipt["outcome"], "applied", "{receipt}");
    let detail = host
        .action(
            "project_decision_read",
            json!({"projectId":project,"decisionId":receipt["decisionId"]}),
        )
        .await
        .unwrap();
    for field in ["title", "context", "decision", "rationale", "consequences"] {
        assert_eq!(detail["decision"][field], args[field]);
    }
    let mut forged = args;
    forged["accepted"] = json!(true);
    assert_eq!(
        host.action("project_decision_propose", forged)
            .await
            .unwrap_err()
            .0,
        "INVALID_PAYLOAD"
    );
    host.shutdown().await;
}
