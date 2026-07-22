//! The binary: boot eframe, register hubs (one line each — Law 4), wire the
//! fabric, and carry the self-verification harness.
//!
//! `devpulselabs --screenshot out.png` boots the real app, paints a few
//! frames, captures its own window to PNG, and exits — Law 2 (test the real
//! artifact) with no human in the loop.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dpl_fabric::Fabric;
use dpl_hub::{Hub, Route};
use dpl_shell::Shell;

fn main() -> eframe::Result {
    // Structured logs: `RUST_LOG=debug devpulselabs` — info by default.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let args: Vec<String> = std::env::args().collect();
    let shot: Option<std::path::PathBuf> = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from);
    // `--route hub/sub` — boot anywhere, so every surface can prove itself.
    let boot_route: Option<(String, String)> = args
        .iter()
        .position(|a| a == "--route")
        .and_then(|i| args.get(i + 1))
        .and_then(|r| {
            r.split_once('/')
                .map(|(h, s)| (h.to_string(), s.to_string()))
        });
    // `--focus <template-id>` — boot the Library gallery solo'd on one
    // template, for per-template proof shots.
    let focus: Option<String> = args
        .iter()
        .position(|a| a == "--focus")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--shot-frame <n>` — capture later than the default frame 6, so sweep
    // animations can reach their rest state before the proof is taken.
    let shot_frame: u32 = args
        .iter()
        .position(|a| a == "--shot-frame")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok())
        .unwrap_or(6);
    // `--pixvix` opens the overlay at boot; `--pixvix-snap` fires a real
    // Img/Full snip through the palette's own handler and exits.
    let pixvix_open = args.iter().any(|a| a == "--pixvix");
    // `--mood <n>` pins the boot mood so backdrop proof shots reproduce.
    let mood: Option<usize> = args
        .iter()
        .position(|a| a == "--mood")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok());
    let pixvix_snap = args.iter().any(|a| a == "--pixvix-snap");

    let options = eframe::NativeOptions {
        // MAXIMIZED — a fixed logical size overflows small displays at
        // dpr > 1 and shoves the status bar under the taskbar (the v1 bug).
        // The inner size only names the restored-state fallback.
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "DevPULSE Labs",
        options,
        Box::new(move |cc| {
            dpl_coldlight::apply_theme(&cc.egui_ctx);
            // Media blocks: file:// images resolve through these loaders.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(
                shot,
                boot_route,
                focus,
                shot_frame,
                pixvix_open,
                pixvix_snap,
                mood,
            )))
        }),
    )
}

struct App {
    shell: Shell,
    pixvix: dpl_pixvix::Pixvix,
    shot: Option<std::path::PathBuf>,
    frames: u32,
    shot_frame: u32,
    pixvix_snap: bool,
    snap_frames: u32,
}

impl App {
    fn new(
        shot: Option<std::path::PathBuf>,
        boot_route: Option<(String, String)>,
        focus: Option<String>,
        shot_frame: u32,
        pixvix_open: bool,
        pixvix_snap: bool,
        mood: Option<usize>,
    ) -> Self {
        let mut library = dpl_hub_library::LibraryHub::default();
        if let Some(id) = focus {
            library.set_focus(id);
        }
        // The registry — a new hub is ONE line here and nothing else.
        let hubs: Vec<Box<dyn Hub>> = vec![
            Box::new(dpl_hub_home::HomeHub),
            Box::new(dpl_hub_workbench::WorkbenchHub),
            Box::new(library),
            Box::new(dpl_hub_wiredfront::WiredfrontHub),
            Box::new(dpl_hub_chat::ChatHub),
        ];

        // ONE fabric, shared: the shell publishes, the overlay subscribes.
        let fabric = Fabric::new();
        let mut pixvix = dpl_pixvix::Pixvix::new(&fabric);
        if pixvix_open {
            pixvix.open_now();
        }
        let mut shell = Shell::new(hubs, fabric);
        // The provenance stamp (git hash · utc, build.rs) — the running app
        // proves which code it is, in the status grid's build cell.
        shell.set_build(env!("DPL_BUILD"));
        if let Some(m) = mood {
            shell.backdrop_mut().set_mood(m);
        }
        match boot_route {
            // The harness route: HubId/SubId are &'static str newtypes, so the
            // CLI strings are leaked once — a harness-only allocation.
            Some((h, s)) => {
                let route = Route {
                    hub: dpl_hub::HubId(Box::leak(h.into_boxed_str())),
                    sub: dpl_hub::SubId(Box::leak(s.into_boxed_str())),
                };
                shell.navigate(route);
            }
            // Default boot: wiredFRONT with the rail extended.
            None => shell.navigate(Route {
                hub: dpl_hub_wiredfront::HUB,
                sub: dpl_hub_wiredfront::PIXVIX,
            }),
        }

        Self {
            shell,
            pixvix,
            shot,
            frames: 0,
            shot_frame,
            pixvix_snap,
            snap_frames: 0,
        }
    }

    /// The harness: request a screenshot on `shot_frame` (default 6 — theme +
    /// fonts settled; pass a later frame to let sweeps reach rest first),
    /// save it when the event arrives, exit.
    fn screenshot_harness(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot.clone() else {
            return;
        };

        self.frames += 1;
        if self.frames == self.shot_frame {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        let captured = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });

        if let Some(img) = captured {
            let [w, h] = img.size;
            let mut rgba = Vec::with_capacity(w * h * 4);
            for px in &img.pixels {
                rgba.extend_from_slice(&px.to_array());
            }
            match image::RgbaImage::from_raw(w as u32, h as u32, rgba).map(|buf| buf.save(&path)) {
                Some(Ok(())) => {
                    eprintln!("screenshot written: {}", path.display());
                    std::process::exit(0);
                }
                other => {
                    eprintln!("screenshot FAILED: {other:?}");
                    std::process::exit(1);
                }
            }
        }

        // Keep frames flowing while we wait for the capture to come back.
        ctx.request_repaint();
        if self.frames > self.shot_frame + 300 {
            eprintln!("screenshot never arrived after 300 frames");
            std::process::exit(1);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui);
        let ctx = ui.ctx().clone();

        // The Steps chords, app-focused scope (OS-global is on the ledger):
        // Ctrl+Shift+9 toggles a guide recording, Ctrl+Shift+X marks.
        let (toggle, mark) = ctx.input(|i| {
            let chord = i.modifiers.ctrl && i.modifiers.shift;
            (
                chord && i.key_pressed(egui::Key::Num9),
                chord && i.key_pressed(egui::Key::X),
            )
        });
        if toggle {
            self.pixvix.toggle_steps();
        }
        if mark {
            self.pixvix.mark_step();
        }

        self.pixvix.ui(&ctx);

        // F12: egui's native inspection overlay — the UI debugger.
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            let open = ctx.data_mut(|d| {
                let v = d.get_temp_mut_or(egui::Id::new("dpl-inspect"), false);
                *v = !*v;
                *v
            });
            tracing::info!(open, "inspection overlay toggled");
        }
        if ctx.data(|d| d.get_temp(egui::Id::new("dpl-inspect")).unwrap_or(false)) {
            egui::Window::new("inspection")
                .default_size([360.0, 480.0])
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| ctx.inspection_ui(ui));
                });
        }

        // `--pixvix-snap`: after a few settle frames, fire the real Img/Full
        // handler, report, exit by its own verdict.
        if self.pixvix_snap {
            self.snap_frames += 1;
            if self.snap_frames == 30 {
                let msg = self.pixvix.snap_full_now().to_string();
                eprintln!("pixvix-snap: {msg}");
                std::process::exit(if msg.starts_with("saved") { 0 } else { 1 });
            }
            ctx.request_repaint();
        }

        self.screenshot_harness(&ctx);
    }
}
