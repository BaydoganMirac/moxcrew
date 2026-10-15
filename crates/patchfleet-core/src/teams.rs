use crate::{
    domain, event, now,
    work::{array, find, intent, text},
    Error, Host, Result,
};
use serde_json::{json, Value};
use std::collections::HashSet;

pub(crate) fn built_in_packs() -> Vec<Value> {
    serde_json::from_str(include_str!("../assets/packs.json")).expect("compiled pack catalog")
}
pub(crate) fn templates() -> Vec<Value> {
    serde_json::from_str(include_str!("../assets/templates.json"))
        .expect("compiled template catalog")
}
pub(crate) fn catalog(bundle: &Value) -> Vec<Value> {
    let mut packs = built_in_packs();
    packs.extend(
        array(&bundle["packs"], "items")
            .iter()
            .map(|p| p["pack"].clone()),
    );
    packs.sort_by(|a, b| {
        a["name"]
            .as_str()
            .cmp(&b["name"].as_str())
            .then(a["id"].as_str().cmp(&b["id"].as_str()))
    });
    packs
}
fn work_id(team: &Value, task: &Value) -> Result<String> {
    let t = text(team, "teamId")?
        .strip_prefix("team:")
        .ok_or_else(|| Error("INVALID_TEAM_ID".into()))?;
    let k = text(task, "taskId")?
        .strip_prefix("task:")
        .ok_or_else(|| Error("INVALID_TASK_ID".into()))?;
    Ok(format!(
        "work:{t}:{k}:{}",
        task["attempt"].as_u64().unwrap_or(1)
    ))
}
fn string_input(v: &Value, key: &str, max: usize) -> Result<String> {
    let value = text(v, key)?.trim();
    if value.is_empty() || value.encode_utf16().count() > max {
        return Err(Error(format!("INVALID_{key}")));
    }
    Ok(value.into())
}

impl Host {
    pub(crate) fn create_team(&self, input: &Value) -> Result<Value> {
        let bundle = self.store.runtime_projections()?;
        if find(&bundle["workspaces"], "workspaceId", &input["workspaceId"]).is_none() {
            return Err(Error("WORKSPACE_NOT_REGISTERED".into()));
        }
        let templates = templates();
        let template = templates
            .iter()
            .find(|t| t["id"] == input["templateId"])
            .ok_or_else(|| Error("UNKNOWN_TEMPLATE".into()))?;
        let worker_ids = input["workerPackIds"]
            .as_array()
            .ok_or_else(|| Error("INVALID_WORKER_PACKS".into()))?;
        let orchestrator = input["orchestratorPackId"]
            .as_str()
            .unwrap_or("pack:orchestrator");
        if worker_ids.is_empty()
            || worker_ids.len() > 11
            || worker_ids.iter().collect::<HashSet<_>>().len() != worker_ids.len()
            || worker_ids.iter().any(|id| id == orchestrator)
        {
            return Err(Error("INVALID_WORKER_PACKS".into()));
        }
        let packs = catalog(&bundle);
        let selected = std::iter::once(json!(orchestrator))
            .chain(worker_ids.iter().cloned())
            .map(|id| {
                packs
                    .iter()
                    .find(|p| p["id"] == id)
                    .cloned()
                    .ok_or_else(|| Error("PACK_NOT_INSTALLED".into()))
            })
            .collect::<Result<Vec<_>>>()?;
        if selected[0]["role"] != "orchestrator" {
            return Err(Error("INVALID_ORCHESTRATOR".into()));
        }
        let agents: Vec<_> = selected
            .into_iter()
            .map(|p| json!({"agentId":format!("agent:{}",uuid::Uuid::new_v4()),"pack":p}))
            .collect();
        let mut workers = agents[1..].to_vec();
        workers.sort_by_key(|a| {
            template["roles"][a["pack"]["role"].as_str().unwrap_or("")]["phase"]
                .as_u64()
                .unwrap_or(u64::MAX)
        });
        let seeds: Vec<_> = workers
            .iter()
            .map(|a| {
                let role = &template["roles"][a["pack"]["role"].as_str().unwrap_or("")];
                (a, role, format!("task:{}", uuid::Uuid::new_v4()))
            })
            .collect();
        let tasks:Vec<_>=seeds.iter().map(|(a,role,id)|{
            let phase=role["phase"].as_u64().unwrap_or(u64::MAX);
            let previous=seeds.iter().filter_map(|(_,r,_)|r["phase"].as_u64()).filter(|p|*p<phase).max();
            let deps:Vec<_>=seeds.iter().filter(|(_,r,_)|previous.is_some() && r["phase"].as_u64()==previous).map(|(_,_,id)|id.clone()).collect();
            json!({"taskId":id,"title":format!("{}: {}",a["pack"]["name"].as_str().unwrap_or("Agent"),input["templateId"].as_str().unwrap_or("").replace('-'," ")),"instruction":role["instruction"],"agentId":a["agentId"],
                "dependsOn":deps,"approvalRequired":role["approvalRequired"],"question":if role["asksReleaseQuestion"]==true {
                    json!({"questionId":format!("question:{}",uuid::Uuid::new_v4()),"prompt":"Confirm the release target and any constraints the release agent must follow.","maxAnswerLength":1000})
                } else {Value::Null}})
        }).collect();
        let settings = &input["settings"];
        let team_id = input["teamId"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| {
                input["commandId"]
                    .as_str()
                    .and_then(|id| id.strip_prefix("cmd:"))
                    .map(|id| format!("team:{id}"))
                    .unwrap_or_else(|| format!("team:{}", uuid::Uuid::new_v4()))
            });
        let team = json!({"schemaVersion":1,"teamId":team_id,"name":string_input(input,"name",80)?,
            "goal":string_input(input,"goal",10_000)?,"workspaceId":input["workspaceId"],"templateId":input["templateId"],
            "orchestratorAgentId":agents[0]["agentId"],"agents":agents,"tasks":tasks,
            "settings":{"concurrency":settings["concurrency"].as_u64().unwrap_or(2),"retryLimit":settings["retryLimit"].as_u64().unwrap_or(1),
                "timeoutMinutes":settings["timeoutMinutes"].as_u64().unwrap_or(120),"failurePolicy":settings["failurePolicy"].as_str().unwrap_or("stop")},"createdAt":now()});
        domain::validate_team(&team)?;
        let mut additions = Vec::new();
        // Only the local owner's creation path assigns source memory scope.
        if input["commandId"].is_string() {
            if let Some(project) = self.store.workspace_project(text(input, "workspaceId")?)? {
                additions.push(crate::project_memory::bind(
                    &project,
                    "team",
                    text(&team, "teamId")?,
                ));
            }
        }
        additions.push(event("team.created", json!({"team":team})));
        self.store.append(additions)?;
        Ok(json!({"reasonCode":"TEAM_CREATED","teamId":team["teamId"]}))
    }

    pub(crate) fn team_event(&self, kind: &str, payload: Value) -> Result<Value> {
        self.store.transaction(|events| {
            let team = find(
                &domain::project_events(events)?["teams"],
                "teamId",
                &payload["teamId"],
            )
            .ok_or_else(|| Error("TEAM_NOT_FOUND".into()))?;
            let active = matches!(team["status"].as_str(), Some("active" | "waiting"));
            let valid = match kind {
                "team.started" => team["status"] == "draft",
                "team.cancelled" | "team.timed_out" => active,
                "team.agent.cancelled" => {
                    active
                        && payload["agentId"] != team["orchestratorAgentId"]
                        && array(&team, "agents").iter().any(|a| {
                            a["agentId"] == payload["agentId"] && a["status"] != "cancelled"
                        })
                }
                "team.approval.decided" => {
                    active
                        && array(&team, "tasks").iter().any(|t| {
                            t["taskId"] == payload["taskId"] && t["status"] == "waiting_approval"
                        })
                }
                "team.question.answered" => {
                    active
                        && array(&team, "tasks").iter().any(|t| {
                            t["question"]["questionId"] == payload["questionId"]
                                && t["status"] == "waiting_question"
                                && payload["answer"]
                                    .as_str()
                                    .map(|s| {
                                        s.encode_utf16().count() as u64
                                            <= t["question"]["maxAnswerLength"]
                                                .as_u64()
                                                .unwrap_or(0)
                                    })
                                    .unwrap_or(false)
                        })
                }
                "team.task.retry_requested" => {
                    active
                        && array(&team, "tasks").iter().any(|t| {
                            t["taskId"] == payload["taskId"]
                                && t["canRetry"] == true
                                && t["attempt"] == payload["previousAttempt"]
                        })
                }
                _ => false,
            };
            if !valid {
                return Err(Error("TEAM_INVALID_ACTION".into()));
            }
            Ok((
                vec![event(kind, payload)],
                json!({"outcome":"applied","reasonCode":"TEAM_UPDATED"}),
            ))
        })
    }

    pub(crate) async fn stop_team_work(&self, team: &Value, agent: Option<&Value>) -> Result<()> {
        for task in array(team, "tasks")
            .iter()
            .filter(|t| agent.map(|a| t["agentId"] == *a).unwrap_or(true))
        {
            let work = self.store.runtime_projections()?["work"].clone();
            if let Some(item) = find(&work, "workItemId", &task["workItemId"]) {
                if item["status"] == "queued" {
                    self.apply_work_intent(intent("remove_queued_work",json!({"workItemId":item["workItemId"],"expectedItemRevision":item["revision"]}),&Value::Null)).await?;
                } else if let Some(run) = array(&work, "runs").iter().find(|r| {
                    r["runId"] == task["runId"]
                        && matches!(r["status"].as_str(), Some("running" | "cancelling"))
                }) {
                    self.apply_work_intent(intent(
                        "cancel_run",
                        json!({"runId":run["runId"],"expectedRunRevision":run["revision"]}),
                        &Value::Null,
                    ))
                    .await?;
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn advance_team(&self, id: &Value) -> Result<Value> {
        let mut bundle = self.store.runtime_projections()?;
        let mut team =
            find(&bundle["teams"], "teamId", id).ok_or_else(|| Error("TEAM_NOT_FOUND".into()))?;
        if !matches!(team["status"].as_str(), Some("active" | "waiting")) {
            return Ok(team);
        }
        let started = chrono::DateTime::parse_from_rfc3339(text(&team, "startedAt")?)
            .map_err(|_| Error("INVALID_TIMESTAMP".into()))?;
        if chrono::Utc::now()
            .signed_duration_since(started)
            .num_minutes()
            >= team["settings"]["timeoutMinutes"].as_i64().unwrap_or(120)
        {
            self.team_event("team.timed_out", json!({"teamId":id}))?;
            self.stop_team_work(&team, None).await?;
            return Ok(find(&self.store.runtime_projections()?["teams"], "teamId", id).unwrap());
        }
        for task in array(&team, "tasks")
            .iter()
            .filter(|t| t["canRetry"] == true)
        {
            self.team_event(
                "team.task.retry_requested",
                json!({"teamId":id,"taskId":task["taskId"],"previousAttempt":task["attempt"]}),
            )?;
        }
        bundle = self.store.runtime_projections()?;
        team = find(&bundle["teams"], "teamId", id).unwrap();
        let workspace = find(&bundle["workspaces"], "workspaceId", &team["workspaceId"])
            .ok_or_else(|| Error("WORKSPACE_NOT_REGISTERED".into()))?;
        let project = self.store.subject_project("team", text(&team, "teamId")?)?;
        let active = array(&team, "tasks")
            .iter()
            .filter(|t| {
                matches!(
                    t["status"].as_str(),
                    Some("running" | "launching" | "cancelling")
                )
            })
            .count();
        let mut slots = team["settings"]["concurrency"]
            .as_u64()
            .unwrap_or(1)
            .saturating_sub(active as u64);
        for task in array(&team, "tasks")
            .iter()
            .filter(|t| t["status"] == "queued" || t["status"] == "ready")
        {
            if slots == 0 {
                break;
            }
            let item_id = json!(work_id(&team, task)?);
            let mut item = find(
                &self.store.runtime_projections()?["work"],
                "workItemId",
                &item_id,
            );
            if item.is_none() {
                let pack = &array(&team, "agents")
                    .iter()
                    .find(|a| a["agentId"] == task["agentId"])
                    .ok_or_else(|| Error("AGENT_NOT_FOUND".into()))?["pack"];
                let title = format!("{}: {}", text(&team, "name")?, text(task, "title")?)
                    .chars()
                    .take(160)
                    .collect::<String>();
                let instruction=format!("Patchfleet team: {}\n\nTeam goal: {}\n\nAssigned task: {}\n\n{}\n\nAgent contract: {}\n\nDeclared permissions: {}. These declarations never widen the provider sandbox.\n\nRequired quality checks: {}.\n\nExpected output: {}.\nOwner answer: {}\nOwner approval note: {}",
                    text(&team,"name")?,text(&team,"goal")?,text(task,"title")?,text(task,"instruction")?,text(pack,"instructions")?,pack["permissions"],pack["qualityChecks"],text(pack,"expectedOutput")?,task["answer"]["answer"].as_str().unwrap_or(""),task["approval"]["note"].as_str().unwrap_or(""));
                self.apply_queue_intent_bound(intent("enqueue_work",json!({"workItem":{"schemaVersion":1,"workItemId":item_id,"title":title,"instruction":instruction.trim(),
                    "providerId":"codex","workingDirectory":workspace["workingDirectory"],"status":"queued","createdAt":now(),"revision":1}}),&Value::Null),project.as_deref())?;
                item = find(
                    &self.store.runtime_projections()?["work"],
                    "workItemId",
                    &item_id,
                );
            }
            if let Some(item) = item.filter(|i| i["status"] == "queued") {
                let r = self
                    .apply_work_intent(intent(
                        "start_work",
                        json!({"workItemId":item_id,"expectedItemRevision":item["revision"]}),
                        &Value::Null,
                    ))
                    .await?;
                if r["outcome"] == "applied" {
                    slots -= 1;
                } else if r["reasonCode"] == "PROVIDER_CONTROL_UNAVAILABLE" {
                    break;
                }
            }
        }
        Ok(find(&self.store.runtime_projections()?["teams"], "teamId", id).unwrap())
    }
}
