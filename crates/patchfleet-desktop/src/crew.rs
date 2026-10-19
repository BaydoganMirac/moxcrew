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

impl CreateState {
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
            if self.project == pending.key.0 {
                self.crew.panel_open = true;
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
        self.crew.plans.contains_key(&self.crew_key())
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
            || matches!(
                scene,
                "productive-resources"
                    | "productive-apps"
                    | "productive-app-preview"
                    | "productive-tools"
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
        if control(ui, "Mox planner", true).clicked() {
            self.create.crew.workflow.open = true;
            self.create.crew.workflow.tab = 0;
        }
        let goal = self.create.draft().goal.clone();
        let project = array(&snapshot["projectMemory"]["projects"])
            .iter()
            .any(|project| {
                text(project, "projectId") == self.create.project && revision(project) > 0
            });
        let enabled = project
            && valid_input(&goal, 20_000)
            && self.create.crew.pending.is_none()
            && self.create.pending.is_none();
        let clicked = ui
            .add_enabled_ui(enabled, |ui| submit(ui, "Plan crew", "project_crew_plan"))
            .inner;
        if (clicked || (shortcut && enabled)) && commands.is_empty() {
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
        }
    }

    pub(super) fn crew_chat_content(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        let project = self.create.project.clone();
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
        if self.create.crew.plans.contains_key(&self.create.crew_key()) {
            self.crew_plan_card(ui, snapshot, commands);
        }
    }

    fn crew_plan_card(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let key = self.create.crew_key();
        let Some(plan) = self.create.crew.plans.get(&key).cloned() else {
            return;
        };
        ui.add_space(12.0);
        egui::Frame::new()
            .fill(colors(ui).surface)
            .stroke(egui::Stroke::new(1.0, colors(ui).primary))
            .corner_radius(18)
            .inner_margin(16)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(studio::title(t(ui, "Your crew plan"), 20.0));
                    hint(
                        ui,
                        if plan.crew_id.is_some() {
                            "Saved project crew"
                        } else if self.create.crew.workflow.models.contains_key(&key) {
                            "Provider-generated plan"
                        } else if plan.recommended_for.is_some() {
                            "Based on bundled guides"
                        } else {
                            "Your edited roster"
                        },
                    );
                });
                ui.label(RichText::new(&plan.name).strong());
                ui.horizontal_wrapped(|ui| {
                    for member in &plan.members {
                        ui.label(RichText::new(text(member, "name")).color(colors(ui).primary));
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
                ui.horizontal_wrapped(|ui| {
                    if control(ui, "Edit roster", true).clicked() {
                        self.create.crew.roster_open = true;
                        self.create.crew.workflow.open=false;
                    }
                    let current_goal=self.create.draft().goal.clone();
                    let model=self.create.crew.workflow.models.get(&key).cloned();
                    let payload=self.create.crew_payload(snapshot).filter(|_|model.as_ref().is_none_or(|model|text(&model.job,"status")=="completed" && model.draft.goal==current_goal && model.reply.as_ref()==self.create.chat.replies.get(&key))).map(|mut payload| {
                        if let Some(model)=&model {
                            payload["planJobId"]=model.job["planJobId"].clone();
                            payload["expectedPlanRevision"]=model.job["revision"].clone();
                            payload["providerAssignments"]=json!(plan.members.iter().enumerate().map(|(index,member)|json!({"memberIndex":index,"providerId":model.assignments.get(text(member,"_uiId")).map(String::as_str).unwrap_or(text(&model.job,"providerId"))})).collect::<Vec<_>>());
                            for key in ["abilityId","crewName","crewId","title","instruction","settings"] {payload.as_object_mut().unwrap().remove(key);}
                        }
                        payload
                    });
                    let clicked = ui
                        .add_enabled_ui(
                            payload.is_some()
                                && self.create.crew.pending.is_none()
                                && self.create.pending.is_none(),
                            |ui| submit(ui, "Prepare crew", "project_crew_prepare"),
                        )
                        .inner;
                    if clicked && commands.is_empty() {
                        if let Some(payload) = payload {
                            let command = Command::new(if model.is_some(){"project_crew_plan_accept"}else{"project_crew_prepare"}, payload);
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
                });
                hint(
                    ui,
                    "Preparing saves the mission. Agents start only when you choose Start crew.",
                );
            });
        ui.add_space(12.0);
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
                                        ui.add(
                                            egui::TextEdit::singleline(&mut name)
                                                .desired_width(
                                                    (ui.available_width() - 110.0).max(120.0),
                                                )
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
                                        egui::ComboBox::from_id_salt("crew-member-role")
                                            .selected_text(t(ui, label))
                                            .width(180.0)
                                            .show_ui(ui, |ui| {
                                                for (value, label) in roles {
                                                    ui.selectable_value(
                                                        &mut selected_role,
                                                        value.into(),
                                                        t(ui, label),
                                                    );
                                                }
                                            });
                                    });
                                    member["role"] = json!(selected_role);
                                    ui.collapsing(t(ui, "Instructions"), |ui| {
                                        let mut instruction =
                                            text(member, "instruction").to_owned();
                                        ui.add(
                                            egui::TextEdit::multiline(&mut instruction)
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
                    ui.label(studio::title(text(crew, "name"), 18.0));
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
        ui.add_space(16.0);
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
        ui.add_space(12.0);
        if show_goal {
            egui::Frame::new()
                .fill(colors(ui).info_surface)
                .corner_radius(16)
                .inner_margin(14)
                .show(ui, |ui| {
                    hint(ui, "You");
                    ui.add(
                        egui::Label::new(text(mission, "instruction"))
                            .wrap()
                            .selectable(true),
                    );
                });
            ui.add_space(10.0);
        }
        egui::Frame::new()
            .fill(colors(ui).surface)
            .stroke(egui::Stroke::new(1.0, colors(ui).border))
            .corner_radius(18)
            .inner_margin(16)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    crate::motion::mox(
                        ui,
                        36.0,
                        matches!(text(mission, "status"), "active" | "cancelling"),
                        snapshot["desktopPreferences"]["reduceMotion"] == true,
                    );
                    ui.label(studio::title(text(mission, "title"), 18.0));
                    status(ui, text(mission, "status"));
                });
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
        ui.add_space(10.0);
    }

    fn crew_mission_actions(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        mission: &Value,
        commands: &mut Vec<Command>,
    ) {
        let valid = text(mission, "projectId") == self.create.project && revision(mission) > 0;
        ui.horizontal_wrapped(|ui| {
            if text(mission,"status")=="prepared" {
                action(ui,commands,"Start crew","project_crew_start",valid && connect::crew_can_start(snapshot,mission),||json!({"projectId":mission["projectId"],"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"]}));
            }
            if matches!(text(mission,"status"),"prepared"|"active") {
                action(ui,commands,"Stop crew","project_crew_cancel",valid,||json!({"projectId":mission["projectId"],"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"]}));
            }
            if matches!(text(mission,"status"),"failed"|"cancelled"|"timed_out") {
                action(ui,commands,"Retry crew","project_crew_retry",valid,||json!({"projectId":mission["projectId"],"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"]}));
            }
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
        ui.add_space(8.0);
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
fn provider_picker(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut String) {
    if value.is_empty() {
        *value = "codex".into();
    }
    egui::ComboBox::from_id_salt(id)
        .selected_text(match value.as_str() {
            "claude" => "Claude Code",
            "gemini" => "Gemini CLI",
            _ => "Codex",
        })
        .width(125.0)
        .show_ui(ui, |ui| {
            for (id, label) in [
                ("codex", "Codex"),
                ("claude", "Claude Code"),
                ("gemini", "Gemini CLI"),
            ] {
                ui.selectable_value(value, id.into(), label);
            }
        });
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
        let reply = if answers.is_some() {
            self.create
                .crew
                .workflow
                .models
                .get(&key)
                .and_then(|model| model.reply.clone())
        } else {
            self.create.chat.replies.get(&key).cloned()
        };
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
        let mut payload = json!({"projectId":self.create.project,"expectedProjectRevision":project_rev,"providerId":self.create.crew.workflow.provider,"title":title,"instruction":draft.goal});
        if !self.create.ability.is_empty() {
            payload["abilityId"] = json!(self.create.ability);
        }
        if let Some(reply) = &reply {
            let mut paths: Vec<_> = reply.references.iter().cloned().collect();
            paths.sort();
            payload["replyToWorkItemId"] = json!(reply.parent);
            payload["referencePaths"] = json!(paths);
        }
        if let Some(answers) = answers {
            payload["answers"] = answers;
        }
        let command = Command::new("project_crew_plan_start", payload);
        self.create.crew.workflow.request = Some(ModelRequest {
            command: text(&command.payload, "commandId").into(),
            key: key.clone(),
            draft,
            provider: self.create.crew.workflow.provider.clone(),
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
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(studio::title(t(ui, "Mox planner"), 20.0));
                status(ui, text(job, "status"));
                raw_hint(ui, text(job, "providerId"));
            });
            hint(
                ui,
                "This plan is generated by your selected provider. It does not start the crew.",
            );
            if !text(job, "failureReasonCode").is_empty() {
                hint(ui, text(job, "failureReasonCode"));
            }
            if self.create.draft().goal != model.draft.goal
                && control(ui, "Use planned goal", !model.draft.goal.is_empty()).clicked()
            {
                *self.create.draft() = model.draft.clone();
                if let Some(reply) = &model.reply {
                    self.create.chat.replies.insert(key.clone(), reply.clone());
                } else {
                    self.create.chat.replies.remove(&key);
                }
            }
            let state = self.create.crew.workflow.models.get_mut(&key).unwrap();
            if text(job, "status") == "needs_input" {
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
                let answers: Vec<_> = array(&job["questions"])
                    .iter()
                    .filter_map(|question| {
                        state
                            .answers
                            .get(text(question, "id"))
                            .map(|answer| json!({"id":question["id"],"answer":answer.trim()}))
                    })
                    .collect();
                if control(
                    ui,
                    "Send answers & replan",
                    answers.len() == array(&job["questions"]).len()
                        && !answers.is_empty()
                        && answers
                            .iter()
                            .all(|answer| valid_input(text(answer, "answer"), 2000)),
                )
                .clicked()
                {
                    self.begin_model_plan(snapshot, commands, Some(json!(answers)));
                }
            }
            if text(job, "status") == "completed" {
                ui.collapsing(t(ui, "Agent providers"), |ui| {
                    for (index, member) in self
                        .create
                        .crew
                        .plans
                        .get(&key)
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
                                    .get_mut(&key)
                                    .unwrap()
                                    .assignments
                                    .entry(text(member, "_uiId").into())
                                    .or_insert_with(|| text(job, "providerId").into()),
                            );
                        });
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                action(ui,commands,"Refresh plan","project_crew_plan_read",true,||json!({"projectId":key.0,"planJobId":job["planJobId"]}));
                action(ui,commands,"Cancel planning","project_crew_plan_cancel",matches!(text(job,"status"),"queued"|"running"),||json!({"projectId":key.0,"planJobId":job["planJobId"],"expectedPlanRevision":job["revision"]}));
            });
        });
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
        let ready = ui.is_enabled();
        let project = self.create.project.clone();
        let Some(_project_rev) = output_studio::project_revision(snapshot, &project) else {
            self.create.crew.workflow.open = false;
            return;
        };
        let read_key = (project.clone(), revision(&snapshot["projectWorkflows"]));
        if !self.hold_automatic_reads
            && ready
            && self.create.crew.workflow.read.as_ref() != Some(&read_key)
            && commands.is_empty()
        {
            self.create.crew.workflow.read = Some(read_key);
            commands.push(Command::new(
                "project_workflows_read",
                json!({"projectId":project}),
            ));
        }
        let plan_ready = self.create.crew.workflow.tab == 0
            && self
                .create
                .crew
                .workflow
                .models
                .get(&self.create.crew_key())
                .is_some_and(|model| text(&model.job, "status") == "completed");
        let id = egui::Id::new("mission-workflow-controls");
        let modal=egui::Modal::new(id).area(egui::Modal::default_area(id).fade_in(false)).show(ui.ctx(),|ui| {
            ui.set_width((ui.ctx().viewport_rect().width()-64.0).clamp(280.0,760.0));
            ui.add_enabled_ui(ready,|ui| {
                ui.label(studio::title(t(ui,"Crew workspace"),24.0));
                ui.horizontal_wrapped(|ui| {
                    for (index,label) in [(0,"Mox planner"),(1,"Direction & providers"),(2,"Recipes"),(3,"Schedules"),(4,"Activity & usage")] {ui.selectable_value(&mut self.create.crew.workflow.tab,index,t(ui,label));}
                });
                egui::ScrollArea::vertical().id_salt(("mission-workflow-body",&project,self.create.crew.workflow.tab)).auto_shrink([false,false]).stick_to_bottom(plan_ready).max_height((ui.ctx().viewport_rect().height()-220.0).max(120.0)).show(ui,|ui| {
                    match self.create.crew.workflow.tab {
                        0=>{
                            let has_model=self.create.crew.workflow.models.contains_key(&self.create.crew_key());
                            if has_model{ui.collapsing(t(ui,"Planning request"),|ui|self.workflow_plan_request(ui,snapshot,commands));}else{self.workflow_plan_request(ui,snapshot,commands);}
                            self.model_plan_card(ui,snapshot,commands);
                            if self.create.crew.plans.contains_key(&self.create.crew_key()){self.crew_plan_card(ui,snapshot,commands);}
                            ui.collapsing(t(ui,"Saved plans"),|ui| {
                                for saved in workflow_values(snapshot,&project,"plans") {
                                    ui.push_id(text(saved,"planJobId"),|ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(text(saved,"title"));status(ui,text(saved,"status"));
                                            if control(ui,"Open plan",true).clicked()&&commands.is_empty(){
                                                let key=self.create.crew_key();
                                                self.create.crew.plans.remove(&key);
                                                self.create.crew.workflow.models.insert(key,ModelPlan{job:saved.clone(),draft:CreateDraft::default(),reply:None,answers:HashMap::new(),assignments:HashMap::new(),read_revision:None});
                                                commands.push(Command::new("project_crew_plan_read",json!({"projectId":project,"planJobId":saved["planJobId"]})));
                                            }
                                        });
                                    });
                                }
                            });

                        },
                        1=>self.workflow_direction(ui,snapshot,commands),
                        2=>self.workflow_recipes(ui,snapshot,commands),
                        3=>self.workflow_schedules(ui,snapshot,commands),
                        _=>self.workflow_activity(ui,snapshot,commands),
                    }
                    let error=&snapshot["projectWorkflowError"];
                    if text(error,"projectId")==project{hint(ui,text(error,"reasonCode"));}
                    let receipt=&snapshot["projectWorkflowReceipt"];
                    if text(receipt,"projectId")==project && !text(receipt,"reasonCode").is_empty(){hint(ui,text(receipt,"reasonCode"));}
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    action(ui,commands,"Refresh","project_workflows_read",!self.hold_automatic_reads,||json!({"projectId":project}));
                    if control(ui,"Close",true).clicked(){self.create.crew.workflow.open=false;}
                });
            });
        });
        if modal.should_close() {
            self.create.crew.workflow.open = false;
        }
    }
    fn workflow_plan_request(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        hint(ui,"Ask a connected provider to plan this goal, or use the bundled guide planner without inference.");
        provider_picker(
            ui,
            "model-planner-provider",
            &mut self.create.crew.workflow.provider,
        );
        ui.label(t(ui, "Goal"));
        ui.add(
            egui::TextEdit::multiline(&mut self.create.draft().goal)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .char_limit(20_000),
        );
        hint(
            ui,
            "Planning calls the selected provider and can consume your account quota.",
        );
        let connected =
            connect::can_attempt_provider(snapshot, &self.create.crew.workflow.provider);
        if !connected {
            hint(ui, "Check the assigned providers before starting.");
            if control(ui, "Connections", true).clicked() {
                self.create.chat.connections_open = true;
                self.create.crew.workflow.open = false;
            }
        }
        let enabled = valid_input(&self.create.draft().goal, 20_000)
            && self.create.crew.workflow.request.is_none()
            && connected;
        if control(ui, "Generate plan", enabled).clicked() {
            self.begin_model_plan(snapshot, commands, None);
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
        ui.separator();
        hint(ui,"Choose affected tasks. Direction stops and replaces their unfinished work and descendants; earlier files remain.");
        for task in array(&mission["tasks"]) {
            let task_id = text(task, "taskId").to_owned();
            let mut selected = control_state.tasks.contains(&task_id);
            let label = if text(task, "title").is_empty() {
                text(task, "taskId")
            } else {
                text(task, "title")
            };
            if ui.checkbox(&mut selected, label).changed() {
                if selected {
                    control_state.tasks.insert(task_id);
                } else {
                    control_state.tasks.remove(&task_id);
                }
            }
        }
        multiline(
            ui,
            "New direction",
            &mut control_state.instruction,
            20_000,
            2,
        );
        let mut affected: Vec<_> = control_state.tasks.iter().cloned().collect();
        affected.sort();
        action(
            ui,
            commands,
            "Apply direction",
            "project_crew_steer",
            text(&mission, "status") == "active"
                && valid_input(&control_state.instruction, 20_000)
                && !affected.is_empty(),
            || json!({"projectId":project,"missionId":id,"expectedMissionRevision":mission["revision"],"instruction":control_state.instruction.trim(),"affectedTaskIds":affected}),
        );
        if !text(&mission["workflow"], "directionStatus").is_empty() {
            status(ui, text(&mission["workflow"], "directionStatus"));
        }
        if let Some(instruction) = mission["workflow"]["currentInstruction"].as_str() {
            ui.collapsing(t(ui, "Current direction"), |ui| {
                ui.add(egui::Label::new(instruction).wrap().selectable(true));
            });
        }
    }
    fn workflow_recipes(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let project = self.create.project.clone();
        hint(
            ui,
            "Recipes prepare a reusable crew. Starting always remains a separate action.",
        );
        if let Some(mission) = self.create.crew_mission(snapshot).cloned().or_else(|| {
            ordered_missions(snapshot, &project)
                .into_iter()
                .next_back()
                .cloned()
        }) {
            let state = self
                .create
                .crew
                .workflow
                .controls
                .entry((project.clone(), text(&mission, "missionId").into()))
                .or_default();
            field(ui, "Recipe name", &mut state.recipe_name, 80, false);
            action(
                ui,
                commands,
                "Save mission as recipe",
                "project_recipe_save",
                valid_input(&state.recipe_name, 80),
                || json!({"projectId":project,"missionId":mission["missionId"],"expectedMissionRevision":mission["revision"],"name":state.recipe_name.trim()}),
            );
        }
        for recipe in workflow_values(snapshot, &project, "recipes") {
            ui.push_id(text(recipe,"recipeId"),|ui|card(ui,|ui| {
                ui.label(studio::title(text(recipe,"name"),18.0));ui.label(text(recipe,"description"));
                ui.collapsing(t(ui,"Roster"),|ui|{
                    for member in array(&recipe["members"]){ui.label(text(member,"name"));hint(ui,text(member,"role"));}
                    for (key,label) in [("deliverables","Deliverables"),("toolRequirements","Required tools")] {
                        if !array(&recipe[key]).is_empty(){ui.label(RichText::new(t(ui,label)).strong());for value in array(&recipe[key]){if let Some(value)=value.as_str(){ui.label(value);}else{ui.label(value.to_string());}}}
                    }
                });
                let key=(project.clone(),text(recipe,"recipeId").into());
                let goal=self.create.crew.workflow.recipe_goal.entry(key).or_default();
                multiline(ui,"New goal (optional)",goal,20_000,2);
                let mut payload=json!({"projectId":project,"expectedProjectRevision":output_studio::project_revision(snapshot,&project),"recipeId":recipe["recipeId"],"expectedRecipeRevision":recipe["revision"]});
                if !goal.trim().is_empty(){payload["instruction"]=json!(goal.trim());}
                action(ui,commands,"Prepare recipe","project_recipe_run",revision(recipe)>0,||payload);
                if control(ui,"Schedule recipe",revision(recipe)>0).clicked(){self.create.crew.workflow.recipe.insert(project.clone(),text(recipe,"recipeId").into());self.create.crew.workflow.tab=3;}
            }));
        }
        self.workflow_page_controls(ui, snapshot, commands, "recipes");
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
            if control(ui,"Previous",offset>=20).clicked()&&commands.is_empty(){commands.push(Command::new("project_workflows_read",json!({"projectId":self.create.project,"section":section,"offset":offset.saturating_sub(20),"limit":20})));}
            if control(ui,"Next",if same{page["nextOffset"].is_u64()}else{values.len()>=20}).clicked()&&commands.is_empty(){commands.push(Command::new("project_workflows_read",json!({"projectId":self.create.project,"section":section,"offset":page["nextOffset"].as_u64().filter(|_|same).unwrap_or(offset+20),"limit":20})));}
        }));
    }
    fn workflow_schedules(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let project = self.create.project.clone();
        hint(ui,"Schedules run only while Moxcrew is awake. Missed intervals are skipped without a catch-up storm.");
        let recipes = workflow_values(snapshot, &project, "recipes");
        let state = &mut self.create.crew.workflow;
        let selected = state.recipe.entry(project.clone()).or_default();
        egui::ComboBox::from_id_salt("schedule-recipe")
            .selected_text(
                recipes
                    .iter()
                    .find(|recipe| text(recipe, "recipeId") == selected)
                    .map(|recipe| text(recipe, "name"))
                    .map(std::borrow::Cow::Borrowed)
                    .unwrap_or_else(|| t(ui, "Choose recipe")),
            )
            .show_ui(ui, |ui| {
                for recipe in &recipes {
                    ui.selectable_value(
                        selected,
                        text(recipe, "recipeId").into(),
                        text(recipe, "name"),
                    );
                }
            });
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
        let valid_date = chrono::DateTime::parse_from_rfc3339(&state.first_run).is_ok();
        if let Some(recipe) = recipe {
            action(
                ui,
                commands,
                "Create enabled schedule",
                "project_schedule_save",
                valid_date,
                || json!({"projectId":project,"expectedProjectRevision":output_studio::project_revision(snapshot,&project),"recipeId":recipe["recipeId"],"expectedRecipeRevision":recipe["revision"],"intervalMinutes":state.schedule_interval,"firstRunAt":state.first_run,"maxOccurrences":state.schedule_count,"budgets":state.schedule_budgets}),
            );
        }
        for schedule in workflow_values(snapshot, &project, "schedules") {
            ui.push_id(text(schedule,"scheduleId"),|ui|card(ui,|ui|{
                ui.label(text(schedule,"scheduleId"));hint(ui,if schedule["enabled"]==true{"Enabled"}else{"Paused"});
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
                if ui
                    .selectable_value(&mut state.section, key.into(), t(ui, label))
                    .clicked()
                {
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
                    ui.label(text(item, "missionId"));
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
                        ui.label(bottleneck.as_str().unwrap_or(""));
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
            if control(ui, "Previous", state.offset >= 20).clicked() && commands.is_empty() {
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
    fn model_planning_acceptance_preserves_async_scope_answers_and_manual_member_providers() {
        let mut data = fixture();
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
                    for _ in 0..4 {
                        assert!(render(&ctx, &mut console, &data, size, false, vec![])
                            .0
                            .is_empty());
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
    fn workflow_schedules_usage_and_minimum_modal_are_explicit_truthful_and_accessible() {
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                for tab in 0..5 {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut data = fixture();
                    data["desktopPreferences"]["language"] = json!(language);
                    data["projectWorkflows"] = json!({"revision":7,"plans":[],"recipes":[{"recipeId":"recipe:one","projectId":null,"revision":2,"name":"Research brief","description":"Bounded project recipe","members":plan().members,"settings":plan().settings}],"schedules":[],"usage":[{"missionId":"mission:one","projectId":"project:one","startedJobs":2,"elapsedSeconds":14,"inputTokens":null,"outputTokens":null,"costMicros":null,"measurementComplete":false}]});
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
                        assert!(copy.contains(if language == "en" {
                            "Unknown"
                        } else {
                            "Bilinmiyor"
                        }));
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
