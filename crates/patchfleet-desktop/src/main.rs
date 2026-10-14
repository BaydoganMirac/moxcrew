#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod command;
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
}

impl Patchfleet {
    fn new(cc: &eframe::CreationContext<'_>, directory: PathBuf, observe: bool) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut style = (*cc.egui_ctx.global_style()).clone();
        let rgb = egui::Color32::from_rgb;
        let text = rgb(232, 235, 239);
        let muted = rgb(134, 145, 159);
        let accent = rgb(139, 224, 188);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.interact_size.y = 30.0;
        for (kind, size) in [
            (egui::TextStyle::Body, 13.0),
            (egui::TextStyle::Button, 13.0),
            (egui::TextStyle::Small, 11.0),
            (egui::TextStyle::Heading, 22.0),
        ] {
            style
                .text_styles
                .insert(kind, egui::FontId::proportional(size));
        }
        style.animation_time = 0.0;
        style.scroll_animation = egui::style::ScrollAnimation::none();
        style.visuals.override_text_color = Some(text);
        style.visuals.weak_text_color = Some(muted);
        style.visuals.panel_fill = rgb(17, 19, 24);
        style.visuals.window_fill = rgb(22, 26, 32);
        style.visuals.text_edit_bg_color = Some(rgb(18, 21, 26));
        style.visuals.selection.bg_fill = rgb(39, 76, 61);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, accent);
        style.visuals.hyperlink_color = accent;
        style.visuals.window_stroke = egui::Stroke::new(1.0, rgb(43, 48, 57));
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.bg_fill = rgb(29, 34, 41);
            widget.weak_bg_fill = rgb(22, 26, 32);
            widget.bg_stroke = egui::Stroke::new(1.0, rgb(43, 48, 57));
            widget.fg_stroke = egui::Stroke::new(1.0, text);
            widget.corner_radius = egui::CornerRadius::same(6);
        }
        style.visuals.widgets.hovered.bg_fill = rgb(40, 48, 57);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, rgb(67, 90, 79));
        style.visuals.widgets.active.bg_fill = rgb(42, 73, 59);
        cc.egui_ctx.set_global_style(style);
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
        Self {
            console: ui::Console::default(),
            bridge,
            directory,
            observe,
            #[cfg(feature = "ui-smoke")]
            smoke_deadline: std::env::var("PATCHFLEET_SMOKE_SECONDS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|seconds| (1..=120).contains(seconds))
                .map(|seconds| Instant::now() + Duration::from_secs(seconds)),
        }
    }
}

impl eframe::App for Patchfleet {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.bridge.poll();
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && !self.bridge.closed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.bridge.begin_close();
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
        if self.bridge.feedback.is_some() || self.bridge.busy || self.bridge.closing {
            egui::Panel::top("host_status").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if self.bridge.closing {
                        ui.label("Stopping owned agents…");
                    } else if self.bridge.busy {
                        ui.label("Applying action…");
                    }
                });
                if let Some(message) = &self.bridge.feedback {
                    ui.label(message);
                }
                if !self.bridge.busy && !self.bridge.closing {
                    if let Some(command) = self.bridge.pending.clone() {
                        if ui.button("Retry the same action").clicked() {
                            self.bridge.submit(command);
                        }
                    }
                }
            });
        }
        if let Some(error) = self.bridge.error.clone() {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("The local host needs attention");
                ui.label("Close another Patchfleet instance if it owns this data directory. Your local state is preserved.");
                ui.monospace(error);
                if !self.bridge.closing && ui.button("Retry opening the host").clicked() {
                    self.bridge.shutdown();
                    self.bridge = Bridge::new(ctx.clone(), self.directory.clone(), self.observe);
                }
            });
        } else if let Some(snapshot) = &self.bridge.snapshot {
            let commands = self
                .console
                .show(ui, snapshot, self.bridge.busy || self.bridge.closing);
            if let Some(command) = commands.into_iter().next() {
                self.bridge.submit(command);
            }
        } else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("Opening Patchfleet");
                ui.label("Loading the local Rust host…");
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
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Patchfleet")
            .with_app_id("com.patchfleet.desktop")
            .with_inner_size([1280.0, 880.0])
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
        "Patchfleet",
        options,
        Box::new(move |cc| Ok(Box::new(Patchfleet::new(cc, directory, observe)))),
    ) {
        eprintln!("{}", command::safe_error(error));
        std::process::exit(1);
    }
}
