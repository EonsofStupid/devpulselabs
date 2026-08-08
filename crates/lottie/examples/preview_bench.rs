//! How smooth is the live preview, actually?
//!
//! Drives the SAME `serve` transport the timeline scrubs through — one
//! persistent helper, a time written per frame, raw RGBA read back — and
//! reports the honest per-frame cost at a given render size.
//!
//! "Smooth" is not a feeling here: 60fps needs ≤16.7ms/frame, 30fps needs
//! ≤33.3ms. The numbers below say which of those the operator gets, and at
//! what resolution they stop getting it.
//!
//! ```text
//! cargo run --release -p dpl-lottie --example preview_bench -- <lottie> <fonts_dir>
//! ```

use dpl_lottie::Player;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let src = PathBuf::from(args.next().unwrap_or_else(|| {
        eprintln!("usage: preview_bench <lottie> [fonts_dir]");
        std::process::exit(2)
    }));
    let fonts = args.next().map(PathBuf::from);

    // The sizes an operator actually previews and delivers at.
    for (w, h, label) in [
        (480u32, 270u32, "scrub thumbnail"),
        (960, 540, "half HD"),
        (1920, 1080, "1080p"),
        (3840, 2160, "4K"),
    ] {
        let open = Instant::now();
        let mut player = match Player::open_with_fonts(&src, w, h, fonts.as_deref()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{w}x{h}: open failed: {e}");
                continue;
            }
        };
        let open_ms = open.elapsed().as_secs_f64() * 1000.0;

        // Walk the animation the way a scrub does — distinct times, never the
        // same frame twice, so nothing is answered from a cached no-op.
        let n = 60;
        let dur = (player.total_frames / player.fps.max(1.0)).max(0.001);
        let mut worst: f64 = 0.0;
        let start = Instant::now();
        for i in 0..n {
            let t = (i as f32 / n as f32) * dur;
            let one = Instant::now();
            if let Err(e) = player.frame_at(t) {
                eprintln!("{w}x{h}: frame {i} failed: {e}");
                break;
            }
            worst = worst.max(one.elapsed().as_secs_f64() * 1000.0);
        }
        let total = start.elapsed().as_secs_f64();
        let mean_ms = total * 1000.0 / f64::from(n);
        let fps = f64::from(n) / total;

        let verdict = if mean_ms <= 16.7 {
            "60fps — smooth"
        } else if mean_ms <= 33.3 {
            "30fps — usable"
        } else if mean_ms <= 100.0 {
            "choppy scrub"
        } else {
            "NOT a live preview"
        };
        println!(
            "{w:>4}x{h:<4} {label:<16} open {open_ms:6.0}ms | mean {mean_ms:6.1}ms  \
             worst {worst:6.1}ms  = {fps:5.1} fps  -> {verdict}"
        );
    }
}
