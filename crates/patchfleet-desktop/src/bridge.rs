use crate::command::{safe_error, Command, Notice};
use crate::folder;
use eframe::egui;
use patchfleet_core::Host;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{runtime::Runtime, sync::mpsc, task::JoinHandle};

enum Request {
    Action(Command),
    Close,
}

enum Message {
    Snapshot(Result<Value, String>),
    Outcome(Command, Result<Value, String>, Result<Value, String>),
    Folder(folder::Target, Option<folder::Selection>),
    Closed,
}

pub struct Bridge {
    runtime: Runtime,
    requests: mpsc::Sender<Request>,
    messages: mpsc::Receiver<Message>,
    output: mpsc::Sender<Message>,
    picker: Option<JoinHandle<()>>,
    worker: Option<JoinHandle<()>>,
    host: Arc<Mutex<Option<Arc<Host>>>>,
    close_requested: Arc<AtomicBool>,
    pub snapshot: Option<Value>,
    memory_page: Option<Value>,
    pub error: Option<String>,
    pub feedback: Option<Notice>,
    pub picking_folder: bool,
    pub folder_selection: Option<(folder::Target, Option<folder::Selection>)>,
    pub pending: Option<Command>,
    pub busy: bool,
    pub closing: bool,
    pub closed: bool,
}

fn poll_delay(snapshot: &Value) -> Duration {
    let active_work = snapshot["work"]["items"].as_array().is_some_and(|items| {
        items.iter().any(|v| {
            matches!(
                v["status"].as_str(),
                Some("launching" | "running" | "cancelling")
            )
        })
    });
    let active_team = snapshot["teams"]["items"]
        .as_array()
        .is_some_and(|items| items.iter().any(|v| v["status"] == "active"));
    Duration::from_secs(
        if active_work || active_team || snapshot["cloud"]["paired"] == true {
            5
        } else {
            60
        },
    )
}

async fn notify(ctx: &egui::Context, output: &mpsc::Sender<Message>, message: Message) -> bool {
    let sent = output.send(message).await.is_ok();
    ctx.request_repaint();
    sent
}

impl Bridge {
    #[cfg(feature = "ui-smoke")]
    pub fn memory_smoke(&mut self) {
        if let Some(project) = self
            .snapshot
            .as_ref()
            .and_then(|s| s["projectMemory"]["projects"].as_array())
            .and_then(|projects| projects.first())
        {
            let id = project["projectId"].clone();
            self.submit(Command::new(
                "project_memory_read",
                serde_json::json!({"projectId":id,"offset":0,"limit":10,"query":""}),
            ));
            for _ in 0..400 {
                self.poll();
                if !self.busy {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    pub fn new(ctx: egui::Context, directory: PathBuf, observe: bool) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Could not create local async runtime");
        let (requests, mut input) = mpsc::channel(4);
        let (output, messages) = mpsc::channel(4);
        let picker_output = output.clone();
        let host_slot = Arc::new(Mutex::new(None));
        let host_worker = Arc::clone(&host_slot);
        let close_requested = Arc::new(AtomicBool::new(false));
        let worker_closing = Arc::clone(&close_requested);
        let worker = runtime.spawn(async move {
            let host = match Host::open(directory) {
                Ok(host) => host,
                Err(error) => {
                    notify(&ctx, &output, Message::Snapshot(Err(safe_error(error)))).await;
                    return;
                }
            };
            *host_worker.lock().unwrap() = Some(Arc::clone(&host));
            if worker_closing.load(Ordering::SeqCst) { host.request_shutdown(); }
            let mut snapshot = host.snapshot().map_err(safe_error);
            if !notify(&ctx, &output, Message::Snapshot(snapshot.clone())).await {
                host.shutdown().await;
                *host_worker.lock().unwrap() = None;
                return;
            }
            let timer = tokio::time::sleep(Duration::ZERO);
            let mut last_observation = None;
            tokio::pin!(timer);
            loop {
                tokio::select! {
                    biased;
                    request = input.recv() => match request {
                        Some(Request::Action(command)) => {
                            let result = host.action(command.action, command.payload.clone()).await.map_err(safe_error);
                            if command.action == "observe" { last_observation = Some(tokio::time::Instant::now()); }
                            snapshot = host.snapshot().map_err(safe_error);
                            if !notify(&ctx, &output, Message::Outcome(command, result, snapshot.clone())).await { break; }
                            let delay = snapshot.as_ref().map(poll_delay).unwrap_or(Duration::from_secs(60));
                            timer.as_mut().reset(tokio::time::Instant::now() + delay);
                        }
                        Some(Request::Close) | None => break,
                    },
                    _ = &mut timer, if observe => {
                        let probe = last_observation.is_none_or(|at: tokio::time::Instant| at.elapsed() >= Duration::from_secs(60));
                        let _ = host.background_tick(probe).await;
                        if probe { last_observation = Some(tokio::time::Instant::now()); }
                        snapshot = host.snapshot().map_err(safe_error);
                        if !notify(&ctx, &output, Message::Snapshot(snapshot.clone())).await { break; }
                        let delay = snapshot.as_ref().map(poll_delay).unwrap_or(Duration::from_secs(60));
                        timer.as_mut().reset(tokio::time::Instant::now() + delay);
                    }
                }
            }
            host.shutdown().await;
            *host_worker.lock().unwrap() = None;
            notify(&ctx, &output, Message::Closed).await;
        });
        Self {
            runtime,
            requests,
            messages,
            output: picker_output,
            picker: None,
            worker: Some(worker),
            host: host_slot,
            close_requested,
            snapshot: None,
            memory_page: None,
            error: None,
            feedback: None,
            picking_folder: false,
            folder_selection: None,
            pending: None,
            busy: false,
            closing: false,
            closed: false,
        }
    }

    pub fn poll(&mut self) {
        while let Ok(message) = self.messages.try_recv() {
            match message {
                Message::Snapshot(snapshot) => self.apply_snapshot(snapshot),
                Message::Outcome(command, result, snapshot) => {
                    match result {
                        Ok(receipt) => {
                            if command.action == "project_memory_read" {
                                self.memory_page = Some(receipt.clone());
                            }
                            self.pending = None;
                            self.feedback = (command.action != "project_memory_read")
                                .then(|| Notice::receipt(command.action, &receipt));
                        }
                        Err(error) => {
                            self.feedback = Some(Notice::error(command.action, &error));
                            self.pending = Some(command);
                        }
                    }
                    self.apply_snapshot(snapshot);
                    self.busy = false;
                }
                Message::Folder(target, selection) => {
                    self.picking_folder = false;
                    self.picker = None;
                    if !self.closing {
                        self.folder_selection = Some((target, selection));
                    }
                }
                Message::Closed => self.closed = true,
            }
        }
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) && self.snapshot.is_none() {
            self.busy = false;
        }
    }

    fn apply_snapshot(&mut self, snapshot: Result<Value, String>) {
        match snapshot {
            Ok(mut snapshot) => {
                if self.memory_page.as_ref().is_some_and(|page| {
                    !snapshot["projectMemory"]["projects"]
                        .as_array()
                        .is_some_and(|projects| {
                            projects.iter().any(|project| {
                                project["projectId"] == page["projectId"]
                                    && project["revision"] == page["revision"]
                            })
                        })
                }) {
                    self.memory_page = None;
                }
                snapshot["memoryPage"] = self.memory_page.clone().unwrap_or(Value::Null);
                self.snapshot = Some(snapshot);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn submit(&mut self, command: Command) {
        if self.busy || self.closing || self.picking_folder || self.error.is_some() {
            return;
        }
        match self.requests.try_send(Request::Action(command.clone())) {
            Ok(()) => {
                if command.action == "project_memory_read" {
                    self.memory_page = None;
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["memoryPage"] = Value::Null;
                    }
                }
                self.busy = true;
                self.pending = Some(command);
                self.feedback = None;
            }
            Err(_) => self.feedback = Some(Notice::error(command.action, "HOST_BUSY")),
        }
    }

    pub fn choose_folder(
        &mut self,
        ctx: egui::Context,
        target: folder::Target,
        choice: impl std::future::Future<Output = Option<PathBuf>> + Send + 'static,
    ) {
        if self.busy || self.closing || self.picking_folder || self.error.is_some() {
            return;
        }
        self.dismiss_feedback();
        self.picking_folder = true;
        ctx.request_repaint();
        let output = self.output.clone();
        self.picker = Some(self.runtime.spawn(async move {
            let selection = match choice.await {
                Some(path) => Some(folder::check(path).await),
                None => None,
            };
            notify(&ctx, &output, Message::Folder(target, selection)).await;
        }));
    }

    pub fn dismiss_feedback(&mut self) {
        self.feedback = None;
        if !self.busy {
            self.pending = None;
        }
    }

    pub fn begin_close(&mut self) {
        if !self.closing {
            self.closing = true;
            if let Some(picker) = self.picker.take() {
                picker.abort();
            }
            self.picking_folder = false;
            self.folder_selection = None;
            self.close_requested.store(true, Ordering::SeqCst);
            if let Some(host) = self.host.lock().unwrap().as_ref() {
                host.request_shutdown();
            }
            if self.requests.try_send(Request::Close).is_err() {
                self.closed = true;
            }
        }
    }

    pub fn shutdown(&mut self) {
        self.begin_close();
        self.messages.close();
        if let Some(worker) = self.worker.take() {
            let _ = self.runtime.block_on(worker);
        }
        self.closed = true;
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn idle_polling_is_sparse_and_active_work_stays_responsive() {
        assert_eq!(poll_delay(&json!({})), Duration::from_secs(60));
        assert_eq!(
            poll_delay(&json!({"work":{"items":[{"status":"queued"}]}})),
            Duration::from_secs(60)
        );
        assert_eq!(
            poll_delay(&json!({"work":{"items":[{"status":"running"}]}})),
            Duration::from_secs(5)
        );
        assert_eq!(
            poll_delay(&json!({"teams":{"items":[{"status":"active"}]}})),
            Duration::from_secs(5)
        );
        assert_eq!(
            poll_delay(&json!({"cloud":{"paired":true}})),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn fresh_host_failure_and_close_never_require_provider_processes() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        for _ in 0..200 {
            bridge.poll();
            if bridge.snapshot.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(bridge.snapshot.is_some());
        let host = bridge.host.lock().unwrap().as_ref().unwrap().clone();
        bridge.begin_close();
        assert_eq!(
            bridge
                .runtime
                .block_on(host.action("observe", json!({})))
                .unwrap_err()
                .0,
            "HOST_CLOSING"
        );
        drop(host);
        bridge.shutdown();
        assert!(bridge.closed);
        assert!(Host::open(directory.path().to_owned()).is_ok());
        let mut failed = Bridge::new(egui::Context::default(), PathBuf::from("relative"), false);
        for _ in 0..200 {
            failed.poll();
            if failed.error.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(failed.error.is_some());
        failed.shutdown();
    }

    fn await_idle(bridge: &mut Bridge) {
        for _ in 0..400 {
            bridge.poll();
            if !bridge.busy && bridge.snapshot.is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("Local bridge did not deliver its outcome");
    }

    #[test]
    fn folder_cancel_check_and_close_never_submit_an_owner_action() {
        let data = tempfile::tempdir().unwrap();
        let candidate = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), data.path().to_owned(), false);
        await_idle(&mut bridge);
        for choice in [None, Some(candidate.path().to_owned())] {
            bridge.choose_folder(
                egui::Context::default(),
                folder::Target::Registration,
                std::future::ready(choice.clone()),
            );
            for _ in 0..200 {
                bridge.poll();
                if !bridge.picking_folder {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(!bridge.picking_folder);
            let (target, result) = bridge.folder_selection.take().unwrap();
            assert_eq!(target, folder::Target::Registration);
            if choice.is_none() {
                assert!(result.is_none());
            } else {
                assert_eq!(
                    result.unwrap().problem.as_deref(),
                    Some("WORKSPACE_NOT_GIT_ROOT")
                );
            }
            assert!(bridge.pending.is_none());
            assert!(bridge.feedback.is_none());
            assert!(!data.path().join("events.jsonl").exists());
        }
        bridge.choose_folder(
            egui::Context::default(),
            folder::Target::Task,
            std::future::pending(),
        );
        bridge.submit(Command::new(
            "project_create",
            serde_json::json!({"name":"Must not be submitted"}),
        ));
        assert!(bridge.pending.is_none());
        bridge.begin_close();
        bridge.shutdown();
        assert!(!bridge.picking_folder);
        assert!(bridge.folder_selection.is_none());
        assert!(Host::open(data.path().to_owned()).is_ok());
        assert!(!data.path().join("events.jsonl").exists());
    }

    #[test]
    fn memory_page_tracks_project_revision_and_clears_before_a_new_read() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new("project_create", json!({"name":"Billing"})));
        await_idle(&mut bridge);
        let project = bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0].clone();
        bridge.submit(Command::new("project_message", json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],"title":"Amounts","text":"Use integer minor units."})));
        await_idle(&mut bridge);
        bridge.submit(Command::new(
            "project_memory_read",
            json!({"projectId":project["projectId"],"query":"integer","limit":10,"offset":0}),
        ));
        await_idle(&mut bridge);
        let page = &bridge.snapshot.as_ref().unwrap()["memoryPage"];
        assert_eq!(page["sources"][0]["text"], "Use integer minor units.");
        assert_eq!(page["query"], "integer");
        let current = bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0].clone();
        bridge.submit(Command::new("project_message", json!({"projectId":current["projectId"],"expectedProjectRevision":current["revision"],"title":"Currency","text":"Store the currency code."})));
        await_idle(&mut bridge);
        assert!(bridge.snapshot.as_ref().unwrap()["memoryPage"].is_null());
        bridge.submit(Command::new(
            "project_memory_read",
            json!({"projectId":project["projectId"],"query":"currency"}),
        ));
        assert!(bridge.snapshot.as_ref().unwrap()["memoryPage"].is_null());
        await_idle(&mut bridge);
        assert_eq!(bridge.snapshot.as_ref().unwrap()["memoryPage"]["total"], 1);
        bridge.shutdown();
    }
}
