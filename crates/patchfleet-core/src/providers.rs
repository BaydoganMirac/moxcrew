//! Provider-supported surfaces only. Native text and credentials never leave this module.
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex as StdMutex, MutexGuard,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::{mpsc, Mutex},
    task::JoinHandle,
    time::timeout,
};

type Result<T> = std::result::Result<T, String>;
const LIMIT: usize = 256 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const EXTRACTION_TIMEOUT: Duration = Duration::from_secs(60);
const EXTRACTION_OUTPUT: usize = 1024 * 1024;
const EXTRACTION_JSON: usize = 128 * 1024;
const EXTRACTION_ARGS: &[&str] = &[
    "exec",
    "--json",
    "--strict-config",
    "--output-schema",
    "schema.json",
    "--sandbox",
    "read-only",
    "--ephemeral",
    "--skip-git-repo-check",
    "--ignore-user-config",
    "--ignore-rules",
    "--cd",
    ".",
    "--color",
    "never",
    "-c",
    "approval_policy=never",
    "-c",
    "web_search=disabled",
    "-c",
    "mcp_servers={}",
    "-c",
    "project_doc_max_bytes=0",
    "--enable",
    "skip_host_skill_discovery",
    "--disable",
    "shell_snapshot",
    "--disable",
    "shell_tool",
    "--disable",
    "unified_exec",
    "--disable",
    "apps",
    "--disable",
    "plugins",
    "--disable",
    "hooks",
    "--disable",
    "multi_agent",
    "--disable",
    "multi_agent_v2",
    "--disable",
    "memories",
    "--disable",
    "view_image",
    "--disable",
    "browser_use",
    "--disable",
    "computer_use",
    "--disable",
    "image_generation",
    "--disable",
    "skill_search",
    "--disable",
    "workspace_dependencies",
    "--disable",
    "code_mode_host",
    "--disable",
    "code_mode",
    "--disable",
    "goals",
    "--disable",
    "sleep_tool",
    "--disable",
    "tool_suggest",
    "-",
];

struct ExtractionCancellation(Option<Arc<AtomicBool>>);
impl Drop for ExtractionCancellation {
    fn drop(&mut self) {
        if let Some(active) = &self.0 {
            active.store(false, Ordering::Release);
        }
    }
}
async fn extraction_cancelled(active: &AtomicBool) {
    while active.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
fn extraction_budget(budget: &AtomicUsize, size: usize, limit: usize) -> Result<()> {
    if budget.fetch_add(size, Ordering::Relaxed) + size > limit {
        Err("EXTRACTION_INVALID_OUTPUT".into())
    } else {
        Ok(())
    }
}
async fn extraction_stderr(
    mut stderr: ChildStderr,
    budget: Arc<AtomicUsize>,
    limit: usize,
) -> Result<()> {
    let mut bytes = [0; 8192];
    loop {
        let size = stderr
            .read(&mut bytes)
            .await
            .map_err(|_| "EXTRACTION_PROVIDER_FAILED")?;
        if size == 0 {
            return Ok(());
        }
        extraction_budget(&budget, size, limit)?;
    }
}
async fn extraction_process<T, F>(
    mut command: Command,
    input: &[u8],
    active: &AtomicBool,
    bound: Duration,
    limit: usize,
    read: impl FnOnce(ChildStdout, Arc<AtomicUsize>) -> F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    if !active.load(Ordering::Acquire) {
        return Err("EXTRACTION_CANCELLED".into());
    }
    command.stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "EXTRACTION_PROVIDER_UNAVAILABLE"
        } else {
            "EXTRACTION_PROVIDER_FAILED"
        }
    })?;
    let (Some(mut stdin), Some(stdout), Some(stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        terminate(&mut child).await;
        return Err("EXTRACTION_PROVIDER_FAILED".into());
    };
    let budget = Arc::new(AtomicUsize::new(0));
    let result = tokio::select! {
        biased;
        _ = extraction_cancelled(active) => Err("EXTRACTION_CANCELLED".into()),
        result = timeout(bound, async {
            let (_, output, _) = tokio::try_join!(
                async move {
                    stdin.write_all(input).await.map_err(|_| "EXTRACTION_PROVIDER_FAILED".to_owned())?;
                    stdin.shutdown().await.map_err(|_| "EXTRACTION_PROVIDER_FAILED".to_owned())?;
                    drop(stdin);
                    Ok(())
                },
                read(stdout, budget.clone()),
                extraction_stderr(stderr, budget, limit),
            )?;
            let status = child.wait().await.map_err(|_| "EXTRACTION_PROVIDER_FAILED")?;
            if status.success() { Ok(output) } else { Err("EXTRACTION_PROVIDER_FAILED".into()) }
        }) => result.unwrap_or_else(|_| Err("EXTRACTION_TIMEOUT".into())),
    };
    if result.is_err() {
        terminate(&mut child).await;
    }
    result
}

#[derive(Default)]
struct ExtractionTranscript {
    message: Option<String>,
    completed: bool,
}
impl ExtractionTranscript {
    fn event(&mut self, event: Value) -> Result<()> {
        if !event.is_object() || self.completed {
            return Err("EXTRACTION_INVALID_OUTPUT".into());
        }
        match event["type"].as_str() {
            Some("thread.started") if !event["thread_id"].as_str().is_some_and(opaque) => {
                Err("EXTRACTION_INVALID_OUTPUT".into())
            }
            Some("thread.started" | "turn.started") => Ok(()),
            Some("item.started" | "item.updated" | "item.completed") => {
                let item = &event["item"];
                if !item["id"].as_str().is_some_and(opaque) {
                    return Err("EXTRACTION_INVALID_OUTPUT".into());
                }
                if item.get("text").is_some_and(|text| !text.is_string()) {
                    return Err("EXTRACTION_INVALID_OUTPUT".into());
                }
                match item["type"].as_str() {
                    Some("reasoning") => Ok(()),
                    Some("agent_message") => {
                        if event["type"] == "item.completed" {
                            let text = item["text"]
                                .as_str()
                                .filter(|v| v.len() <= EXTRACTION_JSON)
                                .ok_or("EXTRACTION_INVALID_OUTPUT")?;
                            self.message = Some(text.into());
                        }
                        Ok(())
                    }
                    _ => Err("EXTRACTION_INVALID_OUTPUT".into()),
                }
            }
            Some("turn.completed") => {
                self.completed = true;
                Ok(())
            }
            Some("turn.failed" | "error") => Err("EXTRACTION_PROVIDER_FAILED".into()),
            _ => Err("EXTRACTION_INVALID_OUTPUT".into()),
        }
    }
    fn finish(self) -> Result<Value> {
        if !self.completed {
            return Err("EXTRACTION_INVALID_OUTPUT".into());
        }
        let value: Value =
            serde_json::from_str(self.message.as_deref().ok_or("EXTRACTION_INVALID_OUTPUT")?)
                .map_err(|_| "EXTRACTION_INVALID_OUTPUT")?;
        if !value.is_object()
            || serde_json::to_vec(&value)
                .map_err(|_| "EXTRACTION_INVALID_OUTPUT")?
                .len()
                > EXTRACTION_JSON
        {
            return Err("EXTRACTION_INVALID_OUTPUT".into());
        }
        Ok(value)
    }
}
async fn extraction_transcript(
    stdout: ChildStdout,
    budget: Arc<AtomicUsize>,
) -> Result<ExtractionTranscript> {
    let mut reader = BufReader::new(stdout);
    let mut transcript = ExtractionTranscript::default();
    loop {
        let mut line = Vec::new();
        loop {
            let bytes = reader
                .fill_buf()
                .await
                .map_err(|_| "EXTRACTION_PROVIDER_FAILED")?;
            if bytes.is_empty() {
                if !line.is_empty() {
                    return Err("EXTRACTION_INVALID_OUTPUT".into());
                }
                return Ok(transcript);
            }
            let size = bytes
                .iter()
                .position(|v| *v == b'\n')
                .map_or(bytes.len(), |v| v + 1);
            extraction_budget(&budget, size, EXTRACTION_OUTPUT)?;
            if line.len() + size > LIMIT {
                return Err("EXTRACTION_INVALID_OUTPUT".into());
            }
            line.extend_from_slice(&bytes[..size]);
            reader.consume(size);
            if line.last() == Some(&b'\n') {
                break;
            }
        }
        transcript
            .event(serde_json::from_slice(&line).map_err(|_| "EXTRACTION_INVALID_OUTPUT")?)?;
    }
}

async fn extract_decisions_worker(
    programs: Programs,
    scratch: PathBuf,
    prompt: String,
    schema: Value,
    active: Arc<AtomicBool>,
    bound: Duration,
) -> Result<Value> {
    let deadline = tokio::time::Instant::now() + bound;
    if prompt.is_empty()
        || prompt.len() > LIMIT
        || !schema.is_object()
        || !scratch.is_absolute()
        || !std::fs::symlink_metadata(&scratch).is_ok_and(|v| v.is_dir())
    {
        return Err("EXTRACTION_INVALID_INPUT".into());
    }
    let bytes = serde_json::to_vec(&schema).map_err(|_| "EXTRACTION_INVALID_INPUT")?;
    if bytes.len() > EXTRACTION_JSON {
        return Err("EXTRACTION_INVALID_INPUT".into());
    }
    crate::store::private_directory(&scratch).map_err(|_| "EXTRACTION_INVALID_INPUT")?;
    let scratch = std::fs::canonicalize(scratch).map_err(|_| "EXTRACTION_INVALID_INPUT")?;
    let mut probe =
        command(&programs.codex, &["--version"]).map_err(|_| "EXTRACTION_PROVIDER_UNAVAILABLE")?;
    probe.current_dir(&scratch);
    let version = extraction_process(
        probe,
        &[],
        &active,
        programs.timeout.min(bound),
        1024,
        |mut stdout, budget| async move {
            let mut bytes = Vec::new();
            let mut chunk = [0; 1024];
            loop {
                let size = stdout
                    .read(&mut chunk)
                    .await
                    .map_err(|_| "EXTRACTION_PROVIDER_FAILED")?;
                if size == 0 {
                    break;
                }
                extraction_budget(&budget, size, 1024)?;
                bytes.extend_from_slice(&chunk[..size]);
            }
            String::from_utf8(bytes).map_err(|_| "EXTRACTION_PROVIDER_UNSUPPORTED".into())
        },
    )
    .await?;
    version
        .trim()
        .strip_prefix("codex-cli ")
        .filter(|v| semver(v) && at_least(v, [0, 155, 1], true))
        .ok_or("EXTRACTION_PROVIDER_UNSUPPORTED")?;
    if !active.load(Ordering::Acquire) {
        return Err("EXTRACTION_CANCELLED".into());
    }
    crate::store::atomic_write(&scratch.join("schema.json"), &bytes)
        .map_err(|_| "EXTRACTION_INVALID_INPUT")?;
    let mut process =
        command(&programs.codex, EXTRACTION_ARGS).map_err(|_| "EXTRACTION_PROVIDER_UNAVAILABLE")?;
    process.current_dir(scratch);
    let remaining = deadline
        .checked_duration_since(tokio::time::Instant::now())
        .ok_or("EXTRACTION_TIMEOUT")?;
    extraction_process(
        process,
        prompt.as_bytes(),
        &active,
        remaining,
        EXTRACTION_OUTPUT,
        extraction_transcript,
    )
    .await?
    .finish()
}

#[cfg(all(test, unix))]
#[tokio::test]
async fn extraction_process_deadline_is_bounded_and_reaped() {
    let active = AtomicBool::new(true);
    let started = std::time::Instant::now();
    let result = extraction_process(
        command(Path::new("/bin/sleep"), &["10"]).unwrap(),
        &[],
        &active,
        Duration::from_millis(20),
        EXTRACTION_OUTPUT,
        extraction_transcript,
    )
    .await;
    assert_eq!(result.err().as_deref(), Some("EXTRACTION_TIMEOUT"));
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(EXTRACTION_TIMEOUT, Duration::from_secs(60));
}

/// Ephemeral, owner-created MCP configuration. Dropping any binding revokes its lease.
pub struct MemoryConfig {
    pub config: Value,
    pub active: Arc<AtomicBool>,
}
impl Drop for MemoryConfig {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

// Reader revocation cannot wait for canonical lifecycle signals to be drained.
enum MemoryTurn {
    Prepared,
    Starting(u64, HashSet<String>),
    Running(String),
}
struct ReaderLease {
    active: Arc<AtomicBool>,
    turn: MemoryTurn,
}
struct ReaderLeases {
    closed: bool,
    pending: Option<Arc<AtomicBool>>,
    threads: HashMap<String, ReaderLease>,
}
impl ReaderLeases {
    fn close(&mut self) {
        self.closed = true;
        if let Some(active) = self.pending.take() {
            active.store(false, Ordering::Release);
        }
        for (_, lease) in self.threads.drain() {
            lease.active.store(false, Ordering::Release);
        }
    }
    fn pending(&mut self, active: Arc<AtomicBool>) {
        if self.closed {
            active.store(false, Ordering::Release);
        } else {
            self.pending = Some(active);
        }
    }
    fn prepared(&mut self, thread: &str, active: Arc<AtomicBool>) {
        self.pending = None;
        if self.closed {
            active.store(false, Ordering::Release);
        } else {
            self.remove(thread);
            self.threads.insert(
                thread.into(),
                ReaderLease {
                    active,
                    turn: MemoryTurn::Prepared,
                },
            );
        }
    }
    fn remove(&mut self, thread: &str) {
        if let Some(lease) = self.threads.remove(thread) {
            lease.active.store(false, Ordering::Release);
        }
    }
    fn starting(&mut self, thread: &str, request: u64) {
        if let Some(lease) = self.threads.get_mut(thread) {
            lease.turn = MemoryTurn::Starting(request, HashSet::new());
        }
    }
    fn received(&mut self, message: &Value) -> Result<()> {
        if message["method"] == "turn/completed" && message.get("id").is_none() {
            let params = &message["params"];
            let thread = params["threadId"]
                .as_str()
                .filter(|v| opaque(v))
                .ok_or("CODEX_PROTOCOL_MALFORMED")?;
            let turn = params["turn"]["id"]
                .as_str()
                .filter(|v| opaque(v))
                .ok_or("CODEX_PROTOCOL_MALFORMED")?;
            if !matches!(
                params["turn"]["status"].as_str(),
                Some("completed" | "failed" | "interrupted")
            ) {
                return Err("CODEX_PROTOCOL_MALFORMED".into());
            }
            if let Some(lease) = self.threads.get_mut(thread) {
                match &mut lease.turn {
                    MemoryTurn::Prepared => {}
                    MemoryTurn::Starting(_, candidates) => {
                        if candidates.len() >= 64 {
                            return Err("CODEX_PROTOCOL_MALFORMED".into());
                        }
                        candidates.insert(turn.into());
                    }
                    MemoryTurn::Running(expected) if expected == turn => self.remove(thread),
                    MemoryTurn::Running(_) => {}
                }
            }
        } else if let Some(request) = message["id"]
            .as_u64()
            .filter(|_| message.get("method").is_none())
        {
            let thread = self.threads.iter().find_map(|(thread, lease)| {
                matches!(lease.turn, MemoryTurn::Starting(id, _) if id == request)
                    .then(|| thread.clone())
            });
            if let Some(thread) = thread {
                if message.get("error").is_some() {
                    self.remove(&thread);
                    return Ok(());
                }
                let turn = &message["result"]["turn"];
                let id = turn["id"].as_str().filter(|v| opaque(v));
                let status = turn["status"].as_str();
                if id.is_none()
                    || !matches!(
                        status,
                        Some("inProgress" | "completed" | "failed" | "interrupted")
                    )
                {
                    return Err("CODEX_PROTOCOL_MALFORMED".into());
                }
                let id = id.unwrap();
                let lease = self.threads.get_mut(&thread).unwrap();
                let terminal = matches!(&lease.turn, MemoryTurn::Starting(_, candidates) if candidates.contains(id));
                if terminal || status != Some("inProgress") {
                    self.remove(&thread);
                } else {
                    lease.turn = MemoryTurn::Running(id.into());
                }
            }
        }
        Ok(())
    }
}
fn lease_guard(leases: &StdMutex<ReaderLeases>) -> MutexGuard<'_, ReaderLeases> {
    leases.lock().unwrap_or_else(|error| error.into_inner())
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn opaque(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 256
        && v.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
}
fn iso(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    let d = DateTime::parse_from_rfc3339(s).ok()?;
    (d.to_rfc3339_opts(SecondsFormat::Millis, true) == s).then(|| s.into())
}
fn seconds(v: &Value) -> Option<String> {
    let n = v.as_i64()?;
    if n < 0 {
        return None;
    }
    let d = DateTime::from_timestamp(n, 0)?;
    Some(d.to_rfc3339_opts(SecondsFormat::Millis, true))
}
fn semver(v: &str) -> bool {
    regex::Regex::new(
        r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$",
    )
    .unwrap()
    .is_match(v)
}
fn at_least(v: &str, minimum: [u64; 3], stable: bool) -> bool {
    if stable && v.contains('-') {
        return false;
    }
    let base = v.split(['-', '+']).next().unwrap_or("");
    let p: Vec<_> = base
        .split('.')
        .filter_map(|n| n.parse::<u64>().ok())
        .collect();
    p.len() == 3 && p.as_slice() >= minimum.as_slice()
}

/// Internal configuration for conformance tests; the desktop exposes no executable picker.
#[derive(Clone)]
pub struct Programs {
    pub codex: PathBuf,
    pub claude: PathBuf,
    pub gemini: PathBuf,
    pub timeout: Duration,
}
impl Default for Programs {
    fn default() -> Self {
        Self {
            codex: discover("codex"),
            claude: discover("claude"),
            gemini: discover("gemini"),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}
fn discover(name: &str) -> PathBuf {
    let mut dirs: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    #[cfg(unix)]
    {
        dirs.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ]);
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            dirs.extend([
                home.join(".local/bin"),
                home.join(".npm-global/bin"),
                home.join(".npm/bin"),
                home.join("bin"),
            ]);
        }
    }
    #[cfg(windows)]
    {
        if let Some(app) = std::env::var_os("APPDATA") {
            dirs.push(PathBuf::from(app).join("npm"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Programs"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            dirs.push(PathBuf::from(home).join(".local/bin"));
        }
    }
    for dir in dirs {
        for ext in if cfg!(windows) {
            &[".exe", ".cmd", ".bat", ""][..]
        } else {
            &[""][..]
        } {
            let p = dir.join(format!("{name}{ext}"));
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from(name)
}

fn command(program: &Path, args: &[&str]) -> Result<Command> {
    #[cfg(windows)]
    let mut cmd = {
        // npm distributes .cmd shims. Only a discovered executable and fixed adapter arguments
        // enter cmd.exe; prompts are sent over stdin. Reject shell metacharacters in its path.
        let found = discover_windows(program)?;
        if found
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("cmd") || x.eq_ignore_ascii_case("bat"))
        {
            let p = found.to_str().ok_or("PROVIDER_COMMAND_INVALID")?;
            if p.chars().any(|c| "\"%!?^&|<>()\r\n\0".contains(c))
                || args.iter().any(|a| {
                    a.chars()
                        .any(|c| !c.is_ascii_alphanumeric() && !"-_.={}".contains(c))
                })
            {
                return Err("PROVIDER_COMMAND_INVALID".into());
            }
            let mut c = Command::new("cmd.exe");
            c.args(["/D", "/V:OFF", "/S", "/C"]);
            // cmd.exe does not use the usual Windows argv escaping rules.
            // The full command is validated above and enclosed in cmd's outer quotes.
            c.as_std_mut()
                .raw_arg(format!("\"\"{}\" {}\"", p, args.join(" ")));
            c
        } else {
            let mut c = Command::new(found);
            c.args(args);
            c
        }
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new(program);
        c.args(args);
        c
    };
    #[cfg(unix)]
    {
        cmd.as_std_mut().process_group(0);
    }
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    Ok(cmd)
}
#[cfg(windows)]
fn discover_windows(program: &Path) -> Result<PathBuf> {
    if program.is_absolute() || program.components().count() > 1 {
        return Ok(program.into());
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        for ext in ["", ".exe", ".cmd", ".bat"] {
            let p = dir.join(format!("{}{ext}", program.to_string_lossy()));
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err("PROVIDER_NOT_FOUND".into())
}
async fn terminate(child: &mut Child) {
    if let Some(pid) = child.id() {
        #[cfg(unix)]
        {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
        #[cfg(windows)]
        {
            let mut task = Command::new("taskkill.exe");
            task.args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x08000000)
                .kill_on_drop(true);
            let _ = timeout(Duration::from_secs(2), task.status()).await;
        }
    }
    let _ = child.kill().await;
    let _ = timeout(Duration::from_secs(2), child.wait()).await;
}
async fn output(program: &Path, args: &[&str], bound: Duration, max: usize) -> Result<String> {
    output_streams(program, args, bound, max)
        .await
        .map(|(stdout, _)| stdout)
}
async fn output_streams(
    program: &Path,
    args: &[&str],
    bound: Duration,
    max: usize,
) -> Result<(String, String)> {
    let mut command = command(program, args).map_err(|e| {
        if e == "PROVIDER_NOT_FOUND" {
            "NOT_FOUND".to_string()
        } else {
            "FAILED".to_string()
        }
    })?;
    command.stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "NOT_FOUND"
        } else {
            "FAILED"
        }
        .to_string()
    })?;
    drop(child.stdin.take());
    let stdout = child.stdout.take().ok_or("FAILED")?;
    let stderr = child.stderr.take().ok_or("FAILED")?;
    let result = timeout(bound, async {
        let mut bytes = Vec::new();
        let mut err = Vec::new();
        let mut out = stdout.take((max + 1) as u64);
        let mut err_out = stderr.take((max + 1) as u64);
        let (a, b) = tokio::join!(out.read_to_end(&mut bytes), err_out.read_to_end(&mut err));
        a.map_err(|_| "FAILED")?;
        b.map_err(|_| "FAILED")?;
        if bytes.len() + err.len() > max {
            return Err("MALFORMED");
        }
        let status = child.wait().await.map_err(|_| "FAILED")?;
        if !status.success() {
            return Err("FAILED");
        }
        Ok((
            String::from_utf8(bytes).map_err(|_| "MALFORMED")?,
            String::from_utf8(err).map_err(|_| "MALFORMED")?,
        ))
    })
    .await;
    let r = match result {
        Ok(r) => r.map_err(str::to_string),
        Err(_) => Err("TIMEOUT".into()),
    };
    if r.is_err() {
        terminate(&mut child).await;
    }
    r
}
fn failed(provider: &str, code: &str, version: Option<&str>) -> Value {
    let display = match provider {
        "codex" => "Codex",
        "claude" => "Claude Code",
        _ => "Gemini CLI",
    };
    let message = match code {
        "CODEX_NOT_FOUND" => "Codex CLI is not installed or is not on PATH.",
        "CODEX_PROBE_FAILED" => "Codex CLI could not be checked.",
        "CODEX_PROBE_TIMEOUT" => "Codex CLI version check timed out.",
        "CODEX_VERSION_MALFORMED" => "Codex CLI returned an unsupported version.",
        "CODEX_SYSTEM_ERROR" => "Codex reported an observation error.",
        "CODEX_APP_SERVER_EXITED" => "Codex observation stopped unexpectedly.",
        "CODEX_APP_SERVER_START_FAILED" => "Codex observation could not start.",
        "CODEX_APP_SERVER_TIMEOUT" => "Codex observation timed out.",
        "CODEX_PROTOCOL_METHOD_ERROR" => "Codex rejected the observation request.",
        "CODEX_PROTOCOL_INVALID_JSON" | "CODEX_PROTOCOL_MALFORMED" => {
            "Codex returned an unsupported response."
        }
        "CLAUDE_NOT_FOUND" => "Claude Code CLI is not installed or is not on PATH.",
        "CLAUDE_PROBE_FAILED" => "Claude Code CLI could not be checked.",
        "CLAUDE_PROBE_TIMEOUT" => "Claude Code CLI version check timed out.",
        "CLAUDE_VERSION_MALFORMED" => "Claude Code CLI returned an unsupported version.",
        "CLAUDE_SURFACE_UNSUPPORTED" => {
            "This Claude Code version lacks the supported Agent View surface."
        }
        "CLAUDE_SNAPSHOT_FAILED" => "Claude Code Agent View could not be checked.",
        "CLAUDE_SNAPSHOT_TIMEOUT" => "Claude Code Agent View timed out.",
        "CLAUDE_SNAPSHOT_MALFORMED" => "Claude Code Agent View returned an unsupported response.",
        "CLAUDE_SNAPSHOT_UNSUPPORTED" => {
            "Claude Code Agent View returned an unsupported session shape."
        }
        "GEMINI_NOT_FOUND" => "Gemini CLI is not installed or is not on PATH.",
        "GEMINI_PROBE_TIMEOUT" => "Gemini CLI version check timed out.",
        "GEMINI_VERSION_MALFORMED" => "Gemini CLI returned an unsupported version.",
        "GEMINI_HOOK_SETUP_REQUIRED" => "Gemini CLI hook setup is required.",
        _ => "Gemini CLI could not be checked.",
    };
    json!({"schemaVersion":1,"provider":{"id":provider,"displayName":display,"state":if code.ends_with("NOT_FOUND"){"unavailable"}else{"degraded"},"version":version,"capabilities":{"recentObservation":false,"explicitLiveStatus":false},"error":{"code":code,"message":message}},"observedAt":now(),"sessions":[]})
}
fn available(provider: &str, version: &str, sessions: Vec<Value>, observed_at: &str) -> Value {
    json!({"schemaVersion":1,"provider":{"id":provider,"displayName":match provider {"codex"=>"Codex","claude"=>"Claude Code",_=>"Gemini CLI"},"state":"available","version":version,"capabilities":{"recentObservation":true,"explicitLiveStatus":true}},"observedAt":observed_at,"sessions":sessions})
}
async fn probe(program: &Path, provider: &str, bound: Duration) -> Result<String> {
    let prefix = provider.to_ascii_uppercase();
    let text = output(program, &["--version"], bound, 1024)
        .await
        .map_err(|e| {
            format!(
                "{prefix}_{}",
                if e == "NOT_FOUND" {
                    "NOT_FOUND"
                } else if e == "TIMEOUT" {
                    "PROBE_TIMEOUT"
                } else {
                    "PROBE_FAILED"
                }
            )
        })?;
    let text = text.trim();
    let v = match provider {
        "codex" => text.strip_prefix("codex-cli "),
        "claude" => text.strip_suffix(" (Claude Code)"),
        _ => Some(text),
    }
    .filter(|v| semver(v))
    .ok_or_else(|| format!("{prefix}_VERSION_MALFORMED"))?;
    Ok(v.into())
}

struct AppServer {
    child: Child,
    stdin: ChildStdin,
    incoming: mpsc::Receiver<Result<Value>>,
    reader: JoinHandle<()>,
    next: u64,
    bound: Duration,
    signals: Vec<Value>,
    owned: HashMap<String, String>,
    memory: HashMap<String, MemoryConfig>,
    leases: Arc<StdMutex<ReaderLeases>>,
}
impl AppServer {
    async fn open(
        program: &Path,
        bound: Duration,
        pending: Option<Arc<AtomicBool>>,
    ) -> Result<Self> {
        let mut child = command(program, &["app-server", "--stdio"])?
            .spawn()
            .map_err(|_| "CODEX_APP_SERVER_START_FAILED")?;
        let stdin = child.stdin.take().ok_or("CODEX_APP_SERVER_START_FAILED")?;
        let stdout = child.stdout.take().ok_or("CODEX_APP_SERVER_START_FAILED")?;
        let (tx, rx) = mpsc::channel(64);
        let leases = Arc::new(StdMutex::new(ReaderLeases {
            closed: false,
            pending,
            threads: HashMap::new(),
        }));
        let reader_leases = leases.clone();
        let reader = tokio::spawn(async move {
            let mut r = BufReader::new(stdout);
            let mut line = Vec::new();
            loop {
                let mut too_large = false;
                loop {
                    let buf = match r.fill_buf().await {
                        Ok(b) => b,
                        Err(_) => {
                            lease_guard(&reader_leases).close();
                            let _ = tx.send(Err("CODEX_APP_SERVER_EXITED".into())).await;
                            return;
                        }
                    };
                    if buf.is_empty() {
                        lease_guard(&reader_leases).close();
                        let _ = tx.send(Err("CODEX_APP_SERVER_EXITED".into())).await;
                        return;
                    }
                    let n = buf
                        .iter()
                        .position(|b| *b == b'\n')
                        .map(|n| n + 1)
                        .unwrap_or(buf.len());
                    let done = buf[n - 1] == b'\n';
                    if line.len() + n > LIMIT {
                        too_large = true;
                    } else {
                        line.extend_from_slice(&buf[..n]);
                    }
                    r.consume(n);
                    if done || too_large {
                        break;
                    }
                }
                if too_large {
                    lease_guard(&reader_leases).close();
                    let _ = tx.send(Err("CODEX_PROTOCOL_MALFORMED".into())).await;
                    return;
                }
                let message = serde_json::from_slice::<Value>(&line)
                    .map_err(|_| "CODEX_PROTOCOL_INVALID_JSON".to_string())
                    .and_then(|message| {
                        if !message.is_object() {
                            return Err("CODEX_PROTOCOL_MALFORMED".into());
                        }
                        lease_guard(&reader_leases).received(&message)?;
                        Ok(message)
                    });
                line.clear();
                let fatal = message.is_err();
                if fatal {
                    lease_guard(&reader_leases).close();
                }
                if tx.send(message).await.is_err() || fatal {
                    lease_guard(&reader_leases).close();
                    return;
                }
            }
        });
        let mut server = Self {
            child,
            stdin,
            incoming: rx,
            reader,
            next: 1,
            bound,
            signals: vec![],
            owned: HashMap::new(),
            memory: HashMap::new(),
            leases,
        };
        let init=server.request("initialize",json!({"clientInfo":{"name":"patchfleet","title":"Patchfleet","version":env!("CARGO_PKG_VERSION")},"capabilities":null})).await;
        if !matches!(init,Ok(ref v) if v.is_object()) {
            server.close().await;
            return Err(init.err().unwrap_or("CODEX_PROTOCOL_MALFORMED".into()));
        }
        if server.write(json!({"method":"initialized"})).await.is_err() {
            server.close().await;
            return Err("CODEX_APP_SERVER_EXITED".into());
        }
        Ok(server)
    }
    async fn write(&mut self, v: Value) -> Result<()> {
        let mut s = serde_json::to_vec(&v).map_err(|_| "CODEX_PROTOCOL_MALFORMED")?;
        s.push(b'\n');
        timeout(self.bound, self.stdin.write_all(&s))
            .await
            .map_err(|_| "CODEX_APP_SERVER_TIMEOUT")?
            .map_err(|_| "CODEX_APP_SERVER_EXITED".into())
    }
    async fn notification(&mut self, m: &Value) -> Result<()> {
        if m.get("id").is_some() {
            return self
                .write(json!({"id":m["id"],"error":{"code":-32601,"message":"Method not found"}}))
                .await;
        }
        if m["method"] == "turn/completed" || m["method"] == "turn/started" {
            let p = &m["params"];
            let id = p["threadId"].as_str().ok_or("CODEX_PROTOCOL_MALFORMED")?;
            if self.owned.contains_key(id) {
                if self.owned[id].is_empty() {
                    return Ok(());
                }
                let turn = &p["turn"];
                let tid = turn["id"]
                    .as_str()
                    .filter(|s| opaque(s))
                    .ok_or("CODEX_PROTOCOL_MALFORMED")?;
                if let Some(expected) = self.owned.get(id) {
                    if !expected.is_empty() && expected != tid {
                        return Err("CODEX_PROTOCOL_MALFORMED".into());
                    }
                }
                let status = match turn["status"].as_str() {
                    Some("inProgress") => "running",
                    Some("completed") => "completed",
                    Some("failed") => "failed",
                    Some("interrupted") => "interrupted",
                    _ => return Err("CODEX_PROTOCOL_MALFORMED".into()),
                };
                if self.signals.len() >= 64 {
                    return Err("CODEX_PROTOCOL_MALFORMED".into());
                }
                self.signals.push(json!({"schemaVersion":1,"providerId":"codex","providerSessionId":id,"status":status,"observedAt":now()}));
                if status != "running" {
                    self.revoke_memory(id);
                    self.owned.remove(id);
                }
            }
        }
        Ok(())
    }
    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next;
        self.next += 1;
        let starting =
            (method == "turn/start").then(|| params["threadId"].as_str().unwrap_or("").to_owned());
        if let Some(thread) = &starting {
            lease_guard(&self.leases).starting(thread, id);
        }
        self.write(json!({"method":method,"id":id,"params":params}))
            .await?;
        timeout(self.bound, async {
            let mut deferred = Vec::new();
            loop {
                let msg = self
                    .incoming
                    .recv()
                    .await
                    .ok_or("CODEX_APP_SERVER_EXITED")??;
                if msg["method"].is_string() {
                    if starting
                        .as_ref()
                        .is_some_and(|thread| msg["params"]["threadId"] == *thread)
                        && matches!(
                            msg["method"].as_str(),
                            Some("turn/started" | "turn/completed")
                        )
                        && msg.get("id").is_none()
                    {
                        if deferred.len() >= 64 {
                            return Err("CODEX_PROTOCOL_MALFORMED".into());
                        }
                        deferred.push(msg);
                        continue;
                    }
                    self.notification(&msg).await?;
                    continue;
                }
                if msg["id"].as_u64() != Some(id) {
                    return Err("CODEX_PROTOCOL_MALFORMED".into());
                }
                if msg.get("error").is_some() {
                    return Err("CODEX_PROTOCOL_METHOD_ERROR".into());
                }
                let result = msg
                    .get("result")
                    .cloned()
                    .ok_or("CODEX_PROTOCOL_MALFORMED")?;
                if let Some(thread) = &starting {
                    let turn = result["turn"]["id"]
                        .as_str()
                        .filter(|v| opaque(v))
                        .ok_or("CODEX_PROTOCOL_MALFORMED")?;
                    if !matches!(
                        result["turn"]["status"].as_str(),
                        Some("inProgress" | "completed" | "failed" | "interrupted")
                    ) {
                        return Err("CODEX_PROTOCOL_MALFORMED".into());
                    }
                    *self
                        .owned
                        .get_mut(thread)
                        .ok_or("CODEX_PROTOCOL_MALFORMED")? = turn.into();
                    for notification in deferred {
                        if notification["params"]["turn"]["id"] == turn {
                            self.notification(&notification).await?;
                        }
                    }
                }
                return Ok(result);
            }
        })
        .await
        .map_err(|_| "CODEX_APP_SERVER_TIMEOUT".to_string())?
    }
    fn revoke_memory(&mut self, thread: &str) {
        self.memory.remove(thread);
        lease_guard(&self.leases).remove(thread);
    }
    async fn memory_ready(&mut self, thread: &str) -> Result<()> {
        timeout(self.bound, async {
            loop {
                let mut cursor = Value::Null;
                loop {
                    let status = self
                        .request("mcpServerStatus/list", json!({"threadId":thread,"detail":"toolsAndAuthOnly","limit":100,"cursor":cursor}))
                        .await?;
                    let entries = status["data"].as_array().ok_or("MEMORY_CONNECTION_FAILED")?;
                    if let Some(server) = entries.iter().find(|v| v["name"] == "patchfleet_memory") {
                        if server["runtimeStatus"] == "connected" {
                            let tools = server["tools"].as_object().ok_or("MEMORY_CONNECTION_FAILED")?;
                            let names = ["project_context", "memory_search", "decision_read"];
                            return if server.get("toolsError") == Some(&Value::Null)
                                && tools.len() == names.len()
                                && names.iter().all(|name| tools.get(*name).is_some_and(|v| v["name"] == *name))
                            {
                                Ok(())
                            } else {
                                Err("MEMORY_CONNECTION_FAILED".into())
                            };
                        }
                        if server["runtimeStatus"] == "failed" || !server["runtimeStatus"].is_string() {
                            return Err("MEMORY_CONNECTION_FAILED".into());
                        }
                        break;
                    }
                    cursor = status.get("nextCursor").cloned().unwrap_or(Value::Null);
                    if cursor.is_null() {
                        break;
                    }
                    if !cursor.is_string() {
                        return Err("MEMORY_CONNECTION_FAILED".into());
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .map_err(|_| "MEMORY_CONNECTION_FAILED".to_string())?
        .map_err(|_: String| "MEMORY_CONNECTION_FAILED".to_string())?;
        let context = self
            .request("mcpServer/tool/call", json!({"threadId":thread,"server":"patchfleet_memory","tool":"project_context","arguments":{"limit":20}}))
            .await
            .map_err(|_| "MEMORY_CONNECTION_FAILED".to_string())?;
        if context["isError"] == false && context["structuredContent"].is_object() {
            Ok(())
        } else {
            Err("MEMORY_CONNECTION_FAILED".into())
        }
    }
    async fn close(&mut self) {
        lease_guard(&self.leases).close();
        self.memory.clear();
        terminate(&mut self.child).await;
        self.reader.abort();
        let _ = (&mut self.reader).await;
    }
}

pub async fn observe_all(_data_dir: &Path) -> Result<Vec<Value>> {
    observe_with(&Programs::default()).await
}
pub async fn observe_with(p: &Programs) -> Result<Vec<Value>> {
    let (a, b, c) = tokio::join!(observe_codex(p), observe_claude(p), observe_gemini(p));
    Ok(vec![a, b, c])
}
async fn observe_codex(p: &Programs) -> Value {
    let version = match probe(&p.codex, "codex", p.timeout).await {
        Ok(v) => v,
        Err(e) => return failed("codex", &e, None),
    };
    let observed_at = now();
    let mut server = match AppServer::open(&p.codex, p.timeout, None).await {
        Ok(c) => c,
        Err(e) => return failed("codex", &e, Some(&version)),
    };
    let result:Result<Value>=timeout(p.timeout.saturating_mul(2),async {
        let list=server.request("thread/list",json!({"archived":false,"limit":20,"sortDirection":"desc","sortKey":"recency_at","sourceKinds":["cli","vscode","exec","appServer"]})).await?;
        let entries=list["data"].as_array().ok_or("CODEX_PROTOCOL_MALFORMED")?;let mut seen=HashSet::new();let mut sessions=vec![];let mut system_error=false;
        for entry in entries.iter().take(20) {let id=entry["id"].as_str().filter(|s|opaque(s)).ok_or("CODEX_PROTOCOL_MALFORMED")?;if !seen.insert(id){return Err("CODEX_PROTOCOL_MALFORMED".into());}
            let r=server.request("thread/read",json!({"threadId":id,"includeTurns":true})).await?;let t=&r["thread"];if t["id"]!=id {return Err("CODEX_PROTOCOL_MALFORMED".into());}
            let turns=t["turns"].as_array().ok_or("CODEX_PROTOCOL_MALFORMED")?;let native=t["status"]["type"].as_str().ok_or("CODEX_PROTOCOL_MALFORMED")?;system_error|=native=="systemError";
            let latest=turns.last();if latest.is_some_and(|l|!l["status"].is_string()){return Err("CODEX_PROTOCOL_MALFORMED".into());}
            let status=match latest.and_then(|l|l["status"].as_str()){Some("completed")=>"completed",Some("failed")=>"failed",Some("interrupted")=>"interrupted",Some("inProgress")=>"running",_=>if native=="active"{"running"}else{"unknown"}};
            let mut session=json!({"providerSessionId":id,"status":status,"createdAt":seconds(&t["createdAt"]).ok_or("CODEX_PROTOCOL_MALFORMED")?,"lastObservedAt":observed_at});
            if status!="running"&&status!="unknown" {if let Some(at)=latest.and_then(|l|l.get("completedAt")).filter(|v|!v.is_null()){session["terminalAt"]=json!(seconds(at).ok_or("CODEX_PROTOCOL_MALFORMED")?);}}
            sessions.push(session);
        }
        let mut v=available("codex",&version,sessions,&observed_at);if system_error {v["provider"]["state"]=json!("degraded");v["provider"]["error"]=failed("codex","CODEX_SYSTEM_ERROR",None)["provider"]["error"].clone();}Ok(v)
    }).await.unwrap_or_else(|_|Err("CODEX_APP_SERVER_TIMEOUT".into()));
    server.close().await;
    result.unwrap_or_else(|e| failed("codex", &e, Some(&version)))
}
async fn observe_claude(p: &Programs) -> Value {
    let v = match probe(&p.claude, "claude", p.timeout).await {
        Ok(v) => v,
        Err(e) => return failed("claude", &e, None),
    };
    if !at_least(&v, [2, 1, 169], false) {
        return failed("claude", "CLAUDE_SURFACE_UNSUPPORTED", Some(&v));
    }
    let native = match output(&p.claude, &["agents", "--json", "--all"], p.timeout, LIMIT).await {
        Ok(s) => s,
        Err(e) => {
            return failed(
                "claude",
                if e == "TIMEOUT" {
                    "CLAUDE_SNAPSHOT_TIMEOUT"
                } else {
                    "CLAUDE_SNAPSHOT_FAILED"
                },
                Some(&v),
            )
        }
    };
    let native: Value = match serde_json::from_str(&native) {
        Ok(v) => v,
        Err(_) => return failed("claude", "CLAUDE_SNAPSHOT_MALFORMED", Some(&v)),
    };
    let observed_at = now();
    let result: Option<Vec<Value>> = (|| {
        let mut seen = HashSet::new();
        let mut sessions = vec![];
        for e in native.as_array()? {
            let job = e.get("id").is_some();
            let id = format!(
                "{}:{}",
                if job { "job" } else { "session" },
                e[if job { "id" } else { "sessionId" }].as_str()?
            );
            if !opaque(&id) || !seen.insert(id.clone()) {
                return None;
            }
            let status = match (job, e[if job { "state" } else { "status" }].as_str()?) {
                (true, "working") | (false, "running") => "running",
                (true, "done") => "completed",
                (true, "failed") => "failed",
                (true, "stopped") => "interrupted",
                (true, "blocked") | (false, "waiting" | "idle") => "unknown",
                _ => return None,
            };
            let at = if let Some(n) = e["startedAt"].as_i64() {
                if !(1_000_000_000_000..=9_999_999_999_999).contains(&n) {
                    return None;
                }
                DateTime::from_timestamp_millis(n)?.to_rfc3339_opts(SecondsFormat::Millis, true)
            } else {
                iso(&e["startedAt"])?
            };
            sessions.push(json!({"providerSessionId":id,"status":status,"createdAt":at,"lastObservedAt":observed_at}));
        }
        sessions.sort_by(|a, b| {
            let terminal = |v: &Value| {
                matches!(
                    v["status"].as_str(),
                    Some("completed" | "failed" | "interrupted")
                )
            };
            terminal(a)
                .cmp(&terminal(b))
                .then_with(|| b["createdAt"].as_str().cmp(&a["createdAt"].as_str()))
                .then_with(|| {
                    a["providerSessionId"]
                        .as_str()
                        .cmp(&b["providerSessionId"].as_str())
                })
        });
        sessions.truncate(20);
        Some(sessions)
    })();
    result
        .map(|s| available("claude", &v, s, &observed_at))
        .unwrap_or_else(|| failed("claude", "CLAUDE_SNAPSHOT_UNSUPPORTED", Some(&v)))
}
async fn observe_gemini(p: &Programs) -> Value {
    let v = match probe(&p.gemini, "gemini", p.timeout).await {
        Ok(v) => v,
        Err(e) => return failed("gemini", &e, None),
    };
    let (stdout, stderr) = match output_streams(
        &p.gemini,
        &["extensions", "list", "--output-format", "json"],
        p.timeout,
        65536,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            return failed(
                "gemini",
                if e == "TIMEOUT" {
                    "GEMINI_PROBE_TIMEOUT"
                } else {
                    "GEMINI_PROBE_FAILED"
                },
                Some(&v),
            )
        }
    };
    if !stdout.trim().is_empty() && !stderr.trim().is_empty() {
        return failed("gemini", "GEMINI_PROBE_FAILED", Some(&v));
    }
    let extensions = if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    };
    let parsed = serde_json::from_str::<Value>(&extensions);
    match parsed.ok().and_then(|v| v.as_array().cloned()) {
        Some(items) => {
            let ext = items.iter().find(|e| e["name"] == "patchfleet-gemini");
            match ext.map(|e| e["isActive"].as_bool()) {
                Some(Some(true)) => available("gemini", &v, vec![], &now()),
                Some(None) => failed("gemini", "GEMINI_PROBE_FAILED", Some(&v)),
                _ => failed("gemini", "GEMINI_HOOK_SETUP_REQUIRED", Some(&v)),
            }
        }
        None => {
            if extensions.trim().is_empty() {
                failed("gemini", "GEMINI_HOOK_SETUP_REQUIRED", Some(&v))
            } else {
                failed("gemini", "GEMINI_PROBE_FAILED", Some(&v))
            }
        }
    }
}
pub fn supports_control(observation: &Value) -> bool {
    let p = &observation["provider"];
    p["id"] == "codex"
        && p["state"] == "available"
        && p["capabilities"]["recentObservation"] == true
        && p["capabilities"]["explicitLiveStatus"] == true
        && p["version"]
            .as_str()
            .is_some_and(|v| semver(v) && at_least(v, [0, 144, 1], true))
}
pub fn supports_codex_control(observation: &Value) -> bool {
    supports_control(observation)
}

pub struct Controller {
    programs: Programs,
    server: Mutex<Option<AppServer>>,
    lost: Mutex<Vec<Value>>,
}
impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}
impl Controller {
    pub fn new() -> Self {
        Self::with_programs(Programs::default())
    }
    pub fn with_programs(programs: Programs) -> Self {
        Self {
            programs,
            server: Mutex::new(None),
            lost: Mutex::new(vec![]),
        }
    }
    /// Source text goes only to stdin; this independent worker never locks owned work sessions.
    pub async fn extract_decisions(
        &self,
        scratch: &Path,
        prompt: &str,
        schema: &Value,
        active: Arc<AtomicBool>,
    ) -> Result<Value> {
        let mut cancellation = ExtractionCancellation(Some(active.clone()));
        let result = async {
            if prompt.is_empty() || prompt.len() > LIMIT {
                return Err("EXTRACTION_INVALID_INPUT".into());
            }
            tokio::spawn(extract_decisions_worker(
                self.programs.clone(),
                scratch.to_owned(),
                prompt.to_owned(),
                schema.clone(),
                active,
                EXTRACTION_TIMEOUT,
            ))
            .await
            .map_err(|_| "EXTRACTION_PROVIDER_FAILED".to_owned())?
        }
        .await;
        cancellation.0 = None;
        result
    }
    async fn discard(&self, guard: &mut Option<AppServer>) {
        if let Some(s) = guard.as_mut() {
            lease_guard(&s.leases).close();
            s.memory.clear();
            let mut lost = self.lost.lock().await;
            lost.append(&mut s.signals);
            for id in s.owned.keys() {
                lost.push(json!({"schemaVersion":1,"providerId":"codex","providerSessionId":id,"status":"unknown","observedAt":now()}));
            }
            s.close().await;
        }
        *guard = None;
    }
    pub async fn prepare(&self, work: &Value, run: &Value) -> Result<String> {
        self.prepare_with_memory(work, run, None).await
    }
    pub async fn prepare_with_memory(
        &self,
        work: &Value,
        run: &Value,
        memory: Option<MemoryConfig>,
    ) -> Result<String> {
        let cwd = validate_workspace(
            work["workingDirectory"]
                .as_str()
                .ok_or("WORKSPACE_NOT_ALLOWED")?,
        )
        .await?;
        let intent = run
            .get("intentId")
            .or_else(|| run.get("runId"))
            .and_then(Value::as_str)
            .filter(|s| opaque(s) && s.len() <= 245)
            .ok_or("CODEX_PROTOCOL_MALFORMED")?;
        let version = probe(&self.programs.codex, "codex", self.programs.timeout).await?;
        if !at_least(&version, [0, 144, 1], true) {
            return Err("CODEX_CONTROL_UNSUPPORTED".into());
        }
        if memory.is_some() && !at_least(&version, [0, 155, 1], true) {
            return Err("AGENT_MEMORY_UNSUPPORTED".into());
        }
        if memory
            .as_ref()
            .is_some_and(|v| !v.active.load(Ordering::Acquire))
        {
            return Err("MEMORY_CONNECTION_FAILED".into());
        }
        let mut guard = self.server.lock().await;
        if guard.is_none() {
            *guard = Some(
                AppServer::open(
                    &self.programs.codex,
                    self.programs.timeout,
                    memory.as_ref().map(|v| v.active.clone()),
                )
                .await?,
            );
        }
        let source = format!("patchfleet:{intent}");
        let s = guard.as_mut().unwrap();
        if let Some(memory) = memory.as_ref() {
            lease_guard(&s.leases).pending(memory.active.clone());
        }
        let mut params = json!({"cwd":cwd,"approvalPolicy":"never","sandbox":"workspace-write","ephemeral":false,"threadSource":source,"experimentalRawEvents":false});
        if let Some(memory) = memory.as_ref() {
            params["config"] = memory.config.clone();
        }
        let result = s.request("thread/start", params).await;
        let id = result.as_ref().ok().and_then(|r| {
            let t = &r["thread"];
            if t["cwd"] != cwd || t["threadSource"] != source || t["ephemeral"] != false {
                return None;
            }
            t["id"].as_str().filter(|s| opaque(s)).map(str::to_string)
        });
        if let Some(id) = id {
            if let Some(memory) = memory.as_ref() {
                lease_guard(&s.leases).prepared(&id, memory.active.clone());
            }
            if memory.is_some() && s.memory_ready(&id).await.is_err() {
                self.discard(&mut guard).await;
                return Err("MEMORY_CONNECTION_FAILED".into());
            }
            if let Some(memory) = memory {
                if !memory.active.load(Ordering::Acquire) {
                    self.discard(&mut guard).await;
                    return Err("MEMORY_CONNECTION_FAILED".into());
                }
                s.memory.insert(id.clone(), memory);
            }
            s.owned.insert(id.clone(), String::new());
            Ok(id)
        } else {
            let err = if matches!(result,Err(ref e) if e=="CODEX_PROTOCOL_METHOD_ERROR") {
                "CODEX_THREAD_START_REJECTED"
            } else {
                "CODEX_PREPARE_FAILED"
            };
            self.discard(&mut guard).await;
            Err(err.into())
        }
    }
    pub async fn start_prepared(
        &self,
        work: &Value,
        run: &Value,
        session: &str,
    ) -> Result<(String, String)> {
        validate_workspace(
            work["workingDirectory"]
                .as_str()
                .ok_or("WORKSPACE_NOT_ALLOWED")?,
        )
        .await?;
        let instruction = work["instruction"]
            .as_str()
            .filter(|s| !s.is_empty() && s.encode_utf16().count() <= 50_000)
            .ok_or("CODEX_PROTOCOL_MALFORMED")?;
        let instruction = match work.get("_memoryGuidance") {
            Some(value) => {
                let guidance = value
                    .as_str()
                    .filter(|s| !s.is_empty() && s.encode_utf16().count() <= 2048)
                    .ok_or("CODEX_PROTOCOL_MALFORMED")?;
                format!("{guidance}\n\n{instruction}")
            }
            None => instruction.to_owned(),
        };
        let intent = run
            .get("intentId")
            .or_else(|| run.get("runId"))
            .and_then(Value::as_str)
            .filter(|s| opaque(s))
            .ok_or("CODEX_PROTOCOL_MALFORMED")?;
        let mut guard = self.server.lock().await;
        let s = guard.as_mut().ok_or("CODEX_SESSION_LOST")?;
        if !opaque(session) || !s.owned.contains_key(session) {
            return Err("CODEX_SESSION_LOST".into());
        }
        if !s.owned[session].is_empty() {
            return Err("CODEX_TURN_ALREADY_STARTED".into());
        }
        if s.memory
            .get(session)
            .is_some_and(|v| !v.active.load(Ordering::Acquire))
        {
            s.revoke_memory(session);
            return Err("MEMORY_CONNECTION_FAILED".into());
        }
        let result=s.request("turn/start",json!({"threadId":session,"clientUserMessageId":intent,"input":[{"type":"text","text":instruction,"text_elements":[]}]})).await;
        let tid = result
            .as_ref()
            .ok()
            .and_then(|r| r["turn"]["id"].as_str().filter(|s| opaque(s)))
            .map(str::to_string);
        let status = result
            .as_ref()
            .ok()
            .and_then(|r| r["turn"]["status"].as_str());
        if let Some(id) = tid.filter(|_| {
            matches!(
                status,
                Some("inProgress" | "completed" | "failed" | "interrupted")
            )
        }) {
            let already_terminal = s.signals.iter().any(|v| {
                v["providerSessionId"] == session
                    && matches!(
                        v["status"].as_str(),
                        Some("completed" | "failed" | "interrupted")
                    )
            });
            if !already_terminal {
                s.owned.insert(session.into(), id.clone());
            }
            if status != Some("inProgress") {
                s.revoke_memory(session);
                s.signals.push(json!({"schemaVersion":1,"providerId":"codex","providerSessionId":session,"status":status,"observedAt":now()}));
                s.owned.remove(session);
            }
            Ok((session.into(), id))
        } else {
            let code = if matches!(result,Err(ref e) if e=="CODEX_PROTOCOL_METHOD_ERROR") {
                "CODEX_TURN_START_REJECTED"
            } else {
                "START_OUTCOME_UNKNOWN"
            };
            self.discard(&mut guard).await;
            Err(code.into())
        }
    }
    pub async fn start(&self, work: &Value, run: &Value) -> Result<(String, String)> {
        let session = self.prepare(work, run).await?;
        self.start_prepared(work, run, &session).await
    }
    pub async fn cancel(&self, run: &Value) -> Result<()> {
        let session = run["providerSessionId"]
            .as_str()
            .filter(|s| opaque(s))
            .ok_or("CODEX_PROTOCOL_MALFORMED")?;
        let tid = run["providerTurnId"]
            .as_str()
            .filter(|s| opaque(s))
            .ok_or("CODEX_PROTOCOL_MALFORMED")?;
        let mut guard = self.server.lock().await;
        let s = guard.as_mut().ok_or("CODEX_SESSION_LOST")?;
        if s.owned.get(session).map(String::as_str) != Some(tid) {
            return Err("CODEX_SESSION_LOST".into());
        }
        s.revoke_memory(session);
        let result = s
            .request("turn/interrupt", json!({"threadId":session,"turnId":tid}))
            .await;
        if matches!(result,Ok(ref v) if v.is_object()) {
            Ok(())
        } else {
            let code = if matches!(result,Err(ref e) if e=="CODEX_PROTOCOL_METHOD_ERROR") {
                "CODEX_TURN_INTERRUPT_REJECTED"
            } else {
                "CANCEL_OUTCOME_UNKNOWN"
            };
            self.discard(&mut guard).await;
            Err(code.into())
        }
    }
    pub async fn drain_signals(&self) -> Vec<Value> {
        let mut guard = self.server.lock().await;
        let mut broken = false;
        if let Some(s) = guard.as_mut() {
            loop {
                match s.incoming.try_recv() {
                    Ok(Ok(m)) if m["method"].is_string() => {
                        if s.notification(&m).await.is_err() {
                            broken = true;
                            break;
                        }
                    }
                    Ok(_) | Err(mpsc::error::TryRecvError::Disconnected) => {
                        broken = true;
                        break;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                }
            }
        }
        if broken {
            self.discard(&mut guard).await;
        }
        let mut result = std::mem::take(&mut *self.lost.lock().await);
        if let Some(s) = guard.as_mut() {
            result.append(&mut s.signals);
        }
        result
    }
    pub async fn shutdown(&self) {
        let mut guard = self.server.lock().await;
        self.discard(&mut guard).await;
    }
}
pub async fn validate_workspace(value: &str) -> Result<String> {
    validate_workspace_for_registration(value)
        .await
        .map_err(|_| "WORKSPACE_NOT_ALLOWED".into())
}

/// Local setup diagnostics never enter the provider-control receipt contract.
pub async fn validate_workspace_for_registration(value: &str) -> Result<String> {
    if value.is_empty()
        || value.len() > 4096
        || value.contains('\0')
        || !Path::new(value).is_absolute()
    {
        return Err("WORKSPACE_PATH_INVALID".into());
    }
    let value = value.to_owned();
    tokio::task::spawn_blocking(move || validate_registration_directory(&value))
        .await
        .map_err(|_| "WORKSPACE_UNAVAILABLE")?
}

fn workspace_io_code(error: &std::io::Error, writing: bool) -> &'static str {
    use std::io::ErrorKind;
    #[cfg(unix)]
    if error.raw_os_error() == Some(libc::EPERM) {
        return "WORKSPACE_ACCESS_DENIED";
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{ERROR_PRIVILEGE_NOT_HELD, ERROR_WRITE_PROTECT};
        if error.raw_os_error() == Some(ERROR_PRIVILEGE_NOT_HELD as i32) {
            return "WORKSPACE_ACCESS_DENIED";
        }
        if error.raw_os_error() == Some(ERROR_WRITE_PROTECT as i32) {
            return "WORKSPACE_READ_ONLY";
        }
    }
    match error.kind() {
        ErrorKind::NotFound => "WORKSPACE_FOLDER_NOT_FOUND",
        ErrorKind::NotADirectory => "WORKSPACE_NOT_DIRECTORY",
        ErrorKind::PermissionDenied if writing => "WORKSPACE_READ_ONLY",
        ErrorKind::PermissionDenied => "WORKSPACE_ACCESS_DENIED",
        ErrorKind::ReadOnlyFilesystem => "WORKSPACE_READ_ONLY",
        ErrorKind::InvalidInput | ErrorKind::InvalidData | ErrorKind::InvalidFilename => {
            "WORKSPACE_PATH_INVALID"
        }
        _ => "WORKSPACE_UNAVAILABLE",
    }
}

fn validate_registration_directory(value: &str) -> Result<String> {
    let p = std::fs::canonicalize(value).map_err(|e| workspace_io_code(&e, false))?;
    let canonical = p
        .to_str()
        .filter(|s| s.len() <= 4096)
        .ok_or("WORKSPACE_PATH_INVALID")?;
    let meta = std::fs::metadata(&p).map_err(|e| workspace_io_code(&e, false))?;
    if !meta.is_dir() {
        return Err("WORKSPACE_NOT_DIRECTORY".into());
    }
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    if p.parent().is_none() || home.is_some_and(|h| std::fs::canonicalize(h).is_ok_and(|h| h == p))
    {
        return Err("WORKSPACE_UNSAFE_ROOT".into());
    }
    let git = std::fs::metadata(p.join(".git")).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "WORKSPACE_NOT_GIT_ROOT"
        } else {
            workspace_io_code(&e, false)
        }
    })?;
    if !git.is_dir() && !git.is_file() {
        return Err("WORKSPACE_NOT_GIT_ROOT".into());
    }
    let mut entries = std::fs::read_dir(&p).map_err(|e| workspace_io_code(&e, false))?;
    if let Some(entry) = entries.next() {
        entry.map_err(|e| workspace_io_code(&e, false))?;
    }
    workspace_access(&p, false).map_err(|e| workspace_io_code(&e, false))?;
    workspace_access(&p, true).map_err(|e| workspace_io_code(&e, true))?;
    Ok(canonical.to_owned())
}

#[cfg(unix)]
fn workspace_access(path: &Path, writing: bool) -> std::io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let path = CString::new(path.as_os_str().as_bytes())?;
    let mode = if writing {
        libc::W_OK
    } else {
        libc::R_OK | libc::X_OK
    };
    // Effective credentials and filesystem ACLs matter; mode bits alone do not.
    if unsafe { libc::faccessat(libc::AT_FDCWD, path.as_ptr(), mode, libc::AT_EACCESS) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn workspace_access(path: &Path, writing: bool) -> std::io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
        FILE_TRAVERSE,
    };
    let access = if writing {
        FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY
    } else {
        FILE_LIST_DIRECTORY | FILE_TRAVERSE
    };
    std::fs::OpenOptions::new()
        .access_mode(access)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map(|_| ())
}

#[cfg(not(any(unix, windows)))]
fn workspace_access(_path: &Path, _writing: bool) -> std::io::Result<()> {
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

#[cfg(all(test, unix))]
#[test]
fn workspace_policy_denials_stay_distinct_from_read_only_access() {
    let denied = std::io::Error::from_raw_os_error(libc::EACCES);
    assert_eq!(workspace_io_code(&denied, false), "WORKSPACE_ACCESS_DENIED");
    assert_eq!(workspace_io_code(&denied, true), "WORKSPACE_READ_ONLY");
    assert_eq!(
        workspace_io_code(&std::io::Error::from_raw_os_error(libc::EPERM), true),
        "WORKSPACE_ACCESS_DENIED"
    );
    assert_eq!(
        workspace_io_code(&std::io::Error::from_raw_os_error(libc::EROFS), true),
        "WORKSPACE_READ_ONLY"
    );
    assert_eq!(
        workspace_io_code(&std::io::Error::other("private detail"), false),
        "WORKSPACE_UNAVAILABLE"
    );
}

/// Strip the opt-in native hook payload to the five-field lifecycle contract.
pub fn decode_gemini_hook(input: &str) -> Result<Option<Value>> {
    if input.len() > 16384 {
        return Err("GEMINI_HOOK_INVALID".into());
    }
    let p: Value = serde_json::from_str(input).map_err(|_| "GEMINI_HOOK_INVALID")?;
    let status = match p["hook_event_name"].as_str() {
        Some("SessionStart") => "unknown",
        Some("BeforeAgent") => "running",
        Some("AfterAgent") => "completed",
        Some("SessionEnd") => "unknown",
        _ => return Err("GEMINI_HOOK_INVALID".into()),
    };
    let id = p["session_id"]
        .as_str()
        .filter(|s| opaque(s))
        .ok_or("GEMINI_HOOK_INVALID")?;
    let at = iso(&p["timestamp"]).ok_or("GEMINI_HOOK_INVALID")?;
    if p["hook_event_name"] == "SessionEnd" {
        return Ok(None);
    }
    Ok(Some(
        json!({"schemaVersion":1,"providerId":"gemini","providerSessionId":id,"status":status,"observedAt":at}),
    ))
}

pub struct InboxSignal {
    pub path: PathBuf,
    pub signal: Value,
}
fn validate_gemini_signal(v: &Value) -> Result<()> {
    let fields = [
        "schemaVersion",
        "providerId",
        "providerSessionId",
        "status",
        "observedAt",
    ];
    let m = v.as_object().ok_or("GEMINI_HOOK_INVALID")?;
    if m.len() != fields.len()
        || fields.iter().any(|k| !m.contains_key(*k))
        || v["schemaVersion"] != 1
        || v["providerId"] != "gemini"
        || !v["providerSessionId"].as_str().is_some_and(opaque)
        || !matches!(
            v["status"].as_str(),
            Some("completed" | "failed" | "interrupted" | "running" | "unknown")
        )
        || iso(&v["observedAt"]).is_none()
    {
        return Err("GEMINI_HOOK_INVALID".into());
    }
    Ok(())
}
/// Read at most 256 opt-in metadata signals. ACK only after the Host's durable append.
pub async fn read_gemini_inbox(data_dir: &Path) -> Result<Vec<InboxSignal>> {
    let directory = data_dir.join("gemini-inbox");
    let mut entries = match tokio::fs::read_dir(&directory).await {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(_) => return Err("GEMINI_INBOX_FAILED".into()),
    };
    let mut names = vec![];
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|_| "GEMINI_INBOX_FAILED")?
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("signal-") {
            names.push(name);
        }
        if names.len() >= 256 {
            break;
        }
    }
    names.sort();
    let valid_name = regex::Regex::new(r"^signal-\d{13}-[0-9a-fA-F-]{36}\.json$").unwrap();
    let mut result = vec![];
    for name in names {
        let path = directory.join(&name);
        let signal: Result<Value> = async {
            if !valid_name.is_match(&name) {
                return Err("GEMINI_HOOK_INVALID".into());
            }
            let m = tokio::fs::symlink_metadata(&path)
                .await
                .map_err(|_| "GEMINI_HOOK_INVALID")?;
            if !m.is_file() || m.len() > 4096 {
                return Err("GEMINI_HOOK_INVALID".into());
            }
            let mut file = tokio::fs::File::open(&path)
                .await
                .map_err(|_| "GEMINI_HOOK_INVALID")?;
            let mut bytes = vec![];
            (&mut file)
                .take(4097)
                .read_to_end(&mut bytes)
                .await
                .map_err(|_| "GEMINI_HOOK_INVALID")?;
            if bytes.len() > 4096 {
                return Err("GEMINI_HOOK_INVALID".into());
            }
            let v: Value = serde_json::from_slice(&bytes).map_err(|_| "GEMINI_HOOK_INVALID")?;
            validate_gemini_signal(&v)?;
            Ok(v)
        }
        .await;
        match signal {
            Ok(signal) => result.push(InboxSignal { path, signal }),
            Err(_) => {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
    }
    Ok(result)
}
pub async fn ack_gemini_inbox(entry: &InboxSignal) -> Result<()> {
    tokio::fs::remove_file(&entry.path)
        .await
        .map_err(|_| "GEMINI_INBOX_FAILED".into())
}
pub fn write_gemini_inbox(data_dir: &Path, signal: &Value) -> Result<()> {
    validate_gemini_signal(signal)?;
    let directory = data_dir.join("gemini-inbox");
    crate::store::private_directory(&directory).map_err(|_| "GEMINI_INBOX_FAILED")?;
    let timestamp = DateTime::parse_from_rfc3339(signal["observedAt"].as_str().unwrap())
        .map_err(|_| "GEMINI_HOOK_INVALID")?
        .timestamp_millis();
    if timestamp < 1_000_000_000_000 {
        return Err("GEMINI_HOOK_INVALID".into());
    }
    let path = directory.join(format!("signal-{timestamp}-{}.json", uuid::Uuid::new_v4()));
    crate::store::atomic_write(
        &path,
        &serde_json::to_vec(signal).map_err(|_| "GEMINI_HOOK_INVALID")?,
    )
    .map_err(|_| "GEMINI_INBOX_FAILED".into())
}
