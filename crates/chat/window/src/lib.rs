//! Compact primary native Chat-window composition.
//!
//! This crate paints one independent peer viewport and emits typed intents.
//! It does not create viewports, publish signals, navigate Main, scan stores,
//! launch providers, or implement Social transport.

use dpl_chat_context::RouteContextState;
use dpl_chat_core::{
    ActivationState, Capability, ChatDomain, ChatEvent, ChatEventContent, ChatEventId,
    ChatSessionId, ContentEpisode, ContentEpisodeId, ContentSpine, ContentView, EventPresentation,
    ProviderAccent, ProviderId, RecoveryActionKind, SessionGroup, SessionState, SessionSummary,
    TurnId,
};
use dpl_chat_interactions::{
    InteractionContent, InteractionId, InteractionLifecycle, InteractionPlacement,
    InteractionProvenance, InteractionRegistry, InteractionSpec, OverlayCoordinator, OverlayEntry,
};
use dpl_chat_profiles::{
    CatalogSupplement, CatalogSupplementAction, ProviderConfigurationKind, ProviderProfile,
    ProviderProfileDraft, SUPPLEMENTS,
};
use dpl_chat_runtime::{ChatRuntime, ProviderView, RegistryError};
use dpl_chat_social::{
    Presence, SocialConversationId, SocialModel, SocialModelError, SocialThreadKind,
};
use dpl_chat_store::{HistoryCatalog, HistoryIndex, HistoryLocator, HistoryPage, IndexedSession};
use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::{AppWindowKey, ProviderEvent, Stamped};
use dpl_hub::Route;
use egui::{
    Align, Align2, Color32, CornerRadius, Layout, NumExt, Rect, RichText, Sense, Stroke, UiBuilder,
};
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    NodeMetadata, PilotId, PilotKey, PilotPath, UiPilotExt,
};
use std::collections::{HashMap, VecDeque};
use std::fmt;

pub const WINDOW_KEY: AppWindowKey = AppWindowKey("chat.primary");
pub const LAUNCHER_ID: PilotId = PilotId::new("shell.window.chat");
pub const AI_MODE_ID: PilotId = PilotId::new("chat.mode.ai");
pub const SOCIAL_MODE_ID: PilotId = PilotId::new("chat.mode.social");
pub const CONTENT_MODE_ID: PilotId = PilotId::new("chat.mode.content");
pub const PROVIDER_TAB_ID: PilotId = PilotId::new("chat.mode.provider");
pub const CONTENT_ID: PilotId = PilotId::new("chat.sidebar.content");
/// Its OWN id, not a child of [`CONTENT_ID`] — census children are keyed by
/// show name, and a show called "refresh" would otherwise collide with it.
pub const CONTENT_REFRESH_ID: PilotId = PilotId::new("chat.sidebar.content.refresh");
pub const CONTENT_BOARD_ID: PilotId = PilotId::new("chat.content.board");
/// The banner naming which past transcript the pane is drawing.
pub const HISTORY_BANNER_ID: PilotId = PilotId::new("chat.history.banner");
/// The RECENT CHATS heading. It exists ONLY when transcripts were found, so
/// its presence is the honest assertion for "the operator's chats surfaced".
pub const HISTORY_SECTION_ID: PilotId = PilotId::new("chat.sidebar.sessions.history");
/// One slot in the title-bar tab strip. Shared by the mode buttons and by the
/// width budget that decides when the search field must yield to them.
const MODE_TAB_WIDTH: f32 = 112.0;
/// The narrowest a tab may become before the strip stops shrinking. Below this
/// the short labels ("FX", "Grok", "Social") stop reading as words.
const MODE_TAB_MIN_WIDTH: f32 = 68.0;
/// The title block's fixed allocation at the left of the strip.
const TITLE_WIDTH: f32 = 166.0;
/// What the right-to-left group at the end of the strip needs: the four window
/// buttons, the availability word, and the status dots.
const TITLE_BAR_RESERVED: f32 = 250.0;
pub const ROUTE_ID: PilotId = PilotId::new("chat.context.route");
pub const SHOW_MAIN_ID: PilotId = PilotId::new("chat.window.show-main");
pub const QUIT_ID: PilotId = PilotId::new("chat.window.quit");
pub const MINIMIZE_ID: PilotId = PilotId::new("chat.window.minimize");
pub const MAXIMIZE_ID: PilotId = PilotId::new("chat.window.maximize");
pub const CLOSE_ID: PilotId = PilotId::new("chat.window.close");
pub const SIDEBAR_ID: PilotId = PilotId::new("chat.sidebar");
pub const PROVIDERS_ID: PilotId = PilotId::new("chat.sidebar.providers");
pub const PROVIDER_CATALOG_ID: PilotId = PilotId::new("chat.provider.catalog");
pub const PROVIDER_CONFIG_ID: PilotId = PilotId::new("chat.provider.configuration");
pub const PROVIDER_CONFIG_NAME_ID: PilotId =
    PilotId::new("chat.provider.configuration.display-name");
pub const PROVIDER_CONFIG_ENDPOINT_ID: PilotId =
    PilotId::new("chat.provider.configuration.endpoint");
pub const PROVIDER_CONFIG_MODEL_ID: PilotId = PilotId::new("chat.provider.configuration.model");
pub const PROVIDER_CONFIG_SAVE_ID: PilotId = PilotId::new("chat.provider.configuration.save");
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
pub const PROVIDER_AUTH_COPY_ID: PilotId = PilotId::new("chat.provider.authorization-copy");
pub const PROVIDER_AUTH_BROWSER_ID: PilotId = PilotId::new("chat.provider.authorization-browser");
pub const PROVIDER_TRACE_ID: PilotId = PilotId::new("chat.provider.activation-trace");
pub const PROVIDER_SETUP_ID: PilotId = PilotId::new("chat.provider.setup");
pub const PROVIDER_REFRESH_ID: PilotId = PilotId::new("chat.provider.refresh");
pub const GENERATION_PANEL_ID: PilotId = PilotId::new("chat.generation.panel");
pub const GENERATION_MODE_IMAGE_ID: PilotId = PilotId::new("chat.generation.mode.image");
pub const GENERATION_MODE_VIDEO_ID: PilotId = PilotId::new("chat.generation.mode.video");
pub const GENERATION_GENERATE_ID: PilotId = PilotId::new("chat.generation.generate");
pub const GENERATION_DRAFT_ID: PilotId = PilotId::new("chat.generation.draft");
pub const UTILITY_OPEN_ID: PilotId = PilotId::new("chat.utility.open");
pub const ASSIST_ID: PilotId = PilotId::new("chat.assist.toggle");

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
    ProviderCatalog,
    ProviderConfiguration(ProviderConfigurationKind),
    Provider(ProviderId),
    SocialConversation(SocialConversationId),
    /// The Imagine generation panel (ADR-011): mode toggle, generate
    /// action, and the governed draft strip.
    Generation,
}

/// What the host has qualified for in-chat generation. Truthful by
/// construction: the window renders exactly this, never an assumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationStatus {
    pub image_available: bool,
    pub unavailable_reason: String,
    pub in_flight: bool,
    pub last_error: Option<String>,
}

impl Default for GenerationStatus {
    fn default() -> Self {
        Self {
            image_available: false,
            unavailable_reason: "Image generation is not provisioned.".to_owned(),
            in_flight: false,
            last_error: None,
        }
    }
}

/// A cached generation draft as the window shows it. `thumb` is a
/// pre-decoded RGBA image (width, height, pixels) prepared by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationDraftView {
    pub id: String,
    pub prompt: String,
    pub created_ms: u64,
    pub thumb: Option<(u32, u32, Vec<u8>)>,
}

/// The operator's answer to the draft-cache flush prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagineFlushDecision {
    /// Flush now; ask again next cycle.
    FlushNow,
    /// Flush now and never ask again.
    FlushAlways,
    /// Keep everything for now; the prompt returns next boot.
    Defer,
}

/// Lazy thumbnail textures. Lives outside the derives because
/// `egui::TextureHandle` has no `Debug`.
#[derive(Clone, Default)]
struct GenerationTextures(HashMap<String, egui::TextureHandle>);

impl std::fmt::Debug for GenerationTextures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GenerationTextures({})", self.0.len())
    }
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
    pub content: DomainViewState,
    pub social: DomainViewState,
}

impl ChatDomainStates {
    pub const fn get(&self, domain: ChatDomain) -> &DomainViewState {
        match domain {
            ChatDomain::Ai => &self.ai,
            ChatDomain::Content => &self.content,
            ChatDomain::Social => &self.social,
        }
    }

    pub const fn get_mut(&mut self, domain: ChatDomain) -> &mut DomainViewState {
        match domain {
            ChatDomain::Ai => &mut self.ai,
            ChatDomain::Content => &mut self.content,
            ChatDomain::Social => &mut self.social,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatWindowIntent {
    SelectDomain(ChatDomain),
    SelectProvider(ProviderId),
    /// Scope the AI domain to one connected provider — the tab beside
    /// AI/Social that a provider earns once it is activated.
    SelectProviderTab(ProviderId),
    /// Inspect one episode. The window records the focus; the host answers
    /// with its spine.
    SelectContentEpisode(ContentEpisodeId),
    /// Re-read the content census from disk.
    RefreshContent,
    SelectSession {
        provider: ProviderId,
        session: ChatSessionId,
    },
    /// Read one transcript already on this machine. The window records the
    /// focus; the host answers with its first page (H1 §3.4).
    SelectHistorySession {
        provider: ProviderId,
        locator: HistoryLocator,
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
    UpdateProviderProfileDraft(ProviderProfileDraft),
    SaveProviderProfile(ProviderProfileDraft),
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
    CopyProviderAuthorizationCode(ProviderId),
    SetProviderTraceExpanded {
        provider: ProviderId,
        expanded: bool,
    },
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
    RequestImageGeneration {
        prompt: String,
    },
    SaveGenerationDraft {
        id: String,
    },
    DiscardGenerationDraft {
        id: String,
    },
    ResolveImagineFlush {
        interaction: InteractionId,
        decision: ImagineFlushDecision,
    },
    /// The operator tapped the assist chip — `enable` is the DESIRED
    /// state. Enabling waits on the one blanket acknowledgement (host
    /// policy); disabling is instant.
    ToggleAssist {
        enable: bool,
    },
    /// The blanket assist acknowledgement resolved — ONE consent for
    /// ALL providers, current and future; never per-provider.
    ResolveAssistAcknowledgement {
        interaction: InteractionId,
        accepted: bool,
    },
    NavigateTo(Route),
    ShowMain,
    QuitApplication,
    /// A slash command recognized at the operator's input-submit event,
    /// from the operator's own text buffer, *before* that text became a
    /// message. `synthetic` is the pilot's attribution of the activating
    /// click — the host refuses to mint a proof when it is true.
    OperatorSlash {
        command: OperatorSlashCommand,
        synthetic: bool,
    },
}

/// Privileged slash commands the Ai composer recognizes at submit.
/// Unrecognized `/…` text stays an ordinary message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorSlashCommand {
    /// Demand strict determinism.
    Strict,
    /// Return to behavioral determinism (operator-demanded strict only).
    Behavioral,
}

impl OperatorSlashCommand {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Behavioral => "behavioral",
        }
    }
}

/// Parse a slash command from the operator's submit buffer.
///
/// Recognition is exact and whole-buffer: the trimmed draft must be exactly
/// `/strict` or `/behavioral`. A `/strict` buried in a sentence is just text
/// — that is the provenance law's other half: pattern-in-string never
/// escalates; only an intentional whole-buffer command at submit does.
pub fn parse_operator_slash(draft: &str) -> Option<OperatorSlashCommand> {
    match draft.trim() {
        "/strict" => Some(OperatorSlashCommand::Strict),
        "/behavioral" => Some(OperatorSlashCommand::Behavioral),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostIntent {
    OpenUtility {
        interaction_id: InteractionId,
        placement: InteractionPlacement,
    },
    /// Read a past transcript's first page from disk. The window cannot do
    /// this itself — a surface does not do IO — so it asks the host, which
    /// owns the catalog (H1 §3.2).
    LoadHistoryPage {
        provider: ProviderId,
        locator: HistoryLocator,
    },
    ActivateProvider(ProviderId),
    OpenProviderAuthorization(ProviderId),
    OpenProviderSetup(ProviderId),
    RefreshProvider(ProviderId),
    ProviderSelected(ProviderId),
    SaveProviderProfile(ProviderProfileDraft),
    CopyProviderAuthorizationCode(ProviderId),
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
    RequestImageGeneration {
        prompt: String,
    },
    SaveGenerationDraft {
        id: String,
    },
    DiscardGenerationDraft {
        id: String,
    },
    ResolveImagineFlush {
        decision: ImagineFlushDecision,
    },
    ToggleAssist {
        enable: bool,
    },
    ResolveAssistAcknowledgement {
        accepted: bool,
    },
    /// Load one episode's chapter→slide structure. Only the requested
    /// episode is read — never the whole library.
    LoadContentSpine(ContentEpisodeId),
    /// Re-read the content census.
    RefreshContent,
    NavigateTo(Route),
    ShowMain,
    QuitApplication,
    /// See [`ChatWindowIntent::OperatorSlash`] — the host is the one mint
    /// site for `OperatorIntent::slash_command`.
    OperatorSlash {
        command: OperatorSlashCommand,
        synthetic: bool,
    },
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
    /// The transcript the operator is reading, if any. Distinct from the live
    /// session focus: a past transcript is read-only.
    history_focus: Option<(ProviderId, HistoryLocator)>,
    /// The page the host last handed back for `history_focus`.
    history_page: Option<HistoryPage>,
    routes: Vec<(String, Route)>,
    social: SocialModel,
    interactions: InteractionRegistry,
    overlay_coordinator: OverlayCoordinator,
    approval_focus_origin: Option<InteractionId>,
    pending_focus_return: Option<InteractionId>,
    provider_permissions: HashMap<InteractionId, ProviderPermissionBinding>,
    provider_trace: VecDeque<Stamped<ProviderEvent>>,
    provider_trace_expanded: HashMap<ProviderId, bool>,
    composer_focus_requested: bool,
    domain: ChatDomain,
    /// The connected provider whose tab is focused. `None` means the AI
    /// tab itself — the roster and every provider's sessions.
    provider_tab: Option<ProviderId>,
    /// The operator's content, as the host last read it. Host-pushed —
    /// the window never reaches for a filesystem.
    content: ContentView,
    /// Which episode the operator is inspecting. Window-owned; the spine
    /// itself arrives separately because loading it is the host's IO.
    content_focus: Option<ContentEpisodeId>,
    domain_states: ChatDomainStates,
    provider_profiles: Vec<ProviderProfile>,
    provider_profile_draft: ProviderProfileDraft,
    generation_status: GenerationStatus,
    generation_drafts: Vec<GenerationDraftView>,
    generation_textures: GenerationTextures,
    imagine_flush_consent: Option<InteractionId>,
    /// The blanket assist acknowledgement's open interaction, if any —
    /// the single-slot binding (the imagine-flush shape).
    assist_acknowledgement: Option<InteractionId>,
    /// Assist mode as the HOST projects it (the window never decides).
    assist_enabled: bool,
}

impl ChatWindow {
    /// The catalog is BORROWED, never owned: reading transcripts is IO, and
    /// a surface does not do IO (D1's law). The window indexes it once here;
    /// the host keeps it and owns every later `page()` call.
    pub fn new(
        runtime: ChatRuntime,
        initial_route: Route,
        catalog: &HistoryCatalog,
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
        catalog: &HistoryCatalog,
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
            history_focus: None,
            history_page: None,
            routes,
            social,
            interactions,
            overlay_coordinator,
            approval_focus_origin,
            pending_focus_return: None,
            provider_permissions: HashMap::new(),
            provider_trace: VecDeque::new(),
            provider_trace_expanded: HashMap::new(),
            composer_focus_requested: false,
            domain: ChatDomain::Ai,
            provider_tab: None,
            content: ContentView::default(),
            content_focus: None,
            domain_states,
            provider_profiles: Vec::new(),
            provider_profile_draft: ProviderProfileDraft::for_kind(
                ProviderConfigurationKind::Local,
            ),
            generation_status: GenerationStatus::default(),
            generation_drafts: Vec::new(),
            generation_textures: GenerationTextures::default(),
            imagine_flush_consent: None,
            assist_acknowledgement: None,
            assist_enabled: false,
        }
    }

    pub fn set_generation_status(&mut self, status: GenerationStatus) {
        self.generation_status = status;
    }

    /// Assist mode as the host's policy truth — projected, never derived.
    pub fn set_assist(&mut self, enabled: bool) {
        self.assist_enabled = enabled;
    }

    pub fn assist_enabled(&self) -> bool {
        self.assist_enabled
    }

    /// Raise the ONE blanket assist acknowledgement: a governed Approval
    /// overlay that consents for ALL providers, current and future —
    /// never per-provider, asked once ever. Two buttons only (no
    /// "always": approving IS the durable answer).
    pub fn register_assist_acknowledgement(
        &mut self,
        interaction: InteractionId,
    ) -> Result<(), ChatWindowError> {
        let spec = InteractionSpec {
            id: interaction,
            provenance: InteractionProvenance::Host,
            required_capability: None,
            lifecycle: InteractionLifecycle::Visible,
            placement: InteractionPlacement::Overlay,
            content: InteractionContent::Approval {
                title: "Assist mode".to_owned(),
                summary: "Chat and the app work as one: chat can see where you are — \
                          your route, your open document, your storyboard's empty slots — \
                          and may offer suggestions as pop-ups you approve or dismiss. \
                          One setting, every provider: this applies across all chat \
                          providers, current and future. Nothing is sent or changed \
                          without your action."
                    .to_owned(),
                approve_label: "Turn on assist — all providers".to_owned(),
                always_approve_label: None,
                reject_label: "Not now".to_owned(),
            },
        };
        self.interactions
            .register(spec)
            .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
        self.overlay_coordinator
            .open(OverlayEntry {
                interaction_id: interaction,
                origin_semantic_id: ASSIST_ID.as_str(),
                priority: 1_000,
                blocks_focus: true,
            })
            .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
        self.assist_acknowledgement = Some(interaction);
        Ok(())
    }

    pub fn replace_generation_drafts(&mut self, drafts: Vec<GenerationDraftView>) {
        self.generation_textures
            .0
            .retain(|id, _| drafts.iter().any(|draft| &draft.id == id));
        self.generation_drafts = drafts;
    }

    /// Raise the flush-day consent prompt (ADR-011): a governed Approval
    /// overlay whose buttons resolve as `HostIntent::ResolveImagineFlush`.
    /// Fails (harmlessly — retry next boot) when another blocking overlay
    /// is already up.
    pub fn register_imagine_flush_prompt(
        &mut self,
        interaction: InteractionId,
        expired: usize,
    ) -> Result<(), ChatWindowError> {
        let spec = InteractionSpec {
            id: interaction,
            provenance: InteractionProvenance::Host,
            required_capability: None,
            lifecycle: InteractionLifecycle::Visible,
            placement: InteractionPlacement::Overlay,
            content: InteractionContent::Approval {
                title: "Clear expired generation drafts?".to_owned(),
                summary: format!(
                    "Today we are dumping {expired} unsaved AI draft(s) older than 30 days. \
                     Anything saved to the library is never touched."
                ),
                approve_label: "Yes, clear them".to_owned(),
                always_approve_label: Some("Yes — never ask again".to_owned()),
                reject_label: "Not today".to_owned(),
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
        self.imagine_flush_consent = Some(interaction);
        Ok(())
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

    /// Receive the content census the host read. Replaces the census and
    /// the error, and KEEPS any loaded spine — a refresh of the list must
    /// not blank the episode the operator is reading.
    /// Receive the transcript page the host read from disk. Late pages for a
    /// row the operator has already navigated away from are DISCARDED — disk
    /// reads race the finger, and the banner must always name what is drawn.
    /// The newest transcript on this machine, if any. The proof harness uses
    /// it to open a REAL conversation rather than an authored stand-in.
    pub fn first_history_session(&self) -> Option<(ProviderId, HistoryLocator)> {
        let providers = self.runtime.provider_views();
        let rows = self.history_rows(&providers);
        rows.first().map(|(provider, session)| {
            (
                provider.registration.manifest.descriptor.id,
                session.locator,
            )
        })
    }

    pub fn observe_history_page(&mut self, page: HistoryPage) {
        if self.history_focus != Some((page.provider_id, page.locator)) {
            return;
        }
        self.history_page = Some(page);
    }

    pub fn observe_content(&mut self, view: ContentView) {
        let spine = self.content.spine.take().or(view.spine);
        self.content = ContentView { spine, ..view };
        // A focus whose episode is gone falls back to the census.
        if let Some(focus) = self.content_focus.clone() {
            if !self
                .content
                .episodes
                .iter()
                .any(|episode| episode.id == focus)
            {
                self.content_focus = None;
                self.content.spine = None;
            }
        }
    }

    /// Receive one episode's spine, loaded by the host on request.
    pub fn observe_content_spine(&mut self, spine: ContentSpine) {
        self.content_focus = Some(spine.id.clone());
        self.content.spine = Some(spine);
    }

    /// Every census address, in rail order. The host uses this to drive a
    /// deterministic proof state without guessing what content exists.
    pub fn content_episode_ids(&self) -> Vec<ContentEpisodeId> {
        self.content
            .episodes
            .iter()
            .map(|episode| episode.id.clone())
            .collect()
    }

    /// The census row the focus names, if it is still present.
    fn content_focus_episode(&self) -> Option<&ContentEpisode> {
        let focus = self.content_focus.as_ref()?;
        self.content
            .episodes
            .iter()
            .find(|episode| &episode.id == focus)
    }

    pub fn selected_provider_id(&self) -> Option<ProviderId> {
        self.runtime.selected_provider_id()
    }

    pub fn replace_provider_profiles(&mut self, profiles: Vec<ProviderProfile>) {
        self.provider_profiles = profiles;
    }

    pub fn upsert_provider_profile(&mut self, profile: ProviderProfile) {
        if let Some(existing) = self
            .provider_profiles
            .iter_mut()
            .find(|existing| existing.key == profile.key)
        {
            *existing = profile;
        } else {
            self.provider_profiles.push(profile);
        }
        self.provider_profiles
            .sort_by(|left, right| left.display_name.cmp(&right.display_name));
        self.domain_states.ai.overlay = Some(DomainOverlay::ProviderCatalog);
    }

    pub fn apply(
        &mut self,
        intent: ChatWindowIntent,
    ) -> Result<Option<HostIntent>, ChatWindowError> {
        match intent {
            ChatWindowIntent::SelectDomain(domain) => {
                self.domain = domain;
                self.provider_tab = None;
                Ok(None)
            }
            ChatWindowIntent::SelectProvider(provider) => {
                self.runtime.select_provider(provider)?;
                Ok(Some(HostIntent::ProviderSelected(provider)))
            }
            ChatWindowIntent::SelectProviderTab(provider) => {
                // The runtime call leads so a rejected provider leaves the
                // tab focus exactly where it was.
                self.runtime.select_provider(provider)?;
                self.domain = ChatDomain::Ai;
                self.provider_tab = Some(provider);
                Ok(Some(HostIntent::ProviderSelected(provider)))
            }
            ChatWindowIntent::SelectContentEpisode(episode) => {
                // Drop the outgoing spine before asking for the new one, so
                // the board never shows one episode's slides under another's
                // name while the host reads.
                self.content.spine = None;
                self.content_focus = Some(episode.clone());
                Ok(Some(HostIntent::LoadContentSpine(episode)))
            }
            ChatWindowIntent::RefreshContent => Ok(Some(HostIntent::RefreshContent)),
            ChatWindowIntent::SelectSession { provider, session } => {
                self.runtime.select_session(provider, session)?;
                // Choosing a LIVE session leaves the transcript reader, or the
                // pane would keep drawing a past conversation over a live one.
                self.history_focus = None;
                self.history_page = None;
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
            ChatWindowIntent::SelectHistorySession { provider, locator } => {
                // Record the focus and drop the stale page in the same beat,
                // so the pane never shows the PREVIOUS transcript under the
                // newly selected row's banner while the host reads disk.
                self.history_focus = Some((provider, locator));
                self.history_page = None;
                self.runtime.select_provider(provider)?;
                Ok(Some(HostIntent::LoadHistoryPage { provider, locator }))
            }
            ChatWindowIntent::UpdateProviderProfileDraft(draft) => {
                self.provider_profile_draft = draft;
                Ok(None)
            }
            ChatWindowIntent::SaveProviderProfile(draft) => {
                Ok(Some(HostIntent::SaveProviderProfile(draft)))
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
            ChatWindowIntent::CopyProviderAuthorizationCode(provider) => {
                Ok(Some(HostIntent::CopyProviderAuthorizationCode(provider)))
            }
            ChatWindowIntent::SetProviderTraceExpanded { provider, expanded } => {
                self.provider_trace_expanded.insert(provider, expanded);
                Ok(None)
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
            ChatWindowIntent::OperatorSlash { command, synthetic } => {
                Ok(Some(HostIntent::OperatorSlash { command, synthetic }))
            }
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
            ChatWindowIntent::RequestImageGeneration { prompt } => {
                Ok(Some(HostIntent::RequestImageGeneration { prompt }))
            }
            ChatWindowIntent::SaveGenerationDraft { id } => {
                Ok(Some(HostIntent::SaveGenerationDraft { id }))
            }
            ChatWindowIntent::DiscardGenerationDraft { id } => {
                Ok(Some(HostIntent::DiscardGenerationDraft { id }))
            }
            ChatWindowIntent::ResolveImagineFlush {
                interaction,
                decision,
            } => {
                self.interactions
                    .transition(interaction, InteractionLifecycle::Completed)
                    .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
                if self.overlay_coordinator.dismiss(interaction).is_some() {
                    self.pending_focus_return = self.approval_focus_origin;
                }
                self.imagine_flush_consent = None;
                Ok(Some(HostIntent::ResolveImagineFlush { decision }))
            }
            // The chip never flips itself: the desired state crosses the
            // typed host boundary and the HOST projects the outcome back
            // (consent may stand between).
            ChatWindowIntent::ToggleAssist { enable } => {
                Ok(Some(HostIntent::ToggleAssist { enable }))
            }
            ChatWindowIntent::ResolveAssistAcknowledgement {
                interaction,
                accepted,
            } => {
                self.interactions
                    .transition(interaction, InteractionLifecycle::Completed)
                    .map_err(|error| ChatWindowError::Interaction(error.to_string()))?;
                if self.overlay_coordinator.dismiss(interaction).is_some() {
                    self.pending_focus_return = self.approval_focus_origin;
                }
                self.assist_acknowledgement = None;
                Ok(Some(HostIntent::ResolveAssistAcknowledgement { accepted }))
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
        self.provider_catalog_overlay(ui, &mut intents);
        self.provider_configuration_overlay(ui, &mut intents);
        self.generation_overlay(ui, &mut intents);
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

    pub fn observe_provider_fact(&mut self, fact: Stamped<ProviderEvent>) {
        const TRACE_CAPACITY: usize = 128;
        if self.provider_trace.len() == TRACE_CAPACITY {
            self.provider_trace.pop_front();
        }
        self.provider_trace.push_back(fact);
    }

    pub fn request_composer_focus(&mut self) {
        self.composer_focus_requested = true;
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

    pub fn append_provider_status_chunk(
        &mut self,
        provider: ProviderId,
        session: ChatSessionId,
        turn: TurnId,
        event: ChatEventId,
        chunk: &str,
    ) -> Result<(), ChatWindowError> {
        self.runtime
            .append_status_chunk(provider, session, turn, event, chunk)?;
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
                    // Every connected provider spends a tab's worth of the row,
                    // so the width the strip needs before the search field may
                    // keep its place grows with them. The +3 is AI, Content and
                    // Social — the three that are ALWAYS in the strip. Counting
                    // only Content under-budgeted by two slots, which stayed
                    // invisible until a third provider activated and pushed
                    // Social underneath the status text.
                    let connected = self.connected_tabs();
                    let tabs = connected.len() + 3;
                    let compact = ui.available_width() < 820.0 + MODE_TAB_WIDTH * tabs as f32;
                    // The strip cannot simply overflow: the window controls and
                    // the status word are laid out right-to-left in this SAME
                    // row, so tabs that do not fit are drawn UNDERNEATH them.
                    // Every slot therefore shares whatever is left once the
                    // title and that right-hand group are reserved, down to a
                    // floor where the short labels still read.
                    let tab_width = ((ui.available_width() - TITLE_WIDTH - TITLE_BAR_RESERVED)
                        / tabs as f32)
                        .clamp(MODE_TAB_MIN_WIDTH, MODE_TAB_WIDTH);
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
                    let focused_tab = self.focused_provider_tab();
                    if mode_button(
                        ui,
                        AI_MODE_ID,
                        "AI",
                        self.domain == ChatDomain::Ai && focused_tab.is_none(),
                        tab_width,
                    )
                    .clicked()
                    {
                        intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Ai));
                    }
                    for (id, label, short_label) in connected {
                        let selected = self.domain == ChatDomain::Ai && focused_tab == Some(id);
                        // A tab is one fixed slot wide, so it carries the SHORT
                        // label always — "ChatGPT / Codex" clips to nonsense at
                        // this size, and a truncated tab is worse than a terse
                        // one. The full name rides in the pilot metadata.
                        let tab = mode_button(
                            ui,
                            PROVIDER_TAB_ID.child(&PilotKey::new(id.0)),
                            short_label,
                            selected,
                            tab_width,
                        )
                        .on_hover_text(label);
                        if tab.clicked() {
                            intents.push(ChatWindowIntent::SelectProviderTab(id));
                        }
                    }
                    if mode_button(
                        ui,
                        CONTENT_MODE_ID,
                        "Content",
                        self.domain == ChatDomain::Content,
                        tab_width,
                    )
                    .clicked()
                    {
                        intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Content));
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
                        tab_width,
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
                    (ChatDomain::Content, Composition::FullRail) => {
                        self.content_sidebar(ui, intents)
                    }
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
                        // A focused provider tab scopes the rail to that one
                        // provider; the AI tab is the roster and the census.
                        match self.focused_provider_tab().and_then(|focus| {
                            providers
                                .iter()
                                .find(|provider| {
                                    provider.registration.manifest.descriptor.id == focus
                                })
                                .cloned()
                        }) {
                            Some(focused) => {
                                self.provider_tab_header(ui, &focused, intents);
                                ui.add_space(9.0);
                                self.session_groups(ui, std::slice::from_ref(&focused), intents);
                            }
                            None => {
                                self.provider_roster(ui, &providers, intents);
                                ui.add_space(9.0);
                                self.session_groups(ui, &providers, intents);
                            }
                        }
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

    /// The content census: the operator's shows first, then the seed
    /// templates, each episode one selectable row. Reading only — this rail
    /// mints no intent that writes.
    fn content_sidebar(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if let Some(error) = self.content.error.as_deref() {
            micro_heading(ui, "CONTENT");
            ui.label(RichText::new(error).small().color(Tone::Coral.color()));
            ui.add_space(6.0);
        }
        let mut shows: Vec<&str> = self
            .content
            .episodes
            .iter()
            .filter_map(|episode| episode.show.as_deref())
            .collect();
        shows.sort_unstable();
        shows.dedup();

        for show in shows {
            micro_heading(ui, &show.to_uppercase());
            for episode in self
                .content
                .episodes
                .iter()
                .filter(|episode| episode.show.as_deref() == Some(show))
            {
                self.content_row(ui, episode, intents);
            }
        }

        let seeds: Vec<&ContentEpisode> = self
            .content
            .episodes
            .iter()
            .filter(|episode| episode.show.is_none())
            .collect();
        if !seeds.is_empty() {
            micro_heading(ui, "SEED TEMPLATES");
            for episode in seeds {
                self.content_row(ui, episode, intents);
            }
        }

        if self.content.episodes.is_empty() && self.content.error.is_none() {
            micro_heading(ui, "CONTENT");
            ui.label(
                RichText::new("No episodes yet — clone a seed to start a show.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
        }

        ui.add_space(2.0);
        let refresh = sidebar_destination_button(ui, "⟳  Refresh", true);
        let mut metadata = NodeMetadata::new(CONTENT_REFRESH_ID, Role::Button, "Refresh content");
        metadata.description = Some("Re-read the content root from disk.".to_owned());
        ui.pilot_track(&refresh, metadata);
        if refresh.clicked() {
            intents.push(ChatWindowIntent::RefreshContent);
        }
    }

    fn content_row(
        &self,
        ui: &mut egui::Ui,
        episode: &ContentEpisode,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let selected = self.content_focus.as_ref() == Some(&episode.id);
        let response = sidebar_row(ui, &episode.title, selected);
        // The address is "<group>/<slug>". Split it into TWO pilot keys —
        // one key holding a slash would fake a hierarchy level that isn't
        // there, and every path consumer would inherit the lie.
        let (group, slug) = split_address(&episode.id);
        let mut metadata = NodeMetadata::new(
            CONTENT_ID
                .child(&PilotKey::new(group))
                .child(&PilotKey::new(slug)),
            Role::Tab,
            episode.title.clone(),
        );
        metadata.selected = Some(selected);
        metadata.value = Some(match episode.show.as_deref() {
            Some(show) => show.to_owned(),
            None => "seed template".to_owned(),
        });
        ui.pilot_track(&response, metadata);
        if response.clicked() {
            intents.push(ChatWindowIntent::SelectContentEpisode(episode.id.clone()));
        }
    }

    /// The focused episode's spine — chapters, and their slides in order.
    fn content_board(&self, ui: &mut egui::Ui) {
        let Some(spine) = self.content.spine.as_ref() else {
            ui.label(
                RichText::new(match self.content_focus.as_ref() {
                    Some(_) => "Reading the episode…",
                    None => "Choose an episode to see its slides.",
                })
                .color(cl::ink::MUTED),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "{} episode{} in the content root",
                    self.content.episodes.len(),
                    if self.content.episodes.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                ))
                .small()
                .color(cl::ink::SUBTLE),
            );
            return;
        };

        ui.label(
            RichText::new(&spine.title)
                .strong()
                .size(17.0)
                .color(cl::ink::STRONG),
        );
        let slides = spine.slides().count();
        ui.label(
            RichText::new(format!(
                "{} chapter{} · {slides} slide{}",
                spine.chapters.len(),
                if spine.chapters.len() == 1 { "" } else { "s" },
                if slides == 1 { "" } else { "s" }
            ))
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("chat.content.board.scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for chapter in &spine.chapters {
                    micro_heading(ui, &chapter.title.to_uppercase());
                    for slide in &chapter.slides {
                        // The deck's own convention: "<chapter> · <name>".
                        // A Slide document has no title of its own.
                        let response =
                            sidebar_row(ui, &format!("{} · {}", slide.chapter, slide.name), false);
                        let (group, slug) = split_address(&spine.id);
                        let mut metadata = NodeMetadata::new(
                            CONTENT_BOARD_ID
                                .child(&PilotKey::new(group))
                                .child(&PilotKey::new(slug))
                                .child(&PilotKey::new(slide.name.clone())),
                            Role::Tab,
                            slide.name.clone(),
                        );
                        metadata.value = Some(slide.template.clone());
                        ui.pilot_track(&response, metadata);
                    }
                }
            });
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
                    if let Some(provider) = self.runtime.selected_provider() {
                        let descriptor = provider.registration.manifest.descriptor;
                        let response = sidebar_row(ui, descriptor.short_label, true);
                        let active = active_session_count(&self.runtime.provider_views());
                        let mut metadata = NodeMetadata::new(
                            PROVIDERS_ID.child(&PilotKey::new("compact")),
                            Role::Button,
                            "Open AI Providers",
                        );
                        metadata.value = Some(format!(
                            "{} · {active} active session{}",
                            descriptor.label,
                            if active == 1 { "" } else { "s" }
                        ));
                        ui.pilot_track(&response, metadata);
                        if response.clicked() {
                            intents.push(ChatWindowIntent::SetOverlay(Some(
                                DomainOverlay::ProviderCatalog,
                            )));
                        }
                    }
                }
                ChatDomain::Content => {
                    // Compact: the focused episode, or the census count when
                    // nothing is chosen. Tapping opens the full picker.
                    let label = match self.content_focus_episode() {
                        Some(episode) => episode.title.clone(),
                        None => format!("{} episodes", self.content.episodes.len()),
                    };
                    let response = sidebar_row(ui, &label, self.content_focus.is_some());
                    let mut metadata = NodeMetadata::new(
                        CONTENT_ID.child(&PilotKey::new("compact")),
                        Role::Button,
                        "Open content",
                    );
                    metadata.value = Some(label);
                    ui.pilot_track(&response, metadata);
                    if response.clicked() {
                        intents.push(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Picker)));
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

    /// The providers that have earned a tab beside AI and Social. A tab is
    /// the reward for being connected, so the predicate is activation and
    /// nothing else; registration order is preserved so the strip never
    /// reshuffles under the operator's finger.
    fn connected_tabs(&self) -> Vec<(ProviderId, &'static str, &'static str)> {
        self.runtime
            .provider_views()
            .iter()
            .filter(|provider| provider.registration.activation.is_activated())
            .map(|provider| {
                let descriptor = provider.registration.manifest.descriptor;
                (descriptor.id, descriptor.label, descriptor.short_label)
            })
            .collect()
    }

    /// The provider the rail is scoped to: `provider_tab`, but only while it
    /// still names a connected provider. Sign-out therefore drops the tab
    /// AND returns the roster without anyone having to send an intent.
    fn focused_provider_tab(&self) -> Option<ProviderId> {
        let focus = self.provider_tab?;
        self.connected_tabs()
            .into_iter()
            .find(|(id, _, _)| *id == focus)
            .map(|(id, _, _)| id)
    }

    /// What stands where the roster stands when the rail is scoped to one
    /// provider: the way back to the roster, then that provider's own row so
    /// its status and its login stay one tap away.
    fn provider_tab_header(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let descriptor = provider.registration.manifest.descriptor;
        let back = sidebar_destination_button(ui, "‹  All providers", true);
        let mut metadata = NodeMetadata::new(
            PROVIDER_TAB_ID.child(&PilotKey::new("all")),
            Role::Button,
            "All providers",
        );
        metadata.description = Some("Back to the full provider roster.".to_owned());
        ui.pilot_track(&back, metadata);
        if back.clicked() {
            intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Ai));
        }
        ui.add_space(2.0);
        micro_heading(ui, descriptor.label);
        self.provider_row(ui, provider, true, false, intents);
    }

    fn provider_roster(
        &self,
        ui: &mut egui::Ui,
        providers: &[ProviderView],
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        micro_heading(ui, "AI PROVIDERS");
        for provider in providers {
            self.provider_row(ui, provider, true, false, intents);
        }
        ui.add_space(2.0);
        let more = sidebar_destination_button(ui, "＋  More providers…", true);
        let mut metadata = NodeMetadata::new(PROVIDERS_ID, Role::Button, "Open AI Providers");
        metadata.description =
            Some("All built-in providers plus Local or Custom setup.".to_owned());
        ui.pilot_track(&more, metadata);
        if more.clicked() {
            intents.push(ChatWindowIntent::SetOverlay(Some(
                DomainOverlay::ProviderCatalog,
            )));
        }
    }

    fn provider_row(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        expanded: bool,
        in_catalog: bool,
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
        let home = if in_catalog {
            PROVIDER_CATALOG_ID
        } else {
            PROVIDERS_ID
        };
        let mut metadata = NodeMetadata::new(
            home.child(&PilotKey::new(descriptor.id.0)),
            Role::Tab,
            descriptor.label,
        );
        metadata.selected = Some(selected);
        metadata.value = Some(provider.registration.activation.label().to_owned());
        ui.pilot_track(&response, metadata);
        if response.clicked() {
            intents.push(ChatWindowIntent::SelectProvider(descriptor.id));
            intents.push(ChatWindowIntent::SetOverlay(None));
        }
    }

    /// The transcripts already on this machine, newest first.
    ///
    /// This is a SECOND source beside the live registry, not a replacement.
    /// History-backed providers have no live sessions at all — which is
    /// exactly why the rail could only ever show a count before H1.
    fn history_rows<'a>(
        &'a self,
        providers: &'a [ProviderView],
    ) -> Vec<(&'a ProviderView, &'a IndexedSession)> {
        let mut rows = providers
            .iter()
            .filter_map(|provider| {
                let id = provider.registration.manifest.descriptor.id;
                let index = self.histories.get(&id)?.index.as_ref()?;
                Some(
                    index
                        .sessions
                        .iter()
                        .map(move |session| (provider, session)),
                )
            })
            .flatten()
            .collect::<Vec<_>>();
        // Most recent first: the operator is looking for what they were just
        // doing, never for their oldest transcript.
        rows.sort_by(|(_, left), (_, right)| right.modified_millis.cmp(&left.modified_millis));
        rows
    }

    fn session_groups(
        &self,
        ui: &mut egui::Ui,
        providers: &[ProviderView],
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let history = self.history_rows(providers);
        if providers
            .iter()
            .all(|provider| provider.registration.sessions.is_empty())
            && history.is_empty()
        {
            micro_heading(ui, "SESSIONS");
            if let Some(error) = self
                .histories
                .values()
                .find_map(|history| history.error.as_deref())
            {
                ui.label(RichText::new(error).small().color(Tone::Coral.color()));
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
        if !history.is_empty() {
            let heading = ui.scope(|ui| micro_heading(ui, "RECENT CHATS")).response;
            let mut metadata = NodeMetadata::new(HISTORY_SECTION_ID, Role::Region, "Recent chats");
            metadata.value = Some(history.len().to_string());
            ui.pilot_track(&heading, metadata);
            for (provider, session) in history {
                let descriptor = provider.registration.manifest.descriptor;
                let selected = self.history_focus == Some((descriptor.id, session.locator));
                let label = format!(
                    "{}  {}
   {} · {}",
                    descriptor.glyph,
                    session.summary.title,
                    descriptor.label,
                    session.summary.workspace
                );
                let response = sidebar_row(ui, &label, selected);
                let mut metadata = NodeMetadata::new(
                    SESSIONS_ID
                        .child(&PilotKey::new(descriptor.id.0))
                        .child(&PilotKey::new("history"))
                        .child(&PilotKey::new(session.locator.value().to_string())),
                    Role::Tab,
                    session.summary.title.clone(),
                );
                metadata.selected = Some(selected);
                metadata.value = Some(format!("{} · {}", descriptor.label, session.started_at));
                ui.pilot_track(&response, metadata);
                if response.clicked() {
                    intents.push(ChatWindowIntent::SelectHistorySession {
                        provider: descriptor.id,
                        locator: session.locator,
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
                    ChatDomain::Content => self.content_board(ui),
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

        // A transcript already on this machine outranks the live stream: the
        // operator picked one specific past conversation, and the pane must
        // draw THAT or say plainly why it cannot.
        if let Some((focus_provider, locator)) = self.history_focus {
            self.history_transcript(ui, focus_provider, locator);
            return;
        }

        if !provider.registration.activation.is_activated() {
            self.provider_activation_ceremony(ui, &provider, intents);
            return;
        }

        let events = provider
            .registration
            .events
            .iter()
            .filter(|event| is_conversation_event(event, provider.active_session));
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

    /// Draw a past transcript. Read-only by construction: nothing on this
    /// path can write, and the composer disables itself for the same reason.
    fn history_transcript(&self, ui: &mut egui::Ui, provider: ProviderId, locator: HistoryLocator) {
        let indexed = self
            .histories
            .get(&provider)
            .and_then(|history| history.index.as_ref())
            .and_then(|index| index.sessions.iter().find(|s| s.locator == locator));
        let response = ui
            .horizontal(|ui| {
                ui.label(
                    RichText::new("Past transcript")
                        .small()
                        .color(Tone::Amber.color()),
                );
                if let Some(session) = indexed {
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            session.summary.workspace, session.started_at
                        ))
                        .small()
                        .color(cl::ink::MUTED),
                    );
                }
            })
            .response;
        let mut metadata = NodeMetadata::new(HISTORY_BANNER_ID, Role::Region, "Past transcript");
        metadata.value = indexed.map(|session| session.summary.title.clone());
        ui.pilot_track(&response, metadata);

        let Some(page) = &self.history_page else {
            empty_state(
                ui,
                "Reading the transcript",
                "The host is reading this conversation from disk.",
            );
            return;
        };
        egui::ScrollArea::vertical()
            .id_salt(("chat.history.transcript", provider, locator.value()))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if page.truncated_before {
                    ui.label(
                        RichText::new("Showing the first page — earlier messages are not loaded.")
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                }
                if page.events.is_empty() {
                    empty_state(
                        ui,
                        "This transcript decoded to no messages",
                        "The file was readable, but nothing in it rendered as a message.",
                    );
                }
                for event in &page.events {
                    event_row(ui, &event.content);
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

    fn composer(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
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
                    // ASSIST — beside the route truth it activates. Never
                    // provider- or capability-gated: one toggle, one
                    // acknowledgement, every provider.
                    let assist_label = if self.assist_enabled {
                        "⚡ Assist on"
                    } else {
                        "⚡ Assist"
                    };
                    let assist = ui
                        .add(egui::Button::new(assist_label).small().selected(self.assist_enabled))
                        .on_hover_text(
                            "Assist mode: chat and the app work as one — chat sees your \
                             route and open work and may offer pop-up suggestions you \
                             approve or dismiss. One setting for every provider.",
                        );
                    let mut assist_metadata =
                        NodeMetadata::new(ASSIST_ID, Role::Button, "Assist mode");
                    assist_metadata.value =
                        Some(if self.assist_enabled { "on" } else { "off" }.to_owned());
                    ui.pilot_track(&assist, assist_metadata);
                    if assist.clicked() {
                        intents.push(ChatWindowIntent::ToggleAssist {
                            enable: !self.assist_enabled,
                        });
                    }
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
                    let media = composer_capability_button(
                        ui,
                        MEDIA_ID,
                        "▧ Image / video",
                        self.generation_status.image_available
                            || supports(Capability::Images)
                            || supports(Capability::Video),
                        "Image generation is not provisioned and this provider has not qualified media.",
                    );
                    if media.clicked() {
                        intents.push(ChatWindowIntent::SetOverlay(Some(
                            DomainOverlay::Generation,
                        )));
                    }
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
                    if self.composer_focus_requested && editor.enabled() {
                        editor.request_focus();
                        self.composer_focus_requested = false;
                    }
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
                            // Content owns no conversation, so a keystroke has
                            // nowhere to persist — deliberately no draft store.
                            ChatDomain::Content => {}
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
                                let send_target = selected.as_ref().and_then(|provider| {
                                    provider.active_session.map(|session| {
                                        (
                                            provider.registration.manifest.descriptor.id,
                                            session,
                                            provider.registration.can_send(),
                                        )
                                    })
                                });
                                let slash = parse_operator_slash(&draft);
                                // A past transcript is read-only, full stop —
                                // operator commands included. Saying so beats
                                // the transport reason, which would be true
                                // but would not be the ACTUAL reason.
                                let reading_history = self.history_focus.is_some();
                                let enabled = !reading_history
                                    && (slash.is_some()
                                        || (send_target.is_some_and(|(_, _, can_send)| can_send)
                                            && !draft.trim().is_empty()));
                                let reason = if reading_history {
                                    "This is a past transcript — it is read-only."
                                } else if slash.is_some() {
                                    "Run this operator command."
                                } else if draft.trim().is_empty() {
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
                                    if let Some(command) = slash {
                                        // Provenance law: recognize at the
                                        // operator's input-submit event, from
                                        // the operator's own text buffer,
                                        // BEFORE the text becomes a message.
                                        // Clear the draft so nothing is sent.
                                        if let Some(provider) = selected.as_ref() {
                                            intents.push(ChatWindowIntent::UpdateAiDraft {
                                                provider: provider
                                                    .registration
                                                    .manifest
                                                    .descriptor
                                                    .id,
                                                value: String::new(),
                                            });
                                        }
                                        intents.push(ChatWindowIntent::OperatorSlash {
                                            command,
                                            synthetic: egui_pilot_ai::input_is_synthetic(ui.ctx()),
                                        });
                                    } else if let Some((provider, session, _)) = send_target {
                                        intents.push(ChatWindowIntent::SendTurn {
                                            provider,
                                            session,
                                            text: draft,
                                        });
                                    }
                                }
                            }
                        }
                        ChatDomain::Content => {
                            const REASON: &str =
                                "Content has no conversation — choose a provider under AI to ask about it.";
                            let send = ui.add_enabled(
                                false,
                                egui::Button::new("Send").min_size(egui::vec2(86.0, 42.0)),
                            );
                            let send = cl::tip(send, REASON);
                            let mut metadata =
                                NodeMetadata::new(SEND_ID, Role::Button, "Send content message");
                            metadata.description = Some(REASON.to_owned());
                            ui.pilot_track(&send, metadata);
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
            ChatDomain::Content => String::new(),
            ChatDomain::Social => self
                .social
                .selected()
                .and_then(|thread| self.social.draft(&thread.id).ok())
                .unwrap_or_default()
                .to_owned(),
        }
    }

    fn composer_hint(&self) -> &'static str {
        if self.domain == ChatDomain::Ai && self.history_focus.is_some() {
            return "Past transcript — read-only";
        }
        match self.domain {
            ChatDomain::Ai => "Draft for this AI session…",
            ChatDomain::Content => "Browsing content — no conversation here",
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
                        if !provider.registration.activation.is_activated() {
                            let continue_setup = ui.button("Continue setup in conversation");
                            if continue_setup.clicked() {
                                intents.push(ChatWindowIntent::SelectProvider(*provider_id));
                                intents.push(ChatWindowIntent::SetOverlay(None));
                            }
                        }
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

    fn provider_catalog_overlay(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if self.domain_states.ai.overlay != Some(DomainOverlay::ProviderCatalog) {
            return;
        }
        let providers = self.runtime.provider_views();
        egui::Window::new("AI Providers")
            .id(egui::Id::new("chat.provider.catalog"))
            .anchor(Align2::LEFT_TOP, [270.0, 70.0])
            .default_width(360.0)
            .collapsible(false)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(7, 22, 26))
                    .stroke(Stroke::new(1.0, Tone::Cyan.color()))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ui.ctx(), |ui| {
                ui.set_width(336.0);
                let panel = ui
                    .vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("chat.provider.catalog.scroll")
                            .max_height(350.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("CURRENT ACTIVE SESSION(S)")
                                            .monospace()
                                            .small()
                                            .color(Tone::Cyan.color()),
                                    );
                                    let active = providers
                                        .iter()
                                        .filter(|provider| {
                                            provider.registration.sessions.iter().any(|session| {
                                                session.group == SessionGroup::Active
                                                    || session.state == SessionState::Running
                                            })
                                        })
                                        .collect::<Vec<_>>();
                                    if active.is_empty() {
                                        ui.label(
                                            RichText::new("No active provider sessions")
                                                .small()
                                                .color(cl::ink::SUBTLE),
                                        );
                                    } else {
                                        for provider in active {
                                            let descriptor =
                                                provider.registration.manifest.descriptor;
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}  {} · {} session{}",
                                                    descriptor.glyph,
                                                    descriptor.label,
                                                    provider.registration.sessions.len(),
                                                    if provider.registration.sessions.len() == 1 {
                                                        ""
                                                    } else {
                                                        "s"
                                                    }
                                                ))
                                                .small()
                                                .color(cl::ink::BASE),
                                            );
                                        }
                                    }
                                    ui.add_space(8.0);
                                    micro_heading(ui, "AVAILABLE PROVIDERS");
                                    for provider in &providers {
                                        self.provider_row(ui, provider, true, true, intents);
                                    }
                                    for supplement in SUPPLEMENTS.iter().filter(|supplement| {
                                        supplement.action == CatalogSupplementAction::FutureAdapter
                                    }) {
                                        let response =
                                            catalog_supplement_row(ui, supplement, false);
                                        let mut metadata = NodeMetadata::new(
                                            PROVIDER_CATALOG_ID
                                                .child(&PilotKey::new(supplement.provider_id.0)),
                                            Role::Status,
                                            supplement.label,
                                        );
                                        metadata.description = Some(supplement.summary.to_owned());
                                        metadata.value = Some("Adapter not qualified".to_owned());
                                        ui.pilot_track(&response, metadata);
                                    }
                                    ui.add_space(8.0);
                                    micro_heading(ui, "CONFIGURED PROFILES");
                                    if self.provider_profiles.is_empty() {
                                        ui.label(
                                            RichText::new("No Local or Custom profiles yet")
                                                .small()
                                                .color(cl::ink::SUBTLE),
                                        );
                                    } else {
                                        for profile in &self.provider_profiles {
                                            egui::Frame::new()
                                                .fill(Color32::from_rgb(9, 28, 31))
                                                .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                                                .corner_radius(CornerRadius::same(3))
                                                .inner_margin(egui::Margin::symmetric(8, 6))
                                                .show(ui, |ui| {
                                                    ui.set_width(ui.available_width());
                                                    ui.label(
                                                        RichText::new(&profile.display_name)
                                                            .small()
                                                            .strong()
                                                            .color(cl::ink::STRONG),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} · {}",
                                                            profile.kind.label(),
                                                            profile.activation_reason()
                                                        ))
                                                        .small()
                                                        .color(Tone::Amber.color()),
                                                    );
                                                });
                                        }
                                    }
                                });
                            });
                        ui.separator();
                        micro_heading(ui, "ADD A PROVIDER");
                        for supplement in SUPPLEMENTS.iter().filter(|supplement| {
                            matches!(supplement.action, CatalogSupplementAction::Configure(_))
                        }) {
                            let response = catalog_supplement_row(ui, supplement, true);
                            let mut metadata = NodeMetadata::new(
                                PROVIDER_CATALOG_ID.child(&PilotKey::new(supplement.provider_id.0)),
                                Role::Button,
                                supplement.label,
                            );
                            metadata.description = Some(supplement.summary.to_owned());
                            ui.pilot_track(&response, metadata);
                            if response.clicked() {
                                if let CatalogSupplementAction::Configure(kind) = supplement.action
                                {
                                    intents.push(ChatWindowIntent::UpdateProviderProfileDraft(
                                        ProviderProfileDraft::for_kind(kind),
                                    ));
                                    intents.push(ChatWindowIntent::SetOverlay(Some(
                                        DomainOverlay::ProviderConfiguration(kind),
                                    )));
                                }
                            }
                        }
                        ui.add_space(6.0);
                        if ui.button("Close").clicked() {
                            intents.push(ChatWindowIntent::SetOverlay(None));
                        }
                    })
                    .response;
                ui.pilot_track(
                    &panel,
                    NodeMetadata::new(PROVIDER_CATALOG_ID, Role::Dialog, "AI provider catalog"),
                );
            });
    }

    fn provider_configuration_overlay(
        &self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let Some(DomainOverlay::ProviderConfiguration(kind)) =
            self.domain_states.ai.overlay.as_ref()
        else {
            return;
        };
        let kind = *kind;
        let mut draft = if self.provider_profile_draft.kind == kind {
            self.provider_profile_draft.clone()
        } else {
            ProviderProfileDraft::for_kind(kind)
        };
        egui::Window::new(format!("Configure {}", kind.label()))
            .id(egui::Id::new("chat.provider.configuration"))
            .anchor(Align2::LEFT_TOP, [270.0, 92.0])
            .default_width(420.0)
            .collapsible(false)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(7, 22, 26))
                    .stroke(Stroke::new(1.0, Tone::Amber.color()))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ui.ctx(), |ui| {
                ui.set_width(390.0);
                let panel = ui
                    .vertical(|ui| {
                        ui.label(
                            RichText::new("NON-SECRET TYPED CONFIGURATION")
                                .monospace()
                                .small()
                                .color(Tone::Amber.color()),
                        );
                        ui.label(
                            RichText::new(match kind {
                                ProviderConfigurationKind::Local => {
                                    "Local endpoints are restricted to this computer."
                                }
                                ProviderConfigurationKind::Custom => {
                                    "Custom endpoints must use HTTPS. Credentials are never entered here."
                                }
                            })
                            .small()
                            .color(cl::ink::MUTED),
                        );
                        ui.add_space(8.0);
                        ui.label(RichText::new("Display name").small());
                        let name = ui.text_edit_singleline(&mut draft.display_name);
                        let mut name_metadata = NodeMetadata::new(
                            PROVIDER_CONFIG_NAME_ID,
                            Role::TextInput,
                            "Provider display name",
                        );
                        name_metadata.value = Some(draft.display_name.clone());
                        ui.pilot_track(&name, name_metadata);
                        ui.label(RichText::new("Endpoint").small());
                        let endpoint = ui.text_edit_singleline(&mut draft.endpoint);
                        let mut endpoint_metadata = NodeMetadata::new(
                            PROVIDER_CONFIG_ENDPOINT_ID,
                            Role::TextInput,
                            "Provider endpoint",
                        );
                        endpoint_metadata.description = Some(match kind {
                            ProviderConfigurationKind::Local => {
                                "Loopback HTTP or HTTPS endpoint only.".to_owned()
                            }
                            ProviderConfigurationKind::Custom => {
                                "HTTPS endpoint without inline credentials.".to_owned()
                            }
                        });
                        endpoint_metadata.value = Some(draft.endpoint.clone());
                        ui.pilot_track(&endpoint, endpoint_metadata);
                        ui.label(RichText::new("Model (optional)").small());
                        let model = ui.text_edit_singleline(&mut draft.model);
                        let mut model_metadata = NodeMetadata::new(
                            PROVIDER_CONFIG_MODEL_ID,
                            Role::TextInput,
                            "Provider model",
                        );
                        model_metadata.value = Some(draft.model.clone());
                        ui.pilot_track(&model, model_metadata);
                        if name.changed() || endpoint.changed() || model.changed() {
                            intents.push(ChatWindowIntent::UpdateProviderProfileDraft(
                                draft.clone(),
                            ));
                        }
                        ui.add_space(8.0);
                        let validation = draft.validate();
                        if let Err(error) = &validation {
                            ui.label(
                                RichText::new(error.to_string())
                                    .small()
                                    .color(Tone::Coral.color()),
                            );
                        } else {
                            ui.label(
                                RichText::new(
                                    "Profile will be saved. Live transport remains unavailable until its adapter qualifies.",
                                )
                                .small()
                                .color(Tone::Amber.color()),
                            );
                        }
                        ui.horizontal(|ui| {
                            let save = ui.add_enabled(
                                validation.is_ok(),
                                egui::Button::new("Save profile"),
                            );
                            ui.pilot_track(
                                &save,
                                NodeMetadata::new(
                                    PROVIDER_CONFIG_SAVE_ID,
                                    Role::Button,
                                    "Save provider profile",
                                ),
                            );
                            if save.clicked() {
                                intents.push(ChatWindowIntent::SaveProviderProfile(draft.clone()));
                            }
                            if ui.button("Back").clicked() {
                                intents.push(ChatWindowIntent::SetOverlay(Some(
                                    DomainOverlay::ProviderCatalog,
                                )));
                            }
                            if ui.button("Cancel").clicked() {
                                intents.push(ChatWindowIntent::SetOverlay(None));
                            }
                        });
                    })
                    .response;
                ui.pilot_track(
                    &panel,
                    NodeMetadata::new(
                        PROVIDER_CONFIG_ID,
                        Role::Dialog,
                        format!("Configure {}", kind.label()),
                    ),
                );
            });
    }

    fn provider_activation_ceremony(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let descriptor = provider.registration.manifest.descriptor;
        let (title, detail) = match &provider.registration.activation {
            ActivationState::SigningIn => (
                format!("Starting {}", descriptor.short_label),
                "Launching the verified provider client and waiting for its device code.",
            ),
            ActivationState::AwaitingAuthorization(_) => (
                "Complete xAI sign-in".to_owned(),
                "DevPULSE sent the secure device page to your computer's default browser. Keep Chat open while xAI and the official Grok client complete authorization.",
            ),
            ActivationState::Verifying => (
                format!("Verifying your {} subscription", descriptor.short_label),
                "Authorization completed. DevPULSE is verifying the provider's subscription session.",
            ),
            ActivationState::Failed { .. } | ActivationState::Degraded { .. } => (
                format!("{} needs attention", descriptor.short_label),
                "The bounded provider flow stopped safely. Review the status and retry from here.",
            ),
            ActivationState::SignedOut { .. } => (
                format!("Connect {}", descriptor.short_label),
                "Sign in through the official provider client. Your subscription and credentials stay provider-owned.",
            ),
            _ => (
                format!("{} is unavailable", descriptor.short_label),
                "This provider cannot start a qualified session yet.",
            ),
        };
        egui::Frame::new()
            .fill(Color32::from_rgb(8, 24, 28))
            .stroke(Stroke::new(1.0, provider_tone(descriptor.accent).color()))
            .corner_radius(CornerRadius::same(5))
            .inner_margin(egui::Margin::same(18))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new("PROVIDER CONNECTION")
                        .monospace()
                        .small()
                        .color(provider_tone(descriptor.accent).color()),
                );
                ui.label(
                    RichText::new(title)
                        .strong()
                        .size(22.0)
                        .color(cl::ink::STRONG),
                );
                ui.label(RichText::new(detail).color(cl::ink::MUTED));
                ui.add_space(10.0);
                self.provider_activation_action(ui, provider, intents);
                self.provider_activation_trace(ui, descriptor.id, intents);
            });
    }

    fn provider_activation_trace(
        &self,
        ui: &mut egui::Ui,
        provider: ProviderId,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let latest_attempt = self.provider_trace.iter().rev().find_map(|fact| {
            (fact.ev.provider() == provider)
                .then(|| fact.ev.attempt())
                .flatten()
        });
        let Some(attempt) = latest_attempt else {
            return;
        };
        let expanded = self
            .provider_trace_expanded
            .get(&provider)
            .copied()
            .unwrap_or(false);
        ui.add_space(10.0);
        let toggle = ui.small_button(if expanded {
            "▾ Hide activation trace"
        } else {
            "▸ Show activation trace"
        });
        ui.pilot_track(
            &toggle,
            NodeMetadata::new(
                PROVIDER_TRACE_ID.child(&PilotKey::new(provider.0)),
                Role::Button,
                "Toggle provider activation trace",
            )
            .value(if expanded { "expanded" } else { "collapsed" }),
        );
        if toggle.clicked() {
            intents.push(ChatWindowIntent::SetProviderTraceExpanded {
                provider,
                expanded: !expanded,
            });
        }
        if !expanded {
            return;
        }
        egui::Frame::new()
            .fill(Color32::from_rgb(3, 12, 14))
            .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
            .corner_radius(CornerRadius::same(3))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for fact in self.provider_trace.iter().filter(|fact| {
                    fact.ev.provider() == provider && fact.ev.attempt() == Some(attempt)
                }) {
                    ui.label(
                        RichText::new(format!(
                            "{:>6}ms  {}",
                            fact.ms,
                            provider_event_line(&fact.ev)
                        ))
                        .monospace()
                        .small()
                        .color(cl::ink::MUTED),
                    );
                }
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
                RichText::new("Authorize Grok in your default browser")
                    .color(Tone::Amber.color())
                    .strong(),
            );
            let code = ui.add(
                egui::Label::new(
                    RichText::new(&prompt.user_code.value)
                        .monospace()
                        .size(24.0)
                        .color(cl::ink::STRONG),
                )
                .selectable(true),
            );
            ui.pilot_track(
                &code,
                NodeMetadata::new(
                    PROVIDER_AUTH_CODE_ID.child(&PilotKey::new(provider_id.0)),
                    Role::Status,
                    "Grok authorization code",
                )
                .value(prompt.user_code.value.clone()),
            );
            ui.label(
                RichText::new(
                    "Copy this code. Paste it into xAI if the page does not fill it automatically.",
                )
                .small()
                .color(cl::ink::MUTED),
            );
            ui.horizontal(|ui| {
                let copy = ui.button("Copy code");
                ui.pilot_track(
                    &copy,
                    NodeMetadata::new(
                        PROVIDER_AUTH_COPY_ID.child(&PilotKey::new(provider_id.0)),
                        Role::Button,
                        "Copy Grok authorization code",
                    ),
                );
                if copy.clicked() {
                    intents.push(ChatWindowIntent::CopyProviderAuthorizationCode(provider_id));
                }
                let reopen = ui.button("Reopen in default browser");
                ui.pilot_track(
                    &reopen,
                    NodeMetadata::new(
                        PROVIDER_AUTH_BROWSER_ID.child(&PilotKey::new(provider_id.0)),
                        Role::Button,
                        "Reopen Grok sign-in in system default browser",
                    ),
                );
                if reopen.clicked() {
                    intents.push(ChatWindowIntent::OpenProviderAuthorization(provider_id));
                }
            });
            return;
        }
        if matches!(provider.registration.activation, ActivationState::Verifying) {
            let label = format!(
                "Verifying {} connection…",
                provider.registration.manifest.descriptor.short_label
            );
            let response = ui.add_enabled(false, egui::Button::new(&label));
            ui.pilot_track(
                &response,
                NodeMetadata::new(
                    PROVIDER_LOGIN_ID.child(&PilotKey::new(provider_id.0)),
                    Role::Button,
                    "Verifying provider connection",
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

    /// The Imagine generation panel (ADR-011). The prompt is the composer
    /// draft; the generate action is the operator's explicit spend
    /// authorization; drafts stay in the governed cache until saved or
    /// discarded.
    fn generation_overlay(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if self.domain_states.ai.overlay != Some(DomainOverlay::Generation) {
            return;
        }
        let status = self.generation_status.clone();
        let prompt = self.current_draft();
        let drafts = self.generation_drafts.clone();
        egui::Window::new("Create with AI")
            .id(egui::Id::new("chat.generation.panel"))
            .anchor(Align2::LEFT_TOP, [270.0, 92.0])
            .default_width(460.0)
            .collapsible(false)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(7, 22, 26))
                    .stroke(Stroke::new(1.0, Tone::Cyan.color()))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ui.ctx(), |ui| {
                ui.set_width(430.0);
                let panel = ui
                    .vertical(|ui| {
                        ui.label(
                            RichText::new("GENERATION DRAFTS · CACHED 30 DAYS UNTIL SAVED")
                                .monospace()
                                .small()
                                .color(Tone::Cyan.color()),
                        );
                        ui.horizontal(|ui| {
                            let image_mode =
                                ui.add_enabled(true, egui::Button::new("▧ Image").selected(true));
                            ui.pilot_track(
                                &image_mode,
                                NodeMetadata::new(
                                    GENERATION_MODE_IMAGE_ID,
                                    Role::Tab,
                                    "Image generation mode",
                                )
                                .value("selected"),
                            );
                            let video_mode = ui
                                .add_enabled(false, egui::Button::new("▶ Video"))
                                .on_disabled_hover_text(
                                    "Video generation (grok-imagine-video-1.5) is staged for the next pass.",
                                );
                            ui.pilot_track(
                                &video_mode,
                                NodeMetadata::new(
                                    GENERATION_MODE_VIDEO_ID,
                                    Role::Tab,
                                    "Video generation mode",
                                )
                                .value("staged"),
                            );
                        });
                        ui.label(
                            RichText::new(if prompt.trim().is_empty() {
                                "Type the prompt in the message composer, then generate."
                                    .to_owned()
                            } else {
                                format!("Prompt: {}", prompt.trim())
                            })
                            .small()
                            .color(cl::ink::MUTED),
                        );
                        let can_generate = status.image_available
                            && !status.in_flight
                            && !prompt.trim().is_empty();
                        let generate_label = if status.in_flight {
                            "Generating…"
                        } else {
                            "Generate image · grok-imagine-image-quality · usage-billed"
                        };
                        let generate =
                            ui.add_enabled(can_generate, egui::Button::new(generate_label));
                        let generate = if status.image_available {
                            generate
                        } else {
                            generate.on_disabled_hover_text(status.unavailable_reason.clone())
                        };
                        let mut generate_metadata = NodeMetadata::new(
                            GENERATION_GENERATE_ID,
                            Role::Button,
                            "Generate image",
                        );
                        generate_metadata.description = Some(if status.image_available {
                            "Sends the composer prompt to the xAI Imagine API. Usage-billed."
                                .to_owned()
                        } else {
                            status.unavailable_reason.clone()
                        });
                        ui.pilot_track(&generate, generate_metadata);
                        if generate.clicked() {
                            intents.push(ChatWindowIntent::RequestImageGeneration {
                                prompt: prompt.trim().to_owned(),
                            });
                        }
                        if let Some(error) = &status.last_error {
                            ui.label(
                                RichText::new(error).small().color(Tone::Coral.color()),
                            );
                        }
                        ui.add_space(8.0);
                        if drafts.is_empty() {
                            ui.label(
                                RichText::new("No drafts yet. Generated results preview here.")
                                    .small()
                                    .color(cl::ink::SUBTLE),
                            );
                        }
                        for draft in &drafts {
                            let row = egui::Frame::new()
                                .fill(Color32::from_rgb(8, 25, 29))
                                .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                                .corner_radius(CornerRadius::same(3))
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        if let Some((width, height, pixels)) = &draft.thumb {
                                            let handle = self
                                                .generation_textures
                                                .0
                                                .entry(draft.id.clone())
                                                .or_insert_with(|| {
                                                    ui.ctx().load_texture(
                                                        format!("imagine-thumb-{}", draft.id),
                                                        egui::ColorImage::from_rgba_unmultiplied(
                                                            [*width as usize, *height as usize],
                                                            pixels,
                                                        ),
                                                        egui::TextureOptions::LINEAR,
                                                    )
                                                });
                                            let scale = 96.0 / (*height).max(1) as f32;
                                            ui.image((
                                                handle.id(),
                                                egui::vec2(
                                                    (*width as f32 * scale).min(160.0),
                                                    96.0,
                                                ),
                                            ));
                                        }
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&draft.prompt)
                                                    .small()
                                                    .color(cl::ink::BASE),
                                            );
                                            ui.horizontal(|ui| {
                                                let save =
                                                    ui.small_button("Save to library");
                                                ui.pilot_track(
                                                    &save,
                                                    NodeMetadata::new(
                                                        GENERATION_DRAFT_ID
                                                            .child(&PilotKey::new(&draft.id))
                                                            .child(&PilotKey::new("save")),
                                                        Role::Button,
                                                        "Save draft to library",
                                                    ),
                                                );
                                                if save.clicked() {
                                                    intents.push(
                                                        ChatWindowIntent::SaveGenerationDraft {
                                                            id: draft.id.clone(),
                                                        },
                                                    );
                                                }
                                                let discard = ui.small_button("Ignore");
                                                ui.pilot_track(
                                                    &discard,
                                                    NodeMetadata::new(
                                                        GENERATION_DRAFT_ID
                                                            .child(&PilotKey::new(&draft.id))
                                                            .child(&PilotKey::new("discard")),
                                                        Role::Button,
                                                        "Discard draft",
                                                    ),
                                                );
                                                if discard.clicked() {
                                                    intents.push(
                                                        ChatWindowIntent::DiscardGenerationDraft {
                                                            id: draft.id.clone(),
                                                        },
                                                    );
                                                }
                                            });
                                        });
                                    });
                                })
                                .response;
                            let mut row_metadata = NodeMetadata::new(
                                GENERATION_DRAFT_ID.child(&PilotKey::new(&draft.id)),
                                Role::Status,
                                "Generation draft",
                            );
                            row_metadata.value = Some(draft.prompt.clone());
                            ui.pilot_track(&row, row_metadata);
                        }
                        ui.add_space(6.0);
                        if ui.button("Close").clicked() {
                            intents.push(ChatWindowIntent::SetOverlay(None));
                        }
                    })
                    .response;
                let mut metadata = NodeMetadata::new(
                    GENERATION_PANEL_ID,
                    Role::Dialog,
                    "Create with AI",
                );
                metadata.description =
                    Some("Imagine generation drafts. Unsaved drafts expire after 30 days.".to_owned());
                ui.pilot_track(&panel, metadata);
            });
    }

    fn compact_picker_overlay(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if self.domain_states.get(self.domain).overlay != Some(DomainOverlay::Picker) {
            return;
        }
        egui::Window::new(match self.domain {
            ChatDomain::Ai => "Choose AI provider or session",
            ChatDomain::Content => "Choose a show or episode",
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
                ChatDomain::Content => self.content_sidebar(ui, intents),
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
                                    } else if self.imagine_flush_consent == Some(spec.id) {
                                        intents.push(ChatWindowIntent::ResolveImagineFlush {
                                            interaction: spec.id,
                                            decision: ImagineFlushDecision::FlushNow,
                                        });
                                    } else if self.assist_acknowledgement == Some(spec.id) {
                                        intents.push(
                                            ChatWindowIntent::ResolveAssistAcknowledgement {
                                                interaction: spec.id,
                                                accepted: true,
                                            },
                                        );
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
                                        } else if self.imagine_flush_consent == Some(spec.id) {
                                            intents.push(ChatWindowIntent::ResolveImagineFlush {
                                                interaction: spec.id,
                                                decision: ImagineFlushDecision::FlushAlways,
                                            });
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
                                    } else if self.imagine_flush_consent == Some(spec.id) {
                                        intents.push(ChatWindowIntent::ResolveImagineFlush {
                                            interaction: spec.id,
                                            decision: ImagineFlushDecision::Defer,
                                        });
                                    } else if self.assist_acknowledgement == Some(spec.id) {
                                        intents.push(
                                            ChatWindowIntent::ResolveAssistAcknowledgement {
                                                interaction: spec.id,
                                                accepted: false,
                                            },
                                        );
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

fn catalog_supplement_row(
    ui: &mut egui::Ui,
    supplement: &CatalogSupplement,
    clickable: bool,
) -> egui::Response {
    let response = egui::Frame::new()
        .fill(Color32::from_rgb(8, 25, 29))
        .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                provider_mark(ui, supplement.accent);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(supplement.label)
                            .small()
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                    ui.label(
                        RichText::new(supplement.summary)
                            .small()
                            .color(cl::ink::MUTED),
                    );
                });
            });
        })
        .response;
    if clickable {
        response.interact(Sense::click())
    } else {
        response.on_hover_text("This adapter is declared but not qualified yet.")
    }
}

#[derive(Debug, Clone, Copy)]
enum SessionPredicate {
    Pinned,
    Active,
    Recent,
}

fn active_session_count(providers: &[ProviderView]) -> usize {
    providers
        .iter()
        .flat_map(|provider| &provider.registration.sessions)
        .filter(|session| {
            session.group == SessionGroup::Active || session.state == SessionState::Running
        })
        .count()
}

fn is_conversation_event(event: &ChatEvent, active_session: Option<ChatSessionId>) -> bool {
    active_session.is_none_or(|session| event.session_id == session)
        && event.content.presentation() == EventPresentation::Conversation
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

/// Split a content address `"<group>/<slug>"` into its two parts. Total: an
/// address without a separator is treated as a seed slug, matching how
/// `open_episode` resolves a bare id.
fn split_address(id: &ContentEpisodeId) -> (&str, &str) {
    id.0.split_once('/').unwrap_or(("template", &id.0))
}

fn mode_button(
    ui: &mut egui::Ui,
    path: impl Into<PilotPath>,
    label: &str,
    selected: bool,
    width: f32,
) -> egui::Response {
    let path = path.into();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 35.0), Sense::click());
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
    // Violet is Social's colour alone; AI and every provider tab under it
    // are cyan, so a provider tab never reads as the other domain.
    let accent = if path == SOCIAL_MODE_ID.path() {
        Tone::Violet.color()
    } else {
        Tone::Cyan.color()
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
    let mut metadata = NodeMetadata::new(path, Role::Tab, label.to_owned());
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

fn provider_event_line(event: &ProviderEvent) -> String {
    match event {
        ProviderEvent::Selected { .. } => "provider selected".to_owned(),
        ProviderEvent::ProfileConfigured { profile_key, .. } => {
            format!("provider profile configured · {profile_key}")
        }
        ProviderEvent::ActivationRequested { .. } => "connect requested".to_owned(),
        ProviderEvent::ClientStarted { client_version, .. } => {
            format!("official client started · {client_version}")
        }
        ProviderEvent::DeviceCodeReceived { .. } => {
            "device code received · value retained only in Chat".to_owned()
        }
        ProviderEvent::CodeCopied { .. } => "device code copied".to_owned(),
        ProviderEvent::SystemBrowserDispatched { .. } => {
            "sign-in sent to system default browser".to_owned()
        }
        ProviderEvent::AuthorizationCompleted { .. } => {
            "provider authorization completed".to_owned()
        }
        ProviderEvent::RuntimeStarted { .. } => "Grok ACP started".to_owned(),
        ProviderEvent::VerificationRetryScheduled {
            retry_index,
            max_retries,
            delay_ms,
            ..
        } => format!("subscription verification retry {retry_index}/{max_retries} · {delay_ms}ms"),
        ProviderEvent::Ready { .. } => "Grok ACP ready".to_owned(),
        ProviderEvent::Failed {
            stage,
            class,
            retryable,
            ..
        } => format!(
            "failed · {} · {} · retryable={retryable}",
            stage.label(),
            class.label()
        ),
        ProviderEvent::Cancelled { .. } => "activation cancelled".to_owned(),
        ProviderEvent::Expired { .. } => "activation expired".to_owned(),
    }
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
            &HistoryCatalog::default(),
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
            &HistoryCatalog::default(),
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
    fn active_session_count_spans_all_providers() {
        let window = multi_provider_window();
        assert_eq!(active_session_count(&window.runtime.provider_views()), 2);
        assert_eq!(window.runtime.provider_views().len(), 2);
    }

    fn episode(address: &str, show: Option<&str>, title: &str) -> ContentEpisode {
        ContentEpisode {
            id: ContentEpisodeId(address.to_owned()),
            show: show.map(str::to_owned),
            title: title.to_owned(),
        }
    }

    fn spine(address: &str, title: &str, slides: &[&str]) -> ContentSpine {
        ContentSpine {
            id: ContentEpisodeId(address.to_owned()),
            title: title.to_owned(),
            chapters: vec![dpl_chat_core::ContentChapter {
                id: "main".to_owned(),
                title: "Main".to_owned(),
                slides: slides
                    .iter()
                    .enumerate()
                    .map(|(index, name)| dpl_chat_core::ContentSlide {
                        chapter: "main".to_owned(),
                        name: (*name).to_owned(),
                        template: "static/title".to_owned(),
                        index,
                    })
                    .collect(),
            }],
        }
    }

    fn content_window() -> ChatWindow {
        let mut window = window();
        window.observe_content(ContentView {
            episodes: vec![
                episode("demo/pilot", Some("demo"), "Pilot"),
                episode("demo/finale", Some("demo"), "Finale"),
                episode("template/blank", None, "Blank"),
            ],
            spine: None,
            error: None,
        });
        window
    }

    #[test]
    fn content_is_its_own_domain_and_keeps_state() {
        let mut window = content_window();
        window
            .apply(ChatWindowIntent::SelectDomain(ChatDomain::Content))
            .expect("domain intent");
        assert_eq!(window.domain, ChatDomain::Content);
        window.domain_states.get_mut(ChatDomain::Content).unread = 4;
        window
            .apply(ChatWindowIntent::SelectDomain(ChatDomain::Ai))
            .expect("domain intent");
        assert_eq!(
            window.domain_states.get(ChatDomain::Content).unread,
            4,
            "each domain owns its view state independently"
        );
        assert_eq!(window.domain_states.get(ChatDomain::Ai).unread, 0);
    }

    #[test]
    fn selecting_an_episode_asks_the_host_for_its_spine() {
        let mut window = content_window();
        let address = ContentEpisodeId("demo/pilot".to_owned());
        assert_eq!(
            window
                .apply(ChatWindowIntent::SelectContentEpisode(address.clone()))
                .expect("content intent"),
            Some(HostIntent::LoadContentSpine(address.clone()))
        );
        assert_eq!(window.content_focus, Some(address));
        assert!(
            window.content.spine.is_none(),
            "the outgoing spine is dropped so no episode wears another's slides"
        );
    }

    #[test]
    fn an_observed_spine_replaces_the_previous_one() {
        let mut window = content_window();
        window.observe_content_spine(spine("demo/pilot", "Pilot", &["title-card"]));
        window.observe_content_spine(spine("demo/finale", "Finale", &["wrap", "credits"]));
        let held = window.content.spine.as_ref().expect("a spine is held");
        assert_eq!(held.id, ContentEpisodeId("demo/finale".to_owned()));
        assert_eq!(held.slides().count(), 2);
        assert_eq!(
            window.content_focus,
            Some(ContentEpisodeId("demo/finale".to_owned())),
            "observing a spine moves the focus with it"
        );
    }

    #[test]
    fn refreshing_the_census_keeps_the_open_spine() {
        let mut window = content_window();
        window.observe_content_spine(spine("demo/pilot", "Pilot", &["title-card"]));
        window.observe_content(ContentView {
            episodes: vec![episode("demo/pilot", Some("demo"), "Pilot")],
            spine: None,
            error: None,
        });
        assert!(
            window.content.spine.is_some(),
            "a census refresh must not blank the episode being read"
        );
        assert_eq!(window.content.episodes.len(), 1);
    }

    #[test]
    fn a_vanished_episode_drops_its_focus_and_spine() {
        let mut window = content_window();
        window.observe_content_spine(spine("demo/pilot", "Pilot", &["title-card"]));
        window.observe_content(ContentView {
            episodes: vec![episode("template/blank", None, "Blank")],
            spine: None,
            error: None,
        });
        assert_eq!(window.content_focus, None);
        assert!(window.content.spine.is_none());
    }

    #[test]
    fn a_content_error_is_state_not_a_panic() {
        let mut window = window();
        window.observe_content(ContentView {
            episodes: Vec::new(),
            spine: None,
            error: Some("content root not found".to_owned()),
        });
        assert_eq!(
            window.content.error.as_deref(),
            Some("content root not found")
        );
        assert!(window.content.episodes.is_empty());
    }

    #[test]
    fn refresh_content_is_a_host_ask() {
        let mut window = content_window();
        assert_eq!(
            window
                .apply(ChatWindowIntent::RefreshContent)
                .expect("refresh intent"),
            Some(HostIntent::RefreshContent)
        );
    }

    #[test]
    fn only_activated_providers_earn_a_tab() {
        let mut ready = ProviderRegistration::planned(MANIFEST);
        ready.activation = ActivationState::Ready;
        let mut signed_out = ProviderRegistration::planned(SECOND_MANIFEST);
        signed_out.activation = ActivationState::SignedOut {
            reason: "Not signed in".into(),
            recovery: Vec::new(),
        };
        let runtime = ChatRuntime::try_from_registrations([ready, signed_out]).expect("runtime");
        let window = ChatWindow::new(
            runtime,
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        );
        let tabs = window.connected_tabs();
        assert_eq!(tabs.len(), 1, "a signed-out provider has not earned a tab");
        assert_eq!(tabs[0].0, PROVIDER);
    }

    #[test]
    fn a_provider_tab_selects_its_provider() {
        let mut window = multi_provider_window();
        window
            .apply(ChatWindowIntent::SelectProviderTab(SECOND_PROVIDER))
            .expect("provider tab intent");
        assert_eq!(window.domain, ChatDomain::Ai);
        assert_eq!(window.focused_provider_tab(), Some(SECOND_PROVIDER));
        assert_eq!(window.runtime.selected_provider_id(), Some(SECOND_PROVIDER));
    }

    #[test]
    fn selecting_a_domain_clears_the_provider_tab() {
        let mut window = multi_provider_window();
        window
            .apply(ChatWindowIntent::SelectProviderTab(SECOND_PROVIDER))
            .expect("provider tab intent");
        window
            .apply(ChatWindowIntent::SelectDomain(ChatDomain::Ai))
            .expect("domain intent");
        assert_eq!(
            window.focused_provider_tab(),
            None,
            "the AI tab is the roster fallback"
        );
    }

    #[test]
    fn signing_out_drops_the_tab_and_its_focus() {
        let mut window = multi_provider_window();
        window
            .apply(ChatWindowIntent::SelectProviderTab(SECOND_PROVIDER))
            .expect("provider tab intent");
        assert_eq!(window.focused_provider_tab(), Some(SECOND_PROVIDER));
        window
            .runtime
            .set_activation(
                SECOND_PROVIDER,
                ActivationState::SignedOut {
                    reason: "Signed out".into(),
                    recovery: Vec::new(),
                },
            )
            .expect("activation");
        assert!(
            window
                .connected_tabs()
                .iter()
                .all(|(id, _, _)| *id != SECOND_PROVIDER),
            "a signed-out provider loses its tab"
        );
        assert_eq!(
            window.focused_provider_tab(),
            None,
            "and the rail falls back to the roster with no intent"
        );
    }

    #[test]
    fn provider_profile_save_crosses_the_typed_host_boundary() {
        let mut window = window();
        let draft = ProviderProfileDraft::for_kind(ProviderConfigurationKind::Local);
        assert_eq!(
            window
                .apply(ChatWindowIntent::SaveProviderProfile(draft.clone()))
                .expect("profile intent"),
            Some(HostIntent::SaveProviderProfile(draft))
        );
    }

    #[test]
    fn protocol_activity_and_unknowns_are_preserved_but_not_conversation_rows() {
        let session = ChatSessionId(2);
        let events = [
            ChatEvent {
                id: ChatEventId(1),
                provider_id: PROVIDER,
                session_id: session,
                turn_id: None,
                content: ChatEventContent::StatusUpdated("The".into()),
            },
            ChatEvent {
                id: ChatEventId(2),
                provider_id: PROVIDER,
                session_id: session,
                turn_id: None,
                content: ChatEventContent::Unknown(dpl_chat_core::BoundedNativePayload::redacted(
                    "acp.available-commands",
                    "AvailableCommandsUpdate(...)",
                )),
            },
            ChatEvent {
                id: ChatEventId(3),
                provider_id: PROVIDER,
                session_id: session,
                turn_id: None,
                content: ChatEventContent::AssistantMessage("Yes — I’m here and working.".into()),
            },
        ];
        let visible = events
            .iter()
            .filter(|event| is_conversation_event(event, Some(session)))
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 3, "diagnostic facts remain preserved");
        assert_eq!(visible.len(), 1);
        assert!(matches!(
            visible[0].content,
            ChatEventContent::AssistantMessage(_)
        ));
    }

    #[test]
    fn provider_selection_and_authorization_copy_cross_the_typed_host_boundary() {
        let mut window = multi_provider_window();
        assert_eq!(
            window
                .apply(ChatWindowIntent::SelectProvider(SECOND_PROVIDER))
                .expect("select provider"),
            Some(HostIntent::ProviderSelected(SECOND_PROVIDER))
        );
        assert_eq!(
            window
                .apply(ChatWindowIntent::CopyProviderAuthorizationCode(
                    SECOND_PROVIDER
                ))
                .expect("copy authorization code"),
            Some(HostIntent::CopyProviderAuthorizationCode(SECOND_PROVIDER))
        );
    }

    #[test]
    fn provider_trace_is_bounded_and_retains_the_latest_facts() {
        let mut window = window();
        for seq in 1..=130 {
            window.observe_provider_fact(Stamped {
                seq,
                ms: seq,
                ev: ProviderEvent::Selected { provider: PROVIDER },
            });
        }
        assert_eq!(window.provider_trace.len(), 128);
        assert_eq!(window.provider_trace.front().map(|fact| fact.seq), Some(3));
        assert_eq!(window.provider_trace.back().map(|fact| fact.seq), Some(130));
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

    /// Assist is host-owned truth: the chip's tap crosses the typed
    /// boundary carrying the DESIRED state, and the window's own flag
    /// moves only when the host projects it back.
    #[test]
    fn assist_toggle_crosses_the_typed_host_boundary() {
        let mut window = window();
        assert!(!window.assist_enabled(), "assist defaults off");
        assert_eq!(
            window
                .apply(ChatWindowIntent::ToggleAssist { enable: true })
                .expect("toggle assist"),
            Some(HostIntent::ToggleAssist { enable: true })
        );
        assert!(
            !window.assist_enabled(),
            "the window never flips itself — consent may stand between"
        );
        window.set_assist(true);
        assert!(window.assist_enabled());
        assert_eq!(
            window
                .apply(ChatWindowIntent::ToggleAssist { enable: false })
                .expect("toggle assist off"),
            Some(HostIntent::ToggleAssist { enable: false })
        );
    }

    /// The blanket acknowledgement: registered once, TWO buttons only
    /// (no "always" — approving IS the durable answer), copy names that
    /// it covers all providers, and both resolutions cross the typed
    /// boundary and clear the overlay.
    #[test]
    fn assist_acknowledgement_is_one_blanket_consent() {
        let mut window = window();
        let interaction = InteractionId(2_000_000_000);
        window
            .register_assist_acknowledgement(interaction)
            .expect("register acknowledgement");
        let spec = window.interaction(interaction).expect("registered spec");
        let InteractionContent::Approval {
            summary,
            always_approve_label,
            ..
        } = &spec.content
        else {
            panic!("acknowledgement must be an Approval");
        };
        assert!(
            always_approve_label.is_none(),
            "two buttons only — no separate 'always'"
        );
        assert!(
            summary.contains("all chat providers"),
            "the copy states the blanket: {summary}"
        );
        assert_eq!(window.overlay_coordinator.ordered().len(), 1);
        assert_eq!(
            window
                .apply(ChatWindowIntent::ResolveAssistAcknowledgement {
                    interaction,
                    accepted: true,
                })
                .expect("resolve acknowledgement"),
            Some(HostIntent::ResolveAssistAcknowledgement { accepted: true })
        );
        assert_eq!(window.overlay_coordinator.ordered().len(), 0);
    }

    /// Declining the acknowledgement forwards `accepted: false` — the
    /// host keeps assist off and persists nothing.
    #[test]
    fn assist_acknowledgement_decline_stays_off() {
        let mut window = window();
        let interaction = InteractionId(2_000_000_001);
        window
            .register_assist_acknowledgement(interaction)
            .expect("register acknowledgement");
        assert_eq!(
            window
                .apply(ChatWindowIntent::ResolveAssistAcknowledgement {
                    interaction,
                    accepted: false,
                })
                .expect("decline acknowledgement"),
            Some(HostIntent::ResolveAssistAcknowledgement { accepted: false })
        );
        assert!(!window.assist_enabled());
        assert_eq!(window.overlay_coordinator.ordered().len(), 0);
    }

    #[test]
    fn dismissing_an_approval_queues_its_real_focus_origin() {
        let fixture = dpl_chat_testkit::authored_chat();
        let mut window = ChatWindow::with_models(
            ChatRuntime::try_from_registrations(fixture.providers).expect("runtime"),
            ROUTE,
            &HistoryCatalog::default(),
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
            &HistoryCatalog::default(),
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

    // ---- H1: local transcripts become rows -------------------------------
    //
    // These pins build a REAL catalog over a real temp directory and let
    // `ChatWindow::new` run its real indexing loop, so the fixture proves the
    // same path the operator's 165 transcripts take — not an injected map.

    struct HistoryTree {
        root: std::path::PathBuf,
    }

    impl HistoryTree {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT: AtomicU64 = AtomicU64::new(1);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("devpulse-chat-window-{}-{id}", std::process::id()));
            std::fs::create_dir_all(&root).expect("create history tree");
            Self { root }
        }

        fn write(&self, name: &str, body: &str) {
            std::fs::write(self.root.join(name), body).expect("write transcript fixture");
        }
    }

    impl Drop for HistoryTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    struct FixtureHistoryDecoder;

    impl dpl_chat_store::HistoryDecoder for FixtureHistoryDecoder {
        fn provider_id(&self) -> ProviderId {
            PROVIDER
        }

        fn index_metadata(
            &self,
            path: &std::path::Path,
            _prefix: &str,
        ) -> Option<dpl_chat_store::HistoryIndexMetadata> {
            let stem = path.file_stem()?.to_string_lossy().into_owned();
            Some(dpl_chat_store::HistoryIndexMetadata {
                native_id: dpl_chat_core::ProviderSessionId(stem.clone()),
                title: format!("Transcript {stem}"),
                workspace: "DevPULSE".to_owned(),
                started_at: "2026-07-30T00:00:00Z".to_owned(),
            })
        }

        fn decode_line(&self, line: &str) -> dpl_chat_store::DecodeOutcome {
            dpl_chat_store::DecodeOutcome::Event(dpl_chat_store::DecodedHistoryEvent {
                turn_id: None,
                content: ChatEventContent::AssistantMessage(line.to_owned()),
            })
        }
    }

    fn history_catalog(tree: &HistoryTree) -> HistoryCatalog {
        let mut catalog = HistoryCatalog::default();
        let source = dpl_chat_store::JsonlHistorySource::open(
            &tree.root,
            FixtureHistoryDecoder,
            dpl_chat_store::HistoryLimits::default(),
        )
        .expect("open fixture history");
        catalog.register(source).expect("register fixture history");
        catalog
    }

    /// A provider with NO live sessions — exactly the shape that rendered a
    /// count and nothing else before H1.
    fn history_window(catalog: &HistoryCatalog) -> ChatWindow {
        let mut registration = ProviderRegistration::planned(MANIFEST);
        registration.activation = ActivationState::Ready;
        ChatWindow::new(
            ChatRuntime::try_from_registrations([registration]).expect("runtime"),
            ROUTE,
            catalog,
            vec![("Library".into(), ROUTE)],
        )
    }

    #[test]
    fn history_sessions_become_rows() {
        let tree = HistoryTree::new();
        tree.write("alpha.jsonl", "first\nsecond\n");
        tree.write("beta.jsonl", "only\n");
        let catalog = history_catalog(&tree);
        let window = history_window(&catalog);

        let providers = window.runtime.provider_views();
        assert!(
            providers
                .iter()
                .all(|provider| provider.registration.sessions.is_empty()),
            "the fixture provider has no LIVE sessions — that is the point"
        );
        let rows = window.history_rows(&providers);
        assert_eq!(
            rows.len(),
            2,
            "both transcripts on disk must become rows, not a count"
        );
    }

    #[test]
    fn history_rows_sort_most_recent_first() {
        let tree = HistoryTree::new();
        tree.write("older.jsonl", "old\n");
        // Distinct mtimes: the sort key is the filesystem's, and two writes in
        // the same instant would make the assertion vacuous.
        std::thread::sleep(std::time::Duration::from_millis(20));
        tree.write("newer.jsonl", "new\n");
        let catalog = history_catalog(&tree);
        let window = history_window(&catalog);

        let providers = window.runtime.provider_views();
        let rows = window.history_rows(&providers);
        assert_eq!(rows.len(), 2);
        assert!(
            rows[0].1.modified_millis >= rows[1].1.modified_millis,
            "the operator is looking for what they were just doing"
        );
        assert!(rows[0].1.summary.title.contains("newer"));
    }

    #[test]
    fn selecting_a_history_row_asks_the_host_for_its_page() {
        let tree = HistoryTree::new();
        tree.write("alpha.jsonl", "first\n");
        let catalog = history_catalog(&tree);
        let mut window = history_window(&catalog);
        let providers = window.runtime.provider_views();
        let locator = window.history_rows(&providers)[0].1.locator;

        let host = window
            .apply(ChatWindowIntent::SelectHistorySession {
                provider: PROVIDER,
                locator,
            })
            .expect("select the transcript");
        assert_eq!(
            host,
            Some(HostIntent::LoadHistoryPage {
                provider: PROVIDER,
                locator
            }),
            "a surface cannot read disk — it must ask the host"
        );
        assert_eq!(window.history_focus, Some((PROVIDER, locator)));
    }

    #[test]
    fn a_history_page_replaces_the_previous_one() {
        let tree = HistoryTree::new();
        tree.write("alpha.jsonl", "first\n");
        tree.write("beta.jsonl", "second\n");
        let catalog = history_catalog(&tree);
        let mut window = history_window(&catalog);
        let providers = window.runtime.provider_views();
        let rows = window.history_rows(&providers);
        let (first, second) = (rows[0].1.locator, rows[1].1.locator);

        window
            .apply(ChatWindowIntent::SelectHistorySession {
                provider: PROVIDER,
                locator: first,
            })
            .expect("select first");
        let page = catalog.page(PROVIDER, first, None).expect("first page");
        window.observe_history_page(page);
        assert!(window.history_page.is_some());

        // Moving to another transcript drops the stale page immediately, so
        // the banner never names one conversation over another's messages.
        window
            .apply(ChatWindowIntent::SelectHistorySession {
                provider: PROVIDER,
                locator: second,
            })
            .expect("select second");
        assert!(window.history_page.is_none());

        // A LATE page for the transcript already navigated away from is
        // discarded — disk reads race the finger.
        let stale = catalog.page(PROVIDER, first, None).expect("stale page");
        window.observe_history_page(stale);
        assert!(
            window.history_page.is_none(),
            "a page for an unfocused transcript must never be drawn"
        );
    }

    #[test]
    fn a_focused_history_session_disables_the_composer() {
        let tree = HistoryTree::new();
        tree.write("alpha.jsonl", "first\n");
        let catalog = history_catalog(&tree);
        let mut window = history_window(&catalog);
        let providers = window.runtime.provider_views();
        let locator = window.history_rows(&providers)[0].1.locator;

        assert_eq!(window.composer_hint(), "Draft for this AI session…");
        window
            .apply(ChatWindowIntent::SelectHistorySession {
                provider: PROVIDER,
                locator,
            })
            .expect("select the transcript");
        assert_eq!(
            window.composer_hint(),
            "Past transcript — read-only",
            "a past transcript is read-only and the composer must say so"
        );
    }

    #[test]
    fn choosing_a_live_session_leaves_the_transcript_reader() {
        let tree = HistoryTree::new();
        tree.write("alpha.jsonl", "first\n");
        let catalog = history_catalog(&tree);
        let mut window = history_window(&catalog);
        let providers = window.runtime.provider_views();
        let locator = window.history_rows(&providers)[0].1.locator;
        window
            .apply(ChatWindowIntent::SelectHistorySession {
                provider: PROVIDER,
                locator,
            })
            .expect("select the transcript");

        window
            .runtime
            .upsert_session(
                PROVIDER,
                SessionSummary {
                    id: ChatSessionId(9),
                    provider_id: PROVIDER,
                    provider_session_id: None,
                    title: "Live again".into(),
                    workspace: "DevPULSE".into(),
                    group: SessionGroup::Active,
                    state: SessionState::Idle,
                },
            )
            .expect("upsert live session");
        window
            .apply(ChatWindowIntent::SelectSession {
                provider: PROVIDER,
                session: ChatSessionId(9),
            })
            .expect("select the live session");
        assert_eq!(window.history_focus, None);
        assert!(window.history_page.is_none());
    }

    #[test]
    fn slash_parse_is_exact_and_whole_buffer() {
        assert_eq!(
            parse_operator_slash("/strict"),
            Some(OperatorSlashCommand::Strict)
        );
        assert_eq!(
            parse_operator_slash("  /behavioral  "),
            Some(OperatorSlashCommand::Behavioral)
        );
        // Buried in a sentence — inert. Pattern-in-string never escalates.
        assert_eq!(parse_operator_slash("please /strict now"), None);
        assert_eq!(parse_operator_slash("/strict please"), None);
        assert_eq!(parse_operator_slash("/stricter"), None);
        assert_eq!(parse_operator_slash("strict"), None);
        assert_eq!(parse_operator_slash(""), None);
    }

    #[test]
    fn slash_commands_name_their_audit_action() {
        assert_eq!(OperatorSlashCommand::Strict.as_str(), "strict");
        assert_eq!(OperatorSlashCommand::Behavioral.as_str(), "behavioral");
    }
}
