use crate::studio;
use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui};
use std::time::Duration;

pub fn mox(ui: &mut Ui, size: f32, active: bool, reduced: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size * 0.8), Sense::hover());
    let time = ui.input(|input| input.time);
    let moving = active && !reduced && ui.is_rect_visible(rect);
    let phase = if moving { time as f32 * 2.4 } else { 0.0 };
    let center = rect.center();
    let p = studio::palette(ui.ctx());
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
        center + egui::vec2(0.0, bob),
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
}
