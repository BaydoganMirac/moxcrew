use patchfleet_core::cloud::{self, Cloud};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
const AT: &str = "2026-10-14T00:00:00.000Z";
fn bundle() -> Value {
    json!({"sequence":1,"recordedAt":AT,"observation":{"observations":[{"provider":{"id":"codex","state":"available","version":"0.144.1","capabilities":{"recentObservation":true,"explicitLiveStatus":true},"token":"SECRET_TOKEN"},"observedAt":AT,"transcript":"SECRET_TRANSCRIPT"}]},"work":{"items":[{"workItemId":"work:one","providerId":"codex","status":"running","revision":2,"createdAt":AT,"title":"SECRET_TITLE","instruction":"SECRET_PROMPT","workingDirectory":"/secret/repo"}],"runs":[{"runId":"run:one","workItemId":"work:one","providerId":"codex","status":"running","revision":1,"startedAt":AT,"terminalAt":null,"providerSessionId":"SECRET_NATIVE_ID","ownerEpoch":"SECRET_EPOCH"}]},"teams":{"items":[]},"workspaces":{"items":[{"workspaceId":"workspace:one","workingDirectory":"/secret/repo","displayName":"SECRET_PROJECT"}]},"agentPacks":[{"id":"pack:test","role":"builder","instructions":"SECRET_PACK"}],"prompt":"SECRET_TOP"})
}
fn intent() -> Value {
    json!({"schemaVersion":1,"intentId":"intent:one","idempotencyKey":"idem:one","type":"cancel_run","actorId":"cloud-owner","createdAt":AT,"expiresAt":"2026-10-14T00:05:00.000Z","payload":{"runId":"run:one","expectedRunRevision":1}})
}
fn receipt() -> Value {
    json!({"schemaVersion":1,"intentId":"intent:one","idempotencyKey":"idem:one","commandType":"cancel_run","outcome":"expired","reasonCode":"COMMAND_EXPIRED","completedAt":AT,"workProjectionRevision":2,"workItemId":"work:one"})
}
#[test]
fn pure_allowlist_has_no_private_data() {
    for version in [1, 2] {
        let p = cloud::build_projection(&bundle(), version, 1, "install:test").unwrap();
        let s = p.to_string();
        assert!(!s.contains("SECRET"));
        assert!(!s.contains("/secret"));
        assert_eq!(p["providers"].as_array().unwrap().len(), 1);
        if version == 2 {
            assert_eq!(
                p["workspaces"][0]["workspaceAlias"],
                cloud::workspace_alias("install:test", "workspace:one")
            );
        }
    }
    let mut bad = bundle();
    bad["work"]["runs"][0]["status"] = json!("run_shell");
    assert!(cloud::build_projection(&bad, 1, 1, "install:test").is_err());
}
#[test]
fn strict_schemas_expiry_and_host_scope() {
    let i = intent();
    let page = json!({"schemaVersion":1,"hostId":"host:test","cursor":1,"intents":[{"sequence":1,"intent":i}]});
    assert_eq!(
        cloud::validate_intent_page(&page, 1, "host:test", 0, "2026-10-14T01:00:00.000Z")
            .unwrap()
            .len(),
        1
    ); // Expired but valid intents receive an owner-writer receipt.
    assert!(cloud::validate_intent_page(&page, 1, "host:other", 0, AT).is_err());
    assert!(cloud::validate_intent_page(&page, 1, "host:test", 1, AT).is_err());
    let mut bad = page.clone();
    bad["extra"] = json!("SECRET");
    assert!(cloud::validate_intent_page(&bad, 1, "host:test", 0, AT).is_err());
    let mut bad = page.clone();
    bad["intents"][0]["intent"]["payload"]["shell"] = json!("rm -rf /");
    assert!(cloud::validate_intent_page(&bad, 1, "host:test", 0, AT).is_err());
    let mut bad = page.clone();
    bad["intents"][0]["intent"]["expiresAt"] = json!("2026-10-14T00:05:00.001Z");
    assert!(cloud::validate_intent_page(&bad, 1, "host:test", 0, AT).is_err());
    let mut bad = page;
    bad["schemaVersion"] = json!(3);
    assert!(cloud::validate_intent_page(&bad, 3, "host:test", 0, AT).is_err());
    let mut r = receipt();
    r["providerOutput"] = json!("SECRET");
    assert!(cloud::build_receipt(&r).is_err());
    assert!(cloud::normalize_url("http://example.com").is_err());
    assert!(cloud::normalize_url("https://user:secret@example.com").is_err());
    assert!(cloud::normalize_url("https://example.com/path").is_err());
}
#[test]
fn v2_team_messages_reject_unknown_fields_and_non_rfc_uuid() {
    let mut i = json!({"schemaVersion":2,"intentId":"d41bd282-fca2-4e9f-beb2-d9fbcfdc1d36","idempotencyKey":"team:cancel","type":"cancel_team","actorId":"cloud-owner","createdAt":AT,"expiresAt":"2026-10-14T00:05:00.000Z","payload":{"teamId":"team:one","expectedTeamRevision":2}});
    cloud::validate_team_intent(&i).unwrap();
    i["intentId"] = json!("00000000-0000-0000-0000-000000000000");
    assert!(cloud::validate_team_intent(&i).is_err());
    i["intentId"] = json!("d41bd282-fca2-4e9f-beb2-d9fbcfdc1d36");
    i["payload"]["script"] = json!("SECRET");
    assert!(cloud::validate_team_intent(&i).is_err());
    let mut r = json!({"schemaVersion":2,"intentId":"d41bd282-fca2-4e9f-beb2-d9fbcfdc1d36","idempotencyKey":"team:cancel","commandType":"cancel_team","outcome":"applied","reasonCode":"TEAM_CANCELLED","completedAt":AT,"teamId":"team:one","teamRevision":3});
    cloud::validate_team_receipt(&r).unwrap();
    r["stdout"] = json!("SECRET");
    assert!(cloud::validate_team_receipt(&r).is_err());
    let empty = json!({"schemaVersion":3,"hostId":"host:test","cursor":0,"intents":[]});
    assert!(cloud::validate_intent_page(&empty, 3, "host:test", 0, AT).is_err());
}
struct Server {
    url: String,
    bodies: Arc<Mutex<Vec<(String, Value)>>>,
    fail_receipt: Arc<AtomicBool>,
    revoked: Arc<AtomicBool>,
    _task: tokio::task::JoinHandle<()>,
}
async fn server() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let bodies = Arc::new(Mutex::new(vec![]));
    let fail_receipt = Arc::new(AtomicBool::new(false));
    let revoked = Arc::new(AtomicBool::new(false));
    let b = bodies.clone();
    let f = fail_receipt.clone();
    let rev = revoked.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let b = b.clone();
            let f = f.clone();
            let rev = rev.clone();
            tokio::spawn(async move {
                let mut bytes = vec![];
                let mut block = [0u8; 1024];
                let (headers, body_start) = loop {
                    let n = stream.read(&mut block).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&block[..n]);
                    if let Some(p) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (String::from_utf8(bytes[..p].to_vec()).unwrap(), p + 4);
                    }
                };
                let length = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < body_start + length {
                    let n = stream.read(&mut block).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&block[..n]);
                }
                let path = headers
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned();
                let body: Value = if length == 0 {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes[body_start..body_start + length]).unwrap()
                };
                b.lock().unwrap().push((path.clone(), body.clone()));
                let (status, response) = if path.ends_with("pairings/consume") {
                    (
                        200,
                        json!({"schemaVersion":1,"hostId":"host:test","workspaceId":"workspace:cloud","credential":"FAKE_LOCAL_CREDENTIAL","protocolVersion":2}),
                    )
                } else if rev.load(Ordering::SeqCst) {
                    (401, json!({"error":"revoked"}))
                } else if path.contains("/intents?") {
                    let after = path.split("after=").nth(1).unwrap().parse::<u64>().unwrap();
                    (
                        200,
                        json!({"schemaVersion":2,"hostId":"host:test","cursor":1,"intents":if after==0{vec![json!({"sequence":1,"intent":intent()})]}else{vec![]}}),
                    )
                } else if path.ends_with("/receipts") && f.swap(false, Ordering::SeqCst) {
                    (503, json!({"error":"temporarily unavailable"}))
                } else {
                    (200, json!({"schemaVersion":1,"accepted":true}))
                };
                let data = response.to_string();
                let response=format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{data}",data.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });
    Server {
        url: format!("http://{addr}"),
        bodies,
        fail_receipt,
        revoked,
        _task: task,
    }
}
#[tokio::test]
async fn receipt_retry_cursor_and_credential_redaction() {
    let server = server().await;
    let dir = tempfile::tempdir().unwrap();
    let mut c = Cloud::open(dir.path()).unwrap();
    let installation = c.installation_id().to_owned();
    let public = c
        .pair(&json!({"cloudUrl":server.url,"pairingCode":"PF-TEST","displayName":"Test host"}))
        .await
        .unwrap();
    assert!(!public.to_string().contains("CREDENTIAL"));
    let batch = c.begin_sync(&bundle()).await.unwrap();
    assert_eq!(batch.intents.len(), 1);
    server.fail_receipt.store(true, Ordering::SeqCst);
    assert!(c.deliver_receipt(1, &receipt()).await.is_err());
    drop(c);
    let mut c = Cloud::open(dir.path()).unwrap();
    assert_eq!(c.installation_id(), installation);
    let batch = c.begin_sync(&bundle()).await.unwrap();
    assert_eq!(batch.intents.len(), 1);
    c.deliver_receipt(1, &receipt()).await.unwrap();
    c.finish_sync().unwrap();
    assert!(c.begin_sync(&bundle()).await.unwrap().intents.is_empty());
    c.finish_sync().unwrap();
    let requests = server.bodies.lock().unwrap();
    let projections: Vec<_> = requests
        .iter()
        .filter(|(p, _)| p.ends_with("/projection"))
        .map(|(_, v)| v)
        .collect();
    assert_eq!(projections[0], projections[1]);
    for p in projections {
        assert!(!p.to_string().contains("SECRET"));
    }
    let receipts: Vec<_> = requests
        .iter()
        .filter(|(p, _)| p.ends_with("/receipts"))
        .map(|(_, v)| v)
        .collect();
    assert_eq!(receipts.len(), 2);
    assert_eq!(receipts[0], receipts[1]);
    drop(requests);
    c.unpair().unwrap();
    assert_eq!(c.public_status(), json!({"paired":false}));
}
#[tokio::test]
async fn revocation_and_outage_do_not_destroy_local_pairing() {
    let server = server().await;
    let dir = tempfile::tempdir().unwrap();
    let mut c = Cloud::open(dir.path()).unwrap();
    c.pair(&json!({"cloudUrl":server.url,"pairingCode":"PF-TEST","displayName":"Test host"}))
        .await
        .unwrap();
    server.revoked.store(true, Ordering::SeqCst);
    assert_eq!(
        c.begin_sync(&bundle()).await.err().unwrap(),
        "CLOUD_AUTH_REJECTED"
    );
    assert_eq!(c.public_status()["lastErrorCode"], "CLOUD_AUTH_REJECTED");
    assert_eq!(c.public_status()["paired"], true);
    assert!(!c.public_status().to_string().contains("FAKE_LOCAL"));
}
