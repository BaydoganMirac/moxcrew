//! Consented source-to-ADR jobs. The single writer publishes proposals only.
use crate::{Error, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const VERSION: &str = "codex-adr-v1";
pub const MAX_INPUT_BYTES: usize = 240 * 1024;
const MAX_SAFE: u64 = 9_007_199_254_740_991;
const ACTIONS: [&str; 3] = [
    "project_extraction_configure",
    "project_extraction_enqueue",
    "project_extraction_retry",
];
const STATUSES: [&str; 6] = [
    "queued",
    "running",
    "completed",
    "failed",
    "cancelled",
    "interrupted",
];
const BODY: [(&str, usize); 5] = [
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
fn exact(v: &Value, fields: &[&str]) -> Result<()> {
    require(
        v.as_object().is_some_and(|m| {
            m.len() == fields.len() && m.keys().all(|k| fields.contains(&k.as_str()))
        }),
        "EXTRACTION_INVALID_FIELDS",
    )
}
fn text(v: &Value, max: usize, empty: bool) -> Result<()> {
    require(
        v.as_str().is_some_and(|s| {
            (empty || !s.is_empty())
                && s.trim() == s
                && s.encode_utf16().count() <= max
                && !s
                    .chars()
                    .any(|c| matches!(c as u32,0..=8|11..=12|14..=31|127))
        }),
        "EXTRACTION_INVALID_TEXT",
    )
}
fn number(v: &Value, min: u64, max: u64) -> Result<u64> {
    v.as_u64()
        .filter(|n| *n >= min && *n <= max)
        .ok_or_else(|| invalid("EXTRACTION_INVALID_INTEGER"))
}
fn id(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").into()
}
fn identity(v: &Value, prefix: &str) -> Result<()> {
    let raw = v
        .as_str()
        .and_then(|s| s.strip_prefix(&format!("{prefix}:")))
        .ok_or_else(|| invalid("EXTRACTION_INVALID_ID"))?;
    let u = uuid::Uuid::parse_str(raw).map_err(|_| invalid("EXTRACTION_INVALID_ID"))?;
    require(
        u.to_string() == raw
            && u.get_variant() == uuid::Variant::RFC4122
            && (1..=8).contains(&u.get_version_num()),
        "EXTRACTION_INVALID_ID",
    )
}
fn opaque(v: &Value) -> Result<()> {
    require(
        v.as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        }),
        "EXTRACTION_INVALID_ID",
    )
}
fn iso(v: &Value) -> Result<DateTime<Utc>> {
    let s = v
        .as_str()
        .ok_or_else(|| invalid("EXTRACTION_INVALID_TIME"))?;
    let t = DateTime::parse_from_rfc3339(s)
        .map_err(|_| invalid("EXTRACTION_INVALID_TIME"))?
        .with_timezone(&Utc);
    require(
        t.to_rfc3339_opts(SecondsFormat::Millis, true) == s,
        "EXTRACTION_INVALID_TIME",
    )?;
    Ok(t)
}
fn reason(v: &Value) -> Result<()> {
    require(
        v.as_str().is_some_and(|s| {
            s.starts_with("EXTRACTION_")
                && s.len() <= 64
                && s.bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        }),
        "EXTRACTION_INVALID_REASON",
    )
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn job_id(project: &str, source: &str, rev: u64) -> String {
    format!(
        "extraction:{}",
        hash(&format!("{project}\n{source}\n{rev}\n{VERSION}"))
    )
}
fn valid_job(v: &Value) -> Result<()> {
    require(
        v.as_str()
            .and_then(|s| s.strip_prefix("extraction:"))
            .is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            }),
        "EXTRACTION_INVALID_ID",
    )
}
fn proposal_command(job: &str, attempt: u64, index: u64) -> String {
    let digest = Sha256::digest(format!("{job}\n{attempt}\n{index}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    format!("cmd:{}", uuid::Uuid::from_bytes(bytes))
}
fn decision_id(cmd: &str) -> String {
    format!("decision:{}", cmd.strip_prefix("cmd:").unwrap_or(""))
}
fn fields(action: &str) -> Result<&'static [&'static str]> {
    match action {
        "project_extraction_configure" => {
            Ok(&["projectId", "expectedPolicyRevision", "enabled", "consent"])
        }
        "project_extraction_enqueue" => Ok(&["projectId", "sourceId", "expectedSourceRevision"]),
        "project_extraction_retry" => Ok(&["projectId", "jobId", "expectedAttempt"]),
        _ => Err(invalid("EXTRACTION_UNKNOWN_ACTION")),
    }
}
fn validate_input(action: &str, p: &Value) -> Result<()> {
    exact(p, fields(action)?)?;
    identity(&p["projectId"], "project")?;
    match action {
        "project_extraction_configure" => {
            number(&p["expectedPolicyRevision"], 0, MAX_SAFE)?;
            require(
                p["enabled"].is_boolean() && p["consent"].is_boolean(),
                "EXTRACTION_INVALID_CONSENT",
            )?;
        }
        "project_extraction_enqueue" => {
            identity(&p["sourceId"], "source")?;
            number(&p["expectedSourceRevision"], 1, MAX_SAFE)?;
        }
        _ => {
            valid_job(&p["jobId"])?;
            number(&p["expectedAttempt"], 0, 3)?;
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
    require(r["actorId"] == "local-owner", "EXTRACTION_INVALID_ACTOR")?;
    validate_input(r["action"].as_str().unwrap_or(""), &r["payload"])?;
    let created = iso(&r["createdAt"])?;
    let expires = iso(&r["expiresAt"])?;
    require(
        expires > created && (expires - created).num_milliseconds() <= 300_000,
        "EXTRACTION_INVALID_TIME",
    )
}
fn validate_evidence(value: &Value) -> Result<()> {
    let rows = value
        .as_array()
        .ok_or_else(|| invalid("EXTRACTION_INVALID_EVIDENCE"))?;
    require(rows.len() == 1, "EXTRACTION_INVALID_EVIDENCE")?;
    exact(&rows[0], &["sourceId", "revision", "excerpt"])?;
    identity(&rows[0]["sourceId"], "source")?;
    number(&rows[0]["revision"], 1, MAX_SAFE)?;
    // Excerpts retain exact whitespace; they are never normalized before anchoring.
    require(
        rows[0]["excerpt"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.encode_utf16().count() <= 1000),
        "EXTRACTION_INVALID_EVIDENCE",
    )
}
fn validate_relation(p: &Value) -> Result<()> {
    require(
        ["new", "possible_conflict", "possible_duplicate"]
            .contains(&p["relation"].as_str().unwrap_or("")),
        "EXTRACTION_INVALID_RELATION",
    )?;
    let related = p["relatedDecisionIds"]
        .as_array()
        .ok_or_else(|| invalid("EXTRACTION_INVALID_RELATION"))?;
    require(related.len() <= 8, "EXTRACTION_INVALID_RELATION")?;
    let mut seen = HashSet::new();
    for d in related {
        identity(d, "decision")?;
        require(
            seen.insert(d.as_str().unwrap()),
            "EXTRACTION_INVALID_RELATION",
        )?;
    }
    require(
        (p["relation"] == "new") == related.is_empty(),
        "EXTRACTION_INVALID_RELATION",
    )
}
pub fn validate_event(e: &Value) -> Result<()> {
    exact(e, &["id", "schemaVersion", "type", "recordedAt", "payload"])?;
    identity(
        &json!(format!("event:{}", e["id"].as_str().unwrap_or(""))),
        "event",
    )?;
    require(
        e["schemaVersion"].as_u64() == Some(7),
        "EXTRACTION_INVALID_VERSION",
    )?;
    iso(&e["recordedAt"])?;
    let p = &e["payload"];
    match e["type"].as_str().unwrap_or("") {
        "extraction.command.requested" => {
            validate_request(p)?;
            require(
                iso(&p["createdAt"])? <= iso(&e["recordedAt"])? + Duration::seconds(60),
                "EXTRACTION_INVALID_TIME",
            )?;
        }
        "extraction.command.receipted" => {
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
                    "policyRevision",
                    "jobId",
                    "attempt",
                ],
            )?;
            require(
                r["schemaVersion"] == 7
                    && ACTIONS.contains(&r["action"].as_str().unwrap_or(""))
                    && ["applied", "rejected", "expired"]
                        .contains(&r["outcome"].as_str().unwrap_or("")),
                "EXTRACTION_INVALID_RECEIPT",
            )?;
            identity(&r["commandId"], "cmd")?;
            identity(&r["projectId"], "project")?;
            number(&r["policyRevision"], 0, MAX_SAFE)?;
            number(&r["attempt"], 0, 3)?;
            reason(&r["reasonCode"])?;
            iso(&r["completedAt"])?;
            if !r["jobId"].is_null() {
                valid_job(&r["jobId"])?;
            }
            require(
                r["completedAt"] == e["recordedAt"],
                "EXTRACTION_INVALID_TIME",
            )?;
        }
        "extraction.settings_changed" => {
            exact(
                p,
                &[
                    "commandId",
                    "projectId",
                    "expectedPolicyRevision",
                    "enabled",
                    "consent",
                ],
            )?;
            identity(&p["commandId"], "cmd")?;
            let mut input = p.clone();
            input.as_object_mut().unwrap().remove("commandId");
            validate_input(ACTIONS[0], &input)?;
        }
        "extraction.job_queued" => {
            exact(
                p,
                &[
                    "commandId",
                    "jobId",
                    "projectId",
                    "sourceId",
                    "sourceRevision",
                    "extractorVersion",
                    "policyRevision",
                    "automatic",
                ],
            )?;
            if !p["commandId"].is_null() {
                identity(&p["commandId"], "cmd")?;
            }
            valid_job(&p["jobId"])?;
            identity(&p["projectId"], "project")?;
            identity(&p["sourceId"], "source")?;
            number(&p["sourceRevision"], 1, MAX_SAFE)?;
            number(&p["policyRevision"], 1, MAX_SAFE)?;
            require(
                p["extractorVersion"] == VERSION && p["automatic"].is_boolean(),
                "EXTRACTION_INVALID_JOB",
            )?;
        }
        "extraction.job_retried" => {
            exact(
                p,
                &["commandId", "jobId", "expectedAttempt", "policyRevision"],
            )?;
            identity(&p["commandId"], "cmd")?;
            valid_job(&p["jobId"])?;
            number(&p["expectedAttempt"], 0, 2)?;
            number(&p["policyRevision"], 1, MAX_SAFE)?;
        }
        "extraction.job_started" => {
            exact(p, &["jobId", "attempt", "ownerEpoch"])?;
            valid_job(&p["jobId"])?;
            number(&p["attempt"], 1, 3)?;
            opaque(&p["ownerEpoch"])?;
        }
        "extraction.job_finished" => {
            exact(
                p,
                &[
                    "jobId",
                    "attempt",
                    "ownerEpoch",
                    "status",
                    "reasonCode",
                    "proposalIds",
                    "skippedDuplicates",
                ],
            )?;
            valid_job(&p["jobId"])?;
            number(&p["attempt"], 0, 3)?;
            if !p["ownerEpoch"].is_null() {
                opaque(&p["ownerEpoch"])?;
            }
            require(
                STATUSES[2..].contains(&p["status"].as_str().unwrap_or("")),
                "EXTRACTION_INVALID_STATUS",
            )?;
            reason(&p["reasonCode"])?;
            number(&p["skippedDuplicates"], 0, 2)?;
            let ds = p["proposalIds"]
                .as_array()
                .ok_or_else(|| invalid("EXTRACTION_INVALID_JOB"))?;
            require(ds.len() <= 6, "EXTRACTION_INVALID_JOB")?;
            let mut seen = HashSet::new();
            for d in ds {
                identity(d, "decision")?;
                require(seen.insert(d.as_str().unwrap()), "EXTRACTION_INVALID_JOB")?;
            }
        }
        "extraction.proposal_bound" => {
            exact(
                p,
                &[
                    "jobId",
                    "attempt",
                    "index",
                    "projectId",
                    "sourceId",
                    "sourceRevision",
                    "decisionId",
                    "commandId",
                    "generator",
                    "evidence",
                    "relation",
                    "relatedDecisionIds",
                ],
            )?;
            valid_job(&p["jobId"])?;
            number(&p["attempt"], 1, 3)?;
            number(&p["index"], 0, 1)?;
            identity(&p["projectId"], "project")?;
            identity(&p["sourceId"], "source")?;
            number(&p["sourceRevision"], 1, MAX_SAFE)?;
            identity(&p["decisionId"], "decision")?;
            identity(&p["commandId"], "cmd")?;
            exact(&p["generator"], &["providerId", "extractorVersion"])?;
            require(
                p["generator"]["providerId"] == "codex"
                    && p["generator"]["extractorVersion"] == VERSION,
                "EXTRACTION_INVALID_GENERATOR",
            )?;
            validate_evidence(&p["evidence"])?;
            validate_relation(p)?;
        }
        _ => return Err(invalid("EXTRACTION_UNKNOWN_EVENT")),
    }
    Ok(())
}
fn event(kind: &str, payload: Value, at: &str) -> Value {
    json!({"id":uuid::Uuid::new_v4().to_string(),"schemaVersion":7,"type":kind,"recordedAt":at,"payload":payload})
}
#[derive(Default)]
struct State {
    revision: u64,
    projects: HashSet<String>,
    settings: BTreeMap<String, Value>,
    cutoffs: HashMap<String, usize>,
    sources: BTreeMap<String, Value>,
    source_positions: HashMap<String, usize>,
    automatic: HashMap<String, Value>,
    jobs: Vec<Value>,
    requests: BTreeMap<String, Value>,
    facts: BTreeMap<String, Value>,
    receipts: BTreeMap<String, Value>,
    provenance: Vec<Value>,
    decisions: BTreeMap<String, Value>,
    proposal_receipts: HashSet<String>,
}
impl State {
    fn policy(&self, project: &str) -> Value {
        self.settings.get(project).cloned().unwrap_or_else(||json!({"projectId":project,"policyRevision":0,"enabled":false,"consent":false,"updatedAt":null}))
    }
    fn job(&self, key: &str) -> Result<&Value> {
        self.jobs
            .iter()
            .find(|j| j["jobId"] == key)
            .ok_or_else(|| invalid("EXTRACTION_JOB_NOT_FOUND"))
    }
    fn job_mut(&mut self, key: &str) -> Result<&mut Value> {
        self.jobs
            .iter_mut()
            .find(|j| j["jobId"] == key)
            .ok_or_else(|| invalid("EXTRACTION_JOB_NOT_FOUND"))
    }
    fn nonterminal(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j["status"] == "queued" || j["status"] == "running")
            .count()
    }
    fn stale(&self, j: &Value) -> Option<&'static str> {
        let policy = self.policy(&id(j, "projectId"));
        if policy["enabled"] != true {
            return Some("EXTRACTION_POLICY_DISABLED");
        }
        if policy["policyRevision"] != j["policyRevision"] {
            return Some("EXTRACTION_POLICY_CHANGED");
        }
        if !self.sources.get(&id(j, "sourceId")).is_some_and(|s| {
            s["projectId"] == j["projectId"] && s["revision"] == j["sourceRevision"]
        }) {
            return Some("EXTRACTION_SOURCE_CHANGED");
        }
        None
    }
    fn eligible(&self, project: &str, source: &str, revision: &Value) -> bool {
        self.sources
            .get(source)
            .is_some_and(|s| s["projectId"] == project && s["revision"] == *revision)
    }
    fn outcome(&self, r: &Value, at: &str) -> (&'static str, &'static str) {
        if self.facts.contains_key(&id(r, "commandId")) {
            return ("applied", applied_reason(r["action"].as_str().unwrap()));
        }
        if r["expiresAt"].as_str().unwrap() <= at {
            return ("expired", "EXTRACTION_COMMAND_EXPIRED");
        }
        let p = &r["payload"];
        let project = id(p, "projectId");
        let policy = self.policy(&project);
        if !self.projects.contains(&project) {
            return ("rejected", "EXTRACTION_PROJECT_NOT_FOUND");
        }
        match r["action"].as_str().unwrap() {
            "project_extraction_configure" => {
                if p["expectedPolicyRevision"] != policy["policyRevision"] {
                    return ("rejected", "EXTRACTION_STALE_POLICY");
                }
                if p["enabled"] == true && p["consent"] != true {
                    return ("rejected", "EXTRACTION_CONSENT_REQUIRED");
                }
                ("applied", "EXTRACTION_CONFIGURED")
            }
            "project_extraction_enqueue" => {
                if policy["enabled"] != true {
                    return ("rejected", "EXTRACTION_POLICY_DISABLED");
                }
                if !self.eligible(&project, &id(p, "sourceId"), &p["expectedSourceRevision"]) {
                    return ("rejected", "EXTRACTION_SOURCE_CHANGED");
                }
                let job = job_id(
                    &project,
                    &id(p, "sourceId"),
                    p["expectedSourceRevision"].as_u64().unwrap(),
                );
                if self.jobs.iter().any(|j| j["jobId"] == job) {
                    return ("rejected", "EXTRACTION_JOB_EXISTS");
                }
                if self.nonterminal() >= 32 {
                    return ("rejected", "EXTRACTION_QUEUE_FULL");
                }
                ("applied", "EXTRACTION_QUEUED")
            }
            _ => {
                let Ok(j) = self.job(&id(p, "jobId")) else {
                    return ("rejected", "EXTRACTION_JOB_NOT_FOUND");
                };
                if j["projectId"] != p["projectId"] {
                    return ("rejected", "EXTRACTION_JOB_NOT_FOUND");
                }
                if j["attempt"] != p["expectedAttempt"] {
                    return ("rejected", "EXTRACTION_STALE_ATTEMPT");
                }
                if j["attempt"].as_u64().unwrap() >= 3 {
                    return ("rejected", "EXTRACTION_ATTEMPT_LIMIT");
                }
                if !["failed", "interrupted", "cancelled"].contains(&j["status"].as_str().unwrap())
                {
                    return ("rejected", "EXTRACTION_NOT_RETRYABLE");
                }
                if policy["enabled"] != true {
                    return ("rejected", "EXTRACTION_POLICY_DISABLED");
                }
                if !self.eligible(&project, &id(j, "sourceId"), &j["sourceRevision"]) {
                    return ("rejected", "EXTRACTION_SOURCE_CHANGED");
                }
                if self.nonterminal() >= 32 {
                    return ("rejected", "EXTRACTION_QUEUE_FULL");
                }
                ("applied", "EXTRACTION_RETRIED")
            }
        }
    }
    fn receipt(&self, r: &Value, outcome: &str, reason: &str, at: &str) -> Value {
        let p = &r["payload"];
        let project = id(p, "projectId");
        let job = match r["action"].as_str().unwrap() {
            "project_extraction_enqueue" => json!(job_id(
                &project,
                &id(p, "sourceId"),
                p["expectedSourceRevision"].as_u64().unwrap()
            )),
            "project_extraction_retry" => p["jobId"].clone(),
            _ => Value::Null,
        };
        let attempt = job
            .as_str()
            .and_then(|j| self.job(j).ok())
            .map(|j| j["attempt"].clone())
            .unwrap_or(json!(0));
        json!({"schemaVersion":7,"commandId":r["commandId"],"action":r["action"],"outcome":outcome,"reasonCode":reason,"completedAt":at,"projectId":project,"policyRevision":self.policy(&project)["policyRevision"],"jobId":job,"attempt":attempt})
    }
    fn command_fact(&self, r: &Value, at: &str) -> Value {
        let p = &r["payload"];
        let project = id(p, "projectId");
        match r["action"].as_str().unwrap() {
            "project_extraction_configure" => {
                let mut payload = p.clone();
                payload["commandId"] = r["commandId"].clone();
                event("extraction.settings_changed", payload, at)
            }
            "project_extraction_enqueue" => event(
                "extraction.job_queued",
                self.queue_payload(
                    &project,
                    &id(p, "sourceId"),
                    p["expectedSourceRevision"].as_u64().unwrap(),
                    r["commandId"].clone(),
                    false,
                ),
                at,
            ),
            _ => event(
                "extraction.job_retried",
                json!({"commandId":r["commandId"],"jobId":p["jobId"],"expectedAttempt":p["expectedAttempt"],"policyRevision":self.policy(&project)["policyRevision"]}),
                at,
            ),
        }
    }
    fn queue_payload(
        &self,
        project: &str,
        source: &str,
        revision: u64,
        command: Value,
        automatic: bool,
    ) -> Value {
        json!({"commandId":command,"jobId":job_id(project,source,revision),"projectId":project,"sourceId":source,"sourceRevision":revision,"extractorVersion":VERSION,"policyRevision":self.policy(project)["policyRevision"],"automatic":automatic})
    }
    fn anchor(&self, p: &Value, j: &Value) -> Result<()> {
        validate_evidence(&p["evidence"])?;
        validate_relation(p)?;
        let source = self
            .sources
            .get(&id(j, "sourceId"))
            .ok_or_else(|| invalid("EXTRACTION_SOURCE_CHANGED"))?;
        let e = &p["evidence"][0];
        require(
            e["sourceId"] == j["sourceId"]
                && e["revision"] == j["sourceRevision"]
                && source["text"]
                    .as_str()
                    .unwrap()
                    .contains(e["excerpt"].as_str().unwrap()),
            "EXTRACTION_UNANCHORED_EVIDENCE",
        )?;
        for d in p["relatedDecisionIds"].as_array().unwrap() {
            require(
                self.decisions
                    .get(d.as_str().unwrap())
                    .is_some_and(|d| d["projectId"] == j["projectId"] && d["status"] == "accepted"),
                "EXTRACTION_FOREIGN_DECISION",
            )?;
        }
        Ok(())
    }
    fn consume(&mut self, e: &Value, ordinal: usize) -> Result<()> {
        let p = &e["payload"];
        let kind = e["type"].as_str().unwrap_or("");
        if kind.starts_with("extraction.") || e["schemaVersion"] == 7 {
            validate_event(e)?;
        }
        match kind {
            "project.created" => {
                self.projects.insert(id(&p["project"], "projectId"));
            }
            "project.source.created" => {
                self.sources
                    .insert(id(&p["source"], "sourceId"), p["source"].clone());
                self.source_positions
                    .insert(id(&p["source"], "sourceId"), ordinal);
            }
            "project.source.edited" => {
                if let Some(s) = self.sources.get_mut(&id(p, "sourceId")) {
                    s["title"] = p["title"].clone();
                    s["text"] = p["text"].clone();
                    s["revision"] = json!(s["revision"].as_u64().unwrap() + 1);
                    s["updatedAt"] = e["recordedAt"].clone();
                    self.source_positions.insert(id(p, "sourceId"), ordinal);
                }
            }
            "decision.proposed" => {
                let d = &p["decision"];
                if let Some(bound) = self
                    .provenance
                    .iter()
                    .find(|b| b["commandId"] == p["commandId"])
                {
                    require(
                        bound["decisionId"] == d["decisionId"]
                            && bound["projectId"] == d["projectId"]
                            && d["sources"].as_array().is_some_and(|s| {
                                s.len() == 1
                                    && s[0]["sourceId"] == bound["sourceId"]
                                    && s[0]["revision"] == bound["sourceRevision"]
                            }),
                        "EXTRACTION_PROPOSAL_MISMATCH",
                    )?;
                    let job_id = id(bound, "jobId");
                    let ids = self.job_mut(&job_id)?["proposalIds"]
                        .as_array_mut()
                        .unwrap();
                    require(
                        !ids.contains(&d["decisionId"]),
                        "EXTRACTION_DUPLICATE_PROPOSAL",
                    )?;
                    ids.push(d["decisionId"].clone());
                }
                self.decisions.insert(id(d, "decisionId"), d.clone());
            }
            "decision.accepted" | "decision.rejected" => {
                if let Some(d) = self.decisions.get_mut(&id(p, "decisionId")) {
                    d["status"] = json!(if kind == "decision.accepted" {
                        "accepted"
                    } else {
                        "rejected"
                    });
                    d["revision"] = json!(d["revision"].as_u64().unwrap() + 1);
                }
            }
            "decision.superseded" => {
                if let Some(d) = self.decisions.get_mut(&id(p, "decisionId")) {
                    d["status"] = json!("superseded");
                    d["revision"] = json!(d["revision"].as_u64().unwrap() + 1);
                }
                if let Some(d) = self.decisions.get_mut(&id(p, "replacementDecisionId")) {
                    d["status"] = json!("accepted");
                    d["revision"] = json!(d["revision"].as_u64().unwrap() + 1);
                }
            }
            "decision.command.receipted" if p["receipt"]["outcome"] == "applied" => {
                self.proposal_receipts
                    .insert(id(&p["receipt"], "commandId"));
            }
            _ => {}
        }
        if let Some(s) = self.automatic.remove(&id(e, "id")) {
            self.source_positions.insert(id(&s, "sourceId"), ordinal);
            self.sources.insert(id(&s, "sourceId"), s);
        }
        if e["schemaVersion"] != 7 {
            return Ok(());
        }
        validate_event(e)?;
        if kind == "extraction.command.requested" {
            require(
                self.requests
                    .insert(id(p, "commandId"), p.clone())
                    .is_none(),
                "EXTRACTION_DUPLICATE_COMMAND",
            )?;
            return Ok(());
        }
        if kind == "extraction.command.receipted" {
            let r = &p["receipt"];
            let request = self
                .requests
                .get(&id(r, "commandId"))
                .ok_or_else(|| invalid("EXTRACTION_RECEIPT_WITHOUT_REQUEST"))?;
            let expected = if let Some(f) = self.facts.get(&id(r, "commandId")) {
                let mut f = f.clone();
                f["completedAt"] = r["completedAt"].clone();
                f
            } else {
                let (o, c) = self.outcome(request, r["completedAt"].as_str().unwrap());
                require(o != "applied", "EXTRACTION_RECEIPT_WITHOUT_FACT")?;
                self.receipt(request, o, c, r["completedAt"].as_str().unwrap())
            };
            require(
                expected == *r
                    && self
                        .receipts
                        .insert(id(r, "commandId"), r.clone())
                        .is_none(),
                "EXTRACTION_INVALID_RECEIPT_TRANSITION",
            )?;
            return Ok(());
        }
        let command = p
            .get("commandId")
            .filter(|c| !c.is_null())
            .and_then(Value::as_str);
        let request = command.and_then(|c| self.requests.get(c)).cloned();
        if ["extraction.settings_changed", "extraction.job_retried"].contains(&kind)
            || kind == "extraction.job_queued" && command.is_some()
        {
            let r = request
                .as_ref()
                .ok_or_else(|| invalid("EXTRACTION_FACT_WITHOUT_REQUEST"))?;
            require(
                !self.facts.contains_key(command.unwrap())
                    && !self.receipts.contains_key(command.unwrap())
                    && self.outcome(r, e["recordedAt"].as_str().unwrap()).0 == "applied"
                    && self.command_fact(r, e["recordedAt"].as_str().unwrap())["payload"] == *p,
                "EXTRACTION_INVALID_COMMAND_TRANSITION",
            )?;
        }
        match kind {
            "extraction.settings_changed" => {
                let project = id(p, "projectId");
                let revision = self.policy(&project)["policyRevision"].as_u64().unwrap() + 1;
                require(revision <= MAX_SAFE, "EXTRACTION_REVISION_OVERFLOW")?;
                self.settings.insert(project.clone(),json!({"projectId":project,"policyRevision":revision,"enabled":p["enabled"],"consent":p["consent"],"updatedAt":e["recordedAt"]}));
                self.cutoffs.insert(project, ordinal);
            }
            "extraction.job_queued" => {
                let project = id(p, "projectId");
                let policy = self.policy(&project);
                require(
                    policy["enabled"] == true
                        && policy["policyRevision"] == p["policyRevision"]
                        && self.eligible(&project, &id(p, "sourceId"), &p["sourceRevision"])
                        && self.nonterminal() < 32
                        && !self.jobs.iter().any(|j| j["jobId"] == p["jobId"])
                        && p["jobId"]
                            == job_id(
                                &project,
                                &id(p, "sourceId"),
                                p["sourceRevision"].as_u64().unwrap(),
                            ),
                    "EXTRACTION_INVALID_QUEUE",
                )?;
                require(
                    if p["automatic"] == true {
                        command.is_none()
                            && self.source_positions.get(&id(p, "sourceId"))
                                > self.cutoffs.get(&project)
                    } else {
                        command.is_some()
                    },
                    "EXTRACTION_INVALID_QUEUE_AUTHORITY",
                )?;
                self.jobs.push(json!({"jobId":p["jobId"],"projectId":project,"sourceId":p["sourceId"],"sourceRevision":p["sourceRevision"],"extractorVersion":VERSION,"policyRevision":p["policyRevision"],"status":"queued","attempt":0,"ownerEpoch":null,"createdAt":e["recordedAt"],"updatedAt":e["recordedAt"],"reasonCode":"EXTRACTION_QUEUED","proposalIds":[],"skippedDuplicates":0}));
            }
            "extraction.job_retried" => {
                let j = self.job_mut(&id(p, "jobId"))?;
                j["status"] = json!("queued");
                j["ownerEpoch"] = Value::Null;
                j["policyRevision"] = p["policyRevision"].clone();
                j["updatedAt"] = e["recordedAt"].clone();
                j["reasonCode"] = json!("EXTRACTION_RETRIED");
                j["skippedDuplicates"] = json!(0);
            }
            "extraction.job_started" => {
                let j = self.job(&id(p, "jobId"))?;
                require(
                    j["status"] == "queued"
                        && self.stale(j).is_none()
                        && p["attempt"].as_u64() == j["attempt"].as_u64().map(|n| n + 1)
                        && !self.jobs.iter().any(|j| j["status"] == "running"),
                    "EXTRACTION_INVALID_START",
                )?;
                let j = self.job_mut(&id(p, "jobId"))?;
                j["status"] = json!("running");
                j["attempt"] = p["attempt"].clone();
                j["ownerEpoch"] = p["ownerEpoch"].clone();
                j["updatedAt"] = e["recordedAt"].clone();
                j["reasonCode"] = json!("EXTRACTION_RUNNING");
            }
            "extraction.proposal_bound" => {
                let j = self.job(&id(p, "jobId"))?;
                let cmd = proposal_command(
                    &id(p, "jobId"),
                    p["attempt"].as_u64().unwrap(),
                    p["index"].as_u64().unwrap(),
                );
                require(
                    j["status"] == "running"
                        && j["attempt"] == p["attempt"]
                        && self.stale(j).is_none()
                        && p["projectId"] == j["projectId"]
                        && p["sourceId"] == j["sourceId"]
                        && p["sourceRevision"] == j["sourceRevision"]
                        && p["commandId"] == cmd
                        && p["decisionId"] == decision_id(&cmd)
                        && !self
                            .provenance
                            .iter()
                            .any(|b| b["commandId"] == p["commandId"])
                        && !self.decisions.contains_key(&id(p, "decisionId")),
                    "EXTRACTION_INVALID_PROVENANCE",
                )?;
                self.anchor(p, j)?;
                self.provenance.push(p.clone());
            }
            "extraction.job_finished" => {
                let j = self.job(&id(p, "jobId"))?;
                require(
                    (j["status"] == "running"
                        || j["status"] == "queued" && p["status"] == "cancelled")
                        && j["attempt"] == p["attempt"]
                        && j["ownerEpoch"] == p["ownerEpoch"]
                        && j["proposalIds"] == p["proposalIds"],
                    "EXTRACTION_INVALID_FINISH",
                )?;
                if p["status"] == "completed" {
                    require(
                        j["status"] == "running"
                            && self.stale(j).is_none()
                            && self
                                .provenance
                                .iter()
                                .filter(|b| {
                                    b["jobId"] == j["jobId"] && b["attempt"] == j["attempt"]
                                })
                                .all(|b| {
                                    self.decisions.contains_key(&id(b, "decisionId"))
                                        && self.proposal_receipts.contains(&id(b, "commandId"))
                                }),
                        "EXTRACTION_INVALID_FINISH",
                    )?;
                }
                let j = self.job_mut(&id(p, "jobId"))?;
                for key in ["status", "reasonCode", "skippedDuplicates"] {
                    j[key] = p[key].clone();
                }
                j["updatedAt"] = e["recordedAt"].clone();
            }
            _ => return Err(invalid("EXTRACTION_UNKNOWN_EVENT")),
        }
        self.revision = self
            .revision
            .checked_add(1)
            .filter(|n| *n <= MAX_SAFE)
            .ok_or_else(|| invalid("EXTRACTION_REVISION_OVERFLOW"))?;
        if let Some(r) = request.filter(|_| {
            [
                "extraction.settings_changed",
                "extraction.job_queued",
                "extraction.job_retried",
            ]
            .contains(&kind)
        }) {
            let reason = applied_reason(r["action"].as_str().unwrap());
            let receipt = self.receipt(&r, "applied", reason, e["recordedAt"].as_str().unwrap());
            self.facts.insert(id(&r, "commandId"), receipt);
        }
        Ok(())
    }
    fn render(&self) -> Value {
        let mut projects:Vec<_>=self.projects.iter().map(|p|json!({"projectId":p,"policyRevision":self.policy(p)["policyRevision"],"enabled":self.policy(p)["enabled"]})).collect();
        projects.sort_by(|a, b| a["projectId"].as_str().cmp(&b["projectId"].as_str()));
        json!({"schemaVersion":7,"revision":self.revision,"projects":projects,"settings":self.settings.values().collect::<Vec<_>>(),"jobs":self.jobs,"receipts":self.receipts.values().collect::<Vec<_>>(),"provenance":self.provenance})
    }
}
fn replay(events: &[Value]) -> Result<State> {
    let memory = crate::project_memory::project_validated(events)?;
    let mut state = State::default();
    for source in memory["sources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["kind"] != "message")
    {
        state
            .automatic
            .insert(id(&source["origin"], "eventId"), source.clone());
    }
    for (ordinal, e) in events.iter().enumerate() {
        state.consume(e, ordinal)?;
    }
    Ok(state)
}
pub fn project_validated(events: &[Value]) -> Result<Value> {
    if !events.iter().any(|e| {
        e["schemaVersion"] == 7
            || e["type"]
                .as_str()
                .is_some_and(|s| s.starts_with("extraction."))
    }) {
        return Ok(Value::Null);
    }
    Ok(replay(events)?.render())
}
pub fn metadata(projection: &Value) -> Value {
    if projection.is_null() {
        return Value::Null;
    }
    let projects:Vec<_>=projection["projects"].as_array().into_iter().flatten().map(|p|{
        let jobs:Vec<_>=projection["jobs"].as_array().into_iter().flatten().filter(|j|j["projectId"]==p["projectId"]).collect();let counts:serde_json::Map<String,Value>=STATUSES.iter().map(|s|((*s).into(),json!(jobs.iter().filter(|j|j["status"]==*s).count()))).collect();
        let recent:Vec<_> = jobs.into_iter().rev().take(10).map(|job| {
            let row:serde_json::Map<String,Value> = ["jobId","projectId","sourceId","sourceRevision","extractorVersion","status","attempt","policyRevision","ownerEpoch","createdAt","updatedAt","reasonCode","proposalIds","skippedDuplicates"].into_iter().map(|key|(key.into(),job[key].clone())).collect();
            Value::Object(row)
        }).collect();
        json!({"projectId":p["projectId"],"policyRevision":p["policyRevision"],"enabled":p["enabled"],"counts":counts,"recentJobs":recent})
    }).collect();
    json!({"schemaVersion":7,"revision":projection["revision"],"projects":projects})
}
fn normalized(events: &[Value], action: &str, payload: &Value, at: &str) -> Result<Value> {
    let names = fields(action)?;
    require(
        payload.as_object().is_some_and(|m| {
            m.keys().all(|k| {
                names.contains(&k.as_str())
                    || ["commandId", "createdAt", "expiresAt"].contains(&k.as_str())
            }) && names.iter().all(|n| m.contains_key(*n))
        }),
        "EXTRACTION_INVALID_FIELDS",
    )?;
    let cmd = payload
        .get("commandId")
        .cloned()
        .unwrap_or_else(|| json!(format!("cmd:{}", uuid::Uuid::new_v4())));
    let prior = events
        .iter()
        .find(|e| e["type"] == "extraction.command.requested" && e["payload"]["commandId"] == cmd)
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
    let input: serde_json::Map<String, Value> = names
        .iter()
        .map(|n| ((*n).into(), payload[*n].clone()))
        .collect();
    let request = json!({"commandId":cmd,"action":action,"actorId":"local-owner","createdAt":created,"expiresAt":expires,"payload":input});
    validate_request(&request)?;
    if prior.is_none() {
        require(
            iso(&request["createdAt"])? <= iso(&json!(at))? + Duration::seconds(60),
            "EXTRACTION_INVALID_TIME",
        )?;
    }
    Ok(request)
}
pub fn apply(events: &[Value], action: &str, payload: &Value) -> Result<(Vec<Value>, Value)> {
    crate::domain::project_legacy_events(events)?;
    let state = replay(events)?;
    let at = crate::now();
    let request = normalized(events, action, payload, &at)?;
    let cmd = id(&request, "commandId");
    if let Some(prior) = state.requests.get(&cmd) {
        require(*prior == request, "IDEMPOTENCY_KEY_REUSED")?;
        if let Some(r) = state.receipts.get(&cmd) {
            return Ok((vec![], r.clone()));
        }
    }
    let mut additions = Vec::new();
    if !state.requests.contains_key(&cmd) {
        additions.push(event("extraction.command.requested", request.clone(), &at));
    }
    let (outcome, reason) = state.outcome(&request, &at);
    if outcome == "applied" && !state.facts.contains_key(&cmd) {
        additions.push(state.command_fact(&request, &at));
    }
    let mut combined = events.to_vec();
    combined.extend(additions.iter().cloned());
    let post = replay(&combined)?;
    let r = if let Some(fact) = post.facts.get(&cmd) {
        let mut fact = fact.clone();
        fact["completedAt"] = json!(at);
        fact
    } else {
        post.receipt(&request, outcome, reason, &at)
    };
    additions.push(event(
        "extraction.command.receipted",
        json!({"receipt":r}),
        &at,
    ));
    combined = events.to_vec();
    combined.extend(additions.iter().cloned());
    replay(&combined)?;
    Ok((additions, r))
}
fn finish_fact(job: &Value, status: &str, reason: &str, duplicates: u64, at: &str) -> Value {
    event(
        "extraction.job_finished",
        json!({"jobId":job["jobId"],"attempt":job["attempt"],"ownerEpoch":job["ownerEpoch"],"status":status,"reasonCode":reason,"proposalIds":job["proposalIds"],"skippedDuplicates":duplicates}),
        at,
    )
}
pub fn reconcile(events: &[Value]) -> Result<Vec<Value>> {
    let mut combined = events.to_vec();
    let state = replay(events)?;
    let mut additions = Vec::new();
    let at = crate::now();
    for j in state
        .jobs
        .iter()
        .filter(|j| j["status"] == "queued" || j["status"] == "running")
    {
        if let Some(reason) = state.stale(j) {
            let e = finish_fact(j, "cancelled", reason, 0, &at);
            combined.push(e.clone());
            additions.push(e);
        }
    }
    let mut state = replay(&combined)?;
    let mut sources: Vec<_> = state.sources.values().cloned().collect();
    sources.sort_by_key(|s| {
        state
            .source_positions
            .get(&id(s, "sourceId"))
            .copied()
            .unwrap_or(0)
    });
    for source in sources {
        let project = id(&source, "projectId");
        let source_id = id(&source, "sourceId");
        let revision = source["revision"].as_u64().unwrap();
        if state.nonterminal() >= 32 {
            break;
        }
        if state.policy(&project)["enabled"] == true
            && state.source_positions.get(&source_id) > state.cutoffs.get(&project)
            && !state
                .jobs
                .iter()
                .any(|j| j["jobId"] == job_id(&project, &source_id, revision))
        {
            let e = event(
                "extraction.job_queued",
                state.queue_payload(&project, &source_id, revision, Value::Null, true),
                &at,
            );
            state.consume(&e, combined.len())?;
            combined.push(e.clone());
            additions.push(e);
        }
    }
    Ok(additions)
}
fn input(state: &State, job: &Value) -> Result<Value> {
    let source = state
        .sources
        .get(&id(job, "sourceId"))
        .ok_or_else(|| invalid("EXTRACTION_SOURCE_CHANGED"))?;
    let mut result = json!({"schemaVersion":1,"extractorVersion":VERSION,"projectId":job["projectId"],"source":{"sourceId":source["sourceId"],"revision":source["revision"],"title":source["title"],"text":source["text"],"kind":source["kind"]},"acceptedDecisions":[],"contextPartial":false});
    require(
        serde_json::to_vec(&result)?.len() <= MAX_INPUT_BYTES,
        "EXTRACTION_INPUT_TOO_LARGE",
    )?;
    let mut rows = Vec::new();
    let mut omitted = false;
    let tokens = |text: &str| -> HashSet<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let source_tokens = tokens(source["text"].as_str().unwrap());
    let mut context: Vec<_> = state
        .decisions
        .values()
        .filter(|d| d["projectId"] == job["projectId"] && d["status"] == "accepted")
        .map(|d| {
            let text = BODY
                .iter()
                .map(|(k, _)| d[*k].as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                .join(" ");
            let score = tokens(&text).intersection(&source_tokens).count();
            (d, score)
        })
        .collect();
    context.sort_by(|(a, sa), (b, sb)| {
        sb.cmp(sa)
            .then(a["decisionId"].as_str().cmp(&b["decisionId"].as_str()))
    });
    for (d, _) in context {
        let mut row = json!({"decisionId":d["decisionId"],"revision":d["revision"]});
        for (key, _) in BODY {
            row[key] = d[key].clone();
        }
        rows.push(row);
        result["acceptedDecisions"] = json!(rows);
        if rows.len() > 20 || serde_json::to_vec(&result)?.len() > MAX_INPUT_BYTES {
            rows.pop();
            omitted = true;
        }
    }
    result["acceptedDecisions"] = json!(rows);
    result["contextPartial"] = json!(omitted);
    require(
        serde_json::to_vec(&result)?.len() <= MAX_INPUT_BYTES,
        "EXTRACTION_INPUT_TOO_LARGE",
    )?;
    Ok(result)
}
pub fn start(events: &[Value], job_id: &str, owner_epoch: &str) -> Result<(Vec<Value>, Value)> {
    opaque(&json!(owner_epoch))?;
    let state = replay(events)?;
    let j = state.job(job_id)?;
    require(
        j["status"] == "queued" && j["attempt"].as_u64().unwrap() < 3,
        "EXTRACTION_NOT_QUEUED",
    )?;
    require(
        state.stale(j).is_none(),
        state.stale(j).unwrap_or("EXTRACTION_SOURCE_CHANGED"),
    )?;
    require(
        !state.jobs.iter().any(|j| j["status"] == "running"),
        "EXTRACTION_BUSY",
    )?;
    let source_input = input(&state, j);
    let e = event(
        "extraction.job_started",
        json!({"jobId":job_id,"attempt":j["attempt"].as_u64().unwrap()+1,"ownerEpoch":owner_epoch}),
        &crate::now(),
    );
    let mut combined = events.to_vec();
    combined.push(e.clone());
    let mut post = replay(&combined)?;
    let mut additions = vec![e];
    let source_input = match source_input {
        Ok(input) => input,
        Err(_) => {
            let failed = finish_fact(
                post.job(job_id)?,
                "failed",
                "EXTRACTION_INPUT_TOO_LARGE",
                0,
                &crate::now(),
            );
            combined.push(failed.clone());
            additions.push(failed);
            post = replay(&combined)?;
            Value::Null
        }
    };
    Ok((
        additions,
        json!({"job":post.job(job_id)?,"input":source_input}),
    ))
}
fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn applied_reason(action: &str) -> &'static str {
    match action {
        "project_extraction_configure" => "EXTRACTION_CONFIGURED",
        "project_extraction_enqueue" => "EXTRACTION_QUEUED",
        _ => "EXTRACTION_RETRIED",
    }
}
fn validate_output(state: &State, job: &Value, value: &Value) -> Result<Vec<Value>> {
    require(
        serde_json::to_vec(value)?.len() <= 128 * 1024,
        "EXTRACTION_INVALID_OUTPUT",
    )?;
    exact(value, &["proposals"])?;
    let proposals = value["proposals"]
        .as_array()
        .ok_or_else(|| invalid("EXTRACTION_INVALID_OUTPUT"))?;
    require(proposals.len() <= 2, "EXTRACTION_INVALID_OUTPUT")?;
    for proposal in proposals {
        exact(
            proposal,
            &[
                "title",
                "context",
                "decision",
                "rationale",
                "consequences",
                "evidence",
                "relation",
                "relatedDecisionIds",
            ],
        )?;
        for (key, max) in BODY {
            text(&proposal[key], max, !["title", "decision"].contains(&key))?;
        }
        state.anchor(proposal, job)?;
    }
    Ok(proposals.clone())
}
fn provider_reason(code: &str) -> &'static str {
    match code {
        "EXTRACTION_TIMEOUT" => "EXTRACTION_TIMEOUT",
        "EXTRACTION_CANCELLED" => "EXTRACTION_CANCELLED",
        "EXTRACTION_INTERRUPTED" => "EXTRACTION_INTERRUPTED",
        "EXTRACTION_PROVIDER_UNAVAILABLE" => "EXTRACTION_PROVIDER_UNAVAILABLE",
        "EXTRACTION_PROVIDER_UNSUPPORTED" => "EXTRACTION_PROVIDER_UNSUPPORTED",
        "EXTRACTION_INVALID_INPUT" => "EXTRACTION_INVALID_INPUT",
        "EXTRACTION_INVALID_OUTPUT" => "EXTRACTION_INVALID_OUTPUT",
        _ => "EXTRACTION_PROVIDER_FAILED",
    }
}
pub fn finish(
    events: &[Value],
    job_id: &str,
    attempt: u64,
    epoch: &str,
    outcome: std::result::Result<Value, String>,
) -> Result<(Vec<Value>, Value)> {
    let state = replay(events)?;
    let job = state.job(job_id)?;
    require(
        job["status"] == "running" && job["attempt"] == attempt && job["ownerEpoch"] == epoch,
        "EXTRACTION_ATTEMPT_CHANGED",
    )?;
    let at = crate::now();
    let outcome = if let Some(reason) = state.stale(job) {
        Err(("cancelled", reason))
    } else {
        match outcome {
            Err(code) => Err((
                match code.as_str() {
                    "EXTRACTION_CANCELLED" => "cancelled",
                    "EXTRACTION_INTERRUPTED" => "interrupted",
                    _ => "failed",
                },
                provider_reason(&code),
            )),
            Ok(value) => validate_output(&state, job, &value)
                .map_err(|_| ("failed", "EXTRACTION_INVALID_OUTPUT")),
        }
    };
    let mut additions = Vec::new();
    let mut combined = events.to_vec();
    let mut duplicates = 0;
    match outcome {
        Err((status, reason)) => additions.push(finish_fact(job, status, reason, 0, &at)),
        Ok(proposals) => {
            let mut seen: HashSet<_> = state
                .decisions
                .values()
                .filter(|d| d["projectId"] == job["projectId"])
                .map(|d| normalize(d["decision"].as_str().unwrap()))
                .collect();
            for (index, proposal) in proposals.iter().enumerate() {
                if !seen.insert(normalize(proposal["decision"].as_str().unwrap())) {
                    duplicates += 1;
                    continue;
                }
                let cmd = proposal_command(job_id, attempt, index as u64);
                let bound = json!({"jobId":job_id,"attempt":attempt,"index":index,"projectId":job["projectId"],"sourceId":job["sourceId"],"sourceRevision":job["sourceRevision"],"decisionId":decision_id(&cmd),"commandId":cmd,"generator":{"providerId":"codex","extractorVersion":VERSION},"evidence":proposal["evidence"],"relation":proposal["relation"],"relatedDecisionIds":proposal["relatedDecisionIds"]});
                let e = event("extraction.proposal_bound", bound, &at);
                combined.push(e.clone());
                additions.push(e);
                let memory = crate::project_memory::project_validated(&combined)?;
                let project = memory["projects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p["projectId"] == job["projectId"])
                    .ok_or_else(|| invalid("EXTRACTION_PROJECT_NOT_FOUND"))?;
                let mut payload = json!({"commandId":cmd,"projectId":job["projectId"],"expectedProjectRevision":project["revision"],"sources":[{"sourceId":job["sourceId"],"revision":job["sourceRevision"]}]});
                for (key, _) in BODY {
                    payload[key] = proposal[key].clone();
                }
                let (next, receipt) =
                    crate::decisions::apply(&combined, "project_decision_propose", &payload)?;
                require(
                    receipt["outcome"] == "applied",
                    "EXTRACTION_PROPOSAL_REJECTED",
                )?;
                combined.extend(next.iter().cloned());
                additions.extend(next);
            }
            let post = replay(&combined)?;
            additions.push(finish_fact(
                post.job(job_id)?,
                "completed",
                if proposals.is_empty() {
                    "EXTRACTION_NO_ADR"
                } else if duplicates == proposals.len() as u64 {
                    "EXTRACTION_DUPLICATE"
                } else {
                    "EXTRACTION_COMPLETED"
                },
                duplicates,
                &at,
            ));
        }
    }
    let mut combined = events.to_vec();
    combined.extend(additions.iter().cloned());
    let post = replay(&combined)?;
    Ok((additions, json!({"job":post.job(job_id)?})))
}
pub fn recover(events: &[Value]) -> Result<Vec<Value>> {
    let state = replay(events)?;
    let mut combined = events.to_vec();
    let mut additions = Vec::new();
    for e in events.iter().filter(|e| {
        e["type"] == "extraction.command.requested"
            && !state.receipts.contains_key(&id(&e["payload"], "commandId"))
    }) {
        let r = &e["payload"];
        let mut payload = r["payload"].clone();
        for key in ["commandId", "createdAt", "expiresAt"] {
            payload[key] = r[key].clone();
        }
        let (next, _) = apply(&combined, r["action"].as_str().unwrap(), &payload)?;
        combined.extend(next.iter().cloned());
        additions.extend(next);
    }
    let state = replay(&combined)?;
    let at = crate::now();
    for job in state.jobs.iter().filter(|j| j["status"] == "running") {
        additions.push(finish_fact(
            job,
            "interrupted",
            "EXTRACTION_INTERRUPTED",
            0,
            &at,
        ));
    }
    Ok(additions)
}
pub fn provenance(projection: &Value, decision_id: &str) -> Option<Value> {
    projection["provenance"]
        .as_array()?
        .iter()
        .find(|p| p["decisionId"] == decision_id)
        .cloned()
}
