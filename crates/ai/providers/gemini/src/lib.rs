//! Gemini provider descriptor.
//!
//! Live ACP integration is CH6, requires explicit installation, and remains
//! blocked on the provider runtime ADR.

use dpl_ai_core::provider::ProviderId;
use dpl_ai_core::usage::{ProviderUsage, TokenUsage, UsageReporting};

pub const PROVIDER: ProviderId = ProviderId("gemini");

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
