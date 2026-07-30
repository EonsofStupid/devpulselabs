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
    match media {
        Media::Image(path) => paint_image(ui, rect, path),
        Media::Video(path) => paint_video(ui, rect, path, None),
        Media::Anim(amb) => ambient_backdrop(ui, rect, amb, tone.color(), now, t0),
    }
}

/// Paint a media BLOCK straight from a document's owned refs (W11) — the
/// `&'static Media` enum above is the template/demo path; `dpl_doc::Block::
/// Media` carries owned Strings. `show` is "image" | "video" (validated at
/// the doc layer; an unknown value falls through to a labeled plate). `fit`
/// is carried for a later precise-letterbox pass — today both modes fill the
/// stage. Ambience is the template's affordance, not a document's.
pub fn media_block_backdrop(
    ui: &mut Ui,
    rect: Rect,
    show: &str,
    src: &str,
    _fit: Option<&str>,
    poster: Option<&str>,
) {
    match show {
        "video" => paint_video(ui, rect, src, poster),
        // "image" and any unrecognized value degrade to the image ladder
        // (locate → paint → labeled plate), never a crash.
        _ => paint_image(ui, rect, src),
    }
}

/// Paint one content asset as a THUMBNAIL — for a picker, not a card.
///
/// Same ladder as the card's own image path (locate → paint → labeled plate)
/// and deliberately the same `file:///` handling, because the three-slash
/// detail is a Windows UNC trap and belongs in exactly one place.
///
/// The difference from [`media_block_backdrop`]: no legibility diffusion. A
/// backdrop is dimmed so words can sit on it; a picker tile IS the subject and
/// dimming it would misrepresent what the operator is choosing.
pub fn asset_thumb(ui: &mut Ui, rect: Rect, src: &str) {
    match dpl_video::locate_asset(src) {
        Some(file) => {
            let uri = file.to_string_lossy().replace('\\', "/");
            egui::Image::new(format!("file:///{uri}"))
                .corner_radius(cl::RADIUS)
                .paint_at(ui, rect);
        }
        None => missing_plate(ui, rect, "file not found"),
    }
}

/// A labeled fallback plate — the honest bottom rung of every ladder.
fn missing_plate(ui: &Ui, rect: Rect, label: &str) {
    let painter = ui.painter();
    painter.rect_filled(rect, cl::RADIUS, cl::surface::RAISED);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(12.0),
        cl::ink::SUBTLE,
    );
}

/// The image ladder: locate → paint (+ legibility diffusion) → labeled plate.
fn paint_image(ui: &mut Ui, rect: Rect, path: &str) {
    match dpl_video::locate_asset(path) {
        Some(file) => {
            // file:/// with THREE slashes — two reads as a UNC host on
            // Windows and the load errors.
            let uri = file.to_string_lossy().replace('\\', "/");
            egui::Image::new(format!("file:///{uri}"))
                .corner_radius(cl::RADIUS)
                .paint_at(ui, rect);
            // Legibility: the dark diffusion the vocabulary demands.
            ui.painter()
                .rect_filled(rect, cl::RADIUS, egui::Color32::from_black_alpha(120));
        }
        None => missing_plate(ui, rect, &format!("image · {path} · file not found")),
    }
}

/// The video ladder (B1): ffmpeg-subprocess loop → poster still → labeled
/// plate. `poster` is an optional explicit ref (W11 documents); absent, the
/// derived `poster_of` still applies.
fn paint_video(ui: &mut Ui, rect: Rect, path: &str, poster: Option<&str>) {
    match dpl_video::locate_asset(path) {
        Some(file) => {
            if !dpl_video::loop_backdrop(ui, rect, &file) {
                // Prefer an explicit poster ref, else the derived sidecar.
                let still = poster
                    .and_then(dpl_video::locate_asset)
                    .filter(|p| p.is_file())
                    .or_else(|| Some(dpl_video::poster_of(&file)).filter(|p| p.is_file()));
                match still {
                    Some(p) => {
                        let uri = p.to_string_lossy().replace('\\', "/");
                        egui::Image::new(format!("file:///{uri}"))
                            .corner_radius(cl::RADIUS)
                            .paint_at(ui, rect);
                    }
                    None => {
                        missing_plate(ui, rect, &format!("video · {path} · waiting for ffmpeg"))
                    }
                }
            }
            // Legibility diffusion, same as the image slot.
            ui.painter()
                .rect_filled(rect, cl::RADIUS, egui::Color32::from_black_alpha(120));
        }
        None => missing_plate(ui, rect, &format!("video · {path} · file not found")),
    }
}

/// Paint a named ambience colored by `color` — media_backdrop's tone path
/// and the document renderer's mood-accent path share this one painter.
pub fn ambient_backdrop(
    ui: &mut Ui,
    rect: Rect,
    amb: Ambient,
    color: egui::Color32,
    now: f64,
    t0: f64,
) {
    let painter = ui.painter();
    match amb {
        Ambient::Drift => {
            // Three slanted tone bands crossing the surface on a 12s loop —
            // per-vertex tinted parallelograms, crisp edges, never blur.
            let fx = cl::motion::AMBIENT_DRIFT;
            let c = color;
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
        Ambient::Pulse => {
            // The constellation: three phase-offset ring centers.
            let fx = cl::motion::HOTSPOT_PULSE;
            let c = color;
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
        Ambient::Rings => {
            // Three breathing rings, phase-offset thirds of the loop.
            let fx = cl::motion::HOTSPOT_PULSE;
            let c = color;
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
