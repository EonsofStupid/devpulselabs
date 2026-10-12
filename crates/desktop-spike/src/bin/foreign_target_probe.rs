//! C2s foreign-target spike — Linux/X11 only.
//!
//! Plan: `docs-history-2026-10-12:docs/plans/coach-c2s-foreign-target-spike.md`.

#[cfg(target_os = "linux")]
#[path = "foreign_target_probe_linux.rs"]
mod linux;

#[cfg(target_os = "linux")]
fn main() {
    linux::entry();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("foreign-target-probe is Linux/X11 only");
    std::process::exit(2);
}
