// spawn-audit: exempt(CLI harness — console output is the product)
use egui_pilot_ai_protocol as protocol;
use egui_pilot_ai_runner::{
    load_flow, save_flow, EguiPilotConfig, ProcessSession, RunnerError, WindowSize, CONFIG_FILE,
};
use protocol::{
    CoachCommand, CoachCondition, CoachId, CoachStyle, Flow, Locator, NodeId, RunOutcome,
    SnapshotRequest, Touchpoint, TriggerKind,
};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn main() {
    if let Err(error) = run() {
        eprintln!("eguipilotai: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "help".into());
    let workspace = std::env::current_dir()?;
    match command.as_str() {
        "doctor" => doctor(&workspace),
        "mcp" => mcp(&workspace),
        "test" => {
            let profile = required(&mut arguments, "profile")?;
            let scenario = PathBuf::from(required(&mut arguments, "scenario")?);
            let fixture = arguments.next();
            test(&workspace, &profile, fixture.as_deref(), &scenario)
        }
        "inspect" => {
            let profile = required(&mut arguments, "profile")?;
            let fixture = arguments.next();
            inspect(&workspace, &profile, fixture.as_deref())
        }
        "coach" => {
            let profile = required(&mut arguments, "profile")?;
            let node = required(&mut arguments, "node id")?;
            coach(&workspace, &profile, &node, CoachOptions::parse(arguments)?)
        }
        "fixtures" => fixtures(&workspace),
        "codegen" => {
            let input = PathBuf::from(required(&mut arguments, "input")?);
            let output = PathBuf::from(required(&mut arguments, "output")?);
            codegen(&input, &output)
        }
        "open" => {
            let run = PathBuf::from(required(&mut arguments, "run directory")?);
            open_run(&workspace, &run)
        }
        "agent" => match arguments.next().as_deref() {
            Some("install") => {
                let agent = required(&mut arguments, "codex|cursor|claude")?;
                install_agent(&agent)
            }
            _ => Err(CliError::Usage(
                "expected `eguipilotai agent install codex|cursor|claude`".into(),
            )),
        },
        "help" | "--help" | "-h" => {
            println!(
                "EguiPilotAI\n\n  doctor\n  mcp\n  test <profile> <scenario.toml> [fixture]\n  inspect <profile> [fixture]\n  coach <profile> <node-id> [--fixture <name>] [--message <text>] [--on <trigger>] [--wait-ms <n>] [--ttl-ms <n>]\n  fixtures\n  codegen <input.toml> <output.toml>  [EXPERIMENTAL normalizer; NOT a recorder]\n  open <run-directory>\n  agent install codex|cursor|claude\n\nCoaching paints a spotlight and reports what fires it.\n  --on press (default) | hover | appeared | value-changed | signal:<id>\nA touchpoint waiting on `appeared` or a signal mounts armed, so it can be set before its target exists.\n`inspect` and MCP `pilot_status` list every signal id this build can publish; nothing outside that list fires.\n\nUnsupported in this build: set-value, scroll-into-view, screenshot expectations, parallel workers, retries, recorder codegen, replay, visual approval, Test Lab live control, the semantic revision ticker, and the desktop coach overlay."
            );
            Ok(())
        }
        _ => Err(CliError::Usage(format!("unknown command `{command}`"))),
    }
}

fn required(arguments: &mut impl Iterator<Item = String>, name: &str) -> Result<String, CliError> {
    arguments
        .next()
        .ok_or_else(|| CliError::Usage(format!("missing {name}")))
}

fn doctor(workspace: &Path) -> Result<(), CliError> {
    let config = EguiPilotConfig::load(workspace)?;
    let profiles: Vec<Value> = config
        .profiles
        .iter()
        .filter(|(_, profile)| profile.supports_current_platform())
        .map(|(name, profile)| {
            let executable = if profile.executable.is_absolute() {
                profile.executable.clone()
            } else {
                workspace.join(&profile.executable)
            };
            json!({
                "name": name,
                "executable": executable,
                "exists": executable.is_file(),
                "adapter_policy": profile.adapter_policy,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": profiles.iter().all(|profile| profile["exists"] == true),
            "config": workspace.join(CONFIG_FILE),
            "profiles": profiles,
            "fixtures": config.fixtures.keys().collect::<Vec<_>>(),
            "capabilities": egui_pilot_ai_protocol::capability_catalog(),
        }))?
    );
    Ok(())
}

fn test(
    workspace: &Path,
    profile: &str,
    fixture: Option<&str>,
    scenario: &Path,
) -> Result<(), CliError> {
    let config = EguiPilotConfig::load(workspace)?;
    let flow = load_flow(scenario)?;
    let declared_fixture = flow.fixture.as_ref().map(|fixture| fixture.0.as_str());
    let fixture = fixture.or(declared_fixture);
    let mut session = ProcessSession::launch(workspace, &config, profile, fixture, None)?;
    let result = session.run(flow)?;
    let run_dir = session.run_dir.clone();
    session.stop()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "passed": result.outcome == RunOutcome::Passed,
            "outcome": result.outcome,
            "steps": result.steps,
            "error": result.error,
            "run_directory": run_dir,
        }))?
    );
    require_passing_outcome(&result, &run_dir)
}

fn require_passing_outcome(
    result: &egui_pilot_ai_protocol::RunResult,
    run_dir: &Path,
) -> Result<(), CliError> {
    if result.outcome == RunOutcome::Passed {
        return Ok(());
    }
    Err(CliError::ScenarioFailed(format!(
        "scenario `{}` finished with {:?}; artifacts: {}",
        result.scenario,
        result.outcome,
        run_dir.display()
    )))
}

fn inspect(workspace: &Path, profile: &str, fixture: Option<&str>) -> Result<(), CliError> {
    let config = EguiPilotConfig::load(workspace)?;
    let mut session = ProcessSession::launch(workspace, &config, profile, fixture, None)?;
    let snapshot = session.snapshot(SnapshotRequest::default())?;
    // The signal catalog rides with the semantic inventory because the two
    // answer the same question: what can a touchpoint be aimed at, and what
    // can it wait for. Neither is discoverable any other way.
    let coach_signals = session.status()?.coach_signals;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "snapshot": snapshot,
            "coach_signals": coach_signals,
        }))?
    );
    session.stop()?;
    Ok(())
}

struct CoachOptions {
    fixture: Option<String>,
    message: Option<String>,
    wait: Duration,
    ttl_ms: Option<u64>,
    on: TriggerKind,
    until: Option<CoachCondition>,
}

impl CoachOptions {
    fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut options = Self {
            fixture: None,
            message: None,
            // A coached human is slower than any automation timeout.
            wait: Duration::from_secs(180),
            ttl_ms: None,
            on: TriggerKind::Press,
            until: None,
        };
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--fixture" => options.fixture = Some(required(&mut arguments, "fixture name")?),
                "--message" => options.message = Some(required(&mut arguments, "message text")?),
                "--wait-ms" => {
                    options.wait = Duration::from_millis(parse_ms(&mut arguments, "--wait-ms")?);
                }
                "--ttl-ms" => options.ttl_ms = Some(parse_ms(&mut arguments, "--ttl-ms")?),
                "--on" => options.on = parse_trigger(&required(&mut arguments, "trigger")?)?,
                "--until" => {
                    let json = required(&mut arguments, "condition json")?;
                    options.until = Some(serde_json::from_str(&json).map_err(|error| {
                        CliError::Usage(format!("`--until` is not a condition: {error}"))
                    })?);
                }
                other => return Err(CliError::Usage(format!("unknown coach flag `{other}`"))),
            }
        }
        Ok(options)
    }
}

/// `press | hover | appeared | value-changed | signal:<id>`
fn parse_trigger(value: &str) -> Result<TriggerKind, CliError> {
    match value {
        "press" => Ok(TriggerKind::Press),
        "hover" => Ok(TriggerKind::Hover),
        "appeared" => Ok(TriggerKind::NodeAppeared),
        "value-changed" => Ok(TriggerKind::ValueChanged),
        other => other
            .strip_prefix("signal:")
            .filter(|signal| !signal.is_empty())
            .map(|signal| TriggerKind::FabricEvent {
                signal: protocol::SignalId::new(signal),
            })
            .ok_or_else(|| {
                CliError::Usage(format!(
                    "unknown trigger `{other}`; expected press, hover, appeared, value-changed, or signal:<id>"
                ))
            }),
    }
}

fn parse_ms(arguments: &mut impl Iterator<Item = String>, flag: &str) -> Result<u64, CliError> {
    required(arguments, flag)?
        .parse()
        .map_err(|_| CliError::Usage(format!("{flag} expects whole milliseconds")))
}

/// Paint a spotlight on one instrumented control and wait for it to be
/// pressed. This is the operator-coaching loop: the highlight never
/// intercepts the press, so the real control still does its real work.
fn coach(
    workspace: &Path,
    profile: &str,
    node: &str,
    options: CoachOptions,
) -> Result<(), CliError> {
    let config = EguiPilotConfig::load(workspace)?;
    let coach_id = CoachId::new("coach-cli");
    let mut session = ProcessSession::launch(
        workspace,
        &config,
        profile,
        options.fixture.as_deref(),
        None,
    )?;
    let mounted = session.coach(CoachCommand::Mount {
        touchpoint: Box::new(Touchpoint {
            coach_id: coach_id.clone(),
            target: Locator {
                id: Some(NodeId::new(node)),
                ..Locator::default()
            },
            visual: CoachStyle::Spotlight,
            on: options.on,
            until: options.until,
            message: options.message,
            ttl_ms: options.ttl_ms,
            once: true,
        }),
    });
    let outcome = match mounted {
        Ok(events) if events.iter().all(|event| !event.is_terminal()) => {
            println!("{}", serde_json::to_string(&json!({"mounted": events}))?);
            session.await_coach(&coach_id, options.wait).map(Some)
        }
        Ok(events) => {
            println!("{}", serde_json::to_string(&json!({"refused": events}))?);
            Ok(None)
        }
        Err(error) => Err(error),
    };
    let run_dir = session.run_dir.clone();
    session.stop()?;
    let event = outcome?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "coach_id": coach_id,
            "event": event,
            "run_directory": run_dir,
        }))?
    );
    match event {
        Some(protocol::CoachEvent::Triggered { .. }) => Ok(()),
        Some(event) => Err(CliError::ScenarioFailed(format!(
            "touchpoint `{coach_id}` ended without a press: {event:?}"
        ))),
        None => Err(CliError::ScenarioFailed(format!(
            "touchpoint `{coach_id}` was refused"
        ))),
    }
}

fn fixtures(workspace: &Path) -> Result<(), CliError> {
    let config = EguiPilotConfig::load(workspace)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&config.fixtures.keys().collect::<Vec<_>>())?
    );
    Ok(())
}

fn codegen(input: &Path, output: &Path) -> Result<(), CliError> {
    let flow = load_flow(input)?;
    save_flow(output, &flow)?;
    println!(
        "EXPERIMENTAL normalizer only (not a recorder): wrote durable scenario {}",
        output.display()
    );
    Ok(())
}

fn open_run(workspace: &Path, run: &Path) -> Result<(), CliError> {
    let path = if run.is_absolute() {
        run.to_path_buf()
    } else {
        workspace.join(run)
    };
    if !path.is_dir() {
        return Err(CliError::Usage(format!(
            "run directory does not exist: {}",
            path.display()
        )));
    }
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer.exe")
        .arg(&path)
        .spawn()?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&path).spawn()?;
    #[cfg(all(unix, not(target_os = "macos")))]
    std::process::Command::new("xdg-open").arg(&path).spawn()?;
    Ok(())
}

fn install_agent(agent: &str) -> Result<(), CliError> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| CliError::Usage("home directory is unavailable".into()))?;
    let home = PathBuf::from(home);
    let (directory, file, body) = match agent {
        "codex" => (
            home.join(".codex/skills/eguipilotai"),
            "SKILL.md",
            include_str!("agent/eguipilotai-codex.md"),
        ),
        "claude" => (
            home.join(".claude/skills/eguipilotai"),
            "SKILL.md",
            include_str!("agent/eguipilotai-claude.md"),
        ),
        "cursor" => (
            home.join(".cursor/rules"),
            "eguipilotai.mdc",
            include_str!("agent/eguipilotai-cursor.mdc"),
        ),
        _ => {
            return Err(CliError::Usage(
                "agent must be codex, cursor, or claude".into(),
            ))
        }
    };
    fs::create_dir_all(&directory)?;
    let path = directory.join(file);
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            CliError::Usage(format!("refusing to overwrite {}: {error}", path.display()))
        })?;
    output.write_all(body.as_bytes())?;
    println!("installed {}", path.display());
    Ok(())
}

struct McpState {
    workspace: PathBuf,
    config: EguiPilotConfig,
    session: Option<ProcessSession>,
}

fn mcp(workspace: &Path) -> Result<(), CliError> {
    let config = EguiPilotConfig::load(workspace)?;
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut state = McpState {
        workspace: workspace.to_path_buf(),
        config,
        session: None,
    };
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = serde_json::from_str(&line)?;
        if request.get("id").is_none() {
            continue;
        }
        let response = handle_mcp(&mut state, &request);
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }
    if let Some(mut session) = state.session {
        let _ = session.stop();
    }
    Ok(())
}

fn handle_mcp(state: &mut McpState, request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "eguipilotai", "version": env!("CARGO_PKG_VERSION")}
        })),
        "tools/list" => Ok(json!({"tools": mcp_tools()})),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or_default();
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            call_tool(state, name, arguments)
                .map(|value| json!({"content": [{"type": "text", "text": value.to_string()}]}))
        }
        _ => Err(CliError::Usage(format!(
            "unsupported MCP method `{method}`"
        ))),
    };
    match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32000, "message": error.to_string()}
        }),
    }
}

fn mcp_tools() -> Vec<Value> {
    vec![
        tool(
            "pilot_start",
            "Launch one declared profile and optional fixture. `window` sets the target's window size as `<width>x<height>` (for example `1280x832`); layout is size-dependent, so a lesson or scenario that needs room — the rail accordion cannot unfold in a small window — should ask for a size rather than inherit whatever the desktop gives it.",
            json!({
                "type": "object", "required": ["profile"], "additionalProperties": false,
                "properties": {
                    "profile": {"type": "string"},
                    "fixture": {"type": "string"},
                    "window": {"type": "string", "pattern": "^[0-9]+[xX][0-9]+$"}
                }
            }),
        ),
        tool(
            "pilot_status",
            "Read compact target status, including `coach_signals`: every signal a `signal:<id>` touchpoint can wait for in this build. The list is declared by the application; a signal that is not in it will never fire.",
            empty_schema(),
        ),
        tool(
            "pilot_snapshot",
            "Read a pruned, changed, or explicitly full semantic projection",
            json!({
                "type": "object", "additionalProperties": false,
                "properties": {"request": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "mode": {"type": "string", "enum": ["agent", "changes", "full"]},
                        "filter": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "viewport": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "properties": {
                                        "id": {"type": "string", "maxLength": 240},
                                        "key": {"type": "string", "maxLength": 240}
                                    }
                                },
                                "subtree": {"type": "string", "maxLength": 512},
                                "role": {"type": "string"},
                                "include_hidden": {"type": "boolean"},
                                "include_decorative": {"type": "boolean"}
                            }
                        },
                        "since_revision": {"type": "integer", "minimum": 0},
                        "continuation": {"type": "string", "maxLength": 120},
                        "max_nodes": {"type": "integer", "minimum": 1, "maximum": 1000},
                        "max_bytes": {"type": "integer", "minimum": 512, "maximum": 65536}
                    }
                }}
            }),
        ),
        tool(
            "pilot_run",
            "Run a typed flow. Unsupported actions and assertions are rejected before execution.",
            json!({
                "type": "object", "required": ["flow"], "additionalProperties": false,
                "properties": {"flow": {
                    "type": "object",
                    "description": "Supported: click, double-click, hover, fill, clear, type, press, drag-to, scroll, focus-viewport, close-viewport, semantic and viewport expectations, wait, and renderer-supported capture. Unsupported: set-value, scroll-into-view, screenshot expectations, deferred viewports, and immediate child capture.",
                    "x-eguipilotai-unsupported": ["action.set-value", "action.scroll-into-view", "expect.screenshot", "viewport.deferred", "viewport.capture.immediate"]
                }}
            }),
        ),
        tool(
            "pilot_capture",
            "Capture one renderer-supported viewport as a deterministic content-addressed image. Eframe 0.35 native wgpu immediate child capture, baseline comparison, and approval are unsupported.",
            json!({
                "type": "object", "required": ["name"], "additionalProperties": false,
                "properties": {
                    "name": {"type": "string", "maxLength": 120},
                    "viewport": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "id": {"type": "string"},
                            "key": {"type": "string"}
                        }
                    }
                },
                "x-eguipilotai-unsupported": ["viewport.capture.immediate", "viewport.capture.deferred", "expect.screenshot", "visual.approval"]
            }),
        ),
        tool(
            "pilot_coach",
            "Mount a spotlight touchpoint on one instrumented node and optionally wait for it to fire. The highlight never intercepts input, so the operator's press still reaches the real control. A touchpoint waiting on `appeared` or `signal:<id>` mounts armed and may outlive its target's absence. Only the spotlight visual exists; other visuals are refused.",
            json!({
                "type": "object", "required": ["action"], "additionalProperties": false,
                "properties": {
                    "action": {"type": "string", "enum": ["mount", "unmount", "unmount-all"]},
                    "coach_id": {"type": "string", "maxLength": 120},
                    "node": {"type": "string", "maxLength": 512},
                    "message": {"type": "string", "maxLength": 240},
                    "on": {
                        "type": "string",
                        "description": "press (default), hover, appeared, value-changed, or signal:<id> for a signal the application publishes",
                        "default": "press"
                    },
                    "until": {
                        "type": "object",
                        "description": "The postcondition that completes the step, read every frame and satisfied at once when already true — unlike `on`, which needs a transition. Use it whenever the goal may already be met.",
                        "required": ["type", "node"], "additionalProperties": false,
                        "properties": {
                            "type": {"type": "string", "enum": ["node-value", "node-visible", "node-gone"]},
                            "node": {"type": "string", "maxLength": 512},
                            "value": {"type": "string", "maxLength": 240}
                        }
                    },
                    "ttl_ms": {"type": "integer", "minimum": 100, "maximum": 600000},
                    "wait_ms": {"type": "integer", "minimum": 0, "maximum": 600000}
                },
                "x-eguipilotai-unsupported": ["coach.visual.extended", "coach.semantic-ticker", "coach.desktop-overlay"]
            }),
        ),
        tool(
            "pilot_stop",
            "Stop the separate target process",
            empty_schema(),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name": name, "description": description, "inputSchema": input_schema})
}

fn empty_schema() -> Value {
    json!({"type": "object", "additionalProperties": false})
}

fn call_tool(state: &mut McpState, name: &str, arguments: Value) -> Result<Value, CliError> {
    match name {
        "pilot_start" => {
            if state.session.is_some() {
                return Err(CliError::Usage("a target session is already active".into()));
            }
            let profile = string_field(&arguments, "profile")?;
            let fixture = arguments.get("fixture").and_then(Value::as_str);
            let window = arguments
                .get("window")
                .and_then(Value::as_str)
                .map(str::parse::<WindowSize>)
                .transpose()?;
            let session =
                ProcessSession::launch(&state.workspace, &state.config, profile, fixture, window)?;
            let result = json!({
                "session": session.session,
                "run_directory": session.run_dir,
                "profile": profile,
                "fixture": fixture,
                "window": window,
            });
            state.session = Some(session);
            Ok(result)
        }
        "pilot_status" => Ok(serde_json::to_value(active(state)?.status()?)?),
        "pilot_snapshot" => {
            let request = match arguments.get("request") {
                Some(value) => serde_json::from_value(value.clone())?,
                None => SnapshotRequest::default(),
            };
            Ok(serde_json::to_value(active(state)?.snapshot(request)?)?)
        }
        "pilot_run" => {
            let flow: Flow = serde_json::from_value(
                arguments
                    .get("flow")
                    .cloned()
                    .ok_or_else(|| CliError::Usage("missing flow".into()))?,
            )?;
            Ok(serde_json::to_value(active(state)?.run(flow)?)?)
        }
        "pilot_capture" => {
            let name = string_field(&arguments, "name")?.to_string();
            let viewport = arguments
                .get("viewport")
                .cloned()
                .map(serde_json::from_value)
                .transpose()?;
            Ok(serde_json::to_value(
                active(state)?.capture_viewport(name, viewport)?,
            )?)
        }
        "pilot_coach" => {
            let action = string_field(&arguments, "action")?.to_string();
            let coach_id = CoachId::new(
                arguments
                    .get("coach_id")
                    .and_then(Value::as_str)
                    .unwrap_or("coach-agent"),
            );
            let command = match action.as_str() {
                "mount" => CoachCommand::Mount {
                    touchpoint: Box::new(Touchpoint {
                        coach_id: coach_id.clone(),
                        target: Locator {
                            id: Some(NodeId::new(string_field(&arguments, "node")?)),
                            ..Locator::default()
                        },
                        visual: CoachStyle::Spotlight,
                        on: arguments
                            .get("on")
                            .and_then(Value::as_str)
                            .map_or(Ok(TriggerKind::Press), parse_trigger)?,
                        until: arguments
                            .get("until")
                            .cloned()
                            .map(serde_json::from_value)
                            .transpose()
                            .map_err(|error| {
                                CliError::Usage(format!("`until` is not a condition: {error}"))
                            })?,
                        message: arguments
                            .get("message")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        ttl_ms: arguments.get("ttl_ms").and_then(Value::as_u64),
                        once: true,
                    }),
                },
                "unmount" => CoachCommand::Unmount {
                    coach_id: coach_id.clone(),
                },
                "unmount-all" => CoachCommand::UnmountAll,
                other => {
                    return Err(CliError::Usage(format!(
                        "coach action must be mount, unmount, or unmount-all; got `{other}`"
                    )))
                }
            };
            let wait = arguments.get("wait_ms").and_then(Value::as_u64);
            let session = active(state)?;
            let events = session.coach(command)?;
            let activated = match wait.filter(|_| action == "mount") {
                Some(wait) if events.iter().all(|event| !event.is_terminal()) => {
                    Some(session.await_coach(&coach_id, Duration::from_millis(wait))?)
                }
                _ => None,
            };
            Ok(json!({"events": events, "activated": activated}))
        }
        "pilot_stop" => {
            let mut session = state
                .session
                .take()
                .ok_or_else(|| CliError::Usage("no target session is active".into()))?;
            let run_dir = session.run_dir.clone();
            session.stop()?;
            Ok(json!({"stopped": true, "run_directory": run_dir}))
        }
        _ => Err(CliError::Usage(format!("unknown tool `{name}`"))),
    }
}

fn active(state: &mut McpState) -> Result<&mut ProcessSession, CliError> {
    state
        .session
        .as_mut()
        .ok_or_else(|| CliError::Usage("pilot_start must be called first".into()))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, CliError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| CliError::Usage(format!("missing string field `{field}`")))
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    ScenarioFailed(String),
    Io(io::Error),
    Json(serde_json::Error),
    Runner(RunnerError),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => f.write_str(message),
            Self::ScenarioFailed(message) => f.write_str(message),
            Self::Io(error) => error.fmt(f),
            Self::Json(error) => error.fmt(f),
            Self::Runner(error) => error.fmt(f),
        }
    }
}

impl From<io::Error> for CliError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<RunnerError> for CliError {
    fn from(value: RunnerError) -> Self {
        Self::Runner(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_exposes_only_the_seven_bounded_tools() {
        let tools = mcp_tools();
        let names: Vec<_> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            names,
            [
                "pilot_start",
                "pilot_status",
                "pilot_snapshot",
                "pilot_run",
                "pilot_capture",
                "pilot_coach",
                "pilot_stop",
            ]
        );
        assert!(tools
            .iter()
            .all(|tool| tool["inputSchema"]["additionalProperties"] == false));
        let snapshot = tools
            .iter()
            .find(|tool| tool["name"] == "pilot_snapshot")
            .expect("pilot_snapshot");
        assert_eq!(
            snapshot["inputSchema"]["properties"]["request"]["additionalProperties"],
            false
        );
        assert_eq!(
            snapshot["inputSchema"]["properties"]["request"]["properties"]["filter"]
                ["additionalProperties"],
            false
        );
    }

    #[test]
    fn snapshot_defaults_to_agent_mode() {
        assert_eq!(
            SnapshotRequest::default().mode,
            egui_pilot_ai_protocol::SnapshotMode::Agent
        );
    }

    #[test]
    fn failed_scenarios_produce_a_cli_error() {
        let result = egui_pilot_ai_protocol::RunResult {
            run_id: egui_pilot_ai_protocol::RunId::new("run-1"),
            scenario: egui_pilot_ai_protocol::ScenarioId::new("failure"),
            outcome: RunOutcome::Failed,
            steps: Vec::new(),
            error: None,
        };
        let error = require_passing_outcome(&result, Path::new("target/run"))
            .expect_err("failed scenarios must make the process exit non-zero");
        assert!(error.to_string().contains("finished with Failed"));
    }
}
