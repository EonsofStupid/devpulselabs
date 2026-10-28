//! Portable conversation state. Runtime authority is never restored from disk.

use super::{ChatRuntime, ProviderEntry};
use dpl_chat_core::{
    ChatEvent, ChatEventContent, ChatEventId, ChatSessionId, ProviderId, ProviderSessionId,
    SessionGroup, SessionState, SessionSummary, TurnId, MAX_NATIVE_PAYLOAD_BYTES,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::io::Write;

const VERSION: u32 = 1;
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_TEXT: usize = 1024 * 1024;
const MAX_PROVIDERS: usize = 64;
const MAX_SESSIONS: usize = 4096;
const MAX_EVENTS: usize = 65_536;
// Imported identities leave ample headroom for host-generated counters.
const MAX_ID: u64 = u64::MAX / 2;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    v: u32,
    selected_provider: Option<String>,
    // Resolve provider identity before decoding its data. An unavailable
    // provider may have event variants this build does not understand.
    providers: Vec<Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderArchive {
    provider: String,
    active_session: Option<u64>,
    sessions: Vec<SavedSession>,
    events: Vec<SavedEvent>,
    // A list supports the pre-session draft (None) without JSON object-key
    // coercions. Duplicate keys are rejected, not silently overwritten.
    drafts: Vec<SavedDraft>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSession {
    id: u64,
    native_id: Option<String>,
    title: String,
    workspace: String,
    group: SessionGroup,
    state: SessionState,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedEvent {
    id: u64,
    session: u64,
    turn: Option<String>,
    content: ChatEventContent,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedDraft {
    session: Option<u64>,
    text: String,
}

struct Restored {
    provider: ProviderId,
    active_session: Option<ChatSessionId>,
    sessions: Vec<SessionSummary>,
    events: Vec<ChatEvent>,
    drafts: BTreeMap<Option<ChatSessionId>, String>,
}

struct Decoded {
    selected: Option<ProviderId>,
    restored: Vec<Restored>,
    unavailable: Vec<Value>,
}

impl ChatRuntime {
    /// A versioned archive of conversations and all drafts, including drafts
    /// outside the selected tab. Credentials, accounts, activation, executable
    /// paths and capability qualification are deliberately not represented.
    pub fn archive(&self) -> Result<Value, String> {
        let mut providers = self
            .providers
            .iter()
            .map(|entry| serde_json::to_value(ProviderArchive::from_entry(entry)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Could not encode project chats: {error}"))?;
        providers.extend(self.archived_unavailable_providers.iter().cloned());
        let value = serde_json::to_value(Archive {
            v: VERSION,
            selected_provider: self.selected_provider.map(|id| id.0.to_owned()),
            providers,
        })
        .map_err(|error| format!("Could not encode project chats: {error}"))?;
        // Apply the same structural/size contract to saves and restores.
        self.decode_archive(&value)?;
        Ok(value)
    }

    /// Replace project conversations transactionally. Current machine facts
    /// and provider authority stay intact. Restored sessions are ReadOnly
    /// until the host successfully resumes their native provider session.
    /// Unavailable providers retain opaque conversation records for future
    /// saves without constructing identities or importing runtime authority.
    pub fn restore_archive(&mut self, value: &Value) -> Result<(), String> {
        let Decoded {
            selected,
            restored,
            unavailable,
        } = self.decode_archive(value)?;
        self.clear_project_state();
        self.archived_unavailable_providers = unavailable;
        for saved in restored {
            // All identities were resolved against this same registry above.
            if let Some(entry) = self
                .providers
                .iter_mut()
                .find(|entry| entry.registration.manifest.descriptor.id == saved.provider)
            {
                entry.active_session = saved.active_session;
                entry.registration.sessions = saved.sessions;
                entry.registration.events = saved.events;
                entry.drafts = saved.drafts;
            }
        }
        if let Some(provider) = selected {
            self.selected_provider = Some(provider);
        }
        Ok(())
    }

    /// Clear only project-owned data. Provider registration, current sign-in
    /// and launch qualification belong to the host and are not reset here.
    pub fn clear_project_state(&mut self) {
        self.archived_unavailable_providers.clear();
        for entry in &mut self.providers {
            entry.registration.sessions.clear();
            entry.registration.events.clear();
            entry.drafts.clear();
            entry.active_session = None;
        }
        self.selected_provider = self
            .providers
            .first()
            .map(|entry| entry.registration.manifest.descriptor.id);
    }

    fn decode_archive(&self, value: &Value) -> Result<Decoded, String> {
        check_value(value, 0)?;
        serde_json::to_writer(LimitedSize(0), value)
            .map_err(|error| format!("Project chat archive is too large: {error}"))?;
        let archive: Archive = serde_json::from_value(value.clone())
            .map_err(|error| format!("Invalid project chat archive: {error}"))?;
        if archive.v != VERSION {
            return Err(format!(
                "Unsupported project chat archive version {}",
                archive.v
            ));
        }
        if archive.providers.len() > MAX_PROVIDERS {
            return Err("Project chat archive contains too many providers".into());
        }
        let resolve = |name: &str| {
            self.providers
                .iter()
                .map(|entry| entry.registration.manifest.descriptor.id)
                .find(|id| id.0 == name)
        };
        let selected = archive.selected_provider.as_deref().and_then(resolve);
        let mut seen = HashSet::new();
        let mut restored = Vec::new();
        let mut unavailable = Vec::new();
        for value in archive.providers {
            let name = value
                .get("provider")
                .and_then(Value::as_str)
                .ok_or("Project chat provider identity is missing")?;
            if name.is_empty() || name.len() > 256 || !seen.insert(name.to_owned()) {
                return Err("Project chat provider identities are invalid or duplicated".into());
            }
            let Some(provider) = resolve(name) else {
                validate_unavailable(&value)?;
                unavailable.push(value);
                continue;
            };
            let saved: ProviderArchive = serde_json::from_value(value)
                .map_err(|error| format!("Invalid project chats for {provider}: {error}"))?;
            restored.push(saved.restore(provider)?);
        }
        Ok(Decoded {
            selected,
            restored,
            unavailable,
        })
    }
}

impl ProviderArchive {
    fn from_entry(entry: &ProviderEntry) -> Self {
        Self {
            provider: entry.registration.manifest.descriptor.id.0.to_owned(),
            active_session: entry.active_session.map(|id| id.0),
            sessions: entry
                .registration
                .sessions
                .iter()
                .map(|session| SavedSession {
                    id: session.id.0,
                    native_id: session.provider_session_id.as_ref().map(|id| id.0.clone()),
                    title: session.title.clone(),
                    workspace: session.workspace.clone(),
                    group: session.group,
                    state: session.state,
                })
                .collect(),
            events: entry
                .registration
                .events
                .iter()
                .map(|event| SavedEvent {
                    id: event.id.0,
                    session: event.session_id.0,
                    turn: event.turn_id.as_ref().map(|id| id.0.clone()),
                    content: event.content.clone(),
                })
                .collect(),
            drafts: entry
                .drafts
                .iter()
                .map(|(session, text)| SavedDraft {
                    session: session.map(|id| id.0),
                    text: text.clone(),
                })
                .collect(),
        }
    }

    fn validate(&self) -> Result<BTreeMap<Option<ChatSessionId>, String>, String> {
        if self.sessions.len() > MAX_SESSIONS
            || self.events.len() > MAX_EVENTS
            || self.drafts.len() > MAX_SESSIONS + 1
        {
            return Err("Project chats exceed the record limit".into());
        }
        let mut session_ids = HashSet::new();
        for session in &self.sessions {
            if session.id > MAX_ID || !session_ids.insert(session.id) {
                return Err("Project chat session identities are invalid or duplicated".into());
            }
            if session
                .native_id
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > 4096)
            {
                return Err("Project chat native session identity is invalid".into());
            }
        }
        if self
            .active_session
            .is_some_and(|id| !session_ids.contains(&id))
        {
            return Err("Project chat selection refers to a missing session".into());
        }
        let mut event_ids = HashSet::new();
        for event in &self.events {
            if event.id > MAX_ID
                || !event_ids.insert(event.id)
                || !session_ids.contains(&event.session)
            {
                return Err(
                    "Project chat event identities or session references are invalid".into(),
                );
            }
            if event
                .turn
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > 4096 || !valid_turn_counter(id))
            {
                return Err("Project chat turn identity is invalid".into());
            }
            if let ChatEventContent::Unknown(native) = &event.content {
                if native.redacted_summary.len() > MAX_NATIVE_PAYLOAD_BYTES
                    || native.original_bytes < native.redacted_summary.len()
                {
                    return Err("Project chat native diagnostic exceeds its bound".into());
                }
            }
        }
        let mut drafts = BTreeMap::new();
        for draft in &self.drafts {
            if draft.session.is_some_and(|id| !session_ids.contains(&id))
                || drafts
                    .insert(draft.session.map(ChatSessionId), draft.text.clone())
                    .is_some()
            {
                return Err(
                    "Project chat drafts have invalid or duplicate session references".into(),
                );
            }
        }
        Ok(drafts)
    }

    fn restore(self, provider: ProviderId) -> Result<Restored, String> {
        let drafts = self.validate()?;
        Ok(Restored {
            provider,
            active_session: self.active_session.map(ChatSessionId),
            sessions: self
                .sessions
                .into_iter()
                .map(|session| SessionSummary {
                    id: ChatSessionId(session.id),
                    provider_id: provider,
                    provider_session_id: session.native_id.map(ProviderSessionId),
                    title: session.title,
                    workspace: session.workspace,
                    group: session.group,
                    // A disk record never establishes a live provider turn.
                    state: SessionState::ReadOnly,
                })
                .collect(),
            events: self
                .events
                .into_iter()
                .map(|event| ChatEvent {
                    id: ChatEventId(event.id),
                    provider_id: provider,
                    session_id: ChatSessionId(event.session),
                    turn_id: event.turn.map(TurnId),
                    content: event.content,
                })
                .collect(),
            drafts,
        })
    }
}

fn valid_turn_counter(id: &str) -> bool {
    id.strip_prefix("grok-turn-")
        .is_none_or(|suffix| suffix.parse::<u64>().is_ok_and(|value| value <= MAX_ID))
}

fn validate_unavailable(value: &Value) -> Result<(), String> {
    // Validate conversation identity and references with the same contract.
    // Future event content stays opaque and is saved verbatim, never applied
    // as a runtime action. Extra provider metadata (auth, executables, etc.)
    // is still rejected by the strict envelope.
    let mut envelope = value.clone();
    if let Some(events) = envelope.get_mut("events").and_then(Value::as_array_mut) {
        for event in events {
            if let Some(content) = event.get_mut("content") {
                if serde_json::from_value::<ChatEventContent>(content.clone()).is_err() {
                    *content = Value::String("SessionStarted".into());
                }
            }
        }
    }
    let saved: ProviderArchive = serde_json::from_value(envelope)
        .map_err(|error| format!("Invalid unavailable provider archive: {error}"))?;
    saved.validate()?;
    Ok(())
}

fn check_value(value: &Value, depth: usize) -> Result<(), String> {
    if depth > 32 {
        return Err("Project chat archive is nested too deeply".into());
    }
    match value {
        Value::String(text) if text.len() > MAX_TEXT => {
            return Err("Project chat text exceeds the size limit".into());
        }
        Value::Array(values) => {
            if values.len() > MAX_EVENTS {
                return Err("Project chat archive contains too many records".into());
            }
            for child in values {
                check_value(child, depth + 1)?;
            }
        }
        Value::Object(values) => {
            if values.len() > MAX_EVENTS || values.keys().any(|key| key.len() > 256) {
                return Err("Project chat archive contains invalid object keys".into());
            }
            for child in values.values() {
                check_value(child, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

struct LimitedSize(usize);

impl Write for LimitedSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > MAX_BYTES {
            return Err(std::io::Error::other("archive exceeds 16 MiB"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderRegistration;
    use dpl_chat_core::{
        AccountSummary, ActivationState, BoundedNativePayload, Capability, ProviderAccent,
        ProviderDescriptor, ProviderManifest, ProviderProfileId, TokenUsage,
    };

    const FIRST: ProviderId = ProviderId("first");
    const SECOND: ProviderId = ProviderId("second");

    fn runtime() -> ChatRuntime {
        ChatRuntime::try_from_registrations([FIRST, SECOND].map(|id| {
            ProviderRegistration::planned(ProviderManifest {
                descriptor: ProviderDescriptor {
                    id,
                    label: "Fixture",
                    short_label: "F",
                    glyph: "F",
                    accent: ProviderAccent::Cyan,
                    summary: "archive fixture",
                    since: "2000-01-01",
                },
                profile: ProviderProfileId("fixture"),
                target_capabilities: &[Capability::ReadSessions],
                delivery_slice: "PC1a",
                evidence: "unit test",
            })
        }))
        .expect("registry")
    }

    fn populated() -> ChatRuntime {
        let mut runtime = runtime();
        for (index, provider) in [FIRST, SECOND].into_iter().enumerate() {
            runtime
                .set_draft(provider, "before a session".into())
                .unwrap();
            for offset in 1..=2 {
                let id = ChatSessionId(index as u64 * 10 + offset);
                runtime
                    .upsert_session(
                        provider,
                        SessionSummary {
                            id,
                            provider_id: provider,
                            provider_session_id: Some(ProviderSessionId(format!(
                                "native-{}",
                                id.0
                            ))),
                            title: format!("Session {}", id.0),
                            workspace: "Project label".into(),
                            group: SessionGroup::Recent,
                            state: SessionState::Running,
                        },
                    )
                    .unwrap();
                runtime.select_session(provider, id).unwrap();
                runtime
                    .set_draft(provider, format!("draft {}", id.0))
                    .unwrap();
            }
        }
        runtime
    }

    fn contents() -> Vec<ChatEventContent> {
        vec![
            ChatEventContent::SessionStarted,
            ChatEventContent::ModelSelected {
                model: "model".into(),
            },
            ChatEventContent::UserMessage("question\n世界".into()),
            ChatEventContent::AssistantMessage("answer".into()),
            ChatEventContent::PlanUpdated("plan".into()),
            ChatEventContent::StatusUpdated("working".into()),
            ChatEventContent::ToolStarted {
                label: "tool".into(),
            },
            ChatEventContent::ToolCompleted {
                label: "tool".into(),
                succeeded: true,
            },
            ChatEventContent::DiffUpdated("diff".into()),
            ChatEventContent::ApprovalRequested {
                summary: "approve".into(),
            },
            ChatEventContent::ApprovalResolved { accepted: false },
            ChatEventContent::UsageUpdated {
                summary: "usage".into(),
                usage: TokenUsage {
                    input_tokens: Some(0),
                    cache_read_tokens: Some(1),
                    cache_write_tokens: Some(2),
                    output_tokens: Some(3),
                    total_tokens: Some(6),
                    context_used: None,
                    context_size: Some(100),
                },
            },
            ChatEventContent::TurnCompleted,
            ChatEventContent::TurnInterrupted,
            ChatEventContent::TurnFailed {
                message: "failure".into(),
            },
            ChatEventContent::ProviderWarning("warning".into()),
            ChatEventContent::Unknown(BoundedNativePayload::redacted("native", "diagnostic")),
        ]
    }

    #[test]
    fn round_trip_preserves_every_event_variant_and_inactive_drafts() {
        let mut source = populated();
        for (index, content) in contents().into_iter().enumerate() {
            source
                .append_event(
                    FIRST,
                    ChatEvent {
                        id: ChatEventId(index as u64 + 1),
                        provider_id: FIRST,
                        session_id: ChatSessionId(1),
                        turn_id: Some(TurnId("turn-42".into())),
                        content,
                    },
                )
                .unwrap();
        }
        let mut destination = runtime();
        destination
            .restore_archive(&source.archive().unwrap())
            .unwrap();
        assert_eq!(destination.selected_provider_id(), Some(SECOND));
        for (before, after) in source.providers.iter().zip(&destination.providers) {
            assert_eq!(before.drafts, after.drafts);
            assert_eq!(before.active_session, after.active_session);
            assert_eq!(before.registration.events, after.registration.events);
            for (original, restored) in before
                .registration
                .sessions
                .iter()
                .zip(&after.registration.sessions)
            {
                let mut expected = original.clone();
                expected.state = SessionState::ReadOnly;
                assert_eq!(&expected, restored);
            }
        }
        assert_eq!(destination.providers[0].drafts.len(), 3);
    }

    #[test]
    fn restore_and_clear_preserve_current_machine_authority() {
        let mut source = populated();
        source
            .set_activation(FIRST, ActivationState::Ready)
            .unwrap();
        source
            .set_account(
                FIRST,
                Some(AccountSummary {
                    display_name: "old machine account".into(),
                    plan: Some("old plan".into()),
                }),
            )
            .unwrap();
        source
            .qualify_capabilities(FIRST, [Capability::ReadSessions])
            .unwrap();
        let archive = source.archive().unwrap();
        let serialized = archive.to_string();
        for excluded in [
            "old machine account",
            "old plan",
            "activation",
            "capabilities",
            "profile",
        ] {
            assert!(!serialized.contains(excluded));
        }
        let mut destination = runtime();
        let activation = destination.providers[0].registration.activation.clone();
        destination.restore_archive(&archive).unwrap();
        let registration = &destination.providers[0].registration;
        assert_eq!(registration.activation, activation);
        assert!(registration.account.is_none());
        assert!(registration.qualified_capabilities.is_empty());
        destination
            .set_activation(FIRST, ActivationState::Ready)
            .unwrap();
        destination
            .set_account(
                FIRST,
                Some(AccountSummary {
                    display_name: "current account".into(),
                    plan: None,
                }),
            )
            .unwrap();
        destination
            .qualify_capabilities(FIRST, [Capability::ReadSessions])
            .unwrap();
        destination.restore_archive(&archive).unwrap();
        destination.clear_project_state();
        let registration = &destination.providers[0].registration;
        assert_eq!(registration.activation, ActivationState::Ready);
        assert_eq!(
            registration.account.as_ref().unwrap().display_name,
            "current account"
        );
        assert!(registration
            .qualified_capabilities
            .contains(&Capability::ReadSessions));
        for entry in &destination.providers {
            assert!(entry.registration.sessions.is_empty());
            assert!(entry.registration.events.is_empty());
            assert!(entry.drafts.is_empty());
            assert!(entry.active_session.is_none());
        }
    }

    #[test]
    fn unavailable_provider_is_retained_without_registering_or_interpreting_events() {
        let mut archive = populated().archive().unwrap();
        archive["selected_provider"] = Value::String("unavailable".into());
        let mut unavailable = archive["providers"][0].clone();
        unavailable["provider"] = Value::String("unavailable".into());
        unavailable["events"] = serde_json::json!([{
            "id": 1, "session": 1, "turn": null, "content": {"FutureVariant": "preserve me"}
        }]);
        archive["providers"]
            .as_array_mut()
            .unwrap()
            .push(unavailable.clone());
        let mut destination = runtime();
        destination.restore_archive(&archive).unwrap();
        assert_eq!(destination.providers.len(), 2);
        assert_eq!(destination.selected_provider_id(), Some(FIRST));
        assert_eq!(destination.providers[1].registration.sessions.len(), 2);
        assert_eq!(destination.archive().unwrap()["providers"][2], unavailable);
        destination.clear_project_state();
        assert_eq!(
            destination.archive().unwrap()["providers"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        unavailable["activation"] = serde_json::json!("Ready");
        archive["providers"][2] = unavailable;
        assert!(destination.restore_archive(&archive).is_err());
    }

    #[test]
    fn invalid_or_future_archives_leave_existing_state_untouched() {
        let mut runtime = populated();
        let original = runtime.archive().unwrap();
        let mut corruptions = Vec::new();
        let mut future = original.clone();
        future["v"] = serde_json::json!(VERSION + 1);
        corruptions.push(future);
        let mut bad_reference = original.clone();
        bad_reference["providers"][1]["active_session"] = serde_json::json!(900);
        corruptions.push(bad_reference);
        let mut duplicate = original.clone();
        duplicate["providers"][1]["sessions"][1]["id"] = serde_json::json!(11);
        corruptions.push(duplicate);
        let mut injected_authority = original.clone();
        injected_authority["providers"][0]["activation"] = serde_json::json!("Ready");
        corruptions.push(injected_authority);
        let mut bad_draft = original.clone();
        bad_draft["providers"][1]["drafts"][0]["session"] = serde_json::json!(99);
        corruptions.push(bad_draft);
        let mut duplicate_draft = original.clone();
        duplicate_draft["providers"][1]["drafts"][1]["session"] = Value::Null;
        corruptions.push(duplicate_draft);
        let mut bad_event = original.clone();
        bad_event["providers"][1]["events"] = serde_json::json!([{
            "id": 7, "session": 99, "turn": null, "content": "TurnCompleted"
        }]);
        corruptions.push(bad_event);
        let mut huge_id = original.clone();
        huge_id["providers"][0]["sessions"][0]["id"] = serde_json::json!(u64::MAX);
        corruptions.push(huge_id);
        let mut exhausted_event = original.clone();
        exhausted_event["providers"][0]["events"] = serde_json::json!([{
            "id": MAX_ID + 1, "session": 1, "turn": null, "content": "TurnCompleted"
        }]);
        corruptions.push(exhausted_event);
        let mut exhausted_turn = original.clone();
        exhausted_turn["providers"][0]["events"] = serde_json::json!([{
            "id": 1, "session": 1, "turn": format!("grok-turn-{}", MAX_ID + 1), "content": "TurnCompleted"
        }]);
        corruptions.push(exhausted_turn);
        for corrupt in corruptions {
            assert!(runtime.restore_archive(&corrupt).is_err());
            assert_eq!(runtime.archive().unwrap(), original);
        }
    }

    #[test]
    fn size_and_native_payload_limits_are_enforced() {
        let mut runtime = populated();
        let original = runtime.archive().unwrap();
        let mut oversized = original.clone();
        oversized["providers"][0]["drafts"][0]["text"] = Value::String("x".repeat(MAX_TEXT + 1));
        assert!(runtime.restore_archive(&oversized).is_err());
        let mut native = original.clone();
        native["providers"][0]["events"] = serde_json::json!([{
            "id": 1, "session": 1, "turn": null,
            "content": {"Unknown": {
                "schema": "native", "redacted_summary": "x".repeat(MAX_NATIVE_PAYLOAD_BYTES + 1),
                "original_bytes": MAX_NATIVE_PAYLOAD_BYTES + 1, "truncated": false
            }}
        }]);
        assert!(runtime.restore_archive(&native).is_err());
        assert_eq!(runtime.archive().unwrap(), original);
    }
}
