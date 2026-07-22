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
}

/// The fabric hands out senders and subscriptions; it never processes.
/// Broadcast is deliberately lossy under pressure (oldest dropped, receiver
/// told how many) — correct for signals, wrong for jobs. Job queues get
/// per-consumer mpsc when the engine lands.
pub struct Fabric {
    shell: broadcast::Sender<ShellEvent>,
}

impl Default for Fabric {
    fn default() -> Self {
        Self::new()
    }
}

impl Fabric {
    pub fn new() -> Self {
        let (shell, _) = broadcast::channel(64);
        Self { shell }
    }

    /// Publish, ignoring "no subscribers yet" — a signal with no listeners
    /// is fine; a listener that must not miss signals uses mpsc, not this.
    pub fn publish_shell(&self, event: ShellEvent) {
        let _ = self.shell.send(event);
    }

    pub fn subscribe_shell(&self) -> broadcast::Receiver<ShellEvent> {
        self.shell.subscribe()
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
        let route = Route { hub: HubId("wiredfront"), sub: SubId("pixvix") };
        fabric.publish_shell(ShellEvent::RouteChanged { route });
        assert_eq!(rx.try_recv().unwrap(), ShellEvent::RouteChanged { route });
    }

    #[test]
    fn publishing_with_no_subscribers_is_not_an_error() {
        let fabric = Fabric::new();
        fabric.publish_shell(ShellEvent::RouteChanged {
            route: Route { hub: HubId("home"), sub: SubId("shows") },
        });
        // No panic, no Result to mishandle — the signal simply had no audience.
    }
}
