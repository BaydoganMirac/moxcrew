use super::*;

#[derive(Clone, PartialEq, Eq)]
struct CrewDraft {
    recommended_for: Option<String>,
    ability: String,
    name: String,
    members: Vec<Value>,
    settings: Value,
    crew_id: Option<String>,
}

struct PlanRequest {
    command: String,
    key: (String, String),
    draft: CreateDraft,
}

struct CrewSubmission {
    command: String,
    key: (String, String),
    draft: CreateDraft,
    reply: Option<ReplyDraft>,
    plan: CrewDraft,
}

#[derive(Default)]
pub(super) struct CrewState {
    pub(super) single: bool,
    plans: HashMap<(String, String), CrewDraft>,
    request: Option<PlanRequest>,
    pending: Option<CrewSubmission>,
    selected: HashMap<String, String>,
    pub(super) panel_open: bool,
    tab: usize,
    roster_open: bool,
    pub(super) catalog_target: Option<(String, String)>,
    workflow: WorkflowState,
    message_offset: usize,
    message_request: Option<(String, String, u64, usize)>,
}

fn missions(snapshot: &Value) -> &[Value] {
    array(&snapshot["projectCrews"]["missions"])
}
fn crews(snapshot: &Value) -> &[Value] {
    array(&snapshot["projectCrews"]["crews"])
}

fn ordered_missions<'a>(snapshot: &'a Value, project: &str) -> Vec<&'a Value> {
    let mut values: Vec<_> = missions(snapshot)
        .iter()
        .filter(|mission| text(mission, "projectId") == project)
        .collect();
    values.sort_by(|a, b| {
        text(a, "createdAt")
            .cmp(text(b, "createdAt"))
            .then_with(|| text(a, "missionId").cmp(text(b, "missionId")))
    });
    values
}

pub(super) fn contains_work(snapshot: &Value, project: &str, work: &str) -> bool {
    missions(snapshot)
        .iter()
        .filter(|mission| text(mission, "projectId") == project)
        .any(|mission| {
            array(&mission["tasks"])
                .iter()
                .any(|task| text(task, "workItemId") == work)
        })
}

fn role(category: &str) -> &str {
    match category {
        "research" => "research",
        "documents" => "document",
        "presentations" => "presentation",
        "video" => "video",
        "images" => "image",
        "audio" => "audio",
        "data" => "data",
        _ => "web",
    }
}

fn role_label(role: &str) -> &str {
    match role {
        "research" => "Research",
        "document" => "Documents",
        "presentation" => "Presentations",
        "video" => "Video",
        "image" => "Images",
        "audio" => "Audio",
        "data" => "Data",
        "web" => "Web",
        "review" => "Review",
        _ => "Agent",
    }
}

impl CreateState {
    pub(super) fn restore_latest_planner(&mut self, snapshot: &Value) {
        if self.project.is_empty()
            || self.has_conversation()
            || self.planner_job().is_some()
            || self.crew.workflow.request.is_some()
            || self.crew.plans.contains_key(&self.crew_key())
        {
            return;
        }
        let latest = workflow_values(snapshot, &self.project, "plans")
            .into_iter()
            .filter(|plan| {
                self.ability.is_empty()
                    || text(plan, "abilityId").is_empty()
                    || text(plan, "abilityId") == self.ability
            })
            .max_by_key(|plan| (text(plan, "createdAt"), text(plan, "planJobId")));
        if let Some(job) = latest.filter(|job| !text(job, "planJobId").is_empty()) {
            self.crew.workflow.models.insert(
                self.crew_key(),
                ModelPlan {
                    job: job.clone(),
                    draft: CreateDraft::default(),
                    answers: HashMap::new(),
                    assignments: HashMap::new(),
                    reply: None,
                    read_revision: None,
                },
            );
        }
    }

    pub(super) fn planner_provider(&mut self) -> &mut String {
        &mut self.crew.workflow.provider
    }

    pub(super) fn resolved_planner_provider(&self, snapshot: &Value) -> Option<&'static str> {
        connect::planning_provider(snapshot, &self.crew.workflow.provider)
    }

    pub(super) fn active_planner_provider(&self) -> Option<&str> {
        self.crew
            .workflow
            .request
            .as_ref()
            .filter(|request| request.key == self.crew_key())
            .map(|request| request.provider.as_str())
            .or_else(|| {
                self.planner_job()
                    .filter(|job| matches!(text(job, "status"), "queued" | "running"))
                    .map(|job| text(job, "providerId"))
            })
    }

    pub(super) fn planner_job(&self) -> Option<&Value> {
        self.crew
            .workflow
            .models
            .get(&self.crew_key())
            .map(|model| &model.job)
    }

    pub(super) fn planner_busy(&self) -> bool {
        self.crew.workflow.request.is_some()
            || self.crew.pending.is_some()
            || self
                .planner_job()
                .is_some_and(|job| matches!(text(job, "status"), "queued" | "running"))
    }

    pub(crate) fn productive_view(&self) -> usize {
        if self.capabilities.open {
            3
        } else if self.studio.open {
            2
        } else if self.crew.workflow.open {
            1
        } else {
            0
        }
    }
    pub(crate) fn select_productive_view(&mut self, view: usize) {
        if view != 2 {
            self.close_studio();
        } else {
            self.studio.open = true;
        }
        self.crew.workflow.open = view == 1;
        self.capabilities.open = view == 3;
        if view != 3 {
            self.capabilities.clear_private();
        }
        self.crew.panel_open = false;
        self.chat.files_open = false;
        self.chat.details_open = false;
        self.crew.roster_open = false;
        self.catalog_open = false;
    }
    fn crew_key(&self) -> (String, String) {
        (self.project.clone(), self.ability.clone())
    }

    pub(super) fn sync_crew(&mut self, snapshot: &Value) {
        self.sync_workflow(snapshot);
        if let Some(request) = &self.crew.request {
            let response = &snapshot["projectCrewPlan"];
            if text(response, "commandId") == request.command
                && text(response, "projectId") == request.key.0
                && text(response, "reasonCode") == "PROJECT_CREW_PLAN"
                && text(response, "basis") == "bundled_guides"
            {
                let plan = CrewDraft {
                    recommended_for: Some(request.draft.goal.clone()),
                    ability: text(response, "abilityId").into(),
                    name: draft_title(&CreateDraft {
                        title: String::new(),
                        goal: request.draft.goal.clone(),
                    })
                    .unwrap_or_default(),
                    members: array(&response["members"]).to_vec(),
                    settings: response["settings"].clone(),
                    crew_id: None,
                };
                self.crew.plans.insert(request.key.clone(), plan);
                self.crew.request = None;
            } else if text(&snapshot["projectCrewError"], "commandId") == request.command
                && text(&snapshot["projectCrewError"], "projectId") == request.key.0
            {
                self.crew.request = None;
            }
        }
        let Some(pending) = &self.crew.pending else {
            return;
        };
        let receipt = [
            &snapshot["projectCrewReceipt"],
            &snapshot["projectWorkflowReceipt"],
        ]
        .into_iter()
        .find(|receipt| text(receipt, "commandId") == pending.command)
        .unwrap_or(&Value::Null);
        if text(receipt, "commandId") != pending.command
            || text(receipt, "projectId") != pending.key.0
        {
            return;
        }
        if text(receipt, "outcome") == "applied" {
            let mission = text(receipt, "missionId");
            if !missions(snapshot).iter().any(|item| {
                text(item, "projectId") == pending.key.0 && text(item, "missionId") == mission
            }) {
                return;
            }
            let current_plan = self.crew_key() == pending.key
                && self.drafts.get(&pending.key) == Some(&pending.draft)
                && self.chat.replies.get(&pending.key) == pending.reply.as_ref()
                && self.crew.plans.get(&pending.key) == Some(&pending.plan);
            let return_to_chat =
                current_plan && self.productive_view() == 1 && self.crew.workflow.tab == 0;
            self.crew
                .selected
                .insert(pending.key.0.clone(), mission.into());
            if self.drafts.get(&pending.key) == Some(&pending.draft)
                && self.chat.replies.get(&pending.key) == pending.reply.as_ref()
                && self.crew.plans.get(&pending.key) == Some(&pending.plan)
            {
                self.drafts.remove(&pending.key);
                self.chat.replies.remove(&pending.key);
                self.crew.plans.remove(&pending.key);
                self.crew.workflow.models.remove(&pending.key);
            }
            if self.project == pending.key.0 && (self.productive_view() == 0 || return_to_chat) {
                if return_to_chat {
                    self.select_productive_view(0);
                }
                self.crew.panel_open = !self.has_conversation();
                self.crew.tab = 0;
                self.chat.files_open = false;
            }
        }
        self.crew.pending = None;
    }

    fn crew_payload(&self, snapshot: &Value) -> Option<Value> {
        let key = self.crew_key();
        let draft = self.drafts.get(&key)?;
        let plan = self.crew.plans.get(&key)?;
        if !valid_input(&draft.goal, 20_000)
            || plan
                .recommended_for
                .as_ref()
                .is_some_and(|goal| goal != &draft.goal)
            || !valid_input(&plan.name, 80)
            || !(1..=6).contains(&plan.members.len())
        {
            return None;
        }
        let project = array(&snapshot["projectMemory"]["projects"])
            .iter()
            .find(|project| text(project, "projectId") == self.project && revision(project) > 0)?;
        if !array(&snapshot["abilities"])
            .iter()
            .any(|ability| text(ability, "id") == plan.ability)
        {
            return None;
        }
        for member in &plan.members {
            if !valid_input(text(member, "name"), 80)
                || !valid_input(text(member, "instruction"), 4000)
                || !matches!(
                    text(member, "role"),
                    "research"
                        | "document"
                        | "presentation"
                        | "video"
                        | "image"
                        | "audio"
                        | "data"
                        | "web"
                        | "review"
                )
                || !array(&snapshot["abilities"])
                    .iter()
                    .any(|ability| ability["id"] == member["abilityId"])
            {
                return None;
            }
        }
        let mut settings = plan.settings.clone();
        settings["concurrency"] = json!(settings["concurrency"]
            .as_u64()?
            .clamp(1, 3)
            .min(plan.members.len() as u64));
        if settings["retryLimit"].as_u64()? > 2
            || !(1..=120).contains(&settings["timeoutMinutes"].as_u64()?)
        {
            return None;
        }
        let mut payload = json!({"projectId":self.project,"expectedProjectRevision":project["revision"],"title":draft_title(draft)?,"instruction":draft.goal.trim(),"abilityId":plan.ability,"crewName":plan.name,
            "settings":settings});
        if let Some(crew) = &plan.crew_id {
            if !crews(snapshot)
                .iter()
                .any(|item| text(item, "projectId") == self.project && text(item, "crewId") == crew)
            {
                return None;
            }
            payload["crewId"] = json!(crew);
        } else {
            payload["members"]=json!(plan.members.iter().map(|member|json!({"name":member["name"],"role":member["role"],"abilityId":member["abilityId"],"instruction":member["instruction"]})).collect::<Vec<_>>());
        }
        if let Some(reply) = self.chat.replies.get(&key) {
            if !job_belongs(snapshot, &self.project, &reply.parent) || reply.references.len() > 8 {
                return None;
            }
            let parent = items(&snapshot["work"])
                .iter()
                .find(|work| text(work, "workItemId") == reply.parent)?;
            if !matches!(
                text(parent, "status"),
                "completed" | "failed" | "interrupted"
            ) {
                return None;
            }
            let mut references: Vec<_> = reply.references.iter().cloned().collect();
            references.sort();
            payload["replyToWorkItemId"] = json!(reply.parent);
            payload["referencePaths"] = json!(references);
        }
        Some(payload)
    }

    fn crew_mission<'a>(&self, snapshot: &'a Value) -> Option<&'a Value> {
        let selected = self.crew.selected.get(&self.project)?;
        missions(snapshot).iter().find(|mission| {
            text(mission, "projectId") == self.project && text(mission, "missionId") == selected
        })
    }

    pub(super) fn crew_has_history(&self, snapshot: &Value) -> bool {
        self.has_conversation()
            || self.crew.workflow.models.contains_key(&self.crew_key())
            || self
                .crew
                .workflow
                .request
                .as_ref()
                .is_some_and(|request| request.key == self.crew_key())
            || self.crew.plans.contains_key(&self.crew_key())
            || missions(snapshot)
                .iter()
                .any(|mission| text(mission, "projectId") == self.project)
    }

    pub(super) fn add_crew_member(&mut self, ability: &Value, language: Language) {
        let Some(key) = self.crew.catalog_target.take() else {
            return;
        };
        let Some(plan) = self.crew.plans.get_mut(&key) else {
            return;
        };
        if plan.members.len() >= 6 {
            return;
        }
        plan.members.push(json!({"name":ability_text(ability,"name",language),"role":role(text(ability,"category")),"abilityId":ability["id"],"instruction":ability["instruction"],"_uiId":uuid::Uuid::new_v4().to_string()}));
        plan.crew_id = None;
        plan.recommended_for = None;
    }
}

impl Console {
    #[cfg(feature = "ui-smoke")]
    pub fn productive_smoke(&mut self, snapshot: &Value, scene: &str) {
        self.create_smoke(snapshot, false);
        self.create.crew.single = false;
        self.hold_automatic_reads = true;
        let project = self.create.project.clone();
        if let Some(mission) = ordered_missions(snapshot, &project).into_iter().next_back() {
            self.create.select_ability(text(mission, "abilityId"));
            *self.create.draft() = CreateDraft {
                title: text(mission, "title").into(),
                goal: text(mission, "instruction").into(),
            };
            self.create
                .crew
                .selected
                .insert(project.clone(), text(mission, "missionId").into());
        }
        if scene.starts_with("productive-output")
            || scene == "productive-compare"
            || matches!(
                scene,
                "productive-resources"
                    | "productive-apps"
                    | "productive-app-preview"
                    | "productive-tools"
                    | "productive-delivery"
            )
        {
            self.create.studio.open = true;
            self.create.studio.tab = match scene {
                "productive-resources" => 1,
                "productive-tools" => 2,
                "productive-apps" | "productive-app-preview" => 3,
                _ => 0,
            };
            self.create.studio.requested = Some((
                project.clone(),
                output_studio::project_revision(snapshot, &project).unwrap_or(0),
            ));
            self.studio_select_first(snapshot);
            if scene == "productive-compare" {
                self.studio_compare_smoke(snapshot);
            }
            if scene == "productive-delivery" {
                self.delivery_smoke(snapshot);
            }
            if scene == "productive-app-preview" {
                self.studio_prepare_smoke(snapshot);
            }
        } else {
            self.create.crew.workflow.open = true;
            self.create.crew.workflow.tab = match scene {
                "productive-direction" => 1,
                "productive-recipes" => 2,
                "productive-schedules" => 3,
                "productive-activity" => 4,
                _ => 0,
            };
            self.create.crew.workflow.provider = "codex".into();
            self.create.crew.workflow.read =
                Some((project, revision(&snapshot["projectWorkflows"])));
            let job = &snapshot["projectCrewPlanJob"];
            if job["planJobId"].is_string() {
                let draft = self.create.draft().clone();
                self.create.crew.workflow.models.insert(
                    self.create.crew_key(),
                    ModelPlan {
                        job: job.clone(),
                        draft,
                        reply: None,
                        answers: HashMap::new(),
                        assignments: HashMap::new(),
                        read_revision: Some(revision(job)),
                    },
                );
                self.create.sync_workflow(snapshot);
            }
        }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn crew_smoke(&mut self, snapshot: &Value, scene: &str) {
        self.create_smoke(snapshot, false);
        self.create.crew.single = false;
        if let Some(mission) = ordered_missions(snapshot, &self.create.project)
            .into_iter()
            .next_back()
        {
            self.create.select_ability(text(mission, "abilityId"));
            *self.create.draft() = CreateDraft {
                title: text(mission, "title").into(),
                goal: text(mission, "instruction").into(),
            };
            self.create.crew.selected.insert(
                self.create.project.clone(),
                text(mission, "missionId").into(),
            );
        }
        if matches!(scene, "crew-plan" | "crew-roster") {
            self.create.crew.request = Some(PlanRequest {
                command: text(&snapshot["projectCrewPlan"], "commandId").into(),
                key: self.create.crew_key(),
                draft: self.create.draft().clone(),
            });
            self.create.sync_crew(snapshot);
            self.create.crew.roster_open = scene == "crew-roster";
        }
        if matches!(scene, "crew-running" | "crew-messages") {
            self.create.crew.panel_open = true;
            self.create.crew.tab = usize::from(scene == "crew-messages");
            if scene == "crew-messages" {
                if let Some(mission) = self.create.crew.selected.get(&self.create.project) {
                    self.create.crew.message_request = Some((
                        self.create.project.clone(),
                        mission.clone(),
                        revision(&snapshot["projectCrews"]),
                        0,
                    ));
                }
            }
        }
        if scene == "crew-teams" {
            self.tab = Tab::Teams;
        }
    }

    pub(super) fn crew_plan_button(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
        shortcut: bool,
    ) {
        let goal = self.create.draft().goal.clone();
        let project = array(&snapshot["projectMemory"]["projects"])
            .iter()
            .any(|project| {
                text(project, "projectId") == self.create.project && revision(project) > 0
            });
        let enabled = studio::commands_ready(ui)
            && project
            && valid_input(&goal, 20_000)
            && self.create.crew.pending.is_none()
            && self.create.pending.is_none();
        let clicked = ui
            .add_enabled_ui(enabled, |ui| submit(ui, "Plan crew", "project_crew_plan"))
            .inner;
        if (clicked || (shortcut && enabled)) && studio::commands_ready(ui) && commands.is_empty() {
            let mut payload = json!({"projectId":self.create.project,"instruction":goal});
            if !self.create.ability.is_empty() {
                payload["abilityId"] = json!(self.create.ability);
            }
            let command = Command::new("project_crew_plan", payload);
            self.create.crew.request = Some(PlanRequest {
                command: text(&command.payload, "commandId").into(),
                key: self.create.crew_key(),
                draft: self.create.draft().clone(),
            });
            commands.push(command);
            self.create.select_productive_view(1);
        }
        if view_control(ui, "Mox planner", true, false).clicked() {
            self.create.select_productive_view(1);
            self.create.crew.workflow.tab = 0;
        }
    }

    pub(super) fn crew_chat_content(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        let project = self.create.project.clone();
        self.conversation_turns(ui);
        for mission in ordered_missions(snapshot, &project)
            .into_iter()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            ui.push_id(text(mission, "missionId"), |ui| {
                self.crew_mission_card(ui, snapshot, mission, commands, true)
            });
        }
        self.model_plan_card(ui, snapshot, commands);
        if self.create.has_conversation() && self.create.crew.workflow.request.is_some() {
            hint(ui, "Waiting for planner confirmation…");
        }
        if self.create.crew.plans.contains_key(&self.create.crew_key()) {
            if self.create.productive_view() == 1 {
                self.crew_plan_chat_summary(ui, snapshot);
            } else {
                self.crew_plan_card(ui, snapshot, commands);
            }
        }
    }

    fn crew_plan_chat_summary(&mut self, ui: &mut Ui, snapshot: &Value) {
        let accent = colors(ui).primary;
        studio::accent_surface(ui, accent, |ui| self.crew_plan_summary(ui, snapshot));
    }

    fn crew_plan_summary(&mut self, ui: &mut Ui, snapshot: &Value) {
        let Some(plan) = self.create.crew.plans.get(&self.create.crew_key()) else {
            return;
        };
        let count = plan.members.len().to_string();
        let goal = plan
            .recommended_for
            .clone()
            .unwrap_or_else(|| self.create.draft().goal.clone());
        ui.horizontal(|ui| {
            crate::motion::mox(
                ui,
                24.0,
                false,
                snapshot["desktopPreferences"]["reduceMotion"] == true,
            );
            ui.label(
                RichText::new(t(ui, "Your crew is ready"))
                    .size(14.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(tf(ui, "{0} agents", &[&count]))
                        .size(12.0)
                        .color(colors(ui).muted),
                );
            });
        });
        ui.add_space(studio::metrics::GAP);
        short_copy(ui, &goal, 14.0, colors(ui).text, 2);
    }

    fn crew_prepare_action(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let key = self.create.crew_key();
        let Some(plan) = self.create.crew.plans.get(&key).cloned() else {
            return;
        };
        let current_goal = self.create.draft().goal.clone();
        let model = self.create.crew.workflow.models.get(&key).cloned();
        let payload=self.create.crew_payload(snapshot).filter(|_|model.as_ref().is_none_or(|model|text(&model.job,"status")=="completed" && model.draft.goal==current_goal && model.reply.as_ref()==self.create.chat.replies.get(&key))).map(|mut payload| {
            if let Some(model)=&model {
                payload["planJobId"]=model.job["planJobId"].clone();
                payload["expectedPlanRevision"]=model.job["revision"].clone();
                payload["providerAssignments"]=json!(plan.members.iter().enumerate().map(|(index,member)|json!({"memberIndex":index,"providerId":model.assignments.get(text(member,"_uiId")).map(String::as_str).unwrap_or(text(&model.job,"providerId"))})).collect::<Vec<_>>());
                for key in ["abilityId","crewId","title","instruction"] {payload.as_object_mut().unwrap().remove(key);}
            }
            payload
        });
        let clicked = ui
            .add_enabled_ui(
                payload.is_some()
                    && studio::commands_ready(ui)
                    && self.create.crew.pending.is_none()
                    && self.create.pending.is_none(),
                |ui| submit(ui, "Prepare crew", "project_crew_prepare"),
            )
            .inner;
        if clicked && studio::commands_ready(ui) && commands.is_empty() {
            if let Some(payload) = payload {
                let command = Command::new(
                    if model.is_some() {
                        "project_crew_plan_accept"
                    } else {
                        "project_crew_prepare"
                    },
                    payload,
                );
                self.create.crew.pending = Some(CrewSubmission {
                    command: text(&command.payload, "commandId").into(),
                    key: key.clone(),
                    draft: self.create.draft().clone(),
                    reply: self.create.chat.replies.get(&key).cloned(),
                    plan: plan.clone(),
                });
                commands.push(command);
            }
        }
    }
    fn crew_roster_summary(&mut self, ui: &mut Ui, snapshot: &Value, show_title: bool) {
        let key = self.create.crew_key();
        let Some(plan) = self.create.crew.plans.get(&key).cloned() else {
            return;
        };
        if show_title {
            ui.label(studio::title(t(ui, "Your crew is ready"), 20.0));
            ui.add_space(studio::metrics::CONTROL_GAP);
        }
        let goal = plan
            .recommended_for
            .clone()
            .unwrap_or_else(|| self.create.draft().goal.clone());
        short_copy(ui, &goal, 14.0, colors(ui).text, 2);
        let source = if plan.crew_id.is_some() {
            "Saved project crew"
        } else if self.create.crew.workflow.models.contains_key(&key) {
            "Provider-generated plan"
        } else if plan.recommended_for.is_some() {
            "Based on bundled guides"
        } else {
            "Your edited roster"
        };
        ui.label(
            RichText::new(format!(
                "{}. {}",
                tf(ui, "{0} agents", &[&plan.members.len().to_string()]),
                t(ui, source)
            ))
            .size(12.0)
            .color(colors(ui).muted),
        );
        ui.add_space(f32::from(studio::metrics::PADDING));
        let columns = if ui.available_width() >= 620.0 && plan.members.len() >= 3 {
            3
        } else if ui.available_width() >= 440.0 && plan.members.len() >= 2 {
            2
        } else {
            1
        };
        let width = (ui.available_width() - studio::metrics::CONTROL_GAP * (columns - 1) as f32)
            / columns as f32;
        egui::Grid::new(("crew-plan-cards", &key))
            .num_columns(columns)
            .spacing(egui::vec2(
                studio::metrics::CONTROL_GAP,
                studio::metrics::GAP,
            ))
            .show(ui, |ui| {
                for (index, member) in plan.members.iter().enumerate() {
                    let role = text(member, "role");
                    let p = colors(ui);
                    let accent = match role {
                        "research" => p.info,
                        "review" => p.success,
                        "presentation" => p.warning,
                        _ => p.primary,
                    };
                    let ability = array(&snapshot["abilities"])
                        .iter()
                        .find(|ability| ability["id"] == member["abilityId"]);
                    let icon = if role == "review" {
                        "review"
                    } else {
                        ability
                            .map(|ability| text(ability, "category"))
                            .unwrap_or("connections")
                    };
                    let capability = if role == "review" {
                        t(ui, "Quality review")
                    } else {
                        ability
                            .map(|ability| {
                                std::borrow::Cow::Borrowed(ability_text(
                                    ability,
                                    "name",
                                    Language::from_snapshot(snapshot),
                                ))
                            })
                            .unwrap_or_else(|| t(ui, role_label(role)))
                    };
                    let cell = ui.allocate_ui_with_layout(
                        egui::vec2(width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::NONE.inner_margin(8).show(ui, |ui| {
                                ui.set_width((width - 16.0).max(studio::metrics::CONTROL_HEIGHT));
                                ui.horizontal(|ui| {
                                    studio::icon(ui, icon, 24.0, accent);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 4.0;
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(text(member, "name"))
                                                    .size(14.0)
                                                    .strong(),
                                            )
                                            .truncate(),
                                        )
                                        .on_hover_text(text(member, "name"));
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(capability).size(12.0).color(accent),
                                            )
                                            .truncate(),
                                        )
                                        .on_hover_text(t(ui, role_label(role)));
                                    });
                                });
                            });
                        },
                    );
                    #[cfg(test)]
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new(("crew-roster-row", index)),
                            (cell.response.rect, ui.clip_rect()),
                        )
                    });
                    #[cfg(not(test))]
                    let _ = cell;
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
        if plan
            .recommended_for
            .as_ref()
            .is_some_and(|goal| goal != &self.create.draft().goal)
        {
            hint(
                ui,
                "Your goal changed. Plan the crew again before preparing.",
            );
        }
    }
    fn crew_plan_card(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        ui.add_space(studio::metrics::GAP);
        let accent = colors(ui).primary;
        let proposal = studio::accent_surface(ui, accent, |ui| {
            if self.create.has_conversation() {
                self.crew_plan_summary(ui, snapshot);
                if let Some(plan) = self.create.crew.plans.get(&self.create.crew_key()) {
                    ui.add_space(studio::metrics::GAP);
                    ui.horizontal_wrapped(|ui| {
                        for (index, member) in plan.members.iter().enumerate() {
                            if index > 0 {
                                ui.label(RichText::new(",").color(colors(ui).muted));
                            }
                            ui.label(RichText::new(text(member, "name")).size(13.0));
                        }
                    });
                }
            } else {
                self.crew_roster_summary(ui, snapshot, true);
            }
            ui.add_space(studio::metrics::CONTROL_GAP);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.crew_prepare_action(ui, snapshot, commands);
                    if self.create.has_conversation() {
                        if view_control(ui, "Review crew", true, false).clicked() {
                            self.create.select_productive_view(1);
                        }
                    } else if view_control(ui, "Edit roster", true, false).clicked() {
                        self.create.crew.roster_open = true;
                    }
                });
            });
            hint(
                ui,
                if self.create.crew.pending.is_some() || !studio::commands_ready(ui) {
                    "Applying action…"
                } else if self.create.crew_payload(snapshot).is_none() {
                    "Review the goal and roster before preparing."
                } else {
                    "Preparing saves the mission. Agents start only when you choose Start crew."
                },
            );
        });
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("crew-proposal-card"),
                (proposal.response.rect, ui.clip_rect()),
            )
        });
        #[cfg(not(test))]
        let _ = proposal;
    }

    pub(super) fn crew_roster_dialog(&mut self, ui: &mut Ui, snapshot: &Value) {
        if !self.create.crew.roster_open {
            return;
        }
        let key = self.create.crew_key();
        let Some(mut plan) = self.create.crew.plans.get(&key).cloned() else {
            self.create.crew.roster_open = false;
            return;
        };
        let before = plan.clone();
        let id = egui::Id::new("crew-roster-editor");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .frame(studio::card(ui))
            .show(ui.ctx(), |ui| {
                ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 640.0));
                ui.label(studio::title(t(ui, "Build your crew"), 24.0));
                field(ui, "Crew name", &mut plan.name, 80, false);
                egui::ScrollArea::vertical()
                    .id_salt("crew-roster-members")
                    .max_height((ui.ctx().viewport_rect().height() - 290.0).max(100.0))
                    .show(ui, |ui| {
                        let mut remove = None;
                        for (index, member) in plan.members.iter_mut().enumerate() {
                            ui.push_id(index, |ui| {
                                card(ui, |ui| {
                                    let mut name = text(member, "name").to_owned();
                                    ui.horizontal(|ui| {
                                        let remove_width = ui
                                            .painter()
                                            .layout_no_wrap(
                                                t(ui, "Remove agent").into_owned(),
                                                egui::FontId::proportional(14.0),
                                                colors(ui).text,
                                            )
                                            .size()
                                            .x
                                            + f32::from(studio::metrics::PADDING) * 2.0;
                                        let name_width = (ui.available_width()
                                            - remove_width
                                            - ui.spacing().item_spacing.x)
                                            .max(studio::metrics::CONTROL_HEIGHT);
                                        ui.add_sized(
                                            egui::vec2(name_width, studio::metrics::CONTROL_HEIGHT),
                                            egui::TextEdit::singleline(&mut name)
                                                .desired_width(name_width)
                                                .font(egui::TextStyle::Body)
                                                .margin(studio::metrics::INPUT_MARGIN)
                                                .hint_text(t(ui, "Agent name"))
                                                .char_limit(80),
                                        );
                                        if control(ui, "Remove agent", true).clicked() {
                                            remove = Some(index);
                                        }
                                    });
                                    member["name"] = json!(name);
                                    let ability = array(&snapshot["abilities"])
                                        .iter()
                                        .find(|ability| ability["id"] == member["abilityId"]);
                                    if let Some(ability) = ability {
                                        raw_hint(
                                            ui,
                                            ability_text(
                                                ability,
                                                "name",
                                                Language::from_snapshot(snapshot),
                                            ),
                                        );
                                    }
                                    let roles = [
                                        ("research", "Research"),
                                        ("document", "Documents"),
                                        ("presentation", "Presentations"),
                                        ("video", "Video"),
                                        ("image", "Images"),
                                        ("audio", "Audio"),
                                        ("data", "Data"),
                                        ("web", "Web"),
                                        ("review", "Review"),
                                    ];
                                    let mut selected_role = text(member, "role").to_owned();
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(t(ui, "Agent role"));
                                        let label = roles
                                            .iter()
                                            .find(|(value, _)| *value == selected_role)
                                            .map(|(_, label)| *label)
                                            .unwrap_or("Choose…");
                                        form_combo(
                                            ui,
                                            egui::ComboBox::from_id_salt("crew-member-role")
                                                .selected_text(t(ui, label))
                                                .width(180.0),
                                            |ui| {
                                                for (value, label) in roles {
                                                    ui.selectable_value(
                                                        &mut selected_role,
                                                        value.into(),
                                                        t(ui, label),
                                                    );
                                                }
                                            },
                                        );
                                    });
                                    member["role"] = json!(selected_role);
                                    ui.collapsing(t(ui, "Instructions"), |ui| {
                                        let mut instruction =
                                            text(member, "instruction").to_owned();
                                        ui.add(
                                            egui::TextEdit::multiline(&mut instruction)
                                                .font(egui::TextStyle::Body)
                                                .margin(studio::metrics::INPUT_MARGIN)
                                                .desired_width(f32::INFINITY)
                                                .desired_rows(3)
                                                .char_limit(4000),
                                        );
                                        member["instruction"] = json!(instruction);
                                    });
                                });
                            });
                        }
                        if let Some(index) = remove {
                            plan.members.remove(index);
                        }
                        if plan.members.is_empty() {
                            hint(ui, "Add at least one agent.");
                        }
                        ui.collapsing(t(ui, "Mission limits"), |ui| {
                            for (field, label, min, max) in [
                                ("concurrency", "Parallel tasks", 1, 3),
                                ("retryLimit", "Retry limit", 0, 2),
                                ("timeoutMinutes", "Time limit (minutes)", 1, 120),
                            ] {
                                let mut value = plan.settings[field].as_u64().unwrap_or(min) as u32;
                                ui.horizontal(|ui| {
                                    ui.label(t(ui, label));
                                    ui.add(
                                        egui::DragValue::new(&mut value).range(min as u32..=max),
                                    );
                                });
                                plan.settings[field] = json!(value);
                            }
                        });
                    });
                ui.horizontal_wrapped(|ui| {
                    if control(ui, "Add from skills", plan.members.len() < 6).clicked() {
                        self.create.crew.catalog_target = Some(key.clone());
                        self.create.catalog_open = true;
                    }
                    if control(ui, "Done", true).clicked() {
                        self.create.crew.roster_open = false;
                    }
                });
            });
        if modal.should_close() {
            self.create.crew.roster_open = false;
        }
        if plan != before {
            plan.crew_id = None;
            plan.recommended_for = None;
            self.create.crew.plans.insert(key, plan);
        }
    }

    fn reuse_project_crew(&mut self, snapshot: &Value, crew: &Value) {
        if text(crew, "projectId") != self.create.project {
            return;
        }
        let previous = ordered_missions(snapshot, &self.create.project)
            .into_iter()
            .rev()
            .find(|mission| mission["crewId"] == crew["crewId"]);
        let Some(previous) = previous else { return };
        self.create.select_ability(text(previous, "abilityId"));
        self.create.crew.plans.insert(
            self.create.crew_key(),
            CrewDraft {
                recommended_for: None,
                ability: text(previous, "abilityId").into(),
                name: text(crew, "name").into(),
                members: array(&crew["members"]).to_vec(),
                settings: previous["settings"].clone(),
                crew_id: Some(text(crew, "crewId").into()),
            },
        );
        self.create.crew.single = false;
        self.create.crew.panel_open = false;
        self.tab = Tab::Create;
    }

    pub(super) fn crew_hub_view(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        section_title(
            ui,
            "Project crews",
            "Save a useful roster. Give the same crew a new goal in its project.",
        );
        let projects = array(&snapshot["projectMemory"]["projects"]);
        if self.create.project.is_empty() && projects.len() == 1 {
            self.create.project = text(&projects[0], "projectId").into();
        }
        if projects.is_empty() {
            hint(
                ui,
                "Create a project to keep your work together. No folder setup needed.",
            );
            if submit(ui, "Create project", "create-project") {
                self.new_project_open = true;
            }
            return;
        }
        project_switcher(
            ui,
            "crew-project-picker",
            &mut self.create.project,
            projects,
        );
        if !projects
            .iter()
            .any(|project| text(project, "projectId") == self.create.project)
        {
            hint(ui, "Choose the project for this idea.");
            return;
        }
        if control(ui, "Recipes & controls", true).clicked() {
            self.create.crew.workflow.open = true;
        }
        let project = self.create.project.clone();
        let saved: Vec<_> = crews(snapshot)
            .iter()
            .filter(|crew| text(crew, "projectId") == project)
            .collect();
        if saved.is_empty() {
            hint(ui, "No saved crew yet. Plan one from your next goal.");
        }
        for crew in saved {
            ui.push_id(text(crew, "crewId"), |ui| {
                card(ui, |ui| {
                    ui.label(studio::title(text(crew, "name"), 20.0));
                    ui.horizontal_wrapped(|ui| {
                        for member in array(&crew["members"]) {
                            ui.label(text(member, "name"));
                        }
                    });
                    if control(ui, "Reuse crew", true).clicked() {
                        self.reuse_project_crew(snapshot, crew);
                    }
                })
            });
        }
        if control(ui, "Plan a new crew", true).clicked() {
            self.create.crew.single = false;
            self.tab = Tab::Create;
        }
        ui.add_space(f32::from(studio::metrics::PADDING));
        ui.label(studio::title(t(ui, "Missions"), 20.0));
        for mission in ordered_missions(snapshot, &project)
            .into_iter()
            .rev()
            .take(20)
        {
            ui.push_id(text(mission, "missionId"), |ui| {
                self.crew_mission_card(ui, snapshot, mission, commands, false)
            });
        }
        self.crew_overlay(ui, snapshot, commands);
    }

    fn crew_mission_card(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        mission: &Value,
        commands: &mut Vec<Command>,
        show_goal: bool,
    ) {
        ui.add_space(studio::metrics::CONTROL_GAP);
        let palette = colors(ui);
        let accent = match text(mission, "status") {
            "completed" => palette.success,
            "failed" | "cancelled" | "timed_out" => palette.danger,
            "active" | "cancelling" => palette.accent,
            _ => palette.primary,
        };
        studio::accent_surface(ui, accent, |ui| {
            ui.horizontal_wrapped(|ui| {
                crate::motion::mox(
                    ui,
                    36.0,
                    matches!(text(mission, "status"), "active" | "cancelling"),
                    snapshot["desktopPreferences"]["reduceMotion"] == true,
                );
                ui.label(studio::title(text(mission, "title"), 20.0));
                status(ui, text(mission, "status"));
            });
            if show_goal {
                let goal = ui.collapsing(t(ui, "Goal"), |ui| {
                    ui.add(
                        egui::Label::new(text(mission, "instruction"))
                            .wrap()
                            .selectable(true),
                    );
                });
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("mission-goal", text(mission, "missionId"))),
                        goal.header_response.id,
                    )
                });
                #[cfg(not(test))]
                let _ = goal;
            }
            let tasks = array(&mission["tasks"]);
            let complete = tasks
                .iter()
                .filter(|task| text(task, "status") == "completed")
                .count();
            hint(
                ui,
                &tf(
                    ui,
                    "{0} of {1} tasks completed",
                    &[&complete.to_string(), &tasks.len().to_string()],
                ),
            );
            if text(mission, "status") == "prepared" {
                hint(ui, "Prepared crew. No agents have started.");
            }
            self.crew_mission_actions(ui, snapshot, mission, commands);
            if control(ui, "Open crew", true).clicked() {
                self.create.crew.selected.insert(
                    self.create.project.clone(),
                    text(mission, "missionId").into(),
                );
                self.create.crew.panel_open = true;
                self.create.crew.tab = 0;
                self.create.chat.files_open = false;
            }
        });
        ui.add_space(studio::metrics::CONTROL_GAP);
    }

    fn crew_mission_actions(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        mission: &Value,
        commands: &mut Vec<Command>,
    ) {
        let valid = text(mission, "projectId") == self.create.project && revision(mission) > 0;
        let mut abilities = Vec::new();
        if !text(mission, "abilityId").is_empty() {
            abilities.push(text(mission, "abilityId"));
        }
        for task in array(&mission["tasks"]) {
            let ability = text(task, "abilityId");
            if !ability.is_empty() && !abilities.contains(&ability) {
                abilities.push(ability);
            }
        }
        let start_ready = if text(mission, "status") == "prepared" {
            self.start_readiness_preflight(
                ui,
                snapshot,
                commands,
                text(mission, "projectId"),
                &abilities,
            )
        } else {
            true
        };
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if text(mission,"status")=="prepared" {
                    action(ui,commands,"Start crew","project_crew_start",start_ready && valid && connect::crew_can_start(snapshot,mission),||json!({"projectId":mission["projectId"],"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"]}));
                }
                if matches!(text(mission,"status"),"prepared"|"active") {
                    action(ui,commands,"Stop crew","project_crew_cancel",valid,||json!({"projectId":mission["projectId"],"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"]}));
                }
                if matches!(text(mission,"status"),"failed"|"cancelled"|"timed_out") {
                    action(ui,commands,"Retry crew","project_crew_retry",valid,||json!({"projectId":mission["projectId"],"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"]}));
                }
            });
        });
        if text(mission, "status") == "prepared" && !connect::crew_can_start(snapshot, mission) {
            ui.horizontal_wrapped(|ui| {
                hint(ui, "Check the assigned providers before starting.");
                if control(ui, "Connections", true).clicked() {
                    self.create.chat.connections_open = true;
                }
            });
        }
    }

    pub(super) fn crew_panel(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if self.create.crew_mission(snapshot).is_none() {
            if let Some(mission) = ordered_missions(snapshot, &self.create.project)
                .into_iter()
                .next_back()
            {
                self.create.crew.selected.insert(
                    self.create.project.clone(),
                    text(mission, "missionId").into(),
                );
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(studio::title(t(ui, "Project crew"), 20.0));
            if control(ui, "Close crew", true).clicked() {
                self.create.crew.panel_open = false;
            }
        });
        let project_missions = ordered_missions(snapshot, &self.create.project);
        if project_missions.len() > 1 {
            let selected = self
                .create
                .crew
                .selected
                .entry(self.create.project.clone())
                .or_default();
            let before = selected.clone();
            let title = project_missions
                .iter()
                .find(|mission| text(mission, "missionId") == selected)
                .map(|mission| text(mission, "title"))
                .unwrap_or("");
            egui::ComboBox::from_id_salt("crew-mission-picker")
                .selected_text(title)
                .truncate()
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for mission in &project_missions {
                        ui.selectable_value(
                            selected,
                            text(mission, "missionId").into(),
                            text(mission, "title"),
                        );
                    }
                });
            if *selected != before {
                self.create.crew.message_offset = 0;
                self.create.crew.message_request = None;
                self.create.result_path.clear();
            }
        }
        let Some(mission) = self.create.crew_mission(snapshot) else {
            hint(ui, "No mission selected.");
            return;
        };
        if control(ui, "Recipes & controls", true).clicked() {
            self.create.crew.workflow.open = true;
        }
        ui.label(RichText::new(text(mission, "title")).strong());
        status(ui, text(mission, "status"));
        ui.horizontal_wrapped(|ui| {
            for (index, label) in [(0, "Agents"), (1, "Messages"), (2, "Files")] {
                if ui
                    .selectable_value(&mut self.create.crew.tab, index, t(ui, label))
                    .clicked()
                {
                    self.create.crew.message_offset = 0;
                    self.create.crew.message_request = None;
                }
            }
        });
        ui.add_space(studio::metrics::GAP);
        match self.create.crew.tab {
            1 => self.crew_messages(ui, snapshot, mission, commands),
            2 => {
                let work_ids: Vec<String> = array(&mission["tasks"])
                    .iter()
                    .filter_map(|task| task["workItemId"].as_str().map(str::to_owned))
                    .collect();
                egui::ScrollArea::vertical()
                    .id_salt("crew-file-list")
                    .max_height((ui.ctx().viewport_rect().height() - 230.0).max(100.0))
                    .show(ui, |ui| {
                        self.create_jobs_scoped(ui, snapshot, commands, Some(&work_ids))
                    });
            }
            _ => {
                self.crew_mission_actions(ui, snapshot, mission, commands);
                egui::ScrollArea::vertical().id_salt("crew-real-agents").max_height((ui.ctx().viewport_rect().height()-420.0).max(100.0)).show(ui,|ui| {
                    if let Some(crew)=crews(snapshot).iter().find(|crew|crew["crewId"]==mission["crewId"] && text(crew,"projectId")==self.create.project) {
                        for member in array(&crew["members"]) {
                            card(ui,|ui| {
                                ui.label(RichText::new(text(member,"name")).strong());
                                let tasks:Vec<_>=array(&mission["tasks"]).iter().filter(|task|task["agentId"]==member["agentId"]).collect();
                                if tasks.is_empty(){hint(ui,"Not started");}
                                for task in tasks {
                                    status(ui,text(task,"status"));
                                    if !text(task,"title").is_empty() {ui.label(text(task,"title"));}
                                    if !text(task,"reasonCode").is_empty() {hint(ui,text(task,"reasonCode"));}
                                    if let Some(work)=task["workItemId"].as_str().filter(|work|job_belongs(snapshot,&self.create.project,work)) {
                                        let before=commands.len();
                                        action(ui,commands,"Agent files","project_results_read",!cfg!(target_os="windows"),||json!({"projectId":self.create.project,"workItemId":work}));
                                        if commands.len()>before {self.create.selected_work.insert(self.create.project.clone(),work.into());self.create.result_path.clear();self.create.crew.tab=2;}
                                    }
                                }
                            });
                        }
                    }
                });
            }
        }
    }

    pub(super) fn crew_overlay(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if !self.create.crew.panel_open {
            return;
        }
        let ready = ui.is_enabled();
        let id = egui::Id::new("project-crew-panel");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .frame(studio::card(ui))
            .show(ui.ctx(), |ui| {
                ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 640.0));
                ui.add_enabled_ui(ready, |ui| self.crew_panel(ui, snapshot, commands));
            });
        if modal.should_close() {
            self.create.crew.panel_open = false;
        }
    }

    fn crew_messages(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        mission: &Value,
        commands: &mut Vec<Command>,
    ) {
        ui.horizontal_wrapped(|ui| {
            if control(ui, "Refresh messages", !self.hold_automatic_reads).clicked() {
                self.create.crew.message_request = None;
            }
            if self.create.crew.message_offset > 0
                && control(ui, "Previous messages", !self.hold_automatic_reads).clicked()
            {
                self.create.crew.message_offset =
                    self.create.crew.message_offset.saturating_sub(20);
            }
        });
        let page = &snapshot["projectCrewMessages"];
        let valid = text(page, "reasonCode") == "PROJECT_CREW_MESSAGES_READ"
            && text(page, "projectId") == self.create.project
            && page["missionId"] == mission["missionId"]
            && page["offset"].as_u64().unwrap_or(0) == self.create.crew.message_offset as u64;
        egui::ScrollArea::vertical()
            .id_salt("crew-messages")
            .max_height((ui.ctx().viewport_rect().height() - 390.0).max(100.0))
            .show(ui, |ui| {
                if !valid {
                    hint(ui, "Load the real messages for this mission.");
                } else if array(&page["items"]).is_empty() {
                    hint(ui, "No messages yet.");
                } else {
                    for message in array(&page["items"]) {
                        card(ui, |ui| {
                            let mut from = self.crew_agent_name(
                                snapshot,
                                text(message, "fromMissionId"),
                                text(message, "fromAgentId"),
                            );
                            let mut to = self.crew_agent_name(
                                snapshot,
                                text(message, "toMissionId"),
                                text(message, "toAgentId"),
                            );
                            for (label, field) in
                                [(&mut from, "fromMissionId"), (&mut to, "toMissionId")]
                            {
                                if message[field] != mission["missionId"] {
                                    if let Some(context) = missions(snapshot).iter().find(|item| {
                                        text(item, "projectId") == self.create.project
                                            && item["missionId"] == message[field]
                                    }) {
                                        *label = format!("{} · {}", label, text(context, "title"));
                                    }
                                }
                            }
                            raw_hint(ui, &format!("{from} > {to}"));
                            ui.add(
                                egui::Label::new(text(message, "body"))
                                    .wrap()
                                    .selectable(true),
                            );
                            hint(
                                ui,
                                if text(message, "delivery") == "read" {
                                    "Read by the recipient"
                                } else {
                                    "Pending recipient read"
                                },
                            );
                        });
                    }
                }
            });
        if valid {
            if let Some(next) = page["nextOffset"].as_u64() {
                if control(ui, "Next messages", !self.hold_automatic_reads).clicked() {
                    self.create.crew.message_offset = next as usize;
                }
            }
        }
        hint(
            ui,
            "Messages do not restart a stopped agent or grant permissions.",
        );
        let key = (
            self.create.project.clone(),
            text(mission, "missionId").to_owned(),
            revision(&snapshot["projectCrews"]),
            self.create.crew.message_offset,
        );
        if self.create.crew.message_request.as_ref() != Some(&key)
            && ui.is_enabled()
            && !self.hold_automatic_reads
            && studio::commands_ready(ui)
            && commands.is_empty()
        {
            self.create.crew.message_request = Some(key);
            commands.push(Command::new("project_crew_messages_read",json!({"projectId":self.create.project,"missionId":mission["missionId"],"offset":self.create.crew.message_offset,"limit":20})));
        }
    }

    fn crew_agent_name(&self, snapshot: &Value, mission_id: &str, agent_id: &str) -> String {
        missions(snapshot)
            .iter()
            .find(|mission| {
                text(mission, "projectId") == self.create.project
                    && text(mission, "missionId") == mission_id
            })
            .and_then(|mission| {
                crews(snapshot).iter().find(|crew| {
                    crew["crewId"] == mission["crewId"]
                        && text(crew, "projectId") == self.create.project
                })
            })
            .and_then(|crew| {
                array(&crew["members"])
                    .iter()
                    .find(|member| text(member, "agentId") == agent_id)
            })
            .map(|member| text(member, "name").to_owned())
            .unwrap_or_else(|| i18n::tr_for_snapshot(snapshot, "Unknown agent").into_owned())
    }
}

#[derive(Clone)]
struct ModelPlan {
    job: Value,
    draft: CreateDraft,
    answers: HashMap<String, String>,
    assignments: HashMap<String, String>,
    reply: Option<ReplyDraft>,
    read_revision: Option<u64>,
}
struct ModelRequest {
    command: String,
    key: (String, String),
    draft: CreateDraft,
    provider: String,
    reply: Option<ReplyDraft>,
}
#[derive(Clone)]
struct MissionControl {
    assignments: HashMap<String, String>,
    budgets: Value,
    instruction: String,
    tasks: HashSet<String>,
    recipe_name: String,
}
impl Default for MissionControl {
    fn default() -> Self {
        Self {
            assignments: HashMap::new(),
            budgets: json!({"maxJobs":12,"maxElapsedMinutes":60,"maxReviewRounds":1}),
            instruction: String::new(),
            tasks: HashSet::new(),
            recipe_name: String::new(),
        }
    }
}
#[derive(Default)]
struct WorkflowState {
    open: bool,
    tab: usize,
    provider: String,
    request: Option<ModelRequest>,
    models: HashMap<(String, String), ModelPlan>,
    controls: HashMap<(String, String), MissionControl>,
    read: Option<(String, u64)>,
    section: String,
    offset: usize,
    recipe: HashMap<String, String>,
    recipe_goal: HashMap<(String, String), String>,
    schedule_provider: String,
    schedule_interval: u32,
    schedule_count: u32,
    first_run: String,
    schedule_budgets: Value,
}
fn workflow_values<'a>(snapshot: &'a Value, project: &str, section: &str) -> Vec<&'a Value> {
    let page = &snapshot["projectWorkflowRead"];
    if text(page, "reasonCode") == "PROJECT_WORKFLOWS_READ"
        && text(page, "projectId") == project
        && text(page, "section") == section
    {
        return array(&page["items"])
            .iter()
            .filter(|item| {
                text(item, "projectId") == project
                    || (section == "recipes" && item["projectId"].is_null())
            })
            .collect();
    }
    let source = if text(page, "reasonCode") == "PROJECT_WORKFLOWS_READ"
        && text(page, "projectId") == project
        && page[section].is_array()
    {
        page
    } else {
        &snapshot["projectWorkflows"]
    };
    array(&source[section])
        .iter()
        .filter(|item| {
            text(item, "projectId") == project
                || (section == "recipes" && item["projectId"].is_null())
        })
        .collect()
}
pub(super) fn provider_picker(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
) {
    connection_picker(ui, id, value, false);
}

pub(super) fn planning_picker(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
) -> bool {
    connection_picker(ui, id, value, true)
}

pub(super) fn planning_options(ui: &mut Ui, value: &mut String) -> bool {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = studio::metrics::GAP * 0.5;
        let mut chosen = false;
        for (id, label) in [
            ("", "Automatic"),
            ("codex", "Codex"),
            ("claude", "Claude Code"),
            ("gemini", "Gemini CLI"),
        ] {
            chosen |= provider_option(ui, value, id, t(ui, label));
        }
        chosen
    })
    .inner
}

fn provider_option(
    ui: &mut Ui,
    value: &mut String,
    id: &str,
    label: impl Into<egui::WidgetText>,
) -> bool {
    let response = ui.selectable_value(value, id.into(), label);
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("chat-provider-option", id)),
            (response.rect, response.id),
        )
    });
    response.clicked()
}

fn connection_picker(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    automatic: bool,
) -> bool {
    if value.is_empty() && !automatic {
        *value = "codex".into();
    }
    let selected = t(
        ui,
        if automatic && value.is_empty() {
            "Automatic"
        } else {
            connect::provider_name(value)
        },
    )
    .into_owned();
    let mut chosen = false;
    let picker = form_combo(
        ui,
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected)
            .width(125.0),
        |ui| {
            if automatic {
                chosen |= provider_option(ui, value, "", t(ui, "Automatic"));
            }
            for (id, label) in [
                ("codex", "Codex"),
                ("claude", "Claude Code"),
                ("gemini", "Gemini CLI"),
            ] {
                chosen |= provider_option(ui, value, id, label);
            }
        },
    );
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("test-action", "memory_control", "Choose provider")),
            picker.response.rect,
        )
    });
    #[cfg(not(test))]
    let _ = picker;
    chosen
}
fn budgets_ui(ui: &mut Ui, budgets: &mut Value) {
    for (key, label, min, max) in [
        ("maxJobs", "Maximum jobs", 1, 36),
        ("maxElapsedMinutes", "Maximum elapsed minutes", 1, 240),
        ("maxReviewRounds", "Correction rounds", 0, 2),
    ] {
        let mut value = budgets[key].as_u64().unwrap_or(min) as u32;
        ui.horizontal_wrapped(|ui| {
            ui.label(t(ui, label));
            ui.add(egui::DragValue::new(&mut value).range(min as u32..=max as u32));
        });
        budgets[key] = json!(value);
    }
}
impl CreateState {
    fn sync_workflow(&mut self, snapshot: &Value) {
        if let Some(request) = &self.crew.workflow.request {
            let receipt = &snapshot["projectWorkflowReceipt"];
            if text(receipt, "commandId") == request.command
                && text(receipt, "projectId") == request.key.0
            {
                if text(receipt, "outcome") == "applied" && !text(receipt, "planJobId").is_empty() {
                    self.crew.workflow.models.insert(request.key.clone(),ModelPlan{job:json!({"projectId":request.key.0,"planJobId":receipt["planJobId"],"status":"queued","revision":receipt["revision"],"providerId":request.provider}),draft:request.draft.clone(),answers:HashMap::new(),assignments:HashMap::new(),reply:request.reply.clone(),read_revision:None});
                }
                self.crew.workflow.request = None;
            }
        }
        let full = &snapshot["projectCrewPlanJob"];
        if text(full, "reasonCode") != "PROJECT_CREW_PLAN_READ" {
            return;
        }
        for (key, model) in &mut self.crew.workflow.models {
            if text(full, "projectId") == key.0
                && full["planJobId"] == model.job["planJobId"]
                && revision(full) >= revision(&model.job)
            {
                model.job = full.clone();
                if model.draft.goal.is_empty() {
                    model.draft = CreateDraft {
                        title: text(full, "title").into(),
                        goal: text(full, "instruction").into(),
                    };
                    if let Some(parent) = full["replyToWorkItemId"].as_str() {
                        model.reply = Some(ReplyDraft {
                            parent: parent.into(),
                            references: array(&full["referencePaths"])
                                .iter()
                                .filter_map(|path| path.as_str().map(str::to_owned))
                                .collect(),
                        });
                    }
                }

                if text(full, "status") == "completed" && full["plan"].is_object() {
                    self.crew
                        .plans
                        .entry(key.clone())
                        .or_insert_with(|| CrewDraft {
                            recommended_for: Some(model.draft.goal.clone()),
                            ability: text(&full["plan"], "abilityId").into(),
                            name: draft_title(&CreateDraft {
                                title: String::new(),
                                goal: model.draft.goal.clone(),
                            })
                            .unwrap_or_default(),
                            members: array(&full["plan"]["members"])
                                .iter()
                                .enumerate()
                                .map(|(index, member)| {
                                    let mut member = member.clone();
                                    member["_uiId"] =
                                        json!(format!("{}-{index}", text(full, "planJobId")));
                                    member
                                })
                                .collect(),
                            settings: full["plan"]["settings"].clone(),
                            crew_id: None,
                        });
                }
            }
        }
    }
}
impl Console {
    fn begin_model_plan(
        &mut self,
        snapshot: &Value,
        commands: &mut Vec<Command>,
        answers: Option<Value>,
    ) {
        if !commands.is_empty() {
            return;
        }
        let key = self.create.crew_key();
        let draft = if answers.is_some() {
            self.create
                .crew
                .workflow
                .models
                .get(&key)
                .map(|model| model.draft.clone())
                .unwrap_or_else(|| self.create.draft().clone())
        } else {
            self.create.draft().clone()
        };
        let reply = self.create.chat.replies.get(&key).cloned();
        let Some(provider) = self
            .create
            .resolved_planner_provider(snapshot)
            .map(str::to_owned)
        else {
            return;
        };
        self.begin_conversation_plan(
            snapshot,
            commands,
            draft,
            reply,
            provider,
            answers
                .unwrap_or_else(|| json!([]))
                .as_array()
                .cloned()
                .unwrap_or_default(),
        );
    }
    pub(super) fn begin_conversation_plan(
        &mut self,
        snapshot: &Value,
        commands: &mut Vec<Command>,
        draft: CreateDraft,
        reply: Option<ReplyDraft>,
        provider: String,
        answers: Vec<Value>,
    ) {
        if !commands.is_empty() {
            return;
        }
        let key = self.create.crew_key();
        let Some(title) = draft_title(&draft) else {
            return;
        };
        let Some(project_rev) = output_studio::project_revision(snapshot, &self.create.project)
        else {
            return;
        };
        if !valid_input(&draft.goal, 20_000) {
            return;
        }
        let mut payload = json!({"projectId":self.create.project,"expectedProjectRevision":project_rev,"providerId":provider,"title":title,"instruction":draft.goal});
        if !self.create.ability.is_empty() {
            payload["abilityId"] = json!(self.create.ability);
        }
        if let Some(reply) = &reply {
            let mut paths: Vec<_> = reply.references.iter().cloned().collect();
            paths.sort();
            payload["replyToWorkItemId"] = json!(reply.parent);
            payload["referencePaths"] = json!(paths);
        }
        if !answers.is_empty() {
            payload["answers"] = json!(answers);
        }
        let command = Command::new("project_crew_plan_start", payload);
        self.create.crew.workflow.request = Some(ModelRequest {
            command: text(&command.payload, "commandId").into(),
            key: key.clone(),
            draft,
            provider,
            reply,
        });
        self.create.crew.plans.remove(&key);
        self.create.crew.workflow.models.remove(&key);
        commands.push(command);
    }
    fn model_plan_card(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let key = self.create.crew_key();
        let Some(model) = self.create.crew.workflow.models.get(&key).cloned() else {
            return;
        };
        let job = &model.job;
        let latest = workflow_values(snapshot, &key.0, "plans")
            .into_iter()
            .find(|plan| plan["planJobId"] == job["planJobId"])
            .map(revision)
            .unwrap_or(revision(job));
        if !self.hold_automatic_reads
            && ui.is_enabled()
            && model.read_revision != Some(latest)
            && studio::commands_ready(ui)
            && commands.is_empty()
        {
            self.create
                .crew
                .workflow
                .models
                .get_mut(&key)
                .unwrap()
                .read_revision = Some(latest);
            commands.push(Command::new(
                "project_crew_plan_read",
                json!({"projectId":key.0,"planJobId":job["planJobId"]}),
            ));
        }
        if self.create.has_conversation() && !self.create.crew.workflow.open {
            if text(job, "status") == "completed" {
                self.model_use_goal(ui, &key, &model);
            } else {
                studio::card(ui).show(ui,|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(t(ui,"Mox planner")).strong());
                        status(ui,text(job,"status"));
                    });
                    for question in array(&job["questions"]) {
                        ui.add(egui::Label::new(text(question,"question")).wrap().selectable(true));
                    }
                    if !text(job,"failureReasonCode").is_empty() {hint(ui,text(job,"failureReasonCode"));}
                    ui.horizontal_wrapped(|ui| {
                        quiet_action(ui,commands,"Refresh plan","project_crew_plan_read",true,||json!({"projectId":key.0,"planJobId":job["planJobId"]}));
                        if matches!(text(job,"status"),"queued"|"running") {
                            quiet_action(ui,commands,"Cancel planning","project_crew_plan_cancel",true,||json!({"projectId":key.0,"planJobId":job["planJobId"],"expectedPlanRevision":job["revision"]}));
                        }
                    });
                });
            }
            return;
        }
        if text(job, "status") == "completed" {
            self.model_use_goal(ui, &key, &model);
            if !self.create.crew.workflow.open {
                ui.collapsing(t(ui, "Planning details"), |ui| {
                    self.model_plan_details(ui, commands, &key, &model)
                });
            }
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(studio::title(
                t(
                    ui,
                    if text(job, "status") == "needs_input" {
                        "A few details first"
                    } else {
                        "Mox planner"
                    },
                ),
                20.0,
            ));
            status(ui, text(job, "status"));
        });
        if !text(job, "failureReasonCode").is_empty() {
            hint(ui, text(job, "failureReasonCode"));
        }
        self.model_use_goal(ui, &key, &model);
        if text(job, "status") == "needs_input" {
            let state = self.create.crew.workflow.models.get_mut(&key).unwrap();
            for question in array(&job["questions"]) {
                field(
                    ui,
                    text(question, "question"),
                    state
                        .answers
                        .entry(text(question, "id").into())
                        .or_default(),
                    2000,
                    false,
                );
            }
            if !self.create.crew.workflow.open {
                self.model_answer_action(ui, snapshot, commands);
            }
        }
        ui.horizontal_wrapped(|ui| {
            quiet_action(ui, commands, "Refresh plan", "project_crew_plan_read", true, || json!({"projectId":key.0,"planJobId":job["planJobId"]}));
            if matches!(text(job, "status"), "queued" | "running") {
                quiet_action(ui, commands, "Cancel planning", "project_crew_plan_cancel", true, || json!({"projectId":key.0,"planJobId":job["planJobId"],"expectedPlanRevision":job["revision"]}));
            }
        });
    }
    fn model_plan_details(
        &mut self,
        ui: &mut Ui,
        commands: &mut Vec<Command>,
        key: &(String, String),
        model: &ModelPlan,
    ) {
        let job = &model.job;
        raw_hint(ui, text(job, "providerId"));
        ui.add(egui::Label::new(&model.draft.goal).wrap().selectable(true));
        ui.add_space(studio::metrics::GAP);
        ui.label(RichText::new(t(ui, "Agent providers")).size(14.0).strong());
        for (index, member) in self
            .create
            .crew
            .plans
            .get(key)
            .map(|plan| plan.members.clone())
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            ui.horizontal_wrapped(|ui| {
                ui.label(text(member, "name"));
                provider_picker(
                    ui,
                    ("plan-agent-provider", index),
                    self.create
                        .crew
                        .workflow
                        .models
                        .get_mut(key)
                        .unwrap()
                        .assignments
                        .entry(text(member, "_uiId").into())
                        .or_insert_with(|| text(job, "providerId").into()),
                );
            });
        }
        quiet_action(
            ui,
            commands,
            "Refresh plan",
            "project_crew_plan_read",
            true,
            || json!({"projectId":key.0,"planJobId":job["planJobId"]}),
        );
    }
    fn model_use_goal(&mut self, ui: &mut Ui, key: &(String, String), model: &ModelPlan) {
        if self.create.draft().goal != model.draft.goal
            && view_control(ui, "Use planned goal", !model.draft.goal.is_empty(), false).clicked()
        {
            *self.create.draft() = model.draft.clone();
            if let Some(reply) = &model.reply {
                self.create.chat.replies.insert(key.clone(), reply.clone());
            } else {
                self.create.chat.replies.remove(key);
            }
        }
    }
    fn model_answer_action(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let Some(model) = self
            .create
            .crew
            .workflow
            .models
            .get(&self.create.crew_key())
        else {
            return;
        };
        let questions = array(&model.job["questions"]);
        let answers: Vec<_> = questions
            .iter()
            .filter_map(|question| {
                model
                    .answers
                    .get(text(question, "id"))
                    .map(|answer| json!({"id":question["id"],"answer":answer.trim()}))
            })
            .collect();
        let enabled = self.create.resolved_planner_provider(snapshot).is_some()
            && !answers.is_empty()
            && answers.len() == questions.len()
            && answers
                .iter()
                .all(|answer| valid_input(text(answer, "answer"), 2000));
        if view_control(
            ui,
            "Send answers & replan",
            enabled && studio::commands_ready(ui),
            true,
        )
        .clicked()
        {
            self.begin_model_plan(snapshot, commands, Some(json!(answers)));
        }
    }
    pub(super) fn workflow_dialog(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if !self.create.crew.workflow.open {
            return;
        }
        let project = self.create.project.clone();
        if output_studio::project_revision(snapshot, &project).is_none() {
            self.create.crew.workflow.open = false;
            return;
        }
        let read_key = (project.clone(), revision(&snapshot["projectWorkflows"]));
        if !self.hold_automatic_reads
            && ui.is_enabled()
            && self.create.crew.workflow.read.as_ref() != Some(&read_key)
            && studio::commands_ready(ui)
            && commands.is_empty()
        {
            self.create.crew.workflow.read = Some(read_key);
            commands.push(Command::new(
                "project_workflows_read",
                json!({"projectId":project}),
            ));
        }
        inline_surface(ui, |ui| {
            let compact = ui.ctx().viewport_rect().height() < 700.0;
            let key = self.create.crew_key();
            let has_plan = self.create.crew.plans.contains_key(&key);
            let labels = ["Plan", "Direction", "Recipes", "Schedules", "Activity"];
            let heading = if self.create.crew.workflow.tab == 0 {
                if has_plan {
                    "Your crew is ready"
                } else {
                    "Build your crew"
                }
            } else {
                labels[self.create.crew.workflow.tab.min(4)]
            };
            ui.horizontal(|ui| {
                let title = ui.label(studio::title(t(ui, heading), 20.0));
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new("workflow-heading"),
                        (title.rect, ui.clip_rect()),
                    )
                });
                #[cfg(not(test))]
                let _ = title;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let picker = egui::ComboBox::from_id_salt("crew-context-view")
                        .height(260.0)
                        .width(if compact { 128.0 } else { 148.0 })
                        .selected_text(t(ui, labels[self.create.crew.workflow.tab.min(4)]))
                        .show_ui(ui, |ui| {
                            for (index, label) in labels.iter().enumerate() {
                                let response = ui.selectable_value(
                                    &mut self.create.crew.workflow.tab,
                                    index,
                                    t(ui, label),
                                );
                                #[cfg(test)]
                                ui.data_mut(|data| {
                                    data.insert_temp(
                                        egui::Id::new(("workflow-view-tab", index)),
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
                            egui::Id::new(("test-action", "memory_control", "Crew view")),
                            picker.response.rect,
                        )
                    });
                    #[cfg(not(test))]
                    let _ = picker;
                    quiet_action(
                        ui,
                        commands,
                        "Refresh",
                        "project_workflows_read",
                        !self.hold_automatic_reads,
                        || json!({"projectId":project}),
                    );
                });
            });
            ui.add_space(if compact {
                f32::from(studio::metrics::PADDING)
            } else {
                studio::metrics::SECTION_GAP
            });
            let reserve = match self.create.crew.workflow.tab {
                0 => {
                    studio::metrics::CONTROL_HEIGHT
                        + f32::from(studio::metrics::PADDING) * 2.0
                        + studio::metrics::SECTION_GAP
                }
                1 | 2 => {
                    studio::metrics::CONTROL_HEIGHT
                        + studio::metrics::CONTROL_GAP
                        + studio::metrics::GAP
                }
                _ => 0.0,
            };
            egui::ScrollArea::vertical().id_salt(("mission-workflow-body",&project,self.create.crew.workflow.tab)).auto_shrink([false,self.create.crew.workflow.tab==0]).max_height((ui.available_height()-reserve).max(60.0)).show(ui,|ui| {
                match self.create.crew.workflow.tab {
                    0=>{
                        let ready=self.create.crew.plans.contains_key(&key);
                        if ready{self.model_plan_card(ui,snapshot,commands);self.crew_roster_summary(ui,snapshot,false);}
                        else if self.create.crew.workflow.models.contains_key(&key){self.model_plan_card(ui,snapshot,commands);}
                        else{self.workflow_plan_request(ui,snapshot);}
                        ui.add_space(f32::from(studio::metrics::PADDING));
                        let details=ui.collapsing(t(ui,"Plan details"),|ui| {
                            if let Some(plan)=self.create.crew.plans.get(&key).cloned(){
                                ui.label(RichText::new(t(ui,"Agent instructions")).strong());
                                for member in &plan.members{ui.label(RichText::new(text(member,"name")).strong());ui.add(egui::Label::new(text(member,"instruction")).wrap().selectable(true));ui.add_space(studio::metrics::GAP);}
                            }
                            if let Some(model)=self.create.crew.workflow.models.get(&key).cloned().filter(|model|text(&model.job,"status")=="completed"){self.model_plan_details(ui,commands,&key,&model);}
                            if ready{
                                ui.add_space(f32::from(studio::metrics::PADDING));self.workflow_plan_request(ui,snapshot);
                                let enabled=valid_input(&self.create.draft().goal,20_000)&&self.create.crew.workflow.request.is_none()&&self.create.resolved_planner_provider(snapshot).is_some();
                                if view_control(ui,"Generate plan",enabled && studio::commands_ready(ui),false).clicked(){self.begin_model_plan(snapshot,commands,None);}
                            }
                            ui.add_space(f32::from(studio::metrics::PADDING));ui.label(RichText::new(t(ui,"Saved plans")).strong());
                            for saved in workflow_values(snapshot,&project,"plans"){
                                ui.push_id(text(saved,"planJobId"),|ui|ui.horizontal_wrapped(|ui|{
                                    ui.label(text(saved,"title"));status(ui,text(saved,"status"));
                                    if view_control(ui,"Open plan",studio::commands_ready(ui),false).clicked()&&studio::commands_ready(ui)&&commands.is_empty(){
                                        let key=self.create.crew_key();self.create.crew.plans.remove(&key);
                                        self.create.crew.workflow.models.insert(key,ModelPlan{job:saved.clone(),draft:CreateDraft::default(),reply:None,answers:HashMap::new(),assignments:HashMap::new(),read_revision:None});
                                        commands.push(Command::new("project_crew_plan_read",json!({"projectId":project,"planJobId":saved["planJobId"]})));
                                    }
                                }));
                            }
                            self.workflow_page_controls(ui,snapshot,commands,"plans");
                        });
                        #[cfg(test)] ui.data_mut(|data|data.insert_temp(egui::Id::new(("test-action","memory_control","Plan details")),details.header_response.rect));
                        #[cfg(not(test))] let _=details;
                    },
                    1=>self.workflow_direction(ui,snapshot,commands),2=>self.workflow_recipes(ui,snapshot,commands),3=>self.workflow_schedules(ui,snapshot,commands),_=>self.workflow_activity(ui,snapshot,commands),
                }
                let error=&snapshot["projectWorkflowError"];if text(error,"projectId")==project{hint(ui,text(error,"reasonCode"));}
                let receipt=&snapshot["projectWorkflowReceipt"];if text(receipt,"projectId")==project&&!text(receipt,"reasonCode").is_empty(){hint(ui,text(receipt,"reasonCode"));}
            });
            ui.add_space(studio::metrics::CONTROL_GAP);
            if self.create.crew.workflow.tab == 0 {
                let preparation = egui::Frame::NONE.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.create.crew.plans.contains_key(&key) {
                                self.crew_prepare_action(ui, snapshot, commands);
                                if view_control(ui, "Edit roster", true, false).clicked() {
                                    self.create.crew.roster_open = true;
                                }
                            } else if self
                                .create
                                .crew
                                .workflow
                                .models
                                .get(&key)
                                .is_some_and(|model| text(&model.job, "status") == "needs_input")
                            {
                                self.model_answer_action(ui, snapshot, commands);
                            } else if !self.create.crew.workflow.models.get(&key).is_some_and(
                                |model| matches!(text(&model.job, "status"), "queued" | "running"),
                            ) {
                                let enabled = valid_input(&self.create.draft().goal, 20_000)
                                    && self.create.crew.workflow.request.is_none()
                                    && self.create.resolved_planner_provider(snapshot).is_some();
                                if view_control(
                                    ui,
                                    "Generate plan",
                                    enabled && studio::commands_ready(ui),
                                    true,
                                )
                                .clicked()
                                {
                                    self.begin_model_plan(snapshot, commands, None);
                                }
                            }
                        });
                    });
                    let copy = if self.create.crew.pending.is_some() || !studio::commands_ready(ui)
                    {
                        "Applying action…"
                    } else if self.create.crew.workflow.request.is_some() {
                        "Waiting for planner confirmation…"
                    } else if self.create.crew.plans.contains_key(&key)
                        && self.create.crew_payload(snapshot).is_none()
                    {
                        "Review the goal and roster before preparing."
                    } else {
                        "Prepare now. Start the crew when you’re ready."
                    };
                    let guidance = ui.add(
                        egui::Label::new(
                            RichText::new(t(ui, copy))
                                .size(12.0)
                                .color(colors(ui).muted),
                        )
                        .wrap(),
                    );
                    #[cfg(test)]
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new("workflow-footer-guidance"),
                            (guidance.rect, ui.clip_rect()),
                        )
                    });
                    #[cfg(not(test))]
                    let _ = guidance;
                });
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new("crew-preparation-bar"),
                        (preparation.response.rect, ui.clip_rect()),
                    )
                });
                #[cfg(not(test))]
                let _ = preparation;
            } else if self.create.crew.workflow.tab == 1 {
                self.direction_action(ui, snapshot, commands);
            } else if self.create.crew.workflow.tab == 2 {
                self.recipe_action(ui, snapshot, commands);
            }
        });
    }
    fn workflow_plan_request(&mut self, ui: &mut Ui, snapshot: &Value) {
        ui.label(studio::title(t(ui, "Plan your next goal"), 20.0));
        ui.add_space(studio::metrics::GAP);
        multiline(ui, "Goal", &mut self.create.draft().goal, 20_000, 3);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(t(ui, "Planning provider"))
                    .size(12.0)
                    .color(colors(ui).muted),
            );
            planning_picker(
                ui,
                "model-planner-provider",
                &mut self.create.crew.workflow.provider,
            );
        });
        hint(
            ui,
            "Planning calls the selected provider and can consume your account quota.",
        );
        if self.create.resolved_planner_provider(snapshot).is_none() {
            hint(ui, "Check the assigned providers before starting.");
            if view_control(ui, "Connections", true, false).clicked() {
                self.create.chat.connections_open = true;
            }
        }
    }
    fn workflow_direction(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let Some(mission) = self.create.crew_mission(snapshot).cloned().or_else(|| {
            ordered_missions(snapshot, &self.create.project)
                .into_iter()
                .next_back()
                .cloned()
        }) else {
            hint(ui, "Prepare a crew to configure providers and direction.");
            return;
        };
        let project = self.create.project.clone();
        let id = text(&mission, "missionId").to_owned();
        ui.label(RichText::new(text(&mission, "title")).strong());
        status(ui, text(&mission, "status"));
        let key = (project.clone(), id.clone());
        let control_state = self
            .create
            .crew
            .workflow
            .controls
            .entry(key)
            .or_insert_with(|| {
                let mut state = MissionControl::default();
                if mission["workflow"]["budgets"].is_object() {
                    state.budgets = mission["workflow"]["budgets"].clone();
                }
                for assignment in array(&mission["workflow"]["providerAssignments"]) {
                    state.assignments.insert(
                        text(assignment, "agentId").into(),
                        text(assignment, "providerId").into(),
                    );
                }
                state
            });
        let compact = ui.ctx().viewport_rect().height() < 700.0;
        let input = ui.scope(|ui| {
            multiline(
                ui,
                "New direction",
                &mut control_state.instruction,
                20_000,
                2,
            )
        });
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("workflow-direction-input"),
                (input.response.rect, ui.clip_rect()),
            )
        });
        #[cfg(not(test))]
        let _ = input;
        let warning=ui.add(egui::Label::new(RichText::new(t(ui,"Choose affected tasks. Direction stops and replaces their unfinished work and descendants; earlier files remain.")).size(12.0).color(colors(ui).muted)).wrap());
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("workflow-direction-warning"),
                (warning.rect, ui.clip_rect()),
            )
        });
        #[cfg(not(test))]
        let _ = warning;
        let affected = egui::CollapsingHeader::new(tf(
            ui,
            "Affected tasks · {0} selected",
            &[&control_state.tasks.len().to_string()],
        ))
        .id_salt(("crew-affected-tasks", &project, &id))
        .default_open(!compact)
        .show(ui, |ui| {
            for task in array(&mission["tasks"]) {
                let task_id = text(task, "taskId").to_owned();
                let mut selected = control_state.tasks.contains(&task_id);
                let label = if text(task, "title").is_empty() {
                    text(task, "taskId")
                } else {
                    text(task, "title")
                };
                let checkbox = ui.checkbox(&mut selected, label);
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("test-action", "affected-task", task_id.as_str())),
                        checkbox.rect,
                    )
                });
                if checkbox.changed() {
                    if selected {
                        control_state.tasks.insert(task_id);
                    } else {
                        control_state.tasks.remove(&task_id);
                    }
                }
            }
        });
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("test-action", "memory_control", "Affected tasks")),
                affected.header_response.rect,
            )
        });
        #[cfg(not(test))]
        let _ = affected;
        if !text(&mission["workflow"], "directionStatus").is_empty() {
            status(ui, text(&mission["workflow"], "directionStatus"));
        }
        if let Some(instruction) = mission["workflow"]["currentInstruction"].as_str() {
            ui.collapsing(t(ui, "Current direction"), |ui| {
                ui.add(egui::Label::new(instruction).wrap().selectable(true));
            });
        }
        ui.add_space(studio::metrics::SECTION_GAP);
        ui.collapsing(t(ui,"Providers & limits"), |ui| {
        if let Some(crew) = crews(snapshot)
            .iter()
            .find(|crew| crew["crewId"] == mission["crewId"] && text(crew, "projectId") == project)
        {
            for member in array(&crew["members"]) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(text(member, "name"));
                    provider_picker(
                        ui,
                        ("mission-agent-provider", text(member, "agentId")),
                        control_state
                            .assignments
                            .entry(text(member, "agentId").into())
                            .or_insert_with(|| "codex".into()),
                    );
                });
            }
        }
        budgets_ui(ui, &mut control_state.budgets);
        let mut assignments: Vec<_> = control_state
            .assignments
            .iter()
            .map(|(agent, provider)| json!({"agentId":agent,"providerId":provider}))
            .collect();
        assignments.sort_by(|a, b| text(a, "agentId").cmp(text(b, "agentId")));
        action(
            ui,
            commands,
            "Save providers & limits",
            "project_crew_configure",
            text(&mission, "status") == "prepared" && !assignments.is_empty(),
            || json!({"projectId":project,"missionId":id,"expectedMissionRevision":mission["revision"],"providerAssignments":assignments,"budgets":control_state.budgets}),
        );
        });
    }
    fn direction_action(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let Some(mission) = self.create.crew_mission(snapshot).or_else(|| {
            ordered_missions(snapshot, &self.create.project)
                .into_iter()
                .next_back()
        }) else {
            return;
        };
        let key = (
            self.create.project.clone(),
            text(mission, "missionId").into(),
        );
        let Some(state) = self.create.crew.workflow.controls.get(&key) else {
            return;
        };
        let mut affected: Vec<_> = state.tasks.iter().cloned().collect();
        affected.sort();
        let enabled = text(mission, "status") == "active"
            && valid_input(&state.instruction, 20_000)
            && !affected.is_empty();
        let response = view_control(
            ui,
            "Apply direction",
            enabled && studio::commands_ready(ui),
            true,
        );
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("test-action", "project_crew_steer", "Apply direction")),
                response.rect,
            )
        });
        if response.clicked() && studio::commands_ready(ui) && commands.is_empty() {
            commands.push(Command::new("project_crew_steer",json!({"projectId":self.create.project,"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"],"instruction":state.instruction.trim(),"affectedTaskIds":affected})));
        }
    }
    fn workflow_recipes(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let project = self.create.project.clone();
        let recipes = workflow_values(snapshot, &project, "recipes");
        let selected = self
            .create
            .crew
            .workflow
            .recipe
            .entry(project.clone())
            .or_default();
        if selected.is_empty() {
            if let Some(recipe) = recipes.first() {
                *selected = text(recipe, "recipeId").into();
            }
        }
        form_combo(
            ui,
            egui::ComboBox::from_id_salt("crew-recipe-picker")
                .width(ui.available_width().min(440.0))
                .selected_text(
                    recipes
                        .iter()
                        .find(|recipe| text(recipe, "recipeId") == selected)
                        .map(|recipe| std::borrow::Cow::Borrowed(text(recipe, "name")))
                        .unwrap_or_else(|| t(ui, "Choose recipe")),
                ),
            |ui| {
                for recipe in &recipes {
                    ui.selectable_value(
                        selected,
                        text(recipe, "recipeId").into(),
                        text(recipe, "name"),
                    );
                }
            },
        );
        if let Some(recipe) = recipes
            .iter()
            .find(|recipe| text(recipe, "recipeId") == selected)
        {
            let compact = ui.ctx().viewport_rect().height() < 700.0;
            ui.add_space(if compact { 8.0 } else { 16.0 });
            if !compact {
                ui.label(studio::title(text(recipe, "name"), 20.0));
                ui.label(text(recipe, "description"));
                ui.add_space(studio::metrics::CONTROL_GAP);
            }
            let input = ui.scope(|ui| {
                multiline(
                    ui,
                    "New goal (optional)",
                    self.create
                        .crew
                        .workflow
                        .recipe_goal
                        .entry((project.clone(), text(recipe, "recipeId").into()))
                        .or_default(),
                    20_000,
                    2,
                )
            });
            #[cfg(test)]
            ui.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("workflow-recipe-input"),
                    (input.response.rect, ui.clip_rect()),
                )
            });
            #[cfg(not(test))]
            let _ = input;
            if compact {
                ui.add(
                    egui::Label::new(
                        RichText::new(text(recipe, "description"))
                            .size(12.0)
                            .color(colors(ui).muted),
                    )
                    .wrap(),
                );
            }
            ui.collapsing(t(ui, "Roster"), |ui| {
                for member in array(&recipe["members"]) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(text(member, "name"));
                        hint(ui, role_label(text(member, "role")));
                    });
                }
                for (key, label) in [
                    ("deliverables", "Deliverables"),
                    ("toolRequirements", "Required tools"),
                ] {
                    if !array(&recipe[key]).is_empty() {
                        ui.label(RichText::new(t(ui, label)).strong());
                        for value in array(&recipe[key]) {
                            if let Some(value) = value.as_str() {
                                ui.label(value);
                            } else {
                                ui.label(value.to_string());
                            }
                        }
                    }
                }
            });
        }
        ui.add_space(studio::metrics::CONTROL_GAP);
        self.workflow_page_controls(ui, snapshot, commands, "recipes");
        if let Some(mission) = self.create.crew_mission(snapshot).cloned().or_else(|| {
            ordered_missions(snapshot, &project)
                .into_iter()
                .next_back()
                .cloned()
        }) {
            ui.collapsing(t(ui,"Save current crew as recipe"),|ui| {
                let state=self.create.crew.workflow.controls.entry((project.clone(),text(&mission,"missionId").into())).or_default();
                field(ui,"Recipe name",&mut state.recipe_name,80,false);
                action(ui,commands,"Save mission as recipe","project_recipe_save",valid_input(&state.recipe_name,80),||json!({"projectId":project,"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"],"name":state.recipe_name.trim()}));
            });
        }
    }
    fn recipe_action(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let project = self.create.project.clone();
        let Some(recipe) = workflow_values(snapshot, &project, "recipes")
            .into_iter()
            .find(|recipe| {
                Some(text(recipe, "recipeId"))
                    == self
                        .create
                        .crew
                        .workflow
                        .recipe
                        .get(&project)
                        .map(String::as_str)
            })
        else {
            return;
        };
        let goal = self
            .create
            .crew
            .workflow
            .recipe_goal
            .get(&(project.clone(), text(recipe, "recipeId").into()))
            .map(String::as_str)
            .unwrap_or("");
        let mut payload = json!({"projectId":project,"expectedProjectRevision":output_studio::project_revision(snapshot,&project),"recipeId":recipe["recipeId"],"expectedRecipeRevision":recipe["revision"]});
        if !goal.trim().is_empty() {
            payload["instruction"] = json!(goal.trim());
        }
        ui.horizontal_wrapped(|ui| {
            let response = view_control(
                ui,
                "Prepare recipe",
                revision(recipe) > 0 && studio::commands_ready(ui),
                true,
            );
            #[cfg(test)]
            ui.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("test-action", "project_recipe_run", "Prepare recipe")),
                    response.rect,
                )
            });
            if response.clicked() && studio::commands_ready(ui) && commands.is_empty() {
                commands.push(Command::new("project_recipe_run", payload));
            }
            if view_control(ui, "Schedule recipe", revision(recipe) > 0, false).clicked() {
                self.create.crew.workflow.tab = 3;
            }
        });
    }
    fn workflow_page_controls(
        &self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
        section: &str,
    ) {
        let page = &snapshot["projectWorkflowRead"];
        let same =
            text(page, "projectId") == self.create.project && text(page, "section") == section;
        let offset = if same {
            page["offset"].as_u64().unwrap_or(0)
        } else {
            0
        };
        let values = workflow_values(snapshot, &self.create.project, section);
        ui.push_id(("workflow-pages",section),|ui|ui.horizontal_wrapped(|ui| {
            if control(ui,"Previous",studio::commands_ready(ui) && offset>=20).clicked()&&studio::commands_ready(ui)&&commands.is_empty(){commands.push(Command::new("project_workflows_read",json!({"projectId":self.create.project,"section":section,"offset":offset.saturating_sub(20),"limit":20})));}
            if control(ui,"Next",studio::commands_ready(ui) && if same{page["nextOffset"].is_u64()}else{values.len()>=20}).clicked()&&studio::commands_ready(ui)&&commands.is_empty(){commands.push(Command::new("project_workflows_read",json!({"projectId":self.create.project,"section":section,"offset":page["nextOffset"].as_u64().filter(|_|same).unwrap_or(offset+20),"limit":20})));}
        }));
    }
    fn workflow_schedules(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let project = self.create.project.clone();
        hint(ui,"Schedules run only while Moxcrew is awake. Missed intervals are skipped without a catch-up storm.");
        let recipes = workflow_values(snapshot, &project, "recipes");
        let state = &mut self.create.crew.workflow;
        let selected = state.recipe.entry(project.clone()).or_default();
        form_combo(
            ui,
            egui::ComboBox::from_id_salt("schedule-recipe").selected_text(
                recipes
                    .iter()
                    .find(|recipe| text(recipe, "recipeId") == selected)
                    .map(|recipe| text(recipe, "name"))
                    .map(std::borrow::Cow::Borrowed)
                    .unwrap_or_else(|| t(ui, "Choose recipe")),
            ),
            |ui| {
                for recipe in &recipes {
                    ui.selectable_value(
                        selected,
                        text(recipe, "recipeId").into(),
                        text(recipe, "name"),
                    );
                }
            },
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(t(ui, "Provider"));
            provider_picker(ui, "schedule-provider", &mut state.schedule_provider);
        });
        let provider_ready = connect::can_attempt_provider(snapshot, &state.schedule_provider);
        if provider_ready {
            hint(
                ui,
                "Scheduled runs will use this provider while Moxcrew is awake.",
            );
        } else {
            hint(
                ui,
                connect::selection_status(snapshot, &state.schedule_provider),
            );
        }
        state.schedule_interval = state.schedule_interval.clamp(15, 10080);
        state.schedule_count = state.schedule_count.clamp(1, 100);
        ui.horizontal_wrapped(|ui| {
            ui.label(t(ui, "Interval (minutes)"));
            ui.add(egui::DragValue::new(&mut state.schedule_interval).range(15..=10080));
            ui.label(t(ui, "Occurrences"));
            ui.add(egui::DragValue::new(&mut state.schedule_count).range(1..=100));
        });
        field(ui, "First run (RFC 3339)", &mut state.first_run, 80, false);
        let recipe = recipes
            .iter()
            .find(|recipe| text(recipe, "recipeId") == selected);
        if !state.schedule_budgets.is_object() {
            state.schedule_budgets =
                json!({"maxJobs":12,"maxElapsedMinutes":60,"maxReviewRounds":1});
        }
        ui.collapsing(t(ui, "Mission limits"), |ui| {
            budgets_ui(ui, &mut state.schedule_budgets)
        });
        let first_run = chrono::DateTime::parse_from_rfc3339(&state.first_run)
            .ok()
            .map(|time| {
                time.with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            });
        if let Some(recipe) = recipe {
            action(
                ui,
                commands,
                "Create enabled schedule",
                "project_schedule_save",
                first_run.is_some() && provider_ready,
                || json!({"projectId":project,"expectedProjectRevision":output_studio::project_revision(snapshot,&project),"recipeId":recipe["recipeId"],"expectedRecipeRevision":recipe["revision"],"providerId":state.schedule_provider,"intervalMinutes":state.schedule_interval,"firstRunAt":first_run,"maxOccurrences":state.schedule_count,"budgets":state.schedule_budgets}),
            );
        }
        for schedule in workflow_values(snapshot, &project, "schedules") {
            ui.push_id(text(schedule,"scheduleId"),|ui|card(ui,|ui|{
                ui.label(text(schedule,"scheduleId"));hint(ui,if schedule["enabled"]==true{"Enabled"}else{"Paused"});
                ui.label(format!("{}: {}",t(ui,"Provider"),connect::provider_name(schedule["providerId"].as_str().unwrap_or("codex"))));
                ui.label(format!("{}: {}",t(ui,"Next run"),text(schedule,"nextRunAt")));
                ui.label(format!("{} / {}",schedule["occurrences"],schedule["maxOccurrences"]));
                action(ui,commands,if schedule["enabled"]==true{"Pause schedule"}else{"Resume schedule"},"project_schedule_set_enabled",revision(schedule)>0,||json!({"projectId":project,"scheduleId":schedule["scheduleId"],"expectedScheduleRevision":schedule["revision"],"enabled":schedule["enabled"]!=true}));
                if !text(schedule,"lastReasonCode").is_empty(){hint(ui,text(schedule,"lastReasonCode"));}
            }));
        }
        self.workflow_page_controls(ui, snapshot, commands, "schedules");
    }
    fn workflow_activity(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let project = self.create.project.clone();
        let state = &mut self.create.crew.workflow;
        if state.section.is_empty() {
            state.section = "usage".into();
        }
        ui.horizontal_wrapped(|ui| {
            for (key, label) in [
                ("usage", "Time & usage"),
                ("offers", "Handoffs"),
                ("reviews", "Reviews"),
                ("directions", "Directions"),
            ] {
                if studio::section_nav(ui, state.section == key, t(ui, label).as_ref()).clicked() {
                    state.section = key.into();
                    state.offset = 0;
                }
            }
        });
        let section = state.section.clone();
        let page = &snapshot["projectWorkflowRead"];
        let values = if text(page, "projectId") == project
            && text(page, "reasonCode") == "PROJECT_WORKFLOWS_READ"
            && text(page, "section") == section
        {
            array(&page["items"]).iter().collect()
        } else {
            workflow_values(snapshot, &project, &section)
        };
        if values.is_empty() {
            hint(ui, "No recorded activity in this project.");
        }
        if section == "reviews" {
            hint(ui,"Reviewer reported checks. These are the agent's report, not independent tool verification.");
        }
        if section == "offers" {
            hint(ui,"Handoffs are sent and accepted by actual project agents. Delivery lists verified output references.");
        }
        for item in values {
            card(ui, |ui| match section.as_str() {
                "usage" => {
                    let title = missions(snapshot)
                        .iter()
                        .find(|mission| {
                            text(mission, "projectId") == project
                                && mission["missionId"] == item["missionId"]
                        })
                        .map(|mission| text(mission, "title"))
                        .filter(|title| !title.is_empty());
                    ui.label(
                        RichText::new(
                            title
                                .map(std::borrow::Cow::Borrowed)
                                .unwrap_or_else(|| t(ui, "Crew")),
                        )
                        .strong(),
                    );
                    ui.collapsing(t(ui, "Details"), |ui| {
                        ui.label(text(item, "missionId"));
                    });
                    for (key, label) in [
                        ("startedJobs", "Jobs started"),
                        ("elapsedSeconds", "Elapsed seconds"),
                        ("inputTokens", "Input tokens"),
                        ("outputTokens", "Output tokens"),
                        ("costMicros", "Cost (millionths)"),
                    ] {
                        ui.label(format!(
                            "{}: {}",
                            t(ui, label),
                            item[key]
                                .as_u64()
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| t(ui, "Unknown").into_owned())
                        ));
                    }
                    if item["measurementComplete"] != true {
                        hint(
                            ui,
                            "Usage is incomplete. Unknown provider measurements are not zero.",
                        );
                    }
                    for bottleneck in array(&item["bottlenecks"]) {
                        hint(
                            ui,
                            bottleneck["reasonCode"]
                                .as_str()
                                .or_else(|| bottleneck.as_str())
                                .unwrap_or(""),
                        );
                    }
                }
                "reviews" => {
                    status(ui, text(item, "verdict"));
                    ui.label(text(item, "feedback"));
                    for check in array(&item["checks"]) {
                        ui.label(format!(
                            "{} · {}",
                            if check["passed"] == true {
                                t(ui, "Passed")
                            } else {
                                t(ui, "Failed")
                            },
                            text(check, "name")
                        ));
                        ui.label(text(check, "note"));
                    }
                }
                "offers" => {
                    ui.label(text(item, "title"));
                    status(ui, text(item, "status"));
                    ui.label(format!(
                        "{} > {}",
                        text(item, "fromAgentId"),
                        text(item, "toAgentId")
                    ));
                    for file in array(&item["files"]) {
                        ui.label(text(file, "path"));
                        ui.monospace(text(file, "sha256"));
                    }
                }
                _ => {
                    status(ui, text(item, "status"));
                    ui.label(text(item, "instruction"));
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            action(
                ui,
                commands,
                "Read activity",
                "project_workflows_read",
                true,
                || json!({"projectId":project,"section":section,"offset":state.offset,"limit":20}),
            );
            if control(ui, "Previous", state.offset >= 20).clicked()
                && studio::commands_ready(ui)
                && commands.is_empty()
            {
                state.offset = state.offset.saturating_sub(20);
                commands.push(Command::new(
                    "project_workflows_read",
                    json!({"projectId":project,"section":section,"offset":state.offset,"limit":20}),
                ));
            }
            if control(
                ui,
                "Next",
                text(page, "projectId") == project
                    && text(page, "section") == section
                    && page["nextOffset"].is_u64(),
            )
            .clicked()
                && studio::commands_ready(ui)
                && commands.is_empty()
            {
                state.offset = page["nextOffset"].as_u64().unwrap_or(0) as usize;
                commands.push(Command::new(
                    "project_workflows_read",
                    json!({"projectId":project,"section":section,"offset":state.offset,"limit":20}),
                ));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click, click_at_size, render, snapshot, state};
    use super::*;

    fn fixture() -> Value {
        let mut data = snapshot();
        data["desktopPreferences"] = json!({"language":"en"});
        data["abilities"] = patchfleet_core::abilities::catalog();
        data["providerConnections"] = json!({"reasonCode":"PROVIDER_CONNECTIONS_READ","providers":[{"id":"codex","installed":true,"version":"0.155.1","authentication":"present","controlSupported":true}]});
        data["projectCrews"] = json!({"revision":9,"crews":[{"crewId":"crew:one","projectId":"project:one","name":"Saved launch crew","members":[
            {"agentId":"agent:builder","name":"Builder","role":"presentation","abilityId":"P1","instruction":"Prepare the requested files."},
            {"agentId":"agent:reviewer","name":"Reviewer","role":"review","abilityId":"P1","instruction":"Review the actual files."}]}],
            "missions":[{"missionId":"mission:one","crewId":"crew:one","projectId":"project:one","title":"Launch deck","instruction":"Create a launch presentation","abilityId":"P1","revision":4,"status":"prepared",
                "tasks":[{"taskId":"task:builder","agentId":"agent:builder","status":"queued","attempt":0,"workItemId":"work:one"},{"taskId":"task:reviewer","agentId":"agent:reviewer","status":"pending","attempt":0,"workItemId":null}],
                "settings":{"concurrency":2,"retryLimit":1,"timeoutMinutes":30}}]});
        data["projectStudio"] = json!({
            "reasonCode":"PROJECT_STUDIO_READ","projectId":"project:one","revision":3,
            "tools":[
                {"id":"python","available":true},{"id":"ffmpeg","available":true},
                {"id":"ffprobe","available":true},{"id":"pdftotext","available":true},
                {"id":"pdftoppm","available":true},{"id":"tesseract","available":true},
                {"id":"whisper","available":true},{"id":"whisperModel","available":true},
                {"id":"libreoffice","available":true},{"id":"chrome","available":true}
            ],
            "imageEngine":{"configured":true,"available":true},
            "connections":[{"providerId":"notion","connected":true}]
        });
        data
    }

    fn plan() -> CrewDraft {
        CrewDraft {
            recommended_for: Some("Owner goal".into()),
            ability: "P1".into(),
            name: "Launch crew".into(),
            members: vec![
                json!({"name":"Builder","role":"presentation","abilityId":"P1","instruction":"Prepare the requested files."}),
                json!({"name":"Reviewer","role":"review","abilityId":"P1","instruction":"Review the actual files."}),
            ],
            settings: json!({"concurrency":2,"retryLimit":1,"timeoutMinutes":30}),
            crew_id: None,
        }
    }

    fn console() -> Console {
        let mut state = state();
        state.crew.single = false;
        state.crew.plans.insert(state.crew_key(), plan());
        Console {
            tab: Tab::Create,
            create: state,
            ..Default::default()
        }
    }

    #[test]
    fn mission_goal_is_inspectable_without_synthesizing_an_owner_message() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("console-language"), Language::En));
        let data = fixture();
        let mission = &data["projectCrews"]["missions"][0];
        let mut console = console();
        for expanded in [false, true] {
            let events = if expanded {
                let id = ctx.data(|data| {
                    data.get_temp::<egui::Id>(egui::Id::new(("mission-goal", "mission:one")))
                        .unwrap()
                });
                ctx.memory_mut(|memory| memory.request_focus(id));
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]
            } else {
                vec![]
            };
            let output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    console.crew_mission_card(ui, &data, mission, &mut vec![], true);
                },
            );
            let copy: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
            output.drop_without_applying_deltas();
            assert!(!copy.iter().any(|line| line == "You"));
            assert!(copy.iter().any(|line| line == "Goal"));
            assert_eq!(
                copy.iter()
                    .any(|line| line == "Create a launch presentation"),
                expanded
            );
        }
    }

    #[test]
    fn proposal_keeps_its_prepare_action_inside_the_same_surface() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectCrews"]["missions"] = json!([]);
        let mut console = console();
        for _ in 0..3 {
            render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(800.0, 560.0),
                false,
                vec![],
            );
        }
        let (card, clip) = ctx.data(|data| {
            data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new("crew-proposal-card"))
                .unwrap()
        });
        let (_, prepare, _) = ctx.data(|data| {
            data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                "test-action-state",
                "project_crew_prepare",
                "Prepare crew",
            )))
            .unwrap()
        });
        assert!(card.contains_rect(prepare) && clip.contains_rect(prepare));
    }

    #[test]
    fn busy_context_keeps_local_goal_and_defers_automatic_workflow_reads() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut console = console();
        console.create.select_productive_view(1);
        console.create.crew.workflow.read = None;
        let goal = console.create.draft().goal.clone();
        for _ in 0..3 {
            let (commands, _) = render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(800.0, 560.0),
                true,
                vec![],
            );
            assert!(commands.is_empty());
            assert!(console.create.crew.workflow.read.is_none());
            assert!(console.create.crew.pending.is_none());
            assert_eq!(console.create.draft().goal, goal);
        }
        let (commands, _) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            vec![],
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_workflows_read");
        assert_eq!(commands[0].payload["projectId"], "project:one");
    }

    #[test]
    fn advanced_planning_resolves_automatic_but_member_assignments_stay_concrete() {
        let mut data = fixture();
        data["providerConnections"]["providers"] = json!([{"id":"claude","version":"2.1.170","installed":true,"authentication":"present","controlSupported":true}]);
        let mut console = console();
        let mut commands = Vec::new();
        console.begin_model_plan(&data, &mut commands, None);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["providerId"], "claude");
        assert!(console.create.crew.workflow.provider.is_empty());
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut member_provider = String::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            provider_picker(ui, "member-provider-fixture", &mut member_provider)
        })
        .drop_without_applying_deltas();
        assert_eq!(
            member_provider, "codex",
            "per-agent assignment never stores Automatic"
        );
        *console.create.planner_provider() = "codex".into();
        commands.clear();
        console.begin_model_plan(&data, &mut commands, None);
        assert!(
            commands.is_empty(),
            "an explicit unavailable provider cannot use ready Claude"
        );
    }

    #[test]
    fn conversation_clear_and_file_replacement_affect_only_future_sends() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectCrews"]["missions"] = json!([]);
        data["work"]["items"][0]["status"] = json!("completed");
        data["work"]["items"].as_array_mut().unwrap().push(
            json!({"workItemId":"work:two","title":"New source","revision":3,"status":"completed"}),
        );
        data["projectJobs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"projectId":"project:one","workItemId":"work:two"}));
        let mut console = console();
        let key = console.create.crew_key();
        let old = ReplyDraft {
            parent: "work:one".into(),
            references: HashSet::from(["outputs/old.md".into()]),
        };
        console.create.chat.replies.insert(key.clone(), old.clone());
        let mut commands = Vec::new();
        console.send_conversation(&data, &mut commands);
        let first = commands.remove(0);
        assert!(
            console
                .create
                .crew
                .workflow
                .request
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                == Some(&old)
        );
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Clear follow-up"
        )
        .is_empty());
        assert!(!console.create.chat.replies.contains_key(&key));
        assert!(
            console
                .create
                .crew
                .workflow
                .request
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                == Some(&old),
            "in-flight request remains frozen"
        );
        console
            .create
            .selected_work
            .insert("project:one".into(), "work:two".into());
        console.create.chat.files_open = true;
        data["projectResults"] = json!({"reasonCode":"PROJECT_RESULTS_READ","projectId":"project:one","workItemId":"work:two","items":[{"path":"outputs/new.md","name":"new.md","bytes":42,"textPreview":true}]});
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "reference",
            "outputs/new.md"
        )
        .is_empty());
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Close files"
        )
        .is_empty());
        let replacement = console.create.chat.replies[&key].clone();
        assert_eq!(replacement.parent, "work:two");
        assert!(replacement.references.contains("outputs/new.md"));
        assert!(
            console
                .create
                .crew
                .workflow
                .request
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                == Some(&old)
        );
        data["projectWorkflowReceipt"] = json!({"commandId":first.payload["commandId"],"projectId":"project:one","planJobId":"plan:old","revision":1,"outcome":"applied"});
        data["projectCrewPlanJob"] = json!({"reasonCode":"PROJECT_CREW_PLAN_READ","projectId":"project:one","planJobId":"plan:old","revision":2,"status":"needs_input","questions":[]});
        console.create.sync(&data);
        assert!(console.create.crew.workflow.models[&key].reply.as_ref() == Some(&old));
        *console.create.conversation_input() = "Use my newly selected source".into();
        console.send_conversation(&data, &mut commands);
        let second = commands.remove(0);
        assert_eq!(second.payload["instruction"], first.payload["instruction"]);
        assert_eq!(second.payload["replyToWorkItemId"], "work:two");
        assert_eq!(second.payload["referencePaths"], json!(["outputs/new.md"]));
        assert!(
            console
                .create
                .crew
                .workflow
                .request
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                == Some(&replacement)
        );
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Clear follow-up"
        )
        .is_empty());
        assert!(
            console
                .create
                .crew
                .workflow
                .request
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                == Some(&replacement)
        );
        data["projectWorkflowReceipt"] = json!({"commandId":second.payload["commandId"],"projectId":"project:one","planJobId":"plan:new","revision":1,"outcome":"applied"});
        data["projectCrewPlanJob"] = json!({"reasonCode":"PROJECT_CREW_PLAN_READ","projectId":"project:one","planJobId":"plan:new","revision":2,"status":"needs_input","questions":[]});
        console.create.sync(&data);
        *console.create.conversation_input() = "Continue without attached files".into();
        console.send_conversation(&data, &mut commands);
        let third = commands.remove(0);
        assert!(third.payload.get("replyToWorkItemId").is_none());
        assert!(third.payload.get("referencePaths").is_none());
        assert!(console
            .create
            .crew
            .workflow
            .request
            .as_ref()
            .unwrap()
            .reply
            .is_none());
        assert_eq!(first.payload["referencePaths"], json!(["outputs/old.md"]));
        assert_eq!(second.payload["referencePaths"], json!(["outputs/new.md"]));
        assert_eq!(third.action, "project_crew_plan_start");
    }

    #[test]
    fn crew_context_paints_one_plan_and_closing_it_restores_the_chat_card() {
        let size = egui::vec2(1280.0, 880.0);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language,"reduceMotion":true});
                data["projectCrews"]["missions"] = json!([]);
                let mut console = console();
                console.hold_automatic_reads = true;
                console.create.draft().goal = "Unsent owner goal retained".into();
                let draft = console.create.draft().clone();
                let plan = console.create.crew.plans[&console.create.crew_key()].clone();
                console.create.select_productive_view(1);

                for _ in 0..3 {
                    let (commands, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                    assert!(commands.is_empty());
                    for label in ["Builder", "Reviewer", "Prepare crew", "Edit roster"] {
                        let localized = i18n::tr(Language::from_code(language), label);
                        assert_eq!(
                            copy.iter()
                                .filter(|line| line.as_str() == localized.as_ref())
                                .count(),
                            1,
                            "{language}/{theme:?}: duplicated {label}: {copy:?}"
                        );
                    }
                    assert!(copy.iter().any(|line| line == "Owner goal"));
                    let count =
                        i18n::tr(Language::from_code(language), "{0} agents").replace("{0}", "2");
                    assert!(copy.iter().any(|line| line == &count));
                }

                let rect = ctx.data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new(("project-view-tab", 0usize)))
                        .unwrap()
                });
                let pos = rect.center();
                for pressed in [true, false] {
                    assert!(render(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        vec![
                            egui::Event::PointerMoved(pos),
                            egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    )
                    .0
                    .is_empty());
                }
                assert_eq!(console.create.productive_view(), 0);
                let (commands, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                assert!(commands.is_empty());
                for label in [
                    "Builder",
                    "Reviewer",
                    "Prepare crew",
                    "Edit roster",
                    "Your crew is ready",
                ] {
                    let localized = i18n::tr(Language::from_code(language), label);
                    assert_eq!(
                        copy.iter()
                            .filter(|line| line.as_str() == localized.as_ref())
                            .count(),
                        1,
                        "{language}/{theme:?}: restored chat card missing {label}"
                    );
                }
                assert!(console.create.draft() == &draft);
                assert!(console.create.crew.plans[&console.create.crew_key()] == plan);
                assert!(console.create.crew.pending.is_none());
            }
        }
    }

    #[test]
    fn teams_inline_plan_keeps_prepare_and_return_visible_at_minimum_size() {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"]["language"] = json!(language);
                let mut console = console();
                console.tab = Tab::Teams;
                console.create.crew.workflow.read = Some(("project:one".into(), 0));
                console.create.select_productive_view(1);
                let size = egui::vec2(800.0, 560.0);
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                for _ in 0..3 {
                    let commands = render(&ctx, &mut console, &data, size, false, vec![]).0;
                    assert!(commands.is_empty(), "{commands:?}");
                }
                let (enabled, rect, clip) = ctx.data(|data| {
                    data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                        "test-action-state",
                        "project_crew_prepare",
                        "Prepare crew",
                    )))
                    .unwrap()
                });
                assert!(
                    enabled && screen.contains_rect(rect) && clip.contains_rect(rect),
                    "{language}/{theme:?} {rect:?}/{clip:?}"
                );
                let commands = click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "project_crew_prepare",
                    "Prepare crew",
                );
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].payload["projectId"], "project:one");
                assert!(click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Back to crews"
                )
                .is_empty());
                assert_eq!(console.create.productive_view(), 0);
                assert!(console.tab == Tab::Teams);
                assert_eq!(console.create.draft().goal, "Owner goal");
                assert!(console
                    .create
                    .crew
                    .plans
                    .contains_key(&console.create.crew_key()));
            }
        }
    }
    #[test]
    fn direction_and_recipe_have_one_visible_frozen_primary_action_at_minimum_size() {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                for tab in [1, 2] {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    data["projectCrews"]["missions"][0]["status"] = json!("active");
                    data["projectWorkflows"] = json!({"revision":7,"recipes":[{"recipeId":"recipe:one","projectId":null,"revision":3,"name":"Brief recipe","description":"Prepare reviewed files","members":plan().members,"settings":plan().settings}]});
                    let mut console = console();
                    console.create.crew.workflow.read = Some(("project:one".into(), 7));
                    console.create.select_productive_view(1);
                    console.create.crew.workflow.tab = tab;
                    console.create.crew.workflow.controls.insert(
                        ("project:one".into(), "mission:one".into()),
                        MissionControl {
                            instruction: "Use the new audience".into(),
                            tasks: HashSet::from(["task:builder".into()]),
                            ..Default::default()
                        },
                    );
                    console.create.crew.workflow.recipe_goal.insert(
                        ("project:one".into(), "recipe:one".into()),
                        "Owner recipe goal".into(),
                    );
                    let size = egui::vec2(800.0, 560.0);
                    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                    let label = if tab == 1 {
                        "Apply direction"
                    } else {
                        "Prepare recipe"
                    };
                    for _ in 0..3 {
                        let (commands, copy) =
                            render(&ctx, &mut console, &data, size, false, vec![]);
                        assert!(commands.is_empty());
                        let localized = i18n::tr(Language::from_code(language), label);
                        assert_eq!(
                            copy.iter()
                                .filter(|line| line.as_str() == localized.as_ref())
                                .count(),
                            1
                        );
                    }
                    let input_id = if tab == 1 {
                        "workflow-direction-input"
                    } else {
                        "workflow-recipe-input"
                    };
                    let (input, input_clip) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new(input_id))
                            .unwrap()
                    });
                    assert!(
                        screen.contains_rect(input) && input_clip.contains_rect(input),
                        "{language}/{theme:?}/{tab} input clipped {input:?}/{input_clip:?}"
                    );
                    if tab == 1 {
                        let (warning, clip) = ctx.data(|data| {
                            data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new(
                                "workflow-direction-warning",
                            ))
                            .unwrap()
                        });
                        assert!(
                            screen.contains_rect(warning)
                                && clip.contains_rect(warning)
                                && input.bottom() <= warning.top(),
                            "replacement warning must remain visible after direction field"
                        );
                        console
                            .create
                            .crew
                            .workflow
                            .controls
                            .get_mut(&("project:one".into(), "mission:one".into()))
                            .unwrap()
                            .tasks
                            .clear();
                        assert!(
                            click_at_size(
                                &ctx,
                                &mut console,
                                &data,
                                size,
                                false,
                                "memory_control",
                                label
                            )
                            .is_empty(),
                            "a direction requires explicit affected-task selection"
                        );
                        assert!(click_at_size(
                            &ctx,
                            &mut console,
                            &data,
                            size,
                            false,
                            "memory_control",
                            "Affected tasks"
                        )
                        .is_empty());
                        assert!(click_at_size(
                            &ctx,
                            &mut console,
                            &data,
                            size,
                            false,
                            "affected-task",
                            "task:builder"
                        )
                        .is_empty());
                        assert!(console.create.crew.workflow.controls
                            [&("project:one".into(), "mission:one".into())]
                            .tasks
                            .contains("task:builder"));
                    }
                    let rect = ctx.data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new((
                            "test-action",
                            "memory_control",
                            label,
                        )))
                        .unwrap()
                    });
                    let (clip, _) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                            "test-action-visibility",
                            label,
                        )))
                        .unwrap()
                    });
                    assert!(screen.contains_rect(rect) && clip.contains_rect(rect));
                    let commands = click_at_size(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        "memory_control",
                        label,
                    );
                    assert_eq!(commands.len(), 1);
                    assert_eq!(commands[0].payload["projectId"], "project:one");
                    if tab == 1 {
                        assert_eq!(commands[0].payload["expectedMissionRevision"], 4);
                        assert_eq!(
                            commands[0].payload["affectedTaskIds"],
                            json!(["task:builder"])
                        );
                    } else {
                        assert_eq!(commands[0].payload["expectedRecipeRevision"], 3);
                        assert_eq!(commands[0].payload["instruction"], "Owner recipe goal");
                    }
                }
            }
        }
    }

    #[test]
    fn exact_prepare_success_returns_to_the_selected_mission_and_exposes_explicit_start() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        let mut console = console();
        let size = egui::vec2(800.0, 560.0);
        console.create.crew.workflow.read = Some(("project:one".into(), 0));
        console.create.select_productive_view(1);
        let prepared = click_at_size(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            "project_crew_prepare",
            "Prepare crew",
        );
        assert_eq!(prepared.len(), 1);
        assert_eq!(console.create.productive_view(), 1);
        data["projectCrewReceipt"] = json!({"commandId":prepared[0].payload["commandId"],"projectId":"project:one","missionId":"mission:one","outcome":"applied"});
        let (commands, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
        assert!(commands
            .iter()
            .all(|command| command.action != "project_crew_start"));
        assert_eq!(console.create.productive_view(), 0);
        assert!(console.create.crew.panel_open);
        assert!(!copy.iter().any(|line| line == "Crew plan"));
        assert!(copy.iter().any(|line| line == "Start crew"));
        let start = click_at_size(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            "project_crew_start",
            "Start crew",
        );
        assert_eq!(start.len(), 1);
        assert_eq!(start[0].payload["missionId"], "mission:one");
        assert_eq!(start[0].payload["expectedMissionRevision"], 4);
        for navigated in [false, true] {
            let mut other = self::console();
            other.create.crew.workflow.read = Some(("project:one".into(), 0));
            other.create.select_productive_view(1);
            let key = other.create.crew_key();
            other.create.crew.pending = Some(CrewSubmission {
                command: "prepare:late".into(),
                key: key.clone(),
                draft: other.create.draft().clone(),
                reply: None,
                plan: plan(),
            });
            if navigated {
                other.create.select_productive_view(2);
            } else {
                other.create.draft().goal = "Later unsent goal".into();
            }
            data["projectCrewReceipt"] = json!({"commandId":"prepare:late","projectId":"project:one","missionId":"mission:one","outcome":"applied"});
            other.create.sync(&data);
            assert_eq!(
                other.create.productive_view(),
                if navigated { 2 } else { 1 }
            );
            if !navigated {
                assert_eq!(other.create.draft().goal, "Later unsent goal");
            }
        }
    }
    #[test]
    fn saved_completed_plan_exposes_explicit_goal_adoption_before_prepare() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut console = console();
        console.create.select_productive_view(1);
        console.create.crew.workflow.read = Some(("project:one".into(), 0));
        let key = console.create.crew_key();
        console
            .create
            .crew
            .plans
            .get_mut(&key)
            .unwrap()
            .recommended_for = Some("Saved plan goal".into());
        console.create.crew.workflow.models.insert(key,ModelPlan{job:json!({"projectId":"project:one","planJobId":"plan:saved","revision":3,"status":"completed","providerId":"codex"}),draft:CreateDraft{title:"Saved plan".into(),goal:"Saved plan goal".into()},reply:None,answers:HashMap::new(),assignments:HashMap::new(),read_revision:Some(3)});
        let size = egui::vec2(800.0, 560.0);
        for _ in 0..3 {
            assert!(render(&ctx, &mut console, &data, size, false, vec![])
                .0
                .is_empty());
        }
        assert_eq!(console.create.draft().goal, "Owner goal");
        let rect = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new((
                "test-action",
                "memory_control",
                "Use planned goal",
            )))
            .unwrap()
        });
        let (clip, _) = ctx.data(|data| {
            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                "test-action-visibility",
                "Use planned goal",
            )))
            .unwrap()
        });
        assert!(clip.contains_rect(rect));
        assert!(click_at_size(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            "project_crew_prepare",
            "Prepare crew"
        )
        .is_empty());
        assert!(click_at_size(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            "memory_control",
            "Use planned goal"
        )
        .is_empty());
        assert_eq!(console.create.draft().goal, "Saved plan goal");
        let commands = click_at_size(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            "project_crew_prepare",
            "Prepare crew",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_plan_accept");
        assert_eq!(commands[0].payload["planJobId"], "plan:saved");
        assert_eq!(commands[0].payload["expectedPlanRevision"], 3);
    }

    #[test]
    fn role_cards_show_localized_capabilities_and_keep_runtime_instructions_in_plan_details() {
        for language in ["en", "tr"] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut data = fixture();
            data["desktopPreferences"]["language"] = json!(language);
            let mut console = console();
            console.create.crew.workflow.read = Some(("project:one".into(), 0));
            console.create.select_productive_view(1);
            let key = console.create.crew_key();
            for member in &mut console.create.crew.plans.get_mut(&key).unwrap().members {
                member["instruction"] = json!("ACTUAL-AGENT-INSTRUCTION-CANARY");
            }
            let size = egui::vec2(800.0, 560.0);
            for _ in 0..3 {
                let (commands, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                assert!(commands.is_empty());
                assert!(copy.iter().any(|line| line
                    == i18n::tr(Language::from_code(language), "Quality review").as_ref()));
                assert!(!copy.join("\n").contains("ACTUAL-AGENT-INSTRUCTION-CANARY"));
                assert!(!copy
                    .iter()
                    .any(|line| line
                        == i18n::tr(Language::from_code(language), "Saved plans").as_ref()));
            }
            let payload = console.create.crew_payload(&data).unwrap();
            assert_eq!(
                payload["members"][0]["instruction"],
                "ACTUAL-AGENT-INSTRUCTION-CANARY"
            );
            assert!(click(
                &ctx,
                &mut console,
                &data,
                false,
                "memory_control",
                "Plan details"
            )
            .is_empty());
            let copy = render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 2200.0),
                false,
                vec![],
            )
            .1;
            assert!(copy.join("\n").contains("ACTUAL-AGENT-INSTRUCTION-CANARY"));
        }
    }

    #[test]
    fn model_planning_acceptance_preserves_async_scope_answers_and_manual_member_providers() {
        let mut data = fixture();
        data["providerConnections"]["providers"].as_array_mut().unwrap().push(json!({"id":"claude","version":"2.1.170","installed":true,"authentication":"present","controlSupported":true}));
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = console();
        console.create.crew.plans.clear();
        console.create.crew.workflow.provider = "claude".into();
        console.create.chat.replies.insert(
            console.create.crew_key(),
            ReplyDraft {
                parent: "work:one".into(),
                references: HashSet::from(["outputs/source.md".into()]),
            },
        );
        data["work"]["items"][0]["status"] = json!("completed");
        let mut commands = Vec::new();
        console.begin_model_plan(&data, &mut commands, None);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_plan_start");
        assert_eq!(commands[0].payload["providerId"], "claude");
        assert_eq!(
            commands[0].payload["referencePaths"],
            json!(["outputs/source.md"])
        );
        let request_id = commands[0].payload["commandId"].clone();
        data["projectWorkflowReceipt"] = json!({"commandId":request_id,"projectId":"project:other","planJobId":"plan:one","outcome":"applied","revision":1});
        console.create.sync(&data);
        assert!(console.create.crew.workflow.models.is_empty());
        data["projectWorkflowReceipt"]["projectId"] = json!("project:one");
        console.create.sync(&data);
        let key = console.create.crew_key();
        assert_eq!(
            console.create.crew.workflow.models[&key].job["providerId"],
            "claude"
        );
        data["projectCrewPlanJob"] = json!({"reasonCode":"PROJECT_CREW_PLAN_READ","projectId":"project:one","planJobId":"plan:one","revision":3,"status":"completed","providerId":"claude","plan":{"abilityId":"P1","members":plan().members,"settings":plan().settings}});
        console.create.sync(&data);
        let member = text(&console.create.crew.plans[&key].members[1], "_uiId").to_owned();
        console
            .create
            .crew
            .workflow
            .models
            .get_mut(&key)
            .unwrap()
            .assignments
            .insert(member, "gemini".into());
        console
            .create
            .crew
            .workflow
            .models
            .get_mut(&key)
            .unwrap()
            .read_revision = Some(3);
        let edited = console.create.crew.plans.get_mut(&key).unwrap();
        edited.name = "Owner edited crew name".into();
        edited.settings = json!({"concurrency":1,"retryLimit":2,"timeoutMinutes":45});
        let accepted = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_crew_prepare",
            "Prepare crew",
        );
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].action, "project_crew_plan_accept");
        assert_eq!(accepted[0].payload["expectedPlanRevision"], 3);
        assert_eq!(accepted[0].payload["crewName"], "Owner edited crew name");
        assert_eq!(
            accepted[0].payload["settings"],
            json!({"concurrency":1,"retryLimit":2,"timeoutMinutes":45})
        );
        assert_eq!(
            accepted[0].payload["providerAssignments"][1],
            json!({"memberIndex":1,"providerId":"gemini"})
        );
        assert_eq!(
            accepted[0].payload["referencePaths"],
            json!(["outputs/source.md"])
        );
        assert!(accepted[0].payload["members"][0].get("_uiId").is_none());
        console.create.draft().goal = "Later unsent owner draft".into();
        data["projectWorkflowReceipt"] = json!({"commandId":accepted[0].payload["commandId"],"projectId":"project:one","missionId":"mission:one","outcome":"applied"});
        console.create.sync(&data);
        assert_eq!(console.create.draft().goal, "Later unsent owner draft");
        assert!(console.create.chat.replies.contains_key(&key));
    }

    #[test]
    fn saved_plan_paging_uses_explicit_keyboard_action_and_exact_project_section_offset() {
        fn frame(
            ctx: &egui::Context,
            console: &Console,
            data: &Value,
            events: Vec<egui::Event>,
        ) -> Vec<Command> {
            let mut commands = Vec::new();
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 560.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| console.workflow_page_controls(ui, data, &mut commands, "plans"),
            )
            .drop_without_applying_deltas();
            commands
        }
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let console = console();
        let mut data = fixture();
        data["projectWorkflowRead"] = json!({"reasonCode":"PROJECT_WORKFLOWS_READ","projectId":"project:one","plans":(0..20).map(|index|json!({"projectId":"project:one","planJobId":format!("plan:{index}")})).collect::<Vec<_>>()});
        assert!(frame(&ctx, &console, &data, vec![]).is_empty());
        let next = ctx.data(|data| {
            data.get_temp::<egui::Id>(egui::Id::new(("test-action-id", "memory_control", "Next")))
                .unwrap()
        });
        ctx.memory_mut(|memory| memory.request_focus(next));
        let commands = frame(
            &ctx,
            &console,
            &data,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_workflows_read");
        assert_eq!(commands[0].payload["projectId"], "project:one");
        assert_eq!(commands[0].payload["section"], "plans");
        assert_eq!(commands[0].payload["offset"], 20);
        data["projectWorkflowRead"] = json!({"reasonCode":"PROJECT_WORKFLOWS_READ","projectId":"project:one","section":"plans","offset":20,"total":21,"nextOffset":null,"items":[{"projectId":"project:one","planJobId":"plan:20"}]});
        assert_eq!(workflow_values(&data, "project:one", "plans").len(), 1);
        assert!(frame(&ctx, &console, &data, vec![]).is_empty());
        let previous = ctx.data(|data| {
            data.get_temp::<egui::Id>(egui::Id::new((
                "test-action-id",
                "memory_control",
                "Previous",
            )))
            .unwrap()
        });
        ctx.memory_mut(|memory| memory.request_focus(previous));
        let commands = frame(
            &ctx,
            &console,
            &data,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["offset"], 0);
    }

    #[test]
    fn right_inspector_keeps_prepare_and_guidance_inside_its_real_panel() {
        let size = egui::vec2(1280.0, 820.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"]["language"] = json!(language);
                let mut console = console();
                console.create.crew.workflow.read = Some(("project:one".into(), 0));
                console.create.select_productive_view(1);
                for _ in 0..3 {
                    let commands = render(&ctx, &mut console, &data, size, false, vec![]).0;
                    assert!(commands
                        .iter()
                        .all(|command| command.action.ends_with("_read")));
                }
                let (bar, clip) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new("crew-preparation-bar"))
                        .unwrap()
                });
                assert!(
                    bar.width() <= 620.0 && bar.left() >= size.x - 652.0,
                    "{language}/{theme:?} must use the bounded right inspector: {bar:?}"
                );
                assert!(
                    screen.contains_rect(bar) && clip.contains_rect(bar),
                    "{language}/{theme:?} preparation bar clipped {bar:?}/{clip:?}"
                );
                let (enabled, button, clip) = ctx.data(|data| {
                    data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                        "test-action-state",
                        "project_crew_prepare",
                        "Prepare crew",
                    )))
                    .unwrap()
                });
                assert!(
                    enabled
                        && bar.contains_rect(button)
                        && screen.contains_rect(button)
                        && clip.contains_rect(button)
                );
                let (guidance, clip) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new(
                        "workflow-footer-guidance",
                    ))
                    .unwrap()
                });
                assert!(bar.contains_rect(guidance) && clip.contains_rect(guidance));
                let commands = click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "project_crew_prepare",
                    "Prepare crew",
                );
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].action, "project_crew_prepare");
                assert_eq!(commands[0].payload["projectId"], "project:one");
                assert_eq!(commands[0].payload["expectedProjectRevision"], 3);
                assert_eq!(commands[0].payload["abilityId"], "P1");
                assert_eq!(commands[0].payload["instruction"], "Owner goal");
                assert_eq!(commands[0].payload["members"], json!(plan().members));
                assert!(commands
                    .iter()
                    .all(|command| command.action != "project_crew_start"));
            }
        }
    }

    #[test]
    fn normal_plan_preparation_bar_follows_details_and_stays_bounded_when_expanded() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let data = fixture();
        let mut console = console();
        console.create.crew.workflow.read = Some(("project:one".into(), 0));
        console.create.select_productive_view(1);
        let key = console.create.crew_key();
        console.create.crew.plans.get_mut(&key).unwrap().members.push(json!({"name":"Researcher","role":"research","abilityId":"R1","instruction":"Research the requested sources."}));
        let size = egui::vec2(1280.0, 880.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for _ in 0..3 {
            assert!(render(&ctx, &mut console, &data, size, false, vec![])
                .0
                .is_empty());
        }
        let details = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new((
                "test-action",
                "memory_control",
                "Plan details",
            )))
            .unwrap()
        });
        let (bar, clip) = ctx.data(|data| {
            data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new("crew-preparation-bar"))
                .unwrap()
        });
        let gap = bar.top() - details.bottom();
        assert!(
            (0.0..=24.0).contains(&gap),
            "preparation detached from plan: gap={gap}, bar={bar:?}, details={details:?}"
        );
        assert!(screen.contains_rect(bar) && clip.contains_rect(bar));
        for member in &mut console.create.crew.plans.get_mut(&key).unwrap().members {
            member["instruction"] = json!("Actual agent guidance. ".repeat(90));
        }
        assert!(click_at_size(
            &ctx,
            &mut console,
            &data,
            size,
            false,
            "memory_control",
            "Plan details"
        )
        .is_empty());
        for _ in 0..3 {
            assert!(render(&ctx, &mut console, &data, size, false, vec![])
                .0
                .is_empty());
        }
        let (bar, clip) = ctx.data(|data| {
            data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new("crew-preparation-bar"))
                .unwrap()
        });
        assert!(
            screen.contains_rect(bar) && clip.contains_rect(bar),
            "expanded details must remain scrollable above the preparation bar"
        );
        let (enabled, button, clip) = ctx.data(|data| {
            data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                "test-action-state",
                "project_crew_prepare",
                "Prepare crew",
            )))
            .unwrap()
        });
        assert!(enabled && screen.contains_rect(button) && clip.contains_rect(button));
    }

    #[test]
    fn completed_plan_and_missing_questions_reveal_the_next_action_at_minimum_size() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                for needs_input in [false, true] {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    let mut console = console();
                    console.create.crew.workflow.open = true;
                    console.create.crew.workflow.read = Some(("project:one".into(), 0));
                    let job = json!({"reasonCode":"PROJECT_CREW_PLAN_READ","projectId":"project:one","planJobId":"plan:one","revision":3,"providerId":"codex","status":if needs_input{"needs_input"}else{"completed"},"questions":[{"id":"audience","question":"Who is this for?"},{"id":"format","question":"Which format should we use?"}]});
                    let model = ModelPlan {
                        job,
                        draft: console.create.draft().clone(),
                        reply: None,
                        answers: HashMap::from([
                            ("audience".into(), "Students".into()),
                            ("format".into(), "Slides".into()),
                        ]),
                        assignments: HashMap::new(),
                        read_revision: Some(3),
                    };
                    console
                        .create
                        .crew
                        .workflow
                        .models
                        .insert(console.create.crew_key(), model);
                    if needs_input {
                        console.create.crew.plans.clear();
                    }
                    if !needs_input {
                        let key = console.create.crew_key();
                        let plan = console.create.crew.plans.get_mut(&key).unwrap();
                        plan.name="Research the audience and prepare a launch presentation for a global community".into();
                        plan.members.push(json!({"name":"Final reviewer","role":"review","abilityId":"P1","instruction":"Review the deliverable."}));
                    }
                    for _ in 0..4 {
                        assert!(render(&ctx, &mut console, &data, size, false, vec![])
                            .0
                            .is_empty());
                    }
                    let (guidance, clip) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new(
                            "workflow-footer-guidance",
                        ))
                        .unwrap()
                    });
                    assert!(
                        screen.contains_rect(guidance) && clip.contains_rect(guidance),
                        "footer hint clipped {language}/{theme:?} {guidance:?}/{clip:?}"
                    );
                    let (heading, heading_clip) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new("workflow-heading"))
                            .unwrap()
                    });
                    let refresh = ctx.data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new((
                            "test-action",
                            "project_workflows_read",
                            "Refresh",
                        )))
                        .unwrap()
                    });
                    assert!(
                        screen.contains_rect(heading)
                            && heading_clip.contains_rect(heading)
                            && !heading.intersects(refresh)
                    );
                    if !needs_input {
                        let (row, clip) = ctx.data(|data| {
                            data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                                "crew-roster-row",
                                2usize,
                            )))
                            .unwrap()
                        });
                        assert!(
                            screen.contains_rect(row) && clip.contains_rect(row),
                            "third role clipped: {language}/{theme:?} {row:?}/{clip:?}"
                        );
                    }
                    if !needs_input {
                        let copy = render(&ctx, &mut console, &data, size, false, vec![]).1;
                        assert!(!copy.iter().any(|line|line=="Generate plan"||line=="Mox planner"||line=="completed"),"completed plan must present one roster and next action, not duplicate planner cards");
                    }
                    let (action_name, label) = if needs_input {
                        ("memory_control", "Send answers & replan")
                    } else {
                        ("project_crew_prepare", "Prepare crew")
                    };
                    let rect = ctx.data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new((
                            "test-action",
                            action_name,
                            label,
                        )))
                        .unwrap()
                    });
                    let clip = if needs_input {
                        ctx.data(|data| {
                            data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                                "test-action-visibility",
                                label,
                            )))
                            .unwrap()
                            .0
                        })
                    } else {
                        ctx.data(|data| {
                            data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                                "test-action-state",
                                action_name,
                                label,
                            )))
                            .unwrap()
                            .2
                        })
                    };
                    assert!(
                        screen.contains_rect(rect) && clip.contains_rect(rect),
                        "{language}/{theme:?}/{needs_input} {rect:?} {clip:?}"
                    );
                    let commands =
                        click_at_size(&ctx, &mut console, &data, size, false, action_name, label);
                    assert_eq!(commands.len(), 1);
                    assert_eq!(
                        commands[0].action,
                        if needs_input {
                            "project_crew_plan_start"
                        } else {
                            "project_crew_plan_accept"
                        }
                    );
                    assert!(commands
                        .iter()
                        .all(|command| command.action != "project_crew_start"));
                }
            }
        }
    }

    #[test]
    fn inline_workflow_navigation_schedules_and_usage_are_explicit_and_accessible() {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                for tab in 0..5 {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    data["projectWorkflows"] = json!({"revision":7,"plans":[],"recipes":[{"recipeId":"recipe:one","projectId":null,"revision":2,"name":"Research brief","description":"Bounded project recipe","members":plan().members,"settings":plan().settings}],"schedules":[],"usage":[{"missionId":"mission:one","projectId":"project:one","startedJobs":2,"elapsedSeconds":14,"inputTokens":null,"outputTokens":null,"costMicros":null,"measurementComplete":false,"bottlenecks":[{"taskId":"task:reviewer","reasonCode":"WAITING_FOR_DEPENDENCIES"}]}]});
                    let mut console = console();
                    console.create.crew.workflow.open = true;
                    console.create.crew.workflow.tab = tab;
                    console.create.crew.workflow.read = Some(("project:one".into(), 7));
                    let size = egui::vec2(800.0, 560.0);
                    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                    for _ in 0..3 {
                        assert!(render(&ctx, &mut console, &data, size, false, vec![])
                            .0
                            .is_empty());
                    }
                    if tab == 3 {
                        let provider = ctx.data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new((
                                "test-action",
                                "memory_control",
                                "Choose provider",
                            )))
                            .unwrap()
                        });
                        assert!(
                            screen.contains_rect(provider),
                            "{language}/{theme:?} provider picker {provider:?}"
                        );
                    }
                    assert!(click_at_size(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        "memory_control",
                        "Crew view"
                    )
                    .is_empty());
                    let next = (tab + 1) % 5;
                    let (rect, clip, id) = ctx.data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect, egui::Id)>(egui::Id::new((
                            "workflow-view-tab",
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
                    assert!(commands.is_empty());
                    assert_eq!(console.create.crew.workflow.tab, next);
                    assert!(!copy
                        .iter()
                        .any(|line| line == "Crew workspace" || line == "Close"));
                    console.create.crew.workflow.tab = tab;
                    if tab == 4 {
                        let copy = render(
                            &ctx,
                            &mut console,
                            &data,
                            egui::vec2(1280.0, 2200.0),
                            false,
                            vec![],
                        )
                        .1
                        .join("\n");
                        assert!(copy.contains("Launch deck"));
                        assert!(
                            !copy.contains("mission:one"),
                            "technical identity belongs in Details"
                        );
                        assert!(copy.contains(if language == "en" {
                            "Unknown"
                        } else {
                            "Bilinmiyor"
                        }));
                        assert!(copy.contains(
                            i18n::tr(Language::from_code(language), "WAITING_FOR_DEPENDENCIES")
                                .as_ref()
                        ));
                    }
                }
            }
        }
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectWorkflows"] = json!({"revision":1,"recipes":[{"recipeId":"recipe:one","projectId":null,"revision":3,"name":"Research brief"}],"schedules":[]});
        let mut console = console();
        console.create.crew.workflow.open = true;
        console.create.crew.workflow.tab = 3;
        console.create.crew.workflow.read = Some(("project:one".into(), 1));
        console
            .create
            .crew
            .workflow
            .recipe
            .insert("project:one".into(), "recipe:one".into());
        console.create.crew.workflow.first_run = "2100-01-01T10:00:00Z".into();
        console.create.crew.workflow.schedule_provider = "claude".into();
        data["providerConnections"]["providers"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"claude","installed":true,"version":"2.1.170","authentication":"present","controlSupported":true}));
        assert!(render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 2200.0),
            false,
            vec![]
        )
        .0
        .is_empty());
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_schedule_save",
            "Create enabled schedule",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["expectedRecipeRevision"], 3);
        assert_eq!(commands[0].payload["providerId"], "claude");
        assert_eq!(
            commands[0].payload["firstRunAt"],
            "2100-01-01T10:00:00.000Z"
        );
        console.create.crew.workflow.schedule_provider = "gemini".into();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_schedule_save",
            "Create enabled schedule",
        )
        .is_empty());
        console.create.crew.workflow.schedule_provider = "claude".into();
        console.create.crew.workflow.first_run = "2100-01-01T10:00:00.123456+03:00".into();
        let offset_commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_schedule_save",
            "Create enabled schedule",
        );
        assert_eq!(offset_commands.len(), 1);
        assert_eq!(offset_commands[0].payload["providerId"], "claude");
        assert_eq!(
            offset_commands[0].payload["firstRunAt"],
            "2100-01-01T07:00:00.123Z"
        );
        assert!(commands
            .iter()
            .all(|command| command.action != "project_crew_start"));
    }

    #[test]
    fn planning_is_explicit_read_only_and_late_plans_never_replace_another_scope() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = Console::launch();
        let mut data = fixture();
        data["projectCrews"]["missions"] = json!([]);
        console.create.project = "project:one".into();
        console.create.draft().goal = "A launch presentation".into();
        let (commands, _) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            vec![],
        );
        assert!(
            commands.is_empty(),
            "typing and opening the page must not plan, prepare or run"
        );
        assert!(!console.create.crew.single);
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_crew_plan",
            "Plan crew",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_plan");
        assert_eq!(commands[0].payload["projectId"], "project:one");
        assert!(commands[0].payload["abilityId"].is_null());
        let command = commands[0].payload["commandId"].clone();
        let response = json!({"reasonCode":"PROJECT_CREW_PLAN","commandId":command,"projectId":"project:two","abilityId":"P1","basis":"bundled_guides","members":plan().members,"settings":plan().settings});
        data["projectCrewPlan"] = response;
        console.create.sync(&data);
        assert!(console.create.crew.plans.is_empty());
        data["projectCrewPlan"]["projectId"] = json!("project:one");
        console.create.project = "project:two".into();
        console.create.draft().goal = "A different unsent goal".into();
        console.create.sync(&data);
        assert!(console
            .create
            .crew
            .plans
            .contains_key(&("project:one".into(), String::new())));
        assert_eq!(console.create.project, "project:two");
        assert_eq!(console.create.draft().goal, "A different unsent goal");
        assert!(console.create.crew_payload(&data).is_none());
        console.create.project = "project:one".into();
        console.create.draft().goal = "Changed after recommendation".into();
        assert!(
            console.create.crew_payload(&data).is_none(),
            "a stale recommendation must be planned again or explicitly edited"
        );
    }

    #[test]
    fn prepare_receipts_require_exact_command_project_mission_and_frozen_draft_reply_roster() {
        for changed in ["none", "goal", "reply", "roster"] {
            let mut state = state();
            let mut data = fixture();
            let key = state.crew_key();
            let original = plan();
            state.crew.plans.insert(key.clone(), original.clone());
            let reply = ReplyDraft {
                parent: "work:one".into(),
                references: HashSet::from(["outputs/a.md".into()]),
            };
            state.chat.replies.insert(key.clone(), reply.clone());
            state.crew.pending = Some(CrewSubmission {
                command: "cmd:prepare".into(),
                key: key.clone(),
                draft: state.draft().clone(),
                reply: Some(reply),
                plan: original,
            });
            match changed {
                "goal" => state.draft().goal = "New unsent goal".into(),
                "reply" => {
                    state
                        .chat
                        .replies
                        .get_mut(&key)
                        .unwrap()
                        .references
                        .insert("outputs/b.md".into());
                }
                "roster" => {
                    state.crew.plans.get_mut(&key).unwrap().name = "Edited after click".into()
                }
                _ => {}
            }
            data["projectCrewReceipt"] = json!({"commandId":"wrong","projectId":"project:one","missionId":"mission:one","outcome":"applied"});
            state.sync(&data);
            assert!(state.crew.pending.is_some());
            data["projectCrewReceipt"]["commandId"] = json!("cmd:prepare");
            data["projectCrewReceipt"]["projectId"] = json!("project:two");
            state.sync(&data);
            assert!(state.crew.pending.is_some());
            data["projectCrewReceipt"]["projectId"] = json!("project:one");
            data["projectCrewReceipt"]["missionId"] = json!("mission:foreign");
            state.sync(&data);
            assert!(state.crew.pending.is_some());
            state.project = "project:two".into();
            state.draft().goal = "Keep this project draft".into();
            data["projectCrewReceipt"]["missionId"] = json!("mission:one");
            state.sync(&data);
            assert!(state.crew.pending.is_none());
            assert_eq!(state.draft().goal, "Keep this project draft");
            assert!(!state.crew.panel_open);
            assert_eq!(state.drafts.contains_key(&key), changed != "none");
            assert_eq!(state.crew.plans.contains_key(&key), changed != "none");
            assert_eq!(state.chat.replies.contains_key(&key), changed != "none");
            assert_eq!(state.crew.selected["project:one"], "mission:one");
        }
    }

    #[test]
    fn minimum_plan_and_prepare_are_visible_clickable_and_never_start_a_provider() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                for action_name in ["project_crew_plan", "project_crew_prepare"] {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut console = console();
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    for _ in 0..3 {
                        assert!(render(&ctx, &mut console, &data, size, false, vec![])
                            .0
                            .is_empty());
                    }
                    let label = if action_name == "project_crew_plan" {
                        assert!(click_at_size(
                            &ctx,
                            &mut console,
                            &data,
                            size,
                            false,
                            "memory_control",
                            "More"
                        )
                        .is_empty());
                        render(&ctx, &mut console, &data, size, false, vec![]);
                        "Plan crew"
                    } else {
                        "Prepare crew"
                    };
                    let (enabled, rect, clip) = ctx.data(|data| {
                        data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                            "test-action-state",
                            action_name,
                            label,
                        )))
                        .unwrap()
                    });
                    assert!(
                        enabled && screen.contains_rect(rect) && clip.contains_rect(rect),
                        "{language} {theme:?} {label} clipped: {rect:?} {clip:?}"
                    );
                    let commands =
                        click_at_size(&ctx, &mut console, &data, size, false, action_name, label);
                    assert_eq!(commands.len(), 1);
                    assert_eq!(commands[0].action, action_name);
                    if action_name == "project_crew_prepare" {
                        assert_eq!(commands[0].payload["projectId"], "project:one");
                        assert_eq!(commands[0].payload["expectedProjectRevision"], 3);
                        assert_eq!(commands[0].payload["abilityId"], "P1");
                        assert_eq!(commands[0].payload["members"].as_array().unwrap().len(), 2);
                    }
                    assert!(commands
                        .iter()
                        .all(|command| command.action != "project_crew_start"
                            && command.action != "work_start"));
                }
            }
        }
    }

    #[test]
    fn reused_roster_is_project_bound_and_each_mission_keeps_its_new_goal() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = console();
        let data = fixture();
        console.tab = Tab::Teams;
        let source = data["projectCrews"].clone();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Reuse crew"
        )
        .is_empty());
        assert!(console.tab == Tab::Create);
        assert!(!console.create.crew.single);
        console.create.draft().goal = "A second goal for the same roster".into();
        let payload = console.create.crew_payload(&data).unwrap();
        assert_eq!(payload["crewId"], "crew:one");
        assert!(payload["members"].is_null());
        assert_eq!(payload["instruction"], "A second goal for the same roster");
        console.create.project = "project:two".into();
        assert!(console.create.crew_payload(&data).is_none());
        assert_eq!(data["projectCrews"], source);
    }

    #[test]
    fn mission_controls_use_current_scope_revision_and_truthful_connection_state() {
        for (status_name, action_name, label) in [
            ("prepared", "project_crew_start", "Start crew"),
            ("active", "project_crew_cancel", "Stop crew"),
            ("failed", "project_crew_retry", "Retry crew"),
        ] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut console = console();
            console.create.crew.plans.clear();
            let mut data = fixture();
            data["projectCrews"]["missions"][0]["status"] = json!(status_name);
            let commands = click(&ctx, &mut console, &data, false, action_name, label);
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].action, action_name);
            assert_eq!(commands[0].payload["projectId"], "project:one");
            assert_eq!(commands[0].payload["missionId"], "mission:one");
            assert_eq!(commands[0].payload["expectedMissionRevision"], 4);
            assert!(click(&ctx, &mut console, &data, true, action_name, label).is_empty());
            if status_name == "prepared" {
                data["providerConnections"]["providers"][0]["authentication"] = json!("missing");
                assert!(click(&ctx, &mut console, &data, false, action_name, label).is_empty());
            }
        }
    }

    #[test]
    fn crew_start_combines_member_requirements_and_routes_missing_tools() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut blocked_console = console();
        blocked_console.create.crew.plans.clear();
        let mut data = fixture();
        data["projectCrews"]["missions"][0]["tasks"][0]["abilityId"] = json!("V1");
        data["projectCrews"]["missions"][0]["tasks"][1]["abilityId"] = json!("P1");
        data["projectStudio"]["tools"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|tool| tool["id"] == "ffprobe")
            .unwrap()["available"] = json!(false);
        render(
            &ctx,
            &mut blocked_console,
            &data,
            egui::vec2(1280.0, 900.0),
            false,
            vec![],
        );
        assert!(!ctx.data(|state| {
            state
                .get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                    "test-action-state",
                    "project_crew_start",
                    "Start crew",
                )))
                .unwrap()
                .0
        }));
        assert!(click(
            &ctx,
            &mut blocked_console,
            &data,
            false,
            "project_crew_start",
            "Start crew"
        )
        .is_empty());
        assert!(click(
            &ctx,
            &mut blocked_console,
            &data,
            false,
            "memory_control",
            "Review setup"
        )
        .is_empty());
        assert_eq!(blocked_console.create.productive_view(), 2);
        assert_eq!(blocked_console.create.studio.tab, 2);

        let mut ready_console = console();
        ready_console.create.crew.plans.clear();
        data["projectStudio"]["tools"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|tool| tool["id"] == "ffprobe")
            .unwrap()["available"] = json!(true);
        let commands = click(
            &ctx,
            &mut ready_console,
            &data,
            false,
            "project_crew_start",
            "Start crew",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_start");
    }

    #[test]
    fn message_reads_are_scoped_bounded_and_pending_delivery_never_claims_a_reply() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = console();
        let mut data = fixture();
        console.create.crew.plans.clear();
        console.create.crew.panel_open = true;
        console.create.crew.tab = 1;
        console
            .create
            .crew
            .selected
            .insert("project:one".into(), "mission:one".into());
        data["projectCrewMessages"] = json!({"reasonCode":"PROJECT_CREW_MESSAGES_READ","projectId":"project:two","missionId":"mission:one","offset":0,"items":[{"body":"FOREIGN_MESSAGE_CANARY"}]});
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        assert!(!copy.join("\n").contains("FOREIGN_MESSAGE_CANARY"));
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_messages_read");
        assert_eq!(commands[0].payload["missionId"], "mission:one");
        assert_eq!(commands[0].payload["limit"], 20);
        assert!(
            render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 880.0),
                false,
                vec![]
            )
            .0
            .is_empty(),
            "a failed read must not loop"
        );
        data["projectCrewMessages"] = json!({"reasonCode":"PROJECT_CREW_MESSAGES_READ","projectId":"project:one","missionId":"mission:one","offset":0,"items":[{"fromMissionId":"mission:one","fromAgentId":"agent:builder","toMissionId":"mission:one","toAgentId":"agent:reviewer","body":"Actual message Projects Memory","delivery":"pending"}]});
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        let copy = copy.join("\n");
        assert!(commands.is_empty());
        assert!(copy.contains("Actual message Projects Memory"));
        assert!(copy.contains("Builder > Reviewer"));
        assert!(copy.contains("Pending recipient read"));
        assert!(!copy.contains("Read by the recipient"));
        let mut origin = data["projectCrews"]["missions"][0].clone();
        origin["missionId"] = json!("mission:earlier");
        origin["title"] = json!("Earlier project mission");
        data["projectCrews"]["missions"]
            .as_array_mut()
            .unwrap()
            .push(origin);
        data["projectCrewMessages"]["items"][0]["fromMissionId"] = json!("mission:earlier");
        let (_, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        assert!(
            copy.join("\n")
                .contains("Builder · Earlier project mission > Reviewer"),
            "cross-mission messages must identify the source mission"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn crew_files_never_render_outputs_from_another_mission_in_the_same_project() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = console();
        let mut data = fixture();
        console.create.crew.plans.clear();
        console.create.crew.panel_open = true;
        console.create.crew.tab = 2;
        console
            .create
            .crew
            .selected
            .insert("project:one".into(), "mission:one".into());
        data["work"]["items"].as_array_mut().unwrap().push(json!({"workItemId":"work:outside","title":"Other mission work","revision":2,"status":"completed"}));
        data["projectJobs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"projectId":"project:one","workItemId":"work:outside"}));
        console
            .create
            .selected_work
            .insert("project:one".into(), "work:outside".into());
        data["projectResults"] = json!({"reasonCode":"PROJECT_RESULTS_READ","projectId":"project:one","workItemId":"work:outside","items":[{"name":"OTHER_MISSION_FILE_CANARY","path":"outputs/outside.md","bytes":10,"textPreview":true}]});
        let (_, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        assert!(!copy.join("\n").contains("OTHER_MISSION_FILE_CANARY"));
        console
            .create
            .selected_work
            .insert("project:one".into(), "work:one".into());
        data["projectResults"]["workItemId"] = json!("work:one");
        data["projectResults"]["items"][0]["name"] = json!("OWN_MISSION_FILE");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_result_read",
            "Preview text",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["workItemId"], "work:one");
        assert_eq!(commands[0].payload["projectId"], "project:one");
    }
    #[test]
    fn roster_editor_removes_and_adds_catalog_agents_without_changing_the_primary_skill() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = console();
        let mut data = fixture();
        data["projectCrews"]["missions"] = json!([]);
        data["projectStudio"] = json!({
            "reasonCode":"PROJECT_STUDIO_READ",
            "projectId":"project:one",
            "revision":3,
            "tools":[],
            "imageEngine":{"configured":false,"available":false},
            "connections":[]
        });
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Edit roster"
        )
        .is_empty());
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        render(&ctx, &mut console, &data, size, false, vec![]);
        for label in ["Done", "Add from skills"] {
            let rect = ctx.data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new(("test-action", "memory_control", label)))
                    .unwrap()
            });
            let (clip, visible) = ctx.data(|data| {
                data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                    "test-action-visibility",
                    label,
                )))
                .unwrap()
            });
            assert!(
                visible && screen.contains_rect(rect) && clip.contains_rect(rect),
                "roster footer must stay reachable: {label} {rect:?} {clip:?}"
            );
        }
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Remove agent"
        )
        .is_empty());
        let key = console.create.crew_key();
        assert_eq!(console.create.crew.plans[&key].members.len(), 1);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Add from skills"
        )
        .is_empty());
        console.create.search = ability_text(
            array(&data["abilities"])
                .iter()
                .find(|ability| ability["id"] == "V1")
                .unwrap(),
            "name",
            Language::En,
        )
        .into();
        assert!(click(&ctx, &mut console, &data, false, "ability", "V1").is_empty());
        assert_eq!(console.create.ability, "P1");
        assert_eq!(console.create.crew.plans[&key].members.len(), 2);
        assert_eq!(
            console.create.crew.plans[&key].members[1]["abilityId"],
            "V1"
        );
        assert!(console.create.crew.plans[&key].recommended_for.is_none());
        assert!(console.create.crew.pending.is_none());
        assert_eq!(console.create.draft().goal, "Owner goal");
    }

    #[test]
    fn narrow_crew_modal_inherits_busy_state_and_keeps_lifecycle_actions_visible() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                for busy in [true, false] {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut console = console();
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    console.create.crew.plans.clear();
                    console.create.crew.panel_open = true;
                    for _ in 0..3 {
                        assert!(render(&ctx, &mut console, &data, size, busy, vec![])
                            .0
                            .is_empty());
                    }
                    let (_, copy) = render(&ctx, &mut console, &data, size, busy, vec![]);
                    let tabs = if language == "tr" {
                        ["Ajanlar", "Mesajlar", "Dosyalar"]
                    } else {
                        ["Agents", "Messages", "Files"]
                    };
                    for tab in tabs {
                        assert!(
                            copy.iter().any(|text| text == tab),
                            "{language} {theme:?} crew tab must use the selected language: {tab}"
                        );
                    }
                    if language == "tr" {
                        assert!(!copy.iter().any(|text| text == "Messages"));
                    }
                    let (enabled, rect, clip) = ctx.data(|data| {
                        data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                            "test-action-state",
                            "project_crew_start",
                            "Start crew",
                        )))
                        .unwrap()
                    });
                    assert_eq!(enabled, !busy);
                    assert!(
                        screen.contains_rect(rect) && clip.contains_rect(rect),
                        "{language} {theme:?} start clipped {rect:?} {clip:?}"
                    );
                    let commands = click_at_size(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        busy,
                        "project_crew_start",
                        "Start crew",
                    );
                    assert_eq!(commands.len(), usize::from(!busy));
                    if let Some(command) = commands.first() {
                        assert_eq!(command.action, "project_crew_start");
                        assert_eq!(command.payload["missionId"], "mission:one");
                    }
                }
            }
        }
    }

    #[test]
    fn timeline_uses_creation_time_and_keeps_unknown_projects_out() {
        let mut data = fixture();
        data["projectCrews"]["missions"][0]["createdAt"] = json!("2026-10-19T10:00:00Z");
        let mut later = data["projectCrews"]["missions"][0].clone();
        later["missionId"] = json!("mission:aaa");
        later["createdAt"] = json!("2026-10-19T11:00:00Z");
        data["projectCrews"]["missions"]
            .as_array_mut()
            .unwrap()
            .insert(0, later);
        let mut foreign = data["projectCrews"]["missions"][0].clone();
        foreign["projectId"] = json!("project:two");
        foreign["createdAt"] = json!("2026-10-19T12:00:00Z");
        data["projectCrews"]["missions"]
            .as_array_mut()
            .unwrap()
            .push(foreign);
        let values = ordered_missions(&data, "project:one");
        assert_eq!(values.len(), 2);
        assert_eq!(values[0]["missionId"], "mission:one");
        assert_eq!(values[1]["missionId"], "mission:aaa");
    }
}
