//! Codex provider descriptor.
//!
//! Live app-server integration is CH4. The runtime exception is ratified, but
//! process launch stays fail-closed until every ADR launch gate passes.

mod activation;
pub mod runtime;

pub use activation::{
    CodexActivationCoordinator, CodexActivationError, CodexDiscoveryError, CodexLoginExit,
    CodexLoginLauncher, CodexLoginProcess, CodexLoginStatus, CodexRuntimeProfile, CodexStatusProbe,
    CodexStatusRequest, CodexVersion, SETUP_URL,
};

use dpl_chat_core::{
    BoundedNativePayload, Capability, ChatEventContent, ProviderAccent, ProviderDescriptor,
    ProviderId, ProviderManifest, ProviderProfileId, TokenUsage,
};
use dpl_chat_store::{
    DecodeOutcome, DecodedHistoryEvent, HistoryDecoder, HistoryIndexMetadata, ProviderUsage,
    UsageReporting,
};
use serde_json::Value;
use std::path::Path;

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
    Capability::Video,
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
        label: "ChatGPT / Codex",
        short_label: "Codex",
        glyph: "C",
        accent: ProviderAccent::Cyan,
        summary: "OpenAI coding agent through the official app-server client protocol.",
        // OpenAI founded 11 December 2015.
        since: "2015-12-11",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "CH4",
    evidence: "docs/research/chat-provider-capabilities.md#codex",
};

#[derive(Debug, Clone, Copy, Default)]
pub struct CodexHistoryDecoder;

impl HistoryDecoder for CodexHistoryDecoder {
    fn provider_id(&self) -> ProviderId {
        PROVIDER
    }

    fn index_metadata(&self, path: &Path, prefix: &str) -> Option<HistoryIndexMetadata> {
        let mut native_id = codex_id_from_path(path)?;
        let mut title = String::new();
        let mut workspace = String::new();
        let mut started_at = String::new();

        for line in prefix.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if started_at.is_empty() {
                started_at = string_at(&value, &["timestamp"]).unwrap_or_default();
            }
            if value.get("type").and_then(Value::as_str) == Some("session_meta") {
                if let Some(id) = string_at(&value, &["payload", "id"]) {
                    native_id = id;
                }
                workspace = string_at(&value, &["payload", "cwd"]).unwrap_or(workspace);
                started_at = string_at(&value, &["payload", "timestamp"]).unwrap_or(started_at);
            }
            if workspace.is_empty() {
                workspace = string_at(&value, &["cwd"]).unwrap_or_default();
            }
            if title.is_empty() {
                title = codex_user_text(&value).unwrap_or_default();
            }
        }
        let fallback = history_title_fallback("Codex", &workspace, &started_at, &native_id);
        Some(HistoryIndexMetadata {
            native_id: dpl_chat_core::ProviderSessionId(native_id),
            title: one_line_title(&title, &fallback),
            workspace,
            started_at,
        })
    }

    fn decode_line(&self, line: &str) -> DecodeOutcome {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return unknown("codex.malformed", "malformed JSON record");
        };
        let record_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("missing");
        match record_type {
            "session_meta" | "turn_context" => DecodeOutcome::IgnoreKnown,
            "response_item" => decode_response_item(value.get("payload").unwrap_or(&Value::Null)),
            "event_msg" => decode_event_message(value.get("payload").unwrap_or(&Value::Null)),
            "item.completed" => decode_completed_item(value.get("item").unwrap_or(&Value::Null)),
            other => unknown(&format!("codex.{other}"), &redacted_shape(other, &value)),
        }
    }
}

fn decode_response_item(payload: &Value) -> DecodeOutcome {
    let item_type = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    match item_type {
        "message" => {
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let text = content_text(payload.get("content"));
            match (role, text) {
                ("user", Some(text)) => event(ChatEventContent::UserMessage(text)),
                ("assistant", Some(text)) => event(ChatEventContent::AssistantMessage(text)),
                _ => unknown(
                    "codex.response_item.message",
                    &redacted_shape(item_type, payload),
                ),
            }
        }
        "function_call" | "custom_tool_call" => {
            let label = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Codex tool")
                .to_owned();
            event(ChatEventContent::ToolStarted { label })
        }
        "function_call_output" | "custom_tool_call_output" => {
            event(ChatEventContent::ToolCompleted {
                label: "Codex tool".into(),
                succeeded: true,
            })
        }
        "reasoning" => DecodeOutcome::IgnoreKnown,
        other => unknown(
            &format!("codex.response_item.{other}"),
            &redacted_shape(other, payload),
        ),
    }
}

fn decode_event_message(payload: &Value) -> DecodeOutcome {
    let event_type = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    match event_type {
        "user_message" => string_at(payload, &["message"])
            .map(ChatEventContent::UserMessage)
            .map_or_else(
                || unknown("codex.event_msg.user_message", "user message without text"),
                event,
            ),
        "agent_message" => string_at(payload, &["message"])
            .map(ChatEventContent::AssistantMessage)
            .map_or_else(
                || {
                    unknown(
                        "codex.event_msg.agent_message",
                        "agent message without text",
                    )
                },
                event,
            ),
        "task_started" => event(ChatEventContent::SessionStarted),
        "task_complete" => event(ChatEventContent::TurnCompleted),
        "turn_aborted" => event(ChatEventContent::TurnInterrupted),
        "token_count" => {
            let usage = codex_token_usage(payload);
            let summary = match (usage.input_tokens, usage.output_tokens) {
                (Some(input), Some(output)) => {
                    format!("Codex tokens · {input} in · {output} out")
                }
                _ => usage.total_tokens.map_or_else(
                    || "Codex token usage updated".to_owned(),
                    |total| format!("Codex tokens · {total} total"),
                ),
            };
            event(ChatEventContent::UsageUpdated { summary, usage })
        }
        // The provider naming its own model — previously fell through to
        // `unknown`, which is why nothing downstream could price a call.
        "session_configured" => string_at(payload, &["model"])
            .map(|model| ChatEventContent::ModelSelected { model })
            .map_or_else(
                || unknown("codex.event_msg.session_configured", "no model field"),
                event,
            ),
        "plan_update" => {
            let summary =
                string_at(payload, &["explanation"]).unwrap_or_else(|| "Codex plan updated".into());
            event(ChatEventContent::PlanUpdated(summary))
        }
        "agent_reasoning" | "world_state" => DecodeOutcome::IgnoreKnown,
        other => unknown(
            &format!("codex.event_msg.{other}"),
            &redacted_shape(other, payload),
        ),
    }
}

fn decode_completed_item(item: &Value) -> DecodeOutcome {
    let item_type = item
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    match item_type {
        "user_message" => string_at(item, &["text"])
            .map(ChatEventContent::UserMessage)
            .map_or_else(
                || unknown("codex.item.user_message", "user item without text"),
                event,
            ),
        "agent_message" => string_at(item, &["text"])
            .map(ChatEventContent::AssistantMessage)
            .map_or_else(
                || unknown("codex.item.agent_message", "agent item without text"),
                event,
            ),
        "command_execution" => event(ChatEventContent::ToolCompleted {
            label: "Command execution".into(),
            succeeded: item.get("status").and_then(Value::as_str) == Some("completed"),
        }),
        "reasoning" => DecodeOutcome::IgnoreKnown,
        other => unknown(&format!("codex.item.{other}"), &redacted_shape(other, item)),
    }
}

fn codex_user_text(value: &Value) -> Option<String> {
    match value.get("type").and_then(Value::as_str) {
        Some("response_item") => {
            let payload = value.get("payload")?;
            (payload.get("type").and_then(Value::as_str) == Some("message")
                && payload.get("role").and_then(Value::as_str) == Some("user"))
            .then(|| content_text(payload.get("content")))
            .flatten()
        }
        Some("event_msg") => {
            let payload = value.get("payload")?;
            (payload.get("type").and_then(Value::as_str) == Some("user_message"))
                .then(|| string_at(payload, &["message"]))
                .flatten()
        }
        Some("item.completed") => {
            let item = value.get("item")?;
            (item.get("type").and_then(Value::as_str) == Some("user_message"))
                .then(|| string_at(item, &["text"]))
                .flatten()
        }
        _ => None,
    }
}

fn content_text(content: Option<&Value>) -> Option<String> {
    let content = content?;
    if let Some(text) = content.as_str() {
        return Some(text.to_owned());
    }
    let joined = content
        .as_array()?
        .iter()
        .filter_map(|part| {
            part.get("text")
                .or_else(|| part.get("input_text"))
                .or_else(|| part.get("output_text"))
                .and_then(Value::as_str)
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!joined.is_empty()).then_some(joined)
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_owned)
}

/// Codex's usage dialect.
///
/// Codex reports a full input/output split plus a combined total and the
/// model context window, and names its model in `session_configured`. It
/// does not price its own calls.
#[derive(Debug, Clone, Copy, Default)]
pub struct CodexUsage;

impl ProviderUsage for CodexUsage {
    fn provider_id(&self) -> ProviderId {
        PROVIDER
    }

    fn reports(&self) -> UsageReporting {
        UsageReporting {
            input_output_split: true,
            total_tokens: true,
            context_window: true,
            provider_priced: false,
            model: true,
            // `total_token_usage` restates the running total each time.
            counts_are_cumulative: true,
            history_backfill: true,
        }
    }

    fn usage_from_line(&self, line: &str) -> Option<TokenUsage> {
        let value: Value = serde_json::from_str(line).ok()?;
        let payload = value.get("payload")?;
        if payload.get("type").and_then(Value::as_str)? != "token_count" {
            return None;
        }
        let usage = codex_token_usage(payload);
        (!usage.is_empty()).then_some(usage)
    }

    fn model_from_line(&self, line: &str) -> Option<String> {
        let value: Value = serde_json::from_str(line).ok()?;
        let payload = value.get("payload")?;
        if payload.get("type").and_then(Value::as_str)? != "session_configured" {
            return None;
        }
        string_at(payload, &["model"])
    }
}

fn u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_u64()
}

/// Codex's `token_count` accounting.
///
/// Read DEFENSIVELY: each field is looked up independently and a missing
/// one stays `None`. If Codex reshapes this payload we lose a field, not
/// correctness — a wrong number priced as real would be worse than an
/// absent one. `total_token_usage` is the cumulative turn total, which is
/// what cost is computed from.
fn codex_token_usage(payload: &Value) -> TokenUsage {
    let at = |leaf: &str| {
        u64_at(payload, &["info", "total_token_usage", leaf])
            .or_else(|| u64_at(payload, &["total_token_usage", leaf]))
    };
    TokenUsage {
        input_tokens: at("input_tokens"),
        // Codex reports cached input separately too — kept separate for
        // the same reason: it bills at a different rate.
        cache_read_tokens: at("cached_input_tokens"),
        cache_write_tokens: None,
        output_tokens: at("output_tokens"),
        total_tokens: at("total_tokens"),
        context_used: None,
        context_size: u64_at(payload, &["info", "model_context_window"])
            .or_else(|| u64_at(payload, &["model_context_window"])),
    }
}

fn codex_id_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    if let Some(candidate) = stem.get(stem.len().saturating_sub(36)..) {
        if looks_like_uuid(candidate) {
            return Some(candidate.to_owned());
        }
    }
    Some(stem.into_owned())
}

fn looks_like_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

fn one_line_title(value: &str, fallback: &str) -> String {
    let title = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() || looks_like_injected_context(&title) {
        fallback.to_owned()
    } else {
        title.chars().take(72).collect()
    }
}

fn looks_like_injected_context(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.starts_with('<')
        || normalized.starts_with("# agents.md instructions")
        || normalized.starts_with("# files mentioned")
        || normalized.contains("<recommended_plugins>")
        || normalized.contains("<in-app-browser-context")
        || normalized.contains("<environment_context")
}

fn history_title_fallback(
    provider: &str,
    workspace: &str,
    started_at: &str,
    native_id: &str,
) -> String {
    let workspace_name = workspace
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or_default();
    let date = started_at.get(..10).unwrap_or_default();
    match (workspace_name.is_empty(), date.is_empty()) {
        (false, false) => format!("{provider} · {workspace_name} · {date}"),
        (false, true) => format!("{provider} · {workspace_name}"),
        (true, false) => format!("{provider} session · {date}"),
        (true, true) => format!(
            "{provider} session · {}",
            native_id.chars().take(8).collect::<String>()
        ),
    }
}

fn redacted_shape(record_type: &str, value: &Value) -> String {
    let keys = value
        .as_object()
        .map(|object| {
            let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
            keys.sort_unstable();
            keys.join(",")
        })
        .unwrap_or_default();
    format!("type={record_type}; keys=[{keys}]")
}

fn event(content: ChatEventContent) -> DecodeOutcome {
    DecodeOutcome::Event(DecodedHistoryEvent {
        turn_id: None,
        content,
    })
}

fn unknown(schema: &str, summary: &str) -> DecodeOutcome {
    event(ChatEventContent::Unknown(BoundedNativePayload::redacted(
        schema, summary,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_index_uses_session_meta_and_first_user_message() {
        let prefix = concat!(
            "{\"timestamp\":\"2026-07-23T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"thr-1\",\"cwd\":\"C:/repo\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"Fix  the tests\"}}\n"
        );
        let metadata = CodexHistoryDecoder
            .index_metadata(Path::new("rollout-fallback.jsonl"), prefix)
            .expect("metadata");
        assert_eq!(metadata.native_id.0, "thr-1");
        assert_eq!(metadata.workspace, "C:/repo");
        assert_eq!(metadata.title, "Fix the tests");
    }

    #[test]
    fn codex_unknown_records_preserve_only_redacted_structure() {
        let outcome = CodexHistoryDecoder.decode_line(
            "{\"type\":\"future.secret\",\"token\":\"must-not-survive\",\"value\":42}",
        );
        let DecodeOutcome::Event(event) = outcome else {
            panic!("event");
        };
        let ChatEventContent::Unknown(payload) = event.content else {
            panic!("unknown");
        };
        assert!(payload.redacted_summary.contains("token"));
        assert!(!payload.redacted_summary.contains("must-not-survive"));
    }

    #[test]
    fn codex_index_rejects_injected_context_as_a_session_title() {
        let prefix = concat!(
            "{\"timestamp\":\"2026-07-23T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"thr-1\",\"cwd\":\"C:/repo\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"<recommended_plugins>secret setup context</recommended_plugins>\"}}\n"
        );
        let metadata = CodexHistoryDecoder
            .index_metadata(Path::new("rollout-fallback.jsonl"), prefix)
            .expect("metadata");
        assert_eq!(metadata.title, "Codex · repo · 2026-07-23");
    }

    #[test]
    fn codex_world_state_is_known_nondisplayable_context() {
        let outcome = CodexHistoryDecoder
            .decode_line("{\"type\":\"event_msg\",\"payload\":{\"type\":\"world_state\"}}");
        assert_eq!(outcome, DecodeOutcome::IgnoreKnown);
    }

    #[test]
    fn token_count_yields_real_numbers_not_prose() {
        let line = r#"{"type":"event_msg","payload":{"type":"token_count","info":{
            "total_token_usage":{"input_tokens":1200,"cached_input_tokens":800,
            "output_tokens":450,"reasoning_output_tokens":100,"total_tokens":1650},
            "model_context_window":272000}}}"#;
        let DecodeOutcome::Event(decoded) = CodexHistoryDecoder.decode_line(line) else {
            panic!("token_count should decode to an event");
        };
        let ChatEventContent::UsageUpdated { summary, usage } = decoded.content else {
            panic!("token_count should decode to UsageUpdated");
        };
        // The counts are EXTRACTED, not summarized away. This payload used
        // to be discarded entirely in favour of a fixed sentence.
        assert_eq!(usage.input_tokens, Some(1200));
        assert_eq!(usage.output_tokens, Some(450));
        assert_eq!(usage.total_tokens, Some(1650));
        assert_eq!(usage.context_size, Some(272_000));
        assert!(!usage.is_empty());
        // The human line still reads well for the activity feed.
        assert_eq!(summary, "Codex tokens · 1200 in · 450 out");
    }

    #[test]
    fn a_reshaped_token_payload_loses_fields_not_correctness() {
        // If Codex moves these fields, each lookup independently returns
        // None. A missing count must never silently become zero — zero is
        // a measurement, absent is not, and cost is computed from these.
        let line = r#"{"type":"event_msg","payload":{"type":"token_count","info":{}}}"#;
        let DecodeOutcome::Event(decoded) = CodexHistoryDecoder.decode_line(line) else {
            panic!("token_count should still decode");
        };
        let ChatEventContent::UsageUpdated { summary, usage } = decoded.content else {
            panic!("expected UsageUpdated");
        };
        assert!(usage.is_empty(), "absent counts stay absent, never zero");
        assert_eq!(summary, "Codex token usage updated");
    }

    #[test]
    fn session_configured_names_the_model() {
        let line = r#"{"type":"event_msg","payload":{"type":"session_configured",
            "model":"gpt-5-codex","session_id":"abc"}}"#;
        let DecodeOutcome::Event(decoded) = CodexHistoryDecoder.decode_line(line) else {
            panic!("session_configured should decode to an event");
        };
        assert_eq!(
            decoded.content,
            ChatEventContent::ModelSelected {
                model: "gpt-5-codex".into()
            },
            "the model the provider is actually running — previously lost to `unknown`"
        );
    }
}
