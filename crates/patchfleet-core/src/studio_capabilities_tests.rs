use super::*;

fn command(mut p: Value) -> Value {
    p["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    let at = chrono::Utc::now();
    p["createdAt"] = json!(at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    p["expiresAt"] =
        json!((at + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    p
}
async fn fixture() -> (tempfile::TempDir, Arc<Host>, Value) {
    let tmp = tempfile::tempdir().unwrap();
    let host = Host::open(tmp.path().join("data")).unwrap();
    let project = host
        .action("project_create", command(json!({"name":"Capabilities"})))
        .await
        .unwrap()["projectId"]
        .clone();
    (tmp, host, project)
}
async fn output(host: &Host, project: &Value, name: &str, bytes: &[u8]) -> Value {
    let job=host.action("project_work_enqueue",command(json!({"projectId":project,"expectedProjectRevision":project_revision(host,project),"abilityId":"D1","title":"Synthetic source fixture","instruction":"Create a document"}))).await.unwrap();
    let (root, _) = host
        .creation_scope(&json!({"projectId":project,"workItemId":job["workItemId"]}))
        .unwrap();
    std::fs::create_dir_all(root.join("outputs")).unwrap();
    std::fs::write(root.join("outputs").join(name), bytes).unwrap();
    json!({"workItemId":job["workItemId"],"path":format!("outputs/{name}"),"expectedSha256":digest(bytes)})
}
fn project_revision(host: &Host, project: &Value) -> Value {
    array(&host.snapshot().unwrap()["projectMemory"], "projects")
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()["revision"]
        .clone()
}

#[tokio::test]
async fn atomic_batch_preserves_order_hashes_idempotency_and_recovery() {
    let (tmp, host, project) = fixture().await;
    let a = tmp.path().join("first.txt");
    let b = tmp.path().join("second.txt");
    std::fs::write(&a, b"first frozen text").unwrap();
    std::fs::write(&b, b"second frozen text").unwrap();
    let p = command(json!({"projectId":project,"expectedProjectRevision":1,"sourcePaths":[a,b]}));
    let receipt = host
        .action("project_resource_import_batch", p.clone())
        .await
        .unwrap();
    assert_eq!(receipt["reasonCode"], "PROJECT_RESOURCES_IMPORTED");
    assert_eq!(receipt["commandId"], p["commandId"]);
    assert_eq!(receipt["resources"][0]["name"], "first.txt");
    assert_eq!(receipt["resources"][1]["name"], "second.txt");
    std::fs::write(&a, b"changed original").unwrap();
    assert_eq!(
        host.action("project_resource_import_batch", p.clone())
            .await
            .unwrap(),
        receipt
    );
    for (index, expected) in [
        b"first frozen text".as_slice(),
        b"second frozen text".as_slice(),
    ]
    .iter()
    .enumerate()
    {
        let resource = &receipt["resources"][index];
        assert_eq!(
            host.studio_resource(
                project.as_str().unwrap(),
                text(resource, "resourceId").unwrap()
            )
            .unwrap()
            .1,
            *expected
        );
        let read=host.action("project_resource_read",command(json!({"projectId":project,"resourceId":resource["resourceId"],"expectedSha256":resource["sha256"]}))).await.unwrap();
        assert_eq!(
            read["preview"]["text"],
            std::str::from_utf8(expected).unwrap()
        );
    }
    let history = host.store.read_events().unwrap();
    let request = history
        .iter()
        .find(|e| {
            e["type"] == "owner.command.requested" && e["payload"]["commandId"] == p["commandId"]
        })
        .unwrap();
    let interrupted = history
        .iter()
        .filter(|e| {
            !(e["type"] == "owner.command.receipted" && e["payload"]["commandId"] == p["commandId"])
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        recovered(&interrupted, &request["payload"]).unwrap(),
        receipt
    );
    let mut forged = history.clone();
    let event = forged
        .iter_mut()
        .find(|e| e["type"] == "project_studio.resources_imported")
        .unwrap();
    event["payload"]["resources"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(super::super::project(&forged).is_err());
    host.shutdown().await;
    drop(host);
    let reopened = Host::open(tmp.path().join("data")).unwrap();
    assert_eq!(
        array(
            &super::super::project(&reopened.store.read_events().unwrap()).unwrap(),
            "resources"
        )
        .len(),
        2
    );
    reopened.shutdown().await;
}

#[tokio::test]
async fn incomplete_batch_never_publishes_partial_resources() {
    let (tmp, host, project) = fixture().await;
    let a = tmp.path().join("good.txt");
    let missing = tmp.path().join("missing.txt");
    std::fs::write(&a, b"good").unwrap();
    assert!(host
        .action(
            "project_resource_import_batch",
            command(
                json!({"projectId":project,"expectedProjectRevision":1,"sourcePaths":[a,missing]})
            )
        )
        .await
        .is_err());
    assert!(array(
        &super::super::project(&host.store.read_events().unwrap()).unwrap(),
        "resources"
    )
    .is_empty());
    let duplicate =
        command(json!({"projectId":project,"expectedProjectRevision":1,"sourcePaths":[a,a]}));
    assert!(host
        .action("project_resource_import_batch", duplicate)
        .await
        .is_err());
    assert!(array(
        &super::super::project(&host.store.read_events().unwrap()).unwrap(),
        "resources"
    )
    .is_empty());
    host.shutdown().await;
}

#[tokio::test]
async fn multi_variant_sources_require_current_hashes_and_same_project() {
    let (_tmp, host, project) = fixture().await;
    let first = output(&host, &project, "one.txt", b"Crew one").await;
    let second = output(&host, &project, "two.txt", b"Crew two").await;
    let receipt = host
        .action(
            "project_resources_from_artifacts",
            command(
                json!({"projectId":project,"expectedProjectRevision":project_revision(&host,&project),"files":[first,second]}),
            ),
        )
        .await
        .unwrap();
    assert_eq!(receipt["resources"][0]["sha256"], first["expectedSha256"]);
    assert_eq!(receipt["resources"][1]["sha256"], second["expectedSha256"]);
    let other = host
        .action("project_create", command(json!({"name":"Other"})))
        .await
        .unwrap()["projectId"]
        .clone();
    assert!(host
        .action(
            "project_resources_from_artifacts",
            command(json!({"projectId":other,"expectedProjectRevision":1,"files":[first]}))
        )
        .await
        .is_err());
    let mut changed = first.clone();
    changed["expectedSha256"] = json!("0".repeat(64));
    assert_eq!(
        host.action(
            "project_resources_from_artifacts",
            command(
                json!({"projectId":project,"expectedProjectRevision":project_revision(&host,&project),"files":[second,changed]})
            )
        )
        .await
        .unwrap_err()
        .0,
        "PROJECT_ARTIFACT_CHANGED"
    );
    assert_eq!(
        array(
            &super::super::project(&host.store.read_events().unwrap()).unwrap(),
            "resources"
        )
        .len(),
        2
    );
    host.shutdown().await;
}

#[tokio::test]
async fn canonical_image_selection_is_frozen_without_mutating_original() {
    use base64::Engine;
    let (_tmp, host, project) = fixture().await;
    let png=base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+yS5kAAAAASUVORK5CYII=").unwrap();
    let selected = output(&host, &project, "image.png", &png).await;
    for section in [
        "region:NaN,0.000000,1.000000,1.000000",
        "region:0.900000,0.000000,0.200000,1.000000",
        "region:0,0,1,1",
    ] {
        assert!(validate_selection(&host, "image.png", &png, section)
            .await
            .is_err());
    }
    let section = "region:0.100000,0.200000,0.300000,0.400000";
    let mut p = selected.clone();
    p["projectId"] = project.clone();
    p["expectedProjectRevision"] = project_revision(&host, &project);
    p["sectionId"] = json!(section);
    p["instruction"] = json!("Make only this region blue");
    p["providerId"] = json!("codex");
    let receipt = host
        .action("project_artifact_revise", command(p))
        .await
        .unwrap();
    assert_eq!(receipt["reasonCode"], "PROJECT_ARTIFACT_REVISION_QUEUED");
    let (root, work) = host
        .creation_scope(&json!({"projectId":project,"workItemId":receipt["workItemId"]}))
        .unwrap();
    assert_eq!(work["status"], "queued");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root.join("inputs/manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["sectionId"], section);
    assert_eq!(manifest["sha256"], selected["expectedSha256"]);
    let mut source = selected.clone();
    source["projectId"] = project.clone();
    assert_eq!(artifact_bytes(&host, &source).unwrap(), png);
    host.shutdown().await;
}

#[tokio::test]
async fn arbitrary_html_cannot_open_even_with_a_csp_looking_string() {
    let (_tmp, host, project) = fixture().await;
    let selected = output(
        &host,
        &project,
        "unsafe.html",
        b"<script>alert(1)</script>Content-Security-Policy default-src 'none'; connect-src 'none'",
    )
    .await;
    let mut p = selected;
    p["projectId"] = project;
    assert!(host
        .action("project_artifact_open", command(p))
        .await
        .is_err());
    host.shutdown().await;
}

#[tokio::test]
async fn canonical_project_image_setup_is_private_and_studio_remains_readable() {
    let (_tmp, host, project) = fixture().await;
    let initial = host
        .action("project_studio_read", command(json!({"projectId":project})))
        .await
        .unwrap();
    assert_eq!(initial["imageEngine"]["configured"], false);
    let url = "http://127.0.0.1:65534";
    let configured = host
        .action(
            "project_image_engine_set",
            command(json!({"projectId":project,"url":url,"checkpoint":"fixture.safetensors"})),
        )
        .await
        .unwrap();
    assert_eq!(configured["imageEngine"]["configured"], true);
    let page = host
        .action("project_studio_read", command(json!({"projectId":project})))
        .await
        .unwrap();
    assert_eq!(page["imageEngine"]["configured"], true);
    assert_eq!(page["imageEngine"]["available"], false);
    assert!(!serde_json::to_string(&host.store.read_events().unwrap())
        .unwrap()
        .contains(url));
    assert!(!serde_json::to_string(&host.snapshot().unwrap())
        .unwrap()
        .contains(url));
    host.action(
        "project_image_engine_remove",
        command(json!({"projectId":project})),
    )
    .await
    .unwrap();
    assert_eq!(
        host.action("project_studio_read", command(json!({"projectId":project})))
            .await
            .unwrap()["imageEngine"]["configured"],
        false
    );
    host.shutdown().await;
}
