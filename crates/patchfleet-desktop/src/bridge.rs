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

const PRODUCTIVE_KEYS: [&str; 16] = [
    "projectStudio",
    "projectArtifact",
    "projectStudioReceipt",
    "projectStudioError",
    "projectWorkflowRead",
    "projectWorkflowReceipt",
    "projectWorkflowError",
    "projectCrewPlanJob",
    "projectResource",
    "projectArtifactVisual",
    "projectArtifactFrame",
    "projectArtifactOpen",
    "projectCapabilities",
    "projectEvidence",
    "projectCapabilityReceipt",
    "projectCapabilityError",
];
fn productive_key(action: &str) -> Option<&'static str> {
    Some(match action {
        "project_studio_read" => "projectStudio",
        "project_artifact_read" => "projectArtifact",
        "project_resource_read" => "projectResource",
        "project_artifact_visual_read" => "projectArtifactVisual",
        "project_artifact_frame_read" => "projectArtifactFrame",
        "project_artifact_open" => "projectArtifactOpen",
        "project_capabilities_read" => "projectCapabilities",
        "project_evidence_read" => "projectEvidence",
        "project_brand_save"
        | "project_skill_propose"
        | "project_skill_accept"
        | "project_skill_revoke"
        | "project_trigger_save"
        | "project_trigger_set_armed"
        | "project_variants_prepare"
        | "project_evidence_add" => "projectCapabilityReceipt",
        "project_workflows_read" => "projectWorkflowRead",
        "project_crew_plan_read" => "projectCrewPlanJob",
        "project_crew_plan_start"
        | "project_crew_plan_cancel"
        | "project_crew_plan_accept"
        | "project_crew_configure"
        | "project_crew_steer"
        | "project_recipe_save"
        | "project_recipe_run"
        | "project_schedule_save"
        | "project_schedule_set_enabled" => "projectWorkflowReceipt",
        "project_delivery_export"
        | "project_artifact_revise"
        | "project_resource_import"
        | "project_resource_import_batch"
        | "project_resource_from_artifact"
        | "project_resources_from_artifacts"
        | "project_image_engine_set"
        | "project_image_engine_remove"
        | "project_resource_remove"
        | "project_tool_configure"
        | "project_connection_set"
        | "project_connection_remove"
        | "project_connection_prepare"
        | "project_connection_execute" => "projectStudioReceipt",
        _ => return None,
    })
}
fn private_command(command: &Command) -> Command {
    let mut command = command.clone();
    if command.action == "project_connection_set" {
        if let Some(payload) = command.payload.as_object_mut() {
            payload.remove("accessToken");
        }
    }
    if command.action == "project_image_engine_set" {
        if let Some(payload) = command.payload.as_object_mut() {
            payload.remove("url");
        }
    }
    command
}

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
            | "project_results_read"
            | "project_result_read"
            | "project_chat_read"
            | "project_crew_plan"
            | "project_crew_messages_read"
            | "provider_connections_read"
            | "project_studio_read"
            | "project_artifact_read"
            | "project_resource_read"
            | "project_artifact_visual_read"
            | "project_artifact_frame_read"
            | "project_artifact_open"
            | "project_capabilities_read"
            | "project_evidence_read"
            | "project_workflows_read"
            | "project_crew_plan_read"
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
    CreateExport(crate::ui::CreateExportRequest, Option<PathBuf>),
    DeliveryExport(crate::ui::DeliveryExportRequest, Option<PathBuf>),
    StudioPath(crate::ui::StudioPathRequest, Option<PathBuf>),
    CapabilityPath(crate::ui::CapabilityPathRequest, Option<Vec<String>>),
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
    project_create_receipt: Option<Value>,
    project_work_receipt: Option<Value>,
    project_results: Option<Value>,
    project_result: Option<Value>,
    project_chat: Option<Value>,
    project_crew_plan: Option<Value>,
    project_crew_receipt: Option<Value>,
    project_crew_messages: Option<Value>,
    project_crew_error: Option<Value>,
    provider_connections: Option<Value>,
    productive_pages: std::collections::HashMap<&'static str, Value>,
    productive_project: Option<Value>,
    productive_requests: std::collections::HashMap<&'static str, String>,
    artifact_open: Option<PathBuf>,
    opened_artifacts: std::collections::HashSet<String>,
    pub capability_paths: Option<(crate::ui::CapabilityPathRequest, Option<Vec<String>>)>,
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
    let extraction = snapshot["projectExtraction"]["projects"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|p| {
            p["counts"]["queued"].as_u64().unwrap_or(0) > 0
                || p["counts"]["running"].as_u64().unwrap_or(0) > 0
        });
    Duration::from_secs(
        if active_work
            || active_team
            || extraction
            || snapshot["projectWorkflows"]["plans"]
                .as_array()
                .is_some_and(|plans| {
                    plans
                        .iter()
                        .any(|plan| matches!(plan["status"].as_str(), Some("queued" | "running")))
                })
            || snapshot["cloud"]["paired"] == true
            || snapshot["projectCapabilities"]["triggers"]
                .as_array()
                .is_some_and(|triggers| triggers.iter().any(|trigger| trigger["armed"] == true))
        {
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
    pub fn capabilities_smoke(&mut self, scene: &str) {
        let mut snapshot = self.snapshot.clone().unwrap();
        let project = snapshot["projectMemory"]["projects"][0]["projectId"].clone();
        let host = self.host.lock().unwrap().as_ref().cloned().unwrap();
        let section = match scene {
            "capabilities-skills" => "skills",
            "capabilities-triggers" => "triggers",
            "capabilities-alternatives" => "variants",
            "capabilities-evidence" => "evidence",
            _ => "profiles",
        };
        for (key, action, payload) in [
            (
                "projectStudio",
                "project_studio_read",
                serde_json::json!({"projectId":project}),
            ),
            (
                "projectWorkflowRead",
                "project_workflows_read",
                serde_json::json!({"projectId":project}),
            ),
            (
                "projectCapabilities",
                "project_capabilities_read",
                serde_json::json!({"projectId":project,"section":section,"offset":0,"limit":20}),
            ),
        ] {
            let page = self.runtime.block_on(host.action(action, payload)).unwrap();
            self.productive_pages.insert(key, page);
        }
        self.productive_project = Some(project);
        if scene == "capabilities-canvas" {
            let artifact = self.productive_pages["projectStudio"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|artifact| {
                    artifact["path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with(".png"))
                })
                .unwrap();
            let preview=self.runtime.block_on(host.action("project_artifact_read",serde_json::json!({"projectId":snapshot["projectMemory"]["projects"][0]["projectId"],"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"]}))).unwrap();
            self.productive_pages.insert("projectArtifact", preview);
        }
        self.productive_overlay(&mut snapshot);
        self.snapshot = Some(snapshot);
    }
    #[cfg(feature = "ui-smoke")]
    pub fn smoke_provider_connections(&mut self, ready: bool, selected: &str) {
        if self.provider_connections.is_some() {
            return;
        }
        assert!(["codex", "claude", "gemini"].contains(&selected));
        let providers: Vec<_> = ["codex", "claude", "gemini"]
            .into_iter()
            .map(|id| {
                let available = ready && id == selected;
                serde_json::json!({"id":id,"installed":available,
                    "version":if available {Some(if id == "codex" {"0.163.0"} else {"fixture"})} else {None},
                    "authentication":if available {"present"} else {"unknown"},"controlSupported":available,
                    "reasonCode":"UI_FIXTURE_NO_PROVIDER_PROBED"})
            })
            .collect();
        self.provider_connections = Some(serde_json::json!({
            "schemaVersion":1,"reasonCode":"PROVIDER_CONNECTIONS_READ",
            "observedAt":"2026-10-20T00:00:00.000Z","providers":providers
        }));
        if let Some(snapshot) = self.snapshot.clone() {
            self.apply_snapshot(Ok(snapshot));
        }
    }

    #[cfg(feature = "ui-smoke")]
    pub fn productive_smoke(&mut self, scene: &str) {
        let snapshot = self.snapshot.clone().unwrap();
        let project = snapshot["projectMemory"]["projects"][0]["projectId"].clone();
        self.productive_project = Some(project.clone());
        let host = self.host.lock().unwrap().as_ref().cloned().unwrap();
        let page = self
            .runtime
            .block_on(host.action(
                "project_studio_read",
                serde_json::json!({"projectId":project}),
            ))
            .unwrap();
        self.productive_pages.insert("projectStudio", page.clone());
        if scene == "productive-app-preview" {
            self.productive_pages.get_mut("projectStudio").unwrap()["connections"] = serde_json::json!([{"providerId":"notion","label":"Rendering fixture · no account","connected":true,"authentication":"unverified","operations":["notion_page"]}]);
            self.productive_pages.insert("projectStudioReceipt",serde_json::json!({"projectId":project,"commandId":"cmd:rendering-preview","outcome":"applied","reasonCode":"PROJECT_CONNECTION_PREPARED","preparedActionId":"connection-action:rendering-fixture","revision":1,"providerId":"notion","operation":"notion_page","arguments":{"parentPageId":"rendering-fixture-parent","title":"Launch brief · UI fixture","content":"Disposable preview. No account is connected and no remote action has been performed."}}));
        }

        if let Some(artifact) = page["artifacts"].as_array().and_then(|rows| rows.first()) {
            let preview=self.runtime.block_on(host.action("project_artifact_read",serde_json::json!({"projectId":project,"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"]}))).unwrap();
            self.productive_pages.insert("projectArtifact", preview);
        }
        let workflows = self
            .runtime
            .block_on(host.action(
                "project_workflows_read",
                serde_json::json!({"projectId":project}),
            ))
            .unwrap();
        self.productive_pages
            .insert("projectWorkflowRead", workflows);
        if matches!(scene, "productive-plan" | "productive-questions") {
            let goal = snapshot["projectCrews"]["missions"][0]["instruction"]
                .as_str()
                .or_else(|| {
                    snapshot["abilities"]
                        .as_array()?
                        .iter()
                        .find(|ability| ability["id"] == "P1")?["starter"]
                        [if snapshot["desktopPreferences"]["language"] == "en" {
                            "en"
                        } else {
                            "tr"
                        }]
                    .as_str()
                })
                .expect("A disposable plan goal is required");
            let plan = self
                .runtime
                .block_on(host.action(
                    "project_crew_plan",
                    serde_json::json!({"projectId":project,"abilityId":"P1","instruction":goal}),
                ))
                .unwrap();
            let english = snapshot["desktopPreferences"]["language"] == "en";
            self.productive_pages.insert("projectCrewPlanJob",serde_json::json!({"reasonCode":"PROJECT_CREW_PLAN_READ","projectId":project,"planJobId":"plan:rendering-fixture","revision":3,"status":if scene=="productive-questions"{"needs_input"}else{"completed"},"providerId":"codex","title":"Illustrative launch plan · no provider run","instruction":goal,"plan":if scene=="productive-questions"{Value::Null}else{serde_json::json!({"abilityId":"P1","members":plan["members"],"settings":plan["settings"]})},"questions":if scene=="productive-questions"{serde_json::json!([{"id":"audience","question":if english {"Who should this launch reach?"} else {"Bu sunum kimler için?"}},{"id":"format","question":if english {"Which deliverable should the crew prioritize?"} else {"En önemli hedefiniz nedir?"}}])}else{serde_json::json!([])}}));
        }
        self.apply_snapshot(Ok(snapshot));
    }
    #[cfg(feature = "ui-smoke")]
    pub fn chat_smoke(&mut self, show_connections: bool) {
        let project =
            self.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0]["projectId"].clone();
        if project.is_string() {
            self.submit(Command::new(
                "project_chat_read",
                serde_json::json!({"projectId":project}),
            ));
            for _ in 0..400 {
                self.poll();
                if !self.busy {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(
                self.snapshot.as_ref().unwrap()["projectChat"]["reasonCode"],
                "PROJECT_CHAT_READ"
            );
        }
        if show_connections {
            // Rendering fixture only: never probe an installed CLI from a smoke scene.
            self.provider_connections = Some(
                serde_json::json!({"schemaVersion":1,"reasonCode":"PROVIDER_CONNECTIONS_READ","observedAt":"2026-10-19T00:00:00.000Z","providers":[
                {"id":"codex","installed":false,"version":null,"authentication":"unknown","controlSupported":false,"reasonCode":"UI_FIXTURE_NO_PROVIDER_PROBED"},
                {"id":"claude","installed":false,"version":null,"authentication":"unknown","controlSupported":false,"reasonCode":"UI_FIXTURE_NO_PROVIDER_PROBED"},
                {"id":"gemini","installed":false,"version":null,"authentication":"unknown","controlSupported":false,"reasonCode":"UI_FIXTURE_NO_PROVIDER_PROBED"}]}),
            );
            self.apply_snapshot(Ok(self.snapshot.clone().unwrap()));
        }
    }

    #[cfg(feature = "ui-smoke")]
    pub fn crew_smoke(&mut self, scene: &str) {
        let mission = self.snapshot.as_ref().unwrap()["projectCrews"]["missions"][0].clone();
        self.submit(Command::new("project_crew_plan", serde_json::json!({"projectId":mission["projectId"],"instruction":mission["instruction"],"abilityId":mission["abilityId"]})));
        for _ in 0..400 {
            self.poll();
            if !self.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            self.snapshot.as_ref().unwrap()["projectCrewPlan"]["basis"],
            "bundled_guides"
        );
        self.project_chat = Some(
            serde_json::json!({"reasonCode":"PROJECT_CHAT_READ","projectId":mission["projectId"],"offset":0,"total":0,"items":[],"nextOffset":null}),
        );
        // Isolated rendering data: no provider is probed, started, or represented as verified.
        let mut snapshot = self.snapshot.clone().unwrap();
        if matches!(scene, "crew-running" | "crew-messages") {
            snapshot["projectCrews"]["missions"][0]["status"] = Value::from("active");
            for (index, task) in snapshot["projectCrews"]["missions"][0]["tasks"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                task["status"] = Value::from(if index == 0 {
                    "completed"
                } else if index == 1 {
                    "running"
                } else {
                    "pending"
                });
            }
        }
        if scene == "crew-messages" {
            let tasks = mission["tasks"].as_array().unwrap();
            self.project_crew_messages = Some(
                serde_json::json!({"reasonCode":"PROJECT_CREW_MESSAGES_READ","projectId":mission["projectId"],"missionId":mission["missionId"],"offset":0,"limit":20,"total":2,"nextOffset":null,"items":[
                    {"messageId":"message:ui-one","projectId":mission["projectId"],"fromMissionId":mission["missionId"],"toMissionId":mission["missionId"],"fromAgentId":tasks[0]["agentId"],"toAgentId":tasks[1]["agentId"],"body":"UI fixture: source notes are ready for the presentation outline.","delivery":"read","createdAt":"2026-10-19T10:00:00.000Z"},
                    {"messageId":"message:ui-two","projectId":mission["projectId"],"fromMissionId":mission["missionId"],"toMissionId":mission["missionId"],"fromAgentId":tasks[1]["agentId"],"toAgentId":tasks[0]["agentId"],"body":"UI fixture: please check the audience estimate before review.","delivery":"pending","createdAt":"2026-10-19T10:01:00.000Z"}
                ]}),
            );
        }
        self.apply_snapshot(Ok(snapshot));
    }

    #[cfg(feature = "ui-smoke")]
    pub fn create_smoke(&mut self) {
        let job = self.snapshot.as_ref().unwrap()["projectJobs"][0].clone();
        self.submit(Command::new(
            "project_results_read",
            serde_json::json!({"projectId":job["projectId"],"workItemId":job["workItemId"]}),
        ));
        for _ in 0..400 {
            self.poll();
            if !self.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let path = self.snapshot.as_ref().unwrap()["projectResults"]["items"][0]["path"].clone();
        self.submit(Command::new("project_result_read", serde_json::json!({"projectId":job["projectId"],"workItemId":job["workItemId"],"path":path})));
        for _ in 0..400 {
            self.poll();
            if !self.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(self.snapshot.as_ref().unwrap()["projectResult"]["text"].is_string());
    }

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
                            if !notify(&ctx, &output, Message::Outcome(private_command(&command), result, snapshot.clone())).await { break; }
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
            project_create_receipt: None,
            project_work_receipt: None,
            project_results: None,
            project_result: None,
            project_chat: None,
            project_crew_plan: None,
            project_crew_receipt: None,
            project_crew_messages: None,
            project_crew_error: None,
            provider_connections: None,
            productive_pages: Default::default(),
            productive_project: None,
            productive_requests: Default::default(),
            artifact_open: None,
            opened_artifacts: Default::default(),
            capability_paths: None,
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
                Message::Snapshot(snapshot) => self.apply_delivered_snapshot(snapshot),
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
                            self.productive_outcome(&command, Ok(&receipt));
                            match command.action {
                                "project_create" => {
                                    self.project_create_receipt = Some(receipt.clone());
                                }
                                "project_work_enqueue" | "project_chat_send" => {
                                    let mut scoped = receipt.clone();
                                    scoped["commandId"] = command.payload["commandId"].clone();
                                    scoped["projectId"] = command.payload["projectId"].clone();
                                    self.project_work_receipt = Some(scoped);
                                    self.project_chat = None;
                                }
                                "project_crew_plan" => {
                                    let mut scoped = receipt.clone();
                                    scoped["commandId"] = command.payload["commandId"].clone();
                                    scoped["projectId"] = command.payload["projectId"].clone();
                                    self.project_crew_plan = Some(scoped);
                                    self.project_crew_error = None;
                                }
                                "project_crew_messages_read" => {
                                    self.project_crew_messages = Some(receipt.clone());
                                    self.project_crew_error = None;
                                }
                                "project_crew_prepare"
                                | "project_crew_start"
                                | "project_crew_cancel"
                                | "project_crew_retry" => {
                                    let mut scoped = receipt.clone();
                                    scoped["commandId"] = command.payload["commandId"].clone();
                                    scoped["projectId"] = command.payload["projectId"].clone();
                                    scoped["action"] = Value::from(command.action);
                                    if scoped["missionId"].is_null() {
                                        scoped["missionId"] = command.payload["missionId"].clone();
                                    }
                                    self.project_crew_receipt = Some(scoped);
                                    self.project_crew_messages = None;
                                    self.project_crew_error = None;
                                }
                                "project_chat_read" => self.project_chat = Some(receipt.clone()),
                                "provider_connections_read" => {
                                    self.provider_connections = Some(receipt.clone())
                                }
                                "project_results_read" => {
                                    self.project_results = Some(receipt.clone())
                                }
                                "project_result_read" => {
                                    self.project_result = Some(receipt.clone())
                                }
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
                            self.productive_outcome(&command, Err(&error));
                            if command.action == "project_create" {
                                self.project_create_receipt = Some(serde_json::json!({
                                    "commandId":command.payload["commandId"],
                                    "outcome":"unknown","reasonCode":safe_error(&error)
                                }));
                            }
                            if command.action.starts_with("project_crew_")
                                && productive_key(command.action).is_none()
                            {
                                let scoped = serde_json::json!({"action":command.action,"commandId":command.payload["commandId"],"projectId":command.payload["projectId"],"missionId":command.payload["missionId"],"outcome":"rejected","reasonCode":safe_error(&error)});
                                if memory_read(command.action) {
                                    self.project_crew_error = Some(scoped);
                                } else {
                                    self.project_crew_receipt = Some(scoped);
                                }
                            }
                            if matches!(
                                command.action,
                                "project_work_enqueue" | "project_chat_send"
                            ) {
                                self.project_work_receipt = Some(
                                    serde_json::json!({"commandId":command.payload["commandId"],"projectId":command.payload["projectId"],"outcome":"failed","reasonCode":safe_error(&error)}),
                                );
                            }
                            self.feedback = Some(Notice::error(command.action, &error));
                            self.pending = (!matches!(
                                command.action,
                                "project_connection_set"
                                    | "project_connection_execute"
                                    | "project_image_engine_set"
                            ))
                            .then_some(command);
                        }
                    }
                    self.apply_delivered_snapshot(snapshot);
                    self.busy = false;
                }
                Message::CapabilityPath(request, selection) => {
                    self.picking_folder = false;
                    self.picker = None;
                    if !self.closing {
                        self.capability_paths = Some((request, selection));
                    }
                }
                Message::StudioPath(request, selection) => {
                    self.picking_folder = false;
                    self.picker = None;
                    if !self.closing {
                        if let Some(command) = crate::ui::Console::studio_path_command(
                            request,
                            selection.and_then(|path| path.to_str().map(str::to_owned)),
                        ) {
                            self.submit(command);
                        }
                    }
                }
                Message::CreateExport(request, selection) => {
                    self.picking_folder = false;
                    self.picker = None;
                    if !self.closing {
                        if let Some(command) = crate::ui::Console::create_export_command(
                            request,
                            selection.and_then(|p| p.to_str().map(str::to_owned)),
                        ) {
                            self.submit(command);
                        }
                    }
                }
                Message::DeliveryExport(request, selection) => {
                    self.picking_folder = false;
                    self.picker = None;
                    if !self.closing {
                        let current_epoch = self
                            .snapshot
                            .as_ref()
                            .map(|snapshot| &snapshot["hostEpoch"])
                            .unwrap_or(&Value::Null);
                        if request.host_epoch != *current_epoch && selection.is_some() {
                            self.feedback = Some(Notice::error(
                                "project_delivery_export",
                                "PROJECT_DELIVERY_HOST_CHANGED",
                            ));
                        } else if let Some(command) = crate::ui::Console::delivery_export_command(
                            request,
                            selection.and_then(|path| path.to_str().map(str::to_owned)),
                            current_epoch,
                        ) {
                            self.submit(command);
                        }
                    }
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

    fn apply_delivered_snapshot(&mut self, delivered: Result<Value, String>) {
        if delivered.as_ref().is_ok_and(|delivered| {
            self.snapshot
                .as_ref()
                .is_none_or(|current| cloud_identity(current) == cloud_identity(delivered))
        }) {
            self.apply_snapshot(delivered);
            return;
        }
        let host = self.host.lock().unwrap().as_ref().cloned();
        let snapshot = match (delivered, host) {
            (Ok(delivered), Some(host)) => match host.snapshot().map_err(safe_error) {
                Ok(current) if cloud_identity(&current) != cloud_identity(&delivered) => {
                    Ok(current)
                }
                Ok(_) => Ok(delivered),
                Err(error) => Err(error),
            },
            (delivered, _) => delivered,
        };
        self.apply_snapshot(snapshot);
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
                    page["extractionRevision"].as_u64().unwrap_or(0)
                        != snapshot["projectExtraction"]["revision"]
                            .as_u64()
                            .unwrap_or(0)
                        || !snapshot["projectMemory"]["projects"]
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
                for cached in [&mut self.project_results, &mut self.project_result] {
                    if cached.as_ref().is_some_and(|page| {
                        !snapshot["projectJobs"].as_array().is_some_and(|jobs| {
                            jobs.iter().any(|job| {
                                job["projectId"] == page["projectId"]
                                    && job["workItemId"] == page["workItemId"]
                                    && job["runtimeIssue"].is_null()
                            })
                        })
                    }) {
                        *cached = None;
                    }
                }
                snapshot["projectWorkReceipt"] =
                    self.project_work_receipt.clone().unwrap_or(Value::Null);
                snapshot["projectResults"] = self.project_results.clone().unwrap_or(Value::Null);
                snapshot["projectResult"] = self.project_result.clone().unwrap_or(Value::Null);
                if self
                    .snapshot
                    .as_ref()
                    .is_some_and(|old| old["hostEpoch"] != snapshot["hostEpoch"])
                {
                    self.project_chat = None;
                    self.project_crew_plan = None;
                    self.project_crew_messages = None;
                    self.project_crew_receipt = None;
                    self.project_crew_error = None;
                    self.provider_connections = None;
                    self.project_create_receipt = None;
                }
                if self.project_chat.as_ref().is_some_and(|page| {
                    !snapshot["projectMemory"]["projects"]
                        .as_array()
                        .is_some_and(|projects| {
                            projects
                                .iter()
                                .any(|project| project["projectId"] == page["projectId"])
                        })
                }) {
                    self.project_chat = None;
                }
                for cache in [&mut self.project_crew_plan, &mut self.project_crew_messages] {
                    if cache.as_ref().is_some_and(|page| {
                        !snapshot["projectMemory"]["projects"]
                            .as_array()
                            .is_some_and(|projects| {
                                projects.iter().any(|p| p["projectId"] == page["projectId"])
                            })
                    }) {
                        *cache = None;
                    }
                }
                if self.project_crew_messages.as_ref().is_some_and(|page| {
                    !snapshot["projectCrews"]["missions"]
                        .as_array()
                        .is_some_and(|missions| {
                            missions.iter().any(|m| {
                                m["projectId"] == page["projectId"]
                                    && m["missionId"] == page["missionId"]
                            })
                        })
                }) {
                    self.project_crew_messages = None;
                }
                snapshot["projectCrewPlan"] = self.project_crew_plan.clone().unwrap_or(Value::Null);
                snapshot["projectCrewReceipt"] =
                    self.project_crew_receipt.clone().unwrap_or(Value::Null);
                snapshot["projectCrewMessages"] =
                    self.project_crew_messages.clone().unwrap_or(Value::Null);
                snapshot["projectCrewError"] =
                    self.project_crew_error.clone().unwrap_or(Value::Null);
                snapshot["projectChat"] = self.project_chat.clone().unwrap_or(Value::Null);
                snapshot["projectCreateReceipt"] =
                    self.project_create_receipt.clone().unwrap_or(Value::Null);
                snapshot["providerConnections"] =
                    self.provider_connections.clone().unwrap_or(Value::Null);
                self.productive_overlay(&mut snapshot);
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
                self.productive_submit(&command);
                match command.action {
                    "project_create" => self.project_create_receipt = None,
                    "project_work_enqueue" | "project_chat_send" => {
                        self.project_work_receipt = None
                    }
                    "project_crew_plan" => {
                        self.project_crew_plan = None;
                        self.project_crew_error = None;
                    }
                    "project_crew_messages_read" => {
                        self.project_crew_messages = None;
                        self.project_crew_error = None;
                    }
                    "project_crew_prepare"
                    | "project_crew_start"
                    | "project_crew_cancel"
                    | "project_crew_retry" => {
                        self.project_crew_receipt = None;
                        self.project_crew_error = None;
                    }
                    "project_chat_read" => self.project_chat = None,
                    "provider_connections_read" => self.provider_connections = None,
                    "project_results_read" => {
                        self.project_results = None;
                        self.project_result = None;
                    }
                    "project_result_read" => self.project_result = None,
                    _ => (),
                }
                if let Some(snapshot) = self.snapshot.as_mut() {
                    snapshot["projectCreateReceipt"] =
                        self.project_create_receipt.clone().unwrap_or(Value::Null);
                    snapshot["projectWorkReceipt"] =
                        self.project_work_receipt.clone().unwrap_or(Value::Null);
                    snapshot["projectResults"] =
                        self.project_results.clone().unwrap_or(Value::Null);
                    snapshot["projectResult"] = self.project_result.clone().unwrap_or(Value::Null);
                    snapshot["projectCrewPlan"] =
                        self.project_crew_plan.clone().unwrap_or(Value::Null);
                    snapshot["projectCrewReceipt"] =
                        self.project_crew_receipt.clone().unwrap_or(Value::Null);
                    snapshot["projectCrewMessages"] =
                        self.project_crew_messages.clone().unwrap_or(Value::Null);
                    snapshot["projectCrewError"] =
                        self.project_crew_error.clone().unwrap_or(Value::Null);
                    snapshot["projectChat"] = self.project_chat.clone().unwrap_or(Value::Null);
                    snapshot["providerConnections"] =
                        self.provider_connections.clone().unwrap_or(Value::Null);
                }
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
                self.pending = Some(private_command(&command));
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

    fn productive_submit(&mut self, command: &Command) {
        if command.payload["projectId"].is_string()
            && self.productive_project.as_ref() != Some(&command.payload["projectId"])
        {
            self.productive_pages.clear();
            self.productive_requests.clear();
            self.artifact_open = None;
            self.productive_project = Some(command.payload["projectId"].clone());
            if let Some(snapshot) = self.snapshot.as_mut() {
                for key in PRODUCTIVE_KEYS {
                    snapshot[key] = Value::Null;
                }
            }
        }
        let Some(key) = productive_key(command.action) else {
            return;
        };
        if self.productive_project.as_ref() != Some(&command.payload["projectId"]) {
            self.productive_pages.clear();
            self.productive_requests.clear();
            self.productive_project = Some(command.payload["projectId"].clone());
        }
        self.productive_requests.insert(
            key,
            command.payload["commandId"].as_str().unwrap_or("").into(),
        );
        self.productive_pages.remove(key);
        self.productive_pages.remove(
            if key.starts_with("projectCapabilit") || key == "projectEvidence" {
                "projectCapabilityError"
            } else if key.starts_with("projectWorkflow") || key == "projectCrewPlanJob" {
                "projectWorkflowError"
            } else {
                "projectStudioError"
            },
        );
        if let Some(snapshot) = self.snapshot.as_mut() {
            for key in PRODUCTIVE_KEYS {
                snapshot[key] = self
                    .productive_pages
                    .get(key)
                    .cloned()
                    .unwrap_or(Value::Null);
            }
        }
    }
    fn productive_outcome(&mut self, command: &Command, result: Result<&Value, &str>) {
        let Some(key) = productive_key(command.action) else {
            return;
        };
        if self.productive_project.as_ref() != Some(&command.payload["projectId"])
            || self.productive_requests.get(key).map(String::as_str)
                != command.payload["commandId"].as_str()
        {
            return;
        }
        if key == "projectArtifactOpen"
            && result.is_ok_and(|receipt| {
                receipt["reasonCode"] != "PROJECT_ARTIFACT_OPEN"
                    || receipt["sha256"] != command.payload["expectedSha256"]
                    || ["projectId", "commandId", "workItemId", "path"]
                        .iter()
                        .any(|field| receipt[*field] != command.payload[*field])
            })
        {
            return;
        }
        if let Ok(receipt) = result {
            for field in [
                "projectId",
                "commandId",
                "missionId",
                "planJobId",
                "workItemId",
                "path",
                "section",
                "offset",
                "resourceId",
                "evidenceId",
                "page",
                "time",
            ] {
                if !command.payload[field].is_null()
                    && !receipt[field].is_null()
                    && command.payload[field] != receipt[field]
                {
                    return;
                }
            }
        }
        if result.is_ok_and(|receipt| {
            !command.payload["expectedSha256"].is_null()
                && !receipt["sha256"].is_null()
                && receipt["sha256"] != command.payload["expectedSha256"]
        }) {
            return;
        }
        let mut scoped = match result {
            Ok(value) => value.clone(),
            Err(code) => serde_json::json!({"outcome":"rejected","reasonCode":safe_error(code)}),
        };
        for identity in [
            "commandId",
            "projectId",
            "missionId",
            "planJobId",
            "workItemId",
            "path",
            "section",
            "offset",
            "resourceId",
            "evidenceId",
            "page",
            "time",
        ] {
            if !command.payload[identity].is_null() && scoped[identity].is_null() {
                scoped[identity] = command.payload[identity].clone();
            }
        }
        scoped["action"] = Value::from(command.action);
        if key == "projectCrewPlanJob" && result.is_ok() {
            scoped["reasonCode"] = Value::from("PROJECT_CREW_PLAN_READ");
        }
        let error_key = if key.starts_with("projectCapabilit") || key == "projectEvidence" {
            "projectCapabilityError"
        } else if key.starts_with("projectWorkflow") || key == "projectCrewPlanJob" {
            "projectWorkflowError"
        } else {
            "projectStudioError"
        };
        if result.is_err() {
            self.productive_pages.insert(error_key, scoped.clone());
        }
        if result.is_ok() || !memory_read(command.action) {
            self.productive_pages.insert(key, scoped.clone());
        }
        if key == "projectArtifactOpen" && result.is_ok() {
            if let (Some(path), Some(id)) =
                (scoped["openPath"].as_str(), scoped["commandId"].as_str())
            {
                if scoped["reasonCode"] == "PROJECT_ARTIFACT_OPEN"
                    && self.opened_artifacts.insert(id.to_owned())
                {
                    self.artifact_open = Some(PathBuf::from(path));
                }
            }
        }
        if key == "projectCapabilityReceipt" {
            self.productive_pages.remove("projectCapabilities");
        }
        if command.action == "project_artifact_revise" && result.is_ok() {
            self.project_work_receipt = Some(scoped);
            self.project_chat = None;
        }
        if key == "projectWorkflowReceipt" {
            self.productive_pages.remove("projectWorkflowRead");
        }
        if matches!(
            command.action,
            "project_resource_import"
                | "project_resource_import_batch"
                | "project_resource_from_artifact"
                | "project_resources_from_artifacts"
                | "project_image_engine_set"
                | "project_image_engine_remove"
                | "project_resource_remove"
                | "project_tool_configure"
                | "project_connection_set"
                | "project_connection_remove"
                | "project_artifact_revise"
        ) {
            self.productive_pages.remove("projectStudio");
        }
    }
    fn productive_overlay(&mut self, snapshot: &mut Value) {
        let changed = self
            .snapshot
            .as_ref()
            .is_some_and(|old| old["hostEpoch"] != snapshot["hostEpoch"]);
        let removed = self.productive_project.as_ref().is_some_and(|id| {
            !snapshot["projectMemory"]["projects"]
                .as_array()
                .is_some_and(|projects| projects.iter().any(|project| project["projectId"] == *id))
        });
        if changed || removed {
            self.productive_pages.clear();
            self.productive_requests.clear();
            self.productive_project = None;
            self.artifact_open = None;
            self.opened_artifacts.clear();
            self.capability_paths = None;
        }
        for key in PRODUCTIVE_KEYS {
            snapshot[key] = self
                .productive_pages
                .get(key)
                .cloned()
                .unwrap_or(Value::Null);
        }
    }
    pub fn choose_studio_path(
        &mut self,
        ctx: egui::Context,
        request: crate::ui::StudioPathRequest,
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
            notify(&ctx, &output, Message::StudioPath(request, choice.await)).await;
        }));
    }
    pub fn take_artifact_open(&mut self) -> Option<PathBuf> {
        self.artifact_open.take()
    }
    pub fn choose_capability_paths(
        &mut self,
        ctx: egui::Context,
        request: crate::ui::CapabilityPathRequest,
        choice: impl std::future::Future<Output = Option<Vec<String>>> + Send + 'static,
    ) {
        if self.busy || self.closing || self.picking_folder || self.error.is_some() {
            return;
        }
        self.dismiss_feedback();
        self.picking_folder = true;
        ctx.request_repaint();
        let output = self.output.clone();
        self.picker = Some(self.runtime.spawn(async move {
            notify(
                &ctx,
                &output,
                Message::CapabilityPath(request, choice.await),
            )
            .await;
        }));
    }

    pub fn choose_create_export(
        &mut self,
        ctx: egui::Context,
        request: crate::ui::CreateExportRequest,
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
            notify(&ctx, &output, Message::CreateExport(request, choice.await)).await;
        }));
    }

    pub fn choose_delivery_export(
        &mut self,
        ctx: egui::Context,
        request: crate::ui::DeliveryExportRequest,
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
            notify(
                &ctx,
                &output,
                Message::DeliveryExport(request, choice.await),
            )
            .await;
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
        for status in ["queued", "running"] {
            assert_eq!(
                poll_delay(&json!({"projectExtraction":{"projects":[{"counts":{status:1}}]}})),
                Duration::from_secs(5)
            );
        }
        assert_eq!(
            poll_delay(
                &json!({"projectExtraction":{"projects":[{"counts":{"failed":1,"completed":2}}]}})
            ),
            Duration::from_secs(60)
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

    #[test]
    fn productive_responses_are_exact_request_project_epoch_scoped_and_tokens_are_not_retryable() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new(
            "project_create",
            json!({"name":"Studio scope"}),
        ));
        await_idle(&mut bridge);
        let project =
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0]["projectId"].clone();
        let old = Command::new(
            "project_artifact_read",
            json!({"projectId":project,"workItemId":"work:old","path":"outputs/old.txt","expectedSha256":"hash-old"}),
        );
        let current = Command::new(
            "project_artifact_read",
            json!({"projectId":project,"workItemId":"work:new","path":"outputs/new.txt","expectedSha256":"hash-new"}),
        );
        bridge.productive_submit(&old);
        bridge.productive_submit(&current);
        bridge.productive_outcome(&old,Ok(&json!({"reasonCode":"PROJECT_ARTIFACT_READ","projectId":project,"workItemId":"work:old","path":"outputs/old.txt","sha256":"hash-old"})));
        assert!(!bridge.productive_pages.contains_key("projectArtifact"));
        bridge.productive_outcome(&current,Ok(&json!({"reasonCode":"PROJECT_ARTIFACT_READ","projectId":"project:foreign","workItemId":"work:new","path":"outputs/new.txt"})));
        assert!(!bridge.productive_pages.contains_key("projectArtifact"));
        bridge.productive_outcome(&current,Ok(&json!({"reasonCode":"PROJECT_ARTIFACT_READ","projectId":project,"workItemId":"work:new","path":"outputs/new.txt","sha256":"hash-new","preview":{"kind":"text","text":"Scoped actual preview"}})));
        let snapshot = bridge.snapshot.clone().unwrap();
        bridge.apply_snapshot(Ok(snapshot));
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectArtifact"]["path"],
            "outputs/new.txt"
        );
        let mut epoch = bridge.snapshot.clone().unwrap();
        epoch["hostEpoch"] = json!("epoch:changed");
        bridge.apply_snapshot(Ok(epoch));
        assert!(bridge.snapshot.as_ref().unwrap()["projectArtifact"].is_null());
        let secret = Command::new(
            "project_connection_set",
            json!({"projectId":project,"providerId":"notion","label":"My Notion","accessToken":"PRIVATE-TOKEN-CANARY"}),
        );
        assert!(!format!("{:?}", private_command(&secret)).contains("PRIVATE-TOKEN-CANARY"));
        // Rejected credentials must require explicit entry again, never expose a retry payload.
        bridge.productive_submit(&secret);
        bridge.pending = Some(private_command(&secret));
        bridge.busy = true;
        let snapshot = bridge.snapshot.clone().unwrap();
        bridge
            .output
            .blocking_send(Message::Outcome(
                private_command(&secret),
                Err("PROJECT_CONNECTION_REJECTED".into()),
                Ok(snapshot),
            ))
            .unwrap();
        bridge.poll();
        assert!(bridge.pending.is_none());
        assert!(!bridge
            .snapshot
            .as_ref()
            .unwrap()
            .to_string()
            .contains("PRIVATE-TOKEN-CANARY"));
        bridge.shutdown();
    }

    #[test]
    fn explicit_output_open_requires_exact_version_and_is_consumed_once() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let command = Command::new(
            "project_artifact_open",
            json!({"projectId":"project:one","workItemId":"work:one","path":"outputs/interactive.html","expectedSha256":"a".repeat(64)}),
        );
        bridge.productive_submit(&command);
        let mut receipt = json!({"reasonCode":"PROJECT_ARTIFACT_OPEN","commandId":command.payload["commandId"],"projectId":"project:one","workItemId":"work:one","path":"outputs/interactive.html","sha256":"b".repeat(64),"openPath":"/tmp/checked/output.html"});
        bridge.productive_outcome(&command, Ok(&receipt));
        assert!(bridge.take_artifact_open().is_none());
        receipt["sha256"] = command.payload["expectedSha256"].clone();
        bridge.productive_outcome(&command, Ok(&receipt));
        assert_eq!(
            bridge.take_artifact_open(),
            Some(PathBuf::from("/tmp/checked/output.html"))
        );
        assert!(bridge.take_artifact_open().is_none());
        bridge.productive_outcome(&command, Ok(&receipt));
        assert!(bridge.take_artifact_open().is_none());
        let changed = Command::new("project_artifact_open", command.payload.clone());
        let mut different = changed.payload.clone();
        different["projectId"] = json!("project:other");
        let other = Command::new("project_artifact_open", different);
        bridge.productive_submit(&other);
        bridge.productive_outcome(&command, Ok(&receipt));
        assert!(bridge.take_artifact_open().is_none());
        bridge.shutdown();
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
    fn retained_normal_snapshots_and_owner_outcomes_cannot_revert_account_transitions() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let host = bridge.host.lock().unwrap().as_ref().unwrap().clone();
        let listener = bridge
            .runtime
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let account_seen = Arc::new(AtomicBool::new(false));
        let account_release = Arc::new(tokio::sync::Notify::new());
        let seen = account_seen.clone();
        let release = account_release.clone();
        let server=bridge.runtime.spawn(async move {
            loop {
                let (mut socket,_)=listener.accept().await.unwrap();
                let seen=seen.clone();
                let release=release.clone();
                tokio::spawn(async move {
                    let mut bytes=Vec::new();
                    let mut chunk=[0;4096];
                    let (start,head)=loop {
                        let n=socket.read(&mut chunk).await.unwrap();
                        if n==0 { return; }
                        bytes.extend_from_slice(&chunk[..n]);
                        if let Some(i)=bytes.windows(4).position(|w|w==b"\r\n\r\n") { break (i+4,String::from_utf8(bytes[..i].to_vec()).unwrap()); }
                    };
                    let length=head.lines().find_map(|line|line.to_ascii_lowercase().strip_prefix("content-length:").and_then(|n|n.trim().parse::<usize>().ok())).unwrap_or(0);
                    while bytes.len()<start+length {
                        let n=socket.read(&mut chunk).await.unwrap();
                        if n==0 { return; }
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    let path=head.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                    let response=if path=="/api/desktop/v1/token" {
                        let body:Value=serde_json::from_slice(&bytes[start..start+length]).unwrap();
                        assert_eq!(body["schemaVersion"],1);
                        assert_eq!(body["code"],"A".repeat(43));
                        json!({"schemaVersion":1,"hostId":"host:bridge-race","workspaceId":"workspace:bridge-race","credential":"BRIDGE_RACE_CREDENTIAL","protocolVersion":2})
                    } else if path.ends_with("/account") {
                        let released=release.notified();
                        seen.store(true,Ordering::SeqCst);
                        released.await;
                        json!({"schemaVersion":1,"hostId":"host:bridge-race","workspaceId":"workspace:bridge-race","accountId":"10000000-0000-4000-8000-000000000001","email":"owner@bridge.test","subscription":{"status":"active","memoryAccess":true,"periodEndsAt":null,"checkedAt":patchfleet_core::now()}})
                    } else if path.ends_with("/logout") {
                        assert_eq!(serde_json::from_slice::<Value>(&bytes[start..start+length]).unwrap(),json!({"schemaVersion":1}));
                        json!({"schemaVersion":1,"revoked":true})
                    } else { panic!("unexpected fixture endpoint"); };
                    let body=serde_json::to_vec(&response).unwrap();
                    let head=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                    if socket.write_all(head.as_bytes()).await.is_ok() { let _=socket.write_all(&body).await; }
                });
            }
        });
        let owner = Command::new("project_create", json!({"name":"Before sign-in"}));
        let receipt = bridge
            .runtime
            .block_on(host.action(owner.action, owner.payload.clone()))
            .unwrap();
        let signed_out = host.snapshot().unwrap();
        bridge.pending = Some(owner.clone());
        bridge.busy = true;
        bridge.submit(Command::new(
            "cloud_login_begin",
            json!({"cloudUrl":origin,"displayName":"Bridge race fixture"}),
        ));
        await_account(&mut bridge);
        let authorization = bridge.cloud_login.as_ref().unwrap()["authorizationUrl"]
            .as_str()
            .unwrap();
        let fields: std::collections::BTreeMap<_, _> = authorization
            .split_once('?')
            .unwrap()
            .1
            .split('&')
            .map(|field| field.split_once('=').unwrap())
            .collect();
        let encoded = fields["redirectUri"].as_bytes();
        let mut decoded = Vec::new();
        let mut i = 0;
        while i < encoded.len() {
            if encoded[i] == b'%' {
                decoded.push(
                    u8::from_str_radix(std::str::from_utf8(&encoded[i + 1..i + 3]).unwrap(), 16)
                        .unwrap(),
                );
                i += 3;
            } else {
                decoded.push(encoded[i]);
                i += 1;
            }
        }
        let redirect = String::from_utf8(decoded).unwrap();
        let (authority, path) = redirect
            .strip_prefix("http://")
            .unwrap()
            .split_once('/')
            .unwrap();
        bridge.runtime.block_on(async {
            let mut socket = tokio::net::TcpStream::connect(authority).await.unwrap();
            socket
                .write_all(
                    format!(
                        "GET /{path}?state={}&code={} HTTP/1.1\r\nHost: {authority}\r\n\r\n",
                        fields["state"],
                        "A".repeat(43)
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            let mut response = Vec::new();
            socket.read_to_end(&mut response).await.unwrap();
            assert!(response.starts_with(b"HTTP/1.1 200"));
        });
        bridge.login_next_poll = Some(Instant::now());
        for _ in 0..800 {
            bridge.poll();
            if account_seen.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(account_seen.load(Ordering::SeqCst));
        assert_eq!(bridge.cloud_login.as_ref().unwrap()["status"], "connected");
        let connected_generation = bridge.account_generation;
        bridge
            .output
            .blocking_send(Message::Snapshot(Ok(signed_out.clone())))
            .unwrap();
        bridge.poll();
        assert_eq!(bridge.snapshot.as_ref().unwrap()["cloud"]["paired"], true);
        assert_eq!(bridge.account_generation, connected_generation);
        assert!(bridge.account_task.is_some());
        bridge
            .output
            .blocking_send(Message::Outcome(owner.clone(), Ok(receipt), Ok(signed_out)))
            .unwrap();
        bridge.poll();
        assert_eq!(bridge.feedback.as_ref().unwrap().code, "PROJECT_CREATED");
        assert!(bridge.pending.is_none() && !bridge.busy);
        assert_eq!(bridge.snapshot.as_ref().unwrap()["cloud"]["paired"], true);
        assert_eq!(bridge.account_generation, connected_generation);
        assert!(bridge.account_task.is_some());
        account_release.notify_one();
        await_account(&mut bridge);
        assert_eq!(
            bridge.cloud_account.as_ref().unwrap()["email"],
            "owner@bridge.test"
        );
        let owner = Command::new("project_create", json!({"name":"Before sign-out"}));
        let receipt = bridge
            .runtime
            .block_on(host.action(owner.action, owner.payload.clone()))
            .unwrap();
        let signed_in = host.snapshot().unwrap();
        bridge.pending = Some(owner.clone());
        bridge.busy = true;
        bridge.submit(Command::new("cloud_logout", json!({})));
        await_account(&mut bridge);
        assert_eq!(bridge.snapshot.as_ref().unwrap()["cloud"]["paired"], false);
        assert_eq!(bridge.logout_remote_revoked, Some(true));
        let signed_out_generation = bridge.account_generation;
        bridge
            .output
            .blocking_send(Message::Snapshot(Ok(signed_in.clone())))
            .unwrap();
        bridge.poll();
        assert_eq!(bridge.snapshot.as_ref().unwrap()["cloud"]["paired"], false);
        assert_eq!(bridge.account_generation, signed_out_generation);
        bridge
            .output
            .blocking_send(Message::Outcome(owner, Ok(receipt), Ok(signed_in.clone())))
            .unwrap();
        bridge.poll();
        assert_eq!(bridge.feedback.as_ref().unwrap().code, "PROJECT_CREATED");
        assert!(bridge.pending.is_none() && !bridge.busy);
        assert_eq!(bridge.snapshot.as_ref().unwrap()["cloud"]["paired"], false);
        assert_eq!(bridge.logout_remote_revoked, Some(true));
        assert_eq!(bridge.account_generation, signed_out_generation);
        assert!(bridge.cloud_account.is_none() && bridge.account_task.is_none());
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let invalid = Command::new("project_create", json!({"name":""}));
        let failure = bridge
            .runtime
            .block_on(host.action(invalid.action, invalid.payload.clone()))
            .map_err(safe_error);
        let error = failure.as_ref().unwrap_err().clone();
        bridge.pending = Some(invalid.clone());
        bridge.busy = true;
        bridge
            .output
            .blocking_send(Message::Outcome(invalid.clone(), failure, Ok(signed_in)))
            .unwrap();
        bridge.poll();
        assert_eq!(bridge.pending.as_ref().unwrap().payload, invalid.payload);
        assert_eq!(bridge.feedback.as_ref().unwrap().code, error);
        assert_eq!(bridge.snapshot.as_ref().unwrap()["cloud"]["paired"], false);
        assert_eq!(bridge.account_generation, signed_out_generation);
        server.abort();
        bridge.shutdown();
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
        bridge.submit(Command::new("project_extraction_configure", json!({"projectId":project["projectId"],"expectedPolicyRevision":0,"enabled":true,"consent":true})));
        await_idle(&mut bridge);
        assert!(bridge.snapshot.as_ref().unwrap()["memoryPage"].is_null());
        bridge.submit(Command::new(
            "project_memory_read",
            json!({"projectId":project["projectId"],"query":"currency"}),
        ));
        await_idle(&mut bridge);
        let snapshot = bridge.snapshot.as_ref().unwrap();
        assert_eq!(
            snapshot["memoryPage"]["extractionRevision"],
            snapshot["projectExtraction"]["revision"]
        );
        assert!(snapshot["memoryPage"]["sources"][0]["extractionJob"].is_null());
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
    fn create_job_results_and_native_export_keep_the_original_scope() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("local");
        let mut bridge = Bridge::new(egui::Context::default(), data, false);
        await_idle(&mut bridge);
        bridge.submit(Command::new(
            "project_create",
            json!({"name":"Create bridge proof"}),
        ));
        await_idle(&mut bridge);
        let snapshot = bridge.snapshot.as_ref().unwrap();
        let project = snapshot["projectMemory"]["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "Create bridge proof")
            .unwrap()
            .clone();
        let queued = Command::new(
            "project_work_enqueue",
            json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],"abilityId":"D3","title":"Real brief","instruction":"Prepare a brief from these notes."}),
        );
        bridge.submit(queued.clone());
        await_idle(&mut bridge);
        let snapshot = bridge.snapshot.as_ref().unwrap();
        let receipt = snapshot["projectWorkReceipt"].clone();
        assert_eq!(receipt["commandId"], queued.payload["commandId"]);
        assert_eq!(receipt["projectId"], project["projectId"]);
        assert_eq!(receipt["outcome"], "applied");
        let work = snapshot["work"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["workItemId"] == receipt["workItemId"])
            .unwrap();
        assert_eq!(work["status"], "queued");
        let file =
            PathBuf::from(work["workingDirectory"].as_str().unwrap()).join("outputs/brief.md");
        std::fs::write(&file, "# Scoped brief\nActual output").unwrap();
        let selection =
            json!({"projectId":project["projectId"],"workItemId":receipt["workItemId"]});
        bridge.submit(Command::new("project_results_read", selection.clone()));
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectResults"]["items"][0]["path"],
            "outputs/brief.md"
        );
        let mut read = selection;
        read["path"] = json!("outputs/brief.md");
        bridge.submit(Command::new("project_result_read", read));
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectResult"]["text"],
            "# Scoped brief\nActual output"
        );

        let request = crate::ui::CreateExportRequest {
            project_id: project["projectId"].as_str().unwrap().into(),
            work_item_id: receipt["workItemId"].as_str().unwrap().into(),
            path: "outputs/brief.md".into(),
            name: "brief.md".into(),
            expected_sha256: None,
        };
        bridge.choose_create_export(egui::Context::default(), request.clone(), async { None });
        for _ in 0..400 {
            bridge.poll();
            if !bridge.picking_folder {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!bridge.picking_folder && !bridge.busy);
        let destination = directory.path().join("exported.md");
        let chosen = destination.clone();
        bridge.choose_create_export(
            egui::Context::default(),
            request,
            async move { Some(chosen) },
        );
        for _ in 0..400 {
            bridge.poll();
            if !bridge.picking_folder && !bridge.busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            std::fs::read_to_string(destination).unwrap(),
            "# Scoped brief\nActual output"
        );
        let mut changed = bridge.snapshot.clone().unwrap();
        changed["projectJobs"][0]["runtimeIssue"] = json!("PROJECT_JOB_DIRECTORY_UNAVAILABLE");
        bridge.apply_snapshot(Ok(changed));
        assert!(bridge.snapshot.as_ref().unwrap()["projectResult"].is_null());
        assert!(bridge.snapshot.as_ref().unwrap()["projectResults"].is_null());
        let mut changed = bridge.snapshot.clone().unwrap();
        changed["projectJobs"] = json!([]);
        bridge.apply_snapshot(Ok(changed));
        assert!(bridge.snapshot.as_ref().unwrap()["projectResult"].is_null());
        assert!(bridge.snapshot.as_ref().unwrap()["projectResults"].is_null());
        bridge.shutdown();
    }

    #[test]
    fn conversation_bridge_keeps_exact_receipts_and_durable_project_history() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let create = Command::new("project_create", json!({"name":"Conversation proof"}));
        bridge.submit(create.clone());
        assert!(bridge.snapshot.as_ref().unwrap()["projectCreateReceipt"].is_null());
        await_idle(&mut bridge);
        let project = bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0].clone();
        let created = bridge.snapshot.as_ref().unwrap()["projectCreateReceipt"].clone();
        assert_eq!(created["commandId"], create.payload["commandId"]);
        assert_eq!(created["projectId"], project["projectId"]);
        assert_eq!(created["projectRevision"], project["revision"]);
        assert_eq!(created["outcome"], "applied");
        bridge.submit(create);
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectCreateReceipt"],
            created
        );
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let send = Command::new(
            "project_chat_send",
            json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],"abilityId":"D3","title":"Owner goal","instruction":"Create an editable brief from my notes."}),
        );
        bridge.submit(send.clone());
        await_idle(&mut bridge);
        let receipt = bridge.snapshot.as_ref().unwrap()["projectWorkReceipt"].clone();
        assert_eq!(receipt["commandId"], send.payload["commandId"]);
        assert_eq!(receipt["projectId"], project["projectId"]);
        assert_eq!(receipt["outcome"], "applied");
        for _ in 0..2 {
            bridge.submit(send.clone());
            await_idle(&mut bridge);
            assert_eq!(
                bridge.snapshot.as_ref().unwrap()["projectWorkReceipt"],
                receipt
            );
        }
        bridge.submit(Command::new(
            "project_chat_read",
            json!({"projectId":project["projectId"],"limit":1}),
        ));
        await_idle(&mut bridge);
        let history = bridge.snapshot.as_ref().unwrap()["projectChat"].clone();
        assert_eq!(history["total"], 1);
        assert_eq!(history["items"][0]["goal"], send.payload["instruction"]);
        assert_eq!(history["items"][0]["workItemId"], receipt["workItemId"]);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectCreateReceipt"],
            created
        );
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectWorkReceipt"],
            receipt
        );
        bridge.shutdown();
        let mut reopened =
            Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut reopened);
        reopened.submit(Command::new(
            "project_chat_read",
            json!({"projectId":project["projectId"],"limit":1}),
        ));
        await_idle(&mut reopened);
        assert_eq!(reopened.snapshot.as_ref().unwrap()["projectChat"], history);
        let invalid_create = Command::new("project_create", json!({"name":""}));
        reopened.submit(invalid_create.clone());
        assert!(reopened.snapshot.as_ref().unwrap()["projectCreateReceipt"].is_null());
        await_idle(&mut reopened);
        let failed = &reopened.snapshot.as_ref().unwrap()["projectCreateReceipt"];
        assert_eq!(failed["commandId"], invalid_create.payload["commandId"]);
        assert_eq!(failed["outcome"], "unknown");
        assert!(failed["projectId"].is_null());
        assert_eq!(
            reopened.snapshot.as_ref().unwrap()["projectMemory"]["projects"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        reopened.provider_connections = Some(json!({"providers":[]}));
        reopened.project_create_receipt = Some(created);
        let mut replacement = reopened.snapshot.clone().unwrap();
        replacement["hostEpoch"] = json!("changed-host");
        reopened.apply_snapshot(Ok(replacement));
        assert!(reopened.snapshot.as_ref().unwrap()["projectChat"].is_null());
        assert!(reopened.snapshot.as_ref().unwrap()["providerConnections"].is_null());
        assert!(reopened.snapshot.as_ref().unwrap()["projectCreateReceipt"].is_null());
        reopened.shutdown();
    }

    #[test]
    fn crew_bridge_plans_prepares_and_replays_exact_mission_without_starting_work() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        bridge.submit(Command::new(
            "project_create",
            json!({"name":"Crew bridge proof"}),
        ));
        await_idle(&mut bridge);
        let project = bridge.snapshot.as_ref().unwrap()["projectMemory"]["projects"][0].clone();
        let plan = Command::new(
            "project_crew_plan",
            json!({"projectId":project["projectId"],"instruction":"Research and prepare a presentation.","abilityId":"P1"}),
        );
        bridge.submit(plan.clone());
        await_idle(&mut bridge);
        let result = bridge.snapshot.as_ref().unwrap()["projectCrewPlan"].clone();
        assert_eq!(result["commandId"], plan.payload["commandId"]);
        assert_eq!(result["basis"], "bundled_guides");
        let prepare = Command::new(
            "project_crew_prepare",
            json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],"title":"Launch crew","instruction":plan.payload["instruction"],"abilityId":"P1","members":result["members"],"settings":result["settings"]}),
        );
        bridge.submit(prepare.clone());
        await_idle(&mut bridge);
        let receipt = bridge.snapshot.as_ref().unwrap()["projectCrewReceipt"].clone();
        assert_eq!(receipt["commandId"], prepare.payload["commandId"]);
        assert_eq!(receipt["outcome"], "applied", "{receipt}");
        bridge.submit(prepare.clone());
        await_idle(&mut bridge);
        let snapshot = bridge.snapshot.as_ref().unwrap();
        assert_eq!(snapshot["projectCrewReceipt"], receipt);
        assert_eq!(
            snapshot["projectCrews"]["missions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(snapshot["work"]["items"].as_array().unwrap().is_empty());
        assert!(snapshot["projectCrews"]["messages"].is_null());
        bridge.submit(Command::new(
            "project_crew_messages_read",
            json!({"projectId":project["projectId"],"missionId":receipt["missionId"]}),
        ));
        await_idle(&mut bridge);
        assert_eq!(
            bridge.snapshot.as_ref().unwrap()["projectCrewMessages"]["total"],
            0
        );
        let mut changed = bridge.snapshot.clone().unwrap();
        changed["hostEpoch"] = json!("new-host");
        bridge.apply_snapshot(Ok(changed));
        assert!(bridge.snapshot.as_ref().unwrap()["projectCrewPlan"].is_null());
        assert!(bridge.snapshot.as_ref().unwrap()["projectCrewMessages"].is_null());
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

    #[test]
    fn late_create_read_never_replaces_a_failed_owner_retry() {
        let directory = tempfile::tempdir().unwrap();
        let mut bridge = Bridge::new(egui::Context::default(), directory.path().to_owned(), false);
        await_idle(&mut bridge);
        let owner = Command::new("project_work_enqueue", json!({"title":"Keep draft"}));
        for action in [
            "project_results_read",
            "project_result_read",
            "project_chat_read",
            "project_crew_plan",
            "project_crew_messages_read",
            "provider_connections_read",
        ] {
            for outcome in [
                Ok(json!({"text":"FOREIGN_CANARY"})),
                Err("HOST_ACTION_FAILED".into()),
            ] {
                bridge.pending = Some(owner.clone());
                bridge.feedback = Some(Notice::error(owner.action, "HOST_ACTION_FAILED"));
                let before = bridge.snapshot.clone();
                bridge
                    .output
                    .blocking_send(Message::Outcome(
                        Command::new(
                            action,
                            json!({"projectId":"foreign","workItemId":"foreign"}),
                        ),
                        outcome,
                        before.clone().ok_or_else(|| "NO_SNAPSHOT".into()),
                    ))
                    .unwrap();
                bridge.poll();
                assert_eq!(bridge.pending.as_ref().unwrap().payload, owner.payload);
                assert_eq!(bridge.snapshot, before);
            }
        }
        bridge.shutdown();
    }
}
