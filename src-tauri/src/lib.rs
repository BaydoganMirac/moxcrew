use patchfleet_core::Host;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const ACTIONS: &[&str] = &[
    "observe",
    "workspace_register",
    "workspace_remove",
    "work_enqueue",
    "work_start",
    "work_retry",
    "work_cancel",
    "work_remove",
    "pack_install",
    "pack_remove",
    "team_create",
    "team_start",
    "team_advance",
    "team_cancel",
    "team_cancel_agent",
    "team_approve",
    "team_answer",
    "cloud_pair",
    "cloud_unpair",
    "cloud_sync",
];

#[derive(Default)]
struct Lifecycle {
    closing: AtomicBool,
    shutdown_complete: AtomicBool,
}

fn local_navigation(url: &tauri::Url) -> bool {
    let production_origin = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || ((url.scheme() == "http" || url.scheme() == "https")
            && url.host_str() == Some("tauri.localhost")
            && url.port().is_none());
    let dev_origin = cfg!(debug_assertions)
        && url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port() == Some(1420);
    (production_origin || dev_origin) && matches!(url.path(), "/" | "/index.html")
}

fn authorize(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main"
        || !local_navigation(&window.url().map_err(|_| "HOST_ACCESS_DENIED")?)
    {
        return Err("HOST_ACCESS_DENIED".into());
    }
    Ok(())
}

fn public_error(error: impl std::fmt::Display) -> String {
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

#[tauri::command]
fn host_snapshot(window: WebviewWindow, host: State<'_, Arc<Host>>) -> Result<Value, String> {
    authorize(&window)?;
    host.snapshot().map_err(public_error)
}

#[tauri::command]
async fn host_action(
    window: WebviewWindow,
    host: State<'_, Arc<Host>>,
    action: String,
    payload: Value,
) -> Result<Value, String> {
    authorize(&window)?;
    if window
        .app_handle()
        .state::<Arc<Lifecycle>>()
        .closing
        .load(Ordering::Acquire)
    {
        return Err("HOST_SHUTTING_DOWN".into());
    }
    if !ACTIONS.contains(&action.as_str())
        || !payload.is_object()
        || payload.to_string().len() > 131_072
    {
        return Err("INVALID_COMMAND".into());
    }
    host.action(&action, payload).await.map_err(public_error)
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let data_dir = match std::env::var_os("PATCHFLEET_DATA_DIR") {
                Some(path) if !path.is_empty() => PathBuf::from(path),
                Some(_) => return Err("PATCHFLEET_DATA_DIR must not be empty".into()),
                None => app.path().app_data_dir()?,
            };
            let host = Host::open(data_dir)?;
            app.manage(Arc::clone(&host));
            let lifecycle = Arc::new(Lifecycle::default());
            app.manage(Arc::clone(&lifecycle));
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Patchfleet")
                .inner_size(1280.0, 880.0)
                .min_inner_size(720.0, 540.0)
                .on_navigation(local_navigation)
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .build()?;
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    if lifecycle.closing.load(Ordering::Acquire) {
                        break;
                    }
                    if host.tick().await.is_err() {
                        eprintln!("Patchfleet background reconciliation did not complete");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![host_snapshot, host_action])
        .build(tauri::generate_context!())
        .expect("Patchfleet could not open its local desktop host");

    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let lifecycle = Arc::clone(app.state::<Arc<Lifecycle>>().inner());
            if lifecycle.shutdown_complete.load(Ordering::Acquire) {
                return;
            }
            api.prevent_exit();
            if lifecycle.closing.swap(true, Ordering::AcqRel) {
                return;
            }
            let host = Arc::clone(app.state::<Arc<Host>>().inner());
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = host.shutdown().await;
                lifecycle.shutdown_complete.store(true, Ordering::Release);
                app.exit(0);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_rejects_remote_pages_and_local_file_urls() {
        for url in [
            "https://cloud.example.com",
            "file:///tmp/index.html",
            "http://localhost:1420",
            "https://tauri.localhost.evil.test",
            "tauri://localhost/foreign.html",
        ] {
            assert!(!local_navigation(&url.parse().unwrap()), "{url}");
        }
        assert!(local_navigation(
            &"tauri://localhost/index.html#queue-work".parse().unwrap()
        ));
        assert!(local_navigation(
            &"http://tauri.localhost/".parse().unwrap()
        ));
    }

    #[test]
    fn private_error_details_never_cross_ipc() {
        assert_eq!(public_error("STALE_ITEM_REVISION"), "STALE_ITEM_REVISION");
        assert_eq!(
            public_error("could not read /Users/owner/private-token"),
            "HOST_ACTION_FAILED"
        );
    }
}
