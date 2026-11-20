pub mod abilities;
pub mod agent_memory;
mod app_connections;
mod browser;
pub mod cloud;
mod conversation;
mod creation;
pub mod decisions;
pub mod desktop_login;
mod desktop_preferences;
pub mod domain;
pub mod extraction;
mod host;
pub mod locales;
mod production;
mod project_capabilities;
mod project_crews;
pub mod project_memory;
mod project_studio;
mod project_workflows;
pub mod providers;
pub mod retrieval;
pub mod shared_memory;
pub mod store;
mod teams;
mod web_access;
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
    let configured = std::env::var_os("MOXCREW_DATA_DIR");
    let previous = std::env::var_os("PATCHFLEET_DATA_DIR");
    if configured.is_some() && previous.is_some() && configured != previous {
        return Err(Error("CONFLICTING_DATA_DIRECTORIES".into()));
    }
    if let Some(value) = configured.or(previous) {
        if value.is_empty() {
            return Err(Error("INVALID_DATA_DIRECTORY".into()));
        }
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            return Err(Error("DATA_DIRECTORY_MUST_BE_ABSOLUTE".into()));
        }
        return Ok(path);
    }
    let (current, legacy) = if cfg!(target_os = "windows") {
        // LocalAppData/Patchfleet is the former installer directory, not a
        // canonical data profile. Keep writable state outside the install tree.
        let roaming = PathBuf::from(
            std::env::var_os("APPDATA").ok_or_else(|| Error("APPDATA_UNAVAILABLE".into()))?,
        );
        (
            roaming.join("Moxcrew"),
            vec![
                roaming.join("com.patchfleet.desktop"),
                roaming.join("Patchfleet"),
            ],
        )
    } else {
        let home = PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| Error("HOME_UNAVAILABLE".into()))?,
        );
        if cfg!(target_os = "macos") {
            let base = home.join("Library/Application Support");
            (
                base.join("Moxcrew"),
                vec![base.join("com.patchfleet.desktop"), base.join("Patchfleet")],
            )
        } else {
            let base = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(".local/share"));
            (
                base.join("moxcrew"),
                vec![base.join("com.patchfleet.desktop")],
            )
        }
    };
    select_default_data_directory(current, &legacy)
}

fn select_default_data_directory(
    current: std::path::PathBuf,
    legacy: &[std::path::PathBuf],
) -> Result<std::path::PathBuf> {
    use std::io::ErrorKind;
    fn directory_exists(path: &std::path::Path) -> Result<bool> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_dir() && !store::linked(&metadata) => Ok(true),
            Ok(_) => Err(Error("UNSAFE_DATA_DIRECTORY".into())),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(_) => Err(Error("DATA_DIRECTORY_ACCESS_FAILED".into())),
        }
    }
    let new_exists = directory_exists(&current)?;
    let mut found = Vec::new();
    for path in legacy {
        if path != &current && !found.contains(path) && directory_exists(path)? {
            found.push(path.clone());
        }
    }
    if found.len() > 1 || (new_exists && !found.is_empty()) {
        return Err(Error("DATA_DIRECTORY_COLLISION".into()));
    }
    if let Some(old) = found.pop() {
        for marker in ["events.jsonl", "writer.lock"] {
            match std::fs::symlink_metadata(old.join(marker)) {
                Ok(metadata) if metadata.is_file() && !store::linked(&metadata) => {
                    // Persisted work items name absolute directories under this
                    // profile. Reopen it in place under the Store's writer lock.
                    return Ok(old);
                }
                Ok(_) => return Err(Error("UNSAFE_DATA_DIRECTORY".into())),
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(_) => return Err(Error("DATA_DIRECTORY_ACCESS_FAILED".into())),
            }
        }
        return Err(Error("UNRECOGNIZED_LEGACY_DATA_DIRECTORY".into()));
    }
    Ok(current)
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

#[cfg(test)]
mod data_directory_tests {
    use super::*;

    #[test]
    fn existing_profile_stays_at_its_original_absolute_path() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("Patchfleet");
        let new = root.path().join("Moxcrew");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("events.jsonl"), b"event-byte-canary\n").unwrap();
        std::fs::write(old.join("paired-host.json"), b"credential-byte-canary").unwrap();
        assert_eq!(
            select_default_data_directory(new.clone(), std::slice::from_ref(&old)).unwrap(),
            old
        );
        assert!(!new.exists());
        assert_eq!(
            std::fs::read(old.join("events.jsonl")).unwrap(),
            b"event-byte-canary\n"
        );
        assert_eq!(
            std::fs::read(old.join("paired-host.json")).unwrap(),
            b"credential-byte-canary"
        );
    }

    #[test]
    fn fresh_install_uses_the_current_name() {
        let root = tempfile::tempdir().unwrap();
        let current = root.path().join("Moxcrew");
        assert_eq!(
            select_default_data_directory(
                current.clone(),
                &[root.path().join("com.patchfleet.desktop")]
            )
            .unwrap(),
            current
        );
        assert!(!current.exists());
    }

    #[test]
    fn collisions_leave_all_directories_untouched() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("Patchfleet");
        let older = root.path().join("com.patchfleet.desktop");
        let new = root.path().join("Moxcrew");
        for path in [&old, &older, &new] {
            std::fs::create_dir(path).unwrap();
            std::fs::write(
                path.join("marker"),
                path.file_name().unwrap().as_encoded_bytes(),
            )
            .unwrap();
        }
        assert_eq!(
            select_default_data_directory(new.clone(), &[old.clone(), older.clone()])
                .unwrap_err()
                .0,
            "DATA_DIRECTORY_COLLISION"
        );
        for path in [&old, &older, &new] {
            assert!(path.join("marker").exists());
        }
    }

    #[test]
    fn unrecognized_legacy_folder_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("Patchfleet");
        let new = root.path().join("Moxcrew");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("moxcrew-desktop.exe"), b"installer-canary").unwrap();
        assert_eq!(
            select_default_data_directory(new.clone(), std::slice::from_ref(&old))
                .unwrap_err()
                .0,
            "UNRECOGNIZED_LEGACY_DATA_DIRECTORY"
        );
        assert!(old.join("moxcrew-desktop.exe").exists());
        assert!(!new.exists());
    }

    #[test]
    fn an_active_legacy_writer_keeps_exclusive_ownership() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("com.patchfleet.desktop");
        let new = root.path().join("Moxcrew");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("events.jsonl"), b"preserve-me\n").unwrap();
        let lock = std::fs::File::create(old.join("writer.lock")).unwrap();
        fs2::FileExt::lock_exclusive(&lock).unwrap();
        let selected =
            select_default_data_directory(new.clone(), std::slice::from_ref(&old)).unwrap();
        assert_eq!(selected, old);
        assert_eq!(
            store::Store::open(selected).err().unwrap().0,
            "LOCAL_WRITER_ALREADY_RUNNING"
        );
        assert_eq!(
            std::fs::read(old.join("events.jsonl")).unwrap(),
            b"preserve-me\n"
        );
        assert!(!new.exists());
        fs2::FileExt::unlock(&lock).unwrap();
    }
}
