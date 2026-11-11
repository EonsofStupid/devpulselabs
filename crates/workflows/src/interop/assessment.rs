//! Static assessment of the three source-grounded UI procedure shapes.
//!
//! These custom calls are descriptive, not callable endpoints. No result from
//! this module authorizes execution or creates a native workflow. A recognized
//! procedure still reports every obligation the current APIs cannot preserve.

use super::{
    exact, field, object, only_keys, pointer, refusal, string, wire_name, MappingIssue,
    MappingResult, DSL_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

const META: &str = "/document/metadata/dplUiProcedure";
const HUMAN: &str = "human-automation-equivalence";
const DURABLE: &str = "durable-artifact-observation";
const VIEWPORT: &str = "viewport-scoped-targets";
const SELECTED: &str = "selected-state-observation";
const JOB: &str = "job-correlated-completion";
const REQUIRED_FEATURES: &[&str] = &[HUMAN, DURABLE, VIEWPORT, SELECTED, JOB];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeatureStatus {
    /// Existing primitives can carry the narrowly scoped requirement. This
    /// does not mean that this assessment ran or qualified those primitives.
    CapabilitySupported,
    Limited,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureAssessment {
    pub feature: String,
    pub status: FeatureStatus,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeWorkflowMapping {
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PilotMappingStatus {
    Partial,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcedureId(pub String);

/// Caller-supplied descriptions retained verbatim for review. These strings
/// are not predicates, executable instructions, or independently checked facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureAnnotations {
    pub sources: Vec<String>,
    pub preconditions: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureAssessment {
    pub id: ProcedureId,
    pub mapping_only: bool,
    pub annotations: ProcedureAnnotations,
    pub preconditions_verified: bool,
    pub required_features: Vec<FeatureAssessment>,
    pub native_workflow: NativeWorkflowMapping,
    pub pilot_mapping: PilotMappingStatus,
    pub diagnostics: Vec<MappingIssue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target<'a> {
    id: &'a str,
    viewport: &'a str,
}

#[derive(Debug)]
enum Task<'a> {
    Fill { text: &'a str },
    SaveLook,
    ObserveLook { value: &'a str },
    SelectProvider { target: Target<'a> },
    ObserveSelected { target: Target<'a> },
    StartExport { task_name: &'a str },
    ObserveExport { producer: &'a str },
}

#[derive(Clone, Copy)]
enum ProcedureKind {
    EditSave,
    WindowSetting,
    Export,
}

/// Inspect a bounded Open Workflow UI mapping without executing anything.
///
/// Unknown behavior, missing outcome obligations and mismatched scopes are
/// refusals. Recognized but unrepresentable obligations are explicit report
/// entries, never silently dropped or called successful native mappings.
pub fn assess_procedure(wire: &Value) -> MappingResult<ProcedureAssessment> {
    let root = object(wire, "")?;
    only_keys(root, &["document", "do"], "")?;
    let document = object(field(root, "document", "")?, "/document")?;
    only_keys(
        document,
        &["dsl", "namespace", "name", "version", "metadata"],
        "/document",
    )?;
    exact(document, "dsl", DSL_VERSION, "/document")?;
    exact(document, "namespace", "dpl-ui-mapping", "/document")?;
    exact(document, "version", "0.1.0", "/document")?;
    let id = literal(field(document, "name", "/document")?, "/document/name")?;
    wire_name(id, "/document/name")?;
    let metadata = object(
        field(document, "metadata", "/document")?,
        "/document/metadata",
    )?;
    only_keys(metadata, &["dplUiProcedure"], "/document/metadata")?;
    let profile = object(
        field(metadata, "dplUiProcedure", "/document/metadata")?,
        META,
    )?;
    only_keys(
        profile,
        &[
            "profile",
            "mappingOnly",
            "required",
            "sources",
            "preconditions",
            "limitations",
        ],
        META,
    )?;
    exact(profile, "profile", "local-mapping-0.1", META)?;
    if field(profile, "mappingOnly", META)? != &Value::Bool(true) {
        return refusal(
            &pointer(META, "mappingOnly"),
            "this profile is mapping-only; execution is unsupported",
        );
    }
    let annotations = ProcedureAnnotations {
        sources: annotation_strings(profile, "sources")?,
        preconditions: annotation_strings(profile, "preconditions")?,
        limitations: annotation_strings(profile, "limitations")?,
    };
    let required = string_array(
        field(profile, "required", META)?,
        &pointer(META, "required"),
    )?;
    let mut seen_features = BTreeSet::new();
    for (index, feature) in required.iter().enumerate() {
        if !REQUIRED_FEATURES.contains(feature) {
            return refusal(
                &format!("{META}/required/{index}"),
                format!("unknown required feature {feature:?}"),
            );
        }
        if !seen_features.insert(*feature) {
            return refusal(
                &format!("{META}/required/{index}"),
                "duplicate required feature",
            );
        }
    }

    let task_values = field(root, "do", "")?.as_array().ok_or_else(|| {
        vec![MappingIssue {
            path: "/do".into(),
            reason: "expected an array of named tasks".into(),
        }]
    })?;
    if task_values.is_empty() || task_values.len() > 16 {
        return refusal(
            "/do",
            "the bounded mapping requires between one and sixteen tasks",
        );
    }
    let mut names = BTreeSet::new();
    let mut tasks = Vec::new();
    for (index, value) in task_values.iter().enumerate() {
        let path = format!("/do/{index}");
        let named = object(value, &path)?;
        if named.len() != 1 {
            return refusal(&path, "each entry must contain exactly one named task");
        }
        let (name, task) = named.iter().next().expect("one task entry checked");
        if name.trim().is_empty() || !names.insert(name) {
            return refusal(&path, "task names must be nonempty and unique");
        }
        tasks.push(parse_task(task, &pointer(&path, name), name)?);
    }

    let kind = match tasks.as_slice() {
        [Task::Fill { text }, Task::SaveLook, Task::ObserveLook { value }] => {
            if *value != format!("Look: {text}") {
                return refusal("/do", "the observed look label must refer to the name supplied before Save");
            }
            ProcedureKind::EditSave
        }
        [Task::SelectProvider { target }, Task::ObserveSelected { target: observed }] => {
            if target != observed {
                return refusal("/do", "selection and completion must refer to the same node and viewport");
            }
            ProcedureKind::WindowSetting
        }
        [Task::StartExport { task_name }, Task::ObserveExport { producer }] => {
            if task_name != producer {
                return refusal("/do", "export completion must correlate to the preceding export task, not another producer");
            }
            ProcedureKind::Export
        }
        _ => return refusal("/do", "unsupported sequence or missing outcome: require fill/save/observe-look, select/observe-selected, or start-export/observe-correlated-completion in order"),
    };
    let expected: &[&str] = match kind {
        ProcedureKind::EditSave => &[DURABLE, HUMAN],
        ProcedureKind::WindowSetting => &[VIEWPORT, SELECTED, HUMAN],
        ProcedureKind::Export => &[JOB, HUMAN],
    };
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    if expected != seen_features {
        return refusal(&pointer(META, "required"), "required features must match all semantic obligations of the task sequence; missing or unrelated obligations cannot be discarded");
    }

    let required_features: Vec<_> = required.into_iter().map(assess_feature).collect();
    let mut diagnostics = vec![MappingIssue {
        path: "/do".into(),
        reason: "dpl_workflows::StageKind has media stages, not UI interaction stages; no native WorkflowDoc lowering or production execution is supplied".into(),
    }];
    for (index, feature) in required_features.iter().enumerate() {
        if feature.status != FeatureStatus::CapabilitySupported {
            diagnostics.push(MappingIssue {
                path: format!("{META}/required/{index}"),
                reason: feature.reason.clone(),
            });
        }
    }
    diagnostics.push(MappingIssue {
        path: "/do".into(),
        reason: match kind {
            ProcedureKind::EditSave => "The current naming field does not publish its value. Flow can Fill, Click and assert the chip's Text substring, but that is not an exact persisted-document verification.",
            ProcedureKind::WindowSetting => "Flow can scope Click to chat.primary. Selected state needs a same-viewport CoachCondition::NodeFlag and a Satisfied expectation; there is no direct selected-state Expectation or child-window screenshot proof.",
            ProcedureKind::Export => "Flow can click composer.guide/export, but cannot bind the returned JobId or await its terminal outcome. jobs.done and a finished job's Clear button cannot establish this export's success.",
        }.into(),
    });
    Ok(ProcedureAssessment {
        id: ProcedureId(id.into()),
        mapping_only: true,
        annotations,
        preconditions_verified: false,
        required_features,
        native_workflow: NativeWorkflowMapping::Unsupported,
        pilot_mapping: PilotMappingStatus::Partial,
        diagnostics,
    })
}

fn literal<'a>(value: &'a Value, path: &str) -> MappingResult<&'a str> {
    let text = string(value, path)?;
    if text.trim().is_empty() || text.contains("${") {
        return refusal(
            path,
            "expected a nonempty literal; expressions are outside this mapping profile",
        );
    }
    Ok(text)
}

fn string_array<'a>(value: &'a Value, path: &str) -> MappingResult<Vec<&'a str>> {
    let values = value.as_array().ok_or_else(|| {
        vec![MappingIssue {
            path: path.into(),
            reason: "expected an array of strings".into(),
        }]
    })?;
    values
        .iter()
        .enumerate()
        .map(|(index, value)| string(value, &format!("{path}/{index}")))
        .collect()
}

fn annotation_strings(profile: &Map<String, Value>, key: &str) -> MappingResult<Vec<String>> {
    match profile.get(key) {
        Some(value) => Ok(string_array(value, &pointer(META, key))?
            .into_iter()
            .map(str::to_owned)
            .collect()),
        None => Ok(Vec::new()),
    }
}

fn target<'a>(args: &'a Map<String, Value>, path: &str) -> MappingResult<Target<'a>> {
    let at = pointer(path, "target");
    let target = object(field(args, "target", path)?, &at)?;
    only_keys(target, &["id", "viewport"], &at)?;
    let id = literal(field(target, "id", &at)?, &pointer(&at, "id"))?;
    let Some((owner, _)) = id.split_once('/') else {
        return refusal(&at, "target ID must include its viewport key");
    };
    let viewport = match target.get("viewport") {
        Some(value) => literal(value, &pointer(&at, "viewport"))?,
        None if owner == "root" => "root",
        None => {
            return refusal(
                &at,
                "secondary-window targets require an explicit viewport key",
            )
        }
    };
    if viewport != owner {
        return refusal(&at, "target ID and viewport scope disagree");
    }
    Ok(Target { id, viewport })
}

fn expect_target(target: Target<'_>, expected: &str, path: &str) -> MappingResult<()> {
    if target.id != expected {
        return refusal(
            &pointer(path, "target"),
            "target is not source-grounded for this call in the bounded profile",
        );
    }
    Ok(())
}

fn parse_task<'a>(value: &'a Value, path: &str, task_name: &'a str) -> MappingResult<Task<'a>> {
    let task = object(value, path)?;
    only_keys(task, &["call", "with"], path)?;
    let call = literal(field(task, "call", path)?, &pointer(path, "call"))?;
    let at = pointer(path, "with");
    let args = object(field(task, "with", path)?, &at)?;
    match call {
        "dpl.ui.fill" => {
            only_keys(args, &["target", "text"], &at)?;
            expect_target(target(args, &at)?, "root/editor/look-name", &at)?;
            let text = literal(field(args, "text", &at)?, &pointer(&at, "text"))?;
            if text != text.trim() {
                return refusal(&pointer(&at, "text"), "look names are trimmed by the editor; this profile requires the canonical name");
            }
            Ok(Task::Fill { text })
        }
        "dpl.ui.click" => {
            only_keys(args, &["target"], &at)?;
            let target = target(args, &at)?;
            match target.id {
                "root/editor/look-save" => Ok(Task::SaveLook),
                "chat.primary/chat.sidebar.providers/claude" => Ok(Task::SelectProvider { target }),
                "root/composer.guide/export" => Ok(Task::StartExport { task_name }),
                _ => refusal(
                    &pointer(&at, "target"),
                    "click target is outside the source-grounded mapping experiment",
                ),
            }
        }
        "dpl.ui.expect-text" => {
            only_keys(args, &["target", "value", "match"], &at)?;
            expect_target(target(args, &at)?, "root/editor/look-chip", &at)?;
            exact(args, "match", "substring", &at)?;
            Ok(Task::ObserveLook {
                value: literal(field(args, "value", &at)?, &pointer(&at, "value"))?,
            })
        }
        "dpl.ui.expect-selected" => {
            only_keys(args, &["target", "selected"], &at)?;
            let target = target(args, &at)?;
            expect_target(target, "chat.primary/chat.sidebar.providers/claude", &at)?;
            if field(args, "selected", &at)? != &Value::Bool(true) {
                return refusal(
                    &pointer(&at, "selected"),
                    "completion requires the clicked provider to be selected",
                );
            }
            Ok(Task::ObserveSelected { target })
        }
        "dpl.jobs.expect-completed" => {
            only_keys(args, &["correlation", "terminal"], &at)?;
            let correlation = literal(
                field(args, "correlation", &at)?,
                &pointer(&at, "correlation"),
            )?;
            let Some(producer) = correlation
                .strip_prefix("job-id-returned-by-")
                .filter(|name| !name.is_empty())
            else {
                return refusal(&pointer(&at, "correlation"), "a correlation must name the export task whose JobId is required; generic jobs.done cannot prove completion");
            };
            exact(args, "terminal", "done", &at)?;
            Ok(Task::ObserveExport { producer })
        }
        _ => refusal(
            &pointer(path, "call"),
            format!("unsupported call {call:?}; no behavior is executed or ignored"),
        ),
    }
}

fn assess_feature(feature: &str) -> FeatureAssessment {
    let (status, reason) = match feature {
        VIEWPORT => (FeatureStatus::CapabilitySupported, "Existing Locator/ViewportSelector and viewport-scoped actions can carry these explicit chat.primary targets; this is a static mapping assessment, not live qualification."),
        SELECTED => (FeatureStatus::Limited, "Selected state is published and expressible through same-viewport CoachCondition::NodeFlag, not a direct Flow Expectation; general cross-window coach conditions remain unsound."),
        DURABLE => (FeatureStatus::Unsupported, "A look-chip Text substring is UI evidence, not a durable-artifact oracle. Flow has no persisted-document expectation; independent load_look verification is required."),
        JOB => (FeatureStatus::Unsupported, "Composer emits JobId-correlated Done/Failed facts, but Flow cannot capture that identity or await the corresponding fact. Generic jobs.done loses identity and cannot prove export success."),
        HUMAN => (FeatureStatus::Unsupported, "Coach touchpoints and agent input exist, but the shared resumable human-follow/automation outcome contract is not implemented or qualified."),
        _ => unreachable!("required features checked before assessment"),
    };
    FeatureAssessment {
        feature: feature.into(),
        status,
        reason: reason.into(),
    }
}
