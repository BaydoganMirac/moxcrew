//! Local Project resources, immutable revisions and typed production jobs.
#[path = "studio_capabilities.rs"]
mod capabilities;
#[path = "delivery.rs"]
mod delivery;

use crate::{
    agent_memory, conversation, creation, domain, event, production, store,
    work::{array, text},
    Error, Host, Result,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

const FILE_LIMIT: u64 = 25 * 1024 * 1024;
fn fail(s: &str) -> Error {
    Error(s.into())
}
fn require(b: bool) -> Result<()> {
    if b {
        Ok(())
    } else {
        Err(fail("PROJECT_STUDIO_INVALID"))
    }
}
fn exact(v: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    let o = v
        .as_object()
        .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
    require(
        required.iter().all(|k| o.contains_key(*k))
            && o.keys()
                .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str())),
    )
}
fn bounded(v: &Value, n: usize) -> Result<&str> {
    v.as_str()
        .filter(|s| !s.is_empty() && s.len() <= n && !s.contains('\0'))
        .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))
}
fn id<'a>(v: &'a Value, prefix: &str) -> Result<&'a str> {
    let s = bounded(v, 100)?;
    let suffix = s
        .strip_prefix(prefix)
        .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
    require(uuid::Uuid::parse_str(suffix).is_ok())?;
    Ok(s)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash(v: &Value) -> Result<&str> {
    let s = bounded(v, 64)?;
    require(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
    )?;
    Ok(s)
}
fn fact(kind: &str, p: Value) -> Value {
    let mut e = event(kind, p);
    e["schemaVersion"] = json!(12);
    e
}
fn review_fact(kind: &str, p: Value) -> Value {
    let mut e = event(kind, p);
    e["schemaVersion"] = json!(16);
    e
}
pub(crate) fn action_fields(action: &str) -> Option<&'static [&'static str]> {
    if let Some(fields) = capabilities::action_fields(action) {
        return Some(fields);
    }
    Some(match action {
        "project_studio_read" => &["projectId"],
        "project_delivery_export" => &["projectId", "files", "targetPath"],
        "project_artifact_read" => &["projectId", "workItemId", "path", "expectedSha256"],
        "project_artifact_review_set" => &[
            "projectId",
            "workItemId",
            "path",
            "expectedSha256",
            "status",
            "note",
        ],
        "project_artifact_revise" => &[
            "projectId",
            "expectedProjectRevision",
            "workItemId",
            "path",
            "expectedSha256",
            "sectionId",
            "instruction",
            "providerId",
        ],
        "project_resource_import" => &["projectId", "expectedProjectRevision", "sourcePath"],
        "project_resource_remove" => &["projectId", "resourceId", "expectedResourceRevision"],
        "project_tool_configure" => &["projectId", "toolId", "executablePath"],
        "project_connection_set" => &["projectId", "providerId", "label", "accessToken"],
        "project_connection_remove" => &["projectId", "providerId"],
        "project_connection_prepare" => &["projectId", "providerId", "operation", "arguments"],
        "project_connection_execute" => {
            &["projectId", "preparedActionId", "expectedActionRevision"]
        }
        _ => return None,
    })
}
pub(crate) fn is_read(action: &str) -> bool {
    capabilities::is_read(action)
        || matches!(action, "project_studio_read" | "project_artifact_read")
}
pub(crate) fn sensitive(action: &str) -> bool {
    capabilities::sensitive(action)
        || matches!(
            action,
            "project_connection_set" | "project_connection_remove"
        )
}

#[derive(Default)]
pub(crate) struct Runtime {
    jobs: BTreeMap<String, Active>,
}
struct Active {
    active: Arc<AtomicBool>,
    binding: agent_memory::Binding,
    handle: tokio::task::JoinHandle<Result<production::Output>>,
}

pub(crate) fn validate_event(e: &Value) -> Result<()> {
    if e["schemaVersion"] == 15 || e["schemaVersion"] == 16 {
        return validate_review_event(e, e["schemaVersion"] == 16);
    }
    if capabilities::validate_event(e)? {
        return Ok(());
    }
    let p = &e["payload"];
    match e["type"].as_str().unwrap_or("") {
        "owner.command.requested" => {
            exact(p, &["commandId", "action", "payload"], &[])?;
            id(&p["commandId"], "cmd:")?;
            let a = bounded(&p["action"], 80)?;
            require(
                (a != "project_artifact_review_set"
                    && action_fields(a).is_some()
                    && !is_read(a)
                    && !sensitive(a))
                    || a == "project_result_export",
            )?;
            let fields = if a == "project_result_export" {
                &[
                    "projectId",
                    "workItemId",
                    "path",
                    "targetPath",
                    "expectedSha256",
                ][..]
            } else {
                action_fields(a).unwrap()
            };
            exact(
                &p["payload"],
                &[],
                &[fields, &["commandId", "createdAt", "expiresAt"]].concat(),
            )?;
            require(p["payload"]["commandId"] == p["commandId"])?;
            if a == "project_delivery_export" {
                delivery::validate_request(&p["payload"])?;
            }
        }
        "owner.command.receipted" => {
            exact(p, &["commandId", "result"], &[])?;
            id(&p["commandId"], "cmd:")?;
            require(p["result"].is_object())?;
        }
        "project_studio.resource_imported" => {
            exact(p, &["projectId", "commandId", "resource"], &[])?;
            id(&p["projectId"], "project:")?;
            id(&p["commandId"], "cmd:")?;
            resource(&p["resource"])?;
            require(p["resource"]["revision"] == 1 && p["resource"]["status"] == "available")?;
        }
        "project_studio.resource_removed" => {
            exact(
                p,
                &[
                    "projectId",
                    "commandId",
                    "resourceId",
                    "expectedResourceRevision",
                ],
                &[],
            )?;
            id(&p["projectId"], "project:")?;
            id(&p["commandId"], "cmd:")?;
            id(&p["resourceId"], "resource:")?;
            require(
                p["expectedResourceRevision"]
                    .as_u64()
                    .is_some_and(|n| n > 0),
            )?;
        }
        "project_studio.revision_requested" => {
            exact(
                p,
                &[
                    "projectId",
                    "commandId",
                    "workItemId",
                    "parentWorkItemId",
                    "path",
                    "sha256",
                    "sectionId",
                    "inputManifest",
                ],
                &[],
            )?;
            id(&p["projectId"], "project:")?;
            id(&p["commandId"], "cmd:")?;
            id(&p["workItemId"], "work:")?;
            id(&p["parentWorkItemId"], "work:")?;
            creation::relative_result(bounded(&p["path"], 1024)?)?;
            hash(&p["sha256"])?;
            require(
                p["sectionId"].is_null() || p["sectionId"].as_str().is_some_and(|s| s.len() <= 80),
            )?;
            require(p["inputManifest"].is_object())?;
        }
        "project_studio.production_queued" => {
            exact(
                p,
                &[
                    "jobId",
                    "projectId",
                    "runId",
                    "workItemId",
                    "ownerEpoch",
                    "nonce",
                    "requestHash",
                    "operation",
                ],
                &[],
            )?;
            id(&p["jobId"], "production:")?;
            id(&p["projectId"], "project:")?;
            id(&p["workItemId"], "work:")?;
            for k in ["runId", "ownerEpoch", "nonce"] {
                bounded(&p[k], 128)?;
            }
            hash(&p["requestHash"])?;
            require(production::OPERATIONS.contains(&bounded(&p["operation"], 60)?))?;
        }
        "project_studio.production_finished" => {
            exact(p, &["jobId", "outcome", "reasonCode", "artifacts"], &[])?;
            id(&p["jobId"], "production:")?;
            require(
                ["completed", "failed", "cancelled", "interrupted"]
                    .contains(&bounded(&p["outcome"], 30)?),
            )?;
            bounded(&p["reasonCode"], 100)?;
            let artifacts = p["artifacts"]
                .as_array()
                .ok_or_else(|| fail("PROJECT_STUDIO_INVALID"))?;
            require(artifacts.len() <= 16)?;
            for a in artifacts {
                artifact(a)?;
            }
            require(p["outcome"] == "completed" || artifacts.is_empty())?;
        }
        "project_studio.connection_prepared" => {
            exact(
                p,
                &[
                    "projectId",
                    "commandId",
                    "preparedActionId",
                    "revision",
                    "providerId",
                    "operation",
                    "arguments",
                    "connectionRevision",
                ],
                &[],
            )?;
            id(&p["projectId"], "project:")?;
            id(&p["commandId"], "cmd:")?;
            id(&p["preparedActionId"], "connection-action:")?;
            require(p["revision"] == 1 && p["connectionRevision"].as_u64().is_some_and(|n| n > 0))?;
            bounded(&p["providerId"], 30)?;
            bounded(&p["operation"], 30)?;
            crate::app_connections::validate_prepare(
                text(p, "providerId")?,
                text(p, "operation")?,
                &p["arguments"],
            )?;
        }
        "project_studio.connection_dispatched" => {
            exact(
                p,
                &[
                    "projectId",
                    "commandId",
                    "preparedActionId",
                    "expectedActionRevision",
                ],
                &[],
            )?;
            id(&p["projectId"], "project:")?;
            id(&p["commandId"], "cmd:")?;
            id(&p["preparedActionId"], "connection-action:")?;
            require(p["expectedActionRevision"] == 1)?;
        }
        "project_studio.connection_finished" => {
            exact(
                p,
                &["preparedActionId", "outcome", "reasonCode", "result"],
                &[],
            )?;
            id(&p["preparedActionId"], "connection-action:")?;
            require(["applied", "failed", "unknown"].contains(&bounded(&p["outcome"], 30)?))?;
            bounded(&p["reasonCode"], 100)?;
            require(p["result"].is_null() || p["result"].is_object())?;
        }
        _ => return Err(fail("PROJECT_STUDIO_EVENT_UNSUPPORTED")),
    }
    require(serde_json::to_vec(p)?.len() <= 512 * 1024)
}
fn validate_review_event(e: &Value, notes: bool) -> Result<()> {
    let p = &e["payload"];
    match e["type"].as_str().unwrap_or("") {
        "owner.command.requested" => {
            exact(p, &["commandId", "action", "payload"], &[])?;
            id(&p["commandId"], "cmd:")?;
            require(p["action"] == "project_artifact_review_set")?;
            let required = [
                "projectId",
                "workItemId",
                "path",
                "expectedSha256",
                "status",
                "commandId",
                "createdAt",
                "expiresAt",
            ];
            exact(
                &p["payload"],
                &required,
                if notes { &["note"] } else { &[] },
            )?;
            require(p["payload"]["commandId"] == p["commandId"])?;
            review_scope(&p["payload"])?;
        }
        "owner.command.receipted" => {
            exact(p, &["commandId", "result"], &[])?;
            id(&p["commandId"], "cmd:")?;
            require(p["result"].is_object())?;
        }
        "project_studio.artifact_reviewed" => {
            let mut required = vec![
                "projectId",
                "commandId",
                "workItemId",
                "path",
                "sha256",
                "status",
            ];
            if notes {
                required.push("note");
            }
            exact(p, &required, &[])?;
            id(&p["commandId"], "cmd:")?;
            let mut scope = json!({
                "projectId":p["projectId"],
                "workItemId":p["workItemId"],
                "path":p["path"],
                "expectedSha256":p["sha256"],
                "status":p["status"]
            });
            if notes {
                scope["note"] = p["note"].clone();
            }
            review_scope(&scope)?;
        }
        _ => return Err(fail("PROJECT_ARTIFACT_REVIEW_EVENT_UNSUPPORTED")),
    }
    require(serde_json::to_vec(p)?.len() <= 32 * 1024)
}
fn review_scope(p: &Value) -> Result<()> {
    id(&p["projectId"], "project:")?;
    id(&p["workItemId"], "work:")?;
    creation::relative_result(bounded(&p["path"], 1024)?)?;
    hash(&p["expectedSha256"])?;
    require(
        p["status"] == "reviewed" || p["status"] == "needs_changes" || p["status"] == "unreviewed",
    )?;
    let note = review_note(p)?;
    require(p["status"] != "unreviewed" || note.is_empty())
}
fn review_note(p: &Value) -> Result<&str> {
    match p.get("note") {
        None => Ok(""),
        Some(note) => note
            .as_str()
            .filter(|note| note.len() <= 4_000 && !note.contains('\0') && note.trim() == *note)
            .ok_or_else(|| fail("PROJECT_STUDIO_INVALID")),
    }
}
fn resource(r: &Value) -> Result<()> {
    exact(
        r,
        &[
            "resourceId",
            "revision",
            "name",
            "kind",
            "bytes",
            "sha256",
            "fileName",
            "status",
        ],
        &[],
    )?;
    id(&r["resourceId"], "resource:")?;
    bounded(&r["name"], 240)?;
    bounded(&r["kind"], 20)?;
    hash(&r["sha256"])?;
    let n = bounded(&r["fileName"], 40)?;
    require(n.starts_with("source.") && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.'))?;
    require(
        r["bytes"].as_u64().is_some_and(|n| n <= FILE_LIMIT)
            && r["revision"].as_u64().is_some_and(|n| n > 0),
    )?;
    Ok(())
}
fn artifact(a: &Value) -> Result<()> {
    exact(
        a,
        &[
            "artifactId",
            "versionId",
            "workItemId",
            "path",
            "name",
            "sha256",
            "bytes",
            "kind",
        ],
        &["parentVersionId"],
    )?;
    id(&a["workItemId"], "work:")?;
    bounded(&a["artifactId"], 200)?;
    hash(&a["versionId"])?;
    hash(&a["sha256"])?;
    require(a["versionId"] == a["sha256"])?;
    creation::relative_result(bounded(&a["path"], 1024)?)?;
    bounded(&a["name"], 240)?;
    bounded(&a["kind"], 20)?;
    require(a["bytes"].as_u64().is_some_and(|n| n <= FILE_LIMIT))
}
fn project_exists(events: &[Value], project: &Value) -> Result<()> {
    require(
        array(&crate::project_memory::project(events)?, "projects")
            .iter()
            .any(|p| p["projectId"] == *project),
    )
}
fn owner_proof(events: &[Value], p: &Value, action: &str) -> Result<Value> {
    let command = events
        .iter()
        .find(|e| {
            e["type"] == "owner.command.requested" && e["payload"]["commandId"] == p["commandId"]
        })
        .ok_or_else(|| fail("PROJECT_STUDIO_MISSING_AUTHORITY"))?;
    require(
        command["schemaVersion"] == 12
            && command["payload"]["action"] == action
            && command["payload"]["payload"]["projectId"] == p["projectId"],
    )?;
    Ok(command["payload"]["payload"].clone())
}

fn owner_review_proof(events: &[Value], p: &Value) -> Result<Value> {
    let command = events
        .iter()
        .find(|e| {
            (e["schemaVersion"] == 15 || e["schemaVersion"] == 16)
                && e["type"] == "owner.command.requested"
                && e["payload"]["commandId"] == p["commandId"]
        })
        .ok_or_else(|| fail("PROJECT_STUDIO_MISSING_AUTHORITY"))?;
    require(
        command["payload"]["action"] == "project_artifact_review_set"
            && command["payload"]["payload"]["projectId"] == p["projectId"],
    )?;
    Ok(command["payload"]["payload"].clone())
}

pub(crate) fn project(events: &[Value]) -> Result<Value> {
    let mut resources: BTreeMap<String, Value> = BTreeMap::new();
    let mut jobs: BTreeMap<String, Value> = BTreeMap::new();
    let mut connections: BTreeMap<String, Value> = BTreeMap::new();
    let mut reviews: BTreeMap<(String, String, String, String), Value> = BTreeMap::new();
    let mut revisions = Vec::new();
    let mut revision = 0;
    for (i, e) in events.iter().enumerate().filter(|(_, e)| {
        (e["schemaVersion"] == 12 || e["schemaVersion"] == 15 || e["schemaVersion"] == 16)
            && e["type"]
                .as_str()
                .is_some_and(|s| s.starts_with("project_studio."))
    }) {
        validate_event(e)?;
        let p = &e["payload"];
        let prior = &events[..i];
        revision += 1;
        if capabilities::replay(prior, e, &mut resources)? {
            continue;
        }
        match e["type"].as_str().unwrap() {
            "project_studio.artifact_reviewed" => {
                project_exists(prior, &p["projectId"])?;
                let c = owner_review_proof(prior, p)?;
                let note_matches = review_note(&c)? == review_note(p)?;
                require(
                    c["workItemId"] == p["workItemId"]
                        && c["path"] == p["path"]
                        && c["expectedSha256"] == p["sha256"]
                        && c["status"] == p["status"]
                        && note_matches,
                )?;
                let key = (
                    text(p, "projectId")?.into(),
                    text(p, "workItemId")?.into(),
                    text(p, "path")?.into(),
                    text(p, "sha256")?.into(),
                );
                if p["status"] == "unreviewed" {
                    reviews.remove(&key);
                } else {
                    let mut review = p.clone();
                    review["reviewedAt"] = e["recordedAt"].clone();
                    reviews.insert(key, review);
                }
            }
            "project_studio.resource_imported" => {
                project_exists(prior, &p["projectId"])?;
                let c = owner_proof(
                    prior,
                    p,
                    if prior.iter().any(|e| {
                        e["type"] == "owner.command.requested"
                            && e["payload"]["commandId"] == p["commandId"]
                            && e["payload"]["action"] == "project_connection_execute"
                    }) {
                        "project_connection_execute"
                    } else {
                        "project_resource_import"
                    },
                )?;
                let _ = c;
                let mut r = p["resource"].clone();
                r["projectId"] = p["projectId"].clone();
                require(
                    resources
                        .insert(text(&r, "resourceId")?.into(), r)
                        .is_none(),
                )?;
            }
            "project_studio.resource_removed" => {
                let c = owner_proof(prior, p, "project_resource_remove")?;
                require(
                    c["resourceId"] == p["resourceId"]
                        && c["expectedResourceRevision"] == p["expectedResourceRevision"],
                )?;
                let r = resources
                    .get_mut(text(p, "resourceId")?)
                    .ok_or_else(|| fail("PROJECT_RESOURCE_NOT_FOUND"))?;
                require(
                    r["projectId"] == p["projectId"]
                        && r["revision"] == p["expectedResourceRevision"]
                        && r["status"] == "available",
                )?;
                r["status"] = json!("removed");
                r["revision"] = json!(r["revision"].as_u64().unwrap() + 1);
            }
            "project_studio.revision_requested" => {
                let c = owner_proof(prior, p, "project_artifact_revise")?;
                let memory = crate::project_memory::project(prior)?;
                require(array(&memory, "bindings").iter().any(|b| {
                    b["subjectKind"] == "work"
                        && b["subjectId"] == p["parentWorkItemId"]
                        && b["projectId"] == p["projectId"]
                }))?;
                require(
                    c["workItemId"] == p["parentWorkItemId"]
                        && c["path"] == p["path"]
                        && c["expectedSha256"] == p["sha256"]
                        && c["sectionId"] == p["sectionId"]
                        && p["workItemId"]
                            == format!("work:{}", text(p, "commandId")?.trim_start_matches("cmd:")),
                )?;
                require(
                    !revisions
                        .iter()
                        .any(|r: &Value| r["workItemId"] == p["workItemId"]),
                )?;
                revisions.push(p.clone());
            }
            "project_studio.production_queued" => {
                project_exists(prior, &p["projectId"])?;
                let w = domain::project_work(prior)?;
                let r = array(&w, "runs")
                    .iter()
                    .find(|r| r["runId"] == p["runId"])
                    .ok_or_else(|| fail("PROJECT_STUDIO_MISSING_AUTHORITY"))?;
                require(
                    r["status"] == "running"
                        && r["workItemId"] == p["workItemId"]
                        && r["ownerEpoch"] == p["ownerEpoch"],
                )?;
                require(prior.iter().any(|e| {
                    e["type"] == "agent_memory.context_bound"
                        && e["payload"]["runId"] == p["runId"]
                        && e["payload"]["projectId"] == p["projectId"]
                        && e["payload"]["ownerEpoch"] == p["ownerEpoch"]
                }))?;
                require(
                    !jobs
                        .values()
                        .any(|j| j["runId"] == p["runId"] && j["nonce"] == p["nonce"]),
                )?;
                let mut j = p.clone();
                j["status"] = json!("running");
                j["createdAt"] = e["recordedAt"].clone();
                require(jobs.insert(text(p, "jobId")?.into(), j).is_none())?;
            }
            "project_studio.production_finished" => {
                let j = jobs
                    .get_mut(text(p, "jobId")?)
                    .ok_or_else(|| fail("PROJECT_STUDIO_MISSING_AUTHORITY"))?;
                require(j["status"] == "running")?;
                for a in array(p, "artifacts") {
                    require(
                        a["workItemId"] == j["workItemId"]
                            && a["path"].as_str().is_some_and(|s| {
                                s.starts_with(&format!(
                                    "outputs/tool-{}/",
                                    text(p, "jobId").unwrap().trim_start_matches("production:")
                                ))
                            }),
                    )?;
                }
                j["status"] = p["outcome"].clone();
                j["reasonCode"] = p["reasonCode"].clone();
                j["artifacts"] = p["artifacts"].clone();
                j["finishedAt"] = e["recordedAt"].clone();
            }
            "project_studio.connection_prepared" => {
                project_exists(prior, &p["projectId"])?;
                let c = owner_proof(prior, p, "project_connection_prepare")?;
                for k in ["providerId", "operation", "arguments"] {
                    require(c[k] == p[k])?;
                }
                let mut a = p.clone();
                a["status"] = json!("prepared");
                require(
                    connections
                        .insert(text(p, "preparedActionId")?.into(), a)
                        .is_none(),
                )?;
            }
            "project_studio.connection_dispatched" => {
                let c = owner_proof(prior, p, "project_connection_execute")?;
                require(
                    c["preparedActionId"] == p["preparedActionId"]
                        && c["expectedActionRevision"] == 1,
                )?;
                let a = connections
                    .get_mut(text(p, "preparedActionId")?)
                    .ok_or_else(|| fail("PROJECT_STUDIO_MISSING_AUTHORITY"))?;
                require(a["status"] == "prepared" && a["projectId"] == p["projectId"])?;
                a["status"] = json!("dispatched");
                a["revision"] = json!(2);
            }
            "project_studio.connection_finished" => {
                let a = connections
                    .get_mut(text(p, "preparedActionId")?)
                    .ok_or_else(|| fail("PROJECT_STUDIO_MISSING_AUTHORITY"))?;
                require(a["status"] == "dispatched")?;
                a["status"] = p["outcome"].clone();
                a["revision"] = json!(3);
                a["reasonCode"] = p["reasonCode"].clone();
                a["result"] = p["result"].clone();
            }
            _ => unreachable!(),
        }
    }
    Ok(
        json!({"revision":revision,"resources":resources.into_values().collect::<Vec<_>>(),"jobs":jobs.into_values().collect::<Vec<_>>(),"revisions":revisions,"connections":connections.into_values().collect::<Vec<_>>(),"reviews":reviews.into_values().collect::<Vec<_>>()}),
    )
}
pub(crate) fn authorized_work(events: &[Value], project: &str, work: &str) -> bool {
    let fact = events.iter().position(|e| {
        e["type"] == "project_studio.revision_requested"
            && e["payload"]["projectId"] == project
            && e["payload"]["workItemId"] == work
    });
    let queue = events.iter().position(|e| {
        e["type"] == "command.requested"
            && e["payload"]["intent"]["payload"]["workItem"]["workItemId"] == work
    });
    matches!((fact,queue),(Some(f),Some(q)) if f<q)
}
pub(crate) fn retry_inputs(
    events: &[Value],
    owner: &Value,
    directory: &Path,
    project: &str,
) -> Result<conversation::FrozenInputs> {
    let revision = events
        .iter()
        .find(|e| {
            e["type"] == "project_studio.revision_requested"
                && e["payload"]["commandId"] == owner["commandId"]
                && e["payload"]["projectId"] == project
        })
        .ok_or_else(|| fail("PROJECT_INPUT_UNAVAILABLE"))?;
    let expected = &revision["payload"]["inputManifest"];
    let manifest =
        conversation::copied_bytes(directory, Path::new("inputs/manifest.json"), 64 * 1024)?;
    let actual: Value = serde_json::from_slice(&manifest)?;
    if actual != *expected {
        return Err(fail("PROJECT_INPUT_CHANGED"));
    }
    let mut files = Vec::new();
    let mut total = 0;
    for entry in array(expected, "files") {
        let path = text(entry, "path")?;
        let name = path
            .strip_prefix("inputs/")
            .filter(|s| !s.is_empty() && !s.contains(['/', '\\']) && !s.starts_with('.'))
            .ok_or_else(|| fail("PROJECT_INPUT_CHANGED"))?;
        let bytes = conversation::copied_bytes(directory, Path::new(path), FILE_LIMIT - total)?;
        total += bytes.len() as u64;
        if digest(&bytes) != entry["sha256"] || entry["bytes"] != bytes.len() {
            return Err(fail("PROJECT_INPUT_CHANGED"));
        }
        files.push((name.to_owned(), bytes));
    }
    require(files.len() == 1)?;
    Ok(conversation::FrozenInputs { files, manifest })
}
pub(crate) fn recover(events: &[Value]) -> Result<Vec<Value>> {
    let s = project(events)?;
    let mut result = Vec::new();
    for j in array(&s, "jobs")
        .iter()
        .filter(|j| j["status"] == "running")
    {
        result.push(fact("project_studio.production_finished",json!({"jobId":j["jobId"],"outcome":"interrupted","reasonCode":"PRODUCTION_INTERRUPTED","artifacts":[]})));
    }
    for a in array(&s, "connections")
        .iter()
        .filter(|a| a["status"] == "dispatched")
    {
        result.push(fact("project_studio.connection_finished",json!({"preparedActionId":a["preparedActionId"],"outcome":"unknown","reasonCode":"PROJECT_CONNECTION_OUTCOME_UNKNOWN","result":null})));
    }
    Ok(result)
}

pub(crate) fn recovered_owner_result(events: &[Value], request: &Value) -> Option<Value> {
    if matches!(
        request["action"].as_str(),
        Some(
            "project_resource_import_batch"
                | "project_resource_from_artifact"
                | "project_resources_from_artifacts"
        )
    ) {
        return capabilities::recovered(events, request);
    }
    let command = &request["commandId"];
    let action = request["action"].as_str()?;
    if action == "project_artifact_revise" {
        let mut result = creation::recovered_owner_result(events, request)?;
        result["reasonCode"] = json!("PROJECT_ARTIFACT_REVISION_QUEUED");
        result["parentVersionId"] = request["payload"]["expectedSha256"].clone();
        return Some(result);
    }
    if action == "project_artifact_review_set" {
        let e = events.iter().find(|e| {
            (e["schemaVersion"] == 15 || e["schemaVersion"] == 16)
                && e["type"] == "project_studio.artifact_reviewed"
                && e["payload"]["commandId"] == *command
                && e["payload"]["projectId"] == request["payload"]["projectId"]
        })?;
        let p = &e["payload"];
        let mut result = json!({
            "outcome":"applied",
            "reasonCode":"PROJECT_ARTIFACT_REVIEW_SET",
            "projectId":p["projectId"],
            "workItemId":p["workItemId"],
            "path":p["path"],
            "sha256":p["sha256"],
            "status":p["status"]
        });
        if p.get("note").is_some() {
            result["note"] = p["note"].clone();
        }
        return Some(result);
    }
    let expected = match action {
        "project_resource_import" => "project_studio.resource_imported",
        "project_resource_remove" => "project_studio.resource_removed",
        "project_connection_prepare" => "project_studio.connection_prepared",
        "project_connection_execute" => "project_studio.connection_dispatched",
        _ => return None,
    };
    let e = events.iter().find(|e| {
        e["type"] == expected
            && e["payload"]["commandId"] == *command
            && e["payload"]["projectId"] == request["payload"]["projectId"]
    })?;
    let p = &e["payload"];
    Some(match action {
        "project_resource_import" => {
            json!({"outcome":"applied","reasonCode":"PROJECT_RESOURCE_IMPORTED","projectId":p["projectId"],"resource":p["resource"]})
        }
        "project_resource_remove" => {
            json!({"outcome":"applied","reasonCode":"PROJECT_RESOURCE_REMOVED","projectId":p["projectId"],"resourceId":p["resourceId"]})
        }
        "project_connection_prepare" => {
            let mut v = p.clone();
            v["outcome"] = json!("applied");
            v["reasonCode"] = json!("PROJECT_CONNECTION_PREPARED");
            v
        }
        "project_connection_execute" => {
            let finish = &events.iter().find(|e| {
                e["type"] == "project_studio.connection_finished"
                    && e["payload"]["preparedActionId"] == p["preparedActionId"]
            })?["payload"];
            json!({"outcome":if finish["outcome"]=="applied"{"applied"}else{"failed"},"reasonCode":finish["reasonCode"],"projectId":p["projectId"],"preparedActionId":p["preparedActionId"],"revision":3,"result":finish["result"]})
        }
        _ => return None,
    })
}

fn kind(path: &str) -> &str {
    match Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "pptx" => "presentation",
        "docx" => "document",
        "csv" | "xlsx" => "data",
        "png" | "jpg" | "jpeg" | "webp" => "image",
        "mp4" | "mov" | "webm" | "mkv" => "video",
        "wav" | "mp3" | "m4a" => "audio",
        "pdf" => "pdf",
        "txt" | "md" | "json" | "html" | "srt" => "text",
        _ => "file",
    }
}
fn metadata(work: &str, path: &str, bytes: &[u8]) -> Value {
    let sha = digest(bytes);
    json!({"artifactId":format!("artifact:{}",digest(format!("{work}:{path}").as_bytes())),"versionId":sha,"workItemId":work,"path":path,"name":Path::new(path).file_name().unwrap_or_default().to_string_lossy(),"sha256":sha,"bytes":bytes.len(),"kind":kind(path)})
}
fn resource_directory(data: &Path, project: &str, resource: &str) -> Result<PathBuf> {
    id(&json!(project), "project:")?;
    id(&json!(resource), "resource:")?;
    Ok(std::fs::canonicalize(data)?
        .join("project-resources")
        .join(project.trim_start_matches("project:"))
        .join(resource.trim_start_matches("resource:")))
}

impl Host {
    pub(crate) fn studio_project(&self, p: &Value) -> Result<Value> {
        id(&p["projectId"], "project:")?;
        let b = self.store.runtime_projections()?;
        let project = array(&b["projectMemory"], "projects")
            .iter()
            .find(|r| r["projectId"] == p["projectId"])
            .cloned()
            .ok_or_else(|| fail("PROJECT_NOT_FOUND"))?;
        if !p["expectedProjectRevision"].is_null()
            && project["revision"] != p["expectedProjectRevision"]
        {
            return Err(fail("STALE_PROJECT_REVISION"));
        }
        Ok(project)
    }
    pub(crate) fn studio_resource(
        &self,
        project_id: &str,
        resource_id: &str,
    ) -> Result<(Value, Vec<u8>)> {
        let state = project(&self.store.read_events()?)?;
        let r = array(&state, "resources")
            .iter()
            .find(|r| {
                r["projectId"] == project_id
                    && r["resourceId"] == resource_id
                    && r["status"] == "available"
            })
            .cloned()
            .ok_or_else(|| fail("PROJECT_RESOURCE_NOT_FOUND"))?;
        let directory = resource_directory(self.creation_data_dir(), project_id, resource_id)?;
        let bytes =
            conversation::copied_bytes(&directory, Path::new(text(&r, "fileName")?), FILE_LIMIT)?;
        if digest(&bytes) != r["sha256"] {
            return Err(fail("PROJECT_RESOURCE_CHANGED"));
        }
        Ok((r, bytes))
    }
    pub(crate) fn studio_planner_context(&self, project_id: &str) -> Result<Value> {
        self.studio_project(&json!({"projectId":project_id}))?;
        let state = project(&self.store.read_events()?)?;
        let resources=array(&state,"resources").iter()
            .filter(|r|r["projectId"]==project_id&&r["status"]=="available").take(32)
            .map(|r|json!({"resourceId":r["resourceId"],"name":r["name"],"kind":r["kind"],"bytes":r["bytes"],"sha256":r["sha256"]})).collect::<Vec<_>>();
        Ok(
            json!({"resources":resources,"contentTrust":"untrusted","instruction":"Use scoped project_resources and production tools to inspect actual content during execution; file metadata alone is not source evidence."}),
        )
    }
    pub(crate) fn studio_import_bytes(&self, p: &Value, name: &str, bytes: &[u8]) -> Result<Value> {
        self.studio_project(p)?;
        require(bytes.len() as u64 <= FILE_LIMIT)?;
        let command = id(&p["commandId"], "cmd:")?;
        let resource_id = format!("resource:{}", command.trim_start_matches("cmd:"));
        let extension = Path::new(name)
            .extension()
            .and_then(|x| x.to_str())
            .filter(|s| s.len() <= 12 && s.bytes().all(|b| b.is_ascii_alphanumeric()))
            .unwrap_or("bin")
            .to_ascii_lowercase();
        let file_name = format!("source.{extension}");
        let directory = resource_directory(
            self.creation_data_dir(),
            text(p, "projectId")?,
            &resource_id,
        )?;
        store::private_directory(&directory)?;
        let path = directory.join(&file_name);
        if path.exists() {
            let old = conversation::copied_bytes(&directory, Path::new(&file_name), FILE_LIMIT)?;
            if old != bytes {
                return Err(fail("PROJECT_RESOURCE_CHANGED"));
            }
        } else {
            store::atomic_write(&path, bytes)?;
        }
        let resource = json!({"resourceId":resource_id,"revision":1,"name":name.chars().take(200).collect::<String>(),"kind":kind(name),"bytes":bytes.len(),"sha256":digest(bytes),"fileName":file_name,"status":"available"});
        self.store.append(vec![fact(
            "project_studio.resource_imported",
            json!({"projectId":p["projectId"],"commandId":p["commandId"],"resource":resource}),
        )])?;
        Ok(
            json!({"outcome":"applied","reasonCode":"PROJECT_RESOURCE_IMPORTED","projectId":p["projectId"],"resource":resource}),
        )
    }
    async fn studio_preview(&self, path: &str, bytes: Vec<u8>) -> Result<Value> {
        if kind(path) == "text"
            || Path::new(path)
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("csv"))
        {
            if bytes.len() > 256 * 1024 {
                return Ok(json!({"kind":"unsupported","reasonCode":"PROJECT_PREVIEW_TOO_LARGE"}));
            }
            return Ok(match String::from_utf8(bytes) {
                Ok(text) => json!({"kind":"text","text":text}),
                Err(_) => json!({"kind":"unsupported","reasonCode":"PROJECT_PREVIEW_ENCODING"}),
            });
        }
        if Path::new(path)
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("png"))
            && bytes.len() <= 2 * 1024 * 1024
        {
            return Ok(json!({"kind":"image","pngBytes":bytes}));
        }
        let source = format!("inputs/{}", conversation::copied_name(0, Path::new(path)));
        match production::run(
            self.creation_data_dir(),
            "inspect",
            &json!({"source":source}),
            &[(source.clone(), bytes)],
            None,
        )
        .await
        {
            Ok(output) => {
                let mut preview = output.value;
                if let Some((_, bytes)) =
                    output.files.iter().find(|(name, _)| name == "preview.png")
                {
                    if bytes.len() <= 2 * 1024 * 1024 {
                        preview["pngBytes"] = json!(bytes);
                    }
                }
                Ok(preview)
            }
            Err(e) => Ok(json!({"kind":"unsupported","reasonCode":safe_error(&e)})),
        }
    }
    pub(crate) async fn studio_action(&self, action: &str, p: &Value) -> Result<Value> {
        if capabilities::action_fields(action).is_some() {
            return capabilities::action(self, action, p).await;
        }
        let current = self.studio_project(p)?;
        if matches!(
            action,
            "project_resource_import" | "project_artifact_revise"
        ) {
            require(p["expectedProjectRevision"].as_u64().is_some_and(|n| n > 0))?;
        }
        let state = project(&self.store.read_events()?)?;
        match action {
            "project_delivery_export" => delivery::export(self, p),
            "project_studio_read" => {
                let mut artifacts = Vec::new();
                let mut scanned_bytes = 0u64;
                let jobs = self.store.creation_jobs(self.creation_data_dir())?;
                for job in jobs
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|j| j["projectId"] == p["projectId"])
                    .rev()
                    .take(50)
                {
                    let selection =
                        json!({"projectId":p["projectId"],"workItemId":job["workItemId"]});
                    let Ok((root, _)) = self.creation_scope(&selection) else {
                        continue;
                    };
                    let Ok(list) = self
                        .creation_action("project_results_read", &selection)
                        .await
                    else {
                        continue;
                    };
                    for a in array(&list, "items").iter().take(100) {
                        let path = text(a, "path")?;
                        if scanned_bytes + a["bytes"].as_u64().unwrap_or(FILE_LIMIT)
                            > 64 * 1024 * 1024
                        {
                            continue;
                        }
                        let Ok(bytes) = conversation::copied_bytes(
                            &root,
                            &creation::relative_result(path)?,
                            FILE_LIMIT,
                        ) else {
                            continue;
                        };
                        scanned_bytes += bytes.len() as u64;
                        let mut meta = metadata(text(job, "workItemId")?, path, &bytes);
                        if let Some(parent) = array(&state, "revisions")
                            .iter()
                            .find(|r| r["workItemId"] == job["workItemId"])
                        {
                            meta["parentVersionId"] = parent["sha256"].clone();
                        }
                        artifacts.push(meta);
                        if artifacts.len() >= 200 {
                            break;
                        }
                    }
                    if artifacts.len() >= 200 {
                        break;
                    }
                }
                Ok(
                    json!({"reasonCode":"PROJECT_STUDIO_READ","projectId":p["projectId"],"revision":state["revision"],"resources":array(&state,"resources").iter().filter(|r|r["projectId"]==p["projectId"]).collect::<Vec<_>>(),"tools":production::readiness(self.creation_data_dir()).await?,"imageEngine":production::image_readiness(self.creation_data_dir(),text(p,"projectId")?).await?,"connections":crate::app_connections::metadata(self.creation_data_dir(),text(p,"projectId")?)?,"reviews":array(&state,"reviews").iter().filter(|r|r["projectId"]==p["projectId"]).collect::<Vec<_>>(),"artifacts":artifacts}),
                )
            }
            "project_artifact_read" => {
                let (root, _) = self.creation_scope(p)?;
                let path = text(p, "path")?;
                let bytes = conversation::copied_bytes(
                    &root,
                    &creation::relative_result(path)?,
                    FILE_LIMIT,
                )?;
                let sha = digest(&bytes);
                if !p["expectedSha256"].is_null() && p["expectedSha256"] != sha {
                    return Err(fail("PROJECT_ARTIFACT_CHANGED"));
                }
                let preview = self.studio_preview(path, bytes).await?;
                let sections = array(&preview, "pages")
                    .iter()
                    .map(|page| json!({"id":page["id"],"label":page["title"]}))
                    .collect::<Vec<_>>();
                Ok(
                    json!({"reasonCode":"PROJECT_ARTIFACT_READ","projectId":p["projectId"],"workItemId":p["workItemId"],"path":path,"sha256":sha,"preview":preview,"sections":sections}),
                )
            }
            "project_artifact_review_set" => {
                let (root, _) = self.creation_scope(p)?;
                let path = text(p, "path")?;
                let bytes = conversation::copied_bytes(
                    &root,
                    &creation::relative_result(path)?,
                    FILE_LIMIT,
                )?;
                if digest(&bytes) != hash(&p["expectedSha256"])? {
                    return Err(fail("PROJECT_ARTIFACT_CHANGED"));
                }
                review_scope(p)?;
                let note = review_note(p)?;
                self.store.append(vec![review_fact(
                    "project_studio.artifact_reviewed",
                    json!({
                        "projectId":p["projectId"],
                        "commandId":p["commandId"],
                        "workItemId":p["workItemId"],
                        "path":p["path"],
                        "sha256":p["expectedSha256"],
                        "status":p["status"],
                        "note":note
                    }),
                )])?;
                Ok(json!({
                    "outcome":"applied",
                    "reasonCode":"PROJECT_ARTIFACT_REVIEW_SET",
                    "projectId":p["projectId"],
                    "workItemId":p["workItemId"],
                    "path":p["path"],
                    "sha256":p["expectedSha256"],
                    "status":p["status"],
                    "note":note
                }))
            }
            "project_artifact_revise" => {
                let (root, old_work) = self.creation_scope(p)?;
                let path = text(p, "path")?;
                let bytes = conversation::copied_bytes(
                    &root,
                    &creation::relative_result(path)?,
                    FILE_LIMIT,
                )?;
                if hash(&p["expectedSha256"])? != digest(&bytes) {
                    return Err(fail("PROJECT_ARTIFACT_CHANGED"));
                }
                let instruction = bounded(&p["instruction"], 12000)?;
                let source = conversation::copied_name(0, Path::new(path));
                let section = p["sectionId"].as_str().unwrap_or("");
                capabilities::validate_selection(self, path, &bytes, section).await?;
                let manifest = json!({"schemaVersion":1,"projectId":p["projectId"],"parentWorkItemId":p["workItemId"],"path":path,"sha256":p["expectedSha256"],"sectionId":p["sectionId"],"files":[{"path":format!("inputs/{source}"),"sha256":digest(&bytes),"bytes":bytes.len()}]});
                let command = id(&p["commandId"], "cmd:")?;
                let work_id = format!("work:{}", command.trim_start_matches("cmd:"));
                let revision = fact(
                    "project_studio.revision_requested",
                    json!({"projectId":p["projectId"],"commandId":command,"workItemId":work_id,"parentWorkItemId":p["workItemId"],"path":path,"sha256":p["expectedSha256"],"sectionId":p["sectionId"],"inputManifest":manifest}),
                );
                self.store.append(vec![revision])?;
                let provider = p["providerId"]
                    .as_str()
                    .unwrap_or(old_work["providerId"].as_str().unwrap_or("codex"));
                let input = json!({"projectId":p["projectId"],"expectedProjectRevision":current["revision"],"commandId":p["commandId"],"createdAt":p["createdAt"],"expiresAt":p["expiresAt"],"providerId":provider,"abilityId":"W1","title":format!("Revise {}",Path::new(path).file_name().unwrap_or_default().to_string_lossy()),"instruction":format!("Revise the frozen input inputs/{source}. Requested section: {section}. Preserve all other sections and the original source. Write a new output file. For slide:N/paragraph:N use presentations.revise/documents.revise. For a normalized region:x,y,width,height use images.mask then images.edit with that exact source/mask. For time:start:end edit only that video interval and preserve the rest. page:N selects the actual rendered page. Owner revision: {instruction}")});
                let mut result =
                    self.creation_enqueue_inputs(&input, &[(source, bytes)], Some(&manifest))?;
                result["reasonCode"] = json!("PROJECT_ARTIFACT_REVISION_QUEUED");
                result["parentVersionId"] = p["expectedSha256"].clone();
                Ok(result)
            }
            "project_resource_import" => {
                let path = Path::new(bounded(&p["sourcePath"], 4096)?);
                require(path.is_absolute())?;
                let parent = path
                    .parent()
                    .ok_or_else(|| fail("PROJECT_RESOURCE_PATH_INVALID"))?;
                let name = path
                    .file_name()
                    .ok_or_else(|| fail("PROJECT_RESOURCE_PATH_INVALID"))?;
                let parent = std::fs::canonicalize(parent)
                    .map_err(|_| fail("PROJECT_RESOURCE_PATH_INVALID"))?;
                let bytes = conversation::copied_bytes(&parent, Path::new(name), FILE_LIMIT)?;
                self.studio_import_bytes(p, &name.to_string_lossy(), &bytes)
            }
            "project_resource_remove" => {
                let r = array(&state, "resources")
                    .iter()
                    .find(|r| {
                        r["projectId"] == p["projectId"]
                            && r["resourceId"] == p["resourceId"]
                            && r["status"] == "available"
                    })
                    .ok_or_else(|| fail("PROJECT_RESOURCE_NOT_FOUND"))?;
                if r["revision"] != p["expectedResourceRevision"] {
                    return Err(fail("PROJECT_RESOURCE_STALE"));
                }
                self.store.append(vec![fact("project_studio.resource_removed",json!({"projectId":p["projectId"],"commandId":p["commandId"],"resourceId":p["resourceId"],"expectedResourceRevision":p["expectedResourceRevision"]}))])?;
                Ok(
                    json!({"outcome":"applied","reasonCode":"PROJECT_RESOURCE_REMOVED","projectId":p["projectId"],"resourceId":p["resourceId"]}),
                )
            }
            "project_tool_configure" => {
                production::configure(
                    self.creation_data_dir(),
                    text(p, "toolId")?,
                    text(p, "executablePath")?,
                )?;
                Ok(
                    json!({"outcome":"applied","reasonCode":"PROJECT_TOOL_CONFIGURED","projectId":p["projectId"],"toolId":p["toolId"]}),
                )
            }
            "project_connection_set" | "project_connection_remove" => {
                let command = p["commandId"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("cmd:{}", uuid::Uuid::new_v4()));
                id(&json!(command), "cmd:")?;
                let metadata = if action == "project_connection_set" {
                    crate::app_connections::configure(
                        self.creation_data_dir(),
                        text(p, "projectId")?,
                        text(p, "providerId")?,
                        text(p, "label")?,
                        text(p, "accessToken")?,
                        &command,
                    )?
                } else {
                    crate::app_connections::remove(
                        self.creation_data_dir(),
                        text(p, "projectId")?,
                        text(p, "providerId")?,
                        &command,
                    )?
                };
                Ok(
                    json!({"outcome":"applied","reasonCode":if action=="project_connection_set"{"PROJECT_CONNECTION_CONFIGURED"}else{"PROJECT_CONNECTION_REMOVED"},"projectId":p["projectId"],"providerId":p["providerId"],"connection":metadata}),
                )
            }
            "project_connection_prepare" => {
                let args = crate::app_connections::validate_prepare(
                    text(p, "providerId")?,
                    text(p, "operation")?,
                    &p["arguments"],
                )?;
                let connections = crate::app_connections::metadata(
                    self.creation_data_dir(),
                    text(p, "projectId")?,
                )?;
                let connection = connections
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|a| a["providerId"] == p["providerId"] && a["connected"] == true)
                    .ok_or_else(|| fail("PROJECT_CONNECTION_SETUP_REQUIRED"))?;
                let action_id = format!(
                    "connection-action:{}",
                    id(&p["commandId"], "cmd:")?.trim_start_matches("cmd:")
                );
                let prepared = json!({"projectId":p["projectId"],"commandId":p["commandId"],"preparedActionId":action_id,"revision":1,"providerId":p["providerId"],"operation":p["operation"],"arguments":args,"connectionRevision":connection["connectionRevision"]});
                self.store.append(vec![fact(
                    "project_studio.connection_prepared",
                    prepared.clone(),
                )])?;
                let mut result = prepared;
                result["outcome"] = json!("applied");
                result["reasonCode"] = json!("PROJECT_CONNECTION_PREPARED");
                Ok(result)
            }
            "project_connection_execute" => {
                let prepared = array(&state, "connections")
                    .iter()
                    .find(|a| {
                        a["projectId"] == p["projectId"]
                            && a["preparedActionId"] == p["preparedActionId"]
                    })
                    .ok_or_else(|| fail("PROJECT_CONNECTION_ACTION_NOT_FOUND"))?;
                if prepared["revision"] != p["expectedActionRevision"]
                    || prepared["status"] != "prepared"
                {
                    return Err(fail("PROJECT_CONNECTION_STALE_ACTION"));
                }
                let connections = crate::app_connections::metadata(
                    self.creation_data_dir(),
                    text(p, "projectId")?,
                )?;
                if !connections.as_array().unwrap().iter().any(|c| {
                    c["providerId"] == prepared["providerId"]
                        && c["connected"] == true
                        && c["connectionRevision"] == prepared["connectionRevision"]
                }) {
                    return Err(fail("PROJECT_CONNECTION_CHANGED"));
                }
                self.store.append(vec![fact("project_studio.connection_dispatched",json!({"projectId":p["projectId"],"commandId":p["commandId"],"preparedActionId":p["preparedActionId"],"expectedActionRevision":p["expectedActionRevision"]}))])?;
                let response = crate::app_connections::execute(
                    self.creation_data_dir(),
                    text(p, "projectId")?,
                    text(p, "preparedActionId")?,
                    text(prepared, "providerId")?,
                    text(prepared, "operation")?,
                    &prepared["arguments"],
                )
                .await;
                let (outcome, code, result) = match response {
                    Ok(output) => {
                        let mut result = output.value;
                        let mut imported = Vec::new();
                        let mut failed = false;
                        for (name, bytes) in output.files {
                            match self.studio_import_bytes(p, &name, &bytes) {
                                Ok(r) => imported.push(r),
                                Err(_) => {
                                    failed = true;
                                }
                            }
                        }
                        if !imported.is_empty() {
                            result["resources"] = json!(imported);
                        }
                        (
                            if failed { "failed" } else { "applied" },
                            if failed {
                                "PROJECT_RESOURCE_IMPORT_FAILED"
                            } else {
                                "PROJECT_CONNECTION_APPLIED"
                            }
                            .to_owned(),
                            result,
                        )
                    }
                    Err(e) => {
                        let code = safe_error(&e);
                        (
                            if code == "PROJECT_CONNECTION_OUTCOME_UNKNOWN" {
                                "unknown"
                            } else {
                                "failed"
                            },
                            code,
                            Value::Null,
                        )
                    }
                };
                self.store.append(vec![fact("project_studio.connection_finished",json!({"preparedActionId":p["preparedActionId"],"outcome":outcome,"reasonCode":code,"result":result}))])?;
                Ok(
                    json!({"outcome":if outcome=="applied"{"applied"}else{"failed"},"reasonCode":code,"projectId":p["projectId"],"preparedActionId":p["preparedActionId"],"revision":3,"result":result}),
                )
            }
            _ => Err(fail("PROJECT_STUDIO_ACTION_UNSUPPORTED")),
        }
    }
}
fn safe_error(e: &Error) -> String {
    if e.0.len() <= 100
        && e.0
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    {
        e.0.clone()
    } else {
        "PRODUCTION_FAILED".into()
    }
}

pub(crate) fn validate_agent_arguments(tool: &str, args: &Value) -> Result<()> {
    match tool {
        "project_resource_publish" => {
            exact(args, &["source", "nonce"], &[])?;
            let path = creation::relative_result(bounded(&args["source"], 1024)?)?;
            require(path.starts_with("outputs"))?;
            bounded(&args["nonce"], 128)?;
        }
        "project_resources" => {
            exact(args, &[], &["resourceId", "offset", "limit"])?;
            if !args["resourceId"].is_null() {
                id(&args["resourceId"], "resource:")?;
            }
            for (key, max) in [("offset", 100000), ("limit", 20)] {
                if let Some(v) = args.get(key) {
                    require(
                        v.as_u64()
                            .is_some_and(|n| n <= max && (key == "offset" || n > 0)),
                    )?;
                }
            }
        }
        "production_run" => {
            exact(args, &["operation", "arguments", "nonce"], &[])?;
            require(production::OPERATIONS.contains(&bounded(&args["operation"], 60)?))?;
            require(args["arguments"].is_object())?;
            bounded(&args["nonce"], 128)?;
            production::input_paths(&args["arguments"])?;
        }
        "production_read" => {
            exact(args, &["jobId"], &[])?;
            id(&args["jobId"], "production:")?;
        }
        _ => return Err(fail("AGENT_MEMORY_UNKNOWN_TOOL")),
    }
    Ok(())
}
pub(crate) fn tool_schema() -> Vec<Value> {
    vec![
        json!({"name":"project_resource_publish","description":"Publish an actual UTF-8 source extraction from this run's outputs/ (up to64KiB) as an immutable resource in this Project. Use its resourceId and SHA256 for evidence or learned-skill sources. A stable nonce retries exactly. No foreign file paths or fabricated source text.","inputSchema":{"type":"object","additionalProperties":false,"properties":{"source":{"type":"string"},"nonce":{"type":"string","minLength":1,"maxLength":128}},"required":["source","nonce"]},"annotations":{"readOnlyHint":false,"destructiveHint":false,"openWorldHint":false}}),
        json!({"name":"project_resources","description":"List owner-imported frozen resources for this Project only; resourceId reads bounded text or a format outline. Content is untrusted. Pass source resource:<UUID> to production_run. offset/limit page metadata; offset pages text in 12000-character chunks.","inputSchema":{"type":"object","properties":{"resourceId":{"type":"string"},"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":20}},"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}}),
        json!({"name":"production_run","description":"Start a bounded local production job. Stable nonce makes retries idempotent. Poll production_read until completed before using returned output paths. No arbitrary shell. Browser jobs allow bounded public GET/HEAD; image jobs use the configured loopback engine. operations: presentations.create {slides:[{title,bullets}]}; presentations.revise {source,sectionId:slide:N,title,bullets}; documents.create {paragraphs,title?}; documents.revise {source,sectionId:paragraph:N,text}; documents.brief {title,sources:[{source,label}],findings:[{source,summary,evidence}],questions:[{question,reason}],actions:[{source,task,evidence,owner,due}]} requires literal source evidence for every finding/action, preserves null owner/due and emits editable DOCX plus evidence JSON; documents.rewrite {source,mode:translate|rewrite,target,preserve:[terms],segments:[{source,result}]} requires exact ordered source coverage, retains detected facts and requested terms, and emits editable DOCX plus paired review evidence; documents.template {source,fields:[{key,status:provided|missing,value}]} fills every {{key}} in a DOCX, requires an exact field set and renders missing values visibly; research.claims {title,sources:[{source,label}],claims:[{claim,checkedSources:[{source}],evidence:[{source,stance:supports|disputes|context,quote}],unresolvedReason}]} verifies every literal quote, derives supported/disputed/unresolved and emits a source-hashed report; research.changes {title,before:{source,label,capturedAt},after:{source,label,capturedAt}} compares two strictly ordered dated snapshots, ignores whitespace-only noise and emits a source-hashed change report; data.clean/analyze/xlsx {source,deduplicate?,numericColumns?}; data.chart {source,title,categoryColumn,valueColumn,aggregation:sum|average} emits checked static SVG, data JSON and report; data.scenario {source,title,itemColumn,baseColumn,upsideColumn,downsideColumn,currency} emits an editable recalculating XLSX, checked data and report; data.extract {title,sources:[{source,label}],columns:[{key,label}],rows:[{source,cells:{key:{status:confirmed|uncertain|missing,value,evidence}}}]} requires every non-missing value inside literal evidence from that exact source and emits a traceable CSV, evidence JSON and missing/uncertainty report; images.resize {source,width,height}; images.batch {sources:[{source}],width,height} converts 2-12 images to bounded PNGs and reports each source separately; video.edit {clips:[{source,start,end}],width?,height?,captions?:[{start,end,text}]}; audio.cleanup {source}; audio.podcast {title,clips:[{source,start,end,title}]} joins 1-12 checked audio segments and emits playable M4A plus chapter JSON; resources.extract {source} for PPTX/DOCX/PDF text and structure (PDF needs pdftotext). Sources must be current job inputs/outputs paths or resource:<UUID> or productionSource from a delivered crew offer. Additional operations: images.generate {prompt,width,height,negativePrompt?,seed?,steps?}; images.edit {source,prompt,width,height,negativePrompt?,seed?,steps?,strength?,mask?:{source}} use the Project configured local ComfyUI checkpoint. Dimensions64-2048 in multiples of8; reference/mask are same-size PNGs. images.mask {source,region:{x,y,width,height}} creates a normalized binary PNG mask. resources.ocr {source,language?} uses Tesseract. audio.transcribe {source,language?} uses whisper.cpp and a local model; returns timed text/JSON/SRT. documents.render {source,pages?} renders up to8 exact1-based pages to PNG using LibreOffice/Poppler. video.frame {source,time} returns an actual PNG. video.package {clips:[{source,start,end}],aspects:[portrait|square|landscape],captions:[{start,end,text}],title,description?,hashtags?} exports MP4s, covers, captions and content text. Captions burn through libass or bounded Office overlays. interactive.create {kind:table|chart|budget,title,columns?,rows?,series?:[{label,value}],currency?,budget?,items?:[{label,amount}]} creates fixed offline HTML with no caller script/network. browser.inspect {url?|source?} and browser.draft {url?|source?,fields:[{selector,value}]} require exactly one URL or frozen HTML source; static-page inspection/form values only, isolated Chrome with page scripts disabled and no submission. Office/data need Python3.9+, media need FFmpeg/FFprobe. Missing engines are real blockers; report them honestly.","inputSchema":{"type":"object","properties":{"operation":{"type":"string","enum":production::OPERATIONS},"arguments":{"type":"object"},"nonce":{"type":"string","minLength":1,"maxLength":128}},"required":["operation","arguments","nonce"],"additionalProperties":false},"annotations":{"readOnlyHint":false,"destructiveHint":false,"openWorldHint":false}}),
        json!({"name":"production_read","description":"Read a production job started by this exact active run. completed includes verified files and hashes. failed/cancelled/interrupted have no valid outputs; do not claim success.","inputSchema":{"type":"object","properties":{"jobId":{"type":"string"}},"required":["jobId"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}}),
    ]
}
impl Host {
    pub(crate) async fn studio_agent_call(
        &self,
        binding: &agent_memory::Binding,
        tool: &str,
        args: &Value,
    ) -> Result<Value> {
        validate_agent_arguments(tool, args)?;
        let _guard = self
            .operations
            .try_lock()
            .map_err(|_| fail("PRODUCTION_BUSY"))?;
        self.checked_agent_scope(binding)?;
        self.studio_project(&json!({"projectId":binding.project_id}))?;
        if tool == "project_resource_publish" {
            return capabilities::publish(self, binding, args);
        }
        let state = project(&self.store.read_events()?)?;
        if tool == "project_resources" {
            let offset = args["offset"].as_u64().unwrap_or(0) as usize;
            if let Some(resource_id) = args["resourceId"].as_str() {
                let (resource, bytes) = self.studio_resource(&binding.project_id, resource_id)?;
                // Parsing expensive Office/media archives is a production job; resource reads stay within the MCP deadline.
                let text = if ["text", "data"].contains(&resource["kind"].as_str().unwrap_or(""))
                    && !text(&resource, "fileName")?.ends_with("xlsx")
                {
                    String::from_utf8(bytes).ok()
                } else {
                    None
                };
                let preview = text.map(|s| {
                    s.chars()
                        .skip(offset * 12000)
                        .take(12000)
                        .collect::<String>()
                });
                return Ok(
                    json!({"resource":resource,"text":preview,"offset":offset,"contentTrust":"untrusted","source":resource_id,"reasonCode":if preview.is_some(){"PROJECT_RESOURCE_READ"}else{"PROJECT_RESOURCE_BINARY"}}),
                );
            }
            let resources = array(&state, "resources")
                .iter()
                .filter(|r| r["projectId"] == binding.project_id && r["status"] == "available")
                .skip(offset)
                .take(args["limit"].as_u64().unwrap_or(20) as usize)
                .collect::<Vec<_>>();
            return Ok(json!({"resources":resources,"offset":offset,"revision":state["revision"]}));
        }
        if tool == "production_read" {
            self.finish_production().await?;
            let state = project(&self.store.read_events()?)?;
            return array(&state, "jobs")
                .iter()
                .find(|j| {
                    j["jobId"] == args["jobId"]
                        && j["runId"] == binding.run_id
                        && j["projectId"] == binding.project_id
                        && j["ownerEpoch"] == self.epoch
                })
                .cloned()
                .ok_or_else(|| fail("PRODUCTION_SCOPE_DENIED"));
        }
        let request_hash = digest(&serde_json::to_vec(args)?);
        if let Some(prior) = array(&state, "jobs")
            .iter()
            .find(|j| j["runId"] == binding.run_id && j["nonce"] == args["nonce"])
        {
            if prior["requestHash"] != request_hash {
                return Err(fail("PRODUCTION_NONCE_REUSED"));
            }
            return Ok(prior.clone());
        }
        if array(&state, "jobs")
            .iter()
            .filter(|j| j["runId"] == binding.run_id)
            .count()
            >= 32
        {
            return Err(fail("PRODUCTION_RUN_LIMIT"));
        }
        if self
            .studio_runtime
            .lock()
            .map_err(|_| fail("PRODUCTION_UNAVAILABLE"))?
            .jobs
            .len()
            >= 4
        {
            return Err(fail("PRODUCTION_BUSY"));
        }
        let (root, _) = self.creation_scope(
            &json!({"projectId":binding.project_id,"workItemId":binding.work_item_id}),
        )?;
        let mut inputs = Vec::new();
        let mut arguments = args["arguments"].clone();
        let mut total = 0;
        let mut mapping = Vec::new();
        for (index, path) in production::input_paths(&arguments)?.into_iter().enumerate() {
            let (name, bytes) = if path.starts_with("resource:") {
                let (r, bytes) = self.studio_resource(&binding.project_id, &path)?;
                (text(&r, "fileName")?.to_owned(), bytes)
            } else if let Some(value) = path.strip_prefix("delivery:") {
                let (offer, index) = value
                    .split_once('/')
                    .ok_or_else(|| fail("PRODUCTION_INPUT_PATH_INVALID"))?;
                let (name, bytes) = self.workflow_delivery_input(
                    binding,
                    &format!("offer:{offer}"),
                    index
                        .parse()
                        .map_err(|_| fail("PRODUCTION_INPUT_PATH_INVALID"))?,
                )?;
                (name, bytes)
            } else {
                let bytes =
                    conversation::copied_bytes(&root, Path::new(&path), FILE_LIMIT - total)?;
                (path.clone(), bytes)
            };
            total += bytes.len() as u64;
            if total > FILE_LIMIT {
                return Err(fail("PRODUCTION_INPUT_LIMIT"));
            }
            let alias = format!(
                "inputs/{}",
                conversation::copied_name(index, Path::new(&name))
            );
            mapping.push((path, alias.clone()));
            inputs.push((alias, bytes));
        }
        production::remap_sources(&mut arguments, &mapping);
        self.checked_agent_scope(binding)?;
        let job_id = format!("production:{}", uuid::Uuid::new_v4());
        let p = json!({"jobId":job_id,"projectId":binding.project_id,"runId":binding.run_id,"workItemId":binding.work_item_id,"ownerEpoch":self.epoch,"nonce":args["nonce"],"requestHash":request_hash,"operation":args["operation"]});
        self.store
            .append(vec![fact("project_studio.production_queued", p.clone())])?;
        let data = self.creation_data_dir().to_path_buf();
        let operation = text(args, "operation")?.to_owned();
        let active = binding.active.clone();
        let child_active = active.clone();
        let project_id = binding.project_id.clone();
        let handle = tokio::spawn(async move {
            production::run_for_project(
                &data,
                &project_id,
                &operation,
                &arguments,
                &inputs,
                Some(child_active),
            )
            .await
        });
        self.studio_runtime
            .lock()
            .map_err(|_| fail("PRODUCTION_UNAVAILABLE"))?
            .jobs
            .insert(
                job_id,
                Active {
                    active,
                    binding: binding.clone(),
                    handle,
                },
            );
        let mut result = p;
        result["status"] = json!("running");
        Ok(result)
    }
    pub(crate) async fn finish_production(&self) -> Result<()> {
        let done = {
            let mut runtime = self
                .studio_runtime
                .lock()
                .map_err(|_| fail("PRODUCTION_UNAVAILABLE"))?;
            let ids = runtime
                .jobs
                .iter()
                .filter(|(_, j)| j.handle.is_finished())
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            ids.into_iter()
                .filter_map(|id| runtime.jobs.remove(&id).map(|j| (id, j)))
                .collect::<Vec<_>>()
        };
        for (job_id, job) in done {
            let output = job
                .handle
                .await
                .unwrap_or_else(|_| Err(fail("PRODUCTION_INTERRUPTED")));
            let scoped = self.checked_agent_scope(&job.binding);
            let result = match (output, scoped) {
                (Ok(output), Ok(_)) => self.publish_production(&job_id, &job.binding, output),
                (Err(e), _) => Err(e),
                (_, Err(_)) => Err(fail("PRODUCTION_CANCELLED")),
            };
            let (outcome, reason, artifacts) = match result {
                Ok(artifacts) => ("completed", "PRODUCTION_COMPLETED".into(), artifacts),
                Err(e) => (
                    if e.0 == "PRODUCTION_CANCELLED" {
                        "cancelled"
                    } else {
                        "failed"
                    },
                    safe_error(&e),
                    Vec::new(),
                ),
            };
            self.store.append(vec![fact(
                "project_studio.production_finished",
                json!({"jobId":job_id,"outcome":outcome,"reasonCode":reason,"artifacts":artifacts}),
            )])?;
        }
        Ok(())
    }
    fn publish_production(
        &self,
        job_id: &str,
        binding: &agent_memory::Binding,
        output: production::Output,
    ) -> Result<Vec<Value>> {
        let (root, _) = self.creation_scope(
            &json!({"projectId":binding.project_id,"workItemId":binding.work_item_id}),
        )?;
        let folder = format!("tool-{}", job_id.trim_start_matches("production:"));
        creation::publish_outputs(&root, &folder, &output.files)?;
        let mut artifacts = Vec::new();
        for (name, bytes) in output.files {
            let path = format!("outputs/{folder}/{name}");
            let verified = conversation::copied_bytes(&root, Path::new(&path), FILE_LIMIT)?;
            if verified != bytes {
                return Err(fail("PRODUCTION_OUTPUT_CHANGED"));
            }
            artifacts.push(metadata(&binding.work_item_id, &path, &bytes));
        }
        require(!artifacts.is_empty())?;
        Ok(artifacts)
    }
    pub(crate) fn cancel_production(&self) {
        if let Ok(runtime) = self.studio_runtime.lock() {
            for j in runtime.jobs.values() {
                j.active.store(false, Ordering::SeqCst);
            }
        }
    }
    pub(crate) async fn shutdown_production(&self) {
        self.cancel_production();
        let jobs = self
            .studio_runtime
            .lock()
            .map(|mut r| std::mem::take(&mut r.jobs))
            .unwrap_or_default();
        for (job_id, job) in jobs {
            let _ = job.handle.await;
            let _=self.store.append(vec![fact("project_studio.production_finished",json!({"jobId":job_id,"outcome":"cancelled","reasonCode":"PRODUCTION_CANCELLED","artifacts":[]}))]);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    fn command(mut p: Value) -> Value {
        p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
        let now = chrono::Utc::now();
        p["createdAt"] = json!(now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        p["expiresAt"] = json!((now + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        p
    }
    async fn fixture() -> (tempfile::TempDir, Arc<Host>, Value) {
        let tmp = tempfile::tempdir().unwrap();
        let host = Host::open(tmp.path().join("data")).unwrap();
        let project = host
            .action("project_create", command(json!({"name":"Studio"})))
            .await
            .unwrap()["projectId"]
            .clone();
        (tmp, host, project)
    }
    #[tokio::test]
    async fn resources_are_frozen_project_scoped_and_detect_tampering() {
        let (tmp, host, project) = fixture().await;
        let source = tmp.path().join("brief.md");
        std::fs::write(&source, b"Frozen owner resource").unwrap();
        let p =
            command(json!({"projectId":project,"expectedProjectRevision":1,"sourcePath":source}));
        let imported = host
            .action("project_resource_import", p.clone())
            .await
            .unwrap();
        assert_eq!(imported["outcome"], "applied");
        assert_eq!(
            host.action("project_resource_import", p).await.unwrap(),
            imported
        );
        std::fs::write(source, b"Changed source").unwrap();
        let rid = imported["resource"]["resourceId"].as_str().unwrap();
        assert_eq!(
            host.studio_resource(project.as_str().unwrap(), rid)
                .unwrap()
                .1,
            b"Frozen owner resource"
        );
        let other = host
            .action("project_create", command(json!({"name":"Other"})))
            .await
            .unwrap()["projectId"]
            .clone();
        assert!(host.studio_resource(other.as_str().unwrap(), rid).is_err());
        let root =
            resource_directory(host.creation_data_dir(), project.as_str().unwrap(), rid).unwrap();
        std::fs::write(root.join("source.md"), b"Modified frozen bytes").unwrap();
        assert_eq!(
            host.studio_resource(project.as_str().unwrap(), rid)
                .unwrap_err()
                .0,
            "PROJECT_RESOURCE_CHANGED"
        );
        let removed = host
            .action(
                "project_resource_remove",
                command(json!({"projectId":project,"resourceId":rid,"expectedResourceRevision":1})),
            )
            .await
            .unwrap();
        assert_eq!(removed["outcome"], "applied");
        assert!(host
            .studio_resource(project.as_str().unwrap(), rid)
            .is_err());
        let events = host.store.read_events().unwrap();
        let bundle = domain::project_events(&events).unwrap();
        assert_eq!(bundle["projectStudio"]["resources"][0]["status"], "removed");
        let private = serde_json::to_string(&events).unwrap();
        assert!(!private.contains("Frozen owner resource"));
        host.shutdown().await;
    }
    #[tokio::test]
    async fn artifact_revision_pins_source_and_export_preserves_original() {
        let (tmp, host, project) = fixture().await;
        let queued=host.action("project_work_enqueue",command(json!({"projectId":project,"expectedProjectRevision":1,"abilityId":"D1","title":"Original","instruction":"Create document"}))).await.unwrap();
        let work = queued["workItemId"].clone();
        let (root, _) = host
            .creation_scope(&json!({"projectId":project,"workItemId":work}))
            .unwrap();
        let original = b"Original text";
        std::fs::write(root.join("outputs/summary.md"), original).unwrap();
        let sha = digest(original);
        let preview=host.action("project_artifact_read",json!({"projectId":project,"workItemId":work,"path":"outputs/summary.md","expectedSha256":sha})).await.unwrap();
        assert_eq!(preview["preview"]["text"], "Original text");
        let revision = host.store.runtime_projections().unwrap()["projectMemory"]["projects"][0]
            ["revision"]
            .clone();
        let request = command(
            json!({"projectId":project,"expectedProjectRevision":revision,"workItemId":work,"path":"outputs/summary.md","expectedSha256":sha,"instruction":"Make the conclusion shorter","providerId":"codex"}),
        );
        let result = host
            .action("project_artifact_revise", request.clone())
            .await
            .unwrap();
        assert_eq!(result["outcome"], "applied");
        assert_eq!(
            host.action("project_artifact_revise", request)
                .await
                .unwrap(),
            result
        );
        assert_eq!(
            std::fs::read(root.join("outputs/summary.md")).unwrap(),
            original
        );
        let (child, child_work) = host
            .creation_scope(&json!({"projectId":project,"workItemId":result["workItemId"]}))
            .unwrap();
        assert_eq!(child_work["status"], "queued");
        assert_eq!(
            std::fs::read(child.join("inputs/reference-01.md")).unwrap(),
            original
        );
        let export = tmp.path().join("export.md");
        host.action("project_result_export",command(json!({"projectId":project,"workItemId":work,"path":"outputs/summary.md","expectedSha256":sha,"targetPath":export}))).await.unwrap();
        assert_eq!(std::fs::read(export).unwrap(), original);
        std::fs::write(root.join("outputs/summary.md"), b"Later changed").unwrap();
        let no_copy = tmp.path().join("must-not-exist.md");
        assert!(host.action("project_result_export",command(json!({"projectId":project,"workItemId":work,"path":"outputs/summary.md","expectedSha256":sha,"targetPath":no_copy}))).await.is_err());
        assert!(!no_copy.exists());
        host.shutdown().await;
    }
    #[tokio::test]
    async fn artifact_review_is_durable_exact_version_scoped_and_never_inherited() {
        let (_tmp, host, project) = fixture().await;
        let queued=host.action("project_work_enqueue",command(json!({"projectId":project,"expectedProjectRevision":1,"abilityId":"D1","title":"Review","instruction":"Create document"}))).await.unwrap();
        let work = queued["workItemId"].clone();
        let (root, _) = host
            .creation_scope(&json!({"projectId":project,"workItemId":work}))
            .unwrap();
        let path = "outputs/summary.md";
        let first = b"First exact version";
        std::fs::write(root.join(path), first).unwrap();
        let first_hash = digest(first);
        let request = command(json!({
            "projectId":project,"workItemId":work,"path":path,
            "expectedSha256":first_hash,"status":"reviewed",
            "note":"Checked structure and source coverage."
        }));
        let receipt = host
            .action("project_artifact_review_set", request.clone())
            .await
            .unwrap();
        assert_eq!(receipt["status"], "reviewed");
        assert_eq!(receipt["note"], "Checked structure and source coverage.");
        assert_eq!(
            host.action("project_artifact_review_set", request.clone())
                .await
                .unwrap(),
            receipt
        );

        let events = host.store.read_events().unwrap();
        let owner = events
            .iter()
            .find(|e| {
                e["schemaVersion"] == 16
                    && e["type"] == "owner.command.requested"
                    && e["payload"]["commandId"] == request["commandId"]
            })
            .unwrap();
        let mut wrong_version = owner.clone();
        wrong_version["schemaVersion"] = json!(15);
        assert!(domain::validate_event(&wrong_version).is_err());
        wrong_version["schemaVersion"] = json!(12);
        assert!(domain::validate_event(&wrong_version).is_err());
        let without_receipt = events
            .iter()
            .filter(|e| {
                !(e["schemaVersion"] == 16
                    && e["type"] == "owner.command.receipted"
                    && e["payload"]["commandId"] == request["commandId"])
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            recovered_owner_result(&without_receipt, &owner["payload"]).unwrap(),
            receipt
        );
        let legacy = events
            .iter()
            .cloned()
            .map(|mut event| {
                if event["schemaVersion"] == 16
                    && event["payload"]["commandId"] == request["commandId"]
                {
                    event["schemaVersion"] = json!(15);
                    if event["type"] == "owner.command.requested" {
                        event["payload"]["payload"]
                            .as_object_mut()
                            .unwrap()
                            .remove("note");
                    } else if event["type"] == "project_studio.artifact_reviewed" {
                        event["payload"].as_object_mut().unwrap().remove("note");
                    } else if event["type"] == "owner.command.receipted" {
                        event["payload"]["result"]
                            .as_object_mut()
                            .unwrap()
                            .remove("note");
                    }
                }
                event
            })
            .collect::<Vec<_>>();
        let legacy_state = super::project(&legacy).unwrap();
        assert_eq!(legacy_state["reviews"][0]["status"], "reviewed");
        assert!(legacy_state["reviews"][0].get("note").is_none());
        let page = host
            .action("project_studio_read", json!({"projectId":project}))
            .await
            .unwrap();
        assert_eq!(page["reviews"][0]["sha256"], first_hash);
        assert_eq!(page["reviews"][0]["status"], "reviewed");
        assert_eq!(
            page["reviews"][0]["note"],
            "Checked structure and source coverage."
        );
        for (status, note) in [
            (
                "unreviewed",
                "A cleared review cannot keep a note".to_owned(),
            ),
            ("reviewed", " padded".to_owned()),
            ("reviewed", "x".repeat(4_001)),
        ] {
            let before = host.store.read_events().unwrap().len();
            assert_eq!(
                host.action(
                    "project_artifact_review_set",
                    command(json!({"projectId":project,"workItemId":work,"path":path,"expectedSha256":first_hash,"status":status,"note":note})),
                )
                .await
                .unwrap_err()
                .0,
                "PROJECT_STUDIO_INVALID"
            );
            assert_eq!(host.store.read_events().unwrap().len(), before);
        }

        let other = host
            .action("project_create", command(json!({"name":"Other"})))
            .await
            .unwrap()["projectId"]
            .clone();
        let foreign = host
            .action(
                "project_artifact_review_set",
                command(json!({"projectId":other,"workItemId":work,"path":path,"expectedSha256":first_hash,"status":"reviewed"})),
            )
            .await
            .unwrap_err();
        assert_eq!(foreign.0, "PROJECT_JOB_NOT_FOUND");

        let second = b"Second exact version";
        std::fs::write(root.join(path), second).unwrap();
        let stale = host
            .action(
                "project_artifact_review_set",
                command(json!({"projectId":project,"workItemId":work,"path":path,"expectedSha256":first_hash,"status":"needs_changes"})),
            )
            .await
            .unwrap_err();
        assert_eq!(stale.0, "PROJECT_ARTIFACT_CHANGED");
        let second_hash = digest(second);
        host.action(
            "project_artifact_review_set",
            command(json!({"projectId":project,"workItemId":work,"path":path,"expectedSha256":second_hash,"status":"needs_changes","note":"Clarify the final recommendation."})),
        )
        .await
        .unwrap();
        let state = super::project(&host.store.read_events().unwrap()).unwrap();
        assert_eq!(array(&state, "reviews").len(), 2);
        assert!(array(&state, "reviews")
            .iter()
            .any(|review| { review["sha256"] == first_hash && review["status"] == "reviewed" }));
        assert!(array(&state, "reviews").iter().any(|review| {
            review["sha256"] == second_hash
                && review["status"] == "needs_changes"
                && review["note"] == "Clarify the final recommendation."
        }));
        host.action(
            "project_artifact_review_set",
            command(json!({"projectId":project,"workItemId":work,"path":path,"expectedSha256":second_hash,"status":"unreviewed"})),
        )
        .await
        .unwrap();
        let state = super::project(&host.store.read_events().unwrap()).unwrap();
        assert_eq!(array(&state, "reviews").len(), 1);
        assert_eq!(state["reviews"][0]["sha256"], first_hash);
        let data = host.creation_data_dir().to_owned();
        host.shutdown().await;
        drop(host);
        let reopened = Host::open(data).unwrap();
        let page = reopened
            .action("project_studio_read", json!({"projectId":project}))
            .await
            .unwrap();
        assert_eq!(array(&page, "reviews").len(), 1);
        assert_eq!(page["reviews"][0]["sha256"], first_hash);
        reopened.shutdown().await;
    }
    #[tokio::test]
    async fn connection_secrets_never_enter_events_and_changed_credentials_invalidate_preview() {
        let (_tmp, host, project) = fixture().await;
        let secret = "CONNECTION_SECRET_CANARY";
        let config = command(
            json!({"projectId":project,"providerId":"notion","label":"Notes","accessToken":secret}),
        );
        let first = host
            .action("project_connection_set", config.clone())
            .await
            .unwrap();
        assert_eq!(
            host.action("project_connection_set", config).await.unwrap(),
            first
        );
        let prepared=host.action("project_connection_prepare",command(json!({"projectId":project,"providerId":"notion","operation":"notion_page","arguments":{"parentPageId":uuid::Uuid::new_v4(),"title":"A preview","content":"Owner approved content"}}))).await.unwrap();
        assert_eq!(prepared["reasonCode"], "PROJECT_CONNECTION_PREPARED");
        host.action("project_connection_set",command(json!({"projectId":project,"providerId":"notion","label":"Other account","accessToken":"ANOTHER_SECRET_CANARY"}))).await.unwrap();
        let rejected=host.action("project_connection_execute",command(json!({"projectId":project,"preparedActionId":prepared["preparedActionId"],"expectedActionRevision":1}))).await.unwrap_err();
        assert_eq!(rejected.0, "PROJECT_CONNECTION_CHANGED");
        let events = host.store.read_events().unwrap();
        assert!(!serde_json::to_string(&events)
            .unwrap()
            .contains("SECRET_CANARY"));
        assert!(!events
            .iter()
            .any(|e| e["type"] == "project_studio.connection_dispatched"));
        assert!(!serde_json::to_string(&host.snapshot().unwrap())
            .unwrap()
            .contains("SECRET_CANARY"));
        host.shutdown().await;
    }
    #[tokio::test]
    async fn replay_rejects_orphaned_studio_facts_and_recovers_uncertain_dispatch() {
        let (_tmp, host, project_id) = fixture().await;
        host.action("project_connection_set",command(json!({"projectId":project_id,"providerId":"google_drive","label":"Drive","accessToken":"EXAMPLE"}))).await.unwrap();
        let prepared=host.action("project_connection_prepare",command(json!({"projectId":project_id,"providerId":"google_drive","operation":"drive_import","arguments":{"fileId":"sample"}}))).await.unwrap();
        let mut history = host.store.read_events().unwrap();
        let cmd = command(
            json!({"projectId":project_id,"preparedActionId":prepared["preparedActionId"],"expectedActionRevision":1}),
        );
        history.push(fact("owner.command.requested",json!({"commandId":cmd["commandId"],"action":"project_connection_execute","payload":cmd})));
        history.push(fact("project_studio.connection_dispatched",json!({"projectId":project_id,"commandId":cmd["commandId"],"preparedActionId":prepared["preparedActionId"],"expectedActionRevision":1})));
        let recovery = recover(&history).unwrap();
        assert_eq!(recovery.len(), 1);
        history.extend(recovery);
        assert_eq!(
            project(&history).unwrap()["connections"][0]["status"],
            "unknown"
        );
        assert!(recover(&history).unwrap().is_empty());
        history.retain(|e| e["type"] != "owner.command.requested");
        assert!(project(&history).is_err());
        host.shutdown().await;
    }
    #[tokio::test]
    async fn durable_resource_and_prepared_action_recover_exact_acknowledgements() {
        let (tmp, host, project) = fixture().await;
        let source = tmp.path().join("input.txt");
        std::fs::write(&source, b"Frozen").unwrap();
        let import =
            command(json!({"projectId":project,"expectedProjectRevision":1,"sourcePath":source}));
        let expected = host
            .action("project_resource_import", import.clone())
            .await
            .unwrap();
        host.action("project_connection_set",command(json!({"projectId":project,"providerId":"notion","label":"Notes","accessToken":"SYNTHETIC"}))).await.unwrap();
        let prepare = command(
            json!({"projectId":project,"providerId":"notion","operation":"notion_page","arguments":{"parentPageId":uuid::Uuid::new_v4(),"title":"Preview","content":"Body"}}),
        );
        let prepared = host
            .action("project_connection_prepare", prepare.clone())
            .await
            .unwrap();
        let events = host.store.read_events().unwrap();
        for (cmd, receipt) in [(&import, &expected), (&prepare, &prepared)] {
            let request = events
                .iter()
                .find(|e| {
                    e["type"] == "owner.command.requested"
                        && e["payload"]["commandId"] == cmd["commandId"]
                })
                .unwrap();
            let without_receipt = events
                .iter()
                .filter(|e| {
                    !(e["type"] == "owner.command.receipted"
                        && e["payload"]["commandId"] == cmd["commandId"])
                })
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(
                recovered_owner_result(&without_receipt, &request["payload"]).unwrap(),
                *receipt
            );
        }
        host.shutdown().await;
    }
    #[tokio::test]
    async fn native_provider_usage_is_persisted_with_unknown_cost_and_exact_run_scope() {
        let (_peer, programs) =
            crate::providers::managed_tests::peer("codex-managed-usage-complete");
        let tmp = tempfile::tempdir().unwrap();
        let host = Host::open_with_controller(
            tmp.path().join("data"),
            crate::providers::Controller::with_programs(programs),
        )
        .unwrap();
        host.store.append(vec![event("provider.observed",json!({"providerId":"codex","state":"available","version":"0.155.1","capabilities":{"recentObservation":true,"explicitLiveStatus":true},"observedAt":crate::now(),"sessionIds":[]}))]).unwrap();
        let project = host
            .action("project_create", command(json!({"name":"Usage"})))
            .await
            .unwrap()["projectId"]
            .clone();
        let queued=host.action("project_work_enqueue",command(json!({"projectId":project,"expectedProjectRevision":1,"abilityId":"D1","title":"Usage proof","instruction":"Create output"}))).await.unwrap();
        let started = host
            .action(
                "work_start",
                command(json!({"workItemId":queued["workItemId"],"expectedItemRevision":1})),
            )
            .await
            .unwrap();
        assert_eq!(started["outcome"], "applied", "{started}");
        for _ in 0..20 {
            host.background_tick(false).await.unwrap();
            if host
                .store
                .read_events()
                .unwrap()
                .iter()
                .any(|e| e["type"] == "provider.usage.reported")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let events = host.store.read_events().unwrap();
        let usage = &events
            .iter()
            .find(|e| e["type"] == "provider.usage.reported")
            .expect("actual usage must reach canonical events")["payload"];
        assert_eq!(usage["inputTokens"], 19);
        assert_eq!(usage["outputTokens"], 11);
        assert!(usage["costMicros"].is_null());
        assert!(usage["currency"].is_null());
        let work = domain::project_work(&events).unwrap();
        assert!(array(&work, "runs")
            .iter()
            .any(|r| r["runId"] == usage["runId"]
                && r["providerSessionId"] == usage["providerSessionId"]));
        let mut forged = events.clone();
        forged.last_mut().unwrap()["payload"]["providerSessionId"] = json!("foreign");
        assert!(domain::project_events(&forged).is_err());
        host.shutdown().await;
    }
}
