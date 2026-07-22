//! The chart stage — 960×540 logical units mapped into any rect.

use dpl_coldlight as cl;
use egui::{Align2, Color32, FontId, Pos2, Rect};

use crate::blocks::eyebrow::tracked;

pub const LOGICAL_W: f32 = 960.0;
pub const LOGICAL_H: f32 = 540.0;

/// Margins in logical units (ported: top 24, right 32, bottom 56, left 32 —
/// room for hand-rolled labels; typography is ours, not a chart lib's).
pub const MARGIN: (f32, f32, f32, f32) = (24.0, 32.0, 56.0, 32.0);

/// A fitted chart stage: logical → screen mapping.
#[derive(Clone, Copy)]
pub struct ChartStage {
    pub rect: Rect,
    scale: f32,
}

impl ChartStage {
    /// Fit the logical space into `avail`, aspect-preserving, centered.
    pub fn fit(avail: Rect) -> Self {
        let scale = (avail.width() / LOGICAL_W).min(avail.height() / LOGICAL_H);
        let size = egui::vec2(LOGICAL_W * scale, LOGICAL_H * scale);
        let min = avail.center() - size * 0.5;
        Self { rect: Rect::from_min_size(min, size), scale }
    }

    /// A logical point on screen.
    pub fn pt(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + egui::vec2(x * self.scale, y * self.scale)
    }

    /// A logical length on screen.
    pub fn len(&self, v: f32) -> f32 {
        v * self.scale
    }

    /// The plot area inside the margins, in logical coordinates
    /// `(x0, y0, x1, y1)`.
    pub fn plot(&self) -> (f32, f32, f32, f32) {
        let (t, r, b, l) = MARGIN;
        (l, t, LOGICAL_W - r, LOGICAL_H - b)
    }

    /// Mono uppercase-tracked label at a logical position/size — the axis and
    /// datum voice every chart shares.
    pub fn mono_label(
        &self,
        painter: &egui::Painter,
        pos: (f32, f32),
        anchor: Align2,
        text: &str,
        logical_size: f32,
        color: Color32,
    ) {
        painter.text(
            self.pt(pos.0, pos.1),
            anchor,
            tracked(text),
            FontId::monospace(self.len(logical_size)),
            color,
        );
    }

    /// Display-voice text (values, readouts) at a logical position/size.
    pub fn display_text(
        &self,
        painter: &egui::Painter,
        pos: (f32, f32),
        anchor: Align2,
        text: &str,
        logical_size: f32,
        color: Color32,
    ) {
        painter.text(
            self.pt(pos.0, pos.1),
            anchor,
            text,
            FontId::proportional(self.len(logical_size)),
            color,
        );
    }

    /// The hairline axis color.
    pub fn hairline(&self) -> Color32 {
        cl::surface::HAIRLINE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_preserves_aspect_and_centers() {
        // A wide host: height binds, horizontal centering splits the slack.
        let host = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(2000.0, 540.0));
        let s = ChartStage::fit(host);
        assert!((s.rect.height() - 540.0).abs() < 0.01);
        assert!((s.rect.width() - 960.0).abs() < 0.01);
        assert!((s.rect.center().x - 1000.0).abs() < 0.01);
        // Logical corners land on the fitted rect.
        assert_eq!(s.pt(0.0, 0.0), s.rect.min);
        assert!((s.pt(LOGICAL_W, LOGICAL_H) - s.rect.max).length() < 0.01);
    }
}
