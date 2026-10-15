use patchfleet_core::{shared_memory as shared, Host};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[derive(Default)]
struct State {
    mode: AtomicUsize,
    revision: AtomicU64,
    shared: Mutex<Value>,
    records: Mutex<Vec<Value>>,
    receipts: Mutex<BTreeMap<String, Value>>,
    operations: Mutex<Vec<Value>>,
    requests: AtomicUsize,
    manifests: AtomicUsize,
    seen: tokio::sync::Notify,
}
struct Server {
    url: String,
    state: Arc<State>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let state = Arc::new(State::default());
    let data = state.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let s = data.clone();
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let mut block = [0u8; 4096];
                let (start, headers) = loop {
                    let n = socket.read(&mut block).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&block[..n]);
                    if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (i + 4, String::from_utf8(bytes[..i].to_vec()).unwrap());
                    }
                };
                let len = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < start + len {
                    let n = socket.read(&mut block).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&block[..n]);
                }
                let request = headers.lines().next().unwrap();
                let path = request.split_whitespace().nth(1).unwrap();
                let parsed = reqwest::Url::parse(&format!("http://127.0.0.1{path}")).unwrap();
                let path = parsed.path();
                let body: Value = if len == 0 {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes[start..start + len]).unwrap()
                };
                assert!(headers
                    .to_ascii_lowercase()
                    .contains("authorization: bearer fake_fixture_credential"));
                s.requests.fetch_add(1, Ordering::SeqCst);
                let mode = s.mode.load(Ordering::SeqCst);
                let mut status = 200;
                let response = if mode == 4 {
                    status = 403;
                    json!({"error":"MEMORY_ACCESS_DENIED"})
                } else if path.ends_with("/operations") {
                    assert!(request.starts_with("POST "));
                    assert!(headers.to_ascii_lowercase().contains("content-length:"));
                    shared::validate_operation(&body).unwrap();
                    s.operations.lock().unwrap().push(body.clone());
                    s.seen.notify_waiters();
                    if mode == 5 {
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                    let mut receipts = s.receipts.lock().unwrap();
                    let key = body["operationId"].as_str().unwrap().to_string();
                    let r = if let Some(r) = receipts.get(&key) {
                        r.clone()
                    } else {
                        let p = &body["payload"];
                        let shared = if p["type"] == "create_project" {
                            let id = json!(format!(
                                "shared:{}",
                                key.strip_prefix("memory-op:").unwrap()
                            ));
                            *s.shared.lock().unwrap() = id.clone();
                            id
                        } else {
                            p["sharedProjectId"].clone()
                        };
                        let reason = match p["type"].as_str().unwrap() {
                            "create_project" => "MEMORY_PROJECT_CREATED",
                            "link_reader" => "MEMORY_READER_LINKED",
                            "publish" => {
                                let revision = s.revision.fetch_add(1, Ordering::SeqCst) + 1;
                                s.records.lock().unwrap().push(json!({"schemaVersion":1,"sharedProjectId":shared,"manifestRevision":revision,"current":true,"publishedAt":patchfleet_core::now(),"decision":p["decision"]}));
                                "MEMORY_PUBLISHED"
                            }
                            _ => "MEMORY_SHARING_UPDATED",
                        };
                        let r = json!({"schemaVersion":1,"operationId":body["operationId"],"operationType":p["type"],"requestHash":shared::request_hash(&body).unwrap(),"outcome":"applied","reasonCode":reason,"sharedProjectId":shared,"revision":s.revision.load(Ordering::SeqCst),"role":"publisher","enabled":true,"completedAt":patchfleet_core::now()});
                        receipts.insert(key, r.clone());
                        r
                    };
                    if mode == 1 {
                        s.mode.store(0, Ordering::SeqCst);
                        return;
                    }
                    r
                } else if path.ends_with("/manifest") {
                    s.manifests.fetch_add(1, Ordering::SeqCst);
                    let offset = parsed
                        .query_pairs()
                        .find(|(k, _)| k == "offset")
                        .unwrap()
                        .1
                        .parse::<usize>()
                        .unwrap();
                    let revision = s.revision.load(Ordering::SeqCst);
                    let records = s.records.lock().unwrap();
                    let mut heads:Vec<_>=records.iter().filter(|r|r["current"]==true).map(|r|json!({"decisionId":r["decision"]["decisionId"],"revision":r["decision"]["revision"],"title":r["decision"]["title"],"publishedAt":r["publishedAt"]})).collect();
                    heads.sort_by(|a, b| a["decisionId"].as_str().cmp(&b["decisionId"].as_str()));
                    let mut m = json!({"schemaVersion":1,"sharedProjectId":s.shared.lock().unwrap().clone(),"revision":revision,"enabled":true,"offset":offset,"limit":20,"total":heads.len(),"decisions":heads.into_iter().skip(offset).take(20).collect::<Vec<_>>()});
                    if mode == 2 {
                        m["extra"] = json!("forged");
                    }
                    if mode == 3 {
                        m["padding"] = json!("x".repeat(shared::MAX_RESPONSE + 1));
                    }
                    if mode == 6 {
                        m["total"] = json!(64);
                    }
                    m
                } else if path.contains("/decisions/") {
                    let id = path.rsplit('/').next().unwrap();
                    let records = s.records.lock().unwrap();
                    let mut r = records
                        .iter()
                        .find(|r| r["decision"]["decisionId"] == id)
                        .unwrap()
                        .clone();
                    r["manifestRevision"] = json!(s.revision.load(Ordering::SeqCst));
                    r
                } else if path.ends_with("/heartbeat") || path.ends_with("/projection") {
                    json!({"schemaVersion":1,"accepted":true})
                } else if path.ends_with("/intents") {
                    json!({"schemaVersion":2,"hostId":"host:fixture","cursor":0,"intents":[]})
                } else {
                    panic!("Unexpected path {path}")
                };
                let body = serde_json::to_vec(&response).unwrap();
                let cache = if mode == 7 {
                    ""
                } else {
                    "Cache-Control: no-store\r\n"
                };
                let response=format!("HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\n{cache}Content-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                if socket.write_all(response.as_bytes()).await.is_ok() {
                    let _ = socket.write_all(&body).await;
                }
            });
        }
    });
    Server { url, state, task }
}
fn command(mut p: Value) -> Value {
    p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    p
}
async fn fixture(server: &Server) -> (tempfile::TempDir, Arc<Host>, Value, Value) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("cloud.json"),serde_json::to_vec(&json!({"schemaVersion":1,"installationId":"install:memory-fixture","connection":{"cloudUrl":server.url,"hostId":"host:fixture","workspaceId":"workspace:fixture","credential":"FAKE_FIXTURE_CREDENTIAL","protocolVersion":2,"cursor":0,"projectionRevisionOffset":0,"lastAttemptAt":null,"lastSuccessAt":null,"lastErrorCode":null}})).unwrap()).unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let project = host
        .action(
            "project_create",
            command(json!({"name":"Private local fixture"})),
        )
        .await
        .unwrap()["projectId"]
        .clone();
    let source=host.action("project_message",command(json!({"projectId":project,"expectedProjectRevision":1,"title":"Private raw source title","text":"PRIVATE_SOURCE_DO_NOT_UPLOAD · TOKEN_NEVER_PUBLISH"}))).await.unwrap()["sourceId"].clone();
    let decision=host.action("project_decision_propose",command(json!({"projectId":project,"expectedProjectRevision":2,"title":"Approved remote title","decision":"Approved remote ADR body","sources":[{"sourceId":source,"revision":1}]}))).await.unwrap()["decisionId"].clone();
    host.action(
        "project_decision_accept",
        command(json!({"projectId":project,"decisionId":decision,"expectedDecisionRevision":1})),
    )
    .await
    .unwrap();
    host.action(
        "memory_cloud_create",
        command(json!({"projectId":project,"expectedProjectRevision":2,"name":"Shared alias"})),
    )
    .await
    .unwrap();
    sync(&host, &project).await;
    (dir, host, project, decision)
}
async fn sync(host: &Host, project: &Value) -> Value {
    host.action("memory_cloud_sync", command(json!({"projectId":project})))
        .await
        .unwrap()
}
async fn publish(host: &Host, project: &Value, decision: &Value) {
    let r=host.action("memory_cloud_publish",command(json!({"projectId":project,"decisionId":decision,"expectedDecisionRevision":2,"expectedSharedRevision":0,"consent":true}))).await.unwrap();
    assert_eq!(r["outcome"], "applied");
}
fn metadata(host: &Host, project: &Value) -> Value {
    host.snapshot().unwrap()["sharedMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()
        .clone()
}
async fn exact(host: &Host, project: &Value, decision: &Value) -> Value {
    host.action(
        "memory_cloud_decision_read",
        json!({"projectId":project,"decisionId":decision,"revision":2}),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn lost_ack_restart_frozen_retry_and_transport_canaries() {
    let server = server().await;
    let (dir, host, project, decision) = fixture(&server).await;
    publish(&host, &project, &decision).await;
    server.state.mode.store(1, Ordering::SeqCst);
    sync(&host, &project).await;
    assert_eq!(
        metadata(&host, &project)["pendingOperation"]["state"],
        "attempted"
    );
    assert_eq!(metadata(&host, &project)["sharedRevision"], 0);
    host.shutdown().await;
    drop(host);
    let host = Host::open(dir.path().into()).unwrap();
    sync(&host, &project).await;
    assert_eq!(metadata(&host, &project)["sharedRevision"], 1);
    assert!(metadata(&host, &project)["pendingOperation"].is_null());
    let body = exact(&host, &project, &decision).await;
    assert_eq!(body["decision"]["decision"], "Approved remote ADR body");
    let operations = server.state.operations.lock().unwrap().clone();
    assert_eq!(operations.len(), 3);
    assert_eq!(operations[1], operations[2]);
    for denied in [
        "PRIVATE_SOURCE_DO_NOT_UPLOAD",
        "TOKEN_NEVER_PUBLISH",
        "FAKE_FIXTURE_CREDENTIAL",
        "origin",
        "workingDirectory",
    ] {
        assert!(!serde_json::to_string(&operations).unwrap().contains(denied));
    }
    drop(operations);
    let canonical = std::fs::read_to_string(dir.path().join("events.jsonl")).unwrap();
    assert!(!canonical.contains("FAKE_FIXTURE_CREDENTIAL"));
    host.shutdown().await;
}

#[tokio::test]
async fn invalid_oversized_partial_and_access_denied_replies_preserve_stale_complete_cache() {
    let server = server().await;
    let (_dir, host, project, decision) = fixture(&server).await;
    publish(&host, &project, &decision).await;
    sync(&host, &project).await;
    let good = exact(&host, &project, &decision).await;
    for mode in [2, 3, 4, 6, 7] {
        server.state.mode.store(mode, Ordering::SeqCst);
        sync(&host, &project).await;
        let cached = exact(&host, &project, &decision).await;
        assert_eq!(cached["decision"], good["decision"]);
        assert_eq!(cached["stale"], true);
        assert!(metadata(&host, &project)["lastErrorCode"]
            .as_str()
            .is_some());
    }
    server.state.mode.store(0, Ordering::SeqCst);
    sync(&host, &project).await;
    assert_eq!(exact(&host, &project, &decision).await["stale"], false);
    host.shutdown().await;
}

#[tokio::test]
async fn complete_twenty_one_head_manifest_is_committed_only_after_bounded_ticks() {
    let server = server().await;
    let (_dir, host, project, decision) = fixture(&server).await;
    let local = host
        .action(
            "project_decision_read",
            json!({"projectId":project,"decisionId":decision}),
        )
        .await
        .unwrap();
    let mut dto = local["decision"].clone();
    for key in [
        "projectId",
        "createdAt",
        "updatedAt",
        "sourceCount",
        "supersededByDecisionId",
        "actorId",
    ] {
        dto.as_object_mut().unwrap().remove(key);
    }
    dto["acceptedAt"] = local["history"][1]["at"].clone();
    dto["sources"] = json!(dto["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| json!({"sourceId":s["sourceId"],"revision":s["revision"]}))
        .collect::<Vec<_>>());
    let mut records = vec![];
    for _ in 0..21 {
        let mut d = dto.clone();
        d["decisionId"] = json!(format!("decision:{}", uuid::Uuid::new_v4()));
        shared::validate_dto(&d).unwrap();
        records.push(json!({"schemaVersion":1,"sharedProjectId":server.state.shared.lock().unwrap().clone(),"manifestRevision":1,"current":true,"publishedAt":patchfleet_core::now(),"decision":d}));
    }
    *server.state.records.lock().unwrap() = records;
    server.state.revision.store(1, Ordering::SeqCst);
    let before = server.state.requests.load(Ordering::SeqCst);
    sync(&host, &project).await;
    assert!(server.state.requests.load(Ordering::SeqCst) - before <= 4);
    let page = host
        .action("memory_cloud_read", json!({"projectId":project}))
        .await
        .unwrap();
    assert_eq!(page["total"], 0);
    for _ in 0..7 {
        let before = server.state.requests.load(Ordering::SeqCst);
        sync(&host, &project).await;
        assert!(server.state.requests.load(Ordering::SeqCst) - before <= 4);
        if metadata(&host, &project)["stale"] == false {
            break;
        }
    }
    let page = host
        .action("memory_cloud_read", json!({"projectId":project,"limit":20}))
        .await
        .unwrap();
    assert_eq!(page["total"], 21);
    assert_eq!(page["decisions"].as_array().unwrap().len(), 20);
    assert_eq!(page["stale"], false);
    host.shutdown().await;
}

#[tokio::test]
async fn shutdown_interrupts_uncertain_http_without_waiting_for_server_timeout() {
    let server = server().await;
    let (_dir, host, project, decision) = fixture(&server).await;
    publish(&host, &project, &decision).await;
    server.state.mode.store(5, Ordering::SeqCst);
    let notified = server.state.seen.notified();
    let h = host.clone();
    let p = project.clone();
    let action = tokio::spawn(async move { sync(&h, &p).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), notified)
        .await
        .unwrap();
    let started = std::time::Instant::now();
    host.request_shutdown();
    tokio::time::timeout(std::time::Duration::from_millis(500), host.shutdown())
        .await
        .unwrap();
    action.await.unwrap();
    assert!(started.elapsed() < std::time::Duration::from_millis(500));
    assert_eq!(
        metadata(&host, &project)["pendingOperation"]["state"],
        "attempted"
    );
}

#[tokio::test]
async fn aggregate_network_budget_leaves_attempt_durable_and_owner_ui_responsive() {
    let server = server().await;
    let (_dir, host, project, decision) = fixture(&server).await;
    publish(&host, &project, &decision).await;
    server.state.mode.store(5, Ordering::SeqCst);
    let at = std::time::Instant::now();
    sync(&host, &project).await;
    assert!(at.elapsed() < std::time::Duration::from_millis(3600));
    assert_eq!(
        metadata(&host, &project)["pendingOperation"]["state"],
        "attempted"
    );
    assert_eq!(
        metadata(&host, &project)["lastErrorCode"],
        "MEMORY_UNAVAILABLE"
    );
    host.action(
        "project_create",
        command(json!({"name":"Free local remains responsive"})),
    )
    .await
    .unwrap();
    host.shutdown().await;
}

#[tokio::test]
async fn disconnected_old_mapping_does_not_starve_background_sync_of_current_connection() {
    let server = server().await;
    let (dir, host, current, _) = fixture(&server).await;
    host.shutdown().await;
    drop(host);
    let store = patchfleet_core::store::Store::open(dir.path().into()).unwrap();
    let old=store.transaction(|events|patchfleet_core::project_memory::apply(events,"project_create",&json!({"commandId":"cmd:00000000-0000-4000-8000-000000000001","name":"Old connection project"}))).unwrap()["projectId"].clone();
    let bound = json!({"cloudUrl":server.url,"workspaceId":"workspace:old","hostId":"host:old"});
    store.transaction(|events|shared::apply(events,"memory_cloud_create",&command(json!({"projectId":old,"expectedProjectRevision":1,"name":"Old shared project"})),&bound)).unwrap();
    drop(store);
    let host = Host::open(dir.path().into()).unwrap();
    assert_eq!(
        host.snapshot().unwrap()["sharedMemory"]["projects"][0]["projectId"],
        old
    );
    assert_eq!(metadata(&host, &old)["state"], "connection_changed");
    let before = server.state.manifests.load(Ordering::SeqCst);
    host.background_tick(false).await.unwrap();
    assert_eq!(server.state.manifests.load(Ordering::SeqCst), before + 1);
    assert_eq!(metadata(&host, &current)["stale"], false);
    assert_eq!(metadata(&host, &old)["state"], "connection_changed");
    host.shutdown().await;
}
