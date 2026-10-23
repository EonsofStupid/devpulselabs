//! The document renderer — layer 0 meets layer 3: a `dpl_doc::Slide` (refs,
//! never values) resolved through the generated registries and painted.
//! ONE code path renders documents everywhere — the Workbench seeds wall,
//! the editor stage (W4), the deck, thumbnails.
//!
//! Degrade law: an unknown ref never crashes — unknown fonts fall back to
//! the default face, unknown sizes to body, unknown inks to base, unknown
//! moods to an honest labeled plate.

use crate::elements::{ambient_backdrop, media_block_backdrop, Ambient, PictureLook};
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
    render_inner(
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
    render_inner(
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
    render_inner(
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
struct Playhead {
    /// `Some(k)` is deck mode at beat k; `None` is wall mode (all visible).
    step: Option<usize>,
    now: f64,
    t0: f64,
    stage: Stage,
}

fn render_inner(
    ui: &mut Ui,
    rect: Rect,
    slide: &dpl_doc::Slide,
    styling: &dpl_doc::Styling,
    at: Playhead,
) -> RenderMap {
    let Playhead {
        step,
        now,
        t0,
        stage,
    } = at;
    // The whole cascade happens HERE, once — the body below paints a
    // fully-specified document exactly as before W3b.
    let slide = &styling.resolve(slide);
    let step = step.map(|s| s.min(slide.step_count() - 1));
    let accent = background(ui, rect, slide.background.as_ref(), now, t0);

    // Where each block sits in the reveal order (None = always visible).
    let reveal_pos = |id: &str| slide.reveals.iter().position(|r| r == id);
    let visible_at = |id: &str| match (step, reveal_pos(id)) {
        (Some(s), Some(p)) => p < s,
        _ => true, // wall mode, or a block outside the reveal order
    };

    let fluid = match stage {
        Stage::Full => cl::Fluid::of(rect),
        Stage::Thumbnail => cl::Fluid::thumbnail(rect),
    };
    // Each block moves by its own entrance (LK3, resolved: its own, its
    // look's, then the slide's): rising blocks stagger in; a block that does
    // not move simply appears.
    let rises = |block: &dpl_doc::Block| {
        block
            .style()
            .is_some_and(|style| style.enter == Some("rise"))
    };
    let fx = cl::motion::ENTER_RISE;

    // Media pre-pass (W11): media blocks paint under any text, over the
    // mood — filling the stage, or all of the picture letterboxed (LK3b),
    // framed in the picture's colour, entering by its own entrance. Reveals
    // honored like text.
    for block in &slide.blocks {
        if let dpl_doc::Block::Media {
            id,
            show,
            src,
            fit,
            poster,
            tone,
            ..
        } = block
        {
            if visible_at(id) {
                let m = match step {
                    None | Some(0) if rises(block) => fx.sample(now, t0, 0),
                    Some(s) if s > 0 && reveal_pos(id) == Some(s - 1) && rises(block) => {
                        fx.sample(now, t0, 0)
                    }
                    _ => cl::motion::Motion::REST,
                };
                media_block_backdrop(
                    ui,
                    rect,
                    show,
                    src,
                    poster.as_deref(),
                    PictureLook {
                        contain: fit.as_deref() == Some("contain"),
                        frame: tone.as_deref().and_then(cl::hue_accent),
                        alpha: m.alpha,
                        lift: fluid.sp_5() * m.offset.y,
                    },
                );
            }
        }
    }
    let painter = ui.painter_at(rect);
    let pad = fluid.sp_7();
    let wrap = (rect.width() - pad * 2.0).max(40.0);

    // Pass 1 — measure (galley layout is cached by egui, this is cheap).
    // ALL TEXT blocks measure, revealed or not — steps never reflow the
    // stack. Media blocks are backdrops (painted above): 0 height, so they
    // never enter the vertical text flow. `heights` stays index-parallel to
    // `blocks` so the paint loop can index it directly.
    // `fit` scales every line's size together (1.0 = as authored).
    let measure = |fit: f32| -> Vec<f32> {
        slide
            .blocks
            .iter()
            .map(|block| {
                let Some(words) = block.words() else {
                    return 0.0;
                };
                let px = words
                    .size
                    .and_then(|size| size_of(&fluid, size))
                    .unwrap_or_else(|| fluid.fs_body());
                painter
                    .layout(
                        words.text.to_owned(),
                        egui::FontId::new(px * fit, family_of(words.font.unwrap_or(""))),
                        cl::ink::BASE,
                        wrap,
                    )
                    .size()
                    .y
            })
            .collect()
    };
    let gap = fluid.sp_4();
    let text_count = slide.blocks.iter().filter(|b| b.words().is_some()).count();
    let total_of =
        |heights: &[f32]| heights.iter().sum::<f32>() + gap * text_count.saturating_sub(1) as f32;
    // Shrink to fit: lines taller than the stage scale down together rather
    // than run off it, so a look's big size never clips the words (LK3).
    // Wrapping changes with size, so a second pass settles it.
    let room = (rect.height() - pad * 2.0).max(1.0);
    let mut fit = 1.0_f32;
    let mut heights = measure(fit);
    let mut total = total_of(&heights);
    for _ in 0..2 {
        if total <= room {
            break;
        }
        fit = (fit * room / total).max(0.35);
        heights = measure(fit);
        total = total_of(&heights);
    }

    // Pass 2 — the entrance and the real paint (each block by its own
    // entrance, above).
    if fx.in_flight(now, t0, slide.blocks.len())
        && (slide.blocks.iter().any(rises) || step.is_some_and(|s| s > 0))
    {
        ui.ctx().request_repaint();
    }
    let mut map = RenderMap {
        background: rect,
        blocks: Vec::with_capacity(slide.blocks.len()),
    };
    let mut y = rect.center().y - total * 0.5;
    for (i, block) in slide.blocks.iter().enumerate() {
        let Some(dpl_doc::Words {
            id,
            text,
            font,
            size,
            ink,
            tone,
            ..
        }) = block.words()
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
        // Animation policy: wall mode / step 0 = the entrance staggering
        // every VISIBLE rising line; later steps animate only the line that
        // just revealed — and only if it rises.
        let m = match step {
            None | Some(0) if rises(block) => fx.sample(now, t0, i),
            Some(s) if s > 0 && rp == Some(s - 1) && rises(block) => fx.sample(now, t0, 0),
            _ => cl::motion::Motion::REST,
        };
        let px = size
            .and_then(|size| size_of(&fluid, size))
            .unwrap_or_else(|| fluid.fs_body())
            * fit;
        // The "tone" ink paints the line's own colour (LK3), else the
        // background's accent.
        let line_accent = tone.and_then(cl::hue_accent).unwrap_or(accent);
        let color = cl::ink::named(ink.unwrap_or("base"), line_accent).gamma_multiply(m.alpha);
        let galley = painter.layout(
            text.to_owned(),
            egui::FontId::new(px, family_of(font.unwrap_or(""))),
            color,
            wrap,
        );
        let block_rect = Rect::from_min_size(
            block_rect.min,
            egui::vec2(galley.size().x.max(wrap * 0.15), heights[i]),
        );
        map.blocks.push((id.to_owned(), block_rect));
        if visible {
            let lift = fluid.sp_5() * m.offset.y;
            painter.galley(egui::pos2(rect.min.x + pad, y + lift), galley, color);
        }
        y += heights[i] + gap;
    }
    map
}

/// A document with its own clock — the wall/stage wrapper.
pub struct DocSlide {
    pub slide: dpl_doc::Slide,
    styling: dpl_doc::Styling,
    clock: Clock,
}

impl DocSlide {
    /// A document outside any show — the fallback theme, with `looks`
    /// (the built-in looks plus the operator's own).
    pub fn new(slide: dpl_doc::Slide, looks: dpl_doc::Looks) -> Self {
        Self::styled(
            slide,
            dpl_doc::Styling {
                theme: dpl_doc::Theme::fallback(),
                looks,
            },
        )
    }

    /// A document inside a show — its theme and looks.
    pub fn styled(slide: dpl_doc::Slide, styling: dpl_doc::Styling) -> Self {
        Self {
            slide,
            styling,
            clock: Clock::default(),
        }
    }

    pub fn restart(&mut self) {
        self.clock.restart();
    }

    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) {
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        render(ui, rect, &self.slide, &self.styling, now, t0);
    }

    /// Draw this document as a miniature — see [`Stage::Thumbnail`].
    pub fn thumbnail_ui(&mut self, ui: &mut Ui, rect: Rect) {
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        render_thumbnail(ui, rect, &self.slide, &self.styling, now, t0);
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
        assert_eq!(cl::ink::named("chartreuse", Color32::RED), cl::ink::BASE);
    }

    /// A text block with an explicit size — the fixtures' common shape.
    fn sized(id: &str, text: &str, size: &str) -> dpl_doc::Block {
        dpl_doc::Block::Text {
            id: id.into(),
            preset: None,
            tone: None,
            text: text.into(),
            font: None,
            size: Some(size.into()),
            ink: None,
            enter: None,
            unlinked_from: None,
        }
    }

    /// A document full of UNKNOWN refs must render without panicking — the
    /// degrade law, proven through a real theme'd context (an unregistered
    /// font family would panic at layout if `family_of` didn't guard).
    #[test]
    fn a_document_of_unknown_refs_renders_degraded_not_crashed() {
        let mut slide = dpl_doc::Slide::new("ghost@1");
        slide.background = Some(dpl_doc::Background::MoodBlend {
            moods: vec!["no-such-mood".into(), "also-missing".into()],
            motion: "warp".into(),
            ambience: Some("strobe".into()),
        });
        slide.blocks = vec![dpl_doc::Block::Text {
            id: "t".into(),
            preset: None,
            tone: None,
            text: "still here".into(),
            font: Some("comic-sans".into()),
            size: Some("gigantic".into()),
            ink: Some("chartreuse".into()),
            enter: None,
            unlinked_from: None,
        }];
        slide.enter = Some("rise".into());
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                render(ui, rect, &slide, &dpl_doc::Styling::standalone(), 1.0, 0.0);
            });
        })
        .textures_delta
        .clear();
    }

    /// The cascade at the paint layer: a slide with NO styling of its own
    /// renders through a theme without panicking (W3b).
    #[test]
    fn a_sparse_slide_renders_with_the_looks_styling() {
        let mut sparse = dpl_doc::Slide::new("title@1");
        // No size either: the paint path falls back to body size.
        sparse.blocks = vec![dpl_doc::Block::text("t", "styled by the show")];
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                render(ui, rect, &sparse, &dpl_doc::Styling::standalone(), 1.0, 0.0);
            });
        })
        .textures_delta
        .clear();
    }

    /// The beat renderer (D2): every step of a reveal slide paints without
    /// panicking, and the RenderMap's block rects are IDENTICAL across
    /// steps — reveals never reflow the stack.
    #[test]
    fn render_step_is_stable_across_every_beat() {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.blocks = vec![
            sized("title", "Always there", "h1"),
            sized("sub", "Revealed on beat one", "h3"),
        ];
        slide.reveals = vec!["sub".into()];
        assert_eq!(slide.step_count(), 2);
        let styling = dpl_doc::Styling::standalone();
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let mut maps: Vec<RenderMap> = Vec::new();
        for step in 0..slide.step_count() {
            ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.max_rect();
                    maps.push(render_step(ui, rect, &slide, &styling, step, 5.0, 0.0));
                });
            })
            .textures_delta
            .clear();
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
        let mut slide = dpl_doc::Slide::new("media@1");
        slide.background = Some(dpl_doc::Background::None);
        slide.blocks = vec![
            dpl_doc::Block::Media {
                id: "shot".into(),
                preset: None,
                tone: None,
                show: "image".into(),
                src: "does/not/exist.png".into(), // degrades to the plate
                fit: Some("cover".into()),
                poster: None,
                enter: None,
                unlinked_from: None,
            },
            sized("cap", "caption over media", "h3"),
        ];
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let mut map = RenderMap::default();
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                map = render_traced(ui, rect, &slide, &dpl_doc::Styling::standalone(), 2.0, 0.0);
                let media_rect = map
                    .blocks
                    .iter()
                    .find(|(id, _)| id == "shot")
                    .map(|(_, r)| *r);
                assert_eq!(media_rect, Some(rect), "media hit-tests the full stage");
            });
        })
        .textures_delta
        .clear();
        assert_eq!(map.blocks.len(), 2, "media + caption both present");
    }

    /// Each line moves by its own entrance (LK3): mid-entrance a rising
    /// line is lifted from where it rests, a still line is already there.
    #[test]
    fn a_line_moves_by_its_own_entrance() {
        let line = |id: &str, enter: &str| {
            let mut block = sized(id, id, "h3");
            if let Some(words) = block.words_mut() {
                *words.enter = Some(enter.into());
            }
            block
        };
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.background = Some(dpl_doc::Background::None);
        slide.blocks = vec![line("still", "none"), line("riser", "rise")];
        // Where each line's words are painted, at `now` seconds after the
        // entrance started.
        let painted_at = |now: f64| -> Vec<f32> {
            let ctx = egui::Context::default();
            cl::apply_theme(&ctx);
            let mut out = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.max_rect();
                    render(ui, rect, &slide, &dpl_doc::Styling::standalone(), now, 0.0);
                });
            });
            out.textures_delta.clear();
            out.shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.pos.y),
                    _ => None,
                })
                .collect()
        };
        let (mid, rest) = (painted_at(0.05), painted_at(30.0));
        assert_eq!(mid.len(), 2, "both lines paint");
        assert_eq!(mid[0], rest[0], "the still line is already where it rests");
        assert_ne!(mid[1], rest[1], "the rising line is still on its way");
    }

    /// Shrink to fit (LK3): a long line at a big size stays on the stage —
    /// every line's words are painted inside it, none run off the top.
    #[test]
    fn lines_taller_than_the_stage_shrink_to_fit() {
        let mut slide = dpl_doc::Slide::new("quote@1");
        slide.background = Some(dpl_doc::Background::None);
        slide.blocks = vec![
            sized(
                "quote",
                "The bottleneck was never typing code. It was deciding what to build — and now the machine waits on exactly that.",
                "display",
            ),
            sized("name", "A working developer", "h2"),
        ];
        let stage = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 180.0));
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render(
                    ui,
                    stage,
                    &slide,
                    &dpl_doc::Styling::standalone(),
                    30.0,
                    0.0,
                );
            });
        });
        out.textures_delta.clear();
        let words: Vec<Rect> = out
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.rect.translate(text.pos.to_vec2())),
                _ => None,
            })
            .collect();
        assert_eq!(words.len(), 2);
        for rect in words {
            assert!(
                rect.min.y >= stage.min.y - 0.5 && rect.max.y <= stage.max.y + 0.5,
                "{rect:?} runs off {stage:?}"
            );
        }
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
            ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.max_rect();
                    render(ui, rect, &slide, &dpl_doc::Styling::standalone(), 2.0, 0.0);
                });
            })
            .textures_delta
            .clear();
        }
    }
}
