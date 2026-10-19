use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudioPathKind {
    Resource,
    Tool { tool_id: String },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioPathRequest {
    pub project_id: String,
    pub expected_project_revision: u64,
    pub kind: StudioPathKind,
}
#[derive(Default)]
pub(super) struct StudioState {
    pub(super) open: bool,
    pub(super) tab: usize,
    pub(super) requested: Option<(String, u64)>,
    selected: HashMap<String, (String, String, String)>,
    section: String,
    revisions: HashMap<(String, String, String, String), String>,
    page: usize,
    texture: Option<(String, egui::TextureHandle)>,
    pub(super) path_request: Option<StudioPathRequest>,
    connection_project: String,
    connection_provider: String,
    label: String,
    token: String,
    operation: String,
    arguments: HashMap<String, String>,
    prepared: Option<(String, String, u64)>,
    pending_prepare: Option<(String, String)>,
    executing: Option<(String, String)>,
    observed_receipt: String,
}

fn studio_page<'a>(snapshot: &'a Value, project: &str) -> Option<&'a Value> {
    let page = &snapshot["projectStudio"];
    (text(page, "projectId") == project && text(page, "reasonCode") == "PROJECT_STUDIO_READ")
        .then_some(page)
}
fn safe_png(bytes: &[u8]) -> bool {
    if bytes.len() < 24
        || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10]
        || &bytes[12..16] != b"IHDR"
    {
        return false;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    width > 0 && height > 0 && width <= 4096 && height <= 4096
}
fn artifact_key(artifact: &Value) -> (String, String, String) {
    (
        text(artifact, "workItemId").into(),
        text(artifact, "path").into(),
        text(artifact, "sha256").into(),
    )
}
fn selected_artifact<'a>(state: &CreateState, snapshot: &'a Value) -> Option<&'a Value> {
    let key = state.studio.selected.get(&state.project)?;
    array(&studio_page(snapshot, &state.project)?["artifacts"])
        .iter()
        .find(|artifact| artifact_key(artifact) == *key)
}
fn artifact_preview<'a>(
    state: &CreateState,
    snapshot: &'a Value,
    artifact: &Value,
) -> Option<&'a Value> {
    let preview = &snapshot["projectArtifact"];
    (text(preview, "reasonCode") == "PROJECT_ARTIFACT_READ"
        && text(preview, "projectId") == state.project
        && artifact_key(preview) == artifact_key(artifact))
    .then_some(preview)
}
pub(super) fn project_revision(snapshot: &Value, project: &str) -> Option<u64> {
    array(&snapshot["projectMemory"]["projects"])
        .iter()
        .find(|item| text(item, "projectId") == project)
        .map(revision)
        .filter(|rev| *rev > 0)
}

fn app_name(provider: &str) -> &str {
    match provider {
        "google_drive" => "Google Drive",
        "notion" => "Notion",
        "google_calendar" => "Google Calendar",
        _ => "App",
    }
}
fn app_operation_name(operation: &str) -> &str {
    match operation {
        "drive_import" => "Import Drive file",
        "notion_page" => "Create Notion page",
        "calendar_event" => "Create Calendar event",
        _ => "App operation",
    }
}
fn app_values(ui: &mut Ui, values: &Value) {
    let Some(values) = values.as_object() else {
        return;
    };
    let labels = [
        ("providerId", "App"),
        ("operation", "Operation"),
        ("fileId", "Drive file ID"),
        ("parentPageId", "Parent page ID"),
        ("calendarId", "Calendar ID"),
        ("pageId", "Page ID"),
        ("eventId", "Event ID"),
        ("title", "Title"),
        ("summary", "Title"),
        ("name", "Name"),
        ("content", "Content"),
        ("description", "Description"),
        ("start", "Start time"),
        ("end", "End time"),
        ("timeZone", "Time zone"),
        ("bytes", "Size (bytes)"),
    ];
    let keys = labels
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| values.contains_key(*key))
        .chain(
            values
                .keys()
                .map(String::as_str)
                .filter(|key| !labels.iter().any(|(known, _)| known == key)),
        );
    for key in keys {
        let label = labels
            .iter()
            .find(|(known, _)| *known == key)
            .map(|(_, label)| *label)
            .unwrap_or(key);
        let value = &values[key];
        let value = match key {
            "providerId" => app_name(value.as_str().unwrap_or("")).to_owned(),
            "operation" => t(ui, app_operation_name(value.as_str().unwrap_or(""))).into_owned(),
            _ => value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string()),
        };
        ui.horizontal_top(|ui| {
            ui.add_sized(
                [130.0, 18.0],
                egui::Label::new(
                    RichText::new(t(ui, label))
                        .size(12.0)
                        .color(colors(ui).muted),
                )
                .wrap(),
            );
            ui.add(egui::Label::new(value).wrap().selectable(true));
        });
        ui.add_space(4.0);
    }
}

impl CreateState {
    pub(crate) fn close_studio(&mut self) {
        self.studio.open = false;
        self.studio.token.clear();
        self.studio.prepared = None;
        self.studio.pending_prepare = None;
    }

    pub(super) fn sync_studio(&mut self, snapshot: &Value) {
        if project_revision(snapshot, &self.project).is_none() {
            self.studio.open = false;
            self.studio.token.clear();
            self.studio.prepared = None;
            self.studio.pending_prepare = None;
        }

        let receipt = &snapshot["projectStudioReceipt"];
        if text(receipt, "projectId") == self.project
            && !text(receipt, "commandId").is_empty()
            && self.studio.observed_receipt != text(receipt, "commandId")
        {
            self.studio.observed_receipt = text(receipt, "commandId").into();
            if text(receipt, "outcome") == "applied"
                && matches!(
                    text(receipt, "action"),
                    "project_resource_import"
                        | "project_resource_remove"
                        | "project_tool_configure"
                        | "project_connection_set"
                        | "project_connection_remove"
                        | "project_artifact_revise"
                )
            {
                self.studio.requested = None;
            }
        }

        if self.studio.connection_project != self.project || !self.studio.open {
            self.studio.token.clear();
            self.studio.connection_project = self.project.clone();
            self.studio.prepared = None;
            self.studio.pending_prepare = None;
        }
        if let Some((command, project)) = &self.studio.pending_prepare {
            let receipt = &snapshot["projectStudioReceipt"];
            if text(receipt, "commandId") == command && text(receipt, "projectId") == project {
                if text(receipt, "outcome") == "applied"
                    && !text(receipt, "preparedActionId").is_empty()
                {
                    self.studio.prepared = Some((
                        project.clone(),
                        text(receipt, "preparedActionId").into(),
                        revision(receipt),
                    ));
                }
                self.studio.pending_prepare = None;
            }
        }
    }
}

impl Console {
    #[cfg(feature = "ui-smoke")]
    pub(super) fn studio_prepare_smoke(&mut self, snapshot: &Value) {
        let receipt = &snapshot["projectStudioReceipt"];
        self.create.studio.connection_project = self.create.project.clone();
        self.create.studio.connection_provider = text(receipt, "providerId").into();
        self.create.studio.arguments = receipt["arguments"]
            .as_object()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.into()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.create.studio.prepared = Some((
            self.create.project.clone(),
            text(receipt, "preparedActionId").into(),
            revision(receipt),
        ));
    }
    #[cfg(feature = "ui-smoke")]
    pub(super) fn studio_select_first(&mut self, snapshot: &Value) {
        if let Some(artifact) = studio_page(snapshot, &self.create.project)
            .and_then(|page| array(&page["artifacts"]).first())
        {
            self.create
                .studio
                .selected
                .insert(self.create.project.clone(), artifact_key(artifact));
        }
    }
    pub fn take_studio_path_request(&mut self) -> Option<StudioPathRequest> {
        self.create.studio.path_request.take()
    }
    pub fn studio_path_command(
        request: StudioPathRequest,
        path: Option<String>,
    ) -> Option<Command> {
        let path = path.filter(|path| valid_input(path, 4096))?;
        Some(match request.kind {
            StudioPathKind::Resource => Command::new(
                "project_resource_import",
                json!({"projectId":request.project_id,"expectedProjectRevision":request.expected_project_revision,"sourcePath":path}),
            ),
            StudioPathKind::Tool { tool_id } => {
                if !valid_input(&tool_id, 80) {
                    return None;
                }
                Command::new(
                    "project_tool_configure",
                    json!({"projectId":request.project_id,"toolId":tool_id,"executablePath":path}),
                )
            }
        })
    }
    pub(super) fn studio_button(&mut self, ui: &mut Ui) {
        if control(ui, "Outputs & resources", !self.create.project.is_empty()).clicked() {
            self.create.studio.open = true;
            self.create.crew.panel_open = false;
            self.create.chat.files_open = false;
        }
    }
    pub(super) fn output_studio(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if !self.create.studio.open {
            self.create.studio.token.clear();
            egui::text_edit::TextEditState::default()
                .store(ui.ctx(), egui::Id::new("app-access-token"));
            return;
        }
        let ready = ui.is_enabled();
        let project = self.create.project.clone();
        let Some(rev) = project_revision(snapshot, &project) else {
            self.create.studio.open = false;
            return;
        };
        let key = (project.clone(), rev);
        if ready
            && !self.hold_automatic_reads
            && self.create.studio.requested.as_ref() != Some(&key)
            && commands.is_empty()
        {
            self.create.studio.requested = Some(key);
            commands.push(Command::new(
                "project_studio_read",
                json!({"projectId":project}),
            ));
        }
        let id = egui::Id::new("output-studio");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .show(ui.ctx(), |ui| {
                ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 860.0));
                ui.add_enabled_ui(ready, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(studio::title(t(ui, "Output Studio"), 24.0));
                        for (index, label) in
                            [(0, "Outputs"), (1, "Resources"), (2, "Tools"), (3, "Apps")]
                        {
                            ui.selectable_value(&mut self.create.studio.tab, index, t(ui, label));
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_salt(("output-studio-body", &project, self.create.studio.tab))
                        .auto_shrink([false, false])
                        .stick_to_bottom(
                            self.create.studio.tab == 3 && self.create.studio.prepared.is_some(),
                        )
                        .max_height((ui.ctx().viewport_rect().height() - 210.0).max(160.0))
                        .show(ui, |ui| {
                            if let Some(page) = studio_page(snapshot, &project) {
                                match self.create.studio.tab {
                                    1 => self.studio_resources(ui, snapshot, page, commands),
                                    2 => self.studio_tools(ui, snapshot, page, commands),
                                    3 => self.studio_apps(ui, snapshot, page, commands),
                                    _ => self.studio_artifacts(ui, snapshot, page, commands),
                                }
                            } else {
                                hint(ui, "Check project outputs and resources to begin.");
                            }
                            let error = &snapshot["projectStudioError"];
                            if text(error, "projectId") == project {
                                hint(ui, text(error, "reasonCode"));
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        action(
                            ui,
                            commands,
                            "Refresh",
                            "project_studio_read",
                            !self.hold_automatic_reads,
                            || json!({"projectId":project}),
                        );
                        if control(ui, "Close", true).clicked() {
                            self.create.studio.open = false;
                            self.create.studio.token.clear();
                        }
                    });
                });
            });
        if modal.should_close() {
            self.create.studio.open = false;
            self.create.studio.token.clear();
        }
    }
    fn studio_artifacts(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: &Value,
        commands: &mut Vec<Command>,
    ) {
        let artifacts = array(&page["artifacts"]);
        if artifacts.is_empty() {
            hint(
                ui,
                "Completed outputs appear here with their original versions.",
            );
            return;
        }
        let current = selected_artifact(&self.create, snapshot)
            .map(|artifact| {
                format!(
                    "{} · {}",
                    text(artifact, "name"),
                    text(artifact, "versionId")
                        .chars()
                        .take(8)
                        .collect::<String>()
                )
            })
            .unwrap_or_else(|| t(ui, "Choose an output").into_owned());
        egui::ComboBox::from_id_salt("studio-artifact-version")
            .selected_text(current)
            .width(ui.available_width().min(540.0))
            .height(220.0)
            .show_ui(ui, |ui| {
                for artifact in artifacts {
                    let key = artifact_key(artifact);
                    let selected =
                        self.create.studio.selected.get(&self.create.project) == Some(&key);
                    let version = text(artifact, "versionId")
                        .chars()
                        .take(8)
                        .collect::<String>();
                    if ui
                        .selectable_label(
                            selected,
                            format!(
                                "{} · {} · {} B",
                                text(artifact, "name"),
                                version,
                                artifact["bytes"].as_u64().unwrap_or(0)
                            ),
                        )
                        .clicked()
                    {
                        self.create
                            .studio
                            .selected
                            .insert(self.create.project.clone(), key);
                        self.create.studio.page = 0;
                        self.create.studio.section.clear();
                    }
                }
            });
        let Some(artifact) = selected_artifact(&self.create, snapshot).cloned() else {
            hint(ui, "Choose an output to preview or revise.");
            return;
        };
        ui.separator();
        ui.label(studio::title(text(&artifact, "name"), 18.0));
        ui.collapsing(t(ui, "Version & provenance"), |ui| {
            for field_name in [
                "versionId",
                "parentVersionId",
                "workItemId",
                "path",
                "sha256",
            ] {
                if !text(&artifact, field_name).is_empty() {
                    ui.label(format!("{field_name}: {}", text(&artifact, field_name)));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            action(ui,commands,"Preview","project_artifact_read",!cfg!(target_os="windows"),||json!({"projectId":self.create.project,"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"]}));
            if control(ui,"Export copy",!cfg!(target_os="windows")).clicked() {
                self.create.export_request=Some(CreateExportRequest{project_id:self.create.project.clone(),work_item_id:text(&artifact,"workItemId").into(),path:text(&artifact,"path").into(),name:text(&artifact,"name").into(),expected_sha256:Some(text(&artifact,"sha256").into())});
            }
        });
        let Some(preview) = artifact_preview(&self.create, snapshot, &artifact) else {
            hint(ui, "Preview is loaded only for this exact file version.");
            return;
        };
        self.studio_preview(ui, &preview["preview"], text(&artifact, "sha256"));
        let sections = array(&preview["sections"]);
        egui::ComboBox::from_id_salt("revision-section")
            .selected_text(
                sections
                    .iter()
                    .find(|section| text(section, "id") == self.create.studio.section)
                    .map(|section| text(section, "label"))
                    .map(std::borrow::Cow::Borrowed)
                    .unwrap_or_else(|| t(ui, "Whole output")),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.create.studio.section,
                    String::new(),
                    t(ui, "Whole output"),
                );
                for section in sections {
                    ui.selectable_value(
                        &mut self.create.studio.section,
                        text(section, "id").into(),
                        text(section, "label"),
                    );
                }
            });
        let key = (
            self.create.project.clone(),
            text(&artifact, "workItemId").into(),
            text(&artifact, "path").into(),
            text(&artifact, "sha256").into(),
        );
        let instruction = self.create.studio.revisions.entry(key).or_default();
        multiline(ui, "Revision instructions", instruction, 12_000, 2);
        let valid = valid_input(instruction, 12_000);
        let mut payload = json!({"projectId":self.create.project,"expectedProjectRevision":project_revision(snapshot,&self.create.project),"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"],"instruction":instruction.trim()});
        if !self.create.studio.section.is_empty() {
            payload["sectionId"] = json!(self.create.studio.section);
        }
        action(
            ui,
            commands,
            "Prepare revision",
            "project_artifact_revise",
            valid,
            || payload,
        );
        hint(ui,"A revision prepares a new task. The original stays intact; start it explicitly in Tasks.");
    }
    fn studio_preview(&mut self, ui: &mut Ui, preview: &Value, hash: &str) {
        match text(preview, "kind") {
            "text" => {
                ui.add(
                    egui::Label::new(text(preview, "text"))
                        .selectable(true)
                        .wrap(),
                );
            }
            "pages" => {
                hint(
                    ui,
                    "Document pages are a text preview of the editable original.",
                );
                let pages = array(&preview["pages"]);
                self.create.studio.page =
                    self.create.studio.page.min(pages.len().saturating_sub(1));
                ui.horizontal_wrapped(|ui| {
                    for (i, page) in pages.iter().enumerate() {
                        ui.selectable_value(&mut self.create.studio.page, i, text(page, "title"));
                    }
                });
                if let Some(page) = pages.get(self.create.studio.page) {
                    ui.add(egui::Label::new(text(page, "text")).wrap().selectable(true));
                }
            }
            "image" | "media" => {
                if self
                    .create
                    .studio
                    .texture
                    .as_ref()
                    .is_none_or(|(key, _)| key != hash)
                {
                    let bytes: Option<Vec<u8>> = preview["pngBytes"]
                        .as_array()
                        .filter(|bytes| bytes.len() <= 8 * 1024 * 1024)
                        .and_then(|bytes| {
                            bytes
                                .iter()
                                .map(|byte| {
                                    byte.as_u64()
                                        .filter(|byte| *byte <= 255)
                                        .map(|byte| byte as u8)
                                })
                                .collect()
                        });
                    self.create.studio.texture = bytes
                        .filter(|bytes| safe_png(bytes))
                        .and_then(|bytes| eframe::icon_data::from_png_bytes(&bytes).ok())
                        .filter(|image| image.width <= 4096 && image.height <= 4096)
                        .map(|image| {
                            let image = egui::ColorImage::from_rgba_unmultiplied(
                                [image.width as usize, image.height as usize],
                                &image.rgba,
                            );
                            (
                                hash.into(),
                                ui.ctx().load_texture(
                                    format!("artifact-{hash}"),
                                    image,
                                    egui::TextureOptions::LINEAR,
                                ),
                            )
                        });
                }
                if let Some((_, texture)) = &self.create.studio.texture {
                    ui.add(
                        egui::Image::new(texture).max_size(egui::vec2(ui.available_width(), 260.0)),
                    );
                }
                for key in ["width", "height", "durationSeconds"] {
                    if preview[key].is_number() {
                        ui.label(format!("{key}: {}", preview[key]));
                    }
                }
                if text(preview, "kind") == "media" {
                    hint(ui,"Media metadata and thumbnail. Export the original for playback or editing.");
                }
            }
            _ => hint(
                ui,
                "Native preview is unavailable for this format. Export the original.",
            ),
        }
    }
    fn studio_resources(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: &Value,
        commands: &mut Vec<Command>,
    ) {
        hint(
            ui,
            "Imported resources are frozen copies available only to this project.",
        );
        if control(ui, "Import resource", true).clicked() {
            self.create.studio.path_request = Some(StudioPathRequest {
                project_id: self.create.project.clone(),
                expected_project_revision: project_revision(snapshot, &self.create.project)
                    .unwrap_or(0),
                kind: StudioPathKind::Resource,
            });
        }
        for resource in array(&page["resources"]) {
            ui.push_id(text(resource,"resourceId"),|ui|card(ui,|ui| {
                ui.label(RichText::new(text(resource,"name")).strong());
                raw_hint(ui,&format!("{} · {} B",text(resource,"kind"),resource["bytes"].as_u64().unwrap_or(0)));
                status(ui,text(resource,"status"));
                ui.collapsing(t(ui,"Provenance"),|ui| {ui.monospace(text(resource,"sha256"));});
                action(ui,commands,"Remove resource","project_resource_remove",revision(resource)>0,||json!({"projectId":self.create.project,"resourceId":resource["resourceId"],"expectedResourceRevision":resource["revision"]}));
            }));
        }
    }
    fn studio_tools(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: &Value,
        _commands: &mut Vec<Command>,
    ) {
        hint(ui,"Production uses installed tools: editable Office files and data with Python; images, video and audio with FFmpeg.");
        hint(ui,"Tool paths are shared by all local projects. Choose an existing executable; nothing is installed automatically.");
        for tool in array(&page["tools"]) {
            ui.push_id(text(tool, "id"), |ui| {
                card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(text(tool, "name")).strong());
                        hint(
                            ui,
                            if tool["available"] == true {
                                "Ready"
                            } else {
                                "Setup needed"
                            },
                        );
                        if control(ui, "Choose executable", !text(tool, "id").is_empty()).clicked()
                        {
                            self.create.studio.path_request = Some(StudioPathRequest {
                                project_id: self.create.project.clone(),
                                expected_project_revision: project_revision(
                                    snapshot,
                                    &self.create.project,
                                )
                                .unwrap_or(0),
                                kind: StudioPathKind::Tool {
                                    tool_id: text(tool, "id").into(),
                                },
                            });
                        }
                        if let Some(url) = tool["setupUrl"]
                            .as_str()
                            .filter(|url| url.starts_with("https://"))
                        {
                            if ui.link(t(ui, "Official setup guide")).clicked() {
                                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                            }
                        }
                    });
                    hint(ui, text(tool, "reasonCode"));
                })
            });
        }
    }
    fn studio_apps(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: &Value,
        commands: &mut Vec<Command>,
    ) {
        ui.add_space(12.0);
        ui.label(studio::title(t(ui, "App connections"), 20.0));
        hint(
            ui,
            "Saved app credentials are unverified until an operation succeeds.",
        );
        for connection in array(&page["connections"]) {
            ui.horizontal_wrapped(|ui| {
                let name=app_name(text(connection,"providerId"));
                ui.label(if text(connection,"label").is_empty(){name.into()}else{format!("{name} · {}",text(connection,"label"))});
                hint(ui,if connection["connected"]==true {"Credentials saved"}else{"Not connected"});
                action(ui,commands,"Disconnect","project_connection_remove",connection["connected"]==true,||json!({"projectId":self.create.project,"providerId":connection["providerId"]}));
            });
        }
        let state = &mut self.create.studio;
        if state.connection_provider.is_empty() {
            state.connection_provider = "google_drive".into();
        }
        let previous = state.connection_provider.clone();
        egui::ComboBox::from_id_salt("app-provider")
            .width(190.0)
            .selected_text(app_name(&state.connection_provider))
            .show_ui(ui, |ui| {
                for (id, label) in [
                    ("google_drive", "Google Drive"),
                    ("notion", "Notion"),
                    ("google_calendar", "Google Calendar"),
                ] {
                    ui.selectable_value(&mut state.connection_provider, id.into(), label);
                }
            });
        if previous != state.connection_provider {
            state.token.clear();
            state.prepared = None;
            state.arguments.clear();
        }
        ui.collapsing(t(ui,"Connect account"),|ui| {
            field(ui,"Connection label",&mut state.label,80,false);
            let token_id=egui::Id::new("app-access-token");
            ui.label(t(ui,"Access token"));
            ui.add(egui::TextEdit::singleline(&mut state.token).id(token_id).password(true).desired_width(f32::INFINITY).char_limit(16_000));
            if let Some(mut state)=egui::text_edit::TextEditState::load(ui.ctx(),token_id){state.clear_undoer();state.store(ui.ctx(),token_id);}
            hint(ui,"The token stays private on this host. Connecting does not perform a remote operation.");
            if control(ui,"Save connection",valid_input(&state.token,16_000)&&valid_input(&state.label,80)).clicked() && commands.is_empty() {
                let token=std::mem::take(&mut state.token);
                egui::text_edit::TextEditState::default().store(ui.ctx(),token_id);
                commands.push(Command::new("project_connection_set",json!({"projectId":self.create.project,"providerId":state.connection_provider,"label":state.label,"accessToken":token})));
            }
        });
        let (operation, fields): (&str, &[(&str, &str)]) = match state.connection_provider.as_str()
        {
            "notion" => (
                "notion_page",
                &[
                    ("parentPageId", "Parent page ID"),
                    ("title", "Title"),
                    ("content", "Content"),
                ],
            ),
            "google_calendar" => (
                "calendar_event",
                &[
                    ("calendarId", "Calendar ID"),
                    ("summary", "Title"),
                    ("description", "Description"),
                    ("start", "Start (RFC 3339)"),
                    ("end", "End (RFC 3339)"),
                    ("timeZone", "Time zone"),
                ],
            ),
            _ => ("drive_import", &[("fileId", "Drive file ID")]),
        };
        state.operation = operation.into();
        let operation_ui=egui::CollapsingHeader::new(t(ui,"Prepare app operation")).default_open(state.prepared.is_some()).show(ui,|ui| {
            let before=state.arguments.clone();
            for (key,label) in fields {field(ui,label,state.arguments.entry((*key).into()).or_default(),20_000,false);}
            if before!=state.arguments{state.prepared=None;state.pending_prepare=None;}
            let connected=array(&page["connections"]).iter().any(|connection|text(connection,"providerId")==state.connection_provider&&connection["connected"]==true);
            if control(ui,"Preview operation",connected&&fields.iter().all(|(key,_)|state.arguments.get(*key).is_some_and(|value|!value.trim().is_empty()))).clicked() && commands.is_empty() {
                let command=Command::new("project_connection_prepare",json!({"projectId":self.create.project,"providerId":state.connection_provider,"operation":operation,"arguments":state.arguments}));
                state.prepared=None;
                state.pending_prepare=Some((text(&command.payload,"commandId").into(),self.create.project.clone()));
                commands.push(command);
            }
            let receipt=&snapshot["projectStudioReceipt"];
            if let Some((project,action_id,rev))=&state.prepared {
                if project==&self.create.project && text(receipt,"preparedActionId")==action_id && text(receipt,"projectId")==project && text(receipt,"reasonCode")=="PROJECT_CONNECTION_PREPARED" && revision(receipt)==*rev && text(receipt,"outcome")=="applied" {
                    ui.label(t(ui,"Review the exact remote operation before approving."));
                    ui.label(t(ui,app_operation_name(text(receipt,"operation"))));
                    if let Some(preview)=receipt["preview"].as_str(){ui.add(egui::Label::new(preview).wrap().selectable(true));}
                    app_values(ui,&receipt["arguments"]);
                    if control(ui,"Approve & execute",state.executing.as_ref().is_none_or(|(_,id)|id!=action_id)).clicked() && commands.is_empty() {
                        let command=Command::new("project_connection_execute",json!({"projectId":project,"preparedActionId":action_id,"expectedActionRevision":rev}));
                        state.executing=Some((project.clone(),action_id.clone()));
                        commands.push(command);
                    }
                }
            }
            if let Some((project,id))=&state.executing {
                if text(receipt,"projectId")==project && text(receipt,"preparedActionId")==id {
                    hint(ui,text(receipt,"reasonCode"));
                    app_values(ui,&receipt["result"]);
                    if text(receipt,"reasonCode")=="PROJECT_CONNECTION_OUTCOME_UNKNOWN" {hint(ui,"Remote outcome is uncertain. Check the connected app before preparing another operation.");}
                }
            }
            hint(ui,"Approval performs the shown operation. An uncertain remote write is never retried automatically.");
        });
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("test-action", "memory_control", "Prepare app operation")),
                operation_ui.header_response.rect,
            )
        });
        #[cfg(not(test))]
        let _ = operation_ui;
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click, render, snapshot, state};
    use super::*;
    fn fixture() -> Value {
        let mut value = snapshot();
        value["desktopPreferences"] = json!({"language":"en"});
        value["projectStudio"] = json!({"reasonCode":"PROJECT_STUDIO_READ","projectId":"project:one","revision":7,"resources":[{"resourceId":"resource:one","revision":2,"name":"Brief.txt","kind":"text","bytes":120,"sha256":"hash-res","status":"ready"}],"tools":[],"connections":[],"artifacts":[{"artifactId":"artifact:one","versionId":"version:one","workItemId":"work:one","path":"outputs/deck.pptx","name":"deck.pptx","sha256":"hash-one","bytes":100,"kind":"presentation"}]});
        value
    }
    fn console(data: &Value) -> Console {
        let mut create = state();
        create.studio.open = true;
        create.studio.requested = Some((
            "project:one".into(),
            project_revision(data, "project:one").unwrap(),
        ));
        create.studio.selected.insert(
            "project:one".into(),
            (
                "work:one".into(),
                "outputs/deck.pptx".into(),
                "hash-one".into(),
            ),
        );
        Console {
            tab: Tab::Create,
            create,
            ..Default::default()
        }
    }
    #[test]
    fn native_preview_revision_and_export_are_exact_file_hash_and_project_scoped() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        let mut console = console(&data);
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_artifact_read",
            "Preview",
        )
        .remove(0);
        assert_eq!(command.payload["expectedSha256"], "hash-one");
        assert_eq!(command.payload["projectId"], "project:one");
        data["projectArtifact"] = json!({"reasonCode":"PROJECT_ARTIFACT_READ","projectId":"project:other","workItemId":"work:one","path":"outputs/deck.pptx","sha256":"hash-one","preview":{"kind":"text","text":"FOREIGN CANARY"},"sections":[]});
        assert!(!render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 2200.0),
            false,
            vec![]
        )
        .1
        .join("\n")
        .contains("FOREIGN CANARY"));
        data["projectArtifact"]["projectId"] = json!("project:one");
        data["projectArtifact"]["sha256"] = json!("changed-hash");
        assert!(artifact_preview(
            &console.create,
            &data,
            &data["projectStudio"]["artifacts"][0]
        )
        .is_none());
        data["projectArtifact"]["sha256"] = json!("hash-one");
        data["projectArtifact"]["sections"] = json!([{"id":"slide:2","label":"Slide 2"}]);
        console.create.studio.section = "slide:2".into();
        console.create.studio.revisions.insert(
            (
                "project:one".into(),
                "work:one".into(),
                "outputs/deck.pptx".into(),
                "hash-one".into(),
            ),
            "Make the second slide concise".into(),
        );
        for _ in 0..3 {
            render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 2200.0),
                false,
                vec![],
            );
        }
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_artifact_revise",
            "Prepare revision",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["sectionId"], "slide:2");
        assert_eq!(commands[0].payload["expectedSha256"], "hash-one");
        assert!(commands
            .iter()
            .all(|command| command.action != "work_start"));
        click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Export copy",
        );
        let request = console.take_create_export_request().unwrap();
        console.create.project = "project:other".into();
        let export =
            Console::create_export_command(request, Some("/tmp/frozen-deck.pptx".into())).unwrap();
        assert_eq!(export.payload["projectId"], "project:one");
        assert_eq!(export.payload["expectedSha256"], "hash-one");
    }
    #[test]
    fn picker_cancellation_is_inert_and_import_tool_targets_remain_frozen() {
        let request = StudioPathRequest {
            project_id: "project:one".into(),
            expected_project_revision: 7,
            kind: StudioPathKind::Resource,
        };
        assert!(Console::studio_path_command(request.clone(), None).is_none());
        let command =
            Console::studio_path_command(request, Some("/tmp/owner-chosen.txt".into())).unwrap();
        assert_eq!(command.action, "project_resource_import");
        assert_eq!(command.payload["expectedProjectRevision"], 7);
        let request = StudioPathRequest {
            project_id: "project:one".into(),
            expected_project_revision: 7,
            kind: StudioPathKind::Tool {
                tool_id: "pdftotext".into(),
            },
        };
        let command = Console::studio_path_command(request, Some("/tmp/pdftotext".into())).unwrap();
        assert_eq!(command.payload["toolId"], "pdftotext");
    }
    #[test]
    fn studio_footer_and_tabs_fit_minimum_locales_themes_without_auto_execution() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                for tab in 0..4 {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    let mut console = console(&data);
                    console.create.studio.tab = tab;
                    for _ in 0..3 {
                        assert!(render(&ctx, &mut console, &data, size, false, vec![])
                            .0
                            .is_empty());
                    }
                    let rect = ctx.data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new((
                            "test-action",
                            "memory_control",
                            "Close",
                        )))
                        .unwrap()
                    });
                    let (clip, _) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                            "test-action-visibility",
                            "Close",
                        )))
                        .unwrap()
                    });
                    assert!(
                        screen.contains_rect(rect) && clip.contains_rect(rect),
                        "{language}/{theme:?}/{tab} {rect:?} {clip:?}"
                    );
                    let id = ctx.data(|data| {
                        data.get_temp::<egui::Id>(egui::Id::new((
                            "test-action-id",
                            "memory_control",
                            "Close",
                        )))
                        .unwrap()
                    });
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
                    assert!(
                        !console.create.studio.open,
                        "keyboard close must work without preparing any task"
                    );
                }
            }
        }
    }
    #[test]
    fn app_approval_requires_the_exact_prepared_receipt_and_unknown_outcome_never_resubmits() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["connections"] =
            json!([{"providerId":"google_drive","label":"Test Drive","connected":true}]);
        let mut console = console(&data);
        console.create.studio.tab = 3;
        console.create.studio.connection_project = "project:one".into();
        console.create.studio.connection_provider = "google_drive".into();
        console
            .create
            .studio
            .arguments
            .insert("fileId".into(), "owner-file".into());
        console.create.studio.prepared =
            Some(("project:one".into(), "connection-action:one".into(), 1));
        data["projectStudioReceipt"] = json!({"projectId":"project:one","preparedActionId":"connection-action:one","revision":1,"outcome":"applied","reasonCode":"PROJECT_CONNECTION_PREPARED","operation":"drive_import","arguments":{"fileId":"owner-file"}});
        for _ in 0..3 {
            render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 2200.0),
                false,
                vec![],
            );
        }
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Approve & execute",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_connection_execute");
        assert_eq!(
            commands[0].payload["preparedActionId"],
            "connection-action:one"
        );
        assert_eq!(commands[0].payload["expectedActionRevision"], 1);
        data["projectStudioReceipt"] = json!({"projectId":"project:one","preparedActionId":"connection-action:one","revision":3,"outcome":"failed","reasonCode":"PROJECT_CONNECTION_OUTCOME_UNKNOWN"});
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 2200.0),
            false,
            vec![],
        );
        assert!(commands.is_empty());
        assert!(!copy.iter().any(|copy| copy == "Approve & execute"));
        assert!(copy.iter().any(|copy| copy.contains("uncertain")));
    }

    #[test]
    fn prepared_app_preview_reveals_exact_approval_at_minimum_size_in_both_languages_and_themes() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"]["language"] = json!(language);
                data["projectStudio"]["connections"] =
                    json!([{"providerId":"notion","label":"Test Notion","connected":true}]);
                data["projectStudioReceipt"] = json!({"projectId":"project:one","preparedActionId":"connection-action:one","revision":1,"outcome":"applied","reasonCode":"PROJECT_CONNECTION_PREPARED","providerId":"notion","operation":"notion_page","arguments":{"parentPageId":"owner-parent","title":"Reviewed title","content":"Reviewed content"}});
                let mut console = console(&data);
                console.create.studio.tab = 3;
                console.create.studio.connection_project = "project:one".into();
                console.create.studio.connection_provider = "notion".into();
                console.create.studio.arguments = HashMap::from([
                    ("parentPageId".into(), "owner-parent".into()),
                    ("title".into(), "Reviewed title".into()),
                    ("content".into(), "Reviewed content".into()),
                ]);
                console.create.studio.prepared =
                    Some(("project:one".into(), "connection-action:one".into(), 1));
                for _ in 0..4 {
                    assert!(render(&ctx, &mut console, &data, size, false, vec![])
                        .0
                        .is_empty());
                }
                let rect = ctx.data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "memory_control",
                        "Approve & execute",
                    )))
                    .unwrap()
                });
                let (clip, _) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Approve & execute",
                    )))
                    .unwrap()
                });
                assert!(
                    screen.contains_rect(rect) && clip.contains_rect(rect),
                    "{language}/{theme:?} {rect:?} {clip:?}"
                );
                let commands = super::super::tests::click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Approve & execute",
                );
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].payload["expectedActionRevision"], 1);
            }
        }
    }

    #[test]
    fn app_selector_and_frozen_arguments_use_readable_localized_rows_without_json() {
        for language in ["en", "tr"] {
            for (provider, operation, args, labels) in [
                (
                    "google_drive",
                    "drive_import",
                    json!({"fileId":"frozen-drive-file"}),
                    vec!["Drive file ID"],
                ),
                (
                    "notion",
                    "notion_page",
                    json!({"parentPageId":"frozen-parent","title":"Frozen title","content":"Frozen content"}),
                    vec!["Parent page ID", "Title", "Content"],
                ),
                (
                    "google_calendar",
                    "calendar_event",
                    json!({"calendarId":"frozen-calendar","summary":"Frozen event","description":"Frozen description","start":"2100-01-01T10:00:00Z","end":"2100-01-01T11:00:00Z","timeZone":"Europe/Istanbul"}),
                    vec![
                        "Calendar ID",
                        "Title",
                        "Description",
                        "Start time",
                        "End time",
                        "Time zone",
                    ],
                ),
            ] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                let mut data = fixture();
                data["desktopPreferences"]["language"] = json!(language);
                data["projectStudio"]["connections"] =
                    json!([{"providerId":provider,"label":"Test account","connected":true}]);
                data["projectStudioReceipt"] = json!({"projectId":"project:one","preparedActionId":"connection-action:one","revision":1,"outcome":"applied","reasonCode":"PROJECT_CONNECTION_PREPARED","providerId":provider,"operation":operation,"arguments":args});
                let mut console = console(&data);
                console.create.studio.tab = 3;
                console.create.studio.connection_project = "project:one".into();
                console.create.studio.connection_provider = provider.into();
                console.create.studio.arguments = args
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(|key| (key.clone(), format!("Editable {key}")))
                    .collect();
                let mut copy = Vec::new();
                for _ in 0..3 {
                    let (commands, frame) = render(
                        &ctx,
                        &mut console,
                        &data,
                        egui::vec2(1280.0, 2200.0),
                        false,
                        vec![],
                    );
                    assert!(commands.is_empty());
                    copy = frame;
                }
                assert!(
                    copy.iter().any(|line| line == app_name(provider)),
                    "friendly app name must be visible before preview: {language}/{provider}"
                );
                assert!(!copy.iter().any(|line| line == provider));
                console.create.studio.prepared =
                    Some(("project:one".into(), "connection-action:one".into(), 1));
                assert!(click(
                    &ctx,
                    &mut console,
                    &data,
                    false,
                    "memory_control",
                    "Prepare app operation"
                )
                .is_empty());
                for _ in 0..4 {
                    let (commands, frame) = render(
                        &ctx,
                        &mut console,
                        &data,
                        egui::vec2(1280.0, 2200.0),
                        false,
                        vec![],
                    );
                    assert!(commands.is_empty());
                    copy = frame;
                }
                for label in labels {
                    assert!(
                        copy.iter()
                            .any(|line| line
                                == i18n::tr(Language::from_code(language), label).as_ref()),
                        "missing localized {label}"
                    );
                }
                for value in args.as_object().unwrap().values() {
                    assert!(
                        copy.iter().any(|line| line == value.as_str().unwrap()),
                        "frozen argument omitted: {value}"
                    );
                }
                assert!(
                    !copy.join("\n").contains(&args.to_string()),
                    "normal preview must use field rows"
                );
            }
        }
    }

    #[test]
    fn credentials_clear_on_close_and_project_switch_and_oversized_png_is_rejected_before_decode() {
        let mut state = state();
        state.studio.open = true;
        state.studio.connection_project = state.project.clone();
        state.studio.token = "PRIVATE-CANARY".into();
        state.project = "project:other".into();
        state.sync_studio(&fixture());
        assert!(state.studio.token.is_empty());
        let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];
        bytes.extend_from_slice(&u32::MAX.to_be_bytes());
        bytes.extend_from_slice(&u32::MAX.to_be_bytes());
        assert!(!safe_png(&bytes));
    }
}
