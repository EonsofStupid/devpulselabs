//! The template registry — the catalog's census.
//!
//! A template = a `'static` spec + a stateful instance. [`demos`] constructs
//! one live instance of every registered template with its authored demo data
//! — the Library gallery renders that list, so a template that exists but
//! never shows is impossible (the gallery gate).
//!
//! Adding a template: its module under `templates/` or `infographics/`, then
//! ONE line in [`demos`]. Nothing else enumerates templates anywhere.

use crate::infographics;
use crate::templates;

/// Template identity — never a bare string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TemplateId(pub &'static str);

impl std::fmt::Display for TemplateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Doctrine taxonomy, carried: "interactive" is what the episode budget
/// counts. Flourishes (a stat flip, a donut slice tap) stay in their class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateClass {
    Static,
    Split,
    Animation,
    Interactive,
}

impl TemplateClass {
    pub const fn label(self) -> &'static str {
        match self {
            TemplateClass::Static => "static",
            TemplateClass::Split => "split",
            TemplateClass::Animation => "animation",
            TemplateClass::Interactive => "interactive",
        }
    }
}

/// What the gallery (and later the Workbench palette) shows for a template.
#[derive(Debug)]
pub struct TemplateSpec {
    pub id: TemplateId,
    pub label: &'static str,
    pub class: TemplateClass,
    /// One line — the same sentence the old registration's `description` carried.
    pub blurb: &'static str,
}

/// A live slide surface. Instances own their interaction state; `ui` draws
/// into the caller's stage rect (`ui.max_rect()`), sized via `Fluid::of`.
pub trait SlideTemplate {
    fn spec(&self) -> &'static TemplateSpec;
    fn ui(&mut self, ui: &mut egui::Ui);
    /// Replay: reset the slide's clock AND its interaction state, so the
    /// next frame runs every motion effect from zero. The preview flow calls
    /// this on entry — clicking a slide always shows its full animation.
    fn restart(&mut self) {}
}

/// One live demo instance of EVERY registered template, registry order:
/// templates first, then the infographic tranche. One line per template.
pub fn demos() -> Vec<Box<dyn SlideTemplate>> {
    vec![
        Box::new(templates::title_card::TitleCard::demo()),
        Box::new(templates::big_number::BigNumber::demo()),
        Box::new(infographics::donut_share::DonutShare::demo()),
        Box::new(infographics::bar_compare::BarCompare::demo()),
        Box::new(infographics::line_trend::LineTrend::demo()),
        Box::new(infographics::probability_bars::ProbabilityBars::demo()),
        Box::new(infographics::stack_layers::StackLayers::demo()),
        Box::new(infographics::cost_meter_face::CostMeterFace::demo()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_registry_is_the_census() {
        let all = demos();
        assert_eq!(all.len(), 8, "S1 ships 2 templates + 6 infographics");
        let ids: HashSet<&str> = all.iter().map(|t| t.spec().id.0).collect();
        assert_eq!(ids.len(), all.len(), "template ids must be unique");
        for t in &all {
            assert!(!t.spec().label.is_empty());
            assert!(!t.spec().blurb.is_empty());
        }
    }
}
