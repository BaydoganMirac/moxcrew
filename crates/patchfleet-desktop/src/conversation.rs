use super::*;

type Scope = (String, String);

fn context_text(answers: &[Value]) -> String {
    if answers.iter().all(|answer| {
        text(answer, "id")
            .strip_prefix("chat-")
            .is_some_and(|suffix| suffix.parse::<usize>().is_ok())
    }) {
        answers
            .iter()
            .filter_map(|answer| answer["answer"].as_str())
            .collect()
    } else {
        answers
            .iter()
            .map(|answer| format!("{}: {}", text(answer, "id"), text(answer, "answer")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn pack_context(previous: &[Value], questions: &str, input: &str) -> Option<Vec<Value>> {
    let mut transcript = context_text(previous);
    if !questions.is_empty() {
        transcript.push_str("\nMox:\n");
        transcript.push_str(questions);
    }
    transcript.push_str("\nOwner:\n");
    transcript.push_str(input);
    let mut chunks = vec![String::new()];
    let mut units = 0;
    for character in transcript.chars() {
        if units + character.len_utf16() > 2000 {
            if chunks.len() == 8 {
                return None;
            }
            chunks.push(String::new());
            units = 0;
        }
        units += character.len_utf16();
        chunks.last_mut().unwrap().push(character);
    }
    Some(
        chunks
            .into_iter()
            .enumerate()
            .map(|(index, answer)| json!({"id":format!("chat-{}",index+1),"answer":answer}))
            .collect(),
    )
}

struct Turn {
    speaker: &'static str,
    body: String,
}

struct Session {
    original: CreateDraft,
    input: String,
    answers: Vec<Value>,
    turns: Vec<Turn>,
}

struct ProjectRequest {
    command: String,
    key: Scope,
    draft: CreateDraft,
    reply: Option<ReplyDraft>,
    provider: String,
    preference: String,
}

#[derive(Default)]
pub(super) struct ConversationState {
    sessions: HashMap<Scope, Session>,
    creation: Option<ProjectRequest>,
    error: Option<&'static str>,
}

impl ConversationState {
    pub(super) fn preserve_inputs(&self, drafts: &mut HashMap<Scope, CreateDraft>) {
        for (key, session) in &self.sessions {
            if !session.input.is_empty() {
                drafts.entry(key.clone()).or_default().goal = session.input.clone();
            }
        }
    }
}

impl CreateState {
    pub(super) fn displayed_planner_provider(&self, snapshot: &Value) -> Option<&str> {
        self.chat
            .conversation
            .creation
            .as_ref()
            .filter(|request| request.key == self.conversation_key())
            .map(|request| request.provider.as_str())
            .or_else(|| self.active_planner_provider())
            .or_else(|| self.resolved_planner_provider(snapshot))
    }

    fn conversation_key(&self) -> Scope {
        (self.project.clone(), self.ability.clone())
    }

    pub(super) fn has_conversation(&self) -> bool {
        self.chat
            .conversation
            .sessions
            .contains_key(&self.conversation_key())
    }

    pub(super) fn conversation_creating(&self) -> bool {
        self.chat.conversation.creation.is_some()
    }

    pub(super) fn conversation_input(&mut self) -> &mut String {
        let key = self.conversation_key();
        if self.chat.conversation.sessions.contains_key(&key) {
            return &mut self.chat.conversation.sessions.get_mut(&key).unwrap().input;
        }
        &mut self.draft().goal
    }

    pub(super) fn conversation_limit(&self) -> usize {
        if self.has_conversation() {
            2000
        } else {
            20_000
        }
    }

    pub(super) fn conversation_send_enabled(&mut self, snapshot: &Value) -> bool {
        let limit = self.conversation_limit();
        valid_input(self.conversation_input(), limit)
            && !self.conversation_creating()
            && !self.planner_busy()
            && self.pending.is_none()
            && self
                .chat
                .conversation
                .sessions
                .get(&self.conversation_key())
                .is_none_or(|session| pack_context(&session.answers, "", &session.input).is_some())
            && self.resolved_planner_provider(snapshot).is_some()
    }
}

impl Console {
    pub(super) fn restore_conversation(&mut self, snapshot: &Value) {
        let key = self.create.conversation_key();
        if self.create.has_conversation() {
            return;
        }
        let full = &snapshot["projectCrewPlanJob"];
        if text(full, "reasonCode") != "PROJECT_CREW_PLAN_READ"
            || text(full, "projectId") != key.0
            || !self
                .create
                .planner_job()
                .is_some_and(|job| job["planJobId"] == full["planJobId"])
            || !valid_input(text(full, "instruction"), 20_000)
        {
            return;
        }
        let original = CreateDraft {
            title: text(full, "title").into(),
            goal: text(full, "instruction").into(),
        };
        let input = self.create.draft().goal.clone();
        if input.is_empty() {
            *self.create.draft() = original.clone();
        }
        let answers = array(&full["answers"]).to_vec();
        let mut turns = vec![Turn {
            speaker: "You",
            body: original.goal.clone(),
        }];
        if !answers.is_empty() {
            turns.push(Turn {
                speaker: "Planning context",
                body: context_text(&answers),
            });
        }
        let reply = full["replyToWorkItemId"].as_str().map(|parent| ReplyDraft {
            parent: parent.into(),
            references: array(&full["referencePaths"])
                .iter()
                .filter_map(|path| path.as_str().map(str::to_owned))
                .collect(),
        });
        if !self.create.chat.replies.contains_key(&key) {
            if let Some(reply) = &reply {
                self.create.chat.replies.insert(key.clone(), reply.clone());
            }
        }
        self.create.chat.conversation.sessions.insert(
            key,
            Session {
                original,
                input,
                answers,
                turns,
            },
        );
    }

    #[cfg(feature = "ui-smoke")]
    pub fn conversation_smoke(&mut self, snapshot: &Value, scene: &str) {
        if matches!(scene, "conversation-plan" | "conversation-questions") {
            self.productive_smoke(snapshot, "productive-plan");
            let key = self.create.conversation_key();
            let draft = self.create.draft().clone();
            self.create.chat.conversation.sessions.insert(
                key,
                Session {
                    turns: vec![Turn {
                        speaker: "You",
                        body: draft.goal.clone(),
                    }],
                    original: draft,
                    input: String::new(),
                    answers: Vec::new(),
                },
            );
        } else {
            self.create_smoke(snapshot, false);
            if scene == "conversation-empty" {
                self.create.project.clear();
                self.create.ability.clear();
                self.create.draft().goal.clear();
            }
        }
        self.create.crew.single = false;
        self.create.planner_provider().clear();
        self.hold_automatic_reads = true;
        self.create.select_productive_view(0);
    }

    pub(super) fn sync_conversation_project(
        &mut self,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        let Some(pending) = &self.create.chat.conversation.creation else {
            return;
        };
        let receipt = &snapshot["projectCreateReceipt"];
        if text(receipt, "commandId") != pending.command {
            return;
        }
        if matches!(text(receipt, "outcome"), "" | "unknown") {
            return;
        }
        if text(receipt, "outcome") != "applied" {
            self.create.chat.conversation.creation = None;
            self.create.chat.conversation.error =
                Some("Project creation failed. Your message is still here; send again when ready.");
            return;
        }
        let project = text(receipt, "projectId");
        if project.is_empty() || output_studio::project_revision(snapshot, project).is_none() {
            return;
        }
        if !commands.is_empty() {
            return;
        }
        let pending = self.create.chat.conversation.creation.take().unwrap();
        if self.create.conversation_key() != pending.key
            || self.create.drafts.get(&pending.key) != Some(&pending.draft)
            || self.create.chat.replies.get(&pending.key) != pending.reply.as_ref()
            || *self.create.planner_provider() != pending.preference
        {
            return;
        }
        let next = (project.to_owned(), pending.key.1.clone());
        if self.create.drafts.contains_key(&next) {
            return;
        }
        self.create.project = project.to_owned();
        self.create.drafts.remove(&pending.key);
        self.create
            .drafts
            .insert(next.clone(), pending.draft.clone());
        if let Some(reply) = pending.reply {
            self.create.chat.replies.remove(&pending.key);
            self.create.chat.replies.insert(next, reply);
        }
        if !connect::can_attempt_provider(snapshot, &pending.provider) {
            self.create.chat.conversation.error =
                Some("Selected provider is unavailable. Check connections, then send again.");
            return;
        }
        self.send_conversation_plan(snapshot, commands, pending.provider);
    }

    pub(super) fn send_conversation(&mut self, snapshot: &Value, commands: &mut Vec<Command>) {
        if !commands.is_empty() || !self.create.conversation_send_enabled(snapshot) {
            return;
        }
        self.create.chat.conversation.error = None;
        let Some(provider) = self
            .create
            .resolved_planner_provider(snapshot)
            .map(str::to_owned)
        else {
            return;
        };
        if output_studio::project_revision(snapshot, &self.create.project).is_none() {
            let draft = self.create.draft().clone();
            let Some(name) = draft_title(&CreateDraft {
                title: String::new(),
                goal: draft.goal.clone(),
            }) else {
                return;
            };
            let command = Command::new("project_create", json!({"name":name}));
            let key = self.create.conversation_key();
            self.create.chat.conversation.creation = Some(ProjectRequest {
                command: text(&command.payload, "commandId").into(),
                reply: self.create.chat.replies.get(&key).cloned(),
                key,
                draft,
                provider,
                preference: self.create.planner_provider().clone(),
            });
            commands.push(command);
            return;
        }
        self.send_conversation_plan(snapshot, commands, provider);
    }

    fn send_conversation_plan(
        &mut self,
        snapshot: &Value,
        commands: &mut Vec<Command>,
        provider: String,
    ) {
        let key = self.create.conversation_key();
        let input = self.create.conversation_input().clone();
        let questions = self
            .create
            .planner_job()
            .filter(|job| text(job, "status") == "needs_input")
            .map(|job| {
                array(&job["questions"])
                    .iter()
                    .map(|q| text(q, "question"))
                    .filter(|q| !q.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let existing = self.create.chat.conversation.sessions.get(&key);
        let reply = self.create.chat.replies.get(&key).cloned();
        let (draft, answers) = if let Some(session) = existing {
            if !valid_input(&input, 2000) {
                return;
            }
            let Some(answers) = pack_context(&session.answers, &questions, &input) else {
                self.create.chat.conversation.error = Some(
                    "Planning context is full. Your message is preserved; start a new request.",
                );
                return;
            };
            (session.original.clone(), answers)
        } else {
            (self.create.draft().clone(), Vec::new())
        };
        if self.create.draft().goal.is_empty() {
            *self.create.draft() = draft.clone();
        }
        self.begin_conversation_plan(
            snapshot,
            commands,
            draft.clone(),
            reply,
            provider,
            answers.clone(),
        );
        if commands.is_empty() {
            return;
        }
        if self.create.draft().goal == input {
            *self.create.draft() = draft.clone();
        }
        let session = self
            .create
            .chat
            .conversation
            .sessions
            .entry(key)
            .or_insert_with(|| Session {
                original: draft,
                input: String::new(),
                answers: Vec::new(),
                turns: Vec::new(),
            });
        if !questions.is_empty() {
            session.turns.push(Turn {
                speaker: "Mox planner",
                body: questions,
            });
        }
        session.turns.push(Turn {
            speaker: "You",
            body: input,
        });
        session.input.clear();
        session.answers = answers;
    }

    pub(super) fn conversation_turns(&self, ui: &mut Ui) {
        let Some(session) = self
            .create
            .chat
            .conversation
            .sessions
            .get(&self.create.conversation_key())
        else {
            return;
        };
        for turn in &session.turns {
            studio::card(ui)
                .fill(if turn.speaker == "You" {
                    colors(ui).info_surface
                } else {
                    colors(ui).surface
                })
                .show(ui, |ui| {
                    hint(ui, turn.speaker);
                    ui.add(egui::Label::new(&turn.body).wrap().selectable(true));
                });
            ui.add_space(studio::metrics::CONTROL_GAP);
        }
    }

    pub(super) fn conversation_feedback(&mut self, ui: &mut Ui) {
        if self.create.conversation_creating() {
            hint(ui, "Creating a project for this conversation…");
        }
        if let Some(error) = self.create.chat.conversation.error {
            hint(ui, error);
        }
        if let Some(session) = self
            .create
            .chat
            .conversation
            .sessions
            .get(&self.create.conversation_key())
        {
            if pack_context(&session.answers, "", &session.input).is_none() {
                hint(
                    ui,
                    "Planning context is full. Your message is preserved; start a new request.",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click_at_size, render, snapshot};
    use super::*;

    fn fixture() -> Value {
        let mut data = snapshot();
        data["abilities"] = patchfleet_core::abilities::catalog();
        data["providerConnections"] = json!({"reasonCode":"PROVIDER_CONNECTIONS_READ","providers":[
            {"id":"codex","installed":true,"version":"0.163.0","authentication":"present","controlSupported":true},
            {"id":"claude","installed":true,"version":"2.1.170","authentication":"present","controlSupported":true},
            {"id":"gemini","installed":false,"authentication":"unknown","controlSupported":false}]});
        data
    }

    fn console() -> Console {
        let mut console = Console::launch();
        console.create.project = "project:one".into();
        console.create.draft().goal = "Prepare a useful launch deck".into();
        console
    }

    fn acknowledge(data: &mut Value, command: &Command, id: &str, status: &str) {
        data["projectWorkflowReceipt"] = json!({"commandId":command.payload["commandId"],"projectId":command.payload["projectId"],"planJobId":id,"revision":1,"outcome":"applied"});
        data["projectCrewPlanJob"] = json!({"reasonCode":"PROJECT_CREW_PLAN_READ","projectId":command.payload["projectId"],"planJobId":id,"revision":2,"providerId":command.payload["providerId"],"status":status,"questions":[{"id":"audience","question":"Who is this for?"}]});
    }

    #[test]
    fn automatic_send_resolves_the_only_usable_provider_before_dispatch() {
        for provider in ["claude", "gemini"] {
            let mut data = fixture();
            data["providerConnections"]["providers"] = json!([{"id":provider,"version":if provider=="claude"{"2.1.170"}else{"0.43.0"},"installed":true,"authentication":"unknown","controlSupported":true}]);
            let mut console = console();
            let mut commands = Vec::new();
            console.send_conversation(&data, &mut commands);
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].action, "project_crew_plan_start");
            assert_eq!(commands[0].payload["providerId"], provider);
            assert!(console.create.planner_provider().is_empty());
            assert_eq!(
                console.create.displayed_planner_provider(&data),
                Some(provider)
            );
        }
    }

    #[test]
    fn automatic_diagnostic_changes_never_retarget_a_frozen_project_chain() {
        for original_still_ready in [true, false] {
            let mut data = fixture();
            data["projectMemory"]["projects"] = json!([]);
            data["providerConnections"]["providers"][0]["installed"] = json!(false);
            let mut console = Console::launch();
            console.create.draft().goal = "Keep this exact intent".into();
            let mut commands = Vec::new();
            console.send_conversation(&data, &mut commands);
            let create = commands.remove(0);
            assert_eq!(
                console
                    .create
                    .chat
                    .conversation
                    .creation
                    .as_ref()
                    .unwrap()
                    .provider,
                "claude"
            );
            data["providerConnections"]["providers"][0]["installed"] = json!(true);
            data["providerConnections"]["providers"][1]["installed"] = json!(original_still_ready);
            assert_eq!(
                console.create.resolved_planner_provider(&data),
                Some("codex")
            );
            assert_eq!(
                console.create.displayed_planner_provider(&data),
                Some("claude")
            );
            data["projectMemory"]["projects"] =
                json!([{"projectId":"project:new","revision":1,"name":"New"}]);
            data["projectCreateReceipt"] = json!({"commandId":create.payload["commandId"],"projectId":"project:new","outcome":"applied"});
            console.sync_conversation_project(&data, &mut commands);
            assert_eq!(console.create.project, "project:new");
            if original_still_ready {
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].payload["providerId"], "claude");
                assert_eq!(
                    console.create.displayed_planner_provider(&data),
                    Some("claude")
                );
            } else {
                assert!(
                    commands.is_empty(),
                    "no automatic retry on a different connection"
                );
                assert_eq!(console.create.draft().goal, "Keep this exact intent");
                console.send_conversation(&data, &mut commands);
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].action, "project_crew_plan_start");
                assert_eq!(
                    commands[0].payload["providerId"], "codex",
                    "only a new explicit Send resolves the new connection"
                );
            }
            let sent = commands.remove(0);
            data["providerConnections"]["providers"][0]["installed"] = json!(false);
            data["providerConnections"]["providers"][1]["installed"] = json!(false);
            assert_eq!(
                console.create.displayed_planner_provider(&data),
                sent.payload["providerId"].as_str()
            );
            console.sync_conversation_project(&data, &mut commands);
            assert!(commands.is_empty());
        }
    }

    fn choose_new_conversation(ctx: &egui::Context, console: &mut Console, data: &Value) {
        let size = egui::vec2(1280.0, 880.0);
        assert!(click_at_size(
            ctx,
            console,
            data,
            size,
            false,
            "memory_control",
            "Conversation project"
        )
        .is_empty());
        for _ in 0..2 {
            assert!(render(ctx, console, data, size, false, vec![]).0.is_empty());
        }
        let rect = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new("conversation-project-new"))
                .unwrap()
        });
        for pressed in [true, false] {
            assert!(render(
                ctx,
                console,
                data,
                size,
                false,
                vec![
                    egui::Event::PointerMoved(rect.center()),
                    egui::Event::PointerButton {
                        pos: rect.center(),
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::default()
                    }
                ]
            )
            .0
            .is_empty());
        }
    }

    #[test]
    fn new_conversation_stays_selected_with_one_project_and_never_resurrects_a_sent_draft() {
        for language in ["en", "tr"] {
            let ctx = egui::Context::default();
            studio::install(&ctx);
            let mut data = fixture();
            data["desktopPreferences"] = json!({"language":language});
            data["projectMemory"]["projects"]
                .as_array_mut()
                .unwrap()
                .truncate(1);
            let mut console = console();
            choose_new_conversation(&ctx, &mut console, &data);
            for _ in 0..3 {
                assert!(render(
                    &ctx,
                    &mut console,
                    &data,
                    egui::vec2(1280.0, 880.0),
                    false,
                    vec![]
                )
                .0
                .is_empty());
            }
            assert!(console.create.project.is_empty());
            assert!(console.create.draft().goal.is_empty());
            assert_eq!(
                console.create.drafts[&("project:one".into(), String::new())].goal,
                "Prepare a useful launch deck"
            );
            console.create.draft().goal = "A completely new conversation".into();
            let commands = click_at_size(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 880.0),
                false,
                "conversation-send",
                "Send",
            );
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].action, "project_create");
            let create = commands[0].clone();
            data["projectCreateReceipt"] = json!({"commandId":create.payload["commandId"],"projectId":"project:new","outcome":"applied"});
            data["projectMemory"]["projects"]
                .as_array_mut()
                .unwrap()
                .push(json!({"projectId":"project:new","name":"New","revision":1}));
            let (commands, _) = render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(1280.0, 880.0),
                false,
                vec![],
            );
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].action, "project_crew_plan_start");
            assert!(!console
                .create
                .drafts
                .contains_key(&(String::new(), String::new())));
            assert_eq!(console.create.project, "project:new");
            choose_new_conversation(&ctx, &mut console, &data);
            for _ in 0..3 {
                assert!(render(
                    &ctx,
                    &mut console,
                    &data,
                    egui::vec2(1280.0, 880.0),
                    false,
                    vec![]
                )
                .0
                .is_empty());
            }
            assert!(console.create.project.is_empty());
            assert!(console.create.conversation_input().is_empty());
            assert_eq!(
                console.create.drafts[&("project:one".into(), String::new())].goal,
                "Prepare a useful launch deck"
            );
            assert_eq!(
                console.create.drafts[&("project:new".into(), String::new())].goal,
                "A completely new conversation"
            );
        }
    }

    #[test]
    fn context_packing_keeps_many_short_turns_and_emoji_exactly_without_truncation() {
        let mut answers = Vec::new();
        let mut expected = String::new();
        for _ in 0..30 {
            let input = "Keep this 🦊 direction, çığ 🙂";
            expected.push_str("\nMox:\nWhich audience?\nOwner:\n");
            expected.push_str(input);
            answers = pack_context(&answers, "Which audience?", input).unwrap();
        }
        assert_eq!(context_text(&answers), expected);
        assert!(answers.len() <= 8);
        assert!(answers
            .iter()
            .all(|answer| text(answer, "answer").encode_utf16().count() <= 2000));
        let nearly_full = vec![json!({"id":"old","answer":"🦊".repeat(7990)})];
        let before = nearly_full.clone();
        assert!(pack_context(&nearly_full, "", "🙂".repeat(30).as_str()).is_none());
        assert_eq!(nearly_full, before);
        let legacy = vec![
            json!({"id":"audience","answer":"b2b"}),
            json!({"id":"format","answer":"slides"}),
        ];
        assert_eq!(context_text(&legacy), "audience: b2b\nformat: slides");
    }

    #[test]
    fn a_fresh_console_reads_latest_project_plan_and_restores_canonical_context_without_overwriting_input(
    ) {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["projectWorkflows"] = json!({"plans":[
            {"projectId":"project:foreign","planJobId":"plan:foreign","createdAt":"2026-10-19T12:00:00Z","revision":9},
            {"projectId":"project:one","planJobId":"plan:older","createdAt":"2026-10-18T12:00:00Z","revision":1},
            {"projectId":"project:one","planJobId":"plan:latest","createdAt":"2026-10-19T12:00:00Z","revision":3,"status":"needs_input"}]});
        let mut console = Console::launch();
        console.create.project = "project:one".into();
        console.create.draft().goal = "Unsent later request".into();
        let (commands, _) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_plan_read");
        assert_eq!(commands[0].payload["planJobId"], "plan:latest");
        let answers = pack_context(&[], "What format?", "Slides please").unwrap();
        data["projectCrewPlanJob"] = json!({"reasonCode":"PROJECT_CREW_PLAN_READ","projectId":"project:one","planJobId":"plan:latest","revision":3,"status":"needs_input","instruction":"Canonical original request","title":"Saved request","answers":answers,"questions":[{"id":"audience","question":"Who will read this?"}]});
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        assert!(commands.is_empty());
        let copy = copy.join("\n");
        assert!(
            copy.contains("Canonical original request")
                && copy.contains("Slides please")
                && copy.contains("Who will read this?")
        );
        assert_eq!(console.create.conversation_input(), "Unsent later request");
        assert_eq!(
            console.create.chat.conversation.sessions[&console.create.conversation_key()].answers,
            answers
        );
        assert_eq!(console.create.project, "project:one");
    }

    #[test]
    fn first_send_creates_once_then_plans_only_after_exact_authoritative_receipt() {
        let mut data = fixture();
        data["projectMemory"]["projects"] = json!([]);
        let mut console = Console::launch();
        console.create.draft().goal = "A first request without setup".into();
        let mut commands = Vec::new();
        console.sync_conversation_project(&data, &mut commands);
        assert!(commands.is_empty());
        console.send_conversation(&data, &mut commands);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_create");
        let command = commands.remove(0);
        data["projectCreateReceipt"] = json!({"commandId":command.payload["commandId"],"outcome":"unknown","reasonCode":"HOST_IO"});
        console.sync_conversation_project(&data, &mut commands);
        assert!(console.create.conversation_creating());
        console.send_conversation(&data, &mut commands);
        assert!(commands.is_empty(), "unknown outcome must not be repeated");
        data["projectCreateReceipt"] =
            json!({"commandId":"foreign","projectId":"project:new","outcome":"applied"});
        console.sync_conversation_project(&data, &mut commands);
        assert!(commands.is_empty());
        data["projectCreateReceipt"]["commandId"] = command.payload["commandId"].clone();
        console.sync_conversation_project(&data, &mut commands);
        assert!(
            commands.is_empty(),
            "a receipt without its Project is insufficient"
        );
        data["projectMemory"]["projects"] =
            json!([{"projectId":"project:foreign","name":"Other","revision":1}]);
        console.sync_conversation_project(&data, &mut commands);
        assert!(commands.is_empty());
        data["projectMemory"]["projects"][0] =
            json!({"projectId":"project:new","name":"Created","revision":1});
        console.sync_conversation_project(&data, &mut commands);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_plan_start");
        assert_eq!(commands[0].payload["projectId"], "project:new");
        assert_eq!(commands[0].payload["expectedProjectRevision"], 1);
        assert_eq!(
            commands[0].payload["instruction"],
            "A first request without setup"
        );
        assert_eq!(
            commands[0].payload["providerId"], "codex",
            "dependent request freezes provider"
        );
        assert_eq!(
            console.create.planner_provider(),
            "",
            "automatic remains selected after resolving the actual provider"
        );
        assert_eq!(console.create.productive_view(), 0);
        commands.clear();
        console.sync_conversation_project(&data, &mut commands);
        assert!(commands.is_empty());
    }

    #[test]
    fn failed_late_or_changed_project_receipts_never_steal_or_resubmit_a_draft() {
        for change in ["failure", "draft", "project", "provider", "epoch"] {
            let mut data = fixture();
            data["hostEpoch"] = json!("boot:one");
            let mut console = Console::launch();
            console.create.sync(&data);
            console.create.draft().goal = "Frozen first message".into();
            let mut commands = Vec::new();
            console.send_conversation(&data, &mut commands);
            let command = commands.remove(0);
            data["projectCreateReceipt"] = json!({"commandId":command.payload["commandId"],"projectId":"project:new","outcome":"applied"});
            data["projectMemory"]["projects"]
                .as_array_mut()
                .unwrap()
                .push(json!({"projectId":"project:new","revision":1,"name":"New"}));
            match change {
                "failure" => data["projectCreateReceipt"]["outcome"] = json!("failed"),
                "draft" => console.create.draft().goal = "Later unsent message".into(),
                "project" => console.create.project = "project:two".into(),
                "provider" => *console.create.planner_provider() = "claude".into(),
                _ => {
                    data["hostEpoch"] = json!("boot:two");
                    console.create.sync(&data);
                }
            }
            console.sync_conversation_project(&data, &mut commands);
            assert!(commands.is_empty(), "{change} must not auto plan");
            assert_ne!(console.create.project, "project:new");
            if change == "draft" {
                assert_eq!(console.create.draft().goal, "Later unsent message");
            }
            if change == "failure" {
                assert_eq!(console.create.draft().goal, "Frozen first message");
            }
            console.sync_conversation_project(&data, &mut commands);
            assert!(commands.is_empty());
        }
    }

    #[test]
    fn an_unavailable_provider_after_creation_keeps_the_message_in_the_created_project() {
        let mut data = fixture();
        data["projectMemory"]["projects"] = json!([]);
        let mut console = Console::launch();
        *console.create.planner_provider() = "codex".into();
        console.create.draft().goal = "Keep my first message".into();
        let mut commands = Vec::new();
        console.send_conversation(&data, &mut commands);
        let sent = commands.remove(0);
        data["projectCreateReceipt"] = json!({"commandId":sent.payload["commandId"],"projectId":"project:new","outcome":"applied"});
        data["projectMemory"]["projects"] =
            json!([{"projectId":"project:new","revision":1,"name":"Created"}]);
        data["providerConnections"]["providers"][0]["authentication"] = json!("missing");
        console.sync_conversation_project(&data, &mut commands);
        assert!(commands.is_empty());
        assert_eq!(console.create.project, "project:new");
        assert_eq!(console.create.draft().goal, "Keep my first message");
        assert!(!console.create.conversation_send_enabled(&data));
        data["providerConnections"]["providers"][0]["authentication"] = json!("present");
        console.send_conversation(&data, &mut commands);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_plan_start");
        assert_eq!(commands[0].payload["projectId"], "project:new");
    }

    #[test]
    fn replies_and_revisions_keep_original_goal_all_answers_and_frozen_file_scope() {
        let mut data = fixture();
        data["work"]["items"][0]["status"] = json!("completed");
        let mut console = console();
        console.create.chat.replies.insert(
            ("project:one".into(), String::new()),
            ReplyDraft {
                parent: "work:one".into(),
                references: HashSet::from(["outputs/source.md".into()]),
            },
        );
        let mut commands = Vec::new();
        console.send_conversation(&data, &mut commands);
        let first = commands.remove(0);
        acknowledge(&mut data, &first, "plan:one", "needs_input");
        console.create.sync(&data);
        *console.create.conversation_input() = "This is for first-time founders".into();
        *console.create.planner_provider() = "claude".into();
        console.send_conversation(&data, &mut commands);
        let second = commands.remove(0);
        assert_eq!(second.action, "project_crew_plan_start");
        assert_eq!(second.payload["providerId"], "claude");
        assert_eq!(second.payload["instruction"], first.payload["instruction"]);
        assert!(context_text(array(&second.payload["answers"]))
            .contains("Mox:\nWho is this for?\nOwner:\nThis is for first-time founders"));
        acknowledge(&mut data, &second, "plan:two", "completed");
        console.create.sync(&data);
        *console.create.conversation_input() = "Use a calmer visual style".into();
        console.send_conversation(&data, &mut commands);
        let third = commands.remove(0);
        assert_eq!(third.payload["instruction"], first.payload["instruction"]);
        let prior = context_text(array(&second.payload["answers"]));
        let current = context_text(array(&third.payload["answers"]));
        assert!(current.starts_with(&prior));
        assert!(current.ends_with("Owner:\nUse a calmer visual style"));
        for command in [first, second, third] {
            assert_eq!(command.payload["replyToWorkItemId"], "work:one");
            assert_eq!(
                command.payload["referencePaths"],
                json!(["outputs/source.md"])
            );
            assert_ne!(command.action, "project_crew_start");
        }
        let key = ("project:one".into(), String::new());
        let turns = &console.create.chat.conversation.sessions[&key].turns;
        assert!(turns
            .iter()
            .any(|turn| turn.speaker == "Mox planner" && turn.body == "Who is this for?"));
        assert!(turns.iter().filter(|turn| turn.speaker == "You").count() == 3);
    }

    #[test]
    fn active_planning_reads_continue_with_an_empty_composer_and_reject_foreign_pages() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        let mut console = console();
        let mut commands = Vec::new();
        console.send_conversation(&data, &mut commands);
        let sent = commands.remove(0);
        acknowledge(&mut data, &sent, "plan:one", "queued");
        console.create.sync(&data);
        assert!(console.create.conversation_input().is_empty());
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        assert!(commands
            .iter()
            .any(|command| command.action == "project_crew_plan_read"));
        assert!(copy.join("\n").contains("Prepare a useful launch deck"));
        data["projectCrewPlanJob"]["projectId"] = json!("project:foreign");
        data["projectCrewPlanJob"]["status"] = json!("needs_input");
        data["projectCrewPlanJob"]["questions"] = json!([{"id":"x","question":"FOREIGN_CANARY"}]);
        data["projectWorkflows"] =
            json!({"plans":[{"projectId":"project:one","planJobId":"plan:one","revision":3}]});
        let (commands, copy) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(1280.0, 880.0),
            false,
            vec![],
        );
        assert!(!copy.join("\n").contains("FOREIGN_CANARY"));
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "project_crew_plan_read");
        assert_eq!(commands[0].payload["projectId"], "project:one");
    }

    #[test]
    fn provider_diagnostics_load_once_without_opening_setup_or_creating_projects() {
        let ctx = egui::Context::default();
        studio::install(&ctx);
        let mut data = fixture();
        data["providerConnections"] = Value::Null;
        data["projectMemory"]["projects"] = json!([]);
        let mut console = Console::launch();
        let (commands, _) = render(
            &ctx,
            &mut console,
            &data,
            egui::vec2(800.0, 560.0),
            false,
            vec![],
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "provider_connections_read");
        for _ in 0..3 {
            assert!(render(
                &ctx,
                &mut console,
                &data,
                egui::vec2(800.0, 560.0),
                false,
                vec![]
            )
            .0
            .is_empty());
        }
        assert!(!console.create.chat.connections_open);
        assert!(!console.create.conversation_creating());
    }

    #[test]
    fn a_chat_plan_is_prepared_then_started_by_separate_visible_clicks_at_minimum_size() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                let mut console = console();
                console.hold_automatic_reads = true;
                let mut commands = Vec::new();
                console.send_conversation(&data, &mut commands);
                let sent = commands.remove(0);
                acknowledge(&mut data, &sent, "plan:chat", "completed");
                data["projectCrewPlanJob"]["plan"] = json!({"abilityId":"P1","members":[
                {"name":"Deck maker","role":"presentation","abilityId":"P1","instruction":"Produce the requested deck."},
                {"name":"Reviewer","role":"review","abilityId":"P1","instruction":"Review the actual deck."}],"settings":{"concurrency":2,"retryLimit":1,"timeoutMinutes":30}});
                for _ in 0..3 {
                    let commands = render(&ctx, &mut console, &data, size, false, vec![]).0;
                    assert!(
                        commands
                            .iter()
                            .all(|command| command.action.ends_with("_read")),
                        "{commands:?}"
                    );
                }
                assert_eq!(console.create.productive_view(), 0);
                let (enabled, button, clip) = ctx.data(|data| {
                    data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                        "test-action-state",
                        "project_crew_prepare",
                        "Prepare crew",
                    )))
                    .unwrap()
                });
                assert!(
                    enabled && screen.contains_rect(button) && clip.contains_rect(button),
                    "{language}/{theme:?} prepare clipped {button:?}/{clip:?}"
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
                assert_eq!(commands[0].action, "project_crew_plan_accept");
                assert_eq!(commands[0].payload["projectId"], "project:one");
                assert_eq!(commands[0].payload["planJobId"], "plan:chat");
                *console.create.conversation_input() = "A later unsent revision".into();
                data["projectWorkflowReceipt"] = json!({"commandId":commands[0].payload["commandId"],"projectId":"project:one","missionId":"mission:chat","outcome":"applied"});
                data["projectCrews"] = json!({"crews":[],"missions":[{"projectId":"project:one","missionId":"mission:chat","revision":1,"status":"prepared","title":"Launch deck","instruction":"Prepare a useful launch deck","tasks":[],"settings":{"concurrency":2,"retryLimit":1,"timeoutMinutes":30}}]});
                for _ in 0..3 {
                    assert!(render(&ctx, &mut console, &data, size, false, vec![])
                        .0
                        .is_empty());
                }
                assert!(
                    !console.create.crew.panel_open,
                    "conversation prepares in place, without a setup overlay"
                );
                let (enabled, button, clip) = ctx.data(|data| {
                    data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                        "test-action-state",
                        "project_crew_start",
                        "Start crew",
                    )))
                    .unwrap()
                });
                assert!(
                    enabled && screen.contains_rect(button) && clip.contains_rect(button),
                    "{language}/{theme:?} start clipped {button:?}/{clip:?}"
                );
                let commands = click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "project_crew_start",
                    "Start crew",
                );
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].action, "project_crew_start");
                assert_eq!(commands[0].payload["missionId"], "mission:chat");
                assert_eq!(commands[0].payload["expectedMissionRevision"], 1);
                assert_eq!(
                    console.create.conversation_input(),
                    "A later unsent revision"
                );
            }
        }
    }

    #[test]
    fn automatic_composer_and_optional_override_are_keyboard_reachable_at_minimum_size() {
        let size = egui::vec2(800.0, 560.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for language in ["en", "tr"] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                studio::install(&ctx);
                ctx.set_theme(theme);
                let mut data = fixture();
                data["desktopPreferences"] = json!({"language":language});
                data["projectMemory"]["projects"] = json!([]);
                let mut console = Console::launch();
                console.create.draft().goal = "A new request without configuration".into();
                for _ in 0..3 {
                    let (commands, copy) = render(&ctx, &mut console, &data, size, false, vec![]);
                    assert!(commands.is_empty());
                    assert!(!copy.join("\n").contains("Prepare task"));
                }
                assert!(console.create.planner_provider().is_empty());
                assert!(
                    ctx.data(|data| data.get_temp::<egui::Rect>(egui::Id::new((
                        "test-action",
                        "memory_control",
                        "Choose provider"
                    ))))
                    .is_none(),
                    "the primary composer must not ask for a provider"
                );
                let (enabled, button, clip) = ctx.data(|data| {
                    data.get_temp::<(bool, egui::Rect, egui::Rect)>(egui::Id::new((
                        "test-action-state",
                        "conversation-send",
                        "Send",
                    )))
                    .unwrap()
                });
                assert!(
                    enabled && screen.contains_rect(button) && clip.contains_rect(button),
                    "{language}/{theme:?} {button:?} {clip:?}"
                );
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
                let (option, id) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Id)>(egui::Id::new((
                        "chat-provider-option",
                        "gemini",
                    )))
                    .unwrap()
                });
                assert!(
                    screen.contains_rect(option),
                    "{language}/{theme:?} manual option clipped {option:?}"
                );
                ctx.memory_mut(|memory| memory.request_focus(id));
                assert!(render(
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
                        modifiers: egui::Modifiers::default()
                    }]
                )
                .0
                .is_empty());
                assert_eq!(console.create.planner_provider(), "gemini");
                assert!(
                    !console.create.conversation_send_enabled(&data),
                    "an unavailable override cannot fall back to ready Codex"
                );
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
                let (option, _) = ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Id)>(egui::Id::new((
                        "chat-provider-option",
                        "",
                    )))
                    .unwrap()
                });
                assert!(
                    screen.contains_rect(option),
                    "{language}/{theme:?} Automatic clipped {option:?}"
                );
                for pressed in [true, false] {
                    assert!(render(
                        &ctx,
                        &mut console,
                        &data,
                        size,
                        false,
                        vec![
                            egui::Event::PointerMoved(option.center()),
                            egui::Event::PointerButton {
                                pos: option.center(),
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::default()
                            }
                        ]
                    )
                    .0
                    .is_empty());
                }
                assert!(console.create.planner_provider().is_empty());
                assert!(console.create.conversation_send_enabled(&data));
                let commands = click_at_size(
                    &ctx,
                    &mut console,
                    &data,
                    size,
                    false,
                    "conversation-send",
                    "Send",
                );
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].action, "project_create");
                assert_eq!(
                    console
                        .create
                        .chat
                        .conversation
                        .creation
                        .as_ref()
                        .unwrap()
                        .provider,
                    "codex"
                );
            }
        }
    }
}
