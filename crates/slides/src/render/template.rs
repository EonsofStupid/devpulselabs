//! A component's own values as a slide, and its gallery card painted by the
//! one renderer.

use super::render;
use egui::Ui;

/// A template's own values (its contract's shape: one key per slot, prop
/// and axis) as a slide document — what its gallery card paints through the
/// one renderer (LS4a: no template hand-paints).
pub fn slide_of(
    contract: &dpl_doc::component::ComponentContract,
    values: &serde_json::Value,
) -> dpl_doc::Slide {
    let pick = |ids: &mut dyn Iterator<Item = &str>| -> serde_json::Map<String, serde_json::Value> {
        ids.filter_map(|id| values.get(id).map(|v| (id.to_owned(), v.clone())))
            .collect()
    };
    let content = pick(&mut contract.slots.iter().map(|s| s.id.as_str()));
    let props = pick(&mut contract.props.iter().map(|p| p.id.as_str()));
    let variant = contract
        .variant_axes
        .iter()
        .filter_map(|axis| {
            values
                .get(&axis.id)
                .and_then(serde_json::Value::as_str)
                .map(|option| (axis.id.clone(), option.to_owned()))
        })
        .collect();
    dpl_doc::slide_from_parts(contract, &content, &props, &variant)
        .unwrap_or_else(|| dpl_doc::Slide::new(format!("{}@1", contract.id)))
}

/// A migrated template's gallery card (LS4a): its own values as a slide,
/// painted by the one renderer against the standalone styling, on the
/// template's clock.
pub(crate) fn paint_template(
    ui: &mut Ui,
    contract: &dpl_doc::component::ComponentContract,
    values: &serde_json::Value,
    clock: &mut crate::motion::Clock,
) {
    static STANDALONE: std::sync::OnceLock<dpl_doc::Styling> = std::sync::OnceLock::new();
    let rect = ui.max_rect();
    let mut slide = slide_of(contract, values);
    // On the bare well, like every gallery card not yet moved: the component,
    // not a theme.
    slide.background.get_or_insert(dpl_doc::Background::None);
    let now = ui.input(|i| i.time);
    let t0 = clock.tick(now);
    render(
        ui,
        rect,
        &slide,
        STANDALONE.get_or_init(dpl_doc::Styling::standalone),
        now,
        t0,
    );
    ui.allocate_rect(rect, egui::Sense::hover());
}
