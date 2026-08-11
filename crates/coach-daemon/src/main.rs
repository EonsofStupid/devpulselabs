//! dpl-coach-daemon — localhost Axum TRANSLATOR onto the protocol seam.
//!
//! Agent hop: HTTP on 127.0.0.1. App hop: AT-SPI / X11 via dpl-coach-runtime.
//! Since S2 (docs/plans/coach-seam-s2-protocol-rehoming.md) the daemon
//! speaks `egui_pilot_ai_protocol::seam` end to end: `body.verb` parses
//! to a typed `SeamVerb` at the HTTP edge (bare strings die here),
//! `/verbs` derives from `seam_verb_catalog()`, every mutating verb
//! answers with a protocol `EffectContract`, and `/events` emits typed
//! protocol `SeamEvent` facts.
//! Reality gates refuse ghost AT-SPI. ADR: docs/plans/coach-control-plane-adr.md
//! Verb catalog: docs/plans/coach-operator-verb-catalog-adr.md
//!
//! ```text
//! dpl-coach-daemon [--port 7843]
//! curl -s http://127.0.0.1:7843/presence?app=gnome-calculator
//! curl -s http://127.0.0.1:7843/verbs
//! ```

#![forbid(unsafe_code)]

// spawn-audit: exempt(devtools daemon — operator launches; may spawn overlay)

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use dpl_coach_fabric::{mint, request_from_inventory_name, MintOutcome};
use dpl_coach_inventory::inventory;
use dpl_coach_runtime::{
    app_assert_state, app_close, app_open, app_press_key, app_scroll, app_select, app_type_text,
    app_wait_until, extents, invoke_gated, mount_spec, probe_presence, require_real_target,
    resolve_seed, window_focus, window_geom, window_move, window_resize, AgencyMode, FabricSession,
    MountSpec, OuterGeom,
};
// Two vocabularies meet here, deliberately qualified: the protocol seam
// (wire types; `seam::SeamEvent` is the /events fact) and the fabric's
// ledger domain (`dpl_fabric::SeamEvent` is the S1 span fact). The
// fabric's private `TransportRung` survives S2 only because its file is
// held by a parallel lane — `fabric_rung` + the label-equality test
// below hold the two enums identical until the named follow-up retires
// the copy (S2 plan §4.1 fallback).
use dpl_fabric::{Fabric as SignalFabric, SeamAgency, SeamEvent, SeamSpan};
use egui_pilot_ai_protocol::seam::{
    self, EffectContract, Observables, SeamVerb, TransportRung, WindowGeom,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Clone)]
struct AppState {
    sessions: Arc<Mutex<HashMap<String, FabricSession>>>,
    /// The tracing spine (seam ADR S1): spans publish here; the ledger
    /// drain task writes them to Parquet. Daemon-local — a separate
    /// process cannot ride the app's in-process fabric; the files join.
    signals: SignalFabric,
}

#[derive(Debug, Deserialize)]
struct PresenceQuery {
    app: String,
}

#[derive(Debug, Deserialize)]
struct ImportBody {
    name: String,
    #[serde(default)]
    seed: Option<String>,
    #[serde(default)]
    agency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionBody {
    session: String,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    require_effect: Option<bool>,
    #[serde(default)]
    spawn_overlay: Option<bool>,
    #[serde(default)]
    seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct VerbBody {
    verb: String,
    #[serde(default)]
    app: Option<String>,
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
    #[serde(default)]
    w: Option<i32>,
    #[serde(default)]
    h: Option<i32>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    predicate: Option<String>,
    #[serde(default)]
    expect: Option<String>,
    #[serde(default)]
    display_contains: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default)]
    delta: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    app: String,
    #[serde(default)]
    seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    refused: bool,
    reason: String,
    tip: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let mut port: u16 = 7843;
    let mut analytics_root: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => {
                port = args.next().and_then(|p| p.parse().ok()).unwrap_or(7843);
            }
            "--analytics-root" => {
                analytics_root = args.next().map(PathBuf::from);
            }
            "--help" | "-h" => {
                println!("usage: dpl-coach-daemon [--port 7843] [--analytics-root <path>]");
                return;
            }
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
    }

    let signals = SignalFabric::new();
    // Open the ledger ears BEFORE serving — the first verb must land.
    // The resolved root is logged loudly (honesty law): silent
    // non-recording is how weeks of stats were lost before S1.
    let root = resolve_analytics_root(analytics_root);
    let tap = match dpl_ledger::LedgerTap::open(&signals, root.clone()) {
        Ok(tap) => {
            tracing::info!(root = %root.display(), "seam spans writing");
            Some(tap)
        }
        Err(e) => {
            tracing::error!(%e, root = %root.display(), "seam spans NOT recorded");
            None
        }
    };
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let drain = tokio::spawn(async move {
        let mut tap = tap;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(250)) => {
                    if let Some(t) = tap.as_mut() {
                        t.drain();
                    }
                }
                _ = shutdown_rx.changed() => break,
            }
        }
        if let Some(mut t) = tap.take() {
            t.drain();
            t.flush_and_join();
        }
    });

    let state = AppState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
        signals,
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/presence", get(presence))
        .route("/inventory", get(inventory_handler))
        .route("/import", post(import_handler))
        .route("/resolve", post(resolve_handler))
        .route("/invoke", post(invoke_handler))
        .route("/mount", post(mount_handler))
        .route("/verbs", get(verbs_handler))
        .route("/verb", post(verb_handler))
        .route("/events", get(events_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("coach control plane listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("bind {addr}: {e}");
            std::process::exit(1);
        });
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
        })
        .await
        .ok();
    // Flush the last spans before exit — exit skips Drop.
    let _ = shutdown_tx.send(true);
    let _ = drain.await;
}

/// Flag → `DPL_ANALYTICS_ROOT` → `USERDATA_ROOT`/analytics → the checkout
/// default the app also lands on when launched here. NOT an
/// `engine-capture` dep — one path fn is not worth that edge; the flag is
/// the exactness lever and the resolved path is logged at boot.
fn resolve_analytics_root(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    if let Ok(p) = std::env::var("DPL_ANALYTICS_ROOT") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("USERDATA_ROOT") {
        return PathBuf::from(p).join("analytics");
    }
    PathBuf::from(".userdata/analytics")
}

const fn seam_agency(a: AgencyMode) -> SeamAgency {
    match a {
        AgencyMode::Coach => SeamAgency::Coach,
        AgencyMode::Teacher => SeamAgency::Teacher,
    }
}

/// Protocol rung → the fabric's private span rung. Wire labels are
/// identical by the test below; the fabric copy retires in its own
/// micro-slice once its file is free (S2 plan §4.1 fallback, taken
/// because a parallel lane holds `crates/fabric/src/lib.rs`).
const fn fabric_rung(r: TransportRung) -> dpl_fabric::TransportRung {
    match r {
        TransportRung::R0Native => dpl_fabric::TransportRung::R0Native,
        TransportRung::R1Semantic => dpl_fabric::TransportRung::R1Semantic,
        TransportRung::R2Injection => dpl_fabric::TransportRung::R2Injection,
        TransportRung::R3Vision => dpl_fabric::TransportRung::R3Vision,
    }
}

async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "dpl-coach-daemon",
        "bind": "127.0.0.1",
        "adr": "docs/plans/coach-control-plane-adr.md",
        "verbs_adr": "docs/plans/coach-operator-verb-catalog-adr.md"
    }))
}

/// One source of truth: `/verbs` derives from the protocol catalog —
/// the introspection surface can never under-claim what `execute_verb`
/// accepts again (`geom` was shipped-but-unlisted for a week).
async fn verbs_handler() -> Json<Value> {
    let verbs: Vec<Value> = seam::seam_verb_catalog()
        .iter()
        .map(|d| {
            json!({
                "name": d.verb,
                "status": d.status.as_str(),
                "target": d.target.as_str(),
            })
        })
        .collect();
    Json(json!({ "verbs": verbs }))
}

/// What one verb execution produced, beside its span facts (plan §4:
/// the rung table is v1 static; `mechanism` names the honest means;
/// scroll resolves its rung from the effect's `via` at runtime). The
/// typed pieces assemble into the wire `EffectContract` at the handler;
/// `attrs` stays the S1 span vocabulary, untouched by S2.
struct VerbOutcome {
    ok: bool,
    /// The effect's own app name (AT-SPI resolved) when it reports one.
    app: Option<String>,
    rung: TransportRung,
    downgraded_from: Option<TransportRung>,
    mechanism: String,
    effect_observed: bool,
    before: Observables,
    after: Observables,
    extra: Vec<(String, String)>,
    attrs: Vec<(String, String)>,
}

impl VerbOutcome {
    fn new(rung: TransportRung, mechanism: &str, effect_observed: bool) -> Self {
        Self {
            ok: true,
            app: None,
            rung,
            downgraded_from: None,
            mechanism: mechanism.into(),
            effect_observed,
            before: Observables::default(),
            after: Observables::default(),
            extra: Vec::new(),
            attrs: Vec::new(),
        }
    }

    fn attr(mut self, key: &str, value: impl ToString) -> Self {
        self.attrs.push((key.into(), value.to_string()));
        self
    }

    fn extra(mut self, key: &str, value: impl ToString) -> Self {
        self.extra.push((key.into(), value.to_string()));
        self
    }

    fn contract(self, verb: &SeamVerb, requested_app: &str) -> EffectContract {
        EffectContract {
            ok: self.ok,
            verb: verb.wire_name().to_string(),
            app: self.app.unwrap_or_else(|| requested_app.to_string()),
            rung: self.rung,
            downgraded_from: self.downgraded_from,
            effect_observed: self.effect_observed,
            before: self.before,
            after: self.after,
            extra: self.extra.into_iter().collect(),
        }
    }
}

async fn verb_handler(
    State(state): State<AppState>,
    Json(body): Json<VerbBody>,
) -> Result<Json<Value>, (StatusCode, Json<ApiError>)> {
    let raw = body.verb.to_ascii_lowercase();
    let app = body.app.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                refused: true,
                reason: "app required".into(),
                tip: "Pass app like Calculator or gnome-calculator.".into(),
            }),
        )
    })?;
    let started = std::time::Instant::now();
    // Bare verb strings die HERE: parse to the protocol vocabulary at
    // the HTTP edge; unknown names refuse by name, exactly as before.
    let result = match SeamVerb::parse(&raw) {
        Ok(verb) => {
            let (verb_c, app_c) = (verb.clone(), app.clone());
            tokio::task::spawn_blocking(move || {
                execute_verb(&verb_c, &app_c, body).map(|outcome| (verb_c, outcome))
            })
            .await
            .map_err(join_err)?
        }
        Err(e) => Err(dpl_coach_fabric::Refused {
            reason: e.to_string(),
            tip: "GET /verbs for shipped names. Use /import|/invoke for fabric verbs.".into(),
        }),
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok((verb, outcome)) => {
            state.signals.publish_seam(SeamEvent::Executed(SeamSpan {
                verb: verb.wire_name().to_string(),
                app: app.clone(),
                rung: fabric_rung(outcome.rung),
                downgraded_from: outcome.downgraded_from.map(fabric_rung),
                mechanism: outcome.mechanism.clone(),
                agency: SeamAgency::Teacher,
                ok: true,
                effect_observed: outcome.effect_observed,
                duration_ms,
                lesson: None,
                beat: None,
                attrs: outcome.attrs.clone(),
            }));
            let contract = outcome.contract(&verb, &app);
            Ok(Json(
                serde_json::to_value(contract).unwrap_or_else(|_| json!({"ok": false})),
            ))
        }
        Err(r) => {
            state.signals.publish_seam(SeamEvent::Refused {
                verb: raw,
                app,
                reason: r.reason.clone(),
                duration_ms,
            });
            Err(map_refused(r))
        }
    }
}

/// The runtime's effect structs (engine internals) dispose into the
/// wire `EffectContract` HERE, at the daemon edge — field by field per
/// the S2 plan §4.5 table. Runtime internals were not touched.
fn execute_verb(
    verb: &SeamVerb,
    app: &str,
    body: VerbBody,
) -> Result<VerbOutcome, dpl_coach_fabric::Refused> {
    let timeout_ms = body.timeout_ms.unwrap_or(8_000);
    match verb {
        SeamVerb::Move => {
            let x = body.x.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "move requires x".into(),
                tip: "Pass x and y.".into(),
            })?;
            let y = body.y.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "move requires y".into(),
                tip: "Pass x and y.".into(),
            })?;
            let e = window_move(app, x, y)?;
            Ok(window_outcome(&e).attr("x", x).attr("y", y))
        }
        SeamVerb::Resize => {
            let w = body.w.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "resize requires w".into(),
                tip: "Pass w and h (≥ 50).".into(),
            })?;
            let h = body.h.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "resize requires h".into(),
                tip: "Pass w and h (≥ 50).".into(),
            })?;
            let e = window_resize(app, w, h)?;
            Ok(window_outcome(&e).attr("w", w).attr("h", h))
        }
        SeamVerb::Focus => {
            let e = window_focus(app)?;
            Ok(window_outcome(&e))
        }
        SeamVerb::Geom => {
            let g = window_geom(app)?;
            let e = dpl_coach_runtime::WindowVerbEffect {
                verb: "geom".into(),
                effect_observed: true,
                before: g,
                after: g,
            };
            Ok(window_outcome(&e))
        }
        SeamVerb::Open => {
            let e = app_open(app, timeout_ms)?;
            Ok(lifecycle_outcome(&e, "spawn"))
        }
        SeamVerb::Close => {
            let e = app_close(app, timeout_ms)?;
            Ok(lifecycle_outcome(&e, "wm-delete"))
        }
        SeamVerb::WaitUntil => {
            let pred = body.predicate.as_deref().unwrap_or("real");
            let e = app_wait_until(app, pred, timeout_ms)?;
            Ok(wait_assert_outcome(&e).attr("predicate", pred))
        }
        SeamVerb::AssertState => {
            let expect = body.expect.as_deref().unwrap_or("real");
            let e = app_assert_state(app, expect, body.display_contains.as_deref())?;
            if !e.ok {
                return Err(dpl_coach_fabric::Refused {
                    reason: e
                        .detail
                        .clone()
                        .unwrap_or_else(|| "assert_state failed".into()),
                    tip: "Open/close the app or adjust expect/display_contains.".into(),
                });
            }
            Ok(wait_assert_outcome(&e).attr("expect", expect))
        }
        SeamVerb::TypeText => {
            let text = body.text.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "type_text requires text".into(),
                tip: "Pass {\"text\":\"3\"}.".into(),
            })?;
            let e = app_type_text(app, &text)?;
            let mut out =
                VerbOutcome::new(TransportRung::R2Injection, "xdotool", e.effect_observed)
                    .extra("text", &e.text)
                    .attr("text_len", text.len());
            out.app = Some(e.atspi_name);
            out.before.text = Some(e.display_before);
            out.after.text = Some(e.display_after);
            Ok(out)
        }
        SeamVerb::PressKey => {
            let key = body.key.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "press_key requires key".into(),
                tip: "Pass {\"key\":\"4\"} or Return.".into(),
            })?;
            let e = app_press_key(app, &key)?;
            // Digits ride AT-SPI Action, everything else xdotool — the
            // effect does not say which; named honestly until it does.
            let mut out = VerbOutcome::new(
                TransportRung::R2Injection,
                "atspi-or-xdotool",
                e.effect_observed,
            )
            .extra("key", &e.key)
            .attr("key", key);
            out.app = Some(e.atspi_name);
            out.before.text = Some(e.display_before);
            out.after.text = Some(e.display_after);
            Ok(out)
        }
        SeamVerb::Select => {
            let name = body.name.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "select requires name".into(),
                tip: "Pass {\"name\":\"Basic\"}.".into(),
            })?;
            let e = app_select(app, &name)?;
            let mut out =
                VerbOutcome::new(TransportRung::R1Semantic, "atspi-action", e.effect_observed)
                    .extra("name", &e.name)
                    .attr("name", name);
            out.app = Some(e.atspi_name);
            out.before.selection = Some(e.state_before);
            out.after.selection = Some(e.state_after);
            Ok(out)
        }
        SeamVerb::Scroll => {
            let direction = body.direction.unwrap_or_else(|| "down".into());
            let delta = body.delta.unwrap_or(3);
            let e = app_scroll(app, &direction, delta)?;
            // The first live downgrade-visible verb: AT-SPI scroll action
            // when the pane offers one, wheel injection when it does not.
            let (rung, downgraded_from) = if e.via == "xdotool_wheel" {
                (TransportRung::R2Injection, Some(TransportRung::R1Semantic))
            } else {
                (TransportRung::R1Semantic, None)
            };
            let mut out = VerbOutcome::new(rung, &e.via, e.effect_observed)
                .extra("direction", &e.direction)
                .extra("delta", e.delta)
                .extra("via", &e.via)
                .attr("direction", direction)
                .attr("delta", delta);
            out.downgraded_from = downgraded_from;
            out.app = Some(e.atspi_name);
            out.before.text = Some(e.snapshot_before);
            out.after.text = Some(e.snapshot_after);
            Ok(out)
        }
        // Typed but not served on /verb: fabric verbs ride their own
        // endpoints, drag is honestly planned, extensions have no host
        // yet. The refusal names the verb, exactly as before.
        other => Err(dpl_coach_fabric::Refused {
            reason: format!("verb not implemented: {}", other.wire_name()),
            tip: "GET /verbs for shipped names. Use /import|/invoke for fabric verbs.".into(),
        }),
    }
}

/// `WindowVerbEffect.before/after: OuterGeom` → typed rect observables;
/// the window id (one window, both sides) rides `extra` (§4.5 table).
fn window_outcome(e: &dpl_coach_runtime::WindowVerbEffect) -> VerbOutcome {
    let mut out = VerbOutcome::new(TransportRung::R2Injection, "x11-wm", e.effect_observed)
        .extra("window_id", e.before.window_id);
    out.before.rect = Some(wire_geom(e.before));
    out.after.rect = Some(wire_geom(e.after));
    out
}

/// `LifecycleEffect.real_before/real_after` → typed running observables;
/// `pid` rides `extra` (§4.5 table).
fn lifecycle_outcome(e: &dpl_coach_runtime::LifecycleEffect, mechanism: &str) -> VerbOutcome {
    let mut out = VerbOutcome::new(TransportRung::R2Injection, mechanism, e.effect_observed);
    out.app = Some(e.atspi_name.clone());
    out.before.running = Some(e.real_before);
    out.after.running = Some(e.real_after);
    if let Some(pid) = e.pid {
        out = out.extra("pid", pid).attr("pid", pid);
    }
    out
}

/// `WaitAssertEffect`: `effect_observed` = matched, `running` = real,
/// `predicate`/`detail`/`pid` ride `extra` (§4.5 table).
fn wait_assert_outcome(e: &dpl_coach_runtime::WaitAssertEffect) -> VerbOutcome {
    let mut out = VerbOutcome::new(TransportRung::R1Semantic, "atspi-read", e.matched)
        .extra("predicate", &e.predicate);
    out.ok = e.ok;
    out.app = Some(e.atspi_name.clone());
    out.after.running = Some(e.real);
    if let Some(detail) = &e.detail {
        out = out.extra("detail", detail);
    }
    if let Some(pid) = e.pid {
        out = out.extra("pid", pid);
    }
    out
}

fn wire_geom(g: OuterGeom) -> WindowGeom {
    WindowGeom {
        window_id: g.window_id,
        x: g.x,
        y: g.y,
        w: g.w,
        h: g.h,
    }
}

async fn events_handler(
    Query(q): Query<EventsQuery>,
) -> Result<
    Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<ApiError>),
> {
    let app = q.app;
    let seconds = q.seconds.unwrap_or(20).clamp(1, 120);
    // Fail fast if not real — don't hang an SSE on a ghost.
    require_real_target(&app).map_err(map_refused)?;

    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    tokio::spawn(async move {
        // The stream speaks typed protocol facts since S2 — the same
        // `kind` names as before, now with one serialization owner.
        let fact = |name: &'static str, event: &seam::SeamEvent| {
            let data = serde_json::to_string(event).unwrap_or_else(|_| "{}".into());
            Ok(Event::default().event(name).data(data))
        };
        let mut last: Option<OuterGeom> = None;
        let end = tokio::time::Instant::now() + Duration::from_secs(seconds);
        let _ = tx
            .send(fact(
                "hello",
                &seam::SeamEvent::ObserveStart {
                    app: app.clone(),
                    seconds,
                },
            ))
            .await;
        while tokio::time::Instant::now() < end {
            let app_c = app.clone();
            let geom = tokio::task::spawn_blocking(move || window_geom(&app_c))
                .await
                .ok()
                .and_then(Result::ok);
            if let Some(g) = geom {
                if let Some(prev) = last {
                    let moved = g.x != prev.x || g.y != prev.y;
                    let resized = g.w != prev.w || g.h != prev.h;
                    if moved {
                        let event = seam::SeamEvent::Moved {
                            app: app.clone(),
                            before: wire_geom(prev),
                            after: wire_geom(g),
                            dx: g.x - prev.x,
                            dy: g.y - prev.y,
                        };
                        if tx.send(fact("moved", &event)).await.is_err() {
                            break;
                        }
                    }
                    if resized {
                        let event = seam::SeamEvent::Resized {
                            app: app.clone(),
                            before: wire_geom(prev),
                            after: wire_geom(g),
                            dw: g.w - prev.w,
                            dh: g.h - prev.h,
                        };
                        if tx.send(fact("resized", &event)).await.is_err() {
                            break;
                        }
                    }
                } else {
                    let event = seam::SeamEvent::Geom {
                        app: app.clone(),
                        geom: wire_geom(g),
                    };
                    if tx.send(fact("geom", &event)).await.is_err() {
                        break;
                    }
                }
                last = Some(g);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let _ = tx
            .send(fact(
                "bye",
                &seam::SeamEvent::ObserveEnd { app: app.clone() },
            ))
            .await;
    });

    let stream = ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn presence(Query(q): Query<PresenceQuery>) -> Json<Value> {
    let p = probe_presence(&q.app);
    Json(json!({
        "app": p.atspi_name,
        "atspi_listed": p.atspi_listed,
        "mapped_window": p.mapped_window,
        "window_id": p.window_id,
        "outer_w": p.outer_w,
        "outer_h": p.outer_h,
        "pid": p.pid,
        "pid_alive": p.pid_alive,
        "real": p.is_real(),
    }))
}

async fn inventory_handler() -> Result<Json<Value>, (StatusCode, Json<ApiError>)> {
    let items = inventory(true).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ApiError {
                refused: true,
                reason: e,
                tip: "Accessibility / desktop catalog unavailable.".into(),
            }),
        )
    })?;
    let rows: Vec<Value> = items
        .into_iter()
        .map(|i| {
            let atspi = i
                .live
                .as_ref()
                .map(|l| l.atspi_name.clone())
                .unwrap_or_else(|| i.exec_hint.clone());
            let presence = if atspi.is_empty() {
                None
            } else {
                Some(probe_presence(&atspi))
            };
            let real = presence.as_ref().is_some_and(|p| p.is_real());
            json!({
                "id": i.id.as_str(),
                "display_name": i.display_name,
                "complexity": i.complexity.as_str(),
                "atspi_live": i.live.is_some(),
                "operator_real": real,
                "presence": presence.map(|p| json!({
                    "mapped_window": p.mapped_window,
                    "pid": p.pid,
                    "pid_alive": p.pid_alive,
                    "outer_w": p.outer_w,
                    "outer_h": p.outer_h,
                })),
            })
        })
        .collect();
    Ok(Json(json!({ "apps": rows })))
}

async fn import_handler(
    State(state): State<AppState>,
    Json(body): Json<ImportBody>,
) -> Result<Json<Value>, (StatusCode, Json<ApiError>)> {
    let started = std::time::Instant::now();
    let requested = body.name.clone();
    let result = import_inner(&state, body).await;
    let duration_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok((value, app, agency, nodes)) => {
            state.signals.publish_seam(SeamEvent::Executed(SeamSpan {
                verb: "import".into(),
                app,
                rung: fabric_rung(TransportRung::R1Semantic),
                downgraded_from: None,
                mechanism: "atspi-tree-walk".into(),
                agency: seam_agency(agency),
                ok: true,
                effect_observed: nodes > 0,
                duration_ms,
                lesson: None,
                beat: None,
                attrs: vec![("nodes".into(), nodes.to_string())],
            }));
            Ok(Json(value))
        }
        Err(err) => {
            state.signals.publish_seam(SeamEvent::Refused {
                verb: "import".into(),
                app: requested,
                reason: err.1.reason.clone(),
                duration_ms,
            });
            Err(err)
        }
    }
}

async fn import_inner(
    state: &AppState,
    body: ImportBody,
) -> Result<(Value, String, AgencyMode, usize), (StatusCode, Json<ApiError>)> {
    let mut request = request_from_inventory_name(&body.name).map_err(map_refused)?;
    if let Some(seed) = body.seed {
        request.seed_name = Some(seed);
    }
    require_real_target(&request.atspi_name).map_err(map_refused)?;

    let agency = parse_agency(body.agency.as_deref());
    let outcome = tokio::task::spawn_blocking(move || mint(&request, None, |_| {}))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    refused: true,
                    reason: format!("mint join: {e}"),
                    tip: "Retry Import.".into(),
                }),
            )
        })?;

    let fabric = match outcome {
        MintOutcome::Ready(f) => f,
        MintOutcome::Refused(r) => return Err(map_refused(r)),
        MintOutcome::Cancelled => {
            return Err((
                StatusCode::CONFLICT,
                Json(ApiError {
                    refused: true,
                    reason: "mint cancelled".into(),
                    tip: "Retry Import.".into(),
                }),
            ));
        }
    };

    let sid = new_session_id();
    let session = FabricSession::new(fabric.clone(), agency);
    state
        .sessions
        .lock()
        .expect("sessions")
        .insert(sid.clone(), session);

    let value = json!({
        "ok": true,
        "session": sid,
        "agency": agency.as_str(),
        "nodes": fabric.nodes.len(),
        "minted_ms": fabric.minted_ms,
        "seed_path": fabric.seed_path,
        "atspi_name": fabric.atspi_name,
    });
    Ok((value, fabric.atspi_name.clone(), agency, fabric.nodes.len()))
}

async fn resolve_handler(
    State(state): State<AppState>,
    Json(body): Json<SessionBody>,
) -> Result<Json<Value>, (StatusCode, Json<ApiError>)> {
    let session = {
        let guard = state.sessions.lock().expect("sessions");
        guard
            .get(&body.session)
            .cloned()
            .ok_or_else(missing_session)?
    };
    let app = session.fabric.atspi_name.clone();
    let agency = session.agency;
    let started = std::time::Instant::now();
    let result = async {
        require_real_target(&session.fabric.atspi_name).map_err(map_refused)?;
        tokio::task::spawn_blocking(move || resolve_seed(&session))
            .await
            .map_err(join_err)?
            .map_err(map_refused)
    }
    .await;
    let duration_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(handle) => {
            let rect = extents(&handle).ok();
            state.signals.publish_seam(SeamEvent::Executed(SeamSpan {
                verb: "resolve".into(),
                app,
                rung: fabric_rung(TransportRung::R1Semantic),
                downgraded_from: None,
                mechanism: "atspi-read".into(),
                agency: seam_agency(agency),
                ok: true,
                effect_observed: true,
                duration_ms,
                lesson: None,
                beat: None,
                attrs: vec![("path".into(), handle.node.path.clone())],
            }));
            Ok(Json(json!({
                "ok": true,
                "path": handle.node.path,
                "name": handle.node.name,
                "role": handle.node.role,
                "actions": handle.node.actions,
                "window_rect": rect.map(|r| r.as_csv()),
            })))
        }
        Err(err) => {
            state.signals.publish_seam(SeamEvent::Refused {
                verb: "resolve".into(),
                app,
                reason: err.1.reason.clone(),
                duration_ms,
            });
            Err(err)
        }
    }
}

async fn invoke_handler(
    State(state): State<AppState>,
    Json(body): Json<SessionBody>,
) -> Result<Json<Value>, (StatusCode, Json<ApiError>)> {
    let session = {
        let guard = state.sessions.lock().expect("sessions");
        guard
            .get(&body.session)
            .cloned()
            .ok_or_else(missing_session)?
    };
    let action = body.action.unwrap_or_else(|| "click".into());
    let require_effect = body.require_effect.unwrap_or(true);
    let agency = session.agency;
    let app = session.fabric.atspi_name.clone();
    let action_c = action.clone();
    let started = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(move || {
        let handle = resolve_seed(&session)?;
        invoke_gated(&handle, &action_c, agency, require_effect)
    })
    .await
    .map_err(join_err)?;
    let duration_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(effect) => {
            state.signals.publish_seam(SeamEvent::Executed(SeamSpan {
                verb: "invoke".into(),
                app,
                rung: fabric_rung(TransportRung::R1Semantic),
                downgraded_from: None,
                mechanism: "atspi-action".into(),
                agency: seam_agency(agency),
                ok: true,
                effect_observed: effect.effect_observed,
                duration_ms,
                lesson: None,
                beat: None,
                attrs: vec![("action".into(), action.clone())],
            }));
            Ok(Json(json!({
                "ok": true,
                "action": effect.action,
                "agency": effect.agency.as_str(),
                "effect_observed": effect.effect_observed,
                "display_before": effect.display_before,
                "display_after": effect.display_after,
            })))
        }
        Err(r) => {
            state.signals.publish_seam(SeamEvent::Refused {
                verb: "invoke".into(),
                app,
                reason: r.reason.clone(),
                duration_ms,
            });
            Err(map_refused(r))
        }
    }
}

async fn mount_handler(
    State(state): State<AppState>,
    Json(body): Json<SessionBody>,
) -> Result<Json<Value>, (StatusCode, Json<ApiError>)> {
    let session = {
        let guard = state.sessions.lock().expect("sessions");
        guard
            .get(&body.session)
            .cloned()
            .ok_or_else(missing_session)?
    };
    let spawn = body.spawn_overlay.unwrap_or(false);
    let seconds = body.seconds.unwrap_or(12);
    let (spec, spawn_result) = tokio::task::spawn_blocking(
        move || -> Result<(MountSpec, Option<String>), dpl_coach_fabric::Refused> {
            let handle = resolve_seed(&session)?;
            let spec = mount_spec(&session, &handle)?;
            let msg = if spawn {
                Some(spawn_overlay_mount(&spec, seconds)?)
            } else {
                None
            };
            Ok((spec, msg))
        },
    )
    .await
    .map_err(join_err)?
    .map_err(map_refused)?;

    Ok(Json(json!({
        "ok": true,
        "atspi_name": spec.atspi_name,
        "expected_name": spec.expected_name,
        "window_rect": spec.window_rect.as_csv(),
        "overlay": spawn_result,
    })))
}

fn spawn_overlay_mount(
    spec: &MountSpec,
    seconds: u64,
) -> Result<String, dpl_coach_fabric::Refused> {
    let bin = overlay_bin();
    let rect = spec.window_rect.as_csv();
    let mut cmd = Command::new(&bin);
    dpl_proc::quiet(&mut cmd);
    cmd.args([
        "spotlight",
        "--app",
        &spec.atspi_name,
        "--name",
        &spec.expected_name,
        "--window-rect",
        &rect,
        "--seconds",
        &seconds.to_string(),
        "--message",
        &format!("Press {}", spec.expected_name),
    ]);
    match cmd.spawn() {
        Ok(_) => Ok(format!(
            "spawned {} with --window-rect {rect}",
            bin.display()
        )),
        Err(e) => Err(dpl_coach_fabric::Refused {
            reason: format!("overlay spawn failed: {e}"),
            tip: "Build dpl-coach-overlay beside the daemon or set DPL_COACH_OVERLAY.".into(),
        }),
    }
}

fn overlay_bin() -> PathBuf {
    if let Ok(p) = std::env::var("DPL_COACH_OVERLAY") {
        return PathBuf::from(p);
    }
    let name = if cfg!(windows) {
        "dpl-coach-overlay.exe"
    } else {
        "dpl-coach-overlay"
    };
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(name);
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from(name)
}

fn parse_agency(s: Option<&str>) -> AgencyMode {
    match s.map(|x| x.to_ascii_lowercase()).as_deref() {
        Some("teacher") => AgencyMode::Teacher,
        _ => AgencyMode::Coach,
    }
}

fn new_session_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("sess-{ms}")
}

fn map_refused(r: dpl_coach_fabric::Refused) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::CONFLICT,
        Json(ApiError {
            refused: true,
            reason: r.reason,
            tip: r.tip,
        }),
    )
}

fn missing_session() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            refused: true,
            reason: "unknown session".into(),
            tip: "POST /import first.".into(),
        }),
    )
}

fn join_err(e: tokio::task::JoinError) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            refused: true,
            reason: format!("worker: {e}"),
            tip: "Retry.".into(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The S2 plan §4.1 fallback contract: while `dpl-fabric` keeps its
    /// private `TransportRung` (its file is held by a parallel lane),
    /// this test holds the two enums to ONE wire vocabulary — a post-S2
    /// span row serializes the same `r0`..`r3` strings the S1-era
    /// Parquet rows carry, so old segments and new answer one query.
    /// Retire this test WITH the fabric copy in the follow-up slice.
    #[test]
    fn protocol_and_fabric_rung_labels_are_one_vocabulary() {
        for rung in [
            TransportRung::R0Native,
            TransportRung::R1Semantic,
            TransportRung::R2Injection,
            TransportRung::R3Vision,
        ] {
            assert_eq!(rung.label(), fabric_rung(rung).label(), "{rung:?}");
        }
    }

    /// `/verbs` derives from the protocol catalog, and every listed
    /// name parses back — introspection can never under-claim what the
    /// edge accepts (the `geom` lesson, S2 plan §1 correction 3).
    #[test]
    fn every_cataloged_verb_parses_at_the_http_edge() {
        for d in seam::seam_verb_catalog() {
            assert!(SeamVerb::parse(d.verb).is_ok(), "{}", d.verb);
        }
        assert!(SeamVerb::parse("window_geom").is_ok(), "shipped alias");
        assert!(SeamVerb::parse("teleport").is_err());
    }
}
