//! The template registry — the catalog's census.
//!
//! A template = its contract (`component.json`, `dpl.component/1`) + a
//! stateful instance. [`demos`] constructs one live instance of every
//! registered template from its contract's first example — the Library
//! gallery renders that list, so a template that exists but never shows is
//! impossible (the gallery gate).
//!
//! Adding a component: its folder under `components/`, named by its contract
//! id, holding its `component.json` and `mod.rs`; one `pub mod` line in
//! `components/mod.rs`; then ONE line in [`demos`]. Nothing else enumerates
//! components anywhere; [`components`] is derived from it.

use crate::components;
use dpl_doc::component::ComponentContract;

/// A template's current values in its contract's shape: one key per slot,
/// prop and axis id. Every template has it, because every template is
/// `Serialize` — the census test compares these keys with the contract, and
/// later passes read a live slide's values through it.
pub trait ComponentValues {
    fn values(&self) -> serde_json::Value;
}

impl<T: serde::Serialize> ComponentValues for T {
    fn values(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("a template's authored fields serialise")
    }
}

/// A live slide surface. Instances own their interaction state; `ui` draws
/// into the caller's stage rect (`ui.max_rect()`), sized via `Fluid::of`.
pub trait SlideTemplate: ComponentValues {
    /// The template's contract (`dpl.component/1`): its name, class,
    /// summary, slots, props, axes and examples — the one source of truth.
    fn spec(&self) -> &'static ComponentContract;
    fn ui(&mut self, ui: &mut egui::Ui);
    /// Replay: reset the slide's clock AND its interaction state, so the
    /// next frame runs every motion effect from zero. The preview flow calls
    /// this on entry — clicking a slide always shows its full animation.
    fn restart(&mut self) {}
}

/// One live demo instance of EVERY registered template, registry order:
/// layouts first, then the charts. One line per component.
pub fn demos() -> Vec<Box<dyn SlideTemplate>> {
    vec![
        Box::new(components::title::TitleCard::demo()),
        Box::new(components::media::MediaFrame::demo()),
        Box::new(components::quote::Quote::demo()),
        Box::new(components::section::SectionDivider::demo()),
        Box::new(components::agenda::Agenda::demo()),
        Box::new(components::stat::BigNumber::demo()),
        Box::new(components::callout::CalloutCard::demo()),
        Box::new(components::split::SplitPanel::demo()),
        Box::new(components::compare::CompareVs::demo()),
        Box::new(components::features::FeatureGrid::demo()),
        Box::new(components::timeline::Timeline::demo()),
        Box::new(components::checklist::Checklist::demo()),
        Box::new(components::reveal::RevealList::demo()),
        Box::new(components::choice::ChoicePoll::demo()),
        Box::new(components::quiz::QuizPanel::demo()),
        Box::new(components::stats::StatsGrid::demo()),
        Box::new(components::code::CodeBlock::demo()),
        Box::new(components::cost_meter::CostMeter::demo()),
        Box::new(components::predict::PredictRank::demo()),
        Box::new(components::drag_match::DragMatch::demo()),
        Box::new(components::donut_share::DonutShare::demo()),
        Box::new(components::gauge_dial::GaugeDial::demo()),
        Box::new(components::radial_progress::RadialProgress::demo()),
        Box::new(components::bar_compare::BarCompare::demo()),
        Box::new(components::line_trend::LineTrend::demo()),
        Box::new(components::probability_bars::ProbabilityBars::demo()),
        Box::new(components::stack_layers::StackLayers::demo()),
        Box::new(components::cost_meter_face::CostMeterFace::demo()),
    ]
}

/// Every registered component's contract, registry order — the manifest the
/// Workbench reads ("only show what exists").
pub fn components() -> Vec<&'static ComponentContract> {
    demos().iter().map(|template| template.spec()).collect()
}

/// The contract registered under `id` — read once, then looked up without
/// building a template, because the renderer asks every frame (LS4a).
pub fn contract_of(id: &str) -> Option<&'static ComponentContract> {
    static ALL: std::sync::OnceLock<Vec<&'static ComponentContract>> = std::sync::OnceLock::new();
    ALL.get_or_init(components)
        .iter()
        .copied()
        .find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_doc::component::{self as contract, declared_ids, example_values};
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    fn component_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read the slides source tree") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                component_files(&path, out);
            } else if path
                .file_name()
                .is_some_and(|name| name == "component.json")
            {
                out.push(path);
            }
        }
    }

    /// An absent optional slot serialises as null, "" or [] depending on
    /// its type; all three mean "empty" and compare equal to a missing key.
    /// Numbers compare at `f32` precision.
    fn without_nulls(value: &Value) -> Value {
        let empty = |v: &Value| {
            v.is_null()
                || v.as_str().is_some_and(str::is_empty)
                || v.as_array().is_some_and(Vec::is_empty)
        };
        match value {
            Value::Object(map) => Value::Object(
                map.iter()
                    .filter(|(_, v)| !empty(v))
                    .map(|(k, v)| (k.clone(), without_nulls(v)))
                    .collect(),
            ),
            Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
            // Templates hold `f32`: compare numbers at the precision they keep.
            Value::Number(number) => number
                .as_f64()
                .and_then(|n| serde_json::Number::from_f64(f64::from(n as f32)))
                .map_or_else(|| value.clone(), Value::Number),
            other => other.clone(),
        }
    }

    #[test]
    fn the_registry_is_the_census() {
        let all = demos();
        assert_eq!(
            all.len(),
            28,
            "20 templates + 8 infographics (diverse-library pass added 7 templates + 2 infographics)"
        );
        for t in &all {
            assert!(!t.spec().name.is_empty());
            assert!(!t.spec().summary.is_empty());
        }
    }

    /// The GALLERY GATE, teeth: every registered template must actually
    /// PAINT — at the start of its animation (alpha 0, fresh clock) and at
    /// rest (settled) — without panicking. Catches bad arc math, NaN rects,
    /// and galley layout errors that construction alone never exercises.
    #[test]
    fn every_demo_renders_at_start_and_at_rest() {
        let ctx = egui::Context::default();
        dpl_coldlight::apply_theme(&ctx);
        for mut demo in demos() {
            for time in [0.05_f64, 3.0_f64] {
                demo.restart();
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
                        demo.ui(ui);
                    });
                })
                .textures_delta
                .clear();
            }
        }
    }

    /// The folder pattern (`docs/design/design-system.md` §4.7): under
    /// `components/` there is `mod.rs` and one folder per component; a
    /// folder is named by its contract id and holds only the fixed file
    /// names; `mod.rs` declares exactly those folders, in order.
    #[test]
    fn every_component_folder_follows_the_pattern() {
        const ALLOWED: [&str; 4] = ["component.json", "mod.rs", "paint.rs", "state.rs"];
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/components");
        let mut folders = Vec::new();
        for entry in std::fs::read_dir(&root).expect("read components/") {
            let path = entry.expect("dir entry").path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a UTF-8 name")
                .to_owned();
            if path.is_file() {
                assert_eq!(name, "mod.rs", "components/ holds mod.rs and folders only");
                continue;
            }
            let text = std::fs::read_to_string(path.join("component.json"))
                .unwrap_or_else(|_| panic!("components/{name} has no component.json"));
            let id = contract::parse(&text).expect("component.json parses").id;
            assert_eq!(
                id.replace('-', "_"),
                name,
                "the folder is named by its contract id"
            );
            assert!(path.join("mod.rs").is_file(), "components/{name}/mod.rs");
            for file in std::fs::read_dir(&path).expect("read the component folder") {
                let file = file.expect("dir entry").file_name();
                let file = file.to_string_lossy();
                assert!(
                    ALLOWED.contains(&file.as_ref()),
                    "components/{name}/{file} is not one of {ALLOWED:?}"
                );
            }
            folders.push(name);
        }
        folders.sort();
        let listed: Vec<String> = std::fs::read_to_string(root.join("mod.rs"))
            .expect("components/mod.rs")
            .lines()
            .filter(|line| !line.starts_with("//!") && !line.trim().is_empty())
            .map(|line| {
                line.strip_prefix("pub mod ")
                    .and_then(|rest| rest.strip_suffix(';'))
                    .unwrap_or_else(|| {
                        panic!("components/mod.rs holds only `pub mod` lines: {line}")
                    })
                    .to_owned()
            })
            .collect();
        assert_eq!(
            listed, folders,
            "components/mod.rs lists every folder, in order"
        );
        let registered: BTreeSet<String> = components()
            .iter()
            .map(|c| c.id.replace('-', "_"))
            .collect();
        assert_eq!(
            registered,
            folders.into_iter().collect::<BTreeSet<_>>(),
            "every folder is registered in demos()"
        );
    }

    /// Every `component.json` on disk passes the JSON Schema AND the rules a
    /// schema cannot say — and each one belongs to a registered template.
    #[test]
    fn every_contract_passes_its_schema_and_its_rules() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let schema: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("../../schemas/component.v1.schema.json"))
                .expect("the component schema"),
        )
        .expect("the schema is JSON");
        let validator = jsonschema::validator_for(&schema).expect("the schema compiles");
        let mut files = Vec::new();
        component_files(&root.join("src"), &mut files);
        let registered: BTreeSet<String> = components().iter().map(|c| c.id.clone()).collect();
        let mut on_disk = BTreeSet::new();
        for file in &files {
            let text = std::fs::read_to_string(file).expect("read component.json");
            let doc: Value = serde_json::from_str(&text).expect("component.json is JSON");
            let errors: Vec<String> = validator.iter_errors(&doc).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "{}: {errors:?}", file.display());
            let parsed = contract::parse(&text).expect("component.json parses");
            let issues = contract::validate(&parsed);
            assert!(issues.is_empty(), "{}: {issues:?}", file.display());
            assert!(
                on_disk.insert(parsed.id.clone()),
                "{} is declared twice",
                parsed.id
            );
        }
        assert_eq!(
            on_disk, registered,
            "contracts on disk vs registered templates"
        );
    }

    /// The registry and the contracts match one-to-one, in registry order.
    #[test]
    fn the_registry_has_one_contract_per_template() {
        let ids: Vec<&str> = components().iter().map(|c| c.id.as_str()).collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len(), "duplicate template ids: {ids:?}");
        assert_eq!(ids.len(), demos().len());
    }

    /// A template's serde shape IS its contract: every field is a declared
    /// slot, prop or axis, and every declared id is a field.
    #[test]
    fn every_template_declares_exactly_its_fields() {
        for demo in demos() {
            let spec = demo.spec();
            let keys: BTreeSet<String> = match demo.values() {
                Value::Object(map) => map.keys().cloned().collect(),
                other => panic!("{} serialises to {other}", spec.id),
            };
            assert_eq!(keys, declared_ids(spec), "{}", spec.id);
        }
    }

    /// LS2: every seed fills a real component — its blocks are that
    /// component's slots, and nothing it carries is refused.
    #[test]
    fn every_seed_fills_its_component_contract() {
        let seeds = dpl_doc::list_seeds();
        assert!(!seeds.is_empty(), "the repo ships seeds");
        for seed in seeds {
            let slide = dpl_doc::load(&seed.dir).expect("a seed loads as v3");
            let contract = components()
                .into_iter()
                .find(|c| c.id == slide.component_id())
                .unwrap_or_else(|| panic!("{}: no component {}", seed.id, slide.component));
            let issues = dpl_doc::validate_against(&slide, contract);
            assert!(issues.is_empty(), "{}: {issues:?}", seed.id);
        }
    }

    /// LS2: tapping a Layout starts from its contract's first example — and
    /// that starting slide is valid, against its contract too.
    #[test]
    fn every_first_example_becomes_a_valid_slide() {
        for contract in components() {
            let slide = dpl_doc::slide_from_example(contract, 0)
                .unwrap_or_else(|| panic!("{} has an example", contract.id));
            assert_eq!(dpl_doc::validate(&slide), Vec::new(), "{}", contract.id);
            assert_eq!(
                dpl_doc::validate_against(&slide, contract),
                Vec::new(),
                "{}",
                contract.id
            );
        }
    }

    /// The gallery demo IS the contract's first example — no hidden data in
    /// code — and the values survive a round trip unchanged.
    #[test]
    fn every_demo_is_its_first_example() {
        for demo in demos() {
            let spec = demo.spec();
            let expected = example_values(spec, 0).expect("an example");
            assert_eq!(
                without_nulls(&demo.values()),
                without_nulls(&expected),
                "{}",
                spec.id
            );
        }
    }
}
