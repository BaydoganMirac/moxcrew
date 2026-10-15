pub mod cloud;
pub mod decisions;
mod desktop_preferences;
pub mod domain;
mod host;
pub mod project_memory;
pub mod providers;
pub mod retrieval;
pub mod shared_memory;
pub mod store;
mod teams;
mod work;

pub use host::Host;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub String);

pub type Result<T> = std::result::Result<T, Error>;

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self(value.to_string())
    }
}
impl From<reqwest::Error> for Error {
    fn from(_value: reqwest::Error) -> Self {
        Self("CLOUD_REQUEST_FAILED".into())
    }
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn event(kind: &str, payload: serde_json::Value) -> serde_json::Value {
    let recorded_at = if kind.ends_with(".receipted") {
        payload["receipt"]["completedAt"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(now)
    } else {
        now()
    };
    serde_json::json!({"id": uuid::Uuid::new_v4().to_string(), "schemaVersion": 1,
        "type": kind, "recordedAt": recorded_at, "payload": payload})
}

pub fn default_data_directory() -> Result<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(value) = std::env::var_os("PATCHFLEET_DATA_DIR") {
        if value.is_empty() {
            return Err(Error("INVALID_DATA_DIRECTORY".into()));
        }
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            return Err(Error("DATA_DIRECTORY_MUST_BE_ABSOLUTE".into()));
        }
        return Ok(path);
    }
    let base = if cfg!(target_os = "windows") {
        PathBuf::from(
            std::env::var_os("APPDATA").ok_or_else(|| Error("APPDATA_UNAVAILABLE".into()))?,
        )
    } else {
        let home = PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| Error("HOME_UNAVAILABLE".into()))?,
        );
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support")
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(".local/share"))
        }
    };
    Ok(base.join("com.patchfleet.desktop"))
}

pub fn gemini_hook() -> Result<()> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().take(131_073).read_to_string(&mut input)?;
    if input.len() > 131_072 {
        return Err(Error("HOOK_PAYLOAD_TOO_LARGE".into()));
    }
    if let Some(signal) = providers::decode_gemini_hook(&input).map_err(Error)? {
        providers::write_gemini_inbox(&default_data_directory()?, &signal).map_err(Error)?;
    }
    Ok(())
}
