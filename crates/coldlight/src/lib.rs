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

/// One vendored font in the pool (tokens/fonts.json → build.rs). The
/// registry the Library fonts wall renders and `apply_theme` registers.
#[derive(Debug, Clone, Copy)]
pub struct FontDef {
    pub id: &'static str,
    pub label: &'static str,
    /// display | retro | ui | mono — what the face is FOR.
    pub class: &'static str,
    pub blurb: &'static str,
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/fonts_gen.rs"));

/// The egui family for a pool font: `FontId::new(size, cl::font_family("orbitron"))`.
pub fn font_family(id: &str) -> egui::FontFamily {
    egui::FontFamily::Name(id.into())
}

/// A LOOK preset — a named bundle of REFS (font, two moods, blend motion,
/// optional ambience) from tokens/presets.json, the 6th source. Applying
/// one is swapping refs on a document; presets are never new code.
#[derive(Debug, Clone, Copy)]
pub struct LookPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub font: &'static str,
    pub moods: (&'static str, &'static str),
    pub motion: &'static str,
    pub ambience: Option<&'static str>,
}

/// A MOTION preset — a named entrance choice over the effect vocabulary.
#[derive(Debug, Clone, Copy)]
pub struct MotionPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub enter: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/presets_gen.rs"));

pub fn look_preset(id: &str) -> Option<&'static LookPreset> {
    LOOK_PRESETS.iter().find(|p| p.id == id)
}

pub fn motion_preset(id: &str) -> Option<&'static MotionPreset> {
    MOTION_PRESETS.iter().find(|p| p.id == id)
}

/// Apply the dark theme once at boot.
pub fn apply_theme(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};

    // The icon font (vendored Phosphor regular, MIT — assets/) rides the
    // default families as a fallback, so any label can carry a glyph from
    // `cl::icon` inline. Vendored bytes + codepoints instead of the
    // egui-phosphor crate: its egui pin would drag a second egui/epaint
    // into the graph.
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "phosphor".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Phosphor.ttf"
        ))),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("phosphor".into());
    }

    // The pool (tokens/fonts.json, vendored OFL): every font registers as
    // its own named family — `cl::font_family(id)` — with the default sans
    // and the icon font as glyph fallback. The default families stay
    // untouched; a chrome default is a design-session decision, not a boot
    // side effect.
    let default_sans = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .and_then(|f| f.first())
        .cloned();
    for def in FONTS {
        fonts.font_data.insert(
            def.id.into(),
            std::sync::Arc::new(egui::FontData::from_static(def.bytes)),
        );
        let mut stack = vec![def.id.to_owned()];
        stack.extend(default_sans.clone());
        stack.push("phosphor".into());
        fonts
            .families
            .insert(egui::FontFamily::Name(def.id.into()), stack);
    }
    ctx.set_fonts(fonts);

    // Dark is the only theme ColdLight ships; pin it and style that slot.
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    // The chrome type scale — raised from egui's small defaults (operator:
    // "the default is way too small"). Content type stays on the fluid
    // `fs-*` ramp; these govern chrome/labels only. Ctrl +/- still zooms
    // the whole app (egui's built-in zoom_factor).
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(26.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(15.5, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(15.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(12.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
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

/// The icon vocabulary — Phosphor glyphs under the OLD lucide inventory's
/// semantic names, so call sites read like the old shell. Codepoints are
/// vendored from egui-phosphor 0.12 (regular variant) alongside the TTF in
/// `assets/` — the crate itself would drag a second egui into the graph.
/// One const per MEANING; adding an icon is one line here and nothing else.
pub mod icon {
    // The rail (old NAV rows).
    pub const HOME: &str = "\u{E2C2}"; // house
    pub const WORKBENCH: &str = "\u{E80E}"; // hammer
    pub const LIBRARY: &str = "\u{E464}"; // squares-four
    pub const WIREDFRONT: &str = "\u{E8C2}"; // film-slate
    pub const BOT: &str = "\u{E762}"; // robot

    // The footer + chrome.
    pub const SNIP: &str = "\u{E10E}"; // camera
    pub const DOCK: &str = "\u{E13C}"; // caret-up
    pub const FULLBLEED: &str = "\u{E0A2}"; // arrows-out
    pub const FULLBLEED_EXIT: &str = "\u{E09A}"; // arrows-in
    pub const CLOSE: &str = "\u{E4F6}"; // x
    pub const BACK: &str = "\u{E138}"; // caret-left

    // The capture surface (old palette tabs + modes + recording).
    pub const IMG: &str = "\u{E10E}"; // camera
    pub const VIDEO: &str = "\u{E4DA}"; // video-camera
    pub const SESSION: &str = "\u{E58C}"; // monitor-play
    pub const STEPS: &str = "\u{EA88}"; // footprints
    pub const REC: &str = "\u{E3EE}"; // record
    pub const STOP: &str = "\u{E45E}"; // square
    pub const MARK: &str = "\u{EA88}"; // footprints
    pub const REGION: &str = "\u{EBB6}"; // scan
    pub const WINDOW: &str = "\u{E5DA}"; // app-window
    pub const FULL: &str = "\u{E32E}"; // monitor
    pub const FREE: &str = "\u{EDC6}"; // lasso
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
    mesh.vertices.push(Vertex {
        pos: band.left_top(),
        uv: WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: band.right_top(),
        uv: WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: band.left_bottom(),
        uv: WHITE_UV,
        color: bottom,
    });
    mesh.vertices.push(Vertex {
        pos: band.right_bottom(),
        uv: WHITE_UV,
        color: bottom,
    });
    mesh.indices.extend_from_slice(&[0, 1, 2, 2, 1, 3]);
    painter.add(egui::Shape::mesh(mesh));
}

// ── MOODS — the backdrop's authored multi-hue treatments ────────────────
// Authored in tokens/moods.json, generated by build.rs. A mood is NEVER one
// color: ≥3 pool-resolved gradient stops + a tinted scrim + an accent. The
// backdrop feature domain consumes this registry; nothing is improvised.

include!(concat!(env!("OUT_DIR"), "/moods_gen.rs"));

impl Mood {
    /// The gradient stops resolved through the pool. Registry-checked at
    /// build time; a missing hue degrades loudly in debug, gray in release.
    pub fn stop_colors(&self) -> Vec<Color32> {
        self.stops
            .iter()
            .map(|(hue, step)| {
                ramp(hue)
                    .and_then(|r| step_index(*step).map(|i| r.steps[i]))
                    .unwrap_or_else(|| {
                        debug_assert!(false, "mood stop unresolvable: {hue}@{step}");
                        Color32::GRAY
                    })
            })
            .collect()
    }

    /// The legibility veil: the FIRST stop's hue at the mood's alpha —
    /// tinted, never neutral black.
    pub fn scrim_color(&self) -> Color32 {
        let base = self.stop_colors()[0];
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), self.scrim)
    }

    pub fn accent_color(&self) -> Color32 {
        ramp(self.accent)
            .map(|r| r.steps[STEP_500 - 1])
            .unwrap_or(Color32::GRAY)
    }

    /// Sample the mood's gradient at `t` ∈ 0..1 (piecewise-linear between
    /// the authored stops, evenly spaced).
    pub fn at(&self, t: f32) -> Color32 {
        let colors = self.stop_colors();
        let n = colors.len() - 1;
        let x = t.clamp(0.0, 1.0) * n as f32;
        let i = (x.floor() as usize).min(n - 1);
        let f = x - i as f32;
        let (a, b) = (colors[i], colors[i + 1]);
        let lerp = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * f).round() as u8;
        Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
    }
}

impl Mood {
    /// Blend two moods' gradient samples: `mix` 0 = all `a`, 1 = all `b`.
    /// Pure — the mood-blend background paints through this, tests pin it.
    pub fn blend_at(a: &Mood, b: &Mood, t: f32, mix: f32) -> Color32 {
        let (ca, cb) = (a.at(t), b.at(t));
        let m = mix.clamp(0.0, 1.0);
        let lerp = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * m).round() as u8;
        Color32::from_rgb(
            lerp(ca.r(), cb.r()),
            lerp(ca.g(), cb.g()),
            lerp(ca.b(), cb.b()),
        )
    }
}

/// Paint TWO moods blended across `rect` — one mesh, the blend happens in
/// the per-vertex SAMPLE (never stacked translucent layers). `mix`
/// crossfades a→b; `pan` shifts the diagonal phase (±, `at` clamps — no
/// wrap seam). The document mood-blend background drives both from the
/// authored `blend-drift` clock.
pub fn mood_blend(painter: &egui::Painter, rect: Rect, a: &Mood, b: &Mood, mix: f32, pan: f32) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    const COLS: usize = 24;
    const ROWS: usize = 14;

    let denom = (rect.width() + rect.height()).max(1.0);
    let mut mesh = Mesh::default();
    for r in 0..=ROWS {
        for c in 0..=COLS {
            let x = rect.min.x + rect.width() * c as f32 / COLS as f32;
            let y = rect.min.y + rect.height() * r as f32 / ROWS as f32;
            let t = ((x - rect.min.x) + (y - rect.min.y)) / denom + pan;
            mesh.vertices.push(Vertex {
                pos: egui::pos2(x, y),
                uv: WHITE_UV,
                color: Mood::blend_at(a, b, t, mix),
            });
        }
    }
    let w = COLS as u32 + 1;
    for r in 0..ROWS as u32 {
        for c in 0..COLS as u32 {
            let i = r * w + c;
            mesh.indices
                .extend_from_slice(&[i, i + 1, i + w, i + w, i + 1, i + w + 1]);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Paint a mood's 135° gradient across `rect` — same grid technique as the
/// aurora, colors from the mood's pool-resolved stops.
pub fn mood_gradient(painter: &egui::Painter, rect: Rect, mood: &Mood) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    const COLS: usize = 24;
    const ROWS: usize = 14;

    let denom = (rect.width() + rect.height()).max(1.0);
    let mut mesh = Mesh::default();
    for r in 0..=ROWS {
        for c in 0..=COLS {
            let x = rect.min.x + rect.width() * c as f32 / COLS as f32;
            let y = rect.min.y + rect.height() * r as f32 / ROWS as f32;
            let t = ((x - rect.min.x) + (y - rect.min.y)) / denom;
            mesh.vertices.push(Vertex {
                pos: egui::pos2(x, y),
                uv: WHITE_UV,
                color: mood.at(t),
            });
        }
    }
    let w = COLS as u32 + 1;
    for r in 0..ROWS as u32 {
        for c in 0..COLS as u32 {
            let i = r * w + c;
            mesh.indices
                .extend_from_slice(&[i, i + 1, i + w, i + w, i + 1, i + w + 1]);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

// ── AURORA — the mainpage base layer, ported verbatim ───────────────────
// Generated stops (oklch-interpolated 135° teal gradient) + the scrim the
// old BackdropStage layered over the loop. See build.rs::generate_aurora.

include!(concat!(env!("OUT_DIR"), "/aurora_gen.rs"));

/// Sample the aurora ramp at `t` ∈ 0..1 (linear between generated samples).
pub fn aurora_at(t: f32) -> Color32 {
    let n = AURORA.len() - 1;
    let x = t.clamp(0.0, 1.0) * n as f32;
    // Clamp the INDEX, not the float: `n as f32 - EPSILON` rounds back to n
    // at this magnitude and t=1.0 would index past the end.
    let i = (x.floor() as usize).min(n - 1);
    let f = x - i as f32;
    let (a, b) = (AURORA[i], AURORA[i + 1]);
    let lerp = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * f).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

/// The scrim color at a given alpha (old: 70% over media, 35% bare).
pub fn scrim(alpha: u8) -> Color32 {
    let (r, g, b) = SCRIM_RGB;
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}
pub const SCRIM_MEDIA_A: u8 = 179; // 70%
pub const SCRIM_BARE_A: u8 = 89; // 35%

/// Paint the 135° oklch aurora across `rect` — a per-vertex-tinted grid
/// (no shader). CSS 135° runs top-left → bottom-right; the projection is
/// the CSS gradient-line formula reduced for the diagonal case.
pub fn aurora(painter: &egui::Painter, rect: Rect) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    const COLS: usize = 24;
    const ROWS: usize = 14;

    let denom = (rect.width() + rect.height()).max(1.0);
    let mut mesh = Mesh::default();
    for r in 0..=ROWS {
        for c in 0..=COLS {
            let x = rect.min.x + rect.width() * c as f32 / COLS as f32;
            let y = rect.min.y + rect.height() * r as f32 / ROWS as f32;
            let t = ((x - rect.min.x) + (y - rect.min.y)) / denom;
            mesh.vertices.push(Vertex {
                pos: egui::pos2(x, y),
                uv: WHITE_UV,
                color: aurora_at(t),
            });
        }
    }
    let w = COLS as u32 + 1;
    for r in 0..ROWS as u32 {
        for c in 0..COLS as u32 {
            let i = r * w + c;
            mesh.indices
                .extend_from_slice(&[i, i + 1, i + w, i + w, i + 1, i + w + 1]);
        }
    }
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
        Self {
            w_rem: rect.width() / 16.0,
            h_rem: rect.height() / 16.0,
        }
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
            for e in [
                EASE_STANDARD,
                EASE_DECELERATE,
                EASE_EMPHASIZED,
                EASE_SPRING_SOFT,
            ] {
                assert!(e.apply(0.0).abs() < 0.01);
                assert!((e.apply(1.0) - 1.0).abs() < 0.01);
            }
        }

        #[test]
        fn springs_overshoot_and_settle() {
            // spring-soft's y control point is 1.56 — mid-curve must exceed 1.
            let peak = (0..=20)
                .map(|i| EASE_SPRING_SOFT.apply(i as f32 / 20.0))
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
            assert_eq!(EFFECT_COUNT, 18, "motion.json ships 18 named effects");
            assert!(
                effect("enter-rise").is_some(),
                "the entrance law is authored"
            );
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
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: WHITE_UV,
        color: c[0],
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: WHITE_UV,
        color: c[1],
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: WHITE_UV,
        color: c[2],
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: WHITE_UV,
        color: c[3],
    });
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
    quad(
        painter,
        band,
        [lit, lit, Color32::TRANSPARENT, Color32::TRANSPARENT],
    );
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
            Rect::from_min_max(
                egui::pos2(content.min.x, content.min.y - 2.0),
                egui::pos2(content.max.x, content.min.y - 1.0),
            ),
            Rect::from_min_max(
                egui::pos2(content.min.x, content.min.y - 1.0),
                egui::pos2(content.max.x, content.min.y),
            ),
        ),
        Edge::Bottom => (
            Rect::from_min_max(
                egui::pos2(content.min.x, content.max.y + 1.0),
                egui::pos2(content.max.x, content.max.y + 2.0),
            ),
            Rect::from_min_max(
                egui::pos2(content.min.x, content.max.y),
                egui::pos2(content.max.x, content.max.y + 1.0),
            ),
        ),
        Edge::Left => (
            Rect::from_min_max(
                egui::pos2(content.min.x - 2.0, content.min.y),
                egui::pos2(content.min.x - 1.0, content.max.y),
            ),
            Rect::from_min_max(
                egui::pos2(content.min.x - 1.0, content.min.y),
                egui::pos2(content.min.x, content.max.y),
            ),
        ),
        Edge::Right => (
            Rect::from_min_max(
                egui::pos2(content.max.x + 1.0, content.min.y),
                egui::pos2(content.max.x + 2.0, content.max.y),
            ),
            Rect::from_min_max(
                egui::pos2(content.max.x, content.min.y),
                egui::pos2(content.max.x + 1.0, content.max.y),
            ),
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
    diffusion(
        painter,
        Rect::from_min_max(c.min, egui::pos2(c.max.x, c.min.y + DEPTH)),
        Edge::Top,
        90,
    );
    diffusion(
        painter,
        Rect::from_min_max(egui::pos2(c.min.x, c.max.y - DEPTH * 0.6), c.max),
        Edge::Bottom,
        60,
    );
    diffusion(
        painter,
        Rect::from_min_max(c.min, egui::pos2(c.min.x + DEPTH, c.max.y)),
        Edge::Left,
        80,
    );
    diffusion(
        painter,
        Rect::from_min_max(egui::pos2(c.max.x - DEPTH * 0.75, c.min.y), c.max),
        Edge::Right,
        70,
    );
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
                assert!(
                    ramp(id).is_some(),
                    "hue \"{id}\" is registered but has no ramp"
                );
            }
        }
    }

    #[test]
    fn unknown_hue_degrades_to_none_not_panic() {
        assert!(ramp("definitely-not-a-hue").is_none());
    }

    #[test]
    fn every_mood_is_authored_multi_hue_and_resolvable() {
        assert_eq!(MOODS.len(), MOOD_COUNT);
        assert!(
            MOODS.len() >= 10,
            "the registry ships at least the 10 authored moods"
        );
        for m in MOODS.iter() {
            assert!(
                m.stops.len() >= 3,
                "mood \"{}\": a mood is never one color",
                m.name
            );
            // Multi-HUE, not one hue at 3 steps: at least 2 distinct hue ids.
            let mut hues: Vec<&str> = m.stops.iter().map(|(h, _)| *h).collect();
            hues.dedup();
            assert!(hues.len() >= 2, "mood \"{}\" uses a single hue", m.name);
            for c in m.stop_colors() {
                assert_ne!(
                    c,
                    Color32::GRAY,
                    "mood \"{}\" has an unresolvable stop",
                    m.name
                );
            }
            assert_ne!(
                m.accent_color(),
                Color32::GRAY,
                "mood \"{}\" accent",
                m.name
            );
            assert!(m.scrim_color().a() > 0);
        }
    }

    #[test]
    fn mood_sampling_hits_the_authored_endpoints() {
        let m = &MOODS[0];
        let colors = m.stop_colors();
        assert_eq!(m.at(0.0), colors[0]);
        assert_eq!(m.at(1.0), colors[colors.len() - 1]);
        assert_eq!(m.at(9.9), colors[colors.len() - 1]); // clamped, in bounds
    }

    #[test]
    fn aurora_endpoints_are_exact_and_in_bounds() {
        // Regression: t=1.0 must land on the LAST sample, not index past it
        // (f32::EPSILON is invisible at magnitude 32 — the index is clamped).
        assert_eq!(aurora_at(0.0), AURORA[0]);
        assert_eq!(aurora_at(1.0), AURORA[AURORA.len() - 1]);
        assert_eq!(aurora_at(2.0), AURORA[AURORA.len() - 1]); // clamped
                                                              // The ramp is teal: green/blue dominate red at every sample.
        for c in AURORA.iter() {
            assert!(c.g() > c.r(), "aurora must stay teal: {c:?}");
        }
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
        for tone in [
            Tone::Cyan,
            Tone::Lime,
            Tone::Violet,
            Tone::Amber,
            Tone::Magenta,
            Tone::Coral,
        ] {
            assert!(ramp(tone.hue_id()).is_some());
            assert_ne!(
                tone.color(),
                Color32::GRAY,
                "tone must not hit the debug fallback"
            );
        }
    }

    #[test]
    fn fluid_hits_min_at_the_small_corner_and_max_at_the_large() {
        // 24rem × 40rem — the small corner of both windows.
        let small = Fluid::of(Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(24.0 * 16.0, 40.0 * 16.0),
        ));
        // 120rem × 100rem — the large corner.
        let large = Fluid::of(Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(120.0 * 16.0, 100.0 * 16.0),
        ));
        // sp-4 is authored 1.0rem … 1.5rem.
        assert!(
            (small.sp_4() - 16.0).abs() < 0.01,
            "sp_4 at small corner: {}",
            small.sp_4()
        );
        assert!(
            (large.sp_4() - 24.0).abs() < 0.01,
            "sp_4 at large corner: {}",
            large.sp_4()
        );
        // Between the corners it interpolates, inside the clamp.
        let mid = Fluid::of(Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(72.0 * 16.0, 70.0 * 16.0),
        ));
        assert!(mid.sp_4() > small.sp_4() && mid.sp_4() < large.sp_4());
    }

    #[test]
    fn the_full_fluid_scale_generated() {
        assert_eq!(
            FLUID_TOKEN_COUNT, 36,
            "space 8 + type 10 + radius 3 + shell 7 + control 8"
        );
    }

    #[test]
    fn mood_blend_endpoints_are_the_single_moods_and_the_middle_is_between() {
        let (a, b) = (&MOODS[0], &MOODS[1]);
        for t in [0.0f32, 0.33, 0.7, 1.0] {
            assert_eq!(Mood::blend_at(a, b, t, 0.0), a.at(t));
            assert_eq!(Mood::blend_at(a, b, t, 1.0), b.at(t));
            let mid = Mood::blend_at(a, b, t, 0.5);
            let between = |m: u8, p: u8, q: u8| {
                let (lo, hi) = (p.min(q), p.max(q));
                m >= lo.saturating_sub(1) && m <= hi.saturating_add(1)
            };
            assert!(between(mid.r(), a.at(t).r(), b.at(t).r()));
            assert!(between(mid.g(), a.at(t).g(), b.at(t).g()));
            assert!(between(mid.b(), a.at(t).b(), b.at(t).b()));
        }
    }

    #[test]
    fn preset_refs_all_resolve_through_the_live_registries() {
        assert_eq!(LOOK_PRESETS.len(), LOOK_PRESET_COUNT);
        assert_eq!(MOTION_PRESETS.len(), MOTION_PRESET_COUNT);
        let mut ids: Vec<&str> = LOOK_PRESETS.iter().map(|p| p.id).collect();
        ids.extend(MOTION_PRESETS.iter().map(|p| p.id));
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate preset id");
        for p in LOOK_PRESETS {
            assert!(FONTS.iter().any(|f| f.id == p.font), "{}: font", p.id);
            for m in [p.moods.0, p.moods.1] {
                assert!(MOODS.iter().any(|x| x.name == m), "{}: mood {m}", p.id);
            }
            assert!(motion::effect(p.motion).is_some(), "{}: motion", p.id);
            if let Some(a) = p.ambience {
                assert!(
                    matches!(a, "pulse" | "drift" | "rings"),
                    "{}: ambience",
                    p.id
                );
            }
        }
        for p in MOTION_PRESETS {
            assert!(motion::effect(p.enter).is_some(), "{}: enter", p.id);
        }
        assert!(look_preset("neon-intro").is_some());
        assert!(look_preset("ghost").is_none());
    }

    #[test]
    fn font_pool_ids_are_unique_and_classed() {
        assert_eq!(FONTS.len(), FONT_COUNT);
        for (i, a) in FONTS.iter().enumerate() {
            assert!(matches!(a.class, "display" | "retro" | "ui" | "mono"));
            for b in &FONTS[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate font id");
            }
        }
    }

    /// Every vendored TTF must actually PARSE and shape a glyph — a corrupt
    /// or unsupported file fails here in the gate, not at app boot.
    #[test]
    fn every_pool_font_lays_out_through_a_real_context() {
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for def in FONTS {
                    ui.label(
                        egui::RichText::new(format!("{} Ag01", def.label))
                            .font(egui::FontId::new(16.0, font_family(def.id))),
                    );
                }
            });
        });
    }
}
