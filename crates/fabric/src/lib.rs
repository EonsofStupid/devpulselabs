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

use dpl_hub::Route;
use tokio::sync::broadcast;

/// Shell-domain signals. Copy — signals are values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellEvent {
    /// Navigation happened. Consumers: future chat context providers,
    /// telemetry, anything that cares where the operator is.
    RouteChanged { route: Route },
    /// The footer Snip launcher was pressed — summon the PixVix overlay.
    /// Consumer: the pixvix crate (B4); until then the inspector's signal
    /// count is the visible proof the launcher fires.
    PixvixSummon,
    /// A hub asks the shell to enter/leave full-bleed (D2: the Present
    /// hub's deck owns the viewport while a show runs). Consumer: the
    /// shell, which drains its receiver each frame.
    FullBleedRequest(bool),
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

/// The fabric hands out senders and subscriptions; it never processes.
/// Broadcast is deliberately lossy under pressure (oldest dropped, receiver
/// told how many) — correct for signals, wrong for jobs. Job queues get
/// per-consumer mpsc when the engine lands.
/// Clone shares the SAME channels — one fabric, many holders.
#[derive(Clone)]
pub struct Fabric {
    shell: broadcast::Sender<ShellEvent>,
    deck: broadcast::Sender<DeckEvent>,
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
        Self { shell, deck }
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
            sub: SubId("pixvix"),
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
