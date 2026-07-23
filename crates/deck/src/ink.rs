//! The ink layer (D3) — stream-time annotation, ported from the old
//! `annotation-layer.tsx`: pressure-width strokes in the annotate ink
//! token, EPHEMERAL by design (cleared when the slide changes, surviving
//! step changes; never persisted — v1 scope, old parity).
//!
//! Retained data, not a canvas: egui repaints every frame, so strokes
//! are points. Coordinates are NORMALIZED to the deck rect — ink stays
//! where it was drawn through a resize (the retained model's free win).

use dpl_coldlight as cl;
use egui::{Pos2, Rect, Ui};

/// One finished stroke: normalized points with their pressure widths.
#[derive(Debug, Clone, Default)]
pub struct Stroke {
    pub points: Vec<(Pos2, f32)>,
}

#[derive(Debug, Default)]
pub struct InkLayer {
    strokes: Vec<Stroke>,
    current: Option<Stroke>,
}

/// The old layer's width law, verbatim: `3 + (pressure || 0.5) * 4`.
pub fn width_of(force: Option<f32>) -> f32 {
    3.0 + force.unwrap_or(0.5) * 4.0
}

/// Normalize a screen point into the deck rect (0..1 both axes).
pub fn normalize(p: Pos2, rect: Rect) -> Pos2 {
    Pos2::new(
        ((p.x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0),
        ((p.y - rect.min.y) / rect.height().max(1.0)).clamp(0.0, 1.0),
    )
}

fn denormalize(p: Pos2, rect: Rect) -> Pos2 {
    Pos2::new(
        rect.min.x + p.x * rect.width(),
        rect.min.y + p.y * rect.height(),
    )
}

impl InkLayer {
    pub fn begin(&mut self, p_norm: Pos2, force: Option<f32>) {
        self.current = Some(Stroke {
            points: vec![(p_norm, width_of(force))],
        });
    }

    pub fn extend(&mut self, p_norm: Pos2, force: Option<f32>) {
        if let Some(s) = &mut self.current {
            s.points.push((p_norm, width_of(force)));
        }
    }

    pub fn finish(&mut self) {
        if let Some(s) = self.current.take() {
            if s.points.len() > 1 {
                self.strokes.push(s);
            }
        }
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
        self.current = None;
    }

    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty() && self.current.is_none()
    }

    pub fn stroke_count(&self) -> usize {
        self.strokes.len()
    }

    /// Paint every stroke into the live rect: per-segment width (the
    /// pressure feel), round caps via endpoint dots.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        for s in self.strokes.iter().chain(self.current.iter()) {
            for pair in s.points.windows(2) {
                let (a, wa) = pair[0];
                let (b, wb) = pair[1];
                let (pa, pb) = (denormalize(a, rect), denormalize(b, rect));
                let w = (wa + wb) * 0.5;
                painter.line_segment([pa, pb], egui::Stroke::new(w, cl::ink::ANNOTATE));
                painter.circle_filled(pb, w * 0.5, cl::ink::ANNOTATE);
            }
            if let Some(&(p, w)) = s.points.first() {
                painter.circle_filled(denormalize(p, rect), w * 0.5, cl::ink::ANNOTATE);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_width_law_is_the_old_ones() {
        assert_eq!(width_of(None), 5.0, "no pressure = 0.5 default");
        assert_eq!(width_of(Some(1.0)), 7.0);
        assert_eq!(width_of(Some(0.0)), 3.0);
    }

    #[test]
    fn strokes_round_trip_and_single_points_are_dropped() {
        let mut ink = InkLayer::default();
        ink.begin(Pos2::new(0.1, 0.1), None);
        ink.extend(Pos2::new(0.2, 0.2), Some(1.0));
        ink.finish();
        assert_eq!(ink.stroke_count(), 1);
        // A tap without movement is not a stroke.
        ink.begin(Pos2::new(0.5, 0.5), None);
        ink.finish();
        assert_eq!(ink.stroke_count(), 1);
        ink.clear();
        assert!(ink.is_empty());
    }

    #[test]
    fn normalization_survives_a_resize() {
        let small = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 50.0));
        let big = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let n = normalize(Pos2::new(60.0, 35.0), small);
        assert!((n.x - 0.5).abs() < 1e-4 && (n.y - 0.5).abs() < 1e-4);
        let back = denormalize(n, big);
        assert!((back.x - 960.0).abs() < 0.5 && (back.y - 540.0).abs() < 0.5);
    }
}
