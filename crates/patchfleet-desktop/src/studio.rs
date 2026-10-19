use eframe::egui::{self, Color32, FontFamily, FontId, RichText};

/// Moxcrew v1 semantic roles, shared with design/moxcrew/tokens/design-tokens.json.
#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub control_border: Color32,
    pub primary: Color32,
    pub on_primary: Color32,
    pub focus: Color32,
    pub success: Color32,
    pub success_surface: Color32,
    pub warning: Color32,
    pub warning_surface: Color32,
    pub danger: Color32,
    pub danger_surface: Color32,
    pub info: Color32,
    pub info_surface: Color32,
}
include!("../assets/brand/palette.rs");

pub fn palette(ctx: &egui::Context) -> Palette {
    match ctx.theme() {
        egui::Theme::Light => LIGHT,
        egui::Theme::Dark => DARK,
    }
}

pub fn title(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(FontId::new(size, FontFamily::Name("studio-display".into())))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, bytes) in [
        (
            "studio-body",
            include_bytes!("../assets/fonts/Manrope-Regular.ttf").as_slice(),
        ),
        (
            "studio-display",
            include_bytes!("../assets/fonts/Manrope-Semibold.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), egui::FontData::from_static(bytes).into());
    }
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "studio-body".into());
    let mut display = fonts.families[&FontFamily::Proportional].clone();
    display.insert(0, "studio-display".into());
    fonts
        .families
        .insert(FontFamily::Name("studio-display".into()), display);
    ctx.set_fonts(fonts);
    // Install once. egui selects these styles when the system preference changes.
    ctx.set_style_of(egui::Theme::Light, style(LIGHT, false));
    ctx.set_style_of(egui::Theme::Dark, style(DARK, true));
    ctx.options_mut(|options| {
        options.theme_preference = egui::ThemePreference::System;
        options.fallback_theme = egui::Theme::Light;
    });
}

fn style(p: Palette, dark: bool) -> egui::Style {
    let mut style = egui::Style {
        visuals: if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        },
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(12.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.interact_size.y = 36.0;
    for (kind, size, family) in [
        (egui::TextStyle::Body, 14.0, FontFamily::Proportional),
        (
            egui::TextStyle::Button,
            14.0,
            FontFamily::Name("studio-display".into()),
        ),
        (egui::TextStyle::Small, 12.0, FontFamily::Proportional),
        (
            egui::TextStyle::Heading,
            24.0,
            FontFamily::Name("studio-display".into()),
        ),
    ] {
        style.text_styles.insert(kind, FontId::new(size, family));
    }
    // No reduced-motion signal is available consistently on every native platform.
    // Immediate feedback avoids animation and an idle repaint loop altogether.
    style.animation_time = 0.0;
    style.scroll_animation = egui::style::ScrollAnimation::none();
    style.visuals.override_text_color = Some(p.text);
    style.visuals.weak_text_color = Some(p.muted);
    style.visuals.panel_fill = p.canvas;
    style.visuals.window_fill = p.surface;
    style.visuals.extreme_bg_color = p.canvas;
    style.visuals.faint_bg_color = p.raised;
    style.visuals.code_bg_color = p.raised;
    style.visuals.text_edit_bg_color = Some(p.surface);
    style.visuals.selection.bg_fill = p.info_surface;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, p.primary);
    style.visuals.hyperlink_color = p.primary;
    style.visuals.warn_fg_color = p.warning;
    style.visuals.error_fg_color = p.danger;
    style.visuals.window_stroke = egui::Stroke::new(1.0, p.border);
    style.visuals.window_corner_radius = egui::CornerRadius::same(20);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(12);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.text);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.bg_fill = p.surface;
        widget.weak_bg_fill = p.surface;
        widget.bg_stroke = egui::Stroke::new(1.0, p.control_border);
        widget.fg_stroke = egui::Stroke::new(1.0, p.text);
        widget.corner_radius = egui::CornerRadius::same(12);
    }
    style.visuals.widgets.hovered.bg_fill = p.raised;
    style.visuals.widgets.hovered.weak_bg_fill = p.raised;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.primary);
    style.visuals.widgets.active.bg_fill = p.info_surface;
    style.visuals.widgets.active.weak_bg_fill = p.info_surface;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, p.focus);
    style.visuals.widgets.open = style.visuals.widgets.active;
    style
}

/// Static light gives the canvas depth without waking an idle window.
pub fn ambient(ui: &egui::Ui) {
    let rect = ui.max_rect();
    let dark = ui.ctx().theme() == egui::Theme::Dark;
    let center = egui::pos2(rect.center().x, rect.top() + rect.height() * 0.27);
    let radius = rect.width().min(1100.0) * 0.58;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(
        center,
        Color32::from_rgba_unmultiplied(65, 90, 230, if dark { 27 } else { 12 }),
    );
    for i in 0..=64 {
        let angle = i as f32 / 64.0 * std::f32::consts::TAU;
        mesh.colored_vertex(
            center + egui::vec2(angle.cos() * radius, angle.sin() * radius * 0.8),
            Color32::TRANSPARENT,
        );
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// Small native line icons share one stroke and never depend on emoji fonts.
pub fn icon(ui: &mut egui::Ui, key: &str, size: f32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    let p = |x: f32, y: f32| rect.min + egui::vec2(x, y) * (size / 24.0);
    let stroke = egui::Stroke::new((size / 15.0).max(1.2), color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    let outline = |a: (f32, f32), b: (f32, f32)| {
        painter.rect_stroke(
            egui::Rect::from_min_max(p(a.0, a.1), p(b.0, b.1)),
            3,
            stroke,
            egui::StrokeKind::Inside,
        );
    };
    match key {
        "presentations" => {
            outline((3.0, 3.0), (21.0, 16.0));
            line((12.0, 16.0), (12.0, 21.0));
            line((7.0, 22.0), (12.0, 19.0));
            line((12.0, 19.0), (17.0, 22.0));
            line((7.0, 12.0), (10.0, 8.0));
            line((10.0, 8.0), (13.0, 11.0));
            line((13.0, 11.0), (17.0, 6.0));
        }
        "video" => {
            outline((2.0, 4.0), (22.0, 20.0));
            painter.add(egui::Shape::closed_line(
                vec![p(10.0, 8.0), p(16.0, 12.0), p(10.0, 16.0)],
                stroke,
            ));
        }
        "images" => {
            outline((2.0, 3.0), (22.0, 21.0));
            painter.circle_stroke(p(16.0, 8.0), size * 0.08, stroke);
            line((3.0, 18.0), (9.0, 11.0));
            line((9.0, 11.0), (14.0, 16.0));
            line((14.0, 16.0), (17.0, 13.0));
            line((17.0, 13.0), (21.0, 18.0));
        }
        "audio" => {
            for (x, h) in [
                (3.0, 3.0),
                (7.5, 6.0),
                (12.0, 9.0),
                (16.5, 5.0),
                (21.0, 2.5),
            ] {
                line((x, 12.0 - h), (x, 12.0 + h));
            }
        }
        "research" => {
            painter.circle_stroke(p(10.0, 10.0), size * 0.29, stroke);
            line((15.0, 15.0), (21.0, 21.0));
            line((7.0, 10.0), (13.0, 10.0));
            line((10.0, 7.0), (10.0, 13.0));
        }
        "documents" | "files" => {
            outline((5.0, 2.0), (20.0, 22.0));
            for y in [7.0, 12.0, 17.0] {
                line((9.0, y), (16.0, y));
            }
        }
        "data" => {
            for (x, h) in [(4.0, 8.0), (10.0, 15.0), (16.0, 11.0)] {
                outline((x, 21.0 - h), (x + 4.0, 21.0));
            }
        }
        "web" => {
            painter.circle_stroke(p(12.0, 12.0), size * 0.4, stroke);
            line((3.0, 12.0), (21.0, 12.0));
            painter.add(egui::Shape::closed_line(
                (0..32)
                    .map(|i| {
                        let a = i as f32 / 32.0 * std::f32::consts::TAU;
                        p(12.0 + a.cos() * 4.0, 12.0 + a.sin() * 9.0)
                    })
                    .collect(),
                stroke,
            ));
        }
        "connections" => {
            for (x, y) in [(5.0, 6.0), (19.0, 6.0), (12.0, 19.0)] {
                painter.circle_stroke(p(x, y), size * 0.12, stroke);
            }
            line((8.0, 6.0), (16.0, 6.0));
            line((6.5, 9.0), (10.5, 16.0));
            line((17.5, 9.0), (13.5, 16.0));
        }
        "refresh" => {
            painter.add(egui::Shape::line(
                (0..28)
                    .map(|i| {
                        let a = 0.6 + i as f32 / 27.0 * 5.0;
                        p(12.0 + a.cos() * 8.0, 12.0 + a.sin() * 8.0)
                    })
                    .collect(),
                stroke,
            ));
            line((19.0, 3.0), (20.0, 9.0));
            line((20.0, 9.0), (14.0, 8.0));
        }
        _ => {
            painter.circle_stroke(rect.center(), size * 0.3, stroke);
        }
    }
    response
}

/// Mox's flat face for small chrome. Geometry follows the canonical vector mark.
pub fn mark(painter: &egui::Painter, rect: egui::Rect) {
    let p = |x, y| rect.min + egui::vec2(rect.width() * x / 256.0, rect.height() * y / 256.0);
    let s = rect.width() / 256.0;
    let blue = Color32::from_rgb(40, 84, 244);
    let orange = Color32::from_rgb(255, 122, 47);
    let cream = Color32::from_rgb(255, 248, 231);
    let ink = Color32::from_rgb(21, 34, 61);
    let fill = |x, y, w, h, radius: u8, color| {
        painter.rect_filled(
            egui::Rect::from_min_size(p(x, y), egui::vec2(w * s, h * s)),
            (radius as f32 * s).round() as u8,
            color,
        );
    };
    fill(12.0, 125.0, 32.0, 58.0, 16, blue);
    fill(212.0, 125.0, 32.0, 58.0, 16, blue);
    // A tilted capsule keeps the tab recognizable down to the navigation size.
    let rotation = egui::emath::Rot2::from_angle(13.0_f32.to_radians());
    let rotate = |x, y| p(156.0, 83.0) + rotation * (p(x, y) - p(156.0, 83.0));
    painter.line_segment(
        [rotate(157.0, 34.0), rotate(157.0, 68.0)],
        egui::Stroke::new(42.0 * s, orange),
    );
    painter.circle_filled(rotate(157.0, 34.0), 10.0 * s, ink);
    fill(124.0, 72.0, 64.0, 20.0, 7, ink);
    fill(28.0, 84.0, 200.0, 144.0, 40, blue);
    for (x, y, r, color) in [
        (90.0, 143.0, 35.0, cream),
        (85.0, 143.0, 14.0, ink),
        (179.0, 153.0, 29.0, orange),
        (179.0, 153.0, 18.0, cream),
        (176.0, 153.0, 8.0, ink),
    ] {
        painter.circle_filled(p(x, y), r * s, color);
    }
    painter.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
        [p(126.0, 182.0), p(138.0, 192.0), p(150.0, 180.0)],
        false,
        Color32::TRANSPARENT,
        egui::Stroke::new(7.0 * s, ink),
    ));
}

/// Decode and upload once. This embedded PNG never reads disk or fetches a URL.
pub fn mascot(ui: &mut egui::Ui, width: f32) {
    let id = egui::Id::new("moxcrew-mascot-texture");
    let cached = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = cached.unwrap_or_else(|| {
        let icon =
            eframe::icon_data::from_png_bytes(include_bytes!("../assets/brand/mox-master.png"))
                .expect("Bundled Mox mascot");
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        );
        let texture = ui
            .ctx()
            .load_texture("Mox", image, egui::TextureOptions::LINEAR);
        ui.ctx()
            .data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    });
    ui.add(
        egui::Image::new(&texture)
            .fit_to_exact_size(egui::vec2(width, width * 2.0 / 3.0))
            .alt_text("Mox, the Moxcrew helper"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_theme_switches_semantic_styles_with_light_fallback() {
        let ctx = egui::Context::default();
        install(&ctx);
        assert_eq!(ctx.theme(), egui::Theme::Light);
        for (theme, p) in [(egui::Theme::Light, LIGHT), (egui::Theme::Dark, DARK)] {
            let output = ctx.run_ui(
                egui::RawInput {
                    system_theme: Some(theme),
                    ..Default::default()
                },
                |_| {},
            );
            output.drop_without_applying_deltas();
            assert_eq!(ctx.theme(), theme);
            assert_eq!(palette(&ctx).text, p.text);
            assert_eq!(ctx.global_style().visuals.panel_fill, p.canvas);
            assert_eq!(ctx.global_style().animation_time, 0.0);
        }
    }
}
