use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct CanvasKey {
    pub project: String,
    pub work: String,
    pub path: String,
    pub hash: String,
    pub part: String,
}

#[derive(Clone, Default, PartialEq)]
pub(super) struct CanvasDraft {
    pub region: Option<egui::Rect>,
    pub range: Option<(f64, f64)>,
    pub note: String,
    anchor: Option<egui::Pos2>,
}

#[derive(Default)]
pub(super) struct CanvasState {
    drafts: HashMap<CanvasKey, CanvasDraft>,
}

impl CanvasState {
    pub(super) fn draft(&mut self, key: &CanvasKey) -> &mut CanvasDraft {
        self.drafts.entry(key.clone()).or_default()
    }
}

fn unit_point(position: egui::Pos2, image: egui::Rect) -> egui::Pos2 {
    egui::pos2(
        ((position.x - image.left()) / image.width()).clamp(0.0, 1.0),
        ((position.y - image.top()) / image.height()).clamp(0.0, 1.0),
    )
}

fn adjust_region(region: egui::Rect, delta: egui::Vec2, resize: bool) -> egui::Rect {
    if resize {
        return egui::Rect::from_min_max(
            region.min,
            egui::pos2(
                (region.max.x + delta.x).clamp(region.min.x + 0.001, 1.0),
                (region.max.y + delta.y).clamp(region.min.y + 0.001, 1.0),
            ),
        );
    }
    let min = egui::pos2(
        (region.min.x + delta.x).clamp(0.0, 1.0 - region.width()),
        (region.min.y + delta.y).clamp(0.0, 1.0 - region.height()),
    );
    egui::Rect::from_min_size(min, region.size())
}

pub(super) fn select_image(
    ui: &mut Ui,
    texture: &egui::TextureHandle,
    key: &CanvasKey,
    state: &mut CanvasState,
) -> egui::Response {
    ui.push_id(("artifact-canvas", key), |ui| {
        let response = ui.add(
            egui::Image::new(texture)
                .max_size(egui::vec2(ui.available_width(), 260.0))
                .sense(egui::Sense::click_and_drag()),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                response.enabled(),
                t(ui, "Select image region"),
            )
        });
        let draft = state.draft(key);
        if response.drag_started() {
            draft.anchor = ui
                .input(|input| input.pointer.press_origin())
                .map(|origin| unit_point(origin, response.rect));
            response.request_focus();
        }
        if response.dragged() || response.drag_stopped() {
            if let Some((start, end)) = draft.anchor.zip(response.interact_pointer_pos()) {
                let region = egui::Rect::from_two_pos(start, unit_point(end, response.rect));
                if region.width() > 0.001 && region.height() > 0.001 {
                    draft.region = Some(region);
                }
            }
        }
        if response.drag_stopped() {
            draft.anchor = None;
        }
        if response.has_focus() {
            ui.input(|input| {
                if input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace) {
                    draft.region = None;
                }
                let mut delta = egui::Vec2::ZERO;
                for (key, direction) in [
                    (egui::Key::ArrowLeft, egui::vec2(-0.01, 0.0)),
                    (egui::Key::ArrowRight, egui::vec2(0.01, 0.0)),
                    (egui::Key::ArrowUp, egui::vec2(0.0, -0.01)),
                    (egui::Key::ArrowDown, egui::vec2(0.0, 0.01)),
                ] {
                    if input.key_pressed(key) {
                        delta += direction;
                    }
                }
                if delta != egui::Vec2::ZERO {
                    let region = draft.region.unwrap_or_else(|| {
                        egui::Rect::from_min_max(egui::pos2(0.25, 0.25), egui::pos2(0.75, 0.75))
                    });
                    draft.region = Some(adjust_region(region, delta, input.modifiers.shift));
                }
            });
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect,
                0,
                egui::Stroke::new(2.0, colors(ui).primary),
                egui::StrokeKind::Inside,
            );
        }
        if let Some(region) = draft.region {
            let region = egui::Rect::from_min_max(
                response.rect.min + region.min.to_vec2() * response.rect.size(),
                response.rect.min + region.max.to_vec2() * response.rect.size(),
            );
            ui.painter().rect(
                region,
                0,
                colors(ui).primary.gamma_multiply(0.12),
                egui::Stroke::new(2.0, colors(ui).primary),
                egui::StrokeKind::Inside,
            );
        }
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("artifact-canvas-image"),
                (response.rect, response.id, ui.clip_rect()),
            )
        });
        response.on_hover_text(t(
            ui,
            "Drag to select a region. Arrow keys move it; Shift adjusts its size.",
        ))
    })
    .inner
}

pub(super) fn range_id(start: f64, end: f64, duration: f64) -> Option<String> {
    if !start.is_finite()
        || !end.is_finite()
        || !duration.is_finite()
        || start < 0.0
        || end > duration
    {
        return None;
    }
    let start = (start * 1000.0).round() / 1000.0;
    let end = (end * 1000.0).floor() / 1000.0;
    (end > start && end <= duration).then(|| format!("time:{start:.3}:{end:.3}"))
}
pub(super) fn region_id(region: egui::Rect) -> String {
    format!(
        "region:{:.6},{:.6},{:.6},{:.6}",
        region.min.x,
        region.min.y,
        region.width(),
        region.height()
    )
}
pub(super) fn matches_read(read: &Value, key: &CanvasKey, reason: &str) -> bool {
    text(read, "reasonCode") == reason
        && text(read, "projectId") == key.project
        && text(read, "workItemId") == key.work
        && text(read, "path") == key.path
        && text(read, "sha256") == key.hash
}
pub(super) fn texture(ui: &mut Ui, preview: &Value, key: &str) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("actual-native-preview", key));
    if let Some(texture) = ui.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let bytes: Option<Vec<u8>> = preview["pngBytes"]
        .as_array()
        .filter(|b| b.len() <= 8 * 1024 * 1024)?
        .iter()
        .map(|v| v.as_u64().filter(|v| *v <= 255).map(|v| v as u8))
        .collect();
    let bytes = bytes?;
    if bytes.len() < 24
        || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10]
        || &bytes[12..16] != b"IHDR"
    {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return None;
    }
    let image = eframe::icon_data::from_png_bytes(&bytes).ok()?;
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &image.rgba,
    );
    let texture = ui.ctx().load_texture(
        format!("actual-preview-{key}"),
        image,
        egui::TextureOptions::LINEAR,
    );
    ui.data_mut(|d| d.insert_temp(id, texture.clone()));
    Some(texture)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_normalization_and_keyboard_resize_never_cross_source_bounds() {
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(200.0, 100.0));
        assert_eq!(
            unit_point(egui::pos2(-5.0, 200.0), rect),
            egui::pos2(0.0, 1.0)
        );
        let source = egui::Rect::from_min_max(egui::pos2(0.1, 0.2), egui::pos2(0.4, 0.6));
        assert_eq!(
            region_id(source),
            "region:0.100000,0.200000,0.300000,0.400000"
        );
        assert_eq!(
            adjust_region(source, egui::vec2(2.0, 2.0), false).max,
            egui::pos2(1.0, 1.0)
        );
        assert_eq!(
            adjust_region(source, egui::vec2(2.0, 2.0), true).max,
            egui::pos2(1.0, 1.0)
        );
    }
    #[test]
    fn actual_canvas_pointer_and_keyboard_selection_are_hash_scoped() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut state = CanvasState::default();
        let key = CanvasKey {
            project: "project:a".into(),
            work: "work:a".into(),
            path: "outputs/a.png".into(),
            hash: "a".repeat(64),
            part: String::new(),
        };
        let mut texture = None;
        let mut draw = |events: Vec<egui::Event>, state: &mut CanvasState| {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 400.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let texture = texture.get_or_insert_with(|| {
                        ui.ctx().load_texture(
                            "real-test-pixels",
                            egui::ColorImage::filled([100, 100], Color32::BLUE),
                            egui::TextureOptions::LINEAR,
                        )
                    });
                    select_image(ui, texture, &key, state);
                },
            );
            output.drop_without_applying_deltas();
        };
        draw(vec![], &mut state);
        let (rect, id, _) = ctx.data(|d| {
            d.get_temp::<(egui::Rect, egui::Id, egui::Rect)>(egui::Id::new("artifact-canvas-image"))
                .unwrap()
        });
        let start = rect.min + rect.size() * 0.2;
        let end = rect.min + rect.size() * 0.7;
        draw(
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            &mut state,
        );
        draw(vec![egui::Event::PointerMoved(end)], &mut state);
        draw(
            vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            &mut state,
        );
        assert!(state.draft(&key).region.is_some());
        assert!(ctx.memory(|m| m.has_focus(id)));
        let old = state.draft(&key).region.unwrap();
        draw(
            vec![egui::Event::Key {
                key: egui::Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
            &mut state,
        );
        assert!(state.draft(&key).region.unwrap().min.x > old.min.x);
        let mut changed = key.clone();
        changed.hash = "b".repeat(64);
        assert!(state.draft(&changed).region.is_none());
    }
    #[test]
    fn stale_visual_cache_never_matches_another_file_page_or_project() {
        let key = CanvasKey {
            project: "project:a".into(),
            work: "work:a".into(),
            path: "outputs/a.pptx".into(),
            hash: "a".repeat(64),
            part: String::new(),
        };
        let mut read = json!({"reasonCode":"PROJECT_ARTIFACT_VISUAL_READ","projectId":key.project,"workItemId":key.work,"path":key.path,"sha256":key.hash,"page":1});
        assert!(matches_read(&read, &key, "PROJECT_ARTIFACT_VISUAL_READ"));
        read["sha256"] = json!("b".repeat(64));
        assert!(!matches_read(&read, &key, "PROJECT_ARTIFACT_VISUAL_READ"));
    }
    #[test]
    fn fractional_video_duration_clamps_to_a_valid_canonical_millisecond_range() {
        assert_eq!(
            range_id(0.0, 0.066667, 0.066667).as_deref(),
            Some("time:0.000:0.066")
        );
        assert!(range_id(0.0665, 0.066667, 0.066667).is_none());
        assert!(range_id(0.0, 0.067, 0.066667).is_none());
    }
}
