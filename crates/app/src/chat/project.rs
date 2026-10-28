//! Project persistence and activation belong to the host, never the paint path.
use super::*;
use dpl_chat_projects::{Binding, Project, ProjectId, Registry};
use dpl_chat_window::{ProjectChoice, ProjectView};
use serde_json::Value;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

pub(super) struct State {
    enabled: bool,
    boot_pending: bool,
    providers_enabled: bool,
    verified: Option<HostIntent>,
    pub(super) pending_chat_close: Option<ViewportGeneration>,
    pub(super) pending_root_close: bool,
    base: Result<PathBuf, String>,
    registry: Option<Registry>,
    active: Option<Project>,
    choices: Vec<Binding>,
    worker: Option<Receiver<Result<Outcome, String>>>,
    switching: bool,
    saved: Option<Value>,
    message: String,
    failed: bool,
    offline: bool,
    last_save: Instant,
    last_health: Instant,
}

struct Prepared {
    project: Project,
    archive: Option<Value>,
    codex: dpl_chat_codex::CodexActivationCoordinator,
    grok: dpl_chat_grok::GrokActivationCoordinator,
}

enum Outcome {
    Empty(Option<Registry>, Vec<Binding>, String),
    Prepared(Registry, Box<Prepared>),
    Activated(Registry, Box<Prepared>, Vec<Binding>),
    Saved(Value),
    Verified(HostIntent, Value),
    Health,
    Offline(String),
}

pub(super) enum Request {
    Setup { workspace: PathBuf, label: String },
    SaveCopy { workspace: PathBuf, label: String },
    Open(PathBuf),
    Select(ProjectId),
}

fn state_home() -> Result<PathBuf, String> {
    state_home_from(|key| std::env::var_os(key).map(PathBuf::from))
}

fn state_home_from(get: impl Fn(&str) -> Option<PathBuf>) -> Result<PathBuf, String> {
    let base = if let Some(path) = get("DPL_CHAT_STATE_HOME") {
        Some(path)
    } else if cfg!(windows) {
        get("LOCALAPPDATA").map(|path| path.join("DevPULSE/chat"))
    } else if cfg!(target_os = "linux") {
        get("XDG_STATE_HOME")
            .or_else(|| get("HOME").map(|path| path.join(".local/state")))
            .map(|path| path.join("devpulse/chat"))
    } else {
        None
    };
    base.filter(|path| path.is_absolute())
        .ok_or_else(|| "The machine-local chat folder is unavailable. Configure an absolute OS profile folder before setting up project chats.".into())
}

impl State {
    pub(super) fn new(enabled: bool, providers_enabled: bool) -> Self {
        Self {
            enabled,
            boot_pending: enabled,
            providers_enabled,
            verified: None,
            pending_chat_close: None,
            pending_root_close: false,
            base: if enabled {
                state_home()
            } else {
                Err("Authored fixture".into())
            },
            registry: None,
            active: None,
            choices: Vec::new(),
            worker: None,
            switching: false,
            saved: None,
            message: if enabled {
                "Choose a project to save chats and drafts automatically.".into()
            } else {
                "Authored preview — project storage is disabled.".into()
            },
            failed: false,
            offline: false,
            last_save: Instant::now(),
            last_health: Instant::now(),
        }
    }

    fn work(
        &mut self,
        ctx: &egui::Context,
        work: impl FnOnce() -> Result<Outcome, String> + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        self.worker = Some(receiver);
        std::thread::spawn(move || {
            let result = work();
            let _ = sender.send(result);
            // Both viewport callbacks can drain completions. Wake the
            // originating peer as well as root; hidden roots may not paint.
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            ctx.request_repaint();
        });
    }
}

/// Called once by the first frame. Reading an existing registry is allowed;
/// creating one requires the explicit Setup/Open action below.
pub(super) fn boot(state: &mut Shared, ctx: &egui::Context) {
    if !state.project.boot_pending || state.project.worker.is_some() {
        return;
    }
    state.project.boot_pending = false;
    let providers_enabled = state.project.providers_enabled;
    let base = state.project.base.clone();
    state.project.switching = true;
    state.window.set_project_input_locked(true);
    state.project.work(ctx, move || {
        let base = base?;
        match std::fs::symlink_metadata(base.join("registry.json")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Outcome::Empty(
                    None,
                    Vec::new(),
                    "Choose a project to save chats and drafts automatically.".into(),
                ));
            }
            Err(error) => return Err(error.to_string()),
            Ok(_) => {}
        }
        let registry = Registry::open(&base)?;
        let choices = registry.list()?;
        match registry.open_active().and_then(|project| {
            project
                .map(|project| prepare(project, providers_enabled))
                .transpose()
        }) {
            Ok(Some(prepared)) => Ok(Outcome::Prepared(registry, Box::new(prepared))),
            Ok(None) => Ok(Outcome::Empty(
                Some(registry),
                choices,
                "Choose a project to begin.".into(),
            )),
            Err(error) => Ok(Outcome::Empty(
                Some(registry),
                choices,
                format!("The saved project is unavailable: {error}"),
            )),
        }
    });
}

fn prepare(project: Project, providers_enabled: bool) -> Result<Prepared, String> {
    let archive = project.load_state()?;
    let codex = if providers_enabled {
        dpl_chat_codex::CodexActivationCoordinator::discover(project.workspace())
    } else {
        unconfigured_codex()
    };
    let grok = if providers_enabled {
        dpl_chat_grok::GrokActivationCoordinator::discover(project.workspace())
    } else {
        unconfigured_grok()
    };
    project.verify()?;
    Ok(Prepared {
        project,
        archive,
        codex,
        grok,
    })
}

pub(super) fn frozen(state: &Shared) -> bool {
    state.project.switching
}

pub(super) fn ready(state: &Shared) -> bool {
    state.project.active.is_some() && !state.project.switching && !state.project.offline
}

pub(super) fn require_ready(state: &mut Shared) -> bool {
    if ready(state) {
        return true;
    }
    state
        .window
        .set_composer_status("Choose a project and finish its setup before starting a chat.");
    false
}

fn idle(state: &Shared) -> bool {
    !state.codex_activation.is_login_running()
        && !state.grok_activation.is_login_running()
        && !matches!(
            state.codex_activation.activation(),
            ActivationState::SigningIn
                | ActivationState::AwaitingAuthorization(_)
                | ActivationState::Verifying
        )
        && !matches!(
            state.grok_activation.activation(),
            ActivationState::SigningIn
                | ActivationState::AwaitingAuthorization(_)
                | ActivationState::Verifying
        )
        && state.ai_calls.is_empty()
        && state.seam_permissions.is_empty()
        && state.grok_permissions.is_empty()
        && !state.imagine.in_flight()
        && !state
            .brainstorm
            .as_ref()
            .is_some_and(|board| board.assist.is_some())
        && state
            .window
            .project_provider_views()
            .iter()
            .all(|provider| {
                !matches!(
                    provider.registration.activation,
                    ActivationState::SigningIn
                        | ActivationState::AwaitingAuthorization(_)
                        | ActivationState::Verifying
                ) && provider.registration.sessions.iter().all(|session| {
                    !matches!(
                        session.state,
                        SessionState::Running | SessionState::Attention
                    )
                })
            })
}

pub(super) fn request(state: &mut Shared, ctx: &egui::Context, request: Request) {
    if !state.project.enabled {
        return;
    }
    if state.project.worker.is_some() || !idle(state) {
        state.project.message =
            "Finish the current turn, approval or sign-in before changing projects.".into();
        return;
    }
    let save_copy = matches!(request, Request::SaveCopy { .. });
    if save_copy && (!state.project.offline || state.project.active.is_none()) {
        state.project.message =
            "Save a copy is available when the active project's archive cannot be saved.".into();
        return;
    }
    let snapshot = match state.window.project_archive() {
        Ok(value) => value,
        Err(error) => {
            fail(state, error);
            return;
        }
    };
    let old = state.project.active.clone();
    let providers_enabled = state.project.providers_enabled;
    let base = state.project.base.clone();
    let registry = state.project.registry.clone();
    let binding = match &request {
        Request::Select(id) => state
            .project
            .choices
            .iter()
            .find(|binding| &binding.id == id)
            .cloned(),
        _ => None,
    };
    state.project.switching = true;
    state.window.set_project_input_locked(true);
    state.project.message = "Saving this project and checking the selected workspace…".into();
    state.project.work(ctx, move || {
        if let Some(old) = old.as_ref().filter(|_| !save_copy) {
            if let Err(error) = old.save_state(&snapshot) {
                return Ok(Outcome::Offline(error));
            }
        }
        let registry = match registry {
            Some(registry) => registry,
            None => Registry::open(&base?)?,
        };
        let target = match request {
            Request::Setup { workspace, label } => Project::create(&workspace, &label)?,
            Request::SaveCopy { workspace, label } => Project::create_new(&workspace, &label)?,
            Request::Open(workspace) => Project::open(&workspace)?,
            Request::Select(_) => binding
                .ok_or("The selected project is no longer registered")?
                .open()?,
        };
        let mut prepared = prepare(target, providers_enabled)?;
        // A draft typed before opting in must not vanish at first setup.
        if save_copy || (old.is_none() && has_conversations(&snapshot)) {
            if prepared.archive.is_some() {
                return Err(
                    "Save the current drafts in a new project before opening an existing archive."
                        .into(),
                );
            }
            prepared.project.save_state(&snapshot)?;
            prepared.archive = Some(snapshot);
        }
        Ok(Outcome::Prepared(registry, Box::new(prepared)))
    });
}

fn has_conversations(value: &Value) -> bool {
    value
        .get("providers")
        .and_then(Value::as_array)
        .is_some_and(|providers| {
            providers.iter().any(|provider| {
                ["sessions", "events"].iter().any(|key| {
                    provider
                        .get(key)
                        .and_then(Value::as_array)
                        .is_some_and(|items| !items.is_empty())
                }) || provider
                    .get("drafts")
                    .and_then(Value::as_array)
                    .is_some_and(|drafts| {
                        drafts.iter().any(|draft| {
                            draft
                                .get("text")
                                .and_then(Value::as_str)
                                .is_some_and(|text| !text.is_empty())
                        })
                    })
            })
        })
}

fn fail(state: &mut Shared, error: String) {
    state.project.switching = false;
    state.project.failed = state.project.offline;
    state.project.message = format!("Project chats were not saved or changed: {error}");
}

pub(super) fn poll(state: &mut Shared, ctx: &egui::Context) {
    let result = state
        .project
        .worker
        .as_ref()
        .and_then(|receiver| match receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "The project storage worker stopped; your chats and drafts remain open.".into(),
            )),
        });
    if let Some(result) = result {
        state.project.worker = None;
        match result {
            Err(error) => fail(state, error),
            Ok(Outcome::Offline(error)) => {
                state.project.offline = true;
                fail(state, error);
            }
            Ok(Outcome::Empty(registry, choices, message)) => {
                state.project.registry = registry;
                state.project.choices = choices;
                state.project.message = message;
                state.project.switching = false;
            }
            Ok(Outcome::Saved(snapshot)) => {
                state.project.saved = Some(snapshot);
                state.project.failed = false;
                state.project.offline = false;
                state.project.message = "Chats and drafts saved in this project.".into();
            }
            Ok(Outcome::Verified(intent, snapshot)) => {
                state.project.saved = Some(snapshot);
                state.project.failed = false;
                state.project.offline = false;
                state.project.switching = false;
                state.project.verified = Some(intent);
            }
            Ok(Outcome::Health) => {
                state.project.offline = false;
            }
            Ok(Outcome::Prepared(registry, prepared)) => {
                let valid = prepared.archive.as_ref().map_or(Ok(()), |archive| {
                    state.window.validate_project_archive(archive)
                });
                if let Err(error) = valid {
                    fail(state, error);
                } else {
                    state.project.work(ctx, move || {
                        // All fallible reads happen BEFORE committing selection.
                        let mut choices = registry.list()?;
                        let binding = Binding {
                            id: prepared.project.id().clone(),
                            label: prepared.project.label().into(),
                            workspace: prepared.project.workspace().into(),
                        };
                        choices.retain(|item| item.id != binding.id);
                        choices.push(binding);
                        registry.activate(&prepared.project)?;
                        Ok(Outcome::Activated(registry, prepared, choices))
                    });
                }
            }
            Ok(Outcome::Activated(registry, prepared, choices)) => {
                activate(state, registry, *prepared, choices)
            }
        }
    }
    publish(state);
    if state.project.worker.is_some() {
        ctx.request_repaint_after(Duration::from_millis(100));
    } else if state.project.active.is_some() {
        ctx.request_repaint_after(Duration::from_millis(750));
    }
}

fn activate(state: &mut Shared, registry: Registry, prepared: Prepared, choices: Vec<Binding>) {
    // Already validated while edits and old runtime events were frozen.
    if let Some(archive) = &prepared.archive {
        if let Err(error) = state.window.restore_project_archive(archive) {
            fail(state, error);
            return;
        }
    } else {
        state.window.clear_project_state();
    }
    let runtimes = std::mem::take(&mut state.runtimes);
    let grok_runtime = state.grok_runtime.take();
    let old_codex = std::mem::replace(&mut state.codex_activation, prepared.codex);
    let old_grok = std::mem::replace(&mut state.grok_activation, prepared.grok);
    // Runtime Drop joins its supervised worker; it must never run on egui.
    std::thread::spawn(move || {
        drop(runtimes);
        drop(grok_runtime);
        drop(old_codex);
        drop(old_grok);
    });
    state.grok_runtime_retry = None;
    state.grok_runtime_retry_at = None;
    state.grok_runtime_ready = false;
    state.grok_sessions.clear();
    state.grok_turns.clear();
    state.grok_permissions.clear();
    state.seam_sessions.clear();
    state.seam_turns.clear();
    state.seam_permissions.clear();
    state.ai_calls.clear();
    state.codex_attempt = None;
    state.grok_attempt = None;
    state.clyffy = None;
    restore_counters(state);
    state.project.registry = Some(registry);
    state.project.active = Some(prepared.project);
    state.project.choices = choices;
    state.project.saved = state.window.project_archive().ok();
    state.project.failed = false;
    state.project.offline = false;
    state.project.switching = false;
    state.project.message = "Project archive connected. Saved chats are available to read.".into();
    let _ = state.window.set_provider_activation(
        dpl_chat_codex::PROVIDER,
        state.codex_activation.activation().clone(),
    );
    let _ = state.window.set_provider_activation(
        dpl_chat_grok::PROVIDER,
        state.grok_activation.activation().clone(),
    );
    if state.provider_runtime_enabled && state.grok_activation.activation_capability_qualified() {
        begin_grok_runtime_verification(state, dpl_chat_grok::GrokVerificationOrigin::Startup);
    }
    sync_clyffy(state);
}

fn restore_counters(state: &mut Shared) {
    for provider in state.window.project_provider_views() {
        for session in provider.registration.sessions {
            state.next_chat_session_id = state.next_chat_session_id.max(session.id.0 + 1);
        }
        for event in provider.registration.events {
            state.next_chat_event_id = state.next_chat_event_id.max(event.id.0 + 1);
            if let Some(turn) = event.turn_id.and_then(|turn| {
                turn.0
                    .strip_prefix("grok-turn-")
                    .and_then(|suffix| suffix.parse::<u64>().ok())
            }) {
                state.next_turn_id = state.next_turn_id.max(turn + 1);
            }
        }
    }
}

fn publish(state: &mut Shared) {
    state
        .window
        .set_project_input_locked(state.project.switching);
    let choice = |project: &Project| ProjectChoice {
        id: project.id().clone(),
        label: project.label().into(),
        workspace: project.workspace().into(),
    };
    state.window.set_project_view(ProjectView {
        machine: crate::presence::platform_label().into(),
        active: state.project.active.as_ref().map(choice),
        choices: state
            .project
            .choices
            .iter()
            .map(|binding| ProjectChoice {
                id: binding.id.clone(),
                label: binding.label.clone(),
                workspace: binding.workspace.clone(),
            })
            .collect(),
        busy: state.project.worker.is_some(),
        can_switch: state.project.enabled && idle(state),
        can_save_copy: state.project.offline,
        message: state.project.message.clone(),
    });
}

pub(super) fn save(state: &mut Shared, ctx: &egui::Context, force: bool) {
    if state.project.worker.is_some() || state.project.switching {
        return;
    }
    let Some(project) = state.project.active.clone() else {
        return;
    };
    if !force
        && (state.project.failed || state.project.last_save.elapsed() < Duration::from_millis(750))
    {
        return;
    }
    // Even unchanged archives must obey the sampling interval; serializing a
    // large history on every repaint would stall the peer window.
    state.project.last_save = Instant::now();
    let snapshot = match state.window.project_archive() {
        Ok(value) => value,
        Err(error) => {
            fail(state, error);
            return;
        }
    };
    if !force && state.project.saved.as_ref() == Some(&snapshot) {
        if state.project.last_health.elapsed() >= Duration::from_secs(3) {
            state.project.last_health = Instant::now();
            state.project.work(ctx, move || {
                Ok(match project.verify() {
                    Ok(()) => Outcome::Health,
                    Err(error) => Outcome::Offline(error),
                })
            });
        }
        return;
    }
    state.project.last_save = Instant::now();
    state.project.work(ctx, move || {
        Ok(match project.save_state(&snapshot) {
            Ok(()) => Outcome::Saved(snapshot),
            Err(error) => Outcome::Offline(error),
        })
    });
}

pub(super) fn can_close(state: &mut Shared, ctx: &egui::Context) -> bool {
    if !state.project.enabled {
        return true;
    }
    if state.project.worker.is_some() {
        return false;
    }
    if !idle(state) {
        state.window.set_composer_status(
            "Finish the current turn, approval or sign-in before closing project chats.",
        );
        return false;
    }
    if state.project.active.is_none() {
        return match state.window.project_archive() {
            Ok(snapshot) if !has_conversations(&snapshot) => true,
            Ok(_) => {
                state.window.open_project_setup();
                state
                    .window
                    .set_composer_status("Choose a project to save your draft before closing.");
                false
            }
            Err(error) => {
                fail(state, error);
                false
            }
        };
    }
    match state.window.project_archive() {
        Ok(snapshot) if state.project.saved.as_ref() == Some(&snapshot) => true,
        Ok(_) => {
            if state.project.failed {
                state.window.set_composer_status("Your project has unsaved changes. Reconnect it and choose Retry save before closing.");
            } else {
                save(state, ctx, true);
            }
            false
        }
        Err(error) => {
            fail(state, error);
            false
        }
    }
}

/// Backstop for OS/application teardown. The worker owns all disk access.
/// Normal close is deferred by can_close while a save is pending or failed.
pub(super) fn flush_on_exit(state: &mut Shared) {
    if let Some(worker) = state.project.worker.take() {
        // Serialize against any in-flight save before writing the last snapshot.
        let _ = worker.recv();
    }
    let Some(project) = state.project.active.clone() else {
        return;
    };
    let snapshot = match state.window.project_archive() {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(%error, "Chat exit archive failed");
            return;
        }
    };
    let result = std::thread::spawn(move || project.save_state(&snapshot)).join();
    if !matches!(result, Ok(Ok(()))) {
        tracing::error!("Project chats could not be saved during application exit");
    }
}

/// Verify storage and persist the still-visible draft before authorizing a
/// new provider action. A removed/replaced drive cannot consume the input.
pub(super) fn verify_intent(state: &mut Shared, ctx: &egui::Context, intent: HostIntent) {
    if !require_ready(state) || !state.provider_runtime_enabled {
        return;
    }
    if let HostIntent::SendTurn {
        provider, session, ..
    } = &intent
    {
        if !live_session(state, *provider, *session) {
            state.window.set_composer_status("This saved chat is read-only. Start a new chat for this project; native resume is not connected yet.");
            return;
        }
    }
    if state.project.worker.is_some() {
        state.window.set_composer_status(
            "Project storage is busy. Your draft is preserved; try again when saving finishes.",
        );
        return;
    }
    let Some(project) = state.project.active.clone() else {
        return;
    };
    let snapshot = match state.window.project_archive() {
        Ok(value) => value,
        Err(error) => {
            fail(state, error);
            return;
        }
    };
    state.project.switching = true;
    state.window.set_project_input_locked(true);
    state.project.work(ctx, move || {
        Ok(match project.save_state(&snapshot) {
            Ok(()) => Outcome::Verified(intent, snapshot),
            Err(error) => Outcome::Offline(error),
        })
    });
}

pub(super) fn take_verified(state: &mut Shared) -> Option<HostIntent> {
    state.project.verified.take()
}

pub(super) fn live_session(state: &Shared, provider: ProviderId, session: ChatSessionId) -> bool {
    let writable = state.window.provider_view(provider).is_some_and(|view| {
        view.registration.sessions.iter().any(|saved| {
            saved.id == session && matches!(saved.state, SessionState::Idle | SessionState::Failed)
        })
    });
    writable
        && if provider == dpl_chat_grok::PROVIDER {
            state.grok_sessions.values().any(|bound| *bound == session)
        } else {
            state
                .seam_sessions
                .iter()
                .any(|((owner, _), bound)| *owner == provider && *bound == session)
        }
}

pub(super) fn workspace_label(state: &Shared) -> String {
    state.project.active.as_ref().map_or_else(
        || "Choose a project".into(),
        |project| project.label().into(),
    )
}

// A boot has no workspace authority. Discovery replaces these inert adapters
// only after a project has been selected and checked on the storage worker.
#[derive(Debug)]
struct NoProject;
impl dpl_chat_codex::CodexLoginLauncher for NoProject {
    fn launch_login(
        &self,
        _: &dpl_chat_codex::CodexRuntimeProfile,
    ) -> std::io::Result<Box<dyn dpl_chat_codex::CodexLoginProcess>> {
        Err(std::io::Error::other("Choose a project first"))
    }
}
impl dpl_chat_codex::CodexStatusProbe for NoProject {
    fn begin(
        &self,
        _: &dpl_chat_codex::CodexRuntimeProfile,
    ) -> std::io::Result<Box<dyn dpl_chat_codex::CodexStatusRequest>> {
        Err(std::io::Error::other("Choose a project first"))
    }
}
impl dpl_chat_grok::LoginLauncher for NoProject {
    fn launch_login(
        &self,
        _: &dpl_chat_grok::GrokRuntimeProfile,
    ) -> std::io::Result<Box<dyn dpl_chat_grok::LoginProcess>> {
        Err(std::io::Error::other("Choose a project first"))
    }
}

pub(crate) fn unconfigured_codex() -> dpl_chat_codex::CodexActivationCoordinator {
    dpl_chat_codex::CodexActivationCoordinator::from_parts(
        Err(dpl_chat_codex::CodexDiscoveryError::WorkingDirectory(
            "Choose a project first".into(),
        )),
        ActivationState::Blocked {
            reason: "Choose a project first".into(),
            recovery: Vec::new(),
        },
        Box::new(NoProject),
        Box::new(NoProject),
    )
}

pub(crate) fn unconfigured_grok() -> dpl_chat_grok::GrokActivationCoordinator {
    dpl_chat_grok::GrokActivationCoordinator::from_parts(
        Err(dpl_chat_grok::GrokDiscoveryError::WorkingDirectory(
            "Choose a project first".into(),
        )),
        Box::new(NoProject),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "dpl-host-project-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn host(base: &Path) -> Shared {
        let fabric = Fabric::new();
        let runtime = ChatRuntime::try_from_registrations([
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_codex::MANIFEST),
            dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_grok::MANIFEST),
        ])
        .unwrap();
        let coordinator = ChatWindowCoordinator::new(
            &fabric,
            runtime,
            ChatWindowConfig {
                initial_route: Route {
                    hub: dpl_hub_home::HUB,
                    sub: dpl_hub_home::MAIN,
                },
                catalog: HistoryCatalog::default(),
                routes: Vec::new(),
                authored_models: Some((SocialModel::default(), InteractionRegistry::default())),
                codex_activation: unconfigured_codex(),
                grok_activation: unconfigured_grok(),
                open_at_boot: false,
                estate: None,
                #[cfg(feature = "eguipilotai-devtools")]
                proof_mode: None,
            },
        );
        let mut state = Arc::try_unwrap(coordinator.shared)
            .unwrap_or_else(|_| panic!("sole host owner"))
            .into_inner()
            .unwrap();
        state.project = State::new(true, false);
        state.project.base = Ok(base.to_owned());
        state
    }

    fn settle(state: &mut Shared, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.project.worker.is_some() {
            assert!(Instant::now() < deadline, "project worker did not finish");
            poll(state, ctx);
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn peer_frames_until(
        shared: &Arc<Mutex<Shared>>,
        ctx: &egui::Context,
        done: impl Fn(&Shared) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            // This is exactly the independent peer callback's service entry;
            // never invoke ChatWindowCoordinator::frame or the root close hook.
            service_project(ctx, shared);
            if done(&lock(shared)) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "peer-only project servicing stalled"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn peer_frames_alone_complete_setup_autosave_preflight_and_deferred_close() {
        let root = scratch("hidden-root");
        let shared = Arc::new(Mutex::new(host(&root.join("machine"))));
        let ctx = egui::Context::default();
        peer_frames_until(&shared, &ctx, |state| state.project.worker.is_none());
        let workspace = root.join("A");
        std::fs::create_dir(&workspace).unwrap();
        request(
            &mut lock(&shared),
            &ctx,
            Request::Setup {
                workspace: workspace.clone(),
                label: "A".into(),
            },
        );
        peer_frames_until(&shared, &ctx, |state| {
            state.project.active.is_some() && !frozen(state)
        });
        {
            let mut state = lock(&shared);
            draft(&mut state, "saved with the main window hidden");
            state.project.last_save = Instant::now() - Duration::from_secs(1);
        }
        peer_frames_until(&shared, &ctx, |state| {
            state.project.worker.is_none()
                && state.project.saved.as_ref() == state.window.project_archive().ok().as_ref()
        });
        assert!(Project::open(&workspace)
            .unwrap()
            .load_state()
            .unwrap()
            .is_some());
        {
            let mut state = lock(&shared);
            state.provider_runtime_enabled = true;
            verify_intent(
                &mut state,
                &ctx,
                HostIntent::NewSession(dpl_chat_codex::PROVIDER),
            );
        }
        // Dispatch takes Shared again: this also guards against accidentally
        // executing verified commands while the service still holds its lock.
        peer_frames_until(&shared, &ctx, |state| {
            state.project.worker.is_none() && !frozen(state) && state.project.verified.is_none()
        });
        let generation = {
            let mut state = lock(&shared);
            let id = state.registry.request_primary(WindowBinding::default());
            transition_opening(&mut state.registry, id);
            state.registry.mark_visible(id).unwrap();
            draft(&mut state, "flush this on close");
            state.registry.record(id).unwrap().generation
        };
        close_chat(&ctx, &shared, generation);
        assert!(lock(&shared).project.pending_chat_close.is_some());
        peer_frames_until(&shared, &ctx, |state| {
            state.project.pending_chat_close.is_none()
        });
        assert_eq!(
            lock(&shared)
                .registry
                .record(ChatWindowId::PRIMARY)
                .unwrap()
                .lifecycle,
            WindowLifecycle::Closed
        );
        let stored = Project::open(&workspace)
            .unwrap()
            .load_state()
            .unwrap()
            .unwrap();
        assert_eq!(Some(&stored), lock(&shared).project.saved.as_ref());
    }

    fn setup(state: &mut Shared, ctx: &egui::Context, root: &Path, label: &str) -> ProjectId {
        let workspace = root.join(label);
        std::fs::create_dir(&workspace).unwrap();
        request(
            state,
            ctx,
            Request::Setup {
                workspace,
                label: label.into(),
            },
        );
        settle(state, ctx);
        state
            .project
            .active
            .as_ref()
            .expect("active project")
            .id()
            .clone()
    }

    fn draft(state: &mut Shared, text: &str) {
        state
            .window
            .apply(dpl_chat_window::ChatWindowIntent::UpdateAiDraft {
                provider: dpl_chat_codex::PROVIDER,
                value: text.into(),
            })
            .unwrap();
    }

    fn draft_text(state: &Shared) -> String {
        state
            .window
            .provider_view(dpl_chat_codex::PROVIDER)
            .unwrap()
            .draft
    }

    #[test]
    fn boot_reads_existing_bindings_but_never_creates_a_registry() {
        let root = scratch("boot");
        let base = root.join("machine");
        let mut state = host(&base);
        let ctx = egui::Context::default();
        boot(&mut state, &ctx);
        settle(&mut state, &ctx);
        assert!(!base.exists());
        assert!(!ready(&state));
        assert!(state.project.choices.is_empty());
    }

    #[test]
    fn project_a_b_a_and_restart_recover_their_own_drafts() {
        let root = scratch("roundtrip");
        let base = root.join("machine");
        let mut state = host(&base);
        let ctx = egui::Context::default();
        let a = setup(&mut state, &ctx, &root, "A");
        draft(&mut state, "A unfinished idea");
        let b = setup(&mut state, &ctx, &root, "B");
        assert!(draft_text(&state).is_empty());
        draft(&mut state, "B different idea");
        request(&mut state, &ctx, Request::Select(a.clone()));
        settle(&mut state, &ctx);
        assert_eq!(draft_text(&state), "A unfinished idea");
        assert_eq!(
            Registry::open(&base).unwrap().active().unwrap().unwrap().id,
            a
        );
        let mut restarted = host(&base);
        boot(&mut restarted, &ctx);
        settle(&mut restarted, &ctx);
        assert_eq!(draft_text(&restarted), "A unfinished idea");
        request(&mut restarted, &ctx, Request::Select(b));
        settle(&mut restarted, &ctx);
        assert_eq!(draft_text(&restarted), "B different idea");
    }

    #[test]
    fn invalid_target_keeps_the_old_project_and_draft_usable() {
        let root = scratch("bad-target");
        let mut state = host(&root.join("machine"));
        let ctx = egui::Context::default();
        let a = setup(&mut state, &ctx, &root, "A");
        draft(&mut state, "keep my work");
        request(&mut state, &ctx, Request::Open(root.join("missing")));
        settle(&mut state, &ctx);
        assert_eq!(state.project.active.as_ref().unwrap().id(), &a);
        assert_eq!(draft_text(&state), "keep my work");
        assert!(ready(&state));
        assert!(!frozen(&state));
    }

    #[test]
    fn failed_flush_blocks_switch_and_close_then_retry_preserves_the_draft() {
        let root = scratch("save-failure");
        let mut state = host(&root.join("machine"));
        let ctx = egui::Context::default();
        let a = setup(&mut state, &ctx, &root, "A");
        draft(&mut state, "unsaved work");
        let lock_path = state
            .project
            .active
            .as_ref()
            .unwrap()
            .archive_dir()
            .join("writer.lock");
        let held = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .unwrap();
        held.lock().unwrap();
        let b = root.join("B");
        std::fs::create_dir(&b).unwrap();
        request(
            &mut state,
            &ctx,
            Request::Setup {
                workspace: b,
                label: "B".into(),
            },
        );
        settle(&mut state, &ctx);
        assert_eq!(state.project.active.as_ref().unwrap().id(), &a);
        assert_eq!(draft_text(&state), "unsaved work");
        assert!(!ready(&state));
        assert!(!can_close(&mut state, &ctx));
        drop(held);
        save(&mut state, &ctx, true);
        settle(&mut state, &ctx);
        assert!(ready(&state));
        assert!(can_close(&mut state, &ctx));
        assert_eq!(draft_text(&state), "unsaved work");
    }

    #[test]
    fn unconfigured_and_offline_actions_never_reach_provider_dispatch() {
        let root = scratch("send-gate");
        let mut state = host(&root.join("machine"));
        let ctx = egui::Context::default();
        state.provider_runtime_enabled = true;
        verify_intent(
            &mut state,
            &ctx,
            HostIntent::NewSession(dpl_chat_codex::PROVIDER),
        );
        assert!(state.project.worker.is_none());
        assert!(take_verified(&mut state).is_none());
        setup(&mut state, &ctx, &root, "A");
        draft(&mut state, "do not lose this");
        let lock_path = state
            .project
            .active
            .as_ref()
            .unwrap()
            .archive_dir()
            .join("writer.lock");
        let held = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .unwrap();
        held.lock().unwrap();
        verify_intent(
            &mut state,
            &ctx,
            HostIntent::NewSession(dpl_chat_codex::PROVIDER),
        );
        settle(&mut state, &ctx);
        assert!(take_verified(&mut state).is_none());
        assert_eq!(draft_text(&state), "do not lose this");
        assert!(!ready(&state));
    }

    #[test]
    fn a_pre_setup_draft_prevents_close_until_it_has_a_project() {
        let root = scratch("pre-setup-close");
        let mut state = host(&root.join("machine"));
        let ctx = egui::Context::default();
        assert!(can_close(&mut state, &ctx));
        draft(&mut state, "not saved yet");
        assert!(!can_close(&mut state, &ctx));
        assert!(!root.join("machine").exists());
        setup(&mut state, &ctx, &root, "A");
        assert_eq!(draft_text(&state), "not saved yet");
        assert!(can_close(&mut state, &ctx));
    }

    #[test]
    fn stale_writer_can_save_a_new_copy_without_changing_either_old_history() {
        let root = scratch("recovery-copy");
        let mut state = host(&root.join("machine"));
        let ctx = egui::Context::default();
        let a = setup(&mut state, &ctx, &root, "A");
        draft(&mut state, "common checkpoint");
        save(&mut state, &ctx, true);
        settle(&mut state, &ctx);
        let other_writer = Project::open(&root.join("A")).unwrap();
        draft(&mut state, "other window's work");
        let remote = state.window.project_archive().unwrap();
        other_writer.save_state(&remote).unwrap();
        draft(&mut state, "my unsaved work");
        save(&mut state, &ctx, true);
        settle(&mut state, &ctx);
        assert!(state.project.offline);
        // Recovery is never permission to adopt an existing marked archive.
        request(
            &mut state,
            &ctx,
            Request::SaveCopy {
                workspace: root.join("A"),
                label: "collision".into(),
            },
        );
        settle(&mut state, &ctx);
        assert_eq!(state.project.active.as_ref().unwrap().id(), &a);
        assert_eq!(other_writer.load_state().unwrap().unwrap(), remote);
        let copy = root.join("copy");
        std::fs::create_dir(&copy).unwrap();
        request(
            &mut state,
            &ctx,
            Request::SaveCopy {
                workspace: copy.clone(),
                label: "Recovered".into(),
            },
        );
        settle(&mut state, &ctx);
        assert_ne!(state.project.active.as_ref().unwrap().id(), &a);
        assert_eq!(draft_text(&state), "my unsaved work");
        assert_eq!(other_writer.load_state().unwrap().unwrap(), remote);
        assert!(Project::open(&copy)
            .unwrap()
            .load_state()
            .unwrap()
            .is_some());
        assert!(ready(&state));
    }

    #[test]
    fn restored_ids_advance_every_host_counter_and_remain_read_only() {
        let root = scratch("counters");
        let mut state = host(&root.join("machine"));
        let ctx = egui::Context::default();
        let a = setup(&mut state, &ctx, &root, "A");
        let provider = dpl_chat_codex::PROVIDER;
        state
            .window
            .upsert_provider_session(
                provider,
                SessionSummary {
                    id: ChatSessionId(9_000_000),
                    provider_id: provider,
                    provider_session_id: Some(ProviderSessionId("native".into())),
                    title: "Saved".into(),
                    workspace: "A".into(),
                    group: SessionGroup::Active,
                    state: SessionState::Idle,
                },
            )
            .unwrap();
        state
            .window
            .append_provider_event(
                provider,
                ChatEvent {
                    id: ChatEventId(8_000_000),
                    provider_id: provider,
                    session_id: ChatSessionId(9_000_000),
                    turn_id: Some(TurnId("grok-turn-7000000".into())),
                    content: ChatEventContent::UserMessage("hello".into()),
                },
            )
            .unwrap();
        setup(&mut state, &ctx, &root, "B");
        request(&mut state, &ctx, Request::Select(a));
        settle(&mut state, &ctx);
        assert!(state.next_chat_session_id > 9_000_000);
        assert!(state.next_chat_event_id > 8_000_000);
        assert!(state.next_turn_id > 7_000_000);
        assert_eq!(
            state
                .window
                .provider_view(provider)
                .unwrap()
                .registration
                .sessions[0]
                .state,
            SessionState::ReadOnly
        );
        assert!(state.seam_sessions.is_empty());
        // A stale typed intent must not bypass the UI's read-only guard.
        state.provider_runtime_enabled = true;
        verify_intent(
            &mut state,
            &ctx,
            HostIntent::SendTurn {
                provider,
                session: ChatSessionId(9_000_000),
                text: "must not send".into(),
            },
        );
        assert!(state.project.worker.is_none());
        assert!(take_verified(&mut state).is_none());
        assert!(!live_session(&state, provider, ChatSessionId(9_000_000)));
    }
    #[test]
    fn missing_or_relative_machine_state_never_falls_back_to_cwd() {
        assert!(state_home_from(|_| None).is_err());
        assert!(state_home_from(
            |key| (key == "DPL_CHAT_STATE_HOME").then(|| PathBuf::from("relative"))
        )
        .is_err());
        let root = std::env::temp_dir().join("project-state-test");
        assert_eq!(
            state_home_from(|key| (key == "DPL_CHAT_STATE_HOME").then(|| root.clone())).unwrap(),
            root
        );
    }
    #[test]
    fn pre_setup_drafts_are_detected_without_importing_empty_views() {
        assert!(!has_conversations(
            &serde_json::json!({"providers":[{"sessions":[],"events":[],"drafts":[{"text":""}]}]})
        ));
        assert!(has_conversations(
            &serde_json::json!({"providers":[{"sessions":[],"events":[],"drafts":[{"text":"keep me"}]}]})
        ));
    }
}
