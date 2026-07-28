//! The coachable signal catalog: which application facts a touchpoint may
//! wait for, and what they are called.
//!
//! This is the ONLY direction signals travel toward the agent. The toolkit
//! never subscribes to the fabric and cannot enumerate it; the application
//! decides here what is publishable, and everything else stays invisible.
//!
//! Naming reuses the agent seam's `domain` + `type` vocabulary
//! (`dpl_seam::event_json` and friends), kebab-cased, so one fact has one
//! name on both surfaces. Every mapping below is an exhaustive match for the
//! same reason the seam's is: a new fabric variant must fail compilation
//! here rather than silently become uncoachable.
//!
//! An id is either **coarse** (`<domain>.<type>`, always published) or
//! **specific** (`<domain>.<type>/<key>`, published alongside the coarse one
//! when the key comes from a set the app can enumerate up front — routes from
//! the hub registry, providers from the registrations, booleans from being
//! booleans). Keys that are free-form runtime strings — a capture mode, a job
//! id, a deck index, an operator's episode name — deliberately have no
//! specific id: an agent cannot be told about them ahead of time, so it waits
//! on the coarse fact and reads the detail from the UI.

use dpl_fabric::{
    BuildEvent, CaptureEvent, DeckEvent, Fabric, JobEvent, ProviderEvent, ShellEvent,
};
use dpl_hub::{HubSpec, Route};
use egui_pilot_ai::protocol::SignalId;

/// The app's live subscriptions, drained once per frame.
///
/// Deliberately NOT the full domain set: the `coach` and `intent` domains
/// are excluded by law (signal-ledger plan §9 / determinism-intent-gate).
/// Coach lifecycle facts are what `coach_fabric` publishes FROM the coach —
/// forwarding them back would let a touchpoint wait on a coach event and
/// self-trigger. Intent mode changes are not lesson material and must not
/// narrate the escalation surface to an agent. Do not "complete" this list.
pub struct CoachSignals {
    shell: dpl_fabric::ShellReceiver,
    deck: dpl_fabric::DeckReceiver,
    capture: dpl_fabric::CaptureReceiver,
    jobs: dpl_fabric::JobReceiver,
    providers: dpl_fabric::ProviderReceiver,
    build: dpl_fabric::BuildReceiver,
}

impl CoachSignals {
    pub fn subscribe(fabric: &Fabric) -> Self {
        Self {
            shell: fabric.subscribe_shell(),
            deck: fabric.subscribe_deck(),
            capture: fabric.subscribe_capture(),
            jobs: fabric.subscribe_jobs(),
            providers: fabric.subscribe_providers(),
            build: fabric.subscribe_build(),
        }
    }

    /// Every signal published since the last drain.
    ///
    /// Lagged is fine, as everywhere else on this fabric: a signal is a
    /// signal, and the next one arrives on the next frame.
    pub fn drain(&mut self) -> Vec<SignalId> {
        let mut signals = Vec::new();
        while let Ok(event) = self.shell.try_recv() {
            signals.extend(shell(&event));
        }
        while let Ok(event) = self.deck.try_recv() {
            signals.extend(deck(&event));
        }
        while let Ok(stamped) = self.capture.try_recv() {
            signals.extend(capture(&stamped.ev));
        }
        while let Ok(stamped) = self.jobs.try_recv() {
            signals.extend(jobs(&stamped.ev));
        }
        while let Ok(stamped) = self.providers.try_recv() {
            signals.extend(providers(&stamped.ev));
        }
        while let Ok(stamped) = self.build.try_recv() {
            signals.extend(build(&stamped.ev));
        }
        signals
    }
}

fn id(value: impl Into<String>) -> SignalId {
    SignalId::new(value)
}

fn route_id(kind: &str, route: &Route) -> SignalId {
    id(format!("{kind}/{}/{}", route.hub.0, route.sub.0))
}

fn shell(event: &ShellEvent) -> Vec<SignalId> {
    match event {
        ShellEvent::RouteChanged { route } => vec![
            id(SHELL_ROUTE_CHANGED),
            route_id(SHELL_ROUTE_CHANGED, route),
        ],
        ShellEvent::NavigateRequest { route } => vec![
            id(SHELL_NAVIGATE_REQUEST),
            route_id(SHELL_NAVIGATE_REQUEST, route),
        ],
        ShellEvent::FullBleedRequest(on) => vec![
            id(SHELL_FULL_BLEED),
            id(format!("{SHELL_FULL_BLEED}/{}", on_off(*on))),
        ],
        ShellEvent::ViziwizSummon => vec![id(SHELL_VIZIWIZ_SUMMON)],
        // The window target is a `&'static str` minted by whichever accessory
        // owns the window; nothing registers them centrally, so there is no
        // enumerable specific id to declare.
        ShellEvent::WindowRequest { .. } => vec![id(SHELL_WINDOW_REQUEST)],
    }
}

fn deck(event: &DeckEvent) -> Vec<SignalId> {
    match event {
        // Slide and step indices are unbounded, so a deck position has no
        // declarable specific id.
        DeckEvent::Beat { .. } => vec![id(DECK_BEAT)],
    }
}

fn capture(event: &CaptureEvent) -> Vec<SignalId> {
    match event {
        CaptureEvent::Started { .. } => vec![id(CAPTURE_STARTED)],
        CaptureEvent::Saved { .. } => vec![id(CAPTURE_SAVED)],
        CaptureEvent::Failed { .. } => vec![id(CAPTURE_FAILED)],
        CaptureEvent::InboxChanged => vec![id(CAPTURE_INBOX_CHANGED)],
        CaptureEvent::PaletteVisibility { visible } => vec![
            id(CAPTURE_PALETTE_VISIBILITY),
            id(format!(
                "{CAPTURE_PALETTE_VISIBILITY}/{}",
                if *visible { "shown" } else { "hidden" }
            )),
        ],
        CaptureEvent::Recording { on, .. } => vec![
            id(CAPTURE_RECORDING),
            id(format!("{CAPTURE_RECORDING}/{}", on_off(*on))),
        ],
        CaptureEvent::Skipped { .. } => vec![id(CAPTURE_SKIPPED)],
    }
}

fn jobs(event: &JobEvent) -> Vec<SignalId> {
    // A JobId is minted per process run, so it can never be declared.
    match event {
        JobEvent::Queued { .. } => vec![id(JOBS_QUEUED)],
        JobEvent::Done { .. } => vec![id(JOBS_DONE)],
        JobEvent::Failed { .. } => vec![id(JOBS_FAILED)],
    }
}

fn providers(event: &ProviderEvent) -> Vec<SignalId> {
    let kind = match event {
        ProviderEvent::Selected { .. } => PROVIDERS_SELECTED,
        ProviderEvent::ProfileConfigured { .. } => PROVIDERS_PROFILE_CONFIGURED,
        ProviderEvent::ActivationRequested { .. } => PROVIDERS_ACTIVATION_REQUESTED,
        ProviderEvent::ClientStarted { .. } => PROVIDERS_CLIENT_STARTED,
        ProviderEvent::DeviceCodeReceived { .. } => PROVIDERS_DEVICE_CODE_RECEIVED,
        ProviderEvent::CodeCopied { .. } => PROVIDERS_CODE_COPIED,
        ProviderEvent::SystemBrowserDispatched { .. } => PROVIDERS_SYSTEM_BROWSER_DISPATCHED,
        ProviderEvent::AuthorizationCompleted { .. } => PROVIDERS_AUTHORIZATION_COMPLETED,
        ProviderEvent::RuntimeStarted { .. } => PROVIDERS_RUNTIME_STARTED,
        ProviderEvent::VerificationRetryScheduled { .. } => PROVIDERS_VERIFICATION_RETRY_SCHEDULED,
        ProviderEvent::Ready { .. } => PROVIDERS_READY,
        ProviderEvent::Failed { .. } => PROVIDERS_FAILED,
        ProviderEvent::Cancelled { .. } => PROVIDERS_CANCELLED,
        ProviderEvent::Expired { .. } => PROVIDERS_EXPIRED,
    };
    // The provider is the one payload here that is registered up front, and
    // the redaction boundary holds: no code, url, token, or version reaches
    // an id.
    vec![id(kind), id(format!("{kind}/{}", event.provider().0))]
}

fn build(event: &BuildEvent) -> Vec<SignalId> {
    // Show and episode names are operator content. They stay out of the id
    // space; an agent waits on the fact and reads the names from the UI.
    match event {
        BuildEvent::SlidePlaced { .. } => vec![id(BUILD_SLIDE_PLACED)],
        BuildEvent::EpisodeChanged { .. } => vec![id(BUILD_EPISODE_CHANGED)],
        BuildEvent::CardAuthored { .. } => vec![id(BUILD_CARD_AUTHORED)],
    }
}

fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

pub const SHELL_ROUTE_CHANGED: &str = "shell.route-changed";
pub const SHELL_NAVIGATE_REQUEST: &str = "shell.navigate-request";
pub const SHELL_FULL_BLEED: &str = "shell.full-bleed";
pub const SHELL_VIZIWIZ_SUMMON: &str = "shell.viziwiz-summon";
pub const SHELL_WINDOW_REQUEST: &str = "shell.window-request";
pub const DECK_BEAT: &str = "deck.beat";
pub const CAPTURE_STARTED: &str = "capture.started";
pub const CAPTURE_SAVED: &str = "capture.saved";
pub const CAPTURE_FAILED: &str = "capture.failed";
pub const CAPTURE_INBOX_CHANGED: &str = "capture.inbox-changed";
pub const CAPTURE_PALETTE_VISIBILITY: &str = "capture.palette-visibility";
pub const CAPTURE_RECORDING: &str = "capture.recording";
pub const CAPTURE_SKIPPED: &str = "capture.skipped";
pub const JOBS_QUEUED: &str = "jobs.queued";
pub const JOBS_DONE: &str = "jobs.done";
pub const JOBS_FAILED: &str = "jobs.failed";
pub const PROVIDERS_SELECTED: &str = "providers.selected";
pub const PROVIDERS_PROFILE_CONFIGURED: &str = "providers.profile-configured";
pub const PROVIDERS_ACTIVATION_REQUESTED: &str = "providers.activation-requested";
pub const PROVIDERS_CLIENT_STARTED: &str = "providers.client-started";
pub const PROVIDERS_DEVICE_CODE_RECEIVED: &str = "providers.device-code-received";
pub const PROVIDERS_CODE_COPIED: &str = "providers.code-copied";
pub const PROVIDERS_SYSTEM_BROWSER_DISPATCHED: &str = "providers.system-browser-dispatched";
pub const PROVIDERS_AUTHORIZATION_COMPLETED: &str = "providers.authorization-completed";
pub const PROVIDERS_RUNTIME_STARTED: &str = "providers.runtime-started";
pub const PROVIDERS_VERIFICATION_RETRY_SCHEDULED: &str = "providers.verification-retry-scheduled";
pub const PROVIDERS_READY: &str = "providers.ready";
pub const PROVIDERS_FAILED: &str = "providers.failed";
pub const PROVIDERS_CANCELLED: &str = "providers.cancelled";
pub const PROVIDERS_EXPIRED: &str = "providers.expired";
pub const BUILD_SLIDE_PLACED: &str = "build.slide-placed";
pub const BUILD_EPISODE_CHANGED: &str = "build.episode-changed";
pub const BUILD_CARD_AUTHORED: &str = "build.card-authored";

/// Every coarse fact, in domain order. One entry per fabric variant.
const COARSE: &[&str] = &[
    SHELL_ROUTE_CHANGED,
    SHELL_NAVIGATE_REQUEST,
    SHELL_FULL_BLEED,
    SHELL_VIZIWIZ_SUMMON,
    SHELL_WINDOW_REQUEST,
    DECK_BEAT,
    CAPTURE_STARTED,
    CAPTURE_SAVED,
    CAPTURE_FAILED,
    CAPTURE_INBOX_CHANGED,
    CAPTURE_PALETTE_VISIBILITY,
    CAPTURE_RECORDING,
    CAPTURE_SKIPPED,
    JOBS_QUEUED,
    JOBS_DONE,
    JOBS_FAILED,
    PROVIDERS_SELECTED,
    PROVIDERS_PROFILE_CONFIGURED,
    PROVIDERS_ACTIVATION_REQUESTED,
    PROVIDERS_CLIENT_STARTED,
    PROVIDERS_DEVICE_CODE_RECEIVED,
    PROVIDERS_CODE_COPIED,
    PROVIDERS_SYSTEM_BROWSER_DISPATCHED,
    PROVIDERS_AUTHORIZATION_COMPLETED,
    PROVIDERS_RUNTIME_STARTED,
    PROVIDERS_VERIFICATION_RETRY_SCHEDULED,
    PROVIDERS_READY,
    PROVIDERS_FAILED,
    PROVIDERS_CANCELLED,
    PROVIDERS_EXPIRED,
    BUILD_SLIDE_PLACED,
    BUILD_EPISODE_CHANGED,
    BUILD_CARD_AUTHORED,
];

/// The declared catalog: every id an agent can wait for in THIS build.
///
/// Routes come from the hub registry and providers from the registrations
/// (Law 4 — the app never names a hub), so adding a hub or a provider widens
/// what is coachable without touching this function.
pub fn declare(
    hubs: impl IntoIterator<Item = &'static HubSpec>,
    providers: impl IntoIterator<Item = dpl_chat_core::ProviderId>,
) -> Vec<SignalId> {
    let mut declared: Vec<SignalId> = COARSE.iter().copied().map(id).collect();
    declared.push(id(format!("{SHELL_FULL_BLEED}/on")));
    declared.push(id(format!("{SHELL_FULL_BLEED}/off")));
    declared.push(id(format!("{CAPTURE_PALETTE_VISIBILITY}/shown")));
    declared.push(id(format!("{CAPTURE_PALETTE_VISIBILITY}/hidden")));
    declared.push(id(format!("{CAPTURE_RECORDING}/on")));
    declared.push(id(format!("{CAPTURE_RECORDING}/off")));
    for spec in hubs {
        for sub in spec.subroutes {
            let route = Route {
                hub: spec.id,
                sub: sub.id,
            };
            declared.push(route_id(SHELL_ROUTE_CHANGED, &route));
            declared.push(route_id(SHELL_NAVIGATE_REQUEST, &route));
        }
    }
    for provider in providers {
        for kind in COARSE.iter().filter(|kind| kind.starts_with("providers.")) {
            declared.push(id(format!("{kind}/{}", provider.0)));
        }
    }
    declared.sort_by(|left, right| left.0.cmp(&right.0));
    declared.dedup();
    declared
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::ProviderId;
    use dpl_fabric::{
        ActivationAttemptId, AppWindowAction, AppWindowKey, JobId, ProviderActivationStage,
        ProviderFailureClass,
    };
    use dpl_hub::{HubId, SubId, SubRoute};
    use std::collections::HashSet;

    static SPEC: HubSpec = HubSpec {
        id: HubId("workbench"),
        label: "Workbench",
        icon: "",
        tone: dpl_coldlight::Tone::Amber,
        subroutes: &[SubRoute {
            id: SubId("library"),
            label: "Library",
            blurb: "",
        }],
    };

    const PROVIDER: ProviderId = ProviderId("grok");

    fn route() -> Route {
        Route {
            hub: HubId("workbench"),
            sub: SubId("library"),
        }
    }

    /// One instance of every fabric variant, in the order they are declared
    /// in `dpl_fabric`. This list is the test's half of the exhaustiveness
    /// contract: the mapping functions cannot compile without covering a new
    /// variant, and `every_published_signal_is_declared` cannot pass without
    /// this list gaining it too.
    fn every_fabric_fact() -> Vec<SignalId> {
        let attempt = ActivationAttemptId(1);
        let mut signals = Vec::new();
        for event in [
            ShellEvent::RouteChanged { route: route() },
            ShellEvent::NavigateRequest { route: route() },
            ShellEvent::FullBleedRequest(true),
            ShellEvent::FullBleedRequest(false),
            ShellEvent::ViziwizSummon,
            ShellEvent::WindowRequest {
                target: AppWindowKey("chat.primary"),
                action: AppWindowAction::OpenOrFocus,
            },
        ] {
            signals.extend(shell(&event));
        }
        signals.extend(deck(&DeckEvent::Beat {
            index: 2,
            step: 1,
            slides: 8,
            steps: 3,
        }));
        for event in [
            CaptureEvent::Started {
                mode: "region".into(),
                target: "screen".into(),
            },
            CaptureEvent::Saved {
                rel_path: "a".into(),
                kind: "shot".into(),
                name: "b".into(),
                location: "c".into(),
                source: "d".into(),
            },
            CaptureEvent::Failed {
                error: "nope".into(),
            },
            CaptureEvent::InboxChanged,
            CaptureEvent::PaletteVisibility { visible: true },
            CaptureEvent::PaletteVisibility { visible: false },
            CaptureEvent::Recording { on: true, count: 1 },
            CaptureEvent::Recording {
                on: false,
                count: 4,
            },
            CaptureEvent::Skipped {
                reason: "nothing changed".into(),
            },
        ] {
            signals.extend(capture(&event));
        }
        for event in [
            JobEvent::Queued {
                job: JobId(1),
                label: "render".into(),
            },
            JobEvent::Done {
                job: JobId(1),
                out: "out.mp4".into(),
            },
            JobEvent::Failed {
                job: JobId(1),
                error: "nope".into(),
            },
        ] {
            signals.extend(jobs(&event));
        }
        for event in [
            ProviderEvent::Selected { provider: PROVIDER },
            ProviderEvent::ProfileConfigured {
                provider: PROVIDER,
                profile_key: "grok-default".into(),
            },
            ProviderEvent::ActivationRequested {
                provider: PROVIDER,
                attempt,
            },
            ProviderEvent::ClientStarted {
                provider: PROVIDER,
                attempt,
                client_version: "1".into(),
            },
            ProviderEvent::DeviceCodeReceived {
                provider: PROVIDER,
                attempt,
                code_len: 8,
            },
            ProviderEvent::CodeCopied {
                provider: PROVIDER,
                attempt,
            },
            ProviderEvent::SystemBrowserDispatched {
                provider: PROVIDER,
                attempt,
            },
            ProviderEvent::AuthorizationCompleted {
                provider: PROVIDER,
                attempt,
            },
            ProviderEvent::RuntimeStarted {
                provider: PROVIDER,
                attempt,
            },
            ProviderEvent::VerificationRetryScheduled {
                provider: PROVIDER,
                attempt,
                retry_index: 1,
                max_retries: 3,
                delay_ms: 250,
            },
            ProviderEvent::Ready {
                provider: PROVIDER,
                attempt,
                client_version: "1".into(),
                auth_method: "subscription",
            },
            ProviderEvent::Failed {
                provider: PROVIDER,
                attempt,
                stage: ProviderActivationStage::Runtime,
                class: ProviderFailureClass::Timeout,
                retryable: true,
            },
            ProviderEvent::Cancelled {
                provider: PROVIDER,
                attempt,
            },
            ProviderEvent::Expired {
                provider: PROVIDER,
                attempt,
            },
        ] {
            signals.extend(providers(&event));
        }
        for event in [
            BuildEvent::SlidePlaced {
                show: "s".into(),
                episode: "e".into(),
                chapter: "c".into(),
                slide: "sl".into(),
            },
            BuildEvent::EpisodeChanged {
                show: "s".into(),
                episode: "e".into(),
            },
            BuildEvent::CardAuthored {
                show: "".into(),
                episode: "".into(),
                slide: "sl".into(),
            },
        ] {
            signals.extend(build(&event));
        }
        signals
    }

    fn declared() -> Vec<SignalId> {
        declare([&SPEC], [PROVIDER])
    }

    /// The dangerous drift: publishing an id no agent was ever told about.
    #[test]
    fn every_published_signal_is_declared() {
        let declared: HashSet<SignalId> = declared().into_iter().collect();
        for signal in every_fabric_fact() {
            assert!(
                declared.contains(&signal),
                "`{signal}` is published but not declared, so nothing could discover it"
            );
        }
    }

    /// The other drift: declaring an id nothing can ever publish.
    #[test]
    fn every_declared_signal_can_actually_be_published() {
        let published: HashSet<SignalId> = every_fabric_fact().into_iter().collect();
        for signal in declared() {
            assert!(
                published.contains(&signal),
                "`{signal}` is declared but nothing publishes it"
            );
        }
    }

    #[test]
    fn every_fabric_domain_contributes_a_coarse_fact() {
        let published: HashSet<SignalId> = every_fabric_fact().into_iter().collect();
        for domain in [
            "shell.",
            "deck.",
            "capture.",
            "jobs.",
            "providers.",
            "build.",
        ] {
            assert!(
                published.iter().any(|signal| signal.0.starts_with(domain)),
                "no signal for the `{domain}` domain"
            );
        }
        for coarse in COARSE {
            assert!(
                published.contains(&SignalId::new(*coarse)),
                "`{coarse}` is listed as a coarse fact but never published"
            );
        }
    }

    /// Operator content and provider secrets must never reach an id, because
    /// the catalog is declared to the agent up front.
    #[test]
    fn ids_carry_no_operator_content_and_no_secrets() {
        for signal in every_fabric_fact() {
            for leaked in ["out.mp4", "nothing changed", "screen", "nope", "1"] {
                assert!(
                    !signal.0.ends_with(leaked),
                    "`{signal}` carries runtime payload `{leaked}`"
                );
            }
        }
        // The build domain publishes the fact, never the show or episode.
        let build_ids: Vec<SignalId> = build(&BuildEvent::SlidePlaced {
            show: "my-show".into(),
            episode: "ep-01".into(),
            chapter: "c".into(),
            slide: "sl".into(),
        });
        assert_eq!(build_ids, vec![SignalId::new(BUILD_SLIDE_PLACED)]);
    }

    #[test]
    fn routes_and_providers_come_from_the_registry_not_a_hand_list() {
        let declared = declared();
        assert!(declared.contains(&SignalId::new("shell.route-changed/workbench/library")));
        assert!(declared.contains(&SignalId::new("shell.navigate-request/workbench/library")));
        assert!(declared.contains(&SignalId::new("providers.ready/grok")));
        // An unregistered hub is not coachable, which is the point.
        assert!(!declared.contains(&SignalId::new("shell.route-changed/ghost/nowhere")));
    }

    /// The mapping functions being right is worth nothing if a domain is
    /// never subscribed. This drives a real `Fabric` end to end.
    #[test]
    fn every_domain_is_actually_subscribed_and_drained() {
        let fabric = Fabric::new();
        let mut signals = CoachSignals::subscribe(&fabric);
        fabric.publish_shell(ShellEvent::RouteChanged { route: route() });
        fabric.publish_deck(DeckEvent::Beat {
            index: 0,
            step: 0,
            slides: 1,
            steps: 1,
        });
        fabric.publish_capture(CaptureEvent::InboxChanged);
        fabric.publish_job(JobEvent::Queued {
            job: JobId(1),
            label: "render".into(),
        });
        fabric.publish_provider(ProviderEvent::Selected { provider: PROVIDER });
        fabric.publish_build(BuildEvent::EpisodeChanged {
            show: "s".into(),
            episode: "e".into(),
        });
        let drained = signals.drain();
        for expected in [
            SHELL_ROUTE_CHANGED,
            DECK_BEAT,
            CAPTURE_INBOX_CHANGED,
            JOBS_QUEUED,
            PROVIDERS_SELECTED,
            BUILD_EPISODE_CHANGED,
        ] {
            assert!(
                drained.contains(&SignalId::new(expected)),
                "`{expected}` never arrived, so its domain is not subscribed"
            );
        }
        assert!(
            signals.drain().is_empty(),
            "a drained signal must not be republished"
        );
    }

    #[test]
    fn the_catalog_is_sorted_and_free_of_duplicates() {
        let declared = declared();
        let mut sorted = declared.clone();
        sorted.sort_by(|left, right| left.0.cmp(&right.0));
        sorted.dedup();
        assert_eq!(declared, sorted);
    }
}
