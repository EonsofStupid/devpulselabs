//! PRESET CONTROLS — what an operator may change in a Lottie, without
//! After Effects and without re-authoring the animation.
//!
//! This is the same idea as a `.mogrt`'s `clientControls`, and the reason it
//! can actually work here: a Lottie is readable JSON, whereas a mogrt's
//! animation lives in `project.aegraphic`, an opaque Adobe payload the
//! reader in `composer/src/mogrt.rs` deliberately never parses. We can
//! retext and recolour a Lottie because we can see inside it.
//!
//! Two pure functions, both testable without spawning anything:
//!
//! - [`discover`] walks a document and reports what is editable, so a Lottie
//!   from ANY source gets controls without being annotated first;
//! - [`apply`] patches a document by JSON pointer, returning a new value and
//!   never mutating the operator's file (sources are never mutated).
//!
//! The render path is unchanged: patch, write to scratch, hand the scratch
//! path to [`crate::Player`] or [`crate::export_sequence`]. Preview and
//! export therefore consume the IDENTICAL patched document — what the
//! operator scrubs is what they get.

use serde_json::Value;

/// What kind of editor a control deserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    /// An RGBA swatch. Lottie stores colour as 0.0–1.0 floats; the app speaks
    /// 0–255 like `dpl_comp::Layer::tint`, and [`apply`] converts.
    Color,
    /// A string of display text.
    Text,
}

/// One editable thing, discovered or authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    /// Stable key the UI and a values map share. Derived from the pointer, so
    /// re-running [`discover`] on the same document yields the same ids.
    pub id: String,
    /// What the operator reads. The layer's own name when it has one — an
    /// author who named a layer "Title" gets "Title" for free.
    pub label: String,
    pub kind: ControlKind,
    /// RFC-6901 pointer to the value inside the Bodymovin document.
    pub pointer: String,
}

/// A value to write into a control.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlValue {
    /// RGBA 0–255 — an array, never a hex string (the repo's shape law).
    Color([u8; 4]),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    /// A value named a control the document does not have. REFUSED rather
    /// than skipped: a silently-ignored control is a preset that looks edited
    /// and renders unchanged.
    UnknownControl(String),
    /// The pointer resolved to nothing — a document that changed shape under
    /// a stale control set.
    DeadPointer(String),
    /// The value kind does not match the control's kind.
    WrongKind(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownControl(id) => write!(f, "no control named {id:?} in this Lottie"),
            Self::DeadPointer(p) => write!(f, "control target {p:?} is not in this Lottie"),
            Self::WrongKind(id) => write!(f, "control {id:?} was given the wrong kind of value"),
        }
    }
}

impl std::error::Error for ControlError {}

/// Escape one path segment for RFC-6901 (`~` → `~0`, `/` → `~1`).
fn esc(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

/// Resolve an RFC-6901 pointer to a mutable slot.
fn at_mut<'v>(doc: &'v mut Value, pointer: &str) -> Option<&'v mut Value> {
    doc.pointer_mut(pointer)
}

/// Walk a document and report every control it offers.
///
/// Finds text layers (`ty: 5`) — their string and their fill colour — and
/// shape fills (`ty: "fl"`) anywhere in the shape tree, including nested
/// groups. Order is document order, so the control list reads the way the
/// animation is built.
pub fn discover(doc: &Value) -> Vec<Control> {
    let mut out = Vec::new();
    let Some(layers) = doc.get("layers").and_then(Value::as_array) else {
        return out;
    };
    for (i, layer) in layers.iter().enumerate() {
        let name = layer
            .get("nm")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Layer {}", i + 1));
        let base = format!("/layers/{i}");

        // A text layer: `ty: 5`, with the document's first keyframe carrying
        // the string (`t`) and the fill colour (`fc`).
        if layer.get("ty").and_then(Value::as_u64) == Some(5) {
            let doc_ptr = format!("{base}/t/d/k/0/s");
            if doc.pointer(&format!("{doc_ptr}/t")).is_some() {
                out.push(Control {
                    id: format!("{name}.text"),
                    label: name.clone(),
                    kind: ControlKind::Text,
                    pointer: format!("{doc_ptr}/t"),
                });
            }
            if doc.pointer(&format!("{doc_ptr}/fc")).is_some() {
                out.push(Control {
                    id: format!("{name}.color"),
                    label: format!("{name} colour"),
                    kind: ControlKind::Color,
                    pointer: format!("{doc_ptr}/fc"),
                });
            }
        }

        if let Some(shapes) = layer.get("shapes").and_then(Value::as_array) {
            walk_shapes(shapes, &format!("{base}/shapes"), &name, &mut out);
        }
    }
    out
}

/// Recurse the shape tree collecting fills. Groups (`ty: "gr"`) nest their
/// children under `it`, which is why this cannot be a flat scan.
fn walk_shapes(shapes: &[Value], base: &str, layer_name: &str, out: &mut Vec<Control>) {
    for (i, shape) in shapes.iter().enumerate() {
        let here = format!("{base}/{i}");
        match shape.get("ty").and_then(Value::as_str) {
            Some("gr") => {
                if let Some(items) = shape.get("it").and_then(Value::as_array) {
                    walk_shapes(items, &format!("{here}/it"), layer_name, out);
                }
            }
            Some("fl") => {
                // A static fill stores its colour at `c.k`; an ANIMATED fill
                // stores keyframes there instead, and recolouring one is a
                // different editing act — skipped rather than half-applied.
                if shape
                    .get("c")
                    .and_then(|c| c.get("a"))
                    .and_then(Value::as_u64)
                    == Some(1)
                {
                    continue;
                }
                if shape
                    .get("c")
                    .and_then(|c| c.get("k"))
                    .and_then(Value::as_array)
                    .is_some()
                {
                    let nm = shape
                        .get("nm")
                        .and_then(Value::as_str)
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or("Fill");
                    out.push(Control {
                        id: format!("{}.{}.{}", esc(layer_name), esc(nm), out.len()),
                        label: format!("{layer_name} — {nm}"),
                        kind: ControlKind::Color,
                        pointer: format!("{here}/c/k"),
                    });
                }
            }
            _ => {}
        }
    }
}

/// Patch a document with operator values, returning a NEW document.
///
/// The source is never mutated — the caller writes the result to scratch and
/// renders that, so the operator's Lottie on disk is untouched.
///
/// An unknown control id is an ERROR, not a skip: a preset that looks edited
/// and renders unchanged is the worst possible outcome of this feature.
pub fn apply(
    doc: &Value,
    controls: &[Control],
    values: &[(String, ControlValue)],
) -> Result<Value, ControlError> {
    let mut patched = doc.clone();
    for (id, value) in values {
        let control = controls
            .iter()
            .find(|c| &c.id == id)
            .ok_or_else(|| ControlError::UnknownControl(id.clone()))?;
        let slot = at_mut(&mut patched, &control.pointer)
            .ok_or_else(|| ControlError::DeadPointer(control.pointer.clone()))?;
        match (control.kind, value) {
            (ControlKind::Text, ControlValue::Text(text)) => {
                *slot = Value::String(text.clone());
            }
            (ControlKind::Color, ControlValue::Color(rgba)) => {
                // Preserve the array's LENGTH: some fills carry [r,g,b] and
                // some [r,g,b,a]; growing one to four elements has been known
                // to upset stricter players.
                let keep_alpha = slot.as_array().is_some_and(|a| a.len() >= 4);
                let f = |c: u8| Value::from(f64::from(c) / 255.0);
                let mut out = vec![f(rgba[0]), f(rgba[1]), f(rgba[2])];
                if keep_alpha {
                    out.push(f(rgba[3]));
                }
                *slot = Value::Array(out);
            }
            _ => return Err(ControlError::WrongKind(id.clone())),
        }
    }
    Ok(patched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        json!({
            "v": "5.7.4", "fr": 30, "ip": 0, "op": 60, "w": 512, "h": 512,
            "layers": [
                {
                    "ty": 5, "nm": "Title",
                    "t": { "d": { "k": [ { "s": { "t": "HELLO", "fc": [0.0, 1.0, 1.0] }, "t": 0 } ] } }
                },
                {
                    "ty": 4, "nm": "Badge",
                    "shapes": [
                        { "ty": "gr", "nm": "group", "it": [
                            { "ty": "rc" },
                            { "ty": "fl", "nm": "Base", "c": { "a": 0, "k": [0.1, 0.2, 0.3, 1.0] } }
                        ]}
                    ]
                }
            ]
        })
    }

    #[test]
    fn discover_finds_a_text_layer_and_a_nested_shape_fill() {
        let found = discover(&fixture());
        let kinds: Vec<_> = found.iter().map(|c| (c.kind, c.pointer.as_str())).collect();
        assert!(kinds.contains(&(ControlKind::Text, "/layers/0/t/d/k/0/s/t")));
        assert!(kinds.contains(&(ControlKind::Color, "/layers/0/t/d/k/0/s/fc")));
        // The fill is INSIDE a group — a flat scan would miss it.
        assert!(kinds.contains(&(ControlKind::Color, "/layers/1/shapes/0/it/1/c/k")));
    }

    #[test]
    fn a_layers_own_name_becomes_the_label() {
        let found = discover(&fixture());
        assert!(found.iter().any(|c| c.label == "Title"));
        assert!(found.iter().any(|c| c.label.starts_with("Badge")));
    }

    #[test]
    fn apply_patches_only_the_named_pointer() {
        let doc = fixture();
        let controls = discover(&doc);
        let text = controls
            .iter()
            .find(|c| c.kind == ControlKind::Text)
            .unwrap()
            .clone();
        let out = apply(
            &doc,
            &controls,
            &[(text.id.clone(), ControlValue::Text("WIRED".into()))],
        )
        .expect("applied");
        assert_eq!(out.pointer("/layers/0/t/d/k/0/s/t").unwrap(), "WIRED");
        // Everything else is untouched.
        assert_eq!(
            out.pointer("/layers/1/shapes/0/it/1/c/k").unwrap(),
            doc.pointer("/layers/1/shapes/0/it/1/c/k").unwrap()
        );
        // And the SOURCE is untouched — sources are never mutated.
        assert_eq!(doc.pointer("/layers/0/t/d/k/0/s/t").unwrap(), "HELLO");
    }

    #[test]
    fn colour_is_written_as_lottie_floats_and_keeps_its_arity() {
        let doc = fixture();
        let controls = discover(&doc);
        // The shape fill carries FOUR components; the text colour carries three.
        let fill = controls
            .iter()
            .find(|c| c.pointer.ends_with("/c/k"))
            .unwrap()
            .clone();
        let text_col = controls
            .iter()
            .find(|c| c.pointer.ends_with("/fc"))
            .unwrap()
            .clone();
        let out = apply(
            &doc,
            &controls,
            &[
                (fill.id, ControlValue::Color([255, 0, 128, 255])),
                (text_col.id, ControlValue::Color([0, 255, 0, 255])),
            ],
        )
        .expect("applied");
        let f = out.pointer("/layers/1/shapes/0/it/1/c/k").unwrap();
        assert_eq!(f.as_array().unwrap().len(), 4, "4-component fill stays 4");
        assert_eq!(f[0].as_f64().unwrap(), 1.0);
        assert_eq!(f[1].as_f64().unwrap(), 0.0);
        let t = out.pointer("/layers/0/t/d/k/0/s/fc").unwrap();
        assert_eq!(t.as_array().unwrap().len(), 3, "3-component colour stays 3");
        assert_eq!(t[1].as_f64().unwrap(), 1.0);
    }

    /// A preset that looks edited and renders unchanged is the worst outcome
    /// this feature can produce, so an unknown id REFUSES.
    #[test]
    fn an_unknown_control_is_refused_not_ignored() {
        let doc = fixture();
        let controls = discover(&doc);
        let err = apply(
            &doc,
            &controls,
            &[("nope.text".into(), ControlValue::Text("x".into()))],
        )
        .unwrap_err();
        assert_eq!(err, ControlError::UnknownControl("nope.text".into()));
    }

    #[test]
    fn a_kind_mismatch_is_refused() {
        let doc = fixture();
        let controls = discover(&doc);
        let text = controls
            .iter()
            .find(|c| c.kind == ControlKind::Text)
            .unwrap()
            .clone();
        assert!(matches!(
            apply(
                &doc,
                &controls,
                &[(text.id, ControlValue::Color([1, 2, 3, 4]))]
            ),
            Err(ControlError::WrongKind(_))
        ));
    }

    #[test]
    fn applying_nothing_returns_the_document_unchanged() {
        let doc = fixture();
        let controls = discover(&doc);
        assert_eq!(apply(&doc, &controls, &[]).unwrap(), doc);
    }

    /// An ANIMATED fill is a different editing act — a single swatch cannot
    /// express a colour that changes over time, so it is not offered.
    #[test]
    fn an_animated_fill_is_not_offered_as_a_swatch() {
        let doc = json!({
            "layers": [ { "ty": 4, "nm": "Pulse", "shapes": [
                { "ty": "fl", "nm": "Animated", "c": { "a": 1, "k": [
                    { "t": 0, "s": [1.0, 0.0, 0.0, 1.0] }
                ]}}
            ]}]
        });
        assert!(discover(&doc).is_empty());
    }

    #[test]
    fn a_document_without_layers_yields_no_controls() {
        assert!(discover(&json!({ "v": "5.7.4" })).is_empty());
    }
}
