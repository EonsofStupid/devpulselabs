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
    /// Every tone. ONE list, so a new tone cannot be half-added — the census
    /// law applied to the smallest registry in the crate.
    pub const ALL: [Tone; 6] = [
        Tone::Cyan,
        Tone::Lime,
        Tone::Violet,
        Tone::Amber,
        Tone::Magenta,
        Tone::Coral,
    ];

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
    /// The stream-annotation stroke (D3) — the old `--ink-annotate`
    /// fallback's warm near-white, visible over every mood.
    pub const ANNOTATE: Color32 = Color32::from_rgb(0xf8, 0xf1, 0xdd);
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
/// Clay surfaces breathe — the generous radius of the soft plate.
pub const RADIUS_LG: u8 = 16;

// ── the DESIGN LANGUAGE primitives (docs/design/doctrine.md) ────────────
//
// The treatments are the composable TRAIT system below (neubrutalist/
// claymorphic base + cyber/retro stack, resolved by `compose`). Surfaces
// paint via `cl::treated(...)` / the `brutal`/`clay` shortcuts — never a
// bespoke frame. `tip` stays here as the tooltip-law primitive.

/// The tooltip LAW: every interactive control explains itself on hover.
/// One call at every control site — a control without a `tip` is a
/// doctrine violation, not a style choice.
pub fn tip(resp: egui::Response, text: &str) -> egui::Response {
    resp.on_hover_text(text)
}

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

/// A registry icon: Lucide geometry tessellated at BUILD time from the
/// vendored pool (tokens/icons.json → build.rs, ADR-015). The mesh is
/// single-color and unopinionated — styling (ink, size, layers) composes
/// at paint time, the visx law.
#[derive(Debug, Clone, Copy)]
pub struct IconDef {
    pub id: &'static str,
    pub label: &'static str,
    /// The vendored source file: assets/lucide/icons/{lucide}.svg.
    pub lucide: &'static str,
    pub blurb: &'static str,
    /// x, y, w, h of the source viewBox (Lucide: 0 0 24 24).
    pub viewbox: (f32, f32, f32, f32),
    pub vertices: &'static [(f32, f32)],
    pub indices: &'static [u32],
}

include!(concat!(env!("OUT_DIR"), "/icons_gen.rs"));

impl IconDef {
    /// The icon as a tinted mesh mapped into `rect` — uniform scale,
    /// centered; icons never stretch. Built for OPAQUE tints: the stroke
    /// tessellation double-covers at caps and joins, so a translucent
    /// color would reveal seams — paint washes as separate layers.
    pub fn mesh(&self, rect: egui::Rect, color: Color32) -> egui::Mesh {
        let (vx, vy, vw, vh) = self.viewbox;
        let scale = (rect.width() / vw).min(rect.height() / vh);
        let ox = rect.center().x - (vx + vw * 0.5) * scale;
        let oy = rect.center().y - (vy + vh * 0.5) * scale;
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(self.vertices.len());
        for &(x, y) in self.vertices {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(x * scale + ox, y * scale + oy),
                uv: egui::epaint::WHITE_UV,
                color,
            });
        }
        mesh.indices.extend_from_slice(self.indices);
        mesh
    }
}

/// Paint a registry icon into `rect`. An unknown id paints an honest
/// hairline box — a missing icon must be SEEN, never silently skipped.
pub fn draw_icon(painter: &egui::Painter, id: &str, rect: egui::Rect, color: Color32) {
    match icon_def(id) {
        Some(def) => {
            painter.add(egui::Shape::mesh(def.mesh(rect, color)));
        }
        None => {
            painter.rect_stroke(
                rect.shrink(2.0),
                CornerRadius::same(2),
                Stroke::new(1.0, color),
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// The brutal rendition: the icon's own mesh re-drawn offset as a hard
/// shadow SHAPE under the tinted pass — doctrine's neubrutalist pop,
/// never a blur. The offset tracks icon size and lands on the trait
/// system's [3, 3] at chrome sizes.
pub fn draw_icon_brutal(
    painter: &egui::Painter,
    id: &str,
    rect: egui::Rect,
    color: Color32,
    shadow: Color32,
) {
    let off = (rect.width().min(rect.height()) / 8.0).max(2.0);
    draw_icon(painter, id, rect.translate(egui::vec2(off, off)), shadow);
    draw_icon(painter, id, rect, color);
}

/// An inline icon control: allocates `size`², paints the registry icon,
/// answers hover/click. Callers still owe the tooltip law (`cl::tip`).
pub fn icon_label(ui: &mut egui::Ui, id: &str, size: f32, color: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        draw_icon(ui.painter(), id, rect, color);
    }
    resp
}

/// A LOOK preset — a named bundle of REFS (font, two moods, blend motion,
/// optional ambience) from tokens/presets.json, the 6th source. Applying
/// one is swapping refs on a document; presets are never new code.
#[derive(Debug, Clone, Copy)]
pub struct ThemePreset {
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

/// A two-colour hover wash: an effect plus the POOL its pair is drawn from.
///
/// Its own kind rather than a wider [`MotionPreset`], which carries one
/// `enter` effect and no colour. The pool is authored in
/// `tokens/presets.json` and every hue id in it is checked against
/// `tokens/hues.json` at BUILD time — the reason a bleed can be reused
/// globally without any widget holding a list of hexes.
#[derive(Debug, Clone, Copy)]
pub struct BleedPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub effect: &'static str,
    pub hues: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/presets_gen.rs"));

pub fn theme_preset(id: &str) -> Option<&'static ThemePreset> {
    THEME_PRESETS.iter().find(|p| p.id == id)
}

pub fn motion_preset(id: &str) -> Option<&'static MotionPreset> {
    MOTION_PRESETS.iter().find(|p| p.id == id)
}

pub fn bleed_preset(id: &str) -> Option<&'static BleedPreset> {
    BLEED_PRESETS.iter().find(|p| p.id == id)
}

/// The ladder step a bleed wash is drawn at — vivid enough to read as ink
/// on the dark surface, short of the near-white top of the ladder.
pub const BLEED_STEP: u32 = 400;

impl BleedPreset {
    /// Two DISTINCT hues from the pool, chosen by `seed`.
    ///
    /// The same xorshift discipline `dpl_backdrop::random_mood_index` uses —
    /// no RNG dependency, and deterministic in the seed so a proof shot
    /// reproduces. The second pick walks a coprime-ish stride from the first
    /// so the pair is never the same hue twice.
    pub fn pair(&self, seed: u64) -> (Color32, Color32) {
        let (a, b) = self.pair_ids(seed);
        (step_on(a, BLEED_STEP), step_on(b, BLEED_STEP))
    }

    /// The chosen hue IDS — what a test can assert on without comparing
    /// colours.
    pub fn pair_ids(&self, seed: u64) -> (&'static str, &'static str) {
        let n = self.hues.len();
        debug_assert!(n >= 2, "a two-colour bleed needs a pool of at least 2");
        let mut x = seed | 1;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let x = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let first = (x % n as u64) as usize;
        // 1..n keeps the offset off zero, so the second is never the first.
        let step = 1 + ((x >> 32) % (n as u64 - 1)) as usize;
        (self.hues[first], self.hues[(first + step) % n])
    }
}

// ── THE TRAIT SYSTEM (docs/design/doctrine.md) ──────────────────────────
//
// A trait is a STACKABLE partial theme, authored in tokens/traits.json,
// generated to traits_gen.rs, and resolved per-axis against a runtime Tone
// by `compose`. Additive axes (shadows/back_glow/tints/anim) concatenate
// across the stack; override axes (fill/edge/radius/ink/font) last-wins.
// This is the original ColdLight trait compositor, ported: one pure fn →
// a concrete `ResolvedStyle`, which the paint layer consumes.

/// The base of a trait color ref, resolved at compose-time.
#[derive(Debug, Clone, Copy)]
pub enum ColorBase {
    /// The surface Tone's ladder at this step (e.g. 400 fill, 700 edge).
    ToneStep(u32),
    /// A fixed pool hue at a step (cyber's cyan, retro's amber).
    Hue(&'static str, u32),
    /// A fixed color (a surface/ink slot, or black for shadows).
    Fixed(Color32),
}

/// A trait color ref: a ladder-relative or fixed base, plus an alpha.
#[derive(Debug, Clone, Copy)]
pub struct ColorRef {
    pub base: ColorBase,
    pub alpha: u8,
}

/// The pool color at `hue`@`step`, or a loud debug fallback.
fn step_on(hue: &str, step: u32) -> Color32 {
    match (ramp(hue), step_index(step)) {
        (Some(r), Some(i)) => r.steps[i],
        _ => {
            debug_assert!(false, "trait ref missing from pool: {hue}@{step}");
            Color32::GRAY
        }
    }
}

impl ColorRef {
    /// Resolve against the surface's tone and the generated pool.
    pub fn resolve(self, tone: Tone) -> Color32 {
        let base = match self.base {
            ColorBase::ToneStep(step) => step_on(tone.hue_id(), step),
            ColorBase::Hue(id, step) => step_on(id, step),
            ColorBase::Fixed(c) => c,
        };
        if self.alpha == 255 {
            base
        } else {
            Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), self.alpha)
        }
    }
}

/// A tone edge: a width + a color ref (override axis).
#[derive(Debug, Clone, Copy)]
pub struct EdgeSpec {
    pub width: f32,
    pub color: ColorRef,
}

/// A hard/soft shadow spec (additive axis) — egui geometry + a color ref.
#[derive(Debug, Clone, Copy)]
pub struct ShadowSpec {
    pub offset: [i8; 2],
    pub blur: u8,
    pub spread: u8,
    pub color: ColorRef,
}

/// A back-glow spec (additive) — the cheap painted "lit-from-behind" halo.
#[derive(Debug, Clone, Copy)]
pub struct GlowSpec {
    pub color: ColorRef,
    pub spread: f32,
}

/// A tint/wash layer (additive) — a directional gradient overlay.
#[derive(Debug, Clone, Copy)]
pub struct TintSpec {
    pub color: ColorRef,
    pub dir: f32,
}

/// The per-axis map of a trait (or a state overlay). Every field is Option:
/// `None` means "this trait does not touch this axis".
#[derive(Debug, Clone, Copy)]
pub struct Axes {
    pub fill: Option<ColorRef>,
    pub edge: Option<EdgeSpec>,
    pub radius: Option<u8>,
    pub ink: Option<ColorRef>,
    pub font: Option<&'static str>,
    pub shadows: Option<&'static [ShadowSpec]>,
    pub back_glow: Option<&'static [GlowSpec]>,
    pub tints: Option<&'static [TintSpec]>,
    pub anim: Option<&'static [&'static str]>,
}

/// A stackable trait: its base axes + optional hover/active state overlays.
#[derive(Debug, Clone, Copy)]
pub struct TraitDef {
    pub id: &'static str,
    /// base = neubrutalist/claymorphic (one per surface); stack = cyber/retro.
    pub role: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub base: Axes,
    pub hover: Option<Axes>,
    pub active: Option<Axes>,
}

include!(concat!(env!("OUT_DIR"), "/traits_gen.rs"));

/// The interaction state a surface is resolved in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitState {
    Idle,
    Hover,
    Active,
}

/// A resolved back-glow (color already carries alpha; painter draws behind).
#[derive(Debug, Clone, Copy)]
pub struct Glow {
    pub color: Color32,
    pub spread: f32,
}

/// A resolved tint layer.
#[derive(Debug, Clone, Copy)]
pub struct Tint {
    pub color: Color32,
    pub dir: f32,
}

/// The concrete style a paint fn consumes — the compositor's output. Pure
/// input → pure output, so it is cacheable by (tone, base, stack, state).
#[derive(Debug, Clone, Default)]
pub struct ResolvedStyle {
    pub fill: Color32,
    pub edge: Stroke,
    pub radius: u8,
    pub ink: Color32,
    pub font: Option<&'static str>,
    pub shadows: Vec<Shadow>,
    pub back_glow: Vec<Glow>,
    pub tints: Vec<Tint>,
    pub anim: Vec<&'static str>,
}

/// A trait by id, or None (unknown → degrade, never panic).
pub fn trait_def(id: &str) -> Option<&'static TraitDef> {
    TRAITS.iter().find(|t| t.id == id)
}

/// A trait's axes for a state: the base overlaid by the state patch, where
/// the patch REPLACES an axis it names (additive axes included) — so a
/// hover shadow replaces the idle shadow rather than stacking on itself.
fn merged(def: &TraitDef, state: TraitState) -> Axes {
    let patch = match state {
        TraitState::Idle => None,
        TraitState::Hover => def.hover,
        TraitState::Active => def.active,
    };
    match patch {
        None => def.base,
        Some(p) => Axes {
            fill: p.fill.or(def.base.fill),
            edge: p.edge.or(def.base.edge),
            radius: p.radius.or(def.base.radius),
            ink: p.ink.or(def.base.ink),
            font: p.font.or(def.base.font),
            shadows: p.shadows.or(def.base.shadows),
            back_glow: p.back_glow.or(def.base.back_glow),
            tints: p.tints.or(def.base.tints),
            anim: p.anim.or(def.base.anim),
        },
    }
}

/// Compose a base trait + a stack of traits into a `ResolvedStyle`, resolved
/// against `tone` and `state`. Override axes take the last trait that names
/// them; additive axes concatenate every trait's contribution. Pure.
pub fn compose(tone: Tone, base: &str, stack: &[&str], state: TraitState) -> ResolvedStyle {
    let mut rs = ResolvedStyle::default();
    for id in std::iter::once(base).chain(stack.iter().copied()) {
        let Some(def) = trait_def(id) else {
            debug_assert!(false, "unknown trait in stack: {id}");
            continue;
        };
        let ax = merged(def, state);
        if let Some(c) = ax.fill {
            rs.fill = c.resolve(tone);
        }
        if let Some(e) = ax.edge {
            rs.edge = Stroke::new(e.width, e.color.resolve(tone));
        }
        if let Some(r) = ax.radius {
            rs.radius = r;
        }
        if let Some(c) = ax.ink {
            rs.ink = c.resolve(tone);
        }
        if let Some(f) = ax.font {
            rs.font = Some(f);
        }
        if let Some(shadows) = ax.shadows {
            rs.shadows.extend(shadows.iter().map(|s| Shadow {
                offset: s.offset,
                blur: s.blur,
                spread: s.spread,
                color: s.color.resolve(tone),
            }));
        }
        if let Some(glows) = ax.back_glow {
            rs.back_glow.extend(glows.iter().map(|g| Glow {
                color: g.color.resolve(tone),
                spread: g.spread,
            }));
        }
        if let Some(tints) = ax.tints {
            rs.tints.extend(tints.iter().map(|t| Tint {
                color: t.color.resolve(tone),
                dir: t.dir,
            }));
        }
        if let Some(anim) = ax.anim {
            rs.anim.extend(anim.iter().copied());
        }
    }
    rs
}

impl ResolvedStyle {
    /// The shapes painted BEHIND the frame: the back-glow (a cheap concentric
    /// falloff — no blur, doctrine — so a saturated surface reads as "lit from
    /// directly behind"), plus any directional shadows beyond the first (the
    /// frame itself paints fill/edge/radius/primary-shadow). Pure + testable.
    pub fn behind_shapes(&self, rect: Rect) -> Vec<egui::Shape> {
        const GLOW_STEPS: usize = 5;
        let cr = CornerRadius::same(self.radius);
        let mut shapes: Vec<egui::Shape> = Vec::new();
        for glow in &self.back_glow {
            // Faintest, widest ring first; strongest ring hugs the rect last.
            for i in (0..GLOW_STEPS).rev() {
                let t = (i as f32 + 1.0) / GLOW_STEPS as f32;
                let alpha = (f32::from(glow.color.a()) * (1.0 - t) * 0.9) as u8;
                if alpha == 0 {
                    continue;
                }
                let c = Color32::from_rgba_unmultiplied(
                    glow.color.r(),
                    glow.color.g(),
                    glow.color.b(),
                    alpha,
                );
                let expand = glow.spread * t;
                shapes.push(egui::Shape::rect_filled(
                    rect.expand(expand),
                    CornerRadius::same(self.radius.saturating_add(expand as u8)),
                    c,
                ));
            }
        }
        // Directional shadows beyond the primary (e.g. claymorphic's light).
        for sh in self.shadows.iter().skip(1) {
            shapes.push(egui::Shape::from(sh.as_shape(rect, cr)));
        }
        shapes
    }
}

/// A treated surface: resolves a trait stack and paints the FULL treatment
/// (back-glow + shadows + fill + edge + tints) behind its content. The
/// drop-in successor to `brutal_frame`/`clay_frame` — `.show(ui, |ui| …)`.
pub struct Treated {
    tone: Tone,
    base: &'static str,
    stack: &'static [&'static str],
    state: TraitState,
    margin: egui::Margin,
}

/// Begin a treated surface with an explicit trait stack + state.
pub fn treated(
    tone: Tone,
    base: &'static str,
    stack: &'static [&'static str],
    state: TraitState,
) -> Treated {
    let margin = if base == "claymorphic" {
        egui::Margin::same(16)
    } else {
        egui::Margin::symmetric(10, 6)
    };
    Treated {
        tone,
        base,
        stack,
        state,
        margin,
    }
}

/// The neubrutalist actionable surface (idle). Successor to `brutal_frame`.
pub fn brutal(tone: Tone) -> Treated {
    treated(tone, "neubrutalist", &[], TraitState::Idle)
}

/// **VIBRON** (vibes + Tron) — the workhorse treatment, and the surface a
/// dense tool screen wears by default.
///
/// A quiet dark plate: overlay fill, 2px hairline edge, hard offset shadow,
/// body ink. Its defining move is RESTRAINT — the plate says nothing so the
/// ONE active control can, and the content stays the loudest thing on screen.
/// `TraitState::Active` swaps the edge to the surface's tone and washes the
/// fill at ~14% (never a solid tone); press sinks it onto its own shadow.
///
/// That is the difference between a tool screen and a bag of pills. Values
/// transcribed from `docs/design/mocks/wiredfront-editor-mock.html`; resolves
/// to the matrix coordinate `brutalist · massive · plate · hard-cast`.
pub fn vibron(tone: Tone, state: TraitState) -> Treated {
    treated(tone, "vibron", &[], state)
}

/// The CHROME treatment every ordinary control wears, as a changeable
/// setting rather than a hardcoded call.
///
/// Chrome is the app's own furniture — buttons, chips, panels. Which
/// treatment it wears is a CHOICE, so surfaces ask for "the chrome" and get
/// whatever is currently set. Swapping the whole app to another look is then
/// one call, not an edit to every widget helper.
///
/// A component may still ask for a SPECIFIC treatment ([`vibron`],
/// [`brutal`], [`clay`]) when it means that one on purpose — which is what
/// lets glass sit inside a brutalist container.
static CHROME: std::sync::OnceLock<std::sync::Mutex<&'static str>> = std::sync::OnceLock::new();

fn chrome_cell() -> &'static std::sync::Mutex<&'static str> {
    CHROME.get_or_init(|| std::sync::Mutex::new(DEFAULT_CHROME))
}

/// The treatment chrome wears until something changes it.
pub const DEFAULT_CHROME: &str = "vibron";

/// Which treatment chrome is currently wearing.
pub fn chrome() -> &'static str {
    *chrome_cell()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Dress the app's chrome in another treatment. Returns whether it took.
///
/// An UNKNOWN id is REFUSED and the current chrome kept, rather than
/// panicking: the realistic bad value is not a typo in code but a STALE
/// preset id read back from persisted operator data after a treatment was
/// renamed. Losing the setting is recoverable; killing the app over it is
/// not. The `bool` is the report — callers that care should check it.
pub fn set_chrome(id: &'static str) -> bool {
    if trait_def(id).is_none() {
        return false;
    }
    *chrome_cell()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = id;
    true
}

/// An ordinary control in whatever chrome is currently set. This is what
/// surfaces call; [`vibron`] and friends are for meaning one on purpose.
pub fn chrome_treated(tone: Tone, state: TraitState) -> Treated {
    treated(tone, chrome(), &[], state)
}

// ── the SEMANTIC layer: what a control MEANS ────────────────────────────

/// What a control means, which is the only thing a widget should have to
/// know. The treatment that paints it is DATA (`tokens/roles.json`), so
/// re-dressing the app never touches a widget.
///
/// Named `Intent` rather than `Role` on purpose: `egui_pilot_ai::protocol::
/// Role` is already imported across these surfaces, and a second `Role`
/// would be a readability trap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// The ordinary control. Most things.
    Chrome,
    /// The ONE primary action on a surface. If two things are heroes,
    /// neither is.
    Hero,
    /// Passive — content sits in it rather than acting.
    Quiet,
    /// Destructive. Pins its own tone so a delete never turns lime because
    /// the hub's tone is lime.
    Danger,
}

impl Intent {
    fn index(self) -> usize {
        match self {
            Intent::Chrome => 0,
            Intent::Hero => 1,
            Intent::Quiet => 2,
            Intent::Danger => 3,
        }
    }
}

/// One role's binding: which treatment paints it, and whether it pins a tone.
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    pub treatment: &'static str,
    /// `Some(hue)` overrides the caller's tone — danger is coral everywhere.
    pub tone: Option<&'static str>,
}

include!(concat!(env!("OUT_DIR"), "/roles_gen.rs"));

/// The binding for a role on a surface: the surface's own if it states one,
/// otherwise the default. A surface therefore declares only what it CHANGES.
pub fn binding(surface: &str, intent: Intent) -> Binding {
    let i = intent.index();
    SURFACE_ROLES
        .iter()
        .find(|(s, _)| *s == surface)
        .and_then(|(_, roles)| roles[i])
        .unwrap_or(DEFAULT_ROLES[i])
}

/// A control dressed by what it MEANS on a given surface.
///
/// This is what widgets call. They name an [`Intent`]; which treatment paints
/// it, and whether it keeps their tone, is authored data.
pub fn dressed(surface: &str, intent: Intent, tone: Tone, state: TraitState) -> Treated {
    let b = binding(surface, intent);
    // A pinned tone wins: `danger` must read the same on every surface.
    let tone = match b.tone.and_then(tone_for_hue) {
        Some(t) => t,
        None => tone,
    };
    treated(tone, b.treatment, &[], state)
}

/// The `Tone` that resolves through a given hue id, when one does. Roles pin
/// hues by NAME in data; this maps that name back onto the tone vocabulary.
fn tone_for_hue(hue: &str) -> Option<Tone> {
    Tone::ALL.iter().copied().find(|t| t.hue_id() == hue)
}

/// The claymorphic passive plane. Successor to `clay_frame`. Claymorphic
/// uses only fixed surface/ink refs, so the tone is inert here.
pub fn clay() -> Treated {
    treated(Tone::Cyan, "claymorphic", &[], TraitState::Idle)
}

impl Treated {
    /// Override the content inset.
    pub fn margin(mut self, margin: egui::Margin) -> Self {
        self.margin = margin;
        self
    }

    /// Paint the treatment behind `add_contents`. Returns the content's
    /// `InnerResponse`, so call sites keep their `.response` usage. The frame
    /// paints fill/edge/radius/primary-shadow (egui's proven background path);
    /// the back-glow + directional shadows paint into a slot reserved behind.
    pub fn show<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let style = compose(self.tone, self.base, self.stack, self.state);
        let behind = ui.painter().add(egui::Shape::Noop);
        let mut frame = egui::Frame::new()
            .fill(style.fill)
            .stroke(style.edge)
            .corner_radius(CornerRadius::same(style.radius))
            .inner_margin(self.margin);
        if let Some(shadow) = style.shadows.first() {
            frame = frame.shadow(*shadow);
        }
        let inner = frame.show(ui, add_contents);
        let shapes = style.behind_shapes(inner.response.rect);
        if !shapes.is_empty() {
            ui.painter().set(behind, egui::Shape::Vec(shapes));
        }
        inner
    }
}

#[cfg(test)]
mod trait_tests {
    use super::*;

    #[test]
    fn every_trait_paints_something_in_every_state() {
        for def in TRAITS {
            for state in [TraitState::Idle, TraitState::Hover, TraitState::Active] {
                let rs = compose(Tone::Lime, def.id, &[], state);
                assert!(
                    rs.edge.width > 0.0 || rs.fill.a() > 0,
                    "trait {} state {state:?} paints nothing",
                    def.id
                );
            }
        }
    }

    #[test]
    fn additive_axes_concatenate_across_the_stack() {
        // neubrutalist has 1 back-glow; cyber adds 2 → 3 total.
        let base = compose(Tone::Cyan, "neubrutalist", &[], TraitState::Idle);
        let stacked = compose(Tone::Cyan, "neubrutalist", &["cyber"], TraitState::Idle);
        assert_eq!(base.back_glow.len(), 1);
        assert_eq!(stacked.back_glow.len(), 3);
    }

    #[test]
    fn override_axes_last_in_the_stack_wins() {
        // cyber overrides the edge → stacked edge is cyan@400, not tone@700.
        let stacked = compose(Tone::Lime, "neubrutalist", &["cyber"], TraitState::Idle);
        assert_eq!(stacked.edge.color, step_on("cyan", 400));
    }

    #[test]
    fn state_overlay_replaces_within_the_trait() {
        let idle = compose(Tone::Amber, "neubrutalist", &[], TraitState::Idle);
        let active = compose(Tone::Amber, "neubrutalist", &[], TraitState::Active);
        assert_eq!(idle.shadows[0].offset, [3, 3]);
        assert_eq!(active.shadows[0].offset, [1, 1]);
    }

    #[test]
    fn tone_relative_fill_tracks_the_surface_tone() {
        let lime = compose(Tone::Lime, "neubrutalist", &[], TraitState::Idle);
        let cyan = compose(Tone::Cyan, "neubrutalist", &[], TraitState::Idle);
        assert_ne!(lime.fill, cyan.fill);
        assert_eq!(lime.fill, step_on("lime", 400));
    }

    #[test]
    fn neubrutalist_paints_a_back_glow() {
        let s = compose(Tone::Lime, "neubrutalist", &[], TraitState::Idle);
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 40.0));
        // neubrutalist authors one back-glow → concentric rings behind.
        assert!(
            !s.behind_shapes(rect).is_empty(),
            "neubrutalist should paint a back-glow behind the frame"
        );
    }
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
    pub const SHOWS: &str = "\u{E464}"; // squares-four — the operator's content, by category
    pub const BOT: &str = "\u{E762}"; // robot
    pub const PRESENT: &str = "\u{E58C}"; // monitor-play — the show machine

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
/// diffusion — a dark shadow bleeding down and out. This is the house theme.
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
    /// A uniform multiplier on every evaluated token. 1.0 for a real stage;
    /// below 1 only for [`Fluid::thumbnail`].
    scale: f32,
}

impl Fluid {
    /// The reference stage a MINIATURE stands for: 48 rem wide, 16:9.
    ///
    /// Not a chosen number. The authored ramp is built so every type token
    /// reaches its `min` at the same stage — ≈46.3 rem on a 16:9 stage — so
    /// 48 rem is the nearest whole rem that clears the floor for all of
    /// them. `nothing_is_clamped_at_the_nominal_stage` pins it, and will
    /// fail if `tokens/fluid.json` is ever re-authored past it.
    pub const NOMINAL_REM: f32 = 48.0;

    /// Fluid sizing for content laid out inside `rect`.
    pub fn of(rect: Rect) -> Self {
        Self {
            w_rem: rect.width() / 16.0,
            h_rem: rect.height() / 16.0,
            scale: 1.0,
        }
    }

    /// Sizing for a stage drawn as a MINIATURE of a full one.
    ///
    /// `Fluid::of` sizes content TO its stage, which is right for every real
    /// stage and wrong for a thumbnail: the ramp has a floor (`clamp(min,
    /// ..)`), so below roughly 46 rem the type stops shrinking while the
    /// stage keeps going, and a whole slide crops inside a card.
    ///
    /// A thumbnail is a picture OF the full stage, not a small stage, so it
    /// is evaluated at [`Fluid::NOMINAL_REM`] and scaled down to fit `rect`.
    /// Everything a document draws — type, padding, radii — flows through
    /// this one struct, so a single factor shrinks the whole render
    /// proportionally with no second mechanism.
    ///
    /// Fits INSIDE `rect` on both axes, so a stage that is not 16:9
    /// letterboxes rather than overflowing.
    pub fn thumbnail(rect: Rect) -> Self {
        let nominal_w = Self::NOMINAL_REM * 16.0;
        let nominal_h = nominal_w * 9.0 / 16.0;
        let scale = (rect.width() / nominal_w).min(rect.height() / nominal_h);
        Self {
            w_rem: Self::NOMINAL_REM,
            h_rem: nominal_h / 16.0,
            // A degenerate rect must not produce a negative or NaN ramp.
            scale: if scale.is_finite() {
                scale.max(0.0)
            } else {
                0.0
            },
        }
    }

    #[inline]
    fn eval(&self, min: f32, max: f32, slope: f32, slope_y: f32, intercept: f32) -> f32 {
        (intercept + slope * self.w_rem + slope_y * self.h_rem).clamp(min, max) * 16.0 * self.scale
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
/// The MATRIX — the faceted classification a Theme is a coordinate in
/// (`docs/design/coldlight-matrix.md`). The vocabularies and the legality
/// rulings are generated from `tokens/matrix.json`; `build.rs` fails if any
/// facet value is left unclassified by any ancestor, so the matrix cannot rot
/// as values are added.
pub mod matrix {
    include!(concat!(env!("OUT_DIR"), "/matrix_gen.rs"));

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The census law, applied to the matrix: eight ancestors ruling on
        /// every facet value, exactly once each. If this number moves without
        /// a vocabulary change, a ruling was lost.
        #[test]
        fn every_ancestor_rules_on_every_facet_value() {
            let values = STRUCTURES.len() + MATERIALS.len() + ATMOSPHERES.len();
            assert_eq!(ANCESTORS.len(), 8, "eight seeds, ratified 2026-07-31");
            // 22 since 2026-08-05: `plate` joined the material axis, because
            // the operator's ruling `brutalist · massive · plate · hard-cast
            // = Vibron` named a material the matrix did not have.
            assert_eq!(values, 22, "6 structures + 10 materials + 6 atmospheres");
            assert_eq!(
                RULING_COUNT,
                ANCESTORS.len() * values,
                "every ancestor must rule on every value"
            );
        }

        /// The ancestral list must stay SHORT — every added seed costs a full
        /// column of rulings, which is why atmospheres and materials are not
        /// promoted to lineages (matrix §3.2).
        #[test]
        fn the_ancestral_list_stays_short() {
            assert!(
                ANCESTORS.len() <= 8,
                "5-8 seeds is the workable range; 20 is not"
            );
        }
    }
}

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

    /// The RESOLVED transform an effect is asking for at a moment in time.
    ///
    /// Before this existed, [`Effect::at`] handed back a bare scalar and every
    /// call site decided for itself what the motion MEANT — `callout_card`
    /// read `ENTER_RISE` as "fade, plus a 6-unit lift", `gauge_dial` read the
    /// same shape as a sweep fraction, and a dozen other templates each made
    /// their own call. That is why the pool could not grow: a new effect was a
    /// new number somebody had to be hand-taught to interpret, not a new
    /// motion.
    ///
    /// Sizes here are NORMALISED, never pixels — `offset` is a fraction of
    /// whatever amplitude the caller chooses, so `Fluid` keeps owning size and
    /// the effect owns shape. A caller writes `fluid.sp_6() * m.offset.y`, and
    /// the one effect then reads correctly at every window size.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Motion {
        /// 1.0 = fully opaque. Clamped — opacity outside 0..1 is not a thing.
        pub alpha: f32,
        /// Displacement as a fraction of the caller's amplitude. Deliberately
        /// UNCLAMPED so a spring ease can overshoot in space.
        pub offset: egui::Vec2,
        /// 1.0 = natural size. Unclamped, for the same reason as `offset`.
        pub scale: f32,
        /// Radians.
        pub rotation: f32,
        /// Reveal fraction for a wipe or a stroke draw. Clamped 0..1.
        pub clip: f32,
    }

    impl Motion {
        /// The resting state — what every one-shot settles to, and what a
        /// caller paints when it has no effect at all.
        pub const REST: Motion = Motion {
            alpha: 1.0,
            offset: egui::Vec2::ZERO,
            scale: 1.0,
            rotation: 0.0,
            clip: 1.0,
        };
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
        /// The authored keyframe track, when the effect has one. `None` keeps
        /// the legacy single-ramp reading — the two coexist so migration is
        /// never big-bang (`docs/plans/keyframe-engine.md` §5).
        pub track: Option<&'static Track>,
    }

    /// One animatable channel — EXACTLY [`Motion`]'s fields, so an authored
    /// track can say everything the resolved struct can carry and nothing it
    /// cannot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Chan {
        Alpha,
        OffsetX,
        OffsetY,
        Scale,
        Rotation,
        Clip,
    }

    /// One keyframe. `ease` applies FROM this stop UNTIL the next one that
    /// declares the same channel — the Web Animations API rule, adopted so
    /// the format is a standard rather than a house invention.
    #[derive(Debug, Clone, Copy)]
    pub struct Stop {
        pub offset: f32,
        pub ease: Ease,
        pub ch: &'static [(Chan, f32)],
    }

    /// An authored keyframe track. Generated const data — no allocation, and
    /// [`Track::sample`] is a pure function of progress, so `PilotClock` can
    /// pin a frame for a deterministic proof shot (doctrine law 4).
    #[derive(Debug, Clone, Copy)]
    pub struct Track {
        pub stops: &'static [Stop],
    }

    impl Track {
        /// One channel's value at `p`. Channels interpolate INDEPENDENTLY:
        /// only the stops that declare a channel bound its segments, so
        /// `scale` may turn over at 0.62 while `alpha` runs straight 0→1.
        /// That independence is the whole reason a track beats a single ramp.
        fn channel(&self, c: Chan, p: f32) -> Option<f32> {
            let mut prev: Option<(f32, f32, Ease)> = None;
            for s in self.stops {
                let Some(&(_, v)) = s.ch.iter().find(|(k, _)| *k == c) else {
                    continue;
                };
                if p <= s.offset {
                    return Some(match prev {
                        // Before the first stop that declares it: hold.
                        None => v,
                        Some((po, pv, pe)) => {
                            let span = s.offset - po;
                            if span <= f32::EPSILON {
                                v
                            } else {
                                let local = ((p - po) / span).clamp(0.0, 1.0);
                                // The LEFT stop's ease owns the segment.
                                pv + (v - pv) * pe.apply(local)
                            }
                        }
                    });
                }
                prev = Some((s.offset, v, s.ease));
            }
            // Past the last stop that declares it: hold the final value.
            prev.map(|(_, v, _)| v)
        }

        /// The resolved [`Motion`] at linear progress `p` (0..1). A channel no
        /// stop declares stays at [`Motion::REST`] — an effect never has to
        /// restate what it does not move.
        pub fn sample(&self, p: f32) -> Motion {
            let p = p.clamp(0.0, 1.0);
            let rest = Motion::REST;
            Motion {
                alpha: self
                    .channel(Chan::Alpha, p)
                    .unwrap_or(rest.alpha)
                    .clamp(0.0, 1.0),
                // Unclamped: a spring must be able to overshoot in space.
                offset: egui::vec2(
                    self.channel(Chan::OffsetX, p).unwrap_or(rest.offset.x),
                    self.channel(Chan::OffsetY, p).unwrap_or(rest.offset.y),
                ),
                scale: self.channel(Chan::Scale, p).unwrap_or(rest.scale),
                rotation: self.channel(Chan::Rotation, p).unwrap_or(rest.rotation),
                clip: self
                    .channel(Chan::Clip, p)
                    .unwrap_or(rest.clip)
                    .clamp(0.0, 1.0),
            }
        }
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

        /// RAW progress — elapsed/dur, NO ease applied. One-shot clamps to
        /// 0..1; looped wraps. This exists because an authored [`Track`] does
        /// its own per-segment easing: feeding it [`Effect::at`] would ease
        /// twice and quietly bend every curve.
        pub fn progress(&self, now: f64, t0: f64, index: usize) -> f32 {
            let elapsed = (now - t0) as f32 - self.delay - self.stagger * index as f32;
            if self.dur <= 0.0 {
                return 1.0;
            }
            let p = elapsed / self.dur;
            if self.looped {
                p.max(0.0).fract()
            } else {
                p.clamp(0.0, 1.0)
            }
        }

        /// The RESOLVED transform this effect asks for at a moment in time.
        ///
        /// See [`Motion`] for why this exists. When the effect authors a
        /// [`Track`], the track IS the interpretation — read from
        /// `tokens/motion.json`, not from this file. Effects with no track
        /// fall through to the legacy reading below (fade in, rise from
        /// below, reveal forward), which is the one every call site was
        /// already applying by hand.
        pub fn sample(&self, now: f64, t0: f64, index: usize) -> Motion {
            if let Some(track) = self.track {
                // The track eases per segment, so it gets LINEAR progress.
                return track.sample(self.progress(now, t0, index));
            }
            let p = self.at(now, t0, index);
            Motion {
                alpha: p.clamp(0.0, 1.0),
                // Unclamped on purpose: a spring ease must be able to
                // overshoot in space, which is the whole point of authoring
                // one. Opacity and clip cannot, so they are clamped above and
                // below.
                offset: egui::vec2(0.0, 1.0 - p),
                scale: 1.0,
                rotation: 0.0,
                clip: p.clamp(0.0, 1.0),
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

        /// A 2-stop track reproduces the LEGACY reading exactly — so
        /// migrating the 19 effects is a reclassification, not a restyle
        /// (`docs/plans/keyframe-engine.md` §5). This is the guard that lets
        /// the port happen at all.
        #[test]
        fn a_two_stop_track_reproduces_the_legacy_reading() {
            // The legacy interpretation, written as data: fade in, rise from
            // below, reveal forward.
            const LEGACY: Track = Track {
                stops: &[
                    Stop {
                        offset: 0.0,
                        ease: EASE_EMPHASIZED,
                        ch: &[
                            (Chan::Alpha, 0.0),
                            (Chan::OffsetY, 1.0),
                            (Chan::Clip, 0.0),
                        ],
                    },
                    Stop {
                        offset: 1.0,
                        ease: EASE_EMPHASIZED,
                        ch: &[
                            (Chan::Alpha, 1.0),
                            (Chan::OffsetY, 0.0),
                            (Chan::Clip, 1.0),
                        ],
                    },
                ],
            };
            // ENTER_RISE wears EASE_EMPHASIZED and has no track, so it takes
            // the legacy path — the exact pair we need to compare.
            let legacy_fx = ENTER_RISE;
            for step in 0..=8 {
                let p = step as f32 / 8.0;
                // Legacy: at() eases, then sample() interprets.
                let now = legacy_fx.delay as f64 + (p * legacy_fx.dur) as f64;
                let old = legacy_fx.sample(now, 0.0, 0);
                // Track: raw progress in, per-segment ease inside.
                let new = LEGACY.sample(legacy_fx.progress(now, 0.0, 0));
                assert!(
                    (old.alpha - new.alpha).abs() < 1e-5
                        && (old.offset.y - new.offset.y).abs() < 1e-5
                        && (old.clip - new.clip).abs() < 1e-5,
                    "p={p}: legacy {old:?} != track {new:?}"
                );
            }
        }

        /// The capability a single ramp CANNOT express: two channels on two
        /// schedules. `enter-bloom` runs alpha straight to 1 by 62% while
        /// scale overshoots to 1.04 and settles back to 1.0.
        #[test]
        fn a_track_moves_channels_on_independent_schedules() {
            let t = ENTER_BLOOM.track.expect("enter-bloom authors a track");
            assert_eq!(t.stops.len(), 3);
            let (start, mid, end) = (t.sample(0.0), t.sample(0.62), t.sample(1.0));
            assert!((start.alpha - 0.0).abs() < 1e-5, "starts transparent");
            assert!((start.scale - 0.86).abs() < 1e-5, "starts small");
            assert!((mid.scale - 1.04).abs() < 1e-4, "overshoots at 62%");
            assert!((end.scale - 1.0).abs() < 1e-5, "settles to natural size");
            assert!((end.alpha - 1.0).abs() < 1e-5, "ends opaque");
            // The overshoot is REAL — somewhere past mid, scale exceeds 1.
            let peak = (0..=100)
                .map(|i| t.sample(i as f32 / 100.0).scale)
                .fold(0.0f32, f32::max);
            assert!(peak > 1.0, "scale must overshoot, peaked at {peak}");
            // A channel no stop declares stays at REST — never restated.
            assert_eq!(end.rotation, Motion::REST.rotation);
        }

        /// Sampling is a pure function of progress — the property `PilotClock`
        /// relies on to pin a frame for a deterministic proof shot (law 4).
        #[test]
        fn track_sampling_is_pure_and_bounded() {
            let t = ENTER_BLOOM.track.expect("track");
            for i in 0..=20 {
                let p = i as f32 / 20.0;
                assert_eq!(t.sample(p), t.sample(p), "same input, same output");
            }
            // Out-of-range progress clamps rather than extrapolating.
            assert_eq!(t.sample(-1.0), t.sample(0.0));
            assert_eq!(t.sample(2.0), t.sample(1.0));
            // Opacity and clip are clamped even when a spring overshoots.
            for i in 0..=40 {
                let m = t.sample(i as f32 / 40.0);
                assert!((0.0..=1.0).contains(&m.alpha), "alpha {} escaped", m.alpha);
                assert!((0.0..=1.0).contains(&m.clip), "clip {} escaped", m.clip);
            }
        }

        /// An effect with no track keeps the legacy path untouched, so the two
        /// coexist and migration is never big-bang.
        #[test]
        fn untracked_effects_still_take_the_legacy_path() {
            assert!(ENTER_FADE.track.is_none());
            assert!(ENTER_BLOOM.track.is_some());
            let m = ENTER_FADE.sample(10.0, 0.0, 0);
            assert_eq!(m, Motion::REST, "a settled untracked effect rests");
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

        /// `sample` must reproduce EXACTLY the arithmetic every call site was
        /// doing by hand, or this refactor is a silent visual regression
        /// across every animated slide. `callout_card` computed
        /// `alpha = p.clamp(0,1)` and `lift = sp_6 * (1.0 - p)`; those two
        /// lines are the contract being pinned here.
        #[test]
        fn sample_reproduces_the_hand_math_it_replaces() {
            for &(now, index) in &[(0.0, 0), (0.35, 0), (0.5, 2), (100.0, 4)] {
                let p = ENTER_RISE.at(now, 0.0, index);
                let m = ENTER_RISE.sample(now, 0.0, index);
                assert_eq!(m.alpha, p.clamp(0.0, 1.0), "alpha at {now}/{index}");
                assert_eq!(m.offset.y, 1.0 - p, "lift fraction at {now}/{index}");
                assert_eq!(m.clip, p.clamp(0.0, 1.0), "reveal at {now}/{index}");
            }
        }

        /// A one-shot settles to the identity transform and stays there — the
        /// presenter law is that every state is a REST state.
        #[test]
        fn a_finished_effect_samples_to_rest() {
            let m = ENTER_RISE.sample(100.0, 0.0, 0);
            assert!((m.alpha - Motion::REST.alpha).abs() < 0.001);
            assert!(
                m.offset.length() < 0.001,
                "settled offset was {:?}",
                m.offset
            );
            assert!((m.clip - Motion::REST.clip).abs() < 0.001);
        }

        /// Opacity is clamped, displacement is not. A spring ease overshoots
        /// past its target in SPACE — that is the reason to author one — while
        /// alpha stays a legal opacity throughout.
        #[test]
        fn a_spring_overshoots_in_space_but_never_in_opacity() {
            let springy = Effect {
                dur: 1.0,
                delay: 0.0,
                stagger: 0.0,
                ease: EASE_SPRING_SOFT,
                looped: false,
                track: None,
            };
            let mut overshot = false;
            for i in 0..=40 {
                let m = springy.sample(i as f64 / 40.0, 0.0, 0);
                assert!(
                    (0.0..=1.0).contains(&m.alpha),
                    "alpha {} left 0..1",
                    m.alpha
                );
                assert!((0.0..=1.0).contains(&m.clip), "clip {} left 0..1", m.clip);
                if m.offset.y < -0.001 {
                    overshot = true;
                }
            }
            assert!(overshot, "spring-soft must carry past its target in space");
        }

        #[test]
        fn the_vocabulary_is_generated_and_complete() {
            assert_eq!(EFFECT_COUNT, 19, "motion.json ships 19 named effects");
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
/// dying out by the midline. Solid color + gradient tint — the sprite theme.
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

    /// A 16:9 stage `rem` rem wide.
    fn stage(rem: f32) -> Rect {
        let w = rem * 16.0;
        Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, w * 9.0 / 16.0))
    }

    /// Every type size a document can name. Kept beside the resolver in
    /// `dpl-slides`; if the ramp grows a step, this list is what has to grow
    /// with it.
    fn type_ramp(f: &Fluid) -> [f32; 10] {
        [
            f.fs_eyebrow(),
            f.fs_body_sm(),
            f.fs_body(),
            f.fs_body_lg(),
            f.fs_h4(),
            f.fs_h3(),
            f.fs_h2(),
            f.fs_h1(),
            f.fs_display(),
            f.fs_display_xl(),
        ]
    }

    /// The definition of "nominal", made executable: at the reference stage
    /// NOTHING is pinned to its floor, so a size there is still a real
    /// function of the stage. A token that has stopped responding would read
    /// identically on a slightly smaller stage — that is what clamped means.
    #[test]
    fn nothing_is_clamped_at_the_nominal_stage() {
        let at = type_ramp(&Fluid::of(stage(Fluid::NOMINAL_REM)));
        let smaller = type_ramp(&Fluid::of(stage(Fluid::NOMINAL_REM * 0.98)));
        for (i, (a, b)) in at.iter().zip(smaller.iter()).enumerate() {
            assert!(
                a > b,
                "type step {i} is already floored at the nominal stage \
                 ({a} vs {b}) — NOMINAL_REM no longer clears the ramp"
            );
        }
    }

    /// The floor is real, and it is what a miniature has to escape: well
    /// below the nominal stage, `of()` stops shrinking.
    #[test]
    fn a_small_stage_floors_the_ramp_but_a_thumbnail_does_not() {
        let small = stage(20.0);
        let smaller = stage(10.0);
        assert_eq!(
            Fluid::of(small).fs_display(),
            Fluid::of(smaller).fs_display(),
            "the floor is the whole reason thumbnail() exists"
        );
        assert!(
            Fluid::thumbnail(smaller).fs_display() < Fluid::thumbnail(small).fs_display(),
            "a miniature must keep shrinking with its stage"
        );
    }

    /// A miniature is the full stage scaled: every token shrinks by ONE
    /// factor, so proportions are identical to the real render.
    #[test]
    fn a_thumbnail_is_the_nominal_stage_scaled_uniformly() {
        let full = Fluid::of(stage(Fluid::NOMINAL_REM));
        let half = Fluid::thumbnail(stage(Fluid::NOMINAL_REM / 2.0));
        for (a, b) in type_ramp(&full).iter().zip(type_ramp(&half).iter()) {
            assert!(
                (b * 2.0 - a).abs() < 0.01,
                "expected exactly half of {a}, got {b}"
            );
        }
        // Padding rides the same factor — a document's spacing is fluid too.
        assert!((half.sp_7() * 2.0 - full.sp_7()).abs() < 0.01);
    }

    /// `of()` is untouched by the new field, and a degenerate rect cannot
    /// produce a NaN or negative size.
    #[test]
    fn of_is_unscaled_and_a_degenerate_rect_stays_finite() {
        let f = Fluid::of(stage(40.0));
        assert!((f.fs_body() - Fluid::of(stage(40.0)).fs_body()).abs() < f32::EPSILON);
        let nothing = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(0.0, 0.0));
        for size in type_ramp(&Fluid::thumbnail(nothing)) {
            assert!(size.is_finite() && size >= 0.0, "got {size}");
        }
    }

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
        assert_eq!(THEME_PRESETS.len(), THEME_PRESET_COUNT);
        assert_eq!(MOTION_PRESETS.len(), MOTION_PRESET_COUNT);
        let mut ids: Vec<&str> = THEME_PRESETS.iter().map(|p| p.id).collect();
        ids.extend(MOTION_PRESETS.iter().map(|p| p.id));
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate preset id");
        for p in THEME_PRESETS {
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
        assert!(theme_preset("neon-intro").is_some());
        assert!(theme_preset("ghost").is_none());
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

    // ── V1: the dwell tier, the ink-bleed, and the brain ────────────────

    #[test]
    fn the_dwell_tier_is_authored_in_the_window_the_operator_named() {
        // Bound to locals so this reads the AUTHORED values as data — the
        // point is that motion.json says this, not that a literal does.
        let (dwell, lo, hi, slow) = (
            motion::DUR_DWELL,
            motion::DUR_DWELL_MIN,
            motion::DUR_DWELL_MAX,
            motion::DUR_SLOW,
        );
        assert!(
            (5.6..=6.3).contains(&dwell),
            "the dwell tier must sit in the operator's 5.6-6.3s window, got {dwell}"
        );
        assert!(lo < hi, "the jitter window is a range, got {lo}..{hi}");
        assert!(
            lo >= 5.6 && hi <= 6.3,
            "the authored bounds are the window itself, got {lo}..{hi}"
        );
        assert!(
            dwell > slow * 10.0,
            "dwell is a different CLASS of motion, not a longer `slow` ({dwell} vs {slow})"
        );
    }

    #[test]
    fn the_ink_bleed_effect_resolves_and_is_not_a_loop() {
        let fx = motion::effect("ink-bleed").expect("ink-bleed is authored in motion.json");
        assert!(
            !fx.looped,
            "the bleed settles at full colour; it does not cycle"
        );
        assert!(
            (5.6..=6.3).contains(&fx.dur),
            "the effect carries the dwell duration, got {}",
            fx.dur
        );
    }

    #[test]
    fn the_brain_icon_is_tessellated() {
        let def = icon_def("brain").expect("brain is authored in icons.json");
        assert!(
            !def.vertices.is_empty() && !def.indices.is_empty(),
            "the vendored brain.svg must tessellate to real geometry"
        );
        assert_eq!(def.label, "Providers");
    }

    /// The census law for the EDITOR's icon set: every id the composer
    /// paints must resolve to real geometry. A dropped icons.json entry
    /// would otherwise degrade silently to `draw_icon`'s hairline box on
    /// a live surface — this fails the gate instead.
    /// Chrome is a SETTING, not a hardcoded call — so the whole app can be
    /// re-dressed later without touching a single widget helper.
    /// A surface states only what it CHANGES; every other role falls back to
    /// `default`. That fallback is what keeps roles.json small as surfaces
    /// multiply.
    #[test]
    fn a_surface_states_only_what_it_changes() {
        // wiredfront overrides chrome only.
        assert_eq!(binding("wiredfront", Intent::Chrome).treatment, "vibron");
        assert_eq!(
            binding("wiredfront", Intent::Hero).treatment,
            binding("nosuchsurface", Intent::Hero).treatment,
            "an unstated role falls back to default"
        );
        // An UNKNOWN surface is not an error — it simply wears the defaults,
        // so a new hub paints correctly before anyone authors it.
        for i in [Intent::Chrome, Intent::Hero, Intent::Quiet, Intent::Danger] {
            assert!(!binding("brand-new-hub", i).treatment.is_empty());
        }
        // Workbench genuinely differs from WiredFRONT.
        assert_ne!(
            binding("workbench", Intent::Chrome).treatment,
            binding("wiredfront", Intent::Chrome).treatment,
            "per-surface chrome must actually differ"
        );
    }

    /// Danger pins its own tone, so a destructive control never takes the
    /// hub's colour — "delete this" must not turn lime on a lime surface.
    #[test]
    fn danger_keeps_its_own_tone_against_any_caller() {
        // Asserted through compose() on the resolved binding, so this checks
        // the PAINTED colour rather than a wrapper's shape.
        let painted = |surface: &str, caller: Tone| {
            let b = binding(surface, Intent::Danger);
            let tone = b
                .tone
                .and_then(|h| Tone::ALL.iter().copied().find(|t| t.hue_id() == h))
                .unwrap_or(caller);
            compose(tone, b.treatment, &[], TraitState::Idle).fill
        };
        assert_eq!(
            painted("wiredfront", Tone::Lime),
            painted("workbench", Tone::Cyan),
            "danger ignores the caller's tone and reads the same everywhere"
        );
        // And it is genuinely NOT the caller's colour.
        assert_ne!(
            painted("wiredfront", Tone::Lime),
            compose(Tone::Lime, "neubrutalist", &[], TraitState::Idle).fill,
            "danger on a lime surface must not be lime"
        );
        // Chrome, by contrast, KEEPS the caller's tone.
        assert!(binding("wiredfront", Intent::Chrome).tone.is_none());
        assert_eq!(binding("wiredfront", Intent::Danger).tone, Some("coral"));
    }

    #[test]
    fn chrome_is_switchable_and_refuses_a_typo() {
        assert_eq!(chrome(), DEFAULT_CHROME, "vibron until something changes it");
        assert!(set_chrome("neubrutalist"), "any authored trait is fair game");
        assert_eq!(chrome(), "neubrutalist");
        // A resolved style really does follow the setting — compared through
        // compose() so this asserts on the PAINTED values, not on a wrapper.
        let loud = compose(Tone::Magenta, chrome(), &[], TraitState::Idle);
        assert!(set_chrome("vibron"));
        let quiet = compose(Tone::Magenta, chrome(), &[], TraitState::Idle);
        assert_ne!(
            loud.fill, quiet.fill,
            "neubrutalist is a saturated fill; vibron is a quiet plate — they              must not resolve to the same colour"
        );
        // A typo is refused and the current chrome survives.
        assert!(!set_chrome("vibrn"), "unknown id refused");
        assert_eq!(chrome(), "vibron", "and the good one is kept");
    }

    #[test]
    fn the_editor_icon_set_is_whole() {
        const EDITOR_ICONS: &[&str] = &[
            // tool strip
            "tool-select",
            "tool-hand",
            "tool-pen",
            "tool-camera",
            "tool-text",
            "tool-null",
            "tool-precomp",
            "tool-track",
            "tool-mblur",
            "tool-3d",
            // transport
            "transport-start",
            "transport-back",
            "transport-play",
            "transport-pause",
            "transport-forward",
            "transport-end",
            // project panel
            "browse",
            "ref-link",
            "ref-copy",
            "proj-comp",
            "proj-pack",
            "proj-video",
            "proj-image",
            "proj-render",
            "fx",
            // fx pane + status
            "search",
            "info",
            // timeline
            "zoom-in",
            "zoom-out",
            "delete",
            "group-open",
            "group-closed",
            // the mock's ⑦ switch columns
            "layer-visible",
            "layer-hidden",
            "layer-lock",
            "layer-parent",
        ];
        for id in EDITOR_ICONS {
            let def = icon_def(id)
                .unwrap_or_else(|| panic!("{id} must be authored in tokens/icons.json"));
            assert!(
                !def.vertices.is_empty() && !def.indices.is_empty(),
                "{id} ({}) must tessellate to real geometry",
                def.lucide
            );
        }
        assert_eq!(
            ICON_COUNT,
            25 + EDITOR_ICONS.len(),
            "the registry is the 25 chrome ids plus the editor set — \
             update this count deliberately when either grows"
        );
    }

    #[test]
    fn the_bleed_pool_is_known_hues() {
        let p = bleed_preset("providers-ink").expect("the providers bleed is authored");
        assert!(p.hues.len() >= 4, "the operator asked for a pool of 4-5");
        for hue in p.hues {
            assert!(ramp(hue).is_some(), "{hue} must exist in the authored pool");
        }
        assert_eq!(p.effect, "ink-bleed");
    }

    #[test]
    fn a_bleed_pair_is_always_two_different_hues() {
        let p = bleed_preset("providers-ink").expect("preset");
        for seed in 0..512u64 {
            let (a, b) = p.pair_ids(seed);
            assert_ne!(
                a, b,
                "seed {seed} drew the same hue twice — that is one colour, not a bleed"
            );
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_pair() {
        // The proof-shot guarantee: a freely random pick would make every
        // capture of the bleed flaky.
        let p = bleed_preset("providers-ink").expect("preset");
        for seed in [0u64, 1, 7, 99, 12_345] {
            assert_eq!(p.pair_ids(seed), p.pair_ids(seed));
            assert_eq!(p.pair(seed), p.pair(seed));
        }
    }

    #[test]
    fn the_pool_is_actually_drawn_from() {
        // A pick that always returned the same pair would pass every test
        // above and still be wrong.
        let p = bleed_preset("providers-ink").expect("preset");
        let seen: std::collections::BTreeSet<_> = (0..256u64).map(|s| p.pair_ids(s)).collect();
        assert!(
            seen.len() > 4,
            "the seed must actually range over the pool, saw {} pairs",
            seen.len()
        );
    }
}
