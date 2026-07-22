//! QuizPanel — recap quiz: commit an answer, THEN the explanation lands.
//!
//! Retrieval practice, carried: the room commits before it sees. Single =
//! radio (pick replaces), multi = toggle (ALL correct must be picked).
//! State is local and resets on restart — no scoring persistence, by
//! doctrine. Every phase is a rest state.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("quiz"),
    label: "Quiz panel",
    class: TemplateClass::Interactive,
    blurb: "Commit-then-explain: pick, lock it in, the truth highlights and the explanation lands.",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    Single,
    Multi,
}

pub struct QuizChoice {
    pub label: &'static str,
    pub correct: bool,
}

pub struct QuizPanel {
    pub tone: Tone,
    pub prompt: &'static str,
    pub select: Select,
    pub choices: Vec<QuizChoice>,
    pub explanation: &'static str,
    picked: Vec<bool>,
    committed: Option<f64>,
}

impl QuizPanel {
    pub fn demo() -> Self {
        let choices = vec![
            QuizChoice {
                label: "It looks up answers in a database",
                correct: false,
            },
            QuizChoice {
                label: "It predicts the next token, over and over",
                correct: true,
            },
            QuizChoice {
                label: "It reasons like a person, just faster",
                correct: false,
            },
        ];
        let n = choices.len();
        Self {
            tone: Tone::Violet,
            prompt: "What is the model actually doing?",
            select: Select::Single,
            choices,
            explanation: "Every reply is next-token prediction in a loop — scale is what makes it feel like more.",
            picked: vec![false; n],
            committed: None,
        }
    }

    /// Toggle choice `i` per the select mode (pure — tested).
    fn pick(&mut self, i: usize) {
        if self.committed.is_some() {
            return;
        }
        match self.select {
            Select::Single => {
                for (j, p) in self.picked.iter_mut().enumerate() {
                    *p = j == i;
                }
            }
            Select::Multi => self.picked[i] = !self.picked[i],
        }
    }

    /// Right = picked set equals the correct set exactly.
    fn is_right(&self) -> bool {
        self.choices
            .iter()
            .zip(&self.picked)
            .all(|(c, p)| c.correct == *p)
    }

    fn any_picked(&self) -> bool {
        self.picked.iter().any(|p| *p)
    }
}

impl SlideTemplate for QuizPanel {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.picked.iter_mut().for_each(|p| *p = false);
        self.committed = None;
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();
        let committed = self.committed;
        let radius = fluid.r_md() as u8;

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.prompt,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let foot_h = fluid.fs_h4() * 2.4 + fluid.sp_5();
        let gap = fluid.sp_3();
        let n = self.choices.len() as f32;
        let row_h = ((stage.rect.max.y - foot_h - top) - gap * (n - 1.0)) / n;

        let mut tapped: Option<usize> = None;
        for (i, choice) in self.choices.iter().enumerate() {
            let plate = Rect::from_min_size(
                egui::pos2(stage.rect.min.x, top + (row_h + gap) * i as f32),
                egui::vec2(stage.rect.width(), row_h),
            );
            let resp = ui.allocate_rect(plate, Sense::click());
            if resp.clicked() {
                tapped = Some(i);
            }
            let picked = self.picked[i];
            // Post-commit paint: truth gets the wash+ring, wrong picks dim.
            let (fill, ring, ink) = if committed.is_some() {
                if choice.correct {
                    (self.tone.wash(), Some(self.tone.color()), cl::ink::STRONG)
                } else if picked {
                    (
                        egui::Color32::from_rgba_unmultiplied(0x11, 0x14, 0x1d, 90),
                        None,
                        cl::ink::SUBTLE,
                    )
                } else {
                    (cl::surface::RAISED, None, cl::ink::MUTED)
                }
            } else if picked {
                (
                    cl::surface::OVERLAY,
                    Some(self.tone.color()),
                    cl::ink::STRONG,
                )
            } else {
                (
                    if resp.hovered() {
                        cl::surface::OVERLAY
                    } else {
                        cl::surface::RAISED
                    },
                    None,
                    cl::ink::BASE,
                )
            };
            painter.rect_filled(plate, radius, fill);
            painter.rect_stroke(
                plate,
                radius,
                egui::Stroke::new(1.0, ring.unwrap_or(cl::surface::HAIRLINE)),
                egui::StrokeKind::Inside,
            );
            // The choice mark: ring, dot when picked.
            let mark = egui::pos2(plate.min.x + fluid.sp_5(), plate.center().y);
            painter.circle_stroke(mark, fluid.sp_2(), egui::Stroke::new(1.5, cl::ink::SUBTLE));
            if picked {
                painter.circle_filled(mark, fluid.sp_1(), self.tone.color());
            }
            painter.text(
                egui::pos2(mark.x + fluid.sp_5(), plate.center().y),
                Align2::LEFT_CENTER,
                choice.label,
                FontId::proportional(fluid.fs_h4()),
                ink,
            );
        }
        if let Some(i) = tapped {
            self.pick(i);
        }

        // Footer: the lock-in control, then the verdict + explanation.
        let foot_center = egui::pos2(stage.rect.center().x, stage.rect.max.y - foot_h * 0.5);
        match committed {
            None => {
                let armed = self.any_picked();
                let label = if self.select == Select::Multi {
                    "Pick every right answer · lock it in"
                } else {
                    "Lock it in"
                };
                let galley = painter.layout_no_wrap(
                    tracked(label),
                    FontId::monospace(fluid.fs_eyebrow()),
                    cl::surface::WELL,
                );
                let btn = Rect::from_center_size(
                    foot_center,
                    galley.size() + egui::vec2(fluid.sp_6() * 2.0, fluid.sp_4()),
                );
                let resp = ui.allocate_rect(btn, Sense::click());
                let fill = if !armed {
                    cl::surface::RAISED
                } else if resp.hovered() {
                    self.tone.color()
                } else {
                    self.tone.color().gamma_multiply(0.85)
                };
                painter.rect_filled(btn, btn.height() * 0.5, fill);
                painter.galley(
                    btn.center() - galley.size() * 0.5,
                    galley,
                    if armed {
                        cl::surface::WELL
                    } else {
                        cl::ink::SUBTLE
                    },
                );
                if armed && resp.clicked() {
                    self.committed = Some(now);
                }
            }
            Some(t0) => {
                let p = cl::motion::ENTER_FADE.at(now, t0, 0);
                if p < 1.0 {
                    ui.ctx().request_repaint();
                }
                let right = self.is_right();
                // Reward pulse under the verdict when the room called it.
                if right {
                    let pulse = cl::motion::REWARD_PULSE.at(now, t0, 0);
                    if pulse < 1.0 {
                        ui.ctx().request_repaint();
                        let c = self.tone.color();
                        painter.circle_stroke(
                            foot_center,
                            fluid.sp_8() * (0.4 + pulse),
                            egui::Stroke::new(
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(
                                    c.r(),
                                    c.g(),
                                    c.b(),
                                    (120.0 * (1.0 - pulse)) as u8,
                                ),
                            ),
                        );
                    }
                }
                painter.text(
                    egui::pos2(foot_center.x, foot_center.y - fluid.fs_h4() * 0.8),
                    Align2::CENTER_CENTER,
                    tracked(if right { "Called it" } else { "Not quite" }),
                    FontId::monospace(fluid.fs_eyebrow()),
                    if right {
                        self.tone.color()
                    } else {
                        cl::ink::MUTED
                    }
                    .gamma_multiply(p),
                );
                painter.text(
                    egui::pos2(foot_center.x, foot_center.y + fluid.fs_body() * 0.6),
                    Align2::CENTER_CENTER,
                    self.explanation,
                    FontId::proportional(fluid.fs_body_lg()),
                    cl::ink::BASE.gamma_multiply(p),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_replaces_multi_toggles_and_right_needs_the_exact_set() {
        let mut q = QuizPanel::demo();
        q.pick(0);
        q.pick(1);
        assert_eq!(q.picked, vec![false, true, false], "single: pick replaces");
        assert!(q.is_right());
        q.select = Select::Multi;
        q.pick(0);
        assert_eq!(q.picked, vec![true, true, false], "multi: pick toggles on");
        assert!(!q.is_right(), "an extra wrong pick breaks exactness");
        q.committed = Some(1.0);
        q.pick(2);
        assert!(!q.picked[2], "committed = locked");
        q.restart();
        assert!(q.picked.iter().all(|p| !p) && q.committed.is_none());
    }
}
