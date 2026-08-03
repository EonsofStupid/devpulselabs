//! Does `mouse_passthrough` deliver clicks to a window underneath?
//!
//! Companion to `overlay-probe` (transparency). Plan:
//! `docs/plans/p2b-overlay-passthrough.md`.
//!
//! Roles of this one binary:
//! - **witness** — opaque hit target; writes `--hit-file` on primary click
//! - **overlay** — transparent always-on-top; passthrough on/off
//! - **(default) orchestrator** — spawns witness → overlay → XTEST click →
//!   asserts the hit file. Run once with passthrough and once with
//!   `--no-passthrough` for the control pair.
//!
//! Self-terminating. An always-on-top click-through window that outlives
//! its probe is an operator hazard.

// spawn-audit: exempt(spike probe — orchestrator must inherit console for role stdout)

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use x11rb::{
    connection::Connection, protocol::xtest::ConnectionExt as _, rust_connection::RustConnection,
};

pub fn entry() -> eframe::Result<()> {
    let options = ProbeOptions::from_args();
    match options.role {
        Role::Orchestrate => match run_orchestrator(&options) {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("passthrough-probe: orchestrator failed: {error}");
                std::process::exit(2);
            }
        },
        Role::Witness => run_witness(options),
        Role::Overlay => run_overlay(options),
    }
}

fn run_orchestrator(options: &ProbeOptions) -> Result<i32, String> {
    let hit_file = options.hit_file.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("dpl-passthrough-hit-{}.txt", std::process::id()))
    });
    let _ = fs::remove_file(&hit_file);

    println!("passthrough-probe: orchestrator {options:?}");
    println!("hit-file: {}", hit_file.display());

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut witness = spawn_role(&exe, Role::Witness, options, &hit_file)?;
    // Let the witness map before covering it.
    thread::sleep(Duration::from_millis(800));

    let mut overlay = spawn_role(&exe, Role::Overlay, options, &hit_file)?;
    // Overlay must finish mapping + AlwaysOnTop raise before the click.
    thread::sleep(Duration::from_millis(1200));

    let (x, y) = click_point(options);
    if let Ok(name) = window_at(x, y) {
        println!("window-at-click before xtest: {name}");
    }
    println!("xtest click at ({x}, {y})");
    xtest_click(x, y)?;

    let deadline = Instant::now() + Duration::from_millis(1500);
    let mut hit = false;
    while Instant::now() < deadline {
        if hit_file.is_file() {
            let body = fs::read_to_string(&hit_file).unwrap_or_default();
            if body.contains("HIT") {
                hit = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    // Tear down children even if they would self-exit.
    let _ = witness.kill();
    let _ = overlay.kill();
    let _ = witness.wait();
    let _ = overlay.wait();

    let expect_hit = options.passthrough;
    let ok = hit == expect_hit;
    println!(
        "result: hit={hit} passthrough={} expect_hit={expect_hit} → {}",
        options.passthrough,
        if ok { "PASS" } else { "FAIL" }
    );
    if let Ok(body) = fs::read_to_string(&hit_file) {
        println!("hit-file body: {body:?}");
    } else {
        println!("hit-file body: (missing)");
    }
    Ok(if ok { 0 } else { 1 })
}

fn spawn_role(
    exe: &Path,
    role: Role,
    options: &ProbeOptions,
    hit_file: &Path,
) -> Result<Child, String> {
    let mut cmd = Command::new(exe);
    cmd.arg(match role {
        Role::Witness => "--witness",
        Role::Overlay => "--overlay",
        Role::Orchestrate => unreachable!(),
    });
    cmd.arg("--size").arg(format!(
        "{}x{}",
        options.width as i32, options.height as i32
    ));
    if let Some([x, y]) = options.position {
        cmd.arg("--at").arg(format!("{},{}", x as i32, y as i32));
    }
    cmd.arg("--seconds").arg(options.seconds.to_string());
    cmd.arg("--hit-file").arg(hit_file);
    if !options.passthrough {
        cmd.arg("--no-passthrough");
    }
    // Control (no-passthrough) uses an opaque overlay so alpha-shaped input
    // cannot masquerade as always-on click-through.
    if !options.passthrough || !options.transparent {
        cmd.arg("--opaque");
    }
    cmd.spawn().map_err(|e| format!("spawn {role:?}: {e}"))
}

fn click_point(options: &ProbeOptions) -> (i16, i16) {
    let [x, y] = options.position.unwrap_or([120.0, 120.0]);
    (
        (x + options.width * 0.5) as i16,
        (y + options.height * 0.5) as i16,
    )
}

fn window_at(root_x: i16, root_y: i16) -> Result<String, String> {
    // Best-effort stacking diagnostic via `xdotool` if present; never required.
    let _ = Command::new("xdotool")
        .args(["mousemove", &root_x.to_string(), &root_y.to_string()])
        .status();
    let output = Command::new("xdotool")
        .args(["getmouselocation", "--shell"])
        .output()
        .map_err(|e| e.to_string())?;
    let shell = String::from_utf8_lossy(&output.stdout);
    let wid = shell
        .lines()
        .find_map(|line| line.strip_prefix("WINDOW="))
        .unwrap_or("?");
    let name = Command::new("xdotool")
        .args(["getwindowname", wid])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "?".into());
    Ok(format!("WINDOW={wid} name={name:?}"))
}

fn xtest_click(root_x: i16, root_y: i16) -> Result<(), String> {
    let (conn, screen_num) = RustConnection::connect(None).map_err(|e| e.to_string())?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // Motion first so the button events land on the intended root coords.
    // X11 event types: MotionNotify=6, ButtonPress=4, ButtonRelease=5.
    conn.xtest_fake_input(6, 0, 0, root, root_x, root_y, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    conn.xtest_fake_input(4, 1, 0, root, root_x, root_y, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    conn.xtest_fake_input(5, 1, 0, root, root_x, root_y, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    conn.flush().map_err(|e| e.to_string())?;
    // Brief settle so the target process sees the events.
    thread::sleep(Duration::from_millis(100));
    Ok(())
}

fn run_witness(options: ProbeOptions) -> eframe::Result<()> {
    println!("passthrough-probe: witness {options:?}");
    let hit_file = options
        .hit_file
        .clone()
        .expect("--witness requires --hit-file");

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("DevPULSE passthrough witness")
        .with_inner_size([options.width, options.height])
        .with_decorations(false)
        .with_taskbar(false)
        // Deliberately NOT always-on-top — the overlay must stack above.
        .with_transparent(false);
    if let Some(position) = options.position {
        viewport = viewport.with_position(position);
    }

    eframe::run_native(
        "passthrough-witness",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |_cc| {
            Ok(Box::new(Witness {
                options,
                hit_file,
                started: Instant::now(),
                armed: true,
            }))
        }),
    )
}

fn run_overlay(options: ProbeOptions) -> eframe::Result<()> {
    println!("passthrough-probe: overlay {options:?}");
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("DevPULSE passthrough overlay")
        .with_inner_size([options.width, options.height])
        .with_decorations(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_transparent(options.transparent)
        .with_mouse_passthrough(options.passthrough);
    if let Some(position) = options.position {
        viewport = viewport.with_position(position);
    }

    eframe::run_native(
        "passthrough-overlay",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| {
            if let Some(state) = &cc.wgpu_render_state {
                let info = state.adapter.get_info();
                println!(
                    "renderer: wgpu · backend {:?} · adapter {} ({:?})",
                    info.backend, info.name, info.device_type
                );
            }
            Ok(Box::new(Overlay {
                options,
                started: Instant::now(),
            }))
        }),
    )
}

struct Witness {
    options: ProbeOptions,
    hit_file: PathBuf,
    started: Instant,
    armed: bool,
}

impl eframe::App for Witness {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.12, 0.45, 0.22, 1.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let remaining = self
            .options
            .seconds
            .saturating_sub(self.started.elapsed().as_secs());
        if remaining == 0 {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        let rect = ui.max_rect();
        let response = ui.allocate_rect(rect, egui::Sense::click());
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 120, 60));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("WITNESS · click me · {remaining}s"),
            egui::FontId::proportional(18.0),
            egui::Color32::WHITE,
        );

        if self.armed && response.clicked() {
            let body = format!(
                "HIT\npassthrough_probe=1\nts={}\n",
                self.started.elapsed().as_millis()
            );
            if let Err(error) = fs::write(&self.hit_file, body) {
                eprintln!("witness: could not write hit-file: {error}");
            } else {
                println!("witness: HIT written to {}", self.hit_file.display());
            }
            self.armed = false;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }
}

struct Overlay {
    options: ProbeOptions,
    started: Instant,
}

impl eframe::App for Overlay {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if self.options.transparent {
            [0.0, 0.0, 0.0, 0.0]
        } else {
            [0.08, 0.09, 0.16, 1.0]
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let remaining = self
            .options
            .seconds
            .saturating_sub(self.started.elapsed().as_secs());
        if remaining == 0 {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        let rect = ui.max_rect();

        // Control runs must own opaque pixels under the click, or a
        // compositor that routes by alpha will theme like "passthrough
        // always works". The ring alone leaves a hollow center.
        if !self.options.passthrough {
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(20, 24, 40, 220),
            );
            let _ = ui.allocate_rect(rect, egui::Sense::click());
        }

        ui.painter().circle_stroke(
            rect.center(),
            rect.height().min(rect.width()) * 0.35,
            egui::Stroke::new(6.0, egui::Color32::from_rgb(0, 229, 255)),
        );
        let card = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + 34.0),
            egui::vec2(rect.width() * 0.85, 44.0),
        );
        ui.painter()
            .rect_filled(card, 8.0, egui::Color32::from_rgb(12, 14, 20));
        ui.painter().text(
            card.center(),
            egui::Align2::CENTER_CENTER,
            format!(
                "overlay passthrough={} · {remaining}s",
                self.options.passthrough
            ),
            egui::FontId::proportional(15.0),
            egui::Color32::from_rgb(0, 229, 255),
        );

        // Keep the overlay raised above the witness for the whole probe.
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Orchestrate,
    Witness,
    Overlay,
}

#[derive(Debug, Clone)]
struct ProbeOptions {
    role: Role,
    width: f32,
    height: f32,
    position: Option<[f32; 2]>,
    seconds: u64,
    transparent: bool,
    passthrough: bool,
    hit_file: Option<PathBuf>,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            role: Role::Orchestrate,
            width: 420.0,
            height: 320.0,
            // Offset from the corner so window managers are less likely to
            // refuse the map; center of this rect is the XTEST target.
            position: Some([160.0, 160.0]),
            seconds: 8,
            transparent: true,
            passthrough: true,
            hit_file: None,
        }
    }
}

impl ProbeOptions {
    /// `--witness | --overlay | (default orchestrate)`
    /// `--size WxH --at X,Y --seconds N --hit-file PATH --opaque --no-passthrough`
    fn from_args() -> Self {
        let mut options = Self::default();
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--witness" => options.role = Role::Witness,
                "--overlay" => options.role = Role::Overlay,
                "--size" => {
                    if let Some((width, height)) =
                        arguments.next().and_then(|v| parse_pair(&v, 'x'))
                    {
                        options.width = width;
                        options.height = height;
                    }
                }
                "--at" => {
                    options.position = arguments
                        .next()
                        .and_then(|v| parse_pair(&v, ','))
                        .map(|(x, y)| [x, y]);
                }
                "--seconds" => {
                    if let Some(seconds) = arguments.next().and_then(|v| v.parse().ok()) {
                        options.seconds = seconds;
                    }
                }
                "--hit-file" => {
                    options.hit_file = arguments.next().map(PathBuf::from);
                }
                "--opaque" => options.transparent = false,
                "--no-passthrough" => options.passthrough = false,
                other => eprintln!("passthrough-probe: ignoring unknown argument `{other}`"),
            }
        }
        options
    }
}

fn parse_pair(value: &str, separator: char) -> Option<(f32, f32)> {
    let (left, right) = value.split_once(separator)?;
    Some((left.trim().parse().ok()?, right.trim().parse().ok()?))
}
