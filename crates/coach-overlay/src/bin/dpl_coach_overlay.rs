//! `dpl-coach-overlay` — foreign / whole-monitor coach chrome (devtools-only).
//!
//! ADR: `docs/plans/coach-overlay-adr.md`
//! Plan: `docs/plans/coach-c2b-overlay-daemon.md`

#[cfg(target_os = "linux")]
#[path = "../linux_self_test.rs"]
mod linux;

#[cfg(target_os = "linux")]
fn main() {
    linux::entry();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("dpl-coach-overlay: Linux/X11 backend only in C2b pass 1 (Windows stub)");
    std::process::exit(2);
}
