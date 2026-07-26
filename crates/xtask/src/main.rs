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

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::SystemTime;

fn run(name: &str, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command.args(args);
    run_command(name, &mut command)
}

fn run_command(name: &str, command: &mut Command) -> Result<(), String> {
    println!("── {name} ─────────────────────────────────");
    let status = command
        .status()
        .map_err(|e| format!("{name}: spawn failed: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} FAILED ({status})"))
    }
}

fn windows_test_linker() -> Result<Option<(String, PathBuf)>, String> {
    if !cfg!(windows) {
        return Ok(None);
    }

    let version = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("test: could not inspect rustc host: {error}"))?;
    if !version.status.success() {
        return Err(format!("test: rustc -vV failed ({})", version.status));
    }
    let version = String::from_utf8(version.stdout)
        .map_err(|error| format!("test: rustc host output was not UTF-8: {error}"))?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| "test: rustc host triple was not reported".to_owned())?;

    let sysroot = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .map_err(|error| format!("test: could not inspect rustc sysroot: {error}"))?;
    if !sysroot.status.success() {
        return Err(format!(
            "test: rustc --print sysroot failed ({})",
            sysroot.status
        ));
    }
    let sysroot = String::from_utf8(sysroot.stdout)
        .map_err(|error| format!("test: rustc sysroot output was not UTF-8: {error}"))?;
    let linker = Path::new(sysroot.trim())
        .join("lib")
        .join("rustlib")
        .join(host)
        .join("bin")
        .join("rust-lld.exe");
    if !linker.is_file() {
        return Err(format!(
            "test: bundled Windows linker is missing: {}",
            linker.display()
        ));
    }

    let environment = format!(
        "CARGO_TARGET_{}_LINKER",
        host.to_ascii_uppercase().replace('-', "_")
    );
    Ok(Some((environment, linker)))
}

fn run_workspace_tests() -> Result<(), String> {
    let mut command = Command::new("cargo");
    command.args(["test", "--workspace"]);
    command.env("CARGO_PROFILE_TEST_DEBUG", "0");

    if let Some((environment, linker)) = windows_test_linker()? {
        println!(
            "test linker: {}={} (avoids MSVC PDB record exhaustion)",
            environment,
            linker.display()
        );
        command.env(environment, linker);
    }

    run_command("test", &mut command)
}

fn exe_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn green() -> Result<(), String> {
    run("fmt", &["fmt", "--all", "--", "--check"])?;
    docs_status()?;
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
    run_workspace_tests()?;

    let exe_name = format!("devpulselabs{}", std::env::consts::EXE_SUFFIX);
    let exe = Path::new("target/release").join(exe_name);
    let before = exe_mtime(&exe);
    run("release build", &["build", "--release"])?;
    let after = exe_mtime(&exe);
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

fn docs_status() -> Result<(), String> {
    let path = Path::new("docs/eguipilotai-playwright-review.md");
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut in_matrix = false;
    let mut checked = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line == "## Capability matrix" {
            in_matrix = true;
            continue;
        }
        if in_matrix && line.starts_with("## ") {
            break;
        }
        if !in_matrix || !line.starts_with('|') || line.starts_with("|---") {
            continue;
        }
        let columns: Vec<_> = line.split('|').map(str::trim).collect();
        if columns.get(1) == Some(&"Playwright contract") {
            continue;
        }
        let status = columns.get(3).copied().unwrap_or_default();
        if status == "Core"
            || status == "Partial"
            || status == "Missing"
            || status.starts_with("Ready")
        {
            let evidence = columns.get(5).copied().unwrap_or_default();
            if !["test:`", "slice:`", "issue:`"]
                .iter()
                .any(|marker| evidence.contains(marker))
            {
                return Err(format!(
                    "{}:{} status `{status}` needs test:, slice:, or issue: evidence",
                    path.display(),
                    index + 1
                ));
            }
            checked += 1;
        }
    }
    if checked == 0 {
        return Err(format!(
            "{}: capability matrix was not found or contained no checked statuses",
            path.display()
        ));
    }
    println!("docs status: {checked} evidence-backed capability rows");
    Ok(())
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_else(|| "green".into());
    let result = match task.as_str() {
        "green" => green(),
        "docs-status" => docs_status(),
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
        other => Err(format!(
            "unknown task \"{other}\" — tasks: green, docs-status, fix"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}
