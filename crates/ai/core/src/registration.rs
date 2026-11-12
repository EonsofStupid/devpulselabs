//! A registered provider: its manifest, activation, the capabilities it has
//! qualified, and the account it is signed in with.
//!
//! Conversations are not here. Sessions and events belong to chat's runtime,
//! keyed by the provider's id.

use crate::activation::{AccountSummary, ActivationState, RecoveryAction, RecoveryActionKind};
use crate::capability::Capability;
use crate::provider::ProviderManifest;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct ProviderRegistration {
    pub manifest: ProviderManifest,
    pub activation: ActivationState,
    pub qualified_capabilities: BTreeSet<Capability>,
    pub account: Option<AccountSummary>,
}

impl ProviderRegistration {
    pub fn planned(manifest: ProviderManifest) -> Self {
        Self {
            activation: ActivationState::Blocked {
                reason: format!(
                    "{} has a reviewed integration boundary but no qualified live adapter.",
                    manifest.descriptor.label
                ),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::OpenDiagnostics,
                    format!(
                        "Complete {} and its conformance gate.",
                        manifest.delivery_slice
                    ),
                )],
            },
            manifest,
            qualified_capabilities: BTreeSet::new(),
            account: None,
        }
    }

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.qualified_capabilities.contains(&capability)
    }

    pub fn can_send(&self) -> bool {
        self.activation.is_activated()
            && self.has_capability(Capability::StreamTurns)
            && (self.has_capability(Capability::StartSession)
                || self.has_capability(Capability::ResumeSession))
    }
}
