//! Stable lifecycle state for attached Chat windows.
//!
//! Native viewport creation belongs to the shell coordinator in CH8. This
//! crate defines ownership and transitions without depending on egui.

use dpl_chat_core::{ChatSessionId, ProviderId, TurnId};
use dpl_chat_interactions::InteractionId;
use dpl_hub::Route;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatWindowId(pub u64);

impl ChatWindowId {
    /// Stable singleton identity of the primary peer Chat window.
    pub const PRIMARY: Self = Self(0);
}

impl fmt::Display for ChatWindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chat-window-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowGeometry {
    pub outer_position: Option<[i32; 2]>,
    pub inner_size: [u32; 2],
}

impl WindowGeometry {
    pub const DEFAULT: Self = Self {
        outer_position: Some([32, 32]),
        inner_size: [1040, 640],
    };
    pub const UTILITY_DEFAULT: Self = Self {
        outer_position: None,
        inner_size: [360, 600],
    };
    pub const MINIMUM: [u32; 2] = [720, 560];
    pub const UTILITY_MINIMUM: [u32; 2] = [320, 360];

    pub fn validated_for(
        primary: bool,
        outer_position: Option<[i32; 2]>,
        inner_size: [u32; 2],
    ) -> Option<Self> {
        let minimum = if primary {
            Self::MINIMUM
        } else {
            Self::UTILITY_MINIMUM
        };
        (inner_size[0] > 0 && inner_size[1] > 0).then_some(Self {
            outer_position,
            inner_size: [inner_size[0].max(minimum[0]), inner_size[1].max(minimum[1])],
        })
    }
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self::DEFAULT
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapEdge {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAttachment {
    Snapped {
        parent: ChatWindowId,
        edge: SnapEdge,
        offset: [i32; 2],
    },
    Detached,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowBinding {
    pub provider_id: Option<ProviderId>,
    pub session_id: Option<ChatSessionId>,
    pub turn_id: Option<TurnId>,
    pub route: Option<Route>,
    pub interaction_id: Option<InteractionId>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCloseDecision {
    HideMain,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatCloseDecision {
    KeepRunning,
    Exit,
}

/// Pure lifetime policy for the two independent application peers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerLifetime {
    main_visible: bool,
    quit_requested: bool,
}

impl Default for PeerLifetime {
    fn default() -> Self {
        Self {
            main_visible: true,
            quit_requested: false,
        }
    }
}

impl PeerLifetime {
    pub const fn main_visible(self) -> bool {
        self.main_visible
    }

    pub fn restore_main(&mut self) {
        self.main_visible = true;
    }

    pub fn request_quit(&mut self) {
        self.quit_requested = true;
    }

    pub const fn quit_requested(self) -> bool {
        self.quit_requested
    }

    pub fn on_root_close(&mut self, chat_live: bool) -> RootCloseDecision {
        if self.quit_requested || !chat_live {
            RootCloseDecision::Exit
        } else {
            self.main_visible = false;
            RootCloseDecision::HideMain
        }
    }

    pub const fn on_chat_close(self) -> ChatCloseDecision {
        if self.main_visible {
            ChatCloseDecision::KeepRunning
        } else {
            ChatCloseDecision::Exit
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatWindowRecord {
    pub id: ChatWindowId,
    pub generation: ViewportGeneration,
    pub kind: WindowKind,
    pub binding: WindowBinding,
    pub lifecycle: WindowLifecycle,
    pub geometry: WindowGeometry,
    pub attachment: Option<WindowAttachment>,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    Unknown(ChatWindowId),
    InvalidTransition {
        id: ChatWindowId,
        from: WindowLifecycle,
        to: &'static str,
    },
    PrimaryAttachment,
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(id) => write!(f, "{id} is not registered"),
            Self::InvalidTransition { id, from, to } => {
                write!(f, "{id} cannot transition from {from:?} to {to}")
            }
            Self::PrimaryAttachment => f.write_str("the primary Chat peer cannot be attached"),
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
    /// Register the one primary Chat peer. Repeated calls return the same
    /// identity and never create a duplicate viewport.
    pub fn request_primary(&mut self, binding: WindowBinding) -> ChatWindowId {
        if self.window(ChatWindowId::PRIMARY).is_some() {
            return ChatWindowId::PRIMARY;
        }
        self.windows.push(ChatWindowRecord {
            id: ChatWindowId::PRIMARY,
            generation: ViewportGeneration(1),
            kind: WindowKind::Conversation,
            binding,
            lifecycle: WindowLifecycle::Requested,
            geometry: WindowGeometry::default(),
            attachment: None,
            pinned: false,
        });
        ChatWindowId::PRIMARY
    }

    pub fn request(&mut self, kind: WindowKind, binding: WindowBinding) -> ChatWindowId {
        self.request_attached(kind, binding, WindowAttachment::Detached)
            .expect("utility requests always use a valid non-primary attachment")
    }

    pub fn request_attached(
        &mut self,
        kind: WindowKind,
        binding: WindowBinding,
        attachment: WindowAttachment,
    ) -> Result<ChatWindowId, WindowError> {
        self.next_id = self.next_id.saturating_add(1);
        let id = ChatWindowId(self.next_id);
        self.windows.push(ChatWindowRecord {
            id,
            generation: ViewportGeneration(1),
            kind,
            binding,
            lifecycle: WindowLifecycle::Requested,
            geometry: WindowGeometry::UTILITY_DEFAULT,
            attachment: Some(attachment),
            pinned: false,
        });
        Ok(id)
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

    pub fn record(&self, id: ChatWindowId) -> Option<&ChatWindowRecord> {
        self.window(id)
    }

    pub fn update_geometry(
        &mut self,
        id: ChatWindowId,
        outer_position: Option<[i32; 2]>,
        inner_size: [u32; 2],
    ) -> Result<bool, WindowError> {
        let window = self.window_mut(id)?;
        let Some(geometry) = WindowGeometry::validated_for(
            window.id == ChatWindowId::PRIMARY,
            outer_position,
            inner_size,
        ) else {
            return Ok(false);
        };
        let changed = window.geometry != geometry;
        window.geometry = geometry;
        Ok(changed)
    }

    pub fn set_attachment(
        &mut self,
        id: ChatWindowId,
        attachment: WindowAttachment,
    ) -> Result<bool, WindowError> {
        let window = self.window_mut(id)?;
        if window.id == ChatWindowId::PRIMARY {
            return Err(WindowError::PrimaryAttachment);
        }
        let changed = window.attachment != Some(attachment);
        if changed {
            window.attachment = Some(attachment);
            window.generation = ViewportGeneration(window.generation.0.saturating_add(1));
            window.lifecycle = WindowLifecycle::Requested;
        }
        Ok(changed)
    }

    pub fn toggle_pinned(&mut self, id: ChatWindowId) -> Result<bool, WindowError> {
        let window = self.window_mut(id)?;
        if window.id == ChatWindowId::PRIMARY {
            return Err(WindowError::PrimaryAttachment);
        }
        window.pinned = !window.pinned;
        Ok(window.pinned)
    }

    pub fn is_live(&self, id: ChatWindowId) -> bool {
        self.window(id).is_some_and(|window| {
            matches!(
                window.lifecycle,
                WindowLifecycle::Requested | WindowLifecycle::Opening | WindowLifecycle::Visible
            )
        })
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

    fn window(&self, id: ChatWindowId) -> Option<&ChatWindowRecord> {
        self.windows.iter().find(|window| window.id == id)
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

    #[test]
    fn primary_is_a_singleton_with_a_stable_identity() {
        let mut windows = ChatWindowRegistry::default();
        assert_eq!(
            windows.request_primary(WindowBinding::default()),
            ChatWindowId::PRIMARY
        );
        assert_eq!(
            windows.request_primary(WindowBinding::default()),
            ChatWindowId::PRIMARY
        );
        assert_eq!(windows.records().len(), 1);
    }

    #[test]
    fn geometry_is_clamped_and_survives_reopen() {
        let mut windows = ChatWindowRegistry::default();
        let id = windows.request_primary(WindowBinding::default());
        assert!(windows
            .update_geometry(id, Some([40, 60]), [240, 320])
            .expect("primary"));
        assert_eq!(
            windows.record(id).expect("primary").geometry,
            WindowGeometry {
                outer_position: Some([40, 60]),
                inner_size: WindowGeometry::MINIMUM,
            }
        );
        windows.request_close(id).expect("closing");
        windows.mark_closed(id).expect("closed");
        windows.reopen(id).expect("reopen");
        assert_eq!(
            windows.record(id).expect("primary").geometry.inner_size,
            WindowGeometry::MINIMUM
        );
    }

    #[test]
    fn invalid_zero_geometry_is_rejected_without_erasing_the_last_value() {
        let mut windows = ChatWindowRegistry::default();
        let id = windows.request_primary(WindowBinding::default());
        assert!(!windows
            .update_geometry(id, None, [0, 720])
            .expect("primary"));
        assert_eq!(
            windows.record(id).expect("primary").geometry,
            WindowGeometry::DEFAULT
        );
    }

    #[test]
    fn utility_snap_detach_and_redock_preserve_binding_and_identity() {
        let mut windows = ChatWindowRegistry::default();
        let session_id = ChatSessionId(9);
        let snapped = WindowAttachment::Snapped {
            parent: ChatWindowId::PRIMARY,
            edge: SnapEdge::Right,
            offset: [12, 0],
        };
        let id = windows
            .request_attached(
                WindowKind::Artifact,
                WindowBinding {
                    provider_id: Some(ProviderId("codex")),
                    session_id: Some(session_id),
                    ..WindowBinding::default()
                },
                snapped,
            )
            .expect("snapped utility");
        assert_eq!(
            windows.record(id).expect("utility").attachment,
            Some(snapped)
        );

        assert!(windows
            .set_attachment(id, WindowAttachment::Detached)
            .expect("detach"));
        let detached_generation = windows.record(id).expect("detached").generation;
        assert_eq!(detached_generation, ViewportGeneration(2));
        assert_eq!(
            windows.record(id).expect("detached").binding.session_id,
            Some(session_id)
        );

        assert!(windows.set_attachment(id, snapped).expect("redock"));
        let redocked = windows.record(id).expect("redocked");
        assert_eq!(redocked.id, id);
        assert_eq!(redocked.generation, ViewportGeneration(3));
        assert_eq!(redocked.attachment, Some(snapped));
        assert_eq!(redocked.binding.session_id, Some(session_id));
    }

    #[test]
    fn utility_geometry_uses_its_own_compact_minimum() {
        let mut windows = ChatWindowRegistry::default();
        let id = windows.request(WindowKind::Artifact, WindowBinding::default());
        assert!(windows
            .update_geometry(id, Some([1048, 24]), [120, 140])
            .expect("utility"));
        assert_eq!(
            windows.record(id).expect("utility").geometry.inner_size,
            WindowGeometry::UTILITY_MINIMUM
        );
    }

    #[test]
    fn primary_window_cannot_be_attached() {
        let mut windows = ChatWindowRegistry::default();
        let id = windows.request_primary(WindowBinding::default());
        assert_eq!(
            windows.set_attachment(id, WindowAttachment::Detached),
            Err(WindowError::PrimaryAttachment)
        );
    }

    #[test]
    fn root_close_hides_main_while_chat_is_live() {
        let mut lifetime = PeerLifetime::default();
        assert_eq!(lifetime.on_root_close(true), RootCloseDecision::HideMain);
        assert!(!lifetime.main_visible());
        assert_eq!(lifetime.on_chat_close(), ChatCloseDecision::Exit);
    }

    #[test]
    fn last_root_close_exits_and_explicit_quit_always_exits() {
        let mut lifetime = PeerLifetime::default();
        assert_eq!(lifetime.on_root_close(false), RootCloseDecision::Exit);
        lifetime.request_quit();
        assert_eq!(lifetime.on_root_close(true), RootCloseDecision::Exit);
    }

    #[test]
    fn restoring_main_keeps_the_process_after_chat_closes() {
        let mut lifetime = PeerLifetime::default();
        assert_eq!(lifetime.on_root_close(true), RootCloseDecision::HideMain);
        lifetime.restore_main();
        assert_eq!(lifetime.on_chat_close(), ChatCloseDecision::KeepRunning);
    }
}
