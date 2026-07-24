//! Codex provider descriptor.
//!
//! Live app-server integration is CH4 and remains blocked on the provider
//! runtime ADR. This crate currently advertises targets, not capabilities.

use dpl_chat_core::{
    Capability, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
};

pub const PROVIDER: ProviderId = ProviderId("codex");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("codex-default");

const TARGETS: &[Capability] = &[
    Capability::ReadSessions,
    Capability::PageHistory,
    Capability::StartSession,
    Capability::ResumeSession,
    Capability::ForkSession,
    Capability::StreamTurns,
    Capability::InterruptTurn,
    Capability::SteerTurn,
    Capability::Approvals,
    Capability::Tools,
    Capability::Diffs,
    Capability::Plans,
    Capability::Attachments,
    Capability::Images,
    Capability::Models,
    Capability::Modes,
    Capability::RateLimits,
    Capability::SubscriptionAuth,
    Capability::DeviceCodeAuth,
    Capability::Headless,
];

pub const MANIFEST: ProviderManifest = ProviderManifest {
    descriptor: ProviderDescriptor {
        id: PROVIDER,
        label: "Codex",
        short_label: "Codex",
        glyph: "C",
        accent: ProviderAccent::Cyan,
        summary: "OpenAI coding agent through the official app-server client protocol.",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "CH4",
    evidence: "docs/research/chat-provider-capabilities.md#codex",
};
