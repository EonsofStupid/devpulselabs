//! Does `mouse_passthrough` deliver clicks to a window underneath?
//! Linux/X11 only (XTEST). Plan: `docs/plans/p2b-overlay-passthrough.md`.

#[cfg(target_os = "linux")]
#[path = "passthrough_probe_linux.rs"]
mod linux;

#[cfg(target_os = "linux")]
fn main() -> eframe::Result<()> {
    linux::entry()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("passthrough-probe is Linux/X11 only");
    std::process::exit(2);
}
