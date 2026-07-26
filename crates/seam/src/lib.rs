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
//! R1w adds the COMP-DOCUMENT surface: `GET /comps` (the inbox census),
//! `GET /comp?dir=` (document + live doctor report), `POST /comp` (the
//! DOCTOR-GATED write — an invalid comp is refused with every issue named
//! and nothing touches disk), and `POST /command {"cmd":"render"}` +
//! `GET /render?dir=` (the ffmpeg stitch, off-thread, polled). Every rel
//! dir is JAILED inside the content root; renders only ever write the NEW
//! `render.mp4` — sources are never mutated.
//!
//! Security boundary: the bind is `127.0.0.1` ONLY.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use dpl_comp::Composition;
use dpl_fabric::{Fabric, ShellEvent};
use dpl_hub::{HubSpec, Route};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
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

/// One render job's polled status (mirrors the editor's `RenderState`).
enum RenderStatus {
    Running,
    Done,
    Failed(String),
}

/// The seam's shared state: the one fabric, the registry, the last route
/// the seam HEARD arrive (RouteChanged is the truth of arrival), the
/// content root every rel dir is JAILED inside, and the render jobs.
pub struct SeamState {
    fabric: Fabric,
    registry: Registry,
    last_route: Mutex<Option<Route>>,
    root: PathBuf,
    renders: Mutex<HashMap<String, RenderStatus>>,
}

type Shared = Arc<SeamState>;

/// The dir JAIL: a forward-slash rel path resolves component-wise inside
/// the content root, or is refused by name — `..`, absolute paths, and
/// drive-letter tricks never escape (the index dump-jail's spirit).
fn jail(root: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.is_empty() {
        return Err("dir must not be empty".into());
    }
    let mut out = root.to_path_buf();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." || seg.contains(['\\', ':']) {
            return Err(format!("dir escapes the content root: {rel}"));
        }
        out.push(seg);
    }
    Ok(out)
}

/// The first free `inbox/comp-N` rel dir — the same convention the
/// editor's Save mints, so agent-born and operator-born comps colocate.
fn fresh_comp_rel(root: &Path) -> String {
    (1..)
        .map(|n| format!("comp-{n}"))
        .find(|name| !root.join("inbox").join(name).exists())
        .map(|name| format!("inbox/{name}"))
        .expect("unbounded")
}

/// The three endpoints on one router — also the unit-test seam (tower
/// oneshot drives this without a socket).
pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/state", get(get_state))
        .route("/command", post(post_command))
        .route("/signals", get(get_signals))
        .route("/comps", get(get_comps))
        .route("/comp", get(get_comp).post(post_comp))
        .route("/render", get(get_render))
        .with_state(state)
}

/// Build the shared state (public so tests and the app share one path).
/// `root` is the content root every rel dir is jailed inside.
pub fn seam_state(fabric: Fabric, registry: Registry, root: PathBuf) -> Shared {
    Arc::new(SeamState {
        fabric,
        registry,
        last_route: Mutex::new(None),
        root,
        renders: Mutex::new(HashMap::new()),
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
/// an agent can read exactly why and correct itself. Success carries the
/// status + any extra payload fields.
fn handle_command(s: &Shared, body: &Value) -> Result<(StatusCode, Value), Vec<String>> {
    match body.get("cmd").and_then(|c| c.as_str()) {
        Some("navigate") => {
            let Some(path) = body.get("route").and_then(|r| r.as_str()) else {
                return Err(vec!["navigate needs a \"route\" string".into()]);
            };
            let route = s.registry.resolve(path).map_err(|e| vec![e])?;
            s.fabric
                .publish_shell(ShellEvent::NavigateRequest { route });
            Ok((StatusCode::OK, json!({})))
        }
        Some("viziwiz-summon") => {
            s.fabric.publish_shell(ShellEvent::ViziwizSummon);
            Ok((StatusCode::OK, json!({})))
        }
        Some("render") => start_render(s, body),
        Some(other) => Err(vec![format!("unknown cmd: {other}")]),
        None => Err(vec!["missing \"cmd\"".into()]),
    }
}

/// Kick the ffmpeg stitch render OFF-thread (the editor's Render chip,
/// over HTTP): the comp must load AND pass the doctor, every clip rel must
/// stay in jail, one render per comp at a time. Output is always the NEW
/// `<dir>/render.mp4` — sources are never mutated.
fn start_render(s: &Shared, body: &Value) -> Result<(StatusCode, Value), Vec<String>> {
    let Some(rel) = body.get("dir").and_then(|r| r.as_str()) else {
        return Err(vec!["render needs a \"dir\" string".into()]);
    };
    let dir = jail(&s.root, rel).map_err(|e| vec![e])?;
    let comp = dpl_comp::load(&dir).map_err(|e| vec![format!("load {rel}: {e}")])?;
    let issues = dpl_comp::validate(&comp);
    if !issues.is_empty() {
        return Err(issues
            .iter()
            .map(|i| format!("{}: {}", i.path, i.message))
            .collect());
    }
    let mut clips = Vec::new();
    for layer in comp.tracks.iter().flat_map(|t| t.layers.iter()) {
        if let dpl_comp::Source::Artifact { rel: clip_rel } = &layer.source {
            let path = jail(&s.root, clip_rel).map_err(|e| vec![e])?;
            clips.push(dpl_engine_media::StitchClip {
                still: dpl_engine_media::is_still(&path),
                path,
                in_pt: layer.in_pt,
                out_pt: layer.out_pt,
            });
        }
    }
    if clips.is_empty() {
        return Err(vec![format!("no playable clips to render in {rel}")]);
    }
    {
        let mut renders = s.renders.lock().expect("seam renders");
        if matches!(renders.get(rel), Some(RenderStatus::Running)) {
            return Err(vec![format!("render already running for {rel}")]);
        }
        renders.insert(rel.to_string(), RenderStatus::Running);
    }
    let out = dir.join("render.mp4");
    let (w, h, fps) = (comp.w, comp.h, comp.fps);
    let state = Arc::clone(s);
    let key = rel.to_string();
    std::thread::Builder::new()
        .name("dpl-seam-render".into())
        .spawn(move || {
            let result = dpl_engine_media::render_stitch(&clips, w, h, fps, &out);
            let status = match result {
                Ok(()) => RenderStatus::Done,
                Err(e) => RenderStatus::Failed(e),
            };
            state
                .renders
                .lock()
                .expect("seam renders")
                .insert(key, status);
        })
        .expect("spawn seam render thread");
    Ok((
        StatusCode::ACCEPTED,
        json!({"out": format!("{rel}/render.mp4")}),
    ))
}

async fn post_command(
    State(s): State<Shared>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    match handle_command(&s, &body) {
        Ok((code, extra)) => {
            let mut v = json!({"ok": true});
            if let (Some(obj), Some(extra)) = (v.as_object_mut(), extra.as_object()) {
                for (k, val) in extra {
                    obj.insert(k.clone(), val.clone());
                }
            }
            (code, Json(v))
        }
        Err(issues) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"ok": false, "issues": issues})),
        ),
    }
}

/// Every `inbox/*/comp.json` under the root — unreadable ones NAMED with
/// their error, never hidden.
async fn get_comps(State(s): State<Shared>) -> Json<Value> {
    let inbox = s.root.join("inbox");
    let mut names: Vec<String> = std::fs::read_dir(&inbox)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    let mut rows = Vec::new();
    for name in names {
        let dir = inbox.join(&name);
        if !dir.join("comp.json").is_file() {
            continue;
        }
        let rel = format!("inbox/{name}");
        match dpl_comp::load(&dir) {
            Ok(c) => rows.push(json!({
                "dir": rel,
                "clips": c.tracks.iter().map(|t| t.layers.len()).sum::<usize>(),
                "seconds": c.duration(),
            })),
            Err(e) => rows.push(json!({"dir": rel, "error": e.to_string()})),
        }
    }
    Json(json!({"comps": rows}))
}

/// The doctor's report as wire strings.
fn issue_strings(comp: &Composition) -> Vec<String> {
    dpl_comp::validate(comp)
        .iter()
        .map(|i| format!("{}: {}", i.path, i.message))
        .collect()
}

async fn get_comp(
    State(s): State<Shared>,
    Query(q): Query<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let Some(rel) = q.get("dir") else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"ok": false, "issues": ["missing ?dir="]})),
        );
    };
    let dir = match jail(&s.root, rel) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"ok": false, "issues": [e]})),
            )
        }
    };
    match dpl_comp::load(&dir) {
        Ok(comp) => {
            let issues = issue_strings(&comp);
            let comp_json = serde_json::to_value(&comp).expect("comp json");
            (
                StatusCode::OK,
                Json(json!({"dir": rel, "comp": comp_json, "issues": issues})),
            )
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "issues": [format!("load {rel}: {e}")]})),
        ),
    }
}

/// The DOCTOR-GATED write: an invalid comp is refused with every issue
/// named and NOTHING touches disk. An absent dir mints the first free
/// `inbox/comp-N` (the editor Save's own convention).
async fn post_comp(State(s): State<Shared>, Json(body): Json<Value>) -> (StatusCode, Json<Value>) {
    let Some(comp_v) = body.get("comp") else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"ok": false, "issues": ["missing \"comp\""]})),
        );
    };
    let comp: Composition = match serde_json::from_value(comp_v.clone()) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"ok": false, "issues": [format!("comp does not parse: {e}")]})),
            )
        }
    };
    let issues = issue_strings(&comp);
    if !issues.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"ok": false, "issues": issues})),
        );
    }
    let rel = body
        .get("dir")
        .and_then(|d| d.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| fresh_comp_rel(&s.root));
    let dir = match jail(&s.root, &rel) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"ok": false, "issues": [e]})),
            )
        }
    };
    match dpl_comp::save(&dir, &comp) {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true, "dir": rel}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "issues": [format!("save {rel}: {e}")]})),
        ),
    }
}

async fn get_render(
    State(s): State<Shared>,
    Query(q): Query<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let Some(rel) = q.get("dir") else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"ok": false, "issues": ["missing ?dir="]})),
        );
    };
    let body = match s.renders.lock().expect("seam renders").get(rel.as_str()) {
        None => json!({"dir": rel, "state": "idle"}),
        Some(RenderStatus::Running) => json!({"dir": rel, "state": "running"}),
        Some(RenderStatus::Done) => {
            json!({"dir": rel, "state": "done", "out": format!("{rel}/render.mp4")})
        }
        Some(RenderStatus::Failed(e)) => json!({"dir": rel, "state": "failed", "error": e}),
    };
    (StatusCode::OK, Json(body))
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
pub fn spawn(
    fabric: Fabric,
    registry: Registry,
    root: PathBuf,
    port: u16,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("dpl-seam".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("seam runtime");
            rt.block_on(async move {
                let state = seam_state(fabric, registry, root);
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

    /// A fresh throwaway content root per test tag — parallel-safe.
    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-seam-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("inbox")).expect("temp root");
        dir
    }

    fn test_state_at(root: PathBuf) -> (Fabric, Shared) {
        let fabric = Fabric::new();
        let state = seam_state(fabric.clone(), Registry::new(vec![&SPEC]), root);
        (fabric, state)
    }

    /// For tests that never touch disk — the root is stored, not used.
    fn test_state() -> (Fabric, Shared) {
        test_state_at(std::env::temp_dir())
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
    fn the_jail_refuses_every_escape_by_name() {
        let root = Path::new("C:/somewhere/content");
        assert!(jail(root, "inbox/comp-1").is_ok());
        for bad in [
            "../out",
            "inbox/../../out",
            "C:/abs",
            "a\\b",
            "",
            "inbox//x",
        ] {
            assert!(jail(root, bad).is_err(), "must refuse {bad:?}");
        }
    }

    fn demo_comp(clip_rel: &str, out_pt: f32) -> Composition {
        let mut comp = Composition::default_1080p30();
        comp.tracks[0].layers.push(dpl_comp::Layer {
            id: "clip-1".into(),
            source: dpl_comp::Source::Artifact {
                rel: clip_rel.into(),
            },
            start: 0.0,
            in_pt: 0.0,
            out_pt,
        });
        comp
    }

    #[tokio::test]
    async fn an_invalid_comp_is_refused_and_nothing_is_written() {
        let root = temp_root("invalid-comp");
        let (_fabric, state) = test_state_at(root.clone());
        // Zero-length layer — the doctor names it, the disk stays empty.
        let comp = demo_comp("inbox/a.mp4", 0.0);
        let resp = router(state)
            .oneshot(post_json(
                "/comp",
                json!({"comp": serde_json::to_value(&comp).expect("json")}),
            ))
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_json(resp).await;
        assert!(body["issues"][0]
            .as_str()
            .expect("issue")
            .contains("clip-1"));
        assert!(
            !root.join("inbox").join("comp-1").exists(),
            "a refused comp must write nothing"
        );
    }

    #[tokio::test]
    async fn a_valid_post_mints_a_dir_and_round_trips_through_get() {
        let root = temp_root("valid-comp");
        let (_fabric, state) = test_state_at(root.clone());
        let comp = demo_comp("inbox/a.mp4", 5.0);
        let resp = router(Arc::clone(&state))
            .oneshot(post_json(
                "/comp",
                json!({"comp": serde_json::to_value(&comp).expect("json")}),
            ))
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["dir"], json!("inbox/comp-1"));
        assert!(root.join("inbox/comp-1/comp.json").is_file());
        // GET /comp returns the same document, doctor-clean.
        let resp = router(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .uri("/comp?dir=inbox/comp-1")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(
            body["comp"]["tracks"][0]["layers"][0]["id"],
            json!("clip-1")
        );
        assert_eq!(body["issues"], json!([]));
        // GET /comps lists it with its duration.
        let resp = router(state)
            .oneshot(
                Request::builder()
                    .uri("/comps")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let body = body_json(resp).await;
        assert_eq!(body["comps"][0]["dir"], json!("inbox/comp-1"));
        assert_eq!(body["comps"][0]["clips"], json!(1));
    }

    #[tokio::test]
    async fn a_render_with_no_playable_clips_is_refused() {
        let root = temp_root("render-empty");
        let (_fabric, state) = test_state_at(root.clone());
        // An element-only comp is valid SHAPE but has nothing to stitch.
        let mut comp = Composition::default_1080p30();
        comp.tracks[0].layers.push(dpl_comp::Layer {
            id: "el-1".into(),
            source: dpl_comp::Source::Element { id: "spark".into() },
            start: 0.0,
            in_pt: 0.0,
            out_pt: 2.0,
        });
        dpl_comp::save(&root.join("inbox/only-elements"), &comp).expect("save");
        let resp = router(state)
            .oneshot(post_json(
                "/command",
                json!({"cmd": "render", "dir": "inbox/only-elements"}),
            ))
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_json(resp).await;
        assert!(body["issues"][0]
            .as_str()
            .expect("issue")
            .contains("no playable clips"));
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
