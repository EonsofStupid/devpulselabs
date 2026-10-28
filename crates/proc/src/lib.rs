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

use std::io::{self, Read};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_READERS: usize = 16;
static READERS: AtomicUsize = AtomicUsize::new(0);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

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

struct ReaderSlot;

impl ReaderSlot {
    fn acquire() -> Option<Self> {
        READERS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used < MAX_READERS).then_some(used + 1)
            })
            .ok()
            .map(|_| Self)
    }
}

impl Drop for ReaderSlot {
    fn drop(&mut self) {
        READERS.fetch_sub(1, Ordering::AcqRel);
    }
}

type Captured = (bool, io::Result<Vec<u8>>);

fn capture_pipe(
    pipe: impl Read + Send + 'static,
    stdout: bool,
    sender: mpsc::SyncSender<Captured>,
    slot: ReaderSlot,
    limit: u64,
) -> io::Result<()> {
    std::thread::Builder::new()
        .name("devpulse-bounded-output".into())
        .spawn(move || {
            let _slot = slot;
            let mut bytes = Vec::new();
            let result = pipe.take(limit).read_to_end(&mut bytes).and_then(|_| {
                if bytes.len() as u64 == limit {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "provider probe exceeded its output limit",
                    ))
                } else {
                    Ok(bytes)
                }
            });
            let _ = sender.send((stdout, result));
        })
        .map(|_| ())
}

fn stop_owned_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

/// Capture a caller-authored named-provider probe with finite output and time.
/// This is plumbing, not a command adapter: callers own executable resolution,
/// arguments, environment and the applicable provider launch gates.
///
/// Null stdin and piped stdout/stderr replace the command's stdio settings.
/// Exceeding either stream limit returns InvalidData; timeout returns TimedOut;
/// a full reader budget returns WouldBlock without spawning. Nonzero child
/// exits are returned normally so callers can classify provider-owned status.
///
/// On failure the exact child is killed/reaped. Escaped descendants are not
/// owned here and can keep inherited pipes open; readers are never joined and
/// retain one of 16 global slots until those pipes close. This bounds detached
/// readers without falsely claiming process-tree termination.
pub fn bounded_output(
    command: &mut Command,
    timeout: Duration,
    max_per_stream: usize,
) -> io::Result<Output> {
    let started = Instant::now();
    let limit = u64::try_from(max_per_stream)
        .ok()
        .and_then(|size| size.checked_add(1))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid output limit"))?;
    if timeout.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "provider probe timed out",
        ));
    }
    let (Some(stdout_slot), Some(stderr_slot)) = (ReaderSlot::acquire(), ReaderSlot::acquire())
    else {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "earlier provider probes are still closing their output",
        ));
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = quiet(command).spawn()?;
    let stdout = child.stdout.take().expect("bounded stdout is piped");
    let stderr = child.stderr.take().expect("bounded stderr is piped");
    let (sender, receiver) = mpsc::sync_channel(2);
    if let Err(error) = capture_pipe(stdout, true, sender.clone(), stdout_slot, limit)
        .and_then(|()| capture_pipe(stderr, false, sender.clone(), stderr_slot, limit))
    {
        stop_owned_child(&mut child);
        return Err(error);
    }
    drop(sender);
    let mut stdout = None;
    let mut stderr = None;
    loop {
        for (is_stdout, result) in receiver.try_iter() {
            match result {
                Ok(bytes) => {
                    if is_stdout {
                        stdout = Some(bytes);
                    } else {
                        stderr = Some(bytes);
                    }
                }
                Err(error) => {
                    stop_owned_child(&mut child);
                    return Err(error);
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) if stdout.is_some() && stderr.is_some() => {
                return Ok(Output {
                    status,
                    stdout: stdout.unwrap(),
                    stderr: stderr.unwrap(),
                });
            }
            Ok(_) => {}
            Err(error) => {
                stop_owned_child(&mut child);
                return Err(error);
            }
        }
        if started.elapsed() >= timeout {
            stop_owned_child(&mut child);
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "provider probe timed out",
            ));
        }
        std::thread::sleep(POLL_INTERVAL.min(timeout.saturating_sub(started.elapsed())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "tests::bounded_process_fixture", "--nocapture"])
            .env("DPL_PROC_CAPTURE_FIXTURE", mode);
        command
    }

    #[test]
    fn bounded_process_fixture() {
        use std::io::Write as _;
        match std::env::var("DPL_PROC_CAPTURE_FIXTURE").as_deref() {
            Ok("streams") => {
                std::io::stdout().write_all(b"stdout-only").unwrap();
                std::io::stderr().write_all(b"stderr-only").unwrap();
                std::process::exit(7);
            }
            Ok("hang") => std::thread::sleep(Duration::from_secs(10)),
            Ok("flood") => {
                let _ = std::io::stderr().write_all(&vec![b'x'; 128 * 1024]);
                println!("stdout finished");
            }
            Ok("descendant") => {
                let mut command = fixture("pipe-holder");
                command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
                let mut child = quiet(&mut command).spawn().unwrap();
                // The probe deadline kills this waiting fixture while its
                // descendant still owns the inherited output pipes.
                child.wait().unwrap();
            }
            Ok("pipe-holder") => std::thread::sleep(Duration::from_secs(4)),
            Ok("reader-budget") => {
                let slots: Vec<_> = (0..MAX_READERS)
                    .map(|_| ReaderSlot::acquire().unwrap())
                    .collect();
                let mut never_spawn = Command::new("dpl-no-such-probe-binary");
                assert_eq!(
                    bounded_output(&mut never_spawn, Duration::from_secs(1), 1024)
                        .unwrap_err()
                        .kind(),
                    io::ErrorKind::WouldBlock
                );
                drop(slots);
                assert!(ReaderSlot::acquire().is_some());
            }
            _ => {}
        }
    }

    #[test]
    fn capture_keeps_streams_and_nonzero_exit_for_caller_classification() {
        let output =
            bounded_output(&mut fixture("streams"), Duration::from_secs(5), 16 * 1024).unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert!(String::from_utf8_lossy(&output.stdout).contains("stdout-only"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("stderr-only"));
        assert_eq!(output.stderr, b"stderr-only");
    }

    #[test]
    fn deadline_flood_and_descendant_pipes_cannot_block_following_probe() {
        for (mode, kind, timeout) in [
            ("hang", io::ErrorKind::TimedOut, Duration::from_millis(200)),
            ("flood", io::ErrorKind::InvalidData, Duration::from_secs(2)),
            (
                "descendant",
                io::ErrorKind::TimedOut,
                Duration::from_millis(200),
            ),
        ] {
            let started = Instant::now();
            let error = bounded_output(&mut fixture(mode), timeout, 16 * 1024).unwrap_err();
            assert_eq!(error.kind(), kind, "{mode}");
            assert!(started.elapsed() < Duration::from_secs(3), "{mode}");
        }
        assert_eq!(
            bounded_output(&mut fixture("streams"), Duration::from_secs(2), 16 * 1024)
                .unwrap()
                .status
                .code(),
            Some(7)
        );
    }

    #[test]
    fn reader_budget_refuses_before_spawn_and_releases_finished_slots() {
        let output = bounded_output(
            &mut fixture("reader-budget"),
            Duration::from_secs(5),
            16 * 1024,
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

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
