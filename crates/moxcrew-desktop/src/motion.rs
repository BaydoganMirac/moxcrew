use crate::studio;
use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui};
use std::time::Duration;

pub fn mox(ui: &mut Ui, size: f32, active: bool, reduced: bool) -> Response {
    draw_mox(ui, size, active, reduced, 0.0)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MoxCue {
    Thinking,
    Answer,
    PlanReady,
    Idle,
}

/// Only validated local outcomes choose a reaction. `id` is stable for one
/// result so repeated egui frames cannot restart its short animation.
pub fn semantic_mox(ui: &mut Ui, size: f32, cue: MoxCue, reduced: bool, id: egui::Id) -> Response {
    let time = ui.input(|input| input.time);
    let (previous, since) = ui
        .ctx()
        .data(|data| data.get_temp::<(MoxCue, f64)>(id))
        .unwrap_or((cue, time));
    let since = if previous == cue { since } else { time };
    ui.ctx().data_mut(|data| data.insert_temp(id, (cue, since)));
    let duration = match cue {
        MoxCue::Answer => 1.25,
        MoxCue::PlanReady => 1.55,
        _ => 0.0,
    };
    let elapsed = (time - since).max(0.0) as f32;
    let reacting = !reduced && ui.is_visible() && elapsed < duration as f32;
    let progress = if duration > 0.0 {
        (elapsed / duration as f32).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let lift = if reacting {
        let wave = (progress * std::f32::consts::PI).sin();
        match cue {
            MoxCue::Answer => -size * 0.045 * wave,
            MoxCue::PlanReady => -size * 0.08 * wave,
            _ => 0.0,
        }
    } else {
        0.0
    };
    let response = draw_mox(ui, size, cue == MoxCue::Thinking, reduced, lift);
    if reacting && ui.is_rect_visible(response.rect) {
        let p = studio::palette(ui.ctx());
        let center = response.rect.center();
        let fade = 1.0 - progress;
        match cue {
            MoxCue::Answer => {
                for (x, y, radius) in [(-0.40, -0.27, 2.4), (0.43, -0.14, 2.0)] {
                    ui.painter().circle_filled(
                        center + egui::vec2(x * size, (y - 0.12 * progress) * size),
                        radius + progress * 1.5,
                        p.accent.gamma_multiply(fade),
                    );
                }
            }
            MoxCue::PlanReady => {
                let ring = size * (0.32 + 0.16 * progress);
                ui.painter().circle_stroke(
                    center,
                    ring,
                    Stroke::new(1.5, p.primary.gamma_multiply(fade * 0.65)),
                );
                for angle in [-2.2_f32, -0.9, 0.2] {
                    ui.painter().circle_filled(
                        center + egui::vec2(angle.cos(), angle.sin()) * ring,
                        2.4,
                        p.accent.gamma_multiply(fade),
                    );
                }
            }
            _ => {}
        }
        ui.ctx().request_repaint_after(Duration::from_millis(42));
    }
    response
}

fn draw_mox(ui: &mut Ui, size: f32, active: bool, reduced: bool, lift: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size * 0.8), Sense::hover());
    let time = ui.input(|input| input.time);
    let moving = active && !reduced && ui.is_rect_visible(rect);
    let phase = if moving { time as f32 * 2.4 } else { 0.0 };
    let center = rect.center();
    let p = studio::palette(ui.ctx());
    if size >= 96.0 {
        // A static orbital stage makes Mox prominent even when no job is running.
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(center, p.primary.gamma_multiply(0.18));
        for i in 0..=48 {
            let angle = i as f32 / 48.0 * std::f32::consts::TAU;
            mesh.colored_vertex(
                center + egui::vec2(angle.cos() * size * 0.6, angle.sin() * size * 0.43),
                Color32::TRANSPARENT,
            );
            if i > 0 {
                mesh.add_triangle(0, i, i + 1);
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        for (radius, opacity) in [(0.48, 0.35), (0.60, 0.15)] {
            let points = (0..64)
                .map(|i| {
                    let angle = i as f32 / 64.0 * std::f32::consts::TAU;
                    center
                        + egui::vec2(
                            angle.cos() * size * radius,
                            angle.sin() * size * radius * 0.58,
                        )
                })
                .collect();
            ui.painter().add(egui::Shape::closed_line(
                points,
                Stroke::new(1.0, p.primary.gamma_multiply(opacity)),
            ));
        }
        for (x, y, radius) in [(-0.45, -0.16, 2.3), (0.44, 0.18, 3.0), (0.18, -0.30, 1.8)] {
            ui.painter().circle_filled(
                center + egui::vec2(x * size, y * size),
                radius,
                p.primary.gamma_multiply(0.65),
            );
        }
    }
    if active {
        ui.painter()
            .circle_stroke(center, size * 0.34, Stroke::new(1.0, p.border));
        for index in 0..3 {
            let angle = phase + index as f32 * std::f32::consts::TAU / 3.0;
            let point = Pos2::new(
                center.x + angle.cos() * size * 0.40,
                center.y + angle.sin() * size * 0.32,
            );
            ui.painter().circle_filled(
                point,
                (size * 0.035).max(1.5),
                if index == 0 {
                    p.primary
                } else {
                    Color32::from_rgba_unmultiplied(
                        p.primary.r(),
                        p.primary.g(),
                        p.primary.b(),
                        110,
                    )
                },
            );
        }
    }
    let bob = if moving { phase.sin() * 1.5 } else { 0.0 };
    let image = Rect::from_center_size(
        center + egui::vec2(0.0, bob + lift),
        egui::vec2(size * 0.82, size * 0.82 * 2.0 / 3.0),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(image), |ui| {
        studio::mascot(ui, size * 0.82)
    });
    if moving {
        ui.ctx().request_repaint_after(Duration::from_millis(42));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_visible_active_unreduced_motion_schedules_a_frame() {
        for (active, reduced, visible) in [
            (false, false, true),
            (false, true, true),
            (true, true, true),
            (true, false, true),
            (true, false, false),
        ] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut delay = Duration::ZERO;
            for _ in 0..3 {
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(400.0, 300.0),
                        )),
                        time: Some(2.0),
                        ..Default::default()
                    },
                    |ui| {
                        if !visible {
                            ui.set_clip_rect(Rect::NOTHING);
                        }
                        mox(ui, 64.0, active, reduced);
                    },
                );
                delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
                output.drop_without_applying_deltas();
            }
            if active && !reduced && visible {
                assert!(delay <= Duration::from_millis(42));
            } else {
                assert!(
                    delay > Duration::from_millis(42),
                    "idle/reduced motion must not wake rendering"
                );
            }
        }
    }

    #[test]
    fn semantic_reaction_is_brief_and_respects_reduced_motion() {
        for (cue, reduced, time, expect_frame) in [
            (MoxCue::Answer, false, 1.0, true),
            (MoxCue::PlanReady, false, 1.0, true),
            (MoxCue::Answer, true, 1.0, false),
            (MoxCue::Idle, false, 1.0, false),
        ] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut delay = Duration::ZERO;
            for _ in 0..3 {
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(400.0, 300.0),
                        )),
                        time: Some(time),
                        ..Default::default()
                    },
                    |ui| {
                        semantic_mox(ui, 48.0, cue, reduced, egui::Id::new("reaction"));
                    },
                );
                delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
                output.drop_without_applying_deltas();
            }
            assert_eq!(delay <= Duration::from_millis(42), expect_frame);
            if expect_frame {
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(400.0, 300.0),
                        )),
                        time: Some(4.0),
                        ..Default::default()
                    },
                    |ui| {
                        semantic_mox(ui, 48.0, cue, reduced, egui::Id::new("reaction"));
                    },
                );
                assert!(
                    output.viewport_output[&egui::ViewportId::ROOT].repaint_delay
                        > Duration::from_millis(42),
                    "completed reaction must stop repainting"
                );
                output.drop_without_applying_deltas();
            }
        }
    }
}
