//! A bounded, lossless projection of native media workflows into Open Workflow.
//!
//! This is an interchange experiment, not a general Open Workflow interpreter.
//! Custom calls describe existing native stages; this crate never executes them.
//! Only the exact profile emitted here is accepted. Unknown behavior, metadata,
//! versions and fields are refused, including fields native serde would ignore.

use crate::{Stage, StageKind, WorkflowDoc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

mod assessment;
mod document;

pub use assessment::{
    assess_procedure, FeatureAssessment, FeatureStatus, NativeWorkflowMapping, PilotMappingStatus,
    ProcedureAnnotations, ProcedureAssessment, ProcedureId,
};
pub use document::{parse_document, MAX_DOCUMENT_BYTES};

pub const DSL_VERSION: &str = "1.0.3";
pub const PROFILE_NAMESPACE: &str = "devpulse-native-workflows";
pub const PROFILE_VERSION: &str = "0.1.0";
pub const CALL_PREFIX: &str = "devpulse.";

/// A mapping refusal. Wire locations use JSON pointers; native validation
/// locations use the owning validator's path prefixed by `native.`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingIssue {
    pub path: String,
    pub reason: String,
}

pub type MappingResult<T> = Result<T, Vec<MappingIssue>>;

fn refusal<T>(path: &str, reason: impl Into<String>) -> MappingResult<T> {
    Err(vec![MappingIssue {
        path: path.into(),
        reason: reason.into(),
    }])
}

fn pointer(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn object<'a>(value: &'a Value, path: &str) -> MappingResult<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| {
        vec![MappingIssue {
            path: path.into(),
            reason: "expected an object".into(),
        }]
    })
}

fn field<'a>(map: &'a Map<String, Value>, key: &str, path: &str) -> MappingResult<&'a Value> {
    map.get(key).ok_or_else(|| {
        vec![MappingIssue {
            path: pointer(path, key),
            reason: "required field is missing".into(),
        }]
    })
}

fn string<'a>(value: &'a Value, path: &str) -> MappingResult<&'a str> {
    value.as_str().ok_or_else(|| {
        vec![MappingIssue {
            path: path.into(),
            reason: "expected a string".into(),
        }]
    })
}

/// Open Workflow 1.0.3 document name/namespace pattern, without silently
/// rewriting a native identity: 1..63 ASCII alphanumeric/hyphen characters,
/// with alphanumeric endpoints. Task keys have no corresponding schema bound.
pub(crate) fn wire_name(name: &str, path: &str) -> MappingResult<()> {
    let bytes = name.as_bytes();
    let valid = (1..=63).contains(&bytes.len())
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-');
    if !valid {
        return refusal(path, "Open Workflow document names require 1..63 ASCII alphanumeric/hyphen characters with alphanumeric endpoints; identity is not sanitized");
    }
    Ok(())
}

fn only_keys(map: &Map<String, Value>, keys: &[&str], path: &str) -> MappingResult<()> {
    let issues: Vec<_> = map
        .keys()
        .filter(|key| !keys.contains(&key.as_str()))
        .map(|key| MappingIssue {
            path: pointer(path, key),
            reason: "field is outside the bounded profile; refusing to discard it".into(),
        })
        .collect();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn exact(map: &Map<String, Value>, key: &str, expected: &str, path: &str) -> MappingResult<()> {
    let at = pointer(path, key);
    let actual = string(field(map, key, path)?, &at)?;
    if actual != expected {
        return refusal(
            &at,
            format!("unsupported value {actual:?}; expected {expected:?}"),
        );
    }
    Ok(())
}

fn serialize<T: Serialize>(value: &T, path: &str) -> MappingResult<Value> {
    serde_json::to_value(value).map_err(|error| {
        vec![MappingIssue {
            path: path.into(),
            reason: format!("cannot serialize native value: {error}"),
        }]
    })
}

fn validate_native(doc: &WorkflowDoc) -> MappingResult<()> {
    let mut issues: Vec<_> = crate::validate(doc)
        .into_iter()
        .map(|issue| MappingIssue {
            path: format!("native.{}", issue.path),
            reason: issue.message,
        })
        .collect();
    if doc.v != crate::SCHEMA_V {
        issues.push(MappingIssue {
            path: "native.v".into(),
            reason: format!(
                "unsupported native version {}; expected {}",
                doc.v,
                crate::SCHEMA_V
            ),
        });
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Export an already typed native workflow. Stage identities become task names;
/// native kind labels become custom calls and their fields become `with` args.
/// Native descriptive/intake fields are preserved in document metadata.
pub fn export_workflow(doc: &WorkflowDoc) -> MappingResult<Value> {
    validate_native(doc)?;
    wire_name(&doc.id, "/document/name")?;
    let native = serialize(doc, "native")?;
    let mut header = object(&native, "native")?.clone();
    header.remove("id");
    header.remove("stages");
    let mut tasks = Vec::with_capacity(doc.stages.len());
    for (index, stage) in doc.stages.iter().enumerate() {
        let kind = serialize(&stage.kind, "native.stages")?;
        let mut args = object(&kind, "native.stages")?.clone();
        args.remove("kind");
        let args = Value::Object(args);
        let task_path = pointer(&format!("/do/{index}"), &stage.id);
        reject_expressions(&args, &pointer(&task_path, "with"))?;
        let task = json!({"call": format!("{CALL_PREFIX}{}", stage.kind.label()), "with": args});
        let mut named = Map::new();
        named.insert(stage.id.clone(), task);
        tasks.push(Value::Object(named));
    }
    Ok(json!({
        "document": {
            "dsl": DSL_VERSION,
            "namespace": PROFILE_NAMESPACE,
            "name": doc.id,
            "version": PROFILE_VERSION,
            "metadata": {"devpulse": header}
        },
        "do": tasks
    }))
}

/// Import only this bounded media-stage profile. Valid Open Workflow features
/// outside it (retry, branching, expressions, arbitrary calls, task metadata,
/// etc.) are rejected. Strings containing `${` in call arguments are refused:
/// this profile has no literal escaping or expression-evaluation contract.
/// Success does not imply that an external engine can
/// execute the custom calls, nor that this crate implements a runner.
pub fn import_workflow(wire: &Value) -> MappingResult<WorkflowDoc> {
    let root = object(wire, "")?;
    only_keys(root, &["document", "do"], "")?;
    let document = object(field(root, "document", "")?, "/document")?;
    only_keys(
        document,
        &["dsl", "namespace", "name", "version", "metadata"],
        "/document",
    )?;
    exact(document, "dsl", DSL_VERSION, "/document")?;
    exact(document, "namespace", PROFILE_NAMESPACE, "/document")?;
    exact(document, "version", PROFILE_VERSION, "/document")?;
    let id = string(field(document, "name", "/document")?, "/document/name")?;
    let metadata = object(
        field(document, "metadata", "/document")?,
        "/document/metadata",
    )?;
    only_keys(metadata, &["devpulse"], "/document/metadata")?;
    let header_value = field(metadata, "devpulse", "/document/metadata")?;
    let mut native = object(header_value, "/document/metadata/devpulse")?.clone();
    for reserved in ["id", "stages"] {
        if native.contains_key(reserved) {
            return refusal(
                &pointer("/document/metadata/devpulse", reserved),
                "identity and stages must come from document.name and do",
            );
        }
    }
    let tasks = field(root, "do", "")?.as_array().ok_or_else(|| {
        vec![MappingIssue {
            path: "/do".into(),
            reason: "expected an array of named tasks".into(),
        }]
    })?;
    let mut stages = Vec::with_capacity(tasks.len());
    for (index, value) in tasks.iter().enumerate() {
        let path = format!("/do/{index}");
        let named = object(value, &path)?;
        if named.len() != 1 {
            return refusal(&path, "each task entry must contain exactly one task name");
        }
        let (name, task_value) = named.iter().next().expect("one task entry checked");
        let task_path = pointer(&path, name);
        let task = object(task_value, &task_path)?;
        only_keys(task, &["call", "with"], &task_path)?;
        let call_path = pointer(&task_path, "call");
        let call = string(field(task, "call", &task_path)?, &call_path)?;
        let Some(label) = call.strip_prefix(CALL_PREFIX) else {
            return refusal(&call_path, "only native profile custom calls are supported");
        };
        let args_path = pointer(&task_path, "with");
        let args = field(task, "with", &task_path)?;
        reject_expressions(args, &args_path)?;
        let mut kind_value = object(args, &args_path)?.clone();
        if kind_value.contains_key("kind") {
            return refusal(
                &pointer(&args_path, "kind"),
                "kind is defined by call, not with",
            );
        }
        kind_value.insert("kind".into(), Value::String(label.into()));
        let kind: StageKind =
            serde_json::from_value(Value::Object(kind_value)).map_err(|error| {
                vec![MappingIssue {
                    path: task_path.clone(),
                    reason: format!("unsupported or invalid native stage: {error}"),
                }]
            })?;
        if call != format!("{CALL_PREFIX}{}", kind.label()) {
            return refusal(
                &call_path,
                "call does not match the canonical native kind label",
            );
        }
        stages.push(Stage {
            id: name.clone(),
            kind,
        });
    }
    native.insert("id".into(), Value::String(id.into()));
    native.insert("stages".into(), serialize(&stages, "native.stages")?);
    let doc: WorkflowDoc = serde_json::from_value(Value::Object(native)).map_err(|error| {
        vec![MappingIssue {
            path: "/document/metadata/devpulse".into(),
            reason: format!("invalid native header: {error}"),
        }]
    })?;
    validate_native(&doc)?;
    wire_name(&doc.id, "/document/name")?;
    // Native serde deliberately tolerates unknown fields. Compare against a
    // fresh projection to detect every ignored/defaulted field before success.
    let canonical = export_workflow(&doc)?;
    let mut issues = Vec::new();
    differences(wire, &canonical, "", &mut issues);
    if issues.is_empty() {
        Ok(doc)
    } else {
        Err(issues)
    }
}

fn reject_expressions(value: &Value, path: &str) -> MappingResult<()> {
    match value {
        Value::String(text) if text.contains("${") => {
            refusal(path, "expression-looking arguments are unsupported; literal escaping and interpolation semantics are not mapped")
        }
        Value::Object(map) => {
            for (key, value) in map {
                reject_expressions(value, &pointer(path, key))?;
            }
            Ok(())
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                reject_expressions(value, &pointer(path, &index.to_string()))?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn differences(input: &Value, canonical: &Value, path: &str, issues: &mut Vec<MappingIssue>) {
    match (input, canonical) {
        (Value::Object(actual), Value::Object(expected)) => {
            for (key, value) in actual {
                let at = pointer(path, key);
                if let Some(other) = expected.get(key) {
                    differences(value, other, &at, issues);
                } else {
                    issues.push(MappingIssue {
                        path: at,
                        reason: "field would be lost by native deserialization".into(),
                    });
                }
            }
            for key in expected.keys().filter(|key| !actual.contains_key(*key)) {
                issues.push(MappingIssue {
                    path: pointer(path, key),
                    reason: "field is required by the canonical profile".into(),
                });
            }
        }
        (Value::Array(actual), Value::Array(expected)) if actual.len() == expected.len() => {
            for (index, (value, other)) in actual.iter().zip(expected).enumerate() {
                differences(value, other, &pointer(path, &index.to_string()), issues);
            }
        }
        _ if input == canonical => {}
        _ => issues.push(MappingIssue {
            path: path.into(),
            reason: "value differs from the canonical native projection".into(),
        }),
    }
}
