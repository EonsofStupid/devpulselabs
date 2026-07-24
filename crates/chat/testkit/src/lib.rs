//! Deterministic Chat fixtures. Production crates do not depend on this crate.

use dpl_chat_core::{
    ActivationState, Capability, ChatEvent, ChatEventContent, ChatEventId, ChatSessionId,
    ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
    SessionState, SessionSummary, TurnId,
};
use dpl_chat_runtime::ProviderRegistration;

pub const FIXTURE_PROVIDER: ProviderId = ProviderId("fixture");

const CAPABILITIES: &[Capability] = &[
    Capability::ReadSessions,
    Capability::PageHistory,
    Capability::StartSession,
    Capability::ResumeSession,
    Capability::StreamTurns,
    Capability::InterruptTurn,
    Capability::Approvals,
    Capability::Tools,
    Capability::Diffs,
    Capability::Plans,
];

pub const FIXTURE_MANIFEST: ProviderManifest = ProviderManifest {
    descriptor: ProviderDescriptor {
        id: FIXTURE_PROVIDER,
        label: "Fixture",
        short_label: "Fixture",
        glyph: "F",
        accent: ProviderAccent::Lime,
        summary: "Deterministic in-memory Chat provider.",
    },
    profile: ProviderProfileId("fixture-default"),
    target_capabilities: CAPABILITIES,
    delivery_slice: "test-only",
    evidence: "dpl-chat-testkit",
};

pub fn ready_provider() -> ProviderRegistration {
    let session_id = ChatSessionId(1);
    ProviderRegistration {
        manifest: FIXTURE_MANIFEST,
        activation: ActivationState::Ready,
        account: None,
        sessions: vec![SessionSummary {
            id: session_id,
            provider_id: FIXTURE_PROVIDER,
            provider_session_id: None,
            title: "A deterministic conversation".into(),
            workspace: "C:/fixture/workspace".into(),
            state: SessionState::Idle,
        }],
        events: vec![
            ChatEvent {
                id: ChatEventId(1),
                provider_id: FIXTURE_PROVIDER,
                session_id,
                turn_id: Some(TurnId("fixture-turn-1".into())),
                content: ChatEventContent::UserMessage(
                    "Explain the route-aware context contract.".into(),
                ),
            },
            ChatEvent {
                id: ChatEventId(2),
                provider_id: FIXTURE_PROVIDER,
                session_id,
                turn_id: Some(TurnId("fixture-turn-1".into())),
                content: ChatEventContent::AssistantMessage(
                    "Route changes offer context; they never send it.".into(),
                ),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_is_ready_but_never_registered_by_production() {
        let fixture = ready_provider();
        assert!(fixture.activation.can_send());
        assert_eq!(fixture.sessions.len(), 1);
        assert_eq!(fixture.events.len(), 2);
    }
}
