//! Explicit Project app operations. The Host owns approval, dispatch and recovery.
//! Secrets stay in private files; native response/error bodies never enter receipts.
use crate::{store, Error, Result};
use chrono::DateTime;
use reqwest::{Client, Method, Url};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

const APPS: [(&str, &str); 3] = [
    ("google_drive", "drive_import"),
    ("notion", "notion_page"),
    ("google_calendar", "calendar_event"),
];
const CONFIG_LIMIT: usize = 1024 * 1024;
const JSON_LIMIT: usize = 256 * 1024;
const FILE_LIMIT: usize = 25 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(10);
fn fail(code: &str) -> Error {
    Error(code.into())
}
fn require(ok: bool, code: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(fail(code))
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":_-".contains(&b))
}
fn app_operation(app: &str) -> Result<&'static str> {
    APPS.iter()
        .find(|(id, _)| *id == app)
        .map(|(_, op)| *op)
        .ok_or_else(|| fail("PROJECT_CONNECTION_UNSUPPORTED"))
}
fn label_ok(s: &str) -> bool {
    !s.trim().is_empty() && s.chars().count() <= 120 && !s.chars().any(char::is_control)
}
fn token_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 8192 && s.bytes().all(|b| (33..=126).contains(&b))
}

fn path(data: &Path, project: &str) -> Result<PathBuf> {
    require(identity(project), "PROJECT_CONNECTION_INVALID_PROJECT")?;
    Ok(data
        .join("app-connections")
        .join(format!("{}.json", digest(project.as_bytes()))))
}
fn config(data: &Path, project: &str) -> Result<Value> {
    let path = path(data, project)?;
    if path
        .parent()
        .unwrap()
        .try_exists()
        .map_err(|_| fail("PROJECT_CONNECTION_STORAGE_FAILED"))?
    {
        store::private_directory(path.parent().unwrap())
            .map_err(|_| fail("PROJECT_CONNECTION_STORAGE_FAILED"))?;
    }
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"credentials":{},"receipts":{},"revisions":{}}))
        }
        Err(_) => return Err(fail("PROJECT_CONNECTION_STORAGE_FAILED")),
        _ => {}
    }
    let mut bytes = Vec::new();
    store::open_private_reader(&path)
        .map_err(|_| fail("PROJECT_CONNECTION_STORAGE_FAILED"))?
        .take((CONFIG_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| fail("PROJECT_CONNECTION_STORAGE_FAILED"))?;
    require(
        bytes.len() <= CONFIG_LIMIT,
        "PROJECT_CONNECTION_CONFIG_LIMIT",
    )?;
    let mut value: Value =
        serde_json::from_slice(&bytes).map_err(|_| fail("PROJECT_CONNECTION_CONFIG_INVALID"))?;
    require(
        value["credentials"].is_object() && value["receipts"].is_object(),
        "PROJECT_CONNECTION_CONFIG_INVALID",
    )?;
    if value.get("revisions").is_none() {
        value["revisions"] = json!({});
    }
    require(
        value["revisions"].as_object().is_some_and(|o| {
            o.iter().all(|(id, v)| {
                app_operation(id).is_ok() && v.as_u64().is_some_and(|r| r <= 1_000_000_000)
            })
        }),
        "PROJECT_CONNECTION_CONFIG_INVALID",
    )?;
    for (id, credential) in value["credentials"].as_object().unwrap() {
        require(
            app_operation(id).is_ok()
                && credential["label"].as_str().is_some_and(label_ok)
                && credential["token"].as_str().is_some_and(token_ok),
            "PROJECT_CONNECTION_CONFIG_INVALID",
        )?;
    }
    Ok(value)
}
fn entry(config: &Value, app: &str) -> Value {
    let configured = config["credentials"][app].is_object();
    json!({"providerId":app,"label":config["credentials"][app]["label"].as_str().unwrap_or(""),"configured":configured,"connected":configured,"connectionRevision":config["revisions"][app].as_u64().unwrap_or(0),"authentication":"unverified","operations":[app_operation(app).unwrap()]})
}
pub(crate) fn metadata(data: &Path, project: &str) -> Result<Value> {
    let config = config(data, project)?;
    Ok(json!(APPS
        .iter()
        .map(|(app, _)| entry(&config, app))
        .collect::<Vec<_>>()))
}
fn change(
    data: &Path,
    project: &str,
    app: &str,
    credential: Option<Value>,
    command: &str,
) -> Result<Value> {
    app_operation(app)?;
    require(identity(command), "PROJECT_CONNECTION_INVALID_COMMAND")?;
    let mut config = config(data, project)?;
    let command_hash = digest(command.as_bytes());
    let intent_hash = digest(
        serde_json::to_vec(&json!([project, app, credential]))
            .unwrap()
            .as_slice(),
    );
    if let Some(receipt) = config["receipts"].get(&command_hash) {
        require(
            receipt["digest"] == intent_hash,
            "PROJECT_CONNECTION_COMMAND_CONFLICT",
        )?;
        // Return only our metadata shape, even if private state was externally modified.
        let result = &receipt["result"];
        require(
            result["providerId"] == app
                && result["connected"].is_boolean()
                && result["label"]
                    .as_str()
                    .is_some_and(|s| s.is_empty() || label_ok(s)),
            "PROJECT_CONNECTION_CONFIG_INVALID",
        )?;
        return Ok(
            json!({"providerId":app,"label":result["label"],"configured":result["connected"],"connected":result["connected"],"connectionRevision":result["connectionRevision"].as_u64().unwrap_or(0),"authentication":"unverified","operations":[app_operation(app)?]}),
        );
    }
    require(
        config["receipts"].as_object().unwrap().len() < 1024,
        "PROJECT_CONNECTION_CONFIG_LIMIT",
    )?;
    let revision = config["revisions"][app].as_u64().unwrap_or(0);
    require(revision < 1_000_000_000, "PROJECT_CONNECTION_CONFIG_LIMIT")?;
    config["revisions"][app] = json!(revision + 1);
    if let Some(value) = credential {
        config["credentials"][app] = value;
    } else {
        config["credentials"].as_object_mut().unwrap().remove(app);
    }
    let result = entry(&config, app);
    config["receipts"][command_hash] = json!({"digest":intent_hash,"result":result});
    let bytes =
        serde_json::to_vec(&config).map_err(|_| fail("PROJECT_CONNECTION_CONFIG_INVALID"))?;
    require(
        bytes.len() <= CONFIG_LIMIT,
        "PROJECT_CONNECTION_CONFIG_LIMIT",
    )?;
    store::atomic_write(&path(data, project)?, &bytes)
        .map_err(|_| fail("PROJECT_CONNECTION_STORAGE_FAILED"))?;
    Ok(result)
}
pub(crate) fn configure(
    data: &Path,
    project: &str,
    app: &str,
    label: &str,
    token: &str,
    command: &str,
) -> Result<Value> {
    require(
        label_ok(label) && token_ok(token),
        "PROJECT_CONNECTION_INVALID_CREDENTIAL",
    )?;
    change(
        data,
        project,
        app,
        Some(json!({"label":label,"token":token})),
        command,
    )
}
pub(crate) fn remove(data: &Path, project: &str, app: &str, command: &str) -> Result<Value> {
    change(data, project, app, None, command)
}
fn text<'a>(value: &'a Value, key: &str, min: usize, max: usize) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|s| s.chars().count() >= min && s.chars().count() <= max && !s.contains('\0'))
        .ok_or_else(|| fail("PROJECT_CONNECTION_INVALID_ARGUMENTS"))
}
fn exact(value: &Value, keys: &[&str]) -> Result<()> {
    require(
        value
            .as_object()
            .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k))),
        "PROJECT_CONNECTION_INVALID_ARGUMENTS",
    )
}
pub(crate) fn validate_prepare(app: &str, operation: &str, args: &Value) -> Result<Value> {
    require(
        app_operation(app)? == operation,
        "PROJECT_CONNECTION_UNSUPPORTED",
    )?;
    require(
        serde_json::to_vec(args)
            .map_err(|_| fail("PROJECT_CONNECTION_INVALID_ARGUMENTS"))?
            .len()
            <= JSON_LIMIT,
        "PROJECT_CONNECTION_INPUT_LIMIT",
    )?;
    match operation {
        "drive_import" => {
            exact(args, &["fileId"])?;
            let id = text(args, "fileId", 1, 256)?;
            require(
                id.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)),
                "PROJECT_CONNECTION_INVALID_ARGUMENTS",
            )?;
        }
        "notion_page" => {
            exact(args, &["parentPageId", "title", "content"])?;
            require(
                uuid::Uuid::parse_str(text(args, "parentPageId", 32, 36)?).is_ok(),
                "PROJECT_CONNECTION_INVALID_ARGUMENTS",
            )?;
            require(
                !text(args, "title", 1, 2000)?.trim().is_empty(),
                "PROJECT_CONNECTION_INVALID_ARGUMENTS",
            )?;
            text(args, "content", 0, 100_000)?;
        }
        "calendar_event" => {
            exact(
                args,
                &[
                    "calendarId",
                    "summary",
                    "description",
                    "start",
                    "end",
                    "timeZone",
                ],
            )?;
            let calendar = text(args, "calendarId", 1, 256)?;
            require(
                calendar
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"@._-".contains(&b)),
                "PROJECT_CONNECTION_INVALID_ARGUMENTS",
            )?;
            require(
                !text(args, "summary", 1, 1000)?.trim().is_empty(),
                "PROJECT_CONNECTION_INVALID_ARGUMENTS",
            )?;
            text(args, "description", 0, 16_000)?;
            let start = DateTime::parse_from_rfc3339(text(args, "start", 1, 64)?)
                .map_err(|_| fail("PROJECT_CONNECTION_INVALID_ARGUMENTS"))?;
            let end = DateTime::parse_from_rfc3339(text(args, "end", 1, 64)?)
                .map_err(|_| fail("PROJECT_CONNECTION_INVALID_ARGUMENTS"))?;
            require(end > start, "PROJECT_CONNECTION_INVALID_ARGUMENTS")?;
            // Shape only: the API verifies that the IANA identifier exists.
            let zone = text(args, "timeZone", 1, 128)?;
            require(
                zone.split('/')
                    .all(|s| !s.is_empty() && s != "." && s != "..")
                    && zone
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"/_+-".contains(&b)),
                "PROJECT_CONNECTION_INVALID_ARGUMENTS",
            )?;
        }
        _ => return Err(fail("PROJECT_CONNECTION_UNSUPPORTED")),
    }
    Ok(args.clone())
}

pub(crate) struct Output {
    pub value: Value,
    pub files: Vec<(String, Vec<u8>)>,
}
struct Transport {
    client: Client,
    #[cfg(test)]
    endpoint: Option<Url>,
}
impl Transport {
    fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .https_only(true)
                .timeout(DEADLINE)
                .build()
                .map_err(|_| fail("PROJECT_CONNECTION_UNAVAILABLE"))?,
            #[cfg(test)]
            endpoint: None,
        })
    }
    fn url(&self, app: &str, parts: &[&str]) -> Url {
        #[cfg(test)]
        if let Some(base) = &self.endpoint {
            let mut url = base.clone();
            url.path_segments_mut().unwrap().extend(parts);
            return url;
        }
        let mut url = Url::parse(if app == "notion" {
            "https://api.notion.com/"
        } else {
            "https://www.googleapis.com/"
        })
        .unwrap();
        url.path_segments_mut().unwrap().extend(parts);
        url
    }
    async fn request(
        &self,
        app: &str,
        token: &str,
        method: Method,
        url: Url,
        body: Option<Value>,
        limit: usize,
    ) -> Result<Vec<u8>> {
        let mut request = self.client.request(method.clone(), url).bearer_auth(token);
        if app == "notion" {
            request = request.header("Notion-Version", "2026-03-11");
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let write = method != Method::GET;
        let uncertain = || {
            fail(if write {
                "PROJECT_CONNECTION_OUTCOME_UNKNOWN"
            } else {
                "PROJECT_CONNECTION_REQUEST_FAILED"
            })
        };
        let mut response = request.send().await.map_err(|_| uncertain())?;
        let status = response.status();
        if !status.is_success() {
            return Err(fail(match status.as_u16() {
                401 => "PROJECT_CONNECTION_AUTH_REQUIRED",
                403 => "PROJECT_CONNECTION_PERMISSION_DENIED",
                404 => "PROJECT_CONNECTION_NOT_FOUND",
                409 => "PROJECT_CONNECTION_CONFLICT",
                429 => "PROJECT_CONNECTION_RATE_LIMITED",
                400 | 422 => "PROJECT_CONNECTION_REMOTE_REJECTED",
                300..=399 => "PROJECT_CONNECTION_REDIRECT_REFUSED",
                _ if write => "PROJECT_CONNECTION_OUTCOME_UNKNOWN",
                _ => "PROJECT_CONNECTION_REQUEST_FAILED",
            }));
        }
        if response.content_length().is_some_and(|n| n > limit as u64) {
            return Err(fail(if write {
                "PROJECT_CONNECTION_OUTCOME_UNKNOWN"
            } else {
                "PROJECT_CONNECTION_OUTPUT_LIMIT"
            }));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| uncertain())? {
            if bytes.len().saturating_add(chunk.len()) > limit {
                return Err(fail(if write {
                    "PROJECT_CONNECTION_OUTCOME_UNKNOWN"
                } else {
                    "PROJECT_CONNECTION_OUTPUT_LIMIT"
                }));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}
fn json_response(bytes: &[u8], write: bool) -> Result<Value> {
    serde_json::from_slice(bytes).map_err(|_| {
        fail(if write {
            "PROJECT_CONNECTION_OUTCOME_UNKNOWN"
        } else {
            "PROJECT_CONNECTION_INVALID_RESPONSE"
        })
    })
}
fn filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .take(120)
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim_matches('.');
    let stem = safe.split('.').next().unwrap_or("").to_ascii_uppercase();
    if safe.is_empty()
        || [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&stem.as_str())
    {
        format!("drive-{safe}.bin")
    } else {
        safe.into()
    }
}
fn calendar_id(project: &str, action: &str) -> String {
    digest(
        serde_json::to_vec(&json!(["moxcrew-calendar-v1", project, action]))
            .unwrap()
            .as_slice(),
    )
}
async fn perform(
    transport: &Transport,
    token: &str,
    project: &str,
    action: &str,
    app: &str,
    args: &Value,
) -> Result<Output> {
    match app {
        "google_drive" => {
            let file = args["fileId"].as_str().unwrap();
            let mut url = transport.url(app, &["drive", "v3", "files", file]);
            url.query_pairs_mut()
                .append_pair("fields", "id,name,mimeType,size");
            let meta = json_response(
                &transport
                    .request(app, token, Method::GET, url, None, JSON_LIMIT)
                    .await?,
                false,
            )?;
            require(meta["id"] == file, "PROJECT_CONNECTION_INVALID_RESPONSE")?;
            let mime = meta["mimeType"]
                .as_str()
                .filter(|s| s.len() <= 128 && s.is_ascii())
                .ok_or_else(|| fail("PROJECT_CONNECTION_INVALID_RESPONSE"))?;
            require(
                !mime.starts_with("application/vnd.google-apps."),
                "PROJECT_CONNECTION_GOOGLE_EXPORT_REQUIRED",
            )?;
            let size = meta["size"]
                .as_str()
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| fail("PROJECT_CONNECTION_INVALID_RESPONSE"))?;
            require(size <= FILE_LIMIT, "PROJECT_CONNECTION_OUTPUT_LIMIT")?;
            let name = filename(
                meta["name"]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 4096)
                    .ok_or_else(|| fail("PROJECT_CONNECTION_INVALID_RESPONSE"))?,
            );
            let mut url = transport.url(app, &["drive", "v3", "files", file]);
            url.query_pairs_mut().append_pair("alt", "media");
            let bytes = transport
                .request(app, token, Method::GET, url, None, FILE_LIMIT)
                .await?;
            require(bytes.len() == size, "PROJECT_CONNECTION_FILE_CHANGED")?;
            Ok(Output {
                value: json!({"providerId":app,"operation":"drive_import","fileId":file,"name":name,"bytes":bytes.len()}),
                files: vec![(name, bytes)],
            })
        }
        "notion" => {
            let content = args["content"].as_str().unwrap();
            let chars: Vec<_> = content.chars().collect();
            let blocks: Vec<_> = chars.chunks(2000).map(|part|json!({"object":"block","type":"paragraph","paragraph":{"rich_text":[{"type":"text","text":{"content":part.iter().collect::<String>()}}]}})).collect();
            let body = json!({"parent":{"page_id":args["parentPageId"]},"properties":{"title":{"title":[{"type":"text","text":{"content":args["title"]}}]}},"children":blocks});
            let value = json_response(
                &transport
                    .request(
                        app,
                        token,
                        Method::POST,
                        transport.url(app, &["v1", "pages"]),
                        Some(body),
                        JSON_LIMIT,
                    )
                    .await?,
                true,
            )?;
            let id = value["id"]
                .as_str()
                .filter(|s| uuid::Uuid::parse_str(s).is_ok())
                .ok_or_else(|| fail("PROJECT_CONNECTION_OUTCOME_UNKNOWN"))?;
            require(
                value["object"] == "page",
                "PROJECT_CONNECTION_OUTCOME_UNKNOWN",
            )?;
            Ok(Output {
                value: json!({"providerId":app,"operation":"notion_page","pageId":id}),
                files: vec![],
            })
        }
        "google_calendar" => {
            let id = calendar_id(project, action);
            let body = json!({"id":id,"summary":args["summary"],"description":args["description"],"start":{"dateTime":args["start"],"timeZone":args["timeZone"]},"end":{"dateTime":args["end"],"timeZone":args["timeZone"]}});
            let url = transport.url(
                app,
                &[
                    "calendar",
                    "v3",
                    "calendars",
                    args["calendarId"].as_str().unwrap(),
                    "events",
                ],
            );
            let value = json_response(
                &transport
                    .request(app, token, Method::POST, url, Some(body), JSON_LIMIT)
                    .await?,
                true,
            )?;
            require(
                value["id"] == id && value["status"] != "cancelled",
                "PROJECT_CONNECTION_OUTCOME_UNKNOWN",
            )?;
            Ok(Output {
                value: json!({"providerId":app,"operation":"calendar_event","calendarId":args["calendarId"],"eventId":id}),
                files: vec![],
            })
        }
        _ => Err(fail("PROJECT_CONNECTION_UNSUPPORTED")),
    }
}
pub(crate) async fn execute(
    data: &Path,
    project: &str,
    action: &str,
    app: &str,
    operation: &str,
    args: &Value,
) -> Result<Output> {
    let transport = Transport::new()?;
    execute_with(&transport, data, project, action, app, operation, args).await
}
async fn execute_with(
    transport: &Transport,
    data: &Path,
    project: &str,
    action: &str,
    app: &str,
    operation: &str,
    args: &Value,
) -> Result<Output> {
    validate_prepare(app, operation, args)?;
    require(identity(action), "PROJECT_CONNECTION_INVALID_ACTION")?;
    let config = config(data, project)?;
    let token = config["credentials"][app]["token"]
        .as_str()
        .ok_or_else(|| fail("PROJECT_CONNECTION_AUTH_REQUIRED"))?;
    // The deadline covers both Drive requests and their bodies; no automatic retry.
    tokio::time::timeout(
        DEADLINE,
        perform(transport, token, project, action, app, args),
    )
    .await
    .map_err(|_| {
        fail(if app == "google_drive" {
            "PROJECT_CONNECTION_TIMEOUT"
        } else {
            "PROJECT_CONNECTION_OUTCOME_UNKNOWN"
        })
    })?
}

#[cfg(test)]
#[path = "app_connections_tests.rs"]
mod tests;
