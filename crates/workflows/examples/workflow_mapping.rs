//! Offline mapping experiment. No app, endpoint, model or workflow is executed.

use dpl_workflow_interop::{
    assess_procedure, export_workflow, import_workflow, parse_document, MappingIssue,
    MappingResult, MAX_DOCUMENT_BYTES,
};
use serde_json::{json, Value};
use std::io::Read;
use std::process::ExitCode;

const OPEN_WORKFLOW_SCHEMA: &str = include_str!("../schemas/open-workflow-1.0.3.derived.json");
const ARAZZO_SCHEMA: &str = include_str!("../schemas/arazzo-1.1-2026-04-15.upstream.json");
const UI_FIXTURES: &[&str] = &[
    include_str!("../fixtures/open-workflow/edit-save.json"),
    include_str!("../fixtures/open-workflow/window-setting.json"),
    include_str!("../fixtures/open-workflow/export-observe.json"),
];
const ARAZZO_FIXTURE: &str = include_str!("../fixtures/arazzo/ui-procedures.json");

fn fail<T>(path: &str, reason: impl Into<String>) -> MappingResult<T> {
    Err(vec![MappingIssue {
        path: path.into(),
        reason: reason.into(),
    }])
}

fn validator(schema: &str) -> MappingResult<jsonschema::Validator> {
    let schema = parse_document(schema)?;
    jsonschema::validator_for(&schema).map_err(|error| {
        vec![MappingIssue {
            path: "schema".into(),
            reason: format!("pinned offline schema does not compile: {error}"),
        }]
    })
}

fn check(schema: &jsonschema::Validator, document: &Value) -> MappingResult<()> {
    let issues: Vec<_> = schema
        .iter_errors(document)
        .take(32)
        .map(|error| MappingIssue {
            path: "standard-schema".into(),
            reason: error.to_string(),
        })
        .collect();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn read(path: &str) -> MappingResult<Value> {
    let file = std::fs::File::open(path).map_err(|error| {
        vec![MappingIssue {
            path: path.into(),
            reason: format!("cannot read document: {error}"),
        }]
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            vec![MappingIssue {
                path: path.into(),
                reason: format!("cannot read document: {error}"),
            }]
        })?;
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        vec![MappingIssue {
            path: path.into(),
            reason: format!("document must be UTF-8 JSON: {error}"),
        }]
    })?;
    parse_document(text)
}

fn demo() -> MappingResult<Value> {
    let open_workflow = validator(OPEN_WORKFLOW_SCHEMA)?;
    let arazzo = validator(ARAZZO_SCHEMA)?;
    let mut controls = Vec::new();
    for native in dpl_workflows::workflows() {
        let wire = export_workflow(&native)?;
        check(&open_workflow, &wire)?;
        let restored = import_workflow(&wire)?;
        if restored != native {
            return fail(
                "round-trip",
                format!("native workflow {} changed", native.id),
            );
        }
        controls.push(json!({
            "id": native.id,
            "standard_schema": "valid",
            "native_round_trip": "equal",
            "execution": "not-run"
        }));
    }
    let mut procedures = Vec::new();
    for text in UI_FIXTURES {
        let wire = parse_document(text)?;
        check(&open_workflow, &wire)?;
        procedures.push(json!({
            "standard_schema": "valid",
            "assessment": assess_procedure(&wire)?
        }));
    }
    let arazzo_fixture = parse_document(ARAZZO_FIXTURE)?;
    check(&arazzo, &arazzo_fixture)?;
    Ok(json!({
        "experiment": "GF0",
        "status": "mapping-checks-passed",
        "runtime_qualification": "not-performed",
        "native_controls": controls,
        "ui_procedures": procedures,
        "arazzo": {
            "standard_schema": "valid",
            "operations": "hypothetical-adapter-contract-only",
            "runtime_support": "not-implemented",
            "export_observation": "bounded-polling-not-event-equivalence"
        }
    }))
}

fn command(arguments: &[String]) -> MappingResult<(Value, u8)> {
    match arguments {
        [] => Ok((demo()?, 0)),
        [mode] if mode == "demo" => Ok((demo()?, 0)),
        [mode, id] if mode == "export" => {
            let native = dpl_workflows::find(id).ok_or_else(|| vec![MappingIssue {
                path: "workflow".into(), reason: format!("unknown registered native workflow {id:?}"),
            }])?;
            let wire = export_workflow(&native)?;
            check(&validator(OPEN_WORKFLOW_SCHEMA)?, &wire)?;
            Ok((wire, 0))
        }
        [mode, path] if mode == "import" => {
            let wire = read(path)?;
            check(&validator(OPEN_WORKFLOW_SCHEMA)?, &wire)?;
            let native = import_workflow(&wire)?;
            Ok((json!({"status":"mapped", "execution":"not-run", "native":native}), 0))
        }
        [mode, path] if mode == "assess" => {
            let wire = read(path)?;
            check(&validator(OPEN_WORKFLOW_SCHEMA)?, &wire)?;
            let report = assess_procedure(&wire)?;
            Ok((json!({"standard_schema":"valid", "status":"unsupported-execution", "assessment":report}), 2))
        }
        _ => fail("arguments", "usage: mapping [demo | export <registered-id> | import <json-file> | assess <json-file>]"),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let (value, code) = match command(&arguments) {
        Ok(result) => result,
        Err(issues) => (json!({"status":"refused", "issues":issues}), 2),
    };
    match serde_json::to_string_pretty(&value) {
        Ok(text) => println!("{text}"),
        Err(error) => {
            eprintln!("cannot serialize mapping result: {error}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::from(code)
}
