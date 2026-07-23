use egui_pilot_ai_protocol::{
    ClientCommand, CommandEnvelope, CommandResult, EventId, EventScope, RequestId,
    ResponseEnvelope, ResponseOutcome, RuntimeEvent, RuntimeEventEnvelope, ServerMessage,
    SessionId,
};
use serde::Serialize;
use serde_json::Value;

fn assert_golden(text: &str, actual: impl Serialize) {
    let expected: Value = serde_json::from_str(text).unwrap();
    assert_eq!(serde_json::to_value(actual).unwrap(), expected);
}

#[test]
fn v2_command_response_and_event_envelopes_are_golden() {
    let request_id = RequestId::new("request-00000000000000000001");
    let session = SessionId::new("session");
    assert_golden(
        include_str!("fixtures/v2/command-status.json"),
        CommandEnvelope {
            request_id: request_id.clone(),
            session: session.clone(),
            command: ClientCommand::Status,
        },
    );
    assert_golden(
        include_str!("fixtures/v2/response-stopped.json"),
        ServerMessage::response(ResponseEnvelope {
            request_id,
            session: session.clone(),
            outcome: ResponseOutcome::Success {
                result: Box::new(CommandResult::Stopped),
            },
        }),
    );
    assert_golden(
        include_str!("fixtures/v2/event-input-cleanup.json"),
        ServerMessage::event(RuntimeEventEnvelope {
            event_id: EventId::from_sequence(1),
            session,
            scope: EventScope::default(),
            event: RuntimeEvent::InputCleanup,
        }),
    );
}

#[test]
fn unknown_required_wire_variants_fail_closed() {
    let unknown = r#"{"request_id":"r","session":"s","command":{"type":"future-command"}}"#;
    assert!(serde_json::from_str::<CommandEnvelope>(unknown).is_err());
    let unknown = r#"{"kind":"event","event_id":"event-00000000000000000001","session":"s","scope":{},"event":{"type":"future-required-event"}}"#;
    assert!(serde_json::from_str::<ServerMessage>(unknown).is_err());
}
