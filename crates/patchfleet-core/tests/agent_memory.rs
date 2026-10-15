use patchfleet_core::{
    agent_memory::{self as memory, Binding, Server},
    Host,
};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    process::{Child, ChildStdin, ChildStdout, Command},
};

const PROJECT: &str = "project:10000000-0000-4000-8000-000000000001";
const OTHER: &str = "project:10000000-0000-4000-8000-000000000002";
const WORK: &str = "work:20000000-0000-4000-8000-000000000001";
const INTENT: &str = "cmd:30000000-0000-4000-8000-000000000001";
const EPOCH: &str = "40000000-0000-4000-8000-000000000001";
fn event(version: u64, kind: &str, payload: Value) -> Value {
    json!({"id":uuid::Uuid::new_v4().to_string(),"schemaVersion":version,"type":kind,"recordedAt":"2026-10-15T12:00:00.000Z","payload":payload})
}
fn context(revision: u64) -> Value {
    event(
        6,
        "agent_memory.context_bound",
        json!({"projectId":PROJECT,"workItemId":WORK,"runId":format!("run:{INTENT}"),"ownerEpoch":EPOCH,"decisionRevision":revision}),
    )
}
fn history() -> Vec<Value> {
    vec![
        event(
            3,
            "project.created",
            json!({"project":{"projectId":PROJECT}}),
        ),
        event(
            3,
            "project.subject.bound",
            json!({"projectId":PROJECT,"subjectKind":"work","subjectId":WORK}),
        ),
        event(
            1,
            "command.requested",
            json!({"intent":{"intentId":INTENT,"type":"start_work","payload":{"workItemId":WORK,"expectedItemRevision":1}}}),
        ),
        event(
            1,
            "run.launching",
            json!({"intentId":INTENT,"workItemId":WORK,"expectedItemRevision":1,"ownerEpoch":EPOCH}),
        ),
    ]
}
fn binding() -> Binding {
    Binding {
        project_id: PROJECT.into(),
        work_item_id: WORK.into(),
        run_id: format!("run:{INTENT}"),
        owner_epoch: EPOCH.into(),
        decision_revision: 0,
        active: Arc::new(AtomicBool::new(true)),
    }
}
#[test]
fn context_fact_is_exact_frozen_to_launch_and_does_not_change_old_projections() {
    assert_eq!(memory::project(&history()).unwrap(), Value::Null);
    let mut events = history();
    events.push(context(0));
    let value = memory::project(&events).unwrap();
    assert_eq!(value["contexts"].as_array().unwrap().len(), 1);
    assert_eq!(value["contexts"][0], events.last().unwrap()["payload"]);
    for (field, value) in [
        ("projectId", json!(OTHER)),
        ("workItemId", json!("work:foreign")),
        ("ownerEpoch", json!("foreign")),
        ("runId", json!("run:foreign")),
        ("decisionRevision", json!(1)),
        ("decisionRevision", json!(1.5)),
    ] {
        let mut events = history();
        let mut fact = context(0);
        fact["payload"][field] = value;
        events.push(fact);
        assert!(memory::project(&events).is_err(), "{field}");
    }
    for field in ["token", "query", "body"] {
        let mut fact = context(0);
        fact["payload"][field] = json!("FORBIDDEN");
        assert!(memory::validate_event(&fact).is_err());
    }
    let mut duplicate = events.clone();
    duplicate.push(context(0));
    assert!(memory::project(&duplicate).is_err());
    let mut late = history();
    let binding = late.remove(1);
    late.push(binding);
    late.push(context(0));
    assert!(memory::project(&late).is_err());
    let mut future = history();
    future.insert(0, context(0));
    assert!(memory::project(&future).is_err());
    let mut prepared = history();
    prepared.push(event(1, "run.prepared", json!({"intentId":INTENT})));
    prepared.push(context(0));
    assert!(memory::project(&prepared).is_err());
    let mut wrong_version = history();
    let mut fact = context(0);
    fact["schemaVersion"] = json!(5);
    wrong_version.push(fact);
    assert!(memory::project(&wrong_version).is_err());
}
#[test]
fn context_revision_counts_only_prior_project_v4_facts() {
    let mut events = history();
    events.insert(
        2,
        event(
            4,
            "decision.proposed",
            json!({"decision":{"projectId":PROJECT}}),
        ),
    );
    events.insert(
        3,
        event(4, "decision.accepted", json!({"projectId":PROJECT})),
    );
    events.insert(
        4,
        event(
            4,
            "decision.proposed",
            json!({"decision":{"projectId":OTHER}}),
        ),
    );
    events.insert(
        5,
        event(
            4,
            "decision.command.receipted",
            json!({"receipt":{"projectId":PROJECT}}),
        ),
    );
    events.push(context(2));
    memory::project(&events).unwrap();
    events.push(event(4, "decision.rejected", json!({"projectId":PROJECT})));
    assert_eq!(
        memory::project(&events).unwrap()["contexts"][0]["decisionRevision"],
        2
    );
    let last = events.len() - 2;
    events[last]["payload"]["decisionRevision"] = json!(3);
    assert!(memory::project(&events).is_err());
}
#[test]
fn terminal_start_receipt_closes_context_admission_without_erasing_a_prior_context() {
    let failed = event(
        1,
        "command.receipted",
        json!({"receipt":{"schemaVersion":1,"intentId":INTENT,"idempotencyKey":INTENT,
            "commandType":"start_work","outcome":"failed","reasonCode":"PROVIDER_CONTROL_FAILED",
            "completedAt":"2026-10-15T12:00:00.000Z","workProjectionRevision":0,"workItemId":WORK}}),
    );
    let mut ended = history();
    ended.push(failed.clone());
    ended.push(context(0));
    assert!(memory::project(&ended).is_err());

    let mut bound_before_failure = history();
    bound_before_failure.push(context(0));
    bound_before_failure.push(failed.clone());
    assert_eq!(
        memory::project(&bound_before_failure).unwrap()["contexts"][0],
        context(0)["payload"]
    );

    let mut other_request_ended = history();
    let mut other_receipt = failed;
    other_receipt["payload"]["receipt"]["intentId"] = json!("cmd:unrelated-start");
    other_request_ended.push(other_receipt);
    other_request_ended.push(context(0));
    assert_eq!(
        memory::project(&other_request_ended).unwrap()["contexts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn repeated_work_contexts_follow_canonical_order_even_when_the_newer_run_sorts_first() {
    let mut events = history();
    events.push(context(0));
    events.push(event(
        1,
        "command.receipted",
        json!({"receipt":{"schemaVersion":1,"intentId":INTENT,"idempotencyKey":INTENT,
            "commandType":"start_work","outcome":"failed","reasonCode":"PROVIDER_CONTROL_FAILED",
            "completedAt":"2026-10-15T12:00:00.000Z","workProjectionRevision":0,"workItemId":WORK}}),
    ));
    let next_intent = "cmd:10000000-0000-4000-8000-000000000001";
    events.push(event(
        1,
        "command.requested",
        json!({"intent":{"intentId":next_intent,"type":"start_work","payload":{"workItemId":WORK,"expectedItemRevision":1}}}),
    ));
    events.push(event(
        1,
        "run.launching",
        json!({"intentId":next_intent,"workItemId":WORK,"expectedItemRevision":1,"ownerEpoch":EPOCH}),
    ));
    let mut newer = context(0);
    newer["payload"]["runId"] = json!(format!("run:{next_intent}"));
    assert!(
        newer["payload"]["runId"].as_str().unwrap()
            < context(0)["payload"]["runId"].as_str().unwrap()
    );
    events.push(newer.clone());
    let projection = memory::project(&events).unwrap();
    assert_eq!(
        projection["contexts"],
        json!([context(0)["payload"], newer["payload"]])
    );
    assert_eq!(
        projection["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .find(|c| c["workItemId"] == WORK)
            .unwrap()["runId"],
        newer["payload"]["runId"]
    );
}
#[test]
fn tool_arguments_are_bounded_read_only_and_never_select_scope() {
    for (tool, args) in [
        ("project_context", json!({})),
        (
            "project_context",
            json!({"offset":20,"limit":20,"revision":2}),
        ),
        (
            "memory_search",
            json!({"query":"🦀".repeat(200),"limit":10}),
        ),
        (
            "decision_read",
            json!({"decisionId":"decision:10000000-0000-4000-8000-000000000001"}),
        ),
    ] {
        memory::validate_arguments(tool, &args).unwrap();
    }
    for (tool, args) in [
        ("project_context", json!({"projectId":OTHER})),
        ("project_context", json!({"offset":1})),
        ("project_context", json!({"limit":21})),
        ("project_context", json!({"revision":1.2})),
        ("memory_search", json!({"query":"🦀".repeat(201)})),
        ("memory_search", json!({"query":"bad\0"})),
        ("memory_search", json!({"query":"Rust","projectId":OTHER})),
        ("decision_read", json!({"decisionId":"decision:other"})),
        (
            "decision_read",
            json!({"decisionId":"decision:10000000-0000-4000-8000-000000000001","path":"/tmp/private"}),
        ),
        ("project_decision_accept", json!({})),
    ] {
        assert!(memory::validate_arguments(tool, &args).is_err(), "{tool}");
    }
}
fn ticket(path: &Path) -> Value {
    serde_json::from_slice(&patchfleet_core::store::read_private_file(path).unwrap()).unwrap()
}
async fn raw(endpoint: &str, secret: &str, body: &Value, extra: &str, method: &str) -> Value {
    let url = reqwest::Url::parse(endpoint).unwrap();
    let port = url.port().unwrap();
    let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let body = body.to_string();
    let head=format!("{method} /agent-memory/v1 HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {secret}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n{body}",body.len());
    socket.write_all(head.as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(4), socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    let start = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    assert!(String::from_utf8_lossy(&bytes[..start]).contains("Cache-Control: no-store"));
    serde_json::from_slice(&bytes[start..]).unwrap()
}
#[tokio::test]
async fn loopback_bounds_declared_sizes_and_closes_stalled_input() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(
        Weak::<Host>::new(),
        dir.path().into(),
        PathBuf::from(env!("CARGO_BIN_EXE_patchfleet")),
    )
    .await
    .unwrap();
    let grant = server.issue(binding()).unwrap();
    let data = ticket(&grant.path);
    let url = reqwest::Url::parse(data["endpoint"].as_str().unwrap()).unwrap();
    let port = url.port().unwrap();
    let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let head = format!("POST /agent-memory/v1 HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", usize::MAX);
    socket.write_all(head.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(1), socket.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&response).contains("AGENT_MEMORY_INVALID_REQUEST"));
    let mut stalled = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stalled.write_all(b"POST ").await.unwrap();
    let mut byte = [0u8; 1];
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(4), stalled.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    server.shutdown();
}
#[tokio::test]
async fn startup_cleans_only_recognized_private_ticket_and_interrupted_claim_files() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("agent-memory-tickets");
    patchfleet_core::store::private_directory(&folder).unwrap();
    let bootstrap = folder.join(format!("{}.json", uuid::Uuid::new_v4()));
    let claim = folder.join(format!(".{}.consumed", uuid::Uuid::new_v4()));
    let sentinel = folder.join("notes.json");
    for path in [&bootstrap, &claim, &sentinel] {
        patchfleet_core::store::atomic_write(path, b"PRIVATE_SENTINEL").unwrap();
    }
    let server = Server::start(
        Weak::<Host>::new(),
        dir.path().into(),
        PathBuf::from(env!("CARGO_BIN_EXE_patchfleet")),
    )
    .await
    .unwrap();
    assert!(!bootstrap.exists());
    assert!(!claim.exists());
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"PRIVATE_SENTINEL");
    server.shutdown();
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"PRIVATE_SENTINEL");
}
#[tokio::test]
async fn loopback_requires_framed_bearer_live_lease_and_read_only_arguments() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(
        Weak::<Host>::new(),
        dir.path().into(),
        PathBuf::from(env!("CARGO_BIN_EXE_patchfleet")),
    )
    .await
    .unwrap();
    let grant = server.issue(binding()).unwrap();
    let data = ticket(&grant.path);
    let endpoint = data["endpoint"].as_str().unwrap();
    let bearer = data["bearer"].as_str().unwrap();
    let request = json!({"schemaVersion":1,"tool":"project_context","arguments":{}});
    assert!(!grant.config.to_string().contains(bearer));
    assert_eq!(
        grant.config["mcp_servers"]["patchfleet_memory"]["required"],
        true
    );
    for (secret, extra, method, expected) in [
        ("a".repeat(64), "", "POST", "AGENT_MEMORY_ACCESS_DENIED"),
        (
            bearer.into(),
            "Origin: https://evil.example\r\n",
            "POST",
            "AGENT_MEMORY_INVALID_REQUEST",
        ),
        (
            bearer.into(),
            "Host: evil.example\r\n",
            "POST",
            "AGENT_MEMORY_INVALID_REQUEST",
        ),
        (
            bearer.into(),
            "Transfer-Encoding: chunked\r\n",
            "POST",
            "AGENT_MEMORY_INVALID_REQUEST",
        ),
        (bearer.into(), "", "GET", "AGENT_MEMORY_INVALID_REQUEST"),
    ] {
        assert_eq!(
            raw(endpoint, &secret, &request, extra, method).await["error"],
            expected
        );
    }
    assert_eq!(
        raw(endpoint, bearer, &request, "", "POST").await["error"],
        "AGENT_MEMORY_UNAVAILABLE"
    ); // Weak Host is deliberately absent.
    let forged =
        json!({"schemaVersion":1,"tool":"project_context","arguments":{"projectId":OTHER}});
    assert_eq!(
        raw(endpoint, bearer, &forged, "", "POST").await["error"],
        "AGENT_MEMORY_INVALID_REQUEST"
    );
    grant.active.store(false, Ordering::SeqCst);
    assert_eq!(
        raw(endpoint, bearer, &request, "", "POST").await["error"],
        "AGENT_MEMORY_ACCESS_DENIED"
    );
    let fresh = server.issue(binding()).unwrap();
    assert!(!grant.path.exists());
    server.shutdown();
    assert!(!fresh.active.load(Ordering::SeqCst));
    assert!(!fresh.path.exists());
}
struct Mcp {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Mcp {
    fn spawn(path: &Path, home: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_patchfleet"))
            .arg("--memory-mcp")
            .arg(path)
            .env_clear()
            .env("HOME", home)
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }
    async fn send(&mut self, value: Value) {
        self.input
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
        self.input.flush().await.unwrap();
    }
    async fn next(&mut self) -> Value {
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(5), self.output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert!(
            !line.is_empty(),
            "MCP child exited without a protocol reply"
        );
        assert!(line.len() <= memory::MCP_MAX_OUTPUT);
        serde_json::from_str(&line).unwrap()
    }
    async fn initialize(&mut self) {
        self.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"synthetic","version":"1.0"}}})).await;
        assert_eq!(self.next().await["result"]["protocolVersion"], "2025-06-18");
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
    }
    async fn finish(mut self) {
        drop(self.input);
        assert!(
            tokio::time::timeout(Duration::from_secs(3), self.child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
        let mut stderr = Vec::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_end(&mut stderr)
            .await
            .unwrap();
        assert!(stderr.is_empty(), "unexpected application logging");
    }
}
#[tokio::test]
async fn stdio_inventory_is_static_first_valid_call_consumes_and_repeated_calls_use_ram() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(
        Weak::<Host>::new(),
        dir.path().into(),
        PathBuf::from(env!("CARGO_BIN_EXE_patchfleet")),
    )
    .await
    .unwrap();
    let grant = server.issue(binding()).unwrap();
    let mut runtime = Mcp::spawn(&grant.path, dir.path());
    runtime
        .send(json!({"jsonrpc":"2.0","id":0,"method":"tools/list"}))
        .await;
    assert_eq!(runtime.next().await["error"]["code"], -32002);
    runtime.initialize().await;
    runtime
        .send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .await;
    let tools = runtime.next().await;
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 3);
    assert!(grant.path.exists());
    let mut probe = Mcp::spawn(&grant.path, dir.path());
    probe.initialize().await;
    probe
        .send(json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}))
        .await;
    assert_eq!(
        probe.next().await["result"]["tools"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    probe.finish().await;
    assert!(grant.path.exists());
    runtime.send(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"project_context","arguments":{"projectId":OTHER}}})).await;
    assert_eq!(runtime.next().await["error"]["code"], -32602);
    assert!(grant.path.exists());
    for id in [5, 6] {
        runtime.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"project_context","arguments":{}}})).await;
        let result = runtime.next().await;
        assert_eq!(result["result"]["isError"], true);
        assert_eq!(
            result["result"]["structuredContent"]["error"],
            "AGENT_MEMORY_UNAVAILABLE"
        );
        assert!(!grant.path.exists());
    }
    runtime.finish().await;
    server.shutdown();
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, vec!["agent-memory-tickets"]); // No writer or Host data was opened by either child.
}
#[tokio::test]
async fn expired_linked_or_outside_bootstrap_is_denied_without_reading_or_deleting_other_files() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("agent-memory-tickets");
    patchfleet_core::store::private_directory(&folder).unwrap();
    let folder = std::fs::canonicalize(folder).unwrap();
    let path = folder.join(format!("{}.json", uuid::Uuid::new_v4()));
    let old = chrono::Utc::now() - chrono::Duration::minutes(2);
    let value = json!({"schemaVersion":1,"endpoint":"http://127.0.0.1:12345/agent-memory/v1","bearer":"a".repeat(64),"createdAt":old.to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"expiresAt":(old+chrono::Duration::seconds(60)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true)});
    patchfleet_core::store::atomic_write(&path, &serde_json::to_vec(&value).unwrap()).unwrap();
    let mut child = Mcp::spawn(&path, dir.path());
    child.initialize().await;
    child.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_context","arguments":{}}})).await;
    assert_eq!(
        child.next().await["result"]["structuredContent"]["error"],
        "AGENT_MEMORY_INVALID_TICKET"
    );
    child.finish().await;
    assert!(!path.exists());
    let outside = dir.path().join(format!("{}.json", uuid::Uuid::new_v4()));
    std::fs::write(&outside, b"DO_NOT_DELETE").unwrap();
    let mut child = Mcp::spawn(&outside, dir.path());
    child.initialize().await;
    child.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_context","arguments":{}}})).await;
    assert_eq!(child.next().await["result"]["isError"], true);
    child.finish().await;
    assert_eq!(std::fs::read(&outside).unwrap(), b"DO_NOT_DELETE");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let link = folder.join(format!("{}.json", uuid::Uuid::new_v4()));
        symlink(&outside, &link).unwrap();
        let mut child = Mcp::spawn(&link, dir.path());
        child.initialize().await;
        child.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_context","arguments":{}}})).await;
        assert_eq!(child.next().await["result"]["isError"], true);
        child.finish().await;
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(&outside).unwrap(), b"DO_NOT_DELETE");
        let public = folder.join(format!("{}.json", uuid::Uuid::new_v4()));
        patchfleet_core::store::atomic_write(&public, &serde_json::to_vec(&value).unwrap())
            .unwrap();
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o644)).unwrap();
        let mut child = Mcp::spawn(&public, dir.path());
        child.initialize().await;
        child.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_context","arguments":{}}})).await;
        assert_eq!(
            child.next().await["result"]["structuredContent"]["error"],
            "AGENT_MEMORY_INVALID_TICKET"
        );
        child.finish().await;
        assert!(public.exists());
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o600)).unwrap();
        let hard = folder.join(format!("{}.json", uuid::Uuid::new_v4()));
        std::fs::hard_link(&public, &hard).unwrap();
        let mut child = Mcp::spawn(&hard, dir.path());
        child.initialize().await;
        child.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_context","arguments":{}}})).await;
        assert_eq!(
            child.next().await["result"]["structuredContent"]["error"],
            "AGENT_MEMORY_INVALID_TICKET"
        );
        child.finish().await;
        assert!(hard.exists() && public.exists());
    }
}
#[tokio::test]
async fn oversized_input_is_explicit_protocol_error_and_does_not_touch_a_ticket() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("not-a-ticket");
    let mut child = Mcp::spawn(&path, dir.path());
    child
        .input
        .write_all(&vec![b'x'; memory::MAX_INPUT + 1])
        .await
        .unwrap();
    assert_eq!(child.next().await["error"]["code"], -32700);
    child.finish().await;
}
#[tokio::test]
async fn final_mcp_size_bound_is_checked_after_structured_and_text_duplication() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("agent-memory-tickets");
    patchfleet_core::store::private_directory(&folder).unwrap();
    let folder = std::fs::canonicalize(folder).unwrap();
    let path = folder.join(format!("{}.json", uuid::Uuid::new_v4()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/agent-memory/v1", listener.local_addr().unwrap());
    let now = chrono::Utc::now();
    let ticket = json!({"schemaVersion":1,"endpoint":endpoint,"bearer":"a".repeat(64),"createdAt":now.to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"expiresAt":(now+chrono::Duration::seconds(60)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true)});
    patchfleet_core::store::atomic_write(&path, &serde_json::to_vec(&ticket).unwrap()).unwrap();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut block = [0u8; 4096];
        loop {
            let n = socket.read(&mut block).await.unwrap();
            if n == 0 {
                return;
            }
            bytes.extend_from_slice(&block[..n]);
            if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let body = json!({"decision":"x".repeat(100*1024)}).to_string();
        let head=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
        socket.write_all(head.as_bytes()).await.unwrap();
        socket.write_all(body.as_bytes()).await.unwrap();
    });
    let mut child = Mcp::spawn(&path, dir.path());
    child.initialize().await;
    child.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_context","arguments":{}}})).await;
    let response = child.next().await;
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(
        response["result"]["structuredContent"]["error"],
        "AGENT_MEMORY_RESPONSE_TOO_LARGE"
    );
    child
        .send(json!({"jsonrpc":"2.0","id":3,"method":"ping"}))
        .await;
    assert_eq!(child.next().await["result"], json!({}));
    child.finish().await;
    task.await.unwrap();
}
