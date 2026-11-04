//! Explicit local checks, separate from observation and work execution.
use super::{at_least, now, output_status, probe, Programs, DEFAULT_TIMEOUT};
use serde_json::{json, Value};
use std::time::Instant;

const AUTH_OUTPUT_LIMIT: usize = 64 * 1024;

pub(super) async fn read(programs: &Programs) -> Value {
    let (codex, claude, gemini) = tokio::join!(
        check("codex", programs),
        check("claude", programs),
        check("gemini", programs)
    );
    json!({"schemaVersion":1,"observedAt":now(),"providers":[codex,claude,gemini]})
}

async fn check(id: &str, programs: &Programs) -> Value {
    let started = Instant::now();
    let bound = programs.timeout.min(DEFAULT_TIMEOUT);
    let program = match id {
        "codex" => &programs.codex,
        "claude" => &programs.claude,
        _ => &programs.gemini,
    };
    let prefix = id.to_ascii_uppercase();
    let mut result = json!({"id":id,"installed":false,"version":null,
        "authentication":"unknown","controlSupported":false,"reasonCode":null});
    let version = match probe(program, id, bound).await {
        Ok(version) => version,
        Err(reason) => {
            result["installed"] = json!(!reason.ends_with("NOT_FOUND"));
            result["reasonCode"] = json!(reason);
            return result;
        }
    };
    result["installed"] = json!(true);
    result["version"] = json!(version);
    result["controlSupported"] = json!(
        (id == "codex" && at_least(&version, [0, 144, 1], true))
            || super::cli_control::supported(id, &version)
    );
    if id != "codex" && result["controlSupported"] == true {
        result["toolReadiness"] = json!("configured_unverified");
    }
    if id == "gemini" {
        result["reasonCode"] = json!("GEMINI_AUTH_CHECK_UNSUPPORTED");
        return result;
    }
    let args: &[&str] = if id == "codex" {
        &["login", "status"]
    } else {
        &["auth", "status"]
    };
    let authentication = match output_status(
        program,
        args,
        bound.saturating_sub(started.elapsed()),
        AUTH_OUTPUT_LIMIT,
    )
    .await
    {
        Ok((status, stdout, _)) => {
            let present = if id == "codex" {
                match status.code() {
                    Some(0) => Some(true),
                    Some(1) => Some(false),
                    _ => None,
                }
            } else {
                claude_authentication(status.code(), &stdout)
            };
            match present {
                Some(present) => {
                    result["authentication"] = json!(if present { "present" } else { "missing" });
                    if present {
                        "CREDENTIALS_PRESENT"
                    } else {
                        "AUTHENTICATION_REQUIRED"
                    }
                }
                None => "AUTH_CHECK_MALFORMED",
            }
        }
        Err(reason) => match reason.as_str() {
            "TIMEOUT" => "AUTH_CHECK_TIMEOUT",
            "MALFORMED" => "AUTH_CHECK_MALFORMED",
            _ => "AUTH_CHECK_FAILED",
        },
    };
    result["reasonCode"] = json!(format!("{prefix}_{authentication}"));
    result
}

fn claude_authentication(exit: Option<i32>, stdout: &[u8]) -> Option<bool> {
    let value: Value = serde_json::from_slice(stdout).ok()?;
    let present = match value.get("authMethod")?.as_str()? {
        "none" => false,
        "claude.ai" | "oauth_token" | "api_key" | "api_key_helper" | "third_party" => true,
        _ => return None,
    };
    if value
        .get("loggedIn")
        .is_some_and(|logged_in| logged_in.as_bool() != Some(present))
        || exit != Some(if present { 0 } else { 1 })
    {
        return None;
    }
    Some(present)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{managed_tests::peer, Controller};
    use std::{path::PathBuf, time::Duration};

    fn fixtures(codex: &str, claude: &str, gemini: &str) -> (Vec<tempfile::TempDir>, Programs) {
        let (codex_dir, mut programs) = peer(codex);
        let (claude_dir, claude_programs) = peer(claude);
        let (gemini_dir, gemini_programs) = peer(gemini);
        programs.claude = claude_programs.codex;
        programs.gemini = gemini_programs.codex;
        // These local peers answer immediately, but process scheduling on a loaded CI host
        // can be delayed while the full process-heavy suite runs. Deliberate timeout cases
        // override this below, so production deadlines and timeout coverage stay unchanged.
        programs.timeout = Duration::from_secs(8);
        (vec![codex_dir, claude_dir, gemini_dir], programs)
    }

    #[tokio::test]
    async fn installed_credentials_and_control_are_independent_and_sanitized() {
        let (_fixtures, programs) = fixtures("codex-auth-present", "claude-auth-present", "gemini");
        let status = Controller::with_programs(programs)
            .connection_status()
            .await;
        let providers = status["providers"].as_array().unwrap();
        assert_eq!(providers.len(), 3);
        for provider in providers {
            assert!(provider["installed"].as_bool().unwrap());
            assert!(provider["version"].is_string(), "{status}");
            assert_eq!(provider.as_object().unwrap().len(), 6);
        }
        assert_eq!(providers[0]["authentication"], "present");
        assert_eq!(providers[0]["controlSupported"], true);
        assert_eq!(providers[1]["authentication"], "present");
        assert_eq!(providers[1]["controlSupported"], false);
        assert_eq!(providers[2]["authentication"], "unknown");
        assert_eq!(providers[2]["controlSupported"], false);
        let serialized = status.to_string();
        for private in [
            "SECRET",
            "/private",
            "@example",
            "configDirectory",
            "authMethod",
        ] {
            assert!(!serialized.contains(private), "leaked {private}");
        }
    }

    #[tokio::test]
    async fn missing_credentials_unsupported_versions_and_absent_programs_stay_distinct() {
        let (_fixtures, mut programs) =
            fixtures("codex-auth-missing", "claude-auth-missing", "gemini");
        programs.gemini = PathBuf::from("moxcrew-no-provider-connections-fixture-123");
        let status = Controller::with_programs(programs)
            .connection_status()
            .await;
        assert_eq!(status["providers"][0]["authentication"], "missing");
        assert_eq!(status["providers"][0]["controlSupported"], true);
        assert_eq!(status["providers"][1]["authentication"], "missing");
        assert_eq!(status["providers"][2]["installed"], false);
        assert_eq!(status["providers"][2]["version"], Value::Null);
        assert_eq!(status["providers"][2]["authentication"], "unknown");
        let (_fixtures, programs) = fixtures("codex-auth-old", "claude-auth-present", "gemini");
        let status = Controller::with_programs(programs)
            .connection_status()
            .await;
        assert_eq!(status["providers"][0]["authentication"], "present");
        assert_eq!(status["providers"][0]["controlSupported"], false);
    }

    #[tokio::test]
    async fn malformed_exit_output_and_version_never_claim_missing_credentials() {
        for mode in ["malformed", "unexpected-exit", "oversize", "invalid-utf8"] {
            let (_fixtures, programs) = fixtures(
                &format!("codex-auth-{mode}"),
                &format!("claude-auth-{mode}"),
                "gemini",
            );
            let status = Controller::with_programs(programs)
                .connection_status()
                .await;
            // Codex has an exit-code contract; output text is deliberately discarded.
            if mode != "malformed" && mode != "invalid-utf8" {
                assert_eq!(status["providers"][0]["authentication"], "unknown");
            }
            assert_eq!(status["providers"][1]["authentication"], "unknown");
            assert!(!status.to_string().contains("SECRET"));
        }
        let (_fixtures, programs) = fixtures("codex-malformed", "claude-auth-present", "gemini");
        let status = Controller::with_programs(programs)
            .connection_status()
            .await;
        assert_eq!(status["providers"][0]["installed"], true);
        assert_eq!(status["providers"][0]["version"], Value::Null);
        assert_eq!(status["providers"][0]["authentication"], "unknown");
    }

    #[tokio::test]
    async fn authentication_deadlines_run_in_parallel() {
        let (_fixtures, mut programs) =
            fixtures("codex-auth-hang", "claude-auth-hang", "gemini-timeout");
        probe(&programs.codex, "codex", programs.timeout)
            .await
            .unwrap();
        probe(&programs.claude, "claude", programs.timeout)
            .await
            .unwrap();
        // Leave enough process-start headroom for Windows runners while keeping
        // the bound below the 1.8 seconds three serial deadlines would require.
        programs.timeout = Duration::from_millis(600);
        let started = Instant::now();
        let status = Controller::with_programs(programs)
            .connection_status()
            .await;
        assert!(started.elapsed() < Duration::from_millis(1_500));
        for provider in &status["providers"].as_array().unwrap()[..2] {
            assert_eq!(provider["authentication"], "unknown");
            assert!(provider["reasonCode"]
                .as_str()
                .unwrap()
                .ends_with("AUTH_CHECK_TIMEOUT"));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn authentication_timeout_reaps_owned_process_and_stops_its_descendant() {
        let (_fixture, programs) = peer("codex-auth-tree-hang");
        probe(&programs.codex, "codex", programs.timeout)
            .await
            .unwrap();
        let result = output_status(
            &programs.codex,
            &["login", "status"],
            Duration::from_millis(200),
            AUTH_OUTPUT_LIMIT,
        )
        .await;
        assert_eq!(result.unwrap_err(), "TIMEOUT");
        assert!(programs.codex.with_extension("child").exists());
        tokio::time::sleep(Duration::from_millis(650)).await;
        assert!(!programs.codex.with_extension("alive").exists());
        let executable = programs.codex.to_string_lossy();
        let processes = std::process::Command::new("/bin/ps")
            .args(["-axo", "command="])
            .output()
            .unwrap();
        assert!(processes.status.success());
        assert!(!String::from_utf8_lossy(&processes.stdout)
            .lines()
            .any(|line| line.starts_with(executable.as_ref())));
    }

    #[test]
    fn claude_schema_requires_documented_method_and_consistent_exit() {
        for method in [
            "claude.ai",
            "oauth_token",
            "api_key",
            "api_key_helper",
            "third_party",
        ] {
            assert_eq!(
                claude_authentication(Some(0), json!({"authMethod":method}).to_string().as_bytes()),
                Some(true)
            );
        }
        assert_eq!(
            claude_authentication(Some(1), br#"{"authMethod":"none","loggedIn":false}"#),
            Some(false)
        );
        for bytes in [
            br#"{"loggedIn":true}"#.as_slice(),
            br#"{"authMethod":"future","loggedIn":true}"#,
            br#"{"authMethod":"claude.ai","loggedIn":false}"#,
            br#"{"authMethod":"claude.ai","loggedIn":"true"}"#,
            br#"{"authMethod":"none","loggedIn":false}"#,
        ] {
            assert_eq!(claude_authentication(Some(0), bytes), None);
        }
    }
}
