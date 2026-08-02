//! The STATUS zone as a data-driven indicator grid — the operator's
//! "well established grids so i can evolve... cssgrid for all of my status
//! indicators", as a registry (Law 4): the bar is three lane columns, a cell
//! is one `StatusCell` in [`CELLS`], and adding an indicator is adding ONE
//! entry there. Cells paint; the shell mutates — a cell answers with a
//! [`StatusAction`] and the owner executes it (the PaletteActions pattern).

use dpl_coldlight::{self as cl, Mood, Tone};
use dpl_fabric::AppWindowKey;
use egui::{Align, Layout, Rect, RichText, Ui, UiBuilder};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, UiPilotExt};

/// The three columns of the bar. Left and Right pack from their outer edge;
/// Center packs from the middle column's center.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    Left,
    Center,
    Right,
}

/// What a clicked cell asks the shell to do. Cells never reach into the
/// shell — they answer, the shell acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusAction {
    /// Summon the ViziWiz overlay (the Snip launcher).
    Snip,
    /// Ask the app owner to open, restore, and focus a native window.
    Window(AppWindowKey),
}

/// Data-only description of an app-owned window accessory.
#[derive(Debug, Clone, Copy)]
pub struct WindowAccessory {
    pub key: AppWindowKey,
    pub pilot_id: PilotId,
    pub glyph: &'static str,
    pub label: &'static str,
    pub tone: Tone,
    pub tooltip: &'static str,
}

/// Everything a cell may read this frame — the shell's live state, borrowed.
pub struct StatusCtx<'a> {
    pub tone: Tone,
    pub viewport: egui::Vec2,
    pub ppp: f32,
    pub navs: u64,
    pub signals: u64,
    pub build: &'a str,
    pub mood: &'static Mood,
    /// The JobBoard's "working…" truth — None = nothing running.
    pub working: Option<String>,
    /// THE ROSTER's truth — None = no provider is activated, and the control
    /// is absent rather than claiming an empty capability.
    pub providers: Option<ProviderChip>,
    /// The session's seeded ink-bleed: which pair of hues this launch drew
    /// and how long its dwell runs. Rolled once by the shell, never here — a
    /// cell paints, it does not decide.
    pub bleed: crate::providers::BleedRoll,
}

/// What the platform can do, ready to paint. Built by the shell from the
/// fabric's roster so the cell stays a painter (Law 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderChip {
    /// `Codex, Claude Code` — the activated providers, named.
    pub summary: String,
    /// Each provider's truthful limit, one per line.
    pub tooltip: String,
}

/// One indicator: identity, which lane column it lives in, and its painter.
pub struct StatusCell {
    pub id: &'static str,
    pub lane: Lane,
    pub render: fn(&mut Ui, &StatusCtx) -> Option<StatusAction>,
}

/// THE registry. Order within a lane is packing order from that lane's edge.
pub static CELLS: &[StatusCell] = &[
    StatusCell {
        id: "brand",
        lane: Lane::Left,
        render: brand,
    },
    StatusCell {
        id: "snip",
        lane: Lane::Left,
        render: snip,
    },
    StatusCell {
        id: "viewport",
        lane: Lane::Center,
        render: viewport,
    },
    StatusCell {
        id: "mood",
        lane: Lane::Center,
        render: mood,
    },
    StatusCell {
        id: "signals",
        lane: Lane::Right,
        render: signals,
    },
    StatusCell {
        id: "jobs",
        lane: Lane::Right,
        render: jobs,
    },
    StatusCell {
        id: "providers",
        lane: Lane::Right,
        render: providers,
    },
    StatusCell {
        id: "build",
        lane: Lane::Right,
        render: build,
    },
];

/// Lay the bar as three lane columns and run every registered cell.
pub fn grid(
    ui: &mut Ui,
    cx: &StatusCtx,
    window_accessories: &[WindowAccessory],
) -> Vec<StatusAction> {
    let bar = ui.max_rect();
    let third = bar.width() / 3.0;
    let lane_rect = |i: f32| {
        Rect::from_min_max(
            egui::pos2(bar.min.x + third * i, bar.min.y),
            egui::pos2(bar.min.x + third * (i + 1.0), bar.max.y),
        )
    };
    let mut actions = Vec::new();
    for (lane, rect) in [
        (Lane::Left, lane_rect(0.0)),
        (Lane::Center, lane_rect(1.0)),
        (Lane::Right, lane_rect(2.0)),
    ] {
        let layout = match lane {
            Lane::Right => Layout::right_to_left(Align::Center),
            Lane::Center => Layout::left_to_right(Align::Center).with_main_align(Align::Center),
            Lane::Left => Layout::left_to_right(Align::Center),
        };
        let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(layout));
        if lane == Lane::Right {
            for accessory in window_accessories {
                let response = crate::launcher_chip(
                    &mut child,
                    accessory.glyph,
                    accessory.label,
                    accessory.tone,
                    accessory.tooltip,
                );
                child.pilot_track(
                    &response,
                    NodeMetadata::new(
                        accessory.pilot_id,
                        Role::Button,
                        format!("Open {}", accessory.label),
                    ),
                );
                if response.clicked() {
                    actions.push(StatusAction::Window(accessory.key));
                }
                child.add_space(cl::space::S3);
            }
        }
        for cell in CELLS.iter().filter(|c| c.lane == lane) {
            if let Some(action) = (cell.render)(&mut child, cx) {
                actions.push(action);
            }
            child.add_space(cl::space::S3);
        }
    }
    actions
}

// ── the cells ───────────────────────────────────────────────────────────

fn brand(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    cl::tone_dot(ui, cx.tone);
    ui.label(RichText::new("ColdLight").small().color(cl::ink::MUTED));
    None
}

fn snip(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    crate::launcher_chip(
        ui,
        cl::icon::SNIP,
        "Snip",
        cx.tone,
        "Open the capture palette (Ctrl+Shift+S from anywhere)",
    )
    .clicked()
    .then_some(StatusAction::Snip)
}

fn viewport(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    ui.label(
        RichText::new(format!(
            "{:.0} × {:.0} · dpr {:.2}",
            cx.viewport.x, cx.viewport.y, cx.ppp
        ))
        .monospace()
        .size(11.5)
        .color(cl::ink::SUBTLE),
    );
    None
}

fn mood(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 3.0, cx.mood.accent_color());
    ui.label(RichText::new(cx.mood.name).small().color(cl::ink::MUTED));
    None
}

fn signals(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    ui.label(
        RichText::new(format!("signals · {}", cx.signals))
            .small()
            .color(cl::ink::SUBTLE),
    );
    None
}

/// The JobBoard's honest "working…" truth — a render, ingest, or export in
/// flight is visible from EVERY route, not only the surface that started it.
fn jobs(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    if let Some(text) = &cx.working {
        cl::tone_dot(ui, cx.tone);
        ui.label(RichText::new(text).small().color(cl::ink::BASE));
        // Progress counters tick without a signal — keep the bar live.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(300));
    }
    None
}

/// The PROVIDERS control (V1) — brain + "Providers" + the `ai` circle, the
/// ink bleed running on hover over the seeded dwell.
///
/// It answers the operator's question — "the providers that get activated in
/// the chat become available in the platform" — on a surface that is not
/// Chat. Absent entirely when nothing is activated: silence, never an empty
/// claim. Read-only; the click opens V2's popup and does nothing yet.
fn providers(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    if let Some(chip) = &cx.providers {
        let response = crate::providers::ProvidersControl {
            names: &chip.summary,
            tooltip: &chip.tooltip,
            roll: cx.bleed,
        }
        .show(ui);
        ui.pilot_track(
            &response,
            // The name is the control AND what it stands for: "Providers"
            // is painted, the roster is what it means. An agent reading the
            // tree learns both, and a scenario can assert on either.
            NodeMetadata::new(
                PROVIDERS_PILOT_ID,
                Role::Button,
                format!("Providers — {}", chip.summary),
            )
            .value(chip.summary.clone()),
        );
    }
    None
}

/// The pilot identity the `platform-sees-providers` scenario asserts on.
pub const PROVIDERS_PILOT_ID: PilotId = PilotId::new("shell.status.providers");

fn build(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    if !cx.build.is_empty() {
        ui.label(
            RichText::new(cx.build)
                .monospace()
                .size(11.0)
                .color(cl::ink::SUBTLE),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_ids_are_unique() {
        for (i, a) in CELLS.iter().enumerate() {
            for b in &CELLS[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate status cell id");
            }
        }
    }

    #[test]
    fn every_lane_carries_at_least_one_cell() {
        for lane in [Lane::Left, Lane::Center, Lane::Right] {
            assert!(
                CELLS.iter().any(|c| c.lane == lane),
                "lane {lane:?} is empty"
            );
        }
    }
}
