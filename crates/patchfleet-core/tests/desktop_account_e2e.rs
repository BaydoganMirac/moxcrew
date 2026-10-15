//! Invoked only by the private disposable HTTP/PostgreSQL fixture driver.
use patchfleet_core::Host;
use serde_json::{json, Value};

fn command(mut value: Value) -> Value {
    let now = chrono::Utc::now();
    value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    value["createdAt"] = json!(now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    value["expiresAt"] =
        json!((now + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    value
}
fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing fixture variable {name}"))
}
async fn control(base: &str, token: &str, value: Value) -> Value {
    let response = reqwest::Client::new()
        .post(format!("{base}/_fixture/control"))
        .header("x-fixture-token", token)
        .json(&value)
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "fixture control status {}: {}",
        response.status(),
        response.text().await.unwrap()
    );
    response.json().await.unwrap()
}
async fn begin(host: &Host, base: &str) -> Value {
    let request = command(json!({"cloudUrl":base,"displayName":"Desktop account fixture"}));
    let waiting = host
        .action("cloud_login_begin", request.clone())
        .await
        .unwrap();
    assert_eq!(waiting["status"], "waiting");
    assert_eq!(
        waiting,
        host.action("cloud_login_begin", request).await.unwrap()
    );
    waiting
}
async fn complete(host: &Host, base: &str, token: &str, waiting: &Value, action: &str) -> Value {
    let url = waiting["authorizationUrl"].as_str().unwrap();
    let response = reqwest::Client::new().get(url).send().await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(response.text().await.unwrap().contains("Allow connection"));
    let result = control(base, token, json!({"action":action,"authorizationUrl":url})).await;
    let callback = result["callback"].as_str().unwrap();
    let mut wrong = reqwest::Url::parse(callback).unwrap();
    let fields: Vec<(String, String)> = wrong
        .query_pairs()
        .map(|(key, value)| {
            (
                key.to_string(),
                if key == "state" {
                    "0".repeat(64)
                } else {
                    value.to_string()
                },
            )
        })
        .collect();
    wrong.query_pairs_mut().clear().extend_pairs(fields);
    assert_eq!(reqwest::get(wrong).await.unwrap().status(), 400);
    assert_eq!(
        host.action("cloud_login_poll", json!({})).await.unwrap()["status"],
        "waiting"
    );
    let response = reqwest::get(callback).await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    for _ in 0..100 {
        let result = host.action("cloud_login_poll", json!({})).await.unwrap();
        if result["status"] != "waiting" {
            return result;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("callback did not finish");
}
fn assert_absent(directory: &std::path::Path, canary: &str) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            assert_absent(&path, canary);
        } else {
            assert!(!String::from_utf8_lossy(&std::fs::read(path).unwrap()).contains(canary));
        }
    }
}

#[tokio::test]
#[ignore = "requires the isolated private desktop-auth-rust-smoke PostgreSQL/HTTP fixture"]
async fn native_browser_approval_account_restart_and_sign_out_use_real_postgres() {
    let base = env("PATCHFLEET_DESKTOP_E2E_URL");
    let origin = reqwest::Url::parse(&base).unwrap();
    assert_eq!(origin.scheme(), "http");
    assert_eq!(origin.host_str(), Some("127.0.0.1"));
    assert_eq!(origin.path(), "/");
    let token = env("PATCHFLEET_DESKTOP_E2E_TOKEN");
    let directory = tempfile::tempdir().unwrap();
    let mut host = Host::open(directory.path().to_owned()).unwrap();
    let waiting = begin(&host, &base).await;
    let authorization = reqwest::Url::parse(waiting["authorizationUrl"].as_str().unwrap()).unwrap();
    let state = authorization
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .to_string();
    assert!(!authorization
        .query_pairs()
        .any(|(key, _)| key == "verifier"));
    let project = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        host.action(
            "project_create",
            command(json!({"name":"Local survives account sign-out"})),
        ),
    )
    .await
    .unwrap()
    .unwrap()["projectId"]
        .clone();
    assert_eq!(
        complete(&host, &base, &token, &waiting, "allow").await["status"],
        "connected"
    );
    let account = host.action("cloud_account_read", json!({})).await.unwrap();
    assert_eq!(account["email"], "desktop-fixture@example.test");
    assert_eq!(account["subscription"]["memoryAccess"], false);
    assert_eq!(account["subscription"]["status"], "inactive");
    assert!(!host
        .snapshot()
        .unwrap()
        .to_string()
        .contains("desktop-fixture@example.test"));
    assert!(!host.snapshot().unwrap().to_string().contains(&state));
    assert_absent(directory.path(), &state);
    assert_absent(directory.path(), "desktop-fixture@example.test");
    let connection: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("cloud.json")).unwrap())
            .unwrap();
    let credential = connection["connection"]["credential"]
        .as_str()
        .unwrap()
        .to_string();
    let host_id = account["hostId"].as_str().unwrap().to_string();
    assert!(!host.snapshot().unwrap().to_string().contains(&credential));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(directory.path().join("cloud.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    host.shutdown().await;
    drop(host);
    host = Host::open(directory.path().to_owned()).unwrap();
    assert_eq!(
        host.action("cloud_account_read", json!({})).await.unwrap()["hostId"],
        host_id
    );
    control(&base, &token, json!({"action":"subscription_active"})).await;
    let active = host.action("cloud_account_read", json!({})).await.unwrap();
    assert_eq!(active["subscription"]["memoryAccess"], true);
    assert_eq!(active["subscription"]["status"], "active");
    let snapshot = host.snapshot().unwrap();
    assert!(snapshot["sharedMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["state"] == "unlinked" && p["sharedProjectId"].is_null()));
    let signed_out = host.action("cloud_logout", json!({})).await.unwrap();
    assert_eq!(
        signed_out,
        json!({"schemaVersion":1,"localSignedOut":true,"remoteRevoked":true})
    );
    assert_absent(directory.path(), &credential);
    assert!(host
        .snapshot()
        .unwrap()
        .to_string()
        .contains(project.as_str().unwrap()));
    let response = reqwest::Client::new()
        .get(format!("{base}/api/desktop/v1/hosts/{host_id}/account"))
        .bearer_auth(&credential)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    let waiting = begin(&host, &base).await;
    assert_eq!(
        complete(&host, &base, &token, &waiting, "deny").await["status"],
        "denied"
    );
    assert!(host.action("cloud_account_read", json!({})).await.is_err());
    let waiting = begin(&host, &base).await;
    let callback = reqwest::Url::parse(waiting["authorizationUrl"].as_str().unwrap())
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "redirectUri")
        .unwrap()
        .1
        .to_string();
    assert_eq!(
        host.action("cloud_login_cancel", json!({})).await.unwrap()["status"],
        "cancelled"
    );
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(reqwest::get(callback).await.is_err());
    host.shutdown().await;
}
