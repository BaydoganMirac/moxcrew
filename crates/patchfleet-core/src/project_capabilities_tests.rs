use super::*;
use crate::project_crews::tests::{
    command, fixture as provider_fixture, lifecycle, mission, prepare,
};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

type UrlFixtures = BTreeMap<(PathBuf, String), (String, Vec<u8>)>;
fn url_fixtures() -> &'static Mutex<UrlFixtures> {
    static FIXTURES: OnceLock<Mutex<UrlFixtures>> = OnceLock::new();
    FIXTURES.get_or_init(Mutex::default)
}
pub(super) fn url_sample(host: &Host, url: &str) -> Option<(String, Vec<u8>)> {
    url_fixtures()
        .lock()
        .unwrap()
        .get(&(host.creation_data_dir().to_owned(), url.to_owned()))
        .cloned()
}
fn url_bytes(host: &Host, url: &str, bytes: &[u8]) {
    url_fixtures().lock().unwrap().insert(
        (host.creation_data_dir().to_owned(), url.into()),
        (url.into(), bytes.to_vec()),
    );
}
fn allocation_failures() -> &'static Mutex<HashSet<PathBuf>> {
    static FAILURES: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    FAILURES.get_or_init(Mutex::default)
}
pub(super) fn take_allocation_failure(host: &Host) -> bool {
    allocation_failures()
        .lock()
        .unwrap()
        .remove(host.creation_data_dir())
}
async fn local() -> (tempfile::TempDir, Arc<Host>, Value) {
    let tmp = tempfile::tempdir().unwrap();
    let host = Host::open(tmp.path().join("data")).unwrap();
    let project = host
        .action("project_create", command(json!({"name":"Capabilities"})))
        .await
        .unwrap()["projectId"]
        .clone();
    (tmp, host, project)
}
#[cfg(unix)]
async fn resource(host: &Host, tmp: &Path, project: &Value, source: &str) -> Value {
    let path = tmp.join(format!("source-{}.txt", uuid::Uuid::new_v4()));
    fs::write(&path, source).unwrap();
    let revision = host.studio_project(&json!({"projectId":project})).unwrap()["revision"].clone();
    let result = host
        .action(
            "project_resource_import",
            command(
                json!({"projectId":project,"expectedProjectRevision":revision,"sourcePath":path}),
            ),
        )
        .await
        .unwrap();
    json!({"resourceId":result["resource"]["resourceId"],"expectedSha256":result["resource"]["sha256"]})
}
fn brand(project: &Value, revision: u64, refs: Value) -> Value {
    command(
        json!({"projectId":project,"expectedProfileRevision":revision,"logoResources":refs,"colours":["#123aEF"],"tone":"Concise and friendly","instructions":"Use clear accessible headings. PRIVATE_BRAND_CANARY"}),
    )
}
fn proposed(project: &Value, source: &Value) -> Value {
    command(
        json!({"projectId":project,"name":"A taught review process","steps":["Inspect the actual frozen input.","Check each claimed result against its source."],"expectedOutputs":["An evidence-backed report"],"sources":[source]}),
    )
}
fn accepted(project: &Value, id: &Value, revision: u64) -> Value {
    command(
        json!({"projectId":project,"skillId":id,"expectedSkillRevision":revision,"name":"Owner-reviewed process","steps":["Follow the owner-reviewed checked process. PRIVATE_SKILL_CANARY"],"expectedOutputs":["A checked report"]}),
    )
}

#[test]
fn standing_context_bounds_complete_serialized_unicode_libraries_and_profiles() {
    let p = json!(format!("project:{}", uuid::Uuid::new_v4()));
    let at = json!(now());
    let refs: Vec<_> = (0..8)
        .map(|_| json!({"resourceId":format!("resource:{}", uuid::Uuid::new_v4()),"expectedSha256":"a".repeat(64)}))
        .collect();
    let mut raw_profile = brand(&p, 0, json!(refs));
    raw_profile["tone"] = json!("調".repeat(500));
    raw_profile["instructions"] = json!("整".repeat(4000));
    raw_profile["colours"] = json!(vec!["#123abc"; 8]);
    let maximal_profile = profile(&raw_profile, None, &at).unwrap();
    let raw_skill = command(
        json!({"projectId":p,"name":"技".repeat(80),"steps":["手".repeat(1000)],"expectedOutputs":["出".repeat(240)],"sources":[refs[0]]}),
    );
    let proposed = skill(&raw_skill, None, "project_skill_propose", &at).unwrap();
    let mut accept = raw_skill.clone();
    accept.as_object_mut().unwrap().remove("sources");
    accept["skillId"] = proposed["skillId"].clone();
    accept["expectedSkillRevision"] = json!(1);
    let template = skill(&accept, Some(&proposed), "project_skill_accept", &at).unwrap();
    let mut skills: Vec<_> = (0..197)
        .map(|_| {
            let mut record = template.clone();
            record["skillId"] = json!(format!("skill:{}", uuid::Uuid::new_v4()));
            record
        })
        .collect();
    for (status, name) in [
        ("proposed", "PROPOSED_CANARY"),
        ("revoked", "REVOKED_CANARY"),
    ] {
        let mut record = template.clone();
        record["status"] = json!(status);
        record["name"] = json!(name);
        skills.push(record);
    }
    let mut foreign = template.clone();
    foreign["projectId"] = json!(format!("project:{}", uuid::Uuid::new_v4()));
    foreign["name"] = json!("FOREIGN_CANARY");
    skills.push(foreign);
    let mut state = json!({"profiles":[maximal_profile],"skills":skills});
    let packed = pack_standing_context(&state, p.as_str().unwrap()).unwrap();
    let bytes = serde_json::to_vec(&packed).unwrap();
    assert!(bytes.len() <= 24_000, "{}", bytes.len());
    assert_eq!(packed["profile"], maximal_profile);
    assert_eq!(packed["profilePartial"], false);
    assert_eq!(packed["acceptedSkillTotal"], 197);
    assert_eq!(packed["acceptedSkillManifestPartial"], true);
    assert_eq!(packed["partial"], true);
    assert!(!array(&packed, "acceptedSkills").is_empty());
    assert!(array(&packed, "acceptedSkillManifest").len() < 197);
    let text = String::from_utf8(bytes).unwrap();
    for canary in ["PROPOSED_CANARY", "REVOKED_CANARY", "FOREIGN_CANARY"] {
        assert!(!text.contains(canary));
    }

    // JSON escaping can exceed the cap even without a skill library.
    raw_profile["tone"] = json!("\u{1}".repeat(500));
    raw_profile["instructions"] = json!("\u{1}".repeat(4000));
    state["profiles"] = json!([profile(&raw_profile, None, &at).unwrap()]);
    assert!(serde_json::to_vec(&state["profiles"][0]).unwrap().len() > 24_000);
    let packed = pack_standing_context(&state, p.as_str().unwrap()).unwrap();
    assert!(serde_json::to_vec(&packed).unwrap().len() <= 24_000);
    assert_eq!(packed["profilePartial"], true);
    assert_eq!(packed["partial"], true);
    assert!(packed["profile"].get("instructions").is_none());
    assert!(packed["profile"].get("tone").is_none());
    assert_eq!(packed["profile"]["logoResources"], json!(refs));

    // Existing small-library fields still contain exact full records and the full index.
    let small_profile = profile(&brand(&p, 0, json!([])), None, &at).unwrap();
    state["profiles"] = json!([small_profile]);
    state["skills"] = json!([template]);
    let packed = pack_standing_context(&state, p.as_str().unwrap()).unwrap();
    assert!(serde_json::to_vec(&packed).unwrap().len() <= 24_000);
    assert_eq!(packed["profile"], small_profile);
    assert_eq!(packed["acceptedSkills"], json!([template]));
    assert_eq!(
        packed["acceptedSkillManifest"],
        json!([{"skillId":template["skillId"],"revision":template["revision"],"name":template["name"]}])
    );
    assert_eq!(packed["partial"], false);
    assert_eq!(packed["acceptedSkillManifestPartial"], false);
    assert_eq!(packed["profilePartial"], false);
    assert_eq!(packed["acceptedSkillTotal"], 1);
}

#[cfg(unix)]
#[tokio::test]
async fn brand_and_taught_skills_are_scoped_edited_accepted_revoked_and_durable() {
    let (tmp, host, p) = local().await;
    let source = resource(&host, tmp.path(), &p, "Actual recorded process transcript").await;
    let request = brand(&p, 0, json!([source]));
    let saved = host
        .action("project_brand_save", request.clone())
        .await
        .unwrap();
    assert_eq!(saved["revision"], 1);
    assert_eq!(
        host.action("project_brand_save", request).await.unwrap(),
        saved
    );
    assert!(host
        .action("project_brand_save", brand(&p, 0, json!([])))
        .await
        .is_err());
    let proposal = host
        .action("project_skill_propose", proposed(&p, &source))
        .await
        .unwrap();
    assert!(
        standing_context(&host.store.read_events().unwrap(), p.as_str().unwrap()).unwrap()
            ["acceptedSkills"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let accept = accepted(&p, &proposal["skillId"], 1);
    let result = host
        .action("project_skill_accept", accept.clone())
        .await
        .unwrap();
    assert_eq!(result["revision"], 2);
    let context =
        standing_context(&host.store.read_events().unwrap(), p.as_str().unwrap()).unwrap();
    assert_eq!(context["profile"]["revision"], 1);
    assert_eq!(
        context["acceptedSkills"][0]["steps"][0],
        "Follow the owner-reviewed checked process. PRIVATE_SKILL_CANARY"
    );
    let read = host
        .action(
            "project_capabilities_read",
            json!({"projectId":p,"section":"skills","offset":0,"limit":1}),
        )
        .await
        .unwrap();
    assert_eq!(read["items"][0]["status"], "accepted");
    assert_eq!(read["section"], "skills");
    let metadata = metadata(&project(&host.store.read_events().unwrap()).unwrap()).to_string();
    assert!(!metadata.contains("PRIVATE_BRAND_CANARY"));
    assert!(!metadata.contains("PRIVATE_SKILL_CANARY"));
    let foreign = host
        .action("project_create", command(json!({"name":"Separate"})))
        .await
        .unwrap()["projectId"]
        .clone();
    assert!(host
        .action(
            "project_skill_accept",
            accepted(&foreign, &proposal["skillId"], 2)
        )
        .await
        .is_err());
    assert!(host
        .action(
            "project_skill_accept",
            accepted(&p, &proposal["skillId"], 1)
        )
        .await
        .is_err());
    let revoked = host
        .action(
            "project_skill_revoke",
            command(json!({"projectId":p,"skillId":proposal["skillId"],"expectedSkillRevision":2})),
        )
        .await
        .unwrap();
    assert_eq!(revoked["revision"], 3);
    assert!(
        standing_context(&host.store.read_events().unwrap(), p.as_str().unwrap()).unwrap()
            ["acceptedSkills"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for version in [1, 2] {
        let mut snapshot = host.snapshot().unwrap();
        snapshot["sequence"] = json!(1);
        snapshot["recordedAt"] = json!(now());
        snapshot["agentPacks"] = snapshot["packs"].clone();
        let cloud = crate::cloud::build_projection(&snapshot, version, 1, "install:test")
            .unwrap()
            .to_string();
        assert!(!cloud.contains("PRIVATE_BRAND_CANARY"));
        assert!(!cloud.contains("PRIVATE_SKILL_CANARY"));
    }
    host.shutdown().await;
    drop(host);
    let reopened = Host::open(tmp.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .action("project_skill_accept", accept)
            .await
            .unwrap(),
        result
    );
    assert_eq!(state(&reopened).unwrap()["skills"][0]["status"], "revoked");
    reopened.shutdown().await;
}
#[cfg(unix)]
#[tokio::test]
async fn evidence_is_a_literal_frozen_utf8_byte_quote_and_preserves_disputes() {
    let (tmp, host, p) = local().await;
    let original="Observed source: İstanbul grew by 12% in the sample. A disputed interpretation remains visible.";
    let source = resource(&host, tmp.path(), &p, original).await;
    let quote = "İstanbul grew by 12%";
    let start = original.find(quote).unwrap();
    let request = command(
        json!({"projectId":p,"claim":"The sample reports growth. PRIVATE_EVIDENCE_CANARY","source":source,"quote":quote,"location":{"startByte":start,"endByte":start+quote.len(),"label":"Source paragraph1"},"observedAt":now(),"stance":"disputes"}),
    );
    let result = host
        .action("project_evidence_add", request.clone())
        .await
        .unwrap();
    assert_eq!(
        host.action("project_evidence_add", request.clone())
            .await
            .unwrap(),
        result
    );
    let read = host
        .action(
            "project_evidence_read",
            json!({"projectId":p,"evidenceId":result["evidenceId"]}),
        )
        .await
        .unwrap();
    assert_eq!(read["quote"], quote);
    assert_eq!(read["stance"], "disputes");
    assert!(read["sourceExcerpt"].as_str().unwrap().contains(quote));
    assert!(read["sourceText"].is_null());
    let mut fake = command(request.clone());
    fake["quote"] = json!("A fabricated claim");
    assert!(host.action("project_evidence_add", fake).await.is_err());
    let mut boundary = command(request);
    boundary["location"]["startByte"] = json!(start + 1);
    assert!(host.action("project_evidence_add", boundary).await.is_err());
    let events = host.store.read_events().unwrap();
    let index = events
        .iter()
        .position(|e| e["type"] == "project_capability.evidence_added")
        .unwrap();
    let mut bad = events.clone();
    bad[index]["payload"]["record"]["sourceText"] = json!("Replaced source");
    assert!(project(&bad).is_err());
    assert!(!metadata(&state(&host).unwrap())
        .to_string()
        .contains("PRIVATE_EVIDENCE_CANARY"));
    let foreign = host
        .action("project_create", command(json!({"name":"Foreign"})))
        .await
        .unwrap()["projectId"]
        .clone();
    assert!(host
        .action(
            "project_evidence_read",
            json!({"projectId":foreign,"evidenceId":result["evidenceId"]})
        )
        .await
        .is_err());
    host.shutdown().await;
}
#[tokio::test]
async fn owner_fact_recovery_and_exact_payload_replay_do_not_invent_acceptance() {
    let (tmp, host, p) = local().await;
    let request = brand(&p, 0, json!([]));
    let result = host
        .action("project_brand_save", request.clone())
        .await
        .unwrap();
    let mut events = host.store.read_events().unwrap();
    let outer = events.pop().unwrap();
    assert_eq!(outer["type"], "owner.command.receipted");
    let mut bad = events.clone();
    let index = bad
        .iter()
        .position(|e| e["type"] == "project_capability.brand_saved")
        .unwrap();
    bad[index]["payload"]["record"]["instructions"] = json!("Different standing instruction");
    assert!(project(&bad).is_err());
    host.shutdown().await;
    drop(host);
    fs::write(
        tmp.path().join("data/events.jsonl"),
        events.iter().map(|e| format!("{e}\n")).collect::<String>(),
    )
    .unwrap();
    let reopened = Host::open(tmp.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .action("project_brand_save", request)
            .await
            .unwrap(),
        result
    );
    reopened.shutdown().await;
}

#[cfg(unix)]
async fn saved_recipe(host: &Host, p: &Value) -> Value {
    let mut payload = prepare(host, p);
    payload["members"] = json!([crews::default_member("document", "D1")]);
    payload["settings"] = json!({"concurrency":1,"retryLimit":0,"timeoutMinutes":10});
    let prepared = host.action("project_crew_prepare", payload).await.unwrap();
    let m = mission(host, &prepared["missionId"]);
    host.action("project_recipe_save",command(json!({"projectId":p,"missionId":m["missionId"],"expectedMissionRevision":m["revision"],"name":"A useful frozen recipe"}))).await.unwrap()
}
#[cfg(unix)]
#[tokio::test]
async fn parallel_variants_are_real_prepared_independent_missions_with_explicit_start() {
    let (tmp, _peer, host, p) = provider_fixture("hold").await;
    let recipe = saved_recipe(&host, &p).await;
    let request = command(
        json!({"projectId":p,"recipeId":recipe["recipeId"],"expectedRecipeRevision":recipe["revision"],"instruction":"Create two distinct alternatives.","labels":["Detailed","Concise"],"providerId":"codex","budgets":{"maxJobs":1,"maxElapsedMinutes":10,"maxReviewRounds":0}}),
    );
    let result = host
        .action("project_variants_prepare", request.clone())
        .await
        .unwrap();
    assert_eq!(result["variants"].as_array().unwrap().len(), 2);
    assert_eq!(
        host.action("project_variants_prepare", request.clone())
            .await
            .unwrap(),
        result
    );
    let ids: Vec<_> = array(&result, "variants")
        .iter()
        .map(|v| v["missionId"].clone())
        .collect();
    assert_ne!(ids[0], ids[1]);
    for id in &ids {
        assert_eq!(mission(&host, id)["status"], "prepared");
    }
    assert!(array(&host.snapshot().unwrap()["work"], "items").is_empty());
    host.action(
        "project_crew_start",
        lifecycle(&p, &mission(&host, &ids[0])),
    )
    .await
    .unwrap();
    assert_eq!(mission(&host, &ids[0])["status"], "active");
    assert_eq!(mission(&host, &ids[1])["status"], "prepared");
    assert_eq!(
        array(&mission(&host, &ids[0]), "tasks")[0]["status"],
        "running"
    );
    let foreign = host
        .action("project_create", command(json!({"name":"Foreign"})))
        .await
        .unwrap()["projectId"]
        .clone();
    let bad = command(
        json!({"projectId":foreign,"recipeId":recipe["recipeId"],"expectedRecipeRevision":1,"instruction":"Wrong Project","labels":["a","b"],"providerId":"codex","budgets":{"maxJobs":1,"maxElapsedMinutes":10,"maxReviewRounds":0}}),
    );
    assert!(host.action("project_variants_prepare", bad).await.is_err());
    host.shutdown().await;
    drop(host);
    let reopened = Host::open(tmp.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .action("project_variants_prepare", request)
            .await
            .unwrap(),
        result
    );
    assert_eq!(mission(&reopened, &ids[1])["status"], "prepared");
    reopened.shutdown().await;
}
#[cfg(unix)]
#[tokio::test]
async fn committed_variant_allocations_recover_without_duplicate_missions_after_crash() {
    let (tmp, _peer, host, p) = provider_fixture("hold").await;
    let recipe = saved_recipe(&host, &p).await;
    let request = command(
        json!({"projectId":p,"recipeId":recipe["recipeId"],"expectedRecipeRevision":recipe["revision"],"instruction":"Prepare inspectable alternatives.","labels":["One","Two","Three"],"providerId":"codex","budgets":{"maxJobs":1,"maxElapsedMinutes":10,"maxReviewRounds":0}}),
    );
    let result = host
        .action("project_variants_prepare", request.clone())
        .await
        .unwrap();
    let events = host.store.read_events().unwrap();
    let committed = events
        .iter()
        .position(|e| e["type"] == "project_capability.variants_prepared")
        .unwrap();
    let prefix = events[..=committed].to_vec();
    host.shutdown().await;
    drop(host);
    fs::write(
        tmp.path().join("data/events.jsonl"),
        prefix.iter().map(|e| format!("{e}\n")).collect::<String>(),
    )
    .unwrap();
    let reopened = Host::open(tmp.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .action("project_variants_prepare", request)
            .await
            .unwrap(),
        result
    );
    assert!(
        !array(&reopened.snapshot().unwrap()["projectCrews"], "missions")
            .iter()
            .any(|m| array(&result, "variants")
                .iter()
                .any(|v| v["missionId"] == m["missionId"]))
    );
    reopened.advance_capabilities().await.unwrap();
    reopened.advance_capabilities().await.unwrap();
    for variant in array(&result, "variants") {
        assert_eq!(
            mission(&reopened, &variant["missionId"])["status"],
            "prepared"
        );
    }
    assert_eq!(
        array(&reopened.snapshot().unwrap()["projectCrews"], "missions")
            .iter()
            .filter(|m| array(&result, "variants")
                .iter()
                .any(|v| v["missionId"] == m["missionId"]))
            .count(),
        3
    );
    assert!(array(&reopened.snapshot().unwrap()["work"], "items").is_empty());
    reopened.shutdown().await;
}

#[cfg(unix)]
#[tokio::test]
async fn committed_variants_keep_applied_receipt_when_mission_preparation_fails() {
    let (tmp, _peer, host, p) = provider_fixture("hold").await;
    let recipe = saved_recipe(&host, &p).await;
    let request = command(
        json!({"projectId":p,"recipeId":recipe["recipeId"],"expectedRecipeRevision":recipe["revision"],"instruction":"Recover committed alternatives after a preparation failure.","labels":["One","Two"],"providerId":"codex","budgets":{"maxJobs":1,"maxElapsedMinutes":10,"maxReviewRounds":0}}),
    );
    allocation_failures()
        .lock()
        .unwrap()
        .insert(host.creation_data_dir().to_owned());
    let result = host
        .action("project_variants_prepare", request.clone())
        .await
        .unwrap();
    assert_eq!(result["outcome"], "applied");
    let events = host.store.read_events().unwrap();
    let receipt = events
        .iter()
        .find(|e| {
            e["type"] == "owner.command.receipted"
                && e["payload"]["commandId"] == request["commandId"]
        })
        .unwrap();
    assert_eq!(receipt["payload"]["result"], result);
    assert!(
        !array(&host.snapshot().unwrap()["projectCrews"], "missions")
            .iter()
            .any(|m| array(&result, "variants")
                .iter()
                .any(|v| v["missionId"] == m["missionId"]))
    );
    assert_eq!(
        host.action("project_variants_prepare", request.clone())
            .await
            .unwrap(),
        result
    );
    host.shutdown().await;
    drop(host);
    let reopened = Host::open(tmp.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .action("project_variants_prepare", request)
            .await
            .unwrap(),
        result
    );
    reopened.advance_capabilities().await.unwrap();
    reopened.advance_capabilities().await.unwrap();
    for variant in array(&result, "variants") {
        assert_eq!(
            mission(&reopened, &variant["missionId"])["status"],
            "prepared"
        );
    }
    assert_eq!(
        array(&reopened.snapshot().unwrap()["projectCrews"], "missions")
            .iter()
            .filter(|m| array(&result, "variants")
                .iter()
                .any(|v| v["missionId"] == m["missionId"]))
            .count(),
        2
    );
    assert!(array(&reopened.snapshot().unwrap()["work"], "items").is_empty());
    reopened.shutdown().await;
}

#[cfg(unix)]
fn after(start: chrono::DateTime<chrono::Utc>, seconds: i64) -> Value {
    json!((start + chrono::Duration::seconds(seconds))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

#[cfg(unix)]
async fn prepared_folder_occurrence(host: &Host, p: &Value, folder: &Path) -> (Value, Value) {
    let recipe = saved_recipe(host, p).await;
    fs::create_dir(folder).unwrap();
    fs::write(folder.join("input.txt"), b"Original").unwrap();
    let saved = host.action("project_trigger_save", command(json!({"projectId":p,"name":"Bounded folder trigger","kind":"folder","source":folder,"recipeId":recipe["recipeId"],"expectedRecipeRevision":1,"providerId":"codex","debounceSeconds":10,"maxOccurrences":3,"budgets":{"maxJobs":1,"maxElapsedMinutes":10,"maxReviewRounds":0}}))).await.unwrap();
    host.action("project_trigger_set_armed", command(json!({"projectId":p,"triggerId":saved["triggerId"],"expectedTriggerRevision":1,"armed":true,"armedForMinutes":1}))).await.unwrap();
    let trigger = state(host).unwrap()["triggers"][0].clone();
    let start = instant(&trigger["lastCheckedAt"]).unwrap();
    fs::write(folder.join("input.txt"), b"A real changed input").unwrap();
    host.advance_capabilities_at(after(start, 11))
        .await
        .unwrap();
    host.advance_capabilities_at(after(start, 22))
        .await
        .unwrap();
    let trigger = state(host).unwrap()["triggers"][0].clone();
    assert_eq!(trigger["occurrences"], 1);
    assert_eq!(
        mission(host, &trigger["activeMissionId"])["status"],
        "prepared"
    );
    let occurrence = host
        .store
        .read_events()
        .unwrap()
        .into_iter()
        .find(|e| e["type"] == "project_capability.occurrence_started")
        .unwrap();
    (trigger, occurrence)
}

#[cfg(unix)]
#[tokio::test]
async fn pausing_and_rearming_does_not_start_an_old_prepared_occurrence() {
    let (tmp, _peer, host, p) = provider_fixture("hold").await;
    let folder = tmp.path().join("watched");
    let (trigger, occurrence) = prepared_folder_occurrence(&host, &p, &folder).await;
    let old_mission = trigger["activeMissionId"].clone();
    host.action("project_trigger_set_armed", command(json!({"projectId":p,"triggerId":trigger["triggerId"],"expectedTriggerRevision":trigger["revision"],"armed":false}))).await.unwrap();
    let paused = state(&host).unwrap()["triggers"][0].clone();
    assert!(paused["activeMissionId"].is_null());
    host.action("project_trigger_set_armed", command(json!({"projectId":p,"triggerId":trigger["triggerId"],"expectedTriggerRevision":paused["revision"],"armed":true,"armedForMinutes":1}))).await.unwrap();
    let rearmed = state(&host).unwrap()["triggers"][0].clone();
    let start = instant(&rearmed["lastCheckedAt"]).unwrap();
    host.advance_capabilities_at(after(start, 30))
        .await
        .unwrap();
    assert_eq!(mission(&host, &old_mission)["status"], "prepared");
    assert_eq!(state(&host).unwrap()["triggers"][0]["occurrences"], 1);
    assert!(array(&host.snapshot().unwrap()["work"], "items").is_empty());
    let events = host.store.read_events().unwrap();
    let forged = json!({"projectId":p,"missionId":old_mission,"expectedMissionRevision":1,"origin":{"kind":"capability","allocationId":occurrence["payload"]["allocationId"]},"at":after(start,30)});
    assert!(automatic_start(&events, events.len(), &forged).is_err());
    fs::write(
        folder.join("input.txt"),
        b"A new change under the new consent",
    )
    .unwrap();
    host.advance_capabilities_at(after(start, 41))
        .await
        .unwrap();
    host.advance_capabilities_at(after(start, 52))
        .await
        .unwrap();
    let fresh = state(&host).unwrap()["triggers"][0].clone();
    assert_eq!(fresh["occurrences"], 2);
    assert_ne!(fresh["activeMissionId"], old_mission);
    host.advance_capabilities_at(after(start, 53))
        .await
        .unwrap();
    host.advance_crews().await.unwrap();
    let running = mission(&host, &fresh["activeMissionId"]);
    assert_eq!(running["tasks"][0]["status"], "running", "{running}");
    let current = state(&host).unwrap()["triggers"][0].clone();
    host.action("project_trigger_set_armed", command(json!({"projectId":p,"triggerId":trigger["triggerId"],"expectedTriggerRevision":current["revision"],"armed":false}))).await.unwrap();
    let paused = state(&host).unwrap()["triggers"][0].clone();
    assert_eq!(paused["activeMissionId"], fresh["activeMissionId"]);
    host.action("project_trigger_set_armed", command(json!({"projectId":p,"triggerId":trigger["triggerId"],"expectedTriggerRevision":paused["revision"],"armed":true,"armedForMinutes":1}))).await.unwrap();
    assert_eq!(
        state(&host).unwrap()["triggers"][0]["activeMissionId"],
        fresh["activeMissionId"]
    );
    assert_eq!(
        mission(&host, &fresh["activeMissionId"])["tasks"][0]["workItemId"],
        running["tasks"][0]["workItemId"]
    );
    assert_eq!(mission(&host, &old_mission)["status"], "prepared");
    host.shutdown().await;
}

#[cfg(unix)]
#[tokio::test]
async fn expired_prepared_occurrence_stays_retired_after_rearm_and_restart() {
    let (tmp, _peer, host, p) = provider_fixture("hold").await;
    let folder = tmp.path().join("watched");
    let (trigger, _) = prepared_folder_occurrence(&host, &p, &folder).await;
    let old_mission = trigger["activeMissionId"].clone();
    let events = host.store.read_events().unwrap();
    let armed = events
        .iter()
        .find(|e| e["type"] == "project_capability.trigger_armed")
        .unwrap();
    host.advance_capabilities_at(after(instant(&armed["payload"]["at"]).unwrap(), 65))
        .await
        .unwrap();
    let expired = state(&host).unwrap()["triggers"][0].clone();
    assert_eq!(expired["armed"], false);
    assert!(expired["activeMissionId"].is_null());
    assert_eq!(expired["lastReasonCode"], "CAPABILITY_TRIGGER_EXPIRED");
    host.action("project_trigger_set_armed", command(json!({"projectId":p,"triggerId":trigger["triggerId"],"expectedTriggerRevision":expired["revision"],"armed":true,"armedForMinutes":1}))).await.unwrap();
    let start = instant(&state(&host).unwrap()["triggers"][0]["lastCheckedAt"]).unwrap();
    host.shutdown().await;
    drop(host);
    let reopened = Host::open(tmp.path().join("data")).unwrap();
    reopened
        .advance_capabilities_at(after(start, 30))
        .await
        .unwrap();
    assert_eq!(mission(&reopened, &old_mission)["status"], "prepared");
    assert_eq!(state(&reopened).unwrap()["triggers"][0]["occurrences"], 1);
    assert!(array(&reopened.snapshot().unwrap()["work"], "items").is_empty());
    fs::write(folder.join("input.txt"), b"Changed after fresh arming").unwrap();
    reopened
        .advance_capabilities_at(after(start, 41))
        .await
        .unwrap();
    reopened
        .advance_capabilities_at(after(start, 52))
        .await
        .unwrap();
    let fresh = state(&reopened).unwrap()["triggers"][0].clone();
    assert_eq!(fresh["occurrences"], 2);
    assert_ne!(fresh["activeMissionId"], old_mission);
    assert_eq!(
        mission(&reopened, &fresh["activeMissionId"])["status"],
        "prepared"
    );
    reopened.shutdown().await;
}
#[cfg(unix)]
#[tokio::test]
async fn armed_folder_trigger_detects_real_bytes_debounces_and_expires_without_storms() {
    let (tmp, _peer, host, p) = provider_fixture("hold").await;
    let recipe = saved_recipe(&host, &p).await;
    let folder = tmp.path().join("watched");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("input.txt"), b"Original").unwrap();
    let save=host.action("project_trigger_save",command(json!({"projectId":p,"name":"Watch selected folder","kind":"folder","source":folder,"recipeId":recipe["recipeId"],"expectedRecipeRevision":1,"providerId":"codex","debounceSeconds":10,"maxOccurrences":2,"budgets":{"maxJobs":1,"maxElapsedMinutes":10,"maxReviewRounds":0}}))).await.unwrap();
    let id = save["triggerId"].clone();
    assert_eq!(state(&host).unwrap()["triggers"][0]["armed"], false);
    host.action("project_trigger_set_armed",command(json!({"projectId":p,"triggerId":id,"expectedTriggerRevision":1,"armed":true,"armedForMinutes":1}))).await.unwrap();
    let t = state(&host).unwrap()["triggers"][0].clone();
    let start = instant(&t["lastCheckedAt"]).unwrap();
    fs::write(folder.join("input.txt"), b"Changed actual source").unwrap();
    let at = |seconds| {
        json!((start + chrono::Duration::seconds(seconds))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    };
    host.advance_capabilities_at(at(11)).await.unwrap();
    assert_eq!(state(&host).unwrap()["triggers"][0]["occurrences"], 0);
    host.advance_capabilities_at(at(22)).await.unwrap();
    let triggered = state(&host).unwrap()["triggers"][0].clone();
    assert_eq!(triggered["occurrences"], 1);
    assert_eq!(
        mission(&host, &triggered["activeMissionId"])["status"],
        "prepared"
    );
    host.advance_capabilities_at(at(23)).await.unwrap();
    assert_eq!(
        mission(&host, &triggered["activeMissionId"])["status"],
        "active"
    );
    host.advance_capabilities_at(at(45)).await.unwrap();
    assert_eq!(state(&host).unwrap()["triggers"][0]["occurrences"], 1);
    host.advance_capabilities_at(at(65)).await.unwrap();
    assert_eq!(state(&host).unwrap()["triggers"][0]["armed"], false);
    assert_eq!(
        state(&host).unwrap()["triggers"][0]["lastReasonCode"],
        "CAPABILITY_TRIGGER_EXPIRED"
    );
    let events = host.store.read_events().unwrap();
    let occurrence = events
        .iter()
        .position(|e| e["type"] == "project_capability.occurrence_started")
        .unwrap();
    let mut bad = events[..=occurrence].to_vec();
    bad[occurrence]["payload"]["ordinal"] = json!(3);
    assert!(project(&bad).is_err());
    let mut bad = events[..=occurrence].to_vec();
    bad[occurrence]["payload"]["allocationId"] =
        json!(format!("allocation:{}", uuid::Uuid::new_v4()));
    assert!(project(&bad).is_err());
    let prepared = events
        .iter()
        .position(|e| {
            e["type"] == "project_workflow.mission_prepared"
                && e["payload"]["mission"]["missionId"] == triggered["activeMissionId"]
        })
        .unwrap();
    let mut late = events[..=prepared].to_vec();
    let t = project(&late).unwrap()["triggers"][0].clone();
    late.push(fact("project_capability.trigger_observed",json!({"projectId":p,"triggerId":id,"expectedTriggerRevision":t["revision"],"fingerprint":null,"reasonCode":"CAPABILITY_TRIGGER_EXPIRED","at":at(65)})));
    late.push(workflows::workflow_fact("project_workflow.mission_started",json!({"projectId":p,"missionId":triggered["activeMissionId"],"expectedMissionRevision":1,"origin":{"kind":"capability","allocationId":events[occurrence]["payload"]["allocationId"]},"at":at(66)})));
    assert!(
        crate::domain::project_events(&late).is_err(),
        "expired arming cannot authorize a late automatic start"
    );
    host.shutdown().await;
}
#[cfg(unix)]
#[tokio::test]
async fn url_trigger_uses_injected_read_only_bytes_and_rearm_takes_a_fresh_baseline() {
    let (_tmp, _peer, host, p) = provider_fixture("hold").await;
    let recipe = saved_recipe(&host, &p).await;
    let url = "https://example.invalid/public-monitor";
    url_bytes(&host, url, b"Original public page");
    let saved=host.action("project_trigger_save",command(json!({"projectId":p,"name":"Public page monitor","kind":"url","source":url,"recipeId":recipe["recipeId"],"expectedRecipeRevision":1,"providerId":"codex","debounceSeconds":10,"maxOccurrences":1,"budgets":{"maxJobs":1,"maxElapsedMinutes":10,"maxReviewRounds":0}}))).await.unwrap();
    let id = saved["triggerId"].clone();
    host.action("project_trigger_set_armed",command(json!({"projectId":p,"triggerId":id,"expectedTriggerRevision":1,"armed":true,"armedForMinutes":60}))).await.unwrap();
    let current = state(&host).unwrap()["triggers"][0].clone();
    host.action("project_trigger_set_armed",command(json!({"projectId":p,"triggerId":id,"expectedTriggerRevision":current["revision"],"armed":false}))).await.unwrap();
    url_bytes(&host, url, b"Changed while paused");
    let current = state(&host).unwrap()["triggers"][0].clone();
    host.action("project_trigger_set_armed",command(json!({"projectId":p,"triggerId":id,"expectedTriggerRevision":current["revision"],"armed":true,"armedForMinutes":60}))).await.unwrap();
    let current = state(&host).unwrap()["triggers"][0].clone();
    let start = instant(&current["lastCheckedAt"]).unwrap();
    let at = |seconds| {
        json!((start + chrono::Duration::seconds(seconds))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    };
    host.advance_capabilities_at(at(15)).await.unwrap();
    assert_eq!(state(&host).unwrap()["triggers"][0]["occurrences"], 0);
    url_bytes(&host, url, b"A second changed public page");
    host.advance_capabilities_at(at(30)).await.unwrap();
    host.advance_capabilities_at(at(41)).await.unwrap();
    assert_eq!(state(&host).unwrap()["triggers"][0]["occurrences"], 1);
    host.advance_capabilities_at(at(1200)).await.unwrap();
    assert_eq!(state(&host).unwrap()["triggers"][0]["occurrences"], 1);
    host.shutdown().await;
}
#[cfg(unix)]
#[test]
fn folder_monitor_rejects_links_internal_data_replacements_and_limits() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    fs::create_dir(&data).unwrap();
    let folder = tmp.path().join("watch");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("actual.txt"), b"Original").unwrap();
    let (_, identity) = folder_sample(&folder, &data, &Value::Null).unwrap();
    std::os::unix::fs::symlink(data.join("private.txt"), folder.join("link.txt")).unwrap();
    assert!(folder_sample(&folder, &data, &identity).is_err());
    fs::remove_file(folder.join("link.txt")).unwrap();
    assert!(folder_sample(&data, &data, &Value::Null).is_err());
    fs::rename(&folder, tmp.path().join("old-watch")).unwrap();
    fs::create_dir(&folder).unwrap();
    assert!(folder_sample(&folder, &data, &identity).is_err());
    for i in 0..33 {
        fs::write(folder.join(format!("{i}.txt")), b"bounded").unwrap();
    }
    assert!(folder_sample(&folder, &data, &Value::Null).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn actual_bound_mcp_proposes_but_never_accepts_skills_and_records_literal_evidence() {
    use std::sync::atomic::AtomicBool;
    let (tmp, _peer, host, p) = provider_fixture("hold").await;
    let source = resource(&host, tmp.path(), &p, "Literal frozen agent source.").await;
    let mut payload = prepare(&host, &p);
    payload["members"] = json!([crews::default_member("document", "D1")]);
    payload["settings"] = json!({"concurrency":1,"retryLimit":0,"timeoutMinutes":10});
    let prepared = host.action("project_crew_prepare", payload).await.unwrap();
    host.action(
        "project_crew_start",
        lifecycle(&p, &mission(&host, &prepared["missionId"])),
    )
    .await
    .unwrap();
    let current = mission(&host, &prepared["missionId"]);
    assert_eq!(current["tasks"][0]["status"], "running", "{current}");
    let work_id = current["tasks"][0]["workItemId"].clone();
    let snapshot = host.snapshot().unwrap();
    let context = array(&snapshot["agentMemory"], "contexts")
        .iter()
        .find(|c| c["workItemId"] == work_id)
        .unwrap();
    let binding = agent_memory::Binding {
        project_id: p.as_str().unwrap().into(),
        work_item_id: work_id.as_str().unwrap().into(),
        run_id: context["runId"].as_str().unwrap().into(),
        owner_epoch: host.epoch.clone(),
        decision_revision: context["decisionRevision"].as_u64().unwrap(),
        active: Arc::new(AtomicBool::new(true)),
    };
    let mut work = array(&snapshot["work"], "items")
        .iter()
        .find(|w| w["workItemId"] == work_id)
        .unwrap()
        .clone();
    work["runId"] = json!(binding.run_id);
    let ticket = host.prepare_agent_memory(&work).await.unwrap().unwrap();
    let credentials: Value =
        serde_json::from_slice(&crate::store::read_private_file(&ticket.path).unwrap()).unwrap();
    async fn call(credentials: &Value, tool: &str, args: Value) -> Value {
        reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(4))
            .build()
            .unwrap()
            .post(credentials["endpoint"].as_str().unwrap())
            .bearer_auth(credentials["bearer"].as_str().unwrap())
            .json(&json!({"schemaVersion":1,"tool":tool,"arguments":args}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }
    let args = json!({"name":"Extracted process","steps":["Inspect the literal source.","Preserve the checked originals."],"expectedOutputs":["Checked report"],"sources":[source],"nonce":"skill-1"});
    let result = call(&credentials, "project_skill_propose", args.clone()).await;
    assert_eq!(
        result["reasonCode"], "CAPABILITY_SKILL_PROPOSED",
        "{result}"
    );
    assert_eq!(
        call(&credentials, "project_skill_propose", args.clone()).await,
        result
    );
    let mut changed = args.clone();
    changed["steps"] = json!(["Changed process"]);
    let rejected = call(&credentials, "project_skill_propose", changed).await;
    assert!(
        rejected.to_string().contains("CAPABILITY_NONCE_REUSED"),
        "{rejected}"
    );
    let mut forged = args;
    forged["projectId"] = json!(format!("project:{}", uuid::Uuid::new_v4()));
    assert!(call(&credentials, "project_skill_propose", forged)
        .await
        .to_string()
        .contains("INVALID"));
    assert!(call(
        &credentials,
        "project_skill_accept",
        json!({"skillId":result["skillId"],"expectedSkillRevision":1})
    )
    .await
    .to_string()
    .contains("UNKNOWN"));
    let read = call(
        &credentials,
        "project_capabilities",
        json!({"section":"skills","limit":1}),
    )
    .await;
    assert_eq!(read["items"][0]["status"], "proposed", "{read}");
    assert!(
        standing_context(&host.store.read_events().unwrap(), p.as_str().unwrap()).unwrap()
            ["acceptedSkills"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let (directory, _) = host
        .creation_scope(&json!({"projectId":p,"workItemId":work_id}))
        .unwrap();
    fs::write(
        directory.join("outputs/extracted.txt"),
        b"Actual extracted source for citation.",
    )
    .unwrap();
    let publish_args = json!({"source":"outputs/extracted.txt","nonce":"source1"});
    let published = call(
        &credentials,
        "project_resource_publish",
        publish_args.clone(),
    )
    .await;
    assert_eq!(
        published["reasonCode"], "PROJECT_RESOURCE_PUBLISHED",
        "{published}"
    );
    assert_eq!(
        call(&credentials, "project_resource_publish", publish_args).await,
        published
    );
    let changed = call(
        &credentials,
        "project_resource_publish",
        json!({"source":"outputs/other.txt","nonce":"source1"}),
    )
    .await;
    assert!(changed.to_string().contains("INVALID"), "{changed}");
    let published_source = json!({"resourceId":published["resource"]["resourceId"],"expectedSha256":published["resource"]["sha256"]});
    let evidence=call(&credentials,"project_evidence_add",json!({"claim":"The actual source includes this phrase.","source":published_source,"quote":"Actual","location":{"startByte":0,"endByte":6,"label":"Start of extracted source"},"observedAt":now(),"stance":"context","nonce":"evidence-1"})).await;
    assert_eq!(
        evidence["reasonCode"], "CAPABILITY_EVIDENCE_ADDED",
        "{evidence}"
    );
    let read = call(
        &credentials,
        "project_capabilities",
        json!({"evidenceId":evidence["evidenceId"]}),
    )
    .await;
    assert!(read["sourceExcerpt"]
        .as_str()
        .unwrap()
        .contains("Actual extracted source for citation."));

    let python = crate::production::paths(host.creation_data_dir()).unwrap()["python"].clone();
    assert!(
        python.is_string(),
        "this real production fixture needs Python3.9+"
    );
    host.action(
        "project_tool_configure",
        command(json!({"projectId":p,"toolId":"python","executablePath":python})),
    )
    .await
    .unwrap();
    let args = json!({"operation":"interactive.create","arguments":{"kind":"table","title":"Actual interactive artifact","columns":["Name"],"rows":[["Checked source"]]},"nonce":"interactive-1"});
    let started = call(&credentials, "production_run", args.clone()).await;
    assert_eq!(started["status"], "running", "{started}");
    assert_eq!(
        call(&credentials, "production_run", args.clone()).await["jobId"],
        started["jobId"]
    );
    let mut completed = Value::Null;
    for _ in 0..80 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        host.background_tick(false).await.unwrap();
        let job = call(
            &credentials,
            "production_read",
            json!({"jobId":started["jobId"]}),
        )
        .await;
        if job["status"] != "running" {
            completed = job;
            break;
        }
    }
    assert_eq!(completed["status"], "completed", "{completed}");
    let artifact = &completed["artifacts"][0];
    assert!(artifact["path"].as_str().unwrap().ends_with(".html"));
    let bytes = crate::conversation::copied_bytes(
        &directory,
        &crate::creation::relative_result(artifact["path"].as_str().unwrap()).unwrap(),
        25 * 1024 * 1024,
    )
    .unwrap();
    assert_eq!(artifact["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    assert!(std::str::from_utf8(&bytes)
        .unwrap()
        .contains("Checked source"));
    let opened = host
        .action(
            "project_artifact_open",
            json!({"projectId":p,"workItemId":work_id,"path":artifact["path"],"expectedSha256":artifact["sha256"]}),
        )
        .await
        .unwrap();
    assert_eq!(opened["reasonCode"], "PROJECT_ARTIFACT_OPEN");
    assert_eq!(opened["sha256"], artifact["sha256"]);
    assert_eq!(
        fs::read(opened["openPath"].as_str().unwrap()).unwrap(),
        bytes
    );
    assert_eq!(
        call(&credentials, "production_run", args).await["jobId"],
        started["jobId"]
    );
    let mut stale = binding.clone();
    stale.owner_epoch = "epoch:stale".into();
    assert!(host
        .capability_agent_call(&stale, "project_capabilities", &json!({}))
        .await
        .is_err());
    host.shutdown().await;
}
