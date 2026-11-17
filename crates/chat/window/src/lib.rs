//! Compact primary native Chat-window composition.
//!
//! This crate paints one independent peer viewport and emits typed intents.
//! It does not create viewports, publish signals, navigate Main, scan stores,
//! launch providers, or implement Social transport.

use dpl_ai_core::activation::{AccountSummary, ActivationState, RecoveryActionKind};
use dpl_ai_core::approval::PermissionMode;
use dpl_ai_core::capability::Capability;
use dpl_ai_core::event::{ChatEventContent, EventPresentation};
use dpl_ai_core::provider::{ProviderAccent, ProviderId};
use dpl_ai_core::registration::OnMachine;
use dpl_ai_core::turn::TurnId;
use dpl_ai_profiles::{
    CatalogSupplement, CatalogSupplementAction, ProviderConfigurationKind, ProviderProfile,
    ProviderProfileDraft, CLYFFY_PROVIDER, SUPPLEMENTS,
};
use dpl_chat_context::RouteContextState;
use dpl_chat_core::composer::{
    in_group, Availability, ComposerEntry, ComposerGroup, ComposerModes,
};
use dpl_chat_core::{
    ChatDomain, ChatEvent, ChatEventId, ChatSessionId, ContentEpisode, ContentEpisodeId,
    ContentSpine, ContentView, SessionGroup, SessionState, SessionSummary,
};
use dpl_chat_interactions::{
    InteractionContent, InteractionId, InteractionLifecycle, InteractionPlacement,
    InteractionProvenance, InteractionRegistry, InteractionSpec, OverlayCoordinator, OverlayEntry,
};
use dpl_chat_projects::trust::Grant;
use dpl_chat_runtime::{ChatRuntime, ProviderView, RegistryError};
use dpl_chat_social::{
    Presence, SocialConversationId, SocialModel, SocialModelError, SocialThreadKind,
};
use dpl_chat_store::{HistoryCatalog, HistoryIndex, HistoryLocator, HistoryPage, IndexedSession};
use dpl_coldlight::color_roles::chat;
use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::{AppWindowKey, ProviderCapability, ProviderEvent, ProviderPresence, Stamped};
use dpl_hub::Route;
use egui::{
    Align, Align2, Color32, CornerRadius, Layout, NumExt, Rect, RichText, Sense, Stroke, UiBuilder,
};
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    NodeMetadata, PilotId, PilotKey, PilotPath, UiPilotExt,
};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::path::PathBuf;

mod projects;
pub use projects::{
    ProjectChoice, ProjectView, PROJECTS_ID, PROJECT_COPY_ID, PROJECT_LABEL_ID, PROJECT_OPEN_ID,
    PROJECT_RETRY_ID, PROJECT_SELECT_ID, PROJECT_SETUP_ID, PROJECT_WORKSPACE_ID,
};

pub const WINDOW_KEY: AppWindowKey = AppWindowKey("chat.primary");
pub const RESUME_CHAT_ID: PilotId = PilotId::new("chat.session.resume");
pub const REFRESH_NATIVE_CHATS_ID: PilotId = PilotId::new("chat.sessions.refresh-native");

pub const LAUNCHER_ID: PilotId = PilotId::new("shell.window.chat");
/// The General tab — the AI domain's home, Clyffy's house session. Its id
/// predates the name and stays, so every scenario that drives it still does.
pub const AI_MODE_ID: PilotId = PilotId::new("chat.mode.ai");
/// The More AIs tab: every known AI that has not earned a tab of its own.
pub const MORE_AIS_TAB_ID: PilotId = PilotId::new("chat.mode.more-ais");
/// The More AIs page; each row is `chat.more-ais/<provider>`.
pub const MORE_AIS_ID: PilotId = PilotId::new("chat.more-ais");
/// The trust dialog (AH1b) and its parts: `chat.trust/read|change|run|allow|deny`.
pub const TRUST_ID: PilotId = PilotId::new("chat.trust");
/// The "Trust…" button in an AI's header: `chat.trust.edit/<provider>`.
pub const TRUST_EDIT_ID: PilotId = PilotId::new("chat.trust.edit");
/// The strip shown while an AI is not allowed in the project:
/// `chat.trust.blocked/<provider>`.
pub const TRUST_BLOCKED_ID: PilotId = PilotId::new("chat.trust.blocked");

/// The open trust dialog: which AI, which project, and the switches as the
/// operator has set them so far.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TrustDialog {
    provider: ProviderId,
    project: String,
    draft: Grant,
}
/// The General tab's header: Clyffy's house session and what backs it.
pub const GENERAL_ID: PilotId = PilotId::new("chat.general");
/// One "backed by" choice per signed-in AI: `chat.general.backing/<provider>`.
pub const GENERAL_BACKING_ID: PilotId = PilotId::new("chat.general.backing");
pub const ROOM_LIBRARY_ID: PilotId = PilotId::new("chat.library");

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

/// The composer's keyboard contract, announced on the node itself so an
/// assistive client or an agent reading the tree learns it — a shortcut that
/// only exists in a hint string is folklore, not an affordance.
pub const COMPOSER_KEYS: &str =
    "Enter sends. Shift+Enter inserts a new line. Ctrl+Enter (Cmd+Enter) also sends.";

/// Take an Enter that means SEND, leaving one that means NEWLINE alone.
///
/// The event is scanned by hand rather than through `consume_key`, because
/// `consume_key` deliberately IGNORES shift when matching — so
/// `Modifiers::NONE` happily matches Shift+Enter and would swallow the one
/// keystroke that has to survive. Pinned by
/// `enter_sends_and_shift_enter_makes_a_new_line`, which caught exactly that.
///
/// Consuming is what keeps a send from also inserting a newline, and it must
/// happen BEFORE the `TextEdit` runs — afterwards the newline is already in
/// the buffer. Shift+Enter (and Alt+Enter) are left in the queue and reach
/// the widget as an ordinary new line.
fn take_composer_submit(input: &mut egui::InputState) -> bool {
    let mut submit = false;
    input.events.retain(|event| {
        let egui::Event::Key {
            key: egui::Key::Enter,
            pressed: true,
            modifiers,
            ..
        } = event
        else {
            return true;
        };
        if modifiers.shift || modifiers.alt {
            return true;
        }
        submit = true;
        false
    });
    submit
}
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
pub const BRAINSTORM_PANEL_ID: PilotId = PilotId::new("chat.brainstorm.panel");
pub const BRAINSTORM_PHASE_ID: PilotId = PilotId::new("chat.brainstorm.phase");
pub const BRAINSTORM_SEED_ID: PilotId = PilotId::new("chat.brainstorm.seed");
pub const BRAINSTORM_ADD_ID: PilotId = PilotId::new("chat.brainstorm.add");
pub const BRAINSTORM_ITEM_ID: PilotId = PilotId::new("chat.brainstorm.item");
pub const BRAINSTORM_CLUSTER_ID: PilotId = PilotId::new("chat.brainstorm.cluster");
pub const BRAINSTORM_ADVANCE_ID: PilotId = PilotId::new("chat.brainstorm.advance");
pub const BRAINSTORM_PROMOTE_ID: PilotId = PilotId::new("chat.brainstorm.promote");
pub const BRAINSTORM_UNDO_ID: PilotId = PilotId::new("chat.brainstorm.undo");
pub const BRAINSTORM_REDO_ID: PilotId = PilotId::new("chat.brainstorm.redo");
pub const BRAINSTORM_SUGGEST_ID: PilotId = PilotId::new("chat.brainstorm.suggest");
pub const COMPOSER_STATUS_ID: PilotId = PilotId::new("chat.composer.status");
pub const COMMAND_STRIP_ID: PilotId = PilotId::new("chat.composer.command-strip");
pub const CLYFFY_ROW_ID: PilotId = PilotId::new("chat.clyffy.session");
/// The composer `+`: one popup holding the census's menu entries.
pub const COMPOSER_MORE_ID: PilotId = PilotId::new("chat.composer.more");
/// The `+` popup itself.
pub const COMPOSER_MENU_ID: PilotId = PilotId::new("chat.composer.menu");
/// One mode chip on the composer: `chat.composer.mode/<slug>`.
pub const MODE_ID: PilotId = PilotId::new("chat.composer.mode");
/// The Send (and Stop) button's size: a touch target in whole space steps.
const SEND_SIZE: egui::Vec2 = egui::vec2(
    cl::space::S6 * 2.0 + cl::space::S5,
    cl::space::S6 + cl::space::S3,
);
/// The "More" chip that holds the modes the row had no room for.
pub const MODES_MORE_ID: PilotId = PilotId::new("chat.composer.modes-more");
/// The popup it opens.
pub const MODES_MENU_ID: PilotId = PilotId::new("chat.composer.modes-menu");
/// The overflow chip's word.
const MODES_MORE_LABEL: &str = "More";
/// The composer's way into the provider catalog — the second trigger beside
/// the rail's AI Providers control.
pub const ADD_PROVIDER_ID: PilotId = PilotId::new("chat.composer.add-provider");
/// The catalog's own close control.
pub const PROVIDER_CATALOG_CLOSE_ID: PilotId = PilotId::new("chat.provider.catalog.close");

/// A subscription tile. Few of them, so each is large enough to carry a
/// mark, a name, a tenure date and a live presence line. Sized in whole
/// chrome grid steps.
const SUB_TILE: egui::Vec2 = egui::vec2(cl::space::S6 * 5.0, cl::space::S6 * 4.0);
/// An API tile. Many of them, so compact: a name and a kind.
const API_TILE: egui::Vec2 = egui::vec2(cl::space::S6 * 5.0, cl::space::S6 * 2.0);
/// The FLOOR, not a cap: the API grid never collapses below this many rows
/// once it has a tile, and takes more as profiles are added.
const API_ROWS_FLOOR: usize = 2;
/// The bleed seed. A constant rather than a clock read: every proof shot of
/// the rail must show the same two hues, or the capture is unreviewable.
const BLEED_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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

/// Which tab the AI domain is showing.
///
/// The strip reads, left to right: General, one tab per AI that is installed
/// and signed in, then More AIs. A provider tab is earned by activation and
/// nothing else, so signing out takes the tab away without an intent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AiTab {
    /// Clyffy's house session, backed by the AI the operator assigned.
    #[default]
    General,
    /// One AI that is installed and signed in.
    Provider(ProviderId),
    /// Every other known AI, its state, and one tap into its setup.
    MoreAis,
}

/// What an AI on the More AIs page needs before it can earn a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoreAiState {
    /// Installed; it needs the operator to sign in.
    SignIn,
    /// A desktop app the scan found: DevPULSE can open it (or bring it to
    /// the front).
    Open,
    /// There is a way to install (or update) it from here.
    Install,
    /// Nothing on this computer can run it yet.
    NotOnThisComputer,
}

impl MoreAiState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SignIn => "Sign in",
            Self::Open => "Open app",
            Self::Install => "Install",
            Self::NotOnThisComputer => "Not on this computer",
        }
    }

    /// The order More AIs lists its rows in: what the operator can act on
    /// first, then what needs an install, then what this computer cannot run.
    const fn rank(self) -> u8 {
        match self {
            Self::SignIn => 0,
            Self::Open => 1,
            Self::Install => 2,
            Self::NotOnThisComputer => 3,
        }
    }

    /// The state an AI that has not earned a tab is in. Read from its
    /// activation and the first way back it offers, so the page never
    /// promises a step the setup flow cannot take.
    pub fn of(activation: &ActivationState) -> Self {
        let first = activation.recovery().first().map(|action| action.kind);
        match activation {
            ActivationState::SignedOut { .. }
            | ActivationState::SigningIn
            | ActivationState::AwaitingAuthorization(_)
            | ActivationState::Verifying => Self::SignIn,
            ActivationState::NotInstalled { .. } | ActivationState::VersionUnsupported { .. } => {
                if first.is_some() {
                    Self::Install
                } else {
                    Self::NotOnThisComputer
                }
            }
            _ => match first {
                Some(RecoveryActionKind::SignIn | RecoveryActionKind::Retry) => Self::SignIn,
                Some(RecoveryActionKind::OpenApp) => Self::Open,
                Some(
                    RecoveryActionKind::InstallInstructions | RecoveryActionKind::UpdateProvider,
                ) => Self::Install,
                _ => Self::NotOnThisComputer,
            },
        }
    }
}

/// One row of the More AIs page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoreAiRow {
    pub provider: ProviderId,
    pub label: &'static str,
    pub state: MoreAiState,
    /// The provider's own words for why it is not ready, when it gave any.
    pub reason: Option<String>,
    /// What the scan of this computer saw ("On this computer · running now"),
    /// once it has answered.
    pub on_machine: Option<&'static str>,
}

/// What the machine scan saw, in plain words. `None` when it saw nothing
/// worth saying, so a row never claims a fact the scan did not find.
pub fn on_machine_words(on_machine: Option<OnMachine>) -> Option<&'static str> {
    match on_machine? {
        OnMachine {
            installed: true,
            running: true,
        } => Some("On this computer · running now"),
        OnMachine {
            installed: true,
            running: false,
        } => Some("On this computer"),
        OnMachine {
            installed: false,
            running: true,
        } => Some("Running now"),
        OnMachine {
            installed: false,
            running: false,
        } => None,
    }
}

/// The tab model, pure: the ready AIs that earn a tab (registration order,
/// so the strip never reshuffles under a finger) and every other AI, which
/// More AIs lists grouped by what the operator can do (catalog order within
/// a group).
pub fn tab_model(providers: &[ProviderView]) -> (Vec<ProviderId>, Vec<MoreAiRow>) {
    let mut ready = Vec::new();
    let mut more = Vec::new();
    for provider in providers {
        let descriptor = provider.registration.manifest.descriptor;
        let activation = &provider.registration.activation;
        if activation.is_activated() {
            ready.push(descriptor.id);
        } else {
            more.push(MoreAiRow {
                provider: descriptor.id,
                label: descriptor.label,
                state: MoreAiState::of(activation),
                reason: activation.reason().map(str::to_owned),
                on_machine: on_machine_words(provider.registration.on_machine),
            });
        }
    }
    more.sort_by_key(|row| row.state.rank());
    (ready, more)
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
    SaveProjectCopy {
        workspace: PathBuf,
        label: String,
    },
    SetupProject {
        workspace: PathBuf,
        label: String,
    },
    OpenProject {
        workspace: PathBuf,
    },
    SelectProject(dpl_chat_projects::ProjectId),
    RetryProjectSave,
    SelectDomain(ChatDomain),
    SelectProvider(ProviderId),
    /// Scope the AI domain to one connected provider — the tab beside
    /// AI/Social that a provider earns once it is activated.
    SelectProviderTab(ProviderId),
    /// Open the More AIs tab: every AI without a tab of its own.
    SelectMoreAis,
    /// The operator chose which signed-in AI backs General (Clyffy).
    AssignGeneralBacking(ProviderId),
    /// Turn one composer mode on or off (a census slug).
    ToggleMode(&'static str),
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
    /// Open a desktop app the machine scan found.
    OpenProviderApp(ProviderId),
    RefreshProvider(ProviderId),
    CopyProviderAuthorizationCode(ProviderId),
    SetProviderTraceExpanded {
        provider: ProviderId,
        expanded: bool,
    },
    NewSession(ProviderId),
    LoadSession {
        provider: ProviderId,
        session: ChatSessionId,
    },
    RefreshNativeSessions(ProviderId),
    SendTurn {
        provider: ProviderId,
        session: ChatSessionId,
        text: String,
        /// The modes standing when the turn was sent. They ride the send so
        /// a host can tell an AI that takes them; one that does not is sent
        /// exactly what it was sent before.
        modes: ComposerModes,
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
    /// The operator chose what a provider may do from chat.
    SetPermissionMode {
        provider: ProviderId,
        mode: PermissionMode,
    },
    /// Open the trust dialog for an AI in the active project (AH1b).
    EditTrust(ProviderId),
    /// The operator answered the trust dialog: a grant (all off is Don't
    /// allow), or `None` when the dialog was closed without an answer.
    AnswerTrust {
        provider: ProviderId,
        grant: Option<Grant>,
    },
    /// A switch in the open trust dialog changed.
    SetTrustDraft(Grant),
    /// Show or hide the Rooms & roles library.
    ToggleRoomLibrary,
    /// Open the folder where the operator's own roles and rooms live.
    OpenRoomLibraryFolder,
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
    /// The whole-buffer brainstorm trigger fired at submit — mint a
    /// fresh board (host-side; the window renders what comes back).
    StartBrainstorm,
    /// One typed board verb from the surface — an operator gesture
    /// speaking the same vocabulary the assist will (D5 parity).
    Brainstorm(dpl_workflows::BrainstormOp),
    BrainstormUndo,
    BrainstormRedo,
    /// Ask the assist for candidate ideas against the seed (assist ON
    /// only — the host guards; proposals arrive as Assist-origin items).
    BrainstormAssist,
    /// Distill the converged board into the explainer intake pre-fill.
    PromoteBrainstorm,
    /// The operator tapped the pinned Clyffy row.
    SelectClyffy,
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
    /// A non-privileged census command recognized at submit (BND1).
    /// `synthetic` rides every command — the whole census refuses
    /// model-minted input, not just the determinism pair.
    Command {
        draft: dpl_chat_core::CommandDraft,
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

/// Recognize a command from the submit buffer — ONE recognizer, the
/// registry census (`dpl_chat_core::recognize`; BND1 law: the two hand
/// parsers this replaced are gone, not wrapped). The privileged
/// determinism pair maps onto [`OperatorSlashCommand`] here — the ONE
/// place a slug becomes the audited wire vocabulary.
pub fn recognize_command(draft: &str) -> Option<dpl_chat_core::CommandDraft> {
    dpl_chat_core::recognize(draft)
}

/// The slug→audited-command mapping for the privileged pair. EXACT and
/// argument-less — `/strict please` is not the audited command (it
/// falls through to the census refusal, which says so honestly).
/// Everything else dispatches as a plain [`ChatWindowIntent::Command`].
pub fn privileged_command(draft: &dpl_chat_core::CommandDraft) -> Option<OperatorSlashCommand> {
    if !draft.rest.trim().is_empty() {
        return None;
    }
    match draft.slug.as_str() {
        "strict" => Some(OperatorSlashCommand::Strict),
        "behavioral" => Some(OperatorSlashCommand::Behavioral),
        _ => None,
    }
}

/// The next free `idea-N` id on a board — deterministic, collision-free
/// against every existing item (removed ids may be reused; the board is
/// the census, not a ledger).
fn next_brainstorm_id(doc: &dpl_workflows::BrainstormDoc) -> String {
    (1..)
        .map(|n| format!("idea-{n}"))
        .find(|candidate| doc.items.iter().all(|i| &i.id != candidate))
        .expect("the counter is unbounded")
}

/// A kebab id minted from an operator-typed label ("The Opening" →
/// "the-opening") — non-alphanumerics collapse to single dashes.
fn kebab_of(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut dash_pending = false;
    for c in label.chars() {
        if c.is_ascii_alphanumeric() {
            if dash_pending && !out.is_empty() {
                out.push('-');
            }
            dash_pending = false;
            out.push(c.to_ascii_lowercase());
        } else {
            dash_pending = true;
        }
    }
    if out.is_empty() {
        "group".to_owned()
    } else {
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostIntent {
    SaveProjectCopy {
        workspace: PathBuf,
        label: String,
    },
    SetupProject {
        workspace: PathBuf,
        label: String,
    },
    OpenProject {
        workspace: PathBuf,
    },
    SelectProject(dpl_chat_projects::ProjectId),
    RetryProjectSave,
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
    OpenProviderApp(ProviderId),
    RefreshProvider(ProviderId),
    ProviderSelected(ProviderId),
    SaveProviderProfile(ProviderProfileDraft),
    CopyProviderAuthorizationCode(ProviderId),
    NewSession(ProviderId),
    LoadSession {
        provider: ProviderId,
        session: ChatSessionId,
    },
    RefreshNativeSessions(ProviderId),
    SendTurn {
        provider: ProviderId,
        session: ChatSessionId,
        text: String,
        /// See [`ChatWindowIntent::SendTurn`]: carried, never required.
        modes: ComposerModes,
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
    /// Save and apply what a provider may do from chat.
    SetPermissionMode {
        provider: ProviderId,
        mode: PermissionMode,
    },
    /// Ask the operator what an AI may do in the active project (AH1b).
    EditTrust(ProviderId),
    /// Save the operator's answer for an AI in the active project.
    AnswerTrust {
        provider: ProviderId,
        grant: Option<Grant>,
    },
    /// Open (creating it if needed) the operator's role and room folder.
    OpenRoomLibraryFolder,
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
    /// Mint a fresh brainstorm board in the inbox and project it back.
    StartBrainstorm,
    /// Apply one typed board verb through the ONE document write path.
    Brainstorm(dpl_workflows::BrainstormOp),
    BrainstormUndo,
    BrainstormRedo,
    /// Spawn the one-shot suggestion job (the imagine drive shape).
    BrainstormAssist,
    /// Distill the converged board into the explainer intake pre-fill.
    PromoteBrainstorm,
    /// The operator tapped the pinned Clyffy row.
    SelectClyffy,
    /// Back General (Clyffy) with this signed-in AI and save the choice in
    /// chat settings.
    AssignGeneralBacking(ProviderId),
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
    /// A census command for the host to validate (against the live
    /// censuses it owns) and dispatch.
    Command {
        draft: dpl_chat_core::CommandDraft,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeSessionStatus {
    pub qualified: bool,
    pub loading: Option<ChatSessionId>,
    pub listing: bool,
}

#[derive(Debug, Clone)]
pub struct ChatWindow {
    runtime: ChatRuntime,
    project_view: ProjectView,
    project_ui: projects::ProjectUi,
    project_input_locked: bool,
    native_sessions: BTreeMap<ProviderId, NativeSessionStatus>,
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
    /// Providers offered a permission selector, with their current mode.
    permission_modes: BTreeMap<ProviderId, PermissionMode>,
    /// What each AI is trusted with in the active project (AH1b): `None`
    /// means asked and not allowed; a missing key means trust does not
    /// apply to it (no project, or not an AI that works in one).
    trust: BTreeMap<ProviderId, Option<PermissionMode>>,
    /// The open trust dialog, if any.
    trust_dialog: Option<TrustDialog>,
    /// The roles and rooms the operator can use (built-ins plus their own).
    room_library: dpl_chat_core::rooms::Library,
    /// Is the Rooms & roles library showing in the AI pane?
    library_open: bool,
    /// The library's filters: one room type, one stance, or everything.
    library_room_filter: Option<dpl_chat_core::rooms::RoomType>,
    library_role_filter: Option<dpl_chat_core::rooms::Stance>,
    provider_trace: VecDeque<Stamped<ProviderEvent>>,
    provider_trace_expanded: HashMap<ProviderId, bool>,
    composer_focus_requested: bool,
    domain: ChatDomain,
    /// Which AI tab is showing: General (the roster and Clyffy's house
    /// session), one connected provider, or More AIs.
    ai_tab: AiTab,
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
    /// The brainstorm board as the HOST projects it (the ContentView
    /// shape: the window renders and emits ops, the host owns the file
    /// and the undo stacks).
    brainstorm: BrainstormView,
    /// In-flight text the operator is typing on the board — window-local
    /// so a keystroke is not an op; commit happens on focus loss or the
    /// explicit button, and only committed text reaches [`apply`].
    brainstorm_seed_draft: String,
    brainstorm_add_draft: String,
    brainstorm_cluster_draft: String,
    brainstorm_item_drafts: HashMap<String, String>,
    /// The Enter-honesty line (BND1): the refusal reason a consumed
    /// keyboard submit paints under the composer. Cleared by the next
    /// keystroke or a successful submit.
    composer_status: Option<String>,
    /// The Clyffy house session as the HOST projects it.
    clyffy: ClyffyView,
    /// The modes standing on the composer, ridden by the next send.
    composer_modes: ComposerModes,
    /// Is the composer `+` popup open?
    composer_menu_open: bool,
    /// Is the popup of modes that did not fit the row open?
    mode_menu_open: bool,
    /// The AI Providers control's seeded hue pair and dwell. Rolled ONCE, so
    /// a capture reproduces it and a repaint does not strobe.
    bleed: cl::patterns::BleedRoll,
}

/// The host's projection of the Clyffy house session (BND1): DevPULSE's
/// own conversation, pinned above the providers. `active` derives from
/// presence (any signed-in backing); `backing` is a CURRENT BINDING,
/// never part of the session's identity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClyffyView {
    pub present: bool,
    pub active: bool,
    pub backing: Option<ProviderId>,
    pub events: usize,
}

/// The host's projection of the brainstorm session into the window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BrainstormView {
    /// The board, verbatim — the surface paints THIS document.
    pub doc: Option<dpl_workflows::BrainstormDoc>,
    /// An honest status line (a refused op's issues, a promote verdict).
    pub status: Option<String>,
    pub can_undo: bool,
    pub can_redo: bool,
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
            project_view: ProjectView::default(),
            project_ui: projects::ProjectUi::default(),
            project_input_locked: false,
            native_sessions: BTreeMap::new(),
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
            permission_modes: BTreeMap::new(),
            trust: BTreeMap::new(),
            trust_dialog: None,
            room_library: dpl_chat_core::rooms::Library::builtin(),
            library_open: false,
            library_room_filter: None,
            library_role_filter: None,
            provider_trace: VecDeque::new(),
            provider_trace_expanded: HashMap::new(),
            composer_focus_requested: false,
            domain: ChatDomain::Ai,
            ai_tab: AiTab::General,
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
            brainstorm: BrainstormView::default(),
            brainstorm_seed_draft: String::new(),
            brainstorm_add_draft: String::new(),
            brainstorm_cluster_draft: String::new(),
            brainstorm_item_drafts: HashMap::new(),
            composer_status: None,
            clyffy: ClyffyView::default(),
            composer_modes: ComposerModes::default(),
            composer_menu_open: false,
            mode_menu_open: false,
            bleed: cl::patterns::BleedRoll::from_seed(BLEED_SEED),
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

    /// The host pushes the brainstorm session's current truth — the
    /// board after an op landed (or was refused), with undo reach.
    /// The window renders exactly this; it never edits the document.
    pub fn observe_brainstorm(&mut self, view: BrainstormView) {
        self.brainstorm = view;
    }

    /// Announce a completed promote as a governed inline artifact card:
    /// the staged intake pre-fill is now a durable transcript fact, not
    /// just a status line that the next op overwrites.
    pub fn register_brainstorm_promoted(
        &mut self,
        interaction: InteractionId,
        answers: usize,
        dir: &str,
    ) -> Result<(), ChatWindowError> {
        self.interactions
            .register(InteractionSpec {
                id: interaction,
                provenance: InteractionProvenance::Host,
                required_capability: None,
                lifecycle: InteractionLifecycle::Visible,
                placement: InteractionPlacement::Inline,
                content: InteractionContent::ArtifactPreview {
                    title: "Intake pre-fill staged".to_owned(),
                    artifact_kind: "intake-prefill".to_owned(),
                    summary: format!("{answers} answers ready for the guided intake · {dir}"),
                },
            })
            .map_err(|error| ChatWindowError::Interaction(error.to_string()))
    }

    /// Read-only view of the projected board (tests + host assertions).
    pub fn brainstorm(&self) -> &BrainstormView {
        &self.brainstorm
    }

    /// The host paints an honest line under the composer (a command
    /// refusal, a named skip) — same surface the Enter-honesty law uses.
    pub fn set_composer_status(&mut self, status: impl Into<String>) {
        self.composer_status = Some(status.into());
    }

    /// The host pushes the Clyffy house session's truth — presence,
    /// backing, transcript reach. The window renders the pinned row.
    pub fn observe_clyffy(&mut self, view: ClyffyView) {
        self.clyffy = view;
    }

    /// Read-only view of the projected house session (tests).
    pub fn clyffy(&self) -> &ClyffyView {
        &self.clyffy
    }

    /// Read-only view of the composer status line (tests).
    pub fn composer_status(&self) -> Option<&str> {
        self.composer_status.as_deref()
    }

    /// The modes standing on the composer right now.
    pub fn composer_modes(&self) -> &ComposerModes {
        &self.composer_modes
    }

    /// Which AI tab is showing.
    pub fn ai_tab(&self) -> AiTab {
        self.ai_tab
    }

    /// What the AI the composer speaks to advertises — the census reads this
    /// to decide which modes and `+` entries it can offer.
    fn advertised(&self) -> (Option<&'static str>, Vec<Capability>) {
        match self.runtime.selected_provider() {
            Some(provider) => (
                Some(provider.registration.manifest.descriptor.short_label),
                provider
                    .registration
                    .qualified_capabilities
                    .iter()
                    .copied()
                    .collect(),
            ),
            None => (None, Vec::new()),
        }
    }

    /// Drop every standing mode the current AI cannot honour, and say so.
    /// Called whenever the AI behind the composer may have changed.
    fn retain_offered_modes(&mut self) {
        let (_, advertised) = self.advertised();
        let dropped = self.composer_modes.retain_offered(&advertised);
        if !dropped.is_empty() {
            let names: Vec<_> = dropped.iter().map(|entry| entry.label).collect();
            self.composer_status = Some(format!(
                "{} turned off — this AI doesn't offer it.",
                names.join(", ")
            ));
        }
    }

    /// The AI backing General, when it is signed in right now.
    fn general_backing(&self) -> Option<ProviderId> {
        let backing = self.clyffy.backing?;
        self.connected_tabs()
            .into_iter()
            .any(|(id, _, _)| id == backing)
            .then_some(backing)
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

    /// What every registered provider can do, as plain data the platform can
    /// hold. The host publishes this onto the fabric's roster.
    ///
    /// This is the same shape D1 established with `content_episode_ids` — an
    /// owned projection, not a handle. It does not leak `ChatRuntime` (which
    /// stays a private field of this SURFACE) and it does no IO: it reads a
    /// registry that is already in memory.
    ///
    /// `ActivationState`'s fourteen shades flatten here, deliberately: a
    /// platform consumer needs the FACT (activated, and what's qualified),
    /// not the chat lifecycle's state machine.
    pub fn provider_capabilities(&self) -> Vec<ProviderCapability> {
        self.runtime
            .provider_views()
            .into_iter()
            .map(|view| {
                let registration = &view.registration;
                ProviderCapability {
                    id: registration.manifest.descriptor.id,
                    label: registration.manifest.descriptor.label.to_string(),
                    activated: registration.activation.is_activated(),
                    note: registration.activation.reason().map(str::to_owned),
                    capabilities: registration.qualified_capabilities.clone(),
                    summary: registration.manifest.descriptor.summary.to_owned(),
                    since: registration.manifest.descriptor.since.to_owned(),
                    // The lifecycle collapses to the states the Providers
                    // surface may show — and only those.
                    presence: match &registration.activation {
                        ActivationState::Ready
                        | ActivationState::Experimental { .. }
                        | ActivationState::Degraded { .. } => ProviderPresence::Ready,
                        ActivationState::SignedOut { .. } => ProviderPresence::SignedOut,
                        ActivationState::NotInstalled { .. } => ProviderPresence::NotInstalled,
                        ActivationState::SigningIn
                        | ActivationState::AwaitingAuthorization(_)
                        | ActivationState::Verifying => ProviderPresence::SigningIn,
                        ActivationState::VersionUnsupported { .. }
                        | ActivationState::Blocked { .. }
                        | ActivationState::Failed { .. }
                        | ActivationState::Disabled { .. } => ProviderPresence::Unavailable,
                    },
                }
            })
            .collect()
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
            ChatWindowIntent::SaveProjectCopy { workspace, label } => {
                Ok(Some(HostIntent::SaveProjectCopy { workspace, label }))
            }
            ChatWindowIntent::SetupProject { workspace, label } => {
                Ok(Some(HostIntent::SetupProject { workspace, label }))
            }
            ChatWindowIntent::OpenProject { workspace } => {
                Ok(Some(HostIntent::OpenProject { workspace }))
            }
            ChatWindowIntent::SelectProject(id) => Ok(Some(HostIntent::SelectProject(id))),
            ChatWindowIntent::RetryProjectSave => Ok(Some(HostIntent::RetryProjectSave)),
            ChatWindowIntent::SelectDomain(domain) => {
                self.domain = domain;
                self.ai_tab = AiTab::General;
                // General speaks through the AI that backs it, when that AI
                // is signed in; otherwise the selection stays where it was.
                if domain == ChatDomain::Ai {
                    if let Some(backing) = self.general_backing() {
                        if self.runtime.selected_provider_id() != Some(backing) {
                            self.runtime.select_provider(backing)?;
                            self.retain_offered_modes();
                            return Ok(Some(HostIntent::ProviderSelected(backing)));
                        }
                    }
                }
                Ok(None)
            }
            ChatWindowIntent::SelectProvider(provider) => {
                self.runtime.select_provider(provider)?;
                self.retain_offered_modes();
                Ok(Some(HostIntent::ProviderSelected(provider)))
            }
            ChatWindowIntent::SelectProviderTab(provider) => {
                // The runtime call leads so a rejected provider leaves the
                // tab focus exactly where it was.
                self.runtime.select_provider(provider)?;
                self.domain = ChatDomain::Ai;
                self.ai_tab = AiTab::Provider(provider);
                self.retain_offered_modes();
                Ok(Some(HostIntent::ProviderSelected(provider)))
            }
            ChatWindowIntent::SelectMoreAis => {
                self.domain = ChatDomain::Ai;
                self.ai_tab = AiTab::MoreAis;
                Ok(None)
            }
            ChatWindowIntent::AssignGeneralBacking(provider) => {
                // Only a signed-in AI can back General; the runtime call
                // leads so a rejected id changes nothing.
                if !self
                    .connected_tabs()
                    .into_iter()
                    .any(|(id, _, _)| id == provider)
                {
                    return Ok(None);
                }
                self.runtime.select_provider(provider)?;
                self.clyffy.backing = Some(provider);
                self.retain_offered_modes();
                Ok(Some(HostIntent::AssignGeneralBacking(provider)))
            }
            ChatWindowIntent::ToggleMode(slug) => {
                // A mode is the composer's own state: it flips here and the
                // NEXT send carries it. One the AI cannot honour is refused.
                let (_, advertised) = self.advertised();
                let offered = dpl_chat_core::composer::resolve(slug)
                    .is_some_and(|entry| entry.availability(&advertised) == Availability::Offered);
                if offered {
                    if let Err(error) = self.composer_modes.toggle(slug) {
                        self.composer_status = Some(error.to_string());
                    }
                }
                Ok(None)
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
            ChatWindowIntent::OpenProviderApp(provider) => {
                Ok(Some(HostIntent::OpenProviderApp(provider)))
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
            ChatWindowIntent::LoadSession { provider, session } => {
                Ok(Some(HostIntent::LoadSession { provider, session }))
            }
            ChatWindowIntent::RefreshNativeSessions(provider) => {
                Ok(Some(HostIntent::RefreshNativeSessions(provider)))
            }
            ChatWindowIntent::SendTurn {
                provider,
                session,
                text,
                modes,
            } => Ok(Some(HostIntent::SendTurn {
                provider,
                session,
                text,
                modes,
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
            ChatWindowIntent::SetPermissionMode { provider, mode } => {
                self.permission_modes.insert(provider, mode);
                Ok(Some(HostIntent::SetPermissionMode { provider, mode }))
            }
            ChatWindowIntent::EditTrust(provider) => Ok(Some(HostIntent::EditTrust(provider))),
            ChatWindowIntent::AnswerTrust { provider, grant } => {
                self.trust_dialog = None;
                Ok(Some(HostIntent::AnswerTrust { provider, grant }))
            }
            ChatWindowIntent::SetTrustDraft(grant) => {
                if let Some(dialog) = self.trust_dialog.as_mut() {
                    dialog.draft = grant;
                }
                Ok(None)
            }
            ChatWindowIntent::ToggleRoomLibrary => {
                self.library_open = !self.library_open;
                Ok(None)
            }
            ChatWindowIntent::OpenRoomLibraryFolder => Ok(Some(HostIntent::OpenRoomLibraryFolder)),
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
            ChatWindowIntent::Command { draft, synthetic } => {
                Ok(Some(HostIntent::Command { draft, synthetic }))
            }
            // The board lives host-side; every brainstorm intent crosses.
            ChatWindowIntent::StartBrainstorm => Ok(Some(HostIntent::StartBrainstorm)),
            ChatWindowIntent::Brainstorm(op) => Ok(Some(HostIntent::Brainstorm(op))),
            ChatWindowIntent::BrainstormUndo => Ok(Some(HostIntent::BrainstormUndo)),
            ChatWindowIntent::BrainstormRedo => Ok(Some(HostIntent::BrainstormRedo)),
            ChatWindowIntent::BrainstormAssist => Ok(Some(HostIntent::BrainstormAssist)),
            ChatWindowIntent::PromoteBrainstorm => Ok(Some(HostIntent::PromoteBrainstorm)),
            ChatWindowIntent::SelectClyffy => Ok(Some(HostIntent::SelectClyffy)),
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
                self.project_bar(ui);
                ui.add_space(6.0);
                let body_height = ui.available_height();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), body_height),
                    Layout::left_to_right(Align::Min),
                    |ui| {
                        if self.project_input_locked {
                            ui.disable();
                        }
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

        self.project_popup(ui, &mut intents);
        self.trust_dialog_view(ui, &mut intents);
        if !self.project_input_locked {
            self.provider_hover_overlay(ui, &mut intents);
            self.provider_catalog_overlay(ui, &mut intents);
            self.provider_configuration_overlay(ui, &mut intents);
            self.generation_overlay(ui, &mut intents);
            self.compact_picker_overlay(ui, &mut intents);
            self.interaction_overlays(ui, &mut intents);
        }
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

    /// What an AI is trusted with in the active project (AH1b): `Some(None)`
    /// asked and not allowed, `Some(Some(ceiling))` allowed up to `ceiling`,
    /// `None` trust does not apply.
    pub fn set_provider_trust(
        &mut self,
        provider: ProviderId,
        trust: Option<Option<PermissionMode>>,
    ) {
        match trust {
            Some(ceiling) => {
                self.trust.insert(provider, ceiling);
            }
            None => {
                self.trust.remove(&provider);
            }
        }
    }

    /// Open the trust dialog for an AI in the active project, starting from
    /// what is granted now (everything off for an AI never asked).
    pub fn request_trust(&mut self, provider: ProviderId, project: String, current: Grant) {
        self.trust_dialog = Some(TrustDialog {
            provider,
            project,
            draft: current,
        });
    }

    pub fn trust_dialog_open(&self) -> Option<ProviderId> {
        self.trust_dialog.as_ref().map(|dialog| dialog.provider)
    }

    /// What the machine scan saw for a provider (AH1a).
    pub fn set_provider_on_machine(
        &mut self,
        provider: ProviderId,
        on_machine: OnMachine,
    ) -> Result<(), ChatWindowError> {
        self.runtime.set_on_machine(provider, on_machine)?;
        Ok(())
    }

    /// The library the host loaded: the built-ins plus the operator's own
    /// role and room files (the host reads disk; the window never does).
    pub fn set_room_library(&mut self, library: dpl_chat_core::rooms::Library) {
        self.room_library = library;
    }

    /// Offer (`Some`, with the current mode) or withdraw (`None`) the
    /// permission selector for `provider`. Only a provider whose runtime
    /// honours modes is offered one, so no setting is ever silently ignored.
    pub fn set_provider_permission_mode(
        &mut self,
        provider: ProviderId,
        mode: Option<PermissionMode>,
    ) {
        match mode {
            Some(mode) => {
                self.permission_modes.insert(provider, mode);
            }
            None => {
                self.permission_modes.remove(&provider);
            }
        }
    }

    /// The account the provider reports (shown beside "Ready", so the
    /// operator sees WHICH subscription a turn spends).
    pub fn set_provider_account(
        &mut self,
        provider: ProviderId,
        account: Option<AccountSummary>,
    ) -> Result<(), ChatWindowError> {
        self.runtime.set_account(provider, account)?;
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

    pub fn set_native_session_status(&mut self, provider: ProviderId, status: NativeSessionStatus) {
        if !status.qualified {
            let _ = self
                .runtime
                .revoke_capabilities(provider, [Capability::ResumeSession]);
        }
        self.native_sessions.insert(provider, status);
    }

    pub fn merge_native_sessions(
        &mut self,
        provider: ProviderId,
        sessions: Vec<SessionSummary>,
    ) -> Result<(), String> {
        let mut staged = self.runtime.clone();
        for session in sessions {
            staged
                .upsert_session(provider, session)
                .map_err(|error| error.to_string())?;
        }
        staged.archive()?;
        self.runtime = staged;
        Ok(())
    }

    pub fn replace_native_session_history(
        &mut self,
        provider: ProviderId,
        session: SessionSummary,
        events: Vec<ChatEvent>,
    ) -> Result<(), String> {
        let previous = self
            .runtime
            .provider_view(provider)
            .and_then(|view| view.active_session);
        let loaded = session.id;
        let mut staged = self.runtime.clone();
        staged.replace_session_history(provider, session, events)?;
        // A slow load must not steal focus from a chat selected in the meantime.
        if let Some(previous) = previous.filter(|previous| *previous != loaded) {
            staged
                .select_session(provider, previous)
                .map_err(|error| error.to_string())?;
        }
        self.runtime = staged;
        self.history_focus = None;
        self.history_page = None;
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
        // Every size in the strip comes from the window's own Fluid scale,
        // so the same code reads at the 720 px minimum and maximized.
        let fluid = cl::Fluid::of(ui.ctx().content_rect());
        let inset = fluid.sp_3().round() as i8;
        egui::Frame::new()
            .fill(chat::panel::STRIP)
            .stroke(Stroke::new(1.0, chat::panel::EDGE))
            .corner_radius(CornerRadius::same(5))
            .inner_margin(egui::Margin {
                left: inset,
                right: inset,
                top: fluid.sp_1().round() as i8,
                bottom: 0,
            })
            .show(ui, |ui| {
                let row = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), fluid.ctl_btn()),
                );
                let gap = fluid.sp_2();
                // The window controls are laid out FIRST and claim their real
                // width. Everything to their left shares only what remains,
                // so a tab can never be drawn underneath them.
                let controls = ui
                    .scope_builder(
                        UiBuilder::new()
                            .max_rect(row)
                            .layout(Layout::right_to_left(Align::Center)),
                        |ui| window_controls(ui, intents),
                    )
                    .response
                    .rect;
                let left = egui::Rect::from_min_max(
                    row.min,
                    egui::pos2((controls.left() - gap).max(row.left()), row.bottom()),
                );
                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(left)
                        .layout(Layout::left_to_right(Align::Center)),
                    |ui| {
                        ui.set_clip_rect(left.intersect(ui.clip_rect()));
                        self.title_and_tabs(ui, &fluid, intents);
                    },
                );
                ui.advance_cursor_after_rect(row);
            });
    }

    /// The title, the mode tabs, the search field and the availability word,
    /// fitted into the room the window controls left.
    ///
    /// When the room runs out the strip sheds in a fixed order — the search
    /// field, then the word "available" (its dots stay and carry it as a
    /// tooltip), then the title's words (the brand mark stays and still
    /// drags the window) — and only then do the tabs narrow. A tab is clipped
    /// to its own slot, so its label ends in "…" instead of running into the
    /// next tab.
    fn title_and_tabs(
        &self,
        ui: &mut egui::Ui,
        fluid: &cl::Fluid,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let gap = fluid.sp_2();
        let spacing = ui.spacing().item_spacing.x;
        let title_font = egui::FontId::proportional(fluid.fs_body_lg());
        let tab_font = egui::FontId::proportional(fluid.fs_body());
        let word_font = egui::TextStyle::Small.resolve(ui.style());

        let connected = self.connected_tabs();
        let social_label = if self.domain_states.social.unread > 0 {
            format!("Social  {}", self.domain_states.social.unread)
        } else {
            "Social".to_owned()
        };
        // General, the ready AIs, More AIs, then the two other domains.
        let tab_count = connected.len() + 4;
        let widest_label = ["General", "More AIs", "Content", social_label.as_str()]
            .into_iter()
            .chain(connected.iter().map(|(_, _, short_label)| *short_label))
            .map(|label| text_width(ui, label, tab_font.clone()))
            .fold(0.0_f32, f32::max);
        let natural_tab = ModeTab::chrome(fluid) + widest_label;
        let title_text = text_width(ui, "DevPULSE Chat", title_font.clone()) + spacing;
        let word = text_width(ui, "available", word_font) + spacing;
        let search_width = fluid.rail_expanded();
        let brand = fluid.ctl_icon();
        let dots = status_dots_width(ui, fluid);

        let gaps = spacing * (tab_count - 1) as f32;
        let tabs_natural = natural_tab * tab_count as f32 + gaps;
        let room = ui.available_width() - brand - spacing - gap * 2.0 - dots;
        let mut spare = room - tabs_natural;
        let show_search = spare >= title_text + word + search_width + gap;
        if show_search {
            spare -= search_width + gap;
        }
        let show_word = spare >= title_text + word;
        if show_word {
            spare -= word;
        }
        let show_title = spare >= title_text;
        if show_title {
            spare -= title_text;
        }
        let tab = ModeTab {
            width: if spare >= 0.0 {
                natural_tab
            } else {
                ((tabs_natural + spare - gaps) / tab_count as f32).max(0.0)
            },
            height: ui.available_height(),
            font: tab_font,
            fluid: *fluid,
        };

        let title = ui
            .scope(|ui| {
                brand_mark(ui, brand);
                if show_title {
                    ui.label(
                        RichText::new("DevPULSE Chat")
                            .strong()
                            .font(title_font)
                            .color(cl::ink::STRONG),
                    );
                }
            })
            .response
            .interact(Sense::click_and_drag());
        let title = cl::tip(title, "Drag to move Chat. Double-click to maximize it.");
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
        ui.add_space(gap);
        let focused_tab = self.focused_provider_tab();
        let in_ai = self.domain == ChatDomain::Ai;
        let more_open = in_ai && self.ai_tab == AiTab::MoreAis;
        let general = cl::tip(
            tab.show(
                ui,
                AI_MODE_ID,
                "General",
                in_ai && focused_tab.is_none() && !more_open,
            ),
            "Your general chat with Clyffy, powered by the AI you choose",
        );
        if general.clicked() {
            intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Ai));
        }
        for (id, label, short_label) in connected {
            let selected = in_ai && focused_tab == Some(id);
            // A tab carries the SHORT label — "ChatGPT / Codex" clips to
            // nonsense at this size. The full name rides in the tooltip.
            let response = cl::tip(
                tab.show(
                    ui,
                    PROVIDER_TAB_ID.child(&PilotKey::new(id.0)),
                    short_label,
                    selected,
                ),
                &format!("Chat with {label}"),
            );
            if response.clicked() {
                intents.push(ChatWindowIntent::SelectProviderTab(id));
            }
        }
        let more = cl::tip(
            tab.show(ui, MORE_AIS_TAB_ID, "More AIs", more_open),
            "Every other AI DevPULSE knows — sign one in or install it to give it a tab",
        );
        if more.clicked() {
            intents.push(ChatWindowIntent::SelectMoreAis);
        }
        let content = cl::tip(
            tab.show(
                ui,
                CONTENT_MODE_ID,
                "Content",
                self.domain == ChatDomain::Content,
            ),
            "Your shows and episodes, to talk about with an AI",
        );
        if content.clicked() {
            intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Content));
        }
        let social = cl::tip(
            tab.show(
                ui,
                SOCIAL_MODE_ID,
                &social_label,
                self.domain == ChatDomain::Social,
            ),
            "Conversations with people",
        );
        if social.clicked() {
            intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Social));
        }
        if show_search {
            ui.add_space(gap);
            let mut search_value = String::new();
            let search = ui.add_sized(
                [search_width, fluid.ctl_btn() - fluid.sp_2()],
                egui::TextEdit::singleline(&mut search_value).hint_text("Search DevPULSE"),
            );
            ui.pilot_track(
                &search,
                NodeMetadata::new(PilotId::new("chat.search"), Role::TextInput, "Search Chat"),
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if show_word {
                ui.label(RichText::new("available").small().color(Tone::Lime.color()));
            }
            let status = ui
                .scope(|ui| status_dots(ui, fluid))
                .response
                .interact(Sense::hover());
            let _ = cl::tip(status, "Chat is available");
        });
    }

    fn sidebar(
        &self,
        ui: &mut egui::Ui,
        composition: Composition,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let response = egui::Frame::new()
            .fill(chat::rail::FILL)
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
                let rooms = cl::tip(
                    sidebar_destination_button(ui, "♜  Rooms & roles", true),
                    "The roles (skeptic, realist, dreamer…) and rooms (council, debate…) AIs can work in, with the research behind each",
                );
                ui.pilot_track(
                    &rooms,
                    NodeMetadata::new(ROOM_LIBRARY_ID, Role::Button, "Rooms & roles"),
                );
                if rooms.clicked() {
                    intents.push(ChatWindowIntent::ToggleRoomLibrary);
                }
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
                if let Some((provider, _, _)) = target {
                    let status = self.native_sessions.get(&provider).copied().unwrap_or_default();
                    if status.qualified {
                        let response = sidebar_destination_button(ui, "Refresh saved chats", !status.listing && status.loading.is_none() && !self.project_input_locked);
                        ui.pilot_track(&response, NodeMetadata::new(REFRESH_NATIVE_CHATS_ID, Role::Button, "Refresh saved chats"));
                        if response.clicked() { intents.push(ChatWindowIntent::RefreshNativeSessions(provider)); }
                    }
                }
                let can_start = target.is_some_and(|(_, _, can_start)| can_start);
                let new_chat = cl::tip(
                    sidebar_destination_button(ui, "+  New chat", can_start),
                    "Start a new chat with the selected provider",
                );
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
                        // The deck's own convention: "<chapter> · <card>".
                        let response =
                            sidebar_row(ui, &format!("{} · {}", slide.chapter, slide.name), false);
                        let (group, slug) = split_address(&spine.id);
                        // Keyed by position: a card's name is its words,
                        // and one card can be shown twice (LS3b).
                        let mut metadata = NodeMetadata::new(
                            CONTENT_BOARD_ID
                                .child(&PilotKey::new(group))
                                .child(&PilotKey::new(slug))
                                .child(&PilotKey::new(slide.index.to_string())),
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
                    // Too narrow for the word, so the control is the brain
                    // alone — always present, so the catalog is one tap away
                    // even before a provider is chosen. The chips are dropped
                    // here rather than re-encoded as a tint: a distinction
                    // carried by colour alone is not one a screen reader hears.
                    let glyph = cl::tip(
                        cl::patterns::bleed_glyph(
                            ui,
                            egui::Id::new("dpl_chat_providers_bleed_compact"),
                            self.bleed,
                        ),
                        "AI Providers — every subscription and API endpoint",
                    );
                    let active = active_session_count(&self.runtime.provider_views());
                    let mut metadata = NodeMetadata::new(
                        PROVIDERS_ID.child(&PilotKey::new("compact")),
                        Role::Button,
                        "Open AI Providers",
                    );
                    metadata.value = Some(match self.runtime.selected_provider() {
                        Some(provider) => format!(
                            "{} · {active} active session{}",
                            provider.registration.manifest.descriptor.label,
                            if active == 1 { "" } else { "s" }
                        ),
                        None => "No provider selected".to_owned(),
                    });
                    ui.pilot_track(&glyph, metadata);
                    if glyph.clicked() {
                        intents.push(ChatWindowIntent::SetOverlay(Some(
                            DomainOverlay::ProviderCatalog,
                        )));
                    }
                    // Which provider the window speaks through, as plain
                    // lettering under the control — the control is the action.
                    if let Some(provider) = self.runtime.selected_provider() {
                        ui.label(
                            RichText::new(provider.registration.manifest.descriptor.short_label)
                                .small()
                                .color(cl::ink::STRONG),
                        );
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
            let picker = cl::tip(
                ui.pilot_button(PilotId::new("chat.sidebar.picker"), "•••"),
                "Show everything in this list",
            );
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

    /// The provider the rail is scoped to: the provider tab, but only while it
    /// still names a connected provider. Sign-out therefore drops the tab
    /// AND returns the roster without anyone having to send an intent.
    fn focused_provider_tab(&self) -> Option<ProviderId> {
        let AiTab::Provider(focus) = self.ai_tab else {
            return None;
        };
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

    /// The rail's provider section: ONE control into the catalog, then the
    /// two populations under their chips.
    ///
    /// **Activation puts a provider here, not registration.** One row per
    /// registered provider pushed "More providers…" off mid-word and left
    /// the session groups below with no height. Every provider, on or off,
    /// is one tap away in the catalog; the rail keeps what is turned on.
    fn provider_roster(
        &self,
        ui: &mut egui::Ui,
        providers: &[ProviderView],
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let subscriptions = rail_subscriptions(providers);
        let width = ui.available_width();
        let control = cl::tip(
            cl::patterns::bleed_control(
                ui,
                // Stable, never `ui.id()`-derived, and distinct from the
                // shell's control so the two do not share one animation.
                egui::Id::new("dpl_chat_providers_bleed"),
                "AI Providers",
                self.bleed,
                Some(width),
            ),
            "See every AI provider — the subscriptions you hold and the API endpoints you saved",
        );
        let mut metadata = NodeMetadata::new(PROVIDERS_ID, Role::Button, "Open AI Providers");
        metadata.description = Some(
            "Every provider: the subscriptions you hold and the API endpoints you saved, plus              Local or Custom setup."
                .to_owned(),
        );
        metadata.value = Some(format!("{} activated", subscriptions.len()));
        ui.pilot_track(&control, metadata);
        if control.clicked() {
            intents.push(ChatWindowIntent::SetOverlay(Some(
                DomainOverlay::ProviderCatalog,
            )));
        }

        // SUBSCRIPTIONS — the accounts already paid for.
        ui.add_space(cl::space::S2);
        let chip = cl::patterns::group_chip(ui, "Subscriptions", None);
        let _ = cl::tip(
            chip,
            "AI accounts you already pay for, signed in on this computer",
        );
        if subscriptions.is_empty() {
            group_awaiting(ui, "Nothing turned on yet");
        } else {
            for provider in subscriptions {
                self.provider_row(ui, provider, true, false, intents);
            }
        }

        // API — the qualifier is the most important word on the rail: a
        // subscription is already paid for, an API call costs money per use.
        // This is where the operator learns which they are about to spend
        // from, BEFORE they pick.
        ui.add_space(cl::space::S2);
        let chip = cl::patterns::group_chip(ui, "API", Some("usage-based"));
        let _ = cl::tip(
            chip,
            "Endpoints you saved yourself — each call is billed as you use it",
        );
        if self.provider_profiles.is_empty() {
            group_awaiting(ui, "No endpoints saved yet");
        } else {
            for profile in &self.provider_profiles {
                self.profile_row(ui, profile, intents);
            }
        }
    }

    /// One saved API endpoint, in the rail. A saved profile cannot chat yet
    /// (its transport adapter is not qualified), so the row says so and opens
    /// the catalog, where every way to add or change one lives.
    fn profile_row(
        &self,
        ui: &mut egui::Ui,
        profile: &ProviderProfile,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let response = cl::tip(
            sidebar_row(ui, &profile.display_name, false),
            "A saved endpoint. It cannot chat until its adapter is qualified — tap to see every provider.",
        );
        let mut metadata = NodeMetadata::new(
            PROVIDERS_ID
                .child(&PilotKey::new("api"))
                .child(&PilotKey::new(profile.key.as_str())),
            Role::Button,
            profile.display_name.clone(),
        );
        metadata.value = Some(format!("{} · usage-based", profile.kind.label()));
        metadata.description = Some(profile.activation_reason().to_owned());
        ui.pilot_track(&response, metadata);
        if response.clicked() {
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
                    chat::rail::SELECTED
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        chat::rail::SELECTED_EDGE
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
                                    RichText::new(provider_status(&provider.registration))
                                        .small()
                                        .color(cl::ink::MUTED),
                                );
                            });
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new("›").color(cl::ink::MUTED));
                            if !provider.sessions.is_empty() {
                                ui.label(
                                    RichText::new(provider.sessions.len().to_string())
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
        metadata.value = Some(provider_status(&provider.registration));
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
        rows.sort_by_key(|(_, session)| std::cmp::Reverse(session.modified_millis));
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
            .all(|provider| provider.sessions.is_empty())
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
        // Clyffy sits pinned above every provider session (BND1): the
        // house conversation, its backing a swappable binding. Present
        // as soon as the platform knows it; ACTIVE only while a
        // signed-in backing exists — the row never claims readiness it
        // borrowed from nobody.
        let mut pinned_heading_painted = false;
        if self.clyffy.present {
            micro_heading(ui, "PINNED");
            pinned_heading_painted = true;
            let backing = self
                .clyffy
                .backing
                .map_or("no backing yet".to_owned(), |p| format!("via {}", p.0));
            let state = if self.clyffy.active {
                "ready"
            } else {
                "waiting for a signed-in provider"
            };
            let label = format!("C  Clyffy\n   {backing} · {state}");
            let response = sidebar_row(ui, &label, false);
            let response = cl::tip(
                response,
                "DevPULSE's own session — /model picks which signed-in provider powers it.",
            );
            let mut metadata = NodeMetadata::new(CLYFFY_ROW_ID, Role::Tab, "Clyffy house session");
            metadata.value = Some(
                self.clyffy
                    .backing
                    .map_or("none".to_owned(), |p| p.0.to_owned()),
            );
            metadata.description = Some(state.to_owned());
            ui.pilot_track(&response, metadata);
            if response.clicked() {
                intents.push(ChatWindowIntent::SelectClyffy);
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
                        .sessions
                        .iter()
                        .filter(move |session| predicate.matches(session))
                        .map(move |session| (provider, session))
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                continue;
            }
            // Clyffy may already own the PINNED heading — one heading,
            // every pinned thing beneath it.
            if heading != "PINNED" || !pinned_heading_painted {
                micro_heading(ui, heading);
            }
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
            .fill(chat::panel::WELL)
            .stroke(Stroke::new(1.0, chat::panel::EDGE))
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

    /// Rooms & roles (R284): every room and role the operator can use, with
    /// its method, what it must produce, how it is judged, and the research
    /// behind it. Read-only: roles are added as files, not edited here.
    fn room_library_view(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        use dpl_chat_core::rooms::{Provenance, RoleCard, RoomSpec};
        let mut room_types = Vec::new();
        for room in &self.room_library.rooms {
            if !room_types.contains(&room.room_type) {
                room_types.push(room.room_type);
            }
        }
        let mut stances = Vec::new();
        for role in &self.room_library.roles {
            if !stances.contains(&role.stance) {
                stances.push(role.stance);
            }
        }
        let room_filter = &mut self.library_room_filter;
        let role_filter = &mut self.library_role_filter;
        let library = &self.room_library;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Rooms & roles")
                    .strong()
                    .size(18.0)
                    .color(cl::ink::STRONG),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let back = cl::tip(ui.button("‹ Back to chat"), "Return to the conversation");
                ui.pilot_track(
                    &back,
                    NodeMetadata::new(
                        ROOM_LIBRARY_ID.child(&PilotKey::new("back")),
                        Role::Button,
                        "Back to chat",
                    ),
                );
                if back.clicked() {
                    intents.push(ChatWindowIntent::ToggleRoomLibrary);
                }
                let folder = cl::tip(
                    ui.button("Open library folder"),
                    "Your own roles and rooms live here as JSON files; a new role is a new file",
                );
                ui.pilot_track(
                    &folder,
                    NodeMetadata::new(
                        ROOM_LIBRARY_ID.child(&PilotKey::new("folder")),
                        Role::Button,
                        "Open library folder",
                    ),
                );
                if folder.clicked() {
                    intents.push(ChatWindowIntent::OpenRoomLibraryFolder);
                }
            });
        });
        ui.label(
            RichText::new(
                "Roles are defined by their method and what they must produce, never by a persona. \
                 A room says who sits in it, in what order they speak, and how ONE answer is chosen: \
                 select, never blend.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.separator();
        let sources = |provenance: &Provenance| {
            let keys = provenance
                .sources
                .iter()
                .map(|key| format!("[{key}]"))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} · sources {keys}", provenance.evidence.label())
        };
        egui::ScrollArea::vertical()
            .id_salt("dpl_chat_room_library")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if !library.rejected.is_empty() {
                    ui.label(
                        RichText::new("Not loaded")
                            .small()
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                    for issue in library.rejected.iter().flat_map(|(_, issues)| issues) {
                        ui.label(
                            RichText::new(issue.to_string())
                                .small()
                                .color(cl::ink::BASE),
                        );
                    }
                    ui.separator();
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Rooms")
                            .small()
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                    library_filter(ui, room_filter, &room_types, "room", |kind| {
                        (kind.as_str(), kind.label())
                    });
                });
                let rooms = library
                    .rooms
                    .iter()
                    .filter(|room| room_filter.is_none_or(|kind| room.room_type == kind));
                for room in rooms {
                    let room: &RoomSpec = room;
                    let header = egui::CollapsingHeader::new(
                        RichText::new(format!("{}  ·  {}", room.name, room.room_type.label()))
                            .color(cl::ink::STRONG),
                    )
                    .id_salt(("dpl_chat_room", room.id.as_str()))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&room.summary).color(cl::ink::BASE));
                        for seat in &room.seats {
                            ui.label(
                                RichText::new(format!(
                                    "Seat {} × {}: {}",
                                    seat.seat, seat.count, seat.role
                                ))
                                .small()
                                .color(cl::ink::MUTED),
                            );
                        }
                        for phase in &room.phases {
                            ui.label(
                                RichText::new(format!(
                                    "Phase {}: {}, {}; each {}",
                                    phase.name,
                                    phase.seats.join(", "),
                                    phase.mode.label(),
                                    phase.visibility.label()
                                ))
                                .small()
                                .color(cl::ink::MUTED),
                            );
                        }
                        if let Some(selection) = &room.selection {
                            ui.label(
                                RichText::new(format!(
                                    "Chosen by {}: {}{}{}",
                                    selection.seat,
                                    if selection.blind { "blind, " } else { "" },
                                    if selection.swap_order {
                                        "both orders, "
                                    } else {
                                        ""
                                    },
                                    if selection.exclude_author_vendors {
                                        "never the authors' vendor"
                                    } else {
                                        "any vendor"
                                    }
                                ))
                                .small()
                                .color(cl::ink::MUTED),
                            );
                        }
                        ui.label(
                            RichText::new(sources(&room.provenance))
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                    });
                    ui.pilot_track(
                        &header.header_response,
                        NodeMetadata::new(
                            ROOM_LIBRARY_ID
                                .child(&PilotKey::new("room"))
                                .child(&PilotKey::new(room.id.as_str())),
                            Role::Button,
                            format!("{} room", room.name),
                        ),
                    );
                }
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Roles")
                            .small()
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                    library_filter(ui, role_filter, &stances, "stance", |stance| {
                        (stance.as_str(), stance.label())
                    });
                });
                let roles = library
                    .roles
                    .iter()
                    .filter(|role| role_filter.is_none_or(|stance| role.stance == stance));
                for role in roles {
                    let role: &RoleCard = role;
                    let header = egui::CollapsingHeader::new(
                        RichText::new(format!("{}  ·  {}", role.name, role.stance.label()))
                            .color(cl::ink::STRONG),
                    )
                    .id_salt(("dpl_chat_role", role.id.as_str()))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&role.summary).color(cl::ink::BASE));
                        ui.label(
                            RichText::new(format!("Method: {}", role.method.name))
                                .small()
                                .strong()
                                .color(cl::ink::BASE),
                        );
                        for (index, step) in role.method.steps.iter().enumerate() {
                            ui.label(
                                RichText::new(format!("{}. {step}", index + 1))
                                    .small()
                                    .color(cl::ink::MUTED),
                            );
                        }
                        let fields = role.output_fields();
                        if !fields.is_empty() {
                            ui.label(
                                RichText::new(format!("Must produce: {}", fields.join(", ")))
                                    .small()
                                    .color(cl::ink::MUTED),
                            );
                        }
                        ui.label(
                            RichText::new(format!("Judged on: {}", role.rubric.join("; ")))
                                .small()
                                .color(cl::ink::MUTED),
                        );
                        if !role.must_not.is_empty() {
                            ui.label(
                                RichText::new(format!("Never: {}", role.must_not.join("; ")))
                                    .small()
                                    .color(cl::ink::MUTED),
                            );
                        }
                        ui.label(
                            RichText::new(sources(&role.provenance))
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                        if let Some(note) = &role.provenance.note {
                            ui.label(RichText::new(note).small().color(cl::ink::SUBTLE));
                        }
                    });
                    ui.pilot_track(
                        &header.header_response,
                        NodeMetadata::new(
                            ROOM_LIBRARY_ID
                                .child(&PilotKey::new("role"))
                                .child(&PilotKey::new(role.id.as_str())),
                            Role::Button,
                            format!("{} role", role.name),
                        ),
                    );
                }
            });
    }

    fn ai_conversation(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if self.library_open {
            self.room_library_view(ui, intents);
            return;
        }
        if self.ai_tab == AiTab::MoreAis {
            self.more_ais_page(ui, intents);
            return;
        }
        if self.focused_provider_tab().is_none() {
            self.general_header(ui, intents);
            ui.add_space(cl::space::S2);
        }
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
                RichText::new(provider_status(&provider.registration))
                    .small()
                    .color(activation_color(
                        &provider.registration.activation,
                        provider_tone(descriptor.accent),
                    )),
            );
            let offered = self.permission_modes.get(&descriptor.id).copied();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("route-aware")
                        .small()
                        .color(Tone::Cyan.color()),
                );
                let ceiling = self.trust.get(&descriptor.id).copied();
                if let Some(Some(_)) = ceiling {
                    let response = cl::tip(
                        ui.small_button("Trust…"),
                        "Change what this AI may do in this project",
                    );
                    ui.pilot_track(
                        &response,
                        NodeMetadata::new(
                            TRUST_EDIT_ID.child(&PilotKey::new(descriptor.id.0)),
                            Role::Button,
                            format!("{} trust in this project", descriptor.label),
                        ),
                    );
                    if response.clicked() {
                        intents.push(ChatWindowIntent::EditTrust(descriptor.id));
                    }
                }
                if let Some(current) = offered {
                    permission_selector(
                        ui,
                        descriptor.id,
                        descriptor.label,
                        current,
                        ceiling.flatten(),
                        intents,
                    );
                }
            });
        });
        if let Some(session) = provider
            .active_session
            .and_then(|id| provider.sessions.iter().find(|item| item.id == id))
        {
            ui.label(RichText::new(&session.title).small().color(cl::ink::MUTED));
        }
        ui.separator();
        self.trust_blocked_strip(ui, &provider, intents);

        // A transcript already on this machine outranks the live stream: the
        // operator picked one specific past conversation, and the pane must
        // draw THAT or say plainly why it cannot.
        if let Some((focus_provider, locator)) = self.history_focus {
            self.history_transcript(ui, focus_provider, locator);
            return;
        }

        // A stored project transcript belongs to the operator regardless of
        // whether this computer can currently authenticate its provider.
        // The heading retains current activation/account status, while the
        // composer independently refuses sending until native resume exists.
        let reading_project_archive = self.project_session_read_only();
        if !provider.registration.activation.is_activated() && !reading_project_archive {
            self.provider_activation_ceremony(ui, &provider, intents);
            return;
        }
        if reading_project_archive {
            // The saved transcript stays readable while the provider is
            // signed out, so the way back in rides above it rather than
            // replacing it with the connection card.
            provider_sign_in_strip(ui, &provider, intents);
            let status = self
                .native_sessions
                .get(&descriptor.id)
                .copied()
                .unwrap_or_default();
            if status.qualified {
                if let Some(session) = provider.active_session.filter(|id| {
                    provider
                        .sessions
                        .iter()
                        .any(|saved| saved.id == *id && saved.provider_session_id.is_some())
                }) {
                    let ready = status.loading.is_none()
                        && !status.listing
                        && !self.project_input_locked
                        && provider.registration.activation.is_activated();
                    let response = cl::tip(
                        ui.add_enabled(
                            ready,
                            egui::Button::new(if status.loading == Some(session) {
                                "Resuming chat..."
                            } else {
                                "Resume chat"
                            }),
                        ),
                        "Pick this saved chat back up where it left off",
                    );
                    ui.pilot_track(
                        &response,
                        NodeMetadata::new(RESUME_CHAT_ID, Role::Button, "Resume saved chat"),
                    );
                    if response.clicked() {
                        intents.push(ChatWindowIntent::LoadSession {
                            provider: descriptor.id,
                            session,
                        });
                    }
                }
            }
            ui.label(
                RichText::new("Saved project chat — read-only")
                    .small()
                    .color(Tone::Amber.color()),
            );
        }

        let events = provider
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
                self.brainstorm_board(ui, intents);
                if rendered == 0 && self.brainstorm.doc.is_none() {
                    let detail = provider.registration.activation.reason().unwrap_or(
                        "Choose a session or qualify this provider before starting a turn.",
                    );
                    empty_state(ui, "No messages in this session", detail);
                }
            });
    }

    /// General's header: Clyffy's house session, and one choice per
    /// signed-in AI for what powers it. The choice is saved by the host in
    /// chat settings; Clyffy's own modes and router arrive with CL1.
    fn general_header(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let connected = self.connected_tabs();
        let backing = self.clyffy.backing;
        let response = egui::Frame::new()
            .fill(chat::panel::RAISED)
            .stroke(Stroke::new(1.0, chat::panel::EDGE))
            .corner_radius(CornerRadius::same(cl::RADIUS_SM))
            .inner_margin(egui::Margin::symmetric(
                cl::space::S3 as i8,
                cl::space::S2 as i8,
            ))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("General").strong().color(cl::ink::STRONG));
                    ui.label(
                        RichText::new("Clyffy, DevPULSE's own chat · powered by")
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    if connected.is_empty() {
                        ui.label(
                            RichText::new("no AI is signed in yet — sign one in under More AIs")
                                .small()
                                .color(Tone::Amber.color()),
                        );
                    }
                    for (id, label, short_label) in connected {
                        let chosen = backing == Some(id);
                        let choice = cl::tip(
                            chat_chip(ui, None, short_label, true, chosen),
                            &format!("Power General with {label}. DevPULSE remembers your choice."),
                        );
                        let mut metadata = NodeMetadata::new(
                            GENERAL_BACKING_ID.child(&PilotKey::new(id.0)),
                            Role::Radio,
                            format!("Power General with {label}"),
                        );
                        metadata.checked = Some(chosen);
                        metadata.selected = Some(chosen);
                        metadata.value =
                            Some(if chosen { "selected" } else { "not selected" }.to_owned());
                        ui.pilot_track(&choice, metadata);
                        if choice.clicked() && !chosen {
                            intents.push(ChatWindowIntent::AssignGeneralBacking(id));
                        }
                    }
                });
            })
            .response;
        let mut metadata = NodeMetadata::new(GENERAL_ID, Role::Region, "General chat");
        metadata.value = Some(
            backing
                .map(|id| id.0.to_owned())
                .unwrap_or_else(|| "no backing yet".to_owned()),
        );
        metadata.description =
            Some("Clyffy's house session, powered by the signed-in AI you choose.".to_owned());
        ui.pilot_track(&response, metadata);
    }

    /// The trust dialog (AH1b): before an AI works in a project folder, the
    /// operator says what it may do there. Every switch starts off, Allow needs
    /// Read. Don't allow is remembered and keeps the AI stopped; Esc closes the
    /// dialog and changes nothing.
    fn trust_dialog_view(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let Some(dialog) = &self.trust_dialog else {
            return;
        };
        let label = self
            .runtime
            .provider_views()
            .iter()
            .find(|view| view.registration.manifest.descriptor.id == dialog.provider)
            .map_or(dialog.provider.0, |view| {
                view.registration.manifest.descriptor.label
            });
        let provider = dialog.provider;
        let mut draft = dialog.draft;
        let modal = egui::Modal::new(egui::Id::new(("dpl_chat_trust", provider.0)))
            .backdrop_color(cl::scrim(cl::SCRIM_MEDIA_A))
            .frame(
                egui::Frame::new()
                    .fill(chat::panel::CARD)
                    .stroke(Stroke::new(1.0, Tone::Amber.color()))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui.ctx(), |ui| {
                ui.set_width(440.0);
                ui.label(
                    RichText::new(format!("Let {label} work in “{}”?", dialog.project))
                        .strong()
                        .size(17.0)
                        .color(cl::ink::STRONG),
                );
                ui.add_space(cl::space::S1);
                ui.label(
                    RichText::new(
                        "Nothing of this AI runs in the project folder until you allow it. \
                         You can change this later from its tab.",
                    )
                    .small()
                    .color(cl::ink::MUTED),
                );
                ui.add_space(cl::space::S2);
                let switches = [
                    (
                        "read",
                        "Read files",
                        "It can open and read the files in this project's folder.",
                        &mut draft.read,
                    ),
                    (
                        "change",
                        "Change files",
                        "It can create and edit files in this project's folder.",
                        &mut draft.change,
                    ),
                    (
                        "run",
                        "Run commands",
                        "It can run programs and commands in this project's folder.",
                        &mut draft.run,
                    ),
                ];
                for (key, name, tip, value) in switches {
                    let response = cl::tip(ui.checkbox(value, name), tip);
                    let mut metadata = NodeMetadata::new(
                        TRUST_ID.child(&PilotKey::new(key)),
                        Role::Checkbox,
                        name,
                    );
                    metadata.selected = Some(*value);
                    ui.pilot_track(&response, metadata);
                }
                if draft != dialog.draft {
                    intents.push(ChatWindowIntent::SetTrustDraft(draft));
                }
                if (draft.change || draft.run) && !draft.read {
                    ui.label(
                        RichText::new("Changing or running needs Read files too.")
                            .small()
                            .color(Tone::Amber.color()),
                    );
                }
                if draft.read && draft.change != draft.run {
                    ui.label(
                        RichText::new(
                            "It will ask you before it changes a file or runs a command: \
                             an AI cannot be allowed one without the other.",
                        )
                        .small()
                        .color(cl::ink::MUTED),
                    );
                }
                ui.add_space(cl::space::S3);
                ui.horizontal(|ui| {
                    let allow = cl::tip(
                        ui.add_enabled(draft.read, egui::Button::new("Allow")),
                        "Save these choices for this AI in this project",
                    );
                    ui.pilot_track(
                        &allow,
                        NodeMetadata::new(
                            TRUST_ID.child(&PilotKey::new("allow")),
                            Role::Button,
                            "Allow",
                        ),
                    );
                    if allow.clicked() {
                        intents.push(ChatWindowIntent::AnswerTrust {
                            provider,
                            grant: Some(draft),
                        });
                    }
                    let deny = cl::tip(
                        ui.button("Don't allow"),
                        "Keep this AI stopped in this project. You can allow it later from its tab.",
                    );
                    ui.pilot_track(
                        &deny,
                        NodeMetadata::new(
                            TRUST_ID.child(&PilotKey::new("deny")),
                            Role::Button,
                            "Don't allow",
                        ),
                    );
                    if deny.clicked() {
                        intents.push(ChatWindowIntent::AnswerTrust {
                            provider,
                            grant: Some(Grant::default()),
                        });
                    }
                });
            });
        ui.pilot_track(
            &modal.response,
            NodeMetadata::new(TRUST_ID, Role::Dialog, format!("Trust {label}")),
        );
        if modal.should_close() {
            intents.push(ChatWindowIntent::AnswerTrust {
                provider,
                grant: None,
            });
        }
    }

    /// While an AI is not allowed in the project, its pane says so and offers
    /// the dialog, so a stopped AI is never a mystery.
    fn trust_blocked_strip(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let descriptor = provider.registration.manifest.descriptor;
        if self.trust.get(&descriptor.id) != Some(&None) {
            return;
        }
        let tone = provider_tone(descriptor.accent);
        cl::vibron(tone, cl::TraitState::Idle).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} is not allowed in this project yet",
                        descriptor.short_label
                    ))
                    .strong()
                    .color(cl::ink::STRONG),
                );
                let response = cl::tip(
                    ui.button("Choose what it may do"),
                    "Open the trust choices for this AI in this project",
                );
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(
                        TRUST_BLOCKED_ID.child(&PilotKey::new(descriptor.id.0)),
                        Role::Button,
                        format!("Allow {} in this project", descriptor.short_label),
                    ),
                );
                if response.clicked() {
                    intents.push(ChatWindowIntent::EditTrust(descriptor.id));
                }
            });
        });
    }

    /// The More AIs tab: every known AI that has not earned a tab, with its
    /// state in plain words. One tap selects it, and its setup — the same
    /// connection card the conversation pane has always shown — opens directly
    /// under that row, so the way in is the existing sign-in and install flow
    /// and it is never below fifty other rows, out of sight.
    fn more_ais_page(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let providers = self.runtime.provider_views();
        let (_, rows) = tab_model(&providers);
        ui.label(
            RichText::new("More AIs")
                .strong()
                .size(18.0)
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "AIs DevPULSE knows that are not ready on this computer yet. Tap one to set it up.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.separator();
        let selected = self.runtime.selected_provider_id();
        // About fifty AIs: the list scrolls, so none is clipped out of reach.
        let list = egui::ScrollArea::vertical()
            .id_salt("chat.more-ais.scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = cl::space::S1;
                    if rows.is_empty() {
                        ui.label(
                            RichText::new(
                                "Every AI DevPULSE knows is signed in and has its own tab.",
                            )
                            .small()
                            .color(cl::ink::SUBTLE),
                        );
                    }
                    for row in &rows {
                        let chosen = selected == Some(row.provider);
                        let next = match row.state {
                            MoreAiState::SignIn => "sign in",
                            MoreAiState::Open => "open it",
                            MoreAiState::Install => "install it",
                            MoreAiState::NotOnThisComputer => "see what it needs",
                        };
                        let response = cl::tip(
                            more_ai_row(ui, row, chosen),
                            &format!("{} — {}. Tap to {next}.", row.label, row.state.label()),
                        );
                        let mut metadata = NodeMetadata::new(
                            MORE_AIS_ID.child(&PilotKey::new(row.provider.0)),
                            Role::Button,
                            row.label,
                        );
                        metadata.value = Some(row.state.label().to_owned());
                        metadata.selected = Some(chosen);
                        metadata.description = match (row.on_machine, &row.reason) {
                            (Some(seen), Some(reason)) => Some(format!("{seen}. {reason}")),
                            (Some(seen), None) => Some(seen.to_owned()),
                            (None, reason) => reason.clone(),
                        };
                        ui.pilot_track(&response, metadata);
                        if response.clicked() {
                            intents.push(ChatWindowIntent::SelectProvider(row.provider));
                            // Its setup card opens under it next frame; bring the
                            // row to the top so the card has room to be seen.
                            response.scroll_to_me(Some(Align::Min));
                        }
                        if chosen {
                            if let Some(provider) = providers.iter().find(|provider| {
                                provider.registration.manifest.descriptor.id == row.provider
                            }) {
                                ui.indent(
                                    MORE_AIS_ID.child(&PilotKey::new(row.provider.0)),
                                    |ui| {
                                        self.provider_activation_ceremony(ui, provider, intents);
                                    },
                                );
                                ui.add_space(cl::space::S2);
                            }
                        }
                    }
                })
                .response
            })
            .inner;
        let mut metadata = NodeMetadata::new(MORE_AIS_ID, Role::List, "More AIs");
        metadata.value = Some(rows.len().to_string());
        ui.pilot_track(&list, metadata);
    }

    /// The composer `+` — the only plus on the composer (the way into AI
    /// Providers is drawn as the providers' mark). A drawn plus and a word;
    /// it says whether its menu is open.
    fn composer_more_button(&self, ui: &mut egui::Ui) -> egui::Response {
        let response = cl::tip(
            icon_word_button(ui, "composer-more", "Add", true, self.composer_menu_open),
            "Add to your message: a file, a picture or clip, a component, or your voice",
        );
        let mut metadata = NodeMetadata::new(COMPOSER_MORE_ID, Role::Button, "Add to message");
        metadata.expanded = Some(self.composer_menu_open);
        metadata.value = Some(
            if self.composer_menu_open {
                "open"
            } else {
                "closed"
            }
            .to_owned(),
        );
        metadata.description =
            Some("Opens the menu: attach files, image or video, a component, or voice.".to_owned());
        ui.pilot_track(&response, metadata);
        response
    }

    /// Close the `+` popup and hand the caret back to the message.
    fn close_composer_menu(&mut self) {
        self.composer_menu_open = false;
        self.composer_focus_requested = true;
    }

    /// The context row: the way into AI Providers, the observed route, the
    /// assist toggle, the AI the message goes to and the route count.
    ///
    /// One line at every width. The route label is the elastic part: it
    /// gets what the fixed controls leave and ends in "…" past that, and the
    /// route count is shed before the route itself gets unreadable.
    fn composer_context_row(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        ui.horizontal(|ui| {
            let route = self.route_context.offered();
            let route_hub = route.map_or_else(
                || "unavailable".to_owned(),
                |snapshot| snapshot.route.hub.0.to_owned(),
            );
            let route_label = route.map_or_else(
                || "No Main route observed".to_owned(),
                |snapshot| format!("{} / {}", snapshot.route.hub.0, snapshot.route.sub.0),
            );
            let assist_label = if self.assist_enabled {
                "⚡ Assist on"
            } else {
                "⚡ Assist"
            };
            let selected = self.runtime.selected_provider();
            let selector_label = selected.as_ref().map(|provider| {
                format!(
                    "{} · provider default",
                    provider.registration.manifest.descriptor.label
                )
            });
            let routes_label = format!("{} routes", self.routes.len());

            // The budget, from the style's own measures.
            let small = egui::TextStyle::Small.resolve(ui.style());
            let pad = 2.0 * ui.spacing().button_padding.x;
            let gap = ui.spacing().item_spacing.x;
            let icon = ui.text_style_height(&egui::TextStyle::Small);
            let catalog = if self.domain == ChatDomain::Ai {
                ui.text_style_height(&egui::TextStyle::Body) + pad + gap
            } else {
                0.0
            };
            let assist = text_width(ui, assist_label, small.clone()) + pad + gap;
            let selector = selector_label.as_deref().map_or(0.0, |label| {
                text_width(ui, label, small.clone()) + pad + gap
            });
            let short_selector = selected
                .as_ref()
                .map(|provider| provider.registration.manifest.descriptor.short_label);
            let selector_short = short_selector.map_or(0.0, |label| {
                text_width(ui, label, small.clone()) + pad + gap
            });
            let routes = text_width(ui, &routes_label, small.clone()) + gap;
            // The row sheds in a fixed order rather than overflow — an
            // overflowing row widens the composer and pushes Send out: first
            // the route count, then the selector's long name, then the route
            // words (the route icon stays), and last the selector itself.
            let readable_route = cl::space::S6 * 2.0;
            let room = ui.available_width() - catalog - (icon + gap) - assist;
            let (show_routes, selector_shown, budget) =
                if room - selector - routes >= readable_route {
                    (true, SelectorShown::Full, room - selector - routes)
                } else if room - selector >= readable_route {
                    (false, SelectorShown::Full, room - selector)
                } else if room - selector_short >= 0.0 {
                    (false, SelectorShown::Short, room - selector_short)
                } else {
                    (false, SelectorShown::Hidden, room)
                };
            let route_width = (text_width(ui, &route_label, small) + gap)
                .min(budget)
                .max(0.0);

            // THE SECOND WAY INTO THE CATALOG, drawn as the providers' own
            // mark so it never reads as the composer's `+` ("Add").
            if self.domain == ChatDomain::Ai {
                let catalog = cl::tip(
                    icon_button(ui, "brain"),
                    "AI Providers — every subscription and API endpoint",
                );
                let mut metadata =
                    NodeMetadata::new(ADD_PROVIDER_ID, Role::Button, "Add a provider");
                metadata.description = Some(
                    "Open AI Providers: the subscriptions you hold and the API \
                     endpoints you saved."
                        .to_owned(),
                );
                ui.pilot_track(&catalog, metadata);
                if catalog.clicked() {
                    intents.push(ChatWindowIntent::SetOverlay(Some(
                        DomainOverlay::ProviderCatalog,
                    )));
                }
            }
            let _ = cl::icon_label(ui, "route", icon, Tone::Cyan.color());
            let route_response = ui
                .allocate_ui_with_layout(
                    egui::vec2(route_width, icon),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(route_label.clone())
                                    .small()
                                    .color(Tone::Cyan.color()),
                            )
                            .truncate(),
                        )
                    },
                )
                .inner;
            let route_response = cl::tip(route_response, &route_label);
            let mut route_metadata =
                NodeMetadata::new(ROUTE_ID, Role::Status, "Observed Main route");
            route_metadata.value = Some(route_hub);
            route_metadata.description = Some(route_label);
            ui.pilot_track(&route_response, route_metadata);
            // ASSIST — beside the route truth it activates. Never provider-
            // or capability-gated: one toggle, one acknowledgement, every
            // provider.
            let assist = ui
                .add(
                    egui::Button::new(assist_label)
                        .small()
                        .selected(self.assist_enabled),
                )
                .on_hover_text(
                    "Assist mode: chat and the app work as one — chat sees your \
                     route and open work and may offer pop-up suggestions you \
                     approve or dismiss. One setting for every provider.",
                );
            let mut assist_metadata = NodeMetadata::new(ASSIST_ID, Role::Button, "Assist mode");
            assist_metadata.value = Some(if self.assist_enabled { "on" } else { "off" }.to_owned());
            ui.pilot_track(&assist, assist_metadata);
            if assist.clicked() {
                intents.push(ChatWindowIntent::ToggleAssist {
                    enable: !self.assist_enabled,
                });
            }
            if let (Some(provider), Some(selector_label), Some(short_label)) =
                (selected, selector_label, short_selector)
            {
                let descriptor = provider.registration.manifest.descriptor;
                let shown = match selector_shown {
                    SelectorShown::Full => Some(selector_label.as_str()),
                    SelectorShown::Short => Some(short_label),
                    SelectorShown::Hidden => None,
                };
                if let Some(shown) = shown {
                    let selector = cl::tip(
                        ui.small_button(shown),
                        "Which AI this message goes to — tap for its details",
                    );
                    let mut metadata =
                        NodeMetadata::new(PROVIDER_MODEL_ID, Role::Button, "Provider and model");
                    metadata.value = Some(selector_label);
                    ui.pilot_track(&selector, metadata);
                    if selector.clicked() {
                        intents.push(ChatWindowIntent::SetOverlay(Some(DomainOverlay::Provider(
                            descriptor.id,
                        ))));
                    }
                }
            }
            if show_routes {
                ui.label(RichText::new(routes_label).small().color(cl::ink::SUBTLE));
            }
        });
    }

    /// The compose row: the one `+` ("Add") and the mode chips, the chips
    /// fitted to what the row has left.
    fn composer_compose_row(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        ui.horizontal(|ui| {
            // THE `+`: one popup holding the census's menu entries, beside
            // where the operator types, as every chat client puts it.
            let more = self.composer_more_button(ui);
            self.composer_menu(ui, &more, intents);
            // THE MODES: how the next turn should run. Only the AI domain has
            // a turn to run.
            if self.domain == ChatDomain::Ai {
                self.mode_chips(ui, intents);
            }
        });
    }

    /// The composer's one popup style, shared by the `+` menu and the modes
    /// overflow: a raised plate rising from its anchor, its entries stacked
    /// full-width. The rise takes a short beat, or none when the style asks
    /// for no motion (the operator's choice, or the pilot's frozen clock).
    fn composer_popup<R>(
        ui: &egui::Ui,
        id: egui::Id,
        anchor: Rect,
        open: bool,
        metadata: NodeMetadata,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<egui::InnerResponse<R>> {
        let duration = if ui.style().animation_time > 0.0 {
            cl::motion::DUR_FAST
        } else {
            0.0
        };
        let t = ui.ctx().animate_bool_with_time(id, open, duration);
        if !open {
            return None;
        }
        let rise = (1.0 - t) * cl::space::S2;
        Some(
            egui::Area::new(id)
                .order(egui::Order::Foreground)
                .pivot(Align2::LEFT_BOTTOM)
                .fixed_pos(anchor.left_top() + egui::vec2(0.0, rise - cl::space::S1))
                .show(ui.ctx(), |ui| {
                    ui.set_opacity(t);
                    let panel = egui::Frame::new()
                        .fill(chat::composer::POPUP)
                        .stroke(Stroke::new(1.0, Tone::Cyan.color()))
                        .corner_radius(CornerRadius::same(cl::RADIUS_SM))
                        .inner_margin(egui::Margin::same(cl::space::S2 as i8))
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                                ui.set_min_width(cl::space::S6 * 6.0);
                                add_contents(ui)
                            })
                            .inner
                        });
                    ui.pilot_track(&panel.response, metadata);
                    panel.inner
                }),
        )
    }

    /// The `+` popup: the census's menu entries, each with its icon, its
    /// tooltip and its pilot id; a withheld one stays visible, disabled, and
    /// says why.
    ///
    /// It closes on a pick, on Esc, or on a click elsewhere, and focus goes
    /// back to the message. While it is open, focus stays inside it: Tab past
    /// the last entry comes back to the first. It rises into place over a
    /// short beat, or at once when the style asks for no motion.
    fn composer_menu(
        &mut self,
        ui: &mut egui::Ui,
        anchor: &egui::Response,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        if anchor.clicked() {
            if self.composer_menu_open {
                self.close_composer_menu();
            } else {
                // One popup at a time: two would fight over the focus trap.
                self.composer_menu_open = true;
                self.mode_menu_open = false;
            }
        }
        if self.composer_menu_open
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.close_composer_menu();
        }
        let (ai, advertised) = self.advertised();
        let rows: Vec<(&'static ComposerEntry, Option<String>)> = in_group(ComposerGroup::Menu)
            .map(|entry| (entry, self.menu_entry_withheld(entry, ai, &advertised)))
            .collect();
        let focused = ui.memory(|memory| memory.focused());
        let mut metadata = NodeMetadata::new(COMPOSER_MENU_ID, Role::Region, "Add to message");
        metadata.value = Some(rows.len().to_string());
        let Some(shown) = Self::composer_popup(
            ui,
            egui::Id::new("dpl_chat_composer_menu"),
            anchor.rect,
            self.composer_menu_open,
            metadata,
            |ui| {
                let mut picked = None;
                let mut entry_ids = Vec::new();
                for (entry, withheld) in &rows {
                    let enabled = withheld.is_none();
                    let response = icon_word_button(ui, entry.icon, entry.label, enabled, false);
                    let response = match withheld {
                        Some(reason) => response.on_disabled_hover_text(reason),
                        None => cl::tip(response, entry.tip),
                    };
                    let mut metadata =
                        NodeMetadata::new(menu_entry_id(entry.slug), Role::MenuItem, entry.label);
                    metadata.description =
                        Some(withheld.clone().unwrap_or_else(|| entry.tip.to_owned()));
                    ui.pilot_track(&response, metadata);
                    if enabled {
                        entry_ids.push(response.id);
                    }
                    if response.clicked() {
                        picked = Some(entry.slug);
                    }
                }
                (picked, entry_ids)
            },
        ) else {
            return;
        };
        let (picked, entry_ids) = shown.inner;
        trap_focus(ui, focused, &entry_ids);
        if let Some(slug) = picked {
            self.close_composer_menu();
            match slug {
                "media" => intents.push(ChatWindowIntent::SetOverlay(Some(
                    DomainOverlay::Generation,
                ))),
                // The Interaction Library's first connected component: the
                // brainstorm board (BS1).
                "component" => intents.push(ChatWindowIntent::StartBrainstorm),
                "attach" => {
                    self.composer_status = Some(
                        "Attaching files isn't connected yet — paste the text into your message for now."
                            .to_owned(),
                    );
                }
                other => {
                    self.composer_status = Some(format!("{other} isn't connected yet."));
                }
            }
        } else if shown.response.clicked_elsewhere() && !anchor.clicked() {
            self.close_composer_menu();
        }
    }

    /// Why a `+` entry is withheld right now, or `None` when it is offered.
    /// Image/video is also offered when the host has image making set up,
    /// whatever the AI advertises; a component needs the AI conversation.
    fn menu_entry_withheld(
        &self,
        entry: &ComposerEntry,
        ai: Option<&str>,
        advertised: &[Capability],
    ) -> Option<String> {
        match entry.slug {
            "media" if self.generation_status.image_available => None,
            "component" if self.domain != ChatDomain::Ai => Some(
                "Components live in the AI conversation — switch to the General tab.".to_owned(),
            ),
            _ => entry.withheld_reason(ai, advertised),
        }
    }

    /// The mode chips, fitted to the row. Every chip that fits stands
    /// inline at its full width; the rest collapse into one "More" chip that
    /// opens them in the composer's popup style. A mode the AI does not
    /// advertise stays visible, disabled, and says why in plain words.
    fn mode_chips(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let (ai, advertised) = self.advertised();
        let entries: Vec<&'static ComposerEntry> = in_group(ComposerGroup::Mode).collect();
        let gap = ui.spacing().item_spacing.x;
        let widths: Vec<f32> = entries
            .iter()
            .map(|entry| chip_width(ui, Some(entry.icon), entry.label) + gap)
            .collect();
        let room = ui.available_width();
        let inline = if widths.iter().sum::<f32>() <= room {
            entries.len()
        } else {
            let mut used = chip_width(ui, None, MODES_MORE_LABEL) + gap;
            widths
                .iter()
                .take_while(|width| {
                    used += **width;
                    used <= room
                })
                .count()
        };
        for entry in &entries[..inline] {
            self.mode_chip(ui, entry, ai, &advertised, intents);
        }
        if inline == entries.len() {
            self.mode_menu_open = false;
            return;
        }
        let hidden = &entries[inline..];
        let more = cl::tip(
            chat_chip(ui, None, MODES_MORE_LABEL, true, self.mode_menu_open),
            "More ways to run your next message",
        );
        let mut metadata = NodeMetadata::new(MODES_MORE_ID, Role::Button, "More modes");
        metadata.expanded = Some(self.mode_menu_open);
        metadata.value = Some(format!("{} more", hidden.len()));
        metadata.description = Some(
            hidden
                .iter()
                .map(|entry| entry.label)
                .collect::<Vec<_>>()
                .join(", "),
        );
        ui.pilot_track(&more, metadata);
        self.mode_overflow(ui, &more, hidden, ai, &advertised, intents);
    }

    /// The modes that did not fit, in the `+` menu's popup. It closes on Esc
    /// or a click elsewhere, focus going back to the message; while open,
    /// focus stays inside it. Toggling a mode leaves it open, so several can
    /// be set in one visit.
    fn mode_overflow(
        &mut self,
        ui: &mut egui::Ui,
        anchor: &egui::Response,
        hidden: &[&'static ComposerEntry],
        ai: Option<&str>,
        advertised: &[Capability],
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        if anchor.clicked() {
            self.mode_menu_open = !self.mode_menu_open;
            if self.mode_menu_open {
                self.composer_menu_open = false;
            } else {
                self.composer_focus_requested = true;
            }
        }
        if self.mode_menu_open
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.mode_menu_open = false;
            self.composer_focus_requested = true;
        }
        let focused = ui.memory(|memory| memory.focused());
        let mut metadata = NodeMetadata::new(MODES_MENU_ID, Role::Region, "More modes");
        metadata.value = Some(hidden.len().to_string());
        let Some(shown) = Self::composer_popup(
            ui,
            egui::Id::new("dpl_chat_modes_menu"),
            anchor.rect,
            self.mode_menu_open,
            metadata,
            |ui| {
                hidden
                    .iter()
                    .filter_map(|entry| self.mode_chip(ui, entry, ai, advertised, intents))
                    .collect::<Vec<_>>()
            },
        ) else {
            return;
        };
        trap_focus(ui, focused, &shown.inner);
        if shown.response.clicked_elsewhere() && !anchor.clicked() {
            self.mode_menu_open = false;
            self.composer_focus_requested = true;
        }
    }

    /// One mode chip. Returns its widget id when it can take focus.
    fn mode_chip(
        &self,
        ui: &mut egui::Ui,
        entry: &ComposerEntry,
        ai: Option<&str>,
        advertised: &[Capability],
        intents: &mut Vec<ChatWindowIntent>,
    ) -> Option<egui::Id> {
        let withheld = entry.withheld_reason(ai, advertised);
        let enabled = withheld.is_none();
        let on = self.composer_modes.is_active(entry.slug);
        let response = chat_chip(ui, Some(entry.icon), entry.label, enabled, on);
        let response = match &withheld {
            Some(reason) => response.on_disabled_hover_text(reason),
            None => cl::tip(response, entry.tip),
        };
        let mut metadata = NodeMetadata::new(
            MODE_ID.child(&PilotKey::new(entry.slug)),
            Role::Button,
            entry.label,
        );
        metadata.checked = Some(on);
        metadata.value = Some(if on { "on" } else { "off" }.to_owned());
        metadata.description = Some(withheld.unwrap_or_else(|| entry.tip.to_owned()));
        ui.pilot_track(&response, metadata);
        if response.clicked() {
            intents.push(ChatWindowIntent::ToggleMode(entry.slug));
        }
        enabled.then_some(response.id)
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
            .fill(chat::composer::FILL)
            .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                // Two single-line rows over the input, neither of which ever
                // wraps: the context row (what the message is about, who
                // answers it) and the compose row (the `+` and the modes).
                // A wrapping row grows taller than the composer and pushes
                // the input and Send out of the window; a budgeted row cannot.
                self.composer_context_row(ui, intents);
                self.composer_compose_row(ui, intents);
                ui.horizontal(|ui| {
                    let mut draft = self.current_draft();
                    // SUBMIT ON ENTER — the convention every chat client the
                    // operator uses already obeys.
                    //
                    // The key must be consumed BEFORE the TextEdit runs, or
                    // the widget has already inserted the newline by the time
                    // we could look. Focus therefore comes from memory (last
                    // frame's truth) against an explicit id, which is also
                    // what lets the editor keep focus across a send.
                    //
                    // Enter and Ctrl/Cmd+Enter submit; SHIFT+Enter is left
                    // alone and falls through as a newline, so a multi-line
                    // message is still typeable.
                    let editor_id = egui::Id::new("dpl_chat_composer");
                    let composer_focused = ui.memory(|m| m.has_focus(editor_id));
                    let submit_key = ui.is_enabled()
                        && composer_focused
                        && ui.input_mut(take_composer_submit);
                    let editor = ui.add_sized(
                        // The editor takes what Send leaves, never more, so
                        // Send stays in the window at every width.
                        [
                            (ui.available_width()
                                - SEND_SIZE.x
                                - 2.0 * ui.spacing().item_spacing.x)
                                .max(0.0),
                            52.0,
                        ],
                        egui::TextEdit::multiline(&mut draft)
                            .id(editor_id)
                            .interactive(!self.project_input_locked)
                            .hint_text(self.composer_hint())
                            .desired_rows(2),
                    );
                    let mut metadata =
                        NodeMetadata::new(COMPOSER_ID, Role::TextInput, "Message composer");
                    metadata.editable = !self.project_input_locked;
                    // ALWAYS report the value, empty included. A text field
                    // that is empty has a value of "" — it does not stop
                    // having one, and reporting `None` makes "the composer
                    // cleared" unassertable and reads to an assistive client
                    // as a field with no content rather than an empty one.
                    metadata.value = Some(draft.clone());
                    // The keys are part of the control's contract, so they are
                    // announced rather than hidden in a hint an agent or a
                    // screen reader never sees.
                    metadata.description = Some(COMPOSER_KEYS.to_owned());
                    ui.pilot_track(&editor, metadata);
                    if self.composer_focus_requested && editor.enabled() {
                        editor.request_focus();
                        self.composer_focus_requested = false;
                    }
                    if editor.changed() && !self.project_input_locked {
                        // A fresh keystroke supersedes the refusal line.
                        self.composer_status = None;
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
                                // The Enter-honesty law holds mid-turn too:
                                // the consumed key says why it cannot send.
                                if submit_key {
                                    self.composer_status = Some(
                                        "A response is running — Stop it, or wait for it to finish."
                                            .to_owned(),
                                    );
                                }
                                let stop = cl::tip(
                                    ui.add_enabled(
                                        can_stop,
                                        egui::Button::new("Stop").min_size(SEND_SIZE),
                                    ),
                                    "Stop the answer that is being written",
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
                                // ONE recognizer: the registry census
                                // (provenance law — whole-buffer, at
                                // submit, from the operator's buffer).
                                let command = recognize_command(&draft);
                                // A past transcript is read-only, full stop —
                                // operator commands included. Saying so beats
                                // the transport reason, which would be true
                                // but would not be the ACTUAL reason.
                                let reading_history = self.history_focus.is_some();
                                let archived_session = self.project_session_read_only();
                                let enabled = !reading_history && !archived_session
                                    && (command.is_some()
                                        || (send_target.is_some_and(|(_, _, can_send)| can_send)
                                            && !draft.trim().is_empty()));
                                let reason = if reading_history {
                                    "This is a past transcript — it is read-only.".to_owned()
                                } else if archived_session {
                                    if self.native_sessions.values().any(|status| status.qualified) {
                                        "Resume this saved chat before sending. Your draft stays here.".to_owned()
                                    } else { "Saved project chat is read-only. Start a new chat; native resume is not connected yet.".to_owned() }
                                } else if let Some(c) =
                                    command.as_ref().and_then(|d| dpl_chat_core::command(&d.slug))
                                {
                                    c.blurb.to_owned()
                                } else if draft.trim().is_empty() {
                                    "Type a message first.".to_owned()
                                } else if send_target.is_none() {
                                    "Choose or create a provider session first.".to_owned()
                                } else if send_target.is_some_and(|(_, _, can_send)| !can_send) {
                                    "This provider's live turn transport is not qualified."
                                        .to_owned()
                                } else {
                                    "Send this message.".to_owned()
                                };
                                let send = ui.add_enabled(
                                    enabled,
                                    egui::Button::new("Send").min_size(SEND_SIZE),
                                );
                                let send = cl::tip(send, &reason);
                                let mut metadata =
                                    NodeMetadata::new(SEND_ID, Role::Button, "Send message");
                                metadata.description = Some(reason.clone());
                                ui.pilot_track(&send, metadata);
                                // Enter submits through the SAME branch the
                                // button does — one submit path, so the
                                // command handling and every `enabled` rule
                                // above cannot drift between mouse and
                                // keyboard.
                                let submitted = send.clicked() || (submit_key && enabled);
                                // THE ENTER-HONESTY LAW (BND1): a consumed
                                // Enter that cannot send SAYS WHY — the
                                // reason paints under the composer instead
                                // of dying in a disabled button's tooltip.
                                if submit_key && !enabled {
                                    self.composer_status = Some(reason.clone());
                                }
                                if submitted {
                                    // Keep the caret where the operator is
                                    // typing; a send should not cost focus.
                                    self.composer_focus_requested = true;
                                    self.composer_status = None;
                                }
                                if submitted {
                                    if let Some(draft_cmd) = command {
                                        let synthetic =
                                            egui_pilot_ai::input_is_synthetic(ui.ctx());
                                        if let Some(command) = privileged_command(&draft_cmd) {
                                            // The audited determinism wire,
                                            // byte-identical to before. A
                                            // privileged command always
                                            // dispatches, so the trigger
                                            // text is consumed here.
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
                                                synthetic,
                                            });
                                        } else {
                                            // EVERY census command — the
                                            // brainstorm trigger included —
                                            // crosses to the host, which
                                            // refuses synthetic input and
                                            // validates args BEFORE the
                                            // draft is cleared: a refused
                                            // command never eats the
                                            // operator's typed buffer.
                                            intents.push(ChatWindowIntent::Command {
                                                draft: draft_cmd,
                                                synthetic,
                                            });
                                        }
                                    } else if let Some((provider, session, _)) = send_target {
                                        intents.push(ChatWindowIntent::SendTurn {
                                            provider,
                                            session,
                                            text: draft,
                                            modes: self.composer_modes.clone(),
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
                                egui::Button::new("Send").min_size(SEND_SIZE),
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
                                egui::Button::new("Send").min_size(SEND_SIZE),
                            );
                            let send = cl::tip(send, reason);
                            let mut metadata =
                                NodeMetadata::new(SEND_ID, Role::Button, "Send social message");
                            metadata.description = Some(reason.to_owned());
                            ui.pilot_track(&send, metadata);
                        }
                    }
                });
                // The command strip (BND1-M2): typing `/` reveals the
                // census. Derived from ONE registry, painted above the
                // composer, click-to-fill; a command the census does
                // not hold is never offered.
                if self.domain == ChatDomain::Ai {
                    self.command_strip(ui, intents);
                }
                // The Enter-honesty line: the refusal a keyboard submit
                // earned, painted where the operator is looking.
                if let Some(status) = self.composer_status.clone() {
                    let line = ui.label(
                        RichText::new(&status).small().color(Tone::Amber.color()),
                    );
                    let mut metadata = NodeMetadata::new(
                        COMPOSER_STATUS_ID,
                        Role::Status,
                        "Composer status",
                    );
                    metadata.value = Some(status);
                    ui.pilot_track(&line, metadata);
                }
            });
    }

    /// The typeahead strip (BND1): the matching census commands, their
    /// asks visible, painted in an anchored foreground area above the
    /// composer. Clicking fills the draft; the census is the ONE
    /// source (`dpl_chat_core::matching_commands`).
    fn command_strip(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let draft = self.current_draft();
        let matches = dpl_chat_core::matching_commands(&draft);
        if matches.is_empty() {
            return;
        }
        let Some(provider) = self.runtime.selected_provider_id() else {
            return;
        };
        let anchor = ui.min_rect().left_top();
        let height = 26.0 * matches.len() as f32 + 20.0;
        egui::Area::new(egui::Id::new("chat.composer.command-strip"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(anchor.x, (anchor.y - height - 6.0).max(8.0)))
            .show(ui.ctx(), |ui| {
                let panel = egui::Frame::new()
                    .fill(chat::composer::POPUP)
                    .stroke(Stroke::new(1.0, Tone::Cyan.color()))
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        for spec in &matches {
                            let asks = spec
                                .args
                                .iter()
                                .map(|a| a.ask)
                                .collect::<Vec<_>>()
                                .join(" · ");
                            let row = ui.button(format!("/{} — {}", spec.slug, spec.label));
                            let tip_text = if asks.is_empty() {
                                spec.blurb.to_owned()
                            } else {
                                format!("{} ({asks})", spec.blurb)
                            };
                            let row = cl::tip(row, &tip_text);
                            let mut metadata = NodeMetadata::new(
                                COMMAND_STRIP_ID.child(&PilotKey::new(spec.slug)),
                                Role::Button,
                                spec.label,
                            );
                            metadata.value = Some(spec.slug.to_owned());
                            metadata.description = Some(tip_text);
                            ui.pilot_track(&row, metadata);
                            if row.clicked() {
                                let fill = if spec.args.is_empty() {
                                    format!("/{}", spec.slug)
                                } else {
                                    format!("/{} ", spec.slug)
                                };
                                intents.push(ChatWindowIntent::UpdateAiDraft {
                                    provider,
                                    value: fill,
                                });
                                self.composer_focus_requested = true;
                            }
                        }
                    });
                let mut metadata =
                    NodeMetadata::new(COMMAND_STRIP_ID, Role::Region, "Command strip");
                metadata.value = Some(matches.len().to_string());
                ui.pilot_track(&panel.response, metadata);
            });
    }

    /// The inline brainstorm board (BS1): the host-projected document
    /// painted as an editable phased surface. Every gesture emits a
    /// typed [`dpl_workflows::BrainstormOp`] — the same vocabulary the
    /// assist speaks — and the host applies, saves, and re-projects.
    fn brainstorm_board(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        use dpl_workflows::BrainstormPhase;
        let Some(doc) = self.brainstorm.doc.clone() else {
            return;
        };
        let can_undo = self.brainstorm.can_undo;
        let can_redo = self.brainstorm.can_redo;
        let status = self.brainstorm.status.clone();
        let panel = egui::Frame::new()
            .fill(chat::bubble::NOTICE)
            .stroke(Stroke::new(1.0, Tone::Amber.color()))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                self.brainstorm_header(ui, intents, &doc, can_undo, can_redo, status.as_deref());
                ui.add_space(6.0);
                self.brainstorm_seed_row(ui, intents, &doc);
                match doc.phase {
                    BrainstormPhase::Diverge => self.brainstorm_items(ui, intents, &doc, None),
                    BrainstormPhase::Cluster => self.brainstorm_cluster_rows(ui, intents, &doc),
                    BrainstormPhase::Converge => self.brainstorm_converged(ui, &doc),
                }
                ui.add_space(6.0);
                self.brainstorm_footer(ui, intents, &doc);
            });
        let mut metadata = NodeMetadata::new(BRAINSTORM_PANEL_ID, Role::Region, "Brainstorm board");
        metadata.value = Some(doc.phase.label().to_owned());
        ui.pilot_track(&panel.response, metadata);
        ui.add_space(7.0);
    }

    fn brainstorm_header(
        &mut self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
        doc: &dpl_workflows::BrainstormDoc,
        can_undo: bool,
        can_redo: bool,
        status: Option<&str>,
    ) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("⊞ Brainstorm")
                    .strong()
                    .color(cl::ink::STRONG),
            );
            let phase = ui.label(
                RichText::new(doc.phase.label())
                    .small()
                    .color(cl::ink::MUTED),
            );
            let mut metadata =
                NodeMetadata::new(BRAINSTORM_PHASE_ID, Role::Status, "Brainstorm phase");
            metadata.value = Some(doc.phase.label().to_owned());
            ui.pilot_track(&phase, metadata);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let redo = ui.add_enabled(can_redo, egui::Button::new("Redo"));
                let redo = cl::tip(redo, "Redo the undone board edit.");
                ui.pilot_track(
                    &redo,
                    NodeMetadata::new(BRAINSTORM_REDO_ID, Role::Button, "Redo board edit"),
                );
                if redo.clicked() {
                    intents.push(ChatWindowIntent::BrainstormRedo);
                }
                let undo = ui.add_enabled(can_undo, egui::Button::new("Undo"));
                let undo = cl::tip(undo, "Undo the last board edit.");
                ui.pilot_track(
                    &undo,
                    NodeMetadata::new(BRAINSTORM_UNDO_ID, Role::Button, "Undo board edit"),
                );
                if undo.clicked() {
                    intents.push(ChatWindowIntent::BrainstormUndo);
                }
            });
        });
        if let Some(status) = status {
            ui.label(RichText::new(status).small().color(Tone::Amber.color()));
        }
    }

    fn brainstorm_seed_row(
        &mut self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
        doc: &dpl_workflows::BrainstormDoc,
    ) {
        if doc.seed.is_empty() {
            ui.horizontal(|ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.brainstorm_seed_draft)
                        .hint_text("The massive idea, in your words…")
                        .desired_width((ui.available_width() - 96.0).max(160.0)),
                );
                let edit = cl::tip(edit, "Seed the board — everything breaks down from this.");
                ui.pilot_track(
                    &edit,
                    NodeMetadata::new(BRAINSTORM_SEED_ID, Role::TextInput, "Brainstorm seed"),
                );
                let seed_ready = !self.brainstorm_seed_draft.trim().is_empty();
                let seed = ui.add_enabled(seed_ready, egui::Button::new("Seed"));
                let seed = cl::tip(seed, "Plant the idea on the board.");
                if seed.clicked() {
                    intents.push(ChatWindowIntent::Brainstorm(
                        dpl_workflows::BrainstormOp::Seed {
                            seed: std::mem::take(&mut self.brainstorm_seed_draft),
                        },
                    ));
                }
            });
        } else {
            ui.label(RichText::new(&doc.seed).strong().color(cl::ink::STRONG));
        }
        ui.add_space(4.0);
    }

    /// The editable item list. `within` scopes to one cluster (`Some`)
    /// or to the whole board (`None` — the Diverge view, tree-indented).
    fn brainstorm_items(
        &mut self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
        doc: &dpl_workflows::BrainstormDoc,
        within: Option<&str>,
    ) {
        use dpl_workflows::{BrainstormOp, BrainstormOrigin};
        for item in doc
            .items
            .iter()
            .filter(|i| within.is_none() || i.cluster.as_deref() == within)
        {
            let depth = {
                let mut d = 0usize;
                let mut cursor = item.parent.as_deref();
                while let Some(id) = cursor {
                    d += 1;
                    cursor = doc
                        .items
                        .iter()
                        .find(|i| i.id == id)
                        .and_then(|i| i.parent.as_deref());
                }
                d
            };
            ui.horizontal(|ui| {
                ui.add_space(16.0 * depth as f32);
                if item.origin == BrainstormOrigin::Assist {
                    let mark = ui.label(RichText::new("assist").small().color(Tone::Cyan.color()));
                    cl::tip(mark, "Proposed by assist — edit it and it becomes yours.");
                }
                let draft = self
                    .brainstorm_item_drafts
                    .entry(item.id.clone())
                    .or_insert_with(|| item.text.clone());
                let edit = ui.add(
                    egui::TextEdit::singleline(draft)
                        .desired_width((ui.available_width() - 150.0).max(140.0)),
                );
                let edit = cl::tip(edit, "Edit this idea — commits when you leave the field.");
                let mut metadata = NodeMetadata::new(
                    BRAINSTORM_ITEM_ID.child(&PilotKey::new(item.id.clone())),
                    Role::TextInput,
                    "Brainstorm idea",
                );
                metadata.value = Some(item.text.clone());
                ui.pilot_track(&edit, metadata);
                if edit.lost_focus() {
                    let committed = self
                        .brainstorm_item_drafts
                        .get(&item.id)
                        .cloned()
                        .unwrap_or_default();
                    if committed.trim() != item.text && !committed.trim().is_empty() {
                        intents.push(ChatWindowIntent::Brainstorm(BrainstormOp::EditItem {
                            id: item.id.clone(),
                            text: committed,
                        }));
                    } else {
                        self.brainstorm_item_drafts.remove(&item.id);
                    }
                }
                if doc.phase == dpl_workflows::BrainstormPhase::Diverge {
                    let split = ui.small_button("↳");
                    let split = cl::tip(split, "Break this idea down — add a child under it.");
                    if split.clicked() {
                        let id = next_brainstorm_id(doc);
                        intents.push(ChatWindowIntent::Brainstorm(BrainstormOp::AddItem {
                            id,
                            text: "…".into(),
                            parent: Some(item.id.clone()),
                            origin: BrainstormOrigin::Operator,
                        }));
                    }
                }
                let remove = ui.small_button("✕");
                let remove = cl::tip(remove, "Remove this idea and its whole breakdown.");
                if remove.clicked() {
                    self.brainstorm_item_drafts.remove(&item.id);
                    intents.push(ChatWindowIntent::Brainstorm(BrainstormOp::RemoveItem {
                        id: item.id.clone(),
                    }));
                }
            });
        }
        if within.is_none() {
            ui.horizontal(|ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.brainstorm_add_draft)
                        .hint_text("Add an idea…")
                        .desired_width((ui.available_width() - 96.0).max(140.0)),
                );
                let edit = cl::tip(edit, "Every idea is welcome here — sorting comes later.");
                ui.pilot_track(
                    &edit,
                    NodeMetadata::new(BRAINSTORM_ADD_ID, Role::TextInput, "Add brainstorm idea"),
                );
                let ready = !self.brainstorm_add_draft.trim().is_empty();
                let add = ui.add_enabled(ready, egui::Button::new("Add"));
                let add = cl::tip(add, "Put it on the board.");
                if add.clicked() {
                    let id = next_brainstorm_id(doc);
                    intents.push(ChatWindowIntent::Brainstorm(BrainstormOp::AddItem {
                        id,
                        text: std::mem::take(&mut self.brainstorm_add_draft),
                        parent: None,
                        origin: BrainstormOrigin::Operator,
                    }));
                }
            });
        }
    }

    fn brainstorm_cluster_rows(
        &mut self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
        doc: &dpl_workflows::BrainstormDoc,
    ) {
        use dpl_workflows::BrainstormOp;
        // Unsorted root ideas first — each with its sort-into picker.
        let unsorted: Vec<_> = doc
            .items
            .iter()
            .filter(|i| i.parent.is_none() && i.cluster.is_none())
            .collect();
        if !unsorted.is_empty() {
            ui.label(
                RichText::new("Unsorted — pick a group for each:")
                    .small()
                    .color(cl::ink::MUTED),
            );
            for item in unsorted {
                ui.horizontal(|ui| {
                    if item.origin == dpl_workflows::BrainstormOrigin::Assist {
                        let mark =
                            ui.label(RichText::new("assist").small().color(Tone::Cyan.color()));
                        cl::tip(mark, "Proposed by assist — edit it and it becomes yours.");
                    }
                    ui.label(RichText::new(&item.text).color(cl::ink::STRONG));
                    for cluster in &doc.clusters {
                        let into = ui.small_button(cluster.label.as_str());
                        let into = cl::tip(into, "Sort this idea into the group.");
                        if into.clicked() {
                            intents.push(ChatWindowIntent::Brainstorm(
                                BrainstormOp::AssignCluster {
                                    id: item.id.clone(),
                                    cluster: Some(cluster.id.clone()),
                                },
                            ));
                        }
                    }
                });
            }
            ui.add_space(4.0);
        }
        for cluster in &doc.clusters {
            let heading = ui.label(
                RichText::new(&cluster.label)
                    .strong()
                    .color(Tone::Cyan.color()),
            );
            let mut metadata = NodeMetadata::new(
                BRAINSTORM_CLUSTER_ID.child(&PilotKey::new(cluster.id.clone())),
                Role::Status,
                "Brainstorm group",
            );
            metadata.value = Some(cluster.label.clone());
            ui.pilot_track(&heading, metadata);
            self.brainstorm_items(ui, intents, doc, Some(cluster.id.as_str()));
        }
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.brainstorm_cluster_draft)
                    .hint_text("Name a group…")
                    .desired_width((ui.available_width() - 130.0).max(120.0)),
            );
            cl::tip(edit, "Groups become the beats of your storyboard.");
            let ready = !self.brainstorm_cluster_draft.trim().is_empty();
            let add = ui.add_enabled(ready, egui::Button::new("Add group"));
            let add = cl::tip(add, "Add this group to sort into.");
            if add.clicked() {
                let label = std::mem::take(&mut self.brainstorm_cluster_draft);
                let id = kebab_of(&label);
                intents.push(ChatWindowIntent::Brainstorm(BrainstormOp::AddCluster {
                    id,
                    label,
                }));
            }
        });
    }

    /// The Converge read-through: groups in order, their ideas beneath —
    /// what Promote will turn into intake answers.
    fn brainstorm_converged(&mut self, ui: &mut egui::Ui, doc: &dpl_workflows::BrainstormDoc) {
        for cluster in &doc.clusters {
            ui.label(
                RichText::new(&cluster.label)
                    .strong()
                    .color(Tone::Cyan.color()),
            );
            for item in doc
                .items
                .iter()
                .filter(|i| i.cluster.as_deref() == Some(cluster.id.as_str()))
            {
                ui.label(RichText::new(format!("  · {}", item.text)).color(cl::ink::STRONG));
            }
        }
    }

    fn brainstorm_footer(
        &mut self,
        ui: &mut egui::Ui,
        intents: &mut Vec<ChatWindowIntent>,
        doc: &dpl_workflows::BrainstormDoc,
    ) {
        use dpl_workflows::BrainstormPhase;
        ui.horizontal(|ui| {
            if doc.phase == BrainstormPhase::Converge {
                let promote = ui.add(egui::Button::new("Promote to intake"));
                let promote = cl::tip(
                    promote,
                    "Distill the board into explainer-intake answers, staged for the guided flow.",
                );
                ui.pilot_track(
                    &promote,
                    NodeMetadata::new(BRAINSTORM_PROMOTE_ID, Role::Button, "Promote brainstorm"),
                );
                if promote.clicked() {
                    intents.push(ChatWindowIntent::PromoteBrainstorm);
                }
            } else {
                if doc.phase == BrainstormPhase::Diverge && self.assist_enabled {
                    let suggest = ui.add(egui::Button::new("✦ Suggest ideas"));
                    let suggest = cl::tip(
                        suggest,
                        "Ask the assist for angles on your seed — proposals arrive marked, editable, yours to keep or cut.",
                    );
                    ui.pilot_track(
                        &suggest,
                        NodeMetadata::new(
                            BRAINSTORM_SUGGEST_ID,
                            Role::Button,
                            "Suggest brainstorm ideas",
                        ),
                    );
                    if suggest.clicked() {
                        intents.push(ChatWindowIntent::BrainstormAssist);
                    }
                }
                let label = match doc.phase {
                    BrainstormPhase::Diverge => "Start grouping",
                    _ => "Lock the order",
                };
                let advance = ui.add(egui::Button::new(label));
                let advance = cl::tip(
                    advance,
                    "Step the board to its next phase — it will say what's missing.",
                );
                ui.pilot_track(
                    &advance,
                    NodeMetadata::new(BRAINSTORM_ADVANCE_ID, Role::Button, "Advance brainstorm"),
                );
                if advance.clicked() {
                    intents.push(ChatWindowIntent::Brainstorm(
                        dpl_workflows::BrainstormOp::AdvancePhase,
                    ));
                }
            }
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
        if self.domain == ChatDomain::Ai && self.project_session_read_only() {
            return "Saved project chat — read-only";
        }
        if self.domain == ChatDomain::Ai && self.history_focus.is_some() {
            return "Past transcript — read-only";
        }
        match self.domain {
            ChatDomain::Ai => "Message… — Enter sends, Shift+Enter new line",
            ChatDomain::Content => "Browsing content — no conversation here",
            ChatDomain::Social => "Message… — Enter sends, Shift+Enter new line",
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
                    .fill(chat::panel::OVERLAY)
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
                            RichText::new(provider_status(&provider.registration))
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
                                provider.sessions.len(),
                                if provider.sessions.len() == 1 {
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
                        let new_chat = cl::tip(
                            ui.add_enabled(can_start, egui::Button::new("+  New chat")),
                            "Start a new chat with this provider",
                        );
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
                            let continue_setup = cl::tip(
                                ui.button("Continue setup in conversation"),
                                "Finish connecting this AI in the chat",
                            );
                            ui.pilot_track(
                                &continue_setup,
                                NodeMetadata::new(
                                    PROVIDER_OVERLAY_ID.child(&PilotKey::new("continue-setup")),
                                    Role::Button,
                                    "Continue setup in conversation",
                                ),
                            );
                            if continue_setup.clicked() {
                                intents.push(ChatWindowIntent::SelectProvider(*provider_id));
                                intents.push(ChatWindowIntent::SetOverlay(None));
                            }
                        }
                        ui.label(RichText::new("♙  Account").color(cl::ink::BASE));
                        ui.horizontal(|ui| {
                            let icon = ui.text_style_height(&egui::TextStyle::Body);
                            let _ = cl::icon_label(ui, "diagnostics", icon, cl::ink::BASE);
                            ui.label(RichText::new("Diagnostics").color(cl::ink::BASE));
                        });
                        let close = cl::tip(ui.button("Close"), "Close this card");
                        ui.pilot_track(
                            &close,
                            NodeMetadata::new(
                                PROVIDER_OVERLAY_ID.child(&PilotKey::new("close")),
                                Role::Button,
                                "Close provider card",
                            ),
                        );
                        if close.clicked() {
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

    /// THE PROVIDER CATALOG — every provider, one overlay, the full window.
    ///
    /// The carousel layout carrying every part of the catalog, top to bottom:
    /// the sessions running now; SUBSCRIPTIONS as one row of large tiles
    /// (few, already paid for); API as a tile grid (many, billed per call);
    /// the Clyffy house adapter; ADD A PROVIDER (Local, Custom); and the
    /// adapters still to come, last and quiet.
    ///
    /// Selecting a subscription is just `SelectProvider`, as from the rail:
    /// an unactivated provider then meets its connection card in the
    /// conversation pane, so this surface builds no sign-in UI of its own.
    fn provider_catalog_overlay(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if self.domain_states.ai.overlay != Some(DomainOverlay::ProviderCatalog) {
            return;
        }
        let providers = self.runtime.provider_views();
        let screen = ui.ctx().content_rect();
        let id = egui::Id::new("chat.provider.catalog");
        let plate = cl::compose(Tone::Cyan, "vibron", &[], cl::TraitState::Idle);
        let edge = cl::compose(Tone::Cyan, "vibron", &[], cl::TraitState::Active).edge;
        // The overlay sits a chrome step inside the window on every side.
        let inset = cl::space::S2;
        let margin = egui::vec2(cl::space::S4, cl::space::S3);
        let mut frame = egui::Frame::new()
            .fill(plate.fill)
            .stroke(edge)
            .corner_radius(CornerRadius::same(plate.radius))
            .inner_margin(egui::Margin::symmetric(margin.x as i8, margin.y as i8));
        if let Some(shadow) = plate.shadows.first() {
            frame = frame.shadow(*shadow);
        }

        let modal = egui::Modal::new(id)
            .backdrop_color(cl::scrim(cl::SCRIM_MEDIA_A))
            .frame(frame)
            .area(
                egui::Modal::default_area(id)
                    .fixed_pos(screen.min + egui::Vec2::splat(inset))
                    .default_width(screen.width() - 2.0 * inset),
            )
            .show(ui.ctx(), |ui| {
                let chrome = 2.0 * (inset + edge.width);
                ui.set_width(screen.width() - chrome - 2.0 * margin.x);
                ui.set_max_height(screen.height() - chrome - 2.0 * margin.y);
                let panel = ui
                    .vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("AI Providers")
                                    .strong()
                                    .color(cl::ink::STRONG),
                            );
                            ui.label(
                                RichText::new("subscriptions you hold · endpoints you saved")
                                    .small()
                                    .color(cl::ink::SUBTLE),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let close = cl::tip(
                                    ui.button("Close"),
                                    "Close the provider list and go back to your chat",
                                );
                                ui.pilot_track(
                                    &close,
                                    NodeMetadata::new(
                                        PROVIDER_CATALOG_CLOSE_ID,
                                        Role::Button,
                                        "Close AI providers",
                                    ),
                                );
                                if close.clicked() {
                                    intents.push(ChatWindowIntent::SetOverlay(None));
                                }
                            });
                        });
                        ui.add_space(cl::space::S2);
                        egui::ScrollArea::vertical()
                            .id_salt("chat.provider.catalog.scroll")
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                self.catalog_sections(ui, &providers, intents);
                            });
                    })
                    .response;
                ui.pilot_track(
                    &panel,
                    NodeMetadata::new(PROVIDER_CATALOG_ID, Role::Dialog, "AI provider catalog"),
                );
            });

        if modal.should_close() {
            intents.push(ChatWindowIntent::SetOverlay(None));
        }
    }

    /// The catalog's sections, in the order the operator reads them.
    fn catalog_sections(
        &self,
        ui: &mut egui::Ui,
        providers: &[ProviderView],
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        // ── the sessions running now ─────────────────────────────────────
        section_heading(ui, "CURRENT ACTIVE SESSION(S)", None);
        let active = providers
            .iter()
            .filter(|provider| {
                provider.sessions.iter().any(|session| {
                    session.group == SessionGroup::Active || session.state == SessionState::Running
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
                let descriptor = provider.registration.manifest.descriptor;
                ui.label(
                    RichText::new(format!(
                        "{}  {} · {} session{}",
                        descriptor.glyph,
                        descriptor.label,
                        provider.sessions.len(),
                        if provider.sessions.len() == 1 {
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

        // ── subscriptions ────────────────────────────────────────────────
        ui.add_space(cl::space::S3);
        section_heading(ui, "SUBSCRIPTIONS", Some(("already paid for", false)));
        let row = cl::patterns::carousel_row(
            ui,
            "chat.provider.catalog.subscriptions",
            cl::patterns::lift_reserve(SUB_TILE).y + cl::space::S3,
            |ui| {
                for provider in providers {
                    self.subscription_tile(ui, provider, intents);
                }
            },
        );
        // The wheel and a drag already scroll the row; the words exist so
        // the row ANNOUNCES that there is more of it.
        let more = match (row.can_left, row.can_right) {
            (true, true) => Some("more on both sides — scroll sideways"),
            (true, false) => Some("more to the left — scroll sideways"),
            (false, true) => Some("more to the right — scroll sideways"),
            (false, false) => None,
        };
        if let Some(more) = more {
            ui.label(RichText::new(more).small().color(cl::ink::SUBTLE));
        }

        // ── API ──────────────────────────────────────────────────────────
        ui.add_space(cl::space::S3);
        section_heading(ui, "API", Some(("usage-based · billed per call", true)));
        if self.provider_profiles.is_empty() {
            ui.label(
                RichText::new("No Local or Custom profiles yet")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
        } else {
            let _ = cl::patterns::tile_grid(ui, API_TILE, API_ROWS_FLOOR, |ui| {
                for profile in &self.provider_profiles {
                    self.api_tile(ui, profile);
                }
            });
        }

        // ── the house adapter ────────────────────────────────────────────
        ui.add_space(cl::space::S3);
        section_heading(ui, "DEVPULSE", Some(("the house adapter", false)));
        for supplement in SUPPLEMENTS
            .iter()
            .filter(|supplement| supplement.provider_id == CLYFFY_PROVIDER)
        {
            unqualified_adapter_row(ui, supplement);
        }

        // ── ways in ──────────────────────────────────────────────────────
        ui.add_space(cl::space::S3);
        section_heading(ui, "ADD A PROVIDER", None);
        for supplement in SUPPLEMENTS
            .iter()
            .filter(|supplement| matches!(supplement.action, CatalogSupplementAction::Configure(_)))
        {
            let response = cl::tip(
                catalog_supplement_row(ui, supplement, true),
                &format!("{} Tap to fill in its settings.", supplement.summary),
            );
            let mut metadata = NodeMetadata::new(
                PROVIDER_CATALOG_ID.child(&PilotKey::new(supplement.provider_id.0)),
                Role::Button,
                supplement.label,
            );
            metadata.description = Some(supplement.summary.to_owned());
            ui.pilot_track(&response, metadata);
            if response.clicked() {
                if let CatalogSupplementAction::Configure(kind) = supplement.action {
                    intents.push(ChatWindowIntent::UpdateProviderProfileDraft(
                        ProviderProfileDraft::for_kind(kind),
                    ));
                    intents.push(ChatWindowIntent::SetOverlay(Some(
                        DomainOverlay::ProviderConfiguration(kind),
                    )));
                }
            }
        }

        // ── adapters still to come — last, and quiet ─────────────────────
        let coming = SUPPLEMENTS
            .iter()
            .filter(|supplement| {
                supplement.action == CatalogSupplementAction::FutureAdapter
                    && supplement.provider_id != CLYFFY_PROVIDER
            })
            .collect::<Vec<_>>();
        if !coming.is_empty() {
            ui.add_space(cl::space::S3);
            section_heading(
                ui,
                "COMING LATER",
                Some(("declared, not qualified yet", false)),
            );
            for supplement in coming {
                unqualified_adapter_row(ui, supplement);
            }
        }
    }

    /// One subscription, as a large tile carrying its live presence.
    fn subscription_tile(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let descriptor = provider.registration.manifest.descriptor;
        let selected = self.runtime.selected_provider_id() == Some(descriptor.id);
        let tone = provider_tone(descriptor.accent);
        let status = provider_status(&provider.registration);
        let presence = activation_color(&provider.registration.activation, tone);
        let (response, ()) = cl::patterns::lift_tile(
            ui,
            ("chat.provider.catalog.subscription", descriptor.id.0),
            SUB_TILE,
            selected,
            tone,
            |ui, style| {
                ui.spacing_mut().item_spacing.y = cl::space::S1;
                provider_mark(ui, descriptor.accent);
                ui.label(
                    RichText::new(descriptor.short_label)
                        .strong()
                        .color(style.ink),
                );
                ui.horizontal(|ui| {
                    status_dot(ui, presence);
                    ui.label(RichText::new(&status).small().color(presence));
                });
            },
        );
        let response = cl::tip(
            response,
            &format!(
                "{} — {status}. {} Tap to chat with it.",
                descriptor.label, descriptor.summary
            ),
        );
        let mut metadata = NodeMetadata::new(
            PROVIDER_CATALOG_ID.child(&PilotKey::new(descriptor.id.0)),
            Role::Tab,
            descriptor.label,
        );
        metadata.selected = Some(selected);
        metadata.value = Some(status);
        metadata.description = Some(descriptor.summary.to_owned());
        ui.pilot_track(&response, metadata);
        if response.clicked() {
            intents.push(ChatWindowIntent::SelectProvider(descriptor.id));
            intents.push(ChatWindowIntent::SetOverlay(None));
        }
    }

    /// One saved endpoint, as a compact tile in the API grid. It reports;
    /// it does not act — a saved profile cannot chat until its transport
    /// adapter is qualified, and the tile says exactly that.
    fn api_tile(&self, ui: &mut egui::Ui, profile: &ProviderProfile) {
        let (response, ()) = cl::patterns::lift_tile(
            ui,
            ("chat.provider.catalog.api", profile.key.as_str()),
            API_TILE,
            false,
            Tone::Cyan,
            |ui, _| {
                ui.label(
                    RichText::new(&profile.display_name)
                        .small()
                        .strong()
                        .color(cl::ink::STRONG),
                );
                ui.label(
                    RichText::new(profile.kind.label())
                        .small()
                        .color(Tone::Amber.color()),
                );
            },
        );
        let model = if profile.model.is_empty() {
            "provider default model"
        } else {
            profile.model.as_str()
        };
        let response = cl::tip(
            response,
            &format!(
                "{} · {model}\n{}",
                profile.endpoint,
                profile.activation_reason()
            ),
        );
        let mut metadata = NodeMetadata::new(
            PROVIDER_CATALOG_ID
                .child(&PilotKey::new("api"))
                .child(&PilotKey::new(profile.key.as_str())),
            Role::Status,
            profile.display_name.clone(),
        );
        metadata.value = Some(format!("{} · usage-based", profile.kind.label()));
        metadata.description = Some(profile.activation_reason().to_owned());
        ui.pilot_track(&response, metadata);
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
                    .fill(chat::panel::RAISED)
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
                            let save = cl::tip(
                                ui.add_enabled(
                                    validation.is_ok(),
                                    egui::Button::new("Save profile"),
                                ),
                                "Save this endpoint so you can pick it later",
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
                            let back = cl::tip(ui.button("Back"), "Back to every AI provider");
                            ui.pilot_track(
                                &back,
                                NodeMetadata::new(
                                    PROVIDER_CONFIG_ID.child(&PilotKey::new("back")),
                                    Role::Button,
                                    "Back to AI providers",
                                ),
                            );
                            if back.clicked() {
                                intents.push(ChatWindowIntent::SetOverlay(Some(
                                    DomainOverlay::ProviderCatalog,
                                )));
                            }
                            let cancel =
                                cl::tip(ui.button("Cancel"), "Close without saving anything");
                            ui.pilot_track(
                                &cancel,
                                NodeMetadata::new(
                                    PROVIDER_CONFIG_ID.child(&PilotKey::new("cancel")),
                                    Role::Button,
                                    "Cancel provider setup",
                                ),
                            );
                            if cancel.clicked() {
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
            .fill(chat::panel::CARD)
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
        let toggle = cl::tip(
            icon_word_button(
                ui,
                if expanded {
                    "group-open"
                } else {
                    "group-closed"
                },
                if expanded {
                    "Hide activation trace"
                } else {
                    "Show activation trace"
                },
                true,
                expanded,
            ),
            "What happened, step by step, while this AI was connecting",
        );
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
            .fill(chat::panel::INSET)
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
                let copy = cl::tip(ui.button("Copy code"), "Copy the sign-in code");
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
                let reopen = cl::tip(
                    ui.button("Reopen in default browser"),
                    "Open the sign-in page again in your web browser",
                );
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
            let response = ui
                .add_enabled(false, egui::Button::new(&label))
                .on_disabled_hover_text("Checking the account — this takes a moment");
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
                provider_sign_in_button(ui, provider_id, label, intents);
            }
            Some(RecoveryActionKind::OpenApp) => {
                let label = recovery
                    .map(|action| action.label.as_str())
                    .unwrap_or("Open app");
                let response = cl::tip(
                    ui.button(label),
                    "Open this app on your computer, or bring it to the front if it is already open",
                );
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(
                        PROVIDER_SETUP_ID.child(&PilotKey::new(provider_id.0)),
                        Role::Button,
                        label,
                    ),
                );
                if response.clicked() {
                    intents.push(ChatWindowIntent::OpenProviderApp(provider_id));
                }
            }
            Some(RecoveryActionKind::InstallInstructions | RecoveryActionKind::UpdateProvider) => {
                let label = recovery
                    .map(|action| action.label.as_str())
                    .unwrap_or("Provider setup");
                let response = cl::tip(
                    ui.button(label),
                    "See how to install or update this AI on this computer",
                );
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
                let refresh = cl::tip(
                    ui.button("Check installation"),
                    "Look again for this AI on this computer",
                );
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
                let response = ui
                    .add_enabled(false, egui::Button::new("Connecting to provider…"))
                    .on_disabled_hover_text("Waiting for the AI's own sign-in to start");
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
                    .fill(chat::panel::RAISED)
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
                            let image_mode = cl::tip(
                                icon_word_button(ui, "media", "Image", true, true),
                                "Make a picture from your prompt",
                            );
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
                                .fill(chat::panel::CARD)
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
                                                    cl::tip(
                                                        ui.small_button("Save to library"),
                                                        "Keep this picture in your library",
                                                    );
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
                                                let discard = cl::tip(
                                                    ui.small_button("Ignore"),
                                                    "Let this draft go",
                                                );
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
                        let close = cl::tip(ui.button("Close"), "Close image making");
                        ui.pilot_track(
                            &close,
                            NodeMetadata::new(
                                GENERATION_PANEL_ID.child(&PilotKey::new("close")),
                                Role::Button,
                                "Close image making",
                            ),
                        );
                        if close.clicked() {
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
            let close = cl::tip(ui.button("Close picker"), "Close this list");
            ui.pilot_track(
                &close,
                NodeMetadata::new(
                    PilotId::new("chat.sidebar.picker.close"),
                    Role::Button,
                    "Close picker",
                ),
            );
            if close.clicked() {
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
                .backdrop_color(cl::scrim(cl::SCRIM_MEDIA_A))
                .frame(
                    egui::Frame::new()
                        .fill(chat::panel::CARD)
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
                                let approve = cl::tip(ui.button(approve_label), "Say yes to this");
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
                                    let always = cl::tip(
                                        ui.button(always_label),
                                        "Say yes now and from now on",
                                    );
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
                                let reject = cl::tip(ui.button(reject_label), "Say no to this");
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
    let style = cl::compose(
        provider_tone(supplement.accent),
        "vibron",
        &[],
        cl::TraitState::Idle,
    );
    let mut frame = egui::Frame::new()
        .fill(style.fill)
        .stroke(style.edge)
        .corner_radius(CornerRadius::same(style.radius))
        .inner_margin(egui::Margin::symmetric(
            cl::space::S2 as i8,
            cl::space::S1 as i8,
        ));
    if let Some(shadow) = style.shadows.first() {
        frame = frame.shadow(*shadow);
    }
    let response = frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                provider_mark(ui, supplement.accent);
                ui.vertical(|ui| {
                    ui.label(RichText::new(supplement.label).small().strong().color(
                        if clickable {
                            cl::ink::STRONG
                        } else {
                            cl::ink::MUTED
                        },
                    ));
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

/// A declared adapter that is honestly not qualified yet: shown, never
/// actionable.
fn unqualified_adapter_row(ui: &mut egui::Ui, supplement: &CatalogSupplement) {
    let response = catalog_supplement_row(ui, supplement, false);
    let mut metadata = NodeMetadata::new(
        PROVIDER_CATALOG_ID.child(&PilotKey::new(supplement.provider_id.0)),
        Role::Status,
        supplement.label,
    );
    metadata.description = Some(supplement.summary.to_owned());
    metadata.value = Some("Adapter not qualified".to_owned());
    ui.pilot_track(&response, metadata);
}

/// A section heading inside the catalog: the name, and what the section
/// costs or is. A cost qualifier is never abbreviated away and wears the
/// warning tone.
fn section_heading(ui: &mut egui::Ui, name: &str, qualifier: Option<(&str, bool)>) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(name)
                .small()
                .strong()
                .color(Tone::Cyan.color()),
        );
        if let Some((qualifier, costs)) = qualifier {
            ui.label(RichText::new(qualifier).small().color(if costs {
                Tone::Amber.color()
            } else {
                cl::ink::SUBTLE
            }));
        }
    });
    ui.add_space(cl::space::S1);
}

/// The subscriptions the rail lists: only what is ACTIVATED, in registration
/// order so the rail never reshuffles under the operator's finger. Everything
/// else is one tap away in the catalog.
fn rail_subscriptions(providers: &[ProviderView]) -> Vec<&ProviderView> {
    providers
        .iter()
        .filter(|provider| provider.registration.activation.is_activated())
        .collect()
}

/// What a chip's group says while it has no entries. The chip stands first
/// and the group fills as the operator turns things on, so "empty" is an
/// ordinary state, not a failure — it has to read as one.
fn group_awaiting(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(cl::space::S3);
        ui.label(RichText::new(text).small().italics().color(cl::ink::SUBTLE));
    });
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
        .flat_map(|provider| &provider.sessions)
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

/// The one sign-in control a provider's recovery offers — on the connection
/// card and on the strip above a saved transcript alike, so both carry the
/// same pilot id, words and tooltip.
fn provider_sign_in_button(
    ui: &mut egui::Ui,
    provider: ProviderId,
    label: &str,
    intents: &mut Vec<ChatWindowIntent>,
) {
    let response = cl::tip(
        ui.button(label),
        "Sign in through the provider's own app in your browser. Your password and subscription stay with the provider.",
    );
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            PROVIDER_LOGIN_ID.child(&PilotKey::new(provider.0)),
            Role::Button,
            label,
        ),
    );
    if response.clicked() {
        intents.push(ChatWindowIntent::ActivateProvider(provider));
    }
}

/// A signed-out provider's saved chats stay on screen, so its sign-in cannot
/// live on the connection card that would hide them. This strip carries the
/// provider's own reason and its sign-in control above the transcript, on the
/// vibron plate in the provider's tone. Nothing is drawn while the provider is
/// activated or offers no sign-in.
fn provider_sign_in_strip(
    ui: &mut egui::Ui,
    provider: &ProviderView,
    intents: &mut Vec<ChatWindowIntent>,
) {
    let activation = &provider.registration.activation;
    if activation.is_activated() {
        return;
    }
    let Some(action) = activation.recovery().first().filter(|action| {
        matches!(
            action.kind,
            RecoveryActionKind::SignIn | RecoveryActionKind::Retry
        )
    }) else {
        return;
    };
    let descriptor = provider.registration.manifest.descriptor;
    let tone = provider_tone(descriptor.accent);
    let fluid = cl::Fluid::of(ui.max_rect());
    cl::vibron(tone, cl::TraitState::Idle).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{} is signed out", descriptor.short_label))
                    .strong()
                    .size(fluid.fs_body())
                    .color(cl::ink::STRONG),
            );
            if let Some(reason) = activation.reason() {
                ui.label(
                    RichText::new(reason)
                        .size(fluid.fs_body_sm())
                        .color(cl::ink::MUTED),
                );
            }
            ui.add_space(fluid.sp_2());
            provider_sign_in_button(ui, descriptor.id, &action.label, intents);
        });
    });
    ui.add_space(fluid.sp_1());
}

/// One slot in the title-bar tab strip. Every slot in a strip shares the
/// width the strip's budget gave it; its insets and type come from Fluid.
struct ModeTab {
    width: f32,
    height: f32,
    font: egui::FontId,
    fluid: cl::Fluid,
}

impl ModeTab {
    /// Everything in a tab but its label: the inset, the dot, the gap after
    /// the dot and the closing inset.
    fn chrome(fluid: &cl::Fluid) -> f32 {
        fluid.sp_2() * 3.0 + fluid.sp_1()
    }

    fn show(
        &self,
        ui: &mut egui::Ui,
        path: impl Into<PilotPath>,
        label: &str,
        selected: bool,
    ) -> egui::Response {
        let path = path.into();
        let pad = self.fluid.sp_2();
        let dot = self.fluid.sp_1();
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(self.width, self.height), Sense::click());
        let fill = if selected {
            chat::tab::SELECTED
        } else if response.hovered() {
            chat::tab::HOVER
        } else {
            chat::tab::IDLE
        };
        let stroke = if response.has_focus() {
            Stroke::new(1.5, Tone::Cyan.color())
        } else if selected {
            Stroke::new(1.0, chat::tab::SELECTED_EDGE)
        } else {
            Stroke::new(1.0, cl::surface::HAIRLINE)
        };
        let radius = self.fluid.r_md().round() as u8;
        // Everything a tab paints stays inside its own slot.
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        painter.rect(
            rect,
            CornerRadius {
                nw: radius,
                ne: radius,
                sw: 0,
                se: 0,
            },
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        if selected {
            let underline = Stroke::new(2.5, Tone::Cyan.color());
            let y = rect.top() + underline.width * 0.5;
            painter.line_segment(
                [
                    egui::pos2(rect.left() + pad, y),
                    egui::pos2(rect.right() - pad, y),
                ],
                underline,
            );
        }
        // Violet is Social's colour alone; AI and every provider tab under it
        // are cyan, so a provider tab never reads as the other domain.
        let accent = if path == SOCIAL_MODE_ID.path() {
            Tone::Violet.color()
        } else {
            Tone::Cyan.color()
        };
        painter.circle_filled(
            egui::pos2(rect.left() + pad + dot * 0.5, rect.center().y),
            dot * 0.5,
            if selected { accent } else { cl::ink::SUBTLE },
        );
        let ink = if selected {
            cl::ink::STRONG
        } else {
            cl::ink::MUTED
        };
        let text_left = rect.left() + pad + dot + pad;
        let mut job = egui::text::LayoutJob::single_section(
            label.to_owned(),
            egui::TextFormat::simple(self.font.clone(), ink),
        );
        job.wrap =
            egui::text::TextWrapping::truncate_at_width((rect.right() - pad - text_left).max(0.0));
        let galley = painter.layout_job(job);
        painter.galley(
            egui::pos2(text_left, rect.center().y - galley.size().y * 0.5),
            galley,
            ink,
        );
        let mut metadata = NodeMetadata::new(path, Role::Tab, label.to_owned());
        metadata.selected = Some(selected);
        // Said in words as well as flagged, so a reader that only hears the
        // value still hears which tab is open.
        metadata.value = Some(if selected { "selected" } else { "not selected" }.to_owned());
        ui.pilot_track(&response, metadata);
        response
    }
}

/// The four window buttons, right to left from the strip's edge.
fn window_controls(ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
    if quiet_button(ui, CLOSE_ID, "×", "Close Chat").clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let maximized = ui
        .ctx()
        .input(|input| input.viewport().maximized.unwrap_or(false));
    if quiet_icon_button(
        ui,
        MAXIMIZE_ID,
        if maximized {
            "window-restore"
        } else {
            "window-maximize"
        },
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
}

/// How wide `text` lays out in `font`, unwrapped.
fn text_width(ui: &egui::Ui, text: &str, font: egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, cl::ink::STRONG)
        .size()
        .x
}

/// The title bar's two status dots, laid out in the caller's direction.
fn status_dots(ui: &mut egui::Ui, fluid: &cl::Fluid) {
    status_dot(ui, Tone::Lime.color());
    ui.add_space(fluid.sp_1());
    status_dot(ui, Tone::Amber.color());
}

/// What [`status_dots`] allocates, so a budget cannot drift from the drawing.
fn status_dots_width(ui: &egui::Ui, fluid: &cl::Fluid) -> f32 {
    STATUS_DOT_WIDTH * 2.0 + fluid.sp_1() + ui.spacing().item_spacing.x * 2.0
}

/// The DevPULSE mark, `size` square: a cyan-edged plate with an amber core,
/// in the proportions of the original 18-point mark.
fn brand_mark(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
    let plate = rect.shrink(size / 9.0);
    ui.painter().rect(
        plate,
        CornerRadius::same((size / 9.0).round() as u8),
        chat::panel::MARK,
        Stroke::new(1.0, Tone::Cyan.color()),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .circle_filled(rect.center(), plate.width() / 5.6, Tone::Amber.color());
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
    ui.painter().circle_filled(center, 14.0, chat::rail::MARK);
    ui.painter()
        .circle_stroke(center, 14.0, Stroke::new(1.0, chat::panel::EDGE));
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
        .fill(chat::rail::HEADER)
        .stroke(Stroke::new(1.0, chat::panel::EDGE))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(9, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (avatar, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), Sense::hover());
                ui.painter()
                    .circle_filled(avatar.center(), 16.0, chat::rail::AVATAR);
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

/// The slot one status dot allocates in a row.
const STATUS_DOT_WIDTH: f32 = 10.0;

fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(STATUS_DOT_WIDTH, 18.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

/// A window control drawn as a registry icon rather than a font glyph: the
/// square glyphs maximize and restore used are missing from the bundled
/// fonts and painted as empty boxes.
fn quiet_icon_button(
    ui: &mut egui::Ui,
    id: PilotId,
    icon: &'static str,
    tooltip: &str,
) -> egui::Response {
    let response = cl::tip(icon_button(ui, icon), tooltip);
    ui.pilot_track(
        &response,
        NodeMetadata::new(id, Role::Button, tooltip.to_owned()),
    );
    response
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
            chat::rail::SELECTED
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0,
            if selected {
                chat::rail::SELECTED_EDGE
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

/// One filter row of the Rooms & roles library: "All" plus every value the
/// library actually holds (only what exists is offered).
fn library_filter<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    filter: &mut Option<T>,
    values: &[T],
    key: &str,
    // A value's stable id (its schema name) and its plain-word label.
    name: impl Fn(T) -> (&'static str, &'static str),
) {
    let all = cl::tip(
        ui.selectable_label(filter.is_none(), "All"),
        "Show every one",
    );
    ui.pilot_track(
        &all,
        NodeMetadata::new(
            ROOM_LIBRARY_ID
                .child(&PilotKey::new("filter"))
                .child(&PilotKey::new(key))
                .child(&PilotKey::new("all")),
            Role::Button,
            format!("All {key}s"),
        ),
    );
    if all.clicked() {
        *filter = None;
    }
    for &value in values {
        let (id, label) = name(value);
        let chip = cl::tip(
            ui.selectable_label(*filter == Some(value), label),
            "Show only these",
        );
        ui.pilot_track(
            &chip,
            NodeMetadata::new(
                ROOM_LIBRARY_ID
                    .child(&PilotKey::new("filter"))
                    .child(&PilotKey::new(key))
                    .child(&PilotKey::new(id)),
                Role::Button,
                label,
            ),
        );
        if chip.clicked() {
            *filter = Some(value);
        }
    }
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

/// A small button carrying a registry icon ahead of its word.
///
/// The icon is DRAWN — a ColdLight mesh painted into a slot the button
/// reserves — never a font glyph: the glyphs these buttons used to carry are
/// in none of the bundled fonts and rendered as empty boxes.
fn icon_word_button(
    ui: &mut egui::Ui,
    icon: &'static str,
    label: &str,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    let slot = egui::Id::new(("chat.icon-word-button", icon, label));
    let size = egui::Vec2::splat(ui.text_style_height(&egui::TextStyle::Small));
    let laid = ui
        .add_enabled_ui(enabled, |ui| {
            egui::Button::new((egui::Atom::custom(slot, size), RichText::new(label).small()))
                .selected(selected)
                .atom_ui(ui)
        })
        .inner;
    if let Some(rect) = laid.rect(slot) {
        let ink = if enabled {
            cl::ink::BASE
        } else {
            cl::ink::SUBTLE
        };
        cl::draw_icon(ui.painter(), icon, rect, ink);
    }
    laid.response
}

/// The pilot id a `+` entry answers to. The four the composer has always
/// had keep their ids, so a scenario that drove them still finds them.
fn menu_entry_id(slug: &str) -> PilotPath {
    match slug {
        "attach" => ATTACHMENT_ID.path(),
        "media" => MEDIA_ID.path(),
        "component" => INTERACTION_COMPONENT_ID.path(),
        "voice" => VOICE_ID.path(),
        other => COMPOSER_MENU_ID.child(&PilotKey::new(other)),
    }
}

/// How much of the AI selector the composer's context row has room for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorShown {
    Full,
    Short,
    Hidden,
}

/// Keep focus inside an open popup: focus that is elsewhere (or nowhere)
/// comes back to its first entry, which is also where opening it lands.
fn trap_focus(ui: &egui::Ui, focused: Option<egui::Id>, entries: &[egui::Id]) {
    if focused.is_none_or(|focus| !entries.contains(&focus)) {
        if let Some(first) = entries.first() {
            ui.memory_mut(|memory| memory.request_focus(*first));
        }
    }
}

/// The width [`chat_chip`] lays out at, from the style's own measures, so a
/// row can budget its chips before drawing any of them.
fn chip_width(ui: &egui::Ui, icon: Option<&str>, label: &str) -> f32 {
    let small = egui::TextStyle::Small.resolve(ui.style());
    let icon = icon.map_or(0.0, |_| {
        ui.text_style_height(&egui::TextStyle::Small) + ui.spacing().icon_spacing
    });
    icon + text_width(ui, label, small) + 2.0 * ui.spacing().button_padding.x
}

/// A small button that is only a registry icon, drawn rather than typed.
/// Callers owe the tooltip and the pilot metadata.
fn icon_button(ui: &mut egui::Ui, icon: &'static str) -> egui::Response {
    let slot = egui::Id::new(("chat.icon-button", icon));
    let size = egui::Vec2::splat(ui.text_style_height(&egui::TextStyle::Body));
    let laid = egui::Button::new(egui::Atom::custom(slot, size))
        .small()
        .atom_ui(ui);
    if let Some(rect) = laid.rect(slot) {
        let ink = if laid.response.hovered() {
            cl::ink::STRONG
        } else {
            cl::ink::BASE
        };
        cl::draw_icon(ui.painter(), icon, rect.shrink(rect.width() * 0.15), ink);
    }
    laid.response
}

/// A chip in the chat's colour roles: raised when off, lit with a bright
/// edge when on. `icon` is a registry icon drawn ahead of the word.
fn chat_chip(
    ui: &mut egui::Ui,
    icon: Option<&'static str>,
    label: &str,
    enabled: bool,
    on: bool,
) -> egui::Response {
    let slot = egui::Id::new(("chat.chip", icon, label));
    let size = egui::Vec2::splat(ui.text_style_height(&egui::TextStyle::Small));
    let word = RichText::new(label).small().color(if on {
        cl::ink::STRONG
    } else if enabled {
        cl::ink::BASE
    } else {
        cl::ink::SUBTLE
    });
    let (fill, edge) = if on {
        (chat::chip::ON, chat::chip::ON_EDGE)
    } else {
        (chat::chip::IDLE, chat::panel::EDGE)
    };
    let laid = ui
        .add_enabled_ui(enabled, |ui| match icon {
            Some(_) => egui::Button::new((egui::Atom::custom(slot, size), word))
                .fill(fill)
                .stroke(Stroke::new(1.0, edge))
                .corner_radius(CornerRadius::same(cl::RADIUS_SM))
                .small()
                .atom_ui(ui),
            None => egui::Button::new(word)
                .fill(fill)
                .stroke(Stroke::new(1.0, edge))
                .corner_radius(CornerRadius::same(cl::RADIUS_SM))
                .small()
                .atom_ui(ui),
        })
        .inner;
    if let (Some(icon), Some(rect)) = (icon, laid.rect(slot)) {
        let ink = if on {
            Tone::Cyan.color()
        } else if enabled {
            cl::ink::BASE
        } else {
            cl::ink::SUBTLE
        };
        cl::draw_icon(ui.painter(), icon, rect, ink);
    }
    laid.response
}

/// One row of the More AIs page: the AI's name, and its state in the tone
/// that state reads in — amber to sign in, cyan to install, quiet when there
/// is nothing to do here yet.
fn more_ai_row(ui: &mut egui::Ui, row: &MoreAiRow, chosen: bool) -> egui::Response {
    let state_tone = match row.state {
        MoreAiState::SignIn => Tone::Amber.color(),
        MoreAiState::Open => Tone::Lime.color(),
        MoreAiState::Install => Tone::Cyan.color(),
        MoreAiState::NotOnThisComputer => cl::ink::SUBTLE,
    };
    egui::Frame::new()
        .fill(if chosen {
            chat::rail::SELECTED
        } else {
            chat::rail::FILL
        })
        .stroke(Stroke::new(
            1.0,
            if chosen {
                chat::rail::SELECTED_EDGE
            } else {
                chat::panel::EDGE
            },
        ))
        .corner_radius(CornerRadius::same(cl::RADIUS_SM))
        .inner_margin(egui::Margin::symmetric(
            cl::space::S3 as i8,
            cl::space::S2 as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(row.label).strong().color(cl::ink::STRONG));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(row.state.label()).small().color(state_tone));
                });
            });
            if let Some(seen) = row.on_machine {
                ui.label(RichText::new(seen).small().color(Tone::Lime.color()));
            }
            if let Some(reason) = &row.reason {
                ui.label(RichText::new(reason).small().color(cl::ink::MUTED));
            }
        })
        .response
        .interact(Sense::click())
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
        ChatEventContent::ModelSelected { model } => ("MODEL", model.as_str(), Tone::Violet),
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
        ChatEventContent::UsageUpdated { summary, .. } => ("USAGE", summary.as_str(), Tone::Violet),
        ChatEventContent::TurnCompleted => ("SYSTEM", "Turn completed", Tone::Lime),
        ChatEventContent::TurnInterrupted => ("SYSTEM", "Turn interrupted", Tone::Amber),
        ChatEventContent::Unknown(payload) => {
            ("UNKNOWN", payload.redacted_summary.as_str(), Tone::Coral)
        }
    };
    egui::Frame::new()
        .fill(chat::bubble::FILL)
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
            .circle_filled(avatar.center(), 13.0, chat::bubble::AVATAR);
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
        .fill(chat::bubble::FILL)
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
        .fill(chat::bubble::NOTICE)
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

/// What `provider` may do from this chat (CW1: "configurable permissions").
/// Each choice carries its own plain-words tooltip; a change applies from the
/// next message.
fn permission_selector(
    ui: &mut egui::Ui,
    provider: ProviderId,
    label: &str,
    current: PermissionMode,
    ceiling: Option<PermissionMode>,
    intents: &mut Vec<ChatWindowIntent>,
) {
    let response = egui::ComboBox::from_id_salt(("dpl_chat_permission_mode", provider.0))
        .selected_text(RichText::new(current.label()).small())
        .show_ui(ui, |ui| {
            // The project's trust grant is the ceiling (ruling 2026-11-16):
            // a mode it does not allow is not offered.
            for mode in PermissionMode::ALL
                .into_iter()
                .filter(|mode| ceiling.is_none_or(|ceiling| mode.within(ceiling)))
            {
                let option = cl::tip(
                    ui.selectable_label(mode == current, mode.label()),
                    mode.tooltip(),
                );
                if option.clicked() && mode != current {
                    intents.push(ChatWindowIntent::SetPermissionMode { provider, mode });
                }
            }
        })
        .response;
    let response = cl::tip(
        response,
        "What this AI may do from this chat: read only, ask before acting, or work in its workspace",
    );
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            PilotId::new("chat.permissions").child(&PilotKey::new(provider.0)),
            Role::ComboBox,
            format!("{label} permissions"),
        )
        .value(current.label()),
    );
}

/// A provider's status as the operator reads it: "Ready · ChatGPT Plus" when
/// its CLI reported the account a turn will spend, otherwise the state.
fn provider_status(registration: &dpl_ai_core::registration::ProviderRegistration) -> String {
    match (&registration.activation, &registration.account) {
        (ActivationState::Ready, Some(account)) => match &account.plan {
            Some(plan) => format!("Ready · {} {plan}", account.display_name),
            None => format!("Ready · {}", account.display_name),
        },
        (activation, _) => activation.label().to_owned(),
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

fn paint_shell_backplate(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::ZERO, chat::panel::BACKPLATE);
    let bands = 24;
    for index in 0..bands {
        let t = index as f32 / bands as f32;
        let y0 = egui::lerp(rect.bottom()..=rect.top(), t);
        let y1 = egui::lerp(
            rect.bottom()..=rect.top(),
            (index + 1) as f32 / bands as f32,
        );
        // The glow rises from the floor and fades toward the top: two
        // colour roles, blended per band.
        let color = chat::panel::GLOW_LOW.lerp_to_gamma(chat::panel::GLOW_HIGH, t);
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
            chat::panel::EMBER.gamma_multiply_u8(alpha),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_ai_core::activation::RecoveryAction;
    use dpl_ai_core::capability::Capability;
    use dpl_ai_core::provider::{ProviderDescriptor, ProviderManifest, ProviderProfileId};
    use dpl_ai_core::registration::ProviderRegistration;
    use dpl_chat_core::SessionSummary;
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
            since: "2000-01-01",
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
            since: "2000-01-01",
        },
        profile: ProviderProfileId("second"),
        target_capabilities: CAPABILITIES,
        delivery_slice: "test",
        evidence: "fixture",
    };

    fn window() -> ChatWindow {
        let mut runtime =
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("runtime");
        let sessions = vec![
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
        runtime
            .replace_sessions(PROVIDER, sessions)
            .expect("fixture sessions");
        ChatWindow::new(
            runtime,
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        )
    }

    #[test]
    fn project_actions_are_typed_host_requests() {
        let mut window = window();
        let workspace = PathBuf::from("project-fixture");
        let id = dpl_chat_projects::ProjectId::parse("project-fixture").unwrap();
        let pairs = [
            (
                ChatWindowIntent::SaveProjectCopy {
                    workspace: workspace.clone(),
                    label: "Recovery copy".into(),
                },
                HostIntent::SaveProjectCopy {
                    workspace: workspace.clone(),
                    label: "Recovery copy".into(),
                },
            ),
            (
                ChatWindowIntent::SetupProject {
                    workspace: workspace.clone(),
                    label: "Fixture".into(),
                },
                HostIntent::SetupProject {
                    workspace: workspace.clone(),
                    label: "Fixture".into(),
                },
            ),
            (
                ChatWindowIntent::OpenProject {
                    workspace: workspace.clone(),
                },
                HostIntent::OpenProject { workspace },
            ),
            (
                ChatWindowIntent::SelectProject(id.clone()),
                HostIntent::SelectProject(id),
            ),
            (
                ChatWindowIntent::RetryProjectSave,
                HostIntent::RetryProjectSave,
            ),
        ];
        for (intent, expected) in pairs {
            assert_eq!(window.apply(intent).unwrap(), Some(expected));
        }
    }

    #[test]
    fn native_resume_is_explicit_and_does_not_replace_plain_archive_selection() {
        let mut window = window();
        window
            .set_provider_session_state(PROVIDER, ChatSessionId(1), SessionState::ReadOnly)
            .unwrap();
        assert_eq!(
            window
                .apply(ChatWindowIntent::SelectSession {
                    provider: PROVIDER,
                    session: ChatSessionId(1)
                })
                .unwrap(),
            None
        );
        assert_eq!(
            window
                .apply(ChatWindowIntent::LoadSession {
                    provider: PROVIDER,
                    session: ChatSessionId(1)
                })
                .unwrap(),
            Some(HostIntent::LoadSession {
                provider: PROVIDER,
                session: ChatSessionId(1)
            })
        );
        assert_eq!(
            window
                .apply(ChatWindowIntent::RefreshNativeSessions(PROVIDER))
                .unwrap(),
            Some(HostIntent::RefreshNativeSessions(PROVIDER))
        );
        assert!(window.project_session_read_only());
    }

    #[test]
    fn project_restore_preserves_drafts_and_exposes_read_only_sessions() {
        let mut window = window();
        window
            .apply(ChatWindowIntent::UpdateAiDraft {
                provider: PROVIDER,
                value: "unfinished project question".into(),
            })
            .unwrap();
        let saved = window.project_archive().unwrap();
        window.validate_project_archive(&saved).unwrap();
        assert_eq!(window.project_archive().unwrap(), saved);
        window.clear_project_state();
        assert!(window.project_provider_views()[0].sessions.is_empty());
        window.restore_project_archive(&saved).unwrap();
        let view = &window.project_provider_views()[0];
        assert_eq!(view.draft, "unfinished project question");
        assert_eq!(view.sessions[0].title, "Pinned design review");
        assert!(view
            .sessions
            .iter()
            .all(|session| session.state == SessionState::ReadOnly));
        assert!(window.project_session_read_only());
        assert_eq!(window.composer_hint(), "Saved project chat — read-only");
        let mut invalid = saved;
        invalid["v"] = serde_json::json!(999);
        let before_validation = window.project_archive().unwrap();
        assert!(window.validate_project_archive(&invalid).is_err());
        assert_eq!(window.project_archive().unwrap(), before_validation);
        assert!(window.restore_project_archive(&invalid).is_err());
        assert_eq!(
            window.project_provider_views()[0].draft,
            "unfinished project question"
        );
    }

    #[test]
    fn project_setup_offer_opens_native_controls_with_honest_support_status() {
        use egui_kittest::kittest::Queryable;
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    let _ = window.ui(ui);
                },
                window(),
            );
        harness.run_steps(2);
        harness.get_by_label("Set up project chat storage");
        harness
            .get_by_label("Choose a workspace for project chats")
            .click();
        harness.run_steps(2);
        harness.get_by_label("Standalone app storage is not connected yet.");
        harness.get_by_label("Create project chat storage in this folder");
        harness.get_by_label("Open a folder already marked for project chat storage");
    }

    #[test]
    fn archived_chat_enter_keeps_the_draft_and_explains_read_only_state() {
        let mut window = window();
        window
            .runtime
            .set_draft(PROVIDER, "keep this question".into())
            .unwrap();
        let archive = window.project_archive().unwrap();
        window.restore_project_archive(&archive).unwrap();
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        assert!(!matches!(intent, ChatWindowIntent::SendTurn { .. }));
                        window.apply(intent).unwrap();
                    }
                },
                window,
            );
        harness.run_steps(2);
        harness
            .ctx
            .memory_mut(|memory| memory.request_focus(egui::Id::new("dpl_chat_composer")));
        harness.run_steps(1);
        harness.input_mut().events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(2);
        assert_eq!(
            harness.state().project_provider_views()[0].draft,
            "keep this question"
        );
        assert!(harness
            .state()
            .composer_status()
            .unwrap()
            .contains("read-only"));
    }

    #[test]
    fn switching_projects_freezes_focused_composer_input() {
        let mut window = window();
        window
            .runtime
            .set_draft(PROVIDER, "saved before switch".into())
            .unwrap();
        window.set_project_input_locked(true);
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        assert!(!matches!(
                            intent,
                            ChatWindowIntent::UpdateAiDraft { .. }
                                | ChatWindowIntent::SendTurn { .. }
                        ));
                        window.apply(intent).unwrap();
                    }
                },
                window,
            );
        harness.run_steps(2);
        harness
            .ctx
            .memory_mut(|memory| memory.request_focus(egui::Id::new("dpl_chat_composer")));
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("do not accept".into()));
        harness.input_mut().events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(2);
        assert_eq!(
            harness.state().project_provider_views()[0].draft,
            "saved before switch"
        );
    }

    fn tab_manifest(
        id: &'static str,
        label: &'static str,
        short_label: &'static str,
    ) -> ProviderManifest {
        ProviderManifest {
            descriptor: ProviderDescriptor {
                id: ProviderId(id),
                label,
                short_label,
                glyph: "T",
                accent: ProviderAccent::Cyan,
                summary: "title-bar fixture",
                since: "2000-01-01",
            },
            profile: ProviderProfileId("title-bar"),
            target_capabilities: CAPABILITIES,
            delivery_slice: "test",
            evidence: "fixture",
        }
    }

    /// The compact proof's tab bar at the 720 px minimum, with every
    /// provider connected: no tab may overlap the next one or reach under
    /// the window controls, and the availability word may not cover a tab.
    #[test]
    fn the_title_bar_tabs_fit_at_the_minimum_window_width() {
        use egui_kittest::kittest::Queryable;
        let manifests = [
            tab_manifest("codex", "ChatGPT / Codex", "Codex"),
            tab_manifest("claude", "Claude Code", "Claude"),
            tab_manifest("gemini", "Gemini CLI", "Gemini"),
            tab_manifest("grok", "Grok Build", "Grok"),
        ];
        let runtime = ChatRuntime::try_from_registrations(manifests.map(|manifest| {
            let mut registration = ProviderRegistration::planned(manifest);
            registration.activation = ActivationState::Ready;
            registration
        }))
        .expect("runtime");
        let window = ChatWindow::new(
            runtime,
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        );
        let width = 720.0;
        let mut harness = Harness::builder().with_size([width, 560.0]).build_ui_state(
            |ui, window: &mut ChatWindow| {
                let _ = window.ui(ui);
            },
            window,
        );
        harness.run_steps(2);
        let tabs: Vec<egui::Rect> = [
            "General", "Codex", "Claude", "Gemini", "Grok", "More AIs", "Content", "Social",
        ]
        .into_iter()
        .map(|label| {
            harness
                .get_by_role_and_label(egui::accesskit::Role::Button, label)
                .rect()
        })
        .collect();
        for pair in tabs.windows(2) {
            assert!(
                pair[0].right() <= pair[1].left(),
                "tabs overlap: {:?} then {:?}",
                pair[0],
                pair[1]
            );
        }
        let controls = harness.get_by_label("Show Main").rect();
        let last = tabs.last().expect("eight tabs");
        assert!(
            last.right() < controls.left(),
            "Social {last:?} reaches under the window controls {controls:?}"
        );
        if let Some(word) = harness.query_by_label("available") {
            assert!(
                word.rect().left() >= last.right(),
                "the availability word covers Social"
            );
        }
        assert!(last.right() <= width);
    }

    #[test]
    fn a_signed_out_provider_keeps_its_saved_chat_and_offers_sign_in_above_it() {
        use egui_kittest::kittest::Queryable;
        let mut window = window();
        window
            .runtime
            .append_event(
                PROVIDER,
                ChatEvent {
                    id: ChatEventId(101),
                    provider_id: PROVIDER,
                    session_id: ChatSessionId(1),
                    turn_id: None,
                    content: ChatEventContent::AssistantMessage(
                        "Archived answer remains readable".into(),
                    ),
                },
            )
            .unwrap();
        let archive = window.project_archive().unwrap();
        window
            .set_provider_activation(
                PROVIDER,
                ActivationState::SignedOut {
                    reason: "Sign in again".into(),
                    recovery: vec![dpl_ai_core::activation::RecoveryAction::new(
                        RecoveryActionKind::SignIn,
                        "Log in again",
                    )],
                },
            )
            .unwrap();
        window.restore_project_archive(&archive).unwrap();
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, state: &mut (ChatWindow, Vec<ChatWindowIntent>)| {
                    let intents = state.0.ui(ui);
                    state.1.extend(intents);
                },
                (window, Vec::new()),
            );
        harness.run_steps(2);
        harness.get_by_label("Archived answer remains readable");
        harness.get_by_label("Saved project chat — read-only");
        harness.get_by_label("Log in again").click();
        harness.run_steps(2);
        assert!(harness
            .state()
            .1
            .contains(&ChatWindowIntent::ActivateProvider(PROVIDER)));
    }

    #[test]
    fn unavailable_providers_show_archived_messages_and_keep_editable_unsent_drafts() {
        use egui_kittest::kittest::Queryable;
        for activation in [
            ActivationState::Blocked {
                reason: "Unavailable here".into(),
                recovery: Vec::new(),
            },
            ActivationState::SignedOut {
                reason: "Sign in again".into(),
                recovery: Vec::new(),
            },
            ActivationState::NotInstalled {
                reason: "Client absent".into(),
                recovery: Vec::new(),
            },
        ] {
            let mut window = window();
            window
                .runtime
                .append_event(
                    PROVIDER,
                    ChatEvent {
                        id: ChatEventId(101),
                        provider_id: PROVIDER,
                        session_id: ChatSessionId(1),
                        turn_id: None,
                        content: ChatEventContent::AssistantMessage(
                            "Archived answer remains readable".into(),
                        ),
                    },
                )
                .unwrap();
            window
                .runtime
                .set_draft(PROVIDER, "saved draft".into())
                .unwrap();
            let archive = window.project_archive().unwrap();
            window
                .set_provider_activation(PROVIDER, activation.clone())
                .unwrap();
            window.restore_project_archive(&archive).unwrap();
            let mut harness = Harness::builder()
                .with_size([1280.0, 800.0])
                .build_ui_state(
                    |ui, window: &mut ChatWindow| {
                        for intent in window.ui(ui) {
                            assert!(!matches!(intent, ChatWindowIntent::SendTurn { .. }));
                            window.apply(intent).unwrap();
                        }
                    },
                    window,
                );
            harness.run_steps(2);
            harness.get_by_label("Archived answer remains readable");
            harness.get_by_label("Saved project chat — read-only");
            harness
                .ctx
                .memory_mut(|memory| memory.request_focus(egui::Id::new("dpl_chat_composer")));
            harness.run_steps(1);
            harness
                .input_mut()
                .events
                .push(egui::Event::Text("edited ".into()));
            harness.run_steps(2);
            let edited = harness.state().project_provider_views()[0].draft.clone();
            assert!(edited.contains("saved draft"));
            assert!(edited.contains("edited "));
            harness.input_mut().events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            harness.run_steps(2);
            assert_eq!(harness.state().project_provider_views()[0].draft, edited);
            assert_eq!(
                harness.state().project_provider_views()[0]
                    .registration
                    .activation,
                activation
            );
            assert!(harness
                .state()
                .composer_status()
                .unwrap()
                .contains("read-only"));
        }
    }

    fn multi_provider_window() -> ChatWindow {
        let mut first = ProviderRegistration::planned(MANIFEST);
        first.activation = ActivationState::Ready;
        let first_sessions = vec![SessionSummary {
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
        let second_sessions = vec![SessionSummary {
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
            .replace_sessions(PROVIDER, first_sessions)
            .expect("first sessions");
        runtime
            .replace_sessions(SECOND_PROVIDER, second_sessions)
            .expect("second sessions");
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
        for session in &provider.sessions {
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

    fn ready_and_signed_out_window() -> ChatWindow {
        let mut ready = ProviderRegistration::planned(MANIFEST);
        ready.activation = ActivationState::Ready;
        let mut signed_out = ProviderRegistration::planned(SECOND_MANIFEST);
        signed_out.activation = ActivationState::SignedOut {
            reason: "Not signed in".into(),
            recovery: Vec::new(),
        };
        // Ready first, so it is the window's default selection and the
        // signed-out provider appears nowhere until it is asked for.
        let runtime = ChatRuntime::try_from_registrations([ready, signed_out]).expect("runtime");
        ChatWindow::new(
            runtime,
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        )
    }

    #[test]
    fn the_rail_lists_only_activated_providers() {
        let window = ready_and_signed_out_window();
        let views = window.runtime.provider_views();
        let listed = rail_subscriptions(&views)
            .iter()
            .map(|provider| provider.registration.manifest.descriptor.id)
            .collect::<Vec<_>>();
        assert_eq!(
            listed,
            vec![PROVIDER],
            "a signed-out provider is not listed"
        );

        // Experimental is an explicit activation, so it is listed too —
        // and registration order is kept, so the rail never reshuffles.
        let mut experimental = ProviderRegistration::planned(SECOND_MANIFEST);
        experimental.activation = ActivationState::Experimental {
            reason: "fixture".into(),
            recovery: Vec::new(),
        };
        let mut ready = ProviderRegistration::planned(MANIFEST);
        ready.activation = ActivationState::Ready;
        let runtime = ChatRuntime::try_from_registrations([experimental, ready]).expect("runtime");
        let views = runtime.provider_views();
        let listed = rail_subscriptions(&views)
            .iter()
            .map(|provider| provider.registration.manifest.descriptor.id)
            .collect::<Vec<_>>();
        assert_eq!(listed, vec![SECOND_PROVIDER, PROVIDER]);

        // Nothing activated lists nothing; the chip says so instead.
        let runtime =
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("runtime");
        assert!(rail_subscriptions(&runtime.provider_views()).is_empty());
    }

    #[test]
    fn an_unactivated_provider_is_one_tap_away_in_the_catalog() {
        use egui_kittest::kittest::Queryable;
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        let _ = window.apply(intent);
                    }
                },
                ready_and_signed_out_window(),
            );
        harness.run_steps(2);
        // The rail carries the activated provider and not the signed-out one.
        assert!(harness.query_all_by_label("Fixture AI").count() >= 1);
        assert_eq!(harness.query_all_by_label("Second AI").count(), 0);

        // The AI Providers control opens the catalog, where EVERY provider
        // is a tile — the signed-out one included.
        harness.get_by_label("Open AI Providers").click();
        harness.run_steps(2);
        assert_eq!(
            harness.state().domain_states.ai.overlay,
            Some(DomainOverlay::ProviderCatalog)
        );
        assert!(harness.query_all_by_label("Second AI").count() >= 1);
        // The catalog keeps the house adapter and both ways to add one. A
        // row's own lettering is a second node with the same words, hence
        // "at least one".
        for kept in [
            "AI provider catalog",
            "Clyffy",
            "Add local provider",
            "Add custom provider",
        ] {
            assert!(
                harness.query_all_by_label(kept).count() >= 1,
                "the catalog lost {kept}"
            );
        }

        // Selecting the signed-out tile is just a selection: it closes the
        // catalog and hands the provider to the conversation pane.
        harness.get_by_label("Second AI").click();
        harness.run_steps(2);
        assert_eq!(harness.state().domain_states.ai.overlay, None);
        assert_eq!(
            harness.state().runtime.selected_provider_id(),
            Some(SECOND_PROVIDER)
        );
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
                content: ChatEventContent::Unknown(
                    dpl_ai_core::event::BoundedNativePayload::redacted(
                        "acp.available-commands",
                        "AvailableCommandsUpdate(...)",
                    ),
                ),
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
    fn the_brainstorm_trigger_is_whole_buffer_only() {
        let slug = |d: &str| recognize_command(d).map(|c| c.slug);
        assert_eq!(slug("brainstorming").as_deref(), Some("brainstorm"));
        assert_eq!(slug("  Brainstorm  ").as_deref(), Some("brainstorm"));
        assert_eq!(slug("/brainstorm").as_deref(), Some("brainstorm"));
        assert_eq!(
            slug("let's do some brainstorming"),
            None,
            "pattern-in-string never escalates (the provenance law)"
        );
        assert_eq!(slug("/brainstorming"), None, "unknown /x stays text");
        assert_eq!(slug(""), None);
    }

    #[test]
    fn brainstorm_intents_cross_to_the_host_verbatim() {
        let fixture = dpl_chat_testkit::authored_chat();
        let mut window = ChatWindow::with_models(
            fixture.runtime().expect("runtime"),
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
            fixture.social,
            InteractionRegistry::default(),
        );
        assert_eq!(
            window.apply(ChatWindowIntent::StartBrainstorm).expect("ok"),
            Some(HostIntent::StartBrainstorm)
        );
        let op = dpl_workflows::BrainstormOp::Seed {
            seed: "the idea".into(),
        };
        assert_eq!(
            window
                .apply(ChatWindowIntent::Brainstorm(op.clone()))
                .expect("ok"),
            Some(HostIntent::Brainstorm(op))
        );
        assert_eq!(
            window
                .apply(ChatWindowIntent::PromoteBrainstorm)
                .expect("ok"),
            Some(HostIntent::PromoteBrainstorm)
        );
    }

    #[test]
    fn observe_brainstorm_projects_the_board_verbatim() {
        let fixture = dpl_chat_testkit::authored_chat();
        let mut window = ChatWindow::with_models(
            fixture.runtime().expect("runtime"),
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
            fixture.social,
            InteractionRegistry::default(),
        );
        assert!(window.brainstorm().doc.is_none());
        let scaffold = dpl_workflows::brainstorm_scaffold("breakdown").expect("registered");
        let doc = dpl_workflows::mint_brainstorm(&scaffold, "proof");
        window.observe_brainstorm(BrainstormView {
            doc: Some(doc.clone()),
            status: Some("Board opened.".into()),
            can_undo: false,
            can_redo: true,
        });
        assert_eq!(window.brainstorm().doc.as_ref(), Some(&doc));
        assert!(window.brainstorm().can_redo);
    }

    #[test]
    fn dismissing_an_approval_queues_its_real_focus_origin() {
        let fixture = dpl_chat_testkit::authored_chat();
        let mut window = ChatWindow::with_models(
            fixture.runtime().expect("runtime"),
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
            fixture.runtime().expect("runtime"),
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

    // ---- RM1: the Rooms & roles library ----------------------------------

    #[test]
    fn rooms_and_roles_opens_the_library_and_names_every_builtin() {
        use egui_kittest::kittest::Queryable;
        let library = dpl_chat_core::rooms::Library::builtin();
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        let _ = window.apply(intent);
                    }
                },
                window(),
            );
        harness.run_steps(2);
        assert!(!harness.state().library_open);

        harness.get_by_label_contains("Rooms & roles").click();
        harness.run_steps(2);
        assert!(harness.state().library_open);
        // Every built-in is listed, by the name the pilot tracks it under.
        for room in &library.rooms {
            harness.get_by_label(&format!("{} room", room.name));
        }
        for role in &library.roles {
            harness.get_by_label(&format!("{} role", role.name));
        }

        harness.get_by_label("Critiques").click();
        harness.run_steps(2);
        let skeptic = library.role("skeptic").expect("built-in skeptic");
        let dreamer = library.role("dreamer").expect("built-in dreamer");
        harness.get_by_label(&format!("{} role", skeptic.name));
        assert!(harness
            .query_by_label(&format!("{} role", dreamer.name))
            .is_none());
        harness.get_by_label("All stances").click();
        harness.run_steps(2);
        harness.get_by_label(&format!("{} role", dreamer.name));

        harness.get_by_label("Back to chat").click();
        harness.run_steps(2);
        assert!(!harness.state().library_open);
    }

    #[test]
    fn opening_the_library_folder_is_the_hosts_job() {
        let mut window = window();
        assert_eq!(
            window.apply(ChatWindowIntent::OpenRoomLibraryFolder),
            Ok(Some(HostIntent::OpenRoomLibraryFolder))
        );
        assert_eq!(window.apply(ChatWindowIntent::ToggleRoomLibrary), Ok(None));
        assert!(window.library_open);
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

    impl dpl_ai_core::history::HistoryDecoder for FixtureHistoryDecoder {
        fn provider_id(&self) -> ProviderId {
            PROVIDER
        }

        fn index_metadata(
            &self,
            path: &std::path::Path,
            _prefix: &str,
        ) -> Option<dpl_ai_core::history::HistoryIndexMetadata> {
            let stem = path.file_stem()?.to_string_lossy().into_owned();
            Some(dpl_ai_core::history::HistoryIndexMetadata {
                native_id: dpl_ai_core::session::ProviderSessionId(stem.clone()),
                title: format!("Transcript {stem}"),
                workspace: "DevPULSE".to_owned(),
                started_at: "2026-07-30T00:00:00Z".to_owned(),
            })
        }

        fn decode_line(&self, line: &str) -> dpl_ai_core::history::DecodeOutcome {
            dpl_ai_core::history::DecodeOutcome::Event(dpl_ai_core::history::DecodedHistoryEvent {
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
                .all(|provider| provider.sessions.is_empty()),
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

        assert_eq!(
            window.composer_hint(),
            "Message… — Enter sends, Shift+Enter new line",
            "the composer advertises its keys where the operator is typing"
        );
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
        let privileged = |d: &str| recognize_command(d).and_then(|c| privileged_command(&c));
        assert_eq!(privileged("/strict"), Some(OperatorSlashCommand::Strict));
        assert_eq!(
            privileged("  /behavioral  "),
            Some(OperatorSlashCommand::Behavioral)
        );
        // Buried in a sentence — inert. Pattern-in-string never escalates.
        assert_eq!(privileged("please /strict now"), None);
        assert!(recognize_command("please /strict now").is_none());
        // Trailing text is NOT the audited command — it recognizes and
        // then refuses honestly ("takes no arguments"), never escalates.
        assert_eq!(privileged("/strict please"), None);
        assert!(recognize_command("/strict please").is_some());
        assert_eq!(privileged("/stricter"), None);
        assert!(recognize_command("/stricter").is_none());
        assert_eq!(privileged("strict"), None);
        assert_eq!(privileged(""), None);
    }

    /// THE ENTER-HONESTY PIN (BND1 V7): a consumed keyboard submit that
    /// cannot send SAYS WHY — the reason lands in `composer_status` (and
    /// paints under the composer), instead of dying in a disabled
    /// button's hover tooltip.
    #[test]
    fn a_refused_enter_paints_its_reason() {
        let fixture = dpl_chat_testkit::authored_chat();
        let window = ChatWindow::with_models(
            fixture.runtime().expect("runtime"),
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
            fixture.social,
            InteractionRegistry::default(),
        );
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1040.0, 640.0))
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    let _ = window.ui(ui);
                },
                window,
            );
        harness.run_steps(2);
        // Focus the composer (the draft is empty — Enter cannot send),
        // then press bare Enter.
        harness
            .ctx
            .memory_mut(|m| m.request_focus(egui::Id::new("dpl_chat_composer")));
        harness.run_steps(1);
        harness.input_mut().events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(2);
        assert_eq!(
            harness.state().composer_status(),
            Some("Type a message first."),
            "the refusal must be painted, not hover-only"
        );
    }

    #[test]
    fn slash_commands_name_their_audit_action() {
        assert_eq!(OperatorSlashCommand::Strict.as_str(), "strict");
        assert_eq!(OperatorSlashCommand::Behavioral.as_str(), "behavioral");
    }

    /// Drive a real `InputState` with one Enter press carrying `modifiers`,
    /// and report whether the composer would take it as a SEND and whether
    /// the key survived for the `TextEdit` to turn into a newline.
    fn enter_with(modifiers: egui::Modifiers) -> (bool, bool) {
        let ctx = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        });
        // epaint 0.36 PANICS when a `TexturesDelta` is dropped with unapplied
        // deltas — dropping the whole `FullOutput` on the floor used to be
        // fine and now is not. This test drives input and paints nothing, so
        // clearing is the honest disposal rather than pretending to upload.
        let mut output = ctx.run_ui(input, |_| {});
        output.textures_delta.clear();
        let submitted = ctx.input_mut(take_composer_submit);
        // Whatever is left is what the text widget would act on.
        let survives = ctx.input(|i| {
            i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        key: egui::Key::Enter,
                        pressed: true,
                        ..
                    }
                )
            })
        });
        (submitted, survives)
    }

    #[test]
    fn enter_sends_and_shift_enter_makes_a_new_line() {
        let (submitted, survives) = enter_with(egui::Modifiers::NONE);
        assert!(submitted, "bare Enter must SEND — the whole point");
        assert!(
            !survives,
            "a sending Enter must be consumed, or it sends AND inserts a newline"
        );

        let (submitted, survives) = enter_with(egui::Modifiers::SHIFT);
        assert!(
            !submitted,
            "Shift+Enter must NOT send — it is how a multi-line message is typed"
        );
        assert!(
            survives,
            "Shift+Enter must reach the TextEdit so it becomes a new line"
        );
    }

    #[test]
    fn ctrl_enter_also_sends() {
        // The operator asked for it by name, and several clients use it as
        // the primary submit.
        let (submitted, survives) = enter_with(egui::Modifiers::COMMAND);
        assert!(submitted, "Ctrl/Cmd+Enter must send");
        assert!(!survives, "and must not also leave a newline behind");
    }

    // ── C2b: the tab model, General, More AIs, the `+` menu and modes ────

    fn registration_in(
        manifest: ProviderManifest,
        activation: ActivationState,
    ) -> ProviderRegistration {
        let mut registration = ProviderRegistration::planned(manifest);
        registration.activation = activation;
        registration
    }

    #[test]
    fn the_tab_model_splits_ready_ais_from_more_ais() {
        let manifests = [
            tab_manifest("codex", "ChatGPT / Codex", "Codex"),
            tab_manifest("gemini", "Gemini CLI", "Gemini"),
            tab_manifest("grok", "Grok Build", "Grok"),
            tab_manifest("kimi", "Kimi Code", "Kimi"),
            tab_manifest("claude", "Claude Code", "Claude"),
            tab_manifest("qwen", "Qwen Code", "Qwen"),
            tab_manifest("hermes", "Hermes", "Hermes"),
        ];
        let runtime = ChatRuntime::try_from_registrations([
            registration_in(manifests[0], ActivationState::Ready),
            registration_in(
                manifests[1],
                ActivationState::Experimental {
                    reason: "fixture".into(),
                    recovery: Vec::new(),
                },
            ),
            registration_in(
                manifests[2],
                ActivationState::SignedOut {
                    reason: "Signed out".into(),
                    recovery: vec![RecoveryAction::new(RecoveryActionKind::SignIn, "Log in")],
                },
            ),
            registration_in(
                manifests[3],
                ActivationState::NotInstalled {
                    reason: "Kimi is not installed".into(),
                    recovery: vec![RecoveryAction::new(
                        RecoveryActionKind::InstallInstructions,
                        "Install Kimi",
                    )],
                },
            ),
            registration_in(
                manifests[4],
                ActivationState::Blocked {
                    reason: "No qualified adapter".into(),
                    recovery: Vec::new(),
                },
            ),
            registration_in(
                manifests[5],
                ActivationState::NotInstalled {
                    reason: "Nothing can run it here".into(),
                    recovery: Vec::new(),
                },
            ),
            registration_in(
                manifests[6],
                ActivationState::Failed {
                    reason: "The sign-in stopped".into(),
                    recovery: vec![RecoveryAction::new(RecoveryActionKind::Retry, "Try again")],
                },
            ),
        ])
        .expect("runtime");
        let (ready, more) = tab_model(&runtime.provider_views());
        assert_eq!(
            ready,
            vec![ProviderId("codex"), ProviderId("gemini")],
            "Ready and Experimental earn a tab, in registration order"
        );
        let states: Vec<(&str, MoreAiState)> =
            more.iter().map(|row| (row.provider.0, row.state)).collect();
        assert_eq!(
            states,
            vec![
                ("grok", MoreAiState::SignIn),
                ("hermes", MoreAiState::SignIn),
                ("kimi", MoreAiState::Install),
                ("claude", MoreAiState::NotOnThisComputer),
                ("qwen", MoreAiState::NotOnThisComputer),
            ],
            "grouped by what the operator can do, registration order within a group"
        );
        assert_eq!(more[0].reason.as_deref(), Some("Signed out"));
        assert_eq!(MoreAiState::Install.label(), "Install");
        assert_eq!(
            MoreAiState::NotOnThisComputer.label(),
            "Not on this computer"
        );
    }

    /// The real AI catalog in More AIs: every AI is listed, and the setup of
    /// the LAST one opens where it can be seen in a compact window, instead
    /// of under fifty other rows.
    #[test]
    fn the_whole_catalog_scrolls_and_the_last_ais_setup_is_in_view() {
        use egui_kittest::kittest::Queryable;
        let catalog = dpl_ai_core::catalog::catalog();
        let registrations = catalog.entries.iter().enumerate().map(|(index, entry)| {
            let mut registration = ProviderRegistration::planned(entry.manifest);
            registration.activation = if index == 0 {
                ActivationState::Ready
            } else {
                ActivationState::NotInstalled {
                    reason: "fixture".into(),
                    recovery: vec![RecoveryAction::new(
                        RecoveryActionKind::InstallInstructions,
                        format!("Set up {}", entry.manifest.descriptor.label),
                    )],
                }
            };
            registration
        });
        let runtime = ChatRuntime::try_from_registrations(registrations).expect("runtime");
        let (_, rows) = tab_model(&runtime.provider_views());
        assert_eq!(
            rows.len(),
            catalog.entries.len() - 1,
            "every other AI is listed"
        );
        let first = rows.first().expect("rows").label;
        let last = rows.last().expect("rows").label;
        let window = ChatWindow::new(
            runtime,
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        );
        let mut harness = Harness::builder().with_size([720.0, 560.0]).build_ui_state(
            |ui, window: &mut ChatWindow| {
                for intent in window.ui(ui) {
                    let _ = window.apply(intent);
                }
            },
            window,
        );
        harness.run_steps(2);
        harness.get_by_label("More AIs").click();
        harness.run_steps(2);
        // Scroll to the end, as a person would, then tap the last AI.
        let list = harness
            .get_by_role_and_label(egui::accesskit::Role::Button, first)
            .rect();
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(list.center()));
        harness.input_mut().events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -100_000.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        harness.run_steps(4);
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, last)
            .click();
        harness.run_steps(4);
        let setup_label = format!("Set up {last}");
        let rect = harness.get_by_label(&setup_label).rect();
        assert!(
            rect.min.y >= 0.0 && rect.max.y <= 560.0,
            "the last AI's setup is in view at 720x560, not clipped: {rect:?}"
        );
    }

    /// The trust dialog (AH1b): everything starts off, Allow needs Read, the
    /// answer carries exactly what was ticked, and Don't allow is an answer.
    #[test]
    fn the_trust_dialog_allows_only_what_was_ticked() {
        use egui_kittest::kittest::Queryable;
        let mut window = ready_and_signed_out_window();
        window.request_trust(PROVIDER, "Lesson 12".into(), Grant::default());
        let answers = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = std::rc::Rc::clone(&answers);
        let mut harness = Harness::builder()
            .with_size([1100.0, 700.0])
            .build_ui_state(
                move |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        if let ChatWindowIntent::AnswerTrust { grant, .. } = &intent {
                            seen.borrow_mut().push(*grant);
                        }
                        let _ = window.apply(intent);
                    }
                },
                window,
            );
        harness.run_steps(2);
        assert_eq!(harness.state().trust_dialog_open(), Some(PROVIDER));
        harness.get_by_label("Allow").click();
        harness.run_steps(2);
        assert!(
            answers.borrow().is_empty(),
            "Allow does nothing until Read is ticked"
        );
        harness.get_by_label("Read files").click();
        harness.run_steps(2);
        harness.get_by_label("Change files").click();
        harness.run_steps(2);
        harness.get_by_label("Allow").click();
        harness.run_steps(2);
        assert_eq!(
            answers.borrow().as_slice(),
            [Some(Grant {
                read: true,
                change: true,
                run: false,
            })]
        );
        assert_eq!(harness.state().trust_dialog_open(), None);

        harness
            .state_mut()
            .request_trust(PROVIDER, "Lesson 12".into(), Grant::default());
        harness.run_steps(2);
        harness.get_by_label("Don't allow").click();
        harness.run_steps(2);
        assert_eq!(answers.borrow().last(), Some(&Some(Grant::default())));
    }

    #[test]
    fn a_trust_grant_is_the_ceiling_of_the_permission_picker() {
        let mut window = ready_and_signed_out_window();
        window.set_provider_trust(PROVIDER, Some(Some(PermissionMode::ReadOnly)));
        assert_eq!(
            window.trust.get(&PROVIDER),
            Some(&Some(PermissionMode::ReadOnly))
        );
        let offered: Vec<_> = PermissionMode::ALL
            .into_iter()
            .filter(|mode| mode.within(PermissionMode::ReadOnly))
            .collect();
        assert_eq!(offered, [PermissionMode::ReadOnly]);
        window.set_provider_trust(PROVIDER, Some(None));
        assert_eq!(
            window.trust.get(&PROVIDER),
            Some(&None),
            "asked, not allowed"
        );
        window.set_provider_trust(PROVIDER, None);
        assert!(
            window.trust.is_empty(),
            "outside a project trust does not apply"
        );
    }

    #[test]
    fn a_found_desktop_app_offers_open_and_says_what_the_scan_saw() {
        let open = ActivationState::Blocked {
            reason: "Running now.".into(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::OpenApp,
                "Bring Claude app to the front",
            )],
        };
        assert_eq!(MoreAiState::of(&open), MoreAiState::Open);
        assert_eq!(MoreAiState::Open.label(), "Open app");
        let mut found = registration_in(tab_manifest("claude-desktop", "Claude", "Claude"), open);
        found.on_machine = Some(OnMachine {
            installed: true,
            running: true,
        });
        let runtime = ChatRuntime::try_from_registrations([found]).expect("runtime");
        let (_, more) = tab_model(&runtime.provider_views());
        assert_eq!(more[0].state, MoreAiState::Open);
        assert_eq!(more[0].on_machine, Some("On this computer · running now"));
    }

    #[test]
    fn the_scan_never_claims_what_it_did_not_see() {
        assert_eq!(on_machine_words(None), None, "no scan answer yet");
        assert_eq!(
            on_machine_words(Some(OnMachine {
                installed: false,
                running: false
            })),
            None
        );
        assert_eq!(
            on_machine_words(Some(OnMachine {
                installed: true,
                running: false
            })),
            Some("On this computer")
        );
        assert_eq!(
            on_machine_words(Some(OnMachine {
                installed: false,
                running: true
            })),
            Some("Running now")
        );
    }

    #[test]
    fn more_ais_is_a_tab_and_one_tap_opens_the_existing_setup() {
        use egui_kittest::kittest::Queryable;
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        let _ = window.apply(intent);
                    }
                },
                ready_and_signed_out_window(),
            );
        harness.run_steps(2);
        harness.get_by_label("More AIs").click();
        harness.run_steps(2);
        assert_eq!(harness.state().ai_tab(), AiTab::MoreAis);
        // The signed-out AI is listed there, and only there.
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Second AI")
            .click();
        harness.run_steps(2);
        assert_eq!(
            harness.state().runtime.selected_provider_id(),
            Some(SECOND_PROVIDER)
        );
        assert_eq!(
            harness.state().ai_tab(),
            AiTab::MoreAis,
            "the setup opens on the same page, not somewhere else"
        );
        // General is the way back, and it speaks through what backs it.
        harness.get_by_label("General").click();
        harness.run_steps(2);
        assert_eq!(harness.state().ai_tab(), AiTab::General);
    }

    #[test]
    fn general_is_backed_only_by_a_signed_in_ai() {
        let mut window = ready_and_signed_out_window();
        // A signed-out AI cannot back General: nothing changes, nothing
        // reaches the host.
        assert_eq!(
            window
                .apply(ChatWindowIntent::AssignGeneralBacking(SECOND_PROVIDER))
                .expect("a refused backing is not an error"),
            None
        );
        assert_eq!(window.clyffy().backing, None);
        // A signed-in one does, and the host is asked to save it.
        assert_eq!(
            window
                .apply(ChatWindowIntent::AssignGeneralBacking(PROVIDER))
                .expect("assign"),
            Some(HostIntent::AssignGeneralBacking(PROVIDER))
        );
        assert_eq!(window.clyffy().backing, Some(PROVIDER));
        assert_eq!(window.runtime.selected_provider_id(), Some(PROVIDER));
    }

    #[test]
    fn the_general_tab_speaks_through_its_backing() {
        let mut window = multi_provider_window();
        window.observe_clyffy(ClyffyView {
            present: true,
            active: true,
            backing: Some(SECOND_PROVIDER),
            events: 0,
        });
        window
            .apply(ChatWindowIntent::SelectProviderTab(PROVIDER))
            .expect("provider tab");
        assert_eq!(window.runtime.selected_provider_id(), Some(PROVIDER));
        assert_eq!(
            window
                .apply(ChatWindowIntent::SelectDomain(ChatDomain::Ai))
                .expect("general"),
            Some(HostIntent::ProviderSelected(SECOND_PROVIDER))
        );
        assert_eq!(window.ai_tab(), AiTab::General);
        assert_eq!(window.runtime.selected_provider_id(), Some(SECOND_PROVIDER));
    }

    /// Two ready AIs whose manifests DECLARE Tools, so a test can qualify it
    /// later — the runtime refuses a capability the manifest never declared.
    fn mode_window() -> ChatWindow {
        const MODE_CAPABILITIES: &[Capability] = &[
            Capability::ReadSessions,
            Capability::StartSession,
            Capability::StreamTurns,
            Capability::InterruptTurn,
            Capability::Approvals,
            Capability::Tools,
        ];
        let mut first = ProviderRegistration::planned(ProviderManifest {
            target_capabilities: MODE_CAPABILITIES,
            ..MANIFEST
        });
        first.activation = ActivationState::Ready;
        let mut second = ProviderRegistration::planned(SECOND_MANIFEST);
        second.activation = ActivationState::Ready;
        let mut runtime = ChatRuntime::try_from_registrations([first, second]).expect("runtime");
        for provider in [PROVIDER, SECOND_PROVIDER] {
            runtime
                .qualify_capabilities(provider, CAPABILITIES.iter().copied())
                .expect("capabilities");
        }
        ChatWindow::new(
            runtime,
            ROUTE,
            &HistoryCatalog::default(),
            vec![("Library".into(), ROUTE)],
        )
    }

    #[test]
    fn a_mode_is_offered_only_when_the_ai_advertises_it_and_rides_the_send() {
        let mut window = mode_window();
        // The fixture advertises no Modes, Plans or Tools: Thinking, Plan
        // first and Web search are refused; Brainstorm is ours.
        for refused in ["thinking", "plan", "web-search"] {
            window
                .apply(ChatWindowIntent::ToggleMode(refused))
                .expect("toggle");
            assert!(
                !window.composer_modes().is_active(refused),
                "{refused} must not stand against an AI that does not offer it"
            );
        }
        window
            .apply(ChatWindowIntent::ToggleMode("brainstorm"))
            .expect("toggle");
        assert!(window.composer_modes().is_active("brainstorm"));

        // An AI that advertises Tools offers Web search.
        window
            .runtime
            .qualify_capabilities(PROVIDER, [Capability::Tools])
            .expect("qualify");
        window
            .apply(ChatWindowIntent::ToggleMode("web-search"))
            .expect("toggle");
        assert!(window.composer_modes().is_active("web-search"));

        // The send carries exactly what stands, to the host, unchanged.
        let modes = window.composer_modes().clone();
        let out = window
            .apply(ChatWindowIntent::SendTurn {
                provider: PROVIDER,
                session: ChatSessionId(1),
                text: "hello".into(),
                modes: modes.clone(),
            })
            .expect("send");
        assert_eq!(
            out,
            Some(HostIntent::SendTurn {
                provider: PROVIDER,
                session: ChatSessionId(1),
                text: "hello".into(),
                modes,
            })
        );

        // Switching to an AI without Tools drops Web search and says so.
        window
            .apply(ChatWindowIntent::SelectProviderTab(SECOND_PROVIDER))
            .expect("tab");
        assert!(!window.composer_modes().is_active("web-search"));
        assert!(window.composer_modes().is_active("brainstorm"));
        assert!(window
            .composer_status()
            .is_some_and(|status| status.contains("Web search")));
    }

    #[test]
    fn the_plus_menu_lists_the_census_and_closes_on_escape_back_to_the_input() {
        use egui_kittest::kittest::Queryable;
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    for intent in window.ui(ui) {
                        let _ = window.apply(intent);
                    }
                },
                multi_provider_window(),
            );
        harness.run_steps(2);
        assert_eq!(harness.query_all_by_label("Attach files").count(), 0);
        harness.get_by_label("Add to message").click();
        harness.run_steps(3);
        assert!(harness.state().composer_menu_open);
        for entry in ["Attach files", "Image / video", "Component", "Voice"] {
            assert!(
                harness.query_all_by_label(entry).count() >= 1,
                "the menu lost {entry}"
            );
        }
        harness.key_press(egui::Key::Escape);
        harness.run_steps(3);
        assert!(!harness.state().composer_menu_open, "Esc closes the menu");
        assert_eq!(harness.query_all_by_label("Attach files").count(), 0);
        assert!(
            harness
                .ctx
                .memory(|memory| memory.has_focus(egui::Id::new("dpl_chat_composer"))),
            "focus returns to the message"
        );
    }

    #[test]
    fn picking_component_from_the_menu_opens_the_board_and_closes_the_menu() {
        use egui_kittest::kittest::Queryable;
        let mut harness = Harness::builder()
            .with_size([1280.0, 800.0])
            .build_ui_state(
                |ui, window: &mut ChatWindow| {
                    let intents = window.ui(ui);
                    let started = intents
                        .iter()
                        .any(|intent| matches!(intent, ChatWindowIntent::StartBrainstorm));
                    if started {
                        window.set_composer_status("board asked for");
                    }
                    for intent in intents {
                        let _ = window.apply(intent);
                    }
                },
                multi_provider_window(),
            );
        harness.run_steps(2);
        harness.get_by_label("Add to message").click();
        harness.run_steps(3);
        harness.get_by_label("Component").click();
        harness.run_steps(3);
        assert!(
            !harness.state().composer_menu_open,
            "a pick closes the menu"
        );
        assert_eq!(harness.state().composer_status(), Some("board asked for"));
    }

    /// The compact window (720×560): the composer's rows are budgeted, so the
    /// input and Send always stay in the window, every chip that is shown
    /// is inside it at no less than its own label's width, and any mode the
    /// row had no room for sits behind the "More" chip instead of being
    /// squeezed.
    #[test]
    fn the_composer_fits_the_compact_window() {
        use egui_kittest::kittest::Queryable;
        let size = egui::vec2(720.0, 560.0);
        let mut harness = Harness::builder().with_size(size).build_ui_state(
            |ui, window: &mut ChatWindow| {
                for intent in window.ui(ui) {
                    let _ = window.apply(intent);
                }
            },
            mode_window(),
        );
        harness.run_steps(2);
        let bounds = Rect::from_min_size(egui::Pos2::ZERO, size);
        for control in ["Message composer", "Send message", "Add to message"] {
            let rect = harness.get_by_label(control).rect();
            assert!(
                bounds.contains_rect(rect) && rect.width() > 0.0 && rect.height() > 0.0,
                "{control} {rect:?} must lie inside the {size:?} window"
            );
        }
        let small = egui::TextStyle::Small.resolve(&harness.ctx.global_style());
        let mut hidden = Vec::new();
        for entry in in_group(ComposerGroup::Mode) {
            let nodes: Vec<Rect> = harness
                .query_all_by_label(entry.label)
                .map(|node| node.rect())
                .collect();
            let Some(rect) = nodes.first().copied() else {
                hidden.push(entry.label);
                continue;
            };
            let label = harness.ctx.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(entry.label.to_owned(), small.clone(), cl::ink::BASE)
                    .size()
                    .x
            });
            assert!(
                bounds.contains_rect(rect),
                "the {} chip {rect:?} must lie inside the window",
                entry.label
            );
            assert!(
                rect.width() >= label,
                "the {} chip is {} wide, narrower than its label ({label})",
                entry.label,
                rect.width()
            );
        }
        if !hidden.is_empty() {
            let more = harness.get_by_label("More modes").rect();
            assert!(
                bounds.contains_rect(more),
                "the More chip {more:?} holding {hidden:?} must lie inside the window"
            );
        }
    }

    /// The modes the row had no room for open from the "More" chip in the
    /// composer's popup, and Esc hands focus back to the message.
    #[test]
    fn the_modes_that_do_not_fit_open_from_more() {
        use egui_kittest::kittest::Queryable;
        // Narrower than the window allows by hand, so the row cannot hold
        // all four chips and the overflow has to carry the rest.
        let size = egui::vec2(420.0, 560.0);
        let mut harness = Harness::builder().with_size(size).build_ui_state(
            |ui, window: &mut ChatWindow| {
                for intent in window.ui(ui) {
                    let _ = window.apply(intent);
                }
            },
            mode_window(),
        );
        harness.run_steps(2);
        let shown_before = in_group(ComposerGroup::Mode)
            .filter(|entry| harness.query_all_by_label(entry.label).count() > 0)
            .count();
        let bounds = Rect::from_min_size(egui::Pos2::ZERO, size);
        for control in ["Message composer", "Send message", "More modes"] {
            let rect = harness.get_by_label(control).rect();
            assert!(
                bounds.contains_rect(rect),
                "{control} {rect:?} must stay inside even this narrow window"
            );
        }
        assert!(shown_before < 4, "420 px cannot hold every chip inline");
        harness.get_by_label("More modes").click();
        harness.run_steps(3);
        assert!(harness.state().mode_menu_open);
        for entry in in_group(ComposerGroup::Mode) {
            assert!(
                harness.query_all_by_label(entry.label).count() >= 1,
                "{} is reachable",
                entry.label
            );
        }
        harness.key_press(egui::Key::Escape);
        harness.run_steps(3);
        assert!(!harness.state().mode_menu_open);
        assert!(harness
            .ctx
            .memory(|memory| memory.has_focus(egui::Id::new("dpl_chat_composer"))));
    }
}
