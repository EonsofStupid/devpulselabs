//! DragMatch — drag each chip onto its bucket; the board rewards you.
//!
//! Carried mechanics: correct drop LOCKS the chip into the bucket (the
//! bucket reward-pulses); wrong drop SHAKES (drag-shake as a damped sine
//! translation) and the chip returns to the tray; all placed → the LAND
//! line reward-BLOOMS in. Every phase is a rest state.
//!
//! ⚠️ Deferred (tracked in the plan, not dropped): the carried keyboard
//! pick-up/place path — lands with egui focus integration.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};
use std::f32::consts::PI;

use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("drag-match"),
    label: "Drag match",
    class: TemplateClass::Interactive,
    blurb: "Drag chips onto their buckets: correct locks with a pulse, wrong shakes home.",
};

pub struct DragItem {
    pub label: &'static str,
    /// Index into `targets`.
    pub target: usize,
    pub tone: Option<Tone>,
}

pub struct DragTarget {
    pub label: &'static str,
    pub tone: Tone,
}

pub struct DragMatch {
    pub tone: Tone,
    pub prompt: &'static str,
    pub items: Vec<DragItem>,
    pub targets: Vec<DragTarget>,
    pub reveal: &'static str,
    placed: Vec<bool>,
    pulse: Option<(usize, f64)>,
    shake: Option<(usize, f64)>,
    done_at: Option<f64>,
}

impl DragMatch {
    pub fn demo() -> Self {
        let items = vec![
            DragItem {
                label: "Runs on your GPU",
                target: 0,
                tone: None,
            },
            DragItem {
                label: "Someone else's weights",
                target: 1,
                tone: None,
            },
            DragItem {
                label: "Zero bytes leave home",
                target: 0,
                tone: None,
            },
            DragItem {
                label: "Pay per token",
                target: 1,
                tone: None,
            },
        ];
        let n = items.len();
        Self {
            tone: Tone::Lime,
            prompt: "Local or cloud — sort the claims",
            items,
            targets: vec![
                DragTarget {
                    label: "Local",
                    tone: Tone::Lime,
                },
                DragTarget {
                    label: "Cloud",
                    tone: Tone::Cyan,
                },
            ],
            reveal: "If you can sort these, you already know which workloads belong on your box.",
            placed: vec![false; n],
            pulse: None,
            shake: None,
            done_at: None,
        }
    }

    /// Resolve a drop of item `i` on target `hit` (pure — tested).
    fn drop_on(&mut self, i: usize, hit: Option<usize>, now: f64) {
        let Some(t) = hit else { return };
        if self.items[i].target == t {
            self.placed[i] = true;
            self.pulse = Some((t, now));
            if self.placed.iter().all(|p| *p) {
                self.done_at = Some(now);
            }
        } else {
            self.shake = Some((i, now));
        }
    }
}

impl SlideTemplate for DragMatch {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.placed.iter_mut().for_each(|p| *p = false);
        self.pulse = None;
        self.shake = None;
        self.done_at = None;
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.prompt,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        // Buckets across the middle.
        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let bucket_h = stage.rect.height() * 0.42;
        let gutter = fluid.sp_5();
        let bw = (stage.rect.width() - gutter * (self.targets.len() - 1) as f32)
            / self.targets.len() as f32;
        let radius = fluid.r_lg() as u8;
        let mut bucket_rects = Vec::with_capacity(self.targets.len());
        for (t, target) in self.targets.iter().enumerate() {
            let bucket = Rect::from_min_size(
                egui::pos2(stage.rect.min.x + (bw + gutter) * t as f32, top),
                egui::vec2(bw, bucket_h),
            );
            bucket_rects.push(bucket);
            painter.rect_filled(bucket, radius, target.tone.wash().gamma_multiply(0.6));
            painter.rect_stroke(
                bucket,
                radius,
                egui::Stroke::new(1.0, target.tone.color().gamma_multiply(0.5)),
                egui::StrokeKind::Inside,
            );
            // The reward pulse blooming off a correct drop.
            if let Some((pt, t0)) = self.pulse {
                if pt == t {
                    let p = cl::motion::REWARD_PULSE.at(now, t0, 0);
                    if p < 1.0 {
                        ui.ctx().request_repaint();
                        let c = target.tone.color();
                        painter.rect_stroke(
                            bucket.expand(20.0 * p),
                            radius,
                            egui::Stroke::new(
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(
                                    c.r(),
                                    c.g(),
                                    c.b(),
                                    (130.0 * (1.0 - p)) as u8,
                                ),
                            ),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
            }
            painter.text(
                egui::pos2(bucket.center().x, bucket.min.y + fluid.sp_3()),
                Align2::CENTER_TOP,
                tracked(target.label),
                FontId::monospace(fluid.fs_eyebrow()),
                target.tone.color(),
            );
            // Locked chips stack inside their bucket.
            let mut cy = bucket.min.y + fluid.sp_6() + fluid.fs_eyebrow();
            for (i, item) in self.items.iter().enumerate() {
                if self.placed[i] && item.target == t {
                    let chip = Rect::from_min_size(
                        egui::pos2(bucket.min.x + fluid.sp_3(), cy),
                        egui::vec2(bucket.width() - fluid.sp_3() * 2.0, fluid.sp_7()),
                    );
                    painter.rect_filled(chip, chip.height() * 0.5, target.tone.wash());
                    painter.text(
                        chip.center(),
                        Align2::CENTER_CENTER,
                        item.label,
                        FontId::proportional(fluid.fs_body()),
                        cl::ink::STRONG,
                    );
                    cy += fluid.sp_7() + fluid.sp_2();
                }
            }
        }

        // The tray: unplaced chips, draggable.
        let tray_top = top + bucket_h + fluid.sp_5();
        let land_h = fluid.fs_h4() * 1.8;
        let mut cx = stage.rect.min.x;
        let mut drop: Option<(usize, Option<usize>)> = None;
        for (i, item) in self.items.iter().enumerate() {
            if self.placed[i] {
                continue;
            }
            let tone = item.tone.unwrap_or(self.tone);
            let galley = painter.layout_no_wrap(
                item.label.into(),
                FontId::proportional(fluid.fs_body_lg()),
                cl::ink::STRONG,
            );
            let size = galley.size() + egui::vec2(fluid.sp_6(), fluid.sp_4());
            if cx + size.x > stage.rect.max.x {
                cx = stage.rect.min.x; // simple wrap — the tray is one band
            }
            let home = Rect::from_min_size(egui::pos2(cx, tray_top), size);
            cx += size.x + fluid.sp_3();

            let resp = ui.allocate_rect(home, Sense::drag());
            let dragging = resp.dragged();
            // The chip draws at home (with shake) or under the pointer.
            let mut at = home;
            if dragging {
                if let Some(p) = resp.interact_pointer_pos() {
                    at = Rect::from_center_size(p, size * 1.06);
                }
            } else if let Some((si, t0)) = self.shake {
                if si == i {
                    let p = cl::motion::DRAG_SHAKE.at(now, t0, 0);
                    if p < 1.0 {
                        ui.ctx().request_repaint();
                        at = at.translate(egui::vec2((p * 5.0 * PI).sin() * (1.0 - p) * 12.0, 0.0));
                    }
                }
            }
            painter.rect_filled(at, at.height() * 0.5, cl::surface::RAISED);
            painter.rect_stroke(
                at,
                at.height() * 0.5,
                egui::Stroke::new(
                    1.0,
                    if dragging {
                        tone.color()
                    } else {
                        cl::surface::HAIRLINE
                    },
                ),
                egui::StrokeKind::Inside,
            );
            painter.galley(at.center() - galley.size() * 0.5, galley, cl::ink::STRONG);

            if resp.drag_stopped() {
                let hit = resp
                    .interact_pointer_pos()
                    .and_then(|p| bucket_rects.iter().position(|b| b.contains(p)));
                drop = Some((i, hit));
            }
        }
        if let Some((i, hit)) = drop {
            self.drop_on(i, hit, now);
        }

        // Counter or the blooming LAND line.
        let foot = egui::pos2(stage.rect.center().x, stage.rect.max.y);
        match self.done_at {
            Some(t0) => {
                let p = cl::motion::REWARD_BLOOM.at(now, t0, 0);
                if p < 1.0 {
                    ui.ctx().request_repaint();
                }
                painter.text(
                    egui::pos2(foot.x, foot.y - land_h * 0.5),
                    Align2::CENTER_CENTER,
                    self.reveal,
                    FontId::proportional(fluid.fs_h4() * (0.92 + 0.12 * p).max(0.05)),
                    cl::ink::BASE.gamma_multiply(p.clamp(0.0, 1.0)),
                );
            }
            None => {
                let placed = self.placed.iter().filter(|p| **p).count();
                painter.text(
                    foot,
                    Align2::CENTER_BOTTOM,
                    tracked(&format!(
                        "{} / {} placed — drag the rest",
                        placed,
                        self.items.len()
                    )),
                    FontId::monospace(fluid.fs_eyebrow()),
                    cl::ink::MUTED,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_locks_wrong_shakes_done_blooms() {
        let mut d = DragMatch::demo();
        d.drop_on(0, Some(1), 1.0);
        assert!(
            !d.placed[0] && d.shake.is_some(),
            "wrong bucket shakes home"
        );
        d.drop_on(0, None, 1.0);
        assert!(!d.placed[0], "dropping on nothing returns quietly");
        for i in 0..4 {
            let t = d.items[i].target;
            d.drop_on(i, Some(t), 2.0);
        }
        assert!(
            d.placed.iter().all(|p| *p) && d.done_at.is_some(),
            "all placed → the bloom"
        );
        d.restart();
        assert!(d.placed.iter().all(|p| !p) && d.done_at.is_none());
    }
}
