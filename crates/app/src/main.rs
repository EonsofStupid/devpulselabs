//! The binary: boot eframe, register hubs (one line each — Law 4), wire the
//! fabric, and carry the self-verification harness.
//!
//! `devpulselabs --screenshot out.png` boots the real app, paints a few
//! frames, captures its own window to PNG, and exits — Law 2 (test the real
//! artifact) with no human in the loop.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod chat;

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
    // `--viziwiz` opens the overlay at boot; `--viziwiz-snap` fires a real
    // Img/Full snip through the palette's own handler and exits.
    let viziwiz_open = args.iter().any(|a| a == "--viziwiz");
    // `--mood <n>` pins the boot mood so backdrop proof shots reproduce.
    let mood: Option<usize> = args
        .iter()
        .position(|a| a == "--mood")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok());
    let viziwiz_snap = args.iter().any(|a| a == "--viziwiz-snap");
    // `--chat-open` opens the independent primary Chat peer at boot. The
    // named EguiPilotAI fixture uses this path; ordinary launches stay quiet.
    let chat_open = args.iter().any(|a| a == "--chat-open");
    // `--steps-hotkey-drill` — send a REAL Ctrl+Shift+9 through the OS
    // (rdev::simulate) and verify the global hook starts, then stops, the
    // Steps recorder (C2). Real input end-to-end; the pilot never fakes
    // OS hooks.
    let steps_hotkey_drill = args.iter().any(|a| a == "--steps-hotkey-drill");
    // `--video-drill <secs>` — record the screen for real through the
    // palette's own handler, stop, and exit on the reported status (C3).
    // `--video-format <f>` pins the format so the matrix is drillable.
    let video_drill: Option<u32> = args
        .iter()
        .position(|a| a == "--video-drill")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok());
    let video_format: Option<String> = args
        .iter()
        .position(|a| a == "--video-format")
        .and_then(|i| args.get(i + 1))
        .cloned();
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
    // `--import-drill <dir>` (W2) imports every media file in the staged
    // folder at boot through the SAME seam an Explorer drop takes —
    // copy to the content tree, poster, index write-through, videos onto
    // the timeline. The real-exe proof of the import path.
    let import_drill: Option<String> = args
        .iter()
        .position(|a| a == "--import-drill")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--pin-pack <dir>` (repeatable, W3) pins a folder as a pack source
    // on the Elements shelf at boot — the SAME seam the Browse window's
    // Pin pack chip takes. Nothing is copied; the folder is listed in
    // place.
    let pin_packs: Vec<String> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| *a == "--pin-pack")
        .filter_map(|(i, _)| args.get(i + 1))
        .cloned()
        .collect();
    // `--open-rail <hub>` pins the accordion open on one house (L1a) —
    // proof shots of the unfolded rail reproduce. Unknown hub degrades to
    // a folded rail, never a panic.
    let open_rail: Option<String> = args
        .iter()
        .position(|a| a == "--open-rail")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // The AGENT SEAM (R1v): localhost HTTP over the fabric so chat can
    // observe and drive the app. `--seam-port <n>` moves it; `--no-seam`
    // turns it off. A busy port logs and degrades — never a boot failure.
    let seam_port: Option<u16> = if args.iter().any(|a| a == "--no-seam") {
        None
    } else {
        Some(
            args.iter()
                .position(|a| a == "--seam-port")
                .and_then(|i| args.get(i + 1))
                .and_then(|n| n.parse().ok())
                .unwrap_or(7341),
        )
    };
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
        // `--window <w>x<h>` boots RESTORED at that size — proofs must
        // run at real window sizes, not only maximized (the footer bug
        // hid behind maximized-only screenshots).
        viewport: {
            let all: Vec<String> = std::env::args().collect();
            let restored: Option<(f32, f32)> = all
                .iter()
                .position(|a| a == "--window")
                .and_then(|i| all.get(i + 1))
                .and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                });
            // Min small enough for scaled displays (the shell's zone test
            // pins chrome at ANY size); the fit guard owns the rest.
            let builder = egui::ViewportBuilder::default().with_min_inner_size([640.0, 480.0]);
            match restored {
                Some((w, h)) => builder.with_maximized(false).with_inner_size([w, h]),
                None => builder
                    .with_maximized(true)
                    .with_inner_size([1440.0, 900.0]),
            }
        },
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
                viziwiz_open,
                viziwiz_snap,
                chat_open,
                steps_hotkey_drill,
                video_drill,
                video_format,
                mood,
                present,
                presenter_drill,
                session_drill,
                open_session,
                open_replay,
                open_rail,
                open_episode,
                import_drill,
                pin_packs,
                seam_port,
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
    viziwiz_open: bool,
    viziwiz_snap: bool,
    chat_open: bool,
    steps_hotkey_drill: bool,
    video_drill: Option<u32>,
    video_format: Option<String>,
    mood: Option<usize>,
    present: Option<String>,
    presenter_drill: Option<usize>,
    session_drill: Option<u32>,
    open_session: Option<String>,
    open_replay: Option<String>,
    open_rail: Option<String>,
    open_episode: Option<String>,
    import_drill: Option<String>,
    pin_packs: Vec<String>,
    /// The agent seam's port; None = `--no-seam`.
    seam_port: Option<u16>,
}

struct App {
    shell: Shell,
    viziwiz: dpl_viziwiz::Viziwiz,
    chat: chat::ChatWindowCoordinator,
    shot: Option<std::path::PathBuf>,
    frames: u32,
    shot_frame: u32,
    viziwiz_snap: bool,
    snap_frames: u32,
    steps_hotkey_drill: bool,
    hotkey_frames: u32,
    hotkey_phase: u8,
    hotkey_mark: u32,
    video_drill: Option<u32>,
    video_format: Option<String>,
    video_frames: u32,
    video_started: Option<std::time::Instant>,
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
            viziwiz_open,
            viziwiz_snap,
            chat_open,
            steps_hotkey_drill,
            video_drill,
            video_format,
            mood,
            present,
            presenter_drill,
            session_drill,
            open_session,
            open_replay,
            open_rail,
            open_episode,
            import_drill,
            pin_packs,
            seam_port,
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
        // The import seam's real-exe drill (W2): the SAME path an
        // Explorer drop takes, headlessly staged.
        if let Some(dir) = &import_drill {
            wiredfront.import_drill(std::path::Path::new(dir));
        }
        // The pack shelf's harness seam (W3): pin sources at boot.
        for dir in &pin_packs {
            wiredfront.pin_pack(std::path::Path::new(dir));
        }
        // The NAVIGATE seam: hubs that render tappable subroute cards
        // publish their asks through the one fabric.
        workbench.wire_fabric(&fabric);
        wiredfront.wire_fabric(&fabric);
        // Chat providers register as data. The peer-window UI never names one;
        // every manifest remains visibly blocked until its real adapter
        // qualifies. Chat is ONLY the status-bar peer window now — no rail hub.
        let chat_runtime = dpl_chat_runtime::ChatRuntime::try_from_registrations([
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_codex::MANIFEST),
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_claude::MANIFEST),
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_grok::MANIFEST),
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_gemini::MANIFEST),
        ])
        .unwrap_or_else(|error| {
            tracing::error!(%error, "chat provider registry rejected");
            dpl_chat_runtime::ChatRuntime::default()
        });
        // The registry — a new hub is ONE line here and nothing else.
        let hubs: Vec<Box<dyn Hub>> = vec![
            Box::new(dpl_hub_home::HomeHub),
            // The doctrine map (§3): the operator's content rides right
            // under Home. Shows are discovered once at boot.
            Box::new(dpl_hub_shows::ShowsHub::new()),
            Box::new(present_hub),
            Box::new(workbench),
            Box::new(wiredfront),
        ];
        // The lite route launcher's targets: each registered hub's label and
        // its home route (HubSpec::default_route → first subroute). Built here,
        // before `hubs` is moved into the shell, and handed to the peer window.
        let chat_routes: Vec<(String, Route)> = hubs
            .iter()
            .map(|hub| {
                let spec = hub.spec();
                (spec.label.to_owned(), spec.default_route())
            })
            .collect();
        // The AGENT SEAM (R1v): the same registered specs become the HTTP
        // route registry — the seam can never name a surface the app lacks.
        if let Some(port) = seam_port {
            let registry = dpl_seam::Registry::new(hubs.iter().map(|h| h.spec()).collect());
            dpl_seam::spawn(
                fabric.clone(),
                registry,
                dpl_engine_capture::content_root(),
                port,
            );
        }

        let mut viziwiz = dpl_viziwiz::Viziwiz::new(&fabric);
        if viziwiz_open {
            viziwiz.open_now();
        }
        let mut shell = Shell::new(hubs, fabric.clone());
        shell
            .register_window_accessory(dpl_shell::status::WindowAccessory {
                key: dpl_chat_window::WINDOW_KEY,
                pilot_id: dpl_chat_window::LAUNCHER_ID,
                glyph: dpl_coldlight::icon::BOT,
                label: "Chat",
                tone: dpl_coldlight::Tone::Cyan,
                tooltip: "Open, restore, or focus the DevPULSE Chat peer window",
            })
            .unwrap_or_else(|key| panic!("duplicate shell window accessory: {key}"));
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
                sub: dpl_hub_wiredfront::VIZIWIZ,
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
        let chat = chat::ChatWindowCoordinator::new(
            &fabric,
            chat_runtime,
            shell.route(),
            local_chat_history_catalog(),
            chat_routes,
            chat_open,
        );

        Self {
            shell,
            viziwiz,
            chat,
            shot,
            frames: 0,
            shot_frame,
            viziwiz_snap,
            snap_frames: 0,
            steps_hotkey_drill,
            hotkey_frames: 0,
            hotkey_phase: 0,
            hotkey_mark: 0,
            video_drill,
            video_format,
            video_frames: 0,
            video_started: None,
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

fn local_chat_history_catalog() -> dpl_chat_store::HistoryCatalog {
    let mut catalog = dpl_chat_store::HistoryCatalog::default();
    #[cfg(windows)]
    let user_profile = std::env::var_os("USERPROFILE").map(std::path::PathBuf::from);
    #[cfg(not(windows))]
    let user_profile = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let Some(user_profile) = user_profile else {
        tracing::warn!("Chat history disabled: user profile root is unavailable");
        return catalog;
    };

    let codex_root = user_profile.join(".codex").join("sessions");
    if codex_root.is_dir() {
        match dpl_chat_store::JsonlHistorySource::open(
            codex_root,
            dpl_chat_codex::CodexHistoryDecoder,
            dpl_chat_store::HistoryLimits::default(),
        ) {
            Ok(source) => {
                if let Err(error) = catalog.register(source) {
                    tracing::warn!(%error, "Codex history source rejected");
                }
            }
            Err(error) => tracing::warn!(%error, "Codex history source unavailable"),
        }
    }

    let claude_root = user_profile.join(".claude").join("projects");
    if claude_root.is_dir() {
        match dpl_chat_store::JsonlHistorySource::open(
            claude_root,
            dpl_chat_claude::ClaudeHistoryDecoder,
            dpl_chat_store::HistoryLimits::default(),
        ) {
            Ok(source) => {
                if let Err(error) = catalog.register(source) {
                    tracing::warn!(%error, "Claude history source rejected");
                }
            }
            Err(error) => tracing::warn!(%error, "Claude history source unavailable"),
        }
    }

    catalog
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

/// The taskbar law, continuous (old repo's `lib.rs`, ported): whenever the
/// RESTORED window exceeds the monitor's usable area — boot on a scaled
/// display, a DPI change, a drag across monitors — shrink the inner size
/// (minus the decoration delta) and nudge the window fully on-screen, so
/// the footer is always physically visible. Maximized/fullscreen already
/// respect the work area and are left alone.
fn fit_window(ctx: &egui::Context) {
    // Decide under the input lock; SEND after releasing it.
    let (resize, reposition) = ctx.input(|i| {
        let v = i.viewport();
        let (Some(monitor), Some(outer), Some(inner)) =
            (v.monitor_size, v.outer_rect, v.inner_rect)
        else {
            return (None, None);
        };
        if v.maximized.unwrap_or(false) || v.fullscreen.unwrap_or(false) {
            return (None, None);
        }
        // egui exposes the monitor, not the work area — allow a
        // conservative 48 logical px for the taskbar/panel.
        let work = egui::vec2(monitor.x, monitor.y - 48.0);
        let decor = outer.size() - inner.size();
        let max_inner = work - decor;
        let mut size = inner.size();
        let mut resize = None;
        if size.x > max_inner.x || size.y > max_inner.y {
            size.x = size.x.min(max_inner.x).max(600.0);
            size.y = size.y.min(max_inner.y).max(400.0);
            resize = Some(size);
        }
        let fitted = resize.map_or(outer.size(), |s| s + decor);
        let mut pos = outer.min;
        pos.x = pos.x.min(work.x - fitted.x).max(0.0);
        pos.y = pos.y.min(work.y - fitted.y).max(0.0);
        let reposition = (pos != outer.min).then_some(pos);
        (resize, reposition)
    });
    if let Some(size) = resize {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    }
    if let Some(pos) = reposition {
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.chat.root_close_requested(ui.ctx());
        fit_window(ui.ctx());
        self.shell.ui(ui);
        let ctx = ui.ctx().clone();
        self.chat.frame(&ctx, self.shell.route());

        // The Steps chords (Ctrl+Shift+S/9/X) are OS-GLOBAL since C2 —
        // the rdev hook inside viziwiz sees them whether or not this app
        // has focus, so no egui input handling here (it would double-fire).
        self.viziwiz.ui(&ctx);

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

        // `--viziwiz-snap`: after a few settle frames, fire the real Img/Full
        // handler, report, exit by its own verdict.
        if self.viziwiz_snap {
            self.snap_frames += 1;
            if self.snap_frames == 30 {
                let msg = self.viziwiz.snap_full_now(&ctx).to_string();
                eprintln!("viziwiz-snap: {msg}");
                std::process::exit(if msg.starts_with("saved") { 0 } else { 1 });
            }
            ctx.request_repaint();
        }

        // `--steps-hotkey-drill`: REAL Ctrl+Shift+9 through the OS →
        // rdev hook → chord → recorder. Phase 0 settles the hook thread,
        // then each chord flips the recorder and the drill watches the
        // real state. Exit by verdict; a hung phase times out honestly.
        if self.steps_hotkey_drill {
            self.hotkey_frames += 1;
            match self.hotkey_phase {
                0 if self.hotkey_frames >= 30 => {
                    self.hotkey_phase = 1;
                    std::thread::spawn(|| {
                        if let Err(e) = dpl_engine_steps::simulate_toggle_chord() {
                            eprintln!("steps-hotkey-drill: send failed: {e}");
                        }
                    });
                }
                1 if self.viziwiz.steps_recording() => {
                    eprintln!("steps-hotkey-drill: chord STARTED the recorder");
                    // Settle before the stop chord — two overlapping
                    // simulated sequences interleave their modifier
                    // releases and break the second chord (seen live).
                    self.hotkey_phase = 2;
                    self.hotkey_mark = self.hotkey_frames;
                }
                2 if self.hotkey_frames >= self.hotkey_mark + 60 => {
                    self.hotkey_phase = 3;
                    std::thread::spawn(|| {
                        if let Err(e) = dpl_engine_steps::simulate_toggle_chord() {
                            eprintln!("steps-hotkey-drill: send failed: {e}");
                        }
                    });
                }
                3 if !self.viziwiz.steps_recording() => {
                    eprintln!(
                        "steps-hotkey-drill: chord STOPPED the recorder · status: {}",
                        self.viziwiz.status_text()
                    );
                    std::process::exit(0);
                }
                _ => {}
            }
            if self.hotkey_frames > 600 {
                eprintln!(
                    "steps-hotkey-drill: TIMEOUT in phase {} · status: {}",
                    self.hotkey_phase,
                    self.viziwiz.status_text()
                );
                std::process::exit(1);
            }
            ctx.request_repaint();
        }

        // `--video-drill <secs>`: fire the REAL Record handler (C3 —
        // format pinned by `--video-format`), stop after the window,
        // exit on the recorder's own reported status.
        if let Some(secs) = self.video_drill {
            self.video_frames += 1;
            if self.video_frames == 30 {
                let format = self.video_format.clone();
                let msg = self.viziwiz.video_start_now(format.as_deref()).to_string();
                eprintln!("video-drill: {msg}");
                if !msg.starts_with("recording") {
                    std::process::exit(1);
                }
                self.video_started = Some(std::time::Instant::now());
            }
            if let Some(t0) = self.video_started {
                if self.viziwiz.video_active() && t0.elapsed().as_secs() >= u64::from(secs) {
                    self.viziwiz.video_stop_now();
                }
                if !self.viziwiz.video_active() {
                    let msg = self.viziwiz.status_text().to_string();
                    eprintln!("video-drill: {msg}");
                    std::process::exit(if msg.starts_with("saved") { 0 } else { 1 });
                }
                if t0.elapsed().as_secs() > u64::from(secs) + 60 {
                    eprintln!(
                        "video-drill: TIMEOUT · status: {}",
                        self.viziwiz.status_text()
                    );
                    std::process::exit(1);
                }
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
                    let msg = self.viziwiz.session_start_now().to_string();
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
                if self.viziwiz.session_active() && t0.elapsed().as_secs() >= u64::from(secs) {
                    self.viziwiz.session_stop_now();
                }
                if !self.viziwiz.session_active() {
                    let msg = self.viziwiz.status_text().to_string();
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
