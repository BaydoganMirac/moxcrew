//! Ephemeral project retrieval. Search never writes facts, caches, or queries.
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashSet;

const MAX_SAFE: u64 = 9_007_199_254_740_991;
pub const MODEL_ID: &str = "Xenova/paraphrase-multilingual-mpnet-base-v2";
pub const MODEL_REVISION: &str = "e5d116277351513fd260955ece953ecddde7046e";

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
fn exact(value: &Value, fields: &[&str]) -> Result<()> {
    require(
        value.as_object().is_some_and(|o| {
            o.len() == fields.len() && o.keys().all(|k| fields.contains(&k.as_str()))
        }),
        "MEMORY_PROTOCOL_INVALID",
    )
}
fn integer(value: &Value, min: u64, max: u64) -> Result<u64> {
    value
        .as_u64()
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| invalid("MEMORY_PROTOCOL_INVALID"))
}
fn identity(value: &Value, prefix: &str) -> Result<()> {
    let text = value
        .as_str()
        .and_then(|s| s.strip_prefix(prefix))
        .ok_or_else(|| invalid("MEMORY_PROTOCOL_INVALID"))?;
    let uuid = uuid::Uuid::parse_str(text).map_err(|_| invalid("MEMORY_PROTOCOL_INVALID"))?;
    require(
        uuid.to_string() == text
            && uuid.get_variant() == uuid::Variant::RFC4122
            && (1..=8).contains(&uuid.get_version_num()),
        "MEMORY_PROTOCOL_INVALID",
    )
}
fn printable(text: &str) -> bool {
    !text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
}

/// Normalize only the read input, preserving the owner's stored ADR text.
pub fn request(payload: &Value) -> Result<Value> {
    let o = payload
        .as_object()
        .ok_or_else(|| invalid("INVALID_SEARCH_FIELDS"))?;
    require(
        o.contains_key("projectId")
            && o.contains_key("query")
            && o.keys()
                .all(|k| ["projectId", "query", "limit"].contains(&k.as_str())),
        "INVALID_SEARCH_FIELDS",
    )?;
    identity(&payload["projectId"], "project:").map_err(|_| invalid("INVALID_PROJECT_ID"))?;
    let query = payload["query"]
        .as_str()
        .ok_or_else(|| invalid("INVALID_SEARCH_QUERY"))?;
    require(printable(query), "INVALID_SEARCH_QUERY")?;
    let query = query.trim();
    require(
        !query.is_empty() && query.encode_utf16().count() <= 400,
        "INVALID_SEARCH_QUERY",
    )?;
    let limit = payload
        .get("limit")
        .map(|v| integer(v, 1, 10))
        .transpose()
        .map_err(|_| invalid("INVALID_SEARCH_LIMIT"))?
        .unwrap_or(5);
    Ok(json!({"projectId":payload["projectId"],"query":query,"limit":limit}))
}

fn folded(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if matches!(c, 'I' | 'İ' | 'ı') {
                'i'.to_lowercase()
            } else {
                c.to_lowercase()
            }
        })
        .collect()
}

/// Weighted distinct-token overlap, deliberately labeled keyword, not semantic.
/// Only a bounded result list is allocated; no derived full-text index is kept.
pub fn search_local(projection: &Value, payload: &Value) -> Result<Value> {
    let input = request(payload)?;
    let manifest = projection["projects"]
        .as_array()
        .and_then(|ps| ps.iter().find(|p| p["projectId"] == input["projectId"]))
        .ok_or_else(|| invalid("PROJECT_NOT_FOUND"))?;
    let query = folded(input["query"].as_str().unwrap());
    let terms: HashSet<_> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let mut matches = Vec::with_capacity(input["limit"].as_u64().unwrap() as usize + 1);
    for decision in projection["decisions"]
        .as_array()
        .ok_or_else(|| invalid("INVALID_DECISION_PROJECTION"))?
        .iter()
        .filter(|d| {
            !terms.is_empty() && d["projectId"] == input["projectId"] && d["status"] == "accepted"
        })
    {
        let mut score = 0;
        for (field, weight) in [
            ("title", 4),
            ("decision", 2),
            ("context", 1),
            ("rationale", 1),
            ("consequences", 1),
        ] {
            let text = folded(decision[field].as_str().unwrap_or(""));
            let hits: HashSet<_> = text
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| terms.contains(s))
                .collect();
            score += hits.len() * weight;
        }
        if score == 0 {
            continue;
        }
        matches.push(json!({"decisionId":decision["decisionId"],"revision":decision["revision"],"title":decision["title"],"score":score}));
        matches.sort_unstable_by(|a, b| {
            b["score"]
                .as_u64()
                .cmp(&a["score"].as_u64())
                .then(a["decisionId"].as_str().cmp(&b["decisionId"].as_str()))
        });
        matches.truncate(input["limit"].as_u64().unwrap() as usize);
    }
    Ok(
        json!({"schemaVersion":1,"reasonCode":"DECISION_SEARCH_READ","projectId":input["projectId"],"revision":manifest["revision"],"query":input["query"],"limit":input["limit"],"mode":"keyword","matches":matches}),
    )
}

/// Reject a different scope/model space and all unbounded or ambiguous rows.
pub fn validate_search_response(response: &Value, shared: &Value, limit: u64) -> Result<()> {
    require((1..=10).contains(&limit), "MEMORY_PROTOCOL_INVALID")?;
    exact(
        response,
        &[
            "schemaVersion",
            "sharedProjectId",
            "manifestRevision",
            "mode",
            "model",
            "index",
            "matches",
        ],
    )?;
    integer(&response["schemaVersion"], 1, 1)?;
    identity(&response["sharedProjectId"], "shared:")?;
    require(
        response["sharedProjectId"] == *shared,
        "MEMORY_PROTOCOL_INVALID",
    )?;
    integer(&response["manifestRevision"], 0, MAX_SAFE)?;
    let semantic = match response["mode"].as_str() {
        Some("semantic") => true,
        Some("keyword") => false,
        _ => return Err(invalid("MEMORY_PROTOCOL_INVALID")),
    };
    let model = &response["model"];
    if semantic {
        exact(
            model,
            &[
                "id",
                "revision",
                "dimensions",
                "normalization",
                "dtype",
                "pooling",
                "chunkVersion",
            ],
        )?;
        require(
            model["id"] == MODEL_ID
                && model["revision"] == MODEL_REVISION
                && model["normalization"] == "l2"
                && model["dtype"] == "q8"
                && model["pooling"] == "mean",
            "MEMORY_PROTOCOL_INVALID",
        )?;
        integer(&model["dimensions"], 768, 768)?;
        integer(&model["chunkVersion"], 1, 1)?;
    } else {
        require(model.is_null(), "MEMORY_PROTOCOL_INVALID")?;
    }
    let index = &response["index"];
    exact(index, &["state", "indexed", "total"])?;
    let total = integer(&index["total"], 0, 64)?;
    let indexed = integer(&index["indexed"], 0, total)?;
    require(
        matches!(
            index["state"].as_str(),
            Some("ready" | "pending" | "failed" | "unavailable")
        ),
        "MEMORY_PROTOCOL_INVALID",
    )?;
    require(
        (index["state"] != "ready" || indexed == total) && (!semantic || index["state"] == "ready"),
        "MEMORY_PROTOCOL_INVALID",
    )?;
    let rows = response["matches"]
        .as_array()
        .ok_or_else(|| invalid("MEMORY_PROTOCOL_INVALID"))?;
    require(
        rows.len() <= limit as usize && rows.len() <= total as usize,
        "MEMORY_PROTOCOL_INVALID",
    )?;
    let mut seen = HashSet::new();
    let mut prior: Option<(f64, &str)> = None;
    for row in rows {
        exact(row, &["decisionId", "revision", "title", "score"])?;
        identity(&row["decisionId"], "decision:")?;
        integer(&row["revision"], 1, MAX_SAFE)?;
        let title = row["title"]
            .as_str()
            .ok_or_else(|| invalid("MEMORY_PROTOCOL_INVALID"))?;
        require(
            !title.is_empty()
                && title.trim() == title
                && title.encode_utf16().count() <= 160
                && !title
                    .chars()
                    .any(|c| matches!(c as u32, 0..=8 | 11..=12 | 14..=31 | 127)),
            "MEMORY_PROTOCOL_INVALID",
        )?;
        let score = row["score"]
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or_else(|| invalid("MEMORY_PROTOCOL_INVALID"))?;
        require(
            if semantic {
                (-1.0..=1.0).contains(&score)
            } else {
                score >= 0.0
            },
            "MEMORY_PROTOCOL_INVALID",
        )?;
        let id = row["decisionId"].as_str().unwrap();
        require(
            seen.insert(id)
                && prior.is_none_or(|(value, key)| value > score || (value == score && key < id)),
            "MEMORY_PROTOCOL_INVALID",
        )?;
        prior = Some((score, id));
    }
    Ok(())
}
