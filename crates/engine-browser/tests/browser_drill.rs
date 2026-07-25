//! LIVE browser drill — ignored in the gate (no browser assumption in CI).
//! Run it deliberately: `cargo test -p dpl-engine-browser -- --ignored`.
//!
//! Proves the whole spine on the real installed browser: resolve → launch
//! headless with an ephemeral profile → navigate → evaluate → screenshot
//! (real PNG bytes) → close (child actually gone).

use dpl_engine_browser::{resolve, Browser, BrowserConfig};

#[test]
#[ignore = "live drill: launches the real installed browser"]
fn headless_launch_navigate_evaluate_screenshot() {
    let (bin, source) = resolve().expect("no chromium-family browser installed");
    eprintln!("drill: resolved {} ({source})", bin.display());

    let mut browser = Browser::launch(&BrowserConfig { window: (800, 600) }).unwrap();
    {
        let mut page = browser.new_page().unwrap();
        page.set_viewport(800, 600).unwrap();
        page.navigate("data:text/html,<h1 id=t>ok</h1>").unwrap();

        let v = page
            .evaluate("document.getElementById('t').textContent")
            .unwrap();
        assert_eq!(v, serde_json::json!("ok"));

        let out =
            std::env::temp_dir().join(format!("dpl-browser-drill-{}.png", std::process::id()));
        let n = page.screenshot_png(&out).unwrap();
        eprintln!("drill: screenshot {n} bytes at {}", out.display());
        let bytes = std::fs::read(&out).unwrap();
        assert!(
            bytes.len() > 1024,
            "suspiciously small: {} bytes",
            bytes.len()
        );
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
            "not a PNG"
        );
        let _ = std::fs::remove_file(&out);
    }
    browser.close().unwrap();
}
