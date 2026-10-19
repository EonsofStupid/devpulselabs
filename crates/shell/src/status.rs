//! The STATUS zone as a data-driven indicator grid — the operator's
//! "well established grids so i can evolve... cssgrid for all of my status
//! indicators", as a registry (Law 4): the bar is three lanes, a cell
//! is one `StatusCell` in [`CELLS`], and adding an indicator is adding ONE
//! entry there. Cells paint; the shell mutates — a cell answers with a
//! [`StatusAction`] and the owner executes it (the PaletteActions pattern).

use dpl_coldlight::{self as cl, Mood, Tone};
use dpl_fabric::AppWindowKey;
use egui::{Align, Layout, Rect, RichText, Ui, UiBuilder};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, UiPilotExt};

/// The three lanes of the bar. Left and Right pack from their outer edge and
/// take what their cells need; Center is centered in the room between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Toggle the Providers roster popup (V2).
    ToggleProviders,
    /// Toggle the ColdLight preset picker — how THIS screen is dressed.
    TogglePresets,
    /// Toggle the job board: every row the fabric holds, finished ones clearable.
    ToggleJobs,
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
    /// The surface the operator is looking at (`route.hub`), so the preset
    /// picker dresses THIS screen and not the whole app. A cell paints; it
    /// does not decide which surface it is on (Law 4).
    pub surface: &'static str,
    pub viewport: egui::Vec2,
    pub ppp: f32,
    pub navs: u64,
    pub signals: u64,
    pub build: &'a str,
    pub mood: &'static Mood,
    /// The JobBoard's "working…" truth — None = nothing running.
    pub working: Option<String>,
    /// Resolved board rows nobody has acknowledged yet — the quiet reminder
    /// that outcomes are waiting on the board after the work stops.
    pub jobs_unacked: usize,
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

/// Lay out the bar and run every registered cell.
///
/// The outer lanes pack from their edges and take what their cells need,
/// the right lane first because it carries the most. The center lane gets
/// only the room left between them, centered there. When its cells do not
/// fit that room (a narrow window, a busy right lane) it steps aside for the
/// frame instead of painting over its neighbours. Whether it fits is judged
/// by the width it used the last time it was shown (egui's two-pass trick).
pub fn grid(
    ui: &mut Ui,
    cx: &StatusCtx,
    window_accessories: &[WindowAccessory],
) -> Vec<StatusAction> {
    let bar = ui.max_rect();
    let mut actions = Vec::new();
    let right = lane(ui, bar, Lane::Right, cx, window_accessories, &mut actions);
    let left_room = Rect::from_min_max(bar.min, egui::pos2(right.min.x, bar.max.y));
    let left = lane(ui, left_room, Lane::Left, cx, &[], &mut actions);
    let middle = Rect::from_min_max(
        egui::pos2(left.max.x + cl::space::S3, bar.min.y),
        egui::pos2(right.min.x - cl::space::S3, bar.max.y),
    );
    let needed_id = ui.id().with("dpl_status_center_needed");
    let needed: f32 = ui.ctx().data(|d| d.get_temp(needed_id)).unwrap_or(0.0);
    if middle.width() >= needed {
        let used = lane(ui, middle, Lane::Center, cx, &[], &mut actions);
        ui.ctx()
            .data_mut(|d| d.insert_temp(needed_id, used.width()));
    }
    actions
}

/// Run one lane's cells inside `room`, packed the lane's way, and return the
/// rect they used. Window accessories ride at the head of the lane passed
/// them (the right lane).
fn lane(
    ui: &mut Ui,
    room: Rect,
    lane: Lane,
    cx: &StatusCtx,
    window_accessories: &[WindowAccessory],
    actions: &mut Vec<StatusAction>,
) -> Rect {
    let layout = match lane {
        Lane::Right => Layout::right_to_left(Align::Center),
        Lane::Center => Layout::left_to_right(Align::Center).with_main_align(Align::Center),
        Lane::Left => Layout::left_to_right(Align::Center),
    };
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt(("dpl_status_lane", lane))
            .max_rect(room)
            .layout(layout),
    );
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
    for cell in CELLS.iter().filter(|c| c.lane == lane) {
        if let Some(action) = (cell.render)(&mut child, cx) {
            actions.push(action);
        }
        child.add_space(cl::space::S3);
    }
    child.min_rect()
}

// ── the cells ───────────────────────────────────────────────────────────

/// The system's own name, and the way into how this screen is dressed.
///
/// It reads "ColdLight" until a preset is picked, then wears that preset's
/// label — so the bar always answers "what am I looking at" without a second
/// indicator.
fn brand(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    cl::tone_dot(ui, cx.tone);
    let worn = cl::chrome_preset(cx.surface);
    let label = worn.map_or("ColdLight", |p| p.label);
    let response = cl::tip(
        ui.add(
            egui::Label::new(RichText::new(label).small().color(cl::ink::MUTED))
                .sense(egui::Sense::click()),
        ),
        "Change how this screen is dressed — every look is a legal point on the ColdLight matrix",
    );
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            PilotId::new("shell.status").child(&egui_pilot_ai::PilotKey::new("presets")),
            Role::Button,
            "Open the ColdLight preset picker",
        )
        .value(worn.map_or("authored", |p| p.id)),
    );
    response.clicked().then_some(StatusAction::TogglePresets)
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
/// flight is visible from EVERY route, not only the surface that started it —
/// and the way INTO the board: clicking the cell opens the job board. Quiet
/// when the board is empty; when finished jobs wait to be cleared it says so
/// instead of going silent.
fn jobs(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    let text = match (&cx.working, cx.jobs_unacked) {
        (Some(text), _) => {
            cl::tone_dot(ui, cx.tone);
            // Progress counters tick without a signal — keep the bar live.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
            text.clone()
        }
        (None, 1) => "1 job done".to_owned(),
        (None, n) if n > 1 => format!("{n} jobs done"),
        (None, _) => return None,
    };
    let response = cl::tip(
        ui.add(
            egui::Label::new(RichText::new(&text).small().color(cl::ink::BASE))
                .sense(egui::Sense::click()),
        ),
        "See what is running and what finished, and clear finished jobs",
    );
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            PilotId::new("shell.status").child(&egui_pilot_ai::PilotKey::new("jobs")),
            Role::Button,
            "Open the job board",
        )
        .value(text),
    );
    response.clicked().then_some(StatusAction::ToggleJobs)
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
        if response.clicked() {
            return Some(StatusAction::ToggleProviders);
        }
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
