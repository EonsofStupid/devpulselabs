//! Timeline — a horizontal run of milestones on a baseline that draws itself
//! in, dots popping in order, labels alternating above/below so they never
//! collide. The "how we got here" slide. Class: animation (pure clock
//! function — sweeps once on the settle, then rests).

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Pos2, Stroke, Ui};

use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("timeline"),
    label: "Timeline",
    class: TemplateClass::Animation,
    blurb: "Milestones on a baseline that draws in, dots popping in order — the 'how we got here'.",
};

pub struct Milestone {
    pub when: &'static str,
    pub what: &'static str,
}

pub struct Timeline {
    pub tone: Tone,
    pub heading: &'static str,
    pub stops: Vec<Milestone>,
    clock: Clock,
}

impl Timeline {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Cyan,
            heading: "Four years, one idea",
            stops: vec![
                Milestone {
                    when: "2017",
                    what: "Attention is all you need",
                },
                Milestone {
                    when: "2020",
                    what: "Scale surprises everyone",
                },
                Milestone {
                    when: "2023",
                    what: "Everyone has one in a tab",
                },
                Milestone {
                    when: "2026",
                    what: "It runs on your desk",
                },
            ],
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for Timeline {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.clock.restart();
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        let painter = ui.painter();

        let draw = cl::motion::CHART_DRAW;
        let pop = cl::motion::REVEAL_POP;
        let n = self.stops.len();
        if draw.in_flight(now, t0, 1) || pop.in_flight(now, t0, n) {
            ui.ctx().request_repaint();
        }

        painter.text(
            egui::pos2(stage.rect.min.x, stage.rect.min.y),
            Align2::LEFT_TOP,
            self.heading,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let tone = self.tone.color();
        let axis_y = stage.rect.center().y + fluid.sp_6();
        let x0 = stage.rect.min.x + fluid.sp_6();
        let x1 = stage.rect.max.x - fluid.sp_6();
        let span = x1 - x0;

        // The baseline draws left→right.
        let d = draw.at(now, t0, 0).clamp(0.0, 1.0);
        painter.line_segment(
            [Pos2::new(x0, axis_y), Pos2::new(x0 + span * d, axis_y)],
            Stroke::new(2.0, cl::surface::HAIRLINE),
        );

        for (i, stop) in self.stops.iter().enumerate() {
            let fx = if n > 1 {
                i as f32 / (n as f32 - 1.0)
            } else {
                0.5
            };
            let cx = x0 + span * fx;
            // The dot pops only after the line has reached it.
            let reached = d >= fx - 0.001;
            let p = if reached {
                pop.at(now, t0, i).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let r = fluid.sp_2() * (0.4 + 0.6 * p);
            painter.circle_filled(
                Pos2::new(cx, axis_y),
                r,
                egui::Color32::from_rgba_unmultiplied(
                    tone.r(),
                    tone.g(),
                    tone.b(),
                    (255.0 * p) as u8,
                ),
            );

            // Labels alternate above / below the axis.
            let above = i % 2 == 0;
            let ly = if above {
                axis_y - fluid.sp_5()
            } else {
                axis_y + fluid.sp_5()
            };
            let anchor = if above {
                Align2::CENTER_BOTTOM
            } else {
                Align2::CENTER_TOP
            };
            painter.text(
                Pos2::new(cx, ly),
                anchor,
                tracked(stop.when),
                FontId::monospace(fluid.fs_eyebrow()),
                tone.gamma_multiply(p),
            );
            let wy = if above {
                ly - fluid.fs_eyebrow() - fluid.sp_1()
            } else {
                ly + fluid.fs_eyebrow() + fluid.sp_1()
            };
            let galley = painter.layout(
                stop.what.to_string(),
                FontId::proportional(fluid.fs_body()),
                cl::ink::BASE.gamma_multiply(p),
                span / n as f32 * 0.9,
            );
            let anchor2 = if above {
                Pos2::new(cx - galley.size().x * 0.5, wy - galley.size().y)
            } else {
                Pos2::new(cx - galley.size().x * 0.5, wy)
            };
            painter.galley(anchor2, galley, cl::ink::BASE);
        }

        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
