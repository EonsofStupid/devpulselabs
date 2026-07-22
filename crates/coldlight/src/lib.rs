//! ColdLight, Rust output target.
//!
//! The JSON tokens in the prior repo remain the authored source; this crate is
//! their second emitter (hand-seeded for P0, generated in P1 — same
//! Style-Dictionary thesis, new target). The paint vocabulary is the
//! operator's, verbatim: **crisp sprite-grade edges, solid fills with gradient
//! tint, black diffusion scatter** — dark falloff painted on top, never
//! backdrop blur. Cheap on this renderer by design.

use egui::{Color32, CornerRadius, Rect, Shadow, Stroke};

/// The tone slots. Each hub claims one; the fx vocabulary tints by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    Cyan,
    Lime,
    Violet,
    Amber,
    Magenta,
}

impl Tone {
    /// The saturated accent — text, active edges, dots.
    pub const fn color(self) -> Color32 {
        match self {
            Tone::Cyan => Color32::from_rgb(0x39, 0xd7, 0xff),
            Tone::Lime => Color32::from_rgb(0xa8, 0xe6, 0x3e),
            Tone::Violet => Color32::from_rgb(0xa8, 0x7c, 0xff),
            Tone::Amber => Color32::from_rgb(0xff, 0xb8, 0x2e),
            Tone::Magenta => Color32::from_rgb(0xff, 0x4f, 0xc3),
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
