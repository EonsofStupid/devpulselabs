//! Compact primary native Chat-window composition.
//!
//! This crate paints one independent peer viewport and emits typed intents.
//! It does not create viewports, publish signals, navigate Main, scan stores,
//! launch providers, or implement Social transport.

use dpl_chat_context::RouteContextState;
use dpl_chat_core::{
    ActivationState, Capability, ChatDomain, ChatEvent, ChatEventContent, ChatEventId,
    ChatSessionId, ProviderAccent, ProviderId, RecoveryActionKind, SessionGroup, SessionState,
    SessionSummary, TurnId,
};
use dpl_chat_interactions::{
    InteractionContent, InteractionId, InteractionLifecycle, InteractionPlacement,
    InteractionProvenance, InteractionRegistry, InteractionSpec, OverlayCoordinator, OverlayEntry,
};
use dpl_chat_runtime::{ChatRuntime, ProviderView, RegistryError};
use dpl_chat_social::{
    Presence, SocialConversationId, SocialModel, SocialModelError, SocialThreadKind,
};
use dpl_chat_store::{HistoryCatalog, HistoryIndex};
use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::AppWindowKey;
use dpl_hub::Route;
use egui::{
    Align, Align2, Color32, CornerRadius, Layout, NumExt, Rect, RichText, Sense, Stroke, UiBuilder,
};
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    NodeMetadata, PilotId, PilotKey, UiPilotExt,
};
use std::collections::HashMap;
use std::fmt;

pub const WINDOW_KEY: AppWindowKey = AppWindowKey("chat.primary");
pub const LAUNCHER_ID: PilotId = PilotId::new("shell.window.chat");
pub const AI_MODE_ID: PilotId = PilotId::new("chat.mode.ai");
pub const SOCIAL_MODE_ID: PilotId = PilotId::new("chat.mode.social");
pub const ROUTE_ID: PilotId = PilotId::new("chat.context.route");
pub const SHOW_MAIN_ID: PilotId = PilotId::new("chat.window.show-main");
pub const QUIT_ID: PilotId = PilotId::new("chat.window.quit");
pub const MINIMIZE_ID: PilotId = PilotId::new("chat.window.minimize");
pub const MAXIMIZE_ID: PilotId = PilotId::new("chat.window.maximize");
pub const CLOSE_ID: PilotId = PilotId::new("chat.window.close");
pub const SIDEBAR_ID: PilotId = PilotId::new("chat.sidebar");
pub const PROVIDERS_ID: PilotId = PilotId::new("chat.sidebar.providers");
pub const SESSIONS_ID: PilotId = PilotId::new("chat.sidebar.sessions");
pub const SOCIAL_THREADS_ID: PilotId = PilotId::new("chat.sidebar.social");
pub const CONVERSATION_ID: PilotId = PilotId::new("chat.conversation");
pub const COMPOSER_ID: PilotId = PilotId::new("chat.composer");
pub const SEND_ID: PilotId = PilotId::new("chat.composer.send");
pub const STOP_ID: PilotId = PilotId::new("chat.composer.stop");
pub const NEW_CHAT_ID: PilotId = PilotId::new("chat.provider.new-chat");
pub const ATTACHMENT_ID: PilotId = PilotId::new("chat.composer.attachment");
pub const MEDIA_ID: PilotId = PilotId::new("chat.composer.media");
pub const INTERACTION_COMPONENT_ID: PilotId = PilotId::new("chat.composer.interaction-component");
pub const VOICE_ID: PilotId = PilotId::new("chat.composer.voice");
pub const PROVIDER_MODEL_ID: PilotId = PilotId::new("chat.composer.provider-model");
pub const OVERLAY_ID: PilotId = PilotId::new("chat.overlay");
pub const PROVIDER_OVERLAY_ID: PilotId = PilotId::new("chat.overlay.provider");
pub const PROVIDER_LOGIN_ID: PilotId = PilotId::new("chat.provider.login");
pub const PROVIDER_AUTH_CODE_ID: PilotId = PilotId::new("chat.provider.authorization-code");
pub const PROVIDER_AUTH_BROWSER_ID: PilotId = PilotId::new("chat.provider.authorization-browser");
pub const PROVIDER_SETUP_ID: PilotId = PilotId::new("chat.provider.setup");
pub const PROVIDER_REFRESH_ID: PilotId = PilotId::new("chat.provider.refresh");
pub const UTILITY_OPEN_ID: PilotId = PilotId::new("chat.utility.open");

const FULL_RAIL_WIDTH: f32 = 840.0;
const SIDEBAR_WIDTH: f32 = 252.0;
const PRESENCE_RAIL_WIDTH: f32 = 76.0;
pub fn viewport_key() -> PilotViewportKey {
    PilotViewportKey::new(WINDOW_KEY.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Composition {
    PresenceRail,
    FullRail,
}

impl Composition {
    pub fn for_width(width: f32) -> Self {
        if width >= FULL_RAIL_WIDTH {
            Self::FullRail
        } else {
            Self::PresenceRail
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainOverlay {
    Picker,
    Provider(ProviderId),
    SocialConversation(SocialConversationId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomainViewState {
    pub scroll_anchor: f32,
    pub unread: u32,
    pub overlay: Option<DomainOverlay>,
}

impl Default for DomainViewState {
    fn default() -> Self {
        Self {
            scroll_anchor: 0.0,
            unread: 0,
            overlay: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChatDomainStates {
    pub ai: DomainViewState,
    pub social: DomainViewState,
}

impl ChatDomainStates {
    pub const fn get(&self, domain: ChatDomain) -> &DomainViewState {
        match domain {
            ChatDomain::Ai => &self.ai,
            ChatDomain::Social => &self.social,
        }
    }

    pub const fn get_mut(&mut self, domain: ChatDomain) -> &mut DomainViewState {
        match domain {
            ChatDomain::Ai => &mut self.ai,
            ChatDomain::Social => &mut self.social,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatWindowIntent {
    SelectDomain(ChatDomain),
    SelectProvider(ProviderId),
    SelectSession {
        provider: ProviderId,
        session: ChatSessionId,
    },
    SelectSocialConversation(SocialConversationId),
    UpdateAiDraft {
        provider: ProviderId,
        value: String,
    },
    UpdateSocialDraft {
        conversation: SocialConversationId,
        value: String,
    },
    SetOverlay(Option<DomainOverlay>),
    CompleteInteraction(InteractionId),
    DismissInteraction(InteractionId),
    OpenUtility {
        interaction_id: InteractionId,
        placement: InteractionPlacement,
    },
    ActivateProvider(ProviderId),
    OpenProviderAuthorization(ProviderId),
    OpenProviderSetup(ProviderId),
    RefreshProvider(ProviderId),
    NewSession(ProviderId),
    SendTurn {
        provider: ProviderId,
        session: ChatSessionId,
        text: String,
    },
    CancelTurn {
        provider: ProviderId,
        session: ChatSessionId,
    },
    ResolveProviderPermission {
        interaction: InteractionId,
        provider: ProviderId,
        request_id: u64,
        option_id: Option<String>,
    },
    NavigateTo(Route),
    ShowMain,
    QuitApplication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostIntent {
    OpenUtility {
        interaction_id: InteractionId,
        placement: InteractionPlacement,
    },
    ActivateProvider(ProviderId),
    OpenProviderAuthorization(ProviderId),
    OpenProviderSetup(ProviderId),
    RefreshProvider(ProviderId),
    NewSession(ProviderId),
    SendTurn {
        provider: ProviderId,
        session: ChatSessionId,
        text: String,
    },
    CancelTurn {
        provider: ProviderId,
        session: ChatSessionId,
    },
    ResolveProviderPermission {
        provider: ProviderId,
        request_id: u64,
        option_id: Option<String>,
    },
    NavigateTo(Route),
    ShowMain,
    QuitApplication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatWindowError {
    Provider(RegistryError),
    Social(SocialModelError),
    Interaction(String),
}

impl fmt::Display for ChatWindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(error) => write!(f, "{error}"),
            Self::Social(error) => write!(f, "{error}"),
            Self::Interaction(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for ChatWindowError {}

impl From<RegistryError> for ChatWindowError {
    fn from(value: RegistryError) -> Self {
        Self::Provider(value)
    }
}

impl From<SocialModelError> for ChatWindowError {
    fn from(value: SocialModelError) -> Self {
        Self::Social(value)
    }
}

#[derive(Debug, Clone)]
struct ProviderHistory {
    index: Option<HistoryIndex>,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPermissionChoiceKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPermissionChoice {
    pub option_id: String,
    pub label: String,
    pub kind: ProviderPermissionChoiceKind,
}

#[derive(Debug, Clone)]
struct ProviderPermissionBinding {
    provider: ProviderId,
    request_id: u64,
    allow_once: String,
    allow_always: Option<String>,
    reject: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatWindow {
    runtime: ChatRuntime,
    route_context: RouteContextState,
    histories: HashMap<ProviderId, ProviderHistory>,
    routes: Vec<(String, Route)>,
    social: SocialModel,
    interactions: InteractionRegistry,
    overlay_coordinator: OverlayCoordinator,
    approval_focus_origin: Option<InteractionId>,
    pending_focus_return: Option<InteractionId>,
    provider_permissions: HashMap<InteractionId, ProviderPermissionBinding>,
    domain: ChatDomain,
    domain_states: ChatDomainStates,
}

impl ChatWindow {
    pub fn new(
        runtime: ChatRuntime,
        initial_route: Route,
        catalog: HistoryCatalog,
        routes: Vec<(String, Route)>,
    ) -> Self {
        Self::with_models(
            runtime,
            initial_route,
            catalog,
            routes,
            SocialModel::production(),
            InteractionRegistry::default(),
        )
    }

    pub fn with_models(
        runtime: ChatRuntime,
        initial_route: Route,
        catalog: HistoryCatalog,
        routes: Vec<(String, Route)>,
        social: SocialModel,
        interactions: InteractionRegistry,
    ) -> Self {
        let mut route_context = RouteContextState::default();
        route_context.observe(initial_route);
        let mut histories = HashMap::new();
        for view in runtime.provider_views() {
            let provider = view.registration.manifest.descriptor.id;
            if !catalog.has_source(provider) {
                continue;
            }
            let entry = match catalog.list(provider, None) {
                Ok(index) => ProviderHistory {
                    index: Some(index),
                    error: None,
                },
                Err(error) => ProviderHistory {
                    index: None,
                    error: Some(error.to_string()),
                },
            };
            histories.insert(provider, entry);
        }
        let mut domain_states = ChatDomainStates::default();
        domain_states.social.unread = social.total_unread();
        let approval_focus_origin = interactions.visible().find_map(|spec| {
            matches!(spec.content, InteractionContent::RouteResult { .. }).then_some(spec.id)
        });
        let mut overlay_coordinator = OverlayCoordinator::default();
        for spec in interactions
            .visible()
            .filter(|spec| spec.placement == InteractionPlacement::Overlay)
        {
            overlay_coordinator
                .open(OverlayEntry {
                    interaction_id: spec.id,
                    origin_semantic_id: UTILITY_OPEN_ID.as_str(),
                    priority: if spec.blocks_focus() { 1_000 } else { 100 },
                    blocks_focus: spec.blocks_focus(),
                })
                .unwrap_or_else(|error| {
                    panic!("validated interaction registry produced an invalid overlay: {error}")
                });
        }
        Self {
            runtime,
            route_context,
            histories,
            routes,
            social,
            interactions,
            overlay_coordinator,
            approval_focus_origin,
            pending_focus_return: None,
            provider_permissions: HashMap::new(),
            domain: ChatDomain::Ai,
            domain_states,
        }
    }

    pub const fn domain(&self) -> ChatDomain {
        self.domain
    }

    pub const fn domain_states(&self) -> &ChatDomainStates {
        &self.domain_states
    }

    pub const fn social(&self) -> &SocialModel {
        &self.social
    }

    pub const fn interactions(&self) -> &InteractionRegistry {
        &self.interactions
    }

    pub fn interaction(&self, id: InteractionId) -> Option<&InteractionSpec> {
        self.interactions.get(id)
    }

    pub fn first_utility_interaction(&self) -> Option<(InteractionId, InteractionPlacement)> {
        self.interactions.visible().find_map(|spec| {
            matches!(
                spec.placement,
                InteractionPlacement::SnappedUtility | InteractionPlacement::DetachedUtility
            )
            .then_some((spec.id, spec.placement))
        })
    }

    pub fn session_history(&self, provider: ProviderId) -> Option<&HistoryIndex> {
        self.histories
            .get(&provider)
            .and_then(|history| history.index.as_ref())
    }

    pub fn observe_route(&mut self, route: Route) {
        self.route_context.observe(route);
    }

    pub fn selected_provider_id(&self) -> Option<ProviderId> {
        self.runtime.selected_provider_id()
    }

    pub fn apply(
        &mut self,
        intent: ChatWindowIntent,
    ) -> Result<Option<HostIntent>, ChatWindowError> {
        match intent {
            ChatWindowIntent::SelectDomain(domain) => {
                self.domain = domain;
                Ok(None)
            }
            ChatWindowIntent::SelectProvider(provider) => {
                self.runtime.select_provider(provider)?;
                Ok(None)
            }
            ChatWindowIntent::SelectSession { provider, session } => {
                self.runtime.select_session(provider, session)?;
                Ok(None)
            }
            ChatWindowIntent::SelectSocialConversation(conversation) => {
                self.social.select(&conversation)?;
                Ok(None)
            }
            ChatWindowIntent::UpdateAiDraft { provider, value } => {
                self.runtime.set_draft(provider, value)?;
                Ok(None)
            }
            ChatWindowIntent::UpdateSocialDraft {
                conversation,
                value,
            } => {
                self.social.set_draft(&conversation, value)?;
                Ok(None)
            }
            ChatWindowIntent::SetOverlay(overlay) => {
                self.domain_states.get_mut(self.domain).overlay = overlay;
                Ok(None)
            }
            ChatWindowIntent::CompleteInteraction(id) => {
                self.interactions
                    .transition(id, InteractionLifecycle::Completed)
                    .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
                if self.overlay_coordinator.dismiss(id).is_some() {
                    self.pending_focus_return = self.approval_focus_origin;
                }
                Ok(None)
            }
            ChatWindowIntent::DismissInteraction(id) => {
                self.interactions
                    .transition(id, InteractionLifecycle::Dismissed)
                    .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
                if self.overlay_coordinator.dismiss(id).is_some() {
                    self.pending_focus_return = self.approval_focus_origin;
                }
                Ok(None)
            }
            ChatWindowIntent::OpenUtility {
                interaction_id,
                placement,
            } => Ok(Some(HostIntent::OpenUtility {
                interaction_id,
                placement,
            })),
            ChatWindowIntent::ActivateProvider(provider) => {
                Ok(Some(HostIntent::ActivateProvider(provider)))
            }
            ChatWindowIntent::OpenProviderAuthorization(provider) => {
                Ok(Some(HostIntent::OpenProviderAuthorization(provider)))
            }
            ChatWindowIntent::OpenProviderSetup(provider) => {
                Ok(Some(HostIntent::OpenProviderSetup(provider)))
            }
            ChatWindowIntent::RefreshProvider(provider) => {
                Ok(Some(HostIntent::RefreshProvider(provider)))
            }
            ChatWindowIntent::NewSession(provider) => Ok(Some(HostIntent::NewSession(provider))),
            ChatWindowIntent::SendTurn {
                provider,
                session,
                text,
            } => Ok(Some(HostIntent::SendTurn {
                provider,
                session,
                text,
            })),
            ChatWindowIntent::CancelTurn { provider, session } => {
                Ok(Some(HostIntent::CancelTurn { provider, session }))
            }
            ChatWindowIntent::ResolveProviderPermission {
                interaction,
                provider,
                request_id,
                option_id,
            } => {
                self.interactions
                    .transition(interaction, InteractionLifecycle::Completed)
                    .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
                if self.overlay_coordinator.dismiss(interaction).is_some() {
                    self.pending_focus_return = self.approval_focus_origin;
                }
                self.provider_permissions.remove(&interaction);
                Ok(Some(HostIntent::ResolveProviderPermission {
                    provider,
                    request_id,
                    option_id,
                }))
            }
            ChatWindowIntent::NavigateTo(route) => Ok(Some(HostIntent::NavigateTo(route))),
            ChatWindowIntent::ShowMain => Ok(Some(HostIntent::ShowMain)),
            ChatWindowIntent::QuitApplication => Ok(Some(HostIntent::QuitApplication)),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Vec<ChatWindowIntent> {
        let mut intents = Vec::new();
        let root = ui.max_rect();
        paint_shell_backplate(ui.painter(), root);
        let composition = Composition::for_width(root.width());

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(root.shrink2(egui::vec2(10.0, 8.0)))
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                self.top_strip(ui, &mut intents);
                ui.add_space(6.0);
                let body_height = ui.available_height();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), body_height),
                    Layout::left_to_right(Align::Min),
                    |ui| {
                        let sidebar_width = match composition {
                            Composition::FullRail => SIDEBAR_WIDTH,
                            Composition::PresenceRail => PRESENCE_RAIL_WIDTH,
                        };
                        ui.allocate_ui_with_layout(
                            egui::vec2(sidebar_width, ui.available_height()),
                            Layout::top_down(Align::Min),
                            |ui| self.sidebar(ui, composition, &mut intents),
                        );
                        ui.add_space(8.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), ui.available_height()),
                            Layout::top_down(Align::Min),
                            |ui| self.conversation_and_composer(ui, &mut intents),
                        );
                    },
                );
            },
        );

        self.provider_hover_overlay(ui, &mut intents);
        self.compact_picker_overlay(ui, &mut intents);
        self.interaction_overlays(ui, &mut intents);
        borderless_resize(ui);
        intents
    }

    pub fn set_provider_activation(
        &mut self,
        provider: ProviderId,
        activation: ActivationState,
    ) -> Result<(), ChatWindowError> {
        self.runtime.set_activation(provider, activation)?;
        Ok(())
    }

    pub fn qualify_provider_capabilities(
        &mut self,
        provider: ProviderId,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Result<(), ChatWindowError> {
        self.runtime.qualify_capabilities(provider, capabilities)?;
        Ok(())
    }

    pub fn replace_provider_sessions(
        &mut self,
        provider: ProviderId,
        sessions: Vec<SessionSummary>,
    ) -> Result<(), ChatWindowError> {
        self.runtime.replace_sessions(provider, sessions)?;
        Ok(())
    }

    pub fn upsert_provider_session(
        &mut self,
        provider: ProviderId,
        session: SessionSummary,
    ) -> Result<(), ChatWindowError> {
        self.runtime.upsert_session(provider, session)?;
        Ok(())
    }

    pub fn set_provider_session_state(
        &mut self,
        provider: ProviderId,
        session: ChatSessionId,
        state: SessionState,
    ) -> Result<(), ChatWindowError> {
        self.runtime.set_session_state(provider, session, state)?;
        Ok(())
    }

    pub fn append_provider_event(
        &mut self,
        provider: ProviderId,
        event: ChatEvent,
    ) -> Result<(), ChatWindowError> {
        self.runtime.append_event(provider, event)?;
        Ok(())
    }

    pub fn append_provider_assistant_chunk(
        &mut self,
        provider: ProviderId,
        session: ChatSessionId,
        turn: TurnId,
        event: ChatEventId,
        chunk: &str,
    ) -> Result<(), ChatWindowError> {
        self.runtime
            .append_assistant_chunk(provider, session, turn, event, chunk)?;
        Ok(())
    }

    pub fn select_provider_session(
        &mut self,
        provider: ProviderId,
        session: ChatSessionId,
    ) -> Result<(), ChatWindowError> {
        self.runtime.select_session(provider, session)?;
        Ok(())
    }

    pub fn clear_provider_draft(&mut self, provider: ProviderId) -> Result<(), ChatWindowError> {
        self.runtime.clear_draft(provider)?;
        Ok(())
    }

    pub fn provider_view(&self, provider: ProviderId) -> Option<ProviderView> {
        self.runtime.provider_view(provider)
    }

    pub fn register_provider_permission(
        &mut self,
        interaction: InteractionId,
        provider: ProviderId,
        session: ChatSessionId,
        request_id: u64,
        summary: String,
        choices: Vec<ProviderPermissionChoice>,
    ) -> Result<(), ChatWindowError> {
        let allow_once = choices
            .iter()
            .find(|choice| choice.kind == ProviderPermissionChoiceKind::AllowOnce)
            .or_else(|| {
                choices.iter().find(|choice| {
                    matches!(
                        choice.kind,
                        ProviderPermissionChoiceKind::AllowOnce
                            | ProviderPermissionChoiceKind::AllowAlways
                    )
                })
            })
            .ok_or_else(|| {
                ChatWindowError::Interaction(
                    "provider permission did not include an allow option".into(),
                )
            })?;
        let allow_always = choices
            .iter()
            .find(|choice| choice.kind == ProviderPermissionChoiceKind::AllowAlways);
        let reject = choices.iter().find(|choice| {
            matches!(
                choice.kind,
                ProviderPermissionChoiceKind::RejectOnce
                    | ProviderPermissionChoiceKind::RejectAlways
            )
        });
        let spec = InteractionSpec {
            id: interaction,
            provenance: InteractionProvenance::Provider {
                provider_id: provider,
                session_id: session,
                event_id: None,
            },
            required_capability: Some(Capability::Approvals),
            lifecycle: InteractionLifecycle::Visible,
            placement: InteractionPlacement::Overlay,
            content: InteractionContent::Approval {
                title: "Allow coding-agent action?".into(),
                summary,
                approve_label: allow_once.label.clone(),
                always_approve_label: allow_always.map(|choice| choice.label.clone()),
                reject_label: reject
                    .map_or_else(|| "Deny".to_owned(), |choice| choice.label.clone()),
            },
        };
        self.interactions
            .register(spec)
            .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
        self.overlay_coordinator
            .open(OverlayEntry {
                interaction_id: interaction,
                origin_semantic_id: COMPOSER_ID.as_str(),
                priority: 1_000,
                blocks_focus: true,
            })
            .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
        self.provider_permissions.insert(
            interaction,
            ProviderPermissionBinding {
                provider,
                request_id,
                allow_once: allow_once.option_id.clone(),
                allow_always: allow_always.map(|choice| choice.option_id.clone()),
                reject: reject.map(|choice| choice.option_id.clone()),
            },
        );
        Ok(())
    }

    fn top_strip(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        egui::Frame::new()
            .fill(Color32::from_rgb(6, 19, 23))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(5))
            .inner_margin(egui::Margin {
                left: 12,
                right: 12,
                top: 6,
                bottom: 0,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let compact = ui.available_width() < 820.0;
                    let title = ui
                        .allocate_ui_with_layout(
                            egui::vec2(166.0, 35.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                brand_mark(ui);
                                ui.label(
                                    RichText::new("DevPULSE Chat")
                                        .strong()
                                        .size(17.0)
                                        .color(cl::ink::STRONG),
                                );
                            },
                        )
                        .response
                        .interact(Sense::click_and_drag());
                    if title.drag_started() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if title.double_clicked() {
                        let maximized = ui
                            .ctx()
                            .input(|input| input.viewport().maximized.unwrap_or(false));
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                    }
                    ui.add_space(if compact { 4.0 } else { 20.0 });
                    if mode_button(ui, AI_MODE_ID, "AI", self.domain == ChatDomain::Ai).clicked() {
                        intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Ai));
                    }
                    let social_label = if self.domain_states.social.unread > 0 {
                        format!("Social  {}", self.domain_states.social.unread)
                    } else {
                        "Social".to_owned()
                    };
                    if mode_button(
                        ui,
                        SOCIAL_MODE_ID,
                        &social_label,
                        self.domain == ChatDomain::Social,
                    )
                    .clicked()
                    {
                        intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Social));
                    }
                    if !compact {
                        ui.add_space(10.0);
                        let mut search_value = String::new();
                        let search = ui.add_sized(
                            [ui.available_width().min(260.0), 27.0],
                            egui::TextEdit::singleline(&mut search_value)
                                .hint_text("Search DevPULSE"),
                        );
                        ui.pilot_track(
                            &search,
                            NodeMetadata::new(
                                PilotId::new("chat.search"),
                                Role::TextInput,
                                "Search Chat",
                            ),
                        );
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if quiet_button(ui, CLOSE_ID, "×", "Close Chat").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let maximized = ui
                            .ctx()
                            .input(|input| input.viewport().maximized.unwrap_or(false));
                        if quiet_button(
                            ui,
                            MAXIMIZE_ID,
                            if maximized { "❐" } else { "□" },
                            if maximized {
                                "Restore Chat"
                            } else {
                                "Maximize Chat"
                            },
                        )
                        .clicked()
                        {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if quiet_button(ui, MINIMIZE_ID, "—", "Minimize Chat").clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        if quiet_button(ui, SHOW_MAIN_ID, "↗", "Show Main").clicked() {
                            intents.push(ChatWindowIntent::ShowMain);
                        }
                        ui.label(RichText::new("available").small().color(Tone::Lime.color()));
                        status_dot(ui, Tone::Lime.color());
                        ui.add_space(3.0);
                        status_dot(ui, Tone::Amber.color());
                    });
                });
            });
    }

    fn sidebar(
        &self,
        ui: &mut egui::Ui,
        composition: Composition,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let response = egui::Frame::new()
            .fill(Color32::from_rgb(7, 17, 21))
            .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(8, 9))
            .show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                if composition == Composition::FullRail {
                    sidebar_operator_header(ui);
                    ui.add_space(9.0);
                }
                match (self.domain, composition) {
                    (ChatDomain::Ai, Composition::FullRail) => self.ai_sidebar(ui, intents),
                    (ChatDomain::Social, Composition::FullRail) => self.social_sidebar(ui, intents),
                    (domain, Composition::PresenceRail) => self.presence_rail(ui, domain, intents),
                }
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(SIDEBAR_ID, Role::Region, "Chat navigation"),
        );
    }

    fn ai_sidebar(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let providers = self.runtime.provider_views();
        let available = ui.available_rect_before_wrap();
        let footer_height = 152.0_f32.min((available.height() * 0.40).max(128.0));
        let gap = 7.0;
        let navigation_rect = egui::Rect::from_min_max(
            available.min,
            egui::pos2(available.max.x, available.max.y - footer_height - gap),
        );
        let footer_rect = egui::Rect::from_min_max(
            egui::pos2(available.min.x, navigation_rect.max.y + gap),
            available.max,
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(navigation_rect)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                ui.set_clip_rect(navigation_rect);
                egui::ScrollArea::vertical()
                    .id_salt("chat.ai.sidebar.navigation")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        micro_heading(ui, "AI PROVIDERS");
                        for provider in &providers {
                            self.provider_row(ui, provider, true, intents);
                        }
                        ui.add_space(9.0);
                        self.session_groups(ui, &providers, intents);
                    });
            },
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(footer_rect)
                .layout(Layout::bottom_up(Align::Min)),
            |ui| {
                ui.set_clip_rect(footer_rect);
                sidebar_destination(ui, "⚙  Settings", false);
                sidebar_destination(ui, "⊞  Interaction Library", false);
                sidebar_destination(ui, "♙  People", false);
                sidebar_destination(ui, "♜  Councils · not qualified", true);
                let selected = self.runtime.selected_provider();
                let target = selected.as_ref().map(|provider| {
                    (
                        provider.registration.manifest.descriptor.id,
                        provider.registration.manifest.descriptor.label,
                        provider.registration.activation.is_activated()
                            && provider
                                .registration
                                .has_capability(Capability::StartSession),
                    )
                });
                let can_start = target.is_some_and(|(_, _, can_start)| can_start);
                let new_chat = sidebar_destination_button(ui, "＋  New chat", can_start);
                let mut metadata = NodeMetadata::new(
                    NEW_CHAT_ID.child(&PilotKey::new("sidebar")),
                    Role::Button,
                    "New chat",
                );
                metadata.description = Some(match target {
                    Some((_, label, true)) => format!("Start a new {label} session."),
                    Some((_, label, false)) => {
                        format!("Connect and qualify {label} before starting a session.")
                    }
                    None => "Choose an AI provider before starting a session.".into(),
                });
                ui.pilot_track(&new_chat, metadata);
                if new_chat.clicked() {
                    if let Some((provider, _, true)) = target {
                        intents.push(ChatWindowIntent::NewSession(provider));
                    }
                }
            },
        );
        ui.advance_cursor_after_rect(available);
    }

    fn social_sidebar(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        micro_heading(ui, "PEOPLE");
        for person in self.social.people() {
            let label = format!(
                "{}  {}",
                presence_glyph(person.presence),
                person.display_name
            );
            let selected = self.social.selected().is_some_and(|thread| {
                matches!(
                    &thread.kind,
                    SocialThreadKind::Direct { person_id } if person_id == &person.id
                )
            });
            let response = sidebar_row(ui, &label, selected);
            if response.clicked() {
                if let Some(thread) = self.social.conversations().find(|thread| {
                    matches!(
                        &thread.kind,
                        SocialThreadKind::Direct { person_id } if person_id == &person.id
                    )
                }) {
                    intents.push(ChatWindowIntent::SelectSocialConversation(
                        thread.id.clone(),
                    ));
                }
            }
        }
        ui.add_space(10.0);
        micro_heading(ui, "CONVERSATIONS");
        for thread in self.social.conversations() {
            let selected = self
                .social
                .selected()
                .is_some_and(|current| current.id == thread.id);
            let glyph = match thread.kind {
                SocialThreadKind::Direct { .. } => "●",
                SocialThreadKind::Group => "◆",
                SocialThreadKind::Project { .. } => "#",
            };
            let unread = if thread.unread > 0 {
                format!("  {}", thread.unread)
            } else {
                String::new()
            };
            let response = sidebar_row(ui, &format!("{glyph}  {}{unread}", thread.title), selected);
            let mut metadata = NodeMetadata::new(
                SOCIAL_THREADS_ID.child(&PilotKey::new(thread.id.as_str())),
                Role::Tab,
                thread.title.clone(),
            );
            metadata.selected = Some(selected);
            metadata.value = Some(format!("{:?}", thread.kind));
            ui.pilot_track(&response, metadata);
            if response.clicked() {
                intents.push(ChatWindowIntent::SelectSocialConversation(
                    thread.id.clone(),
                ));
            }
        }
        ui.add_space(10.0);
        micro_heading(ui, "PROJECTS");
        for project in self.social.projects() {
            ui.label(
                RichText::new(format!("#  {}", project.name))
                    .small()
                    .color(cl::ink::MUTED),
            );
        }
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.label(
                RichText::new(self.social.transport().reason())
                    .small()
                    .color(Tone::Amber.color()),
            );
        });
    }

    fn presence_rail(
        &self,
        ui: &mut egui::Ui,
        domain: ChatDomain,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        ui.vertical_centered(|ui| {
            micro_heading(ui, domain.label());
            match domain {
                ChatDomain::Ai => {
                    for provider in self.runtime.provider_views() {
                        self.provider_row(ui, &provider, false, intents);
                    }
                }
                ChatDomain::Social => {
                    for person in self.social.people() {
                        let response = sidebar_row(
                            ui,
                            &format!(
                                "{} {}",
                                presence_glyph(person.presence),
                                initial(&person.display_name)
                            ),
                            false,
                        );
                        if response.clicked() {
                            intents.push(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Picker)));
                        }
                    }
                }
            }
            ui.add_space(8.0);
            let picker = ui.pilot_button(PilotId::new("chat.sidebar.picker"), "•••");
            if picker.clicked() {
                intents.push(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Picker)));
            }
        });
    }

    fn provider_row(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        expanded: bool,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let descriptor = provider.registration.manifest.descriptor;
        let selected = self.runtime.selected_provider_id() == Some(descriptor.id);
        let response = if expanded {
            egui::Frame::new()
                .fill(if selected {
                    Color32::from_rgb(11, 48, 52)
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        Color32::from_rgb(20, 105, 108)
                    } else {
                        Color32::TRANSPARENT
                    },
                ))
                .corner_radius(CornerRadius::same(3))
                .inner_margin(egui::Margin::symmetric(7, 5))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        provider_mark(ui, descriptor.accent);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(
                                RichText::new(descriptor.label)
                                    .small()
                                    .strong()
                                    .color(cl::ink::STRONG),
                            );
                            ui.horizontal(|ui| {
                                status_dot(
                                    ui,
                                    activation_color(
                                        &provider.registration.activation,
                                        provider_tone(descriptor.accent),
                                    ),
                                );
                                ui.label(
                                    RichText::new(provider.registration.activation.label())
                                        .small()
                                        .color(cl::ink::MUTED),
                                );
                            });
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new("›").color(cl::ink::MUTED));
                            if !provider.registration.sessions.is_empty() {
                                ui.label(
                                    RichText::new(provider.registration.sessions.len().to_string())
                                        .monospace()
                                        .small()
                                        .color(Tone::Cyan.color()),
                                );
                            }
                        });
                    });
                })
                .response
                .interact(Sense::click())
        } else {
            sidebar_row(ui, descriptor.short_label, selected)
        };
        let mut metadata = NodeMetadata::new(
            PROVIDERS_ID.child(&PilotKey::new(descriptor.id.0)),
            Role::Tab,
            descriptor.label,
        );
        metadata.selected = Some(selected);
        metadata.value = Some(provider.registration.activation.label().to_owned());
        ui.pilot_track(&response, metadata);
        if response.clicked() {
            intents.push(ChatWindowIntent::SelectProvider(descriptor.id));
            intents.push(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Provider(
                descriptor.id,
            ))));
        } else if response.hovered() {
            intents.push(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Provider(
                descriptor.id,
            ))));
        }
    }

    fn session_groups(
        &self,
        ui: &mut egui::Ui,
        providers: &[ProviderView],
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        if providers
            .iter()
            .all(|provider| provider.registration.sessions.is_empty())
        {
            micro_heading(ui, "SESSIONS");
            let local_count = self
                .histories
                .values()
                .filter_map(|history| history.index.as_ref())
                .map(|index| index.sessions.len())
                .sum::<usize>();
            if let Some(error) = self
                .histories
                .values()
                .find_map(|history| history.error.as_deref())
            {
                ui.label(RichText::new(error).small().color(Tone::Coral.color()));
            } else if local_count > 0 {
                ui.label(
                    RichText::new(format!(
                        "{local_count} local histor{} available",
                        if local_count == 1 { "y" } else { "ies" }
                    ))
                    .small()
                    .color(cl::ink::MUTED),
                );
            } else {
                ui.label(
                    RichText::new("Connect a provider or choose New chat")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            }
        }
        for (heading, predicate) in [
            ("PINNED", SessionPredicate::Pinned),
            ("ACTIVE", SessionPredicate::Active),
            ("RECENT", SessionPredicate::Recent),
        ] {
            let matching = providers
                .iter()
                .flat_map(|provider| {
                    provider
                        .registration
                        .sessions
                        .iter()
                        .filter(move |session| predicate.matches(session))
                        .map(move |session| (provider, session))
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                continue;
            }
            micro_heading(ui, heading);
            for (provider, session) in matching {
                let descriptor = provider.registration.manifest.descriptor;
                let selected = self.runtime.selected_provider_id() == Some(descriptor.id)
                    && provider.active_session == Some(session.id);
                let state = match session.state {
                    SessionState::Running => "working",
                    SessionState::Attention => "needs attention",
                    SessionState::Failed => "failed",
                    SessionState::Expired => "expired",
                    SessionState::ReadOnly => "read only",
                    SessionState::Idle => "ready",
                };
                let label = format!(
                    "{}  {}\n   {} · {}",
                    descriptor.glyph, session.title, descriptor.label, state
                );
                let response = sidebar_row(ui, &label, selected);
                let mut metadata = NodeMetadata::new(
                    SESSIONS_ID
                        .child(&PilotKey::new(descriptor.id.0))
                        .child(&PilotKey::new(session.id.to_string())),
                    Role::Tab,
                    session.title.clone(),
                );
                metadata.selected = Some(selected);
                metadata.value = Some(format!("{} · {state}", descriptor.label));
                ui.pilot_track(&response, metadata);
                if response.clicked() {
                    intents.push(ChatWindowIntent::SelectSession {
                        provider: descriptor.id,
                        session: session.id,
                    });
                }
            }
        }
    }

    fn conversation_and_composer(
        &mut self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let available = ui.available_rect_before_wrap();
        let gap = 8.0;
        let composer_height = 118.0_f32.min((available.height() * 0.26).max(104.0));
        let conversation_rect = egui::Rect::from_min_max(
            available.min,
            egui::pos2(available.max.x, available.max.y - composer_height - gap),
        );
        let composer_rect = egui::Rect::from_min_max(
            egui::pos2(available.min.x, conversation_rect.max.y + gap),
            available.max,
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(conversation_rect)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                ui.set_clip_rect(conversation_rect);
                self.conversation(ui, intents);
            },
        );
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(composer_rect)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                ui.set_clip_rect(composer_rect);
                self.composer(ui, intents);
            },
        );
        ui.advance_cursor_after_rect(available);
    }

    fn conversation(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let response = egui::Frame::new()
            .fill(Color32::from_rgb(5, 12, 15))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ui.set_min_size(egui::vec2(
                    ui.available_width(),
                    (ui.available_height() - 28.0).max(220.0),
                ));
                match self.domain {
                    ChatDomain::Ai => self.ai_conversation(ui, intents),
                    ChatDomain::Social => self.social_conversation(ui),
                }
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(CONVERSATION_ID, Role::Region, "Conversation"),
        );
    }

    fn ai_conversation(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let Some(provider) = self.runtime.selected_provider() else {
            empty_state(
                ui,
                "No AI provider registered",
                "Provider registry is empty.",
            );
            return;
        };
        let descriptor = provider.registration.manifest.descriptor;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(descriptor.label)
                    .strong()
                    .size(18.0)
                    .color(cl::ink::STRONG),
            );
            ui.label(
                RichText::new(provider.registration.activation.label())
                    .small()
                    .color(activation_color(
                        &provider.registration.activation,
                        provider_tone(descriptor.accent),
                    )),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("route-aware")
                        .small()
                        .color(Tone::Cyan.color()),
                );
            });
        });
        if let Some(session) = provider.active_session.and_then(|id| {
            provider
                .registration
                .sessions
                .iter()
                .find(|item| item.id == id)
        }) {
            ui.label(RichText::new(&session.title).small().color(cl::ink::MUTED));
        }
        ui.separator();

        let events = provider.registration.events.iter().filter(|event| {
            provider
                .active_session
                .is_none_or(|session| event.session_id == session)
        });
        let mut rendered = 0_usize;
        egui::ScrollArea::vertical()
            .id_salt(("chat.ai.transcript", descriptor.id, provider.active_session))
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for event in events {
                    rendered += 1;
                    event_row(ui, &event.content);
                }
                self.inline_interactions(ui, intents);
                if rendered == 0 {
                    let detail = provider.registration.activation.reason().unwrap_or(
                        "Choose a session or qualify this provider before starting a turn.",
                    );
                    empty_state(ui, "No messages in this session", detail);
                }
            });
    }

    fn social_conversation(&self, ui: &mut egui::Ui) {
        let Some(thread) = self.social.selected() else {
            empty_state(
                ui,
                "Social is not connected",
                self.social.transport().reason(),
            );
            return;
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&thread.title)
                    .strong()
                    .size(18.0)
                    .color(cl::ink::STRONG),
            );
            ui.label(
                RichText::new(match thread.kind {
                    SocialThreadKind::Direct { .. } => "Direct",
                    SocialThreadKind::Group => "Group",
                    SocialThreadKind::Project { .. } => "Project",
                })
                .small()
                .color(Tone::Cyan.color()),
            );
        });
        ui.label(
            RichText::new(self.social.transport().reason())
                .small()
                .color(Tone::Amber.color()),
        );
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt(("chat.social.transcript", &thread.id))
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if let Ok(messages) = self.social.messages_for(&thread.id) {
                    for message in messages {
                        social_message_row(ui, message);
                    }
                }
            });
    }

    fn inline_interactions(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let interactions = self
            .interactions
            .visible()
            .filter(|spec| {
                matches!(
                    spec.placement,
                    InteractionPlacement::Inline
                        | InteractionPlacement::SnappedUtility
                        | InteractionPlacement::DetachedUtility
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        for spec in interactions {
            let action = match &spec.content {
                InteractionContent::RouteResult { .. } => {
                    Some(("Preview", InteractionPlacement::SnappedUtility))
                }
                InteractionContent::ArtifactPreview { .. } => {
                    Some(("Open preview", spec.placement))
                }
                InteractionContent::Notice { .. } | InteractionContent::Approval { .. } => None,
            };
            let response = match &spec.content {
                InteractionContent::Notice { title, body } => {
                    compact_interaction(ui, Tone::Cyan, title, body, None)
                }
                InteractionContent::RouteResult {
                    route_label,
                    detail,
                    ..
                } => compact_interaction(
                    ui,
                    Tone::Cyan,
                    route_label,
                    detail,
                    action.map(|(label, _)| (label, spec.id)),
                ),
                InteractionContent::ArtifactPreview {
                    title,
                    artifact_kind,
                    summary,
                } => compact_interaction(
                    ui,
                    Tone::Amber,
                    title,
                    &format!("{artifact_kind} · {summary}"),
                    action.map(|(label, _)| (label, spec.id)),
                ),
                InteractionContent::Approval { .. } => None,
            };
            if let (Some(response), Some((label, placement))) = (response, action) {
                if self.pending_focus_return == Some(spec.id) {
                    response.request_focus();
                    self.pending_focus_return = None;
                }
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(
                        UTILITY_OPEN_ID.child(&PilotKey::new(spec.id.to_string())),
                        Role::Button,
                        label,
                    ),
                );
                if response.clicked() {
                    intents.push(ChatWindowIntent::OpenUtility {
                        interaction_id: spec.id,
                        placement,
                    });
                }
            }
        }
    }

    fn composer(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        egui::Frame::new()
            .fill(Color32::from_rgb(8, 18, 22))
            .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let route = self.route_context.offered();
                    let route_hub = route.map_or_else(
                        || "unavailable".to_owned(),
                        |snapshot| snapshot.route.hub.0.to_owned(),
                    );
                    let route_label = route.map_or_else(
                        || "No Main route observed".to_owned(),
                        |snapshot| {
                            format!("⌁  {} / {}", snapshot.route.hub.0, snapshot.route.sub.0)
                        },
                    );
                    let route_response = ui.label(
                        RichText::new(route_label.clone())
                            .small()
                            .color(Tone::Cyan.color()),
                    );
                    let mut route_metadata =
                        NodeMetadata::new(ROUTE_ID, Role::Status, "Observed Main route");
                    route_metadata.value = Some(route_hub);
                    route_metadata.description = Some(route_label);
                    ui.pilot_track(&route_response, route_metadata);
                    let selected = self.runtime.selected_provider();
                    let supports = |capability| {
                        selected.as_ref().is_some_and(|provider| {
                            provider.registration.has_capability(capability)
                        })
                    };
                    composer_capability_button(
                        ui,
                        ATTACHMENT_ID,
                        "＋ Attach",
                        supports(Capability::Attachments),
                        "This provider has not qualified attachment input yet.",
                    );
                    composer_capability_button(
                        ui,
                        MEDIA_ID,
                        "▧ Image / video",
                        supports(Capability::Images) || supports(Capability::Video),
                        "This provider has not qualified image or video input yet.",
                    );
                    composer_capability_button(
                        ui,
                        INTERACTION_COMPONENT_ID,
                        "⊞ Component",
                        false,
                        "The governed Interaction Library picker is not connected in this slice.",
                    );
                    composer_capability_button(
                        ui,
                        VOICE_ID,
                        "◉ Voice",
                        false,
                        "Voice input is visible but is not qualified for this provider.",
                    );
                    if let Some(provider) = selected {
                        let descriptor = provider.registration.manifest.descriptor;
                        let selector_label = format!("{} · provider default", descriptor.label);
                        let selector = ui.small_button(&selector_label);
                        let mut metadata = NodeMetadata::new(
                            PROVIDER_MODEL_ID,
                            Role::Button,
                            "Provider and model",
                        );
                        metadata.value = Some(selector_label);
                        ui.pilot_track(&selector, metadata);
                        if selector.clicked() {
                            intents.push(ChatWindowIntent::SetOverlay(Some(
                                DomainOverlay::Provider(descriptor.id),
                            )));
                        }
                    }
                    ui.label(
                        RichText::new(format!("{} routes", self.routes.len()))
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                });
                ui.horizontal(|ui| {
                    let mut draft = self.current_draft();
                    let editor = ui.add_sized(
                        [(ui.available_width() - 104.0).max(160.0), 52.0],
                        egui::TextEdit::multiline(&mut draft)
                            .hint_text(self.composer_hint())
                            .desired_rows(2),
                    );
                    let mut metadata =
                        NodeMetadata::new(COMPOSER_ID, Role::TextInput, "Message composer");
                    metadata.editable = true;
                    metadata.value = (!draft.is_empty()).then(|| draft.clone());
                    ui.pilot_track(&editor, metadata);
                    if editor.changed() {
                        match self.domain {
                            ChatDomain::Ai => {
                                if let Some(provider) = self.runtime.selected_provider_id() {
                                    intents.push(ChatWindowIntent::UpdateAiDraft {
                                        provider,
                                        value: draft.clone(),
                                    });
                                }
                            }
                            ChatDomain::Social => {
                                if let Some(thread) = self.social.selected() {
                                    intents.push(ChatWindowIntent::UpdateSocialDraft {
                                        conversation: thread.id.clone(),
                                        value: draft.clone(),
                                    });
                                }
                            }
                        }
                    }
                    match self.domain {
                        ChatDomain::Ai => {
                            let selected = self.runtime.selected_provider();
                            let running = selected.as_ref().and_then(|provider| {
                                provider.active_session.and_then(|active| {
                                    provider
                                        .registration
                                        .sessions
                                        .iter()
                                        .find(|session| session.id == active)
                                        .filter(|session| session.state == SessionState::Running)
                                        .map(|session| {
                                            (
                                                provider.registration.manifest.descriptor.id,
                                                session.id,
                                                provider
                                                    .registration
                                                    .has_capability(Capability::InterruptTurn),
                                            )
                                        })
                                })
                            });
                            if let Some((provider, session, can_stop)) = running {
                                let stop = ui.add_enabled(
                                    can_stop,
                                    egui::Button::new("Stop").min_size(egui::vec2(86.0, 42.0)),
                                );
                                let mut metadata =
                                    NodeMetadata::new(STOP_ID, Role::Button, "Stop response");
                                if !can_stop {
                                    metadata.description =
                                        Some("This provider cannot interrupt a turn.".into());
                                }
                                ui.pilot_track(&stop, metadata);
                                if stop.clicked() {
                                    intents
                                        .push(ChatWindowIntent::CancelTurn { provider, session });
                                }
                            } else {
                                let send_target = selected.and_then(|provider| {
                                    provider.active_session.map(|session| {
                                        (
                                            provider.registration.manifest.descriptor.id,
                                            session,
                                            provider.registration.can_send(),
                                        )
                                    })
                                });
                                let enabled = send_target.is_some_and(|(_, _, can_send)| can_send)
                                    && !draft.trim().is_empty();
                                let reason = if draft.trim().is_empty() {
                                    "Type a message first."
                                } else if send_target.is_none() {
                                    "Choose or create a provider session first."
                                } else if send_target.is_some_and(|(_, _, can_send)| !can_send) {
                                    "This provider's live turn transport is not qualified."
                                } else {
                                    "Send this message."
                                };
                                let send = ui.add_enabled(
                                    enabled,
                                    egui::Button::new("Send").min_size(egui::vec2(86.0, 42.0)),
                                );
                                let send = cl::tip(send, reason);
                                let mut metadata =
                                    NodeMetadata::new(SEND_ID, Role::Button, "Send message");
                                metadata.description = Some(reason.to_owned());
                                ui.pilot_track(&send, metadata);
                                if send.clicked() {
                                    if let Some((provider, session, _)) = send_target {
                                        intents.push(ChatWindowIntent::SendTurn {
                                            provider,
                                            session,
                                            text: draft,
                                        });
                                    }
                                }
                            }
                        }
                        ChatDomain::Social => {
                            let reason = self.social.transport().reason();
                            let send = ui.add_enabled(
                                false,
                                egui::Button::new("Send").min_size(egui::vec2(86.0, 42.0)),
                            );
                            let send = cl::tip(send, reason);
                            let mut metadata =
                                NodeMetadata::new(SEND_ID, Role::Button, "Send social message");
                            metadata.description = Some(reason.to_owned());
                            ui.pilot_track(&send, metadata);
                        }
                    }
                });
            });
    }

    fn current_draft(&self) -> String {
        match self.domain {
            ChatDomain::Ai => self
                .runtime
                .selected_provider()
                .map_or_else(String::new, |provider| provider.draft),
            ChatDomain::Social => self
                .social
                .selected()
                .and_then(|thread| self.social.draft(&thread.id).ok())
                .unwrap_or_default()
                .to_owned(),
        }
    }

    fn composer_hint(&self) -> &'static str {
        match self.domain {
            ChatDomain::Ai => "Draft for this AI session…",
            ChatDomain::Social => "Draft for this conversation…",
        }
    }

    fn provider_hover_overlay(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let Some(DomainOverlay::Provider(provider_id)) =
            self.domain_states.get(self.domain).overlay.as_ref()
        else {
            return;
        };
        let Some(provider) = self
            .runtime
            .provider_views()
            .into_iter()
            .find(|provider| provider.registration.manifest.descriptor.id == *provider_id)
        else {
            return;
        };
        let descriptor = provider.registration.manifest.descriptor;
        egui::Area::new(egui::Id::new("chat.provider.hover"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(246.0, 84.0))
            .show(ui.ctx(), |ui| {
                let response = egui::Frame::new()
                    .fill(Color32::from_rgb(9, 28, 31))
                    .stroke(Stroke::new(1.0, Tone::Cyan.color()))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(248.0);
                        ui.label(
                            RichText::new(descriptor.label)
                                .strong()
                                .size(16.0)
                                .color(cl::ink::STRONG),
                        );
                        ui.label(
                            RichText::new(provider.registration.activation.label())
                                .small()
                                .color(activation_color(
                                    &provider.registration.activation,
                                    provider_tone(descriptor.accent),
                                )),
                        );
                        if let Some(reason) = provider.registration.activation.reason() {
                            ui.label(RichText::new(reason).small().color(cl::ink::MUTED));
                        }
                        ui.label(
                            RichText::new(format!(
                                "{} session{}",
                                provider.registration.sessions.len(),
                                if provider.registration.sessions.len() == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            ))
                            .small()
                            .color(cl::ink::MUTED),
                        );
                        ui.label(
                            RichText::new(descriptor.summary)
                                .small()
                                .color(cl::ink::MUTED),
                        );
                        ui.separator();
                        let can_start = provider.registration.activation.is_activated()
                            && provider
                                .registration
                                .has_capability(Capability::StartSession);
                        let new_chat = ui.add_enabled(can_start, egui::Button::new("＋  New chat"));
                        let mut metadata = NodeMetadata::new(
                            NEW_CHAT_ID
                                .child(&PilotKey::new("provider-card"))
                                .child(&PilotKey::new(descriptor.id.0)),
                            Role::Button,
                            format!("New {} chat", descriptor.label),
                        );
                        if !can_start {
                            metadata.description =
                                Some("Connect and qualify this provider first.".into());
                        }
                        ui.pilot_track(&new_chat, metadata);
                        if new_chat.clicked() {
                            intents.push(ChatWindowIntent::NewSession(descriptor.id));
                            intents.push(ChatWindowIntent::SetOverlay(None));
                        }
                        ui.label(
                            RichText::new("♜  Add to Council · not qualified")
                                .color(cl::ink::SUBTLE),
                        );
                        self.provider_activation_action(ui, &provider, intents);
                        ui.label(RichText::new("♙  Account").color(cl::ink::BASE));
                        ui.label(RichText::new("⌁  Diagnostics").color(cl::ink::BASE));
                        if ui.button("Close").clicked() {
                            intents.push(ChatWindowIntent::SetOverlay(None));
                        }
                    })
                    .response;
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(PROVIDER_OVERLAY_ID, Role::Dialog, "Provider hover card"),
                );
            });
    }

    fn provider_activation_action(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let provider_id = provider.registration.manifest.descriptor.id;
        if let ActivationState::AwaitingAuthorization(prompt) = &provider.registration.activation {
            ui.label(
                RichText::new("Authorize Grok in the DevPULSE browser")
                    .color(Tone::Amber.color())
                    .strong(),
            );
            let code = ui.label(
                RichText::new(&prompt.user_code)
                    .monospace()
                    .size(18.0)
                    .color(cl::ink::STRONG),
            );
            ui.pilot_track(
                &code,
                NodeMetadata::new(
                    PROVIDER_AUTH_CODE_ID.child(&PilotKey::new(provider_id.0)),
                    Role::Status,
                    "Grok authorization code",
                )
                .value(prompt.user_code.clone()),
            );
            ui.label(
                RichText::new("The official Grok client is waiting for xAI authorization.")
                    .small()
                    .color(cl::ink::MUTED),
            );
            let reopen = ui.button("Open sign-in window");
            ui.pilot_track(
                &reopen,
                NodeMetadata::new(
                    PROVIDER_AUTH_BROWSER_ID.child(&PilotKey::new(provider_id.0)),
                    Role::Button,
                    "Open Grok sign-in window",
                ),
            );
            if reopen.clicked() {
                intents.push(ChatWindowIntent::OpenProviderAuthorization(provider_id));
            }
            return;
        }
        if matches!(provider.registration.activation, ActivationState::Verifying) {
            let response = ui.add_enabled(false, egui::Button::new("Verifying Grok connection…"));
            ui.pilot_track(
                &response,
                NodeMetadata::new(
                    PROVIDER_LOGIN_ID.child(&PilotKey::new(provider_id.0)),
                    Role::Button,
                    "Verifying Grok connection",
                ),
            );
            return;
        }
        let recovery = provider.registration.activation.recovery().first();
        match recovery.map(|action| action.kind) {
            Some(RecoveryActionKind::SignIn | RecoveryActionKind::Retry) => {
                let label = recovery
                    .map(|action| action.label.as_str())
                    .unwrap_or("Log in");
                let response = ui.button(label);
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(
                        PROVIDER_LOGIN_ID.child(&PilotKey::new(provider_id.0)),
                        Role::Button,
                        label,
                    ),
                );
                if response.clicked() {
                    intents.push(ChatWindowIntent::ActivateProvider(provider_id));
                }
            }
            Some(RecoveryActionKind::InstallInstructions | RecoveryActionKind::UpdateProvider) => {
                let label = recovery
                    .map(|action| action.label.as_str())
                    .unwrap_or("Provider setup");
                let response = ui.button(label);
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(
                        PROVIDER_SETUP_ID.child(&PilotKey::new(provider_id.0)),
                        Role::Button,
                        label,
                    ),
                );
                if response.clicked() {
                    intents.push(ChatWindowIntent::OpenProviderSetup(provider_id));
                }
                let refresh = ui.button("Check installation");
                ui.pilot_track(
                    &refresh,
                    NodeMetadata::new(
                        PROVIDER_REFRESH_ID.child(&PilotKey::new(provider_id.0)),
                        Role::Button,
                        "Check provider installation",
                    ),
                );
                if refresh.clicked() {
                    intents.push(ChatWindowIntent::RefreshProvider(provider_id));
                }
            }
            _ if matches!(provider.registration.activation, ActivationState::SigningIn) => {
                let response = ui.add_enabled(false, egui::Button::new("Connecting to provider…"));
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(
                        PROVIDER_LOGIN_ID.child(&PilotKey::new(provider_id.0)),
                        Role::Button,
                        "Provider login in progress",
                    ),
                );
            }
            _ if matches!(provider.registration.activation, ActivationState::Ready) => {
                ui.label(RichText::new("◎  Account verified").color(Tone::Lime.color()));
            }
            _ => {
                ui.label(RichText::new("◎  Account unavailable").color(cl::ink::SUBTLE));
            }
        }
    }

    fn compact_picker_overlay(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if self.domain_states.get(self.domain).overlay != Some(DomainOverlay::Picker) {
            return;
        }
        egui::Window::new(match self.domain {
            ChatDomain::Ai => "Choose AI provider or session",
            ChatDomain::Social => "Choose person or conversation",
        })
        .id(egui::Id::new("chat.compact.picker"))
        .anchor(Align2::LEFT_TOP, [86.0, 64.0])
        .default_width(280.0)
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            match self.domain {
                ChatDomain::Ai => self.ai_sidebar(ui, intents),
                ChatDomain::Social => self.social_sidebar(ui, intents),
            }
            if ui.button("Close picker").clicked() {
                intents.push(ChatWindowIntent::SetOverlay(None));
            }
        });
    }

    fn interaction_overlays(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        for overlay in self.overlay_coordinator.ordered() {
            let Some(spec) = self.interactions.get(overlay.interaction_id) else {
                continue;
            };
            if !matches!(spec.content, InteractionContent::Approval { .. }) {
                continue;
            }
            let InteractionContent::Approval {
                title,
                summary,
                approve_label,
                always_approve_label,
                reject_label,
            } = &spec.content
            else {
                continue;
            };
            egui::Modal::new(egui::Id::new(spec.id))
                .backdrop_color(Color32::from_black_alpha(176))
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(9, 24, 28))
                        .stroke(Stroke::new(1.0, Tone::Amber.color()))
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(egui::Margin::same(18)),
                )
                .show(ui.ctx(), |ui| {
                    ui.set_width(420.0);
                    let panel = ui
                        .vertical(|ui| {
                            ui.label(
                                RichText::new("APPROVAL REQUIRED")
                                    .monospace()
                                    .small()
                                    .color(Tone::Amber.color()),
                            );
                            ui.label(
                                RichText::new(title)
                                    .strong()
                                    .size(18.0)
                                    .color(cl::ink::STRONG),
                            );
                            ui.label(RichText::new(summary).color(cl::ink::BASE));
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let interaction = PilotKey::new(spec.id.to_string());
                                let approve = ui.button(approve_label);
                                ui.pilot_track(
                                    &approve,
                                    NodeMetadata::new(
                                        OVERLAY_ID
                                            .child(&interaction)
                                            .child(&PilotKey::new("approve")),
                                        Role::Button,
                                        approve_label.clone(),
                                    ),
                                );
                                if approve.clicked() {
                                    if let Some(binding) = self.provider_permissions.get(&spec.id) {
                                        intents.push(ChatWindowIntent::ResolveProviderPermission {
                                            interaction: spec.id,
                                            provider: binding.provider,
                                            request_id: binding.request_id,
                                            option_id: Some(binding.allow_once.clone()),
                                        });
                                    } else {
                                        intents
                                            .push(ChatWindowIntent::CompleteInteraction(spec.id));
                                    }
                                }
                                if let Some(always_label) = always_approve_label {
                                    let always = ui.button(always_label);
                                    ui.pilot_track(
                                        &always,
                                        NodeMetadata::new(
                                            OVERLAY_ID
                                                .child(&interaction)
                                                .child(&PilotKey::new("always")),
                                            Role::Button,
                                            always_label.clone(),
                                        ),
                                    );
                                    if always.clicked() {
                                        if let Some(binding) =
                                            self.provider_permissions.get(&spec.id)
                                        {
                                            intents.push(
                                                ChatWindowIntent::ResolveProviderPermission {
                                                    interaction: spec.id,
                                                    provider: binding.provider,
                                                    request_id: binding.request_id,
                                                    option_id: binding.allow_always.clone(),
                                                },
                                            );
                                        }
                                    }
                                }
                                let reject = ui.button(reject_label);
                                ui.pilot_track(
                                    &reject,
                                    NodeMetadata::new(
                                        OVERLAY_ID
                                            .child(&interaction)
                                            .child(&PilotKey::new("reject")),
                                        Role::Button,
                                        reject_label.clone(),
                                    ),
                                );
                                if reject.clicked() {
                                    if let Some(binding) = self.provider_permissions.get(&spec.id) {
                                        intents.push(ChatWindowIntent::ResolveProviderPermission {
                                            interaction: spec.id,
                                            provider: binding.provider,
                                            request_id: binding.request_id,
                                            option_id: binding.reject.clone(),
                                        });
                                    } else {
                                        intents.push(ChatWindowIntent::DismissInteraction(spec.id));
                                    }
                                }
                            });
                        })
                        .response;
                    let mut metadata = NodeMetadata::new(
                        OVERLAY_ID.child(&PilotKey::new(spec.id.to_string())),
                        Role::Dialog,
                        title.clone(),
                    );
                    metadata.description = Some(summary.clone());
                    ui.pilot_track(&panel, metadata);
                });
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SessionPredicate {
    Pinned,
    Active,
    Recent,
}

impl SessionPredicate {
    fn matches(self, session: &dpl_chat_core::SessionSummary) -> bool {
        matches!(
            (self, session.group),
            (Self::Pinned, SessionGroup::Pinned)
                | (Self::Active, SessionGroup::Active)
                | (Self::Recent, SessionGroup::Recent)
        )
    }
}

fn mode_button(ui: &mut egui::Ui, id: PilotId, label: &str, selected: bool) -> egui::Response {
    const TAB_SIZE: egui::Vec2 = egui::vec2(112.0, 35.0);
    let (rect, response) = ui.allocate_exact_size(TAB_SIZE, Sense::click());
    let fill = if selected {
        Color32::from_rgb(8, 31, 36)
    } else if response.hovered() {
        Color32::from_rgb(9, 25, 30)
    } else {
        Color32::from_rgb(4, 13, 17)
    };
    let stroke = if response.has_focus() {
        Stroke::new(1.5, Tone::Cyan.color())
    } else if selected {
        Stroke::new(1.0, Color32::from_rgb(24, 104, 108))
    } else {
        Stroke::new(1.0, cl::surface::HAIRLINE)
    };
    ui.painter().rect(
        rect,
        CornerRadius {
            nw: 7,
            ne: 7,
            sw: 0,
            se: 0,
        },
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    if selected {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 8.0, rect.top() + 1.5),
                egui::pos2(rect.right() - 8.0, rect.top() + 1.5),
            ],
            Stroke::new(2.5, Tone::Cyan.color()),
        );
    }
    let accent = if id == AI_MODE_ID {
        Tone::Cyan.color()
    } else {
        Tone::Violet.color()
    };
    ui.painter().circle_filled(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        3.5,
        if selected { accent } else { cl::ink::SUBTLE },
    );
    ui.painter().text(
        egui::pos2(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(15.0),
        if selected {
            cl::ink::STRONG
        } else {
            cl::ink::MUTED
        },
    );
    let mut metadata = NodeMetadata::new(id, Role::Tab, label.to_owned());
    metadata.selected = Some(selected);
    ui.pilot_track(&response, metadata);
    response
}

fn brand_mark(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
    ui.painter().rect(
        rect.shrink(2.0),
        CornerRadius::same(2),
        Color32::from_rgb(5, 24, 29),
        Stroke::new(1.0, Tone::Cyan.color()),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .circle_filled(rect.center(), 2.5, Tone::Amber.color());
}

fn borderless_resize(ui: &egui::Ui) {
    let maximized = ui
        .ctx()
        .input(|input| input.viewport().maximized.unwrap_or(false));
    if maximized {
        return;
    }
    let Some(pointer) = ui.ctx().pointer_hover_pos() else {
        return;
    };
    let rect = ui.max_rect();
    let margin = 6.0;
    let west = pointer.x <= rect.left() + margin;
    let east = pointer.x >= rect.right() - margin;
    let north = pointer.y <= rect.top() + margin;
    let south = pointer.y >= rect.bottom() - margin;
    let direction = match (west, east, north, south) {
        (true, _, true, _) => Some(egui::ResizeDirection::NorthWest),
        (_, true, true, _) => Some(egui::ResizeDirection::NorthEast),
        (true, _, _, true) => Some(egui::ResizeDirection::SouthWest),
        (_, true, _, true) => Some(egui::ResizeDirection::SouthEast),
        (true, _, _, _) => Some(egui::ResizeDirection::West),
        (_, true, _, _) => Some(egui::ResizeDirection::East),
        (_, _, true, _) => Some(egui::ResizeDirection::North),
        (_, _, _, true) => Some(egui::ResizeDirection::South),
        _ => None,
    };
    let Some(direction) = direction else {
        return;
    };
    ui.ctx().set_cursor_icon(match direction {
        egui::ResizeDirection::North | egui::ResizeDirection::South => {
            egui::CursorIcon::ResizeVertical
        }
        egui::ResizeDirection::East | egui::ResizeDirection::West => {
            egui::CursorIcon::ResizeHorizontal
        }
        egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
            egui::CursorIcon::ResizeNeSw
        }
        egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
            egui::CursorIcon::ResizeNwSe
        }
    });
    if ui.ctx().input(|input| input.pointer.primary_pressed()) {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
    }
}

fn provider_mark(ui: &mut egui::Ui, accent: ProviderAccent) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
    let tone = provider_tone(accent).color();
    let center = rect.center();
    ui.painter()
        .circle_filled(center, 14.0, Color32::from_rgb(6, 22, 27));
    ui.painter()
        .circle_stroke(center, 14.0, Stroke::new(1.0, teal_hairline()));
    match accent {
        ProviderAccent::Cyan => {
            ui.painter().text(
                center,
                Align2::CENTER_CENTER,
                "</>",
                egui::FontId::monospace(9.0),
                tone,
            );
        }
        ProviderAccent::Amber => {
            for index in 0..8 {
                let angle = index as f32 * std::f32::consts::TAU / 8.0;
                let direction = egui::vec2(angle.cos(), angle.sin());
                ui.painter().line_segment(
                    [center + direction * 4.0, center + direction * 10.0],
                    Stroke::new(2.0, tone),
                );
            }
            ui.painter().circle_filled(center, 3.0, tone);
        }
        ProviderAccent::Violet => {
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    center + egui::vec2(0.0, -10.0),
                    center + egui::vec2(7.0, 0.0),
                    center + egui::vec2(0.0, 10.0),
                    center + egui::vec2(-7.0, 0.0),
                ],
                tone,
                Stroke::NONE,
            ));
        }
        ProviderAccent::Magenta => {
            ui.painter()
                .circle_stroke(center, 9.0, Stroke::new(1.8, tone));
            ui.painter().line_segment(
                [
                    center + egui::vec2(-7.0, 7.0),
                    center + egui::vec2(7.0, -7.0),
                ],
                Stroke::new(2.2, tone),
            );
            ui.painter().circle_filled(center, 2.5, tone);
        }
        ProviderAccent::Lime => {
            ui.painter()
                .circle_stroke(center, 9.0, Stroke::new(2.0, tone));
            ui.painter().circle_filled(center, 3.0, tone);
        }
    }
}

fn sidebar_operator_header(ui: &mut egui::Ui) {
    let response = egui::Frame::new()
        .fill(Color32::from_rgb(8, 22, 26))
        .stroke(Stroke::new(1.0, teal_hairline()))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(9, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (avatar, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), Sense::hover());
                ui.painter()
                    .circle_filled(avatar.center(), 16.0, Color32::from_rgb(12, 38, 43));
                ui.painter().circle_stroke(
                    avatar.center(),
                    16.0,
                    Stroke::new(1.0, Tone::Amber.color()),
                );
                ui.painter().text(
                    avatar.center(),
                    Align2::CENTER_CENTER,
                    "OP",
                    egui::FontId::proportional(12.0),
                    Tone::Amber.color(),
                );
                ui.vertical(|ui| {
                    ui.label(RichText::new("Operator").strong().color(cl::ink::STRONG));
                    ui.horizontal(|ui| {
                        status_dot(ui, Tone::Lime.color());
                        ui.label(RichText::new("Online").small().color(cl::ink::MUTED));
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("●").color(Tone::Amber.color()));
                });
            });
        })
        .response;
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            PilotId::new("chat.operator.presence"),
            Role::Status,
            "Operator online",
        ),
    );
}

fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 18.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

fn quiet_button(ui: &mut egui::Ui, id: PilotId, label: &str, tooltip: &str) -> egui::Response {
    let response = cl::tip(ui.small_button(label), tooltip);
    ui.pilot_track(
        &response,
        NodeMetadata::new(id, Role::Button, tooltip.to_owned()),
    );
    response
}

fn sidebar_row(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    egui::Frame::new()
        .fill(if selected {
            Color32::from_rgb(11, 48, 52)
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0,
            if selected {
                Color32::from_rgb(20, 105, 108)
            } else {
                Color32::TRANSPARENT
            },
        ))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).small().color(if selected {
                cl::ink::STRONG
            } else {
                cl::ink::BASE
            }));
        })
        .response
        .interact(Sense::click())
}

fn sidebar_destination(ui: &mut egui::Ui, label: &str, muted: bool) {
    ui.add_space(3.0);
    ui.label(RichText::new(label).small().color(if muted {
        cl::ink::SUBTLE
    } else {
        cl::ink::MUTED
    }));
}

fn sidebar_destination_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    ui.add_space(3.0);
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).small())
            .frame(false)
            .min_size(egui::vec2(ui.available_width(), 23.0)),
    );
    if !enabled {
        response.on_disabled_hover_text("Connect and qualify the selected provider first.")
    } else {
        response
    }
}

fn composer_capability_button(
    ui: &mut egui::Ui,
    id: PilotId,
    label: &str,
    enabled: bool,
    unavailable_reason: &str,
) -> egui::Response {
    let response = ui.add_enabled(enabled, egui::Button::new(RichText::new(label).small()));
    let response = if enabled {
        response
    } else {
        response.on_disabled_hover_text(unavailable_reason)
    };
    let mut metadata = NodeMetadata::new(id, Role::Button, label);
    metadata.description = Some(if enabled {
        format!("{label} is available for the selected provider.")
    } else {
        unavailable_reason.to_owned()
    });
    ui.pilot_track(&response, metadata);
    response
}

fn micro_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .small()
            .color(cl::ink::SUBTLE),
    );
    ui.add_space(3.0);
}

fn event_row(ui: &mut egui::Ui, content: &ChatEventContent) {
    match content {
        ChatEventContent::UserMessage(body) => {
            conversation_message(ui, "OP", "Operator", body, Tone::Cyan);
            return;
        }
        ChatEventContent::AssistantMessage(body) => {
            conversation_message(ui, "AI", "Assistant", body, Tone::Amber);
            return;
        }
        _ => {}
    }
    let (role, body, tone) = match content {
        ChatEventContent::UserMessage(body) => ("YOU", body.as_str(), Tone::Cyan),
        ChatEventContent::AssistantMessage(body) => ("ASSISTANT", body.as_str(), Tone::Amber),
        ChatEventContent::StatusUpdated(body) => ("STATUS", body.as_str(), Tone::Violet),
        ChatEventContent::PlanUpdated(body) => ("PLAN", body.as_str(), Tone::Violet),
        ChatEventContent::ToolStarted { label } => ("TOOL", label.as_str(), Tone::Cyan),
        ChatEventContent::ToolCompleted { label, succeeded } => (
            if *succeeded {
                "TOOL COMPLETE"
            } else {
                "TOOL FAILED"
            },
            label.as_str(),
            if *succeeded { Tone::Lime } else { Tone::Coral },
        ),
        ChatEventContent::ApprovalRequested { summary } => {
            ("APPROVAL", summary.as_str(), Tone::Amber)
        }
        ChatEventContent::ProviderWarning(body) => ("WARNING", body.as_str(), Tone::Coral),
        ChatEventContent::TurnFailed { message } => ("FAILED", message.as_str(), Tone::Coral),
        ChatEventContent::SessionStarted => ("SYSTEM", "Session started", Tone::Violet),
        ChatEventContent::DiffUpdated(body) => ("DIFF", body.as_str(), Tone::Violet),
        ChatEventContent::ApprovalResolved { accepted } => (
            "APPROVAL",
            if *accepted { "Approved" } else { "Rejected" },
            if *accepted { Tone::Lime } else { Tone::Coral },
        ),
        ChatEventContent::UsageUpdated { summary } => ("USAGE", summary.as_str(), Tone::Violet),
        ChatEventContent::TurnCompleted => ("SYSTEM", "Turn completed", Tone::Lime),
        ChatEventContent::TurnInterrupted => ("SYSTEM", "Turn interrupted", Tone::Amber),
        ChatEventContent::Unknown(payload) => {
            ("UNKNOWN", payload.redacted_summary.as_str(), Tone::Coral)
        }
    };
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(11, 22, 26, 224))
        .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.label(RichText::new(role).monospace().small().color(tone.color()));
            ui.label(RichText::new(body).color(cl::ink::BASE));
        });
    ui.add_space(7.0);
}

fn conversation_message(ui: &mut egui::Ui, initials: &str, role: &str, body: &str, tone: Tone) {
    ui.horizontal_top(|ui| {
        let (avatar, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
        ui.painter()
            .circle_filled(avatar.center(), 13.0, Color32::from_rgb(8, 28, 32));
        ui.painter()
            .circle_stroke(avatar.center(), 13.0, Stroke::new(1.0, tone.color()));
        ui.painter().text(
            avatar.center(),
            Align2::CENTER_CENTER,
            initials,
            egui::FontId::monospace(10.0),
            tone.color(),
        );
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(role).strong().small().color(cl::ink::STRONG));
            ui.label(RichText::new(body).color(cl::ink::BASE));
        });
    });
    ui.add_space(10.0);
}

fn social_message_row(ui: &mut egui::Ui, message: &dpl_chat_social::SocialMessage) {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(11, 23, 26, 224))
        .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(message.author_id.as_str())
                        .strong()
                        .color(Tone::Cyan.color()),
                );
                ui.label(
                    RichText::new(&message.timestamp)
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            });
            ui.label(RichText::new(&message.body).color(cl::ink::BASE));
        });
    ui.add_space(7.0);
}

fn compact_interaction(
    ui: &mut egui::Ui,
    tone: Tone,
    title: &str,
    detail: &str,
    action: Option<(&str, InteractionId)>,
) -> Option<egui::Response> {
    let mut action_response = None;
    egui::Frame::new()
        .fill(Color32::from_rgb(10, 25, 28))
        .stroke(Stroke::new(1.0, tone.color()))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).strong().color(cl::ink::STRONG));
                    ui.label(RichText::new(detail).small().color(cl::ink::MUTED));
                });
                if let Some((label, id)) = action {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let response = stable_interaction_button(ui, label, id);
                        action_response = Some(response);
                    });
                }
            });
        });
    ui.add_space(7.0);
    action_response
}

fn stable_interaction_button(
    ui: &mut egui::Ui,
    label: &str,
    interaction_id: InteractionId,
) -> egui::Response {
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font_id, cl::ink::STRONG);
    let padding = ui.spacing().button_padding;
    let desired_size = (galley.size() + 2.0 * padding).at_least(ui.spacing().interact_size);
    let (_, rect) = ui.allocate_space(desired_size);
    let response = ui.interact(
        rect,
        egui::Id::new(("dpl.chat.interaction.action", interaction_id.0)),
        Sense::click(),
    );
    let visuals = ui.style().interact(&response);
    let painted_rect = rect.expand(visuals.expansion);
    ui.painter().rect(
        painted_rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        if response.has_focus() {
            Stroke::new(1.0, Tone::Cyan.color())
        } else {
            visuals.bg_stroke
        },
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        rect.center() - 0.5 * galley.size(),
        galley,
        visuals.text_color(),
    );
    response
}

fn empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(54.0);
        ui.label(
            RichText::new(title)
                .strong()
                .size(18.0)
                .color(cl::ink::STRONG),
        );
        ui.label(RichText::new(detail).color(cl::ink::MUTED));
    });
}

fn presence_glyph(presence: Presence) -> &'static str {
    match presence {
        Presence::Online => "●",
        Presence::Away => "◐",
        Presence::InFlow => "◆",
        Presence::Offline => "○",
    }
}

fn initial(value: &str) -> &str {
    value.get(..1).unwrap_or("?")
}

fn provider_tone(accent: ProviderAccent) -> Tone {
    match accent {
        ProviderAccent::Amber => Tone::Amber,
        ProviderAccent::Cyan => Tone::Cyan,
        ProviderAccent::Violet => Tone::Violet,
        ProviderAccent::Magenta => Tone::Magenta,
        ProviderAccent::Lime => Tone::Lime,
    }
}

fn activation_color(state: &ActivationState, provider_tone: Tone) -> Color32 {
    match state {
        ActivationState::Ready => Tone::Lime.color(),
        ActivationState::SigningIn
        | ActivationState::AwaitingAuthorization(_)
        | ActivationState::Verifying => Tone::Amber.color(),
        ActivationState::Experimental { .. } => provider_tone.color(),
        ActivationState::Degraded { .. } | ActivationState::Blocked { .. } => Tone::Amber.color(),
        ActivationState::Failed { .. } | ActivationState::VersionUnsupported { .. } => {
            Tone::Coral.color()
        }
        ActivationState::NotInstalled { .. }
        | ActivationState::SignedOut { .. }
        | ActivationState::Disabled { .. } => cl::ink::SUBTLE,
    }
}

fn teal_hairline() -> Color32 {
    Color32::from_rgb(20, 69, 73)
}

fn paint_shell_backplate(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(3, 9, 12));
    let bands = 24;
    for index in 0..bands {
        let t = index as f32 / bands as f32;
        let y0 = egui::lerp(rect.bottom()..=rect.top(), t);
        let y1 = egui::lerp(
            rect.bottom()..=rect.top(),
            (index + 1) as f32 / bands as f32,
        );
        let color = Color32::from_rgba_unmultiplied(
            4,
            (31.0 - 20.0 * t) as u8,
            (35.0 - 22.0 * t) as u8,
            190,
        );
        painter.rect_filled(
            Rect::from_min_max(egui::pos2(rect.left(), y1), egui::pos2(rect.right(), y0)),
            CornerRadius::ZERO,
            color,
        );
    }
    for (x, y, radius, alpha) in [
        (0.08, 0.93, 1.4, 85),
        (0.17, 0.86, 0.9, 68),
        (0.29, 0.96, 1.8, 78),
        (0.43, 0.89, 1.1, 64),
        (0.61, 0.95, 1.5, 76),
        (0.74, 0.87, 0.8, 58),
        (0.88, 0.93, 1.6, 82),
        (0.96, 0.83, 0.7, 52),
    ] {
        painter.circle_filled(
            egui::pos2(
                egui::lerp(rect.left()..=rect.right(), x),
                egui::lerp(rect.top()..=rect.bottom(), y),
            ),
            radius,
            Color32::from_rgba_unmultiplied(245, 126, 38, alpha),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::{
        Capability, ProviderDescriptor, ProviderManifest, ProviderProfileId, SessionSummary,
    };
    use dpl_chat_runtime::ProviderRegistration;
    use dpl_hub::{HubId, SubId};
    use egui_kittest::Harness;

    const PROVIDER: ProviderId = ProviderId("fixture");
    const SECOND_PROVIDER: ProviderId = ProviderId("second");
    const CAPABILITIES: &[Capability] = &[
        Capability::ReadSessions,
        Capability::StartSession,
        Capability::StreamTurns,
        Capability::InterruptTurn,
        Capability::Approvals,
    ];
    const MANIFEST: ProviderManifest = ProviderManifest {
        descriptor: ProviderDescriptor {
            id: PROVIDER,
            label: "Fixture AI",
            short_label: "FX",
            glyph: "F",
            accent: ProviderAccent::Cyan,
            summary: "deterministic fixture",
        },
        profile: ProviderProfileId("fixture"),
        target_capabilities: CAPABILITIES,
        delivery_slice: "test",
        evidence: "fixture",
    };
    const ROUTE: Route = Route {
        hub: HubId("library"),
        sub: SubId("templates"),
    };

    const SECOND_MANIFEST: ProviderManifest = ProviderManifest {
        descriptor: ProviderDescriptor {
            id: SECOND_PROVIDER,
            label: "Second AI",
            short_label: "S2",
            glyph: "S",
            accent: ProviderAccent::Amber,
            summary: "second deterministic fixture",
        },
        profile: ProviderProfileId("second"),
        target_capabilities: CAPABILITIES,
        delivery_slice: "test",
        evidence: "fixture",
    };

    fn window() -> ChatWindow {
        let mut registration = ProviderRegistration::planned(MANIFEST);
        registration.sessions = vec![
            SessionSummary {
                id: ChatSessionId(1),
                provider_id: PROVIDER,
                provider_session_id: None,
                title: "Pinned design review".into(),
                workspace: "DevPULSE".into(),
                group: SessionGroup::Pinned,
                state: SessionState::Idle,
            },
            SessionSummary {
                id: ChatSessionId(2),
                provider_id: PROVIDER,
                provider_session_id: None,
                title: "Active route audit".into(),
                workspace: "DevPULSE".into(),
                group: SessionGroup::Active,
                state: SessionState::Running,
            },
        ];
        ChatWindow::new(
            ChatRuntime::try_from_registrations([registration]).expect("runtime"),
            ROUTE,
            HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        )
    }

    fn multi_provider_window() -> ChatWindow {
        let mut first = ProviderRegistration::planned(MANIFEST);
        first.activation = ActivationState::Ready;
        first.sessions = vec![SessionSummary {
            id: ChatSessionId(1),
            provider_id: PROVIDER,
            provider_session_id: None,
            title: "Fixture session".into(),
            workspace: "DevPULSE".into(),
            group: SessionGroup::Active,
            state: SessionState::Idle,
        }];
        let mut second = ProviderRegistration::planned(SECOND_MANIFEST);
        second.activation = ActivationState::Ready;
        second.sessions = vec![SessionSummary {
            id: ChatSessionId(20),
            provider_id: SECOND_PROVIDER,
            provider_session_id: None,
            title: "Second provider session".into(),
            workspace: "DevPULSE".into(),
            group: SessionGroup::Active,
            state: SessionState::Running,
        }];
        let mut runtime = ChatRuntime::try_from_registrations([first, second]).expect("runtime");
        runtime
            .qualify_capabilities(PROVIDER, CAPABILITIES.iter().copied())
            .expect("first capabilities");
        runtime
            .qualify_capabilities(SECOND_PROVIDER, CAPABILITIES.iter().copied())
            .expect("second capabilities");
        ChatWindow::new(
            runtime,
            ROUTE,
            HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        )
    }

    #[test]
    fn responsive_composition_never_has_a_third_column() {
        assert_eq!(Composition::for_width(719.0), Composition::PresenceRail);
        assert_eq!(Composition::for_width(839.9), Composition::PresenceRail);
        assert_eq!(Composition::for_width(840.0), Composition::FullRail);
        assert_eq!(Composition::for_width(1360.0), Composition::FullRail);
    }

    #[test]
    fn ai_and_social_view_state_are_isolated() {
        let mut window = window();
        window
            .apply(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Provider(
                PROVIDER,
            ))))
            .expect("AI overlay");
        window
            .apply(ChatWindowIntent::SelectDomain(ChatDomain::Social))
            .expect("Social");
        window
            .apply(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Picker)))
            .expect("Social picker");
        assert_eq!(
            window.domain_states().ai.overlay,
            Some(DomainOverlay::Provider(PROVIDER))
        );
        assert_eq!(
            window.domain_states().social.overlay,
            Some(DomainOverlay::Picker)
        );
    }

    #[test]
    fn ai_session_drafts_remain_isolated() {
        let mut window = window();
        window
            .apply(ChatWindowIntent::UpdateAiDraft {
                provider: PROVIDER,
                value: "first".into(),
            })
            .expect("first draft");
        window
            .apply(ChatWindowIntent::SelectSession {
                provider: PROVIDER,
                session: ChatSessionId(2),
            })
            .expect("second session");
        window
            .apply(ChatWindowIntent::UpdateAiDraft {
                provider: PROVIDER,
                value: "second".into(),
            })
            .expect("second draft");
        window
            .apply(ChatWindowIntent::SelectSession {
                provider: PROVIDER,
                session: ChatSessionId(1),
            })
            .expect("first session");
        assert_eq!(
            window.runtime.selected_provider().expect("provider").draft,
            "first"
        );
    }

    #[test]
    fn authored_session_groups_are_disjoint() {
        let window = window();
        let provider = window.runtime.selected_provider().expect("provider");
        for session in &provider.registration.sessions {
            let matches = [
                SessionPredicate::Pinned,
                SessionPredicate::Active,
                SessionPredicate::Recent,
            ]
            .into_iter()
            .filter(|predicate| predicate.matches(session))
            .count();
            assert_eq!(matches, 1, "{} must render exactly once", session.title);
        }
    }

    #[test]
    fn authored_sizes_render_without_layout_panic() {
        for size in [[720.0, 560.0], [1040.0, 640.0], [1360.0, 800.0]] {
            let chat = window();
            let mut harness = Harness::builder().with_size(size).build_ui(move |ui| {
                let mut chat = chat.clone();
                let _ = chat.ui(ui);
            });
            harness.run();
        }
    }

    #[test]
    fn production_social_transport_is_truthfully_unavailable() {
        let window = window();
        assert!(!window.social().transport().can_send());
    }

    #[test]
    fn provider_activation_is_a_typed_host_intent_and_never_enables_send() {
        let mut window = window();
        assert_eq!(
            window
                .apply(ChatWindowIntent::ActivateProvider(PROVIDER))
                .expect("activation intent"),
            Some(HostIntent::ActivateProvider(PROVIDER))
        );
        window
            .set_provider_activation(PROVIDER, ActivationState::Ready)
            .expect("activation state");
        let provider = window.runtime.selected_provider().expect("provider");
        assert!(matches!(
            provider.registration.activation,
            ActivationState::Ready
        ));
        assert!(!provider
            .registration
            .has_capability(Capability::StreamTurns));
    }

    #[test]
    fn shared_session_navigation_retains_provider_identity() {
        let mut window = multi_provider_window();
        window
            .apply(ChatWindowIntent::SelectSession {
                provider: SECOND_PROVIDER,
                session: ChatSessionId(20),
            })
            .expect("select second provider session");
        assert_eq!(window.selected_provider_id(), Some(SECOND_PROVIDER));
        assert_eq!(
            window
                .runtime
                .selected_provider()
                .expect("selected provider")
                .active_session,
            Some(ChatSessionId(20))
        );
    }

    #[test]
    fn qualified_bottom_new_chat_maps_to_the_selected_provider() {
        let mut window = multi_provider_window();
        window
            .apply(ChatWindowIntent::SelectProvider(SECOND_PROVIDER))
            .expect("select second provider");
        assert_eq!(
            window
                .apply(ChatWindowIntent::NewSession(SECOND_PROVIDER))
                .expect("new session"),
            Some(HostIntent::NewSession(SECOND_PROVIDER))
        );
    }

    #[test]
    fn provider_permission_preserves_allow_once_always_and_deny_choices() {
        let mut window = multi_provider_window();
        let interaction = InteractionId(9_001);
        window
            .register_provider_permission(
                interaction,
                SECOND_PROVIDER,
                ChatSessionId(20),
                77,
                "Run the focused test suite".into(),
                vec![
                    ProviderPermissionChoice {
                        option_id: "once".into(),
                        label: "Allow once".into(),
                        kind: ProviderPermissionChoiceKind::AllowOnce,
                    },
                    ProviderPermissionChoice {
                        option_id: "always".into(),
                        label: "Always allow".into(),
                        kind: ProviderPermissionChoiceKind::AllowAlways,
                    },
                    ProviderPermissionChoice {
                        option_id: "deny".into(),
                        label: "Deny".into(),
                        kind: ProviderPermissionChoiceKind::RejectOnce,
                    },
                ],
            )
            .expect("register permission");
        assert_eq!(window.overlay_coordinator.ordered().len(), 1);
        assert_eq!(
            window
                .apply(ChatWindowIntent::ResolveProviderPermission {
                    interaction,
                    provider: SECOND_PROVIDER,
                    request_id: 77,
                    option_id: Some("always".into()),
                })
                .expect("resolve permission"),
            Some(HostIntent::ResolveProviderPermission {
                provider: SECOND_PROVIDER,
                request_id: 77,
                option_id: Some("always".into()),
            })
        );
        assert_eq!(window.overlay_coordinator.ordered().len(), 0);
    }

    #[test]
    fn dismissing_an_approval_queues_its_real_focus_origin() {
        let fixture = dpl_chat_testkit::authored_chat();
        let mut window = ChatWindow::with_models(
            ChatRuntime::try_from_registrations(fixture.providers).expect("runtime"),
            ROUTE,
            HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
            fixture.social,
            fixture.interactions,
        );
        assert_eq!(window.overlay_coordinator.ordered().len(), 1);

        window
            .apply(ChatWindowIntent::DismissInteraction(
                dpl_chat_testkit::AUTHORED_APPROVAL_ID,
            ))
            .expect("dismiss");

        assert_eq!(window.overlay_coordinator.ordered().len(), 0);
        assert_eq!(
            window.pending_focus_return,
            Some(dpl_chat_testkit::AUTHORED_ROUTE_RESULT_ID)
        );
    }

    #[test]
    fn queued_focus_return_targets_the_current_origin_response() {
        let fixture = dpl_chat_testkit::authored_chat();
        let mut window = ChatWindow::with_models(
            ChatRuntime::try_from_registrations(fixture.providers).expect("runtime"),
            ROUTE,
            HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
            fixture.social,
            fixture.interactions,
        );
        window
            .apply(ChatWindowIntent::DismissInteraction(
                dpl_chat_testkit::AUTHORED_APPROVAL_ID,
            ))
            .expect("dismiss");

        let mut harness = Harness::builder()
            .with_size([1040.0, 640.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    let _ = window.ui(ui);
                },
                window,
            );
        harness.run_steps(1);

        let expected = egui::Id::new((
            "dpl.chat.interaction.action",
            dpl_chat_testkit::AUTHORED_ROUTE_RESULT_ID.0,
        ));
        assert!(harness.ctx.memory(|memory| memory.has_focus(expected)));
        assert_eq!(harness.state().pending_focus_return, None);
    }
}
