//! The document renderer — layer 0 meets layer 3: a `dpl_doc::Slide` (refs,
//! never values) resolved through the generated registries and painted.
//! ONE code path renders documents everywhere — the Workbench seeds wall,
//! the editor stage (W4), the deck, thumbnails.
//!
//! Degrade law: an unknown ref never crashes — unknown fonts fall back to
//! the default face, unknown sizes to body, unknown inks to base, unknown
//! moods to an honest labeled plate.

mod background;
pub mod doc_slide;
mod refs;
mod stack;
pub mod template;
#[cfg(test)]
mod tests;

use egui::{Rect, Ui};

/// Where each part of a document landed after paint — the editor (W4)
/// hit-tests taps against this. `background` is the whole stage; `blocks`
/// pairs each block id with the rect it painted into.
#[derive(Debug, Clone)]
pub struct RenderMap {
    pub background: Rect,
    pub blocks: Vec<(String, Rect)>,
}

impl Default for RenderMap {
    fn default() -> Self {
        Self {
            background: Rect::ZERO,
            blocks: Vec::new(),
        }
    }
}

impl RenderMap {
    /// The innermost block a point falls in (topmost wins), else None.
    pub fn block_at(&self, p: egui::Pos2) -> Option<&str> {
        self.blocks
            .iter()
            .rev()
            .find(|(_, r)| r.contains(p))
            .map(|(id, _)| id.as_str())
    }
}

/// How a document's stage is sized.
///
/// The distinction exists because `Fluid`'s ramp has a floor: below roughly
/// 46 rem the type stops shrinking while the stage keeps going, so a whole
/// slide drawn into a card-sized rect crops. That floor is CORRECT for a
/// real stage — text has to stay legible — and wrong for a picture of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    /// A real stage: content sizes TO the rect, floor and all.
    #[default]
    Full,
    /// A miniature OF a full stage: sized at the nominal stage and scaled
    /// down uniformly, so the whole slide fits and keeps its proportions.
    Thumbnail,
}

/// Render a document into `rect`. The ONE path — every surface calls this.
/// `styling` is the tiers above the slide (W3b, LK2): the slide's absent
/// styling fields inherit from the look it follows, then the show's theme,
/// via `dpl_doc::resolve`; standalone slides pass
/// `&dpl_doc::Styling::standalone()`.
pub fn render(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    styling: &dpl_doc::Styling,
    now: f64,
    t0: f64,
) {
    let _ = render_traced(ui, rect, slide, styling, now, t0);
}

/// Render a document as a MINIATURE — the seeds wall, the authoring
/// carousel, the filmstrip. Same document, same code path; only the stage
/// sizing differs (see [`Stage`]).
pub fn render_thumbnail(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    styling: &dpl_doc::Styling,
    now: f64,
    t0: f64,
) -> RenderMap {
    stack::render_inner(
        ui,
        rect,
        slide,
        styling,
        Playhead {
            step: None,
            now,
            t0,
            stage: Stage::Thumbnail,
        },
    )
}

/// A MINIATURE at a beat (RL): what the card shows after `step` taps — the
/// On tap panel's preview, sized like every thumbnail.
pub fn render_thumbnail_step(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    styling: &dpl_doc::Styling,
    step: usize,
    now: f64,
    t0: f64,
) -> RenderMap {
    stack::render_inner(
        ui,
        rect,
        slide,
        styling,
        Playhead {
            step: Some(step),
            now,
            t0,
            stage: Stage::Thumbnail,
        },
    )
}

/// Render AND return where every block landed (for the editor's hit-test).
/// Wall mode: every block visible, the entrance staggers them all.
pub fn render_traced(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    styling: &dpl_doc::Styling,
    now: f64,
    t0: f64,
) -> RenderMap {
    stack::render_inner(
        ui,
        rect,
        slide,
        styling,
        Playhead {
            step: None,
            now,
            t0,
            stage: Stage::Full,
        },
    )
}

/// Deck mode (D2): render at a BEAT position. Step 0 = setup (unrevealed
/// blocks hidden, the visible ones play the slide entrance); step k shows
/// reveals[0..k], the LATEST reveal rising from `t0` (the deck restarts
/// its clock every beat, so each beat animates exactly one thing). Layout
/// measures ALL blocks — no reflow between steps; hidden blocks aren't
/// painted but their rects still land in the RenderMap.
pub fn render_step(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    styling: &dpl_doc::Styling,
    step: usize,
    now: f64,
    t0: f64,
) -> RenderMap {
    stack::render_inner(
        ui,
        rect,
        slide,
        styling,
        Playhead {
            step: Some(step),
            now,
            t0,
            stage: Stage::Full,
        },
    )
}

/// Where one render sits in time, and how its stage is sized.
///
/// Bundled rather than passed loose: the four travel together through every
/// entry point, and splitting them across a long argument list is how the
/// deck's beat and the wall's stagger got confused for each other.
pub(super) struct Playhead {
    /// `Some(k)` is deck mode at beat k; `None` is wall mode (all visible).
    pub(super) step: Option<usize>,
    pub(super) now: f64,
    pub(super) t0: f64,
    pub(super) stage: Stage,
}

/// The layout a slide's component gives it, with the slide's variant
/// applied (LS4a) — None when the component is not registered or has no
/// layout yet (the lines then flow in their own order).
pub fn laid_out(slide: &dpl_doc::Slide) -> Option<dpl_doc::component::LaidOut> {
    crate::registry::contract_of(slide.component_id())?.laid_out(&slide.variant)
}

/// The slide as the editor and every tray should read it: resolved through
/// its looks, its theme and its component's slot defaults — the values the
/// renderer paints.
pub fn shown(slide: &dpl_doc::Slide, styling: &dpl_doc::Styling) -> dpl_doc::Slide {
    styling.resolve_laid_out(slide, laid_out(slide).as_ref())
}
