pub(super) mod project;

use dpl_chat_accounts::ActivationLedger;
use dpl_chat_browser_auth::{BrowserAuthCoordinator, BrowserAuthError, BrowserAuthEvent};
use dpl_chat_core::{
    ActivationState, BoundedNativePayload, Capability, ChatEvent, ChatEventContent, ChatEventId,
    ChatSessionId, ContentChapter, ContentEpisode, ContentEpisodeId, ContentSlide, ContentSpine,
    ContentView, PermissionMode, ProviderId, ProviderSessionId, RecoveryAction, RecoveryActionKind,
    SessionGroup, SessionState, SessionSummary, TokenUsage, TurnId,
};
#[cfg(feature = "eguipilotai-devtools")]
use dpl_chat_core::{ChatDomain, DeviceAuthorizationPrompt, GuidedValue, GuidedValueSensitivity};
use dpl_chat_imagine::{ImagineCache, ImagineClient, ImaginePolicy, SystemImagineClient};
use dpl_chat_interactions::{
    InteractionContent, InteractionId, InteractionPlacement, InteractionRegistry,
};
use dpl_chat_profiles::{ProfileStore, ProviderProfileDraft};
use dpl_chat_runtime::seam::{
    qualified_capabilities, ProviderRuntime, RuntimeCommand, RuntimeEvent, RuntimeFailureKind,
    RuntimePermissionOptionKind, RuntimeUpdate,
};
use dpl_chat_runtime::ChatRuntime;
use dpl_chat_social::SocialModel;
use dpl_chat_store::HistoryCatalog;
use dpl_chat_window::{
    ChatWindow, GenerationDraftView, GenerationStatus, HostIntent, ImagineFlushDecision,
    OperatorSlashCommand, ProviderPermissionChoice, ProviderPermissionChoiceKind,
};
use dpl_chat_windows::{
    ChatCloseDecision, ChatWindowId, ChatWindowRecord, ChatWindowRegistry, PeerLifetime,
    RootCloseDecision, SnapEdge, ViewportGeneration, WindowAttachment, WindowBinding,
    WindowGeometry, WindowKind, WindowLifecycle,
};
use dpl_fabric::{
    ActivationAttemptId, AiEvent, AiFailureClass, AiOutcome, AppWindowAction, CallId, CaptureEvent,
    Fabric, IntentEvent, JobEvent, JobId, Origin, ProviderActivationStage, ProviderCapability,
    ProviderEvent, ProviderFailureClass, ProviderReceiver, ShellEvent, ShellReceiver,
};
use dpl_hub::Route;
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    ContextPilotExt, NodeMetadata, PilotId, UiPilotExt,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

#[cfg(feature = "eguipilotai-devtools")]
type PilotRuntime = egui_pilot_ai_runtime::RuntimeGuard<egui_pilot_ai_transport::StdioTransport>;

#[cfg(feature = "eguipilotai-devtools")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatProofMode {
    Approval,
    Ai,
    Social,
    ProviderCatalog,
    ProviderHover,
    CodexLogin,
    Generation,
    GrokLogin,
    GrokAuthorization,
    Compact,
    Snapped,
    /// The blanket assist acknowledgement, open — the visual proof state.
    Assist,
    /// The Content domain with a real episode spine loaded (D1).
    Content,
    /// A REAL local transcript open in the conversation pane (H1).
    History,
    /// The inline brainstorm board, worked to Cluster with an assist
    /// item visible (BS1) — deterministic, in-memory only.
    Brainstorm,
    /// The command strip open over a `/ro` draft (BND1) — the census
    /// offering /route and /routines.
    Commands,
    /// The Rooms & roles library open in the AI pane (RM1).
    Library,
    Projects,
    ProjectsCompact,
}

#[cfg(feature = "eguipilotai-devtools")]
impl ChatProofMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "approval" => Some(Self::Approval),
            "ai" => Some(Self::Ai),
            "social" => Some(Self::Social),
            "providers" => Some(Self::ProviderCatalog),
            "hover" => Some(Self::ProviderHover),
            "codex" => Some(Self::CodexLogin),
            "generation" => Some(Self::Generation),
            "grok" => Some(Self::GrokLogin),
            "grok-auth" => Some(Self::GrokAuthorization),
            "compact" => Some(Self::Compact),
            "snapped" => Some(Self::Snapped),
            "assist" => Some(Self::Assist),
            "content" => Some(Self::Content),
            "history" => Some(Self::History),
            "brainstorm" => Some(Self::Brainstorm),
            "commands" => Some(Self::Commands),
            "library" => Some(Self::Library),
            "projects" => Some(Self::Projects),
            "projects-compact" => Some(Self::ProjectsCompact),
            _ => None,
        }
    }
}

fn primary_viewport_id(generation: ViewportGeneration) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("devpulse.chat.primary", generation.0))
}

struct Shared {
    project: project::State,
    registry: ChatWindowRegistry,
    lifetime: PeerLifetime,
    window: ChatWindow,
    /// The transcript catalog lives HERE, not in the window: paging a past
    /// session is IO, and the host owns IO. The window borrowed it once to
    /// build its index (H1 §3.2).
    catalog: HistoryCatalog,
    codex_activation: dpl_chat_codex::CodexActivationCoordinator,
    grok_activation: dpl_chat_grok::GrokActivationCoordinator,
    grok_runtime: Option<dpl_chat_grok::GrokAcpRuntime>,
    grok_runtime_retry: Option<dpl_chat_grok::GrokRuntimeRetryState>,
    grok_runtime_retry_at: Option<Instant>,
    grok_runtime_ready: bool,
    grok_sessions: BTreeMap<String, ChatSessionId>,
    grok_turns: BTreeMap<String, TurnId>,
    grok_permissions: BTreeMap<u64, PendingGrokPermission>,
    /// LIVE PROVIDER RUNTIMES, driven through the one seam path.
    ///
    /// Grok is deliberately absent: it owns a working bespoke runtime above,
    /// and converging it is its own pass (codex-host-wiring §5). Everything
    /// else that gains a transport lands here and costs the host no new code.
    runtimes: BTreeMap<ProviderId, Box<dyn ProviderRuntime>>,
    /// Keyed by PROVIDER and session, because two providers can mint the same
    /// session string — an unkeyed map would splice two conversations into one
    /// transcript.
    seam_sessions: BTreeMap<(ProviderId, String), ChatSessionId>,
    seam_turns: BTreeMap<(ProviderId, String), TurnId>,
    /// Approval cards a seam provider (Codex) is waiting on.
    seam_permissions: BTreeMap<(ProviderId, u64), PendingSeamPermission>,
    /// What each provider may do from chat: the operator's setting.
    permission_modes: BTreeMap<ProviderId, PermissionMode>,
    /// In-flight AI calls keyed by provider session id — the AI domain's
    /// correlation state. Mint at dispatch, first-token once, finish once.
    ai_calls: BTreeMap<String, AiCall>,
    grok_attempt: Option<ActivationAttemptId>,
    codex_attempt: Option<ActivationAttemptId>,
    imagine: ImagineState,
    assist: AssistPolicy,
    provider_events: ProviderReceiver,
    provider_runtime_enabled: bool,
    auth_browser: BrowserAuthCoordinator,
    activation_ledger: Option<ActivationLedger>,
    profile_store: Option<ProfileStore>,
    next_chat_session_id: u64,
    next_chat_event_id: u64,
    next_turn_id: u64,
    next_interaction_id: u64,
    primary_monitor: Option<([u32; 2], f32)>,
    /// A fabric handle for PUBLISHING the peer window's own navigation asks.
    /// The coordinator publishes `ShellEvent::NavigateRequest`; the shell
    /// drains it and navigates the main window.
    fabric: Fabric,
    /// THE ROSTER as last published — the comparison that makes the publish
    /// change-driven instead of per-frame chatter.
    roster: Vec<ProviderCapability>,
    /// The full `hub/sub` route census (BND1): /route validates against
    /// and navigates through THIS — the platform's registry, loaded
    /// live, never a hardcoded list.
    route_table: Vec<(String, Route)>,
    /// The Clyffy house session (BND1-M3a). Host-owned like the board;
    /// minted on first presence sync, never re-identified.
    clyffy: Option<ClyffySession>,
    /// The one-shot presence sweep: each vendor's own tool answering whether
    /// the operator is already signed in. Polled from the frame loop and
    /// dropped once it lands.
    presence: Option<std::sync::mpsc::Receiver<Vec<crate::presence::Presence>>>,
    /// Where the roster is remembered between launches (ADR-017 §2). `None`
    /// when the estate failed to open; the roster is derived, so the only
    /// cost is that the platform starts empty instead of remembered.
    estate: Option<Arc<dpl_estate::Estate>>,
    /// The session's determinism gate — slash mint site and UI channel
    /// share one state machine (determinism-intent-gate plan).
    determinism: dpl_intent::DeterminismGate,
    /// The live brainstorm board, if one is open (BS1). Host-owned: the
    /// document, its folder, and the undo reach all live here; the
    /// window gets a projection and sends typed ops back.
    brainstorm: Option<BrainstormSession>,
    #[cfg(feature = "eguipilotai-devtools")]
    pilot_runtime: Option<Arc<Mutex<PilotRuntime>>>,
}

/// One open brainstorm board and its undo reach (the composer
/// checkpoint shape: snapshots, dedupe at the call site, redo cleared
/// by any new edit).
struct BrainstormSession {
    dir: std::path::PathBuf,
    doc: dpl_workflows::BrainstormDoc,
    undo: Vec<dpl_workflows::BrainstormDoc>,
    redo: Vec<dpl_workflows::BrainstormDoc>,
    /// The last honest status line (a refusal's issues, a promote note).
    status: Option<String>,
    /// The in-flight suggestion call, if any (the imagine correlation
    /// shape: job minted at dispatch, one terminal transition).
    assist: Option<BrainstormAssistCall>,
}

struct BrainstormAssistCall {
    job: dpl_fabric::JobId,
    rx: std::sync::mpsc::Receiver<Result<Vec<String>, String>>,
}

const BRAINSTORM_UNDO_CAP: usize = 100;

impl BrainstormSession {
    /// Snapshot before an edit — dedupe, cap, and clear redo (the
    /// composer `checkpoint` contract).
    fn checkpoint(&mut self) {
        if self.undo.last() != Some(&self.doc) {
            if self.undo.len() >= BRAINSTORM_UNDO_CAP {
                self.undo.remove(0);
            }
            self.undo.push(self.doc.clone());
        }
        self.redo.clear();
    }
}

struct PendingGrokPermission {
    interaction: InteractionId,
    provider_session_id: String,
    allow_options: BTreeSet<String>,
}

/// One approval card a seam provider is waiting on.
struct PendingSeamPermission {
    interaction: InteractionId,
    provider_session_id: String,
    allow_options: BTreeSet<String>,
}

/// Chat Imagine generation state (ADR-011). `None` cache in authored
/// fixtures — deterministic proofs never touch the real Ring-0 cache.
struct ImagineState {
    cache: Option<ImagineCache>,
    policy: ImaginePolicy,
    /// One in-flight generation at a time; the worker reports through this.
    worker: Option<std::sync::mpsc::Receiver<Result<Vec<u8>, String>>>,
    pending_prompt: Option<String>,
    job: Option<JobId>,
    last_error: Option<String>,
}

impl ImagineState {
    fn in_flight(&self) -> bool {
        self.worker.is_some()
    }
}

/// Assist-mode policy — ONE toggle, ONE acknowledgement, blanketing ALL
/// providers (never per-provider). `acknowledged` persists the one-time
/// consent; the toggle asks it exactly once, ever. The `imagine-policy`
/// file shape: infallible load failing closed to Default, tmp-then-rename
/// save; authored fixtures never touch disk.
const ASSIST_POLICY_FILE: &str = "assist-policy.json";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct AssistPolicy {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    acknowledged: bool,
}

impl AssistPolicy {
    fn load(userdata: &std::path::Path) -> Self {
        std::fs::read(userdata.join(ASSIST_POLICY_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save(&self, userdata: &std::path::Path) -> std::io::Result<()> {
        let userdata = &dpl_roots::writable(userdata).map_err(std::io::Error::other)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let temporary = userdata.join(format!("{ASSIST_POLICY_FILE}.tmp"));
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(temporary, userdata.join(ASSIST_POLICY_FILE))
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// The estate key THE ROSTER is stored under (ADR-017 §2). `Tree::Agents`
/// already exists for "agent (chat provider) session state" — the capability
/// registry is exactly that.
const ROSTER_KEY: &str = "providers/roster";

/// THE ROSTER on disk.
///
/// A separate type from [`ProviderCapability`] on purpose: the in-memory
/// fact holds `ProviderId(&'static str)` and a live `Capability` set, and
/// NEITHER can be deserialized straight back — a `&'static str` cannot be
/// minted from disk bytes without leaking, and a capability this build no
/// longer has must not be resurrected. So the stored form speaks strings,
/// and restore RESOLVES them against what this build actually knows.
/// Unknown names are dropped, never invented (the doctor law).
///
/// Live process handles are deliberately unrepresentable here: this records
/// what was TRUE and is cheap to re-verify, never a connection.
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredProviderCapability {
    id: String,
    label: String,
    activated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    since: Option<String>,
}

/// The stored envelope: when it was true, and what was true.
///
/// The stamp is what lets a boot tell a roster written moments ago from one
/// left by a launch weeks back, without which "remembered" and "stale" look
/// identical on disk. Epoch ms because that is the clock this file already
/// keeps; the workspace has no date crate and a cosmetic string is not worth
/// one.
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredRoster {
    written_at_ms: u64,
    providers: Vec<StoredProviderCapability>,
}

fn store_roster(estate: Option<&dpl_estate::Estate>, roster: &[ProviderCapability]) {
    let Some(estate) = estate else {
        return;
    };
    let stored = StoredRoster {
        written_at_ms: now_unix_ms(),
        providers: roster
            .iter()
            .map(|p| StoredProviderCapability {
                id: p.id.0.to_owned(),
                label: p.label.clone(),
                activated: p.activated,
                note: p.note.clone(),
                capabilities: p.capabilities.iter().map(|c| c.slug().to_owned()).collect(),
                since: Some(p.since.clone()),
            })
            .collect(),
    };
    if let Err(error) = estate.put_json(dpl_estate::Tree::Agents, ROSTER_KEY, &stored) {
        // A roster that cannot be remembered is not a reason to fail a
        // launch — the live one is recomputed every boot regardless.
        tracing::warn!(%error, "the provider roster could not be persisted");
    }
}

/// Read the remembered roster, resolving every stored name against what this
/// build knows. `known` is the live registry's ids — a provider that no
/// longer exists is dropped rather than resurrected with a leaked id.
fn restore_roster(
    estate: Option<&dpl_estate::Estate>,
    known: &[ProviderId],
) -> Vec<ProviderCapability> {
    let Some(estate) = estate else {
        return Vec::new();
    };
    let stored: StoredRoster = match estate
        .get_json::<StoredRoster>(dpl_estate::Tree::Agents, ROSTER_KEY)
    {
        Ok(Some(stored)) => stored,
        Ok(None) => return Vec::new(),
        Err(error) => {
            tracing::warn!(%error, "the remembered provider roster is unreadable — starting empty");
            return Vec::new();
        }
    };
    stored
        .providers
        .into_iter()
        .filter_map(|row| {
            let id = *known.iter().find(|known| known.0 == row.id)?;
            Some(ProviderCapability {
                id,
                label: row.label,
                activated: row.activated,
                note: row.note,
                summary: String::new(),
                since: row.since.unwrap_or_default(),
                // The remembered roster records what was TRUE, not which of
                // the probe's states produced it, and the live sweep replaces
                // this within a second of boot. Claiming SignedOut here would
                // be inventing a reason the disk never carried.
                presence: if row.activated {
                    dpl_fabric::ProviderPresence::Ready
                } else {
                    dpl_fabric::ProviderPresence::Unavailable
                },
                capabilities: row
                    .capabilities
                    .iter()
                    .filter_map(|slug| Capability::from_slug(slug))
                    .collect(),
            })
        })
        .collect()
}

/// Take the presence sweep the moment it lands, then never look again.
///
/// A non-blocking poll on the frame path: the probe runs four short-lived
/// vendor processes on its own thread, and the egui thread only ever asks
/// whether the answer has arrived.
fn drain_presence(state: &mut Shared) {
    let Some(receiver) = state.presence.as_ref() else {
        return;
    };
    let found = match receiver.try_recv() {
        Ok(found) => found,
        Err(std::sync::mpsc::TryRecvError::Empty) => return,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            state.presence = None;
            return;
        }
    };
    state.presence = None;
    for one in &found {
        tracing::info!(provider = %one.provider, verdict = ?one.verdict, "provider presence");
        apply_presence(state, one);
    }
}

/// Fold one presence answer into the window's activation, if it has anything
/// to add.
///
/// It is deliberately NON-DESTRUCTIVE. A provider that is already activated
/// keeps whatever activated it — H1's Claude is `Experimental` off its
/// transcripts, and Codex and Grok own full activation coordinators with
/// launchers and runtimes. Presence's job is to fill in the providers nobody
/// has spoken for, and to state honestly what the machine says about the
/// rest. Overwriting a live coordinator's state from a one-shot probe is how
/// a working provider ends up reported as signed out mid-session.
fn apply_presence(state: &mut Shared, found: &crate::presence::Presence) {
    use dpl_chat_core::PresenceVerdict;

    let already_activated = state
        .window
        .provider_capabilities()
        .iter()
        .any(|p| p.id == found.provider && p.activated);
    if already_activated {
        return;
    }

    let note = found.note.clone().unwrap_or_default();
    let activation = match found.verdict {
        // Signed in, but nothing here qualifies a live turn transport yet —
        // say exactly that rather than claiming Ready.
        PresenceVerdict::SignedIn => ActivationState::Experimental {
            reason: "Signed in on this machine. Live turns are not qualified yet.".to_owned(),
            recovery: Vec::new(),
        },
        PresenceVerdict::SignedOut => ActivationState::SignedOut {
            reason: note,
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::SignIn,
                "Sign in with your browser",
            )],
        },
        PresenceVerdict::NotInstalled => ActivationState::NotInstalled {
            reason: note,
            recovery: Vec::new(),
        },
        // An answer nobody taught us to read is not evidence of anything.
        PresenceVerdict::Unknown => return,
    };

    if let Err(error) = state
        .window
        .set_provider_activation(found.provider, activation)
    {
        tracing::warn!(%error, provider = %found.provider, "presence could not be applied");
    }
}

/// Publish THE ROSTER when — and only when — it changed.
///
/// Change-detection rather than a list of "activation-affecting" intents:
/// activation is mutated from fourteen `set_provider_activation` sites plus
/// the provider-event drain, and any enumeration of them is one new call
/// site away from being silently wrong. Comparing the projection cannot be
/// forgotten. The cost is building a handful of small rows per frame, which
/// is far below the threshold law 6 protects.
fn republish_roster(state: &mut Shared) {
    let roster = state.window.provider_capabilities();
    if roster == state.roster {
        return;
    }
    state.fabric.publish_provider_roster(roster.clone());
    store_roster(state.estate.as_deref(), &roster);
    state.roster = roster;
    // Clyffy derives from the same truth — and rides the same change
    // gate, so a steady state pays one comparison, not a re-walk.
    sync_clyffy(state);
}

/// The sole owner of primary Chat viewport creation and peer lifetime policy.
pub struct ChatWindowCoordinator {
    shared: Arc<Mutex<Shared>>,
    events: ShellReceiver,
}

pub struct ChatWindowConfig {
    pub initial_route: Route,
    pub catalog: HistoryCatalog,
    pub routes: Vec<(String, Route)>,
    pub authored_models: Option<(SocialModel, InteractionRegistry)>,
    pub codex_activation: dpl_chat_codex::CodexActivationCoordinator,
    pub grok_activation: dpl_chat_grok::GrokActivationCoordinator,
    pub open_at_boot: bool,
    /// The Ring-0 estate, when it opened. THE ROSTER is remembered here so
    /// the platform knows what was activated last launch before any probe
    /// runs (ADR-017 §2).
    pub estate: Option<Arc<dpl_estate::Estate>>,
    #[cfg(feature = "eguipilotai-devtools")]
    pub proof_mode: Option<ChatProofMode>,
}

impl ChatWindowCoordinator {
    pub fn new(fabric: &Fabric, runtime: ChatRuntime, config: ChatWindowConfig) -> Self {
        let ChatWindowConfig {
            initial_route,
            catalog,
            routes,
            authored_models,
            codex_activation,
            grok_activation,
            open_at_boot,
            estate,
            #[cfg(feature = "eguipilotai-devtools")]
            proof_mode,
        } = config;
        let mut registry = ChatWindowRegistry::default();
        if open_at_boot {
            let id = registry.request_primary(WindowBinding::default());
            #[cfg(feature = "eguipilotai-devtools")]
            if proof_mode == Some(ChatProofMode::Compact) {
                registry
                    .update_geometry(id, Some([32, 32]), WindowGeometry::MINIMUM)
                    .expect("the authored compact proof geometry is valid");
            }
            #[cfg(feature = "eguipilotai-devtools")]
            if matches!(
                proof_mode,
                Some(ChatProofMode::Projects | ChatProofMode::ProjectsCompact)
            ) {
                let size = if proof_mode == Some(ChatProofMode::ProjectsCompact) {
                    [1100, 700]
                } else {
                    [1440, 900]
                };
                registry
                    .update_geometry(id, Some([32, 32]), size)
                    .expect("project proof geometry");
            }
            transition_opening(&mut registry, id);
        }
        let provider_runtime_enabled = authored_models.is_none();
        #[cfg(feature = "eguipilotai-devtools")]
        let project_storage_enabled = provider_runtime_enabled
            || (matches!(
                proof_mode,
                Some(ChatProofMode::Projects | ChatProofMode::ProjectsCompact)
            ) && std::env::var_os("DPL_CHAT_STATE_HOME").is_some());
        #[cfg(not(feature = "eguipilotai-devtools"))]
        let project_storage_enabled = provider_runtime_enabled;
        let (profile_store, provider_profiles) = if provider_runtime_enabled {
            match ProfileStore::open_default() {
                Ok(store) => {
                    let profiles = match store.list() {
                        Ok(profiles) => profiles,
                        Err(error) => {
                            tracing::warn!(%error, "Chat provider profiles are unreadable");
                            Vec::new()
                        }
                    };
                    (Some(store), profiles)
                }
                Err(error) => {
                    tracing::warn!(%error, "Chat provider profile store is unavailable");
                    (None, Vec::new())
                }
            }
        } else {
            (None, Vec::new())
        };
        // The host keeps its own copy of the route census: /route
        // validates against and navigates through it (BND1).
        let route_table = routes.clone();
        let mut window = match authored_models {
            Some((social, interactions)) => ChatWindow::with_models(
                runtime,
                initial_route,
                &catalog,
                routes,
                social,
                interactions,
            ),
            None => ChatWindow::new(runtime, initial_route, &catalog, routes),
        };
        window.replace_provider_profiles(provider_profiles);
        if dpl_roots::try_layout().is_ok() {
            window.set_room_library(load_room_library(&room_library_root()));
        } else {
            window.set_room_library(dpl_chat_core::rooms::Library::builtin());
        }
        // Trigger 0 of the content refresh policy (D1 plan §3.4): seed the
        // census at CONSTRUCTION. The window has more than one way to be
        // opened — `--chat-open` never raises a WindowRequest — so the open
        // path is a refresh, never the only read.
        if dpl_roots::try_layout().is_ok() {
            window.observe_content(read_content_view());
        }
        #[cfg(feature = "eguipilotai-devtools")]
        if let Some(mode) = proof_mode {
            if mode != ChatProofMode::Approval {
                window
                    .apply(dpl_chat_window::ChatWindowIntent::DismissInteraction(
                        dpl_chat_testkit::AUTHORED_APPROVAL_ID,
                    ))
                    .expect("the authored approval exists");
            }
            match mode {
                ChatProofMode::Social => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SelectDomain(
                            ChatDomain::Social,
                        ))
                        .expect("the authored Social domain is valid");
                }
                ChatProofMode::Content => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SelectDomain(
                            ChatDomain::Content,
                        ))
                        .expect("the Content domain is valid");
                    // Load a REAL spine so the proof shows slides, not an
                    // empty board: the first census row, whatever it is.
                    if let Some(first) = window.content_episode_ids().first().cloned() {
                        if let Some(spine) = read_content_spine(&first) {
                            window.observe_content_spine(spine);
                        }
                    }
                }
                ChatProofMode::History => {
                    // Open the operator's OWN newest transcript. If the rail
                    // found nothing the proof shot shows exactly that, which
                    // is the failure worth seeing.
                    if let Some((provider, locator)) = window.first_history_session() {
                        window
                            .apply(dpl_chat_window::ChatWindowIntent::SelectHistorySession {
                                provider,
                                locator,
                            })
                            .expect("the selected transcript is registered");
                        if let Ok(page) = catalog.page(provider, locator, None) {
                            window.observe_history_page(page);
                        }
                    }
                }
                ChatProofMode::ProviderHover => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SetOverlay(Some(
                            dpl_chat_window::DomainOverlay::Provider(ProviderId("codex")),
                        )))
                        .expect("the authored Codex provider exists");
                }
                ChatProofMode::ProviderCatalog => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SetOverlay(Some(
                            dpl_chat_window::DomainOverlay::ProviderCatalog,
                        )))
                        .expect("the provider catalog overlay is valid");
                }
                ChatProofMode::Generation => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::UpdateAiDraft {
                            provider: dpl_chat_codex::PROVIDER,
                            value: "a neon lighthouse over a data sea".to_owned(),
                        })
                        .expect("the authored generation prompt is valid");
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SetOverlay(Some(
                            dpl_chat_window::DomainOverlay::Generation,
                        )))
                        .expect("the generation overlay is valid");
                }
                ChatProofMode::Assist => {
                    // The acknowledgement itself is raised AFTER Shared
                    // exists (the interaction id rides its counter).
                }
                ChatProofMode::CodexLogin => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SelectProvider(
                            dpl_chat_codex::PROVIDER,
                        ))
                        .expect("the authored Codex provider exists");
                    window
                        .set_provider_activation(
                            dpl_chat_codex::PROVIDER,
                            ActivationState::SignedOut {
                                reason:
                                    "Log in with ChatGPT to activate subscription-backed Codex."
                                        .to_owned(),
                                recovery: vec![RecoveryAction::new(
                                    RecoveryActionKind::SignIn,
                                    "Log in with ChatGPT",
                                )],
                            },
                        )
                        .expect("the authored Codex sign-in ceremony is valid");
                }
                ChatProofMode::GrokLogin => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SelectProvider(
                            dpl_chat_grok::PROVIDER,
                        ))
                        .expect("the authored Grok provider exists");
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SetOverlay(Some(
                            dpl_chat_window::DomainOverlay::Provider(dpl_chat_grok::PROVIDER),
                        )))
                        .expect("the authored Grok provider overlay is valid");
                }
                ChatProofMode::GrokAuthorization => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SelectProvider(
                            dpl_chat_grok::PROVIDER,
                        ))
                        .expect("the authored Grok provider exists");
                    window
                        .set_provider_activation(
                            dpl_chat_grok::PROVIDER,
                            ActivationState::AwaitingAuthorization(DeviceAuthorizationPrompt {
                                provider_id: dpl_chat_grok::PROVIDER,
                                verification_url: GuidedValue::provider_authoritative(
                                    "https://accounts.x.ai/oauth2/device?user_code=TEST-CODE"
                                        .to_owned(),
                                    false,
                                    GuidedValueSensitivity::TransientSecret,
                                ),
                                user_code: GuidedValue::provider_authoritative(
                                    "TEST-CODE".to_owned(),
                                    true,
                                    GuidedValueSensitivity::TransientSecret,
                                ),
                            }),
                        )
                        .expect("the authored Grok authorization is valid");
                }
                ChatProofMode::Brainstorm => {
                    // A deterministic worked board, in memory only —
                    // authored proofs never touch the content tree. It
                    // stands at Cluster with the assist item unsorted,
                    // so the proof shows the sort affordance AND the
                    // origin disclosure at once.
                    let scaffold = dpl_workflows::brainstorm_scaffold("breakdown")
                        .expect("the scaffold census ships breakdown");
                    let mut doc = dpl_workflows::mint_brainstorm(&scaffold, "authored-proof");
                    use dpl_workflows::{BrainstormOp as Op, BrainstormOrigin as Origin};
                    for op in [
                        Op::Seed {
                            seed: "One video that explains the whole product".into(),
                        },
                        Op::AddItem {
                            id: "pain".into(),
                            text: "Name the pain".into(),
                            parent: None,
                            origin: Origin::Operator,
                        },
                        Op::AddItem {
                            id: "demo".into(),
                            text: "Show it working".into(),
                            parent: None,
                            origin: Origin::Assist,
                        },
                        Op::AdvancePhase,
                        Op::AddCluster {
                            id: "open".into(),
                            label: "The opening".into(),
                        },
                        Op::AssignCluster {
                            id: "pain".into(),
                            cluster: Some("open".into()),
                        },
                    ] {
                        dpl_workflows::apply(&mut doc, &op)
                            .expect("the authored board ops are legal");
                    }
                    window.observe_brainstorm(dpl_chat_window::BrainstormView {
                        doc: Some(doc),
                        status: None,
                        can_undo: true,
                        can_redo: false,
                    });
                }
                ChatProofMode::Commands => {
                    // A `/ro` draft opens the strip over the census —
                    // /route and /routines offered, asks in the tips.
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::UpdateAiDraft {
                            provider: dpl_chat_codex::PROVIDER,
                            value: "/ro".to_owned(),
                        })
                        .expect("the authored codex draft is valid");
                }
                ChatProofMode::Library => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::ToggleRoomLibrary)
                        .expect("the library opens");
                }
                ChatProofMode::Approval
                | ChatProofMode::Ai
                | ChatProofMode::Compact
                | ChatProofMode::Snapped
                | ChatProofMode::Projects
                | ChatProofMode::ProjectsCompact => {}
            }
        }
        #[cfg(feature = "eguipilotai-devtools")]
        if matches!(
            proof_mode,
            Some(ChatProofMode::Projects | ChatProofMode::ProjectsCompact)
        ) {
            let archive = window.project_archive().expect("authored project archive");
            window
                .restore_project_archive(&archive)
                .expect("authored read-only project chats");
            window.open_project_setup();
        }
        if open_at_boot {
            if let Some((interaction_id, placement)) = window.first_utility_interaction() {
                request_utility_record(&mut registry, interaction_id, placement);
            }
        }
        let primary_monitor = dpl_engine_capture::list_monitors()
            .into_iter()
            .find(|monitor| monitor.is_primary)
            .map(|monitor| {
                (
                    [monitor.width, monitor.height],
                    monitor.scale_factor.max(1.0),
                )
            });
        let probe_grok_cached_subscription =
            provider_runtime_enabled && grok_activation.activation_capability_qualified();
        let mut saved_permission_modes = BTreeMap::new();
        let activation_ledger = if !provider_runtime_enabled {
            None
        } else {
            match ActivationLedger::open_default() {
                Ok(ledger) => {
                    match ledger.get(dpl_chat_grok::PROVIDER) {
                        Ok(Some(evidence)) => tracing::info!(
                            state = ?evidence.state,
                            version = %evidence.client_version,
                            method = %evidence.auth_method,
                            "loaded non-secret Grok activation evidence"
                        ),
                        Ok(None) => tracing::info!("no prior Grok activation evidence"),
                        Err(error) => tracing::warn!(%error, "Grok activation evidence unreadable"),
                    }
                    match ledger.get(dpl_chat_codex::PROVIDER) {
                        Ok(Some(evidence)) => tracing::info!(
                            state = ?evidence.state,
                            version = %evidence.client_version,
                            method = %evidence.auth_method,
                            "loaded non-secret Codex activation evidence"
                        ),
                        Ok(None) => tracing::info!("no prior Codex activation evidence"),
                        Err(error) => {
                            tracing::warn!(%error, "Codex activation evidence unreadable")
                        }
                    }
                    for provider in [dpl_chat_codex::PROVIDER] {
                        match ledger.permission_mode(provider) {
                            Ok(Some(mode)) => {
                                saved_permission_modes.insert(provider, mode);
                            }
                            Ok(None) => {}
                            Err(error) => {
                                tracing::warn!(%provider, %error, "saved permission mode unreadable");
                            }
                        }
                    }
                    Some(ledger)
                }
                Err(error) => {
                    tracing::warn!(%error, "Chat activation ledger is unavailable");
                    None
                }
            }
        };
        // Persist only evidence supplied by an already configured coordinator.
        // Production boots inert; project recovery discovers its workspace
        // asynchronously. Ready is advisory evidence, never launch authority.
        if provider_runtime_enabled {
            if let Some(ledger) = activation_ledger.as_ref() {
                let version = codex_activation.runtime_profile().map_or_else(
                    || "unknown".to_owned(),
                    |profile| profile.version().to_string(),
                );
                let recorded = match codex_activation.activation() {
                    ActivationState::Ready => {
                        ledger.record_ready(dpl_chat_codex::PROVIDER, &version, "chatgpt")
                    }
                    ActivationState::SignedOut { .. } => {
                        ledger.record_signed_out(dpl_chat_codex::PROVIDER, &version, "chatgpt")
                    }
                    _ => Ok(()),
                };
                if let Err(error) = recorded {
                    tracing::warn!(%error, "Codex boot evidence could not be stored");
                }
            }
        }
        let imagine = if provider_runtime_enabled {
            ImagineState {
                cache: Some(ImagineCache::open_default()),
                policy: ImaginePolicy::load(&dpl_roots::userdata_root()),
                worker: None,
                pending_prompt: None,
                job: None,
                last_error: None,
            }
        } else {
            ImagineState {
                cache: None,
                policy: ImaginePolicy::default(),
                worker: None,
                pending_prompt: None,
                job: None,
                last_error: None,
            }
        };
        if provider_runtime_enabled {
            window.set_generation_status(generation_status_for(&imagine));
            window.replace_generation_drafts(imagine_draft_views(&imagine));
        } else {
            // Authored fixtures get a deterministic, enabled panel; the
            // host still refuses to spend (activation-style fixture gate).
            window.set_generation_status(GenerationStatus {
                image_available: true,
                unavailable_reason: String::new(),
                in_flight: false,
                last_error: None,
            });
        }
        // Assist mode: the persisted blanket consent (authored fixtures
        // start from defaults and never write disk).
        let assist = if provider_runtime_enabled {
            AssistPolicy::load(&dpl_roots::userdata_root())
        } else {
            AssistPolicy::default()
        };
        window.set_assist(assist.enabled);
        let mut shared = Shared {
            project: project::State::new(project_storage_enabled, provider_runtime_enabled),
            registry,
            lifetime: PeerLifetime::default(),
            window,
            catalog,
            codex_activation,
            grok_activation,
            grok_runtime: None,
            grok_runtime_retry: None,
            grok_runtime_retry_at: None,
            grok_runtime_ready: false,
            grok_sessions: BTreeMap::new(),
            grok_turns: BTreeMap::new(),
            ai_calls: BTreeMap::new(),
            grok_permissions: BTreeMap::new(),
            runtimes: BTreeMap::new(),
            seam_sessions: BTreeMap::new(),
            seam_turns: BTreeMap::new(),
            seam_permissions: BTreeMap::new(),
            permission_modes: saved_permission_modes,
            grok_attempt: None,
            codex_attempt: None,
            imagine,
            assist,
            provider_events: fabric.subscribe_providers(),
            provider_runtime_enabled,
            auth_browser: BrowserAuthCoordinator::new(),
            activation_ledger,
            profile_store,
            next_chat_session_id: 1_000_000,
            next_chat_event_id: 1_000_000,
            next_turn_id: 1,
            next_interaction_id: 2_000_000_000,
            primary_monitor,
            fabric: fabric.clone(),
            roster: Vec::new(),
            route_table,
            clyffy: None,
            presence: provider_runtime_enabled
                .then(|| crate::presence::probe_all(crate::presence::specs())),
            estate,
            determinism: dpl_intent::DeterminismGate::default(),
            brainstorm: None,
            #[cfg(feature = "eguipilotai-devtools")]
            pilot_runtime: None,
        };
        #[cfg(feature = "eguipilotai-devtools")]
        if proof_mode == Some(ChatProofMode::Assist) {
            let interaction = InteractionId(shared.next_interaction_id);
            shared.next_interaction_id = shared.next_interaction_id.saturating_add(1);
            if let Err(error) = shared.window.register_assist_acknowledgement(interaction) {
                tracing::warn!(%error, "assist proof state could not raise the acknowledgement");
            }
        }
        #[cfg(feature = "eguipilotai-devtools")]
        if proof_mode == Some(ChatProofMode::GrokAuthorization) {
            let attempt = shared.fabric.mint_provider_attempt();
            shared.grok_attempt = Some(attempt);
            shared
                .fabric
                .publish_provider(ProviderEvent::ActivationRequested {
                    provider: dpl_chat_grok::PROVIDER,
                    attempt,
                });
            shared
                .fabric
                .publish_provider(ProviderEvent::DeviceCodeReceived {
                    provider: dpl_chat_grok::PROVIDER,
                    attempt,
                    code_len: 9,
                });
            shared
                .fabric
                .publish_provider(ProviderEvent::SystemBrowserDispatched {
                    provider: dpl_chat_grok::PROVIDER,
                    attempt,
                });
            drain_provider_facts(&mut shared);
        }
        if probe_grok_cached_subscription {
            begin_grok_runtime_verification(
                &mut shared,
                dpl_chat_grok::GrokVerificationOrigin::Startup,
            );
            drive_grok_runtime(&mut shared, Instant::now());
        }
        if provider_runtime_enabled {
            let now = now_unix_ms();
            let due = shared
                .imagine
                .cache
                .as_ref()
                .is_some_and(|cache| cache.retention_due(now));
            if due {
                if shared.imagine.policy.auto_flush {
                    if let Some(cache) = shared.imagine.cache.as_mut() {
                        let flushed = cache.flush_expired(now);
                        tracing::info!(flushed, "expired Imagine drafts were auto-flushed");
                    }
                    project_imagine(&mut shared);
                } else {
                    let expired = shared
                        .imagine
                        .cache
                        .as_ref()
                        .map(|cache| {
                            cache
                                .drafts()
                                .iter()
                                .filter(|draft| draft.expired(now))
                                .count()
                        })
                        .unwrap_or_default();
                    let interaction = InteractionId(shared.next_interaction_id);
                    shared.next_interaction_id = shared.next_interaction_id.saturating_add(1);
                    if let Err(error) = shared
                        .window
                        .register_imagine_flush_prompt(interaction, expired)
                    {
                        tracing::warn!(%error, "Imagine flush prompt could not be raised");
                    }
                }
            }
        }
        // Trigger 0 for THE ROSTER, the same lesson D1's census learned: the
        // window has more than one way to be opened, so the platform's view
        // of what providers can do is seeded at CONSTRUCTION, never only on
        // the open path.
        //
        // The remembered roster is published FIRST so a consumer reading in
        // the gap sees what was true last launch rather than nothing at all
        // (ADR-017 §2); the live projection immediately follows and wins
        // wherever the two disagree. Recomputing is always the authority —
        // persistence only decides what is shown before it lands.
        let known: Vec<ProviderId> = shared
            .window
            .provider_capabilities()
            .into_iter()
            .map(|p| p.id)
            .collect();
        let remembered = restore_roster(shared.estate.as_deref(), &known);
        if !remembered.is_empty() {
            shared.fabric.publish_provider_roster(remembered.clone());
            shared.roster = remembered;
        }
        republish_roster(&mut shared);

        Self {
            shared: Arc::new(Mutex::new(shared)),
            events: fabric.subscribe_shell(),
        }
    }

    pub fn flush_project_on_exit(&self) {
        project::flush_on_exit(&mut lock(&self.shared));
    }

    pub fn attach_determinism(&self, gate: dpl_intent::DeterminismGate) {
        lock(&self.shared).determinism = gate;
    }

    #[cfg(feature = "eguipilotai-devtools")]
    pub fn attach_pilot_runtime(&self, runtime: Arc<Mutex<PilotRuntime>>) {
        lock(&self.shared).pilot_runtime = Some(runtime);
    }

    #[cfg(feature = "eguipilotai-devtools")]
    pub fn consume_pilot_close_requests(&self, ctx: &egui::Context) {
        consume_pilot_close_requests(ctx, &self.shared);
    }

    pub fn root_close_requested(&self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested())
            && !lock(&self.shared).project.pending_root_close
        {
            return;
        }
        let decision = {
            let mut shared = lock(&self.shared);
            let chat_live = shared.registry.is_live(ChatWindowId::PRIMARY);
            shared.lifetime.on_root_close(chat_live)
        };
        if decision == RootCloseDecision::HideMain {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            let mut shared = lock(&self.shared);
            if project::can_close(&mut shared, ctx) {
                if shared.project.pending_root_close {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                shared.project.pending_root_close = false;
            } else {
                shared.project.pending_root_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
    }

    pub fn frame(&mut self, ctx: &egui::Context, current_route: Route) {
        service_project(ctx, &self.shared);
        let mut open_requested = false;
        {
            let mut shared = lock(&self.shared);
            shared.window.observe_route(current_route);
            drain_provider_facts(&mut shared);
            drain_presence(&mut shared);
            // Every activation change — from the drain above or from any of
            // the host intents applied last frame — reaches the platform
            // here. Publishing is change-gated, so a steady state costs one
            // comparison.
            republish_roster(&mut shared);
            while let Ok(event) = self.events.try_recv() {
                match event {
                    ShellEvent::RouteChanged { route } => shared.window.observe_route(route),
                    ShellEvent::WindowRequest {
                        target,
                        action: AppWindowAction::OpenOrFocus,
                    } if target == dpl_chat_window::WINDOW_KEY => open_requested = true,
                    ShellEvent::ViziwizSummon
                    | ShellEvent::FullBleedRequest(_)
                    | ShellEvent::NavigateRequest { .. }
                    | ShellEvent::WindowRequest { .. } => {}
                }
            }
        }
        if open_requested {
            // Trigger 1 of the content refresh policy (D1 plan §3.4): read the
            // census when the window opens. NEVER on the frame path — this
            // function runs every frame and the census is a directory walk.
            let view = read_content_view();
            lock(&self.shared).window.observe_content(view);
            self.open_or_focus(ctx);
        }

        let primary = {
            let shared = lock(&self.shared);
            shared
                .registry
                .record(ChatWindowId::PRIMARY)
                .filter(|record| {
                    matches!(
                        record.lifecycle,
                        WindowLifecycle::Opening | WindowLifecycle::Visible
                    )
                })
                .cloned()
        };
        let Some(primary) = primary else {
            return;
        };
        let generation = primary.generation;
        let chat_viewport_id = primary_viewport_id(generation);
        if ctx.pilot_take_viewport_close_request(chat_viewport_id) {
            close_chat(ctx, &self.shared, generation);
            return;
        }
        let geometry = primary.geometry;

        let mut builder = egui::ViewportBuilder::default()
            .with_title("DevPULSE Chat")
            .with_inner_size([geometry.inner_size[0] as f32, geometry.inner_size[1] as f32])
            .with_min_inner_size([
                dpl_chat_windows::WindowGeometry::MINIMUM[0] as f32,
                dpl_chat_windows::WindowGeometry::MINIMUM[1] as f32,
            ])
            .with_resizable(true)
            .with_decorations(false)
            .with_taskbar(true);
        if let Some(position) = geometry.outer_position {
            builder = builder.with_position([position[0] as f32, position[1] as f32]);
        }

        let shared = Arc::clone(&self.shared);
        ctx.pilot_show_viewport_deferred(
            dpl_chat_window::viewport_key(),
            chat_viewport_id,
            builder,
            move |ui, class| {
                if class == egui::ViewportClass::EmbeddedWindow {
                    embedded_failure(ui, &shared);
                    return;
                }
                // Native peer callbacks continue when the main viewport is
                // hidden. Never make project progress depend on root paint.
                service_project(ui.ctx(), &shared);
                let current_generation_is_live = {
                    let state = lock(&shared);
                    state
                        .registry
                        .record(ChatWindowId::PRIMARY)
                        .is_some_and(|record| {
                            record.generation == generation
                                && matches!(
                                    record.lifecycle,
                                    WindowLifecycle::Opening | WindowLifecycle::Visible
                                )
                        })
                };
                if !current_generation_is_live {
                    return;
                }

                let viewport = ui.ctx().input(|input| input.viewport().clone());
                let close_requested = viewport.close_requested();
                let geometry = viewport_geometry(&viewport);

                let (
                    intents,
                    activation_changed,
                    runtime_changed,
                    provider_runtime_live,
                    login_running,
                ) = {
                    let mut state = lock(&shared);
                    if matches!(
                        state
                            .registry
                            .record(ChatWindowId::PRIMARY)
                            .map(|record| &record.lifecycle),
                        Some(WindowLifecycle::Opening)
                    ) {
                        if let Err(error) = state.registry.mark_visible(ChatWindowId::PRIMARY) {
                            tracing::error!(%error, "Chat viewport failed to become visible");
                        }
                    }
                    if let Some((position, size)) = geometry {
                        if let Err(error) =
                            state
                                .registry
                                .update_geometry(ChatWindowId::PRIMARY, position, size)
                        {
                            tracing::warn!(%error, "Chat viewport geometry was rejected");
                        }
                    }
                    let frozen = project::frozen(&state);
                    let activation_changed = !frozen && poll_provider_activations(&mut state);
                    let runtime_changed = !frozen
                        && (poll_grok_runtime(&mut state)
                            | drive_grok_runtime(&mut state, Instant::now())
                            | poll_seam_runtimes(&mut state)
                            | drive_codex_runtime(&mut state)
                            | poll_imagine(&mut state)
                            | poll_brainstorm_assist(&mut state));
                    let provider_runtime_live = state.grok_runtime.is_some()
                        || state.grok_runtime_retry_at.is_some()
                        || !state.runtimes.is_empty();
                    let login_running = state.codex_activation.is_login_running()
                        || state.grok_activation.is_login_running()
                        || state.imagine.in_flight();
                    let intents = egui::CentralPanel::default()
                        .frame(
                            egui::Frame::new()
                                .fill(dpl_coldlight::surface::WELL)
                                .inner_margin(egui::Margin::ZERO),
                        )
                        .show(ui, |ui| state.window.ui(ui))
                        .inner;
                    (
                        intents,
                        activation_changed,
                        runtime_changed,
                        provider_runtime_live,
                        login_running,
                    )
                };

                let mut host_intents = Vec::new();
                let mut window_changed = false;
                {
                    let mut state = lock(&shared);
                    for intent in intents {
                        match state.window.apply(intent) {
                            Ok(Some(host)) => {
                                window_changed = true;
                                host_intents.push(host);
                            }
                            Ok(None) => window_changed = true,
                            Err(error) => {
                                tracing::warn!(%error, "Chat window intent was rejected");
                            }
                        }
                    }
                }
                for intent in host_intents {
                    apply_host_intent(ui.ctx(), &shared, intent);
                }
                if window_changed || activation_changed || runtime_changed {
                    ui.ctx().request_repaint_of(chat_viewport_id);
                    // Either viewport can service the shared host; wake both
                    // so saving never depends on which one remains visible.
                    ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                }
                if login_running || provider_runtime_live {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(100));
                }

                if close_requested {
                    close_chat(ui.ctx(), &shared, generation);
                    return;
                }
                declare_utilities(ui.ctx(), &shared);
                #[cfg(feature = "eguipilotai-devtools")]
                {
                    pump_pilot_runtime(&shared);
                    consume_pilot_close_requests(ui.ctx(), &shared);
                }
            },
        );
    }

    fn open_or_focus(&self, ctx: &egui::Context) {
        let generation = {
            let mut shared = lock(&self.shared);
            match shared
                .registry
                .record(ChatWindowId::PRIMARY)
                .map(|record| record.lifecycle.clone())
            {
                None => {
                    let id = shared.registry.request_primary(WindowBinding::default());
                    transition_opening(&mut shared.registry, id);
                }
                Some(WindowLifecycle::Closed | WindowLifecycle::Failed { .. }) => {
                    if let Err(error) = shared.registry.reopen(ChatWindowId::PRIMARY) {
                        tracing::error!(%error, "Chat viewport failed to reopen");
                    } else {
                        transition_opening(&mut shared.registry, ChatWindowId::PRIMARY);
                    }
                }
                Some(WindowLifecycle::Requested) => {
                    transition_opening(&mut shared.registry, ChatWindowId::PRIMARY);
                }
                Some(
                    WindowLifecycle::Opening | WindowLifecycle::Visible | WindowLifecycle::Closing,
                ) => {}
            }
            current_generation(&shared.registry, ChatWindowId::PRIMARY)
        };
        let Some(generation) = generation else {
            return;
        };
        let chat_viewport_id = primary_viewport_id(generation);
        ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Focus);
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }
}

fn transition_opening(registry: &mut ChatWindowRegistry, id: ChatWindowId) {
    if let Err(error) = registry.mark_opening(id) {
        tracing::error!(%error, "Chat viewport failed to enter opening state");
    }
}

fn viewport_geometry(viewport: &egui::ViewportInfo) -> Option<(Option<[i32; 2]>, [u32; 2])> {
    let inner = viewport.inner_rect?;
    let width = inner.width().round();
    let height = inner.height().round();
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let position = viewport.outer_rect.and_then(|outer| {
        let x = outer.min.x.round();
        let y = outer.min.y.round();
        (x.is_finite() && y.is_finite()).then_some([x as i32, y as i32])
    });
    Some((position, [width as u32, height as u32]))
}

/// The address grammar the Workbench's `open_episode` already accepts:
/// `"<show>/<episode>"` for operator content, `"template/<episode>"` for a
/// seed. Minted here so Chat's addresses ARE the harness's addresses.
fn content_address(entry: &dpl_doc::EpisodeEntry) -> ContentEpisodeId {
    ContentEpisodeId(match entry.show.as_deref() {
        Some(show) => format!("{show}/{}", entry.id),
        None => format!("template/{}", entry.id),
    })
}

/// Resolve an address back to its entry — the inverse of [`content_address`],
/// matching `dpl_hub_workbench::open_episode`'s resolution exactly.
fn find_content_entry(id: &ContentEpisodeId) -> Option<dpl_doc::EpisodeEntry> {
    let union = dpl_doc::all_episodes(&dpl_roots::content_root());
    match id.0.split_once('/') {
        Some(("template", ep)) => union.into_iter().find(|e| e.show.is_none() && e.id == ep),
        Some((show, ep)) => union
            .into_iter()
            .find(|e| e.show.as_deref() == Some(show) && e.id == ep),
        None => union.into_iter().find(|e| e.id == id.0),
    }
}

/// The content census — a directory walk, never a full library load. Called
/// on the triggers named in the D1 plan §3.4, NEVER from the frame path.
fn read_content_view() -> ContentView {
    let root = dpl_roots::content_root();
    // A missing content root is a NOTE, never an early return: `all_episodes`
    // unions the root with `seeds/episodes/`, which lives outside it, so the
    // seed templates are still real census rows and must still be listed.
    let error = (!root.exists())
        .then(|| format!("No content root yet at {} — showing seeds.", root.display()));
    let episodes = dpl_doc::all_episodes(&root)
        .into_iter()
        .map(|entry| {
            // A title needs the episode document; an unreadable one is a row
            // with a fallback name, never a missing row.
            let title = dpl_doc::episode::summary(&entry.dir, &root)
                .map_or_else(|| entry.id.clone(), |ep| ep.title);
            ContentEpisode {
                id: content_address(&entry),
                show: entry.show.clone(),
                title,
            }
        })
        .collect();
    ContentView {
        episodes,
        spine: None,
        error,
    }
}

/// One episode's chapter→slide structure, read through the one episode
/// walk (LS3b) — an episode built from lessons and a seed template alike.
/// Only this episode is read.
fn read_content_spine(id: &ContentEpisodeId) -> Option<ContentSpine> {
    let entry = find_content_entry(id)?;
    let shown = dpl_doc::episode::shows(&entry.dir, &dpl_roots::content_root()).ok()?;
    let chapters = shown
        .chapters
        .iter()
        .map(|chapter| ContentChapter {
            id: chapter.id.clone(),
            title: chapter.title.clone(),
            slides: chapter
                .cards
                .iter()
                .map(|card| ContentSlide {
                    chapter: chapter.title.clone(),
                    name: card.label.clone(),
                    // A card that does not load is a displayable fact,
                    // never a failed episode.
                    template: card.slide.as_ref().map_or_else(
                        |_| "unknown".to_owned(),
                        |slide| slide.component_id().to_owned(),
                    ),
                    index: card.index,
                })
                .collect(),
        })
        .collect();
    Some(ContentSpine {
        id: id.clone(),
        title: shown.title,
        chapters,
    })
}

/// Idempotent shared host servicing for BOTH root and independent peer frames.
/// Commands and close retries run outside the Shared lock because they enter
/// the same host handlers as ordinary native UI actions.
fn service_project(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>) {
    let (verified, pending_chat_close, pending_root_close) = {
        let mut state = lock(shared);
        project::boot(&mut state, ctx);
        project::poll(&mut state, ctx);
        project::save(&mut state, ctx, false);
        (
            project::take_verified(&mut state),
            state.project.pending_chat_close,
            state.project.pending_root_close,
        )
    };
    if let Some(intent) = verified {
        apply_host_intent_inner(ctx, shared, intent);
    }
    if let Some(generation) = pending_chat_close {
        close_chat(ctx, shared, generation);
    }
    if pending_root_close {
        let mut state = lock(shared);
        if project::can_close(&mut state, ctx) {
            state.project.pending_root_close = false;
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        } else {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::CancelClose);
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn apply_host_intent(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>, intent: HostIntent) {
    if project::frozen(&lock(shared)) {
        return;
    }
    if matches!(
        intent,
        HostIntent::NewSession(_) | HostIntent::SendTurn { .. } | HostIntent::ActivateProvider(_)
    ) {
        project::verify_intent(&mut lock(shared), ctx, intent);
        return;
    }
    apply_host_intent_inner(ctx, shared, intent);
}

fn apply_host_intent_inner(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>, intent: HostIntent) {
    match intent {
        HostIntent::SetupProject { workspace, label } => project::request(
            &mut lock(shared),
            ctx,
            project::Request::Setup { workspace, label },
        ),
        HostIntent::SaveProjectCopy { workspace, label } => project::request(
            &mut lock(shared),
            ctx,
            project::Request::SaveCopy { workspace, label },
        ),
        HostIntent::OpenProject { workspace } => {
            project::request(&mut lock(shared), ctx, project::Request::Open(workspace))
        }
        HostIntent::SelectProject(id) => {
            project::request(&mut lock(shared), ctx, project::Request::Select(id))
        }
        HostIntent::RetryProjectSave => project::save(&mut lock(shared), ctx, true),
        HostIntent::OpenUtility {
            interaction_id,
            placement,
        } => open_utility(ctx, shared, interaction_id, placement),
        HostIntent::ActivateProvider(provider) => {
            activate_provider(shared, provider);
            ctx.request_repaint();
        }
        HostIntent::LoadHistoryPage { provider, locator } => {
            // The host owns the catalog because reading a transcript is IO.
            // A failed read is reported to the operator, never swallowed —
            // an empty pane with no reason is how the count label happened.
            let mut state = lock(shared);
            match state.catalog.page(provider, locator, None) {
                Ok(page) => state.window.observe_history_page(page),
                Err(error) => {
                    tracing::warn!(%error, %provider, "chat transcript page unavailable");
                }
            }
            drop(state);
            ctx.request_repaint();
        }
        HostIntent::ProviderSelected(provider) => {
            lock(shared)
                .fabric
                .publish_provider(ProviderEvent::Selected { provider });
            ctx.request_repaint();
        }
        HostIntent::SaveProviderProfile(draft) => {
            save_provider_profile(shared, draft);
            ctx.request_repaint();
        }
        HostIntent::CopyProviderAuthorizationCode(provider) => {
            copy_provider_authorization_code(ctx, shared, provider);
            ctx.request_repaint();
        }
        HostIntent::OpenProviderAuthorization(provider) => {
            let mut state = lock(shared);
            if provider != dpl_chat_grok::PROVIDER {
                tracing::warn!(%provider, "provider authorization browser is not qualified");
            } else {
                open_grok_authorization_browser(&mut state, true);
            }
            ctx.request_repaint();
        }
        HostIntent::OpenProviderSetup(provider)
            if provider == dpl_chat_codex::PROVIDER
                && dpl_chat_codex::adoption::installed_codex().is_some() =>
        {
            update_codex(ctx, shared);
        }
        HostIntent::OpenProviderSetup(provider) => {
            let url = match provider {
                id if id == dpl_chat_codex::PROVIDER => dpl_chat_codex::SETUP_URL,
                id if id == dpl_chat_grok::PROVIDER => dpl_chat_grok::SETUP_URL,
                _ => {
                    tracing::warn!(%provider, "provider setup action is not qualified");
                    return;
                }
            };
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
        HostIntent::RefreshProvider(provider) => {
            let mut state = lock(shared);
            if provider == dpl_chat_codex::PROVIDER {
                match state.codex_activation.refresh_discovery() {
                    Ok(_) => {
                        let activation = state.codex_activation.activation().clone();
                        if let Err(error) =
                            state.window.set_provider_activation(provider, activation)
                        {
                            tracing::warn!(%error, "Codex rediscovery state could not be projected");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Codex rediscovery was rejected");
                    }
                }
            } else if provider == dpl_chat_grok::PROVIDER {
                match state.grok_activation.refresh_discovery() {
                    Ok(_) => {
                        let activation = state.grok_activation.activation().clone();
                        if let Err(error) =
                            state.window.set_provider_activation(provider, activation)
                        {
                            tracing::warn!(%error, "Grok rediscovery state could not be projected");
                        }
                        if state.grok_activation.activation_capability_qualified() {
                            request_grok_runtime_shutdown(&mut state);
                            begin_grok_runtime_verification(
                                &mut state,
                                dpl_chat_grok::GrokVerificationOrigin::Recovery,
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Grok rediscovery was rejected");
                    }
                }
            } else {
                tracing::warn!(%provider, "provider rediscovery is not qualified");
                return;
            }
            ctx.request_repaint();
        }
        HostIntent::NewSession(provider) => {
            let mut state = lock(shared);
            if !project::require_ready(&mut state) {
                return;
            }
            if provider == dpl_chat_grok::PROVIDER {
                send_grok_command(&state, dpl_chat_grok::GrokRuntimeCommand::NewSession);
            } else if state.runtimes.contains_key(&provider) {
                send_seam_command(&mut state, provider, RuntimeCommand::NewSession);
            } else {
                tracing::warn!(%provider, "new-session runtime is not qualified");
                return;
            }
            ctx.request_repaint();
        }
        HostIntent::SendTurn {
            provider,
            session,
            text,
        } => {
            let mut state = lock(shared);
            if !project::require_ready(&mut state) {
                return;
            }
            if !project::live_session(&state, provider, session) {
                state.window.set_composer_status(
                    "This saved chat is read-only. Start a new chat for this project.",
                );
                return;
            }
            if provider == dpl_chat_grok::PROVIDER {
                send_grok_turn(&mut state, session, text);
            } else if state.runtimes.contains_key(&provider) {
                send_seam_turn(&mut state, provider, session, text);
            } else {
                tracing::warn!(%provider, "live turn runtime is not qualified");
                return;
            }
            ctx.request_repaint();
        }
        HostIntent::CancelTurn { provider, session } => {
            let mut state = lock(shared);
            let Some(provider_session_id) = provider_session_id(&state, provider, session) else {
                tracing::warn!(%provider, %session, "provider session binding is missing");
                return;
            };
            if provider == dpl_chat_grok::PROVIDER {
                cancel_grok_permissions(&mut state, Some(&provider_session_id), true);
                send_grok_command(
                    &state,
                    dpl_chat_grok::GrokRuntimeCommand::Cancel {
                        provider_session_id,
                    },
                );
            } else if state.runtimes.contains_key(&provider) {
                send_seam_command(
                    &mut state,
                    provider,
                    RuntimeCommand::Cancel {
                        provider_session_id,
                    },
                );
            } else {
                tracing::warn!(%provider, "turn cancellation runtime is not qualified");
                return;
            }
            ctx.request_repaint();
        }
        HostIntent::ResolveProviderPermission {
            provider,
            request_id,
            option_id,
        } => {
            if provider != dpl_chat_grok::PROVIDER {
                let mut state = lock(shared);
                if let Some(pending) = state.seam_permissions.remove(&(provider, request_id)) {
                    let accepted = option_id
                        .as_ref()
                        .is_some_and(|option| pending.allow_options.contains(option));
                    append_seam_event(
                        &mut state,
                        provider,
                        &pending.provider_session_id,
                        ChatEventContent::ApprovalResolved { accepted },
                    );
                    send_seam_command(
                        &mut state,
                        provider,
                        RuntimeCommand::ResolvePermission {
                            request_id,
                            option_id,
                        },
                    );
                } else {
                    tracing::warn!(%provider, request_id, "no approval is waiting on that request");
                }
                ctx.request_repaint();
                return;
            }
            let mut state = lock(shared);
            if let Some(pending) = state.grok_permissions.remove(&request_id) {
                let accepted = option_id
                    .as_ref()
                    .is_some_and(|option| pending.allow_options.contains(option));
                append_grok_event(
                    &mut state,
                    &pending.provider_session_id,
                    ChatEventContent::ApprovalResolved { accepted },
                );
            }
            send_grok_command(
                &state,
                dpl_chat_grok::GrokRuntimeCommand::ResolvePermission {
                    request_id,
                    option_id,
                },
            );
            ctx.request_repaint();
        }
        HostIntent::SetPermissionMode { provider, mode } => {
            let mut state = lock(shared);
            state.permission_modes.insert(provider, mode);
            if let Some(ledger) = state.activation_ledger.as_ref() {
                if let Err(error) = ledger.set_permission_mode(provider, mode) {
                    tracing::warn!(%provider, %error, "permission mode could not be saved");
                }
            }
            tracing::info!(%provider, mode = mode.as_str(), "permission mode chosen");
            send_seam_command(
                &mut state,
                provider,
                RuntimeCommand::SetPermissionMode { mode },
            );
            ctx.request_repaint();
        }
        HostIntent::OpenRoomLibraryFolder => {
            let root = room_library_root();
            let made = dpl_roots::writable(&root)
                .map_err(std::io::Error::other)
                .and_then(|root| {
                    ["roles", "rooms"]
                        .iter()
                        .try_for_each(|kind| std::fs::create_dir_all(root.join(kind)))
                });
            if let Err(error) = made.and_then(|()| reveal_folder(&root)) {
                tracing::warn!(path = %root.display(), %error, "room library folder could not be opened");
            }
        }
        HostIntent::RequestImageGeneration { prompt } => {
            let mut state = lock(shared);
            request_image_generation(&mut state, prompt);
            ctx.request_repaint();
        }
        HostIntent::SaveGenerationDraft { id } => {
            let mut state = lock(shared);
            save_generation_draft(&mut state, &id);
            ctx.request_repaint();
        }
        HostIntent::DiscardGenerationDraft { id } => {
            let mut state = lock(shared);
            if let Some(cache) = state.imagine.cache.as_mut() {
                if let Err(error) = cache.remove(&id) {
                    tracing::warn!(%error, "generation draft could not be discarded");
                }
            }
            project_imagine(&mut state);
            ctx.request_repaint();
        }
        HostIntent::ResolveImagineFlush { decision } => {
            let mut state = lock(shared);
            resolve_imagine_flush(&mut state, decision);
            ctx.request_repaint();
        }
        HostIntent::Command { draft, synthetic } => {
            let mut state = lock(shared);
            dispatch_command(&mut state, &draft, synthetic);
            ctx.request_repaint();
        }
        HostIntent::SelectClyffy => {
            let mut state = lock(shared);
            // M3a honesty: the row exists and binds; the conversation
            // pane arrives with the send-through-binding slice. The
            // status must agree with the pinned row — a signed-out
            // backing is WAITING, never "backed by".
            let activated: Vec<ProviderId> = state
                .window
                .provider_capabilities()
                .into_iter()
                .filter(|p| p.activated)
                .map(|p| p.id)
                .collect();
            let line = match state.clyffy.as_ref().and_then(|c| c.backing) {
                Some(backing) if activated.contains(&backing) => format!(
                    "Clyffy — backed by {} · sends arrive with the next slice (/model swaps the backing).",
                    backing.0
                ),
                Some(backing) => format!(
                    "Clyffy — {} is signed out; sign it back in, or /model a signed-in provider.",
                    backing.0
                ),
                None => "Clyffy — no backing yet: sign a provider in, or /model one.".to_owned(),
            };
            state.window.set_composer_status(line);
            ctx.request_repaint();
        }
        HostIntent::StartBrainstorm => {
            let mut state = lock(shared);
            start_brainstorm(&mut state);
            ctx.request_repaint();
        }
        HostIntent::Brainstorm(op) => {
            let mut state = lock(shared);
            apply_brainstorm_op(&mut state, &op);
            ctx.request_repaint();
        }
        HostIntent::BrainstormUndo => {
            let mut state = lock(shared);
            brainstorm_history_jump(&mut state, true);
            ctx.request_repaint();
        }
        HostIntent::BrainstormRedo => {
            let mut state = lock(shared);
            brainstorm_history_jump(&mut state, false);
            ctx.request_repaint();
        }
        HostIntent::BrainstormAssist => {
            let mut state = lock(shared);
            brainstorm_assist(&mut state);
            ctx.request_repaint();
        }
        HostIntent::PromoteBrainstorm => {
            let mut state = lock(shared);
            promote_brainstorm(&mut state);
            ctx.request_repaint();
        }
        HostIntent::ToggleAssist { enable } => {
            let mut state = lock(shared);
            toggle_assist(&mut state, enable);
            ctx.request_repaint();
        }
        HostIntent::ResolveAssistAcknowledgement { accepted } => {
            let mut state = lock(shared);
            resolve_assist_acknowledgement(&mut state, accepted);
            ctx.request_repaint();
        }
        HostIntent::RefreshContent => {
            let view = read_content_view();
            lock(shared).window.observe_content(view);
        }
        HostIntent::LoadContentSpine(episode) => {
            if let Some(spine) = read_content_spine(&episode) {
                lock(shared).window.observe_content_spine(spine);
            }
        }
        HostIntent::NavigateTo(route) => {
            // Route loading lives in the MAIN app: publish the ask and let the
            // shell drain it and navigate. Chat never renders the route itself.
            lock(shared)
                .fabric
                .publish_shell(ShellEvent::NavigateRequest { route });
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
        HostIntent::ShowMain => {
            lock(shared).lifetime.restore_main();
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::Minimized(false),
            );
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
            // Native restoration completes asynchronously. Wake the root so
            // its next input frame records that lifecycle transition before
            // an agent advances to a viewport assertion.
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
        HostIntent::QuitApplication => {
            lock(shared).lifetime.request_quit();
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
        HostIntent::OperatorSlash { command, synthetic } => {
            apply_operator_slash(shared, command, synthetic);
        }
    }
}

/// The ONE slash-channel mint site (determinism-intent-gate plan §3.3).
/// Model output never reaches here; synthetic activation refuses.
fn apply_operator_slash(
    shared: &Arc<Mutex<Shared>>,
    command: OperatorSlashCommand,
    synthetic: bool,
) {
    let state = lock(shared);
    if let Some(ev) = resolve_operator_slash(&state.determinism, command, synthetic) {
        state.fabric.publish_intent(ev);
    }
}

/// Mint-site policy for privileged slash commands — extracted so the
/// synthetic-refusal path is unit-testable without constructing Chat `Shared`.
///
/// Returns the fabric fact to publish, or `None` when the command is a no-op
/// (already behavioral).
fn resolve_operator_slash(
    gate: &dpl_intent::DeterminismGate,
    command: OperatorSlashCommand,
    synthetic: bool,
) -> Option<IntentEvent> {
    let action = command.as_str().to_owned();
    if synthetic {
        return Some(IntentEvent::IntentRefused {
            action,
            reason: "synthetic-input".into(),
        });
    }
    match command {
        OperatorSlashCommand::Strict => {
            let channel = gate.escalate(dpl_intent::OperatorIntent::slash_command());
            Some(IntentEvent::DeterminismChanged {
                mode: dpl_intent::DeterminismMode::Strict,
                cause: Some(dpl_intent::StrictCause::Operator(channel)),
            })
        }
        OperatorSlashCommand::Behavioral => match gate.deescalate() {
            dpl_intent::Deescalation::Cleared => Some(IntentEvent::DeterminismChanged {
                mode: dpl_intent::DeterminismMode::Behavioral,
                cause: None,
            }),
            dpl_intent::Deescalation::RefusedEvidence => Some(IntentEvent::IntentRefused {
                action,
                reason: "evidence-locked".into(),
            }),
            dpl_intent::Deescalation::AlreadyBehavioral => None,
        },
    }
}

fn save_provider_profile(shared: &Arc<Mutex<Shared>>, draft: ProviderProfileDraft) {
    let mut state = lock(shared);
    let Some(store) = state.profile_store.as_ref() else {
        tracing::warn!("provider profile save requested without an available profile store");
        return;
    };
    let profile = match store.create(draft) {
        Ok(profile) => profile,
        Err(error) => {
            tracing::warn!(%error, "provider profile was rejected");
            return;
        }
    };
    state
        .fabric
        .publish_provider(ProviderEvent::ProfileConfigured {
            provider: profile.kind.provider_id(),
            profile_key: profile.key.as_str().to_owned(),
        });
    state.window.upsert_provider_profile(profile);
}

fn activate_provider(shared: &Arc<Mutex<Shared>>, provider: ProviderId) {
    let mut state = lock(shared);
    if !state.provider_runtime_enabled {
        tracing::debug!(%provider, "provider activation is disabled in authored Chat fixtures");
        return;
    }
    let activation = if provider == dpl_chat_codex::PROVIDER {
        let attempt = state.fabric.mint_provider_attempt();
        match state.codex_activation.start_login() {
            Ok(()) => {
                state.codex_attempt = Some(attempt);
                state
                    .fabric
                    .publish_provider(ProviderEvent::ActivationRequested { provider, attempt });
                state.fabric.publish_provider(ProviderEvent::ClientStarted {
                    provider,
                    attempt,
                    client_version: codex_client_version(&state),
                });
            }
            Err(error) => {
                tracing::warn!(%error, "ChatGPT / Codex activation was rejected");
                if !matches!(&error, dpl_chat_codex::CodexActivationError::AlreadyRunning) {
                    state.codex_attempt = Some(attempt);
                    if matches!(
                        &error,
                        dpl_chat_codex::CodexActivationError::LaunchFailed(_)
                            | dpl_chat_codex::CodexActivationError::PollFailed(_)
                            | dpl_chat_codex::CodexActivationError::StatusFailed(_)
                    ) {
                        state.codex_activation.fail(error);
                        record_codex_failed(&state, "authorization_launch");
                    }
                    publish_codex_failure(
                        &state,
                        ProviderActivationStage::Launch,
                        ProviderFailureClass::Launch,
                        true,
                    );
                }
            }
        }
        state.codex_activation.activation().clone()
    } else if provider == dpl_chat_grok::PROVIDER {
        let attempt = state.fabric.mint_provider_attempt();
        request_grok_runtime_shutdown(&mut state);
        state.grok_runtime_retry = None;
        state.grok_runtime_retry_at = None;
        state.grok_runtime_ready = false;
        let result = state.grok_activation.start_login();
        match result {
            Ok(()) => {
                state.grok_attempt = Some(attempt);
                state
                    .fabric
                    .publish_provider(ProviderEvent::ActivationRequested { provider, attempt });
                state.fabric.publish_provider(ProviderEvent::ClientStarted {
                    provider,
                    attempt,
                    client_version: grok_client_version(&state),
                });
            }
            Err(error) => {
                tracing::warn!(%error, "Grok activation was rejected");
                if !matches!(&error, dpl_chat_grok::GrokActivationError::AlreadyRunning) {
                    state.grok_attempt = Some(attempt);
                    if matches!(
                        &error,
                        dpl_chat_grok::GrokActivationError::LaunchFailed(_)
                            | dpl_chat_grok::GrokActivationError::PollFailed(_)
                    ) {
                        state.grok_activation.fail(error);
                        record_grok_failed(&state, "device_code", "authorization_launch");
                    }
                    publish_grok_failure(
                        &state,
                        ProviderActivationStage::Launch,
                        ProviderFailureClass::Launch,
                        true,
                    );
                }
            }
        }
        state.grok_activation.activation().clone()
    } else if let Some((_, spec)) = crate::presence::specs()
        .into_iter()
        .find(|(id, _)| *id == provider)
    {
        // A2 — subscription initialization for every provider that answers
        // through a presence probe rather than a bespoke coordinator. The
        // vendor's own CLI opens the operator's DEFAULT browser and runs the
        // real flow; we watch until the probe confirms it took.
        match crate::presence::begin_sign_in(provider, spec) {
            Ok(receiver) => {
                state.presence = Some(receiver);
                let attempt = state.fabric.mint_provider_attempt();
                state
                    .fabric
                    .publish_provider(ProviderEvent::ActivationRequested { provider, attempt });
                state
                    .fabric
                    .publish_provider(ProviderEvent::SystemBrowserDispatched { provider, attempt });
                ActivationState::SigningIn
            }
            Err(error) => {
                tracing::warn!(%error, %provider, "provider sign-in could not start");
                ActivationState::SignedOut {
                    reason: error,
                    recovery: vec![RecoveryAction::new(
                        RecoveryActionKind::InstallInstructions,
                        "Install the provider's CLI",
                    )],
                }
            }
        }
    } else {
        tracing::warn!(%provider, "provider activation action is not qualified");
        return;
    };
    if let Err(error) = state.window.set_provider_activation(provider, activation) {
        tracing::warn!(%error, "provider activation state could not be projected");
    }
}

fn copy_provider_authorization_code(
    ctx: &egui::Context,
    shared: &Arc<Mutex<Shared>>,
    provider: ProviderId,
) {
    let state = lock(shared);
    if provider != dpl_chat_grok::PROVIDER {
        tracing::warn!(%provider, "provider authorization code copy is not qualified");
        return;
    }
    let Some(provider_view) = state.window.provider_view(provider) else {
        tracing::warn!("Grok authorization code copy requested without provider state");
        return;
    };
    let ActivationState::AwaitingAuthorization(prompt) = provider_view.registration.activation
    else {
        tracing::warn!("Grok authorization code copy requested without a device prompt");
        return;
    };
    if !prompt.user_code.copyable {
        tracing::warn!("Grok authorization code is not marked copyable");
        return;
    }
    ctx.copy_text(prompt.user_code.value);
    if let Some(attempt) = state.grok_attempt {
        state
            .fabric
            .publish_provider(ProviderEvent::CodeCopied { provider, attempt });
    }
}

fn open_grok_authorization_browser(state: &mut Shared, explicit_reopen: bool) -> bool {
    if !state.provider_runtime_enabled {
        tracing::debug!("authorization browser is disabled in authored Chat fixtures");
        return false;
    }
    let Some(prompt) = state.grok_activation.current_prompt().cloned() else {
        tracing::warn!("Grok authorization browser requested without a device prompt");
        return false;
    };
    let dispatch = if explicit_reopen {
        state.auth_browser.reopen(prompt)
    } else {
        state.auth_browser.open_once(prompt)
    };
    match dispatch {
        Ok(started) => started,
        Err(BrowserAuthError::DispatchInProgress(_)) => false,
        Err(error) => {
            tracing::warn!(%error, "Grok authorization browser was rejected");
            state
                .grok_activation
                .fail(dpl_chat_grok::GrokActivationError::LaunchFailed(
                    error.to_string(),
                ));
            let activation = state.grok_activation.activation().clone();
            if let Err(project_error) = state
                .window
                .set_provider_activation(dpl_chat_grok::PROVIDER, activation)
            {
                tracing::warn!(%project_error, "Grok browser failure could not be projected");
            }
            record_grok_failed(state, "device_code", "authorization_browser");
            publish_grok_failure(
                state,
                ProviderActivationStage::Browser,
                ProviderFailureClass::Browser,
                true,
            );
            true
        }
    }
}

fn grok_client_version(state: &Shared) -> String {
    state.grok_activation.runtime_profile().map_or_else(
        || "unknown".to_owned(),
        |profile| profile.version().to_string(),
    )
}

fn record_grok_failed(state: &Shared, auth_method: &str, failure_class: &str) {
    if let Some(ledger) = state.activation_ledger.as_ref() {
        if let Err(error) = ledger.record_failed_for_attempt(
            dpl_chat_grok::PROVIDER,
            state.grok_attempt.map(|attempt| attempt.0),
            &grok_client_version(state),
            auth_method,
            failure_class,
        ) {
            tracing::warn!(%error, "Grok failure evidence could not be stored");
        }
    }
}

fn publish_grok_failure(
    state: &Shared,
    stage: ProviderActivationStage,
    class: ProviderFailureClass,
    retryable: bool,
) {
    let Some(attempt) = state.grok_attempt else {
        return;
    };
    state.fabric.publish_provider(ProviderEvent::Failed {
        provider: dpl_chat_grok::PROVIDER,
        attempt,
        stage,
        class,
        retryable,
    });
}

fn codex_client_version(state: &Shared) -> String {
    state.codex_activation.runtime_profile().map_or_else(
        || "unknown".to_owned(),
        |profile| profile.version().to_string(),
    )
}

fn record_codex_failed(state: &Shared, failure_class: &str) {
    if let Some(ledger) = state.activation_ledger.as_ref() {
        if let Err(error) = ledger.record_failed_for_attempt(
            dpl_chat_codex::PROVIDER,
            state.codex_attempt.map(|attempt| attempt.0),
            &codex_client_version(state),
            "chatgpt",
            failure_class,
        ) {
            tracing::warn!(%error, "Codex failure evidence could not be stored");
        }
    }
}

fn publish_codex_failure(
    state: &Shared,
    stage: ProviderActivationStage,
    class: ProviderFailureClass,
    retryable: bool,
) {
    let Some(attempt) = state.codex_attempt else {
        return;
    };
    state.fabric.publish_provider(ProviderEvent::Failed {
        provider: dpl_chat_codex::PROVIDER,
        attempt,
        stage,
        class,
        retryable,
    });
}

fn ensure_grok_attempt(state: &mut Shared) -> ActivationAttemptId {
    if let Some(attempt) = state.grok_attempt {
        return attempt;
    }
    let attempt = state.fabric.mint_provider_attempt();
    state.grok_attempt = Some(attempt);
    attempt
}

fn drain_provider_facts(state: &mut Shared) {
    while let Ok(fact) = state.provider_events.try_recv() {
        state.window.observe_provider_fact(fact);
    }
}

fn generation_status_for(imagine: &ImagineState) -> GenerationStatus {
    let provisioned = dpl_chat_imagine::resolve_api_key().is_some();
    GenerationStatus {
        image_available: provisioned,
        unavailable_reason: if provisioned {
            String::new()
        } else {
            "Set XAI_API_KEY in your environment to enable image generation (ADR-011).".to_owned()
        },
        in_flight: imagine.in_flight(),
        last_error: imagine.last_error.clone(),
    }
}

fn imagine_draft_views(imagine: &ImagineState) -> Vec<GenerationDraftView> {
    let Some(cache) = imagine.cache.as_ref() else {
        return Vec::new();
    };
    cache
        .drafts()
        .iter()
        .map(|draft| GenerationDraftView {
            id: draft.id.clone(),
            prompt: draft.prompt.clone(),
            created_ms: draft.created_ms,
            thumb: std::fs::read(cache.artifact_path(draft))
                .ok()
                .and_then(|bytes| decode_generation_thumb(&bytes)),
        })
        .collect()
}

fn decode_generation_thumb(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let thumb = decoded.thumbnail(256, 256).to_rgba8();
    Some((thumb.width(), thumb.height(), thumb.into_raw()))
}

fn project_imagine(state: &mut Shared) {
    let status = generation_status_for(&state.imagine);
    state.window.set_generation_status(status);
    let views = imagine_draft_views(&state.imagine);
    state.window.replace_generation_drafts(views);
}

fn request_image_generation(state: &mut Shared, prompt: String) {
    if !state.provider_runtime_enabled {
        tracing::debug!("image generation is disabled in authored Chat fixtures");
        return;
    }
    if state.imagine.in_flight() {
        tracing::warn!("an image generation is already in flight");
        return;
    }
    if prompt.trim().is_empty() {
        return;
    }
    let Some(key) = dpl_chat_imagine::resolve_api_key() else {
        state.imagine.last_error =
            Some("XAI_API_KEY is not provisioned in this session.".to_owned());
        project_imagine(state);
        return;
    };
    let job = state.fabric.mint_job();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker_prompt = prompt.clone();
    let spawned = std::thread::Builder::new()
        .name("devpulse-imagine".to_owned())
        .spawn(move || {
            let result = SystemImagineClient.generate_image(&key, &worker_prompt);
            let _ = sender.send(result);
        });
    if let Err(error) = spawned {
        state.imagine.last_error = Some(format!("the generation worker could not start: {error}"));
        project_imagine(state);
        return;
    }
    state.imagine.worker = Some(receiver);
    state.imagine.pending_prompt = Some(prompt);
    state.imagine.job = Some(job);
    state.imagine.last_error = None;
    state.fabric.publish_job(JobEvent::Queued {
        job,
        label: "imagine image".to_owned(),
    });
    project_imagine(state);
}

fn poll_imagine(state: &mut Shared) -> bool {
    let Some(receiver) = state.imagine.worker.as_ref() else {
        return false;
    };
    let result = match receiver.try_recv() {
        Ok(result) => result,
        Err(std::sync::mpsc::TryRecvError::Empty) => return false,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            Err("the generation worker stopped".to_owned())
        }
    };
    state.imagine.worker = None;
    let prompt = state.imagine.pending_prompt.take().unwrap_or_default();
    let job = state.imagine.job.take();
    match result {
        Ok(bytes) => {
            let outcome = state.imagine.cache.as_mut().map(|cache| {
                cache.add(
                    &prompt,
                    dpl_chat_imagine::IMAGE_MODEL,
                    &bytes,
                    now_unix_ms(),
                )
            });
            match outcome {
                Some(Ok(draft)) => {
                    if let Some(job) = job {
                        state.fabric.publish_job(JobEvent::Done {
                            job,
                            out: draft.file.clone(),
                        });
                    }
                }
                Some(Err(error)) => {
                    state.imagine.last_error = Some(error.clone());
                    if let Some(job) = job {
                        state.fabric.publish_job(JobEvent::Failed { job, error });
                    }
                }
                None => {}
            }
        }
        Err(error) => {
            state.imagine.last_error = Some(error.clone());
            if let Some(job) = job {
                state.fabric.publish_job(JobEvent::Failed { job, error });
            }
        }
    }
    project_imagine(state);
    true
}

fn save_generation_draft(state: &mut Shared, id: &str) {
    if !state.provider_runtime_enabled {
        tracing::debug!("draft promotion is disabled in authored Chat fixtures");
        return;
    }
    let Some(path) = state
        .imagine
        .cache
        .as_ref()
        .and_then(|cache| cache.get(id).map(|draft| cache.artifact_path(draft)))
    else {
        tracing::warn!(%id, "unknown generation draft");
        return;
    };
    match dpl_composer::import::import_generated(&path) {
        Ok(_record) => {
            tracing::info!(%id, "generated draft promoted into the library inbox");
            if let Some(cache) = state.imagine.cache.as_mut() {
                if let Err(error) = cache.remove(id) {
                    tracing::warn!(%error, "promoted draft could not leave the cache");
                }
            }
            state.fabric.publish_capture(CaptureEvent::InboxChanged);
        }
        Err(error) => {
            state.imagine.last_error = Some(format!("Save failed: {error}"));
        }
    }
    project_imagine(state);
}

fn resolve_imagine_flush(state: &mut Shared, decision: ImagineFlushDecision) {
    match decision {
        ImagineFlushDecision::Defer => {
            tracing::info!("Imagine draft flush deferred by the operator");
        }
        ImagineFlushDecision::FlushNow | ImagineFlushDecision::FlushAlways => {
            if let Some(cache) = state.imagine.cache.as_mut() {
                let flushed = cache.flush_expired(now_unix_ms());
                tracing::info!(flushed, "expired Imagine drafts were flushed");
            }
            if decision == ImagineFlushDecision::FlushAlways {
                state.imagine.policy.auto_flush = true;
                if let Err(error) = state.imagine.policy.save(&dpl_roots::userdata_root()) {
                    tracing::warn!(%error, "Imagine flush policy could not be persisted");
                }
            }
            project_imagine(state);
        }
    }
}

/// The assist toggle's host arm: disabling is instant; enabling stands
/// behind the ONE blanket acknowledgement (asked once, ever — it covers
/// every provider). The window's chip only moves via `set_assist`.
fn toggle_assist(state: &mut Shared, enable: bool) {
    if !enable {
        state.assist.enabled = false;
        save_assist(state);
        state.window.set_assist(false);
        return;
    }
    if state.assist.acknowledged {
        state.assist.enabled = true;
        save_assist(state);
        state.window.set_assist(true);
        return;
    }
    let interaction = InteractionId(state.next_interaction_id);
    state.next_interaction_id = state.next_interaction_id.saturating_add(1);
    if let Err(error) = state.window.register_assist_acknowledgement(interaction) {
        tracing::warn!(%error, "assist acknowledgement could not be raised");
    }
}

/// The acknowledgement resolved: accepting is the durable consent
/// (enabled + acknowledged persist together); declining changes nothing
/// on disk and assist stays off.
fn resolve_assist_acknowledgement(state: &mut Shared, accepted: bool) {
    if accepted {
        state.assist.enabled = true;
        state.assist.acknowledged = true;
        save_assist(state);
    } else {
        tracing::info!("assist mode declined by the operator — staying off");
    }
    state.window.set_assist(state.assist.enabled);
}

/// Persist the assist policy — never in authored fixtures (they run on
/// in-memory defaults and must not touch Ring-0).
fn save_assist(state: &Shared) {
    if !state.provider_runtime_enabled {
        return;
    }
    if let Err(error) = state.assist.save(&dpl_roots::userdata_root()) {
        tracing::warn!(%error, "assist policy could not be persisted");
    }
}

// ── the Clyffy house session (BND1-M3a: host-owned, like the board) ─────

/// DevPULSE's own conversation. The id is minted ONCE and never changes;
/// `backing` is a current binding /model swaps; events keep the
/// provider tag of whoever produced them (history is never rewritten).
struct ClyffySession {
    id: ChatSessionId,
    backing: Option<ProviderId>,
    events: Vec<dpl_chat_core::ChatEvent>,
}

impl ClyffySession {
    /// Rebind the backing. The identity law: the session id and every
    /// recorded event are UNTOUCHED — a swap is a re-binding, never a
    /// new transcript and never a rewritten history.
    fn rebind(&mut self, provider: ProviderId) -> ChatSessionId {
        self.backing = Some(provider);
        self.id
    }

    /// Active = the current binding is among the activated providers.
    /// Clyffy never claims readiness it borrowed from nobody.
    fn is_active(&self, activated: &[ProviderId]) -> bool {
        self.backing.is_some_and(|b| activated.contains(&b))
    }
}

/// Derive Clyffy's truth from presence: the session EXISTS from first
/// sync; it is ACTIVE only while at least one provider is activated
/// (signed-in). The binding defaults to the first activated provider
/// and survives sign-outs (an inactive binding is shown, not erased).
fn sync_clyffy(state: &mut Shared) {
    if state.provider_runtime_enabled && !project::ready(state) {
        state.window.observe_clyffy(dpl_chat_window::ClyffyView {
            present: true,
            active: false,
            backing: None,
            events: 0,
        });
        return;
    }
    let activated: Vec<ProviderId> = state
        .window
        .provider_capabilities()
        .into_iter()
        .filter(|p| p.activated)
        .map(|p| p.id)
        .collect();
    if state.clyffy.is_none() {
        let id = ChatSessionId(state.next_chat_session_id);
        state.next_chat_session_id = state.next_chat_session_id.saturating_add(1);
        state.clyffy = Some(ClyffySession {
            id,
            backing: None,
            events: Vec::new(),
        });
    }
    let session = state.clyffy.as_mut().expect("minted above");
    if session.backing.is_none() {
        session.backing = activated.first().copied();
    }
    let active = session.is_active(&activated);
    let view = dpl_chat_window::ClyffyView {
        present: true,
        active,
        backing: session.backing,
        events: session.events.len(),
    };
    state.window.observe_clyffy(view);
}

/// /model — rebind Clyffy's backing. The argument already validated
/// against the roster census; the session id and history are untouched
/// (the swap is a re-binding, never a new transcript).
fn set_clyffy_backing(state: &mut Shared, provider: ProviderId) {
    let Some(session) = &mut state.clyffy else {
        state
            .window
            .set_composer_status("Clyffy has not arrived yet — sign a provider in first.");
        return;
    };
    session.rebind(provider);
    state
        .window
        .set_composer_status(format!("Clyffy is now backed by {}.", provider.0));
    sync_clyffy(state);
}

// ── command dispatch (BND1: the host validates against live censuses) ───

/// Validate a recognized command against the censuses the host owns and
/// dispatch it. Refusals paint under the composer naming what IS
/// registered; synthetic input is refused for the WHOLE census.
fn dispatch_command(state: &mut Shared, draft: &dpl_chat_core::CommandDraft, synthetic: bool) {
    if synthetic {
        tracing::warn!(slug = %draft.slug, "command refused: synthetic input");
        state
            .window
            .set_composer_status("Commands come from the operator, not from injected input.");
        return;
    }
    // /model's census is the SIGNED-IN providers — its blurb promises
    // exactly that, and a success line must never detach a working
    // binding onto a dead backing.
    let activated: Vec<ProviderId> = state
        .window
        .provider_capabilities()
        .into_iter()
        .filter(|p| p.activated)
        .map(|p| p.id)
        .collect();
    let censuses = dpl_chat_core::CommandCensuses {
        routes: state
            .route_table
            .iter()
            .map(|(path, _)| path.clone())
            .collect(),
        providers: activated.iter().map(|p| p.0.to_owned()).collect(),
        workflows: dpl_workflows::workflows()
            .iter()
            .map(|w| w.id.clone())
            .collect(),
    };
    if let Err(issues) = dpl_chat_core::validate_command(draft, &censuses) {
        let named = issues
            .iter()
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join(" · ");
        state.window.set_composer_status(named);
        return;
    }
    // Validation passed — NOW the trigger text is consumed. A refused
    // command above never eats the operator's typed buffer.
    clear_ai_draft(state);
    match draft.slug.as_str() {
        "route" => {
            let target = draft.rest.trim();
            if let Some((_, route)) = state.route_table.iter().find(|(path, _)| path == target) {
                state
                    .fabric
                    .publish_shell(ShellEvent::NavigateRequest { route: *route });
                state
                    .window
                    .set_composer_status(format!("Opening {target} in the main window."));
            }
        }
        "routines" => {
            let listed = dpl_workflows::workflows()
                .iter()
                .map(|w| format!("{} — {}", w.id, w.blurb))
                .collect::<Vec<_>>()
                .join("\n");
            state
                .window
                .set_composer_status(format!("Registered workflows:\n{listed}"));
            // The blurb PROMISES the Workflows tab opens — keep it.
            if let Some((_, route)) = state
                .route_table
                .iter()
                .find(|(path, _)| path.starts_with("wiredfront/"))
            {
                state
                    .fabric
                    .publish_shell(ShellEvent::NavigateRequest { route: *route });
            }
        }
        "model" => {
            let target = draft.rest.trim();
            if let Some(provider) = activated.iter().find(|p| p.0 == target).copied() {
                set_clyffy_backing(state, provider);
            }
        }
        "brainstorm" => {
            // The trigger rides the SAME refusal + validation ladder
            // as every command — no special-cased side door.
            start_brainstorm(state);
        }
        other => {
            tracing::warn!(slug = %other, "recognized command with no dispatch arm");
        }
    }
}

/// Consume the composer's Ai draft after a command dispatched — the
/// host-side half of the provenance law's "nothing is sent".
fn clear_ai_draft(state: &mut Shared) {
    if let Some(provider) = state.window.selected_provider_id() {
        if let Err(error) = state
            .window
            .apply(dpl_chat_window::ChatWindowIntent::UpdateAiDraft {
                provider,
                value: String::new(),
            })
        {
            tracing::warn!(%error, "the dispatched command's draft could not be cleared");
        }
    }
}

// ── the brainstorm session (BS1: the board lives host-side) ─────────────

/// Push the session's current truth into the window — after EVERY
/// mutation, refusal included, so the surface never renders stale state.
fn project_brainstorm(state: &mut Shared) {
    let view = match &state.brainstorm {
        Some(session) => dpl_chat_window::BrainstormView {
            doc: Some(session.doc.clone()),
            status: session.status.clone(),
            can_undo: !session.undo.is_empty(),
            can_redo: !session.redo.is_empty(),
        },
        None => dpl_chat_window::BrainstormView::default(),
    };
    state.window.observe_brainstorm(view);
}

/// Mint a fresh board in the content inbox (no-clobber) and project it.
/// One live board per chat session — retriggering while one is open
/// just surfaces it (the operator's board is never silently replaced).
fn start_brainstorm(state: &mut Shared) {
    if let Some(session) = &mut state.brainstorm {
        session.status = Some("The board is already open — right here.".to_owned());
        project_brainstorm(state);
        return;
    }
    let scaffold = dpl_workflows::brainstorm_scaffolds()
        .into_iter()
        .next()
        .expect("the scaffold census is never empty");
    let inbox = dpl_roots::content_root().join("inbox");
    let dir = dpl_workflows::fresh_dir(&inbox, "brainstorm");
    let stem = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "brainstorm".to_owned());
    let doc = dpl_workflows::mint_brainstorm(&scaffold, &stem);
    let status = match dpl_workflows::save_brainstorm(&dir, &doc) {
        Ok(()) => Some("Board opened — type the massive idea and add it.".to_owned()),
        Err(error) => Some(format!(
            "Board opened in memory only — save failed: {error}"
        )),
    };
    state.brainstorm = Some(BrainstormSession {
        dir,
        doc,
        undo: Vec::new(),
        redo: Vec::new(),
        status,
        assist: None,
    });
    project_brainstorm(state);
}

/// Spawn the one-shot suggestion call (the imagine drive shape): guard
/// honestly, mint the job BEFORE the worker, one terminal transition.
fn brainstorm_assist(state: &mut Shared) {
    if !state.assist.enabled {
        if let Some(session) = &mut state.brainstorm {
            session.status =
                Some("Assist is off — the ⚡ chip by the composer turns it on.".to_owned());
        }
        project_brainstorm(state);
        return;
    }
    let Some(session) = &mut state.brainstorm else {
        return;
    };
    if session.assist.is_some() {
        session.status = Some("The assist is already thinking…".to_owned());
        project_brainstorm(state);
        return;
    }
    if session.doc.seed.trim().is_empty() {
        session.status = Some("Seed the board first — suggestions grow from the idea.".to_owned());
        project_brainstorm(state);
        return;
    }
    let Some(key) = dpl_chat_imagine::resolve_api_key() else {
        session.status =
            Some("No XAI_API_KEY in this session — suggestions stay off, honestly.".to_owned());
        project_brainstorm(state);
        return;
    };
    let seed = session.doc.seed.clone();
    let existing: Vec<String> = session.doc.items.iter().map(|i| i.text.clone()).collect();
    let job = state.fabric.mint_job();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("devpulse-brainstorm-assist".into())
        .spawn(move || {
            use dpl_chat_imagine::SuggestClient as _;
            let result =
                dpl_chat_imagine::SystemImagineClient.suggest_ideas(&key, &seed, &existing);
            let _ = tx.send(result);
        });
    if spawned.is_err() {
        session.status = Some("The suggestion worker could not start.".to_owned());
        project_brainstorm(state);
        return;
    }
    session.assist = Some(BrainstormAssistCall { job, rx });
    session.status = Some("Assist is thinking…".to_owned());
    state.fabric.publish_job(dpl_fabric::JobEvent::Queued {
        job,
        label: "brainstorm · populate".to_owned(),
    });
    project_brainstorm(state);
}

/// Drain the suggestion call: proposals land as Assist-origin items
/// through the SAME `apply` path, one checkpoint for the whole batch
/// (one undo step — a proposal wave is one gesture).
fn poll_brainstorm_assist(state: &mut Shared) -> bool {
    let Some(session) = &mut state.brainstorm else {
        return false;
    };
    let Some(call) = &session.assist else {
        return false;
    };
    let result = match call.rx.try_recv() {
        Ok(result) => result,
        Err(std::sync::mpsc::TryRecvError::Empty) => return false,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            Err("the suggestion worker vanished".to_owned())
        }
    };
    let job = call.job;
    session.assist = None;
    match result {
        Ok(ideas) => {
            session.checkpoint();
            let mut landed = 0usize;
            for text in &ideas {
                let id = (1..)
                    .map(|n| format!("idea-{n}"))
                    .find(|c| session.doc.items.iter().all(|i| &i.id != c))
                    .expect("unbounded");
                if dpl_workflows::apply(
                    &mut session.doc,
                    &dpl_workflows::BrainstormOp::AddItem {
                        id,
                        text: text.clone(),
                        parent: None,
                        origin: dpl_workflows::BrainstormOrigin::Assist,
                    },
                )
                .is_ok()
                {
                    landed += 1;
                }
            }
            session.status = Some(format!(
                "Assist proposed {landed} ideas — yours to keep or cut."
            ));
            if let Err(error) = dpl_workflows::save_brainstorm(&session.dir, &session.doc) {
                session.status = Some(format!("Proposals kept in memory — save failed: {error}"));
            }
            state.fabric.publish_job(dpl_fabric::JobEvent::Done {
                job,
                out: format!("{landed} ideas"),
            });
        }
        Err(error) => {
            session.status = Some(error.clone());
            state
                .fabric
                .publish_job(dpl_fabric::JobEvent::Failed { job, error });
        }
    }
    project_brainstorm(state);
    true
}

/// The ONE write path: checkpoint → apply → save → project. A refused
/// op rolls the checkpoint back and surfaces its issues honestly.
fn apply_brainstorm_op(state: &mut Shared, op: &dpl_workflows::BrainstormOp) {
    let Some(session) = &mut state.brainstorm else {
        return;
    };
    session.checkpoint();
    match dpl_workflows::apply(&mut session.doc, op) {
        Ok(()) => {
            session.status = None;
            if let Err(error) = dpl_workflows::save_brainstorm(&session.dir, &session.doc) {
                session.status = Some(format!("Edit kept in memory — save failed: {error}"));
            }
        }
        Err(issues) => {
            // The checkpoint pushed a snapshot equal to the (unmutated)
            // doc; pop it so a refusal never pollutes the undo reach.
            session.undo.pop();
            let named = issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join(" · ");
            session.status = Some(named);
        }
    }
    project_brainstorm(state);
}

/// Undo/redo — the composer pop-swap shape, persisted like any edit.
fn brainstorm_history_jump(state: &mut Shared, back: bool) {
    let Some(session) = &mut state.brainstorm else {
        return;
    };
    let (from, to) = if back {
        (&mut session.undo, &mut session.redo)
    } else {
        (&mut session.redo, &mut session.undo)
    };
    if let Some(prev) = from.pop() {
        to.push(std::mem::replace(&mut session.doc, prev));
        session.status = None;
        if let Err(error) = dpl_workflows::save_brainstorm(&session.dir, &session.doc) {
            session.status = Some(format!("History jump unsaved — {error}"));
        }
    } else {
        session.status = Some(if back {
            "Nothing to undo.".to_owned()
        } else {
            "Nothing to redo.".to_owned()
        });
    }
    project_brainstorm(state);
}

/// Distill the converged board into the explainer intake pre-fill,
/// landed beside the board (`intake-prefill.json`) — the honest BS-M2
/// exit; the guided intake itself arrives with EX1-M3.
fn promote_brainstorm(state: &mut Shared) {
    let Some(session) = &mut state.brainstorm else {
        return;
    };
    let announce = match dpl_workflows::distill(&session.doc) {
        Ok(prefill) => match dpl_workflows::save_intake_prefill(&session.dir, &prefill) {
            Ok(()) => {
                session.status = Some(format!(
                    "Promoted — {} intake answers staged in {}.",
                    prefill.len(),
                    session.dir.display()
                ));
                Some((prefill.len(), session.dir.display().to_string()))
            }
            Err(error) => {
                session.status = Some(format!("Promote failed to save: {error}"));
                None
            }
        },
        Err(issues) => {
            let named = issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join(" · ");
            session.status = Some(named);
            None
        }
    };
    // The promote is a durable transcript fact, not only a status line
    // the next op overwrites. (Session borrow ends before the window.)
    if let Some((answers, dir)) = announce {
        let interaction = InteractionId(state.next_interaction_id);
        state.next_interaction_id = state.next_interaction_id.saturating_add(1);
        if let Err(error) = state
            .window
            .register_brainstorm_promoted(interaction, answers, &dir)
        {
            tracing::warn!(%error, "the promote announce could not register");
        }
    }
    project_brainstorm(state);
}

fn poll_provider_activations(state: &mut Shared) -> bool {
    if !state.provider_runtime_enabled {
        return false;
    }
    let codex_changed = match state.codex_activation.poll() {
        Ok(changed) => changed,
        Err(error) => {
            tracing::warn!(%error, "ChatGPT / Codex activation polling failed");
            let failure_class = if matches!(
                &error,
                dpl_chat_codex::CodexActivationError::StatusFailed(_)
            ) {
                "status_verification"
            } else {
                "authorization_process"
            };
            state.codex_activation.fail(error);
            record_codex_failed(state, failure_class);
            publish_codex_failure(
                state,
                ProviderActivationStage::Authorization,
                ProviderFailureClass::Authorization,
                true,
            );
            true
        }
    };
    if codex_changed {
        let activation = state.codex_activation.activation().clone();
        if let Err(error) = state
            .window
            .set_provider_activation(dpl_chat_codex::PROVIDER, activation)
        {
            tracing::warn!(%error, "ChatGPT / Codex activation state could not be projected");
        }
    }
    if state.codex_activation.take_login_completed() {
        if let Some(attempt) = state.codex_attempt {
            state
                .fabric
                .publish_provider(ProviderEvent::AuthorizationCompleted {
                    provider: dpl_chat_codex::PROVIDER,
                    attempt,
                });
        }
    }
    if let Some(status) = state.codex_activation.take_verified_status() {
        let version = codex_client_version(state);
        match status {
            dpl_chat_codex::CodexLoginStatus::ChatGpt => {
                if let Some(ledger) = state.activation_ledger.as_ref() {
                    if let Err(error) = ledger.record_ready_for_attempt(
                        dpl_chat_codex::PROVIDER,
                        state.codex_attempt.map(|attempt| attempt.0),
                        &version,
                        "chatgpt",
                    ) {
                        tracing::warn!(%error, "Codex ready evidence could not be stored");
                    }
                }
                if let Some(attempt) = state.codex_attempt {
                    state.fabric.publish_provider(ProviderEvent::Ready {
                        provider: dpl_chat_codex::PROVIDER,
                        attempt,
                        client_version: version,
                        auth_method: "chatgpt",
                    });
                }
            }
            dpl_chat_codex::CodexLoginStatus::ApiKey
            | dpl_chat_codex::CodexLoginStatus::SignedOut => {
                if let Some(ledger) = state.activation_ledger.as_ref() {
                    if let Err(error) = ledger.record_signed_out_for_attempt(
                        dpl_chat_codex::PROVIDER,
                        state.codex_attempt.map(|attempt| attempt.0),
                        &version,
                        "chatgpt",
                    ) {
                        tracing::warn!(%error, "Codex signed-out evidence could not be stored");
                    }
                }
                publish_codex_failure(
                    state,
                    ProviderActivationStage::Authorization,
                    ProviderFailureClass::Authorization,
                    false,
                );
            }
        }
    }

    let had_grok_prompt = state.grok_activation.current_prompt().is_some();
    let grok_changed = match state.grok_activation.poll() {
        Ok(changed) => changed,
        Err(error) => {
            tracing::warn!(%error, "Grok activation polling failed");
            state.grok_activation.fail(error);
            record_grok_failed(state, "device_code", "authorization_process");
            publish_grok_failure(
                state,
                ProviderActivationStage::Authorization,
                ProviderFailureClass::Authorization,
                true,
            );
            true
        }
    };
    if grok_changed {
        let activation = state.grok_activation.activation().clone();
        if let Err(error) = state
            .window
            .set_provider_activation(dpl_chat_grok::PROVIDER, activation)
        {
            tracing::warn!(%error, "Grok activation state could not be projected");
        }
    }
    if !had_grok_prompt {
        if let (Some(prompt), Some(attempt)) =
            (state.grok_activation.current_prompt(), state.grok_attempt)
        {
            let code_len = u16::try_from(prompt.user_code.value.len()).unwrap_or(u16::MAX);
            state
                .fabric
                .publish_provider(ProviderEvent::DeviceCodeReceived {
                    provider: dpl_chat_grok::PROVIDER,
                    attempt,
                    code_len,
                });
        }
    }

    let mut browser_changed = false;
    if state.grok_activation.current_prompt().is_some() {
        browser_changed |= open_grok_authorization_browser(state, false);
    }
    if state.grok_activation.take_login_completed() {
        if let Some(attempt) = state.grok_attempt {
            state
                .fabric
                .publish_provider(ProviderEvent::AuthorizationCompleted {
                    provider: dpl_chat_grok::PROVIDER,
                    attempt,
                });
        }
        state.auth_browser.reset(dpl_chat_grok::PROVIDER);
        begin_grok_runtime_verification(state, dpl_chat_grok::GrokVerificationOrigin::PostLogin);
        browser_changed = true;
    }
    while let Some(event) = state.auth_browser.try_event() {
        browser_changed = true;
        match event {
            BrowserAuthEvent::Dispatched { provider } => {
                tracing::info!(%provider, "provider authorization sent to system browser");
                if let Some(attempt) = state.grok_attempt {
                    state
                        .fabric
                        .publish_provider(ProviderEvent::SystemBrowserDispatched {
                            provider,
                            attempt,
                        });
                }
            }
            BrowserAuthEvent::Failed { provider, reason } => {
                tracing::warn!(%provider, %reason, "provider authorization browser failed");
                state
                    .grok_activation
                    .fail(dpl_chat_grok::GrokActivationError::LaunchFailed(reason));
                let activation = state.grok_activation.activation().clone();
                if let Err(error) = state.window.set_provider_activation(provider, activation) {
                    tracing::warn!(%error, "browser failure state could not be projected");
                }
                record_grok_failed(state, "device_code", "authorization_browser");
                publish_grok_failure(
                    state,
                    ProviderActivationStage::Browser,
                    ProviderFailureClass::Browser,
                    true,
                );
            }
        }
    }

    codex_changed || grok_changed || browser_changed
}

fn request_grok_runtime_shutdown(state: &mut Shared) {
    let Some(runtime) = state.grok_runtime.as_ref() else {
        return;
    };
    if let Err(error) = runtime.try_send(dpl_chat_grok::GrokRuntimeCommand::Shutdown) {
        tracing::warn!(%error, "Grok ACP shutdown request could not be queued");
    }
}

fn begin_grok_runtime_verification(
    state: &mut Shared,
    origin: dpl_chat_grok::GrokVerificationOrigin,
) {
    state.grok_runtime_retry = Some(dpl_chat_grok::GrokRuntimeRetryState::new(origin));
    state.grok_runtime_retry_at = Some(Instant::now());
    state.grok_runtime_ready = false;
    if let Err(error) = state
        .window
        .set_provider_activation(dpl_chat_grok::PROVIDER, ActivationState::Verifying)
    {
        tracing::warn!(%error, "Grok verification state could not be projected");
    }
}

fn drive_grok_runtime(state: &mut Shared, now: Instant) -> bool {
    if !state.provider_runtime_enabled || !project::ready(state) {
        return false;
    }
    if state.grok_runtime.is_some() {
        return false;
    }
    let Some(retry_at) = state.grok_runtime_retry_at else {
        return false;
    };
    if now < retry_at {
        return false;
    }
    state.grok_runtime_retry_at = None;
    let Some(profile) = state.grok_activation.runtime_profile() else {
        tracing::warn!("Grok runtime profile is unavailable after activation");
        return false;
    };
    let attempt = ensure_grok_attempt(state);
    match dpl_chat_grok::GrokAcpRuntime::start(profile) {
        Ok(runtime) => {
            state.grok_runtime = Some(runtime);
            state
                .fabric
                .publish_provider(ProviderEvent::RuntimeStarted {
                    provider: dpl_chat_grok::PROVIDER,
                    attempt,
                });
            true
        }
        Err(error) => {
            handle_grok_runtime_failure(
                state,
                dpl_chat_grok::GrokRuntimeFailure {
                    kind: dpl_chat_grok::GrokRuntimeFailureKind::Launch,
                    reason: format!("Grok ACP could not start: {error}"),
                    provider_session_id: None,
                },
            );
            true
        }
    }
}

fn handle_grok_runtime_failure(state: &mut Shared, failure: dpl_chat_grok::GrokRuntimeFailure) {
    tracing::warn!(
        kind = ?failure.kind,
        reason = %failure.reason,
        "Grok ACP failure"
    );
    let was_ready = state.grok_runtime_ready;
    state.grok_runtime_ready = false;
    if was_ready || state.grok_runtime_retry.is_none() {
        state.grok_runtime_retry = Some(dpl_chat_grok::GrokRuntimeRetryState::new(if was_ready {
            dpl_chat_grok::GrokVerificationOrigin::Recovery
        } else {
            dpl_chat_grok::GrokVerificationOrigin::Startup
        }));
    }
    let disposition = state
        .grok_runtime_retry
        .as_mut()
        .expect("the Grok retry state was established")
        .on_failure(failure.kind);
    match disposition {
        dpl_chat_grok::GrokRuntimeFailureDisposition::Retry {
            retry_index,
            max_retries,
            delay,
        } => {
            state.grok_runtime_retry_at = Some(Instant::now() + delay);
            if let Err(error) = state
                .window
                .set_provider_activation(dpl_chat_grok::PROVIDER, ActivationState::Verifying)
            {
                tracing::warn!(%error, "Grok retry state could not be projected");
            }
            let attempt = ensure_grok_attempt(state);
            state
                .fabric
                .publish_provider(ProviderEvent::VerificationRetryScheduled {
                    provider: dpl_chat_grok::PROVIDER,
                    attempt,
                    retry_index,
                    max_retries,
                    delay_ms: u16::try_from(delay.as_millis()).unwrap_or(u16::MAX),
                });
        }
        dpl_chat_grok::GrokRuntimeFailureDisposition::SignedOut => {
            state.grok_runtime_retry = None;
            state.grok_runtime_retry_at = None;
            state
                .grok_activation
                .mark_signed_out(failure.reason.clone());
            let activation = ActivationState::SignedOut {
                reason: failure.reason,
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::SignIn,
                    "Log in with Grok",
                )],
            };
            if let Err(error) = state
                .window
                .set_provider_activation(dpl_chat_grok::PROVIDER, activation)
            {
                tracing::warn!(%error, "Grok sign-in requirement could not be projected");
            }
            if let Some(ledger) = state.activation_ledger.as_ref() {
                if let Err(error) = ledger.record_signed_out_for_attempt(
                    dpl_chat_grok::PROVIDER,
                    state.grok_attempt.map(|attempt| attempt.0),
                    &grok_client_version(state),
                    "cached_token",
                ) {
                    tracing::warn!(%error, "Grok signed-out evidence could not be stored");
                }
            }
            publish_grok_failure(
                state,
                ProviderActivationStage::Runtime,
                ProviderFailureClass::Authorization,
                true,
            );
        }
        dpl_chat_grok::GrokRuntimeFailureDisposition::Failed => {
            state.grok_runtime_retry = None;
            state.grok_runtime_retry_at = None;
            let activation = ActivationState::Failed {
                reason: failure.reason,
                recovery: vec![RecoveryAction::new(RecoveryActionKind::Retry, "Retry Grok")],
            };
            if let Err(error) = state
                .window
                .set_provider_activation(dpl_chat_grok::PROVIDER, activation)
            {
                tracing::warn!(%error, "Grok runtime failure could not be projected");
            }
            record_grok_failed(state, "cached_token", "acp_runtime");
            let class = match failure.kind {
                dpl_chat_grok::GrokRuntimeFailureKind::Launch => ProviderFailureClass::Launch,
                dpl_chat_grok::GrokRuntimeFailureKind::Protocol
                | dpl_chat_grok::GrokRuntimeFailureKind::Unsupported
                | dpl_chat_grok::GrokRuntimeFailureKind::InvalidCommand
                | dpl_chat_grok::GrokRuntimeFailureKind::Backpressure => {
                    ProviderFailureClass::Protocol
                }
                dpl_chat_grok::GrokRuntimeFailureKind::CredentialUnavailable
                | dpl_chat_grok::GrokRuntimeFailureKind::Authentication => {
                    ProviderFailureClass::Authorization
                }
                dpl_chat_grok::GrokRuntimeFailureKind::Timeout => ProviderFailureClass::Timeout,
                dpl_chat_grok::GrokRuntimeFailureKind::Disconnected => {
                    ProviderFailureClass::Disconnect
                }
            };
            publish_grok_failure(state, ProviderActivationStage::Runtime, class, true);
        }
    }
}

fn poll_grok_runtime(state: &mut Shared) -> bool {
    let mut changed = false;
    loop {
        let next = match state.grok_runtime.as_ref() {
            Some(runtime) => runtime.try_event(),
            None => return changed,
        };
        let event = match next {
            Ok(Some(event)) => event,
            Ok(None) => return changed,
            Err(error) => {
                tracing::warn!(%error, "Grok ACP event channel closed");
                state.grok_runtime = None;
                handle_grok_runtime_failure(
                    state,
                    dpl_chat_grok::GrokRuntimeFailure {
                        kind: dpl_chat_grok::GrokRuntimeFailureKind::Disconnected,
                        reason: "Grok ACP event channel closed.".to_owned(),
                        provider_session_id: None,
                    },
                );
                return true;
            }
        };
        changed = true;
        match event {
            dpl_chat_grok::GrokRuntimeEvent::Connected { capabilities } => {
                state.grok_runtime_retry = None;
                state.grok_runtime_retry_at = None;
                state.grok_runtime_ready = true;
                state.grok_activation.mark_ready();
                if let Err(error) = state
                    .window
                    .set_provider_activation(dpl_chat_grok::PROVIDER, ActivationState::Ready)
                {
                    tracing::warn!(%error, "Grok authenticated state could not be projected");
                }
                if let Some(ledger) = state.activation_ledger.as_ref() {
                    if let Err(error) = ledger.record_ready_for_attempt(
                        dpl_chat_grok::PROVIDER,
                        state.grok_attempt.map(|attempt| attempt.0),
                        &grok_client_version(state),
                        "cached_token",
                    ) {
                        tracing::warn!(%error, "Grok ready evidence could not be stored");
                    }
                }
                if let Some(attempt) = state.grok_attempt {
                    state.fabric.publish_provider(ProviderEvent::Ready {
                        provider: dpl_chat_grok::PROVIDER,
                        attempt,
                        client_version: grok_client_version(state),
                        auth_method: "cached_token",
                    });
                }
                let mut qualified = vec![
                    Capability::StartSession,
                    Capability::StreamTurns,
                    Capability::InterruptTurn,
                    Capability::Approvals,
                    Capability::Tools,
                    Capability::Diffs,
                    Capability::Plans,
                    Capability::Headless,
                ];
                if capabilities.list_sessions {
                    qualified.push(Capability::ReadSessions);
                }
                if capabilities.load_session {
                    qualified.push(Capability::ResumeSession);
                }
                if let Err(error) = state
                    .window
                    .qualify_provider_capabilities(dpl_chat_grok::PROVIDER, qualified)
                {
                    tracing::warn!(%error, "Grok capabilities could not be projected");
                }
                // Ambient native history is not this project's archive.
                if state
                    .window
                    .provider_view(dpl_chat_grok::PROVIDER)
                    .is_some_and(|view| view.registration.sessions.is_empty())
                {
                    send_grok_command(state, dpl_chat_grok::GrokRuntimeCommand::NewSession);
                }
            }
            dpl_chat_grok::GrokRuntimeEvent::SessionsPage {
                sessions,
                next_cursor,
                replace,
            } => {
                let page_empty = sessions.is_empty();
                project_grok_sessions(state, sessions, replace);
                if let Some(cursor) = next_cursor {
                    send_grok_command(
                        state,
                        dpl_chat_grok::GrokRuntimeCommand::ListSessions {
                            cursor: Some(cursor),
                        },
                    );
                } else if replace && page_empty {
                    send_grok_command(state, dpl_chat_grok::GrokRuntimeCommand::NewSession);
                }
            }
            dpl_chat_grok::GrokRuntimeEvent::SessionReady {
                provider_session_id,
                is_new,
            } => {
                let session = grok_session_summary(
                    state,
                    provider_session_id,
                    is_new.then_some("New Grok chat"),
                    None,
                    SessionGroup::Active,
                );
                let session_id = session.id;
                if let Err(error) = state
                    .window
                    .upsert_provider_session(dpl_chat_grok::PROVIDER, session)
                    .and_then(|()| {
                        state
                            .window
                            .select_provider_session(dpl_chat_grok::PROVIDER, session_id)
                    })
                {
                    tracing::warn!(%error, "Grok session could not be selected");
                } else {
                    state.window.request_composer_focus();
                }
            }
            dpl_chat_grok::GrokRuntimeEvent::TurnStarted {
                provider_session_id,
            } => {
                if let Some(session) = state.grok_sessions.get(&provider_session_id).copied() {
                    if !state.grok_turns.contains_key(&provider_session_id) {
                        let turn = next_turn_id(state);
                        state.grok_turns.insert(provider_session_id, turn);
                    }
                    if let Err(error) = state.window.set_provider_session_state(
                        dpl_chat_grok::PROVIDER,
                        session,
                        SessionState::Running,
                    ) {
                        tracing::warn!(%error, "Grok running state could not be projected");
                    }
                }
            }
            dpl_chat_grok::GrokRuntimeEvent::SessionUpdate {
                provider_session_id,
                update,
            } => project_grok_update(state, &provider_session_id, update),
            dpl_chat_grok::GrokRuntimeEvent::PermissionRequested {
                request_id,
                provider_session_id,
                summary,
                options,
            } => {
                let Some(session) = state.grok_sessions.get(&provider_session_id).copied() else {
                    send_grok_command(
                        state,
                        dpl_chat_grok::GrokRuntimeCommand::ResolvePermission {
                            request_id,
                            option_id: None,
                        },
                    );
                    continue;
                };
                let interaction = InteractionId(state.next_interaction_id);
                state.next_interaction_id = state.next_interaction_id.saturating_add(1);
                let choices = options
                    .iter()
                    .cloned()
                    .map(|option| ProviderPermissionChoice {
                        option_id: option.option_id,
                        label: option.label,
                        kind: match option.kind {
                            dpl_chat_grok::GrokPermissionOptionKind::AllowOnce => {
                                ProviderPermissionChoiceKind::AllowOnce
                            }
                            dpl_chat_grok::GrokPermissionOptionKind::AllowAlways => {
                                ProviderPermissionChoiceKind::AllowAlways
                            }
                            dpl_chat_grok::GrokPermissionOptionKind::RejectOnce => {
                                ProviderPermissionChoiceKind::RejectOnce
                            }
                            dpl_chat_grok::GrokPermissionOptionKind::RejectAlways => {
                                ProviderPermissionChoiceKind::RejectAlways
                            }
                        },
                    })
                    .collect::<Vec<_>>();
                let allow_options = options
                    .iter()
                    .filter(|option| {
                        matches!(
                            option.kind,
                            dpl_chat_grok::GrokPermissionOptionKind::AllowOnce
                                | dpl_chat_grok::GrokPermissionOptionKind::AllowAlways
                        )
                    })
                    .map(|option| option.option_id.clone())
                    .collect::<BTreeSet<_>>();
                if let Err(error) = state.window.register_provider_permission(
                    interaction,
                    dpl_chat_grok::PROVIDER,
                    session,
                    request_id,
                    summary.clone(),
                    choices,
                ) {
                    tracing::warn!(%error, "Grok permission overlay could not be registered");
                    send_grok_command(
                        state,
                        dpl_chat_grok::GrokRuntimeCommand::ResolvePermission {
                            request_id,
                            option_id: None,
                        },
                    );
                } else {
                    state.grok_permissions.insert(
                        request_id,
                        PendingGrokPermission {
                            interaction,
                            provider_session_id: provider_session_id.clone(),
                            allow_options,
                        },
                    );
                    append_grok_event(
                        state,
                        &provider_session_id,
                        ChatEventContent::ApprovalRequested { summary },
                    );
                }
            }
            dpl_chat_grok::GrokRuntimeEvent::TurnFinished {
                provider_session_id,
                stop_reason: _,
            } => {
                append_grok_event(state, &provider_session_id, ChatEventContent::TurnCompleted);
                ai_call_finished(state, &provider_session_id, AiOutcome::Ok);
                set_grok_session_idle(state, &provider_session_id);
                state.grok_turns.remove(&provider_session_id);
            }
            dpl_chat_grok::GrokRuntimeEvent::Failure(failure) => {
                if let Some(provider_session_id) = failure.provider_session_id.clone() {
                    append_grok_event(
                        state,
                        &provider_session_id,
                        ChatEventContent::TurnFailed {
                            message: failure.reason,
                        },
                    );
                    ai_call_finished(
                        state,
                        &provider_session_id,
                        AiOutcome::Error(AiFailureClass::Runtime),
                    );
                    set_grok_session_idle(state, &provider_session_id);
                    state.grok_turns.remove(&provider_session_id);
                } else {
                    handle_grok_runtime_failure(state, failure);
                }
            }
            dpl_chat_grok::GrokRuntimeEvent::Stopped => {
                cancel_grok_permissions(state, None, false);
                state.grok_runtime = None;
                state.grok_runtime_ready = false;
                return true;
            }
        }
    }
}

fn project_grok_sessions(
    state: &mut Shared,
    sessions: Vec<dpl_chat_grok::GrokSessionSummary>,
    replace: bool,
) {
    let mut projected = if replace {
        Vec::new()
    } else {
        state
            .window
            .provider_view(dpl_chat_grok::PROVIDER)
            .map_or_else(Vec::new, |provider| provider.registration.sessions)
    };
    for session in sessions {
        let projected_session = grok_session_summary(
            state,
            session.provider_session_id,
            session.title.as_deref(),
            Some(session.working_directory),
            SessionGroup::Recent,
        );
        if let Some(existing) = projected
            .iter_mut()
            .find(|existing| existing.id == projected_session.id)
        {
            *existing = projected_session;
        } else {
            projected.push(projected_session);
        }
    }
    let first_session = projected.first().map(|session| session.id);
    if let Err(error) = state
        .window
        .replace_provider_sessions(dpl_chat_grok::PROVIDER, projected)
    {
        tracing::warn!(%error, "Grok session page could not be projected");
    } else if let Some(session) = first_session {
        let has_active = state
            .window
            .provider_view(dpl_chat_grok::PROVIDER)
            .and_then(|provider| provider.active_session)
            .is_some();
        if !has_active {
            if let Err(error) = state
                .window
                .select_provider_session(dpl_chat_grok::PROVIDER, session)
            {
                tracing::warn!(%error, "first Grok session could not be selected");
            } else {
                state.window.request_composer_focus();
            }
        }
    }
}

fn grok_session_summary(
    state: &mut Shared,
    provider_session_id: String,
    title: Option<&str>,
    working_directory: Option<std::path::PathBuf>,
    group: SessionGroup,
) -> SessionSummary {
    let id = grok_chat_session_id(state, &provider_session_id);
    SessionSummary {
        id,
        provider_id: dpl_chat_grok::PROVIDER,
        provider_session_id: Some(ProviderSessionId(provider_session_id)),
        title: title.unwrap_or("Grok chat").to_owned(),
        workspace: working_directory
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| project::workspace_label(state)),
        group,
        state: SessionState::Idle,
    }
}

fn grok_chat_session_id(state: &mut Shared, provider_session_id: &str) -> ChatSessionId {
    if let Some(id) = state.grok_sessions.get(provider_session_id) {
        return *id;
    }
    let id = ChatSessionId(state.next_chat_session_id);
    state.next_chat_session_id = state.next_chat_session_id.saturating_add(1);
    state
        .grok_sessions
        .insert(provider_session_id.to_owned(), id);
    id
}

/// Resolve the PROVIDER's own id for a chat session.
///
/// Takes the provider explicitly: this used to read Grok's view no matter who
/// asked, which was harmless only while Grok was the one provider with live
/// sessions. This pass is what ends that.
fn provider_session_id(
    state: &Shared,
    provider: ProviderId,
    session: ChatSessionId,
) -> Option<String> {
    state
        .window
        .provider_view(provider)?
        .registration
        .sessions
        .into_iter()
        .find(|candidate| candidate.id == session)?
        .provider_session_id
        .map(|id| id.0)
}

/// One in-flight AI call. Providers speak their own dialects; this is the
/// single shape the AI domain is minted from.
struct AiCall {
    call: CallId,
    /// Kept so the drift check knows whose dialect to hold to account.
    provider: ProviderId,
    started: Instant,
    /// TTFT fires exactly once per call, on the first content chunk.
    first_token_seen: bool,
    /// Accumulated across the turn — providers report usage in pieces,
    /// and later reports supersede earlier ones field-by-field.
    usage: TokenUsage,
}

/// THE CHAT → AI BRIDGE.
///
/// Provider crates stay fabric-free (their ceilings forbid it); the app
/// owns the translation, exactly like the `publish_provider` sites above.
/// These four functions are the ONLY place `AiEvent` is minted, so the
/// correlation invariant — one `CallId`, one start, one finish — is
/// enforced in one readable place rather than scattered per provider.
fn ai_call_started(
    state: &mut Shared,
    provider: dpl_chat_core::ProviderId,
    provider_session_id: &str,
    model: &str,
) {
    let call = state.fabric.mint_ai_call();
    state.ai_calls.insert(
        provider_session_id.to_owned(),
        AiCall {
            call,
            provider,
            started: Instant::now(),
            first_token_seen: false,
            usage: TokenUsage::default(),
        },
    );
    state.fabric.publish_ai(AiEvent::CallStarted {
        call,
        provider,
        model: model.to_owned(),
        // The chat window is its own OS surface and does not own the main
        // window's route; `observe_route` feeds the projection, not this.
        // Route-bearing origins arrive with the editor surface (PR 3) —
        // an absent route is honest, a guessed one would poison the join.
        origin: Origin::surface("chat"),
        // Witnessed as it happens — the backfill pass is the only
        // producer of the other kind.
        observation: dpl_fabric::Observation::Live,
    });
}

/// The latency operators actually feel. Idempotent per call.
fn ai_first_token(state: &mut Shared, provider_session_id: &str) {
    let Some(entry) = state.ai_calls.get_mut(provider_session_id) else {
        return;
    };
    if entry.first_token_seen {
        return;
    }
    entry.first_token_seen = true;
    let (call, ttft_ms) = (entry.call, entry.started.elapsed().as_millis() as u32);
    state
        .fabric
        .publish_ai(AiEvent::FirstToken { call, ttft_ms });
}

/// Closes the call and RETIRES the correlation entry, so a late duplicate
/// finish cannot publish a second `CallFinished` for the same id.
fn ai_call_finished(state: &mut Shared, provider_session_id: &str, outcome: AiOutcome) {
    let Some(entry) = state.ai_calls.remove(provider_session_id) else {
        return;
    };
    let usage = entry.usage;
    // A provider that DECLARED it reports usage and then delivered none
    // for a whole call has likely changed its payload shape — surface the
    // drift instead of recording a silent zero.
    crate::ai_usage::note_usage_drift(entry.provider, usage);
    state.fabric.publish_ai(AiEvent::CallFinished {
        call: entry.call,
        tokens_in: usage.input_tokens,
        cache_read: usage.cache_read_tokens,
        cache_write: usage.cache_write_tokens,
        tokens_out: usage.output_tokens,
        total_tokens: usage.total_tokens,
        context_used: usage.context_used,
        context_size: usage.context_size,
        // Cost stays unpriced HERE: the versioned price table (PR 2) is
        // what turns tokens into money, and it needs the model id. The
        // token counts above are the truth cost is recomputed from.
        cost_micros: None,
        duration_ms: entry.started.elapsed().as_millis() as u32,
        outcome,
    });
}

/// A provider reported token accounting mid-turn. MERGED, not replaced —
/// Codex reports an input/output split, Grok reports context occupancy,
/// and a turn may carry several reports before it ends.
fn ai_usage_observed(state: &mut Shared, provider_session_id: &str, usage: TokenUsage) {
    if let Some(entry) = state.ai_calls.get_mut(provider_session_id) {
        entry.usage = entry.usage.merge(usage);
    }
}

fn send_grok_turn(state: &mut Shared, session: ChatSessionId, text: String) {
    let Some(provider_session_id) = provider_session_id(state, dpl_chat_grok::PROVIDER, session)
    else {
        tracing::warn!(%session, "Grok session binding is missing");
        return;
    };
    let turn = next_turn_id(state);
    let event_id = next_event_id(state);
    let event = ChatEvent {
        id: event_id,
        provider_id: dpl_chat_grok::PROVIDER,
        session_id: session,
        turn_id: Some(turn.clone()),
        content: ChatEventContent::UserMessage(text.clone()),
    };
    let command = dpl_chat_grok::GrokRuntimeCommand::Prompt {
        provider_session_id: provider_session_id.clone(),
        text,
    };
    if !send_grok_command(state, command) {
        return;
    }
    // The call is airborne — mint its correlation id before any chunk
    // can arrive, so TTFT has something to attach to.
    // Model is EMPTY here on purpose: the Grok ACP runtime does not
    // surface a model id at this layer. Model resolution lands with the
    // price table (PR 2) — the two are inseparable, since pricing is
    // per-model. An empty model reads as "not yet known", never as a
    // wrong model quietly priced.
    ai_call_started(state, dpl_chat_grok::PROVIDER, &provider_session_id, "");
    state.grok_turns.insert(provider_session_id, turn);
    if let Err(error) = state
        .window
        .append_provider_event(dpl_chat_grok::PROVIDER, event)
        .and_then(|()| state.window.clear_provider_draft(dpl_chat_grok::PROVIDER))
    {
        tracing::warn!(%error, "Grok user message could not be projected");
    }
}

fn project_grok_update(
    state: &mut Shared,
    provider_session_id: &str,
    update: dpl_chat_grok::GrokSessionUpdate,
) {
    match update {
        dpl_chat_grok::GrokSessionUpdate::UserText(_) => {
            // The submitted message is projected before the provider call so
            // the composer clears atomically with a successful enqueue.
        }
        dpl_chat_grok::GrokSessionUpdate::AssistantText(chunk) => {
            ai_first_token(state, provider_session_id);
            let Some(session) = state.grok_sessions.get(provider_session_id).copied() else {
                return;
            };
            let turn = state
                .grok_turns
                .get(provider_session_id)
                .cloned()
                .unwrap_or_else(|| next_turn_id(state));
            let event = next_event_id(state);
            if let Err(error) = state.window.append_provider_assistant_chunk(
                dpl_chat_grok::PROVIDER,
                session,
                turn,
                event,
                &chunk,
            ) {
                tracing::warn!(%error, "Grok assistant chunk could not be projected");
            }
        }
        dpl_chat_grok::GrokSessionUpdate::ThoughtText(text) => {
            let Some(session) = state.grok_sessions.get(provider_session_id).copied() else {
                return;
            };
            let turn = state
                .grok_turns
                .get(provider_session_id)
                .cloned()
                .unwrap_or_else(|| next_turn_id(state));
            let event = next_event_id(state);
            if let Err(error) = state.window.append_provider_status_chunk(
                dpl_chat_grok::PROVIDER,
                session,
                turn,
                event,
                &text,
            ) {
                tracing::warn!(%error, "Grok reasoning activity could not be projected");
            }
        }
        dpl_chat_grok::GrokSessionUpdate::Tool(label) => append_grok_event(
            state,
            provider_session_id,
            ChatEventContent::ToolStarted { label },
        ),
        dpl_chat_grok::GrokSessionUpdate::Plan(plan) => append_grok_event(
            state,
            provider_session_id,
            ChatEventContent::PlanUpdated(plan),
        ),
        dpl_chat_grok::GrokSessionUpdate::SessionInfo(info) => append_grok_event(
            state,
            provider_session_id,
            ChatEventContent::StatusUpdated(info),
        ),
        dpl_chat_grok::GrokSessionUpdate::AvailableCommands { count } => append_grok_event(
            state,
            provider_session_id,
            ChatEventContent::StatusUpdated(format!("{count} provider commands available")),
        ),
        dpl_chat_grok::GrokSessionUpdate::Usage { used, size } => {
            // The dialect lives in GROK'S crate, not here — one adapter
            // per provider (Phase 3 §3.2b), so a Grok release changes one
            // file and touches no other provider.
            let usage = dpl_chat_grok::GrokUsage::usage_from_update(
                &dpl_chat_grok::GrokSessionUpdate::Usage { used, size },
            )
            .unwrap_or_default();
            ai_usage_observed(state, provider_session_id, usage);
            append_grok_event(
                state,
                provider_session_id,
                ChatEventContent::UsageUpdated {
                    summary: format!("{used} of {size} context tokens"),
                    usage,
                },
            );
        }
        dpl_chat_grok::GrokSessionUpdate::Other(summary) => append_grok_event(
            state,
            provider_session_id,
            ChatEventContent::Unknown(BoundedNativePayload::redacted(
                "grok.acp.session-update",
                summary,
            )),
        ),
    }
}

fn append_grok_event(state: &mut Shared, provider_session_id: &str, content: ChatEventContent) {
    let Some(session) = state.grok_sessions.get(provider_session_id).copied() else {
        return;
    };
    let event = ChatEvent {
        id: next_event_id(state),
        provider_id: dpl_chat_grok::PROVIDER,
        session_id: session,
        turn_id: state.grok_turns.get(provider_session_id).cloned(),
        content,
    };
    if let Err(error) = state
        .window
        .append_provider_event(dpl_chat_grok::PROVIDER, event)
    {
        tracing::warn!(%error, "Grok event could not be projected");
    }
}

// ─────────────────────────── THE SEAM HOST ───────────────────────────
//
// ONE path for every provider that implements `ProviderRuntime`. The Grok
// functions above are its ancestor — structurally identical, but hard-bound to
// one provider. These take a `ProviderId`, so the next provider to grow a
// transport costs a `start()` and nothing here.
//
// Grok deliberately does NOT flow through this path yet: it works, and
// converging it is its own pass (docs-history-2026-10-12:docs/plans/codex-host-wiring.md §5).

/// Start Codex's app-server runtime once activation says the operator is
/// actually signed in.
///
/// Idempotent and cheap: it returns immediately when a runtime already exists
/// or activation has not reached an activated state, so the frame loop can
/// call it every frame.
fn drive_codex_runtime(state: &mut Shared) -> bool {
    if !state.provider_runtime_enabled
        || !project::ready(state)
        || state.runtimes.contains_key(&dpl_chat_codex::PROVIDER)
    {
        return false;
    }
    if !state.codex_activation.activation().is_activated() {
        return false;
    }
    let Some(profile) = state.codex_activation.runtime_profile() else {
        return false;
    };
    // The pinned schema and the discovered binary can disagree — the depot
    // copy and a PATH copy are different builds. Say so in the log rather than
    // let a protocol mismatch surface later as a turn that never answers.
    let discovered = profile.version().to_string();
    if !discovered.is_empty()
        && !dpl_chat_codex::runtime::PINNED_CLI_VERSION.starts_with(&discovered)
    {
        tracing::info!(
            discovered = %discovered,
            pinned = %dpl_chat_codex::runtime::PINNED_CLI_VERSION,
            "Codex app-server build differs from the pinned schema"
        );
    }
    match dpl_chat_codex::runtime::CodexAppServerRuntime::start(&profile) {
        Ok(runtime) => {
            state
                .runtimes
                .insert(dpl_chat_codex::PROVIDER, Box::new(runtime));
            if let Some(attempt) = state.codex_attempt {
                state
                    .fabric
                    .publish_provider(ProviderEvent::RuntimeStarted {
                        provider: dpl_chat_codex::PROVIDER,
                        attempt,
                    });
            }
            true
        }
        Err(error) => {
            tracing::warn!(%error, "Codex app-server could not start");
            let activation = ActivationState::Failed {
                reason: format!("ChatGPT could not start: {error}"),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::Retry,
                    "Retry ChatGPT",
                )],
            };
            if let Err(error) = state
                .window
                .set_provider_activation(dpl_chat_codex::PROVIDER, activation)
            {
                tracing::warn!(%error, "Codex launch failure could not be projected");
            }
            record_codex_failed(state, "app_server_launch");
            publish_codex_failure(
                state,
                ProviderActivationStage::Runtime,
                ProviderFailureClass::Launch,
                true,
            );
            true
        }
    }
}

/// Drain every seam runtime's event queue.
fn poll_seam_runtimes(state: &mut Shared) -> bool {
    let providers = state.runtimes.keys().copied().collect::<Vec<_>>();
    let mut changed = false;
    for provider in providers {
        while let Some(event) = take_seam_event(state, provider) {
            changed = true;
            project_seam_event(state, provider, event);
        }
        // A runtime retired mid-drain is a state change the frame must see,
        // even if it never produced an event.
        changed |= !state.runtimes.contains_key(&provider);
    }
    changed
}

/// Take one event, retiring the runtime if its channel has closed.
///
/// A closed channel reads the same as an empty one to the caller — `None` —
/// because both mean "nothing more this frame". The difference is recorded by
/// removing the runtime, which is what lets the next frame restart it.
fn take_seam_event(state: &mut Shared, provider: ProviderId) -> Option<RuntimeEvent> {
    // Bound first so the immutable borrow ends before the arms need `state`.
    let taken = state.runtimes.get(&provider)?.try_event();
    match taken {
        Ok(event) => event,
        Err(error) => {
            tracing::warn!(%provider, %error, "provider runtime channel closed");
            state.runtimes.remove(&provider);
            None
        }
    }
}

fn project_seam_event(state: &mut Shared, provider: ProviderId, event: RuntimeEvent) {
    match event {
        RuntimeEvent::Connected { capabilities } => {
            if let Err(error) = state
                .window
                .set_provider_activation(provider, ActivationState::Ready)
            {
                tracing::warn!(%provider, %error, "provider ready state could not be projected");
            }
            // EARNED, never assumed: one function derives the capability set
            // from what the provider advertised, so a surface can never offer
            // what the adapter cannot honour.
            let qualified = qualified_capabilities(capabilities);
            // The one line that says a provider became TALKABLE. Without it a
            // connected runtime is indistinguishable from a stalled one in a
            // log, which is how "it shows active and I cannot chat" stayed
            // unexplained for so long.
            tracing::info!(
                %provider,
                can_send = qualified.contains(&Capability::StreamTurns),
                qualified = ?qualified,
                "provider runtime connected"
            );
            if let Err(error) = state
                .window
                .qualify_provider_capabilities(provider, qualified)
            {
                tracing::warn!(%provider, %error, "provider capabilities could not be projected");
            }
            // The operator's permission mode rides from the first turn, for a
            // runtime that can honour it (one that cannot never advertised it).
            if capabilities.permission_modes {
                let mode = state
                    .permission_modes
                    .get(&provider)
                    .copied()
                    .unwrap_or_default();
                state
                    .window
                    .set_provider_permission_mode(provider, Some(mode));
                send_seam_command(state, provider, RuntimeCommand::SetPermissionMode { mode });
            }
            // A connection must not replace the restored, read-only chat.
            if state
                .window
                .provider_view(provider)
                .is_some_and(|view| view.registration.sessions.is_empty())
            {
                send_seam_command(state, provider, RuntimeCommand::NewSession);
            }
        }
        RuntimeEvent::Account(account) => {
            // "Ready" is the ACCOUNT, not the process: a healthy app-server
            // whose CLI is signed out can send nothing.
            let activation = match &account {
                Some(account) => {
                    tracing::info!(
                        %provider,
                        account = %account.display_name,
                        plan = account.plan.as_deref().unwrap_or("none"),
                        "provider account"
                    );
                    ActivationState::Ready
                }
                None => ActivationState::SignedOut {
                    reason: "Not signed in. Sign in with your own account, then press Retry."
                        .to_owned(),
                    recovery: vec![
                        RecoveryAction::new(RecoveryActionKind::SignIn, "Sign in"),
                        RecoveryAction::new(RecoveryActionKind::Retry, "Retry"),
                    ],
                },
            };
            if let Err(error) = state
                .window
                .set_provider_account(provider, account)
                .and_then(|()| state.window.set_provider_activation(provider, activation))
            {
                tracing::warn!(%provider, %error, "provider account could not be projected");
            }
        }
        RuntimeEvent::SessionsPage { .. } => {
            // Never reached today: no seam provider advertises `list_sessions`,
            // so the surface never asks. Kept explicit so adding one is a
            // compile-time conversation rather than a silent no-op.
        }
        RuntimeEvent::SessionReady {
            provider_session_id,
            is_new,
        } => {
            let session = seam_chat_session_id(state, provider, &provider_session_id);
            let summary = SessionSummary {
                id: session,
                provider_id: provider,
                title: if is_new {
                    "New chat".to_owned()
                } else {
                    provider_session_id.clone()
                },
                provider_session_id: Some(ProviderSessionId(provider_session_id)),
                workspace: project::workspace_label(state),
                group: SessionGroup::Active,
                state: SessionState::Idle,
            };
            if let Err(error) = state
                .window
                .upsert_provider_session(provider, summary)
                .and_then(|()| state.window.select_provider_session(provider, session))
            {
                tracing::warn!(%provider, %error, "provider session could not be selected");
            } else {
                state.window.request_composer_focus();
            }
        }
        RuntimeEvent::TurnStarted {
            provider_session_id,
        } => {
            let key = (provider, provider_session_id);
            if let Some(session) = state.seam_sessions.get(&key).copied() {
                if !state.seam_turns.contains_key(&key) {
                    let turn = next_turn_id(state);
                    state.seam_turns.insert(key, turn);
                }
                if let Err(error) = state.window.set_provider_session_state(
                    provider,
                    session,
                    SessionState::Running,
                ) {
                    tracing::warn!(%provider, %error, "running state could not be projected");
                }
            }
        }
        RuntimeEvent::SessionUpdate {
            provider_session_id,
            update,
        } => project_seam_update(state, provider, &provider_session_id, update),
        RuntimeEvent::PermissionRequested {
            request_id,
            provider_session_id,
            summary,
            options,
        } => project_seam_permission(
            state,
            provider,
            request_id,
            provider_session_id,
            summary,
            options,
        ),
        RuntimeEvent::TurnFinished {
            provider_session_id,
            stop_reason: _,
        } => {
            append_seam_event(
                state,
                provider,
                &provider_session_id,
                ChatEventContent::TurnCompleted,
            );
            ai_call_finished(state, &provider_session_id, AiOutcome::Ok);
            set_seam_session_idle(state, provider, &provider_session_id);
            state.seam_turns.remove(&(provider, provider_session_id));
        }
        RuntimeEvent::Failure(failure) => {
            tracing::warn!(%provider, kind = ?failure.kind, reason = %failure.reason, "provider runtime failure");
            if let Some(provider_session_id) = failure.provider_session_id.clone() {
                cancel_seam_permissions(state, provider, &provider_session_id);
                let plain = plain_failure(&failure.reason);
                if let Some(activation) = plain.activation.clone() {
                    if let Err(error) = state.window.set_provider_activation(provider, activation) {
                        tracing::warn!(%provider, %error, "failure state could not be projected");
                    }
                }
                append_seam_event(
                    state,
                    provider,
                    &provider_session_id,
                    ChatEventContent::TurnFailed {
                        message: plain.message,
                    },
                );
                ai_call_finished(
                    state,
                    &provider_session_id,
                    AiOutcome::Error(AiFailureClass::Runtime),
                );
                set_seam_session_idle(state, provider, &provider_session_id);
                state.seam_turns.remove(&(provider, provider_session_id));
            } else {
                let class = match failure.kind {
                    RuntimeFailureKind::Launch => ProviderFailureClass::Launch,
                    RuntimeFailureKind::CredentialUnavailable
                    | RuntimeFailureKind::Authentication => ProviderFailureClass::Authorization,
                    RuntimeFailureKind::Timeout => ProviderFailureClass::Timeout,
                    RuntimeFailureKind::Disconnected => ProviderFailureClass::Disconnect,
                    RuntimeFailureKind::Protocol
                    | RuntimeFailureKind::Unsupported
                    | RuntimeFailureKind::InvalidCommand
                    | RuntimeFailureKind::Backpressure => ProviderFailureClass::Protocol,
                };
                let activation = ActivationState::Failed {
                    reason: failure.reason,
                    recovery: vec![RecoveryAction::new(RecoveryActionKind::Retry, "Retry")],
                };
                if let Err(error) = state.window.set_provider_activation(provider, activation) {
                    tracing::warn!(%provider, %error, "runtime failure could not be projected");
                }
                if provider == dpl_chat_codex::PROVIDER {
                    record_codex_failed(state, "app_server_runtime");
                    publish_codex_failure(state, ProviderActivationStage::Runtime, class, true);
                }
            }
        }
        RuntimeEvent::Stopped => {
            state.runtimes.remove(&provider);
        }
    }
}

fn project_seam_update(
    state: &mut Shared,
    provider: ProviderId,
    provider_session_id: &str,
    update: RuntimeUpdate,
) {
    let key = (provider, provider_session_id.to_owned());
    match update {
        RuntimeUpdate::UserText(_) => {
            // Projected at submit time, so the composer clears atomically with
            // a successful enqueue.
        }
        RuntimeUpdate::AssistantText(chunk) => {
            ai_first_token(state, provider_session_id);
            let Some(session) = state.seam_sessions.get(&key).copied() else {
                return;
            };
            let turn = state
                .seam_turns
                .get(&key)
                .cloned()
                .unwrap_or_else(|| next_turn_id(state));
            let event = next_event_id(state);
            if let Err(error) = state
                .window
                .append_provider_assistant_chunk(provider, session, turn, event, &chunk)
            {
                tracing::warn!(%provider, %error, "assistant chunk could not be projected");
            }
        }
        RuntimeUpdate::ThoughtText(text) => {
            let Some(session) = state.seam_sessions.get(&key).copied() else {
                return;
            };
            let turn = state
                .seam_turns
                .get(&key)
                .cloned()
                .unwrap_or_else(|| next_turn_id(state));
            let event = next_event_id(state);
            if let Err(error) = state
                .window
                .append_provider_status_chunk(provider, session, turn, event, &text)
            {
                tracing::warn!(%provider, %error, "reasoning activity could not be projected");
            }
        }
        RuntimeUpdate::Tool(label) => append_seam_event(
            state,
            provider,
            provider_session_id,
            ChatEventContent::ToolStarted { label },
        ),
        RuntimeUpdate::Plan(plan) => append_seam_event(
            state,
            provider,
            provider_session_id,
            ChatEventContent::PlanUpdated(plan),
        ),
        RuntimeUpdate::SessionInfo(info) => append_seam_event(
            state,
            provider,
            provider_session_id,
            ChatEventContent::StatusUpdated(info),
        ),
        RuntimeUpdate::AvailableCommands { count } => append_seam_event(
            state,
            provider,
            provider_session_id,
            ChatEventContent::StatusUpdated(format!("{count} provider commands available")),
        ),
        RuntimeUpdate::Usage { used, size } => {
            let usage = TokenUsage {
                context_used: Some(used),
                context_size: Some(size),
                ..TokenUsage::default()
            };
            ai_usage_observed(state, provider_session_id, usage);
            append_seam_event(
                state,
                provider,
                provider_session_id,
                ChatEventContent::UsageUpdated {
                    summary: format!("{used} of {size} context tokens"),
                    usage,
                },
            );
        }
        // An unknown frame is EVIDENCE, kept redacted rather than dropped.
        RuntimeUpdate::Other(summary) => append_seam_event(
            state,
            provider,
            provider_session_id,
            ChatEventContent::Unknown(BoundedNativePayload::redacted(
                "provider.seam.session-update",
                summary,
            )),
        ),
    }
}

fn seam_chat_session_id(
    state: &mut Shared,
    provider: ProviderId,
    provider_session_id: &str,
) -> ChatSessionId {
    let key = (provider, provider_session_id.to_owned());
    if let Some(id) = state.seam_sessions.get(&key) {
        return *id;
    }
    let id = ChatSessionId(state.next_chat_session_id);
    state.next_chat_session_id = state.next_chat_session_id.saturating_add(1);
    state.seam_sessions.insert(key, id);
    id
}

fn append_seam_event(
    state: &mut Shared,
    provider: ProviderId,
    provider_session_id: &str,
    content: ChatEventContent,
) {
    let key = (provider, provider_session_id.to_owned());
    let Some(session) = state.seam_sessions.get(&key).copied() else {
        return;
    };
    let event = ChatEvent {
        id: next_event_id(state),
        provider_id: provider,
        session_id: session,
        turn_id: state.seam_turns.get(&key).cloned(),
        content,
    };
    if let Err(error) = state.window.append_provider_event(provider, event) {
        tracing::warn!(%provider, %error, "provider event could not be projected");
    }
}

fn set_seam_session_idle(state: &mut Shared, provider: ProviderId, provider_session_id: &str) {
    let Some(session) = state
        .seam_sessions
        .get(&(provider, provider_session_id.to_owned()))
        .copied()
    else {
        return;
    };
    if let Err(error) =
        state
            .window
            .set_provider_session_state(provider, session, SessionState::Idle)
    {
        tracing::warn!(%provider, %error, "idle state could not be projected");
    }
}

/// Queue one command, translating backpressure honestly.
///
/// `Full` is TRANSIENT and must never read as the operator's failure; `Closed`
/// retires the runtime so the next frame can restart it.
fn send_seam_command(state: &mut Shared, provider: ProviderId, command: RuntimeCommand) -> bool {
    let Some(runtime) = state.runtimes.get(&provider) else {
        return false;
    };
    match runtime.try_send(command) {
        Ok(()) => true,
        Err(dpl_chat_runtime::seam::RuntimeSendError::Full) => {
            tracing::warn!(%provider, "provider command queue is full; command dropped");
            false
        }
        Err(dpl_chat_runtime::seam::RuntimeSendError::Closed) => {
            tracing::warn!(%provider, "provider runtime is closed");
            state.runtimes.remove(&provider);
            false
        }
    }
}

fn send_seam_turn(state: &mut Shared, provider: ProviderId, session: ChatSessionId, text: String) {
    let Some(provider_session_id) = provider_session_id(state, provider, session) else {
        tracing::warn!(%provider, %session, "provider session binding is missing");
        return;
    };
    let turn = next_turn_id(state);
    let event_id = next_event_id(state);
    let event = ChatEvent {
        id: event_id,
        provider_id: provider,
        session_id: session,
        turn_id: Some(turn.clone()),
        content: ChatEventContent::UserMessage(text.clone()),
    };
    if !send_seam_command(
        state,
        provider,
        RuntimeCommand::Prompt {
            provider_session_id: provider_session_id.clone(),
            text,
        },
    ) {
        return;
    }
    // Airborne — mint the correlation id before any chunk can arrive, so TTFT
    // has something to attach to. Model stays EMPTY until the price table
    // lands; an empty model reads "not yet known", never a wrong model priced.
    ai_call_started(state, provider, &provider_session_id, "");
    state
        .seam_turns
        .insert((provider, provider_session_id), turn);
    if let Err(error) = state
        .window
        .append_provider_event(provider, event)
        .and_then(|()| state.window.clear_provider_draft(provider))
    {
        tracing::warn!(%provider, %error, "user message could not be projected");
    }
}

fn set_grok_session_idle(state: &mut Shared, provider_session_id: &str) {
    let Some(session) = state.grok_sessions.get(provider_session_id).copied() else {
        return;
    };
    if let Err(error) = state.window.set_provider_session_state(
        dpl_chat_grok::PROVIDER,
        session,
        SessionState::Idle,
    ) {
        tracing::warn!(%error, "Grok idle state could not be projected");
    }
}

/// A failed turn, in words the operator can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlainFailure {
    message: String,
    /// The state the provider moves to when the fix is the operator's.
    activation: Option<ActivationState>,
}

/// Translate a provider's failure into a sentence. The provider's own words
/// stay in the log; raw JSON never reaches the chat.
fn plain_failure(reason: &str) -> PlainFailure {
    // Codex sometimes forwards the whole API error as JSON; its message is
    // the part a person can read.
    let message =
        serde_json::from_str::<serde_json::Value>(reason.trim_start_matches(|c: char| c != '{'))
            .ok()
            .and_then(|value| {
                value
                    .pointer("/error/message")
                    .or_else(|| value.get("message"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| reason.to_owned());
    let lower = message.to_ascii_lowercase();
    if lower.contains("requires a newer version of codex") {
        let sentence = "Codex needs an update for this model. Choose Update Codex, then try again.";
        return PlainFailure {
            message: sentence.to_owned(),
            activation: Some(ActivationState::VersionUnsupported {
                reason: sentence.to_owned(),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::UpdateProvider,
                    "Update Codex",
                )],
            }),
        };
    }
    if lower.contains("unauthorized") || lower.contains("401") || lower.contains("not logged in") {
        let sentence = "Codex is not signed in, or its sign-in expired. Sign in, then try again.";
        return PlainFailure {
            message: sentence.to_owned(),
            activation: Some(ActivationState::SignedOut {
                reason: sentence.to_owned(),
                recovery: vec![
                    RecoveryAction::new(RecoveryActionKind::SignIn, "Sign in"),
                    RecoveryAction::new(RecoveryActionKind::Retry, "Retry"),
                ],
            }),
        };
    }
    if lower.contains("usage limit") || lower.contains("rate limit") {
        return PlainFailure {
            message: "You have reached this provider's usage limit for now. Try again later."
                .to_owned(),
            activation: None,
        };
    }
    PlainFailure {
        message,
        activation: None,
    }
}

/// "Update Codex": adopt the operator's installed Codex (OpenAI's signature,
/// its version, its protocol) into the depot beside this exe, then bring
/// Codex back on the new build. Off the frame thread: it copies a large
/// binary and asks Windows to vouch for it.
/// Where the operator's own roles and rooms live (R284): one JSON file
/// each, under `roles/` and `rooms/`.
fn room_library_root() -> PathBuf {
    dpl_roots::userdata_root().join("chat").join("library")
}

/// The built-in library plus every role and room file the operator added.
/// A file that cannot be read, or fails its schema or rules, is named in the
/// Rooms & roles view and never loaded.
fn load_room_library(root: &Path) -> dpl_chat_core::rooms::Library {
    let mut unreadable = Vec::new();
    let mut read = |kind: &str| {
        let mut files = Vec::new();
        let Ok(entries) = std::fs::read_dir(root.join(kind)) else {
            return files;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let name = format!(
                "{kind}/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            match std::fs::read_to_string(&path) {
                Ok(text) => files.push((name, text)),
                Err(error) => unreadable.push((
                    name.clone(),
                    vec![dpl_chat_core::rooms::LibraryIssue {
                        at: name,
                        problem: format!("could not be read: {error}"),
                    }],
                )),
            }
        }
        files.sort();
        files
    };
    let roles = read("roles");
    let rooms = read("rooms");
    let mut library = dpl_chat_core::rooms::Library::with_user(&roles, &rooms);
    library.rejected.extend(unreadable);
    for issue in library.rejected.iter().flat_map(|(_, issues)| issues) {
        tracing::warn!(%issue, "room library file not loaded");
    }
    library
}

/// Show a folder in the system's file manager.
fn reveal_folder(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(path);
    let mut child = dpl_proc::quiet(&mut command).spawn()?;
    // Reap it off the UI thread; the file manager outlives nothing we own.
    std::thread::spawn(move || child.wait());
    Ok(())
}

fn update_codex(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>) {
    let Some(installed) = dpl_chat_codex::adoption::installed_codex() else {
        return;
    };
    let tools_root = match std::env::current_exe() {
        Ok(exe) => match exe.parent() {
            Some(folder) => folder.join(dpl_tools_deps::TOOLS_DIRECTORY),
            None => return,
        },
        Err(error) => {
            tracing::warn!(%error, "the running exe could not be located for the update");
            return;
        }
    };
    {
        let mut state = lock(shared);
        // Stop Codex first, so the old build is not in use while it is replaced.
        state.runtimes.remove(&dpl_chat_codex::PROVIDER);
        let _ = state
            .window
            .set_provider_activation(dpl_chat_codex::PROVIDER, ActivationState::Verifying);
    }
    let shared = Arc::clone(shared);
    let ctx = ctx.clone();
    let spawned = std::thread::Builder::new()
        .name("devpulse-codex-update".to_owned())
        .spawn(move || {
            let outcome = dpl_chat_codex::adoption::adopt(
                &installed,
                &tools_root,
                &dpl_chat_codex::adoption::SystemInspector,
            );
            let mut state = lock(&shared);
            match outcome {
                Ok((verified, installed)) => {
                    dpl_tools_deps::reload_current();
                    tracing::info!(
                        version = %verified.version,
                        replaced = installed.replaced.as_deref().unwrap_or("none"),
                        "Codex updated from the operator's installed build"
                    );
                    if let Some(folder) = installed.left_behind {
                        tracing::info!(folder = %folder.display(), "the old Codex stays until the next update");
                    }
                    match state.codex_activation.refresh_discovery() {
                        Ok(_) => {
                            let activation = state.codex_activation.activation().clone();
                            let _ = state
                                .window
                                .set_provider_activation(dpl_chat_codex::PROVIDER, activation);
                        }
                        Err(error) => tracing::warn!(%error, "Codex rediscovery was rejected"),
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "Codex update refused");
                    let _ = state.window.set_provider_activation(
                        dpl_chat_codex::PROVIDER,
                        ActivationState::VersionUnsupported {
                            reason: error.to_string(),
                            recovery: vec![RecoveryAction::new(
                                RecoveryActionKind::UpdateProvider,
                                "Update Codex",
                            )],
                        },
                    );
                }
            }
            drop(state);
            ctx.request_repaint();
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "the Codex update could not start");
    }
}

/// An approval request from a seam provider becomes the operator's card, or
/// (Read only) a decline the chat records.
fn project_seam_permission(
    state: &mut Shared,
    provider: ProviderId,
    request_id: u64,
    provider_session_id: String,
    summary: String,
    options: Vec<dpl_chat_runtime::seam::RuntimePermissionOption>,
) {
    let mode = state
        .permission_modes
        .get(&provider)
        .copied()
        .unwrap_or_default();
    let session = state
        .seam_sessions
        .get(&(provider, provider_session_id.clone()))
        .copied();
    let decline = |state: &mut Shared, summary: String| {
        append_seam_event(
            state,
            provider,
            &provider_session_id,
            ChatEventContent::ApprovalRequested { summary },
        );
        append_seam_event(
            state,
            provider,
            &provider_session_id,
            ChatEventContent::ApprovalResolved { accepted: false },
        );
        send_seam_command(
            state,
            provider,
            RuntimeCommand::ResolvePermission {
                request_id,
                option_id: None,
            },
        );
    };
    if mode == PermissionMode::ReadOnly {
        decline(
            state,
            format!("{summary}. Declined: this chat is set to Read only."),
        );
        return;
    }
    let Some(session) = session else {
        // No session to show a card in: declining is the honest answer.
        decline(state, summary);
        return;
    };
    let interaction = InteractionId(state.next_interaction_id);
    state.next_interaction_id = state.next_interaction_id.saturating_add(1);
    let allow_options = options
        .iter()
        .filter(|option| {
            matches!(
                option.kind,
                RuntimePermissionOptionKind::AllowOnce | RuntimePermissionOptionKind::AllowAlways
            )
        })
        .map(|option| option.option_id.clone())
        .collect::<BTreeSet<_>>();
    let choices = options
        .into_iter()
        .map(|option| ProviderPermissionChoice {
            option_id: option.option_id,
            label: option.label,
            kind: match option.kind {
                RuntimePermissionOptionKind::AllowOnce => ProviderPermissionChoiceKind::AllowOnce,
                RuntimePermissionOptionKind::AllowAlways => {
                    ProviderPermissionChoiceKind::AllowAlways
                }
                RuntimePermissionOptionKind::RejectOnce => ProviderPermissionChoiceKind::RejectOnce,
                RuntimePermissionOptionKind::RejectAlways => {
                    ProviderPermissionChoiceKind::RejectAlways
                }
            },
        })
        .collect::<Vec<_>>();
    if let Err(error) = state.window.register_provider_permission(
        interaction,
        provider,
        session,
        request_id,
        summary.clone(),
        choices,
    ) {
        tracing::warn!(%provider, %error, "approval card could not be shown; declining");
        decline(state, summary);
        return;
    }
    state.seam_permissions.insert(
        (provider, request_id),
        PendingSeamPermission {
            interaction,
            provider_session_id: provider_session_id.clone(),
            allow_options,
        },
    );
    append_seam_event(
        state,
        provider,
        &provider_session_id,
        ChatEventContent::ApprovalRequested { summary },
    );
}

/// Retire the cards a session was waiting on when its turn ends badly, so no
/// stale Approve button outlives the turn that asked.
fn cancel_seam_permissions(state: &mut Shared, provider: ProviderId, provider_session_id: &str) {
    let pending = state
        .seam_permissions
        .iter()
        .filter(|((owner, _), pending)| {
            *owner == provider && pending.provider_session_id == provider_session_id
        })
        .map(|((_, request_id), pending)| (*request_id, pending.interaction))
        .collect::<Vec<_>>();
    for (request_id, interaction) in pending {
        let _ = state.window.apply(
            dpl_chat_window::ChatWindowIntent::ResolveProviderPermission {
                interaction,
                provider,
                request_id,
                option_id: None,
            },
        );
        state.seam_permissions.remove(&(provider, request_id));
        append_seam_event(
            state,
            provider,
            provider_session_id,
            ChatEventContent::ApprovalResolved { accepted: false },
        );
    }
}

fn cancel_grok_permissions(
    state: &mut Shared,
    provider_session_id: Option<&str>,
    notify_runtime: bool,
) {
    let pending = state
        .grok_permissions
        .iter()
        .filter_map(|(request_id, pending)| {
            provider_session_id
                .is_none_or(|session| pending.provider_session_id == session)
                .then_some((
                    *request_id,
                    pending.interaction,
                    pending.provider_session_id.clone(),
                ))
        })
        .collect::<Vec<_>>();
    for (request_id, interaction, provider_session_id) in pending {
        let _ = state.window.apply(
            dpl_chat_window::ChatWindowIntent::ResolveProviderPermission {
                interaction,
                provider: dpl_chat_grok::PROVIDER,
                request_id,
                option_id: None,
            },
        );
        state.grok_permissions.remove(&request_id);
        append_grok_event(
            state,
            &provider_session_id,
            ChatEventContent::ApprovalResolved { accepted: false },
        );
        if notify_runtime {
            send_grok_command(
                state,
                dpl_chat_grok::GrokRuntimeCommand::ResolvePermission {
                    request_id,
                    option_id: None,
                },
            );
        }
    }
}

fn next_event_id(state: &mut Shared) -> ChatEventId {
    let id = ChatEventId(state.next_chat_event_id);
    state.next_chat_event_id = state.next_chat_event_id.saturating_add(1);
    id
}

fn next_turn_id(state: &mut Shared) -> TurnId {
    let id = TurnId(format!("grok-turn-{}", state.next_turn_id));
    state.next_turn_id = state.next_turn_id.saturating_add(1);
    id
}

fn send_grok_command(state: &Shared, command: dpl_chat_grok::GrokRuntimeCommand) -> bool {
    let Some(runtime) = state.grok_runtime.as_ref() else {
        tracing::warn!("Grok ACP runtime is not connected");
        return false;
    };
    match runtime.try_send(command) {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(%error, "Grok ACP command was rejected");
            false
        }
    }
}

fn request_utility_record(
    registry: &mut ChatWindowRegistry,
    interaction_id: InteractionId,
    placement: InteractionPlacement,
) -> Option<ChatWindowId> {
    let attachment = attachment_for(placement);
    match registry.request_attached(
        WindowKind::Artifact,
        WindowBinding {
            interaction_id: Some(interaction_id),
            ..WindowBinding::default()
        },
        attachment,
    ) {
        Ok(id) => {
            transition_opening(registry, id);
            Some(id)
        }
        Err(error) => {
            tracing::error!(%error, "Chat utility registration failed");
            None
        }
    }
}

fn attachment_for(placement: InteractionPlacement) -> WindowAttachment {
    match placement {
        InteractionPlacement::DetachedUtility => WindowAttachment::Detached,
        InteractionPlacement::Inline
        | InteractionPlacement::Overlay
        | InteractionPlacement::SnappedUtility => WindowAttachment::Snapped {
            parent: ChatWindowId::PRIMARY,
            edge: SnapEdge::Right,
            offset: [8, 0],
        },
    }
}

fn open_utility(
    ctx: &egui::Context,
    shared: &Arc<Mutex<Shared>>,
    interaction_id: InteractionId,
    placement: InteractionPlacement,
) {
    let attachment = attachment_for(placement);
    let target = {
        let mut state = lock(shared);
        let existing = state
            .registry
            .records()
            .iter()
            .find(|record| record.binding.interaction_id == Some(interaction_id))
            .map(|record| (record.id, record.lifecycle.clone(), record.attachment));
        match existing {
            Some((id, lifecycle, current_attachment)) => {
                if current_attachment != Some(attachment) {
                    if let Err(error) = state.registry.set_attachment(id, attachment) {
                        tracing::error!(%error, "Chat utility attachment failed");
                        return;
                    }
                    transition_opening(&mut state.registry, id);
                } else if matches!(
                    lifecycle,
                    WindowLifecycle::Closed | WindowLifecycle::Failed { .. }
                ) {
                    if let Err(error) = state.registry.reopen(id) {
                        tracing::error!(%error, "Chat utility reopen failed");
                        return;
                    }
                    transition_opening(&mut state.registry, id);
                }
                state
                    .registry
                    .record(id)
                    .map(|record| (id, record.generation))
            }
            None => request_utility_record(&mut state.registry, interaction_id, placement)
                .and_then(|id| {
                    state
                        .registry
                        .record(id)
                        .map(|record| (id, record.generation))
                }),
        }
    };
    if let Some((id, generation)) = target {
        let viewport = utility_viewport_id(id, generation);
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

fn show_utility(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>, record: ChatWindowRecord) {
    let Some(interaction_id) = record.binding.interaction_id else {
        tracing::error!(window = %record.id, "Chat utility has no interaction binding");
        return;
    };
    let spec = {
        let state = lock(shared);
        state.window.interaction(interaction_id).cloned()
    };
    let Some(spec) = spec else {
        tracing::error!(window = %record.id, %interaction_id, "Chat utility interaction is missing");
        return;
    };
    let title = match &spec.content {
        InteractionContent::ArtifactPreview { title, .. }
        | InteractionContent::Notice { title, .. }
        | InteractionContent::Approval { title, .. } => title.clone(),
        InteractionContent::RouteResult { route_label, .. } => route_label.clone(),
    };
    let utility_id = utility_viewport_id(record.id, record.generation);
    let (viewport_monitor, native_pixels_per_point) = ctx.input(|input| {
        (
            input.viewport().monitor_size.map(|size| [size.x, size.y]),
            input.viewport().native_pixels_per_point,
        )
    });
    let pixels_per_point = native_pixels_per_point.unwrap_or_else(|| ctx.pixels_per_point());
    let capture_monitor = lock(shared)
        .primary_monitor
        .map(|([width, height], scale_factor)| {
            [
                width as f32 / pixels_per_point.max(scale_factor),
                height as f32 / pixels_per_point.max(scale_factor),
            ]
        });
    let monitor_size = match (viewport_monitor, capture_monitor) {
        (Some(viewport), Some(capture)) => {
            Some([viewport[0].min(capture[0]), viewport[1].min(capture[1])])
        }
        (Some(viewport), None) => Some(viewport),
        (None, Some(capture)) => Some(capture),
        (None, None) => None,
    };
    let mut builder = utility_builder(shared, &record, &title, monitor_size);
    if let Some(position) = record.geometry.outer_position {
        if matches!(record.attachment, Some(WindowAttachment::Detached)) {
            builder = builder.with_position([position[0] as f32, position[1] as f32]);
        }
    }
    let pilot_key = PilotViewportKey::new(format!(
        "chat.utility.{}.{}",
        record.id.0, record.generation.0
    ));
    let shared = Arc::clone(shared);
    ctx.pilot_show_viewport_immediate(pilot_key, utility_id, builder, move |ui, class| {
        if class == egui::ViewportClass::EmbeddedWindow {
            ui.colored_label(
                dpl_coldlight::Tone::Coral.color(),
                "This utility requires a native viewport.",
            );
            let mut state = lock(&shared);
            if current_generation(&state.registry, record.id) == Some(record.generation) {
                if let Err(error) = state
                    .registry
                    .mark_failed(record.id, "backend embedded Chat utility viewport")
                {
                    tracing::error!(%error, "Chat utility failure could not be recorded");
                }
            }
            return;
        }

        let viewport = ui.ctx().input(|input| input.viewport().clone());
        let close_requested = viewport.close_requested();
        let geometry = viewport_geometry(&viewport);
        {
            let mut state = lock(&shared);
            if current_generation(&state.registry, record.id) == Some(record.generation) {
                if matches!(
                    state
                        .registry
                        .record(record.id)
                        .map(|current| &current.lifecycle),
                    Some(WindowLifecycle::Opening)
                ) {
                    if let Err(error) = state.registry.mark_visible(record.id) {
                        tracing::error!(%error, "Chat utility failed to become visible");
                    }
                }
                if let Some((position, size)) = geometry {
                    if let Err(error) = state.registry.update_geometry(record.id, position, size) {
                        tracing::warn!(%error, "Chat utility geometry was rejected");
                    }
                }
            }
        }

        let action = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(dpl_coldlight::surface::WELL)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| utility_ui(ui, &record, &spec))
            .inner;
        if let Some(action) = action {
            apply_utility_action(ui.ctx(), &shared, &record, action);
        }
        if close_requested
            && current_generation(&lock(&shared).registry, record.id) == Some(record.generation)
        {
            close_utility(&shared, record.id, record.generation);
        }
        #[cfg(feature = "eguipilotai-devtools")]
        {
            pump_pilot_runtime(&shared);
            consume_pilot_close_requests(ui.ctx(), &shared);
        }
    });
}

fn declare_utilities(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>) {
    let utilities = {
        let state = lock(shared);
        state
            .registry
            .records()
            .iter()
            .filter(|record| {
                record.id != ChatWindowId::PRIMARY
                    && matches!(
                        record.lifecycle,
                        WindowLifecycle::Opening | WindowLifecycle::Visible
                    )
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    for record in utilities {
        let utility_id = utility_viewport_id(record.id, record.generation);
        if ctx.pilot_take_viewport_close_request(utility_id) {
            close_utility(shared, record.id, record.generation);
            ctx.pilot_retire_viewport(utility_id);
        } else {
            show_utility(ctx, shared, record);
        }
    }
}

#[cfg(feature = "eguipilotai-devtools")]
fn consume_pilot_close_requests(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>) {
    let primary_generation = {
        let state = lock(shared);
        current_generation(&state.registry, ChatWindowId::PRIMARY)
    };
    if let Some(generation) = primary_generation {
        if ctx.pilot_take_viewport_close_request(primary_viewport_id(generation)) {
            close_chat(ctx, shared, generation);
            return;
        }
    }
    let utilities = {
        let state = lock(shared);
        state
            .registry
            .records()
            .iter()
            .filter(|record| record.id != ChatWindowId::PRIMARY)
            .map(|record| (record.id, record.generation))
            .collect::<Vec<_>>()
    };
    for (id, generation) in utilities {
        let utility_id = utility_viewport_id(id, generation);
        if ctx.pilot_take_viewport_close_request(utility_id) {
            close_utility(shared, id, generation);
            ctx.pilot_retire_viewport(utility_id);
        }
    }
}

#[cfg(feature = "eguipilotai-devtools")]
fn pump_pilot_runtime(shared: &Arc<Mutex<Shared>>) {
    let runtime = lock(shared).pilot_runtime.clone();
    if let Some(runtime) = runtime {
        lock(&runtime).pump();
    }
}

fn utility_builder(
    shared: &Arc<Mutex<Shared>>,
    record: &ChatWindowRecord,
    title: &str,
    monitor_size: Option<[f32; 2]>,
) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default()
        .with_title(format!("DevPULSE Chat — {title}"))
        .with_inner_size([
            record.geometry.inner_size[0] as f32,
            record.geometry.inner_size[1] as f32,
        ])
        .with_min_inner_size([
            WindowGeometry::UTILITY_MINIMUM[0] as f32,
            WindowGeometry::UTILITY_MINIMUM[1] as f32,
        ])
        .with_resizable(true);
    match record.attachment {
        Some(WindowAttachment::Snapped {
            parent,
            edge,
            offset,
        }) => {
            let parent_geometry = lock(shared)
                .registry
                .record(parent)
                .map(|record| record.geometry)
                .unwrap_or(WindowGeometry::DEFAULT);
            let parent_position = parent_geometry.outer_position.unwrap_or([48, 48]);
            let preferred_x = match edge {
                SnapEdge::Left => {
                    parent_position[0] - record.geometry.inner_size[0] as i32 + offset[0]
                }
                SnapEdge::Right => {
                    parent_position[0] + parent_geometry.inner_size[0] as i32 + offset[0]
                }
            };
            let fallback_x = match edge {
                SnapEdge::Left => {
                    parent_position[0] + parent_geometry.inner_size[0] as i32 - offset[0]
                }
                SnapEdge::Right => {
                    parent_position[0] - record.geometry.inner_size[0] as i32 - offset[0]
                }
            };
            let mut x = preferred_x;
            let mut y = parent_position[1] + offset[1];
            if let Some([monitor_width, monitor_height]) = monitor_size {
                let utility_width = record.geometry.inner_size[0] as i32;
                let utility_height = record.geometry.inner_size[1] as i32;
                let max_x = (monitor_width as i32 - utility_width - 8).max(8);
                let max_y = (monitor_height as i32 - utility_height - 8).max(8);
                if x < 8 || x > max_x {
                    x = fallback_x;
                }
                x = x.clamp(8, max_x);
                y = y.clamp(8, max_y);
            }
            builder = builder
                .with_position([x as f32, y as f32])
                .with_decorations(false)
                .with_taskbar(false);
        }
        Some(WindowAttachment::Detached) | None => {
            builder = builder.with_decorations(true).with_taskbar(true);
        }
    }
    builder
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UtilityAction {
    TogglePin,
    Detach,
    Redock,
    FocusChat,
    Close,
}

fn utility_ui(
    ui: &mut egui::Ui,
    record: &ChatWindowRecord,
    spec: &dpl_chat_interactions::InteractionSpec,
) -> Option<UtilityAction> {
    let mut action = None;
    let root = ui.max_rect();
    ui.painter().rect_filled(
        root,
        egui::CornerRadius::ZERO,
        egui::Color32::from_rgb(4, 15, 18),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(root.shrink(10.0))
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("DEV PULSE / UTILITY")
                        .monospace()
                        .small()
                        .color(dpl_coldlight::Tone::Cyan.color()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if utility_button(ui, "chat.utility.close", "×", "Close utility").clicked() {
                        action = Some(UtilityAction::Close);
                    }
                    if utility_button(ui, "chat.utility.focus-origin", "↙", "Focus Chat").clicked()
                    {
                        action = Some(UtilityAction::FocusChat);
                    }
                    match record.attachment {
                        Some(WindowAttachment::Snapped { .. }) => {
                            if utility_button(ui, "chat.utility.detach", "□", "Detach utility")
                                .clicked()
                            {
                                action = Some(UtilityAction::Detach);
                            }
                        }
                        Some(WindowAttachment::Detached) | None => {
                            if utility_button(ui, "chat.utility.redock", "▥", "Re-dock utility")
                                .clicked()
                            {
                                action = Some(UtilityAction::Redock);
                            }
                        }
                    }
                    if utility_button(
                        ui,
                        "chat.utility.pin",
                        if record.pinned { "●" } else { "○" },
                        "Pin utility",
                    )
                    .clicked()
                    {
                        action = Some(UtilityAction::TogglePin);
                    }
                });
            });
            ui.separator();
            match &spec.content {
                InteractionContent::ArtifactPreview {
                    title,
                    artifact_kind,
                    summary,
                } => {
                    ui.label(
                        egui::RichText::new(artifact_kind)
                            .monospace()
                            .small()
                            .color(dpl_coldlight::Tone::Amber.color()),
                    );
                    ui.label(
                        egui::RichText::new(title)
                            .strong()
                            .size(22.0)
                            .color(dpl_coldlight::ink::STRONG),
                    );
                    ui.label(egui::RichText::new(summary).color(dpl_coldlight::ink::MUTED));
                    ui.add_space(14.0);
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(8, 27, 30))
                        .stroke(egui::Stroke::new(1.0, dpl_coldlight::Tone::Cyan.color()))
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Library / Slide templates")
                                    .strong()
                                    .color(dpl_coldlight::ink::STRONG),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Observed route preview · context remains unattached",
                                )
                                .small()
                                .color(dpl_coldlight::ink::MUTED),
                            );
                        });
                }
                InteractionContent::RouteResult {
                    route_label,
                    detail,
                    ..
                } => {
                    ui.heading(route_label);
                    ui.label(detail);
                }
                InteractionContent::Notice { title, body } => {
                    ui.heading(title);
                    ui.label(body);
                }
                InteractionContent::Approval { title, summary, .. } => {
                    ui.heading(title);
                    ui.label(summary);
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} · generation {} · session preserved",
                        record.id, record.generation.0
                    ))
                    .monospace()
                    .small()
                    .color(dpl_coldlight::ink::SUBTLE),
                );
            });
        },
    );
    action
}

fn utility_button(
    ui: &mut egui::Ui,
    id: &'static str,
    label: &str,
    description: &str,
) -> egui::Response {
    let response = ui.small_button(label).on_hover_text(description);
    ui.pilot_track(
        &response,
        NodeMetadata::new(PilotId::new(id), Role::Button, description),
    );
    response
}

fn apply_utility_action(
    ctx: &egui::Context,
    shared: &Arc<Mutex<Shared>>,
    record: &ChatWindowRecord,
    action: UtilityAction,
) {
    match action {
        UtilityAction::TogglePin => {
            if let Err(error) = lock(shared).registry.toggle_pinned(record.id) {
                tracing::warn!(%error, "Chat utility pin failed");
            }
        }
        UtilityAction::Detach | UtilityAction::Redock => {
            let attachment = if action == UtilityAction::Detach {
                WindowAttachment::Detached
            } else {
                WindowAttachment::Snapped {
                    parent: ChatWindowId::PRIMARY,
                    edge: SnapEdge::Right,
                    offset: [8, 0],
                }
            };
            let mut state = lock(shared);
            if current_generation(&state.registry, record.id) != Some(record.generation) {
                return;
            }
            match state.registry.set_attachment(record.id, attachment) {
                Ok(true) => {
                    transition_opening(&mut state.registry, record.id);
                }
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "Chat utility move failed"),
            }
            drop(state);
            ctx.pilot_retire_viewport(utility_viewport_id(record.id, record.generation));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            if let Some(chat_viewport_id) = live_primary_viewport_id(shared) {
                ctx.request_repaint_of(chat_viewport_id);
            }
        }
        UtilityAction::FocusChat => {
            if let Some(chat_viewport_id) = live_primary_viewport_id(shared) {
                ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Focus);
            }
        }
        UtilityAction::Close => {
            close_utility(shared, record.id, record.generation);
            ctx.pilot_retire_viewport(utility_viewport_id(record.id, record.generation));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            if let Some(chat_viewport_id) = live_primary_viewport_id(shared) {
                ctx.request_repaint_of(chat_viewport_id);
            }
        }
    }
}

fn close_utility(shared: &Arc<Mutex<Shared>>, id: ChatWindowId, generation: ViewportGeneration) {
    let mut state = lock(shared);
    if current_generation(&state.registry, id) != Some(generation) {
        return;
    }
    if let Err(error) = state.registry.request_close(id) {
        tracing::warn!(%error, "Chat utility failed to enter closing state");
        return;
    }
    if let Err(error) = state.registry.mark_closed(id) {
        tracing::warn!(%error, "Chat utility failed to close");
    }
}

fn current_generation(
    registry: &ChatWindowRegistry,
    id: ChatWindowId,
) -> Option<ViewportGeneration> {
    registry.record(id).map(|record| record.generation)
}

fn live_primary_viewport_id(shared: &Arc<Mutex<Shared>>) -> Option<egui::ViewportId> {
    let state = lock(shared);
    state
        .registry
        .record(ChatWindowId::PRIMARY)
        .filter(|record| {
            matches!(
                record.lifecycle,
                WindowLifecycle::Opening | WindowLifecycle::Visible
            )
        })
        .map(|record| primary_viewport_id(record.generation))
}

fn utility_viewport_id(id: ChatWindowId, generation: ViewportGeneration) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("devpulse.chat.utility", id.0, generation.0))
}

fn close_chat(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>, generation: ViewportGeneration) {
    let decision = {
        let mut state = lock(shared);
        let Some(record) = state.registry.record(ChatWindowId::PRIMARY) else {
            return;
        };
        if record.generation != generation {
            return;
        }
        let lifecycle = record.lifecycle.clone();
        if !project::can_close(&mut state, ctx) {
            state.project.pending_chat_close = Some(generation);
            ctx.send_viewport_cmd_to(
                primary_viewport_id(generation),
                egui::ViewportCommand::CancelClose,
            );
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        state.project.pending_chat_close = None;
        ctx.send_viewport_cmd_to(
            primary_viewport_id(generation),
            egui::ViewportCommand::Close,
        );
        if !matches!(
            lifecycle,
            WindowLifecycle::Closing | WindowLifecycle::Closed
        ) {
            if let Err(error) = state.registry.request_close(ChatWindowId::PRIMARY) {
                tracing::error!(%error, "Chat viewport failed to enter closing state");
                return;
            }
        }
        if !matches!(lifecycle, WindowLifecycle::Closed) {
            if let Err(error) = state.registry.mark_closed(ChatWindowId::PRIMARY) {
                tracing::error!(%error, "Chat viewport failed to close");
                return;
            }
        }
        state.lifetime.on_chat_close()
    };
    ctx.pilot_retire_viewport(primary_viewport_id(generation));
    if decision == ChatCloseDecision::Exit {
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
    } else {
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }
}

fn embedded_failure(ui: &mut egui::Ui, shared: &Arc<Mutex<Shared>>) {
    ui.colored_label(
        dpl_coldlight::Tone::Coral.color(),
        "Chat requires native multi-viewport support; this backend embedded the peer window.",
    );
    let mut state = lock(shared);
    if let Err(error) = state.registry.mark_failed(
        ChatWindowId::PRIMARY,
        "backend embedded the primary Chat viewport",
    ) {
        tracing::error!(%error, "Chat embedded-viewport failure could not be recorded");
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod plain_failure_tests {
    use super::*;

    /// The failure the operator hit on 2026-10-19: raw JSON in the chat.
    #[test]
    fn an_outdated_codex_reads_as_an_update_with_the_button() {
        let plain = plain_failure(
            r#"{"type":"error","status":400,"error":{"type":"invalid_request_error","message":"The 'gpt-6-astra' model requires a newer version of Codex. Please upgrade to the latest app or CLI and try again."}}"#,
        );
        assert!(!plain.message.contains('{'), "no raw JSON reaches the chat");
        assert!(plain.message.starts_with("Codex needs an update"));
        assert!(matches!(
            plain.activation,
            Some(ActivationState::VersionUnsupported { recovery, .. })
                if recovery.first().map(|action| action.kind)
                    == Some(RecoveryActionKind::UpdateProvider)
        ));
    }

    #[test]
    fn an_expired_sign_in_moves_the_provider_to_sign_in() {
        let plain =
            plain_failure("unexpected status 401 Unauthorized: workspace routing discovery");
        assert!(matches!(
            plain.activation,
            Some(ActivationState::SignedOut { .. })
        ));
    }

    #[test]
    fn anything_else_keeps_the_providers_own_words() {
        let plain = plain_failure("Codex refused turn/start: thread not found");
        assert_eq!(plain.message, "Codex refused turn/start: thread not found");
        assert_eq!(plain.activation, None);
    }
}

#[cfg(test)]
mod roster_tests {
    use super::*;

    fn estate_at(tag: &str) -> dpl_estate::Estate {
        let dir = std::env::temp_dir().join(format!("dpl-roster-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dpl_estate::Estate::open(&dir).expect("estate open")
    }

    fn claude() -> ProviderCapability {
        ProviderCapability {
            id: ProviderId("claude"),
            label: "Claude Code".into(),
            activated: true,
            note: Some("local transcripts only".into()),
            summary: String::new(),
            since: "2021-01-28".into(),
            presence: dpl_fabric::ProviderPresence::Ready,
            capabilities: BTreeSet::from([Capability::ReadSessions, Capability::PageHistory]),
        }
    }

    #[test]
    fn the_roster_survives_a_restart() {
        let estate = estate_at("restart");
        let roster = vec![claude()];
        store_roster(Some(&estate), &roster);
        estate.persist().expect("estate persist");

        // Reopened as a NEW launch would: the ids this build knows are what
        // the stored names resolve against.
        let restored = restore_roster(Some(&estate), &[ProviderId("claude")]);
        assert_eq!(
            restored, roster,
            "what was activated last launch is known before any probe runs"
        );
        assert!(
            restored[0].can(Capability::PageHistory),
            "the capability SET round-trips, not just the fact of activation"
        );
    }

    #[test]
    fn a_provider_this_build_no_longer_knows_is_dropped_not_resurrected() {
        let estate = estate_at("unknown-provider");
        store_roster(
            Some(&estate),
            &[
                claude(),
                ProviderCapability {
                    id: ProviderId("retired"),
                    label: "Retired".into(),
                    ..claude()
                },
            ],
        );
        estate.persist().expect("estate persist");
        let restored = restore_roster(Some(&estate), &[ProviderId("claude")]);
        assert_eq!(
            restored.len(),
            1,
            "only what this build registers comes back"
        );
        assert_eq!(restored[0].id, ProviderId("claude"));
    }

    #[test]
    fn an_unknown_stored_capability_is_dropped_not_invented() {
        let estate = estate_at("unknown-capability");
        let stored = StoredRoster {
            written_at_ms: 1,
            providers: vec![StoredProviderCapability {
                id: "claude".into(),
                label: "Claude Code".into(),
                activated: true,
                note: None,
                capabilities: vec!["page-history".into(), "teleportation".into()],
                since: None,
            }],
        };
        estate
            .put_json(dpl_estate::Tree::Agents, ROSTER_KEY, &stored)
            .expect("write");
        estate.persist().expect("estate persist");
        let restored = restore_roster(Some(&estate), &[ProviderId("claude")]);
        assert_eq!(
            restored[0].capabilities,
            BTreeSet::from([Capability::PageHistory]),
            "a capability this build does not have is refused by name"
        );
    }

    #[test]
    fn no_estate_is_not_an_error() {
        // The roster is derived; a failed estate open costs a remembered
        // start, never a launch.
        store_roster(None, &[claude()]);
        assert!(restore_roster(None, &[ProviderId("claude")]).is_empty());
    }
}

#[cfg(test)]
mod clyffy_tests {
    use super::*;

    fn event(provider: &'static str, id: u64) -> dpl_chat_core::ChatEvent {
        dpl_chat_core::ChatEvent {
            id: dpl_chat_core::ChatEventId(id),
            provider_id: ProviderId(provider),
            session_id: ChatSessionId(1),
            turn_id: None,
            content: dpl_chat_core::ChatEventContent::UserMessage(format!("m{id}")),
        }
    }

    /// V10: a backing swap is a re-binding — the session id is
    /// unchanged, every prior event survives in order, and each event
    /// keeps the provider tag of whoever produced it.
    #[test]
    fn swapping_clyffys_backing_preserves_the_transcript() {
        let mut session = ClyffySession {
            id: ChatSessionId(42),
            backing: Some(ProviderId("codex")),
            events: vec![event("codex", 1), event("codex", 2)],
        };
        let id = session.rebind(ProviderId("claude"));
        session.events.push(event("claude", 3));
        assert_eq!(id, ChatSessionId(42), "the identity never changes");
        assert_eq!(session.id, ChatSessionId(42));
        assert_eq!(
            session
                .events
                .iter()
                .map(|e| (e.provider_id.0, e.id.0))
                .collect::<Vec<_>>(),
            [("codex", 1), ("codex", 2), ("claude", 3)],
            "history keeps its per-event provider tags, in order"
        );
    }

    /// V9: Clyffy never claims readiness it borrowed from nobody —
    /// active iff the CURRENT binding is among the activated providers.
    #[test]
    fn clyffy_is_inert_without_a_signed_in_backing() {
        let mut session = ClyffySession {
            id: ChatSessionId(7),
            backing: None,
            events: Vec::new(),
        };
        assert!(!session.is_active(&[]), "no backing, no presence — inert");
        assert!(
            !session.is_active(&[ProviderId("codex")]),
            "presence without a binding is not activation"
        );
        session.rebind(ProviderId("codex"));
        assert!(session.is_active(&[ProviderId("codex")]));
        assert!(
            !session.is_active(&[ProviderId("claude")]),
            "the binding signed out — Clyffy waits, honestly"
        );
    }
}

#[cfg(test)]
mod slash_mint_tests {
    use super::*;

    #[test]
    fn synthetic_strict_refuses_and_leaves_behavioral() {
        let gate = dpl_intent::DeterminismGate::default();
        // Policy only — no Shared. The refuse fact is what SSE/feed would
        // carry when an agent injects Send on `/strict`.
        let ev = resolve_operator_slash(&gate, OperatorSlashCommand::Strict, true)
            .expect("synthetic /strict always publishes a refusal");
        assert!(matches!(
            &ev,
            IntentEvent::IntentRefused { action, reason }
                if action == "strict" && reason == "synthetic-input"
        ));
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_intent();
        fabric.publish_intent(ev);
        let stamped = rx.try_recv().expect("refusal reached the intent domain");
        assert!(matches!(
            &stamped.ev,
            IntentEvent::IntentRefused { reason, .. } if reason == "synthetic-input"
        ));
        assert_eq!(gate.mode(), dpl_intent::DeterminismMode::Behavioral);
    }

    #[test]
    fn real_strict_escalates() {
        let gate = dpl_intent::DeterminismGate::default();
        let ev = resolve_operator_slash(&gate, OperatorSlashCommand::Strict, false)
            .expect("operator /strict publishes a mode change");
        assert!(matches!(
            ev,
            IntentEvent::DeterminismChanged {
                mode: dpl_intent::DeterminismMode::Strict,
                cause: Some(dpl_intent::StrictCause::Operator(
                    dpl_intent::IntentChannel::SlashCommand
                )),
            }
        ));
        assert_eq!(gate.mode(), dpl_intent::DeterminismMode::Strict);
    }

    #[test]
    fn evidence_locked_behavioral_refuses() {
        let gate = dpl_intent::DeterminismGate::default();
        gate.force_evidence();
        let ev = resolve_operator_slash(&gate, OperatorSlashCommand::Behavioral, false)
            .expect("evidence-locked /behavioral publishes a refusal");
        assert!(matches!(
            &ev,
            IntentEvent::IntentRefused { action, reason }
                if action == "behavioral" && reason == "evidence-locked"
        ));
        assert_eq!(gate.mode(), dpl_intent::DeterminismMode::Strict);
    }

    #[test]
    fn already_behavioral_is_silent() {
        let gate = dpl_intent::DeterminismGate::default();
        assert!(resolve_operator_slash(&gate, OperatorSlashCommand::Behavioral, false).is_none());
    }
}

#[cfg(test)]
mod room_library_tests {
    use super::*;

    #[test]
    fn the_operators_files_join_the_library_and_a_broken_one_is_named() {
        let root = std::env::temp_dir().join(format!("dpl-room-library-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roles = root.join("roles");
        std::fs::create_dir_all(&roles).expect("roles folder");
        std::fs::create_dir_all(root.join("rooms")).expect("rooms folder");

        let builtin = dpl_chat_core::rooms::Library::builtin();
        let mut skeptic = builtin.role("skeptic").expect("built-in skeptic").clone();
        skeptic.version = "1.1.0".to_owned();
        skeptic.summary = "The operator's own skeptic.".to_owned();
        std::fs::write(
            roles.join("skeptic.json"),
            serde_json::to_string_pretty(&skeptic).expect("serialize"),
        )
        .expect("write skeptic");
        std::fs::write(roles.join("broken.json"), "{").expect("write broken");
        std::fs::write(roles.join("notes.txt"), "not a role").expect("write notes");

        let library = load_room_library(&root);
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(
            library.role("skeptic").map(|role| role.summary.as_str()),
            Some("The operator's own skeptic.")
        );
        assert_eq!(library.roles.len(), builtin.roles.len());
        let rejected: Vec<&str> = library
            .rejected
            .iter()
            .map(|(file, _)| file.as_str())
            .collect();
        assert_eq!(rejected, ["roles/broken.json"]);
    }

    #[test]
    fn no_library_folder_means_the_builtins_alone() {
        let root =
            std::env::temp_dir().join(format!("dpl-room-library-none-{}", std::process::id()));
        let library = load_room_library(&root);
        let builtin = dpl_chat_core::rooms::Library::builtin();
        assert_eq!(library.roles.len(), builtin.roles.len());
        assert_eq!(library.rooms.len(), builtin.rooms.len());
        assert!(library.rejected.is_empty());
    }
}
