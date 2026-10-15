//! Explicit opt-in publication. V5 owns consent and delivery; downloaded data
//! is a bounded disposable cache and never creates local acceptance facts.
use crate::{Error, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

pub const MAX_RESPONSE: usize = 192 * 1024;
pub const MAX_READER_BYTES: usize = 16 * 1024 * 1024;
const MAX_SAFE: u64 = 9_007_199_254_740_991;
const ACTIONS: &[&str] = &[
    "memory_cloud_create",
    "memory_cloud_link",
    "memory_cloud_publish",
    "memory_cloud_sharing",
    "memory_cloud_sync",
];
const BODY: &[(&str, usize)] = &[
    ("title", 160),
    ("context", 10000),
    ("decision", 20000),
    ("rationale", 10000),
    ("consequences", 10000),
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
        .ok_or_else(|| invalid("MEMORY_INVALID_PAYLOAD"))
}
fn exact(v: &Value, keys: &[&str]) -> Result<()> {
    let o = object(v)?;
    require(
        o.len() == keys.len() && o.keys().all(|k| keys.contains(&k.as_str())),
        "MEMORY_INVALID_PAYLOAD",
    )
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| invalid("MEMORY_INVALID_PAYLOAD"))
}
fn number(v: &Value, min: u64, max: u64) -> Result<u64> {
    let n = v
        .as_u64()
        .ok_or_else(|| invalid("MEMORY_INVALID_PAYLOAD"))?;
    require(n >= min && n <= max.min(MAX_SAFE), "MEMORY_INVALID_PAYLOAD")?;
    Ok(n)
}
fn uuid(s: &str) -> Result<()> {
    let u = uuid::Uuid::parse_str(s).map_err(|_| invalid("MEMORY_INVALID_PAYLOAD"))?;
    require(
        u.to_string() == s
            && u.get_variant() == uuid::Variant::RFC4122
            && (1..=8).contains(&u.get_version_num()),
        "MEMORY_INVALID_PAYLOAD",
    )
}
fn identity(v: &Value, prefix: &str) -> Result<()> {
    uuid(
        string(v)?
            .strip_prefix(&format!("{prefix}:"))
            .ok_or_else(|| invalid("MEMORY_INVALID_PAYLOAD"))?,
    )
}
fn opaque(v: &Value) -> Result<()> {
    let s = string(v)?;
    require(
        !s.is_empty() && s.len() <= 256 && s.bytes().all(|b| b.is_ascii_graphic()),
        "MEMORY_INVALID_PAYLOAD",
    )
}
fn iso(v: &Value) -> Result<DateTime<Utc>> {
    let s = string(v)?;
    let t = DateTime::parse_from_rfc3339(s)
        .map_err(|_| invalid("MEMORY_INVALID_TIME"))?
        .with_timezone(&Utc);
    require(
        t.to_rfc3339_opts(SecondsFormat::Millis, true) == s,
        "MEMORY_INVALID_TIME",
    )?;
    Ok(t)
}
fn text(v: &Value, max: usize, empty: bool) -> Result<()> {
    let s = string(v)?;
    require(
        (empty || !s.is_empty())
            && s.trim() == s
            && s.encode_utf16().count() <= max
            && !s
                .chars()
                .any(|c| matches!(c as u32,0..=8|11..=12|14..=31|127)),
        "MEMORY_INVALID_PAYLOAD",
    )
}
fn id(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").into()
}
fn derived(prefix: &str, command: &str) -> String {
    format!(
        "{prefix}:{}",
        command
            .strip_prefix("cmd:")
            .expect("validated command UUID")
    )
}
fn safe_code(v: &Value) -> Result<()> {
    let s = string(v)?;
    require(
        (2..=64).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'),
        "MEMORY_INVALID_PAYLOAD",
    )
}
fn connection(v: &Value) -> Result<()> {
    if v.is_null() {
        return Ok(());
    }
    exact(v, &["cloudUrl", "workspaceId", "hostId"])?;
    require(
        crate::cloud::normalize_url(string(&v["cloudUrl"])?).map_err(Error)? == v["cloudUrl"],
        "MEMORY_INVALID_CONNECTION",
    )?;
    let url = reqwest::Url::parse(string(&v["cloudUrl"])?)
        .map_err(|_| invalid("MEMORY_INVALID_CONNECTION"))?;
    require(
        url.scheme() == "https" || matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "::1")),
        "MEMORY_INVALID_CONNECTION",
    )?;
    opaque(&v["workspaceId"])?;
    opaque(&v["hostId"])
}
pub fn request_hash(operation: &Value) -> Result<String> {
    fn sorted(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut entries: Vec<_> = m.iter().collect();
                entries.sort_unstable_by_key(|(key, _)| *key);
                Value::Object(
                    entries
                        .into_iter()
                        .map(|(k, v)| (k.clone(), sorted(v)))
                        .collect(),
                )
            }
            Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
            _ => v.clone(),
        }
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&sorted(operation))?)
    ))
}
pub fn validate_dto(d: &Value) -> Result<()> {
    exact(
        d,
        &[
            "decisionId",
            "revision",
            "status",
            "title",
            "context",
            "decision",
            "rationale",
            "consequences",
            "acceptedAt",
            "sources",
            "supersedesDecisionId",
        ],
    )?;
    identity(&d["decisionId"], "decision")?;
    number(&d["revision"], 1, MAX_SAFE)?;
    require(d["status"] == "accepted", "MEMORY_INVALID_PAYLOAD")?;
    for (key, max) in BODY {
        text(&d[*key], *max, !["title", "decision"].contains(key))?;
    }
    iso(&d["acceptedAt"])?;
    if !d["supersedesDecisionId"].is_null() {
        identity(&d["supersedesDecisionId"], "decision")?;
        require(
            d["supersedesDecisionId"] != d["decisionId"],
            "MEMORY_INVALID_PAYLOAD",
        )?;
    }
    let refs = d["sources"]
        .as_array()
        .ok_or_else(|| invalid("MEMORY_INVALID_PAYLOAD"))?;
    require((1..=8).contains(&refs.len()), "MEMORY_INVALID_PAYLOAD")?;
    let mut seen = HashSet::new();
    for r in refs {
        exact(r, &["sourceId", "revision"])?;
        identity(&r["sourceId"], "source")?;
        number(&r["revision"], 1, MAX_SAFE)?;
        require(
            seen.insert(string(&r["sourceId"])?),
            "MEMORY_INVALID_PAYLOAD",
        )?;
    }
    Ok(())
}
pub fn validate_operation(o: &Value) -> Result<()> {
    exact(
        o,
        &[
            "schemaVersion",
            "operationId",
            "hostId",
            "createdAt",
            "expiresAt",
            "payload",
        ],
    )?;
    require(o["schemaVersion"] == 1, "MEMORY_INVALID_PAYLOAD")?;
    identity(&o["operationId"], "memory-op")?;
    opaque(&o["hostId"])?;
    let created = iso(&o["createdAt"])?;
    let expires = iso(&o["expiresAt"])?;
    require(
        expires > created
            && expires.signed_duration_since(created).num_milliseconds() <= 7 * 86400000,
        "MEMORY_INVALID_TIME",
    )?;
    let p = &o["payload"];
    match string(&p["type"])? {
        "create_project" => {
            exact(p, &["type", "name"])?;
            text(&p["name"], 80, false)?;
        }
        "link_reader" => {
            exact(p, &["type", "sharedProjectId"])?;
            identity(&p["sharedProjectId"], "shared")?;
        }
        "publish" => {
            exact(
                p,
                &["type", "sharedProjectId", "expectedRevision", "decision"],
            )?;
            identity(&p["sharedProjectId"], "shared")?;
            number(&p["expectedRevision"], 0, MAX_SAFE)?;
            validate_dto(&p["decision"])?;
        }
        "sharing" => {
            exact(
                p,
                &["type", "sharedProjectId", "expectedRevision", "enabled"],
            )?;
            identity(&p["sharedProjectId"], "shared")?;
            number(&p["expectedRevision"], 0, MAX_SAFE)?;
            require(p["enabled"].is_boolean(), "MEMORY_INVALID_PAYLOAD")?;
        }
        _ => return Err(invalid("MEMORY_INVALID_PAYLOAD")),
    }
    require(
        serde_json::to_vec(o)?.len() <= MAX_RESPONSE,
        "MEMORY_INVALID_PAYLOAD",
    )
}
pub fn validate_receipt(r: &Value, operation: &Value) -> Result<()> {
    exact(
        r,
        &[
            "schemaVersion",
            "operationId",
            "operationType",
            "requestHash",
            "outcome",
            "reasonCode",
            "sharedProjectId",
            "revision",
            "role",
            "enabled",
            "completedAt",
        ],
    )?;
    require(
        r["schemaVersion"] == 1
            && r["operationId"] == operation["operationId"]
            && r["operationType"] == operation["payload"]["type"]
            && r["requestHash"] == request_hash(operation)?,
        "MEMORY_RECEIPT_MISMATCH",
    )?;
    require(
        ["applied", "rejected", "expired"].contains(&string(&r["outcome"])?),
        "MEMORY_INVALID_PAYLOAD",
    )?;
    safe_code(&r["reasonCode"])?;
    iso(&r["completedAt"])?;
    number(&r["revision"], 0, MAX_SAFE)?;
    if !r["sharedProjectId"].is_null() {
        identity(&r["sharedProjectId"], "shared")?;
    }
    require(
        r["role"].is_null() || ["publisher", "reader"].contains(&string(&r["role"])?),
        "MEMORY_INVALID_PAYLOAD",
    )?;
    require(
        r["enabled"].is_null() || r["enabled"].is_boolean(),
        "MEMORY_INVALID_PAYLOAD",
    )?;
    let p = &operation["payload"];
    if !r["sharedProjectId"].is_null() {
        let expected = if p["type"] == "create_project" {
            json!(format!(
                "shared:{}",
                string(&operation["operationId"])?
                    .strip_prefix("memory-op:")
                    .unwrap()
            ))
        } else {
            p["sharedProjectId"].clone()
        };
        require(r["sharedProjectId"] == expected, "MEMORY_RECEIPT_MISMATCH")?;
    }
    if r["outcome"] == "applied" {
        require(
            !r["sharedProjectId"].is_null() && r["enabled"].is_boolean(),
            "MEMORY_RECEIPT_MISMATCH",
        )?;
        let reason = match string(&p["type"])? {
            "create_project" => "MEMORY_PROJECT_CREATED",
            "link_reader" => "MEMORY_READER_LINKED",
            "publish" => "MEMORY_PUBLISHED",
            _ => "MEMORY_SHARING_UPDATED",
        };
        require(
            r["reasonCode"] == reason
                && if p["type"] == "link_reader" {
                    ["publisher", "reader"].contains(&string(&r["role"])?)
                } else {
                    r["role"] == "publisher"
                },
            "MEMORY_RECEIPT_MISMATCH",
        )?;
        if p["type"] == "create_project" {
            require(
                r["revision"] == 0 && r["enabled"] == true,
                "MEMORY_RECEIPT_MISMATCH",
            )?;
        }
        if p["type"] == "publish" {
            require(
                r["revision"].as_u64()
                    == p["expectedRevision"]
                        .as_u64()
                        .and_then(|n| n.checked_add(1))
                    && r["enabled"] == true,
                "MEMORY_RECEIPT_MISMATCH",
            )?;
        }
        if p["type"] == "sharing" {
            let rev = number(&r["revision"], 0, MAX_SAFE)?;
            let wanted = number(&p["expectedRevision"], 0, MAX_SAFE)?;
            require(
                (rev == wanted || rev == wanted + 1) && r["enabled"] == p["enabled"],
                "MEMORY_RECEIPT_MISMATCH",
            )?;
        }
    } else if r["outcome"] == "expired" {
        require(
            r["reasonCode"] == "MEMORY_OPERATION_EXPIRED"
                && iso(&r["completedAt"])? >= iso(&operation["expiresAt"])?,
            "MEMORY_RECEIPT_MISMATCH",
        )?;
    }
    Ok(())
}
pub fn validate_manifest(
    m: &Value,
    shared: &Value,
    offset: u64,
    expected: Option<u64>,
) -> Result<()> {
    exact(
        m,
        &[
            "schemaVersion",
            "sharedProjectId",
            "revision",
            "enabled",
            "offset",
            "limit",
            "total",
            "decisions",
        ],
    )?;
    require(
        m["schemaVersion"] == 1
            && m["sharedProjectId"] == *shared
            && m["enabled"] == true
            && m["offset"] == offset
            && m["limit"] == 20,
        "MEMORY_PROTOCOL_INVALID",
    )?;
    let rev = number(&m["revision"], 0, MAX_SAFE)?;
    if let Some(e) = expected {
        require(rev == e, "MEMORY_CONFLICT")?;
    }
    let total = number(&m["total"], 0, 64)?;
    require(offset <= total || offset == 0, "MEMORY_PROTOCOL_INVALID")?;
    let rows = m["decisions"]
        .as_array()
        .ok_or_else(|| invalid("MEMORY_PROTOCOL_INVALID"))?;
    require(
        rows.len() as u64 == total.saturating_sub(offset).min(20),
        "MEMORY_PROTOCOL_INVALID",
    )?;
    let mut previous = None;
    for d in rows {
        exact(d, &["decisionId", "revision", "title", "publishedAt"])?;
        identity(&d["decisionId"], "decision")?;
        number(&d["revision"], 1, MAX_SAFE)?;
        text(&d["title"], 160, false)?;
        iso(&d["publishedAt"])?;
        let key = string(&d["decisionId"])?;
        require(previous.is_none_or(|p| p < key), "MEMORY_PROTOCOL_INVALID")?;
        previous = Some(key);
    }
    Ok(())
}
pub fn validate_exact(v: &Value, shared: &Value, decision: &Value, revision: u64) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "sharedProjectId",
            "manifestRevision",
            "current",
            "publishedAt",
            "decision",
        ],
    )?;
    require(
        v["schemaVersion"] == 1
            && v["sharedProjectId"] == *shared
            && v["decision"]["decisionId"] == *decision
            && v["decision"]["revision"] == revision
            && v["current"].is_boolean(),
        "MEMORY_PROTOCOL_INVALID",
    )?;
    number(&v["manifestRevision"], 0, MAX_SAFE)?;
    iso(&v["publishedAt"])?;
    validate_dto(&v["decision"])
}
fn fields(action: &str) -> Result<&'static [&'static str]> {
    match action {
        "memory_cloud_create" => Ok(&["projectId", "expectedProjectRevision", "name"]),
        "memory_cloud_link" => Ok(&["projectId", "expectedProjectRevision", "sharedProjectId"]),
        "memory_cloud_publish" => Ok(&[
            "projectId",
            "decisionId",
            "expectedDecisionRevision",
            "expectedSharedRevision",
            "consent",
        ]),
        "memory_cloud_sharing" => Ok(&["projectId", "expectedSharedRevision", "enabled"]),
        "memory_cloud_sync" => Ok(&["projectId"]),
        _ => Err(invalid("MEMORY_UNKNOWN_ACTION")),
    }
}
fn validate_input(action: &str, p: &Value) -> Result<()> {
    exact(p, fields(action)?)?;
    identity(&p["projectId"], "project")?;
    match action {
        "memory_cloud_create" => {
            number(&p["expectedProjectRevision"], 1, MAX_SAFE)?;
            text(&p["name"], 80, false)?;
        }
        "memory_cloud_link" => {
            number(&p["expectedProjectRevision"], 1, MAX_SAFE)?;
            identity(&p["sharedProjectId"], "shared")?;
        }
        "memory_cloud_publish" => {
            identity(&p["decisionId"], "decision")?;
            number(&p["expectedDecisionRevision"], 1, MAX_SAFE)?;
            number(&p["expectedSharedRevision"], 0, MAX_SAFE)?;
            require(p["consent"] == true, "MEMORY_CONSENT_REQUIRED")?;
        }
        "memory_cloud_sharing" => {
            number(&p["expectedSharedRevision"], 0, MAX_SAFE)?;
            require(p["enabled"].is_boolean(), "MEMORY_INVALID_PAYLOAD")?;
        }
        _ => (),
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
            "connection",
        ],
    )?;
    identity(&r["commandId"], "cmd")?;
    require(r["actorId"] == "local-owner", "MEMORY_INVALID_ACTOR")?;
    validate_input(string(&r["action"])?, &r["payload"])?;
    connection(&r["connection"])?;
    let c = iso(&r["createdAt"])?;
    let e = iso(&r["expiresAt"])?;
    require(
        e > c && e.signed_duration_since(c).num_milliseconds() <= 300000,
        "MEMORY_INVALID_TIME",
    )
}
fn event(kind: &str, payload: Value, at: &str) -> Value {
    json!({"id":uuid::Uuid::new_v4().to_string(),"schemaVersion":5,"type":kind,"recordedAt":at,"payload":payload})
}
pub fn validate_event(e: &Value) -> Result<()> {
    exact(e, &["id", "schemaVersion", "type", "recordedAt", "payload"])?;
    uuid(string(&e["id"])?)?;
    require(e["schemaVersion"] == 5, "MEMORY_INVALID_VERSION")?;
    iso(&e["recordedAt"])?;
    let p = &e["payload"];
    match string(&e["type"])? {
        "shared_memory.command.requested" => {
            validate_request(p)?;
            require(
                iso(&p["createdAt"])? <= iso(&e["recordedAt"])? + Duration::seconds(60),
                "MEMORY_INVALID_TIME",
            )?;
        }
        "shared_memory.command.receipted" => {
            exact(p, &["receipt"])?;
            let r = &p["receipt"];
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
                    "operationId",
                    "revision",
                ],
            )?;
            require(
                r["schemaVersion"] == 5 && ACTIONS.contains(&string(&r["action"])?),
                "MEMORY_INVALID_PAYLOAD",
            )?;
            identity(&r["commandId"], "cmd")?;
            identity(&r["projectId"], "project")?;
            if !r["operationId"].is_null() {
                identity(&r["operationId"], "memory-op")?;
            }
            require(
                ["applied", "rejected", "expired"].contains(&string(&r["outcome"])?),
                "MEMORY_INVALID_PAYLOAD",
            )?;
            safe_code(&r["reasonCode"])?;
            number(&r["revision"], 0, MAX_SAFE)?;
            iso(&r["completedAt"])?;
            require(r["completedAt"] == e["recordedAt"], "MEMORY_INVALID_TIME")?;
        }
        "shared_memory.operation.queued" => {
            exact(p, &["commandId", "projectId", "operation"])?;
            identity(&p["commandId"], "cmd")?;
            identity(&p["projectId"], "project")?;
            validate_operation(&p["operation"])?;
            require(
                p["operation"]["operationId"] == derived("memory-op", string(&p["commandId"])?)
                    && p["operation"]["createdAt"] == e["recordedAt"],
                "MEMORY_INVALID_PAYLOAD",
            )?;
        }
        "shared_memory.sync.queued" => {
            exact(p, &["commandId", "projectId"])?;
            identity(&p["commandId"], "cmd")?;
            identity(&p["projectId"], "project")?;
        }
        "shared_memory.operation.attempted" => {
            exact(p, &["projectId", "operationId", "requestHash"])?;
            identity(&p["projectId"], "project")?;
            identity(&p["operationId"], "memory-op")?;
            text(&p["requestHash"], 64, false)?;
        }
        "shared_memory.operation.receipted" => {
            exact(p, &["projectId", "operationId", "receipt"])?;
            identity(&p["projectId"], "project")?;
            identity(&p["operationId"], "memory-op")?;
            object(&p["receipt"])?;
        }
        "shared_memory.operation.cancelled" => {
            exact(p, &["projectId", "operationId", "reasonCode"])?;
            identity(&p["projectId"], "project")?;
            identity(&p["operationId"], "memory-op")?;
            require(
                ["STALE_DECISION_REVISION", "MEMORY_OPERATION_EXPIRED"]
                    .contains(&string(&p["reasonCode"])?),
                "MEMORY_INVALID_PAYLOAD",
            )?;
        }
        "shared_memory.sync.failed" => {
            exact(p, &["projectId", "connection", "reasonCode"])?;
            identity(&p["projectId"], "project")?;
            connection(&p["connection"])?;
            require(!p["connection"].is_null(), "MEMORY_INVALID_CONNECTION")?;
            safe_code(&p["reasonCode"])?;
        }
        "shared_memory.manifest.observed" => {
            exact(
                p,
                &["projectId", "connection", "sharedProjectId", "revision"],
            )?;
            identity(&p["projectId"], "project")?;
            identity(&p["sharedProjectId"], "shared")?;
            connection(&p["connection"])?;
            number(&p["revision"], 0, MAX_SAFE)?;
        }
        _ => return Err(invalid("MEMORY_UNKNOWN_EVENT")),
    }
    Ok(())
}

#[derive(Default)]
struct Local {
    projects: BTreeMap<String, u64>,
    links: BTreeMap<String, String>,
    automatic: BTreeMap<String, String>,
    decisions: BTreeMap<String, Value>,
}
impl Local {
    fn new(memory: &Value) -> Self {
        Self {
            automatic: memory["sources"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|s| s["kind"] != "message")
                .map(|s| (id(&s["origin"], "eventId"), id(s, "projectId")))
                .collect(),
            ..Self::default()
        }
    }
    fn touch(&mut self, p: &str) {
        *self.projects.get_mut(p).expect("validated V3 project") += 1;
    }
    fn consume(&mut self, e: &Value) {
        let p = &e["payload"];
        match e["type"].as_str().unwrap() {
            "project.created" => {
                self.projects.insert(id(&p["project"], "projectId"), 1);
            }
            "project.workspace.linked" => {
                self.links.insert(id(p, "workspaceId"), id(p, "projectId"));
                self.touch(&id(p, "projectId"));
            }
            "project.workspace.unlinked" => {
                self.links.remove(&id(p, "workspaceId"));
                self.touch(&id(p, "projectId"));
            }
            "workspace.removed" => {
                if let Some(project) = self.links.remove(&id(p, "workspaceId")) {
                    self.touch(&project);
                }
            }
            "project.source.created" => self.touch(&id(&p["source"], "projectId")),
            "project.source.edited" => self.touch(&id(p, "projectId")),
            "decision.proposed" => {
                let mut d = p["decision"].clone();
                d["acceptedAt"] = Value::Null;
                self.decisions.insert(id(&d, "decisionId"), d);
            }
            "decision.accepted" | "decision.rejected" | "decision.superseded" => {
                let d = self
                    .decisions
                    .get_mut(&id(p, "decisionId"))
                    .expect("validated V4 decision");
                d["revision"] = json!(d["revision"].as_u64().unwrap() + 1);
                d["status"] = json!(match e["type"].as_str().unwrap() {
                    "decision.accepted" => "accepted",
                    "decision.rejected" => "rejected",
                    _ => "superseded",
                });
                if e["type"] == "decision.accepted" {
                    d["acceptedAt"] = e["recordedAt"].clone();
                }
                if e["type"] == "decision.superseded" {
                    let replacement = self
                        .decisions
                        .get_mut(&id(p, "replacementDecisionId"))
                        .unwrap();
                    replacement["revision"] = json!(replacement["revision"].as_u64().unwrap() + 1);
                    replacement["status"] = json!("accepted");
                    replacement["acceptedAt"] = e["recordedAt"].clone();
                    replacement["supersedesDecisionId"] = p["decisionId"].clone();
                }
            }
            _ => (),
        }
        if let Some(project) = self.automatic.remove(&id(e, "id")) {
            self.touch(&project);
        }
    }
    fn dto(&self, project: &str, decision: &str, revision: u64) -> Result<Value> {
        let d = self
            .decisions
            .get(decision)
            .filter(|d| d["projectId"] == project)
            .ok_or_else(|| invalid("DECISION_NOT_FOUND"))?;
        require(d["revision"] == revision, "STALE_DECISION_REVISION")?;
        require(d["status"] == "accepted", "DECISION_NOT_ACCEPTED")?;
        dto(d, &d["acceptedAt"])
    }
}
fn dto(d: &Value, at: &Value) -> Result<Value> {
    let sources: Vec<_> = d["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| json!({"sourceId":s["sourceId"],"revision":s["revision"]}))
        .collect();
    let mut out = json!({"decisionId":d["decisionId"],"revision":d["revision"],"status":"accepted","acceptedAt":at,"sources":sources,"supersedesDecisionId":d["supersedesDecisionId"]});
    for (key, _) in BODY {
        out[*key] = d[*key].clone();
    }
    validate_dto(&out)?;
    Ok(out)
}
pub fn publication_dto(
    projection: &Value,
    project: &str,
    decision: &str,
    revision: u64,
) -> Result<Value> {
    let d = projection["decisions"]
        .as_array()
        .and_then(|ds| {
            ds.iter()
                .find(|d| d["projectId"] == project && d["decisionId"] == decision)
        })
        .ok_or_else(|| invalid("DECISION_NOT_FOUND"))?;
    require(d["revision"] == revision, "STALE_DECISION_REVISION")?;
    require(d["status"] == "accepted", "DECISION_NOT_ACCEPTED")?;
    let at = projection["histories"]
        .as_array()
        .and_then(|hs| hs.iter().find(|h| h["decisionId"] == decision))
        .and_then(|h| h["entries"].as_array())
        .and_then(|hs| {
            hs.iter()
                .find(|h| h["status"] == "accepted" && h["revision"] == revision)
        })
        .ok_or_else(|| invalid("MEMORY_INVALID_LOCAL_DECISION"))?;
    dto(d, &at["at"])
}
#[derive(Default)]
struct State {
    revision: u64,
    local: Local,
    mappings: BTreeMap<String, Value>,
    operations: BTreeMap<String, Value>,
    requests: BTreeMap<String, Value>,
    facts: BTreeMap<String, Value>,
    receipts: BTreeMap<String, Value>,
}
impl State {
    fn touch(&mut self) -> Result<()> {
        self.revision = self
            .revision
            .checked_add(1)
            .filter(|n| *n <= MAX_SAFE)
            .ok_or_else(|| invalid("MEMORY_REVISION_OVERFLOW"))?;
        Ok(())
    }
    fn pending(&self, project: &str) -> Option<&Value> {
        self.operations.values().find(|o| {
            o["projectId"] == project
                && ["queued", "attempted"].contains(&o["state"].as_str().unwrap_or(""))
        })
    }
    fn outcome(&self, r: &Value, at: &str) -> (&'static str, &'static str) {
        if r["expiresAt"].as_str().unwrap() <= at {
            return ("expired", "COMMAND_EXPIRED");
        }
        let p = &r["payload"];
        let project = id(p, "projectId");
        let action = r["action"].as_str().unwrap();
        let Some(revision) = self.local.projects.get(&project) else {
            return ("rejected", "PROJECT_NOT_FOUND");
        };
        if r["connection"].is_null() {
            return ("rejected", "CLOUD_UNPAIRED");
        }
        let mapping = self.mappings.get(&project);
        let setup = [ACTIONS[0], ACTIONS[1]].contains(&action);
        let changed = mapping.is_some_and(|m| m["connection"] != r["connection"]);
        if setup {
            if json!(revision) != p["expectedProjectRevision"] {
                return ("rejected", "STALE_PROJECT_REVISION");
            }
            if mapping.is_some_and(|m| !m["role"].is_null()) && !changed {
                return ("rejected", "MEMORY_ALREADY_LINKED");
            }
            let shared = if action == ACTIONS[0] {
                json!(derived("shared", string(&r["commandId"]).unwrap()))
            } else {
                p["sharedProjectId"].clone()
            };
            if self.mappings.values().any(|m| {
                m["projectId"] != project
                    && m["connection"] == r["connection"]
                    && m["sharedProjectId"] == shared
            }) {
                return ("rejected", "MEMORY_ALREADY_LINKED");
            }
        } else {
            let Some(m) = mapping else {
                return ("rejected", "MEMORY_NOT_LINKED");
            };
            if changed {
                return ("rejected", "MEMORY_CONNECTION_CHANGED");
            }
            if m["role"].is_null() && action != ACTIONS[4] {
                return ("rejected", "MEMORY_LINK_PENDING");
            }
            if action == ACTIONS[2] || action == ACTIONS[3] {
                if m["role"] != "publisher" {
                    return ("rejected", "MEMORY_PUBLISHER_REQUIRED");
                }
                if m["sharedRevision"] != p["expectedSharedRevision"] {
                    return ("rejected", "MEMORY_STALE_REVISION");
                }
            }
            if action == ACTIONS[2] {
                if m["enabled"] != true {
                    return ("rejected", "MEMORY_SHARING_DISABLED");
                }
                if let Err(e) = self.local.dto(
                    &project,
                    &id(p, "decisionId"),
                    p["expectedDecisionRevision"].as_u64().unwrap(),
                ) {
                    return (
                        "rejected",
                        match e.0.as_str() {
                            "DECISION_NOT_FOUND" => "DECISION_NOT_FOUND",
                            "DECISION_NOT_ACCEPTED" => "DECISION_NOT_ACCEPTED",
                            _ => "STALE_DECISION_REVISION",
                        },
                    );
                }
            }
        }
        if action != ACTIONS[4] {
            if self.pending(&project).is_some() && !(setup && changed) {
                return ("rejected", "MEMORY_OPERATION_PENDING");
            }
            if self
                .operations
                .values()
                .filter(|o| ["queued", "attempted"].contains(&o["state"].as_str().unwrap()))
                .count()
                >= 32
                && !(setup && changed)
            {
                return ("rejected", "MEMORY_QUEUE_FULL");
            }
        }
        (
            "applied",
            if action == ACTIONS[4] {
                "MEMORY_SYNC_QUEUED"
            } else {
                "MEMORY_OPERATION_QUEUED"
            },
        )
    }
    fn fact(&self, r: &Value, at: &str) -> Result<Value> {
        let p = &r["payload"];
        let project = id(p, "projectId");
        let cmd = id(r, "commandId");
        let action = r["action"].as_str().unwrap();
        if action == ACTIONS[4] {
            return Ok(event(
                "shared_memory.sync.queued",
                json!({"commandId":cmd,"projectId":project}),
                at,
            ));
        }
        let mapping = self.mappings.get(&project);
        let payload = match action {
            "memory_cloud_create" => json!({"type":"create_project","name":p["name"]}),
            "memory_cloud_link" => {
                json!({"type":"link_reader","sharedProjectId":p["sharedProjectId"]})
            }
            "memory_cloud_publish" => {
                json!({"type":"publish","sharedProjectId":mapping.unwrap()["sharedProjectId"],"expectedRevision":p["expectedSharedRevision"],"decision":self.local.dto(&project,&id(p,"decisionId"),p["expectedDecisionRevision"].as_u64().unwrap())?})
            }
            _ => {
                json!({"type":"sharing","sharedProjectId":mapping.unwrap()["sharedProjectId"],"expectedRevision":p["expectedSharedRevision"],"enabled":p["enabled"]})
            }
        };
        let operation = json!({"schemaVersion":1,"operationId":derived("memory-op",&cmd),"hostId":r["connection"]["hostId"],"createdAt":at,"expiresAt":(iso(&json!(at))?+Duration::days(7)).to_rfc3339_opts(SecondsFormat::Millis,true),"payload":payload});
        validate_operation(&operation)?;
        Ok(event(
            "shared_memory.operation.queued",
            json!({"commandId":cmd,"projectId":project,"operation":operation}),
            at,
        ))
    }
    fn receipt(&self, r: &Value, outcome: &str, reason: &str, at: &str) -> Value {
        json!({"schemaVersion":5,"commandId":r["commandId"],"action":r["action"],"outcome":outcome,"reasonCode":reason,"completedAt":at,"projectId":r["payload"]["projectId"],"operationId":if r["action"]==ACTIONS[4]{Value::Null}else{json!(derived("memory-op",r["commandId"].as_str().unwrap()))},"revision":self.revision})
    }
    fn terminal(&self, r: &Value, at: &str) -> Value {
        if let Some(f) = self.facts.get(&id(r, "commandId")) {
            let mut f = f.clone();
            f["completedAt"] = json!(at);
            f
        } else {
            let (o, c) = self.outcome(r, at);
            self.receipt(r, o, c, at)
        }
    }
    fn queued(&mut self, e: &Value) -> Result<()> {
        let p = &e["payload"];
        let cmd = id(p, "commandId");
        let r = self
            .requests
            .get(&cmd)
            .ok_or_else(|| invalid("MEMORY_FACT_WITHOUT_REQUEST"))?
            .clone();
        require(
            !self.facts.contains_key(&cmd) && !self.receipts.contains_key(&cmd),
            "MEMORY_DUPLICATE_FACT",
        )?;
        let at = string(&e["recordedAt"])?;
        require(
            self.outcome(&r, at).0 == "applied",
            "MEMORY_INVALID_TRANSITION",
        )?;
        let expected = self.fact(&r, at)?;
        require(
            expected["type"] == e["type"] && expected["payload"] == *p,
            "MEMORY_FACT_REQUEST_MISMATCH",
        )?;
        let project = id(p, "projectId");
        if e["type"] == "shared_memory.operation.queued" {
            let op = &p["operation"];
            if ["create_project", "link_reader"].contains(&op["payload"]["type"].as_str().unwrap())
            {
                if let Some(old) = self
                    .pending(&project)
                    .map(|o| id(&o["operation"], "operationId"))
                {
                    let old = self.operations.get_mut(&old).unwrap();
                    old["state"] = json!("unknown");
                    old["lastErrorCode"] = json!("MEMORY_CONNECTION_CHANGED");
                }
                let shared = if op["payload"]["type"] == "create_project" {
                    json!(derived("shared", &cmd))
                } else {
                    op["payload"]["sharedProjectId"].clone()
                };
                self.mappings.insert(project.clone(),json!({"projectId":project,"connection":r["connection"],"sharedProjectId":shared,"role":null,"enabled":null,"sharedRevision":0,"lastSyncAt":null,"lastErrorCode":null,"lastPublishedDecisionId":null,"lastPublishedRevision":0}));
            }
            let previous_enabled = self
                .mappings
                .get(&project)
                .map(|m| m["enabled"].clone())
                .unwrap_or(Value::Null);
            self.operations.insert(id(op,"operationId"),json!({"projectId":project,"connection":r["connection"],"operation":op,"state":"queued","attemptedAt":null,"receipt":null,"lastErrorCode":null,"previousEnabled":previous_enabled}));
        }
        if let Some(m) = self.mappings.get_mut(&project) {
            m["lastErrorCode"] = Value::Null;
        }
        self.touch()?;
        let reason = if e["type"] == "shared_memory.sync.queued" {
            "MEMORY_SYNC_QUEUED"
        } else {
            "MEMORY_OPERATION_QUEUED"
        };
        let receipt = self.receipt(&r, "applied", reason, at);
        self.facts.insert(cmd, receipt);
        Ok(())
    }
    fn operation_mut(&mut self, p: &Value) -> Result<&mut Value> {
        let o = self
            .operations
            .get_mut(&id(p, "operationId"))
            .filter(|o| o["projectId"] == p["projectId"])
            .ok_or_else(|| invalid("MEMORY_OPERATION_NOT_FOUND"))?;
        require(
            ["queued", "attempted"].contains(&string(&o["state"])?),
            "MEMORY_OPERATION_TERMINAL",
        )?;
        Ok(o)
    }
    fn consume(&mut self, e: &Value) -> Result<()> {
        self.local.consume(e);
        if e["schemaVersion"] != 5 {
            return Ok(());
        }
        let p = &e["payload"];
        let at = string(&e["recordedAt"])?;
        match string(&e["type"])? {
            "shared_memory.command.requested" => {
                require(
                    self.requests
                        .insert(id(p, "commandId"), p.clone())
                        .is_none(),
                    "MEMORY_DUPLICATE_COMMAND",
                )?;
            }
            "shared_memory.command.receipted" => {
                let r = &p["receipt"];
                let cmd = id(r, "commandId");
                let request = self
                    .requests
                    .get(&cmd)
                    .ok_or_else(|| invalid("MEMORY_RECEIPT_WITHOUT_REQUEST"))?;
                require(
                    !self.receipts.contains_key(&cmd),
                    "MEMORY_DUPLICATE_RECEIPT",
                )?;
                let expected = self.terminal(request, at);
                require(
                    expected["outcome"] != "applied" || self.facts.contains_key(&cmd),
                    "MEMORY_RECEIPT_WITHOUT_FACT",
                )?;
                require(expected == *r, "MEMORY_RECEIPT_MISMATCH")?;
                self.receipts.insert(cmd, r.clone());
            }
            "shared_memory.operation.queued" | "shared_memory.sync.queued" => self.queued(e)?,
            "shared_memory.operation.attempted" => {
                let existing = self
                    .operations
                    .get(&id(p, "operationId"))
                    .ok_or_else(|| invalid("MEMORY_OPERATION_NOT_FOUND"))?;
                require(existing["state"] == "queued", "MEMORY_DUPLICATE_ATTEMPT")?;
                let op = &existing["operation"];
                require(
                    p["requestHash"] == request_hash(op)? && op["expiresAt"].as_str().unwrap() > at,
                    "MEMORY_INVALID_ATTEMPT",
                )?;
                if op["payload"]["type"] == "publish" {
                    self.local.dto(
                        &id(p, "projectId"),
                        &id(&op["payload"]["decision"], "decisionId"),
                        op["payload"]["decision"]["revision"].as_u64().unwrap(),
                    )?;
                }
                let o = self.operation_mut(p)?;
                o["state"] = json!("attempted");
                o["attemptedAt"] = json!(at);
                self.touch()?;
            }
            "shared_memory.operation.cancelled" => {
                let existing = self
                    .operations
                    .get(&id(p, "operationId"))
                    .ok_or_else(|| invalid("MEMORY_OPERATION_NOT_FOUND"))?;
                require(
                    existing["state"] == "queued",
                    "MEMORY_CANNOT_CANCEL_UNCERTAIN",
                )?;
                let op = &existing["operation"];
                let expected = if op["expiresAt"].as_str().unwrap() <= at {
                    "MEMORY_OPERATION_EXPIRED"
                } else {
                    require(
                        op["payload"]["type"] == "publish"
                            && self
                                .local
                                .dto(
                                    &id(p, "projectId"),
                                    &id(&op["payload"]["decision"], "decisionId"),
                                    op["payload"]["decision"]["revision"].as_u64().unwrap(),
                                )
                                .is_err(),
                        "MEMORY_INVALID_CANCELLATION",
                    )?;
                    "STALE_DECISION_REVISION"
                };
                require(p["reasonCode"] == expected, "MEMORY_INVALID_CANCELLATION")?;
                let o = self.operation_mut(p)?;
                o["state"] = json!("cancelled");
                o["lastErrorCode"] = p["reasonCode"].clone();
                self.mappings.get_mut(&id(p, "projectId")).unwrap()["lastErrorCode"] =
                    p["reasonCode"].clone();
                self.touch()?;
            }
            "shared_memory.operation.receipted" => {
                let existing = self
                    .operations
                    .get(&id(p, "operationId"))
                    .ok_or_else(|| invalid("MEMORY_OPERATION_NOT_FOUND"))?;
                require(
                    existing["state"] == "attempted",
                    "MEMORY_RECEIPT_WITHOUT_ATTEMPT",
                )?;
                let op = existing["operation"].clone();
                let bound = existing["connection"].clone();
                validate_receipt(&p["receipt"], &op)?;
                if op["payload"]["type"] == "sharing" && p["receipt"]["outcome"] == "applied" {
                    let expected = op["payload"]["expectedRevision"].as_u64().unwrap()
                        + u64::from(existing["previousEnabled"] != op["payload"]["enabled"]);
                    require(
                        p["receipt"]["revision"] == expected,
                        "MEMORY_RECEIPT_MISMATCH",
                    )?;
                }
                let r = &p["receipt"];
                let o = self.operation_mut(p)?;
                o["state"] = r["outcome"].clone();
                o["receipt"] = r.clone();
                o["lastErrorCode"] = if r["outcome"] == "applied" {
                    Value::Null
                } else {
                    r["reasonCode"].clone()
                };
                let m = self
                    .mappings
                    .get_mut(&id(p, "projectId"))
                    .ok_or_else(|| invalid("MEMORY_NOT_LINKED"))?;
                require(m["connection"] == bound, "MEMORY_CONNECTION_CHANGED")?;
                if r["outcome"] == "applied" {
                    m["role"] = r["role"].clone();
                    if r["revision"].as_u64() >= m["sharedRevision"].as_u64() {
                        m["enabled"] = r["enabled"].clone();
                        m["sharedRevision"] = r["revision"].clone();
                    }
                    m["lastErrorCode"] = Value::Null;
                    if op["payload"]["type"] == "publish" {
                        m["lastPublishedDecisionId"] =
                            op["payload"]["decision"]["decisionId"].clone();
                        m["lastPublishedRevision"] = op["payload"]["decision"]["revision"].clone();
                    }
                } else {
                    m["lastErrorCode"] = r["reasonCode"].clone();
                    if !r["sharedProjectId"].is_null()
                        && !r["role"].is_null()
                        && r["revision"].as_u64() >= m["sharedRevision"].as_u64()
                    {
                        m["sharedRevision"] = r["revision"].clone();
                        m["enabled"] = r["enabled"].clone();
                    }
                }
                m["lastSyncAt"] = json!(at);
                self.touch()?;
            }
            "shared_memory.sync.failed" => {
                let m = self
                    .mappings
                    .get_mut(&id(p, "projectId"))
                    .ok_or_else(|| invalid("MEMORY_NOT_LINKED"))?;
                require(
                    m["connection"] == p["connection"],
                    "MEMORY_CONNECTION_CHANGED",
                )?;
                m["lastErrorCode"] = p["reasonCode"].clone();
                self.touch()?;
            }
            "shared_memory.manifest.observed" => {
                let m = self
                    .mappings
                    .get_mut(&id(p, "projectId"))
                    .ok_or_else(|| invalid("MEMORY_NOT_LINKED"))?;
                require(
                    m["connection"] == p["connection"]
                        && m["sharedProjectId"] == p["sharedProjectId"]
                        && !m["role"].is_null()
                        && p["revision"].as_u64() >= m["sharedRevision"].as_u64(),
                    "MEMORY_INVALID_MANIFEST_SCOPE",
                )?;
                m["sharedRevision"] = p["revision"].clone();
                m["enabled"] = json!(true);
                m["lastSyncAt"] = json!(at);
                m["lastErrorCode"] = Value::Null;
                self.touch()?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    fn render(&self) -> Value {
        let projects:Vec<_>=self.local.projects.keys().map(|project|{let m=self.mappings.get(project);let pending=self.pending(project);let pending=pending.map(|o|json!({"operationId":o["operation"]["operationId"],"type":o["operation"]["payload"]["type"],"state":o["state"],"decisionId":o["operation"]["payload"]["decision"]["decisionId"],"decisionRevision":o["operation"]["payload"]["decision"]["revision"].as_u64().unwrap_or(0)}));let get=|key:&str|m.map(|m|m[key].clone()).unwrap_or(Value::Null);let diverged=m.is_some_and(|m|m["lastPublishedDecisionId"].as_str().is_some_and(|id|self.local.decisions.get(id).is_none_or(|d|d["status"]!="accepted"||d["revision"]!=m["lastPublishedRevision"])));
        json!({"projectId":project,"sharedProjectId":get("sharedProjectId"),"role":get("role"),"enabled":get("enabled"),"sharedRevision":m.map(|m|m["sharedRevision"].clone()).unwrap_or(json!(0)),"state":if m.is_none(){"unlinked"}else if pending.as_ref().is_some_and(|o|o["state"]=="queued"){"queued"}else if pending.is_some(){"pending"}else if !get("lastErrorCode").is_null(){"error"}else{"ready"},"lastSyncAt":get("lastSyncAt"),"lastErrorCode":get("lastErrorCode"),"pendingOperation":pending,"verifiedAt":null,"stale":true,"offline":true,"diverged":diverged})}).collect();
        json!({"schemaVersion":5,"revision":self.revision,"projects":projects,"mappings":self.mappings.values().collect::<Vec<_>>(),"operations":self.operations.values().collect::<Vec<_>>()})
    }
}
fn replay(events: &[Value]) -> Result<State> {
    let memory = crate::project_memory::project_validated(events)?;
    crate::decisions::project_validated(events)?;
    let mut state = State {
        local: Local::new(&memory),
        ..State::default()
    };
    for e in events {
        state.consume(e)?;
    }
    Ok(state)
}
pub fn project_validated(events: &[Value]) -> Result<Value> {
    Ok(replay(events)?.render())
}
fn normalized(events: &[Value], action: &str, p: &Value, c: &Value, at: &str) -> Result<Value> {
    let names = fields(action)?;
    let map = object(p)?;
    require(
        map.keys().all(|k| {
            names.contains(&k.as_str())
                || ["commandId", "createdAt", "expiresAt"].contains(&k.as_str())
        }) && names.iter().all(|k| map.contains_key(*k)),
        "MEMORY_INVALID_PAYLOAD",
    )?;
    let command = p
        .get("commandId")
        .cloned()
        .unwrap_or_else(|| json!(format!("cmd:{}", uuid::Uuid::new_v4())));
    identity(&command, "cmd")?;
    let prior = events
        .iter()
        .find(|e| {
            e["type"] == "shared_memory.command.requested" && e["payload"]["commandId"] == command
        })
        .map(|e| &e["payload"]);
    let created = p
        .get("createdAt")
        .cloned()
        .or_else(|| prior.map(|p| p["createdAt"].clone()))
        .unwrap_or_else(|| json!(at));
    let created_time = iso(&created)?;
    let expires =
        p.get("expiresAt")
            .cloned()
            .or_else(|| prior.map(|p| p["expiresAt"].clone()))
            .unwrap_or_else(|| {
                json!((created_time + Duration::minutes(5))
                    .to_rfc3339_opts(SecondsFormat::Millis, true))
            });
    let input: serde_json::Map<_, _> = names.iter().map(|k| ((*k).into(), p[*k].clone())).collect();
    let r = json!({"commandId":command,"action":action,"actorId":"local-owner","createdAt":created,"expiresAt":expires,"payload":input,"connection":prior.map(|r|r["connection"].clone()).unwrap_or_else(||c.clone())});
    validate_request(&r)?;
    if prior.is_none() {
        require(
            created_time <= iso(&json!(at))? + Duration::seconds(60),
            "MEMORY_INVALID_TIME",
        )?;
    }
    Ok(r)
}
pub fn apply(events: &[Value], action: &str, p: &Value, c: &Value) -> Result<(Vec<Value>, Value)> {
    crate::domain::project_legacy_events(events)?;
    let state = replay(events)?;
    let at = crate::now();
    let r = normalized(events, action, p, c, &at)?;
    let cmd = id(&r, "commandId");
    if let Some(prior) = state.requests.get(&cmd) {
        require(*prior == r, "IDEMPOTENCY_KEY_REUSED")?;
        if let Some(receipt) = state.receipts.get(&cmd) {
            return Ok((vec![], receipt.clone()));
        }
    }
    let mut add = Vec::new();
    if !state.requests.contains_key(&cmd) {
        add.push(event("shared_memory.command.requested", r.clone(), &at));
    }
    if !state.facts.contains_key(&cmd) && state.outcome(&r, &at).0 == "applied" {
        add.push(state.fact(&r, &at)?);
    }
    let mut combined = events.to_vec();
    combined.extend(add.iter().cloned());
    let receipt = replay(&combined)?.terminal(&r, &at);
    add.push(event(
        "shared_memory.command.receipted",
        json!({"receipt":receipt}),
        &at,
    ));
    combined = events.to_vec();
    combined.extend(add.iter().cloned());
    crate::domain::project_events(&combined)?;
    Ok((add, receipt))
}
pub fn recover(events: &[Value]) -> Result<Vec<Value>> {
    crate::domain::project_legacy_events(events)?;
    let state = replay(events)?;
    let mut combined = events.to_vec();
    let mut add = Vec::new();
    for e in events.iter().filter(|e| {
        e["type"] == "shared_memory.command.requested"
            && !state.receipts.contains_key(&id(&e["payload"], "commandId"))
    }) {
        let r = &e["payload"];
        let mut p = r["payload"].clone();
        for k in ["commandId", "createdAt", "expiresAt"] {
            p[k] = r[k].clone();
        }
        let (next, _) = apply(&combined, string(&r["action"])?, &p, &r["connection"])?;
        combined.extend(next.iter().cloned());
        add.extend(next);
    }
    Ok(add)
}

pub fn prepare_delivery(events: &[Value], project: &str, c: &Value) -> Result<(Vec<Value>, Value)> {
    crate::domain::project_legacy_events(events)?;
    let state = replay(events)?;
    let Some(o) = state.pending(project) else {
        return Ok((vec![], Value::Null));
    };
    require(o["connection"] == *c, "MEMORY_CONNECTION_CHANGED")?;
    let op = &o["operation"];
    if o["state"] == "attempted" {
        return Ok((vec![], op.clone()));
    }
    let at = crate::now();
    let reason = if op["expiresAt"].as_str().unwrap() <= at.as_str() {
        Some("MEMORY_OPERATION_EXPIRED")
    } else if op["payload"]["type"] == "publish"
        && state
            .local
            .dto(
                project,
                &id(&op["payload"]["decision"], "decisionId"),
                op["payload"]["decision"]["revision"].as_u64().unwrap(),
            )
            .is_err()
    {
        Some("STALE_DECISION_REVISION")
    } else {
        None
    };
    let payload = json!({"projectId":project,"operationId":op["operationId"]});
    if let Some(reason) = reason {
        let mut payload = payload;
        payload["reasonCode"] = json!(reason);
        Ok((
            vec![event("shared_memory.operation.cancelled", payload, &at)],
            Value::Null,
        ))
    } else {
        let mut payload = payload;
        payload["requestHash"] = json!(request_hash(op)?);
        Ok((
            vec![event("shared_memory.operation.attempted", payload, &at)],
            op.clone(),
        ))
    }
}
pub fn acknowledge(project: &str, operation: &Value, receipt: &Value) -> Result<Value> {
    validate_receipt(receipt, operation)?;
    Ok(event(
        "shared_memory.operation.receipted",
        json!({"projectId":project,"operationId":operation["operationId"],"receipt":receipt}),
        &crate::now(),
    ))
}
pub fn failure(project: &str, c: &Value, code: &str) -> Value {
    event(
        "shared_memory.sync.failed",
        json!({"projectId":project,"connection":c,"reasonCode":code}),
        &crate::now(),
    )
}
pub fn observed(project: &str, c: &Value, shared: &Value, revision: &Value) -> Value {
    event(
        "shared_memory.manifest.observed",
        json!({"projectId":project,"connection":c,"sharedProjectId":shared,"revision":revision}),
        &crate::now(),
    )
}

pub fn empty_reader() -> Value {
    json!({"schemaVersion":1,"revision":0,"projects":[]})
}
fn mapping<'a>(projection: &'a Value, project: &str) -> Option<&'a Value> {
    projection["mappings"]
        .as_array()
        .and_then(|ms| ms.iter().find(|m| m["projectId"] == project))
}
fn cached<'a>(reader: &'a Value, project: &str) -> Option<&'a Value> {
    reader["projects"]
        .as_array()
        .and_then(|ps| ps.iter().find(|p| p["projectId"] == project))
}
fn validate_heads(heads: &Value) -> Result<()> {
    let rows = heads
        .as_array()
        .ok_or_else(|| invalid("MEMORY_CACHE_INVALID"))?;
    require(rows.len() <= 64, "MEMORY_CACHE_INVALID")?;
    let mut previous = None;
    for d in rows {
        exact(d, &["decisionId", "revision", "title", "publishedAt"])?;
        identity(&d["decisionId"], "decision")?;
        number(&d["revision"], 1, MAX_SAFE)?;
        text(&d["title"], 160, false)?;
        iso(&d["publishedAt"])?;
        let key = string(&d["decisionId"])?;
        require(previous.is_none_or(|p| p < key), "MEMORY_CACHE_INVALID")?;
        previous = Some(key);
    }
    Ok(())
}
pub fn validate_reader(reader: &Value) -> Result<()> {
    exact(reader, &["schemaVersion", "revision", "projects"])?;
    require(reader["schemaVersion"] == 1, "MEMORY_CACHE_INVALID")?;
    number(&reader["revision"], 0, MAX_SAFE)?;
    let projects = reader["projects"]
        .as_array()
        .ok_or_else(|| invalid("MEMORY_CACHE_INVALID"))?;
    let mut seen = HashSet::new();
    for p in projects {
        exact(
            p,
            &[
                "projectId",
                "connection",
                "sharedProjectId",
                "verifiedAt",
                "manifest",
                "records",
            ],
        )?;
        identity(&p["projectId"], "project")?;
        require(
            seen.insert(string(&p["projectId"])?),
            "MEMORY_CACHE_INVALID",
        )?;
        connection(&p["connection"])?;
        require(!p["connection"].is_null(), "MEMORY_CACHE_INVALID")?;
        identity(&p["sharedProjectId"], "shared")?;
        iso(&p["verifiedAt"])?;
        exact(&p["manifest"], &["revision", "decisions"])?;
        number(&p["manifest"]["revision"], 0, MAX_SAFE)?;
        validate_heads(&p["manifest"]["decisions"])?;
        let heads = p["manifest"]["decisions"].as_array().unwrap();
        let records = p["records"]
            .as_array()
            .ok_or_else(|| invalid("MEMORY_CACHE_INVALID"))?;
        require(records.len() <= 128, "MEMORY_CACHE_INVALID")?;
        let mut versions = HashSet::new();
        for r in records {
            validate_exact(
                r,
                &p["sharedProjectId"],
                &r["decision"]["decisionId"],
                number(&r["decision"]["revision"], 1, MAX_SAFE)?,
            )?;
            require(
                versions.insert((
                    id(&r["decision"], "decisionId"),
                    r["decision"]["revision"].as_u64().unwrap(),
                )) && r["manifestRevision"] == p["manifest"]["revision"],
                "MEMORY_CACHE_INVALID",
            )?;
            let head = heads.iter().find(|h| {
                h["decisionId"] == r["decision"]["decisionId"]
                    && h["revision"] == r["decision"]["revision"]
            });
            require(r["current"] == head.is_some(), "MEMORY_CACHE_INVALID")?;
            if let Some(h) = head {
                require(
                    h["publishedAt"] == r["publishedAt"] && h["title"] == r["decision"]["title"],
                    "MEMORY_CACHE_INVALID",
                )?;
            }
        }
        for h in heads {
            require(
                records.iter().any(|r| {
                    r["current"] == true
                        && r["decision"]["decisionId"] == h["decisionId"]
                        && r["decision"]["revision"] == h["revision"]
                }),
                "MEMORY_CACHE_INCOMPLETE",
            )?;
        }
    }
    require(
        serde_json::to_vec(reader)?.len() <= MAX_READER_BYTES,
        "MEMORY_CACHE_TOO_LARGE",
    )
}
pub fn prune_reader(reader: &Value, projection: &Value) -> Value {
    let mut next = reader.clone();
    let rows = next["projects"].as_array_mut().unwrap();
    let before = rows.len();
    rows.retain(|p| {
        mapping(projection, &id(p, "projectId")).is_some_and(|m| {
            m["connection"] == p["connection"] && m["sharedProjectId"] == p["sharedProjectId"]
        })
    });
    if before != rows.len() {
        next["revision"] = json!(reader["revision"].as_u64().unwrap() + 1);
    }
    next
}
pub fn merge_reader(
    reader: &Value,
    projection: &Value,
    project: &str,
    c: &Value,
    manifest: &Value,
    records: &[Value],
) -> Result<Value> {
    let m = mapping(projection, project).ok_or_else(|| invalid("MEMORY_NOT_LINKED"))?;
    require(
        m["connection"] == *c && !m["role"].is_null(),
        "MEMORY_CONNECTION_CHANGED",
    )?;
    exact(manifest, &["revision", "decisions"])?;
    validate_heads(&manifest["decisions"])?;
    number(&manifest["revision"], 0, MAX_SAFE)?;
    require(
        manifest["revision"].as_u64() >= m["sharedRevision"].as_u64(),
        "MEMORY_CONFLICT",
    )?;
    let mut versions: BTreeMap<(String, u64), Value> = BTreeMap::new();
    if let Some(old) = cached(reader, project)
        .filter(|p| p["connection"] == *c && p["sharedProjectId"] == m["sharedProjectId"])
    {
        for r in old["records"].as_array().unwrap() {
            versions.insert(
                (
                    id(&r["decision"], "decisionId"),
                    r["decision"]["revision"].as_u64().unwrap(),
                ),
                r.clone(),
            );
        }
    }
    for r in records {
        validate_exact(
            r,
            &m["sharedProjectId"],
            &r["decision"]["decisionId"],
            number(&r["decision"]["revision"], 1, MAX_SAFE)?,
        )?;
        require(
            r["manifestRevision"] == manifest["revision"] && r["current"] == true,
            "MEMORY_CONFLICT",
        )?;
        require(
            manifest["decisions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|head| {
                    head["decisionId"] == r["decision"]["decisionId"]
                        && head["revision"] == r["decision"]["revision"]
                        && head["title"] == r["decision"]["title"]
                        && head["publishedAt"] == r["publishedAt"]
                }),
            "MEMORY_CACHE_INVALID",
        )?;
        let key = (
            id(&r["decision"], "decisionId"),
            r["decision"]["revision"].as_u64().unwrap(),
        );
        if let Some(prior) = versions.get(&key) {
            require(
                prior["decision"] == r["decision"] && prior["publishedAt"] == r["publishedAt"],
                "MEMORY_IMMUTABLE_CONFLICT",
            )?;
        }
        versions.insert(key, r.clone());
    }
    let heads = manifest["decisions"].as_array().unwrap();
    for r in versions.values_mut() {
        r["manifestRevision"] = manifest["revision"].clone();
        r["current"] = json!(heads
            .iter()
            .any(|h| h["decisionId"] == r["decision"]["decisionId"]
                && h["revision"] == r["decision"]["revision"]));
    }
    let mut records: Vec<_> = versions.into_values().collect();
    records.sort_by(|a, b| {
        b["current"]
            .as_bool()
            .cmp(&a["current"].as_bool())
            .then(b["publishedAt"].as_str().cmp(&a["publishedAt"].as_str()))
            .then(
                a["decision"]["decisionId"]
                    .as_str()
                    .cmp(&b["decision"]["decisionId"].as_str()),
            )
    });
    records.truncate(128);
    let project_row = json!({"projectId":project,"connection":c,"sharedProjectId":m["sharedProjectId"],"verifiedAt":crate::now(),"manifest":manifest,"records":records});
    let mut next = prune_reader(reader, projection);
    next["projects"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["projectId"] != project);
    next["projects"].as_array_mut().unwrap().push(project_row);
    next["revision"] = json!(reader["revision"].as_u64().unwrap() + 1);
    // Retain complete current projects. Evict historical versions first, then
    // least-recently verified whole projects to enforce a 16 MiB reader budget.
    let mut encoded_bytes = serde_json::to_vec(&next)?.len();
    while encoded_bytes > MAX_READER_BYTES {
        let rows = next["projects"].as_array_mut().unwrap();
        let current = rows.iter_mut().find(|p| p["projectId"] == project).unwrap();
        let rs = current["records"].as_array_mut().unwrap();
        if let Some(i) = rs.iter().rposition(|r| r["current"] == false) {
            let comma = usize::from(rs.len() > 1);
            let removed = rs.remove(i);
            encoded_bytes =
                encoded_bytes.saturating_sub(serde_json::to_vec(&removed)?.len() + comma);
            continue;
        }
        let oldest = rows
            .iter()
            .enumerate()
            .filter(|(_, p)| p["projectId"] != project)
            .min_by(|(_, a), (_, b)| a["verifiedAt"].as_str().cmp(&b["verifiedAt"].as_str()))
            .map(|(i, _)| i)
            .ok_or_else(|| invalid("MEMORY_CACHE_TOO_LARGE"))?;
        let removed = rows.remove(oldest);
        encoded_bytes = encoded_bytes.saturating_sub(serde_json::to_vec(&removed)?.len() + 1);
    }
    validate_reader(&next)?;
    Ok(next)
}
pub fn metadata(projection: &Value, reader: &Value, c: &Value) -> Value {
    let mut projects = projection["projects"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for p in &mut projects {
        let project = id(p, "projectId");
        let mapping = mapping(projection, &project);
        let cache = cached(reader, &project).filter(|r| {
            mapping.is_some_and(|m| {
                m["connection"] == r["connection"] && m["sharedProjectId"] == r["sharedProjectId"]
            })
        });
        let changed = mapping.is_some_and(|m| m["connection"] != *c);
        let old = cache.is_none_or(|r| {
            iso(&r["verifiedAt"])
                .map(|t| Utc::now().signed_duration_since(t).num_seconds() > 120)
                .unwrap_or(true)
        });
        let stale = changed
            || old
            || p["enabled"] != true
            || !p["lastErrorCode"].is_null()
            || cache.is_some_and(|r| r["manifest"]["revision"] != p["sharedRevision"]);
        p["verifiedAt"] = cache
            .map(|r| r["verifiedAt"].clone())
            .unwrap_or(Value::Null);
        if let Some(r) = cache {
            p["lastSyncAt"] = r["verifiedAt"].clone();
        }
        p["stale"] = json!(stale);
        p["offline"] = json!(
            changed
                || old
                || [
                    "MEMORY_UNAVAILABLE",
                    "CLOUD_UNPAIRED",
                    "MEMORY_SYNC_INTERRUPTED"
                ]
                .contains(&p["lastErrorCode"].as_str().unwrap_or(""))
        );
        if changed {
            p["state"] = json!("connection_changed");
            p["lastErrorCode"] = json!("MEMORY_CONNECTION_CHANGED");
        }
    }
    json!({"schemaVersion":5,"revision":projection["revision"].as_u64().unwrap_or(0)+reader["revision"].as_u64().unwrap_or(0),"projects":projects})
}
fn read_project<'a>(
    projection: &Value,
    reader: &'a Value,
    c: &Value,
    project: &str,
) -> Result<(Value, Option<&'a Value>)> {
    let meta = metadata(projection, reader, c);
    let row = meta["projects"]
        .as_array()
        .and_then(|ps| ps.iter().find(|p| p["projectId"] == project))
        .ok_or_else(|| invalid("PROJECT_NOT_FOUND"))?
        .clone();
    let cache = cached(reader, project).filter(|r| {
        mapping(projection, project).is_some_and(|m| {
            m["connection"] == r["connection"] && m["sharedProjectId"] == r["sharedProjectId"]
        })
    });
    Ok((row, cache))
}
pub fn read_list(projection: &Value, reader: &Value, c: &Value, p: &Value) -> Result<Value> {
    let map = object(p)?;
    require(
        map.contains_key("projectId")
            && map
                .keys()
                .all(|k| ["projectId", "offset", "limit"].contains(&k.as_str())),
        "MEMORY_INVALID_PAYLOAD",
    )?;
    identity(&p["projectId"], "project")?;
    let offset = p
        .get("offset")
        .map(|v| number(v, 0, MAX_SAFE))
        .transpose()?
        .unwrap_or(0);
    let limit = p
        .get("limit")
        .map(|v| number(v, 1, 20))
        .transpose()?
        .unwrap_or(10);
    let (meta, cache) = read_project(projection, reader, c, &id(p, "projectId"))?;
    let heads = cache.map(|r| r["manifest"]["decisions"].as_array().unwrap());
    let total = heads.map(Vec::len).unwrap_or(0);
    let rows: Vec<_> = heads
        .into_iter()
        .flatten()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(limit as usize)
        .cloned()
        .collect();
    let mut out = meta;
    out["reasonCode"] = json!("SHARED_MEMORY_READ");
    out["revision"] = metadata(projection, reader, c)["revision"].clone();
    out["offset"] = json!(offset);
    out["limit"] = json!(limit);
    out["total"] = json!(total);
    out["decisions"] = json!(rows);
    Ok(out)
}
pub fn read_detail(projection: &Value, reader: &Value, c: &Value, p: &Value) -> Result<Value> {
    exact(p, &["projectId", "decisionId", "revision"])?;
    identity(&p["projectId"], "project")?;
    identity(&p["decisionId"], "decision")?;
    number(&p["revision"], 1, MAX_SAFE)?;
    let (meta, cache) = read_project(projection, reader, c, &id(p, "projectId"))?;
    let mut r = cache
        .and_then(|r| r["records"].as_array())
        .and_then(|rs| {
            rs.iter().find(|r| {
                r["decision"]["decisionId"] == p["decisionId"]
                    && r["decision"]["revision"] == p["revision"]
            })
        })
        .ok_or_else(|| invalid("MEMORY_NOT_CACHED"))?
        .clone();
    r["reasonCode"] = json!("SHARED_DECISION_READ");
    r["projectId"] = p["projectId"].clone();
    r["revision"] = metadata(projection, reader, c)["revision"].clone();
    for key in ["verifiedAt", "stale", "offline"] {
        r[key] = meta[key].clone();
    }
    Ok(r)
}

/// At most one bounded download is staged per Host. It becomes visible only
/// after every manifest page and current exact head agrees on one revision.
pub(crate) struct Download {
    pub project: String,
    pub connection: Value,
    pub shared: Value,
    pub revision: Option<u64>,
    pub total: Option<u64>,
    pub heads: Vec<Value>,
    pub records: Vec<Value>,
}
impl Download {
    pub fn new(project: &str, c: &Value, shared: &Value) -> Self {
        Self {
            project: project.into(),
            connection: c.clone(),
            shared: shared.clone(),
            revision: None,
            total: None,
            heads: vec![],
            records: vec![],
        }
    }
    pub fn needs_page(&self) -> bool {
        self.total.is_none_or(|n| self.heads.len() < (n as usize))
    }
    pub fn page(&mut self, page: Value) -> Result<()> {
        validate_manifest(&page, &self.shared, self.heads.len() as u64, self.revision)?;
        if let Some(total) = self.total {
            require(page["total"] == total, "MEMORY_CONFLICT")?;
        }
        self.revision = page["revision"].as_u64();
        self.total = page["total"].as_u64();
        let rows = page["decisions"].as_array().unwrap();
        if let (Some(last), Some(first)) = (self.heads.last(), rows.first()) {
            require(
                string(&last["decisionId"])? < string(&first["decisionId"])?,
                "MEMORY_PROTOCOL_INVALID",
            )?;
        }
        self.heads.extend(rows.iter().cloned());
        Ok(())
    }
    pub fn head(&self) -> Option<&Value> {
        self.heads.get(self.records.len())
    }
    pub fn record(&mut self, record: Value) -> Result<()> {
        let head = self
            .head()
            .ok_or_else(|| invalid("MEMORY_PROTOCOL_INVALID"))?;
        validate_exact(
            &record,
            &self.shared,
            &head["decisionId"],
            head["revision"].as_u64().unwrap(),
        )?;
        require(
            record["manifestRevision"].as_u64() == self.revision
                && record["current"] == true
                && record["publishedAt"] == head["publishedAt"]
                && record["decision"]["title"] == head["title"],
            "MEMORY_CONFLICT",
        )?;
        self.records.push(record);
        Ok(())
    }
    pub fn complete(&self) -> bool {
        !self.needs_page() && self.records.len() == self.heads.len()
    }
    pub fn manifest(&self) -> Value {
        json!({"revision":self.revision.unwrap_or(0),"decisions":self.heads})
    }
}
