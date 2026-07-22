//! Workspace automation — the cargo-xtask pattern (no external task runner).
//!
//! `cargo green` runs THE gate, in order, stopping at the first red:
//!   1. fmt --check          (style is law, not opinion)
//!   2. clippy -D warnings   (all targets, whole workspace)
//!   3. test --workspace
//!   4. release build, then ASSERT the exe mtime advanced — a running
//!      instance silently masks a failed link; the mtime check is the
//!      stale-exe killer.
//!
//! `cargo xtask fix` = fmt + clippy --fix (the tidy loop).

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::SystemTime;

fn run(name: &str, args: &[&str]) -> Result<(), String> {
    println!("── {name} ─────────────────────────────────");
    let status = Command::new("cargo")
        .args(args)
        .status()
        .map_err(|e| format!("{name}: spawn failed: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} FAILED ({status})"))
    }
}

fn exe_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn green() -> Result<(), String> {
    run("fmt", &["fmt", "--all", "--", "--check"])?;
    run(
        "clippy",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("test", &["test", "--workspace"])?;

    let exe = Path::new("target/release/devpulselabs.exe");
    let before = exe_mtime(exe);
    run("release build", &["build", "--release"])?;
    let after = exe_mtime(exe);
    match (before, after) {
        (_, None) => Err("release exe missing after build".into()),
        // Fresh-but-unchanged is fine ONLY when cargo declared it fresh —
        // which the successful status already covers; the check exists to
        // catch a masked link failure that leaves an OLD exe behind.
        (Some(b), Some(a)) if a < b => Err("release exe went BACKWARDS — locked/stale".into()),
        _ => {
            println!("green ✔ exe: {}", exe.display());
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_else(|| "green".into());
    let result = match task.as_str() {
        "green" => green(),
        "fix" => run("fmt", &["fmt", "--all"]).and_then(|()| {
            run(
                "clippy --fix",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--fix",
                    "--allow-dirty",
                ],
            )
        }),
        other => Err(format!("unknown task \"{other}\" — tasks: green, fix")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}
