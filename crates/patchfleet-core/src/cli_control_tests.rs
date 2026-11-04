use super::*;
use std::sync::OnceLock;

const FIXTURE: &str = r###"
use std::{env,fs,io::{self,Read,Write},process::{Command,Stdio},thread,time::Duration};
fn flush(){io::stdout().flush().unwrap();}
fn main(){
 let exe=env::current_exe().unwrap();let mode=exe.file_stem().unwrap().to_str().unwrap();let args:Vec<_>=env::args().collect();
 if args.get(1).map(String::as_str)==Some("--child"){thread::sleep(Duration::from_secs(20));return}
 let claude=mode.starts_with("claude");
 if args.get(1).map(String::as_str)==Some("--version"){
  if mode.contains("bad-version"){println!("SECRET_VERSION")}else if claude{println!("{} (Claude Code)",if mode.contains("old"){"2.1.169"}else{"2.1.170"})}else{println!("{}",if mode.contains("old"){"0.42.0"}else{"0.43.0"})};return
 }
 if args.get(1).map(String::as_str)==Some("auth"){println!("{}",r#"{"loggedIn":true,"authMethod":"claude.ai","private":"SECRET_AUTH"}"#);return}
 let get=|key:&str|args.windows(2).find(|v|v[0]==key).unwrap()[1].clone();
 assert_eq!(get("--output-format"),"stream-json");let session=get("--session-id");
 assert!(args.iter().all(|v|!v.contains("SOURCE_BODY_CANARY")&&!v.contains("SECRET_TICKET")));
 let config=if claude{
  assert_eq!(get("--permission-mode"),"dontAsk");assert_eq!(get("--setting-sources"),"");
  assert!(args.iter().any(|v|v=="--strict-mcp-config"));assert!(args.iter().any(|v|v=="--disable-slash-commands"));assert!(!args.iter().any(|v|v=="--bare"));
  let settings=fs::read_to_string(get("--settings")).unwrap();assert!(settings.contains("\"disableAllHooks\":true"));
  fs::read_to_string(get("--mcp-config")).unwrap()
 }else{
  assert_eq!(get("--extensions"),"none");
  let settings=fs::read_to_string(env::var("GEMINI_CLI_SYSTEM_SETTINGS_PATH").unwrap()).unwrap();
  assert!(settings.contains("\"hooksConfig\":{\"enabled\":false}"));assert!(settings.contains("\"serverCommand\":\"\""));settings
 };
 let planner=if claude{args.iter().any(|v|v=="--json-schema")}else{config.contains("\"core\":[]")};
 if planner&&claude{assert_eq!(get("--tools"),"");}
 if !planner&&mode.contains("memory"){assert!(config.contains("--memory-mcp"));assert!(config.contains("PRIVATE_TICKET_PATH"));}
 fs::write(exe.with_extension("config"),&config).unwrap();
 let mut input=String::new();io::stdin().read_to_string(&mut input).unwrap();assert!(input.contains("SOURCE_BODY_CANARY"));
 assert!(!input.contains("PRIVATE_TICKET_PATH"));assert!(!input.contains("SECRET_TICKET"));
 fs::write(exe.with_extension("started"),std::process::id().to_string()).unwrap();
 if mode.contains("malformed"){println!("SECRET_NOT_JSON");flush();return}
 if mode.contains("huge"){println!("{}","x".repeat(300000));flush();return}
 let id=if mode.contains("foreign"){String::from("foreign-session")}else{session.clone()};
 if claude{println!("{{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"{id}\",\"mcp_servers\":[{{\"name\":\"patchfleet_memory\",\"status\":\"{}\"}}]}}",if mode.contains("broken-memory"){"failed"}else{"connected"});}
 else{println!("{{\"type\":\"init\",\"session_id\":\"{id}\",\"model\":\"fixture-only\"}}");}
 flush();
 if mode.contains("hang")&&!mode.contains("terminal-hang"){
  let child=Command::new(&exe).arg("--child").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();fs::write(exe.with_extension("child"),child.id().to_string()).unwrap();thread::sleep(Duration::from_secs(20));return
 }
 if mode.contains("tool")&&planner{if claude{println!("{}",r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash"}]}}"#)}else{println!("{}",r#"{"type":"tool_use","tool_name":"run_shell_command"}"#)}flush();thread::sleep(Duration::from_secs(10));return}
 if mode.contains("incomplete"){return}
 if !planner{fs::write("answer.txt","actual fixture output").unwrap();}
 if claude{
  println!("{{\"type\":\"result\",\"session_id\":\"{session}\",\"subtype\":\"success\",\"is_error\":false,\"structured_output\":{{\"plan\":\"fixture\"}},\"permission_denials\":{},\"usage\":{{\"input_tokens\":13,\"output_tokens\":7}},\"total_cost_usd\":0.012345}}",if mode.contains("denied"){r#"[{"tool_name":"Write","private":"SECRET_DENIAL"}]"#}else{"[]"});
 }else{
  if planner{println!("{}",r#"{"type":"message","role":"assistant","content":"{\"plan\":\"fixture\"}"}"#);}
  println!("{}",if mode.contains("failed"){r#"{"type":"result","status":"error","private":"SECRET_FAILURE"}"#}else if mode.contains("no-usage"){r#"{"type":"result","status":"success"}"#}else{r#"{"type":"result","status":"success","stats":{"input_tokens":13,"output_tokens":7}}"#});
 }
 flush();
 if mode.contains("terminal-hang"){thread::sleep(Duration::from_secs(20));}
 if mode.contains("after-terminal"){println!("{}",r#"{"type":"message","role":"assistant","content":"SECRET_TRAILING"}"#);flush();}
 if mode.contains("bad-exit"){eprintln!("SECRET_ERROR");std::process::exit(7)}
}
"###;

fn peer(provider: &str, mode: &str) -> (tempfile::TempDir, Programs) {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    let built = BUILT.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap().keep();
        let source = dir.join("peer.rs");
        std::fs::write(&source, FIXTURE).unwrap();
        let executable = dir.join(if cfg!(windows) { "peer.exe" } else { "peer" });
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| {
            PathBuf::from(env!("CARGO"))
                .with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" })
                .into_os_string()
        });
        assert!(std::process::Command::new(rustc)
            .args(["--edition=2021"])
            .arg(source)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap()
            .success());
        executable
    });
    let dir = tempfile::tempdir().unwrap();
    let executable = dir.path().join(format!(
        "{provider}-{mode}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::copy(built, &executable).unwrap();
    (
        dir,
        Programs {
            codex: "unused".into(),
            claude: executable.clone(),
            gemini: executable,
            timeout: Duration::from_secs(2),
        },
    )
}
fn work(provider: &str, directory: &Path) -> (Value, Value) {
    (
        json!({"providerId":provider,"workItemId":uuid::Uuid::new_v4(),"workingDirectory":directory,"instruction":"SOURCE_BODY_CANARY; $(touch UNSAFE) `echo private`"}),
        json!({"providerId":provider,"runId":format!("run:{}",uuid::Uuid::new_v4())}),
    )
}
fn memory() -> MemoryConfig {
    MemoryConfig {
        config: json!({"mcp_servers":{"patchfleet_memory":{"command":"scoped-mcp-fixture","args":["--memory-mcp","PRIVATE_TICKET_PATH"],"enabled_tools":crate::agent_memory::TOOLS}}}),
        active: Arc::new(AtomicBool::new(true)),
    }
}
async fn wait_file(path: &Path) {
    timeout(Duration::from_secs(5), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
async fn terminal(controller: &Controller) -> Value {
    timeout(Duration::from_secs(5), async {
        loop {
            let signals = controller.drain_signals().await;
            if let Some(value) = signals.into_iter().next() {
                break value;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn both_cli_providers_prepare_start_complete_with_actual_usage_and_scoped_config() {
    for provider in ["claude", "gemini"] {
        let (_fixture, programs) = peer(provider, "memory");
        let controller = Controller::with_programs(programs.clone());
        let directory = tempfile::tempdir().unwrap();
        let path = std::fs::canonicalize(directory.path()).unwrap();
        let (work, mut run) = work(provider, &path);
        let memory = memory();
        let active = memory.active.clone();
        let session = controller
            .prepare_managed_with_memory(&work, &run, &path, Some(memory))
            .await
            .unwrap();
        assert!(
            !programs.claude.with_extension("started").exists(),
            "prepare must not send goal"
        );
        let (id, turn) = controller
            .start_prepared_managed(&work, &run, &path, &session)
            .await
            .unwrap();
        run["providerSessionId"] = json!(id);
        run["providerTurnId"] = json!(turn);
        assert!(controller
            .start_prepared_managed(&work, &run, &path, &session)
            .await
            .is_err());
        let signal = terminal(&controller).await;
        assert_eq!(signal["status"], "completed");
        assert_eq!(signal["providerId"], provider);
        assert_eq!(signal["providerSessionId"], session);
        assert!(!active.load(Ordering::Acquire));
        assert_eq!(
            std::fs::read_to_string(path.join("answer.txt")).unwrap(),
            "actual fixture output"
        );
        let usage = controller.drain_usage().await;
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0]["inputTokens"], 13);
        assert_eq!(usage[0]["outputTokens"], 7);
        assert_eq!(usage[0]["runId"], run["runId"]);
        if provider == "claude" {
            assert_eq!(usage[0]["costMicros"], 12345);
            assert_eq!(usage[0]["currency"], "USD");
        } else {
            assert!(usage[0].get("costMicros").is_none());
        }
        assert!(!signal.to_string().contains("SECRET"));
        assert!(controller.drain_usage().await.is_empty());
        controller.shutdown().await;
    }
}

#[tokio::test]
async fn malformed_foreign_incomplete_denied_and_failed_processes_never_claim_success() {
    for provider in ["claude", "gemini"] {
        let modes = if provider == "claude" {
            vec![
                "malformed",
                "foreign",
                "incomplete",
                "huge",
                "denied",
                "after-terminal",
                "bad-exit",
                "broken-memory",
            ]
        } else {
            vec![
                "malformed",
                "foreign",
                "incomplete",
                "huge",
                "failed",
                "after-terminal",
                "bad-exit",
            ]
        };
        for mode in modes {
            let (_fixture, programs) = peer(provider, mode);
            let controller = Controller::with_programs(programs);
            let dir = tempfile::tempdir().unwrap();
            let path = std::fs::canonicalize(dir.path()).unwrap();
            let (work, run) = work(provider, &path);
            let memory = memory();
            let active = memory.active.clone();
            let session = controller
                .prepare_managed_with_memory(&work, &run, &path, Some(memory))
                .await
                .unwrap();
            controller
                .start_prepared_managed(&work, &run, &path, &session)
                .await
                .unwrap();
            let signal = terminal(&controller).await;
            assert_eq!(signal["status"], "failed", "{provider}/{mode}");
            assert!(!active.load(Ordering::Acquire));
            assert!(controller.drain_usage().await.is_empty());
            assert!(!signal.to_string().contains("SECRET"));
            controller.shutdown().await;
        }
    }
}

#[tokio::test]
async fn foreign_start_cancel_and_revoked_lease_do_not_dispatch() {
    for provider in ["claude", "gemini"] {
        let (_fixture, programs) = peer(provider, "memory");
        let controller = Controller::with_programs(programs.clone());
        let dir = tempfile::tempdir().unwrap();
        let path = std::fs::canonicalize(dir.path()).unwrap();
        let (work, run) = work(provider, &path);
        let memory = memory();
        let active = memory.active.clone();
        let session = controller
            .prepare_managed_with_memory(&work, &run, &path, Some(memory))
            .await
            .unwrap();
        let mut foreign = run.clone();
        foreign["runId"] = json!(uuid::Uuid::new_v4());
        assert!(controller
            .start_prepared_managed(&work, &foreign, &path, &session)
            .await
            .is_err());
        assert!(controller.cancel(&foreign).await.is_err());
        active.store(false, Ordering::Release);
        assert_eq!(
            controller
                .start_prepared_managed(&work, &run, &path, &session)
                .await
                .unwrap_err(),
            "MEMORY_CONNECTION_FAILED"
        );
        assert!(!programs.claude.with_extension("started").exists());
        controller.shutdown().await;
    }
}

#[tokio::test]
async fn cancel_and_terminal_revoke_before_host_poll_and_reap_owned_process() {
    for provider in ["claude", "gemini"] {
        for mode in ["memory-hang", "memory-terminal-hang"] {
            let (_fixture, programs) = peer(provider, mode);
            let controller = Controller::with_programs(programs.clone());
            let dir = tempfile::tempdir().unwrap();
            let path = std::fs::canonicalize(dir.path()).unwrap();
            let (work, mut run) = work(provider, &path);
            let memory = memory();
            let active = memory.active.clone();
            let session = controller
                .prepare_managed_with_memory(&work, &run, &path, Some(memory))
                .await
                .unwrap();
            let (id, turn) = controller
                .start_prepared_managed(&work, &run, &path, &session)
                .await
                .unwrap();
            run["providerSessionId"] = json!(id);
            run["providerTurnId"] = json!(turn);
            wait_file(&programs.claude.with_extension("started")).await;
            if mode.contains("terminal") {
                timeout(Duration::from_secs(2), async {
                    while active.load(Ordering::Acquire) {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                })
                .await
                .unwrap();
            } else {
                wait_file(&programs.claude.with_extension("child")).await;
            }
            let began = std::time::Instant::now();
            controller.cancel(&run).await.unwrap();
            assert!(began.elapsed() < Duration::from_secs(2));
            assert!(!active.load(Ordering::Acquire));
            assert_eq!(terminal(&controller).await["status"], "interrupted");
            #[cfg(unix)]
            {
                let pid: i32 = std::fs::read_to_string(programs.claude.with_extension("started"))
                    .unwrap()
                    .parse()
                    .unwrap();
                assert_ne!(unsafe { libc::kill(pid, 0) }, 0);
            }
            controller.shutdown().await;
        }
    }
}

#[tokio::test]
async fn selected_provider_structured_generation_is_tool_free_bounded_and_cancellable() {
    for provider in ["claude", "gemini"] {
        for mode in [
            "valid",
            "malformed",
            "foreign",
            "incomplete",
            "tool",
            "old",
            "bad-version",
            "hang",
        ] {
            let (_fixture, programs) = peer(provider, mode);
            let controller = Arc::new(Controller::with_programs(programs.clone()));
            let scratch = tempfile::tempdir().unwrap();
            let active = Arc::new(AtomicBool::new(true));
            let result = if mode == "hang" {
                let (c, path, flag) = (
                    controller.clone(),
                    scratch.path().to_owned(),
                    active.clone(),
                );
                let id = provider.to_owned();
                let task = tokio::spawn(async move {
                    c.generate_structured(
                        &id,
                        &path,
                        "SOURCE_BODY_CANARY",
                        &json!({"type":"object"}),
                        flag,
                    )
                    .await
                });
                wait_file(&programs.claude.with_extension("child")).await;
                active.store(false, Ordering::Release);
                task.await.unwrap()
            } else {
                controller
                    .generate_structured(
                        provider,
                        scratch.path(),
                        "SOURCE_BODY_CANARY",
                        &json!({"type":"object"}),
                        active,
                    )
                    .await
            };
            if mode == "valid" {
                assert_eq!(result.unwrap(), json!({"plan":"fixture"}));
            } else {
                let error = result.unwrap_err();
                assert!(
                    error.starts_with("EXTRACTION_"),
                    "{provider}/{mode}: {error}"
                );
                assert!(!error.contains("SECRET"));
            }
            assert!(!scratch.path().join("answer.txt").exists());
            controller.shutdown().await;
        }
    }
}

#[tokio::test]
async fn diagnostic_support_and_unknown_usage_are_honest() {
    let (_fixture, mut programs) = peer("claude", "valid");
    let (_gemini, gemini) = peer("gemini", "no-usage");
    programs.gemini = gemini.gemini;
    let controller = Controller::with_programs(programs);
    let connections = controller.connection_status().await;
    assert_eq!(connections["providers"][1]["controlSupported"], true);
    assert_eq!(connections["providers"][1]["authentication"], "present");
    assert_eq!(connections["providers"][2]["controlSupported"], true);
    assert_eq!(connections["providers"][2]["authentication"], "unknown");
    assert_eq!(
        connections["providers"][2]["toolReadiness"],
        "configured_unverified"
    );
    assert!(!connections.to_string().contains("SECRET"));
    let dir = tempfile::tempdir().unwrap();
    let path = std::fs::canonicalize(dir.path()).unwrap();
    let (work, run) = work("gemini", &path);
    let session = controller
        .prepare_managed_with_memory(&work, &run, &path, None)
        .await
        .unwrap();
    controller
        .start_prepared_managed(&work, &run, &path, &session)
        .await
        .unwrap();
    assert_eq!(terminal(&controller).await["status"], "completed");
    assert!(controller.drain_usage().await.is_empty());
    controller.shutdown().await;
}

#[tokio::test]
async fn generation_input_and_capability_boundaries_fail_before_dispatch() {
    for provider in ["claude", "gemini"] {
        let (_fixture, programs) = peer(provider, "valid");
        let controller = Controller::with_programs(programs.clone());
        let scratch = tempfile::tempdir().unwrap();
        for (prompt, schema) in [
            (String::new(), json!({"type":"object"})),
            ("x".repeat(LIMIT + 1), json!({"type":"object"})),
            ("SOURCE_BODY_CANARY".into(), json!([])),
        ] {
            assert_eq!(
                controller
                    .generate_structured(
                        provider,
                        scratch.path(),
                        &prompt,
                        &schema,
                        Arc::new(AtomicBool::new(true))
                    )
                    .await
                    .unwrap_err(),
                "EXTRACTION_INVALID_INPUT"
            );
        }
        assert_eq!(
            controller
                .generate_structured(
                    provider,
                    scratch.path(),
                    "SOURCE_BODY_CANARY",
                    &json!({"type":"object"}),
                    Arc::new(AtomicBool::new(false))
                )
                .await
                .unwrap_err(),
            "EXTRACTION_CANCELLED"
        );
        assert!(!programs.claude.with_extension("started").exists());
    }
    let available = json!({"provider":{"id":"gemini","version":"0.43.0","state":"degraded","error":{"code":"GEMINI_HOOK_SETUP_REQUIRED"}}});
    assert!(supports_control(&available));
    assert!(!supports_codex_control(&available));
    let mut failed = available.clone();
    failed["provider"]["error"]["code"] = json!("GEMINI_PROBE_FAILED");
    assert!(!supports_control(&failed));
    failed = available;
    failed["provider"]["version"] = json!("0.42.0");
    assert!(!supports_control(&failed));
}

#[cfg(unix)]
#[tokio::test]
async fn failed_managed_revision_retry_keeps_provider_schema_and_frozen_inputs() {
    use sha2::{Digest, Sha256};

    fn owner(mut value: Value) -> Value {
        value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
        let now = chrono::Utc::now();
        value["createdAt"] = json!(now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        value["expiresAt"] = json!((now + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        value
    }

    for provider in ["claude", "gemini"] {
        let (_fixture, programs) = peer(provider, "bad-exit");
        let temporary = tempfile::tempdir().unwrap();
        let data = temporary.path().join("data");
        let host =
            crate::Host::open_with_controller(data.clone(), Controller::with_programs(programs))
                .unwrap();
        host.store
            .append(vec![crate::event(
                "provider.observed",
                json!({
                    "providerId":provider,"state":"available",
                    "version":if provider == "claude" {"2.1.170"} else {"0.43.0"},
                    "capabilities":{"recentObservation":true,"explicitLiveStatus":true},
                    "observedAt":crate::now(),"sessionIds":[]
                }),
            )])
            .unwrap();
        let project = host
            .action("project_create", owner(json!({"name":"Revision retry"})))
            .await
            .unwrap()["projectId"]
            .clone();
        let original = host
            .action(
                "project_work_enqueue",
                owner(json!({
                    "projectId":project,"expectedProjectRevision":1,"abilityId":"D1",
                    "title":"Original document","instruction":"Prepare the source document"
                })),
            )
            .await
            .unwrap();
        let (source_root, _) = host
            .creation_scope(&json!({
                "projectId":project,"workItemId":original["workItemId"]
            }))
            .unwrap();
        let source = b"The exact frozen reference for this revision.";
        std::fs::write(source_root.join("outputs/source.md"), source).unwrap();
        let revision = host.snapshot().unwrap()["projectMemory"]["projects"][0]["revision"].clone();
        let queued = host.action("project_artifact_revise", owner(json!({
            "projectId":project,"expectedProjectRevision":revision,
            "workItemId":original["workItemId"],"path":"outputs/source.md",
            "expectedSha256":format!("{:x}",Sha256::digest(source)),
            "instruction":"SOURCE_BODY_CANARY: make this document shorter","providerId":provider
        }))).await.unwrap();
        assert_eq!(queued["outcome"], "applied", "{queued}");
        let (revision_root, item) = host
            .creation_scope(&json!({
                "projectId":project,"workItemId":queued["workItemId"]
            }))
            .unwrap();
        assert_eq!(item["schemaVersion"], 13);
        assert_eq!(item["providerId"], provider);
        let manifest = std::fs::read(revision_root.join("inputs/manifest.json")).unwrap();
        let started = host
            .action(
                "work_start",
                owner(json!({
                    "workItemId":queued["workItemId"],"expectedItemRevision":item["revision"]
                })),
            )
            .await
            .unwrap();
        assert_eq!(started["outcome"], "applied", "{started}");
        let failed = timeout(Duration::from_secs(5), async {
            loop {
                host.background_tick(false).await.unwrap();
                let (_, item) = host
                    .creation_scope(&json!({
                        "projectId":project,"workItemId":queued["workItemId"]
                    }))
                    .unwrap();
                if item["status"] == "failed" {
                    break item;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let state = host.snapshot().unwrap();
        let run = state["work"]["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["workItemId"] == queued["workItemId"])
            .unwrap();
        assert_eq!(run["schemaVersion"], 13);
        assert_eq!(run["providerId"], provider);
        assert_eq!(run["status"], "failed");
        // Retry must use the first revision's durable manifest, not reread a changed parent.
        std::fs::write(
            source_root.join("outputs/source.md"),
            b"Changed parent output",
        )
        .unwrap();
        let request = owner(json!({
            "workItemId":queued["workItemId"],"expectedItemRevision":failed["revision"]
        }));
        let retried = host.action("work_retry", request.clone()).await.unwrap();
        assert_eq!(retried["outcome"], "applied", "{retried}");
        let (retry_root, retry) = host
            .creation_scope(&json!({
                "projectId":project,"workItemId":retried["workItemId"]
            }))
            .unwrap();
        assert_eq!(retry["schemaVersion"], 13);
        assert_eq!(retry["providerId"], provider);
        assert_eq!(retry["instruction"], failed["instruction"]);
        assert_eq!(retry["status"], "queued");
        assert_eq!(
            std::fs::read(retry_root.join("inputs/reference-01.md")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read(retry_root.join("inputs/manifest.json")).unwrap(),
            manifest
        );
        let conversation = host
            .action("project_chat_read", json!({"projectId":project}))
            .await
            .unwrap();
        for id in [&queued["workItemId"], &retried["workItemId"]] {
            assert!(conversation["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| &item["workItemId"] == id));
        }
        let snapshot = host.snapshot().unwrap();
        for id in [&queued["workItemId"], &retried["workItemId"]] {
            let local_job = snapshot["projectJobs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|job| &job["workItemId"] == id)
                .unwrap();
            assert_eq!(local_job["abilityId"], "W1");
            assert!(local_job.get("instruction").is_none());
        }
        // A separately requested retry cannot accept modified frozen bytes.
        std::fs::write(
            revision_root.join("inputs/reference-01.md"),
            b"Tampered frozen input",
        )
        .unwrap();
        let rejected = host
            .action(
                "work_retry",
                owner(json!({
                    "workItemId":queued["workItemId"],"expectedItemRevision":failed["revision"]
                })),
            )
            .await
            .unwrap_err();
        assert_eq!(rejected.0, "PROJECT_INPUT_CHANGED");
        host.shutdown().await;
        drop(host);
        let recovered = crate::Host::open(data).unwrap();
        assert_eq!(
            recovered.action("work_retry", request).await.unwrap(),
            retried
        );
        recovered.shutdown().await;
    }
}
