use super::*;
#[path = "canvas.rs"]
mod canvas;
#[path = "capabilities.rs"]
mod capabilities;
pub use capabilities::{CapabilityPathKind, CapabilityPathRequest};
#[path = "chat.rs"]
mod chat;
#[path = "connect.rs"]
mod connect;
#[path = "conversation.rs"]
mod conversation;
#[path = "crew.rs"]
mod crew;
#[path = "output_studio.rs"]
mod output_studio;
pub use output_studio::{DeliveryExportRequest, StudioPathKind, StudioPathRequest};

// Inline project views share quiet secondary actions and a single accent action.
fn view_control(ui: &mut Ui, label: &str, enabled: bool, primary: bool) -> egui::Response {
    let response = studio::button(
        ui,
        t(ui, label).as_ref(),
        if primary {
            studio::ButtonKind::Primary
        } else {
            studio::ButtonKind::Ghost
        },
        enabled,
    );
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("test-action", "memory_control", label)),
            response.rect,
        );
        data.insert_temp(
            egui::Id::new(("test-action-visibility", label)),
            (
                ui.clip_rect(),
                response.enabled() && ui.clip_rect().intersects(response.rect),
            ),
        );
        data.insert_temp(
            egui::Id::new(("test-action-id", "memory_control", label)),
            response.id,
        );
    });
    response
}
fn quiet_action(
    ui: &mut Ui,
    commands: &mut Vec<Command>,
    label: &str,
    name: &'static str,
    enabled: bool,
    payload: impl FnOnce() -> Value,
) {
    let response = view_control(ui, label, enabled, false);
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
fn form_combo<R>(
    ui: &mut Ui,
    combo: egui::ComboBox,
    contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let previous = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = studio::metrics::CONTROL_HEIGHT;
    let result = combo.show_ui(ui, |ui| {
        ui.spacing_mut().interact_size.y = studio::metrics::COMPACT_HEIGHT;
        contents(ui)
    });
    ui.spacing_mut().interact_size.y = previous;
    result
}

fn inline_surface(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    let width = ui.available_width().min(980.0);
    ui.horizontal_top(|ui| {
        ui.add_space(((ui.available_width() - width) * 0.5).max(0.0));
        ui.vertical(|ui| {
            ui.set_width(width);
            if ui.ctx().viewport_rect().height() < 700.0 {
                ui.spacing_mut().item_spacing.y = studio::metrics::GAP;
            }
            content(ui);
        });
    });
}

fn short_copy(ui: &mut Ui, text: &str, size: f32, color: Color32, rows: usize) -> egui::Response {
    let mut job = egui::text::LayoutJob::simple(
        text.into(),
        egui::FontId::proportional(size),
        color,
        ui.available_width(),
    );
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    ui.add(egui::Label::new(job)).on_hover_text(text)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateExportRequest {
    pub project_id: String,
    pub work_item_id: String,
    pub path: String,
    pub name: String,
    pub expected_sha256: Option<String>,
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

#[derive(Clone, Default, PartialEq, Eq)]
struct ReplyDraft {
    parent: String,
    references: HashSet<String>,
}

#[derive(Default)]
struct ChatState {
    project: String,
    offset: usize,
    requested: Option<(String, u64, u64, usize)>,
    refresh: bool,
    files_open: bool,
    connections_open: bool,
    connection_requested: bool,
    details_open: bool,
    replies: HashMap<(String, String), ReplyDraft>,
    pending_reply: Option<(String, Option<ReplyDraft>)>,
    conversation: conversation::ConversationState,
}

#[derive(Default)]
pub(super) struct CreateState {
    project: String,
    ability: String,
    category: String,
    search: String,
    catalog_open: bool,
    drafts: HashMap<(String, String), CreateDraft>,
    selected_work: HashMap<String, String>,
    result_path: String,
    chat: ChatState,
    crew: crew::CrewState,
    studio: output_studio::StudioState,
    capabilities: capabilities::CapabilityState,
    canvas: canvas::CanvasState,
    pending: Option<Submission>,
    export_request: Option<CreateExportRequest>,
    host_epoch: Option<Value>,
}

impl CreateState {
    pub(super) fn sync(&mut self, snapshot: &Value) {
        if !snapshot["hostEpoch"].is_null() {
            if self
                .host_epoch
                .as_ref()
                .is_some_and(|epoch| *epoch != snapshot["hostEpoch"])
            {
                self.chat.conversation.preserve_inputs(&mut self.drafts);
                self.crew = Default::default();
                self.studio = Default::default();
                self.capabilities = Default::default();
                self.canvas = Default::default();
                self.chat = Default::default();
                self.pending = None;
                self.export_request = None;
            }
            self.host_epoch = Some(snapshot["hostEpoch"].clone());
        }
        self.sync_crew(snapshot);
        self.sync_studio(snapshot);
        self.sync_capabilities(snapshot);
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
            let same_reply = self
                .chat
                .pending_reply
                .as_ref()
                .filter(|(command, _)| command == &pending.command_id)
                .is_none_or(|(_, reply)| reply.as_ref() == self.chat.replies.get(&key));
            if self.drafts.get(&key) == Some(&pending.draft) && same_reply {
                self.drafts.remove(&key);
                if self
                    .chat
                    .pending_reply
                    .as_ref()
                    .is_some_and(|(command, reply)| {
                        command == &pending.command_id
                            && reply.as_ref() == self.chat.replies.get(&key)
                    })
                {
                    self.chat.replies.remove(&key);
                }
            }
            if self.project == pending.project {
                self.result_path.clear();
                let total = jobs(snapshot)
                    .iter()
                    .filter(|job| text(job, "projectId") == self.project)
                    .count();
                self.chat.offset = total.saturating_sub(1) / 20 * 20;
                self.chat.refresh = true;
            }
        }
        self.chat.pending_reply = None;
        self.pending = None;
    }

    fn draft(&mut self) -> &mut CreateDraft {
        self.drafts
            .entry((self.project.clone(), self.ability.clone()))
            .or_default()
    }

    fn select_ability(&mut self, ability: &str) {
        if self.ability.is_empty() {
            let initial = self
                .drafts
                .get(&(self.project.clone(), String::new()))
                .cloned();
            if let Some(initial) = initial {
                self.drafts
                    .entry((self.project.clone(), ability.to_owned()))
                    .or_insert(initial);
            }
        }
        if self.ability != ability {
            if let Some(reply) = self
                .chat
                .replies
                .get(&(self.project.clone(), self.ability.clone()))
                .cloned()
            {
                // A manual skill choice changes the requested output, not its source files.
                // Keep the old scoped copy for any receipt that is still in flight.
                self.chat
                    .replies
                    .insert((self.project.clone(), ability.to_owned()), reply);
            }
        }
        self.ability = ability.into();
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
        if !valid_input(&draft.goal, 50_000) {
            return None;
        }
        let title = draft_title(draft)?;
        Some(
            json!({"projectId":self.project,"expectedProjectRevision":project["revision"],
            "abilityId":self.ability,"title":title,"instruction":draft.goal.trim()}),
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

fn draft_title(draft: &CreateDraft) -> Option<String> {
    if !draft.title.trim().is_empty() {
        return valid_input(&draft.title, 160).then(|| draft.title.trim().into());
    }
    let normalized = draft.goal.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title = String::new();
    let mut units = 0;
    for character in normalized.chars() {
        if units + character.len_utf16() > 80 {
            break;
        }
        units += character.len_utf16();
        title.push(character);
    }
    valid_input(&title, 160).then(|| title.trim_end().into())
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
    pub fn chat_smoke(&mut self, snapshot: &Value, show_files: bool, show_connections: bool) {
        self.create_smoke(snapshot, show_files);
        self.create.chat.connections_open = show_connections;
        self.create.catalog_open =
            std::env::var("PATCHFLEET_SMOKE_SCENE").as_deref() == Ok("chat-catalog");
    }

    #[cfg(feature = "ui-smoke")]
    pub fn create_smoke(&mut self, snapshot: &Value, show_results: bool) {
        self.tab = Tab::Create;
        self.create.project = array(&snapshot["projectMemory"]["projects"])
            .first()
            .map(|project| text(project, "projectId").to_owned())
            .unwrap_or_default();
        self.create.ability = "P1".into();
        self.create.chat.files_open = show_results;
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
        let mut payload = json!({"projectId":request.project_id, "workItemId":request.work_item_id,"path":request.path,"targetPath":destination});
        if let Some(hash) = request.expected_sha256 {
            payload["expectedSha256"] = json!(hash);
        }
        Some(Command::new("project_result_export", payload))
    }

    pub(super) fn create_studio(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        self.chat_studio(ui, snapshot, commands);
        self.crew_roster_dialog(ui, snapshot);
        self.create_catalog_dialog(ui, snapshot);
        self.connection_dialog(ui, snapshot, commands);
    }

    pub(super) fn crew_hub(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        if self.create.productive_view() != 0
            && view_control(ui, "Back to crews", true, false).clicked()
        {
            self.create.select_productive_view(0);
        }
        match self.create.productive_view() {
            1 => self.workflow_dialog(ui, snapshot, commands),
            2 => self.output_studio(ui, snapshot, commands),
            3 => self.project_capabilities(ui, snapshot, commands),
            _ => self.crew_hub_view(ui, snapshot, commands),
        }
        self.crew_roster_dialog(ui, snapshot);
        self.create_catalog_dialog(ui, snapshot);
        self.connection_dialog(ui, snapshot, commands);
    }

    fn create_catalog_dialog(&mut self, ui: &mut Ui, snapshot: &Value) {
        if !self.create.catalog_open {
            return;
        }
        let ready = ui.is_enabled();
        let id = egui::Id::new("create-ability-catalog");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .frame(studio::card(ui))
            .show(ui.ctx(), |ui| {
                ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 680.0));
                egui::ScrollArea::vertical()
                    .id_salt("create-catalog-dialog")
                    .max_height((ui.ctx().viewport_rect().height() - 150.0).max(120.0))
                    .show(ui, |ui| {
                        ui.add_enabled_ui(ready, |ui| {
                            self.create_catalog(
                                ui,
                                array(&snapshot["abilities"]),
                                Language::from_snapshot(snapshot),
                            )
                        });
                    });
                if control(ui, "Close", true).clicked() {
                    self.create.catalog_open = false;
                    self.create.crew.catalog_target = None;
                }
            });
        if modal.should_close() {
            self.create.catalog_open = false;
            self.create.crew.catalog_target = None;
        }
    }

    fn create_catalog(&mut self, ui: &mut Ui, abilities: &[Value], language: Language) {
        egui::Frame::NONE.show(ui, |ui| {
            section_title(
                ui,
                "Explore abilities",
                "Task recipes guide your agent. Results depend on available tools and access.",
            );
            ui.add_space(studio::metrics::CONTROL_GAP);
            field(ui, "Search abilities", &mut self.create.search, 200, false);
            ui.horizontal_wrapped(|ui| {
                if studio::section_nav(
                    ui,
                    self.create.category.is_empty(),
                    t(ui, "All abilities").as_ref(),
                )
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
                    if studio::section_nav(
                        ui,
                        self.create.category == category,
                        t(ui, category_name(category)).as_ref(),
                    )
                    .clicked()
                    {
                        self.create.category = category.into();
                    }
                }
            });
            ui.add_space(studio::metrics::CONTROL_GAP);
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
                            let response = studio::button(
                                ui,
                                label,
                                if selected {
                                    studio::ButtonKind::Primary
                                } else {
                                    studio::ButtonKind::Secondary
                                },
                                true,
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    response.enabled(),
                                    selected,
                                    label,
                                )
                            });
                            #[cfg(test)]
                            ui.data_mut(|data| {
                                data.insert_temp(
                                    egui::Id::new(("test-ability-id", text(ability, "id"))),
                                    response.id,
                                );
                                data.insert_temp(
                                    egui::Id::new(("test-action", "ability", text(ability, "id"))),
                                    response.rect,
                                )
                            });
                            if response.clicked() {
                                if self.create.crew.catalog_target.is_some() {
                                    self.create.add_crew_member(ability, language);
                                } else {
                                    self.create.select_ability(text(ability, "id"));
                                }
                                self.create.catalog_open = false;
                            }
                            raw_hint(ui, ability_text(ability, "description", language));
                            ui.add_space(studio::metrics::GAP);
                        });
                    }
                });
        });
    }

    fn create_jobs(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        self.create_jobs_scoped(ui, snapshot, commands, None);
    }

    fn create_jobs_scoped(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
        scope: Option<&[String]>,
    ) {
        let project_jobs: Vec<_> = items(&snapshot["work"])
            .iter()
            .filter(|w| job_belongs(snapshot, &self.create.project, text(w, "workItemId")))
            .filter(|work| {
                scope.is_none_or(|ids| ids.iter().any(|id| id == text(work, "workItemId")))
            })
            .collect();
        if project_jobs.is_empty() {
            hint(
                ui,
                if scope.is_some() {
                    "No agent files are available for this mission yet."
                } else {
                    "No creations in this project yet. Open New creation to begin."
                },
            );
            return;
        }
        if project_jobs.len() > 1 || self.create.selected_work(snapshot).is_none() {
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
        }
        let Some(work) = self.create.selected_work(snapshot).filter(|work| {
            scope.is_none_or(|ids| ids.iter().any(|id| id == text(work, "workItemId")))
        }) else {
            hint(ui, "Choose a task to inspect its outputs.");
            return;
        };
        let runtime_issue = job_issue(snapshot, &self.create.project, text(work, "workItemId"));
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(text(work, "title"));
                status(ui, text(work, "status"));
            });
            ui.collapsing(t(ui, "Details"), |ui| {
                ui.label(text(work, "instruction"));
                hint(ui, "Files may be partial while a task is running or after a failure. Review them before use.");
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
                    action(ui, commands, "Start task", "work_start", runtime_issue.is_none() && connect::can_start(snapshot) && revision(work) > 0,
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
            if text(work, "status") == "queued" {
                if connect::can_start(snapshot) {
                    hint(ui, "Prepared. Start when you're ready.");
                } else {
                    ui.horizontal_wrapped(|ui| {
                        hint(ui, "Codex is not ready.");
                        if control(ui, "Set up Codex", true).clicked() {
                            self.create.chat.connections_open = true;
                        }
                    });
                }
            }
            if runtime_issue.is_some() || cfg!(target_os = "windows") {
                return;
            }
            ui.add_space(studio::metrics::CONTROL_GAP);
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
                    self.chat_reference_choice(ui,snapshot,work,result);
                    ui.horizontal_wrapped(|ui| {
                        if result["textPreview"] == true {
                            let before = commands.len();
                            action(ui, commands, "Preview text", "project_result_read", true,
                                || json!({"projectId":self.create.project,"workItemId":work["workItemId"],"path":result["path"]}));
                            if commands.len() > before { self.create.result_path = text(result, "path").into(); }
                        } else { hint(ui, "Export to open in its app."); }
                        if control(ui, "Export copy…", self.create.export_request.is_none()).clicked() {
                            self.create.export_request = Some(CreateExportRequest { project_id: self.create.project.clone(),
                                work_item_id: text(work, "workItemId").into(), path: text(result, "path").into(), name: text(result, "name").into(), expected_sha256: None });
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
                ui.add_space(studio::metrics::CONTROL_GAP);
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

    pub(super) fn snapshot() -> Value {
        json!({"projectMemory":{"projects":[{"projectId":"project:one","revision":3,"name":"Owner project"},
            {"projectId":"project:two","revision":4,"name":"Second project"}]},
            "abilities":[{"id":"P1","category":"presentations","name":{"en":"Presentation","tr":"Sunum"},
                "description":{"en":"Build a deck","tr":"Sunum hazırlayın"},"starter":{"en":"Present my idea","tr":"Fikrimi sun"},"instruction":"Prepare useful files."},
                {"id":"V1","category":"video","name":{"en":"Edit video","tr":"Video düzenle"},"description":{"en":"Trim a recording","tr":"Kaydı kesin"}}],
            "projectJobs":[{"projectId":"project:one","workItemId":"work:one"}],
            "providerConnections":{"reasonCode":"PROVIDER_CONNECTIONS_READ","providers":[{"id":"codex","version":"0.163.0","installed":true,"authentication":"present","controlSupported":true}]},
            "work":{"items":[{"workItemId":"work:one","title":"Existing work","revision":2,"status":"queued"}]},"projectChat":{"reasonCode":"PROJECT_CHAT_READ","projectId":"project:one","offset":0,"nextOffset":null,"total":0,"items":[]}})
    }

    fn chat_entry(data: &mut Value, status: &str) {
        data["work"]["items"][0]["status"] = json!(status);
        data["projectChat"]["total"] = json!(1);
        data["projectChat"]["items"] = json!([{"workItemId":"work:one","abilityId":"P1","goal":"Owner request retained","status":status,"revision":2,"replyToWorkItemId":null}]);
    }

    pub(super) fn state() -> CreateState {
        let mut state = CreateState {
            project: "project:one".into(),
            ability: "P1".into(),
            ..Default::default()
        };
        *state.draft() = CreateDraft {
            title: "Owner title".into(),
            goal: "Owner goal".into(),
        };
        state.crew.single = true;
        state
    }

    pub(super) fn render(
        ctx: &egui::Context,
        console: &mut Console,
        snapshot: &Value,
        size: egui::Vec2,
        busy: bool,
        events: Vec<egui::Event>,
    ) -> (Vec<Command>, Vec<String>) {
        ctx.global_style_mut(|style| style.animation_time = 0.0);
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

    pub(super) fn click(
        ctx: &egui::Context,
        console: &mut Console,
        data: &Value,
        busy: bool,
        action: &str,
        label: &str,
    ) -> Vec<Command> {
        click_at_size(
            ctx,
            console,
            data,
            egui::vec2(1280.0, 2200.0),
            busy,
            action,
            label,
        )
    }

    pub(super) fn click_at_size(
        ctx: &egui::Context,
        console: &mut Console,
        data: &Value,
        size: egui::Vec2,
        busy: bool,
        action: &str,
        label: &str,
    ) -> Vec<Command> {
        if matches!(label, "Skills" | "Plan crew" | "Details") && !egui::Popup::is_any_open(ctx) {
            assert!(
                click_at_size(ctx, console, data, size, busy, "memory_control", "More").is_empty()
            );
        }
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
    fn optional_catalog_fits_the_minimum_viewport_and_keyboard_selection_only_changes_the_draft() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                for key in [egui::Key::Enter, egui::Key::Space] {
                    let ctx = egui::Context::default();
                    studio::install(&ctx);
                    ctx.set_theme(theme);
                    let mut console = Console {
                        tab: Tab::Create,
                        create: state(),
                        ..Default::default()
                    };
                    console.create.crew.single = false;
                    let mut data = snapshot();
                    data["desktopPreferences"] = json!({"language":language});
                    data["abilities"] = patchfleet_core::abilities::catalog();
                    let (commands, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                    assert!(commands.is_empty());
                    assert!(!copy.join("\n").contains("Search abilities"));
                    assert!(!console.create.catalog_open);
                    assert!(click_at_size(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        "memory_control",
                        "Skills"
                    )
                    .is_empty());
                    console.create.search = "V1".into();
                    for _ in 0..3 {
                        assert!(render(&ctx, &mut console, &data, size, false, vec![])
                            .0
                            .is_empty());
                    }
                    let rect = ctx.data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new(("test-action", "ability", "V1")))
                            .unwrap()
                    });
                    assert!(
                        screen.contains_rect(rect),
                        "{language}/{theme:?} catalog option clipped {rect:?}"
                    );
                    let id = ctx.data(|data| {
                        data.get_temp::<egui::Id>(egui::Id::new(("test-ability-id", "V1")))
                            .unwrap()
                    });
                    ctx.memory_mut(|memory| memory.request_focus(id));
                    let (commands, _) = render(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        vec![egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::default(),
                        }],
                    );
                    assert!(
                        commands.is_empty(),
                        "skill selection must never prepare or start work"
                    );
                    assert_eq!(console.create.ability, "V1");
                    assert_eq!(
                        console.create.drafts[&("project:one".into(), "P1".into())].goal,
                        "Owner goal"
                    );
                    assert!(console.create.draft().goal.is_empty());
                    assert!(!console.create.catalog_open);
                }
            }
        }
    }

    #[test]
    fn connections_dialog_fits_minimum_viewport_and_only_runs_explicit_local_actions() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut console = Console {
                    tab: Tab::Create,
                    create: state(),
                    ..Default::default()
                };
                console.create.chat.connections_open = true;
                let mut data = snapshot();
                data["desktopPreferences"] = json!({"language":language,"reduceMotion":false});
                data["providerConnections"] = json!({"reasonCode":"PROVIDER_CONNECTIONS_READ","providers":[
                    {"id":"codex","installed":false,"authentication":"missing","controlSupported":false},
                    {"id":"claude","installed":true,"authentication":"present","controlSupported":false},
                    {"id":"gemini","installed":false,"authentication":"unknown","controlSupported":false}]});
                for _ in 0..3 {
                    assert!(render(&ctx, &mut console, &data, size, false, vec![])
                        .0
                        .is_empty());
                }
                let (_, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                assert!(copy.iter().any(|text| text
                    == i18n::tr(Language::from_code(language), "Install or update CLI").as_ref()));
                let close = ctx.data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "memory_control",
                        "Close",
                    )))
                    .unwrap()
                });
                let (close_clip, visible) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "test-action-visibility",
                        "Close",
                    )))
                    .unwrap()
                });
                assert!(
                    visible && screen.contains_rect(close) && close_clip.contains_rect(close),
                    "{language} {theme:?} close clipped: {close:?} {close_clip:?}"
                );
                for (action, label) in [
                    ("provider_connections_read", "Check connections"),
                    ("desktop_motion", "Reduce motion"),
                ] {
                    render(&ctx, &mut console, &data, size, false, vec![]);
                    let (enabled, rect, clip) = ctx.data(|data| {
                        data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                            "test-action-state",
                            action,
                            label,
                        )))
                        .unwrap()
                    });
                    assert!(
                        enabled && screen.contains_rect(rect) && clip.contains_rect(rect),
                        "{language} {theme:?} {label} clipped: {rect:?} {clip:?}"
                    );
                    let commands =
                        click_at_size(&ctx, &mut console, &data, size, false, action, label);
                    assert_eq!(commands.len(), 1);
                    assert_eq!(commands[0].action, action);
                    if action == "desktop_motion" {
                        assert_eq!(commands[0].payload["reduceMotion"], true);
                    }
                    assert!(
                        click_at_size(&ctx, &mut console, &data, size, true, action, label)
                            .is_empty()
                    );
                }
                assert!(click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "memory_control",
                    "Close"
                )
                .is_empty());
                assert!(
                    !console.create.chat.connections_open,
                    "visible close must receive a real click"
                );
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn file_checkbox_prepares_scoped_follow_up_without_mutating_source_or_starting() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = Console {
            tab: Tab::Create,
            create: state(),
            ..Default::default()
        };
        let mut data = snapshot();
        data["desktopPreferences"] = json!({"language":"en"});
        chat_entry(&mut data, "completed");
        data["projectResults"] = json!({"reasonCode":"PROJECT_RESULTS_READ","projectId":"project:one","workItemId":"work:one",
            "items":[{"path":"outputs/report.md","name":"Original report.md","bytes":12,"textPreview":true}]});
        let source = data["projectResults"].clone();
        console
            .create
            .selected_work
            .insert("project:one".into(), "work:one".into());
        console.create.chat.files_open = true;
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "reference",
            "outputs/report.md"
        )
        .is_empty());
        let key = ("project:one".into(), "P1".into());
        assert_eq!(console.create.chat.replies[&key].parent, "work:one");
        assert!(console.create.chat.replies[&key]
            .references
            .contains("outputs/report.md"));
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Close files"
        )
        .is_empty());
        console.create.draft().goal = "Revise the introduction using the selected report".into();
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_chat_send",
            "Prepare task",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_chat_send");
        assert_eq!(commands[0].payload["projectId"], "project:one");
        assert_eq!(commands[0].payload["abilityId"], "P1");
        assert_eq!(commands[0].payload["replyToWorkItemId"], "work:one");
        assert_eq!(
            commands[0].payload["referencePaths"],
            json!(["outputs/report.md"])
        );
        assert_eq!(data["projectResults"], source);
        assert_eq!(
            console.create.chat.replies[&key].references.len(),
            1,
            "references survive until a matching durable receipt"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn manual_skill_change_keeps_follow_up_files_before_and_after_file_selection() {
        for select_files_first in [true, false] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut console = Console {
                tab: Tab::Create,
                create: state(),
                ..Default::default()
            };
            let mut data = snapshot();
            data["desktopPreferences"] = json!({"language":"en"});
            data["abilities"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":"D1","category":"documents","name":{"en":"Document"}}));
            chat_entry(&mut data, "completed");
            data["projectChat"]["items"][0]["abilityId"] = json!("D1");
            data["projectResults"] = json!({"reasonCode":"PROJECT_RESULTS_READ","projectId":"project:one","workItemId":"work:one",
                "items":[{"path":"outputs/report.md","name":"Original report.md","bytes":12,"textPreview":true}]});
            let source = data["projectResults"].clone();
            console.create.ability = "D1".into();
            console.create.draft().goal = "Unsent document draft".into();
            assert!(click(
                &ctx,
                &mut console,
                &data,
                false,
                "memory_control",
                "Follow up"
            )
            .is_empty());
            for files_step in [true, false] {
                if files_step == select_files_first {
                    let commands = click(
                        &ctx,
                        &mut console,
                        &data,
                        false,
                        "project_results_read",
                        "Files",
                    );
                    assert_eq!(commands.len(), 1);
                    assert_eq!(commands[0].action, "project_results_read");
                    assert!(click(
                        &ctx,
                        &mut console,
                        &data,
                        false,
                        "reference",
                        "outputs/report.md"
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
                } else {
                    assert!(
                        click(&ctx, &mut console, &data, false, "memory_control", "Skills")
                            .is_empty()
                    );
                    assert!(click(&ctx, &mut console, &data, false, "ability", "P1").is_empty());
                }
            }
            assert_eq!(
                console.create.ability, "P1",
                "file selection must retain the manual skill choice"
            );
            let key = ("project:one".to_owned(), "P1".to_owned());
            assert_eq!(console.create.chat.replies[&key].parent, "work:one");
            assert!(console.create.chat.replies[&key]
                .references
                .contains("outputs/report.md"));
            console.create.draft().goal = "Create a presentation from the selected report".into();
            let commands = click(
                &ctx,
                &mut console,
                &data,
                false,
                "project_chat_send",
                "Prepare task",
            );
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].action, "project_chat_send");
            assert_eq!(commands[0].payload["projectId"], "project:one");
            assert_eq!(commands[0].payload["expectedProjectRevision"], 3);
            assert_eq!(commands[0].payload["abilityId"], "P1");
            assert_eq!(commands[0].payload["replyToWorkItemId"], "work:one");
            assert_eq!(
                commands[0].payload["referencePaths"],
                json!(["outputs/report.md"])
            );
            assert_eq!(
                commands[0].payload["instruction"],
                "Create a presentation from the selected report"
            );

            // A delayed receipt acknowledges only the submitted skill's frozen draft.
            assert!(click(&ctx, &mut console, &data, false, "memory_control", "Skills").is_empty());
            assert!(click(&ctx, &mut console, &data, false, "ability", "V1").is_empty());
            console.create.draft().goal = "A separate unsent video follow-up".into();
            let video_key = ("project:one".to_owned(), "V1".to_owned());
            console
                .create
                .chat
                .replies
                .get_mut(&video_key)
                .unwrap()
                .references
                .insert("outputs/new.md".into());
            data["projectWorkReceipt"] = json!({"commandId":commands[0].payload["commandId"],"projectId":"project:one","workItemId":"work:one","outcome":"applied"});
            console.create.sync(&data);
            assert_eq!(console.create.ability, "V1");
            assert_eq!(
                console.create.draft().goal,
                "A separate unsent video follow-up"
            );
            assert_eq!(console.create.chat.replies[&video_key].parent, "work:one");
            assert_eq!(console.create.chat.replies[&video_key].references.len(), 2);
            assert!(!console.create.drafts.contains_key(&key));
            assert!(!console.create.chat.replies.contains_key(&key));
            assert_eq!(
                console.create.drafts[&("project:one".into(), "D1".into())].goal,
                "Unsent document draft"
            );
            assert_eq!(data["projectResults"], source);
        }
    }

    #[test]
    fn simple_prompt_has_no_hidden_recipe_and_keeps_catalog_and_details_out_of_the_way() {
        for language in ["tr", "en"] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut data = snapshot();
            data["desktopPreferences"] = json!({"language":language});
            data["abilities"] = patchfleet_core::abilities::catalog();
            let mut console = Console::launch();
            console.create.project = "project:one".into();
            console.create.draft().goal = "Owner launch idea".into();
            console.create.crew.single = true;
            let (commands, copy) = render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(800.0, 560.0),
                false,
                vec![],
            );
            assert!(commands.is_empty());
            assert!(
                console.create.ability.is_empty(),
                "typing must not silently select research"
            );
            assert!(console.create.payload(&data).is_none());
            let copy = copy.join("\n");
            for hidden in [
                "Search abilities",
                "Yetenek ara",
                "Task name (optional)",
                "Görev adı (isteğe bağlı)",
                "Recipe instructions",
                "Tarif yönergeleri",
            ] {
                assert!(
                    !copy.contains(hidden),
                    "advanced content leaked onto primary flow: {hidden}"
                );
            }
            assert!(copy.contains(if language == "tr" {
                "Bir tür seçin"
            } else {
                "Choose a type"
            }));
            let prompt = ctx.data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new("create-goal-prompt"))
                    .unwrap()
            });
            assert!(
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 560.0))
                    .contains_rect(prompt)
            );
            let enabled = ctx.data(|data| {
                data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                    "test-action-state",
                    "project_chat_send",
                    "Prepare task",
                )))
                .unwrap()
                .0
            });
            assert!(!enabled);
            assert!(click(&ctx, &mut console, &data, false, "memory_control", "Skills").is_empty());
            console.create.search = "P1".into();
            assert!(click(&ctx, &mut console, &data, false, "ability", "P1").is_empty());
            assert_eq!(console.create.ability, "P1");
            assert_eq!(console.create.draft().goal, "Owner launch idea");
            assert_eq!(
                console.create.payload(&data).unwrap()["title"],
                "Owner launch idea"
            );
            let copy = render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(800.0, 560.0),
                false,
                vec![],
            )
            .1
            .join("\n");
            assert!(copy.contains(if language == "tr" {
                "Sunum hazırla"
            } else {
                "Create a presentation"
            }));
            assert!(click(&ctx, &mut console, &data, false, "memory_control", "Skills").is_empty());
            assert!(console.create.catalog_open);
            console.create.category.clear();
            console.create.search = "V1".into();
            let copy = render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(800.0, 560.0),
                false,
                vec![],
            )
            .1
            .join("\n");
            assert!(copy.contains(if language == "tr" {
                "Yetenek ara"
            } else {
                "Search abilities"
            }));
            assert!(click(&ctx, &mut console, &data, false, "ability", "V1").is_empty());
            assert_eq!(console.create.ability, "V1");
            assert!(!console.create.catalog_open);
            console.create.select_ability("P1");
            assert_eq!(
                console.create.draft().goal,
                "Owner launch idea",
                "returning to a recipe restores its exact draft"
            );
        }
    }

    #[test]
    fn optional_title_is_derived_from_goal_without_splitting_unicode_boundaries() {
        let mut draft = CreateDraft {
            goal: "  Plan\n the\t launch  ".into(),
            ..Default::default()
        };
        assert_eq!(draft_title(&draft).unwrap(), "Plan the launch");
        draft.goal = "😀".repeat(100);
        assert_eq!(draft_title(&draft).unwrap(), "😀".repeat(40));
        assert_eq!(utf16_len(&draft_title(&draft).unwrap()), 80);
        draft.goal = format!("{}😀next", "a".repeat(79));
        assert_eq!(draft_title(&draft).unwrap(), "a".repeat(79));
        draft.title = "😀".repeat(80);
        assert_eq!(utf16_len(&draft_title(&draft).unwrap()), 160);
        draft.title.push('a');
        assert!(draft_title(&draft).is_none());
        draft.title = "  Owner's exact title  ".into();
        assert_eq!(draft_title(&draft).unwrap(), "Owner's exact title");
        let mut state = state();
        state.draft().title.clear();
        state.draft().goal = "A useful result".into();
        assert_eq!(
            state.payload(&snapshot()).unwrap()["title"],
            "A useful result"
        );
    }

    #[test]
    fn secondary_navigation_is_hidden_until_more_and_all_routes_remain_accessible() {
        for tab in [
            Tab::Memory,
            Tab::Decisions,
            Tab::Agents,
            Tab::Providers,
            Tab::Cloud,
        ] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut console = Console {
                tab: Tab::Create,
                create: state(),
                ..Default::default()
            };
            let data = snapshot();
            render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 1200.0),
                false,
                vec![],
            );
            assert!(ctx
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new((
                    "test-action",
                    "navigation",
                    tab.name()
                ))))
                .is_none());
            assert!(click(&ctx, &mut console, &data, false, "navigation", "More").is_empty());
            let commands = click(&ctx, &mut console, &data, false, "navigation", tab.name());
            assert!(commands.iter().all(|command| !matches!(
                command.action,
                "work_start" | "work_enqueue" | "project_chat_send"
            )));
            assert!(
                console.tab == tab,
                "More must preserve route {}",
                tab.name()
            );
            let rect = ctx.data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new((
                    "test-action",
                    "navigation",
                    tab.name(),
                )))
                .unwrap()
            });
            assert!(rect.is_positive());
        }
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = Console {
            tab: Tab::Create,
            create: state(),
            ..Default::default()
        };
        let data = snapshot();
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "navigation",
            Tab::Teams.name()
        )
        .is_empty());
        assert!(
            console.tab == Tab::Teams,
            "Project crews must be reachable without opening More"
        );
    }

    #[test]
    fn unavailable_provider_has_a_clear_setup_route_without_starting_a_task() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut console = Console {
            tab: Tab::Create,
            create: state(),
            ..Default::default()
        };
        let mut data = snapshot();
        data["providerConnections"]["providers"][0]["authentication"] = json!("missing");
        data["desktopPreferences"] = json!({"language":"en"});
        console.create.chat.files_open = false;
        chat_entry(&mut data, "queued");
        console
            .create
            .selected_work
            .insert("project:one".into(), "work:one".into());
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
        let copy = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            vec![],
        )
        .1
        .join("\n");
        assert!(copy.contains("Connect a supported Codex account to start."));
        assert!(click(
            &ctx,
            &mut console,
            &data,
            true,
            "memory_control",
            "Set up Codex"
        )
        .is_empty());
        assert!(console.tab == Tab::Create);
        assert!(click(
            &ctx,
            &mut console,
            &data,
            false,
            "memory_control",
            "Set up Codex"
        )
        .iter()
        .all(|command| command.action == "provider_connections_read"));
        assert!(console.create.chat.connections_open);
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
        assert!(copy.join("\n").contains("What would you like to make?"));
        let commands = click(
            &ctx,
            &mut console,
            &data,
            true,
            "project_chat_send",
            "Prepare task",
        );
        assert!(commands.is_empty());
        assert_eq!(console.create.draft().goal, "Owner goal");
        let commands = click(
            &ctx,
            &mut console,
            &data,
            false,
            "project_chat_send",
            "Prepare task",
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_chat_send");
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
            "project_chat_send",
            "Prepare task"
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
                console.create.chat.files_open = true;
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
        console.create.chat.files_open = true;
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
        console.create.chat.files_open = true;
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
                        ("project_results_read", "Files"),
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
                        console.create.chat.files_open = false;
                        chat_entry(&mut data, "queued");
                        data["providerConnections"] = json!({"reasonCode":"PROVIDER_CONNECTIONS_READ","providers":[{"id":"codex","installed":true,"version":"0.155.1","authentication":if provider_ready {"present"} else {"missing"},"controlSupported":true}]});
                        console
                            .create
                            .selected_work
                            .insert("project:one".into(), "work:one".into());
                        for _ in 0..3 {
                            render(&ctx, &mut console, &data, size, busy, vec![]);
                        }
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
                        "project_chat_send",
                        "Prepare task",
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
                assert_eq!(commands[0].action, "project_chat_send");
                assert_eq!(commands[0].payload["instruction"], "Owner goal");
                assert_eq!(console.create.draft().goal, "Owner goal");
            }
        }
    }

    #[test]
    fn fresh_console_opens_a_general_composer_without_requiring_project_or_folder_setup() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut console = Console::launch();
                let data = json!({"desktopPreferences":{"language":language}});
                let (commands, copy) = render(
                    &ctx,
                    &mut console,
                    &data,
                    egui::vec2(800.0, 560.0),
                    false,
                    vec![],
                );
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].action, "provider_connections_read");
                assert!(console.tab == Tab::Create);
                assert!(!console.quick_start.open);
                assert!(console.folder_request.is_none());
                assert!(!copy.join("\n").contains("Git"));
                assert!(copy.join("\n").contains(if language == "tr" {
                    "Ne üretmek istersiniz?"
                } else {
                    "What would you like to make?"
                }));
                let action = ctx.data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "conversation-send",
                        "Send",
                    )))
                    .unwrap()
                });
                assert!(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 560.0))
                        .contains_rect(action)
                );
                assert!(click(
                    &ctx,
                    &mut console,
                    &data,
                    false,
                    "conversation-send",
                    "Send"
                )
                .is_empty());
                assert!(!console.new_project_open);
                assert!(!console.create.conversation_creating());
            }
        }
    }

    #[test]
    fn initial_route_preserves_owner_navigation_populated_data_and_coding_guide() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let empty = json!({"desktopPreferences":{"language":"en"},"providerConnections":{"reasonCode":"PROVIDER_CONNECTIONS_READ","providers":[{"id":"codex","version":"0.163.0","installed":true,"authentication":"present","controlSupported":true}]}});
        let mut console = Console::launch();
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
            let mut console = Console::launch();
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
            .iter()
            .all(|command| command.action == "project_chat_read"));
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
            expected_sha256: None,
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
