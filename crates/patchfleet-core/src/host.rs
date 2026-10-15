use crate::{
    agent_memory,
    cloud::Cloud,
    decisions, desktop_login, domain, event, extraction, now, project_memory, providers, retrieval,
    shared_memory,
    store::Store,
    teams::{catalog, templates},
    work::{array, find, intent, text},
    Error, Result,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
};
use tokio::sync::Mutex as AsyncMutex;

#[cfg(test)]
#[path = "host_memory_tests.rs"]
mod memory_tests;

#[path = "host_extraction.rs"]
mod extraction_runtime;
#[cfg(test)]
#[path = "host_extraction_tests.rs"]
mod extraction_tests;

pub struct Host {
    pub(crate) store: Store,
    pub(crate) epoch: String,
    pub(crate) controller: providers::Controller,
    data_dir: PathBuf,
    operations: AsyncMutex<()>,
    cloud: AsyncMutex<Option<Cloud>>,
    cloud_status: Mutex<Value>,
    desktop_preferences: Mutex<Value>,
    desktop_login: Mutex<Option<desktop_login::Login>>,
    self_ref: Weak<Host>,
    agent_memory_server: AsyncMutex<Option<agent_memory::Server>>,
    agent_memory_progress: Mutex<std::collections::BTreeMap<String, AgentMemoryProgress>>,
    extraction_runtime: Mutex<extraction_runtime::Runtime>,
    memory_download: AsyncMutex<Option<shared_memory::Download>>,
    memory_poll: Mutex<std::collections::BTreeMap<String, std::time::Instant>>,
    shutdown_notify: tokio::sync::Notify,
    pub(crate) closing: AtomicBool,
}
struct AgentMemoryProgress {
    revision: u64,
    next_offset: u64,
    complete: bool,
    active: Weak<AtomicBool>,
    issue: Option<String>,
}
fn second_event(kind: &str, payload: Value) -> Value {
    let mut e = event(kind, payload);
    e["schemaVersion"] = json!(2);
    e
}
fn owner_event(kind: &str, payload: Value, version: u64) -> Value {
    let mut e = event(kind, payload);
    e["schemaVersion"] = json!(version);
    e
}
fn public_templates() -> Vec<Value> {
    templates()
        .into_iter()
        .map(|mut t| {
            t.as_object_mut().unwrap().remove("roles");
            t
        })
        .collect()
}
fn public_connection(status: &Value) -> Value {
    if status["paired"] == true {
        json!({"cloudUrl":status["cloudUrl"],"workspaceId":status["workspaceId"],"hostId":status["hostId"]})
    } else {
        Value::Null
    }
}
fn exact_action(action: &str, p: &Value) -> Result<()> {
    let allowed: &[&str] = match action {
        "observe" | "cloud_unpair" | "cloud_sync" | "cloud_login_poll" | "cloud_login_cancel"
        | "cloud_account_read" | "cloud_logout" => &[],
        "cloud_login_begin" => &["cloudUrl", "displayName"],
        "workspace_register" => &["workingDirectory", "displayName"],
        "workspace_remove" => &["workspaceId", "expectedWorkspaceRevision"],
        "project_create" => &["name"],
        "project_link_workspace" | "project_unlink_workspace" => {
            &["projectId", "workspaceId", "expectedProjectRevision"]
        }
        "project_message" => &["projectId", "title", "text", "expectedProjectRevision"],
        "project_source_edit" => &[
            "projectId",
            "sourceId",
            "title",
            "text",
            "expectedSourceRevision",
        ],
        "project_memory_read" => &["projectId", "offset", "limit", "query"],
        "project_extraction_configure" => {
            &["projectId", "expectedPolicyRevision", "enabled", "consent"]
        }
        "project_extraction_enqueue" => &["projectId", "sourceId", "expectedSourceRevision"],
        "project_extraction_retry" => &["projectId", "jobId", "expectedAttempt"],
        "project_decision_propose" => &[
            "projectId",
            "expectedProjectRevision",
            "title",
            "context",
            "decision",
            "rationale",
            "consequences",
            "sources",
        ],
        "project_decision_accept" | "project_decision_reject" => {
            &["projectId", "decisionId", "expectedDecisionRevision"]
        }
        "project_decision_supersede" => &[
            "projectId",
            "decisionId",
            "expectedDecisionRevision",
            "replacementDecisionId",
            "expectedReplacementRevision",
        ],
        "project_decisions_read" => &["projectId", "offset", "limit", "query", "status"],
        "project_decision_read" => &["projectId", "decisionId"],
        "project_decision_search" | "memory_cloud_search" => &["projectId", "query", "limit"],
        "memory_cloud_create" => &["projectId", "expectedProjectRevision", "name"],
        "memory_cloud_link" => &["projectId", "expectedProjectRevision", "sharedProjectId"],
        "memory_cloud_publish" => &[
            "projectId",
            "decisionId",
            "expectedDecisionRevision",
            "expectedSharedRevision",
            "consent",
        ],
        "memory_cloud_sharing" => &["projectId", "expectedSharedRevision", "enabled"],
        "memory_cloud_sync" => &["projectId"],
        "memory_cloud_read" => &["projectId", "offset", "limit"],
        "memory_cloud_decision_read" => &["projectId", "decisionId", "revision"],
        "desktop_language" => &["language"],
        "work_enqueue" => &["title", "instruction", "workspaceId", "workingDirectory"],
        "work_start" | "work_retry" | "work_remove" => &["workItemId", "expectedItemRevision"],
        "work_cancel" => &["runId", "expectedRunRevision"],
        "pack_install" => &["manifest"],
        "pack_remove" => &["packId"],
        "team_create" => &[
            "name",
            "goal",
            "workspaceId",
            "templateId",
            "orchestratorPackId",
            "workerPackIds",
            "settings",
        ],
        "team_start" | "team_advance" | "team_cancel" => &["teamId", "expectedTeamRevision"],
        "team_cancel_agent" => &["teamId", "agentId", "expectedTeamRevision"],
        "team_approve" => &[
            "teamId",
            "taskId",
            "decision",
            "note",
            "expectedTeamRevision",
        ],
        "team_answer" => &["teamId", "questionId", "answer", "expectedTeamRevision"],
        "cloud_pair" => &["cloudUrl", "displayName", "pairingCode"],
        _ => return Err(Error("UNKNOWN_ACTION".into())),
    };
    let obj = p
        .as_object()
        .ok_or_else(|| Error("INVALID_PAYLOAD".into()))?;
    if serde_json::to_vec(p)?.len() > Host::action_payload_limit(action)
        || obj.keys().any(|k| {
            !allowed.contains(&k.as_str())
                && !["commandId", "createdAt", "expiresAt"].contains(&k.as_str())
        })
    {
        return Err(Error("INVALID_PAYLOAD".into()));
    }
    Ok(())
}

impl Host {
    pub fn action_payload_limit(action: &str) -> usize {
        if matches!(
            action,
            "project_message" | "project_source_edit" | "project_decision_propose"
        ) {
            192 * 1024
        } else {
            128 * 1024
        }
    }
    pub fn open(data_dir: PathBuf) -> Result<Arc<Self>> {
        Self::open_with_controller(data_dir, providers::Controller::new())
    }
    fn open_with_controller(
        data_dir: PathBuf,
        controller: providers::Controller,
    ) -> Result<Arc<Self>> {
        if !data_dir.is_absolute() {
            return Err(Error("DATA_DIRECTORY_MUST_BE_ABSOLUTE".into()));
        }
        // The legacy host has no cross-runtime file lock. Its old runtime marker
        // therefore requires an explicit offline copy instead of unsafe takeover.
        if data_dir.join("runtime.json").exists() {
            return Err(Error(
                "LEGACY_HOST_DIRECTORY: stop the legacy host and import an offline copy".into(),
            ));
        }
        let store = Store::open(data_dir.clone())?;
        let cloud = Cloud::open(&data_dir).ok();
        let status = cloud
            .as_ref()
            .map(Cloud::public_status)
            .unwrap_or(json!({"paired":false,"error":true,"lastErrorCode":"CLOUD_STATE_INVALID"}));
        let host = Arc::new_cyclic(|weak| Self {
            store,
            epoch: uuid::Uuid::new_v4().to_string(),
            controller,
            desktop_preferences: Mutex::new(crate::desktop_preferences::load(&data_dir)),
            desktop_login: Mutex::new(None),
            self_ref: weak.clone(),
            agent_memory_server: AsyncMutex::new(None),
            agent_memory_progress: Mutex::new(std::collections::BTreeMap::new()),
            extraction_runtime: Mutex::new(extraction_runtime::Runtime::default()),
            memory_download: AsyncMutex::new(None),
            memory_poll: Mutex::new(std::collections::BTreeMap::new()),
            shutdown_notify: tokio::sync::Notify::new(),
            data_dir,
            operations: AsyncMutex::new(()),
            cloud: AsyncMutex::new(cloud),
            cloud_status: Mutex::new(status),
            closing: AtomicBool::new(false),
        });
        host.store
            .transaction(|events| Ok((project_memory::recover(events)?, Value::Null)))?;
        host.store
            .transaction(|events| Ok((decisions::recover(events)?, Value::Null)))?;
        host.store.transaction(|events| {
            let recovered = if events.iter().any(|event| event["schemaVersion"] == 7) {
                extraction::recover(events)?
            } else {
                Vec::new()
            };
            Ok((recovered, Value::Null))
        })?;
        host.clean_extraction_scratch();
        host.store
            .transaction(|events| Ok((shared_memory::recover(events)?, Value::Null)))?;
        let events = host.store.read_events()?;
        let workspace_receipts: std::collections::HashSet<_> = events
            .iter()
            .filter(|e| e["type"] == "workspace.command.receipted")
            .filter_map(|e| e["payload"]["receipt"]["intentId"].as_str())
            .collect();
        for e in events.iter().filter(|e| {
            e["type"] == "workspace.command.requested"
                && !workspace_receipts
                    .contains(e["payload"]["intent"]["intentId"].as_str().unwrap_or(""))
        }) {
            host.apply_workspace_request(e["payload"]["intent"].clone())?;
        }
        let receipts: std::collections::HashSet<_> = events
            .iter()
            .filter(|e| e["type"] == "command.receipted")
            .filter_map(|e| e["payload"]["receipt"]["intentId"].as_str())
            .collect();
        for e in events.iter().filter(|e| {
            e["type"] == "command.requested"
                && !receipts.contains(e["payload"]["intent"]["intentId"].as_str().unwrap_or(""))
                && matches!(
                    e["payload"]["intent"]["type"].as_str(),
                    Some("enqueue_work" | "remove_queued_work")
                )
        }) {
            host.apply_queue_intent(e["payload"]["intent"].clone())?;
        }
        host.reconcile_ownership()?;
        host.store.transaction(|events| {
            let pending=events.iter().filter(|e|e["type"]=="owner.command.requested" && !events.iter().any(|r|r["type"]=="owner.command.receipted" && r["payload"]["commandId"]==e["payload"]["commandId"]));
            let additions=pending.map(|e|owner_event("owner.command.receipted",json!({"commandId":e["payload"]["commandId"],"result":{"outcome":"failed","reasonCode":"OWNER_COMMAND_OUTCOME_UNKNOWN"}}),e["schemaVersion"].as_u64().unwrap())).collect();
            Ok((additions,Value::Null))
        })?;
        Ok(host)
    }

    pub fn snapshot(&self) -> Result<Value> {
        let mut bundle = self.store.runtime_projections()?;
        self.extraction_overlay(&mut bundle);
        if let Some(contexts) = bundle["agentMemory"]["contexts"].as_array_mut() {
            let progress = self
                .agent_memory_progress
                .lock()
                .map_err(|_| Error("AGENT_MEMORY_UNAVAILABLE".into()))?;
            for context in contexts {
                let state = context["runId"].as_str().and_then(|run| progress.get(run));
                context["manifestLoaded"] = json!(state.is_some_and(|p| p.complete
                    && Some(p.revision) == context["currentDecisionRevision"].as_u64()));
                context["issue"] = state
                    .and_then(|p| p.issue.as_deref())
                    .map_or(Value::Null, |s| json!(s));
            }
        }
        if bundle["work"].is_null() {
            bundle["work"] =
                json!({"schemaVersion":1,"revision":0,"items":[],"runs":[],"receipts":[]});
        }
        if bundle["workspaces"].is_null() {
            bundle["workspaces"] = json!({"schemaVersion":1,"revision":0,"items":[],"receipts":[]});
        }
        if bundle["teams"].is_null() {
            bundle["teams"] = json!({"schemaVersion":1,"revision":0,"items":[]});
        }
        let workspaces = array(&bundle["workspaces"], "items").to_vec();
        for team in bundle["teams"]["items"].as_array_mut().unwrap() {
            if matches!(
                team["status"].as_str(),
                Some("draft" | "active" | "waiting")
            ) && !workspaces
                .iter()
                .any(|w| w["workspaceId"] == team["workspaceId"])
            {
                team["runtimeIssue"] = json!("WORKSPACE_NOT_REGISTERED");
            }
        }
        let packs = catalog(&bundle);
        bundle["packs"] = json!(packs);
        bundle["templates"] = json!(public_templates());
        bundle["cloud"] = self
            .cloud_status
            .lock()
            .map_err(|_| Error("HOST_STATE_UNAVAILABLE".into()))?
            .clone();
        bundle["sharedMemory"] = self
            .store
            .shared_metadata(&public_connection(&bundle["cloud"]))?;
        bundle["appVersion"] = json!(env!("CARGO_PKG_VERSION"));
        bundle["hostEpoch"] = json!(self.epoch);
        bundle["desktopPreferences"] = self
            .desktop_preferences
            .lock()
            .map_err(|_| Error("HOST_STATE_UNAVAILABLE".into()))?
            .clone();
        Ok(bundle)
    }

    pub async fn action(&self, action: &str, payload: Value) -> Result<Value> {
        let result = self.action_inner(action, payload).await;
        if result.is_ok()
            && (action.starts_with("project_extraction_")
                || matches!(
                    action,
                    "project_message"
                        | "project_source_edit"
                        | "work_enqueue"
                        | "work_retry"
                        | "team_create"
                        | "team_answer"
                        | "team_approve"
                ))
        {
            let _guard = self.operations.lock().await;
            if !self.closing.load(Ordering::SeqCst) {
                // Source acknowledgement stays truthful even if its optional job cannot queue.
                let _ = self.pump_extraction(false).await;
            }
        }
        result
    }

    async fn action_inner(&self, action: &str, mut payload: Value) -> Result<Value> {
        let account_action = matches!(
            action,
            "cloud_login_begin"
                | "cloud_login_poll"
                | "cloud_login_cancel"
                | "cloud_account_read"
                | "cloud_logout"
        );
        exact_action(action, &payload).map_err(|e| {
            if account_action {
                Error("DESKTOP_INVALID_PAYLOAD".into())
            } else {
                e
            }
        })?;
        if account_action {
            return match action {
                "cloud_login_begin" => self.desktop_await(self.login_begin(&payload)).await,
                "cloud_login_poll" => self.login_poll().await,
                "cloud_login_cancel" => {
                    self.cancel_login();
                    Ok(json!({"schemaVersion":1,"status":"cancelled"}))
                }
                "cloud_account_read" => self.desktop_await(self.account_read()).await,
                _ => self.account_logout().await,
            };
        }
        // An ephemeral network read must not block Local commands behind its wait.
        if matches!(action, "project_decision_search" | "memory_cloud_search") {
            let stopped = self.shutdown_notify.notified();
            if self.closing.load(Ordering::SeqCst) {
                return Err(Error("HOST_CLOSING".into()));
            }
            if action == "project_decision_search" {
                return self.store.search_decisions(&payload);
            }
            for key in ["commandId", "createdAt", "expiresAt"] {
                payload.as_object_mut().unwrap().remove(key);
            }
            // Include credential-lock admission and the final scope check in the budget.
            return tokio::select! {
                _ = stopped => Err(Error("HOST_CLOSING".into())),
                result = tokio::time::timeout(std::time::Duration::from_secs(3), self.search_shared_memory(&payload)) => result.unwrap_or_else(|_| Err(Error("MEMORY_UNAVAILABLE".into())))
            };
        }
        let _guard = self.operations.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("HOST_CLOSING".into()));
        }
        if action == "project_memory_read" {
            return self.store.read_memory(&payload);
        }
        if action == "memory_cloud_read" {
            return self
                .store
                .read_shared_memory(&payload, &self.memory_connection().await?);
        }
        if action == "memory_cloud_decision_read" {
            return self
                .store
                .read_shared_decision(&payload, &self.memory_connection().await?);
        }
        if action.starts_with("memory_cloud_") {
            let connection = self.memory_connection().await?;
            let receipt = self.store.transaction(|events| {
                shared_memory::apply(events, action, &payload, &connection)
            })?;
            if action == "memory_cloud_sync" && receipt["outcome"] == "applied" {
                let _ = self
                    .sync_shared_memory(payload["projectId"].as_str(), true)
                    .await;
            }
            return Ok(receipt);
        }
        if action == "project_decisions_read" {
            return self.store.read_decisions(&payload);
        }
        if action == "project_decision_read" {
            return self.store.read_decision(&payload);
        }
        if action.starts_with("project_extraction_") {
            return self
                .store
                .transaction(|events| extraction::apply(events, action, &payload));
        }
        if action.starts_with("project_decision_") {
            return self
                .store
                .transaction(|events| decisions::apply(events, action, &payload));
        }
        if action.starts_with("project_") {
            return self
                .store
                .transaction(|events| project_memory::apply(events, action, &payload));
        }
        let audit = action.starts_with("team_")
            || action.starts_with("pack_")
            || action == "desktop_language";
        let audit_version = if action == "desktop_language" { 3 } else { 2 };
        let mut command = Value::Null;
        if audit {
            let base = intent("owner_action", Value::Null, &payload);
            payload["createdAt"] = base["createdAt"].clone();
            payload["expiresAt"] = base["expiresAt"].clone();
            let created = chrono::DateTime::parse_from_rfc3339(text(&payload, "createdAt")?)
                .map_err(|_| Error("INVALID_COMMAND_TIME".into()))?;
            let expires = chrono::DateTime::parse_from_rfc3339(text(&payload, "expiresAt")?)
                .map_err(|_| Error("INVALID_COMMAND_TIME".into()))?;
            if created.to_rfc3339_opts(chrono::SecondsFormat::Millis, true) != payload["createdAt"]
                || expires.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                    != payload["expiresAt"]
                || expires <= created
                || expires.signed_duration_since(created).num_milliseconds() > 300_000
            {
                return Err(Error("INVALID_COMMAND_TIME".into()));
            }
            if payload["commandId"].is_null() {
                payload["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
            }
            command = payload["commandId"].clone();
            let requested = json!({"commandId":command,"action":action,"payload":payload});
            let prior=self.store.transaction(|events| {
                if let Some(e)=events.iter().find(|e|e["type"]=="owner.command.requested" && e["payload"]["commandId"]==command) {
                    if e["payload"]!=requested {return Err(Error("IDEMPOTENCY_KEY_REUSED".into()));}
                    let r=events.iter().find(|r|r["type"]=="owner.command.receipted" && r["payload"]["commandId"]==command);
                    return Ok((vec![],r.map(|r|r["payload"]["result"].clone()).unwrap_or(json!({"outcome":"failed","reasonCode":"OWNER_COMMAND_OUTCOME_UNKNOWN"}))));
                }
                if created > chrono::Utc::now() + chrono::Duration::seconds(60) {
                    return Err(Error("INVALID_COMMAND_TIME".into()));
                }
                Ok((vec![owner_event("owner.command.requested",requested,audit_version)],Value::Null))
            })?;
            if !prior.is_null() {
                return Ok(prior);
            }
        }
        let result = if audit && text(&payload, "expiresAt")? <= now().as_str() {
            Ok(json!({"outcome":"expired","reasonCode":"COMMAND_EXPIRED"}))
        } else {
            self.execute(action, &payload).await
        };
        if audit {
            let recorded = match &result {
                Ok(v) => v.clone(),
                Err(_) => json!({"outcome":"rejected","reasonCode":"INVALID_ACTION"}),
            };
            self.store.append(vec![owner_event(
                "owner.command.receipted",
                json!({"commandId":command,"result":recorded}),
                audit_version,
            )])?;
        }
        result
    }

    async fn execute(&self, action: &str, p: &Value) -> Result<Value> {
        if action.starts_with("team_")
            && action != "team_create"
            && !p["expectedTeamRevision"].is_null()
        {
            let team = find(
                &self.store.runtime_projections()?["teams"],
                "teamId",
                &p["teamId"],
            )
            .ok_or_else(|| Error("TEAM_NOT_FOUND".into()))?;
            if team["revision"] != p["expectedTeamRevision"] {
                return Err(Error("STALE_TEAM_REVISION".into()));
            }
        }
        match action {
            "desktop_language" => {
                let value = crate::desktop_preferences::save(&self.data_dir, text(p,"language")?)?;
                *self.desktop_preferences.lock().map_err(|_| Error("HOST_STATE_UNAVAILABLE".into()))? = value.clone();
                Ok(json!({"reasonCode":"DESKTOP_LANGUAGE_SET","language":value["language"]}))
            },
            "observe"=>{self.observe().await?;self.advance_all().await?;Ok(json!({"reasonCode":"OBSERVED"}))},
            "workspace_register"|"workspace_remove"=>self.workspace_action(action,p).await,
            "work_enqueue"=>{
                let selected=p["workspaceId"].as_str().unwrap_or(""); let manual=p["workingDirectory"].as_str().unwrap_or("").trim();
                if !selected.is_empty() && !manual.is_empty() {return Err(Error("WORKSPACE_SELECTION_CONFLICT".into()));}
                let directory=if selected.is_empty() {providers::validate_workspace(manual).await.map_err(Error)?}
                    else {text(&find(&self.store.runtime_projections()?["workspaces"],"workspaceId",&json!(selected)).ok_or_else(||Error("WORKSPACE_NOT_REGISTERED".into()))?,"workingDirectory")?.to_owned()};
                let base=intent("enqueue_work",Value::Null,p);
                let item=json!({"schemaVersion":1,"workItemId":format!("work:{}",text(&base,"intentId")?.trim_start_matches("cmd:")),
                    "title":text(p,"title")?.trim(),"instruction":text(p,"instruction")?.trim(),"providerId":"codex","workingDirectory":directory,"status":"queued","createdAt":base["createdAt"],"revision":1});
                let workspaces=self.store.runtime_projections()?["workspaces"].clone();
                let workspace_id=array(&workspaces,"items").iter().find(|w|w["workingDirectory"]==item["workingDirectory"]).and_then(|w|w["workspaceId"].as_str());
                let project=workspace_id.map(|id|self.store.workspace_project(id)).transpose()?.flatten();
                let mut i=base;i["payload"]=json!({"workItem":item});self.apply_queue_intent_bound(i,project.as_deref())
            },
            "work_start"|"work_remove"=>self.apply_work_intent(intent(if action=="work_start" {"start_work"} else {"remove_queued_work"},json!({"workItemId":p["workItemId"],"expectedItemRevision":p["expectedItemRevision"]}),p)).await,
            "work_cancel"=>self.apply_work_intent(intent("cancel_run",json!({"runId":p["runId"],"expectedRunRevision":p["expectedRunRevision"]}),p)).await,
            "work_retry"=>{
                let item=find(&self.store.runtime_projections()?["work"],"workItemId",&p["workItemId"]).ok_or_else(||Error("WORK_ITEM_NOT_FOUND".into()))?;
                if !matches!(item["status"].as_str(),Some("failed"|"interrupted"|"blocked")) || item["revision"]!=p["expectedItemRevision"] {return Err(Error("WORK_RETRY_UNAVAILABLE".into()));}
                let mut new=item.clone();let base=intent("enqueue_work",Value::Null,p);
                new["workItemId"]=json!(format!("work:{}",text(&base,"intentId")?.trim_start_matches("cmd:")));
                new["status"]=json!("queued");new["revision"]=json!(1);new["createdAt"]=base["createdAt"].clone();
                let project=self.store.subject_project("work",text(&item,"workItemId")?)?;
                let mut i=base;i["payload"]=json!({"workItem":new});self.apply_queue_intent_bound(i,project.as_deref())
            },
            "pack_install"=>self.install_pack(&p["manifest"]),
            "pack_remove"=>self.remove_pack(&p["packId"]),
            "team_create"=>self.create_team(p),
            "team_start"=>{
                let bundle=self.store.runtime_projections()?;
                let team=find(&bundle["teams"],"teamId",&p["teamId"]).ok_or_else(||Error("TEAM_NOT_FOUND".into()))?;
                if find(&bundle["workspaces"],"workspaceId",&team["workspaceId"]).is_none() {return Err(Error("WORKSPACE_NOT_REGISTERED".into()));}
                self.team_event("team.started",json!({"teamId":p["teamId"]}))?;self.advance_team(&p["teamId"]).await
            },
            "team_advance"=>self.advance_team(&p["teamId"]).await,
            "team_cancel"|"team_cancel_agent"=>{
                let team=find(&self.store.runtime_projections()?["teams"],"teamId",&p["teamId"]).ok_or_else(||Error("TEAM_NOT_FOUND".into()))?;
                if !p["expectedTeamRevision"].is_null() && p["expectedTeamRevision"]!=team["revision"] {return Err(Error("STALE_TEAM_REVISION".into()));}
                self.team_event(if action=="team_cancel" {"team.cancelled"} else {"team.agent.cancelled"},if action=="team_cancel" {json!({"teamId":p["teamId"]})} else {json!({"teamId":p["teamId"],"agentId":p["agentId"]})})?;
                self.stop_team_work(&team,if action=="team_cancel_agent" {Some(&p["agentId"])} else {None}).await?;
                Ok(json!({"outcome":"applied","reasonCode":if action=="team_cancel" {"TEAM_CANCELLED"} else {"AGENT_CANCELLED"}}))
            },
            "team_approve"=>{
                let note=if p["note"].as_str().map(|s|s.trim().is_empty()).unwrap_or(true) {Value::Null} else {json!(text(p,"note")?.trim())};
                self.team_event("team.approval.decided",json!({"teamId":p["teamId"],"taskId":p["taskId"],"decision":p["decision"],"note":note}))?;
                self.advance_team(&p["teamId"]).await
            },
            "team_answer"=>{self.team_event("team.question.answered",json!({"teamId":p["teamId"],"questionId":p["questionId"],"answer":text(p,"answer")?.trim()}))?;self.advance_team(&p["teamId"]).await},
            "cloud_pair"=>{self.cancel_login();let mut state=self.cloud.lock().await;let c=state.as_mut().ok_or_else(||Error("CLOUD_RECOVERY_REQUIRED".into()))?;let result=c.pair(&json!({"cloudUrl":p["cloudUrl"],"displayName":p["displayName"],"pairingCode":p["pairingCode"]})).await.map_err(Error);self.cache_cloud(c)?;result},
            "cloud_unpair"=>{self.cancel_login();let mut state=self.cloud.lock().await;let c=state.as_mut().ok_or_else(||Error("CLOUD_RECOVERY_REQUIRED".into()))?;let result=c.unpair().map_err(Error);self.cache_cloud(c)?;result},
            "cloud_sync"=>Box::pin(self.sync_cloud()).await,
            _=>Err(Error("UNKNOWN_ACTION".into()))
        }
    }

    fn cache_cloud(&self, cloud: &Cloud) -> Result<()> {
        *self
            .cloud_status
            .lock()
            .map_err(|_| Error("HOST_STATE_UNAVAILABLE".into()))? = cloud.public_status();
        Ok(())
    }
    fn cancel_login(&self) {
        if let Ok(mut login) = self.desktop_login.lock() {
            if let Some(login) = login.as_mut() {
                login.cancel();
            }
        }
    }
    async fn desktop_await(
        &self,
        work: impl std::future::Future<Output = Result<Value>>,
    ) -> Result<Value> {
        let stopped = self.shutdown_notify.notified();
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("DESKTOP_CLOSING".into()));
        }
        tokio::select! {
            _=stopped=>Err(Error("DESKTOP_CLOSING".into())),
            result=tokio::time::timeout(std::time::Duration::from_secs(3),work)=>result.unwrap_or_else(|_|Err(Error("DESKTOP_UNAVAILABLE".into())))
        }
    }
    async fn login_begin(&self, payload: &Value) -> Result<Value> {
        let command = payload
            .get("commandId")
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| Error("DESKTOP_INVALID_COMMAND".into()))
            })
            .transpose()?
            .unwrap_or_else(|| format!("cmd:{}", uuid::Uuid::new_v4()));
        command
            .strip_prefix("cmd:")
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .filter(|id| {
                format!("cmd:{id}") == command
                    && id.get_variant() == uuid::Variant::RFC4122
                    && (1..=8).contains(&id.get_version_num())
            })
            .ok_or_else(|| Error("DESKTOP_INVALID_COMMAND".into()))?;
        let input = json!({"cloudUrl":payload["cloudUrl"],"displayName":payload["displayName"]});
        let _guard = self.operations.lock().await;
        {
            let login = self
                .desktop_login
                .lock()
                .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?;
            if let Some(login) = login.as_ref() {
                if login.matches(&command, &input)? {
                    return login.status();
                }
            }
        }
        let (installation, bound) = {
            let cloud = self.cloud.lock().await;
            let cloud = cloud
                .as_ref()
                .ok_or_else(|| Error("DESKTOP_STATE_FAILED".into()))?;
            (
                cloud.installation_id().to_string(),
                cloud.memory_connection(),
            )
        };
        if !bound.is_null() {
            return Err(Error("DESKTOP_ALREADY_SIGNED_IN".into()));
        }
        self.cancel_login();
        let login = desktop_login::Login::begin(command, input, installation, bound).await?;
        let result = login.status()?;
        let mut current = self
            .desktop_login
            .lock()
            .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?;
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("DESKTOP_CLOSING".into()));
        }
        *current = Some(login);
        Ok(result)
    }
    async fn login_poll(&self) -> Result<Value> {
        let exchange = {
            let mut flow = self
                .desktop_login
                .lock()
                .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?;
            let Some(flow) = flow.as_mut() else {
                return Ok(json!({"schemaVersion":1,"status":"cancelled"}));
            };
            let exchange = flow.exchange()?;
            if exchange.is_none() {
                return flow.status();
            }
            exchange.unwrap()
        };
        let result = self
            .desktop_await(async {
                let request = {
                    let cloud = self.cloud.lock().await;
                    cloud
                        .as_ref()
                        .ok_or_else(|| Error("DESKTOP_STATE_FAILED".into()))?
                        .desktop_token_request(&exchange.origin, &exchange.body)
                        .map_err(Error)?
                };
                let response = desktop_login::response(request).await?;
                let _guard = self.operations.lock().await;
                let mut cloud = self.cloud.lock().await;
                let cloud = cloud
                    .as_mut()
                    .ok_or_else(|| Error("DESKTOP_STATE_FAILED".into()))?;
                if self.closing.load(Ordering::SeqCst) {
                    return Err(Error("DESKTOP_CLOSING".into()));
                }
                if cloud.memory_connection() != exchange.bound
                    || cloud.installation_id() != exchange.installation
                {
                    return Err(Error("DESKTOP_CONNECTION_CHANGED".into()));
                }
                let mut flow = self
                    .desktop_login
                    .lock()
                    .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?;
                let flow = flow
                    .as_mut()
                    .filter(|flow| flow.is_exchange(&exchange.generation))
                    .ok_or_else(|| Error("DESKTOP_LOGIN_CANCELLED".into()))?;
                cloud
                    .adopt_desktop_pairing(&exchange.origin, &response)
                    .map_err(Error)?;
                self.cache_cloud(cloud)
                    .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?;
                flow.finish(Ok(()));
                flow.status()
            })
            .await;
        if let Err(error) = &result {
            if let Ok(mut flow) = self.desktop_login.lock() {
                if let Some(flow) = flow
                    .as_mut()
                    .filter(|flow| flow.is_exchange(&exchange.generation))
                {
                    flow.finish(Err(error.0.clone()));
                }
            }
        }
        result
    }
    async fn account_read(&self) -> Result<Value> {
        let (bound, request) = {
            let cloud = self.cloud.lock().await;
            cloud
                .as_ref()
                .ok_or_else(|| Error("DESKTOP_STATE_FAILED".into()))?
                .desktop_account_request(false)
                .map_err(Error)?
        };
        let response = desktop_login::response(request).await?;
        let _guard = self.operations.lock().await;
        let cloud = self.cloud.lock().await;
        let cloud = cloud
            .as_ref()
            .ok_or_else(|| Error("DESKTOP_STATE_FAILED".into()))?;
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("DESKTOP_CLOSING".into()));
        }
        if cloud.memory_connection() != bound {
            return Err(Error("DESKTOP_CONNECTION_CHANGED".into()));
        }
        desktop_login::validate_account_response(&response, &cloud.public_status())?;
        Ok(response)
    }
    async fn account_logout(&self) -> Result<Value> {
        self.cancel_login();
        let bound = public_connection(
            &*self
                .cloud_status
                .lock()
                .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?,
        );
        let remote = self
            .desktop_await(async {
                let (scope, request) = {
                    let cloud = self.cloud.lock().await;
                    cloud
                        .as_ref()
                        .ok_or_else(|| Error("DESKTOP_STATE_FAILED".into()))?
                        .desktop_account_request(true)
                        .map_err(Error)?
                };
                if scope != bound {
                    return Err(Error("DESKTOP_CONNECTION_CHANGED".into()));
                }
                let response = desktop_login::response(request).await?;
                if response != json!({"schemaVersion":1,"revoked":true}) {
                    return Err(Error("DESKTOP_PROTOCOL_INVALID".into()));
                }
                Ok(response)
            })
            .await
            .is_ok();
        let _guard = self.operations.lock().await;
        let mut cloud = self.cloud.lock().await;
        let cloud = cloud
            .as_mut()
            .ok_or_else(|| Error("DESKTOP_STATE_FAILED".into()))?;
        let current = cloud.memory_connection();
        if !current.is_null() && current != bound {
            return Err(Error("DESKTOP_CONNECTION_CHANGED".into()));
        }
        cloud
            .unpair()
            .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?;
        self.cache_cloud(cloud)
            .map_err(|_| Error("DESKTOP_STATE_FAILED".into()))?;
        Ok(json!({"schemaVersion":1,"localSignedOut":true,"remoteRevoked":remote}))
    }

    async fn workspace_action(&self, action: &str, p: &Value) -> Result<Value> {
        let mut request = if action == "workspace_register" {
            let base = intent("register_workspace", Value::Null, p);
            let directory = text(p, "workingDirectory")?;
            let canonical_replay = self.store.transaction(|events| {
                let previous = events.iter().find(|e| {
                    let old = &e["payload"]["intent"];
                    e["type"] == "workspace.command.requested"
                        && (old["intentId"] == base["intentId"]
                            || old["idempotencyKey"] == base["idempotencyKey"])
                });
                let replay = previous.is_some_and(|e| {
                    let old = &e["payload"]["intent"];
                    old["type"] == "register_workspace"
                        && old["payload"]["workspace"]["workingDirectory"] == directory
                        && events.iter().any(|r| {
                            r["type"] == "workspace.command.receipted"
                                && r["payload"]["receipt"]["intentId"] == old["intentId"]
                        })
                });
                Ok((vec![], json!(replay)))
            })?;
            // Only exact canonical retries with a receipt bypass current eligibility.
            let canonical = if canonical_replay == true {
                directory.to_owned()
            } else {
                providers::validate_workspace_for_registration(directory)
                    .await
                    .map_err(Error)?
            };
            let display = p["displayName"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    PathBuf::from(&canonical)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                });
            let workspace = json!({"schemaVersion":1,"workspaceId":format!("workspace:{}",text(&base,"intentId")?.trim_start_matches("cmd:")),"displayName":display.trim(),"workingDirectory":canonical,"createdAt":base["createdAt"],"revision":1});
            let mut i = base;
            i["payload"] = json!({"workspace":workspace});
            i
        } else {
            intent(
                "remove_workspace",
                json!({"workspaceId":p["workspaceId"],"expectedWorkspaceRevision":p["expectedWorkspaceRevision"]}),
                p,
            )
        };
        request["actorId"] = json!("local-owner");
        self.apply_workspace_request(request)
    }

    fn apply_workspace_request(&self, request: Value) -> Result<Value> {
        domain::validate_workspace_command_intent(&request)?;
        self.store.transaction(|events| {
            let mut seen=false;
            if let Some(e)=events.iter().find(|e|e["type"]=="workspace.command.requested" && (e["payload"]["intent"]["intentId"]==request["intentId"] || e["payload"]["intent"]["idempotencyKey"]==request["idempotencyKey"])) {
                if e["payload"]["intent"]!=request {return Err(Error("IDEMPOTENCY_KEY_REUSED".into()));}
                if let Some(r)=events.iter().find(|r|r["type"]=="workspace.command.receipted" && r["payload"]["receipt"]["intentId"]==request["intentId"]) {return Ok((vec![],r["payload"]["receipt"].clone()));}
                seen=true;
            }
            let bundle=domain::project_events(events)?;
            if !seen && request["type"]=="remove_workspace" && array(&bundle["teams"],"items").iter().any(|t|t["workspaceId"]==request["payload"]["workspaceId"] && matches!(t["status"].as_str(),Some("active"|"waiting"))) {
                return Err(Error("WORKSPACE_IN_USE".into()));
            }
            let w=bundle["workspaces"].clone();
            let mut add=if seen {vec![]} else {vec![event("workspace.command.requested",json!({"intent":request}))]};
            let fact=events.iter().find(|e|matches!(e["type"].as_str(),Some("workspace.registered"|"workspace.removed")) && e["payload"]["intentId"]==request["intentId"]);
            let mut outcome="applied";let reason;let mut target;
            if request["type"]=="register_workspace" {
                let wanted=&request["payload"]["workspace"];target=wanted["workspaceId"].clone();
                if fact.is_some() {reason="WORKSPACE_REGISTERED";}
                else if request["expiresAt"].as_str().unwrap_or("")<=now().as_str() {outcome="expired";reason="COMMAND_EXPIRED";}
                else if let Some(old)=array(&w,"items").iter().find(|x|x["workingDirectory"]==wanted["workingDirectory"]) {outcome="rejected";reason="WORKSPACE_ALREADY_REGISTERED";target=old["workspaceId"].clone();}
                else {add.push(event("workspace.registered",json!({"intentId":request["intentId"],"workspace":wanted})));reason="WORKSPACE_REGISTERED";}
            } else {
                target=request["payload"]["workspaceId"].clone();let old=find(&w,"workspaceId",&target);
                if fact.is_some() {reason="WORKSPACE_REMOVED";}
                else if request["expiresAt"].as_str().unwrap_or("")<=now().as_str() {outcome="expired";reason="COMMAND_EXPIRED";}
                else if let Some(old)=old {
                    if old["revision"]!=request["payload"]["expectedWorkspaceRevision"] {outcome="rejected";reason="STALE_WORKSPACE_REVISION";}
                    else {add.push(event("workspace.removed",json!({"intentId":request["intentId"],"workspaceId":target,"expectedWorkspaceRevision":request["payload"]["expectedWorkspaceRevision"]})));reason="WORKSPACE_REMOVED";}
                } else {outcome="rejected";reason="WORKSPACE_NOT_FOUND";}
            }
            let mut combined=events.to_vec();combined.extend_from_slice(&add);
            let revision=domain::project_events(&combined)?["workspaces"]["revision"].as_u64().unwrap_or(0);
            let r=json!({"schemaVersion":1,"intentId":request["intentId"],"idempotencyKey":request["idempotencyKey"],"commandType":request["type"],"outcome":outcome,"reasonCode":reason,"completedAt":now(),"workspaceProjectionRevision":revision,"workspaceId":target});
            add.push(event("workspace.command.receipted",json!({"receipt":r})));Ok((add,r))
        })
    }

    fn install_pack(&self, pack: &Value) -> Result<Value> {
        domain::validate_pack(pack)?;
        if pack["provenance"]["kind"] != "local"
            || crate::teams::built_in_packs()
                .iter()
                .any(|p| p["id"] == pack["id"])
        {
            return Err(Error("INVALID_PACK_PROVENANCE".into()));
        }
        self.store.transaction(|events| {
            let current = array(&domain::project_events(events)?["packs"], "items")
                .iter()
                .find(|p| p["pack"]["id"] == pack["id"])
                .cloned();
            if let Some(old) = current {
                let version = |v: &Value| {
                    v.as_str()
                        .unwrap_or("")
                        .split('.')
                        .map(|n| n.parse::<u64>().unwrap_or(0))
                        .collect::<Vec<_>>()
                };
                if version(&pack["version"]) <= version(&old["pack"]["version"]) {
                    return Err(Error("PACK_VERSION_MUST_INCREASE".into()));
                }
            }
            Ok((
                vec![event("agent_pack.installed", json!({"pack":pack}))],
                json!({"outcome":"applied","reasonCode":"PACK_INSTALLED"}),
            ))
        })
    }
    fn remove_pack(&self, id: &Value) -> Result<Value> {
        if crate::teams::built_in_packs()
            .iter()
            .any(|p| p["id"] == *id)
        {
            return Err(Error("BUILT_IN_PACK".into()));
        }
        self.store.transaction(|events| {
            let p = domain::project_events(events)?["packs"].clone();
            let old = array(&p, "items")
                .iter()
                .find(|p| p["pack"]["id"] == *id)
                .ok_or_else(|| Error("PACK_NOT_INSTALLED".into()))?;
            Ok((
                vec![event(
                    "agent_pack.removed",
                    json!({"packId":id,"expectedRevision":old["revision"]}),
                )],
                json!({"outcome":"applied","reasonCode":"PACK_REMOVED"}),
            ))
        })
    }

    fn persist_observation(&self, observation: &Value) -> Result<()> {
        let provider = &observation["provider"];
        let provider_id = &provider["id"];
        let at = &observation["observedAt"];
        let old = self.store.runtime_projections()?["observation"]["observations"]
            .as_array()
            .and_then(|v| v.iter().find(|o| o["provider"]["id"] == *provider_id))
            .cloned()
            .unwrap_or(Value::Null);
        let mut add = Vec::new();
        for session in array(observation, "sessions") {
            let mut payload = session.clone();
            payload["providerId"] = provider_id.clone();
            add.push(event("session.observed", payload));
            let prior = array(&old, "sessions")
                .iter()
                .find(|s| s["providerSessionId"] == session["providerSessionId"]);
            if matches!(
                session["status"].as_str(),
                Some("completed" | "failed" | "interrupted")
            ) && prior
                .map(|s| s["status"] != session["status"])
                .unwrap_or(true)
            {
                let mut terminal = json!({"providerId":provider_id,"providerSessionId":session["providerSessionId"],"status":session["status"],"lastObservedAt":session["lastObservedAt"]});
                if !session["terminalAt"].is_null() {
                    terminal["terminalAt"] = session["terminalAt"].clone();
                }
                add.push(event("session.terminal", terminal));
            }
        }
        if !provider["error"].is_null() {
            add.push(event(
                "observation.failed",
                json!({"providerId":provider_id,"observedAt":at,"error":provider["error"]}),
            ));
        }
        let mut p = json!({"providerId":provider_id,"state":provider["state"],"version":provider["version"],"capabilities":provider["capabilities"],"observedAt":at,
            "sessionIds":array(observation,"sessions").iter().map(|s|s["providerSessionId"].clone()).collect::<Vec<_>>()});
        if !provider["error"].is_null() {
            p["error"] = provider["error"].clone();
        }
        add.push(event("provider.observed", p));
        self.store.append(add)
    }
    async fn observe(&self) -> Result<()> {
        for mut o in providers::observe_all(&self.data_dir)
            .await
            .map_err(Error)?
        {
            if o["provider"]["id"] == "gemini"
                && (o["provider"]["state"] == "available"
                    || (o["provider"]["state"] == "degraded"
                        && o["provider"]["error"]["code"] != "GEMINI_HOOK_SETUP_REQUIRED"))
            {
                let old = self.store.runtime_projections()?["observation"]["observations"]
                    .as_array()
                    .and_then(|items| items.iter().find(|item| item["provider"]["id"] == "gemini"))
                    .cloned()
                    .unwrap_or(Value::Null);
                o["sessions"] = old["sessions"]
                    .as_array()
                    .cloned()
                    .map(Value::Array)
                    .unwrap_or(json!([]));
            }
            self.persist_observation(&o)?;
        }
        let bundle = self.store.runtime_projections()?;
        let active = bundle["observation"]["observations"]
            .as_array()
            .and_then(|items| {
                items.iter().find(|o| {
                    o["provider"]["id"] == "gemini" && o["provider"]["state"] == "available"
                })
            })
            .cloned();
        if let Some(mut observation) = active {
            for entry in providers::read_gemini_inbox(&self.data_dir)
                .await
                .map_err(Error)?
            {
                let signal = &entry.signal;
                let mut sessions = array(&observation, "sessions").to_vec();
                sessions.retain(|s| s["providerSessionId"] != signal["providerSessionId"]);
                sessions.push(json!({"providerSessionId":signal["providerSessionId"],"status":signal["status"],"createdAt":null,"lastObservedAt":signal["observedAt"]}));
                sessions.sort_by(|a, b| {
                    b["lastObservedAt"]
                        .as_str()
                        .cmp(&a["lastObservedAt"].as_str())
                });
                sessions.truncate(20);
                observation["sessions"] = json!(sessions);
                self.persist_observation(&observation)?;
                providers::ack_gemini_inbox(&entry).await.map_err(Error)?;
            }
        }
        self.persist_signals().await
    }
    async fn persist_signals(&self) -> Result<()> {
        for signal in self.controller.drain_signals().await {
            if signal["providerId"] == "codex" && signal["status"] == "unknown" {
                let work = self.store.runtime_projections()?["work"].clone();
                let lost = array(&work, "runs")
                    .iter()
                    .filter(|r| {
                        r["providerSessionId"] == signal["providerSessionId"]
                            && matches!(r["status"].as_str(), Some("running" | "cancelling"))
                    })
                    .map(|r| {
                        event(
                            "run.session_lost",
                            json!({"runId":r["runId"],"ownerEpoch":r["ownerEpoch"]}),
                        )
                    })
                    .collect::<Vec<_>>();
                if !lost.is_empty() {
                    self.store.append(lost)?;
                }
            }
            let mut payload = json!({"providerId":signal["providerId"],"providerSessionId":signal["providerSessionId"],"status":signal["status"],"createdAt":null,"lastObservedAt":signal["observedAt"]});
            if matches!(
                signal["status"].as_str(),
                Some("completed" | "failed" | "interrupted")
            ) {
                payload["terminalAt"] = signal["observedAt"].clone();
            }
            self.store
                .append(vec![event("session.observed", payload)])?;
        }
        Ok(())
    }
    async fn advance_all(&self) -> Result<()> {
        let bundle = self.store.runtime_projections()?;
        for team in array(&bundle["teams"], "items")
            .iter()
            .filter(|t| matches!(t["status"].as_str(), Some("active" | "waiting")))
        {
            if let Err(error) = self.advance_team(&team["teamId"]).await {
                if error.0 != "WORKSPACE_NOT_REGISTERED" {
                    return Err(error);
                }
            }
        }
        Ok(())
    }
    pub async fn tick(&self) -> Result<()> {
        self.background_tick(true).await
    }

    pub async fn background_tick(&self, observe_providers: bool) -> Result<()> {
        let _guard = self.operations.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Ok(());
        }
        if observe_providers {
            self.observe().await?;
        } else {
            self.persist_signals().await?;
        }
        if self.closing.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.advance_all().await?;
        self.pump_extraction(true).await?;
        let _ = self.sync_cloud().await;
        let _ = self.sync_shared_memory(None, false).await;
        Ok(())
    }
    pub fn request_shutdown(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.cancel_login();
        self.cancel_extraction();
        if let Ok(server) = self.agent_memory_server.try_lock() {
            if let Some(server) = server.as_ref() {
                server.shutdown();
            }
        }
        if let Ok(progress) = self.agent_memory_progress.lock() {
            for state in progress.values() {
                if let Some(active) = state.active.upgrade() {
                    active.store(false, Ordering::SeqCst);
                }
            }
        }
        self.shutdown_notify.notify_waiters();
    }

    pub async fn shutdown(&self) {
        self.request_shutdown();
        if let Some(server) = self.agent_memory_server.lock().await.as_ref() {
            server.shutdown();
        }
        let _guard = self.operations.lock().await;
        self.shutdown_extraction().await;
        self.controller.shutdown().await;
        let _ = self.persist_signals().await;
    }

    pub(crate) async fn prepare_agent_memory(
        &self,
        work: &Value,
    ) -> Result<Option<agent_memory::Ticket>> {
        let work_id = work["workItemId"]
            .as_str()
            .ok_or_else(|| Error("AGENT_MEMORY_SCOPE_DENIED".into()))?;
        let Some(project) = self.store.subject_project("work", work_id)? else {
            return Ok(None);
        };
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("HOST_CLOSING".into()));
        }
        let run_id = work["runId"]
            .as_str()
            .ok_or_else(|| Error("AGENT_MEMORY_SCOPE_DENIED".into()))?;
        let manifest = self
            .store
            .read_decisions(&json!({"projectId":project,"status":"accepted","limit":1}))?;
        let revision = manifest["revision"]
            .as_u64()
            .ok_or_else(|| Error("AGENT_MEMORY_SCOPE_DENIED".into()))?;
        let context = json!({"runId":run_id,"workItemId":work_id,"ownerEpoch":self.epoch,"projectId":project,"decisionRevision":revision});
        self.store.transaction(|events| {
            let prior = events.iter().find(|e| {
                e["type"] == "agent_memory.context_bound" && e["payload"]["runId"] == run_id
            });
            if let Some(prior) = prior {
                if prior["payload"] != context {
                    return Err(Error("AGENT_MEMORY_CONTEXT_STALE".into()));
                }
                return Ok((vec![], Value::Null));
            }
            let mut bound = event("agent_memory.context_bound", context);
            bound["schemaVersion"] = json!(6);
            Ok((vec![bound], Value::Null))
        })?;
        let active = Arc::new(AtomicBool::new(true));
        let binding = agent_memory::Binding {
            project_id: project,
            work_item_id: work_id.to_owned(),
            run_id: run_id.to_owned(),
            owner_epoch: self.epoch.clone(),
            decision_revision: revision,
            active: active.clone(),
        };
        let mut server = self.agent_memory_server.lock().await;
        if server.is_none() {
            *server = Some(
                agent_memory::Server::start(
                    self.self_ref.clone(),
                    self.data_dir.clone(),
                    std::env::current_exe()?,
                )
                .await?,
            );
        }
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("HOST_CLOSING".into()));
        }
        let ticket = server.as_ref().unwrap().issue(binding)?;
        let mut progress = self
            .agent_memory_progress
            .lock()
            .map_err(|_| Error("AGENT_MEMORY_UNAVAILABLE".into()))?;
        progress.retain(|_, p| p.active.upgrade().is_some_and(|a| a.load(Ordering::SeqCst)));
        progress.insert(
            run_id.to_owned(),
            AgentMemoryProgress {
                revision,
                next_offset: 0,
                complete: false,
                active: Arc::downgrade(&active),
                issue: None,
            },
        );
        Ok(Some(ticket))
    }

    pub(crate) fn agent_memory_issue(&self, run_id: &str, code: &str) {
        if let Ok(mut progress) = self.agent_memory_progress.lock() {
            if let Some(p) = progress.get_mut(run_id) {
                p.issue = Some(
                    if code == "AGENT_MEMORY_UNSUPPORTED" {
                        code
                    } else {
                        "AGENT_MEMORY_UNAVAILABLE"
                    }
                    .to_owned(),
                );
            }
        }
    }

    fn checked_agent_scope(&self, binding: &agent_memory::Binding) -> Result<Value> {
        let denied = || Error("AGENT_MEMORY_SCOPE_DENIED".into());
        if self.closing.load(Ordering::SeqCst)
            || !binding.active.load(Ordering::SeqCst)
            || binding.owner_epoch != self.epoch
        {
            return Err(denied());
        }
        let scope = self
            .store
            .agent_memory_scope(&binding.work_item_id, &binding.run_id)
            .map_err(|_| denied())?;
        let context = &scope["context"];
        if context["runId"] != binding.run_id
            || context["workItemId"] != binding.work_item_id
            || context["ownerEpoch"] != self.epoch
            || context["projectId"] != binding.project_id
            || context["decisionRevision"] != binding.decision_revision
            || scope["binding"] != binding.project_id
            || scope["manifest"]["revision"].as_u64().is_none()
            || scope["work"].is_null()
        {
            return Err(denied());
        }
        let run = &scope["run"];
        if run.is_null() {
            if scope["work"]["status"] != "launching" || scope["terminalStart"] == true {
                return Err(denied());
            }
        } else if run["status"] != "running"
            || run["ownerEpoch"] != self.epoch
            || run["workItemId"] != binding.work_item_id
            || scope["work"]["status"] != "running"
        {
            return Err(denied());
        }
        Ok(scope)
    }

    pub async fn agent_memory_call(
        &self,
        binding: &agent_memory::Binding,
        tool: &str,
        arguments: Value,
    ) -> Result<Value> {
        let invalid = || Error("AGENT_MEMORY_INVALID_ARGUMENTS".into());
        let scope = self.checked_agent_scope(binding)?;
        let fields = arguments.as_object().ok_or_else(invalid)?;
        let allowed: &[&str] = match tool {
            "project_context" => &["offset", "limit", "revision"],
            "memory_search" => &["query", "limit"],
            "decision_read" => &["decisionId"],
            _ => return Err(Error("AGENT_MEMORY_UNKNOWN_TOOL".into())),
        };
        if fields.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err(invalid());
        }
        let current = scope["manifest"]["revision"].as_u64().unwrap();
        let response = if tool == "project_context" {
            let integer = |key: &str, default: u64, min: u64, max: u64| -> Result<u64> {
                arguments
                    .get(key)
                    .map(|value| {
                        value
                            .as_u64()
                            .filter(|n| (min..=max).contains(n))
                            .ok_or_else(invalid)
                    })
                    .unwrap_or(Ok(default))
            };
            let offset = integer("offset", 0, 0, 9_007_199_254_740_991)?;
            let limit = integer("limit", 20, 1, 20)?;
            if let Some(revision) = arguments.get("revision") {
                if revision.as_u64() != Some(current) {
                    return Err(Error("AGENT_MEMORY_CONTEXT_STALE".into()));
                }
            } else if offset != 0 {
                return Err(invalid());
            }
            {
                let progress = self
                    .agent_memory_progress
                    .lock()
                    .map_err(|_| Error("AGENT_MEMORY_UNAVAILABLE".into()))?;
                let p = progress
                    .get(&binding.run_id)
                    .ok_or_else(|| Error("AGENT_MEMORY_SCOPE_DENIED".into()))?;
                if offset != 0 && (p.revision != current || p.next_offset != offset || p.complete) {
                    return Err(Error("AGENT_MEMORY_CONTEXT_STALE".into()));
                }
            }
            let page = self.store.read_decisions(&json!({"projectId":binding.project_id,"status":"accepted","offset":offset,"limit":limit})).map_err(|_| invalid())?;
            if page["revision"].as_u64() != Some(current) {
                return Err(Error("AGENT_MEMORY_CONTEXT_STALE".into()));
            }
            let decisions:Vec<_> = page["decisions"].as_array().unwrap().iter().map(|d|json!({"decisionId":d["decisionId"],"revision":d["revision"],"title":d["title"]})).collect();
            let total = page["total"].as_u64().unwrap();
            if offset > total {
                return Err(invalid());
            }
            let next = offset + decisions.len() as u64;
            let verified = self.checked_agent_scope(binding)?;
            if verified["manifest"]["revision"].as_u64() != Some(current) {
                return Err(Error("AGENT_MEMORY_CONTEXT_STALE".into()));
            }
            let mut progress = self
                .agent_memory_progress
                .lock()
                .map_err(|_| Error("AGENT_MEMORY_UNAVAILABLE".into()))?;
            let p = progress
                .get_mut(&binding.run_id)
                .ok_or_else(|| Error("AGENT_MEMORY_SCOPE_DENIED".into()))?;
            p.revision = current;
            p.next_offset = next;
            p.complete = next == total;
            json!({"schemaVersion":1,"projectId":binding.project_id,"name":scope["name"],"workItemId":binding.work_item_id,"runId":binding.run_id,"decisionRevision":current,"startDecisionRevision":binding.decision_revision,"stale":current!=binding.decision_revision,"offset":offset,"limit":limit,"total":total,"nextOffset":if next<total {json!(next)}else{Value::Null},"decisions":decisions})
        } else {
            {
                let progress = self
                    .agent_memory_progress
                    .lock()
                    .map_err(|_| Error("AGENT_MEMORY_UNAVAILABLE".into()))?;
                if !progress
                    .get(&binding.run_id)
                    .is_some_and(|p| p.complete && p.revision == current)
                {
                    return Err(Error("AGENT_MEMORY_CONTEXT_REQUIRED".into()));
                }
            }
            if tool == "memory_search" {
                let mut input = arguments.clone();
                input["projectId"] = json!(binding.project_id);
                self.store.search_decisions(&input).map_err(|_| invalid())?
            } else {
                if fields.len() != 1 {
                    return Err(invalid());
                }
                let detail = self.store.read_decision(&json!({"projectId":binding.project_id,"decisionId":arguments["decisionId"]})).map_err(|_| Error("AGENT_MEMORY_DECISION_UNAVAILABLE".into()))?;
                let d = &detail["decision"];
                if d["status"] != "accepted" {
                    return Err(Error("AGENT_MEMORY_DECISION_UNAVAILABLE".into()));
                }
                let mut decision = serde_json::Map::new();
                for key in [
                    "decisionId",
                    "revision",
                    "status",
                    "title",
                    "context",
                    "decision",
                    "rationale",
                    "consequences",
                    "acceptedAt",
                    "supersedesDecisionId",
                ] {
                    decision.insert(key.into(), d[key].clone());
                }
                decision.insert(
                    "sources".into(),
                    json!(d["sources"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|s| json!({"sourceId":s["sourceId"],"revision":s["revision"]}))
                        .collect::<Vec<_>>()),
                );
                json!({"schemaVersion":1,"projectId":binding.project_id,"decisionRevision":detail["revision"],"decision":decision})
            }
        };
        let after = self.checked_agent_scope(binding)?;
        if after["manifest"]["revision"].as_u64() != Some(current) {
            return Err(Error("AGENT_MEMORY_CONTEXT_STALE".into()));
        }
        Ok(response)
    }

    async fn memory_connection(&self) -> Result<Value> {
        let cloud = self.cloud.lock().await;
        Ok(cloud
            .as_ref()
            .map(Cloud::memory_connection)
            .unwrap_or(Value::Null))
    }
    async fn search_shared_memory(&self, payload: &Value) -> Result<Value> {
        let input = retrieval::request(payload)?;
        let project = input["projectId"].as_str().unwrap();
        let bound = self.memory_connection().await?;
        if bound.is_null() {
            return Err(Error("CLOUD_UNPAIRED".into()));
        }
        let scope = self.store.shared_search_scope(project, &bound)?;
        let shared = scope["mapping"]["sharedProjectId"].as_str().unwrap();
        let body = json!({"schemaVersion":1,"query":input["query"],"limit":input["limit"]});
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut response = self
            .memory_http(
                &bound,
                reqwest::Method::POST,
                &["projects", shared, "search"],
                &[],
                Some(&body),
                deadline,
            )
            .await?;
        // Linearize the final binding check with pairing and project mutations.
        let _guard = self.operations.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("HOST_CLOSING".into()));
        }
        if self.memory_connection().await? != bound {
            return Err(Error("MEMORY_CONNECTION_CHANGED".into()));
        }
        if self.store.shared_search_scope(project, &bound)? != scope {
            return Err(Error("MEMORY_CONFLICT".into()));
        }
        retrieval::validate_search_response(
            &response,
            &scope["mapping"]["sharedProjectId"],
            input["limit"].as_u64().unwrap(),
        )?;
        if response["manifestRevision"].as_u64() < scope["mapping"]["sharedRevision"].as_u64() {
            return Err(Error("MEMORY_PROTOCOL_INVALID".into()));
        }
        response["reasonCode"] = json!("SHARED_MEMORY_SEARCH_READ");
        response["projectId"] = input["projectId"].clone();
        response["revision"] = scope["revision"].clone();
        response["query"] = input["query"].clone();
        Ok(response)
    }
    async fn memory_http(
        &self,
        bound: &Value,
        method: reqwest::Method,
        segments: &[&str],
        query: &[(&str, String)],
        body: Option<&Value>,
        deadline: tokio::time::Instant,
    ) -> Result<Value> {
        let stopped = self.shutdown_notify.notified();
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("HOST_CLOSING".into()));
        }
        tokio::select! {
            _ = stopped => Err(Error("HOST_CLOSING".into())),
            result = tokio::time::timeout_at(deadline, async {
                let cloud = self.cloud.lock().await;
                let cloud = cloud.as_ref().ok_or_else(||Error("CLOUD_RECOVERY_REQUIRED".into()))?;
                cloud.memory_request(bound, method, segments, query, body).await.map_err(Error)
            }) => result.unwrap_or_else(|_| Err(Error("MEMORY_UNAVAILABLE".into())))
        }
    }
    fn shared_failure(&self, project: &str, bound: &Value, code: &str) -> Result<()> {
        let code = if (2..=64).contains(&code.len())
            && code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        {
            code
        } else {
            "MEMORY_LOCAL_STATE_FAILED"
        };
        let Some(mapping) = self.store.shared_mapping(project)? else {
            return Ok(());
        };
        if mapping["connection"] == *bound
            && (mapping["lastErrorCode"] != code || mapping["lastErrorTerminal"] == true)
        {
            self.store
                .append(vec![shared_memory::failure(project, bound, code)])?;
        }
        Ok(())
    }
    async fn sync_shared_memory(&self, selected: Option<&str>, force: bool) -> Result<Value> {
        if self.closing.load(Ordering::SeqCst) {
            return Ok(json!({"kind":"closing"}));
        }
        let bound = self.memory_connection().await?;
        if bound.is_null() {
            return Ok(json!({"kind":"unpaired"}));
        }
        let metadata = self.store.shared_metadata(&bound)?;
        let mut download = self.memory_download.lock().await;
        let project = if let Some(project) = selected {
            Some(project.to_string())
        } else if let Some(download) = download.as_ref() {
            Some(download.project.clone())
        } else {
            let polls = self
                .memory_poll
                .lock()
                .map_err(|_| Error("HOST_STATE_UNAVAILABLE".into()))?;
            metadata["projects"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|p| !p["sharedProjectId"].is_null())
                .filter(|p| p["state"] != "connection_changed")
                .filter(|p| {
                    polls
                        .get(p["projectId"].as_str().unwrap())
                        .is_none_or(|at| at.elapsed().as_secs() >= 30)
                        || p["pendingOperation"]["state"] == "queued"
                })
                .min_by_key(|p| {
                    (
                        p["pendingOperation"].is_null(),
                        polls.get(p["projectId"].as_str().unwrap()).copied(),
                    )
                })
                .and_then(|p| p["projectId"].as_str())
                .map(str::to_owned)
        };
        let Some(project) = project else {
            return Ok(json!({"kind":"idle"}));
        };
        let Some(mut mapping) = self.store.shared_mapping(&project)? else {
            return Ok(json!({"kind":"unlinked"}));
        };
        if mapping["connection"] != bound {
            self.shared_failure(
                &project,
                &mapping["connection"],
                "MEMORY_CONNECTION_CHANGED",
            )?;
            *download = None;
            return Err(Error("MEMORY_CONNECTION_CHANGED".into()));
        }
        {
            let mut polls = self
                .memory_poll
                .lock()
                .map_err(|_| Error("HOST_STATE_UNAVAILABLE".into()))?;
            if !force
                && download.is_none()
                && polls
                    .get(&project)
                    .is_some_and(|at| at.elapsed().as_secs() < 30)
                && metadata["projects"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|p| p["projectId"] == project)
                    .is_none_or(|p| p["pendingOperation"]["state"] != "queued")
            {
                return Ok(json!({"kind":"throttled"}));
            }
            polls.insert(project.clone(), std::time::Instant::now());
        }
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut requests = 0;
        let result = async {
            let pending = metadata["projects"].as_array().into_iter().flatten().find(|p|p["projectId"]==project).is_some_and(|p|!p["pendingOperation"].is_null());
            if pending {
                let operation = self.store.transaction(|events| shared_memory::prepare_delivery(events,&project,&bound))?;
                if !operation.is_null() {
                    requests += 1;
                    let receipt = self.memory_http(&bound,reqwest::Method::POST,&["operations"],&[],Some(&operation),deadline).await?;
                    self.store.append(vec![shared_memory::acknowledge(&project,&operation,&receipt)?])?;
                }
                mapping = self.store.shared_mapping(&project)?.unwrap();
            }
            if mapping["role"].is_null() || mapping["enabled"] != true {
                *download = None;
                return Ok(json!({"kind":"queued","projectId":project}));
            }
            if download.as_ref().is_none_or(|d|d.project!=project||d.connection!=bound||d.shared!=mapping["sharedProjectId"]) {
                *download = Some(shared_memory::Download::new(&project,&bound,&mapping["sharedProjectId"]));
            }
            let staged = download.as_mut().unwrap();
            loop {
                if self.closing.load(Ordering::SeqCst) { return Err(Error("HOST_CLOSING".into())); }
                if staged.complete() {
                    self.store.commit_shared_cache(&project,&bound,&staged.manifest(),&staged.records)?;
                    if mapping["sharedRevision"] != staged.manifest()["revision"] || (!mapping["lastErrorCode"].is_null() && mapping["lastErrorTerminal"] != true) {
                        self.store.append(vec![shared_memory::observed(&project,&bound,&staged.shared,&staged.manifest()["revision"])])?;
                    }
                    *download = None;
                    return Ok(json!({"kind":"synced","projectId":project}));
                }
                if requests>=4 || tokio::time::Instant::now()>=deadline { return Ok(json!({"kind":"partial","projectId":project})); }
                if staged.needs_page() {
                    let shared=staged.shared.as_str().unwrap();
                    let mut query=vec![("offset",staged.heads.len().to_string()),("limit","20".into())];
                    if let Some(revision)=staged.revision { query.push(("expectedRevision",revision.to_string())); }
                    requests+=1;
                    let page=self.memory_http(&bound,reqwest::Method::GET,&["projects",shared,"manifest"],&query,None,deadline).await?;
                    staged.page(page)?;
                    // A validated page establishes the server revision even
                    // while bodies are staged. Keep the old complete cache
                    // visibly stale until the whole new manifest is ready.
                    let revision = json!(staged.revision.unwrap());
                    if mapping["sharedRevision"] != revision {
                        self.store.append(vec![shared_memory::observed(
                            &project, &bound, &staged.shared, &revision,
                        )])?;
                        mapping = self.store.shared_mapping(&project)?.unwrap();
                    }
                } else {
                    let head=staged.head().unwrap().clone();
                    let cached=self.store.read_shared_decision(&json!({"projectId":project,"decisionId":head["decisionId"],"revision":head["revision"]}),&bound).ok();
                    if let Some(mut record)=cached.filter(|r|r["decision"]["title"]==head["title"]&&r["publishedAt"]==head["publishedAt"]) {
                        for key in ["reasonCode","projectId","revision","verifiedAt","stale","offline"] { record.as_object_mut().unwrap().remove(key); }
                        record["manifestRevision"]=json!(staged.revision.unwrap());record["current"]=json!(true);staged.record(record)?;
                    } else {
                        requests+=1;
                        let record=self.memory_http(&bound,reqwest::Method::GET,&["projects",staged.shared.as_str().unwrap(),"decisions",head["decisionId"].as_str().unwrap()],&[("revision",head["revision"].to_string())],None,deadline).await?;
                        staged.record(record)?;
                    }
                }
            }
        }.await;
        if let Err(error) = &result {
            *download = None;
            if error.0 != "HOST_CLOSING" {
                self.shared_failure(&project, &bound, &error.0)?;
            }
        }
        result
    }

    async fn sync_cloud(&self) -> Result<Value> {
        {
            let state = self.cloud.lock().await;
            let cloud = state
                .as_ref()
                .ok_or_else(|| Error("CLOUD_RECOVERY_REQUIRED".into()))?;
            if cloud.public_status()["paired"] == false {
                return Ok(json!({"kind":"unpaired"}));
            }
        }
        let mut bundle = self.store.runtime_projections()?;
        let (sequence, recorded_at) = self.store.event_cursor()?;
        bundle["agentPacks"] = json!(catalog(&bundle));
        bundle["sequence"] = json!(sequence);
        bundle["recordedAt"] =
            json!(recorded_at.unwrap_or_else(|| "1970-01-01T00:00:00.000Z".into()));
        let batch = {
            let mut state = self.cloud.lock().await;
            let c = state
                .as_mut()
                .ok_or_else(|| Error("CLOUD_RECOVERY_REQUIRED".into()))?;
            let result = c.begin_sync(&bundle).await.map_err(Error);
            self.cache_cloud(c)?;
            result?
        };
        for entry in batch.intents {
            let r = if entry["intent"]["schemaVersion"] == 1 {
                self.apply_work_intent(entry["intent"].clone()).await?
            } else {
                self.apply_cloud_team(&entry).await?
            };
            let mut state = self.cloud.lock().await;
            let c = state
                .as_mut()
                .ok_or_else(|| Error("CLOUD_RECOVERY_REQUIRED".into()))?;
            let result = c
                .deliver_receipt(
                    entry["sequence"]
                        .as_u64()
                        .ok_or_else(|| Error("INVALID_CLOUD_SEQUENCE".into()))?,
                    &r,
                )
                .await
                .map_err(Error);
            self.cache_cloud(c)?;
            result?;
        }
        let mut state = self.cloud.lock().await;
        let c = state
            .as_mut()
            .ok_or_else(|| Error("CLOUD_RECOVERY_REQUIRED".into()))?;
        let r = c.finish_sync().map_err(Error);
        self.cache_cloud(c)?;
        r
    }

    async fn apply_cloud_team(&self, entry: &Value) -> Result<Value> {
        let request = &entry["intent"];
        let previous = self.store.transaction(|events| {
            let existing = events.iter().find(|e| {
                e["type"] == "cloud_team.command.requested"
                    && (e["payload"]["intent"]["intentId"] == request["intentId"]
                        || e["payload"]["intent"]["idempotencyKey"] == request["idempotencyKey"])
            });
            if let Some(old) = existing {
                if old["payload"]["intent"] != *request {
                    return Err(Error("IDEMPOTENCY_KEY_REUSED".into()));
                }
                let receipt = events.iter().find(|e| {
                    e["type"] == "cloud_team.command.receipted"
                        && e["payload"]["receipt"]["intentId"] == request["intentId"]
                });
                return Ok((
                    vec![],
                    receipt
                        .map(|e| e["payload"]["receipt"].clone())
                        .unwrap_or(Value::Null),
                ));
            }
            Ok((
                vec![second_event("cloud_team.command.requested", entry.clone())],
                Value::Null,
            ))
        })?;
        if !previous.is_null() {
            return Ok(previous);
        }
        let kind = text(request, "type")?;
        let p = &request["payload"];
        let mut team = find(
            &self.store.runtime_projections()?["teams"],
            "teamId",
            &p["teamId"],
        );
        let mut outcome = "applied";
        let mut reason = match kind {
            "start_team" => "TEAM_STARTED",
            "cancel_team" => "TEAM_CANCELLED",
            "cancel_agent" => "AGENT_CANCELLED",
            "approve_decision" => "DECISION_APPROVED",
            "reject_decision" => "DECISION_REJECTED",
            _ => "QUESTION_ANSWERED",
        };
        if kind == "start_team" {
            team = find(
                &self.store.runtime_projections()?["teams"],
                "teamId",
                &json!(format!("team:{}", text(request, "intentId")?)),
            );
        }
        let already = team
            .as_ref()
            .map(|t| match kind {
                "start_team" => t["status"] != "draft",
                "cancel_team" => t["status"] == "cancelled",
                "cancel_agent" => array(t, "agents")
                    .iter()
                    .any(|a| a["agentId"] == p["agentId"] && a["status"] == "cancelled"),
                "approve_decision" | "reject_decision" => array(t, "tasks").iter().any(|task| {
                    task["taskId"] == p["taskId"]
                        && task["approval"]["decision"]
                            == if kind == "approve_decision" {
                                "approved"
                            } else {
                                "rejected"
                            }
                }),
                "answer_question" => array(t, "tasks").iter().any(|task| {
                    task["question"]["questionId"] == p["questionId"]
                        && task["answer"]["answer"] == p["answer"]
                }),
                _ => false,
            })
            .unwrap_or(false);
        if !already {
            if request["expiresAt"].as_str().unwrap_or("") <= now().as_str() {
                outcome = "expired";
                reason = "COMMAND_EXPIRED";
            } else if kind != "start_team" && team.is_none() {
                outcome = "rejected";
                reason = "TEAM_NOT_FOUND";
            } else if kind != "start_team"
                && team.as_ref().unwrap()["revision"] != p["expectedTeamRevision"]
            {
                outcome = "rejected";
                reason = "STALE_TEAM_REVISION";
            } else {
                let result = if kind == "start_team" {
                    let installation = {
                        self.cloud
                            .lock()
                            .await
                            .as_ref()
                            .ok_or_else(|| Error("CLOUD_RECOVERY_REQUIRED".into()))?
                            .installation_id()
                            .to_owned()
                    };
                    let workspaces = self.store.runtime_projections()?["workspaces"].clone();
                    let workspace = array(&workspaces, "items").iter().find(|w| {
                        crate::cloud::workspace_alias(
                            &installation,
                            w["workspaceId"].as_str().unwrap_or(""),
                        ) == p["workspaceAlias"]
                    });
                    if let Some(workspace) = workspace {
                        let mut input = json!({"name":p["teamName"],"goal":p["goal"],"workspaceId":workspace["workspaceId"],"templateId":p["templateId"],"orchestratorPackId":p["orchestratorPackId"],"workerPackIds":p["workerPackIds"],"settings":p["settings"],"teamId":format!("team:{}",text(request,"intentId")?)});
                        if team.is_none() {
                            self.create_team(&input)?;
                        }
                        input["teamId"] = json!(format!("team:{}", text(request, "intentId")?));
                        self.execute("team_start", &input).await
                    } else {
                        Err(Error("WORKSPACE_ALIAS_NOT_FOUND".into()))
                    }
                } else {
                    let action = match kind {
                        "cancel_team" => "team_cancel",
                        "cancel_agent" => "team_cancel_agent",
                        "answer_question" => "team_answer",
                        _ => "team_approve",
                    };
                    let mut input = p.clone();
                    if action == "team_approve" {
                        input["decision"] = json!(if kind == "approve_decision" {
                            "approved"
                        } else {
                            "rejected"
                        });
                    }
                    self.execute(action, &input).await
                };
                if result.is_err() {
                    outcome = "rejected";
                    reason = if kind == "start_team" {
                        "TEAM_CONFIGURATION_INVALID"
                    } else {
                        "TEAM_ACTION_UNAVAILABLE"
                    };
                }
            }
        }
        let id = if kind == "start_team" {
            json!(format!("team:{}", text(request, "intentId")?))
        } else {
            p["teamId"].clone()
        };
        team = find(&self.store.runtime_projections()?["teams"], "teamId", &id);
        let r = json!({"schemaVersion":2,"intentId":request["intentId"],"idempotencyKey":request["idempotencyKey"],"commandType":kind,"outcome":outcome,"reasonCode":reason,"completedAt":now(),"teamId":team.as_ref().map(|t|t["teamId"].clone()).unwrap_or_else(||if kind=="start_team" {Value::Null} else {p["teamId"].clone()}),"teamRevision":team.as_ref().map(|t|t["revision"].clone()).unwrap_or(json!(0))});
        self.store.append(vec![second_event(
            "cloud_team.command.receipted",
            json!({"sequence":entry["sequence"],"receipt":r}),
        )])?;
        Ok(r)
    }
}
