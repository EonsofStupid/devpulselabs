use dpl_workflows::interop::{export_workflow, import_workflow, MappingIssue, CALL_PREFIX};
use serde_json::{json, Value};

fn fixture() -> Value {
    export_workflow(&dpl_workflows::workflows()[0]).expect("registered workflow exports")
}

fn first_task(wire: &mut Value) -> &mut Value {
    wire["do"][0]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()
}

fn assert_path(issues: &[MappingIssue], suffix: &str) {
    assert!(
        issues.iter().any(|issue| issue.path.ends_with(suffix)),
        "expected path suffix {suffix:?}; got {issues:?}"
    );
}

#[test]
fn every_registered_workflow_round_trips_without_an_opaque_payload() {
    let workflows = dpl_workflows::workflows();
    assert!(!workflows.is_empty());
    for native in workflows {
        let wire = export_workflow(&native).unwrap();
        assert_eq!(import_workflow(&wire).unwrap(), native);
        assert_eq!(wire["document"]["name"], native.id);
        let metadata = &wire["document"]["metadata"]["devpulse"];
        assert!(metadata.get("stages").is_none());
        assert!(metadata.get("id").is_none());
        for (task, stage) in wire["do"].as_array().unwrap().iter().zip(&native.stages) {
            let body = &task[&stage.id];
            assert_eq!(body["call"], format!("{CALL_PREFIX}{}", stage.kind.label()));
            assert!(body["with"].is_object());
            assert!(body["with"].get("kind").is_none());
            assert!(body["with"].get("stages").is_none());
        }
    }
}

#[test]
fn versions_and_foreign_profiles_are_refused() {
    for (key, value) in [
        ("dsl", "1.0.2"),
        ("namespace", "other"),
        ("version", "0.2.0"),
    ] {
        let mut wire = fixture();
        wire["document"][key] = json!(value);
        assert_path(
            &import_workflow(&wire).unwrap_err(),
            &format!("/document/{key}"),
        );
    }
    for version in [0, dpl_workflows::SCHEMA_V + 1] {
        let mut native = dpl_workflows::workflows().remove(0);
        native.v = version;
        assert_path(&export_workflow(&native).unwrap_err(), "native.v");
        let mut wire = fixture();
        wire["document"]["metadata"]["devpulse"]["v"] = json!(version);
        assert_path(&import_workflow(&wire).unwrap_err(), "native.v");
    }
}

#[test]
fn unsupported_task_behavior_never_disappears() {
    for key in ["then", "if", "retry", "timeout", "run", "metadata"] {
        let mut wire = fixture();
        first_task(&mut wire)[key] = json!({"required": true});
        assert_path(&import_workflow(&wire).unwrap_err(), &format!("/{key}"));
    }
    let mut wire = fixture();
    wire["use"] = json!({"functions": {}});
    assert_path(&import_workflow(&wire).unwrap_err(), "/use");
    let mut wire = fixture();
    wire["do"][0]["second-task"] = json!({"call":"devpulse.render","with":{}});
    assert_path(&import_workflow(&wire).unwrap_err(), "/do/0");
}

#[test]
fn unknown_call_and_invalid_native_stage_are_refused() {
    for call in ["http", "devpulse.shell", "devpulse.future-stage"] {
        let mut wire = fixture();
        first_task(&mut wire)["call"] = json!(call);
        assert!(import_workflow(&wire).is_err(), "{call}");
    }
    let mut wire = fixture();
    let task = first_task(&mut wire);
    task["call"] = json!("devpulse.outline");
    task["with"] = json!({"beats": []});
    let errors = import_workflow(&wire).unwrap_err();
    assert!(errors
        .iter()
        .any(|issue| issue.reason == "outline has no beats"));
    let mut wire = fixture();
    first_task(&mut wire)["with"]["kind"] = json!("render");
    assert_path(&import_workflow(&wire).unwrap_err(), "/with/kind");
}

#[test]
fn native_serde_cannot_silently_drop_fields_at_any_depth() {
    let mut wire = fixture();
    first_task(&mut wire)["with"]["future-required-behavior"] = json!(true);
    assert_path(
        &import_workflow(&wire).unwrap_err(),
        "/with/future-required-behavior",
    );
    let mut wire = fixture();
    wire["document"]["metadata"]["devpulse"]["secret-policy"] = json!("required");
    assert_path(&import_workflow(&wire).unwrap_err(), "/secret-policy");
    let mut wire = fixture();
    let task = first_task(&mut wire);
    task["call"] = json!("devpulse.outline");
    task["with"] = json!({"beats":[{
        "id":"example", "role":"hook", "title":"Example", "brief":"A beat",
        "must-check":"persisted"
    }]});
    assert_path(&import_workflow(&wire).unwrap_err(), "/beats/0/must-check");
}

#[test]
fn metadata_cannot_override_identity_or_task_sequence() {
    for key in ["id", "stages"] {
        let mut wire = fixture();
        wire["document"]["metadata"]["devpulse"][key] = json!("replacement");
        assert_path(
            &import_workflow(&wire).unwrap_err(),
            &format!("/devpulse/{key}"),
        );
    }
}

#[test]
fn duplicate_task_ids_and_missing_fields_are_refused() {
    let mut wire = fixture();
    let task = wire["do"][0].clone();
    wire["do"].as_array_mut().unwrap().push(task);
    assert!(import_workflow(&wire)
        .unwrap_err()
        .iter()
        .any(|issue| issue.reason == "duplicate stage id"));
    for key in ["document", "do"] {
        let mut wire = fixture();
        wire.as_object_mut().unwrap().remove(key);
        assert_path(&import_workflow(&wire).unwrap_err(), &format!("/{key}"));
    }
    let mut wire = fixture();
    first_task(&mut wire)
        .as_object_mut()
        .unwrap()
        .remove("with");
    assert_path(&import_workflow(&wire).unwrap_err(), "/with");
}

#[test]
fn expression_looking_arguments_are_not_reinterpreted_as_native_literals() {
    let mut wire = fixture();
    let task = first_task(&mut wire);
    task["call"] = json!("devpulse.outline");
    task["with"] = json!({"beats":[{
        "id":"example", "role":"hook", "title":"Example", "brief":"${ .secret }"
    }]});
    assert_path(&import_workflow(&wire).unwrap_err(), "/beats/0/brief");
    let mut native = dpl_workflows::workflows().remove(0);
    native.stages[0].kind = dpl_workflows::StageKind::Outline {
        beats: vec![dpl_workflows::Beat {
            id: "example".into(),
            role: "hook".into(),
            title: "Example".into(),
            brief: "A literal ${ expression-looking value".into(),
        }],
    };
    assert_path(&export_workflow(&native).unwrap_err(), "/beats/0/brief");
}

#[test]
fn wire_names_refuse_native_ids_that_violate_upstream_bounds() {
    let base = dpl_workflows::workflows().remove(0);
    for name in [
        "_".to_string(),
        "demo_name".into(),
        "-demo".into(),
        "demo-".into(),
        "a".repeat(64),
    ] {
        let mut native = base.clone();
        native.id = name.clone();
        assert!(
            dpl_workflows::validate(&native).is_empty(),
            "native allows {name:?}"
        );
        assert_path(&export_workflow(&native).unwrap_err(), "/document/name");
        let mut wire = fixture();
        wire["document"]["name"] = json!(name);
        assert_path(&import_workflow(&wire).unwrap_err(), "/document/name");
    }
    let mut native = base;
    native.id.clear();
    assert_path(&export_workflow(&native).unwrap_err(), "native.id");
    let mut wire = fixture();
    wire["document"]["name"] = json!("");
    assert!(import_workflow(&wire).is_err());
}

#[test]
fn native_wire_name_boundary_values_preserve_identity() {
    for name in [
        "a".to_string(),
        "0".into(),
        "a".repeat(63),
        format!("a{}0", "-".repeat(61)),
    ] {
        let mut native = dpl_workflows::workflows().remove(0);
        native.id = name.clone();
        let wire = export_workflow(&native).unwrap();
        assert_eq!(wire["document"]["name"], name);
        assert_eq!(import_workflow(&wire).unwrap(), native);
    }
}
