//! dpl-coach-daemon — localhost Axum Coach control plane.
//!
//! Agent hop: HTTP on 127.0.0.1. App hop: AT-SPI / X11 via dpl-coach-runtime.
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
    resolve_seed, verb_catalog, window_focus, window_geom, window_move, window_resize, AgencyMode,
    FabricSession, MountSpec, OuterGeom,
};
use dpl_fabric::{Fabric as SignalFabric, SeamAgency, SeamEvent, SeamSpan, TransportRung};
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

async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "dpl-coach-daemon",
        "bind": "127.0.0.1",
        "adr": "docs/plans/coach-control-plane-adr.md",
        "verbs_adr": "docs/plans/coach-operator-verb-catalog-adr.md"
    }))
}

async fn verbs_handler() -> Json<Value> {
    let verbs: Vec<Value> = verb_catalog()
        .iter()
        .map(|v| {
            json!({
                "name": v.name,
                "status": v.status,
                "target": v.target,
            })
        })
        .collect();
    Json(json!({ "verbs": verbs }))
}

/// What one verb execution produced, beside its span facts (plan §4:
/// the rung table is v1 static; `mechanism` names the honest means;
/// scroll resolves its rung from the effect's `via` at runtime).
struct VerbOutcome {
    value: Value,
    rung: TransportRung,
    downgraded_from: Option<TransportRung>,
    mechanism: String,
    effect_observed: bool,
    attrs: Vec<(String, String)>,
}

impl VerbOutcome {
    fn new(value: Value, rung: TransportRung, mechanism: &str, effect_observed: bool) -> Self {
        Self {
            value,
            rung,
            downgraded_from: None,
            mechanism: mechanism.into(),
            effect_observed,
            attrs: Vec::new(),
        }
    }

    fn attr(mut self, key: &str, value: impl ToString) -> Self {
        self.attrs.push((key.into(), value.to_string()));
        self
    }
}

async fn verb_handler(
    State(state): State<AppState>,
    Json(body): Json<VerbBody>,
) -> Result<Json<Value>, (StatusCode, Json<ApiError>)> {
    let verb = body.verb.to_ascii_lowercase();
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
    let (verb_c, app_c) = (verb.clone(), app.clone());
    let result = tokio::task::spawn_blocking(move || execute_verb(&verb_c, &app_c, body))
        .await
        .map_err(join_err)?;
    let duration_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(outcome) => {
            state.signals.publish_seam(SeamEvent::Executed(SeamSpan {
                verb,
                app,
                rung: outcome.rung,
                downgraded_from: outcome.downgraded_from,
                mechanism: outcome.mechanism,
                agency: SeamAgency::Teacher,
                ok: true,
                effect_observed: outcome.effect_observed,
                duration_ms,
                lesson: None,
                beat: None,
                attrs: outcome.attrs,
            }));
            Ok(Json(outcome.value))
        }
        Err(r) => {
            state.signals.publish_seam(SeamEvent::Refused {
                verb,
                app,
                reason: r.reason.clone(),
                duration_ms,
            });
            Err(map_refused(r))
        }
    }
}

fn execute_verb(
    verb: &str,
    app: &str,
    body: VerbBody,
) -> Result<VerbOutcome, dpl_coach_fabric::Refused> {
    let timeout_ms = body.timeout_ms.unwrap_or(8_000);
    match verb {
        "move" => {
            let x = body.x.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "move requires x".into(),
                tip: "Pass x and y.".into(),
            })?;
            let y = body.y.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "move requires y".into(),
                tip: "Pass x and y.".into(),
            })?;
            let e = window_move(app, x, y)?;
            Ok(VerbOutcome::new(
                verb_effect_json(&e),
                TransportRung::R2Injection,
                "x11-wm",
                e.effect_observed,
            )
            .attr("x", x)
            .attr("y", y))
        }
        "resize" => {
            let w = body.w.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "resize requires w".into(),
                tip: "Pass w and h (≥ 50).".into(),
            })?;
            let h = body.h.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "resize requires h".into(),
                tip: "Pass w and h (≥ 50).".into(),
            })?;
            let e = window_resize(app, w, h)?;
            Ok(VerbOutcome::new(
                verb_effect_json(&e),
                TransportRung::R2Injection,
                "x11-wm",
                e.effect_observed,
            )
            .attr("w", w)
            .attr("h", h))
        }
        "focus" => {
            let e = window_focus(app)?;
            Ok(VerbOutcome::new(
                verb_effect_json(&e),
                TransportRung::R2Injection,
                "x11-wm",
                e.effect_observed,
            ))
        }
        "geom" | "window_geom" => {
            let g = window_geom(app)?;
            let e = dpl_coach_runtime::WindowVerbEffect {
                verb: "geom".into(),
                effect_observed: true,
                before: g,
                after: g,
            };
            Ok(VerbOutcome::new(
                verb_effect_json(&e),
                TransportRung::R2Injection,
                "x11-wm",
                true,
            ))
        }
        "open" => {
            let e = app_open(app, timeout_ms)?;
            let value = lifecycle_json(&e);
            let mut out = VerbOutcome::new(
                value,
                TransportRung::R2Injection,
                "spawn",
                e.effect_observed,
            );
            if let Some(pid) = e.pid {
                out = out.attr("pid", pid);
            }
            Ok(out)
        }
        "close" => {
            let e = app_close(app, timeout_ms)?;
            let value = lifecycle_json(&e);
            let mut out = VerbOutcome::new(
                value,
                TransportRung::R2Injection,
                "wm-delete",
                e.effect_observed,
            );
            if let Some(pid) = e.pid {
                out = out.attr("pid", pid);
            }
            Ok(out)
        }
        "wait_until" => {
            let pred = body.predicate.as_deref().unwrap_or("real");
            let e = app_wait_until(app, pred, timeout_ms)?;
            let matched = e.matched;
            Ok(VerbOutcome::new(
                wait_assert_json(&e),
                TransportRung::R1Semantic,
                "atspi-read",
                matched,
            )
            .attr("predicate", pred))
        }
        "assert_state" => {
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
            let matched = e.matched;
            Ok(VerbOutcome::new(
                wait_assert_json(&e),
                TransportRung::R1Semantic,
                "atspi-read",
                matched,
            )
            .attr("expect", expect))
        }
        "type_text" => {
            let text = body.text.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "type_text requires text".into(),
                tip: "Pass {\"text\":\"3\"}.".into(),
            })?;
            let e = app_type_text(app, &text)?;
            let value = json!({
                "ok": true,
                "verb": e.verb,
                "effect_observed": e.effect_observed,
                "atspi_name": e.atspi_name,
                "text": e.text,
                "display_before": e.display_before,
                "display_after": e.display_after,
            });
            Ok(VerbOutcome::new(
                value,
                TransportRung::R2Injection,
                "xdotool",
                e.effect_observed,
            )
            .attr("text_len", text.len()))
        }
        "press_key" => {
            let key = body.key.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "press_key requires key".into(),
                tip: "Pass {\"key\":\"4\"} or Return.".into(),
            })?;
            let e = app_press_key(app, &key)?;
            let value = json!({
                "ok": true,
                "verb": e.verb,
                "effect_observed": e.effect_observed,
                "atspi_name": e.atspi_name,
                "key": e.key,
                "display_before": e.display_before,
                "display_after": e.display_after,
            });
            // Digits ride AT-SPI Action, everything else xdotool — the
            // effect does not say which; named honestly until it does.
            Ok(VerbOutcome::new(
                value,
                TransportRung::R2Injection,
                "atspi-or-xdotool",
                e.effect_observed,
            )
            .attr("key", key))
        }
        "select" => {
            let name = body.name.ok_or_else(|| dpl_coach_fabric::Refused {
                reason: "select requires name".into(),
                tip: "Pass {\"name\":\"Basic\"}.".into(),
            })?;
            let e = app_select(app, &name)?;
            let value = json!({
                "ok": true,
                "verb": e.verb,
                "effect_observed": e.effect_observed,
                "atspi_name": e.atspi_name,
                "name": e.name,
                "state_before": e.state_before,
                "state_after": e.state_after,
            });
            Ok(VerbOutcome::new(
                value,
                TransportRung::R1Semantic,
                "atspi-action",
                e.effect_observed,
            )
            .attr("name", name))
        }
        "scroll" => {
            let direction = body.direction.unwrap_or_else(|| "down".into());
            let delta = body.delta.unwrap_or(3);
            let e = app_scroll(app, &direction, delta)?;
            let value = json!({
                "ok": true,
                "verb": e.verb,
                "effect_observed": e.effect_observed,
                "atspi_name": e.atspi_name,
                "direction": e.direction,
                "delta": e.delta,
                "via": e.via,
                "snapshot_before": e.snapshot_before,
                "snapshot_after": e.snapshot_after,
            });
            // The first live downgrade-visible verb: AT-SPI scroll action
            // when the pane offers one, wheel injection when it does not.
            let (rung, downgraded_from) = if e.via == "xdotool_wheel" {
                (TransportRung::R2Injection, Some(TransportRung::R1Semantic))
            } else {
                (TransportRung::R1Semantic, None)
            };
            let mut out = VerbOutcome::new(value, rung, &e.via, e.effect_observed)
                .attr("direction", direction)
                .attr("delta", delta);
            out.downgraded_from = downgraded_from;
            Ok(out)
        }
        other => Err(dpl_coach_fabric::Refused {
            reason: format!("verb not implemented: {other}"),
            tip: "GET /verbs for shipped names. Use /import|/invoke for fabric verbs.".into(),
        }),
    }
}

fn lifecycle_json(e: &dpl_coach_runtime::LifecycleEffect) -> Value {
    json!({
        "ok": true,
        "verb": e.verb,
        "effect_observed": e.effect_observed,
        "atspi_name": e.atspi_name,
        "real_before": e.real_before,
        "real_after": e.real_after,
        "pid": e.pid,
    })
}

fn wait_assert_json(e: &dpl_coach_runtime::WaitAssertEffect) -> Value {
    json!({
        "ok": e.ok,
        "verb": e.verb,
        "matched": e.matched,
        "predicate": e.predicate,
        "atspi_name": e.atspi_name,
        "real": e.real,
        "pid": e.pid,
        "detail": e.detail,
    })
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
        let mut last: Option<OuterGeom> = None;
        let end = tokio::time::Instant::now() + Duration::from_secs(seconds);
        let _ = tx
            .send(Ok(Event::default().event("hello").data(
                json!({
                    "kind": "observe_start",
                    "app": app,
                    "seconds": seconds,
                })
                .to_string(),
            )))
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
                        let payload = json!({
                            "kind": "moved",
                            "app": app,
                            "before": geom_json(prev),
                            "after": geom_json(g),
                            "dx": g.x - prev.x,
                            "dy": g.y - prev.y,
                        });
                        if tx
                            .send(Ok(Event::default()
                                .event("moved")
                                .data(payload.to_string())))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    if resized {
                        let payload = json!({
                            "kind": "resized",
                            "app": app,
                            "before": geom_json(prev),
                            "after": geom_json(g),
                            "dw": g.w - prev.w,
                            "dh": g.h - prev.h,
                        });
                        if tx
                            .send(Ok(Event::default()
                                .event("resized")
                                .data(payload.to_string())))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                } else {
                    let payload = json!({
                        "kind": "geom",
                        "app": app,
                        "geom": geom_json(g),
                    });
                    if tx
                        .send(Ok(Event::default().event("geom").data(payload.to_string())))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                last = Some(g);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let _ = tx
            .send(Ok(Event::default().event("bye").data(
                json!({ "kind": "observe_end", "app": app }).to_string(),
            )))
            .await;
    });

    let stream = ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn geom_json(g: OuterGeom) -> Value {
    json!({
        "window_id": g.window_id,
        "x": g.x,
        "y": g.y,
        "w": g.w,
        "h": g.h,
        "csv": g.as_csv(),
    })
}

fn verb_effect_json(e: &dpl_coach_runtime::WindowVerbEffect) -> Value {
    json!({
        "ok": true,
        "verb": e.verb,
        "effect_observed": e.effect_observed,
        "before": geom_json(e.before),
        "after": geom_json(e.after),
    })
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
                rung: TransportRung::R1Semantic,
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
                rung: TransportRung::R1Semantic,
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
                rung: TransportRung::R1Semantic,
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
