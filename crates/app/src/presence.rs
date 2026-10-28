//! Provider PRESENCE — asking each vendor's own tool whether the operator is
//! already signed in on this machine (A1).
//!
//! > "When I open this laptop and I work in grok build subscription and if i
//! > have chatGPT already open that means this system is auth already the
//! > same as claude, they are running and we can absolutely have devpulse
//! > labs despite being portable have that awareness."
//!
//! The law this obeys: **presence is probed, never scraped.** No cookie jar,
//! no credential store, no keychain. Each provider names a non-secret status
//! subcommand in its own crate as plain data ([`PresenceSpec`]); this module
//! is the only place that spawns anything, and it does so on a worker thread
//! so the egui thread never waits on a provider process.
//!
//! Every command here was read off the real tool on the operator's machine:
//!
//! | provider | command | signed-in output |
//! |---|---|---|
//! | Claude Code 2.1.220 | `claude auth status` | JSON `"loggedIn": true` |
//! | Grok 0.2.117 | `grok models` | `You are logged in with grok.com.` |
//! | Codex 0.146.0 | `codex login status` | `Logged in using ChatGPT` |
//! | Cursor | `cursor-agent status` | not installed here — unverified |

use dpl_chat_core::{PresenceSpec, PresenceVerdict, ProviderId};

use std::process::{Command, Stdio};

use std::time::Duration;

/// Stop reading a tool's output after this much. A status command prints a
/// line or a small JSON object; anything larger is a tool misbehaving, and
/// an unbounded read would hand it the host's memory.
const MAX_OUTPUT: usize = 16 * 1024;
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// The OS of this running app, not a machine fact saved in a portable project.
pub const fn platform_label() -> &'static str {
    if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Other operating system"
    }
}

fn unknown(provider: ProviderId, note: String) -> Presence {
    Presence {
        provider,
        verdict: PresenceVerdict::Unknown,
        note: Some(note),
    }
}

/// One provider's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    pub provider: ProviderId,
    pub verdict: PresenceVerdict,
    /// A short non-secret note for the operator — why the verdict is what it
    /// is. Never the tool's raw output, which could carry an account detail
    /// nobody asked to display.
    pub note: Option<String>,
}

/// Run one probe to completion. Blocking — callers use [`probe_all`], which
/// owns the thread.
pub fn probe(provider: ProviderId, spec: &PresenceSpec) -> Presence {
    let mut command = Command::new(spec.program);
    command.args(spec.args);
    probe_command(provider, spec, command, PROBE_TIMEOUT)
}

// Only the declared provider probe above supplies this command in production.
// Tests substitute the current test executable, never a live AI or credential.
fn probe_command(
    provider: ProviderId,
    spec: &PresenceSpec,
    mut command: Command,
    timeout: Duration,
) -> Presence {
    let captured = match dpl_proc::bounded_output(&mut command, timeout, MAX_OUTPUT) {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Presence {
                provider,
                verdict: PresenceVerdict::NotInstalled,
                note: Some(format!("{} is not installed.", spec.program)),
            };
        }
        Err(error) => {
            let note = match error.kind() {
                std::io::ErrorKind::TimedOut => {
                    format!("{} status check timed out. Try again later.", spec.program)
                }
                std::io::ErrorKind::InvalidData => format!(
                    "{} returned unreadable or excessive status output.",
                    spec.program
                ),
                std::io::ErrorKind::WouldBlock => {
                    "Earlier provider checks are still closing their output. Try again later."
                        .into()
                }
                _ => format!("{} could not be run: {error}", spec.program),
            };
            return unknown(provider, note);
        }
    };
    let status = captured.status;
    let output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&captured.stdout),
        String::from_utf8_lossy(&captured.stderr)
    );
    let classified = spec.classify(&output);
    // Several vendors use a nonzero exit for a normal signed-out verdict.
    // An unsuccessful process must never be promoted to SignedIn merely
    // because an error happened to repeat that phrase.
    let verdict = if status.success() || classified == PresenceVerdict::SignedOut {
        classified
    } else {
        PresenceVerdict::Unknown
    };
    let note = match verdict {
        PresenceVerdict::SignedIn => None,
        PresenceVerdict::SignedOut => Some(format!(
            "{} is installed but signed out — sign in to activate it.",
            spec.program
        )),
        PresenceVerdict::NotInstalled => Some(format!("{} is not installed.", spec.program)),
        PresenceVerdict::Unknown if !status.success() => Some(format!(
            "{} status command failed; sign-in could not be verified.",
            spec.program
        )),
        PresenceVerdict::Unknown => Some(format!(
            "{} ran but its status output was not recognized.",
            spec.program
        )),
    };
    Presence {
        provider,
        verdict,
        note,
    }
}

/// Every provider that declares a probe, run on ONE background thread.
///
/// One coordinator checks providers in sequence; each probe has two bounded
/// pipe readers. The receiver is polled from the frame loop, never awaited.
pub fn probe_all(
    specs: Vec<(ProviderId, PresenceSpec)>,
) -> std::sync::mpsc::Receiver<Vec<Presence>> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("devpulse-provider-presence".to_owned())
        .spawn(move || {
            let found = specs
                .iter()
                .map(|(provider, spec)| probe(*provider, spec))
                .collect();
            let _ = sender.send(found);
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "provider presence probe could not start");
    }
    receiver
}

/// How long to keep watching for a sign-in to take before giving up. The
/// operator has to find the browser tab, pick an account, and approve — a
/// short window would report failure while they were still typing.
const SIGN_IN_WINDOW: std::time::Duration = std::time::Duration::from_secs(180);
const SIGN_IN_POLL: std::time::Duration = std::time::Duration::from_secs(2);

/// Hand sign-in to the vendor's own tool, then watch until it takes (A2).
///
/// The CLI opens the operator's DEFAULT browser itself and runs the vendor's
/// real flow, so DevPULSE never sees a credential and never has to embed a
/// login form. That is also why the in-app browser is not involved: these are
/// different tools with different trust.
///
/// Returns a receiver that yields ONE verdict — the moment presence flips to
/// signed in, or the last verdict when the window closes. `Err` when the
/// provider declares no sign-in verb, which is not something to invent.
pub fn begin_sign_in(
    provider: ProviderId,
    spec: PresenceSpec,
) -> Result<std::sync::mpsc::Receiver<Vec<Presence>>, String> {
    if spec.login.is_empty() {
        return Err(format!("{} has no CLI sign-in", spec.program));
    }
    let mut command = Command::new(spec.program);
    command
        .args(spec.login)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Quiet for the same reason the probes are: no console flash on Windows.
    let mut child = dpl_proc::quiet(&mut command)
        .spawn()
        .map_err(|error| format!("{} sign-in could not start: {error}", spec.program))?;

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("devpulse-provider-sign-in".to_owned())
        .spawn(move || {
            let started = std::time::Instant::now();
            let mut last = Presence {
                provider,
                verdict: PresenceVerdict::Unknown,
                note: None,
            };
            while started.elapsed() < SIGN_IN_WINDOW {
                std::thread::sleep(SIGN_IN_POLL);
                last = probe(provider, &spec);
                if last.verdict == PresenceVerdict::SignedIn {
                    break;
                }
            }
            // The login process is the operator's; if it is still sitting
            // there when the window closes, stop waiting on it rather than
            // leaving a zombie behind.
            let _ = child.kill();
            let _ = child.wait();
            let _ = sender.send(vec![last]);
        });
    if let Err(error) = spawned {
        return Err(format!("the sign-in watcher could not start: {error}"));
    }
    Ok(receiver)
}

/// The providers that can answer for themselves, in rail order.
pub fn specs() -> Vec<(ProviderId, PresenceSpec)> {
    vec![
        (dpl_chat_codex::PROVIDER, CODEX),
        (dpl_chat_claude::PROVIDER, dpl_chat_claude::PRESENCE),
        (dpl_chat_grok::PROVIDER, dpl_chat_grok::PRESENCE),
        (dpl_chat_cursor::PROVIDER, dpl_chat_cursor::PRESENCE),
        (dpl_chat_gemini::PROVIDER, dpl_chat_gemini::PRESENCE),
    ]
}

/// Codex already owns a full activation coordinator with its own launcher and
/// runtime; this spec exists so the ROSTER can answer for Codex on the same
/// footing as the rest without going through that ceremony.
const CODEX: PresenceSpec = PresenceSpec {
    program: "codex",
    args: &["login", "status"],
    signed_in: &["logged in using chatgpt", "logged in"],
    signed_out: &["not logged in"],
    // Codex's own coordinator owns its login ceremony (device code + system
    // browser); the roster only needs to READ its presence, so no sign-in
    // verb is declared here and `activate_provider` keeps routing Codex to
    // that coordinator.
    login: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const FIXTURE_SPEC: PresenceSpec = PresenceSpec {
        program: "presence-test-fixture",
        args: &[],
        signed_in: &["logged in"],
        signed_out: &["not logged in"],
        login: &[],
    };

    fn fixture_command(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "presence::tests::probe_process_fixture",
                "--nocapture",
            ])
            .env("DPL_PRESENCE_TEST_FIXTURE", mode);
        command
    }

    fn fixture_probe(mode: &str, timeout: Duration) -> Presence {
        probe_command(
            ProviderId("presence-test"),
            &FIXTURE_SPEC,
            fixture_command(mode),
            timeout,
        )
    }

    /// Runs only in a child copy of the test binary. No provider is launched
    /// and no machine account or environment is changed in the parent.
    #[test]
    fn probe_process_fixture() {
        use std::io::Write as _;
        match std::env::var("DPL_PRESENCE_TEST_FIXTURE").as_deref() {
            Ok("slow") => std::thread::sleep(Duration::from_secs(10)),
            Ok("flood") => {
                // stdout remains open while stderr exceeds its pipe buffer:
                // sequential stdout-then-stderr reads would deadlock here.
                let _ = std::io::stderr().write_all(&vec![b'x'; MAX_OUTPUT * 16]);
                println!("logged in");
            }
            Ok("failure") => {
                println!("logged in");
                std::process::exit(1);
            }
            Ok("signed-out") => {
                println!("not logged in");
                std::process::exit(1);
            }
            Ok("signed-in") => println!("logged in"),
            Ok("inherited-pipe") => {
                let mut command = fixture_command("brief-pipe-holder");
                command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
                let mut child = dpl_proc::quiet(&mut command).spawn().unwrap();
                println!("logged in");
                // The probe timeout kills this waiting parent; the brief
                // descendant continues holding the inherited output pipes.
                child.wait().unwrap();
            }
            Ok("brief-pipe-holder") => std::thread::sleep(Duration::from_secs(4)),
            _ => {}
        }
    }

    #[test]
    fn a_hanging_probe_times_out_and_the_next_probe_can_run() {
        let started = Instant::now();
        let timed_out = fixture_probe("slow", Duration::from_millis(150));
        assert_eq!(timed_out.verdict, PresenceVerdict::Unknown);
        assert!(timed_out.note.unwrap().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert_eq!(
            fixture_probe("signed-in", Duration::from_secs(5)).verdict,
            PresenceVerdict::SignedIn
        );
    }

    #[test]
    fn excessive_stderr_is_refused_without_waiting_for_stdout() {
        let found = fixture_probe("flood", Duration::from_secs(5));
        assert_eq!(found.verdict, PresenceVerdict::Unknown);
        assert!(found.note.unwrap().contains("excessive"));
    }

    #[test]
    fn a_failed_status_cannot_claim_signed_in_but_can_report_signed_out() {
        assert_eq!(
            fixture_probe("failure", Duration::from_secs(5)).verdict,
            PresenceVerdict::Unknown
        );
        assert_eq!(
            fixture_probe("signed-out", Duration::from_secs(5)).verdict,
            PresenceVerdict::SignedOut
        );
    }

    #[test]
    fn an_inherited_pipe_cannot_extend_the_probe_deadline() {
        let started = Instant::now();
        let found = fixture_probe("inherited-pipe", Duration::from_millis(200));
        assert_eq!(found.verdict, PresenceVerdict::Unknown);
        assert!(found.note.unwrap().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn a_missing_cli_is_not_installed_never_signed_out() {
        // The distinction the operator sees: "you don't have it" and "you
        // have it and are logged out" are different problems with different
        // fixes, and a probe that conflates them tells them to sign in to
        // something they never installed.
        let spec = PresenceSpec {
            program: "dpl-definitely-not-a-real-binary",
            args: &["status"],
            signed_in: &["logged in"],
            signed_out: &["not logged in"],
            login: &[],
        };
        let found = probe(ProviderId("ghost"), &spec);
        assert_eq!(found.verdict, PresenceVerdict::NotInstalled);
    }

    #[test]
    fn a_real_tool_is_classified_from_its_own_words() {
        // `cargo` is on PATH wherever this test runs — on every OS, unlike
        // `true`, which does not exist on Windows and turned this test into
        // NotInstalled on the CI VMs. Its version line matches no phrase
        // below — proving we never invent a verdict from unrecognized output.
        let spec = PresenceSpec {
            program: "cargo",
            args: &["--version"],
            signed_in: &["logged in"],
            signed_out: &["not logged in"],
            login: &[],
        };
        assert_eq!(
            probe(ProviderId("quiet"), &spec).verdict,
            PresenceVerdict::Unknown
        );
    }

    #[test]
    fn signed_out_wins_over_signed_in() {
        // Real grok/cursor sign-out text contains "log in".
        let spec = PresenceSpec {
            program: "x",
            args: &[],
            signed_in: &["logged in"],
            signed_out: &["not logged in"],
            login: &[],
        };
        assert_eq!(
            spec.classify("You are not logged in. Run `x login` to log in."),
            PresenceVerdict::SignedOut
        );
    }

    #[test]
    fn a_provider_with_no_cli_sign_in_is_refused_not_invented() {
        // Gemini's sign-in verb is unknown on this machine. Guessing one and
        // spawning it is how an operator gets a mystery process instead of a
        // login, so the host refuses BY NAME and the UI keeps offering the
        // install route.
        let spec = PresenceSpec {
            program: "true",
            args: &[],
            signed_in: &["x"],
            signed_out: &[],
            login: &[],
        };
        let refused = begin_sign_in(ProviderId("gemini"), spec)
            .expect_err("a provider with no login verb must be refused");
        assert!(refused.contains("no CLI sign-in"), "{refused}");
    }

    #[test]
    fn sign_in_is_refused_when_the_cli_is_absent() {
        let spec = PresenceSpec {
            program: "dpl-definitely-not-a-real-binary",
            args: &["status"],
            signed_in: &["logged in"],
            signed_out: &[],
            login: &["login"],
        };
        assert!(
            begin_sign_in(ProviderId("ghost"), spec).is_err(),
            "a sign-in that cannot even start is an error, not a pending state"
        );
    }

    #[test]
    fn only_codex_defers_its_sign_in_to_a_coordinator() {
        // Codex owns a device-code ceremony of its own. Every other probe-
        // backed provider must carry a login verb, or its Sign in button is
        // the dead end this pass exists to remove.
        for (provider, spec) in specs() {
            if provider == dpl_chat_codex::PROVIDER || provider == dpl_chat_gemini::PROVIDER {
                continue;
            }
            assert!(
                !spec.login.is_empty(),
                "{provider} needs a sign-in verb or its button does nothing"
            );
        }
    }

    #[test]
    fn every_registered_provider_declares_a_probe() {
        let specs = specs();
        assert_eq!(specs.len(), 5, "codex, claude, grok, cursor, gemini");
        for (provider, spec) in specs {
            assert!(
                !spec.program.is_empty() && !spec.signed_in.is_empty(),
                "{provider} must name a program and what signed-in looks like"
            );
            assert!(
                spec.signed_in.iter().all(|n| n == &n.to_ascii_lowercase()),
                "{provider}: needles are matched lowercase, so they must BE lowercase"
            );
            assert!(
                spec.signed_out.iter().all(|n| n == &n.to_ascii_lowercase()),
                "{provider}: needles are matched lowercase, so they must BE lowercase"
            );
        }
    }
}
