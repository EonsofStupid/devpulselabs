//! C2b `dpl-coach-overlay` — Linux/X11 backend (devtools-only).
//!
//! ADR: `docs/plans/coach-overlay-adr.md`. Plan: `docs/plans/coach-c2b-overlay-daemon.md`.
//!
//! Subcommands:
//! - `self-test` — locate + stream-verify + WINDOW paint + Sync-grab + XTEST → Triggered
//! - `locate` / `invoke` / `spotlight` — spike-parity operators (debug)

// spawn-audit: exempt(devtools overlay — operator console is the product)

use std::{
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use async_io::block_on;
use atspi::{
    proxy::{accessible::AccessibleProxy, action::ActionProxy, component::ComponentProxy},
    CoordType, ObjectRefOwned, Role,
};
use dpl_coach_overlay::{
    client_size, compose_screen_rect, press_edge_in_hotspot, FrameExtents, IRect, PointerSample,
    WindowOrigin,
};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{self, ConnectionExt as _},
        xtest::ConnectionExt as _,
    },
    rust_connection::RustConnection,
};
use zbus::{proxy, Connection as ZbusConnection};

pub fn entry() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("dpl-coach-overlay: {error}");
            2
        }
    };
    std::process::exit(code);
}

fn run(args: &[String]) -> Result<i32, String> {
    let mut app = "gnome-calculator".to_string();
    let mut name = "7".to_string();
    let mut seconds = 20u64;
    let mut message = "Press 7".to_string();
    let mut command = "self-test".to_string();
    let mut proof_xtest = false;
    let mut debug_pointer = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "self-test" | "locate" | "invoke" | "spotlight" => command = args[i].clone(),
            "--self-test" => command = "self-test".into(),
            "--app" => {
                i += 1;
                app = args.get(i).cloned().ok_or("--app needs a value")?;
            }
            "--name" => {
                i += 1;
                name = args.get(i).cloned().ok_or("--name needs a value")?;
            }
            "--seconds" => {
                i += 1;
                seconds = args
                    .get(i)
                    .ok_or("--seconds needs a value")?
                    .parse()
                    .map_err(|e| format!("--seconds: {e}"))?;
            }
            "--message" => {
                i += 1;
                message = args.get(i).cloned().ok_or("--message needs a value")?;
            }
            "--proof-xtest" => proof_xtest = true,
            "--debug-pointer" => debug_pointer = true,
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    if command == "self-test" {
        proof_xtest = true;
        if seconds > 12 {
            seconds = 8;
        }
        debug_pointer = true;
        message = format!("self-test {name}");
        command = "spotlight".into();
        println!("dpl-coach-overlay: self-test app={app} name={name}");
    }

    ensure_calculator_running(&app)?;

    match command.as_str() {
        "locate" => {
            let located = block_on(locate_target(&app, &name))?;
            println!(
                "locate: app={app} name={name} window_rect={}×{}@{},{} screen_rect={}×{}@{},{} pid_window={:?}",
                located.window_rect.w,
                located.window_rect.h,
                located.window_rect.x,
                located.window_rect.y,
                located.screen_rect.w,
                located.screen_rect.h,
                located.screen_rect.x,
                located.screen_rect.y,
                located.window_id
            );
            Ok(0)
        }
        "invoke" => {
            let changed = block_on(invoke_click(&app, &name))?;
            println!(
                "invoke: doAction(click) ok · display_changed={changed:?} (None = Text iface empty on this GTK4)"
            );
            Ok(0)
        }
        "spotlight" => {
            let located = block_on(locate_target(&app, &name))?;
            let verified = block_on(stream_verify_window_hit(&app, located.window_rect, &name))?;
            println!(
                "spotlight: screen_rect={}×{}@{},{} · stream_hit={verified:?} · {seconds}s · {message} · proof_xtest={proof_xtest}",
                located.screen_rect.w,
                located.screen_rect.h,
                located.screen_rect.x,
                located.screen_rect.y
            );
            let triggered = Arc::new(AtomicBool::new(false));
            let stop_watch = Arc::new(AtomicBool::new(false));
            spawn_hotspot_watch(
                located.screen_rect,
                Arc::clone(&triggered),
                Arc::clone(&stop_watch),
                debug_pointer,
            );
            // Display observe only for digit targets — and only when the
            // display text matches that digit (never Triggered on a wrong key).
            if name.chars().all(|c| c.is_ascii_digit()) {
                spawn_display_watch(
                    app.clone(),
                    name.clone(),
                    Arc::clone(&triggered),
                    Arc::clone(&stop_watch),
                    debug_pointer,
                );
            }
            if proof_xtest {
                let rect = located.screen_rect;
                thread::spawn(move || {
                    // Let the overlay map before injecting.
                    thread::sleep(Duration::from_millis(700));
                    let cx = (rect.x + rect.w / 2) as i16;
                    let cy = (rect.y + rect.h / 2) as i16;
                    if let Err(error) = xtest_press_hold(cx, cy, Duration::from_millis(80)) {
                        eprintln!("proof-xtest: {error}");
                    }
                });
            }
            let caption = if message.is_empty() {
                format!("Press {verified}")
            } else {
                format!("{message} · stream:{verified}")
            };
            // Paint in AT-SPI WINDOW space: overlay sits on the calculator
            // client rect so paint coords == GetExtents(WINDOW) == hit-test.
            // Observe/grab still use screen_rect (root pixels).
            let result = run_spotlight(SpotlightPaint {
                window_hotspot: located.window_rect,
                client_origin: located.client_origin,
                client_size: located.client_size,
                screen_hotspot: located.screen_rect,
                message: caption,
                seconds,
                triggered: Arc::clone(&triggered),
                debug: debug_pointer,
            });
            stop_watch.store(true, Ordering::SeqCst);
            result.map_err(|e| e.to_string())?;
            let hit = triggered.load(Ordering::SeqCst);
            println!(
                "result: {}",
                if hit {
                    "Triggered (hotspot ButtonPress and/or matching display digit)"
                } else {
                    "timeout — no hotspot press and no matching display digit"
                }
            );
            Ok(if hit { 0 } else { 1 })
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn ensure_calculator_running(app: &str) -> Result<(), String> {
    if app != "gnome-calculator" {
        return Ok(());
    }
    let running = block_on(async {
        match a11y_connection().await {
            Ok(conn) => find_app_root(&conn, app).await.is_ok(),
            Err(_) => false,
        }
    });
    if running {
        return Ok(());
    }
    println!("dpl-coach-overlay: launching {app}");
    // spawn-audit: exempt covered by file-level marker
    Command::new(app)
        .spawn()
        .map_err(|e| format!("spawn {app}: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        let ready = block_on(async {
            let Ok(conn) = a11y_connection().await else {
                return false;
            };
            find_app_root(&conn, app).await.is_ok()
        });
        if ready {
            thread::sleep(Duration::from_millis(400));
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }
    Err(format!("{app} did not appear on the AT-SPI bus"))
}

struct Located {
    window_rect: IRect,
    screen_rect: IRect,
    /// Calculator client origin in root pixels (X11 origin + frame extents).
    client_origin: (i32, i32),
    client_size: (i32, i32),
    window_id: u32,
}

async fn locate_target(app: &str, control_name: &str) -> Result<Located, String> {
    let conn = a11y_connection().await.map_err(|e| e.to_string())?;
    let target = find_named_button(&conn, app, control_name).await?;
    let component = ComponentProxy::builder(&conn)
        .destination(target.inner().destination())
        .map_err(|e| e.to_string())?
        .path(target.inner().path())
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let (x, y, w, h) = component
        .get_extents(CoordType::Window)
        .await
        .map_err(|e| e.to_string())?;
    let window_rect = IRect::new(x, y, w, h);
    let (window_id, origin, extents, outer_w, outer_h) =
        x11_window_for_class("gnome-calculator")
            .ok_or_else(|| "refused: no resolvable X11 window".to_string())?;
    if outer_w <= 0 || outer_h <= 0 {
        return Err("refused: degenerate X11 window".into());
    }
    let (cw, ch) = client_size(outer_w, outer_h, extents);
    let client_origin = dpl_coach_overlay::client_origin(origin, extents);
    println!(
        "x11: id=0x{window_id:x} origin={origin:?} extents={extents:?} client={cw}×{ch}@{},{}",
        client_origin.0, client_origin.1
    );
    let screen_rect =
        compose_screen_rect(origin, extents, window_rect).map_err(|e| format!("{e:?}"))?;
    Ok(Located {
        window_rect,
        screen_rect,
        client_origin,
        client_size: (cw, ch),
        window_id,
    })
}

/// Stream identity check: AT-SPI `GetAccessibleAtPoint` in WINDOW space at the
/// composed control's center must resolve to the expected name (button or its
/// label). No screenshots — the a11y tree is the source of truth.
async fn stream_verify_window_hit(
    app: &str,
    window_rect: IRect,
    expected: &str,
) -> Result<String, String> {
    let conn = a11y_connection().await.map_err(|e| e.to_string())?;
    let root = find_app_root(&conn, app).await?;
    let frame = find_toplevel_frame(&conn, &root).await?;
    let component = ComponentProxy::builder(&conn)
        .destination(frame.inner().destination())
        .map_err(|e| e.to_string())?
        .path(frame.inner().path())
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let cx = window_rect.x + window_rect.w / 2;
    let cy = window_rect.y + window_rect.h / 2;
    let hit = component
        .get_accessible_at_point(cx, cy, CoordType::Window)
        .await
        .map_err(|e| e.to_string())?;
    let hit_name = accessible_name_walk(&conn, &hit, 0).await?;
    if hit_name == expected {
        return Ok(hit_name);
    }
    Err(format!(
        "stream-verify refused: WINDOW center ({cx},{cy}) hit {hit_name:?}, expected {expected:?}"
    ))
}

async fn find_toplevel_frame<'a>(
    conn: &'a ZbusConnection,
    root: &AccessibleProxy<'_>,
) -> Result<AccessibleProxy<'a>, String> {
    let children = root.get_children().await.map_err(|e| e.to_string())?;
    for child in children {
        let Some(proxy) = proxy_for(conn, &child).await? else {
            continue;
        };
        let role = proxy.get_role().await.unwrap_or(Role::Invalid);
        if matches!(role, Role::Frame | Role::Window | Role::Application) {
            let dest = proxy.inner().destination().to_owned();
            let path = proxy.inner().path().to_owned();
            return AccessibleProxy::builder(conn)
                .destination(dest)
                .map_err(|e| e.to_string())?
                .path(path)
                .map_err(|e| e.to_string())?
                .build()
                .await
                .map_err(|e| e.to_string());
        }
    }
    // Fall back to the app root itself.
    let dest = root.inner().destination().to_owned();
    let path = root.inner().path().to_owned();
    AccessibleProxy::builder(conn)
        .destination(dest)
        .map_err(|e| e.to_string())?
        .path(path)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())
}

async fn accessible_name_walk(
    conn: &ZbusConnection,
    obj: &ObjectRefOwned,
    depth: usize,
) -> Result<String, String> {
    if depth > 6 {
        return Ok(String::new());
    }
    let Some(proxy) = proxy_for(conn, obj).await? else {
        return Ok(String::new());
    };
    let name = proxy.name().await.unwrap_or_default();
    if !name.is_empty() {
        return Ok(name);
    }
    // GTK often hits the unlabeled panel; try parent.
    if let Ok(parent) = proxy.parent().await {
        return Box::pin(accessible_name_walk(conn, &parent, depth + 1)).await;
    }
    Ok(String::new())
}

async fn invoke_click(app: &str, control_name: &str) -> Result<Option<bool>, String> {
    let conn = a11y_connection().await.map_err(|e| e.to_string())?;
    // Clear first so a repeated "7" still changes the display when it was
    // already showing 7 from a prior run.
    if let Ok(clear) = find_named_button(&conn, app, "C").await {
        if let Ok(action) = ActionProxy::builder(&conn)
            .destination(clear.inner().destination())
            .map_err(|e| e.to_string())?
            .path(clear.inner().path())
            .map_err(|e| e.to_string())?
            .build()
            .await
        {
            let _ = action.do_action(0).await;
            thread::sleep(Duration::from_millis(150));
        }
    }
    let before = read_display_text_on(&conn, app).await.unwrap_or_default();
    let target = find_named_button(&conn, app, control_name).await?;
    let action = ActionProxy::builder(&conn)
        .destination(target.inner().destination())
        .map_err(|e| e.to_string())?
        .path(target.inner().path())
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let actions = action.get_actions().await.map_err(|e| e.to_string())?;
    let index = actions
        .iter()
        .position(|a| a.name.eq_ignore_ascii_case("click"))
        .or(if actions.is_empty() { None } else { Some(0) })
        .ok_or_else(|| format!("no click action on {control_name}; actions={actions:?}"))?;
    let ok = action
        .do_action(index as i32)
        .await
        .map_err(|e| e.to_string())?;
    if !ok {
        return Err(format!(
            "do_action({index}) returned false; actions={actions:?}"
        ));
    }
    thread::sleep(Duration::from_millis(300));
    let after = read_display_text_on(&conn, app).await.unwrap_or_default();
    println!("invoke: display before={before:?} after={after:?}");
    if before.is_empty() && after.is_empty() {
        Ok(None)
    } else {
        Ok(Some(after != before))
    }
}

async fn read_display_text_on(conn: &ZbusConnection, app: &str) -> Result<String, String> {
    let root = find_app_root(conn, app).await?;
    // Calculator's entry is Role::Text (not labels). Prefer get_text(0, -1).
    if let Some(text) = find_display_text(conn, &root, 0).await? {
        return Ok(text);
    }
    Ok(String::new())
}

async fn find_display_text(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    depth: usize,
) -> Result<Option<String>, String> {
    if depth > 12 {
        return Ok(None);
    }
    let role = node.get_role().await.unwrap_or(Role::Invalid);
    if matches!(role, Role::Text | Role::Entry | Role::PasswordText) {
        if let Ok(text) = atspi::proxy::text::TextProxy::builder(conn)
            .destination(node.inner().destination())
            .map_err(|e| e.to_string())?
            .path(node.inner().path())
            .map_err(|e| e.to_string())?
            .build()
            .await
        {
            // -1 = to end; GTK4 calculator returns body via this form.
            if let Ok(body) = text.get_text(0, -1).await {
                let body = body.trim().to_string();
                if !body.is_empty() {
                    return Ok(Some(body));
                }
            }
            if let Ok(count) = text.character_count().await {
                if count > 0 {
                    if let Ok(body) = text.get_text(0, count).await {
                        let body = body.trim().to_string();
                        if !body.is_empty() {
                            return Ok(Some(body));
                        }
                    }
                }
            }
        }
    }
    let children = node.get_children().await.map_err(|e| e.to_string())?;
    for child in children {
        let Some(child_proxy) = proxy_for(conn, &child).await? else {
            continue;
        };
        if let Some(found) = Box::pin(find_display_text(conn, &child_proxy, depth + 1)).await? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn find_named_button(
    conn: &ZbusConnection,
    app: &str,
    control_name: &str,
) -> Result<AccessibleProxy<'static>, String> {
    let root = find_app_root(conn, app).await?;
    if let Some(found) = find_button_named(conn, &root, control_name, 0).await? {
        return Ok(found);
    }
    Err(format!(
        "refused: no Button named {control_name:?} under {app}"
    ))
}

async fn find_button_named(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    control_name: &str,
    depth: usize,
) -> Result<Option<AccessibleProxy<'static>>, String> {
    if depth > 12 {
        return Ok(None);
    }
    let role = node.get_role().await.unwrap_or(Role::Invalid);
    let name = node.name().await.unwrap_or_default();
    if name == control_name && matches!(role, Role::Button | Role::PushButtonMenu) {
        // Re-build as 'static via destination/path owned strings.
        let dest = node.inner().destination().to_owned();
        let path = node.inner().path().to_owned();
        let rebuilt = AccessibleProxy::builder(conn)
            .destination(dest)
            .map_err(|e| e.to_string())?
            .path(path)
            .map_err(|e| e.to_string())?
            .build()
            .await
            .map_err(|e| e.to_string())?;
        return Ok(Some(rebuilt));
    }
    let children = node.get_children().await.map_err(|e| e.to_string())?;
    for child in children {
        let Some(child_proxy) = proxy_for(conn, &child).await? else {
            continue;
        };
        if let Some(found) = Box::pin(find_button_named(
            conn,
            &child_proxy,
            control_name,
            depth + 1,
        ))
        .await?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn find_app_root(
    conn: &ZbusConnection,
    app: &str,
) -> Result<AccessibleProxy<'static>, String> {
    let desktop = AccessibleProxy::builder(conn)
        .destination("org.a11y.atspi.Registry")
        .map_err(|e| e.to_string())?
        .path("/org/a11y/atspi/accessible/root")
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let children = desktop.get_children().await.map_err(|e| e.to_string())?;
    for child in children {
        let Some(proxy) = proxy_for(conn, &child).await? else {
            continue;
        };
        let name = proxy.name().await.unwrap_or_default();
        if name == app {
            let dest = proxy.inner().destination().to_owned();
            let path = proxy.inner().path().to_owned();
            let rebuilt = AccessibleProxy::builder(conn)
                .destination(dest)
                .map_err(|e| e.to_string())?
                .path(path)
                .map_err(|e| e.to_string())?
                .build()
                .await
                .map_err(|e| e.to_string())?;
            return Ok(rebuilt);
        }
    }
    Err(format!("refused: AT-SPI app {app:?} not found"))
}

async fn proxy_for<'a>(
    conn: &'a ZbusConnection,
    obj: &'a ObjectRefOwned,
) -> Result<Option<AccessibleProxy<'a>>, String> {
    let Some(name) = obj.name_as_str() else {
        return Ok(None);
    };
    let path = obj.path_as_str();
    if path == "/org/a11y/atspi/null" {
        return Ok(None);
    }
    let proxy = AccessibleProxy::builder(conn)
        .destination(name)
        .map_err(|e| e.to_string())?
        .path(path)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(proxy))
}

#[proxy(
    interface = "org.a11y.Bus",
    default_service = "org.a11y.Bus",
    default_path = "/org/a11y/bus"
)]
trait A11yBus {
    #[zbus(name = "GetAddress")]
    fn get_address(&self) -> zbus::Result<String>;
}

async fn a11y_connection() -> zbus::Result<ZbusConnection> {
    let session = ZbusConnection::session().await?;
    let bus = A11yBusProxy::new(&session).await?;
    let address = bus.get_address().await?;
    zbus::connection::Builder::address(address.as_str())?
        .build()
        .await
}

fn x11_window_for_class(class_needle: &str) -> Option<(u32, WindowOrigin, FrameExtents, i32, i32)> {
    let (conn, screen_num) = RustConnection::connect(None).ok()?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let atom_client = intern(&conn, "_NET_CLIENT_LIST")?;
    let atom_pid = intern(&conn, "_NET_WM_PID")?;
    let atom_class = intern(&conn, "WM_CLASS")?;
    let atom_extents = intern(&conn, "_GTK_FRAME_EXTENTS")?;
    let atom_name = intern(&conn, "_NET_WM_NAME")?;
    let _ = (atom_pid, atom_name);

    let reply = conn
        .get_property(false, root, atom_client, xproto::AtomEnum::WINDOW, 0, 8192)
        .ok()?
        .reply()
        .ok()?;
    let raw: Vec<u8> = reply.value;
    if !raw.len().is_multiple_of(4) {
        return None;
    }
    let windows: Vec<u32> = raw
        .chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let mut best: Option<(u32, WindowOrigin, FrameExtents, i32, i32, i32)> = None;
    for &wid in &windows {
        let class = string_prop(&conn, wid, atom_class).unwrap_or_default();
        if !class.to_lowercase().contains(&class_needle.to_lowercase()) {
            continue;
        }
        let geom = conn.get_geometry(wid).ok()?.reply().ok()?;
        let tree = conn.query_tree(wid).ok()?.reply().ok()?;
        let translated = conn
            .translate_coordinates(tree.parent, root, geom.x, geom.y)
            .ok()?
            .reply()
            .ok()?;
        let outer_w = i32::from(geom.width);
        let outer_h = i32::from(geom.height);
        if outer_w < 50 || outer_h < 50 {
            continue;
        }
        let extents = card4_prop(&conn, wid, atom_extents).unwrap_or_default();
        let area = outer_w * outer_h;
        let candidate = (
            wid,
            WindowOrigin {
                x: i32::from(translated.dst_x),
                y: i32::from(translated.dst_y),
            },
            extents,
            outer_w,
            outer_h,
            area,
        );
        if best.as_ref().map(|b| b.5).unwrap_or(0) < area {
            best = Some(candidate);
        }
    }
    best.map(|(id, o, e, w, h, _)| (id, o, e, w, h))
}

fn intern(conn: &RustConnection, name: &str) -> Option<xproto::Atom> {
    conn.intern_atom(false, name.as_bytes())
        .ok()?
        .reply()
        .ok()
        .map(|r| r.atom)
}

fn string_prop(conn: &RustConnection, wid: u32, atom: xproto::Atom) -> Option<String> {
    let reply = conn
        .get_property(false, wid, atom, xproto::AtomEnum::ANY, 0, 4096)
        .ok()?
        .reply()
        .ok()?;
    let bytes: Vec<u8> = reply.value;
    Some(String::from_utf8_lossy(&bytes).replace('\0', " "))
}

fn card4_prop(conn: &RustConnection, wid: u32, atom: xproto::Atom) -> Option<FrameExtents> {
    let reply = conn
        .get_property(false, wid, atom, xproto::AtomEnum::CARDINAL, 0, 4)
        .ok()?
        .reply()
        .ok()?;
    let raw: Vec<u8> = reply.value;
    if raw.len() < 16 {
        return None;
    }
    let vals: Vec<u32> = raw
        .chunks_exact(4)
        .take(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    Some(FrameExtents {
        left: vals[0] as i32,
        right: vals[1] as i32,
        top: vals[2] as i32,
        bottom: vals[3] as i32,
    })
}

fn xtest_press_hold(root_x: i16, root_y: i16, hold: Duration) -> Result<(), String> {
    let (conn, screen_num) = RustConnection::connect(None).map_err(|e| e.to_string())?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    // MotionNotify=6, ButtonPress=4, ButtonRelease=5.
    conn.xtest_fake_input(6, 0, 0, root, root_x, root_y, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    conn.xtest_fake_input(4, 1, 0, root, root_x, root_y, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    conn.flush().map_err(|e| e.to_string())?;
    // Hold so QueryPointer sampling in the overlay can see Button1 down.
    thread::sleep(hold);
    conn.xtest_fake_input(5, 1, 0, root, root_x, root_y, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn spawn_display_watch(
    app: String,
    expected_digit: String,
    triggered: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    debug: bool,
) {
    thread::spawn(move || {
        // Reset so a press of the same digit is still a visible change.
        let _ = block_on(clear_calculator_display(&app));
        // Wait for clear to settle — GTK can briefly publish "" mid-update.
        let mut before = String::new();
        for _ in 0..20 {
            if stop.load(Ordering::SeqCst) || triggered.load(Ordering::SeqCst) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
            before = block_on(read_display_text(&app)).unwrap_or_default();
            if !before.is_empty() {
                break;
            }
        }
        if debug {
            eprintln!("display-watch: expect={expected_digit:?} before={before:?}");
        }
        while !stop.load(Ordering::SeqCst) && !triggered.load(Ordering::SeqCst) {
            let Ok(now) = block_on(read_display_text(&app)) else {
                thread::sleep(Duration::from_millis(50));
                continue;
            };
            // Only the coached digit counts — pressing 5 while coaching 7 must not Trigger.
            if !now.is_empty()
                && now != before
                && (now == expected_digit || now.ends_with(&expected_digit))
            {
                if debug {
                    eprintln!(
                        "display-watch: TRIGGERED expect={expected_digit:?} before={before:?} after={now:?}"
                    );
                }
                triggered.store(true, Ordering::SeqCst);
                return;
            }
            thread::sleep(Duration::from_millis(40));
        }
    });
}

async fn clear_calculator_display(app: &str) -> Result<(), String> {
    let conn = a11y_connection().await.map_err(|e| e.to_string())?;
    let clear = find_named_button(&conn, app, "C").await?;
    let action = ActionProxy::builder(&conn)
        .destination(clear.inner().destination())
        .map_err(|e| e.to_string())?
        .path(clear.inner().path())
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let _ = action.do_action(0).await;
    Ok(())
}

async fn read_display_text(app: &str) -> Result<String, String> {
    let conn = a11y_connection().await.map_err(|e| e.to_string())?;
    read_display_text_on(&conn, app).await
}

fn spawn_hotspot_watch(
    hotspot: IRect,
    triggered: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    debug: bool,
) {
    thread::spawn(move || {
        let Ok((conn, screen_num)) = RustConnection::connect(None) else {
            eprintln!("hotspot-watch: failed to connect to X11");
            return;
        };
        let root = conn.setup().roots[screen_num].root;
        if let Err(error) = (|| {
            conn.grab_button(
                false,
                root,
                xproto::EventMask::BUTTON_PRESS,
                xproto::GrabMode::SYNC,
                xproto::GrabMode::ASYNC,
                x11rb::NONE,
                x11rb::NONE,
                xproto::ButtonIndex::M1,
                xproto::ModMask::ANY,
            )?
            .check()?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })() {
            eprintln!("hotspot-watch: grab_button failed: {error}");
            return;
        }
        if debug {
            eprintln!(
                "hotspot-watch: sync-grab Button1 · hotspot={}×{}@{},{}",
                hotspot.w, hotspot.h, hotspot.x, hotspot.y
            );
        }
        while !stop.load(Ordering::SeqCst) && !triggered.load(Ordering::SeqCst) {
            let event = match conn.poll_for_event() {
                Ok(Some(event)) => event,
                Ok(None) => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => {
                    eprintln!("hotspot-watch: poll error: {error}");
                    break;
                }
            };
            let x11rb::protocol::Event::ButtonPress(press) = event else {
                continue;
            };
            let sample = PointerSample {
                x: i32::from(press.root_x),
                y: i32::from(press.root_y),
                button1_down: true,
            };
            let previous = PointerSample {
                x: sample.x,
                y: sample.y,
                button1_down: false,
            };
            let inside = press_edge_in_hotspot(hotspot, previous, sample);
            if debug {
                eprintln!(
                    "hotspot-watch: ButtonPress at {},{} inside={inside}",
                    sample.x, sample.y
                );
            }
            // Always replay so the calculator (or whatever is under the
            // pointer) still receives the real click.
            if let Err(error) = (|| {
                conn.allow_events(xproto::Allow::REPLAY_POINTER, x11rb::CURRENT_TIME)?
                    .check()?;
                Ok::<(), Box<dyn std::error::Error>>(())
            })() {
                eprintln!("hotspot-watch: allow_events failed: {error}");
            }
            let _ = conn.flush();
            if inside {
                if debug {
                    eprintln!("hotspot-watch: TRIGGERED at {},{}", sample.x, sample.y);
                }
                triggered.store(true, Ordering::SeqCst);
                break;
            }
        }
        let _ = conn.ungrab_button(xproto::ButtonIndex::M1, root, xproto::ModMask::ANY);
        let _ = conn.flush();
    });
}

struct SpotlightPaint {
    window_hotspot: IRect,
    client_origin: (i32, i32),
    client_size: (i32, i32),
    screen_hotspot: IRect,
    message: String,
    seconds: u64,
    triggered: Arc<AtomicBool>,
    debug: bool,
}

fn run_spotlight(paint: SpotlightPaint) -> eframe::Result<()> {
    let SpotlightPaint {
        window_hotspot,
        client_origin,
        client_size,
        screen_hotspot,
        message,
        seconds,
        triggered,
        debug,
    } = paint;
    let started = Instant::now();
    let (cw, ch) = client_size;
    if cw <= 0 || ch <= 0 {
        eprintln!("spotlight: refused degenerate client size {cw}×{ch}");
        return Ok(());
    }

    // Overlay matches the calculator's visible client area — same origin the
    // AT-SPI WINDOW coordinate system uses after frame-extents composition.
    let viewport = egui::ViewportBuilder::default()
        .with_title("DevPULSE foreign-target spotlight")
        .with_inner_size([cw as f32, ch as f32])
        .with_position([client_origin.0 as f32, client_origin.1 as f32])
        .with_decorations(false)
        .with_transparent(true)
        .with_mouse_passthrough(true)
        .with_always_on_top()
        .with_taskbar(false);

    if debug {
        eprintln!(
            "spotlight-paint: client={}×{}@{},{} window_hotspot={}×{}@{},{} screen_hotspot={}×{}@{},{}",
            cw,
            ch,
            client_origin.0,
            client_origin.1,
            window_hotspot.w,
            window_hotspot.h,
            window_hotspot.x,
            window_hotspot.y,
            screen_hotspot.w,
            screen_hotspot.h,
            screen_hotspot.x,
            screen_hotspot.y
        );
    }

    eframe::run_native(
        "dpl-coach-overlay",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |_cc| {
            Ok(Box::new(SpotlightApp {
                window_hotspot,
                screen_hotspot,
                client_origin,
                client_size,
                message,
                started,
                seconds,
                triggered,
                debug,
                logged_metrics: false,
            }))
        }),
    )
}

struct SpotlightApp {
    /// Hotspot in AT-SPI WINDOW pixels (paint space).
    window_hotspot: IRect,
    /// Same hotspot in root pixels (observe / debug).
    screen_hotspot: IRect,
    client_origin: (i32, i32),
    client_size: (i32, i32),
    message: String,
    started: Instant,
    seconds: u64,
    triggered: Arc<AtomicBool>,
    debug: bool,
    logged_metrics: bool,
}

impl eframe::App for SpotlightApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.triggered.load(Ordering::SeqCst) || self.started.elapsed().as_secs() >= self.seconds
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let ppp = ui.ctx().pixels_per_point();
        if self.debug && !self.logged_metrics {
            self.logged_metrics = true;
            let outer = ui.ctx().input(|i| i.viewport().outer_rect);
            eprintln!(
                "spotlight-paint: ppp={ppp} outer={outer:?} expected_client={}×{}@{},{} window_hotspot={}×{}@{},{} screen_hotspot={}×{}@{},{}",
                self.client_size.0,
                self.client_size.1,
                self.client_origin.0,
                self.client_origin.1,
                self.window_hotspot.w,
                self.window_hotspot.h,
                self.window_hotspot.x,
                self.window_hotspot.y,
                self.screen_hotspot.w,
                self.screen_hotspot.h,
                self.screen_hotspot.x,
                self.screen_hotspot.y
            );
        }
        // Paint in WINDOW pixels inside the client-sized overlay.
        let target = egui::Rect::from_min_size(
            egui::pos2(
                self.window_hotspot.x as f32 / ppp,
                self.window_hotspot.y as f32 / ppp,
            ),
            egui::vec2(
                self.window_hotspot.w as f32 / ppp,
                self.window_hotspot.h as f32 / ppp,
            ),
        );
        let screen = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(
                self.client_size.0 as f32 / ppp,
                self.client_size.1 as f32 / ppp,
            ),
        );
        let painter = ui.painter();
        let palette = ForeignPalette::default();
        paint_spotlight(painter, &palette, screen, target, &self.message);
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

struct ForeignPalette {
    ring: egui::Color32,
    tint: egui::Color32,
    scatter: egui::Color32,
    caption_fill: egui::Color32,
    caption_ink: egui::Color32,
    ring_width: f32,
    corner_radius: f32,
}

impl Default for ForeignPalette {
    fn default() -> Self {
        // Matches egui_pilot_ai::coach::CoachPalette::default ink.
        Self {
            ring: egui::Color32::from_rgb(0, 229, 255),
            tint: egui::Color32::from_rgba_unmultiplied(0, 229, 255, 48),
            scatter: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 132),
            caption_fill: egui::Color32::from_rgb(12, 14, 20),
            caption_ink: egui::Color32::from_rgb(0, 229, 255),
            ring_width: 3.0,
            corner_radius: 6.0,
        }
    }
}

fn paint_spotlight(
    painter: &egui::Painter,
    palette: &ForeignPalette,
    screen: egui::Rect,
    target: egui::Rect,
    message: &str,
) {
    let target = target.intersect(screen);
    if target.width() <= 0.0 || target.height() <= 0.0 {
        return;
    }
    for band in scatter_bands(screen, &[target]) {
        painter.rect_filled(band, 0.0, palette.scatter);
    }
    // Gradient tint inside the hole.
    let head = palette.tint;
    let tail = egui::Color32::from_rgba_unmultiplied(head.r(), head.g(), head.b(), 0);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(target.left_top(), head);
    mesh.colored_vertex(target.right_top(), head);
    mesh.colored_vertex(target.left_bottom(), tail);
    mesh.colored_vertex(target.right_bottom(), tail);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    painter.add(egui::Shape::mesh(mesh));
    painter.rect_stroke(
        target,
        palette.corner_radius,
        egui::Stroke::new(palette.ring_width, palette.ring),
        egui::StrokeKind::Outside,
    );
    if !message.is_empty() {
        let galley = painter.layout_no_wrap(
            message.to_string(),
            egui::FontId::proportional(16.0),
            palette.caption_ink,
        );
        let size = galley.size() + egui::vec2(18.0, 12.0);
        // Prefer ABOVE the hotspot. Below lands on the next keypad row
        // (coaching 7 put the cyan plate on 5 — operator clicked the plate).
        let mut pos = egui::pos2(target.center().x - size.x * 0.5, target.top() - size.y - 10.0);
        if pos.y < screen.min.y + 4.0 {
            pos.y = target.bottom() + 10.0;
        }
        pos.x = pos.x.clamp(screen.min.x + 8.0, screen.max.x - size.x - 8.0);
        let plate = egui::Rect::from_min_size(pos, size);
        painter.rect_filled(plate, 8.0, palette.caption_fill);
        painter.galley(
            plate.min + egui::vec2(9.0, 6.0),
            galley,
            palette.caption_ink,
        );
    }
}

fn scatter_bands(screen: egui::Rect, cutouts: &[egui::Rect]) -> Vec<egui::Rect> {
    let cutouts: Vec<egui::Rect> = cutouts
        .iter()
        .map(|cutout| cutout.intersect(screen))
        .filter(|cutout| cutout.width() > 0.0 && cutout.height() > 0.0)
        .collect();
    if cutouts.is_empty() {
        return vec![screen];
    }
    let mut edges: Vec<f32> = Vec::with_capacity(cutouts.len() * 2 + 2);
    edges.push(screen.min.y);
    edges.push(screen.max.y);
    for cutout in &cutouts {
        edges.push(cutout.min.y);
        edges.push(cutout.max.y);
    }
    edges.sort_by(f32::total_cmp);
    edges.dedup();
    let mut bands = Vec::new();
    for pair in edges.windows(2) {
        let (top, bottom) = (pair[0], pair[1]);
        if bottom <= top {
            continue;
        }
        let mut spans: Vec<(f32, f32)> = cutouts
            .iter()
            .filter(|cutout| cutout.min.y <= top && cutout.max.y >= bottom)
            .map(|cutout| (cutout.min.x, cutout.max.x))
            .collect();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut x = screen.min.x;
        for (start, end) in spans {
            if start > x {
                bands.push(egui::Rect::from_min_max(
                    egui::pos2(x, top),
                    egui::pos2(start, bottom),
                ));
            }
            x = x.max(end);
        }
        if x < screen.max.x {
            bands.push(egui::Rect::from_min_max(
                egui::pos2(x, top),
                egui::pos2(screen.max.x, bottom),
            ));
        }
    }
    bands
}
