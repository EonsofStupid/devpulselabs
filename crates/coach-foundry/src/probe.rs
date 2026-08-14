//! The live probe battery — safe, restorative verbs executed K times
//! each, every run an S1 seam span into the same Parquet layout the
//! daemon and the app write. Reliability is measured on the seam,
//! never asserted (seam ADR §3.5).

use crate::artifacts::{BatteryRow, ProbeRow};
use dpl_coach_runtime::{
    app_assert_state, app_wait_until, require_real_target, window_focus, window_geom, window_move,
    window_resize,
};
use dpl_fabric::{Fabric as SignalFabric, SeamAgency, SeamEvent, SeamSpan, TransportRung};
use std::path::Path;
use std::time::Instant;

/// The Foundry's span channel: a local signal fabric with the ledger
/// listening — the same meeting-point-files pattern the daemon uses.
pub struct SpanSink {
    signals: SignalFabric,
    tap: Option<dpl_ledger::LedgerTap>,
    /// `None` only when the ledger could not open — reported, never silent.
    pub ledger_error: Option<String>,
}

impl SpanSink {
    pub fn open(analytics_root: &Path) -> Self {
        let signals = SignalFabric::new();
        match dpl_ledger::LedgerTap::open(&signals, analytics_root.to_path_buf()) {
            Ok(tap) => Self {
                signals,
                tap: Some(tap),
                ledger_error: None,
            },
            Err(e) => Self {
                signals,
                tap: None,
                ledger_error: Some(e),
            },
        }
    }

    fn publish(&mut self, event: SeamEvent) {
        self.signals.publish_seam(event);
        if let Some(tap) = self.tap.as_mut() {
            tap.drain();
        }
    }

    /// Drain the last spans to Parquet. Call once, at pipeline end.
    pub fn flush(mut self) {
        if let Some(mut tap) = self.tap.take() {
            tap.drain();
            tap.flush_and_join();
        }
    }
}

/// One safe probe: verb name, oracle name, honest mechanism, and the
/// execution returning `effect_observed` (or a refusal string).
struct Probe {
    verb: &'static str,
    oracle: &'static str,
    mechanism: &'static str,
    run: fn(&str) -> Result<bool, String>,
}

/// The v0 probe set — read-mostly, and the two mutators restore what
/// they moved. Lifecycle (`open`/`close`) stays out of v0: killing the
/// operator's live app is not a probe (plan §4.1 safety rail).
const PROBES: [Probe; 7] = [
    Probe {
        verb: "presence",
        oracle: "real",
        mechanism: "atspi-read",
        run: |app| match require_real_target(app) {
            Ok(_) => Ok(true),
            Err(r) => Err(r.reason),
        },
    },
    Probe {
        verb: "geom",
        oracle: "geometry-read",
        mechanism: "x11-wm",
        run: |app| match window_geom(app) {
            Ok(g) => Ok(g.w > 0 && g.h > 0),
            Err(r) => Err(r.reason),
        },
    },
    Probe {
        verb: "focus",
        oracle: "active-window",
        mechanism: "x11-wm",
        run: |app| match window_focus(app) {
            Ok(e) => Ok(e.effect_observed),
            Err(r) => Err(r.reason),
        },
    },
    Probe {
        verb: "move",
        oracle: "geometry-delta",
        mechanism: "x11-wm",
        run: |app| {
            let before = window_geom(app).map_err(|r| r.reason)?;
            let effect = window_move(app, before.x + 24, before.y + 24)
                .map_err(|r| r.reason)?
                .effect_observed;
            // Restore — a probe never leaves the operator's window moved.
            let _ = window_move(app, before.x, before.y);
            Ok(effect)
        },
    },
    Probe {
        verb: "resize",
        oracle: "geometry-delta",
        mechanism: "x11-wm",
        run: |app| {
            let before = window_geom(app).map_err(|r| r.reason)?;
            let effect = window_resize(app, before.w + 30, before.h + 30)
                .map_err(|r| r.reason)?
                .effect_observed;
            let _ = window_resize(app, before.w, before.h);
            Ok(effect)
        },
    },
    Probe {
        verb: "wait_until",
        oracle: "predicate-match",
        mechanism: "atspi-read",
        run: |app| match app_wait_until(app, "real", 2_000) {
            Ok(e) => Ok(e.matched),
            Err(r) => Err(r.reason),
        },
    },
    Probe {
        verb: "assert_state",
        oracle: "predicate-match",
        mechanism: "atspi-read",
        run: |app| match app_assert_state(app, "real", None) {
            Ok(e) => Ok(e.matched),
            Err(r) => Err(r.reason),
        },
    },
];

fn span(
    probe: &Probe,
    app_id: &str,
    atspi_name: &str,
    stage: &str,
    effect: bool,
    duration_ms: u64,
) -> SeamSpan {
    SeamSpan {
        verb: probe.verb.into(),
        app: app_id.into(),
        rung: TransportRung::R1Semantic,
        downgraded_from: None,
        mechanism: probe.mechanism.into(),
        agency: SeamAgency::Teacher,
        ok: true,
        effect_observed: effect,
        duration_ms,
        lesson: None,
        beat: None,
        attrs: vec![
            ("driver".into(), "foundry".into()),
            ("foundry_stage".into(), stage.into()),
            ("atspi_name".into(), atspi_name.into()),
        ],
    }
}

/// Probe stage: every probe K times against the live app. Refusals are
/// first-class facts — they publish `SeamEvent::Refused` and count
/// against reliability (the denominator is every attempt).
pub fn run_probes(
    app_id: &str,
    atspi_name: &str,
    runs_per_verb: u32,
    sink: &mut SpanSink,
    mut on_progress: impl FnMut(&str, u32, u32),
) -> Vec<ProbeRow> {
    PROBES
        .iter()
        .map(|probe| {
            let mut row = ProbeRow {
                verb: probe.verb.into(),
                rung: TransportRung::R1Semantic.label().into(),
                oracle: probe.oracle.into(),
                runs: 0,
                effect_ok: 0,
                total_ms: 0,
            };
            for i in 0..runs_per_verb {
                on_progress(probe.verb, i + 1, runs_per_verb);
                let started = Instant::now();
                let outcome = (probe.run)(atspi_name);
                let duration_ms = started.elapsed().as_millis() as u64;
                row.runs += 1;
                row.total_ms += duration_ms;
                match outcome {
                    Ok(effect) => {
                        if effect {
                            row.effect_ok += 1;
                        }
                        sink.publish(SeamEvent::Executed(span(
                            probe,
                            app_id,
                            atspi_name,
                            "probe",
                            effect,
                            duration_ms,
                        )));
                    }
                    Err(reason) => {
                        sink.publish(SeamEvent::Refused {
                            verb: probe.verb.into(),
                            app: app_id.into(),
                            reason,
                            duration_ms,
                        });
                    }
                }
            }
            row
        })
        .collect()
}

/// Battery stage: one verifying run per granted verb — the draft's
/// grants must all hold before the pack may publish.
pub fn run_battery(
    app_id: &str,
    atspi_name: &str,
    granted_verbs: &[String],
    sink: &mut SpanSink,
) -> Vec<BatteryRow> {
    granted_verbs
        .iter()
        .map(|verb| {
            let Some(probe) = PROBES.iter().find(|p| p.verb == verb.as_str()) else {
                return BatteryRow {
                    verb: verb.clone(),
                    ok: false,
                    detail: "no probe implementation for this grant".into(),
                };
            };
            let started = Instant::now();
            let outcome = (probe.run)(atspi_name);
            let duration_ms = started.elapsed().as_millis() as u64;
            match outcome {
                Ok(effect) => {
                    sink.publish(SeamEvent::Executed(span(
                        probe,
                        app_id,
                        atspi_name,
                        "battery",
                        effect,
                        duration_ms,
                    )));
                    BatteryRow {
                        verb: verb.clone(),
                        ok: effect,
                        detail: if effect {
                            "effect observed".into()
                        } else {
                            "ran, no observable effect".into()
                        },
                    }
                }
                Err(reason) => {
                    sink.publish(SeamEvent::Refused {
                        verb: verb.clone(),
                        app: app_id.into(),
                        reason: reason.clone(),
                        duration_ms,
                    });
                    BatteryRow {
                        verb: verb.clone(),
                        ok: false,
                        detail: reason,
                    }
                }
            }
        })
        .collect()
}

/// The probe verbs in battery order — what Induct may grant in v0.
pub fn probe_verbs() -> Vec<&'static str> {
    PROBES.iter().map(|p| p.verb).collect()
}
