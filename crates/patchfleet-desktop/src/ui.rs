use crate::command::Command;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::{json, Value};
use std::collections::HashMap;

const MINT: Color32 = Color32::from_rgb(139, 224, 188);
const MUTED: Color32 = Color32::from_rgb(134, 145, 159);
const SURFACE: Color32 = Color32::from_rgb(22, 26, 32);
const TEXT: Color32 = Color32::from_rgb(232, 235, 239);
const BORDER: Color32 = Color32::from_rgb(43, 48, 57);
const RED: Color32 = Color32::from_rgb(246, 142, 151);

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Projects,
    Work,
    Teams,
    Agents,
    Providers,
    Cloud,
}
impl Tab {
    const ALL: [Self; 6] = [
        Self::Projects,
        Self::Work,
        Self::Teams,
        Self::Agents,
        Self::Providers,
        Self::Cloud,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Projects => "Projects",
            Self::Work => "Work",
            Self::Teams => "Teams",
            Self::Agents => "Agents",
            Self::Providers => "Providers",
            Self::Cloud => "Cloud",
        }
    }
    fn summary(self) -> &'static str {
        match self {
            Self::Projects => {
                "Register a local Git worktree once, then use it for tasks and teams."
            }
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
            return Err("Choose a registered project or one absolute worktree path.");
        }
        if !selected.is_empty()
            && !items(&snapshot["workspaces"])
                .iter()
                .any(|w| text(w, "workspaceId") == selected)
        {
            return Err("That project is no longer registered. Choose a current project.");
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
            return Err("Choose a registered project.");
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

#[derive(Default)]
pub struct Console {
    tab: Tab,
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
    pub fn show(&mut self, ui: &mut Ui, snapshot: &Value, busy: bool) -> Vec<Command> {
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
                    ui.label(RichText::new("Patchfleet").size(15.0).strong().color(TEXT));
                });
                ui.add_space(26.0);
                ui.label(RichText::new("WORKSPACE").size(10.0).color(MUTED));
                ui.add_space(8.0);
                for tab in Tab::ALL {
                    let count = match tab {
                        Tab::Projects => items(&snapshot["workspaces"]).len(),
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
                            ui.colored_label(MINT, "•");
                            ui.label(RichText::new("Local engine").size(11.0).color(TEXT));
                        });
                        ui.add_space(7.0);
                        ui.separator();
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
                        RichText::new(self.tab.name())
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
                            ui.label("Applying action…");
                        }
                    });
                });
                ui.add_space(3.0);
                ui.label(RichText::new(self.tab.summary()).size(12.0).color(MUTED));
                ui.add_space(24.0);
                if let Some(error) = self.form_error {
                    ui.colored_label(RED, error);
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .id_salt(("console-body", self.tab.name()))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width().min(1040.0));
                        ui.add_enabled_ui(!busy, |ui| match self.tab {
                            Tab::Projects => self.projects(ui, snapshot, &mut commands),
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
        let workspaces = items(&snapshot["workspaces"]);
        if workspaces.is_empty() {
            ui.label(RichText::new("YOUR LOCAL WORKSPACE").size(10.0).color(MINT));
            ui.add_space(8.0);
            ui.label(
                RichText::new("Add your first project")
                    .size(28.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Connect a repository. Give your agents a place to work.")
                    .size(13.0)
                    .color(MUTED),
            );
            ui.add_space(28.0);
            if ui.available_width() >= 720.0 {
                ui.columns(2, |columns| {
                    self.project_form(&mut columns[0], commands);
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: 16,
                            right: 0,
                            top: 8,
                            bottom: 0,
                        })
                        .show(&mut columns[1], onboarding);
                });
            } else {
                self.project_form(ui, commands);
                ui.add_space(20.0);
                onboarding(ui);
            }
            ui.add_space(26.0);
            ui.separator();
            ui.add_space(18.0);
            ui.label(RichText::new("YOUR PROVIDERS").size(10.0).color(MUTED));
            ui.add_space(10.0);
            ui.columns(3, |columns| {
                for (index, (id, name)) in [
                    ("codex", "Codex"),
                    ("claude", "Claude Code"),
                    ("gemini", "Gemini CLI"),
                ]
                .into_iter()
                .enumerate()
                {
                    card(&mut columns[index], |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                            draw_icon(ui.painter(), rect, Tab::Providers, MUTED);
                            ui.label(RichText::new(name).size(12.0).strong().color(TEXT));
                        });
                        ui.add_space(9.0);
                        let observation = array(&snapshot["observation"]["observations"])
                            .iter()
                            .find(|o| text(&o["provider"], "id") == id);
                        status(
                            ui,
                            observation
                                .map(|o| text(&o["provider"], "state"))
                                .unwrap_or("Not checked"),
                        );
                    });
                }
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(RichText::new("REGISTERED PROJECTS").size(10.0).color(MUTED));
                ui.label(
                    RichText::new(workspaces.len().to_string())
                        .size(11.0)
                        .color(TEXT),
                );
            });
            ui.add_space(10.0);
            for workspace in workspaces {
                ui.push_id(text(workspace,"workspaceId"),|ui|card(ui,|ui| {
                ui.horizontal(|ui| {
                    let (r,_) = ui.allocate_exact_size(egui::vec2(28.0,28.0),egui::Sense::hover());draw_icon(ui.painter(),r,Tab::Projects,MINT);
                    ui.vertical(|ui| {ui.label(RichText::new(text(workspace,"displayName")).size(14.0).strong().color(TEXT));hint(ui,text(workspace,"workingDirectory"));});
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{action(ui,commands,"Unregister","workspace_remove",revision(workspace)>0,||json!({"workspaceId":workspace["workspaceId"],"expectedWorkspaceRevision":workspace["revision"]}));});
                });
            }));
                ui.add_space(8.0);
            }
            ui.add_space(16.0);
            egui::CollapsingHeader::new("Add another project").show(ui, |ui| {
                constrained(ui, 520.0, |ui| self.project_form(ui, commands))
            });
        }
    }

    fn project_form(&mut self, ui: &mut Ui, commands: &mut Vec<Command>) {
        card(ui, |ui| {
            section_title(
                ui,
                "Repository",
                "An existing Git repository on this device.",
            );
            ui.add_space(16.0);
            field_hint(
                ui,
                "Repository path",
                &mut self.project_path,
                4096,
                "/absolute/path/to/repository",
            );
            field_hint(
                ui,
                "Project name · optional",
                &mut self.project_name,
                160,
                "e.g. Website, API, Mobile app",
            );
            ui.add_space(8.0);
            if submit(ui, "Add project", "workspace_register") {
                if self.project_path.trim().is_empty() {
                    self.form_error = Some("Enter the absolute path of an existing Git worktree.");
                } else if utf16_len(self.project_path.trim()) > 4096 {
                    self.form_error = Some("The project path is too long.");
                } else {
                    commands.push(Command::new("workspace_register",json!({"workingDirectory":self.project_path.trim(),"displayName":self.project_name.trim()})));
                    self.form_error = None;
                }
            }
            ui.add_space(10.0);
            hint(ui, "Files stay where they are. Nothing is uploaded.");
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
            ui.label(RichText::new("TASK QUEUE").size(10.0).color(MUTED));
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
                    .add_enabled(self.work_page > 0, egui::Button::new("Previous"))
                    .clicked()
                {
                    self.work_page -= 1;
                }
                ui.label(format!(
                    "Page {} of {} · {} tasks",
                    self.work_page + 1,
                    page_count,
                    list.len()
                ));
                if ui
                    .add_enabled(self.work_page + 1 < page_count, egui::Button::new("Next"))
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
                hint(ui,workspace.map(|w|text(w,"displayName")).unwrap_or("Unregistered local project"));
                let run = array(&work["runs"]).iter().rev().find(|r|r["workItemId"]==item["workItemId"]);
                ui.collapsing("Local task details",|ui| {ui.monospace(text(item,"workingDirectory"));ui.label(text(item,"instruction"));if let Some(run)=run {ui.monospace(format!("Run {} · turn {}",text(run,"runId"),text(run,"providerTurnId")));}});
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
        ui.collapsing("Recent durable activity", |ui| {
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
                    ui.label(text(r, "reasonCode").replace('_', " "));
                    hint(ui, &format_time(text(r, "completedAt")));
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
            ui.collapsing("Use another Git worktree once", |ui| {
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
        egui::CollapsingHeader::new("Form a local team")
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
                            &format!(
                                "Agent {}{}",
                                slot + 1,
                                if slot == 0 { " (required)" } else { "" }
                            ),
                            &mut self.team.workers[slot],
                            &workers,
                            ("id", "name"),
                            true,
                        );
                    }
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Concurrency");
                        ui.add(egui::DragValue::new(&mut self.team.concurrency).range(1..=4));
                        ui.label("Retries");
                        ui.add(egui::DragValue::new(&mut self.team.retries).range(0..=3));
                        ui.label("Minutes");
                        ui.add(egui::DragValue::new(&mut self.team.minutes).range(5..=480));
                    });
                    ui.checkbox(
                        &mut self.team.continue_independent,
                        "Continue independent tasks after a failure",
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
            ui.label(text(team,"goal"));hint(ui,&format!("{} · concurrency {} · {} retries",text(team,"templateId").replace('-'," "),team["settings"]["concurrency"],team["settings"]["retryLimit"]));
            ui.add_space(10.0);
            let active=matches!(text(team,"status"),"active"|"waiting");
            ui.horizontal_wrapped(|ui| {
                if text(team,"status")=="draft" {action(ui,commands,"Start team","team_start",codex_ready(snapshot)&&revision(team)>0,||team_target(team));}
                if active {action(ui,commands,"Run ready work","team_advance",revision(team)>0,||team_target(team));action(ui,commands,"Cancel team","team_cancel",revision(team)>0,||team_target(team));}
            });
            ui.add_space(10.0);
            for task in array(&team["tasks"]) {ui.push_id(text(task,"taskId"),|ui| {
                ui.separator();ui.horizontal_wrapped(|ui|{ui.strong(text(task,"title"));status(ui,text(task,"status"));});
                let agent=array(&team["agents"]).iter().find(|a|a["agentId"]==task["agentId"]);
                hint(ui,&format!("{} · attempt {}/{}",agent.map(|a|text(&a["pack"],"name")).unwrap_or("Unassigned"),task["attempt"],task["maxAttempts"]));
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
            ui.collapsing("Agents and controls",|ui|for agent in array(&team["agents"]) {ui.push_id(text(agent,"agentId"),|ui| {
                ui.horizontal_wrapped(|ui| {ui.strong(text(&agent["pack"],"name"));status(ui,text(agent,"status"));
                    if active&&agent["agentId"]!=team["orchestratorAgentId"]&&!matches!(text(agent,"status"),"completed"|"cancelled") {action(ui,commands,"Cancel agent","team_cancel_agent",revision(team)>0,||json!({"teamId":team["teamId"],"expectedTeamRevision":team["revision"],"agentId":agent["agentId"]}));}
                });
            });});
        }));
            ui.add_space(14.0);
        }
    }

    fn agents(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let packs = array(&snapshot["packs"]);
        hint(ui, &format!("{} inspectable local packs", packs.len()));
        ui.add_space(12.0);
        for pack in packs {
            ui.push_id(text(pack, "id"), |ui| {
                card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(text(pack, "name"));
                        status(ui, text(pack, "role"));
                        hint(ui, text(&pack["provenance"], "kind"));
                    });
                    ui.label(text(pack, "description"));
                    egui::CollapsingHeader::new("Inspect pack contract").show(ui, |ui| {
                        ui.label(format!(
                            "Provider: {} · {} minutes · {} attempts",
                            text(pack, "providerId"),
                            pack["limits"]["timeoutMinutes"],
                            pack["limits"]["maxAttempts"]
                        ));
                        ui.label(format!("Permissions: {}", joined(&pack["permissions"])));
                        ui.label(format!(
                            "Quality checks: {}",
                            joined(&pack["qualityChecks"])
                        ));
                        ui.label(text(pack, "instructions"));
                        ui.strong("Expected output");
                        ui.label(text(pack, "expectedOutput"));
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
            ui.heading("Install a custom pack");
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
                        &format!(
                            "CLI version: {}",
                            provider["version"].as_str().unwrap_or("Not available")
                        ),
                    );
                    ui.label(format!(
                        "Recent observation: {} · explicit live status: {}",
                        if provider["capabilities"]["recentObservation"] == true {
                            "supported"
                        } else {
                            "unavailable"
                        },
                        if provider["capabilities"]["explicitLiveStatus"] == true {
                            "supported"
                        } else {
                            "unavailable"
                        }
                    ));
                    if !provider["error"].is_null() {
                        ui.colored_label(RED, text(&provider["error"], "message"));
                        ui.monospace(text(&provider["error"], "code"));
                    }
                    hint(
                        ui,
                        &format!(
                            "Stored snapshot from {}",
                            format_time(text(observation, "observedAt"))
                        ),
                    );
                    let sessions = array(&observation["sessions"]);
                    ui.add_space(12.0);
                    ui.strong(format!("Recent sessions ({})", sessions.len()));
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
                            &format!(
                                "Last observed {}",
                                format_time(text(session, "lastObservedAt"))
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
                ui.heading("Patchfleet Cloud");
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
                    "Cloud settings need local recovery. Local work remains available.",
                );
                ui.label("Close Patchfleet and back up the private cloud.json file in its data directory. Explicitly repair or remove that file, then reopen the app.");
                hint(
                    ui,
                    "Pairing is unavailable until the local Cloud state can be read safely.",
                );
                return;
            }
            if cloud["paired"] == true {
                ui.label(text(cloud, "cloudUrl"));
                ui.monospace(format!("Host {}", text(cloud, "hostId")));
                hint(
                    ui,
                    &format!(
                        "Last successful sync: {}",
                        format_time(text(cloud, "lastSuccessAt"))
                    ),
                );
                if let Some(code) = cloud["lastErrorCode"].as_str() {
                    ui.colored_label(RED, cloud_error(code));
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
fn format_time(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|t| t.format("%b %d, %H:%M UTC").to_string())
        .unwrap_or_else(|_| "Not recorded".into())
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
    ui.label(RichText::new(value).size(11.0).color(MUTED));
}
fn status(ui: &mut Ui, value: &str) {
    let color = match value {
        "available" | "completed" | "applied" | "Codex ready" | "Paired" => MINT,
        "failed" | "rejected" | "timed_out" => RED,
        _ => MUTED,
    };
    let label = value.replace('_', " ");
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
        ui.heading(title);
        ui.add_space(8.0);
        hint(ui, description);
    });
    ui.add_space(14.0);
}
fn field(ui: &mut Ui, label: &str, value: &mut String, max: usize, password: bool) {
    ui.label(RichText::new(label).size(11.0).color(MUTED));
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
    ui.label(RichText::new(label).size(11.0).color(MUTED));
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
        "Local project",
        selected,
        items(&snapshot["workspaces"]),
        ("workspaceId", "displayName"),
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
    ui.label(RichText::new(label).size(11.0).color(MUTED));
    let current = options
        .iter()
        .find(|v| text(v, id_key) == selected)
        .map(|v| text(v, name_key))
        .unwrap_or(if selected.is_empty() {
            "Choose…"
        } else {
            "Unavailable selection"
        });
    egui::ComboBox::from_id_salt(egui::Id::new(id))
        .selected_text(current)
        .width(ui.available_width().min(600.0))
        .show_ui(ui, |ui| {
            if empty {
                ui.selectable_value(selected, String::new(), "None");
            }
            for value in options {
                ui.selectable_value(
                    selected,
                    text(value, id_key).to_owned(),
                    text(value, name_key),
                );
            }
        });
    ui.add_space(8.0);
}
fn submit(ui: &mut Ui, label: &str, action: &str) -> bool {
    let response = ui.add(
        egui::Button::new(
            RichText::new(label)
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
        egui::Button::new(RichText::new(label).size(11.0).color(TEXT))
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
    ui.label(RichText::new(title).size(16.0).strong().color(TEXT));
    ui.add_space(3.0);
    hint(ui, description);
}
fn field_hint(ui: &mut Ui, label: &str, value: &mut String, max: usize, placeholder: &str) {
    ui.label(RichText::new(label).size(11.0).color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Body)
            .margin(egui::Margin::symmetric(10, 7))
            .char_limit(max)
            .hint_text(placeholder),
    );
    ui.add_space(12.0);
}
fn onboarding(ui: &mut Ui) {
    ui.label(
        RichText::new("A workspace that grows with you")
            .size(16.0)
            .strong()
            .color(TEXT),
    );
    ui.add_space(18.0);
    for (tab, title, copy) in [
        (
            Tab::Projects,
            "Connect a repository",
            "Point Patchfleet at an existing Git worktree.",
        ),
        (
            Tab::Work,
            "Run a focused task",
            "Give Codex one clear outcome, then start when ready.",
        ),
        (
            Tab::Teams,
            "Bring your agents together",
            "Choose roles, set limits, and keep decisions in your hands.",
        ),
    ] {
        ui.horizontal_top(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 6, Color32::from_rgb(28, 35, 40));
            draw_icon(ui.painter(), rect.shrink(4.0), tab, MINT);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(12.0).strong().color(TEXT));
                ui.add_space(3.0);
                hint(ui, copy);
            });
        });
        ui.add_space(18.0);
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
            tab.name(),
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
        tab.name(),
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
            "Add project",
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
}
