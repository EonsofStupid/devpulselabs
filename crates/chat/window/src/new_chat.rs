//! Guided session creation. The host owns discovery and session creation.
use super::*;

pub const NEW_CHAT_POPUP_ID: PilotId = PilotId::new("chat.new");
pub const NEW_CHAT_CREATE_ID: PilotId = PilotId::new("chat.new.create");
pub const NEW_CHAT_REFRESH_ID: PilotId = PilotId::new("chat.new.refresh");
pub const NEW_CHAT_ROLE_ID: PilotId = PilotId::new("chat.new.role");
pub const NEW_CHAT_TASK_ID: PilotId = PilotId::new("chat.new.task");
const SETUP_ID: PilotId = PilotId::new("chat.new.setup");
const EXAMPLE_ID: PilotId = PilotId::new("chat.new.example");
const PERMISSIONS_ID: PilotId = PilotId::new("chat.new.permissions");

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoleChoice(String);

#[derive(Debug, Clone, Default)]
pub(super) struct NewChatUi {
    open: bool,
    provider: Option<ProviderId>,
    role: Option<RoleChoice>,
    task: String,
    pending: Option<(ProviderId, String)>,
    message: String,
    checking: bool,
}

impl ChatWindow {
    pub fn open_new_chat(&mut self, provider: Option<ProviderId>) {
        self.new_chat.open = true;
        self.new_chat.provider = provider.or_else(|| {
            let views = self.runtime.provider_views();
            views
                .iter()
                .find(|view| ready(view))
                .or_else(|| views.first())
                .map(|view| view.registration.manifest.descriptor.id)
        });
    }

    pub fn set_machine_checking(&mut self, checking: bool) {
        self.new_chat.checking = checking;
    }

    pub fn can_prepare_chat(&self, provider: ProviderId) -> bool {
        self.new_chat.pending.is_none()
            && !self.project_input_locked
            && self
                .runtime
                .provider_view(provider)
                .is_some_and(|view| ready(&view))
    }

    pub fn chat_creation_pending(&self) -> bool {
        self.new_chat.pending.is_some()
    }

    /// Called only after the host successfully queues NewSession.
    pub fn chat_creation_queued(&mut self, provider: ProviderId, starter: String) {
        self.new_chat.pending = Some((provider, starter));
        self.new_chat.message = "Opening your conversation…".into();
    }

    pub fn chat_creation_failed(&mut self, provider: ProviderId, message: &str) {
        if self.new_chat.provider != Some(provider)
            && !self
                .new_chat
                .pending
                .as_ref()
                .is_some_and(|(id, _)| *id == provider)
        {
            return;
        }
        if self
            .new_chat
            .pending
            .as_ref()
            .is_some_and(|(id, _)| *id == provider)
        {
            self.new_chat.pending = None;
        }
        self.new_chat.message = message.into();
    }

    /// Apply only to an acknowledged NEW session, after selecting that session.
    /// The old session's draft remains in the runtime's per-session draft map.
    pub fn finish_chat_creation(&mut self, provider: ProviderId) -> Result<(), ChatWindowError> {
        if !self
            .new_chat
            .pending
            .as_ref()
            .is_some_and(|(id, _)| *id == provider)
        {
            return Ok(());
        }
        let (_, starter) = self.new_chat.pending.as_ref().expect("checked pending");
        self.runtime.set_draft(provider, starter.clone())?;
        self.new_chat.pending = None;
        self.new_chat.task.clear();
        self.new_chat.message.clear();
        self.new_chat.open = false;
        self.history_focus = None;
        self.history_page = None;
        self.library_open = false;
        self.domain = ChatDomain::Ai;
        self.ai_tab = AiTab::Provider(provider);
        self.request_composer_focus();
        Ok(())
    }

    pub(super) fn new_chat_popup(
        &mut self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        if !self.new_chat.open || self.trust_dialog.is_some() || self.project_ui.open {
            return;
        }
        let fluid = cl::Fluid::of(ui.max_rect());
        let views = self.runtime.provider_views();
        let scroll_height = (ui.max_rect().height() * 0.85 - fluid.tap_min() - 100.0).max(120.0);
        let mut open = true;
        egui::Window::new("New chat")
            .id(egui::Id::new(NEW_CHAT_POPUP_ID.as_str()))
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open).collapsible(false).resizable(true)
            .default_width((fluid.panel_width() * 1.6).min(ui.max_rect().width() * 0.85))
            .default_height(ui.max_rect().height() * 0.82)
            .max_height(ui.max_rect().height() * 0.85)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().max_height(scroll_height).show(ui, |ui| {
                    ui.label(RichText::new("A fresh conversation, set up for you").strong().color(cl::ink::STRONG));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&self.project_view.machine).small().color(cl::ink::MUTED));
                        if let Some(project) = &self.project_view.active {
                            cl::tip(ui.label(&project.label), &project.workspace.display().to_string());
                        } else {
                            ui.label("DevPULSE");
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("1. Choose your AI").strong());
                        let response = ui.add_enabled(!self.new_chat.checking, egui::Button::new("Check again"));
                        ui.pilot_track(&response, NodeMetadata::new(NEW_CHAT_REFRESH_ID, Role::Button, "Check AI availability"));
                        if response.clicked() { intents.push(ChatWindowIntent::RefreshMachine); }
                        if self.new_chat.checking { ui.spinner(); }
                    });
                    ui.add_enabled_ui(self.new_chat.pending.is_none(), |ui| {
                        ui.horizontal_wrapped(|ui| {
                        let selected_provider = self.new_chat.provider;
                        for view in views.iter().filter(|view| view.registration.has_capability(Capability::StartSession) || Some(view.registration.manifest.descriptor.id) == selected_provider) {
                            let descriptor = view.registration.manifest.descriptor;
                            let status = if ready(view) { "Ready to chat" } else if !view.registration.has_capability(Capability::StartSession) {
                                "Chat not connected"
                            } else { "Setup needed" };
                            let response = ui.selectable_label(self.new_chat.provider == Some(descriptor.id), format!("{}  ·  {status}", descriptor.label));
                            ui.pilot_track(&response, NodeMetadata::new(NEW_CHAT_POPUP_ID.child(&PilotKey::new(descriptor.id.0)), Role::Button, format!("Choose {}", descriptor.label)));
                            if response.clicked() { self.new_chat.provider = Some(descriptor.id); }
                        }
                        });
                    });
                    let selected = views.iter().find(|view| Some(view.registration.manifest.descriptor.id) == self.new_chat.provider);
                    if let Some(view) = selected {
                        let provider = view.registration.manifest.descriptor.id;
                        if let Some(observation) = view.registration.on_machine {
                            let status = if observation.running { "Running on this computer" } else if observation.installed { "Installed on this computer" } else { "Not found on this computer" };
                            ui.label(RichText::new(status).small().color(cl::ink::MUTED));
                        }
                        if ready(view) {
                            ui.label(RichText::new(provider_status(&view.registration)).small().color(cl::ink::MUTED));
                            if let Some(mode) = self.permission_modes.get(&provider).copied() {
                                permission_selector(ui, PERMISSIONS_ID, provider, view.registration.manifest.descriptor.label, mode, self.trust.get(&provider).copied().flatten(), intents);
                            }
                        }
                        if !ready(view) {
                            ui.label(view.registration.activation.reason().unwrap_or("This provider's chat connection is not available in this build."));
                            if quiet_button(ui, SETUP_ID, "Open AI setup", "Show this provider's connection options").clicked() {
                                intents.push(ChatWindowIntent::SelectProvider(provider));
                                self.new_chat.open = false;
                            }
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new("2. Give your AI a role").strong());
                    ui.add_enabled_ui(self.new_chat.pending.is_none(), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let response = ui.selectable_label(self.new_chat.role.is_none(), "Everyday assistant");
                            ui.pilot_track(&response, NodeMetadata::new(NEW_CHAT_ROLE_ID.child(&PilotKey::new("none")), Role::Button, "Everyday assistant"));
                            if response.clicked() { self.new_chat.role = None; }
                            for role in &self.room_library.roles {
                                let choice = RoleChoice(role.id.clone());
                                let response = cl::tip(ui.selectable_label(self.new_chat.role.as_ref() == Some(&choice), &role.name), &role.summary);
                                ui.pilot_track(&response, NodeMetadata::new(NEW_CHAT_ROLE_ID.child(&PilotKey::new(&role.id)), Role::Button, format!("Use {} role", role.name)));
                                if response.clicked() { self.new_chat.role = Some(choice); }
                            }
                        });
                    });
                    let role = self.new_chat.role.as_ref().and_then(|choice| self.room_library.role(&choice.0));
                    if let Some(role) = role {
                        ui.label(&role.summary);
                        ui.label(RichText::new("A single AI follows this role. Tools still use your selected permissions.").small().color(cl::ink::MUTED));
                    }
                    ui.separator();
                    ui.label(RichText::new("3. What would you like help with?").strong());
                    ui.add_enabled_ui(self.new_chat.pending.is_none(), |ui| {
                        let response = ui.add(egui::TextEdit::multiline(&mut self.new_chat.task).desired_rows(3).desired_width(f32::INFINITY).hint_text("Describe your task, or try the example below."));
                        ui.pilot_track(&response, NodeMetadata::new(NEW_CHAT_TASK_ID, Role::TextInput, "Your first message"));
                        if self.new_chat.task.is_empty() && quiet_button(ui, EXAMPLE_ID, "Try an example", "Prepare an example task").clicked() {
                            self.new_chat.task = "Help me turn a large unfinished project into one small, useful milestone I can test today.".into();
                        }
                    });
                    let starter = starter_message(role, &self.new_chat.task);
                    ui.collapsing("Preview first message", |ui| { ui.label(&starter); });
                });
                    let role = self.new_chat.role.as_ref().and_then(|choice| self.room_library.role(&choice.0));
                    let starter = starter_message(role, &self.new_chat.task);
                    ui.label(RichText::new("Nothing is sent yet. Review the draft, then press Send.").small().color(cl::ink::MUTED));
                    if !self.new_chat.message.is_empty() { ui.label(&self.new_chat.message); }
                    let enabled = self.new_chat.provider.is_some_and(|provider| self.can_prepare_chat(provider))
                        && !self.project_view.busy;
                    let response = ui.add_enabled(enabled, egui::Button::new("Create chat").min_size(egui::vec2(0.0, fluid.tap_min())));
                    ui.pilot_track(&response, NodeMetadata::new(NEW_CHAT_CREATE_ID, Role::Button, "Create chat"));
                    if response.clicked() {
                        if let Some(provider) = self.new_chat.provider {
                            intents.push(ChatWindowIntent::CreateChat { provider, starter });
                        }
                    }
            });
        self.new_chat.open &= open;
    }
}

fn ready(view: &ProviderView) -> bool {
    view.registration.activation.is_activated()
        && view.registration.has_capability(Capability::StartSession)
}

fn starter_message(role: Option<&dpl_chat_core::rooms::RoleCard>, task: &str) -> String {
    let Some(role) = role else {
        return task.trim().into();
    };
    format!(
        "Role: {}\n{}\n\nMethod:\n- {}\n\nDo not: {}\n\nExpected output: {}\n\nTask:\n{}",
        role.name,
        role.instructions,
        role.method.steps.join("\n- "),
        role.must_not.join("; "),
        role.output_fields().join(", "),
        task.trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{kittest::Queryable, Harness};

    #[test]
    fn new_chat_uses_the_ready_provider_without_a_separate_project_picker() {
        let mut window = super::super::tests::multi_provider_window();
        window.open_new_chat(None);
        assert!(window.project_view.active.is_none());
        let mut harness = Harness::builder()
            .with_size([1100.0, 700.0])
            .build_ui_state(
                |ui, state: &mut (ChatWindow, Vec<HostIntent>)| {
                    for intent in state.0.ui(ui) {
                        if let Ok(Some(host)) = state.0.apply(intent) {
                            state.1.push(host);
                        }
                    }
                },
                (window, Vec::new()),
            );
        harness.run_steps(3);
        // The existing project controls stay intact; the popup owns no project registry.
        assert!(harness.query_by_label("Choose project").is_none());
        harness.get_by_label("Create chat").click();
        harness.run_steps(2);
        assert!(matches!(
            harness.state().1.last(),
            Some(HostIntent::CreateChat { .. })
        ));
    }

    #[test]
    fn unavailable_provider_still_allows_opening_new_chat_and_roles() {
        let mut harness = Harness::builder()
            .with_size([1100.0, 700.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        let _ = window.apply(intent);
                    }
                },
                super::super::tests::window(),
            );
        harness.run_steps(2);
        harness.get_by_label("New chat").click();
        harness.run_steps(3);
        assert!(harness.state().new_chat.open);
        harness.get_by_label("Use Skeptic role").click();
        harness.run_steps(2);
        assert_eq!(
            harness.state().new_chat.role,
            Some(RoleChoice("skeptic".into()))
        );
        harness.get_by_label("Create chat");
        let provider = harness.state().new_chat.provider.unwrap();
        assert!(!harness.state().can_prepare_chat(provider));
        harness.get_by_label("Prepare an example task").click();
        harness.run_steps(2);
        assert!(!harness.state().new_chat.task.is_empty());
    }

    #[test]
    fn role_starter_contains_the_authored_method_and_task() {
        let library = dpl_chat_core::rooms::Library::builtin();
        for role in &library.roles {
            let starter = starter_message(Some(role), "Review my test plan");
            assert!(starter.contains(&role.instructions));
            assert!(starter.contains(&role.method.steps[0]));
            assert!(starter.ends_with("Review my test plan"));
        }
        assert_eq!(starter_message(None, " Hello "), "Hello");
    }

    #[test]
    fn creation_waits_for_ack_and_preserves_old_draft_and_other_provider() {
        let mut window = super::super::tests::multi_provider_window();
        let provider = window.runtime.provider_views()[0]
            .registration
            .manifest
            .descriptor
            .id;
        let other = window.runtime.provider_views()[1]
            .registration
            .manifest
            .descriptor
            .id;
        window
            .runtime
            .select_session(provider, ChatSessionId(1))
            .unwrap();
        window
            .runtime
            .set_draft(provider, "Unsent original".into())
            .unwrap();
        window.chat_creation_queued(provider, "Role: Skeptic\nTask: Review".into());
        assert!(!window.can_prepare_chat(provider));
        window.finish_chat_creation(other).unwrap();
        assert!(window.chat_creation_pending());
        assert_eq!(
            window.runtime.provider_view(provider).unwrap().draft,
            "Unsent original"
        );
        window
            .upsert_provider_session(
                provider,
                SessionSummary {
                    id: ChatSessionId(99),
                    provider_id: provider,
                    provider_session_id: None,
                    title: "New".into(),
                    workspace: "test".into(),
                    group: SessionGroup::Active,
                    state: SessionState::Idle,
                },
            )
            .unwrap();
        window
            .select_provider_session(provider, ChatSessionId(99))
            .unwrap();
        window.finish_chat_creation(provider).unwrap();
        assert_eq!(
            window.runtime.provider_view(provider).unwrap().draft,
            "Role: Skeptic\nTask: Review"
        );
        window
            .select_provider_session(provider, ChatSessionId(1))
            .unwrap();
        assert_eq!(
            window.runtime.provider_view(provider).unwrap().draft,
            "Unsent original"
        );
        assert!(!window.chat_creation_pending());
    }

    #[test]
    fn rejected_creation_keeps_the_task_for_retry() {
        let mut window = super::super::tests::multi_provider_window();
        let provider = window.runtime.provider_views()[0]
            .registration
            .manifest
            .descriptor
            .id;
        window.new_chat.task = "Keep this task".into();
        window.chat_creation_queued(provider, "starter".into());
        window.chat_creation_failed(provider, "Disconnected");
        assert!(!window.chat_creation_pending());
        assert_eq!(window.new_chat.task, "Keep this task");
    }

    #[test]
    fn long_role_draft_can_be_sent_from_a_small_window() {
        let mut window = super::super::tests::multi_provider_window();
        let provider = window.runtime.selected_provider_id().unwrap();
        let starter = starter_message(
            window.room_library.role("skeptic"),
            "Check platform awareness.",
        );
        window.runtime.set_draft(provider, starter.clone()).unwrap();
        let mut harness = Harness::builder()
            .with_size([1040.0, 640.0])
            .build_ui_state(
                |ui, state: &mut (ChatWindow, Vec<HostIntent>)| {
                    for intent in state.0.ui(ui) {
                        if let Ok(Some(host)) = state.0.apply(intent) {
                            state.1.push(host);
                        }
                    }
                },
                (window, Vec::new()),
            );
        harness.run_steps(3);
        harness.get_by_label("Send message").click();
        harness.run_steps(2);
        assert!(harness
            .state()
            .1
            .iter()
            .any(|intent| matches!(intent, HostIntent::SendTurn { text, .. } if text == &starter)));
    }

    #[test]
    fn another_provider_failure_does_not_pollute_the_selected_chat() {
        let mut window = super::super::tests::multi_provider_window();
        let providers = window.runtime.provider_views();
        let selected = providers[0].registration.manifest.descriptor.id;
        let other = providers[1].registration.manifest.descriptor.id;
        window.open_new_chat(Some(selected));
        window.chat_creation_failed(other, "Unrelated failure");
        assert!(window.new_chat.message.is_empty());
        window.chat_creation_queued(selected, "Keep this task".into());
        window.chat_creation_failed(other, "Unrelated failure");
        assert!(window.chat_creation_pending());
        assert!(!window.new_chat.message.contains("Unrelated"));
    }
}
