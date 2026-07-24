//! Gemini provider descriptor.
//!
//! Live ACP integration is CH6, requires explicit installation, and remains
//! blocked on the provider runtime ADR.

use dpl_chat_core::{
    Capability, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
};

pub const PROVIDER: ProviderId = ProviderId("gemini");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("gemini-default");

const TARGETS: &[Capability] = &[
    Capability::ReadSessions,
    Capability::StartSession,
    Capability::ResumeSession,
    Capability::StreamTurns,
    Capability::InterruptTurn,
    Capability::Approvals,
    Capability::Tools,
    Capability::Attachments,
    Capability::Images,
    Capability::Models,
    Capability::Modes,
    Capability::SubscriptionAuth,
    Capability::Headless,
];

pub const MANIFEST: ProviderManifest = ProviderManifest {
    descriptor: ProviderDescriptor {
        id: PROVIDER,
        label: "Gemini",
        short_label: "Gemini",
        glyph: "G",
        accent: ProviderAccent::Violet,
        summary: "Google coding agent through Gemini CLI's official ACP mode.",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "CH6",
    evidence: "docs/research/chat-provider-capabilities.md#gemini",
};
