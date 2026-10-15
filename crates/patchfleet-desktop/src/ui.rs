use crate::command::{Command, Notice, NoticeKind};
use crate::folder;
use crate::i18n::{self, Language};
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::{json, Value};
use std::collections::HashMap;

const MINT: Color32 = Color32::from_rgb(139, 224, 188);
const MUTED: Color32 = Color32::from_rgb(134, 145, 159);
const SURFACE: Color32 = Color32::from_rgb(22, 26, 32);
const TEXT: Color32 = Color32::from_rgb(232, 235, 239);
const BORDER: Color32 = Color32::from_rgb(43, 48, 57);
const RED: Color32 = Color32::from_rgb(246, 142, 151);

macro_rules! localized_format {
    ($ui:expr,$key:literal,$($value:expr),+ $(,)?) => {
        tf($ui,$key,&[$(&$value.to_string()),+])
    };
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Projects,
    Memory,
    Decisions,
    Work,
    Teams,
    Agents,
    Providers,
    Cloud,
}
impl Tab {
    const ALL: [Self; 8] = [
        Self::Projects,
        Self::Memory,
        Self::Decisions,
        Self::Work,
        Self::Teams,
        Self::Agents,
        Self::Providers,
        Self::Cloud,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Projects => "Projects",
            Self::Memory => "Memory",
            Self::Decisions => "Decisions",
            Self::Work => "Work",
            Self::Teams => "Teams",
            Self::Agents => "Agents",
            Self::Providers => "Providers",
            Self::Cloud => "Cloud",
        }
    }
    fn summary(self) -> &'static str {
        match self {
            Self::Projects => "Your projects, local folders and shared memory.",
            Self::Memory => "Project sources with provenance. ADR extraction comes next.",
            Self::Decisions => "Sourced local decisions. Acceptance stays in your hands.",
            Self::Work => "Queue a bounded task, start Codex, and verify its durable outcome.",
            Self::Teams => "Choose agents and a bounded plan. Owner decisions stay explicit.",
            Self::Agents => {
                "Inspect instructions, permissions and limits before selecting an agent."
            }
            Self::Providers => {
                "Capabilities reflect the installed CLIs and last local observation."
            }
            Self::Cloud => "An optional sanitized remote view. Local operation stays available.",
        }
    }
}

#[derive(Default)]
struct WorkDraft {
    title: String,
    instruction: String,
    workspace: String,
    path: String,
    path_problem: Option<String>,
}
impl WorkDraft {
    fn payload(&self, snapshot: &Value) -> Result<Value, &'static str> {
        required(&self.title, 160, "Add a task name (up to 160 characters).")?;
        required(
            &self.instruction,
            50_000,
            "Add a bounded instruction (up to 50,000 characters).",
        )?;
        let selected = self.workspace.trim();
        let manual = self.path.trim();
        if !manual.is_empty() && self.path_problem.is_some() {
            return Err("Choose an accessible Git folder before adding this task.");
        }
        if selected.is_empty() == manual.is_empty() {
            return Err("Choose a registered worktree or one absolute worktree path.");
        }
        if !selected.is_empty()
            && !items(&snapshot["workspaces"])
                .iter()
                .any(|w| text(w, "workspaceId") == selected)
        {
            return Err("That worktree is no longer registered. Choose a current worktree.");
        }
        Ok(
            json!({"title":self.title.trim(),"instruction":self.instruction.trim(),"workspaceId":selected,"workingDirectory":manual}),
        )
    }
}

struct TeamDraft {
    name: String,
    goal: String,
    workspace: String,
    template: String,
    orchestrator: String,
    workers: [String; 4],
    concurrency: u32,
    retries: u32,
    minutes: u32,
    continue_independent: bool,
}
impl Default for TeamDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            goal: String::new(),
            workspace: String::new(),
            template: "product-feature".into(),
            orchestrator: "pack:orchestrator".into(),
            workers: Default::default(),
            concurrency: 2,
            retries: 1,
            minutes: 120,
            continue_independent: false,
        }
    }
}
impl TeamDraft {
    fn payload(&self, snapshot: &Value) -> Result<Value, &'static str> {
        required(&self.name, 80, "Add a team name (up to 80 characters).")?;
        required(
            &self.goal,
            10_000,
            "Describe the team goal (up to 10,000 characters).",
        )?;
        if !items(&snapshot["workspaces"])
            .iter()
            .any(|w| text(w, "workspaceId") == self.workspace)
        {
            return Err("Choose a registered worktree.");
        }
        if !array(&snapshot["templates"])
            .iter()
            .any(|t| text(t, "id") == self.template)
        {
            return Err("Choose an available team template.");
        }
        let packs = array(&snapshot["packs"]);
        if !packs
            .iter()
            .any(|p| text(p, "id") == self.orchestrator && text(p, "role") == "orchestrator")
        {
            return Err("Choose an installed orchestrator.");
        }
        let workers: Vec<&str> = self
            .workers
            .iter()
            .map(String::as_str)
            .filter(|s| !s.is_empty())
            .collect();
        if workers.is_empty() {
            return Err("Choose at least one worker agent.");
        }
        for (i, id) in workers.iter().enumerate() {
            if workers[..i].contains(id) {
                return Err("Select each worker pack only once.");
            }
            if !packs
                .iter()
                .any(|p| text(p, "id") == *id && text(p, "role") != "orchestrator")
            {
                return Err("A selected worker is no longer installed.");
            }
        }
        if !(1..=4).contains(&self.concurrency)
            || self.retries > 3
            || !(5..=480).contains(&self.minutes)
        {
            return Err("Use concurrency 1–4, retries 0–3 and a time budget of 5–480 minutes.");
        }
        Ok(
            json!({"name":self.name.trim(),"goal":self.goal.trim(),"workspaceId":self.workspace,"templateId":self.template,"orchestratorPackId":self.orchestrator,"workerPackIds":workers,"settings":{"concurrency":self.concurrency,"retryLimit":self.retries,"timeoutMinutes":self.minutes,"failurePolicy":if self.continue_independent {"continue"} else {"stop"}}}),
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
struct MemoryRequest {
    project: String,
    revision: u64,
    offset: u64,
    query: String,
}
#[derive(Default)]
struct MemoryDraft {
    project: String,
    title: String,
    body: String,
    keyword_input: String,
    query: String,
    offset: u64,
    requested: Option<MemoryRequest>,
    refresh_requested: bool,
    edit: Option<SourceEdit>,
}
struct SourceEdit {
    project: String,
    source: String,
    revision: u64,
    title: String,
    body: String,
}
impl MemoryDraft {
    fn select(&mut self, project: &str) {
        if self.project != project {
            *self = Self {
                project: project.to_owned(),
                ..Default::default()
            };
        }
    }
    fn page_matches(&self, page: &Value, project: &Value) -> bool {
        text(page, "projectId") == self.project
            && revision(page) == revision(project)
            && page["offset"].as_u64() == Some(self.offset)
            && text(page, "query") == self.query
            && text(page, "reasonCode") == "PROJECT_MEMORY_READ"
    }
    fn next_read(&mut self, project: &Value, page: &Value, busy: bool) -> Option<Command> {
        if busy
            || self.project.is_empty()
            || revision(project) == 0
            || (!self.refresh_requested && self.page_matches(page, project))
        {
            return None;
        }
        let same = self.requested.as_ref().is_some_and(|r| {
            r.project == self.project
                && r.revision == revision(project)
                && r.offset == self.offset
                && r.query == self.query
        });
        if same && !self.refresh_requested {
            return None;
        }
        self.refresh_requested = false;
        self.requested = Some(MemoryRequest {
            project: self.project.clone(),
            revision: revision(project),
            offset: self.offset,
            query: self.query.clone(),
        });
        Some(Command::new(
            "project_memory_read",
            json!({"projectId":self.project,"offset":self.offset,"limit":10,"query":self.query}),
        ))
    }
    fn message_payload(&self, project: &Value) -> Result<Value, &'static str> {
        if text(project, "projectId") != self.project || revision(project) == 0 {
            return Err("Choose a current project before writing a message.");
        }
        required(
            &self.title,
            160,
            "Add a message title of up to 160 characters.",
        )?;
        required(
            &self.body,
            50_000,
            "Add source text of up to 50,000 characters.",
        )?;
        Ok(
            json!({"projectId":self.project,"title":self.title.trim(),"text":self.body.trim(),"expectedProjectRevision":project["revision"]}),
        )
    }
}

#[derive(Default)]
struct DecisionPanel {
    project: String,
    query_input: String,
    query: String,
    status: String,
    offset: u64,
    list_requested: Option<(String, u64, u64, String, String)>,
    refresh_list: bool,
    selected: String,
    detail_requested: Option<(String, u64, String)>,
    refresh_detail: bool,
    replacement: String,
    composer_open: bool,
    pending_proposal_manifest: Option<u64>,
    title: String,
    context: String,
    decision: String,
    rationale: String,
    consequences: String,
    sources: MemoryDraft,
    citation_epoch: u64,
    citations: Vec<Citation>,
    sources_changed: bool,
}
struct Citation {
    source: String,
    revision: u64,
    title: String,
}
impl DecisionPanel {
    fn finish_proposal(&mut self, snapshot: &Value, notice: Option<&Notice>, busy: bool) {
        if !busy
            && self
                .pending_proposal_manifest
                .is_some_and(|before| decision_manifest(snapshot, &self.project) > before)
            && notice.is_some_and(|n| {
                n.kind == NoticeKind::Success && n.code == "PROJECT_DECISION_PROPOSED"
            })
        {
            self.pending_proposal_manifest = None;
            self.composer_open = false;
            self.title.clear();
            self.context.clear();
            self.decision.clear();
            self.rationale.clear();
            self.consequences.clear();
            self.citations.clear();
            self.sources_changed = false;
            self.status = "all".into();
            self.query.clear();
            self.query_input.clear();
            self.offset = 0;
            self.selected.clear();
            self.refresh_list = true;
        }
    }
    fn select(&mut self, project: &str) {
        if self.project != project {
            *self = Self {
                project: project.to_owned(),
                status: "all".into(),
                ..Default::default()
            };
            self.sources.select(project);
        }
    }
    fn page_matches(&self, page: &Value, manifest: u64) -> bool {
        text(page, "reasonCode") == "PROJECT_DECISIONS_READ"
            && text(page, "projectId") == self.project
            && revision(page) == manifest
            && page["offset"].as_u64() == Some(self.offset)
            && page["limit"].as_u64() == Some(10)
            && text(page, "query") == self.query
            && text(page, "status") == self.status
    }
    fn detail_matches(&self, detail: &Value, manifest: u64) -> bool {
        text(detail, "reasonCode") == "PROJECT_DECISION_READ"
            && text(detail, "projectId") == self.project
            && revision(detail) == manifest
            && text(&detail["decision"], "projectId") == self.project
            && text(&detail["decision"], "decisionId") == self.selected
            && revision(&detail["decision"]) > 0
    }
    fn refresh(&mut self) {
        self.refresh_list = true;
        self.refresh_detail = true;
    }
    fn next_list(&mut self, page: &Value, manifest: u64, busy: bool) -> Option<Command> {
        if busy
            || self.project.is_empty()
            || (!self.refresh_list && self.page_matches(page, manifest))
        {
            return None;
        }
        let key = (
            self.project.clone(),
            manifest,
            self.offset,
            self.query.clone(),
            self.status.clone(),
        );
        if !self.refresh_list && self.list_requested.as_ref() == Some(&key) {
            return None;
        }
        self.refresh_list = false;
        self.list_requested = Some(key);
        Some(Command::new(
            "project_decisions_read",
            json!({"projectId":self.project,"offset":self.offset,"limit":10,"query":self.query,"status":self.status}),
        ))
    }
    fn next_detail(&mut self, detail: &Value, manifest: u64, busy: bool) -> Option<Command> {
        if busy
            || self.selected.is_empty()
            || (!self.refresh_detail && self.detail_matches(detail, manifest))
        {
            return None;
        }
        let key = (self.project.clone(), manifest, self.selected.clone());
        if !self.refresh_detail && self.detail_requested.as_ref() == Some(&key) {
            return None;
        }
        self.refresh_detail = false;
        self.detail_requested = Some(key);
        Some(Command::new(
            "project_decision_read",
            json!({"projectId":self.project,"decisionId":self.selected}),
        ))
    }
    fn verify_citation_epoch(&mut self, project: &Value) {
        if self.citation_epoch != revision(project) {
            self.sources_changed = !self.citations.is_empty();
            self.citations.clear();
            self.citation_epoch = revision(project);
        }
    }
    fn proposal(&self, project: &Value) -> Result<Value, &'static str> {
        if text(project, "projectId") != self.project
            || revision(project) == 0
            || self.citation_epoch != revision(project)
        {
            return Err("Refresh the project and select current evidence before saving.");
        }
        if !decision_text(&self.title, 160, true)
            || !decision_text(&self.decision, 20_000, true)
            || !decision_text(&self.context, 10_000, false)
            || !decision_text(&self.rationale, 10_000, false)
            || !decision_text(&self.consequences, 10_000, false)
        {
            return Err("Add a title and decision within the text limits. Control characters are not allowed.");
        }
        if !(1..=8).contains(&self.citations.len())
            || self.citations.iter().enumerate().any(|(i, c)| {
                c.revision == 0
                    || c.source.is_empty()
                    || self.citations[..i]
                        .iter()
                        .any(|other| other.source == c.source)
            })
        {
            return Err("Select one to eight current project sources.");
        }
        Ok(
            json!({"projectId":self.project,"expectedProjectRevision":revision(project),"title":self.title.trim(),"context":self.context.trim(),"decision":self.decision.trim(),"rationale":self.rationale.trim(),"consequences":self.consequences.trim(),"sources":self.citations.iter().map(|c|json!({"sourceId":c.source,"revision":c.revision})).collect::<Vec<_>>()}),
        )
    }
}

#[derive(Default)]
pub struct Console {
    tab: Tab,
    new_project_name: String,
    link_choices: HashMap<String, String>,
    memory: MemoryDraft,
    decisions: DecisionPanel,
    project_name: String,
    project_path: String,
    project_path_problem: Option<String>,
    registration_open: bool,
    new_project_open: bool,
    registration_project: Option<String>,
    folder_request: Option<folder::Target>,
    pub notice_action: Option<NoticeAction>,
    work: WorkDraft,
    team: TeamDraft,
    manifest: String,
    cloud_url: String,
    host_name: String,
    pairing_code: String,
    notes: HashMap<String, String>,
    answers: HashMap<String, String>,
    form_error: Option<&'static str>,
    work_page: usize,
}

#[derive(Clone, Copy)]
pub enum NoticeAction {
    Dismiss,
    Retry,
    #[cfg(target_os = "macos")]
    Settings,
}

impl Console {
    #[cfg(feature = "ui-smoke")]
    pub fn memory_smoke(&mut self) {
        self.tab = Tab::Memory;
    }

    #[cfg(feature = "ui-smoke")]
    pub fn decision_smoke(&mut self) {
        self.tab = Tab::Decisions;
    }

    #[cfg(feature = "ui-smoke")]
    pub fn folder_smoke(&mut self, path: &std::path::Path) {
        self.project_path = path.to_string_lossy().into_owned();
        self.folder_request = Some(folder::Target::Registration);
    }

    pub fn take_folder_request(&mut self) -> Option<folder::Target> {
        self.folder_request.take()
    }

    pub fn folder_path(&self, target: folder::Target) -> &str {
        match target {
            folder::Target::Registration => &self.project_path,
            folder::Target::Task => &self.work.path,
        }
    }

    pub fn accept_folder(&mut self, target: folder::Target, selection: Option<folder::Selection>) {
        let Some(selection) = selection else {
            return;
        };
        self.form_error = None;
        match target {
            folder::Target::Registration => {
                let old_name = std::path::Path::new(&self.project_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if self.project_name.is_empty() || self.project_name == old_name {
                    self.project_name = std::path::Path::new(&selection.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_owned();
                }
                self.project_path = selection.path;
                self.project_path_problem = selection.problem;
                self.registration_open = true;
            }
            folder::Target::Task => {
                self.work.path = selection.path;
                self.work.path_problem = selection.problem;
                self.work.workspace.clear();
            }
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        busy: bool,
        notice: Option<&Notice>,
        can_retry: bool,
    ) -> Vec<Command> {
        if let Some(project) = &self.registration_project {
            if let Some(workspace) = items(&snapshot["workspaces"])
                .iter()
                .find(|w| text(w, "workingDirectory") == self.project_path)
            {
                self.link_choices
                    .insert(project.clone(), text(workspace, "workspaceId").to_owned());
                self.registration_project = None;
            }
        }
        self.decisions.finish_proposal(snapshot, notice, busy);
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new("console-language"),
                Language::from_snapshot(snapshot),
            )
        });
        ui.style_mut().animation_time = 0.0;
        ui.style_mut().scroll_animation = egui::style::ScrollAnimation::none();
        let mut commands = Vec::new();
        egui::Panel::left("console-navigation")
            .resizable(false)
            .default_size(196.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(18, 21, 26))
                    .inner_margin(14),
            )
            .show(ui, |ui| {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 6, MINT);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "P",
                        egui::FontId::proportional(16.0),
                        Color32::from_rgb(18, 37, 29),
                    );
                    ui.label(
                        RichText::new(t(ui, "Patchfleet"))
                            .size(15.0)
                            .strong()
                            .color(TEXT),
                    );
                });
                ui.add_space(26.0);
                ui.label(RichText::new(t(ui, "WORKSPACE")).size(10.0).color(MUTED));
                ui.add_space(8.0);
                let footer_height = if snapshot["desktopPreferences"]["error"] == true {
                    220.0
                } else {
                    138.0
                };
                let nav_height = (ui.available_height() - footer_height).max(32.0);
                egui::ScrollArea::vertical()
                    .id_salt("sidebar-navigation")
                    .max_height(nav_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for tab in Tab::ALL {
                            let count = match tab {
                                Tab::Projects => {
                                    array(&snapshot["projectMemory"]["projects"]).len()
                                }
                                Tab::Memory => 0,
                                Tab::Decisions => array(&snapshot["projectDecisions"]["projects"])
                                    .iter()
                                    .map(|p| p["decisionCount"].as_u64().unwrap_or(0) as usize)
                                    .sum(),
                                Tab::Work => items(&snapshot["work"]).len(),
                                Tab::Teams => items(&snapshot["teams"]).len(),
                                _ => 0,
                            };
                            if navigation(ui, tab, self.tab == tab, count) {
                                self.tab = tab;
                                self.form_error = None;
                            }
                            ui.add_space(2.0);
                        }
                    });
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::bottom_up(egui::Align::LEFT),
                    |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!("Patchfleet {}", text(snapshot, "appVersion")))
                                .size(10.0)
                                .color(MUTED),
                        );
                        ui.horizontal(|ui| {
                            ui.colored_label(MINT, t(ui, "•"));
                            ui.label(RichText::new(t(ui, "Local engine")).size(11.0).color(TEXT));
                        });
                        ui.add_space(7.0);
                        ui.separator();
                        ui.add_space(9.0);
                        language_selector(ui, snapshot, busy, &mut commands);
                    },
                );
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(17, 19, 24))
                    .inner_margin(24),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new(t(ui, self.tab.name()))
                            .size(23.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if matches!(self.tab, Tab::Providers | Tab::Work | Tab::Teams) {
                            action(
                                ui,
                                &mut commands,
                                "Refresh providers",
                                "observe",
                                !busy,
                                || json!({}),
                            );
                        }
                        if busy {
                            ui.label(t(ui, "Applying action…"));
                        }
                    });
                });
                ui.add_space(3.0);
                ui.label(
                    RichText::new(i18n::tr_for_snapshot(snapshot, self.tab.summary()))
                        .size(12.0)
                        .color(MUTED),
                );
                ui.add_space(24.0);
                if let Some(notice) = notice {
                    self.feedback(ui, notice, !busy, can_retry);
                    ui.add_space(14.0);
                }
                if let Some(error) = self.form_error {
                    ui.colored_label(RED, t(ui, error));
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .id_salt(("console-body", self.tab.name()))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width().min(1040.0));
                        ui.add_enabled_ui(!busy, |ui| match self.tab {
                            Tab::Projects => self.projects(ui, snapshot, &mut commands),
                            Tab::Memory => self.project_memory(ui, snapshot, &mut commands),
                            Tab::Decisions => self.decision_panel(ui, snapshot, &mut commands),
                            Tab::Work => self.work_panel(ui, snapshot, &mut commands),
                            Tab::Teams => self.teams(ui, snapshot, &mut commands),
                            Tab::Agents => self.agents(ui, snapshot, &mut commands),
                            Tab::Providers => self.providers(ui, snapshot),
                            Tab::Cloud => self.cloud(ui, snapshot, &mut commands),
                        });
                    });
            });
        if self.registration_open {
            let id = egui::Id::new("register-folder");
            let modal = egui::Modal::new(id)
                .area(egui::Modal::default_area(id).fade_in(false))
                .show(ui.ctx(), |ui| {
                    ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 460.0));
                    ui.add_enabled_ui(!busy, |ui| self.project_form(ui, snapshot, &mut commands));
                });
            if modal.should_close() {
                self.registration_open = false;
            }
        }
        if self.new_project_open {
            let id = egui::Id::new("create-project");
            let modal = egui::Modal::new(id)
                .area(egui::Modal::default_area(id).fade_in(false))
                .show(ui.ctx(), |ui| {
                    ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 460.0));
                    ui.add_enabled_ui(!busy, |ui| self.create_project_form(ui, &mut commands));
                });
            if modal.should_close() {
                self.new_project_open = false;
            }
        }
        commands
    }

    fn feedback(&mut self, ui: &mut Ui, notice: &Notice, ready: bool, can_retry: bool) {
        if notice.kind == NoticeKind::Success {
            egui::Frame::new()
                .fill(MINT.gamma_multiply(0.06))
                .corner_radius(8)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let message = t(ui, &notice.code);
                        ui.label(
                            RichText::new(if message == notice.code {
                                t(ui, "The local action was recorded.")
                            } else {
                                message
                            })
                            .color(MINT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(t(ui, "Dismiss")).clicked() {
                                self.notice_action = Some(NoticeAction::Dismiss);
                            }
                        });
                    });
                });
            return;
        }
        let color = match notice.kind {
            NoticeKind::Success => MINT,
            NoticeKind::Warning => Color32::from_rgb(239, 197, 132),
            NoticeKind::Error => RED,
        };
        egui::Frame::new().fill(color.gamma_multiply(0.07)).stroke(egui::Stroke::new(1.0,color.gamma_multiply(0.3))).corner_radius(8).inner_margin(12).show(ui,|ui| {
            let title = match notice.code.as_str() {
                "WORKSPACE_ACCESS_DENIED" | "WORKSPACE_READ_ONLY" => "Folder access needs attention",
                "WORKSPACE_NOT_GIT_ROOT" => "Select the repository folder",
                _ => match notice.kind { NoticeKind::Success=>"Action completed",NoticeKind::Warning=>"Action was not applied",NoticeKind::Error=>"Action could not be completed" },
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(t(ui,title)).strong().color(color));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.small_button(t(ui,"Dismiss")).clicked() { self.notice_action = Some(NoticeAction::Dismiss); }
                });
            });
            let description = if notice.action.starts_with("project_decision_")&&notice.code=="STALE_SOURCE_REVISION" {t(ui,"Decision sources changed. Create a new draft from current sources before accepting.")} else {t(ui,&notice.code)};
            if description == notice.code {
                hint(ui, if notice.kind == NoticeKind::Success { "The local action was recorded." } else { "Review the current state and try the action again. Details are available below." });
            } else { ui.label(description); }
            if folder::is_problem(&notice.code) {
                if matches!(notice.code.as_str(),"WORKSPACE_ACCESS_DENIED" | "WORKSPACE_READ_ONLY") { hint(ui,folder::permission_guidance(&notice.code)); }
                ui.horizontal_wrapped(|ui| {
                    if matches!(notice.action,"workspace_register" | "work_enqueue") && control(ui,"Choose another folder",ready).clicked() {
                        self.folder_request = Some(if notice.action == "work_enqueue" { folder::Target::Task } else { folder::Target::Registration });
                        self.notice_action = Some(NoticeAction::Dismiss);
                    }
                    #[cfg(target_os="macos")]
                    if notice.code == "WORKSPACE_ACCESS_DENIED" && control(ui,"Open permission settings",true).clicked() { self.notice_action = Some(NoticeAction::Settings); }
                });
            } else if ready && can_retry && notice.retryable() && control(ui,"Retry the same action",true).clicked() { self.notice_action = Some(NoticeAction::Retry); }
            if notice.kind != NoticeKind::Success { ui.collapsing(t(ui,"Technical details"),|ui| { ui.monospace(&notice.code); }); }
        });
    }

    fn projects(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let projects = array(&snapshot["projectMemory"]["projects"]);
        let workspaces = items(&snapshot["workspaces"]);
        if projects.is_empty() {
            ui.label(
                RichText::new(t(ui, "YOUR LOCAL WORKSPACE"))
                    .size(10.0)
                    .color(MINT),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(t(ui, "Create your first project"))
                    .size(28.0)
                    .strong()
                    .color(TEXT),
            );
            hint(ui, "Keep your folders, agents and project memory together.");
            ui.add_space(18.0);
            constrained(ui, 560.0, |ui| self.create_project_form(ui, commands));
        } else {
            ui.horizontal(|ui| {
                hint(
                    ui,
                    &localized_format!(
                        ui,
                        "{0} projects · {1} local folders",
                        projects.len(),
                        workspaces.len()
                    ),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if control(ui, "New project", true).clicked() {
                        self.new_project_open = true;
                        self.form_error = None;
                    }
                });
            });
            ui.add_space(14.0);
            for project in projects {
                ui.push_id(text(project,"projectId"), |ui| card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 8, Color32::from_rgb(31, 53, 44));
                        draw_icon(ui.painter(), rect.shrink(8.0), Tab::Projects, MINT);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(text(project,"name")).size(18.0).strong().color(TEXT));
                            hint(ui, &localized_format!(ui,"{0} folders · {1} sources",array(&project["workspaceIds"]).len(),project["sourceCount"].as_u64().unwrap_or(0)));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if control(ui,"Open memory",true).clicked() { self.memory.select(text(project,"projectId")); self.tab = Tab::Memory; }
                        });
                    });
                    ui.add_space(14.0);
                    if array(&project["workspaceIds"]).is_empty() {
                        egui::Frame::new().fill(Color32::from_rgb(18,22,28)).corner_radius(8).inner_margin(12).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.label(RichText::new(t(ui,"Connect a local folder")).strong().color(TEXT));
                            hint(ui,"Choose the Git repository where your agents will work.");
                            ui.add_space(8.0);
                            if submit(ui,"Choose folder","folder_picker") {
                                self.registration_project = Some(text(project,"projectId").to_owned());
                                self.folder_request = Some(folder::Target::Registration);
                                self.form_error = None;
                            }
                        });
                    } else {
                        for id in array(&project["workspaceIds"]) {
                            let workspace = workspaces.iter().find(|w|w["workspaceId"] == *id);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(workspace.map(|w|std::borrow::Cow::Borrowed(text(w,"displayName"))).unwrap_or_else(||t(ui,"Removed folder")));
                                action(ui,commands,"Unlink","project_unlink_workspace",revision(project)>0,||json!({"projectId":project["projectId"],"workspaceId":id,"expectedProjectRevision":project["revision"]}));
                            });
                            if let Some(workspace) = workspace { path_label(ui,text(workspace,"workingDirectory")); }
                            ui.add_space(5.0);
                        }
                        if control(ui,"Add another folder",true).clicked() {
                            self.registration_project = Some(text(project,"projectId").to_owned());
                            self.folder_request = Some(folder::Target::Registration);
                        }
                    }
                    let available: Vec<_> = workspaces.iter().filter(|w|!projects.iter().any(|p|array(&p["workspaceIds"]).contains(&w["workspaceId"]))).collect();
                    if !available.is_empty() {
                        ui.add_space(12.0);
                        let selected = self.link_choices.entry(text(project,"projectId").to_owned()).or_default();
                        if selected.is_empty() && available.len() == 1 { *selected = text(available[0],"workspaceId").to_owned(); }
                        choose_refs(ui,"link-worktree","Link an existing folder",selected,&available,("workspaceId","displayName"),true);
                        let workspace = available.iter().find(|w|text(w,"workspaceId") == selected);
                        if let Some(workspace) = workspace { path_label(ui,text(workspace,"workingDirectory")); }
                        action(ui,commands,"Link folder","project_link_workspace",workspace.is_some()&&revision(project)>0,||json!({"projectId":project["projectId"],"workspaceId":selected,"expectedProjectRevision":project["revision"]}));
                    }
                    ui.add_space(8.0);
                    ui.collapsing(t(ui,"Project details"),|ui| {
                        ui.monospace(text(project,"projectId"));
                        hint(ui,&localized_format!(ui,"Revision {0} · created {1}",revision(project),format_time(ui,text(project,"createdAt"))));
                    });
                }));
                ui.add_space(12.0);
            }
        }
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t(ui, "LOCAL FOLDERS"))
                    .size(10.0)
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if control(ui, "Add folder", true).clicked() {
                    self.registration_project = None;
                    self.folder_request = Some(folder::Target::Registration);
                    self.form_error = None;
                }
            });
        });
        if workspaces.is_empty() {
            hint(
                ui,
                "No folders added yet. Choose a repository using the system folder picker.",
            );
        } else {
            for workspace in workspaces {
                ui.push_id(text(workspace,"workspaceId"),|ui| {
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(text(workspace,"displayName")).strong().color(TEXT));
                        if let Some(project) = projects.iter().find(|p|array(&p["workspaceIds"]).contains(&workspace["workspaceId"])) { raw_status(ui,text(project,"name")); }
                        else { status(ui,"Unlinked"); }
                        action(ui,commands,"Remove folder","workspace_remove",revision(workspace)>0,||json!({"workspaceId":workspace["workspaceId"],"expectedWorkspaceRevision":workspace["revision"]}));
                    });
                    path_label(ui,text(workspace,"workingDirectory"));
                });
            }
        }
    }

    fn create_project_form(&mut self, ui: &mut Ui, commands: &mut Vec<Command>) {
        card(ui, |ui| {
            section_title(
                ui,
                "Project",
                "A stable identity independent of repository paths.",
            );
            ui.add_space(14.0);
            field_hint(
                ui,
                "Project name",
                &mut self.new_project_name,
                80,
                "e.g. Patchfleet, Website, Mobile app",
            );
            if submit(ui, "Create project", "project_create") {
                match required(
                    &self.new_project_name,
                    80,
                    "Enter a project name of up to 80 characters.",
                ) {
                    Ok(()) => {
                        commands.push(Command::new(
                            "project_create",
                            json!({"name":self.new_project_name.trim()}),
                        ));
                        self.form_error = None;
                        self.new_project_open = false;
                    }
                    Err(error) => self.form_error = Some(error),
                }
            }
            hint(ui,"Link worktrees after creating the project. Existing folders are not grouped automatically.");
        });
    }

    fn project_form(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        section_title(
            ui,
            "Add a local folder",
            "Your agents work here. Files stay on this device.",
        );
        ui.add_space(16.0);
        let existing = items(&snapshot["workspaces"])
            .iter()
            .find(|w| text(w, "workingDirectory") == self.project_path);
        egui::ScrollArea::vertical().id_salt("folder-setup-body").max_height((ui.ctx().viewport_rect().height()-180.0).max(150.0)).show(ui, |ui| {
        folder_preview(ui, &self.project_path);
        if control(ui, "Change folder", true).clicked() {
            self.folder_request = Some(folder::Target::Registration);
        }
        if let Some(code) = &self.project_path_problem {
            ui.add_space(12.0);
            ui.colored_label(RED, t(ui, code));
            if matches!(
                code.as_str(),
                "WORKSPACE_ACCESS_DENIED" | "WORKSPACE_READ_ONLY"
            ) {
                hint(ui, folder::permission_guidance(code));
                #[cfg(target_os = "macos")]
                if code == "WORKSPACE_ACCESS_DENIED" && control(ui, "Open permission settings", true).clicked() {
                    self.notice_action = Some(NoticeAction::Settings);
                }
            }
        } else {
            ui.add_space(8.0);
            hint(
                ui,
                "Git folder found. Access will be checked again when you add it.",
            );
        }
        if let Some(existing) = existing {
            if let Some(project) = array(&snapshot["projectMemory"]["projects"])
                .iter()
                .find(|p| array(&p["workspaceIds"]).contains(&existing["workspaceId"]))
            {
                hint(ui,&localized_format!(ui,"This folder is linked to {0}. Choose another folder or unlink it there first.",text(project,"name")));
            } else {
                hint(
                    ui,
                    "This folder is already added. You can link it from the project card.",
                );
            }
        }
        ui.add_space(16.0);
        field_hint(
            ui,
            "Folder label · optional",
            &mut self.project_name,
            160,
            "e.g. Website, API, Mobile app",
        );
        if self.registration_project.is_some() {
            hint(
                ui,
                "After adding this folder, link it from the project card.",
            );
        }
        if let Some(error) = self.form_error {
            ui.colored_label(RED, t(ui, error));
        }
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let allowed = !self.project_path.is_empty() && self.project_path_problem.is_none() && existing.is_none();
            let clicked = ui.add_enabled_ui(allowed,|ui|submit(ui,"Add folder","workspace_register")).inner;
            if clicked {
                if utf16_len(self.project_name.trim()) > 160 { self.form_error = Some("The folder label is limited to 160 characters."); }
                else {
                    commands.push(Command::new("workspace_register",json!({"workingDirectory":self.project_path,"displayName":self.project_name.trim()})));
                    self.form_error = None;
                    self.registration_open = false;
                }
            }
            if control(ui,"Cancel",true).clicked() { self.registration_open = false; }
        });
    }

    fn project_memory(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let projects = array(&snapshot["projectMemory"]["projects"]);
        if projects.is_empty() {
            empty(
                ui,
                "Create a project first",
                "Project memory keeps owner sources together across explicitly linked worktrees.",
            );
            return;
        }
        ui.horizontal_wrapped(|ui| {
            status(ui, "Source context");
            hint(
                ui,
                "ADR extraction comes next. These records are not accepted decisions.",
            );
        });
        ui.add_space(16.0);
        if self.memory.project.is_empty() && projects.len() == 1 {
            self.memory.select(text(&projects[0], "projectId"));
        }
        let previous = self.memory.project.clone();
        choose(
            ui,
            "memory-project",
            "Project",
            &mut self.memory.project,
            projects,
            ("projectId", "name"),
            true,
        );
        if self.memory.project != previous {
            let selected = self.memory.project.clone();
            self.memory.project = previous;
            self.memory.select(&selected);
        }
        let Some(project) = projects
            .iter()
            .find(|p| text(p, "projectId") == self.memory.project)
        else {
            hint(
                ui,
                "Choose a project to read or write its local source context.",
            );
            return;
        };
        let page = &snapshot["memoryPage"];
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(t(ui, "Keyword filter"))
                    .size(11.0)
                    .color(MUTED),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.memory.keyword_input)
                    .desired_width(230.0)
                    .font(egui::TextStyle::Body)
                    .margin(egui::Margin::symmetric(10, 7))
                    .char_limit(160)
                    .hint_text(t(ui, "Words in source titles or text")),
            );
            if control(ui, "Apply keyword", true).clicked() {
                if utf16_len(&self.memory.keyword_input) <= 160 {
                    self.memory.query = self.memory.keyword_input.trim().to_owned();
                    self.memory.offset = 0;
                    self.memory.requested = None;
                    self.memory.refresh_requested = true;
                    self.form_error = None;
                } else {
                    self.form_error = Some("Keyword filter is limited to 160 characters.");
                }
            }
            if control(ui, "Refresh memory", true).clicked() {
                self.memory.requested = None;
                self.memory.refresh_requested = true;
            }
        });
        ui.add_space(18.0);
        let page_current = self.memory.page_matches(page, project);
        if let Some(edit) = self.memory.edit.as_mut() {
            let current = page_current
                && self.memory.project == edit.project
                && text(project, "projectId") == edit.project
                && page["projectId"] == edit.project
                && revision(page) == revision(project)
                && array(&page["sources"]).iter().any(|s| {
                    text(s, "sourceId") == edit.source
                        && text(s, "projectId") == edit.project
                        && text(s, "kind") == "message"
                        && revision(s) == edit.revision
                });
            let mut cancel = false;
            constrained(ui, 700.0, |ui| {
                card(ui, |ui| {
                    section_title(
                        ui,
                        "Edit project message",
                        "Captured task and team sources remain immutable.",
                    );
                    ui.add_space(12.0);
                    field(ui, "Title", &mut edit.title, 160, false);
                    multiline(ui, "Source text", &mut edit.body, 50_000, 4);
                    let valid = current
                        && required(&edit.title, 160, "").is_ok()
                        && required(&edit.body, 50_000, "").is_ok();
                    ui.horizontal(|ui| {
                    action(ui,commands,"Save message","project_source_edit",valid,||json!({"projectId":edit.project,"sourceId":edit.source,"title":edit.title.trim(),"text":edit.body.trim(),"expectedSourceRevision":edit.revision}));
                    if control(ui,"Cancel edit",true).clicked(){cancel=true;}
                });
                    if !current {
                        hint(ui,"This source is no longer current on this page. Cancel and reopen it before saving.");
                    }
                })
            });
            if cancel {
                self.memory.edit = None;
            }
        } else {
            egui::CollapsingHeader::new(t(ui, "Write a project message")).default_open(project["sourceCount"]==0).show(ui,|ui|constrained(ui,700.0,|ui|card(ui,|ui| {
                field(ui,"Message title",&mut self.memory.title,160,false);multiline(ui,"Source text",&mut self.memory.body,50_000,4);
                if submit(ui,"Add message","project_message"){match self.memory.message_payload(project){Ok(payload)=>{commands.push(Command::new("project_message",payload));self.form_error=None;},Err(error)=>self.form_error=Some(error)}}
                hint(ui,"Owner-authored context stays local. Adding it does not accept an ADR.");
            })));
        }
        ui.add_space(18.0);
        if self.memory.page_matches(page, project) {
            let total = page["total"].as_u64().unwrap_or(0);
            ui.horizontal_wrapped(|ui| {
                ui.label(tf(
                    ui,
                    if self.memory.query.is_empty() {
                        "{0} sources"
                    } else {
                        "{0} sources matching keyword"
                    },
                    &[&total.to_string()],
                ));
                if control(ui, "Previous sources", self.memory.offset > 0).clicked() {
                    self.memory.offset = self.memory.offset.saturating_sub(10);
                    self.memory.requested = None;
                }
                ui.label(localized_format!(
                    ui,
                    "Page {0}",
                    self.memory.offset / 10 + 1
                ));
                if control(
                    ui,
                    "Next sources",
                    self.memory.offset.saturating_add(10) < total,
                )
                .clicked()
                {
                    self.memory.offset = self.memory.offset.saturating_add(10);
                    self.memory.requested = None;
                }
            });
            ui.add_space(10.0);
            if self.memory.page_matches(page, project) && array(&page["sources"]).is_empty() {
                empty(
                    ui,
                    if self.memory.query.is_empty() {
                        "No sources yet"
                    } else {
                        "No keyword matches"
                    },
                    "Write a project message or submit work through a linked worktree.",
                );
            }
            for source in
                array(&page["sources"])
                    .iter()
                    .take(if self.memory.page_matches(page, project) {
                        10
                    } else {
                        0
                    })
            {
                ui.push_id(text(source, "sourceId"), |ui| {
                    card(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(text(source, "title"))
                                    .size(14.0)
                                    .strong()
                                    .color(TEXT),
                            );
                            status(ui, source_kind_label(text(source, "kind")));
                            hint(ui, &localized_format!(ui, "Revision {0}", revision(source)));
                        });
                        let preview: String = text(source, "text").chars().take(240).collect();
                        ui.label(preview);
                        ui.collapsing(t(ui, "Full source and provenance"), |ui| {
                            ui.label(text(source, "text"));
                            ui.separator();
                            ui.monospace(text(source, "sourceId"));
                            hint(
                                ui,
                                &localized_format!(
                                    ui,
                                    "Author {0} · updated {1}",
                                    text(source, "actorId"),
                                    format_time(ui, text(source, "updatedAt"))
                                ),
                            );
                            hint(
                                ui,
                                &localized_format!(
                                    ui,
                                    "Origin {0} · {1} · {2}",
                                    text(&source["origin"], "type"),
                                    text(&source["origin"], "id"),
                                    text(&source["origin"], "eventId")
                                ),
                            );
                        });
                        if text(source, "kind") == "message"
                            && text(source, "projectId") == self.memory.project
                        {
                            if control(ui, "Edit message", true).clicked() {
                                self.memory.edit = Some(SourceEdit {
                                    project: self.memory.project.clone(),
                                    source: text(source, "sourceId").to_owned(),
                                    revision: revision(source),
                                    title: text(source, "title").to_owned(),
                                    body: text(source, "text").to_owned(),
                                });
                            }
                        } else {
                            hint(ui, "Captured source · read only");
                        }
                    })
                });
                ui.add_space(10.0);
            }
        } else {
            hint(ui,"The current source page is unavailable. Use Refresh memory to retry a failed read.");
        }
        if commands.is_empty() {
            if let Some(command) = self.memory.next_read(project, page, !ui.is_enabled()) {
                commands.push(command);
            }
        }
    }

    fn decision_panel(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let projects = array(&snapshot["projectMemory"]["projects"]);
        if projects.is_empty() {
            empty(
                ui,
                "Create a project first",
                "Decisions belong to a project and cite its local sources.",
            );
            return;
        }
        if self.decisions.project.is_empty() && projects.len() == 1 {
            self.decisions.select(text(&projects[0], "projectId"));
        }
        let previous = self.decisions.project.clone();
        choose(
            ui,
            "decision-project",
            "Project",
            &mut self.decisions.project,
            projects,
            ("projectId", "name"),
            true,
        );
        if self.decisions.project != previous {
            let selected = self.decisions.project.clone();
            self.decisions.project = previous;
            self.decisions.select(&selected);
        }
        let Some(project) = projects
            .iter()
            .find(|p| text(p, "projectId") == self.decisions.project)
        else {
            hint(ui, "Choose a project to review its decisions.");
            return;
        };
        self.decisions.verify_citation_epoch(project);
        let manifest = decision_manifest(snapshot, &self.decisions.project);
        let page = &snapshot["decisionPage"];
        let detail = &snapshot["decisionDetail"];
        if self.decisions.selected.is_empty() && self.decisions.page_matches(page, manifest) {
            self.decisions.selected = if text(detail, "projectId") == self.decisions.project
                && revision(detail) == manifest
                && text(detail, "reasonCode") == "PROJECT_DECISION_READ"
            {
                text(&detail["decision"], "decisionId").to_owned()
            } else {
                array(&page["decisions"])
                    .first()
                    .map(|d| text(d, "decisionId").to_owned())
                    .unwrap_or_default()
            };
        }
        ui.horizontal_wrapped(|ui| {
            status(ui, "Free · Local");
            let counts = array(&snapshot["projectDecisions"]["projects"])
                .iter()
                .find(|p| text(p, "projectId") == self.decisions.project);
            hint(
                ui,
                &localized_format!(
                    ui,
                    "{0} accepted · {1} proposed",
                    counts
                        .and_then(|c| c["acceptedCount"].as_u64())
                        .unwrap_or(0),
                    counts
                        .and_then(|c| c["proposedCount"].as_u64())
                        .unwrap_or(0)
                ),
            );
        });
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            if control(ui, "New decision draft", !self.decisions.composer_open).clicked() {
                self.decisions.composer_open = true;
            }
            if control(ui, "Refresh decisions", true).clicked() {
                self.decisions.refresh();
            }
            if control(ui, "Open project sources", true).clicked() {
                self.memory.select(&self.decisions.project);
                self.tab = Tab::Memory;
            }
        });
        ui.add_space(14.0);
        if self.decisions.composer_open {
            constrained(ui, 860.0, |ui| {
                self.decision_composer(ui, project, &snapshot["memoryPage"], manifest, commands)
            });
            ui.add_space(20.0);
        }
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("decision-status")
                .selected_text(t(ui, decision_status_label(&self.decisions.status)))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for value in ["all", "proposed", "accepted", "rejected", "superseded"] {
                        if ui
                            .selectable_value(
                                &mut self.decisions.status,
                                value.to_owned(),
                                t(ui, decision_status_label(value)),
                            )
                            .changed()
                        {
                            self.decisions.offset = 0;
                            self.decisions.refresh_list = true;
                        }
                    }
                });
            ui.add(
                egui::TextEdit::singleline(&mut self.decisions.query_input)
                    .font(egui::TextStyle::Body)
                    .margin(egui::Margin::symmetric(10, 7))
                    .desired_width(230.0)
                    .char_limit(160)
                    .hint_text(t(ui, "Search decision text")),
            );
            if control(ui, "Apply decision filter", true).clicked() {
                if utf16_len(self.decisions.query_input.trim()) <= 160 {
                    self.decisions.query = self.decisions.query_input.trim().to_owned();
                    self.decisions.offset = 0;
                    self.decisions.refresh_list = true;
                    self.form_error = None;
                } else {
                    self.form_error = Some("Keyword filter is limited to 160 characters.");
                }
            }
        });
        ui.add_space(16.0);
        if ui.available_width() >= 850.0 {
            ui.columns(2, |columns| {
                self.decision_list(&mut columns[0], page, manifest);
                self.decision_detail(&mut columns[1], detail, page, manifest, commands);
            });
        } else {
            self.decision_detail(ui, detail, page, manifest, commands);
            ui.add_space(16.0);
            self.decision_list(ui, page, manifest);
        }
        if commands.is_empty() {
            let busy = !ui.is_enabled();
            let read = self
                .decisions
                .next_list(page, manifest, busy)
                .or_else(|| self.decisions.next_detail(detail, manifest, busy))
                .or_else(|| {
                    if self.decisions.composer_open {
                        self.decisions
                            .sources
                            .next_read(project, &snapshot["memoryPage"], busy)
                    } else {
                        None
                    }
                });
            if let Some(read) = read {
                commands.push(read);
            }
        }
    }

    fn decision_composer(
        &mut self,
        ui: &mut Ui,
        project: &Value,
        page: &Value,
        manifest: u64,
        commands: &mut Vec<Command>,
    ) {
        card(ui, |ui| {
            section_title(
                ui,
                "New decision draft",
                "Saved drafts are immutable. A changed decision starts a new draft.",
            );
            ui.add_space(12.0);
            field(ui, "Decision title", &mut self.decisions.title, 160, false);
            multiline(
                ui,
                "Decision statement",
                &mut self.decisions.decision,
                20_000,
                3,
            );
            egui::CollapsingHeader::new(t(ui, "Context, rationale and consequences"))
                .id_salt("decision-context-fields")
                .show(ui, |ui| {
                    multiline(
                        ui,
                        "Context (optional)",
                        &mut self.decisions.context,
                        10_000,
                        2,
                    );
                    multiline(
                        ui,
                        "Rationale (optional)",
                        &mut self.decisions.rationale,
                        10_000,
                        2,
                    );
                    multiline(
                        ui,
                        "Consequences (optional)",
                        &mut self.decisions.consequences,
                        10_000,
                        2,
                    );
                });
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);
            ui.label(
                RichText::new(t(ui, "Cited project sources"))
                    .strong()
                    .color(TEXT),
            );
            hint(ui,"Select one to eight source versions. Their original text remains attached to the decision.");
            if self.decisions.sources_changed {
                ui.colored_label(
                    RED,
                    t(
                        ui,
                        "Sources changed. Select current evidence again before saving.",
                    ),
                );
            }
            let command_count = commands.len();
            ui.horizontal_wrapped(|ui| {
                ui.label(localized_format!(
                    ui,
                    "{0}/8 sources selected",
                    self.decisions.citations.len()
                ));
                if control(ui, "Refresh draft sources", true).clicked() {
                    self.decisions.sources.refresh_requested = true;
                }
            });
            if self.decisions.sources.page_matches(page, project) {
                for source in array(&page["sources"])
                    .iter()
                    .filter(|s| text(s, "projectId") == self.decisions.project && revision(s) > 0)
                    .take(10)
                {
                    ui.push_id(text(source, "sourceId"), |ui| {
                        let position = self
                            .decisions
                            .citations
                            .iter()
                            .position(|c| c.source == text(source, "sourceId"));
                        let mut selected = position.is_some();
                        let response = ui.add_enabled(
                            selected || self.decisions.citations.len() < 8,
                            egui::Checkbox::new(&mut selected, text(source, "title")),
                        );
                        #[cfg(test)]
                        ui.data_mut(|d| {
                            d.insert_temp(
                                egui::Id::new((
                                    "test-action",
                                    "decision-citation",
                                    text(source, "sourceId"),
                                )),
                                response.rect,
                            )
                        });
                        if response.changed() {
                            if selected {
                                self.decisions.citations.push(Citation {
                                    source: text(source, "sourceId").to_owned(),
                                    revision: revision(source),
                                    title: text(source, "title").to_owned(),
                                });
                                self.decisions.sources_changed = false;
                            } else if let Some(position) = position {
                                self.decisions.citations.remove(position);
                            }
                        }
                        hint(ui, &localized_format!(ui, "Revision {0}", revision(source)));
                    });
                }
                let total = page["total"].as_u64().unwrap_or(0);
                ui.horizontal_wrapped(|ui| {
                    if control(
                        ui,
                        "Previous draft sources",
                        self.decisions.sources.offset > 0,
                    )
                    .clicked()
                    {
                        self.decisions.sources.offset =
                            self.decisions.sources.offset.saturating_sub(10);
                    }
                    ui.label(localized_format!(
                        ui,
                        "Page {0}",
                        self.decisions.sources.offset / 10 + 1
                    ));
                    if control(
                        ui,
                        "Next draft sources",
                        self.decisions.sources.offset.saturating_add(10) < total,
                    )
                    .clicked()
                    {
                        self.decisions.sources.offset =
                            self.decisions.sources.offset.saturating_add(10);
                    }
                });
                if total == 0 {
                    hint(
                        ui,
                        "Add a project message in Memory before creating a sourced decision.",
                    );
                }
            } else {
                hint(ui,"Read the current project sources before selecting evidence. Refresh retries a failed read.");
            }
            if !self.decisions.citations.is_empty() {
                ui.collapsing(t(ui, "Selected evidence"), |ui| {
                    for citation in &self.decisions.citations {
                        ui.label(&citation.title);
                        hint(
                            ui,
                            &localized_format!(ui, "Revision {0}", citation.revision),
                        );
                    }
                });
            }
            ui.add_space(14.0);
            let payload = self.decisions.proposal(project);
            let ready = self.decisions.sources.page_matches(page, project) && payload.is_ok();
            ui.horizontal_wrapped(|ui| {
                action(
                    ui,
                    commands,
                    "Save decision draft",
                    "project_decision_propose",
                    ready,
                    || payload.unwrap(),
                );
                if control(ui, "Close draft", true).clicked() {
                    self.decisions.composer_open = false;
                }
            });
            if commands.len() > command_count
                && commands
                    .last()
                    .is_some_and(|c| c.action == "project_decision_propose")
            {
                self.decisions.pending_proposal_manifest = Some(manifest);
            }
            if let Err(message) = self.decisions.proposal(project) {
                hint(ui, message);
            }
        });
    }

    fn decision_list(&mut self, ui: &mut Ui, page: &Value, manifest: u64) {
        if !self.decisions.page_matches(page, manifest) {
            empty(
                ui,
                "Decision list",
                if self.decisions.list_requested.is_none() || !ui.is_enabled() {
                    "Loading local decisions…"
                } else {
                    "The decision list is unavailable. Refresh decisions to retry."
                },
            );
            return;
        }
        let total = page["total"].as_u64().unwrap_or(0);
        ui.horizontal_wrapped(|ui| {
            ui.label(localized_format!(ui, "{0} decisions", total));
            if control(ui, "Previous decisions", self.decisions.offset > 0).clicked() {
                self.decisions.offset = self.decisions.offset.saturating_sub(10);
            }
            if control(
                ui,
                "Next decisions",
                self.decisions.offset.saturating_add(10) < total,
            )
            .clicked()
            {
                self.decisions.offset = self.decisions.offset.saturating_add(10);
            }
        });
        ui.add_space(10.0);
        if !self.decisions.page_matches(page, manifest) {
            return;
        }
        if total == 0 {
            empty(
                ui,
                "No decisions in this view",
                "Create a sourced draft or change the status and keyword filters.",
            );
        }
        for decision in array(&page["decisions"])
            .iter()
            .filter(|d| text(d, "projectId") == self.decisions.project)
            .take(10)
        {
            ui.push_id(text(decision, "decisionId"), |ui| {
                let selected = self.decisions.selected == text(decision, "decisionId");
                egui::Frame::new()
                    .fill(if selected {
                        Color32::from_rgb(25, 36, 33)
                    } else {
                        SURFACE
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if selected {
                            MINT.gamma_multiply(0.5)
                        } else {
                            BORDER
                        },
                    ))
                    .corner_radius(8)
                    .inner_margin(14)
                    .show(ui, |ui| {
                        ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                        ui.label(
                            RichText::new(text(decision, "title"))
                                .size(14.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.horizontal_wrapped(|ui| {
                            status(ui, text(decision, "status"));
                            hint(
                                ui,
                                &localized_format!(
                                    ui,
                                    "{0} sources",
                                    decision["sourceCount"].as_u64().unwrap_or(0)
                                ),
                            );
                        });
                        if control(ui, "Open decision", true).clicked() {
                            self.decisions.selected = text(decision, "decisionId").to_owned();
                            self.decisions.replacement.clear();
                        }
                    });
            });
            ui.add_space(8.0);
        }
    }

    fn decision_detail(
        &mut self,
        ui: &mut Ui,
        detail: &Value,
        page: &Value,
        manifest: u64,
        commands: &mut Vec<Command>,
    ) {
        if self.decisions.selected.is_empty() {
            empty(
                ui,
                "Decision details",
                "Open a decision to inspect its evidence and history.",
            );
            return;
        }
        if !self.decisions.detail_matches(detail, manifest) {
            empty(
                ui,
                "Decision details",
                if self.decisions.detail_requested.is_none() || !ui.is_enabled() {
                    "Loading decision details…"
                } else {
                    "Decision details are unavailable. Refresh decisions to retry."
                },
            );
            return;
        }
        let decision = &detail["decision"];
        card(ui, |ui| {
            ui.label(
                RichText::new(text(decision, "title"))
                    .size(19.0)
                    .strong()
                    .color(TEXT),
            );
            ui.horizontal_wrapped(|ui| {
                status(ui, text(decision, "status"));
                hint(
                    ui,
                    &localized_format!(ui, "Revision {0}", revision(decision)),
                );
            });
            ui.add_space(12.0);
            match text(decision, "status") {
                "proposed" => {
                    hint(
                        ui,
                        "Accept makes this decision active. Reject preserves its history.",
                    );
                    ui.horizontal_wrapped(|ui| {
                        action(
                            ui,
                            commands,
                            "Accept decision",
                            "project_decision_accept",
                            true,
                            || decision_target(decision),
                        );
                        action(
                            ui,
                            commands,
                            "Reject decision",
                            "project_decision_reject",
                            true,
                            || decision_target(decision),
                        );
                    });
                }
                "accepted" => {
                    hint(ui,"This is an active decision. Replace it only with an explicitly selected draft.");
                    let candidates: Vec<_> = if self.decisions.page_matches(page, manifest) {
                        array(&page["decisions"])
                            .iter()
                            .filter(|d| {
                                text(d, "projectId") == self.decisions.project
                                    && text(d, "status") == "proposed"
                                    && revision(d) > 0
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    if candidates.is_empty() {
                        if control(ui, "Browse proposed replacements", true).clicked() {
                            self.decisions.status = "proposed".into();
                            self.decisions.query.clear();
                            self.decisions.query_input.clear();
                            self.decisions.offset = 0;
                            self.decisions.refresh_list = true;
                        }
                    } else {
                        choose_refs(
                            ui,
                            "decision-replacement",
                            "Replacement draft",
                            &mut self.decisions.replacement,
                            &candidates,
                            ("decisionId", "title"),
                            true,
                        );
                        let replacement = candidates
                            .iter()
                            .find(|d| text(d, "decisionId") == self.decisions.replacement);
                        hint(ui,"This accepts the selected draft and supersedes this decision together.");
                        action(
                            ui,
                            commands,
                            "Supersede decision",
                            "project_decision_supersede",
                            replacement.is_some(),
                            || {
                                let mut payload = decision_target(decision);
                                let replacement = replacement.unwrap();
                                payload["replacementDecisionId"] =
                                    replacement["decisionId"].clone();
                                payload["expectedReplacementRevision"] =
                                    replacement["revision"].clone();
                                payload
                            },
                        );
                    }
                }
                _ => hint(
                    ui,
                    "This decision is historical. Its content and evidence remain available.",
                ),
            }
            for (key, label) in [
                ("decision", "Decision statement"),
                ("context", "Context"),
                ("rationale", "Rationale"),
                ("consequences", "Consequences"),
            ] {
                if !text(decision, key).is_empty() {
                    ui.label(RichText::new(t(ui, label)).size(11.0).color(MUTED));
                    ui.label(text(decision, key));
                    ui.add_space(10.0);
                }
            }
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);
            ui.label(
                RichText::new(t(ui, "Original cited evidence"))
                    .strong()
                    .color(TEXT),
            );
            hint(
                ui,
                "Evidence preserves the cited version, even after its source is edited.",
            );
            for source in array(&detail["sources"])
                .iter()
                .filter(|s| text(s, "projectId") == self.decisions.project)
                .take(8)
            {
                ui.push_id(text(source, "sourceId"), |ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(text(source, "title")).strong().color(TEXT));
                    ui.horizontal_wrapped(|ui| {
                        status(ui, source_kind_label(text(source, "kind")));
                        hint(ui, &localized_format!(ui, "Revision {0}", revision(source)));
                    });
                    ui.label(text(source, "text").chars().take(180).collect::<String>());
                    ui.collapsing(t(ui, "Full source and provenance"), |ui| {
                        ui.label(text(source, "text"));
                        ui.monospace(text(source, "sourceId"));
                        raw_hint(
                            ui,
                            &localized_format!(
                                ui,
                                "Origin {0} · {1} · {2}",
                                text(&source["origin"], "type"),
                                text(&source["origin"], "id"),
                                text(&source["origin"], "eventId")
                            ),
                        );
                    });
                });
            }
            ui.add_space(14.0);
            egui::CollapsingHeader::new(t(ui, "Decision history"))
                .id_salt("decision-history")
                .default_open(true)
                .show(ui, |ui| {
                    for entry in array(&detail["history"]) {
                        ui.horizontal_wrapped(|ui| {
                            status(ui, text(entry, "status"));
                            hint(ui, &localized_format!(ui, "Revision {0}", revision(entry)));
                            raw_hint(ui, text(entry, "actorId"));
                            hint(ui, &format_time(ui, text(entry, "at")));
                        });
                    }
                });
            ui.collapsing(t(ui, "Decision identity and links"), |ui| {
                ui.monospace(text(decision, "decisionId"));
                raw_hint(ui, text(decision, "actorId"));
                for (key, label) in [
                    ("supersedesDecisionId", "Supersedes"),
                    ("supersededByDecisionId", "Replaced by"),
                ] {
                    if let Some(id) = decision[key].as_str() {
                        ui.label(t(ui, label));
                        ui.monospace(id);
                    }
                }
            });
        });
    }

    fn work_panel(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        ui.horizontal_wrapped(|ui| {
            status(
                ui,
                if codex_ready(snapshot) {
                    "Codex ready"
                } else {
                    "Codex not ready"
                },
            );
            hint(ui, "Runs stay local. Every action leaves a receipt.");
        });
        ui.add_space(18.0);
        if ui.available_width() >= 780.0 {
            ui.columns(2, |columns| {
                self.work_list(&mut columns[0], snapshot, commands);
                self.work_composer(&mut columns[1], snapshot, commands);
            });
        } else {
            self.work_composer(ui, snapshot, commands);
            ui.add_space(18.0);
            self.work_list(ui, snapshot, commands);
        }
    }
    fn work_list(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let ready = codex_ready(snapshot);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t(ui, "TASK QUEUE")).size(10.0).color(MUTED));
            ui.label(
                RichText::new(items(&snapshot["work"]).len().to_string())
                    .size(11.0)
                    .color(TEXT),
            );
        });
        ui.add_space(10.0);
        let work = &snapshot["work"];
        let list = items(work);
        if list.is_empty() {
            empty(
                ui,
                "Nothing in the queue",
                "Create a task and it will appear here before an agent starts.",
            );
        }
        let page_count = list.len().div_ceil(25).max(1);
        self.work_page = self.work_page.min(page_count - 1);
        if page_count > 1 {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.work_page > 0, egui::Button::new(t(ui, "Previous")))
                    .clicked()
                {
                    self.work_page -= 1;
                }
                ui.label(localized_format!(
                    ui,
                    "Page {0} of {1} · {2} tasks",
                    self.work_page + 1,
                    page_count,
                    list.len()
                ));
                if ui
                    .add_enabled(
                        self.work_page + 1 < page_count,
                        egui::Button::new(t(ui, "Next")),
                    )
                    .clicked()
                {
                    self.work_page += 1;
                }
            });
        }
        for item in list.iter().skip(self.work_page * 25).take(25) {
            ui.push_id(text(item,"workItemId"), |ui| card(ui, |ui| {
                ui.horizontal_wrapped(|ui| {ui.strong(text(item,"title"));status(ui,text(item,"status"));});
                let workspace = items(&snapshot["workspaces"]).iter().find(|w| w["workingDirectory"]==item["workingDirectory"]);
                if let Some(workspace)=workspace {raw_hint(ui,text(workspace,"displayName"));} else {hint(ui,"Unregistered worktree");}
                let run = array(&work["runs"]).iter().rev().find(|r|r["workItemId"]==item["workItemId"]);
                ui.collapsing(t(ui, "Local task details"),|ui| {ui.monospace(text(item,"workingDirectory"));ui.label(text(item,"instruction"));if let Some(run)=run {ui.monospace(localized_format!(ui, "Run {0} · turn {1}",text(run,"runId"),text(run,"providerTurnId")));}});
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if text(item,"status")=="queued" {
                        action(ui,commands,"Start Codex","work_start",ready&&revision(item)>0,||json!({"workItemId":item["workItemId"],"expectedItemRevision":item["revision"]}));
                        action(ui,commands,"Remove","work_remove",revision(item)>0,||json!({"workItemId":item["workItemId"],"expectedItemRevision":item["revision"]}));
                    }
                    if matches!(text(item,"status"),"failed"|"interrupted"|"blocked") {
                        action(ui,commands,"Queue retry","work_retry",revision(item)>0,||json!({"workItemId":item["workItemId"],"expectedItemRevision":item["revision"]}));
                    }
                    if let Some(run)=run.filter(|r|text(r,"status")=="running") {
                        let owned=run["ownerEpoch"]==snapshot["hostEpoch"];
                        action(ui,commands,"Cancel run","work_cancel",ready&&owned&&revision(run)>0,||json!({"runId":run["runId"],"expectedRunRevision":run["revision"]}));
                        if !owned {hint(ui,"Control belongs to an earlier host session. Refresh to reconcile safely.");}
                    }
                });
            }));
            ui.add_space(10.0);
        }
        ui.add_space(16.0);
        ui.collapsing(t(ui, "Recent durable activity"), |ui| {
            let receipts = array(&work["receipts"]);
            if receipts.is_empty() {
                hint(
                    ui,
                    "No receipts yet. Accepted actions leave a durable outcome here.",
                );
            }
            for r in receipts.iter().rev().take(8) {
                ui.horizontal_wrapped(|ui| {
                    status(ui, text(r, "outcome"));
                    ui.label(t(ui, text(r, "reasonCode")));
                    hint(ui, &format_time(ui, text(r, "completedAt")));
                });
            }
        });
    }
    fn work_composer(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        card(ui, |ui| {
            section_title(ui, "New task", "One clear outcome for Codex.");
            ui.add_space(14.0);
            field(ui, "Task name", &mut self.work.title, 160, false);
            let previous_workspace = self.work.workspace.clone();
            choose_workspace(ui, "work-project", &mut self.work.workspace, snapshot);
            if self.work.workspace != previous_workspace && !self.work.workspace.is_empty() {
                self.work.path.clear();
                self.work.path_problem = None;
            }
            ui.collapsing(t(ui, "Use another Git worktree once"), |ui| {
                if !self.work.path.is_empty() {
                    folder_preview(ui, &self.work.path);
                }
                if control(
                    ui,
                    if self.work.path.is_empty() {
                        "Choose folder"
                    } else {
                        "Change folder"
                    },
                    true,
                )
                .clicked()
                {
                    self.folder_request = Some(folder::Target::Task);
                }
                if let Some(code) = &self.work.path_problem {
                    ui.colored_label(RED, t(ui, code));
                    if matches!(
                        code.as_str(),
                        "WORKSPACE_ACCESS_DENIED" | "WORKSPACE_READ_ONLY"
                    ) {
                        hint(ui, folder::permission_guidance(code));
                    }
                }
                if !self.work.path.is_empty()
                    && control(ui, "Use a registered folder instead", true).clicked()
                {
                    self.work.path.clear();
                    self.work.path_problem = None;
                }
                hint(
                    ui,
                    "Select a folder for this task, or choose a registered folder above.",
                );
            });
            multiline(ui, "Instruction", &mut self.work.instruction, 50_000, 3);
            if submit(ui, "Add to queue", "work_enqueue") {
                match self.work.payload(snapshot) {
                    Ok(payload) => {
                        commands.push(Command::new("work_enqueue", payload));
                        self.form_error = None;
                    }
                    Err(e) => self.form_error = Some(e),
                }
            }
            hint(
                ui,
                "Review the queued task before starting. Instructions stay local.",
            );
        });
    }

    fn teams(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let teams = items(&snapshot["teams"]);
        egui::CollapsingHeader::new(t(ui, "Form a local team"))
            .default_open(teams.is_empty())
            .show(ui, |ui| {
                card(ui, |ui| {
                    field(ui, "Team name", &mut self.team.name, 80, false);
                    choose_workspace(ui, "team-project", &mut self.team.workspace, snapshot);
                    choose(
                        ui,
                        "team-template",
                        "Template",
                        &mut self.team.template,
                        array(&snapshot["templates"]),
                        ("id", "name"),
                        false,
                    );
                    let packs = array(&snapshot["packs"]);
                    let orchestrators: Vec<_> = packs
                        .iter()
                        .filter(|p| text(p, "role") == "orchestrator")
                        .collect();
                    let workers: Vec<_> = packs
                        .iter()
                        .filter(|p| text(p, "role") != "orchestrator")
                        .collect();
                    choose_refs(
                        ui,
                        "orchestrator",
                        "Orchestrator",
                        &mut self.team.orchestrator,
                        &orchestrators,
                        ("id", "name"),
                        false,
                    );
                    for slot in 0..4 {
                        choose_refs(
                            ui,
                            format!("worker-{slot}"),
                            &localized_format!(
                                ui,
                                "Agent {0}{1}",
                                slot + 1,
                                if slot == 0 {
                                    t(ui, " (required)").into_owned()
                                } else {
                                    String::new()
                                }
                            ),
                            &mut self.team.workers[slot],
                            &workers,
                            ("id", "name"),
                            true,
                        );
                    }
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(t(ui, "Concurrency"));
                        ui.add(egui::DragValue::new(&mut self.team.concurrency).range(1..=4));
                        ui.label(t(ui, "Retries"));
                        ui.add(egui::DragValue::new(&mut self.team.retries).range(0..=3));
                        ui.label(t(ui, "Minutes"));
                        ui.add(egui::DragValue::new(&mut self.team.minutes).range(5..=480));
                    });
                    ui.checkbox(
                        &mut self.team.continue_independent,
                        t(ui, "Continue independent tasks after a failure"),
                    );
                    multiline(
                        ui,
                        "Goal and acceptance criteria",
                        &mut self.team.goal,
                        10_000,
                        5,
                    );
                    if submit(ui, "Create team plan", "team_create") {
                        match self.team.payload(snapshot) {
                            Ok(payload) => {
                                commands.push(Command::new("team_create", payload));
                                self.form_error = None;
                            }
                            Err(e) => self.form_error = Some(e),
                        }
                    }
                    hint(
                        ui,
                        "The team is saved as a draft. Review it before starting.",
                    );
                })
            });
        ui.add_space(16.0);
        if teams.is_empty() {
            empty(
                ui,
                "No team yet",
                "Select an orchestrator, worker roles and a bounded template above.",
            );
        }
        for team in teams {
            ui.push_id(text(team,"teamId"),|ui|card(ui,|ui| {
            ui.horizontal_wrapped(|ui|{ui.heading(text(team,"name"));status(ui,text(team,"status"));});
            ui.label(text(team,"goal"));hint(ui,&localized_format!(ui, "{0} · concurrency {1} · {2} retries",t(ui,&text(team,"templateId").replace('-'," ")),team["settings"]["concurrency"],team["settings"]["retryLimit"]));
            ui.add_space(10.0);
            let active=matches!(text(team,"status"),"active"|"waiting");
            ui.horizontal_wrapped(|ui| {
                if text(team,"status")=="draft" {action(ui,commands,"Start team","team_start",codex_ready(snapshot)&&revision(team)>0,||team_target(team));}
                if active {action(ui,commands,"Run ready work","team_advance",revision(team)>0,||team_target(team));action(ui,commands,"Cancel team","team_cancel",revision(team)>0,||team_target(team));}
            });
            ui.add_space(10.0);
            for task in array(&team["tasks"]) {ui.push_id(text(task,"taskId"),|ui| {
                let agent=array(&team["agents"]).iter().find(|a|a["agentId"]==task["agentId"]);
                ui.separator();ui.horizontal_wrapped(|ui|{ui.strong(task_title(ui,task,team,agent));status(ui,text(task,"status"));});
                hint(ui,&localized_format!(ui, "{0} · attempt {1}/{2}",agent.map(|a|catalog_text(ui,&a["pack"],"name")).unwrap_or_else(||t(ui,"Unassigned")),task["attempt"],task["maxAttempts"]));
                if active&&text(task,"status")=="waiting_question"&&!task["question"].is_null() {
                    let q=&task["question"];let key=format!("{}:{}",text(team,"teamId"),text(q,"questionId"));
                    let answer=self.answers.entry(key).or_default();let max=q["maxAnswerLength"].as_u64().unwrap_or(1000).min(50_000) as usize;
                    multiline(ui,text(q,"prompt"),answer,max,3);
                    let valid=required(answer,max,"").is_ok();
                    action(ui,commands,"Record answer","team_answer",valid&&revision(team)>0,||json!({"teamId":team["teamId"],"expectedTeamRevision":team["revision"],"questionId":q["questionId"],"answer":answer.trim()}));
                }
                if active&&text(task,"status")=="waiting_approval" {
                    let note=self.notes.entry(format!("{}:{}",text(team,"teamId"),text(task,"taskId"))).or_default();field(ui,"Decision note (optional)",note,1000,false);
                    let valid=utf16_len(note)<=1000&&revision(team)>0;
                    ui.horizontal_wrapped(|ui| {
                        action(ui,commands,"Approve","team_approve",valid,||json!({"teamId":team["teamId"],"expectedTeamRevision":team["revision"],"taskId":task["taskId"],"decision":"approved","note":note.trim()}));
                        action(ui,commands,"Reject","team_approve",valid,||json!({"teamId":team["teamId"],"expectedTeamRevision":team["revision"],"taskId":task["taskId"],"decision":"rejected","note":note.trim()}));
                    });
                }
            });}
            ui.add_space(10.0);
            ui.collapsing(t(ui, "Agents and controls"),|ui|for agent in array(&team["agents"]) {ui.push_id(text(agent,"agentId"),|ui| {
                ui.horizontal_wrapped(|ui| {ui.strong(catalog_text(ui,&agent["pack"],"name"));status(ui,text(agent,"status"));
                    if active&&agent["agentId"]!=team["orchestratorAgentId"]&&!matches!(text(agent,"status"),"completed"|"cancelled") {action(ui,commands,"Cancel agent","team_cancel_agent",revision(team)>0,||json!({"teamId":team["teamId"],"expectedTeamRevision":team["revision"],"agentId":agent["agentId"]}));}
                });
            });});
        }));
            ui.add_space(14.0);
        }
    }

    fn agents(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let packs = array(&snapshot["packs"]);
        hint(
            ui,
            &localized_format!(ui, "{0} inspectable local packs", packs.len()),
        );
        ui.add_space(12.0);
        for pack in packs {
            ui.push_id(text(pack, "id"), |ui| {
                card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(catalog_text(ui, pack, "name"));
                        status(ui, text(pack, "role"));
                        hint(ui, text(&pack["provenance"], "kind"));
                    });
                    ui.label(catalog_text(ui, pack, "description"));
                    egui::CollapsingHeader::new(t(ui, "Inspect pack contract")).show(ui, |ui| {
                        ui.label(localized_format!(
                            ui,
                            "Provider: {0} · {1} minutes · {2} attempts",
                            text(pack, "providerId"),
                            pack["limits"]["timeoutMinutes"],
                            pack["limits"]["maxAttempts"]
                        ));
                        ui.label(localized_format!(
                            ui,
                            "Permissions: {0}",
                            joined(&pack["permissions"])
                        ));
                        ui.label(localized_format!(
                            ui,
                            "Quality checks: {0}",
                            joined(&pack["qualityChecks"])
                        ));
                        ui.label(catalog_text(ui, pack, "instructions"));
                        ui.strong(t(ui, "Expected output"));
                        ui.label(catalog_text(ui, pack, "expectedOutput"));
                        ui.monospace(format!("{}@{}", text(pack, "id"), text(pack, "version")));
                        if text(&pack["provenance"], "kind") == "local" {
                            action(
                                ui,
                                commands,
                                "Remove custom pack",
                                "pack_remove",
                                true,
                                || json!({"packId":pack["id"]}),
                            );
                        }
                    });
                })
            });
            ui.add_space(8.0);
        }
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.heading(t(ui, "Install a custom pack"));
            multiline(
                ui,
                "Inspectable JSON manifest",
                &mut self.manifest,
                65_536,
                6,
            );
            if submit(ui, "Install custom pack", "pack_install") {
                match parse_manifest(&self.manifest) {
                    Ok(manifest) => {
                        commands.push(Command::new("pack_install", json!({"manifest":manifest})));
                        self.form_error = None;
                    }
                    Err(e) => self.form_error = Some(e),
                }
            }
            hint(ui,"Packs are local data. They cannot load executable plugins or widen the Codex sandbox.");
        });
    }

    fn providers(&self, ui: &mut Ui, snapshot: &Value) {
        let observations = array(&snapshot["observation"]["observations"]);
        if observations.is_empty() {
            empty(
                ui,
                "Providers have not been observed",
                "Refresh providers to check the installed CLIs and proven controls.",
            );
        }
        for observation in observations {
            let provider = &observation["provider"];
            ui.push_id(text(provider, "id"), |ui| {
                card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(text(provider, "displayName"));
                        status(ui, text(provider, "state"));
                    });
                    hint(
                        ui,
                        &localized_format!(
                            ui,
                            "CLI version: {0}",
                            provider["version"]
                                .as_str()
                                .map(std::borrow::Cow::Borrowed)
                                .unwrap_or_else(|| t(ui, "Not available"))
                        ),
                    );
                    ui.label(localized_format!(
                        ui,
                        "Recent observation: {0} · explicit live status: {1}",
                        if provider["capabilities"]["recentObservation"] == true {
                            t(ui, "supported")
                        } else {
                            t(ui, "unavailable")
                        },
                        if provider["capabilities"]["explicitLiveStatus"] == true {
                            t(ui, "supported")
                        } else {
                            t(ui, "unavailable")
                        }
                    ));
                    if !provider["error"].is_null() {
                        ui.colored_label(RED, t(ui, text(&provider["error"], "message")));
                        ui.monospace(text(&provider["error"], "code"));
                    }
                    hint(
                        ui,
                        &localized_format!(
                            ui,
                            "Stored snapshot from {0}",
                            format_time(ui, text(observation, "observedAt"))
                        ),
                    );
                    let sessions = array(&observation["sessions"]);
                    ui.add_space(12.0);
                    ui.strong(localized_format!(
                        ui,
                        "Recent sessions ({0})",
                        sessions.len()
                    ));
                    if sessions.is_empty() {
                        hint(ui, "No recent interactive sessions returned.");
                    }
                    for session in sessions {
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(text(session, "providerSessionId"));
                            status(ui, text(session, "status"));
                        });
                        hint(
                            ui,
                            &localized_format!(
                                ui,
                                "Last observed {0}",
                                format_time(ui, text(session, "lastObservedAt"))
                            ),
                        );
                    }
                })
            });
            ui.add_space(14.0);
        }
    }

    fn cloud(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let cloud = &snapshot["cloud"];
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(t(ui, "Patchfleet Cloud"));
                status(
                    ui,
                    if cloud["paired"] == true {
                        "Paired"
                    } else {
                        "Local only"
                    },
                );
            });
            hint(ui,"Only allowlisted operational status leaves this machine. Instructions, paths and transcripts stay local.");
            ui.add_space(16.0);
            if cloud["error"] == true {
                ui.colored_label(
                    RED,
                    t(
                        ui,
                        "Cloud settings need local recovery. Local work remains available.",
                    ),
                );
                ui.label(t(ui, "Close Patchfleet and back up the private cloud.json file in its data directory. Explicitly repair or remove that file, then reopen the app."));
                hint(
                    ui,
                    "Pairing is unavailable until the local Cloud state can be read safely.",
                );
                return;
            }
            if cloud["paired"] == true {
                ui.label(text(cloud, "cloudUrl"));
                ui.monospace(localized_format!(ui, "Host {0}", text(cloud, "hostId")));
                hint(
                    ui,
                    &localized_format!(
                        ui,
                        "Last successful sync: {0}",
                        format_time(ui, text(cloud, "lastSuccessAt"))
                    ),
                );
                if let Some(code) = cloud["lastErrorCode"].as_str() {
                    ui.colored_label(RED, t(ui, cloud_error(code)));
                    ui.monospace(code);
                }
                ui.horizontal(|ui| {
                    action(ui, commands, "Sync now", "cloud_sync", true, || json!({}));
                    action(
                        ui,
                        commands,
                        "Disconnect Cloud",
                        "cloud_unpair",
                        true,
                        || json!({}),
                    );
                });
            } else {
                field(ui, "Cloud URL", &mut self.cloud_url, 2048, false);
                field(ui, "Host name", &mut self.host_name, 80, false);
                field(ui, "Pairing code", &mut self.pairing_code, 256, true);
                if submit(ui, "Pair this host", "cloud_pair") {
                    if required(&self.cloud_url, 2048, "").is_err()
                        || required(&self.host_name, 80, "").is_err()
                        || required(&self.pairing_code, 256, "").is_err()
                    {
                        self.form_error = Some("Enter a Cloud URL, host name and pairing code.");
                    } else {
                        commands.push(Command::new("cloud_pair",json!({"cloudUrl":self.cloud_url.trim(),"displayName":self.host_name.trim(),"pairingCode":self.pairing_code.trim()})));
                        self.pairing_code.clear();
                        self.form_error = None;
                    }
                }
            }
        });
    }
}

fn array(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn decision_manifest(snapshot: &Value, project: &str) -> u64 {
    array(&snapshot["projectDecisions"]["projects"])
        .iter()
        .find(|p| text(p, "projectId") == project)
        .map(revision)
        .unwrap_or(0)
}
fn decision_target(decision: &Value) -> Value {
    json!({"projectId":decision["projectId"],"decisionId":decision["decisionId"],"expectedDecisionRevision":revision(decision)})
}
fn decision_text(value: &str, max: usize, required: bool) -> bool {
    (!required || !value.trim().is_empty())
        && utf16_len(value) <= max
        && !value
            .chars()
            .any(|c| matches!(c as u32,0..=8|11..=12|14..=31|127))
}
fn decision_status_label(value: &str) -> &str {
    match value {
        "all" => "All decisions",
        "proposed" => "Proposed drafts",
        "accepted" => "Accepted decisions",
        "rejected" => "Rejected decisions",
        "superseded" => "Superseded decisions",
        _ => value,
    }
}
fn language(ui: &Ui) -> Language {
    ui.ctx()
        .data(|data| data.get_temp(egui::Id::new("console-language")))
        .unwrap_or_default()
}
fn t<'a>(ui: &Ui, key: &'a str) -> std::borrow::Cow<'a, str> {
    i18n::tr(language(ui), key)
}
fn tf(ui: &Ui, key: &str, args: &[&str]) -> String {
    i18n::format(language(ui), key, args)
}
fn catalog_text<'a>(ui: &Ui, value: &'a Value, key: &str) -> std::borrow::Cow<'a, str> {
    let raw = text(value, key);
    let builtin = text(&value["provenance"], "kind") == "built-in"
        || (value["provenance"].is_null()
            && matches!(
                text(value, "id"),
                "product-feature"
                    | "bug-fix"
                    | "saas-launch"
                    | "design-frontend"
                    | "security-audit"
                    | "release"
            ));
    if !builtin {
        return std::borrow::Cow::Borrowed(raw);
    }
    i18n::catalog(language(ui), raw)
}
fn task_title<'a>(
    ui: &Ui,
    task: &'a Value,
    team: &Value,
    agent: Option<&Value>,
) -> std::borrow::Cow<'a, str> {
    let raw = text(task, "title");
    if let Some(pack) = agent
        .map(|agent| &agent["pack"])
        .filter(|pack| text(&pack["provenance"], "kind") == "built-in")
    {
        let template = text(team, "templateId").replace('-', " ");
        if raw == format!("{}: {}", text(pack, "name"), template) {
            return std::borrow::Cow::Owned(localized_format!(
                ui,
                "{0}: {1}",
                catalog_text(ui, pack, "name"),
                t(ui, &template)
            ));
        }
    }
    std::borrow::Cow::Borrowed(raw)
}
fn language_selector(ui: &mut Ui, snapshot: &Value, busy: bool, commands: &mut Vec<Command>) {
    ui.label(RichText::new(t(ui, "Language")).size(10.0).color(MUTED));
    ui.add_enabled_ui(!busy, |ui| {
        ui.horizontal(|ui| {
            for (choice, label) in [(Language::Tr, "Türkçe"), (Language::En, "English")] {
                let selected = language(ui) == choice;
                let response = ui.selectable_label(selected, label);
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("test-action", "desktop_language", label)),
                        response.rect,
                    )
                });
                if response.clicked()
                    && (!selected || snapshot["desktopPreferences"]["error"] == true)
                    && commands.is_empty()
                {
                    commands.push(Command::new(
                        "desktop_language",
                        json!({"language":choice.code()}),
                    ));
                }
            }
        })
    });
    if snapshot["desktopPreferences"]["error"] == true {
        hint(
            ui,
            "Language preference could not be read. Select a language to replace it.",
        );
    }
}
fn items(value: &Value) -> &[Value] {
    array(&value["items"])
}
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
fn revision(value: &Value) -> u64 {
    value["revision"].as_u64().unwrap_or(0)
}
fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}
fn required(value: &str, max: usize, message: &'static str) -> Result<(), &'static str> {
    if value.trim().is_empty() || utf16_len(value.trim()) > max {
        Err(message)
    } else {
        Ok(())
    }
}
fn team_target(team: &Value) -> Value {
    json!({"teamId":team["teamId"],"expectedTeamRevision":team["revision"]})
}
fn parse_manifest(input: &str) -> Result<Value, &'static str> {
    if input.len() > 65_536 {
        return Err("Pack manifest exceeds 64 KiB.");
    }
    let manifest: Value =
        serde_json::from_str(input).map_err(|_| "Pack manifest must be valid JSON.")?;
    if !manifest.is_object() {
        return Err("Pack manifest must be a JSON object.");
    }
    Ok(manifest)
}
fn codex_ready(snapshot: &Value) -> bool {
    array(&snapshot["observation"]["observations"])
        .iter()
        .any(patchfleet_core::providers::supports_codex_control)
}
fn joined(value: &Value) -> String {
    array(value)
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
fn format_time(ui: &Ui, value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|_| t(ui, "Not recorded").into_owned())
}
fn cloud_error(code: &str) -> &'static str {
    match code {
        "CLOUD_AUTH_REJECTED" => "Host access was revoked. Disconnect and pair again.",
        "CLOUD_PROTOCOL_INVALID" => "Cloud returned an incompatible response.",
        "CLOUD_CONFLICT" => "Cloud rejected a conflicting snapshot. Local work stays available.",
        _ => "Cloud sync is unavailable. Local work continues while the host retries.",
    }
}
fn hint(ui: &mut Ui, value: &str) {
    ui.label(RichText::new(t(ui, value)).size(11.0).color(MUTED));
}
fn raw_hint(ui: &mut Ui, value: &str) {
    ui.label(RichText::new(value).size(11.0).color(MUTED));
}

fn path_label(ui: &mut Ui, value: &str) {
    ui.add(egui::Label::new(RichText::new(value).size(11.0).color(MUTED)).truncate())
        .on_hover_text(value);
}

fn folder_preview(ui: &mut Ui, path: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(18, 22, 28))
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(12)
        .show(ui, |ui| {
            let text_width = (ui.available_width() - 28.0 - ui.spacing().item_spacing.x).max(0.0);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                draw_icon(ui.painter(), rect.shrink(4.0), Tab::Projects, MINT);
                ui.allocate_ui_with_layout(
                    egui::vec2(text_width, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_max_width(text_width);
                        let name = std::path::Path::new(path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(path);
                        if name.is_empty() {
                            ui.label(t(ui, "No folder selected"));
                        } else {
                            ui.add(
                                egui::Label::new(RichText::new(name).strong().color(TEXT))
                                    .truncate(),
                            );
                            path_label(ui, path);
                        }
                    },
                );
            });
        });
    ui.add_space(8.0);
}
fn status(ui: &mut Ui, value: &str) {
    let color = match value {
        "available" | "completed" | "applied" | "Codex ready" | "Paired" => MINT,
        "failed" | "rejected" | "timed_out" => RED,
        _ => MUTED,
    };
    let label = t(ui, value).replace('_', " ");
    paint_status(ui, label, color);
}
fn raw_status(ui: &mut Ui, value: &str) {
    paint_status(ui, value.to_owned(), MUTED);
}
fn paint_status(ui: &mut Ui, label: String, color: Color32) {
    let galley =
        ui.painter()
            .layout_no_wrap(label.clone(), egui::FontId::proportional(10.0), color);
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + egui::vec2(14.0, 6.0), egui::Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), &label)
    });
    ui.painter()
        .rect_filled(rect, 4, color.gamma_multiply(0.10));
    ui.painter()
        .galley(rect.min + egui::vec2(7.0, 3.0), galley, color);
}
fn card(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(18)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            contents(ui);
        });
}
fn empty(ui: &mut Ui, title: &str, description: &str) {
    card(ui, |ui| {
        ui.heading(t(ui, title));
        ui.add_space(8.0);
        hint(ui, description);
    });
    ui.add_space(14.0);
}
fn field(ui: &mut Ui, label: &str, value: &mut String, max: usize, password: bool) {
    ui.label(RichText::new(t(ui, label)).size(11.0).color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Body)
            .margin(egui::Margin::symmetric(10, 7))
            .char_limit(max)
            .password(password),
    );
    ui.add_space(8.0);
}
fn multiline(ui: &mut Ui, label: &str, value: &mut String, max: usize, rows: usize) {
    ui.label(RichText::new(t(ui, label)).size(11.0).color(MUTED));
    ui.add(
        egui::TextEdit::multiline(value)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Body)
            .margin(egui::Margin::symmetric(10, 7))
            .desired_rows(rows)
            .char_limit(max),
    );
    ui.add_space(8.0);
}
fn choose_workspace(ui: &mut Ui, id: &str, selected: &mut String, snapshot: &Value) {
    choose(
        ui,
        id,
        "Registered worktree",
        selected,
        items(&snapshot["workspaces"]),
        ("workspaceId", "workingDirectory"),
        true,
    );
}
fn choose(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    selected: &mut String,
    options: &[Value],
    keys: (&str, &str),
    empty: bool,
) {
    let refs: Vec<_> = options.iter().collect();
    choose_refs(ui, id, label, selected, &refs, keys, empty);
}
fn choose_refs(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    selected: &mut String,
    options: &[&Value],
    keys: (&str, &str),
    empty: bool,
) {
    let (id_key, name_key) = keys;
    ui.label(RichText::new(t(ui, label)).size(11.0).color(MUTED));
    let current = options
        .iter()
        .find(|v| text(v, id_key) == selected)
        .map(|v| {
            if id_key == "id" {
                catalog_text(ui, v, name_key)
            } else {
                std::borrow::Cow::Borrowed(text(v, name_key))
            }
        })
        .unwrap_or_else(|| {
            t(
                ui,
                if selected.is_empty() {
                    "Choose…"
                } else {
                    "Unavailable selection"
                },
            )
        });
    egui::ComboBox::from_id_salt(egui::Id::new(id))
        .selected_text(current)
        .truncate()
        .width(ui.available_width().min(600.0))
        .show_ui(ui, |ui| {
            if empty {
                ui.selectable_value(selected, String::new(), t(ui, "None"));
            }
            for value in options {
                ui.selectable_value(
                    selected,
                    text(value, id_key).to_owned(),
                    if id_key == "id" {
                        catalog_text(ui, value, name_key)
                    } else {
                        std::borrow::Cow::Borrowed(text(value, name_key))
                    },
                );
            }
        });
    ui.add_space(8.0);
}
fn control(ui: &mut Ui, label: &str, enabled: bool) -> egui::Response {
    let response = ui.add_enabled(enabled, egui::Button::new(t(ui, label)));
    #[cfg(test)]
    let visibility = (ui.clip_rect(), ui.is_visible());
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("test-action", "memory_control", label)),
            response.rect,
        );
        data.insert_temp(egui::Id::new(("test-action-visibility", label)), visibility);
    });
    response
}
fn submit(ui: &mut Ui, label: &str, action: &str) -> bool {
    let response = ui.add(
        egui::Button::new(
            RichText::new(t(ui, label))
                .size(12.0)
                .strong()
                .color(Color32::from_rgb(18, 37, 29)),
        )
        .fill(MINT)
        .stroke(egui::Stroke::NONE)
        .corner_radius(6)
        .min_size(egui::vec2(128.0, 32.0)),
    );
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(egui::Id::new(("test-action", action, label)), response.rect)
    });
    let _ = action;
    response.clicked()
}
fn action(
    ui: &mut Ui,
    commands: &mut Vec<Command>,
    label: &str,
    name: &'static str,
    enabled: bool,
    payload: impl FnOnce() -> Value,
) {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(t(ui, label)).size(11.0).color(TEXT))
            .fill(Color32::TRANSPARENT)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .corner_radius(5)
            .min_size(egui::vec2(74.0, 28.0)),
    );
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(egui::Id::new(("test-action", name, label)), response.rect)
    });
    if response.clicked() && commands.is_empty() {
        commands.push(Command::new(name, payload()));
    }
}

fn constrained(ui: &mut Ui, max_width: f32, contents: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width().min(max_width), 0.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| contents(ui),
    );
}
fn section_title(ui: &mut Ui, title: &str, description: &str) {
    ui.label(RichText::new(t(ui, title)).size(16.0).strong().color(TEXT));
    ui.add_space(3.0);
    hint(ui, description);
}
fn field_hint(ui: &mut Ui, label: &str, value: &mut String, max: usize, placeholder: &str) {
    ui.label(RichText::new(t(ui, label)).size(11.0).color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Body)
            .margin(egui::Margin::symmetric(10, 7))
            .char_limit(max)
            .hint_text(t(ui, placeholder)),
    );
    ui.add_space(12.0);
}
fn source_kind_label(kind: &str) -> &'static str {
    match kind {
        "message" => "Project message",
        "work_instruction" => "Task instruction",
        "team_goal" => "Team goal",
        "owner_answer" => "Owner answer",
        "approval_note" => "Approval note",
        _ => "Source context",
    }
}
fn navigation(ui: &mut Ui, tab: Tab, selected: bool, count: usize) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());
    #[cfg(test)]
    let clipped = rect.intersect(ui.clip_rect());
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("test-action", "navigation", tab.name())),
            rect,
        );
        data.insert_temp(egui::Id::new(("navigation-clip", tab.name())), clipped);
    });
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            selected,
            t(ui, tab.name()),
        )
    });
    let background = if selected {
        Color32::from_rgb(27, 41, 36)
    } else if response.hovered() {
        Color32::from_rgb(27, 31, 38)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6, background);
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 8.0), egui::vec2(2.0, 18.0)),
            1,
            MINT,
        );
    }
    let color = if selected { TEXT } else { MUTED };
    draw_icon(
        ui.painter(),
        egui::Rect::from_min_size(rect.min + egui::vec2(11.0, 8.0), egui::vec2(18.0, 18.0)),
        tab,
        if selected { MINT } else { MUTED },
    );
    ui.painter().text(
        rect.min + egui::vec2(39.0, 17.0),
        egui::Align2::LEFT_CENTER,
        t(ui, tab.name()),
        egui::FontId::proportional(13.0),
        color,
    );
    if count > 0 {
        ui.painter().text(
            egui::pos2(rect.max.x - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            count.to_string(),
            egui::FontId::proportional(10.0),
            MUTED,
        );
    }
    response.clicked()
        || (response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)))
}
fn draw_icon(painter: &egui::Painter, rect: egui::Rect, tab: Tab, color: Color32) {
    let p =
        |x: f32, y: f32| rect.min + egui::vec2(x * rect.width() / 24.0, y * rect.height() / 24.0);
    let stroke = egui::Stroke::new(1.3, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    match tab {
        Tab::Projects => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(2.0, 8.0), p(22.0, 20.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((2.0, 8.0), (2.0, 5.0));
            line((2.0, 5.0), (9.0, 5.0));
            line((9.0, 5.0), (12.0, 8.0));
        }
        Tab::Memory => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(4.0, 3.0), p(20.0, 21.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((8.0, 8.0), (16.0, 8.0));
            line((8.0, 12.0), (16.0, 12.0));
            line((8.0, 16.0), (13.0, 16.0));
        }
        Tab::Decisions => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(4.0, 2.0), p(20.0, 22.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((8.0, 7.0), (16.0, 7.0));
            line((8.0, 11.0), (16.0, 11.0));
            line((8.0, 16.0), (11.0, 19.0));
            line((11.0, 19.0), (17.0, 13.0));
        }
        Tab::Work => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(4.0, 3.0), p(20.0, 22.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
            for y in [8.0, 13.0, 18.0] {
                line((8.0, y), (16.0, y));
            }
        }
        Tab::Teams => {
            for (x, y) in [(12.0, 6.0), (5.0, 18.0), (19.0, 18.0)] {
                painter.circle_stroke(p(x, y), rect.width() / 8.0, stroke);
            }
            line((10.0, 9.0), (7.0, 14.0));
            line((14.0, 9.0), (17.0, 14.0));
        }
        Tab::Agents => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(3.0, 7.0), p(21.0, 21.0)),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((12.0, 3.0), (12.0, 7.0));
            painter.circle_filled(p(8.0, 13.0), rect.width() / 15.0, color);
            painter.circle_filled(p(16.0, 13.0), rect.width() / 15.0, color);
            line((8.0, 17.0), (16.0, 17.0));
        }
        Tab::Providers => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(2.0, 4.0), p(22.0, 21.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((6.0, 9.0), (9.0, 12.0));
            line((9.0, 12.0), (6.0, 15.0));
            line((12.0, 15.0), (18.0, 15.0));
        }
        Tab::Cloud => {
            let points = [
                (6.0, 18.0),
                (3.0, 15.0),
                (3.0, 12.0),
                (6.0, 9.0),
                (9.0, 9.0),
                (11.0, 5.0),
                (15.0, 5.0),
                (18.0, 10.0),
                (21.0, 11.0),
                (22.0, 15.0),
                (19.0, 18.0),
            ];
            painter.add(egui::Shape::closed_line(
                points.map(|(x, y)| p(x, y)).to_vec(),
                stroke,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    fn decision_snapshot(status: &str) -> Value {
        let mut data = memory_snapshot();
        let record = json!({"decisionId":"decision:fixture","projectId":"project:fixture","title":"Memory","status":status,"revision":2,"createdAt":"2026-10-15T00:00:00.000Z","updatedAt":"2026-10-15T00:00:00.000Z","sourceCount":1,"context":"Work","decision":"Keep owner text unchanged.","rationale":"Projects","consequences":"Local only","actorId":"local-owner","sources":[{"sourceId":"source:fixture","revision":3}],"supersedesDecisionId":null,"supersededByDecisionId":null});
        data["projectDecisions"] = json!({"schemaVersion":4,"revision":6,"projects":[{"projectId":"project:fixture","revision":6,"decisionCount":1,"acceptedCount":u8::from(status=="accepted"),"proposedCount":u8::from(status=="proposed")}]});
        data["decisionPage"] = json!({"reasonCode":"PROJECT_DECISIONS_READ","projectId":"project:fixture","revision":6,"offset":0,"limit":10,"query":"","status":"all","total":1,"decisions":[record.clone()]});
        let source = json!({"sourceId":"source:fixture","projectId":"project:fixture","title":"Owner evidence","text":"Original source text","kind":"message","revision":3,"origin":{"type":"message","id":"cmd:original","eventId":"event:original"}});
        data["memoryPage"]["sources"] = json!([source.clone()]);
        data["memoryPage"]["total"] = json!(1);
        data["decisionDetail"] = json!({"reasonCode":"PROJECT_DECISION_READ","projectId":"project:fixture","revision":6,"decision":record,"history":[{"status":"proposed","revision":1,"actorId":"local-owner","at":"2026-10-15T00:00:00.000Z"}],"sources":[source]});
        data
    }

    #[test]
    fn decision_proposal_click_uses_current_citations_and_never_submits_busy_or_stale() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Decisions,
            ..Default::default()
        };
        let mut data = decision_snapshot("proposed");
        console.decisions.select("project:fixture");
        console.decisions.composer_open = true;
        console.decisions.title = "Memory".into();
        console.decisions.decision = "Keep owner text unchanged.".into();
        console.decisions.context = "Work".into();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "decision-citation",
            "source:fixture"
        )
        .is_empty());
        assert_eq!(console.decisions.citations.len(), 1);
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_decision_propose",
            "Save decision draft",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["expectedProjectRevision"], 9);
        assert_eq!(
            commands[0].payload["sources"],
            json!([{"sourceId":"source:fixture","revision":3}])
        );
        assert_eq!(commands[0].payload["title"], "Memory");
        assert_eq!(commands[0].payload["context"], "Work");
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "project_decision_propose",
            "Save decision draft"
        )
        .is_empty());
        data["projectMemory"]["projects"][0]["revision"] = json!(10);
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_decision_propose",
            "Save decision draft",
        );
        assert!(!commands
            .iter()
            .any(|c| c.action == "project_decision_propose"));
        assert!(console.decisions.citations.is_empty());
        assert!(console.decisions.sources_changed);
    }

    #[test]
    fn decision_lifecycle_clicks_require_current_scoped_details_and_explicit_replacement() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Decisions,
            ..Default::default()
        };
        let mut data = decision_snapshot("proposed");
        for (action, label) in [
            ("project_decision_accept", "Accept decision"),
            ("project_decision_reject", "Reject decision"),
        ] {
            let commands = click(&ctx, &mut console, &data, false, action, label);
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].payload["decisionId"], "decision:fixture");
            assert_eq!(commands[0].payload["expectedDecisionRevision"], 2);
        }
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "project_decision_accept",
            "Accept decision"
        )
        .is_empty());
        data = decision_snapshot("accepted");
        data["decisionPage"]["decisions"].as_array_mut().unwrap().push(json!({"projectId":"project:fixture","decisionId":"decision:replacement","title":"Replacement","status":"proposed","revision":1,"sourceCount":1}));
        assert!(
            render(&ctx, &mut console, &data, false, vec![])
                .0
                .is_empty(),
            "opening an accepted decision never accepts a replacement"
        );
        console.decisions.replacement = "decision:replacement".into();
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_decision_supersede",
            "Supersede decision",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["decisionId"], "decision:fixture");
        assert_eq!(commands[0].payload["expectedDecisionRevision"], 2);
        assert_eq!(
            commands[0].payload["replacementDecisionId"],
            "decision:replacement"
        );
        assert_eq!(commands[0].payload["expectedReplacementRevision"], 1);
        data["decisionPage"]["decisions"][1]["status"] = json!("accepted");
        assert!(!click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_decision_supersede",
            "Supersede decision"
        )
        .iter()
        .any(|c| c.action == "project_decision_supersede"));
        data["projectDecisions"]["projects"][0]["revision"] = json!(7);
        assert!(!click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_decision_accept",
            "Accept decision"
        )
        .iter()
        .any(|c| c.action == "project_decision_accept"));
    }

    #[test]
    fn decision_reads_page_filter_and_refresh_are_bounded_without_failed_read_loops() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Decisions,
            ..Default::default()
        };
        let mut data = decision_snapshot("proposed");
        data["decisionPage"] = Value::Null;
        data["decisionDetail"] = Value::Null;
        let first = render(&ctx, &mut console, &data, false, vec![]).0;
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].action, "project_decisions_read");
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        let refresh = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Refresh decisions",
        );
        assert_eq!(refresh.len(), 1);
        assert_eq!(refresh[0].payload["limit"], 10);
        console.decisions.query_input = "amounts".into();
        let query = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Apply decision filter",
        );
        assert_eq!(query[0].payload["query"], "amounts");
        assert_eq!(query[0].payload["offset"], 0);
        data = decision_snapshot("proposed");
        data["decisionPage"]["query"] = json!("amounts");
        data["decisionPage"]["total"] = json!(23);
        let next = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Next decisions",
        );
        assert_eq!(next[0].payload["offset"], 10);
        data["decisionPage"]["offset"] = json!(10);
        let previous = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Previous decisions",
        );
        assert_eq!(previous[0].payload["offset"], 0);
        console.decisions.select("project:other");
        assert!(console.decisions.citations.is_empty());
        assert!(console.decisions.selected.is_empty());
        assert!(console.decisions.title.is_empty());
    }

    #[test]
    fn applied_proposal_resets_once_and_rejections_preserve_owner_draft() {
        let mut panel = DecisionPanel::default();
        panel.select("project:fixture");
        panel.composer_open = true;
        panel.title = "Owner draft".into();
        panel.decision = "Exact text".into();
        panel.pending_proposal_manifest = Some(6);
        let mut data = decision_snapshot("proposed");
        panel.finish_proposal(
            &data,
            Some(&Notice::error(
                "project_decision_propose",
                "STALE_SOURCE_REVISION",
            )),
            false,
        );
        assert_eq!(panel.title, "Owner draft");
        assert!(panel.composer_open);
        data["projectDecisions"]["projects"][0]["revision"] = json!(7);
        let success = Notice::receipt(
            "project_decision_propose",
            &json!({"reasonCode":"PROJECT_DECISION_PROPOSED","outcome":"applied"}),
        );
        panel.finish_proposal(&data, Some(&success), false);
        assert!(panel.title.is_empty());
        assert!(!panel.composer_open);
        panel.composer_open = true;
        panel.title = "Next draft".into();
        panel.finish_proposal(&data, Some(&success), false);
        assert!(panel.composer_open);
        assert_eq!(panel.title, "Next draft");
        assert!(!decision_text(&"😀".repeat(81), 160, true));
        assert!(!decision_text("control\u{7}", 160, true));
    }

    #[test]
    fn decision_localization_preserves_original_evidence_and_rejects_foreign_detail() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Decisions,
            ..Default::default()
        };
        let mut data = decision_snapshot("proposed");
        data["memoryPage"]["sources"][0]["text"] = json!("Later source edit");
        data["memoryPage"]["sources"][0]["revision"] = json!(4);
        let copy = painted_copy(&ctx, &mut console, &data);
        assert!(copy.iter().any(|c| c == "Kararlar"));
        assert!(copy.iter().any(|c| c == "Kararı kabul et"));
        assert!(copy.iter().any(|c| c == "Memory"));
        assert!(copy.iter().any(|c| c == "Work"));
        assert!(copy.iter().any(|c| c == "Original source text"));
        assert!(!copy.iter().any(|c| c == "Later source edit"));
        data["desktopPreferences"] = json!({"language":"en","error":false});
        let copy = painted_copy(&ctx, &mut console, &data);
        assert!(copy.iter().any(|c| c == "Accept decision"));
        data["decisionDetail"]["projectId"] = json!("project:other");
        data["decisionDetail"]["decision"]["title"] = json!("FOREIGN_DECISION_CANARY");
        let copy = painted_copy(&ctx, &mut console, &data);
        assert!(!copy.iter().any(|c| c.contains("FOREIGN_DECISION_CANARY")));
    }

    #[test]
    fn minimum_sidebar_navigation_never_overlaps_language_controls() {
        for code in ["tr", "en"] {
            let ctx = egui::Context::default();
            let mut console = Console {
                tab: Tab::Decisions,
                ..Default::default()
            };
            let mut data = decision_snapshot("proposed");
            data["desktopPreferences"] = json!({"language":code,"error":false});
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 560.0))),
                    ..Default::default()
                },
                |ui| {
                    console.show(ui, &data, false, None, false);
                },
            );
            output.drop_without_applying_deltas();
            let footer: Rect = ctx.data(|d| {
                d.get_temp(egui::Id::new((
                    "test-action",
                    "desktop_language",
                    "English",
                )))
                .unwrap()
            });
            for tab in Tab::ALL {
                let clipped: Rect = ctx.data(|d| {
                    d.get_temp(egui::Id::new(("navigation-clip", tab.name())))
                        .unwrap()
                });
                if clipped.is_positive() {
                    assert!(
                        clipped.max.y <= footer.min.y,
                        "{} overlaps the language selector",
                        tab.name()
                    );
                }
            }
            let accept: Rect = ctx.data(|d| {
                d.get_temp(egui::Id::new((
                    "test-action",
                    "project_decision_accept",
                    "Accept decision",
                )))
                .unwrap()
            });
            assert!(accept.max.x <= 800.0);
            assert!(
                accept.min.y < 560.0,
                "acceptance must be discoverable before scrolling long owner text"
            );
        }
    }

    fn snapshot() -> Value {
        json!({"appVersion":"0.4.0","hostEpoch":"boot:current",
            "workspaces":{"items":[{"workspaceId":"workspace:fixture","displayName":"Fixture project","workingDirectory":"/tmp/fixture","revision":3}]},
            "work":{"items":[{"workItemId":"work:fixture","title":"Fixture task","instruction":"Bounded fixture","workingDirectory":"/tmp/fixture","status":"queued","revision":7}],"runs":[],"receipts":[]},
            "packs":[{"id":"pack:orchestrator","name":"Orchestrator","role":"orchestrator","provenance":{"kind":"built-in"}}, {"id":"pack:qa","name":"QA Engineer","role":"qa","provenance":{"kind":"built-in"}}],
            "templates":[{"id":"product-feature","name":"Product feature"}],"teams":{"items":[]},"cloud":{"paired":false},
            "projectMemory":{"schemaVersion":3,"revision":0,"projects":[]},"memoryPage":null,
            "observation":{"observations":[{"provider":{"id":"codex","displayName":"Codex","state":"available","version":"0.144.1","capabilities":{"recentObservation":true,"explicitLiveStatus":true}},"sessions":[]}]}})
    }
    fn render(
        ctx: &egui::Context,
        console: &mut Console,
        snapshot: &Value,
        busy: bool,
        events: Vec<Event>,
    ) -> (Vec<Command>, usize) {
        let mut commands = vec![];
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 1200.0))),
            events,
            ..Default::default()
        };
        let output = ctx.run_ui(input, |ui| {
            commands.extend(console.show(ui, snapshot, busy, None, false))
        });
        let shapes = output.shapes.len();
        output.drop_without_applying_deltas();
        (commands, shapes)
    }
    fn click(
        ctx: &egui::Context,
        console: &mut Console,
        snapshot: &Value,
        busy: bool,
        name: &str,
        label: &str,
    ) -> Vec<Command> {
        render(ctx, console, snapshot, busy, vec![]);
        let rect: Rect = ctx.data(|d| {
            d.get_temp(egui::Id::new(("test-action", name, label)))
                .expect("action was rendered")
        });
        let pos = rect.center();
        render(
            ctx,
            console,
            snapshot,
            busy,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
            ],
        );
        render(
            ctx,
            console,
            snapshot,
            busy,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::default(),
            }],
        )
        .0
    }

    #[test]
    fn native_tabs_render_fresh_and_populated_snapshots_without_side_effects() {
        for snapshot in [snapshot(), json!({})] {
            for tab in Tab::ALL {
                let ctx = egui::Context::default();
                let mut console = Console {
                    tab,
                    ..Default::default()
                };
                let (commands, shapes) = render(&ctx, &mut console, &snapshot, false, vec![]);
                assert!(commands.is_empty());
                assert!(shapes > 10, "{} should paint a usable frame", tab.name());
            }
        }
    }

    #[test]
    fn owner_click_keeps_current_revision_and_busy_or_unproven_control_blocks_start() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Work,
            ..Default::default()
        };
        let mut snapshot = snapshot();
        let commands = click(
            &ctx,
            &mut console,
            &snapshot,
            false,
            "work_start",
            "Start Codex",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "work_start");
        assert_eq!(commands[0].payload["expectedItemRevision"], 7);
        assert!(commands[0].payload["commandId"]
            .as_str()
            .unwrap()
            .starts_with("cmd:"));
        assert!(click(
            &ctx,
            &mut console,
            &snapshot,
            true,
            "work_start",
            "Start Codex"
        )
        .is_empty());
        snapshot["observation"]["observations"][0]["provider"]["version"] = json!("0.144.1-beta");
        assert!(click(
            &ctx,
            &mut console,
            &snapshot,
            false,
            "work_start",
            "Start Codex"
        )
        .is_empty());
    }

    #[test]
    fn workspace_form_click_emits_only_local_registration() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Projects,
            project_name: "Native fixture".into(),
            project_path: "/tmp/native-fixture".into(),
            registration_open: true,
            ..Default::default()
        };
        let mut fresh = snapshot();
        fresh["workspaces"]["items"] = json!([]);
        render(&ctx, &mut console, &fresh, false, vec![]);
        let commands = click(
            &ctx,
            &mut console,
            &fresh,
            false,
            "workspace_register",
            "Add folder",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "workspace_register");
        assert_eq!(
            commands[0].payload["workingDirectory"],
            "/tmp/native-fixture"
        );
    }

    #[test]
    fn folder_selection_is_explicit_and_cancel_preserves_the_draft() {
        let ctx = egui::Context::default();
        let mut console = Console {
            project_path: "/tmp/old-folder".into(),
            project_name: "Custom label".into(),
            ..Default::default()
        };
        console.accept_folder(folder::Target::Registration, None);
        assert_eq!(console.project_path, "/tmp/old-folder");
        assert_eq!(console.project_name, "Custom label");
        let commands = click(
            &ctx,
            &mut console,
            &snapshot(),
            false,
            "memory_control",
            "Add folder",
        );
        assert!(commands.is_empty());
        assert_eq!(
            console.take_folder_request(),
            Some(folder::Target::Registration)
        );
        console.accept_folder(
            folder::Target::Registration,
            Some(folder::Selection {
                path: "/tmp/not-a-repository".into(),
                problem: Some("WORKSPACE_NOT_GIT_ROOT".into()),
            }),
        );
        assert_eq!(console.project_name, "Custom label");
        assert!(click(
            &ctx,
            &mut console,
            &snapshot(),
            false,
            "workspace_register",
            "Add folder"
        )
        .is_empty());
        console.accept_folder(
            folder::Target::Registration,
            Some(folder::Selection {
                path: "/tmp/valid-repository".into(),
                problem: None,
            }),
        );
        let commands = click(
            &ctx,
            &mut console,
            &snapshot(),
            false,
            "workspace_register",
            "Add folder",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].payload["workingDirectory"],
            "/tmp/valid-repository"
        );
        assert_eq!(commands[0].payload["displayName"], "Custom label");
        assert!(!console.registration_open);
        assert!(commands
            .iter()
            .all(|c| c.action != "project_link_workspace"));
    }

    #[test]
    fn choosing_one_off_folder_clears_the_registered_target_and_blocks_invalid_access() {
        let mut console = Console {
            work: WorkDraft {
                title: "Task".into(),
                instruction: "One outcome".into(),
                workspace: "workspace:fixture".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        console.accept_folder(
            folder::Target::Task,
            Some(folder::Selection {
                path: "/tmp/read-only".into(),
                problem: Some("WORKSPACE_READ_ONLY".into()),
            }),
        );
        assert!(console.work.workspace.is_empty());
        assert!(console.work.payload(&snapshot()).is_err());
        console.accept_folder(folder::Target::Task, None);
        assert_eq!(console.work.path, "/tmp/read-only");
        assert!(console.work.path_problem.is_some());
        console.accept_folder(
            folder::Target::Task,
            Some(folder::Selection {
                path: "/tmp/valid-repository".into(),
                problem: None,
            }),
        );
        assert_eq!(
            console.work.payload(&snapshot()).unwrap()["workingDirectory"],
            "/tmp/valid-repository"
        );
    }

    #[test]
    fn registration_suggests_a_link_once_without_overriding_a_later_selection() {
        let ctx = egui::Context::default();
        let mut console = Console {
            project_path: "/tmp/fixture".into(),
            registration_project: Some("project:fixture".into()),
            ..Default::default()
        };
        let mut data = memory_snapshot();
        data["workspaces"]["items"].as_array_mut().unwrap().push(json!({"workspaceId":"workspace:other","displayName":"Other folder","workingDirectory":"/tmp/other","revision":1}));
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        assert_eq!(console.link_choices["project:fixture"], "workspace:fixture");
        assert!(console.registration_project.is_none());
        console
            .link_choices
            .insert("project:fixture".into(), "workspace:other".into());
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        assert_eq!(console.link_choices["project:fixture"], "workspace:other");
    }

    #[test]
    fn folder_recovery_requires_an_idle_host_but_never_a_retryable_command() {
        for ready in [false, true] {
            let ctx = egui::Context::default();
            let mut console = Console::default();
            let notice = Notice::error("workspace_register", "WORKSPACE_ACCESS_DENIED");
            let paint = |console: &mut Console, events| {
                let output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(800.0, 560.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| console.feedback(ui, &notice, ready, false),
                );
                output.drop_without_applying_deltas();
            };
            paint(&mut console, vec![]);
            paint(&mut console, vec![]);
            let rect: Rect = ctx.data(|d| {
                d.get_temp(egui::Id::new((
                    "test-action",
                    "memory_control",
                    "Choose another folder",
                )))
                .unwrap()
            });
            let pos = rect.center();
            paint(
                &mut console,
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    },
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
            assert_eq!(
                console.take_folder_request(),
                ready.then_some(folder::Target::Registration)
            );
            assert_eq!(
                matches!(console.notice_action, Some(NoticeAction::Dismiss)),
                ready
            );
        }
    }

    #[test]
    fn permission_form_fits_the_minimum_window_and_localized_error_is_actionable() {
        let ctx = egui::Context::default();
        let mut console = Console::default();
        console.accept_folder(
            folder::Target::Registration,
            Some(folder::Selection {
                path: "/tmp/read-only-repository".into(),
                problem: Some("WORKSPACE_READ_ONLY".into()),
            }),
        );
        for _ in 0..3 {
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 560.0))),
                    ..Default::default()
                },
                |ui| {
                    assert!(console.show(ui, &snapshot(), false, None, false).is_empty());
                },
            );
            output.drop_without_applying_deltas();
        }
        let rect: Rect = ctx.data(|d| {
            d.get_temp(egui::Id::new(("test-action", "memory_control", "Cancel")))
                .unwrap()
        });
        assert!(
            rect.min.x >= 0.0 && rect.max.x <= 800.0 && rect.min.y >= 0.0 && rect.max.y <= 560.0,
            "{rect:?}"
        );
        let (clip, visible): (Rect, bool) = ctx.data(|d| {
            d.get_temp(egui::Id::new(("test-action-visibility", "Cancel")))
                .unwrap()
        });
        assert!(
            visible && clip.contains_rect(rect),
            "{rect:?} clipped by {clip:?}"
        );
        for language in [Language::Tr, Language::En] {
            for code in [
                "WORKSPACE_NOT_GIT_ROOT",
                "WORKSPACE_FOLDER_NOT_FOUND",
                "WORKSPACE_ACCESS_DENIED",
                "WORKSPACE_READ_ONLY",
                "WORKSPACE_NOT_ALLOWED",
            ] {
                let message = i18n::tr(language, code);
                assert_ne!(message, code);
                assert!(!message.contains("/tmp/"));
            }
        }
    }

    #[test]
    fn draft_validation_rejects_ambiguous_project_duplicate_roles_and_bad_limits() {
        let snapshot = snapshot();
        let mut work = WorkDraft {
            title: "Task".into(),
            instruction: "One outcome".into(),
            workspace: "workspace:fixture".into(),
            path: "/tmp/another".into(),
            path_problem: None,
        };
        assert!(work.payload(&snapshot).is_err());
        work.path.clear();
        assert!(work.payload(&snapshot).is_ok());
        work.workspace = "workspace:removed".into();
        assert!(work.payload(&snapshot).is_err());
        let mut team = TeamDraft {
            name: "Fixture".into(),
            goal: "Check one outcome".into(),
            workspace: "workspace:fixture".into(),
            workers: [
                "pack:qa".into(),
                "pack:qa".into(),
                String::new(),
                String::new(),
            ],
            ..Default::default()
        };
        assert!(team.payload(&snapshot).is_err());
        team.workers[1].clear();
        assert!(team.payload(&snapshot).is_ok());
        team.concurrency = 5;
        assert!(team.payload(&snapshot).is_err());
        team.concurrency = 2;
        team.minutes = 481;
        assert!(team.payload(&snapshot).is_err());
        assert!(required(&"😀".repeat(81), 160, "too long").is_err());
        for invalid in ["/tmp/manifest.json", "null", "[]", "not JSON"] {
            assert!(parse_manifest(invalid).is_err());
        }
    }

    #[test]
    fn explicit_owner_rejection_keeps_team_revision_and_decision() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Teams,
            ..Default::default()
        };
        let mut snapshot = snapshot();
        snapshot["teams"]["items"] = json!([{"teamId":"team:fixture","name":"Owner gate","goal":"Review release","status":"waiting","revision":9,"settings":{"concurrency":1,"retryLimit":0},"agents":[],"tasks":[{"taskId":"task:fixture","title":"Release gate","status":"waiting_approval","attempt":1,"maxAttempts":1}]}]);
        let commands = click(
            &ctx,
            &mut console,
            &snapshot,
            false,
            "team_approve",
            "Reject",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["decision"], "rejected");
        assert_eq!(commands[0].payload["expectedTeamRevision"], 9);
    }

    #[test]
    fn prior_host_run_has_no_cancel_authority() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Work,
            ..Default::default()
        };
        let mut snapshot = snapshot();
        snapshot["work"]["items"][0]["status"] = json!("running");
        snapshot["work"]["runs"] = json!([{"runId":"run:old","workItemId":"work:fixture","ownerEpoch":"boot:old","status":"running","revision":4}]);
        assert!(click(
            &ctx,
            &mut console,
            &snapshot,
            false,
            "work_cancel",
            "Cancel run"
        )
        .is_empty());
    }

    #[test]
    fn malformed_cloud_state_has_recovery_guidance_without_reset_or_pair_controls() {
        let mut console = Console {
            tab: Tab::Cloud,
            ..Default::default()
        };
        let mut snapshot = snapshot();
        let pair_id = egui::Id::new(("test-action", "cloud_pair", "Pair this host"));
        let valid_ctx = egui::Context::default();
        render(&valid_ctx, &mut console, &snapshot, false, vec![]);
        assert!(valid_ctx.data(|d| d.get_temp::<Rect>(pair_id)).is_some());

        snapshot["cloud"] = json!({"paired":false,"error":true});
        let ctx = egui::Context::default();
        let (commands, shapes) = render(&ctx, &mut console, &snapshot, false, vec![]);
        assert!(commands.is_empty());
        assert!(shapes > 10);
        assert!(ctx.data(|d| d.get_temp::<Rect>(pair_id)).is_none());
        let reset_id = egui::Id::new(("test-action", "cloud_unpair", "Reset Cloud connection"));
        assert!(ctx.data(|d| d.get_temp::<Rect>(reset_id)).is_none());
    }
    fn memory_snapshot() -> Value {
        let mut snapshot = snapshot();
        snapshot["projectMemory"] = json!({"schemaVersion":3,"revision":9,"projects":[{"projectId":"project:fixture","name":"Billing sandbox","revision":9,"workspaceIds":[],"sourceCount":0}]});
        snapshot["memoryPage"] = json!({"reasonCode":"PROJECT_MEMORY_READ","projectId":"project:fixture","revision":9,"offset":0,"query":"","total":0,"sources":[]});
        snapshot
    }

    #[test]
    fn stable_project_create_and_explicit_link_unlink_preserve_target_revision() {
        let ctx = egui::Context::default();
        let mut console = Console {
            new_project_name: "Stable project".into(),
            ..Default::default()
        };
        let command = click(
            &ctx,
            &mut console,
            &snapshot(),
            false,
            "project_create",
            "Create project",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].action, "project_create");
        assert_eq!(command[0].payload["name"], "Stable project");
        let mut data = memory_snapshot();
        console
            .link_choices
            .insert("project:fixture".into(), "workspace:fixture".into());
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_link_workspace",
            "Link folder",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].payload["workspaceId"], "workspace:fixture");
        assert_eq!(command[0].payload["expectedProjectRevision"], 9);
        data["projectMemory"]["projects"][0]["workspaceIds"] = json!(["workspace:fixture"]);
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_unlink_workspace",
            "Unlink",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].payload["expectedProjectRevision"], 9);
    }

    #[test]
    fn memory_message_and_message_edit_clicks_use_scoped_current_revisions() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Memory,
            ..Default::default()
        };
        let mut data = memory_snapshot();
        console.memory.select("project:fixture");
        console.memory.title = "Owner message".into();
        console.memory.body = "Local source context".into();
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_message",
            "Add message",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].action, "project_message");
        assert_eq!(command[0].payload["projectId"], "project:fixture");
        assert_eq!(command[0].payload["expectedProjectRevision"], 9);
        data["projectMemory"]["projects"][0]["sourceCount"] = json!(1);
        data["memoryPage"]["sources"] = json!([{"sourceId":"source:fixture","projectId":"project:fixture","kind":"message","title":"Owner message","text":"Local source context","revision":3,"actorId":"local-owner","origin":{"type":"message","id":"cmd:fixture","eventId":"event:fixture"}}]);
        data["memoryPage"]["total"] = json!(1);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Edit message"
        )
        .is_empty());
        console.memory.edit.as_mut().unwrap().body = "Revised context".into();
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_source_edit",
            "Save message",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].payload["expectedSourceRevision"], 3);
        assert_eq!(command[0].payload["text"], "Revised context");
        data["memoryPage"]["sources"][0]["revision"] = json!(4);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_source_edit",
            "Save message"
        )
        .is_empty());
        data["memoryPage"]["sources"][0]["revision"] = json!(3);
        data["memoryPage"]["sources"][0]["kind"] = json!("work_instruction");
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_source_edit",
            "Save message"
        )
        .is_empty());
    }

    #[test]
    fn memory_reader_does_not_spin_on_failure_and_busy_does_not_consume_request() {
        let project = json!({"projectId":"project:fixture","revision":9});
        let mut memory = MemoryDraft::default();
        memory.select("project:fixture");
        assert!(memory.next_read(&project, &Value::Null, true).is_none());
        assert!(memory.requested.is_none());
        let first = memory.next_read(&project, &Value::Null, false).unwrap();
        assert_eq!(first.payload["limit"], 10);
        assert!(memory.next_read(&project, &Value::Null, false).is_none());
        memory.refresh_requested = true;
        assert!(memory.next_read(&project, &Value::Null, false).is_some());
        let changed = json!({"projectId":"project:fixture","revision":10});
        assert!(memory.next_read(&changed, &Value::Null, false).is_some());
        memory.edit = Some(SourceEdit {
            project: "project:fixture".into(),
            source: "source:fixture".into(),
            revision: 1,
            title: "Private title".into(),
            body: "Old project draft".into(),
        });
        memory.select("project:other");
        assert!(memory.edit.is_none());
        assert!(memory.title.is_empty());
        assert!(memory.query.is_empty());
        assert!(memory.requested.is_none());
    }

    #[test]
    fn keyword_filter_and_page_controls_emit_bounded_reads_and_hide_stale_pages() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Memory,
            ..Default::default()
        };
        let mut data = memory_snapshot();
        console.memory.select("project:fixture");
        console.memory.keyword_input = "billing".into();
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Apply keyword",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].action, "project_memory_read");
        assert_eq!(command[0].payload["query"], "billing");
        assert_eq!(command[0].payload["offset"], 0);
        assert!(!console
            .memory
            .page_matches(&data["memoryPage"], &data["projectMemory"]["projects"][0]));
        data["memoryPage"]["query"] = json!("billing");
        data["memoryPage"]["total"] = json!(25);
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Next sources",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].payload["offset"], 10);
        assert_eq!(command[0].payload["limit"], 10);
        data["memoryPage"]["offset"] = json!(10);
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Previous sources",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].payload["offset"], 0);
    }

    #[test]
    fn opening_other_project_drops_edit_and_reads_its_scope_once_then_refresh_retries() {
        let ctx = egui::Context::default();
        let mut console = Console::default();
        let mut data = memory_snapshot();
        data["projectMemory"]["projects"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "projectId":"project:other","name":"Other project","revision":4,
                "workspaceIds":[],"sourceCount":2
            }));
        console.memory.select("project:fixture");
        console.memory.title = "Old draft".into();
        console.memory.edit = Some(SourceEdit {
            project: "project:fixture".into(),
            source: "source:fixture".into(),
            revision: 3,
            title: "Old title".into(),
            body: "Old text".into(),
        });
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Open memory"
        )
        .is_empty());
        assert!(console.tab == Tab::Memory);
        assert_eq!(console.memory.project, "project:other");
        assert!(console.memory.edit.is_none());
        assert!(console.memory.title.is_empty());
        let command = render(&ctx, &mut console, &data, false, vec![]).0;
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].action, "project_memory_read");
        assert_eq!(command[0].payload["projectId"], "project:other");
        assert_eq!(command[0].payload["offset"], 0);
        assert_eq!(command[0].payload["limit"], 10);
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        let retry = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Refresh memory",
        );
        assert_eq!(retry.len(), 1);
        assert_eq!(retry[0].payload["projectId"], "project:other");
        data["memoryPage"] = json!({"reasonCode":"PROJECT_MEMORY_READ","projectId":"project:other","revision":4,"offset":0,"query":"","total":0,"sources":[]});
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        let refresh = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Refresh memory",
        );
        assert_eq!(
            refresh.len(),
            1,
            "a valid cached page can be refreshed explicitly"
        );
    }

    #[test]
    fn preloaded_memory_page_renders_without_read_and_stale_query_disables_edit() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Memory,
            ..Default::default()
        };
        let mut data = memory_snapshot();
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        assert_eq!(console.memory.project, "project:fixture");
        data["memoryPage"]["sources"] = json!([{"sourceId":"source:fixture","projectId":"project:fixture","kind":"message","title":"Message","text":"Context","revision":3}]);
        console.memory.edit = Some(SourceEdit {
            project: "project:fixture".into(),
            source: "source:fixture".into(),
            revision: 3,
            title: "Message".into(),
            body: "Updated context".into(),
        });
        console.memory.query = "other keyword".into();
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_source_edit",
            "Save message",
        );
        assert!(!commands.iter().any(|c| c.action == "project_source_edit"));
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Cancel edit"
        )
        .is_empty());
        assert!(console.memory.edit.is_none());
    }

    #[test]
    fn memory_filter_controls_fit_the_minimum_native_window() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Memory,
            ..Default::default()
        };
        let data = memory_snapshot();
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 560.0));
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                assert!(console.show(ui, &data, false, None, false).is_empty());
            },
        );
        output.drop_without_applying_deltas();
        for label in ["Apply keyword", "Refresh memory"] {
            let rect: Rect = ctx.data(|d| {
                d.get_temp(egui::Id::new(("test-action", "memory_control", label)))
                    .unwrap()
            });
            assert!(
                screen.contains_rect(rect),
                "{label} must stay reachable in the minimum window"
            );
        }
    }

    fn painted_copy(ctx: &egui::Context, console: &mut Console, data: &Value) -> Vec<String> {
        fn collect(shape: &egui::Shape, copy: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => copy.push(text.galley.text().to_owned()),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, copy)
                    }
                }
                _ => {}
            }
        }
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 880.0))),
                ..Default::default()
            },
            |ui| {
                console.show(ui, data, false, None, false);
            },
        );
        let mut copy = Vec::new();
        for shape in &output.shapes {
            collect(&shape.shape, &mut copy);
        }
        output.drop_without_applying_deltas();
        copy
    }

    #[test]
    fn language_switch_changes_controls_and_keeps_owner_key_collisions_and_drafts_raw() {
        let ctx = egui::Context::default();
        let mut console = Console {
            tab: Tab::Memory,
            ..Default::default()
        };
        let mut data = memory_snapshot();
        data["desktopPreferences"] = json!({"language":"tr","error":false});
        data["projectMemory"]["projects"][0]["name"] = json!("Work");
        data["projectMemory"]["projects"][0]["sourceCount"] = json!(1);
        data["memoryPage"]["total"] = json!(1);
        data["memoryPage"]["sources"] = json!([{"sourceId":"source:locale","projectId":"project:fixture","kind":"message","revision":2,"title":"Memory","text":"Work"}]);
        let copy = painted_copy(&ctx, &mut console, &data);
        assert!(copy.iter().any(|v| v == "Hafıza"));
        assert!(copy.iter().any(|v| v == "Anahtar kelime filtresi"));
        assert!(copy.iter().any(|v| v == "Memory"), "source title stays raw");
        assert!(
            copy.iter().filter(|v| *v == "Work").count() >= 2,
            "project name and source text stay raw"
        );
        console.memory.title = "Projects".into();
        console.memory.body = "Memory".into();
        let command = click(
            &ctx,
            &mut console,
            &data,
            false,
            "desktop_language",
            "English",
        );
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].action, "desktop_language");
        assert_eq!(command[0].payload["language"], "en");
        data["desktopPreferences"]["language"] = json!("en");
        let copy = painted_copy(&ctx, &mut console, &data);
        assert!(copy.iter().any(|v| v == "Keyword filter"));
        assert!(!copy.iter().any(|v| v == "Anahtar kelime filtresi"));
        assert_eq!(console.memory.title, "Projects");
        assert_eq!(console.memory.body, "Memory");
        assert_eq!(data["memoryPage"]["sources"][0]["title"], "Memory");
        assert_eq!(data["memoryPage"]["sources"][0]["text"], "Work");
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "desktop_language",
            "Türkçe"
        )
        .is_empty());
    }

    #[test]
    fn corrupted_language_preference_can_be_explicitly_replaced_by_current_default() {
        let ctx = egui::Context::default();
        let mut console = Console::default();
        let mut data = snapshot();
        data["desktopPreferences"] = json!({"language":"tr","error":true});
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "desktop_language",
            "Türkçe",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["language"], "tr");
        data["desktopPreferences"]["error"] = json!(false);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "desktop_language",
            "Türkçe"
        )
        .is_empty());
    }

    #[test]
    fn built_in_catalog_translation_never_changes_a_custom_pack_with_the_same_name() {
        let ctx = egui::Context::default();
        let output=ctx.run_ui(RawInput::default(),|ui| {
            ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new("console-language"),Language::Tr));
            let custom=json!({"id":"pack:custom","name":"Orchestrator","description":"Memory","provenance":{"kind":"local"}});
            assert_eq!(catalog_text(ui,&custom,"name"),"Orchestrator");
            assert_eq!(catalog_text(ui,&custom,"description"),"Memory");
            let builtin=json!({"id":"pack:orchestrator","name":"Orchestrator","provenance":{"kind":"built-in"}});
            assert_eq!(catalog_text(ui,&builtin,"name"),"Koordinatör");
        });
        output.drop_without_applying_deltas();
    }
}
