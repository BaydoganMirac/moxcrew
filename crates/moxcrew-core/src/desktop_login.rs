//! Browser account entry. Pending proofs and callback results exist only in RAM.
use crate::{Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::{RequestBuilder, Url};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn error(code: &str) -> Error {
    Error(code.into())
}
fn require(ok: bool) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(error("DESKTOP_PROTOCOL_INVALID"))
    }
}
fn exact(value: &Value, keys: &[&str]) -> Result<()> {
    require(
        value
            .as_object()
            .is_some_and(|m| m.len() == keys.len() && m.keys().all(|k| keys.contains(&k.as_str()))),
    )
}
fn opaque(value: &Value) -> Result<&str> {
    value
        .as_str()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        })
        .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))
}
fn canonical_time(value: &Value) -> Result<()> {
    let text = value
        .as_str()
        .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
    let parsed =
        DateTime::parse_from_rfc3339(text).map_err(|_| error("DESKTOP_PROTOCOL_INVALID"))?;
    require(
        parsed
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, true)
            == text,
    )
}
pub fn validate_cloud_origin(value: &str) -> Result<String> {
    let normalized =
        crate::cloud::normalize_url(value).map_err(|_| error("DESKTOP_ORIGIN_INVALID"))?;
    let url = Url::parse(&normalized).map_err(|_| error("DESKTOP_ORIGIN_INVALID"))?;
    if url.scheme() == "http" && !matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "::1")) {
        return Err(error("DESKTOP_ORIGIN_INVALID"));
    }
    Ok(normalized)
}
fn redirect_uri(value: &str) -> Result<()> {
    let port = value
        .strip_prefix("http://127.0.0.1:")
        .and_then(|s| s.strip_suffix("/auth/callback"))
        .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
    let number = port
        .parse::<u16>()
        .map_err(|_| error("DESKTOP_PROTOCOL_INVALID"))?;
    require(number != 0 && number.to_string() == port)
}
fn hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn base64url43(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
pub fn challenge(verifier: &str) -> Result<String> {
    require(hex64(verifier))?;
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())))
}
pub fn validate_authorization_url(value: &str, origin: &str) -> Result<()> {
    let origin = validate_cloud_origin(origin)?;
    let url = Url::parse(value).map_err(|_| error("DESKTOP_PROTOCOL_INVALID"))?;
    require(
        url.origin().ascii_serialization() == origin
            && url.path() == "/desktop/authorize"
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none(),
    )?;
    let mut fields = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        require(fields.insert(key.to_string(), value.to_string()).is_none())?;
    }
    let keys = [
        "schemaVersion",
        "installationId",
        "displayName",
        "appVersion",
        "protocolVersion",
        "osFamily",
        "redirectUri",
        "state",
        "codeChallenge",
    ];
    require(fields.len() == keys.len() && keys.iter().all(|k| fields.contains_key(*k)))?;
    require(
        fields["schemaVersion"] == "1"
            && fields["protocolVersion"] == "2"
            && matches!(fields["osFamily"].as_str(), "darwin" | "linux" | "windows")
            && hex64(&fields["state"])
            && base64url43(&fields["codeChallenge"]),
    )?;
    opaque(&json!(fields["installationId"]))?;
    let display = &fields["displayName"];
    require(
        !display.is_empty()
            && display.trim() == display
            && display.encode_utf16().count() <= 80
            && !display.chars().any(char::is_control),
    )?;
    static SEMVER: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$").unwrap()
    });
    require(SEMVER.is_match(&fields["appVersion"]) && fields["appVersion"].len() <= 256)?;
    redirect_uri(&fields["redirectUri"])
}
pub fn validate_account_response(value: &Value, status: &Value) -> Result<()> {
    exact(
        value,
        &[
            "schemaVersion",
            "hostId",
            "workspaceId",
            "accountId",
            "email",
            "subscription",
        ],
    )?;
    require(
        value["schemaVersion"].as_u64() == Some(1)
            && status["paired"] == true
            && value["hostId"] == status["hostId"]
            && value["workspaceId"] == status["workspaceId"],
    )?;
    opaque(&value["hostId"])?;
    opaque(&value["workspaceId"])?;
    let account = value["accountId"]
        .as_str()
        .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
    let id = uuid::Uuid::parse_str(account).map_err(|_| error("DESKTOP_PROTOCOL_INVALID"))?;
    require(
        id.to_string() == account
            && id.get_variant() == uuid::Variant::RFC4122
            && (1..=8).contains(&id.get_version_num()),
    )?;
    if !value["email"].is_null() {
        let email = value["email"]
            .as_str()
            .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
        require(
            !email.is_empty()
                && email.encode_utf16().count() <= 254
                && email.trim() == email
                && !email.chars().any(|c| c.is_whitespace() || c.is_control())
                && email.split('@').count() == 2
                && email.split('@').all(|s| !s.is_empty()),
        )?;
    }
    let sub = &value["subscription"];
    exact(
        sub,
        &["status", "memoryAccess", "periodEndsAt", "checkedAt"],
    )?;
    require(
        matches!(
            sub["status"].as_str(),
            Some("inactive" | "trialing" | "active" | "past_due" | "paused" | "canceled")
        ) && sub["memoryAccess"].is_boolean(),
    )?;
    canonical_time(&sub["checkedAt"])?;
    if !sub["periodEndsAt"].is_null() {
        canonical_time(&sub["periodEndsAt"])?;
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum Callback {
    Code(String),
    Denied,
}
pub fn parse_callback(bytes: &[u8], port: u16, state: &str) -> Result<Callback> {
    require(bytes.len() <= 4096 && hex64(state) && port != 0)?;
    let text = std::str::from_utf8(bytes).map_err(|_| error("DESKTOP_PROTOCOL_INVALID"))?;
    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
    require(body.is_empty())?;
    let mut lines = head.split("\r\n");
    let request: Vec<_> = lines.next().unwrap_or("").split(' ').collect();
    require(
        request.len() == 3
            && request[0] == "GET"
            && request[2] == "HTTP/1.1"
            && request[1].starts_with("/auth/callback?"),
    )?;
    let mut host = None;
    let mut length = None;
    for line in lines {
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
        require(
            !key.is_empty()
                && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !value.chars().any(|c| c.is_control() && c != '\t'),
        )?;
        match key.to_ascii_lowercase().as_str() {
            "host" => {
                require(host.is_none())?;
                host = Some(value.trim());
            }
            "content-length" => {
                require(length.is_none())?;
                length = Some(value.trim());
            }
            "transfer-encoding" => return Err(error("DESKTOP_PROTOCOL_INVALID")),
            _ => {}
        }
    }
    require(host == Some(format!("127.0.0.1:{port}").as_str()) && length.is_none_or(|s| s == "0"))?;
    let url = Url::parse(&format!("http://127.0.0.1:{port}{}", request[1]))
        .map_err(|_| error("DESKTOP_PROTOCOL_INVALID"))?;
    require(url.path() == "/auth/callback" && url.fragment().is_none())?;
    let mut fields = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        require(fields.insert(key.to_string(), value.to_string()).is_none())?;
    }
    require(fields.len() == 2 && fields.get("state").is_some_and(|s| s == state))?;
    if fields.get("error").is_some_and(|s| s == "access_denied") {
        return Ok(Callback::Denied);
    }
    let code = fields
        .get("code")
        .filter(|s| base64url43(s))
        .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
    Ok(Callback::Code(code.clone()))
}

enum State {
    Waiting,
    Code(String),
    Exchanging,
    Connected,
    Denied,
    Expired,
    Cancelled,
    Failed(String),
}
struct Session {
    state: State,
    verifier: Option<String>,
    waiting: Value,
    expires: tokio::time::Instant,
}
impl Session {
    fn terminal(&mut self, state: State) {
        self.state = state;
        self.verifier = None;
        self.waiting = Value::Null;
    }
    fn expire(&mut self) {
        if tokio::time::Instant::now() >= self.expires
            && matches!(
                self.state,
                State::Waiting | State::Code(_) | State::Exchanging
            )
        {
            self.terminal(State::Expired);
        }
    }
}
pub(crate) struct Exchange {
    pub generation: String,
    pub origin: String,
    pub installation: String,
    pub bound: Value,
    pub body: Value,
}
pub(crate) struct Login {
    command: String,
    input: Value,
    generation: String,
    origin: String,
    installation: String,
    bound: Value,
    state: Arc<Mutex<Session>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Login {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Login {
    pub(crate) fn matches(&self, command: &str, input: &Value) -> Result<bool> {
        if self.command != command {
            return Ok(false);
        }
        require(self.input == *input)?;
        Ok(true)
    }
    pub(crate) async fn begin(
        command: String,
        input: Value,
        installation: String,
        bound: Value,
    ) -> Result<Self> {
        let origin = validate_cloud_origin(
            input["cloudUrl"]
                .as_str()
                .ok_or_else(|| error("DESKTOP_ORIGIN_INVALID"))?,
        )?;
        let display = input["displayName"]
            .as_str()
            .ok_or_else(|| error("DESKTOP_INVALID_DISPLAY_NAME"))?;
        if display.is_empty()
            || display.trim() != display
            || display.encode_utf16().count() > 80
            || display.chars().any(char::is_control)
        {
            return Err(error("DESKTOP_INVALID_DISPLAY_NAME"));
        }
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| error("DESKTOP_CALLBACK_UNAVAILABLE"))?;
        let port = listener
            .local_addr()
            .map_err(|_| error("DESKTOP_CALLBACK_UNAVAILABLE"))?
            .port();
        let verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let nonce = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let mut authorization = Url::parse(&format!("{origin}/desktop/authorize"))
            .map_err(|_| error("DESKTOP_ORIGIN_INVALID"))?;
        let os = if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };
        authorization.query_pairs_mut().extend_pairs([
            ("schemaVersion", "1"),
            ("installationId", installation.as_str()),
            ("displayName", display),
            ("appVersion", env!("CARGO_PKG_VERSION")),
            ("protocolVersion", "2"),
            ("osFamily", os),
            (
                "redirectUri",
                &format!("http://127.0.0.1:{port}/auth/callback"),
            ),
            ("state", &nonce),
            ("codeChallenge", &challenge(&verifier)?),
        ]);
        let expires = (Utc::now() + chrono::Duration::minutes(10))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let comparison = nonce[..8].to_ascii_uppercase();
        let waiting = json!({"schemaVersion":1,"status":"waiting","authorizationUrl":authorization.as_str(),"expiresAt":expires,"confirmationCode":format!("{}-{}",&comparison[..4],&comparison[4..])});
        validate_authorization_url(authorization.as_str(), &origin)?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(600);
        let state = Arc::new(Mutex::new(Session {
            state: State::Waiting,
            verifier: Some(verifier),
            waiting,
            expires: deadline,
        }));
        let observed = state.clone();
        let task = tokio::spawn(async move {
            for _ in 0..16 {
                let accepted = tokio::time::timeout_at(deadline, listener.accept()).await;
                let Ok(Ok((mut socket, _))) = accepted else {
                    if let Ok(mut s) = observed.lock() {
                        s.terminal(State::Expired);
                    }
                    return;
                };
                let read = async {
                    let mut bytes = Vec::new();
                    let mut chunk = [0u8; 1024];
                    loop {
                        let n = socket.read(&mut chunk).await.ok()?;
                        if n == 0 {
                            return None;
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                        if bytes.len() > 4096 {
                            return None;
                        }
                        if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                            return Some(bytes);
                        }
                    }
                };
                let result = tokio::time::timeout_at(
                    deadline.min(tokio::time::Instant::now() + Duration::from_secs(2)),
                    read,
                )
                .await
                .ok()
                .flatten()
                .and_then(|bytes| parse_callback(&bytes, port, &nonce).ok());
                let valid = result.is_some();
                let text = if valid {
                    "Patchfleet sign-in received. You may return to the application."
                } else {
                    "Invalid Patchfleet sign-in callback."
                };
                let html = format!(
                    "<!doctype html><meta charset=utf-8><title>Patchfleet</title><p>{text}</p>"
                );
                let response=format!("HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",if valid {"200 OK"}else{"400 Bad Request"},html.len());
                let _ = tokio::time::timeout(
                    Duration::from_millis(500),
                    socket.write_all(response.as_bytes()),
                )
                .await;
                drop(socket);
                if let Some(result) = result {
                    let denied = matches!(result, Callback::Denied);
                    if let Ok(mut s) = observed.lock() {
                        match result {
                            Callback::Code(code) => s.state = State::Code(code),
                            Callback::Denied => s.terminal(State::Denied),
                        };
                        s.expire();
                    }
                    drop(listener);
                    if !denied {
                        tokio::time::sleep_until(deadline).await;
                        if let Ok(mut s) = observed.lock() {
                            if matches!(
                                s.state,
                                State::Code(_) | State::Exchanging | State::Waiting
                            ) {
                                s.terminal(State::Expired);
                            }
                        }
                    }
                    return;
                }
            }
            if let Ok(mut s) = observed.lock() {
                s.terminal(State::Failed("DESKTOP_CALLBACK_REJECTED".into()));
            }
        });
        Ok(Self {
            command,
            input,
            generation: uuid::Uuid::new_v4().to_string(),
            origin,
            installation,
            bound,
            state,
            task,
        })
    }
    pub(crate) fn status(&self) -> Result<Value> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("DESKTOP_STATE_FAILED"))?;
        state.expire();
        match &state.state {
            State::Waiting | State::Code(_) | State::Exchanging => Ok(state.waiting.clone()),
            State::Failed(code) => Err(error(code)),
            state => Ok(
                json!({"schemaVersion":1,"status":match state{State::Connected=>"connected",State::Denied=>"denied",State::Expired=>"expired",_=>"cancelled"}}),
            ),
        }
    }
    pub(crate) fn exchange(&mut self) -> Result<Option<Exchange>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("DESKTOP_STATE_FAILED"))?;
        state.expire();
        if !matches!(state.state, State::Code(_)) {
            return Ok(None);
        }
        let State::Code(code) = std::mem::replace(&mut state.state, State::Exchanging) else {
            unreachable!()
        };
        let verifier = state
            .verifier
            .take()
            .ok_or_else(|| error("DESKTOP_STATE_FAILED"))?;
        Ok(Some(Exchange {
            generation: self.generation.clone(),
            origin: self.origin.clone(),
            installation: self.installation.clone(),
            bound: self.bound.clone(),
            body: json!({"schemaVersion":1,"code":code,"verifier":verifier,"installationId":self.installation}),
        }))
    }
    pub(crate) fn is_exchange(&self, generation: &str) -> bool {
        self.generation == generation
            && self.state.lock().is_ok_and(|mut s| {
                s.expire();
                matches!(s.state, State::Exchanging)
            })
    }
    pub(crate) fn finish(&mut self, result: std::result::Result<(), String>) {
        self.task.abort();
        if let Ok(mut state) = self.state.lock() {
            state.terminal(match result {
                Ok(()) => State::Connected,
                Err(code) => State::Failed(code),
            });
        }
    }
    pub(crate) fn cancel(&mut self) {
        self.task.abort();
        if let Ok(mut state) = self.state.lock() {
            state.terminal(State::Cancelled);
        }
    }
}
pub(crate) async fn response(request: RequestBuilder) -> Result<Value> {
    let mut response = request
        .send()
        .await
        .map_err(|_| error("DESKTOP_UNAVAILABLE"))?;
    let content = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    require(
        (content.eq_ignore_ascii_case("application/json")
            || content.eq_ignore_ascii_case("application/json; charset=utf-8"))
            && response
                .headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|s| {
                    s.split(',')
                        .any(|part| part.trim().eq_ignore_ascii_case("no-store"))
                }),
    )?;
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| error("DESKTOP_UNAVAILABLE"))?
    {
        require(bytes.len() + chunk.len() <= 8192)?;
        bytes.extend_from_slice(&chunk);
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| error("DESKTOP_PROTOCOL_INVALID"))?;
    if !status.is_success() {
        exact(&value, &["error"])?;
        let code = value["error"]
            .as_str()
            .filter(|s| {
                s.starts_with("DESKTOP_")
                    && s.len() > 8
                    && s.len() <= 64
                    && s.bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            })
            .ok_or_else(|| error("DESKTOP_PROTOCOL_INVALID"))?;
        return Err(error(code));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expiry_drops_waiting_url_code_and_verifier_without_a_polling_owner() {
        let mut session = Session {
            state: State::Code("CODE_CANARY".into()),
            verifier: Some("VERIFIER_CANARY".into()),
            waiting: json!({"authorizationUrl":"STATE_CANARY"}),
            expires: tokio::time::Instant::now(),
        };
        session.expire();
        assert!(matches!(session.state, State::Expired));
        assert!(session.verifier.is_none());
        assert!(session.waiting.is_null());
    }
}
