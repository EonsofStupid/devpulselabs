use super::{allowed_environment_keys, hide_child_console};
use agent_client_protocol::schema::v1::{
    CreateTerminalRequest, CreateTerminalResponse, KillTerminalRequest, KillTerminalResponse,
    ReleaseTerminalRequest, ReleaseTerminalResponse, TerminalExitStatus, TerminalOutputRequest,
    TerminalOutputResponse, WaitForTerminalExitRequest, WaitForTerminalExitResponse,
};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

const DEFAULT_OUTPUT_LIMIT: usize = 1024 * 1024;
const MAX_OUTPUT_LIMIT: usize = 4 * 1024 * 1024;
const MAX_COMMAND_BYTES: usize = 16 * 1024;
const MAX_ARGUMENTS: usize = 1_024;
const MAX_ENVIRONMENT_ENTRIES: usize = 256;

#[derive(Clone)]
pub(crate) struct AcpTerminalStore {
    root: PathBuf,
    next_id: Arc<AtomicU64>,
    records: Arc<Mutex<BTreeMap<String, TerminalRecord>>>,
}

struct TerminalRecord {
    session_id: String,
    child: Child,
    output_rx: Receiver<Vec<u8>>,
    output: Vec<u8>,
    output_limit: usize,
    truncated: bool,
    exit_status: Option<ExitStatus>,
}

impl AcpTerminalStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            next_id: Arc::new(AtomicU64::new(1)),
            records: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub(crate) fn create(
        &self,
        request: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse, String> {
        if request.command.trim().is_empty() || request.command.len() > MAX_COMMAND_BYTES {
            return Err("the terminal command is empty or exceeds the bounded limit".into());
        }
        if request.args.len() > MAX_ARGUMENTS {
            return Err("the terminal request contains too many arguments".into());
        }
        if request.env.len() > MAX_ENVIRONMENT_ENTRIES {
            return Err("the terminal request contains too many environment entries".into());
        }
        let cwd = request.cwd.as_deref().unwrap_or(&self.root);
        let cwd = canonical_workspace_directory(&self.root, cwd)?;
        let output_limit = request
            .output_byte_limit
            .and_then(|limit| usize::try_from(limit).ok())
            .unwrap_or(DEFAULT_OUTPUT_LIMIT)
            .min(MAX_OUTPUT_LIMIT);

        let mut command = Command::new(&request.command);
        command
            .args(&request.args)
            .current_dir(cwd)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for key in allowed_environment_keys() {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        for variable in request.env {
            command.env(variable.name, variable.value);
        }
        hide_child_console(&mut command);
        let mut child = command.spawn().map_err(|error| public_io_reason(&error))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "terminal stdout was unavailable".to_owned())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "terminal stderr was unavailable".to_owned())?;
        let (output_tx, output_rx) = mpsc::channel();
        spawn_output_reader(stdout, output_tx.clone());
        spawn_output_reader(stderr, output_tx);

        let terminal_id = format!(
            "grok-terminal-{}",
            self.next_id.fetch_add(1, Ordering::Relaxed)
        );
        let session_id = request.session_id.0.to_string();
        self.records
            .lock()
            .map_err(|_| "terminal registry was unavailable".to_owned())?
            .insert(
                terminal_id.clone(),
                TerminalRecord {
                    session_id,
                    child,
                    output_rx,
                    output: Vec::new(),
                    output_limit,
                    truncated: false,
                    exit_status: None,
                },
            );
        Ok(CreateTerminalResponse::new(terminal_id))
    }

    pub(crate) fn output(
        &self,
        request: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse, String> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| "terminal registry was unavailable".to_owned())?;
        let record = records
            .get_mut(request.terminal_id.0.as_ref())
            .ok_or_else(|| "the requested terminal is unknown".to_owned())?;
        require_terminal_session(record, request.session_id.0.as_ref())?;
        refresh_record(record)?;
        let output = String::from_utf8_lossy(&record.output).into_owned();
        Ok(TerminalOutputResponse::new(output, record.truncated)
            .exit_status(record.exit_status.as_ref().map(exit_status)))
    }

    pub(crate) fn kill(
        &self,
        request: KillTerminalRequest,
    ) -> Result<KillTerminalResponse, String> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| "terminal registry was unavailable".to_owned())?;
        let record = records
            .get_mut(request.terminal_id.0.as_ref())
            .ok_or_else(|| "the requested terminal is unknown".to_owned())?;
        require_terminal_session(record, request.session_id.0.as_ref())?;
        if record
            .child
            .try_wait()
            .map_err(|error| public_io_reason(&error))?
            .is_none()
        {
            record
                .child
                .kill()
                .map_err(|error| public_io_reason(&error))?;
        }
        record.exit_status = Some(
            record
                .child
                .wait()
                .map_err(|error| public_io_reason(&error))?,
        );
        refresh_record(record)?;
        Ok(KillTerminalResponse::new())
    }

    pub(crate) fn release(
        &self,
        request: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse, String> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| "terminal registry was unavailable".to_owned())?;
        let record = records
            .get(request.terminal_id.0.as_ref())
            .ok_or_else(|| "the requested terminal is unknown".to_owned())?;
        require_terminal_session(record, request.session_id.0.as_ref())?;
        let mut record = records
            .remove(request.terminal_id.0.as_ref())
            .expect("the checked terminal record remains registered");
        terminate_and_reap(&mut record.child);
        Ok(ReleaseTerminalResponse::new())
    }

    pub(crate) async fn wait(
        &self,
        request: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse, String> {
        loop {
            let status = {
                let mut records = self
                    .records
                    .lock()
                    .map_err(|_| "terminal registry was unavailable".to_owned())?;
                let record = records
                    .get_mut(request.terminal_id.0.as_ref())
                    .ok_or_else(|| "the requested terminal is unknown".to_owned())?;
                require_terminal_session(record, request.session_id.0.as_ref())?;
                refresh_record(record)?;
                record.exit_status
            };
            if let Some(status) = status {
                return Ok(WaitForTerminalExitResponse::new(exit_status(&status)));
            }
            async_io::Timer::after(Duration::from_millis(10)).await;
        }
    }

    pub(crate) fn shutdown(&self) {
        let Ok(mut records) = self.records.lock() else {
            return;
        };
        for (_, mut record) in std::mem::take(&mut *records) {
            terminate_and_reap(&mut record.child);
        }
    }
}

fn require_terminal_session(record: &TerminalRecord, session_id: &str) -> Result<(), String> {
    if record.session_id == session_id {
        Ok(())
    } else {
        Err("the requested terminal belongs to another provider session".to_owned())
    }
}

fn canonical_workspace_directory(root: &Path, path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("the terminal requested a non-absolute working directory".into());
    }
    let path = std::fs::canonicalize(path).map_err(|error| public_io_reason(&error))?;
    if !path.starts_with(root) || !path.is_dir() {
        return Err("the terminal working directory is outside the approved workspace".into());
    }
    Ok(path)
}

fn spawn_output_reader(mut stream: impl Read + Send + 'static, tx: mpsc::Sender<Vec<u8>>) {
    let _ = thread::Builder::new()
        .name("devpulse-grok-terminal-output".into())
        .spawn(move || {
            let mut buffer = [0_u8; 8 * 1024];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => return,
                    Ok(read) => {
                        if tx.send(buffer[..read].to_vec()).is_err() {
                            return;
                        }
                    }
                }
            }
        });
}

fn refresh_record(record: &mut TerminalRecord) -> Result<(), String> {
    while let Ok(chunk) = record.output_rx.try_recv() {
        record.output.extend_from_slice(&chunk);
        if record.output.len() > record.output_limit {
            let remove = record.output.len() - record.output_limit;
            record.output.drain(..remove);
            record.truncated = true;
        }
    }
    if record.exit_status.is_none() {
        record.exit_status = record
            .child
            .try_wait()
            .map_err(|error| public_io_reason(&error))?;
    }
    Ok(())
}

fn exit_status(status: &ExitStatus) -> TerminalExitStatus {
    TerminalExitStatus::new().exit_code(status.code().and_then(|code| u32::try_from(code).ok()))
}

fn terminate_and_reap(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn public_io_reason(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::NotFound => "required command or file was not found".to_owned(),
        std::io::ErrorKind::PermissionDenied => "permission was denied".to_owned(),
        std::io::ErrorKind::TimedOut => "the terminal operation timed out".to_owned(),
        kind => format!("{kind:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::SessionId;

    #[test]
    fn terminal_rejects_working_directories_outside_the_workspace() {
        let root = std::env::current_dir().expect("workspace");
        let outside = std::env::temp_dir();
        if outside.starts_with(&root) {
            return;
        }
        let store = AcpTerminalStore::new(root);
        let request = CreateTerminalRequest::new(SessionId::new("session"), "rustc").cwd(outside);
        let error = store.create(request).expect_err("outside cwd must fail");
        assert!(error.contains("outside the approved workspace"));
    }

    #[test]
    fn terminal_executes_a_direct_bounded_command_and_reaps_it() {
        async_io::block_on(async {
            let root = std::fs::canonicalize(std::env::current_dir().expect("workspace"))
                .expect("canonical workspace");
            let store = AcpTerminalStore::new(root.clone());
            let session_id = SessionId::new("session");
            let created = store
                .create(
                    CreateTerminalRequest::new(session_id.clone(), "rustc")
                        .args(vec!["--version".into()])
                        .cwd(root)
                        .output_byte_limit(4_096),
                )
                .expect("create terminal");
            let cross_session = store
                .output(TerminalOutputRequest::new(
                    SessionId::new("other-session"),
                    created.terminal_id.clone(),
                ))
                .expect_err("cross-session terminal access must fail");
            assert!(cross_session.contains("another provider session"));
            let status = store
                .wait(WaitForTerminalExitRequest::new(
                    session_id.clone(),
                    created.terminal_id.clone(),
                ))
                .await
                .expect("wait for terminal");
            assert_eq!(status.exit_status.exit_code, Some(0));
            let output = store
                .output(TerminalOutputRequest::new(
                    session_id.clone(),
                    created.terminal_id.clone(),
                ))
                .expect("terminal output");
            assert!(output.output.contains("rustc "));
            store
                .release(ReleaseTerminalRequest::new(session_id, created.terminal_id))
                .expect("release terminal");
            store.shutdown();
        });
    }
}
