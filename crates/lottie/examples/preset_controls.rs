//! THE PROOF: retext and recolour a preset with no After Effects.
//!
//! Drives the shipped path exactly as the app will — `discover` the controls,
//! `apply` operator values, write the patched document to scratch, and render
//! through the same helper the timeline and the exporter use.
//!
//! ```text
//! cargo run -p dpl-lottie --example preset_controls -- <preset.json> <fonts_dir> <out_dir>
//! ```
//!
//! Deliberately an example: it spawns the render helper and needs the font
//! pool staged on disk, and a test that does either is one CI will run.

use dpl_lottie::controls::{self, ControlKind, ControlValue};
use std::path::{Path, PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let src = PathBuf::from(args.next().unwrap_or_else(|| usage()));
    let fonts = PathBuf::from(args.next().unwrap_or_else(|| usage()));
    let out = PathBuf::from(args.next().unwrap_or_else(|| usage()));
    std::fs::create_dir_all(&out).expect("out dir");

    let bytes = std::fs::read(&src).expect("read preset");
    let doc: serde_json::Value = serde_json::from_slice(&bytes).expect("parse preset");

    let found = controls::discover(&doc);
    println!("discovered {} control(s):", found.len());
    for c in &found {
        println!("  {:<28} {:?}  <- {}", c.id, c.kind, c.pointer);
    }
    if found.is_empty() {
        eprintln!("PROOF FAILED: nothing editable was discovered");
        std::process::exit(1);
    }

    let text_id = found
        .iter()
        .find(|c| c.kind == ControlKind::Text)
        .map(|c| c.id.clone());
    let color_ids: Vec<String> = found
        .iter()
        .filter(|c| c.kind == ControlKind::Color)
        .map(|c| c.id.clone())
        .collect();

    // Three genuinely different operator edits.
    let variants: [(&str, [u8; 4]); 3] = [
        ("OFFLINE", [255, 64, 0, 255]),
        ("NO ADOBE", [80, 220, 120, 255]),
        ("4K NATIVE", [200, 80, 255, 255]),
    ];

    for (i, (text, rgba)) in variants.iter().enumerate() {
        let mut values: Vec<(String, ControlValue)> = Vec::new();
        if let Some(id) = &text_id {
            values.push((id.clone(), ControlValue::Text((*text).to_owned())));
        }
        for id in &color_ids {
            values.push((id.clone(), ControlValue::Color(*rgba)));
        }

        let patched = controls::apply(&doc, &found, &values).expect("apply");
        let patched_path = out.join(format!("variant-{i}.json"));
        std::fs::write(
            &patched_path,
            serde_json::to_vec_pretty(&patched).expect("serialize"),
        )
        .expect("write patched");

        // The operator's source is NEVER mutated — prove it, do not assert it.
        let after = std::fs::read(&src).expect("re-read source");
        assert_eq!(bytes, after, "the source Lottie was modified");

        // Render the SAME patched document small and at 4K: resolution is a
        // render-time argument, never baked into the preset.
        for (w, h) in [(512u32, 512u32), (3840, 3840)] {
            let png = out.join(format!("variant-{i}-{w}.png"));
            render(&patched_path, w, h, 0.5, &png, &fonts);
            println!("  wrote {}", png.display());
        }
    }

    println!(
        "\nPROOF: {} variants, each rendered at 512 and 3840, from ONE preset \
         with no After Effects. Open the PNGs — an exit code proves nothing \
         here (text rendered blank at exit 0 before fonts were loaded).",
        variants.len()
    );
}

fn render(src: &Path, w: u32, h: u32, t: f32, out: &Path, fonts: &Path) {
    let mut cmd =
        std::process::Command::new(dpl_lottie::helper_path().expect("locate the Lottie renderer"));
    // The quiet-spawn law (W8) applies to a proof run too — a console flash
    // is a console flash.
    dpl_proc::quiet(&mut cmd);
    let status = cmd
        .arg("frame")
        .arg(src)
        .arg(w.to_string())
        .arg(h.to_string())
        .arg(t.to_string())
        .arg(out)
        .arg("--fonts")
        .arg(fonts)
        .status()
        .expect("spawn helper");
    assert!(status.success(), "render failed for {}", out.display());
}

fn usage() -> ! {
    eprintln!("usage: preset_controls <preset.json> <fonts_dir> <out_dir>");
    std::process::exit(2);
}
