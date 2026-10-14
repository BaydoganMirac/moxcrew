use patchfleet_core::providers::{self, Controller, Programs};
use serde_json::json;
use std::{
    path::PathBuf,
    process::Command,
    sync::{Mutex, OnceLock},
    time::Duration,
};
fn fixture(mode: &str) -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    static COPYING: Mutex<()> = Mutex::new(());
    let dir = BUILT.get_or_init(|| {
        let d = tempfile::tempdir().unwrap().keep();
        let exe = d.join(if cfg!(windows) {
            "fixture.exe"
        } else {
            "fixture"
        });
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| {
            PathBuf::from(env!("CARGO"))
                .with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" })
                .into_os_string()
        });
        let status = Command::new(rustc)
            .args([
                "--edition=2021",
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/provider_fake.rs"),
                "-o",
            ])
            .arg(&exe)
            .status()
            .unwrap();
        assert!(status.success());
        d
    });
    let name = format!("{mode}{}", if cfg!(windows) { ".exe" } else { "" });
    let path = dir.join(name);
    let _copy = COPYING.lock().unwrap();
    if !path.exists() {
        std::fs::copy(
            dir.join(if cfg!(windows) {
                "fixture.exe"
            } else {
                "fixture"
            }),
            &path,
        )
        .unwrap();
    }
    path
}
fn programs(mode: &str) -> Programs {
    Programs {
        codex: fixture(mode),
        claude: fixture("claude"),
        gemini: fixture("gemini"),
        timeout: Duration::from_secs(2),
    }
}
fn work() -> (tempfile::TempDir, serde_json::Value, serde_json::Value) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    let w = json!({"workingDirectory":dir.path().to_str().unwrap(),"instruction":"SECRET_LOCAL_INSTRUCTION"});
    let r = json!({"runId":"run:test","intentId":"intent:test"});
    (dir, w, r)
}
#[tokio::test]
async fn supported_observation_is_bounded_and_sanitized() {
    let all = providers::observe_with(&programs("codex")).await.unwrap();
    assert_eq!(all.len(), 3);
    assert!(providers::supports_codex_control(&all[0]));
    assert_eq!(all[0]["sessions"][0]["status"], "running");
    assert_eq!(all[1]["sessions"][0]["providerSessionId"], "job:job-1");
    assert_eq!(all[2]["provider"]["state"], "available");
    let s = serde_json::to_string(&all).unwrap();
    assert!(!s.contains("SECRET"));
    assert!(!s.contains("/secret"));
    let mut p = programs("codex");
    p.gemini = fixture("gemini-stderr");
    let all = providers::observe_with(&p).await.unwrap();
    assert_eq!(all[2]["provider"]["state"], "available");
}
#[tokio::test]
async fn unavailable_malformed_timeout_are_safe() {
    let mut p = programs("codex-malformed");
    let all = providers::observe_with(&p).await.unwrap();
    assert_eq!(
        all[0]["provider"]["error"]["code"],
        "CODEX_VERSION_MALFORMED"
    );
    assert!(!all[0].to_string().contains("SECRET"));
    p.codex = fixture("codex-timeout");
    let at = std::time::Instant::now();
    let all = providers::observe_with(&p).await.unwrap();
    assert_eq!(all[0]["provider"]["error"]["code"], "CODEX_PROBE_TIMEOUT");
    assert!(at.elapsed() < Duration::from_secs(5));
    p.codex = PathBuf::from("patchfleet-no-such-provider-1234");
    let all = providers::observe_with(&p).await.unwrap();
    assert_eq!(all[0]["provider"]["state"], "unavailable");
    p.codex = fixture("codex-bad-rpc");
    let all = providers::observe_with(&p).await.unwrap();
    assert_eq!(
        all[0]["provider"]["error"]["code"],
        "CODEX_PROTOCOL_INVALID_JSON"
    );
    assert!(!all[0].to_string().contains("SECRET"));
    p.codex = fixture("codex-rpc-hang");
    let all = providers::observe_with(&p).await.unwrap();
    assert_eq!(
        all[0]["provider"]["error"]["code"],
        "CODEX_APP_SERVER_TIMEOUT"
    );
}
#[tokio::test]
async fn completion_failure_and_two_phase_start() {
    for (mode, status) in [("codex-complete", "completed"), ("codex-fail", "failed")] {
        let (_dir, w, r) = work();
        let c = Controller::with_programs(programs(mode));
        let session = c.prepare(&w, &r).await.unwrap();
        assert_eq!(session, "owned-thread");
        assert!(c.drain_signals().await.is_empty());
        let pair = c.start_prepared(&w, &r, &session).await.unwrap();
        assert_eq!(pair, ("owned-thread".into(), "owned-turn".into()));
        tokio::time::sleep(Duration::from_millis(50)).await;
        let signals = c.drain_signals().await;
        assert_eq!(signals.last().unwrap()["status"], status);
        assert!(!serde_json::to_string(&signals).unwrap().contains("SECRET"));
        c.shutdown().await;
    }
}

#[tokio::test]
async fn unicode_instruction_uses_the_same_utf16_limit_as_intake() {
    let (_dir, mut w, r) = work();
    w["instruction"] = json!("界".repeat(30_000));
    let c = Controller::with_programs(programs("codex-complete"));
    let session = c.prepare(&w, &r).await.unwrap();
    assert!(c.start_prepared(&w, &r, &session).await.is_ok());
    c.shutdown().await;
}
#[tokio::test]
async fn cancel_and_session_loss_never_reattach() {
    let (_dir, w, r) = work();
    let c = Controller::with_programs(programs("codex-cancel"));
    let (session, turn) = c.start(&w, &r).await.unwrap();
    let run = json!({"providerSessionId":session,"providerTurnId":turn});
    c.cancel(&run).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        c.drain_signals().await.last().unwrap()["status"],
        "interrupted"
    );
    c.shutdown().await;
    assert_eq!(c.cancel(&run).await.unwrap_err(), "CODEX_SESSION_LOST");
    let c = Controller::with_programs(programs("codex-lost"));
    c.start(&w, &r).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(c.drain_signals().await.last().unwrap()["status"], "unknown");
    assert_eq!(c.cancel(&run).await.unwrap_err(), "CODEX_SESSION_LOST");
}
#[tokio::test]
async fn uncertain_turn_is_not_retried() {
    let (_dir, w, r) = work();
    let c = Controller::with_programs(programs("codex-start-hang"));
    let s = c.prepare(&w, &r).await.unwrap();
    assert_eq!(
        c.start_prepared(&w, &r, &s).await.unwrap_err(),
        "START_OUTCOME_UNKNOWN"
    );
    assert_eq!(
        c.start_prepared(&w, &r, &s).await.unwrap_err(),
        "CODEX_SESSION_LOST"
    );
}
#[test]
fn hook_and_version_contract() {
    assert!(!providers::supports_codex_control(
        &json!({"provider":{"id":"codex","state":"available","version":"0.144.1-beta.1","capabilities":{"recentObservation":true,"explicitLiveStatus":true}}})
    ));
    let signal=providers::decode_gemini_hook(r#"{"hook_event_name":"BeforeAgent","session_id":"gemini:one","timestamp":"2026-10-14T00:00:00.000Z","prompt":"SECRET","cwd":"/secret"}"#).unwrap().unwrap();
    assert_eq!(signal["status"], "running");
    assert_eq!(signal.as_object().unwrap().len(), 5);
    assert!(!signal.to_string().contains("SECRET"));
    assert!(providers::decode_gemini_hook(r#"{"hook_event_name":"Invalid"}"#).is_err());
}
#[tokio::test]
async fn inbox_is_acknowledged_only_after_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let signal=providers::decode_gemini_hook(r#"{"hook_event_name":"AfterAgent","session_id":"gemini:one","timestamp":"2026-10-14T00:00:00.000Z","prompt":"SECRET"}"#).unwrap().unwrap();
    providers::write_gemini_inbox(dir.path(), &signal).unwrap();
    let entries = providers::read_gemini_inbox(dir.path()).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].signal, signal);
    assert!(entries[0].path.exists());
    assert_eq!(
        providers::read_gemini_inbox(dir.path())
            .await
            .unwrap()
            .len(),
        1
    ); // Simulate a failed durable append: no ACK, safe retry.
    providers::ack_gemini_inbox(&entries[0]).await.unwrap();
    assert!(providers::read_gemini_inbox(dir.path())
        .await
        .unwrap()
        .is_empty());
    std::fs::write(
        dir.path().join("gemini-inbox/signal-invalid.json"),
        r#"{"prompt":"SECRET"}"#,
    )
    .unwrap();
    assert!(providers::read_gemini_inbox(dir.path())
        .await
        .unwrap()
        .is_empty());
    assert!(!dir.path().join("gemini-inbox/signal-invalid.json").exists());
}
#[cfg(windows)]
#[tokio::test]
async fn npm_cmd_shim_with_spaces_is_supported() {
    let dir = tempfile::tempdir().unwrap();
    let directory = dir.path().join("shim with spaces");
    std::fs::create_dir(&directory).unwrap();
    let cmd = directory.join("codex.cmd");
    std::fs::write(
        &cmd,
        format!(
            "@echo off\r\n\"{}\" %*\r\n",
            fixture("codex-cancel").display()
        ),
    )
    .unwrap();
    let mut p = programs("codex-cancel");
    p.codex = cmd;
    let all = providers::observe_with(&p).await.unwrap();
    assert!(providers::supports_codex_control(&all[0]));
    let (_dir, w, r) = work();
    let c = Controller::with_programs(p);
    let (s, t) = c.start(&w, &r).await.unwrap();
    c.cancel(&json!({"providerSessionId":s,"providerTurnId":t}))
        .await
        .unwrap();
    c.shutdown().await;
}
