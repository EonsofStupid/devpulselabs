//! dpl-lottie-render — the ThorVG subprocess render helper (ADR-009).
//!
//! The app NEVER links ThorVG; it spawns this leaf binary, exactly like
//! ffmpeg/aerender. All C++ FFI and `unsafe` live here, behind a safe
//! `Lottie` facade. Renders a Bodymovin `.json`/`.lottie` frame-at-`t` to
//! RGBA — for the timeline preview (`serve`: raw RGBA over stdout) and for
//! export (`export`: a PNG sequence for ffmpeg). One-shot `frame` for proofs.
//!
//! ABGR8888 is ThorVG's native SW target; we convert to RGBA on the way out.

use std::io::{BufRead, Write};

mod engine {
    //! The ONLY unsafe in the tree — a thin safe facade over thorvg-sys.
    use std::ffi::CString;
    use thorvg_sys as tvg;

    pub struct Lottie {
        canvas: tvg::Tvg_Canvas,
        anim: tvg::Tvg_Animation,
        buf: Vec<u32>,
        /// The frame ThorVG is currently sitting on. A freshly loaded
        /// animation IS on frame 0 — that is precisely why asking for frame 0
        /// used to fail. `update`/`draw`/`sync` still run on every render, so
        /// skipping a redundant `set_frame` never leaves the canvas undrawn.
        frame: f32,
        pub total_frames: f32,
        pub fps: f32,
    }

    fn ok(r: tvg::Tvg_Result, what: &str) -> Result<(), String> {
        if r == tvg::Tvg_Result::TVG_RESULT_SUCCESS {
            Ok(())
        } else {
            Err(format!("thorvg {what} failed: {r:?}"))
        }
    }

    /// Register a scalable font with ThorVG, keyed by the family name inside
    /// the file — which is what a Lottie's `fonts.list[].fFamily` must match.
    ///
    /// **Text renders as NOTHING for a family ThorVG was never given, and it
    /// does not complain.** Measured 2026-08-07: a text fixture rendered a
    /// fully blank frame at exit code 0 because this call did not exist. So a
    /// failure here is REPORTED — a silent font is the whole defect.
    pub fn load_font(path: &std::path::Path) -> Result<(), String> {
        let c = CString::new(path.to_string_lossy().as_ref())
            .map_err(|e| format!("font path {}: {e}", path.display()))?;
        unsafe { ok(tvg::tvg_font_load(c.as_ptr()), "font_load") }
            .map_err(|e| format!("{e} ({})", path.display()))
    }

    impl Lottie {
        /// Load a Bodymovin buffer at a fixed render size.
        pub fn new(data: &[u8], w: u32, h: u32) -> Result<Self, String> {
            unsafe {
                ok(tvg::tvg_engine_init(0), "engine_init")?;
                // Tvg_Engine_Option(1) == TVG_ENGINE_OPTION_DEFAULT (the
                // named const isn't re-exported at the sys crate root).
                let canvas = tvg::tvg_swcanvas_create(tvg::Tvg_Engine_Option(1));
                if canvas.is_null() {
                    return Err("swcanvas_create returned null".into());
                }
                let mut buf = vec![0u32; (w * h) as usize];
                ok(
                    tvg::tvg_swcanvas_set_target(
                        canvas,
                        buf.as_mut_ptr(),
                        w,
                        w,
                        h,
                        // STRAIGHT alpha (not premultiplied) — correct for
                        // egui `from_rgba_unmultiplied` and PNG export when
                        // the Lottie has transparency.
                        tvg::Tvg_Colorspace::TVG_COLORSPACE_ABGR8888S,
                    ),
                    "set_target",
                )?;

                let anim = tvg::tvg_animation_new();
                if anim.is_null() {
                    return Err("animation_new returned null".into());
                }
                let pic = tvg::tvg_animation_get_picture(anim);
                let mime = CString::new("lottie").unwrap();
                // copy = true: ThorVG owns its own copy, so `data` may drop.
                ok(
                    tvg::tvg_picture_load_data(
                        pic,
                        data.as_ptr() as *const std::os::raw::c_char,
                        data.len() as u32,
                        mime.as_ptr(),
                        std::ptr::null(),
                        true,
                    ),
                    "load_data",
                )?;
                ok(
                    tvg::tvg_picture_set_size(pic, w as f32, h as f32),
                    "set_size",
                )?;
                // Keep the animation in control of the picture after handing
                // it to the canvas (the ThorVG 1.0 ownership rule).
                tvg::tvg_paint_ref(pic);
                ok(tvg::tvg_canvas_add(canvas, pic), "canvas_add")?;

                let mut total = 0.0f32;
                ok(
                    tvg::tvg_animation_get_total_frame(anim, &mut total),
                    "get_total_frame",
                )?;
                let mut dur = 0.0f32;
                ok(
                    tvg::tvg_animation_get_duration(anim, &mut dur),
                    "get_duration",
                )?;
                let fps = if dur > f32::EPSILON {
                    total / dur
                } else {
                    30.0
                };

                Ok(Self {
                    canvas,
                    anim,
                    buf,
                    frame: 0.0,
                    total_frames: total,
                    fps,
                })
            }
        }

        /// Render the frame at `t` seconds (clamped) and return packed RGBA8.
        pub fn rgba_at(&mut self, t_secs: f32) -> Result<Vec<u8>, String> {
            let last = (self.total_frames - 1.0).max(0.0);
            let frame = (t_secs.max(0.0) * self.fps).clamp(0.0, last);
            unsafe {
                // `tvg_animation_set_frame` reports INSUFFICIENT_CONDITION when
                // the requested frame is ALREADY the current one. That is a
                // no-op, not a failure — and the animation loads sitting on
                // frame 0, so treating it as fatal made **frame 0 of every
                // Lottie unrenderable**: `frame … 0.0` always died, `export`
                // died on its first frame and produced nothing at all, and
                // `serve` died whenever the operator scrubbed to the same time
                // twice. Measured 2026-08-07 against a real animated fixture.
                //
                // Skipping the call when the frame has not moved is the honest
                // fix: it makes the no-op a no-op instead of asking ThorVG to
                // report one.
                if self.frame != frame {
                    ok(tvg::tvg_animation_set_frame(self.anim, frame), "set_frame")?;
                    self.frame = frame;
                }
                ok(tvg::tvg_canvas_update(self.canvas), "update")?;
                ok(tvg::tvg_canvas_draw(self.canvas, true), "draw")?;
                ok(tvg::tvg_canvas_sync(self.canvas), "sync")?;
            }
            let mut rgba = Vec::with_capacity(self.buf.len() * 4);
            for px in &self.buf {
                rgba.extend_from_slice(&[
                    (px & 0xFF) as u8,
                    ((px >> 8) & 0xFF) as u8,
                    ((px >> 16) & 0xFF) as u8,
                    ((px >> 24) & 0xFF) as u8,
                ]);
            }
            Ok(rgba)
        }
    }

    impl Drop for Lottie {
        fn drop(&mut self) {
            unsafe {
                tvg::tvg_canvas_destroy(self.canvas);
                tvg::tvg_animation_del(self.anim);
                tvg::tvg_engine_term();
            }
        }
    }
}

fn die(msg: &str) -> ! {
    eprintln!("dpl-lottie-render: {msg}");
    std::process::exit(1);
}

/// Split `--font <path>` / `--fonts <dir>` out of argv, returning the font
/// files to register and the remaining POSITIONAL args untouched — so the
/// existing `<mode> <lottie> <w> <h> …` grammar is unchanged for every caller
/// that passes no fonts.
fn take_font_args(argv: &[String]) -> (Vec<std::path::PathBuf>, Vec<String>) {
    let mut fonts = Vec::new();
    let mut rest = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--font" => {
                if let Some(p) = argv.get(i + 1) {
                    fonts.push(std::path::PathBuf::from(p));
                }
                i += 2;
            }
            "--fonts" => {
                if let Some(dir) = argv.get(i + 1) {
                    // Every scalable font in the directory, sorted so the load
                    // order is the same on every machine.
                    let mut found: Vec<_> = std::fs::read_dir(dir)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| {
                            matches!(
                                p.extension().and_then(|x| x.to_str()),
                                Some("ttf" | "otf" | "ttc")
                            )
                        })
                        .collect();
                    found.sort();
                    fonts.extend(found);
                }
                i += 2;
            }
            _ => {
                rest.push(argv[i].clone());
                i += 1;
            }
        }
    }
    (fonts, rest)
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let (fonts, a) = take_font_args(&argv);
    let usage =
        "usage: dpl-lottie-render <frame|export|serve> <lottie> <w> <h> [args] [--fonts <dir>] [--font <file>]";
    let mode = a.get(1).map(String::as_str).unwrap_or_else(|| die(usage));
    let src = a.get(2).unwrap_or_else(|| die(usage));
    let w: u32 = a
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| die(usage));
    let h: u32 = a
        .get(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| die(usage));
    let data = std::fs::read(src).unwrap_or_else(|e| die(&format!("read {src}: {e}")));
    let mut lottie = engine::Lottie::new(&data, w, h).unwrap_or_else(|e| die(&e));
    // AFTER `Lottie::new`, which owns `tvg_engine_init` — ThorVG will not
    // accept a font before its engine exists. Loud on failure: an unloaded
    // font renders text as nothing, at exit code 0 (see `load_font`).
    for font in &fonts {
        engine::load_font(font).unwrap_or_else(|e| die(&e));
    }

    match mode {
        // frame <lottie> <w> <h> <t_secs> <out.png>
        "frame" => {
            let t: f32 = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let out = a.get(6).map(String::as_str).unwrap_or("frame.png");
            let rgba = lottie.rgba_at(t).unwrap_or_else(|e| die(&e));
            image::save_buffer(out, &rgba, w, h, image::ColorType::Rgba8)
                .unwrap_or_else(|e| die(&format!("png: {e}")));
            println!(
                "frame t={t}s -> {out} ({w}x{h}, {:.2}fps, {} frames)",
                lottie.fps, lottie.total_frames as u32
            );
        }
        // export <lottie> <w> <h> <fps> <out_dir>
        "export" => {
            let fps: f32 = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(lottie.fps);
            let dir = a.get(6).map(String::as_str).unwrap_or(".");
            std::fs::create_dir_all(dir).ok();
            let dur = lottie.total_frames / lottie.fps.max(1.0);
            let n = (dur * fps).round().max(1.0) as u32;
            for i in 0..n {
                let t = i as f32 / fps;
                let rgba = lottie.rgba_at(t).unwrap_or_else(|e| die(&e));
                let path = format!("{dir}/frame-{i:05}.png");
                image::save_buffer(&path, &rgba, w, h, image::ColorType::Rgba8)
                    .unwrap_or_else(|e| die(&format!("png: {e}")));
            }
            println!("export {n} frames @ {fps}fps -> {dir}/frame-%05d.png");
        }
        // serve <lottie> <w> <h>: print a READY header, then for each
        // time (seconds) read on stdin, write w*h*4 raw RGBA bytes to
        // stdout — the timeline scrub protocol (one persistent process).
        "serve" => {
            let mut out = std::io::stdout().lock();
            writeln!(
                out,
                "READY {w} {h} {:.4} {}",
                lottie.fps, lottie.total_frames
            )
            .ok();
            out.flush().ok();
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                let line = line.trim();
                if line == "q" || line.is_empty() {
                    break;
                }
                let t: f32 = line.parse().unwrap_or(0.0);
                let rgba = lottie.rgba_at(t).unwrap_or_else(|e| die(&e));
                if out.write_all(&rgba).is_err() {
                    break;
                }
                out.flush().ok();
            }
        }
        other => die(&format!("unknown mode {other:?}; {usage}")),
    }
}
