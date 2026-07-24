//! Declarative Chat extension placement and trigger registry.
//!
//! Extensions contribute typed metadata and intents. This crate intentionally
//! exposes no egui callback, provider handle, filesystem handle, or shell API.

use dpl_chat_core::{Capability, ProviderId};
use dpl_hub::{HubId, Route, SubId};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatExtensionId(pub &'static str);

impl fmt::Display for ChatExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChatSurface {
    ProviderBarAction,
    SessionRailSection,
    ConversationRenderer,
    ComposerAction,
    ContextInspectorSection,
    Overlay,
    StatusDock,
    AttachedWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutePredicate {
    pub hub: Option<HubId>,
    pub sub: Option<SubId>,
}

impl RoutePredicate {
    pub const ANY: Self = Self {
        hub: None,
        sub: None,
    };

    pub fn matches(self, route: Route) -> bool {
        let hub_matches = match self.hub {
            Some(hub) => hub.0 == route.hub.0,
            None => true,
        };
        let sub_matches = match self.sub {
            Some(sub) => sub.0 == route.sub.0,
            None => true,
        };
        hub_matches && sub_matches
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionManifest {
    pub id: ChatExtensionId,
    pub label: &'static str,
    pub description: &'static str,
    pub surfaces: &'static [ChatSurface],
    pub route: RoutePredicate,
    pub provider: Option<ProviderId>,
    pub requires: &'static [Capability],
    pub priority: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionView {
    pub manifest: ExtensionManifest,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
struct ExtensionEntry {
    manifest: ExtensionManifest,
    pinned: bool,
    dismissed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionError {
    Duplicate(ChatExtensionId),
    Unknown(ChatExtensionId),
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(id) => write!(f, "Chat extension {id} was registered twice"),
            Self::Unknown(id) => write!(f, "Chat extension {id} is not registered"),
        }
    }
}

impl std::error::Error for ExtensionError {}

#[derive(Debug, Clone, Default)]
pub struct ExtensionRegistry {
    entries: Vec<ExtensionEntry>,
}

impl ExtensionRegistry {
    pub fn try_from_manifests(
        manifests: impl IntoIterator<Item = ExtensionManifest>,
    ) -> Result<Self, ExtensionError> {
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        for manifest in manifests {
            if !seen.insert(manifest.id) {
                return Err(ExtensionError::Duplicate(manifest.id));
            }
            entries.push(ExtensionEntry {
                manifest,
                pinned: false,
                dismissed: false,
            });
        }
        entries.sort_by_key(|entry| (entry.manifest.priority, entry.manifest.id));
        Ok(Self { entries })
    }

    pub fn active_for(
        &self,
        route: Route,
        provider: ProviderId,
        capabilities: &[Capability],
    ) -> Vec<ExtensionView> {
        self.entries
            .iter()
            .filter(|entry| !entry.dismissed)
            .filter(|entry| entry.manifest.route.matches(route))
            .filter(|entry| entry.manifest.provider.is_none_or(|id| id == provider))
            .filter(|entry| {
                entry
                    .manifest
                    .requires
                    .iter()
                    .all(|required| capabilities.contains(required))
            })
            .map(|entry| ExtensionView {
                manifest: entry.manifest,
                pinned: entry.pinned,
            })
            .collect()
    }

    pub fn pin(&mut self, id: ChatExtensionId, pinned: bool) -> Result<(), ExtensionError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.manifest.id == id)
            .ok_or(ExtensionError::Unknown(id))?;
        entry.pinned = pinned;
        Ok(())
    }

    pub fn dismiss(&mut self, id: ChatExtensionId) -> Result<(), ExtensionError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.manifest.id == id)
            .ok_or(ExtensionError::Unknown(id))?;
        entry.dismissed = true;
        Ok(())
    }

    pub fn restore(&mut self, id: ChatExtensionId) -> Result<(), ExtensionError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.manifest.id == id)
            .ok_or(ExtensionError::Unknown(id))?;
        entry.dismissed = false;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: HubId = HubId("home");
    const SHOWS: SubId = SubId("shows");
    const CODEX: ProviderId = ProviderId("codex");
    const NEEDS_DIFFS: &[Capability] = &[Capability::Diffs];
    const SURFACES: &[ChatSurface] = &[ChatSurface::ComposerAction];
    const MANIFEST: ExtensionManifest = ExtensionManifest {
        id: ChatExtensionId("review-this"),
        label: "Review this",
        description: "Offer a review action on Home / Shows.",
        surfaces: SURFACES,
        route: RoutePredicate {
            hub: Some(HOME),
            sub: Some(SHOWS),
        },
        provider: Some(CODEX),
        requires: NEEDS_DIFFS,
        priority: 10,
    };
    const ROUTE: Route = Route {
        hub: HOME,
        sub: SHOWS,
    };

    #[test]
    fn duplicate_extension_ids_fail_immediately() {
        assert_eq!(
            ExtensionRegistry::try_from_manifests([MANIFEST, MANIFEST])
                .expect_err("duplicate must fail"),
            ExtensionError::Duplicate(MANIFEST.id)
        );
    }

    #[test]
    fn route_provider_and_capability_must_all_match() {
        let registry = ExtensionRegistry::try_from_manifests([MANIFEST]).expect("registry");
        assert!(registry.active_for(ROUTE, CODEX, &[]).is_empty());
        assert_eq!(
            registry
                .active_for(ROUTE, CODEX, &[Capability::Diffs])
                .len(),
            1
        );
        assert!(registry
            .active_for(ROUTE, ProviderId("claude"), &[Capability::Diffs])
            .is_empty());
    }

    #[test]
    fn dismissal_and_restore_are_explicit() {
        let mut registry = ExtensionRegistry::try_from_manifests([MANIFEST]).expect("registry");
        registry.dismiss(MANIFEST.id).expect("known");
        assert!(registry
            .active_for(ROUTE, CODEX, &[Capability::Diffs])
            .is_empty());
        registry.restore(MANIFEST.id).expect("known");
        assert_eq!(
            registry
                .active_for(ROUTE, CODEX, &[Capability::Diffs])
                .len(),
            1
        );
    }
}
