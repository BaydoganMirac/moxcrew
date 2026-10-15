use patchfleet_core::providers::{Controller, Programs};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    time::Duration,
};

const FIXTURE: &str = r###"
use std::{env,fs,io::{self,BufRead,Read,Write},process::{Command,Stdio},thread,time::Duration};
fn flush(){io::stdout().flush().unwrap();}
fn message(text:&str){println!("{{\"type\":\"item.completed\",\"item\":{{\"id\":\"msg-1\",\"type\":\"agent_message\",\"text\":{text:?}}}}}");flush();}
fn token(line:&str,key:&str)->String{
 let start=line.find(&format!("\"{key}\":" )).unwrap()+key.len()+3;let s=&line[start..];
 if s.starts_with('"'){let mut escape=false;for(i,c)in s.char_indices().skip(1){if c=='"'&&!escape{return s[..=i].into()}escape=c=='\\'&&!escape;}}
 s.split([',','}']).next().unwrap().into()
}
fn send(id:&str,value:&str){println!("{{\"id\":{id},\"result\":{value}}}");flush();}
fn hang(exe:&std::path::Path){
 fs::write(exe.with_extension("pid"),std::process::id().to_string()).unwrap();
 let child=Command::new(exe).arg("--child").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
 fs::write(exe.with_extension("child"),child.id().to_string()).unwrap();
 thread::sleep(Duration::from_secs(10));
}
fn main(){
 let exe=env::current_exe().unwrap();let mode=exe.file_stem().unwrap().to_str().unwrap();let args:Vec<_>=env::args().collect();
 if args.get(1).map(String::as_str)==Some("--child"){thread::sleep(Duration::from_secs(10));return}
 if args.get(1).map(String::as_str)==Some("--version"){
  if mode=="probe-hang"{hang(&exe);return}
  if mode=="old"{println!("codex-cli 0.144.1")}else if mode=="bad-version"{println!("SECRET_PRIVATE_VERSION")}else{println!("codex-cli 0.155.1")};return
 }
 if args.get(1).map(String::as_str)==Some("app-server"){
  for line in io::stdin().lock().lines(){let line=line.unwrap();if !line.contains("\"id\":"){continue}let id=token(&line,"id");
   if line.contains("\"thread/start\""){let cwd=token(&line,"cwd");let source=token(&line,"threadSource");send(&id,&format!("{{\"thread\":{{\"id\":\"owned-thread\",\"cwd\":{cwd},\"threadSource\":{source},\"ephemeral\":false}}}}"));}else{send(&id,"{}")}
  }return
 }
 assert_eq!(args[1],"exec");assert_eq!(args.last().unwrap(),"-");
 let has=|flag:&str,value:&str|args.windows(2).any(|v|v[0]==flag&&v[1]==value);
 for flag in ["--json","--strict-config","--ephemeral","--skip-git-repo-check","--ignore-user-config","--ignore-rules"]{assert!(args.iter().any(|v|v==flag));}
 for(flag,value)in[("--sandbox","read-only"),("--output-schema","schema.json"),("--cd","."),("--color","never"),("-c","approval_policy=never"),("-c","web_search=disabled"),("-c","mcp_servers={}"),("-c","project_doc_max_bytes=0"),("--enable","skip_host_skill_discovery")]{assert!(has(flag,value));}
 for feature in ["shell_tool","unified_exec","apps","plugins","hooks","multi_agent","multi_agent_v2","memories","view_image","browser_use","computer_use","image_generation","skill_search","workspace_dependencies","code_mode_host","code_mode","shell_snapshot","goals","sleep_tool","tool_suggest"]{assert!(has("--disable",feature));}
 assert!(args.iter().skip(1).all(|v|!v.contains("SOURCE_BODY_CANARY")&&!std::path::Path::new(v).is_absolute()));
 let cwd=env::current_dir().unwrap();let schema=fs::read_to_string(cwd.join("schema.json")).unwrap();assert!(schema.contains("\"type\":\"object\""));assert!(!cwd.join(".git").exists());
 let mut input=String::new();io::stdin().read_to_string(&mut input).unwrap();assert!(input.contains("SOURCE_BODY_CANARY"));
 fs::write(exe.with_extension("started"),"argv-and-stdin-verified").unwrap();
 if mode=="hang"{hang(&exe);return}
 if mode=="exit"{eprintln!("SECRET_PROVIDER_ERROR");std::process::exit(17)}
 if mode=="failed"{println!("{}",r#"{"type":"turn.failed","error":{"message":"SECRET_PROVIDER_ERROR"}}"#);flush();return}
 if mode=="malformed"{println!("SECRET_NOT_JSON");flush();return}
 if mode=="unknown"{println!("{}",r#"{"type":"surprise"}"#);flush();return}
 if mode=="bad-thread"{println!("{}",r#"{"type":"thread.started","thread_id":3}"#);flush();return}
 if mode=="huge-line"{println!("{}","x".repeat(256*1024+1));flush();return}
 if mode=="stderr"{eprint!("{}","SECRET_STDERR".repeat(100000));io::stderr().flush().unwrap();return}
 if mode=="huge-total"{for _ in 0..100{println!("{{\"type\":\"item.completed\",\"item\":{{\"id\":\"reason\",\"type\":\"reasoning\",\"text\":\"{}\"}}}}","x".repeat(16384));flush();}return}
 if mode.starts_with("tool-"){
  fs::write(exe.with_extension("pid"),std::process::id().to_string()).unwrap();
  println!("{{\"type\":\"item.started\",\"item\":{{\"id\":\"tool-1\",\"type\":\"{}\"}}}}",&mode[5..]);flush();
  thread::sleep(Duration::from_secs(10));fs::write(exe.with_extension("side-effect"),"should-be-killed").unwrap();return
 }
 println!("{}",r#"{"type":"thread.started","thread_id":"thread-1"}"#);
 println!("{}",r#"{"type":"turn.started"}"#);
 println!("{}",r#"{"type":"item.completed","item":{"id":"reason-1","type":"reasoning","text":"SECRET_REASONING"}}"#);
 if mode=="progress"{message("I will inspect the source.");}
 if mode=="array"{message("[]")}else if mode=="huge-final"{message(&format!("{{\"x\":\"{}\"}}","x".repeat(128*1024)))}else if mode=="empty"{message(r#"{"proposals":[]}"#)}else{message(r#"{"proposals":[{"title":"Türkçe karar / English decision"}]}"#)}
 if mode!="incomplete"{println!("{}",r#"{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}"#);flush();}
 if mode=="after-terminal"{message(r#"{"proposals":[]}"#);}
}
"###;

fn programs(mode: &str) -> (tempfile::TempDir, Programs) {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    let built = BUILT.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap().keep();
        let source = directory.join("extraction_peer.rs");
        std::fs::write(&source, FIXTURE).unwrap();
        let executable = directory.join(if cfg!(windows) { "peer.exe" } else { "peer" });
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| {
            PathBuf::from(env!("CARGO"))
                .with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" })
                .into_os_string()
        });
        assert!(Command::new(rustc)
            .arg("--edition=2021")
            .arg(source)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap()
            .success());
        executable
    });
    let directory = tempfile::tempdir().unwrap();
    let executable = directory
        .path()
        .join(format!("{mode}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::copy(built, &executable).unwrap();
    (
        directory,
        Programs {
            codex: executable,
            claude: "unused".into(),
            gemini: "unused".into(),
            timeout: Duration::from_secs(2),
        },
    )
}
fn schema() -> Value {
    json!({"type":"object","properties":{"proposals":{"type":"array"}}})
}
const PROMPT: &str = "SOURCE_BODY_CANARY Türkçe / English $(touch UNSAFE) `echo UNSAFE`";

#[tokio::test]
async fn structured_messages_use_isolated_read_only_stdin_without_native_output_leakage() {
    for mode in ["valid", "progress", "empty"] {
        let (_fixture, p) = programs(mode);
        let c = Controller::with_programs(p.clone());
        let scratch = tempfile::tempdir().unwrap();
        let active = Arc::new(AtomicBool::new(true));
        let result = c
            .extract_decisions(scratch.path(), PROMPT, &schema(), active.clone())
            .await
            .unwrap();
        assert!(
            active.load(Ordering::Acquire),
            "success must not look cancelled to the Host"
        );
        assert!(p.codex.with_extension("started").exists());
        if mode == "empty" {
            assert_eq!(result, json!({"proposals":[]}));
        } else {
            assert_eq!(
                result["proposals"][0]["title"],
                "Türkçe karar / English decision"
            );
        }
        assert!(!result.to_string().contains("SECRET"));
        assert!(!result.to_string().contains("SOURCE_BODY_CANARY"));
        let entries: Vec<_> = std::fs::read_dir(scratch.path())
            .unwrap()
            .map(|v| v.unwrap().file_name())
            .collect();
        assert_eq!(entries, ["schema.json"]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(scratch.path().join("schema.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(scratch.path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }
}

#[tokio::test]
async fn malformed_oversize_or_side_effect_output_is_killed_and_safely_normalized() {
    for mode in [
        "malformed",
        "unknown",
        "bad-thread",
        "array",
        "incomplete",
        "after-terminal",
        "huge-line",
        "huge-total",
        "huge-final",
        "stderr",
        "tool-command_execution",
        "tool-file_change",
        "tool-mcp_tool_call",
        "tool-web_search",
        "tool-unknown",
    ] {
        let (_fixture, p) = programs(mode);
        let c = Controller::with_programs(p.clone());
        let scratch = tempfile::tempdir().unwrap();
        let error = c
            .extract_decisions(
                scratch.path(),
                PROMPT,
                &schema(),
                Arc::new(AtomicBool::new(true)),
            )
            .await
            .unwrap_err();
        assert_eq!(error, "EXTRACTION_INVALID_OUTPUT", "fixture {mode}");
        assert!(!p.codex.with_extension("side-effect").exists());
    }
}

#[tokio::test]
async fn provider_failures_and_input_bounds_never_return_private_diagnostics() {
    for mode in ["old", "bad-version", "exit", "failed", "missing"] {
        let (_fixture, mut p) = programs(mode);
        if mode == "missing" {
            p.codex = p.codex.with_file_name("missing-codex-extraction-peer");
        }
        let c = Controller::with_programs(p);
        let scratch = tempfile::tempdir().unwrap();
        let error = c
            .extract_decisions(
                scratch.path(),
                PROMPT,
                &schema(),
                Arc::new(AtomicBool::new(true)),
            )
            .await
            .unwrap_err();
        assert_eq!(
            error,
            match mode {
                "old" | "bad-version" => "EXTRACTION_PROVIDER_UNSUPPORTED",
                "missing" => "EXTRACTION_PROVIDER_UNAVAILABLE",
                _ => "EXTRACTION_PROVIDER_FAILED",
            }
        );
    }
    let (_fixture, p) = programs("valid");
    let c = Controller::with_programs(p.clone());
    let scratch = tempfile::tempdir().unwrap();
    for (prompt, output_schema) in [
        ("".to_owned(), schema()),
        ("x".repeat(256 * 1024 + 1), schema()),
        (PROMPT.into(), json!([])),
    ] {
        assert_eq!(
            c.extract_decisions(
                scratch.path(),
                &prompt,
                &output_schema,
                Arc::new(AtomicBool::new(true))
            )
            .await
            .unwrap_err(),
            "EXTRACTION_INVALID_INPUT"
        );
    }
    assert!(!p.codex.with_extension("started").exists());
    assert!(std::fs::read_dir(scratch.path()).unwrap().next().is_none());
    assert_eq!(
        c.extract_decisions(
            scratch.path(),
            PROMPT,
            &schema(),
            Arc::new(AtomicBool::new(false))
        )
        .await
        .unwrap_err(),
        "EXTRACTION_CANCELLED"
    );
    assert!(!p.codex.with_extension("started").exists());
}

async fn wait_for_file(path: &Path) {
    tokio::time::timeout(Duration::from_secs(4), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("native fixture must reach its process-tree wait");
}
#[cfg(unix)]
async fn wait_reaped(path: &Path) {
    let pid: i32 = std::fs::read_to_string(path).unwrap().parse().unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while unsafe { libc::kill(pid, 0) } == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("cancelled native process tree must be killed and reaped");
}

#[tokio::test]
async fn cancellation_and_aborted_callers_reap_the_tree_without_locking_owned_work() {
    for mode in ["hang", "probe-hang"] {
        let (_fixture, p) = programs(mode);
        let c = Arc::new(Controller::with_programs(p.clone()));
        let scratch = tempfile::tempdir().unwrap();
        let active = Arc::new(AtomicBool::new(true));
        let task = {
            let (c, active, path) = (c.clone(), active.clone(), scratch.path().to_owned());
            tokio::spawn(async move { c.extract_decisions(&path, PROMPT, &schema(), active).await })
        };
        wait_for_file(&p.codex.with_extension("child")).await;
        if mode == "hang" {
            let repo = tempfile::tempdir().unwrap();
            std::fs::create_dir(repo.path().join(".git")).unwrap();
            tokio::time::timeout(
                Duration::from_secs(1),
                c.prepare(
                    &json!({"workingDirectory":repo.path()}),
                    &json!({"intentId":"intent:parallel"}),
                ),
            )
            .await
            .unwrap()
            .unwrap();
        }
        let cancelled = std::time::Instant::now();
        active.store(false, Ordering::Release);
        assert_eq!(task.await.unwrap().unwrap_err(), "EXTRACTION_CANCELLED");
        assert!(cancelled.elapsed() < Duration::from_secs(1));
        #[cfg(unix)]
        {
            wait_reaped(&p.codex.with_extension("pid")).await;
            wait_reaped(&p.codex.with_extension("child")).await;
        }
        c.shutdown().await;
    }
    let (_fixture, p) = programs("hang");
    let c = Arc::new(Controller::with_programs(p.clone()));
    let scratch = tempfile::tempdir().unwrap();
    let active = Arc::new(AtomicBool::new(true));
    let task = {
        let (c, active, path) = (c.clone(), active.clone(), scratch.path().to_owned());
        tokio::spawn(async move { c.extract_decisions(&path, PROMPT, &schema(), active).await })
    };
    wait_for_file(&p.codex.with_extension("child")).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(!active.load(Ordering::Acquire));
    #[cfg(unix)]
    {
        wait_reaped(&p.codex.with_extension("pid")).await;
        wait_reaped(&p.codex.with_extension("child")).await;
    }
    c.shutdown().await;
}
