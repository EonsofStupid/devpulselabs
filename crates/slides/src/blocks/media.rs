//! Media — the drop-in slot block (S1b). A slide takes `Option<Media>` and
//! this block composes it with the template: a pre-made vector ambience, an
//! image, or (once the engine pass lands frames — ADR-009) a video. Adding
//! media to a slide is DATA, not new paint code.

use dpl_coldlight::{self as cl, Tone};
use dpl_vector::arc_stroke;
use egui::{Align2, Rect, Ui};
use std::f32::consts::TAU;

/// Pre-made vector ambiences — named, pool-colored, motion-runtime driven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ambient {
    /// Concentric rings breathing outward behind content (hotspot-pulse loop).
    Rings,
}

/// What a slide's media slot can carry.
#[derive(Debug, Clone, Copy)]
pub enum Media {
    /// Absolute or repo-relative path, rendered via the installed file loader.
    Image(&'static str),
    /// Honest placeholder until the engine pass delivers frames (ADR-009).
    Video(&'static str),
    /// A named pre-made vector animation.
    Anim(Ambient),
}

/// Paint the media as a BACKDROP inside `rect` (under the template's
/// content). `now`/`t0` come from the slide's clock so replay restarts
/// ambience too.
pub fn media_backdrop(ui: &mut Ui, rect: Rect, media: Media, tone: Tone, now: f64, t0: f64) {
    let painter = ui.painter();
    match media {
        Media::Image(path) => {
            egui::Image::new(format!("file://{path}"))
                .corner_radius(cl::RADIUS)
                .paint_at(ui, rect);
            // Legibility: the dark diffusion the vocabulary demands, not blur.
            painter.rect_filled(rect, cl::RADIUS, egui::Color32::from_black_alpha(120));
        }
        Media::Video(path) => {
            painter.rect_filled(rect, cl::RADIUS, cl::surface::RAISED);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("video · {path} · engine pass (ADR-009)"),
                egui::FontId::monospace(12.0),
                cl::ink::SUBTLE,
            );
        }
        Media::Anim(Ambient::Rings) => {
            // Three breathing rings, phase-offset thirds of the loop.
            let fx = cl::motion::HOTSPOT_PULSE;
            let c = tone.color();
            let center = rect.center();
            let max_r = rect.width().min(rect.height()) * 0.48;
            for k in 0..3u32 {
                let p = fx.at(now + (k as f64) * (fx.dur as f64) / 3.0, t0, 0);
                let r = max_r * (0.35 + 0.65 * p);
                let alpha = (46.0 * (1.0 - p)) as u8;
                painter.add(egui::Shape::mesh(arc_stroke(
                    center,
                    r,
                    2.0,
                    0.0,
                    TAU,
                    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
                )));
            }
            ui.ctx().request_repaint(); // looped ambience never rests
        }
    }
}
