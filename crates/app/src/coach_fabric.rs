//! The reverse seam: coach lifecycle facts onto the fabric (COACH domain).
//!
//! Two one-way modules, one direction each: `coach_signals` carries app
//! facts TO the coach; this carries coach facts FROM it — armed, mounted,
//! triggered, satisfied, unmounted — as stamped fabric signals the activity
//! feed, the session recorder, and (Phase 2) the ledger can hear.
//!
//! LAW (signal-ledger plan §9, Phase 1.5): the coach never hears its own
//! domain back. `CoachSignals` deliberately does not subscribe to `coach` —
//! a touchpoint waiting on a coach event would self-trigger.
//!
//! `Refused` maps to `None`: a refusal is a command result answered on the
//! pipe, not a lifecycle fact — and the plugin's tap never carries one.

use egui_pilot_ai::protocol;

pub fn to_fabric(event: &protocol::CoachEvent) -> Option<dpl_fabric::CoachEvent> {
    Some(match event {
        protocol::CoachEvent::Armed { coach_id } => dpl_fabric::CoachEvent::Armed {
            coach_id: coach_id.0.clone(),
        },
        protocol::CoachEvent::Mounted { coach_id, .. } => dpl_fabric::CoachEvent::Mounted {
            coach_id: coach_id.0.clone(),
        },
        protocol::CoachEvent::Triggered {
            coach_id,
            kind,
            actor,
        } => dpl_fabric::CoachEvent::Triggered {
            coach_id: coach_id.0.clone(),
            trigger: trigger_name(kind),
            actor: actor_name(*actor),
        },
        protocol::CoachEvent::Satisfied { coach_id } => dpl_fabric::CoachEvent::Satisfied {
            coach_id: coach_id.0.clone(),
        },
        protocol::CoachEvent::Unmounted { coach_id, reason } => dpl_fabric::CoachEvent::Unmounted {
            coach_id: coach_id.0.clone(),
            reason: reason_name(*reason).into(),
        },
        protocol::CoachEvent::Refused { .. } => return None,
    })
}

/// The CLI's `--on` vocabulary, so one trigger has one name on every surface.
fn trigger_name(kind: &protocol::TriggerKind) -> String {
    match kind {
        protocol::TriggerKind::Press => "press".into(),
        protocol::TriggerKind::Hover => "hover".into(),
        protocol::TriggerKind::NodeAppeared => "node-appeared".into(),
        protocol::TriggerKind::ValueChanged => "value-changed".into(),
        protocol::TriggerKind::FabricEvent { signal } => format!("signal:{signal}"),
    }
}

fn actor_name(actor: protocol::CoachActor) -> dpl_fabric::CoachActor {
    match actor {
        protocol::CoachActor::Operator => dpl_fabric::CoachActor::Operator,
        protocol::CoachActor::Agent => dpl_fabric::CoachActor::Agent,
        protocol::CoachActor::App => dpl_fabric::CoachActor::App,
    }
}

/// The protocol's kebab-case wire vocabulary, hand-held here because the
/// fabric stays serde-free (doctrine law 4).
fn reason_name(reason: protocol::CoachUnmountReason) -> &'static str {
    match reason {
        protocol::CoachUnmountReason::Ttl => "ttl",
        protocol::CoachUnmountReason::Operator => "operator",
        protocol::CoachUnmountReason::Agent => "agent",
        protocol::CoachUnmountReason::Once => "once",
        protocol::CoachUnmountReason::TargetLost => "target-lost",
        protocol::CoachUnmountReason::SessionEnded => "session-ended",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_pilot_ai::protocol::{CoachId, SignalId};

    #[test]
    fn every_lifecycle_fact_crosses_and_the_refusal_stays_behind() {
        let id = || CoachId::new("lesson-1/step-2");
        assert_eq!(
            to_fabric(&protocol::CoachEvent::Satisfied { coach_id: id() }),
            Some(dpl_fabric::CoachEvent::Satisfied {
                coach_id: "lesson-1/step-2".into()
            })
        );
        assert_eq!(
            to_fabric(&protocol::CoachEvent::Triggered {
                coach_id: id(),
                kind: protocol::TriggerKind::FabricEvent {
                    signal: SignalId::new("build.slide-placed"),
                },
                actor: protocol::CoachActor::App,
            }),
            Some(dpl_fabric::CoachEvent::Triggered {
                coach_id: "lesson-1/step-2".into(),
                trigger: "signal:build.slide-placed".into(),
                actor: dpl_fabric::CoachActor::App,
            })
        );
        assert_eq!(
            to_fabric(&protocol::CoachEvent::Refused {
                coach_id: id(),
                reason: protocol::CoachRefusal::TargetNotFound,
                detail: String::new(),
            }),
            None,
            "a refusal is a command result, not a lifecycle fact"
        );
    }
}
