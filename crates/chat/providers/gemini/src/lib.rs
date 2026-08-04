//! Gemini provider descriptor.
//!
//! Live ACP integration is CH6, requires explicit installation, and remains
//! blocked on the provider runtime ADR.

use dpl_ai_core::{
    Capability, PresenceSpec, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest,
    ProviderProfileId,
};
use dpl_chat_core::TokenUsage;
use dpl_chat_store::{ProviderUsage, UsageReporting};

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
        // Gemini's public launch; Google DeepMind itself is far older.
        since: "2023-12-06",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "CH6",
    evidence: "docs/research/chat-provider-capabilities.md#gemini",
};

/// Gemini's usage dialect — **nothing yet**.
///
/// Gemini is a descriptor today: no history decoder, no live runtime, no
/// usage surface. Declaring `NOTHING` is a truthful state, not a stub
/// that lies: a surface can say "Gemini does not report usage" instead of
/// showing a silent zero, and the census below proves the gap is KNOWN
/// rather than forgotten.
///
/// When Gemini lands, this is the one file that changes.
#[derive(Debug, Clone, Copy, Default)]
pub struct GeminiUsage;

impl ProviderUsage for GeminiUsage {
    fn provider_id(&self) -> ProviderId {
        PROVIDER
    }

    fn reports(&self) -> UsageReporting {
        UsageReporting::NOTHING
    }

    fn usage_from_line(&self, _line: &str) -> Option<TokenUsage> {
        None
    }
}
