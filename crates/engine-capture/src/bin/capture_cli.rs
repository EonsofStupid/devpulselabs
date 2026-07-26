// spawn-audit: exempt(CLI tool)
//! Headless capture — the same pipeline as the GUI, minus the window, so the
//! capture→encode→content path (and the Stage 2 crop math) is verifiable
//! without a display server attached to the preview tools. Usage:
//!   capture-cli full   <episode-id> <name> [seconds]
//!   capture-cli region <episode-id> <name> <fx> <fy> <fw> <fh> [seconds]
//!   capture-cli <episode-id> <name> [seconds]        # back-compat = full
//! Requires CONTENT_ROOT to point at the repo's content/ dir. ffmpeg: an
//! exe-adjacent `bin/ffmpeg` wins, else PATH (dev CLI — never ships).
use dpl_engine_capture::{capture_display, capture_region, set_ffmpeg_path, FracRect};

fn main() {
    #[cfg(target_os = "windows")]
    const FFMPEG_BIN: &str = "ffmpeg.exe";
    #[cfg(not(target_os = "windows"))]
    const FFMPEG_BIN: &str = "ffmpeg";
    if let Some(local) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("bin").join(FFMPEG_BIN)))
        .filter(|p| p.is_file())
    {
        set_ffmpeg_path(local);
    }

    let a: Vec<String> = std::env::args().collect();
    let usage = || {
        eprintln!(
            "usage:\n  capture-cli full <episode> <name> [seconds]\n  capture-cli region <episode> <name> <fx> <fy> <fw> <fh> [seconds]"
        );
        std::process::exit(2);
    };

    let result = match a.get(1).map(String::as_str) {
        Some("region") => {
            if a.len() < 8 {
                usage();
            }
            let f = |i: usize| a[i].parse::<f64>().unwrap_or(f64::NAN);
            let frac = FracRect {
                x: f(4),
                y: f(5),
                w: f(6),
                h: f(7),
            };
            if [frac.x, frac.y, frac.w, frac.h].iter().any(|v| v.is_nan()) {
                usage();
            }
            let secs = a.get(8).and_then(|s| s.parse().ok()).unwrap_or(5);
            capture_region(&a[2], &a[3], secs, frac)
        }
        Some("full") => {
            if a.len() < 4 {
                usage();
            }
            let secs = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
            capture_display(&a[2], &a[3], secs)
        }
        // back-compat: `capture-cli <episode> <name> [seconds]`
        Some(_ep) if a.len() >= 3 => {
            let secs = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(5);
            capture_display(&a[1], &a[2], secs)
        }
        _ => {
            usage();
            unreachable!()
        }
    };

    println!("{}", serde_json::to_string(&result).unwrap_or_default());
    if !result.ok {
        std::process::exit(1);
    }
}
