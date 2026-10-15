use patchfleet_core::desktop_login::{self as login, Callback};
use serde_json::{json, Value};

fn authorization() -> String {
    let mut url = reqwest::Url::parse("https://cloud.example/desktop/authorize").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("schemaVersion", "1"),
        ("installationId", "install:fixture"),
        ("displayName", "Synthetic Mac"),
        ("appVersion", "0.8.0"),
        ("protocolVersion", "2"),
        ("osFamily", "darwin"),
        ("redirectUri", "http://127.0.0.1:12345/auth/callback"),
        ("state", &"a".repeat(64)),
        ("codeChallenge", &"A".repeat(43)),
    ]);
    url.to_string()
}
fn account() -> (Value, Value) {
    (
        json!({"schemaVersion":1,"hostId":"host:fixture","workspaceId":"workspace:fixture","accountId":"10000000-0000-4000-8000-000000000001","email":"owner@example.test","subscription":{"status":"active","memoryAccess":true,"periodEndsAt":null,"checkedAt":"2026-10-15T12:00:00.000Z"}}),
        json!({"paired":true,"hostId":"host:fixture","workspaceId":"workspace:fixture"}),
    )
}
fn callback(target: &str, headers: &str) -> Vec<u8> {
    format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:12345\r\n{headers}\r\n").into_bytes()
}

#[test]
fn pkce_is_canonical_s256_without_url_secret_or_padding() {
    assert_eq!(
        login::challenge(&"a".repeat(64)).unwrap(),
        "_-BU_nrgy23GXDr5th1SCfQ5hR20PQulmXM33xVGaOs"
    );
    for bad in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
        assert!(login::challenge(&bad).is_err());
    }
}
#[test]
fn browser_target_is_exact_validated_origin_authorize_and_loopback_callback() {
    login::validate_authorization_url(&authorization(), "https://cloud.example").unwrap();
    for origin in [
        "http://cloud.example",
        "http://localhost:1234",
        "https://user:secret@cloud.example",
        "https://cloud.example/path",
        "https://cloud.example?query=1",
        "https://cloud.example/#fragment",
    ] {
        assert!(login::validate_cloud_origin(origin).is_err(), "{origin}");
    }
    for target in [
        authorization().replace("cloud.example", "other.example"),
        authorization().replace("/desktop/authorize", "/authorize"),
        format!("{}&state={}", authorization(), "a".repeat(64)),
        format!("{}&next=https://other.example", authorization()),
        authorization().replace("127.0.0.1%3A12345", "localhost%3A12345"),
        authorization().replace("protocolVersion=2", "protocolVersion=1"),
    ] {
        assert!(
            login::validate_authorization_url(&target, "https://cloud.example").is_err(),
            "{target}"
        );
    }
}
#[test]
fn callback_accepts_only_exact_state_code_or_denial_get_and_host() {
    let target = format!(
        "/auth/callback?state={}&code={}",
        "a".repeat(64),
        "A".repeat(43)
    );
    assert_eq!(
        login::parse_callback(&callback(&target, ""), 12345, &"a".repeat(64)).unwrap(),
        Callback::Code("A".repeat(43))
    );
    let denied = format!(
        "/auth/callback?state={}&error=access_denied",
        "a".repeat(64)
    );
    assert_eq!(
        login::parse_callback(&callback(&denied, ""), 12345, &"a".repeat(64)).unwrap(),
        Callback::Denied
    );
    for bytes in [
        callback(&target.replace("state=a", "state=b"), ""),
        callback(&format!("{target}&code={}", "B".repeat(43)), ""),
        callback(&format!("{target}&instruction=run"), ""),
        callback(&target, "Host: evil.example\r\n"),
        callback(&target, "Transfer-Encoding: chunked\r\n"),
        callback(&target, "Content-Length: 1\r\n"),
        callback(
            &target.replace("/auth/callback?", "/auth/callback/other?"),
            "",
        ),
        callback(&target, "X-Large: ")
            .into_iter()
            .chain(vec![b'x'; 4096])
            .collect(),
        format!("{}BODY", String::from_utf8(callback(&target, "")).unwrap()).into_bytes(),
        String::from_utf8(callback(&target, ""))
            .unwrap()
            .replace("GET ", "POST ")
            .into_bytes(),
        String::from_utf8(callback(&target, ""))
            .unwrap()
            .replace("127.0.0.1:12345", "localhost:12345")
            .into_bytes(),
    ] {
        assert!(login::parse_callback(&bytes, 12345, &"a".repeat(64)).is_err());
    }
}
#[test]
fn account_is_scoped_verified_identity_with_exact_fresh_subscription_dto() {
    let (good, status) = account();
    login::validate_account_response(&good, &status).unwrap();
    for name in [
        "inactive", "trialing", "active", "past_due", "paused", "canceled",
    ] {
        let mut value = good.clone();
        value["subscription"]["status"] = json!(name);
        value["email"] = Value::Null;
        login::validate_account_response(&value, &status).unwrap();
    }
    for (path, value) in [
        ("/hostId", json!("host:other")),
        ("/workspaceId", json!("workspace:other")),
        ("/schemaVersion", json!(2)),
        ("/accountId", json!("OWNER")),
        ("/email", json!("@example.test")),
        ("/email", json!("owner@bad domain")),
        ("/email", json!("a".repeat(255))),
        ("/subscription/status", json!("unknown")),
        ("/subscription/memoryAccess", json!("true")),
        (
            "/subscription/checkedAt",
            json!("2026-10-15T13:00:00.000+01:00"),
        ),
        ("/subscription/periodEndsAt", json!("tomorrow")),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(
            login::validate_account_response(&bad, &status).is_err(),
            "{path}"
        );
    }
    for path in ["", "/subscription"] {
        let mut bad = good.clone();
        bad.pointer_mut(path).unwrap()["token"] = json!("never permitted");
        assert!(login::validate_account_response(&bad, &status).is_err());
    }
    assert!(login::validate_account_response(&good, &json!({"paired":false})).is_err());
}
