use eframe::egui::{self, Color32, FontFamily, FontId, RichText};

pub const CANVAS: Color32 = Color32::from_rgb(16, 19, 21);
pub const SURFACE: Color32 = Color32::from_rgb(25, 30, 33);
pub const SIDEBAR: Color32 = Color32::from_rgb(20, 24, 26);
pub const TEXT: Color32 = Color32::from_rgb(238, 240, 233);
pub const MUTED: Color32 = Color32::from_rgb(156, 168, 165);
pub const MINT: Color32 = Color32::from_rgb(182, 237, 202);
pub const BORDER: Color32 = Color32::from_rgb(45, 53, 55);
pub const RED: Color32 = Color32::from_rgb(246, 157, 152);

pub fn title(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text)
        .font(FontId::new(size, FontFamily::Name("studio-display".into())))
        .color(TEXT)
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
    // Keep egui's bundled fallback for symbols outside the small Latin/TR font.
    let mut display = fonts.families[&FontFamily::Proportional].clone();
    display.insert(0, "studio-display".into());
    fonts
        .families
        .insert(FontFamily::Name("studio-display".into()), display);
    ctx.set_fonts(fonts);
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(10.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.interact_size.y = 34.0;
    for (kind, size, family) in [
        (egui::TextStyle::Body, 13.0, FontFamily::Proportional),
        (
            egui::TextStyle::Button,
            12.0,
            FontFamily::Name("studio-display".into()),
        ),
        (egui::TextStyle::Small, 11.0, FontFamily::Proportional),
        (
            egui::TextStyle::Heading,
            22.0,
            FontFamily::Name("studio-display".into()),
        ),
    ] {
        style.text_styles.insert(kind, FontId::new(size, family));
    }
    // Short hover/focus transitions repaint only while a widget changes.
    style.animation_time = 0.12;
    style.scroll_animation = egui::style::ScrollAnimation::none();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.weak_text_color = Some(MUTED);
    style.visuals.panel_fill = CANVAS;
    style.visuals.window_fill = SURFACE;
    style.visuals.text_edit_bg_color = Some(CANVAS);
    style.visuals.selection.bg_fill = Color32::from_rgb(43, 70, 57);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, MINT);
    style.visuals.hyperlink_color = MINT;
    style.visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
    style.visuals.window_corner_radius = egui::CornerRadius::same(14);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(10);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.bg_fill = Color32::from_rgb(34, 41, 43);
        widget.weak_bg_fill = SURFACE;
        widget.bg_stroke = egui::Stroke::new(1.0, BORDER);
        widget.fg_stroke = egui::Stroke::new(1.0, TEXT);
        widget.corner_radius = egui::CornerRadius::same(8);
    }
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(43, 53, 51);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(35, 44, 42);
    style.visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(95, 121, 109));
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(43, 70, 57);
    ctx.set_global_style(style);
}

pub fn mark(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
    let point = |x, y| rect.min + egui::vec2(rect.width() * x / 30.0, rect.height() * y / 28.0);
    let width = rect.width() * 4.0 / 30.0;
    for x in [3.0, 11.0, 19.0] {
        let (a, b) = (point(x, 23.0), point(x + 8.0, 5.0));
        painter.line_segment([a, b], egui::Stroke::new(width, color));
        painter.circle_filled(a, width * 0.5, color);
        painter.circle_filled(b, width * 0.5, color);
    }
}
