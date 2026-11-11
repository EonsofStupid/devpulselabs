//! Actual-disk preservation drill. Creates its own temporary fixture only.
//! Run: cargo run -p dpl-engine-steps --example guide_revision
//! The printed fixture directory is retained for inspection.
use dpl_engine_steps::edit::{update_guide, GuideStepEdit};
use serde_json::{json, Value};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn run() -> Result<(), String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "dpl-guide-revision-example-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root).map_err(|e| format!("create exclusive fixture: {e}"))?;
    let source = root.join("source");
    fs::create_dir(&source).map_err(|e| e.to_string())?;
    fs::write(source.join("first.webp"), b"synthetic first frame").map_err(|e| e.to_string())?;
    fs::write(source.join("second.webp"), b"synthetic second frame").map_err(|e| e.to_string())?;
    fs::write(source.join("notes.txt"), b"retained attachment").map_err(|e| e.to_string())?;
    let manifest = json!({
        "kind": "guide", "name": "Synthetic revision drill", "stepCount": 2, "totalBytes": 43,
        "extension": {"retained": true},
        "steps": [
            {"order": 1, "file": "first.webp", "bytes": 21, "caption": "original", "extension": {"keep": 1}},
            {"order": 2, "file": "second.webp", "bytes": 22, "caption": "to prune"}
        ]
    });
    let original = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(source.join("guide.json"), &original).map_err(|e| e.to_string())?;
    let mut kept: GuideStepEdit =
        serde_json::from_value(manifest["steps"][0].clone()).map_err(|e| e.to_string())?;
    kept.caption = Some("reviewed caption".into());
    let revision = update_guide(&source, vec![kept])?;
    let revised: Value = serde_json::from_slice(
        &fs::read(revision.dir.join("guide.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if fs::read(source.join("guide.json")).map_err(|e| e.to_string())? != original
        || revised["steps"][0]["caption"] != "reviewed caption"
        || revised["extension"] != manifest["extension"]
        || revised["steps"][0]["extension"] != manifest["steps"][0]["extension"]
        || !revision.dir.join("second.webp").is_file()
        || !revision.dir.join("notes.txt").is_file()
        || !revision.dir.join(".guide-revision-complete").is_file()
        || revision.dir.join(".guide-revision-pending").exists()
    {
        return Err(format!(
            "preservation verification failed; fixture: {}",
            root.display()
        ));
    }
    println!("{}", serde_json::to_string_pretty(&json!({
        "status": "passed", "scope": "actual disk revision; synthetic files, no capture or UI",
        "source": source, "revision": revision.dir, "steps": revision.steps,
        "sourceUnchanged": true, "prunedMediaPreserved": true, "extensionMetadataPreserved": true
    })).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
