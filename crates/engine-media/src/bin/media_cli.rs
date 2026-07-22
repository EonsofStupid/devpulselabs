//! `media-cli <source-dir> [generated-dir]` — run the optimization pipeline
//! by hand (the old `bun run media`, without bun). Defaults the output to
//! the source's `../generated` sibling, the episode-tree convention.

use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(source) = args.next().map(PathBuf::from) else {
        eprintln!("usage: media-cli <source-dir> [generated-dir]");
        std::process::exit(2);
    };
    let generated = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| source.parent().unwrap_or(&source).join("generated"));

    if !source.is_dir() {
        eprintln!("not a directory: {}", source.display());
        std::process::exit(2);
    }

    match dpl_engine_media::optimize_dir(&source, &generated) {
        Ok(report) => {
            println!(
                "media: {} written, {} fresh (skipped){}",
                report.written,
                report.skipped,
                if report.errors.is_empty() { "" } else { " — WITH ERRORS:" }
            );
            for e in &report.errors {
                eprintln!("  ! {e}");
            }
            std::process::exit(if report.errors.is_empty() { 0 } else { 1 });
        }
        Err(e) => {
            eprintln!("media: {e}");
            std::process::exit(1);
        }
    }
}
