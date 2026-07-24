//! Provider registry and provider-neutral Chat state.
//!
//! CH1 contains no process transport. Live provider ownership is blocked on
//! the Chat provider-runtime ADR.

use dpl_chat_core::{
    AccountSummary, ActivationState, ChatEvent, ChatSessionId, ProviderId, ProviderManifest,
    SessionSummary,
};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ProviderRegistration {
    pub manifest: ProviderManifest,
    pub activation: ActivationState,
    pub account: Option<AccountSummary>,
    pub sessions: Vec<SessionSummary>,
    pub events: Vec<ChatEvent>,
}

impl ProviderRegistration {
    pub fn planned(manifest: ProviderManifest) -> Self {
        Self {
            activation: ActivationState::Blocked {
                reason: format!(
                    "{} has a reviewed integration boundary but no qualified live adapter.",
                    manifest.descriptor.label
                ),
                action: format!(
                    "Complete {} and its conformance gate.",
                    manifest.delivery_slice
                ),
            },
            manifest,
            account: None,
            sessions: Vec::new(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ProviderEntry {
    registration: ProviderRegistration,
    active_session: Option<ChatSessionId>,
    draft: String,
}

#[derive(Debug, Clone)]
pub struct ProviderView {
    pub registration: ProviderRegistration,
    pub active_session: Option<ChatSessionId>,
    pub draft: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateProvider(ProviderId),
    UnknownProvider(ProviderId),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateProvider(id) => write!(f, "provider {id} was registered twice"),
            Self::UnknownProvider(id) => write!(f, "provider {id} is not registered"),
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Debug, Clone, Default)]
pub struct ChatRuntime {
    providers: Vec<ProviderEntry>,
    selected_provider: Option<ProviderId>,
}

impl ChatRuntime {
    pub fn try_from_registrations(
        registrations: impl IntoIterator<Item = ProviderRegistration>,
    ) -> Result<Self, RegistryError> {
        let mut seen = HashSet::new();
        let mut providers = Vec::new();

        for registration in registrations {
            let provider_id = registration.manifest.descriptor.id;
            if !seen.insert(provider_id) {
                return Err(RegistryError::DuplicateProvider(provider_id));
            }
            let active_session = registration.sessions.first().map(|session| session.id);
            providers.push(ProviderEntry {
                registration,
                active_session,
                draft: String::new(),
            });
        }

        let selected_provider = providers
            .first()
            .map(|entry| entry.registration.manifest.descriptor.id);
        Ok(Self {
            providers,
            selected_provider,
        })
    }

    pub fn provider_views(&self) -> Vec<ProviderView> {
        self.providers
            .iter()
            .map(|entry| ProviderView {
                registration: entry.registration.clone(),
                active_session: entry.active_session,
                draft: entry.draft.clone(),
            })
            .collect()
    }

    pub const fn selected_provider_id(&self) -> Option<ProviderId> {
        self.selected_provider
    }

    pub fn selected_provider(&self) -> Option<ProviderView> {
        let selected = self.selected_provider?;
        self.providers
            .iter()
            .find(|entry| entry.registration.manifest.descriptor.id == selected)
            .map(|entry| ProviderView {
                registration: entry.registration.clone(),
                active_session: entry.active_session,
                draft: entry.draft.clone(),
            })
    }

    pub fn select_provider(&mut self, provider_id: ProviderId) -> Result<(), RegistryError> {
        if self
            .providers
            .iter()
            .any(|entry| entry.registration.manifest.descriptor.id == provider_id)
        {
            self.selected_provider = Some(provider_id);
            Ok(())
        } else {
            Err(RegistryError::UnknownProvider(provider_id))
        }
    }

    pub fn select_session(
        &mut self,
        provider_id: ProviderId,
        session_id: ChatSessionId,
    ) -> Result<(), RegistryError> {
        let entry = self
            .providers
            .iter_mut()
            .find(|entry| entry.registration.manifest.descriptor.id == provider_id)
            .ok_or(RegistryError::UnknownProvider(provider_id))?;
        if entry
            .registration
            .sessions
            .iter()
            .any(|session| session.id == session_id)
        {
            entry.active_session = Some(session_id);
        }
        Ok(())
    }

    pub fn set_draft(
        &mut self,
        provider_id: ProviderId,
        draft: String,
    ) -> Result<(), RegistryError> {
        let entry = self
            .providers
            .iter_mut()
            .find(|entry| entry.registration.manifest.descriptor.id == provider_id)
            .ok_or(RegistryError::UnknownProvider(provider_id))?;
        entry.draft = draft;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::{Capability, ProviderAccent, ProviderDescriptor, ProviderProfileId};

    const ID: ProviderId = ProviderId("test");
    const OTHER: ProviderId = ProviderId("other");
    const CAPABILITIES: &[Capability] = &[Capability::ReadSessions];
    const MANIFEST: ProviderManifest = ProviderManifest {
        descriptor: ProviderDescriptor {
            id: ID,
            label: "Test",
            short_label: "T",
            glyph: "T",
            accent: ProviderAccent::Cyan,
            summary: "test provider",
        },
        profile: ProviderProfileId("default"),
        target_capabilities: CAPABILITIES,
        delivery_slice: "CH1",
        evidence: "fixture",
    };

    #[test]
    fn registration_order_is_selection_order() {
        let runtime =
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("valid registry");
        assert_eq!(runtime.selected_provider_id(), Some(ID));
    }

    #[test]
    fn duplicate_provider_ids_fail_immediately() {
        let error = ChatRuntime::try_from_registrations([
            ProviderRegistration::planned(MANIFEST),
            ProviderRegistration::planned(MANIFEST),
        ])
        .expect_err("duplicate must fail");
        assert_eq!(error, RegistryError::DuplicateProvider(ID));
    }

    #[test]
    fn provider_local_drafts_survive_tab_switches() {
        let other = ProviderManifest {
            descriptor: ProviderDescriptor {
                id: OTHER,
                label: "Other",
                short_label: "O",
                glyph: "O",
                accent: ProviderAccent::Amber,
                summary: "other provider",
            },
            ..MANIFEST
        };
        let mut runtime = ChatRuntime::try_from_registrations([
            ProviderRegistration::planned(MANIFEST),
            ProviderRegistration::planned(other),
        ])
        .expect("valid registry");

        runtime.set_draft(ID, "test draft".into()).expect("known");
        runtime.select_provider(OTHER).expect("known");
        runtime
            .set_draft(OTHER, "other draft".into())
            .expect("known");
        runtime.select_provider(ID).expect("known");

        assert_eq!(
            runtime.selected_provider().expect("selected").draft,
            "test draft"
        );
    }

    #[test]
    fn unknown_provider_selection_is_explicit() {
        let mut runtime =
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("valid registry");
        assert_eq!(
            runtime.select_provider(OTHER),
            Err(RegistryError::UnknownProvider(OTHER))
        );
    }
}
