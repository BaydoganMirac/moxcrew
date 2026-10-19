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

pub mod metrics {
    use eframe::egui;
    pub const CONTROL_HEIGHT: f32 = 44.0;
    pub const COMPACT_HEIGHT: f32 = 36.0;
    pub const GAP: f32 = 8.0;
    pub const CONTROL_GAP: f32 = 12.0;
    pub const SECTION_GAP: f32 = 24.0;
    pub const PADDING: i8 = 16;
    pub const PAGE_PADDING: i8 = 24;
    pub const CONTROL_RADIUS: u8 = 12;
    pub const CARD_RADIUS: u8 = 16;
    pub const INPUT_MARGIN: egui::Margin = egui::Margin::symmetric(12, 10);
}

#[derive(Clone, Copy, Debug)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub fn button(ui: &mut egui::Ui, label: &str, kind: ButtonKind, enabled: bool) -> egui::Response {
    kit_button(ui, label, kind, metrics::CONTROL_HEIGHT, enabled)
}

pub fn compact_button(
    ui: &mut egui::Ui,
    label: &str,
    kind: ButtonKind,
    enabled: bool,
) -> egui::Response {
    kit_button(ui, label, kind, metrics::COMPACT_HEIGHT, enabled)
}

fn kit_button(
    ui: &mut egui::Ui,
    label: &str,
    kind: ButtonKind,
    height: f32,
    enabled: bool,
) -> egui::Response {
    let p = palette(ui.ctx());
    let previous = ui.style().clone();
    ui.spacing_mut().button_padding = egui::vec2(16.0, 8.0);
    let (fill, hover, pressed, foreground, border) = match kind {
        ButtonKind::Primary => (
            p.primary,
            p.primary.gamma_multiply(0.94),
            p.primary.gamma_multiply(0.86),
            p.on_primary,
            Color32::TRANSPARENT,
        ),
        ButtonKind::Secondary => (
            p.surface,
            p.raised,
            p.info_surface,
            p.text,
            p.control_border,
        ),
        ButtonKind::Ghost => (
            Color32::TRANSPARENT,
            p.raised,
            p.info_surface,
            p.muted,
            Color32::TRANSPARENT,
        ),
        ButtonKind::Danger => (
            p.danger_surface,
            p.danger_surface.gamma_multiply(0.94),
            p.danger_surface.gamma_multiply(0.86),
            p.danger,
            p.danger,
        ),
    };
    let widgets = &mut ui.visuals_mut().widgets;
    for (widget, background) in [
        (&mut widgets.inactive, fill),
        (&mut widgets.hovered, hover),
        (&mut widgets.active, pressed),
        (&mut widgets.open, pressed),
    ] {
        widget.bg_fill = background;
        widget.weak_bg_fill = background;
        widget.bg_stroke = egui::Stroke::new(1.0, border);
        widget.fg_stroke = egui::Stroke::new(1.0, foreground);
        widget.corner_radius = egui::CornerRadius::same(metrics::CONTROL_RADIUS);
    }
    widgets.hovered.bg_stroke.color = if matches!(kind, ButtonKind::Ghost) {
        p.border
    } else {
        border
    };
    widgets.noninteractive.bg_fill = p.raised;
    widgets.noninteractive.weak_bg_fill = p.raised;
    widgets.noninteractive.fg_stroke.color = p.muted;
    // Add directly to the parent so wrapped rows measure the whole button.
    let response =
        ui.add_enabled(
            enabled,
            egui::Button::new(RichText::new(label).size(14.0).color(
                if enabled && ui.is_enabled() {
                    foreground
                } else {
                    p.muted
                },
            ))
            .wrap_mode(egui::TextWrapMode::Extend)
            .min_size(egui::vec2(0.0, height)),
        );
    ui.set_style(previous);
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            metrics::CONTROL_RADIUS,
            egui::Stroke::new(2.0, p.focus),
            egui::StrokeKind::Inside,
        );
    }
    response
}

pub fn input(
    ui: &mut egui::Ui,
    value: &mut String,
    max: usize,
    hint: &str,
    password: bool,
) -> egui::Response {
    ui.add_sized(
        egui::vec2(ui.available_width(), metrics::CONTROL_HEIGHT),
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Body)
            .margin(metrics::INPUT_MARGIN)
            .char_limit(max)
            .hint_text(hint)
            .password(password),
    )
}

pub fn card(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(palette(ui.ctx()).surface)
        .stroke(egui::Stroke::new(1.0, palette(ui.ctx()).border))
        .corner_radius(metrics::CARD_RADIUS)
        .inner_margin(metrics::PADDING)
}

pub fn palette(ctx: &egui::Context) -> Palette {
    match ctx.theme() {
        egui::Theme::Light => LIGHT,
        egui::Theme::Dark => DARK,
    }
}

pub fn title(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(FontId::new(size, FontFamily::Name("studio-display".into())))
}

pub fn section_nav(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let response = ui.add(
        egui::Button::new(RichText::new(label).size(13.0).color(if selected {
            p.text
        } else {
            p.muted
        }))
        .frame(false)
        .min_size(egui::vec2(0.0, 32.0)),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            response.enabled(),
            selected,
            label,
        )
    });
    if selected {
        ui.painter().line_segment(
            [
                response.rect.left_bottom() + egui::vec2(8.0, 0.0),
                response.rect.right_bottom() - egui::vec2(8.0, 0.0),
            ],
            egui::Stroke::new(2.0, p.primary),
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            5,
            egui::Stroke::new(2.0, p.focus),
            egui::StrokeKind::Inside,
        );
    }
    response
}

pub fn quiet_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    compact_button(ui, label, ButtonKind::Ghost, true)
}

pub fn view_switch(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let response = ui.add(
        egui::Button::new(RichText::new(label).size(12.0).color(if selected {
            p.text
        } else {
            p.muted
        }))
        .fill(if selected {
            p.raised
        } else {
            Color32::TRANSPARENT
        })
        .stroke(egui::Stroke::NONE)
        .corner_radius(7)
        .min_size(egui::vec2(66.0, metrics::COMPACT_HEIGHT)),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            response.enabled(),
            selected,
            label,
        )
    });
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            7,
            egui::Stroke::new(2.0, p.focus),
            egui::StrokeKind::Inside,
        );
    }
    response
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
    style.spacing.item_spacing = egui::vec2(metrics::CONTROL_GAP, metrics::GAP);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.interact_size.y = metrics::COMPACT_HEIGHT;
    style.spacing.window_margin = egui::Margin::same(metrics::PADDING);
    style.spacing.menu_margin = egui::Margin::same(metrics::GAP as i8);
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
    style.visuals.window_corner_radius = egui::CornerRadius::same(metrics::CARD_RADIUS);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(metrics::CONTROL_RADIUS);
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
        "review" => {
            painter.circle_stroke(p(12.0, 12.0), size * 0.4, stroke);
            line((7.0, 12.0), (10.5, 15.5));
            line((10.5, 15.5), (17.0, 8.0));
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
    fn button_semantic_foregrounds_survive_the_global_body_text_color() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let ctx = egui::Context::default();
            install(&ctx);
            ctx.set_theme(theme);
            let p = palette(&ctx);
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                button(ui, "Primary", ButtonKind::Primary, true);
                button(ui, "Cancel run", ButtonKind::Danger, true);
            });
            for (label, expected) in [("Primary", p.on_primary), ("Cancel run", p.danger)] {
                let color = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.job.text == label => {
                            Some(text.galley.job.sections[0].format.color)
                        }
                        _ => None,
                    })
                    .expect("Button label is painted");
                assert_eq!(color, expected, "{theme:?}/{label}");
            }
            output.drop_without_applying_deltas();
        }
    }
    #[test]
    fn kit_metrics_match_the_brand_contract_and_controls_keep_one_height() {
        let contract: serde_json::Value = serde_json::from_str(include_str!(
            "../../../design/moxcrew/tokens/design-tokens.json"
        ))
        .unwrap();
        assert_eq!(
            contract["control"]["defaultHeight"].as_f64().unwrap(),
            metrics::CONTROL_HEIGHT as f64
        );
        assert_eq!(
            contract["control"]["compactHeight"].as_f64().unwrap(),
            metrics::COMPACT_HEIGHT as f64
        );
        assert_eq!(contract["radius"]["control"], metrics::CONTROL_RADIUS);
        assert_eq!(contract["radius"]["card"], metrics::CARD_RADIUS);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let ctx = egui::Context::default();
            install(&ctx);
            ctx.set_theme(theme);
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 560.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    for kind in [
                        ButtonKind::Primary,
                        ButtonKind::Secondary,
                        ButtonKind::Ghost,
                        ButtonKind::Danger,
                    ] {
                        ui.horizontal(|ui| {
                            let normal = button(ui, "Action", kind, true);
                            let compact = compact_button(ui, "Action", kind, true);
                            let disabled = button(ui, "Unavailable", kind, false);
                            assert_eq!(
                                normal.rect.height(),
                                metrics::CONTROL_HEIGHT,
                                "{theme:?}/{kind:?}"
                            );
                            assert_eq!(compact.rect.height(), metrics::COMPACT_HEIGHT);
                            assert!(!disabled.enabled() && !disabled.clicked());
                        });
                    }
                    let mut value = String::new();
                    let field = input(ui, &mut value, 160, "Project name", false);
                    assert_eq!(field.rect.height(), metrics::CONTROL_HEIGHT);
                    let frame = card(ui);
                    assert_eq!(frame.inner_margin, egui::Margin::same(metrics::PADDING));
                    assert_eq!(
                        frame.corner_radius,
                        egui::CornerRadius::same(metrics::CARD_RADIUS)
                    );
                },
            )
            .drop_without_applying_deltas();
        }
    }
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
