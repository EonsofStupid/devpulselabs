//! Where the overlay painter (`dpl-coach-overlay`) lives — the ONE resolver
//! the Coach hub and the coach daemon share (they each carried a copy).
//!
//! An explicit `DPL_COACH_OVERLAY` override (a named dev/drill seam) wins;
//! otherwise the painter must sit beside the running exe, where every build
//! ships it (ADR-009). Never PATH: a helper the app did not ship is not the
//! app's, and its absence is an honest error naming where it was expected.

use std::path::{Path, PathBuf};

/// The painter's file name on this platform.
fn overlay_name() -> &'static str {
    if cfg!(windows) {
        "dpl-coach-overlay.exe"
    } else {
        "dpl-coach-overlay"
    }
}

/// Locate the overlay painter for this process.
pub fn overlay_path() -> Result<PathBuf, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    resolve_overlay(
        std::env::var_os("DPL_COACH_OVERLAY").map(PathBuf::from),
        exe_dir.as_deref(),
    )
}

/// The resolution itself, free of process state so it is testable.
fn resolve_overlay(
    override_path: Option<PathBuf>,
    exe_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    let dir =
        exe_dir.ok_or("the overlay painter cannot be located: the app's own folder is unknown")?;
    let path = dir.join(overlay_name());
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "the overlay painter (dpl-coach-overlay) is missing beside the app — expected {}",
            path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_painter_is_an_honest_error_never_a_path_lookup() {
        let empty = std::env::temp_dir().join(format!("dpl-overlay-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        let err = resolve_overlay(None, Some(&empty)).expect_err("nothing beside the app");
        assert!(err.contains("missing beside the app"), "{err}");
        assert!(
            err.contains(overlay_name()),
            "names the expected file: {err}"
        );
        assert!(resolve_overlay(None, None).is_err());
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn the_painter_beside_the_app_resolves_and_an_override_wins() {
        let dir = std::env::temp_dir().join(format!("dpl-overlay-beside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(overlay_name()), b"stand-in").unwrap();
        assert_eq!(
            resolve_overlay(None, Some(&dir)),
            Ok(dir.join(overlay_name()))
        );
        let named = PathBuf::from("named-by-the-drill");
        assert_eq!(resolve_overlay(Some(named.clone()), Some(&dir)), Ok(named));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
