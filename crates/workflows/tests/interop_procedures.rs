use dpl_workflows::interop::assessment::{
    assess_procedure, FeatureStatus, NativeWorkflowMapping, PilotMappingStatus,
};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("interop/fixtures/open-workflow")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn has_unsupported(wire: &Value, feature: &str) {
    let report = assess_procedure(wire).unwrap();
    assert_eq!(report.native_workflow, NativeWorkflowMapping::Unsupported);
    assert_eq!(report.pilot_mapping, PilotMappingStatus::Partial);
    assert!(report
        .required_features
        .iter()
        .any(|item| { item.feature == feature && item.status == FeatureStatus::Unsupported }));
}

#[test]
fn ui_text_never_becomes_durable_document_proof() {
    let wire = fixture("edit-save");
    has_unsupported(&wire, "durable-artifact-observation");
    has_unsupported(&wire, "human-automation-equivalence");
    let mut different_name = wire.clone();
    different_name["do"][0]["name-look"]["with"]["text"] = json!("Another look");
    assert!(
        assess_procedure(&different_name).is_err(),
        "old UI text cannot prove a different save"
    );
    different_name["do"][2]["observe-chip"]["with"]["value"] = json!("Look: Another look");
    has_unsupported(&different_name, "durable-artifact-observation");
}

#[test]
fn window_mapping_checks_actual_scope_and_selected_state() {
    let wire = fixture("window-setting");
    let report = assess_procedure(&wire).unwrap();
    assert!(report.required_features.iter().any(|item| {
        item.feature == "viewport-scoped-targets"
            && item.status == FeatureStatus::CapabilitySupported
    }));
    assert!(report.required_features.iter().any(|item| {
        item.feature == "selected-state-observation" && item.status == FeatureStatus::Limited
    }));
    let mut wrong = wire.clone();
    wrong["do"][1]["observe-selected"]["with"]["target"]["viewport"] = json!("root");
    assert!(assess_procedure(&wrong).is_err());
    let mut absent = wire.clone();
    absent["do"][1]["observe-selected"]["with"]["target"]
        .as_object_mut()
        .unwrap()
        .remove("viewport");
    assert!(assess_procedure(&absent).is_err());
    let mut opposite = wire;
    opposite["do"][1]["observe-selected"]["with"]["selected"] = json!(false);
    assert!(assess_procedure(&opposite).is_err());
}

#[test]
fn export_requires_a_correlated_success_contract_and_still_reports_the_gap() {
    let wire = fixture("export-observe");
    has_unsupported(&wire, "job-correlated-completion");
    let mut generic = wire.clone();
    generic["do"][1]["observe-export"]["with"]["correlation"] = json!("jobs.done");
    assert!(assess_procedure(&generic).is_err());
    let mut failed = wire.clone();
    failed["do"][1]["observe-export"]["with"]["terminal"] = json!("failed");
    assert!(assess_procedure(&failed).is_err());
    let mut wrong_job = wire;
    wrong_job["do"][1]["observe-export"]["with"]["correlation"] =
        json!("job-id-returned-by-another-export");
    assert!(assess_procedure(&wrong_job).is_err());
}

#[test]
fn export_correlation_follows_the_task_identity_when_renamed() {
    let mut wire = fixture("export-observe");
    let task = wire["do"][0]
        .as_object_mut()
        .unwrap()
        .remove("start-export")
        .unwrap();
    wire["do"][0] = json!({"another-export": task});
    assert!(
        assess_procedure(&wire).is_err(),
        "old producer reference is dangling"
    );
    wire["do"][1]["observe-export"]["with"]["correlation"] =
        json!("job-id-returned-by-another-export");
    has_unsupported(&wire, "job-correlated-completion");
}

#[test]
fn no_procedure_can_omit_or_reorder_its_outcome() {
    for name in ["edit-save", "window-setting", "export-observe"] {
        let mut missing = fixture(name);
        missing["do"].as_array_mut().unwrap().pop();
        assert!(
            assess_procedure(&missing).is_err(),
            "{name}: missing outcome"
        );
        let mut reordered = fixture(name);
        reordered["do"].as_array_mut().unwrap().reverse();
        assert!(
            assess_procedure(&reordered).is_err(),
            "{name}: reversed causality"
        );
    }
}

#[test]
fn unsupported_versions_features_and_behaviors_are_not_silently_ignored() {
    let original = fixture("edit-save");
    for (path, value) in [
        ("/document/dsl", json!("2.0.0")),
        ("/document/version", json!("9.0.0")),
        (
            "/document/metadata/dplUiProcedure/profile",
            json!("future-profile"),
        ),
        (
            "/document/metadata/dplUiProcedure/mappingOnly",
            json!(false),
        ),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = value;
        assert!(assess_procedure(&changed).is_err(), "{path}");
    }
    let mut unknown_feature = original.clone();
    unknown_feature["document"]["metadata"]["dplUiProcedure"]["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("automatic-secret-recovery"));
    assert!(assess_procedure(&unknown_feature).is_err());
    let mut lost_obligation = original.clone();
    lost_obligation["document"]["metadata"]["dplUiProcedure"]["required"] =
        json!(["human-automation-equivalence"]);
    assert!(assess_procedure(&lost_obligation).is_err());
    let mut branch = original.clone();
    branch["do"][1]["save-look"]["then"] = json!("exit");
    assert!(assess_procedure(&branch).is_err());
    let mut arbitrary = original.clone();
    arbitrary["do"][1]["save-look"]["call"] = json!("shell");
    assert!(assess_procedure(&arbitrary).is_err());
    let mut expression = original;
    expression["do"][0]["name-look"]["with"]["text"] = json!("${ .secret }");
    assert!(assess_procedure(&expression).is_err());
}

#[test]
fn procedure_names_do_not_determine_capability_results() {
    let original = fixture("export-observe");
    let report = assess_procedure(&original).unwrap();
    let mut renamed = original;
    renamed["document"]["name"] = json!("edit-save");
    let renamed_report = assess_procedure(&renamed).unwrap();
    assert_eq!(renamed_report.required_features, report.required_features);
    assert_eq!(renamed_report.diagnostics, report.diagnostics);
}

#[test]
fn caller_descriptions_survive_without_becoming_verified_preconditions() {
    let mut wire = fixture("edit-save");
    let baseline = assess_procedure(&wire).unwrap();
    let added = "Operator must independently verify that the scratch library is mounted.";
    wire["document"]["metadata"]["dplUiProcedure"]["preconditions"]
        .as_array_mut()
        .unwrap()
        .push(json!(added));
    let report = assess_procedure(&wire).unwrap();
    let profile = &wire["document"]["metadata"]["dplUiProcedure"];
    assert_eq!(
        serde_json::to_value(&report.annotations.sources).unwrap(),
        profile["sources"]
    );
    assert_eq!(
        serde_json::to_value(&report.annotations.preconditions).unwrap(),
        profile["preconditions"]
    );
    assert_eq!(
        serde_json::to_value(&report.annotations.limitations).unwrap(),
        profile["limitations"]
    );
    assert!(!report.preconditions_verified);
    assert_eq!(report.required_features, baseline.required_features);
    assert_eq!(report.diagnostics, baseline.diagnostics);
    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["preconditions_verified"], false);
    assert!(json["annotations"]["preconditions"]
        .as_array()
        .unwrap()
        .contains(&json!(added)));
}

#[test]
fn document_identity_must_obey_the_upstream_wire_name_contract() {
    for invalid in [
        "contains space".to_owned(),
        "has_underscore".into(),
        "-starts".into(),
        "ends-".into(),
        "nöte".into(),
        "a".repeat(64),
    ] {
        let mut wire = fixture("edit-save");
        wire["document"]["name"] = json!(invalid);
        let issues = assess_procedure(&wire).unwrap_err();
        assert!(issues.iter().any(|issue| issue.path == "/document/name"));
    }
    let mut longest = fixture("edit-save");
    longest["document"]["name"] = json!("a".repeat(63));
    assert!(assess_procedure(&longest).is_ok());
}
