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
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
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
    style.visuals.window_corner_radius = egui::CornerRadius::same(16);
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
