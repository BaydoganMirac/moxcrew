use super::*;
#[path = "delivery.rs"]
mod delivery;
pub use delivery::DeliveryExportRequest;

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
struct ArtifactResourceRequest {
    command: String,
    project: String,
    scope: (String, String),
    files: Vec<(String, String, String)>,
    input: String,
    epoch: Option<Value>,
}
#[derive(Default)]
pub(super) struct StudioState {
    resource_bundle: Option<ArtifactResourceRequest>,
    pub(super) open: bool,
    pub(super) tab: usize,
    pub(super) requested: Option<(String, u64)>,
    selected: HashMap<String, (String, String, String)>,
    section: String,
    revisions: HashMap<(String, String, String, String), String>,
    page: usize,
    pub(super) path_request: Option<StudioPathRequest>,
    connection_project: String,
    connection_provider: String,
    label: String,
    token: String,
    arguments: HashMap<String, String>,
    prepared: Option<(String, String, u64)>,
    pending_prepare: Option<(String, String)>,
    executing: Option<(String, String)>,
    observed_receipt: String,
    files_project: String,
    query: String,
    file_type: usize,
    package_mode: bool,
    packages: HashMap<String, std::collections::BTreeSet<(String, String, String)>>,
    delivery_request: Option<DeliveryExportRequest>,
}

pub(super) fn studio_page<'a>(snapshot: &'a Value, project: &str) -> Option<&'a Value> {
    let page = &snapshot["projectStudio"];
    (text(page, "projectId") == project && text(page, "reasonCode") == "PROJECT_STUDIO_READ")
        .then_some(page)
}
#[cfg(test)]
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

#[derive(Debug, PartialEq, Eq)]
struct ArtifactVerification {
    status: &'static str,
    evidence: &'static str,
    count: Option<usize>,
    review: &'static str,
    inspected: bool,
}

fn artifact_verification(preview: &Value) -> ArtifactVerification {
    match text(preview, "kind") {
        "text" => ArtifactVerification {
            status: "Format inspected",
            evidence: "Readable text reopened from the exact file.",
            count: None,
            review: "Review factual accuracy, completeness and wording before use.",
            inspected: true,
        },
        "pages" if !array(&preview["pages"]).is_empty() => ArtifactVerification {
            status: "Format inspected",
            evidence: "{0} structured sections reopened from the exact file.",
            count: Some(array(&preview["pages"]).len()),
            review: "The outline does not verify rendered layout, clipping or typography.",
            inspected: true,
        },
        "image"
            if preview["pngBytes"]
                .as_array()
                .is_some_and(|bytes| !bytes.is_empty()) =>
        {
            ArtifactVerification {
                status: "Format inspected",
                evidence: "Exact image bytes loaded for the native preview.",
                count: None,
                review: "Review visual fidelity, crop, text and brand accuracy at full size.",
                inspected: true,
            }
        }
        "media"
            if preview["durationSeconds"]
                .as_f64()
                .is_some_and(|duration| duration.is_finite() && duration > 0.0)
                && !array(&preview["metadata"]["streams"]).is_empty() =>
        {
            ArtifactVerification {
                status: "Format inspected",
                evidence: "{0} media streams and duration parsed from the exact file.",
                count: Some(array(&preview["metadata"]["streams"]).len()),
                review: "Export and play the full file to review timing, audio and visual quality.",
                inspected: true,
            }
        }
        _ => ArtifactVerification {
            status: "Inspection limited",
            evidence: "The exact file version is available, but this format was not inspected.",
            count: None,
            review: "Export and inspect the file in a compatible application before use.",
            inspected: false,
        },
    }
}

fn artifact_verification_card(ui: &mut Ui, artifact: &Value, preview: &Value) {
    let verification = artifact_verification(preview);
    let accent = if verification.inspected {
        colors(ui).success
    } else {
        colors(ui).accent
    };
    let card = studio::accent_surface(ui, accent, |ui| {
        ui.set_min_width((ui.available_width() - 1.0).max(0.0));
        ui.horizontal_wrapped(|ui| {
            ui.label(studio::title(t(ui, "Verification"), 17.0));
            status(ui, "Exact version");
            status(ui, verification.status);
        });
        hint(
            ui,
            "The file hash matches the selected Project output version.",
        );
        let evidence = verification
            .count
            .map(|count| tf(ui, verification.evidence, &[&count.to_string()]))
            .unwrap_or_else(|| t(ui, verification.evidence).into_owned());
        raw_hint(ui, &evidence);
        hint(ui, verification.review);
        raw_hint(
            ui,
            &format!(
                "{} · {} B",
                text(artifact, "name"),
                artifact["bytes"].as_u64().unwrap_or(0)
            ),
        );
    });
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new("artifact-verification-card"),
            (card.response.rect, ui.clip_rect()),
        );
    });
    #[cfg(not(test))]
    let _ = card;
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

fn prepared_app<'a>(state: &StudioState, snapshot: &'a Value, project: &str) -> Option<&'a Value> {
    let (pending_project, action_id, rev) = state.prepared.as_ref()?;
    let receipt = &snapshot["projectStudioReceipt"];
    (pending_project == project
        && text(receipt, "projectId") == project
        && text(receipt, "preparedActionId") == action_id
        && revision(receipt) == *rev
        && text(receipt, "reasonCode") == "PROJECT_CONNECTION_PREPARED"
        && text(receipt, "outcome") == "applied")
        .then_some(receipt)
}
fn app_fields(provider: &str) -> (&str, &[(&str, &str)]) {
    match provider {
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
    }
}

fn save_connection(
    ui: &mut Ui,
    state: &mut StudioState,
    project: &str,
    commands: &mut Vec<Command>,
    primary: bool,
) {
    if view_control(
        ui,
        "Save connection",
        studio::commands_ready(ui)
            && valid_input(&state.token, 16_000)
            && valid_input(&state.label, 80),
        primary,
    )
    .clicked()
        && studio::commands_ready(ui)
        && commands.is_empty()
    {
        let token = std::mem::take(&mut state.token);
        egui::text_edit::TextEditState::default()
            .store(ui.ctx(), egui::Id::new("app-access-token"));
        commands.push(Command::new("project_connection_set",json!({"projectId":project,"providerId":state.connection_provider,"label":state.label,"accessToken":token})));
    }
}
fn connection_fields(
    ui: &mut Ui,
    state: &mut StudioState,
    project: &str,
    connection: Option<&Value>,
    commands: &mut Vec<Command>,
    save_inside: bool,
) {
    if save_inside {
        hint(
            ui,
            "Saved app credentials are unverified until an operation succeeds.",
        );
        if let Some(connection) = connection {
            if !text(connection, "label").is_empty() {
                ui.label(text(connection, "label"));
            }
            quiet_action(
                ui,
                commands,
                "Disconnect",
                "project_connection_remove",
                connection["connected"] == true,
                || json!({"projectId":project,"providerId":connection["providerId"]}),
            );
        }
    } else {
        ui.label(studio::title(t(ui, "Connect account"), 20.0));
    }
    field(ui, "Connection label", &mut state.label, 80, false);
    let token_id = egui::Id::new("app-access-token");
    ui.label(t(ui, "Access token"));
    let token_input = ui.add_sized(
        egui::vec2(ui.available_width(), studio::metrics::CONTROL_HEIGHT),
        egui::TextEdit::singleline(&mut state.token)
            .id(token_id)
            .password(true)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Body)
            .margin(studio::metrics::INPUT_MARGIN)
            .char_limit(16_000),
    );
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new("app-private-input"),
            (token_input.rect, token_input.id),
        )
    });
    #[cfg(not(test))]
    let _ = token_input;
    if let Some(mut edit) = egui::text_edit::TextEditState::load(ui.ctx(), token_id) {
        edit.clear_undoer();
        edit.store(ui.ctx(), token_id);
    }
    hint(
        ui,
        "The token stays private on this host. Connecting does not perform a remote operation.",
    );
    if save_inside {
        save_connection(ui, state, project, commands, false);
    }
}

impl CreateState {
    pub(crate) fn close_studio(&mut self) {
        self.studio.open = false;
        self.studio.token.clear();
    }

    pub(super) fn sync_studio(&mut self, snapshot: &Value) {
        if self.studio.files_project != self.project {
            self.studio.files_project = self.project.clone();
            self.studio.query.clear();
            self.studio.file_type = 0;
            self.studio.package_mode = false;
            self.studio.section.clear();
            self.studio.page = 0;
        }
        self.studio
            .packages
            .retain(|project, _| project_revision(snapshot, project).is_some());
        if project_revision(snapshot, &self.project).is_none() {
            self.studio.open = false;
            self.studio.token.clear();
            self.studio.prepared = None;
            self.studio.pending_prepare = None;
            self.studio.executing = None;
        }

        let receipt = &snapshot["projectStudioReceipt"];
        if text(receipt, "projectId") == self.project
            && !text(receipt, "commandId").is_empty()
            && self.studio.observed_receipt != text(receipt, "commandId")
            && !matches!(text(receipt, "outcome"), "" | "unknown")
        {
            self.studio.observed_receipt = text(receipt, "commandId").into();
            if text(receipt, "outcome") == "applied"
                && matches!(
                    text(receipt, "action"),
                    "project_resource_import"
                        | "project_resource_import_batch"
                        | "project_resource_from_artifact"
                        | "project_resources_from_artifacts"
                        | "project_image_engine_set"
                        | "project_image_engine_remove"
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

        if !self.studio.open {
            self.studio.token.clear();
        }
        if self.studio.connection_project != self.project {
            self.studio.token.clear();
            self.studio.connection_project = self.project.clone();
            self.studio.prepared = None;
            self.studio.pending_prepare = None;
            self.studio.executing = None;
            self.studio.arguments.clear();
            self.studio.label.clear();
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
    pub(super) fn studio_canvas_smoke(&mut self, snapshot: &Value) {
        self.create.select_productive_view(2);
        self.create.studio.tab = 0;
        if let Some(artifact) = studio_page(snapshot, &self.create.project).and_then(|p| {
            array(&p["artifacts"])
                .iter()
                .find(|a| text(a, "path").to_ascii_lowercase().ends_with(".png"))
        }) {
            self.create
                .studio
                .selected
                .insert(self.create.project.clone(), artifact_key(artifact));
        }
    }
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
    pub(super) fn output_studio(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if !self.create.studio.open {
            self.create.studio.token.clear();
            return;
        }
        let project = self.create.project.clone();
        let Some(rev) = project_revision(snapshot, &project) else {
            self.create.close_studio();
            return;
        };
        let key = (project.clone(), rev);
        if ui.is_enabled()
            && !self.hold_automatic_reads
            && self.create.studio.requested.as_ref() != Some(&key)
            && studio::commands_ready(ui)
            && commands.is_empty()
        {
            self.create.studio.requested = Some(key);
            commands.push(Command::new(
                "project_studio_read",
                json!({"projectId":project}),
            ));
        }
        inline_surface(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(studio::title(
                    t(
                        ui,
                        if self.create.studio.tab == 0 && self.create.studio.package_mode {
                            "Delivery package"
                        } else {
                            "Outputs"
                        },
                    ),
                    20.0,
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let before = self.create.studio.tab;
                    let labels = ["Files", "Resources", "Tools", "Apps"];
                    let picker = egui::ComboBox::from_id_salt("studio-context-view")
                        .width(140.0)
                        .selected_text(t(ui, labels[self.create.studio.tab.min(3)]))
                        .show_ui(ui, |ui| {
                            for (index, label) in labels.iter().enumerate() {
                                let response = ui.selectable_value(
                                    &mut self.create.studio.tab,
                                    index,
                                    t(ui, label),
                                );
                                #[cfg(test)]
                                ui.data_mut(|data| {
                                    data.insert_temp(
                                        egui::Id::new(("studio-view-tab", index)),
                                        (response.rect, ui.clip_rect(), response.id),
                                    )
                                });
                                #[cfg(not(test))]
                                let _ = response;
                            }
                        });
                    #[cfg(test)]
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new(("test-action", "memory_control", "Output view")),
                            picker.response.rect,
                        )
                    });
                    #[cfg(not(test))]
                    let _ = picker;
                    if before != self.create.studio.tab {
                        self.create.studio.token.clear();
                    }
                    quiet_action(
                        ui,
                        commands,
                        "Refresh",
                        "project_studio_read",
                        !self.hold_automatic_reads,
                        || json!({"projectId":project}),
                    );
                });
            });
            ui.add_space(studio::metrics::GAP);
            let height = (ui.available_height()
                - if self.create.studio.tab == 0 && self.create.studio.package_mode {
                    delivery::footer_height(ui, &self.create, snapshot)
                } else {
                    studio::metrics::CONTROL_HEIGHT
                        + studio::metrics::CONTROL_GAP
                        + studio::metrics::GAP
                })
            .max(60.0);
            egui::ScrollArea::vertical()
                .id_salt(("output-studio-body", &project, self.create.studio.tab))
                .auto_shrink([false, false])
                .max_height(height)
                .show(ui, |ui| {
                    if let Some(page) = studio_page(snapshot, &project) {
                        match self.create.studio.tab {
                            1 => self.studio_resources(ui, snapshot, page, commands),
                            2 => self.studio_tools(ui, snapshot, page, commands),
                            3 => self.studio_apps(ui, snapshot, page, commands),
                            _ if self.create.studio.package_mode => {
                                self.studio_delivery(ui, snapshot, page)
                            }
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
            ui.add_space(studio::metrics::CONTROL_GAP);
            self.studio_footer(ui, snapshot, commands);
        });
    }
    fn studio_resource_bundle_action(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: &Value,
        commands: &mut Vec<Command>,
    ) {
        let files: Vec<_> = self
            .create
            .studio
            .packages
            .get(&self.create.project)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        let mut bytes = 0u64;
        let valid = !files.is_empty()
            && files.len() <= 12
            && files.iter().all(|key| {
                array(&page["artifacts"])
                    .iter()
                    .find(|a| artifact_key(a) == *key)
                    .is_some_and(|a| {
                        bytes = bytes.saturating_add(a["bytes"].as_u64().unwrap_or(u64::MAX));
                        capabilities::valid_hash(&key.2)
                    })
            })
            && bytes <= 25 * 1024 * 1024;
        let before = commands.len();
        action(
            ui,
            commands,
            "Use selected files in Chat",
            "project_resources_from_artifacts",
            valid && self.create.studio.resource_bundle.is_none(),
            || json!({"projectId":self.create.project,"expectedProjectRevision":project_revision(snapshot,&self.create.project),"files":files.iter().map(|(work,path,hash)|json!({"workItemId":work,"path":path,"expectedSha256":hash})).collect::<Vec<_>>()}),
        );
        if commands.len() > before {
            self.create.studio.resource_bundle = Some(ArtifactResourceRequest {
                command: text(&commands.last().unwrap().payload, "commandId").into(),
                project: self.create.project.clone(),
                scope: (self.create.project.clone(), self.create.ability.clone()),
                files,
                input: self.create.conversation_input().clone(),
                epoch: self.create.host_epoch.clone(),
            });
        }
        hint(
            ui,
            "Use up to 12 files, 25 MiB total, as frozen sources for a new conversation request.",
        );
    }
    pub(super) fn sync_artifact_resources(
        &mut self,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        let Some(pending) = &self.create.studio.resource_bundle else {
            return;
        };
        let receipt = &snapshot["projectStudioReceipt"];
        if text(receipt, "commandId") != pending.command
            || text(receipt, "projectId") != pending.project
            || matches!(text(receipt, "outcome"), "" | "unknown")
        {
            return;
        }
        if text(receipt, "outcome") != "applied" {
            self.create.studio.resource_bundle = None;
            return;
        }
        if !capabilities::complete_import(receipt, pending.files.len()) {
            return;
        }
        let pending = self.create.studio.resource_bundle.take().unwrap();
        if pending.epoch != self.create.host_epoch
            || pending.scope != (self.create.project.clone(), self.create.ability.clone())
            || self.create.conversation_input() != &pending.input
        {
            return;
        }
        let refs = array(&receipt["resources"])
            .iter()
            .map(|r| format!("{} {}", text(r, "resourceId"), text(r, "sha256")))
            .collect::<Vec<_>>()
            .join("\n");
        let input = format!(
            "{}\n\nUse these exact Project resources:\n{refs}",
            pending.input
        );
        if input.encode_utf16().count() > self.create.conversation_limit() {
            self.create.chat.conversation.set_error("This request is too long for the current conversation. Start a new conversation or shorten it.");
            return;
        }
        *self.create.conversation_input() = input;
        self.create.crew.single = false;
        self.create.select_productive_view(0);
        let _ = commands;
        if let Some(cart) = self.create.studio.packages.get_mut(&pending.project) {
            for key in pending.files {
                cart.remove(&key);
            }
        }
    }
    fn studio_footer(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        match self.create.studio.tab {
            0 => {
                if self.create.studio.package_mode {
                    self.studio_delivery_footer(ui, snapshot, commands);
                    return;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(artifact) = selected_artifact(&self.create, snapshot).cloned() {
                        let has_preview = artifact_preview(&self.create, snapshot, &artifact).is_some();
                        if has_preview {
                            if view_control(ui, "Revise in Chat", true, true).clicked() {
                                let section = artifact_preview(&self.create, snapshot, &artifact)
                                    .and_then(|preview| array(&preview["sections"]).iter().find(|section| text(section, "id") == self.create.studio.section))
                                    .map(|section| text(section, "id").to_owned())
                                    .unwrap_or_default();
                                self.use_visual_in_chat(&artifact, section);
                            }
                        } else {
                            let response = view_control(ui, "Preview", studio::commands_ready(ui) && !cfg!(target_os = "windows"), true);
                            #[cfg(test)]
                            ui.data_mut(|data| data.insert_temp(egui::Id::new(("test-action", "project_artifact_read", "Preview")), response.rect));
                            if response.clicked() && studio::commands_ready(ui) && commands.is_empty() {
                                commands.push(Command::new("project_artifact_read", json!({"projectId":self.create.project,"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"]})));
                            }
                        }
                        if view_control(ui, "Export copy", studio::commands_ready(ui) && !cfg!(target_os = "windows"), false).clicked() {
                            self.create.export_request = Some(CreateExportRequest {
                                project_id: self.create.project.clone(),
                                work_item_id: text(&artifact, "workItemId").into(),
                                path: text(&artifact, "path").into(),
                                name: text(&artifact, "name").into(),
                                expected_sha256: Some(text(&artifact, "sha256").into()),
                            });
                        }
                    }
                });
            }
            1 => {
                if view_control(ui, "Import resource", studio::commands_ready(ui), true).clicked() {
                    self.create.studio.path_request = Some(StudioPathRequest {
                        project_id: self.create.project.clone(),
                        expected_project_revision: project_revision(snapshot, &self.create.project)
                            .unwrap_or(0),
                        kind: StudioPathKind::Resource,
                    });
                }
            }
            3 => self.studio_app_action(ui, snapshot, commands),
            _ => {}
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
        delivery::file_filters(ui, &mut self.create.studio);
        ui.add_space(studio::metrics::GAP);
        if !artifacts.iter().any(|artifact| {
            delivery::matches_file(
                artifact,
                &self.create.studio.query,
                self.create.studio.file_type,
            )
        }) {
            hint(ui, "No matching outputs.");
        }
        if selected_artifact(&self.create, snapshot).is_some_and(|artifact| {
            !delivery::matches_file(
                artifact,
                &self.create.studio.query,
                self.create.studio.file_type,
            )
        }) {
            hint(ui, "The selected output is outside this filter.");
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
        let package_width = ui
            .painter()
            .layout_no_wrap(
                t(ui, "Create package").into_owned(),
                egui::FontId::proportional(14.0),
                colors(ui).text,
            )
            .size()
            .x
            + 32.0;
        ui.horizontal(|ui| {
            form_combo(
                ui,
                egui::ComboBox::from_id_salt("studio-artifact-version")
                    .selected_text(current)
                    .width(
                        (ui.available_width() - package_width - studio::metrics::GAP)
                            .clamp(120.0, 540.0),
                    )
                    .height(220.0),
                |ui| {
                    let mut count = 0;
                    for artifact in artifacts.iter().filter(|artifact| {
                        delivery::matches_file(
                            artifact,
                            &self.create.studio.query,
                            self.create.studio.file_type,
                        )
                    }) {
                        count += 1;
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
                    if count == 0 {
                        hint(ui, "No matching outputs.");
                    }
                },
            );
            if view_control(ui, "Create package", true, false).clicked() {
                self.create.studio.package_mode = true;
            }
        });
        let Some(artifact) = selected_artifact(&self.create, snapshot).cloned() else {
            hint(ui, "Choose an output to preview or revise.");
            return;
        };
        ui.add_space(studio::metrics::CONTROL_GAP);
        if text(&artifact, "path")
            .to_ascii_lowercase()
            .ends_with(".html")
        {
            quiet_action(
                ui,
                commands,
                "Open interactive output",
                "project_artifact_open",
                !cfg!(target_os = "windows"),
                || json!({"projectId":self.create.project,"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"]}),
            );
        }
        if let Some(preview) = artifact_preview(&self.create, snapshot, &artifact) {
            artifact_verification_card(ui, &artifact, &preview["preview"]);
            ui.add_space(studio::metrics::GAP);
            self.studio_visual_preview(ui, snapshot, &artifact, &preview["preview"], commands);
            if text(&preview["preview"], "kind") == "text" {
                quiet_action(
                    ui,
                    commands,
                    "Use text as Project resource",
                    "project_resource_from_artifact",
                    true,
                    || json!({"projectId":self.create.project,"expectedProjectRevision":project_revision(snapshot,&self.create.project),"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"]}),
                );
            }
            ui.add_space(studio::metrics::SECTION_GAP);
            let key = (
                self.create.project.clone(),
                text(&artifact, "workItemId").into(),
                text(&artifact, "path").into(),
                text(&artifact, "sha256").into(),
            );
            let has_draft = self
                .create
                .studio
                .revisions
                .get(&key)
                .is_some_and(|draft| !draft.is_empty());
            let revision_ui=egui::CollapsingHeader::new(t(ui,"Revise this output")).default_open(has_draft).show(ui,|ui| {
                let sections=array(&preview["sections"]);
                form_combo(ui,egui::ComboBox::from_id_salt("revision-section").selected_text(sections.iter().find(|section|text(section,"id")==self.create.studio.section).map(|section|std::borrow::Cow::Borrowed(text(section,"label"))).unwrap_or_else(||t(ui,"Whole output"))),|ui| {
                    ui.selectable_value(&mut self.create.studio.section,String::new(),t(ui,"Whole output"));
                    for section in sections {ui.selectable_value(&mut self.create.studio.section,text(section,"id").into(),text(section,"label"));}
                });
                let instruction=self.create.studio.revisions.entry(key).or_default();
                multiline(ui,"Revision instructions",instruction,12_000,2);
                let valid=valid_input(instruction,12_000);
                let mut payload=json!({"projectId":self.create.project,"expectedProjectRevision":project_revision(snapshot,&self.create.project),"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"],"instruction":instruction.trim()});
                if !self.create.studio.section.is_empty(){payload["sectionId"]=json!(self.create.studio.section);}
                let provider=self.create.resolved_planner_provider(snapshot);
                if let Some(provider)=provider {payload["providerId"]=json!(provider);}
                action(ui,commands,"Prepare revision","project_artifact_revise",valid&&provider.is_some(),||payload);
                hint(ui,"A revision prepares a new task. The original stays intact; start it explicitly in Tasks.");
            });
            #[cfg(test)]
            ui.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("test-action", "memory_control", "Revise this output")),
                    revision_ui.header_response.rect,
                )
            });
            #[cfg(not(test))]
            let _ = revision_ui;
        } else {
            hint(ui, "Preview is loaded only for this exact file version.");
        }
        ui.add_space(studio::metrics::CONTROL_GAP);
        ui.collapsing(t(ui, "Version & provenance"), |ui| {
            for (key, label) in [
                ("versionId", "Version"),
                ("parentVersionId", "Previous version"),
                ("workItemId", "Task"),
                ("path", "File path"),
                ("sha256", "File hash"),
            ] {
                if !text(&artifact, key).is_empty() {
                    ui.label(
                        RichText::new(t(ui, label))
                            .size(12.0)
                            .color(colors(ui).muted),
                    );
                    ui.add(
                        egui::Label::new(text(&artifact, key))
                            .wrap()
                            .selectable(true),
                    );
                }
            }
        });
    }
    fn use_visual_in_chat(&mut self, artifact: &Value, section: String) {
        let key = (self.create.project.clone(), self.create.ability.clone());
        self.create.capabilities.selection.insert(
            key,
            capabilities::VisualSelection {
                project: self.create.project.clone(),
                work: text(artifact, "workItemId").into(),
                path: text(artifact, "path").into(),
                hash: text(artifact, "sha256").into(),
                section,
            },
        );
        self.create.crew.single = false;
        self.create.select_productive_view(0);
    }
    fn studio_visual_preview(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        artifact: &Value,
        preview: &Value,
        commands: &mut Vec<Command>,
    ) {
        let key = canvas::CanvasKey {
            project: self.create.project.clone(),
            work: text(artifact, "workItemId").into(),
            path: text(artifact, "path").into(),
            hash: text(artifact, "sha256").into(),
            part: String::new(),
        };
        match text(preview, "kind") {
            "text" => {
                ui.add(
                    egui::Label::new(text(preview, "text"))
                        .selectable(true)
                        .wrap(),
                );
            }
            "pages" => {
                let pages = array(&preview["pages"]);
                self.create.studio.page =
                    self.create.studio.page.min(pages.len().saturating_sub(1));
                form_combo(
                    ui,
                    egui::ComboBox::from_id_salt("visual-document-page").selected_text(
                        pages
                            .get(self.create.studio.page)
                            .map(|p| text(p, "title"))
                            .unwrap_or(t(ui, "Page").as_ref()),
                    ),
                    |ui| {
                        for (i, page) in pages.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.create.studio.page,
                                i,
                                text(page, "title"),
                            );
                        }
                    },
                );
                let page = self.create.studio.page as u64 + 1;
                quiet_action(
                    ui,
                    commands,
                    "Render this page",
                    "project_artifact_visual_read",
                    !cfg!(target_os = "windows"),
                    || json!({"projectId":self.create.project,"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"],"page":page}),
                );
                let visual = &snapshot["projectArtifactVisual"];
                let exact = canvas::matches_read(visual, &key, "PROJECT_ARTIFACT_VISUAL_READ")
                    && visual["page"] == page;
                if exact {
                    if let Some(texture) = canvas::texture(
                        ui,
                        &visual["preview"],
                        &format!("{}:page:{page}", key.hash),
                    ) {
                        ui.add(
                            egui::Image::new(&texture)
                                .max_size(egui::vec2(ui.available_width(), 260.0)),
                        );
                        if view_control(ui, "Use page in Chat", true, false).clicked() {
                            self.use_visual_in_chat(artifact, format!("page:{page}"));
                        }
                    } else {
                        hint(ui, text(visual, "reasonCode"));
                        hint(
                            ui,
                            "The renderer is unavailable. The text preview remains available.",
                        );
                    }
                } else {
                    hint(ui,"Render a page for visual selection. The text below is a structural preview.");
                }
                if let Some(page) = pages.get(self.create.studio.page) {
                    ui.add(egui::Label::new(text(page, "text")).wrap().selectable(true));
                }
            }
            "image" => {
                if let Some(texture) = canvas::texture(ui, preview, &key.hash) {
                    canvas::select_image(ui, &texture, &key, &mut self.create.canvas);
                    let section = self.create.canvas.draft(&key).region.map(canvas::region_id);
                    if view_control(ui, "Use region in Chat", section.is_some(), false).clicked() {
                        self.use_visual_in_chat(artifact, section.unwrap());
                    }
                    hint(
                        ui,
                        "Drag to select a region. Arrow keys move it; Shift adjusts its size.",
                    );
                } else {
                    hint(
                        ui,
                        "Native preview is unavailable for this format. Export the original.",
                    );
                }
            }
            "media" => {
                if let Some(texture) = canvas::texture(ui, preview, &key.hash) {
                    ui.add(
                        egui::Image::new(&texture)
                            .max_size(egui::vec2(ui.available_width(), 220.0)),
                    );
                }
                if let Some(duration) = preview["durationSeconds"]
                    .as_f64()
                    .filter(|d| d.is_finite() && *d > 0.0 && text(artifact, "kind") == "video")
                {
                    let draft = self.create.canvas.draft(&key);
                    let (mut start, mut end) = draft.range.unwrap_or((0.0, duration.min(10.0)));
                    ui.label(t(ui, "Video range (seconds)"));
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut start)
                                .range(0.0..=duration)
                                .speed(0.1),
                        );
                        ui.label("–");
                        ui.add(
                            egui::DragValue::new(&mut end)
                                .range(0.0..=duration)
                                .speed(0.1),
                        );
                    });
                    draft.range = Some((start, end));
                    quiet_action(
                        ui,
                        commands,
                        "Preview selected frame",
                        "project_artifact_frame_read",
                        start <= duration,
                        || json!({"projectId":self.create.project,"workItemId":artifact["workItemId"],"path":artifact["path"],"expectedSha256":artifact["sha256"],"time":start}),
                    );
                    let frame = &snapshot["projectArtifactFrame"];
                    if canvas::matches_read(frame, &key, "PROJECT_ARTIFACT_FRAME_READ")
                        && frame["time"].as_f64() == Some(start)
                    {
                        if let Some(texture) = canvas::texture(
                            ui,
                            &frame["preview"],
                            &format!("{}:frame:{start}", key.hash),
                        ) {
                            ui.add(
                                egui::Image::new(&texture)
                                    .max_size(egui::vec2(ui.available_width(), 220.0)),
                            );
                        }
                    }
                    let range = canvas::range_id(start, end, duration);
                    if view_control(ui, "Use range in Chat", range.is_some(), false).clicked() {
                        self.use_visual_in_chat(artifact, range.unwrap());
                    }
                }
                hint(
                    ui,
                    "Media metadata and thumbnail. Export the original for playback or editing.",
                );
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
        for resource in array(&page["resources"]) {
            ui.push_id(text(resource,"resourceId"),|ui| {
                ui.label(RichText::new(text(resource,"name")).strong());
                raw_hint(ui,&format!("{} · {} B",text(resource,"kind"),resource["bytes"].as_u64().unwrap_or(0)));
                status(ui,text(resource,"status"));
                quiet_action(ui,commands,"Read resource","project_resource_read",text(resource,"status")=="available",||json!({"projectId":self.create.project,"resourceId":resource["resourceId"],"expectedSha256":resource["sha256"]}));
                let read=&snapshot["projectResource"];
                let exact=text(read,"reasonCode")=="PROJECT_RESOURCE_READ" && text(read,"projectId")==self.create.project && read["resourceId"]==resource["resourceId"] && (read["sha256"]==resource["sha256"] || read["resource"]["sha256"]==resource["sha256"]);
                if exact {
                    let preview=&read["preview"];
                    if let Some(text)=preview["text"].as_str(){ui.add(egui::Label::new(text).wrap().selectable(true));}
                    let key=canvas::CanvasKey{project:self.create.project.clone(),work:text(resource,"resourceId").into(),path:String::new(),hash:text(resource,"sha256").into(),part:"resource".into()};
                    if let Some(texture)=canvas::texture(ui,preview,&key.hash){canvas::select_image(ui,&texture,&key,&mut self.create.canvas);multiline(ui,"Annotation",&mut self.create.canvas.draft(&key).note,2000,2);let annotation=self.create.canvas.draft(&key).clone();if view_control(ui,"Add annotation to Chat",studio::commands_ready(ui) && valid_input(&annotation.note,2000),false).clicked(){self.stage_operation(snapshot,"Apply this image annotation to the exact frozen resource.",json!({"resourceId":resource["resourceId"],"expectedSha256":resource["sha256"],"region":annotation.region.map(canvas::region_id),"annotation":annotation.note}),commands);}}
                    ui.horizontal_wrapped(|ui|{for(label,operation)in [("Extract text (OCR)","resources.ocr"),("Transcribe audio","audio.transcribe")]{if view_control(ui,label,studio::commands_ready(ui),false).clicked(){self.stage_operation(snapshot,"Extract real text from this Project resource; report missing tools honestly.",json!({"operation":operation,"arguments":{"source":resource["resourceId"]},"expectedSha256":resource["sha256"]}),commands);}}});
                }

                ui.collapsing(t(ui,"Provenance"),|ui| {ui.monospace(text(resource,"sha256"));});
                quiet_action(ui,commands,"Remove resource","project_resource_remove",revision(resource)>0,||json!({"projectId":self.create.project,"resourceId":resource["resourceId"],"expectedResourceRevision":resource["revision"]}));
                ui.add_space(f32::from(studio::metrics::PADDING));
            });
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
                ui.vertical(|ui| {
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
                    });
                    egui::CollapsingHeader::new(t(ui, "Setup details"))
                        .default_open(tool["available"] != true)
                        .show(ui, |ui| {
                            if view_control(
                                ui,
                                "Choose executable",
                                studio::commands_ready(ui) && !text(tool, "id").is_empty(),
                                false,
                            )
                            .clicked()
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
                            hint(ui, text(tool, "reasonCode"));
                        });
                    ui.add_space(f32::from(studio::metrics::PADDING));
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
        let project = self.create.project.clone();
        let state = &mut self.create.studio;
        if state.connection_provider.is_empty() {
            state.connection_provider = "google_drive".into();
        }
        let previous = state.connection_provider.clone();
        form_combo(
            ui,
            egui::ComboBox::from_id_salt("app-provider")
                .width(220.0)
                .selected_text(app_name(&state.connection_provider)),
            |ui| {
                for provider in ["google_drive", "notion", "google_calendar"] {
                    ui.selectable_value(
                        &mut state.connection_provider,
                        provider.into(),
                        app_name(provider),
                    );
                }
            },
        );
        if previous != state.connection_provider {
            state.token.clear();
            state.prepared = None;
            state.pending_prepare = None;
            state.arguments.clear();
        }
        let connection = array(&page["connections"])
            .iter()
            .find(|connection| text(connection, "providerId") == state.connection_provider);
        let connected = connection.is_some_and(|connection| connection["connected"] == true);
        hint(
            ui,
            if connected {
                "Credentials saved"
            } else {
                "Not connected"
            },
        );
        ui.add_space(studio::metrics::CONTROL_GAP);
        let prepared = prepared_app(state, snapshot, &project).cloned();
        if let Some(receipt) = prepared {
            ui.label(studio::title(
                t(ui, app_operation_name(text(&receipt, "operation"))),
                20.0,
            ));
            hint(ui, "Review the exact remote operation before approving.");
            ui.add_space(studio::metrics::CONTROL_GAP);
            if let Some(preview) = receipt["preview"].as_str() {
                ui.add(egui::Label::new(preview).wrap().selectable(true));
            }
            app_values(ui, &receipt["arguments"]);
        } else if let Some((executing_project, id)) = &state.executing {
            let receipt = &snapshot["projectStudioReceipt"];
            if text(receipt, "projectId") == executing_project
                && text(receipt, "preparedActionId") == id
            {
                hint(ui, text(receipt, "reasonCode"));
                app_values(ui, &receipt["result"]);
                if text(receipt, "reasonCode") == "PROJECT_CONNECTION_OUTCOME_UNKNOWN" {
                    hint(ui,"Remote outcome is uncertain. Check the connected app before preparing another operation.");
                }
            }
        } else if !connected {
            connection_fields(ui, state, &project, connection, commands, false);
        } else {
            ui.label(studio::title(
                t(
                    ui,
                    app_operation_name(app_fields(&state.connection_provider).0),
                ),
                20.0,
            ));
            let before = state.arguments.clone();
            for (key, label) in app_fields(&state.connection_provider).1 {
                if matches!(*key, "content" | "description") {
                    multiline(
                        ui,
                        label,
                        state.arguments.entry((*key).into()).or_default(),
                        20_000,
                        2,
                    );
                } else {
                    field(
                        ui,
                        label,
                        state.arguments.entry((*key).into()).or_default(),
                        20_000,
                        false,
                    );
                }
            }
            if before != state.arguments {
                state.prepared = None;
                state.pending_prepare = None;
            }
            if !connected {
                hint(ui, "Connect this app before previewing an operation.");
            }
        }
        if connected {
            ui.add_space(studio::metrics::SECTION_GAP);
            ui.collapsing(t(ui, "Connection settings"), |ui| {
                connection_fields(ui, state, &project, connection, commands, true)
            });
        }
    }
    fn studio_app_action(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let project = self.create.project.clone();
        let state = &mut self.create.studio;
        if let Some(receipt) = prepared_app(state, snapshot, &project).cloned() {
            let action_id = text(&receipt, "preparedActionId");
            let rev = revision(&receipt);
            ui.horizontal_wrapped(|ui| {
                if view_control(ui,"Approve & execute",studio::commands_ready(ui) && state.executing.as_ref().is_none_or(|(_,id)|id!=action_id),true).clicked() && studio::commands_ready(ui) && commands.is_empty() {
                    state.executing=Some((project.clone(),action_id.into()));
                    commands.push(Command::new("project_connection_execute",json!({"projectId":project,"preparedActionId":action_id,"expectedActionRevision":rev})));
                }
                if view_control(ui,"Edit operation",true,false).clicked() {state.prepared=None;state.pending_prepare=None;}
            });
        } else if state.executing.is_some() {
            if view_control(ui, "New operation", true, false).clicked() {
                state.executing = None;
                state.prepared = None;
                state.pending_prepare = None;
            }
        } else {
            let (operation, fields) = app_fields(&state.connection_provider);
            let connected = studio_page(snapshot, &project).is_some_and(|page| {
                array(&page["connections"]).iter().any(|connection| {
                    text(connection, "providerId") == state.connection_provider
                        && connection["connected"] == true
                })
            });
            if !connected {
                save_connection(ui, state, &project, commands, true);
                return;
            }
            let enabled = connected
                && fields.iter().all(|(key, _)| {
                    state
                        .arguments
                        .get(*key)
                        .is_some_and(|value| !value.trim().is_empty())
                })
                && state.pending_prepare.is_none();
            if view_control(
                ui,
                "Preview operation",
                enabled && studio::commands_ready(ui),
                true,
            )
            .clicked()
                && studio::commands_ready(ui)
                && commands.is_empty()
            {
                let command = Command::new(
                    "project_connection_prepare",
                    json!({"projectId":project,"providerId":state.connection_provider,"operation":operation,"arguments":state.arguments}),
                );
                state.prepared = None;
                state.pending_prepare = Some((text(&command.payload, "commandId").into(), project));
                commands.push(command);
            }
        }
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
    fn verification_summary_separates_exact_bytes_format_evidence_and_human_review() {
        for (preview, status, evidence, count, inspected) in [
            (
                json!({"kind":"text","text":"Owner text"}),
                "Format inspected",
                "Readable text reopened from the exact file.",
                None,
                true,
            ),
            (
                json!({"kind":"pages","pages":[{"id":"slide:1"},{"id":"slide:2"}]}),
                "Format inspected",
                "{0} structured sections reopened from the exact file.",
                Some(2),
                true,
            ),
            (
                json!({"kind":"image","pngBytes":[1,2,3]}),
                "Format inspected",
                "Exact image bytes loaded for the native preview.",
                None,
                true,
            ),
            (
                json!({"kind":"media","durationSeconds":2.5,"metadata":{"streams":[{"codec_type":"video"},{"codec_type":"audio"}]}}),
                "Format inspected",
                "{0} media streams and duration parsed from the exact file.",
                Some(2),
                true,
            ),
            (
                json!({"kind":"unsupported","reasonCode":"PROJECT_PREVIEW_UNSUPPORTED"}),
                "Inspection limited",
                "The exact file version is available, but this format was not inspected.",
                None,
                false,
            ),
        ] {
            let summary = artifact_verification(&preview);
            assert_eq!(summary.status, status);
            assert_eq!(summary.evidence, evidence);
            assert_eq!(summary.count, count);
            assert_eq!(summary.inspected, inspected);
            assert!(!summary.review.is_empty());
        }
        assert!(!artifact_verification(&json!({"kind":"pages","pages":[]})).inspected);
        assert!(
            !artifact_verification(
                &json!({"kind":"media","durationSeconds":0,"metadata":{"streams":[]}})
            )
            .inspected
        );
    }

    #[test]
    fn exact_verification_card_is_localized_and_visible_at_minimum_size() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"]["language"] = json!(language);
                data["projectArtifact"] = json!({
                    "reasonCode":"PROJECT_ARTIFACT_READ",
                    "projectId":"project:one",
                    "workItemId":"work:one",
                    "path":"outputs/deck.pptx",
                    "sha256":"hash-one",
                    "preview":{"kind":"pages","pages":[{"id":"slide:1","title":"Launch","text":"Evidence"}]},
                    "sections":[{"id":"slide:1","label":"Launch"}]
                });
                let mut console = console(&data);
                let size = egui::vec2(800.0, 560.0);
                let mut copy = Vec::new();
                for _ in 0..3 {
                    let (commands, painted) =
                        render(&ctx, &mut console, &data, size, false, vec![]);
                    assert!(commands.is_empty());
                    copy = painted;
                }
                for key in [
                    "Verification",
                    "Exact version",
                    "Format inspected",
                    "The file hash matches the selected Project output version.",
                    "The outline does not verify rendered layout, clipping or typography.",
                ] {
                    assert!(
                        copy.iter()
                            .any(|line| line
                                == i18n::tr(Language::from_code(language), key).as_ref()),
                        "{language}/{theme:?}: missing {key}"
                    );
                }
                let (card, clip): (egui::Rect, egui::Rect) = ctx.data(|data| {
                    data.get_temp(egui::Id::new("artifact-verification-card"))
                        .unwrap()
                });
                assert!(
                    card.intersects(clip)
                        && egui::Rect::from_min_size(egui::Pos2::ZERO, size)
                            .contains_rect(card.intersect(clip)),
                    "{language}/{theme:?}: verification evidence must stay reachable: card={card:?} clip={clip:?}"
                );
            }
        }
    }

    #[test]
    fn revise_in_chat_preserves_exact_selected_version_and_never_starts_work() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectArtifact"] = json!({"reasonCode":"PROJECT_ARTIFACT_READ","projectId":"project:one","workItemId":"work:one","path":"outputs/deck.pptx","sha256":"hash-one","preview":{"kind":"text","text":"Original content"},"sections":[{"id":"slide:2","label":"Slide 2"}]});
        let original = data["projectStudio"]["artifacts"].clone();
        let mut console = console(&data);
        render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            vec![],
        );
        console.create.studio.section = "slide:2".into();
        *console.create.conversation_input() = "Keep my draft".into();
        let commands = super::super::tests::click_at_size(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            "memory_control",
            "Revise in Chat",
        );
        assert!(commands.is_empty());
        assert_eq!(console.create.productive_view(), 0);
        let selected = console.create.visual_selection().unwrap();
        assert_eq!(selected.project, "project:one");
        assert_eq!(selected.work, "work:one");
        assert_eq!(selected.path, "outputs/deck.pptx");
        assert_eq!(selected.hash, "hash-one");
        assert_eq!(selected.section, "slide:2");
        assert_eq!(
            console.create.conversation_input().as_str(),
            "Keep my draft"
        );
        assert_eq!(data["projectStudio"]["artifacts"], original);
    }

    #[test]
    fn disconnected_apps_foreground_private_setup_and_visible_explicit_save_at_minimum_size() {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                for provider in ["google_drive", "notion", "google_calendar"] {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    let mut console = console(&data);
                    console.create.studio.tab = 3;
                    console.create.studio.connection_project = "project:one".into();
                    console.create.studio.connection_provider = provider.into();
                    console.create.studio.label = "Owner connection".into();
                    console.create.studio.token = "SYNTHETIC-PRIVATE-CANARY".into();
                    console
                        .create
                        .studio
                        .arguments
                        .insert("content".into(), "Unsent operation draft".into());
                    let size = egui::vec2(800.0, 560.0);
                    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                    for _ in 0..3 {
                        let (commands, copy) =
                            render(&ctx, &mut console, &data, size, false, vec![]);
                        assert!(commands.is_empty());
                        assert!(copy.iter().any(|line| line
                            == i18n::tr(Language::from_code(language), "Connect account")
                                .as_ref()));
                        assert!(!copy.iter().any(|line| line
                            == i18n::tr(Language::from_code(language), "Preview operation")
                                .as_ref()));
                        assert!(!copy.join("\n").contains("SYNTHETIC-PRIVATE-CANARY"));
                    }
                    let (input, _) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, egui::Id)>(egui::Id::new("app-private-input"))
                            .unwrap()
                    });
                    assert!((input.height() - studio::metrics::CONTROL_HEIGHT).abs() < 0.1);
                    let rect = ctx.data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new((
                            "test-action",
                            "memory_control",
                            "Save connection",
                        )))
                        .unwrap()
                    });
                    let (clip, _) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                            "test-action-visibility",
                            "Save connection",
                        )))
                        .unwrap()
                    });
                    assert!(
                        screen.contains_rect(rect) && clip.contains_rect(rect),
                        "{language}/{theme:?}/{provider}"
                    );
                    let commands = super::super::tests::click_at_size(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        "memory_control",
                        "Save connection",
                    );
                    assert_eq!(commands.len(), 1);
                    assert_eq!(commands[0].action, "project_connection_set");
                    assert_eq!(commands[0].payload["projectId"], "project:one");
                    assert_eq!(commands[0].payload["providerId"], provider);
                    assert_eq!(
                        commands[0].payload["accessToken"],
                        "SYNTHETIC-PRIVATE-CANARY"
                    );
                    assert!(console.create.studio.token.is_empty());
                    assert_eq!(
                        console.create.studio.arguments["content"],
                        "Unsent operation draft"
                    );
                }
            }
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
    fn inline_studio_tabs_fit_minimum_locales_themes_and_support_keyboard_navigation() {
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
                    assert!(super::super::tests::click_at_size(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        "memory_control",
                        "Output view"
                    )
                    .is_empty());
                    let next = (tab + 1) % 4;
                    let (rect, clip, id) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect, egui::Id)>(egui::Id::new((
                            "studio-view-tab",
                            next,
                        )))
                        .unwrap()
                    });
                    assert!(
                        screen.contains_rect(rect) && clip.contains_rect(rect),
                        "{language}/{theme:?}/{tab} {rect:?} {clip:?}"
                    );
                    ctx.memory_mut(|memory| memory.request_focus(id));
                    let (commands, copy) = render(
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
                    assert_eq!(console.create.studio.tab, next);
                    assert!(commands.is_empty());
                    assert!(
                        !copy
                            .iter()
                            .any(|line| line == "Output Studio" || line == "Close"),
                        "inline view must not render the old modal chrome"
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
                assert!(
                    !copy.iter().any(|line| line.starts_with("Editable ")),
                    "frozen approval replaces editable request fields"
                );
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
    fn switching_inline_views_clears_secrets_but_preserves_pending_scope_and_frozen_approval() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["connections"] = json!([{"providerId":"notion","connected":true}]);
        let mut console = console(&data);
        console.hold_automatic_reads = true;
        console.create.studio.tab = 3;
        console.create.studio.connection_project = "project:one".into();
        console.create.studio.connection_provider = "notion".into();
        console.create.studio.pending_prepare = Some(("prepare:one".into(), "project:one".into()));
        console.create.studio.token = "PRIVATE-CANARY".into();
        console.create.select_productive_view(1);
        assert!(console.create.studio.token.is_empty());
        data["projectStudioReceipt"] = json!({"projectId":"project:other","commandId":"prepare:one","preparedActionId":"action:foreign","revision":1,"outcome":"applied","reasonCode":"PROJECT_CONNECTION_PREPARED"});
        console.create.sync(&data);
        assert!(console.create.studio.prepared.is_none());
        assert!(console.create.studio.pending_prepare.is_some());
        data["projectStudioReceipt"] = json!({"projectId":"project:one","commandId":"prepare:one","preparedActionId":"action:one","revision":1,"outcome":"applied","reasonCode":"PROJECT_CONNECTION_PREPARED","operation":"notion_page","arguments":{"parentPageId":"frozen-parent","title":"Frozen title","content":"Frozen content"}});
        console.create.sync(&data);
        assert_eq!(console.create.productive_view(), 1);
        assert!(console.create.studio.pending_prepare.is_none());
        console.create.select_productive_view(0);
        console.create.sync(&data);
        console.create.select_productive_view(2);
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            vec![],
        );
        assert!(commands.is_empty());
        assert!(copy.iter().any(|line| line == "Frozen title"));
        assert!(copy.iter().any(|line| line == "Approve & execute"));
        assert!(!copy.join("\n").contains("PRIVATE-CANARY"));
        assert_eq!(console.create.draft().goal, "Owner goal");
        let commands = super::super::tests::click_at_size(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            "memory_control",
            "Approve & execute",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["preparedActionId"], "action:one");
        console.create.project = "project:other".into();
        console.create.sync(&data);
        assert!(console.create.studio.prepared.is_none());
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
    #[test]
    fn multi_variant_files_are_frozen_and_only_exact_complete_import_stages_chat_without_execution()
    {
        use super::super::tests::click_at_size;
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["artifacts"] = json!([
            {"workItemId":"work:one","path":"outputs/a.txt","name":"a.txt","sha256":"a".repeat(64),"bytes":100},
            {"workItemId":"work:two","path":"outputs/b.txt","name":"b.txt","sha256":"b".repeat(64),"bytes":200}
        ]);
        let mut c = console(&data);
        c.hold_automatic_reads = true;
        c.create.sync(&data);
        c.create.studio.package_mode = true;
        c.create.studio.packages.insert(
            "project:one".into(),
            [
                ("work:one".into(), "outputs/a.txt".into(), "a".repeat(64)),
                ("work:two".into(), "outputs/b.txt".into(), "b".repeat(64)),
            ]
            .into_iter()
            .collect(),
        );
        let goal = c.create.conversation_input().clone();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        assert!(click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "memory_control",
            "Package actions"
        )
        .is_empty());
        let commands = click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "project_resources_from_artifacts",
            "Use selected files in Chat",
        );
        assert_eq!(commands.len(), 1);
        let command = &commands[0];
        assert_eq!(command.payload["files"][0]["workItemId"], "work:one");
        assert_eq!(command.payload["files"][1]["workItemId"], "work:two");
        assert_eq!(
            command.payload["files"][1]["expectedSha256"],
            "b".repeat(64)
        );
        data["projectStudioReceipt"] = json!({"commandId":command.payload["commandId"],"projectId":"project:one","reasonCode":"PROJECT_RESOURCES_IMPORTED","outcome":"applied","resources":[{"resourceId":"resource:00000000-0000-4000-8000-000000000001","revision":1,"status":"available","sha256":"c".repeat(64)}]});
        let mut next = Vec::new();
        c.sync_artifact_resources(&data, &mut next);
        assert_eq!(c.create.conversation_input(), &goal);
        assert!(c.create.studio.resource_bundle.is_some());
        assert_eq!(c.create.studio.packages["project:one"].len(), 2);
        data["projectStudioReceipt"]["resources"].as_array_mut().unwrap().push(json!({"resourceId":"resource:00000000-0000-4000-8000-000000000002","revision":1,"status":"available","sha256":"d".repeat(64)}));
        c.sync_artifact_resources(&data, &mut next);
        assert!(next.is_empty());
        assert_eq!(c.create.productive_view(), 0);
        assert!(c.create.conversation_input().starts_with(&goal));
        assert!(c
            .create
            .conversation_input()
            .contains("resource:00000000-0000-4000-8000-000000000002"));
        assert!(c.create.conversation_input().contains(&"d".repeat(64)));
        assert!(c.create.studio.packages["project:one"].is_empty());
    }
}
