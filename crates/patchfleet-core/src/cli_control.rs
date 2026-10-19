//! Supported headless CLI processes. Native content stays private to this adapter.
use super::*;
use tokio::sync::oneshot;

const RUN_OUTPUT: usize = 16 * 1024 * 1024;
const RUN_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

pub(super) fn supported(provider: &str, version: &str) -> bool {
    if !semver(version) {
        return false;
    }
    match provider {
        "claude" => at_least(version, [2, 1, 170], true),
        "gemini" => at_least(version, [0, 43, 0], true),
        _ => false,
    }
}
fn program<'a>(programs: &'a Programs, provider: &str) -> Result<&'a Path> {
    match provider {
        "claude" => Ok(&programs.claude),
        "gemini" => Ok(&programs.gemini),
        _ => Err("PROVIDER_CONTROL_UNSUPPORTED".into()),
    }
}

struct Configuration(PathBuf);
impl Configuration {
    fn new() -> Result<Self> {
        let directory =
            std::env::temp_dir().join(format!("moxcrew-provider-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).map_err(|_| "PROVIDER_CONFIG_FAILED")?;
        crate::store::private_directory(&directory).map_err(|_| "PROVIDER_CONFIG_FAILED")?;
        Ok(Self(directory))
    }
    fn write(&self, name: &str, value: &Value) -> Result<PathBuf> {
        let path = self.0.join(name);
        crate::store::atomic_write(
            &path,
            &serde_json::to_vec(value).map_err(|_| "PROVIDER_CONFIG_FAILED")?,
        )
        .map_err(|_| "PROVIDER_CONFIG_FAILED")?;
        Ok(path)
    }
}
impl Drop for Configuration {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn invocation(
    program: &Path,
    provider: &str,
    session: &str,
    cwd: &str,
    memory: Option<&MemoryConfig>,
    schema: Option<&Value>,
) -> Result<(Command, Configuration)> {
    if provider == "gemini" && gemini_managed_settings_present() {
        // A per-process settings file must never replace an administrator's policy.
        return Err("PROVIDER_CONFIG_CONFLICT".into());
    }
    let configuration = Configuration::new()?;
    let planning = schema.is_some();
    let mut servers = json!({});
    let mut allowed = Vec::<String>::new();
    if let Some(memory) = memory {
        if !memory.active.load(Ordering::Acquire) {
            return Err("MEMORY_CONNECTION_FAILED".into());
        }
        let source = &memory.config["mcp_servers"]["patchfleet_memory"];
        let executable = source["command"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or("MEMORY_CONNECTION_FAILED")?;
        let args = source["args"]
            .as_array()
            .filter(|v| v.len() == 2 && v[0] == "--memory-mcp" && v[1].is_string())
            .ok_or("MEMORY_CONNECTION_FAILED")?;
        let tools = source["enabled_tools"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 32)
            .ok_or("MEMORY_CONNECTION_FAILED")?;
        for tool in tools {
            let tool = tool
                .as_str()
                .filter(|v| opaque(v) && !v.contains(['.', ':', '-']))
                .ok_or("MEMORY_CONNECTION_FAILED")?;
            allowed.push(format!("mcp__patchfleet_memory__{tool}"));
        }
        servers["patchfleet_memory"] = json!({"command":executable,"args":args});
        if provider == "gemini" {
            servers["patchfleet_memory"]["trust"] = json!(true);
            servers["patchfleet_memory"]["includeTools"] = json!(tools);
            servers["patchfleet_memory"]["timeout"] = json!(5000);
        }
    }
    let mut cmd = if provider == "claude" {
        let mcp = configuration.write("mcp.json", &json!({"mcpServers":servers}))?;
        if !planning {
            allowed.extend([
                "Read(./**)".into(),
                "Edit(./**)".into(),
                "Write(./**)".into(),
            ]);
        }
        let settings = configuration.write(
            "claude.json",
            &json!({
            "disableAllHooks":true,"autoMemoryEnabled":false,"claudeMdExcludes":["**/*"],
                "permissions":{"allow":allowed,"defaultMode":"dontAsk"}
            }),
        )?;
        let mut args = vec![
            "--print".to_owned(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--permission-mode".into(),
            "dontAsk".into(),
            "--setting-sources".into(),
            String::new(),
            "--settings".into(),
            settings.to_string_lossy().into_owned(),
            "--strict-mcp-config".into(),
            "--mcp-config".into(),
            mcp.to_string_lossy().into_owned(),
            "--disable-slash-commands".into(),
            "--no-chrome".into(),
            "--no-session-persistence".into(),
            "--session-id".into(),
            session.into(),
            "--tools".into(),
            if planning {
                ""
            } else {
                "Read,Write,Edit,Glob,Grep"
            }
            .into(),
        ];
        if let Some(schema) = schema {
            args.extend(["--json-schema".into(), schema.to_string()]);
        }
        let mut cmd = command(
            program,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )?;
        // --bare disables subscription OAuth on supported versions; isolate settings without it.
        cmd.env_remove("CLAUDE_CODE_SIMPLE")
            .env_remove("CLAUDE_CODE_SAFE_MODE");
        cmd
    } else {
        let core = if planning {
            json!([])
        } else {
            json!([
                "list_directory",
                "read_file",
                "glob",
                "grep_search",
                "read_many_files",
                "write_file",
                "replace"
            ])
        };
        let settings = configuration.write("gemini.json", &json!({
            "mcpServers":servers,"mcp":{"serverCommand":"","allowed":if memory.is_some(){vec!["patchfleet_memory"]}else{vec![]}},
            "admin":{"mcp":{"enabled":memory.is_some()},"extensions":{"enabled":false},"skills":{"enabled":false}},
            "tools":{"core":core,"discoveryCommand":"","callCommand":"","exclude":["run_shell_command","web_fetch","google_web_search","delegate_to_agent","activate_skill"]},
            "hooksConfig":{"enabled":false},"skills":{"enabled":false},
            "context":{"fileName":[],"includeDirectories":[],"loadMemoryFromIncludeDirectories":false},
            "experimental":{"enableAgents":false,"autoMemory":false,"memoryV2":false,"jitContext":false,"extensionReloading":false},
            "general":{"enableAutoUpdate":false},"model":{"maxSessionTurns":if planning {1}else{100}},
            "telemetry":{"enabled":false,"logPrompts":false}
        }))?;
        let mut cmd = command(
            program,
            &[
                "--output-format",
                "stream-json",
                "--approval-mode",
                if planning { "plan" } else { "auto_edit" },
                "--extensions",
                "none",
                "--allowed-mcp-server-names",
                if memory.is_some() {
                    "patchfleet_memory"
                } else {
                    "moxcrew_no_servers"
                },
                "--session-id",
                session,
            ],
        )?;
        cmd.env("GEMINI_CLI_SYSTEM_SETTINGS_PATH", settings)
            .env("GEMINI_TELEMETRY_ENABLED", "false")
            .env_remove("GEMINI_SYSTEM_MD")
            .env_remove("GEMINI_CLI_IDE_SERVER_STDIO_COMMAND")
            .env_remove("GEMINI_CLI_IDE_SERVER_STDIO_ARGS");
        cmd
    };
    cmd.current_dir(cwd);
    Ok((cmd, configuration))
}

fn gemini_managed_settings_present() -> bool {
    if std::env::var_os("GEMINI_CLI_SYSTEM_SETTINGS_PATH").is_some() {
        return true;
    }
    #[cfg(target_os = "macos")]
    let settings = PathBuf::from("/Library/Application Support/GeminiCli/settings.json");
    #[cfg(all(unix, not(target_os = "macos")))]
    let settings = PathBuf::from("/etc/gemini-cli/settings.json");
    #[cfg(windows)]
    let settings =
        PathBuf::from(std::env::var_os("PROGRAMDATA").unwrap_or_else(|| "C:\\ProgramData".into()))
            .join("gemini-cli/settings.json");
    settings.try_exists().unwrap_or(true)
}

#[derive(Default)]
struct Transcript {
    initialized: bool,
    terminal: bool,
    text: String,
    value: Option<Value>,
    usage: Option<Value>,
}
impl Transcript {
    fn event(
        &mut self,
        provider: &str,
        session: &str,
        planning: bool,
        memory: bool,
        event: Value,
    ) -> Result<()> {
        if self.terminal || !event.is_object() {
            return Err("EXTRACTION_INVALID_OUTPUT".into());
        }
        let kind = event["type"].as_str().ok_or("EXTRACTION_INVALID_OUTPUT")?;
        let init = (provider == "claude" && kind == "system" && event["subtype"] == "init")
            || (provider == "gemini" && kind == "init");
        if init {
            if self.initialized || event["session_id"] != session {
                return Err("EXTRACTION_INVALID_OUTPUT".into());
            }
            if memory && provider == "claude" {
                let connected = event["mcp_servers"].as_array().is_some_and(|v| {
                    v.iter()
                        .any(|v| v["name"] == "patchfleet_memory" && v["status"] == "connected")
                });
                let errors = event["mcp_server_errors"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty());
                if !connected || errors {
                    return Err("MEMORY_CONNECTION_FAILED".into());
                }
            }
            self.initialized = true;
            return Ok(());
        }
        // Startup metadata may precede init; it never establishes readiness or success.
        if !self.initialized {
            if provider == "claude" && kind == "system" {
                return Ok(());
            }
            return Err("EXTRACTION_INVALID_OUTPUT".into());
        }
        if let Some(id) = event.get("session_id") {
            if id != session {
                return Err("EXTRACTION_INVALID_OUTPUT".into());
            }
        }
        if planning
            && (kind == "tool_use"
                || (kind == "assistant"
                    && event["message"]["content"]
                        .as_array()
                        .is_some_and(|v| v.iter().any(|c| c["type"] == "tool_use"))))
        {
            return Err("EXTRACTION_INVALID_OUTPUT".into());
        }
        match kind {
            "result" => {
                let success = if provider == "claude" {
                    event["subtype"] == "success"
                        && event["is_error"] == false
                        && !event["permission_denials"]
                            .as_array()
                            .is_some_and(|v| !v.is_empty())
                } else {
                    event["status"] == "success"
                };
                if !success {
                    return Err("EXTRACTION_PROVIDER_FAILED".into());
                }
                self.terminal = true;
                self.usage = usage(provider, &event);
                if planning {
                    self.value = if provider == "claude" && event["structured_output"].is_object() {
                        Some(event["structured_output"].clone())
                    } else {
                        let text = if provider == "claude" {
                            event["result"]
                                .as_str()
                                .ok_or("EXTRACTION_INVALID_OUTPUT")?
                        } else {
                            &self.text
                        };
                        Some(serde_json::from_str(text).map_err(|_| "EXTRACTION_INVALID_OUTPUT")?)
                    };
                    if !self.value.as_ref().is_some_and(Value::is_object)
                        || self.value.as_ref().unwrap().to_string().len() > EXTRACTION_JSON
                    {
                        return Err("EXTRACTION_INVALID_OUTPUT".into());
                    }
                }
            }
            "message" if provider == "gemini" && planning && event["role"] == "assistant" => {
                let content = event["content"]
                    .as_str()
                    .ok_or("EXTRACTION_INVALID_OUTPUT")?;
                if self.text.len() + content.len() > EXTRACTION_JSON {
                    return Err("EXTRACTION_INVALID_OUTPUT".into());
                }
                self.text.push_str(content);
            }
            "error" if event["severity"] != "warning" => {
                return Err("EXTRACTION_PROVIDER_FAILED".into())
            }
            "system" if event["subtype"] == "permission_denied" => {
                return Err("EXTRACTION_PROVIDER_FAILED".into())
            }
            "system" | "assistant" | "user" | "message" | "tool_use" | "tool_result" | "error"
            | "stream_event" | "rate_limit_event" => {}
            _ => return Err("EXTRACTION_INVALID_OUTPUT".into()),
        }
        Ok(())
    }
}
fn usage(provider: &str, event: &Value) -> Option<Value> {
    let native = &event[if provider == "claude" {
        "usage"
    } else {
        "stats"
    }];
    let mut value = json!({});
    for (target, source) in [
        ("inputTokens", "input_tokens"),
        ("outputTokens", "output_tokens"),
    ] {
        if let Some(count) = native[source].as_u64().filter(|v| *v <= i64::MAX as u64) {
            value[target] = json!(count);
        }
    }
    if provider == "claude" {
        if let Some(cost) = event["total_cost_usd"]
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1_000_000.0)
        {
            value["costMicros"] = json!((cost * 1_000_000.0).round() as u64);
            value["currency"] = json!("USD");
        }
    }
    (!value.as_object().unwrap().is_empty()).then_some(value)
}
async fn read_stream(
    stdout: ChildStdout,
    budget: Arc<AtomicUsize>,
    provider: String,
    session: String,
    planning: bool,
    memory: Option<Arc<AtomicBool>>,
) -> Result<Transcript> {
    let mut reader = BufReader::new(stdout);
    let mut transcript = Transcript::default();
    loop {
        let mut bytes = Vec::new();
        loop {
            let buffer = reader
                .fill_buf()
                .await
                .map_err(|_| "EXTRACTION_PROVIDER_FAILED")?;
            if buffer.is_empty() {
                if !bytes.is_empty() || !transcript.terminal {
                    return Err("EXTRACTION_INVALID_OUTPUT".into());
                }
                return Ok(transcript);
            }
            let end = buffer.iter().position(|v| *v == b'\n').map(|v| v + 1);
            let size = end.unwrap_or(buffer.len());
            extraction_budget(
                &budget,
                size,
                if planning {
                    EXTRACTION_OUTPUT
                } else {
                    RUN_OUTPUT
                },
            )?;
            if bytes.len() + size > LIMIT {
                return Err("EXTRACTION_INVALID_OUTPUT".into());
            }
            bytes.extend_from_slice(&buffer[..size]);
            reader.consume(size);
            if end.is_some() {
                break;
            }
        }
        let event = serde_json::from_slice(&bytes).map_err(|_| "EXTRACTION_INVALID_OUTPUT")?;
        let result = transcript.event(&provider, &session, planning, memory.is_some(), event);
        if transcript.terminal || result.is_err() {
            if let Some(active) = &memory {
                active.store(false, Ordering::Release);
            }
        }
        result?;
    }
}

struct Prepared {
    provider: String,
    work: String,
    run: String,
    cwd: String,
    command: Command,
    configuration: Configuration,
    memory: Option<MemoryConfig>,
}
struct Running {
    provider: String,
    run: String,
    turn: String,
    active: Arc<AtomicBool>,
    memory_active: Option<Arc<AtomicBool>>,
    task: JoinHandle<()>,
}
#[derive(Default)]
pub(super) struct Control {
    prepared: Mutex<HashMap<String, Prepared>>,
    running: Mutex<HashMap<String, Running>>,
    signals: Arc<Mutex<Vec<Value>>>,
    usage: Arc<Mutex<Vec<Value>>>,
}
impl Control {
    pub(super) async fn prepare(
        &self,
        programs: &Programs,
        work: &Value,
        run: &Value,
        memory: Option<MemoryConfig>,
        cwd: String,
    ) -> Result<String> {
        let provider = work["providerId"]
            .as_str()
            .ok_or("PROVIDER_CONTROL_UNSUPPORTED")?;
        if run["providerId"] != provider {
            return Err("PROVIDER_INVALID_INPUT".into());
        }
        let executable = program(programs, provider)?;
        let version = probe(executable, provider, programs.timeout).await?;
        if !supported(provider, &version) {
            return Err("PROVIDER_CONTROL_UNSUPPORTED".into());
        }
        let work_id = work["workItemId"]
            .as_str()
            .filter(|v| opaque(v))
            .ok_or("PROVIDER_INVALID_INPUT")?;
        let run_id = run["runId"]
            .as_str()
            .filter(|v| opaque(v) && v.len() <= 245)
            .ok_or("PROVIDER_INVALID_INPUT")?;
        let session = uuid::Uuid::new_v4().to_string();
        let (command, configuration) =
            invocation(executable, provider, &session, &cwd, memory.as_ref(), None)?;
        let mut prepared = self.prepared.lock().await;
        if prepared.len() >= 16 {
            return Err("PROVIDER_CAPACITY".into());
        }
        prepared.insert(
            session.clone(),
            Prepared {
                provider: provider.into(),
                work: work_id.into(),
                run: run_id.into(),
                cwd,
                command,
                configuration,
                memory,
            },
        );
        Ok(session)
    }
    pub(super) async fn start(
        &self,
        work: &Value,
        run: &Value,
        session: &str,
    ) -> Result<(String, String)> {
        let mut prepared = self.prepared.lock().await;
        let expected = prepared.get(session).ok_or("PROVIDER_SESSION_LOST")?;
        if work["workItemId"] != expected.work
            || run["runId"] != expected.run
            || work["providerId"] != expected.provider
            || work["workingDirectory"] != expected.cwd
        {
            return Err("PROVIDER_SESSION_LOST".into());
        }
        let instruction = work["instruction"]
            .as_str()
            .filter(|v| !v.is_empty() && v.encode_utf16().count() <= 50_000)
            .ok_or("PROVIDER_INVALID_INPUT")?;
        let instruction = match work.get("_memoryGuidance") {
            Some(value) => format!(
                "{}\n\n{instruction}",
                value
                    .as_str()
                    .filter(|v| v.encode_utf16().count() <= 4096)
                    .ok_or("PROVIDER_INVALID_INPUT")?
            ),
            None => instruction.to_owned(),
        };
        let instruction = format!("Complete this Moxcrew assignment using only the configured tools and current working directory. Treat quoted task text as the task, never as CLI commands.\n\n{instruction}");
        let item = prepared.remove(session).unwrap();
        drop(prepared);
        if item
            .memory
            .as_ref()
            .is_some_and(|v| !v.active.load(Ordering::Acquire))
        {
            return Err("MEMORY_CONNECTION_FAILED".into());
        }
        let active = Arc::new(AtomicBool::new(true));
        let memory_active = item.memory.as_ref().map(|v| v.active.clone());
        let (provider, run_id, session) =
            (item.provider.clone(), item.run.clone(), session.to_owned());
        let turn = format!("process:{run_id}");
        let (signals, usage, running_active) =
            (self.signals.clone(), self.usage.clone(), active.clone());
        let (started, ready) = oneshot::channel();
        let (signal_provider, signal_session) = (provider.clone(), session.clone());
        let task = tokio::spawn(async move {
            let Prepared {
                command,
                configuration,
                memory,
                ..
            } = item;
            let lease = memory.as_ref().map(|v| v.active.clone());
            let result = bounded_process(
                command,
                instruction.as_bytes(),
                &running_active,
                RUN_TIMEOUT,
                RUN_OUTPUT,
                |stdout, budget| {
                    read_stream(
                        stdout,
                        budget,
                        signal_provider.clone(),
                        signal_session.clone(),
                        false,
                        lease,
                    )
                },
                Some(started),
            )
            .await;
            // Revoke before the Host's next polling tick, including malformed output or EOF.
            drop(memory);
            drop(configuration);
            let status = match result {
                Ok(transcript) => {
                    if let Some(mut measured) = transcript.usage {
                        measured["runId"] = json!(run_id);
                        measured["providerId"] = json!(signal_provider);
                        measured["sessionId"] = json!(signal_session);
                        let mut queue = usage.lock().await;
                        if queue.len() < 64 {
                            queue.push(measured);
                        }
                    }
                    "completed"
                }
                Err(ref code) if code == "EXTRACTION_CANCELLED" => "interrupted",
                Err(_) => "failed",
            };
            let mut queue = signals.lock().await;
            if queue.len() < 64 {
                queue.push(json!({"schemaVersion":1,"providerId":signal_provider,"providerSessionId":signal_session,"status":status,"observedAt":now()}));
            }
        });
        let mut running = self.running.lock().await;
        if running.len() >= 16 {
            active.store(false, Ordering::Release);
            drop(running);
            let _ = task.await;
            return Err("PROVIDER_CAPACITY".into());
        }
        running.insert(
            session.clone(),
            Running {
                provider,
                run: run["runId"].as_str().unwrap().into(),
                turn: turn.clone(),
                active: active.clone(),
                memory_active,
                task,
            },
        );
        drop(running);
        if !matches!(timeout(DEFAULT_TIMEOUT, ready).await, Ok(Ok(()))) {
            active.store(false, Ordering::Release);
            return Err("START_OUTCOME_UNKNOWN".into());
        }
        Ok((session, turn))
    }
    pub(super) async fn cancel(&self, run: &Value) -> Result<()> {
        let session = run["providerSessionId"]
            .as_str()
            .ok_or("PROVIDER_SESSION_LOST")?;
        let mut running = self.running.lock().await;
        let item = running.get(session).ok_or("PROVIDER_SESSION_LOST")?;
        if run["runId"] != item.run
            || run["providerId"] != item.provider
            || run["providerTurnId"] != item.turn
        {
            return Err("PROVIDER_SESSION_LOST".into());
        }
        let item = running.remove(session).unwrap();
        drop(running);
        if let Some(active) = item.memory_active {
            active.store(false, Ordering::Release);
        }
        item.active.store(false, Ordering::Release);
        item.task
            .await
            .map_err(|_| "CANCEL_OUTCOME_UNKNOWN".to_owned())?;
        Ok(())
    }
    pub(super) async fn drain_signals(&self) -> Vec<Value> {
        self.running
            .lock()
            .await
            .retain(|_, run| !run.task.is_finished());
        std::mem::take(&mut *self.signals.lock().await)
    }
    pub(super) async fn drain_usage(&self) -> Vec<Value> {
        std::mem::take(&mut *self.usage.lock().await)
    }
    pub(super) async fn shutdown(&self) {
        self.prepared.lock().await.clear();
        let running = std::mem::take(&mut *self.running.lock().await);
        for run in running.values() {
            run.active.store(false, Ordering::Release);
            if let Some(active) = &run.memory_active {
                active.store(false, Ordering::Release);
            }
        }
        for (_, run) in running {
            let _ = run.task.await;
        }
    }
}
impl Drop for Control {
    fn drop(&mut self) {
        if let Ok(running) = self.running.try_lock() {
            for run in running.values() {
                run.active.store(false, Ordering::Release);
                if let Some(active) = &run.memory_active {
                    active.store(false, Ordering::Release);
                }
            }
        }
    }
}

pub(super) async fn generate(
    programs: &Programs,
    provider: &str,
    scratch: &Path,
    prompt: &str,
    schema: &Value,
    active: Arc<AtomicBool>,
) -> Result<Value> {
    if prompt.is_empty()
        || prompt.len() > LIMIT
        || !schema.is_object()
        || schema.to_string().len() > EXTRACTION_JSON
        || !scratch.is_absolute()
        || !std::fs::symlink_metadata(scratch)
            .is_ok_and(|v| v.is_dir() && !v.file_type().is_symlink())
    {
        return Err("EXTRACTION_INVALID_INPUT".into());
    }
    if !active.load(Ordering::Acquire) {
        return Err("EXTRACTION_CANCELLED".into());
    }
    let executable = program(programs, provider).map_err(|_| "EXTRACTION_PROVIDER_UNSUPPORTED")?;
    let deadline = tokio::time::Instant::now() + EXTRACTION_TIMEOUT;
    let probe =
        command(executable, &["--version"]).map_err(|_| "EXTRACTION_PROVIDER_UNAVAILABLE")?;
    let version = extraction_process(
        probe,
        &[],
        &active,
        programs.timeout.min(EXTRACTION_TIMEOUT),
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
    let version = if provider == "claude" {
        version
            .trim()
            .strip_suffix(" (Claude Code)")
            .ok_or("EXTRACTION_PROVIDER_UNSUPPORTED")?
    } else {
        version.trim()
    };
    if !semver(version) {
        return Err("EXTRACTION_PROVIDER_UNSUPPORTED".into());
    }
    if !supported(provider, version) {
        return Err("EXTRACTION_PROVIDER_UNSUPPORTED".into());
    }
    crate::store::private_directory(scratch).map_err(|_| "EXTRACTION_INVALID_INPUT")?;
    let cwd = std::fs::canonicalize(scratch).map_err(|_| "EXTRACTION_INVALID_INPUT")?;
    let session = uuid::Uuid::new_v4().to_string();
    let (command, _configuration) = invocation(
        executable,
        provider,
        &session,
        cwd.to_str().ok_or("EXTRACTION_INVALID_INPUT")?,
        None,
        Some(schema),
    )?;
    let prompt = if provider == "gemini" {
        format!("Return exactly one JSON object matching this schema, without Markdown or tools:\n{schema}\n\n{prompt}")
    } else {
        prompt.into()
    };
    let bound = deadline.saturating_duration_since(tokio::time::Instant::now());
    let transcript = extraction_process(
        command,
        prompt.as_bytes(),
        &active,
        bound,
        EXTRACTION_OUTPUT,
        |stdout, budget| read_stream(stdout, budget, provider.into(), session, true, None),
    )
    .await?;
    transcript.value.ok_or("EXTRACTION_INVALID_OUTPUT".into())
}

#[cfg(test)]
#[path = "cli_control_tests.rs"]
mod tests;
