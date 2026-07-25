//! Windows-only proof harness for the real DevPULSE Chat peer viewports.
//!
//! The capture engine intentionally excludes windows owned by its own process
//! on Windows. Keeping this harness out-of-process both avoids that deadlock
//! guard and proves that Chat is a discoverable native OS window.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_TITLE: &str = "DevPULSE Chat";
const WINDOW_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const STABLE_POLLS: u8 = 6;

fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("chat-native-proof is Windows-only");
        std::process::exit(2);
    }

    #[cfg(target_os = "windows")]
    {
        if let Err(error) = ensure_dpi_aware_process() {
            eprintln!("chat-native-proof FAILED: {error}");
            std::process::exit(1);
        }
        if let Err(error) = run() {
            eprintln!("chat-native-proof FAILED: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(target_os = "windows")]
fn run() -> Result<(), String> {
    let arguments = std::env::args().collect::<Vec<_>>();
    let output = argument_value(&arguments, "--out")
        .map(PathBuf::from)
        .ok_or_else(|| {
            "usage: chat-native-proof --out <png> [--state approval|ai|social|hover|grok|compact|snapped] [--title <exact title>] [--app <exe>]".to_owned()
        })?;
    let title = argument_value(&arguments, "--title").unwrap_or_else(|| DEFAULT_TITLE.to_owned());
    let state = argument_value(&arguments, "--state").unwrap_or_else(|| "approval".to_owned());
    if !matches!(
        state.as_str(),
        "approval" | "ai" | "social" | "hover" | "grok" | "compact" | "snapped"
    ) {
        return Err(format!("unknown native Chat proof state {state:?}"));
    }
    let verbose = arguments.iter().any(|argument| argument == "--verbose");
    if verbose {
        for monitor in dpl_engine_capture::list_monitors() {
            eprintln!(
                "monitor {}: {}x{} scale={} primary={}",
                monitor.name,
                monitor.width,
                monitor.height,
                monitor.scale_factor,
                monitor.is_primary
            );
        }
    }
    let app = argument_value(&arguments, "--app")
        .map(PathBuf::from)
        .unwrap_or(default_app_path()?);
    if !app.is_file() {
        return Err(format!(
            "DevPULSE executable does not exist: {}",
            app.display()
        ));
    }

    let child = Command::new(&app)
        .args([
            "--chat-open",
            "--chat-authored",
            "--chat-proof",
            &state,
            "--mood",
            "0",
            "--no-seam",
        ])
        .env(
            "RUST_LOG",
            if verbose {
                "devpulselabs::chat=debug"
            } else {
                "off"
            },
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .spawn()
        .map_err(|error| format!("could not launch {}: {error}", app.display()))?;
    let mut child = ChildGuard::new(child);

    let snapped = state == "snapped";
    let windows = if snapped {
        let candidates = wait_for_stable_windows(
            &mut child,
            "Chat and its snapped utility",
            |window| window.title.starts_with(DEFAULT_TITLE),
            |windows| {
                windows.iter().any(|window| window.title == DEFAULT_TITLE)
                    && windows.iter().any(|window| window.title != DEFAULT_TITLE)
            },
        )?;
        let primary = candidates
            .iter()
            .find(|window| window.title == DEFAULT_TITLE)
            .cloned()
            .ok_or_else(|| "primary Chat window disappeared before capture".to_owned())?;
        let utility = candidates
            .iter()
            .find(|window| window.title != DEFAULT_TITLE)
            .cloned()
            .ok_or_else(|| "snapped utility disappeared before capture".to_owned())?;
        vec![primary, utility]
    } else {
        wait_for_stable_windows(
            &mut child,
            &format!("{title:?}"),
            |window| window.title == title,
            |windows| !windows.is_empty(),
        )?
    };

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    if snapped {
        capture_snapped_composition(&output, &windows)?;
    } else {
        let window = windows
            .first()
            .ok_or_else(|| format!("{title:?} disappeared before capture"))?;
        dpl_engine_capture::still_window_to(&output, window.id)
            .map_err(|error| format!("could not capture {title:?}: {error}"))?;
    }
    child.stop();
    if snapped {
        println!(
            "captured Chat plus snapped utility composition from {} native windows to {}",
            windows.len(),
            output.display()
        );
    } else if let Some(window) = windows.first() {
        println!(
            "captured {title:?} at ({}, {}) {}x{} to {}",
            window.x,
            window.y,
            window.width,
            window.height,
            output.display()
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn wait_for_stable_windows(
    child: &mut ChildGuard,
    description: &str,
    predicate: impl Fn(&dpl_engine_capture::WindowInfo) -> bool,
    ready: impl Fn(&[dpl_engine_capture::WindowInfo]) -> bool,
) -> Result<Vec<dpl_engine_capture::WindowInfo>, String> {
    let started = Instant::now();
    let mut last_geometry = None;
    let mut stable_polls = 0_u8;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("could not inspect DevPULSE process: {error}"))?
        {
            return Err(format!(
                "DevPULSE exited before {description} could be captured ({status})"
            ));
        }

        let (_, windows) = dpl_engine_capture::list_targets();
        let mut windows = windows
            .into_iter()
            .filter(|window| predicate(window))
            .collect::<Vec<_>>();
        windows.sort_by(|left, right| {
            left.x
                .cmp(&right.x)
                .then_with(|| left.y.cmp(&right.y))
                .then_with(|| left.title.cmp(&right.title))
        });
        if ready(&windows) {
            let geometry = windows
                .iter()
                .map(|window| (window.id, window.x, window.y, window.width, window.height))
                .collect::<Vec<_>>();
            if last_geometry.as_ref() == Some(&geometry) {
                stable_polls = stable_polls.saturating_add(1);
            } else {
                last_geometry = Some(geometry);
                stable_polls = 1;
            }
            if stable_polls >= STABLE_POLLS {
                return Ok(windows);
            }
        } else {
            last_geometry = None;
            stable_polls = 0;
        }

        if started.elapsed() >= WINDOW_TIMEOUT {
            return Err(format!(
                "{description} was not discoverable as stable native windows within {} seconds",
                WINDOW_TIMEOUT.as_secs()
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(target_os = "windows")]
fn capture_snapped_composition(
    output: &std::path::Path,
    windows: &[dpl_engine_capture::WindowInfo],
) -> Result<(), String> {
    let mut captures = Vec::with_capacity(windows.len());
    for window in windows {
        let temporary = std::env::temp_dir().join(format!(
            "devpulse-chat-proof-{}-{}.png",
            std::process::id(),
            window.id
        ));
        dpl_engine_capture::still_window_to(&temporary, window.id)
            .map_err(|error| format!("could not capture {:?}: {error}", window.title))?;
        let decoded = image::open(&temporary);
        let _ = std::fs::remove_file(&temporary);
        let image = decoded
            .map_err(|error| format!("could not read {}: {error}", temporary.display()))?
            .to_rgba8();
        captures.push((window, image));
    }

    let min_x = captures
        .iter()
        .map(|(window, _)| window.x)
        .min()
        .ok_or_else(|| "snapped proof has no windows".to_owned())?;
    let min_y = captures
        .iter()
        .map(|(window, _)| window.y)
        .min()
        .ok_or_else(|| "snapped proof has no windows".to_owned())?;
    let max_x = captures
        .iter()
        .map(|(window, image)| window.x.saturating_add_unsigned(image.width()))
        .max()
        .ok_or_else(|| "snapped proof has no windows".to_owned())?;
    let max_y = captures
        .iter()
        .map(|(window, image)| window.y.saturating_add_unsigned(image.height()))
        .max()
        .ok_or_else(|| "snapped proof has no windows".to_owned())?;
    let width = u32::try_from(max_x - min_x)
        .map_err(|_| "snapped proof width is outside the supported range".to_owned())?;
    let height = u32::try_from(max_y - min_y)
        .map_err(|_| "snapped proof height is outside the supported range".to_owned())?;
    let mut composition = image::RgbaImage::from_pixel(width, height, image::Rgba([3, 9, 12, 255]));
    for (window, image) in captures {
        image::imageops::overlay(
            &mut composition,
            &image,
            i64::from(window.x - min_x),
            i64::from(window.y - min_y),
        );
    }
    composition
        .save(output)
        .map_err(|error| format!("could not save {}: {error}", output.display()))
}

#[cfg(target_os = "windows")]
fn ensure_dpi_aware_process() -> Result<(), String> {
    const READY: &str = "DPL_CHAT_PROOF_DPI_READY";
    if std::env::var_os(READY).is_some() {
        return Ok(());
    }

    // Windows applies the HighDpiAware compatibility layer while loading the
    // child executable, before xcap asks for any geometry. Re-executing here
    // keeps the capture process and the eframe process in the same DPI mode
    // without unsafe process-wide API calls.
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not resolve proof executable: {error}"))?;
    let status = Command::new(executable)
        .args(std::env::args_os().skip(1))
        .env("__COMPAT_LAYER", "HighDpiAware")
        .env(READY, "1")
        .status()
        .map_err(|error| format!("could not start DPI-aware proof process: {error}"))?;
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(target_os = "windows")]
fn argument_value(arguments: &[String], flag: &str) -> Option<String> {
    arguments
        .iter()
        .position(|argument| argument == flag)
        .and_then(|index| arguments.get(index + 1))
        .cloned()
}

#[cfg(target_os = "windows")]
fn default_app_path() -> Result<PathBuf, String> {
    let harness = std::env::current_exe()
        .map_err(|error| format!("could not resolve proof executable: {error}"))?;
    let directory = harness
        .parent()
        .ok_or_else(|| format!("proof executable has no parent: {}", harness.display()))?;
    Ok(directory.join("devpulselabs.exe"))
}

#[cfg(target_os = "windows")]
struct ChildGuard {
    child: Child,
    stopped: bool,
}

#[cfg(target_os = "windows")]
impl ChildGuard {
    const fn new(child: Child) -> Self {
        Self {
            child,
            stopped: false,
        }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }

    fn stop(&mut self) {
        if self.stopped {
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.stopped = true;
    }
}

#[cfg(target_os = "windows")]
impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.stop();
    }
}
