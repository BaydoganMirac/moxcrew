//! Owner-authorized, sourced local ADRs. V4 facts freeze references, while
//! original source bodies are resolved once from canonical V3 evidence.
use crate::{Error, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

const MAX_SAFE: u64 = 9_007_199_254_740_991;
const ACTIONS: &[&str] = &[
    "project_decision_propose",
    "project_decision_accept",
    "project_decision_reject",
    "project_decision_supersede",
];
const BODY: &[(&str, usize)] = &[
    ("title", 160),
    ("context", 10_000),
    ("decision", 20_000),
    ("rationale", 10_000),
    ("consequences", 10_000),
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
        .ok_or_else(|| invalid("INVALID_DECISION_OBJECT"))
}
fn exact(v: &Value, fields: &[&str]) -> Result<()> {
    let m = object(v)?;
    require(
        m.len() == fields.len() && m.keys().all(|k| fields.contains(&k.as_str())),
        "INVALID_DECISION_FIELDS",
    )
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| invalid("INVALID_DECISION_STRING"))
}
fn number(v: &Value, min: u64, max: u64) -> Result<u64> {
    let n = v
        .as_u64()
        .ok_or_else(|| invalid("INVALID_DECISION_INTEGER"))?;
    require(
        n >= min && n <= max.min(MAX_SAFE),
        "INVALID_DECISION_INTEGER",
    )?;
    Ok(n)
}
fn uuid(s: &str) -> Result<()> {
    let u = uuid::Uuid::parse_str(s).map_err(|_| invalid("INVALID_DECISION_ID"))?;
    require(
        s.len() == 36
            && [8, 13, 18, 23].iter().all(|i| s.as_bytes()[*i] == b'-')
            && u.get_variant() == uuid::Variant::RFC4122
            && (1..=8).contains(&u.get_version_num()),
        "INVALID_DECISION_ID",
    )
}
fn identity(v: &Value, prefix: &str) -> Result<()> {
    uuid(
        string(v)?
            .strip_prefix(&format!("{prefix}:"))
            .ok_or_else(|| invalid("INVALID_DECISION_ID"))?,
    )
}
fn iso(v: &Value) -> Result<DateTime<Utc>> {
    let s = string(v)?;
    let t = DateTime::parse_from_rfc3339(s)
        .map_err(|_| invalid("INVALID_DECISION_TIME"))?
        .with_timezone(&Utc);
    require(
        t.to_rfc3339_opts(SecondsFormat::Millis, true) == s,
        "INVALID_DECISION_TIME",
    )?;
    Ok(t)
}
fn safe_text(s: &str, max: usize, empty: bool) -> Result<()> {
    require(
        (empty || !s.is_empty())
            && s.trim() == s
            && s.encode_utf16().count() <= max
            && !s
                .chars()
                .any(|c| matches!(c as u32, 0..=8 | 11..=12 | 14..=31 | 127)),
        "INVALID_DECISION_TEXT",
    )
}
fn id(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_owned()
}
fn derived(command: &str) -> String {
    format!(
        "decision:{}",
        command.strip_prefix("cmd:").expect("validated command")
    )
}
fn fields(action: &str) -> Result<&'static [&'static str]> {
    match action {
        "project_decision_propose" => Ok(&[
            "projectId",
            "expectedProjectRevision",
            "title",
            "context",
            "decision",
            "rationale",
            "consequences",
            "sources",
        ]),
        "project_decision_accept" | "project_decision_reject" => {
            Ok(&["projectId", "decisionId", "expectedDecisionRevision"])
        }
        "project_decision_supersede" => Ok(&[
            "projectId",
            "decisionId",
            "expectedDecisionRevision",
            "replacementDecisionId",
            "expectedReplacementRevision",
        ]),
        _ => Err(invalid("UNKNOWN_DECISION_ACTION")),
    }
}
fn validate_input(action: &str, p: &Value) -> Result<()> {
    exact(p, fields(action)?)?;
    identity(&p["projectId"], "project")?;
    if action == "project_decision_propose" {
        number(&p["expectedProjectRevision"], 1, MAX_SAFE)?;
        for (key, max) in BODY {
            safe_text(
                string(&p[*key])?,
                *max,
                !["title", "decision"].contains(key),
            )?;
        }
        let refs = p["sources"]
            .as_array()
            .ok_or_else(|| invalid("INVALID_DECISION_SOURCES"))?;
        require((1..=8).contains(&refs.len()), "INVALID_DECISION_SOURCES")?;
        let mut seen = HashSet::new();
        for r in refs {
            exact(r, &["sourceId", "revision"])?;
            identity(&r["sourceId"], "source")?;
            number(&r["revision"], 1, MAX_SAFE)?;
            require(
                seen.insert(string(&r["sourceId"])?),
                "DUPLICATE_DECISION_SOURCE",
            )?;
        }
    } else {
        identity(&p["decisionId"], "decision")?;
        number(&p["expectedDecisionRevision"], 1, MAX_SAFE)?;
        if action == "project_decision_supersede" {
            identity(&p["replacementDecisionId"], "decision")?;
            number(&p["expectedReplacementRevision"], 1, MAX_SAFE)?;
        }
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
    require(r["actorId"] == "local-owner", "INVALID_DECISION_ACTOR")?;
    validate_input(string(&r["action"])?, &r["payload"])?;
    let created = iso(&r["createdAt"])?;
    let expires = iso(&r["expiresAt"])?;
    require(
        expires > created && expires.signed_duration_since(created).num_milliseconds() <= 300_000,
        "INVALID_DECISION_TIME",
    )
}
fn validate_record(d: &Value) -> Result<()> {
    exact(
        d,
        &[
            "decisionId",
            "projectId",
            "title",
            "status",
            "revision",
            "createdAt",
            "updatedAt",
            "sourceCount",
            "supersedesDecisionId",
            "supersededByDecisionId",
            "context",
            "decision",
            "rationale",
            "consequences",
            "actorId",
            "sources",
        ],
    )?;
    identity(&d["decisionId"], "decision")?;
    identity(&d["projectId"], "project")?;
    require(
        d["actorId"] == "local-owner"
            && d["status"] == "proposed"
            && d["revision"] == 1
            && d["supersedesDecisionId"].is_null()
            && d["supersededByDecisionId"].is_null(),
        "INVALID_DECISION_CREATION",
    )?;
    iso(&d["createdAt"])?;
    iso(&d["updatedAt"])?;
    for (key, max) in BODY {
        safe_text(
            string(&d[*key])?,
            *max,
            !["title", "decision"].contains(key),
        )?;
    }
    let refs = d["sources"]
        .as_array()
        .ok_or_else(|| invalid("INVALID_DECISION_SOURCES"))?;
    require(
        (1..=8).contains(&refs.len()) && d["sourceCount"] == refs.len(),
        "INVALID_DECISION_SOURCES",
    )?;
    let mut seen = HashSet::new();
    for r in refs {
        exact(r, &["sourceId", "revision", "origin"])?;
        identity(&r["sourceId"], "source")?;
        number(&r["revision"], 1, MAX_SAFE)?;
        exact(&r["origin"], &["type", "id", "eventId"])?;
        string(&r["origin"]["type"])?;
        string(&r["origin"]["id"])?;
        uuid(string(&r["origin"]["eventId"])?)?;
        require(
            seen.insert(string(&r["sourceId"])?),
            "DUPLICATE_DECISION_SOURCE",
        )?;
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
            "decisionId",
            "decisionRevision",
            "replacementDecisionId",
            "replacementRevision",
        ],
    )?;
    require(r["schemaVersion"] == 4, "INVALID_DECISION_RECEIPT_VERSION")?;
    identity(&r["commandId"], "cmd")?;
    require(
        ACTIONS.contains(&string(&r["action"])?),
        "UNKNOWN_DECISION_ACTION",
    )?;
    require(
        ["applied", "rejected", "expired"].contains(&string(&r["outcome"])?),
        "INVALID_DECISION_OUTCOME",
    )?;
    let reason = string(&r["reasonCode"])?;
    require(
        (2..=64).contains(&reason.len())
            && reason
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'),
        "INVALID_DECISION_REASON",
    )?;
    identity(&r["projectId"], "project")?;
    for key in ["decisionId", "replacementDecisionId"] {
        if !r[key].is_null() {
            identity(&r[key], "decision")?;
        }
    }
    for key in ["projectRevision", "decisionRevision", "replacementRevision"] {
        number(&r[key], 0, MAX_SAFE)?;
    }
    iso(&r["completedAt"])?;
    Ok(())
}
pub fn validate_event(e: &Value) -> Result<()> {
    exact(e, &["id", "schemaVersion", "type", "recordedAt", "payload"])?;
    uuid(string(&e["id"])?)?;
    require(e["schemaVersion"] == 4, "INVALID_DECISION_EVENT_VERSION")?;
    iso(&e["recordedAt"])?;
    let p = &e["payload"];
    match string(&e["type"])? {
        "decision.command.requested" => {
            validate_request(p)?;
            require(
                iso(&p["createdAt"])? <= iso(&e["recordedAt"])? + Duration::seconds(60),
                "INVALID_DECISION_TIME",
            )?;
        }
        "decision.command.receipted" => {
            exact(p, &["receipt"])?;
            validate_receipt(&p["receipt"])?;
            require(
                p["receipt"]["completedAt"] == e["recordedAt"],
                "DECISION_RECEIPT_TIME_MISMATCH",
            )?;
        }
        "decision.proposed" => {
            exact(p, &["commandId", "decision"])?;
            identity(&p["commandId"], "cmd")?;
            validate_record(&p["decision"])?;
            require(
                p["decision"]["decisionId"] == derived(string(&p["commandId"])?)
                    && p["decision"]["createdAt"] == e["recordedAt"]
                    && p["decision"]["updatedAt"] == e["recordedAt"],
                "INVALID_DECISION_CREATION",
            )?;
        }
        "decision.accepted" | "decision.rejected" | "decision.superseded" => {
            let action = match string(&e["type"])? {
                "decision.accepted" => ACTIONS[1],
                "decision.rejected" => ACTIONS[2],
                _ => ACTIONS[3],
            };
            let mut input = p.clone();
            let cmd = object(&input)?
                .get("commandId")
                .cloned()
                .ok_or_else(|| invalid("INVALID_DECISION_FIELDS"))?;
            identity(&cmd, "cmd")?;
            input.as_object_mut().unwrap().remove("commandId");
            validate_input(action, &input)?;
        }
        _ => return Err(invalid("UNKNOWN_DECISION_EVENT")),
    }
    Ok(())
}
fn event(kind: &str, payload: Value, at: &str) -> Value {
    json!({"id":uuid::Uuid::new_v4().to_string(),"schemaVersion":4,"type":kind,"recordedAt":at,"payload":payload})
}

/// V3 has already validated source attribution and edits. Track its timeline
/// once so a decision cannot cite a source or revision from the future.
#[derive(Default)]
struct Evidence {
    projects: BTreeMap<String, u64>,
    workspaces: BTreeMap<String, String>,
    current: BTreeMap<String, Value>,
    automatic: BTreeMap<String, Value>,
}
impl Evidence {
    fn new(memory: &Value) -> Self {
        let automatic = memory["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["kind"] != "message")
            .map(|s| (id(&s["origin"], "eventId"), s.clone()))
            .collect();
        Self {
            automatic,
            ..Self::default()
        }
    }
    fn touch(&mut self, project: &str) -> Result<()> {
        let r = self
            .projects
            .get_mut(project)
            .ok_or_else(|| invalid("PROJECT_NOT_FOUND"))?;
        *r = r
            .checked_add(1)
            .filter(|n| *n <= MAX_SAFE)
            .ok_or_else(|| invalid("DECISION_REVISION_OVERFLOW"))?;
        Ok(())
    }
    fn consume(&mut self, e: &Value) -> Result<()> {
        let p = &e["payload"];
        match e["type"].as_str().unwrap() {
            "project.created" => {
                self.projects.insert(id(&p["project"], "projectId"), 1);
            }
            "project.workspace.linked" => {
                self.workspaces
                    .insert(id(p, "workspaceId"), id(p, "projectId"));
                self.touch(string(&p["projectId"])?)?;
            }
            "project.workspace.unlinked" => {
                self.workspaces.remove(&id(p, "workspaceId"));
                self.touch(string(&p["projectId"])?)?;
            }
            "workspace.removed" => {
                if let Some(project) = self.workspaces.remove(&id(p, "workspaceId")) {
                    self.touch(&project)?;
                }
            }
            "project.source.created" => {
                self.current
                    .insert(id(&p["source"], "sourceId"), p["source"].clone());
                self.touch(string(&p["source"]["projectId"])?)?;
            }
            "project.source.edited" => {
                let s = self
                    .current
                    .get_mut(&id(p, "sourceId"))
                    .ok_or_else(|| invalid("SOURCE_NOT_FOUND"))?;
                s["title"] = p["title"].clone();
                s["text"] = p["text"].clone();
                s["updatedAt"] = e["recordedAt"].clone();
                s["revision"] = json!(s["revision"].as_u64().unwrap() + 1);
                self.touch(string(&p["projectId"])?)?;
            }
            _ => (),
        }
        if let Some(s) = self.automatic.remove(&id(e, "id")) {
            self.touch(string(&s["projectId"])?)?;
            self.current.insert(id(&s, "sourceId"), s);
        }
        Ok(())
    }
    fn freshness(&self, project: &str, refs: &Value) -> Option<&'static str> {
        for r in refs.as_array().unwrap() {
            let Some(s) = self.current.get(&id(r, "sourceId")) else {
                return Some("SOURCE_NOT_FOUND");
            };
            if s["projectId"] != project {
                return Some("SOURCE_PROJECT_MISMATCH");
            }
            if s["revision"] != r["revision"] {
                return Some("STALE_SOURCE_REVISION");
            }
        }
        None
    }
}
#[derive(Default)]
struct State {
    revision: u64,
    manifests: BTreeMap<String, u64>,
    decisions: BTreeMap<String, Value>,
    history: BTreeMap<String, Vec<Value>>,
    cited: BTreeMap<(String, u64), Value>,
    requests: BTreeMap<String, Value>,
    facts: BTreeMap<String, Value>,
    receipts: BTreeMap<String, Value>,
    evidence: Evidence,
}
impl State {
    fn render(&self) -> Value {
        let projects: Vec<_> = self.evidence.projects.keys().map(|project| {
            let ds: Vec<_> = self.decisions.values().filter(|d| d["projectId"] == *project).collect();
            json!({"projectId":project,"revision":self.manifests.get(project).copied().unwrap_or(0),"decisionCount":ds.len(),"acceptedCount":ds.iter().filter(|d|d["status"]=="accepted").count(),"proposedCount":ds.iter().filter(|d|d["status"]=="proposed").count()})
        }).collect();
        let histories: Vec<_> = self
            .history
            .iter()
            .map(|(decision, entries)| json!({"decisionId":decision,"entries":entries}))
            .collect();
        json!({"schemaVersion":4,"revision":self.revision,"projects":projects,"decisions":self.decisions.values().collect::<Vec<_>>(),"histories":histories,"sources":self.cited.values().collect::<Vec<_>>()})
    }
    fn target(request: &Value) -> String {
        if request["action"] == ACTIONS[0] {
            derived(request["commandId"].as_str().unwrap())
        } else {
            id(&request["payload"], "decisionId")
        }
    }
    fn applied_reason(action: &str) -> &'static str {
        match action {
            "project_decision_propose" => "PROJECT_DECISION_PROPOSED",
            "project_decision_accept" => "PROJECT_DECISION_ACCEPTED",
            "project_decision_reject" => "PROJECT_DECISION_REJECTED",
            _ => "PROJECT_DECISION_SUPERSEDED",
        }
    }
    fn outcome(&self, request: &Value, at: &str) -> (&'static str, &'static str) {
        if request["expiresAt"].as_str().unwrap() <= at {
            return ("expired", "COMMAND_EXPIRED");
        }
        let p = &request["payload"];
        let project = id(p, "projectId");
        let Some(source_revision) = self.evidence.projects.get(&project) else {
            return ("rejected", "PROJECT_NOT_FOUND");
        };
        let action = request["action"].as_str().unwrap();
        if action == ACTIONS[0] {
            if json!(source_revision) != p["expectedProjectRevision"] {
                return ("rejected", "STALE_PROJECT_REVISION");
            }
            if let Some(reason) = self.evidence.freshness(&project, &p["sources"]) {
                return ("rejected", reason);
            }
            if self.decisions.contains_key(&Self::target(request)) {
                return ("rejected", "DECISION_ALREADY_EXISTS");
            }
        } else {
            let Some(d) = self
                .decisions
                .get(&id(p, "decisionId"))
                .filter(|d| d["projectId"] == project)
            else {
                return ("rejected", "DECISION_NOT_FOUND");
            };
            if d["revision"] != p["expectedDecisionRevision"] {
                return ("rejected", "STALE_DECISION_REVISION");
            }
            if action == ACTIONS[3] {
                if d["status"] != "accepted" {
                    return ("rejected", "DECISION_NOT_ACCEPTED");
                }
                let Some(replacement) = self
                    .decisions
                    .get(&id(p, "replacementDecisionId"))
                    .filter(|d| d["projectId"] == project)
                else {
                    return ("rejected", "REPLACEMENT_NOT_FOUND");
                };
                if replacement["revision"] != p["expectedReplacementRevision"] {
                    return ("rejected", "STALE_REPLACEMENT_REVISION");
                }
                if replacement["status"] != "proposed" {
                    return ("rejected", "REPLACEMENT_NOT_PROPOSED");
                }
                if let Some(reason) = self.evidence.freshness(&project, &replacement["sources"]) {
                    return ("rejected", reason);
                }
            } else {
                if d["status"] != "proposed" {
                    return ("rejected", "DECISION_NOT_PROPOSED");
                }
                if action == ACTIONS[1] {
                    if let Some(reason) = self.evidence.freshness(&project, &d["sources"]) {
                        return ("rejected", reason);
                    }
                }
            }
        }
        ("applied", Self::applied_reason(action))
    }
    fn fact(&self, request: &Value, at: &str) -> Value {
        let p = &request["payload"];
        let cmd = &request["commandId"];
        let action = request["action"].as_str().unwrap();
        if action == ACTIONS[0] {
            let sources: Vec<_> = p["sources"].as_array().unwrap().iter().map(|r| {
                let source = &self.evidence.current[&id(r,"sourceId")];
                json!({"sourceId":r["sourceId"],"revision":r["revision"],"origin":source["origin"]})
            }).collect();
            let mut d = json!({"decisionId":Self::target(request),"projectId":p["projectId"],"status":"proposed","revision":1,"createdAt":at,"updatedAt":at,"sourceCount":sources.len(),"supersedesDecisionId":null,"supersededByDecisionId":null,"actorId":"local-owner","sources":sources});
            for (key, _) in BODY {
                d[*key] = p[*key].clone();
            }
            event(
                "decision.proposed",
                json!({"commandId":cmd,"decision":d}),
                at,
            )
        } else {
            let mut payload = p.clone();
            payload["commandId"] = cmd.clone();
            event(
                match action {
                    "project_decision_accept" => "decision.accepted",
                    "project_decision_reject" => "decision.rejected",
                    _ => "decision.superseded",
                },
                payload,
                at,
            )
        }
    }
    fn receipt(&self, request: &Value, outcome: &str, reason: &str, at: &str) -> Value {
        let p = &request["payload"];
        let project = id(p, "projectId");
        let decision = Self::target(request);
        let replacement = p
            .get("replacementDecisionId")
            .cloned()
            .unwrap_or(Value::Null);
        let revision = |key: &str| {
            self.decisions
                .get(key)
                .filter(|d| d["projectId"] == project)
                .map(|d| d["revision"].clone())
                .unwrap_or(json!(0))
        };
        json!({"schemaVersion":4,"commandId":request["commandId"],"action":request["action"],"outcome":outcome,"reasonCode":reason,"completedAt":at,"projectId":project,"projectRevision":self.manifests.get(&project).copied().unwrap_or(0),"decisionId":decision,"decisionRevision":revision(&decision),"replacementDecisionId":replacement,"replacementRevision":replacement.as_str().map(revision).unwrap_or(json!(0))})
    }
    fn terminal(&self, request: &Value, at: &str) -> Value {
        if let Some(fact_receipt) = self.facts.get(&id(request, "commandId")) {
            let mut receipt = fact_receipt.clone();
            receipt["completedAt"] = json!(at);
            receipt
        } else {
            let (outcome, reason) = self.outcome(request, at);
            self.receipt(request, outcome, reason, at)
        }
    }
    fn transition(&mut self, decision: &str, status: &str, at: &str) {
        let d = self.decisions.get_mut(decision).unwrap();
        d["revision"] = json!(d["revision"].as_u64().unwrap() + 1);
        d["status"] = json!(status);
        d["updatedAt"] = json!(at);
        self.history.get_mut(decision).unwrap().push(
            json!({"status":status,"revision":d["revision"],"actorId":"local-owner","at":at}),
        );
    }
    fn apply_fact(&mut self, e: &Value) -> Result<()> {
        let cmd = id(&e["payload"], "commandId");
        let request = self
            .requests
            .get(&cmd)
            .ok_or_else(|| invalid("DECISION_FACT_WITHOUT_REQUEST"))?
            .clone();
        require(
            !self.facts.contains_key(&cmd) && !self.receipts.contains_key(&cmd),
            "DUPLICATE_DECISION_FACT",
        )?;
        let at = string(&e["recordedAt"])?;
        require(
            self.outcome(&request, at).0 == "applied",
            "INVALID_DECISION_FACT_TRANSITION",
        )?;
        let expected = self.fact(&request, at);
        require(
            expected["type"] == e["type"] && expected["payload"] == e["payload"],
            "DECISION_FACT_REQUEST_MISMATCH",
        )?;
        let p = &request["payload"];
        let target = Self::target(&request);
        match request["action"].as_str().unwrap() {
            "project_decision_propose" => {
                let d = &e["payload"]["decision"];
                for r in d["sources"].as_array().unwrap() {
                    let source = self.evidence.current[&id(r, "sourceId")].clone();
                    self.cited
                        .entry((id(r, "sourceId"), r["revision"].as_u64().unwrap()))
                        .or_insert(source);
                }
                self.decisions.insert(target.clone(), d.clone());
                self.history.insert(
                    target,
                    vec![json!({"status":"proposed","revision":1,"actorId":"local-owner","at":at})],
                );
            }
            "project_decision_accept" => self.transition(&target, "accepted", at),
            "project_decision_reject" => self.transition(&target, "rejected", at),
            _ => {
                let replacement = id(p, "replacementDecisionId");
                self.transition(&target, "superseded", at);
                self.transition(&replacement, "accepted", at);
                self.decisions.get_mut(&target).unwrap()["supersededByDecisionId"] =
                    json!(replacement);
                self.decisions.get_mut(&replacement).unwrap()["supersedesDecisionId"] =
                    json!(target);
            }
        }
        let project = id(p, "projectId");
        let manifest = self.manifests.entry(project).or_default();
        *manifest = manifest
            .checked_add(1)
            .filter(|n| *n <= MAX_SAFE)
            .ok_or_else(|| invalid("DECISION_REVISION_OVERFLOW"))?;
        self.revision = self
            .revision
            .checked_add(1)
            .filter(|n| *n <= MAX_SAFE)
            .ok_or_else(|| invalid("DECISION_REVISION_OVERFLOW"))?;
        let receipt = self.receipt(
            &request,
            "applied",
            Self::applied_reason(request["action"].as_str().unwrap()),
            at,
        );
        self.facts.insert(cmd, receipt);
        Ok(())
    }
    fn consume(&mut self, e: &Value) -> Result<()> {
        self.evidence.consume(e)?;
        if e["schemaVersion"] != 4 {
            return Ok(());
        }
        let p = &e["payload"];
        match e["type"].as_str().unwrap() {
            "decision.command.requested" => {
                require(
                    self.requests
                        .insert(id(p, "commandId"), p.clone())
                        .is_none(),
                    "DUPLICATE_DECISION_COMMAND",
                )?;
            }
            "decision.command.receipted" => {
                let r = &p["receipt"];
                let cmd = id(r, "commandId");
                let request = self
                    .requests
                    .get(&cmd)
                    .ok_or_else(|| invalid("DECISION_RECEIPT_WITHOUT_REQUEST"))?;
                require(
                    !self.receipts.contains_key(&cmd),
                    "DUPLICATE_DECISION_RECEIPT",
                )?;
                let expected = self.terminal(request, string(&r["completedAt"])?);
                require(
                    expected["outcome"] != "applied" || self.facts.contains_key(&cmd),
                    "DECISION_RECEIPT_WITHOUT_FACT",
                )?;
                require(expected == *r, "INVALID_DECISION_RECEIPT_TRANSITION")?;
                self.receipts.insert(cmd, r.clone());
            }
            _ => self.apply_fact(e)?,
        }
        Ok(())
    }
}
fn replay_validated(events: &[Value]) -> Result<State> {
    let memory = crate::project_memory::project_validated(events)?;
    let mut state = State {
        evidence: Evidence::new(&memory),
        ..State::default()
    };
    for e in events {
        state.consume(e)?;
    }
    Ok(state)
}
pub fn project_validated(events: &[Value]) -> Result<Value> {
    Ok(replay_validated(events)?.render())
}
fn normalized(events: &[Value], action: &str, payload: &Value, at: &str) -> Result<Value> {
    let names = fields(action)?;
    let map = object(payload)?;
    require(
        map.keys().all(|k| {
            names.contains(&k.as_str())
                || ["commandId", "createdAt", "expiresAt"].contains(&k.as_str())
        }),
        "INVALID_DECISION_FIELDS",
    )?;
    let command = payload
        .get("commandId")
        .cloned()
        .unwrap_or_else(|| json!(format!("cmd:{}", uuid::Uuid::new_v4())));
    identity(&command, "cmd")?;
    let prior = events
        .iter()
        .find(|e| e["type"] == "decision.command.requested" && e["payload"]["commandId"] == command)
        .map(|e| &e["payload"]);
    let created = payload
        .get("createdAt")
        .cloned()
        .or_else(|| prior.map(|p| p["createdAt"].clone()))
        .unwrap_or_else(|| json!(at));
    let created_time = iso(&created)?;
    let expires =
        payload
            .get("expiresAt")
            .cloned()
            .or_else(|| prior.map(|p| p["expiresAt"].clone()))
            .unwrap_or_else(|| {
                json!((created_time + Duration::minutes(5))
                    .to_rfc3339_opts(SecondsFormat::Millis, true))
            });
    let mut input = serde_json::Map::new();
    for name in names {
        let v = payload
            .get(*name)
            .cloned()
            .or_else(|| {
                (action == ACTIONS[0] && ["context", "rationale", "consequences"].contains(name))
                    .then(|| json!(""))
            })
            .ok_or_else(|| invalid("INVALID_DECISION_FIELDS"))?;
        input.insert((*name).into(), v);
    }
    let request = json!({"commandId":command,"action":action,"actorId":"local-owner","createdAt":created,"expiresAt":expires,"payload":input});
    validate_request(&request)?;
    if prior.is_none() {
        require(
            iso(&request["createdAt"])? <= iso(&json!(at))? + Duration::seconds(60),
            "INVALID_DECISION_TIME",
        )?;
    }
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
        additions.push(event("decision.command.requested", request.clone(), &at));
    }
    if !state.facts.contains_key(&cmd) && state.outcome(&request, &at).0 == "applied" {
        additions.push(state.fact(&request, &at));
    }
    let mut combined = events.to_vec();
    combined.extend(additions.iter().cloned());
    let post = replay_validated(&combined)?;
    let receipt = post.terminal(&request, &at);
    additions.push(event(
        "decision.command.receipted",
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
        e["type"] == "decision.command.requested"
            && !state.receipts.contains_key(&id(&e["payload"], "commandId"))
    }) {
        let request = &e["payload"];
        let mut payload = request["payload"].clone();
        for key in ["commandId", "createdAt", "expiresAt"] {
            payload[key] = request[key].clone();
        }
        let (next, _) = apply(&combined, string(&request["action"])?, &payload)?;
        combined.extend(next.iter().cloned());
        additions.extend(next);
    }
    Ok(additions)
}
pub fn metadata(projection: &Value) -> Value {
    json!({"schemaVersion":4,"revision":projection["revision"],"projects":projection["projects"]})
}
fn project_manifest<'a>(projection: &'a Value, project: &Value) -> Result<&'a Value> {
    projection["projects"]
        .as_array()
        .and_then(|ps| ps.iter().find(|p| p["projectId"] == *project))
        .ok_or_else(|| invalid("PROJECT_NOT_FOUND"))
}
fn summary(d: &Value) -> Value {
    let mut result = serde_json::Map::new();
    for key in [
        "decisionId",
        "projectId",
        "title",
        "status",
        "revision",
        "createdAt",
        "updatedAt",
        "sourceCount",
        "supersedesDecisionId",
        "supersededByDecisionId",
    ] {
        result.insert(key.into(), d[key].clone());
    }
    Value::Object(result)
}
pub fn read_list(projection: &Value, payload: &Value) -> Result<Value> {
    let map = object(payload)?;
    require(
        map.contains_key("projectId")
            && map
                .keys()
                .all(|k| ["projectId", "offset", "limit", "query", "status"].contains(&k.as_str())),
        "INVALID_DECISION_FIELDS",
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
    // Reads normalize surrounding whitespace; owner-authored ADR bodies do not.
    require(
        query.encode_utf16().count() <= 160
            && !query
                .chars()
                .any(|c| matches!(c as u32,0..=8|11..=12|14..=31|127)),
        "INVALID_DECISION_TEXT",
    )?;
    let query = query.trim();
    let lower = query.to_lowercase();
    let status = payload
        .get("status")
        .map(string)
        .transpose()?
        .unwrap_or("all");
    require(
        ["all", "proposed", "accepted", "rejected", "superseded"].contains(&status),
        "INVALID_DECISION_STATUS",
    )?;
    let manifest = project_manifest(projection, &payload["projectId"])?;
    let mut matches: Vec<_> = projection["decisions"]
        .as_array()
        .ok_or_else(|| invalid("INVALID_DECISION_PROJECTION"))?
        .iter()
        .filter(|d| {
            d["projectId"] == payload["projectId"]
                && (status == "all" || d["status"] == status)
                && (lower.is_empty()
                    || BODY.iter().any(|(key, _)| {
                        d[*key]
                            .as_str()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&lower)
                    }))
        })
        .collect();
    matches.sort_by(|a, b| {
        b["updatedAt"]
            .as_str()
            .cmp(&a["updatedAt"].as_str())
            .then(a["decisionId"].as_str().cmp(&b["decisionId"].as_str()))
    });
    let total = matches.len();
    let decisions: Vec<_> = matches
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(limit as usize)
        .map(summary)
        .collect();
    Ok(
        json!({"reasonCode":"PROJECT_DECISIONS_READ","projectId":payload["projectId"],"revision":manifest["revision"],"offset":offset,"limit":limit,"query":query,"status":status,"total":total,"decisions":decisions}),
    )
}
pub fn read_detail(projection: &Value, payload: &Value) -> Result<Value> {
    exact(payload, &["projectId", "decisionId"])?;
    identity(&payload["projectId"], "project")?;
    identity(&payload["decisionId"], "decision")?;
    let manifest = project_manifest(projection, &payload["projectId"])?;
    let decision = projection["decisions"]
        .as_array()
        .and_then(|ds| {
            ds.iter().find(|d| {
                d["projectId"] == payload["projectId"] && d["decisionId"] == payload["decisionId"]
            })
        })
        .ok_or_else(|| invalid("DECISION_NOT_FOUND"))?;
    let history = projection["histories"]
        .as_array()
        .and_then(|hs| hs.iter().find(|h| h["decisionId"] == payload["decisionId"]))
        .ok_or_else(|| invalid("INVALID_DECISION_PROJECTION"))?;
    let evidence = projection["sources"]
        .as_array()
        .ok_or_else(|| invalid("INVALID_DECISION_PROJECTION"))?;
    let sources: Vec<_> = decision["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            evidence
                .iter()
                .find(|s| {
                    s["sourceId"] == r["sourceId"]
                        && s["revision"] == r["revision"]
                        && s["projectId"] == payload["projectId"]
                        && s["origin"] == r["origin"]
                })
                .cloned()
                .ok_or_else(|| invalid("INVALID_DECISION_EVIDENCE"))
        })
        .collect::<Result<_>>()?;
    Ok(
        json!({"reasonCode":"PROJECT_DECISION_READ","projectId":payload["projectId"],"revision":manifest["revision"],"decision":decision,"history":history["entries"],"sources":sources}),
    )
}
