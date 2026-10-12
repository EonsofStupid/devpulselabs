//! dpl-proc — the QUIET-SPAWN law (W8, docs-history-2026-10-12:docs/plans/w8-faithfulness.md).
//!
//! Every subprocess a GUI-reachable crate spawns goes through `quiet`,
//! so no child ever pops a console window over the app (the GUI exe is
//! `windows_subsystem = "windows"`; an unflagged console child z-fights
//! it). Zero dependencies — any crate may lean on this leaf.
//!
//! Enforcement: the `cargo green` gate audits the workspace — a file
//! containing `Command::new` must import this helper or carry an
//! explicit `// spawn-audit: exempt(<reason>)` marker.

use std::process::Command;

/// Suppress the child's console window on Windows (CREATE_NO_WINDOW,
/// 0x0800_0000); a no-op elsewhere. Chainable.
pub fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_is_chainable_and_spawnable() {
        // The flag is write-only (std exposes no getter) — the honest
        // assertion is that a quieted command still runs cleanly.
        let mut cmd = Command::new(if cfg!(windows) { "cmd" } else { "true" });
        if cfg!(windows) {
            cmd.args(["/C", "exit 0"]);
        }
        let status = quiet(&mut cmd).status().expect("spawn");
        assert!(status.success());
    }
}
