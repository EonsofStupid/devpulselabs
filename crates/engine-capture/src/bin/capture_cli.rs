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
    if let Err(error) = dpl_roots::initialize() {
        eprintln!("capture-cli: {error}");
        std::process::exit(1);
    }
    #[cfg(target_os = "windows")]
    const FFMPEG_BIN: &str = "ffmpeg.exe";
    #[cfg(not(target_os = "windows"))]
    const FFMPEG_BIN: &str = "ffmpeg";
    // An exe-adjacent ffmpeg wins; otherwise one named by `FFMPEG` (a dev
    // shell pointing at the depot's verified copy). PATH is never searched.
    if let Some(local) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("bin").join(FFMPEG_BIN)))
        .filter(|p| p.is_file())
        .or_else(|| {
            std::env::var_os("FFMPEG")
                .map(std::path::PathBuf::from)
                .filter(|p| p.is_file())
        })
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
        // RA1a: `capture-cli record <dir> <seconds> [<pause at> <resume at>] [mic]`
        // records the primary monitor in segments, pausing and resuming on the
        // clock, and prints the folder's description.
        Some("record") => {
            if a.len() < 4 {
                usage();
            }
            record(&a)
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

/// RA1a's proof: a real recording of the primary monitor, paused and resumed
/// on the clock. Exits after printing what landed.
fn record(a: &[String]) -> ! {
    use dpl_engine_capture::recording::{record, Recording};
    use std::time::Duration;
    let number = |i: usize| a.get(i).and_then(|s| s.parse::<f64>().ok());
    let dir = std::path::PathBuf::from(&a[2]);
    let seconds = number(3).unwrap_or(4.0);
    let pause = number(4).zip(number(5));
    let opts = dpl_engine_capture::VideoOpts {
        height: 0,
        format: "mp4".into(),
        audio: if a.iter().any(|s| s == "mic") {
            "mic"
        } else {
            "none"
        }
        .into(),
        audio_device: None,
        max_seconds: None,
    };
    let Some(monitor) = dpl_engine_capture::list_monitors()
        .into_iter()
        .find(|m| m.is_primary)
        .map(|m| m.id)
    else {
        eprintln!("capture-cli: no primary monitor");
        std::process::exit(1);
    };
    let rec = std::sync::Arc::new(Recording::default());
    let runner = {
        let rec = std::sync::Arc::clone(&rec);
        std::thread::spawn(move || record(&dir, monitor, &opts, &rec))
    };
    let started = std::time::Instant::now();
    let until = |at: f64| {
        let at = Duration::from_secs_f64(at);
        if let Some(wait) = at.checked_sub(started.elapsed()) {
            std::thread::sleep(wait);
        }
    };
    if let Some((pause_at, resume_at)) = pause {
        until(pause_at);
        rec.pause();
        eprintln!("paused at frame {}", rec.frame());
        until(resume_at);
        rec.resume();
        eprintln!("resumed at frame {}", rec.frame());
    }
    until(seconds);
    rec.stop();
    match runner.join() {
        Ok(Ok(manifest)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&manifest).unwrap_or_default()
            );
            std::process::exit(0);
        }
        Ok(Err(error)) => eprintln!("capture-cli: {error}"),
        Err(_) => eprintln!("capture-cli: the recording thread panicked"),
    }
    std::process::exit(1);
}
