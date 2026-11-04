use super::*;
use std::{ffi::OsStr, os::unix::fs::PermissionsExt};

fn executable(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

fn node_bin(root: &Path, version: &str) -> PathBuf {
    let bin = root.join("versions/node").join(version).join("bin");
    executable(&bin.join("node"), "#!/bin/sh\nexit 0\n");
    bin
}

#[test]
fn nvm_discovery_is_numeric_bounded_and_keeps_path_precedence() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("nvm");
    let older = node_bin(&root, "v9.9.9");
    let current = node_bin(&root, "v24.12.0");
    let minor = node_bin(&root, "v24.2.0");
    for malformed in [
        "latest",
        "v25",
        "v024.12.0",
        "v25.0.0-rc.1",
        "v25.0.0+build",
    ] {
        node_bin(&root, malformed);
    }
    let incomplete = node_bin(&root, "v99.0.0");
    std::fs::set_permissions(
        incomplete.join("node"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert_eq!(nvm_bins(&root), vec![current.clone(), minor, older.clone()]);
    assert!(nvm_bins(&fixture.path().join("missing")).is_empty());
    std::fs::write(fixture.path().join("not-directory"), "fixture").unwrap();
    assert!(nvm_bins(&fixture.path().join("not-directory")).is_empty());

    executable(&older.join("codex"), "#!/bin/sh\nexit 0\n");
    assert_eq!(discover_in("codex", &nvm_bins(&root)), older.join("codex"));
    executable(&current.join("codex"), "#!/bin/sh\nexit 0\n");
    assert_eq!(
        discover_in("codex", &nvm_bins(&root)),
        current.join("codex")
    );
    let preferred = fixture.path().join("path-bin");
    executable(&preferred.join("codex"), "#!/bin/sh\nexit 0\n");
    let dirs: Vec<_> = std::iter::once(preferred.clone())
        .chain(nvm_bins(&root))
        .collect();
    assert_eq!(discover_in("codex", &dirs), preferred.join("codex"));
    std::fs::set_permissions(
        preferred.join("codex"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert_eq!(discover_in("codex", &dirs), current.join("codex"));

    for index in 0..129 {
        std::fs::write(
            root.join("versions/node").join(format!("extra-{index}")),
            "",
        )
        .unwrap();
    }
    assert!(nvm_bins(&root).is_empty());
}

#[test]
fn sibling_runtime_is_verified_and_only_changes_the_child_path() {
    let fixture = tempfile::tempdir().unwrap();
    let bin = node_bin(fixture.path(), "v24.12.0");
    let program = bin.join("codex");
    executable(&program, "#!/usr/bin/env node\n");
    let inherited = std::env::join_paths([
        PathBuf::from("/usr/bin"),
        bin.clone(),
        PathBuf::from("/bin"),
    ])
    .unwrap();
    let path = sibling_node_path(&program, &inherited).unwrap();
    assert_eq!(
        sibling_node_path(&program, OsStr::new("")).unwrap(),
        bin.as_os_str()
    );
    assert_eq!(
        std::env::split_paths(&path).collect::<Vec<_>>(),
        vec![
            bin.clone(),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin")
        ]
    );
    let original_environment = std::env::var_os("PATH");
    let cmd = command(&program, &["--version"]).unwrap();
    assert_eq!(cmd.as_std().get_program(), program.as_os_str());
    assert_eq!(
        cmd.as_std().get_args().collect::<Vec<_>>(),
        vec![OsStr::new("--version")]
    );
    let changed: Vec<_> = cmd.as_std().get_envs().collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, OsStr::new("PATH"));
    assert_eq!(
        std::env::split_paths(changed[0].1.unwrap()).next(),
        Some(bin.clone())
    );
    assert_eq!(std::env::var_os("PATH"), original_environment);
    assert!(sibling_node_path(Path::new("codex"), &inherited).is_none());
    std::fs::remove_file(bin.join("node")).unwrap();
    assert!(sibling_node_path(&program, &inherited).is_none());
    assert_eq!(
        command(&program, &["--version"])
            .unwrap()
            .as_std()
            .get_envs()
            .count(),
        0
    );
}

#[tokio::test]
async fn nvm_shim_runs_with_gui_path_without_a_shell_profile() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("nvm");
    let bin = node_bin(&root, "v24.12.0");
    let program = bin.join("codex");
    executable(&program, "#!/usr/bin/env node\n");
    executable(&bin.join("node"), "#!/bin/sh\ncase \"$2\" in --version) printf 'codex-cli 0.163.0\\n';; login) test \"$3\" = status;; *) exit 2;; esac\n");
    let empty_path = fixture.path().join("gui-path");
    std::fs::create_dir(&empty_path).unwrap();
    let selected = discover_in("codex", &nvm_bins(&root));
    assert_eq!(selected, program);

    let without_runtime = timeout(
        Duration::from_secs(4),
        Command::new(&program)
            .arg("--version")
            .env_clear()
            .env("PATH", &empty_path)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(without_runtime.status.code(), Some(127));
    for args in [&["--version"][..], &["login", "status"][..]] {
        let path = sibling_node_path(&selected, empty_path.as_os_str()).unwrap();
        let mut cmd = command(&selected, args).unwrap();
        cmd.env_clear().env("PATH", path);
        let result = timeout(Duration::from_secs(4), cmd.output())
            .await
            .unwrap()
            .unwrap();
        assert!(result.status.success());
        if args == ["--version"] {
            assert_eq!(result.stdout, b"codex-cli 0.163.0\n");
        }
    }
}
