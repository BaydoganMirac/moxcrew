use patchfleet_core::{desktop_login, Host};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Default)]
struct State {
    mode: AtomicUsize,
    held: AtomicBool,
    challenge: Mutex<String>,
    requests: Mutex<Vec<Value>>,
    seen: tokio::sync::Notify,
    release: tokio::sync::Notify,
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
async fn make_server() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let state = Arc::new(State::default());
    let observed = state.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let state = observed.clone();
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let mut chunk = [0u8; 4096];
                let (start, head) = loop {
                    let n = socket.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (i + 4, String::from_utf8(bytes[..i].to_vec()).unwrap());
                    }
                };
                let length = head
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < start + length {
                    let n = socket.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                }
                let path = head
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                let body: Value = if length == 0 {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes[start..start + length]).unwrap()
                };
                state
                    .requests
                    .lock()
                    .unwrap()
                    .push(json!({"path":path,"body":body}));
                let released = state.release.notified();
                state.seen.notify_one();
                if state.held.load(Ordering::SeqCst) && path.starts_with("/api/desktop/") {
                    released.await;
                }
                let mode = state.mode.load(Ordering::SeqCst);
                let mut status = 200;
                let response = if path == "/api/desktop/v1/token" {
                    assert_eq!(body.as_object().unwrap().len(), 4);
                    assert_eq!(body["schemaVersion"], 1);
                    assert_eq!(body["code"], "A".repeat(43));
                    assert_eq!(
                        desktop_login::challenge(body["verifier"].as_str().unwrap()).unwrap(),
                        *state.challenge.lock().unwrap()
                    );
                    assert!(!head.to_ascii_lowercase().contains("authorization:"));
                    if mode == 2 {
                        status = 403;
                        json!({"error":"DESKTOP_CODE_EXPIRED"})
                    } else {
                        json!({"schemaVersion":1,"hostId":"host:login","workspaceId":"workspace:login","credential":"CREDENTIAL_ONLY_CLOUD_FILE","protocolVersion":if mode==4 {1}else{2}})
                    }
                } else if path.ends_with("/account") {
                    assert!(head
                        .to_ascii_lowercase()
                        .contains("authorization: bearer credential_only_cloud_file"));
                    if mode == 5 {
                        status = 403;
                        json!({"error":"DESKTOP_ACCESS_DENIED"})
                    } else {
                        json!({"schemaVersion":1,"hostId":if mode==1 {"host:foreign"}else{"host:login"},"workspaceId":"workspace:login","accountId":"10000000-0000-4000-8000-000000000001","email":"owner@example.test","subscription":{"status":if mode==6 {"inactive"}else{"active"},"memoryAccess":mode!=6,"periodEndsAt":null,"checkedAt":patchfleet_core::now()}})
                    }
                } else if path.ends_with("/logout") {
                    assert_eq!(body, json!({"schemaVersion":1}));
                    if mode == 7 {
                        status = 503;
                        json!({"error":"DESKTOP_UNAVAILABLE"})
                    } else {
                        json!({"schemaVersion":1,"revoked":true})
                    }
                } else if path == "/api/v1/pairings/consume" {
                    json!({"schemaVersion":1,"hostId":"host:replacement","workspaceId":"workspace:login","credential":"REPLACEMENT_CREDENTIAL","protocolVersion":2})
                } else {
                    panic!("unexpected synthetic endpoint {path}")
                };
                let body = serde_json::to_vec(&response).unwrap();
                let response=format!("HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                if socket.write_all(response.as_bytes()).await.is_ok() {
                    let _ = socket.write_all(&body).await;
                }
            });
        }
    });
    Server { url, state, task }
}
fn command(mut value: Value) -> Value {
    value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    value
}
async fn begin(host: &Host, server: &Server) -> (Value, Value) {
    let payload = command(json!({"cloudUrl":server.url,"displayName":"Synthetic laptop"}));
    let waiting = host
        .action("cloud_login_begin", payload.clone())
        .await
        .unwrap();
    let url = reqwest::Url::parse(waiting["authorizationUrl"].as_str().unwrap()).unwrap();
    *server.state.challenge.lock().unwrap() = url
        .query_pairs()
        .find(|(k, _)| k == "codeChallenge")
        .unwrap()
        .1
        .into();
    (payload, waiting)
}
fn endpoint(waiting: &Value) -> (reqwest::Url, String) {
    let url = reqwest::Url::parse(waiting["authorizationUrl"].as_str().unwrap()).unwrap();
    let redirect = url
        .query_pairs()
        .find(|(k, _)| k == "redirectUri")
        .unwrap()
        .1
        .into_owned();
    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .into_owned();
    (reqwest::Url::parse(&redirect).unwrap(), state)
}
async fn callback(waiting: &Value, denied: bool, wrong: bool) -> String {
    let (url, mut state) = endpoint(waiting);
    if wrong {
        state = "b".repeat(64);
    }
    let mut socket = TcpStream::connect(("127.0.0.1", url.port().unwrap()))
        .await
        .unwrap();
    let query = if denied {
        "error=access_denied".into()
    } else {
        format!("code={}", "A".repeat(43))
    };
    let request = format!(
        "GET /auth/callback?{query}&state={state} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
        url.port().unwrap()
    );
    socket.write_all(request.as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    String::from_utf8(bytes).unwrap()
}
async fn connected(host: &Host) -> Value {
    for _ in 0..20 {
        let result = host.action("cloud_login_poll", json!({})).await.unwrap();
        if result["status"] != "waiting" {
            return result;
        }
        tokio::task::yield_now().await;
    }
    panic!("fixture callback not observed")
}
async fn fixture(server: &Server) -> (tempfile::TempDir, Arc<Host>) {
    let dir = tempfile::tempdir().unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let (_, waiting) = begin(&host, server).await;
    let html = callback(&waiting, false, false).await;
    assert!(html.starts_with("HTTP/1.1 200"));
    assert_eq!(connected(&host).await["status"], "connected");
    (dir, host)
}
fn files(path: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
async fn wait_seen(server: &Server) {
    tokio::time::timeout(Duration::from_secs(5), server.state.seen.notified())
        .await
        .unwrap();
}

#[tokio::test]
async fn real_callback_s256_exchange_restart_account_and_fresh_billing_leave_proofs_ephemeral() {
    let server = make_server().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let before = files(dir.path());
    let (payload, waiting) = begin(&host, &server).await;
    assert_eq!(
        host.action("cloud_login_begin", payload.clone())
            .await
            .unwrap(),
        waiting
    );
    assert_eq!(files(dir.path()), before);
    let (_, state) = endpoint(&waiting);
    let html = callback(&waiting, false, false).await;
    assert!(html.contains("Cache-Control: no-store"));
    assert!(html.contains("Referrer-Policy: no-referrer"));
    assert!(!html.contains(&state));
    assert!(!html.contains(&"A".repeat(43)));
    assert_eq!(
        connected(&host).await,
        json!({"schemaVersion":1,"status":"connected"})
    );
    let requests = server.state.requests.lock().unwrap().clone();
    let token = &requests
        .iter()
        .find(|r| r["path"] == "/api/desktop/v1/token")
        .unwrap()["body"];
    let snapshot = serde_json::to_string(&host.snapshot().unwrap()).unwrap();
    for secret in [
        &state,
        token["verifier"].as_str().unwrap(),
        token["code"].as_str().unwrap(),
        "CREDENTIAL_ONLY_CLOUD_FILE",
    ] {
        assert!(!snapshot.contains(secret));
        for (name, bytes) in files(dir.path()) {
            if name != "cloud.json" {
                assert!(!String::from_utf8_lossy(&bytes).contains(secret), "{name}");
            }
        }
    }
    let stored: Value = serde_json::from_slice(&files(dir.path())["cloud.json"]).unwrap();
    assert_eq!(
        stored["connection"]["credential"],
        "CREDENTIAL_ONLY_CLOUD_FILE"
    );
    assert!(!String::from_utf8_lossy(&files(dir.path())["cloud.json"])
        .contains(token["verifier"].as_str().unwrap()));
    host.shutdown().await;
    drop(host);
    let host = Host::open(dir.path().into()).unwrap();
    let before = files(dir.path());
    let account = host.action("cloud_account_read", json!({})).await.unwrap();
    assert_eq!(account["subscription"]["memoryAccess"], true);
    server.state.mode.store(6, Ordering::SeqCst);
    assert_eq!(
        host.action("cloud_account_read", json!({})).await.unwrap()["subscription"]["memoryAccess"],
        false
    );
    assert_eq!(files(dir.path()), before);
    assert!(!host
        .snapshot()
        .unwrap()
        .to_string()
        .contains("owner@example.test"));
    host.shutdown().await;
}
#[tokio::test]
async fn denial_invalid_callbacks_cancel_and_shutdown_release_the_listener_without_durable_state() {
    let server = make_server().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let before = files(dir.path());
    let (_, waiting) = begin(&host, &server).await;
    assert!(callback(&waiting, false, true)
        .await
        .starts_with("HTTP/1.1 400"));
    assert_eq!(
        host.action("cloud_login_poll", json!({})).await.unwrap()["status"],
        "waiting"
    );
    callback(&waiting, true, false).await;
    assert_eq!(connected(&host).await["status"], "denied");
    let (_, waiting) = begin(&host, &server).await;
    let port = endpoint(&waiting).0.port().unwrap();
    host.action("cloud_login_cancel", json!({})).await.unwrap();
    tokio::task::yield_now().await;
    assert!(TcpStream::connect(("127.0.0.1", port)).await.is_err());
    let (_, waiting) = begin(&host, &server).await;
    let port = endpoint(&waiting).0.port().unwrap();
    host.request_shutdown();
    tokio::task::yield_now().await;
    assert!(TcpStream::connect(("127.0.0.1", port)).await.is_err());
    assert_eq!(files(dir.path()), before);
    host.shutdown().await;
}
#[tokio::test]
async fn token_rejection_never_retries_consumed_code_and_unknown_protocol_never_pairs() {
    for mode in [2, 4] {
        let server = make_server().await;
        server.state.mode.store(mode, Ordering::SeqCst);
        let dir = tempfile::tempdir().unwrap();
        let host = Host::open(dir.path().into()).unwrap();
        let (_, waiting) = begin(&host, &server).await;
        callback(&waiting, false, false).await;
        let first = host
            .action("cloud_login_poll", json!({}))
            .await
            .unwrap_err();
        assert!(first.0.starts_with("DESKTOP_"));
        assert_eq!(
            host.action("cloud_login_poll", json!({}))
                .await
                .unwrap_err()
                .0,
            first.0
        );
        assert_eq!(server.state.requests.lock().unwrap().len(), 1);
        assert_eq!(host.snapshot().unwrap()["cloud"]["paired"], false);
        host.shutdown().await;
    }
}
#[tokio::test]
async fn slow_exchange_does_not_block_local_commands_and_cancel_prevents_late_install() {
    let server = make_server().await;
    server.state.held.store(true, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let (_, waiting) = begin(&host, &server).await;
    callback(&waiting, false, false).await;
    let h = host.clone();
    let poll = tokio::spawn(async move { h.action("cloud_login_poll", json!({})).await });
    wait_seen(&server).await;
    let created = tokio::time::timeout(
        Duration::from_millis(300),
        host.action(
            "project_create",
            command(json!({"name":"Free Local during sign-in"})),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(created["outcome"], "applied");
    host.action("cloud_login_cancel", json!({})).await.unwrap();
    server.state.release.notify_one();
    assert_eq!(
        poll.await.unwrap().unwrap_err().0,
        "DESKTOP_LOGIN_CANCELLED"
    );
    assert_eq!(host.snapshot().unwrap()["cloud"]["paired"], false);
    host.shutdown().await;
}
#[tokio::test]
async fn shutdown_interrupts_token_http_and_scope_changes_discard_account_reads() {
    let server = make_server().await;
    server.state.held.store(true, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let (_, waiting) = begin(&host, &server).await;
    callback(&waiting, false, false).await;
    let h = host.clone();
    let poll = tokio::spawn(async move { h.action("cloud_login_poll", json!({})).await });
    wait_seen(&server).await;
    host.request_shutdown();
    assert_eq!(
        tokio::time::timeout(Duration::from_millis(500), poll)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .0,
        "DESKTOP_CLOSING"
    );
    host.shutdown().await;
    server.state.release.notify_one();
    let server = make_server().await;
    let (_dir, host) = fixture(&server).await;
    server.state.seen.notified().await;
    server.state.held.store(true, Ordering::SeqCst);
    let h = host.clone();
    let read = tokio::spawn(async move { h.action("cloud_account_read", json!({})).await });
    wait_seen(&server).await;
    host.action("cloud_unpair", json!({})).await.unwrap();
    server.state.release.notify_one();
    assert_eq!(
        read.await.unwrap().unwrap_err().0,
        "DESKTOP_CONNECTION_CHANGED"
    );
    host.shutdown().await;
}
#[tokio::test]
async fn revoked_or_foreign_account_is_denied_and_offline_logout_always_keeps_local_data() {
    let server = make_server().await;
    let (dir, host) = fixture(&server).await;
    let local = host
        .action("project_create", command(json!({"name":"Retained Local"})))
        .await
        .unwrap();
    for (mode, expected) in [
        (1, "DESKTOP_PROTOCOL_INVALID"),
        (5, "DESKTOP_ACCESS_DENIED"),
    ] {
        server.state.mode.store(mode, Ordering::SeqCst);
        assert_eq!(
            host.action("cloud_account_read", json!({}))
                .await
                .unwrap_err()
                .0,
            expected
        );
    }
    server.state.mode.store(7, Ordering::SeqCst);
    let before = files(dir.path());
    let result = host.action("cloud_logout", json!({})).await.unwrap();
    assert_eq!(
        result,
        json!({"schemaVersion":1,"localSignedOut":true,"remoteRevoked":false})
    );
    for (name, bytes) in before {
        if name != "cloud.json" {
            assert_eq!(files(dir.path())[&name], bytes);
        }
    }
    assert_eq!(host.snapshot().unwrap()["cloud"]["paired"], false);
    assert_eq!(
        host.snapshot().unwrap()["projectMemory"]["projects"][0]["projectId"],
        local["projectId"]
    );
    host.shutdown().await;
}
#[tokio::test]
async fn logout_does_not_unpair_a_newer_manual_connection() {
    let server = make_server().await;
    let (_dir, host) = fixture(&server).await;
    server.state.seen.notified().await;
    server.state.held.store(true, Ordering::SeqCst);
    let h = host.clone();
    let logout = tokio::spawn(async move { h.action("cloud_logout", json!({})).await });
    wait_seen(&server).await;
    host.action("cloud_unpair", json!({})).await.unwrap();
    host.action("cloud_pair",json!({"cloudUrl":server.url,"displayName":"New explicit connection","pairingCode":"PF-SYNTHETIC"})).await.unwrap();
    server.state.release.notify_one();
    assert_eq!(
        logout.await.unwrap().unwrap_err().0,
        "DESKTOP_CONNECTION_CHANGED"
    );
    assert_eq!(
        host.snapshot().unwrap()["cloud"]["hostId"],
        "host:replacement"
    );
    host.shutdown().await;
}

#[tokio::test]
async fn callback_reads_and_rejected_connection_count_are_bounded() {
    let server = make_server().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let (_, waiting) = begin(&host, &server).await;
    let port = endpoint(&waiting).0.port().unwrap();
    let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    socket.write_all(b"GET /auth/callback").await.unwrap();
    let started = std::time::Instant::now();
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert!(started.elapsed() < Duration::from_millis(2700));
    assert!(String::from_utf8(bytes)
        .unwrap()
        .starts_with("HTTP/1.1 400"));
    for _ in 0..15 {
        assert!(callback(&waiting, false, true)
            .await
            .starts_with("HTTP/1.1 400"));
    }
    assert_eq!(
        host.action("cloud_login_poll", json!({}))
            .await
            .unwrap_err()
            .0,
        "DESKTOP_CALLBACK_REJECTED"
    );
    assert!(TcpStream::connect(("127.0.0.1", port)).await.is_err());
    host.shutdown().await;
}

#[tokio::test]
async fn token_exchange_timeout_is_terminal_and_never_redeems_twice() {
    let server = make_server().await;
    server.state.held.store(true, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let host = Host::open(dir.path().into()).unwrap();
    let (_, waiting) = begin(&host, &server).await;
    callback(&waiting, false, false).await;
    let started = std::time::Instant::now();
    assert_eq!(
        host.action("cloud_login_poll", json!({}))
            .await
            .unwrap_err()
            .0,
        "DESKTOP_UNAVAILABLE"
    );
    assert!(started.elapsed() < Duration::from_millis(3700));
    assert_eq!(
        host.action("cloud_login_poll", json!({}))
            .await
            .unwrap_err()
            .0,
        "DESKTOP_UNAVAILABLE"
    );
    assert_eq!(server.state.requests.lock().unwrap().len(), 1);
    assert_eq!(host.snapshot().unwrap()["cloud"]["paired"], false);
    host.shutdown().await;
    server.state.release.notify_one();
}
