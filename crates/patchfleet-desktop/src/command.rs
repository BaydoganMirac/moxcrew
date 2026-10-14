use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct Command {
    pub action: &'static str,
    pub payload: Value,
}

impl Command {
    pub fn new(action: &'static str, mut payload: Value) -> Self {
        payload["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
        payload["createdAt"] = json!(patchfleet_core::now());
        payload["expiresAt"] = json!((chrono::Utc::now() + chrono::Duration::minutes(5))
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
        assert!((299_990..=300_010)
            .contains(&expires.signed_duration_since(created).num_milliseconds()));
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
}
