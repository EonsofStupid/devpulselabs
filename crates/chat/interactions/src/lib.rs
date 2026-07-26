//! Governed interaction contracts shared by AI and operator-authored surfaces.
//!
//! Content is declarative. Providers cannot supply egui callbacks, geometry,
//! colors, z-order, filesystem paths, shell handles, or arbitrary actions.

use dpl_chat_core::{Capability, ChatEventId, ChatSessionId, ProviderId};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

pub const MAX_INTERACTION_TEXT_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InteractionId(pub u64);

impl fmt::Display for InteractionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "interaction-{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionProvenance {
    Host,
    Operator,
    Provider {
        provider_id: ProviderId,
        session_id: ChatSessionId,
        event_id: Option<ChatEventId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionPlacement {
    Inline,
    Overlay,
    SnappedUtility,
    DetachedUtility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionLifecycle {
    Proposed,
    Visible,
    Completed,
    Dismissed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteResultState {
    Opened,
    Unavailable,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionContent {
    Notice {
        title: String,
        body: String,
    },
    Approval {
        title: String,
        summary: String,
        approve_label: String,
        always_approve_label: Option<String>,
        reject_label: String,
    },
    RouteResult {
        route_label: String,
        detail: String,
        state: RouteResultState,
    },
    ArtifactPreview {
        title: String,
        artifact_kind: String,
        summary: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionSpec {
    pub id: InteractionId,
    pub provenance: InteractionProvenance,
    pub required_capability: Option<Capability>,
    pub lifecycle: InteractionLifecycle,
    pub placement: InteractionPlacement,
    pub content: InteractionContent,
}

impl InteractionSpec {
    pub fn validate(&self) -> Result<(), InteractionError> {
        for text in self.text_fields() {
            if text.trim().is_empty() {
                return Err(InteractionError::EmptyText(self.id));
            }
            if text.len() > MAX_INTERACTION_TEXT_BYTES {
                return Err(InteractionError::TextTooLarge {
                    id: self.id,
                    bytes: text.len(),
                });
            }
        }
        if matches!(self.content, InteractionContent::Approval { .. })
            && self.placement != InteractionPlacement::Overlay
        {
            return Err(InteractionError::ApprovalMustBeOverlay(self.id));
        }
        Ok(())
    }

    pub const fn blocks_focus(&self) -> bool {
        matches!(self.content, InteractionContent::Approval { .. })
            && matches!(
                self.lifecycle,
                InteractionLifecycle::Proposed | InteractionLifecycle::Visible
            )
    }

    fn text_fields(&self) -> Vec<&str> {
        match &self.content {
            InteractionContent::Notice { title, body } => vec![title, body],
            InteractionContent::Approval {
                title,
                summary,
                approve_label,
                always_approve_label,
                reject_label,
            } => {
                let mut fields: Vec<&str> = vec![title, summary, approve_label, reject_label];
                if let Some(always) = always_approve_label {
                    fields.push(always);
                }
                fields
            }
            InteractionContent::RouteResult {
                route_label,
                detail,
                ..
            } => vec![route_label, detail],
            InteractionContent::ArtifactPreview {
                title,
                artifact_kind,
                summary,
            } => vec![title, artifact_kind, summary],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionError {
    Duplicate(InteractionId),
    Unknown(InteractionId),
    EmptyText(InteractionId),
    TextTooLarge {
        id: InteractionId,
        bytes: usize,
    },
    ApprovalMustBeOverlay(InteractionId),
    BlockingOverlayExists(InteractionId),
    InvalidLifecycle {
        id: InteractionId,
        from: InteractionLifecycle,
        to: InteractionLifecycle,
    },
}

impl fmt::Display for InteractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(id) => write!(f, "duplicate {id}"),
            Self::Unknown(id) => write!(f, "unknown {id}"),
            Self::EmptyText(id) => write!(f, "{id} contains empty text"),
            Self::TextTooLarge { id, bytes } => {
                write!(f, "{id} contains {bytes} text bytes")
            }
            Self::ApprovalMustBeOverlay(id) => {
                write!(f, "{id} approval must use overlay placement")
            }
            Self::BlockingOverlayExists(id) => {
                write!(f, "{id} cannot open while another approval owns focus")
            }
            Self::InvalidLifecycle { id, from, to } => {
                write!(f, "{id} cannot transition from {from:?} to {to:?}")
            }
        }
    }
}

impl std::error::Error for InteractionError {}

#[derive(Debug, Clone, Default)]
pub struct InteractionRegistry {
    specs: BTreeMap<InteractionId, InteractionSpec>,
}

impl InteractionRegistry {
    pub fn register(&mut self, spec: InteractionSpec) -> Result<(), InteractionError> {
        spec.validate()?;
        if spec.blocks_focus()
            && self
                .specs
                .values()
                .any(|candidate| candidate.blocks_focus())
        {
            return Err(InteractionError::BlockingOverlayExists(spec.id));
        }
        if self.specs.contains_key(&spec.id) {
            return Err(InteractionError::Duplicate(spec.id));
        }
        self.specs.insert(spec.id, spec);
        Ok(())
    }

    pub fn get(&self, id: InteractionId) -> Option<&InteractionSpec> {
        self.specs.get(&id)
    }

    pub fn visible(&self) -> impl Iterator<Item = &InteractionSpec> {
        self.specs.values().filter(|spec| {
            matches!(
                spec.lifecycle,
                InteractionLifecycle::Proposed | InteractionLifecycle::Visible
            )
        })
    }

    pub fn transition(
        &mut self,
        id: InteractionId,
        to: InteractionLifecycle,
    ) -> Result<(), InteractionError> {
        let spec = self
            .specs
            .get_mut(&id)
            .ok_or(InteractionError::Unknown(id))?;
        let from = spec.lifecycle;
        let valid = matches!(
            (from, to),
            (
                InteractionLifecycle::Proposed,
                InteractionLifecycle::Visible
            ) | (
                InteractionLifecycle::Proposed | InteractionLifecycle::Visible,
                InteractionLifecycle::Completed
                    | InteractionLifecycle::Dismissed
                    | InteractionLifecycle::Expired
            )
        );
        if !valid {
            return Err(InteractionError::InvalidLifecycle { id, from, to });
        }
        spec.lifecycle = to;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayEntry {
    pub interaction_id: InteractionId,
    pub origin_semantic_id: &'static str,
    pub priority: u16,
    pub blocks_focus: bool,
}

#[derive(Debug, Clone, Default)]
pub struct OverlayCoordinator {
    queue: VecDeque<OverlayEntry>,
    focus_return: Option<&'static str>,
}

impl OverlayCoordinator {
    pub fn open(&mut self, entry: OverlayEntry) -> Result<(), InteractionError> {
        if entry.blocks_focus && self.queue.iter().any(|candidate| candidate.blocks_focus) {
            return Err(InteractionError::BlockingOverlayExists(
                entry.interaction_id,
            ));
        }
        let index = self
            .queue
            .iter()
            .position(|candidate| candidate.priority < entry.priority)
            .unwrap_or(self.queue.len());
        self.queue.insert(index, entry);
        Ok(())
    }

    pub fn ordered(&self) -> impl ExactSizeIterator<Item = &OverlayEntry> {
        self.queue.iter()
    }

    pub fn dismiss(&mut self, id: InteractionId) -> Option<&'static str> {
        let index = self
            .queue
            .iter()
            .position(|entry| entry.interaction_id == id)?;
        let entry = self.queue.remove(index)?;
        self.focus_return = Some(entry.origin_semantic_id);
        self.focus_return
    }

    pub const fn take_focus_return(&mut self) -> Option<&'static str> {
        self.focus_return.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(id: u64, placement: InteractionPlacement) -> InteractionSpec {
        InteractionSpec {
            id: InteractionId(id),
            provenance: InteractionProvenance::Host,
            required_capability: None,
            lifecycle: InteractionLifecycle::Proposed,
            placement,
            content: InteractionContent::Notice {
                title: "Route ready".into(),
                body: "Preview before opening in Main.".into(),
            },
        }
    }

    fn approval(id: u64) -> InteractionSpec {
        InteractionSpec {
            id: InteractionId(id),
            provenance: InteractionProvenance::Operator,
            required_capability: Some(Capability::Approvals),
            lifecycle: InteractionLifecycle::Proposed,
            placement: InteractionPlacement::Overlay,
            content: InteractionContent::Approval {
                title: "Open route?".into(),
                summary: "Restore Main and open Library / Templates.".into(),
                approve_label: "Open in Main".into(),
                always_approve_label: None,
                reject_label: "Keep here".into(),
            },
        }
    }

    #[test]
    fn validation_rejects_provider_layout_by_limiting_the_vocabulary() {
        let spec = notice(1, InteractionPlacement::Inline);
        spec.validate().expect("known host vocabulary");
        assert!(matches!(spec.content, InteractionContent::Notice { .. }));
    }

    #[test]
    fn approval_is_overlay_only_and_single_owner() {
        let mut registry = InteractionRegistry::default();
        registry.register(approval(1)).expect("first approval");
        assert!(matches!(
            registry.register(approval(2)),
            Err(InteractionError::BlockingOverlayExists(_))
        ));
        let mut invalid = approval(3);
        invalid.placement = InteractionPlacement::Inline;
        assert!(matches!(
            invalid.validate(),
            Err(InteractionError::ApprovalMustBeOverlay(_))
        ));
    }

    #[test]
    fn overlay_ordering_and_focus_return_are_deterministic() {
        let mut overlays = OverlayCoordinator::default();
        overlays
            .open(OverlayEntry {
                interaction_id: InteractionId(1),
                origin_semantic_id: "chat.provider.codex",
                priority: 10,
                blocks_focus: false,
            })
            .expect("hover");
        overlays
            .open(OverlayEntry {
                interaction_id: InteractionId(2),
                origin_semantic_id: "chat.route.open",
                priority: 100,
                blocks_focus: true,
            })
            .expect("approval");
        assert_eq!(
            overlays
                .ordered()
                .map(|entry| entry.interaction_id)
                .collect::<Vec<_>>(),
            vec![InteractionId(2), InteractionId(1)]
        );
        assert_eq!(overlays.dismiss(InteractionId(2)), Some("chat.route.open"));
        assert_eq!(overlays.take_focus_return(), Some("chat.route.open"));
        assert_eq!(overlays.take_focus_return(), None);
    }
}
