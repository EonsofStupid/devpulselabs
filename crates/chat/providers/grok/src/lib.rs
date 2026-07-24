//! Grok provider descriptor.
//!
//! Live ACP integration is experimental CH7 work, requires explicit
//! installation, and remains blocked on the provider runtime ADR.

use dpl_chat_core::{
    Capability, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
};

pub const PROVIDER: ProviderId = ProviderId("grok");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("grok-default");

const TARGETS: &[Capability] = &[
    Capability::ReadSessions,
    Capability::StartSession,
    Capability::ResumeSession,
    Capability::StreamTurns,
    Capability::InterruptTurn,
    Capability::Approvals,
    Capability::Tools,
    Capability::Diffs,
    Capability::Plans,
    Capability::Models,
    Capability::Modes,
    Capability::SubscriptionAuth,
    Capability::DeviceCodeAuth,
    Capability::Headless,
];

pub const MANIFEST: ProviderManifest = ProviderManifest {
    descriptor: ProviderDescriptor {
        id: PROVIDER,
        label: "Grok",
        short_label: "Grok",
        glyph: "X",
        accent: ProviderAccent::Magenta,
        summary: "xAI coding agent through Grok Build's documented ACP mode.",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "CH7",
    evidence: "docs/research/chat-provider-capabilities.md#grok",
};
