#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod command;
mod folder;
mod i18n;
mod motion;
mod studio;
mod ui;
#[cfg(feature = "ui-smoke")]
mod ui_kit;

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
        let console = ui::Console::launch();
        #[cfg(feature = "ui-smoke")]
        let console = {
            let mut console = console;
            if !observe {
                if let Ok(scene) = std::env::var("PATCHFLEET_SMOKE_SCENE") {
                    if scene.starts_with("quick-") {
                        console.quick_start_smoke();
                        if matches!(scene.as_str(), "quick-task" | "quick-failed") {
                            // Illustrative readiness only; no installed provider is probed or run.
                            bridge.snapshot.as_mut().unwrap()["observation"] = serde_json::json!({"observations":[{"provider":{"id":"codex","displayName":"Codex · UI fixture","state":"available","version":"0.155.1","capabilities":{"recentObservation":true,"explicitLiveStatus":true}},"sessions":[]}]});
                        }
                        if scene == "quick-failed" {
                            bridge.feedback = Some(command::Notice::error(
                                "work_start",
                                "PROVIDER_CONTROL_FAILED",
                            ));
                        }
                    }
                    if scene.starts_with("studio-") {
                        if scene == "studio-system-check" {
                            bridge.system_check_smoke();
                        }
                        console.studio_smoke(&scene, bridge.snapshot.as_ref().unwrap());
                    }
                    if scene.starts_with("capabilities-") {
                        bridge.chat_smoke(false);
                        bridge.capabilities_smoke(&scene);
                        bridge.smoke_provider_connections(true, "codex");
                        console.capabilities_smoke(bridge.snapshot.as_ref().unwrap(), &scene);
                    }
                    if scene.starts_with("conversation-") {
                        if matches!(
                            scene.as_str(),
                            "conversation-plan" | "conversation-questions"
                        ) {
                            bridge.productive_smoke(if scene == "conversation-questions" {
                                "productive-questions"
                            } else {
                                "productive-plan"
                            });
                        } else {
                            bridge.chat_smoke(false);
                        }
                        let provider = match scene.as_str() {
                            "conversation-claude" => "claude",
                            "conversation-gemini" => "gemini",
                            _ => "codex",
                        };
                        bridge
                            .smoke_provider_connections(scene != "conversation-offline", provider);
                        console.conversation_smoke(bridge.snapshot.as_ref().unwrap(), &scene);
                    }
                    if scene.starts_with("create-") {
                        let results = scene == "create-results";
                        if results {
                            bridge.create_smoke();
                        }
                        console.create_smoke(bridge.snapshot.as_ref().unwrap(), results);
                    }
                    if scene.starts_with("productive-") {
                        bridge.productive_smoke(&scene);
                        console.productive_smoke(bridge.snapshot.as_ref().unwrap(), &scene);
                    }
                    if scene.starts_with("crew-") {
                        bridge.crew_smoke(&scene);
                        console.crew_smoke(bridge.snapshot.as_ref().unwrap(), &scene);
                    }
                    if scene.starts_with("chat-") {
                        let files = scene == "chat-results";
                        let connections = scene == "chat-connections";
                        if files {
                            bridge.create_smoke();
                        }
                        bridge.chat_smoke(connections);
                        console.chat_smoke(bridge.snapshot.as_ref().unwrap(), files, connections);
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
            if !observe {
                bridge.smoke_provider_connections(false, "codex");
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
fn seed_quick_start_scene(directory: &std::path::Path, scene: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        if std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en") {
            host.action("desktop_language", command::Command::new("desktop_language", serde_json::json!({"language":"en"})).payload).await.unwrap();
        }
        if scene != "quick-project" {
            let created = host.action("project_create", command::Command::new("project_create", serde_json::json!({"name":"Welcome project · UI fixture"})).payload).await.unwrap();
            assert_eq!(created["outcome"], "applied");
            if scene != "quick-folder" {
                let path = directory.join("preview-repository");
                std::fs::create_dir(&path).unwrap();
                assert!(std::process::Command::new("git").args(["init", "--quiet"]).arg(&path).status().unwrap().success());
                let registration = host.action("workspace_register", command::Command::new("workspace_register", serde_json::json!({"workingDirectory":path,"displayName":"Welcome repository · UI fixture"})).payload).await.unwrap();
                assert_eq!(registration["outcome"], "applied");
                let snapshot = host.snapshot().unwrap();
                let linked = host.action("project_link_workspace", command::Command::new("project_link_workspace", serde_json::json!({"projectId":created["projectId"],"workspaceId":registration["workspaceId"],"expectedProjectRevision":snapshot["projectMemory"]["projects"][0]["revision"]})).payload).await.unwrap();
                assert_eq!(linked["outcome"], "applied");
                if scene == "quick-failed" {
                    let queued = host.action("work_enqueue", command::Command::new("work_enqueue", serde_json::json!({"title":"Welcome task · UI fixture","instruction":"Illustrative queued task. No provider turn was requested.","workspaceId":registration["workspaceId"],"workingDirectory":""})).payload).await.unwrap();
                    assert_eq!(queued["outcome"], "applied");
                }
            }
        }
        host.shutdown().await;
    });
}

#[cfg(feature = "ui-smoke")]
fn seed_create_scene(directory: &std::path::Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let host = patchfleet_core::Host::open(directory.to_owned()).unwrap();
        if std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en") {
            host.action("desktop_language", command::Command::new("desktop_language", serde_json::json!({"language":"en"})).payload).await.unwrap();
        }
        if matches!(std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref(), Ok("create-empty" | "chat-empty" | "conversation-empty")) {
            assert_eq!(host.snapshot().unwrap()["projectMemory"]["projects"], serde_json::json!([]));
            host.shutdown().await;
            return;
        }
        let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
        let scenario = std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()
            == Ok("capabilities-data-scenario");
        let extraction = std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()
            == Ok("capabilities-data-extract");
        let brief = std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()
            == Ok("capabilities-documents-brief");
        let template = std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()
            == Ok("capabilities-documents-template");
        let rewrite = std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()
            == Ok("capabilities-documents-rewrite");
        let source_changes = std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()
            == Ok("capabilities-source-changes");
        let claim_check = std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()
            == Ok("capabilities-claim-check");
        let project_name = if scenario {
            if english { "Operating plan" } else { "Faaliyet planı" }
        } else if extraction {
            if english { "Vendor review" } else { "Tedarikçi incelemesi" }
        } else if brief {
            if english { "Pilot decision" } else { "Pilot kararı" }
        } else if template {
            if english { "Partner onboarding" } else { "İş ortağı başlangıcı" }
        } else if rewrite {
            if english { "Launch translation" } else { "Lansman çevirisi" }
        } else if source_changes {
            if english { "Launch monitoring" } else { "Lansman takibi" }
        } else if claim_check {
            if english { "Launch evidence" } else { "Lansman kanıtları" }
        } else if english {
            "Product launch"
        } else {
            "Ürün lansmanı"
        };
        let created = host.action("project_create", command::Command::new("project_create",serde_json::json!({"name":project_name})).payload).await.unwrap();
        if std::env::var("PATCHFLEET_SMOKE_SCENE").is_ok_and(|scene| scene.starts_with("conversation-")) {
            host.shutdown().await;
            return;
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").is_ok_and(|scene| (scene.starts_with("crew-") || scene.starts_with("productive-")) && !matches!(scene.as_str(),"productive-output"|"productive-compare"|"productive-review"|"productive-resources"|"productive-apps"|"productive-delivery"|"productive-delivery-review")) {
            let goal = if english { "Research the audience and create a launch presentation backed by sources." } else { "Hedef kitleyi araştır ve kaynaklarla desteklenen bir lansman sunumu hazırla." };
            let plan = host.action("project_crew_plan", serde_json::json!({"projectId":created["projectId"],"instruction":goal,"abilityId":"P1"})).await.unwrap();
            let prepared = host.action("project_crew_prepare", command::Command::new("project_crew_prepare", serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":1,"title":if english {"Launch crew"} else {"Lansman ekibi"},"instruction":goal,"abilityId":"P1","members":plan["members"],"settings":plan["settings"]})).payload).await.unwrap();
            assert_eq!(prepared["outcome"],"applied");
            host.shutdown().await;
            return;
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("chat-compose") {
            host.shutdown().await;
            return;
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("chat-reduced") {
            host.action("desktop_motion", command::Command::new("desktop_motion", serde_json::json!({"reduceMotion":true})).payload).await.unwrap();
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("productive-resources") {
            let source=directory.join("rendering-input.txt");std::fs::write(&source,"Project brief · disposable UI resource. No provider was run.").unwrap();
            let snapshot=host.snapshot().unwrap();
            host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":snapshot["projectMemory"]["projects"][0]["revision"],"sourcePath":source})).payload).await.unwrap();
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-batch-images") {
            for (name, bytes) in [("campaign-cover.png", b"disposable image fixture one".as_slice()), ("product-photo.jpg", b"disposable image fixture two".as_slice())] {
                let source=directory.join(name);std::fs::write(&source,bytes).unwrap();
                let snapshot=host.snapshot().unwrap();
                let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
                host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
            }
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-podcast") {
            let source=directory.join("raw-episode.wav");std::fs::write(&source,b"disposable audio fixture").unwrap();
            let snapshot=host.snapshot().unwrap();
            let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
            host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-data-chart") {
            let source=directory.join("regional-revenue.csv");std::fs::write(&source,b"region,revenue\nNorth,42000\nSouth,31500\nWest,38750\nEast,46200\n").unwrap();
            let snapshot=host.snapshot().unwrap();
            let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
            host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-data-scenario") {
            let source=directory.join("operating-plan.csv");std::fs::write(&source,b"line_item,base_plan,growth_case,lean_case\nPlatform,950000,1200000,780000\nGrowth,620000,860000,410000\nOperations,440000,510000,390000\nPeople,1350000,1580000,1180000\n").unwrap();
            let snapshot=host.snapshot().unwrap();
            let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
            host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-data-extract") {
            for (name, body) in [
                ("atlas-renewal.txt", "Vendor: Atlas Cloud\nRenewal: 30 April 2027\nSeats: 120\n"),
                ("northwind-notes.md", "Vendor: Northwind AI\nRenewal: late Q2, date uncertain\nSeats were not stated.\n"),
            ] {
                let source=directory.join(name);std::fs::write(&source,body).unwrap();
                let snapshot=host.snapshot().unwrap();
                let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
                host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
            }
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-documents-brief") {
            for (name, body) in [
                ("pilot-notes.md", "Decision: Launch the pilot in May.\nOwner: Ece\nBudget approval is pending.\n"),
                ("security-review.txt", "Risk: Security review has not started.\nNext step: Schedule the review.\nOwner and date are not stated.\n"),
            ] {
                let source=directory.join(name);std::fs::write(&source,body).unwrap();
                let snapshot=host.snapshot().unwrap();
                let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
                host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
            }
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-documents-template") {
            let source=directory.join("partner-onboarding-template.docx");std::fs::write(&source,b"Disposable DOCX UI resource. No provider or template worker was run.").unwrap();
            let snapshot=host.snapshot().unwrap();
            let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
            host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-documents-rewrite") {
            let source=directory.join("launch-brief.txt");std::fs::write(&source,"Moxcrew Launch OS 2027-03-15 tarihinde başlıyor.\nBütçe: 1,250 USD.\nDetaylar: https://mox.example/launch\n").unwrap();
            let snapshot=host.snapshot().unwrap();
            let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
            host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-source-changes") {
            for (name, body) in [
                ("launch-page-may.md", "Plan: Private beta\nPrice: 100 TRY\nRegions: Türkiye\n"),
                ("launch-page-june.md", "Plan: Public launch\nPrice: 120 TRY\nRegions: Türkiye, Europe\n"),
            ] {
                let source=directory.join(name);std::fs::write(&source,body).unwrap();
                let snapshot=host.snapshot().unwrap();
                let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
                host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
            }
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-claim-check") {
            for (name, body) in [
                ("launch-survey.md", "Survey results\nİstanbul adoption reached 72%.\nThe sample included 50 teams.\n"),
                ("independent-audit.txt", "Independent audit\nThe 72% figure was not reproducible.\nNo retention data was collected.\n"),
            ] {
                let source=directory.join(name);std::fs::write(&source,body).unwrap();
                let snapshot=host.snapshot().unwrap();
                let project=snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
                host.action("project_resource_import",command::Command::new("project_resource_import",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"sourcePath":source})).payload).await.unwrap();
            }
        }
        let snapshot = host.snapshot().unwrap();
        let project = snapshot["projectMemory"]["projects"].as_array().unwrap().iter().find(|p|p["projectId"]==created["projectId"]).unwrap();
        let batch_images=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-batch-images");
        let podcast=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-podcast");
        let data_chart=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-data-chart");
        let data_scenario=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-data-scenario");
        let data_extract=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-data-extract");
        let documents_brief=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-documents-brief");
        let documents_template=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-documents-template");
        let documents_rewrite=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-documents-rewrite");
        let claim_check=std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-claim-check");
        let (ability,title,instruction)=if batch_images {
            ("I3",if english{"Campaign image batch"}else{"Kampanya görsel paketi"},if english{"Convert the selected campaign images within explicit bounds and report every source separately."}else{"Seçili kampanya görsellerini açık sınırlar içinde dönüştür ve her kaynağı ayrı raporla."})
        } else if podcast {
            ("A4",if english{"Podcast edit package"}else{"Podcast kurgu paketi"},if english{"Edit the selected recording into ordered chapters and produce a playable package."}else{"Seçili kaydı sıralı bölümlere kurgula ve oynatılabilir bir paket üret."})
        } else if data_chart {
            ("T3",if english{"Regional revenue chart"}else{"Bölgesel gelir grafiği"},if english{"Aggregate the checked CSV columns and produce a static chart with a concise report."}else{"Kontrol edilen CSV sütunlarını grupla ve kısa raporla birlikte statik grafik üret."})
        } else if data_scenario {
            ("T4",if english{"Operating plan scenarios"}else{"Faaliyet planı senaryoları"},if english{"Create an editable workbook whose checked formulas recalculate across base, upside and downside scenarios."}else{"Temel, iyimser ve temkinli senaryolarda yeniden hesaplanan doğrulanmış formüllü düzenlenebilir bir çalışma kitabı oluştur."})
        } else if data_extract {
            ("T5",if english{"Traceable vendor table"}else{"İzlenebilir tedarikçi tablosu"},if english{"Extract a structured table with literal evidence while keeping uncertain and missing fields visible."}else{"Belirsiz ve eksik alanları görünür tutarak birebir kanıtlı yapılandırılmış bir tablo çıkar."})
        } else if documents_brief {
            ("D3",if english{"Pilot decision brief"}else{"Pilot karar özeti"},if english{"Turn the selected notes into an attributed brief, explicit questions and an actionable plan."}else{"Seçili notları kaynaklı bir briefe, açık sorulara ve uygulanabilir eylem planına dönüştür."})
        } else if documents_template {
            ("D5",if english{"Partner onboarding form"}else{"İş ortağı başlangıç formu"},if english{"Fill every supported Word template field while keeping missing values explicit."}else{"Desteklenen tüm Word şablon alanlarını doldur ve eksik değerleri açık tut."})
        } else if documents_rewrite {
            ("D4",if english{"Launch translation"}else{"Lansman çevirisi"},if english{"Translate the exact launch copy while preserving its facts and reviewable source pairs."}else{"Lansman metnini bilgilerini ve incelenebilir kaynak eşlerini koruyarak çevir."})
        } else if source_changes {
            ("R5",if english{"Launch source changes"}else{"Lansman kaynağı değişiklikleri"},if english{"Compare two dated frozen launch snapshots and keep exact source hashes and explicit no-change handling."}else{"Tarihli iki dondurulmuş lansman sürümünü karşılaştır; kaynak özetlerini ve açık değişiklik yok sonucunu koru."})
        } else if claim_check {
            ("R4",if english{"Launch claim check"}else{"Lansman iddia doğrulaması"},if english{"Check launch claims against frozen sources while keeping support, disputes and unresolved evidence visible."}else{"Lansman iddialarını dondurulmuş kaynaklara göre kontrol et; destek, itiraz ve çözümlenmemiş kanıtları görünür tut."})
        } else {
            ("P1",if english{"Launch presentation · UI fixture"}else{"Lansman sunumu · arayüz sahnesi"},if english{"Prepare a launch presentation. Disposable rendering fixture; no provider execution was requested."}else{"Bir lansman sunumu hazırla. Bu tek kullanımlık arayüz sahnesinde sağlayıcı çalıştırılmadı."})
        };
        let queued = host.action("project_work_enqueue", command::Command::new("project_work_enqueue", serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"abilityId":ability,"title":title,"instruction":instruction})).payload).await.unwrap();
        assert_eq!(queued["outcome"], "applied");
        let snapshot = host.snapshot().unwrap();
        let work = snapshot["work"]["items"].as_array().unwrap().iter().find(|w|w["workItemId"]==queued["workItemId"]).unwrap();
        let output = std::path::Path::new(work["workingDirectory"].as_str().unwrap()).join("outputs");
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("capabilities-canvas") {
            let source=PathBuf::from(std::env::var("PATCHFLEET_SMOKE_ARTIFACT_FROM").expect("An actual PNG rendering fixture is required"));
            assert!(source.is_absolute());let bytes=std::fs::read(source).unwrap();
            assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n")&&bytes.len()<=2*1024*1024);
            std::fs::write(output.join("canvas.png"),bytes).unwrap();
        }
        std::fs::write(output.join("brief.md"), "# Launch presentation brief\n\nA real text file in this disposable project's output folder.\n\nThe job is queued. No model or presentation renderer was run for this interface fixture.\n\n## Requested deliverables\n\n- Source-backed research\n- Editable presentation\n- Short launch video\n\nOwner review remains explicit.\n").unwrap();
        std::fs::write(output.join("outline.csv"), "slide,topic\n1,The idea\n2,The audience\n3,The evidence\n").unwrap();
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("productive-delivery-review") {
            let page=host.action("project_studio_read",serde_json::json!({"projectId":created["projectId"]})).await.unwrap();
            for (path,status) in [("outputs/brief.md","reviewed"),("outputs/outline.csv","needs_changes")] {
                let artifact=page["artifacts"].as_array().unwrap().iter().find(|artifact|artifact["path"]==path).unwrap();
                let result=host.action("project_artifact_review_set",command::Command::new("project_artifact_review_set",serde_json::json!({"projectId":created["projectId"],"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"],"status":status})).payload).await.unwrap();
                assert_eq!(result["status"],status);
            }
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("productive-review") {
            let page=host.action("project_studio_read",serde_json::json!({"projectId":created["projectId"]})).await.unwrap();
            let artifact=&page["artifacts"][0];
            let reviewed=host.action("project_artifact_review_set",command::Command::new("project_artifact_review_set",serde_json::json!({"projectId":created["projectId"],"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"],"status":"reviewed","note":"Kaynak kapsamını ve çıktı yapısını kontrol ettim."})).payload).await.unwrap();
            assert_eq!(reviewed["status"],"reviewed");
        }
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref()==Ok("productive-compare") {
            let page=host.action("project_studio_read",serde_json::json!({"projectId":created["projectId"]})).await.unwrap();
            let artifact=page["artifacts"].as_array().unwrap().iter().find(|artifact|artifact["path"]=="outputs/brief.md").unwrap();
            let current=host.snapshot().unwrap();
            let project=current["projectMemory"]["projects"].as_array().unwrap().iter().find(|project|project["projectId"]==created["projectId"]).unwrap();
            let revised=host.action("project_artifact_revise",command::Command::new("project_artifact_revise",serde_json::json!({"projectId":created["projectId"],"expectedProjectRevision":project["revision"],"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"],"instruction":"Tighten the owner review statement and preserve the source-backed deliverables.","providerId":"codex"})).payload).await.unwrap();
            let current=host.snapshot().unwrap();
            let work=current["work"]["items"].as_array().unwrap().iter().find(|work|work["workItemId"]==revised["workItemId"]).unwrap();
            let output=std::path::Path::new(work["workingDirectory"].as_str().unwrap()).join("outputs");
            std::fs::write(output.join("brief.md"),"# Launch presentation brief · reviewed\n\nThis preserved revision sharpens the launch story without replacing the original file.\n\n## Requested deliverables\n\n- Source-backed audience research\n- Editable presentation with evidence\n- Short launch video\n\nOwner review remains required before delivery.\n").unwrap();
        }
        host.shutdown().await;
    });
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
        if let Some((request, paths)) = self.bridge.capability_paths.take() {
            self.console.complete_capability_paths(request, paths);
        }
        if let Some(path) = self.bridge.take_artifact_open() {
            #[cfg(target_os = "macos")]
            let opened = std::process::Command::new("/usr/bin/open")
                .arg(&path)
                .spawn();
            #[cfg(target_os = "windows")]
            let opened = std::process::Command::new("explorer.exe")
                .arg(&path)
                .spawn();
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            let opened = std::process::Command::new("xdg-open").arg(&path).spawn();
            if opened.is_err() {
                self.bridge.feedback = Some(command::Notice::error(
                    "project_artifact_open",
                    "PROJECT_ARTIFACT_OPEN_FAILED",
                ));
            }
        }
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
            if ctx.cumulative_frame_nr() >= 8 && !self.bridge.busy && !self.bridge.closing {
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
        #[cfg(feature = "ui-smoke")]
        if std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("ui-kit") {
            ui_kit::show(ui);
            return;
        }
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
            let command_busy =
                self.bridge.busy || self.bridge.closing || self.bridge.picking_folder;
            #[cfg(feature = "ui-smoke")]
            let command_busy = command_busy
                || std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("conversation-busy");
            let commands = self.console.show(
                ui,
                snapshot,
                command_busy,
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
            if !self.bridge.busy
                && !self.bridge.closing
                && !self.bridge.picking_folder
                && self.bridge.error.is_none()
            {
                if let Some(request) = self.console.take_studio_path_request() {
                    let title = match &request.kind {
                        ui::StudioPathKind::Resource => "Import resource",
                        ui::StudioPathKind::Tool { .. } => "Choose executable",
                    };
                    let choice = rfd::AsyncFileDialog::new()
                        .set_title(i18n::tr(language, title).into_owned())
                        .set_parent(frame)
                        .pick_file();
                    self.bridge
                        .choose_studio_path(ctx.clone(), request, async move {
                            choice.await.map(|file| file.path().to_owned())
                        });
                }
            }
            if !self.bridge.busy
                && !self.bridge.closing
                && !self.bridge.picking_folder
                && self.bridge.error.is_none()
            {
                if let Some(request) = self.console.take_capability_path_request() {
                    let kind = request.kind.clone();
                    let title = match kind {
                        ui::CapabilityPathKind::Resources => "Attach files",
                        ui::CapabilityPathKind::TriggerFolder => "Choose folder",
                    };
                    let dialog = rfd::AsyncFileDialog::new()
                        .set_title(i18n::tr(language, title).into_owned())
                        .set_parent(frame);
                    self.bridge
                        .choose_capability_paths(ctx.clone(), request, async move {
                            match kind {
                                ui::CapabilityPathKind::Resources => {
                                    dialog.pick_files().await.map(|files| {
                                        files
                                            .into_iter()
                                            .map(|f| f.path().to_string_lossy().into_owned())
                                            .collect()
                                    })
                                }
                                ui::CapabilityPathKind::TriggerFolder => dialog
                                    .pick_folder()
                                    .await
                                    .map(|f| vec![f.path().to_string_lossy().into_owned()]),
                            }
                        });
                }
            }
            if !self.bridge.busy
                && !self.bridge.closing
                && !self.bridge.picking_folder
                && self.bridge.error.is_none()
            {
                if let Some(request) = self.console.take_create_export_request() {
                    let choice = rfd::AsyncFileDialog::new()
                        .set_title(i18n::tr(language, "Export result").into_owned())
                        .set_file_name(&request.name)
                        .set_parent(frame)
                        .save_file();
                    self.bridge
                        .choose_create_export(ctx.clone(), request, async move {
                            choice.await.map(|file| file.path().to_owned())
                        });
                }
            }
            let delivery_ready = !self.bridge.busy
                && !self.bridge.closing
                && !self.bridge.picking_folder
                && self.bridge.error.is_none();
            if let Some(request) = self.console.take_delivery_export_request(delivery_ready) {
                let choice = rfd::AsyncFileDialog::new()
                    .set_title(i18n::tr(language, "Export package").into_owned())
                    .set_file_name(&request.name)
                    .add_filter("ZIP", &["zip"])
                    .set_parent(frame)
                    .save_file();
                self.bridge
                    .choose_delivery_export(ctx.clone(), request, async move {
                        choice.await.map(|file| file.path().to_owned())
                    });
            }
            if !self.bridge.busy
                && !self.bridge.closing
                && !self.bridge.picking_folder
                && self.bridge.error.is_none()
            {
                if let Some(target) = self.console.take_folder_request() {
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
        if let Ok(scene) = std::env::var("PATCHFLEET_SMOKE_SCENE") {
            if scene.starts_with("create-")
                || scene.starts_with("conversation-")
                || scene.starts_with("chat-")
                || scene.starts_with("crew-")
                || scene.starts_with("productive-")
                || scene.starts_with("capabilities-")
            {
                seed_create_scene(&directory);
            }
            if scene.starts_with("quick-") {
                seed_quick_start_scene(&directory, &scene);
            }
        }
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
    let window_size =
        if !observe && std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("ui-kit") {
            [1280.0, 1040.0]
        } else if !observe && std::env::var_os("PATCHFLEET_SMOKE_MIN_WINDOW").is_some() {
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
