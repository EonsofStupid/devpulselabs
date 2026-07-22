//! Real-artifact smoke (Law 2): grab an actual frame from the actual screen.
//! Needs a display, so it is `#[ignore]`d for plain `cargo test` and run
//! explicitly by the verify pass: `cargo test -p dpl-engine-capture -- --ignored`.

use dpl_engine_capture::freeze_primary_to;

#[test]
#[ignore = "needs a live display — run explicitly in the verify pass"]
fn freeze_frame_writes_a_real_png() {
    let path = std::env::temp_dir().join("dpl-engine-capture-smoke.png");
    let _ = std::fs::remove_file(&path);
    let (w, h) = freeze_primary_to(&path).expect("primary monitor freeze");
    assert!(w > 0 && h > 0, "captured dimensions must be real: {w}x{h}");
    let bytes = std::fs::metadata(&path).expect("png written").len();
    assert!(bytes > 10_000, "a real screen frame is never {bytes} bytes");
    std::fs::remove_file(&path).ok();
}
