//! ColdLight, Rust output target.
//!
//! The JSON tokens in the prior repo remain the authored source; this crate is
//! their second emitter (hand-seeded for P0, generated in P1 — same
//! Style-Dictionary thesis, new target). The paint vocabulary is the
//! operator's, verbatim: **crisp sprite-grade edges, solid fills with gradient
//! tint, black diffusion scatter** — dark falloff painted on top, never
//! backdrop blur. Cheap on this renderer by design.

use egui::{Color32, CornerRadius, Rect, Shadow, Stroke};

// ── THE GENERATED ENGINE ────────────────────────────────────────────────
// build.rs ports the palette engine (generate-scales.mjs) and the fluid XY
// clamp core (fluid-axis.mjs) from the vendored authored sources. Editing
// tokens/*.json and running `cargo build` IS the design build.
include!(concat!(env!("OUT_DIR"), "/ramps_gen.rs"));

/// Index of the 500 step (the accent anchor) in `LADDER_STEPS`.
pub const STEP_500: usize = 6;

/// Position of a ladder step, when it exists.
pub fn step_index(step: u32) -> Option<usize> {
    LADDER_STEPS.iter().position(|s| *s == step)
}

/// The hub tone slots — semantic names over the GENERATED pool. No hex lives
/// here anymore: every color resolves through a curved 13-step OKLCH ladder.
/// (The 18 fx-tone slots arrive with the fx port; this enum is the shell's.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    Cyan,
    Lime,
    Violet,
    Amber,
    Magenta,
    Coral,
}

impl Tone {
    /// The hue id this tone resolves through in the pool.
    pub const fn hue_id(self) -> &'static str {
        match self {
            Tone::Cyan => "cyan",
            Tone::Lime => "lime",
            Tone::Violet => "violet",
            Tone::Amber => "amber",
            Tone::Magenta => "magenta",
            Tone::Coral => "coral",
        }
    }

    /// The saturated accent — the ladder's 400 step (curved, not flat).
    pub fn color(self) -> Color32 {
        // The pool is generated from the same registry these ids come from, so
        // the lookup cannot miss; degrade loudly in debug if it ever does.
        match ramp(self.hue_id()) {
            Some(r) => r.steps[STEP_500 - 1], // 400 — punchier on the dark stack
            None => {
                debug_assert!(false, "tone hue missing from pool: {}", self.hue_id());
                Color32::GRAY
            }
        }
    }

    /// Low-alpha fill for active rail items and selected cards.
    pub fn wash(self) -> Color32 {
        let c = self.color();
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 26)
    }

    /// The gradient-tint head color (fades to transparent).
    pub fn tint(self) -> Color32 {
        let c = self.color();
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 18)
    }
}

/// Surfaces — the dark stack. The WELL is the recessed content plane; chrome
/// plates (bars, rails) sit on RAISED above it and cast diffusion into it.
pub mod surface {
    use egui::Color32;
    /// The recessed content plane — everything is framed against this.
    pub const WELL: Color32 = Color32::from_rgb(0x07, 0x09, 0x0e);
    pub const BASE: Color32 = Color32::from_rgb(0x0a, 0x0c, 0x12);
    pub const RAISED: Color32 = Color32::from_rgb(0x11, 0x14, 0x1d);
    pub const OVERLAY: Color32 = Color32::from_rgb(0x18, 0x1c, 0x27);
    pub const HAIRLINE: Color32 = Color32::from_rgb(0x23, 0x28, 0x36);
}

/// Ink — the text stack.
pub mod ink {
    use egui::Color32;
    pub const STRONG: Color32 = Color32::from_rgb(0xee, 0xf2, 0xf9);
    pub const BASE: Color32 = Color32::from_rgb(0xc2, 0xc9, 0xd6);
    pub const MUTED: Color32 = Color32::from_rgb(0x8a, 0x93, 0xa6);
    pub const SUBTLE: Color32 = Color32::from_rgb(0x5b, 0x64, 0x78);
}

/// Spacing steps. Fluid ramps (viewport-aware fns) arrive in P1;
/// the STEP names are already the ones the fluid law will keep.
pub mod space {
    pub const S1: f32 = 4.0;
    pub const S2: f32 = 8.0;
    pub const S3: f32 = 12.0;
    pub const S4: f32 = 16.0;
    pub const S5: f32 = 24.0;
    pub const S6: f32 = 32.0;
}

pub const RADIUS: u8 = 10;
pub const RADIUS_SM: u8 = 6;

/// Apply the dark theme once at boot.
pub fn apply_theme(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};

    // Dark is the only theme ColdLight ships; pin it and style that slot.
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(space::S2, space::S2);
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = surface::BASE;
    style.visuals.window_fill = surface::RAISED;
    style.visuals.extreme_bg_color = surface::BASE;
    style.visuals.override_text_color = Some(ink::BASE);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, surface::HAIRLINE);
    ctx.set_style_of(egui::Theme::Dark, style);
}

/// The scatter card: solid raised fill, crisp hairline edge, and the black
/// diffusion — a dark shadow bleeding down and out. This is the house look.
pub fn scatter_frame(tone: Tone) -> egui::Frame {
    egui::Frame::new()
        .fill(surface::RAISED)
        .stroke(Stroke::new(1.0, tone_edge(tone)))
        .corner_radius(CornerRadius::same(RADIUS))
        .shadow(Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 2,
            color: Color32::from_black_alpha(160),
        })
        .inner_margin(egui::Margin::same(14))
        .outer_margin(egui::Margin::symmetric(2, 6))
}

/// The quiet sibling: same geometry, hairline edge, softer scatter. For
/// content that should sit back until it is the active thing.
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(surface::RAISED)
        .stroke(Stroke::new(1.0, surface::HAIRLINE))
        .corner_radius(CornerRadius::same(RADIUS))
        .shadow(Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(100),
        })
        .inner_margin(egui::Margin::same(14))
        .outer_margin(egui::Margin::symmetric(2, 6))
}

/// A hairline that leans toward the tone without shouting.
fn tone_edge(tone: Tone) -> Color32 {
    let t = tone.color();
    let h = surface::HAIRLINE;
    Color32::from_rgb(
        ((h.r() as u16 * 3 + t.r() as u16) / 4) as u8,
        ((h.g() as u16 * 3 + t.g() as u16) / 4) as u8,
        ((h.b() as u16 * 3 + t.b() as u16) / 4) as u8,
    )
}

/// The gradient tint: a two-color mesh from the tone's tint to transparent,
/// painted over the top band of a rect. Per-vertex color — no shader.
pub fn gradient_tint(painter: &egui::Painter, rect: Rect, tone: Tone) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};

    let band = Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.min.y + (rect.height() * 0.45).min(64.0)),
    );
    let top = tone.tint();
    let bottom = Color32::TRANSPARENT;

    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex { pos: band.left_top(), uv: WHITE_UV, color: top });
    mesh.vertices.push(Vertex { pos: band.right_top(), uv: WHITE_UV, color: top });
    mesh.vertices.push(Vertex { pos: band.left_bottom(), uv: WHITE_UV, color: bottom });
    mesh.vertices.push(Vertex { pos: band.right_bottom(), uv: WHITE_UV, color: bottom });
    mesh.indices.extend_from_slice(&[0, 1, 2, 2, 1, 3]);
    painter.add(egui::Shape::mesh(mesh));
}

// ── FLUID — the clamp core, container-based ─────────────────────────────
//
// The comparable of `clamp(min, calc(base + N·vw + M·svh), max)`, as real
// math. THE UPGRADE over the CSS version: `Fluid::of(rect)` takes ANY rect,
// so a slide sizes to its STAGE — fullscreen on the 75" kiosk, an editor pane
// in Workbench — with identical code and zero ScaledStage/transform hacks.
// 1 rem = 16 egui points; DPI lives below this layer (points already scale).

#[derive(Debug, Clone, Copy)]
pub struct Fluid {
    w_rem: f32,
    h_rem: f32,
}

impl Fluid {
    /// Fluid sizing for content laid out inside `rect`.
    pub fn of(rect: Rect) -> Self {
        Self { w_rem: rect.width() / 16.0, h_rem: rect.height() / 16.0 }
    }

    #[inline]
    fn eval(&self, min: f32, max: f32, slope: f32, slope_y: f32, intercept: f32) -> f32 {
        (intercept + slope * self.w_rem + slope_y * self.h_rem).clamp(min, max) * 16.0
    }
}

include!(concat!(env!("OUT_DIR"), "/fluid_gen.rs"));

/// The motion runtime — the generated effect vocabulary slides pull.
///
/// Authored once in `tokens/motion.json` (eases ported verbatim from the
/// prior theme), generated by `cargo build`, evaluated here. An effect is a
/// PURE function of the clock: `EFFECT.at(now, t0, index)` → 0..1 progress
/// (index applies the authored stagger). Presenter law holds by construction
/// — one-shot effects rest at 1, looped effects wrap, nothing stores state.
pub mod motion {
    /// Cubic-bézier ease, CSS convention: control points (x1, y1, x2, y2).
    /// y may leave [0,1] — that's how the spring eases overshoot.
    #[derive(Debug, Clone, Copy)]
    pub struct Ease(pub f32, pub f32, pub f32, pub f32);

    impl Ease {
        /// y at horizontal progress `p` (0..1). Newton solve for the curve
        /// parameter, then evaluate y — the same math the browser ran.
        pub fn apply(&self, p: f32) -> f32 {
            let p = p.clamp(0.0, 1.0);
            let (x1, y1, x2, y2) = (self.0, self.1, self.2, self.3);
            let bez = |a: f32, b: f32, t: f32| {
                3.0 * (1.0 - t) * (1.0 - t) * t * a + 3.0 * (1.0 - t) * t * t * b + t * t * t
            };
            let mut t = p;
            for _ in 0..8 {
                let x = bez(x1, x2, t) - p;
                // dx/dt
                let d = 3.0 * (1.0 - t) * (1.0 - t) * x1
                    + 6.0 * (1.0 - t) * t * (x2 - x1)
                    + 3.0 * t * t * (1.0 - x2);
                if d.abs() < 1e-6 {
                    break;
                }
                t = (t - x / d).clamp(0.0, 1.0);
            }
            bez(y1, y2, t)
        }
    }

    /// A named motion effect: duration, authored delay, per-index stagger,
    /// ease, and whether it loops (hotspot-pulse) or rests at 1.
    #[derive(Debug, Clone, Copy)]
    pub struct Effect {
        pub dur: f32,
        pub delay: f32,
        pub stagger: f32,
        pub ease: Ease,
        pub looped: bool,
    }

    impl Effect {
        /// Eased progress for item `index` at wall-clock `now`, given the
        /// slide clock's `t0`. One-shot: clamped 0..1, resting at 1.
        /// Looped: wraps forever.
        pub fn at(&self, now: f64, t0: f64, index: usize) -> f32 {
            let elapsed = (now - t0) as f32 - self.delay - self.stagger * index as f32;
            if self.dur <= 0.0 {
                return 1.0;
            }
            let p = elapsed / self.dur;
            if self.looped {
                self.ease.apply(p.max(0.0).fract())
            } else {
                self.ease.apply(p)
            }
        }

        /// True while any of `count` staggered items is still in flight —
        /// the "keep repainting" question every animated chart asks.
        pub fn in_flight(&self, now: f64, t0: f64, count: usize) -> bool {
            if self.looped {
                return true;
            }
            let last = count.saturating_sub(1);
            ((now - t0) as f32) < self.delay + self.stagger * last as f32 + self.dur
        }
    }

    include!(concat!(env!("OUT_DIR"), "/motion_gen.rs"));

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn eases_hit_their_endpoints() {
            for e in [EASE_STANDARD, EASE_DECELERATE, EASE_EMPHASIZED, EASE_SPRING_SOFT] {
                assert!(e.apply(0.0).abs() < 0.01);
                assert!((e.apply(1.0) - 1.0).abs() < 0.01);
            }
        }

        #[test]
        fn springs_overshoot_and_settle() {
            // spring-soft's y control point is 1.56 — mid-curve must exceed 1.
            let peak = (0..=20).map(|i| EASE_SPRING_SOFT.apply(i as f32 / 20.0))
                .fold(0.0f32, f32::max);
            assert!(peak > 1.0, "spring must overshoot, peaked at {peak}");
            assert!((EASE_SPRING_SOFT.apply(1.0) - 1.0).abs() < 0.01);
        }

        #[test]
        fn effects_rest_and_stagger() {
            // Before its stagger slot, item 2 is at 0; long after, everything rests at 1.
            assert_eq!(CHART_RISE.at(0.0, 0.0, 2), 0.0);
            assert!((CHART_RISE.at(100.0, 0.0, 2) - 1.0).abs() < 0.01);
            assert!(!CHART_RISE.in_flight(100.0, 0.0, 5));
            assert!(CHART_RISE.in_flight(0.1, 0.0, 5));
        }

        #[test]
        fn the_vocabulary_is_generated_and_complete() {
            assert_eq!(EFFECT_COUNT, 14, "motion.json ships 14 named effects");
            assert!(effect("enter-rise").is_some(), "the entrance law is authored");
            assert!(effect("chart-rise").is_some());
            assert!(effect("hotspot-pulse").unwrap().looped);
            assert!(effect("not-an-effect").is_none());
        }
    }
}

/// A small tone dot — status, provider chips, active markers.
pub fn tone_dot(ui: &mut egui::Ui, tone: Tone) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.5, tone.color());
}

// ── the sprite-frame vocabulary ─────────────────────────────────────────
//
// The shell's depth is built from exactly three moves, all cheap meshes:
//   sheen()      a top-down gradient tint on a chrome plate
//   diffusion()  black scatter cast off a plate into the well
//   seam()       the crisp 2px sprite bevel — lit line, then a pure-black cut

/// Which edge of a rect is the "dark" edge of a gradient, i.e. where the
/// chrome is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

fn quad(painter: &egui::Painter, rect: Rect, c: [Color32; 4]) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex { pos: rect.left_top(), uv: WHITE_UV, color: c[0] });
    mesh.vertices.push(Vertex { pos: rect.right_top(), uv: WHITE_UV, color: c[1] });
    mesh.vertices.push(Vertex { pos: rect.left_bottom(), uv: WHITE_UV, color: c[2] });
    mesh.vertices.push(Vertex { pos: rect.right_bottom(), uv: WHITE_UV, color: c[3] });
    mesh.indices.extend_from_slice(&[0, 1, 2, 2, 1, 3]);
    painter.add(egui::Shape::mesh(mesh));
}

/// The gradient tint a chrome plate wears: a faint sheen from its top edge,
/// dying out by the midline. Solid color + gradient tint — the sprite look.
pub fn sheen(painter: &egui::Painter, rect: Rect) {
    let lit = Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 7);
    let band = Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.min.y + (rect.height() * 0.5).min(28.0)),
    );
    quad(painter, band, [lit, lit, Color32::TRANSPARENT, Color32::TRANSPARENT]);
}

/// Black diffusion scattered off chrome into the well. `edge` names where the
/// chrome is; the scatter fades away from it.
pub fn diffusion(painter: &egui::Painter, strip: Rect, edge: Edge, max_alpha: u8) {
    let dark = Color32::from_black_alpha(max_alpha);
    let none = Color32::TRANSPARENT;
    let colors = match edge {
        Edge::Top => [dark, dark, none, none],
        Edge::Bottom => [none, none, dark, dark],
        Edge::Left => [dark, none, dark, none],
        Edge::Right => [none, dark, none, dark],
    };
    quad(painter, strip, colors);
}

/// The crisp seam where a chrome plate meets the well: a 1px lit line on the
/// chrome side, then a 1px pure-black cut at the boundary. `edge` names the
/// side of `content` the chrome sits on. `accent` tints the lit line (the
/// topbar seam answers the active hub's tone).
pub fn seam(painter: &egui::Painter, content: Rect, edge: Edge, accent: Option<Tone>) {
    let lit = match accent {
        Some(t) => {
            let c = t.color();
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 64)
        }
        None => Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 12),
    };
    let cut = Color32::from_rgb(0x02, 0x03, 0x05);
    let (lit_rect, cut_rect) = match edge {
        Edge::Top => (
            Rect::from_min_max(egui::pos2(content.min.x, content.min.y - 2.0), egui::pos2(content.max.x, content.min.y - 1.0)),
            Rect::from_min_max(egui::pos2(content.min.x, content.min.y - 1.0), egui::pos2(content.max.x, content.min.y)),
        ),
        Edge::Bottom => (
            Rect::from_min_max(egui::pos2(content.min.x, content.max.y + 1.0), egui::pos2(content.max.x, content.max.y + 2.0)),
            Rect::from_min_max(egui::pos2(content.min.x, content.max.y), egui::pos2(content.max.x, content.max.y + 1.0)),
        ),
        Edge::Left => (
            Rect::from_min_max(egui::pos2(content.min.x - 2.0, content.min.y), egui::pos2(content.min.x - 1.0, content.max.y)),
            Rect::from_min_max(egui::pos2(content.min.x - 1.0, content.min.y), egui::pos2(content.min.x, content.max.y)),
        ),
        Edge::Right => (
            Rect::from_min_max(egui::pos2(content.max.x + 1.0, content.min.y), egui::pos2(content.max.x + 2.0, content.max.y)),
            Rect::from_min_max(egui::pos2(content.max.x, content.min.y), egui::pos2(content.max.x + 1.0, content.max.y)),
        ),
    };
    painter.rect_filled(lit_rect, 0, lit);
    painter.rect_filled(cut_rect, 0, cut);
}

/// The whole frame overlay: seams on all four chrome boundaries + diffusion
/// scattered into the well. Call once per frame, after every panel, with the
/// content rect. `top_accent` is the active hub's tone.
pub fn frame_overlay(painter: &egui::Painter, content: Rect, top_accent: Tone) {
    const DEPTH: f32 = 16.0;
    let c = content;
    diffusion(painter, Rect::from_min_max(c.min, egui::pos2(c.max.x, c.min.y + DEPTH)), Edge::Top, 90);
    diffusion(painter, Rect::from_min_max(egui::pos2(c.min.x, c.max.y - DEPTH * 0.6), c.max), Edge::Bottom, 60);
    diffusion(painter, Rect::from_min_max(c.min, egui::pos2(c.min.x + DEPTH, c.max.y)), Edge::Left, 80);
    diffusion(painter, Rect::from_min_max(egui::pos2(c.max.x - DEPTH * 0.75, c.min.y), c.max), Edge::Right, 70);
    seam(painter, c, Edge::Top, Some(top_accent));
    seam(painter, c, Edge::Left, None);
    seam(painter, c, Edge::Right, None);
    seam(painter, c, Edge::Bottom, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pool_is_the_whole_registry_not_a_seed() {
        // The disease this crate exists to cure: hand-seeding a handful of
        // tones and calling it a system. The vendored registry carries 53.
        assert_eq!(HUE_COUNT, 53, "the generated pool must match hues.json");
        let listed: usize = HUE_GROUPS.iter().map(|(_, ids)| ids.len()).sum();
        assert_eq!(listed, HUE_COUNT);
    }

    #[test]
    fn every_registered_hue_resolves() {
        for (_, ids) in HUE_GROUPS.iter() {
            for id in ids.iter() {
                assert!(ramp(id).is_some(), "hue \"{id}\" is registered but has no ramp");
            }
        }
    }

    #[test]
    fn unknown_hue_degrades_to_none_not_panic() {
        assert!(ramp("definitely-not-a-hue").is_none());
    }

    #[test]
    fn ladders_are_curves_not_flat_ramps() {
        let cyan = ramp("cyan").unwrap();
        let first = cyan.steps[0];
        let last = cyan.steps[12];
        // Light end is light, dark end is dark, and the two differ wildly —
        // a flat ramp or a constant fill would fail all three.
        let sum = |c: Color32| c.r() as u16 + c.g() as u16 + c.b() as u16;
        assert!(sum(first) > 600, "step 25 must be near-white");
        assert!(sum(last) < 200, "step 975 must be deep");
        assert_ne!(first, last);
    }

    #[test]
    fn hub_tones_resolve_through_the_pool() {
        for tone in [Tone::Cyan, Tone::Lime, Tone::Violet, Tone::Amber, Tone::Magenta, Tone::Coral] {
            assert!(ramp(tone.hue_id()).is_some());
            assert_ne!(tone.color(), Color32::GRAY, "tone must not hit the debug fallback");
        }
    }

    #[test]
    fn fluid_hits_min_at_the_small_corner_and_max_at_the_large() {
        // 24rem × 40rem — the small corner of both windows.
        let small = Fluid::of(Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(24.0 * 16.0, 40.0 * 16.0)));
        // 120rem × 100rem — the large corner.
        let large = Fluid::of(Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(120.0 * 16.0, 100.0 * 16.0)));
        // sp-4 is authored 1.0rem … 1.5rem.
        assert!((small.sp_4() - 16.0).abs() < 0.01, "sp_4 at small corner: {}", small.sp_4());
        assert!((large.sp_4() - 24.0).abs() < 0.01, "sp_4 at large corner: {}", large.sp_4());
        // Between the corners it interpolates, inside the clamp.
        let mid = Fluid::of(Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(72.0 * 16.0, 70.0 * 16.0)));
        assert!(mid.sp_4() > small.sp_4() && mid.sp_4() < large.sp_4());
    }

    #[test]
    fn the_full_fluid_scale_generated() {
        assert_eq!(FLUID_TOKEN_COUNT, 36, "space 8 + type 10 + radius 3 + shell 7 + control 8");
    }
}
