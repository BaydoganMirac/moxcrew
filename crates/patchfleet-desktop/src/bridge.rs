use crate::command::{account_action, safe_error, Command, Notice, NoticeKind};
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
    time::{Duration, Instant},
};
use tokio::{runtime::Runtime, sync::mpsc, task::JoinHandle};

enum Request {
    Action(Command),
    Close,
}

fn memory_read(action: &str) -> bool {
    matches!(
        action,
        "project_memory_read"
            | "project_decisions_read"
            | "project_decision_read"
            | "memory_cloud_read"
            | "memory_cloud_decision_read"
            | "project_decision_search"
            | "memory_cloud_search"
    )
}

fn cloud_identity(snapshot: &Value) -> Value {
    let cloud = &snapshot["cloud"];
    serde_json::json!({"paired":cloud["paired"],"cloudUrl":cloud["cloudUrl"],"hostId":cloud["hostId"],"workspaceId":cloud["workspaceId"]})
}

enum Message {
    Snapshot(Result<Value, String>),
    Outcome(Command, Result<Value, String>, Result<Value, String>),
    Folder(folder::Target, Option<folder::Selection>),
    Account(u64, Command, Value, Result<Value, String>),
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
    context: egui::Context,
    account_task: Option<JoinHandle<()>>,
    account_pending: Option<Command>,
    account_generation: u64,
    account_connection: Option<Value>,
    account_refresh_connection: Option<Value>,
    account_auth_failure: Option<Value>,
    cloud_login: Option<Value>,
    cloud_account: Option<Value>,
    account_error: Option<Notice>,
    login_cloud_url: Option<String>,
    login_open_command: Option<String>,
    login_next_poll: Option<Instant>,
    logout_remote_revoked: Option<bool>,
    pub snapshot: Option<Value>,
    memory_page: Option<Value>,
    decision_page: Option<Value>,
    decision_detail: Option<Value>,
    shared_memory_page: Option<Value>,
    shared_decision_detail: Option<Value>,
    decision_search: Option<Value>,
    shared_search: Option<Value>,
    shared_search_connection: Option<Value>,
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
    pub fn account_smoke(&mut self, cloud_url: Option<&str>) {
        if let Some(url) = cloud_url {
            self.submit(Command::new(
                "cloud_login_begin",
                serde_json::json!({"cloudUrl":url,"displayName":"Native sign-in preview"}),
            ));
        } else if self
            .snapshot
            .as_ref()
            .is_some_and(|s| s["cloud"]["paired"] == true)
        {
            self.submit(Command::new("cloud_account_read", serde_json::json!({})));
        }
        for _ in 0..800 {
            self.poll();
            if self.account_task.is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[cfg(feature = "ui-smoke")]
    pub fn search_smoke(&mut self, shared: bool, query: &str) {
        let Some(project) = self.snapshot.as_ref().and_then(|s| {
            s[if shared {
                "sharedMemory"
            } else {
                "projectDecisions"
            }]["projects"]
                .as_array()
                .and_then(|rows| rows.first())
                .map(|p| p["projectId"].clone())
        }) else {
            return;
        };
        self.submit(Command::new(
            if shared {
                "memory_cloud_search"
            } else {
                "project_decision_search"
            },
            serde_json::json!({"projectId":project,"query":query,"limit":5}),
        ));
        for _ in 0..1600 {
            self.poll();
            if !self.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[cfg(feature = "ui-smoke")]
    pub fn decision_smoke(&mut self) {
        let Some(id) = self
            .snapshot
            .as_ref()
            .and_then(|s| s["projectMemory"]["projects"].as_array())
            .and_then(|projects| projects.first())
            .map(|p| p["projectId"].clone())
        else {
            return;
        };
        self.submit(Command::new(
            "project_decisions_read",
            serde_json::json!({"projectId":id,"offset":0,"limit":10,"query":"","status":"all"}),
        ));
        for _ in 0..400 {
            self.poll();
            if !self.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        if let Some(decision) = self
            .decision_page
            .as_ref()
            .and_then(|p| p["decisions"].as_array())
            .and_then(|d| {
                d.iter()
                    .find(|v| v["status"] == "accepted")
                    .or_else(|| d.first())
            })
        {
            let decision_id = decision["decisionId"].clone();
            self.submit(Command::new(
                "project_decision_read",
                serde_json::json!({"projectId":id,"decisionId":decision_id}),
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

    #[cfg(feature = "ui-smoke")]
    pub fn shared_smoke(&mut self) {
        let Some(project) = self
            .snapshot
            .as_ref()
            .and_then(|s| s["sharedMemory"]["projects"].as_array())
            .and_then(|ps| ps.iter().find(|p| p["sharedProjectId"].is_string()))
            .map(|p| p["projectId"].clone())
        else {
            return;
        };
        self.submit(Command::new(
            "memory_cloud_read",
            serde_json::json!({"projectId":project,"offset":0,"limit":10}),
        ));
        for _ in 0..400 {
            self.poll();
            if !self.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let first = self
            .shared_memory_page
            .as_ref()
            .and_then(|page| page["decisions"].as_array())
            .and_then(|rows| rows.first())
            .cloned();
        if let Some(first) = first {
            self.submit(Command::new("memory_cloud_decision_read",serde_json::json!({"projectId":project,"decisionId":first["decisionId"],"revision":first["revision"]})));
            for _ in 0..400 {
                self.poll();
                if !self.busy {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

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
        let account_context = ctx.clone();
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
            context: account_context,
            account_task: None,
            account_pending: None,
            account_generation: 0,
            account_connection: None,
            account_refresh_connection: None,
            account_auth_failure: None,
            cloud_login: None,
            cloud_account: None,
            account_error: None,
            login_cloud_url: None,
            login_open_command: None,
            login_next_poll: None,
            logout_remote_revoked: None,
            snapshot: None,
            memory_page: None,
            decision_page: None,
            decision_detail: None,
            shared_memory_page: None,
            shared_decision_detail: None,
            decision_search: None,
            shared_search: None,
            shared_search_connection: None,
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
                    let preserve_owner = memory_read(command.action)
                        && self
                            .pending
                            .as_ref()
                            .is_some_and(|pending| !memory_read(pending.action))
                        && self
                            .feedback
                            .as_ref()
                            .is_some_and(|notice| notice.kind == NoticeKind::Error);
                    if preserve_owner {
                        continue;
                    }
                    match result {
                        Ok(receipt) => {
                            match command.action {
                                "project_memory_read" => self.memory_page = Some(receipt.clone()),
                                "project_decisions_read" => {
                                    self.decision_page = Some(receipt.clone())
                                }
                                "project_decision_read" => {
                                    self.decision_detail = Some(receipt.clone())
                                }
                                "memory_cloud_read" => {
                                    self.shared_memory_page = Some(receipt.clone())
                                }
                                "memory_cloud_decision_read" => {
                                    self.shared_decision_detail = Some(receipt.clone())
                                }
                                "project_decision_search" => {
                                    self.decision_search = Some(receipt.clone())
                                }
                                "memory_cloud_search" => self.shared_search = Some(receipt.clone()),
                                _ => (),
                            }
                            self.pending = None;
                            self.feedback = (!memory_read(command.action))
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
                Message::Account(generation, command, connection, result) => {
                    self.account_outcome(generation, command, connection, result)
                }
                Message::Closed => self.closed = true,
            }
        }
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) && self.snapshot.is_none() {
            self.busy = false;
        }
        if !self.closing && self.account_task.is_none() {
            let connection = self.snapshot.as_ref().map(cloud_identity);
            if self
                .cloud_login
                .as_ref()
                .is_some_and(|v| v["status"] == "waiting")
            {
                let next = self.login_next_poll.unwrap_or_else(Instant::now);
                if Instant::now() >= next {
                    self.submit_account(Command::new("cloud_login_poll", serde_json::json!({})));
                } else {
                    self.context
                        .request_repaint_after(next.saturating_duration_since(Instant::now()));
                }
            } else if connection.as_ref().is_some_and(|c| c["paired"] == true)
                && self.account_refresh_connection != connection
                && self.error.is_none()
            {
                self.account_refresh_connection = connection;
                self.submit_account(Command::new("cloud_account_read", serde_json::json!({})));
            }
        }
    }

    fn account_overlays(&mut self) {
        if let Some(snapshot) = self.snapshot.as_mut() {
            snapshot["cloudLogin"] = self.cloud_login.clone().unwrap_or(Value::Null);
            snapshot["cloudAccount"] = self.cloud_account.clone().unwrap_or(Value::Null);
            snapshot["cloudAccountUi"] = serde_json::json!({"busy":self.account_task.is_some(),"action":self.account_pending.as_ref().map(|c|c.action),"error":self.account_error.as_ref().map(|n|n.code.as_str()),"errorAction":self.account_error.as_ref().map(|n|n.action),"loginCloudUrl":self.login_cloud_url,"openCommandId":self.login_open_command,"logoutRemoteRevoked":self.logout_remote_revoked});
        }
    }

    fn submit_account(&mut self, command: Command) {
        if self.closing || self.error.is_some() {
            return;
        }
        if self.account_task.is_some() {
            if !matches!(command.action, "cloud_login_cancel" | "cloud_logout") {
                return;
            }
            self.account_task.take().unwrap().abort();
        }
        let Some(host) = self.host.lock().unwrap().as_ref().cloned() else {
            return;
        };
        let Some(connection) = self.snapshot.as_ref().map(cloud_identity) else {
            return;
        };
        self.account_generation += 1;
        let generation = self.account_generation;
        if command.action == "cloud_login_begin" {
            self.cloud_login = None;
            self.login_cloud_url = command.payload["cloudUrl"].as_str().map(str::to_owned);
            self.login_open_command = None;
            self.logout_remote_revoked = None;
        }
        if matches!(command.action, "cloud_login_cancel" | "cloud_logout") {
            self.login_next_poll = None;
            self.login_open_command = None;
        }
        if command.action == "cloud_account_read" {
            self.cloud_account = None;
            self.account_refresh_connection = Some(connection.clone());
        }
        self.account_error = None;
        self.account_pending = Some(command.clone());
        let ctx = self.context.clone();
        let output = self.output.clone();
        self.account_task = Some(self.runtime.spawn(async move {
            let result = host
                .action(command.action, command.payload.clone())
                .await
                .map_err(safe_error);
            notify(
                &ctx,
                &output,
                Message::Account(generation, command, connection, result),
            )
            .await;
        }));
        self.account_overlays();
    }

    fn account_outcome(
        &mut self,
        generation: u64,
        command: Command,
        connection: Value,
        result: Result<Value, String>,
    ) {
        if self.closing
            || generation != self.account_generation
            || self.account_pending.as_ref().is_none_or(|pending| {
                pending.payload != command.payload || pending.action != command.action
            })
        {
            return;
        }
        let snapshot = self
            .host
            .lock()
            .unwrap()
            .as_ref()
            .cloned()
            .and_then(|host| host.snapshot().ok());
        let Some(snapshot) = snapshot else {
            self.account_task = None;
            self.account_pending = None;
            return;
        };
        let current = cloud_identity(&snapshot);
        let transition = result.as_ref().is_ok_and(|v| {
            (command.action == "cloud_login_poll"
                && v["status"] == "connected"
                && current["paired"] == true)
                || (command.action == "cloud_logout"
                    && v["localSignedOut"] == true
                    && current["paired"] == false)
        });
        self.account_task = None;
        self.account_pending = None;
        if current != connection && !transition {
            self.apply_snapshot(Ok(snapshot));
            return;
        }
        self.apply_snapshot(Ok(snapshot));
        match result {
            Ok(value) => match command.action {
                "cloud_login_begin" | "cloud_login_poll" | "cloud_login_cancel" => {
                    if value["status"] == "waiting" {
                        if command.action == "cloud_login_begin" {
                            self.login_open_command =
                                command.payload["commandId"].as_str().map(str::to_owned);
                        }
                        self.login_next_poll = Some(Instant::now() + Duration::from_secs(3));
                        self.context.request_repaint_after(Duration::from_secs(3));
                    } else {
                        self.login_next_poll = None;
                        self.login_open_command = None;
                    }
                    self.cloud_login = Some(value);
                }
                "cloud_account_read" => {
                    if patchfleet_core::desktop_login::validate_account_response(
                        &value,
                        &self.snapshot.as_ref().unwrap()["cloud"],
                    )
                    .is_ok()
                    {
                        self.cloud_account = Some(value);
                    } else {
                        self.account_error =
                            Some(Notice::error(command.action, "DESKTOP_ACCOUNT_INVALID"));
                    }
                }
                "cloud_logout" => {
                    self.cloud_login = None;
                    self.cloud_account = None;
                    self.logout_remote_revoked = value["remoteRevoked"].as_bool();
                }
                _ => (),
            },
            Err(error) => {
                self.account_error = Some(Notice::error(command.action, &error));
                self.cloud_account = None;
                if command.action.starts_with("cloud_login_") {
                    self.cloud_login = None;
                    self.login_next_poll = None;
                    self.login_open_command = None;
                }
            }
        }
        self.account_overlays();
    }

    fn apply_snapshot(&mut self, snapshot: Result<Value, String>) {
        match snapshot {
            Ok(mut snapshot) => {
                let connection = cloud_identity(&snapshot);
                if self
                    .account_connection
                    .as_ref()
                    .is_some_and(|previous| *previous != connection)
                {
                    if let Some(task) = self.account_task.take() {
                        task.abort();
                    }
                    self.account_generation += 1;
                    self.account_pending = None;
                    self.cloud_account = None;
                    self.cloud_login = None;
                    self.account_error = None;
                    self.login_next_poll = None;
                    self.login_open_command = None;
                    self.login_cloud_url = None;
                    self.logout_remote_revoked = None;
                    self.account_refresh_connection = None;
                    self.account_auth_failure = None;
                }
                if snapshot["cloud"]["lastErrorCode"] == "CLOUD_AUTH_REJECTED" {
                    let failure = serde_json::json!({"connection":connection,"at":snapshot["cloud"]["lastAttemptAt"]});
                    if self.account_auth_failure.as_ref() != Some(&failure) {
                        self.cloud_account = None;
                        self.account_error =
                            Some(Notice::error("cloud_account_read", "DESKTOP_ACCESS_DENIED"));
                        self.account_auth_failure = Some(failure);
                    }
                } else {
                    self.account_auth_failure = None;
                }
                self.account_connection = Some(connection);
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
                for page in [
                    &mut self.decision_page,
                    &mut self.decision_detail,
                    &mut self.decision_search,
                ] {
                    if page.as_ref().is_some_and(|page| {
                        !snapshot["projectDecisions"]["projects"]
                            .as_array()
                            .is_some_and(|projects| {
                                projects.iter().any(|project| {
                                    project["projectId"] == page["projectId"]
                                        && project["revision"] == page["revision"]
                                })
                            })
                    }) {
                        *page = None;
                    }
                }
                snapshot["decisionPage"] = self.decision_page.clone().unwrap_or(Value::Null);
                snapshot["decisionDetail"] = self.decision_detail.clone().unwrap_or(Value::Null);
                snapshot["decisionSearch"] = self.decision_search.clone().unwrap_or(Value::Null);
                for page in [
                    &mut self.shared_memory_page,
                    &mut self.shared_decision_detail,
                ] {
                    if page.as_ref().is_some_and(|page| {
                        page["revision"] != snapshot["sharedMemory"]["revision"]
                            || !snapshot["sharedMemory"]["projects"].as_array().is_some_and(
                                |projects| {
                                    projects.iter().any(|project| {
                                        project["projectId"] == page["projectId"]
                                            && project["sharedProjectId"] == page["sharedProjectId"]
                                    })
                                },
                            )
                    }) {
                        *page = None;
                    }
                }
                snapshot["sharedMemoryPage"] =
                    self.shared_memory_page.clone().unwrap_or(Value::Null);
                snapshot["sharedDecisionDetail"] =
                    self.shared_decision_detail.clone().unwrap_or(Value::Null);
                if self.shared_search.as_ref().is_some_and(|page| {
                    self.shared_search_connection.as_ref() != Some(&cloud_identity(&snapshot))
                        || page["revision"] != snapshot["sharedMemory"]["revision"]
                        || !snapshot["sharedMemory"]["projects"].as_array().is_some_and(
                            |projects| {
                                projects.iter().any(|project| {
                                    project["projectId"] == page["projectId"]
                                        && project["sharedProjectId"] == page["sharedProjectId"]
                                        && project["state"] != "connection_changed"
                                        && project["enabled"] == true
                                        && page["manifestRevision"]
                                            .as_u64()
                                            .zip(project["sharedRevision"].as_u64())
                                            .is_some_and(|(found, current)| found >= current)
                                })
                            },
                        )
                }) {
                    self.shared_search = None;
                }
                snapshot["sharedSearch"] = self.shared_search.clone().unwrap_or(Value::Null);
                self.snapshot = Some(snapshot);
                self.account_overlays();
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn submit(&mut self, command: Command) {
        if account_action(command.action) {
            self.submit_account(command);
            return;
        }
        if self.busy || self.closing || self.picking_folder || self.error.is_some() {
            return;
        }
        // Reads cannot replace a failed owner's exact retry identity.
        if memory_read(command.action)
            && self
                .pending
                .as_ref()
                .is_some_and(|pending| !memory_read(pending.action))
            && self
                .feedback
                .as_ref()
                .is_some_and(|notice| notice.kind == NoticeKind::Error)
        {
            return;
        }
        match self.requests.try_send(Request::Action(command.clone())) {
            Ok(()) => {
                for (result, key) in [
                    (&mut self.decision_search, "decisionSearch"),
                    (&mut self.shared_search, "sharedSearch"),
                ] {
                    if result.as_ref().is_some_and(|page| {
                        command.payload["projectId"].is_string()
                            && (page["projectId"] != command.payload["projectId"]
                                || (matches!(
                                    command.action,
                                    "project_decision_search" | "memory_cloud_search"
                                ) && page["query"].as_str()
                                    != command.payload["query"].as_str().map(str::trim)))
                    }) {
                        *result = None;
                    }
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot[key] = result.clone().unwrap_or(Value::Null);
                    }
                }
                if command.action == "project_decision_search" {
                    self.decision_search = None;
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["decisionSearch"] = Value::Null;
                    }
                }
                if command.action == "memory_cloud_search" {
                    self.shared_search = None;
                    self.shared_search_connection = self.snapshot.as_ref().map(cloud_identity);
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["sharedSearch"] = Value::Null;
                    }
                }
                if matches!(
                    command.action,
                    "memory_cloud_read" | "memory_cloud_decision_read"
                ) {
                    for page in [
                        &mut self.shared_memory_page,
                        &mut self.shared_decision_detail,
                    ] {
                        if page
                            .as_ref()
                            .is_some_and(|page| page["projectId"] != command.payload["projectId"])
                        {
                            *page = None;
                        }
                    }
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["sharedMemoryPage"] =
                            self.shared_memory_page.clone().unwrap_or(Value::Null);
                        snapshot["sharedDecisionDetail"] =
                            self.shared_decision_detail.clone().unwrap_or(Value::Null);
                    }
                }
                if command.action == "project_memory_read" {
                    self.memory_page = None;
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["memoryPage"] = Value::Null;
                    }
                }
                if command.action == "project_decisions_read" {
                    self.decision_page = None;
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["decisionPage"] = Value::Null;
                    }
                }
                if command.action == "project_decision_read" {
                    self.decision_detail = None;
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["decisionDetail"] = Value::Null;
                    }
                }
                if command.action == "memory_cloud_read" {
                    self.shared_memory_page = None;
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["sharedMemoryPage"] = Value::Null;
                    }
                }
                if command.action == "memory_cloud_decision_read" {
                    self.shared_decision_detail = None;
                    if let Some(snapshot) = self.snapshot.as_mut() {
                        snapshot["sharedDecisionDetail"] = Value::Null;
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
            if let Some(task) = self.account_task.take() {
                task.abort();
            }
            self.account_generation += 1;
            self.account_pending = None;
            self.cloud_login = None;
            self.cloud_account = None;
            self.login_next_poll = None;
            self.login_open_command = None;
            self.login_cloud_url = None;
            self.account_overlays();
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

    fn await_account(bridge: &mut Bridge) {
        for _ in 0..800 {
            bridge.poll();
            if bridge.account_task.is_none() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("Account lane did not finish");
    }

    #[test]
    fn browser_sign_in_waits_in_a_separate_lane_and_never_blocks_local_commands() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new("cloud_login_begin",serde_json::json!({"cloudUrl":"https://cloud.example.invalid","displayName":"Local test"})));
        assert!(!bridge.busy);
        assert!(bridge.pending.is_none());
        assert!(bridge.account_task.is_some());
        bridge.submit(Command::new(
            "project_create",
            serde_json::json!({"name":"Local still works"}),
        ));
        await_idle(&mut bridge);
        await_account(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0]["name"],
            "Local still works"
        );
        let login = bridge.cloud_login.as_ref().unwrap().clone();
        assert_eq!(login["status"], "waiting");
        assert!(patchfleet_core::desktop_login::validate_authorization_url(
            login["authorizationUrl"].as_str().unwrap(),
            "https://cloud.example.invalid"
        )
        .is_ok());
        let generation = bridge.account_generation;
        for _ in 0..25 {
            bridge.poll();
        }
        assert_eq!(bridge.account_generation, generation);
        bridge.login_next_poll = Some(Instant::now() - Duration::from_secs(1));
        bridge.poll();
        assert!(bridge.account_task.is_some());
        assert!(!bridge.busy);
        bridge.submit(Command::new("cloud_login_cancel", serde_json::json!({})));
        await_account(&mut bridge);
        assert_eq!(bridge.cloud_login.as_ref().unwrap()["status"], "cancelled");
        assert!(bridge.login_next_poll.is_none());
        let host = bridge.host.lock().unwrap().as_ref().unwrap().clone();
        assert!(host.snapshot().unwrap().get("cloudLogin").is_none());
        for entry in std::fs::read_dir(directory.path()).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                let bytes = std::fs::read(entry.path()).unwrap();
                let text = String::from_utf8_lossy(&bytes);
                assert!(
                    !text.contains("authorizationUrl")
                        && !text.contains("codeChallenge")
                        && !text.contains("confirmationCode")
                );
            }
        }
        bridge.shutdown();
        assert!(bridge.cloud_login.is_none());
    }

    #[test]
    fn account_failure_and_stale_outcomes_preserve_owner_retry_identity() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let owner = Command::new(
            "project_decision_propose",
            serde_json::json!({"title":"Keep draft"}),
        );
        bridge.pending = Some(owner.clone());
        bridge.feedback = Some(Notice::error(owner.action, "HOST_ACTION_FAILED"));
        bridge.submit(Command::new("cloud_account_read", serde_json::json!({})));
        await_account(&mut bridge);
        assert!(bridge.account_error.is_some());
        assert_eq!(bridge.pending.as_ref().unwrap().payload, owner.payload);
        assert_eq!(bridge.feedback.as_ref().unwrap().code, "HOST_ACTION_FAILED");
        assert!(!bridge.busy);
        let before = bridge.snapshot.clone();
        let stale = Command::new(
            "cloud_login_begin",
            serde_json::json!({"cloudUrl":"https://other.example.invalid"}),
        );
        bridge.output.blocking_send(Message::Account(bridge.account_generation.saturating_sub(1),stale,serde_json::json!({"paired":false}),Ok(serde_json::json!({"status":"waiting","authorizationUrl":"https://evil.invalid"})))).unwrap();
        bridge.poll();
        assert_eq!(bridge.snapshot, before);
        assert_eq!(bridge.pending.as_ref().unwrap().payload, owner.payload);
        bridge.shutdown();
    }

    #[test]
    fn account_overlays_clear_on_connection_change_and_close() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.cloud_login = Some(serde_json::json!({"status":"waiting"}));
        bridge.cloud_account = Some(serde_json::json!({"email":"owner@example.invalid"}));
        bridge.login_next_poll = Some(Instant::now() + Duration::from_secs(3));
        bridge.login_open_command = Some("cmd:begin".into());
        bridge.login_cloud_url = Some("https://earlier.example.invalid".into());
        bridge.logout_remote_revoked = Some(false);
        let mut snapshot = bridge.snapshot.as_ref().unwrap().clone();
        snapshot["cloud"] = serde_json::json!({"paired":true,"cloudUrl":"https://cloud.example.invalid","hostId":"host:new","workspaceId":"workspace:new"});
        bridge.apply_snapshot(Ok(snapshot));
        assert!(bridge.cloud_account.is_none() && bridge.cloud_login.is_none());
        assert!(bridge.login_next_poll.is_none() && bridge.login_open_command.is_none());
        assert!(bridge.login_cloud_url.is_none() && bridge.logout_remote_revoked.is_none());
        assert!(bridge.snapshot.as_ref().unwrap()["cloudAccount"].is_null());
        bridge.cloud_account = Some(serde_json::json!({"email":"earlier@example.invalid"}));
        let mut rejected = bridge.snapshot.as_ref().unwrap().clone();
        rejected["cloud"]["lastErrorCode"] = serde_json::json!("CLOUD_AUTH_REJECTED");
        rejected["cloud"]["lastAttemptAt"] = serde_json::json!("2026-10-15T01:00:00.000Z");
        bridge.apply_snapshot(Ok(rejected));
        assert!(bridge.cloud_account.is_none());
        assert_eq!(
            bridge.account_error.as_ref().unwrap().code,
            "DESKTOP_ACCESS_DENIED"
        );
        bridge.begin_close();
        assert!(bridge.account_task.is_none());
        bridge.shutdown();
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

    #[test]
    fn decision_pages_are_ephemeral_scoped_and_invalidated_on_manifest_change() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new("project_create", json!({"name":"Decisions"})));
        await_idle(&mut bridge);
        let project =
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0]["projectId"].clone();
        bridge.submit(Command::new(
            "project_decisions_read",
            json!({"projectId":project}),
        ));
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["decisionPage"]["total"],
            0
        );
        assert!(bridge.feedback.is_none());
        bridge.submit(Command::new("project_message", json!({"projectId":project,"expectedProjectRevision":1,"title":"Rule","text":"Integer money"})));
        await_idle(&mut bridge);
        let source = bridge.feedback.as_ref().unwrap();
        assert_eq!(source.code, "PROJECT_MESSAGE_CREATED");
        bridge.submit(Command::new(
            "project_memory_read",
            json!({"projectId":project}),
        ));
        await_idle(&mut bridge);
        let source =
            bridge.snapshot.as_ref().unwrap()["memoryPage"]["sources"][0]["sourceId"].clone();
        bridge.submit(Command::new("project_decision_propose", json!({"projectId":project,"expectedProjectRevision":2,
            "title":"Amounts","context":"Source-backed","decision":"Use integer money","rationale":"Exact totals","consequences":"Convert at boundaries",
            "sources":[{"sourceId":source,"revision":1}]})));
        await_idle(&mut bridge);
        assert!(bridge.snapshot.as_ref().unwrap()["decisionPage"].is_null());
        bridge.submit(Command::new(
            "project_decisions_read",
            json!({"projectId":project,"query":"money"}),
        ));
        assert!(bridge.snapshot.as_ref().unwrap()["decisionPage"].is_null());
        await_idle(&mut bridge);
        let decision =
            bridge.snapshot.as_ref().unwrap()["decisionPage"]["decisions"][0]["decisionId"].clone();
        bridge.submit(Command::new(
            "project_decision_read",
            json!({"projectId":project,"decisionId":decision}),
        ));
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["decisionDetail"]["decision"]["status"],
            "proposed"
        );
        assert!(bridge.feedback.is_none());
        let retry = Command::new(
            "project_decision_accept",
            json!({"projectId":project,"decisionId":decision,"expectedDecisionRevision":1}),
        );
        bridge.submit(retry.clone());
        await_idle(&mut bridge);
        assert_eq!(
            bridge.feedback.as_ref().unwrap().code,
            "PROJECT_DECISION_ACCEPTED"
        );
        for key in ["decisionPage", "decisionDetail"] {
            assert!(bridge.snapshot.as_ref().unwrap()[key].is_null());
        }
        let manifest = bridge.snapshot.as_ref().unwrap()["projectDecisions"].clone();
        bridge.submit(retry);
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectDecisions"],
            manifest
        );
        bridge.submit(Command::new(
            "project_decision_read",
            json!({"projectId":project,"decisionId":decision}),
        ));
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["decisionDetail"]["decision"]["status"],
            "accepted"
        );
        let mut stale_snapshot = bridge.snapshot.as_ref().unwrap().clone();
        stale_snapshot["projectDecisions"]["projects"] = json!([]);
        bridge.apply_snapshot(Ok(stale_snapshot));
        assert!(bridge.snapshot.as_ref().unwrap()["decisionDetail"].is_null());
        bridge.shutdown();
    }

    #[test]
    fn shared_read_outcome_is_atomic_scoped_and_invalidated_by_global_revision() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let mut snapshot = bridge.snapshot.as_ref().unwrap().clone();
        snapshot["sharedMemory"] = json!({"schemaVersion":5,"revision":12,"projects":[{"projectId":"project:one","sharedProjectId":"shared:one"}]});
        let page = json!({"reasonCode":"SHARED_MEMORY_READ","projectId":"project:one","sharedProjectId":"shared:one","revision":12,"offset":0,"limit":10,"total":1,"decisions":[{"decisionId":"decision:one","revision":2}]});
        let detail = json!({"reasonCode":"SHARED_DECISION_READ","projectId":"project:one","sharedProjectId":"shared:one","revision":12,"decision":{"decisionId":"decision:one","revision":2,"decision":"Downloaded owner text"}});
        for (action, receipt) in [
            ("memory_cloud_read", page.clone()),
            ("memory_cloud_decision_read", detail.clone()),
        ] {
            bridge.busy = true;
            bridge.pending = Some(Command::new(action, json!({"projectId":"project:one"})));
            bridge
                .output
                .blocking_send(Message::Outcome(
                    bridge.pending.clone().unwrap(),
                    Ok(receipt),
                    Ok(snapshot.clone()),
                ))
                .unwrap();
            bridge.poll();
            assert!(!bridge.busy);
            assert!(bridge.pending.is_none());
            assert!(bridge.feedback.is_none());
        }
        assert_eq!(bridge.snapshot.as_ref().unwrap()["sharedMemoryPage"], page);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["sharedDecisionDetail"],
            detail
        );
        let mut changed = snapshot.clone();
        changed["sharedMemory"]["revision"] = json!(13);
        bridge.apply_snapshot(Ok(changed));
        assert!(bridge.shared_memory_page.is_none());
        assert!(bridge.shared_decision_detail.is_none());
        for (project, shared) in [
            ("project:foreign", "shared:one"),
            ("project:one", "shared:foreign"),
        ] {
            bridge.shared_memory_page = Some(page.clone());
            bridge.shared_decision_detail = Some(detail.clone());
            let mut foreign = snapshot.clone();
            foreign["sharedMemory"]["projects"][0]["projectId"] = json!(project);
            foreign["sharedMemory"]["projects"][0]["sharedProjectId"] = json!(shared);
            bridge.apply_snapshot(Ok(foreign));
            assert!(bridge.snapshot.as_ref().unwrap()["sharedMemoryPage"].is_null());
            assert!(bridge.snapshot.as_ref().unwrap()["sharedDecisionDetail"].is_null());
        }
        bridge.shared_memory_page = Some(page);
        bridge.shared_decision_detail = Some(detail);
        bridge.apply_snapshot(Ok(snapshot));
        bridge.submit(Command::new(
            "memory_cloud_read",
            json!({"projectId":"project:other"}),
        ));
        assert!(bridge.snapshot.as_ref().unwrap()["sharedMemoryPage"].is_null());
        assert!(bridge.snapshot.as_ref().unwrap()["sharedDecisionDetail"].is_null());
        await_idle(&mut bridge);
        bridge.shutdown();
    }

    #[test]
    fn unpaired_shared_list_reads_are_local_and_errors_keep_explicit_read_retry() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new("project_create", json!({"name":"Local only"})));
        await_idle(&mut bridge);
        let project =
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0]["projectId"].clone();
        bridge.submit(Command::new(
            "memory_cloud_read",
            json!({"projectId":project,"offset":0,"limit":10}),
        ));
        await_idle(&mut bridge);
        let page = &bridge.snapshot.as_ref().unwrap()["sharedMemoryPage"];
        assert_eq!(page["reasonCode"], "SHARED_MEMORY_READ");
        assert_eq!(page["total"], 0);
        assert!(page["sharedProjectId"].is_null());
        assert!(bridge.feedback.is_none());
        assert_eq!(bridge.snapshot.as_ref().unwrap()["cloud"]["paired"], false);
        let read = Command::new(
            "memory_cloud_decision_read",
            json!({"projectId":project,"decisionId":format!("decision:{}",uuid::Uuid::new_v4()),"revision":1}),
        );
        bridge.submit(read.clone());
        await_idle(&mut bridge);
        assert_eq!(bridge.feedback.as_ref().unwrap().kind, NoticeKind::Error);
        assert_eq!(bridge.feedback.as_ref().unwrap().code, "MEMORY_NOT_CACHED");
        assert_eq!(bridge.pending.as_ref().unwrap().payload, read.payload);
        assert!(bridge.snapshot.as_ref().unwrap()["sharedDecisionDetail"].is_null());
        bridge.shutdown();
    }

    #[test]
    fn durable_mutation_cache_failure_keeps_exact_retry_until_owner_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new("project_create", json!({"name":"Recovery"})));
        await_idle(&mut bridge);
        let project =
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0]["projectId"].clone();
        bridge.submit(Command::new(
            "project_message",
            json!({"projectId":project,"expectedProjectRevision":1,
            "title":"Evidence","text":"Use exact totals."}),
        ));
        await_idle(&mut bridge);
        bridge.submit(Command::new(
            "project_memory_read",
            json!({"projectId":project}),
        ));
        await_idle(&mut bridge);
        let source =
            bridge.snapshot.as_ref().unwrap()["memoryPage"]["sources"][0]["sourceId"].clone();
        let proposal = Command::new(
            "project_decision_propose",
            json!({"projectId":project,"expectedProjectRevision":2,
            "title":"Amounts","context":"","decision":"Use integer amounts.","rationale":"","consequences":"",
            "sources":[{"sourceId":source,"revision":1}]}),
        );
        let cache = directory.path().join("project-decisions.json");
        std::fs::remove_file(&cache).unwrap();
        std::fs::create_dir(&cache).unwrap();
        bridge.submit(proposal.clone());
        await_idle(&mut bridge);
        assert_eq!(bridge.feedback.as_ref().unwrap().kind, NoticeKind::Error);
        assert!(bridge.feedback.as_ref().unwrap().retryable());
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectDecisions"]["projects"][0]["revision"],
            1
        );
        let failure = bridge.feedback.as_ref().unwrap().code.clone();
        let events = std::fs::read(directory.path().join("events.jsonl")).unwrap();
        for action in [
            "project_memory_read",
            "project_decisions_read",
            "project_decision_read",
            "memory_cloud_read",
            "memory_cloud_decision_read",
            "project_decision_search",
            "memory_cloud_search",
        ] {
            bridge.submit(Command::new(action, json!({"projectId":project})));
            assert!(!bridge.busy);
            assert_eq!(bridge.pending.as_ref().unwrap().payload, proposal.payload);
            assert_eq!(bridge.feedback.as_ref().unwrap().code, failure);
        }
        std::fs::remove_dir(cache).unwrap();
        bridge.submit(bridge.pending.as_ref().unwrap().clone());
        assert_eq!(bridge.pending.as_ref().unwrap().payload, proposal.payload);
        await_idle(&mut bridge);
        assert_eq!(
            bridge.feedback.as_ref().unwrap().code,
            "PROJECT_DECISION_PROPOSED"
        );
        assert_eq!(
            std::fs::read(directory.path().join("events.jsonl")).unwrap(),
            events
        );
        assert!(bridge.pending.is_none());
        bridge.submit(Command::new(
            "project_decisions_read",
            json!({"projectId":project}),
        ));
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["decisionPage"]["total"],
            1
        );
        bridge.shutdown();
    }

    #[test]
    fn local_search_is_an_ephemeral_read_and_query_replacement_clears_it() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new("project_create", json!({"name":"Search"})));
        await_idle(&mut bridge);
        let project =
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0]["projectId"].clone();
        let events = std::fs::read(directory.path().join("events.jsonl")).unwrap();
        bridge.submit(Command::new(
            "project_decision_search",
            json!({"projectId":project,"query":"  PRIVATE_SEARCH_CANARY  ","limit":5}),
        ));
        await_idle(&mut bridge);
        let result = &bridge.snapshot.as_ref().unwrap()["decisionSearch"];
        assert_eq!(result["query"], "PRIVATE_SEARCH_CANARY");
        assert_eq!(result["reasonCode"], "DECISION_SEARCH_READ");
        assert_eq!(result["mode"], "keyword");
        assert!(bridge.pending.is_none());
        assert!(bridge.feedback.is_none());
        assert_eq!(
            std::fs::read(directory.path().join("events.jsonl")).unwrap(),
            events
        );
        for entry in std::fs::read_dir(directory.path()).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                assert!(
                    !String::from_utf8_lossy(&std::fs::read(entry.path()).unwrap())
                        .contains("PRIVATE_SEARCH_CANARY")
                );
            }
        }
        bridge.submit(Command::new(
            "project_decision_search",
            json!({"projectId":project,"query":"new query"}),
        ));
        assert!(bridge.snapshot.as_ref().unwrap()["decisionSearch"].is_null());
        await_idle(&mut bridge);
        let mut changed = bridge.snapshot.as_ref().unwrap().clone();
        changed["projectDecisions"]["projects"] = json!([]);
        bridge.apply_snapshot(Ok(changed));
        assert!(bridge.snapshot.as_ref().unwrap()["decisionSearch"].is_null());
        bridge.shutdown();
    }

    #[test]
    fn shared_search_is_scoped_to_connection_metadata_and_current_remote_revision() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let mut snapshot = bridge.snapshot.as_ref().unwrap().clone();
        snapshot["cloud"] = json!({"paired":true,"cloudUrl":"https://cloud.example.invalid","hostId":"host:one","workspaceId":"workspace:one"});
        snapshot["sharedMemory"] = json!({"revision":12,"projects":[{"projectId":"project:one","sharedProjectId":"shared:one","sharedRevision":4,"enabled":true,"state":"ready"}]});
        let result = json!({"schemaVersion":1,"reasonCode":"SHARED_MEMORY_SEARCH_READ","projectId":"project:one","revision":12,"query":"why Rust","sharedProjectId":"shared:one","manifestRevision":5,"mode":"semantic","model":{},"index":{"state":"ready","indexed":1,"total":1},"matches":[{"decisionId":"decision:one","revision":2,"title":"Owner text","score":0.8}]});
        for change in [
            "host",
            "workspace",
            "url",
            "unpaired",
            "global",
            "scope",
            "manifest",
            "disabled",
            "remap",
        ] {
            bridge.shared_search = Some(result.clone());
            bridge.shared_search_connection = Some(cloud_identity(&snapshot));
            bridge.apply_snapshot(Ok(snapshot.clone()));
            assert_eq!(bridge.snapshot.as_ref().unwrap()["sharedSearch"], result);
            let mut changed = snapshot.clone();
            match change {
                "host" => changed["cloud"]["hostId"] = json!("host:two"),
                "workspace" => changed["cloud"]["workspaceId"] = json!("workspace:two"),
                "url" => changed["cloud"]["cloudUrl"] = json!("https://other.example.invalid"),
                "unpaired" => changed["cloud"]["paired"] = json!(false),
                "global" => changed["sharedMemory"]["revision"] = json!(13),
                "scope" => {
                    changed["sharedMemory"]["projects"][0]["sharedProjectId"] = json!("shared:two")
                }
                "manifest" => changed["sharedMemory"]["projects"][0]["sharedRevision"] = json!(6),
                "disabled" => changed["sharedMemory"]["projects"][0]["enabled"] = json!(false),
                _ => changed["sharedMemory"]["projects"][0]["state"] = json!("connection_changed"),
            }
            bridge.apply_snapshot(Ok(changed));
            assert!(
                bridge.snapshot.as_ref().unwrap()["sharedSearch"].is_null(),
                "{change}"
            );
        }
        bridge.shutdown();
    }

    #[test]
    fn late_search_outcome_never_replaces_failed_owner_retry() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let owner = Command::new("project_decision_propose", json!({"title":"Keep editor"}));
        for outcome in [
            Ok(json!({"reasonCode":"DECISION_SEARCH_READ"})),
            Err("CLOUD_UNAVAILABLE".into()),
        ] {
            bridge.pending = Some(owner.clone());
            bridge.feedback = Some(Notice::error(owner.action, "HOST_ACTION_FAILED"));
            let before = bridge.snapshot.clone();
            let mut old_snapshot = before.as_ref().unwrap().clone();
            old_snapshot["projectMemory"]["projects"] =
                json!([{"projectId":"project:old","revision":1}]);
            bridge
                .output
                .blocking_send(Message::Outcome(
                    Command::new("project_decision_search", json!({"query":"private query"})),
                    outcome,
                    Ok(old_snapshot),
                ))
                .unwrap();
            bridge.poll();
            assert_eq!(bridge.pending.as_ref().unwrap().payload, owner.payload);
            assert_eq!(bridge.feedback.as_ref().unwrap().action, owner.action);
            assert_eq!(bridge.feedback.as_ref().unwrap().code, "HOST_ACTION_FAILED");
            assert_eq!(bridge.snapshot, before);
        }
        bridge.shutdown();
    }
}
