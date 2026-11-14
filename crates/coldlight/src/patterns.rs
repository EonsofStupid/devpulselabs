//! Reusable interaction patterns — the generic half of what a surface builds.
//!
//! A pattern built for one surface owes the library a GENERIC version in the
//! same pass. These are those versions. Chat's providers rail and catalog are
//! the first callers; nothing here knows that, and nothing here may learn it.
//!
//! Every value painted here comes from the authored tokens: colour from the
//! trait compositor (`compose`), the surface and ink ramps and the hue pool;
//! sizes from `space`, the radius consts and the chrome text styles; timing
//! from `motion`. The few bare numbers left are PROPORTIONS (how far a tile
//! lifts, where a wash starts), never pixel sizes. No blur anywhere: depth is
//! a hard offset shadow and concentric rings, per the doctrine.

use crate::{compose, ink, motion, space, surface, ResolvedStyle, Tone, TraitState};
use egui::{
    emath::TSTransform, Align, Color32, CornerRadius, Id, Layout, Rect, Response, RichText, Sense,
    Stroke, TextStyle, Ui, UiBuilder, Vec2,
};

/// The treatment every pattern here dresses in: the quiet workhorse plate
/// with a hard offset shadow and no blur. Its `Active` state is the one
/// selected control on a surface (tone edge, tone wash).
const PLATE: &str = "vibron";

// ── the group chip ───────────────────────────────────────────────────────

/// An elevated label that INTRODUCES a group, valid before the group has any
/// entries.
///
/// Deliberately a step up the elevation scale from a flat micro heading: a
/// heading is lettering in the rail's own plane, a chip is a raised object
/// sitting on it. The distinction carries meaning — a chip says "everything
/// under me is one KIND of thing", which a heading over an empty group cannot.
///
/// `qualifier` is rendered in the warning tone and never abbreviated. Where a
/// group costs money, the qualifier is the only place the operator learns it
/// before choosing, so it is information and not decoration.
pub fn group_chip(ui: &mut Ui, label: &str, qualifier: Option<&str>) -> Response {
    let style = compose(Tone::Cyan, PLATE, &[], TraitState::Idle);
    let mut frame = egui::Frame::new()
        .fill(style.fill)
        .stroke(style.edge)
        .corner_radius(CornerRadius::same(style.radius))
        .inner_margin(egui::Margin::symmetric(
            space::S2 as i8,
            (space::S1 / 2.0) as i8,
        ));
    // The hard lower shadow IS the elevation: one offset, zero blur.
    if let Some(shadow) = style.shadows.first() {
        frame = frame.shadow(*shadow);
    }
    let response = frame
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = space::S1;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label.to_uppercase())
                        .small()
                        .strong()
                        .color(style.ink),
                );
                if let Some(qualifier) = qualifier {
                    ui.label(RichText::new(qualifier).small().color(Tone::Amber.color()));
                }
            });
        })
        .response;
    ui.add_space(space::S1);
    response
}

// ── the lift tile ────────────────────────────────────────────────────────

/// How much a lifted tile grows, as a proportion of its resting size. Small
/// enough that four in a row still read as a row; large enough to be
/// unmistakable at a glance.
pub const LIFT: f32 = 1.09;

/// The size a tile with `resting` paint RESERVES — its hit box, from the
/// first frame, whether or not it is lifted.
pub fn lift_reserve(resting: Vec2) -> Vec2 {
    resting * LIFT
}

/// A tile that rises off the canvas on hover or keyboard focus.
///
/// **Reserve-then-fill.** The tile ALLOCATES its lifted size and PAINTS its
/// resting size inset within it. Growing means filling space that was already
/// spoken for.
///
/// This is not a style choice, it is the only shape that works. egui's own
/// note on `Ui::with_visual_transform`: *"This only affects visuals, inputs
/// will not be transformed."* So a transform grows the paint and leaves the
/// hit box behind. Growing the real rect instead re-enters the immediate-mode
/// reflow loop — the grown tile shoves its siblings, the pointer lands on a
/// neighbour, this tile un-hovers and shrinks, and the boundary flickers.
///
/// `id_salt` must be stable across frames. An id derived from `ui.id()` is a
/// NEW id every frame, so egui's animation snaps straight to its target and
/// the lift never plays.
///
/// `selected` paints the plate's `Active` state — the tone edge and wash that
/// mark the one chosen thing on a surface. The caller still owes the tooltip
/// and the pilot node.
pub fn lift_tile<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    resting: Vec2,
    selected: bool,
    tone: Tone,
    contents: impl FnOnce(&mut Ui, &ResolvedStyle) -> R,
) -> (Response, R) {
    let (outer, response) = ui.allocate_exact_size(lift_reserve(resting), Sense::click());

    let id = Id::new(("cl.lift_tile", id_salt));
    let raised = response.hovered() || response.has_focus();
    let t = ui.ctx().animate_bool_with_time(
        id,
        raised,
        if ui.is_visible() {
            motion::DUR_FAST
        } else {
            0.0
        },
    );

    let state = if selected || response.is_pointer_button_down_on() {
        TraitState::Active
    } else if raised {
        TraitState::Hover
    } else {
        TraitState::Idle
    };
    let style = compose(tone, PLATE, &[], state);
    // The lift deepens the shadow along the authored idle→hover offsets, so
    // elevation reads as the surface rising toward the light, never a slide.
    let rest_shadow = compose(tone, PLATE, &[], TraitState::Idle).shadows;
    let lift_shadow = compose(tone, PLATE, &[], TraitState::Hover).shadows;

    // The resting rect, centred in what was reserved. The transform scales
    // about that centre, so a tile grows in place and never toward a corner.
    let inner = Rect::from_center_size(outer.center(), resting);
    let scale = 1.0 + (LIFT - 1.0) * t;
    let centre = outer.center().to_vec2();
    let transform = TSTransform::from_translation(centre)
        * TSTransform::from_scaling(scale)
        * TSTransform::from_translation(-centre);

    // The transform scope must NOT allocate again: `allocate_exact_size`
    // above already took this tile's space. A child bounded to `outer`
    // paints in the space already reserved and advances nothing — left on
    // the parent, every tile in a wrapped row would land lower than the last.
    let mut child = ui.new_child(UiBuilder::new().max_rect(outer));
    let inner_result = child
        .with_visual_transform(transform, |ui| {
            let radius = CornerRadius::same(style.radius);
            if let (Some(rest), Some(lift)) = (rest_shadow.first(), lift_shadow.first()) {
                let offset = Vec2::new(
                    f32::from(rest.offset[0]) + f32::from(lift.offset[0] - rest.offset[0]) * t,
                    f32::from(rest.offset[1]) + f32::from(lift.offset[1] - rest.offset[1]) * t,
                );
                ui.painter()
                    .rect_filled(inner.translate(offset), radius, lift.color);
            }
            let edge = if selected {
                style.edge
            } else {
                Stroke::new(
                    style.edge.width,
                    style.edge.color.lerp_to_gamma(tone.color(), t),
                )
            };
            ui.painter().rect_filled(inner, radius, style.fill);
            ui.painter()
                .rect_stroke(inner, radius, edge, egui::StrokeKind::Inside);
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(inner.shrink2(Vec2::new(space::S3, space::S2)))
                    .layout(Layout::top_down(Align::Min)),
                |ui| contents(ui, &style),
            )
            .inner
        })
        .inner;

    // KEYBOARD REACH. A tile scrolled past the edge of its container is
    // focusable but invisible — Tab lands on it and nothing appears to
    // happen. Bringing the focused tile into view makes a scrolling container
    // operable without a pointer at all.
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
/// exactly one direction is enabled — so the flag is set on a scope rather
/// than the behaviour hand-rolled.
pub fn carousel_row(
    ui: &mut Ui,
    id_salt: &str,
    height: f32,
    contents: impl FnOnce(&mut Ui),
) -> CarouselRow {
    // SCOPED, not set. `style_mut` mutates this `Ui`'s own style for the
    // rest of its life, so setting the flag on the caller's `ui` would leak
    // it to every widget added after the carousel.
    let output = ui
        .scope(|ui| {
            ui.style_mut().always_scroll_the_only_direction = true;
            egui::ScrollArea::horizontal()
                .id_salt(id_salt)
                .max_height(height)
                .auto_shrink([false, true])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = space::S3;
                        contents(ui);
                    });
                })
        })
        .inner;

    // One physical pixel of tolerance: anything less is sub-pixel jitter
    // from the scroll animation, not content the operator cannot see.
    let pixel = ui.ctx().pixels_per_point().recip();
    let offset = output.state.offset.x;
    let overflow = (output.content_size.x - output.inner_rect.width()).max(0.0);
    CarouselRow {
        can_left: offset > pixel,
        can_right: offset < overflow - pixel,
    }
}

// ── the tile grid ────────────────────────────────────────────────────────

/// The gap between grid tiles.
const GRID_GAP: f32 = space::S2;

/// The least height a grid of lift tiles sized `resting` takes for
/// `rows_floor` rows. A FLOOR, never a cap.
pub fn grid_floor(resting: Vec2, rows_floor: usize) -> f32 {
    (lift_reserve(resting).y + GRID_GAP) * rows_floor as f32
}

/// A wrapped grid that never takes less than `rows_floor` rows' worth of
/// height — the shape for a population whose VOLUME is the reason it is not
/// a row.
///
/// The floor is a minimum, not a fixed height: more tiles take more rows.
/// The grid does not scroll by itself; it lives inside its surface's own
/// vertical scroll, so a page never nests two scrolls on one axis.
///
/// Deliberately NOT virtualized. egui reserves `show_rows` for the scale
/// where per-frame layout of every child actually costs something; a few
/// dozen small tiles is not that scale.
pub fn tile_grid(
    ui: &mut Ui,
    resting: Vec2,
    rows_floor: usize,
    contents: impl FnOnce(&mut Ui),
) -> Response {
    ui.scope(|ui| {
        ui.set_min_height(grid_floor(resting, rows_floor));
        ui.spacing_mut().item_spacing = Vec2::splat(GRID_GAP);
        ui.horizontal_wrapped(contents);
    })
    .response
}

// ── the bleed control ────────────────────────────────────────────────────
//
// Built for the shell status bar first. The chat rail needs the SAME control,
// so it lives here and both callers paint one implementation of a six-second
// animation instead of two that drift.

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
        let span = motion::DUR_DWELL_MAX - motion::DUR_DWELL_MIN;
        let frac = ((seed >> 16) % 1000) as f32 / 1000.0;
        Self {
            a,
            b,
            dwell: motion::DUR_DWELL_MIN + span * frac,
        }
    }
}

/// The control's height: one chrome row and a half-step of breathing room.
const BLEED_H: f32 = space::S5 + space::S1 / 2.0;
/// The brain, a half-step inside the row so its line-art never kisses the
/// control's edge.
const BRAIN: f32 = space::S5 - space::S1 / 2.0;
const GAP: f32 = space::S2;
const PAD_L: f32 = space::S2;
const PAD_R: f32 = space::S3;
/// The `ai` circle's diameter IS the cap-height of the P beside it — the
/// operator's alignment, as a proportion of the label's size.
const AI_RING_OF_LABEL: f32 = 0.72;
/// The ring's weight and its lettering, as proportions of the ring.
const AI_RING_WEIGHT: f32 = 0.13;
const AI_TEXT_OF_RING: f32 = 0.52;

/// The chrome font the control's word is set in.
fn bleed_font(ui: &Ui) -> egui::FontId {
    TextStyle::Button.resolve(ui.style())
}

/// The width this control wants for `label`, so a caller can budget for it.
pub fn bleed_control_width(ui: &Ui, label: &str) -> f32 {
    let font = bleed_font(ui);
    let ring = font.size * AI_RING_OF_LABEL;
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, ink::MUTED);
    PAD_L + ring + GAP + BRAIN + GAP + galley.size().x + PAD_R
}

/// The `(ai) brain <label>` control: line-art in muted ink at rest, two
/// seeded hues seeping in over the authored dwell on hover or focus.
///
/// `anim_id` MUST be stable across frames. A `ui.id()`-derived id is a new id
/// every frame, and egui treats an unseen id as already-arrived — which turns
/// a six-second bleed into an instant swap.
///
/// `width` lets a caller stretch the control to fill a rail; `None` measures.
/// The caller owes the tooltip.
pub fn bleed_control(
    ui: &mut Ui,
    anim_id: Id,
    label: &str,
    roll: BleedRoll,
    width: Option<f32>,
) -> Response {
    let font = bleed_font(ui);
    let ring = font.size * AI_RING_OF_LABEL;
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, ink::MUTED);
    let w = width.unwrap_or_else(|| bleed_control_width(ui, label));
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, BLEED_H), Sense::click());

    let t = bleed_factor(ui, anim_id, &resp, roll);

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(crate::RADIUS_SM), surface::RAISED);
    if t > 0.0 {
        // Two origins, seeping outward — SCALE, not opacity, so it reads as
        // ink spreading into paper rather than a panel fading up.
        bleed_wash(painter, rect, roll.a, t, WASH_A);
        bleed_wash(painter, rect, roll.b, t, WASH_B);
    }

    let line = ink::SUBTLE.lerp_to_gamma(roll.a, t);
    let mut x = rect.min.x + PAD_L;

    let c = egui::pos2(x + ring / 2.0, rect.center().y);
    painter.circle_stroke(c, ring / 2.0, Stroke::new(ring * AI_RING_WEIGHT, line));
    painter.text(
        c,
        egui::Align2::CENTER_CENTER,
        "ai",
        egui::FontId::proportional(ring * AI_TEXT_OF_RING),
        line,
    );
    x += ring + GAP;

    let brain = Rect::from_min_size(
        egui::pos2(x, rect.center().y - BRAIN / 2.0),
        Vec2::splat(BRAIN),
    );
    crate::draw_icon(painter, "brain", brain, line);
    x += BRAIN + GAP;

    painter.galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        ink::MUTED.lerp_to_gamma(ink::STRONG, t),
    );
    resp
}

/// The brain alone, for a rail too narrow to carry the word (the chat
/// window's presence rail). Same bleed, same id discipline. The caller owes
/// the tooltip.
pub fn bleed_glyph(ui: &mut Ui, anim_id: Id, roll: BleedRoll) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(BLEED_H), Sense::click());
    let t = bleed_factor(ui, anim_id, &resp, roll);
    let painter = ui.painter();
    if t > 0.0 {
        bleed_wash(painter, rect, roll.a, t, WASH_GLYPH);
    }
    let brain = Rect::from_center_size(rect.center(), Vec2::splat(BRAIN));
    crate::draw_icon(
        painter,
        "brain",
        brain,
        ink::SUBTLE.lerp_to_gamma(roll.a, t),
    );
    resp
}

/// Hover OR keyboard focus drives one factor over the seeded dwell — a
/// control only reachable by pointer is one half of the operators who use it.
fn bleed_factor(ui: &Ui, anim_id: Id, resp: &Response, roll: BleedRoll) -> f32 {
    let t =
        ui.ctx()
            .animate_bool_with_time(anim_id, resp.hovered() || resp.has_focus(), roll.dwell);
    // The wash only advances on frames that actually run, so drive them for
    // as long as it is mid-bleed.
    if t > 0.0 && t < 1.0 {
        ui.ctx().request_repaint();
    }
    t
}

/// Where a wash starts, as fractions of the control (may sit outside it, so
/// the ink seeps IN from the corner rather than appearing on it).
#[derive(Clone, Copy)]
struct WashOrigin {
    x: f32,
    y: f32,
}

const WASH_A: WashOrigin = WashOrigin { x: -0.18, y: -0.35 };
const WASH_B: WashOrigin = WashOrigin { x: 0.62, y: 0.40 };
const WASH_GLYPH: WashOrigin = WashOrigin { x: 0.2, y: 0.2 };
/// How far a full wash reaches, as a fraction of the control's width.
const WASH_REACH: f32 = 0.95;
/// The seed radius a wash starts from, as a fraction of its reach.
const WASH_SEED: f32 = 0.04;
/// The densest a wash ever gets, as an opacity.
const WASH_PEAK: f32 = 0.42;
/// Concentric rings per wash — enough that the falloff reads as soft without
/// a blur.
const WASH_RINGS: u32 = 7;

/// One origin of the bleed — concentric rings, widest and faintest first.
fn bleed_wash(painter: &egui::Painter, rect: Rect, hue: Color32, t: f32, origin: WashOrigin) {
    let at = egui::pos2(
        rect.min.x + rect.width() * origin.x,
        rect.min.y + rect.height() * origin.y,
    );
    let r = rect.width() * WASH_REACH * (WASH_SEED + (1.0 - WASH_SEED) * t);
    let peak = WASH_PEAK * t;
    let clip = painter.with_clip_rect(rect);
    for i in (1..=WASH_RINGS).rev() {
        let f = i as f32 / WASH_RINGS as f32;
        let alpha = peak * (1.0 - f) * (1.0 - f);
        if alpha <= 0.0 {
            continue;
        }
        clip.circle_filled(at, r * f, hue.gamma_multiply(alpha));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Vec2 = Vec2::new(800.0, 600.0);
    const RESTING: Vec2 = Vec2::new(160.0, 120.0);

    /// One frame of two side-by-side lift tiles, with the pointer at
    /// `pointer`. Returns both tiles' hit rects and whether the first is
    /// hovered.
    fn two_tiles(
        ctx: &egui::Context,
        time: f64,
        pointer: Option<egui::Pos2>,
    ) -> (Rect, Rect, bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            time: Some(time),
            ..Default::default()
        };
        if let Some(pointer) = pointer {
            input.events.push(egui::Event::PointerMoved(pointer));
        }
        let mut seen = None;
        ctx.run_ui(input, |ui| {
            ui.horizontal(|ui| {
                let (first, ()) = lift_tile(ui, "a", RESTING, false, Tone::Cyan, |_, _| ());
                let (second, ()) = lift_tile(ui, "b", RESTING, false, Tone::Cyan, |_, _| ());
                seen = Some((first.rect, second.rect, first.hovered()));
            });
        })
        .textures_delta
        .clear();
        seen.expect("the frame ran")
    }

    #[test]
    fn a_lifted_tile_keeps_its_hit_rect_and_its_neighbour_still() {
        let ctx = egui::Context::default();
        let (rest_a, rest_b, hovered) = two_tiles(&ctx, 0.0, None);
        assert!(!hovered);
        // The hit box is the LIFTED size from the very first frame (egui
        // snaps allocations to the pixel grid, hence the one-pixel slack).
        assert!((rest_a.size() - lift_reserve(RESTING)).length() <= 1.0);

        // Hover the first tile and let the lift run to completion.
        let mut lifted = (rest_a, rest_b, false);
        for frame in 1..=20 {
            lifted = two_tiles(&ctx, f64::from(frame) * 0.05, Some(rest_a.center()));
        }
        let (lift_a, lift_b, hovered) = lifted;
        assert!(hovered, "the pointer is over the first tile");
        assert_eq!(lift_a, rest_a, "lifting must not move the hit box");
        assert_eq!(lift_b, rest_b, "lifting must not shove the neighbour");
    }

    #[test]
    fn the_lift_grows_in_place_about_the_centre() {
        // The resting paint is centred in the reservation, so growth is
        // symmetric — a tile that grew toward a corner would drift under the
        // pointer.
        let outer = Rect::from_min_size(egui::Pos2::ZERO, lift_reserve(RESTING));
        let inner = Rect::from_center_size(outer.center(), RESTING);
        assert!(((inner.left() - outer.left()) - (outer.right() - inner.right())).abs() < 0.001);
        assert!(((inner.top() - outer.top()) - (outer.bottom() - inner.bottom())).abs() < 0.001);
        // And fully lifted, the paint fills exactly what was reserved.
        assert_eq!(RESTING * LIFT, outer.size());
    }

    /// A helper that changes style must not change it for its CALLER. The
    /// carousel enables `always_scroll_the_only_direction`; a leak would make
    /// whatever is laid out after it scroll sideways on a vertical wheel.
    #[test]
    fn the_carousel_does_not_leak_its_style_to_the_caller() {
        let ctx = egui::Context::default();
        let mut leaked = None;
        // epaint 0.36 panics if a TexturesDelta is dropped unapplied, so the
        // output is cleared rather than discarded.
        ctx.run_ui(Default::default(), |ui| {
            assert!(!ui.style().always_scroll_the_only_direction);
            carousel_row(ui, "test.carousel", 40.0, |ui| {
                ui.label("tile");
            });
            leaked = Some(ui.style().always_scroll_the_only_direction);
        })
        .textures_delta
        .clear();
        assert_eq!(
            leaked,
            Some(false),
            "carousel_row left always_scroll_the_only_direction set on its caller"
        );
    }

    fn grid_height(tiles: usize, width: f32) -> f32 {
        let ctx = egui::Context::default();
        let mut height = None;
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(width, 4000.0),
                )),
                ..Default::default()
            },
            |ui| {
                let response = tile_grid(ui, RESTING, 2, |ui| {
                    for i in 0..tiles {
                        let _ = lift_tile(ui, i, RESTING, false, Tone::Cyan, |_, _| ());
                    }
                });
                height = Some(response.rect.height());
            },
        )
        .textures_delta
        .clear();
        height.expect("the frame ran")
    }

    #[test]
    fn the_grid_floor_is_a_minimum_not_a_cap() {
        let floor = grid_floor(RESTING, 2);
        // One tile still takes the floor's two rows — the section never
        // collapses into a strip (to within egui's pixel snapping).
        assert!(grid_height(1, 800.0) >= floor - 1.0);
        // Many tiles in a narrow column take MORE than the floor.
        assert!(grid_height(12, 400.0) > floor);
    }

    #[test]
    fn the_roll_is_seeded_and_lands_inside_the_authored_window() {
        for seed in [0u64, 1, 42, 9_999] {
            let r = BleedRoll::from_seed(seed);
            assert_eq!(
                (r.a, r.b, r.dwell.to_bits()),
                (
                    BleedRoll::from_seed(seed).a,
                    BleedRoll::from_seed(seed).b,
                    BleedRoll::from_seed(seed).dwell.to_bits()
                ),
                "the same seed must draw the same bleed — proof shots depend on it"
            );
            assert!(
                (motion::DUR_DWELL_MIN..=motion::DUR_DWELL_MAX).contains(&r.dwell),
                "dwell {} escaped the authored window",
                r.dwell
            );
            assert_ne!(r.a, r.b, "a two-colour bleed needs two colours");
        }
    }

    #[test]
    fn the_dwell_is_a_passive_experience_not_a_transition() {
        // The whole point of the tier: nobody should be able to quietly
        // retune this to feel "snappy".
        let r = BleedRoll::from_seed(7);
        assert!(
            r.dwell > 5.0,
            "the bleed is passive by design, got {}s",
            r.dwell
        );
    }

    #[test]
    fn the_bleed_control_measures_what_it_paints() {
        let ctx = egui::Context::default();
        crate::apply_theme(&ctx);
        let roll = BleedRoll::from_seed(3);
        let mut seen = None;
        ctx.run_ui(Default::default(), |ui| {
            let want = bleed_control_width(ui, "Providers");
            let got = bleed_control(ui, Id::new("test.bleed"), "Providers", roll, None);
            seen = Some((want, got.rect.width(), got.rect.height()));
        })
        .textures_delta
        .clear();
        let (want, width, height) = seen.expect("the frame ran");
        // Exactly what is drawn, to within egui's pixel snapping.
        assert!((want - width).abs() <= 1.0, "budgeted {want}, drew {width}");
        assert_eq!(height, BLEED_H);
    }
}
