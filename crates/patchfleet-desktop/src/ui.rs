use crate::command::{Command, Notice, NoticeKind};
use crate::folder;
use crate::i18n::{self, Language};
use crate::studio;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::{json, Value};
use std::collections::HashMap;

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
    #[cfg(test)]
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
            Self::Work => "Tasks",
            Self::Teams => "Teams",
            Self::Agents => "Agents",
            Self::Providers => "Providers",
            Self::Cloud => "Cloud",
        }
    }
    fn group(self) -> &'static str {
        match self {
            Self::Projects | Self::Work | Self::Teams => "WORKSPACE",
            Self::Memory | Self::Decisions => "KNOWLEDGE",
            Self::Agents | Self::Providers | Self::Cloud => "SYSTEM",
        }
    }
    fn summary(self) -> &'static str {
        match self {
            Self::Projects => "Your projects, local folders and shared memory.",
            Self::Memory => "Keep project context, sources and decision drafts together.",
            Self::Decisions => "Sourced local decisions. Acceptance stays in your hands.",
            Self::Work => "Give your agents a clear task. Review the result before moving on.",
            Self::Teams => "Bring your agents together around one goal. Keep control of decisions.",
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
    extraction_revision: Option<u64>,
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
    fn observe_extraction_revision(&mut self, revision: u64) {
        if self.extraction_revision != Some(revision) {
            self.extraction_revision = Some(revision);
            self.requested = None;
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
    pending_proposal: Option<(u64, Value)>,
    title: String,
    context: String,
    decision: String,
    rationale: String,
    consequences: String,
    sources: MemoryDraft,
    citation_epoch: u64,
    citations: Vec<Citation>,
    sources_changed: bool,
    search: SearchPanel,
}
struct Citation {
    source: String,
    revision: u64,
    title: String,
}

struct SearchPanel {
    input: String,
    limit: u64,
    submitted_limit: u64,
}
impl Default for SearchPanel {
    fn default() -> Self {
        Self {
            input: String::new(),
            limit: 5,
            submitted_limit: 5,
        }
    }
}
impl SearchPanel {
    fn payload(&self, project: &str) -> Option<Value> {
        let query = self.input.trim();
        (!project.is_empty() && decision_text(query, 400, true) && (1..=10).contains(&self.limit))
            .then(|| json!({"projectId":project,"query":query,"limit":self.limit}))
    }
    fn matches(&self, result: &Value, project: &str, manifest: u64, shared: bool) -> bool {
        self.payload(project).is_some()
            && text(result, "reasonCode")
                == if shared {
                    "SHARED_MEMORY_SEARCH_READ"
                } else {
                    "DECISION_SEARCH_READ"
                }
            && text(result, "projectId") == project
            && revision(result) == manifest
            && text(result, "query") == self.input.trim()
            && self.limit == self.submitted_limit
            && (!result["limit"].is_number() || result["limit"] == self.limit)
    }
}

#[derive(Default)]
struct SharedPanel {
    project: String,
    name: String,
    link_id: String,
    offset: u64,
    selected: String,
    selected_revision: u64,
    exact_id: String,
    exact_revision: u64,
    page_requested: Option<(String, u64, u64)>,
    detail_requested: Option<(String, u64, String, u64)>,
    refresh_page: bool,
    refresh_detail: bool,
    preview_open: bool,
    consent_scope: Option<Value>,
    consent: bool,
    search: SearchPanel,
    search_open: bool,
}
impl SharedPanel {
    fn select(&mut self, project: &str) {
        if self.project != project {
            *self = Self {
                project: project.into(),
                exact_revision: 1,
                ..Default::default()
            };
        }
    }
    fn page_matches(&self, page: &Value, metadata: &Value, global: u64) -> bool {
        text(page, "reasonCode") == "SHARED_MEMORY_READ"
            && text(page, "projectId") == self.project
            && revision(page) == global
            && page["sharedProjectId"] == metadata["sharedProjectId"]
            && page["offset"].as_u64() == Some(self.offset)
            && page["limit"].as_u64() == Some(10)
    }
    fn detail_matches(&self, detail: &Value, metadata: &Value, global: u64) -> bool {
        text(detail, "reasonCode") == "SHARED_DECISION_READ"
            && text(detail, "projectId") == self.project
            && revision(detail) == global
            && detail["sharedProjectId"] == metadata["sharedProjectId"]
            && text(&detail["decision"], "decisionId") == self.selected
            && revision(&detail["decision"]) == self.selected_revision
    }
    fn next_read(
        &mut self,
        page: &Value,
        detail: &Value,
        metadata: &Value,
        global: u64,
        blocked: bool,
    ) -> Option<Command> {
        if blocked || self.project.is_empty() || !metadata["sharedProjectId"].is_string() {
            return None;
        }
        let key = (self.project.clone(), global, self.offset);
        if (self.refresh_page || !self.page_matches(page, metadata, global))
            && (self.refresh_page || self.page_requested.as_ref() != Some(&key))
        {
            self.refresh_page = false;
            self.page_requested = Some(key);
            return Some(Command::new(
                "memory_cloud_read",
                json!({"projectId":self.project,"offset":self.offset,"limit":10}),
            ));
        }
        let key = (
            self.project.clone(),
            global,
            self.selected.clone(),
            self.selected_revision,
        );
        if !self.selected.is_empty()
            && self.selected_revision > 0
            && (self.refresh_detail || !self.detail_matches(detail, metadata, global))
            && (self.refresh_detail || self.detail_requested.as_ref() != Some(&key))
        {
            self.refresh_detail = false;
            self.detail_requested = Some(key);
            return Some(Command::new(
                "memory_cloud_decision_read",
                json!({"projectId":self.project,"decisionId":self.selected,"revision":self.selected_revision}),
            ));
        }
        None
    }
}
impl DecisionPanel {
    fn finish_proposal(&mut self, snapshot: &Value, notice: Option<&Notice>, busy: bool) {
        if !busy
            && self
                .pending_proposal
                .as_ref()
                .is_some_and(|(before, _)| decision_manifest(snapshot, &self.project) > *before)
            && notice.is_some_and(|n| {
                n.action == "project_decision_propose"
                    && n.kind == NoticeKind::Success
                    && n.code == "PROJECT_DECISION_PROPOSED"
            })
        {
            let (_, submitted) = self.pending_proposal.take().unwrap();
            let current = array(&snapshot["projectMemory"]["projects"])
                .iter()
                .find(|p| text(p, "projectId") == self.project)
                .and_then(|project| self.proposal(project).ok());
            // An exact retry may succeed after the owner has already edited a new draft.
            if current.as_ref() != Some(&submitted) {
                return;
            }
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
struct ExtractionPanel {
    project: String,
    policy_revision: u64,
    setup_open: bool,
    consent: bool,
}
impl ExtractionPanel {
    fn scope(&mut self, project: &str, policy_revision: u64) {
        if self.project != project || self.policy_revision != policy_revision {
            *self = Self {
                project: project.into(),
                policy_revision,
                ..Default::default()
            };
        }
    }
}

#[derive(Default)]
pub struct Console {
    tab: Tab,
    navigation_tab: Option<Tab>,
    new_project_name: String,
    link_choices: HashMap<String, String>,
    memory: MemoryDraft,
    decisions: DecisionPanel,
    extraction: ExtractionPanel,
    shared: SharedPanel,
    #[cfg(feature = "ui-smoke")]
    shared_smoke_focus: bool,
    #[cfg(feature = "ui-smoke")]
    search_smoke_query: Option<(bool, String)>,
    #[cfg(feature = "ui-smoke")]
    suppress_account_browser: bool,
    project_name: String,
    project_path: String,
    project_path_problem: Option<String>,
    registration_open: bool,
    new_project_open: bool,
    registration_project: Option<String>,
    folder_request: Option<folder::Target>,
    pub notice_action: Option<NoticeAction>,
    hold_automatic_reads: bool,
    work: WorkDraft,
    team: TeamDraft,
    manifest: String,
    cloud_url: String,
    host_name: String,
    pairing_code: String,
    account_advanced: bool,
    account_form_error: Option<&'static str>,
    opened_login_command: Option<String>,
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
    pub fn studio_smoke(&mut self, scene: &str, snapshot: &Value) {
        self.tab = match scene {
            "studio-memory" => Tab::Memory,
            "studio-work" => Tab::Work,
            "studio-teams" => Tab::Teams,
            "studio-agents" => Tab::Agents,
            "studio-providers" => Tab::Providers,
            _ => Tab::Projects,
        };
        if let Some(project) = array(&snapshot["projectMemory"]["projects"]).first() {
            self.memory.select(text(project, "projectId"));
        }
        if let Some(workspace) = items(&snapshot["workspaces"]).first() {
            self.work.workspace = text(workspace, "workspaceId").to_owned();
            self.team.workspace = text(workspace, "workspaceId").to_owned();
        }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn account_smoke(&mut self, cloud_url: Option<&str>) {
        self.tab = Tab::Cloud;
        if let Some(url) = cloud_url {
            self.cloud_url = url.into();
        }
        self.suppress_account_browser = true;
    }
    #[cfg(feature = "ui-smoke")]
    pub fn memory_smoke(&mut self) {
        self.tab = Tab::Memory;
        self.extraction.setup_open = true;
    }

    #[cfg(feature = "ui-smoke")]
    pub fn decision_smoke(&mut self) {
        self.tab = Tab::Decisions;
    }

    #[cfg(feature = "ui-smoke")]
    pub fn shared_smoke(&mut self) {
        self.tab = Tab::Cloud;
        self.shared_smoke_focus = true;
    }

    #[cfg(feature = "ui-smoke")]
    pub fn publication_smoke(&mut self) {
        self.tab = Tab::Decisions;
        self.shared.preview_open = true;
    }

    #[cfg(feature = "ui-smoke")]
    pub fn search_smoke(&mut self, shared: bool, query: &str) {
        self.tab = if shared { Tab::Cloud } else { Tab::Decisions };
        self.search_smoke_query = Some((shared, query.into()));
        if shared {
            self.shared.search_open = true;
        }
    }

    #[cfg(feature = "ui-smoke")]
    fn initialize_search_smoke(&mut self, shared: bool) {
        let project = if shared {
            &self.shared.project
        } else {
            &self.decisions.project
        };
        if project.is_empty()
            || !self
                .search_smoke_query
                .as_ref()
                .is_some_and(|(scope, _)| *scope == shared)
        {
            return;
        }
        let (_, query) = self.search_smoke_query.take().unwrap();
        if shared {
            self.shared.search.input = query;
            self.shared.search_open = true;
        } else {
            self.decisions.search.input = query;
        }
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
        self.open_login_browser(ui.ctx(), snapshot);
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
        self.hold_automatic_reads = can_retry
            && notice.is_some_and(|n| {
                n.kind == NoticeKind::Error
                    && !matches!(
                        n.action,
                        "project_memory_read"
                            | "project_decisions_read"
                            | "project_decision_read"
                            | "memory_cloud_read"
                            | "memory_cloud_decision_read"
                            | "project_decision_search"
                            | "memory_cloud_search"
                    )
            });
        self.decisions.finish_proposal(snapshot, notice, busy);
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new("console-language"),
                Language::from_snapshot(snapshot),
            )
        });
        ui.style_mut().scroll_animation = egui::style::ScrollAnimation::none();
        let mut commands = Vec::new();
        let reveal_navigation = self.navigation_tab != Some(self.tab);
        self.navigation_tab = Some(self.tab);
        egui::Panel::left("console-navigation")
            .resizable(false)
            .default_size(216.0)
            .frame(egui::Frame::new().fill(colors(ui).surface).inner_margin(16))
            .show(ui, |ui| {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
                    studio::mark(ui.painter(), rect);
                    ui.vertical(|ui| {
                        ui.label(studio::title("Moxcrew", 18.0));
                        ui.label(
                            RichText::new(t(ui, "LOCAL STUDIO"))
                                .size(10.0)
                                .color(colors(ui).muted)
                                .extra_letter_spacing(1.8),
                        );
                    });
                });
                ui.add_space(25.0);
                let account_footer =
                    if verified_account(snapshot).is_some_and(|a| a["email"].is_string()) {
                        64.0
                    } else {
                        48.0
                    };
                let footer_height = if snapshot["desktopPreferences"]["error"] == true {
                    210.0 + account_footer
                } else {
                    126.0 + account_footer
                };
                let nav_height = (ui.available_height() - footer_height).max(32.0);
                egui::ScrollArea::vertical()
                    .id_salt("sidebar-navigation")
                    .max_height(nav_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (group, tabs) in [
                            ("WORKSPACE", &[Tab::Projects, Tab::Work, Tab::Teams][..]),
                            ("KNOWLEDGE", &[Tab::Memory, Tab::Decisions][..]),
                            ("SYSTEM", &[Tab::Agents, Tab::Providers, Tab::Cloud][..]),
                        ] {
                            ui.add_space(7.0);
                            ui.label(
                                RichText::new(t(ui, group))
                                    .size(10.0)
                                    .color(colors(ui).muted)
                                    .extra_letter_spacing(1.4),
                            );
                            ui.add_space(6.0);
                            for &tab in tabs {
                                let count = match tab {
                                    Tab::Projects => {
                                        array(&snapshot["projectMemory"]["projects"]).len()
                                    }
                                    Tab::Decisions => {
                                        array(&snapshot["projectDecisions"]["projects"])
                                            .iter()
                                            .map(|p| {
                                                p["decisionCount"].as_u64().unwrap_or(0) as usize
                                            })
                                            .sum()
                                    }
                                    Tab::Work => items(&snapshot["work"]).len(),
                                    Tab::Teams => items(&snapshot["teams"]).len(),
                                    _ => 0,
                                };
                                if navigation(ui, tab, self.tab == tab, count, reveal_navigation) {
                                    self.tab = tab;
                                    self.form_error = None;
                                }
                                ui.add_space(2.0);
                            }
                            ui.add_space(10.0);
                        }
                    });
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::bottom_up(egui::Align::LEFT),
                    |ui| {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let (dot, _) =
                                ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                            ui.painter()
                                .circle_filled(dot.center(), 2.5, colors(ui).primary);
                            ui.label(
                                RichText::new(t(ui, "Local engine"))
                                    .size(10.0)
                                    .color(colors(ui).muted),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(text(snapshot, "appVersion"))
                                            .size(11.0)
                                            .monospace()
                                            .color(colors(ui).muted),
                                    );
                                },
                            );
                        });
                        ui.add_space(9.0);
                        language_selector(ui, snapshot, busy, &mut commands);
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);
                        if let Some(email) =
                            verified_account(snapshot).and_then(|a| a["email"].as_str())
                        {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(email).size(10.0).color(colors(ui).muted),
                                )
                                .truncate(),
                            );
                        }
                        if control(
                            ui,
                            if snapshot["cloud"]["paired"] == true {
                                "Your account"
                            } else {
                                "Sign in · optional"
                            },
                            true,
                        )
                        .clicked()
                        {
                            self.tab = Tab::Cloud;
                        }
                    },
                );
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(colors(ui).canvas)
                    .inner_margin(egui::Margin::symmetric(28, 20)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t(ui, self.tab.group()))
                            .size(11.0)
                            .color(colors(ui).muted)
                            .extra_letter_spacing(1.2),
                    );
                    ui.label(RichText::new("/").size(10.0).color(colors(ui).border));
                    ui.label(
                        RichText::new(t(ui, self.tab.name()))
                            .size(10.0)
                            .color(colors(ui).text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if busy {
                            ui.label(
                                RichText::new(t(ui, "Applying action…"))
                                    .size(10.0)
                                    .color(colors(ui).primary),
                            );
                        } else {
                            ui.label(
                                RichText::new(t(ui, "Free · Local"))
                                    .size(11.0)
                                    .color(colors(ui).muted),
                            );
                        }
                    });
                });
                ui.add_space(if ui.ctx().viewport_rect().height() < 700.0 {
                    6.0
                } else {
                    18.0
                });
                ui.horizontal(|ui| {
                    ui.label(studio::title(
                        t(ui, self.tab.name()),
                        if ui.ctx().viewport_rect().height() < 700.0 {
                            25.0
                        } else {
                            30.0
                        },
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.tab == Tab::Projects {
                            if !array(&snapshot["projectMemory"]["projects"]).is_empty()
                                && control(ui, "New project", !busy).clicked()
                            {
                                self.new_project_open = true;
                                self.form_error = None;
                            }
                            if control(ui, "Add folder", !busy).clicked() {
                                self.registration_project = None;
                                self.folder_request = Some(folder::Target::Registration);
                                self.form_error = None;
                            }
                        }
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
                    });
                });
                ui.add_space(3.0);
                ui.label(
                    RichText::new(i18n::tr_for_snapshot(snapshot, self.tab.summary()))
                        .size(12.0)
                        .color(colors(ui).muted),
                );
                ui.add_space(if ui.ctx().viewport_rect().height() < 700.0 {
                    8.0
                } else {
                    18.0
                });
                ui.separator();
                ui.add_space(if ui.ctx().viewport_rect().height() < 700.0 {
                    12.0
                } else {
                    20.0
                });
                if let Some(notice) = notice {
                    self.feedback(ui, notice, !busy, can_retry);
                    ui.add_space(14.0);
                }
                if let Some(error) = self.form_error {
                    ui.colored_label(colors(ui).danger, t(ui, error));
                    ui.add_space(8.0);
                }
                if self.hold_automatic_reads
                    && matches!(self.tab, Tab::Memory | Tab::Decisions | Tab::Cloud)
                {
                    hint(ui, "Retry or dismiss the failed action before refreshing.");
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .id_salt(("console-body", self.tab.name()))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width().min(if self.tab == Tab::Memory {
                            980.0
                        } else {
                            1180.0
                        }));
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
        if self.shared.search_open {
            let id = egui::Id::new("shared-decision-search");
            let mut close = false;
            let modal = egui::Modal::new(id)
                .area(egui::Modal::default_area(id).fade_in(false))
                .show(ui.ctx(), |ui| {
                    ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 620.0));
                    let start = ui.next_widget_position().y;
                    if let Some(notice) = notice.filter(|n| n.kind == NoticeKind::Error) {
                        self.feedback(ui, notice, !busy, can_retry);
                    }
                    let feedback_height = ui.next_widget_position().y - start;
                    egui::ScrollArea::vertical()
                        .id_salt("shared-search-body")
                        .max_height(
                            (ui.ctx().viewport_rect().height() - 140.0 - feedback_height)
                                .max(120.0),
                        )
                        .show(ui, |ui| {
                            ui.add_enabled_ui(!busy, |ui| {
                                self.memory_search(ui, snapshot, true, &mut commands)
                            });
                        });
                    close = control(ui, "Close search", true).clicked();
                });
            if close || modal.should_close() {
                self.shared.search_open = false;
            }
        }
        commands
    }

    fn feedback(&mut self, ui: &mut Ui, notice: &Notice, ready: bool, can_retry: bool) {
        if notice.kind == NoticeKind::Success {
            egui::Frame::new()
                .fill(colors(ui).success_surface)
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
                            .color(colors(ui).success),
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
            NoticeKind::Success => colors(ui).success,
            NoticeKind::Warning => colors(ui).warning,
            NoticeKind::Error => colors(ui).danger,
        };
        egui::Frame::new().fill(if notice.kind == NoticeKind::Warning { colors(ui).warning_surface } else { colors(ui).danger_surface }).stroke(egui::Stroke::new(1.0,color)).corner_radius(8).inner_margin(12).show(ui,|ui| {
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
            if ui.ctx().viewport_rect().height() >= 700.0 {
                ui.label(
                    RichText::new(t(ui, "YOUR LOCAL WORKSPACE"))
                        .size(10.0)
                        .color(colors(ui).primary),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t(ui, "Create your first project"))
                        .size(24.0)
                        .strong()
                        .color(colors(ui).text),
                );
                hint(ui, "Keep your folders, agents and project memory together.");
                ui.add_space(16.0);
            }
            if ui.available_width() > 800.0 {
                ui.columns(2, |columns| {
                    self.create_project_form(&mut columns[0], commands);
                    studio_flow(&mut columns[1]);
                });
            } else {
                ui.horizontal(|ui| {
                    studio::mascot(ui, 104.0);
                    ui.vertical(|ui| {
                        ui.label(studio::title(t(ui, "Meet Mox. Your crew is ready."), 18.0));
                        hint(
                            ui,
                            "Start with one project. Your work stays on this device.",
                        );
                    });
                });
                ui.add_space(12.0);
                constrained(ui, 560.0, |ui| self.create_project_form(ui, commands));
            }
        } else {
            let accepted = array(&snapshot["projectDecisions"]["projects"])
                .iter()
                .map(|p| p["acceptedCount"].as_u64().unwrap_or(0))
                .sum::<u64>();
            ui.columns(3, |columns| {
                for (column, (label, count, icon)) in columns.iter_mut().zip([
                    ("Projects", projects.len() as u64, Tab::Projects),
                    ("Local folders", workspaces.len() as u64, Tab::Work),
                    ("Accepted decisions", accepted, Tab::Decisions),
                ]) {
                    studio_metric(column, label, count, icon);
                }
            });
            ui.add_space(22.0);
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
            });
            ui.add_space(14.0);
            let columns = if ui.available_width() >= 940.0 && projects.len() > 1 {
                2
            } else {
                1
            };
            for row in projects.chunks(columns) {
                ui.columns(columns, |columns| {
                    for (project, ui) in row.iter().zip(columns.iter_mut()) {
                ui.push_id(text(project,"projectId"), |ui| card(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 8, colors(ui).info_surface);
                        draw_icon(ui.painter(), rect.shrink(8.0), Tab::Projects, colors(ui).primary);
                        let name_width = (ui.available_width() - 116.0).max(80.0);
                        ui.allocate_ui_with_layout(egui::vec2(name_width, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            ui.set_max_width(name_width);
                            ui.add(egui::Label::new(studio::title(text(project,"name"), 20.0)).truncate()).on_hover_text(text(project,"name"));
                            hint(ui, &localized_format!(ui,"{0} folders · {1} sources",array(&project["workspaceIds"]).len(),project["sourceCount"].as_u64().unwrap_or(0)));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if control(ui,"Open memory",true).clicked() { self.memory.select(text(project,"projectId")); self.tab = Tab::Memory; }
                        });
                    });
                    ui.add_space(14.0);
                    if array(&project["workspaceIds"]).is_empty() {
                        egui::Frame::new().fill(colors(ui).raised).corner_radius(8).inner_margin(12).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.label(RichText::new(t(ui,"Connect a local folder")).strong().color(colors(ui).text));
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
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        if control(ui,"Open decisions",true).clicked() {
                            self.decisions.select(text(project,"projectId"));
                            self.tab = Tab::Decisions;
                        }
                        let workspace = array(&project["workspaceIds"]).iter()
                            .find_map(|id| workspaces.iter().find(|w| w["workspaceId"] == *id));
                        let new_task = self.work.title.is_empty() && self.work.instruction.is_empty();
                        if control(ui,if new_task { "New task" } else { "Open task draft" },workspace.is_some()).clicked() {
                            if new_task {
                                self.work.workspace = text(workspace.unwrap(),"workspaceId").to_owned();
                                self.work.path.clear();
                                self.work.path_problem = None;
                            }
                            self.tab = Tab::Work;
                            self.form_error = None;
                        }
                    });
                }));
                ui.add_space(12.0);
                    }
                });
            }
        }
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t(ui, "LOCAL FOLDERS"))
                    .size(10.0)
                    .color(colors(ui).muted),
            );
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
                        ui.label(RichText::new(text(workspace,"displayName")).strong().color(colors(ui).text));
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
                "e.g. Moxcrew, Website, Mobile app",
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
            ui.colored_label(colors(ui).danger, t(ui, code));
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
            ui.colored_label(colors(ui).danger, t(ui, error));
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
            self.extraction.consent = false;
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
                "Sources are evidence. Only owner-accepted decisions become active.",
            );
        });
        ui.add_space(16.0);
        if self.memory.project.is_empty() && projects.len() == 1 {
            self.memory.select(text(&projects[0], "projectId"));
        }
        let previous = self.memory.project.clone();
        project_switcher(ui, "memory-project", &mut self.memory.project, projects);
        if self.memory.project != previous {
            let selected = self.memory.project.clone();
            self.memory.project = previous;
            self.memory.select(&selected);
        }
        let Some(project) = projects
            .iter()
            .find(|p| text(p, "projectId") == self.memory.project)
        else {
            self.extraction.consent = false;
            hint(
                ui,
                "Choose a project to read or write its local source context.",
            );
            return;
        };
        self.extraction_controls(ui, snapshot, project, commands);
        let extraction_revision = revision(&snapshot["projectExtraction"]);
        self.memory.observe_extraction_revision(extraction_revision);
        let page = &snapshot["memoryPage"];
        let extraction_page_current = page["extractionRevision"].as_u64()
            == Some(extraction_revision)
            || (snapshot["projectExtraction"].is_null() && page["extractionRevision"].is_null());
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(t(ui, "Keyword filter"))
                    .size(12.0)
                    .color(colors(ui).muted),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.memory.keyword_input)
                    .desired_width(230.0)
                    .font(egui::TextStyle::Body)
                    .margin(egui::Margin::symmetric(10, 7))
                    .char_limit(160)
                    .hint_text(t(ui, "Words in source titles or text")),
            );
            if control(ui, "Apply keyword", !self.hold_automatic_reads).clicked() {
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
            if control(ui, "Refresh memory", !self.hold_automatic_reads).clicked() {
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
                hint(ui,"Sources are stored locally. Enabled drafting sends sources and relevant accepted Local decisions to Codex; adding one never accepts an ADR.");
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
                if control(
                    ui,
                    "Previous sources",
                    self.memory.offset > 0 && !self.hold_automatic_reads,
                )
                .clicked()
                {
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
                    self.memory.offset.saturating_add(10) < total && !self.hold_automatic_reads,
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
                                    .color(colors(ui).text),
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
                        if let Some((_,enabled))=extraction_policy(snapshot,&self.memory.project) {
                            if text(source,"projectId")==self.memory.project && revision(source)>0 && matches!(text(source,"kind"),"message"|"work_instruction"|"team_goal"|"owner_answer"|"approval_note") {
                                let job=match source.get("extractionJob") {
                                    Some(value)=>(!value.is_null()).then_some(value),
                                    None=>extraction_jobs(snapshot,&self.memory.project).iter().find(|job|job["projectId"]==source["projectId"]&&job["sourceId"]==source["sourceId"]&&job["sourceRevision"]==source["revision"]),
                                };
                                let job_scoped=job.is_none_or(|job|job["projectId"]==source["projectId"]&&job["sourceId"]==source["sourceId"]&&job["sourceRevision"]==source["revision"]);
                                if !extraction_page_current || !job_scoped {
                                    hint(ui,"Refresh memory before drafting or retrying this source; its job status changed.");
                                } else if let Some(job)=job {
                                    extraction_job_controls(ui,job,&self.memory.project,enabled,commands);
                                } else if enabled {
                                    action(ui,commands,"Draft decision from this source","project_extraction_enqueue",true,||json!({"projectId":self.memory.project,"sourceId":source["sourceId"],"expectedSourceRevision":revision(source)}));
                                }
                            }
                        }
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
            if let Some(command) = self.memory.next_read(
                project,
                if extraction_page_current {
                    page
                } else {
                    &Value::Null
                },
                !ui.is_enabled() || self.hold_automatic_reads,
            ) {
                commands.push(command);
            }
        }
    }

    fn extraction_controls(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        project: &Value,
        commands: &mut Vec<Command>,
    ) {
        let project_id = text(project, "projectId");
        let Some((policy_revision, enabled)) = extraction_policy(snapshot, project_id) else {
            self.extraction.consent = false;
            hint(
                ui,
                "Drafting settings are unavailable. Refresh this view before changing them.",
            );
            ui.add_space(10.0);
            return;
        };
        #[cfg(feature = "ui-smoke")]
        let initial_smoke_setup = self.extraction.project.is_empty() && self.extraction.setup_open;
        self.extraction.scope(project_id, policy_revision);
        #[cfg(feature = "ui-smoke")]
        if initial_smoke_setup {
            self.extraction.setup_open = true;
        }
        if enabled {
            self.extraction.consent = false;
            self.extraction.setup_open = false;
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.strong(t(ui,"Automatic ADR drafts"));
            status(ui,if enabled {"Drafting enabled"} else {"Drafting off"});
            if enabled {
                let before=commands.len();
                action(ui,commands,"Pause drafting","project_extraction_configure",true,||json!({"projectId":project_id,"expectedPolicyRevision":policy_revision,"enabled":false,"consent":false}));
                if commands.len()>before {self.extraction.consent=false;self.extraction.setup_open=false;}
            } else if control(ui,"Set up drafting",true).clicked() {
                self.extraction.setup_open = !self.extraction.setup_open;
                self.extraction.consent=false;
            }
        });
        if !enabled && self.extraction.setup_open {
            card(ui, |ui| {
                hint(ui,"New, edited or explicitly selected project sources and relevant accepted Local decisions will be sent to the installed Codex provider. Sources include messages, tasks, team goals, answers and notes. Provider usage may incur costs. Local storage remains free. Generated drafts require your explicit acceptance.");
                hint(ui,"Only future sources are queued automatically. Older sources require individual queueing; accepted decisions from before enablement may still be included as context.");
                let response = ui.checkbox(
                    &mut self.extraction.consent,
                    t(
                        ui,
                        "I allow this project's sources and relevant accepted Local decisions to be sent to Codex for drafting.",
                    ),
                );
                #[cfg(test)]
                ui.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new((
                            "test-action",
                            "extraction-consent",
                            "Allow Codex drafting",
                        )),
                        response.rect,
                    )
                });
                let _ = response;
                let before = commands.len();
                action(
                    ui,
                    commands,
                    "Enable automatic drafting",
                    "project_extraction_configure",
                    self.extraction.consent,
                    || json!({"projectId":project_id,"expectedPolicyRevision":policy_revision,"enabled":true,"consent":true}),
                );
                if commands.len() > before {
                    self.extraction.consent = false;
                    self.extraction.setup_open = false;
                }
            });
        }
        if self.extraction.setup_open && !codex_ready(snapshot) {
            hint(ui,"Installed Codex is not ready. Sources remain saved; check Providers and retry failed jobs manually.");
        }
        let jobs = extraction_jobs(snapshot, project_id);
        if enabled || !jobs.is_empty() {
            let row = extraction_project(snapshot, project_id);
            hint(
                ui,
                &localized_format!(
                    ui,
                    "{0} queued · {1} running · {2} failed",
                    row["counts"]["queued"].as_u64().unwrap_or(0),
                    row["counts"]["running"].as_u64().unwrap_or(0),
                    row["counts"]["failed"].as_u64().unwrap_or(0)
                ),
            );
            if enabled {
                hint(ui,"New sources are drafted automatically. You review every result; nothing is accepted or shared automatically.");
            }
        }
        if !jobs.is_empty() {
            hint(
                ui,
                "Activity shows only the latest 10 jobs; counts include all jobs.",
            );
            egui::CollapsingHeader::new(t(ui, "Recent drafting activity"))
                .id_salt(("extraction-activity", project_id))
                .default_open(jobs.iter().any(|job| {
                    matches!(text(job, "status"), "failed" | "interrupted" | "cancelled")
                }))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(("extraction-jobs", project_id))
                        .max_height(190.0)
                        .show(ui, |ui| {
                            for job in jobs
                                .iter()
                                .filter(|job| text(job, "projectId") == project_id)
                                .take(10)
                            {
                                ui.push_id(text(job, "jobId"), |ui| {
                                    extraction_job_controls(ui, job, project_id, enabled, commands);
                                    ui.separator();
                                });
                            }
                        });
                });
        }
        ui.add_space(14.0);
    }

    fn memory_search(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        shared: bool,
        commands: &mut Vec<Command>,
    ) {
        #[cfg(feature = "ui-smoke")]
        self.initialize_search_smoke(shared);
        let project = if shared {
            &self.shared.project
        } else {
            &self.decisions.project
        }
        .clone();
        let metadata = shared_project(snapshot, &project);
        let global = if shared {
            revision(&snapshot["sharedMemory"])
        } else {
            decision_manifest(snapshot, &project)
        };
        let result = &snapshot[if shared {
            "sharedSearch"
        } else {
            "decisionSearch"
        }];
        let connected = snapshot["cloud"]["paired"] == true && snapshot["cloud"]["error"] != true;
        let allowed = !self.hold_automatic_reads
            && (!shared
                || (connected
                    && metadata["enabled"] == true
                    && metadata["sharedProjectId"].is_string()
                    && text(metadata, "state") != "connection_changed"));
        let search = if shared {
            &mut self.shared.search
        } else {
            &mut self.decisions.search
        };
        let mut selected = None;
        ui.push_id(if shared { "cloud-search" } else { "local-search" }, |ui| {
            egui::CollapsingHeader::new(t(ui, if shared { "Search published decisions" } else { "Search accepted Local decisions" }))
                .default_open(!search.input.is_empty())
                .open(shared.then_some(true))
                .show(ui, |ui| {
            card(ui, |ui| {
                if shared {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(t(ui,"Project"));
                        if let Some(selected) = array(&snapshot["projectMemory"]["projects"]).iter().find(|p|text(p,"projectId")==project) {
                            ui.label(RichText::new(text(selected,"name")).strong().color(colors(ui).text));
                        }
                    });
                    ui.collapsing(t(ui,"Shared project identity"),|ui| { ui.monospace(text(metadata,"sharedProjectId")); });
                }
                hint(ui, if shared {
                    "Only Search Cloud sends this query to your selected shared project. Typing stays on this device."
                } else { "Free keyword ranking over current accepted decisions. Your query stays on this device." });
                ui.add_space(8.0);
                ui.add(egui::TextEdit::singleline(&mut search.input)
                    .desired_width(ui.available_width())
                    .margin(egui::Margin::symmetric(10,7))
                    .char_limit(400)
                    .hint_text(t(ui,"Ask about a project decision…")));
                ui.horizontal_wrapped(|ui| {
                    ui.label(t(ui,"Results"));
                    ui.add(egui::DragValue::new(&mut search.limit).range(1..=10));
                    let payload = search.payload(&project);
                    let before = commands.len();
                    action(ui,commands,if shared { "Search Cloud" } else { "Search Local" },
                        if shared { "memory_cloud_search" } else { "project_decision_search" },
                        allowed && payload.is_some(),||payload.unwrap());
                    if commands.len() > before { search.submitted_limit = search.limit; }
                });
                if !search.input.trim().is_empty() && search.payload(&project).is_none() {
                    hint(ui,"Use a query of up to 400 characters without control characters.");
                }
                let current = search.matches(result,&project,global,shared)
                    && (!shared || (result["sharedProjectId"] == metadata["sharedProjectId"]
                        && result["manifestRevision"].as_u64().zip(metadata["sharedRevision"].as_u64()).is_some_and(|(found,current)| found >= current)));
                if !current {
                    if !result.is_null() { hint(ui,"Search again to refresh results for this query and project."); }
                    return;
                }
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    status(ui,if text(result,"mode") == "semantic" { "Semantic matches" }
                        else if shared { "Keyword fallback" } else { "Local keyword matches" });
                    hint(ui,&localized_format!(ui,"Search snapshot · revision {0}",if shared { result["manifestRevision"].as_u64().unwrap_or(0) } else { global }));
                });
                if shared {
                    let index = &result["index"];
                    hint(ui,&localized_format!(ui,"Indexed {0} of {1} current decisions",index["indexed"].as_u64().unwrap_or(0),index["total"].as_u64().unwrap_or(0)));
                    hint(ui,match text(index,"state") {
                        "ready" => "Semantic index ready",
                        "pending" => "Indexing in progress · keyword results cover current decisions.",
                        "failed" => "Indexing failed · keyword fallback is available.",
                        _ => "Semantic service unavailable · keyword fallback is available.",
                    });
                }
                let downloaded = !shared || (metadata["sharedRevision"] == result["manifestRevision"]
                    && metadata["verifiedAt"].is_string() && metadata["stale"] == false
                    && metadata["offline"] == false && allowed);
                if shared && !downloaded {
                    hint(ui,"Sync shared memory to download and verify these exact revisions, then search again. Remote matches do not change Local acceptance.");
                    action(ui,commands,"Sync shared memory","memory_cloud_sync",allowed,||json!({"projectId":project}));
                }
                if array(&result["matches"]).is_empty() { hint(ui,"No matching current decisions found."); }
                for record in array(&result["matches"]).iter().take(search.limit as usize) {
                    ui.push_id(text(record,"decisionId"),|ui| {
                        ui.separator();
                        ui.label(RichText::new(text(record,"title")).strong().color(colors(ui).text));
                        hint(ui,&localized_format!(ui,"Revision {0}",revision(record)));
                        if control(ui,if shared { "Read exact downloaded copy" } else { "Read accepted decision" },
                            allowed && downloaded && revision(record)>0).clicked() {
                            selected = Some((text(record,"decisionId").to_owned(),revision(record)));
                        }
                    });
                }
            });
                });
        });
        if let Some((decision, revision)) = selected {
            if shared {
                self.shared.search_open = false;
                self.shared.selected = decision.clone();
                self.shared.selected_revision = revision;
                commands.push(Command::new(
                    "memory_cloud_decision_read",
                    json!({"projectId":project,"decisionId":decision,"revision":revision}),
                ));
            } else {
                self.decisions.selected = decision.clone();
                self.decisions.replacement.clear();
                commands.push(Command::new(
                    "project_decision_read",
                    json!({"projectId":project,"decisionId":decision}),
                ));
            }
        }
    }

    fn decision_panel(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let projects = array(&snapshot["projectMemory"]["projects"]);
        if projects.is_empty() {
            self.extraction.consent = false;
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
        project_switcher(
            ui,
            "decision-project",
            &mut self.decisions.project,
            projects,
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
            self.extraction.consent = false;
            hint(ui, "Choose a project to review its decisions.");
            return;
        };
        if let Some((policy, _)) = extraction_policy(snapshot, text(project, "projectId")) {
            self.extraction.scope(text(project, "projectId"), policy);
        } else {
            self.extraction.consent = false;
        }
        self.decisions
            .sources
            .observe_extraction_revision(revision(&snapshot["projectExtraction"]));
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
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
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
            if control(ui, "New decision draft", !self.decisions.composer_open).clicked() {
                self.decisions.composer_open = true;
            }
            if control(ui, "Refresh decisions", !self.hold_automatic_reads).clicked() {
                self.decisions.refresh();
            }
            if control(ui, "Open project sources", true).clicked() {
                self.memory.select(&self.decisions.project);
                self.tab = Tab::Memory;
            }
            if control(ui, "Drafting settings", true).clicked() {
                self.memory.select(&self.decisions.project);
                self.extraction.consent = false;
                self.extraction.setup_open = true;
                self.tab = Tab::Memory;
            }
        });
        ui.add_space(8.0);
        if ui.available_width() < 650.0 {
            egui::CollapsingHeader::new(t(ui, "Search & filters"))
                .id_salt("decision-compact-filters")
                .show(ui, |ui| self.decision_filters(ui, snapshot, commands));
        } else {
            self.decision_filters(ui, snapshot, commands);
        }
        ui.add_space(8.0);

        if self.decisions.composer_open {
            constrained(ui, 860.0, |ui| {
                self.decision_composer(ui, project, &snapshot["memoryPage"], manifest, commands)
            });
            ui.add_space(20.0);
        }
        ui.add_space(if ui.available_width() < 650.0 {
            0.0
        } else {
            16.0
        });
        if ui.available_width() >= 850.0 {
            ui.columns(2, |columns| {
                self.decision_list(&mut columns[0], page, manifest);
                self.decision_detail(&mut columns[1], detail, page, manifest, snapshot, commands);
            });
        } else {
            self.decision_detail(ui, detail, page, manifest, snapshot, commands);
            ui.add_space(16.0);
            self.decision_list(ui, page, manifest);
        }
        if commands.is_empty() {
            let busy = !ui.is_enabled() || self.hold_automatic_reads;
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

    fn decision_filters(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        self.memory_search(ui, snapshot, false, commands);
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!self.hold_automatic_reads, |ui| {
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
            });
            ui.add(
                egui::TextEdit::singleline(&mut self.decisions.query_input)
                    .font(egui::TextStyle::Body)
                    .margin(egui::Margin::symmetric(10, 7))
                    .desired_width(230.0)
                    .char_limit(160)
                    .hint_text(t(ui, "Search decision text")),
            );
            if control(ui, "Apply decision filter", !self.hold_automatic_reads).clicked() {
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
                    .color(colors(ui).text),
            );
            hint(ui,"Select one to eight source versions. Their original text remains attached to the decision.");
            if self.decisions.sources_changed {
                ui.colored_label(
                    colors(ui).danger,
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
                if control(ui, "Refresh draft sources", !self.hold_automatic_reads).clicked() {
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
                        self.decisions.sources.offset > 0 && !self.hold_automatic_reads,
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
                        self.decisions.sources.offset.saturating_add(10) < total
                            && !self.hold_automatic_reads,
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
                let mut submitted = commands.last().unwrap().payload.clone();
                for key in ["commandId", "createdAt", "expiresAt"] {
                    submitted.as_object_mut().unwrap().remove(key);
                }
                self.decisions.pending_proposal = Some((manifest, submitted));
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
            if control(
                ui,
                "Previous decisions",
                self.decisions.offset > 0 && !self.hold_automatic_reads,
            )
            .clicked()
            {
                self.decisions.offset = self.decisions.offset.saturating_sub(10);
            }
            if control(
                ui,
                "Next decisions",
                self.decisions.offset.saturating_add(10) < total && !self.hold_automatic_reads,
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
                        colors(ui).info_surface
                    } else {
                        colors(ui).surface
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if selected {
                            colors(ui).primary
                        } else {
                            colors(ui).border
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
                                .color(colors(ui).text),
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
                        if control(ui, "Open decision", !self.hold_automatic_reads).clicked() {
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
        snapshot: &Value,
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
                    .color(colors(ui).text),
            );
            ui.horizontal_wrapped(|ui| {
                status(ui, text(decision, "status"));
                hint(
                    ui,
                    &localized_format!(ui, "Revision {0}", revision(decision)),
                );
            });
            ui.add_space(12.0);
            extraction_provenance(ui, decision);
            if text(decision, "status") == "accepted" {
                self.publication_controls(ui, snapshot, detail, commands);
            }
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
                        if control(
                            ui,
                            "Browse proposed replacements",
                            !self.hold_automatic_reads,
                        )
                        .clicked()
                        {
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
                    ui.label(
                        RichText::new(t(ui, label))
                            .size(12.0)
                            .color(colors(ui).muted),
                    );
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
                    .color(colors(ui).text),
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
                    ui.label(
                        RichText::new(text(source, "title"))
                            .strong()
                            .color(colors(ui).text),
                    );
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

    fn publication_controls(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        detail: &Value,
        commands: &mut Vec<Command>,
    ) {
        let decision = &detail["decision"];
        let project = text(decision, "projectId");
        let metadata = shared_project(snapshot, project);
        let cloud = &snapshot["cloud"];
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        if cloud["paired"] != true || cloud["error"] == true {
            self.shared.consent = false;
            hint(
                ui,
                "This decision stays private. Pair Cloud to configure optional sharing.",
            );
            return;
        }
        if text(metadata, "role") != "publisher" || text(metadata, "state") == "connection_changed"
        {
            self.shared.consent = false;
            hint(
                ui,
                if text(metadata, "role") == "reader" {
                    "This installation is a reader. Local decisions remain private."
                } else {
                    "Create a shared project before publishing this accepted decision."
                },
            );
            if control(ui, "Configure shared memory", true).clicked() {
                self.shared.select(project);
                self.tab = Tab::Cloud;
            }
            return;
        }
        if control(ui, "Preview Cloud publication", true).clicked() {
            self.shared.preview_open = !self.shared.preview_open;
        }
        if !self.shared.preview_open {
            return;
        }
        let Some(preview) = publication_preview(detail) else {
            self.shared.consent = false;
            hint(
                ui,
                "Accepted proof is unavailable. Refresh the local decision before publishing.",
            );
            return;
        };
        let scope = json!({"projectId":project,"decisionId":decision["decisionId"],"revision":decision["revision"],"sharedProjectId":metadata["sharedProjectId"],"sharedRevision":metadata["sharedRevision"],"role":metadata["role"],"enabled":metadata["enabled"],"state":metadata["state"],"pendingOperation":metadata["pendingOperation"],"cloudUrl":cloud["cloudUrl"],"workspaceId":cloud["workspaceId"],"hostId":cloud["hostId"]});
        if self.shared.consent_scope.as_ref() != Some(&scope) {
            self.shared.consent = false;
            self.shared.consent_scope = Some(scope);
        }
        ui.add_space(8.0);
        section_title(ui,"Publication preview","Only these accepted decision fields and opaque source references leave this device. Source text and origin records stay local.");
        egui::ScrollArea::vertical()
            .id_salt("publication-preview-body")
            .max_height(220.0)
            .show(ui, |ui| {
                for (key, label) in [
                    ("title", "Decision title"),
                    ("decision", "Decision statement"),
                    ("context", "Context"),
                    ("rationale", "Rationale"),
                    ("consequences", "Consequences"),
                ] {
                    ui.label(
                        RichText::new(t(ui, label))
                            .size(12.0)
                            .color(colors(ui).muted),
                    );
                    ui.label(if text(&preview, key).is_empty() {
                        "—"
                    } else {
                        text(&preview, key)
                    });
                    ui.add_space(6.0);
                }
                hint(
                    ui,
                    &localized_format!(
                        ui,
                        "Accepted {0}",
                        format_time(ui, text(&preview, "acceptedAt"))
                    ),
                );
                hint(
                    ui,
                    &localized_format!(
                        ui,
                        "{0} source references · source text stays local",
                        array(&preview["sources"]).len()
                    ),
                );
                ui.collapsing(t(ui, "Technical publication details"), |ui| {
                    ui.label(t(ui, "Exact upload fields"));
                    ui.add(
                        egui::Label::new(
                            RichText::new(serde_json::to_string_pretty(&preview).unwrap())
                                .monospace()
                                .size(12.0),
                        )
                        .wrap(),
                    );
                });
            });
        let consent = ui.checkbox(
            &mut self.shared.consent,
            t(ui, "I approve uploading exactly the fields shown above."),
        );
        #[cfg(test)]
        ui.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("test-action", "publication-consent", "Approve upload")),
                consent.rect,
            )
        });
        let _ = consent;
        let before = commands.len();
        action(
            ui,
            commands,
            "Queue explicit publication",
            "memory_cloud_publish",
            self.shared.consent && shared_publisher_ready(metadata, cloud),
            || json!({"projectId":project,"decisionId":decision["decisionId"],"expectedDecisionRevision":decision["revision"],"expectedSharedRevision":metadata["sharedRevision"],"consent":true}),
        );
        if commands.len() > before {
            self.shared.consent = false;
        }
        if !metadata["pendingOperation"].is_null() {
            hint(
                ui,
                "Queued locally, not confirmed by Cloud. Sync retries the existing operation.",
            );
        }
        if metadata["enabled"] != true {
            hint(
                ui,
                "Sharing is disabled. Enable it in Cloud before publishing.",
            );
        }
        if metadata["diverged"] == true {
            hint(
                ui,
                "Local and published revisions differ. Nothing is republished automatically.",
            );
        }
        ui.add_space(12.0);
    }

    fn shared_memory(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        ui.add_space(20.0);
        section_title(ui,"Shared decision memory","Optional Cloud publication is separate from private Local decisions. Cloud verifies subscription and access on every request.");
        ui.add_space(12.0);
        let projects = array(&snapshot["projectMemory"]["projects"]);
        if projects.is_empty() {
            hint(ui, "Create a project first");
            return;
        }
        if self.shared.project.is_empty() && projects.len() == 1 {
            self.shared.select(text(&projects[0], "projectId"));
        }
        let previous = self.shared.project.clone();
        choose(
            ui,
            "shared-project",
            "Project",
            &mut self.shared.project,
            projects,
            ("projectId", "name"),
            true,
        );
        if self.shared.project != previous {
            let selected = self.shared.project.clone();
            self.shared.project = previous;
            self.shared.select(&selected);
        }
        let Some(project) = projects
            .iter()
            .find(|p| text(p, "projectId") == self.shared.project)
        else {
            return;
        };
        #[cfg(feature = "ui-smoke")]
        self.initialize_search_smoke(true);
        let metadata = shared_project(snapshot, &self.shared.project);
        let cloud = &snapshot["cloud"];
        let connected = cloud["paired"] == true && cloud["error"] != true;
        let remap = text(metadata, "state") == "connection_changed";
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status(ui, shared_role_label(text(metadata, "role")));
                status(ui, shared_state_label(text(metadata, "state")));
                hint(
                    ui,
                    &localized_format!(
                        ui,
                        "Shared revision {0}",
                        metadata["sharedRevision"].as_u64().unwrap_or(0)
                    ),
                );
            });
            if !connected {
                hint(
                    ui,
                    "Cloud is disconnected. Private decisions and downloaded copies remain local.",
                );
            }
            if metadata["sharedProjectId"].is_string() && text(metadata, "state") != "unlinked" {
                if metadata["offline"] == true {
                    ui.colored_label(
                        colors(ui).danger,
                        t(ui, "Offline cache · verification is not current."),
                    );
                } else if metadata["stale"] == true {
                    hint(
                        ui,
                        "Cached data may be stale. Sync to verify current shared context.",
                    );
                }
            }
            hint(
                ui,
                &localized_format!(
                    ui,
                    "Last verified {0}",
                    format_time(ui, text(metadata, "verifiedAt"))
                ),
            );
            if metadata["diverged"] == true {
                hint(
                    ui,
                    "Local and published revisions differ. Nothing is republished automatically.",
                );
            }
            if let Some(code) = metadata["lastErrorCode"].as_str() {
                let message = t(ui, code);
                hint(
                    ui,
                    if message == code {
                        "Shared memory could not be updated. Local decisions remain available."
                    } else {
                        message.as_ref()
                    },
                );
            }
            if !metadata["pendingOperation"].is_null() {
                hint(
                    ui,
                    "Queued locally, not confirmed by Cloud. Sync retries the existing operation.",
                );
            }
            if remap {
                hint(ui,"The paired connection changed. Old publication consent will not transfer. Create or link a shared project explicitly.");
            }
            let unlinked = !metadata["sharedProjectId"].is_string() || remap;
            if unlinked {
                let create = egui::CollapsingHeader::new(t(ui,"Create a shared project")).id_salt("shared-create").show(ui,|ui| {
                    field(ui,"Shared project name",&mut self.shared.name,80,false);
                    action(ui,commands,"Queue shared project creation","memory_cloud_create",connected&&revision(project)>0&&(remap||metadata["pendingOperation"].is_null())&&decision_text(&self.shared.name,80,true),||json!({"projectId":self.shared.project,"expectedProjectRevision":revision(project),"name":self.shared.name.trim()}));
                });
                let link = egui::CollapsingHeader::new(t(ui,"Link this installation as a reader")).id_salt("shared-link").show(ui,|ui| {
                    field(ui,"Shared project ID",&mut self.shared.link_id,80,false);
                    action(ui,commands,"Queue reader link","memory_cloud_link",connected&&revision(project)>0&&(remap||metadata["pendingOperation"].is_null())&&opaque_id(self.shared.link_id.trim(),"shared"),||json!({"projectId":self.shared.project,"expectedProjectRevision":revision(project),"sharedProjectId":self.shared.link_id.trim()}));
                    hint(ui,"Use a shared project ID from the same Cloud workspace. This does not grant publisher rights.");
                });
                #[cfg(test)]
                ui.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new(("test-action", "shared-header", "Create")),
                        create.header_response.rect,
                    );
                    d.insert_temp(
                        egui::Id::new(("test-action", "shared-header", "Link")),
                        link.header_response.rect,
                    );
                });
                let _ = (create, link);
            } else {
                ui.collapsing(t(ui, "Shared project identity"), |ui| {
                    ui.monospace(text(metadata, "sharedProjectId"));
                });
                ui.horizontal_wrapped(|ui| {
                    action(ui,commands,"Sync shared memory","memory_cloud_sync",connected,||json!({"projectId":self.shared.project}));
                    if control(ui,"Find a published decision",!self.hold_automatic_reads).clicked() {
                        self.shared.search_open = true;
                    }
                    if text(metadata,"role")=="publisher" {
                        let enabled=metadata["enabled"]==true;
                        action(ui,commands,if enabled {"Disable sharing"}else{"Enable sharing"},"memory_cloud_sharing",connected&&metadata["pendingOperation"].is_null(),||json!({"projectId":self.shared.project,"expectedSharedRevision":metadata["sharedRevision"],"enabled":!enabled}));
                    }
                });
                if metadata["enabled"] == false {
                    hint(ui,"Sharing is disabled. New Cloud access is blocked; downloaded copies are not erased.");
                }
            }
        });
        if !metadata["sharedProjectId"].is_string() {
            return;
        }
        ui.add_space(14.0);
        hint(ui,"Published copies are read only here. They do not accept or replace private Local decisions.");
        let global = revision(&snapshot["sharedMemory"]);
        let page = &snapshot["sharedMemoryPage"];
        let detail = &snapshot["sharedDecisionDetail"];
        if self.shared.selected.is_empty() {
            if text(detail, "reasonCode") == "SHARED_DECISION_READ"
                && text(detail, "projectId") == self.shared.project
                && revision(detail) == global
                && detail["sharedProjectId"] == metadata["sharedProjectId"]
            {
                self.shared.selected = text(&detail["decision"], "decisionId").into();
                self.shared.selected_revision = revision(&detail["decision"]);
            } else if self.shared.page_matches(page, metadata, global) {
                if let Some(first) = array(&page["decisions"]).first() {
                    self.shared.selected = text(first, "decisionId").into();
                    self.shared.selected_revision = revision(first);
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            if control(ui, "Refresh downloaded copies", !self.hold_automatic_reads).clicked() {
                self.shared.refresh_page = true;
                self.shared.refresh_detail = true;
            }
            if self.shared.page_matches(page, metadata, global) {
                if control(
                    ui,
                    "Previous shared records",
                    !self.hold_automatic_reads && self.shared.offset > 0,
                )
                .clicked()
                {
                    self.shared.offset = self.shared.offset.saturating_sub(10);
                }
                if control(
                    ui,
                    "Next shared records",
                    !self.hold_automatic_reads
                        && self.shared.offset.saturating_add(10)
                            < page["total"].as_u64().unwrap_or(0),
                )
                .clicked()
                {
                    self.shared.offset = self.shared.offset.saturating_add(10);
                }
            }
        });
        if self.shared.page_matches(page, metadata, global) {
            for record in array(&page["decisions"]).iter().take(10) {
                ui.push_id(text(record, "decisionId"), |ui| {
                    card(ui, |ui| {
                        ui.label(
                            RichText::new(text(record, "title"))
                                .strong()
                                .color(colors(ui).text),
                        );
                        hint(ui, &localized_format!(ui, "Revision {0}", revision(record)));
                        if control(ui, "Read published copy", !self.hold_automatic_reads).clicked()
                        {
                            self.shared.selected = text(record, "decisionId").into();
                            self.shared.selected_revision = revision(record);
                        }
                    })
                });
                ui.add_space(8.0);
            }
            if page["total"] == 0 {
                hint(ui, "No published heads are cached for this project.");
            }
        } else {
            hint(
                ui,
                "Downloaded manifest unavailable. Refresh retries a failed cached read.",
            );
        }
        ui.collapsing(t(ui, "Open an exact downloaded revision"), |ui| {
            field(ui, "Decision ID", &mut self.shared.exact_id, 80, false);
            ui.horizontal_wrapped(|ui| {
                ui.label(t(ui, "Revision"));
                ui.add(
                    egui::DragValue::new(&mut self.shared.exact_revision)
                        .range(1..=9_007_199_254_740_991u64),
                );
            });
            if control(
                ui,
                "Read exact downloaded revision",
                !self.hold_automatic_reads
                    && opaque_id(self.shared.exact_id.trim(), "decision")
                    && self.shared.exact_revision > 0,
            )
            .clicked()
            {
                self.shared.selected = self.shared.exact_id.trim().into();
                self.shared.selected_revision = self.shared.exact_revision;
                self.shared.refresh_detail = true;
            }
        });
        if self.shared.detail_matches(detail, metadata, global) {
            ui.add_space(12.0);
            #[cfg(feature = "ui-smoke")]
            if self.shared_smoke_focus {
                ui.scroll_to_cursor(Some(egui::Align::TOP));
                self.shared_smoke_focus = false;
            }
            card(ui, |ui| {
                status(
                    ui,
                    if detail["current"] == true {
                        "Current shared head"
                    } else {
                        "Historical shared version"
                    },
                );
                if detail["current"] != true {
                    hint(
                        ui,
                        "Historical version · do not treat this as current shared context.",
                    );
                }
                if detail["offline"] == true
                    || detail["stale"] == true
                    || metadata["offline"] == true
                    || metadata["stale"] == true
                {
                    hint(
                        ui,
                        "Cached data may be stale. Sync to verify current shared context.",
                    );
                }
                let record = &detail["decision"];
                ui.label(
                    RichText::new(text(record, "title"))
                        .size(16.0)
                        .strong()
                        .color(colors(ui).text),
                );
                for (key, label) in [
                    ("decision", "Decision statement"),
                    ("context", "Context"),
                    ("rationale", "Rationale"),
                    ("consequences", "Consequences"),
                ] {
                    if !text(record, key).is_empty() {
                        ui.label(
                            RichText::new(t(ui, label))
                                .size(12.0)
                                .color(colors(ui).muted),
                        );
                        ui.label(text(record, key));
                        ui.add_space(6.0);
                    }
                }
                hint(ui, &localized_format!(ui, "Revision {0}", revision(record)));
                hint(
                    ui,
                    &localized_format!(
                        ui,
                        "Published {0}",
                        format_time(ui, text(detail, "publishedAt"))
                    ),
                );
                ui.collapsing(t(ui, "Published identity and references"), |ui| {
                    ui.monospace(text(record, "decisionId"));
                    for source in array(&record["sources"]) {
                        ui.monospace(text(source, "sourceId"));
                        hint(ui, &localized_format!(ui, "Revision {0}", revision(source)));
                    }
                });
            });
        } else if !self.shared.selected.is_empty() {
            hint(
                ui,
                "Exact downloaded revision unavailable. Refresh retries the cached read.",
            );
        }
        if commands.is_empty() && !self.shared.search_open {
            if let Some(read) = self.shared.next_read(
                page,
                detail,
                metadata,
                global,
                !ui.is_enabled() || self.hold_automatic_reads,
            ) {
                commands.push(read);
            }
        }
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
            ui.label(
                RichText::new(t(ui, "TASK QUEUE"))
                    .size(10.0)
                    .color(colors(ui).muted),
            );
            ui.label(
                RichText::new(items(&snapshot["work"]).len().to_string())
                    .size(12.0)
                    .color(colors(ui).text),
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
                if let Some(memory) = array(&snapshot["agentMemory"]["contexts"]).iter().rev().find(|r|r["workItemId"]==item["workItemId"]) {
                    hint(ui,&localized_format!(ui,"Project memory · start {0} · current {1}",memory["decisionRevision"].as_u64().unwrap_or(0),memory["currentDecisionRevision"].as_u64().unwrap_or(0)));
                    if memory["issue"] == "AGENT_MEMORY_UNSUPPORTED" {
                        hint(ui,"Update Codex to 0.155.1 or later to connect project memory.");
                    } else if !memory["issue"].is_null() {
                        hint(ui,"Project memory could not connect. Fix the connection and start this task again.");
                    } else if memory["manifestLoaded"] == true {
                        hint(ui,"Current accepted decision list loaded.");
                    } else if memory["stale"] == true {
                        hint(ui,"Decisions changed. The agent must refresh project context before reading decisions.");
                    } else {
                        hint(ui,"Project context is bound to this run.");
                    }
                }
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
                    ui.colored_label(colors(ui).danger, t(ui, code));
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
                    if ui.available_width() >= 780.0 {
                        ui.columns(2, |columns| {
                            self.team_direction(&mut columns[0], snapshot);
                            self.team_roles(&mut columns[1], snapshot);
                        });
                    } else {
                        self.team_direction(ui, snapshot);
                        ui.add_space(18.0);
                        self.team_roles(ui, snapshot);
                    }
                    ui.add_space(18.0);
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
                if task["status"] == "queued" && array(&snapshot["work"]["receipts"]).iter().rev().find(|r|r["commandType"]=="start_work" && r["workItemId"]==task["workItemId"]).is_some_and(|r|r["outcome"]!="applied") {
                    hint(ui,"Start failed. Fix the provider connection, then start this task from Work. Automatic retries are paused.");
                }
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

    fn team_direction(&mut self, ui: &mut Ui, snapshot: &Value) {
        ui.label(
            RichText::new(t(ui, "01 / DIRECTION"))
                .size(10.0)
                .color(colors(ui).primary),
        );
        ui.add_space(12.0);
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
        multiline(
            ui,
            "Goal and acceptance criteria",
            &mut self.team.goal,
            10_000,
            5,
        );
    }

    fn team_roles(&mut self, ui: &mut Ui, snapshot: &Value) {
        ui.label(
            RichText::new(t(ui, "02 / TEAM & LIMITS"))
                .size(10.0)
                .color(colors(ui).primary),
        );
        ui.add_space(12.0);
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
    }

    fn agents(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let packs = array(&snapshot["packs"]);
        hint(
            ui,
            &localized_format!(ui, "{0} inspectable local packs", packs.len()),
        );
        ui.add_space(12.0);
        let column_count = if ui.available_width() >= 1000.0 {
            3
        } else if ui.available_width() >= 620.0 {
            2
        } else {
            1
        };
        for row in packs.chunks(column_count) {
            ui.columns(column_count, |columns| {
                for (pack, ui) in row.iter().zip(columns.iter_mut()) {
                    ui.push_id(text(pack, "id"), |ui| {
                        card(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(studio::title(catalog_text(ui, pack, "name"), 16.0));
                                status(ui, text(pack, "role"));
                                hint(ui, text(&pack["provenance"], "kind"));
                            });
                            ui.label(catalog_text(ui, pack, "description"));
                            egui::CollapsingHeader::new(t(ui, "Inspect pack contract")).show(
                                ui,
                                |ui| {
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
                                    ui.monospace(format!(
                                        "{}@{}",
                                        text(pack, "id"),
                                        text(pack, "version")
                                    ));
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
                                },
                            );
                        })
                    });
                    ui.add_space(8.0);
                }
            });
            ui.add_space(10.0);
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
                        ui.colored_label(
                            colors(ui).danger,
                            t(ui, text(&provider["error"], "message")),
                        );
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

    fn open_login_browser(&mut self, ctx: &egui::Context, snapshot: &Value) {
        #[cfg(feature = "ui-smoke")]
        if self.suppress_account_browser {
            return;
        }
        let login = &snapshot["cloudLogin"];
        let command = snapshot["cloudAccountUi"]["openCommandId"].as_str();
        if login["status"] == "waiting"
            && command.is_some()
            && command != self.opened_login_command.as_deref()
            && valid_login_url(snapshot)
        {
            self.opened_login_command = command.map(str::to_owned);
            ctx.open_url(egui::OpenUrl::new_tab(text(login, "authorizationUrl")));
        }
    }

    fn account_panel(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let cloud = &snapshot["cloud"];
        let login = &snapshot["cloudLogin"];
        let account_ui = &snapshot["cloudAccountUi"];
        let account_busy = account_ui["busy"] == true;
        let waiting = login["status"] == "waiting";
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(t(ui, "Moxcrew Cloud"));
                status(
                    ui,
                    if cloud["paired"] == true {
                        "Connected"
                    } else {
                        "Free · Local"
                    },
                );
            });
            hint(ui,"Local is free and works without an account. Sign in for optional subscription Cloud services.");
            ui.add_space(12.0);
            if cloud["error"] == true {
                ui.colored_label(
                    colors(ui).danger,
                    t(
                        ui,
                        "Cloud settings need local recovery. Local work remains available.",
                    ),
                );
                ui.label(t(ui, "Close Moxcrew and back up the private cloud.json file in its data directory. Explicitly repair or remove that file, then reopen the app."));
                hint(
                    ui,
                    "Pairing is unavailable until the local Cloud state can be read safely.",
                );
                return;
            }
            if let Some(error) = account_ui["error"].as_str() {
                ui.colored_label(colors(ui).danger, t(ui, error));
                hint(ui,"Account status could not be verified. Local work and downloaded decisions remain available.");
            }
            if let Some(error) = self.account_form_error {
                ui.colored_label(colors(ui).danger, t(ui, error));
            }
            if account_ui["logoutRemoteRevoked"] == false {
                hint(ui,"Signed out on this device. Cloud could not confirm revocation; sign out this installation from Cloud when online.");
            } else if account_ui["logoutRemoteRevoked"] == true {
                hint(
                    ui,
                    "Signed out. Local projects, providers and downloaded decisions are kept.",
                );
            }
            if waiting {
                status(ui, "Waiting for browser approval");
                hint(
                    ui,
                    "Compare this code in the browser before approving this installation.",
                );
                ui.label(
                    RichText::new(text(login, "confirmationCode"))
                        .monospace()
                        .size(22.0)
                        .strong()
                        .color(colors(ui).primary),
                );
                hint(
                    ui,
                    &localized_format!(
                        ui,
                        "Sign-in expires {0}",
                        format_time(ui, text(login, "expiresAt"))
                    ),
                );
                let valid = valid_login_url(snapshot);
                if !valid {
                    ui.colored_label(
                        colors(ui).danger,
                        t(
                            ui,
                            "The browser address was rejected. Cancel and start sign-in again.",
                        ),
                    );
                }
                ui.horizontal_wrapped(|ui| {
                    if control(ui, "Open browser", valid).clicked() {
                        ui.ctx()
                            .open_url(egui::OpenUrl::new_tab(text(login, "authorizationUrl")));
                    }
                    if control(ui, "Copy sign-in link", valid).clicked() {
                        ui.ctx()
                            .copy_text(text(login, "authorizationUrl").to_owned());
                    }
                    action(
                        ui,
                        commands,
                        "Cancel sign-in",
                        "cloud_login_cancel",
                        true,
                        || json!({}),
                    );
                });
                hint(ui,"The browser keeps your login session. Moxcrew receives only this installation's connection credential.");
            } else if cloud["paired"] == true {
                if let Some(account) = verified_account(snapshot) {
                    if let Some(email) = account["email"].as_str() {
                        ui.label(RichText::new(email).strong().color(colors(ui).text));
                    } else {
                        ui.label(t(ui, "Verified account"));
                    }
                    ui.horizontal_wrapped(|ui| {
                        status(
                            ui,
                            subscription_label(text(&account["subscription"], "status")),
                        );
                        hint(
                            ui,
                            if account["subscription"]["memoryAccess"] == true {
                                "Subscription memory access available"
                            } else {
                                "Cloud memory needs an eligible subscription"
                            },
                        );
                    });
                    hint(
                        ui,
                        &localized_format!(
                            ui,
                            "Account checked {0}",
                            format_time(ui, text(&account["subscription"], "checkedAt"))
                        ),
                    );
                } else {
                    status(
                        ui,
                        if account_busy {
                            "Checking account…"
                        } else {
                            "Account not verified"
                        },
                    );
                    hint(ui,"A connection is not proof of an active subscription. Refresh to verify account access.");
                }
                ui.horizontal_wrapped(|ui| {
                    action(
                        ui,
                        commands,
                        "Refresh account",
                        "cloud_account_read",
                        !account_busy,
                        || json!({}),
                    );
                    if control(ui, "Subscription and billing", true).clicked() {
                        if let Ok(origin) = patchfleet_core::desktop_login::validate_cloud_origin(
                            text(cloud, "cloudUrl"),
                        ) {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(format!(
                                "{}/#plan-title",
                                origin.trim_end_matches('/')
                            )));
                        }
                    }
                    action(ui, commands, "Sign out", "cloud_logout", true, || json!({}));
                });
                if account_ui["error"].as_str().is_some_and(|code| {
                    matches!(code, "DESKTOP_ACCESS_DENIED" | "CLOUD_AUTH_REJECTED")
                }) {
                    hint(
                        ui,
                        "Sign out, then sign in again to reconnect this installation.",
                    );
                }
            } else {
                match text(login, "status") {
                    "denied" => hint(ui, "Sign-in was denied in the browser. You can try again."),
                    "expired" => hint(ui, "Sign-in expired. Start again when you are ready."),
                    "cancelled" => hint(ui, "Sign-in cancelled. Local work is ready."),
                    _ => (),
                }
                if control(
                    ui,
                    if account_busy {
                        "Starting sign-in…"
                    } else {
                        "Sign in to Cloud"
                    },
                    !account_busy,
                )
                .clicked()
                {
                    if let Ok(origin) =
                        patchfleet_core::desktop_login::validate_cloud_origin(self.cloud_url.trim())
                    {
                        let name = if self.host_name.trim().is_empty() {
                            "Moxcrew desktop"
                        } else {
                            self.host_name.trim()
                        };
                        if required(name, 80, "").is_ok() {
                            self.account_form_error = None;
                            commands.push(Command::new(
                                "cloud_login_begin",
                                json!({"cloudUrl":origin,"displayName":name}),
                            ));
                        } else {
                            self.account_advanced = true;
                            self.account_form_error =
                                Some("Choose a host name of up to 80 characters.");
                        }
                    } else {
                        self.account_advanced = true;
                        self.account_form_error = Some(
                            "Set your Cloud address in connection settings before signing in.",
                        );
                    }
                }
            }
            ui.add_space(8.0);
            if control(
                ui,
                "Advanced connection settings",
                !waiting && !account_busy,
            )
            .clicked()
            {
                self.account_advanced = !self.account_advanced;
            }
            if self.account_advanced {
                ui.separator();
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
                        ui.colored_label(colors(ui).danger, t(ui, cloud_error(code)));
                    }
                    ui.horizontal_wrapped(|ui| {
                        action(ui, commands, "Sync now", "cloud_sync", true, || json!({}));
                    });
                } else {
                    field(ui, "Cloud URL", &mut self.cloud_url, 2048, false);
                    field(ui, "Host name", &mut self.host_name, 80, false);
                    hint(
                        ui,
                        "Manual pairing is available for an existing Cloud pairing code.",
                    );
                    field(ui, "Pairing code", &mut self.pairing_code, 256, true);
                    if submit(ui, "Pair this host", "cloud_pair") {
                        if required(&self.cloud_url, 2048, "").is_err()
                            || required(&self.host_name, 80, "").is_err()
                            || required(&self.pairing_code, 256, "").is_err()
                        {
                            self.form_error =
                                Some("Enter a Cloud URL, host name and pairing code.");
                        } else {
                            commands.push(Command::new("cloud_pair",json!({"cloudUrl":self.cloud_url.trim(),"displayName":self.host_name.trim(),"pairingCode":self.pairing_code.trim()})));
                            self.pairing_code.clear();
                            self.form_error = None;
                        }
                    }
                }
            }
        });
    }

    fn cloud(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        if snapshot["cloud"]["paired"] == true && ui.available_width() >= 900.0 {
            ui.columns(2, |columns| {
                self.account_panel(&mut columns[0], snapshot, commands);
                self.shared_memory(&mut columns[1], snapshot, commands);
            });
        } else {
            self.account_panel(ui, snapshot, commands);
            self.shared_memory(ui, snapshot, commands);
        }
    }
}

fn array(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn verified_account(snapshot: &Value) -> Option<&Value> {
    let account = &snapshot["cloudAccount"];
    patchfleet_core::desktop_login::validate_account_response(account, &snapshot["cloud"])
        .is_ok()
        .then_some(account)
}
fn valid_login_url(snapshot: &Value) -> bool {
    patchfleet_core::desktop_login::validate_authorization_url(
        text(&snapshot["cloudLogin"], "authorizationUrl"),
        text(&snapshot["cloudAccountUi"], "loginCloudUrl"),
    )
    .is_ok()
}
fn subscription_label(status: &str) -> &str {
    match status {
        "active" => "Subscription active",
        "trialing" => "Subscription trial",
        "past_due" => "Payment needs attention",
        "paused" => "Subscription paused",
        "canceled" => "Subscription canceled",
        _ => "No active subscription",
    }
}
fn extraction_project<'a>(snapshot: &'a Value, project: &str) -> &'a Value {
    const EMPTY: Value = Value::Null;
    array(&snapshot["projectExtraction"]["projects"])
        .iter()
        .find(|row| text(row, "projectId") == project)
        .unwrap_or(&EMPTY)
}
fn extraction_policy(snapshot: &Value, project: &str) -> Option<(u64, bool)> {
    let metadata = &snapshot["projectExtraction"];
    if metadata.is_null() {
        return Some((0, false));
    }
    if metadata["schemaVersion"] != 7
        || metadata["error"] == true
        || metadata["revision"].as_u64().is_none()
        || !metadata["projects"].is_array()
    {
        return None;
    }
    let row = extraction_project(snapshot, project);
    if row.is_null() {
        return Some((0, false));
    }
    if !row["recentJobs"].is_array()
        || [
            "queued",
            "running",
            "completed",
            "failed",
            "cancelled",
            "interrupted",
        ]
        .iter()
        .any(|key| row["counts"][key].as_u64().is_none())
    {
        return None;
    }
    Some((
        row["policyRevision"]
            .as_u64()
            .filter(|revision| *revision <= 9_007_199_254_740_991)?,
        row["enabled"].as_bool()?,
    ))
}
fn extraction_jobs<'a>(snapshot: &'a Value, project: &str) -> &'a [Value] {
    array(&extraction_project(snapshot, project)["recentJobs"])
}
fn extraction_job_controls(
    ui: &mut Ui,
    job: &Value,
    project: &str,
    enabled: bool,
    commands: &mut Vec<Command>,
) {
    ui.horizontal_wrapped(|ui| {
        status(ui,extraction_status(job));
        hint(ui,&localized_format!(ui,"Source revision {0} · {1}/3 attempts used",job["sourceRevision"].as_u64().unwrap_or(0),job["attempt"].as_u64().unwrap_or(0)));
        if matches!(text(job,"status"),"failed"|"interrupted"|"cancelled") && text(job,"reasonCode")!="EXTRACTION_SOURCE_CHANGED" {
            action(ui,commands,"Retry drafting","project_extraction_retry",enabled&&text(job,"projectId")==project&&job["attempt"].as_u64().is_some_and(|attempt|attempt<3)&&!text(job,"jobId").is_empty(),||json!({"projectId":project,"jobId":job["jobId"],"expectedAttempt":job["attempt"]}));
        }
    });
    if text(job, "status") == "completed" {
        hint(
            ui,
            if array(&job["proposalIds"]).is_empty() {
                if job["skippedDuplicates"].as_u64().unwrap_or(0) > 0 {
                    "Matching decisions already exist. No duplicate draft was saved."
                } else {
                    "No ADR found in this source."
                }
            } else {
                "Drafts are ready in Decisions. Review and accept them explicitly."
            },
        );
    }
    if let Some(code) = job["reasonCode"].as_str().filter(|code| {
        !code.is_empty() && matches!(text(job, "status"), "failed" | "interrupted" | "cancelled")
    }) {
        let message = t(ui, code);
        hint(
            ui,
            if message == code {
                "Drafting failed. Check Providers and review the source before a manual retry."
            } else {
                message.as_ref()
            },
        );
    }
    ui.collapsing(t(ui, "Drafting job details"), |ui| {
        ui.monospace(text(job, "jobId"));
        ui.monospace(text(job, "sourceId"));
        raw_hint(ui, text(job, "extractorVersion"));
        hint(ui, &format_time(ui, text(job, "updatedAt")));
    });
}
fn extraction_status(job: &Value) -> &'static str {
    if text(job, "reasonCode") == "EXTRACTION_SOURCE_CHANGED" {
        return "Drafting source changed";
    }
    match text(job, "status") {
        "queued" => "Drafting queued",
        "running" => "Drafting running",
        "completed" => "Drafting completed",
        "failed" => "Drafting failed",
        "cancelled" => "Drafting cancelled",
        "interrupted" => "Drafting interrupted",
        _ => "Drafting state unavailable",
    }
}
fn extraction_provenance(ui: &mut Ui, decision: &Value) {
    let extraction = &decision["extraction"];
    if extraction.is_null() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        status(
            ui,
            if text(extraction, "providerId") == "codex" {
                "Generated by Codex"
            } else {
                "Generated proposal"
            },
        );
        raw_hint(ui, text(extraction, "extractorVersion"));
    });
    hint(ui, "Model suggestion · verify the supporting evidence.");
    match text(extraction, "relation") {
        "possible_conflict" => {
            ui.colored_label(
                colors(ui).danger,
                t(ui, "Possible conflict · compare with accepted decisions."),
            );
        }
        "possible_duplicate" => hint(ui, "Possible duplicate · compare with accepted decisions."),
        _ => {}
    }
    egui::ScrollArea::vertical()
        .id_salt(("extraction-evidence", text(decision, "decisionId")))
        .max_height(150.0)
        .show(ui, |ui| {
            for evidence in array(&extraction["evidence"]).iter().take(8) {
                ui.label(
                    RichText::new(t(ui, "Supporting excerpt"))
                        .size(12.0)
                        .color(colors(ui).muted),
                );
                ui.label(text(evidence, "excerpt"));
                hint(
                    ui,
                    &localized_format!(ui, "Source revision {0}", revision(evidence)),
                );
            }
        });
    ui.collapsing(t(ui, "Generator provenance"), |ui| {
        ui.monospace(text(extraction, "jobId"));
        ui.monospace(text(extraction, "sourceId"));
        raw_hint(ui, text(extraction, "providerId"));
        raw_hint(ui, text(extraction, "extractorVersion"));
        hint(
            ui,
            &localized_format!(
                ui,
                "Source revision {0}",
                extraction["sourceRevision"].as_u64().unwrap_or(0)
            ),
        );
        for id in array(&extraction["relatedDecisionIds"]).iter().take(20) {
            if let Some(id) = id.as_str() {
                ui.monospace(id);
            }
        }
    });
    ui.add_space(12.0);
}
fn shared_project<'a>(snapshot: &'a Value, project: &str) -> &'a Value {
    const EMPTY: Value = Value::Null;
    array(&snapshot["sharedMemory"]["projects"])
        .iter()
        .find(|p| text(p, "projectId") == project)
        .unwrap_or(&EMPTY)
}
fn shared_publisher_ready(metadata: &Value, cloud: &Value) -> bool {
    cloud["paired"] == true
        && cloud["error"] != true
        && text(metadata, "role") == "publisher"
        && metadata["sharedProjectId"].is_string()
        && metadata["enabled"] == true
        && metadata["pendingOperation"].is_null()
        && text(metadata, "state") != "connection_changed"
}
fn shared_role_label(role: &str) -> &str {
    match role {
        "publisher" => "Publisher installation",
        "reader" => "Reader installation",
        _ => "Not linked",
    }
}
fn shared_state_label(state: &str) -> &str {
    match state {
        "queued" => "Queued locally",
        "pending" => "Delivery pending",
        "ready" => "Shared project ready",
        "error" => "Cloud verification failed",
        "connection_changed" => "Connection changed",
        _ => "Private Local project",
    }
}
fn opaque_id(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(&format!("{prefix}:"))
        .and_then(|suffix| {
            uuid::Uuid::parse_str(suffix).ok().map(|id| {
                id.to_string() == suffix
                    && id.get_variant() == uuid::Variant::RFC4122
                    && (1..=8).contains(&id.get_version_num())
            })
        })
        .unwrap_or(false)
}
fn publication_preview(detail: &Value) -> Option<Value> {
    let decision = &detail["decision"];
    if text(decision, "status") != "accepted"
        || revision(decision) == 0
        || text(decision, "title").is_empty()
        || text(decision, "decision").is_empty()
    {
        return None;
    }
    let accepted_at = array(&detail["history"])
        .iter()
        .find(|entry| text(entry, "status") == "accepted" && revision(entry) == revision(decision))
        .and_then(|entry| entry["at"].as_str())?;
    chrono::DateTime::parse_from_rfc3339(accepted_at).ok()?;
    let sources = array(&decision["sources"]);
    if !(1..=8).contains(&sources.len())
        || sources
            .iter()
            .any(|s| text(s, "sourceId").is_empty() || revision(s) == 0)
    {
        return None;
    }
    Some(
        json!({"decisionId":decision["decisionId"],"revision":decision["revision"],"status":"accepted","title":decision["title"],"context":decision["context"],"decision":decision["decision"],"rationale":decision["rationale"],"consequences":decision["consequences"],"acceptedAt":accepted_at,"sources":sources.iter().map(|s|json!({"sourceId":s["sourceId"],"revision":s["revision"]})).collect::<Vec<_>>(),"supersedesDecisionId":decision["supersedesDecisionId"]}),
    )
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
fn colors(ui: &Ui) -> studio::Palette {
    studio::palette(ui.ctx())
}

fn hint(ui: &mut Ui, value: &str) {
    ui.label(
        RichText::new(t(ui, value))
            .size(12.0)
            .color(colors(ui).muted),
    );
}
fn raw_hint(ui: &mut Ui, value: &str) {
    ui.label(RichText::new(value).size(12.0).color(colors(ui).muted));
}

fn path_label(ui: &mut Ui, value: &str) {
    ui.add(egui::Label::new(RichText::new(value).size(12.0).color(colors(ui).muted)).truncate())
        .on_hover_text(value);
}

fn folder_preview(ui: &mut Ui, path: &str) {
    egui::Frame::new()
        .fill(colors(ui).raised)
        .stroke(egui::Stroke::new(1.0, colors(ui).border))
        .corner_radius(8)
        .inner_margin(12)
        .show(ui, |ui| {
            let text_width = (ui.available_width() - 28.0 - ui.spacing().item_spacing.x).max(0.0);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                draw_icon(
                    ui.painter(),
                    rect.shrink(4.0),
                    Tab::Projects,
                    colors(ui).primary,
                );
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
                                egui::Label::new(
                                    RichText::new(name).strong().color(colors(ui).text),
                                )
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
    let p = colors(ui);
    let (color, background) = match value {
        "available" | "completed" | "applied" | "accepted" | "active" | "Connected"
        | "Codex ready" | "Paired" => (p.success, p.success_surface),
        "running" | "launching" | "queued" | "proposed" => (p.info, p.info_surface),
        "cancelling" | "waiting_question" | "waiting_approval" => (p.warning, p.warning_surface),
        "failed" | "rejected" | "timed_out" => (p.danger, p.danger_surface),
        _ => (p.muted, p.raised),
    };
    let label = t(ui, value).replace('_', " ");
    paint_status(ui, label, color, background);
}
fn raw_status(ui: &mut Ui, value: &str) {
    paint_status(ui, value.to_owned(), colors(ui).muted, colors(ui).raised);
}
fn paint_status(ui: &mut Ui, label: String, color: Color32, background: Color32) {
    let galley =
        ui.painter()
            .layout_no_wrap(label.clone(), egui::FontId::proportional(12.0), color);
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + egui::vec2(16.0, 8.0), egui::Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), &label)
    });
    ui.painter().rect_filled(rect, 8, background);
    ui.painter()
        .galley(rect.min + egui::vec2(8.0, 4.0), galley, color);
}
fn studio_metric(ui: &mut Ui, label: &str, count: u64, icon: Tab) {
    egui::Frame::new()
        .fill(colors(ui).surface)
        .stroke(egui::Stroke::new(1.0, colors(ui).border))
        .corner_radius(10)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t(ui, label))
                        .size(10.0)
                        .color(colors(ui).muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
                    draw_icon(ui.painter(), rect, icon, colors(ui).muted);
                });
            });
            ui.add_space(8.0);
            ui.label(studio::title(count.to_string(), 28.0));
        });
}

fn studio_flow(ui: &mut Ui) {
    egui::Frame::new()
        .fill(colors(ui).info_surface)
        .stroke(egui::Stroke::new(1.0, colors(ui).border))
        .corner_radius(12)
        .inner_margin(24)
        .show(ui, |ui| {
            ui.label(
                RichText::new(t(ui, "BUILT AROUND YOUR PROJECT"))
                    .size(11.0)
                    .color(colors(ui).primary)
                    .extra_letter_spacing(1.3),
            );
            studio::mascot(ui, ui.available_width().min(260.0));
            ui.add_space(12.0);
            ui.label(studio::title(t(ui, "One project. Shared direction."), 24.0));
            ui.add_space(22.0);
            for (number, title, detail) in [
                (
                    "01",
                    "Connect your folder",
                    "Your code stays on this device.",
                ),
                (
                    "02",
                    "Bring your agents together",
                    "Bounded roles, one visible workflow.",
                ),
                (
                    "03",
                    "Keep the decisions",
                    "Shared context. Explicit acceptance.",
                ),
            ] {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(number)
                            .monospace()
                            .size(10.0)
                            .color(colors(ui).primary),
                    );
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(studio::title(t(ui, title), 13.0));
                        ui.label(
                            RichText::new(t(ui, detail))
                                .size(12.0)
                                .color(colors(ui).muted),
                        );
                    });
                });
                ui.add_space(18.0);
            }
        });
}
fn card(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(colors(ui).surface)
        .stroke(egui::Stroke::new(1.0, colors(ui).border))
        .corner_radius(12)
        .inner_margin(20)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), contents);
        });
}
fn empty(ui: &mut Ui, title: &str, description: &str) {
    card(ui, |ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 28.0), egui::Sense::hover());
        studio::mark(ui.painter(), rect);
        ui.add_space(12.0);
        ui.label(studio::title(t(ui, title), 20.0));
        ui.add_space(8.0);
        hint(ui, description);
    });
    ui.add_space(14.0);
}
fn field(ui: &mut Ui, label: &str, value: &mut String, max: usize, password: bool) {
    ui.label(
        RichText::new(t(ui, label))
            .size(12.0)
            .color(colors(ui).muted),
    );
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
    ui.label(
        RichText::new(t(ui, label))
            .size(12.0)
            .color(colors(ui).muted),
    );
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
fn project_switcher(ui: &mut Ui, id: &str, selected: &mut String, projects: &[Value]) {
    ui.horizontal_wrapped(|ui| {
        let (icon, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        draw_icon(ui.painter(), icon, Tab::Projects, colors(ui).primary);
        ui.label(
            RichText::new(t(ui, "Project"))
                .size(12.0)
                .color(colors(ui).muted),
        );
        let name = projects
            .iter()
            .find(|p| text(p, "projectId") == selected)
            .map(|p| std::borrow::Cow::Borrowed(text(p, "name")))
            .unwrap_or_else(|| t(ui, "Choose…"));
        egui::ComboBox::from_id_salt(id)
            .selected_text(name)
            .truncate()
            .width((ui.available_width() - 2.0).clamp(150.0, 360.0))
            .show_ui(ui, |ui| {
                ui.selectable_value(selected, String::new(), t(ui, "None"));
                for project in projects {
                    ui.selectable_value(
                        selected,
                        text(project, "projectId").to_owned(),
                        text(project, "name"),
                    );
                }
            });
    });
    ui.add_space(10.0);
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
    ui.label(
        RichText::new(t(ui, label))
            .size(12.0)
            .color(colors(ui).muted),
    );
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
                .color(colors(ui).on_primary),
        )
        .fill(colors(ui).primary)
        .stroke(egui::Stroke::NONE)
        .corner_radius(12)
        .min_size(egui::vec2(128.0, 44.0)),
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
        egui::Button::new(
            RichText::new(t(ui, label))
                .size(12.0)
                .color(colors(ui).text),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(egui::Stroke::new(1.0, colors(ui).control_border))
        .corner_radius(8)
        .min_size(egui::vec2(74.0, 36.0)),
    );
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(egui::Id::new(("test-action", name, label)), response.rect);
        data.insert_temp(
            egui::Id::new(("test-action-state", name, label)),
            (response.enabled(), response.rect, ui.clip_rect()),
        );
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
    ui.label(studio::title(t(ui, title), 18.0));
    ui.add_space(3.0);
    hint(ui, description);
}
fn field_hint(ui: &mut Ui, label: &str, value: &mut String, max: usize, placeholder: &str) {
    ui.label(
        RichText::new(t(ui, label))
            .size(12.0)
            .color(colors(ui).muted),
    );
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
fn navigation(ui: &mut Ui, tab: Tab, selected: bool, count: usize, reveal: bool) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::click());
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
    if selected && reveal {
        response.scroll_to_me(Some(egui::Align::Center));
    }
    let background = if selected {
        colors(ui).info_surface
    } else if response.hovered() {
        colors(ui).raised
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 7, background);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            7,
            egui::Stroke::new(1.0, colors(ui).primary),
            egui::StrokeKind::Inside,
        );
    }
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 8.0), egui::vec2(2.0, 18.0)),
            1,
            colors(ui).primary,
        );
    }
    let color = if selected {
        colors(ui).text
    } else {
        colors(ui).muted
    };
    draw_icon(
        ui.painter(),
        egui::Rect::from_min_size(rect.min + egui::vec2(11.0, 8.0), egui::vec2(18.0, 18.0)),
        tab,
        if selected {
            colors(ui).primary
        } else {
            colors(ui).muted
        },
    );
    ui.painter().text(
        rect.min + egui::vec2(39.0, 18.0),
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
            colors(ui).muted,
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
    fn test_context() -> egui::Context {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        ctx
    }

    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    fn extraction_snapshot(enabled: bool) -> Value {
        let mut data = decision_snapshot("proposed");
        data["projectExtraction"] = json!({"schemaVersion":7,"revision":4,"projects":[{"projectId":"project:fixture","policyRevision":2,"enabled":enabled,"counts":{"queued":0,"running":0,"completed":0,"failed":0,"cancelled":0,"interrupted":0},"recentJobs":[]}]});
        data["memoryPage"]["extractionRevision"] = json!(4);
        data
    }
    fn extraction_console() -> Console {
        Console {
            tab: Tab::Memory,
            ..Default::default()
        }
    }
    fn extraction_job(data: &mut Value, status: &str, attempt: u64, reason: &str) {
        data["projectExtraction"]["projects"][0]["counts"][status] = json!(1);
        data["projectExtraction"]["projects"][0]["recentJobs"] = json!([{"jobId":"extraction:fixture","projectId":"project:fixture","sourceId":"source:fixture","sourceRevision":3,"extractorVersion":"codex-adr-v1","status":status,"attempt":attempt,"reasonCode":reason,"proposalIds":[],"skippedDuplicates":0}]);
        data["memoryPage"]["sources"][0]["extractionJob"] =
            data["projectExtraction"]["projects"][0]["recentJobs"][0].clone();
    }
    #[test]
    fn older_failed_source_job_is_reachable_beyond_ten_recent_jobs_and_requires_fresh_metadata() {
        let ctx = test_context();
        let mut data = extraction_snapshot(true);
        extraction_job(&mut data, "failed", 1, "EXTRACTION_PROVIDER_FAILED");
        let old = data["memoryPage"]["sources"][0]["extractionJob"].clone();
        data["projectExtraction"]["projects"][0]["recentJobs"]=Value::Array((0..10).map(|index|json!({"jobId":format!("extraction:new-{index}"),"projectId":"project:fixture","sourceId":format!("source:new-{index}"),"sourceRevision":1,"status":"completed","attempt":1,"proposalIds":[],"skippedDuplicates":0,"reasonCode":"EXTRACTION_NO_ADR"})).collect());
        data["projectExtraction"]["projects"][0]["counts"]["completed"] = json!(10);
        assert_eq!(
            data["projectExtraction"]["projects"][0]["recentJobs"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
        assert!(!extraction_jobs(&data, "project:fixture")
            .iter()
            .any(|job| job["jobId"] == old["jobId"]));
        let mut console = extraction_console();
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_retry",
            "Retry drafting",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"projectId":"project:fixture","jobId":"extraction:fixture","expectedAttempt":1})
        );
        assert!(!painted_copy(&ctx, &mut console, &data)
            .join("\n")
            .contains("Bu kaynaktan karar taslağı üret"));
        data["projectExtraction"]["revision"] = json!(5);
        let commands = render(&ctx, &mut console, &data, false, vec![]).0;
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_memory_read");
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_retry",
            "Retry drafting"
        )
        .iter()
        .all(|command| command.action != "project_extraction_retry"));
        assert!(
            render(&ctx, &mut console, &data, false, vec![])
                .0
                .is_empty(),
            "unchanged failed page read must not loop"
        );
        data["memoryPage"]["extractionRevision"] = json!(5);
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_retry",
            "Retry drafting",
        );
        assert_eq!(commands[0].payload["jobId"], old["jobId"]);
        data["memoryPage"]["sources"][0]["extractionJob"]["projectId"] = json!("project:foreign");
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .iter()
            .all(|command| command.action != "project_extraction_enqueue"
                && command.action != "project_extraction_retry"));
    }
    #[test]
    fn extraction_only_revision_refresh_is_bounded_and_preserves_failed_owner_recovery() {
        let ctx = test_context();
        let mut data = extraction_snapshot(true);
        let mut console = extraction_console();
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        data["projectExtraction"]["revision"] = json!(5);
        data["memoryPage"] = Value::Null;
        let notice = Notice::error("project_extraction_retry", "HOST_ACTION_FAILED");
        let mut commands = Vec::new();
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 1200.0))),
                ..Default::default()
            },
            |ui| commands = console.show(ui, &data, false, Some(&notice), true),
        );
        output.drop_without_applying_deltas();
        assert!(commands.is_empty());
        assert!(console.memory.requested.is_none());
        let commands = render(&ctx, &mut console, &data, false, vec![]).0;
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_memory_read");
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
    }
    #[test]
    fn manual_draft_sources_refresh_once_after_extraction_only_page_invalidation() {
        let ctx = test_context();
        let mut data = extraction_snapshot(true);
        let mut console = Console {
            tab: Tab::Decisions,
            ..Default::default()
        };
        console.decisions.select("project:fixture");
        console.decisions.selected = "decision:fixture".into();
        console.decisions.composer_open = true;
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        data["projectExtraction"]["revision"] = json!(5);
        data["memoryPage"] = Value::Null;
        let commands = render(&ctx, &mut console, &data, false, vec![]).0;
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_memory_read");
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
    }

    #[test]
    fn positive_drafting_states_do_not_paint_failure_or_retry_guidance_in_either_language() {
        for language in [Language::Tr, Language::En] {
            for (status, code, proposal_ids, duplicates) in [
                ("running", "EXTRACTION_RUNNING", json!([]), 0),
                (
                    "completed",
                    "EXTRACTION_COMPLETED",
                    json!(["decision:generated"]),
                    0,
                ),
                ("completed", "EXTRACTION_NO_ADR", json!([]), 0),
                ("completed", "EXTRACTION_DUPLICATE", json!([]), 1),
            ] {
                let ctx = test_context();
                let job = json!({"jobId":"extraction:fixture","projectId":"project:fixture","sourceId":"source:fixture","sourceRevision":1,"status":status,"reasonCode":code,"proposalIds":proposal_ids,"skippedDuplicates":duplicates,"attempt":1});
                let mut commands = Vec::new();
                let output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(800.0, 560.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        ui.data_mut(|d| d.insert_temp(egui::Id::new("console-language"), language));
                        extraction_job_controls(ui, &job, "project:fixture", true, &mut commands);
                    },
                );
                let copy = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.text().to_lowercase()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                output.drop_without_applying_deltas();
                assert!(
                    !copy.contains("failed")
                        && !copy.contains("failure")
                        && !copy.contains("retry")
                        && !copy.contains("başarısız")
                        && !copy.contains("yeniden"),
                    "{code}: {copy}"
                );
                assert!(!copy.contains("extraction_"));
                assert!(commands.is_empty());
            }
        }
    }

    #[cfg(feature = "ui-smoke")]
    #[test]
    fn memory_smoke_opens_initial_disclosure_without_enabling_or_prechecking_consent() {
        let ctx = test_context();
        let mut console = Console::default();
        console.memory_smoke();
        let data = extraction_snapshot(false);
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        assert!(console.extraction.setup_open);
        assert!(!console.extraction.consent);
        assert!(ctx
            .data(|d| d.get_temp::<Rect>(egui::Id::new((
                "test-action",
                "extraction-consent",
                "Allow Codex drafting"
            ))))
            .is_some());
        let data = extraction_snapshot(true);
        render(&ctx, &mut console, &data, false, vec![]);
        assert!(!console.extraction.setup_open);
    }

    #[test]
    fn drafting_is_off_by_default_and_a_real_consent_click_is_required_to_enable() {
        let ctx = test_context();
        let data = decision_snapshot("proposed");
        let mut console = extraction_console();
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Set up drafting",
        );
        assert!(!console.extraction.consent);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_configure",
            "Enable automatic drafting"
        )
        .is_empty());
        click(
            &ctx,
            &mut console,
            &data,
            false,
            "extraction-consent",
            "Allow Codex drafting",
        );
        assert!(console.extraction.consent);
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_configure",
            "Enable automatic drafting",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_extraction_configure");
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"projectId":"project:fixture","expectedPolicyRevision":0,"enabled":true,"consent":true})
        );
        assert!(!console.extraction.consent);
        assert!(!console.extraction.setup_open);
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
    }
    #[test]
    fn drafting_consent_resets_on_project_policy_and_malformed_metadata_without_sending() {
        for change in [
            "project",
            "empty",
            "removed",
            "revision",
            "error",
            "malformed",
        ] {
            let ctx = test_context();
            let mut data = extraction_snapshot(false);
            let mut console = extraction_console();
            click(
                &ctx,
                &mut console,
                &data,
                false,
                "memory_control",
                "Set up drafting",
            );
            click(
                &ctx,
                &mut console,
                &data,
                false,
                "extraction-consent",
                "Allow Codex drafting",
            );
            assert!(console.extraction.consent);
            match change {
                "project" => {
                    data["projectMemory"]["projects"].as_array_mut().unwrap().push(json!({"projectId":"project:other","revision":1,"name":"Work","sourceCount":0}));
                    console.memory.select("project:other");
                }
                "empty" => {
                    data["projectMemory"]["projects"].as_array_mut().unwrap().push(json!({"projectId":"project:other","revision":1,"name":"Other","sourceCount":0}));
                    console.memory.select("");
                }
                "removed" => {
                    data["projectMemory"]["projects"] = json!([]);
                }
                "revision" => data["projectExtraction"]["projects"][0]["policyRevision"] = json!(3),
                "error" => data["projectExtraction"]["error"] = json!(true),
                _ => data["projectExtraction"]["projects"][0]["enabled"] = json!("yes"),
            }
            let commands = render(&ctx, &mut console, &data, false, vec![]).0;
            assert!(!console.extraction.consent, "{change} must discard consent");
            assert!(commands
                .iter()
                .all(|command| !command.action.starts_with("project_extraction_")));
            if matches!(change, "error" | "malformed") {
                assert!(extraction_policy(&data, "project:fixture").is_none());
            }
        }
        let ctx = test_context();
        let data = extraction_snapshot(false);
        let mut console = extraction_console();
        click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Set up drafting",
        );
        click(
            &ctx,
            &mut console,
            &data,
            false,
            "extraction-consent",
            "Allow Codex drafting",
        );
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "project_extraction_configure",
            "Enable automatic drafting"
        )
        .is_empty());
    }
    #[test]
    fn enabled_drafting_can_pause_without_provider_and_running_jobs_do_not_block_local_controls() {
        let ctx = test_context();
        let mut data = extraction_snapshot(true);
        data["observation"] = json!({"observations":[]});
        extraction_job(&mut data, "running", 1, "");
        let mut console = extraction_console();
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_configure",
            "Pause drafting",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"projectId":"project:fixture","expectedPolicyRevision":2,"enabled":false,"consent":false})
        );
        click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Edit message",
        );
        assert!(
            console.memory.edit.is_some(),
            "background inference must not disable local message editing"
        );
    }
    #[test]
    fn existing_source_drafting_requires_current_verified_scope_revision_and_enabled_policy() {
        let ctx = test_context();
        let mut data = extraction_snapshot(true);
        let mut console = extraction_console();
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_enqueue",
            "Draft decision from this source",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"projectId":"project:fixture","sourceId":"source:fixture","expectedSourceRevision":3})
        );
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "project_extraction_enqueue",
            "Draft decision from this source"
        )
        .is_empty());
        extraction_job(&mut data, "completed", 1, "");
        assert!(!painted_copy(&ctx, &mut console, &data)
            .join("\n")
            .contains("Bu kaynaktan karar taslağı üret"));
        data["projectExtraction"]["projects"][0]["recentJobs"][0]["sourceRevision"] = json!(2);
        data["memoryPage"]["sources"][0]["extractionJob"] = Value::Null;
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_enqueue",
            "Draft decision from this source",
        );
        assert_eq!(commands[0].payload["expectedSourceRevision"], 3);
        data["memoryPage"]["revision"] = json!(8);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_enqueue",
            "Draft decision from this source"
        )
        .iter()
        .all(|c| c.action != "project_extraction_enqueue"));
        data["memoryPage"]["revision"] = json!(9);
        data["memoryPage"]["sources"][0]["projectId"] = json!("project:foreign");
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .iter()
            .all(|c| c.action != "project_extraction_enqueue"));
        data["projectExtraction"]["error"] = json!(true);
        assert!(extraction_policy(&data, "project:fixture").is_none());
    }
    #[test]
    fn manual_drafting_retries_keep_attempt_and_block_stale_paused_exhausted_jobs() {
        for state in ["failed", "interrupted", "cancelled"] {
            let ctx = test_context();
            let mut data = extraction_snapshot(true);
            extraction_job(&mut data, state, 2, "EXTRACTION_PROVIDER_FAILED");
            let mut console = extraction_console();
            let commands = click(
                &ctx,
                &mut console,
                &data,
                false,
                "project_extraction_retry",
                "Retry drafting",
            );
            assert_eq!(commands.len(), 1);
            assert_eq!(
                normalized_decision_command(&commands[0]),
                json!({"projectId":"project:fixture","jobId":"extraction:fixture","expectedAttempt":2})
            );
            assert!(click(
                &ctx,
                &mut console,
                &data,
                true,
                "project_extraction_retry",
                "Retry drafting"
            )
            .is_empty());
            data["projectExtraction"]["projects"][0]["recentJobs"][0]["attempt"] = json!(3);
            data["memoryPage"]["sources"][0]["extractionJob"]["attempt"] = json!(3);
            assert!(click(
                &ctx,
                &mut console,
                &data,
                false,
                "project_extraction_retry",
                "Retry drafting"
            )
            .is_empty());
            data["projectExtraction"]["projects"][0]["recentJobs"][0]["attempt"] = json!(1);
            data["memoryPage"]["sources"][0]["extractionJob"]["attempt"] = json!(1);
            data["projectExtraction"]["projects"][0]["enabled"] = json!(false);
            assert!(click(
                &ctx,
                &mut console,
                &data,
                false,
                "project_extraction_retry",
                "Retry drafting"
            )
            .is_empty());
        }
        let ctx = test_context();
        let mut data = extraction_snapshot(true);
        extraction_job(&mut data, "cancelled", 0, "EXTRACTION_POLICY_CHANGED");
        let mut console = extraction_console();
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_extraction_retry",
            "Retry drafting",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["expectedAttempt"], 0);
        let ctx = test_context();
        let mut data = extraction_snapshot(true);
        extraction_job(&mut data, "cancelled", 1, "EXTRACTION_SOURCE_CHANGED");
        let mut console = extraction_console();
        let copy = painted_copy(&ctx, &mut console, &data).join("\n");
        assert!(copy.contains("Taslağın kaynağı değişti"));
        assert!(!copy.contains("Taslağı yeniden dene"));
        data["projectExtraction"]["projects"][0]["recentJobs"][0]["reasonCode"] =
            json!("do not show PRIVATE_STDERR_PATH");
        data["memoryPage"]["sources"][0]["extractionJob"]["reasonCode"] =
            json!("do not show PRIVATE_STDERR_PATH");
        assert!(!painted_copy(&ctx, &mut console, &data)
            .join("\n")
            .contains("PRIVATE_STDERR_PATH"));
    }
    #[test]
    fn generated_provenance_is_localized_while_owner_evidence_and_acceptance_stay_explicit() {
        for lang in ["tr", "en"] {
            let ctx = test_context();
            let mut data = extraction_snapshot(true);
            data["desktopPreferences"] = json!({"language":lang});
            data["decisionDetail"]["decision"]["extraction"] = json!({"providerId":"codex","extractorVersion":"codex-adr-v1","jobId":"job:private","sourceId":"source:fixture","sourceRevision":3,"evidence":[{"sourceId":"source:fixture","revision":3,"excerpt":"Memory · owner original alıntı"}],"relation":"possible_conflict","relatedDecisionIds":["decision:accepted-private"]});
            let mut console = Console {
                tab: Tab::Decisions,
                ..Default::default()
            };
            console.decisions.select("project:fixture");
            console.decisions.selected = "decision:fixture".into();
            assert!(render(&ctx, &mut console, &data, false, vec![])
                .0
                .is_empty());
            for status in ["proposed", "accepted"] {
                data["decisionDetail"]["decision"]["status"] = json!(status);
                let output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(800.0, 560.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        ui.data_mut(|d| {
                            d.insert_temp(
                                egui::Id::new("console-language"),
                                Language::from_code(lang),
                            )
                        });
                        extraction_provenance(ui, &data["decisionDetail"]["decision"]);
                    },
                );
                let copy = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                output.drop_without_applying_deltas();
                assert!(copy.contains(if lang == "tr" {
                    "Codex tarafından üretildi"
                } else {
                    "Generated by Codex"
                }));
                assert!(copy.contains("Memory · owner original alıntı"));
                assert!(copy.contains(if lang == "tr" {
                    "Olası çelişki"
                } else {
                    "Possible conflict"
                }));
                assert!(!copy.contains("job:private"));
                assert!(!copy.contains("decision:accepted-private"));
            }
            data["decisionDetail"]["decision"]["status"] = json!("proposed");
            let commands = click(
                &ctx,
                &mut console,
                &data,
                false,
                "project_decision_accept",
                "Accept decision",
            );
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].action, "project_decision_accept");
        }
    }
    #[test]
    fn drafting_disclosure_and_controls_fit_the_minimum_native_width_in_both_languages() {
        for language in [Language::Tr, Language::En] {
            let ctx = test_context();
            let data = extraction_snapshot(false);
            let mut console = extraction_console();
            console.extraction.scope("project:fixture", 2);
            console.extraction.setup_open = true;
            let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 560.0));
            let mut commands = Vec::new();
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("console-language"), language));
                    ui.set_max_width(520.0);
                    console.extraction_controls(
                        ui,
                        &data,
                        &data["projectMemory"]["projects"][0],
                        &mut commands,
                    );
                },
            );
            output.drop_without_applying_deltas();
            let consent = ctx.data(|d| {
                d.get_temp::<Rect>(egui::Id::new((
                    "test-action",
                    "extraction-consent",
                    "Allow Codex drafting",
                )))
                .unwrap()
            });
            let (_, button, clip) = ctx.data(|d| {
                d.get_temp::<(bool, Rect, Rect)>(egui::Id::new((
                    "test-action-state",
                    "project_extraction_configure",
                    "Enable automatic drafting",
                )))
                .unwrap()
            });
            assert!(screen.contains_rect(consent));
            assert!(screen.contains_rect(button));
            assert!(clip.contains_rect(button));
            assert!(commands.is_empty());
        }
    }

    fn normalized_decision_command(command: &Command) -> Value {
        let mut payload = command.payload.clone();
        for key in ["commandId", "createdAt", "expiresAt"] {
            payload.as_object_mut().unwrap().remove(key);
        }
        payload
    }
    const SHARED_ID: &str = "shared:60c029a4-bc68-44c8-87c9-08d0c2688e9b";
    fn account_snapshot(waiting: bool) -> Value {
        let mut data = snapshot();
        data["cloudAccountUi"] = json!({"busy":false,"error":null,"logoutRemoteRevoked":null});
        if waiting {
            let url=format!("https://cloud.example.invalid/desktop/authorize?schemaVersion=1&installationId=install%3Atest&displayName=Native%20test&appVersion=0.8.0&protocolVersion=2&osFamily=darwin&redirectUri=http%3A%2F%2F127.0.0.1%3A49152%2Fauth%2Fcallback&state={}&codeChallenge={}","a".repeat(64),"A".repeat(43));
            data["cloudLogin"] = json!({"schemaVersion":1,"status":"waiting","authorizationUrl":url,"expiresAt":"2026-10-15T23:59:00.000Z","confirmationCode":"AAAA-AAAA"});
            data["cloudAccountUi"]["loginCloudUrl"] = json!("https://cloud.example.invalid");
            data["cloudAccountUi"]["openCommandId"] = json!("cmd:begin-one");
        } else {
            data["cloud"] = json!({"paired":true,"cloudUrl":"https://cloud.example.invalid","hostId":"host:fixture","workspaceId":"workspace:fixture"});
            data["cloudAccount"] = json!({"schemaVersion":1,"hostId":"host:fixture","workspaceId":"workspace:fixture","accountId":"33333333-3333-4333-8333-333333333333","email":"Projects@example.invalid","subscription":{"status":"active","memoryAccess":true,"periodEndsAt":"2026-11-15T01:00:00.000Z","checkedAt":"2026-10-15T01:00:00.000Z"}});
        }
        data
    }
    fn account_output(
        ctx: &egui::Context,
        console: &mut Console,
        data: &Value,
        events: Vec<Event>,
    ) -> (Vec<Command>, Vec<egui::OutputCommand>) {
        let mut commands = vec![];
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 560.0))),
                events,
                ..Default::default()
            },
            |ui| {
                commands = console.show(ui, data, false, None, false);
            },
        );
        let browser = output.platform_output.commands.clone();
        output.drop_without_applying_deltas();
        (commands, browser)
    }
    fn account_click(
        ctx: &egui::Context,
        console: &mut Console,
        data: &Value,
        action: &str,
        label: &str,
    ) -> (Vec<Command>, Vec<egui::OutputCommand>) {
        account_output(ctx, console, data, vec![]);
        let rect: Rect = ctx.data(|d| {
            d.get_temp(egui::Id::new(("test-action", action, label)))
                .unwrap()
        });
        account_output(
            ctx,
            console,
            data,
            vec![
                Event::PointerMoved(rect.center()),
                Event::PointerButton {
                    pos: rect.center(),
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerButton {
                    pos: rect.center(),
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ],
        )
    }

    #[test]
    fn account_sign_in_is_explicit_configurable_and_never_triggered_by_typing() {
        let ctx = test_context();
        let data = snapshot();
        let mut console = Console {
            tab: Tab::Cloud,
            ..Default::default()
        };
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Sign in to Cloud"
        )
        .is_empty());
        assert!(console.account_advanced);
        assert!(console.account_form_error.is_some());
        console.cloud_url = "https://cloud.example.invalid".into();
        console.host_name = "  Work laptop  ".into();
        assert!(render(
            &ctx,
            &mut console,
            &data,
            false,
            vec![Event::Text("typing only".into())]
        )
        .0
        .is_empty());
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Sign in to Cloud",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "cloud_login_begin");
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"cloudUrl":"https://cloud.example.invalid","displayName":"Work laptop"})
        );
        assert!(commands
            .iter()
            .all(|c| !c.action.starts_with("memory_cloud_")));
    }

    #[test]
    fn browser_opens_once_per_explicit_begin_and_rejects_foreign_urls() {
        let ctx = test_context();
        let mut data = account_snapshot(true);
        let mut console = Console {
            tab: Tab::Cloud,
            ..Default::default()
        };
        let (_, output) = account_output(&ctx, &mut console, &data, vec![]);
        let urls: Vec<_> = output
            .iter()
            .filter_map(|o| {
                if let egui::OutputCommand::OpenUrl(url) = o {
                    Some(url)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].new_tab);
        assert_eq!(urls[0].url, data["cloudLogin"]["authorizationUrl"]);
        for _ in 0..4 {
            assert!(account_output(&ctx, &mut console, &data, vec![])
                .1
                .is_empty());
        }
        data["cloudAccountUi"]["busy"] = json!(true);
        assert!(account_output(&ctx, &mut console, &data, vec![])
            .1
            .is_empty());
        data["cloudAccountUi"]["busy"] = json!(false);
        let (commands, output) =
            account_click(&ctx, &mut console, &data, "memory_control", "Open browser");
        assert!(commands.is_empty());
        assert!(output
            .iter()
            .any(|o| matches!(o, egui::OutputCommand::OpenUrl(_))));
        let (_, output) = account_click(
            &ctx,
            &mut console,
            &data,
            "memory_control",
            "Copy sign-in link",
        );
        assert!(output
            .iter()
            .any(|o| matches!(o, egui::OutputCommand::CopyText(_))));
        let commands = account_click(
            &ctx,
            &mut console,
            &data,
            "cloud_login_cancel",
            "Cancel sign-in",
        )
        .0;
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "cloud_login_cancel");
        data["cloudAccountUi"]["openCommandId"] = json!("cmd:begin-two");
        data["cloudLogin"]["authorizationUrl"] =
            json!("https://evil.example.invalid/desktop/authorize");
        assert!(account_output(&ctx, &mut console, &data, vec![])
            .1
            .is_empty());
        assert!(
            account_click(&ctx, &mut console, &data, "memory_control", "Open browser")
                .1
                .is_empty()
        );
        data["cloudLogin"]["authorizationUrl"] = json!("file:///private/synthetic");
        assert!(account_output(&ctx, &mut console, &data, vec![])
            .1
            .is_empty());
    }

    #[test]
    fn verified_account_and_billing_link_are_scoped_and_owner_text_stays_original() {
        for language in ["tr", "en"] {
            let ctx = test_context();
            let mut data = account_snapshot(false);
            data["desktopPreferences"] = json!({"language":language});
            let mut console = Console {
                tab: Tab::Cloud,
                ..Default::default()
            };
            let copy = painted_copy(&ctx, &mut console, &data).join("\n");
            assert!(copy.contains("Projects@example.invalid"));
            assert!(copy.contains(if language == "tr" {
                "Abonelik etkin"
            } else {
                "Subscription active"
            }));
            let commands = account_click(
                &ctx,
                &mut console,
                &data,
                "cloud_account_read",
                "Refresh account",
            )
            .0;
            assert_eq!(commands[0].action, "cloud_account_read");
            let (_, output) = account_click(
                &ctx,
                &mut console,
                &data,
                "memory_control",
                "Subscription and billing",
            );
            assert!(output.iter().any(|o|matches!(o,egui::OutputCommand::OpenUrl(url) if url.url=="https://cloud.example.invalid/#plan-title" && url.new_tab)));
            let commands = account_click(&ctx, &mut console, &data, "cloud_logout", "Sign out").0;
            assert_eq!(commands[0].action, "cloud_logout");
            data["cloudAccount"]["hostId"] = json!("host:foreign");
            assert!(!painted_copy(&ctx, &mut console, &data)
                .join("\n")
                .contains("Projects@example.invalid"));
            data["cloudAccount"]["hostId"] = json!("host:fixture");
            data["cloudAccount"]["workspaceId"] = json!("workspace:foreign");
            assert!(!painted_copy(&ctx, &mut console, &data)
                .join("\n")
                .contains("Projects@example.invalid"));
        }
    }

    #[test]
    fn account_states_are_localized_truthful_and_fit_the_minimum_window() {
        for language in ["tr", "en"] {
            for waiting in [false, true] {
                let ctx = test_context();
                let mut data = account_snapshot(waiting);
                data["desktopPreferences"] = json!({"language":language});
                let mut console = Console {
                    tab: Tab::Cloud,
                    ..Default::default()
                };
                account_output(&ctx, &mut console, &data, vec![]);
                for (action, label) in if waiting {
                    vec![
                        ("cloud_login_cancel", "Cancel sign-in"),
                        ("memory_control", "Open browser"),
                    ]
                } else {
                    vec![
                        ("cloud_logout", "Sign out"),
                        ("cloud_account_read", "Refresh account"),
                    ]
                } {
                    let rect: Rect = ctx.data(|d| {
                        d.get_temp(egui::Id::new(("test-action", action, label)))
                            .unwrap()
                    });
                    assert!(
                        rect.min.x >= 0.0
                            && rect.max.x <= 800.0
                            && rect.min.y >= 0.0
                            && rect.max.y <= 560.0,
                        "{language} {label}: {rect:?}"
                    );
                }
            }
            let ctx = test_context();
            let mut data = snapshot();
            data["desktopPreferences"] = json!({"language":language});
            data["cloudAccountUi"] = json!({"logoutRemoteRevoked":false});
            let mut console = Console {
                tab: Tab::Cloud,
                ..Default::default()
            };
            let copy = painted_copy(&ctx, &mut console, &data).join("\n");
            assert!(copy.contains(if language == "tr" {
                "Bu cihazdaki oturum kapatıldı"
            } else {
                "Signed out on this device"
            }));
            for state in ["denied", "expired", "cancelled"] {
                data["cloudLogin"] = json!({"schemaVersion":1,"status":state});
                assert!(!account_output(&ctx, &mut console, &data, vec![])
                    .1
                    .iter()
                    .any(|o| matches!(o, egui::OutputCommand::OpenUrl(_))));
            }
        }
    }
    fn search_snapshot(shared: bool) -> Value {
        let mut data = shared_snapshot("reader");
        let result = json!({"schemaVersion":1,"reasonCode":if shared {"SHARED_MEMORY_SEARCH_READ"} else {"DECISION_SEARCH_READ"},"projectId":"project:fixture","revision":if shared {8} else {decision_manifest(&data,"project:fixture")},"query":"Why Rust?","mode":"keyword","limit":5,"matches":[{"decisionId":if shared {"decision:published"} else {"decision:fixture"},"revision":if shared {3} else {2},"title":"SEARCH_OWNER_CANARY Projects","score":2.0}]});
        data[if shared {
            "sharedSearch"
        } else {
            "decisionSearch"
        }] = result;
        if shared {
            data["sharedSearch"]
                .as_object_mut()
                .unwrap()
                .remove("limit");
            data["sharedSearch"]["sharedProjectId"] = json!(SHARED_ID);
            data["sharedSearch"]["manifestRevision"] = json!(4);
            data["sharedSearch"]["model"] = Value::Null;
            data["sharedSearch"]["index"] = json!({"state":"pending","indexed":0,"total":1});
        }
        data
    }
    fn search_console(shared: bool) -> Console {
        let mut console = Console {
            tab: if shared { Tab::Cloud } else { Tab::Decisions },
            ..Default::default()
        };
        if shared {
            console.shared.select("project:fixture");
            console.shared.search.input = "Why Rust?".into();
            console.shared.search_open = true;
        } else {
            console.decisions.select("project:fixture");
            console.decisions.search.input = "Why Rust?".into();
        }
        console
    }

    #[test]
    fn search_input_is_trimmed_bounded_and_never_searches_while_typing() {
        let mut search = SearchPanel {
            input: "  Why Rust?  ".into(),
            ..Default::default()
        };
        assert_eq!(
            search.payload("project:one"),
            Some(json!({"projectId":"project:one","query":"Why Rust?","limit":5}))
        );
        for input in [
            "".to_owned(),
            " \t ".into(),
            "bad\0query".into(),
            "😀".repeat(201),
        ] {
            search.input = input;
            assert!(search.payload("project:one").is_none());
        }
        search.input = "😀".repeat(200);
        assert!(search.payload("project:one").is_some());
        search.limit = 11;
        assert!(search.payload("project:one").is_none());
        for shared in [false, true] {
            let ctx = test_context();
            let data = search_snapshot(shared);
            let mut console = search_console(shared);
            assert!(render(&ctx, &mut console, &data, false, vec![])
                .0
                .iter()
                .all(|c| !c.action.ends_with("_search")));
            let action = if shared {
                "memory_cloud_search"
            } else {
                "project_decision_search"
            };
            let label = if shared {
                "Search Cloud"
            } else {
                "Search Local"
            };
            let commands = click(&ctx, &mut console, &data, false, action, label);
            let search = commands.iter().find(|c| c.action == action).unwrap();
            assert_eq!(
                normalized_decision_command(search),
                json!({"projectId":"project:fixture","query":"Why Rust?","limit":5})
            );
            assert!(click(&ctx, &mut console, &data, true, action, label)
                .iter()
                .all(|c| c.action != action));
            if !shared {
                console.decisions.replacement = "decision:previous-selection".into();
                let commands = click(
                    &ctx,
                    &mut console,
                    &data,
                    false,
                    "memory_control",
                    "Read accepted decision",
                );
                let read = commands
                    .iter()
                    .find(|c| c.action == "project_decision_read")
                    .unwrap();
                assert_eq!(
                    normalized_decision_command(read),
                    json!({"projectId":"project:fixture","decisionId":"decision:fixture"})
                );
                assert!(console.decisions.replacement.is_empty());
            }
        }
    }

    #[test]
    fn cloud_search_discloses_scope_and_fallback_without_translating_owner_text() {
        for lang in ["tr", "en"] {
            let ctx = test_context();
            let mut data = search_snapshot(true);
            data["desktopPreferences"] = json!({"language":lang});
            let mut console = search_console(true);
            for _ in 0..4 {
                painted_copy(&ctx, &mut console, &data);
            }
            let copy = painted_copy(&ctx, &mut console, &data).join("\n");
            assert!(copy.contains("SEARCH_OWNER_CANARY Projects"), "{copy}");
            assert!(copy.contains("Why Rust?"));
            assert!(copy.contains(if lang == "tr" {
                "Yedek anahtar sözcük araması"
            } else {
                "Keyword fallback"
            }));
            assert!(copy.contains(if lang == "tr" {
                "Yazarken bu cihazda kalır"
            } else {
                "Typing stays on this device"
            }));
            assert!(copy.contains(if lang == "tr" {
                "İndeksleme sürüyor"
            } else {
                "Indexing in progress"
            }));
            let commands = click(
                &ctx,
                &mut console,
                &data,
                false,
                "memory_control",
                "Read exact downloaded copy",
            );
            let read = commands
                .iter()
                .find(|c| c.action == "memory_cloud_decision_read")
                .unwrap();
            assert_eq!(
                normalized_decision_command(read),
                json!({"projectId":"project:fixture","decisionId":"decision:published","revision":3})
            );
            data["sharedSearch"]["manifestRevision"] = json!(5);
            console.shared.search_open = true;
            for _ in 0..4 {
                painted_copy(&ctx, &mut console, &data);
            }
            let copy = painted_copy(&ctx, &mut console, &data).join("\n");
            assert!(copy.contains("SEARCH_OWNER_CANARY"));
            assert!(copy.contains(if lang == "tr" {
                "indirip doğrulamak"
            } else {
                "download and verify"
            }));
            assert!(click(
                &ctx,
                &mut console,
                &data,
                false,
                "memory_control",
                "Read exact downloaded copy"
            )
            .iter()
            .all(|c| c.action != "memory_cloud_decision_read"));
        }
    }

    #[test]
    fn search_results_hide_changed_query_limit_scope_and_revisions() {
        for shared in [false, true] {
            let data = search_snapshot(shared);
            for changed in [
                "query",
                "limit",
                "project",
                "revision",
                "shared",
                "remoteRevision",
            ] {
                let ctx = test_context();
                let mut console = search_console(shared);
                let mut changed_data = data.clone();
                let key = if shared {
                    "sharedSearch"
                } else {
                    "decisionSearch"
                };
                match changed {
                    "query" => changed_data[key]["query"] = json!("other query"),
                    "limit" => {
                        if shared {
                            console.shared.search.limit = 10;
                        } else {
                            console.decisions.search.limit = 10;
                        }
                    }
                    "project" => changed_data[key]["projectId"] = json!("project:foreign"),
                    "revision" => changed_data[key]["revision"] = json!(999),
                    "shared" if shared => {
                        changed_data[key]["sharedProjectId"] = json!("shared:foreign")
                    }
                    "remoteRevision" if shared => changed_data[key]["manifestRevision"] = json!(3),
                    _ => continue,
                }
                assert!(
                    !painted_copy(&ctx, &mut console, &changed_data)
                        .join("\n")
                        .contains("SEARCH_OWNER_CANARY"),
                    "{shared} {changed}"
                );
            }
            let ctx = test_context();
            let mut console = search_console(shared);
            let notice = Notice::error("project_decision_propose", "HOST_ACTION_FAILED");
            let mut commands = vec![];
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 1200.0))),
                    ..Default::default()
                },
                |ui| commands = console.show(ui, &data, false, Some(&notice), true),
            );
            output.drop_without_applying_deltas();
            assert!(commands.is_empty());
            assert_eq!(
                if shared {
                    &console.shared.search.input
                } else {
                    &console.decisions.search.input
                },
                "Why Rust?"
            );
        }
    }

    #[test]
    fn search_forms_and_actions_fit_a_narrow_native_panel() {
        for shared in [false, true] {
            for language in [Language::Tr, Language::En] {
                let ctx = test_context();
                let data = search_snapshot(shared);
                let mut console = search_console(shared);
                let mut commands = vec![];
                let output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(800.0, 560.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        ui.data_mut(|d| d.insert_temp(egui::Id::new("console-language"), language));
                        ui.set_max_width(470.0);
                        console.memory_search(ui, &data, shared, &mut commands);
                    },
                );
                output.drop_without_applying_deltas();
                let action = if shared {
                    "memory_cloud_search"
                } else {
                    "project_decision_search"
                };
                let label = if shared {
                    "Search Cloud"
                } else {
                    "Search Local"
                };
                let rect: Rect = ctx.data(|d| {
                    d.get_temp(egui::Id::new(("test-action", action, label)))
                        .unwrap()
                });
                assert!(rect.max.x <= 480.0 && rect.max.y <= 560.0);
                assert!(commands.is_empty());
            }
        }
    }

    #[test]
    fn cloud_search_modal_holds_background_reads_until_explicit_search_or_close() {
        let ctx = test_context();
        let mut data = search_snapshot(true);
        data["sharedMemoryPage"] = Value::Null;
        data["sharedDecisionDetail"] = Value::Null;
        let mut console = search_console(true);
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        assert!(console.shared.page_requested.is_none());
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_search",
            "Search Cloud",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "memory_cloud_search");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Close search",
        );
        assert!(commands.is_empty());
        assert!(!console.shared.search_open);
        let commands = render(&ctx, &mut console, &data, false, vec![]).0;
        assert_eq!(commands[0].action, "memory_cloud_read");
    }

    #[test]
    fn production_find_action_opens_the_search_modal_after_initial_project_selection() {
        let ctx = test_context();
        let data = shared_snapshot("reader");
        let mut console = Console {
            tab: Tab::Cloud,
            ..Default::default()
        };
        render(&ctx, &mut console, &data, false, vec![]);
        assert_eq!(console.shared.project, "project:fixture");
        assert!(!console.shared.search_open);
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Find a published decision",
        );
        assert!(commands.is_empty());
        assert!(console.shared.search_open);
        for _ in 0..3 {
            painted_copy(&ctx, &mut console, &data);
        }
        let copy = painted_copy(&ctx, &mut console, &data).join("\n");
        assert!(copy.contains("Sorgu yalnızca Bulutta ara düğmesiyle"));
        assert!(copy.contains("Aramayı kapat"));
        assert!(console.shared.search.input.is_empty());
    }

    #[cfg(feature = "ui-smoke")]
    #[test]
    fn cloud_search_smoke_startup_initializes_only_after_the_real_project_is_selected() {
        let ctx = test_context();
        let data = search_snapshot(true);
        let mut console = Console::default();
        console.search_smoke(true, "Why Rust?");
        assert!(console.shared.project.is_empty());
        let commands = render(&ctx, &mut console, &data, false, vec![]).0;
        assert!(commands.is_empty());
        assert_eq!(console.shared.project, "project:fixture");
        assert_eq!(console.shared.search.input, "Why Rust?");
        assert!(console.shared.search_open);
        assert!(console.search_smoke_query.is_none());
        for _ in 0..3 {
            painted_copy(&ctx, &mut console, &data);
        }
        let copy = painted_copy(&ctx, &mut console, &data).join("\n");
        assert!(copy.contains("Why Rust?"));
        assert!(copy.contains("SEARCH_OWNER_CANARY Projects"));
        assert!(copy.contains("Yedek anahtar sözcük araması"));
    }
    fn shared_snapshot(role: &str) -> Value {
        let mut data = decision_snapshot("accepted");
        data["cloud"] = json!({"paired":true,"cloudUrl":"https://cloud.example.invalid","hostId":"host:current","workspaceId":"workspace:current"});
        data["decisionDetail"]["history"].as_array_mut().unwrap().push(json!({"status":"accepted","revision":2,"actorId":"local-owner","at":"2026-10-15T01:00:00.000Z"}));
        let mut mapping = json!({"projectId":"project:fixture","sharedProjectId":SHARED_ID,"role":role,"enabled":true,"sharedRevision":4,"state":"ready","pendingOperation":null,"verifiedAt":"2026-10-15T01:01:00.000Z","lastErrorCode":null,"stale":false,"offline":false,"diverged":false});
        data["sharedMemory"] = json!({"schemaVersion":5,"revision":8,"projects":[mapping.clone()]});
        mapping["reasonCode"] = json!("SHARED_MEMORY_READ");
        mapping["revision"] = json!(8);
        mapping["offset"] = json!(0);
        mapping["limit"] = json!(10);
        mapping["total"] = json!(1);
        mapping["decisions"] = json!([{"decisionId":"decision:published","revision":3,"title":"Original published title","publishedAt":"2026-10-15T01:01:00.000Z"}]);
        data["sharedMemoryPage"] = mapping;
        let mut published = publication_preview(&data["decisionDetail"]).unwrap();
        published["decisionId"] = json!("decision:published");
        published["revision"] = json!(3);
        published["title"] = json!("Original published title");
        published["decision"] = json!("Published owner text stays original.");
        data["sharedDecisionDetail"] = json!({"schemaVersion":1,"reasonCode":"SHARED_DECISION_READ","projectId":"project:fixture","revision":8,"sharedProjectId":SHARED_ID,"manifestRevision":4,"current":true,"publishedAt":"2026-10-15T01:01:00.000Z","verifiedAt":"2026-10-15T01:01:00.000Z","stale":false,"offline":false,"decision":published});
        data
    }
    fn publication_console() -> Console {
        let mut console = Console {
            tab: Tab::Decisions,
            ..Default::default()
        };
        console.decisions.select("project:fixture");
        console.decisions.selected = "decision:fixture".into();
        console.shared.preview_open = true;
        console
    }

    #[test]
    fn publication_preview_is_an_allowlist_with_exact_acceptance_proof() {
        let mut data = shared_snapshot("publisher");
        let detail = &mut data["decisionDetail"];
        detail["decision"]["privatePath"] = json!("FORBIDDEN_PATH");
        detail["decision"]["extraction"] =
            json!({"evidence":[{"excerpt":"FORBIDDEN_GENERATOR_QUOTE"}]});
        detail["decision"]["actorId"] = json!("FORBIDDEN_ACTOR");
        detail["decision"]["sources"][0]["origin"] = json!({"eventId":"FORBIDDEN_ORIGIN"});
        detail["decision"]["sources"][0]["text"] = json!("FORBIDDEN_SOURCE_BODY");
        detail["sources"][0]["text"] = json!("FORBIDDEN_FROZEN_BODY");
        let preview = publication_preview(detail).unwrap();
        assert_eq!(preview.as_object().unwrap().len(), 11);
        assert_eq!(preview["acceptedAt"], "2026-10-15T01:00:00.000Z");
        assert_eq!(
            preview["sources"],
            json!([{"sourceId":"source:fixture","revision":3}])
        );
        assert!(!preview.to_string().contains("FORBIDDEN"));
        detail["history"][1]["revision"] = json!(1);
        assert!(publication_preview(detail).is_none());
        detail["history"][1]["revision"] = json!(2);
        detail["decision"]["status"] = json!("superseded");
        assert!(publication_preview(detail).is_none());
    }

    #[test]
    fn publication_requires_a_real_consent_click_and_queues_only_current_ids() {
        let ctx = test_context();
        let data = shared_snapshot("publisher");
        let mut console = publication_console();
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_publish",
            "Queue explicit publication"
        )
        .is_empty());
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "publication-consent",
            "Approve upload"
        )
        .is_empty());
        assert!(console.shared.consent);
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_publish",
            "Queue explicit publication",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "memory_cloud_publish");
        let payload = normalized_decision_command(&commands[0]);
        assert_eq!(
            payload,
            json!({"projectId":"project:fixture","decisionId":"decision:fixture","expectedDecisionRevision":2,"expectedSharedRevision":4,"consent":true})
        );
        assert!(!console.shared.consent);
        assert!(render(&ctx, &mut console, &data, false, vec![])
            .0
            .is_empty());
    }

    #[test]
    fn long_publication_preview_keeps_consent_and_action_outside_its_scroll_body() {
        for language in [Language::Tr, Language::En] {
            let ctx = test_context();
            let mut data = shared_snapshot("publisher");
            data["decisionDetail"]["decision"]["decision"] =
                json!("Long owner content. ".repeat(900));
            let mut console = publication_console();
            let mut commands = Vec::new();
            let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 560.0));
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("console-language"), language));
                    console.publication_controls(ui, &data, &data["decisionDetail"], &mut commands);
                },
            );
            output.drop_without_applying_deltas();
            let consent = ctx.data(|d| {
                d.get_temp::<Rect>(egui::Id::new((
                    "test-action",
                    "publication-consent",
                    "Approve upload",
                )))
                .unwrap()
            });
            let (_, button, clip) = ctx.data(|d| {
                d.get_temp::<(bool, Rect, Rect)>(egui::Id::new((
                    "test-action-state",
                    "memory_cloud_publish",
                    "Queue explicit publication",
                )))
                .unwrap()
            });
            assert!(screen.contains_rect(consent));
            assert!(screen.contains_rect(button));
            assert!(clip.contains_rect(button));
            assert!(button.min.y >= consent.max.y);
            assert!(commands.is_empty());
        }
    }

    #[test]
    fn publication_consent_does_not_survive_scope_revision_or_connection_changes() {
        for change in [
            "sharedRevision",
            "decisionRevision",
            "host",
            "reader",
            "pending",
            "disabled",
            "offline",
        ] {
            let ctx = test_context();
            let mut data = shared_snapshot("publisher");
            let mut console = publication_console();
            render(&ctx, &mut console, &data, false, vec![]);
            click(
                &ctx,
                &mut console,
                &data,
                false,
                "publication-consent",
                "Approve upload",
            );
            assert!(console.shared.consent);
            match change {
                "sharedRevision" => {
                    data["sharedMemory"]["projects"][0]["sharedRevision"] = json!(5)
                }
                "decisionRevision" => {
                    data["decisionDetail"]["decision"]["revision"] = json!(3);
                    data["decisionDetail"]["history"][1]["revision"] = json!(3);
                }
                "host" => data["cloud"]["hostId"] = json!("host:changed"),
                "reader" => data["sharedMemory"]["projects"][0]["role"] = json!("reader"),
                "pending" => {
                    data["sharedMemory"]["projects"][0]["pendingOperation"] =
                        json!({"operationId":"memory-op:existing"})
                }
                "disabled" => data["sharedMemory"]["projects"][0]["enabled"] = json!(false),
                _ => data["cloud"]["paired"] = json!(false),
            }
            assert!(render(&ctx, &mut console, &data, false, vec![])
                .0
                .is_empty());
            assert!(!console.shared.consent, "consent must reset on {change}");
        }
        let ctx = test_context();
        let data = shared_snapshot("publisher");
        let mut console = publication_console();
        render(&ctx, &mut console, &data, false, vec![]);
        click(
            &ctx,
            &mut console,
            &data,
            false,
            "publication-consent",
            "Approve upload",
        );
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "memory_cloud_publish",
            "Queue explicit publication"
        )
        .is_empty());
    }

    #[test]
    fn shared_reader_is_scoped_read_only_and_marks_historical_offline_data() {
        for lang in ["tr", "en"] {
            let ctx = test_context();
            let mut data = shared_snapshot("reader");
            data["desktopPreferences"] = json!({"language":lang});
            data["sharedMemory"]["projects"][0]["offline"] = json!(true);
            data["sharedDecisionDetail"]["current"] = json!(false);
            let mut console = Console {
                tab: Tab::Cloud,
                ..Default::default()
            };
            console.shared.select("project:fixture");
            let commands = render(&ctx, &mut console, &data, false, vec![]).0;
            assert!(commands.is_empty());
            let copy = painted_copy(&ctx, &mut console, &data).join("\n");
            assert!(copy.contains(if lang == "tr" {
                "Çevrimdışı önbellek"
            } else {
                "Offline cache"
            }));
            assert!(copy.contains("Original published title"));
            assert!(copy.contains(if lang == "tr" {
                "Geçmiş paylaşılan sürüm"
            } else {
                "Historical shared version"
            }));
            assert!(copy.contains("Published owner text stays original."));
            assert!(!copy.contains(if lang == "tr" {
                "Paylaşımı kapat"
            } else {
                "Disable sharing"
            }));
            data["sharedDecisionDetail"]["projectId"] = json!("project:foreign");
            let copy = painted_copy(&ctx, &mut console, &data).join("\n");
            assert!(!copy.contains("Published owner text stays original."));
            data["sharedDecisionDetail"]["projectId"] = json!("project:fixture");
            data["sharedDecisionDetail"]["sharedProjectId"] = json!("shared:foreign");
            assert!(!painted_copy(&ctx, &mut console, &data)
                .join("\n")
                .contains("Published owner text stays original."));
        }
    }

    #[test]
    fn unlinked_local_projects_do_not_claim_an_offline_or_stale_cloud_cache() {
        for language in ["tr", "en"] {
            for offline in [true, false] {
                let ctx = test_context();
                let mut data = shared_snapshot("reader");
                data["desktopPreferences"] = json!({"language":language});
                let metadata = &mut data["sharedMemory"]["projects"][0];
                metadata["sharedProjectId"] = Value::Null;
                metadata["role"] = Value::Null;
                metadata["state"] = json!("unlinked");
                metadata["offline"] = json!(offline);
                metadata["stale"] = json!(true);
                data["sharedMemoryPage"] = Value::Null;
                data["sharedDecisionDetail"] = Value::Null;
                let mut console = Console {
                    tab: Tab::Cloud,
                    ..Default::default()
                };
                let copy = painted_copy(&ctx, &mut console, &data).join("\n");
                assert!(copy.contains(if language == "tr" {
                    "Özel yerel proje"
                } else {
                    "Private Local project"
                }));
                assert!(copy.contains(if language == "tr" {
                    "Bağlı değil"
                } else {
                    "Not linked"
                }));
                assert!(!copy.contains(if language == "tr" {
                    "Çevrimdışı önbellek"
                } else {
                    "Offline cache"
                }));
                assert!(!copy.contains(if language == "tr" {
                    "Önbellekteki veri eski olabilir"
                } else {
                    "Cached data may be stale"
                }));
            }
        }
    }

    #[test]
    fn cached_shared_reads_are_bounded_and_failed_reads_need_explicit_refresh() {
        let data = shared_snapshot("reader");
        let metadata = shared_project(&data, "project:fixture");
        let mut panel = SharedPanel::default();
        panel.select("project:fixture");
        assert!(panel
            .next_read(&Value::Null, &Value::Null, metadata, 8, true)
            .is_none());
        assert!(panel.page_requested.is_none());
        let read = panel
            .next_read(&Value::Null, &Value::Null, metadata, 8, false)
            .unwrap();
        assert_eq!(read.action, "memory_cloud_read");
        assert_eq!(read.payload["limit"], 10);
        assert_eq!(read.payload["offset"], 0);
        assert!(panel
            .next_read(&Value::Null, &Value::Null, metadata, 8, false)
            .is_none());
        panel.refresh_page = true;
        assert!(panel
            .next_read(&Value::Null, &Value::Null, metadata, 8, true)
            .is_none());
        assert!(panel.refresh_page);
        assert!(panel
            .next_read(&Value::Null, &Value::Null, metadata, 8, false)
            .is_some());
        panel.selected = "decision:published".into();
        panel.selected_revision = 3;
        let exact = panel
            .next_read(&data["sharedMemoryPage"], &Value::Null, metadata, 8, false)
            .unwrap();
        assert_eq!(exact.action, "memory_cloud_decision_read");
        assert_eq!(exact.payload["revision"], 3);
        assert!(panel
            .next_read(&data["sharedMemoryPage"], &Value::Null, metadata, 8, false)
            .is_none());
        assert!(panel
            .next_read(
                &data["sharedMemoryPage"],
                &data["sharedDecisionDetail"],
                metadata,
                9,
                false
            )
            .is_some());
    }

    #[test]
    fn failed_owner_mutation_blocks_shared_read_controls_without_consuming_keys() {
        let ctx = test_context();
        let mut data = shared_snapshot("reader");
        data["sharedMemoryPage"] = Value::Null;
        data["sharedDecisionDetail"] = Value::Null;
        let mut console = Console {
            tab: Tab::Cloud,
            ..Default::default()
        };
        console.shared.select("project:fixture");
        let notice = Notice::error("memory_cloud_publish", "HOST_ACTION_FAILED");
        let mut commands = vec![];
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 1200.0))),
                ..Default::default()
            },
            |ui| commands = console.show(ui, &data, false, Some(&notice), true),
        );
        output.drop_without_applying_deltas();
        assert!(commands.is_empty());
        assert!(console.shared.page_requested.is_none());
        assert!(console.shared.detail_requested.is_none());
        assert!(console.hold_automatic_reads);
        let commands = render(&ctx, &mut console, &data, false, vec![]).0;
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "memory_cloud_read");
    }

    #[test]
    fn shared_setup_and_publisher_controls_keep_roles_and_expected_revisions() {
        let ctx = test_context();
        let mut data = shared_snapshot("publisher");
        let mut console = Console {
            tab: Tab::Cloud,
            ..Default::default()
        };
        console.shared.select("project:fixture");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_sharing",
            "Disable sharing",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"projectId":"project:fixture","expectedSharedRevision":4,"enabled":false})
        );
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_sync",
            "Sync shared memory",
        );
        assert_eq!(commands[0].payload["projectId"], "project:fixture");
        data["sharedMemory"]["projects"][0]["sharedProjectId"] = Value::Null;
        data["sharedMemory"]["projects"][0]["role"] = Value::Null;
        data["sharedMemory"]["projects"][0]["state"] = json!("unlinked");
        console.shared.name = "  Shared Billing  ".into();
        console.shared.link_id = SHARED_ID.into();
        click(&ctx, &mut console, &data, false, "shared-header", "Create");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_create",
            "Queue shared project creation",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"projectId":"project:fixture","expectedProjectRevision":9,"name":"Shared Billing"})
        );
        click(&ctx, &mut console, &data, false, "shared-header", "Link");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_link",
            "Queue reader link",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            normalized_decision_command(&commands[0]),
            json!({"projectId":"project:fixture","expectedProjectRevision":9,"sharedProjectId":SHARED_ID})
        );
        data["sharedMemory"]["projects"][0]["pendingOperation"] =
            json!({"operationId":"memory-op:queued"});
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_create",
            "Queue shared project creation"
        )
        .is_empty());
        data["sharedMemory"]["projects"][0]["pendingOperation"] = Value::Null;
        console.shared.link_id = "shared:../invalid".into();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_link",
            "Queue reader link"
        )
        .is_empty());
        console.shared.link_id = SHARED_ID.into();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "memory_cloud_link",
            "Queue reader link"
        )
        .is_empty());
        data["cloud"]["paired"] = json!(false);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_cloud_link",
            "Queue reader link"
        )
        .is_empty());
    }

    #[test]
    fn shared_control_rows_fit_the_minimum_window_and_ids_are_validated() {
        assert!(opaque_id(SHARED_ID, "shared"));
        for bad in [
            "shared:../other",
            "shared:60C029A4-BC68-44C8-87C9-08D0C2688E9B",
            "decision:60c029a4-bc68-44c8-87c9-08d0c2688e9b",
            "shared:",
        ] {
            assert!(!opaque_id(bad, "shared"));
        }
        for lang in ["tr", "en"] {
            let ctx = test_context();
            let mut data = shared_snapshot("publisher");
            data["desktopPreferences"] = json!({"language":lang});
            let mut console = Console {
                tab: Tab::Cloud,
                ..Default::default()
            };
            let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 560.0));
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    console.show(ui, &data, false, None, false);
                },
            );
            output.drop_without_applying_deltas();
            for (action, label) in [
                ("memory_cloud_sync", "Sync shared memory"),
                ("memory_cloud_sharing", "Disable sharing"),
            ] {
                let (_, rect, clip) = ctx.data(|d| {
                    d.get_temp::<(bool, Rect, Rect)>(egui::Id::new((
                        "test-action-state",
                        action,
                        label,
                    )))
                    .unwrap()
                });
                assert!(
                    rect.min.x >= clip.min.x && rect.max.x <= clip.max.x,
                    "{lang}: {label} cannot overflow horizontally: {rect:?} vs {clip:?}"
                );
                assert!(rect.max.x <= screen.max.x);
            }
        }
    }

    fn decision_snapshot(status: &str) -> Value {
        let mut data = memory_snapshot();
        data["projectMemory"]["projects"][0]["sourceCount"] = json!(1);
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
        let ctx = test_context();
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
        assert_eq!(
            &console.decisions.pending_proposal.as_ref().unwrap().1,
            &console
                .decisions
                .proposal(&data["projectMemory"]["projects"][0])
                .unwrap()
        );
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let mut data = decision_snapshot("proposed");
        panel.citation_epoch = 9;
        panel.citations.push(Citation {
            source: "source:fixture".into(),
            revision: 3,
            title: "Evidence".into(),
        });
        panel.pending_proposal = Some((
            6,
            panel
                .proposal(&data["projectMemory"]["projects"][0])
                .unwrap(),
        ));
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
    fn original_proposal_retry_preserves_edits_and_citations_after_cache_write_failure() {
        let mut data = decision_snapshot("proposed");
        let mut panel = DecisionPanel::default();
        panel.select("project:fixture");
        panel.composer_open = true;
        panel.title = "Original title".into();
        panel.decision = "Original content".into();
        panel.citation_epoch = 9;
        panel.citations.push(Citation {
            source: "source:fixture".into(),
            revision: 3,
            title: "Original evidence".into(),
        });
        let submitted = panel
            .proposal(&data["projectMemory"]["projects"][0])
            .unwrap();
        panel.pending_proposal = Some((6, submitted));
        data["projectDecisions"]["projects"][0]["revision"] = json!(7);
        panel.finish_proposal(
            &data,
            Some(&Notice::error(
                "project_decision_propose",
                "HOST_ACTION_FAILED",
            )),
            false,
        );
        assert_eq!(panel.decision, "Original content");
        assert!(panel.composer_open);
        panel.title = "New title".into();
        panel.context = "New context".into();
        panel.decision = "New content".into();
        panel.rationale = "New rationale".into();
        panel.consequences = "New consequences".into();
        panel.citations = vec![Citation {
            source: "source:new".into(),
            revision: 4,
            title: "New evidence".into(),
        }];
        let updated = panel
            .proposal(&data["projectMemory"]["projects"][0])
            .unwrap();
        let success = Notice::receipt(
            "project_decision_propose",
            &json!({"reasonCode":"PROJECT_DECISION_PROPOSED","outcome":"applied"}),
        );
        panel.finish_proposal(&data, Some(&success), false);
        assert_eq!(
            panel
                .proposal(&data["projectMemory"]["projects"][0])
                .unwrap(),
            updated
        );
        assert!(panel.composer_open);
        assert!(panel.pending_proposal.is_none());
        panel.finish_proposal(&data, Some(&success), false);
        assert_eq!(panel.title, "New title");
        assert_eq!(panel.citations[0].source, "source:new");
    }

    #[test]
    fn proposal_completion_never_clears_a_changed_project_or_source_epoch() {
        let mut data = decision_snapshot("proposed");
        let mut panel = DecisionPanel::default();
        panel.select("project:fixture");
        panel.title = "Draft title".into();
        panel.decision = "Keep this text".into();
        panel.composer_open = true;
        panel.citation_epoch = 9;
        panel.citations.push(Citation {
            source: "source:fixture".into(),
            revision: 3,
            title: "Evidence".into(),
        });
        let submitted = panel
            .proposal(&data["projectMemory"]["projects"][0])
            .unwrap();
        panel.pending_proposal = Some((6, submitted));
        data["projectDecisions"]["projects"][0]["revision"] = json!(7);
        data["projectMemory"]["projects"][0]["revision"] = json!(10);
        let success = Notice::receipt(
            "project_decision_propose",
            &json!({"reasonCode":"PROJECT_DECISION_PROPOSED","outcome":"applied"}),
        );
        panel.finish_proposal(&data, Some(&success), false);
        assert_eq!(panel.decision, "Keep this text");
        assert_eq!(panel.citations.len(), 1);
        assert!(panel.pending_proposal.is_none());
        panel.select("project:other");
        panel.title = "Other project draft".into();
        panel.decision = "Other project content".into();
        panel.composer_open = true;
        panel.finish_proposal(&data, Some(&success), false);
        assert_eq!(panel.title, "Other project draft");
        assert_eq!(panel.decision, "Other project content");
        assert!(panel.composer_open);
    }

    #[test]
    fn recoverable_mutation_error_holds_all_automatic_reads_without_consuming_request_keys() {
        fn show(
            ctx: &egui::Context,
            console: &mut Console,
            data: &Value,
            notice: Option<&Notice>,
            can_retry: bool,
            events: Vec<Event>,
        ) -> Vec<Command> {
            let mut commands = vec![];
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 1200.0))),
                    events,
                    ..Default::default()
                },
                |ui| commands = console.show(ui, data, false, notice, can_retry),
            );
            output.drop_without_applying_deltas();
            commands
        }
        let error = Notice::error("project_decision_propose", "HOST_ACTION_FAILED");
        for tab in [Tab::Memory, Tab::Decisions] {
            let ctx = test_context();
            let mut console = Console {
                tab,
                ..Default::default()
            };
            let mut data = decision_snapshot("proposed");
            console.memory.select("project:fixture");
            console.decisions.select("project:fixture");
            console.decisions.selected = "decision:fixture".into();
            console.decisions.composer_open = true;
            console.decisions.title = "Editable after error".into();
            console.decisions.decision = "Keep my pending draft".into();
            data["projectDecisions"]["projects"][0]["revision"] = json!(7);
            data["projectMemory"]["projects"][0]["revision"] = json!(10);
            assert!(show(&ctx, &mut console, &data, Some(&error), true, vec![]).is_empty());
            assert!(console.memory.requested.is_none());
            assert!(console.decisions.list_requested.is_none());
            assert!(console.decisions.detail_requested.is_none());
            assert!(console.decisions.sources.requested.is_none());
            assert_eq!(console.decisions.title, "Editable after error");
            assert_eq!(console.decisions.decision, "Keep my pending draft");
            let label = if tab == Tab::Memory {
                "Refresh memory"
            } else {
                "Refresh decisions"
            };
            let rect: Rect = ctx.data(|d| {
                d.get_temp(egui::Id::new(("test-action", "memory_control", label)))
                    .unwrap()
            });
            let pos = rect.center();
            for pressed in [true, false] {
                assert!(show(
                    &ctx,
                    &mut console,
                    &data,
                    Some(&error),
                    true,
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
                            button: PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::default()
                        }
                    ]
                )
                .is_empty());
            }
            let commands = show(&ctx, &mut console, &data, None, false, vec![]);
            assert_eq!(commands.len(), 1);
            assert_eq!(
                commands[0].action,
                if tab == Tab::Memory {
                    "project_memory_read"
                } else {
                    "project_decisions_read"
                }
            );
        }
        let ctx = test_context();
        let mut console = Console {
            tab: Tab::Memory,
            ..Default::default()
        };
        let mut data = decision_snapshot("proposed");
        data["memoryPage"] = Value::Null;
        let read_error = Notice::error("project_memory_read", "HOST_ACTION_FAILED");
        assert_eq!(
            show(&ctx, &mut console, &data, Some(&read_error), true, vec![])[0].action,
            "project_memory_read",
            "read-only failures do not lock mutation recovery"
        );
    }

    #[test]
    fn decision_localization_preserves_original_evidence_and_rejects_foreign_detail() {
        let ctx = test_context();
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
            let ctx = test_context();
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
            let account: Rect = ctx.data(|d| {
                d.get_temp(egui::Id::new((
                    "test-action",
                    "memory_control",
                    "Sign in · optional",
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
                        clipped.max.y <= footer.min.y.min(account.min.y),
                        "{} overlaps a sidebar footer control",
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
                accept.max.y <= 560.0,
                "acceptance must be discoverable before scrolling long owner text: {accept:?}"
            );
        }
    }

    #[test]
    fn agent_memory_copy_reports_loaded_stale_and_unsupported_in_both_languages() {
        for (language, loaded, stale, unsupported) in [
            (
                "tr",
                "Güncel onaylı karar listesi okundu.",
                "Kararlar değişti.",
                "Codex’i 0.155.1",
            ),
            (
                "en",
                "Current accepted decision list loaded.",
                "Decisions changed.",
                "Codex to 0.155.1",
            ),
        ] {
            let ctx = test_context();
            let mut console = Console {
                tab: Tab::Work,
                ..Default::default()
            };
            let mut data = snapshot();
            data["desktopPreferences"] = json!({"language":language});
            data["agentMemory"] = json!({"contexts":[{"runId":"run:ffffffff-ffff-4fff-8fff-ffffffffffff","workItemId":"work:fixture","decisionRevision":1,"currentDecisionRevision":3,"stale":true,"manifestLoaded":false,"issue":"AGENT_MEMORY_UNSUPPORTED"},{"runId":"run:00000000-0000-4000-8000-000000000000","workItemId":"work:fixture","decisionRevision":2,"currentDecisionRevision":3,"stale":true,"manifestLoaded":true,"issue":null}]});
            for _ in 0..3 {
                painted_copy(&ctx, &mut console, &data);
            }
            assert!(painted_copy(&ctx, &mut console, &data)
                .join("\n")
                .contains(loaded));
            assert!(!painted_copy(&ctx, &mut console, &data)
                .join("\n")
                .contains(unsupported));
            data["agentMemory"]["contexts"][1]["manifestLoaded"] = json!(false);
            assert!(painted_copy(&ctx, &mut console, &data)
                .join("\n")
                .contains(stale));
            data["agentMemory"]["contexts"][1]["issue"] = json!("AGENT_MEMORY_UNSUPPORTED");
            assert!(painted_copy(&ctx, &mut console, &data)
                .join("\n")
                .contains(unsupported));
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
        // Keep disclosure clicks deterministic while exercising the production fonts/theme.
        ctx.global_style_mut(|style| style.animation_time = 0.0);
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
        for (snapshot, theme) in [snapshot(), json!({})].into_iter().flat_map(|snapshot| {
            [egui::Theme::Light, egui::Theme::Dark]
                .into_iter()
                .map(move |theme| (snapshot.clone(), theme))
        }) {
            for tab in Tab::ALL {
                let ctx = test_context();
                ctx.set_theme(theme);
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
    fn first_project_action_is_visible_in_both_themes_and_languages_at_minimum_size() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            for code in ["tr", "en"] {
                let ctx = test_context();
                ctx.set_theme(theme);
                let mut console = Console::default();
                let data = json!({"desktopPreferences":{"language":code,"error":false}});
                let mut commands = vec![];
                let output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(800.0, 560.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        commands = console.show(ui, &data, false, None, false);
                    },
                );
                output.drop_without_applying_deltas();
                let create: Rect = ctx.data(|d| {
                    d.get_temp(egui::Id::new((
                        "test-action",
                        "project_create",
                        "Create project",
                    )))
                    .unwrap()
                });
                assert!(
                    create.max.x <= 800.0 && create.max.y <= 540.0,
                    "first-project action should be visible: {create:?}"
                );
                assert!(commands.is_empty());
            }
        }
    }

    #[test]
    fn owner_click_keeps_current_revision_and_busy_or_unproven_control_blocks_start() {
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
            let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let valid_ctx = test_context();
        click(
            &valid_ctx,
            &mut console,
            &snapshot,
            false,
            "memory_control",
            "Advanced connection settings",
        );
        assert!(valid_ctx.data(|d| d.get_temp::<Rect>(pair_id)).is_some());

        snapshot["cloud"] = json!({"paired":false,"error":true});
        let ctx = test_context();
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
    fn project_shortcuts_keep_scope_and_only_prepare_a_local_draft() {
        let ctx = test_context();
        let mut data = memory_snapshot();
        let mut console = Console::default();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "New task"
        )
        .is_empty());
        assert!(
            console.tab == Tab::Projects,
            "an unlinked project cannot select a task folder"
        );
        data["projectMemory"]["projects"][0]["workspaceIds"] = json!(["workspace:fixture"]);
        console.work.path = "/tmp/old-manual-selection".into();
        console.work.path_problem = Some("WORKSPACE_ACCESS_DENIED".into());
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "New task"
        )
        .is_empty());
        assert!(console.tab == Tab::Work);
        assert_eq!(console.work.workspace, "workspace:fixture");
        assert!(console.work.path.is_empty() && console.work.path_problem.is_none());
        console.tab = Tab::Projects;
        console.work.instruction = "Keep my unsent task".into();
        console.work.workspace = "workspace:other-draft".into();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Open task draft"
        )
        .is_empty());
        assert_eq!(console.work.instruction, "Keep my unsent task");
        assert_eq!(console.work.workspace, "workspace:other-draft");
        console.tab = Tab::Projects;
        console.decisions.select("project:old");
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Open decisions"
        )
        .is_empty());
        assert!(console.tab == Tab::Decisions);
        assert_eq!(console.decisions.project, "project:fixture");
        assert!(console.decisions.selected.is_empty());
    }

    #[test]
    fn stable_project_create_and_explicit_link_unlink_preserve_target_revision() {
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
                // Text-boundary assertions inspect the whole short fixture record.
                // Dedicated minimum-window tests separately check initial visibility.
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 1600.0))),
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
        let ctx = test_context();
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
        let ctx = test_context();
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
        let ctx = test_context();
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
