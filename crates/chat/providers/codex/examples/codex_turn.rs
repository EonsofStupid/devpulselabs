//! THE LIVE PROOF: one real ChatGPT turn through the seam.
//!
//! Not a mock and not a script — this drives the SAME
//! [`CodexAppServerRuntime`] the app drives, through the SAME
//! [`ProviderRuntime`] trait the host calls, so a pass here is evidence about
//! the shipped code rather than about a parallel reimplementation.
//!
//! Deliberately an example rather than a test: it needs the operator's real
//! `codex` login and the network, and a test that does either is one CI will
//! eventually run.
//!
//! ```text
//! cargo run -p dpl-chat-codex --example codex_turn
//! cargo run -p dpl-chat-codex --example codex_turn -- "your prompt"
//! ```

use dpl_chat_codex::runtime::CodexAppServerRuntime;
use dpl_chat_codex::CodexRuntimeProfile;
use dpl_chat_runtime::seam::{
    ProviderRuntime, RuntimeCommand, RuntimeEvent, RuntimeSendError, RuntimeUpdate,
};
use std::time::{Duration, Instant};

/// Generous: a cold `app-server` plus a real model turn.
const TURN_TIMEOUT: Duration = Duration::from_secs(180);

fn main() {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Reply with exactly: OK".to_owned());
    let working_directory = std::env::current_dir().expect("a working directory");

    let profile = match CodexRuntimeProfile::discover(&working_directory) {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("codex could not be discovered: {error}");
            std::process::exit(1);
        }
    };
    println!(
        "executable: {}\nversion:    {}",
        profile.executable().display(),
        profile.version()
    );

    let runtime = match CodexAppServerRuntime::start(&profile) {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("codex app-server could not start: {error}");
            std::process::exit(1);
        }
    };

    let mut answer = String::new();
    let mut thread_id = None;
    let mut prompted = false;
    let started = Instant::now();

    while started.elapsed() < TURN_TIMEOUT {
        match runtime.try_event() {
            Ok(Some(event)) => match event {
                RuntimeEvent::Connected { capabilities } => {
                    println!("connected; advertised {capabilities:?}");
                    send(&runtime, RuntimeCommand::NewSession);
                }
                RuntimeEvent::SessionReady {
                    provider_session_id,
                    ..
                } => {
                    println!("thread:     {provider_session_id}");
                    thread_id = Some(provider_session_id.clone());
                    if !prompted {
                        prompted = true;
                        send(
                            &runtime,
                            RuntimeCommand::Prompt {
                                provider_session_id,
                                text: prompt.clone(),
                            },
                        );
                    }
                }
                RuntimeEvent::SessionUpdate { update, .. } => match update {
                    RuntimeUpdate::AssistantText(chunk) => answer.push_str(&chunk),
                    RuntimeUpdate::Usage { used, size } => {
                        println!("usage:      {used} of {size}");
                    }
                    other => println!("update:     {other:?}"),
                },
                RuntimeEvent::TurnFinished { stop_reason, .. } => {
                    println!("\nstop:       {stop_reason}");
                    println!("ASSISTANT ANSWER: {answer:?}");
                    println!(
                        "\nPROOF: {}",
                        if answer.trim().is_empty() {
                            "FAILED — the turn completed with no assistant text"
                        } else {
                            "the app's own transport carried a real ChatGPT turn"
                        }
                    );
                    return;
                }
                RuntimeEvent::Failure(failure) => {
                    eprintln!("failure:    {:?} — {}", failure.kind, failure.reason);
                }
                RuntimeEvent::Stopped => {
                    eprintln!("codex app-server stopped before the turn finished");
                    std::process::exit(1);
                }
                other => println!("event:      {other:?}"),
            },
            // Nothing waiting. The seam never blocks, so the caller paces
            // itself exactly like the app's frame loop does.
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                eprintln!("runtime closed: {error}");
                std::process::exit(1);
            }
        }
    }

    eprintln!("timed out after {TURN_TIMEOUT:?}; thread was {thread_id:?}");
    std::process::exit(1);
}

fn send(runtime: &CodexAppServerRuntime, command: RuntimeCommand) {
    if let Err(error) = runtime.try_send(command) {
        // `Full` is transient by contract; anything else is terminal here.
        eprintln!("command refused: {error}");
        if error == RuntimeSendError::Closed {
            std::process::exit(1);
        }
    }
}
