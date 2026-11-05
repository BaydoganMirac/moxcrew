use super::*;

fn fixture() -> (tempfile::TempDir, PathBuf) {
    static BUILT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    let built=BUILT.get_or_init(||{
        let dir=tempfile::tempdir().unwrap().keep();let source=dir.join("typed.rs");
        std::fs::write(&source,r#"
use std::{env,fs,io::{self,Read,Write},process::{Command,Stdio},thread,time::Duration};
fn main(){let args:Vec<_>=env::args().collect();let root=std::path::Path::new(&args[2]);
 if args[1]=="child" {for _ in 0..2000{if root.join("escape-now").exists(){fs::write(root.join("escaped"),b"still running").unwrap();return}thread::sleep(Duration::from_millis(5));}return}
 let mut input=String::new();io::stdin().read_to_string(&mut input).unwrap();assert_eq!(input,"typed request");
 if args[1]=="success"{print!("typed output");return}
 if args[1]=="huge"{print!("{}","x".repeat(16384));return}
 if args[1]=="stderr"{eprint!("{}","x".repeat(16384));return}
 if args[1]=="exit"{std::process::exit(7)}
 let child=Command::new(env::current_exe().unwrap()).arg("child").arg(root).stdin(Stdio::null()).spawn().unwrap();
 fs::write(root.join("child"),child.id().to_string()).unwrap();fs::write(root.join("parent"),std::process::id().to_string()).unwrap();
 print!("partial");io::stdout().flush().unwrap();
 if args[1]=="inherited"{return}thread::sleep(Duration::from_secs(5));
}
"#).unwrap();
        let executable=dir.join(if cfg!(windows){"typed.exe"}else{"typed"});
        let rustc=std::env::var_os("RUSTC").unwrap_or_else(||PathBuf::from(env!("CARGO")).with_file_name(if cfg!(windows){"rustc.exe"}else{"rustc"}).into_os_string());
        assert!(std::process::Command::new(rustc).arg("--edition=2021").arg(source).arg("-o").arg(&executable).status().unwrap().success());executable
    });
    (tempfile::tempdir().unwrap(), built.clone())
}
fn command(executable: &Path, root: &Path, mode: &str) -> Command {
    let mut command = Command::new(executable);
    command.arg(mode).arg(root);
    command
}
async fn started(root: &Path) {
    timeout(Duration::from_secs(5), async {
        while !root.join("child").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn typed_worker_checks_exit_stream_budget_and_pre_dispatch_cancellation() {
    let (dir, exe) = fixture();
    let active = Arc::new(AtomicBool::new(true));
    assert_eq!(
        typed_process(
            command(&exe, dir.path(), "success"),
            b"typed request".to_vec(),
            active.clone(),
            Duration::from_secs(2),
            1024
        )
        .await
        .unwrap(),
        b"typed output"
    );
    assert!(active.load(Ordering::Acquire));
    for (mode, expected) in [
        ("huge", "EXTRACTION_INVALID_OUTPUT"),
        ("stderr", "EXTRACTION_INVALID_OUTPUT"),
        ("exit", "EXTRACTION_PROVIDER_FAILED"),
    ] {
        assert_eq!(
            typed_process(
                command(&exe, dir.path(), mode),
                b"typed request".to_vec(),
                active.clone(),
                Duration::from_secs(2),
                1024
            )
            .await
            .unwrap_err(),
            expected
        );
    }
    active.store(false, Ordering::Release);
    assert_eq!(
        typed_process(
            command(&exe, dir.path(), "hang"),
            b"typed request".to_vec(),
            active,
            Duration::from_secs(2),
            1024
        )
        .await
        .unwrap_err(),
        "EXTRACTION_CANCELLED"
    );
    assert!(!dir.path().join("child").exists());
}

#[tokio::test]
async fn typed_worker_reaps_descendants_after_parent_exit_cancel_and_dropped_caller() {
    for mode in ["inherited", "cancel", "drop"] {
        let (dir, exe) = fixture();
        let active = Arc::new(AtomicBool::new(true));
        let process = command(&exe, dir.path(), mode);
        let worker_active = active.clone();
        let bound = if mode == "inherited" {
            // The timeout starts before the fixture has spawned its descendant. Give
            // parallel test hosts room to schedule both processes; the tree-reap
            // assertions below still prove the timeout cleanup itself.
            Duration::from_secs(4)
        } else {
            Duration::from_secs(2)
        };
        let task = tokio::spawn(async move {
            typed_process(
                process,
                b"typed request".to_vec(),
                worker_active,
                bound,
                1024,
            )
            .await
        });
        started(dir.path()).await;
        let began = std::time::Instant::now();
        if mode == "cancel" {
            active.store(false, Ordering::Release);
        }
        if mode == "drop" {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        } else {
            assert_eq!(
                task.await.unwrap().unwrap_err(),
                if mode == "inherited" {
                    "EXTRACTION_TIMEOUT"
                } else {
                    "EXTRACTION_CANCELLED"
                }
            );
        }
        assert!(
            began.elapsed()
                < if mode == "inherited" {
                    Duration::from_secs(5)
                } else {
                    Duration::from_secs(1)
                }
        );
        tokio::time::sleep(Duration::from_millis(900)).await;
        std::fs::write(dir.path().join("escape-now"), b"go").unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(
            !dir.path().join("escaped").exists(),
            "descendant survived {mode}"
        );
        #[cfg(unix)]
        {
            let parent: i32 = std::fs::read_to_string(dir.path().join("parent"))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                unsafe { libc::kill(parent, 0) },
                -1,
                "owned parent was not reaped"
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
        }
    }
}
