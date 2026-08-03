//! The authoring surface's controls — things you can press that LOOK like
//! things you can press.
//!
//! Before this module the surface had **nine clickable text labels and zero
//! buttons**: `ui.label(..).interact(Sense::click())` everywhere. Every
//! primary action — the dashboard lanes, Back, Put them in, Start a blank
//! card, the shelf chips, the tab rail — was bare text with no fill, no edge,
//! no radius and no tap target. The operator's report was exact: "a wall of
//! text that does nothing to indicate buttons".
//!
//! The ratified mock (R1) puts a fill, a 1px edge, a radius and a ≥44px
//! min-height on every one of them (`.btn`, `.chip`, `.shelf`, `.tab`,
//! `.lane`), and the doctrine already said which treatment to use:
//! **NEUBRUTALIST is the actionable base** — flat saturated fill, crisp hard
//! edge, hard offset shadow. Claymorphic is for passive surfaces content sits
//! IN. A button is actionable, so a button is neubrutalist.
//!
//! Everything here paints through `cl::compose`, so a control picks up hover
//! and pressed states from the trait system rather than inventing them, and
//! the tap target comes from the `tap-min` token rather than a magic pixel
//! count.

use dpl_coldlight::{self as cl, Tone};
use egui::{Response, Sense, Ui};

/// How loud a control is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Weight {
    /// The ordinary control — an edge and a quiet fill.
    Quiet,
    /// The one thing on the surface you most likely want (`.btn.tone`).
    Loud,
}

/// One pressable control: fill, edge, radius, and a real tap target.
///
/// `selected` is the mock's `aria-selected` — a chip that is currently on
/// wears the tone on its edge and its text, so "which one is active" is
/// visible without reading.
pub(crate) fn control(
    ui: &mut Ui,
    label: &str,
    selected: bool,
    tone: Tone,
    weight: Weight,
) -> Response {
    let fluid = cl::Fluid::of(ui.max_rect());
    let pad_x = cl::space::S4;
    let font = egui::FontId::proportional(fluid.fs_body());

    // Measure first: a control is as wide as its words plus padding, and
    // never shorter than the tap token. Touch-first is a law, not a nicety.
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), cl::ink::BASE);
    let w = galley.size().x + pad_x * 2.0;
    let h = fluid.tap_min().max(galley.size().y + cl::space::S3 * 2.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());

    let state = if resp.is_pointer_button_down_on() {
        cl::TraitState::Active
    } else if resp.hovered() {
        cl::TraitState::Hover
    } else {
        cl::TraitState::Idle
    };
    // Actionable == neubrutalist (doctrine). The stack is empty: a control
    // wears the base treatment, not a theme the operator picked for a card.
    let style = cl::compose(tone, "neubrutalist", &[], state);
    let radius = egui::CornerRadius::same(style.radius);
    let painter = ui.painter();

    // The hard offset shadow IS the affordance — it reads as a thing sitting
    // above the surface, which is the whole point of the treatment.
    for sh in &style.shadows {
        painter.rect_filled(
            rect.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
            radius,
            sh.color,
        );
    }
    painter.rect_filled(rect, radius, style.fill);
    let edge = if selected || weight == Weight::Loud {
        egui::Stroke::new(2.0, tone.color())
    } else {
        style.edge
    };
    painter.rect_stroke(rect, radius, edge, egui::StrokeKind::Inside);
    painter.galley(
        egui::pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        if selected || weight == Weight::Loud {
            tone.color()
        } else {
            style.ink
        },
    );

    // A painted control carries no name of its own — say what it is, or
    // neither a screen reader nor the headless harness can find it.
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label)
    });
    ui.add_space(cl::space::S2);
    resp
}

/// A control with a second line under its title — the mock's `.lane` and
/// `.shelf`, where the description is part of the control rather than beside
/// it.
pub(crate) fn stacked_control(
    ui: &mut Ui,
    title: &str,
    sub: &str,
    foot: &str,
    selected: bool,
    tone: Tone,
    w: f32,
) -> Response {
    let fluid = cl::Fluid::of(ui.max_rect());
    let pad = cl::space::S4;
    let title_font = egui::FontId::proportional(fluid.fs_h4());
    let body_font = egui::FontId::proportional(fluid.fs_body_sm());

    let inner = (w - pad * 2.0).max(fluid.tap_min());
    let t = ui
        .painter()
        .layout(title.to_owned(), title_font, cl::ink::STRONG, inner);
    let s = ui
        .painter()
        .layout(sub.to_owned(), body_font.clone(), cl::ink::BASE, inner);
    let f = ui
        .painter()
        .layout(foot.to_owned(), body_font, cl::ink::SUBTLE, inner);
    let h = (t.size().y + s.size().y + f.size().y + cl::space::S3 * 2.0 + pad * 2.0)
        .max(fluid.tap_min());

    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
    let state = if resp.is_pointer_button_down_on() {
        cl::TraitState::Active
    } else if resp.hovered() {
        cl::TraitState::Hover
    } else {
        cl::TraitState::Idle
    };
    let style = cl::compose(tone, "neubrutalist", &[], state);
    let radius = egui::CornerRadius::same(style.radius);
    let painter = ui.painter();
    for sh in &style.shadows {
        painter.rect_filled(
            rect.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
            radius,
            sh.color,
        );
    }
    painter.rect_filled(rect, radius, style.fill);
    painter.rect_stroke(
        rect,
        radius,
        if selected {
            egui::Stroke::new(2.0, tone.color())
        } else {
            style.edge
        },
        egui::StrokeKind::Inside,
    );

    let mut y = rect.min.y + pad;
    painter.galley(egui::pos2(rect.min.x + pad, y), t.clone(), tone.color());
    y += t.size().y + cl::space::S2;
    painter.galley(egui::pos2(rect.min.x + pad, y), s.clone(), cl::ink::BASE);
    y += s.size().y + cl::space::S2;
    painter.galley(egui::pos2(rect.min.x + pad, y), f, cl::ink::SUBTLE);

    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, title)
    });
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every control clears the tap token. Touch-first is a law and the mock
    /// puts a min-height on every pressable thing.
    #[test]
    fn a_control_is_never_smaller_than_the_tap_target() {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(800.0, 600.0))
            .build_ui(|ui| {
                let r = control(ui, "Put them in", false, Tone::Amber, Weight::Quiet);
                let tap = cl::Fluid::of(ui.max_rect()).tap_min();
                assert!(
                    r.rect.height() >= tap - 0.5,
                    "control {} high, tap-min {tap}",
                    r.rect.height()
                );
            });
        harness.run();
    }

    /// A painted control must still be findable by name — screen readers and
    /// the headless harness both read `widget_info`.
    #[test]
    fn a_control_says_what_it_is() {
        use egui_kittest::kittest::Queryable as _;
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(400.0, 200.0))
            .build_ui(|ui| {
                control(ui, "Start a blank card", false, Tone::Amber, Weight::Loud);
            });
        harness.run();
        harness.get_by_label("Start a blank card");
    }
}
