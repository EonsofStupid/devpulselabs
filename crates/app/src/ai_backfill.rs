//! THE PIGGYBACK PASS — recover AI cost from sessions already on disk.
//!
//! One concern: walk the provider sessions the operator has ALREADY run
//! and turn their stored token counts into ledger facts, so analytics
//! start with history instead of an empty warehouse.
//!
//! Two properties make this safe rather than merely convenient:
//!
//! 1. **Recovered facts are marked.** Every emitted `CallStarted` carries
//!    `Observation::Backfilled`, and `FirstToken` is NEVER emitted — a
//!    log file cannot yield time-to-first-token, and a synthetic `seq`
//!    was never ordered against the captures it now sits beside. Queries
//!    exclude these from latency and ordering questions.
//! 2. **It is idempotent.** A manifest records which sessions have been
//!    ingested, so pointing the pass at the same history twice does not
//!    double-count spend.
//!
//! Only providers declaring `history_backfill` are walked — live-only
//! transports (Grok's ACP) have nothing on disk and are excluded by
//! construction, not by a special case here.

use dpl_chat_core::ProviderId;
use dpl_chat_store::{HistorySource, JsonlHistorySource, ProviderUsage, SessionUsage};
use dpl_fabric::{AiEvent, AiOutcome, Fabric, Observation, Origin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Manifest schema version — additive only (document law).
pub const MANIFEST_VERSION: u32 = 1;

/// Which sessions have already been folded into the ledger.
///
/// Keyed by `provider/native-session-id`, which is stable across runs —
/// a re-scan of the same session is recognised and skipped rather than
/// billed twice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackfillManifest {
    pub v: u32,
    pub ingested: BTreeSet<String>,
}

impl Default for BackfillManifest {
    fn default() -> Self {
        Self {
            v: MANIFEST_VERSION,
            ingested: BTreeSet::new(),
        }
    }
}

impl BackfillManifest {
    /// Load, or start empty. A NEWER manifest is refused (document law):
    /// a future version may mean things this build cannot honour, and
    /// guessing would risk double-counting real money.
    pub fn load(path: &Path) -> Result<Self, String> {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Ok(Self::default());
        };
        let doc: Self = serde_json::from_str(&raw)
            .map_err(|e| format!("backfill manifest: {} — {e}", path.display()))?;
        if doc.v > MANIFEST_VERSION {
            return Err(format!(
                "backfill manifest v{} is newer than this build understands (v{MANIFEST_VERSION}); \
                 refusing rather than risk double-counting spend",
                doc.v
            ));
        }
        Ok(doc)
    }

    /// Crash-safe write: tmp → rename, so an interrupted save cannot
    /// leave a truncated manifest that would re-ingest everything.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("backfill manifest dir: {e}"))?;
        }
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| format!("backfill manifest encode: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, body + "\n").map_err(|e| format!("backfill manifest write: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("backfill manifest rename: {e}"))
    }

    fn key(provider: ProviderId, native_id: &str) -> String {
        format!("{}/{native_id}", provider.0)
    }

    pub fn contains(&self, provider: ProviderId, native_id: &str) -> bool {
        self.ingested.contains(&Self::key(provider, native_id))
    }

    pub fn mark(&mut self, provider: ProviderId, native_id: &str) {
        self.ingested.insert(Self::key(provider, native_id));
    }
}

/// What one backfill run did — reported, never silently assumed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BackfillReport {
    pub sessions_seen: usize,
    pub sessions_ingested: usize,
    pub sessions_skipped_already_done: usize,
    pub sessions_without_usage: usize,
}

/// Emit one recovered session as AI facts.
///
/// `CallStarted` + `CallFinished` ONLY. No `FirstToken`: that number does
/// not exist in a stored record, and inventing it would poison every
/// latency percentile computed later.
fn emit(fabric: &Fabric, provider: ProviderId, session: &SessionUsage) {
    let call = fabric.mint_ai_call();
    fabric.publish_ai(AiEvent::CallStarted {
        call,
        provider,
        model: session.model.clone().unwrap_or_default(),
        origin: Origin::surface("backfill"),
        observation: Observation::Backfilled,
    });
    fabric.publish_ai(AiEvent::CallFinished {
        call,
        tokens_in: session.usage.input_tokens,
        cache_read: session.usage.cache_read_tokens,
        cache_write: session.usage.cache_write_tokens,
        tokens_out: session.usage.output_tokens,
        total_tokens: session.usage.total_tokens,
        context_used: session.usage.context_used,
        context_size: session.usage.context_size,
        // Provider-reported money when the provider prices itself;
        // otherwise absent until the price table runs (PR 2).
        cost_micros: session.cost_micros,
        // A stored session carries no honest wall-clock for the call.
        // Zero here means "not measurable", and `Observation::Backfilled`
        // is what tells a query to never read it as a duration.
        duration_ms: 0,
        outcome: AiOutcome::Ok,
    });
}

/// Walk one provider's stored sessions into the ledger.
pub fn backfill_provider(
    fabric: &Fabric,
    source: &JsonlHistorySource,
    usage: &dyn ProviderUsage,
    manifest: &mut BackfillManifest,
) -> Result<BackfillReport, String> {
    let provider = usage.provider_id();
    let index = source
        .list(None)
        .map_err(|e| format!("backfill list {provider}: {e:?}"))?;
    let mut report = BackfillReport::default();

    for session in index.sessions {
        report.sessions_seen += 1;
        // Prefer the provider's own session id; fall back to the locator,
        // which is derived from the file's path and is equally stable across
        // runs. A session with NO stable key would be re-billed every boot.
        let native = session.summary.provider_session_id.as_ref().map_or_else(
            || format!("locator-{}", session.locator.value()),
            |id| id.0.clone(),
        );
        if manifest.contains(provider, &native) {
            report.sessions_skipped_already_done += 1;
            continue;
        }
        let scanned = match source.scan_usage(session.locator, usage) {
            Ok(scanned) => scanned,
            Err(error) => {
                tracing::warn!(%provider, session = %native, ?error, "backfill: session unreadable");
                continue;
            }
        };
        if scanned.is_empty() {
            // Nothing to recover is a FACT, not a failure — and it is
            // still marked, so the next run does not re-read it.
            report.sessions_without_usage += 1;
            manifest.mark(provider, &native);
            continue;
        }
        emit(fabric, provider, &scanned);
        manifest.mark(provider, &native);
        report.sessions_ingested += 1;
    }
    Ok(report)
}

/// The manifest's home, beside the ledger it protects.
pub fn manifest_path(analytics_root: &Path) -> PathBuf {
    analytics_root.join("backfill.v1.json")
}

/// Every backfillable provider, in one pass. Providers that declare no
/// history are skipped by construction, not by a special case.
pub fn run(
    fabric: &Fabric,
    analytics_root: &Path,
    sources: &[(&JsonlHistorySource, &'static dyn ProviderUsage)],
) -> Result<BackfillReport, String> {
    let path = manifest_path(analytics_root);
    let mut manifest = BackfillManifest::load(&path)?;
    let mut total = BackfillReport::default();

    for (source, usage) in sources {
        if !usage.reports().history_backfill {
            continue;
        }
        let report = backfill_provider(fabric, source, *usage, &mut manifest)?;
        total.sessions_seen += report.sessions_seen;
        total.sessions_ingested += report.sessions_ingested;
        total.sessions_skipped_already_done += report.sessions_skipped_already_done;
        total.sessions_without_usage += report.sessions_without_usage;
    }

    manifest.save(&path)?;
    tracing::info!(
        seen = total.sessions_seen,
        ingested = total.sessions_ingested,
        already_done = total.sessions_skipped_already_done,
        no_usage = total.sessions_without_usage,
        "AI backfill from sessions already on disk",
    );
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_usage;
    use dpl_chat_core::TokenUsage;
    use dpl_chat_store::HistoryLimits;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-backfill-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_newer_manifest_is_refused_rather_than_guessed() {
        let dir = scratch("newer");
        let path = dir.join("backfill.v1.json");
        fs::write(&path, r#"{"v":99,"ingested":[]}"#).unwrap();
        let error = BackfillManifest::load(&path).expect_err("must refuse");
        assert!(
            error.contains("newer") && error.contains("double-counting"),
            "the refusal must say WHY: {error}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_manifest_starts_empty_not_broken() {
        let dir = scratch("missing");
        let manifest = BackfillManifest::load(&dir.join("nope.json")).expect("starts empty");
        assert_eq!(manifest.v, MANIFEST_VERSION);
        assert!(manifest.ingested.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_manifest_round_trips_and_dedupes_by_provider_and_session() {
        let dir = scratch("roundtrip");
        let path = dir.join("backfill.v1.json");
        let mut manifest = BackfillManifest::default();
        manifest.mark(ProviderId("codex"), "abc");
        manifest.save(&path).unwrap();

        let back = BackfillManifest::load(&path).unwrap();
        assert!(back.contains(ProviderId("codex"), "abc"));
        // Same session id under a DIFFERENT provider is a different fact.
        assert!(!back.contains(ProviderId("claude"), "abc"));
        assert!(!back.contains(ProviderId("codex"), "xyz"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn backfill_is_idempotent_and_never_emits_first_token() {
        let dir = scratch("idempotent");
        let sessions = dir.join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        // A real Codex session shape: a model, then token accounting.
        fs::write(
            sessions.join("s1.jsonl"),
            concat!(
                r#"{"type":"event_msg","payload":{"type":"session_configured","model":"gpt-5-codex"}}"#,
                "\n",
                r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1200,"output_tokens":450,"total_tokens":1650}}}}"#,
                "\n",
            ),
        )
        .unwrap();

        let source = JsonlHistorySource::open(
            &sessions,
            dpl_chat_codex::CodexHistoryDecoder,
            HistoryLimits::default(),
        )
        .expect("source");
        let usage = ai_usage::usage_for(dpl_chat_codex::PROVIDER).expect("codex adapter");

        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_ai();
        let mut manifest = BackfillManifest::default();

        let first = backfill_provider(&fabric, &source, usage, &mut manifest).unwrap();
        assert_eq!(first.sessions_ingested, 1, "the session is recovered");

        let mut kinds = Vec::new();
        let mut finished_tokens = None;
        while let Ok(stamped) = rx.try_recv() {
            match stamped.ev {
                AiEvent::CallStarted {
                    observation, model, ..
                } => {
                    assert_eq!(
                        observation,
                        Observation::Backfilled,
                        "recovered facts must never claim to be live"
                    );
                    assert_eq!(model, "gpt-5-codex", "the model is recovered too");
                    kinds.push("started");
                }
                AiEvent::FirstToken { .. } => kinds.push("firstToken"),
                AiEvent::CallFinished {
                    tokens_in,
                    tokens_out,
                    ..
                } => {
                    finished_tokens = Some((tokens_in, tokens_out));
                    kinds.push("finished");
                }
                AiEvent::Disposition { .. } => kinds.push("disposition"),
            }
        }
        assert_eq!(
            kinds,
            vec!["started", "finished"],
            "a stored record cannot yield time-to-first-token, so none is invented"
        );
        assert_eq!(finished_tokens, Some((Some(1200), Some(450))));

        // Running again must not bill the same session twice.
        let second = backfill_provider(&fabric, &source, usage, &mut manifest).unwrap();
        assert_eq!(second.sessions_ingested, 0);
        assert_eq!(second.sessions_skipped_already_done, 1);
        assert!(rx.try_recv().is_err(), "a second run emits nothing");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_session_with_no_accounting_is_marked_not_retried_forever() {
        let dir = scratch("nousage");
        let sessions = dir.join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join("s1.jsonl"),
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
        )
        .unwrap();

        let source = JsonlHistorySource::open(
            &sessions,
            dpl_chat_codex::CodexHistoryDecoder,
            HistoryLimits::default(),
        )
        .expect("source");
        let usage = ai_usage::usage_for(dpl_chat_codex::PROVIDER).expect("codex");
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_ai();
        let mut manifest = BackfillManifest::default();

        let report = backfill_provider(&fabric, &source, usage, &mut manifest).unwrap();
        assert_eq!(report.sessions_without_usage, 1);
        assert_eq!(report.sessions_ingested, 0);
        assert!(
            rx.try_recv().is_err(),
            "nothing to recover means nothing is published — not a zeroed call"
        );
        // Still marked, so it is not re-read on every boot.
        let again = backfill_provider(&fabric, &source, usage, &mut manifest).unwrap();
        assert_eq!(again.sessions_skipped_already_done, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_live_only_provider_is_never_walked() {
        let dir = scratch("liveonly");
        let sessions = dir.join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(sessions.join("s1.jsonl"), "{}\n").unwrap();
        let source = JsonlHistorySource::open(
            &sessions,
            dpl_chat_codex::CodexHistoryDecoder,
            HistoryLimits::default(),
        )
        .expect("source");
        let grok = ai_usage::usage_for(dpl_chat_grok::PROVIDER).expect("grok");
        let fabric = Fabric::new();

        // `run` filters on the declaration, so Grok is skipped even when
        // handed a source — the exclusion is structural, not a special case.
        let report = run(&fabric, &dir, &[(&source, grok)]).unwrap();
        assert_eq!(report.sessions_seen, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn claude_backfill_recovers_real_money() {
        let dir = scratch("claudecost");
        let sessions = dir.join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join("c1.jsonl"),
            concat!(
                r#"{"type":"assistant","message":{"model":"claude-opus-5","usage":{"input_tokens":100,"cache_read_input_tokens":900,"output_tokens":200}}}"#,
                "\n",
                r#"{"type":"result","total_cost_usd":0.0425,"usage":{"input_tokens":1000,"output_tokens":200}}"#,
                "\n",
            ),
        )
        .unwrap();
        let source = JsonlHistorySource::open(
            &sessions,
            dpl_chat_claude::ClaudeHistoryDecoder,
            HistoryLimits::default(),
        )
        .expect("source");
        let usage = ai_usage::usage_for(dpl_chat_claude::PROVIDER).expect("claude");
        let fabric = Fabric::new();
        let mut rx = fabric.subscribe_ai();
        let mut manifest = BackfillManifest::default();

        backfill_provider(&fabric, &source, usage, &mut manifest).unwrap();
        let mut cost = None;
        while let Ok(stamped) = rx.try_recv() {
            if let AiEvent::CallFinished { cost_micros, .. } = stamped.ev {
                cost = cost_micros;
            }
        }
        assert_eq!(
            cost,
            Some(42_500),
            "money Claude already billed is recovered, not re-derived"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn per_turn_counts_are_summed_and_cumulative_counts_are_not() {
        // CLAUDE reports each turn separately → the session is the SUM.
        let dir = scratch("policy-claude");
        let sessions = dir.join("s");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join("c.jsonl"),
            concat!(
                r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":10}}}"#,
                "\n",
                r#"{"type":"assistant","message":{"usage":{"input_tokens":200,"output_tokens":20}}}"#,
                "\n",
            ),
        )
        .unwrap();
        let source = JsonlHistorySource::open(
            &sessions,
            dpl_chat_claude::ClaudeHistoryDecoder,
            HistoryLimits::default(),
        )
        .unwrap();
        let usage = ai_usage::usage_for(dpl_chat_claude::PROVIDER).unwrap();
        let index = source.list(None).unwrap();
        let scanned = source.scan_usage(index.sessions[0].locator, usage).unwrap();
        assert_eq!(
            (scanned.usage.input_tokens, scanned.usage.output_tokens),
            (Some(300), Some(30)),
            "per-turn reports SUM — taking the last would undercount real spend"
        );
        let _ = fs::remove_dir_all(&dir);

        // CODEX restates the running total → the LAST report is the truth.
        let dir = scratch("policy-codex");
        let sessions = dir.join("s");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join("x.jsonl"),
            concat!(
                r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":10}}}}"#,
                "\n",
                r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":300,"output_tokens":30}}}}"#,
                "\n",
            ),
        )
        .unwrap();
        let source = JsonlHistorySource::open(
            &sessions,
            dpl_chat_codex::CodexHistoryDecoder,
            HistoryLimits::default(),
        )
        .unwrap();
        let usage = ai_usage::usage_for(dpl_chat_codex::PROVIDER).unwrap();
        let index = source.list(None).unwrap();
        let scanned = source.scan_usage(index.sessions[0].locator, usage).unwrap();
        assert_eq!(
            (scanned.usage.input_tokens, scanned.usage.output_tokens),
            (Some(300), Some(30)),
            "cumulative reports must NOT be summed — that would double-count"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_usage_merges_to_nothing() {
        // Guard on the merge the scan relies on.
        let a = TokenUsage {
            input_tokens: Some(10),
            ..TokenUsage::default()
        };
        let b = TokenUsage {
            output_tokens: Some(5),
            ..TokenUsage::default()
        };
        let merged = a.merge(b);
        assert_eq!(merged.input_tokens, Some(10));
        assert_eq!(merged.output_tokens, Some(5));
    }
}
