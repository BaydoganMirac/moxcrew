//! Run-scoped, read-only memory transport. The existing Host remains the writer.
use crate::{Error, Host, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Notify,
    task::{JoinHandle, JoinSet},
    time::Instant,
};

pub const MAX_INPUT: usize = 8 * 1024;
pub const MAX_OUTPUT: usize = 512 * 1024;
pub const MCP_MAX_OUTPUT: usize = 192 * 1024;
const MAX_SAFE: u64 = 9_007_199_254_740_991;
const TICKETS: &str = "agent-memory-tickets";
const TOOLS: [&str; 3] = ["project_context", "memory_search", "decision_read"];
fn invalid(code: &str) -> Error {
    Error(code.into())
}
fn require(ok: bool) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(invalid("AGENT_MEMORY_INVALID_REQUEST"))
    }
}
fn exact(value: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    require(value.as_object().is_some_and(|m| {
        required.iter().all(|k| m.contains_key(*k))
            && m.keys()
                .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str()))
    }))
}
fn number(value: &Value, min: u64, max: u64) -> Result<u64> {
    value
        .as_u64()
        .filter(|n| *n >= min && *n <= max)
        .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))
}
fn opaque(value: &Value) -> Result<&str> {
    value
        .as_str()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        })
        .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))
}
fn uuid(value: &str) -> Result<()> {
    let id = uuid::Uuid::parse_str(value).map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
    require(
        id.to_string() == value
            && id.get_variant() == uuid::Variant::RFC4122
            && (1..=8).contains(&id.get_version_num()),
    )
}
fn project_id(value: &Value) -> Result<()> {
    uuid(
        opaque(value)?
            .strip_prefix("project:")
            .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))?,
    )
}
fn timestamp(value: &Value) -> Result<DateTime<Utc>> {
    let s = value
        .as_str()
        .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
    let time = DateTime::parse_from_rfc3339(s)
        .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?
        .with_timezone(&Utc);
    require(time.to_rfc3339_opts(SecondsFormat::Millis, true) == s)?;
    Ok(time)
}
fn secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
fn secret_valid(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn safe_code(code: &str) -> &str {
    if code.starts_with("AGENT_MEMORY_")
        && code.len() <= 64
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    {
        code
    } else {
        "AGENT_MEMORY_UNAVAILABLE"
    }
}

#[derive(Clone)]
pub struct Binding {
    pub project_id: String,
    pub work_item_id: String,
    pub run_id: String,
    pub owner_epoch: String,
    pub decision_revision: u64,
    pub active: Arc<AtomicBool>,
}
pub struct Ticket {
    pub config: Value,
    pub active: Arc<AtomicBool>,
    pub path: PathBuf,
}
struct Lease {
    binding: Binding,
    expires: Instant,
    expires_at: DateTime<Utc>,
    ticket_expires: Instant,
    ticket_cleaned: bool,
    path: PathBuf,
}
struct State {
    closed: AtomicBool,
    leases: Mutex<HashMap<String, Lease>>,
    changed: Notify,
}
pub struct Server {
    state: Arc<State>,
    task: Mutex<Option<JoinHandle<()>>>,
    directory: PathBuf,
    endpoint: String,
    executable: PathBuf,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn own_file(path: &Path) -> bool {
    path.parent()
        .is_some_and(|p| p.file_name().is_some_and(|n| n == TICKETS))
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|s| s.strip_suffix(".json"))
            .is_some_and(|s| uuid(s).is_ok())
}
fn claimed_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        ".{}.consumed",
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("")
    ))
}
fn own_claim(path: &Path) -> bool {
    path.parent()
        .is_some_and(|p| p.file_name().is_some_and(|n| n == TICKETS))
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|s| s.strip_prefix('.'))
            .and_then(|s| s.strip_suffix(".consumed"))
            .is_some_and(|s| uuid(s).is_ok())
}
fn remove_private(path: &Path) {
    if crate::store::open_private_reader(path).is_ok() {
        let _ = fs::remove_file(path);
    }
}
fn remove_ticket(path: &Path) {
    if own_file(path) {
        remove_private(path);
        remove_private(&claimed_path(path));
    }
}
impl State {
    fn maintain(&self) -> Instant {
        let now = Instant::now();
        let mut next = now + Duration::from_secs(86400);
        if let Ok(mut leases) = self.leases.lock() {
            leases.retain(|_, lease| {
                if self.closed.load(Ordering::SeqCst)
                    || !lease.binding.active.load(Ordering::SeqCst)
                    || now >= lease.expires
                    || Utc::now() >= lease.expires_at
                {
                    lease.binding.active.store(false, Ordering::SeqCst);
                    remove_ticket(&lease.path);
                    return false;
                }
                if now >= lease.ticket_expires && !lease.ticket_cleaned {
                    remove_ticket(&lease.path);
                    lease.ticket_cleaned = true;
                } else if !lease.ticket_cleaned {
                    next = next.min(lease.ticket_expires);
                }
                next = next.min(lease.expires);
                true
            });
        }
        next
    }
}
impl Server {
    pub async fn start(host: Weak<Host>, data_dir: PathBuf, executable: PathBuf) -> Result<Self> {
        require(data_dir.is_absolute() && executable.is_absolute())?;
        let directory = fs::canonicalize(data_dir)
            .map_err(|_| invalid("AGENT_MEMORY_STORAGE_FAILED"))?
            .join(TICKETS);
        crate::store::private_directory(&directory)
            .map_err(|_| invalid("AGENT_MEMORY_STORAGE_FAILED"))?;
        for item in fs::read_dir(&directory).map_err(|_| invalid("AGENT_MEMORY_STORAGE_FAILED"))? {
            let item = item.map_err(|_| invalid("AGENT_MEMORY_STORAGE_FAILED"))?;
            if own_file(&item.path()) {
                remove_ticket(&item.path());
            } else if own_claim(&item.path()) {
                remove_private(&item.path());
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
        let port = listener
            .local_addr()
            .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?
            .port();
        let endpoint = format!("http://127.0.0.1:{port}/agent-memory/v1");
        let state = Arc::new(State {
            closed: AtomicBool::new(false),
            leases: Mutex::new(HashMap::new()),
            changed: Notify::new(),
        });
        let observed = state.clone();
        let task = tokio::spawn(async move {
            let mut requests = JoinSet::new();
            loop {
                let deadline = observed.maintain();
                if observed.closed.load(Ordering::SeqCst) {
                    break;
                }
                tokio::select! {
                    _=observed.changed.notified()=>{},
                    _=tokio::time::sleep_until(deadline)=>{},
                    _=requests.join_next(),if !requests.is_empty()=>{},
                    accepted=listener.accept()=>{
                        let Ok((socket,_))=accepted else{break;};
                        if requests.len()>=16 {drop(socket);continue;}
                        let state=observed.clone();let host=host.clone();
                        requests.spawn(async move {let _=tokio::time::timeout(Duration::from_secs(3),rpc(socket,port,state,host)).await;});
                    }
                }
            }
        });
        Ok(Self {
            state,
            task: Mutex::new(Some(task)),
            directory,
            endpoint,
            executable,
        })
    }
    pub fn issue(&self, binding: Binding) -> Result<Ticket> {
        require(
            binding.active.load(Ordering::SeqCst) && !self.state.closed.load(Ordering::SeqCst),
        )?;
        validate_binding(&binding)?;
        self.state.maintain();
        let mut leases = self
            .state
            .leases
            .lock()
            .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
        require(
            !self.state.closed.load(Ordering::SeqCst) && binding.active.load(Ordering::SeqCst),
        )?;
        if leases.len() >= 64 {
            return Err(invalid("AGENT_MEMORY_CAPACITY"));
        }
        let bearer = secret();
        let path = self
            .directory
            .join(format!("{}.json", uuid::Uuid::new_v4()));
        let created = Utc::now();
        let expires = created + chrono::Duration::seconds(60);
        let value = json!({"schemaVersion":1,"endpoint":self.endpoint,"bearer":bearer,"createdAt":created.to_rfc3339_opts(SecondsFormat::Millis,true),"expiresAt":expires.to_rfc3339_opts(SecondsFormat::Millis,true)});
        crate::store::atomic_write(&path, &serde_json::to_vec(&value)?)
            .map_err(|_| invalid("AGENT_MEMORY_STORAGE_FAILED"))?;
        let now = Instant::now();
        let active = binding.active.clone();
        leases.insert(
            digest(&bearer),
            Lease {
                binding,
                expires: now + Duration::from_secs(86400),
                expires_at: created + chrono::Duration::hours(24),
                ticket_expires: now + Duration::from_secs(60),
                ticket_cleaned: false,
                path: path.clone(),
            },
        );
        self.state.changed.notify_one();
        Ok(Ticket {
            config: json!({"mcp_servers":{"patchfleet_memory":{"command":self.executable,"args":["--memory-mcp",path],"required":true,"enabled_tools":TOOLS,"startup_timeout_sec":5,"tool_timeout_sec":5}}}),
            active,
            path,
        })
    }
    pub fn shutdown(&self) {
        self.state.closed.store(true, Ordering::SeqCst);
        if let Ok(mut leases) = self.state.leases.lock() {
            for lease in leases.values() {
                lease.binding.active.store(false, Ordering::SeqCst);
                remove_ticket(&lease.path);
            }
            leases.clear();
        }
        self.state.changed.notify_waiters();
        if let Ok(mut task) = self.task.lock() {
            if let Some(task) = task.take() {
                task.abort();
            }
        }
    }
}
fn validate_binding(binding: &Binding) -> Result<()> {
    project_id(&json!(binding.project_id))?;
    opaque(&json!(binding.work_item_id))?;
    opaque(&json!(binding.run_id))?;
    opaque(&json!(binding.owner_epoch))?;
    number(&json!(binding.decision_revision), 0, MAX_SAFE)?;
    Ok(())
}
pub fn validate_arguments(tool: &str, args: &Value) -> Result<()> {
    match tool {
        "project_context" => {
            exact(args, &[], &["offset", "limit", "revision"])?;
            for key in ["offset", "revision"] {
                if let Some(value) = args.get(key) {
                    number(value, 0, MAX_SAFE)?;
                }
            }
            if let Some(value) = args.get("limit") {
                number(value, 1, 20)?;
            }
            require(args["offset"].as_u64().unwrap_or(0) == 0 || args.get("revision").is_some())?;
        }
        "memory_search" => {
            exact(args, &["query"], &["limit"])?;
            let mut request = args.clone();
            request["projectId"] = json!("project:10000000-0000-4000-8000-000000000001");
            crate::retrieval::request(&request)
                .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
        }
        "decision_read" => {
            exact(args, &["decisionId"], &[])?;
            uuid(
                opaque(&args["decisionId"])?
                    .strip_prefix("decision:")
                    .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))?,
            )?;
        }
        _ => return Err(invalid("AGENT_MEMORY_UNKNOWN_TOOL")),
    }
    Ok(())
}
async fn rpc(mut socket: TcpStream, port: u16, state: Arc<State>, host: Weak<Host>) -> Result<()> {
    let outcome = async {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 2048];
        let (start, head) = loop {
            let n = socket
                .read(&mut chunk)
                .await
                .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
            require(n != 0)?;
            bytes.extend_from_slice(&chunk[..n]);
            require(bytes.len() <= MAX_INPUT)?;
            if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                break (
                    i + 4,
                    std::str::from_utf8(&bytes[..i])
                        .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?
                        .to_string(),
                );
            }
        };
        let mut lines = head.split("\r\n");
        require(lines.next() == Some("POST /agent-memory/v1 HTTP/1.1"))?;
        let mut headers = BTreeMap::new();
        for line in lines {
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
            require(
                !key.is_empty()
                    && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    && !value.chars().any(char::is_control),
            )?;
            require(
                headers
                    .insert(key.to_ascii_lowercase(), value.trim().to_string())
                    .is_none(),
            )?;
        }
        require(
            !headers.contains_key("origin")
                && !headers.contains_key("transfer-encoding")
                && headers.get("host") == Some(&format!("127.0.0.1:{port}"))
                && headers
                    .get("content-type")
                    .is_some_and(|s| s.eq_ignore_ascii_case("application/json")),
        )?;
        let length = headers
            .get("content-length")
            .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
        let size = length
            .parse::<usize>()
            .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
        require(
            length == &size.to_string() && size > 0 && size <= MAX_INPUT.saturating_sub(start),
        )?;
        while bytes.len() < start + size {
            let n = socket
                .read(&mut chunk)
                .await
                .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
            require(n != 0)?;
            bytes.extend_from_slice(&chunk[..n]);
            require(bytes.len() <= start + size)?;
        }
        require(bytes.len() == start + size)?;
        let bearer = headers
            .get("authorization")
            .and_then(|s| s.strip_prefix("Bearer "))
            .filter(|s| secret_valid(s))
            .ok_or_else(|| invalid("AGENT_MEMORY_ACCESS_DENIED"))?;
        let binding = {
            let leases = state
                .leases
                .lock()
                .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
            let lease = leases
                .get(&digest(bearer))
                .filter(|l| {
                    Instant::now() < l.expires
                        && Utc::now() < l.expires_at
                        && l.binding.active.load(Ordering::SeqCst)
                        && !state.closed.load(Ordering::SeqCst)
                })
                .ok_or_else(|| invalid("AGENT_MEMORY_ACCESS_DENIED"))?;
            lease.binding.clone()
        };
        let request: Value = serde_json::from_slice(&bytes[start..])
            .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
        exact(&request, &["schemaVersion", "tool", "arguments"], &[])?;
        require(request["schemaVersion"].as_u64() == Some(1))?;
        let tool = request["tool"]
            .as_str()
            .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
        validate_arguments(tool, &request["arguments"])?;
        let host = host
            .upgrade()
            .ok_or_else(|| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
        let value = host
            .agent_memory_call(&binding, tool, request["arguments"].clone())
            .await?;
        if !binding.active.load(Ordering::SeqCst) || state.closed.load(Ordering::SeqCst) {
            return Err(invalid("AGENT_MEMORY_ACCESS_DENIED"));
        }
        Ok(value)
    }
    .await;
    let (status, value) = match outcome {
        Ok(value) => (200, value),
        Err(error) => (403, json!({"error":safe_code(&error.0)})),
    };
    let mut body = serde_json::to_vec(&value).map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
    let status = if body.len() > MAX_OUTPUT {
        body = serde_json::to_vec(&json!({"error":"AGENT_MEMORY_RESPONSE_TOO_LARGE"}))?;
        413
    } else {
        status
    };
    let header=format!("HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
    socket
        .write_all(header.as_bytes())
        .await
        .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
    socket
        .write_all(&body)
        .await
        .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
    Ok(())
}

fn consume_ticket(path: &Path) -> Result<Value> {
    consume_ticket_inner(path).map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))
}
fn ticket_reader(path: &Path) -> Result<fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata =
            fs::symlink_metadata(path).map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
        // A bootstrap must already be owner-only before any credential is read.
        // geteuid only reads the process identity and has no pointer arguments.
        require(metadata.mode() & 0o077 == 0 && metadata.uid() == unsafe { libc::geteuid() })?;
    }
    crate::store::open_private_reader(path).map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))
}
fn consume_ticket_inner(path: &Path) -> Result<Value> {
    require(path.is_absolute() && own_file(path))?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
    require(
        fs::canonicalize(parent).map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))? == parent,
    )?;
    drop(ticket_reader(path)?);
    let claimed = claimed_path(path);
    fs::rename(path, &claimed).map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
    let result: Result<Value> = (|| {
        let file = ticket_reader(&claimed)?;
        let mut bytes = Vec::new();
        file.take((MAX_INPUT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
        require(bytes.len() <= MAX_INPUT)?;
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
        exact(
            &value,
            &[
                "schemaVersion",
                "endpoint",
                "bearer",
                "createdAt",
                "expiresAt",
            ],
            &[],
        )?;
        require(
            value["schemaVersion"].as_u64() == Some(1)
                && value["bearer"].as_str().is_some_and(secret_valid),
        )?;
        let created = timestamp(&value["createdAt"])?;
        let expires = timestamp(&value["expiresAt"])?;
        let now = Utc::now();
        require(
            expires > now
                && expires > created
                && expires.signed_duration_since(created).num_milliseconds() <= 60000
                && created <= now + chrono::Duration::seconds(1),
        )?;
        let endpoint = value["endpoint"]
            .as_str()
            .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
        let port = endpoint
            .strip_prefix("http://127.0.0.1:")
            .and_then(|s| s.strip_suffix("/agent-memory/v1"))
            .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
        let number = port
            .parse::<u16>()
            .map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))?;
        require(number > 0 && number.to_string() == port)?;
        Ok(value)
    })();
    let _ = fs::remove_file(&claimed);
    result.map_err(|_| invalid("AGENT_MEMORY_INVALID_TICKET"))
}
fn tools() -> Value {
    json!({"tools":[
        {"name":"project_context","description":"Read the complete paginated current accepted decision manifest for this run's frozen project. Refresh after a revision change.","inputSchema":{"type":"object","properties":{"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":20},"revision":{"type":"integer","minimum":0}},"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}},
        {"name":"memory_search","description":"Keyword search of current owner-accepted decisions in this run's project. Complete project_context first; scores are lexical, not semantic.","inputSchema":{"type":"object","properties":{"query":{"type":"string","minLength":1,"maxLength":400},"limit":{"type":"integer","minimum":1,"maximum":10}},"required":["query"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}},
        {"name":"decision_read","description":"Read an exact current accepted decision in this run's project. Complete project_context first. Source references contain IDs and revisions only.","inputSchema":{"type":"object","properties":{"decisionId":{"type":"string"}},"required":["decisionId"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}}
    ]})
}
async fn line<R: AsyncBufRead + Unpin>(input: &mut R) -> Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    loop {
        let buffer = input
            .fill_buf()
            .await
            .map_err(|_| invalid("AGENT_MEMORY_INVALID_REQUEST"))?;
        if buffer.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err(invalid("AGENT_MEMORY_INVALID_REQUEST"))
            };
        }
        let end = buffer.iter().position(|b| *b == b'\n').map(|i| i + 1);
        let size = end.unwrap_or(buffer.len());
        require(bytes.len() + size <= MAX_INPUT)?;
        bytes.extend_from_slice(&buffer[..size]);
        input.consume(size);
        if end.is_some() {
            bytes.pop();
            return Ok(Some(bytes));
        }
    }
}
fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn tool_result(id: Value, value: Value, is_error: bool) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":is_error}})
}
async fn remote(
    client: &reqwest::Client,
    ticket: &Value,
    tool: &str,
    args: Value,
) -> Result<Value> {
    let result = tokio::time::timeout(Duration::from_secs(3), async {
        let mut response = client
            .post(
                ticket["endpoint"]
                    .as_str()
                    .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_TICKET"))?,
            )
            .bearer_auth(
                ticket["bearer"]
                    .as_str()
                    .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_TICKET"))?,
            )
            .header("content-type", "application/json")
            .header("cache-control", "no-store")
            .json(&json!({"schemaVersion":1,"tool":tool,"arguments":args}))
            .send()
            .await
            .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
        require(
            response
                .headers()
                .get("content-type")
                .and_then(|s| s.to_str().ok())
                == Some("application/json")
                && response
                    .headers()
                    .get("cache-control")
                    .and_then(|s| s.to_str().ok())
                    .is_some_and(|s| {
                        s.split(',')
                            .any(|x| x.trim().eq_ignore_ascii_case("no-store"))
                    }),
        )?;
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?
        {
            if bytes.len() + chunk.len() > MAX_OUTPUT {
                return Err(invalid("AGENT_MEMORY_RESPONSE_TOO_LARGE"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
        if !status.is_success() {
            exact(&value, &["error"], &[])?;
            return Err(invalid(safe_code(value["error"].as_str().unwrap_or(""))));
        }
        Ok(value)
    })
    .await;
    result.unwrap_or_else(|_| Err(invalid("AGENT_MEMORY_TIMEOUT")))
}
pub async fn serve_stdio(ticket: &Path) -> Result<()> {
    // Codex starts a separate inventory probe. Only its runtime tool caller
    // consumes the one-shot bootstrap, then keeps the checked credential in RAM.
    let mut bootstrap: Option<std::result::Result<Value, String>> = None;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(3))
        .build()
        .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
    let mut input = BufReader::new(tokio::io::stdin());
    let mut output = tokio::io::stdout();
    let mut negotiated = false;
    let mut initialized = false;
    loop {
        let bytes = match line(&mut input).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(()),
            Err(_) => {
                let error = rpc_error(Value::Null, -32700, "Invalid bounded JSON-RPC input");
                output
                    .write_all(format!("{error}\n").as_bytes())
                    .await
                    .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
                return Ok(());
            }
        };
        let request: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                let error = rpc_error(Value::Null, -32700, "Invalid JSON");
                output
                    .write_all(format!("{error}\n").as_bytes())
                    .await
                    .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
                continue;
            }
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request["method"].as_str().unwrap_or("");
        let valid = exact(&request, &["jsonrpc", "method"], &["id", "params"]).is_ok()
            && request["jsonrpc"] == "2.0"
            && request["method"]
                .as_str()
                .is_some_and(|s| !s.is_empty() && s.len() <= 128)
            && (request.get("id").is_none()
                || id.as_str().is_some_and(|s| s.len() <= 256)
                || id.as_i64().is_some());
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let response = if !valid {
            Some(rpc_error(Value::Null, -32600, "Invalid request"))
        } else if method == "notifications/initialized" && request.get("id").is_none() {
            if negotiated && exact(&params, &[], &["_meta"]).is_ok() {
                initialized = true;
            }
            None
        } else if request.get("id").is_none() {
            None
        } else if method == "initialize" {
            if negotiated
                || exact(
                    &params,
                    &["protocolVersion", "capabilities", "clientInfo"],
                    &["_meta"],
                )
                .is_err()
                || !params["capabilities"].is_object()
                || exact(
                    &params["clientInfo"],
                    &["name", "version"],
                    &["title", "description", "icons", "websiteUrl"],
                )
                .is_err()
                || !params["clientInfo"]["name"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty() && s.len() <= 256)
                || !params["clientInfo"]["version"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty() && s.len() <= 256)
                || !params["protocolVersion"]
                    .as_str()
                    .is_some_and(|s| s.len() <= 32)
            {
                Some(rpc_error(id, -32602, "Invalid initialization"))
            } else {
                negotiated = true;
                let requested = params["protocolVersion"].as_str().unwrap();
                let version = if ["2025-11-25", "2025-06-18", "2024-11-05"].contains(&requested) {
                    requested
                } else {
                    "2025-11-25"
                };
                Some(
                    json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"patchfleet-memory","version":env!("CARGO_PKG_VERSION")}}}),
                )
            }
        } else if method == "ping" && exact(&params, &[], &["_meta"]).is_ok() {
            Some(json!({"jsonrpc":"2.0","id":id,"result":{}}))
        } else if !initialized {
            Some(rpc_error(id, -32002, "Initialization required"))
        } else if method == "tools/list" && exact(&params, &[], &["_meta"]).is_ok() {
            Some(json!({"jsonrpc":"2.0","id":id,"result":tools()}))
        } else if method == "tools/call" {
            if exact(&params, &["name", "arguments"], &["_meta"]).is_err()
                || validate_arguments(params["name"].as_str().unwrap_or(""), &params["arguments"])
                    .is_err()
            {
                Some(rpc_error(id, -32602, "Invalid tool arguments"))
            } else {
                if bootstrap.is_none() {
                    bootstrap = Some(consume_ticket(ticket).map_err(|e| e.0));
                }
                let result = match bootstrap.as_ref().unwrap() {
                    Ok(ticket) => {
                        remote(
                            &client,
                            ticket,
                            params["name"].as_str().unwrap(),
                            params["arguments"].clone(),
                        )
                        .await
                    }
                    Err(code) => Err(invalid(code)),
                };
                let (value, is_error) = match result {
                    Ok(value) => (value, false),
                    Err(error) => (json!({"error":safe_code(&error.0)}), true),
                };
                Some(tool_result(id, value, is_error))
            }
        } else {
            Some(rpc_error(id, -32601, "Method not found"))
        };
        if let Some(mut response) = response {
            let mut bytes =
                serde_json::to_vec(&response).map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
            if bytes.len() + 1 > MCP_MAX_OUTPUT {
                response = tool_result(
                    response["id"].clone(),
                    json!({"error":"AGENT_MEMORY_RESPONSE_TOO_LARGE"}),
                    true,
                );
                bytes = serde_json::to_vec(&response)?;
            }
            bytes.push(b'\n');
            output
                .write_all(&bytes)
                .await
                .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
            output
                .flush()
                .await
                .map_err(|_| invalid("AGENT_MEMORY_UNAVAILABLE"))?;
        }
    }
}

pub fn validate_event(event: &Value) -> Result<()> {
    exact(
        event,
        &["id", "schemaVersion", "type", "recordedAt", "payload"],
        &[],
    )?;
    uuid(
        event["id"]
            .as_str()
            .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_CONTEXT"))?,
    )?;
    timestamp(&event["recordedAt"])?;
    require(
        event["schemaVersion"].as_u64() == Some(6) && event["type"] == "agent_memory.context_bound",
    )?;
    let p = &event["payload"];
    exact(
        p,
        &[
            "runId",
            "workItemId",
            "ownerEpoch",
            "projectId",
            "decisionRevision",
        ],
        &[],
    )?;
    for key in ["runId", "workItemId", "ownerEpoch"] {
        opaque(&p[key])?;
    }
    project_id(&p["projectId"])?;
    number(&p["decisionRevision"], 0, MAX_SAFE)?;
    Ok(())
}
pub fn project(events: &[Value]) -> Result<Value> {
    if !events
        .iter()
        .any(|e| e["schemaVersion"] == 6 || e["type"] == "agent_memory.context_bound")
    {
        return Ok(Value::Null);
    }
    let mut intents = BTreeMap::new();
    let mut launches = BTreeMap::new();
    let mut launch_scopes = BTreeMap::new();
    let mut closed = HashSet::new();
    let mut projects = HashSet::new();
    let mut bindings = BTreeMap::new();
    let mut revisions = BTreeMap::<String, u64>::new();
    let mut contexts = Vec::new();
    let mut context_runs = HashSet::new();
    for event in events {
        let p = &event["payload"];
        match event["type"].as_str().unwrap_or("") {
            "project.created" if event["schemaVersion"] == 3 => {
                projects.insert(p["project"]["projectId"].as_str().unwrap_or("").to_string());
            }
            "project.subject.bound"
                if event["schemaVersion"] == 3 && p["subjectKind"] == "work" =>
            {
                let work = p["subjectId"].as_str().unwrap_or("").to_string();
                if let Some(prior) = bindings.insert(work, p["projectId"].clone()) {
                    require(prior == p["projectId"])?;
                }
            }
            "command.requested" if event["schemaVersion"] == 1 => {
                intents.insert(
                    p["intent"]["intentId"].as_str().unwrap_or("").to_string(),
                    p["intent"].clone(),
                );
            }
            "run.launching" if event["schemaVersion"] == 1 => {
                let intent = p["intentId"].as_str().unwrap_or("");
                let request = intents
                    .get(intent)
                    .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_CONTEXT"))?;
                require(
                    request["type"] == "start_work"
                        && request["payload"]["workItemId"] == p["workItemId"]
                        && request["payload"]["expectedItemRevision"] == p["expectedItemRevision"],
                )?;
                let run = format!("run:{intent}");
                launch_scopes.insert(
                    run.clone(),
                    bindings
                        .get(p["workItemId"].as_str().unwrap_or(""))
                        .cloned(),
                );
                launches.insert(run, p.clone());
            }
            "run.prepared" | "turn.requested" | "run.started" | "run.start_unknown"
                if event["schemaVersion"] == 1 =>
            {
                closed.insert(format!("run:{}", p["intentId"].as_str().unwrap_or("")));
            }
            "command.receipted"
                if event["schemaVersion"] == 1 && p["receipt"]["commandType"] == "start_work" =>
            {
                closed.insert(format!(
                    "run:{}",
                    p["receipt"]["intentId"].as_str().unwrap_or("")
                ));
            }
            "decision.proposed"
            | "decision.accepted"
            | "decision.rejected"
            | "decision.superseded"
                if event["schemaVersion"] == 4 =>
            {
                let project = if event["type"] == "decision.proposed" {
                    &p["decision"]["projectId"]
                } else {
                    &p["projectId"]
                };
                let revision = revisions
                    .entry(project.as_str().unwrap_or("").to_string())
                    .or_default();
                *revision = revision
                    .checked_add(1)
                    .filter(|v| *v <= MAX_SAFE)
                    .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_CONTEXT"))?;
            }
            _ => {}
        }
        if event["schemaVersion"] == 6 || event["type"] == "agent_memory.context_bound" {
            validate_event(event)?;
            let run = p["runId"].as_str().unwrap();
            let launch = launches
                .get(run)
                .ok_or_else(|| invalid("AGENT_MEMORY_INVALID_CONTEXT"))?;
            let work = p["workItemId"].as_str().unwrap();
            let scope = p["projectId"].as_str().unwrap();
            require(
                !closed.contains(run)
                    && launch["workItemId"] == p["workItemId"]
                    && launch["ownerEpoch"] == p["ownerEpoch"]
                    && bindings.get(work) == Some(&p["projectId"])
                    && launch_scopes.get(run).and_then(Option::as_ref) == Some(&p["projectId"])
                    && projects.contains(scope)
                    && revisions.get(scope).copied().unwrap_or(0)
                        == p["decisionRevision"].as_u64().unwrap()
                    && !context_runs.contains(run),
            )?;
            context_runs.insert(run.to_string());
            contexts.push(p.clone());
        }
    }
    Ok(json!({"schemaVersion":6,"contexts":contexts}))
}
