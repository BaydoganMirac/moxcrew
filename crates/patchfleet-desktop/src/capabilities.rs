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
fn podcast_source(resource: &Value) -> bool {
    let name = text(resource, "name").to_ascii_lowercase();
    [".wav", ".mp3", ".m4a", ".mp4", ".mov", ".mkv", ".webm"]
        .iter()
        .any(|extension| name.ends_with(extension))
}
fn narration_source(resource: &Value) -> bool {
    let name = text(resource, "name").to_ascii_lowercase();
    [".wav", ".mp3", ".m4a"]
        .iter()
        .any(|extension| name.ends_with(extension))
}
fn video_source(resource: &Value) -> bool {
    let name = text(resource, "name").to_ascii_lowercase();
    [".mp4", ".mov", ".mkv", ".webm"]
        .iter()
        .any(|extension| name.ends_with(extension))
}
fn composition_image_source(resource: &Value) -> bool {
    let name = text(resource, "name").to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".webp"]
        .iter()
        .any(|extension| name.ends_with(extension))
}
fn chart_source(resource: &Value) -> bool {
    text(resource, "name")
        .to_ascii_lowercase()
        .ends_with(".csv")
}
fn template_source(resource: &Value) -> bool {
    text(resource, "name")
        .to_ascii_lowercase()
        .ends_with(".docx")
}
fn rewrite_source(resource: &Value) -> bool {
    let name = text(resource, "name").to_ascii_lowercase();
    [".txt", ".md", ".docx"]
        .iter()
        .any(|extension| name.ends_with(extension))
}
fn pdf_source(resource: &Value) -> bool {
    text(resource, "name")
        .to_ascii_lowercase()
        .ends_with(".pdf")
}
fn presentation_source(resource: &Value) -> bool {
    text(resource, "name")
        .to_ascii_lowercase()
        .ends_with(".pptx")
}
fn presentation_logo_source(resource: &Value) -> bool {
    let name = text(resource, "name").to_ascii_lowercase();
    [".png", ".jpg", ".jpeg"]
        .iter()
        .any(|extension| name.ends_with(extension))
}
fn presentation_hex(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('#').to_ascii_uppercase();
    (value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(value)
}
fn presentation_slides(value: &str, chart_source: &str) -> Option<Vec<Value>> {
    let rows = value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if !(1..=12).contains(&rows.len()) {
        return None;
    }
    rows.into_iter()
        .map(|row| {
            let mut parts = row.splitn(4, '|').map(str::trim);
            let layout = parts.next()?;
            let title = parts.next()?;
            let content = parts.next().unwrap_or_default();
            let extra = parts.next().unwrap_or_default();
            if !valid_input(title, 160) {
                return None;
            }
            match layout {
                "cover" | "statement" => valid_input(content, 240)
                    .then(|| json!({"layout":layout,"title":title,"subtitle":content})),
                "content" => {
                    let bullets = content
                        .split(';')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .collect::<Vec<_>>();
                    (!bullets.is_empty()
                        && bullets.len() <= 6
                        && bullets.iter().all(|item| valid_input(item, 220)))
                    .then(|| json!({"layout":"content","title":title,"bullets":bullets}))
                }
                "bar-chart" => {
                    let series = content
                        .split(';')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(|item| {
                            let (label, value) = item.split_once('=')?;
                            let value = value.trim().parse::<f64>().ok()?;
                            (valid_input(label.trim(), 60)
                                && value.is_finite()
                                && (0.0..=1e12).contains(&value))
                            .then(|| json!({"label":label.trim(),"value":value}))
                        })
                        .collect::<Option<Vec<_>>>()?;
                    if series.is_empty()
                        || series.len() > 6
                        || !valid_input(extra, 16)
                        || !valid_input(chart_source, 240)
                    {
                        return None;
                    }
                    Some(json!({"layout":"bar-chart","title":title,"chart":{
                        "series":series,"unit":extra,"sourceLabel":chart_source
                    }}))
                }
                _ => None,
            }
        })
        .collect()
}
fn presentation_brand_fields(ui: &mut Ui, draft: &mut CapabilityDraft) -> Option<Value> {
    for (key, value) in [
        ("brandName", "Moxcrew"),
        ("primary", "2854F4"),
        ("accent", "FF7A2F"),
        ("background", "F8F7F3"),
        ("text", "15223D"),
        ("muted", "56627A"),
        ("font", "Arial"),
    ] {
        if draft.get(key).is_empty() {
            *draft.value(key) = value.into();
        }
    }
    field(ui, "Brand name", draft.value("brandName"), 80, false);
    field(ui, "Theme font", draft.value("font"), 80, false);
    ui.label(t(ui, "Theme colours · hexadecimal"));
    for (label, key) in [
        ("Primary", "primary"),
        ("Accent", "accent"),
        ("Background", "background"),
        ("Text", "text"),
        ("Muted text", "muted"),
    ] {
        ui.horizontal(|ui| {
            ui.label(t(ui, label));
            studio::input(ui, draft.value(key), 7, "2854F4", false);
        });
    }
    let colors = ["primary", "accent", "background", "text", "muted"]
        .map(|key| presentation_hex(draft.get(key)))
        .into_iter()
        .collect::<Option<Vec<_>>>()?;
    (valid_input(draft.get("brandName"), 80) && valid_input(draft.get("font"), 80)).then(|| {
        json!({
            "name":draft.get("brandName"),"font":draft.get("font"),
            "primary":colors[0],"accent":colors[1],"background":colors[2],
            "text":colors[3],"muted":colors[4]
        })
    })
}
fn extraction_source(resource: &Value) -> bool {
    let name = text(resource, "name").to_ascii_lowercase();
    [
        ".txt", ".md", ".csv", ".json", ".html", ".htm", ".srt", ".pdf", ".png", ".jpg", ".jpeg",
        ".webp",
    ]
    .iter()
    .any(|extension| name.ends_with(extension))
}
fn extraction_key(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
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
        if scene == "capabilities-transcription" {
            self.create.ability = "A1".into();
        } else if scene == "capabilities-audio-cleanup" {
            self.create.ability = "A3".into();
        } else if scene == "capabilities-data-profile" {
            self.create.ability = "T1".into();
        } else if matches!(
            scene,
            "capabilities-video-compose"
                | "capabilities-video-highlights"
                | "capabilities-video-motion"
        ) {
            self.create.ability = match scene {
                "capabilities-video-highlights" => "V2",
                "capabilities-video-motion" => "V5",
                _ => "V4",
            }
            .into();
        }
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
            | "capabilities-podcast"
            | "capabilities-data-chart"
            | "capabilities-data-scenario"
            | "capabilities-data-extract"
            | "capabilities-documents-brief"
            | "capabilities-claim-check"
            | "capabilities-research-synthesis"
            | "capabilities-research-comparison"
            | "capabilities-web-research"
            | "capabilities-pdf-merge"
            | "capabilities-presentations"
            | "capabilities-presentation-brand"
            | "capabilities-presentation-narration"
            | "capabilities-video-compose"
            | "capabilities-video-highlights"
            | "capabilities-video-motion"
            | "capabilities-transcription"
            | "capabilities-audio-cleanup"
            | "capabilities-data-profile"
            | "capabilities-source-changes"
            | "capabilities-documents-rewrite"
            | "capabilities-documents-template"
            | "capabilities-interactive" => 5,
            "capabilities-browser" => 6,
            _ => 0,
        };
        self.create.capabilities.requested =
            Some((self.create.project.clone(), self.create.capabilities.tab, 0));
        if self.create.capabilities.tab == 5 {
            let project = self.create.project.clone();
            let available = resources(snapshot, &project);
            let selected = available
                .iter()
                .map(|resource| text(resource, "resourceId").to_owned())
                .collect::<Vec<_>>();
            let audio = available
                .iter()
                .find(|resource| podcast_source(resource))
                .map(|resource| text(resource, "resourceId").to_owned());
            let csv = available
                .iter()
                .find(|resource| chart_source(resource))
                .map(|resource| text(resource, "resourceId").to_owned());
            let change_sources = available
                .iter()
                .filter(|resource| extraction_source(resource))
                .take(2)
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
                "capabilities-podcast" => 8,
                "capabilities-data-chart" => 9,
                "capabilities-data-scenario" => 10,
                "capabilities-data-extract" => 11,
                "capabilities-documents-brief" => 12,
                "capabilities-documents-template" => 13,
                "capabilities-documents-rewrite" => 14,
                "capabilities-source-changes" => 15,
                "capabilities-claim-check" => 16,
                "capabilities-research-synthesis" => 17,
                "capabilities-research-comparison" => 18,
                "capabilities-web-research" => 19,
                "capabilities-pdf-merge" => 21,
                "capabilities-presentations" => 22,
                "capabilities-presentation-brand" => 23,
                "capabilities-presentation-narration" => 24,
                "capabilities-video-compose" => 25,
                "capabilities-video-highlights" => 26,
                "capabilities-video-motion" => 27,
                "capabilities-transcription" => 5,
                "capabilities-audio-cleanup" => 28,
                "capabilities-data-profile" => 29,
                _ => 0,
            };
            if scene == "capabilities-batch-images" {
                draft.sources.extend(selected);
                *draft.value("width") = "1600".into();
                *draft.value("height") = "1600".into();
            } else if scene == "capabilities-podcast" {
                draft.record = audio.unwrap_or_default();
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Signal & Craft · Episode 04"
                } else {
                    "Sinyal ve Zanaat · Bölüm 04"
                }
                .into();
                *draft.value("segments") = if english {
                    "0 | 18.5 | Opening\n42 | 96 | The core idea\n128 | 176 | Takeaways"
                } else {
                    "0 | 18.5 | Açılış\n42 | 96 | Ana fikir\n128 | 176 | Çıkarımlar"
                }
                .into();
            } else if scene == "capabilities-data-chart" {
                draft.record = csv.unwrap_or_default();
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Revenue by region"
                } else {
                    "Bölgelere göre gelir"
                }
                .into();
                *draft.value("categoryColumn") = "region".into();
                *draft.value("valueColumn") = "revenue".into();
                *draft.value("aggregation") = "sum".into();
            } else if scene == "capabilities-presentations" {
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                draft.record = available
                    .iter()
                    .find(|resource| presentation_logo_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                *draft.value("brandName") = "Moxcrew".into();
                *draft.value("primary") = "2854F4".into();
                *draft.value("accent") = "FF7A2F".into();
                *draft.value("background") = "F8F7F3".into();
                *draft.value("text") = "15223D".into();
                *draft.value("muted") = "56627A".into();
                *draft.value("font") = "Arial".into();
                *draft.value("chartSource") = if english {
                    "Source: checked pilot dataset"
                } else {
                    "Kaynak: kontrol edilmiş pilot veri seti"
                }
                .into();
                *draft.value("presentationSlides") = if english {
                    "cover | Work with a crew you can inspect | One local project, one visible plan\ncontent | What stays under your control | Sources stay local; Preparation remains explicit; Every output keeps its version\nbar-chart | Pilot completion | Before=42; With Mox=78 | %"
                } else {
                    "cover | Denetleyebildiğin bir ekiple çalış | Tek yerel proje, görünür bir plan\ncontent | Kontrol sende kalır | Kaynaklar yerelde kalır; Hazırlık açıkça yapılır; Her çıktı sürümünü korur\nbar-chart | Pilot tamamlama oranı | Önce=42; Mox ile=78 | %"
                }
                .into();
                *draft.value("renderSlides") = "true".into();
            } else if scene == "capabilities-presentation-narration" {
                draft.record = available
                    .iter()
                    .find(|resource| presentation_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                draft.sources.extend(
                    available
                        .iter()
                        .filter(|resource| narration_source(resource))
                        .map(|resource| text(resource, "resourceId").to_owned()),
                );
                *draft.value("presentationFps") = "30".into();
            } else if scene == "capabilities-video-compose" {
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                draft.record = available
                    .iter()
                    .find(|resource| video_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                *draft.value("compositionImage") = available
                    .iter()
                    .find(|resource| composition_image_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                *draft.value("compositionAudio") = available
                    .iter()
                    .find(|resource| narration_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                *draft.value("compositionTitle") = if english {
                    "Every handoff stays visible"
                } else {
                    "Her teslim görünür kalır"
                }
                .into();
                *draft.value("compositionTitleSeconds") = "2.5".into();
                *draft.value("compositionOverlayStart") = "3".into();
                *draft.value("compositionOverlayEnd") = "12".into();
                *draft.value("compositionPosition") = "bottom-right".into();
                *draft.value("compositionAudioMode") = "replace".into();
                *draft.value("compositionAudioLevel") = "0.8".into();
            } else if scene == "capabilities-video-highlights" {
                draft.record = available
                    .iter()
                    .find(|resource| video_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                *draft.value("highlightTargetSeconds") = "8".into();
                *draft.value("highlightClipSeconds") = "4".into();
                *draft.value("highlightEmphasis") = "balanced".into();
            } else if scene == "capabilities-video-motion" {
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("motionTitle") = if english {
                    "Mox keeps work visible"
                } else {
                    "Mox işi görünür tutar"
                }
                .into();
                *draft.value("motionTreatment") = "drift".into();
                *draft.value("motionFps") = "30".into();
                *draft.value("motionScenes") = if english {
                    "cover | 3 | MEET MOX | A crew you can actually see | Plan, produce and review in one local workspace.\nstatement | 3 | OWNER CONTROL | Every handoff stays visible. | Nothing starts until you say so.\nstatement | 3 | REAL OUTPUTS | Files, checks and revisions—not mystery progress. | Each delivery keeps its evidence.\ncover | 3 | MOXCREW | Ship with proof. | Keep the final call."
                } else {
                    "cover | 3 | MOX İLE TANIŞ | Gerçekten görebildiğin bir ekip | Tek yerel çalışma alanında planla, üret ve incele.\nstatement | 3 | KONTROL SENDE | Her devir görünür kalır. | Sen söylemeden hiçbir şey başlamaz.\nstatement | 3 | GERÇEK ÇIKTILAR | Gizemli ilerleme değil; dosya, kontrol ve sürüm. | Her teslimat kanıtını korur.\ncover | 3 | MOXCREW | Kanıtla teslim et. | Son karar sende kalsın."
                }
                .into();
            } else if scene == "capabilities-transcription" {
                draft.record = available
                    .iter()
                    .find(|resource| podcast_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                *draft.value("language") = "tr".into();
            } else if scene == "capabilities-audio-cleanup" {
                draft.record = available
                    .iter()
                    .find(|resource| podcast_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                *draft.value("cleanupTargetLufs") = "-16".into();
                *draft.value("cleanupTreatment") = "voice".into();
            } else if scene == "capabilities-data-profile" {
                draft.record = csv.unwrap_or_default();
                *draft.value("dataDeduplicate") = "true".into();
                *draft.value("dataNumericColumns") = "revenue, units".into();
                *draft.value("dataOutput") = "xlsx".into();
            } else if scene == "capabilities-data-scenario" {
                draft.record = csv.unwrap_or_default();
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "2027 operating plan"
                } else {
                    "2027 faaliyet planı"
                }
                .into();
                *draft.value("scenarioColumns") =
                    "line_item | base_plan | growth_case | lean_case".into();
                *draft.value("currency") = "TRY".into();
            } else if scene == "capabilities-data-extract" {
                draft.sources.extend(
                    available
                        .iter()
                        .filter(|resource| extraction_source(resource))
                        .map(|resource| text(resource, "resourceId").to_owned()),
                );
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Vendor renewal review"
                } else {
                    "Tedarikçi yenileme incelemesi"
                }
                .into();
                *draft.value("extractionColumns") = if english {
                    "vendor: Vendor | renewal: Renewal | seats: Seats"
                } else {
                    "vendor: Tedarikçi | renewal: Yenileme | seats: Kullanıcı"
                }
                .into();
            } else if scene == "capabilities-documents-brief" {
                draft.sources.extend(
                    available
                        .iter()
                        .filter(|resource| extraction_source(resource))
                        .map(|resource| text(resource, "resourceId").to_owned()),
                );
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Pilot decision brief"
                } else {
                    "Pilot karar özeti"
                }
                .into();
            } else if scene == "capabilities-documents-template" {
                draft.record = available
                    .iter()
                    .find(|resource| template_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("templateFields") = if english {
                    "client_name | Atlas Cloud\napproval_date |\ncontact_email | ops@atlas.example"
                } else {
                    "client_name | Atlas Bulut\napproval_date |\ncontact_email | operasyon@atlas.example"
                }
                .into();
            } else if scene == "capabilities-documents-rewrite" {
                draft.record = available
                    .iter()
                    .find(|resource| rewrite_source(resource))
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .unwrap_or_default();
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("rewriteMode") = "translate".into();
                *draft.value("rewriteTarget") =
                    if english { "Turkish" } else { "İngilizce" }.into();
                *draft.value("preserveTerms") = "Moxcrew, Launch OS".into();
            } else if scene == "capabilities-source-changes" {
                draft.record = change_sources.first().cloned().unwrap_or_default();
                *draft.value("afterSource") = change_sources.get(1).cloned().unwrap_or_default();
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Launch source changes"
                } else {
                    "Lansman kaynağı değişiklikleri"
                }
                .into();
                *draft.value("beforeDate") = "2027-05-01".into();
                *draft.value("afterDate") = "2027-06-01".into();
            } else if scene == "capabilities-claim-check" {
                draft.sources.extend(
                    available
                        .iter()
                        .filter(|resource| extraction_source(resource))
                        .map(|resource| text(resource, "resourceId").to_owned()),
                );
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Launch claim check"
                } else {
                    "Lansman iddia doğrulaması"
                }
                .into();
                *draft.value("claims") = if english {
                    "The launch reached 72% adoption.\nRetention lasted twelve months."
                } else {
                    "Lansman %72 benimsenmeye ulaştı.\nKullanım on iki ay sürdü."
                }
                .into();
            } else if scene == "capabilities-research-synthesis" {
                draft.sources.extend(
                    available
                        .iter()
                        .filter(|resource| extraction_source(resource))
                        .map(|resource| text(resource, "resourceId").to_owned()),
                );
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Launch source synthesis"
                } else {
                    "Lansman kaynak sentezi"
                }
                .into();
                *draft.value("synthesisFocus") = if english {
                    "Where do customer needs and pricing evidence agree or conflict?"
                } else {
                    "Müşteri ihtiyaçları ve fiyat kanıtları nerede uyuşuyor veya çelişiyor?"
                }
                .into();
            } else if scene == "capabilities-research-comparison" {
                draft.sources.extend(
                    available
                        .iter()
                        .filter(|resource| extraction_source(resource))
                        .map(|resource| text(resource, "resourceId").to_owned()),
                );
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Vendor comparison"
                } else {
                    "Tedarikçi karşılaştırması"
                }
                .into();
                *draft.value("comparisonQuestion") = if english {
                    "Which vendor best fits the owner's launch criteria?"
                } else {
                    "Hangi tedarikçi sahibin lansman kriterlerine en uygun?"
                }
                .into();
                *draft.value("comparisonCriteria") = if english {
                    "cost: Monthly cost | setup: Setup speed | offline: Offline export"
                } else {
                    "cost: Aylık maliyet | setup: Kurulum hızı | offline: Çevrimdışı dışa aktarma"
                }
                .into();
                *draft.value("comparisonOptions") = "Atlas | Boreal".into();
            } else if scene == "capabilities-web-research" {
                let english = std::env::var("PATCHFLEET_SMOKE_LANGUAGE").as_deref() == Ok("en");
                *draft.value("title") = if english {
                    "Launch market research"
                } else {
                    "Lansman pazar araştırması"
                }
                .into();
                *draft.value("webResearchQuestion") = if english {
                    "What do the public launch sources establish about demand and pricing?"
                } else {
                    "Herkese açık lansman kaynakları talep ve fiyatlama hakkında ne gösteriyor?"
                }
                .into();
                *draft.value("webResearchSources") =
                    "Market bulletin | https://example.com/market\nPricing brief | https://example.org/pricing"
                        .into();
                *draft.value("webResearchClaims") = if english {
                    "Teams request weekly summaries.\nThe public price is 120 TRY."
                } else {
                    "Ekipler haftalık özet istiyor.\nHerkese açık fiyat 120 TRY."
                }
                .into();
            } else if scene == "capabilities-pdf-merge" {
                let mut pdfs = available
                    .iter()
                    .filter(|resource| pdf_source(resource))
                    .map(|resource| {
                        (
                            text(resource, "name").to_owned(),
                            text(resource, "resourceId").to_owned(),
                        )
                    })
                    .collect::<Vec<_>>();
                pdfs.sort_by(|left, right| left.0.cmp(&right.0));
                let pdfs = pdfs.into_iter().map(|(_, id)| id).collect::<Vec<_>>();
                draft.sources.extend(pdfs.iter().cloned());
                *draft.value("pdfMergeOrder") = pdfs.join("\n");
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
            "Edit podcast",
            "Chart from CSV",
            "Scenario workbook",
            "Extract table",
            "Brief + action plan",
            "Fill Word template",
            "Translate or rewrite",
            "Track source changes",
            "Check claims",
            "Synthesize sources",
            "Compare options",
            "Web research report",
            "Extract PDF text",
            "Merge PDFs",
            "Design presentation",
            "Apply presentation brand",
            "Narrate presentation",
            "Compose video",
            "Find video highlights",
            "Create motion video",
            "Clean and level audio",
            "Clean and analyze data",
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
        if ![10, 11, 12, 13, 14, 15, 16, 17, 18, 19].contains(&draft.kind) {
            hint(
                ui,
                "This adds a precise request to Chat. Send, review the plan and start explicitly.",
            );
        }
        if ![15, 16, 17, 18, 19].contains(&draft.kind)
            && studio::quiet_button(ui, t(ui, "Set up tools").as_ref()).clicked()
        {
            self.create.select_productive_view(2);
            self.create.studio.tab = 2;
            return;
        }

        let all = resources(snapshot, &self.create.project);
        if ![0, 3, 7, 11, 12, 15, 16, 17, 18, 19, 21, 22, 23, 24, 27].contains(&draft.kind) {
            let kind = draft.kind;
            form_combo(
                ui,
                egui::ComboBox::from_id_salt("production-source").selected_text(
                    all.iter()
                        .find(|r| {
                            text(r, "resourceId") == draft.record
                                && match kind {
                                    5 | 8 | 28 => podcast_source(r),
                                    9 | 10 | 29 => chart_source(r),
                                    13 => template_source(r),
                                    14 => rewrite_source(r),
                                    20 => pdf_source(r),
                                    25 | 26 => video_source(r),
                                    _ => true,
                                }
                        })
                        .map(|r| text(r, "name"))
                        .unwrap_or(t(ui, "Choose a resource").as_ref()),
                ),
                |ui| {
                    for r in all.iter().filter(|r| {
                        if ![5, 8, 9, 10, 13, 14, 20, 25, 26, 28, 29].contains(&kind) {
                            return true;
                        }
                        text(r, "status") == "available"
                            && match kind {
                                5 | 8 | 28 => podcast_source(r),
                                9 | 10 | 29 => chart_source(r),
                                13 => template_source(r),
                                14 => rewrite_source(r),
                                20 => pdf_source(r),
                                25 | 26 => video_source(r),
                                _ => false,
                            }
                    }) {
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
            5 => {
                hint(
                    ui,
                    "Transcribe one exact recording locally and keep timed SRT/VTT subtitles with source-hash verification.",
                );
                if draft.get("language").is_empty() {
                    *draft.value("language") = "auto".into();
                }
                field(
                    ui,
                    "Language · auto or ISO 639 code",
                    draft.value("language"),
                    12,
                    false,
                );
                hint(
                    ui,
                    "Use auto for detection or a two/three-letter lowercase code such as tr or en.",
                );
                hint(
                    ui,
                    "Delivers transcript.txt, transcript.json, captions.srt, captions.vtt and a verification manifest.",
                );
                let language = draft.get("language").trim();
                let valid_language = language == "auto"
                    || ((2..=3).contains(&language.len())
                        && language.bytes().all(|byte| byte.is_ascii_lowercase()));
                if let Some(source) =
                    source_id.filter(|_| source.is_some_and(podcast_source) && valid_language)
                {
                    operation = Some(json!({"operation":"audio.transcribe","arguments":{
                        "source":source,"language":language
                    }}));
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
            8 => {
                let audio_source = source.is_some_and(podcast_source);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Episode title"));
                    studio::input(ui, draft.value("title"), 160, "", false);
                });
                multiline(
                    ui,
                    "Segments · start | end | chapter title",
                    draft.value("segments"),
                    6000,
                    2,
                );
                hint(
                    ui,
                    "Segments are joined in this order. Chapter times are recalculated for the edited episode.",
                );
                let clips: Option<Vec<Value>> = draft
                    .get("segments")
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| {
                        let mut parts = line.splitn(3, '|');
                        let start = parts.next()?.trim().parse::<f64>().ok()?;
                        let end = parts.next()?.trim().parse::<f64>().ok()?;
                        let title = parts.next()?.trim();
                        (start.is_finite()
                            && end.is_finite()
                            && start >= 0.0
                            && end > start
                            && end - start <= 1200.0
                            && valid_input(title, 160))
                        .then(|| {
                            json!({
                                "source":source_id.as_deref().unwrap_or_default(),
                                "start":start,
                                "end":end,
                                "title":title
                            })
                        })
                    })
                    .collect();
                let total = clips.as_ref().map(|clips| {
                    clips
                        .iter()
                        .map(|clip| {
                            clip["end"].as_f64().unwrap_or_default()
                                - clip["start"].as_f64().unwrap_or_default()
                        })
                        .sum::<f64>()
                });
                if audio_source
                    && source_id.is_some()
                    && clips
                        .as_ref()
                        .is_some_and(|clips| (1..=12).contains(&clips.len()))
                    && total.is_some_and(|seconds| seconds <= 1200.0)
                    && valid_input(draft.get("title"), 160)
                {
                    operation = Some(json!({
                        "operation":"audio.podcast",
                        "arguments":{"title":draft.get("title"),"clips":clips}
                    }));
                }
            }
            9 => {
                let csv_source = source.is_some_and(chart_source);
                for (label, key) in [
                    ("Chart title", "title"),
                    ("Category column", "categoryColumn"),
                    ("Value column", "valueColumn"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(t(ui, label));
                        studio::input(ui, draft.value(key), 160, "", false);
                    });
                }
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Calculation"));
                    for (value, label) in [("sum", "Sum"), ("average", "Average")] {
                        ui.selectable_value(
                            draft.value("aggregation"),
                            value.to_owned(),
                            t(ui, label),
                        );
                    }
                });
                let aggregation = match draft.get("aggregation") {
                    "average" => "average",
                    _ => "sum",
                };
                if csv_source
                    && source_id.is_some()
                    && valid_input(draft.get("title"), 160)
                    && valid_input(draft.get("categoryColumn"), 160)
                    && valid_input(draft.get("valueColumn"), 160)
                    && draft.get("categoryColumn") != draft.get("valueColumn")
                {
                    operation = Some(json!({
                        "operation":"data.chart",
                        "arguments":{
                            "source":source_id,
                            "title":draft.get("title"),
                            "categoryColumn":draft.get("categoryColumn"),
                            "valueColumn":draft.get("valueColumn"),
                            "aggregation":aggregation
                        }
                    }));
                }
            }
            10 => {
                let csv_source = source.is_some_and(chart_source);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Model title"));
                    studio::input(ui, draft.value("title"), 160, "", false);
                });
                ui.label(t(ui, "Columns · item | base | upside | downside"));
                studio::input(ui, draft.value("scenarioColumns"), 640, "", false);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Currency"));
                    studio::input(ui, draft.value("currency"), 3, "", false);
                });
                hint(
                    ui,
                    "This only adds to Chat. The workbook keeps editable inputs and recalculates when its scenario selector changes.",
                );
                let currency = draft.get("currency").trim().to_ascii_uppercase();
                let columns = draft
                    .get("scenarioColumns")
                    .split('|')
                    .map(str::trim)
                    .collect::<Vec<_>>();
                if csv_source
                    && source_id.is_some()
                    && valid_input(draft.get("title"), 160)
                    && columns.len() == 4
                    && columns.iter().all(|column| valid_input(column, 160))
                    && columns.iter().enumerate().all(|(index, column)| {
                        !columns[..index].iter().any(|previous| previous == column)
                    })
                    && currency.len() == 3
                    && currency.bytes().all(|byte| byte.is_ascii_uppercase())
                {
                    operation = Some(json!({
                        "operation":"data.scenario",
                        "arguments":{
                            "source":source_id,
                            "title":draft.get("title"),
                            "itemColumn":columns[0],
                            "baseColumn":columns[1],
                            "upsideColumn":columns[2],
                            "downsideColumn":columns[3],
                            "currency":currency
                        }
                    }));
                }
            }
            11 => {
                hint(
                    ui,
                    "Select 1–8 sources. Every value needs a literal quote; uncertain and missing cells stay visible.",
                );
                let candidates = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && extraction_source(resource)
                    })
                    .collect::<Vec<_>>();
                ui.horizontal_wrapped(|ui| {
                    for resource in &candidates {
                        let id = text(resource, "resourceId");
                        let mut selected = draft.sources.contains(id);
                        let enabled = selected || draft.sources.len() < 8;
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Checkbox::new(&mut selected, text(resource, "name")),
                            )
                            .changed()
                        {
                            if selected {
                                draft.sources.insert(id.into());
                            } else {
                                draft.sources.remove(id);
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Extraction title"));
                    studio::input(ui, draft.value("title"), 160, "", false);
                });
                ui.label(t(ui, "Columns · key: label | key: label"));
                studio::input(ui, draft.value("extractionColumns"), 1200, "", false);
                hint(
                    ui,
                    "Chat fills candidate rows. The local tool rejects values without exact source evidence.",
                );
                let mut seen = HashSet::new();
                let columns: Option<Vec<Value>> = draft
                    .get("extractionColumns")
                    .split('|')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| {
                        let (key, label) = value.split_once(':')?;
                        let key = key.trim();
                        let label = label.trim();
                        (extraction_key(key)
                            && valid_input(label, 80)
                            && seen.insert(key.to_owned()))
                        .then(|| json!({"key":key,"label":label}))
                    })
                    .collect();
                let sources = candidates
                    .iter()
                    .filter(|resource| draft.sources.contains(text(resource, "resourceId")))
                    .take(8)
                    .map(|resource| {
                        json!({"source":resource["resourceId"],"label":resource["name"]})
                    })
                    .collect::<Vec<_>>();
                if (1..=8).contains(&sources.len())
                    && columns
                        .as_ref()
                        .is_some_and(|columns| (1..=12).contains(&columns.len()))
                    && valid_input(draft.get("title"), 160)
                {
                    operation = Some(json!({
                        "operation":"data.extract",
                        "arguments":{
                            "title":draft.get("title"),
                            "sources":sources,
                            "columns":columns,
                            "rows":[]
                        }
                    }));
                }
            }
            12 => {
                hint(
                    ui,
                    "Select 1–8 sources. Findings and actions keep literal evidence; unresolved questions stay explicit.",
                );
                let candidates = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && extraction_source(resource)
                    })
                    .collect::<Vec<_>>();
                ui.horizontal_wrapped(|ui| {
                    for resource in &candidates {
                        let id = text(resource, "resourceId");
                        let mut selected = draft.sources.contains(id);
                        let enabled = selected || draft.sources.len() < 8;
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Checkbox::new(&mut selected, text(resource, "name")),
                            )
                            .changed()
                        {
                            if selected {
                                draft.sources.insert(id.into());
                            } else {
                                draft.sources.remove(id);
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Brief title"));
                    studio::input(ui, draft.value("title"), 160, "", false);
                });
                hint(
                    ui,
                    "Chat drafts the content. The local tool rejects invented evidence and keeps missing owners or dates visible.",
                );
                let sources = candidates
                    .iter()
                    .filter(|resource| draft.sources.contains(text(resource, "resourceId")))
                    .take(8)
                    .map(|resource| {
                        json!({"source":resource["resourceId"],"label":resource["name"]})
                    })
                    .collect::<Vec<_>>();
                if (1..=8).contains(&sources.len()) && valid_input(draft.get("title"), 160) {
                    operation = Some(json!({
                        "operation":"documents.brief",
                        "arguments":{
                            "title":draft.get("title"),
                            "sources":sources,
                            "findings":[],
                            "questions":[],
                            "actions":[]
                        }
                    }));
                }
            }
            13 => {
                ui.label(t(ui, "Template fields · key | value (blank stays missing)"));
                multiline(ui, "", draft.value("templateFields"), 12000, 4);
                hint(
                    ui,
                    "Use each {{field_key}} from the DOCX exactly once. Blank values remain visibly missing.",
                );
                let mut seen = HashSet::new();
                let fields: Option<Vec<Value>> = draft
                    .get("templateFields")
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| {
                        let (key, value) = line.split_once('|')?;
                        let key = key.trim();
                        let value = value.trim();
                        (extraction_key(key)
                            && value.encode_utf16().count() <= 4000
                            && seen.insert(key.to_owned()))
                        .then(|| {
                            if value.is_empty() {
                                json!({"key":key,"status":"missing","value":null})
                            } else {
                                json!({"key":key,"status":"provided","value":value})
                            }
                        })
                    })
                    .collect();
                if source.is_some_and(template_source)
                    && source_id.is_some()
                    && fields
                        .as_ref()
                        .is_some_and(|fields| (1..=100).contains(&fields.len()))
                {
                    operation = Some(json!({
                        "operation":"documents.template",
                        "arguments":{"source":source_id,"fields":fields}
                    }));
                }
            }
            14 => {
                let mut mode = if draft.get("rewriteMode") == "rewrite" {
                    1
                } else {
                    0
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Change"));
                    ui.selectable_value(&mut mode, 0, t(ui, "Translate"));
                    ui.selectable_value(&mut mode, 1, t(ui, "Rewrite style"));
                });
                *draft.value("rewriteMode") =
                    if mode == 0 { "translate" } else { "rewrite" }.into();
                let mut target = draft.get("rewriteTarget").to_owned();
                let mut terms = draft.get("preserveTerms").to_owned();
                ui.columns(2, |columns| {
                    field(
                        &mut columns[0],
                        if mode == 0 {
                            "Target language"
                        } else {
                            "Target style"
                        },
                        &mut target,
                        240,
                        false,
                    );
                    field(
                        &mut columns[1],
                        "Protected terms · comma separated (optional)",
                        &mut terms,
                        3630,
                        false,
                    );
                });
                *draft.value("rewriteTarget") = target;
                *draft.value("preserveTerms") = terms;
                hint(
                    ui,
                    "Every non-empty source line or DOCX paragraph stays paired for review. Numbers, dates, links and email addresses are protected automatically.",
                );
                let mut seen = HashSet::new();
                let preserve: Option<Vec<&str>> = draft
                    .get("preserveTerms")
                    .split(',')
                    .filter(|part| !part.trim().is_empty())
                    .map(|part| {
                        let value = part.trim();
                        (value.encode_utf16().count() <= 120 && seen.insert(value)).then_some(value)
                    })
                    .collect();
                if source.is_some_and(rewrite_source)
                    && source_id.is_some()
                    && valid_input(draft.get("rewriteTarget"), 240)
                    && preserve.as_ref().is_some_and(|values| values.len() <= 30)
                {
                    operation = Some(json!({
                        "operation":"documents.rewrite",
                        "arguments":{
                            "source":source_id,
                            "mode":draft.get("rewriteMode"),
                            "target":draft.get("rewriteTarget"),
                            "preserve":preserve,
                            "segments":[]
                        }
                    }));
                }
            }
            15 => {
                let candidates = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && extraction_source(resource)
                    })
                    .collect::<Vec<_>>();
                let mut after_source = draft.get("afterSource").to_owned();
                let earlier_placeholder = t(ui, "Choose earlier snapshot").into_owned();
                let later_placeholder = t(ui, "Choose later snapshot").into_owned();
                ui.columns(2, |columns| {
                    form_combo(
                        &mut columns[0],
                        egui::ComboBox::from_id_salt("source-change-before").selected_text(
                            candidates
                                .iter()
                                .find(|resource| text(resource, "resourceId") == draft.record)
                                .map(|resource| text(resource, "name"))
                                .unwrap_or(earlier_placeholder.as_str()),
                        ),
                        |ui| {
                            for resource in &candidates {
                                ui.selectable_value(
                                    &mut draft.record,
                                    text(resource, "resourceId").into(),
                                    text(resource, "name"),
                                );
                            }
                        },
                    );
                    form_combo(
                        &mut columns[1],
                        egui::ComboBox::from_id_salt("source-change-after").selected_text(
                            candidates
                                .iter()
                                .find(|resource| text(resource, "resourceId") == after_source)
                                .map(|resource| text(resource, "name"))
                                .unwrap_or(later_placeholder.as_str()),
                        ),
                        |ui| {
                            for resource in &candidates {
                                ui.selectable_value(
                                    &mut after_source,
                                    text(resource, "resourceId").into(),
                                    text(resource, "name"),
                                );
                            }
                        },
                    );
                });
                *draft.value("afterSource") = after_source;
                let mut title = draft.get("title").to_owned();
                let mut before_date = draft.get("beforeDate").to_owned();
                let mut after_date = draft.get("afterDate").to_owned();
                field(ui, "Report title", &mut title, 160, false);
                ui.columns(2, |columns| {
                    field(
                        &mut columns[0],
                        "Earlier date · YYYY-MM-DD",
                        &mut before_date,
                        10,
                        false,
                    );
                    field(
                        &mut columns[1],
                        "Later date · YYYY-MM-DD",
                        &mut after_date,
                        10,
                        false,
                    );
                });
                *draft.value("title") = title;
                *draft.value("beforeDate") = before_date;
                *draft.value("afterDate") = after_date;
                hint(
                    ui,
                    "The local tool compares exact frozen text, dates and hashes. Whitespace-only differences are reported as no meaningful change.",
                );
                let before = candidates
                    .iter()
                    .find(|resource| text(resource, "resourceId") == draft.record);
                let after = candidates
                    .iter()
                    .find(|resource| text(resource, "resourceId") == draft.get("afterSource"));
                let dates = chrono::NaiveDate::parse_from_str(draft.get("beforeDate"), "%Y-%m-%d")
                    .ok()
                    .zip(
                        chrono::NaiveDate::parse_from_str(draft.get("afterDate"), "%Y-%m-%d").ok(),
                    );
                if let (Some(before), Some(after), Some((before_date, after_date))) =
                    (before, after, dates)
                {
                    if before["resourceId"] != after["resourceId"]
                        && after_date > before_date
                        && valid_input(draft.get("title"), 160)
                    {
                        operation = Some(json!({
                            "operation":"research.changes",
                            "arguments":{
                                "title":draft.get("title"),
                                "before":{
                                    "source":before["resourceId"],
                                    "label":before["name"],
                                    "capturedAt":draft.get("beforeDate")
                                },
                                "after":{
                                    "source":after["resourceId"],
                                    "label":after["name"],
                                    "capturedAt":draft.get("afterDate")
                                }
                            }
                        }));
                    }
                }
            }
            16 => {
                hint(
                    ui,
                    "Select 1–8 sources. Every status is derived from literal evidence; unresolved claims require an explicit reason.",
                );
                let candidates = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && extraction_source(resource)
                    })
                    .collect::<Vec<_>>();
                ui.horizontal_wrapped(|ui| {
                    for resource in &candidates {
                        let id = text(resource, "resourceId");
                        let mut selected = draft.sources.contains(id);
                        let enabled = selected || draft.sources.len() < 8;
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Checkbox::new(&mut selected, text(resource, "name")),
                            )
                            .changed()
                        {
                            if selected {
                                draft.sources.insert(id.into());
                            } else {
                                draft.sources.remove(id);
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Claim report title"));
                    studio::input(ui, draft.value("title"), 160, "", false);
                });
                ui.label(t(ui, "Claims · one per line"));
                multiline(ui, "", draft.value("claims"), 12000, 3);
                hint(
                    ui,
                    "Chat inspects the selected frozen sources. The local tool rejects invented quotes and never assigns an automatic truth score.",
                );
                let sources = candidates
                    .iter()
                    .filter(|resource| draft.sources.contains(text(resource, "resourceId")))
                    .take(8)
                    .map(|resource| {
                        json!({"source":resource["resourceId"],"label":resource["name"]})
                    })
                    .collect::<Vec<_>>();
                let mut seen = HashSet::new();
                let claims: Option<Vec<Value>> = draft
                    .get("claims")
                    .lines()
                    .map(str::trim)
                    .filter(|claim| !claim.is_empty())
                    .map(|claim| {
                        (valid_input(claim, 2000) && seen.insert(claim)).then(|| {
                            json!({
                                "claim":claim,
                                "checkedSources":[],
                                "evidence":[],
                                "unresolvedReason":null
                            })
                        })
                    })
                    .collect();
                if (1..=8).contains(&sources.len())
                    && valid_input(draft.get("title"), 160)
                    && claims
                        .as_ref()
                        .is_some_and(|claims| (1..=50).contains(&claims.len()))
                {
                    operation = Some(json!({
                        "operation":"research.claims",
                        "arguments":{
                            "title":draft.get("title"),
                            "sources":sources,
                            "claims":claims
                        }
                    }));
                }
            }
            17 => {
                hint(
                    ui,
                    "Select 2–8 sources. Every source must be cited, and at least one finding must combine evidence from multiple sources.",
                );
                let candidates = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && extraction_source(resource)
                    })
                    .collect::<Vec<_>>();
                ui.horizontal_wrapped(|ui| {
                    for resource in &candidates {
                        let id = text(resource, "resourceId");
                        let mut selected = draft.sources.contains(id);
                        let enabled = selected || draft.sources.len() < 8;
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Checkbox::new(&mut selected, text(resource, "name")),
                            )
                            .changed()
                        {
                            if selected {
                                draft.sources.insert(id.into());
                            } else {
                                draft.sources.remove(id);
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Synthesis report title"));
                    studio::input(ui, draft.value("title"), 160, "", false);
                });
                ui.label(t(ui, "Synthesis focus"));
                multiline(ui, "", draft.value("synthesisFocus"), 2000, 2);
                hint(
                    ui,
                    "Chat drafts attributed findings and source-distinct disagreements. The local tool rejects unused sources and invented quotes.",
                );
                let sources = candidates
                    .iter()
                    .filter(|resource| draft.sources.contains(text(resource, "resourceId")))
                    .take(8)
                    .map(|resource| {
                        json!({"source":resource["resourceId"],"label":resource["name"]})
                    })
                    .collect::<Vec<_>>();
                if (2..=8).contains(&sources.len())
                    && valid_input(draft.get("title"), 160)
                    && valid_input(draft.get("synthesisFocus"), 2000)
                {
                    operation = Some(json!({
                        "operation":"research.synthesis",
                        "arguments":{
                            "title":draft.get("title"),
                            "focus":draft.get("synthesisFocus"),
                            "sources":sources,
                            "findings":[],
                            "disagreements":[]
                        }
                    }));
                }
            }
            18 => {
                hint(
                    ui,
                    "Select 1–8 sources. The same owner criteria are applied to every option; recommendation and uncertainty stay separate.",
                );
                let candidates = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && extraction_source(resource)
                    })
                    .collect::<Vec<_>>();
                ui.horizontal_wrapped(|ui| {
                    for resource in &candidates {
                        let id = text(resource, "resourceId");
                        let mut selected = draft.sources.contains(id);
                        let enabled = selected || draft.sources.len() < 8;
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Checkbox::new(&mut selected, text(resource, "name")),
                            )
                            .changed()
                        {
                            if selected {
                                draft.sources.insert(id.into());
                            } else {
                                draft.sources.remove(id);
                            }
                        }
                    }
                });
                ui.columns(2, |columns| {
                    field(
                        &mut columns[0],
                        "Comparison report title",
                        draft.value("title"),
                        160,
                        false,
                    );
                    field(
                        &mut columns[1],
                        "Decision question",
                        draft.value("comparisonQuestion"),
                        2000,
                        false,
                    );
                });
                ui.columns(2, |columns| {
                    field(
                        &mut columns[0],
                        "Criteria · key: label | key: label",
                        draft.value("comparisonCriteria"),
                        2400,
                        false,
                    );
                    field(
                        &mut columns[1],
                        "Options · separated by |",
                        draft.value("comparisonOptions"),
                        2880,
                        false,
                    );
                });
                hint(
                    ui,
                    "Chat fills every option × criterion cell with literal evidence or an explicit missing status. The local tool rejects uneven tables and invented quotes.",
                );
                let mut seen_keys = HashSet::new();
                let mut seen_labels = HashSet::new();
                let criteria: Option<Vec<(&str, &str)>> = draft
                    .get("comparisonCriteria")
                    .split('|')
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(|line| {
                        let (key, label) = line.split_once(':')?;
                        let key = key.trim();
                        let label = label.trim();
                        (extraction_key(key)
                            && valid_input(label, 160)
                            && seen_keys.insert(key)
                            && seen_labels.insert(label))
                        .then_some((key, label))
                    })
                    .collect();
                let mut seen_options = HashSet::new();
                let options: Option<Vec<&str>> = draft
                    .get("comparisonOptions")
                    .split('|')
                    .map(str::trim)
                    .filter(|option| !option.is_empty())
                    .map(|option| {
                        (valid_input(option, 240) && seen_options.insert(option)).then_some(option)
                    })
                    .collect();
                let sources = candidates
                    .iter()
                    .filter(|resource| draft.sources.contains(text(resource, "resourceId")))
                    .take(8)
                    .map(|resource| {
                        json!({"source":resource["resourceId"],"label":resource["name"]})
                    })
                    .collect::<Vec<_>>();
                if (1..=8).contains(&sources.len())
                    && valid_input(draft.get("title"), 160)
                    && valid_input(draft.get("comparisonQuestion"), 2000)
                    && criteria
                        .as_ref()
                        .is_some_and(|criteria| (1..=12).contains(&criteria.len()))
                    && options
                        .as_ref()
                        .is_some_and(|options| (2..=12).contains(&options.len()))
                {
                    let criteria = criteria.unwrap();
                    let criteria_json = criteria
                        .iter()
                        .map(|(key, label)| json!({"key":key,"label":label}))
                        .collect::<Vec<_>>();
                    let options_json = options
                        .unwrap()
                        .iter()
                        .map(|option| {
                            let cells = criteria
                                .iter()
                                .map(|(key, _)| {
                                    (
                                        (*key).to_owned(),
                                        json!({"status":"missing","assessment":null,"evidence":[]}),
                                    )
                                })
                                .collect::<serde_json::Map<_, _>>();
                            json!({"option":option,"cells":cells})
                        })
                        .collect::<Vec<_>>();
                    operation = Some(json!({
                        "operation":"research.compare",
                        "arguments":{
                            "title":draft.get("title"),
                            "question":draft.get("comparisonQuestion"),
                            "sources":sources,
                            "criteria":criteria_json,
                            "options":options_json,
                            "recommendation":{
                                "option":null,
                                "reason":"",
                                "basedOnCriteria":[],
                                "uncertainty":""
                            }
                        }
                    }));
                }
            }
            19 => {
                hint(
                    ui,
                    "Retrieve 1–4 public pages with scripts disabled. Every material claim keeps an access time, response hash and literal quote.",
                );
                ui.columns(2, |columns| {
                    field(
                        &mut columns[0],
                        "Research report title",
                        draft.value("title"),
                        160,
                        false,
                    );
                    field(
                        &mut columns[1],
                        "Research question",
                        draft.value("webResearchQuestion"),
                        2000,
                        false,
                    );
                });
                ui.columns(2, |columns| {
                    multiline(
                        &mut columns[0],
                        "Public sources · label | URL",
                        draft.value("webResearchSources"),
                        9200,
                        2,
                    );
                    multiline(
                        &mut columns[1],
                        "Claims to verify · one per line",
                        draft.value("webResearchClaims"),
                        12000,
                        2,
                    );
                });
                hint(
                    ui,
                    "Chat inspects each page first, then fills only exact quotes. Unknowns stay explicit; nothing is submitted or published.",
                );
                let mut seen_urls = HashSet::new();
                let mut seen_labels = HashSet::new();
                let sources: Option<Vec<Value>> = draft
                    .get("webResearchSources")
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(|line| {
                        let (label, url) = line.split_once('|')?;
                        let label = label.trim();
                        let url = url.trim();
                        (valid_input(label, 200)
                            && (url.starts_with("https://") || url.starts_with("http://"))
                            && valid_input(url, 4096)
                            && seen_labels.insert(label)
                            && seen_urls.insert(url))
                        .then(|| json!({"url":url,"label":label}))
                    })
                    .collect();
                let mut seen_claims = HashSet::new();
                let claims: Option<Vec<Value>> = draft
                    .get("webResearchClaims")
                    .lines()
                    .map(str::trim)
                    .filter(|claim| !claim.is_empty())
                    .map(|claim| {
                        (valid_input(claim, 2000) && seen_claims.insert(claim)).then(|| {
                            json!({
                                "claim":claim,
                                "classification":"fact",
                                "evidence":[],
                                "note":null
                            })
                        })
                    })
                    .collect();
                if valid_input(draft.get("title"), 160)
                    && valid_input(draft.get("webResearchQuestion"), 2000)
                    && sources
                        .as_ref()
                        .is_some_and(|sources| (1..=4).contains(&sources.len()))
                    && claims
                        .as_ref()
                        .is_some_and(|claims| (1..=50).contains(&claims.len()))
                {
                    operation = Some(json!({
                        "operation":"browser.research",
                        "arguments":{
                            "title":draft.get("title"),
                            "question":draft.get("webResearchQuestion"),
                            "sources":sources,
                            "claims":claims
                        }
                    }));
                }
            }
            20 => {
                hint(
                    ui,
                    "Extract a PDF's text layer, force OCR, or use OCR only on pages without enough embedded text.",
                );
                let mut mode = match draft.get("pdfExtractMode") {
                    "text" => 1,
                    "ocr" => 2,
                    _ => 0,
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Extraction mode"));
                    ui.selectable_value(&mut mode, 0, t(ui, "Automatic"));
                    ui.selectable_value(&mut mode, 1, t(ui, "Text layer"));
                    ui.selectable_value(&mut mode, 2, "OCR");
                });
                let mode = match mode {
                    1 => "text",
                    2 => "ocr",
                    _ => "auto",
                };
                *draft.value("pdfExtractMode") = mode.into();
                if mode != "text" {
                    field(ui, "OCR language", draft.value("language"), 40, false);
                }
                hint(
                    ui,
                    if mode == "text" {
                        "Text mode supports up to 100 pages and never invents missing text."
                    } else {
                        "OCR and automatic mode are bounded to 12 pages; every page records the method used."
                    },
                );
                if source.is_some_and(pdf_source) {
                    if let Some(source) = source_id {
                        let mut args = json!({"source":source,"mode":mode});
                        if mode != "text" && !draft.get("language").is_empty() {
                            args["language"] = json!(draft.get("language"));
                        }
                        operation = Some(json!({"operation":"pdf.extract","arguments":args}));
                    }
                }
            }
            21 => {
                hint(
                    ui,
                    "Select 2–12 PDFs, then set their exact order. The merged file is reopened, counted and rendered before delivery.",
                );
                let candidates = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && pdf_source(resource)
                    })
                    .collect::<Vec<_>>();
                let candidate_ids = candidates
                    .iter()
                    .map(|resource| text(resource, "resourceId").to_owned())
                    .collect::<HashSet<_>>();
                let mut order = Vec::new();
                let mut seen = HashSet::new();
                for id in draft.get("pdfMergeOrder").lines().map(str::trim) {
                    if candidate_ids.contains(id)
                        && draft.sources.contains(id)
                        && seen.insert(id.to_owned())
                    {
                        order.push(id.to_owned());
                    }
                }
                for resource in &candidates {
                    let id = text(resource, "resourceId");
                    if draft.sources.contains(id) && seen.insert(id.to_owned()) {
                        order.push(id.to_owned());
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    for resource in &candidates {
                        let id = text(resource, "resourceId");
                        let mut selected = draft.sources.contains(id);
                        if ui
                            .add_enabled(
                                selected || draft.sources.len() < 12,
                                egui::Checkbox::new(&mut selected, text(resource, "name")),
                            )
                            .changed()
                        {
                            if selected {
                                draft.sources.insert(id.into());
                                if !order.iter().any(|value| value == id) {
                                    order.push(id.into());
                                }
                            } else {
                                draft.sources.remove(id);
                                order.retain(|value| value != id);
                            }
                        }
                    }
                });
                ui.label(t(ui, "Merge order"));
                let mut movement = None;
                for (index, id) in order.iter().enumerate() {
                    if let Some(resource) = candidates
                        .iter()
                        .find(|resource| text(resource, "resourceId") == id)
                    {
                        ui.push_id(("pdf-merge-order", id), |ui| {
                            ui.horizontal(|ui| {
                                ui.monospace(format!("{:02}", index + 1));
                                ui.label(text(resource, "name"));
                                let move_up = t(ui, "Move up").into_owned();
                                if ui
                                    .add_enabled(index > 0, egui::Button::new(move_up))
                                    .clicked()
                                {
                                    movement = Some((index, index - 1));
                                }
                                let move_down = t(ui, "Move down").into_owned();
                                if ui
                                    .add_enabled(
                                        index + 1 < order.len(),
                                        egui::Button::new(move_down),
                                    )
                                    .clicked()
                                {
                                    movement = Some((index, index + 1));
                                }
                            });
                        });
                    }
                }
                if let Some((from, to)) = movement {
                    order.swap(from, to);
                }
                *draft.value("pdfMergeOrder") = order.join("\n");
                let sources = order
                    .iter()
                    .map(|source| json!({"source":source}))
                    .collect::<Vec<_>>();
                if (2..=12).contains(&sources.len()) {
                    operation =
                        Some(json!({"operation":"pdf.merge","arguments":{"sources":sources}}));
                }
            }
            22 => {
                hint(
                    ui,
                    "Build a theme-aware editable deck. Each non-empty line is one slide; renders require LibreOffice and Poppler.",
                );
                let mut brand = presentation_brand_fields(ui, draft);
                let logos = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available"
                            && presentation_logo_source(resource)
                    })
                    .collect::<Vec<_>>();
                form_combo(
                    ui,
                    egui::ComboBox::from_id_salt("presentation-logo").selected_text(
                        logos
                            .iter()
                            .find(|resource| text(resource, "resourceId") == draft.record)
                            .map(|resource| text(resource, "name"))
                            .unwrap_or(t(ui, "No logo").as_ref()),
                    ),
                    |ui| {
                        ui.selectable_value(&mut draft.record, String::new(), t(ui, "No logo"));
                        for resource in &logos {
                            ui.selectable_value(
                                &mut draft.record,
                                text(resource, "resourceId").into(),
                                text(resource, "name"),
                            );
                        }
                    },
                );
                multiline(
                    ui,
                    "Slides · layout | title | content | unit",
                    draft.value("presentationSlides"),
                    12_000,
                    6,
                );
                hint(
                    ui,
                    "Layouts: cover or statement use supporting text; content uses semicolon-separated bullets; bar-chart uses Label=Value pairs.",
                );
                field(
                    ui,
                    "Chart source label (optional)",
                    draft.value("chartSource"),
                    240,
                    false,
                );
                let mut render = draft.get("renderSlides") != "false";
                if ui
                    .checkbox(&mut render, t(ui, "Render every slide for review"))
                    .changed()
                {
                    *draft.value("renderSlides") = render.to_string();
                }
                let slides =
                    presentation_slides(draft.get("presentationSlides"), draft.get("chartSource"));
                if let Some(resource) = logos
                    .iter()
                    .find(|resource| text(resource, "resourceId") == draft.record)
                {
                    if let Some(value) = brand.as_mut() {
                        value["logo"] = json!({
                            "source":format!("resource:{}",text(resource,"resourceId").trim_start_matches("resource:")),
                            "alt":text(resource,"name")
                        });
                    }
                }
                if let (Some(brand), Some(slides)) = (brand, slides) {
                    operation = Some(json!({
                        "operation":"presentations.create",
                        "arguments":{"brand":brand,"slides":slides,"render":render}
                    }));
                }
            }
            23 => {
                hint(
                    ui,
                    "Apply a native Office theme while preserving every slide, note, image and relationship part.",
                );
                let decks = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && presentation_source(resource)
                    })
                    .collect::<Vec<_>>();
                form_combo(
                    ui,
                    egui::ComboBox::from_id_salt("presentation-brand-source").selected_text(
                        decks
                            .iter()
                            .find(|resource| text(resource, "resourceId") == draft.record)
                            .map(|resource| text(resource, "name"))
                            .unwrap_or(t(ui, "Choose a presentation").as_ref()),
                    ),
                    |ui| {
                        for resource in &decks {
                            ui.selectable_value(
                                &mut draft.record,
                                text(resource, "resourceId").into(),
                                text(resource, "name"),
                            );
                        }
                    },
                );
                let brand = presentation_brand_fields(ui, draft);
                let mut render = draft.get("renderSlides") != "false";
                if ui
                    .checkbox(&mut render, t(ui, "Render every slide for review"))
                    .changed()
                {
                    *draft.value("renderSlides") = render.to_string();
                }
                if let (Some(source), Some(brand)) = (
                    decks
                        .iter()
                        .find(|resource| text(resource, "resourceId") == draft.record),
                    brand,
                ) {
                    operation = Some(json!({
                        "operation":"presentations.brand",
                        "arguments":{
                            "source":format!("resource:{}",text(source,"resourceId").trim_start_matches("resource:")),
                            "brand":brand,"render":render
                        }
                    }));
                }
            }
            24 => {
                hint(
                    ui,
                    "Turn exact speaker notes and one approved audio clip per slide into a checked presentation video.",
                );
                let decks = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && presentation_source(resource)
                    })
                    .collect::<Vec<_>>();
                form_combo(
                    ui,
                    egui::ComboBox::from_id_salt("presentation-narration-source").selected_text(
                        decks
                            .iter()
                            .find(|resource| text(resource, "resourceId") == draft.record)
                            .map(|resource| text(resource, "name"))
                            .unwrap_or(t(ui, "Choose a presentation").as_ref()),
                    ),
                    |ui| {
                        for resource in &decks {
                            ui.selectable_value(
                                &mut draft.record,
                                text(resource, "resourceId").into(),
                                text(resource, "name"),
                            );
                        }
                    },
                );
                ui.label(t(ui, "Narration clips · filename order"));
                let mut audio = all
                    .iter()
                    .filter(|resource| narration_source(resource))
                    .cloned()
                    .collect::<Vec<_>>();
                audio.sort_by_key(|resource| text(resource, "name").to_lowercase());
                sources(ui, draft, &audio);
                let mut selected = audio
                    .iter()
                    .filter(|resource| draft.sources.contains(text(resource, "resourceId")))
                    .collect::<Vec<_>>();
                selected.sort_by_key(|resource| text(resource, "name").to_lowercase());
                for (index, resource) in selected.iter().enumerate() {
                    raw_hint(
                        ui,
                        &format!("{:02} · {}", index + 1, text(resource, "name")),
                    );
                }
                hint(
                    ui,
                    "Use numbered filenames. Every slide must already contain speaker notes; spoken-word matching remains a human review.",
                );
                let mut fps = draft.get("presentationFps").parse::<u64>().unwrap_or(30);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Frames per second"));
                    for value in [24, 25, 30] {
                        ui.selectable_value(&mut fps, value, value.to_string());
                    }
                });
                *draft.value("presentationFps") = fps.to_string();
                if let Some(source) = decks
                    .iter()
                    .find(|resource| text(resource, "resourceId") == draft.record)
                    .filter(|_| (1..=8).contains(&selected.len()))
                {
                    let segments = selected
                        .iter()
                        .enumerate()
                        .map(|(index, resource)| {
                            json!({"slide":index+1,"source":format!("resource:{}",text(resource,"resourceId").trim_start_matches("resource:"))})
                        })
                        .collect::<Vec<_>>();
                    operation = Some(json!({
                        "operation":"presentations.narrate",
                        "arguments":{
                            "source":format!("resource:{}",text(source,"resourceId").trim_start_matches("resource:")),
                            "segments":segments,"fps":fps
                        }
                    }));
                }
            }
            25 => {
                hint(
                    ui,
                    "Combine an exact title, image and approved audio in a checked 1280×720 video.",
                );
                let images = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available"
                            && composition_image_source(resource)
                    })
                    .collect::<Vec<_>>();
                let audio = all
                    .iter()
                    .filter(|resource| {
                        text(resource, "status") == "available" && narration_source(resource)
                    })
                    .collect::<Vec<_>>();
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Overlay image"));
                    form_combo(
                        ui,
                        egui::ComboBox::from_id_salt("video-composition-image").selected_text(
                            images
                                .iter()
                                .find(|resource| {
                                    text(resource, "resourceId") == draft.get("compositionImage")
                                })
                                .map(|resource| text(resource, "name"))
                                .unwrap_or(t(ui, "Choose an image").as_ref()),
                        ),
                        |ui| {
                            for resource in &images {
                                ui.selectable_value(
                                    draft.value("compositionImage"),
                                    text(resource, "resourceId").into(),
                                    text(resource, "name"),
                                );
                            }
                        },
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Approved audio"));
                    form_combo(
                        ui,
                        egui::ComboBox::from_id_salt("video-composition-audio").selected_text(
                            audio
                                .iter()
                                .find(|resource| {
                                    text(resource, "resourceId") == draft.get("compositionAudio")
                                })
                                .map(|resource| text(resource, "name"))
                                .unwrap_or(t(ui, "Choose audio").as_ref()),
                        ),
                        |ui| {
                            for resource in &audio {
                                ui.selectable_value(
                                    draft.value("compositionAudio"),
                                    text(resource, "resourceId").into(),
                                    text(resource, "name"),
                                );
                            }
                        },
                    );
                });
                field(
                    ui,
                    "On-video title",
                    draft.value("compositionTitle"),
                    120,
                    false,
                );
                let mut title_seconds = draft
                    .get("compositionTitleSeconds")
                    .parse::<f64>()
                    .unwrap_or(2.5);
                let mut start = draft
                    .get("compositionOverlayStart")
                    .parse::<f64>()
                    .unwrap_or(3.0);
                let mut end = draft
                    .get("compositionOverlayEnd")
                    .parse::<f64>()
                    .unwrap_or(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Title duration (seconds)"));
                    ui.add(egui::DragValue::new(&mut title_seconds).range(0.5..=10.0));
                    ui.label(t(ui, "Image start"));
                    ui.add(egui::DragValue::new(&mut start).range(0.0..=1200.0));
                    ui.label(t(ui, "Image end"));
                    ui.add(egui::DragValue::new(&mut end).range(0.0..=1200.0));
                });
                *draft.value("compositionTitleSeconds") = title_seconds.to_string();
                *draft.value("compositionOverlayStart") = start.to_string();
                *draft.value("compositionOverlayEnd") = end.to_string();
                let mut position = match draft.get("compositionPosition") {
                    "top-left" => 0,
                    "top-right" => 1,
                    "bottom-left" => 2,
                    _ => 3,
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Image position"));
                    for (index, label) in ["Top left", "Top right", "Bottom left", "Bottom right"]
                        .iter()
                        .enumerate()
                    {
                        ui.selectable_value(&mut position, index, t(ui, label));
                    }
                });
                let position = ["top-left", "top-right", "bottom-left", "bottom-right"][position];
                *draft.value("compositionPosition") = position.into();
                let mut mode = if draft.get("compositionAudioMode") == "mix" {
                    "mix"
                } else {
                    "replace"
                };
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Audio behavior"));
                    ui.selectable_value(&mut mode, "replace", t(ui, "Replace source audio"));
                    ui.selectable_value(&mut mode, "mix", t(ui, "Mix with source audio"));
                });
                *draft.value("compositionAudioMode") = mode.into();
                let mut level = draft
                    .get("compositionAudioLevel")
                    .parse::<f64>()
                    .unwrap_or(0.8);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Added audio level"));
                    ui.add(
                        egui::DragValue::new(&mut level)
                            .range(0.05..=1.0)
                            .speed(0.05)
                            .fixed_decimals(2),
                    );
                });
                *draft.value("compositionAudioLevel") = level.to_string();
                hint(
                    ui,
                    "The full video is decoded; four review frames are retained.",
                );
                let selected_image = images
                    .iter()
                    .find(|resource| text(resource, "resourceId") == draft.get("compositionImage"));
                let selected_audio = audio
                    .iter()
                    .find(|resource| text(resource, "resourceId") == draft.get("compositionAudio"));
                if let (Some(source), Some(image), Some(audio)) = (
                    source.filter(|resource| video_source(resource)),
                    selected_image,
                    selected_audio,
                ) {
                    if valid_input(draft.get("compositionTitle"), 120)
                        && title_seconds.is_finite()
                        && (0.5..=10.0).contains(&title_seconds)
                        && start.is_finite()
                        && end.is_finite()
                        && start >= 0.0
                        && end > start
                        && end <= 1200.0
                        && level.is_finite()
                    {
                        operation = Some(json!({
                            "operation":"video.compose",
                            "arguments":{
                                "source":format!("resource:{}",text(source,"resourceId").trim_start_matches("resource:")),
                                "image":{"source":format!("resource:{}",text(image,"resourceId").trim_start_matches("resource:"))},
                                "audio":{"source":format!("resource:{}",text(audio,"resourceId").trim_start_matches("resource:"))},
                                "title":draft.get("compositionTitle"),"titleSeconds":title_seconds,
                                "overlay":{"start":start,"end":end,"position":position},
                                "audioMode":mode,"audioLevel":level
                            }
                        }));
                    }
                }
            }
            26 => {
                hint(
                    ui,
                    "Rank real source ranges by measured motion and audio energy; meaning and taste still need human review.",
                );
                let mut target = draft
                    .get("highlightTargetSeconds")
                    .parse::<f64>()
                    .unwrap_or(8.0);
                let mut clip = draft
                    .get("highlightClipSeconds")
                    .parse::<f64>()
                    .unwrap_or(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Highlight duration (seconds)"));
                    ui.add(
                        egui::DragValue::new(&mut target)
                            .range(2.0..=180.0)
                            .speed(1.0),
                    );
                    ui.label(t(ui, "Fixed clip length (seconds)"));
                    ui.add(egui::DragValue::new(&mut clip).range(2.0..=30.0).speed(1.0));
                });
                *draft.value("highlightTargetSeconds") = target.to_string();
                *draft.value("highlightClipSeconds") = clip.to_string();
                let mut emphasis = match draft.get("highlightEmphasis") {
                    "visual" => "visual",
                    "audio" => "audio",
                    _ => "balanced",
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Selection signal"));
                    for value in ["balanced", "visual", "audio"] {
                        ui.selectable_value(&mut emphasis, value, t(ui, value));
                    }
                });
                *draft.value("highlightEmphasis") = emphasis.into();
                let count = if clip > 0.0 { target / clip } else { 0.0 };
                if let Some(source) = source.filter(|resource| video_source(resource)) {
                    if target.is_finite()
                        && clip.is_finite()
                        && (2.0..=180.0).contains(&target)
                        && (2.0..=30.0).contains(&clip)
                        && target >= clip
                        && count <= 12.0
                        && (count - count.round()).abs() < 0.001
                    {
                        operation = Some(json!({
                            "operation":"video.highlights",
                            "arguments":{
                                "source":format!("resource:{}",text(source,"resourceId").trim_start_matches("resource:")),
                                "targetSeconds":target,"clipSeconds":clip,"emphasis":emphasis
                            }
                        }));
                    }
                }
            }
            27 => {
                hint(
                    ui,
                    "Turn an approved scene plan into an editable storyboard and real pan/zoom motion video.",
                );
                field(
                    ui,
                    "Motion video title",
                    draft.value("motionTitle"),
                    120,
                    false,
                );
                multiline(
                    ui,
                    "Scenes · layout | seconds | eyebrow | title | subtitle",
                    draft.value("motionScenes"),
                    6000,
                    6,
                );
                let mut treatment = match draft.get("motionTreatment") {
                    "calm" => "calm",
                    "push" => "push",
                    _ => "drift",
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui, "Motion treatment"));
                    for value in ["calm", "push", "drift"] {
                        ui.selectable_value(&mut treatment, value, t(ui, value));
                    }
                });
                *draft.value("motionTreatment") = treatment.into();
                let mut fps = draft.get("motionFps").parse::<u64>().unwrap_or(30);
                ui.horizontal(|ui| {
                    ui.label(t(ui, "Frames per second"));
                    for value in [24, 25, 30] {
                        ui.selectable_value(&mut fps, value, value.to_string());
                    }
                });
                *draft.value("motionFps") = fps.to_string();
                let scenes: Option<Vec<Value>> = draft
                    .get("motionScenes")
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| {
                        let mut parts = line.splitn(5, '|').map(str::trim);
                        let layout = parts.next()?;
                        let duration = parts.next()?.parse::<f64>().ok()?;
                        let eyebrow = parts.next()?;
                        let title = parts.next()?;
                        let subtitle = parts.next()?;
                        (matches!(layout, "cover" | "statement")
                            && duration.is_finite()
                            && (2.0..=12.0).contains(&duration)
                            && valid_input(eyebrow, 80)
                            && valid_input(title, 160)
                            && valid_input(subtitle, 240))
                        .then(|| {
                            json!({"layout":layout,"duration":duration,
                                "eyebrow":eyebrow,"title":title,"subtitle":subtitle})
                        })
                    })
                    .collect();
                if let Some(scenes) = scenes.filter(|scenes| {
                    (2..=8).contains(&scenes.len())
                        && scenes
                            .iter()
                            .filter_map(|scene| scene["duration"].as_f64())
                            .sum::<f64>()
                            <= 90.0
                }) {
                    if valid_input(draft.get("motionTitle"), 120) {
                        operation = Some(json!({"operation":"video.motion","arguments":{
                            "title":draft.get("motionTitle"),"scenes":scenes,
                            "motion":treatment,"fps":fps
                        }}));
                    }
                }
            }
            28 => {
                hint(
                    ui,
                    "Clean and level one exact recording up to two minutes while preserving its duration, channels and original resource.",
                );
                let mut treatment = match draft.get("cleanupTreatment") {
                    "level-only" => "level-only",
                    "gentle" => "gentle",
                    _ => "voice",
                };
                ui.label(t(ui, "Cleanup treatment"));
                ui.horizontal(|ui| {
                    for value in ["level-only", "gentle", "voice"] {
                        ui.selectable_value(&mut treatment, value, t(ui, value));
                    }
                });
                *draft.value("cleanupTreatment") = treatment.into();
                let mut target = draft.get("cleanupTargetLufs").parse::<i64>().unwrap_or(-16);
                ui.label(t(ui, "Target loudness · LUFS"));
                ui.horizontal(|ui| {
                    for value in [-24, -20, -16, -14] {
                        ui.selectable_value(&mut target, value, value.to_string());
                    }
                });
                *draft.value("cleanupTargetLufs") = target.to_string();
                if let Some(source) = source_id.filter(|_| source.is_some_and(podcast_source)) {
                    operation = Some(json!({"operation":"audio.cleanup","arguments":{
                        "source":source,"targetLufs":target,"treatment":treatment
                    }}));
                }
            }
            29 => {
                hint(
                    ui,
                    "Trim exact CSV cells, optionally remove exact duplicate rows and verify selected numeric columns.",
                );
                let mut deduplicate = draft.get("dataDeduplicate") == "true";
                ui.checkbox(&mut deduplicate, t(ui, "Remove exact duplicate rows"));
                *draft.value("dataDeduplicate") = deduplicate.to_string();
                field(
                    ui,
                    "Numeric columns · comma-separated (optional)",
                    draft.value("dataNumericColumns"),
                    2000,
                    false,
                );
                let mut output = match draft.get("dataOutput") {
                    "csv" => "csv",
                    _ => "xlsx",
                };
                ui.label(t(ui, "Cleaned output"));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut output, "csv", t(ui, "CSV + profile"));
                    ui.selectable_value(&mut output, "xlsx", t(ui, "Editable XLSX + profile"));
                });
                *draft.value("dataOutput") = output.into();
                let raw = draft.get("dataNumericColumns").trim();
                let columns = if raw.is_empty() {
                    Some(Vec::new())
                } else {
                    let values = raw.split(',').map(str::trim).collect::<Vec<_>>();
                    ((1..=20).contains(&values.len())
                        && values.iter().all(|value| valid_input(value, 160))
                        && values.iter().collect::<HashSet<_>>().len() == values.len())
                    .then(|| values.into_iter().map(str::to_owned).collect::<Vec<_>>())
                };
                if let (Some(source), Some(columns)) = (
                    source_id.filter(|_| source.is_some_and(chart_source)),
                    columns,
                ) {
                    operation = Some(json!({
                        "operation":if output=="xlsx"{"data.xlsx"}else{"data.analyze"},
                        "arguments":{"source":source,"deduplicate":deduplicate,
                            "numericColumns":columns}
                    }));
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
                    match draft.kind {
                        5 => "Use audio.transcribe with this exact Project recording and language policy. Preserve the source hash, deliver transcript text/JSON plus SRT/VTT and the verification manifest, and report normalized-audio decode and timeline checks. Transcript content accuracy, speaker identity and uncertain passages still require human review.",
                        11 => "Read the exact Project sources and populate rows before calling the typed tool. Every confirmed or uncertain value must appear inside its literal evidence quote. Mark absent fields missing with null value and evidence; never infer or hide uncertainty.",
                        12 => "Read the exact Project sources and populate findings, questions and actions before calling the typed tool. Every finding and action needs literal evidence from its exact source. Keep unresolved questions explicit; use null owner or due when the notes do not supply them, and never guess.",
                        13 => "Inspect the exact DOCX template before calling the typed tool. Supply every {{field_key}} exactly once. Keep blank values missing with null instead of guessing, and preserve all non-placeholder template content.",
                        14 => "Read the exact frozen source before calling the typed tool. Populate one segment for every non-empty text line or DOCX body paragraph in exact order. Keep each source string literal, preserve facts and requested terms, and never claim semantic equivalence without owner review.",
                        15 => "Compare only these two exact frozen snapshots with the typed local tool. Keep the supplied capture dates and source hashes in the report; do not substitute a live page, infer a capture date or hide an explicit no-change result.",
                        16 => "Read every selected frozen source before calling the typed local tool. For each claim, list every source actually checked as {source}, add only literal quotes with source and supports, disputes or context stance, and give an explicit unresolvedReason when no support or dispute exists. Do not invent evidence, copy a requested status or present the derived status as an automatic truth score.",
                        17 => "Read every selected frozen source before calling the typed local tool. Cite every selected source, populate at least one finding with literal evidence from two or more distinct sources, and preserve each disagreement as two or more source-distinct positions with literal quotes. Keep disagreements unresolved; use an empty list only when none was found, and never turn absence of a supplied disagreement into a consensus claim.",
                        18 => "Read every selected frozen source before calling the typed local tool. Apply every owner criterion to every option using the exact criterion keys. For each cell, use confirmed or uncertain with an assessment and literal source evidence, or missing with null assessment and no evidence. Cite every selected source. Keep recommendation.option, reason and basedOnCriteria separate from uncertainty; use a null option and empty basedOnCriteria when the evidence cannot support a recommendation. Never invent quotes, silently score criteria or present the recommendation as automatic truth.",
                        19 => "Inspect each requested public URL with browser.inspect before calling browser.research. For every claim, use fact with literal evidence and null note, inference with literal evidence and an explicit review note, or unknown with no evidence and a reason. Cite every requested URL by its exact source value. The typed browser will retrieve the pages again, reject changed or invented quotes, record UTC access times and response/text hashes, and emit research-report.md, sources.csv and research-evidence.json. Do not use private pages, submit forms, invent citations or present classifications as automatic truth.",
                        20 => "Use pdf.extract on this exact Project PDF. Keep the selected text/OCR mode and language. Report page limits or missing Poppler/Tesseract honestly; do not infer text that the tool did not extract.",
                        21 => "Use pdf.merge with the exact source order shown. Do not reorder, omit or duplicate a source. Deliver merged.pdf only after the tool reports the expected page count, verified order, source hashes and first/last render previews.",
                        22 => "Use presentations.create with these exact slide layouts, chart values, theme fields and optional logo. Keep the slide text editable, preserve the supplied data literally and deliver the PPTX only after every requested slide render succeeds.",
                        23 => "Use presentations.brand on this exact PPTX and theme. Preserve every slide, note, image and relationship part; report explicit colours that cannot follow the theme and do not claim full visual adaptation until the renders are reviewed.",
                        24 => "Use presentations.narrate on this exact PPTX and filename-ordered audio set. Require one speaker-note part and one approved clip per slide, preserve the deck, verify the rendered video and state that spoken-word matching still needs human review.",
                        25 => "Use video.compose with these exact video, image and audio resources. Preserve every source, keep the visible title/timing/position/audio policy unchanged, decode the full output and require human review of the retained placement frames before delivery.",
                        26 => "Use video.highlights with this exact video and bounded selection policy. Preserve every measured score and selected range, decode the full output and require human review because motion and loudness do not prove meaning or aesthetic quality.",
                        27 => "Use video.motion with this exact approved scene plan. Preserve every title and duration, deliver the editable storyboard and rendered MP4, verify intra-scene movement and full decode, and keep visual-quality review explicit.",
                        28 => "Use audio.cleanup with this exact source, cleanup treatment and LUFS target. Preserve the original, duration and channels; deliver the checked WAV and manifest, verify full decode and before/after measured level, and keep subjective-quality review explicit.",
                        29 => "Use the requested typed data operation with this exact CSV, deduplication choice and numeric column list. Preserve the source, deliver the cleaned CSV or editable XLSX plus analysis.json, and report hashes, trimmed cells, removed duplicates, missing counts and checked statistics without inventing a trend narrative.",
                        _ => "Produce these deliverables using the exact Project sources.",
                    },
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
    use super::super::tests::{click, click_at_size, render, snapshot};
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
    fn podcast_request_freezes_ordered_segments_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"raw-episode.wav","kind":"audio","sha256":"a".repeat(64),"bytes":64,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"cover.png","kind":"image","sha256":"b".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 8;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("title") = "Signal & Craft".into();
        *draft.value("segments") = "0 | 12.5 | Opening\n40 | 82 | Main point".into();
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
        assert!(request.contains("\"operation\": \"audio.podcast\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"start\": 0.0"));
        assert!(request.contains("\"end\": 12.5"));
        assert!(request.contains("\"title\": \"Opening\""));
        assert!(request.contains("\"title\": \"Main point\""));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn csv_chart_request_freezes_columns_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"sales.csv","kind":"data","sha256":"a".repeat(64),"bytes":64,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"notes.txt","kind":"text","sha256":"b".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 9;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("title") = "Revenue by region".into();
        *draft.value("categoryColumn") = "region".into();
        *draft.value("valueColumn") = "revenue".into();
        *draft.value("aggregation") = "average".into();
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
        assert!(request.contains("\"operation\": \"data.chart\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"categoryColumn\": \"region\""));
        assert!(request.contains("\"valueColumn\": \"revenue\""));
        assert!(request.contains("\"aggregation\": \"average\""));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn scenario_workbook_request_freezes_columns_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"budget.csv","kind":"data","sha256":"a".repeat(64),"bytes":128,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"notes.txt","kind":"text","sha256":"b".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 10;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("title") = "2027 operating plan".into();
        *draft.value("scenarioColumns") = "line_item | base_plan | growth_case | lean_case".into();
        *draft.value("currency") = "try".into();
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
        assert!(request.contains("\"operation\": \"data.scenario\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"itemColumn\": \"line_item\""));
        assert!(request.contains("\"baseColumn\": \"base_plan\""));
        assert!(request.contains("\"upsideColumn\": \"growth_case\""));
        assert!(request.contains("\"downsideColumn\": \"lean_case\""));
        assert!(request.contains("\"currency\": \"TRY\""));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn structured_extraction_request_freezes_sources_and_schema_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"atlas.txt","kind":"text","sha256":"a".repeat(64),"bytes":128,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"northwind.pdf","kind":"document","sha256":"b".repeat(64),"bytes":256,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"private.bin","kind":"file","sha256":"c".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 11;
        draft.sources.extend([
            "resource:11111111-1111-4111-8111-111111111111".into(),
            "resource:22222222-2222-4222-8222-222222222222".into(),
            "resource:33333333-3333-4333-8333-333333333333".into(),
        ]);
        *draft.value("title") = "Vendor renewal review".into();
        *draft.value("extractionColumns") =
            "vendor: Vendor | renewal: Renewal | seats: Seats".into();
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
        assert!(request.contains("\"operation\": \"data.extract\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("\"key\": \"vendor\""));
        assert!(request.contains("\"label\": \"Renewal\""));
        assert!(request.contains("\"rows\": []"));
        assert!(request.contains("literal evidence quote"));
        assert!(request.contains("Mark absent fields missing"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn attributed_brief_request_freezes_supported_sources_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"pilot-notes.md","kind":"text","sha256":"a".repeat(64),"bytes":128,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"security.pdf","kind":"document","sha256":"b".repeat(64),"bytes":256,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"archive.bin","kind":"file","sha256":"c".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 12;
        draft.sources.extend([
            "resource:11111111-1111-4111-8111-111111111111".into(),
            "resource:22222222-2222-4222-8222-222222222222".into(),
            "resource:33333333-3333-4333-8333-333333333333".into(),
        ]);
        *draft.value("title") = "Pilot decision brief".into();
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
        assert!(request.contains("\"operation\": \"documents.brief\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("\"findings\": []"));
        assert!(request.contains("\"questions\": []"));
        assert!(request.contains("\"actions\": []"));
        assert!(request.contains("literal evidence from its exact source"));
        assert!(request.contains("use null owner or due"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn word_template_request_keeps_blank_fields_missing_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"onboarding-template.docx","kind":"document","sha256":"a".repeat(64),"bytes":512,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"reference.pdf","kind":"document","sha256":"b".repeat(64),"bytes":256,"status":"available"}
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
        draft.kind = 13;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("templateFields") =
            "client_name | Atlas Cloud\napproval_date |\ncontact_email | ops@atlas.example".into();
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
        assert!(request.contains("\"operation\": \"documents.template\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"key\": \"client_name\""));
        assert!(request.contains("\"value\": \"Atlas Cloud\""));
        assert!(request.contains("\"key\": \"approval_date\""));
        assert!(request.contains("\"status\": \"missing\""));
        assert!(request.contains("\"value\": null"));
        assert!(request.contains("Supply every {{field_key}} exactly once"));
        assert!(request.contains("preserve all non-placeholder template content"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn rewrite_request_pins_supported_source_and_stages_protected_terms_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"launch-copy.txt","kind":"text","sha256":"a".repeat(64),"bytes":512,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"reference.pdf","kind":"document","sha256":"b".repeat(64),"bytes":256,"status":"available"}
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
        draft.kind = 14;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("rewriteMode") = "translate".into();
        *draft.value("rewriteTarget") = "Turkish".into();
        *draft.value("preserveTerms") = "Moxcrew, Launch OS".into();
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
        assert!(request.contains("\"operation\": \"documents.rewrite\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"mode\": \"translate\""));
        assert!(request.contains("\"target\": \"Turkish\""));
        assert!(request.contains("\"Moxcrew\""));
        assert!(request.contains("\"Launch OS\""));
        assert!(request.contains("\"segments\": []"));
        assert!(request.contains("every non-empty text line or DOCX body paragraph"));
        assert!(request.contains("never claim semantic equivalence without owner review"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn source_change_request_pins_two_ordered_dated_snapshots_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"launch-may.md","kind":"text","sha256":"a".repeat(64),"bytes":512,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"launch-june.md","kind":"text","sha256":"b".repeat(64),"bytes":640,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"archive.bin","kind":"file","sha256":"c".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 15;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("afterSource") = "resource:22222222-2222-4222-8222-222222222222".into();
        *draft.value("title") = "Launch source changes".into();
        *draft.value("beforeDate") = "2027-05-01".into();
        *draft.value("afterDate") = "2027-06-01".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let rendered = c
            .create
            .capabilities
            .drafts
            .get(&("project:one".into(), 5))
            .unwrap();
        assert_eq!(rendered.kind, 15);
        assert_eq!(
            rendered.record,
            "resource:11111111-1111-4111-8111-111111111111"
        );
        assert_eq!(
            rendered.get("afterSource"),
            "resource:22222222-2222-4222-8222-222222222222"
        );
        assert_eq!(rendered.get("beforeDate"), "2027-05-01");
        assert_eq!(rendered.get("afterDate"), "2027-06-01");
        let visibility = ctx.data(|data| {
            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                "test-action-visibility",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(visibility.1, "button visibility: {visibility:?}");
        let action = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new((
                "test-action",
                "memory_control",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(
            visibility.0.contains(action.center()),
            "button center must be inside the 800×560 clip: clip={:?}, action={action:?}",
            visibility.0
        );
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
        assert!(
            request.contains("\"operation\": \"research.changes\""),
            "{request}"
        );
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("\"capturedAt\": \"2027-05-01\""));
        assert!(request.contains("\"capturedAt\": \"2027-06-01\""));
        assert!(request.contains("do not substitute a live page"));
        assert!(request.contains("hide an explicit no-change result"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn claim_check_request_stages_frozen_sources_and_unresolved_contract_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"survey.md","kind":"text","sha256":"a".repeat(64),"bytes":512,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"audit.txt","kind":"text","sha256":"b".repeat(64),"bytes":640,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"archive.bin","kind":"file","sha256":"c".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 16;
        draft
            .sources
            .insert("resource:11111111-1111-4111-8111-111111111111".into());
        draft
            .sources
            .insert("resource:22222222-2222-4222-8222-222222222222".into());
        *draft.value("title") = "Launch claim check".into();
        *draft.value("claims") =
            "The launch reached 72% adoption.\nRetention lasted twelve months.".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let rendered = c
            .create
            .capabilities
            .drafts
            .get(&("project:one".into(), 5))
            .unwrap();
        assert_eq!(rendered.kind, 16);
        assert_eq!(rendered.sources.len(), 2);
        let visibility = ctx.data(|data| {
            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                "test-action-visibility",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(visibility.1, "button visibility: {visibility:?}");
        let action = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new((
                "test-action",
                "memory_control",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(
            visibility.0.contains(action.center()),
            "claim button center must be inside the 800×560 clip: clip={:?}, action={action:?}",
            visibility.0
        );
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
        assert!(
            request.contains("\"operation\": \"research.claims\""),
            "{request}"
        );
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("The launch reached 72% adoption."));
        assert!(request.contains("Retention lasted twelve months."));
        assert!(request.contains("\"checkedSources\": []"));
        assert!(request.contains("\"evidence\": []"));
        assert!(request.contains("\"unresolvedReason\": null"));
        assert!(request.contains("list every source actually checked"));
        assert!(request.contains("Do not invent evidence"));
        assert!(request.contains("automatic truth score"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn synthesis_request_requires_multiple_supported_sources_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"market.md","kind":"text","sha256":"a".repeat(64),"bytes":512,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"support.txt","kind":"text","sha256":"b".repeat(64),"bytes":640,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"finance.md","kind":"text","sha256":"c".repeat(64),"bytes":700,"status":"available"},
            {"resourceId":"resource:44444444-4444-4444-8444-444444444444","revision":1,"name":"archive.bin","kind":"file","sha256":"d".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 17;
        draft.sources.extend([
            "resource:11111111-1111-4111-8111-111111111111".into(),
            "resource:22222222-2222-4222-8222-222222222222".into(),
            "resource:33333333-3333-4333-8333-333333333333".into(),
            "resource:44444444-4444-4444-8444-444444444444".into(),
        ]);
        *draft.value("title") = "Launch source synthesis".into();
        *draft.value("synthesisFocus") =
            "Where do customer needs and pricing evidence agree or conflict?".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let visibility = ctx.data(|data| {
            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                "test-action-visibility",
                "Add request to Chat",
            )))
            .unwrap()
        });
        let action = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new((
                "test-action",
                "memory_control",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(visibility.1, "button visibility: {visibility:?}");
        assert!(
            visibility.0.contains(action.center()),
            "synthesis button center must fit the 800×560 clip: clip={:?}, action={action:?}",
            visibility.0
        );
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
        assert!(request.contains("\"operation\": \"research.synthesis\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(!request.contains("44444444-4444-4444-8444-444444444444"));
        assert!(request.contains("customer needs and pricing evidence"));
        assert!(request.contains("\"findings\": []"));
        assert!(request.contains("\"disagreements\": []"));
        assert!(request.contains("Cite every selected source"));
        assert!(request.contains("two or more distinct sources"));
        assert!(request.contains("never turn absence"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn comparison_request_stages_even_owner_criteria_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"atlas.md","kind":"text","sha256":"a".repeat(64),"bytes":512,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"boreal.txt","kind":"text","sha256":"b".repeat(64),"bytes":640,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"archive.bin","kind":"file","sha256":"c".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 18;
        draft.sources.extend([
            "resource:11111111-1111-4111-8111-111111111111".into(),
            "resource:22222222-2222-4222-8222-222222222222".into(),
            "resource:33333333-3333-4333-8333-333333333333".into(),
        ]);
        *draft.value("title") = "Vendor comparison".into();
        *draft.value("comparisonQuestion") =
            "Which vendor best fits the owner's launch criteria?".into();
        *draft.value("comparisonCriteria") =
            "cost: Monthly cost | setup: Setup speed | offline: Offline export".into();
        *draft.value("comparisonOptions") = "Atlas | Boreal".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let visibility = ctx.data(|data| {
            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                "test-action-visibility",
                "Add request to Chat",
            )))
            .unwrap()
        });
        let action = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new((
                "test-action",
                "memory_control",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(visibility.1, "button visibility: {visibility:?}");
        assert!(
            visibility.0.contains(action.center()),
            "comparison button center must fit the 800×560 clip: clip={:?}, action={action:?}",
            visibility.0
        );
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
        assert!(request.contains("\"operation\": \"research.compare\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("Which vendor best fits"));
        assert!(request.contains("\"option\": \"Atlas\""));
        assert!(request.contains("\"option\": \"Boreal\""));
        assert!(request.contains("\"offline\": {"));
        assert!(request.contains("\"status\": \"missing\""));
        assert!(request.contains("\"recommendation\": {"));
        assert!(request.contains("\"basedOnCriteria\": []"));
        assert!(request.contains("Apply every owner criterion to every option"));
        assert!(request.contains("Never invent quotes"));
        assert!(request.contains("automatic truth"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn web_research_request_stages_public_sources_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut c = console();
        c.create.select_productive_view(3);
        c.create.capabilities.tab = 5;
        let draft = c
            .create
            .capabilities
            .drafts
            .entry(("project:one".into(), 5))
            .or_default();
        draft.kind = 19;
        *draft.value("title") = "Launch market research".into();
        *draft.value("webResearchQuestion") =
            "What do public sources establish about demand and pricing?".into();
        *draft.value("webResearchSources") =
            "Market bulletin | https://example.com/market\nPricing brief | https://example.org/pricing"
                .into();
        *draft.value("webResearchClaims") =
            "Teams request weekly summaries.\nThe public price is 120 TRY.".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let visibility = ctx.data(|data| {
            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                "test-action-visibility",
                "Add request to Chat",
            )))
            .unwrap()
        });
        let action = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new((
                "test-action",
                "memory_control",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(visibility.1, "button visibility: {visibility:?}");
        assert!(
            visibility.0.contains(action.center()),
            "web research button center must fit the 800×560 clip: clip={:?}, action={action:?}",
            visibility.0
        );
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
        assert!(request.contains("\"operation\": \"browser.research\""));
        assert!(request.contains("https://example.com/market"));
        assert!(request.contains("https://example.org/pricing"));
        assert!(request.contains("Teams request weekly summaries."));
        assert!(request.contains("\"classification\": \"fact\""));
        assert!(request.contains("\"evidence\": []"));
        assert!(request.contains("Inspect each requested public URL"));
        assert!(request.contains("record UTC access times"));
        assert!(request.contains("Do not use private pages"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn pdf_merge_request_keeps_owner_order_and_stages_no_direct_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"executive-summary.pdf","kind":"document","sha256":"a".repeat(64),"bytes":256,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"financial-appendix.pdf","kind":"document","sha256":"b".repeat(64),"bytes":256,"status":"available"},
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
        draft.kind = 21;
        draft
            .sources
            .insert("resource:11111111-1111-4111-8111-111111111111".into());
        draft
            .sources
            .insert("resource:22222222-2222-4222-8222-222222222222".into());
        *draft.value("pdfMergeOrder") = "resource:22222222-2222-4222-8222-222222222222\nresource:11111111-1111-4111-8111-111111111111".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let visibility = ctx.data(|data| {
            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                "test-action-visibility",
                "Add request to Chat",
            )))
            .unwrap()
        });
        assert!(visibility.1, "button visibility: {visibility:?}");
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
        assert!(request.contains("\"operation\": \"pdf.merge\""));
        let second = request
            .find("resource:22222222-2222-4222-8222-222222222222")
            .unwrap();
        let first = request
            .find("resource:11111111-1111-4111-8111-111111111111")
            .unwrap();
        assert!(second < first, "owner PDF order changed: {request}");
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("verified order"));
        assert!(request.contains("first/last render previews"));
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
    fn branded_presentation_request_stages_editable_layouts_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"brand-logo.png","kind":"image","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"notes.txt","kind":"text","sha256":"b".repeat(64),"bytes":64,"status":"available"}
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
        draft.kind = 22;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("brandName") = "Moxcrew".into();
        *draft.value("primary") = "2854F4".into();
        *draft.value("accent") = "FF7A2F".into();
        *draft.value("background") = "F8F7F3".into();
        *draft.value("text") = "15223D".into();
        *draft.value("muted") = "56627A".into();
        *draft.value("font") = "Arial".into();
        *draft.value("chartSource") = "Source: checked pilot dataset".into();
        *draft.value("presentationSlides") = "cover | Work with a crew you can inspect | One local project, one visible plan\ncontent | What stays under your control | Sources stay local; Preparation remains explicit; Every output keeps its version\nbar-chart | Pilot completion | Before=42; With Mox=78 | %".into();
        *draft.value("renderSlides") = "true".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        let commands = click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat",
        );
        assert!(commands.is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"presentations.create\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"layout\": \"bar-chart\""));
        assert!(request.contains("\"value\": 78.0"));
        assert!(request.contains("\"render\": true"));
        assert!(request.contains("preserve the supplied data literally"));
        assert_eq!(c.create.productive_view(), 0);
    }
    #[test]
    fn presentation_slide_rows_reject_unknown_layouts_and_parse_chart_values() {
        let slides = presentation_slides(
            "statement | One clear point | Supporting detail\nbar-chart | Adoption | Before=41.5; After=79 | %",
            "Source: pilot",
        )
        .unwrap();
        assert_eq!(slides[0]["layout"], "statement");
        assert_eq!(slides[1]["chart"]["series"][0]["value"], 41.5);
        assert_eq!(slides[1]["chart"]["sourceLabel"], "Source: pilot");
        assert!(presentation_slides("dashboard | Unsupported | Value", "").is_none());
    }
    #[test]
    fn presentation_form_is_scroll_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                let mut c = console();
                c.create.select_productive_view(3);
                c.create.capabilities.tab = 5;
                let draft = c
                    .create
                    .capabilities
                    .drafts
                    .entry(("project:one".into(), 5))
                    .or_default();
                draft.kind = 22;
                *draft.value("presentationSlides") =
                    "cover | Visible work | One project, one plan".into();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                for _ in 0..5 {
                    render(
                        &ctx,
                        &mut c,
                        &data,
                        size,
                        false,
                        vec![
                            egui::Event::PointerMoved(egui::pos2(680.0, 360.0)),
                            egui::Event::MouseWheel {
                                unit: egui::MouseWheelUnit::Page,
                                delta: egui::vec2(0.0, -1.0),
                                phase: egui::TouchPhase::Move,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    );
                }
                render(&ctx, &mut c, &data, size, false, vec![]);
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(
                    visibility.1,
                    "{language}/{theme:?}: action must be scroll-reachable"
                );
                assert!(click_at_size(
                    &ctx,
                    &mut c,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Add request to Chat"
                )
                .is_empty());
                assert!(c
                    .create
                    .conversation_input()
                    .contains("\"operation\": \"presentations.create\""));
            }
        }
    }
    #[test]
    fn presentation_brand_request_preserves_the_selected_deck_and_only_stages_chat() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"launch-deck.pptx","kind":"document","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"wrong-deck.pptx","kind":"document","sha256":"b".repeat(64),"bytes":4096,"status":"available"}
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
        draft.kind = 23;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("brandName") = "Night Shift".into();
        *draft.value("primary") = "9BAEFF".into();
        *draft.value("accent") = "FF9B66".into();
        *draft.value("background") = "090D18".into();
        *draft.value("text") = "F2F5FF".into();
        *draft.value("muted") = "A7B3CC".into();
        *draft.value("font") = "Arial".into();
        *draft.value("renderSlides") = "false".into();
        for _ in 0..3 {
            render(&ctx, &mut c, &data, egui::vec2(800.0, 560.0), false, vec![]);
        }
        assert!(click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"presentations.brand\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"render\": false"));
        assert!(request.contains("Preserve every slide, note, image and relationship part"));
        assert!(request.contains("do not claim full visual adaptation"));
    }
    #[test]
    fn narrated_presentation_request_uses_exact_filename_order_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"03-proof.m4a","kind":"audio","sha256":"c".repeat(64),"bytes":300,"status":"available"},
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"brief.pptx","kind":"document","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:44444444-4444-4444-8444-444444444444","revision":1,"name":"unused.m4a","kind":"audio","sha256":"d".repeat(64),"bytes":300,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"01-opening.m4a","kind":"audio","sha256":"b".repeat(64),"bytes":300,"status":"available"}
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
        draft.kind = 24;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        draft
            .sources
            .insert("resource:33333333-3333-4333-8333-333333333333".into());
        draft
            .sources
            .insert("resource:22222222-2222-4222-8222-222222222222".into());
        *draft.value("presentationFps") = "25".into();
        for _ in 0..3 {
            render(&ctx, &mut c, &data, egui::vec2(800.0, 560.0), false, vec![]);
        }
        assert!(click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"presentations.narrate\""));
        assert!(request.contains("\"fps\": 25"));
        let first = request
            .find("22222222-2222-4222-8222-222222222222")
            .unwrap();
        let third = request
            .find("33333333-3333-4333-8333-333333333333")
            .unwrap();
        assert!(first < third, "narration order changed: {request}");
        assert!(!request.contains("44444444-4444-4444-8444-444444444444"));
        assert!(request.contains("spoken-word matching still needs human review"));
    }
    #[test]
    fn narration_form_is_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectStudio"]["resources"] = json!([
                    {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"brief.pptx","kind":"document","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
                    {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"01-opening.m4a","kind":"audio","sha256":"b".repeat(64),"bytes":300,"status":"available"}
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
                draft.kind = 24;
                draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
                draft
                    .sources
                    .insert("resource:22222222-2222-4222-8222-222222222222".into());
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                for _ in 0..3 {
                    render(
                        &ctx,
                        &mut c,
                        &data,
                        size,
                        false,
                        vec![
                            egui::Event::PointerMoved(egui::pos2(680.0, 360.0)),
                            egui::Event::MouseWheel {
                                unit: egui::MouseWheelUnit::Page,
                                delta: egui::vec2(0.0, -1.0),
                                phase: egui::TouchPhase::Move,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    );
                }
                render(&ctx, &mut c, &data, size, false, vec![]);
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(visibility.1, "{language}/{theme:?}: narration action");
            }
        }
    }
    #[test]
    fn composed_video_request_freezes_exact_resources_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"source.mp4","kind":"video","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"mox.png","kind":"image","sha256":"b".repeat(64),"bytes":1024,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"approved.wav","kind":"audio","sha256":"c".repeat(64),"bytes":2048,"status":"available"},
            {"resourceId":"resource:44444444-4444-4444-8444-444444444444","revision":1,"name":"unused.wav","kind":"audio","sha256":"d".repeat(64),"bytes":2048,"status":"available"}
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
        draft.kind = 25;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("compositionImage") = "resource:22222222-2222-4222-8222-222222222222".into();
        *draft.value("compositionAudio") = "resource:33333333-3333-4333-8333-333333333333".into();
        *draft.value("compositionTitle") = "Every handoff stays visible".into();
        *draft.value("compositionTitleSeconds") = "2.5".into();
        *draft.value("compositionOverlayStart") = "3".into();
        *draft.value("compositionOverlayEnd") = "8".into();
        *draft.value("compositionPosition") = "top-right".into();
        *draft.value("compositionAudioMode") = "replace".into();
        *draft.value("compositionAudioLevel") = "0.8".into();
        for _ in 0..3 {
            render(&ctx, &mut c, &data, egui::vec2(800.0, 560.0), false, vec![]);
        }
        assert!(click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"video.compose\""));
        for id in [
            "11111111-1111-4111-8111-111111111111",
            "22222222-2222-4222-8222-222222222222",
            "33333333-3333-4333-8333-333333333333",
        ] {
            assert!(request.contains(id), "missing frozen resource {id}");
        }
        assert!(!request.contains("44444444-4444-4444-8444-444444444444"));
        assert!(request.contains("\"position\": \"top-right\""));
        assert!(request.contains("\"audioMode\": \"replace\""));
        assert!(request.contains("decode the full output"));
    }
    #[test]
    fn composed_video_form_is_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectStudio"]["resources"] = json!([
                    {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"source.mp4","kind":"video","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
                    {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"mox.png","kind":"image","sha256":"b".repeat(64),"bytes":1024,"status":"available"},
                    {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"approved.wav","kind":"audio","sha256":"c".repeat(64),"bytes":2048,"status":"available"}
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
                draft.kind = 25;
                draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
                *draft.value("compositionImage") =
                    "resource:22222222-2222-4222-8222-222222222222".into();
                *draft.value("compositionAudio") =
                    "resource:33333333-3333-4333-8333-333333333333".into();
                *draft.value("compositionTitle") = "Visible composition".into();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                for _ in 0..5 {
                    render(
                        &ctx,
                        &mut c,
                        &data,
                        size,
                        false,
                        vec![
                            egui::Event::PointerMoved(egui::pos2(680.0, 360.0)),
                            egui::Event::MouseWheel {
                                unit: egui::MouseWheelUnit::Page,
                                delta: egui::vec2(0.0, -1.0),
                                phase: egui::TouchPhase::Move,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    );
                }
                render(&ctx, &mut c, &data, size, false, vec![]);
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(visibility.1, "{language}/{theme:?}: composition action");
            }
        }
    }
    #[test]
    fn video_highlight_request_freezes_exact_source_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"source.mp4","kind":"video","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"unused.mp4","kind":"video","sha256":"b".repeat(64),"bytes":4096,"status":"available"}
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
        draft.kind = 26;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("highlightTargetSeconds") = "12".into();
        *draft.value("highlightClipSeconds") = "4".into();
        *draft.value("highlightEmphasis") = "visual".into();
        for _ in 0..3 {
            render(&ctx, &mut c, &data, egui::vec2(800.0, 560.0), false, vec![]);
        }
        assert!(click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"video.highlights\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"targetSeconds\": 12.0"));
        assert!(request.contains("\"clipSeconds\": 4.0"));
        assert!(request.contains("\"emphasis\": \"visual\""));
        assert!(request.contains("motion and loudness do not prove meaning"));
    }
    #[test]
    fn video_highlight_form_is_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectStudio"]["resources"] = json!([
                    {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"source.mp4","kind":"video","sha256":"a".repeat(64),"bytes":4096,"status":"available"}
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
                draft.kind = 26;
                draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
                *draft.value("highlightTargetSeconds") = "8".into();
                *draft.value("highlightClipSeconds") = "4".into();
                *draft.value("highlightEmphasis") = "balanced".into();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(visibility.1, "{language}/{theme:?}: highlight action");
            }
        }
    }
    #[test]
    fn motion_video_request_freezes_exact_scene_plan_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut c = console();
        c.create.select_productive_view(3);
        c.create.capabilities.tab = 5;
        let draft = c
            .create
            .capabilities
            .drafts
            .entry(("project:one".into(), 5))
            .or_default();
        draft.kind = 27;
        *draft.value("motionTitle") = "Mox keeps work visible".into();
        *draft.value("motionScenes") = "cover | 3 | MEET MOX | A crew you can actually see | Plan, produce and review in one local workspace.\nstatement | 4 | OWNER CONTROL | Every handoff stays visible. | Nothing starts until you say so.".into();
        *draft.value("motionTreatment") = "drift".into();
        *draft.value("motionFps") = "30".into();
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            render(&ctx, &mut c, &data, size, false, vec![]);
        }
        for _ in 0..4 {
            render(
                &ctx,
                &mut c,
                &data,
                size,
                false,
                vec![
                    egui::Event::PointerMoved(egui::pos2(680.0, 360.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Page,
                        delta: egui::vec2(0.0, -1.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
            );
        }
        assert!(click_at_size(
            &ctx,
            &mut c,
            &data,
            size,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"video.motion\""));
        assert!(request.contains("A crew you can actually see"));
        assert!(request.contains("Every handoff stays visible."));
        assert!(request.contains("\"duration\": 4.0"));
        assert!(request.contains("\"motion\": \"drift\""));
        assert!(request.contains("\"fps\": 30"));
        assert!(request.contains("editable storyboard and rendered MP4"));
    }
    #[test]
    fn motion_video_form_is_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                let mut c = console();
                c.create.select_productive_view(3);
                c.create.capabilities.tab = 5;
                let draft = c
                    .create
                    .capabilities
                    .drafts
                    .entry(("project:one".into(), 5))
                    .or_default();
                draft.kind = 27;
                *draft.value("motionTitle") = "Visible motion".into();
                *draft.value("motionScenes") = "cover | 3 | MEET MOX | Visible work | One local workspace.\nstatement | 3 | OWNER CONTROL | Keep the final call. | Every handoff stays visible.".into();
                *draft.value("motionTreatment") = "calm".into();
                *draft.value("motionFps") = "24".into();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                for _ in 0..4 {
                    render(
                        &ctx,
                        &mut c,
                        &data,
                        size,
                        false,
                        vec![
                            egui::Event::PointerMoved(egui::pos2(680.0, 360.0)),
                            egui::Event::MouseWheel {
                                unit: egui::MouseWheelUnit::Page,
                                delta: egui::vec2(0.0, -1.0),
                                phase: egui::TouchPhase::Move,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    );
                }
                render(&ctx, &mut c, &data, size, false, vec![]);
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(visibility.1, "{language}/{theme:?}: motion action");
            }
        }
    }
    #[test]
    fn transcription_request_freezes_exact_source_and_language_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"interview.wav","kind":"audio","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"unused.wav","kind":"audio","sha256":"b".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:33333333-3333-4333-8333-333333333333","revision":1,"name":"notes.txt","kind":"document","sha256":"c".repeat(64),"bytes":4096,"status":"available"}
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
        draft.kind = 5;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("language") = "tr".into();
        for _ in 0..3 {
            render(&ctx, &mut c, &data, egui::vec2(800.0, 560.0), false, vec![]);
        }
        assert!(click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"audio.transcribe\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(!request.contains("33333333-3333-4333-8333-333333333333"));
        assert!(request.contains("\"language\": \"tr\""));
        assert!(request.contains("Transcript content accuracy"));
    }
    #[test]
    fn transcription_form_is_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectStudio"]["resources"] = json!([
                    {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"interview.wav","kind":"audio","sha256":"a".repeat(64),"bytes":4096,"status":"available"}
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
                draft.kind = 5;
                draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
                *draft.value("language") = "auto".into();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(visibility.1, "{language}/{theme:?}: transcription action");
            }
        }
    }
    #[test]
    fn audio_cleanup_request_freezes_exact_source_and_policy_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"field.wav","kind":"audio","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"unused.wav","kind":"audio","sha256":"b".repeat(64),"bytes":4096,"status":"available"}
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
        draft.kind = 28;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("cleanupTreatment") = "voice".into();
        *draft.value("cleanupTargetLufs") = "-16".into();
        for _ in 0..3 {
            render(&ctx, &mut c, &data, egui::vec2(800.0, 560.0), false, vec![]);
        }
        assert!(click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"audio.cleanup\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"targetLufs\": -16"));
        assert!(request.contains("\"treatment\": \"voice\""));
        assert!(request.contains("before/after measured level"));
    }
    #[test]
    fn audio_cleanup_form_is_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectStudio"]["resources"] = json!([
                    {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"field.wav","kind":"audio","sha256":"a".repeat(64),"bytes":4096,"status":"available"}
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
                draft.kind = 28;
                draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
                *draft.value("cleanupTreatment") = "gentle".into();
                *draft.value("cleanupTargetLufs") = "-20".into();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(visibility.1, "{language}/{theme:?}: audio cleanup action");
            }
        }
    }
    #[test]
    fn data_profile_request_freezes_exact_source_and_policy_without_execution() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectStudio"]["resources"] = json!([
            {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"operations.csv","kind":"data","sha256":"a".repeat(64),"bytes":4096,"status":"available"},
            {"resourceId":"resource:22222222-2222-4222-8222-222222222222","revision":1,"name":"unused.csv","kind":"data","sha256":"b".repeat(64),"bytes":4096,"status":"available"}
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
        draft.kind = 29;
        draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
        *draft.value("dataDeduplicate") = "true".into();
        *draft.value("dataNumericColumns") = "revenue, units".into();
        *draft.value("dataOutput") = "xlsx".into();
        for _ in 0..3 {
            render(&ctx, &mut c, &data, egui::vec2(800.0, 560.0), false, vec![]);
        }
        assert!(click(
            &ctx,
            &mut c,
            &data,
            false,
            "memory_control",
            "Add request to Chat"
        )
        .is_empty());
        let request = c.create.conversation_input();
        assert!(request.contains("\"operation\": \"data.xlsx\""));
        assert!(request.contains("11111111-1111-4111-8111-111111111111"));
        assert!(!request.contains("22222222-2222-4222-8222-222222222222"));
        assert!(request.contains("\"deduplicate\": true"));
        assert!(request.contains("\"revenue\""));
        assert!(request.contains("\"units\""));
        assert!(request.contains("trimmed cells"));
    }
    #[test]
    fn data_profile_form_is_reachable_at_minimum_size_in_both_languages_and_themes() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectStudio"]["resources"] = json!([
                    {"resourceId":"resource:11111111-1111-4111-8111-111111111111","revision":1,"name":"operations.csv","kind":"data","sha256":"a".repeat(64),"bytes":4096,"status":"available"}
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
                draft.kind = 29;
                draft.record = "resource:11111111-1111-4111-8111-111111111111".into();
                *draft.value("dataDeduplicate") = "true".into();
                *draft.value("dataNumericColumns") = "revenue".into();
                *draft.value("dataOutput") = "csv".into();
                let size = egui::vec2(800.0, 560.0);
                for _ in 0..3 {
                    render(&ctx, &mut c, &data, size, false, vec![]);
                }
                for _ in 0..3 {
                    render(
                        &ctx,
                        &mut c,
                        &data,
                        size,
                        false,
                        vec![
                            egui::Event::PointerMoved(egui::pos2(680.0, 360.0)),
                            egui::Event::MouseWheel {
                                unit: egui::MouseWheelUnit::Page,
                                delta: egui::vec2(0.0, -1.0),
                                phase: egui::TouchPhase::Move,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    );
                }
                render(&ctx, &mut c, &data, size, false, vec![]);
                let visibility = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Add request to Chat",
                    )))
                    .unwrap()
                });
                assert!(visibility.1, "{language}/{theme:?}: data profile action");
            }
        }
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
