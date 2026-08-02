//! The PROVIDERS control (V1) — the operator's way into the frontier AI on
//! this machine, and the origin of the ink-bleed preset.
//!
//! > "I click on what will be a neat optimized bleed effect that as the mouse
//! > just overs ever so slowly like ink to paper think of it as a passive
//! > experience not meant to be instant but literally over the coarse of 5.6
//! > to 6.3 seconds before the full color takes over the icon and icon is
//! > brain and Providers and just equal at the height of P to the left we
//! > have a small circle with ai inside."
//!
//! Every design value here is AUTHORED and generated — the dwell window and
//! the easing curve from `tokens/motion.json`, the hue pool from
//! `tokens/presets.json` cross-checked against `tokens/hues.json`, the brain
//! from `tokens/icons.json` over the vendored Lucide pool. Nothing in this
//! file is a magic number that a designer cannot reach.
//!
//! What it does NOT do is open anything: the popup is V2, and the click
//! lands with the surface it opens.

use dpl_coldlight::{self as cl, BleedPreset};
use egui::{Color32, Rect, Response, Sense, Ui};

/// The authored bleed this control is the origin of.
pub const BLEED: &str = "providers-ink";

/// The seeded state a control keeps for its lifetime: which pair of hues it
/// drew, and how long its dwell runs.
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
    /// Draw the pair and the dwell from `seed`.
    ///
    /// Deterministic in the seed, so `--mood <n>` pins the whole look of the
    /// bleed for a capture the same way it already pins the backdrop.
    pub fn from_seed(seed: u64) -> Self {
        let preset: &BleedPreset =
            cl::bleed_preset(BLEED).expect("the providers bleed is authored");
        let (a, b) = preset.pair(seed);
        // The jitter rides inside the AUTHORED window rather than a literal.
        let span = cl::motion::DUR_DWELL_MAX - cl::motion::DUR_DWELL_MIN;
        let frac = ((seed >> 16) % 1000) as f32 / 1000.0;
        Self {
            a,
            b,
            dwell: cl::motion::DUR_DWELL_MIN + span * frac,
        }
    }
}

/// What the control paints, once the shell has read the roster.
pub struct ProvidersControl<'a> {
    /// `Codex, Claude Code` — the activated providers, named.
    pub names: &'a str,
    /// Each provider's truthful limit, one per line.
    pub tooltip: &'a str,
    pub roll: BleedRoll,
}

type Galley2 = std::sync::Arc<egui::Galley>;

/// The width the control occupies: the `ai` circle, the brain, and the word.
fn measure(ui: &Ui) -> (f32, Galley2) {
    let galley = ui.painter().layout_no_wrap(
        "Providers".to_owned(),
        egui::FontId::proportional(LABEL_SIZE),
        cl::ink::MUTED,
    );
    (
        PAD_L + AI_D + GAP + BRAIN + GAP + galley.size().x + PAD_R,
        galley,
    )
}

const LABEL_SIZE: f32 = 15.0;
/// The `ai` circle's diameter IS the cap-height of the P — the operator's
/// alignment, expressed as the ratio rather than a second magic number.
const AI_D: f32 = LABEL_SIZE * 0.72;
const BRAIN: f32 = 22.0;
const GAP: f32 = 8.0;
const PAD_L: f32 = 9.0;
const PAD_R: f32 = 12.0;
const H: f32 = 26.0;

impl ProvidersControl<'_> {
    /// Paint the control and return its response. `t` — the bleed factor —
    /// is driven by hover over the seeded dwell.
    pub fn show(&self, ui: &mut Ui) -> Response {
        let (w, galley) = measure(ui);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, H), Sense::click());

        // THE BLEED. Hover drives one factor over the seeded dwell; every
        // colour below is a lerp along it, so the wash, the glyph and the
        // word all arrive together rather than as three separate animations.
        // A STABLE id, not one derived from `ui.id()`. The status bar builds
        // its lane as a fresh child Ui every frame, so a ui-derived id is a
        // new id each frame — egui treats an unseen id as already-arrived and
        // snaps straight to the target, which turns a six-second bleed into
        // an instant swap. The rail's unfold uses a fixed `Id::new` for the
        // same reason.
        let t = ui.ctx().animate_bool_with_time(
            egui::Id::new("dpl_providers_bleed"),
            resp.hovered(),
            self.roll.dwell,
        );
        // The wash only advances on frames that actually run, so drive them
        // for as long as it is mid-bleed. Without this the bleed stalls
        // whenever nothing else asks for a repaint.
        if t > 0.0 && t < 1.0 {
            ui.ctx().request_repaint();
        }

        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(cl::RADIUS_SM),
            cl::surface::RAISED,
        );

        if t > 0.0 {
            // Two origins, seeping outward — SCALE, not opacity, so it reads
            // as ink spreading into paper rather than a panel fading up.
            wash(painter, rect, self.roll.a, t, -0.18, -0.35);
            wash(painter, rect, self.roll.b, t, 0.62, 0.40);
        }

        let ink = lerp_color(cl::ink::SUBTLE, self.roll.a, t);
        let mut x = rect.min.x + PAD_L;

        // the `ai` circle, cap-height of the P, to its LEFT
        let c = egui::pos2(x + AI_D / 2.0, rect.center().y);
        painter.circle_stroke(c, AI_D / 2.0, egui::Stroke::new(1.4, ink));
        painter.text(
            c,
            egui::Align2::CENTER_CENTER,
            "ai",
            egui::FontId::proportional(AI_D * 0.52),
            ink,
        );
        x += AI_D + GAP;

        // the brain, line-art washing to full colour
        let brain = Rect::from_min_size(
            egui::pos2(x, rect.center().y - BRAIN / 2.0),
            egui::vec2(BRAIN, BRAIN),
        );
        cl::draw_icon(painter, "brain", brain, ink);
        x += BRAIN + GAP;

        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            lerp_color(cl::ink::MUTED, cl::ink::STRONG, t),
        );

        resp.on_hover_text(self.tooltip)
    }
}

/// One origin of the bleed: a soft radial wash whose RADIUS grows with `t`.
///
/// egui has no blur, so the softness is built the way the rest of the shell
/// builds glow — concentric rings at falling alpha. `ox`/`oy` place the
/// origin relative to the control, matching the mock's two corners.
fn wash(painter: &egui::Painter, rect: Rect, hue: Color32, t: f32, ox: f32, oy: f32) {
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

fn lerp_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgb(
        m(from.r(), to.r()),
        m(from.g(), to.g()),
        m(from.b(), to.b()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
                (cl::motion::DUR_DWELL_MIN..=cl::motion::DUR_DWELL_MAX).contains(&r.dwell),
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
}
