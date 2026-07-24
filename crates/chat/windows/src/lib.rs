//! Stable lifecycle state for attached Chat windows.
//!
//! Native viewport creation belongs to the shell coordinator in CH8. This
//! crate defines ownership and transitions without depending on egui.

use dpl_chat_core::{ChatSessionId, ProviderId, TurnId};
use dpl_hub::Route;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatWindowId(pub u64);

impl fmt::Display for ChatWindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chat-window-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    Conversation,
    Plan,
    Diff,
    Tool,
    Artifact,
    ProviderDiagnostics,
    Council,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowBinding {
    pub provider_id: Option<ProviderId>,
    pub session_id: Option<ChatSessionId>,
    pub turn_id: Option<TurnId>,
    pub route: Option<Route>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowLifecycle {
    Requested,
    Opening,
    Visible,
    Closing,
    Closed,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatWindowRecord {
    pub id: ChatWindowId,
    pub generation: ViewportGeneration,
    pub kind: WindowKind,
    pub binding: WindowBinding,
    pub lifecycle: WindowLifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    Unknown(ChatWindowId),
    InvalidTransition {
        id: ChatWindowId,
        from: WindowLifecycle,
        to: &'static str,
    },
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(id) => write!(f, "{id} is not registered"),
            Self::InvalidTransition { id, from, to } => {
                write!(f, "{id} cannot transition from {from:?} to {to}")
            }
        }
    }
}

impl std::error::Error for WindowError {}

#[derive(Debug, Clone, Default)]
pub struct ChatWindowRegistry {
    next_id: u64,
    windows: Vec<ChatWindowRecord>,
}

impl ChatWindowRegistry {
    pub fn request(&mut self, kind: WindowKind, binding: WindowBinding) -> ChatWindowId {
        self.next_id = self.next_id.saturating_add(1);
        let id = ChatWindowId(self.next_id);
        self.windows.push(ChatWindowRecord {
            id,
            generation: ViewportGeneration(1),
            kind,
            binding,
            lifecycle: WindowLifecycle::Requested,
        });
        id
    }

    pub fn mark_opening(&mut self, id: ChatWindowId) -> Result<(), WindowError> {
        self.transition(id, "Opening", |current| {
            matches!(current, WindowLifecycle::Requested).then_some(WindowLifecycle::Opening)
        })
    }

    pub fn mark_visible(&mut self, id: ChatWindowId) -> Result<(), WindowError> {
        self.transition(id, "Visible", |current| {
            matches!(current, WindowLifecycle::Opening).then_some(WindowLifecycle::Visible)
        })
    }

    pub fn request_close(&mut self, id: ChatWindowId) -> Result<(), WindowError> {
        self.transition(id, "Closing", |current| {
            matches!(
                current,
                WindowLifecycle::Requested
                    | WindowLifecycle::Opening
                    | WindowLifecycle::Visible
                    | WindowLifecycle::Failed { .. }
            )
            .then_some(WindowLifecycle::Closing)
        })
    }

    pub fn mark_closed(&mut self, id: ChatWindowId) -> Result<(), WindowError> {
        self.transition(id, "Closed", |current| {
            matches!(current, WindowLifecycle::Closing).then_some(WindowLifecycle::Closed)
        })
    }

    pub fn mark_failed(
        &mut self,
        id: ChatWindowId,
        reason: impl Into<String>,
    ) -> Result<(), WindowError> {
        let reason = reason.into();
        self.transition(id, "Failed", |current| {
            matches!(
                current,
                WindowLifecycle::Requested | WindowLifecycle::Opening
            )
            .then(|| WindowLifecycle::Failed { reason })
        })
    }

    pub fn reopen(&mut self, id: ChatWindowId) -> Result<(), WindowError> {
        let window = self.window_mut(id)?;
        if !matches!(
            window.lifecycle,
            WindowLifecycle::Closed | WindowLifecycle::Failed { .. }
        ) {
            return Err(WindowError::InvalidTransition {
                id,
                from: window.lifecycle.clone(),
                to: "Requested",
            });
        }
        window.generation = ViewportGeneration(window.generation.0.saturating_add(1));
        window.lifecycle = WindowLifecycle::Requested;
        Ok(())
    }

    pub fn records(&self) -> &[ChatWindowRecord] {
        &self.windows
    }

    pub fn visible_count(&self) -> usize {
        self.windows
            .iter()
            .filter(|window| matches!(window.lifecycle, WindowLifecycle::Visible))
            .count()
    }

    fn transition(
        &mut self,
        id: ChatWindowId,
        target: &'static str,
        next: impl FnOnce(&WindowLifecycle) -> Option<WindowLifecycle>,
    ) -> Result<(), WindowError> {
        let window = self.window_mut(id)?;
        let from = window.lifecycle.clone();
        let Some(next) = next(&window.lifecycle) else {
            return Err(WindowError::InvalidTransition {
                id,
                from,
                to: target,
            });
        };
        window.lifecycle = next;
        Ok(())
    }

    fn window_mut(&mut self, id: ChatWindowId) -> Result<&mut ChatWindowRecord, WindowError> {
        self.windows
            .iter_mut()
            .find(|window| window.id == id)
            .ok_or(WindowError::Unknown(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_is_explicit_and_stable() {
        let mut windows = ChatWindowRegistry::default();
        let id = windows.request(WindowKind::Conversation, WindowBinding::default());
        windows.mark_opening(id).expect("opening");
        windows.mark_visible(id).expect("visible");
        assert_eq!(windows.visible_count(), 1);
        windows.request_close(id).expect("closing");
        windows.mark_closed(id).expect("closed");
        assert_eq!(windows.visible_count(), 0);
    }

    #[test]
    fn reopen_increments_viewport_generation() {
        let mut windows = ChatWindowRegistry::default();
        let id = windows.request(WindowKind::Diff, WindowBinding::default());
        windows.request_close(id).expect("closing");
        windows.mark_closed(id).expect("closed");
        windows.reopen(id).expect("reopen");
        assert_eq!(windows.records()[0].generation, ViewportGeneration(2));
        assert_eq!(windows.records()[0].lifecycle, WindowLifecycle::Requested);
    }

    #[test]
    fn impossible_transitions_fail_instead_of_guessing() {
        let mut windows = ChatWindowRegistry::default();
        let id = windows.request(WindowKind::Tool, WindowBinding::default());
        assert!(matches!(
            windows.mark_visible(id),
            Err(WindowError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn bound_session_survives_window_close() {
        let mut windows = ChatWindowRegistry::default();
        let session_id = ChatSessionId(7);
        let id = windows.request(
            WindowKind::Conversation,
            WindowBinding {
                provider_id: Some(ProviderId("codex")),
                session_id: Some(session_id),
                ..WindowBinding::default()
            },
        );
        windows.request_close(id).expect("closing");
        windows.mark_closed(id).expect("closed");
        assert_eq!(windows.records()[0].binding.session_id, Some(session_id));
    }
}
