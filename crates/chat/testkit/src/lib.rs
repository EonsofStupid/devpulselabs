//! Deterministic Chat fixtures. Production crates do not depend on this crate.

use dpl_chat_core::{
    ActivationState, Capability, ChatEvent, ChatEventContent, ChatEventId, ChatSessionId,
    ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
    SessionState, SessionSummary, TurnId,
};
use dpl_chat_runtime::ProviderRegistration;
use dpl_chat_social::{
    Person, PersonId, Presence, Project, ProjectId, SocialConversation, SocialConversationId,
    SocialMessage, SocialMessageId, SocialModel, SocialThreadKind,
};
use std::collections::BTreeSet;

pub const FIXTURE_PROVIDER: ProviderId = ProviderId("fixture");
pub const AUTHORED_ROUTE_RESULT_ID: dpl_chat_interactions::InteractionId =
    dpl_chat_interactions::InteractionId(1001);
pub const AUTHORED_APPROVAL_ID: dpl_chat_interactions::InteractionId =
    dpl_chat_interactions::InteractionId(1003);

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
        qualified_capabilities: CAPABILITIES.iter().copied().collect::<BTreeSet<_>>(),
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

#[derive(Debug, Clone)]
pub struct AuthoredChatFixture {
    pub providers: Vec<ProviderRegistration>,
    pub social: SocialModel,
    pub interactions: dpl_chat_interactions::InteractionRegistry,
}

/// Complete deterministic visual fixture for the independent Chat peer.
///
/// Nothing in this fixture claims a live provider or Social transport. The
/// app can only register it behind its test/devtools feature.
pub fn authored_chat() -> AuthoredChatFixture {
    AuthoredChatFixture {
        providers: authored_providers(),
        social: authored_social(),
        interactions: authored_interactions(),
    }
}

fn authored_providers() -> Vec<ProviderRegistration> {
    let mut codex = authored_provider(
        dpl_chat_codex::MANIFEST,
        ChatSessionId(101),
        "Steam-chat AI/Social foundation",
        SessionState::Running,
        vec![
            ChatEventContent::UserMessage(
                "Keep Chat compact, route-aware, and independent from Main.".into(),
            ),
            ChatEventContent::AssistantMessage(
                "The primary viewport now has two persistent columns. Approvals float; sustained previews snap out as native utilities.".into(),
            ),
            ChatEventContent::ToolCompleted {
                label: "Validated AI/Social state isolation".into(),
                succeeded: true,
            },
        ],
    );
    codex.activation = ActivationState::Experimental {
        reason: "Authored fixture replay; no live Codex transport.".into(),
        recovery: Vec::new(),
    };
    codex.sessions.push(SessionSummary {
        id: ChatSessionId(102),
        provider_id: dpl_chat_codex::MANIFEST.descriptor.id,
        provider_session_id: None,
        title: "Route bridge review".into(),
        workspace: "C:/fixture/devpulse".into(),
        state: SessionState::Attention,
    });
    codex.sessions.push(SessionSummary {
        id: ChatSessionId(103),
        provider_id: dpl_chat_codex::MANIFEST.descriptor.id,
        provider_session_id: None,
        title: "Library visual polish".into(),
        workspace: "C:/fixture/devpulse".into(),
        state: SessionState::Idle,
    });

    let mut claude = authored_provider(
        dpl_chat_claude::MANIFEST,
        ChatSessionId(201),
        "Architecture critique",
        SessionState::Idle,
        vec![ChatEventContent::AssistantMessage(
            "Fixture-only read view. Claude activation remains unqualified.".into(),
        )],
    );
    claude.activation = ActivationState::Blocked {
        reason: "Claude subscription adapter is not qualified.".into(),
        recovery: Vec::new(),
    };

    let mut gemini = authored_provider(
        dpl_chat_gemini::MANIFEST,
        ChatSessionId(301),
        "Multimodal research",
        SessionState::ReadOnly,
        vec![ChatEventContent::ProviderWarning(
            "Gemini transport research is incomplete.".into(),
        )],
    );
    gemini.activation = ActivationState::Experimental {
        reason: "Research fixture only; no activation path.".into(),
        recovery: Vec::new(),
    };

    let mut grok = authored_provider(
        dpl_chat_grok::MANIFEST,
        ChatSessionId(401),
        "Social signal review",
        SessionState::ReadOnly,
        Vec::new(),
    );
    grok.activation = ActivationState::SignedOut {
        reason: "Grok subscription activation is not qualified.".into(),
        recovery: Vec::new(),
    };

    vec![codex, claude, gemini, grok]
}

fn authored_provider(
    manifest: ProviderManifest,
    session_id: ChatSessionId,
    title: &str,
    state: SessionState,
    content: Vec<ChatEventContent>,
) -> ProviderRegistration {
    let provider_id = manifest.descriptor.id;
    ProviderRegistration {
        manifest,
        activation: ActivationState::Blocked {
            reason: "Authored fixture only.".into(),
            recovery: Vec::new(),
        },
        qualified_capabilities: [Capability::ReadSessions, Capability::PageHistory]
            .into_iter()
            .collect(),
        account: None,
        sessions: vec![SessionSummary {
            id: session_id,
            provider_id,
            provider_session_id: None,
            title: title.into(),
            workspace: "C:/fixture/devpulse".into(),
            state,
        }],
        events: content
            .into_iter()
            .enumerate()
            .map(|(index, content)| ChatEvent {
                id: ChatEventId(index as u64 + 1),
                provider_id,
                session_id,
                turn_id: Some(TurnId("authored-turn".into())),
                content,
            })
            .collect(),
    }
}

fn authored_social() -> SocialModel {
    let mut model = SocialModel::production();
    let operator = person(
        "operator",
        "Jessica",
        "@operator",
        Presence::Online,
        Some("Reviewing Chat foundation"),
    );
    let maya = person(
        "maya",
        "Maya Chen",
        "@maya",
        Presence::InFlow,
        Some("Polishing the Library"),
    );
    let andre = person(
        "andre",
        "Andre Silva",
        "@andre",
        Presence::Away,
        Some("Back in 12 min"),
    );
    let sam = person(
        "sam",
        "Sam Rivera",
        "@sam",
        Presence::Online,
        Some("Route bridge tests"),
    );
    for person in [operator.clone(), maya.clone(), andre, sam] {
        model.insert_person(person).expect("authored person");
    }

    let devpulse = Project {
        id: ProjectId::new("devpulse").expect("project id"),
        name: "DevPULSE".into(),
        summary: "Native creative and agent workspace".into(),
    };
    let lab = Project {
        id: ProjectId::new("egui-pilot").expect("project id"),
        name: "EguiPilotAI".into(),
        summary: "Deterministic native egui testing".into(),
    };
    model
        .insert_project(devpulse.clone())
        .expect("DevPULSE project");
    model.insert_project(lab).expect("pilot project");

    let maya_thread = SocialConversation {
        id: SocialConversationId::new("maya-dm").expect("conversation"),
        title: "Maya Chen".into(),
        kind: SocialThreadKind::Direct {
            person_id: maya.id.clone(),
        },
        unread: 2,
        pinned: true,
    };
    let product_room = SocialConversation {
        id: SocialConversationId::new("product-room").expect("conversation"),
        title: "# product-room".into(),
        kind: SocialThreadKind::Project {
            project_id: devpulse.id,
        },
        unread: 3,
        pinned: true,
    };
    let design_group = SocialConversation {
        id: SocialConversationId::new("design-council").expect("conversation"),
        title: "Design crew".into(),
        kind: SocialThreadKind::Group,
        unread: 0,
        pinned: false,
    };
    for thread in [maya_thread.clone(), product_room.clone(), design_group] {
        model
            .insert_conversation(thread)
            .expect("authored conversation");
    }
    model
        .replace_messages(
            &maya_thread.id,
            vec![
                social_message(
                    "maya-1",
                    &maya_thread.id,
                    &maya.id,
                    "The compact rail reads much more like a real chat client now.",
                    "09:41",
                ),
                social_message(
                    "maya-2",
                    &maya_thread.id,
                    &operator.id,
                    "Good. The route preview should stay snapped beside Chat, never become a permanent column.",
                    "09:43",
                ),
            ],
        )
        .expect("Maya messages");
    model
        .replace_messages(
            &product_room.id,
            vec![
                social_message(
                    "product-1",
                    &product_room.id,
                    &maya.id,
                    "AI and Social now preserve their own selected thread and draft.",
                    "10:02",
                ),
                social_message(
                    "product-2",
                    &product_room.id,
                    &operator.id,
                    "Keep send disabled until the Social transport security plan is qualified.",
                    "10:04",
                ),
            ],
        )
        .expect("project messages");
    model
}

fn person(
    id: &str,
    display_name: &str,
    handle: &str,
    presence: Presence,
    activity: Option<&str>,
) -> Person {
    Person {
        id: PersonId::new(id).expect("person id"),
        display_name: display_name.into(),
        handle: handle.into(),
        presence,
        activity: activity.map(str::to_owned),
    }
}

fn social_message(
    id: &str,
    conversation_id: &SocialConversationId,
    author_id: &PersonId,
    body: &str,
    timestamp: &str,
) -> SocialMessage {
    SocialMessage {
        id: SocialMessageId::new(id).expect("message id"),
        conversation_id: conversation_id.clone(),
        author_id: author_id.clone(),
        body: body.into(),
        timestamp: timestamp.into(),
    }
}

fn authored_interactions() -> dpl_chat_interactions::InteractionRegistry {
    use dpl_chat_interactions::{
        InteractionContent, InteractionId, InteractionLifecycle, InteractionPlacement,
        InteractionProvenance, InteractionSpec, RouteResultState,
    };

    let mut registry = dpl_chat_interactions::InteractionRegistry::default();
    for spec in [
        InteractionSpec {
            id: AUTHORED_ROUTE_RESULT_ID,
            provenance: InteractionProvenance::Host,
            required_capability: None,
            lifecycle: InteractionLifecycle::Visible,
            placement: InteractionPlacement::Inline,
            content: InteractionContent::RouteResult {
                route_label: "Library / Slide templates".into(),
                detail: "Observed from Main · context not attached".into(),
                state: RouteResultState::Opened,
            },
        },
        InteractionSpec {
            id: InteractionId(1002),
            provenance: InteractionProvenance::Host,
            required_capability: None,
            lifecycle: InteractionLifecycle::Visible,
            placement: InteractionPlacement::SnappedUtility,
            content: InteractionContent::ArtifactPreview {
                title: "AI/Social foundation".into(),
                artifact_kind: "Route preview".into(),
                summary: "Compact Chat shell with governed snap-out work".into(),
            },
        },
        InteractionSpec {
            id: AUTHORED_APPROVAL_ID,
            provenance: InteractionProvenance::Host,
            required_capability: None,
            lifecycle: InteractionLifecycle::Visible,
            placement: InteractionPlacement::Overlay,
            content: InteractionContent::Approval {
                title: "Open route in Main?".into(),
                summary:
                    "Restore Main and open Library / Slide templates. No context will be sent."
                        .into(),
                approve_label: "Restore and open".into(),
                reject_label: "Keep in Chat".into(),
            },
        },
    ] {
        registry.register(spec).expect("authored interaction");
    }
    registry
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

    #[test]
    fn authored_fixture_has_ai_social_and_governed_surfaces() {
        let fixture = authored_chat();
        assert_eq!(fixture.providers.len(), 4);
        assert_eq!(fixture.social.people().len(), 4);
        assert_eq!(fixture.social.projects().len(), 2);
        assert_eq!(fixture.social.conversations().len(), 3);
        assert_eq!(fixture.interactions.visible().count(), 3);
        assert!(!fixture.social.transport().can_send());
    }
}
