//! DonutShare — share-of-whole donut ("where inference runs").
//!
//! The vector engine's birth certificate. Upgrades over the SVG original:
//! arcs are crisp ring-sector MESHES with a radial gradient (inner edge one
//! ladder step lighter — sprite depth, no blur); slices SWEEP in, staggered,
//! as a pure function of the clock (presenter law: after the sweep, rest);
//! and slices are TAPPABLE — a tap explodes the slice and puts its readout
//! in the center; tapping again returns to the authored center.

use dpl_coldlight::{self as cl, Tone};
use dpl_vector::ring_sector;
use egui::{Align2, Sense, Ui};
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, TAU};

use super::stage::{ChartStage, LOGICAL_H, LOGICAL_W};
use super::tone_cycle::series_tone;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// One slice — an item of the `slices` list slot.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    pub label: String,
    /// Any scale: a slice is drawn as its share of the slices' sum.
    pub value: f32,
    /// `None` = the series cycle under the slide's tone.
    #[serde(default, with = "crate::contract::tone_opt")]
    pub tone: Option<Tone>,
}

/// Fields are the contract's slots and props (`component.json`); its demo
/// example is the authored EOS-001 data, carried.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DonutShare {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    #[serde(default)]
    pub center: String,
    #[serde(default)]
    pub center_note: String,
    pub slices: Vec<Slice>,
    #[serde(skip)]
    clock: Clock,
    #[serde(skip)]
    selected: Option<usize>,
}

impl DonutShare {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0)
            .expect("the donut-share demo is its example")
    }

    fn slice_tone(&self, i: usize) -> Tone {
        self.slices[i]
            .tone
            .unwrap_or_else(|| series_tone(i, self.tone))
    }
}

/// Slice fill: outer = the tone's 400 step, inner = 300 (one step lighter —
/// the radial sheen that makes the ring read as a solid object).
fn slice_colors(tone: Tone) -> (egui::Color32, egui::Color32) {
    let ramp = cl::ramp(tone.hue_id()).expect("tone hue is in the pool");
    (ramp.steps[cl::STEP_500 - 2], ramp.steps[cl::STEP_500 - 1])
}

impl SlideTemplate for DonutShare {
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
    }

    fn restart(&mut self) {
        self.clock.restart();
        self.selected = None;
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = ChartStage::fit(ui.max_rect());
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        let painter = ui.painter();

        let center = stage.pt(LOGICAL_W * 0.5, LOGICAL_H * 0.5);
        let radius = stage.len((LOGICAL_W.min(LOGICAL_H)) * 0.5 - 48.0);
        let r_in = radius * 0.62;
        let total: f32 = self
            .slices
            .iter()
            .map(|s| s.value)
            .sum::<f32>()
            .max(f32::EPSILON);
        const PAD: f32 = 0.02; // radians shaved off each slice edge (ported)
        const EXPLODE: f32 = 10.0; // logical offset of the selected slice

        // Geometry + the sweep. Angles start at 12 o'clock, clockwise.
        let sweep = cl::motion::CHART_SWEEP;
        let mut a = -FRAC_PI_2;
        for (i, slice) in self.slices.iter().enumerate() {
            let full = (slice.value / total) * TAU;
            let progress = sweep.at(now, t0, i);
            let a0 = a + PAD * 0.5;
            let a1 = a + (full - PAD * 0.5).max(PAD) * progress;
            let tone = self.slice_tone(i);
            let (inner, outer) = slice_colors(tone);
            let bisector = egui::Vec2::angled(a + full * 0.5);
            let c = if self.selected == Some(i) {
                center + bisector * stage.len(EXPLODE)
            } else {
                center
            };
            painter.add(egui::Shape::mesh(ring_sector(
                c, r_in, radius, a0, a1, inner, outer,
            )));

            // The datum label rides outside the ring, anchored AWAY from it:
            // right-side labels grow rightward, left-side leftward — text
            // never crosses back over the arcs.
            let label_at = center + bisector * (radius * 1.06 + stage.len(18.0));
            let pct = ((slice.value / total) * 100.0).round() as i32;
            let galley = painter.layout_no_wrap(
                crate::elements::eyebrow::tracked(&format!("{} · {}%", slice.label, pct)),
                egui::FontId::monospace(stage.len(17.0)),
                cl::ink::BASE,
            );
            // Anchor away from the ring, then CLAMP fully inside the stage —
            // a label never crosses the arcs and never leaves the slide.
            let size = galley.size();
            let mut pos = if bisector.x.abs() < 0.35 {
                egui::pos2(
                    label_at.x - size.x * 0.5,
                    if bisector.y < 0.0 {
                        label_at.y - size.y
                    } else {
                        label_at.y
                    },
                )
            } else if bisector.x > 0.0 {
                egui::pos2(label_at.x, label_at.y - size.y * 0.5)
            } else {
                egui::pos2(label_at.x - size.x, label_at.y - size.y * 0.5)
            };
            pos.x = pos.x.clamp(stage.rect.min.x, stage.rect.max.x - size.x);
            pos.y = pos.y.clamp(stage.rect.min.y, stage.rect.max.y - size.y);
            painter.galley(pos, galley, cl::ink::BASE);
            a += full;
        }
        if sweep.in_flight(now, t0, self.slices.len()) {
            ui.ctx().request_repaint();
        }

        // Center readout: the authored center, or the tapped slice's.
        match self.selected {
            Some(i) => {
                let tone = self.slice_tone(i);
                let pct = ((self.slices[i].value / total) * 100.0).round() as i32;
                stage.display_text(
                    painter,
                    (LOGICAL_W * 0.5, LOGICAL_H * 0.5 - 14.0),
                    Align2::CENTER_CENTER,
                    &format!("{pct}%"),
                    58.0,
                    tone.color(),
                );
                stage.mono_label(
                    painter,
                    (LOGICAL_W * 0.5, LOGICAL_H * 0.5 + 34.0),
                    Align2::CENTER_CENTER,
                    &self.slices[i].label,
                    13.0,
                    cl::ink::MUTED,
                );
            }
            None => {
                stage.display_text(
                    painter,
                    (LOGICAL_W * 0.5, LOGICAL_H * 0.5 - 10.0),
                    Align2::CENTER_CENTER,
                    &self.center,
                    58.0,
                    self.tone.color(),
                );
                // Sized to stay inside the hole (the note is the longest line).
                stage.mono_label(
                    painter,
                    (LOGICAL_W * 0.5, LOGICAL_H * 0.5 + 36.0),
                    Align2::CENTER_CENTER,
                    &self.center_note,
                    12.0,
                    cl::ink::MUTED,
                );
            }
        }

        // Tap: polar hit-test against the ring; a hit toggles the slice.
        let response = ui.allocate_rect(stage.rect, Sense::click());
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let v = pos - center;
                let dist = v.length();
                if dist >= r_in * 0.9 && dist <= radius + stage.len(EXPLODE) {
                    let mut angle = v.angle() + FRAC_PI_2; // 0 at 12 o'clock
                    if angle < 0.0 {
                        angle += TAU;
                    }
                    let mut acc = 0.0;
                    for (i, slice) in self.slices.iter().enumerate() {
                        acc += (slice.value / total) * TAU;
                        if angle <= acc {
                            self.selected = if self.selected == Some(i) {
                                None
                            } else {
                                Some(i)
                            };
                            break;
                        }
                    }
                } else {
                    self.selected = None;
                }
            }
        }
    }
}
