//! The binary: boot eframe, register hubs (one line each — Law 4), wire the
//! fabric, and carry the self-verification harness.
//!
//! `devpulselabs --screenshot out.png` boots the real app, paints a few
//! frames, captures its own window to PNG, and exits — Law 2 (test the real
//! artifact) with no human in the loop.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai_backfill;
mod ai_usage;
mod chat;
#[cfg(feature = "eguipilotai-devtools")]
mod coach_fabric;
#[cfg(feature = "eguipilotai-devtools")]
mod coach_signals;
mod presence;

use dpl_fabric::Fabric;
use dpl_hub::{Hub, Route};
use dpl_shell::Shell;

/// Recover AI cost from sessions already on disk (Phase 3 §3.2c).
///
/// Only providers DECLARING `history_backfill` are walked, so a live-only
/// transport is excluded structurally rather than by a special case. A
/// missing history root is not an error — the operator simply has not run
/// that provider here.
fn run_ai_backfill(fabric: &dpl_fabric::Fabric, analytics_root: &std::path::Path) {
    use dpl_chat_store::{HistoryLimits, JsonlHistorySource};

    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        tracing::warn!("AI backfill: no HOME — cannot locate provider sessions");
        return;
    };
    // Each provider stores its sessions in its own well-known place.
    let codex_root = home.join(".codex/sessions");
    let claude_root = home.join(".claude/projects");

    let mut sources = Vec::new();
    let codex = codex_root.exists().then(|| {
        JsonlHistorySource::open(
            &codex_root,
            dpl_chat_codex::CodexHistoryDecoder,
            HistoryLimits::default(),
        )
    });
    let claude = claude_root.exists().then(|| {
        JsonlHistorySource::open(
            &claude_root,
            dpl_chat_claude::ClaudeHistoryDecoder,
            HistoryLimits::default(),
        )
    });
    if let Some(Ok(source)) = codex.as_ref() {
        if let Some(usage) = ai_usage::usage_for(dpl_chat_codex::PROVIDER) {
            sources.push((source, usage));
        }
    }
    if let Some(Ok(source)) = claude.as_ref() {
        if let Some(usage) = ai_usage::usage_for(dpl_chat_claude::PROVIDER) {
            sources.push((source, usage));
        }
    }
    if sources.is_empty() {
        tracing::info!("AI backfill: no provider sessions found on this machine");
        return;
    }
    match ai_backfill::run(fabric, analytics_root, &sources) {
        Ok(report) => tracing::info!(?report, "AI backfill complete"),
        Err(error) => tracing::warn!(%error, "AI backfill did not complete"),
    }
}

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
    // The user-data root, resolved ONCE by the ported portable law
    // (engine-capture) — logged with the full candidate list so "where
    // did my files go" always has an answer (content_status honesty).
    {
        let root = dpl_engine_capture::content_root();
        if root.exists() {
            tracing::info!(root = %root.display(), "user-data root");
        } else {
            tracing::warn!(
                root = %root.display(),
                checked = ?dpl_engine_capture::checked_roots(),
                "user-data root does not exist yet — created on first write; these locations were checked"
            );
        }
        // The three-ring root (ADR-014): assets/ + .userdata/ stand up
        // beside content/ at every boot, and the log NAMES all three.
        match dpl_engine_capture::ensure_rings() {
            Ok(r) => tracing::info!(
                content = %r.content.display(),
                assets = %r.assets.display(),
                userdata = %r.userdata.display(),
                "three-ring root ready"
            ),
            Err(e) => tracing::warn!(error = %e, "three-ring root could not be created"),
        }
        // Same honesty law, applied to AI accounting: name which providers
        // report usage and which do not, so an empty cost column is
        // explained rather than mysterious.
        crate::ai_usage::log_census();
    }
    // Managed ffmpeg (the original's tools.rs law, ported): resolve ONE
    // absolute binary at boot and inject it into every engine seam, so a
    // double-clicked exe with a stale Explorer PATH still previews,
    // probes, and renders. W6.
    init_ffmpeg();
    // Managed pdftoppm (same law): the staged poppler tree feeds PDF →
    // page-slide rasterization without touching PATH.
    init_pdftoppm();
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
    // `--lib-section <slug>` — boot the Workbench Library on ONE section by
    // its chip slug (seeds/templates/blocks/elements/themes/colors/presets/
    // fonts/moods), so every wall is provable on the real exe, not just
    // templates.
    let lib_section: Option<String> = args
        .iter()
        .position(|a| a == "--lib-section")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--author-editor` — mint a card on the Author surface and open its
    // overlay editor at boot. The overlay opens only on a tap, so this is how
    // it gets photographed; pair it with `--content-root` to write to scratch.
    let author_editor = args.iter().any(|a| a == "--author-editor");
    // `--card-tab <slug>` — open the card editor on one tab (words/background/
    // movement/quote/picture/on-tap/help). Words is the default, so this is
    // how any other tab gets photographed.
    let card_tab: Option<String> = args
        .iter()
        .position(|a| a == "--card-tab")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--author-container <kind>` — point the Author surface at one container
    // kind (lesson/guide/episode/deck), so a kind with nowhere to live yet can
    // be photographed saying so beside one that is real.
    let author_container: Option<String> = args
        .iter()
        .position(|a| a == "--author-container")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--author-draft <n>` — start n cards WITHOUT opening the editor, so the
    // filmstrip can be photographed with something in it rather than under an
    // overlay that covers it.
    let author_draft: usize = args
        .iter()
        .position(|a| a == "--author-draft")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
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
    // Authored Chat content is compile-time devtools-only. Production builds
    // cannot register fixture providers, people, messages, or interactions.
    #[cfg(feature = "eguipilotai-devtools")]
    let chat_authored = args.iter().any(|a| a == "--chat-authored");
    // Native proof presets are devtools-only and only arrange the deterministic
    // authored fixture. They cannot activate a provider or Social transport.
    #[cfg(feature = "eguipilotai-devtools")]
    let chat_proof = chat_authored.then(|| {
        args.iter()
            .position(|a| a == "--chat-proof")
            .and_then(|i| args.get(i + 1))
            .and_then(|value| chat::ChatProofMode::parse(value))
    });
    #[cfg(feature = "eguipilotai-devtools")]
    let chat_proof = chat_proof.flatten();
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
    // `--steps-drill <n>` (W5.3) — the AI-driven Steps proof: start the
    // recorder through the REAL handler (self-capture allowed), mark n
    // real moments (each a real focused-window capture), stop, and SAVE
    // through the real Save-card path. Exit by the recorder's verdict.
    let steps_drill: Option<u32> = args
        .iter()
        .position(|a| a == "--steps-drill")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok());
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
    // `--save-comp <name>` (R1z) saves the booted comp under `name`
    // through the Save chip's exact path — paired with `--import-drill`,
    // the reopen-last drill's first half.
    let save_comp: Option<String> = args
        .iter()
        .position(|a| a == "--save-comp")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--lottie-import <file>` (L2) imports a native Lottie and adds it to
    // the timeline — the native-Lottie drill.
    let lottie_import: Option<String> = args
        .iter()
        .position(|a| a == "--lottie-import")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--fx-drill <clip-rel> <fx-rel>` (L3): base cut + transformed/tinted
    // FX overlay, rendered synchronously — the alpha-overlay proof.
    let fx_drill: Option<(String, String)> = args
        .iter()
        .position(|a| a == "--fx-drill")
        .and_then(|i| Some((args.get(i + 1)?.clone(), args.get(i + 2)?.clone())));
    // `--l4-drill` (L4): graded base + screen leak + matted overlay from
    // conventional scratch-root files — the blend/matte/LUT proof.
    let l4_drill = args.iter().any(|a| a == "--l4-drill");
    // `--make-slide <asset-rel>` (W11) mints a media slide from a
    // content-relative artifact through the shelf's "Make slide" path —
    // image/video → one slide, `.pdf` → page slides.
    let make_slide: Option<String> = args
        .iter()
        .position(|a| a == "--make-slide")
        .and_then(|i| args.get(i + 1))
        .cloned();
    // `--run-workflow <id>` (WF1) runs a registered workflow through the
    // Workflows tab's exact Run path — verdict on stdout; pair with
    // `--screenshot` to terminate.
    let run_workflow: Option<String> = args
        .iter()
        .position(|a| a == "--run-workflow")
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
    // `--inspector` pins the inspector OVERLAY open — proof shots of the
    // floating panel reproduce (it is closed by default; content owns the
    // full width).
    let inspector = args.iter().any(|a| a == "--inspector");
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
    // Signal ledger (Phase 2): off by default. `--ledger` or `DPL_LEDGER=1`
    // starts the Parquet writer under `.userdata/analytics/`.
    let ledger = args.iter().any(|a| a == "--ledger")
        || std::env::var_os("DPL_LEDGER").is_some_and(|v| v != "0" && v != "false");
    // `--ai-backfill` — the PIGGYBACK pass: walk provider sessions already
    // on disk and recover the cost they already record. Off by default,
    // idempotent, and it only runs when the ledger is on — there is no
    // point recovering facts with nowhere to write them.
    let ai_backfill_on = args.iter().any(|a| a == "--ai-backfill");
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

    // `--export-guide <guide-rel> [<session-rel>]` — the headless W5
    // drill: run the EXACT Guide-tab export path (dpl_composer::run_export
    // — jail, mint, dpl-guide, index write-through), print the html path,
    // exit by verdict; no window boots.
    if let Some(i) = args.iter().position(|a| a == "--export-guide") {
        let Some(guide) = args.get(i + 1).filter(|a| !a.starts_with("--")) else {
            eprintln!("usage: --export-guide <guide-rel> [<session-rel>]");
            std::process::exit(2);
        };
        let session = args.get(i + 2).filter(|a| !a.starts_with("--")).cloned();
        match dpl_composer::run_export(
            &dpl_engine_capture::content_root(),
            guide,
            session.as_deref(),
            None,
            dpl_guide::ExportOpts::default(),
            None,
        ) {
            Ok(export) => {
                println!("exported {}", export.html.display());
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
                lib_section,
                author_editor,
                author_container,
                author_draft,
                card_tab,
                shot_frame,
                viziwiz_open,
                viziwiz_snap,
                chat_open,
                #[cfg(feature = "eguipilotai-devtools")]
                chat_authored,
                #[cfg(feature = "eguipilotai-devtools")]
                chat_proof,
                steps_hotkey_drill,
                steps_drill,
                video_drill,
                video_format,
                mood,
                present,
                presenter_drill,
                session_drill,
                open_session,
                open_replay,
                open_rail,
                inspector,
                open_episode,
                import_drill,
                pin_packs,
                save_comp,
                lottie_import,
                fx_drill,
                l4_drill,
                make_slide,
                run_workflow,
                seam_port,
                ledger,
                ai_backfill: ai_backfill_on,
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
    lib_section: Option<String>,
    author_editor: bool,
    author_container: Option<String>,
    author_draft: usize,
    card_tab: Option<String>,
    shot_frame: u32,
    viziwiz_open: bool,
    viziwiz_snap: bool,
    chat_open: bool,
    #[cfg(feature = "eguipilotai-devtools")]
    chat_authored: bool,
    #[cfg(feature = "eguipilotai-devtools")]
    chat_proof: Option<chat::ChatProofMode>,
    steps_hotkey_drill: bool,
    steps_drill: Option<u32>,
    video_drill: Option<u32>,
    video_format: Option<String>,
    mood: Option<usize>,
    present: Option<String>,
    presenter_drill: Option<usize>,
    session_drill: Option<u32>,
    open_session: Option<String>,
    open_replay: Option<String>,
    open_rail: Option<String>,
    inspector: bool,
    open_episode: Option<String>,
    import_drill: Option<String>,
    pin_packs: Vec<String>,
    save_comp: Option<String>,
    lottie_import: Option<String>,
    fx_drill: Option<(String, String)>,
    l4_drill: bool,
    make_slide: Option<String>,
    run_workflow: Option<String>,
    /// The agent seam's port; None = `--no-seam`.
    seam_port: Option<u16>,
    /// Signal ledger (Phase 2) — Parquet writer under `.userdata/analytics/`.
    ledger: bool,
    /// The piggyback pass — recover cost from sessions already on disk.
    ai_backfill: bool,
}

struct App {
    shell: Shell,
    /// The one fabric — kept for the first-frame waker install (the
    /// egui Context exists only inside frames; doctrine law 6).
    fabric: Fabric,
    waker_installed: bool,
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
    steps_drill: Option<u32>,
    sd_frames: u32,
    sd_phase: u8,
    sd_marks: u32,
    video_drill: Option<u32>,
    video_format: Option<String>,
    video_frames: u32,
    video_started: Option<std::time::Instant>,
    session_drill: Option<u32>,
    drill_frames: u32,
    drill_started: Option<std::time::Instant>,
    #[cfg(feature = "eguipilotai-devtools")]
    pilot_runtime: Option<
        std::sync::Arc<
            std::sync::Mutex<
                egui_pilot_ai_runtime::RuntimeGuard<egui_pilot_ai_transport::StdioTransport>,
            >,
        >,
    >,
    #[cfg(feature = "eguipilotai-devtools")]
    coach_signals: Option<coach_signals::CoachSignals>,
    /// Every signal id a touchpoint may wait for in this build, resolved from
    /// the hub registry and the provider registrations at boot.
    #[cfg(feature = "eguipilotai-devtools")]
    coach_catalog: Vec<egui_pilot_ai::protocol::SignalId>,
    /// Durable stamped-signal writer. `None` when `--ledger` was not set.
    ledger: Option<dpl_ledger::LedgerTap>,
}

impl App {
    fn new(boot: Boot) -> Self {
        let Boot {
            shot,
            boot_route,
            focus,
            lib_section,
            author_editor,
            author_container,
            author_draft,
            card_tab,
            shot_frame,
            viziwiz_open,
            viziwiz_snap,
            chat_open,
            #[cfg(feature = "eguipilotai-devtools")]
            chat_authored,
            #[cfg(feature = "eguipilotai-devtools")]
            chat_proof,
            steps_hotkey_drill,
            steps_drill,
            video_drill,
            video_format,
            mood,
            present,
            presenter_drill,
            session_drill,
            open_session,
            open_replay,
            open_rail,
            inspector,
            open_episode,
            import_drill,
            pin_packs,
            save_comp,
            lottie_import,
            fx_drill,
            l4_drill,
            make_slide,
            run_workflow,
            seam_port,
            ledger: ledger_on,
            ai_backfill: ai_backfill_on,
        } = boot;
        let mut workbench = dpl_hub_workbench::WorkbenchHub::default();
        if let Some(id) = focus {
            workbench.set_focus(id);
        }
        // Section BEFORE focus would be pointless — `set_focus` pins TEMPLATES
        // — so the section seam runs after and wins when both are given.
        // Container BEFORE the drafts: a draft started under the wrong
        // heading would be labelled by the wrong container in the strip.
        if let Some(slug) = &author_container {
            workbench.author_container(slug);
        }
        if author_draft > 0 {
            workbench.author_draft(author_draft);
        }
        if author_editor {
            workbench.open_author_editor();
        }
        // After the editor seam: the tab only exists once the overlay is open,
        // and this opens one itself when nothing else did.
        if let Some(slug) = &card_tab {
            workbench.open_card_tab(slug);
        }
        if let Some(slug) = &lib_section {
            if !workbench.set_lib_section(slug) {
                tracing::warn!(
                    slug = %slug,
                    "unknown --lib-section; the Library opens on its default wall"
                );
            }
        }
        // The episode-wall seam (D5 proofs).
        if let Some(id) = &open_episode {
            workbench.open_episode(id);
        }
        // ONE fabric, shared: the shell publishes AND (since D2) listens;
        // the Present hub requests full-bleed over it.
        let fabric = Fabric::new();
        // Open the ledger ears AT FABRIC BIRTH — before boot drills publish —
        // so harness mutations land in Parquet exactly like real ones.
        let ledger = if ledger_on {
            let root = dpl_engine_capture::userdata_root().join("analytics");
            match dpl_ledger::LedgerTap::open(&fabric, root.clone()) {
                Ok(tap) => {
                    tracing::info!(root = %root.display(), "signal ledger writing");
                    if ai_backfill_on {
                        run_ai_backfill(&fabric, &root);
                    }
                    Some(tap)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "signal ledger failed to open — continuing without it");
                    None
                }
            }
        } else {
            None
        };
        // Open the shell's signal ears AT FABRIC BIRTH (W8): the boot
        // drills below publish real signals; ears opened later miss them.
        let shell_capture_rx = fabric.subscribe_capture();
        let shell_jobs_rx = fabric.subscribe_jobs();
        let shell_providers_rx = fabric.subscribe_providers();
        let mut present_hub = dpl_hub_present::PresentHub::new(fabric.clone());
        if let Some(id) = &present {
            present_hub.open_show(id);
            // The teleprompter drill (D4 proofs) rides an open show.
            if let Some(n) = presenter_drill {
                present_hub.presenter_drill(n);
            }
        }
        // Ring-0 estate (Fjall+Moka): durable shell route + WiredFRONT
        // lastComp for the agent seam. Open failure never kills the app.
        let estate: Option<std::sync::Arc<dpl_estate::Estate>> = match dpl_estate::Estate::open(
            &dpl_engine_capture::userdata_root().join("estate"),
        ) {
            Ok(e) => {
                if let Some(aside) = e.stepped_aside() {
                    // Says it ONCE, plainly, and names where the old store
                    // went. The estate only ever held `shell/route` and
                    // `wiredfront/last-comp`, so the cost is that the app
                    // reopens on the default route — not lost work.
                    tracing::warn!(
                        stale = %aside.display(),
                        "estate was unreadable (older on-disk format) — moved aside and started fresh; \
                         the remembered route and last comp reset"
                    );
                }
                Some(std::sync::Arc::new(e))
            }
            Err(e) => {
                tracing::warn!(%e, "estate open failed — durable seam state disabled");
                None
            }
        };
        // The harness path into the SESSIONS player (S2).
        let mut wiredfront = dpl_hub_wiredfront::WiredfrontHub::default();
        // WIRE THE FABRIC BEFORE THE DRILLS (W8 fix): harness mutations
        // must signal exactly like real actions — the old ordering ran
        // import/pin/save drills against an unwired hub, silently
        // emitting nothing.
        workbench.wire_fabric(&fabric);
        wiredfront.wire_fabric(&fabric);
        if let Some(est) = &estate {
            wiredfront.wire_estate(std::sync::Arc::clone(est));
        }
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
        // R1z's harness seam: persist the drilled comp (and the
        // last-comp state) through the Save chip's exact path.
        if let Some(name) = &save_comp {
            wiredfront.save_comp(name);
        }
        // L2: import a native Lottie onto the timeline (the drill seam).
        if let Some(file) = &lottie_import {
            wiredfront.import_lottie(std::path::Path::new(file));
        }
        // L3: the alpha-overlay proof — base cut + transformed/tinted FX
        // overlay, rendered synchronously before the first frame.
        if let Some((clip, fx)) = &fx_drill {
            wiredfront.fx_drill(clip, fx);
        }
        // L4's harness seam: blend + matte + LUT in one synchronous proof.
        if l4_drill {
            wiredfront.l4_drill();
        }
        // W11's harness seam: mint a media slide from an artifact through the
        // shelf's exact "Make slide" path (image/video → one, pdf → pages).
        if let Some(rel) = &make_slide {
            wiredfront.make_slide(rel);
        }
        // WF1's harness seam: run a registered workflow through the
        // Workflows tab's exact Run path (jobs facts ride the wired fabric).
        if let Some(id) = &run_workflow {
            wiredfront.run_workflow(id);
        }
        // Chat coding providers work inside the canonical launch workspace,
        // not DevPULSE's media/content store. The launcher (and later the
        // named-project registry) owns this root; provider or conversation
        // input can never replace it.
        let provider_working_directory = std::env::current_dir().unwrap_or_default();
        let codex_activation =
            dpl_chat_codex::CodexActivationCoordinator::discover(&provider_working_directory);
        let grok_activation =
            dpl_chat_grok::GrokActivationCoordinator::discover(&provider_working_directory);
        // One census, read BEFORE the registrations so activation can be
        // derived from what is actually on this machine (H1 §3.1). The same
        // value is threaded into the chat window config below — the roots are
        // never walked twice.
        let history_catalog = local_chat_history_catalog();
        // Chat providers register as data. The peer-window UI never names one;
        // only independently qualified subscription activation is projected.
        // Conversation transport remains visibly blocked.
        #[cfg(feature = "eguipilotai-devtools")]
        let authored_chat = chat_authored.then(dpl_chat_testkit::authored_chat);
        #[cfg(feature = "eguipilotai-devtools")]
        let chat_registrations = authored_chat.as_ref().map_or_else(
            || {
                vec![
                    activation_registration(
                        dpl_chat_codex::MANIFEST,
                        codex_activation.activation().clone(),
                        codex_activation.activation_capability_qualified(),
                    ),
                    dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_claude::MANIFEST),
                    dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_cursor::MANIFEST),
                    dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_gemini::MANIFEST),
                    activation_registration(
                        dpl_chat_grok::MANIFEST,
                        grok_activation.activation().clone(),
                        grok_activation.activation_capability_qualified(),
                    ),
                ]
                .into_iter()
                .map(|registration| with_local_history(registration, &history_catalog))
                .collect()
            },
            |fixture| fixture.providers.clone(),
        );
        #[cfg(not(feature = "eguipilotai-devtools"))]
        let chat_registrations: Vec<dpl_chat_runtime::ProviderRegistration> = vec![
            activation_registration(
                dpl_chat_codex::MANIFEST,
                codex_activation.activation().clone(),
                codex_activation.activation_capability_qualified(),
            ),
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_claude::MANIFEST),
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_cursor::MANIFEST),
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_gemini::MANIFEST),
            activation_registration(
                dpl_chat_grok::MANIFEST,
                grok_activation.activation().clone(),
                grok_activation.activation_capability_qualified(),
            ),
        ]
        .into_iter()
        .map(|registration| with_local_history(registration, &history_catalog))
        .collect();
        // Read the registered providers before the registrations are moved —
        // the coach catalog names them so an agent can wait for one provider's
        // activation rather than any provider's.
        #[cfg(feature = "eguipilotai-devtools")]
        let coach_providers: Vec<dpl_ai_core::ProviderId> = chat_registrations
            .iter()
            .map(|registration| registration.manifest.descriptor.id)
            .collect();
        let chat_runtime =
            dpl_chat_runtime::ChatRuntime::try_from_registrations(chat_registrations)
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
            Box::new(dpl_hub_coach::CoachHub::new()),
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
        // Registry for estate hydrate + the agent seam (same specs the shell has).
        let seam_registry = dpl_seam::Registry::new(hubs.iter().map(|h| h.spec()).collect());
        #[cfg(feature = "eguipilotai-devtools")]
        let coach_catalog = coach_signals::declare(hubs.iter().map(|h| h.spec()), coach_providers);

        let mut viziwiz = dpl_viziwiz::Viziwiz::new(&fabric);
        if viziwiz_open {
            viziwiz.open_now();
        }
        let mut shell = Shell::with_ears(
            hubs,
            fabric.clone(),
            shell_capture_rx,
            shell_jobs_rx,
            shell_providers_rx,
        );
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
            // Estate restore when present + registry-valid; else the prep house.
            None => {
                if let Some(route) = dpl_seam::hydrated_route(estate.as_deref(), &seam_registry) {
                    shell.navigate(route);
                } else {
                    shell.navigate(Route {
                        hub: dpl_hub_wiredfront::HUB,
                        sub: dpl_hub_wiredfront::VIZIWIZ,
                    });
                }
            }
        }
        // Spawn the seam AFTER the boot navigate so we can seed last_route
        // (broadcast subscribers miss events published before they subscribe).
        if let Some(port) = seam_port {
            dpl_seam::spawn(
                fabric.clone(),
                seam_registry,
                dpl_engine_capture::content_root(),
                estate.clone(),
                Some(shell.route()),
                port,
            );
        }
        // `--open-rail <hub>` — the accordion pinned open (L1a proofs).
        // The CLI string is leaked once, like the boot route's.
        if let Some(h) = open_rail {
            shell.set_open_house(Some(dpl_hub::HubId(Box::leak(h.into_boxed_str()))));
        }
        // `--inspector` — the overlay pinned open for proof shots.
        if inspector {
            shell.set_inspector_open(true);
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
        // One session, one determinism gate. Evidence-forced when this boot
        // is a proof shot — policy, not operator whim (§9.2).
        let determinism = dpl_intent::DeterminismGate::default();
        if shot.is_some() {
            determinism.force_evidence();
            fabric.publish_intent(dpl_fabric::IntentEvent::DeterminismChanged {
                mode: dpl_intent::DeterminismMode::Strict,
                cause: Some(dpl_intent::StrictCause::Evidence),
            });
        }
        shell.attach_determinism(determinism.clone());
        let chat = chat::ChatWindowCoordinator::new(
            &fabric,
            chat_runtime,
            chat::ChatWindowConfig {
                initial_route: shell.route(),
                catalog: history_catalog,
                routes: chat_routes,
                authored_models: {
                    #[cfg(feature = "eguipilotai-devtools")]
                    {
                        authored_chat.map(|fixture| (fixture.social, fixture.interactions))
                    }
                    #[cfg(not(feature = "eguipilotai-devtools"))]
                    {
                        None
                    }
                },
                codex_activation,
                grok_activation,
                open_at_boot: chat_open,
                estate: estate.clone(),
                #[cfg(feature = "eguipilotai-devtools")]
                proof_mode: chat_proof,
            },
        );
        chat.attach_determinism(determinism);

        Self {
            shell,
            fabric,
            waker_installed: false,
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
            steps_drill,
            sd_frames: 0,
            sd_phase: 0,
            sd_marks: 0,
            video_drill,
            video_format,
            video_frames: 0,
            video_started: None,
            session_drill,
            drill_frames: 0,
            drill_started: None,
            #[cfg(feature = "eguipilotai-devtools")]
            pilot_runtime: None,
            #[cfg(feature = "eguipilotai-devtools")]
            coach_signals: None,
            #[cfg(feature = "eguipilotai-devtools")]
            coach_catalog,
            ledger,
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
        let guard = egui_pilot_ai_runtime::RuntimeGuard::attach(
            &cc.egui_ctx,
            egui_pilot_ai_transport::StdioTransport::stdio(64),
            config,
        );
        guard.pilot().set_coach_palette(Self::coach_palette());
        guard
            .pilot()
            .declare_coach_signals(self.coach_catalog.clone());
        // The reverse seam: every coach lifecycle fact becomes a stamped
        // COACH-domain signal. The tap is a witness — the devtools pipe
        // still drains its own copy.
        let fabric = self.fabric.clone();
        guard.pilot().set_coach_event_tap(move |event| {
            if let Some(event) = coach_fabric::to_fabric(event) {
                fabric.publish_coach(event);
            }
        });
        let runtime = std::sync::Arc::new(std::sync::Mutex::new(guard));
        self.chat
            .attach_pilot_runtime(std::sync::Arc::clone(&runtime));
        self.pilot_runtime = Some(runtime);
        self.coach_signals = Some(coach_signals::CoachSignals::subscribe(&self.fabric));
        tracing::info!("EguiPilotAI private devtools runtime attached");
    }

    /// Forward this frame's app signals to the coach so a touchpoint can wait
    /// on "the build finished" or "they landed on Workbench".
    ///
    /// One way, and the app decides what is publishable: the toolkit never
    /// subscribes to the fabric, so an agent can only wait for what the
    /// `coach_signals` catalog declares.
    #[cfg(feature = "eguipilotai-devtools")]
    fn publish_coach_signals(&mut self) {
        let (Some(signals), Some(runtime)) = (&mut self.coach_signals, &self.pilot_runtime) else {
            return;
        };
        let drained = signals.drain();
        if drained.is_empty() {
            return;
        }
        let runtime = runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for signal in drained {
            runtime.pilot().note_coach_signal(signal);
        }
    }

    /// A coached control wears the ColdLight paint vocabulary: a crisp cyan
    /// edge, a gradient tint head, and black diffusion scatter dimming the
    /// rest of the screen — never a backdrop blur.
    #[cfg(feature = "eguipilotai-devtools")]
    fn coach_palette() -> egui_pilot_ai::CoachPalette {
        let accent = dpl_coldlight::Tone::Cyan;
        egui_pilot_ai::CoachPalette {
            ring: accent.color(),
            ring_width: 2.0,
            tint: accent.tint(),
            scatter: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 150),
            caption_plate: dpl_coldlight::surface::RAISED,
            caption_edge: dpl_coldlight::surface::HAIRLINE,
            caption_ink: dpl_coldlight::ink::STRONG,
            caption_font: egui::FontId::proportional(13.0),
            corner_radius: dpl_coldlight::RADIUS_SM,
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
                    // process::exit skips Drop — flush the ledger explicitly.
                    if let Some(tap) = self.ledger.take() {
                        tap.flush_and_join();
                    }
                    std::process::exit(0);
                }
                other => {
                    eprintln!("screenshot FAILED: {other:?}");
                    if let Some(tap) = self.ledger.take() {
                        tap.flush_and_join();
                    }
                    std::process::exit(1);
                }
            }
        }

        // Keep frames flowing while we wait for the capture to come back.
        ctx.request_repaint();
        if self.frames > self.shot_frame + 300 {
            eprintln!("screenshot never arrived after 300 frames");
            if let Some(tap) = self.ledger.take() {
                tap.flush_and_join();
            }
            std::process::exit(1);
        }
    }
}

fn activation_registration(
    manifest: dpl_ai_core::ProviderManifest,
    activation: dpl_ai_core::ActivationState,
    subscription_auth_qualified: bool,
) -> dpl_chat_runtime::ProviderRegistration {
    let mut registration = dpl_chat_runtime::ProviderRegistration::planned(manifest);
    registration.activation = activation;
    if subscription_auth_qualified {
        registration
            .qualified_capabilities
            .insert(dpl_ai_core::Capability::SubscriptionAuth);
    }
    registration
}

/// Local transcripts are a real, working capability — not a consolation
/// prize for a missing live adapter. A provider whose history root exists on
/// this machine is ACTIVATED for what it can actually do: read and page its
/// own past sessions. `can_send` stays false because it never gains
/// `StreamTurns`, so the composer disables itself with no new gate, and
/// `Experimental`'s reason string never over-claims what is wired.
///
/// The `is_activated` guard is what keeps this safe for Codex and Grok: a
/// provider that already activated LIVE keeps its own state and merely gains
/// the two history capabilities it genuinely has.
fn with_local_history(
    mut registration: dpl_chat_runtime::ProviderRegistration,
    catalog: &dpl_chat_store::HistoryCatalog,
) -> dpl_chat_runtime::ProviderRegistration {
    if !catalog.has_source(registration.manifest.descriptor.id) {
        return registration;
    }
    registration
        .qualified_capabilities
        .insert(dpl_ai_core::Capability::ReadSessions);
    registration
        .qualified_capabilities
        .insert(dpl_ai_core::Capability::PageHistory);
    if !registration.activation.is_activated() {
        registration.activation = dpl_ai_core::ActivationState::Experimental {
            reason: "Local transcripts only — live turns are not qualified yet.".to_owned(),
            // No recovery action: nothing is broken and there is nothing for
            // the operator to fix. The live adapter simply is not built yet.
            recovery: Vec::new(),
        };
    }
    registration
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

/// Resolve the verified app-relative ffmpeg payload and inject that one
/// absolute path into every engine seam. Production never searches PATH,
/// package managers, user installs, or the working directory.
fn init_ffmpeg() {
    match dpl_tools_deps::resolve_current(dpl_tools_deps::FFMPEG) {
        Ok(dependency) => {
            let path = dependency.path().to_owned();
            tracing::info!(
                ffmpeg = %path.display(),
                version = dependency.version(),
                sha256 = dependency.sha256(),
                "verified embedded ffmpeg resolved"
            );
            dpl_engine_capture::set_ffmpeg_path(path.clone());
            dpl_engine_media::set_ffmpeg_path(path.clone());
            dpl_video::set_ffmpeg_path(path);
        }
        Err(error) => {
            tracing::warn!(
                %error,
                "verified embedded ffmpeg is unavailable; previews, probes, and renders will fail"
            );
        }
    }
}

/// Resolve the verified app-relative poppler tree and inject pdftoppm into
/// the composer's PDF rasterizer (the ffmpeg pattern). Unavailability is a
/// warn, not a failure — the rasterizer degrades to the PATH name and its
/// absence stays an honest per-call error.
fn init_pdftoppm() {
    match dpl_tools_deps::resolve_current(dpl_tools_deps::PDFTOPPM) {
        Ok(dependency) => {
            let path = dependency.path().to_owned();
            tracing::info!(
                pdftoppm = %path.display(),
                version = dependency.version(),
                sha256 = dependency.sha256(),
                "verified embedded pdftoppm resolved"
            );
            dpl_composer::import::set_pdftoppm_path(path);
        }
        Err(error) => {
            tracing::warn!(
                %error,
                "verified embedded pdftoppm is unavailable; PDF slides fall back to PATH"
            );
        }
    }
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
    fn on_exit(&mut self) {
        if let Some(tap) = self.ledger.take() {
            tap.flush_and_join();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The WAKER (signal doctrine law 6), installed once: background
        // publishes repaint the UI the same frame — no polling burn.
        if !self.waker_installed {
            self.waker_installed = true;
            let ctx = ui.ctx().clone();
            self.fabric
                .attach_waker(std::sync::Arc::new(move || ctx.request_repaint()));
        }
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

        // `--steps-drill <n>` (W5.3): the AI-driven Steps proof — REAL
        // recorder start through the palette handler (self-capture
        // allowed), n real marked window captures, real stop, then the
        // Save-card save path. Exit by the recorder's own verdict.
        if let Some(n) = self.steps_drill {
            self.sd_frames += 1;
            match self.sd_phase {
                0 if self.sd_frames >= 30 => {
                    let msg = self.viziwiz.steps_start_drill_now().to_string();
                    eprintln!("steps-drill: {msg}");
                    if !msg.contains("recording") {
                        std::process::exit(1);
                    }
                    self.sd_phase = 1;
                }
                1 if self.sd_frames.is_multiple_of(45) => {
                    if self.sd_marks < n {
                        self.viziwiz.steps_mark_now();
                        self.sd_marks += 1;
                    } else if self.viziwiz.steps_count() >= n {
                        let msg = self.viziwiz.steps_stop_now().to_string();
                        eprintln!("steps-drill: {msg}");
                        self.sd_phase = 2;
                    }
                }
                2 => {
                    let msg = self.viziwiz.steps_save_now().to_string();
                    eprintln!("steps-drill: {msg}");
                    std::process::exit(if msg.starts_with("guide:") && msg.contains('→') {
                        0
                    } else {
                        1
                    });
                }
                _ => {}
            }
            if self.sd_frames > 1200 {
                eprintln!(
                    "steps-drill: TIMEOUT in phase {} · status: {}",
                    self.sd_phase,
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

        if let Some(tap) = &mut self.ledger {
            tap.drain();
        }

        self.screenshot_harness(&ctx);

        #[cfg(feature = "eguipilotai-devtools")]
        self.publish_coach_signals();

        #[cfg(feature = "eguipilotai-devtools")]
        if let Some(runtime) = &self.pilot_runtime {
            let mut runtime = runtime
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            runtime.pump();
            let runtime_active = !runtime.is_stopped();
            drop(runtime);
            self.chat.consume_pilot_close_requests(&ctx);
            if runtime_active {
                // A private native automation session owns a root-frame
                // heartbeat. Deferred child viewports can otherwise finish a
                // one-tick action after eframe has coalesced the parent wake.
                // This path cannot exist in production builds.
                ctx.request_repaint_after(std::time::Duration::from_millis(1));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::with_local_history;
    use dpl_ai_core::{ActivationState, Capability};
    use dpl_chat_runtime::ProviderRegistration;
    use dpl_chat_store::{HistoryCatalog, HistoryLimits, JsonlHistorySource};

    /// A catalog holding a Claude source and nothing else. The root is this
    /// crate's own directory: `has_source` keys on the DECODER's provider id,
    /// never on what the root contains, so no transcript fixture is needed
    /// and the test cannot depend on the operator's real machine.
    fn claude_catalog() -> HistoryCatalog {
        let mut catalog = HistoryCatalog::default();
        let source = JsonlHistorySource::open(
            env!("CARGO_MANIFEST_DIR"),
            dpl_chat_claude::ClaudeHistoryDecoder,
            HistoryLimits::default(),
        )
        .expect("this crate's own directory is a readable root");
        catalog.register(source).expect("the catalog is empty");
        catalog
    }

    #[test]
    fn local_transcripts_activate_a_planned_provider() {
        let registration = with_local_history(
            ProviderRegistration::planned(dpl_chat_claude::MANIFEST),
            &claude_catalog(),
        );
        assert!(
            registration.activation.is_activated(),
            "transcripts on disk are a real capability, so the provider is activated"
        );
        assert!(registration.has_capability(Capability::ReadSessions));
        assert!(registration.has_capability(Capability::PageHistory));
    }

    #[test]
    fn a_history_activated_provider_still_cannot_send() {
        let registration = with_local_history(
            ProviderRegistration::planned(dpl_chat_claude::MANIFEST),
            &claude_catalog(),
        );
        assert!(
            !registration.can_send(),
            "reading a past transcript never implies a live turn transport"
        );
    }

    #[test]
    fn a_live_activation_is_never_downgraded_by_history() {
        let mut live = ProviderRegistration::planned(dpl_chat_claude::MANIFEST);
        live.activation = ActivationState::Ready;
        let registration = with_local_history(live, &claude_catalog());
        assert_eq!(registration.activation, ActivationState::Ready);
        assert!(registration.has_capability(Capability::ReadSessions));
    }

    #[test]
    fn a_provider_without_transcripts_is_untouched() {
        let registration = with_local_history(
            ProviderRegistration::planned(dpl_chat_gemini::MANIFEST),
            &claude_catalog(),
        );
        assert!(!registration.activation.is_activated());
        assert!(!registration.has_capability(Capability::ReadSessions));
    }
}
