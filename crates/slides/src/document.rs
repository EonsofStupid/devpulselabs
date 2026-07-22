//! The document renderer — layer 0 meets layer 3: a `dpl_doc::Slide` (refs,
//! never values) resolved through the generated registries and painted.
//! ONE code path renders documents everywhere — the Workbench seeds wall,
//! the editor stage (W4), the deck, thumbnails.
//!
//! Degrade law: an unknown ref never crashes — unknown fonts fall back to
//! the default face, unknown sizes to body, unknown inks to base, unknown
//! moods to an honest labeled plate.

use crate::blocks::{ambient_backdrop, Ambient};
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
/// color (the "tone" ink and the ambience ride it).
fn background(ui: &mut Ui, rect: Rect, slide: &dpl_doc::Slide, now: f64, t0: f64) -> Color32 {
    let find = |name: &str| cl::MOODS.iter().find(|m| m.name == name);
    let painter = ui.painter_at(rect);
    match &slide.background {
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

/// Render a document into `rect`. The ONE path — every surface calls this.
pub fn render(ui: &mut Ui, rect: Rect, slide: &dpl_doc::Slide, now: f64, t0: f64) {
    let accent = background(ui, rect, slide, now, t0);

    let fluid = cl::Fluid::of(rect);
    let painter = ui.painter_at(rect);
    let pad = fluid.sp_7();
    let wrap = (rect.width() - pad * 2.0).max(40.0);

    // Pass 1 — measure (galley layout is cached by egui, this is cheap).
    let mut heights = Vec::with_capacity(slide.blocks.len());
    for block in &slide.blocks {
        let dpl_doc::Block::Text {
            text, font, size, ..
        } = block;
        let px = size_of(&fluid, size).unwrap_or_else(|| fluid.fs_body());
        let galley = painter.layout(
            text.clone(),
            egui::FontId::new(px, family_of(font)),
            cl::ink::BASE,
            wrap,
        );
        heights.push(galley.size().y);
    }
    let gap = fluid.sp_4();
    let total: f32 = heights.iter().sum::<f32>() + gap * heights.len().saturating_sub(1) as f32;

    // Pass 2 — the entrance (rise, staggered) and the real paint.
    let rising = slide.enter.as_deref() == Some("rise");
    let fx = cl::motion::ENTER_RISE;
    if rising && fx.in_flight(now, t0, slide.blocks.len()) {
        ui.ctx().request_repaint();
    }
    let mut y = rect.center().y - total * 0.5;
    for (i, block) in slide.blocks.iter().enumerate() {
        let dpl_doc::Block::Text {
            text,
            font,
            size,
            ink,
            ..
        } = block;
        let p = if rising { fx.at(now, t0, i) } else { 1.0 };
        let px = size_of(&fluid, size).unwrap_or_else(|| fluid.fs_body());
        let color = ink_of(ink, accent).gamma_multiply(p);
        let galley = painter.layout(
            text.clone(),
            egui::FontId::new(px, family_of(font)),
            color,
            wrap,
        );
        let lift = fluid.sp_5() * (1.0 - p);
        painter.galley(egui::pos2(rect.min.x + pad, y + lift), galley, color);
        y += heights[i] + gap;
    }
}

/// A document with its own clock — the wall/stage wrapper.
pub struct DocSlide {
    pub slide: dpl_doc::Slide,
    clock: Clock,
}

impl DocSlide {
    pub fn new(slide: dpl_doc::Slide) -> Self {
        Self {
            slide,
            clock: Clock::default(),
        }
    }

    pub fn restart(&mut self) {
        self.clock.restart();
    }

    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) {
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        render(ui, rect, &self.slide, now, t0);
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
            background: dpl_doc::Background::MoodBlend {
                moods: vec!["no-such-mood".into(), "also-missing".into()],
                motion: "warp".into(),
                ambience: Some("strobe".into()),
            },
            blocks: vec![dpl_doc::Block::Text {
                id: "t".into(),
                text: "still here".into(),
                font: "comic-sans".into(),
                size: "gigantic".into(),
                ink: "chartreuse".into(),
            }],
            enter: Some("rise".into()),
        };
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                render(ui, rect, &slide, 1.0, 0.0);
            });
        });
    }

    /// Every SEED renders through the real pipeline (registries + theme).
    #[test]
    fn every_seed_renders_through_the_one_path() {
        let seeds = dpl_doc::list_seeds();
        assert!(!seeds.is_empty(), "seed set missing");
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        for (id, dir) in seeds {
            let slide = dpl_doc::load(&dir).unwrap_or_else(|e| panic!("seed {id}: {e}"));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = ui.max_rect();
                    render(ui, rect, &slide, 2.0, 0.0);
                });
            });
        }
    }
}
