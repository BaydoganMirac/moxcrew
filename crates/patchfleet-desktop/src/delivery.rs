use super::*;

const FILES_MAX: usize = 32;
const TOTAL_MAX: u64 = 64 * 1024 * 1024;
const FILE_MAX: u64 = 25 * 1024 * 1024;
const TYPES: [&str; 8] = [
    "All files",
    "Documents",
    "Presentations",
    "Images",
    "Video",
    "Audio",
    "Data",
    "Other files",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryFile {
    pub work_item_id: String,
    pub path: String,
    pub expected_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryExportRequest {
    pub project_id: String,
    pub name: String,
    pub files: Vec<DeliveryFile>,
    pub host_epoch: Value,
}

fn file_type(artifact: &Value) -> usize {
    let path = std::path::Path::new(text(artifact, "path"));
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "md" | "txt" | "pdf" | "docx" => 1,
        "pptx" => 2,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" => 3,
        "mp4" | "mov" | "mkv" | "webm" => 4,
        "wav" | "mp3" | "m4a" | "ogg" | "flac" => 5,
        "csv" | "xlsx" | "xls" | "json" => 6,
        _ => 7,
    }
}

pub(super) fn matches_file(artifact: &Value, query: &str, kind: usize) -> bool {
    let query = query.trim().to_lowercase();
    (kind == 0 || file_type(artifact) == kind)
        && (query.is_empty()
            || text(artifact, "name").to_lowercase().contains(&query)
            || text(artifact, "path").to_lowercase().contains(&query))
}

pub(super) fn file_filters(ui: &mut Ui, state: &mut StudioState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().interact_size.y = studio::metrics::CONTROL_HEIGHT;
        let width = (ui.available_width() - 160.0).max(80.0);
        let query = ui.add_sized(
            egui::vec2(width, studio::metrics::CONTROL_HEIGHT),
            egui::TextEdit::singleline(&mut state.query)
                .id_salt("output-file-query")
                .hint_text(t(ui, "Search outputs…"))
                .char_limit(200)
                .margin(studio::metrics::INPUT_MARGIN)
                .desired_width(width),
        );
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(egui::Id::new("delivery-search"), (query.rect, query.id));
        });
        #[cfg(not(test))]
        let _ = query;
        let picker = egui::ComboBox::from_id_salt("output-file-type")
            .width(140.0)
            .height(360.0)
            .selected_text(t(ui, TYPES[state.file_type.min(TYPES.len() - 1)]))
            .show_ui(ui, |ui| {
                ui.spacing_mut().interact_size.y = studio::metrics::COMPACT_HEIGHT;
                for (index, label) in TYPES.iter().enumerate() {
                    let response = ui.selectable_value(&mut state.file_type, index, t(ui, label));
                    if response.clicked() {
                        ui.close();
                    }
                    #[cfg(test)]
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new(("delivery-type-option", index)),
                            (response.id, response.rect, ui.clip_rect()),
                        )
                    });
                    #[cfg(not(test))]
                    let _ = response;
                }
            });
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("test-action", "memory_control", "File type")),
                picker.response.rect,
            );
            data.insert_temp(
                egui::Id::new(("test-action", "memory_control", "Search outputs")),
                query.rect,
            );
        });
        #[cfg(not(test))]
        let _ = picker;
    });
}

fn selected_files(state: &CreateState, snapshot: &Value) -> Option<(Vec<DeliveryFile>, u64)> {
    let selected = state.studio.packages.get(&state.project)?;
    if selected.is_empty() || selected.len() > FILES_MAX {
        return None;
    }
    let page = studio_page(snapshot, &state.project)?;
    let mut total = 0u64;
    let mut files = Vec::new();
    for key in selected {
        let file = array(&page["artifacts"])
            .iter()
            .find(|file| artifact_key(file) == *key)?;
        let bytes = file["bytes"].as_u64().filter(|bytes| *bytes <= FILE_MAX)?;
        total = total
            .checked_add(bytes)
            .filter(|bytes| *bytes <= TOTAL_MAX)?;
        if key.2.len() != 64
            || !key
                .2
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return None;
        }
        files.push(DeliveryFile {
            work_item_id: key.0.clone(),
            path: key.1.clone(),
            expected_sha256: key.2.clone(),
        });
    }
    Some((files, total))
}

fn size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    }
}

impl Console {
    #[cfg(feature = "ui-smoke")]
    pub(crate) fn delivery_smoke(&mut self, snapshot: &Value) {
        self.create.studio.files_project = self.create.project.clone();
        self.create.studio.package_mode = true;
        let files = studio_page(snapshot, &self.create.project)
            .map(|page| {
                array(&page["artifacts"])
                    .iter()
                    .take(3)
                    .map(artifact_key)
                    .collect()
            })
            .unwrap_or_default();
        self.create
            .studio
            .packages
            .insert(self.create.project.clone(), files);
    }
    pub fn take_delivery_export_request(&mut self, ready: bool) -> Option<DeliveryExportRequest> {
        if ready {
            self.create.studio.delivery_request.take()
        } else {
            None
        }
    }

    pub fn delivery_export_command(
        request: DeliveryExportRequest,
        destination: Option<String>,
        current_epoch: &Value,
    ) -> Option<Command> {
        let destination = destination.filter(|path| !path.is_empty())?;
        if request.host_epoch != *current_epoch
            || request.files.is_empty()
            || request.files.len() > FILES_MAX
        {
            return None;
        }
        Some(Command::new(
            "project_delivery_export",
            json!({"projectId":request.project_id,"targetPath":destination,
            "files":request.files.iter().map(|file|json!({"workItemId":file.work_item_id,"path":file.path,"expectedSha256":file.expected_sha256})).collect::<Vec<_>>() }),
        ))
    }

    pub(super) fn studio_delivery(&mut self, ui: &mut Ui, snapshot: &Value, page: &Value) {
        file_filters(ui, &mut self.create.studio);
        hint(
            ui,
            "Select up to 32 files, 64 MiB total. The ZIP includes a source and version manifest.",
        );
        let project = self.create.project.clone();
        let selected = self
            .create
            .studio
            .packages
            .entry(project.clone())
            .or_default()
            .clone();
        if !selected.is_empty() {
            ui.collapsing(
                tf(ui, "Review selection ({0})", &[&selected.len().to_string()]),
                |ui| {
                    for key in &selected {
                        ui.push_id(key, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                let present = array(&page["artifacts"])
                                    .iter()
                                    .any(|file| artifact_key(file) == *key);
                                ui.label(&key.1);
                                ui.label(
                                    RichText::new(key.2.chars().take(8).collect::<String>())
                                        .size(11.0)
                                        .color(colors(ui).muted),
                                );
                                if !present {
                                    ui.colored_label(
                                        colors(ui).warning,
                                        t(ui, "Changed or unavailable"),
                                    );
                                }
                                if view_control(ui, "Remove from package", true, false).clicked() {
                                    self.create
                                        .studio
                                        .packages
                                        .get_mut(&project)
                                        .unwrap()
                                        .remove(key);
                                }
                            })
                        });
                    }
                },
            );
        }
        let visible: Vec<_> = array(&page["artifacts"])
            .iter()
            .filter(|file| {
                matches_file(
                    file,
                    &self.create.studio.query,
                    self.create.studio.file_type,
                )
            })
            .collect();
        let union: std::collections::BTreeSet<_> = selected
            .iter()
            .cloned()
            .chain(visible.iter().map(|file| artifact_key(file)))
            .collect();
        ui.horizontal(|ui| {
            if view_control(
                ui,
                "Select shown",
                !visible.is_empty() && union.len() <= FILES_MAX,
                false,
            )
            .clicked()
            {
                *self.create.studio.packages.get_mut(&project).unwrap() = union;
            }
            if view_control(ui, "Clear selection", !selected.is_empty(), false).clicked() {
                self.create
                    .studio
                    .packages
                    .get_mut(&project)
                    .unwrap()
                    .clear();
            }
        });
        if visible.is_empty() {
            hint(ui, "No matching outputs.");
        }
        for file in visible {
            let key = artifact_key(file);
            let selection = self.create.studio.packages.get_mut(&project).unwrap();
            let mut checked = selection.contains(&key);
            let can_select = checked || selection.len() < FILES_MAX;
            let task = items(&snapshot["work"])
                .iter()
                .find(|task| task["workItemId"] == file["workItemId"])
                .map(|task| text(task, "title"))
                .filter(|title| !title.is_empty())
                .unwrap_or(text(file, "path"));
            let row = egui::Frame::new()
                .fill(colors(ui).surface)
                .stroke(egui::Stroke::new(1.0, colors(ui).border))
                .corner_radius(studio::metrics::CARD_RADIUS)
                .inner_margin(studio::metrics::PADDING)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let response = ui.add_enabled(
                        can_select,
                        egui::Checkbox::new(&mut checked, text(file, "name")),
                    );
                    #[cfg(test)]
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new(("delivery-file", &key)), response.rect)
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} · {} · {}",
                            task,
                            size(file["bytes"].as_u64().unwrap_or(0)),
                            key.2.chars().take(8).collect::<String>()
                        ))
                        .size(11.0)
                        .color(colors(ui).muted),
                    );
                    response.changed()
                });
            if row.inner {
                let selection = self.create.studio.packages.get_mut(&project).unwrap();
                if checked {
                    selection.insert(key);
                } else {
                    selection.remove(&key);
                }
            }
        }
    }

    pub(super) fn studio_delivery_footer(&mut self, ui: &mut Ui, snapshot: &Value) {
        let selected = selected_files(&self.create, snapshot);
        let count = self
            .create
            .studio
            .packages
            .get(&self.create.project)
            .map_or(0, |files| files.len());
        let summary = if let Some((_, bytes)) = &selected {
            format!(
                "{} · {}",
                tf(ui, "{0} files selected", &[&count.to_string()]),
                size(*bytes)
            )
        } else {
            tf(ui, "{0} files selected", &[&count.to_string()])
        };
        let footer = egui::Frame::new()
            .fill(colors(ui).raised)
            .stroke(egui::Stroke::new(1.0, colors(ui).border))
            .corner_radius(studio::metrics::CARD_RADIUS)
            .inner_margin(studio::metrics::PADDING)
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if view_control(
                        ui,
                        "Export package",
                        selected.is_some() && !cfg!(target_os = "windows"),
                        true,
                    )
                    .clicked()
                    {
                        let (files, _) = selected.clone().unwrap();
                        self.create.studio.delivery_request = Some(DeliveryExportRequest {
                            project_id: self.create.project.clone(),
                            name: "moxcrew-delivery.zip".into(),
                            files,
                            host_epoch: snapshot["hostEpoch"].clone(),
                        });
                    }
                    if view_control(ui, "Back to files", true, false).clicked() {
                        self.create.studio.package_mode = false;
                    }

                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_max_width(ui.available_width());
                        ui.label(summary);
                        if count > 0 && selected.is_none() {
                            ui.label(
                                RichText::new(t(ui, "Review changed files or package limits."))
                                    .size(11.0)
                                    .color(colors(ui).warning),
                            );
                        }
                    });
                });
            });
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("delivery-footer"),
                (footer.response.rect, ui.clip_rect()),
            )
        });
        #[cfg(not(test))]
        let _ = footer;
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::{click_at_size, render, snapshot, state};
    use super::*;

    fn fixture() -> (Console, Value) {
        let mut data = snapshot();
        data["hostEpoch"] = json!("host:one");
        data["projectStudio"] = json!({"reasonCode":"PROJECT_STUDIO_READ","projectId":"project:one","artifacts":[
            {"workItemId":"work:one","name":"Sunum.PPTX","path":"outputs/Sunum.PPTX","sha256":"a".repeat(64),"bytes":123},
            {"workItemId":"work:two","name":"Özet.md","path":"outputs/Özet.md","sha256":"b".repeat(64),"bytes":456}
        ]});
        let mut create = state();
        create.sync(&data);
        create.select_productive_view(2);
        create.studio.requested = Some((
            "project:one".into(),
            project_revision(&data, "project:one").unwrap(),
        ));
        (
            Console {
                tab: Tab::Create,
                create,
                ..Default::default()
            },
            data,
        )
    }

    #[test]
    fn output_search_and_types_preserve_unicode_and_original_selection() {
        let (_, data) = fixture();
        let files = array(&data["projectStudio"]["artifacts"]);
        assert!(matches_file(&files[0], "sunum", 2));
        assert!(!matches_file(&files[0], "sunum", 1));
        assert!(matches_file(&files[1], "  ÖZET  ", 1));
        assert!(matches_file(&files[1], "outputs/ö", 0));
        assert!(!matches_file(&files[1], "missing", 0));
        assert!(matches_file(&json!({"path":"outputs/a.bin"}), "", 7));
    }

    #[test]
    fn normal_files_filters_accept_pointer_typing_and_last_type_without_a_parent_popup() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let (mut console, data) = fixture();
        let size = egui::vec2(800.0, 560.0);
        assert!(click_at_size(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            "memory_control",
            "Search outputs"
        )
        .is_empty());
        render(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            vec![egui::Event::Text("Özet".into())],
        );
        assert_eq!(console.create.studio.query, "Özet");
        for index in [7usize, 1] {
            assert!(click_at_size(
                &ctx,
                &mut console,
                &data,
                size,
                false,
                "memory_control",
                "File type"
            )
            .is_empty());
            render(&ctx, &mut console, &data, size, false, vec![]);
            let (id, rect, clip) = ctx.data(|d| {
                d.get_temp::<(egui::Id, egui::Rect, egui::Rect)>(egui::Id::new((
                    "delivery-type-option",
                    index,
                )))
                .unwrap()
            });
            assert!(
                clip.contains_rect(rect),
                "option {index}: {rect:?} / {clip:?}"
            );
            ctx.memory_mut(|memory| memory.request_focus(id));
            render(
                &ctx,
                &mut console,
                &data,
                size,
                false,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                }],
            );
            assert_eq!(console.create.studio.file_type, index);
            assert_eq!(console.create.studio.query, "Özet");
            render(
                &ctx,
                &mut console,
                &data,
                size,
                false,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                }],
            );
        }
        assert!(!console.create.studio.package_mode);
    }

    #[test]
    fn right_inspector_keeps_package_actions_and_invalid_selection_warning_bounded() {
        let size = egui::vec2(1280.0, 820.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let (mut console, mut data) = fixture();
                data["desktopPreferences"] = json!({"language":language,"reduceMotion":true});
                console.create.studio.package_mode = true;
                let selected = array(&data["projectStudio"]["artifacts"])
                    .iter()
                    .map(artifact_key)
                    .collect();
                console
                    .create
                    .studio
                    .packages
                    .insert("project:one".into(), selected);
                for invalid in [false, true] {
                    if invalid {
                        data["projectStudio"]["artifacts"][0]["sha256"] = json!("changed-version");
                    }
                    for _ in 0..3 {
                        let commands = render(&ctx, &mut console, &data, size, false, vec![]).0;
                        assert!(commands
                            .iter()
                            .all(|command| command.action.ends_with("_read")));
                    }
                    let (footer, clip) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new("delivery-footer"))
                            .unwrap()
                    });
                    assert!(
                        footer.width() <= 620.0 && footer.left() >= size.x - 652.0,
                        "{language}/{theme:?} bounded inspector {footer:?}"
                    );
                    assert!(
                        screen.contains_rect(footer) && clip.contains_rect(footer),
                        "{language}/{theme:?}/{invalid} footer clipped {footer:?}/{clip:?}"
                    );
                    for label in ["Export package", "Back to files"] {
                        let rect = ctx.data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new((
                                "test-action",
                                "memory_control",
                                label,
                            )))
                            .unwrap()
                        });
                        let (clip, enabled) = ctx.data(|data| {
                            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                                "test-action-visibility",
                                label,
                            )))
                            .unwrap()
                        });
                        assert!(footer.contains_rect(rect) && clip.contains_rect(rect));
                        if label == "Export package" {
                            assert_eq!(enabled, !invalid && !cfg!(target_os = "windows"));
                        }
                    }
                    if invalid {
                        assert!(click_at_size(
                            &ctx,
                            &mut console,
                            &data,
                            size,
                            false,
                            "memory_control",
                            "Export package"
                        )
                        .is_empty());
                        assert!(console.take_delivery_export_request(true).is_none());
                    }
                }
            }
        }
    }

    #[test]
    fn package_selection_filtering_and_native_request_are_minimum_sized_and_frozen() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let (mut console, mut data) = fixture();
                data["desktopPreferences"] = json!({"language":language,"reduceMotion":true});
                let size = egui::vec2(800.0, 560.0);
                assert!(click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Create package"
                )
                .is_empty());
                assert!(console.create.studio.package_mode);
                assert!(click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Select shown"
                )
                .is_empty());
                assert_eq!(selected_files(&console.create, &data).unwrap().0.len(), 2);
                let (_, search) = ctx.data(|d| {
                    d.get_temp::<(egui::Rect, egui::Id)>(egui::Id::new("delivery-search"))
                        .unwrap()
                });
                ctx.memory_mut(|memory| memory.request_focus(search));
                render(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    vec![egui::Event::Text("missing".into())],
                );
                assert_eq!(console.create.studio.query, "missing");
                let (_, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                assert!(copy.iter().any(|s| s
                    == if language == "tr" {
                        "Eşleşen çıktı yok."
                    } else {
                        "No matching outputs."
                    }));
                let (clip, enabled) = ctx.data(|d| {
                    d.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Export package",
                    )))
                    .unwrap()
                });
                let rect = ctx.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "memory_control",
                        "Export package",
                    )))
                    .unwrap()
                });
                assert!(
                    enabled
                        && clip.contains_rect(rect)
                        && egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(rect),
                    "{language}/{theme:?} {rect:?} {clip:?}"
                );
                assert!(click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    true,
                    "memory_control",
                    "Export package"
                )
                .is_empty());
                assert!(console.take_delivery_export_request(true).is_none());
                assert!(click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Export package"
                )
                .is_empty());
                assert!(
                    console.take_delivery_export_request(false).is_none(),
                    "Automatic reads cannot consume a frozen save request"
                );
                let request = console.take_delivery_export_request(true).unwrap();
                assert_eq!(
                    request.files.len(),
                    2,
                    "Hidden filtered files remain explicitly selected"
                );
                assert_eq!(request.files[1].path, "outputs/Özet.md");
                assert!(Console::delivery_export_command(
                    request.clone(),
                    None,
                    &data["hostEpoch"]
                )
                .is_none());
                assert!(Console::delivery_export_command(
                    request.clone(),
                    Some("/tmp/out.zip".into()),
                    &json!("host:new")
                )
                .is_none());
                console.create.project = "project:other".into();
                let command = Console::delivery_export_command(
                    request,
                    Some("/tmp/out.zip".into()),
                    &data["hostEpoch"],
                )
                .unwrap();
                assert_eq!(command.payload["projectId"], "project:one");
                assert_eq!(
                    command.payload["files"][0]["expectedSha256"],
                    "a".repeat(64)
                );
                assert!(command.payload.get("hostEpoch").is_none());
                assert_eq!(
                    console.create.drafts[&("project:one".into(), "P1".into())].goal,
                    "Owner goal"
                );
            }
        }
    }

    #[test]
    fn package_changed_versions_missing_scope_and_limits_block_export_without_silently_dropping_selection(
    ) {
        let (mut console, mut data) = fixture();
        let keys = array(&data["projectStudio"]["artifacts"])
            .iter()
            .map(artifact_key)
            .collect();
        console
            .create
            .studio
            .packages
            .insert("project:one".into(), keys);
        assert_eq!(selected_files(&console.create, &data).unwrap().1, 579);
        data["projectStudio"]["artifacts"][0]["sha256"] = json!("c".repeat(64));
        assert!(selected_files(&console.create, &data).is_none());
        assert_eq!(console.create.studio.packages["project:one"].len(), 2);
        data["projectStudio"]["artifacts"][0]["sha256"] = json!("a".repeat(64));
        data["projectStudio"]["artifacts"][0]["bytes"] = json!(FILE_MAX + 1);
        assert!(selected_files(&console.create, &data).is_none());
        data["projectStudio"]["artifacts"][0]["bytes"] = json!(123);
        data["projectStudio"]["projectId"] = json!("project:other");
        assert!(selected_files(&console.create, &data).is_none());
        data["projectStudio"]["projectId"] = json!("project:one");
        console
            .create
            .studio
            .packages
            .get_mut("project:one")
            .unwrap()
            .extend((0..33).map(|i| (format!("work:{i}"), "outputs/a.txt".into(), "a".repeat(64))));
        assert!(selected_files(&console.create, &data).is_none());
        data["hostEpoch"] = json!("host:two");
        console.create.sync(&data);
        assert!(console.create.studio.packages.is_empty());
        assert!(console.take_delivery_export_request(true).is_none());
    }
}
