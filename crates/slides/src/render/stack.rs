//! The one paint path: a slide's lines in its component's layout — the
//! column, its container, its decorations — measured, shrunk to fit, painted.

use super::background::background;
use super::refs::{family_of, size_of};
use super::{laid_out, Playhead, RenderMap, Stage};
use crate::elements::divider::rule;
use crate::elements::media::{media_block_backdrop, PictureLook};
use dpl_coldlight::{self as cl};
use dpl_doc::component::{Anchor, Container, Mark, Place};
use egui::{Color32, Rect, Ui};

pub(super) fn render_inner(
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
