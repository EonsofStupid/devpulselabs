//! The PROMOTION drill (ADR-014 Ring 1 → Ring 2) — proves the
//! transactional move against a scratch three-ring root. Ignored by
//! default; run it as:
//!
//! ```text
//! CONTENT_ROOT=<scratch>/content ASSETS_ROOT=<scratch>/assets \
//!   USERDATA_ROOT=<scratch>/.userdata \
//!   cargo test -p dpl-composer --test promote_drill -- --ignored --nocapture
//! ```
//!
//! Asserts: the asset entered `content/inbox/` with an index row, and
//! the working original is GONE from assets (move-on-confirm); a source
//! OUTSIDE assets promotes as a plain import and is never deleted.

use std::path::Path;

#[test]
#[ignore = "needs a staged three-ring scratch root; run explicitly"]
fn promote_moves_assets_and_never_deletes_external_sources() {
    let assets = dpl_roots::assets_root();
    let content = dpl_roots::content_root();
    std::fs::create_dir_all(assets.join("captures/web/drill")).unwrap();
    std::fs::create_dir_all(&content).unwrap();

    // A staged capture in the working ring (the seam routes by
    // extension and never decodes stills — bytes suffice).
    let png_ish: &[u8] = b"\x89PNG\r\n\x1a\ndrill-bytes";
    let staged = assets.join("captures/web/drill/still.png");
    std::fs::write(&staged, png_ish).unwrap();

    let rec = dpl_composer::import::promote_asset(&staged).expect("promote");
    assert!(rec.rel_path.starts_with("inbox/"), "{}", rec.rel_path);
    let dest = rec
        .rel_path
        .split('/')
        .fold(content.clone(), |a, s| a.join(s));
    assert!(dest.is_file(), "copy missing: {}", dest.display());
    assert!(
        !staged.exists(),
        "promotion MOVES: the assets original goes"
    );
    let rows = dpl_index::Index::open_default()
        .and_then(|ix| ix.recent(50, None))
        .expect("index readable");
    assert!(rows.iter().any(|r| r.rel_path == rec.rel_path));

    // An EXTERNAL source (outside assets) survives its promotion.
    let external_dir = content.parent().unwrap().join("external-src");
    std::fs::create_dir_all(&external_dir).unwrap();
    let external = external_dir.join("keepme.png");
    std::fs::write(&external, png_ish).unwrap();
    let rec2 = dpl_composer::import::promote_asset(&external).expect("external promote");
    assert!(external.is_file(), "external sources are NEVER deleted");
    assert!(rec2.rel_path.starts_with("inbox/"));

    println!(
        "promote drill ok: moved {} · preserved {}",
        rec.rel_path,
        Path::new("external-src/keepme.png").display()
    );
}
