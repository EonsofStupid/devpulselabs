//! The editor's tiny persisted memory (R1z) — `composer-state.json` at
//! the user-data root (the `artifacts.db`/`sources.json` precedent:
//! app-written operator data, never committed, never authored by hand).
//! One fact today: the last comp the operator worked in, so the editor
//! never opens empty-handed (experience gate row 1). Best-effort on both
//! ends — a missing or corrupt file is a default, never an error surface.

use serde::{Deserialize, Serialize};
use std::path::Path;

const FILE: &str = "composer-state.json";

/// What the editor remembers between runs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EditorState {
    /// The last saved/opened comp folder, content-root-relative with
    /// forward slashes (the index rel_path convention).
    pub last_comp: Option<String>,
}

/// Read the state; absent or corrupt reads as the default.
pub fn load(root: &Path) -> EditorState {
    std::fs::read_to_string(root.join(FILE))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Write the state — best-effort pretty JSON + LF; callers never fail on it.
pub fn save(root: &Path, state: &EditorState) {
    if let Ok(mut json) = serde_json::to_string_pretty(state) {
        json.push('\n');
        let _ = std::fs::create_dir_all(root);
        let _ = std::fs::write(root.join(FILE), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-composer-state-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn round_trips_and_defaults_when_absent() {
        let root = scratch("roundtrip");
        assert_eq!(load(&root), EditorState::default(), "absent → default");
        let state = EditorState {
            last_comp: Some("inbox/comp-3".into()),
        };
        save(&root, &state);
        assert_eq!(load(&root), state);
        let raw = std::fs::read_to_string(root.join(FILE)).unwrap();
        assert!(raw.ends_with('\n'), "LF-terminated like every house JSON");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_state_reads_as_default_never_an_error() {
        let root = scratch("corrupt");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(FILE), "{not json").unwrap();
        assert_eq!(load(&root), EditorState::default());
        let _ = std::fs::remove_dir_all(&root);
    }
}
