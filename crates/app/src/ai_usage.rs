//! THE USAGE REGISTRY — every provider's usage adapter, in one census.
//!
//! One concern: name every `ProviderUsage` adapter the app knows, so
//! "which providers report what" is a lookup rather than a hunt through
//! four crates.
//!
//! Adding the next provider is three steps and the census enforces the
//! third: implement `ProviderUsage` in its crate, add its manifest to the
//! app's provider list, add it HERE. Miss the last one and
//! `every_provider_has_a_usage_adapter` fails — the "grow as the releases
//! come out" property is a test, not a hope.

use dpl_chat_core::{ProviderId, ProviderManifest};
use dpl_chat_store::{ProviderUsage, UsageReporting};

/// Every provider manifest the app ships. The census pins the usage
/// registry against THIS list, so a new provider cannot arrive with its
/// accounting silently missing.
pub const PROVIDER_MANIFESTS: &[ProviderManifest] = &[
    dpl_chat_codex::MANIFEST,
    dpl_chat_claude::MANIFEST,
    dpl_chat_grok::MANIFEST,
    dpl_chat_gemini::MANIFEST,
];

/// Every provider's usage adapter, one per major.
///
/// A provider with nothing to report still appears here declaring
/// `UsageReporting::NOTHING` — absence is stated, never implied.
pub fn usage_adapters() -> &'static [&'static dyn ProviderUsage] {
    const CODEX: dpl_chat_codex::CodexUsage = dpl_chat_codex::CodexUsage;
    const CLAUDE: dpl_chat_claude::ClaudeUsage = dpl_chat_claude::ClaudeUsage;
    const GROK: dpl_chat_grok::GrokUsage = dpl_chat_grok::GrokUsage;
    const GEMINI: dpl_chat_gemini::GeminiUsage = dpl_chat_gemini::GeminiUsage;
    &[&CODEX, &CLAUDE, &GROK, &GEMINI]
}

/// The adapter for one provider, if it has one.
pub fn usage_for(provider: ProviderId) -> Option<&'static dyn ProviderUsage> {
    usage_adapters()
        .iter()
        .copied()
        .find(|adapter| adapter.provider_id() == provider)
}

/// Providers whose already-stored sessions can be walked for usage —
/// the piggyback set (Phase 3 §3.2c). Live-only transports are excluded
/// because their usage exists on the wire and nowhere else.
pub fn backfillable() -> impl Iterator<Item = &'static dyn ProviderUsage> {
    usage_adapters()
        .iter()
        .copied()
        .filter(|adapter| adapter.reports().history_backfill)
}

/// What a provider reports, as one operator-readable line. Used wherever
/// a surface would otherwise render a silent zero.
pub fn reporting_summary(reports: UsageReporting) -> String {
    if !reports.reports_anything() {
        return "does not report usage".into();
    }
    let mut parts = Vec::new();
    if reports.input_output_split {
        parts.push("input/output split");
    }
    if reports.total_tokens {
        parts.push("total tokens");
    }
    if reports.context_window {
        parts.push("context window");
    }
    if reports.provider_priced {
        parts.push("provider-priced");
    }
    parts.join(" · ")
}

/// NAME the usage census in the boot log (the honesty law, ADR-014 §5).
///
/// The operator can see, without reading code, which providers account
/// for their tokens and which do not — so an empty cost column reads as
/// "Gemini does not report usage" rather than "the analytics are broken".
pub fn log_census() {
    // Walk the MANIFESTS, not the adapters: the manifest list is the
    // anchor, so a provider that shipped without an adapter shows up here
    // as a loud gap instead of simply being absent from the log.
    for manifest in PROVIDER_MANIFESTS {
        let id = manifest.descriptor.id;
        let Some(adapter) = usage_for(id) else {
            tracing::warn!(
                provider = %id,
                label = manifest.descriptor.label,
                "provider has NO usage adapter — its tokens and cost are invisible",
            );
            continue;
        };
        let reports = adapter.reports();
        tracing::info!(
            provider = %id,
            label = manifest.descriptor.label,
            reports = %reporting_summary(reports),
            backfill = reports.history_backfill,
            "provider usage accounting",
        );
    }
    let backfill: Vec<&str> = backfillable().map(|a| a.provider_id().0).collect();
    tracing::info!(
        providers = ?backfill,
        "sessions already on disk can be walked for past usage",
    );
}

/// Did this provider promise more than it delivered?
///
/// A provider that DECLARES an input/output split and then reports
/// nothing for a whole call has almost certainly changed its payload
/// shape in a release. That is exactly the drift this seam exists to
/// catch — surfaced as a warning instead of a silent zero.
pub fn note_usage_drift(provider: ProviderId, usage: dpl_chat_core::TokenUsage) {
    let Some(adapter) = usage_for(provider) else {
        return;
    };
    let reports = adapter.reports();
    if reports.reports_anything() && usage.is_empty() {
        tracing::warn!(
            %provider,
            expected = %reporting_summary(reports),
            "provider declared usage reporting but a whole call carried none —              its payload shape may have changed in a release",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_has_a_usage_adapter() {
        for manifest in PROVIDER_MANIFESTS {
            let id = manifest.descriptor.id;
            assert!(
                usage_for(id).is_some(),
                "provider `{id}` ships with no usage adapter — implement \
                 `ProviderUsage` in its crate and register it in \
                 `usage_adapters()`. A provider that reports nothing still \
                 registers, declaring `UsageReporting::NOTHING`."
            );
        }
    }

    #[test]
    fn the_registry_names_each_provider_once() {
        let mut seen = Vec::new();
        for adapter in usage_adapters() {
            let id = adapter.provider_id();
            assert!(!seen.contains(&id), "provider `{id}` registered twice");
            seen.push(id);
        }
        assert_eq!(
            seen.len(),
            PROVIDER_MANIFESTS.len(),
            "the usage registry and the manifest list have drifted"
        );
    }

    #[test]
    fn each_adapter_declares_what_it_actually_reports() {
        // Codex: full split, backfillable from session files, no pricing.
        let codex = usage_for(dpl_chat_codex::PROVIDER).expect("codex");
        let r = codex.reports();
        assert!(r.input_output_split && r.total_tokens && r.model);
        assert!(r.history_backfill, "codex sessions are JSONL on disk");
        assert!(!r.provider_priced, "codex does not price its own calls");

        // Claude: the one major that reports REAL MONEY.
        let claude = usage_for(dpl_chat_claude::PROVIDER).expect("claude");
        let r = claude.reports();
        assert!(r.provider_priced, "claude reports total_cost_usd");
        assert!(r.input_output_split && r.history_backfill);

        // Grok: context occupancy only, and LIVE-ONLY.
        let grok = usage_for(dpl_chat_grok::PROVIDER).expect("grok");
        let r = grok.reports();
        assert!(r.context_window);
        assert!(
            !r.input_output_split,
            "grok reports occupancy, never a split — inventing one would fabricate numbers"
        );
        assert!(
            !r.history_backfill,
            "grok usage lives on the ACP wire and nowhere else"
        );

        // Gemini: nothing yet — stated, not implied.
        let gemini = usage_for(dpl_chat_gemini::PROVIDER).expect("gemini");
        assert_eq!(gemini.reports(), UsageReporting::NOTHING);
        assert_eq!(reporting_summary(gemini.reports()), "does not report usage");
    }

    #[test]
    fn the_backfill_set_is_exactly_the_file_backed_providers() {
        let ids: Vec<ProviderId> = backfillable().map(ProviderUsage::provider_id).collect();
        assert!(ids.contains(&dpl_chat_codex::PROVIDER));
        assert!(ids.contains(&dpl_chat_claude::PROVIDER));
        assert!(
            !ids.contains(&dpl_chat_grok::PROVIDER),
            "a live-only provider must never be walked for history"
        );
        assert!(!ids.contains(&dpl_chat_gemini::PROVIDER));
    }

    #[test]
    fn claude_reads_real_money_and_keeps_cache_separate() {
        let claude = usage_for(dpl_chat_claude::PROVIDER).expect("claude");
        let assistant = r#"{"type":"assistant","message":{"model":"claude-opus-5",
            "usage":{"input_tokens":100,"cache_read_input_tokens":900,
            "cache_creation_input_tokens":50,"output_tokens":200}}}"#;
        let usage = claude.usage_from_line(assistant).expect("usage");
        // Cache reads bill at a fraction of fresh input, so they stay
        // SEPARATE — folding them would overestimate cost, and the split
        // can never be recovered from a folded number.
        assert_eq!(usage.input_tokens, Some(100), "fresh input only");
        assert_eq!(usage.cache_read_tokens, Some(900));
        assert_eq!(usage.cache_write_tokens, Some(50));
        assert_eq!(usage.output_tokens, Some(200));
        // The total is still everything billed: 100 + 900 + 50 + 200.
        assert_eq!(usage.total_tokens, Some(1250));
        assert_eq!(
            claude.model_from_line(assistant).as_deref(),
            Some("claude-opus-5")
        );

        let result = r#"{"type":"result","total_cost_usd":0.0425,
            "usage":{"input_tokens":1050,"output_tokens":200}}"#;
        assert_eq!(
            claude.cost_micros_from_line(result),
            Some(42_500),
            "provider-reported money is authoritative over any price table"
        );
    }

    #[test]
    fn a_nonsense_cost_is_refused_not_cast() {
        let claude = usage_for(dpl_chat_claude::PROVIDER).expect("claude");
        for bad in [
            r#"{"type":"result","total_cost_usd":-1.0}"#,
            r#"{"type":"result","total_cost_usd":null}"#,
            r#"{"type":"result"}"#,
        ] {
            assert_eq!(
                claude.cost_micros_from_line(bad),
                None,
                "a bad cost must be absent, never a plausible-looking number"
            );
        }
    }

    #[test]
    fn grok_reports_occupancy_through_its_typed_live_path() {
        let usage =
            dpl_chat_grok::GrokUsage::usage_from_update(&dpl_chat_grok::GrokSessionUpdate::Usage {
                used: 12_000,
                size: 128_000,
            })
            .expect("usage");
        assert_eq!(usage.context_used, Some(12_000));
        assert_eq!(usage.context_size, Some(128_000));
        assert_eq!(
            usage.input_tokens, None,
            "occupancy is never inflated into a split grok did not give"
        );

        // A non-usage update yields nothing.
        assert_eq!(
            dpl_chat_grok::GrokUsage::usage_from_update(
                &dpl_chat_grok::GrokSessionUpdate::AssistantText("hi".into())
            ),
            None
        );
    }

    #[test]
    fn an_unknown_provider_has_no_adapter() {
        assert!(usage_for(ProviderId("not-a-provider")).is_none());
    }

    #[test]
    fn absent_counts_never_become_zero() {
        let codex = usage_for(dpl_chat_codex::PROVIDER).expect("codex");
        // A token_count with no numbers yields None, not a zeroed usage —
        // cost computed from a guessed zero would be a lie.
        let empty = r#"{"type":"event_msg","payload":{"type":"token_count","info":{}}}"#;
        assert_eq!(codex.usage_from_line(empty), None);
        assert!(dpl_chat_core::TokenUsage::default().is_empty());
    }
}
