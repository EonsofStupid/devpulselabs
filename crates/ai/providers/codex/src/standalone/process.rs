//! Private lifetime plumbing for the fixed Codex standalone command.
//!
//! Windows gets an owned job assigned before its first instruction. Explicit
//! cleanup signals the job and boundedly reaps the direct child. The wrapper
//! cannot confirm job emptiness and does NOT set kill-on-job-close. Cleanup
//! therefore returns an explicit error so the caller retains its lease.
//! These remain qualification blockers. Unix terminal foreground ownership is
//! not implemented, so launching there is explicitly refused before spawn.

use std::io;
use std::process::{Command, ExitStatus};

#[cfg(windows)]
use process_wrap::std::{ChildWrapper, CommandWrap, JobObject};
#[cfg(windows)]
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(super) struct OwnedProcess {
    #[cfg(windows)]
    child: Option<Box<dyn ChildWrapper>>,
    #[cfg(windows)]
    exit: Option<ExitStatus>,
    #[cfg(windows)]
    cleanup_unconfirmed: bool,
}

impl OwnedProcess {
    pub(super) fn spawn(command: Command) -> io::Result<Self> {
        #[cfg(windows)]
        {
            // JobObject's pre-spawn hook overwrites Command creation flags with
            // temporary CREATE_SUSPENDED, assigns the job, then resumes. With
            // no CreationFlags wrapper, this shares the invoking console.
            let mut command = CommandWrap::from(command);
            command.wrap(JobObject);
            Ok(Self {
                child: Some(command.spawn()?),
                exit: None,
                cleanup_unconfirmed: false,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = command;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Standalone Codex terminal ownership is not implemented on this platform",
            ))
        }
    }

    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        #[cfg(windows)]
        {
            if let Some(child) = &mut self.child {
                // Do not consume completion-port notifications via the
                // wrapper's try_wait: it only reports direct-child status.
                // We install exactly one wrapper (JobObject). Borrow its
                // direct inner child without consuming/unwrapping the owner.
                self.exit = child.inner_mut().try_wait()?;
            }
            Ok(self.exit)
        }
        #[cfg(not(windows))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "No standalone process",
            ))
        }
    }

    pub(super) fn terminate_and_reap(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            if !self.cleanup_unconfirmed {
                if let Some(child) = self.child.as_mut() {
                    // Even a finished direct child may have descendants in its job.
                    child.start_kill()?;
                }
                let started = Instant::now();
                while self.try_wait()?.is_none() {
                    if started.elapsed() >= Duration::from_secs(5) {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "Codex child cleanup exceeded its deadline",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                self.cleanup_unconfirmed = true;
            }
            // process-wrap 10.0.1's wait_on_job accepts any completion packet;
            // it does not check ACTIVE_PROCESS_ZERO. Neither wait API can
            // certify that every descendant has exited. Never return success
            // and authorize a profile relocation from that incomplete evidence.
            Err(io::Error::new(io::ErrorKind::Unsupported,
                "Codex was stopped, but complete process-tree cleanup is unconfirmed; its profile lease is retained"))
        }
        #[cfg(not(windows))]
        Ok(())
    }
}

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.terminate_and_reap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // spawn-audit: exempt(test-only fixed current-exe lifecycle fixtures)

    #[cfg(not(windows))]
    #[test]
    fn unsupported_terminal_refuses_before_executable_access() {
        let command = Command::new("missing-codex-test-must-not-run");
        assert_eq!(
            OwnedProcess::spawn(command).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[cfg(windows)]
    fn fixture(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "standalone::process::tests::process_fixture",
                "--nocapture",
            ])
            .env("DPL_CODEX_PROCESS_FIXTURE", mode);
        command
    }

    #[cfg(windows)]
    #[test]
    fn process_fixture() {
        match std::env::var("DPL_CODEX_PROCESS_FIXTURE").as_deref() {
            Ok("exit") => std::process::exit(7),
            Ok("hang") => std::thread::sleep(Duration::from_secs(10)),
            Ok("descendant") => {
                println!("descendant-ready");
                std::thread::sleep(Duration::from_secs(10));
            }
            Ok("tree") => {
                let mut descendant = fixture("descendant").spawn().unwrap();
                // The descendant inherits the top-level fixture's stdout pipe.
                // The parent stays alive until job cleanup terminates both.
                descendant.wait().unwrap();
            }
            _ => {}
        }
    }

    #[cfg(windows)]
    #[test]
    fn owned_job_reaps_running_child_and_cleanup_is_idempotent() {
        let mut child = OwnedProcess::spawn(fixture("hang")).unwrap();
        assert!(child.try_wait().unwrap().is_none());
        assert_eq!(
            child.terminate_and_reap().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(child.try_wait().unwrap().is_some());
        assert_eq!(
            child.terminate_and_reap().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[cfg(windows)]
    #[test]
    fn owned_job_preserves_exit_status_before_cleanup() {
        let mut child = OwnedProcess::spawn(fixture("exit")).unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert_eq!(status.code(), Some(7));
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            child.terminate_and_reap().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(child.try_wait().unwrap().unwrap().code(), Some(7));
    }

    #[cfg(windows)]
    #[test]
    fn dropping_owned_job_closes_descendant_pipe_before_natural_exit() {
        use std::io::{BufRead, BufReader, Read};
        use std::process::Stdio;
        use std::sync::mpsc;

        let mut command = fixture("tree");
        command.stdout(Stdio::piped()).stderr(Stdio::null());
        let mut child = OwnedProcess::spawn(command).unwrap();
        let pipe = child.child.as_mut().unwrap().stdout().take().unwrap();
        let (ready_send, ready_recv) = mpsc::sync_channel(1);
        let (closed_send, closed_recv) = mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let mut pipe = BufReader::new(pipe);
            let mut line = String::new();
            loop {
                line.clear();
                if pipe.read_line(&mut line).unwrap() == 0 {
                    return;
                }
                if line.trim() == "descendant-ready" {
                    ready_send.send(()).unwrap();
                    break;
                }
            }
            let mut remaining = Vec::new();
            pipe.read_to_end(&mut remaining).unwrap();
            closed_send.send(()).unwrap();
        });
        ready_recv.recv_timeout(Duration::from_secs(5)).unwrap();
        let started = Instant::now();
        drop(child);
        closed_recv.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(started.elapsed() < Duration::from_secs(5));
        reader.join().unwrap();
    }
}
