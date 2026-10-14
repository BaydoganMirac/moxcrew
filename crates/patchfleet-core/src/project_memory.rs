//! Project identity and local owner-source evidence. Sources are context, never
//! accepted decisions. Captured evidence derives from explicitly bound V1 facts.
use crate::{Error, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

const MAX_SAFE: u64 = 9_007_199_254_740_991;
const ACTIONS: &[&str] = &[
    "project_create",
    "project_link_workspace",
    "project_unlink_workspace",
    "project_message",
    "project_source_edit",
];
const KINDS: &[&str] = &[
    "message",
    "work_instruction",
    "team_goal",
    "owner_answer",
    "approval_note",
];
fn invalid(code: &str) -> Error {
    Error(code.into())
}
fn require(ok: bool, code: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(invalid(code))
    }
}
fn object(v: &Value) -> Result<&serde_json::Map<String, Value>> {
    v.as_object()
        .ok_or_else(|| invalid("INVALID_PROJECT_OBJECT"))
}
fn exact(v: &Value, fields: &[&str]) -> Result<()> {
    let m = object(v)?;
    require(
        m.len() == fields.len() && m.keys().all(|k| fields.contains(&k.as_str())),
        "INVALID_PROJECT_FIELDS",
    )
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| invalid("INVALID_PROJECT_STRING"))
}
fn number(v: &Value, min: u64, max: u64) -> Result<u64> {
    let n = v
        .as_u64()
        .ok_or_else(|| invalid("INVALID_PROJECT_INTEGER"))?;
    require(
        n >= min && n <= max.min(MAX_SAFE),
        "INVALID_PROJECT_INTEGER",
    )?;
    Ok(n)
}
fn uuid(s: &str) -> Result<()> {
    let u = uuid::Uuid::parse_str(s).map_err(|_| invalid("INVALID_PROJECT_ID"))?;
    require(
        s.len() == 36
            && [8, 13, 18, 23].iter().all(|i| s.as_bytes()[*i] == b'-')
            && u.get_variant() == uuid::Variant::RFC4122
            && (1..=8).contains(&u.get_version_num()),
        "INVALID_PROJECT_ID",
    )
}
fn identity(v: &Value, prefix: &str) -> Result<()> {
    let s = string(v)?;
    uuid(
        s.strip_prefix(&format!("{prefix}:"))
            .ok_or_else(|| invalid("INVALID_PROJECT_ID"))?,
    )
}
fn iso(v: &Value) -> Result<DateTime<Utc>> {
    let s = string(v)?;
    let t = DateTime::parse_from_rfc3339(s)
        .map_err(|_| invalid("INVALID_PROJECT_TIME"))?
        .with_timezone(&Utc);
    require(
        t.to_rfc3339_opts(SecondsFormat::Millis, true) == s,
        "INVALID_PROJECT_TIME",
    )?;
    Ok(t)
}
fn text(v: &Value, max: usize, safe: bool) -> Result<()> {
    let s = string(v)?;
    require(
        !s.is_empty()
            && s.trim() == s
            && s.encode_utf16().count() <= max
            && (!safe
                || !s
                    .chars()
                    .any(|c| matches!(c as u32,0..=8|11..=12|14..=31|127))),
        "INVALID_PROJECT_TEXT",
    )
}
fn subject(kind: &str, id: &str) -> Result<()> {
    match kind {
        "work" => require(
            !id.is_empty()
                && id.len() <= 256
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b)),
            "INVALID_PROJECT_SUBJECT",
        ),
        "team" => identity(&json!(id), "team"),
        _ => Err(invalid("INVALID_PROJECT_SUBJECT")),
    }
}
fn fields(action: &str) -> Result<&'static [&'static str]> {
    match action {
        "project_create" => Ok(&["name"]),
        "project_link_workspace" | "project_unlink_workspace" => {
            Ok(&["projectId", "workspaceId", "expectedProjectRevision"])
        }
        "project_message" => Ok(&["projectId", "title", "text", "expectedProjectRevision"]),
        "project_source_edit" => Ok(&[
            "projectId",
            "sourceId",
            "title",
            "text",
            "expectedSourceRevision",
        ]),
        _ => Err(invalid("UNKNOWN_PROJECT_ACTION")),
    }
}
fn validate_input(action: &str, p: &Value) -> Result<()> {
    exact(p, fields(action)?)?;
    if action == "project_create" {
        return text(&p["name"], 80, true);
    }
    identity(&p["projectId"], "project")?;
    match action {
        "project_link_workspace" | "project_unlink_workspace" => {
            identity(&p["workspaceId"], "workspace")?;
            number(&p["expectedProjectRevision"], 1, MAX_SAFE)?;
        }
        "project_message" => {
            text(&p["title"], 160, true)?;
            text(&p["text"], 50_000, true)?;
            number(&p["expectedProjectRevision"], 1, MAX_SAFE)?;
        }
        "project_source_edit" => {
            identity(&p["sourceId"], "source")?;
            text(&p["title"], 160, true)?;
            text(&p["text"], 50_000, true)?;
            number(&p["expectedSourceRevision"], 1, MAX_SAFE)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
fn validate_request(r: &Value) -> Result<()> {
    exact(
        r,
        &[
            "commandId",
            "action",
            "actorId",
            "createdAt",
            "expiresAt",
            "payload",
        ],
    )?;
    identity(&r["commandId"], "cmd")?;
    require(r["actorId"] == "local-owner", "INVALID_PROJECT_ACTOR")?;
    let action = string(&r["action"])?;
    require(ACTIONS.contains(&action), "UNKNOWN_PROJECT_ACTION")?;
    validate_input(action, &r["payload"])?;
    let created = iso(&r["createdAt"])?;
    let expires = iso(&r["expiresAt"])?;
    require(
        expires > created && expires.signed_duration_since(created).num_milliseconds() <= 300_000,
        "INVALID_PROJECT_TIME",
    )
}
fn project_record(p: &Value) -> Result<()> {
    exact(
        p,
        &[
            "projectId",
            "name",
            "createdAt",
            "revision",
            "workspaceIds",
            "sourceCount",
        ],
    )?;
    identity(&p["projectId"], "project")?;
    text(&p["name"], 80, true)?;
    iso(&p["createdAt"])?;
    number(&p["revision"], 1, MAX_SAFE)?;
    number(&p["sourceCount"], 0, MAX_SAFE)?;
    let ids = p["workspaceIds"]
        .as_array()
        .ok_or_else(|| invalid("INVALID_PROJECT_WORKSPACES"))?;
    let mut seen = HashSet::new();
    for id in ids {
        identity(id, "workspace")?;
        require(seen.insert(string(id)?), "DUPLICATE_PROJECT_WORKSPACE")?;
    }
    Ok(())
}
fn source_record(s: &Value) -> Result<()> {
    exact(
        s,
        &[
            "sourceId",
            "projectId",
            "kind",
            "title",
            "text",
            "revision",
            "actorId",
            "createdAt",
            "updatedAt",
            "origin",
        ],
    )?;
    identity(&s["sourceId"], "source")?;
    identity(&s["projectId"], "project")?;
    let kind = string(&s["kind"])?;
    require(KINDS.contains(&kind), "INVALID_PROJECT_SOURCE_KIND")?;
    // V1 work text permitted controls; do not reinterpret already accepted
    // owner input. Newly entered messages and other V1 evidence stay strict.
    text(&s["title"], 160, kind != "work_instruction")?;
    text(&s["text"], 50_000, kind != "work_instruction")?;
    number(&s["revision"], 1, MAX_SAFE)?;
    let actor = if ["owner_answer", "approval_note"].contains(&kind) {
        "owner"
    } else {
        "local-owner"
    };
    require(s["actorId"] == actor, "INVALID_PROJECT_ACTOR")?;
    iso(&s["createdAt"])?;
    iso(&s["updatedAt"])?;
    exact(&s["origin"], &["type", "id", "eventId"])?;
    uuid(string(&s["origin"]["eventId"])?)?;
    let origin_type = match kind {
        "message" => "project_message",
        "work_instruction" => "work.enqueued",
        "team_goal" => "team.created",
        "owner_answer" => "team.question.answered",
        "approval_note" => "team.approval.decided",
        _ => unreachable!(),
    };
    require(
        s["origin"]["type"] == origin_type,
        "INVALID_PROJECT_SOURCE_ORIGIN",
    )?;
    match kind {
        "message" => identity(&s["origin"]["id"], "cmd")?,
        "work_instruction" => subject("work", string(&s["origin"]["id"])?)?,
        "team_goal" => identity(&s["origin"]["id"], "team")?,
        "owner_answer" => identity(&s["origin"]["id"], "question")?,
        "approval_note" => identity(&s["origin"]["id"], "task")?,
        _ => unreachable!(),
    }
    Ok(())
}
fn validate_receipt(r: &Value) -> Result<()> {
    exact(
        r,
        &[
            "schemaVersion",
            "commandId",
            "action",
            "outcome",
            "reasonCode",
            "completedAt",
            "projectId",
            "projectRevision",
            "memoryRevision",
            "sourceId",
            "sourceRevision",
        ],
    )?;
    require(r["schemaVersion"] == 3, "INVALID_PROJECT_RECEIPT_VERSION")?;
    identity(&r["commandId"], "cmd")?;
    require(
        ACTIONS.contains(&string(&r["action"])?),
        "UNKNOWN_PROJECT_ACTION",
    )?;
    require(
        ["applied", "rejected", "expired"].contains(&string(&r["outcome"])?),
        "INVALID_PROJECT_OUTCOME",
    )?;
    let reason = string(&r["reasonCode"])?;
    require(
        (2..=64).contains(&reason.len())
            && reason
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'),
        "INVALID_PROJECT_REASON",
    )?;
    if !r["projectId"].is_null() {
        identity(&r["projectId"], "project")?;
    }
    if !r["sourceId"].is_null() {
        identity(&r["sourceId"], "source")?;
    }
    for k in ["projectRevision", "memoryRevision", "sourceRevision"] {
        number(&r[k], 0, MAX_SAFE)?;
    }
    iso(&r["completedAt"])?;
    Ok(())
}

pub fn validate_event(e: &Value) -> Result<()> {
    exact(e, &["id", "schemaVersion", "type", "recordedAt", "payload"])?;
    uuid(string(&e["id"])?)?;
    require(e["schemaVersion"] == 3, "INVALID_PROJECT_EVENT_VERSION")?;
    iso(&e["recordedAt"])?;
    let p = &e["payload"];
    match string(&e["type"])? {
        "project.command.requested" => {
            validate_request(p)?;
            require(
                iso(&p["createdAt"])? <= iso(&e["recordedAt"])? + chrono::Duration::seconds(60),
                "INVALID_PROJECT_TIME",
            )?;
        }
        "project.command.receipted" => {
            exact(p, &["receipt"])?;
            validate_receipt(&p["receipt"])?;
            require(
                p["receipt"]["completedAt"] == e["recordedAt"],
                "PROJECT_RECEIPT_TIME_MISMATCH",
            )?;
        }
        "project.created" => {
            exact(p, &["commandId", "project"])?;
            identity(&p["commandId"], "cmd")?;
            project_record(&p["project"])?;
            require(
                p["project"]["projectId"] == derived("project", string(&p["commandId"])?)
                    && p["project"]["createdAt"] == e["recordedAt"]
                    && p["project"]["revision"] == 1
                    && p["project"]["sourceCount"] == 0
                    && p["project"]["workspaceIds"].as_array().unwrap().is_empty(),
                "INVALID_PROJECT_CREATION",
            )?;
        }
        "project.workspace.linked" | "project.workspace.unlinked" => {
            exact(
                p,
                &[
                    "commandId",
                    "projectId",
                    "workspaceId",
                    "expectedProjectRevision",
                ],
            )?;
            identity(&p["commandId"], "cmd")?;
            identity(&p["projectId"], "project")?;
            identity(&p["workspaceId"], "workspace")?;
            number(&p["expectedProjectRevision"], 1, MAX_SAFE)?;
        }
        "project.source.created" => {
            exact(p, &["commandId", "source"])?;
            identity(&p["commandId"], "cmd")?;
            source_record(&p["source"])?;
            require(
                p["source"]["kind"] == "message",
                "INVALID_PROJECT_SOURCE_KIND",
            )?;
            require(
                p["source"]["sourceId"] == derived("source", string(&p["commandId"])?)
                    && p["source"]["revision"] == 1
                    && p["source"]["createdAt"] == e["recordedAt"]
                    && p["source"]["updatedAt"] == e["recordedAt"]
                    && p["source"]["origin"]["eventId"] == e["id"]
                    && p["source"]["origin"]["id"] == p["commandId"],
                "INVALID_PROJECT_SOURCE_ORIGIN",
            )?;
        }
        "project.source.edited" => {
            exact(
                p,
                &[
                    "commandId",
                    "projectId",
                    "sourceId",
                    "title",
                    "text",
                    "expectedSourceRevision",
                ],
            )?;
            identity(&p["commandId"], "cmd")?;
            identity(&p["projectId"], "project")?;
            identity(&p["sourceId"], "source")?;
            text(&p["title"], 160, true)?;
            text(&p["text"], 50_000, true)?;
            number(&p["expectedSourceRevision"], 1, MAX_SAFE)?;
        }
        "project.subject.bound" => {
            exact(p, &["projectId", "subjectKind", "subjectId"])?;
            identity(&p["projectId"], "project")?;
            subject(string(&p["subjectKind"])?, string(&p["subjectId"])?)?;
        }
        _ => return Err(invalid("UNKNOWN_PROJECT_EVENT")),
    }
    Ok(())
}
fn event(kind: &str, payload: Value, at: &str) -> Value {
    json!({"id":uuid::Uuid::new_v4().to_string(),"schemaVersion":3,"type":kind,"recordedAt":at,"payload":payload})
}
pub fn bind(project_id: &str, subject_kind: &str, subject_id: &str) -> Value {
    event(
        "project.subject.bound",
        json!({"projectId":project_id,"subjectKind":subject_kind,"subjectId":subject_id}),
        &crate::now(),
    )
}
fn derived(prefix: &str, command: &str) -> String {
    format!(
        "{prefix}:{}",
        command
            .strip_prefix("cmd:")
            .expect("validated command UUID")
    )
}
fn id(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_owned()
}
fn truncate(s: &str, max: usize) -> String {
    let mut units = 0;
    s.chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= max
        })
        .collect()
}

#[derive(Default)]
struct State {
    revision: u64,
    projects: BTreeMap<String, Value>,
    sources: BTreeMap<String, Value>,
    bindings: BTreeMap<(String, String), String>,
    workspaces: HashSet<String>,
    requests: BTreeMap<String, Value>,
    facts: BTreeMap<String, Value>,
    receipts: BTreeMap<String, Value>,
}
impl State {
    fn render(&self) -> Value {
        let mut projects: Vec<_> = self.projects.values().cloned().collect();
        projects.sort_by(|a, b| {
            a["createdAt"]
                .as_str()
                .cmp(&b["createdAt"].as_str())
                .then(a["projectId"].as_str().cmp(&b["projectId"].as_str()))
        });
        let mut sources: Vec<_> = self.sources.values().cloned().collect();
        sources.sort_by(|a, b| {
            a["createdAt"]
                .as_str()
                .cmp(&b["createdAt"].as_str())
                .then(a["sourceId"].as_str().cmp(&b["sourceId"].as_str()))
        });
        let bindings: Vec<_> = self
            .bindings
            .iter()
            .map(|((kind, id), p)| json!({"projectId":p,"subjectKind":kind,"subjectId":id}))
            .collect();
        json!({"schemaVersion":3,"revision":self.revision,"projects":projects,"sources":sources,"bindings":bindings})
    }
    fn touch(&mut self, project_id: &str, source_added: bool) -> Result<()> {
        let p = self
            .projects
            .get_mut(project_id)
            .ok_or_else(|| invalid("PROJECT_NOT_FOUND"))?;
        p["revision"] = json!(p["revision"].as_u64().unwrap() + 1);
        if source_added {
            p["sourceCount"] = json!(p["sourceCount"].as_u64().unwrap() + 1);
        }
        self.revision += 1;
        Ok(())
    }
    fn workspace_owner(&self, w: &str) -> Option<String> {
        self.projects
            .values()
            .find(|p| p["workspaceIds"].as_array().unwrap().iter().any(|v| v == w))
            .map(|p| id(p, "projectId"))
    }
    fn targets(request: &Value) -> (String, Option<String>) {
        let action = request["action"].as_str().unwrap();
        let cmd = request["commandId"].as_str().unwrap();
        let p = &request["payload"];
        let project = if action == "project_create" {
            derived("project", cmd)
        } else {
            id(p, "projectId")
        };
        let source = match action {
            "project_message" => Some(derived("source", cmd)),
            "project_source_edit" => Some(id(p, "sourceId")),
            _ => None,
        };
        (project, source)
    }
    fn outcome(&self, request: &Value, at: &str) -> (&'static str, &'static str) {
        if request["expiresAt"].as_str().unwrap() <= at {
            return ("expired", "COMMAND_EXPIRED");
        }
        let action = request["action"].as_str().unwrap();
        let p = &request["payload"];
        let (project_id, source_id) = Self::targets(request);
        if action == "project_create" {
            return if self.projects.contains_key(&project_id) {
                ("rejected", "PROJECT_ALREADY_EXISTS")
            } else {
                ("applied", "PROJECT_CREATED")
            };
        }
        let Some(project) = self.projects.get(&project_id) else {
            return ("rejected", "PROJECT_NOT_FOUND");
        };
        if action != "project_source_edit" && project["revision"] != p["expectedProjectRevision"] {
            return ("rejected", "STALE_PROJECT_REVISION");
        }
        match action {
            "project_link_workspace" => {
                if !self.workspaces.contains(&id(p, "workspaceId")) {
                    return ("rejected", "WORKSPACE_NOT_FOUND");
                }
                if let Some(owner) = self.workspace_owner(&id(p, "workspaceId")) {
                    return if owner == project_id {
                        ("rejected", "WORKSPACE_ALREADY_LINKED")
                    } else {
                        ("rejected", "WORKSPACE_LINKED_TO_OTHER_PROJECT")
                    };
                }
                ("applied", "PROJECT_WORKSPACE_LINKED")
            }
            "project_unlink_workspace" => {
                if self.workspace_owner(&id(p, "workspaceId")).as_deref() != Some(&project_id) {
                    ("rejected", "WORKSPACE_NOT_LINKED")
                } else {
                    ("applied", "PROJECT_WORKSPACE_UNLINKED")
                }
            }
            "project_message" => {
                if self.sources.contains_key(source_id.as_ref().unwrap()) {
                    ("rejected", "SOURCE_ALREADY_EXISTS")
                } else {
                    ("applied", "PROJECT_MESSAGE_CREATED")
                }
            }
            "project_source_edit" => {
                let Some(source) = self.sources.get(source_id.as_ref().unwrap()) else {
                    return ("rejected", "SOURCE_NOT_FOUND");
                };
                if source["projectId"] != p["projectId"] {
                    return ("rejected", "SOURCE_NOT_IN_PROJECT");
                }
                if source["kind"] != "message" {
                    return ("rejected", "SOURCE_IMMUTABLE");
                }
                if source["revision"] != p["expectedSourceRevision"] {
                    return ("rejected", "STALE_SOURCE_REVISION");
                }
                ("applied", "PROJECT_SOURCE_EDITED")
            }
            _ => unreachable!(),
        }
    }
    fn fact(request: &Value, at: &str) -> Value {
        let action = request["action"].as_str().unwrap();
        let cmd = request["commandId"].as_str().unwrap();
        let p = &request["payload"];
        let (project_id, source_id) = Self::targets(request);
        match action {
            "project_create" => event(
                "project.created",
                json!({"commandId":cmd,"project":{"projectId":project_id,"name":p["name"],"createdAt":at,"revision":1,"workspaceIds":[],"sourceCount":0}}),
                at,
            ),
            "project_link_workspace" | "project_unlink_workspace" => event(
                if action == "project_link_workspace" {
                    "project.workspace.linked"
                } else {
                    "project.workspace.unlinked"
                },
                json!({"commandId":cmd,"projectId":project_id,"workspaceId":p["workspaceId"],"expectedProjectRevision":p["expectedProjectRevision"]}),
                at,
            ),
            "project_message" => {
                let mut e = event("project.source.created", Value::Null, at);
                e["payload"] = json!({"commandId":cmd,"source":{"sourceId":source_id,"projectId":project_id,"kind":"message","title":p["title"],"text":p["text"],"revision":1,"actorId":"local-owner","createdAt":at,"updatedAt":at,"origin":{"type":"project_message","id":cmd,"eventId":e["id"]}}});
                e
            }
            "project_source_edit" => event(
                "project.source.edited",
                json!({"commandId":cmd,"projectId":project_id,"sourceId":source_id,"title":p["title"],"text":p["text"],"expectedSourceRevision":p["expectedSourceRevision"]}),
                at,
            ),
            _ => unreachable!(),
        }
    }
    fn receipt(&self, request: &Value, outcome: &str, reason: &str, at: &str) -> Value {
        let (project_id, source_id) = Self::targets(request);
        json!({"schemaVersion":3,"commandId":request["commandId"],"action":request["action"],"outcome":outcome,"reasonCode":reason,"completedAt":at,"projectId":project_id,"projectRevision":self.projects.get(&project_id).map(|v|v["revision"].clone()).unwrap_or(json!(0)),"memoryRevision":self.revision,"sourceId":source_id,"sourceRevision":source_id.as_ref().and_then(|id|self.sources.get(id)).filter(|s|s["projectId"]==project_id).map(|v|v["revision"].clone()).unwrap_or(json!(0))})
    }
    fn apply_fact(&mut self, e: &Value) -> Result<()> {
        let p = &e["payload"];
        let cmd = id(p, "commandId");
        let request = self
            .requests
            .get(&cmd)
            .ok_or_else(|| invalid("PROJECT_FACT_WITHOUT_REQUEST"))?
            .clone();
        require(
            !self.facts.contains_key(&cmd) && !self.receipts.contains_key(&cmd),
            "DUPLICATE_PROJECT_FACT",
        )?;
        require(
            self.outcome(&request, string(&e["recordedAt"])?).0 == "applied",
            "INVALID_PROJECT_FACT_TRANSITION",
        )?;
        let mut expected = Self::fact(&request, string(&e["recordedAt"])?);
        if e["type"] == "project.source.created" {
            expected["payload"]["source"]["origin"]["eventId"] = e["id"].clone();
        }
        require(
            expected["type"] == e["type"] && expected["payload"] == *p,
            "PROJECT_FACT_REQUEST_MISMATCH",
        )?;
        let (project_id, _) = Self::targets(&request);
        match string(&e["type"])? {
            "project.created" => {
                self.projects.insert(project_id, p["project"].clone());
                self.revision += 1;
            }
            "project.workspace.linked" | "project.workspace.unlinked" => {
                let ids = self.projects.get_mut(&project_id).unwrap()["workspaceIds"]
                    .as_array_mut()
                    .unwrap();
                if e["type"] == "project.workspace.linked" {
                    ids.push(p["workspaceId"].clone());
                    ids.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                } else {
                    ids.retain(|v| *v != p["workspaceId"]);
                }
                self.touch(&project_id, false)?;
            }
            "project.source.created" => {
                self.sources
                    .insert(id(&p["source"], "sourceId"), p["source"].clone());
                self.touch(&project_id, true)?;
            }
            "project.source.edited" => {
                let s = self.sources.get_mut(&id(p, "sourceId")).unwrap();
                s["title"] = p["title"].clone();
                s["text"] = p["text"].clone();
                s["updatedAt"] = e["recordedAt"].clone();
                s["revision"] = json!(s["revision"].as_u64().unwrap() + 1);
                self.touch(&project_id, false)?;
            }
            _ => unreachable!(),
        }
        self.facts.insert(cmd, e.clone());
        Ok(())
    }
    fn automatic(
        &mut self,
        e: &Value,
        bound_subject: (&str, &str),
        kind: &str,
        title: &str,
        text: &str,
        origin_id: &str,
    ) -> Result<()> {
        let (subject_kind, subject_id) = bound_subject;
        let Some(project_id) = self
            .bindings
            .get(&(subject_kind.into(), subject_id.into()))
            .cloned()
        else {
            return Ok(());
        };
        let source_id = format!("source:{}", string(&e["id"])?);
        require(
            !self.sources.contains_key(&source_id),
            "DUPLICATE_PROJECT_SOURCE",
        )?;
        let s = json!({"sourceId":source_id,"projectId":project_id,"kind":kind,"title":title,"text":text,"revision":1,"actorId":if ["owner_answer","approval_note"].contains(&kind){"owner"}else{"local-owner"},"createdAt":e["recordedAt"],"updatedAt":e["recordedAt"],"origin":{"type":e["type"],"id":origin_id,"eventId":e["id"]}});
        source_record(&s)?;
        self.sources.insert(source_id, s);
        self.touch(&project_id, true)
    }
    fn consume(&mut self, e: &Value) -> Result<()> {
        let p = &e["payload"];
        match string(&e["type"])? {
            "project.command.requested" => {
                let cmd = id(p, "commandId");
                require(
                    self.requests.insert(cmd, p.clone()).is_none(),
                    "DUPLICATE_PROJECT_COMMAND",
                )?;
            }
            "project.command.receipted" => {
                let r = &p["receipt"];
                let cmd = id(r, "commandId");
                let request = self
                    .requests
                    .get(&cmd)
                    .ok_or_else(|| invalid("PROJECT_RECEIPT_WITHOUT_REQUEST"))?;
                require(
                    !self.receipts.contains_key(&cmd),
                    "DUPLICATE_PROJECT_RECEIPT",
                )?;
                let (outcome, reason) = if self.facts.contains_key(&cmd) {
                    (
                        "applied",
                        match request["action"].as_str().unwrap() {
                            "project_create" => "PROJECT_CREATED",
                            "project_link_workspace" => "PROJECT_WORKSPACE_LINKED",
                            "project_unlink_workspace" => "PROJECT_WORKSPACE_UNLINKED",
                            "project_message" => "PROJECT_MESSAGE_CREATED",
                            "project_source_edit" => "PROJECT_SOURCE_EDITED",
                            _ => unreachable!(),
                        },
                    )
                } else {
                    self.outcome(request, string(&r["completedAt"])?)
                };
                require(
                    outcome != "applied" || self.facts.contains_key(&cmd),
                    "PROJECT_RECEIPT_WITHOUT_FACT",
                )?;
                require(
                    self.receipt(request, outcome, reason, string(&r["completedAt"])?) == *r,
                    "INVALID_PROJECT_RECEIPT_TRANSITION",
                )?;
                self.receipts.insert(cmd, r.clone());
            }
            "project.created"
            | "project.workspace.linked"
            | "project.workspace.unlinked"
            | "project.source.created"
            | "project.source.edited" => self.apply_fact(e)?,
            "project.subject.bound" => {
                let project_id = id(p, "projectId");
                require(self.projects.contains_key(&project_id), "PROJECT_NOT_FOUND")?;
                let key = (id(p, "subjectKind"), id(p, "subjectId"));
                if let Some(existing) = self.bindings.get(&key) {
                    require(*existing == project_id, "PROJECT_SUBJECT_ALREADY_BOUND")?;
                } else {
                    self.bindings.insert(key, project_id);
                    self.revision += 1;
                }
            }
            "workspace.registered" => {
                self.workspaces.insert(id(&p["workspace"], "workspaceId"));
            }
            "workspace.removed" => {
                let w = id(p, "workspaceId");
                self.workspaces.remove(&w);
                if let Some(project_id) = self.workspace_owner(&w) {
                    self.projects.get_mut(&project_id).unwrap()["workspaceIds"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|v| v != &json!(w));
                    self.touch(&project_id, false)?;
                }
            }
            "work.enqueued" => {
                let work = &p["workItem"];
                let work_id = id(work, "workItemId");
                self.automatic(
                    e,
                    ("work", &work_id),
                    "work_instruction",
                    string(&work["title"])?,
                    string(&work["instruction"])?,
                    &work_id,
                )?;
            }
            "team.created" => {
                let t = &p["team"];
                let team_id = id(t, "teamId");
                self.automatic(
                    e,
                    ("team", &team_id),
                    "team_goal",
                    string(&t["name"])?,
                    string(&t["goal"])?,
                    &team_id,
                )?;
            }
            "team.question.answered" => {
                let t = id(p, "teamId");
                self.automatic(
                    e,
                    ("team", &t),
                    "owner_answer",
                    "Owner answer",
                    string(&p["answer"])?,
                    string(&p["questionId"])?,
                )?;
            }
            "team.approval.decided" => {
                if let Some(note) = p["note"].as_str() {
                    let t = id(p, "teamId");
                    let title =
                        truncate(&format!("Owner approval: {}", string(&p["decision"])?), 160);
                    self.automatic(
                        e,
                        ("team", &t),
                        "approval_note",
                        &title,
                        note,
                        string(&p["taskId"])?,
                    )?;
                }
            }
            _ => (),
        }
        Ok(())
    }
}
fn replay_validated(events: &[Value]) -> Result<State> {
    let mut state = State::default();
    for e in events {
        state.consume(e)?;
    }
    Ok(state)
}
pub(crate) fn project_validated(events: &[Value]) -> Result<Value> {
    Ok(replay_validated(events)?.render())
}
pub fn project(events: &[Value]) -> Result<Value> {
    crate::domain::project_legacy_events(events)?;
    project_validated(events)
}

fn normalized(events: &[Value], action: &str, payload: &Value, at: &str) -> Result<Value> {
    let fields = fields(action)?;
    let map = object(payload)?;
    require(
        map.keys().all(|k| {
            fields.contains(&k.as_str())
                || ["commandId", "createdAt", "expiresAt"].contains(&k.as_str())
        }) && fields.iter().all(|f| map.contains_key(*f)),
        "INVALID_PROJECT_FIELDS",
    )?;
    let command = payload
        .get("commandId")
        .cloned()
        .unwrap_or_else(|| json!(format!("cmd:{}", uuid::Uuid::new_v4())));
    identity(&command, "cmd")?;
    let prior = events
        .iter()
        .find(|e| e["type"] == "project.command.requested" && e["payload"]["commandId"] == command)
        .map(|e| &e["payload"]);
    let created = payload
        .get("createdAt")
        .cloned()
        .or_else(|| prior.map(|p| p["createdAt"].clone()))
        .unwrap_or_else(|| json!(at));
    let created_time = iso(&created)?;
    let expires = payload
        .get("expiresAt")
        .cloned()
        .or_else(|| prior.map(|p| p["expiresAt"].clone()))
        .unwrap_or_else(|| {
            json!((created_time + chrono::Duration::minutes(5))
                .to_rfc3339_opts(SecondsFormat::Millis, true))
        });
    let input: serde_json::Map<String, Value> = fields
        .iter()
        .map(|f| ((*f).to_owned(), payload[*f].clone()))
        .collect();
    let request = json!({"commandId":command,"action":action,"actorId":"local-owner","createdAt":created,"expiresAt":expires,"payload":input});
    validate_request(&request)?;
    require(
        iso(&request["createdAt"])? <= iso(&json!(at))? + chrono::Duration::seconds(60),
        "INVALID_PROJECT_TIME",
    )?;
    Ok(request)
}
pub fn apply(events: &[Value], action: &str, payload: &Value) -> Result<(Vec<Value>, Value)> {
    crate::domain::project_legacy_events(events)?;
    let state = replay_validated(events)?;
    let at = crate::now();
    let request = normalized(events, action, payload, &at)?;
    let cmd = id(&request, "commandId");
    if let Some(prior) = state.requests.get(&cmd) {
        require(*prior == request, "IDEMPOTENCY_KEY_REUSED")?;
        if let Some(receipt) = state.receipts.get(&cmd) {
            return Ok((vec![], receipt.clone()));
        }
    }
    let mut additions = Vec::new();
    if !state.requests.contains_key(&cmd) {
        additions.push(event("project.command.requested", request.clone(), &at));
    }
    let (outcome, reason) = if state.facts.contains_key(&cmd) {
        (
            "applied",
            match action {
                "project_create" => "PROJECT_CREATED",
                "project_link_workspace" => "PROJECT_WORKSPACE_LINKED",
                "project_unlink_workspace" => "PROJECT_WORKSPACE_UNLINKED",
                "project_message" => "PROJECT_MESSAGE_CREATED",
                "project_source_edit" => "PROJECT_SOURCE_EDITED",
                _ => unreachable!(),
            },
        )
    } else {
        state.outcome(&request, &at)
    };
    if outcome == "applied" && !state.facts.contains_key(&cmd) {
        additions.push(State::fact(&request, &at));
    }
    let mut combined = events.to_vec();
    combined.extend(additions.iter().cloned());
    let post = replay_validated(&combined)?;
    let receipt = post.receipt(&request, outcome, reason, &at);
    additions.push(event(
        "project.command.receipted",
        json!({"receipt":receipt}),
        &at,
    ));
    combined = events.to_vec();
    combined.extend(additions.iter().cloned());
    crate::domain::project_events(&combined)?;
    Ok((additions, receipt))
}
pub fn recover(events: &[Value]) -> Result<Vec<Value>> {
    crate::domain::project_legacy_events(events)?;
    let state = replay_validated(events)?;
    let mut combined = events.to_vec();
    let mut additions = Vec::new();
    for e in events.iter().filter(|e| {
        e["type"] == "project.command.requested"
            && !state.receipts.contains_key(&id(&e["payload"], "commandId"))
    }) {
        let request = &e["payload"];
        let mut payload = request["payload"].clone();
        for k in ["commandId", "createdAt", "expiresAt"] {
            payload[k] = request[k].clone();
        }
        let (next, _) = apply(&combined, string(&request["action"])?, &payload)?;
        combined.extend(next.iter().cloned());
        additions.extend(next);
    }
    Ok(additions)
}
pub fn metadata(projection: &Value) -> Value {
    json!({"schemaVersion":3,"revision":projection["revision"],"projects":projection["projects"]})
}
pub fn read(projection: &Value, payload: &Value) -> Result<Value> {
    let map = object(payload)?;
    require(
        map.contains_key("projectId")
            && map
                .keys()
                .all(|k| ["projectId", "offset", "limit", "query"].contains(&k.as_str())),
        "INVALID_PROJECT_FIELDS",
    )?;
    identity(&payload["projectId"], "project")?;
    let offset = payload
        .get("offset")
        .map(|v| number(v, 0, MAX_SAFE))
        .transpose()?
        .unwrap_or(0);
    let limit = payload
        .get("limit")
        .map(|v| number(v, 1, 20))
        .transpose()?
        .unwrap_or(10);
    let query = payload.get("query").map(string).transpose()?.unwrap_or("");
    require(
        query.encode_utf16().count() <= 160
            && !query
                .chars()
                .any(|c| matches!(c as u32,0..=8|11..=12|14..=31|127)),
        "INVALID_PROJECT_QUERY",
    )?;
    let query = query.trim();
    let lower = query.to_lowercase();
    let project = projection["projects"]
        .as_array()
        .and_then(|ps| ps.iter().find(|p| p["projectId"] == payload["projectId"]))
        .ok_or_else(|| invalid("PROJECT_NOT_FOUND"))?;
    let mut matches: Vec<_> = projection["sources"]
        .as_array()
        .ok_or_else(|| invalid("INVALID_PROJECT_PROJECTION"))?
        .iter()
        .filter(|s| {
            s["projectId"] == payload["projectId"]
                && (lower.is_empty()
                    || s["title"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&lower)
                    || s["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&lower))
        })
        .collect();
    matches.sort_by(|a, b| {
        b["updatedAt"]
            .as_str()
            .cmp(&a["updatedAt"].as_str())
            .then(a["sourceId"].as_str().cmp(&b["sourceId"].as_str()))
    });
    let total = matches.len();
    let sources: Vec<_> = matches
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .cloned()
        .collect();
    Ok(
        json!({"reasonCode":"PROJECT_MEMORY_READ","projectId":payload["projectId"],"revision":project["revision"],"offset":offset,"query":query,"total":total,"sources":sources}),
    )
}
