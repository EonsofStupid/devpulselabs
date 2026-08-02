//! Gemini provider descriptor.
//!
//! Live ACP integration is CH6, requires explicit installation, and remains
//! blocked on the provider runtime ADR.

use dpl_chat_core::{
    Capability, PresenceSpec, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest,
    ProviderProfileId,
};

pub const PROVIDER: ProviderId = ProviderId("gemini");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("gemini-default");

/// Gemini CLI is not installed on the operator's machine, so these needles
/// are UNVERIFIED against a real signed-in run — the probe reports
/// `NotInstalled` today, which is the honest answer, and the needles are
/// confirmed the first time the CLI is present.
pub const PRESENCE: PresenceSpec = PresenceSpec {
    program: "gemini",
    args: &["--version"],
    signed_in: &["gemini"],
    signed_out: &["not authenticated", "please sign in"],
    // Gemini CLI is absent here; its sign-in verb is unverified, so none is
    // declared rather than guessed.
    login: &[],
};

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
    Capability::Video,
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
