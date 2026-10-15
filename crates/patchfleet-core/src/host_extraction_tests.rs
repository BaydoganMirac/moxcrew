use super::*;
use std::{path::Path, process::Command, sync::OnceLock, time::Duration};

const PEER: &str = r###"
use std::{env,fs,io::{self,Read,Write},thread,time::Duration};
fn quote(s:&str)->String {format!("\"{}\"",s.replace('\\',"\\\\").replace('"',"\\\"").replace('\n',"\\n").replace('\r',"\\r").replace('\t',"\\t"))}
fn main(){
 let exe=env::current_exe().unwrap();let args:Vec<_>=env::args().skip(1).collect();
 if args==["--version"] {println!("codex-cli 0.155.1");return}
 assert!(args.contains(&"exec".into()));for flag in ["--ignore-user-config","--ignore-rules","--ephemeral","--json","--output-schema","read-only"] {assert!(args.contains(&flag.into()),"missing {flag}");}
 let mut prompt=String::new();io::stdin().read_to_string(&mut prompt).unwrap();
 assert!(prompt.contains("DATA:"));assert!(!args.iter().any(|a|a.contains("SOURCE_INPUT_CANARY")));
 fs::write(exe.with_extension("input"),prompt).unwrap();fs::write(exe.with_extension("ready"),b"ready").unwrap();
 if exe.file_stem().unwrap().to_string_lossy().contains("waiting") {for _ in 0..1200 {thread::sleep(Duration::from_millis(50));}}
 let output=fs::read_to_string(exe.with_extension("response")).unwrap_or_else(|_|"{\"proposals\":[]}".into());
 println!("{{\"type\":\"thread.started\",\"thread_id\":\"fixture-thread\"}}");
 println!("{{\"type\":\"turn.started\"}}");
 println!("{{\"type\":\"item.completed\",\"item\":{{\"id\":\"fixture-item\",\"type\":\"agent_message\",\"text\":{}}}}}",quote(&output));
 println!("{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":1,\"cached_input_tokens\":0,\"output_tokens\":1}}}}");io::stdout().flush().unwrap();
}
"###;

fn binary() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap().keep();
        let source = dir.join("peer.rs");
        std::fs::write(&source, PEER).unwrap();
        let binary = dir.join(if cfg!(windows) { "peer.exe" } else { "peer" });
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| {
            PathBuf::from(env!("CARGO"))
                .with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" })
                .into_os_string()
        });
        assert!(Command::new(rustc)
            .args(["--edition=2021"])
            .arg(source)
            .arg("-o")
            .arg(&binary)
            .status()
            .unwrap()
            .success());
        binary
    })
}
fn command(mut value: Value) -> Value {
    value["commandId"] = json!(format!("cmd:{}", uuid::Uuid::new_v4()));
    value
}
async fn apply(host: &Host, action: &str, value: Value) -> Value {
    let receipt = host.action(action, command(value)).await.unwrap();
    assert_eq!(receipt["outcome"], "applied", "{action}: {receipt}");
    receipt
}
fn project_revision(host: &Host, project: &Value) -> Value {
    host.snapshot().unwrap()["projectMemory"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == *project)
        .unwrap()["revision"]
        .clone()
}
async fn message(host: &Host, project: &Value, text: &str) -> Value {
    apply(host,"project_message",json!({"projectId":project,"expectedProjectRevision":project_revision(host,project),"title":"Money","text":text})).await["sourceId"].clone()
}
async fn enable(host: &Host, project: &Value, revision: u64, enabled: bool) -> Value {
    apply(host,"project_extraction_configure",json!({"projectId":project,"expectedPolicyRevision":revision,"enabled":enabled,"consent":enabled})).await
}
fn output(source: &Value, revision: u64) -> Value {
    json!({"proposals":[{"title":"Tam sayı para birimi","context":"Faturalama","decision":"Tutarları tam sayı kuruş olarak sakla.","rationale":"","consequences":"","evidence":[{"sourceId":source,"revision":revision,"excerpt":"Para tutarlarını tam sayı kuruş olarak sakla."}],"relation":"new","relatedDecisionIds":[]}]})
}
struct Fixture {
    _directory: tempfile::TempDir,
    data: PathBuf,
    program: PathBuf,
    host: Arc<Host>,
    project: Value,
}
impl Fixture {
    async fn new(mode: &str, installed: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let program = directory.path().join(format!(
            "codex-{mode}{}",
            if cfg!(windows) { ".exe" } else { "" }
        ));
        if installed {
            std::fs::copy(binary(), &program).unwrap();
        }
        let data = directory.path().join("data");
        let host = Host::open_with_controller(data.clone(), controller(&program)).unwrap();
        let project =
            apply(&host, "project_create", json!({"name":"Invoices"})).await["projectId"].clone();
        Self {
            _directory: directory,
            data,
            program,
            host,
            project,
        }
    }
    fn response(&self, value: Value) {
        std::fs::write(self.program.with_extension("response"), value.to_string()).unwrap();
    }
    fn jobs(&self) -> Vec<Value> {
        self.host.store.projections().unwrap()["projectExtraction"]["jobs"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }
    async fn done(&self) -> Value {
        for _ in 0..150 {
            self.host.background_tick(false).await.unwrap();
            if let Some(job) = self
                .jobs()
                .last()
                .filter(|j| !matches!(j["status"].as_str(), Some("queued" | "running")))
            {
                return job.clone();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("fixture job did not finish: {:?}", self.jobs())
    }
    async fn started(&self) {
        self.host.background_tick(false).await.unwrap();
        for _ in 0..150 {
            if self.program.with_extension("ready").exists() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("fixture did not start");
    }
}
fn controller(program: &Path) -> providers::Controller {
    providers::Controller::with_programs(providers::Programs {
        codex: program.into(),
        claude: "uninstalled".into(),
        gemini: "uninstalled".into(),
        timeout: Duration::from_secs(3),
    })
}

#[tokio::test]
async fn generated_draft_is_attributed_reviewed_then_shared_by_accepted_only_agent_tools() {
    let f = Fixture::new("valid", true).await;
    let old = message(
        &f.host,
        &f.project,
        "Historical context must not silently leave this device.",
    )
    .await;
    enable(&f.host, &f.project, 0, true).await;
    f.host.background_tick(false).await.unwrap();
    assert!(f.jobs().is_empty());
    assert!(!f.program.with_extension("input").exists());
    let foreign =
        apply(&f.host, "project_create", json!({"name":"Foreign"})).await["projectId"].clone();
    message(&f.host, &foreign, "FOREIGN_SOURCE_CANARY").await;
    let source = message(
        &f.host,
        &f.project,
        "Para tutarlarını tam sayı kuruş olarak sakla. SOURCE_INPUT_CANARY",
    )
    .await;
    f.response(output(&source, 1));
    let job = f.done().await;
    assert_eq!(job["status"], "completed", "{job}");
    let decision = job["proposalIds"][0].clone();
    let detail = f
        .host
        .action(
            "project_decision_read",
            json!({"projectId":f.project,"decisionId":decision}),
        )
        .await
        .unwrap();
    assert_eq!(detail["decision"]["status"], "proposed");
    assert_eq!(detail["decision"]["extraction"]["providerId"], "codex");
    assert_eq!(
        detail["decision"]["extraction"]["evidence"][0]["excerpt"],
        "Para tutarlarını tam sayı kuruş olarak sakla."
    );
    let sent = std::fs::read_to_string(f.program.with_extension("input")).unwrap();
    assert!(sent.contains("SOURCE_INPUT_CANARY"));
    assert!(!sent.contains("FOREIGN_SOURCE_CANARY"));
    assert!(!sent.contains(old.as_str().unwrap()));
    apply(
        &f.host,
        "project_decision_accept",
        json!({"projectId":f.project,"decisionId":decision,"expectedDecisionRevision":1}),
    )
    .await;
    // A later edit produces one new source-version job, without duplicating the decision.
    apply(&f.host,"project_source_edit",json!({"projectId":f.project,"sourceId":source,"expectedSourceRevision":1,"title":"Money updated","text":"Para tutarlarını tam sayı kuruş olarak sakla. NEW_SOURCE_CANARY"})).await;
    f.response(output(&source, 2));
    assert_eq!(f.done().await["reasonCode"], "EXTRACTION_DUPLICATE");
    let exact = f
        .host
        .action(
            "project_decision_read",
            json!({"projectId":f.project,"decisionId":decision}),
        )
        .await
        .unwrap();
    assert_eq!(exact["decision"]["status"], "accepted");
    assert!(exact["sources"][0]["text"]
        .as_str()
        .unwrap()
        .contains("SOURCE_INPUT_CANARY"));
    assert_eq!(
        exact["decision"]["extraction"],
        detail["decision"]["extraction"]
    );
    let repository = f._directory.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repository)
        .status()
        .unwrap()
        .success());
    let workspace = apply(
        &f.host,
        "workspace_register",
        json!({"workingDirectory":repository}),
    )
    .await["workspaceId"]
        .clone();
    apply(&f.host,"project_link_workspace",json!({"projectId":f.project,"workspaceId":workspace,"expectedProjectRevision":project_revision(&f.host,&f.project)})).await;
    let work=apply(&f.host,"work_enqueue",json!({"workspaceId":workspace,"title":"Apply decisions","instruction":"Follow the accepted ADR"})).await["workItemId"].clone();
    let request = intent(
        "start_work",
        json!({"workItemId":work,"expectedItemRevision":1}),
        &command(json!({})),
    );
    let run = format!("run:{}", request["intentId"].as_str().unwrap());
    f.host.store.append(vec![event("command.requested",json!({"intent":request})),event("run.launching",json!({"intentId":request["intentId"],"workItemId":work,"expectedItemRevision":1,"ownerEpoch":f.host.epoch}))]).unwrap();
    let ticket = f
        .host
        .prepare_agent_memory(&json!({"workItemId":work,"runId":run}))
        .await
        .unwrap()
        .unwrap();
    let binding = agent_memory::Binding {
        project_id: f.project.as_str().unwrap().into(),
        work_item_id: work.as_str().unwrap().into(),
        run_id: run,
        owner_epoch: f.host.epoch.clone(),
        decision_revision: 2,
        active: ticket.active.clone(),
    };
    assert_eq!(
        f.host
            .agent_memory_call(&binding, "project_context", json!({}))
            .await
            .unwrap()["total"],
        1
    );
    let accepted = f
        .host
        .agent_memory_call(&binding, "decision_read", json!({"decisionId":decision}))
        .await
        .unwrap();
    assert_eq!(accepted["decision"]["status"], "accepted");
    assert!(!accepted.to_string().contains("SOURCE_INPUT_CANARY"));
    let snapshot = f.host.snapshot().unwrap();
    assert!(!snapshot["projectExtraction"]
        .to_string()
        .contains("tam sayı"));
    for version in [1, 2] {
        let outgoing = crate::cloud::build_projection(&snapshot, version, 1, "install:test")
            .unwrap()
            .to_string();
        for forbidden in [
            "projectExtraction",
            "SOURCE_INPUT_CANARY",
            "FOREIGN_SOURCE_CANARY",
            "Tam sayı",
            "extraction",
        ] {
            assert!(!outgoing.contains(forbidden), "{forbidden}");
        }
    }
    f.host.shutdown().await;
}

#[tokio::test]
async fn disabling_inference_cancels_it_without_blocking_local_commands_or_creating_drafts() {
    let f = Fixture::new("waiting", true).await;
    enable(&f.host, &f.project, 0, true).await;
    let source = message(
        &f.host,
        &f.project,
        "Para tutarlarını tam sayı kuruş olarak sakla. SOURCE_INPUT_CANARY",
    )
    .await;
    f.response(output(&source, 1));
    f.started().await;
    tokio::time::timeout(
        Duration::from_secs(1),
        apply(
            &f.host,
            "project_create",
            json!({"name":"Local remains responsive"}),
        ),
    )
    .await
    .unwrap();
    enable(&f.host, &f.project, 1, false).await;
    let cancelled = f.done().await;
    assert_eq!(cancelled["status"], "cancelled");
    for _ in 0..50 {
        f.host.background_tick(false).await.unwrap();
        if std::fs::read_dir(f.data.join("extraction-scratch"))
            .unwrap()
            .next()
            .is_none()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let decisions = f
        .host
        .action("project_decisions_read", json!({"projectId":f.project}))
        .await
        .unwrap();
    assert_eq!(decisions["total"], 0);
    assert!(std::fs::read_dir(f.data.join("extraction-scratch"))
        .unwrap()
        .next()
        .is_none());
    f.host.shutdown().await;
}

#[tokio::test]
async fn missing_provider_failure_survives_restart_and_requires_manual_retry() {
    let mut f = Fixture::new("installed-later", false).await;
    enable(&f.host, &f.project, 0, true).await;
    let source = message(
        &f.host,
        &f.project,
        "Para tutarlarını tam sayı kuruş olarak sakla. SOURCE_INPUT_CANARY",
    )
    .await;
    let failed = f.done().await;
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["reasonCode"], "EXTRACTION_PROVIDER_UNAVAILABLE");
    let cursor = f.host.store.event_cursor().unwrap();
    for _ in 0..3 {
        f.host.background_tick(false).await.unwrap();
    }
    assert_eq!(f.host.store.event_cursor().unwrap(), cursor);
    f.host.shutdown().await;
    // Release the writer before replacing the Host, without using a production override.
    let temporary = Host::open_with_controller(
        f._directory.path().join("unused-data"),
        controller(&f.program),
    )
    .unwrap();
    drop(std::mem::replace(&mut f.host, temporary));
    std::fs::copy(binary(), &f.program).unwrap();
    f.response(output(&source, 1));
    let reopened = Host::open_with_controller(f.data.clone(), controller(&f.program)).unwrap();
    f.host.shutdown().await;
    f.host = reopened;
    f.host.background_tick(false).await.unwrap();
    assert_eq!(f.jobs()[0]["status"], "failed");
    assert!(!f.program.with_extension("ready").exists());
    apply(
        &f.host,
        "project_extraction_retry",
        json!({"projectId":f.project,"jobId":failed["jobId"],"expectedAttempt":1}),
    )
    .await;
    let done = f.done().await;
    assert_eq!(done["status"], "completed", "{done}");
    assert_eq!(done["attempt"], 2);
    f.host.shutdown().await;
}

#[tokio::test]
async fn edited_source_cancels_old_inference_and_only_current_version_can_complete() {
    let mut f = Fixture::new("waiting-edit", true).await;
    enable(&f.host, &f.project, 0, true).await;
    let source = message(
        &f.host,
        &f.project,
        "Para tutarlarını tam sayı kuruş olarak sakla.",
    )
    .await;
    f.response(output(&source, 1));
    f.started().await;
    apply(&f.host, "project_source_edit", json!({"projectId":f.project,"sourceId":source,"expectedSourceRevision":1,"title":"Question","text":"Should we change this later?"})).await;
    assert_eq!(f.jobs()[0]["status"], "cancelled");
    assert_eq!(f.jobs()[0]["reasonCode"], "EXTRACTION_SOURCE_CHANGED");
    assert_eq!(f.jobs()[1]["status"], "queued");
    f.host.shutdown().await;
    let temporary = Host::open_with_controller(
        f._directory.path().join("unused-data"),
        controller(&f.program),
    )
    .unwrap();
    drop(std::mem::replace(&mut f.host, temporary));
    f.program = f._directory.path().join(if cfg!(windows) {
        "codex-resume.exe"
    } else {
        "codex-resume"
    });
    std::fs::copy(binary(), &f.program).unwrap();
    f.response(json!({"proposals":[]}));
    f.host.shutdown().await;
    f.host = Host::open_with_controller(f.data.clone(), controller(&f.program)).unwrap();
    let job = f.done().await;
    assert_eq!(job["status"], "completed");
    assert_eq!(job["sourceRevision"], 2);
    assert_eq!(job["reasonCode"], "EXTRACTION_NO_ADR");
    assert!(job["proposalIds"].as_array().unwrap().is_empty());
    assert_eq!(
        f.host
            .action("project_decisions_read", json!({"projectId":f.project}))
            .await
            .unwrap()["total"],
        0
    );
    f.host.shutdown().await;
}

#[tokio::test]
async fn shutdown_interrupts_inference_and_reopen_does_not_charge_an_automatic_retry() {
    let mut f = Fixture::new("waiting-close", true).await;
    enable(&f.host, &f.project, 0, true).await;
    message(
        &f.host,
        &f.project,
        "Para tutarlarını tam sayı kuruş olarak sakla.",
    )
    .await;
    f.started().await;
    tokio::time::timeout(Duration::from_secs(3), f.host.shutdown())
        .await
        .unwrap();
    assert_eq!(f.jobs()[0]["status"], "interrupted");
    assert_eq!(f.jobs()[0]["reasonCode"], "EXTRACTION_INTERRUPTED");
    assert!(std::fs::read_dir(f.data.join("extraction-scratch"))
        .unwrap()
        .next()
        .is_none());
    let temporary = Host::open_with_controller(
        f._directory.path().join("unused-data"),
        controller(&f.program),
    )
    .unwrap();
    drop(std::mem::replace(&mut f.host, temporary));
    std::fs::remove_file(f.program.with_extension("ready")).unwrap();
    f.host.shutdown().await;
    f.host = Host::open_with_controller(f.data.clone(), controller(&f.program)).unwrap();
    let cursor = f.host.store.event_cursor().unwrap();
    f.host.background_tick(false).await.unwrap();
    assert_eq!(f.host.store.event_cursor().unwrap(), cursor);
    assert_eq!(f.jobs()[0]["attempt"], 1);
    assert!(!f.program.with_extension("ready").exists());
    f.host.shutdown().await;
}

#[tokio::test]
async fn older_failed_job_is_recoverable_through_its_bounded_source_page() {
    let f = Fixture::new("missing-history", false).await;
    enable(&f.host, &f.project, 0, true).await;
    let source = message(
        &f.host,
        &f.project,
        "Oldest source reference: store integer money.",
    )
    .await;
    let oldest = f.done().await;
    for index in 0..10 {
        message(&f.host, &f.project, &format!("Later source {index}")).await;
        assert_eq!(f.done().await["status"], "failed");
    }
    let snapshot = f.host.snapshot().unwrap();
    let recent = snapshot["projectExtraction"]["projects"][0]["recentJobs"]
        .as_array()
        .unwrap();
    assert_eq!(recent.len(), 10);
    assert!(!recent.iter().any(|job| job["jobId"] == oldest["jobId"]));
    let page = f
        .host
        .action(
            "project_memory_read",
            json!({"projectId":f.project,"query":"Oldest source reference","limit":1}),
        )
        .await
        .unwrap();
    assert_eq!(page["sources"].as_array().unwrap().len(), 1);
    assert_eq!(
        page["extractionRevision"],
        snapshot["projectExtraction"]["revision"]
    );
    let job = &page["sources"][0]["extractionJob"];
    assert_eq!(page["sources"][0]["sourceId"], source);
    assert_eq!(job["jobId"], oldest["jobId"]);
    assert_eq!(job["status"], "failed");
    assert!(job.get("rawProviderOutput").is_none());
    apply(
        &f.host,
        "project_extraction_retry",
        json!({"projectId":f.project,"jobId":job["jobId"],"expectedAttempt":job["attempt"]}),
    )
    .await;
    assert_eq!(f.jobs()[0]["status"], "queued");
    f.host.shutdown().await;
}
