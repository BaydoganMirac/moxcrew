//! The V1 local event contract. These projectors intentionally preserve the
//! legacy JavaScript event meaning; migration must not reinterpret old facts.
use crate::{Error, Result};
use chrono::{DateTime, SecondsFormat};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

const MAX_SAFE: u64 = 9_007_199_254_740_991;
const TERMINAL: &[&str] = &["completed", "failed", "interrupted"];
const WORK_STATUS: &[&str] = &[
    "queued",
    "launching",
    "running",
    "cancelling",
    "blocked",
    "completed",
    "failed",
    "interrupted",
];
const COMMANDS: &[&str] = &[
    "enqueue_work",
    "remove_queued_work",
    "start_work",
    "cancel_run",
];
const UUID_PATTERN: &str =
    "[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";

fn fail(label: &str) -> Error {
    Error(format!("LOCAL_STORAGE_CORRUPT: {label}"))
}
fn require(ok: bool, label: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(fail(label))
    }
}
fn object(v: &Value) -> Result<&serde_json::Map<String, Value>> {
    v.as_object().ok_or_else(|| fail("object required"))
}
fn exact(v: &Value, fields: &[&str]) -> Result<()> {
    let m = object(v)?;
    require(
        m.len() == fields.len() && m.keys().all(|k| fields.contains(&k.as_str())),
        "unknown or missing field",
    )
}
fn allowed(v: &Value, fields: &[&str], required: &[&str]) -> Result<()> {
    let m = object(v)?;
    require(
        m.keys().all(|k| fields.contains(&k.as_str()))
            && required.iter().all(|k| m.contains_key(*k)),
        "unknown or missing field",
    )
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| fail("string required"))
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or_else(|| fail("array required"))
}
fn number(v: &Value, min: u64, max: u64) -> Result<u64> {
    let n = v.as_u64().ok_or_else(|| fail("integer required"))?;
    require(n >= min && n <= max.min(MAX_SAFE), "integer out of range")?;
    Ok(n)
}
fn revision(v: &Value, min: u64) -> Result<u64> {
    number(v, min, MAX_SAFE)
}
fn matches(v: &Value, pattern: &str) -> Result<()> {
    static PATTERNS: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();
    let mut cache = PATTERNS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| fail("validator lock poisoned"))?;
    let regex = cache
        .entry(pattern.to_owned())
        .or_insert_with(|| Regex::new(pattern).expect("constant regex"));
    require(regex.is_match(string(v)?), "invalid identity or version")
}
fn prefixed(v: &Value, prefix: &str) -> Result<()> {
    matches(v, &format!("(?i)^{prefix}:{UUID_PATTERN}$"))
}
fn opaque(v: &Value) -> Result<()> {
    matches(v, r"^[A-Za-z0-9._:-]{1,256}$")
}
fn iso(v: &Value) -> Result<()> {
    let s = string(v)?;
    let d = DateTime::parse_from_rfc3339(s).map_err(|_| fail("invalid ISO timestamp"))?;
    require(
        d.with_timezone(&chrono::Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, true)
            == s,
        "noncanonical ISO timestamp",
    )
}
fn text(v: &Value, maximum: usize, safe_controls: bool) -> Result<()> {
    let s = string(v)?;
    // JavaScript counts UTF-16 code units, including the two units of an emoji.
    let len = s.encode_utf16().count();
    require(
        !s.is_empty()
            && len <= maximum
            && s.trim() == s
            && (!safe_controls
                || !s
                    .chars()
                    .any(|c| matches!(c as u32, 0..=8 | 11..=12 | 14..=31 | 127))),
        "invalid text",
    )
}
fn one_of(v: &Value, values: &[&str]) -> Result<()> {
    require(values.contains(&string(v)?), "unsupported enum value")
}
fn unique_enum(v: &Value, values: &[&str], min: usize) -> Result<()> {
    let a = array(v)?;
    require(
        a.len() >= min && a.len() <= values.len(),
        "invalid enum array",
    )?;
    let mut seen = HashSet::new();
    for x in a {
        one_of(x, values)?;
        require(seen.insert(string(x)?), "duplicate enum value")?;
    }
    Ok(())
}
fn absolute(v: &Value, trim: bool, reject_nul: bool) -> Result<()> {
    let s = string(v)?;
    // Records are portable: accept either POSIX or Windows absolute paths even
    // when replaying on a different host. The host resolves live paths itself.
    let b = s.as_bytes();
    let windows = s.starts_with("\\\\")
        || (b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && matches!(b[2], b'\\' | b'/'));
    require(
        s.encode_utf16().count() <= 4096
            && (!trim || s.trim() == s)
            && (!reject_nul || !s.contains('\0'))
            && (s.starts_with('/') || windows),
        "absolute directory required",
    )
}

pub fn validate_work_item(v: &Value) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "workItemId",
            "title",
            "instruction",
            "providerId",
            "workingDirectory",
            "status",
            "createdAt",
            "revision",
        ],
    )?;
    require(
        v["schemaVersion"] == 1 && v["providerId"] == "codex",
        "invalid work version/provider",
    )?;
    opaque(&v["workItemId"])?;
    text(&v["title"], 160, false)?;
    text(&v["instruction"], 50_000, false)?;
    absolute(&v["workingDirectory"], false, true)?;
    one_of(&v["status"], WORK_STATUS)?;
    iso(&v["createdAt"])?;
    revision(&v["revision"], 1)?;
    Ok(())
}
pub fn validate_run(v: &Value) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "runId",
            "workItemId",
            "providerId",
            "ownerEpoch",
            "providerSessionId",
            "providerTurnId",
            "status",
            "startedAt",
            "terminalAt",
            "revision",
        ],
    )?;
    require(
        v["schemaVersion"] == 1 && v["providerId"] == "codex",
        "invalid run version/provider",
    )?;
    for k in [
        "runId",
        "workItemId",
        "ownerEpoch",
        "providerSessionId",
        "providerTurnId",
    ] {
        opaque(&v[k])?;
    }
    one_of(
        &v["status"],
        &[
            "running",
            "cancelling",
            "completed",
            "failed",
            "interrupted",
        ],
    )?;
    if !v["terminalAt"].is_null() {
        iso(&v["terminalAt"])?;
    }
    require(
        TERMINAL.contains(&string(&v["status"])?) == !v["terminalAt"].is_null(),
        "run terminal state mismatch",
    )?;
    iso(&v["startedAt"])?;
    revision(&v["revision"], 1)?;
    Ok(())
}
pub fn validate_command_intent(v: &Value) -> Result<()> {
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
    require(v["schemaVersion"] == 1, "unsupported command version")?;
    one_of(&v["type"], COMMANDS)?;
    for k in ["intentId", "idempotencyKey", "actorId"] {
        opaque(&v[k])?;
    }
    iso(&v["createdAt"])?;
    iso(&v["expiresAt"])?;
    require(
        string(&v["expiresAt"])? > string(&v["createdAt"])?,
        "invalid command expiry",
    )?;
    let p = &v["payload"];
    match string(&v["type"])? {
        "enqueue_work" => {
            exact(p, &["workItem"])?;
            validate_work_item(&p["workItem"])?;
            require(
                p["workItem"]["status"] == "queued" && p["workItem"]["revision"] == 1,
                "new work must be queued at revision one",
            )?;
        }
        "cancel_run" => {
            exact(p, &["runId", "expectedRunRevision"])?;
            opaque(&p["runId"])?;
            revision(&p["expectedRunRevision"], 1)?;
        }
        _ => {
            exact(p, &["workItemId", "expectedItemRevision"])?;
            opaque(&p["workItemId"])?;
            revision(&p["expectedItemRevision"], 1)?;
        }
    }
    Ok(())
}
pub fn validate_command_receipt(v: &Value) -> Result<()> {
    exact(
        v,
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
        ],
    )?;
    require(v["schemaVersion"] == 1, "unsupported receipt version")?;
    one_of(&v["commandType"], COMMANDS)?;
    one_of(&v["outcome"], &["applied", "rejected", "expired", "failed"])?;
    let reasons: &[&str] = match (string(&v["commandType"])?, string(&v["outcome"])?) {
        ("enqueue_work", "applied") => &["WORK_ENQUEUED"],
        ("enqueue_work", "rejected") => &["WORK_ITEM_EXISTS"],
        ("remove_queued_work", "applied") => &["WORK_REMOVED"],
        ("remove_queued_work", "rejected") => &[
            "STALE_ITEM_REVISION",
            "WORK_ITEM_NOT_FOUND",
            "WORK_ITEM_NOT_QUEUED",
        ],
        ("start_work", "applied") => &["WORK_STARTED"],
        ("start_work", "rejected") => &[
            "PROVIDER_CONTROL_UNAVAILABLE",
            "STALE_ITEM_REVISION",
            "WORKSPACE_NOT_ALLOWED",
            "WORK_ITEM_NOT_FOUND",
            "WORK_ITEM_NOT_QUEUED",
            "WORK_START_PENDING",
        ],
        ("cancel_run", "applied") => &["RUN_CANCELLED"],
        ("cancel_run", "rejected") => &[
            "PROVIDER_CONTROL_UNAVAILABLE",
            "RUN_CANCEL_PENDING",
            "RUN_NOT_ACTIVE",
            "RUN_NOT_FOUND",
            "STALE_RUN_REVISION",
        ],
        (_, "expired") => &["COMMAND_EXPIRED"],
        ("start_work", "failed") => &["PROVIDER_CONTROL_FAILED", "START_OUTCOME_UNKNOWN"],
        ("cancel_run", "failed") => &["PROVIDER_CONTROL_FAILED", "RUN_SESSION_LOST"],
        (_, "failed") => &["COMMAND_FAILED"],
        _ => &[],
    };
    one_of(&v["reasonCode"], reasons)?;
    opaque(&v["intentId"])?;
    opaque(&v["idempotencyKey"])?;
    if !v["workItemId"].is_null() {
        opaque(&v["workItemId"])?;
    }
    iso(&v["completedAt"])?;
    revision(&v["workProjectionRevision"], 0)?;
    Ok(())
}
pub fn validate_workspace(v: &Value) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "workspaceId",
            "displayName",
            "workingDirectory",
            "createdAt",
            "revision",
        ],
    )?;
    require(v["schemaVersion"] == 1, "unsupported workspace version")?;
    prefixed(&v["workspaceId"], "workspace")?;
    text(&v["displayName"], 80, false)?;
    absolute(&v["workingDirectory"], true, false)?;
    iso(&v["createdAt"])?;
    revision(&v["revision"], 1)?;
    Ok(())
}
pub fn validate_workspace_command_intent(v: &Value) -> Result<()> {
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
    require(
        v["schemaVersion"] == 1 && v["actorId"] == "local-owner",
        "invalid workspace command",
    )?;
    one_of(&v["type"], &["register_workspace", "remove_workspace"])?;
    prefixed(&v["intentId"], "cmd")?;
    prefixed(&v["idempotencyKey"], "cmd")?;
    iso(&v["createdAt"])?;
    iso(&v["expiresAt"])?;
    require(
        string(&v["expiresAt"])? > string(&v["createdAt"])?,
        "invalid workspace expiry",
    )?;
    let p = &v["payload"];
    if v["type"] == "register_workspace" {
        exact(p, &["workspace"])?;
        validate_workspace(&p["workspace"])?;
    } else {
        exact(p, &["workspaceId", "expectedWorkspaceRevision"])?;
        prefixed(&p["workspaceId"], "workspace")?;
        revision(&p["expectedWorkspaceRevision"], 1)?;
    }
    Ok(())
}
pub fn validate_workspace_command_receipt(v: &Value) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "intentId",
            "idempotencyKey",
            "commandType",
            "outcome",
            "reasonCode",
            "completedAt",
            "workspaceProjectionRevision",
            "workspaceId",
        ],
    )?;
    require(v["schemaVersion"] == 1, "unsupported workspace receipt")?;
    one_of(
        &v["commandType"],
        &["register_workspace", "remove_workspace"],
    )?;
    one_of(&v["outcome"], &["applied", "rejected", "expired"])?;
    matches(&v["reasonCode"], r"^[A-Z][A-Z0-9_]{1,63}$")?;
    prefixed(&v["intentId"], "cmd")?;
    prefixed(&v["idempotencyKey"], "cmd")?;
    prefixed(&v["workspaceId"], "workspace")?;
    iso(&v["completedAt"])?;
    revision(&v["workspaceProjectionRevision"], 0)?;
    Ok(())
}
pub fn validate_pack(v: &Value) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "id",
            "version",
            "name",
            "role",
            "description",
            "providerId",
            "instructions",
            "requiredCapabilities",
            "permissions",
            "defaultModel",
            "limits",
            "expectedOutput",
            "qualityChecks",
            "provenance",
        ],
    )?;
    require(
        v["schemaVersion"] == 1 && v["providerId"] == "codex",
        "invalid pack version/provider",
    )?;
    matches(&v["id"], r"^pack:[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$")?;
    matches(&v["version"], r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")?;
    one_of(
        &v["role"],
        &[
            "orchestrator",
            "product",
            "design",
            "frontend",
            "backend",
            "fullstack",
            "qa",
            "review",
            "security",
            "release",
            "docs",
            "research",
        ],
    )?;
    for (key, max) in [
        ("name", 80),
        ("description", 280),
        ("instructions", 20_000),
        ("expectedOutput", 500),
    ] {
        text(&v[key], max, true)?;
    }
    if !v["defaultModel"].is_null() {
        text(&v["defaultModel"], 120, true)?;
    }
    unique_enum(
        &v["requiredCapabilities"],
        &["work.start", "work.cancel"],
        1,
    )?;
    unique_enum(
        &v["permissions"],
        &["read_workspace", "write_workspace", "run_checks"],
        1,
    )?;
    unique_enum(
        &v["qualityChecks"],
        &["tests", "lint", "build", "review", "security"],
        0,
    )?;
    exact(&v["limits"], &["maxAttempts", "timeoutMinutes"])?;
    number(&v["limits"]["maxAttempts"], 1, 5)?;
    number(&v["limits"]["timeoutMinutes"], 1, 240)?;
    exact(&v["provenance"], &["kind", "source"])?;
    one_of(&v["provenance"]["kind"], &["built-in", "local"])?;
    text(&v["provenance"]["source"], 120, true)?;
    Ok(())
}
pub fn team_work_item_id(team: &str, task: &str, attempt: u64) -> Result<String> {
    prefixed(&json!(team), "team")?;
    prefixed(&json!(task), "task")?;
    require((1..=6).contains(&attempt), "invalid attempt")?;
    Ok(format!("work:{}:{}:{attempt}", &team[5..], &task[5..]))
}
pub fn validate_team(v: &Value) -> Result<()> {
    exact(
        v,
        &[
            "schemaVersion",
            "teamId",
            "name",
            "goal",
            "workspaceId",
            "templateId",
            "orchestratorAgentId",
            "agents",
            "tasks",
            "settings",
            "createdAt",
        ],
    )?;
    require(v["schemaVersion"] == 1, "unsupported team version")?;
    prefixed(&v["teamId"], "team")?;
    prefixed(&v["workspaceId"], "workspace")?;
    text(&v["name"], 80, true)?;
    text(&v["goal"], 10_000, true)?;
    iso(&v["createdAt"])?;
    one_of(
        &v["templateId"],
        &[
            "product-feature",
            "bug-fix",
            "saas-launch",
            "design-frontend",
            "security-audit",
            "release",
        ],
    )?;
    let agents = array(&v["agents"])?;
    let tasks = array(&v["tasks"])?;
    require(
        (2..=12).contains(&agents.len()) && (1..=32).contains(&tasks.len()),
        "invalid team size",
    )?;
    let mut ids = HashSet::new();
    let mut orchestrator = false;
    for agent in agents {
        exact(agent, &["agentId", "pack"])?;
        prefixed(&agent["agentId"], "agent")?;
        validate_pack(&agent["pack"])?;
        require(
            ids.insert(string(&agent["agentId"])?),
            "duplicate team agent",
        )?;
        if agent["agentId"] == v["orchestratorAgentId"] {
            orchestrator = agent["pack"]["role"] == "orchestrator";
        }
    }
    require(orchestrator, "invalid orchestrator")?;
    let mut definitions = BTreeMap::new();
    for task in tasks {
        exact(
            task,
            &[
                "taskId",
                "title",
                "instruction",
                "agentId",
                "dependsOn",
                "approvalRequired",
                "question",
            ],
        )?;
        prefixed(&task["taskId"], "task")?;
        text(&task["title"], 100, true)?;
        text(&task["instruction"], 4000, true)?;
        require(
            ids.contains(string(&task["agentId"])?)
                && task["agentId"] != v["orchestratorAgentId"]
                && task["approvalRequired"].is_boolean(),
            "invalid task owner",
        )?;
        let dependencies = array(&task["dependsOn"])?;
        require(dependencies.len() <= 31, "too many dependencies")?;
        let mut seen = HashSet::new();
        for d in dependencies {
            prefixed(d, "task")?;
            require(seen.insert(string(d)?), "duplicate dependency")?;
        }
        let q = &task["question"];
        if !q.is_null() {
            exact(q, &["questionId", "prompt", "maxAnswerLength"])?;
            prefixed(&q["questionId"], "question")?;
            text(&q["prompt"], 500, true)?;
            number(&q["maxAnswerLength"], 1, 2000)?;
        }
        require(
            definitions.insert(string(&task["taskId"])?, task).is_none(),
            "duplicate task",
        )?;
    }
    fn visit<'a>(
        id: &'a str,
        defs: &BTreeMap<&'a str, &'a Value>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(());
        }
        require(visiting.insert(id), "cyclic task dependencies")?;
        let task = defs.get(id).ok_or_else(|| fail("unknown dependency"))?;
        for dep in array(&task["dependsOn"])? {
            let dep = string(dep)?;
            require(dep != id, "self dependency")?;
            visit(dep, defs, visiting, visited)?;
        }
        visiting.remove(id);
        visited.insert(id);
        Ok(())
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for id in definitions.keys() {
        visit(id, &definitions, &mut visiting, &mut visited)?;
    }
    exact(
        &v["settings"],
        &[
            "concurrency",
            "retryLimit",
            "timeoutMinutes",
            "failurePolicy",
        ],
    )?;
    number(&v["settings"]["concurrency"], 1, 4)?;
    number(&v["settings"]["retryLimit"], 0, 3)?;
    number(&v["settings"]["timeoutMinutes"], 5, 480)?;
    one_of(&v["settings"]["failurePolicy"], &["stop", "continue"])?;
    Ok(())
}

pub fn safe_observation_error(provider: &str, code: &str) -> Result<Value> {
    let message = match (provider, code) {
        ("codex", "CODEX_APP_SERVER_EXITED") => "Codex observation stopped unexpectedly.",
        ("codex", "CODEX_APP_SERVER_START_FAILED") => "Codex observation could not start.",
        ("codex", "CODEX_APP_SERVER_TIMEOUT") => "Codex observation timed out.",
        ("codex", "CODEX_NOT_FOUND") => "Codex CLI is not installed or is not on PATH.",
        ("codex", "CODEX_PROBE_FAILED") => "Codex CLI could not be checked.",
        ("codex", "CODEX_PROBE_TIMEOUT") => "Codex CLI version check timed out.",
        ("codex", "CODEX_PROTOCOL_INVALID_JSON" | "CODEX_PROTOCOL_MALFORMED") => {
            "Codex returned an unsupported response."
        }
        ("codex", "CODEX_PROTOCOL_METHOD_ERROR") => "Codex rejected the observation request.",
        ("codex", "CODEX_SYSTEM_ERROR") => "Codex reported an observation error.",
        ("codex", "CODEX_VERSION_MALFORMED") => "Codex CLI returned an unsupported version.",
        ("claude", "CLAUDE_NOT_FOUND") => "Claude Code CLI is not installed or is not on PATH.",
        ("claude", "CLAUDE_PROBE_FAILED") => "Claude Code CLI could not be checked.",
        ("claude", "CLAUDE_PROBE_TIMEOUT") => "Claude Code CLI version check timed out.",
        ("claude", "CLAUDE_SNAPSHOT_FAILED") => "Claude Code Agent View could not be checked.",
        ("claude", "CLAUDE_SNAPSHOT_MALFORMED" | "CLAUDE_SNAPSHOT_UNSUPPORTED") => {
            "Claude Code Agent View returned an unsupported response."
        }
        ("claude", "CLAUDE_SNAPSHOT_TIMEOUT") => "Claude Code Agent View timed out.",
        ("claude", "CLAUDE_SURFACE_UNSUPPORTED") => {
            "This Claude Code version lacks the supported Agent View surface."
        }
        ("claude", "CLAUDE_VERSION_MALFORMED") => {
            "Claude Code CLI returned an unsupported version."
        }
        ("gemini", "GEMINI_HOOK_SETUP_REQUIRED") => "Gemini CLI hook setup is required.",
        ("gemini", "GEMINI_NOT_FOUND") => "Gemini CLI is not installed or is not on PATH.",
        ("gemini", "GEMINI_PROBE_FAILED") => "Gemini CLI could not be checked.",
        ("gemini", "GEMINI_PROBE_TIMEOUT") => "Gemini CLI version check timed out.",
        ("gemini", "GEMINI_VERSION_MALFORMED") => "Gemini CLI returned an unsupported version.",
        _ => return Err(fail("unknown observation error")),
    };
    // This one legacy code has a different message from the malformed code.
    let message = if code == "CLAUDE_SNAPSHOT_UNSUPPORTED" {
        "Claude Code Agent View returned an unsupported session shape."
    } else {
        message
    };
    Ok(json!({"code": code, "message": message}))
}
fn validate_error(provider: &str, v: &Value) -> Result<()> {
    exact(v, &["code", "message"])?;
    require(
        *v == safe_observation_error(provider, string(&v["code"])?)?,
        "unsafe observation error",
    )
}
pub fn provider_name(provider: &str) -> Result<&'static str> {
    match provider {
        "codex" => Ok("Codex"),
        "claude" => Ok("Claude Code"),
        "gemini" => Ok("Gemini CLI"),
        _ => Err(fail("unknown observation provider")),
    }
}
fn validate_session(p: &Value, created: bool) -> Result<()> {
    opaque(&p["providerSessionId"])?;
    one_of(
        &p["status"],
        if created {
            &["completed", "failed", "interrupted", "running", "unknown"]
        } else {
            TERMINAL
        },
    )?;
    if created && !p["createdAt"].is_null() {
        iso(&p["createdAt"])?;
    }
    iso(&p["lastObservedAt"])?;
    if let Some(v) = p.get("terminalAt") {
        iso(v)?;
    }
    Ok(())
}
fn validate_observation_event(kind: &str, p: &Value) -> Result<()> {
    let provider = string(&p["providerId"])?;
    provider_name(provider)?;
    match kind {
        "provider.observed" => {
            allowed(
                p,
                &[
                    "providerId",
                    "state",
                    "version",
                    "capabilities",
                    "error",
                    "observedAt",
                    "sessionIds",
                ],
                &[
                    "providerId",
                    "state",
                    "version",
                    "capabilities",
                    "observedAt",
                    "sessionIds",
                ],
            )?;
            one_of(&p["state"], &["available", "degraded", "unavailable"])?;
            if !p["version"].is_null() {
                matches(
                    &p["version"],
                    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$",
                )?;
            }
            exact(
                &p["capabilities"],
                &["recentObservation", "explicitLiveStatus"],
            )?;
            require(
                p["capabilities"]["recentObservation"].is_boolean()
                    && p["capabilities"]["explicitLiveStatus"].is_boolean(),
                "invalid observation capabilities",
            )?;
            let has_error = p.get("error").is_some_and(|e| !e.is_null());
            require(
                (p["state"] == "available") != has_error,
                "provider state/error mismatch",
            )?;
            if has_error {
                validate_error(provider, &p["error"])?;
            }
            iso(&p["observedAt"])?;
            let ids = array(&p["sessionIds"])?;
            let mut seen = HashSet::new();
            for id in ids {
                opaque(id)?;
                require(seen.insert(string(id)?), "duplicate observed session")?;
            }
        }
        "session.observed" => {
            allowed(
                p,
                &[
                    "providerId",
                    "providerSessionId",
                    "status",
                    "createdAt",
                    "lastObservedAt",
                    "terminalAt",
                ],
                &[
                    "providerId",
                    "providerSessionId",
                    "status",
                    "createdAt",
                    "lastObservedAt",
                ],
            )?;
            validate_session(p, true)?;
        }
        "session.terminal" => {
            allowed(
                p,
                &[
                    "providerId",
                    "providerSessionId",
                    "status",
                    "lastObservedAt",
                    "terminalAt",
                ],
                &[
                    "providerId",
                    "providerSessionId",
                    "status",
                    "lastObservedAt",
                ],
            )?;
            validate_session(p, false)?;
        }
        "observation.failed" => {
            exact(p, &["providerId", "observedAt", "error"])?;
            iso(&p["observedAt"])?;
            validate_error(provider, &p["error"])?;
        }
        _ => return Err(fail("unsupported observation event")),
    }
    Ok(())
}

pub fn validate_event(v: &Value) -> Result<()> {
    if v["schemaVersion"] == 3 {
        return crate::project_memory::validate_event(v);
    }
    exact(v, &["id", "schemaVersion", "type", "recordedAt", "payload"])?;
    matches(&v["id"], &format!("(?i)^{UUID_PATTERN}$"))?;
    iso(&v["recordedAt"])?;
    let p = &v["payload"];
    let kind = string(&v["type"])?;
    object(p)?;
    if v["schemaVersion"] == 2 {
        match kind {
            "cloud_team.command.requested" => {
                exact(p, &["sequence", "intent"])?;
                number(&p["sequence"], 1, MAX_SAFE)?;
                crate::cloud::validate_team_intent(&p["intent"])?;
            }
            "cloud_team.command.receipted" => {
                exact(p, &["sequence", "receipt"])?;
                number(&p["sequence"], 1, MAX_SAFE)?;
                crate::cloud::validate_team_receipt(&p["receipt"])?;
                require(
                    p["receipt"]["completedAt"] == v["recordedAt"],
                    "cloud receipt timestamp mismatch",
                )?;
            }
            "owner.command.requested" => {
                exact(p, &["commandId", "action", "payload"])?;
                opaque(&p["commandId"])?;
                one_of(&p["action"], OWNER_ACTIONS)?;
                bounded_object(&p["payload"])?;
            }
            "owner.command.receipted" => {
                exact(p, &["commandId", "result"])?;
                opaque(&p["commandId"])?;
                bounded_object(&p["result"])?;
            }
            _ => return Err(fail("unsupported V2 local event")),
        }
        return Ok(());
    }
    require(v["schemaVersion"] == 1, "unsupported event version")?;
    match kind {
        "provider.observed" | "session.observed" | "session.terminal" | "observation.failed" => {
            validate_observation_event(kind, p)?
        }
        "command.requested" => {
            exact(p, &["intent"])?;
            validate_command_intent(&p["intent"])?;
        }
        "command.receipted" => {
            exact(p, &["receipt"])?;
            validate_command_receipt(&p["receipt"])?;
            require(
                p["receipt"]["completedAt"] == v["recordedAt"],
                "receipt timestamp mismatch",
            )?;
        }
        "work.enqueued" => {
            exact(p, &["intentId", "workItem"])?;
            opaque(&p["intentId"])?;
            validate_work_item(&p["workItem"])?;
        }
        "work.removed" | "run.launching" | "run.start_unknown" => {
            exact(
                p,
                if kind == "work.removed" {
                    &["intentId", "workItemId", "expectedItemRevision"]
                } else {
                    &[
                        "intentId",
                        "workItemId",
                        "expectedItemRevision",
                        "ownerEpoch",
                    ]
                },
            )?;
            opaque(&p["intentId"])?;
            opaque(&p["workItemId"])?;
            revision(&p["expectedItemRevision"], 1)?;
            if kind != "work.removed" {
                opaque(&p["ownerEpoch"])?;
            }
        }
        "run.prepared" => {
            exact(
                p,
                &[
                    "intentId",
                    "workItemId",
                    "providerId",
                    "providerSessionId",
                    "expectedItemRevision",
                    "ownerEpoch",
                ],
            )?;
            require(p["providerId"] == "codex", "invalid prepared provider")?;
            for k in ["intentId", "workItemId", "providerSessionId", "ownerEpoch"] {
                opaque(&p[k])?;
            }
            revision(&p["expectedItemRevision"], 1)?;
        }
        "turn.requested" => {
            exact(p, &["intentId", "providerSessionId", "ownerEpoch"])?;
            for k in ["intentId", "providerSessionId", "ownerEpoch"] {
                opaque(&p[k])?;
            }
        }
        "run.started" => {
            exact(
                p,
                &["intentId", "run", "expectedItemRevision", "ownerEpoch"],
            )?;
            opaque(&p["intentId"])?;
            opaque(&p["ownerEpoch"])?;
            validate_run(&p["run"])?;
            revision(&p["expectedItemRevision"], 1)?;
        }
        "run.session_lost" => {
            exact(p, &["runId", "ownerEpoch"])?;
            opaque(&p["runId"])?;
            opaque(&p["ownerEpoch"])?;
        }
        "run.interrupted" => {
            exact(p, &["intentId", "runId", "expectedRunRevision"])?;
            opaque(&p["intentId"])?;
            opaque(&p["runId"])?;
            revision(&p["expectedRunRevision"], 1)?;
        }
        "workspace.command.requested" => {
            exact(p, &["intent"])?;
            validate_workspace_command_intent(&p["intent"])?;
        }
        "workspace.command.receipted" => {
            exact(p, &["receipt"])?;
            validate_workspace_command_receipt(&p["receipt"])?;
        }
        "workspace.registered" => {
            exact(p, &["intentId", "workspace"])?;
            prefixed(&p["intentId"], "cmd")?;
            validate_workspace(&p["workspace"])?;
        }
        "workspace.removed" => {
            exact(p, &["intentId", "workspaceId", "expectedWorkspaceRevision"])?;
            prefixed(&p["intentId"], "cmd")?;
            prefixed(&p["workspaceId"], "workspace")?;
            revision(&p["expectedWorkspaceRevision"], 1)?;
        }
        "agent_pack.installed" => {
            exact(p, &["pack"])?;
            validate_pack(&p["pack"])?;
            require(
                p["pack"]["provenance"]["kind"] == "local",
                "installed pack must be local",
            )?;
        }
        "agent_pack.removed" => {
            exact(p, &["packId", "expectedRevision"])?;
            matches(&p["packId"], r"^pack:[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$")?;
            revision(&p["expectedRevision"], 1)?;
        }
        "team.created" => {
            exact(p, &["team"])?;
            validate_team(&p["team"])?;
        }
        "team.started" | "team.cancelled" | "team.timed_out" => {
            exact(p, &["teamId"])?;
            prefixed(&p["teamId"], "team")?;
        }
        "team.agent.cancelled" => {
            exact(p, &["teamId", "agentId"])?;
            prefixed(&p["teamId"], "team")?;
            prefixed(&p["agentId"], "agent")?;
        }
        "team.task.retry_requested" => {
            exact(p, &["teamId", "taskId", "previousAttempt"])?;
            prefixed(&p["teamId"], "team")?;
            prefixed(&p["taskId"], "task")?;
            number(&p["previousAttempt"], 1, 5)?;
        }
        "team.approval.decided" => {
            exact(p, &["teamId", "taskId", "decision", "note"])?;
            prefixed(&p["teamId"], "team")?;
            prefixed(&p["taskId"], "task")?;
            one_of(&p["decision"], &["approved", "rejected"])?;
            if !p["note"].is_null() {
                text(&p["note"], 1000, true)?;
            }
        }
        "team.question.answered" => {
            exact(p, &["teamId", "questionId", "answer"])?;
            prefixed(&p["teamId"], "team")?;
            prefixed(&p["questionId"], "question")?;
            text(&p["answer"], 2000, true)?;
        }
        _ => return Err(fail("unsupported complete event")),
    }
    Ok(())
}

const OWNER_ACTIONS: &[&str] = &[
    "pack_install",
    "pack_remove",
    "team_create",
    "team_start",
    "team_advance",
    "team_cancel",
    "team_cancel_agent",
    "team_approve",
    "team_answer",
];
fn bounded_object(v: &Value) -> Result<()> {
    object(v)?;
    require(
        serde_json::to_vec(v)?.len() <= 128 * 1024,
        "local command payload too large",
    )
}
fn validate_extended_replay(events: &[Value]) -> Result<()> {
    let mut owner_requests = Values::new();
    let mut owner_receipts = HashSet::new();
    let mut cloud_requests = Values::new();
    let mut cloud_keys = HashSet::new();
    let mut cloud_receipts = HashSet::new();
    for e in events {
        let p = &e["payload"];
        match e["type"].as_str().unwrap() {
            "owner.command.requested" => {
                require(
                    owner_requests
                        .insert(id(p, "commandId"), p.clone())
                        .is_none(),
                    "duplicate owner command",
                )?;
            }
            "owner.command.receipted" => {
                let i = id(p, "commandId");
                require(
                    owner_requests.contains_key(&i) && owner_receipts.insert(i),
                    "owner receipt has no unique request",
                )?;
            }
            "cloud_team.command.requested" => {
                let intent = &p["intent"];
                require(
                    cloud_requests
                        .insert(id(intent, "intentId"), p.clone())
                        .is_none()
                        && cloud_keys.insert(id(intent, "idempotencyKey")),
                    "duplicate cloud team request",
                )?;
            }
            "cloud_team.command.receipted" => {
                let receipt = &p["receipt"];
                let i = id(receipt, "intentId");
                let request = cloud_requests
                    .get(&i)
                    .ok_or_else(|| fail("cloud team receipt has no request"))?;
                require(
                    cloud_receipts.insert(i)
                        && request["sequence"] == p["sequence"]
                        && request["intent"]["idempotencyKey"] == receipt["idempotencyKey"]
                        && request["intent"]["type"] == receipt["commandType"],
                    "cloud team receipt attribution mismatch",
                )?;
                if request["intent"]["type"] != "start_team" && !receipt["teamId"].is_null() {
                    require(
                        request["intent"]["payload"]["teamId"] == receipt["teamId"],
                        "cloud receipt team mismatch",
                    )?;
                }
            }
            _ => (),
        }
    }
    Ok(())
}

fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    // Display ordering is deterministic across hosts, independent of OS ICU.
    // Event/revision/status meaning is unchanged. Name ordering can differ
    // from the legacy UI's environment-dependent localeCompare.
    a.cmp(b)
}
fn sort_values(items: &mut [Value], keys: &[&str], descending: bool) {
    items.sort_by(|a, b| {
        for (i, k) in keys.iter().enumerate() {
            let mut ord = locale_cmp(a[k].as_str().unwrap_or(""), b[k].as_str().unwrap_or(""));
            if descending && i == 0 {
                ord = ord.reverse();
            }
            if !ord.is_eq() {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn project_observation(events: &[Value]) -> Result<Value> {
    let mut sessions: BTreeMap<(String, String), Value> = BTreeMap::new();
    let mut providers = BTreeMap::new();
    for e in events {
        let p = &e["payload"];
        let kind = string(&e["type"])?;
        if kind == "session.observed" {
            let mut s = json!({"providerSessionId":p["providerSessionId"], "status":p["status"], "createdAt":p["createdAt"], "lastObservedAt":p["lastObservedAt"]});
            if let Some(t) = p.get("terminalAt") {
                s["terminalAt"] = t.clone();
            }
            sessions.insert(
                (
                    string(&p["providerId"])?.into(),
                    string(&p["providerSessionId"])?.into(),
                ),
                s,
            );
        } else if kind == "session.terminal" {
            let s = sessions
                .get_mut(&(
                    string(&p["providerId"])?.into(),
                    string(&p["providerSessionId"])?.into(),
                ))
                .ok_or_else(|| fail("terminal observation has no session"))?;
            s["status"] = p["status"].clone();
            s["lastObservedAt"] = p["lastObservedAt"].clone();
            if let Some(t) = p.get("terminalAt") {
                s["terminalAt"] = t.clone();
            }
        } else if kind == "provider.observed" {
            let provider = string(&p["providerId"])?;
            for id in array(&p["sessionIds"])? {
                require(
                    sessions.contains_key(&(provider.into(), string(id)?.into())),
                    "provider references unknown session",
                )?;
            }
            providers.insert(provider.to_owned(), p.clone());
        }
    }
    if providers.is_empty() {
        return Ok(Value::Null);
    }
    let mut observations = Vec::new();
    for id in ["codex", "claude", "gemini"] {
        if let Some(p) = providers.get(id) {
            let ids = array(&p["sessionIds"])?;
            require(
                ids.len() <= 20 && (p["state"] != "unavailable" || ids.is_empty()),
                "invalid projected session list",
            )?;
            let session_values: Vec<_> = ids
                .iter()
                .map(|s| sessions[&(id.into(), s.as_str().unwrap().into())].clone())
                .collect();
            let mut provider = json!({"id":id, "displayName":provider_name(id)?, "state":p["state"], "version":p["version"], "capabilities":p["capabilities"]});
            if let Some(err) = p.get("error").filter(|v| !v.is_null()) {
                provider["error"] = err.clone();
            }
            observations.push(json!({"schemaVersion":1,"provider":provider,"observedAt":p["observedAt"],"sessions":session_values}));
        }
    }
    Ok(json!({"schemaVersion":2,"observations":observations}))
}

type Values = BTreeMap<String, Value>;
fn id(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_owned()
}
fn pending(
    intents: &Values,
    facts: &BTreeMap<String, String>,
    receipted: &HashSet<String>,
    predicate: impl Fn(&Value) -> bool,
) -> bool {
    intents.values().any(|v| {
        !facts.contains_key(&id(v, "intentId"))
            && !receipted.contains(&id(v, "intentId"))
            && predicate(v)
    })
}
fn changed(mut v: Value, status: &str, terminal_at: Option<&Value>) -> Value {
    v["status"] = json!(status);
    v["revision"] = json!(v["revision"].as_u64().unwrap() + 1);
    if let Some(t) = terminal_at {
        v["terminalAt"] = t.clone();
    }
    v
}
fn project_work(events: &[Value]) -> Result<Value> {
    let mut intents = Values::new();
    let mut keys = HashSet::new();
    let mut items = Values::new();
    let mut runs = Values::new();
    let mut sessions: BTreeMap<String, String> = BTreeMap::new();
    let mut seen_items = HashSet::new();
    let mut launches = Values::new();
    let mut preparations = Values::new();
    let mut turns = Values::new();
    let mut cancel_snapshots = Values::new();
    let mut facts: BTreeMap<String, String> = BTreeMap::new();
    let mut receipts = Vec::new();
    let mut receipted = HashSet::new();
    let mut revision_value = 0u64;
    let mut found = false;
    for e in events {
        let kind = string(&e["type"])?;
        let p = &e["payload"];
        if kind == "session.observed" || kind == "session.terminal" {
            if p["providerId"] != "codex" {
                continue;
            }
            let run = sessions
                .get(&id(p, "providerSessionId"))
                .and_then(|key| runs.get(key))
                .cloned();
            let Some(run) = run else {
                continue;
            };
            let next = string(&p["status"])?;
            if !["running", "completed", "failed", "interrupted"].contains(&next)
                || TERMINAL.contains(&string(&run["status"])?)
                || p["status"] == run["status"]
            {
                continue;
            }
            let terminal = if TERMINAL.contains(&next) {
                p.get("terminalAt").unwrap_or(&p["lastObservedAt"])
            } else {
                &Value::Null
            };
            let item = items
                .get(&id(&run, "workItemId"))
                .ok_or_else(|| fail("observed run has no work item"))?
                .clone();
            let next_run = changed(run, next, Some(terminal));
            validate_run(&next_run)?;
            runs.insert(id(&next_run, "runId"), next_run);
            let next_item = changed(item, next, None);
            validate_work_item(&next_item)?;
            items.insert(id(&next_item, "workItemId"), next_item);
            revision_value += 1;
            continue;
        }
        if !(kind.starts_with("command.")
            || kind.starts_with("work.")
            || kind.starts_with("run.")
            || kind.starts_with("turn."))
        {
            continue;
        }
        found = true;
        let intent_id = id(p, "intentId");
        match kind {
            "command.requested" => {
                let intent = &p["intent"];
                let i = id(intent, "intentId");
                require(
                    !intents.contains_key(&i) && keys.insert(id(intent, "idempotencyKey")),
                    "duplicate command",
                )?;
                if intent["type"] == "cancel_run" {
                    cancel_snapshots.insert(
                        i.clone(),
                        runs.get(&id(&intent["payload"], "runId"))
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                }
                intents.insert(i, intent.clone());
            }
            "work.enqueued" => {
                let intent = intents
                    .get(&intent_id)
                    .ok_or_else(|| fail("enqueue has no intent"))?;
                let item = &p["workItem"];
                let wi = id(item, "workItemId");
                require(
                    intent["type"] == "enqueue_work"
                        && !facts.contains_key(&intent_id)
                        && intent["payload"]["workItem"] == *item
                        && seen_items.insert(wi.clone()),
                    "invalid enqueue event",
                )?;
                items.insert(wi, item.clone());
                facts.insert(intent_id, kind.into());
                revision_value += 1;
            }
            "work.removed" => {
                let intent = intents
                    .get(&intent_id)
                    .ok_or_else(|| fail("remove has no intent"))?;
                let wi = id(p, "workItemId");
                let item = items.get(&wi).ok_or_else(|| fail("removed item missing"))?;
                require(
                    intent["type"] == "remove_queued_work"
                        && !facts.contains_key(&intent_id)
                        && intent["payload"]["workItemId"] == p["workItemId"]
                        && intent["payload"]["expectedItemRevision"] == p["expectedItemRevision"]
                        && item["status"] == "queued"
                        && item["revision"] == p["expectedItemRevision"],
                    "invalid remove event",
                )?;
                items.remove(&wi);
                facts.insert(intent_id, kind.into());
                revision_value += 1;
            }
            "run.launching" | "run.prepared" | "run.start_unknown" | "run.started" => {
                let intent = intents
                    .get(&intent_id)
                    .ok_or_else(|| fail("run has no start intent"))?;
                let wi = if kind == "run.started" {
                    id(&p["run"], "workItemId")
                } else {
                    id(p, "workItemId")
                };
                let item = items
                    .get(&wi)
                    .ok_or_else(|| fail("start work item missing"))?
                    .clone();
                require(
                    intent["type"] == "start_work"
                        && intent["payload"]["workItemId"] == json!(wi)
                        && intent["payload"]["expectedItemRevision"] == p["expectedItemRevision"]
                        && item["status"] == "queued"
                        && item["revision"] == p["expectedItemRevision"],
                    "invalid run start reference",
                )?;
                if kind == "run.launching" {
                    require(!launches.contains_key(&intent_id), "duplicate run launch")?;
                    launches.insert(intent_id, p.clone());
                } else {
                    let launch = launches
                        .get(&intent_id)
                        .ok_or_else(|| fail("run has no launch"))?;
                    require(
                        launch["ownerEpoch"] == p["ownerEpoch"],
                        "run launch ownership mismatch",
                    )?;
                    if kind == "run.prepared" {
                        require(
                            !preparations.contains_key(&intent_id),
                            "duplicate run preparation",
                        )?;
                        preparations.insert(intent_id, p.clone());
                    } else {
                        require(!facts.contains_key(&intent_id), "duplicate start outcome")?;
                        if kind == "run.start_unknown" {
                            let next = changed(item, "blocked", None);
                            validate_work_item(&next)?;
                            items.insert(wi, next);
                        } else {
                            let run = &p["run"];
                            let preparation = preparations
                                .get(&intent_id)
                                .ok_or_else(|| fail("start has no preparation"))?;
                            let turn = turns
                                .get(&intent_id)
                                .ok_or_else(|| fail("start has no turn request"))?;
                            let run_id = id(run, "runId");
                            let session = id(run, "providerSessionId");
                            require(
                                run_id == format!("run:{intent_id}")
                                    && preparation["ownerEpoch"] == p["ownerEpoch"]
                                    && turn["ownerEpoch"] == p["ownerEpoch"]
                                    && run["ownerEpoch"] == p["ownerEpoch"]
                                    && preparation["providerSessionId"] == run["providerSessionId"]
                                    && turn["providerSessionId"] == run["providerSessionId"]
                                    && !runs.contains_key(&run_id)
                                    && !sessions.contains_key(&session),
                                "invalid started run",
                            )?;
                            runs.insert(run_id.clone(), run.clone());
                            sessions.insert(session, run_id);
                            let next = changed(item, string(&run["status"])?, None);
                            validate_work_item(&next)?;
                            items.insert(wi, next);
                        }
                        facts.insert(intent_id, kind.into());
                        revision_value += 1;
                    }
                }
            }
            "turn.requested" => {
                let preparation = preparations
                    .get(&intent_id)
                    .ok_or_else(|| fail("turn has no preparation"))?;
                require(
                    !turns.contains_key(&intent_id)
                        && preparation["providerSessionId"] == p["providerSessionId"]
                        && preparation["ownerEpoch"] == p["ownerEpoch"],
                    "invalid turn request",
                )?;
                turns.insert(intent_id, p.clone());
            }
            "run.session_lost" => {
                let run = runs
                    .get(&id(p, "runId"))
                    .ok_or_else(|| fail("lost run missing"))?
                    .clone();
                let wi = id(&run, "workItemId");
                let item = items
                    .get(&wi)
                    .ok_or_else(|| fail("lost run item missing"))?
                    .clone();
                require(
                    run["ownerEpoch"] == p["ownerEpoch"] && run["status"] == "running",
                    "invalid session loss",
                )?;
                let next_run = changed(run, "failed", Some(&e["recordedAt"]));
                validate_run(&next_run)?;
                runs.insert(id(&next_run, "runId"), next_run);
                let next = changed(item, "blocked", None);
                validate_work_item(&next)?;
                items.insert(wi, next);
                revision_value += 1;
            }
            "run.interrupted" => {
                let intent = intents
                    .get(&intent_id)
                    .ok_or_else(|| fail("interruption has no intent"))?;
                let run = runs
                    .get(&id(p, "runId"))
                    .ok_or_else(|| fail("interrupted run missing"))?
                    .clone();
                let wi = id(&run, "workItemId");
                let item = items
                    .get(&wi)
                    .ok_or_else(|| fail("interrupted item missing"))?
                    .clone();
                let snapshot = cancel_snapshots
                    .get(&intent_id)
                    .ok_or_else(|| fail("cancel snapshot missing"))?;
                let transitions =
                    run["status"] == "running" && run["revision"] == p["expectedRunRevision"];
                let already_interrupted = run["status"] == "interrupted"
                    && run["revision"].as_u64() == p["expectedRunRevision"].as_u64().map(|n| n + 1);
                require(
                    intent["type"] == "cancel_run"
                        && !facts.contains_key(&intent_id)
                        && intent["payload"]["runId"] == p["runId"]
                        && intent["payload"]["expectedRunRevision"] == p["expectedRunRevision"]
                        && snapshot["status"] == "running"
                        && snapshot["revision"] == p["expectedRunRevision"]
                        && (transitions || already_interrupted),
                    "invalid interrupt transition",
                )?;
                if transitions {
                    let next_run = changed(run, "interrupted", Some(&e["recordedAt"]));
                    validate_run(&next_run)?;
                    runs.insert(id(&next_run, "runId"), next_run);
                    let next = changed(item, "interrupted", None);
                    validate_work_item(&next)?;
                    items.insert(wi, next);
                    revision_value += 1;
                }
                facts.insert(intent_id, kind.into());
            }
            "command.receipted" => {
                let receipt = &p["receipt"];
                let i = id(receipt, "intentId");
                let intent = intents
                    .get(&i)
                    .ok_or_else(|| fail("receipt has no request"))?;
                let fact = facts.get(&i).map(String::as_str);
                let command = string(&intent["type"])?;
                let wi = match command {
                    "enqueue_work" => intent["payload"]["workItem"]["workItemId"].clone(),
                    "cancel_run" => runs
                        .get(&id(&intent["payload"], "runId"))
                        .map(|r| r["workItemId"].clone())
                        .unwrap_or(Value::Null),
                    _ => intent["payload"]["workItemId"].clone(),
                };
                let item = wi.as_str().and_then(|wi| items.get(wi));
                let run = if command == "cancel_run" {
                    runs.get(&id(&intent["payload"], "runId"))
                } else {
                    None
                };
                let reason = string(&receipt["reasonCode"])?;
                let rejected = match command {
                    "enqueue_work" => {
                        reason == "WORK_ITEM_EXISTS"
                            && wi.as_str().is_some_and(|wi| seen_items.contains(wi))
                    }
                    "remove_queued_work" => {
                        let start_pending = pending(&intents, &facts, &receipted, |v| {
                            v["type"] == "start_work" && v["payload"]["workItemId"] == wi
                        });
                        (reason == "WORK_ITEM_NOT_FOUND" && item.is_none())
                            || (reason == "WORK_ITEM_NOT_QUEUED"
                                && item.is_some_and(|v| v["status"] != "queued" || start_pending))
                            || (reason == "STALE_ITEM_REVISION"
                                && item.is_some_and(|v| {
                                    v["status"] == "queued"
                                        && v["revision"]
                                            != intent["payload"]["expectedItemRevision"]
                                }))
                    }
                    "start_work" => {
                        let another = pending(&intents, &facts, &receipted, |v| {
                            v["intentId"] != intent["intentId"]
                                && v["type"] == "start_work"
                                && v["payload"]["workItemId"] == wi
                        });
                        ["PROVIDER_CONTROL_UNAVAILABLE", "WORKSPACE_NOT_ALLOWED"].contains(&reason)
                            || (reason == "WORK_ITEM_NOT_FOUND" && item.is_none())
                            || (reason == "WORK_ITEM_NOT_QUEUED"
                                && item.is_some_and(|v| v["status"] != "queued"))
                            || (reason == "WORK_START_PENDING" && another)
                            || (reason == "STALE_ITEM_REVISION"
                                && item.is_some_and(|v| {
                                    v["status"] == "queued"
                                        && v["revision"]
                                            != intent["payload"]["expectedItemRevision"]
                                }))
                    }
                    "cancel_run" => {
                        let another = pending(&intents, &facts, &receipted, |v| {
                            v["intentId"] != intent["intentId"]
                                && v["type"] == "cancel_run"
                                && v["payload"]["runId"] == intent["payload"]["runId"]
                        });
                        reason == "PROVIDER_CONTROL_UNAVAILABLE"
                            || (reason == "RUN_NOT_FOUND" && run.is_none())
                            || (reason == "RUN_NOT_ACTIVE"
                                && run.is_some_and(|r| r["status"] != "running"))
                            || (reason == "RUN_CANCEL_PENDING" && another)
                            || (reason == "STALE_RUN_REVISION"
                                && run.is_some_and(|r| {
                                    r["status"] == "running"
                                        && r["revision"] != intent["payload"]["expectedRunRevision"]
                                }))
                    }
                    _ => false,
                };
                let applied_reason = match fact {
                    Some("work.enqueued") => "WORK_ENQUEUED",
                    Some("work.removed") => "WORK_REMOVED",
                    Some("run.started") => "WORK_STARTED",
                    Some("run.interrupted") => "RUN_CANCELLED",
                    _ => "",
                };
                let outcome = string(&receipt["outcome"])?;
                let outcome_valid = (outcome == "applied"
                    && fact.is_some()
                    && reason == applied_reason)
                    || (outcome == "rejected"
                        && fact.is_none()
                        && string(&receipt["completedAt"])? < string(&intent["expiresAt"])?
                        && rejected)
                    || (outcome == "expired"
                        && fact.is_none()
                        && string(&receipt["completedAt"])? >= string(&intent["expiresAt"])?)
                    || (outcome == "failed"
                        && ((fact.is_none()
                            && ["PROVIDER_CONTROL_FAILED", "RUN_SESSION_LOST"].contains(&reason))
                            || (fact == Some("run.start_unknown")
                                && reason == "START_OUTCOME_UNKNOWN")));
                require(
                    intent["idempotencyKey"] == receipt["idempotencyKey"]
                        && intent["type"] == receipt["commandType"]
                        && !receipted.contains(&i)
                        && receipt["workProjectionRevision"] == json!(revision_value)
                        && outcome_valid
                        && receipt["workItemId"] == wi,
                    "invalid command receipt outcome",
                )?;
                receipts.push(receipt.clone());
                receipted.insert(i);
            }
            _ => return Err(fail("unsupported work event")),
        }
    }
    if !found {
        return Ok(Value::Null);
    }
    for intent in intents.values() {
        let i = id(intent, "intentId");
        if facts.contains_key(&i) || receipted.contains(&i) {
            continue;
        }
        if intent["type"] == "start_work" {
            if let Some(item) = items.get_mut(&id(&intent["payload"], "workItemId")) {
                if item["status"] == "queued" {
                    item["status"] = json!("launching");
                }
            }
        } else if intent["type"] == "cancel_run" {
            if let Some(run) = runs.get_mut(&id(&intent["payload"], "runId")) {
                if run["status"] == "running" {
                    run["status"] = json!("cancelling");
                }
                if let Some(item) = items.get_mut(&id(run, "workItemId")) {
                    if item["status"] == "running" {
                        item["status"] = json!("cancelling");
                    }
                }
            }
        }
    }
    let mut item_values: Vec<_> = items.into_values().collect();
    sort_values(&mut item_values, &["createdAt", "workItemId"], false);
    let mut run_values: Vec<_> = runs.into_values().collect();
    sort_values(&mut run_values, &["startedAt", "runId"], false);
    Ok(
        json!({"schemaVersion":1,"revision":revision_value,"items":item_values,"runs":run_values,"receipts":receipts}),
    )
}

fn project_workspaces(events: &[Value]) -> Result<Value> {
    let mut intents = Values::new();
    let mut keys = HashSet::new();
    let mut items = Values::new();
    let mut paths: BTreeMap<String, String> = BTreeMap::new();
    let mut facts: BTreeMap<String, String> = BTreeMap::new();
    let mut snapshots = Values::new();
    let mut receipts = Vec::new();
    let mut receipted = HashSet::new();
    let mut revision_value = 0u64;
    let mut found = false;
    for e in events {
        let kind = string(&e["type"])?;
        if !kind.starts_with("workspace.") {
            continue;
        }
        found = true;
        let p = &e["payload"];
        let i = id(p, "intentId");
        match kind {
            "workspace.command.requested" => {
                let intent = &p["intent"];
                let i = id(intent, "intentId");
                require(
                    !intents.contains_key(&i) && keys.insert(id(intent, "idempotencyKey")),
                    "duplicate workspace command",
                )?;
                if intent["type"] == "remove_workspace" {
                    snapshots.insert(
                        i.clone(),
                        items
                            .get(&id(&intent["payload"], "workspaceId"))
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                }
                intents.insert(i, intent.clone());
            }
            "workspace.registered" => {
                let intent = intents
                    .get(&i)
                    .ok_or_else(|| fail("workspace registration has no request"))?;
                let workspace = &p["workspace"];
                let wi = id(workspace, "workspaceId");
                let path = id(workspace, "workingDirectory");
                require(
                    intent["type"] == "register_workspace"
                        && !facts.contains_key(&i)
                        && intent["payload"]["workspace"] == *workspace
                        && !items.contains_key(&wi)
                        && !paths.contains_key(&path),
                    "invalid workspace registration",
                )?;
                items.insert(wi.clone(), workspace.clone());
                paths.insert(path, wi);
                facts.insert(i, kind.into());
                revision_value += 1;
            }
            "workspace.removed" => {
                let intent = intents
                    .get(&i)
                    .ok_or_else(|| fail("workspace removal has no request"))?;
                let wi = id(p, "workspaceId");
                let workspace = items
                    .get(&wi)
                    .ok_or_else(|| fail("removed workspace missing"))?;
                require(
                    intent["type"] == "remove_workspace"
                        && !facts.contains_key(&i)
                        && intent["payload"]["workspaceId"] == p["workspaceId"]
                        && intent["payload"]["expectedWorkspaceRevision"]
                            == p["expectedWorkspaceRevision"]
                        && workspace["revision"] == p["expectedWorkspaceRevision"],
                    "invalid workspace removal",
                )?;
                paths.remove(&id(workspace, "workingDirectory"));
                items.remove(&wi);
                facts.insert(i, kind.into());
                revision_value += 1;
            }
            "workspace.command.receipted" => {
                let receipt = &p["receipt"];
                let i = id(receipt, "intentId");
                let intent = intents
                    .get(&i)
                    .ok_or_else(|| fail("workspace receipt has no request"))?;
                let fact = facts.get(&i).map(String::as_str);
                let outcome = string(&receipt["outcome"])?;
                let reason = string(&receipt["reasonCode"])?;
                let register = intent["type"] == "register_workspace";
                let wi = if register {
                    &intent["payload"]["workspace"]["workspaceId"]
                } else {
                    &intent["payload"]["workspaceId"]
                };
                let valid = if register {
                    let existing = items.values().find(|v| {
                        v["workingDirectory"] == intent["payload"]["workspace"]["workingDirectory"]
                    });
                    (outcome == "applied"
                        && fact == Some("workspace.registered")
                        && reason == "WORKSPACE_REGISTERED"
                        && receipt["workspaceId"] == *wi)
                        || (outcome == "rejected"
                            && fact.is_none()
                            && reason == "WORKSPACE_ALREADY_REGISTERED"
                            && existing.is_some_and(|v| v["workspaceId"] == receipt["workspaceId"]))
                } else {
                    let snapshot = snapshots.get(&i).unwrap_or(&Value::Null);
                    (outcome == "applied"
                        && fact == Some("workspace.removed")
                        && reason == "WORKSPACE_REMOVED"
                        && receipt["workspaceId"] == *wi)
                        || (outcome == "rejected"
                            && fact.is_none()
                            && reason == "WORKSPACE_NOT_FOUND"
                            && snapshot.is_null()
                            && receipt["workspaceId"] == *wi)
                        || (outcome == "rejected"
                            && fact.is_none()
                            && reason == "STALE_WORKSPACE_REVISION"
                            && snapshot["revision"]
                                != intent["payload"]["expectedWorkspaceRevision"]
                            && receipt["workspaceId"] == *wi)
                };
                let expired = outcome == "expired"
                    && fact.is_none()
                    && reason == "COMMAND_EXPIRED"
                    && string(&receipt["completedAt"])? >= string(&intent["expiresAt"])?
                    && receipt["workspaceId"] == *wi;
                require(
                    intent["idempotencyKey"] == receipt["idempotencyKey"]
                        && intent["type"] == receipt["commandType"]
                        && !receipted.contains(&i)
                        && receipt["workspaceProjectionRevision"] == json!(revision_value)
                        && (valid || expired),
                    "invalid workspace receipt outcome",
                )?;
                receipts.push(receipt.clone());
                receipted.insert(i);
            }
            _ => return Err(fail("unsupported workspace event")),
        }
    }
    if !found {
        return Ok(Value::Null);
    }
    let mut values: Vec<_> = items.into_values().collect();
    sort_values(&mut values, &["displayName", "workspaceId"], false);
    Ok(json!({"schemaVersion":1,"revision":revision_value,"items":values,"receipts":receipts}))
}
fn compare_versions(left: &Value, right: &Value) -> Result<std::cmp::Ordering> {
    // Semver components are arbitrary decimal strings in the legacy schema.
    // Compare numerically without introducing an integer overflow limit.
    for (a, b) in string(left)?.split('.').zip(string(right)?.split('.')) {
        let ord = a.len().cmp(&b.len()).then(a.cmp(b));
        if !ord.is_eq() {
            return Ok(ord);
        }
    }
    Ok(std::cmp::Ordering::Equal)
}
fn project_packs(events: &[Value]) -> Result<Value> {
    let mut items = Values::new();
    let mut revision_value = 0u64;
    let mut found = false;
    for e in events {
        let kind = string(&e["type"])?;
        if !kind.starts_with("agent_pack.") {
            continue;
        }
        found = true;
        let p = &e["payload"];
        if kind == "agent_pack.installed" {
            let pack = &p["pack"];
            let pi = id(pack, "id");
            let current = items.get(&pi);
            if let Some(c) = current {
                require(
                    compare_versions(&pack["version"], &c["pack"]["version"])?
                        == std::cmp::Ordering::Greater,
                    "pack version must increase",
                )?;
            }
            let revision = current
                .map(|v| v["revision"].as_u64().unwrap())
                .unwrap_or(0)
                + 1;
            items.insert(
                pi,
                json!({"pack":pack,"revision":revision,"installedAt":e["recordedAt"]}),
            );
        } else {
            let pi = id(p, "packId");
            require(
                items
                    .get(&pi)
                    .is_some_and(|v| v["revision"] == p["expectedRevision"]),
                "invalid pack removal revision",
            )?;
            items.remove(&pi);
        }
        revision_value += 1;
    }
    if !found {
        return Ok(Value::Null);
    }
    let mut values: Vec<_> = items.into_values().collect();
    values.sort_by(|a, b| {
        locale_cmp(
            a["pack"]["name"].as_str().unwrap(),
            b["pack"]["name"].as_str().unwrap(),
        )
        .then_with(|| {
            locale_cmp(
                a["pack"]["id"].as_str().unwrap(),
                b["pack"]["id"].as_str().unwrap(),
            )
        })
    });
    Ok(json!({"schemaVersion":1,"revision":revision_value,"items":values}))
}

struct TeamState {
    team: Value,
    revision: u64,
    started_at: Value,
    cancelled_at: Value,
    timed_out_at: Value,
    cancelled: bool,
    timed_out: bool,
    cancelled_agents: HashSet<String>,
    retries: BTreeMap<String, u64>,
    approvals: Values,
    answers: Values,
}
impl TeamState {
    fn new(team: Value) -> Self {
        Self {
            team,
            revision: 1,
            started_at: Value::Null,
            cancelled_at: Value::Null,
            timed_out_at: Value::Null,
            cancelled: false,
            timed_out: false,
            cancelled_agents: HashSet::new(),
            retries: BTreeMap::new(),
            approvals: Values::new(),
            answers: Values::new(),
        }
    }
}
fn derive_team(state: &TeamState, work: &Value) -> Result<Value> {
    let team = &state.team;
    let work_items: Values = work["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| (id(v, "workItemId"), v.clone()))
        .collect();
    let runs: Values = work["runs"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| (id(v, "workItemId"), v.clone()))
        .collect();
    let definitions: Values = array(&team["tasks"])?
        .iter()
        .map(|v| (id(v, "taskId"), v.clone()))
        .collect();
    let mut derived = Values::new();
    fn derive(
        task: &Value,
        team: &Value,
        state: &TeamState,
        work_items: &Values,
        runs: &Values,
        definitions: &Values,
        derived: &mut Values,
    ) -> Result<Value> {
        let ti = id(task, "taskId");
        if let Some(v) = derived.get(&ti) {
            return Ok(v.clone());
        }
        let attempt = state.retries.get(&ti).copied().unwrap_or(0) + 1;
        let wi = team_work_item_id(string(&team["teamId"])?, string(&task["taskId"])?, attempt)?;
        let item = work_items.get(&wi);
        let mut dependencies = Vec::new();
        for d in array(&task["dependsOn"])? {
            dependencies.push(derive(
                &definitions[string(d)?],
                team,
                state,
                work_items,
                runs,
                definitions,
                derived,
            )?);
        }
        let question = &task["question"];
        let approval = state.approvals.get(&ti);
        let answer = if question.is_null() {
            None
        } else {
            state.answers.get(&id(question, "questionId"))
        };
        let status = if state.cancelled
            || state.timed_out
            || state.cancelled_agents.contains(&id(task, "agentId"))
        {
            "cancelled"
        } else if dependencies
            .iter()
            .any(|d| ["failed", "blocked", "cancelled"].contains(&d["status"].as_str().unwrap()))
        {
            "blocked"
        } else if dependencies.iter().any(|d| d["status"] != "completed") {
            "pending"
        } else if !question.is_null() && answer.is_none() {
            "waiting_question"
        } else if task["approvalRequired"] == true && approval.is_none() {
            "waiting_approval"
        } else if approval.is_some_and(|a| a["decision"] == "rejected") {
            "cancelled"
        } else {
            item.and_then(|v| v["status"].as_str()).unwrap_or("ready")
        };
        let agent = array(&team["agents"])?
            .iter()
            .find(|a| a["agentId"] == task["agentId"])
            .ok_or_else(|| fail("task agent missing"))?;
        let max_attempts = agent["pack"]["limits"]["maxAttempts"]
            .as_u64()
            .unwrap()
            .min(team["settings"]["retryLimit"].as_u64().unwrap() + 1);
        let mut result = task.clone();
        result["status"] = json!(status);
        result["attempt"] = json!(attempt);
        result["maxAttempts"] = json!(max_attempts);
        result["canRetry"] =
            json!(["failed", "interrupted"].contains(&status) && attempt < max_attempts);
        result["workItemId"] = if item.is_some() {
            json!(wi)
        } else {
            Value::Null
        };
        result["runId"] = runs
            .get(&wi)
            .map(|v| v["runId"].clone())
            .unwrap_or(Value::Null);
        result["approval"] = approval.cloned().unwrap_or(Value::Null);
        result["answer"] = answer.cloned().unwrap_or(Value::Null);
        derived.insert(ti, result.clone());
        Ok(result)
    }
    let mut tasks = Vec::new();
    for task in array(&team["tasks"])? {
        tasks.push(derive(
            task,
            team,
            state,
            &work_items,
            &runs,
            &definitions,
            &mut derived,
        )?);
    }
    let status = if state.timed_out {
        "timed_out"
    } else if state.cancelled {
        "cancelled"
    } else if state.started_at.is_null() {
        "draft"
    } else if tasks.iter().all(|t| t["status"] == "completed") {
        "completed"
    } else if tasks.iter().any(|t| t["canRetry"] == true) {
        "active"
    } else if tasks.iter().all(|t| {
        ["completed", "failed", "blocked", "cancelled"].contains(&t["status"].as_str().unwrap())
    }) || (team["settings"]["failurePolicy"] == "stop"
        && tasks
            .iter()
            .any(|t| ["failed", "blocked", "cancelled"].contains(&t["status"].as_str().unwrap())))
    {
        "failed"
    } else if tasks
        .iter()
        .any(|t| ["waiting_question", "waiting_approval"].contains(&t["status"].as_str().unwrap()))
    {
        "waiting"
    } else {
        "active"
    };
    let mut agents = Vec::new();
    for agent in array(&team["agents"])? {
        let assigned: Vec<_> = tasks
            .iter()
            .filter(|t| t["agentId"] == agent["agentId"])
            .collect();
        let agent_status = if agent["agentId"] == team["orchestratorAgentId"] {
            status
        } else if state.cancelled_agents.contains(&id(agent, "agentId")) {
            "cancelled"
        } else if assigned.iter().any(|t| {
            ["running", "launching", "cancelling"].contains(&t["status"].as_str().unwrap())
        }) {
            "running"
        } else if assigned.iter().all(|t| t["status"] == "completed") {
            "completed"
        } else if assigned
            .iter()
            .any(|t| ["failed", "blocked"].contains(&t["status"].as_str().unwrap()))
        {
            "failed"
        } else {
            "waiting"
        };
        let mut a = agent.clone();
        a["status"] = json!(agent_status);
        agents.push(a);
    }
    let revision = state.revision
        + tasks
            .iter()
            .filter_map(|t| {
                t["workItemId"]
                    .as_str()
                    .and_then(|id| work_items.get(id))
                    .and_then(|v| v["revision"].as_u64())
            })
            .sum::<u64>();
    for task in &tasks {
        for attempt in 1..task["attempt"].as_u64().unwrap() {
            let wi =
                team_work_item_id(string(&team["teamId"])?, string(&task["taskId"])?, attempt)?;
            require(
                work_items.get(&wi).is_some_and(|v| {
                    ["failed", "interrupted"].contains(&v["status"].as_str().unwrap_or(""))
                }),
                "retry has no failed attempt",
            )?;
        }
    }
    let mut result = team.clone();
    result["revision"] = json!(revision);
    result["status"] = json!(status);
    result["startedAt"] = state.started_at.clone();
    result["cancelledAt"] = state.cancelled_at.clone();
    result["timedOutAt"] = state.timed_out_at.clone();
    result["agents"] = json!(agents);
    result["tasks"] = json!(tasks);
    Ok(result)
}
fn project_teams(events: &[Value], work: &Value) -> Result<Value> {
    let mut states: BTreeMap<String, TeamState> = BTreeMap::new();
    let mut revision_value = 0u64;
    let mut found = false;
    for e in events {
        let kind = string(&e["type"])?;
        if !kind.starts_with("team.") {
            continue;
        }
        found = true;
        let p = &e["payload"];
        if kind == "team.created" {
            let ti = id(&p["team"], "teamId");
            require(!states.contains_key(&ti), "duplicate team")?;
            states.insert(ti, TeamState::new(p["team"].clone()));
            continue;
        }
        let state = states
            .get_mut(&id(p, "teamId"))
            .ok_or_else(|| fail("team event has no team"))?;
        match kind {
            "team.started" => {
                require(
                    state.started_at.is_null() && !state.cancelled,
                    "invalid team start",
                )?;
                state.started_at = e["recordedAt"].clone();
            }
            "team.cancelled" => {
                require(
                    !state.cancelled && !state.timed_out,
                    "duplicate team cancellation",
                )?;
                state.cancelled = true;
                state.cancelled_at = e["recordedAt"].clone();
            }
            "team.timed_out" => {
                require(
                    !state.cancelled && !state.timed_out && !state.started_at.is_null(),
                    "invalid team timeout",
                )?;
                state.timed_out = true;
                state.timed_out_at = e["recordedAt"].clone();
            }
            "team.agent.cancelled" => {
                require(
                    array(&state.team["agents"])?
                        .iter()
                        .any(|v| v["agentId"] == p["agentId"])
                        && p["agentId"] != state.team["orchestratorAgentId"]
                        && state.cancelled_agents.insert(id(p, "agentId")),
                    "invalid agent cancellation",
                )?;
            }
            "team.task.retry_requested" => {
                let task = array(&state.team["tasks"])?
                    .iter()
                    .find(|v| v["taskId"] == p["taskId"])
                    .ok_or_else(|| fail("retry task missing"))?;
                let agent = array(&state.team["agents"])?
                    .iter()
                    .find(|v| v["agentId"] == task["agentId"])
                    .ok_or_else(|| fail("retry agent missing"))?;
                let ti = id(p, "taskId");
                let current = state.retries.get(&ti).copied().unwrap_or(0);
                require(
                    p["previousAttempt"] == json!(current + 1)
                        && current
                            < state.team["settings"]["retryLimit"]
                                .as_u64()
                                .unwrap()
                                .min(agent["pack"]["limits"]["maxAttempts"].as_u64().unwrap() - 1),
                    "invalid retry limit",
                )?;
                state.retries.insert(ti, current + 1);
            }
            "team.approval.decided" => {
                let task = array(&state.team["tasks"])?
                    .iter()
                    .find(|v| v["taskId"] == p["taskId"])
                    .ok_or_else(|| fail("approval task missing"))?;
                let ti = id(p, "taskId");
                require(
                    task["approvalRequired"] == true && !state.approvals.contains_key(&ti),
                    "invalid team approval",
                )?;
                state.approvals.insert(
                    ti,
                    json!({"decision":p["decision"],"note":p["note"],"decidedAt":e["recordedAt"]}),
                );
            }
            "team.question.answered" => {
                let task = array(&state.team["tasks"])?
                    .iter()
                    .find(|v| {
                        !v["question"].is_null() && v["question"]["questionId"] == p["questionId"]
                    })
                    .ok_or_else(|| fail("answer question missing"))?;
                let qi = id(p, "questionId");
                require(
                    !state.answers.contains_key(&qi)
                        && string(&p["answer"])?.encode_utf16().count() as u64
                            <= task["question"]["maxAnswerLength"].as_u64().unwrap(),
                    "invalid question answer",
                )?;
                state.answers.insert(
                    qi,
                    json!({"answer":p["answer"],"answeredAt":e["recordedAt"]}),
                );
            }
            _ => return Err(fail("unsupported team event")),
        }
        state.revision += 1;
        revision_value += 1;
    }
    if !found {
        return Ok(Value::Null);
    }
    let mut items = Vec::new();
    for state in states.values() {
        items.push(derive_team(state, work)?);
    }
    sort_values(&mut items, &["createdAt", "teamId"], true);
    Ok(json!({"schemaVersion":1,"revision":revision_value,"items":items}))
}

/// Replay all projections from canonical events, rejecting unknown complete
/// records, duplicate identifiers and facts that lack their required request.
pub(crate) fn project_legacy_events(events: &[Value]) -> Result<Value> {
    let mut seen = HashSet::new();
    for e in events {
        validate_event(e)?;
        require(seen.insert(string(&e["id"])?), "duplicate event id")?;
    }
    validate_extended_replay(events)?;
    let observation = project_observation(events)?;
    let work = project_work(events)?;
    let workspaces = project_workspaces(events)?;
    let packs = project_packs(events)?;
    let teams = project_teams(events, &work)?;
    Ok(
        json!({"observation":observation,"work":work,"workspaces":workspaces,"packs":packs,"teams":teams}),
    )
}

pub fn project_events(events: &[Value]) -> Result<Value> {
    let mut bundle = project_legacy_events(events)?;
    bundle["projectMemory"] = crate::project_memory::project_validated(events)?;
    Ok(bundle)
}
