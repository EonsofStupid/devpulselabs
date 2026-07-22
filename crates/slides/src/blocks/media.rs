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
    /// Slow diagonal tone washes drifting across the surface (ambient-drift).
    Drift,
    /// A constellation of three breathing ring centers (hotspot-pulse loop).
    Pulse,
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
            // Real playback (B1): ffmpeg-subprocess loop → texture. Honest
            // degradation ladder: frames → poster still → labeled plate.
            match dpl_video::locate_asset(path) {
                Some(file) => {
                    if !dpl_video::loop_backdrop(ui, rect, &file) {
                        let poster = dpl_video::poster_of(&file);
                        if poster.is_file() {
                            let uri = poster.to_string_lossy().replace('\\', "/");
                            egui::Image::new(format!("file://{uri}"))
                                .corner_radius(cl::RADIUS)
                                .paint_at(ui, rect);
                        } else {
                            painter.rect_filled(rect, cl::RADIUS, cl::surface::RAISED);
                            painter.text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                format!("video · {path} · waiting for ffmpeg"),
                                egui::FontId::monospace(12.0),
                                cl::ink::SUBTLE,
                            );
                        }
                    }
                    // Legibility diffusion, same as the image slot.
                    ui.painter().rect_filled(rect, cl::RADIUS, egui::Color32::from_black_alpha(120));
                }
                None => {
                    painter.rect_filled(rect, cl::RADIUS, cl::surface::RAISED);
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        format!("video · {path} · file not found"),
                        egui::FontId::monospace(12.0),
                        cl::ink::SUBTLE,
                    );
                }
            }
        }
        Media::Anim(Ambient::Drift) => {
            // Three slanted tone bands crossing the surface on a 12s loop —
            // per-vertex tinted parallelograms, crisp edges, never blur.
            let fx = cl::motion::AMBIENT_DRIFT;
            let c = tone.color();
            for k in 0..3u32 {
                let p = fx.at(now + (k as f64) * (fx.dur as f64) / 3.0, t0, 0);
                let w = rect.width();
                let band_w = w * 0.22;
                let x = rect.min.x - band_w + (w + band_w * 2.0) * p;
                let slant = rect.height() * 0.35;
                let alpha = 14 + k as u8 * 4;
                let tint = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha);
                let mut mesh = egui::epaint::Mesh::default();
                let v = |px: f32, py: f32| egui::epaint::Vertex {
                    pos: egui::pos2(px, py),
                    uv: egui::epaint::WHITE_UV,
                    color: tint,
                };
                mesh.vertices.push(v(x, rect.min.y));
                mesh.vertices.push(v(x + band_w, rect.min.y));
                mesh.vertices.push(v(x - slant, rect.max.y));
                mesh.vertices.push(v(x + band_w - slant, rect.max.y));
                mesh.indices.extend_from_slice(&[0, 1, 2, 2, 1, 3]);
                painter.add(egui::Shape::mesh(mesh));
            }
            ui.ctx().request_repaint();
        }
        Media::Anim(Ambient::Pulse) => {
            // The constellation: three phase-offset ring centers.
            let fx = cl::motion::HOTSPOT_PULSE;
            let c = tone.color();
            let anchors = [(0.25f32, 0.35f32), (0.72, 0.28), (0.55, 0.72)];
            for (k, (ax, ay)) in anchors.iter().enumerate() {
                let center = egui::pos2(
                    rect.min.x + rect.width() * ax,
                    rect.min.y + rect.height() * ay,
                );
                let max_r = rect.width().min(rect.height()) * 0.22;
                for j in 0..2u32 {
                    let phase = (k as f64) * 0.5 + (j as f64) * (fx.dur as f64) / 2.0;
                    let p = fx.at(now + phase, t0, 0);
                    let r = max_r * (0.3 + 0.7 * p);
                    let alpha = (40.0 * (1.0 - p)) as u8;
                    painter.add(egui::Shape::mesh(arc_stroke(
                        center,
                        r,
                        1.5,
                        0.0,
                        TAU,
                        egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
                    )));
                }
            }
            ui.ctx().request_repaint();
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
