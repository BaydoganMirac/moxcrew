use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateExportRequest {
    pub project_id: String,
    pub work_item_id: String,
    pub path: String,
    pub name: String,
}

#[derive(Clone, Default, PartialEq, Eq)]
struct CreateDraft {
    title: String,
    goal: String,
}

struct Submission {
    command_id: String,
    project: String,
    ability: String,
    draft: CreateDraft,
}

#[derive(Default)]
pub(super) struct CreateState {
    project: String,
    ability: String,
    category: String,
    search: String,
    drafts: HashMap<(String, String), CreateDraft>,
    selected_work: HashMap<String, String>,
    result_path: String,
    view_results: bool,
    pending: Option<Submission>,
    export_request: Option<CreateExportRequest>,
}

impl CreateState {
    pub(super) fn sync(&mut self, snapshot: &Value) {
        let Some(pending) = &self.pending else { return };
        let receipt = &snapshot["projectWorkReceipt"];
        if text(receipt, "commandId") != pending.command_id
            || text(receipt, "projectId") != pending.project
        {
            return;
        }
        if text(receipt, "outcome") == "applied" {
            let work = text(receipt, "workItemId");
            if !job_belongs(snapshot, &pending.project, work) {
                return;
            }
            self.selected_work
                .insert(pending.project.clone(), work.into());
            let key = (pending.project.clone(), pending.ability.clone());
            // Only the exact acknowledged draft may be cleared after an async receipt.
            if self.drafts.get(&key) == Some(&pending.draft) {
                self.drafts.remove(&key);
            }
            if self.project == pending.project {
                self.result_path.clear();
                self.view_results = true;
            }
        }
        self.pending = None;
    }

    fn draft(&mut self) -> &mut CreateDraft {
        self.drafts
            .entry((self.project.clone(), self.ability.clone()))
            .or_default()
    }

    fn payload(&self, snapshot: &Value) -> Option<Value> {
        let project = array(&snapshot["projectMemory"]["projects"])
            .iter()
            .find(|p| text(p, "projectId") == self.project && revision(p) > 0)?;
        if !array(&snapshot["abilities"])
            .iter()
            .any(|a| text(a, "id") == self.ability)
        {
            return None;
        }
        let draft = self
            .drafts
            .get(&(self.project.clone(), self.ability.clone()))?;
        if !valid_input(&draft.title, 160) || !valid_input(&draft.goal, 50_000) {
            return None;
        }
        Some(
            json!({"projectId":self.project,"expectedProjectRevision":project["revision"],
            "abilityId":self.ability,"title":draft.title.trim(),"instruction":draft.goal.trim()}),
        )
    }

    fn selected_work<'a>(&self, snapshot: &'a Value) -> Option<&'a Value> {
        let id = self.selected_work.get(&self.project)?;
        if !job_belongs(snapshot, &self.project, id) {
            return None;
        }
        items(&snapshot["work"])
            .iter()
            .find(|w| text(w, "workItemId") == id)
    }

    fn scoped<'a>(&self, snapshot: &'a Value, field: &str, reason: &str) -> Option<&'a Value> {
        let work = self.selected_work(snapshot)?;
        if job_issue(snapshot, &self.project, text(work, "workItemId")).is_some() {
            return None;
        }
        let value = &snapshot[field];
        (text(value, "reasonCode") == reason
            && text(value, "projectId") == self.project
            && value["workItemId"] == work["workItemId"])
            .then_some(value)
    }
}

fn valid_input(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && utf16_len(value.trim()) <= max && !value.contains('\0')
}

fn jobs(snapshot: &Value) -> &[Value] {
    snapshot["projectJobs"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| items(&snapshot["projectJobs"]))
}

pub(super) fn job_belongs(snapshot: &Value, project: &str, work: &str) -> bool {
    !project.is_empty()
        && !work.is_empty()
        && jobs(snapshot)
            .iter()
            .any(|job| text(job, "projectId") == project && text(job, "workItemId") == work)
}

fn job_issue<'a>(snapshot: &'a Value, project: &str, work: &str) -> Option<&'a str> {
    jobs(snapshot)
        .iter()
        .find(|job| text(job, "projectId") == project && text(job, "workItemId") == work)
        .and_then(|job| job["runtimeIssue"].as_str())
        .filter(|issue| !issue.is_empty())
}

fn ability_text<'a>(ability: &'a Value, field: &str, language: Language) -> &'a str {
    ability[field][language.code()]
        .as_str()
        .or_else(|| ability[field]["en"].as_str())
        .or_else(|| ability[field].as_str())
        .unwrap_or("")
}

fn category_name(category: &str) -> &str {
    match category {
        "presentations" => "Presentations",
        "video" => "Video",
        "images" => "Images",
        "audio" => "Audio",
        "research" => "Research",
        "documents" => "Documents",
        "data" => "Data",
        "web" => "Web & actions",
        _ => category,
    }
}

fn matches_ability(ability: &Value, category: &str, query: &str, language: Language) -> bool {
    (category.is_empty() || text(ability, "category") == category)
        && (query.trim().is_empty()
            || format!(
                "{} {} {} {}",
                text(ability, "id"),
                ability_text(ability, "name", language),
                ability_text(ability, "description", language),
                i18n::tr(language, category_name(text(ability, "category")))
            )
            .to_lowercase()
            .contains(&query.trim().to_lowercase()))
}

impl Console {
    #[cfg(feature = "ui-smoke")]
    pub fn create_smoke(&mut self, snapshot: &Value, show_results: bool) {
        self.tab = Tab::Create;
        self.create.project = array(&snapshot["projectMemory"]["projects"])
            .first()
            .map(|project| text(project, "projectId").to_owned())
            .unwrap_or_default();
        self.create.ability = "P1".into();
        self.create.view_results = show_results;
        self.create.result_path = text(&snapshot["projectResult"], "path").into();
        if let Some(ability) = array(&snapshot["abilities"])
            .iter()
            .find(|ability| text(ability, "id") == "P1")
        {
            let language = Language::from_snapshot(snapshot);
            *self.create.draft() = CreateDraft {
                title: ability_text(ability, "name", language).into(),
                goal: ability_text(ability, "starter", language).into(),
            };
        }
        if let Some(job) = jobs(snapshot)
            .iter()
            .find(|job| text(job, "projectId") == self.create.project)
        {
            self.create
                .selected_work
                .insert(self.create.project.clone(), text(job, "workItemId").into());
        }
    }

    pub fn take_create_export_request(&mut self) -> Option<CreateExportRequest> {
        self.create.export_request.take()
    }

    pub fn create_export_command(
        request: CreateExportRequest,
        destination: Option<String>,
    ) -> Option<Command> {
        let destination = destination.filter(|value| !value.is_empty())?;
        Some(Command::new(
            "project_result_export",
            json!({"projectId":request.project_id,
            "workItemId":request.work_item_id,"path":request.path,"targetPath":destination}),
        ))
    }

    pub(super) fn create_studio(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if !self.create.view_results || ui.ctx().viewport_rect().height() >= 700.0 {
            egui::Frame::new()
                .fill(colors(ui).info_surface)
                .corner_radius(14)
                .inner_margin(14)
                .show(ui, |ui| {
                    ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                    if ui.ctx().viewport_rect().height() >= 700.0 {
                        ui.label(
                            RichText::new(t(ui, "CREATE WITH MOX"))
                                .size(10.0)
                                .color(colors(ui).primary)
                                .extra_letter_spacing(1.4),
                        );
                        ui.add_space(4.0);
                    }
                    ui.label(studio::title(t(ui, "One idea. A whole crew."), 26.0));
                    ui.add_space(4.0);
                    hint(ui, "Give Mox a goal. Review the result.");
                });
            ui.add_space(10.0);
        }
        let projects = array(&snapshot["projectMemory"]["projects"]);
        if projects.is_empty() {
            empty(
                ui,
                "Create a project first",
                "Give your ideas a home. No repository or folder is required here.",
            );
            if submit(ui, "Create project", "create-project") {
                self.new_project_open = true;
            }
            return;
        }
        if self.create.project.is_empty() && projects.len() == 1 {
            self.create.project = text(&projects[0], "projectId").into();
        }
        let previous = self.create.project.clone();
        project_switcher(
            ui,
            "create-project-picker",
            &mut self.create.project,
            projects,
        );
        if previous != self.create.project {
            self.create.result_path.clear();
        }
        if !projects
            .iter()
            .any(|p| text(p, "projectId") == self.create.project)
        {
            hint(ui, "Choose the project for this idea.");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.create.view_results, false, t(ui, "New creation"));
            ui.selectable_value(
                &mut self.create.view_results,
                true,
                t(ui, "Project creations"),
            );
        });
        ui.add_space(12.0);
        if self.create.view_results {
            self.create_jobs(ui, snapshot, commands);
            return;
        }
        let abilities = array(&snapshot["abilities"]);
        if abilities.is_empty() {
            empty(
                ui,
                "Ability catalog unavailable",
                "Refresh the local host to load the built-in task recipes.",
            );
            action(ui, commands, "Refresh", "observe", true, || json!({}));
            return;
        }
        if self.create.ability.is_empty() {
            self.create.ability = text(&abilities[0], "id").into();
        }
        if ui.available_width() >= 850.0 {
            ui.columns(2, |columns| {
                self.create_catalog(
                    &mut columns[0],
                    abilities,
                    Language::from_snapshot(snapshot),
                );
                self.create_composer(&mut columns[1], snapshot, commands);
            });
        } else {
            egui::CollapsingHeader::new(t(ui, "Explore abilities"))
                .id_salt("create-catalog")
                .default_open(false)
                .show(ui, |ui| {
                    self.create_catalog(ui, abilities, Language::from_snapshot(snapshot))
                });
            ui.add_space(14.0);
            self.create_composer(ui, snapshot, commands);
        }
    }

    fn create_catalog(&mut self, ui: &mut Ui, abilities: &[Value], language: Language) {
        card(ui, |ui| {
            section_title(
                ui,
                "What would you like to make?",
                "Task recipes guide your agent. Results depend on available tools and access.",
            );
            ui.add_space(10.0);
            field(ui, "Search abilities", &mut self.create.search, 200, false);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(self.create.category.is_empty(), t(ui, "All abilities"))
                    .clicked()
                {
                    self.create.category.clear();
                }
                for category in [
                    "presentations",
                    "video",
                    "images",
                    "audio",
                    "research",
                    "documents",
                    "data",
                    "web",
                ] {
                    if ui
                        .selectable_label(
                            self.create.category == category,
                            t(ui, category_name(category)),
                        )
                        .clicked()
                    {
                        self.create.category = category.into();
                    }
                }
            });
            ui.add_space(10.0);
            let matching: Vec<_> = abilities
                .iter()
                .filter(|ability| {
                    matches_ability(
                        ability,
                        &self.create.category,
                        &self.create.search,
                        language,
                    )
                })
                .collect();
            if matching.is_empty() {
                hint(ui, "No matching abilities. Try another search or category.");
            }
            egui::ScrollArea::vertical()
                .id_salt("create-ability-list")
                .max_height(240.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for ability in matching {
                        ui.push_id(text(ability, "id"), |ui| {
                            let selected = self.create.ability == text(ability, "id");
                            let label = ability_text(ability, "name", language);
                            let response = ui.add(
                                egui::Button::selectable(selected, RichText::new(label).strong())
                                    .wrap()
                                    .min_size(egui::vec2(ui.available_width(), 34.0)),
                            );
                            #[cfg(test)]
                            ui.data_mut(|data| {
                                data.insert_temp(
                                    egui::Id::new(("test-action", "ability", text(ability, "id"))),
                                    response.rect,
                                )
                            });
                            if response.clicked() {
                                self.create.ability = text(ability, "id").into();
                            }
                            raw_hint(ui, ability_text(ability, "description", language));
                            ui.add_space(9.0);
                        });
                    }
                });
        });
    }

    fn create_composer(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let language = Language::from_snapshot(snapshot);
        let Some(ability) = array(&snapshot["abilities"])
            .iter()
            .find(|a| text(a, "id") == self.create.ability)
        else {
            hint(ui, "Choose an ability to shape your task.");
            return;
        };
        card(ui, |ui| {
            let payload = self.create.payload(snapshot);
            ui.horizontal_wrapped(|ui| {
                status(ui, "Task recipe");
                ui.add_space(8.0);
                ui.add_enabled_ui(payload.is_some() && self.create.pending.is_none(), |ui| {
                    if submit(ui, "Queue project task", "project_work_enqueue")
                        && commands.is_empty()
                    {
                        if let Some(payload) = payload {
                            let command = Command::new("project_work_enqueue", payload);
                            self.create.pending = Some(Submission {
                                command_id: text(&command.payload, "commandId").into(),
                                project: self.create.project.clone(),
                                ability: self.create.ability.clone(),
                                draft: self.create.draft().clone(),
                            });
                            commands.push(command);
                        }
                    }
                });
            });
            ui.add_space(8.0);
            ui.label(studio::title(ability_text(ability, "name", language), 21.0));
            raw_hint(ui, ability_text(ability, "description", language));
            ui.add_space(12.0);
            let draft = self.create.draft();
            field(ui, "Task name", &mut draft.title, 160, false);
            multiline(ui, "What should Mox deliver?", &mut draft.goal, 50_000, 4);
            if control(ui, "Use example", true).clicked() {
                self.create.draft().goal = ability_text(ability, "starter", language).into();
            }
            egui::CollapsingHeader::new(t(ui, "Recipe instructions"))
                .id_salt("create-instructions")
                .show(ui, |ui| {
                    ui.label(text(ability, "instruction"));
                });
            ui.add_space(10.0);
            ui.add_space(8.0);
            hint(
                ui,
                "Queueing does not start the agent. Review the task, then start it explicitly.",
            );
            hint(ui, "Your provider may receive this goal and the accepted decisions for this project when you start.");
            if !quick_codex_ready(snapshot) {
                ui.add_space(6.0);
                hint(ui, "Starting project tasks requires supported Codex access. Your draft can be queued first.");
                if control(ui, "Open providers", true).clicked() {
                    self.tab = Tab::Providers;
                }
            }
        });
    }

    fn create_jobs(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        section_title(
            ui,
            "Project creations",
            "Follow the task, inspect actual files and export a copy.",
        );
        ui.add_space(8.0);
        let project_jobs: Vec<_> = items(&snapshot["work"])
            .iter()
            .filter(|w| job_belongs(snapshot, &self.create.project, text(w, "workItemId")))
            .collect();
        if project_jobs.is_empty() {
            hint(
                ui,
                "No creations in this project yet. Open New creation to begin.",
            );
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(t(ui, "Task"));
            let selected = self
                .create
                .selected_work
                .entry(self.create.project.clone())
                .or_default();
            let before = selected.clone();
            let name = project_jobs
                .iter()
                .find(|job| text(job, "workItemId") == selected)
                .map(|job| std::borrow::Cow::Borrowed(text(job, "title")))
                .unwrap_or_else(|| t(ui, "Choose…"));
            egui::ComboBox::from_id_salt("create-task-picker")
                .selected_text(name)
                .truncate()
                .width((ui.available_width() - 6.0).clamp(150.0, 420.0))
                .show_ui(ui, |ui| {
                    for job in &project_jobs {
                        ui.selectable_value(
                            selected,
                            text(job, "workItemId").into(),
                            text(job, "title"),
                        );
                    }
                });
            if *selected != before {
                self.create.result_path.clear();
            }
        });
        let Some(work) = self.create.selected_work(snapshot) else {
            hint(ui, "Choose a task to inspect its outputs.");
            return;
        };
        let runtime_issue = job_issue(snapshot, &self.create.project, text(work, "workItemId"));
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(text(work, "title"));
                status(ui, text(work, "status"));
            });
            ui.collapsing(t(ui, "Task goal"), |ui| {
                ui.label(text(work, "instruction"));
            });
            if let Some(issue) = runtime_issue {
                ui.colored_label(
                    colors(ui).warning,
                    t(ui, "This task's output folder needs attention."),
                );
                hint(ui, issue);
            }
            #[cfg(target_os = "windows")]
            hint(ui, "PROJECT_RESULTS_UNSUPPORTED_PLATFORM");
            ui.horizontal_wrapped(|ui| {
                if text(work, "status") == "queued" {
                    action(ui, commands, "Start task", "work_start", runtime_issue.is_none() && quick_codex_ready(snapshot) && revision(work) > 0,
                        || json!({"workItemId":work["workItemId"],"expectedItemRevision":work["revision"]}));
                }
                if matches!(text(work, "status"), "failed" | "interrupted" | "blocked") {
                    action(ui, commands, "Queue retry", "work_retry", revision(work) > 0,
                        || json!({"workItemId":work["workItemId"],"expectedItemRevision":work["revision"]}));
                }
                if let Some(run) = array(&snapshot["work"]["runs"]).iter().rev()
                    .find(|run| run["workItemId"] == work["workItemId"] && text(run, "status") == "running") {
                    action(ui, commands, "Cancel run", "work_cancel", codex_ready(snapshot)
                        && run["ownerEpoch"] == snapshot["hostEpoch"] && revision(run) > 0,
                        || json!({"runId":run["runId"],"expectedRunRevision":run["revision"]}));
                }
                action(ui, commands, "Refresh outputs", "project_results_read", !cfg!(target_os = "windows") && runtime_issue.is_none() && !self.hold_automatic_reads,
                    || json!({"projectId":self.create.project,"workItemId":work["workItemId"]}));
            });
            if runtime_issue.is_some() || cfg!(target_os = "windows") {
                return;
            }
            ui.add_space(10.0);
            hint(ui, "Files may be partial while a task is running or after a failure. Review them before use.");
            let Some(results) =
                self.create
                    .scoped(snapshot, "projectResults", "PROJECT_RESULTS_READ")
            else {
                hint(ui, "Refresh outputs to list the files for this task.");
                return;
            };
            if array(&results["items"]).is_empty() {
                hint(
                    ui,
                    "No output files yet. The task may still be working or need attention.",
                );
            }
            if results["truncated"] == true {
                hint(
                    ui,
                    "Output list limit reached. Only the first files are shown.",
                );
            }
            for result in array(&results["items"]) {
                ui.push_id(text(result, "path"), |ui| {
                    ui.separator();
                    ui.label(RichText::new(text(result, "name")).strong());
                    raw_hint(ui, &format!("{} · {} B", text(result, "path"), result["bytes"].as_u64().unwrap_or(0)));
                    ui.horizontal_wrapped(|ui| {
                        if result["textPreview"] == true {
                            let before = commands.len();
                            action(ui, commands, "Preview text", "project_result_read", true,
                                || json!({"projectId":self.create.project,"workItemId":work["workItemId"],"path":result["path"]}));
                            if commands.len() > before { self.create.result_path = text(result, "path").into(); }
                        } else { hint(ui, "Export to open in its app."); }
                        if control(ui, "Export copy…", self.create.export_request.is_none()).clicked() {
                            self.create.export_request = Some(CreateExportRequest { project_id: self.create.project.clone(),
                                work_item_id: text(work, "workItemId").into(), path: text(result, "path").into(), name: text(result, "name").into() });
                        }
                    });
                });
            }
            if let Some(detail) = self
                .create
                .scoped(snapshot, "projectResult", "PROJECT_RESULT_READ")
                .filter(|detail| {
                    !self.create.result_path.is_empty()
                        && text(detail, "path") == self.create.result_path
                        && array(&results["items"]).iter().any(|item| {
                            item["path"] == detail["path"] && item["textPreview"] == true
                        })
                })
            {
                ui.add_space(12.0);
                ui.separator();
                ui.label(RichText::new(t(ui, "Output preview")).strong());
                raw_hint(ui, text(detail, "path"));
                egui::ScrollArea::vertical()
                    .id_salt("create-output-preview")
                    .max_height(280.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(text(detail, "text")).monospace().size(12.0),
                            )
                            .wrap()
                            .selectable(true),
                        );
                    });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Value {
        json!({"projectMemory":{"projects":[{"projectId":"project:one","revision":3,"name":"Owner project"},
            {"projectId":"project:two","revision":4,"name":"Second project"}]},
            "abilities":[{"id":"P1","category":"presentations","name":{"en":"Presentation","tr":"Sunum"},
                "description":{"en":"Build a deck","tr":"Sunum hazırlayın"},"starter":{"en":"Present my idea","tr":"Fikrimi sun"},"instruction":"Prepare useful files."},
                {"id":"V1","category":"video","name":{"en":"Edit video","tr":"Video düzenle"},"description":{"en":"Trim a recording","tr":"Kaydı kesin"}}],
            "projectJobs":[{"projectId":"project:one","workItemId":"work:one"}],
            "work":{"items":[{"workItemId":"work:one","title":"Existing work","revision":2,"status":"queued"}]}})
    }

    fn state() -> CreateState {
        let mut state = CreateState {
            project: "project:one".into(),
            ability: "P1".into(),
            ..Default::default()
        };
        *state.draft() = CreateDraft {
            title: "Owner title".into(),
            goal: "Owner goal".into(),
        };
        state
    }

    fn render(
        ctx: &egui::Context,
        console: &mut Console,
        snapshot: &Value,
        size: egui::Vec2,
        busy: bool,
        events: Vec<egui::Event>,
    ) -> (Vec<Command>, Vec<String>) {
        let mut commands = Vec::new();
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ui| commands = console.show(ui, snapshot, busy, None, false),
        );
        fn text_shapes(shape: &egui::Shape, copy: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => copy.push(text.galley.text().into()),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text_shapes(shape, copy);
                    }
                }
                _ => {}
            }
        }
        let mut copy = Vec::new();
        for shape in &output.shapes {
            text_shapes(&shape.shape, &mut copy);
        }
        output.drop_without_applying_deltas();
        (commands, copy)
    }

    fn click(
        ctx: &egui::Context,
        console: &mut Console,
        data: &Value,
        busy: bool,
        action: &str,
        label: &str,
    ) -> Vec<Command> {
        let size = egui::vec2(1280.0, 2200.0);
        render(ctx, console, data, size, busy, vec![]);
        let rect = ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new(("test-action", action, label)))
                .expect("action rendered")
        });
        let position = rect.center();
        render(
            ctx,
            console,
            data,
            size,
            busy,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
        render(
            ctx,
            console,
            data,
            size,
            busy,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            }],
        )
        .0
    }

    #[test]
    fn create_screen_only_queues_after_explicit_click_and_preserves_busy_draft() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = Console {
            tab: Tab::Create,
            create: state(),
            ..Default::default()
        };
        let mut data = snapshot();
        data["desktopPreferences"] = json!({"language":"en"});
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 2200.0),
            false,
            vec![],
        );
        assert!(
            commands.is_empty(),
            "opening the studio must not enqueue or start work"
        );
        assert!(copy.join("\n").contains("One idea. A whole crew."));
        let commands = click(
            &ctx,
            &mut console,
            &data,
            true,
            "project_work_enqueue",
            "Queue project task",
        );
        assert!(commands.is_empty());
        assert_eq!(console.create.draft().goal, "Owner goal");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_work_enqueue",
            "Queue project task",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_work_enqueue");
        assert_eq!(commands[0].payload["projectId"], "project:one");
        assert!(commands[0].payload["workingDirectory"].is_null());
        assert!(commands
            .iter()
            .all(|command| command.action != "work_start"));
        assert_eq!(
            console.create.draft().goal,
            "Owner goal",
            "only an applied matching receipt may clear this draft"
        );
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_work_enqueue",
            "Queue project task"
        )
        .is_empty());
    }

    #[test]
    fn damaged_previous_folder_allows_explicit_retry_of_exact_failed_job_only() {
        for issue in [
            "PROJECT_JOB_DIRECTORY_UNAVAILABLE",
            "PROJECT_JOB_DIRECTORY_UNSAFE",
        ] {
            for status in ["failed", "interrupted", "blocked"] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                let mut console = Console {
                    tab: Tab::Create,
                    create: state(),
                    ..Default::default()
                };
                let mut data = snapshot();
                data["desktopPreferences"] = json!({"language":"en"});
                data["projectJobs"][0]["runtimeIssue"] = json!(issue);
                data["work"]["items"][0]["status"] = json!(status);
                data["observation"] = json!({"observations":[{"provider":{"id":"codex","state":"available","version":"0.155.1",
                    "capabilities":{"recentObservation":true,"explicitLiveStatus":true}},"sessions":[]}]});
                assert!(quick_codex_ready(&data));
                console.create.view_results = true;
                console
                    .create
                    .selected_work
                    .insert("project:one".into(), "work:one".into());
                let commands = click(
                    &ctx,
                    &mut console,
                    &data,
                    false,
                    "work_retry",
                    "Queue retry",
                );
                assert_eq!(
                    commands.len(),
                    1,
                    "{status} {issue} must allow safe recovery"
                );
                assert_eq!(commands[0].action, "work_retry");
                assert_eq!(commands[0].payload["workItemId"], "work:one");
                assert_eq!(commands[0].payload["expectedItemRevision"], 2);
                assert!(
                    click(&ctx, &mut console, &data, true, "work_retry", "Queue retry").is_empty()
                );
                assert!(click(
                    &ctx,
                    &mut console,
                    &data,
                    false,
                    "project_results_read",
                    "Refresh outputs"
                )
                .is_empty());
                data["work"]["items"][0]["status"] = json!("queued");
                assert!(
                    click(&ctx, &mut console, &data, false, "work_start", "Start task").is_empty()
                );
                let copy = render(
                    &ctx,
                    &mut console,
                    &data,
                    egui::vec2(1280.0, 1200.0),
                    false,
                    vec![],
                )
                .1
                .join("\n");
                assert!(!copy.contains("Preview text") && !copy.contains("Export copy…"));
                assert!(console.take_create_export_request().is_none());
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_build_discloses_unavailable_outputs_and_hides_cached_files() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = Console {
            tab: Tab::Create,
            create: state(),
            ..Default::default()
        };
        let mut data = snapshot();
        data["desktopPreferences"] = json!({"language":"en"});
        console.create.view_results = true;
        console
            .create
            .selected_work
            .insert("project:one".into(), "work:one".into());
        data["projectResults"] = json!({"reasonCode":"PROJECT_RESULTS_READ","projectId":"project:one","workItemId":"work:one",
            "items":[{"path":"outputs/report.md","name":"CACHED_FILE_CANARY","bytes":12,"textPreview":true}]});
        let copy = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 1200.0),
            false,
            vec![],
        )
        .1
        .join("\n");
        assert!(copy.contains("Output browsing and export are unavailable on Windows"));
        assert!(!copy.contains("CACHED_FILE_CANARY") && !copy.contains("Export copy…"));
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_results_read",
            "Refresh outputs"
        )
        .is_empty());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn text_preview_and_export_controls_only_use_exact_selected_project_job() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = Console {
            tab: Tab::Create,
            create: state(),
            ..Default::default()
        };
        let mut data = snapshot();
        data["desktopPreferences"] = json!({"language":"en"});
        console
            .create
            .selected_work
            .insert("project:one".into(), "work:one".into());
        console.create.view_results = true;
        data["projectResults"] = json!({"reasonCode":"PROJECT_RESULTS_READ","projectId":"project:two","workItemId":"work:one",
            "items":[{"path":"outputs/report.md","name":"FOREIGN_RESULT_CANARY","bytes":12,"textPreview":true}]});
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
        assert!(!copy.contains("FOREIGN_RESULT_CANARY"));
        data["projectResults"]["projectId"] = json!("project:one");
        data["projectResults"]["items"][0]["name"] = json!("Owner report.md");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_result_read",
            "Preview text",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].payload["path"], "outputs/report.md");
        assert_eq!(commands[0].payload["workItemId"], "work:one");
        data["projectResult"] = json!({"reasonCode":"PROJECT_RESULT_READ","projectId":"project:one","workItemId":"work:one",
            "path":"outputs/other.md","text":"FOREIGN_PREVIEW_CANARY"});
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
        assert!(!copy.contains("FOREIGN_PREVIEW_CANARY"));
        data["projectResult"]["path"] = json!("outputs/report.md");
        data["projectResult"]["text"] = json!("OWNER_TEXT Projects Memory");
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
        assert!(
            copy.contains("OWNER_TEXT Projects Memory"),
            "path={}\n{copy}",
            console.create.result_path
        );
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Export copy…"
        )
        .is_empty());
        let request = console.take_create_export_request().unwrap();
        assert_eq!(request.project_id, "project:one");
        assert_eq!(request.path, "outputs/report.md");
        assert!(console.take_create_export_request().is_none());
        data["projectJobs"][0]["runtimeIssue"] = json!("PROJECT_JOB_DIRECTORY_UNSAFE");
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
        assert!(copy.contains("This task's output folder needs attention."));
        assert!(
            !copy.contains("OWNER_TEXT Projects Memory"),
            "unavailable folders must not present cached files as current outputs"
        );
        assert!(!copy.contains("Export copy…"));
    }

    #[test]
    fn minimum_results_controls_are_visible_and_respect_busy_provider_state() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                for (provider_ready, busy) in [(true, false), (false, false), (true, true)] {
                    for (action, label) in [
                        ("work_start", "Start task"),
                        ("project_results_read", "Refresh outputs"),
                    ] {
                        let ctx = egui::Context::default();
                        studio::install(&ctx);
                        ctx.set_theme(theme);
                        let mut console = Console {
                            tab: Tab::Create,
                            create: state(),
                            ..Default::default()
                        };
                        let mut data = snapshot();
                        data["desktopPreferences"] = json!({"language":language});
                        data["work"]["items"][0]["title"] =
                            json!("Launch presentation · UI fixture");
                        data["observation"] = json!({"observations":[{"provider":{"id":"codex","state":if provider_ready {"available"} else {"unavailable"},"version":"0.155.1",
                            "capabilities":{"recentObservation":true,"explicitLiveStatus":true}},"sessions":[]}]});
                        console.create.view_results = true;
                        console
                            .create
                            .selected_work
                            .insert("project:one".into(), "work:one".into());
                        let (commands, copy) =
                            render(&ctx, &mut console, &data, size, busy, vec![]);
                        assert!(commands.is_empty());
                        assert!(!copy.iter().any(|text| text == "One idea. A whole crew."
                            || text == "Bir fikir. Koca bir ekip."));
                        let (enabled, rect, clip) = ctx.data(|data| {
                            data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                                "test-action-state",
                                action,
                                label,
                            )))
                            .unwrap()
                        });
                        let expected = !busy
                            && if action == "work_start" {
                                provider_ready
                            } else {
                                !cfg!(target_os = "windows")
                            };
                        assert_eq!(
                            enabled, expected,
                            "{language} {theme:?} {action} must reflect actual readiness"
                        );
                        assert!(
                            screen.contains_rect(rect),
                            "{language} {theme:?} {action} outside viewport: {rect:?}"
                        );
                        assert!(
                            clip.contains_rect(rect),
                            "{language} {theme:?} {action} clipped: {rect:?} {clip:?}"
                        );
                        let position = rect.center();
                        render(
                            &ctx,
                            &mut console,
                            &data,
                            size,
                            busy,
                            vec![
                                egui::Event::PointerMoved(position),
                                egui::Event::PointerButton {
                                    pos: position,
                                    button: egui::PointerButton::Primary,
                                    pressed: true,
                                    modifiers: egui::Modifiers::default(),
                                },
                            ],
                        );
                        let (commands, _) = render(
                            &ctx,
                            &mut console,
                            &data,
                            size,
                            busy,
                            vec![egui::Event::PointerButton {
                                pos: position,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers: egui::Modifiers::default(),
                            }],
                        );
                        assert_eq!(
                            commands.len(),
                            usize::from(expected),
                            "{language} {theme:?} {action} click must respect readiness"
                        );
                        if let Some(command) = commands.first() {
                            assert_eq!(command.action, action);
                            assert_eq!(command.payload["workItemId"], "work:one");
                            if action == "work_start" {
                                assert_eq!(command.payload["expectedItemRevision"], 2);
                            } else {
                                assert_eq!(command.payload["projectId"], "project:one");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn minimum_viewport_keeps_valid_draft_queue_action_visible_and_clickable() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut console = Console {
                    tab: Tab::Create,
                    create: state(),
                    ..Default::default()
                };
                let mut data = snapshot();
                data["desktopPreferences"] = json!({"language":language});
                data["abilities"] = patchfleet_core::abilities::catalog();
                let (commands, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                assert!(commands.is_empty());
                assert!(
                    !copy.iter().any(|text| text == "P1"),
                    "internal ability IDs are not user guidance"
                );
                let (enabled, rect, clip) = ctx.data(|data| {
                    data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                        "test-action-state",
                        "project_work_enqueue",
                        "Queue project task",
                    )))
                    .unwrap()
                });
                assert!(enabled, "a valid draft must enable queue");
                assert!(
                    screen.contains_rect(rect),
                    "{language} {theme:?}: action outside viewport: {rect:?}"
                );
                assert!(
                    clip.contains_rect(rect),
                    "{language} {theme:?}: action clipped: {rect:?} {clip:?}"
                );
                let position = rect.center();
                render(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::default(),
                        },
                    ],
                );
                let (commands, _) = render(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    vec![egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::default(),
                    }],
                );
                assert_eq!(
                    commands.len(),
                    1,
                    "visible action must receive a real click"
                );
                assert_eq!(commands[0].action, "project_work_enqueue");
                assert_eq!(commands[0].payload["instruction"], "Owner goal");
                assert_eq!(console.create.draft().goal, "Owner goal");
            }
        }
    }

    #[test]
    fn fresh_console_opens_general_create_with_visible_project_action() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut console = Console::default();
                let data = json!({"desktopPreferences":{"language":language}});
                let (commands, copy) = render(
                    &ctx,
                    &mut console,
                    &data,
                    egui::vec2(800.0, 560.0),
                    false,
                    vec![],
                );
                assert!(commands.is_empty());
                assert!(console.tab == Tab::Create);
                assert!(!console.quick_start.open);
                assert!(console.folder_request.is_none());
                assert!(!copy.join("\n").contains("Git"));
                assert!(copy.join("\n").contains(if language == "tr" {
                    "Bir fikir. Koca bir ekip."
                } else {
                    "One idea. A whole crew."
                }));
                let action = ctx.data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "create-project",
                        "Create project",
                    )))
                    .unwrap()
                });
                assert!(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 560.0))
                        .contains_rect(action),
                    "first action clipped: {action:?}"
                );
                assert!(click(
                    &ctx,
                    &mut console,
                    &data,
                    false,
                    "create-project",
                    "Create project"
                )
                .is_empty());
                assert!(
                    console.new_project_open,
                    "project form opens only after its explicit button click"
                );
                assert!(console.folder_request.is_none());
            }
        }
    }

    #[test]
    fn initial_route_preserves_owner_navigation_populated_data_and_coding_guide() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let empty = json!({"desktopPreferences":{"language":"en"}});
        let mut console = Console::default();
        assert!(render(
            &ctx,
            &mut console,
            &empty,
            egui::vec2(1280.0, 1200.0),
            false,
            vec![]
        )
        .0
        .is_empty());
        assert!(console.tab == Tab::Create);
        assert!(click(&ctx, &mut console, &empty, false, "navigation", "Projects").is_empty());
        assert!(console.tab == Tab::Projects);
        assert!(!console.quick_start.open);
        assert!(click(
            &ctx,
            &mut console,
            &empty,
            false,
            "memory_control",
            "Quick start"
        )
        .is_empty());
        assert!(
            console.quick_start.open,
            "explicit coding guide remains available"
        );
        assert!(click(
            &ctx,
            &mut console,
            &empty,
            false,
            "memory_control",
            "Leave guide"
        )
        .is_empty());
        assert!(!console.quick_start.open);
        render(
            &ctx,
            &mut console,
            &empty,
            egui::vec2(1280.0, 1200.0),
            false,
            vec![],
        );
        assert!(
            console.tab == Tab::Projects,
            "empty refresh must not override owner navigation"
        );
        for existing in [
            "project",
            "workspace",
            "work",
            "team",
            "draft",
            "navigation",
        ] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut console = Console::default();
            let mut data = empty.clone();
            match existing {
                "project" => {
                    data["projectMemory"] = json!({"projects":[{"projectId":"project:existing","name":"Owner project","revision":1}]})
                }
                "workspace" => {
                    data["workspaces"] = json!({"items":[{"workspaceId":"workspace:existing","displayName":"Owner folder"}]})
                }
                "work" => {
                    data["work"] =
                        json!({"items":[{"workItemId":"work:existing","title":"Owner work"}]})
                }
                "team" => data["teams"] = json!({"items":[{"teamId":"team:existing"}]}),
                "draft" => console.work.title = "Owner draft".into(),
                "navigation" => console.tab = Tab::Providers,
                _ => unreachable!(),
            }
            let expected = console.tab;
            assert!(render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 1200.0),
                false,
                vec![]
            )
            .0
            .is_empty());
            assert!(
                console.tab == expected,
                "{existing} must preserve the existing default or requested view"
            );
            assert!(!console.quick_start.open);
            if existing == "draft" {
                assert_eq!(console.work.title, "Owner draft");
            }
        }
    }

    #[test]
    fn project_jobs_need_current_identity_and_a_real_catalog_entry() {
        let mut snapshot = snapshot();
        let mut state = state();
        assert_eq!(
            state.payload(&snapshot),
            Some(
                json!({"projectId":"project:one","expectedProjectRevision":3,
            "abilityId":"P1","title":"Owner title","instruction":"Owner goal"})
            )
        );
        assert!(
            snapshot["workspaces"].is_null(),
            "project intake must not need Git folders"
        );
        state.ability = "unknown".into();
        assert!(state.payload(&snapshot).is_none());
        state.ability = "P1".into();
        snapshot["projectMemory"]["projects"][0]["revision"] = json!(0);
        assert!(state.payload(&snapshot).is_none());
        snapshot["projectMemory"]["projects"][0]["revision"] = json!(3);
        for invalid in [
            "".into(),
            " \n ".into(),
            "bad\0goal".into(),
            "😀".repeat(25_001),
        ] {
            state.draft().goal = invalid;
            assert!(state.payload(&snapshot).is_none());
        }
    }

    #[test]
    fn selection_keeps_drafts_and_receipts_clear_only_exact_acknowledged_text() {
        let mut snapshot = snapshot();
        let mut state = state();
        state.pending = Some(Submission {
            command_id: "cmd:one".into(),
            project: "project:one".into(),
            ability: "P1".into(),
            draft: state.draft().clone(),
        });
        state.project = "project:two".into();
        state.ability = "V1".into();
        *state.draft() = CreateDraft {
            title: "Other title".into(),
            goal: "Other goal".into(),
        };
        snapshot["projectWorkReceipt"] = json!({"commandId":"cmd:old","projectId":"project:one","workItemId":"work:one","outcome":"applied"});
        state.sync(&snapshot);
        assert!(state.pending.is_some());
        snapshot["projectWorkReceipt"]["commandId"] = json!("cmd:one");
        snapshot["projectWorkReceipt"]["projectId"] = json!("project:foreign");
        state.sync(&snapshot);
        assert!(state.pending.is_some());
        snapshot["projectWorkReceipt"]["projectId"] = json!("project:one");
        snapshot["projectJobs"] = json!([]);
        state.sync(&snapshot);
        assert!(
            state.pending.is_some(),
            "receipt alone cannot attribute a job"
        );
        snapshot["projectJobs"] = json!([{"projectId":"project:one","workItemId":"work:one"}]);
        state.sync(&snapshot);
        assert!(state.pending.is_none());
        assert_eq!(state.draft().goal, "Other goal");
        assert_eq!(state.project, "project:two");
        assert_eq!(state.selected_work.get("project:one").unwrap(), "work:one");
        state.project = "project:one".into();
        state.ability = "P1".into();
        assert!(state.draft().goal.is_empty());
        state.draft().goal = "New owner draft".into();
        let submitted = state.draft().clone();
        state.pending = Some(Submission {
            command_id: "cmd:two".into(),
            project: "project:one".into(),
            ability: "P1".into(),
            draft: submitted,
        });
        state.draft().goal = "Edited after submission".into();
        snapshot["projectWorkReceipt"]["commandId"] = json!("cmd:two");
        state.sync(&snapshot);
        assert_eq!(state.draft().goal, "Edited after submission");
    }

    #[test]
    fn rejected_enqueue_keeps_draft_and_cross_project_results_never_match() {
        let mut snapshot = snapshot();
        let mut state = state();
        state.pending = Some(Submission {
            command_id: "cmd:one".into(),
            project: "project:one".into(),
            ability: "P1".into(),
            draft: state.draft().clone(),
        });
        snapshot["projectWorkReceipt"] =
            json!({"commandId":"cmd:one","projectId":"project:one","outcome":"rejected"});
        state.sync(&snapshot);
        assert!(state.pending.is_none());
        assert_eq!(state.draft().goal, "Owner goal");
        state
            .selected_work
            .insert("project:one".into(), "work:one".into());
        snapshot["projectResult"] = json!({"reasonCode":"PROJECT_RESULT_READ","projectId":"project:two","workItemId":"work:one","path":"outputs/report.md","text":"PRIVATE_FOREIGN_CANARY"});
        assert!(state
            .scoped(&snapshot, "projectResult", "PROJECT_RESULT_READ")
            .is_none());
        snapshot["projectResult"]["projectId"] = json!("project:one");
        assert!(state
            .scoped(&snapshot, "projectResult", "PROJECT_RESULT_READ")
            .is_some());
        snapshot["projectJobs"][0]["projectId"] = json!("project:two");
        assert!(state
            .scoped(&snapshot, "projectResult", "PROJECT_RESULT_READ")
            .is_none());
    }

    #[test]
    fn export_dialog_cancellation_does_nothing_and_selection_is_frozen() {
        let request = CreateExportRequest {
            project_id: "project:one".into(),
            work_item_id: "work:one".into(),
            path: "outputs/report.md".into(),
            name: "report.md".into(),
        };
        assert!(Console::create_export_command(request.clone(), None).is_none());
        assert!(Console::create_export_command(request.clone(), Some(String::new())).is_none());
        let command =
            Console::create_export_command(request, Some("/tmp/owner-selected-report.md".into()))
                .unwrap();
        assert_eq!(command.action, "project_result_export");
        assert_eq!(command.payload["projectId"], "project:one");
        assert_eq!(command.payload["path"], "outputs/report.md");
        assert_eq!(
            command.payload["targetPath"],
            "/tmp/owner-selected-report.md"
        );
        assert!(command.payload["commandId"].is_string());
    }

    #[test]
    fn catalog_search_uses_selected_language_and_never_translates_owner_inputs() {
        let snapshot = snapshot();
        let ability = &snapshot["abilities"][0];
        assert!(matches_ability(
            ability,
            "presentations",
            "SUNUM",
            Language::Tr
        ));
        assert!(matches_ability(ability, "", "deck", Language::En));
        assert!(!matches_ability(ability, "video", "", Language::En));
        assert!(!matches_ability(ability, "", "unknown", Language::Tr));
        assert_eq!(ability_text(ability, "name", Language::Tr), "Sunum");
        let mut state = state();
        state.draft().goal = "Projects Memory".into();
        assert_eq!(
            state.payload(&snapshot).unwrap()["instruction"],
            "Projects Memory"
        );
    }
}
