use crate::command::Command;
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
    Work,
    Teams,
    Agents,
    Providers,
    Cloud,
}
impl Tab {
    const ALL: [Self; 7] = [
        Self::Projects,
        Self::Memory,
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
            Self::Work => "Work",
            Self::Teams => "Teams",
            Self::Agents => "Agents",
            Self::Providers => "Providers",
            Self::Cloud => "Cloud",
        }
    }
    fn summary(self) -> &'static str {
        match self {
            Self::Projects => "Stable projects, linked worktrees and shared local context.",
            Self::Memory => "Project sources with provenance. ADR extraction comes next.",
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
pub struct Console {
    tab: Tab,
    new_project_name: String,
    link_choices: HashMap<String, String>,
    memory: MemoryDraft,
    project_name: String,
    project_path: String,
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

impl Console {
    #[cfg(feature = "ui-smoke")]
    pub fn memory_smoke(&mut self) {
        self.tab = Tab::Memory;
    }

    pub fn show(&mut self, ui: &mut Ui, snapshot: &Value, busy: bool) -> Vec<Command> {
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
                for tab in Tab::ALL {
                    let count = match tab {
                        Tab::Projects => array(&snapshot["projectMemory"]["projects"]).len(),
                        Tab::Memory => 0,
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
                        action(
                            ui,
                            &mut commands,
                            "Refresh providers",
                            "observe",
                            !busy,
                            || json!({}),
                        );
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
                            Tab::Work => self.work_panel(ui, snapshot, &mut commands),
                            Tab::Teams => self.teams(ui, snapshot, &mut commands),
                            Tab::Agents => self.agents(ui, snapshot, &mut commands),
                            Tab::Providers => self.providers(ui, snapshot),
                            Tab::Cloud => self.cloud(ui, snapshot, &mut commands),
                        });
                    });
            });
        commands
    }

    fn projects(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let projects = array(&snapshot["projectMemory"]["projects"]);
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
            ui.add_space(8.0);
            hint(
                ui,
                "Give your project a stable home for worktrees, tasks and local context.",
            );
            ui.add_space(24.0);
            constrained(ui, 560.0, |ui| self.create_project_form(ui, commands));
        } else {
            egui::CollapsingHeader::new(t(ui, "Create a project")).show(ui, |ui| {
                constrained(ui, 560.0, |ui| self.create_project_form(ui, commands))
            });
            ui.add_space(14.0);
            for project in projects {
                ui.push_id(text(project,"projectId"),|ui|card(ui,|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(text(project,"name")).size(16.0).strong().color(TEXT));
                    status(ui,&localized_format!(ui, "{0} sources",project["sourceCount"].as_u64().unwrap_or(0)));
                    if control(ui,"Open memory",true).clicked(){self.memory.select(text(project,"projectId"));self.tab=Tab::Memory;}
                });
                hint(ui,"Linked worktrees share this project's source context.");ui.add_space(10.0);
                for id in array(&project["workspaceIds"]) {
                    let workspace=items(&snapshot["workspaces"]).iter().find(|w|w["workspaceId"]==*id);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(workspace.map(|w|std::borrow::Cow::Borrowed(text(w,"displayName"))).unwrap_or_else(||t(ui,"Removed worktree")));
                        action(ui,commands,"Unlink","project_unlink_workspace",revision(project)>0,||json!({"projectId":project["projectId"],"workspaceId":id,"expectedProjectRevision":project["revision"]}));
                    });
                    if let Some(workspace)=workspace {hint(ui,text(workspace,"workingDirectory"));}
                }
                let available:Vec<_>=items(&snapshot["workspaces"]).iter().filter(|w|!projects.iter().any(|p|array(&p["workspaceIds"]).contains(&w["workspaceId"]))).collect();
                if available.is_empty(){hint(ui,"Register an unlinked worktree below to link it here.");}
                else {
                    let selected=self.link_choices.entry(text(project,"projectId").to_owned()).or_default();
                    choose_refs(ui,"link-worktree","Link a registered worktree",selected,&available,("workspaceId","workingDirectory"),true);
                    let workspace=available.iter().find(|w|text(w,"workspaceId")==selected);
                    if let Some(workspace)=workspace {hint(ui,text(workspace,"workingDirectory"));}
                    let exists=workspace.is_some();
                    action(ui,commands,"Link worktree","project_link_workspace",exists&&revision(project)>0,||json!({"projectId":project["projectId"],"workspaceId":selected,"expectedProjectRevision":project["revision"]}));
                }
                ui.collapsing(t(ui, "Project identity"),|ui|{ui.monospace(text(project,"projectId"));hint(ui,&localized_format!(ui, "Revision {0} · created {1}",revision(project),format_time(ui,text(project,"createdAt"))));});
            }));
                ui.add_space(12.0);
            }
        }
        ui.add_space(22.0);
        ui.separator();
        ui.add_space(18.0);
        ui.label(
            RichText::new(t(ui, "REGISTERED WORKTREES"))
                .size(10.0)
                .color(MUTED),
        );
        hint(ui,"Worktrees are execution folders. Unlinked worktrees remain usable for tasks and teams.");
        ui.add_space(10.0);
        for workspace in items(&snapshot["workspaces"]) {
            ui.push_id(text(workspace,"workspaceId"),|ui|card(ui,|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(text(workspace,"displayName")).size(13.0).strong().color(TEXT));
                let project=projects.iter().find(|p|array(&p["workspaceIds"]).contains(&workspace["workspaceId"]));
                if let Some(project)=project {raw_status(ui,text(project,"name"));} else {status(ui,"Unlinked");}
                action(ui,commands,"Unregister","workspace_remove",revision(workspace)>0,||json!({"workspaceId":workspace["workspaceId"],"expectedWorkspaceRevision":workspace["revision"]}));
            });
            hint(ui,text(workspace,"workingDirectory"));
        }));
            ui.add_space(8.0);
        }
        egui::CollapsingHeader::new(t(ui, "Register a worktree"))
            .default_open(items(&snapshot["workspaces"]).is_empty())
            .show(ui, |ui| {
                constrained(ui, 560.0, |ui| self.project_form(ui, commands))
            });
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
                    }
                    Err(error) => self.form_error = Some(error),
                }
            }
            hint(ui,"Link worktrees after creating the project. Existing folders are not grouped automatically.");
        });
    }

    fn project_form(&mut self, ui: &mut Ui, commands: &mut Vec<Command>) {
        card(ui, |ui| {
            section_title(
                ui,
                "Register worktree",
                "An existing Git worktree used to execute tasks.",
            );
            ui.add_space(16.0);
            field_hint(
                ui,
                "Worktree path",
                &mut self.project_path,
                4096,
                "/absolute/path/to/repository",
            );
            field_hint(
                ui,
                "Worktree label · optional",
                &mut self.project_name,
                160,
                "e.g. Website, API, Mobile app",
            );
            ui.add_space(8.0);
            if submit(ui, "Register worktree", "workspace_register") {
                if self.project_path.trim().is_empty() {
                    self.form_error = Some("Enter the absolute path of an existing Git worktree.");
                } else if utf16_len(self.project_path.trim()) > 4096 {
                    self.form_error = Some("The worktree path is too long.");
                } else if utf16_len(self.project_name.trim()) > 160 {
                    self.form_error = Some("The worktree label is limited to 160 characters.");
                } else {
                    commands.push(Command::new("workspace_register",json!({"workingDirectory":self.project_path.trim(),"displayName":self.project_name.trim()})));
                    self.form_error = None;
                }
            }
            ui.add_space(10.0);
            hint(ui, "Files stay where they are. Nothing is uploaded.");
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
            choose_workspace(ui, "work-project", &mut self.work.workspace, snapshot);
            ui.collapsing(t(ui, "Use another Git worktree once"), |ui| {
                field(
                    ui,
                    "Absolute worktree path",
                    &mut self.work.path,
                    4096,
                    false,
                );
                hint(ui, "Leave this blank when selecting a registered project.");
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
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("test-action", "memory_control", label)),
            response.rect,
        )
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
            commands.extend(console.show(ui, snapshot, busy))
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
            ..Default::default()
        };
        let mut fresh = snapshot();
        fresh["workspaces"]["items"] = json!([]);
        let commands = click(
            &ctx,
            &mut console,
            &fresh,
            false,
            "workspace_register",
            "Register worktree",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "workspace_register");
        assert_eq!(
            commands[0].payload["workingDirectory"],
            "/tmp/native-fixture"
        );
    }

    #[test]
    fn draft_validation_rejects_ambiguous_project_duplicate_roles_and_bad_limits() {
        let snapshot = snapshot();
        let mut work = WorkDraft {
            title: "Task".into(),
            instruction: "One outcome".into(),
            workspace: "workspace:fixture".into(),
            path: "/tmp/another".into(),
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
            "Link worktree",
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
                assert!(console.show(ui, &data, false).is_empty());
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
                console.show(ui, data, false);
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
