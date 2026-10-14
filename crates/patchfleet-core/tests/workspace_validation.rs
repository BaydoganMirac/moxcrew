use patchfleet_core::{
    now,
    providers::{validate_workspace, validate_workspace_for_registration},
    Host,
};
use serde_json::json;
use std::{fs, path::Path};

async fn rejected(path: &str, code: &str) {
    assert_eq!(
        validate_workspace_for_registration(path).await.unwrap_err(),
        code
    );
    assert_eq!(
        validate_workspace(path).await.unwrap_err(),
        "WORKSPACE_NOT_ALLOWED"
    );
}

#[tokio::test]
async fn registration_distinguishes_invalid_missing_file_and_non_git_folder() {
    let directory = tempfile::tempdir().unwrap();
    for invalid in ["", "relative/path", "\0"] {
        rejected(invalid, "WORKSPACE_PATH_INVALID").await;
    }
    rejected(&format!("/{}", "x".repeat(4096)), "WORKSPACE_PATH_INVALID").await;
    rejected(
        directory.path().join("missing").to_str().unwrap(),
        "WORKSPACE_FOLDER_NOT_FOUND",
    )
    .await;
    let file = directory.path().join("file");
    fs::write(&file, "not a directory").unwrap();
    rejected(file.to_str().unwrap(), "WORKSPACE_NOT_DIRECTORY").await;
    rejected(directory.path().to_str().unwrap(), "WORKSPACE_NOT_GIT_ROOT").await;
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn registration_rejects_filesystem_and_home_roots_before_folder_enumeration() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().ancestors().last().unwrap();
    rejected(root.to_str().unwrap(), "WORKSPACE_UNSAFE_ROOT").await;
    if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        if let Some(home) = Path::new(&home).to_str() {
            rejected(home, "WORKSPACE_UNSAFE_ROOT").await;
        }
    }
}

#[tokio::test]
async fn registration_keeps_git_directories_linked_markers_and_canonical_aliases() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(workspace.join(".git")).unwrap();
    let canonical = fs::canonicalize(&workspace).unwrap();
    let alias = workspace.join(".");
    for path in [&workspace, &alias] {
        assert_eq!(
            validate_workspace_for_registration(path.to_str().unwrap())
                .await
                .unwrap(),
            canonical.to_str().unwrap()
        );
    }
    fs::remove_dir(workspace.join(".git")).unwrap();
    fs::write(
        workspace.join(".git"),
        "gitdir: ../repository/.git/worktrees/test\n",
    )
    .unwrap();
    assert_eq!(
        validate_workspace(workspace.to_str().unwrap())
            .await
            .unwrap(),
        canonical.to_str().unwrap()
    );
    assert_eq!(fs::read_dir(&workspace).unwrap().count(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn registration_checks_effective_read_traverse_and_write_access() {
    use std::os::unix::fs::PermissionsExt;
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(workspace.join(".git")).unwrap();
    for (mode, expected) in [
        (0o100, "WORKSPACE_ACCESS_DENIED"),
        (0o600, "WORKSPACE_ACCESS_DENIED"),
        (0o500, "WORKSPACE_READ_ONLY"),
    ] {
        fs::set_permissions(&workspace, fs::Permissions::from_mode(mode)).unwrap();
        let result = validate_workspace_for_registration(workspace.to_str().unwrap()).await;
        let legacy = validate_workspace(workspace.to_str().unwrap()).await;
        fs::set_permissions(&workspace, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(result.unwrap_err(), expected);
        assert_eq!(legacy.unwrap_err(), "WORKSPACE_NOT_ALLOWED");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn canonical_non_utf8_paths_reject_and_symbolic_aliases_resolve() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(workspace.join(".git")).unwrap();
    let alias = directory.path().join("alias");
    symlink(&workspace, &alias).unwrap();
    assert_eq!(
        validate_workspace_for_registration(alias.to_str().unwrap())
            .await
            .unwrap(),
        fs::canonicalize(&workspace).unwrap().to_str().unwrap()
    );
    #[cfg(not(target_os = "macos"))]
    {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let non_utf8 = directory.path().join(OsString::from_vec(vec![b'w', 0xff]));
        fs::create_dir(&non_utf8).unwrap();
        fs::create_dir(non_utf8.join(".git")).unwrap();
        fs::remove_file(&alias).unwrap();
        symlink(non_utf8, &alias).unwrap();
        rejected(alias.to_str().unwrap(), "WORKSPACE_PATH_INVALID").await;
    }
}

#[tokio::test]
async fn detailed_registration_rejection_writes_no_durable_request_or_receipt() {
    let directory = tempfile::tempdir().unwrap();
    let data = directory.path().join("data");
    let non_git = directory.path().join("not-a-worktree");
    fs::create_dir(&non_git).unwrap();
    let host = Host::open(data.clone()).unwrap();
    assert!(!data.join("events.jsonl").exists());
    let snapshot = host.snapshot().unwrap();
    for (path, expected) in [
        (non_git.clone(), "WORKSPACE_NOT_GIT_ROOT"),
        (
            directory.path().join("missing"),
            "WORKSPACE_FOLDER_NOT_FOUND",
        ),
    ] {
        let result = host
            .action(
                "workspace_register",
                json!({
                    "commandId":format!("cmd:{}",uuid::Uuid::new_v4()),
                    "createdAt":now(),
                    "expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5))
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis,true),
                    "workingDirectory":path,
                    "displayName":"Test"
                }),
            )
            .await;
        assert_eq!(result.unwrap_err().0, expected);
        assert!(!data.join("events.jsonl").exists());
        assert_eq!(
            host.snapshot().unwrap()["workspaces"],
            snapshot["workspaces"]
        );
    }
    host.shutdown().await;
}

#[tokio::test]
async fn canonical_registration_replays_receipt_after_folder_removal_and_rejects_reused_payloads() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(workspace.join(".git")).unwrap();
    let canonical = fs::canonicalize(&workspace).unwrap();
    let data = directory.path().join("data");
    let host = Host::open(data.clone()).unwrap();
    let mut command = json!({
        "commandId":format!("cmd:{}",uuid::Uuid::new_v4()),
        "createdAt":now(),
        "expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis,true),
        "workingDirectory":canonical,
        "displayName":"  Test repository  "
    });
    let receipt = host
        .action("workspace_register", command.clone())
        .await
        .unwrap();
    assert_eq!(receipt["outcome"], "applied");
    let before = fs::read(data.join("events.jsonl")).unwrap();

    #[cfg(unix)]
    if unsafe { libc::geteuid() } != 0 {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&workspace, fs::Permissions::from_mode(0o500)).unwrap();
        let replay = host.action("workspace_register", command.clone()).await;
        let mut new_command = command.clone();
        new_command["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
        let admission = host.action("workspace_register", new_command).await;
        fs::set_permissions(&workspace, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(replay.unwrap(), receipt);
        assert_eq!(admission.unwrap_err().0, "WORKSPACE_READ_ONLY");
        assert_eq!(fs::read(data.join("events.jsonl")).unwrap(), before);
    }

    fs::remove_dir_all(&workspace).unwrap();
    command["displayName"] = json!("Test repository");
    assert_eq!(
        host.action("workspace_register", command.clone())
            .await
            .unwrap(),
        receipt
    );
    for (field, changed) in [
        ("displayName", json!("Different name")),
        (
            "createdAt",
            json!(
                (chrono::DateTime::parse_from_rfc3339(command["createdAt"].as_str().unwrap())
                    .unwrap()
                    + chrono::Duration::seconds(1))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            ),
        ),
        (
            "expiresAt",
            json!(
                (chrono::DateTime::parse_from_rfc3339(command["expiresAt"].as_str().unwrap())
                    .unwrap()
                    + chrono::Duration::seconds(1))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            ),
        ),
    ] {
        let mut reused = command.clone();
        reused[field] = changed;
        assert_eq!(
            host.action("workspace_register", reused)
                .await
                .unwrap_err()
                .0,
            "IDEMPOTENCY_KEY_REUSED"
        );
        assert_eq!(fs::read(data.join("events.jsonl")).unwrap(), before);
    }
    let mut new_command = command;
    new_command["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    assert_eq!(
        host.action("workspace_register", new_command)
            .await
            .unwrap_err()
            .0,
        "WORKSPACE_FOLDER_NOT_FOUND"
    );
    assert_eq!(fs::read(data.join("events.jsonl")).unwrap(), before);
    host.shutdown().await;
}
