//! The trust gate (AH1b, chat plan §4.8.2 rule 7): no AI process starts with a
//! project folder as its working directory until the operator has said what
//! it may do there.
//!
//! Grants are stored on this computer beside the project registry
//! (`dpl_chat_projects::trust`), never in the project, so a cloned project
//! cannot bring its own. A grant is the ceiling of the AI's permission mode in
//! that project (ruling 2026-11-16): the picker offers nothing above it, and a
//! saved mode above it is lowered there.

use super::{project, Shared};
use dpl_ai_core::approval::PermissionMode;
use dpl_ai_core::provider::ProviderId;
use dpl_chat_projects::trust::Grant;
use std::collections::{BTreeMap, BTreeSet};

/// The active project's answers, and which AIs were already asked this time
/// the project was opened (a dialog the operator closed is not raised again
/// on the next frame).
#[derive(Debug, Default)]
pub(super) struct State {
    grants: BTreeMap<ProviderId, Grant>,
    asked: BTreeSet<ProviderId>,
}

/// The AIs that work inside a project folder: the catalog AIs DevPULSE drives
/// with an adapter.
fn project_ais() -> impl Iterator<Item = ProviderId> {
    dpl_ai_core::catalog::catalog()
        .entries
        .iter()
        .filter(|entry| entry.adapter.is_some())
        .map(|entry| entry.id())
}

/// The most a grant allows. `None`: the AI may not start in the project.
pub(super) fn ceiling_of(grant: Grant) -> Option<PermissionMode> {
    PermissionMode::ceiling(grant.read, grant.change, grant.run)
}

/// Read the active project's grants and show them. Called when a project
/// becomes active, before any runtime can start in it.
pub(super) fn load(state: &mut Shared) {
    state.trust = State::default();
    let mut grants = BTreeMap::new();
    if let Some((registry, project)) = project::trust_store(state) {
        for provider in project_ais() {
            match registry.trust(project, provider.0) {
                Ok(Some(grant)) => {
                    grants.insert(provider, grant);
                }
                Ok(None) => {}
                Err(error) => {
                    // An unreadable answer is no answer: the AI stays stopped
                    // and the operator is asked again.
                    tracing::warn!(%error, %provider, "trust could not be read");
                }
            }
        }
    }
    state.trust.grants = grants;
    publish(state);
}

/// Show every project AI's trust to the window: its ceiling, or not allowed.
fn publish(state: &mut Shared) {
    let in_project = project::configured(state);
    for provider in project_ais() {
        let trust = in_project
            .then(|| state.trust.grants.get(&provider).copied())
            .map(|grant| grant.and_then(ceiling_of));
        state.window.set_provider_trust(provider, trust);
        if let Some(Some(ceiling)) = trust {
            if let Some(mode) = state.permission_modes.get(&provider).copied() {
                if !mode.within(ceiling) {
                    state
                        .window
                        .set_provider_permission_mode(provider, Some(mode.capped(ceiling)));
                }
            }
        }
    }
}

/// What `provider` may do in the active project, when it may start at all.
///
/// With no answer yet, the dialog is raised once (one AI at a time) and
/// `None` is returned, so the runtime simply does not start this frame.
pub(super) fn runtime_grant(state: &mut Shared, provider: ProviderId) -> Option<Grant> {
    match state.trust.grants.get(&provider).copied() {
        Some(grant) if grant.allows_start() => Some(grant),
        Some(_) => None,
        None => {
            if !state.trust.asked.contains(&provider) && state.window.trust_dialog_open().is_none()
            {
                state.trust.asked.insert(provider);
                ask(state, provider, Grant::default());
            }
            None
        }
    }
}

/// The permission mode `provider` runs with in the active project: the saved
/// mode, lowered to the project's ceiling.
pub(super) fn effective_mode(state: &Shared, provider: ProviderId) -> PermissionMode {
    let saved = state
        .permission_modes
        .get(&provider)
        .copied()
        .unwrap_or_default();
    match state
        .trust
        .grants
        .get(&provider)
        .copied()
        .and_then(ceiling_of)
    {
        Some(ceiling) => saved.capped(ceiling),
        None => saved,
    }
}

fn ask(state: &mut Shared, provider: ProviderId, current: Grant) {
    let project = project::workspace_label(state);
    state.window.request_trust(provider, project, current);
}

/// The operator asked to change an AI's trust in the active project.
pub(super) fn edit(state: &mut Shared, provider: ProviderId) {
    if !project::configured(state) {
        return;
    }
    let current = state
        .trust
        .grants
        .get(&provider)
        .copied()
        .unwrap_or_default();
    ask(state, provider, current);
}

/// Whether an answer changes anything: closing the dialog (None) or giving
/// the same answer does not. Any real change stops a running runtime, which
/// starts again on the next frame with what is now granted (Grok's client
/// capabilities are fixed when it starts). Pure, so the rule is tested alone.
pub(super) fn changes(old: Option<Grant>, new: Option<Grant>) -> bool {
    new.is_some() && new != old
}

/// Save the operator's answer and apply it.
pub(super) fn answer(state: &mut Shared, provider: ProviderId, grant: Option<Grant>) {
    let old = state.trust.grants.get(&provider).copied();
    if !changes(old, grant) {
        return;
    }
    let Some(grant) = grant else {
        return;
    };
    let Some((registry, project)) = project::trust_store(state) else {
        return;
    };
    if let Err(error) = registry.set_trust(project, provider.0, grant) {
        tracing::warn!(%error, %provider, "trust could not be saved");
        state
            .window
            .set_composer_status("Your choice could not be saved, so nothing changed.");
        return;
    }
    tracing::info!(%provider, read = grant.read, change = grant.change, run = grant.run, "trust answered");
    state.trust.grants.insert(provider, grant);
    publish(state);
    super::stop_project_runtime(state, provider);
}

#[cfg(test)]
mod tests {
    use super::*;

    const READ: Grant = Grant {
        read: true,
        change: false,
        run: false,
    };
    const ALL: Grant = Grant {
        read: true,
        change: true,
        run: true,
    };

    #[test]
    fn closing_the_dialog_or_the_same_answer_changes_nothing() {
        assert!(!changes(None, None));
        assert!(!changes(Some(ALL), None));
        assert!(!changes(Some(ALL), Some(ALL)));
    }

    #[test]
    fn every_new_answer_restarts_including_dont_allow() {
        assert!(changes(None, Some(READ)));
        assert!(changes(Some(READ), Some(ALL)));
        assert!(changes(Some(ALL), Some(Grant::default())));
    }

    #[test]
    fn no_grant_means_no_ceiling_and_no_start() {
        assert_eq!(ceiling_of(Grant::default()), None);
        assert_eq!(ceiling_of(READ), Some(PermissionMode::ReadOnly));
        assert_eq!(ceiling_of(ALL), Some(PermissionMode::Workspace));
    }
}
