//! Explicit route-context offers and per-provider attachments.
//!
//! Observing a route can only change the offer. Attaching is a separate
//! operator action and no API in this crate can send a provider message.

use dpl_ai_core::ProviderId;
use dpl_hub::Route;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContextSourceId(pub &'static str);

impl fmt::Display for ContextSourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContextSnapshotId(pub u64);

pub const ROUTE_SOURCE: ContextSourceId = ContextSourceId("shell-route");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteContextSnapshot {
    pub id: ContextSnapshotId,
    pub source: ContextSourceId,
    pub route: Route,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextError {
    NoOffer,
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOffer => f.write_str("no route context is currently offered"),
        }
    }
}

impl std::error::Error for ContextError {}

#[derive(Debug, Clone, Default)]
pub struct RouteContextState {
    revision: u64,
    offered: Option<RouteContextSnapshot>,
    attached: HashMap<ProviderId, RouteContextSnapshot>,
}

impl RouteContextState {
    /// Observe a route without attaching it to any provider.
    pub fn observe(&mut self, route: Route) -> RouteContextSnapshot {
        if let Some(current) = self.offered {
            if current.route == route {
                return current;
            }
        }
        self.revision = self.revision.saturating_add(1);
        let snapshot = RouteContextSnapshot {
            id: ContextSnapshotId(self.revision),
            source: ROUTE_SOURCE,
            route,
        };
        self.offered = Some(snapshot);
        snapshot
    }

    pub const fn offered(&self) -> Option<RouteContextSnapshot> {
        self.offered
    }

    pub fn attach_offered(
        &mut self,
        provider_id: ProviderId,
    ) -> Result<RouteContextSnapshot, ContextError> {
        let snapshot = self.offered.ok_or(ContextError::NoOffer)?;
        self.attached.insert(provider_id, snapshot);
        Ok(snapshot)
    }

    pub fn remove(&mut self, provider_id: ProviderId) -> Option<RouteContextSnapshot> {
        self.attached.remove(&provider_id)
    }

    pub fn attached(&self, provider_id: ProviderId) -> Option<RouteContextSnapshot> {
        self.attached.get(&provider_id).copied()
    }

    pub fn is_stale(&self, provider_id: ProviderId) -> bool {
        match (self.attached(provider_id), self.offered) {
            (Some(attached), Some(offered)) => attached.id != offered.id,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_ai_core::ProviderId;
    use dpl_hub::{HubId, SubId};

    const CODEX: ProviderId = ProviderId("codex");
    const CLAUDE: ProviderId = ProviderId("claude");
    const HOME: Route = Route {
        hub: HubId("home"),
        sub: SubId("shows"),
    };
    const WORKBENCH: Route = Route {
        hub: HubId("workbench"),
        sub: SubId("library"),
    };

    #[test]
    fn observing_a_route_never_attaches_it() {
        let mut state = RouteContextState::default();
        state.observe(HOME);
        assert_eq!(state.offered().expect("offer").route, HOME);
        assert!(state.attached(CODEX).is_none());
    }

    #[test]
    fn attachments_are_provider_isolated() {
        let mut state = RouteContextState::default();
        state.observe(HOME);
        state.attach_offered(CODEX).expect("offer");
        assert_eq!(state.attached(CODEX).expect("attached").route, HOME);
        assert!(state.attached(CLAUDE).is_none());
    }

    #[test]
    fn later_routes_mark_old_explicit_attachments_stale() {
        let mut state = RouteContextState::default();
        state.observe(HOME);
        state.attach_offered(CODEX).expect("offer");
        state.observe(WORKBENCH);
        assert!(state.is_stale(CODEX));
        assert_eq!(state.attached(CODEX).expect("old attachment").route, HOME);
        assert_eq!(state.offered().expect("new offer").route, WORKBENCH);
    }

    #[test]
    fn repeated_observation_keeps_the_same_revision() {
        let mut state = RouteContextState::default();
        let first = state.observe(HOME);
        let second = state.observe(HOME);
        assert_eq!(first.id, second.id);
    }
}
