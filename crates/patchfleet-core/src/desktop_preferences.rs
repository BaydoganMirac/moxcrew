use crate::{store, Result};
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;

pub(crate) fn load(directory: &Path) -> Value {
    let path = directory.join("desktop-preferences.json");
    if std::fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return json!({"language":"tr","error":false});
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
            if value.as_object().is_some_and(|m| m.len() == 2)
                && value["schemaVersion"] == 1
                && matches!(value["language"].as_str(), Some("tr" | "en")) =>
        {
            json!({"language":value["language"],"error":false})
        }
        _ => json!({"language":"tr","error":true}),
    }
}

pub(crate) fn save(directory: &Path, language: &str) -> Result<Value> {
    if !matches!(language, "tr" | "en") {
        return Err(crate::Error("INVALID_DESKTOP_LANGUAGE".into()));
    }
    store::atomic_write(
        &directory.join("desktop-preferences.json"),
        &serde_json::to_vec(&json!({"schemaVersion":1,"language":language}))?,
    )?;
    Ok(json!({"language":language,"error":false}))
}
