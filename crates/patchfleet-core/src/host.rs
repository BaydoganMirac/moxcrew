use crate::{
    cloud::Cloud,
    domain, event, now, project_memory, providers,
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
        Arc, Mutex,
    },
};
use tokio::sync::Mutex as AsyncMutex;

pub struct Host {
    pub(crate) store: Store,
    pub(crate) epoch: String,
    pub(crate) controller: providers::Controller,
    data_dir: PathBuf,
    operations: AsyncMutex<()>,
    cloud: AsyncMutex<Option<Cloud>>,
    cloud_status: Mutex<Value>,
    desktop_preferences: Mutex<Value>,
    pub(crate) closing: AtomicBool,
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
fn exact_action(action: &str, p: &Value) -> Result<()> {
    let allowed: &[&str] = match action {
        "observe" | "cloud_unpair" | "cloud_sync" => &[],
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
        if matches!(action, "project_message" | "project_source_edit") {
            192 * 1024
        } else {
            128 * 1024
        }
    }
    pub fn open(data_dir: PathBuf) -> Result<Arc<Self>> {
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
        let host = Arc::new(Self {
            store,
            epoch: uuid::Uuid::new_v4().to_string(),
            controller: providers::Controller::new(),
            desktop_preferences: Mutex::new(crate::desktop_preferences::load(&data_dir)),
            data_dir,
            operations: AsyncMutex::new(()),
            cloud: AsyncMutex::new(cloud),
            cloud_status: Mutex::new(status),
            closing: AtomicBool::new(false),
        });
        host.store
            .transaction(|events| Ok((project_memory::recover(events)?, Value::Null)))?;
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
        bundle["appVersion"] = json!(env!("CARGO_PKG_VERSION"));
        bundle["hostEpoch"] = json!(self.epoch);
        bundle["desktopPreferences"] = self
            .desktop_preferences
            .lock()
            .map_err(|_| Error("HOST_STATE_UNAVAILABLE".into()))?
            .clone();
        Ok(bundle)
    }

    pub async fn action(&self, action: &str, mut payload: Value) -> Result<Value> {
        exact_action(action, &payload)?;
        let _guard = self.operations.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Err(Error("HOST_CLOSING".into()));
        }
        if action == "project_memory_read" {
            return self.store.read_memory(&payload);
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
            "cloud_pair"=>{let mut state=self.cloud.lock().await;let c=state.as_mut().ok_or_else(||Error("CLOUD_RECOVERY_REQUIRED".into()))?;let result=c.pair(&json!({"cloudUrl":p["cloudUrl"],"displayName":p["displayName"],"pairingCode":p["pairingCode"]})).await.map_err(Error);self.cache_cloud(c)?;result},
            "cloud_unpair"=>{let mut state=self.cloud.lock().await;let c=state.as_mut().ok_or_else(||Error("CLOUD_RECOVERY_REQUIRED".into()))?;let result=c.unpair().map_err(Error);self.cache_cloud(c)?;result},
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

    async fn workspace_action(&self, action: &str, p: &Value) -> Result<Value> {
        let mut request = if action == "workspace_register" {
            let canonical = providers::validate_workspace(text(p, "workingDirectory")?)
                .await
                .map_err(Error)?;
            let base = intent("register_workspace", Value::Null, p);
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
        let _ = self.sync_cloud().await;
        Ok(())
    }
    pub fn request_shutdown(&self) {
        self.closing.store(true, Ordering::SeqCst);
    }

    pub async fn shutdown(&self) {
        self.request_shutdown();
        let _guard = self.operations.lock().await;
        self.controller.shutdown().await;
        let _ = self.persist_signals().await;
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
