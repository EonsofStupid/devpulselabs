//! Component contracts (`dpl.component/1`, LS1) — the document model of a
//! slide component: its slots, props, variant axes and examples, authored as
//! `component.json` beside each template's code in `dpl-slides` (R44/R47).
//!
//! The contract is the template's source of truth (R170): the gallery demo
//! IS the contract's first example, and the Workbench reads names, classes
//! and summaries from it — nothing is written twice. Content goes in slots,
//! choices in props, looks in variant axes; never colours or positions.
//!
//! It lives in the doc layer (egui-free) so the ONE evolution rule
//! ([`evolution_problems`]) serves both the slides crate's tests and the
//! `contracts` phase of `cargo green`, which compares every contract with
//! its copy on `main` (docs/research/lesson-slide-schema.md §5).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The `schema` value every contract carries.
pub const COMPONENT_SCHEMA: &str = "dpl.component/1";

/// Doctrine taxonomy, carried: "interactive" is what the episode budget
/// counts. Flourishes (a stat flip, a donut slice tap) stay in their class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateClass {
    Static,
    Split,
    Animation,
    Interactive,
}

impl TemplateClass {
    pub const ALL: [TemplateClass; 4] = [
        TemplateClass::Static,
        TemplateClass::Split,
        TemplateClass::Animation,
        TemplateClass::Interactive,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            TemplateClass::Static => "static",
            TemplateClass::Split => "split",
            TemplateClass::Animation => "animation",
            TemplateClass::Interactive => "interactive",
        }
    }
}

/// What a slot holds — the entity a preset and its overrides address (LS2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Entity {
    Text,
    Picture,
    Quote,
    Data,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Number,
    Bool,
    Tone,
    /// A list of numbers (a chart series).
    Numbers,
    /// A nested list; its items' fields are the field's own `item`.
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropKind {
    Text,
    Number,
    Bool,
    Enum,
    Tone,
}

/// One component's contract (`dpl.component/1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentContract {
    pub schema: String,
    /// Permanent; equals the template id the registry knows.
    pub id: String,
    pub version: String,
    pub name: String,
    pub summary: String,
    pub class: TemplateClass,
    #[serde(default)]
    pub tags: Vec<String>,
    pub slots: Vec<Slot>,
    pub props: Vec<Prop>,
    pub variant_axes: Vec<Axis>,
    #[serde(default)]
    pub forbidden: Vec<Forbidden>,
    pub examples: Vec<Example>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: String,
    pub label: String,
    pub entity: Entity,
    pub required: bool,
    pub tooltip: String,
    #[serde(default)]
    pub limits: Limits,
    /// A list slot's item fields, or the fields of the one object a `data`
    /// slot holds. Values are checked against them, all the way down.
    #[serde(default)]
    pub item: Vec<Field>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default)]
    pub max_chars: Option<usize>,
    #[serde(default)]
    pub min_items: Option<usize>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub kind: FieldKind,
    pub tooltip: String,
    /// A `list` field's item fields.
    #[serde(default)]
    pub item: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prop {
    pub id: String,
    pub label: String,
    pub kind: PropKind,
    pub default: Value,
    pub tooltip: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    pub id: String,
    pub label: String,
    pub tooltip: String,
    pub options: Vec<AxisOption>,
    pub default: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Forbidden {
    pub when: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub content: Map<String, Value>,
    #[serde(default)]
    pub props: Map<String, Value>,
    #[serde(default)]
    pub variant: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    #[serde(default)]
    pub forked_from: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// One problem with a contract, at a path an author can find.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractIssue {
    pub at: String,
    pub problem: String,
}

impl std::fmt::Display for ContractIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.at, self.problem)
    }
}

/// Parse one `component.json`.
pub fn parse(text: &str) -> Result<ComponentContract, serde_json::Error> {
    serde_json::from_str(text)
}

/// The template's values for example `index`: the example's content, props
/// and axis choices over the contract's defaults, as one object keyed by
/// slot, prop and axis id — exactly the template's serde shape.
pub fn example_values(contract: &ComponentContract, index: usize) -> Option<Value> {
    let example = contract.examples.get(index)?;
    let mut values = Map::new();
    for prop in &contract.props {
        values.insert(prop.id.clone(), prop.default.clone());
    }
    for axis in &contract.variant_axes {
        values.insert(axis.id.clone(), Value::String(axis.default.clone()));
    }
    for (key, value) in example.content.iter().chain(&example.props) {
        values.insert(key.clone(), value.clone());
    }
    for (key, option) in &example.variant {
        values.insert(key.clone(), Value::String(option.clone()));
    }
    Some(Value::Object(values))
}

/// Every slot, prop and axis id — the keys a template's serde shape must
/// have, no more and no fewer.
pub fn declared_ids(contract: &ComponentContract) -> BTreeSet<String> {
    contract
        .slots
        .iter()
        .map(|slot| slot.id.clone())
        .chain(contract.props.iter().map(|prop| prop.id.clone()))
        .chain(contract.variant_axes.iter().map(|axis| axis.id.clone()))
        .collect()
}

/// `1.2.3` → `(1, 2, 3)`.
pub fn semver(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let parsed = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(parsed)
}

fn is_kebab(value: &str) -> bool {
    is_joined(value, '-')
}

fn is_snake(value: &str) -> bool {
    is_joined(value, '_')
}

fn is_joined(value: &str, joiner: char) -> bool {
    let mut parts = value.split(joiner);
    let first_ok = parts.next().is_some_and(|first| {
        first
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
            && first
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    });
    first_ok
        && parts.all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// A tone by its authored hue id (`"amber"`).
pub fn tone_from_id(id: &str) -> Option<dpl_coldlight::Tone> {
    dpl_coldlight::Tone::ALL
        .into_iter()
        .find(|tone| tone.hue_id() == id)
}

fn is_tone(value: &str) -> bool {
    tone_from_id(value).is_some()
}

/// The rules a JSON Schema cannot say. Empty means the contract is sound.
pub fn validate(contract: &ComponentContract) -> Vec<ContractIssue> {
    let mut issues = Vec::new();
    let id = &contract.id;
    let mut push = |at: String, problem: String| issues.push(ContractIssue { at, problem });
    let at = |path: &str| format!("{id}.{path}");

    if contract.schema != COMPONENT_SCHEMA {
        push(at("schema"), format!("must be {COMPONENT_SCHEMA}"));
    }
    if !is_kebab(id) {
        push(at("id"), "must be kebab-case".into());
    }
    if semver(&contract.version).is_none() {
        push(at("version"), "must be x.y.z".into());
    }
    for tag in &contract.tags {
        if !is_kebab(tag) {
            push(at("tags"), format!("`{tag}` must be kebab-case"));
        }
    }
    if let Some(forked) = &contract.provenance.forked_from {
        let sound = forked
            .split_once('@')
            .is_some_and(|(from, version)| is_kebab(from) && semver(version).is_some());
        if !sound {
            push(at("provenance.forked_from"), "must be id@x.y.z".into());
        }
    }

    // One namespace: slots, props and axes flatten into the template's
    // serde shape, so an id may appear once across all three.
    let mut seen = BTreeSet::new();
    let all_ids = contract
        .slots
        .iter()
        .map(|slot| {
            (
                format!("slots.{}", slot.id),
                &slot.id,
                &slot.label,
                &slot.tooltip,
            )
        })
        .chain(contract.props.iter().map(|prop| {
            (
                format!("props.{}", prop.id),
                &prop.id,
                &prop.label,
                &prop.tooltip,
            )
        }))
        .chain(contract.variant_axes.iter().map(|axis| {
            (
                format!("variant_axes.{}", axis.id),
                &axis.id,
                &axis.label,
                &axis.tooltip,
            )
        }));
    for (path, field, label, tooltip) in all_ids {
        if !is_snake(field) {
            push(at(&path), "id must be snake_case".into());
        }
        if !seen.insert(field.clone()) {
            push(
                at(&path),
                "id is used twice across slots, props and axes".into(),
            );
        }
        if label.trim().is_empty() || tooltip.trim().is_empty() {
            push(at(&path), "needs a label and a tooltip".into());
        }
    }

    for slot in &contract.slots {
        let path = format!("slots.{}", slot.id);
        let is_list = slot.entity == Entity::List;
        if is_list && slot.item.is_empty() {
            push(at(&path), "a list slot names its item fields".into());
        }
        if !matches!(slot.entity, Entity::List | Entity::Data) && !slot.item.is_empty() {
            push(
                at(&path),
                "only list and data slots have item fields".into(),
            );
        }
        if !is_list && (slot.limits.min_items.is_some() || slot.limits.max_items.is_some()) {
            push(at(&path), "item limits belong to list slots".into());
        }
        if let (Some(min), Some(max)) = (slot.limits.min_items, slot.limits.max_items) {
            if min > max {
                push(at(&path), "min_items is above max_items".into());
            }
        }
        for (at_field, problem) in field_problems(&slot.item, &format!("{path}.item")) {
            push(at(&at_field), problem);
        }
    }

    for prop in &contract.props {
        let path = format!("props.{}", prop.id);
        let default_ok = match prop.kind {
            PropKind::Text => prop.default.is_string(),
            PropKind::Bool => prop.default.is_boolean(),
            PropKind::Number => prop.default.as_f64().is_some_and(|n| {
                prop.min.is_none_or(|min| n >= min) && prop.max.is_none_or(|max| n <= max)
            }),
            PropKind::Enum => prop
                .default
                .as_str()
                .is_some_and(|value| prop.options.iter().any(|o| o == value)),
            PropKind::Tone => prop.default.as_str().is_some_and(is_tone),
        };
        if !default_ok {
            push(at(&path), "default does not fit the prop's kind".into());
        }
        if (prop.kind == PropKind::Enum) != !prop.options.is_empty() {
            push(
                at(&path),
                "options belong to enum props, and enums need them".into(),
            );
        }
    }

    for axis in &contract.variant_axes {
        let path = format!("variant_axes.{}", axis.id);
        let mut options = BTreeSet::new();
        for option in &axis.options {
            if !is_snake(&option.id) || !options.insert(option.id.as_str()) {
                push(
                    at(&format!("{path}.{}", option.id)),
                    "option ids are unique snake_case".into(),
                );
            }
        }
        if axis.options.len() < 2 {
            push(at(&path), "an axis offers at least two options".into());
        }
        if !options.contains(axis.default.as_str()) {
            push(at(&path), "default is not one of its options".into());
        }
    }

    let axis = |name: &str| contract.variant_axes.iter().find(|axis| axis.id == name);
    for (index, rule) in contract.forbidden.iter().enumerate() {
        let path = format!("forbidden.{index}");
        if rule.when.len() < 2 {
            push(
                at(&path),
                "a forbidden combination names two axes or more".into(),
            );
        }
        for (name, values) in &rule.when {
            match axis(name) {
                Some(axis) => {
                    for value in values {
                        if !axis.options.iter().any(|o| &o.id == value) {
                            push(at(&path), format!("`{name}` has no option `{value}`"));
                        }
                    }
                }
                None => push(at(&path), format!("no axis `{name}`")),
            }
        }
    }

    if contract.examples.is_empty() {
        push(
            at("examples"),
            "at least one example (the gallery demo)".into(),
        );
    }
    let mut example_ids = BTreeSet::new();
    for example in &contract.examples {
        let path = format!("examples.{}", example.id);
        if !example_ids.insert(example.id.as_str()) {
            push(at(&path), "example ids are unique".into());
        }
        for key in example.content.keys() {
            if !contract.slots.iter().any(|slot| &slot.id == key) {
                push(at(&path), format!("content `{key}` is not a slot"));
            }
        }
        for key in example.props.keys() {
            if !contract.props.iter().any(|prop| &prop.id == key) {
                push(at(&path), format!("prop `{key}` is not declared"));
            }
        }
        for (name, value) in &example.variant {
            match axis(name) {
                Some(axis) if axis.options.iter().any(|o| &o.id == value) => {}
                Some(_) => push(at(&path), format!("`{name}` has no option `{value}`")),
                None => push(at(&path), format!("no axis `{name}`")),
            }
        }
        for slot in &contract.slots {
            // Empty text or an empty list is as empty as a missing value.
            let value = example.content.get(&slot.id).filter(|v| {
                !(v.is_null()
                    || v.as_str().is_some_and(str::is_empty)
                    || v.as_array().is_some_and(Vec::is_empty))
            });
            match value {
                None if slot.required => {
                    push(at(&path), format!("required slot `{}` is empty", slot.id));
                }
                None => {}
                Some(value) => {
                    if let Some(problem) = slot_value_issue(slot, value) {
                        push(at(&format!("{path}.{}", slot.id)), problem);
                    }
                }
            }
        }
        let chosen = |name: &str| {
            example
                .variant
                .get(name)
                .cloned()
                .or_else(|| axis(name).map(|axis| axis.default.clone()))
        };
        for rule in &contract.forbidden {
            let hit = rule
                .when
                .iter()
                .all(|(name, values)| chosen(name).is_some_and(|choice| values.contains(&choice)));
            if hit {
                push(at(&path), "uses a forbidden axis combination".into());
            }
        }
    }
    issues
}

/// Item fields: unique snake_case ids, labels and tooltips, and a `list`
/// field (only) names its own item fields — recursively.
fn field_problems(fields: &[Field], path: &str) -> Vec<(String, String)> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for field in fields {
        let at = format!("{path}.{}", field.id);
        if !is_snake(&field.id) || !seen.insert(field.id.as_str()) {
            problems.push((at.clone(), "item field ids are unique snake_case".into()));
        }
        if field.label.trim().is_empty() || field.tooltip.trim().is_empty() {
            problems.push((at.clone(), "needs a label and a tooltip".into()));
        }
        match (field.kind == FieldKind::List, field.item.is_empty()) {
            (true, true) => {
                problems.push((at.clone(), "a list field names its item fields".into()));
            }
            (false, false) => {
                problems.push((at.clone(), "only a list field has item fields".into()));
            }
            _ => {}
        }
        problems.extend(field_problems(&field.item, &format!("{at}.item")));
    }
    problems
}

/// Does `value` fit an object of `fields`? Unknown keys and mistyped values
/// are named; absent keys are allowed (they take the template's default).
fn object_problem(fields: &[Field], value: &Value) -> Option<String> {
    let Value::Object(map) = value else {
        return Some("expected an object of its fields".into());
    };
    map.iter().find_map(|(key, item)| {
        let Some(field) = fields.iter().find(|field| &field.id == key) else {
            return Some(format!("field `{key}` is not declared"));
        };
        field_value_problem(field, item).map(|problem| format!("`{key}`: {problem}"))
    })
}

fn field_value_problem(field: &Field, value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let fits = match field.kind {
        FieldKind::Text => value.is_string(),
        FieldKind::Number => value.is_number(),
        FieldKind::Bool => value.is_boolean(),
        FieldKind::Tone => value.as_str().is_some_and(is_tone),
        FieldKind::Numbers => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_number)),
        FieldKind::List => {
            let Some(items) = value.as_array() else {
                return Some("expected a list".into());
            };
            return items
                .iter()
                .find_map(|item| list_item_problem(&field.item, item));
        }
    };
    (!fits).then(|| format!("expected {:?}", field.kind).to_lowercase())
}

/// One list item: an object of the fields, or — for a one-field item — the
/// bare value.
fn list_item_problem(fields: &[Field], item: &Value) -> Option<String> {
    match (item, fields) {
        (Value::Object(_), _) => object_problem(fields, item),
        (_, [only]) => field_value_problem(only, item),
        _ => Some("a list item is an object of its fields".into()),
    }
}

/// Does `value` fit `slot` (kind, limits, item fields all the way down)?
/// `None` means it fits.
pub fn slot_value_issue(slot: &Slot, value: &Value) -> Option<String> {
    match slot.entity {
        Entity::Text | Entity::Quote => {
            let Some(text) = value.as_str() else {
                return Some("a text slot holds text".into());
            };
            slot.limits
                .max_chars
                .filter(|max| text.chars().count() > *max)
                .map(|max| format!("longer than its {max} characters"))
        }
        Entity::Picture => {
            let kind = value.get("kind").and_then(Value::as_str);
            (!matches!(kind, Some("image" | "video" | "anim")))
                .then(|| "a picture slot holds {kind: image | video | anim}".into())
        }
        Entity::Data if slot.item.is_empty() => {
            let numeric = value.is_number()
                || value
                    .as_array()
                    .is_some_and(|items| items.iter().all(Value::is_number));
            (!numeric).then(|| {
                "a data slot without fields holds a number or a list of numbers; name its fields"
                    .into()
            })
        }
        Entity::Data => object_problem(&slot.item, value),
        Entity::List => {
            let Some(items) = value.as_array() else {
                return Some("a list slot holds a list".into());
            };
            if slot.limits.min_items.is_some_and(|min| items.len() < min)
                || slot.limits.max_items.is_some_and(|max| items.len() > max)
            {
                return Some(format!("{} items is outside its limits", items.len()));
            }
            items
                .iter()
                .find_map(|item| list_item_problem(&slot.item, item))
        }
    }
}

/// The contracts gate's rule (LS1): a later version of a contract keeps
/// everything the earlier one had. `before` is the copy on `main`.
///
/// - Same version → the contract must be unchanged.
/// - A higher version → every slot (same entity), prop (same kind), axis and
///   axis option survives, and nothing becomes required that was optional.
/// - A lower version is refused.
pub fn evolution_problems(before: &ComponentContract, after: &ComponentContract) -> Vec<String> {
    let mut problems = Vec::new();
    let (Some(old), Some(new)) = (semver(&before.version), semver(&after.version)) else {
        return vec!["a version is not x.y.z".into()];
    };
    if new < old {
        problems.push(format!(
            "version went down ({} → {})",
            before.version, after.version
        ));
        return problems;
    }
    if new == old {
        if before != after {
            problems.push(format!(
                "changed without raising its version ({})",
                after.version
            ));
        }
        return problems;
    }
    for slot in &before.slots {
        match after.slots.iter().find(|s| s.id == slot.id) {
            None => problems.push(format!("slot `{}` was removed", slot.id)),
            Some(now) if now.entity != slot.entity => {
                problems.push(format!("slot `{}` changed what it holds", slot.id));
            }
            Some(now) if now.required && !slot.required => {
                problems.push(format!("slot `{}` became required", slot.id));
            }
            Some(now) => fields_lost(&slot.item, &now.item, &slot.id, &mut problems),
        }
    }
    for prop in &before.props {
        match after.props.iter().find(|p| p.id == prop.id) {
            None => problems.push(format!("prop `{}` was removed", prop.id)),
            Some(now) if now.kind != prop.kind => {
                problems.push(format!("prop `{}` changed kind", prop.id));
            }
            Some(now) => {
                for option in &prop.options {
                    if !now.options.contains(option) {
                        problems.push(format!("prop `{}` lost option `{option}`", prop.id));
                    }
                }
            }
        }
    }
    for axis in &before.variant_axes {
        match after.variant_axes.iter().find(|a| a.id == axis.id) {
            None => problems.push(format!("axis `{}` was removed", axis.id)),
            Some(now) => {
                for option in &axis.options {
                    if !now.options.iter().any(|o| o.id == option.id) {
                        problems.push(format!("axis `{}` lost option `{}`", axis.id, option.id));
                    }
                }
            }
        }
    }
    problems
}

/// Every item field `before` had must survive in `after`, with its kind —
/// recursively through nested lists.
fn fields_lost(before: &[Field], after: &[Field], path: &str, problems: &mut Vec<String>) {
    for field in before {
        match after.iter().find(|f| f.id == field.id) {
            Some(now) if now.kind == field.kind => {
                fields_lost(
                    &field.item,
                    &now.item,
                    &format!("{path}.{}", field.id),
                    problems,
                );
            }
            _ => problems.push(format!(
                "slot `{path}` lost or changed item field `{}`",
                field.id
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ComponentContract {
        parse(
            r#"{
              "schema": "dpl.component/1", "id": "sample", "version": "1.0.0",
              "name": "Sample", "summary": "A test component.", "class": "static",
              "slots": [
                {"id": "title", "label": "Title", "entity": "text", "required": true,
                 "tooltip": "The big line", "limits": {"max_chars": 20}},
                {"id": "items", "label": "Items", "entity": "list", "required": false,
                 "tooltip": "The points", "limits": {"max_items": 2},
                 "item": [{"id": "label", "label": "Label", "kind": "text", "tooltip": "One point"}]}
              ],
              "props": [
                {"id": "tone", "label": "Colour", "kind": "tone", "default": "cyan", "tooltip": "The slide's colour"}
              ],
              "variant_axes": [
                {"id": "align", "label": "Align", "tooltip": "Where the text sits", "default": "left",
                 "options": [{"id": "left", "label": "Left"}, {"id": "center", "label": "Center"}]},
                {"id": "density", "label": "Density", "tooltip": "How tight", "default": "comfy",
                 "options": [{"id": "comfy", "label": "Comfy"}, {"id": "compact", "label": "Compact"}]}
              ],
              "forbidden": [{"when": {"align": ["center"], "density": ["compact"]}}],
              "examples": [{"id": "demo", "label": "Demo", "content": {"title": "Hello"}}],
              "provenance": {}
            }"#,
        )
        .expect("sample parses")
    }

    #[test]
    fn the_seed_class_folders_are_the_template_classes() {
        let labels = TemplateClass::ALL.map(TemplateClass::label);
        assert_eq!(labels, crate::SEED_CLASSES);
    }

    #[test]
    fn a_sound_contract_has_no_issues() {
        assert_eq!(validate(&sample()), Vec::new());
    }

    #[test]
    fn examples_are_checked_against_slots_limits_and_forbidden_combinations() {
        let mut contract = sample();
        contract.examples[0].content.insert(
            "title".into(),
            Value::String("far too long a title".repeat(3)),
        );
        contract.examples[0]
            .variant
            .insert("align".into(), "center".into());
        contract.examples[0]
            .variant
            .insert("density".into(), "compact".into());
        contract.examples[0]
            .content
            .insert("ghost".into(), Value::Bool(true));
        let problems: Vec<String> = validate(&contract).into_iter().map(|i| i.problem).collect();
        assert!(
            problems.iter().any(|p| p.contains("20 characters")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("forbidden")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("`ghost` is not a slot")),
            "{problems:?}"
        );
    }

    #[test]
    fn structured_values_are_checked_all_the_way_down() {
        let mut contract = sample();
        contract.slots.push(
            serde_json::from_value(serde_json::json!({
                "id": "rounds", "label": "Rounds", "entity": "list", "required": false,
                "tooltip": "Each round",
                "item": [
                    {"id": "prompt", "label": "Prompt", "kind": "text", "tooltip": "The question"},
                    {"id": "series", "label": "Series", "kind": "numbers", "tooltip": "The values"},
                    {"id": "guesses", "label": "Guesses", "kind": "list", "tooltip": "The options",
                     "item": [{"id": "label", "label": "Label", "kind": "text", "tooltip": "One"}]}
                ]
            }))
            .expect("slot"),
        );
        assert_eq!(validate(&contract), Vec::new());
        let problems = |contract: &ComponentContract| -> Vec<String> {
            validate(contract).into_iter().map(|i| i.problem).collect()
        };
        contract.examples[0].content.insert(
            "rounds".into(),
            serde_json::json!([{"prompt": "Q", "series": [1, "two"]}]),
        );
        let found = problems(&contract);
        assert!(found.iter().any(|p| p.contains("`series`")), "{found:?}");
        contract.examples[0].content.insert(
            "rounds".into(),
            serde_json::json!([{"prompt": "Q", "guesses": [{"lable": "x"}]}]),
        );
        let found = problems(&contract);
        assert!(
            found.iter().any(|p| p.contains("`lable` is not declared")),
            "{found:?}"
        );
    }

    #[test]
    fn ids_share_one_namespace_and_defaults_must_fit() {
        let mut contract = sample();
        contract.props[0].id = "align".into();
        contract.props[0].default = Value::String("chartreuse".into());
        let problems: Vec<String> = validate(&contract).into_iter().map(|i| i.problem).collect();
        assert!(
            problems.iter().any(|p| p.contains("used twice")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("does not fit")),
            "{problems:?}"
        );
    }

    #[test]
    fn example_values_fill_defaults_under_the_example() {
        let values = example_values(&sample(), 0).expect("example 0");
        assert_eq!(values["title"], "Hello");
        assert_eq!(values["tone"], "cyan");
        assert_eq!(values["align"], "left");
    }

    #[test]
    fn a_later_version_must_keep_everything_the_earlier_one_had() {
        let before = sample();
        let mut same_version = sample();
        same_version.summary = "Changed.".into();
        assert!(evolution_problems(&before, &same_version)
            .iter()
            .any(|p| p.contains("without raising")));

        let mut dropped = sample();
        dropped.version = "1.1.0".into();
        dropped.variant_axes[0].options.pop();
        dropped.slots.pop();
        let problems = evolution_problems(&before, &dropped);
        assert!(
            problems.iter().any(|p| p.contains("lost option `center`")),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("slot `items` was removed")),
            "{problems:?}"
        );

        let mut grown = sample();
        grown.version = "1.1.0".into();
        grown.variant_axes[0].options.push(AxisOption {
            id: "right".into(),
            label: "Right".into(),
        });
        assert_eq!(evolution_problems(&before, &grown), Vec::<String>::new());
    }
}
