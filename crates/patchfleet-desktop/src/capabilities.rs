use super::*;

type Scope = (String, String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityPathKind {
    Resources,
    TriggerFolder,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityPathRequest {
    pub request_id: String,
    pub project_id: String,
    pub host_epoch: Option<Value>,
    pub kind: CapabilityPathKind,
    scope: Scope,
    source_before: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct VisualSelection {
    pub project: String,
    pub work: String,
    pub path: String,
    pub hash: String,
    pub section: String,
}
struct RevisionRequest {
    command: String,
    selection: VisualSelection,
    input: String,
    scope: Scope,
}

#[derive(Default)]
pub(super) struct CapabilityState {
    pub open: bool,
    pub(super) tab: usize,
    drafts: HashMap<(String, usize), CapabilityDraft>,
    pub attachments: HashMap<Scope, Vec<String>>,
    pub selection: HashMap<Scope, VisualSelection>,
    path_request: Option<CapabilityPathRequest>,
    waiting_path: Option<CapabilityPathRequest>,
    requested: Option<(String, usize, usize)>,
    dependencies: HashSet<(String, &'static str)>,
    offset: usize,
    pending: Option<(String, String)>,
    observed: HashMap<&'static str, (String, String)>,
    revision: Option<RevisionRequest>,
    private_project: String,
    engine_url: String,
    engine_checkpoint: String,
}
#[derive(Default)]
struct CapabilityDraft {
    values: HashMap<String, String>,
    sources: HashSet<String>,
    record: String,
    recipe: String,
    kind: usize,
    loaded: bool,
}
impl CapabilityDraft {
    fn value(&mut self, key: &str) -> &mut String {
        self.values.entry(key.into()).or_default()
    }
    fn get(&self, key: &str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or("")
    }
}

pub(super) fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub(super) fn complete_import(receipt: &Value, count: usize) -> bool {
    let resources = array(&receipt["resources"]);
    let ids: HashSet<_> = resources.iter().map(|r| text(r, "resourceId")).collect();
    text(receipt, "reasonCode") == "PROJECT_RESOURCES_IMPORTED"
        && text(receipt, "outcome") == "applied"
        && count > 0
        && resources.len() == count
        && ids.len() == count
        && resources.iter().all(|r| {
            text(r, "resourceId")
                .strip_prefix("resource:")
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
                && valid_hash(text(r, "sha256"))
                && revision(r) > 0
                && text(r, "status") == "available"
        })
}
fn section(tab: usize) -> Option<&'static str> {
    match tab {
        1 => Some("skills"),
        2 => Some("triggers"),
        3 => Some("variants"),
        4 => Some("evidence"),
        _ => None,
    }
}
fn page<'a>(snapshot: &'a Value, project: &str) -> Option<&'a Value> {
    let page = &snapshot["projectCapabilities"];
    (text(page, "projectId") == project && text(page, "reasonCode") == "PROJECT_CAPABILITIES_READ")
        .then_some(page)
}
fn records<'a>(page: &'a Value, kind: &str) -> &'a [Value] {
    if text(page, "section") == kind {
        array(&page["items"])
    } else if page["section"].is_null() {
        array(&page[kind])
    } else {
        &[]
    }
}
fn resources<'a>(snapshot: &'a Value, project: &str) -> &'a [Value] {
    let p = &snapshot["projectStudio"];
    if text(p, "projectId") == project && text(p, "reasonCode") == "PROJECT_STUDIO_READ" {
        array(&p["resources"])
    } else {
        &[]
    }
}
fn source_refs(draft: &CapabilityDraft, resources: &[Value]) -> Vec<Value> {
    resources
        .iter()
        .filter(|r| {
            draft.sources.contains(text(r, "resourceId"))
                && text(r, "status") == "available"
                && valid_hash(text(r, "sha256"))
        })
        .take(8)
        .map(|r| json!({"resourceId":r["resourceId"],"expectedSha256":r["sha256"]}))
        .collect()
}
fn optional_text(value: &str, max: usize) -> bool {
    value.encode_utf16().count() <= max && !value.contains('\0')
}
fn quote_span(source: &str, quote: &str) -> Option<(usize, usize)> {
    (!quote.is_empty())
        .then(|| source.find(quote))
        .flatten()
        .map(|start| (start, start + quote.len()))
}
fn lines(value: &str, max: usize, length: usize) -> Option<Vec<String>> {
    let values: Vec<_> = value
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    (!values.is_empty() && values.len() <= max && values.iter().all(|v| valid_input(v, length)))
        .then_some(values)
}
fn budgets(ui: &mut Ui, draft: &mut CapabilityDraft) -> Value {
    let mut jobs = draft.get("jobs").parse::<u64>().unwrap_or(6);
    let mut minutes = draft.get("minutes").parse::<u64>().unwrap_or(30);
    let mut rounds = draft.get("rounds").parse::<u64>().unwrap_or(1);
    ui.horizontal_wrapped(|ui| {
        ui.label(t(ui, "Jobs"));
        ui.add(egui::DragValue::new(&mut jobs).range(1..=36));
        ui.label(t(ui, "Minutes"));
        ui.add(egui::DragValue::new(&mut minutes).range(1..=240));
        ui.label(t(ui, "Review rounds"));
        ui.add(egui::DragValue::new(&mut rounds).range(0..=2));
    });
    *draft.value("jobs") = jobs.to_string();
    *draft.value("minutes") = minutes.to_string();
    *draft.value("rounds") = rounds.to_string();
    json!({"maxJobs":jobs,"maxElapsedMinutes":minutes,"maxReviewRounds":rounds})
}
fn sources(ui: &mut Ui, draft: &mut CapabilityDraft, resources: &[Value]) {
    if resources.is_empty() {
        hint(ui, "Import a source in Outputs / Resources first.");
    }
    for resource in resources
        .iter()
        .filter(|r| text(r, "status") == "available")
    {
        let id = text(resource, "resourceId");
        let mut checked = draft.sources.contains(id);
        if ui
            .add_enabled(
                checked || draft.sources.len() < 8,
                egui::Checkbox::new(&mut checked, text(resource, "name")),
            )
            .changed()
        {
            if checked {
                draft.sources.insert(id.into());
            } else {
                draft.sources.remove(id);
            }
        }
    }
}
fn recipes<'a>(snapshot: &'a Value, project: &str) -> &'a [Value] {
    for key in ["projectWorkflowRead", "projectWorkflows"] {
        let page = &snapshot[key];
        if text(page, "projectId") == project {
            if text(page, "section") == "recipes" {
                return array(&page["items"]);
            }
            if page["section"].is_null() {
                return array(&page["recipes"]);
            }
        }
    }
    &[]
}
fn choose_recipe(ui: &mut Ui, draft: &mut CapabilityDraft, recipes: &[Value]) -> Option<Value> {
    form_combo(
        ui,
        egui::ComboBox::from_id_salt("capability-recipe")
            .width(ui.available_width())
            .selected_text(
                recipes
                    .iter()
                    .find(|r| text(r, "recipeId") == draft.recipe)
                    .map(|r| text(r, "name"))
                    .unwrap_or(t(ui, "Choose a recipe").as_ref()),
            ),
        |ui| {
            for recipe in recipes {
                ui.selectable_value(
                    &mut draft.recipe,
                    text(recipe, "recipeId").into(),
                    text(recipe, "name"),
                );
            }
        },
    );
    recipes
        .iter()
        .find(|r| text(r, "recipeId") == draft.recipe && revision(r) > 0)
        .cloned()
}
fn send(
    ui: &mut Ui,
    commands: &mut Vec<Command>,
    state: &mut CapabilityState,
    project: &str,
    label: &str,
    name: &'static str,
    payload: Option<Value>,
) {
    let enabled = payload.is_some() && state.pending.is_none() && commands.is_empty();
    let clicked = ui
        .add_enabled_ui(enabled, |ui| submit(ui, label, name))
        .inner;
    if clicked {
        let command = Command::new(name, payload.unwrap());
        state.pending = Some((text(&command.payload, "commandId").into(), project.into()));
        commands.push(command);
    }
}

fn attachment_chip(ui: &mut Ui, path: &str) -> egui::Response {
    let name = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let id = ui.id().with(("attachment-chip", path));
    let response = ui
        .scope_builder(egui::UiBuilder::new().id(id), |ui| {
            studio::input_chip(ui, &name, &tf(ui, "Remove attachment · {0}", &[&name]))
        })
        .inner;
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("composer-remove-attachment", path)),
            (response.rect, response.id, ui.clip_rect()),
        );
    });
    response
}

fn selection_label(ui: &Ui, selection: &VisualSelection) -> String {
    let filename = std::path::Path::new(&selection.path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let section = if let Some(page) = selection.section.strip_prefix("page:") {
        tf(ui, "Page {0}", &[page])
    } else if selection.section.starts_with("region:") {
        t(ui, "Image region").into_owned()
    } else if let Some(range) = selection.section.strip_prefix("time:") {
        format!("{} · {} s", t(ui, "Video range"), range.replace(':', "–"))
    } else {
        selection.section.clone()
    };
    let target = if section.is_empty() {
        filename.into_owned()
    } else {
        format!("{filename} · {section}")
    };
    tf(ui, "Revise · {0}", &[&target])
}

impl CapabilityState {
    pub(super) fn clear_private(&mut self) {
        self.engine_url.clear();
        self.engine_checkpoint.clear();
    }
}

impl CreateState {
    pub(super) fn visual_pending(&self) -> bool {
        self.capabilities.revision.is_some()
    }
    pub(super) fn visual_selection(&self) -> Option<&VisualSelection> {
        self.capabilities
            .selection
            .get(&(self.project.clone(), self.ability.clone()))
    }
    pub(super) fn visual_revision_enabled(&mut self, snapshot: &Value) -> bool {
        self.visual_selection().is_some()
            && self.capabilities.revision.is_none()
            && self.attachment_paths().is_empty()
            && valid_input(self.conversation_input(), 12_000)
            && self.resolved_planner_provider(snapshot).is_some()
    }
    pub(super) fn attachment_paths(&self) -> Vec<String> {
        self.capabilities
            .attachments
            .get(&(self.project.clone(), self.ability.clone()))
            .cloned()
            .unwrap_or_default()
    }
    pub(super) fn queue_paths(&mut self, paths: Vec<String>) {
        let queue = self
            .capabilities
            .attachments
            .entry((self.project.clone(), self.ability.clone()))
            .or_default();
        let previous = queue.len();
        let mut overflow = false;
        for path in paths {
            if queue.len() >= 12 && !queue.contains(&path) {
                overflow = true;
            }
            if queue.len() < 12
                && !path.contains('\0')
                && std::path::Path::new(&path).is_absolute()
                && !queue.contains(&path)
            {
                queue.push(path);
            }
        }
        if queue.len() > previous {
            self.crew.single = false;
        }
        if overflow {
            self.chat
                .conversation
                .set_error("Attach up to 12 files per request. Extra files were not added.");
        }
    }
    pub(super) fn request_capability_paths(&mut self, kind: CapabilityPathKind) {
        let source = self
            .capabilities
            .drafts
            .get(&(self.project.clone(), 2))
            .map(|d| d.get("source").to_owned())
            .unwrap_or_default();
        self.request_capability_paths_from(kind, source);
    }
    fn request_capability_paths_from(&mut self, kind: CapabilityPathKind, source_before: String) {
        let command = Command::new("path_request", json!({}));
        let request = CapabilityPathRequest {
            request_id: text(&command.payload, "commandId").into(),
            project_id: self.project.clone(),
            host_epoch: self.host_epoch.clone(),
            source_before,
            kind,
            scope: (self.project.clone(), self.ability.clone()),
        };
        self.capabilities.waiting_path = Some(request.clone());
        self.capabilities.path_request = Some(request);
    }
    pub(super) fn sync_capabilities(&mut self, snapshot: &Value) {
        if self.capabilities.private_project != self.project {
            self.capabilities.clear_private();
            self.capabilities.private_project = self.project.clone();
        }
        for cache in [
            "projectCapabilityReceipt",
            "projectCapabilityError",
            "projectStudioReceipt",
            "projectStudioError",
        ] {
            let receipt = &snapshot[cache];
            if matches!(
                text(receipt, "action"),
                "project_capabilities_read"
                    | "project_evidence_read"
                    | "project_studio_read"
                    | "project_resource_read"
                    | "project_artifact_read"
                    | "project_artifact_visual_read"
                    | "project_artifact_frame_read"
            ) {
                continue;
            }
            if cache.starts_with("projectStudio")
                && !matches!(
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
                && !matches!(
                    text(receipt, "reasonCode"),
                    "PROJECT_RESOURCES_IMPORTED" | "PROJECT_IMAGE_ENGINE_CONFIGURED"
                )
            {
                continue;
            }
            let command = text(receipt, "commandId");
            let project = text(receipt, "projectId");
            if command.is_empty()
                || project != self.project
                || matches!(text(receipt, "outcome"), "" | "unknown")
            {
                continue;
            }
            let identity = (project.to_owned(), command.to_owned());
            if self.capabilities.observed.get(cache) == Some(&identity) {
                continue;
            }
            self.capabilities.observed.insert(cache, identity);
            if cache.starts_with("projectCapability") {
                self.capabilities.requested = None;
            } else {
                self.capabilities
                    .dependencies
                    .remove(&(project.into(), "project_studio_read"));
                self.studio.requested = None;
            }
        }
        if let Some((id, project)) = &self.capabilities.pending {
            let receipt = &snapshot["projectCapabilityReceipt"];
            let error = &snapshot["projectCapabilityError"];
            if [receipt, error].iter().any(|r| {
                text(r, "commandId") == id
                    && text(r, "projectId") == project
                    && !matches!(text(r, "outcome"), "" | "unknown")
            }) {
                self.capabilities.pending = None;
                self.capabilities.requested = None;
            }
        }
        if let Some(request) = &self.capabilities.revision {
            let receipt = &snapshot["projectStudioReceipt"];
            if text(receipt, "commandId") == request.command
                && text(receipt, "projectId") == request.selection.project
                && !matches!(text(receipt, "outcome"), "" | "unknown")
            {
                if text(receipt, "outcome") == "applied"
                    && self.capabilities.selection.get(&request.scope) == Some(&request.selection)
                {
                    let input = if self.chat.conversation.has_session(&request.scope) {
                        self.chat.conversation.input_for(&request.scope)
                    } else {
                        self.drafts.get(&request.scope).map(|d| d.goal.as_str())
                    };
                    if input == Some(request.input.as_str()) {
                        self.capabilities.selection.remove(&request.scope);
                        if self.chat.conversation.has_session(&request.scope) {
                            self.chat.conversation.clear_input_for(&request.scope);
                        } else if let Some(draft) = self.drafts.get_mut(&request.scope) {
                            draft.goal.clear();
                        }
                    }
                }
                self.capabilities.revision = None;
            }
        }
    }
}
impl Console {
    #[cfg(feature = "ui-smoke")]
    pub fn capabilities_smoke(&mut self, snapshot: &Value, scene: &str) {
        self.chat_smoke(snapshot, false, false);
        self.hold_automatic_reads = true;
        let project = self.create.project.clone();
        self.create.capabilities.dependencies.extend([
            (project.clone(), "project_studio_read"),
            (project.clone(), "project_workflows_read"),
        ]);
        self.create.studio.requested =
            output_studio::project_revision(snapshot, &project).map(|rev| (project, rev));
        self.create.select_productive_view(3);
        if scene == "capabilities-canvas" {
            self.studio_canvas_smoke(snapshot);
            return;
        }
        if scene == "capabilities-attachments" {
            self.create.select_productive_view(0);
            self.create
                .queue_paths((1..=12).map(|i| format!("/qa/source-{i:02}.png")).collect());
            let key = (self.create.project.clone(), self.create.ability.clone());
            if let Some(job) = jobs(snapshot)
                .iter()
                .find(|j| text(j, "projectId") == self.create.project)
            {
                self.create.chat.replies.insert(
                    key.clone(),
                    ReplyDraft {
                        parent: text(job, "workItemId").into(),
                        references: HashSet::new(),
                    },
                );
            }
            if let Some(artifact) = array(&snapshot["projectStudio"]["artifacts"]).first() {
                self.create.capabilities.selection.insert(
                    key,
                    VisualSelection {
                        project: self.create.project.clone(),
                        work: text(artifact, "workItemId").into(),
                        path: text(artifact, "path").into(),
                        hash: text(artifact, "sha256").into(),
                        section: String::new(),
                    },
                );
            }
            return;
        }
        self.create.capabilities.tab = match scene {
            "capabilities-skills" => 1,
            "capabilities-triggers" => 2,
            "capabilities-alternatives" => 3,
            "capabilities-evidence" => 4,
            "capabilities-production"
            | "capabilities-images"
            | "capabilities-video"
            | "capabilities-batch-images"
            | "capabilities-interactive" => 5,
            "capabilities-browser" => 6,
            _ => 0,
        };
        self.create.capabilities.requested =
            Some((self.create.project.clone(), self.create.capabilities.tab, 0));
        if self.create.capabilities.tab == 5 {
            let project = self.create.project.clone();
            let selected = resources(snapshot, &project)
                .iter()
                .map(|resource| text(resource, "resourceId").to_owned())
                .collect::<Vec<_>>();
            let draft = self
                .create
                .capabilities
                .drafts
                .entry((project, 5))
                .or_default();
            draft.kind = match scene {
                "capabilities-video" => 2,
                "capabilities-interactive" => 3,
                "capabilities-batch-images" => 7,
                _ => 0,
            };
            if scene == "capabilities-batch-images" {
                draft.sources.extend(selected);
                *draft.value("width") = "1600".into();
                *draft.value("height") = "1600".into();
            }
        }
    }
    pub fn take_capability_path_request(&mut self) -> Option<CapabilityPathRequest> {
        self.create.capabilities.path_request.take()
    }
    pub fn complete_capability_paths(
        &mut self,
        request: CapabilityPathRequest,
        paths: Option<Vec<String>>,
    ) {
        if self.create.capabilities.waiting_path.as_ref() != Some(&request) {
            return;
        }
        self.create.capabilities.waiting_path = None;
        if request.host_epoch != self.create.host_epoch
            || request.scope != (self.create.project.clone(), self.create.ability.clone())
        {
            return;
        }
        let Some(paths) = paths else {
            return;
        };
        match request.kind {
            CapabilityPathKind::Resources => self.create.queue_paths(paths),
            CapabilityPathKind::TriggerFolder => {
                let draft = self
                    .create
                    .capabilities
                    .drafts
                    .entry((request.project_id.clone(), 2))
                    .or_default();
                if draft.kind != 0 || draft.get("source") != request.source_before {
                    return;
                }
                if let Some(path) = paths
                    .into_iter()
                    .find(|p| std::path::Path::new(p).is_absolute() && !p.contains('\0'))
                {
                    *self
                        .create
                        .capabilities
                        .drafts
                        .entry((request.project_id, 2))
                        .or_default()
                        .value("source") = path;
                }
            }
        }
    }
    pub(super) fn receive_dropped_resources(&mut self, ui: &mut Ui) {
        let dropped = ui.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() {
            self.create.queue_paths(dropped);
            ui.ctx().request_repaint();
        }
    }
    pub(super) fn attachment_controls(&mut self, ui: &mut Ui, _snapshot: &Value) {
        let key = (self.create.project.clone(), self.create.ability.clone());
        let paths = self.create.attachment_paths();
        let selection = self.create.capabilities.selection.get(&key).cloned();
        let conflict = selection.is_some() && !paths.is_empty();
        let visible = if selection.is_some() || ui.available_width() < 460.0 {
            1
        } else {
            2
        };
        let slots = paths.len().min(visible) + usize::from(selection.is_some());
        let overflow = paths.len().saturating_sub(visible);
        let overflow_width = if overflow > 0 { 96.0 } else { 0.0 };
        let chip_width =
            ((ui.available_width() - overflow_width - studio::metrics::GAP * slots as f32)
                / slots.max(1) as f32)
                .clamp(100.0, 280.0);
        let mut removed = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = studio::metrics::GAP;
            if let Some(selection) = &selection {
                let label = selection_label(ui, selection);
                let selection_id = egui::Id::new(("composer-selection", &selection.project, &self.create.ability, &selection.work, &selection.path, &selection.hash, &selection.section));
                let clear = ui.scope_builder(egui::UiBuilder::new().id(selection_id), |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(chip_width, studio::metrics::CHIP_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| studio::input_chip(ui, &label, t(ui, "Clear selection").as_ref()),
                    ).inner
                }).inner.on_hover_text(&selection.section);
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new("composer-clear-selection"),
                        (clear.rect, ui.clip_rect()),
                    );
                });
                if clear.clicked() {
                    self.create.capabilities.selection.remove(&key);
                }
            }
            for path in paths.iter().take(visible) {
                if ui.push_id(("composer-file", path), |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(chip_width, studio::metrics::CHIP_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| attachment_chip(ui, path),
                    ).inner
                }).inner.clicked() {
                    removed = Some(path.clone());
                }
            }
            if overflow > 0 {
                let more = studio::suggestion_chip(ui, &tf(ui, "+{0} files", &[&overflow.to_string()]));
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new("composer-attachment-overflow"), more.rect);
                });
                egui::Popup::menu(&more).width(320.0).show(|ui| {
                    hint(ui, &tf(ui, "{0} files ready to attach", &[&paths.len().to_string()]));
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for path in &paths {
                                if attachment_chip(ui, path).clicked() {
                                    removed = Some(path.clone());
                                }
                            }
                        });
                    if studio::quiet_button(ui, t(ui, "Remove attachments").as_ref()).clicked() {
                        self.create.capabilities.attachments.remove(&key);
                        ui.close();
                    }
                    hint(ui, "Files are imported as Project resources before planning. Nothing runs on drop.");
                });
            }
        });
        if let Some(path) = removed {
            if let Some(paths) = self.create.capabilities.attachments.get_mut(&key) {
                paths.retain(|value| value != &path);
            }
        }
        if conflict {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(t(ui, "Choose attachments or a revision."));
                let clear = studio::quiet_button(ui, t(ui, "Remove attachments").as_ref());
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new("composer-remove-attachments"),
                        (clear.rect, ui.clip_rect()),
                    );
                });
                if clear.clicked() {
                    self.create.capabilities.attachments.remove(&key);
                }
            });
        }
    }
    pub(super) fn prepare_visual_revision(
        &mut self,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if self.command_busy || !self.create.visual_revision_enabled(snapshot) {
            return;
        }
        let scope = (self.create.project.clone(), self.create.ability.clone());
        let Some(selection) = self.create.capabilities.selection.get(&scope).cloned() else {
            return;
        };
        let instruction = self.create.conversation_input().clone();
        let Some(provider) = self.create.resolved_planner_provider(snapshot) else {
            return;
        };
        if !commands.is_empty()
            || self.create.capabilities.revision.is_some()
            || !valid_input(&instruction, 12_000)
        {
            return;
        }
        let Some(rev) = output_studio::project_revision(snapshot, &selection.project) else {
            return;
        };
        let command = Command::new(
            "project_artifact_revise",
            json!({"projectId":selection.project,"expectedProjectRevision":rev,"workItemId":selection.work,"path":selection.path,"expectedSha256":selection.hash,"sectionId":selection.section,"instruction":instruction,"providerId":provider}),
        );
        self.create.capabilities.revision = Some(RevisionRequest {
            command: text(&command.payload, "commandId").into(),
            selection,
            input: instruction,
            scope,
        });
        commands.push(command);
    }
    pub(super) fn project_capabilities(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        let project = self.create.project.clone();
        if output_studio::project_revision(snapshot, &project).is_none() {
            hint(ui, "Choose a project to manage its tools.");
            return;
        }
        let state = &mut self.create.capabilities;
        let previous_tab = state.tab;
        let labels = [
            "Brand",
            "Project skills",
            "Triggers",
            "Alternatives",
            "Evidence",
            "Create outputs",
            "Browser",
        ];
        inline_surface(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(studio::title(t(ui, labels[state.tab]), 20.0));
                form_combo(
                    ui,
                    egui::ComboBox::from_id_salt("project-tools-view")
                        .selected_text(t(ui, "Project tools")),
                    |ui| {
                        for (i, label) in labels.iter().enumerate() {
                            ui.selectable_value(&mut state.tab, i, t(ui, label));
                        }
                    },
                );
                if studio::quiet_button(ui, t(ui, "Refresh").as_ref()).clicked() {
                    state.requested = None;
                    state.dependencies.clear();
                }
            });
        });
        if previous_tab != state.tab {
            state.clear_private();
            state.offset = 0;
            state.requested = None;
        }
        let tab = state.tab;
        if ui.is_enabled()
            && studio::commands_ready(ui)
            && !self.hold_automatic_reads
            && commands.is_empty()
        {
            for (action, needed) in [
                ("project_studio_read", true),
                (
                    "project_workflows_read",
                    (tab == 2 || tab == 3) && recipes(snapshot, &project).is_empty(),
                ),
            ] {
                if needed && state.dependencies.insert((project.clone(), action)) {
                    let mut payload = json!({"projectId":project});
                    if action == "project_workflows_read" {
                        payload["section"] = json!("recipes");
                        payload["limit"] = json!(50);
                    }
                    commands.push(Command::new(action, payload));
                    break;
                }
            }
        }
        let key = (project.clone(), tab, state.offset);
        if ui.is_enabled()
            && studio::commands_ready(ui)
            && !self.hold_automatic_reads
            && commands.is_empty()
            && state.requested.as_ref() != Some(&key)
        {
            state.requested = Some(key);
            let mut payload = json!({"projectId":project,"offset":state.offset,"limit":20});
            if let Some(section) = section(tab) {
                payload["section"] = json!(section);
            }
            commands.push(Command::new("project_capabilities_read", payload));
        }
        let page = page(snapshot, &project).cloned();
        let mut draft = state
            .drafts
            .remove(&(project.clone(), tab))
            .unwrap_or_default();
        ui.add_space(studio::metrics::GAP);
        if tab == 0 {
            self.brand_form(ui, snapshot, page.as_ref(), &mut draft, commands);
            self.create
                .capabilities
                .drafts
                .insert((project, tab), draft);
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt(("project-tools", &project, tab))
            .max_height(ui.available_height())
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match tab {
                    0 => self.brand_form(ui, snapshot, page.as_ref(), &mut draft, commands),
                    1 => self.skills_form(ui, snapshot, page.as_ref(), &mut draft, commands),
                    2 => self.triggers_form(ui, snapshot, page.as_ref(), &mut draft, commands),
                    3 => self.variants_form(ui, snapshot, page.as_ref(), &mut draft, commands),
                    4 => self.evidence_form(ui, snapshot, page.as_ref(), &mut draft, commands),
                    5 => self.production_form(ui, snapshot, &mut draft, commands),
                    _ => self.browser_form(ui, snapshot, &mut draft, commands),
                }
                if let Some(page) = &page {
                    if section(tab).is_some() {
                        ui.horizontal_wrapped(|ui| {
                            if studio::quiet_button(ui, t(ui, "Previous").as_ref()).clicked() {
                                self.create.capabilities.offset =
                                    self.create.capabilities.offset.saturating_sub(20);
                                self.create.capabilities.requested = None;
                            }
                            if let Some(next) = page["nextOffset"].as_u64() {
                                if studio::quiet_button(ui, t(ui, "Next").as_ref()).clicked() {
                                    self.create.capabilities.offset = next as usize;
                                    self.create.capabilities.requested = None;
                                }
                            }
                        });
                    }
                }
                for key in ["projectCapabilityReceipt", "projectCapabilityError"] {
                    let receipt = &snapshot[key];
                    if text(receipt, "projectId") == project
                        && !text(receipt, "reasonCode").is_empty()
                    {
                        hint(ui, text(receipt, "reasonCode"));
                    }
                }
            });
        self.create
            .capabilities
            .drafts
            .insert((project, tab), draft);
    }
    fn brand_form(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: Option<&Value>,
        draft: &mut CapabilityDraft,
        commands: &mut Vec<Command>,
    ) {
        let Some(page) = page else {
            hint(ui, "Load this Project's tools to continue.");
            return;
        };
        let profile = &page["profile"];
        if !draft.loaded {
            for (key, value) in [
                ("tone", profile["tone"].as_str().unwrap_or("")),
                (
                    "instructions",
                    profile["instructions"].as_str().unwrap_or(""),
                ),
            ] {
                *draft.value(key) = value.into();
            }
            *draft.value("colours") = array(&profile["colours"])
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n");
            draft.sources = array(&profile["logoResources"])
                .iter()
                .filter_map(|r| r["resourceId"].as_str().map(str::to_owned))
                .collect();
            draft.loaded = true;
        }
        hint(ui, "A shared style for future work in this Project.");
        ui.add_space(studio::metrics::GAP);
        let height = (ui.available_height() - 100.0).max(40.0);
        let body = egui::ScrollArea::vertical()
            .id_salt(("brand-fields", &self.create.project))
            .max_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                field(ui, "Tone", draft.value("tone"), 500, false);
                multiline(
                    ui,
                    "Style instructions",
                    draft.value("instructions"),
                    4000,
                    3,
                );
                multiline(
                    ui,
                    "Colours · one #RRGGBB per line",
                    draft.value("colours"),
                    64,
                    2,
                );
                ui.collapsing(t(ui, "Logo resources"), |ui| {
                    sources(ui, draft, resources(snapshot, &self.create.project))
                });
            });
        #[cfg(test)]
        ui.data_mut(|data| data.insert_temp(egui::Id::new("brand-fields-bounds"), body.inner_rect));
        #[cfg(not(test))]
        let _ = body;
        let colours: Vec<_> = draft
            .get("colours")
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let valid_colours = colours.len() <= 8
            && colours.iter().all(|c| {
                c.len() == 7
                    && c.starts_with('#')
                    && c.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            });
        let valid = valid_colours
            && optional_text(draft.get("tone"), 500)
            && optional_text(draft.get("instructions"), 4000);
        let mut logos = source_refs(draft, resources(snapshot, &self.create.project));
        // Preserve selected saved logos outside the currently loaded resource page.
        for logo in array(&profile["logoResources"]) {
            if draft.sources.contains(text(logo, "resourceId"))
                && valid_hash(text(logo, "expectedSha256"))
                && !logos.iter().any(|r| r["resourceId"] == logo["resourceId"])
            {
                logos.push(logo.clone());
            }
        }
        let current_logos: HashSet<_> = array(&profile["logoResources"])
            .iter()
            .map(|r| (text(r, "resourceId"), text(r, "expectedSha256")))
            .collect();
        let selected_logos: HashSet<_> = logos
            .iter()
            .map(|r| (text(r, "resourceId"), text(r, "expectedSha256")))
            .collect();
        let dirty = draft.get("tone") != text(profile, "tone")
            || draft.get("instructions") != text(profile, "instructions")
            || colours
                != array(&profile["colours"])
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
            || current_logos != selected_logos;
        let payload = (valid && dirty).then(|| {
            json!({
                "projectId":self.create.project,"expectedProfileRevision":revision(profile),
                "tone":draft.get("tone"),"instructions":draft.get("instructions"),
                "colours":colours,"logoResources":logos
            })
        });
        ui.add_space(studio::metrics::GAP);
        ui.separator();
        if !valid_colours {
            ui.colored_label(
                colors(ui).danger,
                t(ui, "Use up to 8 colours in #RRGGBB format."),
            );
        } else if !valid {
            ui.colored_label(
                colors(ui).danger,
                t(ui, "Complete the required fields to continue."),
            );
        } else {
            hint(
                ui,
                if self.create.capabilities.pending.is_some() || !studio::commands_ready(ui) {
                    "Applying action…"
                } else if dirty {
                    "Unsaved changes"
                } else {
                    "No changes to save"
                },
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            send(
                ui,
                commands,
                &mut self.create.capabilities,
                &self.create.project,
                "Save brand",
                "project_brand_save",
                payload,
            );
        });
    }

    fn skills_form(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: Option<&Value>,
        draft: &mut CapabilityDraft,
        commands: &mut Vec<Command>,
    ) {
        hint(
            ui,
            "Review the sources and steps before accepting standing instructions.",
        );
        if let Some(page) = page {
            for skill in records(page, "skills") {
                ui.push_id(text(skill, "skillId"), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(text(skill, "name"));
                        status(ui, text(skill, "status"));
                        if studio::quiet_button(ui, t(ui, "Review skill").as_ref()).clicked() {
                            draft.record = text(skill, "skillId").into();
                            *draft.value("name") = text(skill, "name").into();
                            *draft.value("steps") = array(&skill["steps"])
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join("\n");
                            *draft.value("outputs") = array(&skill["expectedOutputs"])
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join("\n");
                        }
                    });
                });
            }
        }
        field(ui, "Skill name", draft.value("name"), 80, false);
        multiline(ui, "Steps · one per line", draft.value("steps"), 6000, 4);
        multiline(
            ui,
            "Expected outputs · one per line",
            draft.value("outputs"),
            3000,
            2,
        );
        let selected = page.and_then(|p| {
            records(p, "skills")
                .iter()
                .find(|s| text(s, "skillId") == draft.record)
        });
        let steps = lines(draft.get("steps"), 20, 1000);
        let outputs = lines(draft.get("outputs"), 12, 240);
        if let Some(skill) = selected {
            for source in array(&skill["sources"]) {
                raw_hint(
                    ui,
                    &format!(
                        "{} · {}",
                        text(source, "resourceId"),
                        text(source, "expectedSha256")
                    ),
                );
            }
            let payload=(valid_input(draft.get("name"),80)&&steps.is_some()&&outputs.is_some()).then(||json!({"projectId":self.create.project,"skillId":skill["skillId"],"expectedSkillRevision":skill["revision"],"name":draft.get("name"),"steps":steps,"expectedOutputs":outputs}));
            if payload.is_none() {
                hint(ui, "Complete the required fields to continue.");
            }
            send(
                ui,
                commands,
                &mut self.create.capabilities,
                &self.create.project,
                "Accept skill",
                "project_skill_accept",
                payload,
            );
            quiet_action(
                ui,
                commands,
                "Revoke skill",
                "project_skill_revoke",
                text(skill, "status") == "accepted",
                || json!({"projectId":self.create.project,"skillId":skill["skillId"],"expectedSkillRevision":skill["revision"]}),
            );
            if studio::quiet_button(ui, t(ui, "New skill").as_ref()).clicked() {
                *draft = Default::default();
            }
        } else {
            sources(ui, draft, resources(snapshot, &self.create.project));
            let refs = source_refs(draft, resources(snapshot, &self.create.project));
            let payload=(valid_input(draft.get("name"),80)&&steps.is_some()&&outputs.is_some()&&!refs.is_empty()).then(||json!({"projectId":self.create.project,"name":draft.get("name"),"steps":steps,"expectedOutputs":outputs,"sources":refs}));
            if payload.is_none() {
                hint(ui, "Complete the required fields to continue.");
            }
            send(
                ui,
                commands,
                &mut self.create.capabilities,
                &self.create.project,
                "Propose skill",
                "project_skill_propose",
                payload,
            );
            if view_control(
                ui,
                "Ask Mox to extract steps",
                !draft.sources.is_empty(),
                false,
            )
            .clicked()
            {
                let request =
                    json!({"sources":source_refs(draft,resources(snapshot,&self.create.project))});
                self.stage_operation(snapshot,"Extract an inspectable Project skill proposal from these exact sources. Use project_skill_propose; do not accept it without the owner.",request,commands);
            }
        }
    }
    fn triggers_form(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: Option<&Value>,
        draft: &mut CapabilityDraft,
        commands: &mut Vec<Command>,
    ) {
        hint(
            ui,
            "Triggers run only while Moxcrew is open. Saving does not arm a trigger.",
        );
        hint(
            ui,
            "Arming records a baseline; only later changes create work.",
        );
        if let Some(page) = page {
            for trigger in records(page, "triggers") {
                ui.push_id(text(trigger,"triggerId"),|ui|{studio::card(ui).show(ui,|ui|{ui.strong(text(trigger,"name"));raw_hint(ui,text(trigger,"source"));ui.label(format!("{} / {}",trigger["occurrences"],trigger["maxOccurrences"]));if !text(trigger,"lastReasonCode").is_empty(){hint(ui,text(trigger,"lastReasonCode"));}
            if trigger["armed"]==true {quiet_action(ui,commands,"Pause trigger","project_trigger_set_armed",true,||json!({"projectId":self.create.project,"triggerId":trigger["triggerId"],"expectedTriggerRevision":trigger["revision"],"armed":false}));}
            else {let mut minutes=draft.get("armedMinutes").parse::<u64>().unwrap_or(60);ui.horizontal(|ui|{ui.label(t(ui,"Arm for minutes"));ui.add(egui::DragValue::new(&mut minutes).range(1..=1440));});*draft.value("armedMinutes")=minutes.to_string();quiet_action(ui,commands,"Arm trigger","project_trigger_set_armed",true,||json!({"projectId":self.create.project,"triggerId":trigger["triggerId"],"expectedTriggerRevision":trigger["revision"],"armed":true,"armedForMinutes":minutes}));}
        });});
            }
        }
        ui.add_space(studio::metrics::CONTROL_GAP);
        field(ui, "Trigger name", draft.value("name"), 80, false);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut draft.kind, 0, t(ui, "Folder changes"));
            ui.selectable_value(&mut draft.kind, 1, t(ui, "Public URL changes"));
        });
        if draft.kind == 0 {
            if view_control(ui, "Choose folder", true, false).clicked() {
                self.create.request_capability_paths_from(
                    CapabilityPathKind::TriggerFolder,
                    draft.get("source").to_owned(),
                );
            }
            raw_hint(ui, draft.get("source"));
            hint(
                ui,
                "Immediate files only: up to 32 files and 64 MiB; linked files are excluded.",
            );
        } else {
            field(ui, "Public URL", draft.value("source"), 4096, false);
        }
        let recipe = choose_recipe(ui, draft, recipes(snapshot, &self.create.project));
        let mut debounce = draft.get("debounce").parse::<u64>().unwrap_or(30);
        let mut occurrences = draft.get("occurrences").parse::<u64>().unwrap_or(10);
        ui.horizontal_wrapped(|ui| {
            ui.label(t(ui, "Debounce seconds"));
            ui.add(egui::DragValue::new(&mut debounce).range(10..=3600));
            ui.label(t(ui, "Maximum occurrences"));
            ui.add(egui::DragValue::new(&mut occurrences).range(1..=100));
        });
        *draft.value("debounce") = debounce.to_string();
        *draft.value("occurrences") = occurrences.to_string();
        let limits = budgets(ui, draft);
        let provider = self.create.resolved_planner_provider(snapshot);
        let payload=recipe.filter(|_|valid_input(draft.get("name"),80)&&valid_input(draft.get("source"),4096)&&provider.is_some()).map(|r|json!({"projectId":self.create.project,"name":draft.get("name"),"kind":if draft.kind==0{"folder"}else{"url"},"source":draft.get("source"),"recipeId":r["recipeId"],"expectedRecipeRevision":r["revision"],"debounceSeconds":debounce,"maxOccurrences":occurrences,"budgets":limits,"providerId":provider}));
        if payload.is_none() {
            hint(ui, "Complete the required fields to continue.");
        }
        send(
            ui,
            commands,
            &mut self.create.capabilities,
            &self.create.project,
            "Save paused trigger",
            "project_trigger_save",
            payload,
        );
    }
    fn variants_form(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: Option<&Value>,
        draft: &mut CapabilityDraft,
        commands: &mut Vec<Command>,
    ) {
        hint(
            ui,
            "Prepare two to four alternatives. Start each crew explicitly.",
        );
        if let Some(page) = page {
            for group in records(page, "variants") {
                ui.push_id(text(group,"variantGroupId"),|ui|{short_copy(ui,text(group,"instruction"),14.0,colors(ui).text,3);for variant in array(&group["variants"]){studio::card(ui).show(ui,|ui|{ui.strong(text(variant,"label"));if let Some(mission)=array(&snapshot["projectCrews"]["missions"]).iter().find(|m|m["missionId"]==variant["missionId"] && text(m,"projectId")==self.create.project){status(ui,text(mission,"status"));quiet_action(ui,commands,"Start crew","project_crew_start",text(mission,"status")=="prepared",||json!({"projectId":self.create.project,"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"]}));for task in array(&mission["tasks"]){if !text(task,"workItemId").is_empty() && view_control(ui,"Compare files",true,false).clicked(){self.create.selected_work.insert(self.create.project.clone(),text(task,"workItemId").into());self.create.select_productive_view(0);self.create.chat.files_open=true;}}}else{hint(ui,"Refresh to inspect the prepared crew.");}});}});
            }
        }
        let recipe = choose_recipe(ui, draft, recipes(snapshot, &self.create.project));
        multiline(ui, "Goal", draft.value("goal"), 20_000, 3);
        multiline(
            ui,
            "Alternative labels · one per line",
            draft.value("labels"),
            324,
            3,
        );
        let labels = lines(draft.get("labels"), 4, 80)
            .filter(|v| v.len() >= 2 && v.iter().collect::<HashSet<_>>().len() == v.len());
        let limits = budgets(ui, draft);
        let provider = self.create.resolved_planner_provider(snapshot);
        let payload=recipe.filter(|_|labels.is_some() && valid_input(draft.get("goal"),20_000)&&provider.is_some()).map(|r|json!({"projectId":self.create.project,"recipeId":r["recipeId"],"expectedRecipeRevision":r["revision"],"instruction":draft.get("goal"),"labels":labels,"budgets":limits,"providerId":provider}));
        if payload.is_none() {
            hint(ui, "Complete the required fields to continue.");
        }
        send(
            ui,
            commands,
            &mut self.create.capabilities,
            &self.create.project,
            "Prepare alternatives",
            "project_variants_prepare",
            payload,
        );
    }
    fn evidence_form(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        page: Option<&Value>,
        draft: &mut CapabilityDraft,
        commands: &mut Vec<Command>,
    ) {
        hint(ui,"Evidence binds a literal quote to a frozen source. It is not an independent truth score.");
        if let Some(page) = page {
            for evidence in records(page, "evidence") {
                ui.push_id(text(evidence,"evidenceId"),|ui|{ui.label(text(evidence,"claim"));quiet_action(ui,commands,"Read source evidence","project_evidence_read",true,||json!({"projectId":self.create.project,"evidenceId":evidence["evidenceId"]}));});
            }
        }
        let evidence = &snapshot["projectEvidence"];
        if text(evidence, "projectId") == self.create.project
            && text(evidence, "reasonCode") == "PROJECT_EVIDENCE_READ"
        {
            let record = if evidence["evidence"].is_object() {
                &evidence["evidence"]
            } else {
                evidence
            };
            studio::card(ui).show(ui, |ui| {
                ui.label(RichText::new(text(record, "claim")).strong());
                status(ui, text(record, "stance"));
                ui.label(text(record, "quote"));
                raw_hint(ui, text(record, "observedAt"));
                ui.collapsing(t(ui, "Actual source excerpt"), |ui| {
                    ui.add(
                        egui::Label::new(text(evidence, "sourceExcerpt"))
                            .wrap()
                            .selectable(true),
                    );
                    raw_hint(ui, text(&record["source"], "expectedSha256"));
                });
            });
        }
        ui.collapsing(t(ui,"Add evidence from a text source"),|ui|{
            let all=resources(snapshot,&self.create.project);
            form_combo(ui,egui::ComboBox::from_id_salt("evidence-source").selected_text(all.iter().find(|r|text(r,"resourceId")==draft.record).map(|r|text(r,"name")).unwrap_or(t(ui,"Choose a resource").as_ref())),|ui|{for r in all {ui.selectable_value(&mut draft.record,text(r,"resourceId").into(),text(r,"name"));}});
            let Some(source)=all.iter().find(|r|text(r,"resourceId")==draft.record)else{return;};
            quiet_action(ui,commands,"Read resource","project_resource_read",true,||json!({"projectId":self.create.project,"resourceId":source["resourceId"],"expectedSha256":source["sha256"]}));
            let read=&snapshot["projectResource"];
            let source_text=if text(read,"projectId")==self.create.project && read["resourceId"]==source["resourceId"] && (read["sha256"]==source["sha256"] || read["resource"]["sha256"]==source["sha256"]) {read["preview"]["text"].as_str()}else{None};
            if let Some(source_text)=source_text {ui.add(egui::Label::new(source_text).wrap().selectable(true));field(ui,"Claim",draft.value("claim"),2000,false);multiline(ui,"Literal quote",draft.value("quote"),4000,2);field(ui,"Location label",draft.value("location"),240,false);
                let quote=draft.get("quote");let span=quote_span(source_text,quote);
                ui.horizontal(|ui|{for(i,label)in ["supports","disputes","context"].iter().enumerate(){ui.selectable_value(&mut draft.kind,i,t(ui,label));}});
                let payload=span.filter(|_|valid_input(draft.get("claim"),2000)&&valid_input(draft.get("location"),240)).map(|(start,end)|json!({"projectId":self.create.project,"claim":draft.get("claim"),"source":{"resourceId":source["resourceId"],"expectedSha256":source["sha256"]},"quote":draft.get("quote"),"location":{"startByte":start,"endByte":end,"label":draft.get("location")},"observedAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"stance":(["supports","disputes","context"][draft.kind])}));
                send(ui,commands,&mut self.create.capabilities,&self.create.project,"Save source evidence","project_evidence_add",payload);
            }else{hint(ui,"Read the exact text resource before selecting a literal quote.");}
        });
    }
    pub(super) fn stage_operation(
        &mut self,
        _snapshot: &Value,
        purpose: &str,
        operation: Value,
        _commands: &mut Vec<Command>,
    ) {
        let instruction=format!("{purpose}\n\nRequested operation (use the typed production tool; report missing tools honestly):\n{}",serde_json::to_string_pretty(&operation).unwrap_or_default());
        let limit = self.create.conversation_limit();
        let current = self.create.conversation_input().clone();
        let combined = if current.trim().is_empty() {
            instruction
        } else {
            format!("{current}\n\n{instruction}")
        };
        if combined.encode_utf16().count() > limit {
            self.create.chat.conversation.set_error("This request is too long for the current conversation. Start a new conversation or shorten it.");
            return;
        }
        *self.create.conversation_input() = combined;
        self.create.crew.single = false;
        self.create.select_productive_view(0);
    }
    fn production_form(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        draft: &mut CapabilityDraft,
        commands: &mut Vec<Command>,
    ) {
        let options = [
            "Generate image",
            "Edit image",
            "Video package",
            "Interactive output",
            "Extract text (OCR)",
            "Transcribe audio",
            "Render document",
            "Batch images",
        ];
        form_combo(
            ui,
            egui::ComboBox::from_id_salt("production-operation")
                .selected_text(t(ui, options[draft.kind.min(options.len() - 1)])),
            |ui| {
                for (i, label) in options.iter().enumerate() {
                    ui.selectable_value(&mut draft.kind, i, t(ui, label));
                }
            },
        );
        hint(
            ui,
            "This adds a precise request to Chat. Send, review the plan and start explicitly.",
        );
        if studio::quiet_button(ui, t(ui, "Set up tools").as_ref()).clicked() {
            self.create.select_productive_view(2);
            self.create.studio.tab = 2;
            return;
        }

        let all = resources(snapshot, &self.create.project);
        if ![0, 3, 7].contains(&draft.kind) {
            form_combo(
                ui,
                egui::ComboBox::from_id_salt("production-source").selected_text(
                    all.iter()
                        .find(|r| text(r, "resourceId") == draft.record)
                        .map(|r| text(r, "name"))
                        .unwrap_or(t(ui, "Choose a resource").as_ref()),
                ),
                |ui| {
                    for r in all {
                        ui.selectable_value(
                            &mut draft.record,
                            text(r, "resourceId").into(),
                            text(r, "name"),
                        );
                    }
                },
            );
        }
        let source = all
            .iter()
            .find(|r| text(r, "resourceId") == draft.record && text(r, "status") == "available");
        let source_id = source.map(|r| {
            format!(
                "resource:{}",
                text(r, "resourceId").trim_start_matches("resource:")
            )
        });
        let mut operation = None;
        match draft.kind {
            0 | 1 => {
                multiline(ui, "Image prompt", draft.value("prompt"), 4000, 3);
                let mut width = draft.get("width").parse::<u64>().unwrap_or(1024);
                let mut height = draft.get("height").parse::<u64>().unwrap_or(1024);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Width"));
                    ui.add(
                        egui::DragValue::new(&mut width)
                            .range(64..=2048)
                            .speed(64.0),
                    );
                    ui.label(t(ui, "Height"));
                    ui.add(
                        egui::DragValue::new(&mut height)
                            .range(64..=2048)
                            .speed(64.0),
                    );
                });
                *draft.value("width") = width.to_string();
                *draft.value("height") = height.to_string();
                if draft.kind == 1 {
                    form_combo(
                        ui,
                        egui::ComboBox::from_id_salt("image-mask").selected_text(
                            all.iter()
                                .find(|r| text(r, "resourceId") == draft.get("mask"))
                                .map(|r| std::borrow::Cow::Borrowed(text(r, "name")))
                                .unwrap_or_else(|| t(ui, "No mask")),
                        ),
                        |ui| {
                            ui.selectable_value(
                                draft.value("mask"),
                                String::new(),
                                t(ui, "No mask"),
                            );
                            for r in all {
                                ui.selectable_value(
                                    draft.value("mask"),
                                    text(r, "resourceId").into(),
                                    text(r, "name"),
                                );
                            }
                        },
                    );
                    hint(ui,"Reference and mask must be PNGs with matching dimensions. White mask pixels are edited.");
                }
                let ready = &snapshot["projectStudio"]["imageEngine"];
                if !text(ready, "reasonCode").is_empty() {
                    hint(ui, text(ready, "reasonCode"));
                }
                if !text(ready, "checkpoint").is_empty() {
                    raw_hint(
                        ui,
                        &format!("{} · {}", t(ui, "Checkpoint"), text(ready, "checkpoint")),
                    );
                }
                hint(
                    ui,
                    if ready["available"] == true {
                        "Image engine is available."
                    } else {
                        "Configure a local image engine before running image generation."
                    },
                );
                ui.collapsing(t(ui,"Image engine settings"),|ui|{ui.label(t(ui,"Local engine URL"));let response=studio::input(ui,&mut self.create.capabilities.engine_url,2048,"",true);if let Some(mut edit)=egui::text_edit::TextEditState::load(ui.ctx(),response.id){edit.clear_undoer();edit.store(ui.ctx(),response.id);}field(ui,"Checkpoint",&mut self.create.capabilities.engine_checkpoint,240,false);let url=self.create.capabilities.engine_url.clone();let checkpoint=self.create.capabilities.engine_checkpoint.clone();quiet_action(ui,commands,"Save image engine","project_image_engine_set",valid_input(&url,2048)&&valid_input(&checkpoint,240),||json!({"projectId":self.create.project,"url":url,"checkpoint":checkpoint}));if commands.last().is_some_and(|c|c.action=="project_image_engine_set"){self.create.capabilities.engine_url.clear();}quiet_action(ui,commands,"Remove image engine","project_image_engine_remove",ready["configured"]==true,||json!({"projectId":self.create.project}));});
                if valid_input(draft.get("prompt"), 4000)
                    && width % 8 == 0
                    && height % 8 == 0
                    && (draft.kind == 0 || source.is_some())
                {
                    let mut args =
                        json!({"prompt":draft.get("prompt"),"width":width,"height":height});
                    if let Some(source) = source_id {
                        if draft.kind == 1 {
                            args["source"] = json!(source);
                            if !draft.get("mask").is_empty() {
                                args["mask"] = json!({"source":draft.get("mask")});
                            }
                        }
                    }
                    operation = Some(
                        json!({"operation":if draft.kind==0{"images.generate"}else{"images.edit"},"arguments":args}),
                    );
                }
            }
            2 => {
                field(ui, "Title", draft.value("title"), 160, false);
                let mut start = draft.get("start").parse::<f64>().unwrap_or(0.0);
                let mut end = draft.get("end").parse::<f64>().unwrap_or(10.0);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Clip start (seconds)"));
                    ui.add(egui::DragValue::new(&mut start).range(0.0..=86400.0));
                    ui.label(t(ui, "Clip end (seconds)"));
                    ui.add(egui::DragValue::new(&mut end).range(0.0..=86400.0));
                });
                *draft.value("start") = start.to_string();
                *draft.value("end") = end.to_string();
                multiline(
                    ui,
                    "Additional clips · start | end",
                    draft.value("clips"),
                    1000,
                    2,
                );
                for aspect in ["portrait", "square", "landscape"] {
                    let mut selected = draft.sources.contains(aspect);
                    if ui.checkbox(&mut selected, t(ui, aspect)).changed() {
                        if selected {
                            draft.sources.insert(aspect.into());
                        } else {
                            draft.sources.remove(aspect);
                        }
                    }
                }
                multiline(
                    ui,
                    "Captions · start | end | text",
                    draft.value("captions"),
                    6000,
                    3,
                );
                multiline(ui, "Description", draft.value("description"), 2000, 2);
                field(ui, "Hashtags", draft.value("hashtags"), 500, false);
                let captions: Option<Vec<Value>> = draft
                    .get("captions")
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|line| {
                        let mut parts = line.splitn(3, '|');
                        let start = parts.next()?.trim().parse::<f64>().ok()?;
                        let end = parts.next()?.trim().parse::<f64>().ok()?;
                        let text = parts.next()?.trim();
                        (start.is_finite()
                            && end.is_finite()
                            && start >= 0.0
                            && end > start
                            && !text.is_empty())
                        .then(|| json!({"start":start,"end":end,"text":text}))
                    })
                    .collect();
                let aspects: Vec<_> = ["portrait", "square", "landscape"]
                    .into_iter()
                    .filter(|a| draft.sources.contains(*a))
                    .collect();
                if let (Some(source), Some(captions)) = (source_id, captions) {
                    if end > start && !aspects.is_empty() && valid_input(draft.get("title"), 160) {
                        let extras: Option<Vec<Value>> = draft
                            .get("clips")
                            .lines()
                            .filter(|l| !l.trim().is_empty())
                            .map(|line| {
                                let (start, end) = line.split_once('|')?;
                                let start = start.trim().parse::<f64>().ok()?;
                                let end = end.trim().parse::<f64>().ok()?;
                                (start.is_finite()
                                    && end.is_finite()
                                    && start >= 0.0
                                    && end > start)
                                    .then(|| json!({"source":source,"start":start,"end":end}))
                            })
                            .collect();
                        if let Some(extras) = extras {
                            let mut clips = vec![json!({"source":source,"start":start,"end":end})];
                            clips.extend(extras);
                            if clips.len() <= 8 {
                                operation = Some(
                                    json!({"operation":"video.package","arguments":{"clips":clips,"aspects":aspects,"captions":captions,"title":draft.get("title"),"description":draft.get("description"),"hashtags":draft.get("hashtags").split_whitespace().collect::<Vec<_>>()}}),
                                );
                            }
                        }
                    }
                }
            }
            3 => {
                field(ui, "Title", draft.value("title"), 160, false);
                ui.horizontal(|ui| {
                    for (i, label) in ["table", "chart", "budget"].iter().enumerate() {
                        ui.selectable_value(
                            draft.value("interactive"),
                            label.to_string(),
                            t(ui, label),
                        );
                        let _ = i;
                    }
                });
                let kind = match draft.get("interactive") {
                    "chart" => "chart",
                    "budget" => "budget",
                    _ => "table",
                };
                if kind == "table" {
                    field(
                        ui,
                        "Columns · separated by |",
                        draft.value("columns"),
                        1000,
                        false,
                    );
                    multiline(
                        ui,
                        "Rows · one per line, cells separated by |",
                        draft.value("rows"),
                        12000,
                        4,
                    );
                } else {
                    multiline(ui, "Values · label | amount", draft.value("rows"), 12000, 4);
                    if kind == "budget" {
                        field(ui, "Currency", draft.value("currency"), 16, false);
                        field(ui, "Budget", draft.value("budget"), 32, false);
                    }
                }
                let rows: Vec<_> = draft
                    .get("rows")
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .collect();
                let mut args = json!({"kind":kind,"title":draft.get("title")});
                let valid = if kind == "table" {
                    let columns: Vec<_> = draft.get("columns").split('|').map(str::trim).collect();
                    let values: Vec<Vec<_>> = rows
                        .iter()
                        .map(|r| r.split('|').map(str::trim).collect())
                        .collect();
                    let valid = !columns.is_empty()
                        && !columns.iter().any(|c| c.is_empty())
                        && !values.is_empty()
                        && values.iter().all(|r| r.len() == columns.len());
                    args["columns"] = json!(columns);
                    args["rows"] = json!(values);
                    valid
                } else {
                    let values: Option<Vec<Value>> = rows
                        .iter()
                        .map(|r| {
                            let (label, value) = r.split_once('|')?;
                            let value = value.trim().parse::<f64>().ok()?;
                            if !value.is_finite() {
                                return None;
                            }
                            Some(if kind == "chart" {
                                json!({"label":label.trim(),"value":value})
                            } else {
                                json!({"label":label.trim(),"amount":value})
                            })
                        })
                        .collect();
                    if kind == "chart" {
                        args["series"] = json!(values);
                    } else {
                        args["items"] = json!(values);
                        args["currency"] = json!(draft.get("currency"));
                        args["budget"] = json!(draft.get("budget").parse::<f64>().ok());
                    }
                    values.is_some() && !rows.is_empty()
                };
                if valid && valid_input(draft.get("title"), 160) {
                    operation = Some(json!({"operation":"interactive.create","arguments":args}));
                }
            }
            7 => {
                hint(
                    ui,
                    "Select 2–12 images. Every source gets its own success or failure result.",
                );
                let images = all
                    .iter()
                    .filter(|resource| {
                        let name = text(resource, "name").to_ascii_lowercase();
                        text(resource, "status") == "available"
                            && (text(resource, "kind") == "image"
                                || [".png", ".jpg", ".jpeg", ".webp"]
                                    .iter()
                                    .any(|extension| name.ends_with(extension)))
                    })
                    .collect::<Vec<_>>();
                for resource in &images {
                    let id = text(resource, "resourceId");
                    let mut selected = draft.sources.contains(id);
                    if ui.checkbox(&mut selected, text(resource, "name")).changed() {
                        if selected {
                            draft.sources.insert(id.into());
                        } else {
                            draft.sources.remove(id);
                        }
                    }
                }
                let mut width = draft.get("width").parse::<u64>().unwrap_or(1600);
                let mut height = draft.get("height").parse::<u64>().unwrap_or(1600);
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Width"));
                    ui.add(
                        egui::DragValue::new(&mut width)
                            .range(64..=4096)
                            .speed(64.0),
                    );
                    ui.label(t(ui, "Height"));
                    ui.add(
                        egui::DragValue::new(&mut height)
                            .range(64..=4096)
                            .speed(64.0),
                    );
                });
                *draft.value("width") = width.to_string();
                *draft.value("height") = height.to_string();
                hint(ui, "Images keep their aspect ratio within these bounds.");
                let sources = images
                    .iter()
                    .filter(|resource| draft.sources.contains(text(resource, "resourceId")))
                    .map(|resource| json!({"source":text(resource,"resourceId")}))
                    .collect::<Vec<_>>();
                if (2..=12).contains(&sources.len()) {
                    operation = Some(
                        json!({"operation":"images.batch","arguments":{"sources":sources,"width":width,"height":height}}),
                    );
                }
            }
            _ => {
                field(
                    ui,
                    "Language (optional)",
                    draft.value("language"),
                    32,
                    false,
                );
                if let Some(source) = source_id {
                    let name = match draft.kind {
                        4 => "resources.ocr",
                        5 => "audio.transcribe",
                        _ => "documents.render",
                    };
                    let mut args = json!({"source":source});
                    if draft.kind != 6 && !draft.get("language").is_empty() {
                        args["language"] = json!(draft.get("language"));
                    }
                    operation = Some(json!({"operation":name,"arguments":args}));
                }
                hint(ui,"Results come from installed tools. Missing tools are reported; no text is invented.");
            }
        }
        if view_control(ui, "Add request to Chat", operation.is_some(), true).clicked() {
            if let Some(operation) = operation {
                self.stage_operation(
                    snapshot,
                    "Produce these deliverables using the exact Project sources.",
                    operation,
                    commands,
                );
            }
        }
    }
    fn browser_form(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        draft: &mut CapabilityDraft,
        commands: &mut Vec<Command>,
    ) {
        hint(ui,"Use an isolated browser to inspect a public page or prepare form values. Nothing is submitted.");
        field(ui, "Public URL", draft.value("url"), 4096, false);
        multiline(
            ui,
            "Form fields · selector | value",
            draft.value("fields"),
            8000,
            4,
        );
        let fields: Option<Vec<Value>> = draft
            .get("fields")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let (selector, value) = l.split_once('|')?;
                (!selector.trim().is_empty())
                    .then(|| json!({"selector":selector.trim(),"value":value.trim()}))
            })
            .collect();
        let valid =
            draft.get("url").starts_with("https://") || draft.get("url").starts_with("http://");
        let operation=fields.filter(|f|f.len()<=20).map(|fields|if fields.is_empty(){json!({"operation":"browser.inspect","arguments":{"url":draft.get("url")}})}else{json!({"operation":"browser.draft","arguments":{"url":draft.get("url"),"fields":fields}})});
        if view_control(
            ui,
            "Add browser request to Chat",
            valid && operation.is_some(),
            true,
        )
        .clicked()
        {
            self.stage_operation(snapshot,"Inspect this page or prepare a form draft. Do not submit forms or use an existing private browser profile.",operation.unwrap(),commands);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click_at_size, render, snapshot};
    use super::*;
    fn fixture() -> Value {
        let mut data = snapshot();
        data["projectCapabilities"] = json!({"reasonCode":"PROJECT_CAPABILITIES_READ","projectId":"project:one","revision":0,"profile":null,"skills":[],"triggers":[],"variants":[],"evidence":[]});
        data["projectStudio"] = json!({"reasonCode":"PROJECT_STUDIO_READ","projectId":"project:one","resources":[{"resourceId":"resource:source","revision":1,"name":"Source.txt","kind":"text","sha256":"a".repeat(64),"bytes":16,"status":"available"}]});
        data
    }
    fn console() -> Console {
        let mut console = Console::launch();
        console.create.project = "project:one".into();
        console.create.draft().goal = "Keep my unsent request".into();
        console.hold_automatic_reads = true;
        console
    }
    fn click_rect(
        ctx: &egui::Context,
        c: &mut Console,
        data: &Value,
        size: egui::Vec2,
        busy: bool,
        rect: egui::Rect,
    ) -> Vec<Command> {
        let pos = rect.center();
        render(
            ctx,
            c,
            data,
            size,
            busy,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        render(
            ctx,
            c,
            data,
            size,
            busy,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
        .0
    }
    #[test]
    fn attachment_chips_remove_only_the_focused_scoped_draft_while_host_is_busy() {
        for language in ["en", "tr"] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut data = fixture();
            data["desktopPreferences"] = json!({"language":language});
            let mut c = console();
            c.create
                .queue_paths(vec!["/tmp/one.pdf".into(), "/tmp/two.pdf".into()]);
            let foreign = ("project:two".into(), c.create.ability.clone());
            c.create
                .capabilities
                .attachments
                .insert(foreign.clone(), vec!["/tmp/private.pdf".into()]);
            let size = egui::vec2(800.0, 560.0);
            for _ in 0..3 {
                assert!(render(&ctx, &mut c, &data, size, true, vec![]).0.is_empty());
            }
            let (rect, id, clip) = ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Id, egui::Rect)>(egui::Id::new((
                    "composer-remove-attachment",
                    "/tmp/one.pdf",
                )))
                .unwrap()
            });
            assert!(clip.contains_rect(rect));
            let second_id = ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Id, egui::Rect)>(egui::Id::new((
                    "composer-remove-attachment",
                    "/tmp/two.pdf",
                )))
                .unwrap()
                .1
            });
            ctx.memory_mut(|memory| memory.request_focus(id));
            let commands = render(
                &ctx,
                &mut c,
                &data,
                size,
                true,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            )
            .0;
            assert!(commands.is_empty());
            assert_eq!(c.create.attachment_paths(), vec!["/tmp/two.pdf"]);
            render(&ctx, &mut c, &data, size, true, vec![]);
            let current_id = ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Id, egui::Rect)>(egui::Id::new((
                    "composer-remove-attachment",
                    "/tmp/two.pdf",
                )))
                .unwrap()
                .1
            });
            assert_eq!(
                second_id, current_id,
                "A different file must not inherit removal focus"
            );
            assert_eq!(
                c.create.capabilities.attachments[&foreign],
                vec!["/tmp/private.pdf"]
            );
            assert_eq!(c.create.conversation_input(), "Keep my unsent request");
            assert!(c.create.capabilities.pending.is_none());
        }
    }
    #[test]
    fn attachment_overflow_reveals_and_removes_a_file_hidden_from_the_composer() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut c = console();
        c.create
            .queue_paths((0..12).map(|i| format!("/tmp/source-{i:02}.pdf")).collect());
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let overflow = ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new("composer-attachment-overflow"))
                .unwrap()
        });
        assert!(click_rect(&ctx, &mut c, &data, size, false, overflow).is_empty());
        for _ in 0..2 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        assert!(egui::Popup::is_any_open(&ctx));
        let (remove, _, clip) = ctx.data(|d| {
            d.get_temp::<(egui::Rect, egui::Id, egui::Rect)>(egui::Id::new((
                "composer-remove-attachment",
                "/tmp/source-02.pdf",
            )))
            .unwrap()
        });
        assert!(
            clip.contains_rect(remove),
            "The hidden file is now reachable: {remove:?} in {clip:?}"
        );
        assert!(click_rect(&ctx, &mut c, &data, size, false, remove).is_empty());
        assert_eq!(c.create.attachment_paths().len(), 11);
        assert!(!c
            .create
            .attachment_paths()
            .contains(&"/tmp/source-02.pdf".into()));
        assert_eq!(c.create.conversation_input(), "Keep my unsent request");
    }
    #[test]
    fn brand_footer_stays_below_fields_and_explains_invalid_or_unchanged_values() {
        for language in ["en", "tr"] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut data = fixture();
            data["desktopPreferences"] = json!({"language":language});
            data["projectCapabilities"]["profile"] = json!({"revision":2,"tone":"Clear","instructions":"Readable","colours":["#2255FF"],"logoResources":[]});
            let mut c = console();
            c.create.select_productive_view(3);
            let size = egui::vec2(800.0, 560.0);
            for _ in 0..3 {
                render(&ctx, &mut c, &data, size, false, vec![]);
            }
            let (enabled, rect, clip) = ctx.data(|d| {
                d.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                    "test-action-state",
                    "project_brand_save",
                    "Save brand",
                )))
                .unwrap()
            });
            let fields = ctx.data(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new("brand-fields-bounds"))
                    .unwrap()
            });
            assert!(!enabled, "Unchanged values are not a new write");
            assert!(rect.top() >= fields.bottom());
            assert!(
                clip.contains_rect(rect),
                "Save stays visible: {rect:?} in {clip:?}"
            );
            assert!(click_at_size(
                &ctx,
                &mut c,
                &data,
                size,
                false,
                "project_brand_save",
                "Save brand"
            )
            .is_empty());
            let key = ("project:one".into(), 0);
            *c.create
                .capabilities
                .drafts
                .get_mut(&key)
                .unwrap()
                .value("colours") = "not-a-colour".into();
            let (commands, copy) = render(&ctx, &mut c, &data, size, false, vec![]);
            assert!(commands.is_empty());
            assert!(
                copy.iter().any(|s| s.contains("#RRGGBB")),
                "Validation explains the expected format"
            );
            assert!(click_at_size(
                &ctx,
                &mut c,
                &data,
                size,
                false,
                "project_brand_save",
                "Save brand"
            )
            .is_empty());
        }
    }
    #[test]
    fn editing_brand_keeps_a_saved_logo_outside_the_loaded_resource_page() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        let logos = json!([{"resourceId":"resource:older-logo","expectedSha256":"b".repeat(64)}]);
        data["projectCapabilities"]["profile"] = json!({"revision":3,"tone":"Saved tone","instructions":"","colours":[],"logoResources":logos});
        let mut c = console();
        c.create.select_productive_view(3);
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        *c.create
            .capabilities
            .drafts
            .get_mut(&("project:one".into(), 0))
            .unwrap()
            .value("tone") = "Clear tone".into();
        let commands = click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "project_brand_save",
            "Save brand",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].payload["logoResources"], logos,
            "Editing text must not silently clear a saved logo"
        );
        assert_eq!(commands[0].payload["expectedProfileRevision"], 3);
    }
    #[test]
    fn busy_frames_do_not_consume_project_tool_reads() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut c = console();
        c.hold_automatic_reads = false;
        c.create.capabilities.tab = 2;
        let frame = |c: &mut Console, enabled: bool| {
            let mut commands = Vec::new();
            c.command_busy = !enabled;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new("console-command-busy"), !enabled)
                });
                c.project_capabilities(ui, &data, &mut commands);
            });
            output.textures_delta.clear();
            commands
        };
        for expected in [
            "project_studio_read",
            "project_workflows_read",
            "project_capabilities_read",
        ] {
            let before = (
                c.create.capabilities.dependencies.clone(),
                c.create.capabilities.requested.clone(),
            );
            assert!(frame(&mut c, false).is_empty());
            assert_eq!(
                before,
                (
                    c.create.capabilities.dependencies.clone(),
                    c.create.capabilities.requested.clone()
                )
            );
            let commands = frame(&mut c, true);
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].action, expected);
        }
        assert!(frame(&mut c, true).is_empty());
    }
    #[test]
    fn native_picker_callbacks_preserve_scope_epoch_and_unsent_path_order() {
        let mut c = console();
        c.create.host_epoch = Some(json!(2));
        c.create
            .request_capability_paths(CapabilityPathKind::Resources);
        let request = c.take_capability_path_request().unwrap();
        let mut foreign = request.clone();
        foreign.request_id = "cmd:other".into();
        c.complete_capability_paths(foreign, Some(vec!["/tmp/foreign".into()]));
        assert!(c.create.attachment_paths().is_empty());
        c.complete_capability_paths(
            request,
            Some(vec![
                "/tmp/a.png".into(),
                "relative".into(),
                "/tmp/a.png".into(),
                "/tmp/b.wav".into(),
            ]),
        );
        assert_eq!(
            c.create.attachment_paths(),
            vec!["/tmp/a.png", "/tmp/b.wav"]
        );
        c.create
            .request_capability_paths(CapabilityPathKind::Resources);
        let late = c.take_capability_path_request().unwrap();
        c.create.project = "project:two".into();
        c.complete_capability_paths(late, Some(vec!["/tmp/private".into()]));
        assert!(c.create.attachment_paths().is_empty());
        c.create.project = "project:one".into();
        assert_eq!(c.create.attachment_paths().len(), 2);
        c.create
            .request_capability_paths(CapabilityPathKind::Resources);
        let late = c.take_capability_path_request().unwrap();
        c.create.host_epoch = Some(json!(3));
        c.complete_capability_paths(late, Some(vec!["/tmp/private".into()]));
        assert_eq!(c.create.attachment_paths().len(), 2);
    }
    #[test]
    fn project_tools_keep_brand_drafts_separate_and_save_exact_frozen_sources() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut c = console();
        c.create.select_productive_view(3);
        let draft = c
            .create
            .capabilities
            .drafts
            .entry(("project:one".into(), 0))
            .or_default();
        draft.loaded = true;
        *draft.value("tone") = "Clear".into();
        *draft.value("instructions") = "Use readable typography".into();
        *draft.value("colours") = "#2255FF".into();
        draft.sources.insert("resource:source".into());
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let commands = click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "project_brand_save",
            "Save brand",
        );
        assert_eq!(
            commands.len(),
            1,
            "save geometry {:?}",
            ctx.data(
                |d| d.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                    "test-action-state",
                    "project_brand_save",
                    "Save brand"
                )))
            )
        );
        assert_eq!(
            commands[0].payload["logoResources"],
            json!([{"resourceId":"resource:source","expectedSha256":"a".repeat(64)}])
        );
        assert_eq!(commands[0].payload["expectedProfileRevision"], 0);
        c.create.project = "project:two".into();
        assert!(!c
            .create
            .capabilities
            .drafts
            .contains_key(&("project:two".into(), 0)));
        assert_eq!(
            c.create.capabilities.drafts[&("project:one".into(), 0)].get("tone"),
            "Clear"
        );
        assert_eq!(c.create.draft().goal, "");
    }
    #[test]
    fn typed_production_request_is_staged_without_execution_or_overwriting_owner_text() {
        let mut c = console();
        let mut commands = Vec::new();
        c.stage_operation(&fixture(),"Make a chart",json!({"operation":"interactive.create","arguments":{"kind":"chart","title":"Actual","series":[{"label":"One","value":2}]}}),&mut commands);
        assert!(commands.is_empty());
        assert!(c
            .create
            .conversation_input()
            .starts_with("Keep my unsent request\n\nMake a chart"));
        assert!(c.create.conversation_input().contains("interactive.create"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn batch_image_request_freezes_two_selected_resources_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"cover.png","kind":"image","sha256":"a".repeat(64),"bytes":64,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"photo.jpg","kind":"image","sha256":"b".repeat(64),"bytes":64,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"notes.txt","kind":"text","sha256":"c".repeat(64),"bytes":64,"status":"available"}
        ]);
        let mut c = console();
        c.create.select_productive_view(3);
        c.create.capabilities.tab = 5;
        let draft = c
            .create
            .capabilities
            .drafts
            .entry(("project:one".into(), 5))
            .or_default();
        draft.kind = 7;
        draft
            .sources
            .insert("resource:11111111-1111-4111-8111-111111111111".into());
        draft
            .sources
            .insert("resource:22222222-2222-4222-8222-222222222222".into());
        *draft.value("width") = "1280".into();
        *draft.value("height") = "720".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let commands = click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "memory_control",
            "Add request to Chat",
        );
        assert!(commands.is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"images.batch\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("\"width\": 1280"));
        assert!(request.contains("\"height\": 720"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn visual_revision_freezes_source_provider_and_only_clears_the_acknowledged_input() {
        let mut c = console();
        let mut data = fixture();
        let scope = (c.create.project.clone(), c.create.ability.clone());
        let selection = VisualSelection {
            project: c.create.project.clone(),
            work: "work:one".into(),
            path: "outputs/real.png".into(),
            hash: "a".repeat(64),
            section: "region:0.100000,0.200000,0.300000,0.400000".into(),
        };
        c.create
            .capabilities
            .selection
            .insert(scope.clone(), selection.clone());
        let mut commands = Vec::new();
        c.prepare_visual_revision(&data, &mut commands);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_artifact_revise");
        assert_eq!(commands[0].payload["providerId"], "codex");
        assert_eq!(commands[0].payload["sectionId"], selection.section);
        assert_eq!(commands[0].payload["expectedSha256"], selection.hash);
        data["projectStudioReceipt"] =
            json!({"commandId":"cmd:foreign","projectId":"project:one","outcome":"applied"});
        c.create.sync(&data);
        assert!(c.create.visual_pending());
        data["projectStudioReceipt"]["commandId"] = commands[0].payload["commandId"].clone();
        c.create.draft().goal = "Later owner edit".into();
        c.create.sync(&data);
        assert!(!c.create.visual_pending());
        assert_eq!(c.create.draft().goal, "Later owner edit");
        assert_eq!(c.create.visual_selection(), Some(&selection));
        commands.clear();
        c.prepare_visual_revision(&data, &mut commands);
        data["projectStudioReceipt"]["commandId"] = commands[0].payload["commandId"].clone();
        c.create.sync(&data);
        assert_eq!(c.create.draft().goal, "");
        assert!(c.create.visual_selection().is_none());
    }
    #[test]
    fn primary_revision_action_and_attachment_summary_fit_minimum_window_in_both_locales_and_themes(
    ) {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language,"reduceMotion":true});
                let mut c = console();
                c.create.capabilities.selection.insert(
                    (c.create.project.clone(), c.create.ability.clone()),
                    VisualSelection {
                        project: c.create.project.clone(),
                        work: "work:one".into(),
                        path: "outputs/a.png".into(),
                        hash: "a".repeat(64),
                        section: "region:0.100000,0.100000,0.500000,0.500000".into(),
                    },
                );
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                let rect = ctx.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "conversation-revise",
                        "Prepare revision",
                    )))
                    .unwrap()
                });
                assert!(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(rect),
                    "{language}/{theme:?}: {rect:?}"
                );
                let commands = click_at_size(
                    &ctx,
                    &mut c,
                    &data,
                    size,
                    false,
                    "conversation-revise",
                    "Prepare revision",
                );
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].payload["projectId"], "project:one");
            }
        }
    }
    #[test]
    fn more_opens_optional_project_tools_without_sending_or_changing_the_goal() {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                let mut c = console();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                assert!(
                    click_at_size(&ctx, &mut c, &data, size, false, "memory_control", "More")
                        .is_empty()
                );
                for _ in 0..2 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                assert!(click_at_size(
                    &ctx,
                    &mut c,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Project tools"
                )
                .is_empty());
                assert_eq!(c.create.productive_view(), 3);
                assert_eq!(c.create.conversation_input(), "Keep my unsent request");
                c.create.select_productive_view(0);
                assert_eq!(c.create.conversation_input(), "Keep my unsent request");
            }
        }
    }
    #[test]
    fn literal_evidence_uses_utf8_bytes_and_never_invents_a_missing_quote() {
        let source = "Ön söz: gerçek 🦊 kanıt.";
        let quote = "gerçek 🦊";
        let (start, end) = quote_span(source, quote).unwrap();
        assert_eq!(source.get(start..end), Some(quote));
        assert_ne!(start, source[..start].chars().count());
        assert!(quote_span(source, "made up").is_none());
        assert!(quote_span(source, "").is_none());
    }
    #[test]
    fn saved_trigger_requires_explicit_arm_with_exact_revision_and_finite_minutes() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectCapabilities"]["triggers"] = json!([{"triggerId":"trigger:one","revision":7,"name":"Research changes","source":"https://example.test/news","armed":false,"occurrences":0,"maxOccurrences":3}]);
        let mut c = console();
        c.create.select_productive_view(3);
        c.create.capabilities.tab = 2;
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            let commands = render(&ctx, &mut c, &data, size, false, vec![]).0;
            assert!(
                commands.iter().all(|c| matches!(
                    c.action,
                    "project_studio_read" | "project_capabilities_read" | "project_workflows_read"
                )),
                "Opening this view must not mutate or start work"
            );
        }
        let commands = click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "project_trigger_set_armed",
            "Arm trigger",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["triggerId"], "trigger:one");
        assert_eq!(commands[0].payload["expectedTriggerRevision"], 7);
        assert_eq!(commands[0].payload["armed"], true);
        assert_eq!(commands[0].payload["armedForMinutes"], 60);
    }
    #[test]
    fn alternatives_prepare_only_and_freeze_selected_recipe_labels_and_actual_provider() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectWorkflowRead"] = json!({"projectId":"project:one","section":"recipes","items":[{"recipeId":"recipe:real","revision":9,"name":"Launch crew"}]});
        let mut c = console();
        c.create.select_productive_view(3);
        c.create.capabilities.tab = 3;
        let draft = c
            .create
            .capabilities
            .drafts
            .entry(("project:one".into(), 3))
            .or_default();
        draft.recipe = "recipe:real".into();
        *draft.value("goal") = "Compare two launch directions".into();
        *draft.value("labels") = "Precise\nPlayful".into();
        let size = egui::vec2(1280.0, 880.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let commands = click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "project_variants_prepare",
            "Prepare alternatives",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_variants_prepare");
        assert_eq!(commands[0].payload["recipeId"], "recipe:real");
        assert_eq!(commands[0].payload["expectedRecipeRevision"], 9);
        assert_eq!(commands[0].payload["labels"], json!(["Precise", "Playful"]));
        assert_eq!(commands[0].payload["providerId"], "codex");
    }
    #[test]
    fn replacing_an_existing_trigger_folder_uses_the_active_draft_and_preserves_later_edits() {
        let mut c = console();
        let key = ("project:one".into(), 2);
        let mut active = CapabilityDraft::default();
        *active.value("source") = "/tmp/old-folder".into();
        // Rendering temporarily owns the active draft outside the map.
        c.create.request_capability_paths_from(
            CapabilityPathKind::TriggerFolder,
            active.get("source").to_owned(),
        );
        let request = c.take_capability_path_request().unwrap();
        assert_eq!(request.source_before, "/tmp/old-folder");
        c.create.capabilities.drafts.insert(key.clone(), active);
        c.complete_capability_paths(request, Some(vec!["/tmp/replacement".into()]));
        assert_eq!(
            c.create.capabilities.drafts[&key].get("source"),
            "/tmp/replacement"
        );
        let active = c.create.capabilities.drafts.remove(&key).unwrap();
        c.create.request_capability_paths_from(
            CapabilityPathKind::TriggerFolder,
            active.get("source").to_owned(),
        );
        let request = c.take_capability_path_request().unwrap();
        c.create.capabilities.drafts.insert(key.clone(), active);
        *c.create
            .capabilities
            .drafts
            .get_mut(&key)
            .unwrap()
            .value("source") = "/tmp/later-owner-edit".into();
        c.complete_capability_paths(request, Some(vec!["/tmp/stale-picker-choice".into()]));
        assert_eq!(
            c.create.capabilities.drafts[&key].get("source"),
            "/tmp/later-owner-edit"
        );
    }
    #[test]
    fn terminal_quiet_mutations_reload_once_and_read_errors_do_not_loop() {
        let mut c = console();
        let mut data = fixture();
        let requested = ("project:one".into(), 2, 0);
        c.create.capabilities.requested = Some(requested.clone());
        data["projectCapabilityReceipt"] = json!({"commandId":"cmd:arm","projectId":"project:one","action":"project_trigger_set_armed","outcome":"applied"});
        c.create.sync(&data);
        assert!(c.create.capabilities.requested.is_none());
        c.create.capabilities.requested = Some(requested.clone());
        c.create.sync(&data);
        assert_eq!(c.create.capabilities.requested, Some(requested.clone()));
        data["projectCapabilityError"] = json!({"commandId":"cmd:read-failed","projectId":"project:one","action":"project_capabilities_read","outcome":"rejected"});
        c.create.sync(&data);
        assert_eq!(c.create.capabilities.requested, Some(requested));
        for (i, action) in [
            "project_image_engine_set",
            "project_image_engine_remove",
            "project_resource_import_batch",
            "project_resource_from_artifact",
            "project_resources_from_artifacts",
        ]
        .iter()
        .enumerate()
        {
            let dependency = ("project:one".into(), "project_studio_read");
            c.create
                .capabilities
                .dependencies
                .insert(dependency.clone());
            c.create.studio.requested = Some(("project:one".into(), 2));
            data["projectStudioReceipt"] = json!({"projectId":"project:one","commandId":format!("cmd:studio-{i}"),"action":action,"outcome":"unknown"});
            c.create.sync(&data);
            assert!(c.create.studio.requested.is_some());
            assert!(c.create.capabilities.dependencies.contains(&dependency));
            data["projectStudioReceipt"]["outcome"] = json!("applied");
            c.create.sync(&data);
            assert!(c.create.studio.requested.is_none());
            assert!(!c.create.capabilities.dependencies.contains(&dependency));
        }
    }
    #[test]
    fn blank_brand_fields_can_save_logo_colours_or_explicitly_clear_old_style() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectCapabilities"]["profile"] = json!({"revision":4,"tone":"Old tone","instructions":"Old instructions","colours":["#112233"]});
        let mut c = console();
        c.create.select_productive_view(3);
        let draft = c
            .create
            .capabilities
            .drafts
            .entry(("project:one".into(), 0))
            .or_default();
        draft.loaded = true;
        *draft.value("colours") = "#2255FF".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let commands = click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "project_brand_save",
            "Save brand",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["expectedProfileRevision"], 4);
        assert_eq!(commands[0].payload["tone"], "");
        assert_eq!(commands[0].payload["instructions"], "");
        assert_eq!(commands[0].payload["colours"], json!(["#2255FF"]));
        assert!(!optional_text("bad\0value", 500));
    }
    #[test]
    fn queued_context_cannot_be_silently_sent_as_a_single_task_and_expanded_files_stay_bounded() {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectChat"]["items"] =
                    json!([{"workItemId":"work:one","goal":"Earlier request"}]);
                let mut c = console();
                c.create
                    .queue_paths((0..12).map(|i| format!("/tmp/source-{i:02}.wav")).collect());
                c.create.capabilities.selection.insert(
                    (c.create.project.clone(), c.create.ability.clone()),
                    VisualSelection {
                        project: c.create.project.clone(),
                        work: "work:one".into(),
                        path: "outputs/actual.png".into(),
                        hash: "a".repeat(64),
                        section: "region:0.100000,0.100000,0.500000,0.500000".into(),
                    },
                );
                c.create.chat.replies.insert(
                    (c.create.project.clone(), c.create.ability.clone()),
                    ReplyDraft {
                        parent: "work:one".into(),
                        references: HashSet::new(),
                    },
                );
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                assert!(
                    ctx.data(|d| d
                        .get_temp::<egui::Rect>(egui::Id::new((
                            "test-action",
                            "memory_control",
                            "Clear follow-up"
                        )))
                        .is_none()),
                    "Revision shows only its active context"
                );
                assert_eq!(
                    c.create.chat.replies.len(),
                    1,
                    "The prior follow-up draft remains saved"
                );
                let (rect, clip) = ctx.data(|d| {
                    d.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new("composer-context-bounds"))
                        .unwrap()
                });
                assert!(
                    clip.contains_rect(rect),
                    "Context is fully visible: {language}/{theme:?}: {rect:?} inside {clip:?}"
                );
                let (remove_attachments, remove_clip) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new(
                        "composer-remove-attachments",
                    ))
                    .unwrap()
                });
                assert!(
                    remove_clip.contains_rect(remove_attachments),
                    "Both conflict resolutions remain visible"
                );
                let (clear, clear_clip) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new(
                        "composer-clear-selection",
                    ))
                    .unwrap()
                });
                assert!(
                    clear_clip.contains_rect(clear),
                    "The conflict can be cleared without scrolling: {language}/{theme:?}"
                );
                let primary = ctx.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "conversation-revise",
                        "Prepare revision",
                    )))
                    .unwrap()
                });
                assert!(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(primary),
                    "{language}/{theme:?}: {primary:?}"
                );
                assert!(c.create.chat_payload(&data).is_none());
                c.create.crew.single = true;
                assert!(c.create.chat_payload(&data).is_none());
                c.create.crew.single = false;
                click_at_size(&ctx, &mut c, &data, size, false, "memory_control", "More");
                render(&ctx, &mut c, &data, size, false, vec![]);
                let (_, enabled) = ctx.data(|d| {
                    d.get_temp::<(egui::Rect, bool)>(egui::Id::new("conversation-single-option"))
                        .unwrap()
                });
                assert!(!enabled);
                assert!(!c.create.crew.single);
            }
        }
    }
}
