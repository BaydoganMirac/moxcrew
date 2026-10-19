use super::*;

const COMPOSER_ROW_WIDTH: f32 = 640.0;

fn composer_frame(ui: &Ui, focused: bool) -> egui::Frame {
    studio::card(ui)
        .fill(colors(ui).surface)
        .stroke(egui::Stroke::new(
            1.0,
            if focused {
                colors(ui).primary
            } else {
                colors(ui).control_border
            },
        ))
        .corner_radius(studio::metrics::CARD_RADIUS)
        .inner_margin(studio::metrics::PADDING)
        .shadow(egui::epaint::Shadow {
            offset: [0, 6],
            blur: 24,
            spread: 0,
            color: colors(ui).primary.gamma_multiply(0.10),
        })
}

fn terminal(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "interrupted")
}

fn centered(ui: &mut Ui, width: f32, contents: impl FnOnce(&mut Ui)) {
    let width = ui.available_width().min(width);
    ui.vertical_centered(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            contents,
        );
    });
}

fn suggestion(goal: &str) -> Option<&'static str> {
    let words = goal.to_lowercase();
    let groups = [
        (
            "P1",
            &[
                "presentation",
                "slides",
                "slide deck",
                "pptx",
                "sunum",
                "slayt",
            ][..],
        ),
        ("V1", &["video", "clip", "subtitle", "altyazı", "kurgu"][..]),
        (
            "I1",
            &["image", "picture", "poster", "görsel", "resim", "afiş"][..],
        ),
        (
            "A1",
            &[
                "audio",
                "podcast",
                "transcribe",
                "transcript",
                "ses kaydı",
                "seslendir",
            ][..],
        ),
        ("R1", &["research", "compare", "araştır", "karşılaştır"][..]),
        (
            "D1",
            &[
                "document", "word", "report", "proposal", "belge", "rapor", "teklif",
            ][..],
        ),
        (
            "T1",
            &[
                "csv",
                "excel",
                "spreadsheet",
                "tablo analizi",
                "veri analizi",
            ][..],
        ),
        (
            "W1",
            &[
                "website",
                "web site",
                "landing page",
                "web sitesi",
                "uygulama",
            ][..],
        ),
    ];
    let found: Vec<_> = groups
        .iter()
        .filter(|(_, terms)| terms.iter().any(|term| words.contains(term)))
        .collect();
    (found.len() == 1).then(|| found[0].0)
}

impl CreateState {
    fn chat_page<'a>(&self, snapshot: &'a Value) -> Option<&'a Value> {
        let page = &snapshot["projectChat"];
        (text(page, "reasonCode") == "PROJECT_CHAT_READ"
            && text(page, "projectId") == self.project
            && (!page["workRevision"].is_number()
                || page["workRevision"] == snapshot["work"]["revision"])
            && page["offset"].as_u64().unwrap_or(0) == self.chat.offset as u64)
            .then_some(page)
    }

    fn chat_payload(&self, snapshot: &Value) -> Option<Value> {
        let draft = self
            .drafts
            .get(&(self.project.clone(), self.ability.clone()))?;
        if !valid_input(&draft.goal, 20_000) {
            return None;
        }
        let mut payload = self.payload(snapshot)?;
        if let Some(reply) = self
            .chat
            .replies
            .get(&(self.project.clone(), self.ability.clone()))
        {
            if !job_belongs(snapshot, &self.project, &reply.parent) || reply.references.len() > 8 {
                return None;
            }
            let parent = items(&snapshot["work"])
                .iter()
                .find(|work| text(work, "workItemId") == reply.parent)?;
            if !terminal(text(parent, "status")) {
                return None;
            }
            let mut references: Vec<_> = reply.references.iter().cloned().collect();
            references.sort();
            payload["replyToWorkItemId"] = json!(reply.parent);
            payload["referencePaths"] = json!(references);
        }
        Some(payload)
    }

    fn next_chat_read(&mut self, snapshot: &Value, busy: bool) -> Option<Command> {
        if self.chat.project != self.project {
            self.chat.project = self.project.clone();
            self.chat.offset = 0;
            self.chat.requested = None;
        }
        let project = array(&snapshot["projectMemory"]["projects"])
            .iter()
            .find(|project| text(project, "projectId") == self.project)?;
        let key = (
            self.project.clone(),
            revision(project),
            revision(&snapshot["work"]),
            self.chat.offset,
        );
        if self.chat.requested.is_none() && self.chat_page(snapshot).is_some() && !self.chat.refresh
        {
            self.chat.requested = Some(key);
            return None;
        }
        if busy || (!self.chat.refresh && self.chat.requested.as_ref() == Some(&key)) {
            return None;
        }
        self.chat.refresh = false;
        self.chat.requested = Some(key);
        Some(Command::new(
            "project_chat_read",
            json!({"projectId":self.project,"offset":self.chat.offset,"limit":20}),
        ))
    }
}

impl Console {
    pub(super) fn chat_studio(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        let projects = array(&snapshot["projectMemory"]["projects"]);
        if self.create.project.is_empty() && projects.len() == 1 {
            self.create.project = text(&projects[0], "projectId").into();
        }
        let compact = ui.ctx().viewport_rect().height() < 700.0;
        let previous = self.create.project.clone();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if !projects.is_empty() {
                let name = projects
                    .iter()
                    .find(|project| text(project, "projectId") == self.create.project)
                    .map(|project| std::borrow::Cow::Borrowed(text(project, "name")))
                    .unwrap_or_else(|| t(ui, "Choose…"));
                ui.scope(|ui| {
                    ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                    ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    egui::ComboBox::from_id_salt("create-project-picker")
                        .selected_text(name)
                        .truncate()
                        .width((ui.available_width() - 340.0).clamp(110.0, 240.0))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.create.project,
                                String::new(),
                                t(ui, "None"),
                            );
                            for project in projects {
                                ui.selectable_value(
                                    &mut self.create.project,
                                    text(project, "projectId").into(),
                                    text(project, "name"),
                                );
                            }
                        });
                });
                ui.add_space(studio::metrics::GAP);
                ui.scope(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = studio::metrics::GAP;
                        for (view, label) in [(0usize, "Chat"), (1, "Crew"), (2, "Outputs")] {
                            let response = studio::view_switch(
                                ui,
                                self.create.productive_view() == view,
                                t(ui, label).as_ref(),
                            );
                            #[cfg(test)]
                            ui.data_mut(|data| {
                                data.insert_temp(
                                    egui::Id::new(("project-view-tab", view)),
                                    response.rect,
                                )
                            });
                            if response.clicked() {
                                let next = if view == self.create.productive_view() {
                                    0
                                } else {
                                    view
                                };
                                self.create.select_productive_view(next);
                                if next == 0 {
                                    ui.memory_mut(|memory| {
                                        memory.request_focus(egui::Id::new((
                                            "chat-goal",
                                            &self.create.project,
                                            &self.create.ability,
                                        )))
                                    });
                                }
                            }
                        }
                    });
                });
            } else {
                ui.label(studio::title(t(ui, "Your studio"), 16.0).color(colors(ui).muted));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if studio::quiet_button(ui, t(ui, "Connections").as_ref()).clicked() {
                    self.create.chat.connections_open = true;
                }
            });
        });
        ui.add_space(if compact { 12.0 } else { 24.0 });
        if projects.is_empty() {
            centered(ui, 820.0, |ui| {
                ui.add_space(if compact {
                    f32::from(studio::metrics::PADDING)
                } else {
                    48.0
                });
                ui.vertical_centered(|ui| {
                    crate::motion::mox(
                        ui,
                        if compact { 120.0 } else { 180.0 },
                        false,
                        snapshot["desktopPreferences"]["reduceMotion"] == true,
                    );
                    ui.add_space(f32::from(studio::metrics::PADDING));
                    ui.label(studio::title(
                        t(ui, "What would you like to make?"),
                        if compact { 32.0 } else { 36.0 },
                    ));
                    ui.add_space(studio::metrics::GAP);
                    hint(
                        ui,
                        "Create a project to keep your work together. No folder setup needed.",
                    );
                    ui.add_space(studio::metrics::SECTION_GAP);
                    if submit(ui, "Create project", "create-project") {
                        self.new_project_open = true;
                    }
                });
            });
            return;
        }
        if previous != self.create.project {
            self.create.result_path.clear();
            self.create.chat.files_open = false;
            self.create.select_productive_view(0);
        }
        if !projects
            .iter()
            .any(|project| text(project, "projectId") == self.create.project)
        {
            hint(ui, "Choose the project for this idea.");
            return;
        }
        let context = self.create.productive_view();
        if context != 0 {
            if ui.ctx().viewport_rect().width() >= 1280.0 {
                egui::Panel::right("project-context-panel")
                    .resizable(false)
                    .exact_size(600.0)
                    .frame(studio::card(ui))
                    .show(ui, |ui| self.project_context(ui, snapshot, commands));
            } else {
                self.project_context(ui, snapshot, commands);
                return;
            }
        }
        let wide = ui.available_width() >= 850.0;
        if wide && self.create.crew.panel_open {
            egui::Panel::right("conversation-crew")
                .resizable(true)
                .default_size(330.0)
                .min_size(290.0)
                .frame(studio::card(ui))
                .show(ui, |ui| self.crew_panel(ui, snapshot, commands));
        }
        if wide && self.create.chat.files_open && !self.create.crew.panel_open {
            egui::Panel::right("conversation-files")
                .resizable(true)
                .default_size(320.0)
                .min_size(270.0)
                .frame(studio::card(ui))
                .show(ui, |ui| {
                    self.chat_files(ui, snapshot, commands);
                });
        }
        let welcome = self
            .create
            .chat_page(snapshot)
            .is_some_and(|page| array(&page["items"]).is_empty())
            && !self.create.crew_has_history(snapshot);
        if !welcome {
            let has_reply = self
                .create
                .chat
                .replies
                .contains_key(&(self.create.project.clone(), self.create.ability.clone()));
            egui::Panel::bottom("conversation-composer")
                .resizable(false)
                .show_separator_line(false)
                .exact_size(
                    if ui.available_width().min(820.0)
                        - composer_frame(ui, false).total_margin().sum().x
                        < COMPOSER_ROW_WIDTH
                    {
                        224.0
                    } else {
                        174.0
                    } + if has_reply { 52.0 } else { 0.0 },
                )
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    centered(ui, 820.0, |ui| {
                        self.chat_composer_frame(ui, snapshot, commands)
                    });
                });
        }
        egui::ScrollArea::vertical()
            .id_salt((
                "project-conversation",
                &self.create.project,
                self.create.chat.offset,
            ))
            .stick_to_bottom(!welcome)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                centered(ui, 820.0, |ui| self.chat_history(ui, snapshot, commands))
            });
        if self.create.crew.panel_open && !wide {
            self.crew_overlay(ui, snapshot, commands);
        }
        if self.create.chat.files_open && !wide && !self.create.crew.panel_open {
            let ready = ui.is_enabled();
            let id = egui::Id::new("conversation-files-dialog");
            let modal = egui::Modal::new(id)
                .area(egui::Modal::default_area(id).fade_in(false))
                .frame(studio::card(ui))
                .show(ui.ctx(), |ui| {
                    ui.set_width((ui.ctx().viewport_rect().width() - 64.0).clamp(280.0, 620.0));
                    ui.add_enabled_ui(ready, |ui| self.chat_files(ui, snapshot, commands));
                });
            if modal.should_close() {
                self.create.chat.files_open = false;
            }
        }
        self.chat_details(ui, snapshot);
        if commands.is_empty() {
            if let Some(command) = self
                .create
                .next_chat_read(snapshot, !ui.is_enabled() || self.hold_automatic_reads)
            {
                commands.push(command);
            }
        }
    }

    fn project_context(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(egui::Id::new("project-context-bounds"), ui.max_rect())
        });
        let id = egui::Id::new(("project-context", &self.create.project));
        ui.scope_builder(egui::UiBuilder::new().id(id), |ui| {
            if studio::quiet_button(ui, t(ui, "Back to chat").as_ref()).clicked() {
                self.create.select_productive_view(0);
                ui.memory_mut(|memory| {
                    memory.request_focus(egui::Id::new((
                        "chat-goal",
                        &self.create.project,
                        &self.create.ability,
                    )))
                });
                return;
            }
            ui.add_space(studio::metrics::GAP);
            match self.create.productive_view() {
                1 => self.workflow_dialog(ui, snapshot, commands),
                2 => self.output_studio(ui, snapshot, commands),
                _ => {}
            }
        });
    }

    fn chat_composer_frame(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let focused = ui.memory(|memory| {
            memory.has_focus(egui::Id::new((
                "chat-goal",
                &self.create.project,
                &self.create.ability,
            )))
        });
        let frame =
            composer_frame(ui, focused).show(ui, |ui| self.chat_composer(ui, snapshot, commands));
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("chat-composer-bounds"),
                (frame.response.rect, ui.clip_rect()),
            )
        });
        #[cfg(not(test))]
        let _ = frame;
    }

    fn chat_composer(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let language = Language::from_snapshot(snapshot);
        if let Some(reply) = self
            .create
            .chat
            .replies
            .get(&(self.create.project.clone(), self.create.ability.clone()))
            .cloned()
        {
            ui.horizontal_wrapped(|ui| {
                hint(
                    ui,
                    &tf(
                        ui,
                        "Follow-up · {0} selected files",
                        &[&reply.references.len().to_string()],
                    ),
                );
                if control(ui, "Clear follow-up", true).clicked() {
                    self.create
                        .chat
                        .replies
                        .remove(&(self.create.project.clone(), self.create.ability.clone()));
                }
            });
        }
        let goal_id = egui::Id::new(("chat-goal", &self.create.project, &self.create.ability));
        // This unframed editor's compact inset is included in the measured composer reservation.
        let response = ui.add(
            egui::TextEdit::multiline(&mut self.create.draft().goal)
                .id(goal_id)
                .frame(egui::Frame::NONE)
                .hint_text(t(ui, "Describe your idea or ask for a change…"))
                .desired_width(f32::INFINITY)
                .desired_rows(2)
                .char_limit(20_000)
                .margin(egui::Margin::symmetric(8, 6)),
        );
        let shortcut = response.has_focus()
            && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
        #[cfg(test)]
        ui.data_mut(|data| data.insert_temp(egui::Id::new("create-goal-prompt"), response.rect));
        ui.add_space(studio::metrics::GAP);
        let goal = self.create.draft().goal.clone();
        let suggested = suggestion(&goal).filter(|id| {
            array(&snapshot["abilities"])
                .iter()
                .any(|ability| text(ability, "id") == *id)
        });
        let payload = self.create.chat_payload(snapshot);
        let two_rows = ui.available_width() < COMPOSER_ROW_WIDTH;
        ui.horizontal_wrapped(|ui| {
            if self.create.crew.single {
                let enabled = payload.is_some() && self.create.pending.is_none();
                let clicked = ui
                    .add_enabled_ui(enabled, |ui| {
                        submit(ui, "Prepare task", "project_chat_send")
                    })
                    .inner;
                if (clicked || (shortcut && enabled)) && commands.is_empty() {
                    if let Some(payload) = payload {
                        let command = Command::new("project_chat_send", payload);
                        let command_id = text(&command.payload, "commandId").to_owned();
                        self.create.chat.pending_reply = Some((
                            command_id.clone(),
                            self.create
                                .chat
                                .replies
                                .get(&(self.create.project.clone(), self.create.ability.clone()))
                                .cloned(),
                        ));
                        self.create.pending = Some(Submission {
                            command_id,
                            project: self.create.project.clone(),
                            ability: self.create.ability.clone(),
                            draft: self.create.draft().clone(),
                        });
                        commands.push(command);
                    }
                }
            } else {
                self.crew_plan_button(ui, snapshot, commands, shortcut);
            }
            if !two_rows {
                self.chat_composer_options(ui);
            }
        });
        if two_rows {
            self.chat_composer_options(ui);
        }
        if let Some(selected) = array(&snapshot["abilities"])
            .iter()
            .find(|ability| text(ability, "id") == self.create.ability)
        {
            ui.add(
                egui::Label::new(
                    RichText::new(ability_text(selected, "name", language))
                        .size(12.0)
                        .color(colors(ui).muted),
                )
                .truncate(),
            );
        } else if let Some(suggested) = suggested.and_then(|id| {
            array(&snapshot["abilities"])
                .iter()
                .find(|ability| text(ability, "id") == id)
        }) {
            if control(
                ui,
                &tf(
                    ui,
                    "Suggested · {0}",
                    &[ability_text(suggested, "name", language)],
                ),
                true,
            )
            .clicked()
            {
                self.create.select_ability(text(suggested, "id"));
            }
        } else if self.create.crew.single {
            hint(ui, "Choose a type");
        } else {
            hint(
                ui,
                "Crew plans use bundled guides. Review before preparing.",
            );
        }
    }

    fn chat_composer_options(&mut self, ui: &mut Ui) {
        let id = egui::Id::new(("chat-composer-options", &self.create.project));
        ui.scope_builder(egui::UiBuilder::new().id(id), |ui| {
            ui.horizontal(|ui| {
                if view_control(ui, "Skills", true, false).clicked() {
                    self.create.crew.catalog_target = None;
                    self.create.catalog_open = true;
                }
                if view_control(ui, "Details", true, false).clicked() {
                    self.create.chat.details_open = true;
                }
                egui::ComboBox::from_id_salt("conversation-work-mode")
                    .selected_text(t(
                        ui,
                        if self.create.crew.single {
                            "Single task"
                        } else {
                            "Crew"
                        },
                    ))
                    .width(94.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.create.crew.single, false, t(ui, "Crew"));
                        ui.selectable_value(
                            &mut self.create.crew.single,
                            true,
                            t(ui, "Single task"),
                        );
                    });
            });
        });
    }

    fn chat_history(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let page = self.create.chat_page(snapshot);
        if page.is_none_or(|page| !array(&page["items"]).is_empty()) || self.create.chat.offset > 0
        {
            ui.horizontal_wrapped(|ui| {
                if studio::compact_button(
                    ui,
                    t(ui, "Refresh conversation").as_ref(),
                    studio::ButtonKind::Ghost,
                    !self.hold_automatic_reads,
                )
                .clicked()
                {
                    self.create.chat.refresh = true;
                }
                if self.create.chat.offset > 0
                    && control(
                        ui,
                        "Previous messages",
                        self.create.chat.offset > 0 && !self.hold_automatic_reads,
                    )
                    .clicked()
                {
                    self.create.chat.offset = self.create.chat.offset.saturating_sub(20);
                }
                if let Some(next) = page.and_then(|page| page["nextOffset"].as_u64()) {
                    if control(ui, "Next messages", !self.hold_automatic_reads).clicked() {
                        self.create.chat.offset = next as usize;
                    }
                }
            });
        }
        let Some(page) = self.create.chat_page(snapshot) else {
            hint(ui, "Loading this project's conversation…");
            self.crew_chat_content(ui, snapshot, commands);
            return;
        };
        if array(&page["items"]).is_empty() && !self.create.crew_has_history(snapshot) {
            self.chat_welcome(ui, snapshot, commands);
            return;
        }
        let project = self.create.project.clone();
        for entry in array(&page["items"]).iter().filter(|entry| {
            job_belongs(snapshot, &project, text(entry, "workItemId"))
                && !crew::contains_work(snapshot, &project, text(entry, "workItemId"))
        }) {
            ui.push_id(text(entry, "workItemId"), |ui| {
                ui.add_space(f32::from(studio::metrics::PADDING));
                let owner_width = ui.available_width() * 0.86;
                ui.horizontal(|ui| {
                    ui.add_space(
                        (ui.available_width() - owner_width - ui.spacing().item_spacing.x).max(0.0),
                    );
                    studio::card(ui)
                        .fill(colors(ui).info_surface)
                        .stroke(egui::Stroke::new(
                            1.0,
                            colors(ui).primary.gamma_multiply(0.18),
                        ))
                        .corner_radius(studio::metrics::CARD_RADIUS)
                        .inner_margin(studio::metrics::PADDING)
                        .show(ui, |ui| {
                            ui.set_max_width((owner_width - 32.0).max(100.0));
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                ui.label(
                                    RichText::new(t(ui, "You"))
                                        .size(12.0)
                                        .color(colors(ui).primary),
                                );
                                ui.add(
                                    egui::Label::new(text(entry, "goal"))
                                        .wrap()
                                        .selectable(true),
                                );
                                if entry["replyToWorkItemId"].is_string() {
                                    hint(ui, "Follow-up request");
                                }
                            });
                        });
                });
                ui.add_space(studio::metrics::CONTROL_GAP);
                self.chat_work_card(ui, snapshot, entry, commands);
                ui.add_space(studio::metrics::CONTROL_GAP);
            });
        }
        self.crew_chat_content(ui, snapshot, commands);
    }

    fn chat_welcome(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        let compact = ui.ctx().viewport_rect().height() < 700.0;
        ui.add_space(if compact { 0.0 } else { 24.0 });
        ui.vertical_centered(|ui| {
            crate::motion::mox(
                ui,
                if compact { 40.0 } else { 80.0 },
                false,
                snapshot["desktopPreferences"]["reduceMotion"] == true,
            );
            ui.add_space(if compact { 4.0 } else { 12.0 });
            ui.label(studio::title(
                t(ui, "What would you like to make?"),
                if compact { 26.0 } else { 32.0 },
            ));
            ui.add_space(4.0);
            ui.label(
                RichText::new(t(ui, "One idea. Start with a direction."))
                    .size(if compact { 13.0 } else { 15.0 })
                    .color(colors(ui).muted),
            );
        });
        ui.add_space(if compact { 12.0 } else { 24.0 });
        self.chat_composer_frame(ui, snapshot, commands);
        ui.add_space(if compact {
            studio::metrics::CONTROL_GAP
        } else {
            studio::metrics::SECTION_GAP
        });
        let columns = if ui.available_width() >= 500.0 { 4 } else { 2 };
        let width =
            (ui.available_width() - studio::metrics::GAP * (columns - 1) as f32) / columns as f32;
        let selected_category = array(&snapshot["abilities"])
            .iter()
            .find(|ability| text(ability, "id") == self.create.ability)
            .map(|ability| text(ability, "category"));
        egui::Grid::new("conversation-categories")
            .num_columns(columns)
            .spacing(egui::vec2(studio::metrics::GAP, studio::metrics::GAP))
            .show(ui, |ui| {
                for (index, category) in [
                    "presentations",
                    "video",
                    "images",
                    "audio",
                    "research",
                    "documents",
                    "data",
                    "web",
                ]
                .iter()
                .enumerate()
                {
                    let available = array(&snapshot["abilities"])
                        .iter()
                        .find(|ability| text(ability, "category") == *category);
                    let selected = selected_category == Some(category);
                    let label = t(
                        ui,
                        if *category == "web" {
                            "Web"
                        } else {
                            category_name(category)
                        },
                    );
                    let response = ui
                        .add_enabled_ui(available.is_some(), |ui| {
                            ui.allocate_exact_size(
                                egui::vec2(width, if compact { 48.0 } else { 64.0 }),
                                egui::Sense::click(),
                            )
                            .1
                        })
                        .inner
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Button,
                            response.enabled(),
                            selected,
                            &label,
                        )
                    });
                    let p = colors(ui);
                    let focused = response.has_focus() || response.hovered();
                    ui.painter().rect(
                        response.rect,
                        10,
                        if selected {
                            p.info_surface
                        } else if focused {
                            p.raised
                        } else {
                            egui::Color32::TRANSPARENT
                        },
                        egui::Stroke::new(
                            1.0,
                            if selected || focused {
                                p.primary
                            } else {
                                egui::Color32::TRANSPARENT
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    let icon_size = if compact { 16.0 } else { 20.0 };
                    let icon_rect = egui::Rect::from_center_size(
                        egui::pos2(
                            response.rect.center().x,
                            response.rect.top() + if compact { 13.0 } else { 20.0 },
                        ),
                        egui::vec2(icon_size, icon_size),
                    );
                    let mut icon_ui = ui.new_child(egui::UiBuilder::new().max_rect(icon_rect));
                    studio::icon(
                        &mut icon_ui,
                        category,
                        icon_size,
                        if selected { p.primary } else { p.muted },
                    );
                    ui.painter().text(
                        egui::pos2(
                            response.rect.center().x,
                            response.rect.bottom() - if compact { 11.0 } else { 15.0 },
                        ),
                        egui::Align2::CENTER_CENTER,
                        &label,
                        egui::FontId::new(
                            if compact { 12.0 } else { 13.0 },
                            egui::FontFamily::Name("studio-display".into()),
                        ),
                        if selected { p.text } else { p.muted },
                    );
                    #[cfg(test)]
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new(("test-action", "category", category)),
                            response.rect,
                        );
                        data.insert_temp(
                            egui::Id::new(("test-category-state", category)),
                            (response.enabled(), selected, response.rect, ui.clip_rect()),
                        );
                        data.insert_temp(
                            egui::Id::new(("test-category-id", category)),
                            response.id,
                        );
                    });
                    if response.enabled()
                        && (response.clicked()
                            || (response.has_focus()
                                && ui.input(|input| {
                                    input.key_pressed(egui::Key::Enter)
                                        || input.key_pressed(egui::Key::Space)
                                })))
                    {
                        if let Some(ability) = available {
                            self.create.select_ability(text(ability, "id"));
                            self.create.category = (*category).into();
                        }
                    }
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(f32::from(studio::metrics::PADDING));
    }

    fn chat_work_card(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        entry: &Value,
        commands: &mut Vec<Command>,
    ) {
        let Some(work) = items(&snapshot["work"])
            .iter()
            .find(|work| work["workItemId"] == entry["workItemId"])
        else {
            return;
        };
        studio::card(ui)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    crate::motion::mox(ui,36.0,matches!(text(work,"status"),"launching"|"running"|"cancelling"),snapshot["desktopPreferences"]["reduceMotion"]==true);
                    ui.vertical(|ui| {
                        ui.label(studio::title("Mox",20.0));
                        ui.label(RichText::new(t(ui,"Work status")).size(12.0).color(colors(ui).muted));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| status(ui,text(work,"status")));
                });
                ui.add_space(studio::metrics::GAP);
                hint(ui,match text(work,"status") {"queued"=>"Prepared. Start when you're ready.","launching"|"running"|"cancelling"=>"Working in this project.","completed"=>"Work finished. Review the actual files.",_=>"Work stopped or needs attention. Review its status and files."});
                let issue=job_issue(snapshot,&self.create.project,text(work,"workItemId"));
                if let Some(issue)=issue {hint(ui,issue);}
                ui.add_space(studio::metrics::GAP);
                ui.horizontal_wrapped(|ui| {
                    if text(work,"status")=="queued" {
                        action(ui,commands,"Start task","work_start",connect::can_start(snapshot) && issue.is_none() && revision(work)>0,
                            ||json!({"workItemId":work["workItemId"],"expectedItemRevision":work["revision"]}));
                    }
                    if matches!(text(work,"status"),"failed"|"interrupted"|"blocked") {
                        action(ui,commands,"Queue retry","work_retry",revision(work)>0,||json!({"workItemId":work["workItemId"],"expectedItemRevision":work["revision"]}));
                    }
                    if let Some(run)=array(&snapshot["work"]["runs"]).iter().rev().find(|run|run["workItemId"]==work["workItemId"] && text(run,"status")=="running") {
                        action(ui,commands,"Cancel run","work_cancel",run["ownerEpoch"]==snapshot["hostEpoch"] && revision(run)>0,
                            ||json!({"runId":run["runId"],"expectedRunRevision":run["revision"]}));
                    }
                    let before=commands.len();
                    action(ui,commands,"Files","project_results_read",issue.is_none() && !cfg!(target_os="windows"),
                        ||json!({"projectId":self.create.project,"workItemId":work["workItemId"]}));
                    if commands.len()>before {
                        self.create.selected_work.insert(self.create.project.clone(),text(work,"workItemId").into());
                        self.create.result_path.clear();self.create.chat.files_open=true;
                        self.create.crew.panel_open=false;
                    }
                    if control(ui,"Follow up",terminal(text(work,"status"))).clicked() {self.begin_chat_reply(snapshot,work);}
                });
                if text(work,"status")=="queued" && !connect::can_start(snapshot) {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {hint(ui,"Connect a supported Codex account to start.");if control(ui,"Set up Codex",true).clicked() {self.create.chat.connections_open=true;}});
                }
            });
    }

    fn chat_files(&mut self, ui: &mut Ui, snapshot: &Value, commands: &mut Vec<Command>) {
        ui.horizontal_wrapped(|ui| {
            ui.label(studio::title(t(ui, "Project files"), 20.0));
            if control(ui, "Close files", true).clicked() {
                self.create.chat.files_open = false;
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("conversation-file-list")
            .max_height((ui.ctx().viewport_rect().height() - 150.0).max(160.0))
            .show(ui, |ui| self.create_jobs(ui, snapshot, commands));
    }

    fn begin_chat_reply(&mut self, snapshot: &Value, work: &Value) {
        if !job_belongs(snapshot, &self.create.project, text(work, "workItemId"))
            || !terminal(text(work, "status"))
        {
            return;
        }
        self.create.crew.single = true;
        if self
            .create
            .chat
            .replies
            .get(&(self.create.project.clone(), self.create.ability.clone()))
            .is_some_and(|reply| reply.parent == text(work, "workItemId"))
        {
            return;
        }
        if let Some(ability) = self
            .create
            .chat_page(snapshot)
            .and_then(|page| {
                array(&page["items"])
                    .iter()
                    .find(|entry| entry["workItemId"] == work["workItemId"])
            })
            .map(|entry| text(entry, "abilityId"))
            .filter(|id| {
                array(&snapshot["abilities"])
                    .iter()
                    .any(|ability| text(ability, "id") == *id)
            })
        {
            self.create.select_ability(ability);
        }
        let reply = self
            .create
            .chat
            .replies
            .entry((self.create.project.clone(), self.create.ability.clone()))
            .or_default();
        if reply.parent != text(work, "workItemId") {
            reply.parent = text(work, "workItemId").into();
            reply.references.clear();
        }
    }

    pub(super) fn chat_reference_choice(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        work: &Value,
        result: &Value,
    ) {
        if !terminal(text(work, "status")) {
            return;
        }
        let key = (self.create.project.clone(), self.create.ability.clone());
        let mut selected = self.create.chat.replies.get(&key).is_some_and(|reply| {
            reply.parent == text(work, "workItemId")
                && reply.references.contains(text(result, "path"))
        });
        let count = self
            .create
            .chat
            .replies
            .get(&key)
            .map(|reply| reply.references.len())
            .unwrap_or(0);
        let response = ui.add_enabled(
            selected || count < 8,
            egui::Checkbox::new(&mut selected, t(ui, "Use in follow-up")),
        );
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("test-action", "reference", text(result, "path"))),
                response.rect,
            )
        });
        if response.changed() {
            self.begin_chat_reply(snapshot, work);
            let reply = self
                .create
                .chat
                .replies
                .entry((self.create.project.clone(), self.create.ability.clone()))
                .or_default();
            if selected {
                reply.references.insert(text(result, "path").into());
            } else {
                reply.references.remove(text(result, "path"));
            }
        }
    }

    fn chat_details(&mut self, ui: &mut Ui, snapshot: &Value) {
        if !self.create.chat.details_open {
            return;
        }
        let id = egui::Id::new("chat-task-details");
        let modal=egui::Modal::new(id).area(egui::Modal::default_area(id).fade_in(false)).frame(studio::card(ui)).show(ui.ctx(),|ui| {
            ui.set_width((ui.ctx().viewport_rect().width()-64.0).clamp(280.0,600.0));
            ui.label(studio::title(t(ui,"Task details"),24.0));
            field(ui,"Task name (optional)",&mut self.create.draft().title,160,false);
            egui::ScrollArea::vertical().id_salt("chat-details-body").max_height((ui.ctx().viewport_rect().height()-260.0).max(100.0)).show(ui,|ui| {
                if let Some(ability)=array(&snapshot["abilities"]).iter().find(|ability|text(ability,"id")==self.create.ability) {
                    ui.strong(ability_text(ability,"name",Language::from_snapshot(snapshot)));
                    ui.label(text(ability,"instruction"));
                }
                hint(ui,"Task recipes guide your agent. Results depend on available tools and access.");
                hint(ui,"Your provider may receive this goal and the accepted decisions for this project when you start.");
            });
            if control(ui,"Close",true).clicked() {self.create.chat.details_open=false;}
        });
        if modal.should_close() {
            self.create.chat.details_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suggestions_are_goal_based_and_ambiguity_never_silently_selects() {
        assert_eq!(
            suggestion("Make a presentation about our launch"),
            Some("P1")
        );
        assert_eq!(suggestion("Bu videoya altyazı ekle"), Some("V1"));
        assert_eq!(suggestion("Build a presentation and video"), None);
        assert_eq!(suggestion("Help me with my project"), None);
        assert!(!terminal("blocked"));
    }

    fn fixture() -> Value {
        json!({"projectMemory":{"projects":[{"projectId":"project:one","revision":2},{"projectId":"project:two","revision":3}]},
            "abilities":[{"id":"P1"}],"work":{"revision":4,"items":[{"workItemId":"work:old","status":"completed"}]},
            "projectJobs":[{"projectId":"project:one","workItemId":"work:old"}]})
    }
    fn draft() -> CreateState {
        let mut state = CreateState {
            project: "project:one".into(),
            ability: "P1".into(),
            ..Default::default()
        };
        state.draft().goal = "Revise the introduction".into();
        state
    }

    #[test]
    fn chat_context_resizes_preserve_search_focus_and_the_conversation_draft() {
        use super::super::tests::{click_at_size, render};
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language,"reduceMotion":true});
                data["projectChat"] = json!({"reasonCode":"PROJECT_CHAT_READ","projectId":"project:one","workRevision":4,"offset":0,"items":[{"projectId":"project:one","workItemId":"work:old","instruction":"Earlier project request","abilityId":"P1"}]});
                data["projectStudio"] = json!({"reasonCode":"PROJECT_STUDIO_READ","projectId":"project:one","artifacts":[{"workItemId":"work:old","name":"Özet.md","path":"outputs/Özet.md","sha256":"a".repeat(64),"bytes":42}]});
                let mut console = Console {
                    tab: Tab::Create,
                    create: draft(),
                    hold_automatic_reads: true,
                    ..Default::default()
                };
                console.create.select_productive_view(2);
                let wide = egui::vec2(1280.0, 820.0);
                for _ in 0..3 {
                    render(&ctx, &mut console, &data, wide, false, vec![]);
                }
                assert!(click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    wide,
                    false,
                    "memory_control",
                    "Search outputs"
                )
                .is_empty());
                render(
                    &ctx,
                    &mut console,
                    &data,
                    wide,
                    false,
                    vec![egui::Event::Text("Özet".into())],
                );
                let (_, search_id) = ctx.data(|d| {
                    d.get_temp::<(egui::Rect, egui::Id)>(egui::Id::new("delivery-search"))
                        .unwrap()
                });
                for size in [
                    wide,
                    egui::vec2(1360.0, 820.0),
                    egui::vec2(1392.0, 820.0),
                    egui::vec2(1400.0, 820.0),
                    egui::vec2(800.0, 560.0),
                    wide,
                ] {
                    for _ in 0..3 {
                        render(&ctx, &mut console, &data, size, false, vec![]);
                    }
                    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                    let (search, current_id) = ctx.data(|d| {
                        d.get_temp::<(egui::Rect, egui::Id)>(egui::Id::new("delivery-search"))
                            .unwrap()
                    });
                    assert!(
                        screen.contains_rect(search),
                        "{language}/{theme:?}/{size:?}: {search:?}"
                    );
                    assert_eq!(
                        current_id, search_id,
                        "Context controls keep identity across layouts"
                    );
                    assert!(
                        ctx.memory(|memory| memory.has_focus(search_id)),
                        "Search focus survives resize"
                    );
                    assert_eq!(console.create.draft().goal, "Revise the introduction");
                    if size.x >= 1280.0 {
                        let goal = ctx.data(|d| {
                            d.get_temp::<egui::Rect>(egui::Id::new("create-goal-prompt"))
                                .unwrap()
                        });
                        let context = ctx.data(|d| {
                            d.get_temp::<egui::Rect>(egui::Id::new("project-context-bounds"))
                                .unwrap()
                        });
                        assert!(
                            screen.contains_rect(goal) && goal.right() <= context.left(),
                            "Composer stays beside context: {goal:?}/{context:?}"
                        );
                        let (frame, clip) = ctx.data(|d| {
                            d.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new(
                                "chat-composer-bounds",
                            ))
                            .unwrap()
                        });
                        assert!(
                            screen.contains_rect(frame) && clip.contains_rect(frame),
                            "{language}/{theme:?}: complete composer visible: {frame:?}/{clip:?}"
                        );
                    }
                }
                let close = ctx.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new(("project-view-tab", 2usize)))
                        .unwrap()
                });
                for pressed in [true, false] {
                    render(
                        &ctx,
                        &mut console,
                        &data,
                        wide,
                        false,
                        vec![
                            egui::Event::PointerMoved(close.center()),
                            egui::Event::PointerButton {
                                pos: close.center(),
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    );
                }
                assert_eq!(console.create.productive_view(), 0);
                assert_eq!(console.create.draft().goal, "Revise the introduction");
            }
        }
    }

    #[test]
    fn project_view_navigation_fits_minimum_window_and_preserves_the_unsent_goal() {
        for language in ["tr", "en"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut snapshot = fixture();
                snapshot["desktopPreferences"] = json!({"language":language,"reduceMotion":true});
                snapshot["projectMemory"]["projects"][0]["name"] = json!("Launch project");
                let mut console = Console {
                    tab: Tab::Create,
                    create: draft(),
                    ..Default::default()
                };
                let mut focused_route = None;
                for width in [1280.0, 800.0, 1280.0] {
                    let screen =
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 560.0));
                    let frame = |console: &mut Console, events: Vec<egui::Event>| {
                        ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(screen),
                                events,
                                ..Default::default()
                            },
                            |ui| {
                                let _ = console.show(ui, &snapshot, false, None, false);
                            },
                        )
                        .drop_without_applying_deltas();
                    };
                    for view in [1usize, 2, 0] {
                        frame(&mut console, vec![]);
                        frame(&mut console, vec![]);
                        if view == 1 {
                            if let Some(id) = focused_route {
                                assert!(
                                    ctx.memory(|memory| memory.has_focus(id)),
                                    "Sidebar focus survives responsive layout"
                                );
                            }
                        }
                        let rect = ctx.data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new(("project-view-tab", view)))
                                .unwrap()
                        });
                        assert!(screen.contains_rect(rect), "{language}/{theme:?}/{view}");
                        let other = ctx.data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new((
                                "project-view-tab",
                                (view + 1) % 3,
                            )))
                            .unwrap()
                        });
                        assert!(
                            (rect.top() - other.top()).abs() < 1.0,
                            "Project views share one header row"
                        );
                        let navigation = ctx.data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new((
                                "test-action",
                                "navigation",
                                "Chat",
                            )))
                            .unwrap()
                        });
                        assert!(screen.contains_rect(navigation));
                        if width < 1000.0 {
                            assert!(navigation.width() < 60.0, "Resizing releases content width");
                        }
                        for pressed in [true, false] {
                            frame(
                                &mut console,
                                vec![
                                    egui::Event::PointerMoved(rect.center()),
                                    egui::Event::PointerButton {
                                        pos: rect.center(),
                                        button: egui::PointerButton::Primary,
                                        pressed,
                                        modifiers: egui::Modifiers::default(),
                                    },
                                ],
                            );
                        }
                        assert_eq!(console.create.productive_view(), view);
                        assert_eq!(console.create.draft().goal, "Revise the introduction");
                    }
                    let route = ctx.data(|data| {
                        data.get_temp::<egui::Id>(egui::Id::new(("navigation-id", "Chat")))
                            .unwrap()
                    });
                    ctx.memory_mut(|memory| memory.request_focus(route));
                    focused_route = Some(route);
                }
                let project_route = ctx.data(|data| {
                    data.get_temp::<egui::Id>(egui::Id::new(("navigation-id", "Projects")))
                        .unwrap()
                });
                ctx.memory_mut(|memory| memory.request_focus(project_route));
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(800.0, 560.0),
                        )),
                        events: vec![egui::Event::Key {
                            key: egui::Key::Enter,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::default(),
                        }],
                        ..Default::default()
                    },
                    |ui| {
                        let _ = console.show(ui, &snapshot, false, None, false);
                    },
                )
                .drop_without_applying_deltas();
                assert!(console.tab == Tab::Projects);
                assert_eq!(console.create.draft().goal, "Revise the introduction");
            }
        }
    }

    #[test]
    fn chat_read_is_bounded_scope_checked_and_never_retries_itself() {
        let mut state = draft();
        let mut snapshot = fixture();
        assert!(state.next_chat_read(&snapshot, true).is_none());
        let read = state.next_chat_read(&snapshot, false).unwrap();
        assert_eq!(read.action, "project_chat_read");
        assert_eq!(read.payload["projectId"], "project:one");
        assert_eq!(read.payload["limit"], 20);
        assert!(
            state.next_chat_read(&snapshot, false).is_none(),
            "failed/unavailable response must not spin"
        );
        snapshot["projectChat"] = json!({"reasonCode":"PROJECT_CHAT_READ","projectId":"project:two","offset":0,"workRevision":4,"items":[{"goal":"FOREIGN_GOAL"}]});
        assert!(state.chat_page(&snapshot).is_none());
        snapshot["projectChat"]["projectId"] = json!("project:one");
        assert!(state.chat_page(&snapshot).is_some());
        snapshot["work"]["revision"] = json!(5);
        assert!(state.chat_page(&snapshot).is_none());
        assert!(state.next_chat_read(&snapshot, true).is_none());
        assert!(state.next_chat_read(&snapshot, false).is_some());
        state.chat.offset = 20;
        assert_eq!(
            state.next_chat_read(&snapshot, false).unwrap().payload["offset"],
            20
        );
        state.project = "project:two".into();
        let read = state.next_chat_read(&snapshot, false).unwrap();
        assert_eq!(read.payload["projectId"], "project:two");
        assert_eq!(read.payload["offset"], 0);
    }

    #[test]
    fn follow_up_payload_freezes_one_terminal_parent_and_sorted_references() {
        let mut state = draft();
        let mut snapshot = fixture();
        state.chat.replies.insert(
            ("project:one".into(), "P1".into()),
            ReplyDraft {
                parent: "work:old".into(),
                references: HashSet::from(["outputs/b.md".into(), "outputs/a.md".into()]),
            },
        );
        let payload = state.chat_payload(&snapshot).unwrap();
        assert_eq!(payload["replyToWorkItemId"], "work:old");
        assert_eq!(
            payload["referencePaths"],
            json!(["outputs/a.md", "outputs/b.md"])
        );
        let command = Command::new("project_chat_send", payload);
        state.project = "project:two".into();
        assert!(state.chat_payload(&snapshot).is_none());
        assert_eq!(command.payload["projectId"], "project:one");
        state.project = "project:one".into();
        for status in ["running", "queued", "blocked"] {
            snapshot["work"]["items"][0]["status"] = json!(status);
            assert!(
                state.chat_payload(&snapshot).is_none(),
                "{status} is not a safe reference parent"
            );
        }
        snapshot["work"]["items"][0]["status"] = json!("interrupted");
        assert!(state.chat_payload(&snapshot).is_some());
        state
            .chat
            .replies
            .get_mut(&("project:one".into(), "P1".into()))
            .unwrap()
            .references = (0..9).map(|n| format!("outputs/{n}.md")).collect();
        assert!(state.chat_payload(&snapshot).is_none());
        state.chat.replies.clear();
        state.draft().goal = "😀".repeat(10_001);
        assert!(state.chat_payload(&snapshot).is_none());
        state.draft().goal = "😀".repeat(10_000);
        assert!(state.chat_payload(&snapshot).is_some());
    }

    #[test]
    fn applied_chat_receipt_does_not_clear_a_changed_reference_draft() {
        let mut state = draft();
        let mut snapshot = fixture();
        let key = ("project:one".to_string(), "P1".to_string());
        let original = ReplyDraft {
            parent: "work:old".into(),
            references: HashSet::from(["outputs/a.md".into()]),
        };
        state.chat.replies.insert(key.clone(), original.clone());
        state.pending = Some(Submission {
            command_id: "cmd:chat".into(),
            project: "project:one".into(),
            ability: "P1".into(),
            draft: state.draft().clone(),
        });
        state.chat.pending_reply = Some(("cmd:chat".into(), Some(original)));
        state
            .chat
            .replies
            .get_mut(&key)
            .unwrap()
            .references
            .insert("outputs/b.md".into());
        snapshot["projectWorkReceipt"] = json!({"commandId":"cmd:chat","projectId":"project:one","workItemId":"work:old","outcome":"applied"});
        state.sync(&snapshot);
        assert_eq!(state.draft().goal, "Revise the introduction");
        assert_eq!(state.chat.replies[&key].references.len(), 2);
    }
}
