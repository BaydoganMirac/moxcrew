//! Durable owner goals and honest work cards; provider transcripts are never inferred.
use crate::{
    creation,
    work::{array, text},
    Error, Host, Result,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Read, path::Path};

const INPUT_LIMIT: u64 = 25 * 1024 * 1024;

fn invalid(code: &str) -> Error {
    Error(code.into())
}

fn origin<'a>(events: &'a [Value], work_id: &str) -> Option<&'a Value> {
    let mut work = work_id.to_owned();
    let mut seen = HashSet::new();
    while seen.insert(work.clone()) {
        let command = format!("cmd:{}", work.strip_prefix("work:")?);
        let request = &events.iter().find(|event| {
            event["type"] == "owner.command.requested" && event["payload"]["commandId"] == command
        })?["payload"];
        if matches!(
            request["action"].as_str(),
            Some("project_work_enqueue" | "project_chat_send")
        ) {
            return Some(request);
        }
        if request["action"] != "work_retry" {
            return None;
        }
        work = request["payload"]["workItemId"].as_str()?.to_owned();
    }
    None
}

pub(crate) struct FrozenInputs {
    pub files: Vec<(String, Vec<u8>)>,
    pub manifest: Vec<u8>,
}

pub(crate) fn copied_name(index: usize, path: &Path) -> String {
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .filter(|v| v.len() <= 12 && v.bytes().all(|b| b.is_ascii_alphanumeric()))
        .map(|v| format!(".{v}"))
        .unwrap_or_default();
    format!("reference-{:02}{extension}", index + 1)
}

pub(crate) fn copied_bytes(directory: &Path, path: &Path, remaining: u64) -> Result<Vec<u8>> {
    let file = creation::open_result(directory, path)?;
    let before = file.metadata()?;
    if before.len() > remaining {
        return Err(invalid("PROJECT_INPUT_LIMIT_EXCEEDED"));
    }
    let mut bytes = Vec::new();
    (&file).take(remaining + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 > remaining {
        return Err(invalid("PROJECT_INPUT_LIMIT_EXCEEDED"));
    }
    if before.len() != bytes.len() as u64
        || before.len() != after.len()
        || before.modified()? != after.modified()?
    {
        return Err(invalid("PROJECT_INPUT_CHANGED"));
    }
    Ok(bytes)
}

/// The manifest is a checked job file, not an immutable canonical hash record.
pub(crate) fn retry_inputs(
    events: &[Value],
    work: &Value,
    directory: &Path,
    project: &str,
) -> Result<Option<FrozenInputs>> {
    let owner = origin(events, text(work, "workItemId")?)
        .ok_or_else(|| invalid("PROJECT_JOB_SCOPE_INVALID"))?;
    let references = owner["payload"]
        .get("referencePaths")
        .and_then(Value::as_array);
    let Some(references) = references.filter(|references| !references.is_empty()) else {
        return Ok(None);
    };
    if owner["action"] != "project_chat_send" || references.len() > 8 {
        return Err(invalid("PROJECT_INPUT_CHANGED"));
    }
    let manifest_bytes = copied_bytes(directory, Path::new("inputs/manifest.json"), 64 * 1024)
        .map_err(|_| invalid("PROJECT_INPUT_UNAVAILABLE"))?;
    let manifest: Value =
        serde_json::from_slice(&manifest_bytes).map_err(|_| invalid("PROJECT_INPUT_CHANGED"))?;
    if manifest.as_object().is_none_or(|fields| fields.len() != 4)
        || manifest["schemaVersion"] != 1
        || manifest["projectId"] != project
        || manifest["replyToWorkItemId"] != owner["payload"]["replyToWorkItemId"]
    {
        return Err(invalid("PROJECT_INPUT_CHANGED"));
    }
    let entries = manifest["files"]
        .as_array()
        .filter(|entries| entries.len() == references.len())
        .ok_or_else(|| invalid("PROJECT_INPUT_CHANGED"))?;
    let mut files = Vec::new();
    let mut total = 0u64;
    for (index, (reference, entry)) in references.iter().zip(entries).enumerate() {
        let reference = reference
            .as_str()
            .ok_or_else(|| invalid("PROJECT_INPUT_CHANGED"))?;
        let path =
            creation::relative_result(reference).map_err(|_| invalid("PROJECT_INPUT_CHANGED"))?;
        let name = copied_name(index, &path);
        let expected = format!("inputs/{name}");
        if entry.as_object().is_none_or(|fields| fields.len() != 4)
            || entry["referencePath"] != reference
            || entry["inputPath"] != expected
        {
            return Err(invalid("PROJECT_INPUT_CHANGED"));
        }
        let bytes = copied_bytes(directory, Path::new(&expected), INPUT_LIMIT - total)
            .map_err(|_| invalid("PROJECT_INPUT_UNAVAILABLE"))?;
        if entry["bytes"].as_u64() != Some(bytes.len() as u64)
            || entry["sha256"] != format!("{:x}", Sha256::digest(&bytes))
        {
            return Err(invalid("PROJECT_INPUT_CHANGED"));
        }
        total += bytes.len() as u64;
        files.push((name, bytes));
    }
    Ok(Some(FrozenInputs {
        files,
        manifest: manifest_bytes,
    }))
}

pub(crate) fn read(
    data_dir: &Path,
    events: &[Value],
    projected: &Value,
    payload: &Value,
) -> Result<Value> {
    let project = text(payload, "projectId")?;
    if !array(&projected["projectMemory"], "projects")
        .iter()
        .any(|p| p["projectId"] == project)
    {
        return Err(invalid("PROJECT_NOT_FOUND"));
    }
    let offset = match payload.get("offset") {
        None => 0,
        Some(value) => usize::try_from(
            value
                .as_u64()
                .ok_or_else(|| invalid("PROJECT_CHAT_PAGE_INVALID"))?,
        )
        .map_err(|_| invalid("PROJECT_CHAT_PAGE_INVALID"))?,
    };
    let limit = payload
        .get("limit")
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| invalid("PROJECT_CHAT_PAGE_INVALID"))
        })
        .transpose()?
        .unwrap_or(20);
    if !(1..=50).contains(&limit) {
        return Err(invalid("PROJECT_CHAT_PAGE_INVALID"));
    }
    let jobs = creation::project_jobs(data_dir, events, projected)?;
    let mut entries = Vec::new();
    for job in jobs
        .as_array()
        .unwrap()
        .iter()
        .filter(|job| job["projectId"] == project)
    {
        let Some(work) = array(&projected["work"], "items")
            .iter()
            .find(|work| work["workItemId"] == job["workItemId"])
        else {
            continue;
        };
        let Some(owner) = origin(events, text(work, "workItemId")?) else {
            continue;
        };
        let sequence = events
            .iter()
            .position(|event| {
                event["type"] == "work.enqueued"
                    && event["payload"]["workItem"]["workItemId"] == work["workItemId"]
            })
            .unwrap_or(0);
        entries.push((work, job, &owner["payload"], sequence));
    }
    entries.sort_by(|(a, _, _, a_sequence), (b, _, _, b_sequence)| {
        a["createdAt"]
            .as_str()
            .cmp(&b["createdAt"].as_str())
            .then(a_sequence.cmp(b_sequence))
    });
    let total = entries.len();
    let page: Vec<_> = entries.into_iter().skip(offset).take(limit as usize).map(|(work,job,original,_)| json!({"workItemId":work["workItemId"],"abilityId":original["abilityId"],"goal":original["instruction"],"replyToWorkItemId":original.get("replyToWorkItemId").unwrap_or(&Value::Null),"status":work["status"],"revision":work["revision"],"createdAt":work["createdAt"],"runtimeIssue":job["runtimeIssue"]})).collect();
    let next = offset.saturating_add(page.len());
    Ok(
        json!({"reasonCode":"PROJECT_CHAT_READ","projectId":project,"workRevision":projected["work"]["revision"],"offset":offset,"limit":limit,"nextOffset":if next<total {json!(next)} else {Value::Null},"total":total,"items":page}),
    )
}

impl Host {
    pub(crate) async fn conversation_action(&self, action: &str, payload: &Value) -> Result<Value> {
        if action == "project_chat_read" {
            return self.store.read_conversation(payload);
        }
        if action != "project_chat_send" {
            return Err(invalid("UNKNOWN_ACTION"));
        }
        let references = match payload.get("referencePaths") {
            None => Vec::new(),
            Some(value) => value
                .as_array()
                .ok_or_else(|| invalid("PROJECT_INPUT_PATH_INVALID"))?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| invalid("PROJECT_INPUT_PATH_INVALID"))
                })
                .collect::<Result<Vec<_>>>()?,
        };
        if references.len() > 8
            || references.iter().collect::<HashSet<_>>().len() != references.len()
        {
            return Err(invalid("PROJECT_INPUT_LIMIT_EXCEEDED"));
        }
        let parent = payload
            .get("replyToWorkItemId")
            .map(|v| v.as_str().ok_or_else(|| invalid("PROJECT_JOB_NOT_FOUND")))
            .transpose()?;
        if parent.is_none() && !references.is_empty() {
            return Err(invalid("PROJECT_INPUT_PARENT_REQUIRED"));
        }
        let Some(parent) = parent else {
            return self.creation_enqueue(payload);
        };
        let selection = json!({"projectId":payload["projectId"],"workItemId":parent});
        let (directory, work) = if references.is_empty() {
            self.creation_parent(&selection)?
        } else {
            self.creation_scope(&selection)?
        };
        if !matches!(
            work["status"].as_str(),
            Some("completed" | "failed" | "interrupted")
        ) {
            return Err(invalid("PROJECT_INPUT_PARENT_ACTIVE"));
        }
        let events = self.store.read_events()?;
        let original = origin(&events, parent).ok_or_else(|| invalid("PROJECT_JOB_NOT_FOUND"))?;
        let mut composed = payload.clone();
        composed["instruction"] = json!(format!("{}\n\nThis follows a prior owner request in the same Project:\n{}\nUse only the explicitly copied reference files, if any; prior output content is not automatically available.", text(payload,"instruction")?, text(&original["payload"],"instruction")?));
        if references.is_empty() {
            return self.creation_enqueue(&composed);
        }
        let mut copies = Vec::new();
        let mut files = Vec::new();
        let mut total = 0u64;
        for (index, reference) in references.iter().enumerate() {
            let path = creation::relative_result(reference)?;
            let bytes = copied_bytes(&directory, &path, INPUT_LIMIT - total)?;
            total += bytes.len() as u64;
            let name = copied_name(index, &path);
            files.push(json!({"referencePath":reference,"inputPath":format!("inputs/{name}"),"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes))}));
            copies.push((name, bytes));
        }
        let manifest = json!({"schemaVersion":1,"projectId":payload["projectId"],"replyToWorkItemId":parent,"files":files});
        self.creation_enqueue_inputs(&composed, &copies, Some(&manifest))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    fn command(mut payload: Value) -> Value {
        payload["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
        payload
    }
    #[tokio::test]
    async fn real_host_conversation_runs_a_native_peer_and_copies_its_actual_output() {
        let (_peer, programs) = crate::providers::managed_tests::peer("codex-managed-complete");
        let directory = tempfile::tempdir().unwrap();
        let host = Host::open_with_controller(
            directory.path().join("data"),
            crate::providers::Controller::with_programs(programs),
        )
        .unwrap();
        host.store.append(vec![crate::event("provider.observed",json!({"providerId":"codex","state":"available","version":"0.155.1","capabilities":{"recentObservation":true,"explicitLiveStatus":true},"observedAt":crate::now(),"sessionIds":[]}))]).unwrap();
        let project = host
            .action(
                "project_create",
                command(json!({"name":"Conversation proof"})),
            )
            .await
            .unwrap()["projectId"]
            .clone();
        let queued = host.action("project_chat_send",command(json!({"projectId":project,"expectedProjectRevision":1,"abilityId":"D1","title":"Initial brief","instruction":"Write an actual brief."}))).await.unwrap();
        let parent = &queued["workItemId"];
        let started = host
            .action(
                "work_start",
                command(json!({"workItemId":parent,"expectedItemRevision":1})),
            )
            .await
            .unwrap();
        assert_eq!(started["outcome"], "applied", "{started}");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        host.background_tick(false).await.unwrap();
        let snapshot = host.snapshot().unwrap();
        let revision = snapshot["projectMemory"]["projects"][0]["revision"].clone();
        let next = host.action("project_chat_send",command(json!({"projectId":project,"expectedProjectRevision":revision,"abilityId":"D1","title":"Revision","instruction":"Shorten this brief.","replyToWorkItemId":parent,"referencePaths":["outputs/summary.md"]}))).await.unwrap();
        assert_eq!(next["outcome"], "applied");
        let (job, _) = host
            .creation_parent(&json!({"projectId":project,"workItemId":next["workItemId"]}))
            .unwrap();
        let bytes = std::fs::read(job.join("inputs/reference-01.md")).unwrap();
        assert!(std::str::from_utf8(&bytes)
            .unwrap()
            .contains("Native fixture artifact"));
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(job.join("inputs/manifest.json")).unwrap())
                .unwrap();
        assert_eq!(
            manifest["files"][0]["sha256"],
            format!("{:x}", Sha256::digest(&bytes))
        );
        let history = host
            .action("project_chat_read", json!({"projectId":project}))
            .await
            .unwrap();
        assert_eq!(history["total"], 2);
        assert_eq!(history["items"][0]["status"], "completed");
        assert_eq!(history["items"][1]["goal"], "Shorten this brief.");
        assert_eq!(history["items"][1]["status"], "queued");
        host.shutdown().await;
    }
}
