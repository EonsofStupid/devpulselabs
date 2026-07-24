use egui_pilot_ai_protocol::{Flow, RunOutcome, SnapshotRequest};
use egui_pilot_ai_runner::{
    load_flow, save_flow, EguiPilotConfig, ProcessSession, RunnerError, CONFIG_FILE,
};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

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
                "EguiPilotAI\n\n  doctor\n  mcp\n  test <profile> <scenario.toml> [fixture]\n  inspect <profile> [fixture]\n  fixtures\n  codegen <input.toml> <output.toml>  [EXPERIMENTAL normalizer; NOT a recorder]\n  open <run-directory>\n  agent install codex|cursor|claude\n\nUnsupported in this build: set-value, scroll-into-view, screenshot expectations, parallel workers, retries, recorder codegen, replay, visual approval, and Test Lab live control."
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
    let mut session = ProcessSession::launch(workspace, &config, profile, fixture)?;
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
    let mut session = ProcessSession::launch(workspace, &config, profile, fixture)?;
    let snapshot = session.snapshot(SnapshotRequest::default())?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    session.stop()?;
    Ok(())
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
            "Launch one declared profile and optional fixture",
            json!({
                "type": "object", "required": ["profile"], "additionalProperties": false,
                "properties": {"profile": {"type": "string"}, "fixture": {"type": "string"}}
            }),
        ),
        tool("pilot_status", "Read compact target status", empty_schema()),
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
            let session =
                ProcessSession::launch(&state.workspace, &state.config, profile, fixture)?;
            let result = json!({
                "session": session.session,
                "run_directory": session.run_dir,
                "profile": profile,
                "fixture": fixture,
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
    fn mcp_exposes_only_the_six_bounded_tools() {
        let tools = mcp_tools();
        assert_eq!(tools.len(), 6);
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
