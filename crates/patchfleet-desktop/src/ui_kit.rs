//! Native component reference, excluded from the production build.
use crate::studio::{self, metrics as m, ButtonKind as B};
use eframe::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui) {
    let p = studio::palette(ui.ctx());
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(p.canvas)
                .inner_margin(m::PAGE_PADDING),
        )
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(m::SECTION_GAP, m::PADDING as f32);
            studio::ambient(ui);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(48.0, 40.0), egui::Sense::hover());
                studio::mark(ui.painter(), rect);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(studio::title("Moxcrew UI Kit", 32.0));
                    ui.label(
                        RichText::new("NATIVE / MANROPE / LIGHT + DARK")
                            .size(12.0)
                            .color(p.muted),
                    );
                });
            });
            ui.label(
                RichText::new("One rhythm. One set of controls.")
                    .size(18.0)
                    .color(p.muted),
            );
            studio::card(ui).show(ui, |ui| {
                section(
                    ui,
                    "CHIPS / CONTEXT, FILTERS, SUGGESTIONS",
                    "32 px targets · named removal · keyboard focus",
                );
                ui.horizontal_wrapped(|ui| {
                    studio::input_chip(ui, "Launch brief.pdf", "Remove Launch brief.pdf");
                    studio::input_chip(ui, "Page 3", "Clear selected page");
                    studio::filter_chip(ui, "Documents", true);
                    studio::filter_chip(ui, "Images", false);
                    studio::suggestion_chip(ui, "Research a topic");
                });
            });
            ui.columns(2, |columns| {
                studio::card(&columns[0]).show(&mut columns[0], |ui| {
                    section(ui, "01 / ACTIONS", "44 px standard · 36 px compact");
                    for (kind, label) in [
                        (B::Primary, "Prepare crew"),
                        (B::Secondary, "Export copy"),
                        (B::Ghost, "View details"),
                        (B::Danger, "Cancel run"),
                    ] {
                        ui.horizontal(|ui| {
                            studio::button(ui, label, kind, true);
                            studio::compact_button(ui, label, kind, true);
                        });
                    }
                    ui.horizontal(|ui| {
                        studio::button(ui, "Unavailable", B::Primary, false);
                        studio::button(ui, "Keyboard focus", B::Secondary, true).request_focus();
                    });
                });
                studio::card(&columns[1]).show(&mut columns[1], |ui| {
                    section(
                        ui,
                        "02 / INPUTS",
                        "12 px horizontal inset · 44 px single line",
                    );
                    ui.label("Project name");
                    let mut name = "Product launch".to_owned();
                    studio::input(ui, &mut name, 160, "Project name", false);
                    ui.label("Search outputs");
                    studio::input(
                        ui,
                        &mut String::new(),
                        200,
                        "Search by name or path…",
                        false,
                    );
                    ui.label("Private access token");
                    studio::input(ui, &mut "private-sample".to_owned(), 100, "", true);
                    ui.label(
                        RichText::new("Labels stay above fields. Help stays below.")
                            .size(12.0)
                            .color(p.muted),
                    );
                });
            });
            ui.columns(3, |columns| {
                studio::card(&columns[0]).show(&mut columns[0], |ui| {
                    section(ui, "03 / SPACING", "A 4 px rhythm, shared by every surface");
                    for size in [4, 8, 12, 16, 24, 32] {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{size:02} px"))
                                    .monospace()
                                    .size(12.0),
                            );
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(size as f32 * 3.0, 12.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 4, p.primary);
                        });
                    }
                });
                studio::card(&columns[1]).show(&mut columns[1], |ui| {
                    section(ui, "04 / TYPE", "Manrope · one readable hierarchy");
                    ui.label(studio::title("Project heading", 24.0));
                    ui.label(studio::title("Section heading", 20.0));
                    ui.label("14 px body and controls");
                    ui.label(
                        RichText::new("12 px metadata and supporting text")
                            .size(12.0)
                            .color(p.muted),
                    );
                    ui.separator();
                    ui.label("16 px card padding");
                    ui.label("24 px page padding");
                    ui.label("12 px controls / 16 px card radius");
                });
                studio::card(&columns[2]).show(&mut columns[2], |ui| {
                    section(ui, "05 / FEEDBACK", "Meaning comes from text and color");
                    for (label, foreground, background) in [
                        ("Ready to start", p.success, p.success_surface),
                        ("Needs your review", p.warning, p.warning_surface),
                        ("Connection unavailable", p.danger, p.danger_surface),
                        ("Local files stay on this device", p.info, p.info_surface),
                    ] {
                        egui::Frame::new()
                            .fill(background)
                            .corner_radius(m::CONTROL_RADIUS)
                            .inner_margin(12)
                            .show(ui, |ui| {
                                ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                                ui.label(RichText::new(label).size(13.0).color(foreground));
                            });
                    }
                });
            });
        });
}

fn section(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.spacing_mut().item_spacing = egui::vec2(m::GAP, m::CONTROL_GAP);
    ui.label(studio::title(title, 14.0));
    ui.label(
        RichText::new(description)
            .size(12.0)
            .color(studio::palette(ui.ctx()).muted),
    );
    ui.add_space(m::GAP);
}
