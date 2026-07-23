//! Guide EDITING — prune, reorder and caption a saved guide (C2; the W5
//! reviewer UI drives this later).
//!
//! The operator records more than they need, then "prunes and tunes" it
//! into a guide worth showing. `update_guide` rewrites `guide.json` from
//! the KEPT steps in their NEW order and deletes the frames that are no
//! longer referenced.
//!
//! Safety (ported from the prior repo's `guide_edit.rs`):
//! - The CALLER owns path scoping (the Tauri inbox jail stayed behind);
//!   every step's `file`/`resultFile` must still be a bare filename that
//!   EXISTS in the guide folder — no separators, no `..`, so a crafted
//!   payload can neither reference nor delete anything outside this guide.
//! - Deleting every step is refused (drop the whole guide instead).
//! - Frame files keep their original names; the manifest's ORDER is the
//!   truth, so a reorder never rewrites image files.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// One step as the reviewer sends it back (the recorder's shape +
/// `caption` — the operator's own words for the step).
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuideStepEdit {
    pub order: u32,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_file: Option<String>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click: Option<[f64; 2]>,
    pub app: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

/// A bare filename that exists inside `dir` — rejects separators and `..`.
fn frame_ok(dir: &Path, file: &str) -> Result<(), String> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err(format!("invalid frame name \"{file}\""));
    }
    if !dir.join(file).is_file() {
        return Err(format!("frame not found: \"{file}\""));
    }
    Ok(())
}

/// Collect every frame filename referenced by a manifest's `steps`.
fn referenced(manifest: &serde_json::Value) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(steps) = manifest.get("steps").and_then(|s| s.as_array()) {
        for s in steps {
            for key in ["file", "resultFile"] {
                if let Some(f) = s.get(key).and_then(|f| f.as_str()) {
                    out.insert(f.to_string());
                }
            }
        }
    }
    out
}

/// Rewrite a guide's steps. `steps` is the KEPT set in its NEW order;
/// anything omitted is dropped and its frames deleted. Returns the new
/// step count.
pub fn update_guide(dir: &Path, steps: Vec<GuideStepEdit>) -> Result<u32, String> {
    if steps.is_empty() {
        return Err("a guide needs at least one step — discard the guide instead".into());
    }
    if !dir.is_dir() {
        return Err(format!("not a guide folder: {}", dir.display()));
    }
    let guide_json = dir.join("guide.json");
    if !guide_json.is_file() {
        return Err("this artifact has no guide.json".into());
    }

    let prev: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&guide_json).map_err(|e| format!("read guide.json: {e}"))?,
    )
    .map_err(|e| format!("parse guide.json: {e}"))?;

    // Validate every incoming frame BEFORE writing anything.
    for s in &steps {
        frame_ok(dir, &s.file)?;
        if let Some(rf) = &s.result_file {
            frame_ok(dir, rf)?;
        }
    }

    // Renumber 1..N in the submitted order (files keep their names).
    let mut kept = steps;
    for (i, s) in kept.iter_mut().enumerate() {
        s.order = (i + 1) as u32;
    }

    let keep_files: HashSet<String> = kept
        .iter()
        .flat_map(|s| std::iter::once(s.file.clone()).chain(s.result_file.clone()))
        .collect();
    let total: usize = kept.iter().map(|s| s.bytes).sum();
    let fallback = dir
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let name = prev
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(&fallback)
        .to_string();

    let manifest = serde_json::json!({
        "kind": "guide",
        "name": name,
        "stepCount": kept.len(),
        "totalBytes": total,
        "steps": kept,
    });
    std::fs::write(
        &guide_json,
        serde_json::to_vec_pretty(&manifest).map_err(|e| format!("guide manifest: {e}"))?,
    )
    .map_err(|e| format!("write guide.json: {e}"))?;

    // Only now drop the frames the operator pruned away.
    for orphan in referenced(&prev).difference(&keep_files) {
        let _ = std::fs::remove_file(dir.join(orphan));
    }
    Ok(kept.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_guide(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-guide-edit-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "step-001.webp",
            "step-001-done.webp",
            "step-002.webp",
            "step-003.webp",
        ] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let step = |order: u32, file: &str, result: Option<&str>| {
            serde_json::json!({
                "order": order, "file": file, "resultFile": result,
                "action": "click", "app": "Test", "title": "Test - App",
                "width": 10, "height": 10, "bytes": 1,
            })
        };
        let manifest = serde_json::json!({
            "kind": "guide", "name": "fixture", "stepCount": 3, "totalBytes": 3,
            "steps": [
                step(1, "step-001.webp", Some("step-001-done.webp")),
                step(2, "step-002.webp", None),
                step(3, "step-003.webp", None),
            ],
        });
        std::fs::write(
            dir.join("guide.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        dir
    }

    fn edit(order: u32, file: &str, result: Option<&str>, caption: Option<&str>) -> GuideStepEdit {
        GuideStepEdit {
            order,
            file: file.into(),
            result_file: result.map(str::to_string),
            action: "click".into(),
            click: None,
            app: "Test".into(),
            title: "Test - App".into(),
            width: 10,
            height: 10,
            bytes: 1,
            caption: caption.map(str::to_string),
        }
    }

    #[test]
    fn reorder_prune_renumbers_and_deletes_orphans() {
        let dir = fixture_guide("reorder");
        // Keep 3 then 1 (2 pruned), caption the first kept step.
        let n = update_guide(
            &dir,
            vec![
                edit(3, "step-003.webp", None, Some("the payoff")),
                edit(1, "step-001.webp", Some("step-001-done.webp"), None),
            ],
        )
        .unwrap();
        assert_eq!(n, 2);
        let m: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("guide.json")).unwrap()).unwrap();
        assert_eq!(m["stepCount"], 2);
        assert_eq!(m["name"], "fixture", "name survives the rewrite");
        assert_eq!(m["steps"][0]["order"], 1, "renumbered in the new order");
        assert_eq!(m["steps"][0]["file"], "step-003.webp");
        assert_eq!(m["steps"][0]["caption"], "the payoff");
        assert_eq!(m["steps"][1]["order"], 2);
        assert!(!dir.join("step-002.webp").exists(), "pruned frame deleted");
        assert!(
            dir.join("step-001-done.webp").exists(),
            "kept pair survives"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refusals_hold_and_write_nothing() {
        let dir = fixture_guide("refuse");
        assert!(update_guide(&dir, vec![]).is_err(), "empty set refused");
        assert!(
            update_guide(&dir, vec![edit(1, "../escape.webp", None, None)]).is_err(),
            "separators refused"
        );
        assert!(
            update_guide(&dir, vec![edit(1, "step-009.webp", None, None)]).is_err(),
            "missing frame refused"
        );
        let m: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("guide.json")).unwrap()).unwrap();
        assert_eq!(m["stepCount"], 3, "manifest untouched by refusals");
        assert!(dir.join("step-002.webp").exists(), "no frame deleted");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
