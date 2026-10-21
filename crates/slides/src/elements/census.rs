//! The element census — the first rung's registry.
//!
//! Census law (architecture.md §4): anywhere a LIST of things exists, ONE
//! registry function enumerates it and a census test fails when reality and
//! the list drift — nothing else enumerates. [`elements`] is that function for
//! the element tier, mirroring `slides::registry::demos()` for templates. The
//! Library element wall renders this list, so an element that exists but never
//! shows is impossible (the gallery gate below has the teeth).
//!
//! Elements stay ergonomic free functions (`elements::stat(ui, fluid, …)` is
//! unchanged); only a thin, stateless [`ElementDemo`] wraps each with authored
//! sample args so the census, the render gate, and the Library share one path.
//!
//! Adding an element: its module under `elements/`, then a spec + demo + ONE
//! line in [`elements`] here. The `cost_tier`/`category` metadata rides the
//! spec.

use dpl_coldlight::{self as cl, Tone};
use egui::Ui;

/// Element identity — never a bare string (mirrors a component contract's `id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(pub &'static str);

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// The SHELF an element sits on in the Library — its category, not its style
/// (design-system §1). Populated shelves paint; empty shelves stay honestly
/// empty (`Container`/`Control` arrive by promotion / with recipes, later
/// build-order steps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementCategory {
    Type,
    Mark,
    Meter,
    Divider,
    Media,
    Container,
    Control,
}

impl ElementCategory {
    /// The STABLE machine id — never drawn on screen.
    pub const fn slug(self) -> &'static str {
        match self {
            ElementCategory::Type => "type",
            ElementCategory::Mark => "mark",
            ElementCategory::Meter => "meter",
            ElementCategory::Divider => "divider",
            ElementCategory::Media => "media",
            ElementCategory::Container => "container",
            ElementCategory::Control => "control",
        }
    }

    /// The shelf heading the OPERATOR reads. Plain words — "type" and "mark"
    /// are trade terms, and a shelf is no place to teach them.
    pub const fn label(self) -> &'static str {
        match self {
            ElementCategory::Type => "Words",
            ElementCategory::Mark => "Tags",
            ElementCategory::Meter => "Levels",
            ElementCategory::Divider => "Lines",
            ElementCategory::Media => "Pictures",
            ElementCategory::Container => "Boxes",
            ElementCategory::Control => "Buttons",
        }
    }

    /// Shelf order for the Library gallery — EVERY category, so empty ones
    /// render honestly empty (the class-section idiom `library.rs` already
    /// uses for templates).
    pub const ALL: [ElementCategory; 7] = [
        ElementCategory::Type,
        ElementCategory::Mark,
        ElementCategory::Meter,
        ElementCategory::Divider,
        ElementCategory::Media,
        ElementCategory::Container,
        ElementCategory::Control,
    ];
}

/// The fidelity tier an element paints at (design-system §5). `Baseline` is the
/// old-i5 default that runs the whole app; `Enhanced`/`Max` only ADD layers on
/// capable hardware — never changing layout or meaning. Declared now; HONORED
/// by the `Caps→Tier` boot probe in a later build-order step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostTier {
    Baseline,
    Enhanced,
    Max,
}

impl CostTier {
    pub const fn label(self) -> &'static str {
        match self {
            CostTier::Baseline => "baseline",
            CostTier::Enhanced => "enhanced",
            CostTier::Max => "max",
        }
    }
}

/// What the Library shelf shows for an element, plus the metadata the treatment
/// and fidelity layers will read. One `static` per element, below.
#[derive(Debug)]
pub struct ElementSpec {
    pub id: ElementId,
    pub label: &'static str,
    pub category: ElementCategory,
    /// Baseline fidelity tier (design-system §5). Every element is `Baseline`
    /// today — none paints an animated wash or multi-layer glow.
    pub cost_tier: CostTier,
    /// One line — the Library shelf copy.
    pub blurb: &'static str,
}

/// A live, self-contained sample of one element — the census unit. `paint`
/// calls the element's free function with authored demo args, so the Library
/// gallery and the render gate exercise every element with no hand-list.
pub trait ElementDemo {
    fn spec(&self) -> &'static ElementSpec;
    /// Paint the sample into the caller's `ui` (a Library plate, or a test
    /// stage). Painter-driven elements allocate their own rect here.
    fn paint(&self, ui: &mut Ui);
}

// ── the elements ──────────────────────────────────────────────────────────
// Demo tone for every sample — one tone keeps the wall a fair comparison.
const DEMO_TONE: Tone = Tone::Cyan;

static EYEBROW: ElementSpec = ElementSpec {
    id: ElementId("eyebrow"),
    label: "Small label",
    category: ElementCategory::Type,
    cost_tier: CostTier::Baseline,
    blurb: "The little line above a title.",
};
struct EyebrowDemo;
impl ElementDemo for EyebrowDemo {
    fn spec(&self) -> &'static ElementSpec {
        &EYEBROW
    }
    fn paint(&self, ui: &mut Ui) {
        let fluid = cl::Fluid::of(ui.max_rect());
        super::eyebrow::eyebrow(ui, &fluid, "coldlight", Some(DEMO_TONE.color()));
    }
}

static STAT: ElementSpec = ElementSpec {
    id: ElementId("stat"),
    label: "Big number",
    category: ElementCategory::Type,
    cost_tier: CostTier::Baseline,
    blurb: "One number with its unit under it.",
};
struct StatDemo;
impl ElementDemo for StatDemo {
    fn spec(&self) -> &'static ElementSpec {
        &STAT
    }
    fn paint(&self, ui: &mut Ui) {
        let fluid = cl::Fluid::of(ui.max_rect());
        super::stat::stat(ui, &fluid, "128", "tokens", DEMO_TONE.color());
    }
}

static BADGE: ElementSpec = ElementSpec {
    id: ElementId("badge"),
    label: "Tag",
    category: ElementCategory::Mark,
    cost_tier: CostTier::Baseline,
    blurb: "A small pill — draft, live, new.",
};
struct BadgeDemo;
impl ElementDemo for BadgeDemo {
    fn spec(&self) -> &'static ElementSpec {
        &BADGE
    }
    fn paint(&self, ui: &mut Ui) {
        let fluid = cl::Fluid::of(ui.max_rect());
        ui.horizontal(|ui| {
            super::badge::badge(ui, &fluid, "draft", None);
            ui.add_space(cl::space::S2);
            super::badge::badge(ui, &fluid, "live", Some(DEMO_TONE));
        });
    }
}

static BAR: ElementSpec = ElementSpec {
    id: ElementId("bar"),
    label: "Progress bar",
    category: ElementCategory::Meter,
    cost_tier: CostTier::Baseline,
    blurb: "A track that fills to a fraction.",
};
struct BarDemo;
impl ElementDemo for BarDemo {
    fn spec(&self) -> &'static ElementSpec {
        &BAR
    }
    fn paint(&self, ui: &mut Ui) {
        let w = ui.available_width().min(280.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
        super::bar::bar(ui.painter(), rect, 0.62, DEMO_TONE, 1.0);
    }
}

static DIVIDER: ElementSpec = ElementSpec {
    id: ElementId("divider"),
    label: "Line",
    category: ElementCategory::Divider,
    cost_tier: CostTier::Baseline,
    blurb: "A rule that separates two things.",
};
struct DividerDemo;
impl ElementDemo for DividerDemo {
    fn spec(&self) -> &'static ElementSpec {
        &DIVIDER
    }
    fn paint(&self, ui: &mut Ui) {
        let w = ui.available_width().min(280.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 12.0), egui::Sense::hover());
        super::divider::rule(
            ui.painter(),
            rect.left(),
            rect.center().y,
            rect.width(),
            DEMO_TONE,
            1.0,
        );
    }
}

static MEDIA: ElementSpec = ElementSpec {
    id: ElementId("media"),
    label: "Picture",
    category: ElementCategory::Media,
    cost_tier: CostTier::Baseline,
    blurb: "An image or video, sitting behind the words.",
};
struct MediaDemo;
impl ElementDemo for MediaDemo {
    fn spec(&self) -> &'static ElementSpec {
        &MEDIA
    }
    fn paint(&self, ui: &mut Ui) {
        let w = ui.available_width().min(280.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 120.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, cl::RADIUS, cl::surface::WELL);
        let now = ui.input(|i| i.time);
        super::media::ambient_backdrop(
            ui,
            rect,
            super::media::Ambient::Rings,
            DEMO_TONE.color(),
            now,
            0.0,
        );
    }
}

/// One demo per registered element, category order. ONE line per element — the
/// census: an element absent here cannot show in the Library, and the gallery
/// gate below paints every entry.
pub fn elements() -> Vec<Box<dyn ElementDemo>> {
    vec![
        Box::new(EyebrowDemo), // Type
        Box::new(StatDemo),    // Type
        Box::new(BadgeDemo),   // Mark
        Box::new(BarDemo),     // Meter
        Box::new(DividerDemo), // Divider
        Box::new(MediaDemo),   // Media
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_registry_is_the_census() {
        let all = elements();
        assert_eq!(
            all.len(),
            6,
            "6 elements: eyebrow, stat, badge, bar, divider, media"
        );
        let ids: HashSet<&str> = all.iter().map(|e| e.spec().id.0).collect();
        assert_eq!(ids.len(), all.len(), "element ids must be unique");
        for e in &all {
            assert!(!e.spec().label.is_empty());
            assert!(!e.spec().blurb.is_empty());
        }
    }

    /// The GALLERY GATE, teeth (mirrors the template registry's): every
    /// registered element must actually PAINT — at entrance (fresh clock) and
    /// at rest — without panicking. Catches bad rect math / NaN before it ever
    /// reaches a wall.
    #[test]
    fn every_element_paints_at_start_and_at_rest() {
        let ctx = egui::Context::default();
        dpl_coldlight::apply_theme(&ctx);
        for element in elements() {
            for time in [0.05_f64, 3.0_f64] {
                let mut input = egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                };
                input.screen_rect = Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(1280.0, 720.0),
                ));
                ctx.run_ui(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        element.paint(ui);
                    });
                })
                .textures_delta
                .clear();
            }
        }
    }
}
