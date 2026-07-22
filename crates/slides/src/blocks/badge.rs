//! Badge — the small emphasis chip (CalloutBadge port). Neutral = raised
//! surface; live = tone fill with the presence dot.

use dpl_coldlight::{self as cl, Fluid, Tone};
use egui::{Sense, Ui};

use super::eyebrow::tracked;

/// Draw the chip. `live: Some(tone)` = the on-air variant; `None` = neutral.
pub fn badge(ui: &mut Ui, fluid: &Fluid, text: &str, live: Option<Tone>) {
    let font = egui::FontId::monospace(fluid.fs_eyebrow());
    let label = tracked(text);
    let galley = ui.painter().layout_no_wrap(
        label,
        font.clone(),
        match live {
            Some(_) => cl::surface::WELL,
            None => cl::ink::BASE,
        },
    );
    let pad_x = fluid.sp_4();
    let pad_y = fluid.sp_1();
    let dot_w = if live.is_some() { fluid.sp_3() } else { 0.0 };
    let size = egui::vec2(
        galley.size().x + pad_x * 2.0 + dot_w,
        galley.size().y + pad_y * 2.0,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let radius = rect.height() * 0.5; // full pill
    match live {
        Some(tone) => {
            ui.painter().rect_filled(rect, radius, tone.color());
            let dot = egui::pos2(rect.min.x + pad_x + dot_w * 0.25, rect.center().y);
            ui.painter().circle_filled(dot, fluid.sp_1() * 0.8, cl::surface::WELL);
        }
        None => {
            ui.painter().rect_filled(rect, radius, cl::surface::RAISED);
            ui.painter().rect_stroke(
                rect,
                radius,
                egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                egui::StrokeKind::Inside,
            );
        }
    }
    let text_pos = egui::pos2(rect.min.x + pad_x + dot_w, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(text_pos, galley, cl::ink::BASE);
}
