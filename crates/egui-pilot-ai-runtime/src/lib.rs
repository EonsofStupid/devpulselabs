//! Transport-independent EguiPilotAI application runtime.
//!
//! An [`RuntimeGuard`] is owned by the application, so its `egui::Context`
//! clone and transport workers end with the application. The egui plugin
//! itself never retains a context.

use egui_pilot_ai::{install, CoachMount, MotionPolicy, PilotHandle, QueuedInput};
pub use egui_pilot_ai_protocol as protocol;
use egui_pilot_ai_transport::{Transport, TransportError};
use protocol::{
    Action, ActionKind, AgentSnapshot, CancellationResult, CancellationState, CaptureResult,
    ClientCommand, CoachCommand, CoachEvent, CoachExpectationState, CoachId, CoachRefusal,
    CoachStyle, CoachUnmountReason, CommandEnvelope, CommandResult, EventId, EventScope,
    Expectation, Flow, Handshake, HandshakeRequest, Locator, NodeId, PilotError, PointerButton,
    RequestId, ResponseEnvelope, ResponseOutcome, RunId, RunOutcome, RunResult, RuntimeEvent,
    RuntimeEventEnvelope, SemanticFrame, SemanticNode, ServerMessage, SessionId, SessionStatus,
    SnapshotRequest, Step, StepId, StepOutcome, Touchpoint, TriggerKind, ViewportDescriptor,
    ViewportEventKind, ViewportExpectationState, ViewportLifecycle, ViewportSelector,
    PROTOCOL_VERSION,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub build: String,
    pub session: SessionId,
    pub session_nonce: String,
    pub artifact_root: PathBuf,
    pub max_commands_per_frame: usize,
    pub renderer_identity: Option<protocol::RendererIdentity>,
}

impl RuntimeConfig {
    pub fn new(build: impl Into<String>, artifact_root: impl Into<PathBuf>) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let nonce = format!("{}-{stamp:x}", process_identity());
        Self {
            build: build.into(),
            session: SessionId::new(format!("session-{nonce}")),
            session_nonce: nonce,
            artifact_root: artifact_root.into(),
            max_commands_per_frame: 32,
            renderer_identity: None,
        }
    }

    pub fn handshake(&self) -> Handshake {
        Handshake {
            protocol: PROTOCOL_VERSION,
            build: self.build.clone(),
            session_nonce: self.session_nonce.clone(),
            features: vec!["native-runtime".into(), "semantic-projection".into()],
            capabilities: protocol::supported_capabilities(),
            limits: protocol::ProtocolLimits::default(),
            queues: protocol::QueuePolicy::default(),
            renderer: self.renderer_identity.clone(),
        }
    }
}

struct ActiveFlow {
    request_id: RequestId,
    run_id: RunId,
    flow: Flow,
    index: usize,
    wait_started: Option<Instant>,
    step_started: bool,
    outcomes: Vec<StepOutcome>,
}

struct ActionExecution {
    request_id: RequestId,
    run_id: RunId,
    step_id: StepId,
    step_index: usize,
    started: Instant,
    sequence: InputSequence,
    wait_viewport: Option<(ViewportSelector, ViewportExpectationState)>,
    settlement: Option<ActionSettlement>,
}

struct ActionSettlement {
    viewport: ViewportDescriptor,
    egui_viewport: egui::ViewportId,
    required_input_generation: u64,
    last_viewport_revision: u64,
    last_semantic_hash: Option<String>,
    last_geometry_hash: Option<String>,
    stable_frames: u8,
}

#[derive(Default)]
struct InputSequence {
    ticks: VecDeque<QueuedInput>,
}

#[derive(Default)]
struct InputSourceState {
    keys: HashMap<egui::Key, (egui::ViewportId, egui::Modifiers)>,
    buttons: Vec<(
        egui::PointerButton,
        egui::ViewportId,
        egui::Pos2,
        egui::Modifiers,
    )>,
    pointer_positions: HashMap<egui::ViewportId, egui::Pos2>,
}

struct Stability {
    rect: protocol::Rect,
    revision: u64,
    frames: u8,
}

/// What a touchpoint has done in this session, so an `expect` step can assert
/// a press that already happened while an earlier step was running.
#[derive(Default, Clone, Copy)]
struct CoachSeen {
    armed: bool,
    mounted: bool,
    triggered: bool,
    satisfied: bool,
    unmounted: bool,
    refused: bool,
}

impl CoachSeen {
    fn record(&mut self, event: &CoachEvent) {
        match event {
            CoachEvent::Armed { .. } => self.armed = true,
            CoachEvent::Mounted { .. } => self.mounted = true,
            CoachEvent::Triggered { .. } => self.triggered = true,
            CoachEvent::Satisfied { .. } => self.satisfied = true,
            CoachEvent::Unmounted { .. } => self.unmounted = true,
            CoachEvent::Refused { .. } => self.refused = true,
        }
    }

    fn matches(self, state: CoachExpectationState) -> bool {
        match state {
            CoachExpectationState::Armed => self.armed,
            CoachExpectationState::Mounted => self.mounted,
            CoachExpectationState::Triggered => self.triggered,
            CoachExpectationState::Satisfied => self.satisfied,
            CoachExpectationState::Unmounted => self.unmounted,
            CoachExpectationState::Refused => self.refused,
        }
    }
}

struct PendingCapture {
    request_id: RequestId,
    run_id: Option<RunId>,
    step_id: Option<StepId>,
    name: String,
    viewport: ViewportDescriptor,
    egui_viewport: egui::ViewportId,
    flow_step: Option<usize>,
    last_semantic_hash: Option<String>,
    last_geometry_hash: Option<String>,
    last_pixel_hash: Option<String>,
    attempts: u8,
    started_at: Instant,
    last_request_at: Instant,
}

pub struct RuntimeGuard<T: Transport> {
    ctx: egui::Context,
    pilot: PilotHandle,
    transport: T,
    config: RuntimeConfig,
    authenticated: bool,
    outbound: VecDeque<ServerMessage>,
    telemetry: Option<(EventScope, RuntimeEvent)>,
    dropped_telemetry: u64,
    event_sequence: u64,
    active_flow: Option<ActiveFlow>,
    action: Option<ActionExecution>,
    stability: HashMap<NodeId, Stability>,
    coach_ledger: HashMap<CoachId, CoachSeen>,
    pending_capture: Option<PendingCapture>,
    capture_job: Option<Receiver<Result<CaptureResult, PilotError>>>,
    terminal_runs: VecDeque<RunResult>,
    input_state: InputSourceState,
    closed_viewports: VecDeque<ViewportDescriptor>,
    viewport_egui_history: HashMap<protocol::ViewportId, egui::ViewportId>,
    poisoned: Option<PilotError>,
    stopping: bool,
    stop_request: Option<RequestId>,
    stop_started: Option<Instant>,
    stop_close_requested: HashSet<egui::ViewportId>,
    stop_response_queued: bool,
    stopped: bool,
}

impl<T: Transport> RuntimeGuard<T> {
    pub fn attach(ctx: &egui::Context, transport: T, config: RuntimeConfig) -> Self {
        let pilot = install(ctx);
        if let Some(renderer) = &config.renderer_identity {
            let _ = std::fs::create_dir_all(&config.artifact_root);
            let path = config.artifact_root.join("renderer-identity.json");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                let _ = serde_json::to_writer_pretty(&mut file, renderer);
            }
        }
        Self {
            ctx: ctx.clone(),
            pilot,
            transport,
            config,
            authenticated: false,
            outbound: VecDeque::new(),
            telemetry: None,
            dropped_telemetry: 0,
            event_sequence: 0,
            active_flow: None,
            action: None,
            stability: HashMap::new(),
            coach_ledger: HashMap::new(),
            pending_capture: None,
            capture_job: None,
            terminal_runs: VecDeque::new(),
            input_state: InputSourceState::default(),
            closed_viewports: VecDeque::new(),
            viewport_egui_history: HashMap::new(),
            poisoned: None,
            stopping: false,
            stop_request: None,
            stop_started: None,
            stop_close_requested: HashSet::new(),
            stop_response_queued: false,
            stopped: false,
        }
    }

    pub fn pilot(&self) -> &PilotHandle {
        &self.pilot
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// Pump bounded control work. Call once near the end of each app frame.
    pub fn pump(&mut self) {
        if self.stopped {
            return;
        }
        self.flush_outbound();
        if self.stopping {
            self.forward_viewport_events();
            self.forward_coach_events();
            self.advance_stop();
            self.flush_outbound();
            self.finish_stop_if_flushed();
            return;
        }
        // Required responses and events are lossless. When the transport is
        // pressured, stop producing at this frame boundary until it drains.
        if self.outbound.len() >= self.config.handshake().queues.required_event_capacity {
            return;
        }
        self.poll_capture_job();
        self.forward_viewport_events();
        self.forward_coach_events();
        self.accept_captured_image();
        self.advance_action();
        self.advance_flow();

        for _ in 0..self.config.max_commands_per_frame {
            if self.outbound.len() + 4 >= self.config.handshake().queues.required_event_capacity {
                break;
            }
            match self.transport.try_receive() {
                Ok(Some(envelope)) => {
                    self.handle_command(envelope);
                    if self.stopping {
                        break;
                    }
                }
                Ok(None) => break,
                Err(TransportError::Closed) => {
                    self.stopped = true;
                    break;
                }
                Err(error) => {
                    self.queue_event(
                        EventScope::default(),
                        RuntimeEvent::Diagnostic {
                            error: PilotError {
                                code: "transport".into(),
                                message: error.to_string(),
                                context: None,
                            },
                        },
                    );
                    break;
                }
            }
        }
        self.flush_outbound();
        self.finish_stop_if_flushed();
    }

    fn handle_command(&mut self, envelope: CommandEnvelope) {
        if envelope.session != self.config.session {
            self.queue_error(
                envelope.request_id,
                "session-mismatch",
                "command session does not match the target session".into(),
                None,
            );
            return;
        }
        if let ClientCommand::Handshake(handshake) = envelope.command {
            self.handshake(envelope.request_id, handshake);
            return;
        }
        if !self.authenticated {
            self.queue_error(
                envelope.request_id,
                "handshake-required",
                "a valid handshake is required before control commands".into(),
                None,
            );
            return;
        }
        match envelope.command {
            ClientCommand::Snapshot(request) => self.snapshot(envelope.request_id, &request),
            ClientCommand::Run { run_id, flow } => {
                self.start_flow(envelope.request_id, run_id, flow)
            }
            ClientCommand::Capture { name, viewport } => {
                self.start_capture(envelope.request_id, name, viewport, None, None, None)
            }
            ClientCommand::Coach { coach } => self.coach(envelope.request_id, coach),
            ClientCommand::Status => self.status(envelope.request_id),
            ClientCommand::Cancel { run_id } => self.cancel(envelope.request_id, run_id),
            ClientCommand::Stop => self.stop(envelope.request_id),
            ClientCommand::Handshake(_) => unreachable!("handshake handled above"),
        }
    }

    fn handshake(&mut self, request_id: RequestId, handshake: HandshakeRequest) {
        if handshake.protocol != PROTOCOL_VERSION {
            self.queue_error(
                request_id,
                "protocol-version",
                format!(
                    "target protocol is {PROTOCOL_VERSION}, client requested {}",
                    handshake.protocol
                ),
                None,
            );
            return;
        }
        let supported = protocol::supported_capabilities();
        let missing: Vec<_> = handshake
            .required_capabilities
            .iter()
            .filter(|capability| !supported.contains(capability))
            .cloned()
            .collect();
        if !missing.is_empty() {
            self.queue_error(
                request_id,
                "missing-capability",
                format!("target does not support required capabilities: {missing:?}"),
                None,
            );
            return;
        }
        if handshake.session_nonce != self.config.session_nonce {
            self.queue_error(
                request_id,
                "nonce-mismatch",
                "session nonce did not match the private child launch".into(),
                None,
            );
            return;
        }
        self.authenticated = true;
        self.queue_success(
            request_id,
            CommandResult::Ready {
                handshake: self.config.handshake(),
            },
        );
    }

    fn snapshot(&mut self, request_id: RequestId, request: &SnapshotRequest) {
        match self.pilot.snapshot(request) {
            Ok(snapshot) => self.queue_success(request_id, CommandResult::Snapshot { snapshot }),
            Err(error) => self.queue_error(request_id, "snapshot", error.to_string(), None),
        }
    }

    fn status(&mut self, request_id: RequestId) {
        let frame = self.pilot.latest_frame();
        let viewports = self.pilot.viewports();
        self.queue_success(
            request_id,
            CommandResult::Status {
                status: SessionStatus {
                    running: !self.stopped,
                    revision: frame.revision,
                    pending_actions: self.pilot.pending_input()
                        + usize::from(self.action.is_some()),
                    busy_tokens: frame.diagnostics.busy_tokens.len(),
                    viewport_count: viewports.len(),
                    viewports,
                    renderer_key: self
                        .config
                        .renderer_identity
                        .as_ref()
                        .map(protocol::RendererIdentity::baseline_key),
                    renderer_device_type: self
                        .config
                        .renderer_identity
                        .as_ref()
                        .map(|renderer| renderer.device_type.clone()),
                    active_run: self.active_flow.as_ref().map(|flow| flow.run_id.clone()),
                    coach_signals: self.pilot.coach_signals(),
                },
            },
        );
    }

    /// Answer one coach command, then report its immediate outcome both as
    /// the response and as events, so a `Mount` that lands and a `Mount` that
    /// is refused are equally visible to a streaming agent.
    fn coach(&mut self, request_id: RequestId, command: CoachCommand) {
        self.forward_coach_events();
        let events = self.apply_coach(command);
        for event in &events {
            self.publish_coach(event.clone());
        }
        self.queue_success(request_id, CommandResult::Coach { events });
    }

    fn apply_coach(&mut self, command: CoachCommand) -> Vec<CoachEvent> {
        match command {
            CoachCommand::Mount { touchpoint } => match self.mount_coach(&touchpoint) {
                Ok(()) => self.pilot.take_coach_events(),
                Err((reason, detail)) => vec![CoachEvent::Refused {
                    coach_id: touchpoint.coach_id.clone(),
                    reason,
                    detail,
                }],
            },
            CoachCommand::Unmount { coach_id } => {
                if self
                    .pilot
                    .unmount_coach(&coach_id, CoachUnmountReason::Agent)
                {
                    self.pilot.take_coach_events()
                } else {
                    vec![CoachEvent::Refused {
                        coach_id,
                        reason: CoachRefusal::UnknownTouchpoint,
                        detail: "no touchpoint with that id is mounted".into(),
                    }]
                }
            }
            CoachCommand::UnmountAll => {
                self.pilot.unmount_all_coach(CoachUnmountReason::Agent);
                self.pilot.take_coach_events()
            }
        }
    }

    /// Resolve a touchpoint against the live frame. Unshipped visuals refuse
    /// here rather than mounting a lie.
    ///
    /// A mount is a **subscription, not a resolution**: a node-id target that
    /// is not on screen *yet* arms and binds when it appears, whatever the
    /// trigger, because "click the thing the last step revealed" is the shape
    /// of every lesson. A `NodeAppeared` touchpoint arms even over a target
    /// that is already on screen: "tell me when this is there" should answer
    /// immediately when it already is, and waiting for a second appearance
    /// would be a trap.
    fn mount_coach(&mut self, touchpoint: &Touchpoint) -> Result<(), (CoachRefusal, String)> {
        if touchpoint.visual != CoachStyle::Spotlight {
            return Err((
                CoachRefusal::UnsupportedStyle,
                format!(
                    "{:?} is not painted yet; only spotlight is",
                    touchpoint.visual
                ),
            ));
        }
        let resolved = self.resolve_touchpoint(touchpoint);
        // Anything not on screen yet waits. An appearance waits even when its
        // target is already there, for the reason on the doc comment.
        let arm = touchpoint.on == TriggerKind::NodeAppeared || resolved.is_err();
        let mount = if arm {
            // A touchpoint that may outlive its target's absence needs a node
            // id to watch for; a name or role locator cannot match a node that
            // does not exist yet.
            let node = touchpoint.target.id.clone().ok_or_else(|| {
                (
                    CoachRefusal::TargetNotFound,
                    format!(
                        "a {:?} touchpoint may wait for a target that does not exist yet, so it needs a node id rather than a name or role locator",
                        touchpoint.on
                    ),
                )
            })?;
            CoachMount {
                coach_id: touchpoint.coach_id.clone(),
                node,
                viewport: None,
                rect: None,
                on: touchpoint.on.clone(),
                until: touchpoint.until.clone(),
                message: touchpoint.message.clone(),
                ttl_ms: touchpoint.ttl_ms,
                once: touchpoint.once,
            }
        } else {
            let (node, viewport) = resolved?;
            CoachMount {
                coach_id: touchpoint.coach_id.clone(),
                node: node.id.clone(),
                viewport: Some(viewport),
                rect: Some(node.rect),
                on: touchpoint.on.clone(),
                until: touchpoint.until.clone(),
                message: touchpoint.message.clone(),
                ttl_ms: touchpoint.ttl_ms,
                once: touchpoint.once,
            }
        };
        let viewport = mount.viewport;
        self.pilot.mount_coach(mount).map_err(|reason| {
            (
                reason,
                format!("touchpoint `{}` is already mounted", touchpoint.coach_id),
            )
        })?;
        // An armed touchpoint needs the root ticking to notice its target
        // arrive; a bound one only needs its own viewport.
        self.ctx
            .request_repaint_of(viewport.unwrap_or(egui::ViewportId::ROOT));
        Ok(())
    }

    fn resolve_touchpoint(
        &self,
        touchpoint: &Touchpoint,
    ) -> Result<(SemanticNode, egui::ViewportId), (CoachRefusal, String)> {
        let frame = self.pilot.latest_frame();
        let node = resolve_locator(&frame, &touchpoint.target).map_err(|error| {
            let reason = if error.code == "locator-ambiguous" {
                CoachRefusal::TargetAmbiguous
            } else {
                CoachRefusal::TargetNotFound
            };
            (reason, error.message)
        })?;
        if !node.visible() {
            return Err((
                CoachRefusal::TargetNotVisible,
                format!("node `{}` is not visible", node.id),
            ));
        }
        let viewport = self.pilot.egui_viewport(&node.viewport).ok_or_else(|| {
            (
                CoachRefusal::TargetNotFound,
                format!("viewport `{}` is no longer live", node.viewport),
            )
        })?;
        Ok((node.clone(), viewport))
    }

    fn forward_coach_events(&mut self) {
        for event in self.pilot.take_coach_events() {
            self.publish_coach(event);
        }
    }

    fn publish_coach(&mut self, event: CoachEvent) {
        self.coach_ledger
            .entry(event.coach_id().clone())
            .or_default()
            .record(&event);
        self.queue_event(EventScope::default(), RuntimeEvent::Coach { event });
    }

    fn forward_viewport_events(&mut self) {
        let mut ownership_failure = None;
        for event in self.pilot.take_viewport_events() {
            if event.state == ViewportLifecycle::Closed {
                self.closed_viewports.push_back(event.viewport.clone());
                while self.closed_viewports.len() > 64 {
                    self.closed_viewports.pop_front();
                }
                if let Some(egui_viewport) = self.viewport_egui_history.remove(&event.viewport.id) {
                    if self.input_state.owns_viewport(egui_viewport) {
                        self.input_state.discard_viewport(egui_viewport);
                        ownership_failure = Some(PilotError {
                            code: "input-owner-viewport-closed".into(),
                            message: format!(
                                "viewport `{}` closed while it owned pressed input; this process cannot be reused",
                                event.viewport.id
                            ),
                            context: None,
                        });
                    }
                }
            } else if event.kind != ViewportEventKind::Closed {
                if let Some(egui_viewport) = self.pilot.egui_viewport(&event.viewport.id) {
                    self.viewport_egui_history
                        .insert(event.viewport.id.clone(), egui_viewport);
                }
            }
            self.queue_event(
                EventScope {
                    viewport_id: Some(event.viewport.id.clone()),
                    ..Default::default()
                },
                RuntimeEvent::ViewportLifecycle { event },
            );
        }
        if let Some(error) = ownership_failure {
            self.poisoned = Some(error.clone());
            self.pilot.clear_input();
            self.action = None;
            if let Some(active) = self.active_flow.take() {
                self.fail_flow(active, error);
            } else {
                self.queue_event(EventScope::default(), RuntimeEvent::Diagnostic { error });
            }
        }
    }

    fn start_flow(&mut self, request_id: RequestId, run_id: RunId, flow: Flow) {
        if let Some(error) = &self.poisoned {
            self.queue_error(
                request_id,
                "runtime-poisoned",
                format!(
                    "target process cannot be reused after infrastructure failure: {}",
                    error.message
                ),
                error.context.as_deref().cloned(),
            );
            return;
        }
        let unsupported = protocol::unsupported_capabilities(&flow);
        if !unsupported.is_empty() {
            self.queue_error(
                request_id,
                "unsupported-capability",
                format!("flow requires unsupported capabilities: {unsupported:?}"),
                None,
            );
            return;
        }
        if self.active_flow.is_some() || self.action.is_some() {
            self.queue_error(
                request_id,
                "busy",
                "another flow is already active".into(),
                self.failure_context(None),
            );
            return;
        }
        self.queue_event(
            EventScope {
                request_id: Some(request_id.clone()),
                run_id: Some(run_id.clone()),
                ..Default::default()
            },
            RuntimeEvent::RunStarted {
                scenario: flow.id.clone(),
            },
        );
        self.active_flow = Some(ActiveFlow {
            request_id,
            run_id,
            flow,
            index: 0,
            wait_started: None,
            step_started: false,
            outcomes: Vec::new(),
        });
        self.ctx.request_repaint();
    }

    fn advance_flow(&mut self) {
        if self.action.is_some() || self.pending_capture.is_some() {
            return;
        }
        let Some(mut active) = self.active_flow.take() else {
            return;
        };
        let Some(step) = active.flow.steps.get(active.index).cloned() else {
            self.finish_run(active, RunOutcome::Passed, None);
            return;
        };
        let step_id = active.flow.step_id(active.index);
        if !active.step_started {
            self.queue_event(
                EventScope {
                    request_id: Some(active.request_id.clone()),
                    run_id: Some(active.run_id.clone()),
                    step_id: Some(step_id.clone()),
                    ..Default::default()
                },
                RuntimeEvent::StepStarted {
                    index: active.index,
                },
            );
            active.step_started = true;
        }
        match step {
            Step::Action { action } => {
                match self.prepare_action(
                    active.request_id.clone(),
                    active.run_id.clone(),
                    step_id,
                    active.index,
                    action,
                ) {
                    Ok(Some(execution)) => {
                        if let Some(settlement) = &execution.settlement {
                            self.pilot
                                .request_parent_wake_after_next_pass(settlement.egui_viewport);
                            self.request_viewport_repaint(
                                settlement.egui_viewport,
                                &settlement.viewport,
                            );
                        }
                        self.action = Some(execution);
                        self.active_flow = Some(active);
                        self.ctx.request_repaint();
                    }
                    Ok(None) => {
                        self.active_flow = Some(active);
                        self.ctx.request_repaint();
                    }
                    Err(error) => self.fail_flow(active, error),
                }
            }
            Step::Coach { coach } => {
                let started = Instant::now();
                self.forward_coach_events();
                let events = self.apply_coach(coach);
                let refusal = events.iter().find_map(|event| match event {
                    CoachEvent::Refused {
                        coach_id,
                        reason,
                        detail,
                    } => Some(PilotError {
                        code: "coach-refused".into(),
                        message: format!("touchpoint `{coach_id}` refused: {reason:?} — {detail}"),
                        context: None,
                    }),
                    _ => None,
                });
                for event in events {
                    self.publish_coach(event);
                }
                match refusal {
                    Some(error) => self.fail_flow(active, error),
                    None => {
                        let outcome = StepOutcome {
                            id: step_id,
                            index: active.index,
                            passed: true,
                            elapsed_ms: started.elapsed().as_millis() as u64,
                            message: None,
                            context: None,
                        };
                        self.complete_step(&mut active, outcome);
                        self.active_flow = Some(active);
                        self.ctx.request_repaint();
                    }
                }
            }
            Step::Expect { expectation } => {
                let started = *active.wait_started.get_or_insert_with(Instant::now);
                match self.evaluate_expectation(&expectation) {
                    Ok(()) => {
                        let outcome = StepOutcome {
                            id: step_id,
                            index: active.index,
                            passed: true,
                            elapsed_ms: started.elapsed().as_millis() as u64,
                            message: None,
                            context: None,
                        };
                        self.complete_step(&mut active, outcome);
                        self.active_flow = Some(active);
                        self.ctx.request_repaint();
                    }
                    Err(_) if started.elapsed() < Duration::from_secs(5) => {
                        self.active_flow = Some(active);
                        self.ctx.request_repaint_after(Duration::from_millis(1));
                    }
                    Err(error) => self.fail_flow(active, error),
                }
            }
            Step::Wait { milliseconds } => {
                let started = active.wait_started.get_or_insert_with(Instant::now);
                if started.elapsed() >= Duration::from_millis(milliseconds) {
                    let elapsed_ms = started.elapsed().as_millis() as u64;
                    let outcome = StepOutcome {
                        id: step_id,
                        index: active.index,
                        passed: true,
                        elapsed_ms,
                        message: None,
                        context: None,
                    };
                    self.complete_step(&mut active, outcome);
                }
                self.active_flow = Some(active);
                self.ctx.request_repaint_after(Duration::from_millis(1));
            }
            Step::Capture { name, viewport } => {
                let index = active.index;
                let request_id = active.request_id.clone();
                let run_id = active.run_id.clone();
                self.active_flow = Some(active);
                self.start_capture(
                    request_id,
                    name,
                    viewport,
                    Some(index),
                    Some(run_id),
                    Some(step_id),
                );
            }
        }
    }

    fn prepare_action(
        &mut self,
        request_id: RequestId,
        run_id: RunId,
        step_id: StepId,
        step_index: usize,
        action: Action,
    ) -> Result<Option<ActionExecution>, PilotError> {
        let frame = self.pilot.latest_frame();
        let target = primary_target(&action)
            .map(|locator| resolve_locator(&frame, locator))
            .transpose()?;
        let input_viewport = if let Some(node) = target {
            let descriptor = frame
                .viewports
                .iter()
                .find(|viewport| viewport.id == node.viewport)
                .cloned()
                .ok_or_else(|| {
                    viewport_error(
                        "viewport-not-found",
                        format!("viewport `{}` is no longer live", node.viewport),
                    )
                })?;
            self.pilot
                .egui_viewport(&descriptor.id)
                .map(|egui_viewport| (descriptor, egui_viewport))
        } else {
            match &action {
                Action::Press { viewport, .. } | Action::Scroll { viewport, .. } => {
                    Some(resolve_input_viewport(&self.pilot, viewport.as_ref())?)
                }
                Action::FocusViewport { .. }
                | Action::MinimizeViewport { .. }
                | Action::RestoreViewport { .. }
                | Action::CloseViewport { .. }
                | Action::Click { .. }
                | Action::DoubleClick { .. }
                | Action::Hover { .. }
                | Action::Fill { .. }
                | Action::Clear { .. }
                | Action::Type { .. }
                | Action::DragTo { .. }
                | Action::ScrollIntoView { .. }
                | Action::SetValue { .. } => None,
            }
        };
        if let Some((descriptor, egui_viewport)) = input_viewport {
            require_native_viewport(&descriptor)?;
            if !descriptor.focused {
                self.ctx
                    .send_viewport_cmd_to(egui_viewport, egui::ViewportCommand::Focus);
                self.request_viewport_repaint(egui_viewport, &descriptor);
            }
        }
        if let Some(node) = target {
            assert_actionable(node, &action)?;
            let needs_stability_frame = {
                let stable = self.stability.entry(node.id.clone()).or_insert(Stability {
                    rect: node.rect,
                    revision: frame.revision,
                    frames: 1,
                });
                if stable.revision != frame.revision {
                    if stable.rect == node.rect {
                        stable.frames = stable.frames.saturating_add(1);
                    } else {
                        stable.rect = node.rect;
                        stable.frames = 1;
                    }
                    stable.revision = frame.revision;
                }
                stable.frames < 2
            };
            if needs_stability_frame {
                if let Some(egui_viewport) = self.pilot.egui_viewport(&node.viewport) {
                    self.pilot
                        .request_parent_wake_after_next_pass(egui_viewport);
                    self.ctx.request_repaint_of(egui_viewport);
                } else {
                    self.ctx.request_repaint();
                }
                return Ok(None);
            }
        }
        let wait_viewport = match &action {
            Action::FocusViewport { viewport } => {
                let (descriptor, egui_viewport) = self.resolve_viewport(viewport)?;
                require_native_viewport(&descriptor)?;
                self.ctx
                    .send_viewport_cmd_to(egui_viewport, egui::ViewportCommand::Focus);
                self.pilot
                    .queue_viewport_command(egui_viewport, egui::ViewportCommand::Focus);
                self.request_viewport_repaint(egui_viewport, &descriptor);
                Some((viewport.clone(), ViewportExpectationState::Focused))
            }
            Action::MinimizeViewport { viewport } => {
                let (descriptor, egui_viewport) = self.resolve_viewport(viewport)?;
                require_native_viewport(&descriptor)?;
                self.ctx
                    .send_viewport_cmd_to(egui_viewport, egui::ViewportCommand::Minimized(true));
                self.pilot
                    .queue_viewport_command(egui_viewport, egui::ViewportCommand::Minimized(true));
                self.request_viewport_repaint(egui_viewport, &descriptor);
                Some((viewport.clone(), ViewportExpectationState::Minimized))
            }
            Action::RestoreViewport { viewport } => {
                let (descriptor, egui_viewport) = self.resolve_viewport(viewport)?;
                require_native_viewport(&descriptor)?;
                self.ctx
                    .send_viewport_cmd_to(egui_viewport, egui::ViewportCommand::Visible(true));
                self.ctx
                    .send_viewport_cmd_to(egui_viewport, egui::ViewportCommand::Minimized(false));
                self.pilot
                    .queue_viewport_command(egui_viewport, egui::ViewportCommand::Visible(true));
                self.pilot
                    .queue_viewport_command(egui_viewport, egui::ViewportCommand::Minimized(false));
                self.request_viewport_repaint(egui_viewport, &descriptor);
                Some((viewport.clone(), ViewportExpectationState::Restored))
            }
            Action::CloseViewport { viewport } => {
                let (descriptor, egui_viewport) = self.resolve_viewport(viewport)?;
                require_native_viewport(&descriptor)?;
                if descriptor.class == protocol::ViewportClass::Root {
                    return Err(PilotError {
                        code: "viewport-root-close".into(),
                        message: "close the root viewport with pilot_stop".into(),
                        context: None,
                    });
                }
                if descriptor.class == protocol::ViewportClass::Deferred {
                    self.pilot.request_viewport_close(egui_viewport);
                    self.ctx
                        .send_viewport_cmd_to(egui_viewport, egui::ViewportCommand::Close);
                } else {
                    self.ctx
                        .send_viewport_cmd_to(egui_viewport, egui::ViewportCommand::Close);
                    self.pilot
                        .queue_viewport_command(egui_viewport, egui::ViewportCommand::Close);
                }
                self.request_viewport_repaint(egui_viewport, &descriptor);
                Some((viewport.clone(), ViewportExpectationState::Closed))
            }
            _ => None,
        };
        let batches = if wait_viewport.is_some() {
            Vec::new()
        } else {
            action_batches(&self.pilot, &frame, &action)?
        };
        let settlement = batches
            .first()
            .map(|batch| {
                let viewport = self
                    .pilot
                    .viewports()
                    .into_iter()
                    .find(|viewport| self.pilot.egui_viewport(&viewport.id) == Some(batch.viewport))
                    .ok_or_else(|| {
                        viewport_error(
                            "viewport-not-found",
                            "input target viewport is no longer live",
                        )
                    })?;
                Ok(ActionSettlement {
                    viewport,
                    egui_viewport: batch.viewport,
                    required_input_generation: 0,
                    last_viewport_revision: self
                        .pilot
                        .viewport_semantic_revision(batch.viewport)
                        .unwrap_or_default(),
                    last_semantic_hash: None,
                    last_geometry_hash: None,
                    stable_frames: 0,
                })
            })
            .transpose()?;
        Ok(Some(ActionExecution {
            request_id,
            run_id,
            step_id,
            step_index,
            started: Instant::now(),
            sequence: InputSequence {
                ticks: batches.into(),
            },
            wait_viewport,
            settlement,
        }))
    }

    fn advance_action(&mut self) {
        let Some(mut action) = self.action.take() else {
            return;
        };
        if self.pilot.pending_input() > 0 {
            if let Some(settlement) = &action.settlement {
                self.pilot
                    .request_parent_wake_after_next_pass(settlement.egui_viewport);
                self.request_viewport_repaint(settlement.egui_viewport, &settlement.viewport);
            } else {
                for viewport in self.pilot.pending_input_viewports() {
                    self.pilot.request_parent_wake_after_next_pass(viewport);
                    self.ctx.request_repaint_of(viewport);
                }
            }
            self.action = Some(action);
            return;
        }
        if let Some((selector, state)) = &action.wait_viewport {
            if self.viewport_state_matches(selector, *state) {
                action.wait_viewport = None;
            } else if action.started.elapsed() >= Duration::from_secs(5) {
                let error = PilotError {
                    code: "viewport-action-timeout".into(),
                    message: format!("viewport did not reach {state:?} after action: {selector:?}"),
                    context: self.failure_context(None).map(Box::new),
                };
                if let Some(active) = self.active_flow.take() {
                    self.fail_flow(active, error);
                }
                return;
            } else {
                self.action = Some(action);
                self.ctx.request_repaint_after(Duration::from_millis(1));
                return;
            }
        }
        if let Some(batch) = action.sequence.ticks.pop_front() {
            if let Err(error) = self.input_state.apply(&batch) {
                if let Some(active) = self.active_flow.take() {
                    self.fail_flow(active, error);
                }
                return;
            }
            let viewport = batch.viewport;
            let final_batch = action.sequence.ticks.is_empty();
            let input_generation = self.pilot.queue_input(batch);
            if final_batch {
                if let Some(settlement) = &mut action.settlement {
                    settlement.required_input_generation = input_generation;
                    settlement.last_viewport_revision = self
                        .pilot
                        .viewport_semantic_revision(viewport)
                        .unwrap_or_default();
                    settlement.last_semantic_hash = None;
                    settlement.last_geometry_hash = None;
                    settlement.stable_frames = 0;
                }
            }
            // Wake both the dormant target and its owning coordinator so a
            // deferred native viewport is re-declared, then keep the
            // post-pass parent wake armed so completion cannot be coalesced.
            self.pilot.request_parent_wake_after_next_pass(viewport);
            if let Some(settlement) = &action.settlement {
                self.request_viewport_repaint(viewport, &settlement.viewport);
            } else {
                self.ctx.request_repaint_of(viewport);
            }
            self.action = Some(action);
            return;
        }
        if let Some(settlement) = &mut action.settlement {
            if self
                .pilot
                .viewport_committed_input_generation(settlement.egui_viewport)
                < settlement.required_input_generation
            {
                self.pilot
                    .request_parent_wake_after_next_pass(settlement.egui_viewport);
                self.request_viewport_repaint(settlement.egui_viewport, &settlement.viewport);
                self.action = Some(action);
                return;
            }
            let viewport_is_live =
                self.pilot.egui_viewport(&settlement.viewport.id) == Some(settlement.egui_viewport);
            let focus_transferred =
                viewport_focus_transferred(&settlement.viewport, &self.pilot.latest_frame());
            // Controls such as "Show Main" intentionally move native focus
            // away from their source viewport. Once the final input pass is
            // committed, that observed focus transfer is the terminal state:
            // the backgrounded source may not receive two more render passes.
            if viewport_is_live && !focus_transferred {
                let viewport_revision = self
                    .pilot
                    .viewport_semantic_revision(settlement.egui_viewport)
                    .unwrap_or_default();
                if viewport_revision <= settlement.last_viewport_revision {
                    self.pilot
                        .request_parent_wake_after_next_pass(settlement.egui_viewport);
                    self.request_viewport_repaint(settlement.egui_viewport, &settlement.viewport);
                    self.action = Some(action);
                    return;
                }

                let frame = self.pilot.latest_frame();
                let semantic = semantic_hash(&frame, &settlement.viewport.id);
                let geometry = geometry_hash(&frame, &settlement.viewport.id);
                if settlement.last_semantic_hash.as_ref() == Some(&semantic)
                    && settlement.last_geometry_hash.as_ref() == Some(&geometry)
                {
                    settlement.stable_frames = settlement.stable_frames.saturating_add(1);
                } else {
                    settlement.last_semantic_hash = Some(semantic);
                    settlement.last_geometry_hash = Some(geometry);
                    settlement.stable_frames = 1;
                }
                settlement.last_viewport_revision = viewport_revision;

                if settlement.stable_frames < 2 {
                    self.pilot
                        .request_parent_wake_after_next_pass(settlement.egui_viewport);
                    self.request_viewport_repaint(settlement.egui_viewport, &settlement.viewport);
                    self.action = Some(action);
                    return;
                }
            }
        }
        let outcome = StepOutcome {
            id: action.step_id,
            index: action.step_index,
            passed: true,
            elapsed_ms: action.started.elapsed().as_millis() as u64,
            message: None,
            context: None,
        };
        if let Some(mut active) = self.active_flow.take() {
            debug_assert_eq!(active.request_id, action.request_id);
            debug_assert_eq!(active.run_id, action.run_id);
            self.complete_step(&mut active, outcome);
            self.active_flow = Some(active);
        }
        self.ctx.request_repaint();
    }

    fn start_capture(
        &mut self,
        request_id: RequestId,
        name: String,
        viewport: Option<ViewportSelector>,
        flow_step: Option<usize>,
        run_id: Option<RunId>,
        step_id: Option<StepId>,
    ) {
        if let Some(error) = &self.poisoned {
            self.fail_capture_start(
                request_id,
                flow_step,
                PilotError {
                    code: "runtime-poisoned".into(),
                    message: format!(
                        "target process cannot be reused after infrastructure failure: {}",
                        error.message
                    ),
                    context: error.context.clone(),
                },
            );
            return;
        }
        if self.pending_capture.is_some() || self.capture_job.is_some() {
            self.fail_capture_start(
                request_id,
                flow_step,
                PilotError {
                    code: "capture-busy".into(),
                    message: "another capture is already active".into(),
                    context: None,
                },
            );
            return;
        }
        let target = match viewport.as_ref() {
            Some(selector) => self.resolve_viewport(selector),
            None => self
                .pilot
                .only_viewport()
                .map_err(|message| viewport_error("viewport-required", message)),
        };
        let (viewport, egui_viewport) = match target {
            Ok(target) => target,
            Err(error) => {
                self.fail_capture_start(request_id, flow_step, error);
                return;
            }
        };
        if let Err(error) = require_native_viewport(&viewport) {
            self.fail_capture_start(request_id, flow_step, error);
            return;
        }
        if let Some(mut error) = viewport_capture_support_error(&viewport) {
            error.context = self.failure_context(None).map(Box::new);
            self.fail_capture_start(request_id, flow_step, error);
            return;
        }
        if let Err(motions) = self
            .pilot
            .prepare_motion(&self.ctx, MotionPolicy::Suppressed)
        {
            let error = PilotError {
                code: "uncontrolled-motion".into(),
                message: format!("capture blocked by uncontrolled motion: {motions:?}"),
                context: self.failure_context(None).map(Box::new),
            };
            self.fail_capture_start(request_id, flow_step, error);
            return;
        }
        self.pending_capture = Some(PendingCapture {
            request_id,
            run_id,
            step_id,
            name,
            viewport: viewport.clone(),
            egui_viewport,
            flow_step,
            last_semantic_hash: None,
            last_geometry_hash: None,
            last_pixel_hash: None,
            attempts: 0,
            started_at: Instant::now(),
            last_request_at: Instant::now(),
        });
        self.pilot.clear_captures_for(egui_viewport);
        self.pilot.queue_viewport_command(
            egui_viewport,
            egui::ViewportCommand::Screenshot(egui::UserData::default()),
        );
        self.request_viewport_repaint(egui_viewport, &viewport);
    }

    fn fail_capture_start(
        &mut self,
        request_id: RequestId,
        flow_step: Option<usize>,
        error: PilotError,
    ) {
        if flow_step.is_some() {
            if let Some(active) = self.active_flow.take() {
                self.fail_flow(active, error);
            }
        } else {
            self.queue_error(
                request_id,
                &error.code,
                error.message,
                error.context.map(|context| *context),
            );
        }
    }

    fn accept_captured_image(&mut self) {
        if self.capture_job.is_some() {
            return;
        }
        let target_viewport = self
            .pending_capture
            .as_ref()
            .map(|pending| pending.egui_viewport);
        let Some(capture) =
            target_viewport.and_then(|viewport| self.pilot.take_capture_for(viewport))
        else {
            let Some(pending) = self.pending_capture.as_mut() else {
                return;
            };
            if pending.started_at.elapsed() >= Duration::from_secs(10) {
                let pending = self.pending_capture.take().expect("capture metadata");
                self.fail_capture_start(
                    pending.request_id,
                    pending.flow_step,
                    PilotError {
                        code: "capture-response-timeout".into(),
                        message: format!(
                            "renderer did not return a screenshot after {} settle attempts; {} viewport commands remain queued",
                            pending.attempts,
                            self.pilot.pending_viewport_commands()
                        ),
                        context: None,
                    },
                );
                return;
            }
            if pending.last_request_at.elapsed() >= Duration::from_millis(250) {
                self.pilot.queue_viewport_command(
                    pending.egui_viewport,
                    egui::ViewportCommand::Screenshot(egui::UserData::default()),
                );
                pending.last_request_at = Instant::now();
            }
            self.ctx.request_repaint_after(Duration::from_millis(16));
            return;
        };
        let Some(mut pending) = self.pending_capture.take() else {
            return;
        };
        let frame = self.pilot.latest_frame();
        let semantic_hash = semantic_hash(&frame, &pending.viewport.id);
        let geometry_hash = geometry_hash(&frame, &pending.viewport.id);
        let pixel_hash = pixel_hash(capture.image.as_ref());
        let stable = pending.last_semantic_hash.as_ref() == Some(&semantic_hash)
            && pending.last_geometry_hash.as_ref() == Some(&geometry_hash)
            && pending.last_pixel_hash.as_ref() == Some(&pixel_hash);
        if !stable {
            pending.last_semantic_hash = Some(semantic_hash);
            pending.last_geometry_hash = Some(geometry_hash);
            pending.last_pixel_hash = Some(pixel_hash);
            pending.attempts = pending.attempts.saturating_add(1);
            self.queue_telemetry(
                EventScope {
                    request_id: Some(pending.request_id.clone()),
                    run_id: pending.run_id.clone(),
                    step_id: pending.step_id.clone(),
                    ..Default::default()
                },
                RuntimeEvent::Telemetry {
                    channel: "capture-settle-attempt".into(),
                    revision: u64::from(pending.attempts),
                },
            );
            if pending.attempts >= 30 {
                let error = PilotError {
                    code: "unstable-capture".into(),
                    message: "semantic, geometry, and pixel hashes did not settle within 30 frames"
                        .into(),
                    context: self.failure_context(None).map(Box::new),
                };
                self.fail_capture_start(pending.request_id, pending.flow_step, error);
                return;
            }
            self.pending_capture = Some(pending);
            self.pilot.queue_viewport_command(
                capture.viewport,
                egui::ViewportCommand::Screenshot(egui::UserData::default()),
            );
            self.ctx.request_repaint_of(capture.viewport);
            return;
        }
        let root = self.config.artifact_root.clone();
        let name = pending.name.clone();
        let renderer_key = self
            .config
            .renderer_identity
            .as_ref()
            .map(protocol::RendererIdentity::baseline_key)
            .unwrap_or_else(|| "renderer-unreported".into());
        let viewport = pending.viewport.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("eguipilotai-capture".into())
            .spawn(move || {
                let result = write_capture(
                    &root,
                    &name,
                    &renderer_key,
                    viewport,
                    capture.image.as_ref(),
                );
                let _ = tx.send(result);
            })
            .expect("EguiPilotAI capture worker");
        self.pending_capture = Some(pending);
        self.capture_job = Some(rx);
        // Keep the UI loop alive while the encoder works. A native egui app can
        // otherwise go completely idle after the stable screenshot frame,
        // leaving the completed worker result unobserved until runner timeout.
        self.ctx.request_repaint_after(Duration::from_millis(1));
    }

    fn poll_capture_job(&mut self) {
        let Some(job) = &self.capture_job else {
            return;
        };
        match job.try_recv() {
            Ok(result) => {
                self.capture_job = None;
                let pending = self.pending_capture.take().expect("capture metadata");
                match result {
                    Ok(capture) => {
                        self.queue_event(
                            EventScope {
                                request_id: Some(pending.request_id.clone()),
                                run_id: pending.run_id.clone(),
                                step_id: pending.step_id.clone(),
                                ..Default::default()
                            },
                            RuntimeEvent::AttachmentCommitted {
                                capture: capture.clone(),
                            },
                        );
                        if let Some(index) = pending.flow_step {
                            if let Some(mut active) = self.active_flow.take() {
                                let outcome = StepOutcome {
                                    id: pending
                                        .step_id
                                        .unwrap_or_else(|| active.flow.step_id(index)),
                                    index,
                                    passed: true,
                                    elapsed_ms: 0,
                                    message: Some("capture stabilized and written".into()),
                                    context: None,
                                };
                                self.complete_step(&mut active, outcome);
                                self.active_flow = Some(active);
                            }
                        } else {
                            self.queue_success(
                                pending.request_id,
                                CommandResult::Capture { capture },
                            );
                        }
                    }
                    Err(error) => {
                        self.fail_capture_start(pending.request_id, pending.flow_step, error)
                    }
                }
                self.ctx.request_repaint();
            }
            Err(TryRecvError::Empty) => {
                self.ctx.request_repaint_after(Duration::from_millis(1));
            }
            Err(TryRecvError::Disconnected) => {
                self.capture_job = None;
                let pending = self.pending_capture.take().expect("capture metadata");
                self.fail_capture_start(
                    pending.request_id,
                    pending.flow_step,
                    PilotError {
                        code: "capture-worker".into(),
                        message: "capture worker disconnected".into(),
                        context: None,
                    },
                );
            }
        }
    }

    fn complete_step(&mut self, active: &mut ActiveFlow, outcome: StepOutcome) {
        self.queue_event(
            EventScope {
                request_id: Some(active.request_id.clone()),
                run_id: Some(active.run_id.clone()),
                step_id: Some(outcome.id.clone()),
                ..Default::default()
            },
            RuntimeEvent::StepFinished {
                outcome: outcome.clone(),
            },
        );
        active.outcomes.push(outcome);
        active.index += 1;
        active.wait_started = None;
        active.step_started = false;
    }

    fn fail_flow(&mut self, mut active: ActiveFlow, mut error: PilotError) {
        if error.context.is_none() {
            error.context = self.failure_context(None).map(Box::new);
        }
        let outcome = StepOutcome {
            id: active.flow.step_id(active.index),
            index: active.index,
            passed: false,
            elapsed_ms: 0,
            message: Some(error.message.clone()),
            context: error.context.clone(),
        };
        self.complete_step(&mut active, outcome);
        self.action = None;
        self.pending_capture = None;
        self.capture_job = None;
        self.cleanup_input(Some(EventScope {
            request_id: Some(active.request_id.clone()),
            run_id: Some(active.run_id.clone()),
            ..Default::default()
        }));
        self.finish_run(active, RunOutcome::Failed, Some(error));
    }

    fn finish_run(&mut self, active: ActiveFlow, outcome: RunOutcome, error: Option<PilotError>) {
        let result = RunResult {
            run_id: active.run_id,
            scenario: active.flow.id,
            outcome,
            steps: active.outcomes,
            error,
        };
        self.queue_success(
            active.request_id,
            CommandResult::Run {
                result: result.clone(),
            },
        );
        self.terminal_runs.push_back(result);
        while self.terminal_runs.len() > 32 {
            self.terminal_runs.pop_front();
        }
    }

    fn cancel(&mut self, request_id: RequestId, run_id: RunId) {
        let state = if self.active_flow.as_ref().map(|run| &run.run_id) == Some(&run_id) {
            let active = self.active_flow.take().expect("matched active run");
            self.action = None;
            self.pending_capture = None;
            self.capture_job = None;
            self.cleanup_input(Some(EventScope {
                request_id: Some(active.request_id.clone()),
                run_id: Some(run_id.clone()),
                ..Default::default()
            }));
            self.pilot.unmount_all_coach(CoachUnmountReason::Agent);
            self.forward_coach_events();
            self.finish_run(active, RunOutcome::Cancelled, None);
            CancellationState::Requested
        } else if self.terminal_runs.iter().any(|run| run.run_id == run_id) {
            CancellationState::AlreadyTerminal
        } else {
            CancellationState::NotFound
        };
        self.queue_success(
            request_id,
            CommandResult::Cancel {
                cancellation: CancellationResult { run_id, state },
            },
        );
    }

    fn failure_context(&self, node: Option<&NodeId>) -> Option<AgentSnapshot> {
        let request = SnapshotRequest {
            filter: protocol::SnapshotFilter {
                subtree: node.cloned(),
                ..Default::default()
            },
            max_nodes: Some(40),
            max_bytes: Some(8 * 1024),
            ..SnapshotRequest::default()
        };
        self.pilot.snapshot(&request).ok()
    }

    fn resolve_viewport(
        &self,
        selector: &ViewportSelector,
    ) -> Result<(ViewportDescriptor, egui::ViewportId), PilotError> {
        self.pilot
            .resolve_viewport(selector)
            .map_err(|message| viewport_error("viewport-not-found", message))
    }

    fn request_viewport_repaint(
        &self,
        egui_viewport: egui::ViewportId,
        descriptor: &ViewportDescriptor,
    ) {
        self.ctx.request_repaint_of(egui_viewport);
        self.ctx
            .request_repaint_after_for(Duration::from_millis(1), egui_viewport);
        if descriptor.class != protocol::ViewportClass::Root {
            let parent = descriptor
                .parent
                .as_ref()
                .and_then(|parent| self.pilot.egui_viewport(parent))
                .unwrap_or(egui::ViewportId::ROOT);
            self.ctx.request_repaint_of(parent);
            self.ctx
                .request_repaint_after_for(Duration::from_millis(1), parent);
        }
    }

    fn viewport_state_matches(
        &self,
        selector: &ViewportSelector,
        state: ViewportExpectationState,
    ) -> bool {
        if state == ViewportExpectationState::Closed {
            return self.closed_viewports.iter().any(|viewport| {
                selector.id.as_ref().is_none_or(|id| id == &viewport.id)
                    && selector.key.as_ref().is_none_or(|key| key == &viewport.key)
            });
        }
        let Ok((viewport, _)) = self.pilot.resolve_viewport(selector) else {
            return false;
        };
        match state {
            ViewportExpectationState::Open => viewport.lifecycle == ViewportLifecycle::Open,
            ViewportExpectationState::Focused => viewport.focused,
            ViewportExpectationState::Minimized => viewport.minimized,
            ViewportExpectationState::Restored => viewport.visible && !viewport.minimized,
            ViewportExpectationState::CloseRequested => viewport.close_requested,
            ViewportExpectationState::Closed => false,
        }
    }

    fn evaluate_expectation(&self, expectation: &Expectation) -> Result<(), PilotError> {
        if let Expectation::Coach { coach_id, state } = expectation {
            return self
                .coach_ledger
                .get(coach_id)
                .is_some_and(|seen| seen.matches(*state))
                .then_some(())
                .ok_or_else(|| PilotError {
                    code: "coach-expectation-failed".into(),
                    message: format!("touchpoint `{coach_id}` has not been {state:?}"),
                    context: None,
                });
        }
        if let Expectation::Viewport { viewport, state } = expectation {
            return self
                .viewport_state_matches(viewport, *state)
                .then_some(())
                .ok_or_else(|| PilotError {
                    code: "viewport-expectation-failed".into(),
                    message: format!("viewport {viewport:?} is not {state:?}"),
                    context: None,
                });
        }
        evaluate_expectation(&self.pilot.latest_frame(), expectation)
    }

    fn stop(&mut self, request_id: RequestId) {
        if let Some(active) = self.active_flow.take() {
            self.action = None;
            self.pending_capture = None;
            self.capture_job = None;
            self.cleanup_input(Some(EventScope {
                request_id: Some(active.request_id.clone()),
                run_id: Some(active.run_id.clone()),
                ..Default::default()
            }));
            self.finish_run(active, RunOutcome::Cancelled, None);
        } else {
            self.cleanup_input(None);
        }
        // A coached spotlight must never outlive the session that mounted it.
        self.pilot
            .unmount_all_coach(CoachUnmountReason::SessionEnded);
        self.forward_coach_events();
        self.stopping = true;
        self.stop_request = Some(request_id);
        self.stop_started = Some(Instant::now());
        self.stop_close_requested.clear();
        self.stop_response_queued = false;
        self.advance_stop();
    }

    fn advance_stop(&mut self) {
        if !self.stopping || self.stop_response_queued {
            return;
        }
        if self.pilot.pending_input() > 0 || !self.input_state.is_clear() {
            self.ctx.request_repaint_after(Duration::from_millis(1));
            return;
        }

        let children: Vec<_> = self
            .pilot
            .viewports()
            .into_iter()
            .filter(|viewport| viewport.class != protocol::ViewportClass::Root)
            .filter_map(|viewport| {
                self.pilot
                    .egui_viewport(&viewport.id)
                    .map(|egui_id| (viewport, egui_id))
            })
            .collect();
        if !children.is_empty() {
            for (descriptor, egui_id) in children {
                if self.stop_close_requested.insert(egui_id) {
                    if descriptor.class == protocol::ViewportClass::Deferred {
                        self.pilot.request_viewport_close(egui_id);
                    } else {
                        self.pilot
                            .queue_viewport_command(egui_id, egui::ViewportCommand::Close);
                    }
                    self.request_viewport_repaint(egui_id, &descriptor);
                }
            }
            if self
                .stop_started
                .is_some_and(|started| started.elapsed() >= Duration::from_secs(5))
            {
                if let Some(request_id) = self.stop_request.take() {
                    self.queue_error(
                        request_id,
                        "viewport-shutdown-timeout",
                        "child viewports did not close within 5 seconds; forcing process shutdown"
                            .into(),
                        self.failure_context(None),
                    );
                }
                self.stop_response_queued = true;
            } else {
                self.ctx.request_repaint_after(Duration::from_millis(16));
            }
            return;
        }

        if let Some(request_id) = self.stop_request.take() {
            self.queue_success(request_id, CommandResult::Stopped);
        }
        self.stop_response_queued = true;
    }

    fn finish_stop_if_flushed(&mut self) {
        if self.stopping
            && self.stop_response_queued
            && self.outbound.is_empty()
            && self.telemetry.is_none()
            && self.pilot.pending_input() == 0
            && self.input_state.is_clear()
        {
            let _ = self.transport.close();
            self.stopped = true;
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.stopping {
            self.ctx.request_repaint_after(Duration::from_millis(1));
        }
    }

    fn cleanup_input(&mut self, scope: Option<EventScope>) {
        self.pilot.clear_input();
        let cleanup = self.input_state.release_all();
        for input in cleanup {
            let viewport = input.viewport;
            self.pilot.queue_input(input);
            self.ctx.request_repaint_of(viewport);
        }
        self.queue_event(scope.unwrap_or_default(), RuntimeEvent::InputCleanup);
    }

    fn queue_error(
        &mut self,
        request_id: RequestId,
        code: &str,
        message: String,
        context: Option<AgentSnapshot>,
    ) {
        self.outbound
            .push_back(ServerMessage::response(ResponseEnvelope {
                request_id,
                session: self.config.session.clone(),
                outcome: ResponseOutcome::Error {
                    error: PilotError {
                        code: code.into(),
                        message,
                        context: context.map(Box::new),
                    },
                },
            }));
    }

    fn queue_success(&mut self, request_id: RequestId, result: CommandResult) {
        self.outbound
            .push_back(ServerMessage::response(ResponseEnvelope {
                request_id,
                session: self.config.session.clone(),
                outcome: ResponseOutcome::Success {
                    result: Box::new(result),
                },
            }));
    }

    fn queue_event(&mut self, scope: EventScope, event: RuntimeEvent) {
        self.event_sequence = self.event_sequence.saturating_add(1);
        self.outbound
            .push_back(ServerMessage::event(RuntimeEventEnvelope {
                event_id: EventId::from_sequence(self.event_sequence),
                session: self.config.session.clone(),
                scope,
                event,
            }));
    }

    /// Queue opt-in high-rate telemetry. Only the newest sample is retained;
    /// pressure is reported as a required event before that sample is sent.
    pub fn queue_telemetry(&mut self, scope: EventScope, event: RuntimeEvent) {
        if self.telemetry.replace((scope, event)).is_some() {
            self.dropped_telemetry = self.dropped_telemetry.saturating_add(1);
        }
    }

    fn flush_outbound(&mut self) {
        loop {
            while let Some(message) = self.outbound.pop_front() {
                match self.transport.try_send(message.clone()) {
                    Ok(()) => {}
                    Err(TransportError::Backpressure { .. }) => {
                        self.outbound.push_front(message);
                        return;
                    }
                    Err(_) => {
                        self.stopped = true;
                        return;
                    }
                }
            }
            if self.dropped_telemetry > 0 {
                let dropped = std::mem::take(&mut self.dropped_telemetry);
                self.queue_event(
                    EventScope::default(),
                    RuntimeEvent::QueuePressure {
                        queue: "telemetry".into(),
                        dropped,
                    },
                );
                continue;
            }
            if let Some((scope, event)) = self.telemetry.take() {
                self.queue_event(scope, event);
                continue;
            }
            break;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn process_identity() -> u32 {
    std::process::id()
}

#[cfg(target_arch = "wasm32")]
fn process_identity() -> u32 {
    0
}

impl<T: Transport> Drop for RuntimeGuard<T> {
    fn drop(&mut self) {
        let _ = self.transport.close();
    }
}

impl InputSourceState {
    fn apply(&mut self, input: &QueuedInput) -> Result<(), PilotError> {
        for event in &input.events {
            match event {
                egui::Event::PointerMoved(position) => {
                    self.pointer_positions.insert(input.viewport, *position);
                }
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                } => {
                    if *pressed {
                        if self
                            .buttons
                            .iter()
                            .any(|(_, viewport, _, _)| *viewport != input.viewport)
                        {
                            return Err(viewport_error(
                                "input-source-viewport-switch",
                                "pointer source cannot switch viewports while a button is pressed",
                            ));
                        }
                        self.buttons
                            .retain(|(existing, _, _, _)| existing != button);
                        self.buttons
                            .push((*button, input.viewport, *pos, *modifiers));
                    } else {
                        self.buttons
                            .retain(|(existing, _, _, _)| existing != button);
                    }
                    self.pointer_positions.insert(input.viewport, *pos);
                }
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => {
                    if *pressed {
                        if self
                            .keys
                            .values()
                            .any(|(viewport, _)| *viewport != input.viewport)
                        {
                            return Err(viewport_error(
                                "input-source-viewport-switch",
                                "keyboard source cannot switch viewports while a key is pressed",
                            ));
                        }
                        self.keys.insert(*key, (input.viewport, *modifiers));
                    } else {
                        self.keys.remove(key);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn release_all(&mut self) -> Vec<QueuedInput> {
        let mut events: HashMap<egui::ViewportId, Vec<egui::Event>> = HashMap::new();
        for (key, (viewport, modifiers)) in self.keys.drain() {
            events
                .entry(viewport)
                .or_default()
                .push(key_event(key, false, modifiers));
        }
        for (button, viewport, position, modifiers) in self.buttons.drain(..) {
            events
                .entry(viewport)
                .or_default()
                .push(egui::Event::PointerButton {
                    pos: position,
                    button,
                    pressed: false,
                    modifiers,
                });
        }
        events
            .into_iter()
            .map(|(viewport, events)| QueuedInput {
                viewport,
                events,
                time: None,
                modifiers: Some(egui::Modifiers::default()),
            })
            .collect()
    }

    fn is_clear(&self) -> bool {
        self.keys.is_empty() && self.buttons.is_empty()
    }

    fn owns_viewport(&self, viewport: egui::ViewportId) -> bool {
        self.keys.values().any(|(owner, _)| *owner == viewport)
            || self
                .buttons
                .iter()
                .any(|(_, owner, _, _)| *owner == viewport)
    }

    fn discard_viewport(&mut self, viewport: egui::ViewportId) {
        self.keys.retain(|_, (owner, _)| *owner != viewport);
        self.buttons.retain(|(_, owner, _, _)| *owner != viewport);
        self.pointer_positions.remove(&viewport);
    }
}

fn primary_target(action: &Action) -> Option<&Locator> {
    match action {
        Action::Click { target, .. }
        | Action::DoubleClick { target }
        | Action::Hover { target }
        | Action::Fill { target, .. }
        | Action::Clear { target }
        | Action::Type { target, .. }
        | Action::ScrollIntoView { target }
        | Action::SetValue { target, .. } => Some(target),
        Action::DragTo { source, .. } => Some(source),
        Action::Press { .. }
        | Action::Scroll { .. }
        | Action::FocusViewport { .. }
        | Action::MinimizeViewport { .. }
        | Action::RestoreViewport { .. }
        | Action::CloseViewport { .. } => None,
    }
}

pub fn resolve_locator<'a>(
    frame: &'a SemanticFrame,
    locator: &Locator,
) -> Result<&'a SemanticNode, PilotError> {
    let matches: Vec<&SemanticNode> = frame
        .nodes
        .iter()
        .filter(|node| locator.id.as_ref().is_none_or(|id| id == &node.id))
        .filter(|node| locator.role.is_none_or(|role| role == node.role))
        .filter(|node| locator.name.as_ref().is_none_or(|name| node.name == *name))
        .filter(|node| {
            locator.text.as_ref().is_none_or(|text| {
                node.name.contains(text)
                    || node
                        .value
                        .as_ref()
                        .is_some_and(|value| value.contains(text))
                    || node
                        .description
                        .as_ref()
                        .is_some_and(|value| value.contains(text))
            })
        })
        .filter(|node| {
            locator
                .within
                .as_ref()
                .is_none_or(|parent| is_descendant(frame, node, parent))
        })
        .filter(|node| {
            locator
                .viewport
                .as_ref()
                .is_none_or(|selector| viewport_descriptor_matches(frame, &node.viewport, selector))
        })
        .collect();
    match matches.as_slice() {
        [node] => Ok(*node),
        [] => Err(PilotError {
            code: "locator-not-found".into(),
            message: format!("locator did not match a live node: {locator:?}"),
            context: None,
        }),
        nodes => Err(PilotError {
            code: "locator-ambiguous".into(),
            message: format!(
                "locator matched {} nodes: {}",
                nodes.len(),
                nodes
                    .iter()
                    .map(|node| node.id.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            context: None,
        }),
    }
}

fn is_descendant(frame: &SemanticFrame, node: &SemanticNode, ancestor: &NodeId) -> bool {
    let mut parent = node.parent.as_ref();
    while let Some(id) = parent {
        if id == ancestor {
            return true;
        }
        parent = frame
            .nodes
            .iter()
            .find(|candidate| &candidate.id == id)
            .and_then(|candidate| candidate.parent.as_ref());
    }
    false
}

fn assert_actionable(node: &SemanticNode, action: &Action) -> Result<(), PilotError> {
    let forced = matches!(action, Action::Click { force: true, .. });
    if !node.visible() {
        return Err(action_error("not-visible", node));
    }
    if node.state.disabled && !forced {
        return Err(action_error("disabled", node));
    }
    let required = match action {
        Action::Click { .. } => Some(ActionKind::Click),
        Action::DoubleClick { .. } => Some(ActionKind::DoubleClick),
        Action::Hover { .. } => Some(ActionKind::Hover),
        Action::Fill { .. } => Some(ActionKind::Fill),
        Action::Clear { .. } => Some(ActionKind::Clear),
        Action::Type { .. } => Some(ActionKind::Type),
        Action::DragTo { .. } => Some(ActionKind::Drag),
        Action::ScrollIntoView { .. } => Some(ActionKind::ScrollIntoView),
        Action::SetValue { .. } => Some(ActionKind::SetValue),
        Action::Press { .. }
        | Action::Scroll { .. }
        | Action::FocusViewport { .. }
        | Action::MinimizeViewport { .. }
        | Action::RestoreViewport { .. }
        | Action::CloseViewport { .. } => None,
    };
    if required.is_some_and(|kind| !node.actions.contains(&kind)) && !forced {
        return Err(action_error("unsupported-action", node));
    }
    Ok(())
}

fn action_error(code: &str, node: &SemanticNode) -> PilotError {
    PilotError {
        code: code.into(),
        message: format!("node `{}` is {code}", node.id),
        context: None,
    }
}

fn action_batches(
    pilot: &PilotHandle,
    frame: &SemanticFrame,
    action: &Action,
) -> Result<Vec<QueuedInput>, PilotError> {
    let modifiers = egui::Modifiers::default();
    let batch = |viewport, events| QueuedInput {
        viewport,
        events,
        time: None,
        modifiers: None,
    };
    match action {
        Action::Press { key, viewport } => {
            let key = parse_key(key).ok_or_else(|| PilotError {
                code: "unknown-key".into(),
                message: format!("unsupported key `{key}`"),
                context: None,
            })?;
            let (_, viewport) = resolve_input_viewport(pilot, viewport.as_ref())?;
            Ok(vec![batch(
                viewport,
                vec![
                    key_event(key, true, modifiers),
                    key_event(key, false, modifiers),
                ],
            )])
        }
        Action::Scroll {
            delta_x,
            delta_y,
            viewport,
        } => {
            let (_, viewport) = resolve_input_viewport(pilot, viewport.as_ref())?;
            Ok(vec![batch(
                viewport,
                vec![egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(*delta_x, *delta_y),
                    phase: egui::TouchPhase::Move,
                    modifiers,
                }],
            )])
        }
        Action::FocusViewport { .. }
        | Action::MinimizeViewport { .. }
        | Action::RestoreViewport { .. }
        | Action::CloseViewport { .. } => {
            unreachable!("viewport commands are prepared without input batches")
        }
        Action::SetValue { .. } => Err(PilotError {
            code: "white-box-adapter-required".into(),
            message: "set-value requires an explicitly registered application adapter".into(),
            context: None,
        }),
        Action::ScrollIntoView { .. } => Err(PilotError {
            code: "scroll-adapter-required".into(),
            message: "scroll-into-view requires a registered scroll-container adapter".into(),
            context: None,
        }),
        Action::DragTo { source, target } => {
            let source = resolve_locator(frame, source)?;
            let target = resolve_locator(frame, target)?;
            assert_actionable(source, action)?;
            let viewport = viewport_for(pilot, source)?;
            if source.viewport != target.viewport {
                return Err(PilotError {
                    code: "cross-viewport-drag".into(),
                    message: "drag source and target must be in one viewport".into(),
                    context: None,
                });
            }
            let from = center(source.rect);
            let to = center(target.rect);
            let points = (1..=5).map(|index| from.lerp(to, index as f32 / 5.0));
            let mut batches = vec![batch(viewport, vec![egui::Event::PointerMoved(from)])];
            batches.push(batch(
                viewport,
                vec![egui::Event::PointerButton {
                    pos: from,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                }],
            ));
            batches.extend(
                points.map(|point| batch(viewport, vec![egui::Event::PointerMoved(point)])),
            );
            batches.push(batch(
                viewport,
                vec![egui::Event::PointerButton {
                    pos: to,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                }],
            ));
            Ok(batches)
        }
        action => {
            let locator = primary_target(action).expect("targeted action");
            let node = resolve_locator(frame, locator)?;
            let viewport = viewport_for(pilot, node)?;
            let pos = center(node.rect);
            let button = match action {
                Action::Click { button, .. } => pointer_button(*button),
                _ => egui::PointerButton::Primary,
            };
            let click = || {
                vec![
                    batch(viewport, vec![egui::Event::PointerMoved(pos)]),
                    batch(
                        viewport,
                        vec![egui::Event::PointerButton {
                            pos,
                            button,
                            pressed: true,
                            modifiers,
                        }],
                    ),
                    batch(
                        viewport,
                        vec![egui::Event::PointerButton {
                            pos,
                            button,
                            pressed: false,
                            modifiers,
                        }],
                    ),
                ]
            };
            match action {
                Action::Hover { .. } => {
                    Ok(vec![batch(viewport, vec![egui::Event::PointerMoved(pos)])])
                }
                Action::Click { .. } => Ok(click()),
                Action::DoubleClick { .. } => {
                    let mut events = click();
                    events.extend(click());
                    Ok(events)
                }
                Action::Fill { text, .. } => {
                    let mut events = click();
                    events.push(batch(viewport, select_all_events()));
                    events.push(batch(viewport, vec![egui::Event::Text(text.clone())]));
                    Ok(events)
                }
                Action::Clear { .. } => {
                    let mut events = click();
                    events.push(batch(viewport, select_all_events()));
                    events.push(batch(
                        viewport,
                        vec![
                            key_event(egui::Key::Backspace, true, modifiers),
                            key_event(egui::Key::Backspace, false, modifiers),
                        ],
                    ));
                    Ok(events)
                }
                Action::Type { text, .. } => {
                    let mut events = click();
                    events.push(batch(viewport, vec![egui::Event::Text(text.clone())]));
                    Ok(events)
                }
                _ => unreachable!(),
            }
        }
    }
}

fn viewport_for(pilot: &PilotHandle, node: &SemanticNode) -> Result<egui::ViewportId, PilotError> {
    pilot
        .egui_viewport(&node.viewport)
        .ok_or_else(|| PilotError {
            code: "viewport-not-found".into(),
            message: format!("viewport `{}` is no longer live", node.viewport),
            context: None,
        })
}

fn resolve_input_viewport(
    pilot: &PilotHandle,
    selector: Option<&ViewportSelector>,
) -> Result<(ViewportDescriptor, egui::ViewportId), PilotError> {
    let result = match selector {
        Some(selector) => pilot.resolve_viewport(selector),
        None => pilot.only_viewport(),
    };
    let (descriptor, viewport) =
        result.map_err(|message| viewport_error("viewport-required", message))?;
    require_native_viewport(&descriptor)?;
    Ok((descriptor, viewport))
}

fn require_native_viewport(viewport: &ViewportDescriptor) -> Result<(), PilotError> {
    if viewport.class == protocol::ViewportClass::Embedded {
        return Err(viewport_error(
            "viewport-embedded-unsupported",
            format!(
                "viewport `{}` is embedded; this action requires a native viewport",
                viewport.id
            ),
        ));
    }
    Ok(())
}

fn viewport_capture_support_error(viewport: &ViewportDescriptor) -> Option<PilotError> {
    match viewport.class {
        protocol::ViewportClass::Immediate => Some(PilotError {
            code: "viewport-capture-unsupported".into(),
            message: format!(
                "native screenshot capture is unsupported for immediate viewport `{}`: eframe 0.35's wgpu immediate-viewport renderer does not forward screenshot requests",
                viewport.key
            ),
            context: None,
        }),
        protocol::ViewportClass::Deferred => Some(PilotError {
            code: "viewport-capture-unsupported".into(),
            message: format!(
                "native screenshot capture is unsupported for deferred viewport `{}`: eframe 0.35 does not deliver its queued screenshot requests",
                viewport.key
            ),
            context: None,
        }),
        protocol::ViewportClass::Root | protocol::ViewportClass::Embedded => None,
    }
}

fn viewport_error(code: &str, message: impl Into<String>) -> PilotError {
    PilotError {
        code: code.into(),
        message: message.into(),
        context: None,
    }
}

fn viewport_descriptor_matches(
    frame: &SemanticFrame,
    id: &protocol::ViewportId,
    selector: &ViewportSelector,
) -> bool {
    frame.viewports.iter().any(|viewport| {
        &viewport.id == id
            && selector
                .id
                .as_ref()
                .is_none_or(|selected| selected == &viewport.id)
            && selector
                .key
                .as_ref()
                .is_none_or(|selected| selected == &viewport.key)
    })
}

fn center(rect: protocol::Rect) -> egui::Pos2 {
    egui::pos2(
        (rect.min_x + rect.max_x) * 0.5,
        (rect.min_y + rect.max_y) * 0.5,
    )
}

fn pointer_button(button: PointerButton) -> egui::PointerButton {
    match button {
        PointerButton::Primary => egui::PointerButton::Primary,
        PointerButton::Secondary => egui::PointerButton::Secondary,
        PointerButton::Middle => egui::PointerButton::Middle,
    }
}

fn key_event(key: egui::Key, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn select_all_events() -> Vec<egui::Event> {
    let command = egui::Modifiers {
        command: true,
        ctrl: cfg!(not(target_os = "macos")),
        mac_cmd: cfg!(target_os = "macos"),
        ..Default::default()
    };
    vec![
        key_event(egui::Key::A, true, command),
        key_event(egui::Key::A, false, command),
    ]
}

fn parse_key(value: &str) -> Option<egui::Key> {
    Some(match value.to_ascii_lowercase().as_str() {
        "enter" => egui::Key::Enter,
        "escape" | "esc" => egui::Key::Escape,
        "tab" => egui::Key::Tab,
        "space" => egui::Key::Space,
        "backspace" => egui::Key::Backspace,
        "delete" => egui::Key::Delete,
        "arrow-up" => egui::Key::ArrowUp,
        "arrow-down" => egui::Key::ArrowDown,
        "arrow-left" => egui::Key::ArrowLeft,
        "arrow-right" => egui::Key::ArrowRight,
        "home" => egui::Key::Home,
        "end" => egui::Key::End,
        _ => return None,
    })
}

fn evaluate_expectation(
    frame: &SemanticFrame,
    expectation: &Expectation,
) -> Result<(), PilotError> {
    let fail = |message: String| PilotError {
        code: "expectation-failed".into(),
        message,
        context: None,
    };
    match expectation {
        Expectation::NoDiagnostics => {
            if frame.diagnostics == protocol::FrameDiagnostics::default() {
                Ok(())
            } else {
                Err(fail(format!(
                    "frame has diagnostics: {:?}",
                    frame.diagnostics
                )))
            }
        }
        Expectation::Screenshot { .. } => Err(fail(
            "screenshot expectations are capture steps, not semantic assertions".into(),
        )),
        Expectation::Coach { .. } => Err(fail(
            "coach expectations read the session ledger, not one semantic frame".into(),
        )),
        Expectation::Count { target, value } => {
            let count = matching_nodes(frame, target).len();
            (count == *value)
                .then_some(())
                .ok_or_else(|| fail(format!("expected {value} nodes, found {count}")))
        }
        expectation => {
            let locator = match expectation {
                Expectation::Visible { target }
                | Expectation::Hidden { target }
                | Expectation::Enabled { target }
                | Expectation::Disabled { target }
                | Expectation::Focused { target }
                | Expectation::Text { target, .. }
                | Expectation::Value { target, .. } => target,
                _ => unreachable!(),
            };
            let node = resolve_locator(frame, locator)?;
            let passed = match expectation {
                Expectation::Visible { .. } => node.visible(),
                Expectation::Hidden { .. } => !node.visible(),
                Expectation::Enabled { .. } => !node.state.disabled,
                Expectation::Disabled { .. } => node.state.disabled,
                Expectation::Focused { .. } => node.state.focused,
                Expectation::Text { value, .. } => node.name.contains(value),
                Expectation::Value { value, .. } => node.value.as_ref() == Some(value),
                _ => unreachable!(),
            };
            passed.then_some(()).ok_or_else(|| {
                fail(format!(
                    "expectation {expectation:?} failed for `{}`",
                    node.id
                ))
            })
        }
    }
}

fn matching_nodes<'a>(frame: &'a SemanticFrame, locator: &Locator) -> Vec<&'a SemanticNode> {
    frame
        .nodes
        .iter()
        .filter(|node| locator.id.as_ref().is_none_or(|id| &node.id == id))
        .filter(|node| locator.role.is_none_or(|role| node.role == role))
        .filter(|node| locator.name.as_ref().is_none_or(|name| &node.name == name))
        .filter(|node| {
            locator
                .viewport
                .as_ref()
                .is_none_or(|selector| viewport_descriptor_matches(frame, &node.viewport, selector))
        })
        .collect()
}

fn write_capture(
    root: &Path,
    name: &str,
    renderer_key: &str,
    viewport: ViewportDescriptor,
    image: &egui::ColorImage,
) -> Result<CaptureResult, PilotError> {
    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_array());
    }
    let hash = protocol::stable_hash(&rgba);
    let capture_root = root.join("captures");
    std::fs::create_dir_all(&capture_root).map_err(|error| PilotError {
        code: "artifact-root".into(),
        message: error.to_string(),
        context: None,
    })?;
    let path = capture_root.join(format!("{hash}.png"));
    if !path.exists() {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| PilotError {
                code: "capture-create".into(),
                message: error.to_string(),
                context: None,
            })?;
        let encoder = image::codecs::png::PngEncoder::new(file);
        image::ImageEncoder::write_image(
            encoder,
            &rgba,
            image.size[0] as u32,
            image.size[1] as u32,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| PilotError {
            code: "capture-encode".into(),
            message: error.to_string(),
            context: None,
        })?;
    }
    Ok(CaptureResult {
        attachment: protocol::AttachmentId::new(format!("fnv1a64:{hash}")),
        name: name.into(),
        artifact: path.to_string_lossy().into_owned(),
        content_hash: hash,
        renderer_key: renderer_key.into(),
        viewport: Some(viewport),
        logical_size: Some([image.size[0] as u32, image.size[1] as u32]),
    })
}

fn semantic_hash(frame: &SemanticFrame, viewport: &protocol::ViewportId) -> String {
    let mut nodes: Vec<_> = frame
        .nodes
        .iter()
        .filter(|node| &node.viewport == viewport)
        .cloned()
        .collect();
    for node in &mut nodes {
        node.changed_at = 0;
        node.rect = protocol::Rect::default();
        node.clip_rect = protocol::Rect::default();
    }
    protocol::stable_hash(&serde_json::to_vec(&nodes).unwrap_or_default())
}

fn geometry_hash(frame: &SemanticFrame, viewport: &protocol::ViewportId) -> String {
    let geometry: Vec<_> = frame
        .nodes
        .iter()
        .filter(|node| &node.viewport == viewport)
        .map(|node| (&node.id, node.rect, node.clip_rect, node.layer))
        .collect();
    protocol::stable_hash(&serde_json::to_vec(&geometry).unwrap_or_default())
}

fn pixel_hash(image: &egui::ColorImage) -> String {
    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_array());
    }
    protocol::stable_hash(&rgba)
}

fn viewport_focus_transferred(initial: &ViewportDescriptor, frame: &SemanticFrame) -> bool {
    initial.focused
        && (frame
            .viewports
            .iter()
            .find(|viewport| viewport.id == initial.id)
            .is_some_and(|viewport| !viewport.focused)
            || frame
                .viewports
                .iter()
                .any(|viewport| viewport.id != initial.id && viewport.focused))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_pilot_ai_protocol::{NodeState, Rect, Role, SemanticNode, ViewportId};
    use egui_pilot_ai_transport::in_memory_pair;

    fn node(id: &str) -> SemanticNode {
        SemanticNode {
            id: NodeId::new(id),
            viewport: ViewportId::new("root"),
            parent: None,
            role: Role::Button,
            name: "Save".into(),
            description: None,
            value: None,
            rect: Rect {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 100.0,
                max_y: 30.0,
            },
            clip_rect: Rect {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 100.0,
                max_y: 100.0,
            },
            state: NodeState::default(),
            actions: vec![ActionKind::Click],
            layer: 0,
            changed_at: 1,
            source: None,
        }
    }

    #[test]
    fn strict_locator_rejects_ambiguity() {
        let frame = SemanticFrame {
            revision: 1,
            timestamp_seconds: 0.0,
            nodes: vec![node("a"), node("b")],
            viewports: Vec::new(),
            diagnostics: Default::default(),
        };
        let error = resolve_locator(
            &frame,
            &Locator {
                role: Some(Role::Button),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "locator-ambiguous");
    }

    #[test]
    fn handshake_is_required_and_nonce_bound() {
        let ctx = egui::Context::default();
        let (client, transport) = in_memory_pair(8);
        let config = RuntimeConfig {
            build: "test".into(),
            session: SessionId::new("s"),
            session_nonce: "n".into(),
            artifact_root: std::env::temp_dir().join("eguipilotai-runtime-test"),
            max_commands_per_frame: 8,
            renderer_identity: None,
        };
        let mut runtime = RuntimeGuard::attach(&ctx, transport, config);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("request-1"),
                session: SessionId::new("s"),
                command: ClientCommand::Status,
            })
            .unwrap();
        runtime.pump();
        let response = client.try_receive().unwrap().unwrap();
        assert!(matches!(
            response,
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Error { .. },
                    ..
                }
            }
        ));

        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("request-2"),
                session: SessionId::new("s"),
                command: ClientCommand::Handshake(HandshakeRequest {
                    protocol: PROTOCOL_VERSION,
                    build: "runner".into(),
                    session_nonce: "n".into(),
                    required_capabilities: vec![],
                    trace_level: protocol::TraceLevel::Off,
                }),
            })
            .unwrap();
        runtime.pump();
        let response = client.try_receive().unwrap().unwrap();
        assert!(matches!(
            response,
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Success { result },
                    ..
                }
            } if matches!(result.as_ref(), CommandResult::Ready { .. })
        ));
    }

    #[test]
    fn protocol_major_mismatch_fails_before_authentication() {
        let (_ctx, client, mut runtime) = test_runtime(8);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("handshake-v3"),
                session: SessionId::new("session"),
                command: ClientCommand::Handshake(HandshakeRequest {
                    protocol: protocol::ProtocolVersion { major: 3, minor: 0 },
                    build: "future-runner".into(),
                    session_nonce: "nonce".into(),
                    required_capabilities: vec![],
                    trace_level: protocol::TraceLevel::Off,
                }),
            })
            .unwrap();
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Error {
                        error: PilotError { code, .. }
                    },
                    ..
                }
            } if code == "protocol-version"
        ));

        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("status-after-v3"),
                session: SessionId::new("session"),
                command: ClientCommand::Status,
            })
            .unwrap();
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Error {
                        error: PilotError { code, .. }
                    },
                    ..
                }
            } if code == "handshake-required"
        ));
    }

    #[test]
    fn protocol_minor_mismatch_fails_closed() {
        let (_ctx, client, mut runtime) = test_runtime(8);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("handshake-v2-0"),
                session: SessionId::new("session"),
                command: ClientCommand::Handshake(HandshakeRequest {
                    protocol: protocol::ProtocolVersion { major: 2, minor: 0 },
                    build: "older-runner".into(),
                    session_nonce: "nonce".into(),
                    required_capabilities: vec![],
                    trace_level: protocol::TraceLevel::Off,
                }),
            })
            .unwrap();
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Error {
                        error: PilotError { code, .. }
                    },
                    ..
                }
            } if code == "protocol-version"
        ));
    }

    fn test_runtime(
        capacity: usize,
    ) -> (
        egui::Context,
        egui_pilot_ai_transport::InMemoryClient,
        RuntimeGuard<egui_pilot_ai_transport::InMemoryTransport>,
    ) {
        let ctx = egui::Context::default();
        let (client, transport) = in_memory_pair(capacity);
        let config = RuntimeConfig {
            build: "test".into(),
            session: SessionId::new("session"),
            session_nonce: "nonce".into(),
            artifact_root: std::env::temp_dir().join("eguipilotai-runtime-v2-test"),
            max_commands_per_frame: 8,
            renderer_identity: None,
        };
        let runtime = RuntimeGuard::attach(&ctx, transport, config);
        (ctx, client, runtime)
    }

    fn authenticate(
        client: &egui_pilot_ai_transport::InMemoryClient,
        runtime: &mut RuntimeGuard<egui_pilot_ai_transport::InMemoryTransport>,
    ) {
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("handshake"),
                session: SessionId::new("session"),
                command: ClientCommand::Handshake(HandshakeRequest {
                    protocol: PROTOCOL_VERSION,
                    build: "test".into(),
                    session_nonce: "nonce".into(),
                    required_capabilities: vec![],
                    trace_level: protocol::TraceLevel::Off,
                }),
            })
            .unwrap();
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Success { result },
                    ..
                }
            } if matches!(result.as_ref(), CommandResult::Ready { .. })
        ));
    }

    const COACH_SAVE: egui_pilot_ai::PilotId = egui_pilot_ai::PilotId::new("coach.save");

    fn coach_frame(ctx: &egui::Context, input: egui::RawInput) {
        use egui_pilot_ai::UiPilotExt;
        ctx.run_ui(input, |ui| {
            ui.pilot_button(COACH_SAVE, "Save");
        })
        .textures_delta
        .clear();
    }

    fn drain_coach_events(client: &egui_pilot_ai_transport::InMemoryClient) -> Vec<CoachEvent> {
        let mut events = Vec::new();
        while let Ok(Some(message)) = client.try_receive() {
            if let ServerMessage::Event { envelope } = message {
                if let RuntimeEvent::Coach { event } = envelope.event {
                    events.push(event);
                }
            }
        }
        events
    }

    fn touchpoint(coach_id: &str, target: Locator) -> Touchpoint {
        Touchpoint {
            coach_id: CoachId::new(coach_id),
            target,
            visual: CoachStyle::Spotlight,
            on: TriggerKind::Press,
            until: None,
            message: Some("Press Save".into()),
            ttl_ms: None,
            once: true,
        }
    }

    fn send_mount(
        client: &egui_pilot_ai_transport::InMemoryClient,
        request_id: &str,
        touchpoint: Touchpoint,
    ) {
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new(request_id),
                session: SessionId::new("session"),
                command: ClientCommand::Coach {
                    coach: CoachCommand::Mount {
                        touchpoint: Box::new(touchpoint),
                    },
                },
            })
            .unwrap();
    }

    #[test]
    fn coach_press_triggers_once_and_unmounts() {
        let (ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);
        coach_frame(&ctx, Default::default());

        let target = Locator {
            id: Some(NodeId::new("root/coach.save")),
            ..Locator::default()
        };
        send_mount(&client, "mount", touchpoint("coach-save", target));
        runtime.pump();
        let mut mounted_rect = None;
        while let Ok(Some(message)) = client.try_receive() {
            if let ServerMessage::Response { envelope } = message {
                let ResponseOutcome::Success { result } = envelope.outcome else {
                    panic!("mount must succeed on a live node");
                };
                let CommandResult::Coach { events } = *result else {
                    panic!("mount must answer with coach events");
                };
                match events.as_slice() {
                    [CoachEvent::Mounted { rect, .. }] => mounted_rect = Some(*rect),
                    other => panic!("unexpected mount outcome: {other:?}"),
                }
            }
        }
        let rect = mounted_rect.expect("mount response");
        assert_eq!(runtime.pilot().mounted_coach_count(), 1);

        let center = egui::pos2(
            (rect.min_x + rect.max_x) / 2.0,
            (rect.min_y + rect.max_y) / 2.0,
        );
        coach_frame(
            &ctx,
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(center),
                    egui::Event::PointerButton {
                        pos: center,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
        );
        runtime.pump();

        assert_eq!(
            drain_coach_events(&client),
            vec![
                CoachEvent::Triggered {
                    coach_id: CoachId::new("coach-save"),
                    kind: TriggerKind::Press,
                    actor: protocol::CoachActor::Operator,
                },
                CoachEvent::Unmounted {
                    coach_id: CoachId::new("coach-save"),
                    reason: CoachUnmountReason::Once,
                },
            ]
        );
        assert_eq!(runtime.pilot().mounted_coach_count(), 0);
    }

    #[test]
    fn an_armed_touchpoint_mounts_when_its_target_appears() {
        let (ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);
        // Deliberately no coach frame yet: the target does not exist.
        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();

        let mut waiting = touchpoint(
            "coach-appear",
            Locator {
                id: Some(NodeId::new("root/coach.save")),
                ..Locator::default()
            },
        );
        waiting.on = TriggerKind::NodeAppeared;
        send_mount(&client, "mount-appear", waiting);
        runtime.pump();

        assert!(
            drain_coach_events(&client).contains(&CoachEvent::Armed {
                coach_id: CoachId::new("coach-appear"),
            }),
            "a touchpoint waiting for a target arms rather than refusing"
        );
        assert_eq!(runtime.pilot().mounted_coach_count(), 1);

        coach_frame(&ctx, Default::default());
        runtime.pump();

        let events = drain_coach_events(&client);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, CoachEvent::Mounted { coach_id, .. }
                    if coach_id == &CoachId::new("coach-appear"))),
            "{events:?}"
        );
        assert!(
            events.contains(&CoachEvent::Triggered {
                coach_id: CoachId::new("coach-appear"),
                kind: TriggerKind::NodeAppeared,
                actor: protocol::CoachActor::App,
            }),
            "{events:?}"
        );
    }

    /// The lesson shape: step two points at a control that only exists because
    /// step one was pressed. A mount is a subscription, not a resolution.
    #[test]
    fn a_press_touchpoint_waits_for_a_target_the_last_step_reveals() {
        let (ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);
        // Deliberately no coach frame yet: the control is not revealed.
        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();

        let target = Locator {
            id: Some(NodeId::new("root/coach.save")),
            ..Locator::default()
        };
        send_mount(&client, "mount-wait", touchpoint("coach-wait", target));
        runtime.pump();

        assert!(
            drain_coach_events(&client).contains(&CoachEvent::Armed {
                coach_id: CoachId::new("coach-wait"),
            }),
            "a press touchpoint waits for its target rather than refusing"
        );
        assert_eq!(runtime.pilot().mounted_coach_count(), 1);

        // The previous step lands and the control comes into existence.
        coach_frame(&ctx, Default::default());
        runtime.pump();

        let rect = drain_coach_events(&client)
            .into_iter()
            .find_map(|event| match event {
                CoachEvent::Mounted { coach_id, rect }
                    if coach_id == CoachId::new("coach-wait") =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .expect("the touchpoint binds when its target appears");

        let center = egui::pos2(
            (rect.min_x + rect.max_x) / 2.0,
            (rect.min_y + rect.max_y) / 2.0,
        );
        coach_frame(
            &ctx,
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(center),
                    egui::Event::PointerButton {
                        pos: center,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
        );
        runtime.pump();

        assert!(
            drain_coach_events(&client).contains(&CoachEvent::Triggered {
                coach_id: CoachId::new("coach-wait"),
                kind: TriggerKind::Press,
                actor: protocol::CoachActor::Operator,
            }),
            "the press fires normally once the wait resolved"
        );
    }

    #[test]
    fn a_touchpoint_that_waits_cannot_be_aimed_with_a_name_locator() {
        let (ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);
        coach_frame(&ctx, Default::default());

        // The node is right there, but a name cannot be matched against a
        // node that has not rendered yet, which is exactly what this
        // touchpoint is for.
        let mut by_name = touchpoint(
            "coach-by-name",
            Locator {
                name: Some("Save".into()),
                ..Locator::default()
            },
        );
        by_name.on = TriggerKind::NodeAppeared;
        send_mount(&client, "mount-by-name", by_name);
        runtime.pump();

        let refusals: Vec<_> = drain_coach_events(&client)
            .into_iter()
            .filter_map(|event| match event {
                CoachEvent::Refused {
                    coach_id, reason, ..
                } => Some((coach_id, reason)),
                _ => None,
            })
            .collect();
        assert_eq!(
            refusals,
            vec![(CoachId::new("coach-by-name"), CoachRefusal::TargetNotFound)]
        );
        assert_eq!(runtime.pilot().mounted_coach_count(), 0);
    }

    #[test]
    fn a_touchpoint_the_target_cannot_honour_is_refused_not_mounted() {
        let (ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);
        coach_frame(&ctx, Default::default());

        // A *name* cannot be watched for, so an unresolved name refuses. The
        // same touchpoint carrying a node id would wait instead — see
        // `a_press_touchpoint_waits_for_a_target_the_last_step_reveals`.
        let missing = Locator {
            name: Some("Nowhere".into()),
            ..Locator::default()
        };
        send_mount(
            &client,
            "mount-missing",
            touchpoint("coach-missing", missing),
        );
        let live = Locator {
            id: Some(NodeId::new("root/coach.save")),
            ..Locator::default()
        };
        let mut outline = touchpoint("coach-outline", live);
        outline.visual = CoachStyle::Outline;
        send_mount(&client, "mount-outline", outline);
        runtime.pump();

        let refusals: Vec<_> = drain_coach_events(&client)
            .into_iter()
            .filter_map(|event| match event {
                CoachEvent::Refused {
                    coach_id, reason, ..
                } => Some((coach_id, reason)),
                _ => None,
            })
            .collect();
        assert_eq!(
            refusals,
            vec![
                (CoachId::new("coach-missing"), CoachRefusal::TargetNotFound),
                (
                    CoachId::new("coach-outline"),
                    CoachRefusal::UnsupportedStyle
                ),
            ]
        );
        assert_eq!(runtime.pilot().mounted_coach_count(), 0);
    }

    #[test]
    fn stopping_the_session_unmounts_every_touchpoint() {
        let (ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);
        coach_frame(&ctx, Default::default());
        let target = Locator {
            id: Some(NodeId::new("root/coach.save")),
            ..Locator::default()
        };
        send_mount(&client, "mount", touchpoint("coach-save", target));
        runtime.pump();
        let _ = drain_coach_events(&client);
        assert_eq!(runtime.pilot().mounted_coach_count(), 1);

        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("stop"),
                session: SessionId::new("session"),
                command: ClientCommand::Stop,
            })
            .unwrap();
        runtime.pump();

        assert!(
            drain_coach_events(&client).contains(&CoachEvent::Unmounted {
                coach_id: CoachId::new("coach-save"),
                reason: CoachUnmountReason::SessionEnded,
            })
        );
        assert_eq!(runtime.pilot().mounted_coach_count(), 0);
    }

    #[test]
    fn run_progress_is_event_only_and_run_has_one_terminal_response() {
        let (_ctx, client, mut runtime) = test_runtime(16);
        authenticate(&client, &mut runtime);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("run-request"),
                session: SessionId::new("session"),
                command: ClientCommand::Run {
                    run_id: RunId::new("run"),
                    flow: Flow {
                        experiment: None,
                        id: protocol::ScenarioId::new("empty"),
                        fixture: None,
                        steps: vec![],
                        record: false,
                    },
                },
            })
            .unwrap();
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Event {
                envelope
            } if matches!(envelope.event, RuntimeEvent::RunStarted { .. })
        ));
        runtime.pump();
        let response = client.try_receive().unwrap().unwrap();
        assert!(matches!(
            response,
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    request_id,
                    outcome: ResponseOutcome::Success { result },
                    ..
                }
            } if request_id == RequestId::new("run-request") && matches!(
                result.as_ref(),
                CommandResult::Run {
                            result: RunResult {
                                outcome: RunOutcome::Passed,
                                ..
                            }
                        }
            )
        ));
        assert!(client.try_receive().unwrap().is_none());
    }

    #[test]
    fn status_interleaves_with_run_and_cancellation_is_idempotent() {
        let (_ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("run-request"),
                session: SessionId::new("session"),
                command: ClientCommand::Run {
                    run_id: RunId::new("run"),
                    flow: Flow {
                        experiment: None,
                        id: protocol::ScenarioId::new("waiting"),
                        fixture: None,
                        steps: vec![Step::Wait {
                            milliseconds: 60_000,
                        }],
                        record: false,
                    },
                },
            })
            .unwrap();
        runtime.pump();
        let _ = client.try_receive().unwrap();

        runtime.pilot.queue_input(QueuedInput {
            viewport: egui::ViewportId::ROOT,
            events: vec![egui::Event::Text("must-not-leak".into())],
            time: None,
            modifiers: None,
        });
        assert_eq!(runtime.pilot.pending_input(), 1);
        runtime.pump();
        let _ = client.try_receive().unwrap();

        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("status"),
                session: SessionId::new("session"),
                command: ClientCommand::Status,
            })
            .unwrap();
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("cancel"),
                session: SessionId::new("session"),
                command: ClientCommand::Cancel {
                    run_id: RunId::new("run"),
                },
            })
            .unwrap();
        runtime.pump();
        let mut terminal_requests = Vec::new();
        while let Some(message) = client.try_receive().unwrap() {
            if let ServerMessage::Response { envelope } = message {
                terminal_requests.push(envelope.request_id);
            }
        }
        assert!(terminal_requests.contains(&RequestId::new("status")));
        assert!(terminal_requests.contains(&RequestId::new("run-request")));
        assert!(terminal_requests.contains(&RequestId::new("cancel")));
        assert_eq!(runtime.pilot.pending_input(), 0);

        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("cancel-again"),
                session: SessionId::new("session"),
                command: ClientCommand::Cancel {
                    run_id: RunId::new("run"),
                },
            })
            .unwrap();
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Success { result },
                    ..
                }
            } if matches!(
                result.as_ref(),
                CommandResult::Cancel {
                            cancellation: CancellationResult {
                                state: CancellationState::AlreadyTerminal,
                                ..
                            }
                        }
            )
        ));
    }

    /// An agent cannot read the host's fabric, so the only way it learns which
    /// `signal:<id>` triggers are real is by the host declaring them and status
    /// carrying the list.
    #[test]
    fn status_reports_the_signal_catalog_the_host_declared() {
        let (_ctx, client, mut runtime) = test_runtime(16);
        authenticate(&client, &mut runtime);
        runtime.pilot.declare_coach_signals(vec![
            protocol::SignalId::new("build.slide-placed"),
            protocol::SignalId::new("shell.route-changed/workbench/library"),
        ]);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("status"),
                session: SessionId::new("session"),
                command: ClientCommand::Status,
            })
            .unwrap();
        runtime.pump();
        let ServerMessage::Response {
            envelope:
                ResponseEnvelope {
                    outcome: ResponseOutcome::Success { result },
                    ..
                },
        } = client.try_receive().unwrap().unwrap()
        else {
            panic!("status must succeed");
        };
        let CommandResult::Status { status } = *result else {
            panic!("expected a status result");
        };
        assert_eq!(
            status.coach_signals,
            vec![
                protocol::SignalId::new("build.slide-placed"),
                protocol::SignalId::new("shell.route-changed/workbench/library"),
            ]
        );
    }

    /// A host that declares nothing reports nothing, rather than the field
    /// vanishing into a default an agent could read as "anything goes".
    #[test]
    fn an_undeclared_host_reports_an_empty_catalog_not_a_missing_one() {
        let (_ctx, client, mut runtime) = test_runtime(16);
        authenticate(&client, &mut runtime);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("status"),
                session: SessionId::new("session"),
                command: ClientCommand::Status,
            })
            .unwrap();
        runtime.pump();
        let ServerMessage::Response {
            envelope:
                ResponseEnvelope {
                    outcome: ResponseOutcome::Success { result },
                    ..
                },
        } = client.try_receive().unwrap().unwrap()
        else {
            panic!("status must succeed");
        };
        let CommandResult::Status { status } = *result else {
            panic!("expected a status result");
        };
        assert!(status.coach_signals.is_empty());
    }

    #[test]
    fn unsupported_actions_fail_before_run_start() {
        let (_ctx, client, mut runtime) = test_runtime(16);
        authenticate(&client, &mut runtime);
        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("run-request"),
                session: SessionId::new("session"),
                command: ClientCommand::Run {
                    run_id: RunId::new("run"),
                    flow: Flow {
                        experiment: None,
                        id: protocol::ScenarioId::new("unsupported"),
                        fixture: None,
                        steps: vec![Step::Action {
                            action: Action::SetValue {
                                target: Locator::default(),
                                value: "5".into(),
                            },
                        }],
                        record: false,
                    },
                },
            })
            .unwrap();
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Error { error },
                    ..
                }
            } if error.code == "unsupported-capability"
        ));
        assert!(client.try_receive().unwrap().is_none());
    }

    #[test]
    fn telemetry_pressure_coalesces_and_reports_drops() {
        let (_ctx, client, mut runtime) = test_runtime(1);
        runtime.queue_telemetry(
            EventScope::default(),
            RuntimeEvent::Telemetry {
                channel: "frame".into(),
                revision: 1,
            },
        );
        runtime.queue_telemetry(
            EventScope::default(),
            RuntimeEvent::Telemetry {
                channel: "frame".into(),
                revision: 2,
            },
        );
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Event {
                envelope
            } if matches!(envelope.event, RuntimeEvent::QueuePressure { dropped: 1, .. })
        ));
        runtime.pump();
        assert!(matches!(
            client.try_receive().unwrap().unwrap(),
            ServerMessage::Event {
                envelope
            } if matches!(envelope.event, RuntimeEvent::Telemetry { revision: 2, .. })
        ));
    }

    #[test]
    fn input_cleanup_releases_sources_in_their_original_viewport() {
        let viewport = egui::ViewportId::from_hash_of("child");
        let mut state = InputSourceState::default();
        state
            .apply(&QueuedInput {
                viewport,
                events: vec![
                    egui::Event::PointerMoved(egui::pos2(10.0, 20.0)),
                    egui::Event::PointerButton {
                        pos: egui::pos2(10.0, 20.0),
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                time: None,
                modifiers: None,
            })
            .unwrap();
        let cleanup = state.release_all();
        assert!(state.is_clear());
        assert_eq!(cleanup.len(), 1);
        assert_eq!(cleanup[0].viewport, viewport);
        assert!(matches!(
            cleanup[0].events.as_slice(),
            [egui::Event::PointerButton { pressed: false, .. }]
        ));
    }

    #[test]
    fn pressed_source_cannot_switch_viewports() {
        let mut state = InputSourceState::default();
        let first = egui::ViewportId::from_hash_of("first");
        let second = egui::ViewportId::from_hash_of("second");
        state
            .apply(&QueuedInput {
                viewport: first,
                events: vec![key_event(
                    egui::Key::Enter,
                    true,
                    egui::Modifiers::default(),
                )],
                time: None,
                modifiers: None,
            })
            .unwrap();
        let error = state
            .apply(&QueuedInput {
                viewport: second,
                events: vec![key_event(
                    egui::Key::Space,
                    true,
                    egui::Modifiers::default(),
                )],
                time: None,
                modifiers: None,
            })
            .unwrap_err();
        assert_eq!(error.code, "input-source-viewport-switch");
    }

    #[test]
    fn immediate_viewport_capture_fails_before_renderer_timeout() {
        let viewport = ViewportDescriptor {
            id: ViewportId::new("devpulse.viziwiz.palette#1"),
            key: protocol::PilotViewportKey::new("devpulse.viziwiz.palette"),
            parent: Some(ViewportId::new("root")),
            class: protocol::ViewportClass::Immediate,
            title: "ViziWiz".into(),
            lifecycle: protocol::ViewportLifecycle::Open,
            lifecycle_revision: 1,
            close_requested: false,
            visible: true,
            focused: false,
            minimized: false,
            maximized: false,
            occluded: false,
            inner_rect: None,
            outer_rect: None,
            native_pixels_per_point: None,
        };
        let error = viewport_capture_support_error(&viewport)
            .expect("immediate child capture must fail explicitly");
        assert_eq!(error.code, "viewport-capture-unsupported");
        assert!(error.message.contains("eframe 0.35"));
    }

    #[test]
    fn deferred_viewport_capture_fails_before_renderer_timeout() {
        let viewport = ViewportDescriptor {
            id: ViewportId::new("chat.primary#1"),
            key: protocol::PilotViewportKey::new("chat.primary"),
            parent: Some(ViewportId::new("root")),
            class: protocol::ViewportClass::Deferred,
            title: "DevPULSE Chat".into(),
            lifecycle: protocol::ViewportLifecycle::Open,
            lifecycle_revision: 1,
            close_requested: false,
            visible: true,
            focused: true,
            minimized: false,
            maximized: false,
            occluded: false,
            inner_rect: None,
            outer_rect: None,
            native_pixels_per_point: None,
        };
        let error = viewport_capture_support_error(&viewport)
            .expect("deferred child capture must fail explicitly");
        assert_eq!(error.code, "viewport-capture-unsupported");
        assert!(error.message.contains("queued screenshot requests"));
    }

    #[test]
    fn committed_action_can_settle_after_native_focus_transfers() {
        let initial = ViewportDescriptor {
            id: ViewportId::new("chat.primary#1"),
            key: protocol::PilotViewportKey::new("chat.primary"),
            parent: Some(ViewportId::new("root")),
            class: protocol::ViewportClass::Deferred,
            title: "DevPULSE Chat".into(),
            lifecycle: protocol::ViewportLifecycle::Open,
            lifecycle_revision: 1,
            close_requested: false,
            visible: true,
            focused: true,
            minimized: false,
            maximized: false,
            occluded: false,
            inner_rect: None,
            outer_rect: None,
            native_pixels_per_point: None,
        };
        let mut transferred = initial.clone();
        transferred.focused = false;
        let mut root = initial.clone();
        root.id = ViewportId::new("root");
        root.key = protocol::PilotViewportKey::new("root");
        let frame = SemanticFrame {
            revision: 2,
            timestamp_seconds: 0.0,
            nodes: Vec::new(),
            viewports: vec![transferred],
            diagnostics: Default::default(),
        };

        assert!(viewport_focus_transferred(&initial, &frame));
        assert!(!viewport_focus_transferred(
            &initial,
            &SemanticFrame {
                viewports: vec![initial.clone()],
                ..frame.clone()
            }
        ));
        assert!(viewport_focus_transferred(
            &initial,
            &SemanticFrame {
                viewports: vec![initial.clone(), root],
                ..frame
            }
        ));
    }

    #[test]
    fn stop_waits_for_child_viewport_lifecycle_closure() {
        let (ctx, client, mut runtime) = test_runtime(32);
        authenticate(&client, &mut runtime);

        let child = egui::ViewportId::from_hash_of("stop-child");
        runtime.pilot().declare_viewport(
            child,
            protocol::PilotViewportKey::new("test.child"),
            protocol::ViewportClass::Immediate,
            "Child",
        );
        let mut with_child = egui::RawInput::default();
        with_child
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        with_child
            .viewports
            .insert(child, egui::ViewportInfo::default());
        ctx.run_ui(with_child, |_| {}).textures_delta.clear();

        client
            .try_send(CommandEnvelope {
                request_id: RequestId::new("stop"),
                session: SessionId::new("session"),
                command: ClientCommand::Stop,
            })
            .unwrap();
        runtime.pump();
        let before_close: Vec<_> = std::iter::from_fn(|| client.try_receive().ok().flatten())
            .take(8)
            .collect();
        assert!(!before_close.iter().any(|message| matches!(
            message,
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Success { result },
                    ..
                }
            } if matches!(result.as_ref(), CommandResult::Stopped)
        )));

        let mut root_only = egui::RawInput::default();
        root_only
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        ctx.run_ui(root_only, |_| {}).textures_delta.clear();
        runtime.pump();
        let after_close: Vec<_> = std::iter::from_fn(|| client.try_receive().ok().flatten())
            .take(8)
            .collect();
        assert!(after_close.iter().any(|message| matches!(
            message,
            ServerMessage::Response {
                envelope: ResponseEnvelope {
                    outcome: ResponseOutcome::Success { result },
                    ..
                }
            } if matches!(result.as_ref(), CommandResult::Stopped)
        )));
        assert!(runtime.is_stopped());
    }
}
