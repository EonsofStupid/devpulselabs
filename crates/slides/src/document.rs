//! The document renderer — layer 0 meets layer 3: a `dpl_doc::Slide` (refs,
//! never values) resolved through the generated registries and painted.
//! ONE code path renders documents everywhere — the Workbench seeds wall,
//! the editor stage (W4), the deck, thumbnails.
//!
//! Degrade law: an unknown ref never crashes — unknown fonts fall back to
//! the default face, unknown sizes to body, unknown inks to base, unknown
//! moods to an honest labeled plate.

use crate::blocks::{ambient_backdrop, media_block_backdrop, Ambient};
use crate::motion::Clock;
use dpl_coldlight::{self as cl};
use egui::{Color32, Rect, Ui};
use std::f32::consts::TAU;

/// Resolve a document size ref ("display", "h3", …) through the fluid ramp.
fn size_of(fluid: &cl::Fluid, name: &str) -> Option<f32> {
    Some(match name {
        "eyebrow" => fluid.fs_eyebrow(),
        "body-sm" => fluid.fs_body_sm(),
        "body" => fluid.fs_body(),
        "body-lg" => fluid.fs_body_lg(),
        "h4" => fluid.fs_h4(),
        "h3" => fluid.fs_h3(),
        "h2" => fluid.fs_h2(),
        "h1" => fluid.fs_h1(),
        "display" => fluid.fs_display(),
        "display-xl" => fluid.fs_display_xl(),
        _ => return None,
    })
}

/// Resolve a semantic ink ref; `tone` is the background's leading accent.
fn ink_of(name: &str, tone: Color32) -> Color32 {
    match name {
        "strong" => cl::ink::STRONG,
        "muted" => cl::ink::MUTED,
        "subtle" => cl::ink::SUBTLE,
        "tone" => tone,
        // "base" and any unknown degrade to the body ink.
        _ => cl::ink::BASE,
    }
}

/// Resolve a font ref against the pool — an UNREGISTERED family name would
/// panic at layout, so unknowns fall back to the default face.
fn family_of(id: &str) -> egui::FontFamily {
    if cl::FONTS.iter().any(|f| f.id == id) {
        cl::font_family(id)
    } else {
        egui::FontFamily::Proportional
    }
}

fn ambient_of(name: &str) -> Option<Ambient> {
    match name {
        "pulse" => Some(Ambient::Pulse),
        "drift" => Some(Ambient::Drift),
        "rings" => Some(Ambient::Rings),
        _ => None,
    }
}

fn plate(ui: &Ui, rect: Rect, message: &str) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0, cl::surface::RAISED);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        message,
        egui::FontId::monospace(12.0),
        cl::ink::SUBTLE,
    );
}

/// Paint the document's background into `rect`; returns the leading accent
/// color (the "tone" ink and the ambience ride it). Takes the RESOLVED
/// background — `None` only if resolution was skipped, painting as the
/// bare well (defensive; `resolve` always fills it).
fn background(
    ui: &mut Ui,
    rect: Rect,
    bg: Option<&dpl_doc::Background>,
    now: f64,
    t0: f64,
) -> Color32 {
    let find = |name: &str| cl::MOODS.iter().find(|m| m.name == name);
    let painter = ui.painter_at(rect);
    match bg.unwrap_or(&dpl_doc::Background::None) {
        dpl_doc::Background::MoodBlend {
            moods, ambience, ..
        } => {
            let (a, b) = (
                moods.first().and_then(|m| find(m)),
                moods.get(1).and_then(|m| find(m)),
            );
            let (Some(a), Some(b)) = (a, b) else {
                plate(ui, rect, "mood-blend · unknown mood ref");
                return cl::ink::BASE;
            };
            // The authored blend-drift clock drives both knobs: a full
            // a→b→a crossfade per loop, the gradient axis breathing ±0.12
            // of its diagonal. Subtle by construction.
            let phase = cl::motion::BLEND_DRIFT.at(now, t0, 0) * TAU;
            let mix = 0.5 - 0.5 * phase.cos();
            let pan = phase.sin() * 0.12;
            cl::mood_blend(&painter, rect, a, b, mix, pan);
            let s = a.scrim_color();
            painter.rect_filled(
                rect,
                0,
                Color32::from_rgba_unmultiplied(s.r(), s.g(), s.b(), s.a() / 2),
            );
            let accent = a.accent_color();
            if let Some(name) = ambience {
                if let Some(amb) = ambient_of(name) {
                    ambient_backdrop_clipped(ui, rect, amb, accent, now, t0);
                }
            }
            ui.ctx().request_repaint(); // the blend never rests
            accent
        }
        dpl_doc::Background::Mood { mood } => match find(mood) {
            Some(m) => {
                cl::mood_gradient(&painter, rect, m);
                let s = m.scrim_color();
                painter.rect_filled(
                    rect,
                    0,
                    Color32::from_rgba_unmultiplied(s.r(), s.g(), s.b(), s.a() / 2),
                );
                m.accent_color()
            }
            None => {
                plate(ui, rect, &format!("unknown mood {mood:?}"));
                cl::ink::BASE
            }
        },
        dpl_doc::Background::None => cl::ink::BASE,
    }
}

fn ambient_backdrop_clipped(
    ui: &mut Ui,
    rect: Rect,
    amb: Ambient,
    color: Color32,
    now: f64,
    t0: f64,
) {
    // The ambience painters draw via ui.painter(); scope a child so their
    // shapes clip to the stage.
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(rect);
    ambient_backdrop(&mut child, rect, amb, color, now, t0);
}

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

/// Render a document into `rect`. The ONE path — every surface calls this.
/// `look` is the show's trait (W3b): the slide's absent styling fields
/// inherit from it via `dpl_doc::resolve`; standalone slides pass
/// `&dpl_doc::Look::fallback()`.
pub fn render(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    look: &dpl_doc::Look,
    now: f64,
    t0: f64,
) {
    let _ = render_traced(ui, rect, slide, look, now, t0);
}

/// Render AND return where every block landed (for the editor's hit-test).
/// Wall mode: every block visible, the entrance staggers them all.
pub fn render_traced(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    look: &dpl_doc::Look,
    now: f64,
    t0: f64,
) -> RenderMap {
    render_inner(ui, rect, slide, look, None, now, t0)
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
    look: &dpl_doc::Look,
    step: usize,
    now: f64,
    t0: f64,
) -> RenderMap {
    render_inner(ui, rect, slide, look, Some(step), now, t0)
}

fn render_inner(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    look: &dpl_doc::Look,
    step: Option<usize>,
    now: f64,
    t0: f64,
) -> RenderMap {
    // The whole cascade happens HERE, once — the body below paints a
    // fully-specified document exactly as before W3b.
    let slide = &dpl_doc::resolve(slide, look);
    let step = step.map(|s| s.min(slide.step_count() - 1));
    let accent = background(ui, rect, slide.background.as_ref(), now, t0);

    // Where each block sits in the reveal order (None = always visible).
    let reveal_pos = |id: &str| slide.reveals.iter().position(|r| r == id);
    let visible_at = |id: &str| match (step, reveal_pos(id)) {
        (Some(s), Some(p)) => p < s,
        _ => true, // wall mode, or a block outside the reveal order
    };

    // Media pre-pass (W11): media blocks paint FULL-BLEED — under any text,
    // over the mood. A media-only slide (the media_frame template) fills the
    // stage; media behind text is a backdrop. Reveals honored like text.
    for block in &slide.blocks {
        if let dpl_doc::Block::Media {
            id,
            show,
            src,
            fit,
            poster,
        } = block
        {
            if visible_at(id) {
                media_block_backdrop(ui, rect, show, src, fit.as_deref(), poster.as_deref());
            }
        }
    }

    let fluid = cl::Fluid::of(rect);
    let painter = ui.painter_at(rect);
    let pad = fluid.sp_7();
    let wrap = (rect.width() - pad * 2.0).max(40.0);

    // Pass 1 — measure (galley layout is cached by egui, this is cheap).
    // ALL TEXT blocks measure, revealed or not — steps never reflow the
    // stack. Media blocks are backdrops (painted above): 0 height, so they
    // never enter the vertical text flow. `heights` stays index-parallel to
    // `blocks` so the paint loop can index it directly.
    let mut heights = Vec::with_capacity(slide.blocks.len());
    for block in &slide.blocks {
        let dpl_doc::Block::Text {
            text, font, size, ..
        } = block
        else {
            heights.push(0.0);
            continue;
        };
        let px = size_of(&fluid, size).unwrap_or_else(|| fluid.fs_body());
        let galley = painter.layout(
            text.clone(),
            egui::FontId::new(px, family_of(font.as_deref().unwrap_or(""))),
            cl::ink::BASE,
            wrap,
        );
        heights.push(galley.size().y);
    }
    let gap = fluid.sp_4();
    let text_count = slide
        .blocks
        .iter()
        .filter(|b| matches!(b, dpl_doc::Block::Text { .. }))
        .count();
    let total: f32 = heights.iter().sum::<f32>() + gap * text_count.saturating_sub(1) as f32;

    // Pass 2 — the entrance (rise, staggered) and the real paint.
    let rising = slide.enter.as_deref() == Some("rise");
    let fx = cl::motion::ENTER_RISE;
    if fx.in_flight(now, t0, slide.blocks.len()) && (rising || step.is_some_and(|s| s > 0)) {
        ui.ctx().request_repaint();
    }
    let mut map = RenderMap {
        background: rect,
        blocks: Vec::with_capacity(slide.blocks.len()),
    };
    let mut y = rect.center().y - total * 0.5;
    for (i, block) in slide.blocks.iter().enumerate() {
        let dpl_doc::Block::Text {
            id,
            text,
            font,
            size,
            ink,
        } = block
        else {
            // Media blocks painted in the pre-pass. They own the whole stage
            // for hit-testing; they take no slot in the text flow (no y
            // advance), so text layout is identical with or without media.
            map.blocks.push((block.id().to_string(), rect));
            continue;
        };
        // The stable rect (rest position) — hit-testing ignores the
        // one-time entrance lift so a tap lands where the text settles.
        let block_rect =
            Rect::from_min_size(egui::pos2(rect.min.x + pad, y), egui::vec2(0.0, heights[i]));
        let rp = reveal_pos(id);
        let visible = visible_at(id);
        // Animation policy: wall mode / step 0 = the slide entrance
        // staggering all VISIBLE blocks; later steps animate only the
        // block that just revealed.
        let p = match step {
            None | Some(0) => {
                if rising {
                    fx.at(now, t0, i)
                } else {
                    1.0
                }
            }
            Some(s) => {
                if rp == Some(s - 1) {
                    fx.at(now, t0, 0)
                } else {
                    1.0
                }
            }
        };
        let px = size_of(&fluid, size).unwrap_or_else(|| fluid.fs_body());
        let color = ink_of(ink.as_deref().unwrap_or("base"), accent).gamma_multiply(p);
        let galley = painter.layout(
            text.clone(),
            egui::FontId::new(px, family_of(font.as_deref().unwrap_or(""))),
            color,
            wrap,
        );
        let block_rect = Rect::from_min_size(
            block_rect.min,
            egui::vec2(galley.size().x.max(wrap * 0.15), heights[i]),
        );
        map.blocks.push((id.clone(), block_rect));
        if visible {
            let lift = fluid.sp_5() * (1.0 - p);
            painter.galley(egui::pos2(rect.min.x + pad, y + lift), galley, color);
        }
        y += heights[i] + gap;
    }
    map
}

/// A document with its own clock — the wall/stage wrapper.
pub struct DocSlide {
    pub slide: dpl_doc::Slide,
    look: dpl_doc::Look,
    clock: Clock,
}

impl DocSlide {
    /// Standalone document — resolves against the fallback look.
    pub fn new(slide: dpl_doc::Slide) -> Self {
        Self::with_look(slide, dpl_doc::Look::fallback())
    }

    /// A document inside a show — resolves against the show's look.
    pub fn with_look(slide: dpl_doc::Slide, look: dpl_doc::Look) -> Self {
        Self {
            slide,
            look,
            clock: Clock::default(),
        }
    }

    pub fn restart(&mut self) {
        self.clock.restart();
    }

    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) {
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        render(ui, rect, &self.slide, &self.look, now, t0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_documented_size_ref_resolves_and_unknowns_degrade() {
        let fluid = cl::Fluid::of(Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 720.0),
        ));
        for name in [
            "eyebrow",
            "body-sm",
            "body",
            "body-lg",
            "h4",
            "h3",
            "h2",
            "h1",
            "display",
            "display-xl",
        ] {
            assert!(size_of(&fluid, name).is_some(), "size {name} must resolve");
        }
        assert!(size_of(&fluid, "gigantic").is_none());
        assert_eq!(family_of("not-in-the-pool"), egui::FontFamily::Proportional);
        assert_eq!(ink_of("chartreuse", Color32::RED), cl::ink::BASE);
    }

    /// A document full of UNKNOWN refs must render without panicking — the
    /// degrade law, proven through a real theme'd context (an unregistered
    /// font family would panic at layout if `family_of` didn't guard).
    #[test]
    fn a_document_of_unknown_refs_renders_degraded_not_crashed() {
        let slide = dpl_doc::Slide {
            v: dpl_doc::SCHEMA_V,
            template: "ghost".into(),
            background: Some(dpl_doc::Background::MoodBlend {
                moods: vec!["no-such-mood".into(), "also-missing".into()],
                motion: "warp".into(),
                ambience: Some("strobe".into()),
            }),
            blocks: vec![dpl_doc::Block::Text {
                id: "t".into(),
                text: "still here".into(),
                font: Some("comic-sans".into()),
                size: "gigantic".into(),
                ink: Some("chartreuse".into()),
            }],
            enter: Some("rise".into()),
            reveals: Vec::new(),
        };
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                render(ui, rect, &slide, &dpl_doc::Look::fallback(), 1.0, 0.0);
            });
        });
    }

    /// The cascade at the paint layer: a slide with NO styling of its own
    /// renders through a look without panicking (W3b).
    #[test]
    fn a_sparse_slide_renders_with_the_looks_styling() {
        let sparse = dpl_doc::Slide {
            v: dpl_doc::SCHEMA_V,
            template: "title".into(),
            background: None,
            blocks: vec![dpl_doc::Block::Text {
                id: "t".into(),
                text: "styled by the show".into(),
                font: None,
                size: "h1".into(),
                ink: None,
            }],
            enter: None,
            reveals: Vec::new(),
        };
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                render(ui, rect, &sparse, &dpl_doc::Look::fallback(), 1.0, 0.0);
            });
        });
    }

    /// The beat renderer (D2): every step of a reveal slide paints without
    /// panicking, and the RenderMap's block rects are IDENTICAL across
    /// steps — reveals never reflow the stack.
    #[test]
    fn render_step_is_stable_across_every_beat() {
        let slide = dpl_doc::Slide {
            v: dpl_doc::SCHEMA_V,
            template: "title".into(),
            background: None,
            blocks: vec![
                dpl_doc::Block::Text {
                    id: "title".into(),
                    text: "Always there".into(),
                    font: None,
                    size: "h1".into(),
                    ink: None,
                },
                dpl_doc::Block::Text {
                    id: "sub".into(),
                    text: "Revealed on beat one".into(),
                    font: None,
                    size: "h3".into(),
                    ink: None,
                },
            ],
            enter: None,
            reveals: vec!["sub".into()],
        };
        assert_eq!(slide.step_count(), 2);
        let look = dpl_doc::Look::fallback();
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let mut maps: Vec<RenderMap> = Vec::new();
        for step in 0..slide.step_count() {
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.max_rect();
                    maps.push(render_step(ui, rect, &slide, &look, step, 5.0, 0.0));
                });
            });
        }
        let rects = |m: &RenderMap| m.blocks.clone();
        assert_eq!(rects(&maps[0]), rects(&maps[1]), "steps must not reflow");
        assert_eq!(maps[0].blocks.len(), 2, "hidden blocks still hit-test");
    }

    /// W11: a media slide renders through the ONE path — a full-bleed media
    /// backdrop with a text caption on top — without panicking even when the
    /// asset is missing (degrade to the labeled plate). The media block owns
    /// the whole stage for hit-testing; the caption still stacks as text.
    #[test]
    fn a_media_slide_renders_full_bleed_and_hit_tests_the_stage() {
        let slide = dpl_doc::Slide {
            v: dpl_doc::SCHEMA_V,
            template: "media".into(),
            background: Some(dpl_doc::Background::None),
            blocks: vec![
                dpl_doc::Block::Media {
                    id: "shot".into(),
                    show: "image".into(),
                    src: "does/not/exist.png".into(), // degrades to the plate
                    fit: Some("cover".into()),
                    poster: None,
                },
                dpl_doc::Block::Text {
                    id: "cap".into(),
                    text: "caption over media".into(),
                    font: None,
                    size: "h3".into(),
                    ink: None,
                },
            ],
            enter: None,
            reveals: Vec::new(),
        };
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let mut map = RenderMap::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                map = render_traced(ui, rect, &slide, &dpl_doc::Look::fallback(), 2.0, 0.0);
                let media_rect = map
                    .blocks
                    .iter()
                    .find(|(id, _)| id == "shot")
                    .map(|(_, r)| *r);
                assert_eq!(media_rect, Some(rect), "media hit-tests the full stage");
            });
        });
        assert_eq!(map.blocks.len(), 2, "media + caption both present");
    }

    /// Every SEED renders through the real pipeline (registries + theme).
    #[test]
    fn every_seed_renders_through_the_one_path() {
        let seeds = dpl_doc::list_seeds();
        assert!(!seeds.is_empty(), "seed set missing");
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        for s in seeds {
            let (id, dir) = (s.id, s.dir);
            let slide = dpl_doc::load(&dir).unwrap_or_else(|e| panic!("seed {id}: {e}"));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.max_rect();
                    render(ui, rect, &slide, &dpl_doc::Look::fallback(), 2.0, 0.0);
                });
            });
        }
    }
}
