//! The slide's background: a mood, a blend of two, or the bare well.

use super::refs::{ambient_of, plate};
use crate::elements::media::{ambient_backdrop, Ambient};
use dpl_coldlight::{self as cl};
use egui::{Color32, Rect, Ui};
use std::f32::consts::TAU;

/// Paint the document's background into `rect`; returns the leading accent
/// color (the "tone" ink and the ambience ride it). Takes the RESOLVED
/// background — `None` only if resolution was skipped, painting as the
/// bare well (defensive; `resolve` always fills it).
pub(super) fn background(
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
