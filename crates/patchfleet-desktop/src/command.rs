use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct Command {
    pub action: &'static str,
    pub payload: Value,
}

pub fn account_action(action: &str) -> bool {
    matches!(
        action,
        "cloud_login_begin"
            | "cloud_login_poll"
            | "cloud_login_cancel"
            | "cloud_account_read"
            | "cloud_logout"
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Notice {
    pub action: &'static str,
    pub code: String,
    pub kind: NoticeKind,
}

impl Notice {
    pub fn receipt(action: &'static str, receipt: &Value) -> Self {
        let kind = match receipt["outcome"].as_str() {
            Some("applied") | None => NoticeKind::Success,
            Some("rejected" | "expired") => NoticeKind::Warning,
            _ => NoticeKind::Error,
        };
        Self {
            action,
            code: safe_error(receipt["reasonCode"].as_str().unwrap_or("ACTION_RECORDED")),
            kind,
        }
    }

    pub fn error(action: &'static str, code: &str) -> Self {
        Self {
            action,
            code: safe_error(code),
            kind: NoticeKind::Error,
        }
    }

    pub fn retryable(&self) -> bool {
        !crate::folder::is_problem(&self.code)
            && !self.code.starts_with("STALE_")
            && !self.code.starts_with("INVALID_")
            && !matches!(
                self.code.as_str(),
                "IDEMPOTENCY_KEY_REUSED"
                    | "OWNER_COMMAND_OUTCOME_UNKNOWN"
                    | "START_OUTCOME_UNKNOWN"
                    | "COMMAND_EXPIRED"
            )
    }
}

impl Command {
    pub fn new(action: &'static str, mut payload: Value) -> Self {
        payload["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
        let created = chrono::Utc::now();
        payload["createdAt"] = json!(created.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        payload["expiresAt"] = json!((created + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        Self { action, payload }
    }
}

pub fn safe_error(error: impl std::fmt::Display) -> String {
    let message = error.to_string();
    if (2..=100).contains(&message.len())
        && message
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    {
        message
    } else {
        "HOST_ACTION_FAILED".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_keeps_original_identity_time_and_target_revision() {
        let command = Command::new(
            "work_start",
            json!({"workItemId":"work:test","expectedItemRevision":7}),
        );
        let retry = command.clone();
        assert_eq!(command.payload, retry.payload);
        assert_eq!(retry.payload["expectedItemRevision"], 7);
        let created =
            chrono::DateTime::parse_from_rfc3339(command.payload["createdAt"].as_str().unwrap())
                .unwrap();
        let expires =
            chrono::DateTime::parse_from_rfc3339(command.payload["expiresAt"].as_str().unwrap())
                .unwrap();
        assert_eq!(
            expires.signed_duration_since(created).num_milliseconds(),
            300_000
        );
        assert_ne!(
            command.payload["commandId"],
            Command::new("work_start", json!({})).payload["commandId"]
        );
    }

    #[test]
    fn errors_do_not_disclose_paths_or_credentials() {
        assert_eq!(safe_error("STALE_ITEM_REVISION"), "STALE_ITEM_REVISION");
        assert_eq!(
            safe_error("failed reading /private/synthetic/token"),
            "HOST_ACTION_FAILED"
        );
    }

    #[test]
    fn feedback_keeps_rejections_unknown_results_and_recovery_truthful() {
        for outcome in ["rejected", "expired"] {
            assert_eq!(
                Notice::receipt(
                    "workspace_register",
                    &json!({"outcome":outcome,"reasonCode":"WORKSPACE_ALREADY_REGISTERED"})
                )
                .kind,
                NoticeKind::Warning
            );
        }
        for outcome in ["failed", "unknown"] {
            assert_eq!(
                Notice::receipt("work_start", &json!({"outcome":outcome})).kind,
                NoticeKind::Error
            );
        }
        for code in [
            "WORKSPACE_NOT_GIT_ROOT",
            "WORKSPACE_ACCESS_DENIED",
            "STALE_PROJECT_REVISION",
            "INVALID_PAYLOAD",
            "OWNER_COMMAND_OUTCOME_UNKNOWN",
            "COMMAND_EXPIRED",
        ] {
            assert!(!Notice::error("workspace_register", code).retryable());
        }
        assert!(Notice::error("workspace_register", "HOST_BUSY").retryable());
        assert_eq!(
            Notice::error(
                "workspace_register",
                "failed reading /private/synthetic/token"
            )
            .code,
            "HOST_ACTION_FAILED"
        );
    }
}
