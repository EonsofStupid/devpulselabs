//! StatsGrid — several numbers at once; cards flip to a detail back.
//!
//! The ensemble to BigNumber's single receipt, carried. Flip is a crossfade
//! (`flip-fade` — tap feedback, no settle) and a FLOURISH, not a beat: both
//! faces are rest states, cards without detail are plain plates. Entrance:
//! staggered layer-fade.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};

use crate::elements::eyebrow::tracked;
use crate::elements::{media_backdrop, Ambient, Media};
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("stats"),
    label: "Stats grid",
    class: TemplateClass::Static,
    blurb: "The ensemble: a grid of toned numbers; tap a card to crossfade to its detail back.",
};

pub struct StatItem {
    pub value: &'static str,
    pub label: &'static str,
    /// `None` = a plain, non-flippable plate.
    pub detail: Option<&'static str>,
    pub tone: Option<Tone>,
}

pub struct StatsGrid {
    pub tone: Tone,
    pub title: &'static str,
    pub stats: Vec<StatItem>,
    pub media: Option<Media>,
    /// (flipped, when-toggled) per card — the crossfade clocks.
    flips: Vec<(bool, f64)>,
    clock: Clock,
}

impl StatsGrid {
    pub fn demo() -> Self {
        let stats = vec![
            StatItem {
                value: "8B",
                label: "params your GPU runs",
                detail: Some("Llama-class open weights, quantized — 6 GB of VRAM"),
                tone: Some(Tone::Lime),
            },
            StatItem {
                value: "43×",
                label: "cheaper per token vs 2023",
                detail: Some("frontier API list price, same benchmark tier"),
                tone: Some(Tone::Cyan),
            },
            StatItem {
                value: "0",
                label: "bytes leaving your network",
                detail: Some("local inference — the privacy number that matters"),
                tone: Some(Tone::Amber),
            },
            StatItem {
                value: "1",
                label: "GPU is enough to start",
                detail: None,
                tone: None,
            },
        ];
        let n = stats.len();
        Self {
            tone: Tone::Cyan,
            title: "The local numbers",
            stats,
            // Rings is a SLIDE ambience (ratified 2026-07-22) — gallery gate.
            media: Some(Media::Anim(Ambient::Rings)),
            flips: vec![(false, f64::NEG_INFINITY); n],
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for StatsGrid {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.clock.restart();
        self.flips
            .iter_mut()
            .for_each(|f| *f = (false, f64::NEG_INFINITY));
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        if let Some(m) = self.media {
            media_backdrop(ui, ui.max_rect(), m, self.tone, now, t0);
        }
        let painter = ui.painter().clone();
        let enter = cl::motion::LAYER_FADE;
        let flip_fx = cl::motion::FLIP_FADE;
        if enter.in_flight(now, t0, self.stats.len()) {
            ui.ctx().request_repaint();
        }

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.title,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        // Auto-fit: 2 columns (the wall reads big); rows as needed.
        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let gap = fluid.sp_4();
        let cols = 2usize;
        let rows = self.stats.len().div_ceil(cols);
        let card_w = (stage.rect.width() - gap * (cols - 1) as f32) / cols as f32;
        let card_h = ((stage.rect.max.y - top) - gap * (rows - 1) as f32) / rows as f32;
        let radius = fluid.r_md() as u8;

        for (i, stat) in self.stats.iter().enumerate() {
            let (col, row) = (i % cols, i / cols);
            let card = Rect::from_min_size(
                egui::pos2(
                    stage.rect.min.x + (card_w + gap) * col as f32,
                    top + (card_h + gap) * row as f32,
                ),
                egui::vec2(card_w, card_h),
            );
            let entered = enter.at(now, t0, i).clamp(0.0, 1.0);
            let tone = stat.tone.unwrap_or(self.tone);
            let flippable = stat.detail.is_some();
            let resp = ui.allocate_rect(card, Sense::click());
            if flippable && resp.clicked() {
                let (f, _) = self.flips[i];
                self.flips[i] = (!f, now);
            }
            let (flipped, flip_t) = self.flips[i];
            // Crossfade: front alpha ↔ back alpha on the flip clock.
            let fp = flip_fx.at(now, flip_t, 0);
            if fp < 1.0 && flip_t.is_finite() {
                ui.ctx().request_repaint();
            }
            let back_a = if flipped { fp } else { 1.0 - fp };
            let front_a = 1.0 - back_a;

            painter.rect_filled(card, radius, cl::surface::RAISED.gamma_multiply(entered));
            painter.rect_stroke(
                card,
                radius,
                egui::Stroke::new(1.0, cl::surface::HAIRLINE.gamma_multiply(entered)),
                egui::StrokeKind::Inside,
            );
            cl::sheen(&painter, card);

            if front_a > 0.01 {
                let a = entered * front_a;
                painter.text(
                    egui::pos2(card.center().x, card.center().y - fluid.fs_h2() * 0.35),
                    Align2::CENTER_CENTER,
                    stat.value,
                    FontId::proportional(fluid.fs_h1()),
                    tone.color().gamma_multiply(a),
                );
                painter.text(
                    egui::pos2(card.center().x, card.center().y + fluid.fs_h2() * 0.75),
                    Align2::CENTER_CENTER,
                    stat.label,
                    FontId::proportional(fluid.fs_body()),
                    cl::ink::BASE.gamma_multiply(a),
                );
            }
            if back_a > 0.01 {
                if let Some(detail) = stat.detail {
                    painter.text(
                        card.center(),
                        Align2::CENTER_CENTER,
                        detail,
                        FontId::proportional(fluid.fs_body_lg()),
                        cl::ink::STRONG.gamma_multiply(entered * back_a),
                    );
                }
            }
            if flippable {
                painter.text(
                    egui::pos2(card.max.x - fluid.sp_3(), card.max.y - fluid.sp_2()),
                    Align2::RIGHT_BOTTOM,
                    tracked(if flipped { "back" } else { "flip" }),
                    FontId::monospace(fluid.fs_eyebrow() * 0.85),
                    cl::ink::SUBTLE.gamma_multiply(entered),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_detail_cards_flip_and_restart_resets() {
        let mut g = StatsGrid::demo();
        g.flips[0] = (true, 5.0);
        g.restart();
        assert!(g.flips.iter().all(|(f, _)| !f));
        assert!(g.stats[3].detail.is_none(), "the plain plate stays plain");
    }
}
