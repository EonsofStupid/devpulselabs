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
        // Stdout is reserved for EguiPilotAI's private framed transport in a
        // devtools launch. Keeping diagnostics on stderr also makes run
        // bundles complete without corrupting protocol bytes.
        .with_writer(std::io::stderr)
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
    // `--focus <template-id>` — boot the Workbench Library's TEMPLATES
    // section solo'd on one template, for per-template proof shots.
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
    // `--session-drill <secs>` — start a REAL session through the
    // palette's own handler, hop routes (observed fabric traffic), stop
    // after the window, exit on the recorder's own verdict (S1).
    let session_drill: Option<u32> = args
        .iter()
        .position(|a| a == "--session-drill")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok());
    // `--present <episode-id>` boots straight into the show, full-bleed —
    // the deck surface proves itself on the real exe.
    let present: Option<String> = args
        .iter()
        .position(|a| a == "--present")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--presenter-drill <n>` — with `--present`, open the teleprompter
    // in PANEL mode (main-viewport screenshots) after walking the deck
    // n beats through its real key path (D4 proofs).
    let presenter_drill: Option<usize> = args
        .iter()
        .position(|a| a == "--presenter-drill")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok());
    // `--open-session <rel>` boots the SESSIONS player on one session
    // (S2) — the same code path a card tap takes.
    let open_session: Option<String> = args
        .iter()
        .position(|a| a == "--open-session")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--open-episode <id>` boots the Workbench episode wall on one
    // episode (D5 proofs) — the same code path a picker tap takes.
    let open_episode: Option<String> = args
        .iter()
        .position(|a| a == "--open-episode")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--open-replay <rel>` boots the GALLERY's in-place session player
    // (L1e) — the same code path a session-thumb tap takes.
    let open_replay: Option<String> = args
        .iter()
        .position(|a| a == "--open-replay")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--open-rail <hub>` pins the accordion open on one house (L1a) —
    // proof shots of the unfolded rail reproduce. Unknown hub degrades to
    // a folded rail, never a panic.
    let open_rail: Option<String> = args
        .iter()
        .position(|a| a == "--open-rail")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--clone-seed <seed-id> <show> <episode>` — the headless clone
    // drill (X3): copy a seed into the real content tree through the
    // engine (`dpl_doc::clone_episode`) and exit; no window boots.
    if let Some(i) = args.iter().position(|a| a == "--clone-seed") {
        let (Some(seed), Some(show), Some(ep)) =
            (args.get(i + 1), args.get(i + 2), args.get(i + 3))
        else {
            eprintln!("usage: --clone-seed <seed-id> <show> <episode>");
            std::process::exit(2);
        };
        let Some((_, dir)) = dpl_doc::list_episodes()
            .into_iter()
            .find(|(e, _)| e == seed)
        else {
            eprintln!("unknown seed episode {seed:?}");
            std::process::exit(2);
        };
        match dpl_doc::clone_episode(&dir, &dpl_engine_capture::content_root(), show, ep) {
            Ok(dest) => {
                println!("cloned {seed} → {}", dest.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }

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
    #[cfg(feature = "eguipilotai-devtools")]
    let options = configure_pilot_adapter(options);

    eframe::run_native(
        "DevPULSE Labs",
        options,
        Box::new(move |cc| {
            dpl_coldlight::apply_theme(&cc.egui_ctx);
            // Semantic instrumentation is inert unless a test/devtools runtime
            // is explicitly compiled and privately launched.
            egui_pilot_ai::install(&cc.egui_ctx);
            // Media blocks: file:// images resolve through these loaders.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let app = App::new(Boot {
                shot,
                boot_route,
                focus,
                shot_frame,
                pixvix_open,
                pixvix_snap,
                mood,
                present,
                presenter_drill,
                session_drill,
                open_session,
                open_replay,
                open_rail,
                open_episode,
            });
            #[cfg(feature = "eguipilotai-devtools")]
            let app = {
                let mut app = app;
                app.attach_pilot_runtime(cc);
                app
            };
            Ok(Box::new(app))
        }),
    )
}

/// The harness/boot flags, parsed once in `main` — one value, not a
/// nine-argument constructor.
struct Boot {
    shot: Option<std::path::PathBuf>,
    boot_route: Option<(String, String)>,
    focus: Option<String>,
    shot_frame: u32,
    pixvix_open: bool,
    pixvix_snap: bool,
    mood: Option<usize>,
    present: Option<String>,
    presenter_drill: Option<usize>,
    session_drill: Option<u32>,
    open_session: Option<String>,
    open_replay: Option<String>,
    open_rail: Option<String>,
    open_episode: Option<String>,
}

struct App {
    shell: Shell,
    pixvix: dpl_pixvix::Pixvix,
    shot: Option<std::path::PathBuf>,
    frames: u32,
    shot_frame: u32,
    pixvix_snap: bool,
    snap_frames: u32,
    session_drill: Option<u32>,
    drill_frames: u32,
    drill_started: Option<std::time::Instant>,
    #[cfg(feature = "eguipilotai-devtools")]
    pilot_runtime: Option<egui_pilot_ai_runtime::RuntimeGuard>,
}

impl App {
    fn new(boot: Boot) -> Self {
        let Boot {
            shot,
            boot_route,
            focus,
            shot_frame,
            pixvix_open,
            pixvix_snap,
            mood,
            present,
            presenter_drill,
            session_drill,
            open_session,
            open_replay,
            open_rail,
            open_episode,
        } = boot;
        let mut workbench = dpl_hub_workbench::WorkbenchHub::default();
        if let Some(id) = focus {
            workbench.set_focus(id);
        }
        // The episode-wall seam (D5 proofs).
        if let Some(id) = &open_episode {
            workbench.open_episode(id);
        }
        // ONE fabric, shared: the shell publishes AND (since D2) listens;
        // the Present hub requests full-bleed over it.
        let fabric = Fabric::new();
        let mut present_hub = dpl_hub_present::PresentHub::new(fabric.clone());
        if let Some(id) = &present {
            present_hub.open_show(id);
            // The teleprompter drill (D4 proofs) rides an open show.
            if let Some(n) = presenter_drill {
                present_hub.presenter_drill(n);
            }
        }
        // The harness path into the SESSIONS player (S2).
        let mut wiredfront = dpl_hub_wiredfront::WiredfrontHub::default();
        if let Some(rel) = &open_session {
            wiredfront.open_session(rel);
        }
        // The gallery's in-place replay seam (L1e).
        if let Some(rel) = &open_replay {
            wiredfront.open_replay(rel);
        }
        // The registry — a new hub is ONE line here and nothing else.
        let hubs: Vec<Box<dyn Hub>> = vec![
            Box::new(dpl_hub_home::HomeHub),
            // The doctrine map (§3): the operator's content rides right
            // under Home. Shows are discovered once at boot.
            Box::new(dpl_hub_shows::ShowsHub::new()),
            Box::new(present_hub),
            Box::new(workbench),
            Box::new(wiredfront),
            Box::new(dpl_hub_chat::ChatHub),
        ];

        let mut pixvix = dpl_pixvix::Pixvix::new(&fabric);
        if pixvix_open {
            pixvix.open_now();
        }
        let mut shell = Shell::new(hubs, fabric);
        // The provenance stamp (git hash · utc, build.rs) — the running app
        // proves which code it is, in the status grid's build cell.
        shell.set_build(env!("DPL_BUILD"));
        if let Some(m) = mood {
            shell.set_harness_mood(m);
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
            // Default boot: wiredFRONT, the prep house (rail boots folded).
            None => shell.navigate(Route {
                hub: dpl_hub_wiredfront::HUB,
                sub: dpl_hub_wiredfront::PIXVIX,
            }),
        }
        // `--open-rail <hub>` — the accordion pinned open (L1a proofs).
        // The CLI string is leaked once, like the boot route's.
        if let Some(h) = open_rail {
            shell.set_open_house(Some(dpl_hub::HubId(Box::leak(h.into_boxed_str()))));
        }
        if present.is_some() {
            // The harness path: the hub opened its deck before the shell
            // subscribed to the fabric, so full-bleed is set directly.
            shell.navigate(Route {
                hub: dpl_hub_present::HUB,
                sub: dpl_hub_present::SHOWS,
            });
            shell.set_full_bleed(true);
        }

        Self {
            shell,
            pixvix,
            shot,
            frames: 0,
            shot_frame,
            pixvix_snap,
            snap_frames: 0,
            session_drill,
            drill_frames: 0,
            drill_started: None,
            #[cfg(feature = "eguipilotai-devtools")]
            pilot_runtime: None,
        }
    }

    #[cfg(feature = "eguipilotai-devtools")]
    fn attach_pilot_runtime(&mut self, cc: &eframe::CreationContext<'_>) {
        if std::env::var_os("EGUIPILOTAI_STDIO").as_deref() != Some(std::ffi::OsStr::new("1")) {
            return;
        }
        let Some(session) = std::env::var("EGUIPILOTAI_SESSION").ok() else {
            tracing::error!("EGUIPILOTAI_SESSION is required for private test launch");
            return;
        };
        let Some(session_nonce) = std::env::var("EGUIPILOTAI_NONCE").ok() else {
            tracing::error!("EGUIPILOTAI_NONCE is required for private test launch");
            return;
        };
        let Some(artifact_root) = std::env::var_os("EGUIPILOTAI_ARTIFACT_ROOT") else {
            tracing::error!("EGUIPILOTAI_ARTIFACT_ROOT is required for private test launch");
            return;
        };
        let mut config = egui_pilot_ai_runtime::RuntimeConfig::new(
            env!("DPL_BUILD"),
            std::path::PathBuf::from(artifact_root),
        );
        config.session.0 = session;
        config.session_nonce = session_nonce;
        config.max_commands_per_frame = 32;
        config.renderer_identity = cc.wgpu_render_state.as_ref().map(|state| {
            let info = state.adapter.get_info();
            egui_pilot_ai_runtime::protocol::RendererIdentity {
                os: std::env::consts::OS.into(),
                backend: format!("{:?}", info.backend),
                adapter: info.name,
                device_type: format!("{:?}", info.device_type),
                driver: format!("{} {}", info.driver, info.driver_info),
                renderer_version: "egui-wgpu-0.35".into(),
                surface_format: format!("{:?}", state.target_format),
                pixels_per_point: cc.egui_ctx.pixels_per_point(),
                font_bundle: "devpulse-coldlight-v1".into(),
                dithering: false,
            }
        });
        self.pilot_runtime = Some(egui_pilot_ai_runtime::RuntimeGuard::attach(
            &cc.egui_ctx,
            egui_pilot_ai_transport::StdioTransport::stdio(64),
            config,
        ));
        tracing::info!("EguiPilotAI private devtools runtime attached");
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

#[cfg(feature = "eguipilotai-devtools")]
fn configure_pilot_adapter(mut options: eframe::NativeOptions) -> eframe::NativeOptions {
    use egui_wgpu::wgpu::DeviceType;
    use std::sync::Arc;

    let Some(policy) = std::env::var("EGUIPILOTAI_ADAPTER_POLICY").ok() else {
        return options;
    };
    let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup else {
        return options;
    };
    setup.native_adapter_selector = Some(Arc::new(move |adapters, surface| {
        let compatible = |adapter: &&egui_wgpu::wgpu::Adapter| {
            surface.is_none_or(|surface| adapter.is_surface_supported(surface))
        };
        let software = adapters
            .iter()
            .filter(compatible)
            .find(|adapter| adapter.get_info().device_type == DeviceType::Cpu);
        let hardware = adapters
            .iter()
            .filter(compatible)
            .find(|adapter| adapter.get_info().device_type != DeviceType::Cpu);
        let selected = match policy.as_str() {
            "software-required" => software,
            "hardware-required" => hardware,
            "hardware-preferred" | "hardware-then-software" => hardware.or(software),
            other => return Err(format!("unknown EguiPilotAI adapter policy `{other}`")),
        };
        selected.cloned().ok_or_else(|| {
            format!("adapter policy `{policy}` could not be satisfied by any compatible adapter")
        })
    }));
    options
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
                let msg = self.pixvix.snap_full_now(&ctx).to_string();
                eprintln!("pixvix-snap: {msg}");
                std::process::exit(if msg.starts_with("saved") { 0 } else { 1 });
            }
            ctx.request_repaint();
        }

        // `--session-drill <secs>`: the S1 drill — the real session
        // handler, two real route hops (the shell PUBLISHES RouteChanged;
        // the recorder only ever observes), stop after the window, exit
        // on the recorder's own status verdict.
        if let Some(secs) = self.session_drill {
            self.drill_frames += 1;
            match self.drill_frames {
                30 => {
                    let msg = self.pixvix.session_start_now().to_string();
                    eprintln!("session-drill: {msg}");
                    if !msg.starts_with("session recording") {
                        std::process::exit(1);
                    }
                    self.drill_started = Some(std::time::Instant::now());
                }
                60 => self.shell.navigate(Route {
                    hub: dpl_hub_workbench::HUB,
                    sub: dpl_hub_workbench::LIBRARY,
                }),
                90 => self.shell.navigate(Route {
                    hub: dpl_hub_present::HUB,
                    sub: dpl_hub_present::SHOWS,
                }),
                _ => {}
            }
            if let Some(t0) = self.drill_started {
                if self.pixvix.session_active() && t0.elapsed().as_secs() >= u64::from(secs) {
                    self.pixvix.session_stop_now();
                }
                if !self.pixvix.session_active() {
                    let msg = self.pixvix.status_text().to_string();
                    eprintln!("session-drill: {msg}");
                    std::process::exit(if msg.starts_with("session saved") {
                        0
                    } else {
                        1
                    });
                }
            }
            ctx.request_repaint();
        }

        self.screenshot_harness(&ctx);

        #[cfg(feature = "eguipilotai-devtools")]
        if let Some(runtime) = &mut self.pilot_runtime {
            runtime.pump();
        }
    }
}
