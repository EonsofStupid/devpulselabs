//! The AGENT SEAM (R1v) — axum signals to HTTP endpoints.
//!
//! A localhost-only HTTP server bridging the typed `Fabric` (tokio
//! broadcast) ↔ HTTP, so chat seats (Claude / Codex / Grok) can observe and
//! drive the app: `GET /state` (where we are + the whole route registry),
//! `POST /command` (validated asks — an unknown route is refused BY NAME,
//! doctor-style, never invented), `GET /signals` (every shell event as an
//! SSE stream). The server runs on its own thread with its own runtime —
//! the egui thread is never touched. The fabric stays serde-free by design;
//! every wire shape lives HERE.
//!
//! Security boundary: the bind is `127.0.0.1` ONLY. Comp-document mutation
//! endpoints ride the doc doctors in a later pass (R1f) — this pass is the
//! signal seam.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use dpl_fabric::{Fabric, ShellEvent};
use dpl_hub::{HubSpec, Route};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

/// The route REGISTRY — the only door from owned HTTP strings into
/// `'static` routes. Built from the same `HubSpec`s the shell registers,
/// so the seam can never name a surface the app doesn't have.
pub struct Registry {
    hubs: Vec<&'static HubSpec>,
}

impl Registry {
    pub fn new(hubs: Vec<&'static HubSpec>) -> Self {
        Self { hubs }
    }

    /// `"hub/sub"` → a real `Route`, or a refusal naming exactly what is
    /// unknown (the doctor law: refuse by name, never invent).
    pub fn resolve(&self, path: &str) -> Result<Route, String> {
        let Some((h, s)) = path.split_once('/') else {
            return Err(format!("route must be \"hub/sub\", got: {path}"));
        };
        let Some(spec) = self.hubs.iter().find(|x| x.id.0 == h) else {
            return Err(format!("unknown hub: {h}"));
        };
        let Some(sub) = spec.subroutes.iter().find(|x| x.id.0 == s) else {
            return Err(format!("unknown subroute: {h}/{s}"));
        };
        Ok(Route {
            hub: spec.id,
            sub: sub.id,
        })
    }

    /// The registry as wire JSON — what an agent reads to learn the map.
    fn hubs_json(&self) -> Value {
        Value::Array(
            self.hubs
                .iter()
                .map(|spec| {
                    json!({
                        "id": spec.id.0,
                        "label": spec.label,
                        "subs": spec.subroutes.iter().map(|s| json!({
                            "id": s.id.0,
                            "label": s.label,
                            "blurb": s.blurb,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect(),
        )
    }
}

/// `Route` on the wire: the same `"hub/sub"` string the CLI `--route`
/// flag speaks — one route grammar everywhere.
fn route_str(r: &Route) -> String {
    format!("{}/{}", r.hub.0, r.sub.0)
}

/// One shell event as wire JSON. Every variant is covered — a new fabric
/// variant fails compilation here instead of silently vanishing from SSE.
pub fn event_json(ev: &ShellEvent) -> Value {
    match ev {
        ShellEvent::RouteChanged { route } => {
            json!({"type": "route-changed", "route": route_str(route)})
        }
        ShellEvent::ViziwizSummon => json!({"type": "viziwiz-summon"}),
        ShellEvent::FullBleedRequest(on) => json!({"type": "full-bleed", "on": on}),
        ShellEvent::NavigateRequest { route } => {
            json!({"type": "navigate-request", "route": route_str(route)})
        }
        ShellEvent::WindowRequest { target, .. } => {
            json!({"type": "window-request", "target": target.0})
        }
    }
}

/// The seam's shared state: the one fabric, the registry, and the last
/// route the seam HEARD arrive (RouteChanged is the truth of arrival).
pub struct SeamState {
    fabric: Fabric,
    registry: Registry,
    last_route: Mutex<Option<Route>>,
}

type Shared = Arc<SeamState>;

/// The three endpoints on one router — also the unit-test seam (tower
/// oneshot drives this without a socket).
pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/state", get(get_state))
        .route("/command", post(post_command))
        .route("/signals", get(get_signals))
        .with_state(state)
}

/// Build the shared state (public so tests and the app share one path).
pub fn seam_state(fabric: Fabric, registry: Registry) -> Shared {
    Arc::new(SeamState {
        fabric,
        registry,
        last_route: Mutex::new(None),
    })
}

async fn get_state(State(s): State<Shared>) -> Json<Value> {
    let route = s
        .last_route
        .lock()
        .expect("seam route")
        .as_ref()
        .map(route_str);
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "route": route,
        "hubs": s.registry.hubs_json(),
    }))
}

/// The validated command door. Refusals are 422 with the issues NAMED —
/// an agent can read exactly why and correct itself.
fn handle_command(s: &SeamState, body: &Value) -> Result<(), Vec<String>> {
    match body.get("cmd").and_then(|c| c.as_str()) {
        Some("navigate") => {
            let Some(path) = body.get("route").and_then(|r| r.as_str()) else {
                return Err(vec!["navigate needs a \"route\" string".into()]);
            };
            let route = s.registry.resolve(path).map_err(|e| vec![e])?;
            s.fabric
                .publish_shell(ShellEvent::NavigateRequest { route });
            Ok(())
        }
        Some("viziwiz-summon") => {
            s.fabric.publish_shell(ShellEvent::ViziwizSummon);
            Ok(())
        }
        Some(other) => Err(vec![format!("unknown cmd: {other}")]),
        None => Err(vec!["missing \"cmd\"".into()]),
    }
}

async fn post_command(
    State(s): State<Shared>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match handle_command(&s, &body) {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true}))),
        Err(issues) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"ok": false, "issues": issues})),
        ),
    }
}

async fn get_signals(
    State(s): State<Shared>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = s.fabric.subscribe_shell();
    let stream = BroadcastStream::new(rx).filter_map(|ev| {
        // Lagged receivers skip (signals are lossy by design); closed ends.
        ev.ok()
            .map(|ev| Ok(Event::default().data(event_json(&ev).to_string())))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Boot the seam on its own named thread + runtime. A failed bind logs and
/// ends the thread — the app never dies for a busy port.
pub fn spawn(fabric: Fabric, registry: Registry, port: u16) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("dpl-seam".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("seam runtime");
            rt.block_on(async move {
                let state = seam_state(fabric, registry);
                // Fold arrivals into last_route so GET /state answers
                // "where is the operator" without asking the UI thread.
                let fold = Arc::clone(&state);
                let mut rx = fold.fabric.subscribe_shell();
                tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(ShellEvent::RouteChanged { route }) => {
                                *fold.last_route.lock().expect("seam route") = Some(route);
                            }
                            Ok(_) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                });
                let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!(%e, port, "agent seam bind failed — seam disabled");
                        return;
                    }
                };
                tracing::info!(port, "agent seam listening on 127.0.0.1");
                if let Err(e) = axum::serve(listener, router(state)).await {
                    tracing::error!(%e, "agent seam server exited");
                }
            });
        })
        .expect("spawn seam thread")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use dpl_hub::{HubId, SubId, SubRoute};
    use tower::util::ServiceExt;

    static SPEC: HubSpec = HubSpec {
        id: HubId("wiredfront"),
        label: "wiredFRONT",
        icon: "•",
        tone: dpl_coldlight::Tone::Magenta,
        subroutes: &[
            SubRoute {
                id: SubId("editor"),
                label: "Editor",
                blurb: "the editor",
            },
            SubRoute {
                id: SubId("library"),
                label: "Library",
                blurb: "the shelf",
            },
        ],
    };

    fn test_state() -> (Fabric, Shared) {
        let fabric = Fabric::new();
        let state = seam_state(fabric.clone(), Registry::new(vec![&SPEC]));
        (fabric, state)
    }

    fn post_json(uri: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request")
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body");
        serde_json::from_slice(&bytes).expect("json")
    }

    #[test]
    fn the_resolver_accepts_registered_and_refuses_unknown_by_name() {
        let reg = Registry::new(vec![&SPEC]);
        let route = reg.resolve("wiredfront/editor").expect("known route");
        assert_eq!(route.hub.0, "wiredfront");
        assert_eq!(route.sub.0, "editor");
        assert!(reg.resolve("nope/editor").unwrap_err().contains("nope"));
        assert!(reg
            .resolve("wiredfront/nowhere")
            .unwrap_err()
            .contains("wiredfront/nowhere"));
        assert!(reg.resolve("slashless").unwrap_err().contains("hub/sub"));
    }

    #[tokio::test]
    async fn a_valid_navigate_publishes_onto_the_real_fabric() {
        let (fabric, state) = test_state();
        let mut rx = fabric.subscribe_shell();
        let resp = router(state)
            .oneshot(post_json(
                "/command",
                json!({"cmd": "navigate", "route": "wiredfront/library"}),
            ))
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
        match rx.try_recv().expect("published") {
            ShellEvent::NavigateRequest { route } => assert_eq!(route.sub.0, "library"),
            other => panic!("wrong event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_unknown_route_is_refused_422_and_publishes_nothing() {
        let (fabric, state) = test_state();
        let mut rx = fabric.subscribe_shell();
        let resp = router(state)
            .oneshot(post_json(
                "/command",
                json!({"cmd": "navigate", "route": "nope/nowhere"}),
            ))
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_json(resp).await;
        assert_eq!(body["ok"], json!(false));
        assert!(body["issues"][0].as_str().expect("issue").contains("nope"));
        assert!(rx.try_recv().is_err(), "a refusal must publish nothing");
    }

    #[tokio::test]
    async fn state_carries_the_registry_and_the_heard_route() {
        let (fabric, state) = test_state();
        // Nothing heard yet → route is null.
        let resp = router(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .uri("/state")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let body = body_json(resp).await;
        assert_eq!(body["route"], Value::Null);
        assert_eq!(body["hubs"][0]["id"], json!("wiredfront"));
        assert_eq!(body["hubs"][0]["subs"][1]["id"], json!("library"));
        // Hearing an arrival fills it (the fold the spawn task performs).
        let route = state.registry.resolve("wiredfront/editor").expect("route");
        *state.last_route.lock().expect("route") = Some(route);
        let _ = fabric; // the fabric outlives the state in the app
        let resp = router(state)
            .oneshot(
                Request::builder()
                    .uri("/state")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(body_json(resp).await["route"], json!("wiredfront/editor"));
    }

    #[test]
    fn every_event_variant_has_a_pinned_wire_shape() {
        let route = Route {
            hub: HubId("wiredfront"),
            sub: SubId("editor"),
        };
        assert_eq!(
            event_json(&ShellEvent::RouteChanged { route }),
            json!({"type": "route-changed", "route": "wiredfront/editor"})
        );
        assert_eq!(
            event_json(&ShellEvent::NavigateRequest { route }),
            json!({"type": "navigate-request", "route": "wiredfront/editor"})
        );
        assert_eq!(
            event_json(&ShellEvent::ViziwizSummon),
            json!({"type": "viziwiz-summon"})
        );
        assert_eq!(
            event_json(&ShellEvent::FullBleedRequest(true)),
            json!({"type": "full-bleed", "on": true})
        );
        assert_eq!(
            event_json(&ShellEvent::WindowRequest {
                target: dpl_fabric::AppWindowKey("chat.primary"),
                action: dpl_fabric::AppWindowAction::OpenOrFocus,
            }),
            json!({"type": "window-request", "target": "chat.primary"})
        );
    }
}
