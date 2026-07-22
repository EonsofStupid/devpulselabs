//! The STATUS zone as a data-driven indicator grid — the operator's
//! "well established grids so i can evolve... cssgrid for all of my status
//! indicators", as a registry (Law 4): the bar is three lane columns, a cell
//! is one `StatusCell` in [`CELLS`], and adding an indicator is adding ONE
//! entry there. Cells paint; the shell mutates — a cell answers with a
//! [`StatusAction`] and the owner executes it (the PaletteActions pattern).

use dpl_coldlight::{self as cl, Mood, Tone};
use egui::{Align, Layout, Rect, RichText, Ui, UiBuilder};

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
    /// Summon the PixVix overlay (the Snip launcher).
    Snip,
    /// Route to the agent hub (the Agents launcher).
    Agents,
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
        id: "agents",
        lane: Lane::Right,
        render: agents,
    },
    StatusCell {
        id: "signals",
        lane: Lane::Right,
        render: signals,
    },
    StatusCell {
        id: "build",
        lane: Lane::Right,
        render: build,
    },
];

/// Lay the bar as three lane columns and run every registered cell.
pub fn grid(ui: &mut Ui, cx: &StatusCtx) -> Vec<StatusAction> {
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
    crate::launcher_chip(ui, cl::icon::SNIP, "Snip", cx.tone).then_some(StatusAction::Snip)
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

fn agents(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    crate::launcher_chip(ui, cl::icon::BOT, "Agents", cx.tone).then_some(StatusAction::Agents)
}

fn signals(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    ui.label(
        RichText::new(format!("signals · {}", cx.signals))
            .small()
            .color(cl::ink::SUBTLE),
    );
    None
}

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
