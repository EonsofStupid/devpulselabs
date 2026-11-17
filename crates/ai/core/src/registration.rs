//! A registered provider: its manifest, activation, the capabilities it has
//! qualified, and the account it is signed in with.
//!
//! Conversations are not here. Sessions and events belong to chat's runtime,
//! keyed by the provider's id.

use crate::activation::{AccountSummary, ActivationState, RecoveryAction, RecoveryActionKind};
use crate::capability::Capability;
use crate::provider::ProviderManifest;
use std::collections::BTreeSet;

/// What a scan of this computer saw for a provider (AH1a). Sign-in is not
/// here: it is the vendor's own answer, carried by the activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OnMachine {
    /// One of the install locations the catalog names exists.
    pub installed: bool,
    /// A process matching one of its tells is running.
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct ProviderRegistration {
    pub manifest: ProviderManifest,
    pub activation: ActivationState,
    pub qualified_capabilities: BTreeSet<Capability>,
    pub account: Option<AccountSummary>,
    /// `None` until the machine scan has answered for this provider.
    pub on_machine: Option<OnMachine>,
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
            on_machine: None,
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
