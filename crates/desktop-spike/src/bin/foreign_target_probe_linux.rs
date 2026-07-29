//! C2s foreign-target spike — resolve / locate / paint / observe / invoke
//! against a Tier A app (`gnome-calculator`) we did not write.
//!
//! Plan: `docs/plans/coach-c2s-foreign-target-spike.md`.
//!
//! Subcommands:
//! - `locate` — print composed screen rect (or refuse)
//! - `invoke` — `Action.doAction("click")` with no coordinates
//! - `spotlight` — paint ColdLight spotlight, click-through, wait for
//!   operator press (display changes) → `Triggered`

// spawn-audit: exempt(spike probe — operator console is the product)

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
use dpl_desktop_spike::{client_size, compose_screen_rect, FrameExtents, IRect, WindowOrigin};
use x11rb::{
    connection::Connection,
    protocol::xproto::{self, ConnectionExt as _},
    rust_connection::RustConnection,
};
use zbus::{proxy, Connection as ZbusConnection};

pub fn entry() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("foreign-target-probe: {error}");
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
    let mut command = "locate".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "locate" | "invoke" | "spotlight" => command = args[i].clone(),
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
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
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
            println!(
                "spotlight: screen_rect={}×{}@{},{} · {seconds}s · {message}",
                located.screen_rect.w,
                located.screen_rect.h,
                located.screen_rect.x,
                located.screen_rect.y
            );
            let before = block_on(read_display_text(&app)).unwrap_or_default();
            let triggered = Arc::new(AtomicBool::new(false));
            let watch = Arc::clone(&triggered);
            let app_watch = app.clone();
            let before_watch = before.clone();
            thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(seconds);
                while Instant::now() < deadline {
                    if let Ok(text) = block_on(read_display_text(&app_watch)) {
                        if text != before_watch && !text.is_empty() {
                            watch.store(true, Ordering::SeqCst);
                            return;
                        }
                    }
                    thread::sleep(Duration::from_millis(80));
                }
            });
            run_spotlight(located.screen_rect, &message, seconds).map_err(|e| e.to_string())?;
            let hit = triggered.load(Ordering::SeqCst);
            println!(
                "result: {}",
                if hit {
                    "Triggered (operator press observed via display change)"
                } else {
                    "timeout — no display change"
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
    println!("foreign-target-probe: launching {app}");
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
    println!("x11: id=0x{window_id:x} origin={origin:?} extents={extents:?} client={cw}×{ch}");
    let screen_rect =
        compose_screen_rect(origin, extents, window_rect).map_err(|e| format!("{e:?}"))?;
    Ok(Located {
        window_rect,
        screen_rect,
        window_id,
    })
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
        .or_else(|| if actions.is_empty() { None } else { Some(0) })
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

async fn read_display_text(app: &str) -> Result<String, String> {
    let conn = a11y_connection().await.map_err(|e| e.to_string())?;
    read_display_text_on(&conn, app).await
}

async fn read_display_text_on(conn: &ZbusConnection, app: &str) -> Result<String, String> {
    let root = find_app_root(conn, app).await?;
    // Prefer an accessible whose role is a text-ish name with non-empty content.
    if let Some(text) = find_first_textish(conn, &root, 0).await? {
        return Ok(text);
    }
    Ok(String::new())
}

async fn find_first_textish(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    depth: usize,
) -> Result<Option<String>, String> {
    if depth > 10 {
        return Ok(None);
    }
    // Force the Text proxy — InterfaceSet checks have been empty on this
    // GTK4 calculator while GI still exposes text nodes.
    if let Ok(text) = atspi::proxy::text::TextProxy::builder(conn)
        .destination(node.inner().destination())
        .map_err(|e| e.to_string())?
        .path(node.inner().path())
        .map_err(|e| e.to_string())?
        .build()
        .await
    {
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
    let children = node.get_children().await.map_err(|e| e.to_string())?;
    for child in children {
        let Some(child_proxy) = proxy_for(conn, &child).await? else {
            continue;
        };
        if let Some(found) = Box::pin(find_first_textish(conn, &child_proxy, depth + 1)).await? {
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
    if raw.len() % 4 != 0 {
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

fn run_spotlight(target: IRect, message: &str, seconds: u64) -> eframe::Result<()> {
    let (conn, screen_num) = RustConnection::connect(None).expect("x11");
    let screen = &conn.setup().roots[screen_num];
    let width = f32::from(screen.width_in_pixels);
    let height = f32::from(screen.height_in_pixels);

    let target_rect = egui::Rect::from_min_size(
        egui::pos2(target.x as f32, target.y as f32),
        egui::vec2(target.w as f32, target.h as f32),
    );
    let message = message.to_string();
    let started = Instant::now();

    let viewport = egui::ViewportBuilder::default()
        .with_title("DevPULSE foreign-target spotlight")
        .with_inner_size([width, height])
        .with_position([0.0, 0.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_mouse_passthrough(true)
        .with_always_on_top()
        .with_taskbar(false);

    eframe::run_native(
        "foreign-target-spotlight",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |_cc| {
            Ok(Box::new(SpotlightApp {
                target: target_rect,
                message,
                started,
                seconds,
                screen: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height)),
            }))
        }),
    )
}

struct SpotlightApp {
    target: egui::Rect,
    message: String,
    started: Instant,
    seconds: u64,
    screen: egui::Rect,
}

impl eframe::App for SpotlightApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.started.elapsed().as_secs() >= self.seconds {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let painter = ui.painter();
        let palette = ForeignPalette::default();
        paint_spotlight(painter, &palette, self.screen, self.target, &self.message);
        ui.ctx().request_repaint_after(Duration::from_millis(50));
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
        let mut pos = egui::pos2(target.center().x - size.x * 0.5, target.bottom() + 10.0);
        if pos.y + size.y > screen.max.y {
            pos.y = target.top() - size.y - 10.0;
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
