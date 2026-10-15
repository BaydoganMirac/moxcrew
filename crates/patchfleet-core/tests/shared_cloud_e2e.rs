//! Run only through the private disposable PostgreSQL/HTTP fixture driver.
use patchfleet_core::Host;
use serde_json::{json, Value};

fn command(mut value: Value) -> Value {
    let now = chrono::Utc::now();
    value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    value["createdAt"] = json!(now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    value["expiresAt"] =
        json!((now + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    value
}
fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing fixture variable {name}"))
}
fn mapping(host: &Host, project: &Value) -> Value {
    host.snapshot().unwrap()["sharedMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()
        .clone()
}
fn source_revision(host: &Host, project: &Value) -> Value {
    host.snapshot().unwrap()["projectMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()["revision"]
        .clone()
}
async fn local_project(host: &Host, name: &str) -> Value {
    host.action("project_create", command(json!({"name":name})))
        .await
        .unwrap()["projectId"]
        .clone()
}
async fn sync(host: &Host, project: &Value) -> patchfleet_core::Result<Value> {
    host.action("memory_cloud_sync", command(json!({"projectId":project})))
        .await
}
async fn control(base: &str, token: &str, action: &str) {
    let result = reqwest::Client::new()
        .post(format!("{base}/_fixture/control"))
        .header("x-fixture-token", token)
        .json(&json!({"action":action}))
        .send()
        .await
        .unwrap();
    assert!(result.status().is_success());
}
async fn page(host: &Host, project: &Value) -> Value {
    host.action("memory_cloud_read", json!({"projectId":project}))
        .await
        .unwrap()
}
async fn exact(host: &Host, project: &Value, decision: &Value) -> Value {
    host.action(
        "memory_cloud_decision_read",
        json!({"projectId":project,"decisionId":decision,"revision":2}),
    )
    .await
    .unwrap()
}
async fn draft(host: &Host, project: &Value, source: &Value, title: &str) -> Value {
    let receipt=host.action("project_decision_propose",command(json!({"projectId":project,
        "expectedProjectRevision":source_revision(host,project),"title":title,"context":"Approved Cloud fixture context",
        "decision":"Store amounts in integer minor units.","rationale":"Exact arithmetic.","consequences":"Convert at input and output.",
        "sources":[{"sourceId":source,"revision":1}]}))).await.unwrap();
    assert_eq!(receipt["outcome"], "applied");
    receipt["decisionId"].clone()
}

#[tokio::test]
#[ignore = "requires the isolated private memory-rust-smoke PostgreSQL/HTTP fixture"]
async fn two_installations_share_exact_decisions_through_postgres_and_recover_lost_ack() {
    let base = env("PATCHFLEET_MEMORY_E2E_URL");
    let parsed = reqwest::Url::parse(&base).unwrap();
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert_eq!(parsed.path(), "/");
    let token = env("PATCHFLEET_MEMORY_E2E_TOKEN");
    let data_a = tempfile::tempdir().unwrap();
    let data_b = tempfile::tempdir().unwrap();
    let mut a = Host::open(data_a.path().to_owned()).unwrap();
    let b = Host::open(data_b.path().to_owned()).unwrap();
    let project_a = local_project(&a, "Publisher local project").await;
    let project_b = local_project(&b, "Reader private project").await;
    for (host, code, name) in [
        (&a, env("PATCHFLEET_MEMORY_E2E_PAIR_A"), "Publisher fixture"),
        (&b, env("PATCHFLEET_MEMORY_E2E_PAIR_B"), "Reader fixture"),
    ] {
        host.action(
            "cloud_pair",
            command(json!({"cloudUrl":base,"pairingCode":code,"displayName":name})),
        )
        .await
        .unwrap();
    }
    let source=a.action("project_message",command(json!({"projectId":project_a,"expectedProjectRevision":1,
        "title":"Private evidence","text":"PRIVATE_RAW_SOURCE_CANARY_0031 · do not upload source bodies."}))).await.unwrap()["sourceId"].clone();
    let first = draft(&a, &project_a, &source, "Published amount decision").await;
    a.action(
        "project_decision_accept",
        command(json!({"projectId":project_a,"decisionId":first,"expectedDecisionRevision":1})),
    )
    .await
    .unwrap();
    a.action("memory_cloud_create",command(json!({"projectId":project_a,"expectedProjectRevision":source_revision(&a,&project_a),"name":"Shared billing fixture"}))).await.unwrap();
    assert_eq!(mapping(&a, &project_a)["state"], "queued");
    sync(&a, &project_a).await.unwrap();
    let shared = mapping(&a, &project_a)["sharedProjectId"].clone();
    assert!(shared.as_str().is_some_and(|s| s.starts_with("shared:")));
    b.action(
        "memory_cloud_link",
        command(
            json!({"projectId":project_b,"expectedProjectRevision":1,"sharedProjectId":shared}),
        ),
    )
    .await
    .unwrap();
    sync(&b, &project_b).await.unwrap();
    assert_eq!(mapping(&b, &project_b)["role"], "reader");

    let publish = command(
        json!({"projectId":project_a,"decisionId":first,"expectedDecisionRevision":2,"expectedSharedRevision":0,"consent":true}),
    );
    let queued = a
        .action("memory_cloud_publish", publish.clone())
        .await
        .unwrap();
    assert_eq!(mapping(&a, &project_a)["sharedRevision"], 0);
    control(&base, &token, "drop_next_operation_response").await;
    let _ = sync(&a, &project_a).await;
    assert!(mapping(&a, &project_a)["pendingOperation"].is_object());
    assert_eq!(mapping(&a, &project_a)["sharedRevision"], 0);
    a.shutdown().await;
    drop(a);
    a = Host::open(data_a.path().to_owned()).unwrap();
    sync(&a, &project_a).await.unwrap();
    assert_eq!(mapping(&a, &project_a)["sharedRevision"], 1);
    assert_eq!(
        a.action("memory_cloud_publish", publish).await.unwrap(),
        queued
    );
    sync(&b, &project_b).await.unwrap();
    let list = page(&b, &project_b).await;
    assert_eq!(list["total"], 1);
    assert_eq!(list["decisions"][0]["decisionId"], first);
    let downloaded = exact(&b, &project_b, &first).await;
    assert_eq!(downloaded["decision"]["title"], "Published amount decision");
    assert_eq!(downloaded["decision"]["revision"], 2);
    assert_eq!(downloaded["current"], true);
    for forbidden in [
        "PRIVATE_RAW_SOURCE_CANARY_0031",
        "origin",
        "providerSessionId",
    ] {
        assert!(!downloaded.to_string().contains(forbidden));
    }
    assert_eq!(
        b.snapshot().unwrap()["projectDecisions"]["projects"][0]["decisionCount"],
        0
    );

    let replacement = draft(&a, &project_a, &source, "Replacement amount decision").await;
    a.action(
        "project_decision_supersede",
        command(
            json!({"projectId":project_a,"decisionId":first,"expectedDecisionRevision":2,
        "replacementDecisionId":replacement,"expectedReplacementRevision":1}),
        ),
    )
    .await
    .unwrap();
    a.action(
        "memory_cloud_publish",
        command(
            json!({"projectId":project_a,"decisionId":replacement,"expectedDecisionRevision":2,
        "expectedSharedRevision":1,"consent":true}),
        ),
    )
    .await
    .unwrap();
    a.shutdown().await;
    drop(a);
    a = Host::open(data_a.path().to_owned()).unwrap();
    sync(&a, &project_a).await.unwrap();
    sync(&b, &project_b).await.unwrap();
    let latest = page(&b, &project_b).await;
    assert_eq!(latest["total"], 1);
    assert_eq!(latest["decisions"][0]["decisionId"], replacement);
    assert_eq!(mapping(&b, &project_b)["sharedRevision"], 2);
    assert_eq!(
        exact(&b, &project_b, &replacement).await["decision"]["supersedesDecisionId"],
        first
    );

    a.action(
        "memory_cloud_sharing",
        command(json!({"projectId":project_a,"expectedSharedRevision":2,"enabled":false})),
    )
    .await
    .unwrap();
    let _ = sync(&a, &project_a).await;
    let _ = sync(&b, &project_b).await;
    assert_eq!(
        mapping(&b, &project_b)["lastErrorCode"],
        "MEMORY_SHARING_DISABLED"
    );
    assert_eq!(page(&b, &project_b).await["total"], 1);
    assert_eq!(exact(&b, &project_b, &replacement).await["stale"], true);
    a.action(
        "memory_cloud_sharing",
        command(json!({"projectId":project_a,"expectedSharedRevision":3,"enabled":true})),
    )
    .await
    .unwrap();
    sync(&a, &project_a).await.unwrap();
    sync(&b, &project_b).await.unwrap();
    control(&base, &token, "subscription_inactive").await;
    let _ = sync(&b, &project_b).await;
    assert_eq!(
        mapping(&b, &project_b)["lastErrorCode"],
        "MEMORY_SUBSCRIPTION_REQUIRED"
    );
    assert_eq!(
        exact(&b, &project_b, &replacement).await["decision"]["title"],
        "Replacement amount decision"
    );
    let local = b
        .action(
            "project_message",
            command(json!({"projectId":project_b,"expectedProjectRevision":1,
        "title":"Still free","text":"Local keeps working without a subscription."})),
        )
        .await
        .unwrap();
    assert_eq!(local["outcome"], "applied");
    a.shutdown().await;
    b.shutdown().await;
}
