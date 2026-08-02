//! Claude provider descriptor.
//!
//! Live structured-stream integration is CH5. The runtime exception is
//! ratified, but process launch stays fail-closed on the ADR launch gates.

use dpl_chat_core::{
    BoundedNativePayload, Capability, ChatEventContent, PresenceSpec, ProviderAccent,
    ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
};
use dpl_chat_store::{DecodeOutcome, DecodedHistoryEvent, HistoryDecoder, HistoryIndexMetadata};
use serde_json::Value;
use std::path::Path;

pub const PROVIDER: ProviderId = ProviderId("claude");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("claude-default");

/// `claude auth status` prints JSON — `loggedIn`, `authMethod`, and the
/// subscription tier. Verified against Claude Code 2.1.220.
pub const PRESENCE: PresenceSpec = PresenceSpec {
    program: "claude",
    args: &["auth", "status"],
    signed_in: &["\"loggedin\": true"],
    signed_out: &["\"loggedin\": false"],
};

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
        label: "Claude Code",
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeHistoryDecoder;

impl HistoryDecoder for ClaudeHistoryDecoder {
    fn provider_id(&self) -> ProviderId {
        PROVIDER
    }

    fn index_metadata(&self, path: &Path, prefix: &str) -> Option<HistoryIndexMetadata> {
        let native_id = path.file_stem()?.to_string_lossy().into_owned();
        let mut title = String::new();
        let mut workspace = String::new();
        let mut started_at = String::new();
        for line in prefix.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if workspace.is_empty() {
                workspace = string_at(&value, &["cwd"]).unwrap_or_default();
            }
            if started_at.is_empty() {
                started_at = string_at(&value, &["timestamp"]).unwrap_or_default();
            }
            if title.is_empty() && value.get("type").and_then(Value::as_str) == Some("user") {
                title = claude_message_text(&value).unwrap_or_default();
            }
        }
        let fallback = history_title_fallback("Claude", &workspace, &started_at, &native_id);
        Some(HistoryIndexMetadata {
            native_id: dpl_chat_core::ProviderSessionId(native_id.clone()),
            title: one_line_title(&title, &fallback),
            workspace,
            started_at,
        })
    }

    fn decode_line(&self, line: &str) -> DecodeOutcome {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return unknown("claude.malformed", "malformed JSON record");
        };
        let record_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("missing");
        match record_type {
            "user" => claude_message_text(&value)
                .map(ChatEventContent::UserMessage)
                .map_or_else(|| unknown("claude.user", "user record without text"), event),
            "assistant" => decode_assistant(&value),
            "system" => {
                if value.get("subtype").and_then(Value::as_str) == Some("init") {
                    event(ChatEventContent::SessionStarted)
                } else {
                    DecodeOutcome::IgnoreKnown
                }
            }
            "result" => {
                if value.get("is_error").and_then(Value::as_bool) == Some(true) {
                    event(ChatEventContent::TurnFailed {
                        message: "Claude reported a failed turn".into(),
                    })
                } else {
                    event(ChatEventContent::TurnCompleted)
                }
            }
            "progress" => event(ChatEventContent::StatusUpdated(
                value
                    .get("subtype")
                    .and_then(Value::as_str)
                    .unwrap_or("Claude progress updated")
                    .to_owned(),
            )),
            "rate_limit_event" => event(ChatEventContent::UsageUpdated {
                summary: "Claude rate-limit status updated".into(),
            }),
            other => unknown(&format!("claude.{other}"), &redacted_shape(other, &value)),
        }
    }
}

fn decode_assistant(value: &Value) -> DecodeOutcome {
    if let Some(text) = claude_message_text(value) {
        return event(ChatEventContent::AssistantMessage(text));
    }
    let content = value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array);
    if let Some(tool) = content.and_then(|parts| {
        parts
            .iter()
            .find(|part| part.get("type").and_then(Value::as_str) == Some("tool_use"))
    }) {
        return event(ChatEventContent::ToolStarted {
            label: tool
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Claude tool")
                .to_owned(),
        });
    }
    unknown(
        "claude.assistant",
        "assistant record without displayable text",
    )
}

fn claude_message_text(value: &Value) -> Option<String> {
    let content = value.get("message")?.get("content")?;
    if let Some(text) = content.as_str() {
        return Some(text.to_owned());
    }
    let joined = content
        .as_array()?
        .iter()
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
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
    fn claude_index_uses_cwd_and_first_user_message() {
        let prefix = concat!(
            "{\"type\":\"system\",\"subtype\":\"init\",\"cwd\":\"C:/repo\",\"timestamp\":\"2026-07-23T00:00:00Z\"}\n",
            "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Review  this\"}]}}\n"
        );
        let metadata = ClaudeHistoryDecoder
            .index_metadata(Path::new("session-1.jsonl"), prefix)
            .expect("metadata");
        assert_eq!(metadata.native_id.0, "session-1");
        assert_eq!(metadata.workspace, "C:/repo");
        assert_eq!(metadata.title, "Review this");
    }

    #[test]
    fn claude_unknown_records_do_not_retain_values() {
        let outcome = ClaudeHistoryDecoder
            .decode_line("{\"type\":\"future\",\"token\":\"must-not-survive\"}");
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
    fn claude_index_rejects_injected_context_as_a_session_title() {
        let prefix = concat!(
            "{\"type\":\"system\",\"subtype\":\"init\",\"cwd\":\"C:/repo\",\"timestamp\":\"2026-07-23T00:00:00Z\"}\n",
            "{\"type\":\"user\",\"message\":{\"content\":\"<environment_context>private setup context</environment_context>\"}}\n"
        );
        let metadata = ClaudeHistoryDecoder
            .index_metadata(Path::new("session-1.jsonl"), prefix)
            .expect("metadata");
        assert_eq!(metadata.title, "Claude · repo · 2026-07-23");
    }
}
