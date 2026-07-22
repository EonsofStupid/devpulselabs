//! PredictRank — call-the-next-word. The room shouts BEFORE the ranking
//! lands: commit-before-reveal, presenter-paced, carried. Reveal animates
//! the ranked bars (the post-tap settle in `bar-grow` is kept as the drama
//! beat); rounds advance; the LAND line holds after the last.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("predict"),
    label: "Predict rank",
    class: TemplateClass::Interactive,
    blurb: "The room calls the next word out loud, THEN the ranked probabilities land.",
};

pub struct Guess {
    pub token: &'static str,
    pub pct: f32,
}

pub struct PredictRound {
    pub prompt: &'static str,
    /// Pre-sorted descending — the ranking as it lands.
    pub guesses: Vec<Guess>,
}

pub struct PredictRank {
    pub tone: Tone,
    pub heading: &'static str,
    pub rounds: Vec<PredictRound>,
    pub reveal: &'static str,
    round: usize,
    revealed: Option<f64>,
}

impl PredictRank {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Violet,
            heading: "Call the next word",
            rounds: vec![
                PredictRound {
                    prompt: "The stream starts in five ___",
                    guesses: vec![
                        Guess { token: "minutes", pct: 71.0 },
                        Guess { token: "seconds", pct: 18.0 },
                        Guess { token: "hours", pct: 6.0 },
                        Guess { token: "bananas", pct: 0.2 },
                    ],
                },
                PredictRound {
                    prompt: "Like and ___",
                    guesses: vec![
                        Guess { token: "subscribe", pct: 88.0 },
                        Guess { token: "share", pct: 7.0 },
                        Guess { token: "comment", pct: 4.0 },
                    ],
                },
            ],
            reveal: "You just did what the model does — ranked the likely next words. It's that, at scale.",
            round: 0,
            revealed: None,
        }
    }

    fn last_round(&self) -> bool {
        self.round >= self.rounds.len().saturating_sub(1)
    }

    /// Next round: hides the ranking again; clamps at the end (tested).
    fn next_round(&mut self) {
        if !self.last_round() {
            self.round += 1;
            self.revealed = None;
        }
    }
}

impl SlideTemplate for PredictRank {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.round = 0;
        self.revealed = None;
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();
        let paint = self.tone.color();
        let current = &self.rounds[self.round.min(self.rounds.len() - 1)];

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.heading,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );
        let prompt_y = stage.rect.min.y + fluid.fs_h2() + fluid.sp_4();
        painter.text(
            egui::pos2(stage.rect.center().x, prompt_y),
            Align2::CENTER_TOP,
            format!("{}   ·   round {}/{}", current.prompt, self.round + 1, self.rounds.len()),
            FontId::monospace(fluid.fs_h4()),
            cl::ink::BASE,
        );

        // The ranking — "?" ghosts until the room has committed.
        let top = prompt_y + fluid.fs_h4() + fluid.sp_6();
        let foot_h = fluid.sp_8() + fluid.fs_h4() * 1.6;
        let gap = fluid.sp_3();
        let n = current.guesses.len() as f32;
        let row_h = (((stage.rect.max.y - foot_h) - top) - gap * (n - 1.0)) / n;
        let token_w = fluid.sp_8() * 2.0;
        let pct_w = fluid.sp_8();
        let max_pct = current.guesses.iter().map(|g| g.pct).fold(1.0f32, f32::max);
        let grow_fx = cl::motion::BAR_GROW;
        for (i, g) in current.guesses.iter().enumerate() {
            let y = top + (row_h + gap) * i as f32;
            let track = Rect::from_min_max(
                egui::pos2(stage.rect.min.x + token_w + gap, y + row_h * 0.2),
                egui::pos2(stage.rect.max.x - pct_w - gap, y + row_h * 0.8),
            );
            let radius = track.height() * 0.5;
            match self.revealed {
                Some(t0) => {
                    let grow = grow_fx.at(now, t0, i);
                    if grow < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    painter.text(
                        egui::pos2(stage.rect.min.x + token_w, y + row_h * 0.5),
                        Align2::RIGHT_CENTER,
                        g.token,
                        FontId::proportional(fluid.fs_h4()),
                        if i == 0 { cl::ink::STRONG } else { cl::ink::MUTED },
                    );
                    painter.rect_filled(track, radius, cl::surface::RAISED);
                    let fill = Rect::from_min_size(
                        track.min,
                        egui::vec2(track.width() * (g.pct / max_pct) * grow, track.height()),
                    );
                    if i == 0 {
                        painter.rect_filled(fill, radius, paint);
                        painter.rect_stroke(
                            fill,
                            radius,
                            egui::Stroke::new(1.0, cl::ink::STRONG.gamma_multiply(0.5)),
                            egui::StrokeKind::Inside,
                        );
                    } else {
                        painter.rect_filled(fill, radius, cl::surface::OVERLAY);
                    }
                    painter.text(
                        egui::pos2(stage.rect.max.x, y + row_h * 0.5),
                        Align2::RIGHT_CENTER,
                        if g.pct < 1.0 { format!("{}%", g.pct) } else { format!("{}%", g.pct.round() as i32) },
                        FontId::monospace(fluid.fs_body_sm()),
                        cl::ink::MUTED,
                    );
                }
                None => {
                    painter.text(
                        egui::pos2(stage.rect.min.x + token_w, y + row_h * 0.5),
                        Align2::RIGHT_CENTER,
                        "?",
                        FontId::proportional(fluid.fs_h4()),
                        cl::ink::SUBTLE,
                    );
                    painter.rect_filled(track, radius, cl::surface::RAISED.gamma_multiply(0.5));
                    painter.text(
                        egui::pos2(stage.rect.max.x, y + row_h * 0.5),
                        Align2::RIGHT_CENTER,
                        "··",
                        FontId::monospace(fluid.fs_body_sm()),
                        cl::ink::SUBTLE,
                    );
                }
            }
        }

        // One obvious tap at every stage (carried).
        let foot = egui::pos2(stage.rect.center().x, stage.rect.max.y);
        let button = |ui: &mut Ui, label: &str, filled: bool| -> bool {
            let galley = painter.layout_no_wrap(
                tracked(label),
                FontId::monospace(fluid.fs_eyebrow()),
                cl::surface::WELL,
            );
            let btn = Rect::from_center_size(
                egui::pos2(foot.x, foot.y - galley.size().y),
                galley.size() + egui::vec2(fluid.sp_6() * 2.0, fluid.sp_4()),
            );
            let resp = ui.allocate_rect(btn, Sense::click());
            let (fill, ink) = if filled {
                (if resp.hovered() { paint } else { paint.gamma_multiply(0.85) }, cl::surface::WELL)
            } else {
                (
                    if resp.hovered() { cl::surface::OVERLAY } else { cl::surface::RAISED },
                    cl::ink::BASE,
                )
            };
            painter.rect_filled(btn, btn.height() * 0.5, fill);
            painter.galley(btn.center() - galley.size() * 0.5, galley, ink);
            resp.clicked()
        };

        match self.revealed {
            None => {
                painter.text(
                    egui::pos2(foot.x, foot.y - fluid.sp_8() - fluid.sp_2()),
                    Align2::CENTER_BOTTOM,
                    tracked("Call it out loud — then reveal"),
                    FontId::monospace(fluid.fs_eyebrow()),
                    cl::ink::MUTED,
                );
                if button(ui, "Reveal the ranking", true) {
                    self.revealed = Some(now);
                }
            }
            Some(t0) => {
                if !self.last_round() {
                    if button(ui, "Next round →", false) {
                        self.next_round();
                    }
                } else {
                    let p = cl::motion::ENTER_FADE.at(now, t0, 0);
                    if p < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    painter.text(
                        foot,
                        Align2::CENTER_BOTTOM,
                        self.reveal,
                        FontId::proportional(fluid.fs_h4()),
                        cl::ink::BASE.gamma_multiply(p),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_advance_hide_again_and_clamp() {
        let mut p = PredictRank::demo();
        p.revealed = Some(1.0);
        p.next_round();
        assert_eq!(p.round, 1);
        assert!(p.revealed.is_none(), "next round hides the ranking");
        p.revealed = Some(2.0);
        p.next_round();
        assert_eq!(p.round, 1, "clamps at the last round");
        p.restart();
        assert_eq!((p.round, p.revealed.is_none()), (0, true));
    }
}
