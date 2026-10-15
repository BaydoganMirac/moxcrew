//! Outbound, bounded, allowlisted Cloud projection and typed-intent transport.
//! The Host applies commands through its canonical writer; this module owns only pairing metadata.
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::{Client, Method, Url};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

type Result<T> = std::result::Result<T, String>;
const MAX: usize = 65536;
const SAFE_INTEGER: u64 = 9_007_199_254_740_991;
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn invalid<T>() -> Result<T> {
    Err("CLOUD_PROTOCOL_INVALID".into())
}
fn exact(v: &Value, keys: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("CLOUD_PROTOCOL_INVALID")?;
    if m.len() != keys.len() || keys.iter().any(|k| !m.contains_key(*k)) {
        return invalid();
    }
    Ok(())
}
fn opaque(v: &Value) -> Result<&str> {
    let s = v.as_str().ok_or("CLOUD_PROTOCOL_INVALID")?;
    if s.is_empty()
        || s.len() > 256
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        return invalid();
    }
    Ok(s)
}
fn int(v: &Value, min: u64, max: u64) -> Result<u64> {
    let n = v.as_u64().ok_or("CLOUD_PROTOCOL_INVALID")?;
    if n < min || n > max {
        return invalid();
    }
    Ok(n)
}
fn iso(v: &Value) -> Result<&str> {
    let s = v.as_str().ok_or("CLOUD_PROTOCOL_INVALID")?;
    let d = DateTime::parse_from_rfc3339(s).map_err(|_| "CLOUD_PROTOCOL_INVALID")?;
    if d.to_rfc3339_opts(SecondsFormat::Millis, true) != s {
        return invalid();
    }
    Ok(s)
}
fn text(v: &Value, max: usize) -> Result<&str> {
    let s = v.as_str().ok_or("CLOUD_PROTOCOL_INVALID")?;
    if s.is_empty()
        || s.chars().count() > max
        || s.trim() != s
        || s.chars()
            .any(|c| c == '\0' || (c.is_control() && !['\t', '\n', '\r'].contains(&c)))
    {
        return invalid();
    }
    Ok(s)
}
fn boolean(v: &Value) -> Result<bool> {
    v.as_bool().ok_or("CLOUD_PROTOCOL_INVALID".into())
}
fn choice<'a>(v: &'a Value, allowed: &[&str]) -> Result<&'a str> {
    let s = v.as_str().ok_or("CLOUD_PROTOCOL_INVALID")?;
    if !allowed.contains(&s) {
        return invalid();
    }
    Ok(s)
}
fn semver(v: &Value) -> Result<&str> {
    let s = v.as_str().ok_or("CLOUD_PROTOCOL_INVALID")?;
    if s.len() > 256
        || !regex::Regex::new(
            r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$",
        )
        .unwrap()
        .is_match(s)
    {
        return invalid();
    }
    Ok(s)
}
fn arr(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("CLOUD_PROTOCOL_INVALID".into())
}
fn nullable_iso(v: &Value) -> Result<()> {
    if !v.is_null() {
        iso(v)?;
    }
    Ok(())
}
fn nullable_id(v: &Value) -> Result<()> {
    if !v.is_null() {
        opaque(v)?;
    }
    Ok(())
}
fn terminal(v: &Value) -> bool {
    matches!(
        v["status"].as_str(),
        Some("completed" | "failed" | "interrupted")
    )
}
fn hash(prefix: &str, data: &str) -> String {
    format!("{prefix}:{:x}", Sha256::digest(data.as_bytes()))
}
pub fn workspace_alias(installation_id: &str, workspace_id: &str) -> String {
    hash(
        "workspace-alias",
        &format!("{installation_id}\0{workspace_id}"),
    )
}
pub fn normalize_url(s: &str) -> Result<String> {
    let u = Url::parse(s).map_err(|_| "CLOUD_URL_INVALID")?;
    let loopback = matches!(
        u.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
    );
    if !(u.scheme() == "https" || (u.scheme() == "http" && loopback))
        || !u.username().is_empty()
        || u.password().is_some()
        || u.path() != "/"
        || u.query().is_some()
        || u.fragment().is_some()
        || u.host_str().is_none()
    {
        return Err("CLOUD_URL_INVALID".into());
    }
    Ok(u.origin().ascii_serialization())
}

/// Pure allowlist builder. Passing a whole local object directly to transport is impossible.
pub fn build_projection(
    bundle: &Value,
    version: u64,
    revision: u64,
    installation_id: &str,
) -> Result<Value> {
    if ![1, 2].contains(&version) || revision > SAFE_INTEGER {
        return invalid();
    }
    let empty = vec![];
    let obs = bundle["observation"]["observations"]
        .as_array()
        .unwrap_or(&empty);
    if obs.len() > 3 {
        return invalid();
    }
    let mut seen = HashSet::new();
    let mut providers = vec![];
    for o in obs {
        let p = &o["provider"];
        let id = choice(&p["id"], &["codex", "claude", "gemini"])?;
        if !seen.insert(id) {
            return invalid();
        }
        let v = if p["version"].is_null() {
            Value::Null
        } else {
            json!(semver(&p["version"])?)
        };
        providers.push(json!({"providerId":id,"state":choice(&p["state"],&["available","degraded","unavailable"])? ,"version":v,"capabilities":{"recentObservation":boolean(&p["capabilities"]["recentObservation"])? ,"explicitLiveStatus":boolean(&p["capabilities"]["explicitLiveStatus"])?},"observedAt":iso(&o["observedAt"])?}));
    }
    let works: Vec<_> = bundle["work"]["items"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter(|w| !terminal(w))
        .collect();
    let runs: Vec<_> = bundle["work"]["runs"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter(|w| !terminal(w))
        .collect();
    let mut work_items = vec![];
    for (position, w) in works.iter().take(32).enumerate() {
        if w["providerId"] != "codex" {
            return invalid();
        }
        work_items.push(json!({"workItemId":opaque(&w["workItemId"])? ,"providerId":"codex","status":choice(&w["status"],&["queued","launching","running","cancelling","blocked"])? ,"revision":int(&w["revision"],1,SAFE_INTEGER)? ,"queuePosition":position,"createdAt":iso(&w["createdAt"])?}));
    }
    let mut run_items = vec![];
    for r in runs.iter().take(32) {
        if r["providerId"] != "codex" {
            return invalid();
        }
        nullable_iso(&r["terminalAt"])?;
        run_items.push(json!({"runId":opaque(&r["runId"])? ,"workItemId":opaque(&r["workItemId"])? ,"providerId":"codex","status":choice(&r["status"],&["running","cancelling"])? ,"revision":int(&r["revision"],1,SAFE_INTEGER)? ,"startedAt":iso(&r["startedAt"])? ,"terminalAt":r["terminalAt"]}));
    }
    let mut payload = json!({"revision":revision,"providers":providers,"workItems":work_items,"runs":run_items,"workItemsTruncated":works.len()>32,"runsTruncated":runs.len()>32});
    if version == 1 {
        return Ok(payload);
    }
    let teams = bundle["teams"]["items"].as_array().unwrap_or(&empty);
    let mut team_items = vec![];
    for t in teams.iter().take(16) {
        let mut agents = vec![];
        for a in arr(&t["agents"])? {
            agents.push(json!({"agentId":opaque(&a["agentId"])? ,"packId":opaque(&a["pack"]["id"])? ,"role":text(&a["pack"]["role"],40)? ,"status":choice(&a["status"],&["draft","active","waiting","running","completed","failed","cancelled","timed_out"])?}));
        }
        let mut tasks = vec![];
        for task in arr(&t["tasks"])? {
            let question = if task["status"] == "waiting_question" && !task["question"].is_null() {
                json!({"questionId":opaque(&task["question"]["questionId"])? ,"kind":"release_constraints","maxAnswerLength":int(&task["question"]["maxAnswerLength"],1,2000)?})
            } else {
                Value::Null
            };
            tasks.push(json!({"taskId":opaque(&task["taskId"])? ,"agentId":opaque(&task["agentId"])? ,"status":choice(&task["status"],&["pending","ready","waiting_question","waiting_approval","queued","launching","running","cancelling","blocked","completed","failed","interrupted","cancelled"])? ,"attempt":int(&task["attempt"],1,6)? ,"maxAttempts":int(&task["maxAttempts"],1,6)? ,"approvalRequired":boolean(&task["approvalRequired"])? ,"question":question}));
        }
        validate_settings(&t["settings"])?;
        team_items.push(json!({"teamId":opaque(&t["teamId"])? ,"workspaceAlias":workspace_alias(installation_id,opaque(&t["workspaceId"])?),"templateId":text(&t["templateId"],40)? ,"status":choice(&t["status"],&["draft","active","waiting","completed","failed","cancelled","timed_out"])? ,"revision":int(&t["revision"],1,SAFE_INTEGER)? ,"settings":{"concurrency":t["settings"]["concurrency"],"retryLimit":t["settings"]["retryLimit"],"timeoutMinutes":t["settings"]["timeoutMinutes"],"failurePolicy":t["settings"]["failurePolicy"]},"agents":agents,"tasks":tasks}));
    }
    let mut spaces = vec![];
    for w in bundle["workspaces"]["items"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .take(32)
    {
        spaces.push(
            json!({"workspaceAlias":workspace_alias(installation_id,opaque(&w["workspaceId"])?)}),
        );
    }
    let mut packs = vec![];
    for p in bundle["agentPacks"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .take(32)
    {
        packs.push(json!({"packId":opaque(&p["id"])? ,"role":text(&p["role"],40)?}));
    }
    payload["protocolVersion"] = json!(2);
    payload["capabilities"] = json!({"teamSupervision":true,"remoteTeamStart":true,"actions":["cancel_run","cancel_team","cancel_agent","approve_decision","reject_decision","answer_question","start_team"]});
    payload["workspaces"] = json!(spaces);
    payload["agentPacks"] = json!(packs);
    payload["teams"] = json!(team_items);
    payload["teamsTruncated"] = json!(teams.len() > 16);
    Ok(payload)
}
fn validate_settings(v: &Value) -> Result<()> {
    exact(
        v,
        &[
            "concurrency",
            "retryLimit",
            "timeoutMinutes",
            "failurePolicy",
        ],
    )?;
    int(&v["concurrency"], 1, 4)?;
    int(&v["retryLimit"], 0, 3)?;
    int(&v["timeoutMinutes"], 5, 480)?;
    choice(&v["failurePolicy"], &["stop", "continue"])?;
    Ok(())
}
pub fn validate_intent(v: &Value, protocol: u64, at: &str) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "intentId",
            "idempotencyKey",
            "type",
            "actorId",
            "createdAt",
            "expiresAt",
            "payload",
        ],
    )?;
    opaque(&v["intentId"])?;
    opaque(&v["idempotencyKey"])?;
    if v["actorId"] != "cloud-owner" {
        return invalid();
    }
    let created = DateTime::parse_from_rfc3339(iso(&v["createdAt"])?)
        .map_err(|_| "CLOUD_PROTOCOL_INVALID")?;
    let expires = DateTime::parse_from_rfc3339(iso(&v["expiresAt"])?)
        .map_err(|_| "CLOUD_PROTOCOL_INVALID")?;
    let current = DateTime::parse_from_rfc3339(at).map_err(|_| "CLOUD_PROTOCOL_INVALID")?;
    let window = expires.signed_duration_since(created).num_milliseconds();
    if window <= 0
        || window > 300000
        || created.signed_duration_since(current).num_milliseconds() > 60000
    {
        return invalid();
    }
    let p = &v["payload"];
    if v["schemaVersion"] == 1 {
        if v["type"] != "cancel_run" {
            return invalid();
        }
        exact(p, &["runId", "expectedRunRevision"])?;
        opaque(&p["runId"])?;
        int(&p["expectedRunRevision"], 1, SAFE_INTEGER)?;
        return Ok(());
    }
    if protocol != 2
        || v["schemaVersion"] != 2
        || !regex::Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$").unwrap().is_match(v["intentId"].as_str().unwrap_or(""))
    {
        return invalid();
    }
    match v["type"].as_str() {
        Some("cancel_team") => {
            exact(p, &["teamId", "expectedTeamRevision"])?;
        }
        Some("cancel_agent") => {
            exact(p, &["teamId", "agentId", "expectedTeamRevision"])?;
            opaque(&p["agentId"])?;
        }
        Some("approve_decision" | "reject_decision") => {
            exact(p, &["teamId", "taskId", "expectedTeamRevision", "note"])?;
            opaque(&p["taskId"])?;
            if !p["note"].is_null() {
                text(&p["note"], 1000)?;
            }
        }
        Some("answer_question") => {
            exact(
                p,
                &["teamId", "questionId", "expectedTeamRevision", "answer"],
            )?;
            opaque(&p["questionId"])?;
            text(&p["answer"], 2000)?;
        }
        Some("start_team") => {
            exact(
                p,
                &[
                    "workspaceAlias",
                    "teamName",
                    "goal",
                    "templateId",
                    "orchestratorPackId",
                    "workerPackIds",
                    "settings",
                ],
            )?;
            opaque(&p["workspaceAlias"])?;
            text(&p["teamName"], 80)?;
            text(&p["goal"], 4000)?;
            text(&p["templateId"], 40)?;
            opaque(&p["orchestratorPackId"])?;
            let workers = arr(&p["workerPackIds"])?;
            if workers.is_empty() || workers.len() > 4 {
                return invalid();
            }
            let mut seen = HashSet::new();
            for w in workers {
                if !seen.insert(opaque(w)?) {
                    return invalid();
                }
            }
            validate_settings(&p["settings"])?;
            return Ok(());
        }
        _ => return invalid(),
    };
    opaque(&p["teamId"])?;
    int(&p["expectedTeamRevision"], 1, SAFE_INTEGER)?;
    Ok(())
}
pub fn validate_intent_page(
    v: &Value,
    protocol: u64,
    host_id: &str,
    after: u64,
    at: &str,
) -> Result<Vec<Value>> {
    if ![1, 2].contains(&protocol) {
        return invalid();
    }
    exact(v, &["schemaVersion", "hostId", "cursor", "intents"])?;
    if v["schemaVersion"] != protocol || opaque(&v["hostId"])? != host_id {
        return invalid();
    }
    let items = arr(&v["intents"])?;
    if items.len() > 20 {
        return invalid();
    }
    let mut cursor = after;
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();
    for entry in items {
        exact(entry, &["sequence", "intent"])?;
        let n = int(&entry["sequence"], 1, SAFE_INTEGER)?;
        if n <= cursor {
            return invalid();
        }
        cursor = n;
        validate_intent(&entry["intent"], protocol, at)?;
        if !ids.insert(entry["intent"]["intentId"].clone().to_string())
            || !keys.insert(entry["intent"]["idempotencyKey"].clone().to_string())
        {
            return invalid();
        }
    }
    if int(&v["cursor"], 0, SAFE_INTEGER)? != cursor {
        return invalid();
    }
    Ok(items.clone())
}
pub fn validate_team_intent(v: &Value) -> Result<()> {
    if v["schemaVersion"] != 2 {
        return invalid();
    }
    validate_intent(v, 2, iso(&v["createdAt"])?)
}
pub fn validate_team_receipt(v: &Value) -> Result<()> {
    if v["schemaVersion"] != 2 {
        return invalid();
    }
    build_receipt(v).map(|_| ())
}
pub fn build_receipt(r: &Value) -> Result<Value> {
    let version = int(&r["schemaVersion"], 1, 2)?;
    exact(
        r,
        if version == 1 {
            &[
                "schemaVersion",
                "intentId",
                "idempotencyKey",
                "commandType",
                "outcome",
                "reasonCode",
                "completedAt",
                "workProjectionRevision",
                "workItemId",
            ]
        } else {
            &[
                "schemaVersion",
                "intentId",
                "idempotencyKey",
                "commandType",
                "outcome",
                "reasonCode",
                "completedAt",
                "teamId",
                "teamRevision",
            ]
        },
    )?;
    opaque(&r["intentId"])?;
    opaque(&r["idempotencyKey"])?;
    iso(&r["completedAt"])?;
    let outcome = choice(&r["outcome"], &["applied", "rejected", "expired", "failed"])?;
    let reason = r["reasonCode"].as_str().ok_or("CLOUD_PROTOCOL_INVALID")?;
    if reason.is_empty()
        || reason.len() > 64
        || !reason
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    {
        return invalid();
    }
    if version == 1 {
        if r["commandType"] != "cancel_run" {
            return invalid();
        }
        let allowed = match outcome {
            "applied" => &["RUN_CANCELLED"][..],
            "rejected" => &[
                "PROVIDER_CONTROL_UNAVAILABLE",
                "RUN_CANCEL_PENDING",
                "RUN_NOT_ACTIVE",
                "RUN_NOT_FOUND",
                "STALE_RUN_REVISION",
            ][..],
            "expired" => &["COMMAND_EXPIRED"][..],
            _ => &["PROVIDER_CONTROL_FAILED", "RUN_SESSION_LOST"][..],
        };
        if !allowed.contains(&reason) {
            return invalid();
        }
        int(&r["workProjectionRevision"], 0, SAFE_INTEGER)?;
        nullable_id(&r["workItemId"])?;
    } else {
        choice(
            &r["commandType"],
            &[
                "cancel_team",
                "cancel_agent",
                "approve_decision",
                "reject_decision",
                "answer_question",
                "start_team",
            ],
        )?;
        nullable_id(&r["teamId"])?;
        int(&r["teamRevision"], 0, SAFE_INTEGER)?;
    }
    Ok(json!({"receipts":[r]}))
}

pub struct SyncBatch {
    pub intents: Vec<Value>,
}
pub struct Cloud {
    directory: PathBuf,
    state: Value,
    client: Client,
    pending: Vec<Value>,
    attempt_at: Option<String>,
}
impl Cloud {
    pub fn open(data_dir: &Path) -> Result<Self> {
        crate::store::private_directory(data_dir).map_err(|_| "CLOUD_STATE_FAILED")?;
        let directory = fs::canonicalize(data_dir).map_err(|_| "CLOUD_STATE_FAILED")?;
        crate::store::secure_file(&directory.join("cloud.json"))
            .map_err(|_| "CLOUD_STATE_FAILED")?;
        let cloud_file = directory.join("cloud.json");
        let state = match crate::store::open_private_reader(&cloud_file) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take((MAX + 1) as u64)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "CLOUD_STATE_FAILED")?;
                if bytes.len() > MAX {
                    return Err("CLOUD_STATE_INVALID".into());
                }
                let v = serde_json::from_slice(&bytes).map_err(|_| "CLOUD_STATE_INVALID")?;
                validate_state(&v)?;
                v
            }
            Err(_) if !cloud_file.exists() => {
                json!({"schemaVersion":1,"installationId":format!("install:{}",Uuid::new_v4()),"connection":null})
            }
            Err(_) => return Err("CLOUD_STATE_FAILED".into()),
        };
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "CLOUD_UNAVAILABLE")?;
        let c = Self {
            directory,
            state,
            client,
            pending: vec![],
            attempt_at: None,
        };
        c.persist()?;
        Ok(c)
    }
    pub fn installation_id(&self) -> &str {
        self.state["installationId"].as_str().unwrap()
    }
    pub fn public_status(&self) -> Value {
        let c = &self.state["connection"];
        if c.is_null() {
            json!({"paired":false})
        } else {
            json!({"paired":true,"cloudUrl":c["cloudUrl"],"hostId":c["hostId"],"workspaceId":c["workspaceId"],"lastAttemptAt":c["lastAttemptAt"],"lastSuccessAt":c["lastSuccessAt"],"lastErrorCode":c["lastErrorCode"]})
        }
    }
    pub(crate) fn memory_connection(&self) -> Value {
        let c = &self.state["connection"];
        if c.is_null() {
            Value::Null
        } else {
            json!({"cloudUrl":c["cloudUrl"],"workspaceId":c["workspaceId"],"hostId":c["hostId"]})
        }
    }
    /// Separate opt-in memory transport reuses the protected paired credential.
    /// The caller supplies path segments, never an arbitrary outbound URL.
    pub(crate) async fn memory_request(
        &self,
        bound: &Value,
        method: Method,
        segments: &[&str],
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value> {
        let c = &self.state["connection"];
        if bound.is_null() || self.memory_connection() != *bound {
            return Err("MEMORY_CONNECTION_CHANGED".into());
        }
        let mut url = Url::parse(c["cloudUrl"].as_str().ok_or("MEMORY_CONNECTION_CHANGED")?)
            .map_err(|_| "MEMORY_PROTOCOL_INVALID")?;
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| "MEMORY_PROTOCOL_INVALID")?;
            path.clear().extend([
                "api",
                "memory",
                "v1",
                "hosts",
                c["hostId"].as_str().unwrap(),
            ]);
            path.extend(segments);
        }
        url.query_pairs_mut()
            .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        let mut request = self
            .client
            .request(method, url)
            .header("content-type", "application/json")
            .header("cache-control", "no-store")
            .bearer_auth(
                c["credential"]
                    .as_str()
                    .ok_or("MEMORY_CONNECTION_CHANGED")?,
            );
        if let Some(body) = body {
            let bytes = serde_json::to_vec(body).map_err(|_| "MEMORY_PROTOCOL_INVALID")?;
            if bytes.len() > crate::shared_memory::MAX_RESPONSE {
                return Err("MEMORY_INVALID_PAYLOAD".into());
            }
            request = request.header("content-length", bytes.len()).body(bytes);
        }
        let mut response = request.send().await.map_err(|_| "MEMORY_UNAVAILABLE")?;
        let status = response.status();
        let content = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let no_store = response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(',')
                    .any(|x| x.trim().eq_ignore_ascii_case("no-store"))
            });
        if !(content.eq_ignore_ascii_case("application/json")
            || content.eq_ignore_ascii_case("application/json; charset=utf-8"))
            || !no_store
        {
            return Err("MEMORY_PROTOCOL_INVALID".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "MEMORY_UNAVAILABLE")? {
            if bytes.len() + chunk.len() > crate::shared_memory::MAX_RESPONSE {
                return Err("MEMORY_PROTOCOL_INVALID".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| "MEMORY_PROTOCOL_INVALID")?;
        if !status.is_success() {
            exact(&value, &["error"]).map_err(|_| "MEMORY_PROTOCOL_INVALID")?;
            let allowed = [
                "MEMORY_SUBSCRIPTION_REQUIRED",
                "MEMORY_ACCESS_DENIED",
                "MEMORY_SHARING_DISABLED",
                "MEMORY_CONFLICT",
                "MEMORY_INVALID_PAYLOAD",
            ];
            return Err(value["error"]
                .as_str()
                .filter(|s| allowed.contains(s))
                .unwrap_or("MEMORY_REQUEST_REJECTED")
                .into());
        }
        Ok(value)
    }
    fn persist(&self) -> Result<()> {
        validate_state(&self.state)?;
        crate::store::atomic_write(
            &self.directory.join("cloud.json"),
            &serde_json::to_vec(&self.state).map_err(|_| "CLOUD_STATE_FAILED")?,
        )
        .map_err(|_| "CLOUD_STATE_FAILED".into())
    }
    pub fn unpair(&mut self) -> Result<Value> {
        self.state["connection"] = Value::Null;
        self.pending.clear();
        self.attempt_at = None;
        self.persist()?;
        Ok(self.public_status())
    }
    async fn request(
        &self,
        method: Method,
        url: String,
        body: Option<Value>,
        credential: Option<&str>,
    ) -> std::result::Result<Value, RequestError> {
        let mut request = self
            .client
            .request(method, url)
            .header("content-type", "application/json");
        if let Some(c) = credential {
            request = request.bearer_auth(c);
        }
        if let Some(b) = body {
            let bytes =
                serde_json::to_vec(&b).map_err(|_| RequestError::code("CLOUD_PROTOCOL_INVALID"))?;
            if bytes.len() > MAX {
                return Err(RequestError::code("CLOUD_PROTOCOL_INVALID"));
            }
            request = request.body(bytes);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| RequestError::code("CLOUD_UNAVAILABLE"))?;
        let status = response.status();
        if status == 401 || status == 403 {
            return Err(RequestError::code("CLOUD_AUTH_REJECTED"));
        }
        let content = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if content != "application/json" && content != "application/json; charset=utf-8" {
            return Err(RequestError::code(if status == 401 || status == 403 {
                "CLOUD_AUTH_REJECTED"
            } else {
                "CLOUD_PROTOCOL_INVALID"
            }));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| RequestError::code("CLOUD_UNAVAILABLE"))?
        {
            if bytes.len() + chunk.len() > MAX {
                return Err(RequestError::code("CLOUD_PROTOCOL_INVALID"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| RequestError::code("CLOUD_PROTOCOL_INVALID"))?;
        if !status.is_success() {
            let code = if status == 401 || status == 403 {
                "CLOUD_AUTH_REJECTED"
            } else if status == 409 {
                "CLOUD_CONFLICT"
            } else {
                "CLOUD_REQUEST_REJECTED"
            };
            return Err(RequestError {
                code: code.into(),
                reason: value["error"].as_str().map(str::to_string),
            });
        }
        Ok(value)
    }
    pub async fn pair(&mut self, config: &Value) -> Result<Value> {
        exact(config, &["cloudUrl", "pairingCode", "displayName"])?;
        if !self.state["connection"].is_null() {
            return Err("CLOUD_ALREADY_PAIRED".into());
        }
        let url = normalize_url(config["cloudUrl"].as_str().ok_or("CLOUD_URL_INVALID")?)?;
        let code = text(&config["pairingCode"], 256)?;
        let display = text(&config["displayName"], 80)?;
        self.persist()?;
        let mut protocol = 2;
        let response = loop {
            let body = json!({"schemaVersion":1,"messageId":format!("pair:{}",Uuid::new_v4()),"occurredAt":now(),"pairingCode":code,"installationId":self.installation_id(),"displayName":display,"appVersion":env!("CARGO_PKG_VERSION"),"protocolVersion":protocol,"osFamily":os_family()});
            match self
                .request(
                    Method::POST,
                    format!("{url}/api/v1/pairings/consume"),
                    Some(body),
                    None,
                )
                .await
            {
                Ok(v) => break v,
                Err(e) if protocol == 2 && e.code == "CLOUD_REQUEST_REJECTED" => protocol = 1,
                Err(e) => return Err(e.code),
            }
        };
        exact(
            &response,
            &[
                "schemaVersion",
                "hostId",
                "workspaceId",
                "credential",
                "protocolVersion",
            ],
        )?;
        if response["schemaVersion"] != 1 {
            return invalid();
        }
        int(&response["protocolVersion"], 1, 2)?;
        opaque(&response["hostId"])?;
        opaque(&response["workspaceId"])?;
        opaque(&response["credential"])?;
        self.state["connection"] = json!({"cloudUrl":url,"hostId":response["hostId"],"workspaceId":response["workspaceId"],"credential":response["credential"],"protocolVersion":response["protocolVersion"],"cursor":0,"projectionRevisionOffset":0,"lastAttemptAt":null,"lastSuccessAt":null,"lastErrorCode":null});
        self.persist()?;
        Ok(self.public_status())
    }
    pub async fn begin_sync(&mut self, bundle: &Value) -> Result<SyncBatch> {
        self.pending.clear();
        if self.state["connection"].is_null() {
            return Ok(SyncBatch { intents: vec![] });
        }
        self.attempt_at = Some(now());
        let result = self.begin_inner(bundle).await;
        match result {
            Ok(batch) => {
                self.pending = batch.intents.clone();
                Ok(batch)
            }
            Err(code) => {
                self.record_error(&code)?;
                Err(code)
            }
        }
    }
    async fn begin_inner(&mut self, bundle: &Value) -> Result<SyncBatch> {
        let c = self.state["connection"].clone();
        let at = self.attempt_at.as_ref().unwrap().clone();
        let sequence = int(&bundle["sequence"], 0, SAFE_INTEGER)?;
        let offset = c["projectionRevisionOffset"].as_u64().unwrap_or(0);
        let mut revision = sequence
            .checked_add(offset)
            .filter(|n| *n <= SAFE_INTEGER)
            .ok_or("CLOUD_PROTOCOL_INVALID")?;
        let protocol = c["protocolVersion"].as_u64().unwrap();
        let occurred = bundle
            .get("recordedAt")
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or(json!("1970-01-01T00:00:00.000Z"));
        iso(&occurred)?;
        let mut payload = build_projection(bundle, protocol, revision, self.installation_id())?;
        let api = format!(
            "{}/api/v{protocol}/hosts/{}",
            c["cloudUrl"].as_str().unwrap(),
            c["hostId"].as_str().unwrap()
        );
        let credential = c["credential"].as_str().unwrap();
        let heartbeat = envelope(
            &c,
            format!("heartbeat:{}", Uuid::new_v4()),
            at.clone(),
            json!({"appVersion":env!("CARGO_PKG_VERSION"),"protocolVersion":protocol,"lastLocalSequence":sequence,"health":"ok"}),
        );
        accepted(
            &self
                .request(
                    Method::POST,
                    format!("{api}/heartbeat"),
                    Some(heartbeat),
                    Some(credential),
                )
                .await
                .map_err(|e| e.code)?,
        )?;
        let send = |p: Value, r: u64| {
            envelope(
                &c,
                hash(
                    "projection",
                    &format!("{}:{r}", c["hostId"].as_str().unwrap()),
                ),
                occurred.as_str().unwrap().into(),
                p,
            )
        };
        let response = self
            .request(
                Method::PUT,
                format!("{api}/projection"),
                Some(send(payload.clone(), revision)),
                Some(credential),
            )
            .await;
        match response {
            Ok(v) => accepted(&v)?,
            Err(e)
                if e.code == "CLOUD_CONFLICT"
                    && matches!(
                        e.reason.as_deref(),
                        Some("conflicting message replay" | "conflicting projection revision")
                    ) =>
            {
                revision += 1;
                payload["revision"] = json!(revision);
                accepted(
                    &self
                        .request(
                            Method::PUT,
                            format!("{api}/projection"),
                            Some(send(payload, revision)),
                            Some(credential),
                        )
                        .await
                        .map_err(|e| e.code)?,
                )?;
                self.state["connection"]["projectionRevisionOffset"] = json!(offset + 1);
                self.persist()?;
            }
            Err(e) => return Err(e.code),
        }
        let after = c["cursor"].as_u64().unwrap();
        let page = self
            .request(
                Method::GET,
                format!("{api}/intents?after={after}"),
                None,
                Some(credential),
            )
            .await
            .map_err(|e| e.code)?;
        let intents =
            validate_intent_page(&page, protocol, c["hostId"].as_str().unwrap(), after, &at)?;
        Ok(SyncBatch { intents })
    }
    pub async fn deliver_receipt(&mut self, sequence: u64, receipt: &Value) -> Result<()> {
        let result = self.deliver_inner(sequence, receipt).await;
        if let Err(ref code) = result {
            self.record_error(code)?;
        }
        result
    }
    async fn deliver_inner(&mut self, sequence: u64, receipt: &Value) -> Result<()> {
        let expected = self.pending.first().ok_or("CLOUD_RECEIPT_ORDER_INVALID")?;
        if expected["sequence"] != sequence
            || expected["intent"]["intentId"] != receipt["intentId"]
            || expected["intent"]["idempotencyKey"] != receipt["idempotencyKey"]
            || expected["intent"]["type"] != receipt["commandType"]
        {
            return Err("CLOUD_RECEIPT_ORDER_INVALID".into());
        }
        let payload = build_receipt(receipt)?;
        let c = &self.state["connection"];
        if c.is_null() {
            return Err("CLOUD_UNPAIRED".into());
        }
        if c["protocolVersion"] == 1 && receipt["schemaVersion"] != 1 {
            return invalid();
        }
        let api = format!(
            "{}/api/v{}/hosts/{}/receipts",
            c["cloudUrl"].as_str().unwrap(),
            c["protocolVersion"].as_u64().unwrap(),
            c["hostId"].as_str().unwrap()
        );
        let body = envelope(
            c,
            hash("receipt", receipt["intentId"].as_str().unwrap()),
            receipt["completedAt"].as_str().unwrap().into(),
            payload,
        );
        accepted(
            &self
                .request(Method::POST, api, Some(body), c["credential"].as_str())
                .await
                .map_err(|e| e.code)?,
        )?;
        self.state["connection"]["cursor"] = json!(sequence);
        self.mark_success();
        self.persist()?;
        self.pending.remove(0);
        Ok(())
    }
    fn mark_success(&mut self) {
        self.state["connection"]["lastAttemptAt"] = json!(self.attempt_at);
        self.state["connection"]["lastSuccessAt"] = json!(now());
        self.state["connection"]["lastErrorCode"] = Value::Null;
    }
    fn record_error(&mut self, code: &str) -> Result<()> {
        if !self.state["connection"].is_null() {
            self.state["connection"]["lastAttemptAt"] = json!(self.attempt_at);
            self.state["connection"]["lastErrorCode"] =
                json!(if code.starts_with("CLOUD_") && code.len() <= 64 {
                    code
                } else {
                    "CLOUD_SYNC_FAILED"
                });
            self.persist()?;
        }
        Ok(())
    }
    pub fn finish_sync(&mut self) -> Result<Value> {
        if self.state["connection"].is_null() {
            return Ok(json!({"kind":"unpaired"}));
        }
        if !self.pending.is_empty() {
            return Err("CLOUD_RECEIPTS_PENDING".into());
        }
        self.mark_success();
        self.persist()?;
        Ok(json!({"kind":"synced","cursor":self.state["connection"]["cursor"]}))
    }
}
struct RequestError {
    code: String,
    reason: Option<String>,
}
impl RequestError {
    fn code(code: &str) -> Self {
        Self {
            code: code.into(),
            reason: None,
        }
    }
}
fn envelope(c: &Value, id: String, at: String, payload: Value) -> Value {
    json!({"schemaVersion":1,"messageId":id,"hostId":c["hostId"],"occurredAt":at,"payload":payload})
}
fn accepted(v: &Value) -> Result<()> {
    exact(v, &["schemaVersion", "accepted"])?;
    if v["schemaVersion"] != 1 || v["accepted"] != true {
        return invalid();
    }
    Ok(())
}
fn os_family() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}
fn validate_state(v: &Value) -> Result<()> {
    exact(v, &["schemaVersion", "installationId", "connection"])?;
    if v["schemaVersion"] != 1 {
        return Err("CLOUD_STATE_INVALID".into());
    }
    opaque(&v["installationId"])?;
    let c = &v["connection"];
    if c.is_null() {
        return Ok(());
    }
    let mut normalized = c.clone();
    if normalized.get("projectionRevisionOffset").is_none() {
        normalized["projectionRevisionOffset"] = json!(0);
    }
    exact(
        &normalized,
        &[
            "cloudUrl",
            "hostId",
            "workspaceId",
            "credential",
            "protocolVersion",
            "cursor",
            "projectionRevisionOffset",
            "lastAttemptAt",
            "lastSuccessAt",
            "lastErrorCode",
        ],
    )?;
    normalize_url(c["cloudUrl"].as_str().ok_or("CLOUD_STATE_INVALID")?)?;
    opaque(&c["hostId"])?;
    opaque(&c["workspaceId"])?;
    opaque(&c["credential"])?;
    int(&c["protocolVersion"], 1, 2)?;
    int(&c["cursor"], 0, SAFE_INTEGER)?;
    int(&normalized["projectionRevisionOffset"], 0, SAFE_INTEGER)?;
    nullable_iso(&c["lastAttemptAt"])?;
    nullable_iso(&c["lastSuccessAt"])?;
    if !c["lastErrorCode"].is_null() {
        let s = c["lastErrorCode"].as_str().ok_or("CLOUD_STATE_INVALID")?;
        if s.len() > 64 || !regex::Regex::new(r"^[A-Z][A-Z0-9_]*$").unwrap().is_match(s) {
            return Err("CLOUD_STATE_INVALID".into());
        }
    }
    Ok(())
}
