//! Reusable interaction patterns — the generic half of what a surface builds.
//!
//! Law (`docs/plans/coldlight-naming-and-the-pattern-library.md` §2): a
//! pattern built for one surface owes the library a GENERIC version in the
//! same pass. These are those versions. Chat's providers carousel is the
//! first caller; nothing here knows that, and nothing here may learn it.
//!
//! They live in ColdLight rather than in `dpl-slides` because they are
//! visual-language primitives that several crates need, and every crate that
//! paints already depends on ColdLight. `dpl-slides` registers thin demos of
//! them into the element census so the Library wall shows them — one
//! implementation, one place it is rendered from.

use crate::{self as cl, ResolvedStyle, Tone, TraitState};
use egui::{
    emath::TSTransform, Align, Color32, CornerRadius, Id, Layout, Rect, Response, RichText, Sense,
    Stroke, Ui, UiBuilder, Vec2,
};

// ── the group chip ───────────────────────────────────────────────────────

/// An elevated label that INTRODUCES a group, valid before the group has any
/// entries.
///
/// This is deliberately a step up the elevation scale from a flat micro
/// heading: a heading is lettering in the rail's own plane, a chip reads as a
/// raised object sitting on it. The distinction carries meaning — a chip says
/// "everything under me is one KIND of thing", which is exactly what a
/// heading over an empty group cannot say.
///
/// `qualifier` is rendered in the warning tone and never abbreviated. Where a
/// group costs money, the qualifier is the only place the operator learns it
/// before choosing, so it is information and not decoration.
pub fn group_chip(ui: &mut Ui, label: &str, qualifier: Option<&str>) -> Response {
    let response = egui::Frame::new()
        .fill(cl::surface::RAISED)
        .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
        .corner_radius(CornerRadius::same(10))
        // The hard lower shadow IS the elevation. One offset, no blur — the
        // same honesty the brutalist patch uses: a raised thing casts a
        // definite edge, not a soft cloud.
        .shadow(egui::Shadow {
            offset: [0, 2],
            blur: 0,
            spread: 0,
            color: Color32::from_black_alpha(190),
        })
        .inner_margin(egui::Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label.to_uppercase())
                        .monospace()
                        .size(10.0)
                        .color(cl::ink::BASE),
                );
                if let Some(qualifier) = qualifier {
                    ui.label(
                        RichText::new(qualifier)
                            .monospace()
                            .size(9.5)
                            .color(Tone::Amber.color()),
                    );
                }
            });
        })
        .response;
    ui.add_space(cl::space::S1);
    response
}

// ── the lift tile ────────────────────────────────────────────────────────

/// How much a lifted tile grows. Small enough that four in a row still read
/// as a row; large enough to be unmistakable at a glance.
pub const LIFT: f32 = 1.09;

/// How long the lift takes. Fast — this is a pointer-following affordance,
/// not a mood.
const LIFT_SECONDS: f32 = 0.16;

/// A tile that rises off the canvas on hover or keyboard focus.
///
/// **Reserve-then-fill.** The tile ALLOCATES its lifted size and PAINTS its
/// resting size inset within it. Growing means filling space that was already
/// spoken for.
///
/// This is not a style choice, it is the only shape that works. egui's own
/// note on `Ui::with_visual_transform` (`egui/src/ui.rs`): *"This only
/// affects visuals, inputs will not be transformed."* So a transform grows
/// the paint and leaves the hit box behind. Growing the real rect instead
/// re-enters the immediate-mode reflow loop — the grown tile shoves its
/// siblings, the pointer lands on a neighbour, this tile un-hovers and
/// shrinks, everything snaps back, and the boundary flickers forever.
///
/// Reserving up front costs ~9% more space per tile at rest and buys a hit
/// box that never moves under the pointer.
///
/// `id_salt` must be stable across frames. An id derived from `ui.id()` is a
/// NEW id every frame, so egui's animation snaps straight to its target and
/// the lift never plays — the same defect that made the providers ink-bleed
/// appear instant.
pub fn lift_tile<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    resting: Vec2,
    selected: bool,
    tone: Tone,
    contents: impl FnOnce(&mut Ui, &ResolvedStyle) -> R,
) -> (Response, R) {
    let reserved = resting * LIFT;
    let (outer, response) = ui.allocate_exact_size(reserved, Sense::click());

    let id = Id::new(("cl.lift_tile", id_salt));
    let raised = response.hovered() || response.has_focus();
    let t = ui.ctx().animate_bool_with_time(
        id,
        raised,
        if ui.is_visible() { LIFT_SECONDS } else { 0.0 },
    );

    let state = if response.is_pointer_button_down_on() {
        TraitState::Active
    } else if raised {
        TraitState::Hover
    } else {
        TraitState::Idle
    };
    let style = cl::compose(tone, "claymorphic", &[], state);

    // The resting rect, centred in what was reserved. The transform scales
    // about that centre, so a tile grows in place and never toward a corner.
    let inner = Rect::from_center_size(outer.center(), resting);
    let scale = 1.0 + (LIFT - 1.0) * t;
    let centre = outer.center().to_vec2();
    let transform = TSTransform::from_translation(centre)
        * TSTransform::from_scaling(scale)
        * TSTransform::from_translation(-centre);

    // The transform scope must NOT allocate again: `allocate_exact_size`
    // above already took this tile's space, and `with_visual_transform` runs
    // a scope that would take more on top of it. Left on the parent, every
    // tile in a wrapped row lands a few pixels lower than the last — a
    // stagger that reads as a rendering fault. A child bounded to `outer`
    // paints in the space already reserved and advances nothing.
    let mut child = ui.new_child(UiBuilder::new().max_rect(outer));
    let inner_result = child
        .with_visual_transform(transform, |ui| {
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(inner)
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    // Elevation reads as the shadow deepening, not as the
                    // tile sliding — the surface is rising toward the light.
                    let depth = 6.0 + 14.0 * t;
                    ui.painter().rect_filled(
                        inner.translate(Vec2::new(0.0, depth * 0.35)),
                        CornerRadius::same(style.radius),
                        Color32::from_black_alpha((90.0 + 90.0 * t) as u8),
                    );
                    let edge = if selected {
                        Stroke::new(1.5, tone.color())
                    } else if t > 0.0 {
                        Stroke::new(1.0, tone.color().gamma_multiply(0.35 + 0.65 * t))
                    } else {
                        style.edge
                    };
                    ui.painter()
                        .rect_filled(inner, CornerRadius::same(style.radius), style.fill);
                    ui.painter().rect_stroke(
                        inner,
                        CornerRadius::same(style.radius),
                        edge,
                        egui::StrokeKind::Inside,
                    );
                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(inner.shrink2(Vec2::new(12.0, 9.0)))
                            .layout(Layout::top_down(Align::Min)),
                        |ui| contents(ui, &style),
                    )
                    .inner
                },
            )
            .inner
        })
        .inner;

    // KEYBOARD REACH. A tile scrolled past the edge of its container is
    // focusable but invisible — Tab lands on it and nothing appears to
    // happen. Bringing the focused tile into view is what makes a scrolling
    // container operable without a pointer at all, and it is the container's
    // job rather than every caller's.
    if response.gained_focus() {
        response.scroll_to_me(Some(Align::Center));
    }

    // A tile mid-lift must keep being asked to paint, or the animation stops
    // at whatever frame the pointer happened to stop moving on.
    if t > 0.0 && t < 1.0 {
        ui.ctx().request_repaint();
    }
    (response, inner_result)
}

// ── the carousel row ─────────────────────────────────────────────────────

/// What a carousel reports back, so a caller can drive its own affordances.
pub struct CarouselRow {
    /// True while there is content scrolled off the left edge.
    pub can_left: bool,
    /// True while there is content scrolled off the right edge.
    pub can_right: bool,
}

/// One row, scrolled horizontally — the shape for a population that is FEW
/// and therefore afforded large tiles.
///
/// A vertical wheel over a one-row carousel is horizontal intent: there is
/// nowhere vertical to go. egui already knows this —
/// `Style::always_scroll_the_only_direction` sums both wheel axes when
/// exactly one direction is enabled — so the flag is set on the scope rather
/// than the behaviour hand-rolled. It defaults to `false`, which is why a
/// plain horizontal `ScrollArea` ignores an ordinary wheel.
///
/// Momentum is drag/touch only — that velocity lives in egui's own
/// `ScrollArea` state. A mouse wheel gets egui's smoothing and no inertia, so
/// "seamless" over a wheel means the animated path, not a flywheel.
pub fn carousel_row(
    ui: &mut Ui,
    id_salt: &str,
    height: f32,
    contents: impl FnOnce(&mut Ui),
) -> CarouselRow {
    ui.style_mut().always_scroll_the_only_direction = true;
    let output = egui::ScrollArea::horizontal()
        .id_salt(id_salt)
        .max_height(height)
        .auto_shrink([false, true])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = cl::space::S3;
                contents(ui);
            });
        });

    let offset = output.state.offset.x;
    let overflow = (output.content_size.x - output.inner_rect.width()).max(0.0);
    CarouselRow {
        can_left: offset > 1.0,
        can_right: offset < overflow - 1.0,
    }
}

// ── the tile grid ────────────────────────────────────────────────────────

/// A grid that never shows fewer than `rows_floor` rows' worth of height —
/// the shape for a population whose VOLUME is the reason it is not a row.
///
/// The floor is a minimum, not a fixed height: given more room the grid uses
/// it. Given less, that is a layout bug upstream and not something to degrade
/// gracefully into a cramped strip.
///
/// Deliberately NOT virtualized. egui's guidance reserves `show_rows` for the
/// scale where per-frame layout of every child actually costs something; a
/// few dozen small frames is not that scale, and `show_rows` would force a
/// uniform row height this grid does not want.
pub fn tile_grid(
    ui: &mut Ui,
    id_salt: &str,
    tile: Vec2,
    rows_floor: usize,
    contents: impl FnOnce(&mut Ui),
) {
    let gap = cl::space::S2;
    let floor = (tile.y + gap) * rows_floor as f32;
    let height = ui.available_height().max(floor);
    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            ui.horizontal_wrapped(contents);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lifted_tile_reserves_more_than_it_paints() {
        // The whole point of reserve-then-fill: the hit box is the LIFTED
        // size from the first frame, so growing never moves it.
        let resting = Vec2::new(160.0, 120.0);
        let reserved = resting * LIFT;
        assert!(reserved.x > resting.x && reserved.y > resting.y);
        // And the resting paint is centred in it, so growth is symmetric —
        // a tile that grew toward a corner would drift under the pointer.
        let outer = Rect::from_min_size(egui::Pos2::ZERO, reserved);
        let inner = Rect::from_center_size(outer.center(), resting);
        assert!((inner.left() - outer.left()) - (outer.right() - inner.right()) < 0.001);
    }

    #[test]
    fn the_grid_floor_is_a_minimum_not_a_cap() {
        let tile = Vec2::new(166.0, 44.0);
        let floor = (tile.y + cl::space::S2) * 4.0;
        // Given less room than the floor, the floor wins.
        assert_eq!(120.0_f32.max(floor), floor);
        // Given more, the room wins — four rows is not a ceiling.
        assert_eq!(900.0_f32.max(floor), 900.0);
    }
}

// ── the bleed control ────────────────────────────────────────────────────
//
// This was built for the shell status bar (V1) and never generalized, which
// is exactly the gap the pattern-library law exists to close. The chat rail
// needs the SAME control, so it moved here and both callers paint it.

/// The authored bleed this control is the origin of.
pub const BLEED: &str = "providers-ink";

/// The seeded state a bleed control keeps for its lifetime: which pair of
/// hues it drew, and how long its dwell runs.
///
/// Rolled ONCE per session, not per frame — a pair that re-drew on every
/// repaint would strobe, and a proof shot could never reproduce it.
#[derive(Debug, Clone, Copy)]
pub struct BleedRoll {
    pub a: Color32,
    pub b: Color32,
    pub dwell: f32,
}

impl BleedRoll {
    /// Draw the pair and the dwell from `seed`. Deterministic, so `--mood
    /// <n>` pins the bleed for a capture the way it already pins the
    /// backdrop.
    pub fn from_seed(seed: u64) -> Self {
        let preset = crate::bleed_preset(BLEED).expect("the providers bleed is authored");
        let (a, b) = preset.pair(seed);
        // The jitter rides inside the AUTHORED window, never a literal.
        let span = crate::motion::DUR_DWELL_MAX - crate::motion::DUR_DWELL_MIN;
        let frac = ((seed >> 16) % 1000) as f32 / 1000.0;
        Self {
            a,
            b,
            dwell: crate::motion::DUR_DWELL_MIN + span * frac,
        }
    }
}

/// The label size the control lays out at.
pub const BLEED_LABEL_SIZE: f32 = 15.0;
/// The `ai` circle's diameter IS the cap-height of the P — the operator's
/// alignment, expressed as a ratio rather than a second magic number.
const AI_D: f32 = BLEED_LABEL_SIZE * 0.72;
const BRAIN: f32 = 22.0;
const GAP: f32 = 8.0;
const PAD_L: f32 = 9.0;
const PAD_R: f32 = 12.0;
const BLEED_H: f32 = 26.0;

/// The width this control wants for `label`, so a caller can budget for it.
pub fn bleed_control_width(ui: &Ui, label: &str) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(BLEED_LABEL_SIZE),
        crate::ink::MUTED,
    );
    PAD_L + AI_D + GAP + BRAIN + GAP + galley.size().x + PAD_R
}

/// The `(ai) 🧠 <label>` control: line-art in muted ink at rest, two seeded
/// hues seeping in over the authored dwell on hover.
///
/// `anim_id` MUST be stable across frames. A `ui.id()`-derived id is a new id
/// every frame, and egui treats an unseen id as already-arrived — which turns
/// a six-second bleed into an instant swap. That defect cost a whole pass
/// once; the parameter is explicit so it cannot happen quietly again.
///
/// `width` lets a caller stretch the control to fill a rail; `None` measures.
pub fn bleed_control(
    ui: &mut Ui,
    anim_id: Id,
    label: &str,
    roll: BleedRoll,
    width: Option<f32>,
) -> Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(BLEED_LABEL_SIZE),
        crate::ink::MUTED,
    );
    let w = width.unwrap_or_else(|| bleed_control_width(ui, label));
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, BLEED_H), Sense::click());

    // Hover OR keyboard focus — a control only reachable by pointer is one
    // half of the operators who use it.
    let t =
        ui.ctx()
            .animate_bool_with_time(anim_id, resp.hovered() || resp.has_focus(), roll.dwell);
    // The wash only advances on frames that actually run, so drive them for
    // as long as it is mid-bleed.
    if t > 0.0 && t < 1.0 {
        ui.ctx().request_repaint();
    }

    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(crate::RADIUS_SM),
        crate::surface::RAISED,
    );
    if t > 0.0 {
        // Two origins, seeping outward — SCALE, not opacity, so it reads as
        // ink spreading into paper rather than a panel fading up.
        bleed_wash(painter, rect, roll.a, t, -0.18, -0.35);
        bleed_wash(painter, rect, roll.b, t, 0.62, 0.40);
    }

    let ink = lerp_color(crate::ink::SUBTLE, roll.a, t);
    let mut x = rect.min.x + PAD_L;

    let c = egui::pos2(x + AI_D / 2.0, rect.center().y);
    painter.circle_stroke(c, AI_D / 2.0, Stroke::new(1.4, ink));
    painter.text(
        c,
        egui::Align2::CENTER_CENTER,
        "ai",
        egui::FontId::proportional(AI_D * 0.52),
        ink,
    );
    x += AI_D + GAP;

    let brain = Rect::from_min_size(
        egui::pos2(x, rect.center().y - BRAIN / 2.0),
        Vec2::splat(BRAIN),
    );
    crate::draw_icon(painter, "brain", brain, ink);
    x += BRAIN + GAP;

    painter.galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        lerp_color(crate::ink::MUTED, crate::ink::STRONG, t),
    );
    resp
}

/// The brain alone, for a rail too narrow to carry the word (the chat
/// window's `PresenceRail` at 76px). Same bleed, same id discipline.
pub fn bleed_glyph(ui: &mut Ui, anim_id: Id, roll: BleedRoll) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(BLEED_H), Sense::click());
    let t =
        ui.ctx()
            .animate_bool_with_time(anim_id, resp.hovered() || resp.has_focus(), roll.dwell);
    if t > 0.0 && t < 1.0 {
        ui.ctx().request_repaint();
    }
    let painter = ui.painter();
    if t > 0.0 {
        bleed_wash(painter, rect, roll.a, t, 0.2, 0.2);
    }
    let brain = Rect::from_center_size(rect.center(), Vec2::splat(BRAIN));
    crate::draw_icon(
        painter,
        "brain",
        brain,
        lerp_color(crate::ink::SUBTLE, roll.a, t),
    );
    resp
}

/// One origin of the bleed — concentric rings, widest and faintest first.
fn bleed_wash(painter: &egui::Painter, rect: Rect, hue: Color32, t: f32, ox: f32, oy: f32) {
    let origin = egui::pos2(
        rect.min.x + rect.width() * ox,
        rect.min.y + rect.height() * oy,
    );
    let full = rect.width() * 0.95;
    let r = full * (0.04 + 0.96 * t);
    let peak = 0.42 * t;
    let rings = 7;
    let clip = painter.with_clip_rect(rect);
    for i in (1..=rings).rev() {
        let f = i as f32 / rings as f32;
        let a = (peak * (1.0 - f) * (1.0 - f) * 255.0) as u8;
        if a == 0 {
            continue;
        }
        clip.circle_filled(
            origin,
            r * f,
            Color32::from_rgba_unmultiplied(hue.r(), hue.g(), hue.b(), a),
        );
    }
}

/// Linear interpolation between two inks.
pub fn lerp_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgb(
        m(from.r(), to.r()),
        m(from.g(), to.g()),
        m(from.b(), to.b()),
    )
}
