use patchfleet_core::providers::{Controller, MemoryConfig, Programs};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    time::Duration,
};

// A native protocol peer, never a model or a user's provider configuration.
const FIXTURE: &str = r###"
use std::{env,fs,io::{self,BufRead,Write},thread,time::Duration};
fn token(line:&str,key:&str)->String {
 let start=line.find(&format!("\"{key}\":" )).unwrap()+key.len()+3;
 let s=&line[start..];
 if s.starts_with('"') { let mut escape=false; for(i,c)in s.char_indices().skip(1){if c=='"'&&!escape{return s[..=i].into()}escape=c=='\\'&&!escape;} }
 s.split([',','}']).next().unwrap().into()
}
fn send(id:&str,result:&str){println!("{{\"id\":{id},\"result\":{result}}}");io::stdout().flush().unwrap();}
fn main(){
 let exe=env::current_exe().unwrap();let mode=exe.file_stem().unwrap().to_str().unwrap();
 if env::args().nth(1).as_deref()==Some("--version"){println!("codex-cli {}",if mode.contains("old"){"0.144.1"}else{"0.155.1"});return}
 let mut count=0;let mut inventories=0;
 for line in io::stdin().lock().lines(){let line=line.unwrap();if !line.contains("\"id\":"){continue}
  let mut log=fs::OpenOptions::new().create(true).append(true).open(exe.with_extension("requests")).unwrap();writeln!(log,"{line}").unwrap();
  let id=token(&line,"id");
  if line.contains("\"initialize\""){send(&id,"{}");}
  else if line.contains("\"thread/start\""){
   if mode.contains("prepare-hang"){thread::sleep(Duration::from_secs(10));}
   count+=1;let cwd=token(&line,"cwd");let source=token(&line,"threadSource");
   send(&id,&format!("{{\"thread\":{{\"id\":\"owned-{count}\",\"cwd\":{cwd},\"threadSource\":{source},\"ephemeral\":false}}}}"));
  }else if line.contains("\"mcpServerStatus/list\""){
   inventories+=1;
   if mode.contains("inventory-fatal")&&count==2{thread::sleep(Duration::from_millis(200));println!("SECRET_INVALID_JSON");io::stdout().flush().unwrap();return}
   if mode.contains("inventory-hang"){thread::sleep(Duration::from_secs(10));}
   if mode.contains("waiting")&&inventories==1{send(&id,r#"{"data":[{"name":"patchfleet_memory","runtimeStatus":"starting"}],"nextCursor":null}"#);continue}
   if mode.contains("paged")&&!line.contains("page2"){send(&id,r#"{"data":[],"nextCursor":"page2"}"#);continue}
   let extra=if mode.contains("extra"){r#", "write_decision":{"name":"write_decision"}"#}else{""};
   let error=if mode.contains("tool-error"){r#""startup failed""#}else{"null"};
   send(&id,&format!(r#"{{"data":[{{"name":"patchfleet_memory","runtimeStatus":"connected","toolsError":{error},"tools":{{"project_context":{{"name":"project_context"}},"memory_search":{{"name":"memory_search"}},"decision_read":{{"name":"decision_read"}}{extra}}}}}],"nextCursor":null}}"#));
  }else if line.contains("\"mcpServer/tool/call\""){
   assert!(inventories>0);assert!(line.contains("\"project_context\""));assert!(line.contains("\"limit\":20"));
   let error=mode.contains("call-error");let body=if mode.contains("call-malformed"){r#""invalid""#}else{r#"{"schemaVersion":1,"privateContext":"CONTEXT_BODY_CANARY"}"#};
   send(&id,&format!(r#"{{"isError":{error},"structuredContent":{body},"content":[{{"type":"text","text":"CONTEXT_BODY_CANARY"}}]}}"#));
   if mode.contains("prepared-notify"){println!("{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"owned-1\",\"turn\":{{\"id\":\"idle-turn\",\"status\":\"completed\"}}}}}}");io::stdout().flush().unwrap();}
  }else if line.contains("\"turn/start\""){
   assert!(!line.contains("ticket"));assert!(!line.contains("synthetic-bearer"));
   if mode.contains("held"){thread::sleep(Duration::from_millis(200));}
   let idthread=token(&line,"threadId");let status=if mode.contains("immediate"){"completed"}else{"inProgress"};
   if mode.contains("before"){let tid=if mode.contains("foreign"){"foreign-turn"}else{"turn-1"};println!("{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":{idthread},\"turn\":{{\"id\":\"{tid}\",\"status\":\"completed\"}}}}}}");io::stdout().flush().unwrap();}
   send(&id,&format!("{{\"turn\":{{\"id\":\"turn-1\",\"status\":\"{status}\"}}}}"));
   if mode.contains("notify")&&!mode.contains("before"){println!("{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":{idthread},\"turn\":{{\"id\":\"turn-1\",\"status\":\"completed\"}}}}}}");io::stdout().flush().unwrap();}
   if mode=="foreign"{println!("{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"foreign-thread\",\"turn\":{{\"id\":\"turn-1\",\"status\":\"completed\"}}}}}}");println!("{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":{idthread},\"turn\":{{\"id\":\"foreign-turn\",\"status\":\"completed\"}}}}}}");io::stdout().flush().unwrap();}
   if mode=="fatal"{println!("SECRET_INVALID_JSON");io::stdout().flush().unwrap();return}
   if mode.contains("lost"){return}
  }else if line.contains("\"turn/interrupt\""){send(&id,"{}");}
  else{send(&id,"{}");}
 }
}
"###;

fn programs(mode: &str) -> (tempfile::TempDir, Programs) {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    let built = BUILT.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap().keep();
        let source = dir.join("peer.rs");
        std::fs::write(&source, FIXTURE).unwrap();
        let exe = dir.join(if cfg!(windows) { "peer.exe" } else { "peer" });
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| {
            PathBuf::from(env!("CARGO"))
                .with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" })
                .into_os_string()
        });
        assert!(Command::new(rustc)
            .args(["--edition=2021"])
            .arg(source)
            .arg("-o")
            .arg(&exe)
            .status()
            .unwrap()
            .success());
        exe
    });
    let dir = tempfile::tempdir().unwrap();
    let codex = dir
        .path()
        .join(format!("{mode}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::copy(built, &codex).unwrap();
    (
        dir,
        Programs {
            codex,
            claude: "unused".into(),
            gemini: "unused".into(),
            timeout: Duration::from_secs(2),
        },
    )
}
fn work() -> (tempfile::TempDir, Value, Value) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    let work = json!({"workingDirectory":dir.path(),"instruction":"OWNER_INSTRUCTION"});
    (
        dir,
        work,
        json!({"intentId":"intent:test","runId":"run:test"}),
    )
}
fn memory(ticket: &str) -> (MemoryConfig, Arc<AtomicBool>, Value) {
    let active = Arc::new(AtomicBool::new(true));
    let config = json!({"mcp_servers":{"patchfleet_memory":{"command":"patchfleet","args":["--memory-mcp",ticket],"required":true,"enabled_tools":["project_context","memory_search","decision_read"],"startup_timeout_sec":5,"tool_timeout_sec":5}}});
    (
        MemoryConfig {
            config: config.clone(),
            active: active.clone(),
        },
        active,
        config,
    )
}
fn requests(program: &Programs) -> Vec<Value> {
    std::fs::read_to_string(program.codex.with_extension("requests"))
        .unwrap_or_default()
        .lines()
        .map(|v| serde_json::from_str(v).unwrap())
        .collect()
}
async fn wait_revoked(active: &AtomicBool) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while active.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("reader must revoke without observing or draining provider signals");
}

#[tokio::test]
async fn memory_is_thread_scoped_verified_before_turn_and_absent_from_owner_input() {
    let (_peer, p) = programs("waiting-paged");
    let c = Controller::with_programs(p.clone());
    let (_dir, mut w, r) = work();
    let (binding, active, config) = memory("private-ticket-A");
    let a = c.prepare_with_memory(&w, &r, Some(binding)).await.unwrap();
    let b = c
        .prepare(&w, &json!({"intentId":"intent:other"}))
        .await
        .unwrap();
    w["_memoryGuidance"] = json!("Read project_context before using accepted decisions.");
    c.start_prepared(&w, &r, &a).await.unwrap();
    assert_eq!(w["instruction"], "OWNER_INSTRUCTION");
    let wire = requests(&p);
    let starts: Vec<_> = wire
        .iter()
        .filter(|v| v["method"] == "thread/start")
        .collect();
    assert_eq!(starts[0]["params"]["config"], config);
    assert!(starts[1]["params"].get("config").is_none());
    let tool_checks: Vec<_> = wire
        .iter()
        .filter(|v| v["method"] == "mcpServerStatus/list")
        .collect();
    assert!(tool_checks.len() >= 3);
    assert!(tool_checks.iter().all(|v| v["params"]["threadId"] == a));
    let turn = wire.iter().find(|v| v["method"] == "turn/start").unwrap();
    assert_eq!(
        turn["params"]["input"][0]["text"],
        "Read project_context before using accepted decisions.\n\nOWNER_INSTRUCTION"
    );
    assert!(!turn.to_string().contains("ticket"));
    assert!(!turn.to_string().contains("CONTEXT_BODY_CANARY"));
    let context_read = wire
        .iter()
        .position(|v| v["method"] == "mcpServer/tool/call")
        .unwrap();
    let turn_start = wire
        .iter()
        .position(|v| v["method"] == "turn/start")
        .unwrap();
    assert!(context_read < turn_start);
    assert!(active.load(Ordering::Acquire));
    c.cancel(&json!({"providerSessionId":a,"providerTurnId":"turn-1"}))
        .await
        .unwrap();
    assert!(!active.load(Ordering::Acquire));
    assert!(c.start_prepared(&w, &r, &b).await.is_ok());
    c.shutdown().await;
}

#[tokio::test]
async fn unsupported_or_unverified_memory_fails_closed_and_revokes() {
    let (_dir, w, r) = work();
    for mode in [
        "old",
        "extra",
        "tool-error",
        "inventory-hang",
        "call-error",
        "call-malformed",
    ] {
        let (_peer, p) = programs(mode);
        let c = Controller::with_programs(p.clone());
        let (binding, active, _) = memory("private-ticket");
        let error = c
            .prepare_with_memory(&w, &r, Some(binding))
            .await
            .unwrap_err();
        assert_eq!(
            error,
            if mode == "old" {
                "AGENT_MEMORY_UNSUPPORTED"
            } else {
                "MEMORY_CONNECTION_FAILED"
            }
        );
        assert!(!active.load(Ordering::Acquire));
        assert!(!requests(&p).iter().any(|v| v["method"] == "turn/start"));
        if mode == "old" {
            assert!(c.prepare(&w, &r).await.is_ok());
        }
        c.shutdown().await;
    }
}

#[tokio::test]
async fn terminal_loss_shutdown_drop_and_cancel_revoke_only_the_owned_binding() {
    let (_dir, w, r) = work();
    for mode in [
        "immediate",
        "notify",
        "notify-before",
        "lost",
        "fatal",
        "shutdown",
        "drop",
        "cancel",
    ] {
        let (_peer, p) = programs(mode);
        let c = Controller::with_programs(p);
        let (binding, active, _) = memory("private-ticket-A");
        let a = c.prepare_with_memory(&w, &r, Some(binding)).await.unwrap();
        let (other, other_active, _) = memory("private-ticket-B");
        let _b = c
            .prepare_with_memory(&w, &json!({"intentId":"intent:b"}), Some(other))
            .await
            .unwrap();
        if mode == "drop" {
            drop(c);
            assert!(!active.load(Ordering::Acquire));
            assert!(!other_active.load(Ordering::Acquire));
            continue;
        }
        if mode != "shutdown" {
            c.start_prepared(&w, &r, &a).await.unwrap();
            if mode == "cancel" {
                assert!(c
                    .cancel(&json!({"providerSessionId":a,"providerTurnId":"foreign"}))
                    .await
                    .is_err());
                assert!(active.load(Ordering::Acquire));
                c.cancel(&json!({"providerSessionId":a,"providerTurnId":"turn-1"}))
                    .await
                    .unwrap();
            } else if mode != "immediate" {
                wait_revoked(&active).await;
            }
            assert!(!active.load(Ordering::Acquire));
            assert_eq!(
                other_active.load(Ordering::Acquire),
                !matches!(mode, "lost" | "fatal")
            );
            c.drain_signals().await;
        }
        c.shutdown().await;
        assert!(!active.load(Ordering::Acquire));
        assert!(!other_active.load(Ordering::Acquire));
    }
}

#[tokio::test]
async fn foreign_turns_and_idle_prepared_threads_cannot_revoke_or_complete_a_run() {
    let (_dir, w, r) = work();
    for mode in ["foreign", "foreign-before", "prepared-notify"] {
        let (_peer, p) = programs(mode);
        let c = Controller::with_programs(p);
        let (binding, active, _) = memory("private-ticket");
        let session = c.prepare_with_memory(&w, &r, Some(binding)).await.unwrap();
        if mode != "prepared-notify" {
            c.start_prepared(&w, &r, &session).await.unwrap();
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(active.load(Ordering::Acquire));
        if mode != "foreign" {
            assert!(c.drain_signals().await.is_empty());
            assert!(active.load(Ordering::Acquire));
        }
        c.shutdown().await;
        assert!(!active.load(Ordering::Acquire));
    }
}

#[tokio::test]
async fn fatal_reader_revokes_owned_and_preparing_leases_without_polling_prepare() {
    let (_peer, p) = programs("inventory-fatal");
    let c = Controller::with_programs(p.clone());
    let (_dir, w, r) = work();
    let (binding, active, _) = memory("private-ticket-A");
    c.prepare_with_memory(&w, &r, Some(binding)).await.unwrap();
    let (binding, preparing_active, _) = memory("private-ticket-B");
    let next = json!({"intentId":"intent:second"});
    let mut preparing = Box::pin(c.prepare_with_memory(&w, &next, Some(binding)));
    tokio::select! {
        biased;
        sent = tokio::time::timeout(Duration::from_secs(3), async {
            while !requests(&p).iter().any(|v| v["method"] == "mcpServerStatus/list" && v["params"]["threadId"] == "owned-2") {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }) => sent.expect("second prepare must reach inventory before fixture failure"),
        result = &mut preparing => panic!("prepare unexpectedly completed before reader proof: {result:?}"),
    }
    // Keep the future alive but unpolled: only the reader can revoke either flag now.
    wait_revoked(&active).await;
    wait_revoked(&preparing_active).await;
    drop(preparing);
    c.shutdown().await;
}

#[tokio::test]
async fn early_terminal_is_bound_and_revoked_before_the_start_future_is_resumed() {
    let (_peer, p) = programs("notify-before-held");
    let c = Controller::with_programs(p.clone());
    let (_dir, w, r) = work();
    let (binding, active, _) = memory("private-ticket");
    let session = c.prepare_with_memory(&w, &r, Some(binding)).await.unwrap();
    let mut starting = Box::pin(c.start_prepared(&w, &r, &session));
    tokio::select! {
        biased;
        sent = tokio::time::timeout(Duration::from_secs(3), async {
            while !requests(&p).iter().any(|v| v["method"] == "turn/start") {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }) => sent.expect("turn start must reach fixture before its terminal response"),
        result = &mut starting => panic!("turn unexpectedly finished before reader proof: {result:?}"),
    }
    wait_revoked(&active).await;
    assert_eq!(starting.await.unwrap(), (session, "turn-1".into()));
    let signals = c.drain_signals().await;
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0]["status"], "completed");
    c.shutdown().await;
}

#[tokio::test]
async fn aborted_preparation_and_revoked_prepared_session_cannot_retain_access() {
    let (_peer, p) = programs("prepare-hang");
    let c = Arc::new(Controller::with_programs(p.clone()));
    let (_dir, w, r) = work();
    let (binding, active, _) = memory("private-ticket");
    let task = {
        let (c, w, r) = (c.clone(), w.clone(), r.clone());
        tokio::spawn(async move { c.prepare_with_memory(&w, &r, Some(binding)).await })
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        while !requests(&p).iter().any(|v| v["method"] == "thread/start") {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(!active.load(Ordering::Acquire));
    c.shutdown().await;

    let (_peer, p) = programs("normal");
    let c = Controller::with_programs(p.clone());
    let (binding, active, _) = memory("private-ticket");
    let session = c.prepare_with_memory(&w, &r, Some(binding)).await.unwrap();
    active.store(false, Ordering::Release);
    assert_eq!(
        c.start_prepared(&w, &r, &session).await.unwrap_err(),
        "MEMORY_CONNECTION_FAILED"
    );
    assert!(!requests(&p).iter().any(|v| v["method"] == "turn/start"));
    c.shutdown().await;
}
