//! The signal fabric — typed enums on tokio channels, end to end.
//!
//! This is the whole point of Path B: there is no boundary to type, no
//! bindings to generate, no mirror to drift. A signal is a Rust enum; rustc
//! enforces every producer and consumer. Per-domain enums on per-domain
//! channels (a flat global enum would wake every consumer on every message).
//!
//! P0 ships the shell domain as the working exemplar. Engine domains
//! (capture, jobs, providers) land with their ports and follow this file's
//! shape exactly.

pub use dpl_chat_core::ProviderId;
use dpl_hub::Route;
use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;

/// Stable identity for an application-owned native window.
///
/// Shell accessories publish this identity without knowing which feature owns
/// the window. The application coordinator resolves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppWindowKey(pub &'static str);

impl fmt::Display for AppWindowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Small, typed window requests suitable for the broadcast signal fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppWindowAction {
    /// Create the window when closed; otherwise restore and focus it.
    OpenOrFocus,
}

/// Shell-domain signals. Copy — signals are values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellEvent {
    /// Navigation happened. Consumers: future chat context providers,
    /// telemetry, anything that cares where the operator is.
    RouteChanged { route: Route },
    /// The footer Snip launcher was pressed — summon the ViziWiz overlay.
    /// Consumer: the viziwiz crate (B4); until then the inspector's signal
    /// count is the visible proof the launcher fires.
    ViziwizSummon,
    /// A hub asks the shell to enter/leave full-bleed (D2: the Present
    /// hub's deck owns the viewport while a show runs). Consumer: the
    /// shell, which drains its receiver each frame.
    FullBleedRequest(bool),
    /// A hub surface asks to GO somewhere (the NAVIGATE seam): tapped
    /// subroute cards publish this; the shell drains and navigates.
    /// `RouteChanged` remains the truth of ARRIVAL — this is the ask.
    NavigateRequest { route: Route },
    /// A generic shell accessory asks the application owner to open, restore,
    /// and focus a native peer window.
    WindowRequest {
        target: AppWindowKey,
        action: AppWindowAction,
    },
}

/// A shell-domain subscription — consumers hold this without needing a
/// tokio dependency of their own.
pub type ShellReceiver = broadcast::Receiver<ShellEvent>;

/// Deck-domain signals (D4): the show machine's position, published on
/// every real move. Indices only — consumers that need names/scripts
/// resolve them from the episode themselves (the event stays Copy and
/// process-portable). Consumer: the presenter teleprompter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckEvent {
    /// The deck landed on `slides[index]` at `step` (0 = setup);
    /// `slides`/`steps` size the show and the current slide.
    Beat {
        index: usize,
        step: usize,
        slides: usize,
        steps: usize,
    },
}

/// A deck-domain subscription.
pub type DeckReceiver = broadcast::Receiver<DeckEvent>;

/// Capture-domain signals (the signal doctrine, docs/design/
/// signal-doctrine.md) — the original app's taxonomy, our index's
/// field vocabulary. Clone (not Copy — Saved carries owned strings);
/// signals are still VALUES.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureEvent {
    /// A capture vehicle began (mode: "full"|"window"|"region"|"free"|
    /// "steps"|"session"|"video"|"render"|"export"|"import"|"ingest").
    Started { mode: String, target: String },
    /// An artifact LANDED and is indexed — fields mirror
    /// `dpl_index::Record`. Fires AFTER the write succeeds.
    Saved {
        rel_path: String,
        kind: String,
        name: String,
        location: String,
        source: String,
    },
    /// A mutation failed where it failed.
    Failed { error: String },
    /// The inbox changed shape (dump, keep, rescan, batch end) —
    /// consumers reload lazily.
    InboxChanged,
    /// The ViziWiz palette showed/hid.
    PaletteVisibility { visible: bool },
    /// The steps/session recorder changed state — `on` = recording,
    /// `count` = steps captured so far. A FACT (latest-wins), stamped
    /// like every capture fact so the activity feed, status counter and
    /// the seam's `/signals` see a recording live. The COMMAND that
    /// toggles recording stays a DIRECT CALL (single process — a
    /// broadcast would be weaker than the call itself), the faithful
    /// egui translation of PORT's cross-window `dpl:steps-state` /
    /// `dpl:session-state`.
    Recording { on: bool, count: u32 },
    /// A click/action did NOT become a step, and why — so the pill reads
    /// `skipped · nothing changed`, never a silently frozen counter.
    /// Honesty is a first-class fact (PORT parity: `dpl:steps-skip`).
    Skipped { reason: String },
}

/// A capture-domain subscription.
pub type CaptureReceiver = broadcast::Receiver<Stamped<CaptureEvent>>;

/// Job lifecycle signals (doctrine law 7: PROGRESS stays in shared
/// polled state; these fire at TRANSITIONS only). Job COMMANDS are a
/// reserved mpsc contract (R2c) — status may be lossy, commands never.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    Queued { job: JobId, label: String },
    Done { job: JobId, out: String },
    Failed { job: JobId, error: String },
}

/// A process-unique job identity (minted from the fabric's sequence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub u64);

/// A jobs-domain subscription.
pub type JobReceiver = broadcast::Receiver<Stamped<JobEvent>>;

/// One job's live row on THE JOBBOARD (doctrine law 7: PROGRESS is shared
/// polled state — this row is that state, centralized; signals stay
/// transition-only). `progress` is a latest-wins unit counter the worker
/// bumps lock-free (files copied, frames done); 0 until reported.
#[derive(Debug, Clone)]
pub struct JobRow {
    pub job: JobId,
    pub label: String,
    pub state: JobState,
    pub progress: Arc<AtomicUsize>,
}

impl JobRow {
    pub fn running(&self) -> bool {
        matches!(self.state, JobState::Running)
    }
}

/// A job's lifecycle position, folded from the jobs stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Running,
    Done { out: String },
    Failed { error: String },
}

/// Resolved rows kept for unacking consumers before eviction (oldest first).
const RESOLVED_CAP: usize = 64;

/// Process-unique identity for one provider activation ceremony.
///
/// It shares the Fabric sequence space so an activation can be ordered
/// against every other application fact without inventing another clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActivationAttemptId(pub u64);

impl fmt::Display for ActivationAttemptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "activation-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderActivationStage {
    Launch,
    DeviceCode,
    Browser,
    Authorization,
    Runtime,
}

impl ProviderActivationStage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::DeviceCode => "device-code",
            Self::Browser => "browser",
            Self::Authorization => "authorization",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderFailureClass {
    UnsupportedVersion,
    Launch,
    InvalidPrompt,
    Browser,
    Authorization,
    Runtime,
    Protocol,
    Timeout,
    Disconnect,
    Cancelled,
}

impl ProviderFailureClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "unsupported-version",
            Self::Launch => "launch",
            Self::InvalidPrompt => "invalid-prompt",
            Self::Browser => "browser",
            Self::Authorization => "authorization",
            Self::Runtime => "runtime",
            Self::Protocol => "protocol",
            Self::Timeout => "timeout",
            Self::Disconnect => "disconnect",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Redacted provider lifecycle facts.
///
/// Codes, authorization URLs, tokens, cookies, raw provider output, and
/// message content are deliberately impossible to place in this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderEvent {
    Selected {
        provider: ProviderId,
    },
    ProfileConfigured {
        provider: ProviderId,
        profile_key: String,
    },
    ActivationRequested {
        provider: ProviderId,
        attempt: ActivationAttemptId,
    },
    ClientStarted {
        provider: ProviderId,
        attempt: ActivationAttemptId,
        client_version: String,
    },
    DeviceCodeReceived {
        provider: ProviderId,
        attempt: ActivationAttemptId,
        code_len: u16,
    },
    CodeCopied {
        provider: ProviderId,
        attempt: ActivationAttemptId,
    },
    SystemBrowserDispatched {
        provider: ProviderId,
        attempt: ActivationAttemptId,
    },
    AuthorizationCompleted {
        provider: ProviderId,
        attempt: ActivationAttemptId,
    },
    RuntimeStarted {
        provider: ProviderId,
        attempt: ActivationAttemptId,
    },
    VerificationRetryScheduled {
        provider: ProviderId,
        attempt: ActivationAttemptId,
        retry_index: u8,
        max_retries: u8,
        delay_ms: u16,
    },
    Ready {
        provider: ProviderId,
        attempt: ActivationAttemptId,
        client_version: String,
        auth_method: &'static str,
    },
    Failed {
        provider: ProviderId,
        attempt: ActivationAttemptId,
        stage: ProviderActivationStage,
        class: ProviderFailureClass,
        retryable: bool,
    },
    Cancelled {
        provider: ProviderId,
        attempt: ActivationAttemptId,
    },
    Expired {
        provider: ProviderId,
        attempt: ActivationAttemptId,
    },
}

impl ProviderEvent {
    pub const fn provider(&self) -> ProviderId {
        match self {
            Self::Selected { provider }
            | Self::ProfileConfigured { provider, .. }
            | Self::ActivationRequested { provider, .. }
            | Self::ClientStarted { provider, .. }
            | Self::DeviceCodeReceived { provider, .. }
            | Self::CodeCopied { provider, .. }
            | Self::SystemBrowserDispatched { provider, .. }
            | Self::AuthorizationCompleted { provider, .. }
            | Self::RuntimeStarted { provider, .. }
            | Self::VerificationRetryScheduled { provider, .. }
            | Self::Ready { provider, .. }
            | Self::Failed { provider, .. }
            | Self::Cancelled { provider, .. }
            | Self::Expired { provider, .. } => *provider,
        }
    }

    pub const fn attempt(&self) -> Option<ActivationAttemptId> {
        match self {
            Self::Selected { .. } | Self::ProfileConfigured { .. } => None,
            Self::ActivationRequested { attempt, .. }
            | Self::ClientStarted { attempt, .. }
            | Self::DeviceCodeReceived { attempt, .. }
            | Self::CodeCopied { attempt, .. }
            | Self::SystemBrowserDispatched { attempt, .. }
            | Self::AuthorizationCompleted { attempt, .. }
            | Self::RuntimeStarted { attempt, .. }
            | Self::VerificationRetryScheduled { attempt, .. }
            | Self::Ready { attempt, .. }
            | Self::Failed { attempt, .. }
            | Self::Cancelled { attempt, .. }
            | Self::Expired { attempt, .. } => Some(*attempt),
        }
    }
}

pub type ProviderReceiver = broadcast::Receiver<Stamped<ProviderEvent>>;

/// BUILD-domain signals (W10) — finished slides landing in episodes.
/// Clone (owned strings); stamped like capture/jobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildEvent {
    /// A finished slide folder was placed into a chapter (after commit).
    SlidePlaced {
        show: String,
        episode: String,
        chapter: String,
        slide: String,
    },
    /// An episode document changed shape (chapters/slides).
    EpisodeChanged { show: String, episode: String },
    /// A card (slide) was authored on the authoring surface — published
    /// AFTER the draft `slide.json` is written (emit-with-mutation). The
    /// authoring charter's first fact: every ask leaves a real card.
    /// `show`/`episode` are empty until the card is placed into an episode
    /// (Studio's `SlidePlaced` is that later fact); `slide` is the draft id.
    CardAuthored {
        show: String,
        episode: String,
        slide: String,
    },
}

pub type BuildReceiver = broadcast::Receiver<Stamped<BuildEvent>>;

/// Who activated a touchpoint — mirrored from the pilot protocol so the
/// fabric stays serde-free (doctrine law 4) and dependency-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoachActor {
    Operator,
    Agent,
    /// Application state moved with no input to explain it.
    App,
}

/// COACH-domain signals — touchpoint lifecycle facts from the operator
/// coach (the EguiPilotAI plugin). The publisher is the devtools-gated
/// pilot bridge in the app; the domain itself is unconditional so every
/// consumer (activity feed, ledger, A/B measurement) compiles once.
///
/// LAW (plan §9, Phase 1.5): the coach NEVER subscribes to this domain.
/// A touchpoint waiting on a coach event would self-trigger — the
/// coachable-signal catalog deliberately excludes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoachEvent {
    /// Accepted, but its target is not on screen yet.
    Armed { coach_id: String },
    /// Bound to a live target.
    Mounted { coach_id: String },
    /// An input edge fired the touchpoint. `trigger` speaks the pilot
    /// CLI's `--on` vocabulary: `press`, `hover`, `node-appeared`,
    /// `value-changed`, `signal:<id>`.
    Triggered {
        coach_id: String,
        trigger: String,
        actor: CoachActor,
    },
    /// The touchpoint's postcondition holds — the step completed because
    /// the application reached the state, not because a button was pressed.
    Satisfied { coach_id: String },
    /// `reason` speaks the protocol's kebab-case vocabulary: `ttl`,
    /// `operator`, `agent`, `once`, `target-lost`, `session-ended`.
    Unmounted { coach_id: String, reason: String },
}

pub type CoachReceiver = broadcast::Receiver<Stamped<CoachEvent>>;

/// INTENT-domain signals — privileged-mode transitions and their refusals
/// (determinism-intent-gate plan; signal-ledger plan §9.2/§9.3). The audit
/// trail of the operator intent gate: every mode change names its cause,
/// every refused escalation is a recorded fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentEvent {
    /// The determinism mode changed. `cause` is `None` for behavioral
    /// (the default has no cause), and names who forced strict otherwise.
    DeterminismChanged {
        mode: dpl_intent::DeterminismMode,
        cause: Option<dpl_intent::StrictCause>,
    },
    /// A privileged transition was asked for and refused. `action` is what
    /// was asked (`strict`, `behavioral`); `reason` is why it was refused
    /// (`synthetic-input` — the pilot attributed the activating input to
    /// the agent; `evidence-locked` — the run is the evidence).
    IntentRefused { action: String, reason: String },
}

pub type IntentReceiver = broadcast::Receiver<Stamped<IntentEvent>>;

/// One AI call, correlated across its lifetime (dispatch → first token →
/// finish → disposition). Minted from the SAME sequence space as stamps
/// and job ids, so a call is causally ordered against every other signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallId(pub u64);

impl fmt::Display for CallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "call-{}", self.0)
    }
}

/// WHERE an AI call came from — the attribution key.
///
/// Without this, "is the AI earning its keep in the authoring loop?" is
/// unanswerable: cost and latency alone cannot say which surface spent
/// the tokens or what the call was working on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Origin {
    /// Where the operator was standing when the call was dispatched.
    pub route: Option<Route>,
    /// The surface that dispatched it (`"chat"`, `"author-editor"`, …).
    /// A static str: surfaces are code, not operator data.
    pub surface: &'static str,
    /// What it was working on — slide/comp/episode id when there is one.
    pub subject: Option<String>,
}

impl Origin {
    /// The common case: a surface with no route or subject context.
    pub const fn surface(surface: &'static str) -> Self {
        Self {
            route: None,
            surface,
            subject: None,
        }
    }
}

/// Why an AI call did not return a usable answer. Distinct from
/// `AiOutcome::Refused` — a refusal is a valid answer the model chose to
/// give; these are failures of the call itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiFailureClass {
    Auth,
    RateLimit,
    Timeout,
    Transport,
    Protocol,
    /// The provider's local runtime broke (parity with
    /// `ProviderFailureClass::Runtime`).
    Runtime,
    Server,
    Unknown,
}

impl AiFailureClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::RateLimit => "rate-limit",
            Self::Timeout => "timeout",
            Self::Transport => "transport",
            Self::Protocol => "protocol",
            Self::Runtime => "runtime",
            Self::Server => "server",
            Self::Unknown => "unknown",
        }
    }
}

/// How an AI call ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiOutcome {
    Ok,
    /// The model answered by declining — a real answer, not a failure.
    Refused,
    Cancelled,
    Error(AiFailureClass),
}

impl AiOutcome {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Refused => "refused",
            Self::Cancelled => "cancelled",
            Self::Error(_) => "error",
        }
    }

    /// The failure class when this outcome is an error.
    pub const fn failure(self) -> Option<AiFailureClass> {
        match self {
            Self::Error(class) => Some(class),
            _ => None,
        }
    }
}

/// Was this fact WITNESSED live, or recovered from records already on
/// disk?
///
/// The distinction is load-bearing, not bookkeeping. A history file
/// cannot yield time-to-first-token, and a backfilled event's `seq` is
/// synthetic — it was never ordered against the captures and cards it
/// now sits beside. Recording recovered facts as live would silently
/// corrupt every latency percentile and every cross-domain join, which
/// is exactly the kind of quiet wrongness analytics must not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Observation {
    /// Witnessed as it happened. Latency is real; ordering is real.
    #[default]
    Live,
    /// Recovered from a stored session. Never carries `FirstToken`, and
    /// queries must exclude it from any latency or ordering question.
    Backfilled,
}

impl Observation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Backfilled => "backfilled",
        }
    }

    /// Whether timing derived from this fact means anything.
    pub const fn timing_is_real(self) -> bool {
        matches!(self, Self::Live)
    }
}

/// AI-domain facts — what the AI cost, how fast it was, what it was
/// doing, and whether the operator kept the result.
///
/// Message CONTENT is deliberately absent from this type: prompts and
/// responses are segregated into their own opt-in, credential-scrubbed
/// store and joined by hash (Phase 3 §3.6). Events stay small and fast.
///
/// `cost_micros` is DERIVED from a versioned price table; `tokens_in`,
/// `tokens_out` and `model` are the truth and are always carried, so
/// cost can be recomputed when the table was wrong or prices moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiEvent {
    /// A call was dispatched to a provider.
    CallStarted {
        call: CallId,
        provider: ProviderId,
        model: String,
        origin: Origin,
        /// Live, or recovered from a stored session. See `Observation`.
        observation: Observation,
    },
    /// First token arrived — time-to-first-token, the latency operators
    /// actually feel.
    FirstToken { call: CallId, ttft_ms: u32 },
    /// The call ended, however it ended.
    ///
    /// Every count is OPTIONAL because providers report different things:
    /// Codex gives an input/output split, Grok gives context-window
    /// occupancy. `None` means "the provider did not say" — which is not
    /// zero, and must never be priced as zero.
    CallFinished {
        call: CallId,
        tokens_in: Option<u64>,
        /// Served from cache — bills at a fraction of fresh input, so it
        /// is never folded into `tokens_in`.
        cache_read: Option<u64>,
        /// Written into the cache — bills above fresh input.
        cache_write: Option<u64>,
        tokens_out: Option<u64>,
        total_tokens: Option<u64>,
        context_used: Option<u64>,
        context_size: Option<u64>,
        /// `None` until the price table prices it (Phase 3 PR 2). Derived,
        /// never authoritative — recomputed from the counts above.
        cost_micros: Option<u64>,
        duration_ms: u32,
        outcome: AiOutcome,
    },
    /// The attribution payoff: did the operator KEEP what the AI made?
    /// `target` names what it became when kept (slide/comp id).
    Disposition {
        call: CallId,
        kept: bool,
        target: Option<String>,
    },
}

impl AiEvent {
    /// Every AI fact belongs to exactly one call — the correlation key.
    pub const fn call(&self) -> CallId {
        match self {
            Self::CallStarted { call, .. }
            | Self::FirstToken { call, .. }
            | Self::CallFinished { call, .. }
            | Self::Disposition { call, .. } => *call,
        }
    }
}

pub type AiReceiver = broadcast::Receiver<Stamped<AiEvent>>;

/// The installed UI waker callback (doctrine law 6).
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// STAMP AT PUBLISH (doctrine law 5): every capture/job signal carries
/// `seq` from ONE global counter (total causal order ACROSS domains)
/// and `ms` monotonic from the fabric's birth — the flight-recorder
/// truth session replay and analytics reconstruct from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamped<T> {
    pub seq: u64,
    pub ms: u64,
    pub ev: T,
}

/// The fabric hands out senders and subscriptions; it never processes.
/// Broadcast is deliberately lossy under pressure (oldest dropped, receiver
/// told how many) — correct for signals, wrong for jobs. Job queues get
/// per-consumer mpsc when the engine lands.
/// Clone shares the SAME channels — one fabric, many holders.
#[derive(Clone)]
pub struct Fabric {
    shell: broadcast::Sender<ShellEvent>,
    deck: broadcast::Sender<DeckEvent>,
    capture: broadcast::Sender<Stamped<CaptureEvent>>,
    jobs: broadcast::Sender<Stamped<JobEvent>>,
    providers: broadcast::Sender<Stamped<ProviderEvent>>,
    build: broadcast::Sender<Stamped<BuildEvent>>,
    coach: broadcast::Sender<Stamped<CoachEvent>>,
    intent: broadcast::Sender<Stamped<IntentEvent>>,
    ai: broadcast::Sender<Stamped<AiEvent>>,
    /// THE JOBBOARD — the fold of the jobs stream (+ each row's latest-wins
    /// progress counter), shared by every Fabric clone. Running rows stay
    /// until resolved; resolved rows stay until acked, bounded by
    /// `RESOLVED_CAP` for publishers whose consumers never ack.
    board: Arc<std::sync::Mutex<Vec<JobRow>>>,
    /// The stamp state: one global sequence + the birth instant.
    seq: Arc<AtomicU64>,
    born: Instant,
    /// The WAKER (doctrine law 6): installed by the app as a
    /// `request_repaint` closure so background publishes surface the
    /// same frame. The fabric stays UI-free — this is just a callback.
    waker: Arc<std::sync::Mutex<Option<Waker>>>,
}

impl Default for Fabric {
    fn default() -> Self {
        Self::new()
    }
}

impl Fabric {
    pub fn new() -> Self {
        let (shell, _) = broadcast::channel(64);
        let (deck, _) = broadcast::channel(64);
        let (capture, _) = broadcast::channel(256);
        let (jobs, _) = broadcast::channel(64);
        let (providers, _) = broadcast::channel(128);
        let (build, _) = broadcast::channel(64);
        let (coach, _) = broadcast::channel(128);
        let (intent, _) = broadcast::channel(64);
        // AI facts arrive up to 4 per call (start/first-token/finish/
        // disposition) across concurrent provider calls — sized like the
        // provider lane, not the chatty capture lane.
        let (ai, _) = broadcast::channel(128);
        Self {
            shell,
            deck,
            capture,
            jobs,
            providers,
            build,
            coach,
            intent,
            ai,
            board: Arc::new(std::sync::Mutex::new(Vec::new())),
            seq: Arc::new(AtomicU64::new(0)),
            born: Instant::now(),
            waker: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Install the UI waker (once, at boot). Every publish invokes it.
    pub fn attach_waker(&self, wake: Waker) {
        *self
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(wake);
    }

    fn wake(&self) {
        if let Some(w) = self
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            w();
        }
    }

    fn stamp<T>(&self, ev: T) -> Stamped<T> {
        Stamped {
            seq: self.seq.fetch_add(1, Ordering::Relaxed),
            ms: self.born.elapsed().as_millis() as u64,
            ev,
        }
    }

    /// Mint a process-unique job id (same sequence space as stamps —
    /// a JobId is causally ordered against every signal).
    pub fn mint_job(&self) -> JobId {
        JobId(self.seq.fetch_add(1, Ordering::Relaxed))
    }

    pub fn mint_provider_attempt(&self) -> ActivationAttemptId {
        ActivationAttemptId(self.seq.fetch_add(1, Ordering::Relaxed))
    }

    /// Mint a process-unique AI call id (same sequence space as stamps —
    /// a CallId is causally ordered against every signal).
    pub fn mint_ai_call(&self) -> CallId {
        CallId(self.seq.fetch_add(1, Ordering::Relaxed))
    }

    /// Publish a capture signal — stamped, waking the UI.
    pub fn publish_capture(&self, event: CaptureEvent) {
        let _ = self.capture.send(self.stamp(event));
        self.wake();
    }

    pub fn subscribe_capture(&self) -> CaptureReceiver {
        self.capture.subscribe()
    }

    /// Publish a job transition — folded into THE JOBBOARD first, then
    /// stamped and broadcast, waking the UI (a consumer woken by the signal
    /// reads a board that is already true).
    pub fn publish_job(&self, event: JobEvent) {
        self.fold_job(&event);
        let _ = self.jobs.send(self.stamp(event));
        self.wake();
    }

    fn fold_job(&self, event: &JobEvent) {
        let mut board = self
            .board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match event {
            JobEvent::Queued { job, label } => {
                board.retain(|r| r.job != *job);
                board.push(JobRow {
                    job: *job,
                    label: label.clone(),
                    state: JobState::Running,
                    progress: Arc::new(AtomicUsize::new(0)),
                });
            }
            JobEvent::Done { job, out } => {
                Self::resolve(&mut board, *job, JobState::Done { out: out.clone() });
            }
            JobEvent::Failed { job, error } => {
                Self::resolve(
                    &mut board,
                    *job,
                    JobState::Failed {
                        error: error.clone(),
                    },
                );
            }
        }
        // Unacked resolved rows are bounded; Running rows never evict.
        let mut resolved = board.iter().filter(|r| !r.running()).count();
        while resolved > RESOLVED_CAP {
            match board.iter().position(|r| !r.running()) {
                Some(i) => {
                    board.remove(i);
                    resolved -= 1;
                }
                None => break,
            }
        }
    }

    /// A transition without a Queued still lands — the board never panics,
    /// never lies by omission.
    fn resolve(board: &mut Vec<JobRow>, job: JobId, state: JobState) {
        match board.iter_mut().find(|r| r.job == job) {
            Some(row) => row.state = state,
            None => board.push(JobRow {
                job,
                label: "(untracked)".into(),
                state,
                progress: Arc::new(AtomicUsize::new(0)),
            }),
        }
    }

    /// One board row, cloned (`progress` shares the live counter).
    pub fn job(&self, job: JobId) -> Option<JobRow> {
        self.board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find(|r| r.job == job)
            .cloned()
    }

    /// The whole board, in arrival order.
    pub fn jobs(&self) -> Vec<JobRow> {
        self.board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// How many jobs are running right now — the "working…" truth.
    pub fn jobs_running(&self) -> usize {
        self.board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|r| r.running())
            .count()
    }

    /// Remove a row — the consumer that showed the outcome acknowledges it.
    pub fn ack_job(&self, job: JobId) {
        self.board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|r| r.job != job);
    }

    pub fn subscribe_jobs(&self) -> JobReceiver {
        self.jobs.subscribe()
    }

    /// Publish a redacted provider lifecycle fact.
    pub fn publish_provider(&self, event: ProviderEvent) {
        let _ = self.providers.send(self.stamp(event));
        self.wake();
    }

    pub fn subscribe_providers(&self) -> ProviderReceiver {
        self.providers.subscribe()
    }

    /// Publish a BUILD-domain fact (slide placed / episode changed).
    pub fn publish_build(&self, event: BuildEvent) {
        let _ = self.build.send(self.stamp(event));
        self.wake();
    }

    pub fn subscribe_build(&self) -> BuildReceiver {
        self.build.subscribe()
    }

    /// Publish a COACH-domain fact (touchpoint lifecycle).
    pub fn publish_coach(&self, event: CoachEvent) {
        let _ = self.coach.send(self.stamp(event));
        self.wake();
    }

    pub fn subscribe_coach(&self) -> CoachReceiver {
        self.coach.subscribe()
    }

    /// Publish an INTENT-domain fact (privileged-mode transition/refusal).
    pub fn publish_intent(&self, event: IntentEvent) {
        let _ = self.intent.send(self.stamp(event));
        self.wake();
    }

    pub fn subscribe_intent(&self) -> IntentReceiver {
        self.intent.subscribe()
    }

    /// Publish an AI-domain fact (cost, latency, attribution, disposition).
    /// Message content never rides this channel — see `AiEvent`.
    pub fn publish_ai(&self, event: AiEvent) {
        let _ = self.ai.send(self.stamp(event));
        self.wake();
    }

    pub fn subscribe_ai(&self) -> AiReceiver {
        self.ai.subscribe()
    }

    /// Publish, ignoring "no subscribers yet" — a signal with no listeners
    /// is fine; a listener that must not miss signals uses mpsc, not this.
    pub fn publish_shell(&self, event: ShellEvent) {
        let _ = self.shell.send(event);
    }

    pub fn subscribe_shell(&self) -> broadcast::Receiver<ShellEvent> {
        self.shell.subscribe()
    }

    pub fn publish_deck(&self, event: DeckEvent) {
        let _ = self.deck.send(event);
    }

    pub fn subscribe_deck(&self) -> DeckReceiver {
        self.deck.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_hub::{HubId, SubId};

    #[test]
    fn a_subscriber_sees_the_typed_event() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_shell();
        let route = Route {
            hub: HubId("wiredfront"),
            sub: SubId("viziwiz"),
        };
        fabric.publish_shell(ShellEvent::RouteChanged { route });
        assert_eq!(rx.try_recv().unwrap(), ShellEvent::RouteChanged { route });
    }

    #[test]
    fn the_deck_domain_round_trips_typed_beats() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_deck();
        let beat = DeckEvent::Beat {
            index: 1,
            step: 2,
            slides: 4,
            steps: 3,
        };
        fabric.publish_deck(beat);
        assert_eq!(rx.try_recv().unwrap(), beat);
        // Domains are separate channels — a deck beat never wakes shell ears.
        let mut shell_rx = fabric.subscribe_shell();
        fabric.publish_deck(beat);
        assert!(shell_rx.try_recv().is_err());
    }

    #[test]
    fn a_navigate_request_round_trips() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_shell();
        let route = Route {
            hub: HubId("workbench"),
            sub: SubId("library"),
        };
        fabric.publish_shell(ShellEvent::NavigateRequest { route });
        assert_eq!(
            rx.try_recv().unwrap(),
            ShellEvent::NavigateRequest { route }
        );
    }

    #[test]
    fn a_window_request_round_trips_without_shell_feature_knowledge() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_shell();
        let event = ShellEvent::WindowRequest {
            target: AppWindowKey("chat.primary"),
            action: AppWindowAction::OpenOrFocus,
        };
        fabric.publish_shell(event);
        assert_eq!(rx.try_recv().unwrap(), event);
    }

    #[test]
    fn capture_signals_stamp_in_total_order_and_round_trip() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_capture();
        fabric.publish_capture(CaptureEvent::InboxChanged);
        fabric.publish_capture(CaptureEvent::Saved {
            rel_path: "inbox/pix.png".into(),
            kind: "image".into(),
            name: "pix.png".into(),
            location: "inbox".into(),
            source: "viziwiz".into(),
        });
        let a = rx.try_recv().unwrap();
        let b = rx.try_recv().unwrap();
        assert_eq!(a.ev, CaptureEvent::InboxChanged);
        assert!(matches!(&b.ev, CaptureEvent::Saved { kind, .. } if kind == "image"));
        assert!(
            b.seq > a.seq,
            "one global sequence orders signals: {} > {}",
            b.seq,
            a.seq
        );
        assert!(b.ms >= a.ms, "monotonic ms");
        // Domains stay separate: capture noise never wakes shell ears.
        let mut shell_rx = fabric.subscribe_shell();
        fabric.publish_capture(CaptureEvent::InboxChanged);
        assert!(shell_rx.try_recv().is_err());
    }

    #[test]
    fn coach_signals_stamp_in_total_order_and_round_trip() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_coach();
        fabric.publish_coach(CoachEvent::Mounted {
            coach_id: "lesson-1/step-2".into(),
        });
        fabric.publish_coach(CoachEvent::Satisfied {
            coach_id: "lesson-1/step-2".into(),
        });
        let a = rx.try_recv().unwrap();
        let b = rx.try_recv().unwrap();
        assert!(matches!(&a.ev, CoachEvent::Mounted { coach_id } if coach_id == "lesson-1/step-2"));
        assert!(matches!(&b.ev, CoachEvent::Satisfied { .. }));
        assert!(b.seq > a.seq, "one global sequence orders signals");
        // Domains stay separate: coach lifecycle never wakes capture ears.
        let mut capture_rx = fabric.subscribe_capture();
        fabric.publish_coach(CoachEvent::Triggered {
            coach_id: "lesson-1/step-3".into(),
            trigger: "press".into(),
            actor: CoachActor::Operator,
        });
        assert!(capture_rx.try_recv().is_err());
    }

    #[test]
    fn intent_facts_round_trip_with_their_cause() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_intent();
        fabric.publish_intent(IntentEvent::DeterminismChanged {
            mode: dpl_intent::DeterminismMode::Strict,
            cause: Some(dpl_intent::StrictCause::Operator(
                dpl_intent::IntentChannel::SlashCommand,
            )),
        });
        fabric.publish_intent(IntentEvent::IntentRefused {
            action: "strict".into(),
            reason: "synthetic-input".into(),
        });
        let a = rx.try_recv().unwrap();
        let b = rx.try_recv().unwrap();
        assert!(matches!(
            a.ev,
            IntentEvent::DeterminismChanged {
                mode: dpl_intent::DeterminismMode::Strict,
                ..
            }
        ));
        assert!(matches!(&b.ev, IntentEvent::IntentRefused { reason, .. }
            if reason == "synthetic-input"));
        assert!(b.seq > a.seq);
    }

    #[test]
    fn recorder_facts_round_trip_on_the_capture_domain() {
        // The steps/session recorder's FACTS (state + skip) are capture
        // signals — the COMMAND that toggles recording stays a direct call.
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_capture();
        fabric.publish_capture(CaptureEvent::Recording { on: true, count: 0 });
        fabric.publish_capture(CaptureEvent::Skipped {
            reason: "nothing changed".into(),
        });
        fabric.publish_capture(CaptureEvent::Recording {
            on: false,
            count: 2,
        });
        let a = rx.try_recv().unwrap();
        let b = rx.try_recv().unwrap();
        let c = rx.try_recv().unwrap();
        assert_eq!(a.ev, CaptureEvent::Recording { on: true, count: 0 });
        assert!(matches!(b.ev, CaptureEvent::Skipped { .. }));
        assert_eq!(
            c.ev,
            CaptureEvent::Recording {
                on: false,
                count: 2
            }
        );
        assert!(c.seq > a.seq, "stamped in total order");
    }

    #[test]
    fn jobs_transitions_stamp_and_job_ids_share_the_sequence() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_jobs();
        let job = fabric.mint_job();
        fabric.publish_job(JobEvent::Queued {
            job,
            label: "render".into(),
        });
        fabric.publish_job(JobEvent::Done {
            job,
            out: "inbox/comp-1/render.mp4".into(),
        });
        let q = rx.try_recv().unwrap();
        let d = rx.try_recv().unwrap();
        assert!(matches!(q.ev, JobEvent::Queued { job: j, .. } if j == job));
        assert!(matches!(d.ev, JobEvent::Done { job: j, .. } if j == job));
        assert!(
            job.0 < q.seq,
            "the JobId is causally before its transitions"
        );
    }

    #[test]
    fn the_board_folds_the_job_stream() {
        let fabric = Fabric::new();
        let job = fabric.mint_job();
        fabric.publish_job(JobEvent::Queued {
            job,
            label: "render".into(),
        });
        assert_eq!(fabric.jobs_running(), 1);
        let row = fabric.job(job).expect("queued row");
        assert!(row.running());
        assert_eq!(row.label, "render");
        fabric.publish_job(JobEvent::Done {
            job,
            out: "inbox/comp-1/render.mp4".into(),
        });
        assert_eq!(fabric.jobs_running(), 0);
        let row = fabric.job(job).expect("resolved row stays until acked");
        assert_eq!(
            row.state,
            JobState::Done {
                out: "inbox/comp-1/render.mp4".into()
            }
        );
        fabric.ack_job(job);
        assert!(fabric.job(job).is_none(), "ack removes the row");
    }

    #[test]
    fn board_progress_is_shared_with_the_worker() {
        let fabric = Fabric::new();
        let job = fabric.mint_job();
        fabric.publish_job(JobEvent::Queued {
            job,
            label: "ingest pack".into(),
        });
        let worker = fabric.job(job).expect("row").progress;
        worker.fetch_add(7, Ordering::Relaxed);
        let seen = fabric
            .job(job)
            .expect("row")
            .progress
            .load(Ordering::Relaxed);
        assert_eq!(seen, 7, "the counter is one shared cell, not a copy");
    }

    #[test]
    fn resolved_rows_evict_at_the_cap_running_never() {
        let fabric = Fabric::new();
        let running = fabric.mint_job();
        fabric.publish_job(JobEvent::Queued {
            job: running,
            label: "long render".into(),
        });
        for _ in 0..70 {
            let job = fabric.mint_job();
            fabric.publish_job(JobEvent::Queued {
                job,
                label: "stage".into(),
            });
            fabric.publish_job(JobEvent::Done {
                job,
                out: "ok".into(),
            });
        }
        let rows = fabric.jobs();
        assert_eq!(rows.iter().filter(|r| !r.running()).count(), 64);
        assert!(
            fabric.job(running).is_some_and(|r| r.running()),
            "the running row outlives every eviction"
        );
    }

    #[test]
    fn build_events_stamp_and_round_trip() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_build();
        fabric.publish_build(BuildEvent::SlidePlaced {
            show: "devpulse-news".into(),
            episode: "ep1".into(),
            chapter: "intro".into(),
            slide: "title-neon".into(),
        });
        fabric.publish_build(BuildEvent::EpisodeChanged {
            show: "devpulse-news".into(),
            episode: "ep1".into(),
        });
        let a = rx.try_recv().unwrap();
        let b = rx.try_recv().unwrap();
        assert!(matches!(
            a.ev,
            BuildEvent::SlidePlaced {
                ref show,
                ref slide,
                ..
            } if show == "devpulse-news" && slide == "title-neon"
        ));
        assert!(matches!(b.ev, BuildEvent::EpisodeChanged { .. }));
        assert!(b.seq > a.seq);
    }

    #[test]
    fn provider_facts_share_one_attempt_without_secret_payload_fields() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_providers();
        let attempt = fabric.mint_provider_attempt();
        fabric.publish_provider(ProviderEvent::ActivationRequested {
            provider: ProviderId("grok"),
            attempt,
        });
        fabric.publish_provider(ProviderEvent::DeviceCodeReceived {
            provider: ProviderId("grok"),
            attempt,
            code_len: 9,
        });
        let requested = rx.try_recv().unwrap();
        let prompted = rx.try_recv().unwrap();
        assert!(matches!(
            requested.ev,
            ProviderEvent::ActivationRequested { attempt: seen, .. } if seen == attempt
        ));
        assert!(matches!(
            prompted.ev,
            ProviderEvent::DeviceCodeReceived {
                attempt: seen,
                code_len: 9,
                ..
            } if seen == attempt
        ));
        assert!(prompted.seq > requested.seq);
        assert_eq!(prompted.ev.provider(), ProviderId("grok"));
        assert_eq!(prompted.ev.attempt(), Some(attempt));
    }

    #[test]
    fn the_waker_fires_on_every_publish() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let fabric = Fabric::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let h = Arc::clone(&hits);
        fabric.attach_waker(Arc::new(move || {
            h.fetch_add(1, Ordering::Relaxed);
        }));
        fabric.publish_capture(CaptureEvent::InboxChanged);
        fabric.publish_job(JobEvent::Queued {
            job: fabric.mint_job(),
            label: "x".into(),
        });
        assert_eq!(
            hits.load(Ordering::Relaxed),
            2,
            "background publishes wake the UI"
        );
    }

    #[test]
    fn publishing_with_no_subscribers_is_not_an_error() {
        let fabric = Fabric::new();
        fabric.publish_shell(ShellEvent::RouteChanged {
            route: Route {
                hub: HubId("home"),
                sub: SubId("shows"),
            },
        });
        // No panic, no Result to mishandle — the signal simply had no audience.
    }

    #[test]
    fn an_ai_call_correlates_across_its_whole_lifetime() {
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_ai();
        let call = fabric.mint_ai_call();

        fabric.publish_ai(AiEvent::CallStarted {
            call,
            provider: ProviderId("claude"),
            model: "claude-opus-5".into(),
            origin: Origin {
                route: Some(Route {
                    hub: HubId("workbench"),
                    sub: SubId("author"),
                }),
                surface: "author-editor",
                subject: Some("slide-7".into()),
            },
            observation: Observation::Live,
        });
        fabric.publish_ai(AiEvent::FirstToken { call, ttft_ms: 320 });
        fabric.publish_ai(AiEvent::CallFinished {
            call,
            tokens_in: Some(1200),
            cache_read: Some(800),
            cache_write: None,
            tokens_out: Some(450),
            total_tokens: Some(1650),
            context_used: None,
            context_size: None,
            cost_micros: None,
            duration_ms: 2_100,
            outcome: AiOutcome::Ok,
        });
        fabric.publish_ai(AiEvent::Disposition {
            call,
            kept: true,
            target: Some("slide-7".into()),
        });

        // All four facts carry the SAME call id — that is what makes
        // cost, latency, attribution and disposition joinable.
        for _ in 0..4 {
            assert_eq!(rx.try_recv().unwrap().ev.call(), call);
        }
    }

    #[test]
    fn ai_facts_are_stamped_and_ordered_against_other_domains() {
        let fabric = Fabric::new();
        let mut ai_rx = fabric.subscribe_ai();
        let mut build_rx = fabric.subscribe_build();

        let call = fabric.mint_ai_call();
        fabric.publish_ai(AiEvent::Disposition {
            call,
            kept: true,
            target: Some("slide-1".into()),
        });
        fabric.publish_build(BuildEvent::CardAuthored {
            show: String::new(),
            episode: String::new(),
            slide: "slide-1".into(),
        });

        let ai = ai_rx.try_recv().unwrap();
        let build = build_rx.try_recv().unwrap();
        // ONE global sequence across domains: the AI disposition provably
        // precedes the card it produced. This is the join the whole
        // analytics program stands on.
        assert!(
            ai.seq < build.seq,
            "ai seq {} should precede build seq {}",
            ai.seq,
            build.seq
        );
    }

    #[test]
    fn a_minted_call_id_never_repeats() {
        let fabric = Fabric::new();
        let a = fabric.mint_ai_call();
        let b = fabric.mint_ai_call();
        assert_ne!(a, b);
        // And it shares the stamp sequence — never a parallel counter.
        let job = fabric.mint_job();
        assert_ne!(a.0, job.0);
        assert_ne!(b.0, job.0);
    }

    #[test]
    fn an_outcome_reports_its_failure_class_or_none() {
        assert_eq!(AiOutcome::Ok.failure(), None);
        assert_eq!(AiOutcome::Refused.failure(), None);
        assert_eq!(AiOutcome::Refused.label(), "refused");
        assert_eq!(
            AiOutcome::Error(AiFailureClass::RateLimit).failure(),
            Some(AiFailureClass::RateLimit)
        );
        assert_eq!(AiFailureClass::RateLimit.label(), "rate-limit");
    }

    #[test]
    fn ai_facts_do_not_wake_other_domain_ears() {
        let fabric = Fabric::new();
        let mut provider_rx = fabric.subscribe_providers();
        fabric.publish_ai(AiEvent::FirstToken {
            call: fabric.mint_ai_call(),
            ttft_ms: 10,
        });
        assert!(
            provider_rx.try_recv().is_err(),
            "AI usage is its own domain, not provider auth lifecycle"
        );
    }
}
