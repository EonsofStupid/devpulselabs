//! Named terminal entry, separate from the native Chat window.
use dpl_chat_codex::standalone;
use dpl_chat_core::PermissionMode;
use dpl_chat_projects::{ProfileGeneration, ProjectId};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

const HELP: &str = "Open Codex for a registered DevPULSE project\n\nUsage: devpulse-codex-project --registry <absolute-path> --project <id> --generation <id> [--mode ask|workspace|read-only]\n\nRequires an existing terminal and a qualified project profile. No setup, arbitrary commands or credential migration are performed.\nThis build keeps standalone support disabled pending live qualification.";

#[derive(Debug)]
struct Options {
    registry: PathBuf,
    project: ProjectId,
    generation: ProfileGeneration,
    mode: PermissionMode,
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Option<Options>, String> {
    let args: Vec<_> = args.into_iter().take(9).collect();
    if args.len() > 8
        || args
            .iter()
            .any(|arg| arg.as_encoded_bytes().len() > 32 * 1024)
    {
        return Err("The named project entry accepts at most four bounded options".into());
    }
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return Ok(None);
    }
    let mut registry = None;
    let mut project = None;
    let mut generation = None;
    let mut mode = None;
    let mut values = args.into_iter();
    while let Some(key) = values.next() {
        let value = values.next().ok_or("Every option requires a value")?;
        match key.to_str() {
            Some("--registry") if registry.is_none() => {
                let path = PathBuf::from(value);
                if !path.is_absolute()
                    || path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
                {
                    return Err("The registry path must be absolute and contain no '..'".into());
                }
                registry = Some(path);
            }
            Some("--project") if project.is_none() => {
                project = Some(ProjectId::parse(text(value)?)?);
            }
            Some("--generation") if generation.is_none() => {
                generation = Some(ProfileGeneration::parse(text(value)?)?);
            }
            Some("--mode") if mode.is_none() => {
                mode = Some(match text(value)?.as_str() {
                    "ask" => PermissionMode::AskFirst,
                    "workspace" => PermissionMode::Workspace,
                    "read-only" => PermissionMode::ReadOnly,
                    _ => return Err("Choose mode ask, workspace or read-only".into()),
                });
            }
            _ => return Err("Unknown or duplicate option; use --help for the named entry".into()),
        }
    }
    Ok(Some(Options {
        registry: registry.ok_or("An existing --registry is required")?,
        project: project.ok_or("A registered --project identity is required")?,
        generation: generation.ok_or("An existing --generation is required")?,
        mode: mode.unwrap_or(PermissionMode::AskFirst),
    }))
}

fn text(value: OsString) -> Result<String, String> {
    value
        .into_string()
        .map_err(|_| "This option requires UTF-8 text".into())
}

fn run(options: Options) -> Result<ExitCode, String> {
    let mut process = standalone::open_project(
        &options.registry,
        &options.project,
        &options.generation,
        options.mode,
    )?;
    loop {
        if let Some(status) = process.try_wait().map_err(|error| error.to_string())? {
            return Ok(if status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn main() -> ExitCode {
    let result = parse(std::env::args_os().skip(1)).and_then(|options| match options {
        Some(options) => run(options),
        None => {
            println!("{HELP}");
            Ok(ExitCode::SUCCESS)
        }
    });
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_args() -> Vec<OsString> {
        vec![
            "--registry".into(),
            std::env::temp_dir().into_os_string(),
            "--project".into(),
            "project-one".into(),
            "--generation".into(),
            "generation-one".into(),
        ]
    }

    #[test]
    fn accepts_explicit_binding_and_only_fixed_permission_modes() {
        let options = parse(valid_args()).unwrap().unwrap();
        assert_eq!(options.project.as_str(), "project-one");
        assert_eq!(options.generation.as_str(), "generation-one");
        assert_eq!(options.mode, PermissionMode::AskFirst);
        for (mode, expected) in [
            ("workspace", PermissionMode::Workspace),
            ("read-only", PermissionMode::ReadOnly),
        ] {
            let mut args = valid_args();
            args.extend(["--mode".into(), mode.into()]);
            assert_eq!(parse(args).unwrap().unwrap().mode, expected);
        }
        let mut args = valid_args();
        args.extend(["--mode".into(), "danger-full-access".into()]);
        assert!(parse(args).is_err());
    }

    #[test]
    fn refuses_ambiguous_paths_generations_duplicate_options_and_commands() {
        assert!(parse(Vec::new()).is_err());
        for (index, value) in [(1, "relative"), (3, "../project"), (5, "../generation")] {
            let mut args = valid_args();
            args[index] = value.into();
            assert!(parse(args).is_err());
        }
        for (key, value) in [
            ("--project", "second"),
            ("--exe", "codex"),
            ("--command", "anything"),
            ("--qualified", "true"),
        ] {
            let mut args = valid_args();
            args.extend([key.into(), value.into()]);
            assert!(parse(args).is_err());
        }
        assert!(parse(vec!["--help".into()]).unwrap().is_none());
        assert!(parse(vec!["--help".into(), "--qualified".into()]).is_err());
        assert!(parse(vec!["extra".into(); 9]).is_err());
        let mut args = valid_args();
        args[1] = "x".repeat(32 * 1024 + 1).into();
        assert!(parse(args).is_err());
    }
}
