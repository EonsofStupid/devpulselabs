use egui_pilot_ai_protocol::{
    CaptureResult, Flow, PilotError, SessionId, SessionStatus, StepOutcome,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
struct V1Handshake {
    protocol_version: u32,
    build: String,
    session_nonce: String,
}

#[derive(Serialize)]
struct V1ClientEnvelope {
    sequence: u64,
    session: SessionId,
    command: V1ClientCommand,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum V1ClientCommand {
    Handshake(V1Handshake),
    Status,
    Run(Flow),
}

#[derive(Serialize)]
struct V1ServerEnvelope {
    sequence: u64,
    session: SessionId,
    event: V1ServerEvent,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum V1ServerEvent {
    Ready(V1Handshake),
    Status(V1SessionStatus),
    Step(V1StepOutcome),
    Capture(V1CaptureResult),
    Error(PilotError),
    Stopped,
}

#[derive(Serialize)]
struct V1SessionStatus {
    running: bool,
    revision: u64,
    pending_actions: usize,
    busy_tokens: usize,
    viewport_count: usize,
    renderer_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    renderer_device_type: Option<String>,
}

#[derive(Serialize)]
struct V1StepOutcome {
    index: usize,
    passed: bool,
    elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context: Option<Value>,
}

#[derive(Serialize)]
struct V1CaptureResult {
    name: String,
    artifact: String,
    content_hash: String,
    renderer_key: String,
}

fn fixture(name: &str) -> Value {
    let text = match name {
        "client-handshake" => include_str!("fixtures/v1/client-handshake.json"),
        "server-ready" => include_str!("fixtures/v1/server-ready.json"),
        "client-status" => include_str!("fixtures/v1/client-status.json"),
        "server-status" => include_str!("fixtures/v1/server-status.json"),
        "client-run-shell" => include_str!("fixtures/v1/client-run-shell.json"),
        "server-step" => include_str!("fixtures/v1/server-step.json"),
        "server-capture" => include_str!("fixtures/v1/server-capture.json"),
        "server-error" => include_str!("fixtures/v1/server-error.json"),
        "server-stopped" => include_str!("fixtures/v1/server-stopped.json"),
        _ => panic!("unknown v1 fixture {name}"),
    };
    serde_json::from_str(text).expect("valid frozen v1 JSON")
}

fn assert_fixture(name: &str, actual: impl Serialize) {
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        fixture(name),
        "{name}"
    );
}

#[test]
fn frozen_v1_wire_examples_remain_reproducible() {
    let session = SessionId::new("compat");
    let handshake = || V1Handshake {
        protocol_version: 1,
        build: "runner-v1".into(),
        session_nonce: "nonce-v1".into(),
    };
    assert_fixture(
        "client-handshake",
        V1ClientEnvelope {
            sequence: 1,
            session: session.clone(),
            command: V1ClientCommand::Handshake(handshake()),
        },
    );
    assert_fixture(
        "server-ready",
        V1ServerEnvelope {
            sequence: 1,
            session: session.clone(),
            event: V1ServerEvent::Ready(V1Handshake {
                build: "target-v1".into(),
                ..handshake()
            }),
        },
    );
    assert_fixture(
        "client-status",
        V1ClientEnvelope {
            sequence: 2,
            session: session.clone(),
            command: V1ClientCommand::Status,
        },
    );
    assert_fixture(
        "server-status",
        V1ServerEnvelope {
            sequence: 2,
            session: session.clone(),
            event: V1ServerEvent::Status(V1SessionStatus {
                running: true,
                revision: 7,
                pending_actions: 0,
                busy_tokens: 0,
                viewport_count: 1,
                renderer_key: None,
                renderer_device_type: None,
            }),
        },
    );

    let shell: Flow = toml::from_str(include_str!("fixtures/v1/shell-navigation.toml")).unwrap();
    assert_fixture(
        "client-run-shell",
        V1ClientEnvelope {
            sequence: 3,
            session: session.clone(),
            command: V1ClientCommand::Run(shell),
        },
    );
    assert_fixture(
        "server-step",
        V1ServerEnvelope {
            sequence: 3,
            session: session.clone(),
            event: V1ServerEvent::Step(V1StepOutcome {
                index: 0,
                passed: true,
                elapsed_ms: 2,
                message: None,
                context: None,
            }),
        },
    );
    assert_fixture(
        "server-capture",
        V1ServerEnvelope {
            sequence: 3,
            session: session.clone(),
            event: V1ServerEvent::Capture(V1CaptureResult {
                name: "workbench-test-lab-flyout".into(),
                artifact: "captures/0123456789abcdef.png".into(),
                content_hash: "0123456789abcdef".into(),
                renderer_key: "windows-compat".into(),
            }),
        },
    );
    assert_fixture(
        "server-error",
        V1ServerEnvelope {
            sequence: 3,
            session: session.clone(),
            event: V1ServerEvent::Error(PilotError {
                code: "locator-ambiguous".into(),
                message: "two nodes matched".into(),
                context: None,
            }),
        },
    );
    assert_fixture(
        "server-stopped",
        V1ServerEnvelope {
            sequence: 4,
            session,
            event: V1ServerEvent::Stopped,
        },
    );
}

#[test]
fn frozen_shell_flow_matches_the_real_migration_evidence() {
    let frozen = include_str!("fixtures/v1/shell-navigation.toml").replace("\r\n", "\n");
    let real =
        include_str!("../../../tests/eguipilotai/shell-navigation.toml").replace("\r\n", "\n");
    assert_eq!(real.trim(), frozen.trim());
}

#[test]
fn current_v2_types_do_not_accidentally_match_v1_shapes() {
    assert!(serde_json::to_value(SessionStatus {
        running: true,
        revision: 0,
        pending_actions: 0,
        busy_tokens: 0,
        viewport_count: 1,
        viewports: Vec::new(),
        renderer_key: None,
        renderer_device_type: None,
        active_run: None,
        coach_signals: Vec::new(),
    })
    .unwrap()
    .get("active_run")
    .is_none());
    assert!(serde_json::to_value(CaptureResult {
        attachment: egui_pilot_ai_protocol::AttachmentId::new("fnv1a64:test"),
        name: "x".into(),
        artifact: "x.png".into(),
        content_hash: "test".into(),
        renderer_key: "test".into(),
        viewport: None,
        logical_size: None,
    })
    .unwrap()
    .get("attachment")
    .is_some());
    assert!(serde_json::to_value(StepOutcome {
        id: egui_pilot_ai_protocol::StepId::new("step"),
        index: 0,
        passed: true,
        elapsed_ms: 0,
        message: None,
        context: None,
    })
    .unwrap()
    .get("id")
    .is_some());
    assert_eq!(
        json!(egui_pilot_ai_protocol::PROTOCOL_VERSION),
        json!({"major": 2, "minor": 6})
    );
}
