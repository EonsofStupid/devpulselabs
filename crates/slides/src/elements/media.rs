//! Media — the drop-in slot block (S1b). A slide takes `Option<Media>` and
//! this block composes it with the template: a pre-made vector ambience, an
//! image, or (once the engine pass lands frames — ADR-009) a video. Adding
//! media to a slide is DATA, not new paint code.

use dpl_coldlight::{self as cl, Tone};
use dpl_vector::arc_stroke;
use egui::{Align2, Rect, Ui};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Pre-made vector ambiences — named, pool-colored, motion-runtime driven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ambient {
    /// Concentric rings breathing outward behind content (hotspot-pulse loop).
    Rings,
    /// Slow diagonal tone washes drifting across the surface (ambient-drift).
    Drift,
    /// A constellation of three breathing ring centers (hotspot-pulse loop).
    Pulse,
}

/// What a slide's media slot can carry — authored as
/// `{"kind": "image" | "video", "src": …}` or `{"kind": "anim", "ambient": …}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Media {
    /// Absolute or repo-relative path, rendered via the installed file loader.
    Image { src: String },
    /// Honest placeholder until the engine pass delivers frames (ADR-009).
    Video { src: String },
    /// A named pre-made vector animation.
    Anim { ambient: Ambient },
}

/// Paint the media as a BACKDROP inside `rect` (under the template's
/// content). `now`/`t0` come from the slide's clock so replay restarts
/// ambience too.
pub fn media_backdrop(ui: &mut Ui, rect: Rect, media: &Media, tone: Tone, now: f64, t0: f64) {
    match media {
        Media::Image { src } => {
            paint_image(ui, rect, src, PictureLook::FILL);
        }
        Media::Video { src } => {
            paint_video(ui, rect, src, None, PictureLook::FILL);
        }
        Media::Anim { ambient } => ambient_backdrop(ui, rect, *ambient, tone.color(), now, t0),
    }
}

/// How a picture paints (LK3b): how it sits, its frame, and where its
/// entrance is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PictureLook {
    /// Show all of it, letterboxed — else it fills the stage, cropped.
    pub contain: bool,
    /// The frame's colour; `None` paints no frame.
    pub frame: Option<egui::Color32>,
    /// The entrance's fade (1 = fully there) and lift, in points.
    pub alpha: f32,
    pub lift: f32,
}

impl PictureLook {
    /// Fill the stage, no frame, at rest — a template's picture.
    pub const FILL: PictureLook = PictureLook {
        contain: false,
        frame: None,
        alpha: 1.0,
        lift: 0.0,
    };
}

/// Where a picture of `size` lands in `rect`, and which part of it shows:
/// `contain` keeps all of it (letterboxed, centred); otherwise it fills
/// `rect`, cropped evenly. A size not known yet fills `rect` whole.
pub fn fit_picture(rect: Rect, size: Option<egui::Vec2>, contain: bool) -> (Rect, Rect) {
    let whole = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let Some(size) = size.filter(|size| size.x > 0.0 && size.y > 0.0) else {
        return (rect, whole);
    };
    if contain {
        let scale = (rect.width() / size.x).min(rect.height() / size.y);
        (Rect::from_center_size(rect.center(), size * scale), whole)
    } else {
        (
            rect,
            dpl_video::cover_uv(size.x / size.y, rect.aspect_ratio()),
        )
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
    poster: Option<&str>,
    look: PictureLook,
) {
    let rect = rect.translate(egui::vec2(0.0, look.lift));
    let painted = match show {
        "video" => paint_video(ui, rect, src, poster, look),
        // "image" and any unrecognized value degrade to the image ladder
        // (locate → paint → labeled plate), never a crash.
        _ => paint_image(ui, rect, src, look),
    };
    // The frame: a hard edge in the picture's colour — never a glow.
    if let (Some(frame), Some(at)) = (look.frame, painted) {
        ui.painter().rect_stroke(
            at,
            cl::RADIUS,
            egui::Stroke::new(2.0, frame.gamma_multiply(look.alpha)),
            egui::StrokeKind::Inside,
        );
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
    // A linked file whose location is offline shows its cached poster.
    let still = dpl_video::locate_asset(src)
        .or_else(|| dpl_video::location_poster(src).filter(|p| p.is_file()));
    match still {
        Some(file) => {
            let uri = file.to_string_lossy().replace('\\', "/");
            egui::Image::new(format!("file:///{uri}"))
                .corner_radius(cl::RADIUS)
                .paint_at(ui, rect);
        }
        None => missing_plate(ui, rect, "file not found"),
    }
}

/// The audience's screen in Run (ruled 2026-10-27, Q-ST5-3): an offline
/// linked file shows its poster there with no note. The Present hub marks
/// the span it paints the audience's deck in.
pub fn set_audience(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(audience_id(), on));
}

fn audience_id() -> egui::Id {
    egui::Id::new("dpl.slides.audience")
}

/// A linked file whose location is offline (ST5): its cached poster, or a
/// plain plate, and — except on the audience's screen — a note naming
/// where it lives. `None` when `path` isn't a linked file.
fn offline(ui: &mut Ui, rect: Rect, path: &str, look: PictureLook) -> Option<Option<Rect>> {
    let location = dpl_video::location_of(path)?;
    let at = dpl_video::location_poster(path)
        .filter(|p| p.is_file())
        .map(|p| paint_still(ui, rect, &p, look));
    if at.is_none() {
        ui.painter()
            .rect_filled(rect, cl::RADIUS, cl::surface::RAISED);
    }
    let audience = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(audience_id()))
        .unwrap_or(false);
    if !audience {
        let note = format!("On '{}' — plug it in", location.label);
        let font = egui::FontId::proportional(13.0);
        let galley = ui
            .painter()
            .layout_no_wrap(note, font, cl::ink::STRONG);
        let pad = egui::vec2(8.0, 4.0);
        let chip = Rect::from_min_size(
            rect.left_bottom() + egui::vec2(pad.x, -galley.size().y - pad.y * 3.0),
            galley.size() + pad * 2.0,
        );
        ui.painter()
            .rect_filled(chip, cl::RADIUS, egui::Color32::from_black_alpha(180));
        ui.painter()
            .galley(chip.min + pad, galley, cl::ink::STRONG);
    }
    Some(at)
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
fn paint_image(ui: &mut Ui, rect: Rect, path: &str, look: PictureLook) -> Option<Rect> {
    match dpl_video::locate_asset(path) {
        Some(file) => Some(paint_still(ui, rect, &file, look)),
        None => {
            if let Some(at) = offline(ui, rect, path, look) {
                return at;
            }
            missing_plate(ui, rect, &format!("image · {path} · file not found"));
            None
        }
    }
}

/// One still (an image, or a video's poster), fitted, faded, and dimmed for
/// legibility. Returns where it landed.
fn paint_still(ui: &mut Ui, rect: Rect, file: &std::path::Path, look: PictureLook) -> Rect {
    // file:/// with THREE slashes — two reads as a UNC host on Windows and
    // the load errors.
    let uri = file.to_string_lossy().replace('\\', "/");
    let image = egui::Image::new(format!("file:///{uri}"))
        .corner_radius(cl::RADIUS)
        .tint(egui::Color32::WHITE.gamma_multiply(look.alpha));
    let size = image
        .load_for_size(ui.ctx(), rect.size())
        .ok()
        .and_then(|poll| poll.size());
    let (at, uv) = fit_picture(rect, size, look.contain);
    image.uv(uv).paint_at(ui, at);
    legibility(ui, at, look.alpha);
    at
}

/// The dark diffusion the vocabulary demands, so words can sit on a
/// picture — faded with the picture's entrance.
fn legibility(ui: &Ui, at: Rect, alpha: f32) {
    let shade = (120.0 * alpha.clamp(0.0, 1.0)) as u8;
    ui.painter()
        .rect_filled(at, cl::RADIUS, egui::Color32::from_black_alpha(shade));
}

/// The video ladder (B1): ffmpeg-subprocess loop → poster still → labeled
/// plate. `poster` is an optional explicit ref (W11 documents); absent, the
/// derived `poster_of` still applies.
fn paint_video(
    ui: &mut Ui,
    rect: Rect,
    path: &str,
    poster: Option<&str>,
    look: PictureLook,
) -> Option<Rect> {
    let Some(file) = dpl_video::locate_asset(path) else {
        if let Some(at) = offline(ui, rect, path, look) {
            return at;
        }
        missing_plate(ui, rect, &format!("video · {path} · file not found"));
        return None;
    };
    let tint = egui::Color32::WHITE.gamma_multiply(look.alpha);
    if let Some(at) = dpl_video::loop_backdrop(ui, rect, &file, look.contain, tint) {
        // Legibility diffusion, same as the image slot.
        legibility(ui, at, look.alpha);
        return Some(at);
    }
    // Prefer an explicit poster ref, else the derived sidecar.
    let still = poster
        .and_then(dpl_video::locate_asset)
        .filter(|p| p.is_file())
        .or_else(|| Some(dpl_video::poster_of(&file)).filter(|p| p.is_file()))
        .or_else(|| dpl_video::location_poster(path).filter(|p| p.is_file()));
    match still {
        Some(p) => Some(paint_still(ui, rect, &p, look)),
        None => {
            missing_plate(ui, rect, &format!("video · {path} · waiting for ffmpeg"));
            None
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stage() -> Rect {
        Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(320.0, 180.0))
    }

    /// "Show all of it" (LK3b): the whole picture, in its own shape, inside
    /// the stage and centred — a tall screenshot is letterboxed, not squashed.
    #[test]
    fn show_all_of_it_keeps_the_whole_picture_in_its_shape() {
        let tall = egui::vec2(400.0, 800.0);
        let (at, uv) = fit_picture(stage(), Some(tall), true);
        assert!(stage().contains_rect(at), "{at:?} inside {:?}", stage());
        assert!(
            (at.width() / at.height() - 0.5).abs() < 1e-3,
            "its own shape"
        );
        assert_eq!(at.height(), stage().height());
        assert_eq!(at.center(), stage().center());
        assert_eq!(
            uv,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
        );
    }

    /// "Fill the card": the stage is filled, the picture cropped evenly.
    #[test]
    fn fill_the_card_fills_the_stage_and_crops_evenly() {
        let tall = egui::vec2(400.0, 800.0);
        let (at, uv) = fit_picture(stage(), Some(tall), false);
        assert_eq!(at, stage());
        assert_eq!((uv.min.x, uv.max.x), (0.0, 1.0), "the full width shows");
        assert!(uv.min.y > 0.0 && uv.max.y < 1.0, "top and bottom cropped");
        assert!((uv.min.y - (1.0 - uv.max.y)).abs() < 1e-5, "cropped evenly");
    }

    /// A picture whose size is not known yet fills the stage whole.
    #[test]
    fn an_unknown_size_fills_the_stage() {
        let (at, _) = fit_picture(stage(), None, true);
        assert_eq!(at, stage());
    }
}
