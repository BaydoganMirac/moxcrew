use serde_json::{json, Value};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn command(mut payload: Value) -> Value {
    let now = chrono::Utc::now();
    payload["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    payload["createdAt"] = json!(now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    payload["expiresAt"] =
        json!((now + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    payload
}

#[test]
fn default_upgrade_reopens_original_jobs_receipts_and_output_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let base = if cfg!(target_os = "macos") {
        root.join("Library/Application Support")
    } else {
        root.to_owned()
    };
    let legacy = base.join("com.patchfleet.desktop");
    let action = |directory: Option<&Path>, name: &str, payload: &Value| {
        let mut cli = Command::new(env!("CARGO_BIN_EXE_moxcrew"));
        cli.env_remove("MOXCREW_DATA_DIR")
            .env_remove("PATCHFLEET_DATA_DIR");
        // Set default path inputs on this child only, independently of the owner's installation.
        if cfg!(windows) {
            cli.env("APPDATA", root);
        } else if cfg!(target_os = "macos") {
            cli.env("HOME", root);
        } else {
            cli.env("XDG_DATA_HOME", root);
        }
        if let Some(path) = directory {
            cli.arg("--data-dir").arg(path);
        }
        let mut child = cli
            .args(["action", name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(payload).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };

    let create = command(json!({"name":"Existing profile"}));
    let created = action(Some(&legacy), "project_create", &create);
    let project = &created["projectId"];
    let enqueue = command(
        json!({"projectId":project,"expectedProjectRevision":1,"abilityId":"D1",
        "title":"Brief","instruction":"Prepare a brief."}),
    );
    let queued = action(Some(&legacy), "project_work_enqueue", &enqueue);
    let work = &queued["workItemId"];
    let job = std::fs::canonicalize(&legacy)
        .unwrap()
        .join("project-jobs")
        .join(project.as_str().unwrap().strip_prefix("project:").unwrap())
        .join(work.as_str().unwrap().strip_prefix("work:").unwrap());
    let contents = b"Original project output\n";
    std::fs::write(job.join("outputs/brief.md"), contents).unwrap();
    let history = std::fs::read(legacy.join("events.jsonl")).unwrap();

    assert_eq!(action(None, "project_create", &create), created);
    assert_eq!(action(None, "project_work_enqueue", &enqueue), queued);
    let selection = json!({"projectId":project, "workItemId":work});
    let listing = action(None, "project_results_read", &selection);
    assert_eq!(listing["items"][0]["path"], "outputs/brief.md");
    assert_eq!(std::fs::read(legacy.join("events.jsonl")).unwrap(), history);
    let destination: PathBuf = root.join("export.md");
    let export = command(json!({"projectId":project, "workItemId":work,
        "path":"outputs/brief.md", "targetPath":destination}));
    let delivered = action(None, "project_result_export", &export);
    assert_eq!(delivered["reasonCode"], "PROJECT_RESULT_EXPORTED");
    assert_eq!(std::fs::read(destination).unwrap(), contents);
    assert_eq!(action(None, "project_result_export", &export), delivered);
    assert!(!base
        .join(if cfg!(target_os = "linux") {
            "moxcrew"
        } else {
            "Moxcrew"
        })
        .exists());
}
