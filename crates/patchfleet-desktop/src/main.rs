#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod command;
mod folder;
mod i18n;
mod studio;
mod ui;

use bridge::Bridge;
use eframe::egui;
use std::path::PathBuf;
#[cfg(feature = "ui-smoke")]
use std::time::{Duration, Instant};

struct Patchfleet {
    console: ui::Console,
    bridge: Bridge,
    directory: PathBuf,
    observe: bool,
    #[cfg(feature = "ui-smoke")]
    smoke_deadline: Option<Instant>,
    #[cfg(feature = "ui-smoke")]
    smoke_capture: Option<PathBuf>,
    #[cfg(feature = "ui-smoke")]
    smoke_capture_requested: bool,
}

impl Patchfleet {
    fn new(cc: &eframe::CreationContext<'_>, directory: PathBuf, observe: bool) -> Self {
        studio::install(&cc.egui_ctx);
        #[cfg(feature = "ui-smoke")]
        if !observe {
            match std::env::var("PATCHFLEET_SMOKE_THEME").as_deref() {
                Ok("dark") => cc.egui_ctx.set_theme(egui::Theme::Dark),
                Ok("light") => cc.egui_ctx.set_theme(egui::Theme::Light),
                _ => {}
            }
        }
        let mut bridge = Bridge::new(cc.egui_ctx.clone(), directory.clone(), observe);
        bridge.poll();
        #[cfg(feature = "ui-smoke")]
        if !observe {
            for _ in 0..400 {
                bridge.poll();
                if bridge.snapshot.is_some() || bridge.error.is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                bridge.snapshot.is_some() || bridge.error.is_some(),
                "Native smoke host failed to open"
            );
        }
        let console = ui::Console::default();
        #[cfg(feature = "ui-smoke")]
        let console = {
            let mut console = console;
            if !observe {
                if let Ok(scene) = std::env::var("PATCHFLEET_SMOKE_SCENE") {
                    if scene.starts_with("studio-") {
                        console.studio_smoke(&scene, bridge.snapshot.as_ref().unwrap());
                    }
                }
            }
            if !observe
                && std::env::var("PATCHFLEET_SMOKE_SCENE").is_ok_and(|s| s.starts_with("account-"))
            {
                let waiting =
                    std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("account-waiting");
                let origin = waiting.then(|| {
                    std::env::var("PATCHFLEET_SMOKE_ACCOUNT_URL")
                        .expect("Disposable account fixture required")
                });
                if let Some(origin) = &origin {
                    assert!(fixture_loopback_url(origin));
                }
                console.account_smoke(origin.as_deref());
                bridge.account_smoke(origin.as_deref());
            }
            if !observe
                && std::env::var("PATCHFLEET_SMOKE_SCENE")
                    .as_deref()
                    .is_ok_and(|s| matches!(s, "memory" | "extraction-memory"))
            {
                console.memory_smoke();
                bridge.memory_smoke();
            }
            if !observe
                && std::env::var("PATCHFLEET_SMOKE_SCENE")
                    .as_deref()
                    .is_ok_and(|s| matches!(s, "decisions" | "extraction-decisions"))
            {
                console.decision_smoke();
                bridge.decision_smoke();
            }
            if !observe
                && matches!(
                    std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref(),
                    Ok("local-search" | "cloud-search")
                )
            {
                let shared =
                    std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("cloud-search");
                let query = if shared {
                    "How do we avoid invoice rounding errors?"
                } else {
                    "tam sayı"
                };
                console.search_smoke(shared, query);
                bridge.search_smoke(shared, query);
            }
            if !observe && std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("shared-reader")
            {
                console.shared_smoke();
                bridge.shared_smoke();
            }
            if !observe
                && std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("shared-publisher")
            {
                console.publication_smoke();
                bridge.decision_smoke();
            }
            if !observe && std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("folder-picker")
            {
                console.folder_smoke(&directory.join("preview-repository"));
            }
            if !observe && std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("folder-error")
            {
                bridge.choose_folder(
                    cc.egui_ctx.clone(),
                    folder::Target::Registration,
                    std::future::ready(Some(directory.join("preview-repository"))),
                );
                // The test framebuffer capture runs before asynchronous UI results settle.
                for _ in 0..400 {
                    bridge.poll();
                    if !bridge.picking_folder {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                assert!(!bridge.picking_folder, "Native folder check did not finish");
            }
            console
        };
        Self {
            console,
            bridge,
            directory,
            observe,
            #[cfg(feature = "ui-smoke")]
            smoke_deadline: std::env::var("PATCHFLEET_SMOKE_SECONDS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|seconds| (1..=120).contains(seconds))
                .map(|seconds| Instant::now() + Duration::from_secs(seconds)),
            #[cfg(feature = "ui-smoke")]
            smoke_capture: (!observe)
                .then(|| std::env::var_os("PATCHFLEET_SMOKE_SCREENSHOT_TO").map(PathBuf::from))
                .flatten(),
            #[cfg(feature = "ui-smoke")]
            smoke_capture_requested: false,
        }
    }
}

#[cfg(feature = "ui-smoke")]
fn seed_memory_scene(directory: &std::path::Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        if std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en") {
            host.action("desktop_language", command::Command::new("desktop_language", serde_json::json!({"language":"en"})).payload).await.unwrap();
        }
        let created = host.action("project_create", command::Command::new("project_create", serde_json::json!({"name":"Billing sandbox"})).payload).await.unwrap();
        for (title, text) in [
            ("Money in minor units", "Store amounts as integer minor units. Keep the currency code beside each amount; avoid floating-point invoice calculations."),
            ("Yuvarlama sınırı", "Yuvarlama yalnızca toplam hesaplanırken yapılır. Vergi ve indirim kuralları ayrı kaynaklarla açıklanmalıdır."),
            ("Native preview fixture", "These illustrative messages were entered through the real local Host for this disposable rendering check. They are source context, not accepted ADRs or real agent activity."),
        ] {
            let snapshot = host.snapshot().unwrap();
            let project = &snapshot["projectMemory"]["projects"][0];
            let args = serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"title":title,"text":text});
            host.action("project_message", command::Command::new("project_message", args).payload).await.unwrap();
        }
        host.shutdown().await;
    });
}

#[cfg(feature = "ui-smoke")]
fn seed_studio_scene(directory: &std::path::Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        for (index, name) in ["Billing sandbox", "Interface lab · demo", "Developer docs · demo"].iter().enumerate() {
            let project_id = if index == 0 {
                host.snapshot().unwrap()["projectMemory"]["projects"][0]["projectId"].clone()
            } else {
                host.action("project_create", command::Command::new("project_create", serde_json::json!({"name":name})).payload).await.unwrap()["projectId"].clone()
            };
            let path = directory.join(format!("demo-repository-{index}"));
            std::fs::create_dir(&path).unwrap();
            assert!(std::process::Command::new("git").args(["init", "--quiet"]).arg(&path).status().unwrap().success());
            let registration = host.action("workspace_register", command::Command::new("workspace_register", serde_json::json!({"workingDirectory":path,"displayName":name})).payload).await.unwrap();
            assert_eq!(registration["outcome"], "applied");
            let snapshot = host.snapshot().unwrap();
            let project = snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|p|p["projectId"] == project_id).unwrap();
            let linked = host.action("project_link_workspace", command::Command::new("project_link_workspace", serde_json::json!({"projectId":project_id,"workspaceId":registration["workspaceId"],"expectedProjectRevision":project["revision"]})).payload).await.unwrap();
            assert_eq!(linked["outcome"], "applied");
            if index == 0 {
                for title in ["Review invoice rounding", "Document currency boundaries"] {
                    let result = host.action("work_enqueue", command::Command::new("work_enqueue", serde_json::json!({"title":title,"instruction":"Illustrative disposable UI fixture. No provider execution requested.","workspaceId":registration["workspaceId"],"workingDirectory":""})).payload).await.unwrap();
                    assert_eq!(result["outcome"], "applied");
                }
            }
        }
        host.shutdown().await;
    });
}

#[cfg(feature = "ui-smoke")]
fn seed_extraction_scene(directory: &std::path::Path) {
    // A deterministic provider result goes through canonical V7/V4 APIs. No model is run.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (project, source) = runtime.block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        let snapshot = host.snapshot().unwrap();
        let project = snapshot["projectMemory"]["projects"][0]["projectId"].clone();
        let enabled = host.action("project_extraction_configure", command::Command::new("project_extraction_configure", serde_json::json!({"projectId":project,"expectedPolicyRevision":0,"enabled":true,"consent":true})).payload).await.unwrap();
        assert_eq!(enabled["outcome"], "applied");
        let revision = host.snapshot().unwrap()["projectMemory"]["projects"][0]["revision"].clone();
        let source = host.action("project_message", command::Command::new("project_message", serde_json::json!({"projectId":project,"expectedProjectRevision":revision,"title":"Kesin para hesapları","text":"Para tutarlarını tam sayı kuruş olarak sakla. Para birimi kodunu tutarın yanında tut. Bu veri yalnızca yerel arayüz kontrolü içindir."})).payload).await.unwrap()["sourceId"].clone();
        host.shutdown().await;
        (project, source)
    });
    let store = patchfleet_core::store::Store::open(directory.to_owned()).unwrap();
    let events = store.read_events().unwrap();
    let projection = patchfleet_core::extraction::project_validated(&events).unwrap();
    let job = projection["jobs"][0]["jobId"].as_str().unwrap();
    store
        .transaction(|events| patchfleet_core::extraction::start(events, job, "native-preview"))
        .unwrap();
    let result = serde_json::json!({"proposals":[{"title":"Para tutarlarının saklanması","context":"Fatura ve ödeme hesapları.","decision":"Tutarları tam sayı kuruş olarak sakla ve para birimi kodunu yanında tut.","rationale":"","consequences":"","evidence":[{"sourceId":source,"revision":1,"excerpt":"Para tutarlarını tam sayı kuruş olarak sakla. Para birimi kodunu tutarın yanında tut."}],"relation":"new","relatedDecisionIds":[]}]});
    let finished = store
        .transaction(|events| {
            patchfleet_core::extraction::finish(events, job, 1, "native-preview", Ok(result))
        })
        .unwrap();
    assert_eq!(finished["job"]["projectId"], project);
    assert_eq!(finished["job"]["status"], "completed");
}

#[cfg(feature = "ui-smoke")]
fn seed_decision_scene(directory: &std::path::Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        let snapshot = host.snapshot().unwrap();
        let project = &snapshot["projectMemory"]["projects"][0];
        let sources = host.action("project_memory_read", serde_json::json!({"projectId":project["projectId"]})).await.unwrap();
        let citations = serde_json::json!([{"sourceId":sources["sources"][0]["sourceId"],"revision":sources["sources"][0]["revision"]}]);
        let mut records = Vec::new();
        for (title, decision) in [
            ("Para tutarlarının saklanması", "Tutarları tam sayı kuruş olarak sakla; para birimi kodunu tutarın yanında tut."),
            ("Kesin tutarlar ve açık yuvarlama sınırı", "Tutarları tam sayı alt birimlerle sakla. Yuvarlamayı toplam sınırında uygula; vergi kuralını ayrıca belgele."),
            ("Floating-point amounts", "Use floating-point invoice calculations."),
            ("Para birimi dönüşüm sınırı", "Döviz kurunu ve kur tarihini dönüşüm kaydında sakla."),
        ] {
            let args = serde_json::json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],
                "title":title,"context":"Fatura hesaplarında tekrar üretilebilir sonuçlar gerekiyor.","decision":decision,
                "rationale":"Kaynak mesajlar kesin hesap ve açık yuvarlama sınırı istiyor.",
                "consequences":"Dış sistemlerin tutarları giriş ve çıkış sınırlarında dönüştürülür. Bu kayıt yalnızca yerel test verisidir.","sources":citations});
            let record = host.action("project_decision_propose", command::Command::new("project_decision_propose", args).payload).await.unwrap();
            assert_eq!(record["outcome"], "applied");
            records.push(record);
        }
        for (action, args) in [
            ("project_decision_accept", serde_json::json!({"projectId":project["projectId"],"decisionId":records[0]["decisionId"],"expectedDecisionRevision":1})),
            ("project_decision_supersede", serde_json::json!({"projectId":project["projectId"],"decisionId":records[0]["decisionId"],"expectedDecisionRevision":2,"replacementDecisionId":records[1]["decisionId"],"expectedReplacementRevision":1})),
            ("project_decision_reject", serde_json::json!({"projectId":project["projectId"],"decisionId":records[2]["decisionId"],"expectedDecisionRevision":1})),
        ] {
            let receipt = host.action(action, command::Command::new(action, args).payload).await.unwrap();
            assert_eq!(receipt["outcome"], "applied");
        }
        host.shutdown().await;
    });
}

#[cfg(feature = "ui-smoke")]
fn seed_shared_scene(directory: &std::path::Path, publisher: bool) {
    let url = std::env::var("PATCHFLEET_SMOKE_MEMORY_URL")
        .expect("Disposable memory fixture URL required");
    let parsed = fixture_loopback_url(&url);
    assert!(
        parsed,
        "Native memory smoke accepts only a literal loopback fixture"
    );
    let code = std::env::var("PATCHFLEET_SMOKE_MEMORY_PAIR").expect("Disposable pairing required");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        let project = host.snapshot().unwrap()["projectMemory"]["projects"][0].clone();
        host.action("cloud_pair",command::Command::new("cloud_pair",serde_json::json!({"cloudUrl":url,"displayName":"Disposable memory preview","pairingCode":code})).payload).await.unwrap();
        if publisher {
            let sources=host.action("project_memory_read",serde_json::json!({"projectId":project["projectId"]})).await.unwrap();
            let draft=host.action("project_decision_propose",command::Command::new("project_decision_propose",serde_json::json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],
                "title":"Kesin tutarlar ve açık yuvarlama sınırı","context":"Bu kayıt yalnızca yerel arayüz test verisidir.","decision":"Tutarları tam sayı alt birimlerle sakla; yuvarlamayı toplam sınırında uygula.","rationale":"Tekrar üretilebilir fatura hesapları.","consequences":"Giriş ve çıkış sınırlarında tutar dönüşümü gerekir.",
                "sources":[{"sourceId":sources["sources"][0]["sourceId"],"revision":sources["sources"][0]["revision"]}]})).payload).await.unwrap();
            host.action("project_decision_accept",command::Command::new("project_decision_accept",serde_json::json!({"projectId":project["projectId"],"decisionId":draft["decisionId"],"expectedDecisionRevision":1})).payload).await.unwrap();
            host.action("memory_cloud_create",command::Command::new("memory_cloud_create",serde_json::json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],"name":"Publication preview fixture"})).payload).await.unwrap();
        } else {
            let shared=std::env::var("PATCHFLEET_SMOKE_MEMORY_PROJECT").expect("Disposable shared project required");
            host.action("memory_cloud_link",command::Command::new("memory_cloud_link",serde_json::json!({"projectId":project["projectId"],"expectedProjectRevision":project["revision"],"sharedProjectId":shared})).payload).await.unwrap();
        }
        host.action("memory_cloud_sync",command::Command::new("memory_cloud_sync",serde_json::json!({"projectId":project["projectId"]})).payload).await.unwrap();
        host.shutdown().await;
    });
}

#[cfg(feature = "ui-smoke")]
fn fixture_loopback_url(value: &str) -> bool {
    // No production/test endpoint override exists in feature-off builds.
    value.strip_prefix("http://127.0.0.1:").is_some_and(|port| {
        !port.is_empty()
            && port.bytes().all(|b| b.is_ascii_digit())
            && port.parse::<u16>().is_ok_and(|n| n > 0)
    })
}

#[cfg(feature = "ui-smoke")]
fn seed_account_scene(directory: &std::path::Path) {
    let url =
        std::env::var("PATCHFLEET_SMOKE_ACCOUNT_URL").expect("Disposable account fixture required");
    assert!(fixture_loopback_url(&url));
    let code = std::env::var("PATCHFLEET_SMOKE_ACCOUNT_PAIR").expect("Disposable pairing required");
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        host.action("cloud_pair", command::Command::new("cloud_pair", serde_json::json!({"cloudUrl":url,"displayName":"Native account preview","pairingCode":code})).payload).await.unwrap();
        host.shutdown().await;
    });
}

impl eframe::App for Patchfleet {
    #[cfg(feature = "ui-smoke")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                if let Some(path) = self.smoke_capture.take() {
                    image::save_buffer(
                        path,
                        image.as_raw(),
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    )
                    .expect("Native smoke screenshot could not be saved");
                    self.bridge.begin_close();
                }
            }
        }
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.bridge.poll();
        if let Some((target, selection)) = self.bridge.folder_selection.take() {
            self.console.accept_folder(target, selection);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.bridge.closed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.bridge.begin_close();
        }
        #[cfg(feature = "ui-smoke")]
        if self.smoke_capture.is_some() && !self.smoke_capture_requested {
            // Areas need a sizing frame before their contents appear in a GL readback.
            ctx.request_repaint();
            if ctx.cumulative_frame_nr() >= 8 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.smoke_capture_requested = true;
            }
        }
        #[cfg(feature = "ui-smoke")]
        if let Some(deadline) = self.smoke_deadline {
            if Instant::now() >= deadline {
                self.bridge.begin_close();
            } else {
                ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
            }
        }
        if self.bridge.closed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let language = self
            .bridge
            .snapshot
            .as_ref()
            .map(i18n::Language::from_snapshot)
            .unwrap_or(i18n::Language::Tr);
        let ctx = ui.ctx().clone();
        let preference_error = self
            .bridge
            .snapshot
            .as_ref()
            .is_some_and(|s| s["desktopPreferences"]["error"] == true);
        if self.bridge.closing || preference_error {
            egui::Panel::top("host_status").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if self.bridge.closing {
                        ui.label(i18n::tr(language, "Stopping owned agents…"));
                    } else if self.bridge.busy {
                        ui.label(i18n::tr(language, "Applying action…"));
                    }
                });
                if preference_error {
                    ui.label(i18n::tr(
                        language,
                        "Language preference could not be read. Select a language to replace it.",
                    ));
                }
            });
        }
        if let Some(error) = self.bridge.error.clone() {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading(i18n::tr(language,"The local host needs attention"));
                if error == "LOCAL_WRITER_ALREADY_RUNNING" {
                    ui.label(i18n::tr(language,"Close another Moxcrew instance if it owns this data directory. Your local state is preserved."));
                } else {
                    ui.label(i18n::tr(language,"Local data could not be opened safely. Close Moxcrew and back up its data before checking the storage or access permissions."));
                }
                ui.collapsing(i18n::tr(language,"Technical details"), |ui| { ui.monospace(&error); });
                if !self.bridge.closing && ui.button(i18n::tr(language,"Retry opening the host")).clicked() {
                    self.bridge.shutdown();
                    self.bridge = Bridge::new(ctx.clone(), self.directory.clone(), self.observe);
                }
            });
        } else if let Some(snapshot) = &self.bridge.snapshot {
            let commands = self.console.show(
                ui,
                snapshot,
                self.bridge.busy || self.bridge.closing || self.bridge.picking_folder,
                self.bridge.feedback.as_ref(),
                self.bridge.pending.is_some(),
            );
            if let Some(action) = self.console.notice_action.take() {
                match action {
                    ui::NoticeAction::Dismiss => self.bridge.dismiss_feedback(),
                    ui::NoticeAction::Retry => if let Some(command) = self.bridge.pending.clone() { self.bridge.submit(command); },
                    #[cfg(target_os="macos")]
                    ui::NoticeAction::Settings => ctx.open_url(egui::OpenUrl::new_tab("x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders")),
                }
            }
            if let Some(command) = commands.into_iter().next() {
                self.bridge.submit(command);
            }
            if let Some(target) = self.console.take_folder_request() {
                if !self.bridge.busy && !self.bridge.closing && !self.bridge.picking_folder {
                    let mut dialog = rfd::AsyncFileDialog::new()
                        .set_title(
                            i18n::tr(language, "Choose your Git repository folder").into_owned(),
                        )
                        .set_can_create_directories(false)
                        .set_parent(frame);
                    let previous = self.console.folder_path(target);
                    if !previous.is_empty() {
                        dialog = dialog.set_directory(previous);
                    }
                    let choice = dialog.pick_folder();
                    self.bridge.choose_folder(ctx.clone(), target, async move {
                        choice.await.map(|file| file.path().to_owned())
                    });
                }
            }
        } else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading(i18n::tr(language, "Opening Moxcrew"));
                ui.label(i18n::tr(language, "Loading the local Rust host…"));
            });
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.bridge.shutdown();
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    if let Some(argument) = args.next() {
        if argument == "--memory-mcp" {
            let ticket = args
                .next()
                .map(PathBuf::from)
                .filter(|path| path.is_absolute());
            if ticket.is_none() || args.next().is_some() {
                eprintln!("AGENT_MEMORY_INVALID_ARGUMENTS");
                std::process::exit(2);
            }
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = runtime.map_err(|_| ()).and_then(|runtime| {
                runtime
                    .block_on(patchfleet_core::agent_memory::serve_stdio(&ticket.unwrap()))
                    .map_err(|_| ())
            });
            if result.is_err() {
                eprintln!("AGENT_MEMORY_TRANSPORT_FAILED");
                std::process::exit(1);
            }
            return;
        }
        if argument == "--gemini-hook" && args.next().is_none() {
            if patchfleet_core::gemini_hook().is_err() {
                eprintln!("GEMINI_HOOK_FAILED");
                std::process::exit(1);
            }
            return;
        }
        eprintln!("INVALID_ARGUMENTS");
        std::process::exit(2);
    }
    let directory = patchfleet_core::default_data_directory().unwrap_or_default();
    let observe =
        !cfg!(feature = "ui-smoke") || std::env::var_os("PATCHFLEET_NATIVE_SMOKE").is_none();
    if !observe {
        // Test-only builds cannot accidentally probe providers or touch an installation.
        assert!(
            directory.is_dir() && std::fs::read_dir(&directory).unwrap().next().is_none(),
            "Native smoke requires an empty test directory"
        );
        #[cfg(feature = "ui-smoke")]
        if std::env::var("PATCHFLEET_SMOKE_SCENE").is_ok_and(|s| s.starts_with("studio-")) {
            seed_memory_scene(&directory);
            seed_decision_scene(&directory);
            seed_studio_scene(&directory);
        }
        #[cfg(feature = "ui-smoke")]
        if std::env::var("PATCHFLEET_SMOKE_SCENE").is_ok_and(|s| s.starts_with("account-")) {
            seed_memory_scene(&directory);
            if matches!(
                std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref(),
                Ok("account-free" | "account-paid")
            ) {
                seed_account_scene(&directory);
            }
        }
        #[cfg(feature = "ui-smoke")]
        if matches!(
            std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref(),
            Ok("memory"
                | "extraction-memory"
                | "extraction-decisions"
                | "decisions"
                | "local-search"
                | "cloud-search"
                | "shared-reader"
                | "shared-publisher"
                | "projects"
                | "folder-picker"
                | "folder-error")
        ) {
            seed_memory_scene(&directory);
            if std::env::var("PATCHFLEET_SMOKE_SCENE").is_ok_and(|s| s.starts_with("extraction-")) {
                seed_extraction_scene(&directory);
            }
            if matches!(
                std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref(),
                Ok("decisions" | "local-search")
            ) {
                seed_decision_scene(&directory);
            }
            if matches!(
                std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref(),
                Ok("shared-reader" | "shared-publisher" | "cloud-search")
            ) {
                seed_shared_scene(
                    &directory,
                    std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("shared-publisher"),
                );
            }
            if matches!(
                std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref(),
                Ok("projects" | "folder-picker" | "folder-error")
            ) {
                let path = directory.join("preview-repository");
                std::fs::create_dir(&path).unwrap();
                assert!(std::process::Command::new("git")
                    .args(["init", "--quiet"])
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success());
                std::fs::write(
                    path.join("README.md"),
                    "Disposable native folder-picker fixture. Not owner work.\n",
                )
                .unwrap();
                #[cfg(unix)]
                if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("folder-error") {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o500))
                        .unwrap();
                }
            }
        }
    }
    let window_size = [1280.0, 880.0];
    #[cfg(feature = "ui-smoke")]
    let window_size = if !observe && std::env::var_os("PATCHFLEET_SMOKE_MIN_WINDOW").is_some() {
        [800.0, 560.0]
    } else {
        window_size
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Moxcrew")
            .with_app_id("com.patchfleet.desktop")
            .with_inner_size(window_size)
            .with_min_inner_size([800.0, 560.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/128x128.png"))
                    .expect("Bundled icon"),
            ),
        renderer: eframe::Renderer::Glow,
        persist_window: false,
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "Moxcrew",
        options,
        Box::new(move |cc| Ok(Box::new(Patchfleet::new(cc, directory, observe)))),
    ) {
        eprintln!("{}", command::safe_error(error));
        std::process::exit(1);
    }
}
