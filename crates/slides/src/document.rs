//! The document renderer — layer 0 meets layer 3: a `dpl_doc::Slide` (refs,
//! never values) resolved through the generated registries and painted.
//! ONE code path renders documents everywhere — the Workbench seeds wall,
//! the editor stage (W4), the deck, thumbnails.
//!
//! Degrade law: an unknown ref never crashes — unknown fonts fall back to
//! the default face, unknown sizes to body, unknown inks to base, unknown
//! moods to an honest labeled plate.

use crate::elements::{ambient_backdrop, media_block_backdrop, rule, Ambient, PictureLook};
use crate::motion::Clock;
use dpl_coldlight::{self as cl};
use dpl_doc::component::{Anchor, Container, Mark, Place};
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
    render_inner(
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
    // The slide's component lays it out (LS4a): its slots in reading order,
    // aligned as its variant says, each slot's default size and ink in the
    // cascade under the slide's own values and looks. No layout: the lines
    // flow in their own order, exactly as before.
    let laid = laid_out(slide);
    // The whole cascade happens HERE, once — the body below paints a
    // fully-specified document exactly as before W3b.
    let mut resolved = styling.resolve_laid_out(slide, laid.as_ref());
    if let Some(laid) = &laid {
        in_reading_order(&mut resolved.blocks, laid);
        // A tracked slot (an eyebrow, a kicker) is drawn in upper case with
        // thin-space tracking — on this painted copy only.
        for block in &mut resolved.blocks {
            let tracked = laid.slot(block.id()).is_some_and(|s| s.tracked);
            if let (true, Some(words)) = (tracked, block.words_mut()) {
                *words.text = crate::elements::eyebrow::tracked(words.text);
            }
        }
    }
    let center = laid
        .as_ref()
        .is_some_and(|l| l.align == dpl_doc::component::Align::Center);
    let slide = &resolved;
    let step = step.map(|s| s.min(slide.step_count() - 1));
    let accent = background(ui, rect, slide.background.as_ref(), now, t0);
    // The card's own colour (its look's, or its component's Colour prop)
    // leads every "tone" line, rule, mark and container bar; else the
    // background's accent. It was never painted before LS4a.
    let card_accent = slide
        .tone
        .as_deref()
        .and_then(cl::hue_accent)
        .unwrap_or(accent);

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
    // The decorations the layout names (LS4a §4b), all in the card's colour:
    // the column's container, an oversized mark, a slot pinned to the top, a
    // rule above a slot, and where the column sits. No layout: none of them,
    // and the column sits in the middle of the stage, exactly as before.
    let slot_of = |block: &dpl_doc::Block| laid.as_ref().and_then(|l| l.slot(block.id()));
    let flow = match laid.as_ref().and_then(|l| l.container) {
        Some(Container::Card) => {
            let card = Rect::from_center_size(
                rect.center(),
                egui::vec2(
                    rect.width() * 0.82,
                    (rect.height() * 0.55).max(fluid.sp_8() * 3.0),
                ),
            );
            paint_card(&painter, card, card_accent, fluid.r_lg());
            card
        }
        None => rect,
    };
    let pad = fluid.sp_7();
    let wrap = (flow.width() - pad * 2.0).max(40.0);
    if laid.as_ref().and_then(|l| l.mark) == Some(Mark::Quote) {
        // ASCII, so every font in the pool has it.
        painter.text(
            flow.min + egui::vec2(pad, pad * 0.5),
            egui::Align2::LEFT_TOP,
            "\"",
            egui::FontId::proportional(fluid.fs_display_xl()),
            Color32::from_rgba_unmultiplied(card_accent.r(), card_accent.g(), card_accent.b(), 90),
        );
    }
    // Words in the column, words pinned to the top, and the column's lines
    // with a rule above them — index-parallel to `blocks`, like `heights`.
    let pinned: Vec<bool> = slide
        .blocks
        .iter()
        .map(|b| b.words().is_some() && slot_of(b).is_some_and(|s| s.place == Some(Place::Top)))
        .collect();
    let in_flow: Vec<bool> = slide
        .blocks
        .iter()
        .zip(&pinned)
        .map(|(b, pin)| b.words().is_some() && !pin)
        .collect();
    let ruled: Vec<Option<dpl_doc::component::Rule>> = slide
        .blocks
        .iter()
        .zip(&in_flow)
        .map(|(b, flows)| flows.then(|| slot_of(b).and_then(|s| s.rule)).flatten())
        .collect();

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
    let flow_count = in_flow.iter().filter(|f| **f).count();
    let pin_count = pinned.iter().filter(|p| **p).count();
    let rule_count = ruled.iter().filter(|r| r.is_some()).count();
    // The column's height, and the pinned lines' (a rule takes one gap).
    let column_of = |heights: &[f32], which: &[bool], count: usize| {
        heights
            .iter()
            .zip(which)
            .filter(|(_, w)| **w)
            .map(|(h, _)| h)
            .sum::<f32>()
            + gap * count.saturating_sub(1) as f32
    };
    let total_of = |heights: &[f32]| {
        let flow_h = column_of(heights, &in_flow, flow_count) + gap * rule_count as f32;
        let pin_h = column_of(heights, &pinned, pin_count);
        (flow_h, pin_h)
    };
    let both = |(flow_h, pin_h): (f32, f32)| {
        flow_h
            + pin_h
            + if pin_count > 0 && flow_count > 0 {
                gap
            } else {
                0.0
            }
    };
    // Shrink to fit: lines taller than the stage scale down together rather
    // than run off it, so a look's big size never clips the words (LK3).
    // Wrapping changes with size, so a second pass settles it.
    let room = (flow.height() - pad * 2.0).max(1.0);
    let mut fit = 1.0_f32;
    let mut heights = measure(fit);
    let mut total = total_of(&heights);
    for _ in 0..2 {
        if both(total) <= room {
            break;
        }
        fit = (fit * room / both(total)).max(0.35);
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
    let (flow_h, pin_h) = total;
    let mut y = match laid.as_ref().map_or(Anchor::Middle, |l| l.anchor) {
        Anchor::Top if pin_count > 0 => flow.min.y + pad + pin_h + gap,
        Anchor::Top => flow.min.y + pad,
        Anchor::Middle => flow.center().y - flow_h * 0.5,
        Anchor::Bottom => flow.max.y - pad - flow_h,
    };
    let mut pin_y = flow.min.y + pad;
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
            // A data block is not painted here (its component paints it,
            // LS4b), so it claims no part of the stage — it used to claim
            // all of it, and a tap could never reach the background.
            if matches!(block, dpl_doc::Block::Media { .. }) {
                map.blocks.push((block.id().to_string(), rect));
            }
            continue;
        };
        let rp = reveal_pos(id);
        let visible = visible_at(id);
        // A rule above the line takes one gap of the column.
        let rule_at = ruled[i].map(|r| {
            let at = y;
            y += gap;
            (r, at)
        });
        let line_y = if pinned[i] { pin_y } else { y };
        // The stable rect (rest position) — hit-testing ignores the
        // one-time entrance lift so a tap lands where the text settles.
        let block_rect = Rect::from_min_size(
            egui::pos2(flow.min.x + pad, line_y),
            egui::vec2(0.0, heights[i]),
        );
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
        // card's, else the background's accent.
        let line_accent = tone.and_then(cl::hue_accent).unwrap_or(card_accent);
        let color = cl::ink::named(ink.unwrap_or("base"), line_accent).gamma_multiply(m.alpha);
        let font = egui::FontId::new(px, family_of(font.unwrap_or("")));
        // Centred lines centre each wrapped row; the galley then sits around
        // the stage's middle (egui lays a centred job out around x = 0).
        let (galley, x) = if center {
            let mut job = egui::text::LayoutJob::simple(text.to_owned(), font, color, wrap);
            job.halign = egui::Align::Center;
            (painter.layout_job(job), flow.center().x)
        } else {
            (
                painter.layout(text.to_owned(), font, color, wrap),
                flow.min.x + pad,
            )
        };
        let block_rect = if center {
            let w = galley.size().x.max(wrap * 0.15);
            Rect::from_min_size(
                egui::pos2(flow.center().x - w * 0.5, line_y),
                egui::vec2(w, heights[i]),
            )
        } else {
            Rect::from_min_size(
                block_rect.min,
                egui::vec2(galley.size().x.max(wrap * 0.15), heights[i]),
            )
        };
        map.blocks.push((id.to_owned(), block_rect));
        if visible {
            let lift = fluid.sp_5() * m.offset.y;
            if let Some((kind, at)) = rule_at {
                let w = match kind {
                    dpl_doc::component::Rule::Half => wrap * 0.5,
                    dpl_doc::component::Rule::Full => wrap,
                };
                let left = if center {
                    flow.center().x - w * 0.5
                } else {
                    flow.min.x + pad
                };
                rule(&painter, left, at, w, card_accent, m.alpha);
            }
            painter.galley(egui::pos2(x, line_y + lift), galley, color);
        }
        if pinned[i] {
            pin_y += heights[i] + gap;
        } else {
            y += heights[i] + gap;
        }
    }
    map
}

/// The `card` container (LS4a §4b, the callout's carried anatomy): a raised
/// surface with a hairline edge, the sheen band, and a bar in the card's
/// colour down its left side.
fn paint_card(painter: &egui::Painter, card: Rect, colour: Color32, radius: f32) {
    let radius = radius as u8;
    painter.rect_filled(card, radius, cl::surface::RAISED);
    painter.rect_stroke(
        card,
        radius,
        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
        egui::StrokeKind::Inside,
    );
    cl::sheen(painter, card);
    painter.rect_filled(
        Rect::from_min_size(card.min, egui::vec2(4.0, card.height())),
        0,
        Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), 200),
    );
}

/// A template's own values (its contract's shape: one key per slot, prop
/// and axis) as a slide document — what its gallery card paints through the
/// one renderer (LS4a: no template hand-paints).
pub fn slide_of(
    contract: &dpl_doc::component::ComponentContract,
    values: &serde_json::Value,
) -> dpl_doc::Slide {
    let pick = |ids: &mut dyn Iterator<Item = &str>| -> serde_json::Map<String, serde_json::Value> {
        ids.filter_map(|id| values.get(id).map(|v| (id.to_owned(), v.clone())))
            .collect()
    };
    let content = pick(&mut contract.slots.iter().map(|s| s.id.as_str()));
    let props = pick(&mut contract.props.iter().map(|p| p.id.as_str()));
    let variant = contract
        .variant_axes
        .iter()
        .filter_map(|axis| {
            values
                .get(&axis.id)
                .and_then(serde_json::Value::as_str)
                .map(|option| (axis.id.clone(), option.to_owned()))
        })
        .collect();
    dpl_doc::slide_from_parts(contract, &content, &props, &variant)
        .unwrap_or_else(|| dpl_doc::Slide::new(format!("{}@1", contract.id)))
}

/// A migrated template's gallery card (LS4a): its own values as a slide,
/// painted by the one renderer against the standalone styling, on the
/// template's clock.
pub(crate) fn paint_template(
    ui: &mut Ui,
    contract: &dpl_doc::component::ComponentContract,
    values: &serde_json::Value,
    clock: &mut crate::motion::Clock,
) {
    static STANDALONE: std::sync::OnceLock<dpl_doc::Styling> = std::sync::OnceLock::new();
    let rect = ui.max_rect();
    let mut slide = slide_of(contract, values);
    // On the bare well, like every gallery card not yet moved: the component,
    // not a theme.
    slide.background.get_or_insert(dpl_doc::Background::None);
    let now = ui.input(|i| i.time);
    let t0 = clock.tick(now);
    render(
        ui,
        rect,
        &slide,
        STANDALONE.get_or_init(dpl_doc::Styling::standalone),
        now,
        t0,
    );
    ui.allocate_rect(rect, egui::Sense::hover());
}

/// The layout a slide's component gives it, with the slide's variant
/// applied (LS4a) — None when the component is not registered or has no
/// layout yet (the lines then flow in their own order).
pub fn laid_out(slide: &dpl_doc::Slide) -> Option<dpl_doc::component::LaidOut> {
    crate::contract_of(slide.component_id())?.laid_out(&slide.variant)
}

/// The slide as the editor and every tray should read it: resolved through
/// its looks, its theme and its component's slot defaults — the values the
/// renderer paints.
pub fn shown(slide: &dpl_doc::Slide, styling: &dpl_doc::Styling) -> dpl_doc::Slide {
    styling.resolve_laid_out(slide, laid_out(slide).as_ref())
}

/// Blocks the layout names first, in its reading order; the rest after, in
/// their own order — nothing the layout does not name ever vanishes.
fn in_reading_order(blocks: &mut [dpl_doc::Block], laid: &dpl_doc::component::LaidOut) {
    let rank = |block: &dpl_doc::Block| {
        laid.slots
            .iter()
            .position(|slot| slot.slot == block.id())
            .unwrap_or(laid.slots.len())
    };
    blocks.sort_by_key(rank);
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
        // The two lines, and the quote's mark (its layout's decoration).
        assert_eq!(words.len(), 3);
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

    /// Render `slide` on a 1280x720 stage and return its map.
    fn mapped(slide: &dpl_doc::Slide) -> (Rect, RenderMap) {
        let stage = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 720.0));
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let mut map = None;
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                map = Some(render_traced(
                    ui,
                    stage,
                    slide,
                    &dpl_doc::Styling::standalone(),
                    30.0,
                    0.0,
                ));
            });
        })
        .textures_delta
        .clear();
        (stage, map.expect("rendered"))
    }

    /// The component lays the slide out (LS4a): its slots in reading order
    /// whatever order the blocks are stored in, and centred when the
    /// variant says so.
    #[test]
    fn a_layout_orders_and_centres_the_lines() {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.background = Some(dpl_doc::Background::None);
        slide.blocks = vec![
            dpl_doc::Block::text("subtitle", "Under it"),
            dpl_doc::Block::text("title", "The headline"),
            dpl_doc::Block::text("eyebrow", "SHOW NAME"),
        ];
        let (stage, map) = mapped(&slide);
        let ids: Vec<&str> = map.blocks.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, ["eyebrow", "title", "subtitle"]);
        let ys: Vec<f32> = map.blocks.iter().map(|(_, r)| r.min.y).collect();
        assert!(ys.windows(2).all(|w| w[0] < w[1]), "top to bottom: {ys:?}");
        // The title is the big line: the slot's default size.
        assert!(map.blocks[1].1.height() > map.blocks[0].1.height() * 2.0);
        slide.variant.insert("align".into(), "center".into());
        let (_, map) = mapped(&slide);
        for (id, rect) in &map.blocks {
            assert!(
                (rect.center().x - stage.center().x).abs() < 1.0,
                "{id} {rect:?}"
            );
        }
    }

    /// A slide whose component has no layout flows as it always did: its
    /// lines in their own order, on the left.
    #[test]
    fn a_slide_without_a_layout_flows_in_its_own_order() {
        let mut slide = dpl_doc::Slide::new("freeform@1");
        slide.background = Some(dpl_doc::Background::None);
        slide.blocks = vec![
            sized("b", "Second written, first shown", "h2"),
            sized("a", "First written", "body"),
        ];
        let (stage, map) = mapped(&slide);
        let ids: Vec<&str> = map.blocks.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, ["b", "a"]);
        let left = stage.min.x + cl::Fluid::of(stage).sp_7();
        for (id, rect) in &map.blocks {
            assert!((rect.min.x - left).abs() < 0.5, "{id} {rect:?}");
        }
    }

    /// A data block is not painted by the flow, so it claims no part of the
    /// stage: the background stays selectable on a list card.
    #[test]
    fn a_data_block_claims_no_part_of_the_stage() {
        let mut slide = dpl_doc::Slide::new("agenda@1");
        slide.background = Some(dpl_doc::Background::None);
        slide.blocks = vec![
            sized("heading", "Today", "h2"),
            dpl_doc::Block::Data {
                id: "items".into(),
                value: serde_json::json!([{"title": "One"}]),
            },
        ];
        let (stage, map) = mapped(&slide);
        assert!(map.blocks.iter().all(|(id, _)| id != "items"));
        assert_eq!(
            map.block_at(stage.right_bottom() - egui::vec2(4.0, 4.0)),
            None
        );
    }

    /// The layout's decorations place the lines (§4b): a section pins its
    /// number to the top and its title to the bottom; a callout's lines sit
    /// inside its card, from the card's top.
    #[test]
    fn the_decorations_place_the_lines() {
        let mut section = dpl_doc::Slide::new("section@1");
        section.background = Some(dpl_doc::Background::None);
        section.blocks = vec![
            dpl_doc::Block::text("title", "The setup"),
            dpl_doc::Block::text("kicker", "Part one"),
            dpl_doc::Block::text("number", "01"),
        ];
        let (stage, map) = mapped(&section);
        let pad = cl::Fluid::of(stage).sp_7();
        let at = |id: &str| map.blocks.iter().find(|(b, _)| b == id).unwrap().1;
        assert!((at("number").min.y - (stage.min.y + pad)).abs() < 0.5);
        assert!((at("title").max.y - (stage.max.y - pad)).abs() < 0.5);
        // The kicker sits above the title, a rule's gap under the number.
        assert!(at("kicker").max.y <= at("title").min.y);
        assert!(at("kicker").min.y > at("number").max.y);

        let mut callout = dpl_doc::Slide::new("callout@1");
        callout.background = Some(dpl_doc::Background::None);
        callout.blocks = vec![
            dpl_doc::Block::text("kicker", "The catch"),
            dpl_doc::Block::text("title", "Free tiers are the demo"),
            dpl_doc::Block::text("body", "Patience is the price."),
        ];
        let (stage, map) = mapped(&callout);
        let card = Rect::from_center_size(
            stage.center(),
            egui::vec2(stage.width() * 0.82, stage.height() * 0.55),
        );
        for (id, rect) in &map.blocks {
            assert!(
                card.shrink(pad - 0.5).contains_rect(*rect),
                "{id} {rect:?} in {card:?}"
            );
        }
        assert!((map.blocks[0].1.min.y - (card.min.y + pad)).abs() < 0.5);
    }

    /// A template's own values become a slide: content into blocks, the
    /// colour into the slide's tone, axes into its variant.
    #[test]
    fn a_templates_values_become_a_slide() {
        let contract = crate::contract_of("title").expect("the title contract");
        let values = serde_json::json!({
            "tone": "coral", "eyebrow": "SHOW", "title": "Hello", "subtitle": "",
            "align": "center", "media": null
        });
        let slide = slide_of(contract, &values);
        assert_eq!(slide.component, "title@1");
        assert_eq!(slide.tone.as_deref(), Some("coral"));
        assert_eq!(
            slide.variant.get("align").map(String::as_str),
            Some("center")
        );
        assert!(slide.block("title").is_some() && slide.block("eyebrow").is_some());
    }
}
