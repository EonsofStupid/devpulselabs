//! Upstream shape checks, not runtime or full standards conformance tests.

use dpl_workflows::interop::export_workflow;
use serde_json::Value;
use std::collections::BTreeSet;

fn parse(source: &str) -> Value {
    serde_json::from_str(source).expect("checked-in JSON must parse")
}

fn workflow_schema() -> jsonschema::Validator {
    jsonschema::validator_for(&parse(include_str!(
        "../interop/schemas/open-workflow-1.0.3.derived.json"
    )))
    .expect("pinned workflow schema compiles offline")
}

fn arazzo_schema() -> jsonschema::Validator {
    jsonschema::validator_for(&parse(include_str!(
        "../interop/schemas/arazzo-1.1-2026-04-15.upstream.json"
    )))
    .expect("pinned Arazzo schema compiles offline")
}

fn assert_valid(validator: &jsonschema::Validator, value: &Value, label: &str) {
    let issues: Vec<String> = validator
        .iter_errors(value)
        .map(|issue| issue.to_string())
        .collect();
    assert!(issues.is_empty(), "{label}: {}", issues.join("; "));
}

#[test]
fn ui_procedure_documents_match_pinned_workflow_shape() {
    let validator = workflow_schema();
    for (name, source) in [
        (
            "edit-save",
            include_str!("../interop/fixtures/open-workflow/edit-save.json"),
        ),
        (
            "window-setting",
            include_str!("../interop/fixtures/open-workflow/window-setting.json"),
        ),
        (
            "export-observe",
            include_str!("../interop/fixtures/open-workflow/export-observe.json"),
        ),
    ] {
        assert_valid(&validator, &parse(source), name);
    }
}

#[test]
fn every_real_registered_native_workflow_exports_to_valid_standard_shape() {
    let validator = workflow_schema();
    let workflows = dpl_workflows::workflows();
    assert!(
        !workflows.is_empty(),
        "positive control must use the real registry"
    );
    for workflow in workflows {
        let projected = export_workflow(&workflow).expect("registered native workflow exports");
        assert_valid(&validator, &projected, &workflow.id);
    }
}

#[test]
fn arazzo_comparison_matches_pinned_schema_and_local_operation_references() {
    let document = parse(include_str!(
        "../interop/fixtures/arazzo/ui-procedures.json"
    ));
    assert_valid(&arazzo_schema(), &document, "Arazzo comparison");
    assert_eq!(
        document["sourceDescriptions"][0]["url"],
        "./fixture-api.openapi.json"
    );
    let api = parse(include_str!(
        "../interop/fixtures/arazzo/fixture-api.openapi.json"
    ));
    let mut operations = BTreeSet::new();
    for path in api["paths"].as_object().expect("paths").values() {
        for operation in path.as_object().expect("path item").values() {
            if let Some(id) = operation.get("operationId").and_then(Value::as_str) {
                assert!(operations.insert(id), "duplicate operationId {id}");
            }
        }
    }
    for workflow in document["workflows"].as_array().expect("workflows") {
        for step in workflow["steps"].as_array().expect("steps") {
            let operation = step["operationId"]
                .as_str()
                .expect("fixture uses operationId");
            assert!(
                operations.contains(operation),
                "unresolved operation {operation}"
            );
        }
    }
}

#[test]
fn upstream_schemas_reject_missing_required_structure() {
    let mut workflow = parse(include_str!(
        "../interop/fixtures/open-workflow/edit-save.json"
    ));
    workflow["document"]
        .as_object_mut()
        .unwrap()
        .remove("namespace");
    assert!(!workflow_schema().is_valid(&workflow));

    let mut arazzo = parse(include_str!(
        "../interop/fixtures/arazzo/ui-procedures.json"
    ));
    arazzo["workflows"][0]["steps"][0]
        .as_object_mut()
        .unwrap()
        .remove("operationId");
    assert!(!arazzo_schema().is_valid(&arazzo));
}
