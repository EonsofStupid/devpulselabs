//! Provider registry and provider-neutral Chat state.
//!
//! CH1 contained no process transport, and live provider ownership was
//! blocked on the Chat provider-runtime ADR. [`seam`] is that seam: one
//! provider-neutral runtime vocabulary every live provider implements, so the
//! host drives them all through one path and emits ONE set of signals.

use dpl_chat_core::{
    AccountSummary, ActivationState, Capability, ChatEvent, ChatEventContent, ChatEventId,
    ChatSessionId, ProviderId, ProviderManifest, RecoveryAction, RecoveryActionKind, SessionGroup,
    SessionState, SessionSummary, TurnId,
};
pub mod seam;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ProviderRegistration {
    pub manifest: ProviderManifest,
    pub activation: ActivationState,
    pub qualified_capabilities: BTreeSet<Capability>,
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
            sessions: Vec::new(),
            events: Vec::new(),
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

#[derive(Debug, Clone)]
struct ProviderEntry {
    registration: ProviderRegistration,
    active_session: Option<ChatSessionId>,
    drafts: BTreeMap<Option<ChatSessionId>, String>,
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
    UnknownSession {
        provider: ProviderId,
        session: ChatSessionId,
    },
    CapabilityNotDeclared {
        provider: ProviderId,
        capability: Capability,
    },
    SessionProviderMismatch {
        expected: ProviderId,
        actual: ProviderId,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateProvider(id) => write!(f, "provider {id} was registered twice"),
            Self::UnknownProvider(id) => write!(f, "provider {id} is not registered"),
            Self::UnknownSession { provider, session } => {
                write!(f, "session {session} is not registered for {provider}")
            }
            Self::CapabilityNotDeclared {
                provider,
                capability,
            } => write!(f, "{provider} did not declare capability {capability:?}"),
            Self::SessionProviderMismatch { expected, actual } => {
                write!(f, "session belongs to {actual}, not {expected}")
            }
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
                drafts: BTreeMap::new(),
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
                draft: entry
                    .drafts
                    .get(&entry.active_session)
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect()
    }

    pub fn provider_view(&self, provider_id: ProviderId) -> Option<ProviderView> {
        self.providers
            .iter()
            .find(|entry| entry.registration.manifest.descriptor.id == provider_id)
            .map(Self::view)
    }

    pub const fn selected_provider_id(&self) -> Option<ProviderId> {
        self.selected_provider
    }

    pub fn selected_provider(&self) -> Option<ProviderView> {
        let selected = self.selected_provider?;
        self.providers
            .iter()
            .find(|entry| entry.registration.manifest.descriptor.id == selected)
            .map(Self::view)
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
        let session = entry
            .registration
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
            .ok_or(RegistryError::UnknownSession {
                provider: provider_id,
                session: session_id,
            })?;
        if session.group != SessionGroup::Pinned {
            session.group = SessionGroup::Active;
        }
        entry.active_session = Some(session_id);
        self.selected_provider = Some(provider_id);
        Ok(())
    }

    pub fn upsert_session(
        &mut self,
        provider_id: ProviderId,
        session: SessionSummary,
    ) -> Result<(), RegistryError> {
        if session.provider_id != provider_id {
            return Err(RegistryError::SessionProviderMismatch {
                expected: provider_id,
                actual: session.provider_id,
            });
        }
        let entry = self.entry_mut(provider_id)?;
        if let Some(existing) = entry
            .registration
            .sessions
            .iter_mut()
            .find(|existing| existing.id == session.id)
        {
            *existing = session;
        } else {
            entry.registration.sessions.push(session);
        }
        if entry.active_session.is_none() {
            entry.active_session = entry.registration.sessions.first().map(|item| item.id);
        }
        Ok(())
    }

    pub fn set_session_state(
        &mut self,
        provider_id: ProviderId,
        session_id: ChatSessionId,
        state: SessionState,
    ) -> Result<(), RegistryError> {
        let entry = self.entry_mut(provider_id)?;
        let session = entry
            .registration
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
            .ok_or(RegistryError::UnknownSession {
                provider: provider_id,
                session: session_id,
            })?;
        session.state = state;
        Ok(())
    }

    pub fn append_event(
        &mut self,
        provider_id: ProviderId,
        event: ChatEvent,
    ) -> Result<(), RegistryError> {
        if event.provider_id != provider_id {
            return Err(RegistryError::SessionProviderMismatch {
                expected: provider_id,
                actual: event.provider_id,
            });
        }
        let entry = self.entry_mut(provider_id)?;
        if !entry
            .registration
            .sessions
            .iter()
            .any(|session| session.id == event.session_id)
        {
            return Err(RegistryError::UnknownSession {
                provider: provider_id,
                session: event.session_id,
            });
        }
        entry.registration.events.push(event);
        Ok(())
    }

    pub fn append_assistant_chunk(
        &mut self,
        provider_id: ProviderId,
        session_id: ChatSessionId,
        turn_id: TurnId,
        event_id: ChatEventId,
        chunk: &str,
    ) -> Result<(), RegistryError> {
        let entry = self.entry_mut(provider_id)?;
        if !entry
            .registration
            .sessions
            .iter()
            .any(|session| session.id == session_id)
        {
            return Err(RegistryError::UnknownSession {
                provider: provider_id,
                session: session_id,
            });
        }
        if let Some(last) = entry.registration.events.last_mut() {
            if last.session_id == session_id && last.turn_id.as_ref() == Some(&turn_id) {
                if let ChatEventContent::AssistantMessage(body) = &mut last.content {
                    body.push_str(chunk);
                    return Ok(());
                }
            }
        }
        entry.registration.events.push(ChatEvent {
            id: event_id,
            provider_id,
            session_id,
            turn_id: Some(turn_id),
            content: ChatEventContent::AssistantMessage(chunk.to_owned()),
        });
        Ok(())
    }

    pub fn append_status_chunk(
        &mut self,
        provider_id: ProviderId,
        session_id: ChatSessionId,
        turn_id: TurnId,
        event_id: ChatEventId,
        chunk: &str,
    ) -> Result<(), RegistryError> {
        const MAX_ACTIVITY_BYTES: usize = 16 * 1_024;
        let entry = self.entry_mut(provider_id)?;
        if !entry
            .registration
            .sessions
            .iter()
            .any(|session| session.id == session_id)
        {
            return Err(RegistryError::UnknownSession {
                provider: provider_id,
                session: session_id,
            });
        }
        if let Some(last) = entry.registration.events.last_mut() {
            if last.session_id == session_id && last.turn_id.as_ref() == Some(&turn_id) {
                if let ChatEventContent::StatusUpdated(body) = &mut last.content {
                    let remaining = MAX_ACTIVITY_BYTES.saturating_sub(body.len());
                    if remaining > 0 {
                        let mut boundary = chunk.len().min(remaining);
                        while !chunk.is_char_boundary(boundary) {
                            boundary -= 1;
                        }
                        body.push_str(&chunk[..boundary]);
                    }
                    return Ok(());
                }
            }
        }
        let mut boundary = chunk.len().min(MAX_ACTIVITY_BYTES);
        while !chunk.is_char_boundary(boundary) {
            boundary -= 1;
        }
        entry.registration.events.push(ChatEvent {
            id: event_id,
            provider_id,
            session_id,
            turn_id: Some(turn_id),
            content: ChatEventContent::StatusUpdated(chunk[..boundary].to_owned()),
        });
        Ok(())
    }

    pub fn replace_sessions(
        &mut self,
        provider_id: ProviderId,
        sessions: Vec<SessionSummary>,
    ) -> Result<(), RegistryError> {
        let entry = self
            .providers
            .iter_mut()
            .find(|entry| entry.registration.manifest.descriptor.id == provider_id)
            .ok_or(RegistryError::UnknownProvider(provider_id))?;
        let valid_active = entry
            .active_session
            .is_some_and(|active| sessions.iter().any(|session| session.id == active));
        entry.registration.sessions = sessions;
        if !valid_active {
            entry.active_session = entry
                .registration
                .sessions
                .first()
                .map(|session| session.id);
        }
        Ok(())
    }

    pub fn qualify_capabilities(
        &mut self,
        provider_id: ProviderId,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Result<(), RegistryError> {
        let entry = self
            .providers
            .iter_mut()
            .find(|entry| entry.registration.manifest.descriptor.id == provider_id)
            .ok_or(RegistryError::UnknownProvider(provider_id))?;
        for capability in capabilities {
            if !entry
                .registration
                .manifest
                .target_capabilities
                .contains(&capability)
            {
                return Err(RegistryError::CapabilityNotDeclared {
                    provider: provider_id,
                    capability,
                });
            }
            entry.registration.qualified_capabilities.insert(capability);
        }
        Ok(())
    }

    pub fn set_activation(
        &mut self,
        provider_id: ProviderId,
        activation: ActivationState,
    ) -> Result<(), RegistryError> {
        let entry = self
            .providers
            .iter_mut()
            .find(|entry| entry.registration.manifest.descriptor.id == provider_id)
            .ok_or(RegistryError::UnknownProvider(provider_id))?;
        entry.registration.activation = activation;
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
        entry.drafts.insert(entry.active_session, draft);
        Ok(())
    }

    pub fn clear_draft(&mut self, provider_id: ProviderId) -> Result<(), RegistryError> {
        self.set_draft(provider_id, String::new())
    }

    fn entry_mut(&mut self, provider_id: ProviderId) -> Result<&mut ProviderEntry, RegistryError> {
        self.providers
            .iter_mut()
            .find(|entry| entry.registration.manifest.descriptor.id == provider_id)
            .ok_or(RegistryError::UnknownProvider(provider_id))
    }

    fn view(entry: &ProviderEntry) -> ProviderView {
        ProviderView {
            registration: entry.registration.clone(),
            active_session: entry.active_session,
            draft: entry
                .drafts
                .get(&entry.active_session)
                .cloned()
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::{
        Capability, ProviderAccent, ProviderDescriptor, ProviderProfileId, ProviderSessionId,
    };

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
            since: "2000-01-01",
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
                since: "2000-01-01",
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

    #[test]
    fn declared_capabilities_do_not_become_qualified_implicitly() {
        let mut runtime =
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("valid registry");
        assert!(!runtime
            .selected_provider()
            .expect("provider")
            .registration
            .has_capability(Capability::ReadSessions));
        runtime
            .qualify_capabilities(ID, [Capability::ReadSessions])
            .expect("declared capability");
        assert!(runtime
            .selected_provider()
            .expect("provider")
            .registration
            .has_capability(Capability::ReadSessions));
        assert_eq!(
            runtime.qualify_capabilities(ID, [Capability::StreamTurns]),
            Err(RegistryError::CapabilityNotDeclared {
                provider: ID,
                capability: Capability::StreamTurns,
            })
        );
    }

    #[test]
    fn activation_updates_do_not_replace_provider_local_state() {
        let mut runtime =
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("valid registry");
        runtime.set_draft(ID, "kept draft".into()).expect("draft");
        runtime
            .set_activation(ID, ActivationState::Ready)
            .expect("activation");

        let provider = runtime.selected_provider().expect("selected provider");
        assert!(matches!(
            provider.registration.activation,
            ActivationState::Ready
        ));
        assert_eq!(provider.draft, "kept draft");
        assert!(!provider.registration.can_send());
    }

    #[test]
    fn selecting_an_unknown_session_fails_instead_of_silently_ignoring_it() {
        let mut runtime =
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("valid registry");
        assert_eq!(
            runtime.select_session(ID, ChatSessionId(99)),
            Err(RegistryError::UnknownSession {
                provider: ID,
                session: ChatSessionId(99),
            })
        );
    }

    #[test]
    fn selecting_a_shared_rail_session_also_selects_its_provider() {
        let other = ProviderManifest {
            descriptor: ProviderDescriptor {
                id: OTHER,
                label: "Other",
                short_label: "O",
                glyph: "O",
                accent: ProviderAccent::Amber,
                summary: "other provider",
                since: "2000-01-01",
            },
            ..MANIFEST
        };
        let mut other_registration = ProviderRegistration::planned(other);
        other_registration
            .sessions
            .push(session(OTHER, 7, "Other session"));
        let mut runtime = ChatRuntime::try_from_registrations([
            ProviderRegistration::planned(MANIFEST),
            other_registration,
        ])
        .expect("valid registry");

        runtime
            .select_session(OTHER, ChatSessionId(7))
            .expect("shared-rail session");

        assert_eq!(runtime.selected_provider_id(), Some(OTHER));
        assert_eq!(
            runtime
                .selected_provider()
                .expect("selected")
                .active_session,
            Some(ChatSessionId(7))
        );
        assert_eq!(
            runtime
                .selected_provider()
                .expect("selected")
                .registration
                .sessions[0]
                .group,
            SessionGroup::Active
        );
    }

    #[test]
    fn streamed_assistant_chunks_coalesce_into_one_message() {
        let mut registration = ProviderRegistration::planned(MANIFEST);
        registration.sessions.push(session(ID, 1, "Live"));
        let mut runtime =
            ChatRuntime::try_from_registrations([registration]).expect("valid registry");

        runtime
            .append_assistant_chunk(
                ID,
                ChatSessionId(1),
                TurnId("turn-1".into()),
                ChatEventId(1),
                "hello",
            )
            .expect("first chunk");
        runtime
            .append_assistant_chunk(
                ID,
                ChatSessionId(1),
                TurnId("turn-1".into()),
                ChatEventId(2),
                " world",
            )
            .expect("second chunk");

        let events = runtime
            .selected_provider()
            .expect("provider")
            .registration
            .events;
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].content,
            ChatEventContent::AssistantMessage("hello world".into())
        );
    }

    #[test]
    fn streamed_reasoning_chunks_coalesce_into_one_bounded_activity_event() {
        let mut registration = ProviderRegistration::planned(MANIFEST);
        registration.sessions.push(session(ID, 1, "Live"));
        let mut runtime =
            ChatRuntime::try_from_registrations([registration]).expect("valid registry");

        runtime
            .append_status_chunk(
                ID,
                ChatSessionId(1),
                TurnId("turn-1".into()),
                ChatEventId(1),
                "The ",
            )
            .expect("first chunk");
        runtime
            .append_status_chunk(
                ID,
                ChatSessionId(1),
                TurnId("turn-1".into()),
                ChatEventId(2),
                "user is asking",
            )
            .expect("second chunk");

        let events = runtime
            .selected_provider()
            .expect("provider")
            .registration
            .events;
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].content,
            ChatEventContent::StatusUpdated("The user is asking".into())
        );
    }

    fn session(provider_id: ProviderId, id: u64, title: &str) -> SessionSummary {
        SessionSummary {
            id: ChatSessionId(id),
            provider_id,
            provider_session_id: Some(ProviderSessionId(format!("provider-{id}"))),
            title: title.into(),
            workspace: "C:\\workspace".into(),
            group: dpl_chat_core::SessionGroup::Recent,
            state: SessionState::Idle,
        }
    }
}
