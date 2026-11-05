use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn notion() -> Value {
    json!({"parentPageId":"c8e418b2-552d-45e4-96b5-8c13d9c624ba","title":"Project brief","content":"One useful paragraph."})
}
fn calendar() -> Value {
    json!({"calendarId":"primary","summary":"Project review","description":"","start":"2026-10-20T10:00:00+03:00","end":"2026-10-20T11:00:00+03:00","timeZone":"Europe/Istanbul"})
}

#[test]
fn credentials_are_private_project_scoped_redacted_and_command_idempotent() {
    let data = tempfile::tempdir().unwrap();
    let initial = metadata(data.path(), "project:a").unwrap();
    assert_eq!(initial.as_array().unwrap().len(), 3);
    assert!(initial
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["connected"] == false));
    let first = configure(
        data.path(),
        "project:a",
        "notion",
        "Work account",
        "SECRET_TOKEN_A",
        "cmd:one",
    )
    .unwrap();
    assert_eq!(first["connected"], true);
    assert_eq!(first["connectionRevision"], 1);
    assert_eq!(first["authentication"], "unverified");
    assert!(!metadata(data.path(), "project:a")
        .unwrap()
        .to_string()
        .contains("SECRET_TOKEN"));
    assert!(metadata(data.path(), "project:b")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["connected"] == false));
    assert_eq!(
        configure(
            data.path(),
            "project:a",
            "notion",
            "Work account",
            "SECRET_TOKEN_A",
            "cmd:one"
        )
        .unwrap(),
        first
    );
    assert_eq!(
        configure(
            data.path(),
            "project:a",
            "notion",
            "Work account",
            "SECRET_TOKEN_B",
            "cmd:one"
        )
        .unwrap_err()
        .0,
        "PROJECT_CONNECTION_COMMAND_CONFLICT"
    );
    assert_eq!(
        remove(data.path(), "project:a", "notion", "cmd:two").unwrap()["connected"],
        false
    );
    assert_eq!(
        configure(
            data.path(),
            "project:a",
            "notion",
            "Work account",
            "SECRET_TOKEN_A",
            "cmd:one"
        )
        .unwrap(),
        first
    );
    assert_eq!(
        metadata(data.path(), "project:a").unwrap()[1]["connected"],
        false,
        "replayed set must not resurrect removed credentials"
    );
    assert_eq!(
        metadata(data.path(), "project:a").unwrap()[1]["connectionRevision"],
        2
    );
    configure(
        data.path(),
        "project:a",
        "google_drive",
        "Drive",
        "DRIVE_TOKEN",
        "cmd:drive",
    )
    .unwrap();
    assert_eq!(
        metadata(data.path(), "project:a").unwrap()[1]["connectionRevision"],
        2
    );
    let stored = std::fs::read_to_string(path(data.path(), "project:a").unwrap()).unwrap();
    assert!(!stored.contains("SECRET_TOKEN"));
    assert!(!stored.contains("cmd:one"));
    assert!(configure(
        data.path(),
        "../foreign",
        "notion",
        "Work",
        "TOKEN",
        "cmd:three"
    )
    .is_err());
    assert!(configure(
        data.path(),
        "project:a",
        "notion",
        "Work",
        "TOKEN\nheader",
        "cmd:three"
    )
    .is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file = path(data.path(), "project:a").unwrap();
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(file.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}

#[cfg(unix)]
#[test]
fn credentials_reject_symlinks_and_bounded_read_does_not_parse_large_files() {
    let data = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), data.path().join("app-connections")).unwrap();
    assert!(configure(
        data.path(),
        "project:a",
        "notion",
        "Work",
        "TOKEN",
        "cmd:one"
    )
    .is_err());
    std::fs::remove_file(data.path().join("app-connections")).unwrap();
    configure(
        data.path(),
        "project:a",
        "notion",
        "Work",
        "TOKEN",
        "cmd:one",
    )
    .unwrap();
    let file = path(data.path(), "project:a").unwrap();
    std::fs::remove_file(&file).unwrap();
    std::fs::write(elsewhere.path().join("secret"), b"{}").unwrap();
    std::os::unix::fs::symlink(elsewhere.path().join("secret"), &file).unwrap();
    assert!(metadata(data.path(), "project:a").is_err());
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, vec![b' '; CONFIG_LIMIT + 1]).unwrap();
    assert_eq!(
        metadata(data.path(), "project:a").unwrap_err().0,
        "PROJECT_CONNECTION_CONFIG_LIMIT"
    );
}

#[test]
fn previews_are_pure_exact_and_reject_foreign_operations_urls_and_hidden_fields() {
    for (app, op, args) in [
        ("google_drive", "drive_import", json!({"fileId":"file_123"})),
        ("notion", "notion_page", notion()),
        ("google_calendar", "calendar_event", calendar()),
    ] {
        assert_eq!(validate_prepare(app, op, &args).unwrap(), args);
        let mut extra = args.clone();
        extra["accessToken"] = json!("SECRET");
        assert!(validate_prepare(app, op, &extra).is_err());
        assert!(validate_prepare(app, "arbitrary", &args).is_err());
    }
    for id in ["https://127.0.0.1/", "../foreign", "file?alt=media", ""] {
        assert!(validate_prepare("google_drive", "drive_import", &json!({"fileId":id})).is_err());
    }
    let mut bad = calendar();
    bad["end"] = bad["start"].clone();
    assert!(validate_prepare("google_calendar", "calendar_event", &bad).is_err());
    let mut bad = calendar();
    bad["calendarId"] = json!("../other");
    assert!(validate_prepare("google_calendar", "calendar_event", &bad).is_err());
    let mut bad = notion();
    bad["parentPageId"] = json!("../other");
    assert!(validate_prepare("notion", "notion_page", &bad).is_err());
    assert_eq!(filename("../../CON.txt"), "_.._CON.txt");
    assert_eq!(filename("CON.txt"), "drive-CON.txt.bin");
    assert_ne!(
        calendar_id("project:a", "action:one"),
        calendar_id("project:b", "action:one")
    );
}

struct Reply {
    status: &'static str,
    headers: String,
    body: Vec<u8>,
}
fn reply(value: Value) -> Reply {
    Reply {
        status: "200 OK",
        headers: String::new(),
        body: serde_json::to_vec(&value).unwrap(),
    }
}
async fn server(replies: Vec<Reply>) -> (Transport, tokio::task::JoinHandle<Vec<(String, Value)>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut requests = vec![];
        for reply in replies {
            let (mut socket, _) = tokio::time::timeout(Duration::from_secs(3), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut bytes = vec![];
            let end = loop {
                let mut b = [0; 1024];
                let n = socket.read(&mut b).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&b[..n]);
                if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                    break i + 4;
                }
                assert!(bytes.len() < 16384);
            };
            let headers = String::from_utf8(bytes[..end].to_vec()).unwrap();
            let length = headers
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_owned)
                })
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            assert!(length < 512 * 1024);
            while bytes.len() < end + length {
                let mut b = [0; 4096];
                let n = socket.read(&mut b).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&b[..n]);
            }
            let body = if length == 0 {
                Value::Null
            } else {
                serde_json::from_slice(&bytes[end..end + length]).unwrap()
            };
            requests.push((headers, body));
            let head = format!(
                "HTTP/1.1 {}\r\nConnection: close\r\nContent-Length: {}\r\n{}\r\n",
                reply.status,
                reply.body.len(),
                reply.headers
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(&reply.body).await.unwrap();
        }
        requests
    });
    (
        Transport {
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(DEADLINE)
                .build()
                .unwrap(),
            endpoint: Some(Url::parse(&format!("http://{address}/")).unwrap()),
        },
        task,
    )
}

#[tokio::test]
async fn loopback_operations_send_only_approved_shapes_and_return_typed_results() {
    let data = tempfile::tempdir().unwrap();
    for (app, _) in APPS {
        configure(
            data.path(),
            "project:a",
            app,
            "Test account",
            "FAKE_PRIVATE_TOKEN",
            &format!("cmd:{app}"),
        )
        .unwrap();
    }
    let (transport,task)=server(vec![reply(json!({"id":"file_123","name":"../Brief.pdf","mimeType":"application/pdf","size":"4","private":"SECRET_NATIVE"})),Reply{status:"200 OK",headers:String::new(),body:b"data".to_vec()}]).await;
    let output = execute_with(
        &transport,
        data.path(),
        "project:a",
        "action:one",
        "google_drive",
        "drive_import",
        &json!({"fileId":"file_123"}),
    )
    .await
    .unwrap();
    assert_eq!(output.files, vec![("_Brief.pdf".into(), b"data".to_vec())]);
    assert!(!output.value.to_string().contains("SECRET_NATIVE"));
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0]
        .0
        .starts_with("GET /drive/v3/files/file_123?fields="));
    assert!(requests[1].0.contains("?alt=media"));
    let page = "038b6f86-44ab-49e1-948b-6b57ceff1d9c";
    let (transport,task)=server(vec![reply(json!({"object":"page","id":page,"url":"https://evil.example/SECRET_NATIVE","private":"SECRET_NATIVE"}))]).await;
    let output = execute_with(
        &transport,
        data.path(),
        "project:a",
        "action:two",
        "notion",
        "notion_page",
        &notion(),
    )
    .await
    .unwrap();
    assert_eq!(
        output.value,
        json!({"providerId":"notion","operation":"notion_page","pageId":page})
    );
    let requests = task.await.unwrap();
    assert!(requests[0].0.starts_with("POST /v1/pages "));
    assert!(requests[0].0.contains("notion-version: 2026-03-11"));
    assert_eq!(
        requests[0].1["children"][0]["paragraph"]["rich_text"][0]["text"]["content"],
        "One useful paragraph."
    );
    let id = calendar_id("project:a", "action:three");
    let (transport, task) = server(vec![reply(
        json!({"id":id,"status":"confirmed","private":"SECRET_NATIVE"}),
    )])
    .await;
    let output = execute_with(
        &transport,
        data.path(),
        "project:a",
        "action:three",
        "google_calendar",
        "calendar_event",
        &calendar(),
    )
    .await
    .unwrap();
    assert_eq!(output.value["eventId"], id);
    assert!(!output.value.to_string().contains("SECRET_NATIVE"));
    let requests = task.await.unwrap();
    assert!(requests[0]
        .0
        .starts_with("POST /calendar/v3/calendars/primary/events "));
    assert_eq!(requests[0].1["id"], id);
    assert!(requests[0].1.get("attendees").is_none());
    assert_eq!(
        requests[0].1["start"],
        json!({"dateTime":"2026-10-20T10:00:00+03:00","timeZone":"Europe/Istanbul"})
    );
    assert!(requests[0]
        .0
        .to_ascii_lowercase()
        .contains("authorization: bearer fake_private_token"));
}

#[tokio::test]
async fn loopback_failures_are_bounded_redacted_never_redirected_or_retried() {
    for (reply, code) in [
        (
            Reply {
                status: "302 Found",
                headers: "Location: http://127.0.0.1:1/SECRET\r\n".into(),
                body: vec![],
            },
            "PROJECT_CONNECTION_REDIRECT_REFUSED",
        ),
        (
            Reply {
                status: "503 Unavailable",
                headers: String::new(),
                body: b"SECRET_NATIVE_ERROR".to_vec(),
            },
            "PROJECT_CONNECTION_OUTCOME_UNKNOWN",
        ),
        (
            Reply {
                status: "200 OK",
                headers: String::new(),
                body: b"SECRET_INVALID_JSON".to_vec(),
            },
            "PROJECT_CONNECTION_OUTCOME_UNKNOWN",
        ),
        (
            Reply {
                status: "401 Unauthorized",
                headers: String::new(),
                body: b"SECRET_NATIVE_ERROR".to_vec(),
            },
            "PROJECT_CONNECTION_AUTH_REQUIRED",
        ),
    ] {
        let (transport, task) = server(vec![reply]).await;
        let error = perform(
            &transport,
            "FAKE_TOKEN",
            "project:a",
            "action:one",
            "notion",
            &notion(),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.0, code);
        assert_eq!(task.await.unwrap().len(), 1);
    }
    for (meta, code) in [
        (
            json!({"id":"file","name":"Doc","mimeType":"application/vnd.google-apps.document"}),
            "PROJECT_CONNECTION_GOOGLE_EXPORT_REQUIRED",
        ),
        (
            json!({"id":"file","name":"large.bin","mimeType":"application/octet-stream","size":(FILE_LIMIT+1).to_string()}),
            "PROJECT_CONNECTION_OUTPUT_LIMIT",
        ),
        (
            json!({"id":"foreign","name":"a","mimeType":"text/plain","size":"1"}),
            "PROJECT_CONNECTION_INVALID_RESPONSE",
        ),
    ] {
        let (transport, task) = server(vec![reply(meta)]).await;
        let error = perform(
            &transport,
            "FAKE_TOKEN",
            "project:a",
            "action:one",
            "google_drive",
            &json!({"fileId":"file"}),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.0, code);
        assert_eq!(task.await.unwrap().len(), 1);
    }
    let (transport, task) = server(vec![Reply {
        status: "200 OK",
        headers: String::new(),
        body: vec![b'x'; JSON_LIMIT + 1],
    }])
    .await;
    assert_eq!(
        perform(
            &transport,
            "FAKE_TOKEN",
            "project:a",
            "action:one",
            "notion",
            &notion()
        )
        .await
        .err()
        .unwrap()
        .0,
        "PROJECT_CONNECTION_OUTCOME_UNKNOWN"
    );
    let _ = task.await; // Client deliberately closes the oversized response.
}

#[tokio::test]
async fn unavailable_credentials_fail_before_network_and_native_hosts_are_fixed() {
    let data = tempfile::tempdir().unwrap();
    assert_eq!(
        execute(
            data.path(),
            "project:a",
            "action:one",
            "notion",
            "notion_page",
            &notion()
        )
        .await
        .err()
        .unwrap()
        .0,
        "PROJECT_CONNECTION_AUTH_REQUIRED"
    );
    let transport = Transport::new().unwrap();
    assert_eq!(
        transport.url("notion", &["v1", "pages"]).as_str(),
        "https://api.notion.com/v1/pages"
    );
    assert_eq!(
        transport
            .url("google_drive", &["drive", "v3", "files", "file"])
            .host_str(),
        Some("www.googleapis.com")
    );
    assert_eq!(
        transport
            .url(
                "google_calendar",
                &["calendar", "v3", "calendars", "a@b.com", "events"]
            )
            .path(),
        "/calendar/v3/calendars/a@b.com/events"
    );
}

#[tokio::test]
async fn stalled_write_response_is_unknown_and_size_check_precedes_body_read() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        drop(socket);
    });
    let transport = Transport {
        client: Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(50))
            .build()
            .unwrap(),
        endpoint: Some(Url::parse(&format!("http://{address}/")).unwrap()),
    };
    let started = std::time::Instant::now();
    assert_eq!(
        perform(
            &transport,
            "FAKE_TOKEN",
            "project:a",
            "action:one",
            "notion",
            &notion()
        )
        .await
        .err()
        .unwrap()
        .0,
        "PROJECT_CONNECTION_OUTCOME_UNKNOWN"
    );
    assert!(started.elapsed() < Duration::from_secs(1));
    task.abort();
    let _ = task.await;
    let (transport, task) = server(vec![Reply {
        status: "200 OK",
        headers: String::new(),
        body: b"too large".to_vec(),
    }])
    .await;
    assert_eq!(
        transport
            .request(
                "google_drive",
                "FAKE_TOKEN",
                Method::GET,
                transport.url("google_drive", &["drive"]),
                None,
                4
            )
            .await
            .unwrap_err()
            .0,
        "PROJECT_CONNECTION_OUTPUT_LIMIT"
    );
    let _ = task.await;
    assert_eq!(DEADLINE, Duration::from_secs(10));
}
