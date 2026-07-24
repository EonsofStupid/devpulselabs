//! Claude provider descriptor.
//!
//! Live structured-stream integration is CH5 and remains blocked on the
//! provider runtime ADR.

use dpl_chat_core::{
    Capability, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
};

pub const PROVIDER: ProviderId = ProviderId("claude");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("claude-default");

const TARGETS: &[Capability] = &[
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
    Capability::Attachments,
    Capability::Models,
    Capability::Modes,
    Capability::SubscriptionAuth,
    Capability::Headless,
];

pub const MANIFEST: ProviderManifest = ProviderManifest {
    descriptor: ProviderDescriptor {
        id: PROVIDER,
        label: "Claude",
        short_label: "Claude",
        glyph: "A",
        accent: ProviderAccent::Amber,
        summary: "Anthropic coding agent through its documented structured CLI stream.",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "CH5",
    evidence: "docs/research/chat-provider-capabilities.md#claude",
};
