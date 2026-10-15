use patchfleet_core::{retrieval, shared_memory, Host};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Server {
    url: String,
    shared: Arc<Mutex<Value>>,
    response: Arc<Mutex<Value>>,
    queries: Arc<Mutex<Vec<Value>>>,
    held: Arc<AtomicBool>,
    seen: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
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
    let shared = Arc::new(Mutex::new(Value::Null));
    let response = Arc::new(Mutex::new(Value::Null));
    let queries = Arc::new(Mutex::new(Vec::new()));
    let held = Arc::new(AtomicBool::new(false));
    let seen = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let state = (
        shared.clone(),
        response.clone(),
        queries.clone(),
        held.clone(),
        seen.clone(),
        release.clone(),
    );
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (shared, response, queries, held, seen, release) = state.clone();
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let mut chunk = [0u8; 4096];
                let (start, headers) = loop {
                    let n = socket.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
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
                    let n = socket.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                }
                assert!(headers
                    .to_ascii_lowercase()
                    .contains("authorization: bearer fake_retrieval_credential"));
                let path = headers
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                let path = path.split('?').next().unwrap();
                let body: Value = if len > 0 {
                    serde_json::from_slice(&bytes[start..start + len]).unwrap()
                } else {
                    Value::Null
                };
                let value = if path.ends_with("/operations") {
                    let id = json!(format!(
                        "shared:{}",
                        body["operationId"]
                            .as_str()
                            .unwrap()
                            .strip_prefix("memory-op:")
                            .unwrap()
                    ));
                    *shared.lock().unwrap() = id.clone();
                    json!({"schemaVersion":1,"operationId":body["operationId"],"operationType":"create_project","requestHash":shared_memory::request_hash(&body).unwrap(),"outcome":"applied","reasonCode":"MEMORY_PROJECT_CREATED","sharedProjectId":id,"revision":0,"role":"publisher","enabled":true,"completedAt":patchfleet_core::now()})
                } else if path.ends_with("/manifest") {
                    json!({"schemaVersion":1,"sharedProjectId":shared.lock().unwrap().clone(),"revision":0,"enabled":true,"offset":0,"limit":20,"total":0,"decisions":[]})
                } else if path.ends_with("/search") {
                    assert!(headers.starts_with("POST "));
                    assert!(headers
                        .to_ascii_lowercase()
                        .contains("cache-control: no-store"));
                    assert_eq!(body.as_object().unwrap().len(), 3);
                    assert_eq!(body["schemaVersion"], 1);
                    queries.lock().unwrap().push(body);
                    let released = release.notified();
                    seen.notify_one();
                    if held.load(Ordering::SeqCst) {
                        released.await;
                    }
                    response.lock().unwrap().clone()
                } else {
                    panic!("Unexpected fixture path {path}")
                };
                let body = serde_json::to_vec(&value).unwrap();
                let header=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                if socket.write_all(header.as_bytes()).await.is_ok() {
                    let _ = socket.write_all(&body).await;
                }
            });
        }
    });
    Server {
        url,
        shared,
        response,
        queries,
        held,
        seen,
        release,
        task,
    }
}
fn command(mut p: Value) -> Value {
    p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    p
}
async fn fixture(server: &Server) -> (tempfile::TempDir, Arc<Host>, Value, Value) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("cloud.json"),serde_json::to_vec(&json!({"schemaVersion":1,"installationId":"install:retrieval-fixture","connection":{"cloudUrl":server.url,"hostId":"host:fixture","workspaceId":"workspace:fixture","credential":"FAKE_RETRIEVAL_CREDENTIAL","protocolVersion":2,"cursor":0,"projectionRevisionOffset":0,"lastAttemptAt":null,"lastSuccessAt":null,"lastErrorCode":null}})).unwrap()).unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let project = host
        .action(
            "project_create",
            command(json!({"name":"Local retrieval fixture"})),
        )
        .await
        .unwrap()["projectId"]
        .clone();
    let source=host.action("project_message",command(json!({"projectId":project,"expectedProjectRevision":1,"title":"Owner source","text":"SOURCE_MUST_STAY_LOCAL"}))).await.unwrap()["sourceId"].clone();
    let decision=host.action("project_decision_propose",command(json!({"projectId":project,"expectedProjectRevision":2,"title":"Rust bellek","decision":"Rust keeps local memory small","sources":[{"sourceId":source,"revision":1}]}))).await.unwrap()["decisionId"].clone();
    host.action(
        "project_decision_accept",
        command(json!({"projectId":project,"decisionId":decision,"expectedDecisionRevision":1})),
    )
    .await
    .unwrap();
    host.action("memory_cloud_create",command(json!({"projectId":project,"expectedProjectRevision":2,"name":"Shared retrieval fixture"}))).await.unwrap();
    host.action("memory_cloud_sync", command(json!({"projectId":project})))
        .await
        .unwrap();
    *server.response.lock().unwrap() = json!({"schemaVersion":1,"sharedProjectId":server.shared.lock().unwrap().clone(),"manifestRevision":1,"mode":"semantic","model":{"id":retrieval::MODEL_ID,"revision":retrieval::MODEL_REVISION,"dimensions":384,"normalization":"l2","dtype":"q8","pooling":"mean","chunkVersion":1},"index":{"state":"ready","indexed":1,"total":1},"matches":[{"decisionId":decision,"revision":2,"title":"Rust bellek","score":0.87}]});
    (dir, host, project, decision)
}
fn files(dir: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().to_string(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
async fn search(host: &Host, project: &Value) -> patchfleet_core::Result<Value> {
    host.action(
        "memory_cloud_search",
        json!({"projectId":project,"query":"  RETRIEVAL_QUERY_CANARY neden rust?  "}),
    )
    .await
}

#[tokio::test]
async fn explicit_local_and_cloud_reads_leave_no_query_in_durable_or_operational_state() {
    let server = server().await;
    let (dir, host, project, decision) = fixture(&server).await;
    let before = files(dir.path());
    let local = host
        .action(
            "project_decision_search",
            command(json!({"projectId":project,"query":"RETRIEVAL_QUERY_CANARY rust"})),
        )
        .await
        .unwrap();
    assert_eq!(local["matches"][0]["decisionId"], decision);
    let remote = search(&host, &project).await.unwrap();
    assert_eq!(remote["reasonCode"], "SHARED_MEMORY_SEARCH_READ");
    assert_eq!(remote["projectId"], project);
    assert_eq!(remote["manifestRevision"], 1); // Newer than the downloaded revision0 is valid.
    assert_eq!(
        remote["revision"],
        host.snapshot().unwrap()["sharedMemory"]["revision"]
    );
    assert_eq!(remote["query"], "RETRIEVAL_QUERY_CANARY neden rust?");
    assert_eq!(files(dir.path()), before);
    let snapshot = host.snapshot().unwrap();
    let snapshot_text = serde_json::to_string(&snapshot).unwrap();
    let operational =
        patchfleet_core::cloud::build_projection(&snapshot, 2, 1, "install:retrieval-fixture")
            .unwrap();
    assert!(!snapshot_text.contains("RETRIEVAL_QUERY_CANARY"));
    assert!(!serde_json::to_string(&operational)
        .unwrap()
        .contains("RETRIEVAL_QUERY_CANARY"));
    let requests = server.queries.lock().unwrap().clone();
    assert_eq!(
        requests,
        json!([{"schemaVersion":1,"query":"RETRIEVAL_QUERY_CANARY neden rust?","limit":5}])
            .as_array()
            .unwrap()
            .clone()
    );
    assert!(!serde_json::to_string(&requests)
        .unwrap()
        .contains("SOURCE_MUST_STAY_LOCAL"));
    host.shutdown().await;
}

#[tokio::test]
async fn malformed_oversized_foreign_and_wrong_model_responses_fail_without_cache_changes() {
    let server = server().await;
    let (dir, host, project, _) = fixture(&server).await;
    let before = files(dir.path());
    let good = server.response.lock().unwrap().clone();
    for (path, value) in [
        ("/model/revision", json!("main")),
        (
            "/sharedProjectId",
            json!("shared:10000000-0000-4000-8000-000000000001"),
        ),
        ("/matches/0/title", json!("x".repeat(193 * 1024))),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        *server.response.lock().unwrap() = bad;
        assert_eq!(
            search(&host, &project).await.unwrap_err().0,
            "MEMORY_PROTOCOL_INVALID"
        );
    }
    assert_eq!(files(dir.path()), before);
    host.shutdown().await;
}

#[tokio::test]
async fn local_commands_continue_during_cloud_read_and_changed_pairing_discards_result() {
    let server = server().await;
    let (_dir, host, project, _) = fixture(&server).await;
    server.held.store(true, Ordering::SeqCst);
    let h = host.clone();
    let p = project.clone();
    let request = tokio::spawn(async move { search(&h, &p).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), server.seen.notified())
        .await
        .unwrap();
    let local=tokio::time::timeout(std::time::Duration::from_millis(300),host.action("project_message",command(json!({"projectId":project,"expectedProjectRevision":2,"title":"During search","text":"Local remains usable"})))).await.unwrap().unwrap();
    assert_eq!(local["outcome"], "applied");
    let h = host.clone();
    let unpair = tokio::spawn(async move { h.action("cloud_unpair", json!({})).await });
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    server.release.notify_one();
    unpair.await.unwrap().unwrap();
    assert_eq!(
        request.await.unwrap().unwrap_err().0,
        "MEMORY_CONNECTION_CHANGED"
    );
    host.shutdown().await;
}

#[tokio::test]
async fn shutdown_cancels_search_without_waiting_for_the_three_second_budget() {
    let server = server().await;
    let (dir, host, project, _) = fixture(&server).await;
    let before = files(dir.path());
    server.held.store(true, Ordering::SeqCst);
    let h = host.clone();
    let p = project.clone();
    let request = tokio::spawn(async move { search(&h, &p).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), server.seen.notified())
        .await
        .unwrap();
    let h = host.clone();
    let p = project.clone();
    let waiting_for_credential = tokio::spawn(async move { search(&h, &p).await });
    tokio::task::yield_now().await;
    host.request_shutdown();
    let result = tokio::time::timeout(std::time::Duration::from_millis(500), request)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(result.0, "HOST_CLOSING");
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_millis(500),
            waiting_for_credential
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap_err()
        .0,
        "HOST_CLOSING"
    );
    tokio::time::timeout(std::time::Duration::from_millis(500), host.shutdown())
        .await
        .unwrap();
    assert_eq!(files(dir.path()), before);
    server.release.notify_one();
}

#[tokio::test]
async fn changed_project_publication_scope_discards_inflight_search() {
    let server = server().await;
    let (dir, host, project, _) = fixture(&server).await;
    server.held.store(true, Ordering::SeqCst);
    let h = host.clone();
    let p = project.clone();
    let request = tokio::spawn(async move { search(&h, &p).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), server.seen.notified())
        .await
        .unwrap();
    let h = host.clone();
    let p = project.clone();
    let sharing = tokio::spawn(async move {
        h.action(
            "memory_cloud_sharing",
            command(json!({"projectId":p,"expectedSharedRevision":0,"enabled":false})),
        )
        .await
    });
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    server.release.notify_one();
    assert_eq!(sharing.await.unwrap().unwrap()["outcome"], "applied");
    assert_eq!(request.await.unwrap().unwrap_err().0, "MEMORY_CONFLICT");
    assert!(!std::fs::read_to_string(dir.path().join("events.jsonl"))
        .unwrap()
        .contains("RETRIEVAL_QUERY_CANARY"));
    host.shutdown().await;
}

#[tokio::test]
async fn search_budget_is_bounded_and_unpaired_local_search_stays_free() {
    let server = server().await;
    let (_dir, host, project, _) = fixture(&server).await;
    server.held.store(true, Ordering::SeqCst);
    let started = std::time::Instant::now();
    assert_eq!(
        search(&host, &project).await.unwrap_err().0,
        "MEMORY_UNAVAILABLE"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(4));
    server.release.notify_one();
    host.action("cloud_unpair", json!({})).await.unwrap();
    assert_eq!(
        search(&host, &project).await.unwrap_err().0,
        "CLOUD_UNPAIRED"
    );
    let local = host
        .action(
            "project_decision_search",
            json!({"projectId":project,"query":"Rust"}),
        )
        .await
        .unwrap();
    assert_eq!(local["matches"].as_array().unwrap().len(), 1);
    host.shutdown().await;
}
