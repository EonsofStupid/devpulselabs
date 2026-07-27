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
use std::sync::atomic::{AtomicU64, Ordering};
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
    BrowserOpened {
        provider: ProviderId,
        attempt: ActivationAttemptId,
        browser: String,
    },
    BrowserClosed {
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
            | Self::ActivationRequested { provider, .. }
            | Self::ClientStarted { provider, .. }
            | Self::DeviceCodeReceived { provider, .. }
            | Self::CodeCopied { provider, .. }
            | Self::BrowserOpened { provider, .. }
            | Self::BrowserClosed { provider, .. }
            | Self::AuthorizationCompleted { provider, .. }
            | Self::RuntimeStarted { provider, .. }
            | Self::Ready { provider, .. }
            | Self::Failed { provider, .. }
            | Self::Cancelled { provider, .. }
            | Self::Expired { provider, .. } => *provider,
        }
    }

    pub const fn attempt(&self) -> Option<ActivationAttemptId> {
        match self {
            Self::Selected { .. } => None,
            Self::ActivationRequested { attempt, .. }
            | Self::ClientStarted { attempt, .. }
            | Self::DeviceCodeReceived { attempt, .. }
            | Self::CodeCopied { attempt, .. }
            | Self::BrowserOpened { attempt, .. }
            | Self::BrowserClosed { attempt, .. }
            | Self::AuthorizationCompleted { attempt, .. }
            | Self::RuntimeStarted { attempt, .. }
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
}

pub type BuildReceiver = broadcast::Receiver<Stamped<BuildEvent>>;

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
        Self {
            shell,
            deck,
            capture,
            jobs,
            providers,
            build,
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

    /// Publish a capture signal — stamped, waking the UI.
    pub fn publish_capture(&self, event: CaptureEvent) {
        let _ = self.capture.send(self.stamp(event));
        self.wake();
    }

    pub fn subscribe_capture(&self) -> CaptureReceiver {
        self.capture.subscribe()
    }

    /// Publish a job transition — stamped, waking the UI.
    pub fn publish_job(&self, event: JobEvent) {
        let _ = self.jobs.send(self.stamp(event));
        self.wake();
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
}
