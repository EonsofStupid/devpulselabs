//! Project-correlated native session listing and atomic history attachment.
//! Capability publication is separate from transport support: only an exactly
//! qualified project profile can enter this path in production.
use super::*;
use dpl_chat_runtime::seam::{RuntimeFailure, RuntimeHistoryItem, RuntimeSessionSummary};
use dpl_chat_window::NativeSessionStatus;
use std::time::Duration;

const MAX_LIST_PAGES: usize = 32;
const MAX_LIST_ROWS: usize = 2048;
const MAX_HISTORY_ITEMS: usize = 32_768;
const MAX_HISTORY_EVENTS: usize = 65_536;
const MAX_HISTORY_BYTES: usize = 8 * 1024 * 1024;
const LOAD_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Default)]
pub(super) struct State {
    qualified: BTreeSet<ProviderId>,
    loads: BTreeMap<ProviderId, PendingLoad>,
    lists: BTreeMap<ProviderId, PendingList>,
}

struct PendingLoad {
    generation: project::Generation,
    session: ChatSessionId,
    native: String,
    started: Instant,
}

struct PendingList {
    generation: project::Generation,
    cursors: BTreeSet<String>,
    pages: usize,
    rows: usize,
    started: Instant,
}

pub(super) fn busy(state: &Shared) -> bool {
    !state.native.loads.is_empty() || !state.native.lists.is_empty()
}

pub(super) fn provider_busy(state: &Shared, provider: ProviderId) -> bool {
    state.native.loads.contains_key(&provider) || state.native.lists.contains_key(&provider)
}

pub(super) fn qualified(state: &Shared, provider: ProviderId) -> bool {
    state.native.qualified.contains(&provider)
}

pub(super) fn connected(state: &mut Shared, provider: ProviderId, supported: bool) -> bool {
    // A schema-capable runtime is not proof of a portable project profile.
    let qualified = supported
        && provider == dpl_chat_codex::PROVIDER
        && state
            .codex_activation
            .runtime_profile()
            .is_some_and(|profile| profile.native_sessions_qualified());
    if qualified {
        state.native.qualified.insert(provider);
    } else {
        state.native.qualified.remove(&provider);
    }
    publish(state, provider);
    qualified
}

fn publish(state: &mut Shared, provider: ProviderId) {
    state.window.set_native_session_status(
        provider,
        NativeSessionStatus {
            qualified: qualified(state, provider),
            loading: state.native.loads.get(&provider).map(|load| load.session),
            listing: state.native.lists.contains_key(&provider),
        },
    );
}

pub(super) fn reset(state: &mut Shared) {
    let providers = state.native.qualified.iter().copied().collect::<Vec<_>>();
    state.native = State::default();
    for provider in providers {
        publish(state, provider);
    }
}

pub(super) fn stopped(state: &mut Shared, provider: ProviderId) {
    state.native.loads.remove(&provider);
    state.native.lists.remove(&provider);
    state.native.qualified.remove(&provider);
    // A native id on disk is not an attached conversation in a new process.
    // Reconnection must explicitly resume it before any subsequent prompt.
    let sessions = state
        .window
        .provider_view(provider)
        .map_or_else(Vec::new, |view| view.registration.sessions);
    for session in sessions {
        if session.provider_session_id.is_some() {
            let _ = state.window.set_provider_session_state(
                provider,
                session.id,
                SessionState::ReadOnly,
            );
        }
    }
    state
        .seam_sessions
        .retain(|(owner, _), _| *owner != provider);
    state.seam_turns.retain(|(owner, _), _| *owner != provider);
    publish(state, provider);
}

fn retire(state: &mut Shared, provider: ProviderId) {
    // Without an operation nonce, a timed-out queue must never feed a later
    // request for the same native id. Drop the entire old receiver off-thread.
    if let Some(runtime) = state.runtimes.remove(&provider) {
        std::thread::spawn(move || drop(runtime));
    }
    stopped(state, provider);
}

fn provider_idle(state: &Shared, provider: ProviderId) -> bool {
    !state
        .ai_calls
        .values()
        .any(|call| call.provider == provider)
        && !state
            .seam_permissions
            .keys()
            .any(|(owner, _)| *owner == provider)
        && state.window.provider_view(provider).is_some_and(|view| {
            view.registration.sessions.iter().all(|session| {
                !matches!(
                    session.state,
                    SessionState::Running | SessionState::Attention
                )
            })
        })
}

pub(super) fn request_load(state: &mut Shared, provider: ProviderId, session: ChatSessionId) {
    if !project::ready(state)
        || !qualified(state, provider)
        || provider_busy(state, provider)
        || !provider_idle(state, provider)
    {
        return;
    }
    let Some(summary) = state.window.provider_view(provider).and_then(|view| {
        view.registration
            .sessions
            .into_iter()
            .find(|saved| saved.id == session && saved.state == SessionState::ReadOnly)
    }) else {
        return;
    };
    let Some(native) = summary.provider_session_id.map(|id| id.0) else {
        return;
    };
    let generation = project::generation(state);
    if send_seam_command(
        state,
        provider,
        RuntimeCommand::LoadSession {
            provider_session_id: native.clone(),
        },
    ) {
        state.native.loads.insert(
            provider,
            PendingLoad {
                generation,
                session,
                native,
                started: Instant::now(),
            },
        );
        state.window.set_composer_status("Loading this saved chat. Its archive and draft remain available until loading succeeds.");
        publish(state, provider);
    }
}

pub(super) fn request_list(state: &mut Shared, provider: ProviderId) {
    if !project::ready(state)
        || !qualified(state, provider)
        || provider_busy(state, provider)
        || !provider_idle(state, provider)
    {
        return;
    }
    let generation = project::generation(state);
    if send_seam_command(
        state,
        provider,
        RuntimeCommand::ListSessions { cursor: None },
    ) {
        state.native.lists.insert(
            provider,
            PendingList {
                generation,
                cursors: BTreeSet::new(),
                pages: 0,
                rows: 0,
                started: Instant::now(),
            },
        );
        publish(state, provider);
    }
}

pub(super) fn sessions_page(
    state: &mut Shared,
    provider: ProviderId,
    sessions: Vec<RuntimeSessionSummary>,
    next: Option<String>,
) {
    let Some(mut pending) = state.native.lists.remove(&provider) else {
        return;
    };
    if pending.generation != project::generation(state) || !qualified(state, provider) {
        publish(state, provider);
        return;
    }
    pending.pages += 1;
    pending.rows = pending.rows.saturating_add(sessions.len());
    if pending.pages > MAX_LIST_PAGES || pending.rows > MAX_LIST_ROWS {
        state.window.set_composer_status(
            "Saved chat listing exceeded its limit. Existing chats and drafts were kept.",
        );
        publish(state, provider);
        return;
    }
    let Some(workspace) = project::workspace(state) else {
        publish(state, provider);
        return;
    };
    let existing = state
        .window
        .provider_view(provider)
        .map_or_else(Vec::new, |view| view.registration.sessions);
    let mut summaries = Vec::new();
    let mut next_id = state.next_chat_session_id;
    let mut seen = BTreeSet::new();
    for session in sessions {
        if !same_workspace(&session.working_directory, &workspace)
            || session.provider_session_id.is_empty()
            || session.provider_session_id.len() > 1024
            || !seen.insert(session.provider_session_id.clone())
        {
            continue;
        }
        if let Some(saved) = existing.iter().find(|saved| {
            saved
                .provider_session_id
                .as_ref()
                .is_some_and(|id| id.0 == session.provider_session_id)
        }) {
            // Existing archived/live metadata and every draft keep their owner.
            summaries.push(saved.clone());
        } else {
            if next_id >= u64::MAX / 2 {
                break;
            }
            summaries.push(SessionSummary {
                id: ChatSessionId(next_id),
                provider_id: provider,
                provider_session_id: Some(ProviderSessionId(session.provider_session_id)),
                title: session.title.unwrap_or_else(|| "Saved chat".into()),
                workspace: project::workspace_label(state),
                group: SessionGroup::Recent,
                state: SessionState::ReadOnly,
            });
            next_id += 1;
        }
    }
    if let Err(error) = state.window.merge_native_sessions(provider, summaries) {
        state
            .window
            .set_composer_status(format!("Saved chats could not be listed: {error}"));
        publish(state, provider);
        return;
    }
    state.next_chat_session_id = next_id;
    if let Some(cursor) = next {
        if cursor.len() > 8192
            || !pending.cursors.insert(cursor.clone())
            || pending.pages >= MAX_LIST_PAGES
        {
            state.window.set_composer_status("Saved chat listing stopped at an invalid or repeated page. Existing chats were kept.");
        } else if send_seam_command(
            state,
            provider,
            RuntimeCommand::ListSessions {
                cursor: Some(cursor),
            },
        ) {
            state.native.lists.insert(provider, pending);
        }
    }
    publish(state, provider);
}

pub(super) fn session_loaded(
    state: &mut Shared,
    provider: ProviderId,
    session: RuntimeSessionSummary,
    items: Vec<RuntimeHistoryItem>,
) {
    let Some(pending) = state.native.loads.get(&provider) else {
        return;
    };
    if pending.native != session.provider_session_id {
        return;
    }
    let pending = state.native.loads.remove(&provider).expect("checked above");
    if pending.generation != project::generation(state)
        || !qualified(state, provider)
        || !project::ready(state)
        || !project::workspace(state)
            .is_some_and(|workspace| same_workspace(&workspace, &session.working_directory))
    {
        state.window.set_composer_status(
            "The saved chat no longer matches the active project. Its archive was kept.",
        );
        if pending.generation == project::generation(state) {
            retire(state, provider);
        }
        publish(state, provider);
        return;
    }
    let existing = state
        .window
        .provider_view(provider)
        .map_or_else(Vec::new, |view| {
            view.registration
                .events
                .into_iter()
                .filter(|event| event.session_id == pending.session)
                .collect()
        });
    let projected = history_events(
        provider,
        pending.session,
        &items,
        &existing,
        state.next_chat_event_id,
    );
    let result = projected.and_then(|(events, next)| {
        let visible = |event: &ChatEvent| event.content.presentation() == dpl_chat_core::EventPresentation::Conversation;
        if existing.iter().any(visible) && !events.iter().any(visible) {
            return Err("The provider did not return this chat's saved conversation. The existing messages remain read-only.".into());
        }
        let summary = SessionSummary {
            id: pending.session,
            provider_id: provider,
            provider_session_id: Some(ProviderSessionId(pending.native.clone())),
            title: session.title.unwrap_or_else(|| "Saved chat".into()),
            workspace: project::workspace_label(state),
            group: SessionGroup::Active,
            state: SessionState::Idle,
        };
        state
            .window
            .replace_native_session_history(provider, summary, events)?;
        state.next_chat_event_id = next;
        Ok(())
    });
    match result {
        Ok(()) => {
            state
                .seam_sessions
                .insert((provider, pending.native), pending.session);
            state
                .window
                .set_composer_status("Saved chat resumed. Your draft is ready.");
        }
        Err(error) => {
            retire(state, provider);
            state.window.set_composer_status(format!(
                "Saved chat could not be loaded; its archive and draft were kept: {error}"
            ));
        }
    }
    publish(state, provider);
}

fn history_events(
    provider: ProviderId,
    session: ChatSessionId,
    items: &[RuntimeHistoryItem],
    existing: &[ChatEvent],
    mut next: u64,
) -> Result<(Vec<ChatEvent>, u64), String> {
    if items.len() > MAX_HISTORY_ITEMS {
        return Err("Too many native history items".into());
    }
    let mut seen = BTreeSet::new();
    let mut bytes = 0usize;
    let mut events = Vec::new();
    for item in items {
        if item.provider_turn_id.is_empty()
            || item.provider_item_id.is_empty()
            || item.provider_turn_id.len() > 1024
            || item.provider_item_id.len() > 1024
            || !seen.insert((&item.provider_turn_id, &item.provider_item_id))
        {
            return Err("Invalid or repeated native history identity".into());
        }
        bytes = bytes.saturating_add(item.provider_turn_id.len() + item.provider_item_id.len());
        for update in &item.updates {
            let content = match update {
                RuntimeUpdate::UserText(text) => ChatEventContent::UserMessage(text.clone()),
                RuntimeUpdate::AssistantText(text) => {
                    ChatEventContent::AssistantMessage(text.clone())
                }
                RuntimeUpdate::ThoughtText(text) | RuntimeUpdate::SessionInfo(text) => {
                    ChatEventContent::StatusUpdated(text.clone())
                }
                RuntimeUpdate::Tool(text) => ChatEventContent::ToolStarted {
                    label: text.clone(),
                },
                RuntimeUpdate::Plan(text) => ChatEventContent::PlanUpdated(text.clone()),
                RuntimeUpdate::Other(text) => ChatEventContent::Unknown(
                    BoundedNativePayload::redacted("provider.native-history", text.clone()),
                ),
                RuntimeUpdate::AvailableCommands { count } => {
                    ChatEventContent::StatusUpdated(format!("{count} provider commands available"))
                }
                RuntimeUpdate::Usage { used, size } => ChatEventContent::UsageUpdated {
                    summary: format!("{used} of {size} context tokens"),
                    usage: TokenUsage {
                        context_used: Some(*used),
                        context_size: Some(*size),
                        ..TokenUsage::default()
                    },
                },
            };
            bytes = bytes.saturating_add(
                serde_json::to_vec(&content)
                    .map_err(|error| error.to_string())?
                    .len(),
            );
            if bytes > MAX_HISTORY_BYTES
                || events.len() >= MAX_HISTORY_EVENTS
                || next >= u64::MAX / 2
            {
                return Err("Native history exceeds the project archive limit".into());
            }
            let turn_id = Some(TurnId(item.provider_turn_id.clone()));
            let id = match existing
                .get(events.len())
                .filter(|event| event.turn_id == turn_id && event.content == content)
            {
                Some(event) => event.id,
                None => {
                    let id = ChatEventId(next);
                    next += 1;
                    id
                }
            };
            events.push(ChatEvent {
                id,
                provider_id: provider,
                session_id: session,
                turn_id,
                content,
            });
        }
    }
    Ok((events, next))
}

/// Load failures are not failed live turns: do not append failure events or
/// change a read-only archive into an apparently attached Idle session.
pub(super) fn failure(state: &mut Shared, provider: ProviderId, failure: &RuntimeFailure) -> bool {
    let matched = state.native.loads.get(&provider).is_some_and(|load| {
        failure
            .provider_session_id
            .as_ref()
            .is_none_or(|native| native == &load.native)
    });
    let listing_failed =
        failure.provider_session_id.is_none() && state.native.lists.contains_key(&provider);
    if matched || listing_failed {
        retire(state, provider);
    }
    if matched {
        state.window.set_composer_status(format!(
            "Saved chat could not be resumed; its archive and draft were kept: {}",
            plain_failure(&failure.reason).message
        ));
    }
    publish(state, provider);
    matched && failure.provider_session_id.is_some()
}

pub(super) fn tick(state: &mut Shared) {
    let expired: BTreeSet<_> = state
        .native
        .loads
        .iter()
        .filter(|(_, load)| load.started.elapsed() >= LOAD_TIMEOUT)
        .map(|(provider, _)| *provider)
        .chain(
            state
                .native
                .lists
                .iter()
                .filter(|(_, list)| list.started.elapsed() >= LOAD_TIMEOUT)
                .map(|(provider, _)| *provider),
        )
        .collect();
    for provider in expired {
        retire(state, provider);
        state
            .window
            .set_composer_status("Saved chat loading timed out. The archive and draft were kept.");
        publish(state, provider);
    }
}

fn same_workspace(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        // Windows canonicalization adds a verbatim prefix. Remove only that
        // spelling difference; never case-fold directory names or accept '..'.
        let normalize = |path: &Path| -> Option<PathBuf> {
            let mut result = PathBuf::new();
            for component in path.components() {
                match component {
                    Component::Prefix(prefix) => match prefix.kind() {
                        Prefix::VerbatimDisk(letter) | Prefix::Disk(letter) => {
                            result.push(format!("{}:", char::from(letter.to_ascii_uppercase())))
                        }
                        Prefix::VerbatimUNC(server, share) | Prefix::UNC(server, share) => {
                            result.push(Path::new(r"\\").join(server).join(share));
                        }
                        _ => return None,
                    },
                    Component::ParentDir => return None,
                    _ => result.push(component.as_os_str()),
                }
            }
            Some(result)
        };
        let left = normalize(left);
        left.is_some() && left == normalize(right)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
pub(super) fn qualify_fixture(state: &mut Shared, provider: ProviderId) {
    state.native.qualified.insert(provider);
    publish(state, provider);
}

#[cfg(test)]
pub(super) fn expire_fixture(state: &mut Shared, provider: ProviderId) {
    state.native.loads.get_mut(&provider).unwrap().started = Instant::now() - LOAD_TIMEOUT;
    tick(state);
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn canonical_windows_prefix_does_not_change_project_identity() {
        assert!(same_workspace(
            Path::new(r"D:\Projects\A"),
            Path::new(r"\\?\D:\Projects\A")
        ));
        assert!(!same_workspace(
            Path::new(r"D:\Projects\A"),
            Path::new(r"\\?\D:\Projects\B")
        ));
        assert!(!same_workspace(
            Path::new(r"D:\Projects\A"),
            Path::new(r"D:\Projects\B\..\A")
        ));
    }
}
