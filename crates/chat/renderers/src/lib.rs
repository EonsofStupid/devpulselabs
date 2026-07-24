//! Registry-driven normalized Chat event rendering.

use dpl_chat_core::{ChatEventContent, ChatEventKind};
use dpl_coldlight::{self as cl, Tone};
use egui::{Align, Layout, RichText};
use std::collections::BTreeSet;
use std::fmt;

pub const EVENT_ROW_HEIGHT: f32 = 92.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventRendererId(pub &'static str);

pub type RenderEvent = fn(&mut egui::Ui, &ChatEventContent, Tone) -> egui::Response;

#[derive(Clone, Copy)]
pub struct EventRendererRegistration {
    pub id: EventRendererId,
    pub kind: ChatEventKind,
    pub render: RenderEvent,
}

impl fmt::Debug for EventRendererRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventRendererRegistration")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererRegistryError {
    DuplicateKind(ChatEventKind),
    MissingKind(ChatEventKind),
}

impl fmt::Display for RendererRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKind(kind) => write!(f, "event kind {kind:?} has two renderers"),
            Self::MissingKind(kind) => write!(f, "event kind {kind:?} has no renderer"),
        }
    }
}

impl std::error::Error for RendererRegistryError {}

#[derive(Debug, Clone)]
pub struct EventRendererRegistry {
    renderers: Vec<EventRendererRegistration>,
}

impl EventRendererRegistry {
    pub fn try_new(
        registrations: impl IntoIterator<Item = EventRendererRegistration>,
    ) -> Result<Self, RendererRegistryError> {
        let mut kinds = BTreeSet::new();
        let mut renderers = Vec::new();
        for registration in registrations {
            if !kinds.insert(registration.kind) {
                return Err(RendererRegistryError::DuplicateKind(registration.kind));
            }
            renderers.push(registration);
        }
        Ok(Self { renderers })
    }

    pub fn built_in() -> Self {
        Self::try_new(BUILT_IN_RENDERERS.iter().copied())
            .expect("built-in Chat renderers must remain unique")
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        content: &ChatEventContent,
        provider_tone: Tone,
    ) -> Result<egui::Response, RendererRegistryError> {
        let kind = content.kind();
        let renderer = self
            .renderers
            .iter()
            .find(|renderer| renderer.kind == kind)
            .ok_or(RendererRegistryError::MissingKind(kind))?;
        Ok((renderer.render)(ui, content, provider_tone))
    }

    pub fn len(&self) -> usize {
        self.renderers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.renderers.is_empty()
    }
}

impl Default for EventRendererRegistry {
    fn default() -> Self {
        Self::built_in()
    }
}

const BUILT_IN_RENDERERS: &[EventRendererRegistration] = &[
    registration("session", ChatEventKind::SessionStarted, render_session),
    registration("user", ChatEventKind::UserMessage, render_message),
    registration("assistant", ChatEventKind::AssistantMessage, render_message),
    registration("plan", ChatEventKind::PlanUpdated, render_detail),
    registration("status", ChatEventKind::StatusUpdated, render_detail),
    registration("tool-start", ChatEventKind::ToolStarted, render_tool),
    registration("tool-complete", ChatEventKind::ToolCompleted, render_tool),
    registration("diff", ChatEventKind::DiffUpdated, render_detail),
    registration(
        "approval-request",
        ChatEventKind::ApprovalRequested,
        render_approval,
    ),
    registration(
        "approval-resolved",
        ChatEventKind::ApprovalResolved,
        render_approval,
    ),
    registration("usage", ChatEventKind::UsageUpdated, render_detail),
    registration("turn-complete", ChatEventKind::TurnCompleted, render_turn),
    registration(
        "turn-interrupt",
        ChatEventKind::TurnInterrupted,
        render_turn,
    ),
    registration("turn-fail", ChatEventKind::TurnFailed, render_turn),
    registration(
        "provider-warning",
        ChatEventKind::ProviderWarning,
        render_warning,
    ),
    registration("unknown", ChatEventKind::Unknown, render_unknown),
];

const fn registration(
    id: &'static str,
    kind: ChatEventKind,
    render: RenderEvent,
) -> EventRendererRegistration {
    EventRendererRegistration {
        id: EventRendererId(id),
        kind,
        render,
    }
}

pub fn semantic_label(content: &ChatEventContent) -> String {
    match content {
        ChatEventContent::SessionStarted => "Session started".into(),
        ChatEventContent::UserMessage(text) => format!("You: {}", preview(text, 160)),
        ChatEventContent::AssistantMessage(text) => format!("Agent: {}", preview(text, 160)),
        ChatEventContent::PlanUpdated(text) => format!("Plan: {}", preview(text, 160)),
        ChatEventContent::StatusUpdated(text) => format!("Status: {}", preview(text, 160)),
        ChatEventContent::ToolStarted { label } => format!("Tool started: {label}"),
        ChatEventContent::ToolCompleted { label, succeeded } => format!(
            "Tool {}: {label}",
            if *succeeded { "completed" } else { "failed" }
        ),
        ChatEventContent::DiffUpdated(text) => format!("Diff: {}", preview(text, 160)),
        ChatEventContent::ApprovalRequested { summary } => {
            format!("Approval requested: {}", preview(summary, 160))
        }
        ChatEventContent::ApprovalResolved { accepted } => format!(
            "Approval {}",
            if *accepted { "accepted" } else { "declined" }
        ),
        ChatEventContent::UsageUpdated { summary } => format!("Usage: {}", preview(summary, 160)),
        ChatEventContent::TurnCompleted => "Turn completed".into(),
        ChatEventContent::TurnInterrupted => "Turn interrupted".into(),
        ChatEventContent::TurnFailed { message } => {
            format!("Turn failed: {}", preview(message, 160))
        }
        ChatEventContent::ProviderWarning(message) => {
            format!("Provider warning: {}", preview(message, 160))
        }
        ChatEventContent::Unknown(payload) => {
            format!(
                "Unknown {}: {}",
                payload.schema,
                preview(&payload.redacted_summary, 160)
            )
        }
    }
}

fn render_session(
    ui: &mut egui::Ui,
    _content: &ChatEventContent,
    provider_tone: Tone,
) -> egui::Response {
    card(ui, "SESSION", "Session started", provider_tone)
}

fn render_message(
    ui: &mut egui::Ui,
    content: &ChatEventContent,
    provider_tone: Tone,
) -> egui::Response {
    match content {
        ChatEventContent::UserMessage(text) => card(ui, "YOU", text, Tone::Violet),
        ChatEventContent::AssistantMessage(text) => card(ui, "AGENT", text, provider_tone),
        _ => render_mismatch(ui, content),
    }
}

fn render_detail(
    ui: &mut egui::Ui,
    content: &ChatEventContent,
    _provider_tone: Tone,
) -> egui::Response {
    match content {
        ChatEventContent::PlanUpdated(text) => card(ui, "PLAN", text, Tone::Amber),
        ChatEventContent::StatusUpdated(text) => card(ui, "STATUS", text, Tone::Cyan),
        ChatEventContent::DiffUpdated(text) => card(ui, "DIFF", text, Tone::Cyan),
        ChatEventContent::UsageUpdated { summary } => card(ui, "USAGE", summary, Tone::Violet),
        _ => render_mismatch(ui, content),
    }
}

fn render_tool(
    ui: &mut egui::Ui,
    content: &ChatEventContent,
    _provider_tone: Tone,
) -> egui::Response {
    match content {
        ChatEventContent::ToolStarted { label } => {
            card(ui, "TOOL", &format!("{label} started"), Tone::Magenta)
        }
        ChatEventContent::ToolCompleted { label, succeeded } => card(
            ui,
            "TOOL",
            &format!(
                "{label} {}",
                if *succeeded { "completed" } else { "failed" }
            ),
            if *succeeded { Tone::Lime } else { Tone::Coral },
        ),
        _ => render_mismatch(ui, content),
    }
}

fn render_approval(
    ui: &mut egui::Ui,
    content: &ChatEventContent,
    _provider_tone: Tone,
) -> egui::Response {
    match content {
        ChatEventContent::ApprovalRequested { summary } => {
            card(ui, "APPROVAL", summary, Tone::Amber)
        }
        ChatEventContent::ApprovalResolved { accepted } => card(
            ui,
            "APPROVAL",
            if *accepted { "Accepted" } else { "Declined" },
            if *accepted { Tone::Lime } else { Tone::Coral },
        ),
        _ => render_mismatch(ui, content),
    }
}

fn render_turn(
    ui: &mut egui::Ui,
    content: &ChatEventContent,
    _provider_tone: Tone,
) -> egui::Response {
    match content {
        ChatEventContent::TurnCompleted => card(ui, "TURN", "Completed", Tone::Lime),
        ChatEventContent::TurnInterrupted => card(ui, "TURN", "Interrupted", Tone::Amber),
        ChatEventContent::TurnFailed { message } => card(ui, "TURN", message, Tone::Coral),
        _ => render_mismatch(ui, content),
    }
}

fn render_warning(
    ui: &mut egui::Ui,
    content: &ChatEventContent,
    _provider_tone: Tone,
) -> egui::Response {
    match content {
        ChatEventContent::ProviderWarning(message) => card(ui, "WARNING", message, Tone::Amber),
        _ => render_mismatch(ui, content),
    }
}

fn render_unknown(
    ui: &mut egui::Ui,
    content: &ChatEventContent,
    _provider_tone: Tone,
) -> egui::Response {
    match content {
        ChatEventContent::Unknown(payload) => {
            let suffix = if payload.truncated {
                " · TRUNCATED"
            } else {
                ""
            };
            card(
                ui,
                "UNKNOWN",
                &format!("{}{suffix}: {}", payload.schema, payload.redacted_summary),
                Tone::Magenta,
            )
        }
        _ => render_mismatch(ui, content),
    }
}

fn render_mismatch(ui: &mut egui::Ui, content: &ChatEventContent) -> egui::Response {
    card(
        ui,
        "RENDERER",
        &format!("Renderer mismatch for {:?}", content.kind()),
        Tone::Coral,
    )
}

fn card(ui: &mut egui::Ui, label: &str, body: &str, accent: Tone) -> egui::Response {
    let preview = preview(body, 320);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), EVENT_ROW_HEIGHT - 6.0),
        Layout::top_down(Align::Min),
        |ui| {
            cl::scatter_frame(accent).show(ui, |ui| {
                ui.set_min_height(EVENT_ROW_HEIGHT - 30.0);
                ui.label(
                    RichText::new(label)
                        .small()
                        .monospace()
                        .color(accent.color()),
                );
                ui.add(egui::Label::new(RichText::new(preview).color(cl::ink::BASE)).wrap());
            });
        },
    )
    .response
}

fn preview(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let text = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{text}…")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::BoundedNativePayload;

    #[test]
    fn built_in_registry_covers_every_kind_once() {
        let registry = EventRendererRegistry::built_in();
        assert_eq!(registry.len(), 16);
        for content in [
            ChatEventContent::SessionStarted,
            ChatEventContent::UserMessage("u".into()),
            ChatEventContent::AssistantMessage("a".into()),
            ChatEventContent::PlanUpdated("p".into()),
            ChatEventContent::StatusUpdated("s".into()),
            ChatEventContent::ToolStarted { label: "t".into() },
            ChatEventContent::ToolCompleted {
                label: "t".into(),
                succeeded: true,
            },
            ChatEventContent::DiffUpdated("d".into()),
            ChatEventContent::ApprovalRequested {
                summary: "a".into(),
            },
            ChatEventContent::ApprovalResolved { accepted: true },
            ChatEventContent::UsageUpdated {
                summary: "u".into(),
            },
            ChatEventContent::TurnCompleted,
            ChatEventContent::TurnInterrupted,
            ChatEventContent::TurnFailed {
                message: "f".into(),
            },
            ChatEventContent::ProviderWarning("w".into()),
            ChatEventContent::Unknown(BoundedNativePayload::redacted("x", "y")),
        ] {
            assert!(
                registry
                    .renderers
                    .iter()
                    .any(|renderer| renderer.kind == content.kind()),
                "missing {:?}",
                content.kind()
            );
        }
    }

    #[test]
    fn duplicate_event_kinds_fail_immediately() {
        let registration = BUILT_IN_RENDERERS[0];
        assert!(matches!(
            EventRendererRegistry::try_new([registration, registration]),
            Err(RendererRegistryError::DuplicateKind(
                ChatEventKind::SessionStarted
            ))
        ));
    }
}
