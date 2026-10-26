//! The import seam's FILESYSTEM drill — proves the real behavior against
//! a scratch content root with real files (and real ffmpeg for posters).
//! Ignored by default (it needs a staged environment); run it as:
//!
//! ```text
//! CONTENT_ROOT=<scratch-root> DPL_IMPORT_DRILL_SRC=<dir-with-media> \
//!   cargo test -p dpl-composer --test import_drill -- --ignored --nocapture
//! ```
//!
//! Asserts, per staged file: the copy landed (inbox for video/image,
//! elements/imported for .aep/.lottie), the SOURCE still exists
//! (copy-never-move), the index row exists, and a video import carries
//! its poster + seconds when ffmpeg is on PATH.

use std::path::PathBuf;

#[test]
#[ignore = "needs CONTENT_ROOT + DPL_IMPORT_DRILL_SRC staged; run explicitly"]
fn import_drill_against_a_scratch_root() {
    let src_dir = PathBuf::from(
        std::env::var("DPL_IMPORT_DRILL_SRC").expect("DPL_IMPORT_DRILL_SRC must point at media"),
    );
    let root = dpl_roots::content_root();
    let mut imported = 0usize;
    for entry in std::fs::read_dir(&src_dir).expect("readable drill src") {
        let src = entry.expect("dir entry").path();
        if !src.is_file() {
            continue;
        }
        let Some(kind) = dpl_composer::import::media_kind(&src) else {
            continue;
        };
        let rec = dpl_composer::import::import_file(&src)
            .unwrap_or_else(|e| panic!("import {} failed: {e}", src.display()));
        imported += 1;
        // The source is NEVER touched.
        assert!(src.is_file(), "source vanished: {}", src.display());
        // The copy landed where its kind belongs.
        let dest = rec.rel_path.split('/').fold(root.clone(), |a, s| a.join(s));
        assert!(dest.is_file(), "copy missing: {}", dest.display());
        match kind {
            "element" => assert!(
                rec.rel_path.starts_with("elements/imported/"),
                "element landed at {}",
                rec.rel_path
            ),
            _ => assert!(
                rec.rel_path.starts_with("inbox/"),
                "{kind} landed at {}",
                rec.rel_path
            ),
        }
        if kind == "video" {
            assert!(rec.seconds.is_some(), "video import missing duration");
            let poster = rec.poster_rel_path.as_ref().expect("video import poster");
            assert!(
                poster
                    .split('/')
                    .fold(root.clone(), |a, s| a.join(s))
                    .is_file(),
                "poster file missing: {poster}"
            );
        }
        // The index row exists (write-through).
        let rows = dpl_index::Index::open_default()
            .and_then(|ix| ix.recent(500, None))
            .expect("index readable");
        assert!(
            rows.iter().any(|r| r.rel_path == rec.rel_path),
            "index row missing for {}",
            rec.rel_path
        );
        println!("imported {} -> {} ({kind})", src.display(), rec.rel_path);
    }
    assert!(
        imported >= 3,
        "drill expects staged media; imported {imported}"
    );
}
