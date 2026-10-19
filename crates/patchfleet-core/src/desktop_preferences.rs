use crate::{store, Result};
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;

pub(crate) fn load(directory: &Path) -> Value {
    let path = directory.join("desktop-preferences.json");
    if std::fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return json!({"language":"tr","reduceMotion":false,"error":false});
    }
    let parsed = store::open_private_reader(&path).ok().and_then(|file| {
        let mut bytes = Vec::new();
        file.take(1025).read_to_end(&mut bytes).ok()?;
        (bytes.len() <= 1024)
            .then(|| serde_json::from_slice::<Value>(&bytes).ok())
            .flatten()
    });
    match parsed {
        Some(value)
            if ((value["schemaVersion"] == 1
                && value.as_object().is_some_and(|m| m.len() == 2))
                || (value["schemaVersion"] == 2
                    && value["reduceMotion"].is_boolean()
                    && value.as_object().is_some_and(|m| m.len() == 3)))
                && matches!(value["language"].as_str(), Some("tr" | "en")) =>
        {
            json!({"language":value["language"],"reduceMotion":value["reduceMotion"].as_bool().unwrap_or(false),"error":false})
        }
        _ => json!({"language":"tr","reduceMotion":false,"error":true}),
    }
}

pub(crate) fn save(directory: &Path, language: &str) -> Result<Value> {
    if !matches!(language, "tr" | "en") {
        return Err(crate::Error("INVALID_DESKTOP_LANGUAGE".into()));
    }
    persist(
        directory,
        language,
        load(directory)["reduceMotion"].as_bool().unwrap_or(false),
    )
}

pub(crate) fn save_motion(directory: &Path, reduce: bool) -> Result<Value> {
    let current = load(directory);
    persist(
        directory,
        current["language"].as_str().unwrap_or("tr"),
        reduce,
    )
}

fn persist(directory: &Path, language: &str, reduce: bool) -> Result<Value> {
    store::atomic_write(
        &directory.join("desktop-preferences.json"),
        &serde_json::to_vec(&json!({"schemaVersion":2,"language":language,"reduceMotion":reduce}))?,
    )?;
    Ok(json!({"language":language,"reduceMotion":reduce,"error":false}))
}
