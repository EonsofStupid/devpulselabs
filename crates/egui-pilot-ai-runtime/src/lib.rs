//! Transport-independent EguiPilotAI application runtime.
//!
//! An [`RuntimeGuard`] is owned by the application, so its `egui::Context`
//! clone and transport workers end with the application. The egui plugin
//! itself never retains a context.

use egui_pilot_ai::{install, MotionPolicy, PilotHandle, QueuedInput};
pub use egui_pilot_ai_protocol as protocol;
use egui_pilot_ai_transport::{Transport, TransportError};
use protocol::{
    Action, ActionKind, AgentSnapshot, CaptureResult, ClientCommand, ClientEnvelope, Expectation,
    Flow, Handshake, Locator, NodeId, PilotError, PointerButton, SemanticFrame, SemanticNode,
    ServerEnvelope, ServerEvent, SessionId, SessionStatus, SnapshotRequest, Step, StepOutcome,
    PROTOCOL_VERSION,
};
use std::collections::{HashMap, VecDeque};
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
            protocol_version: PROTOCOL_VERSION,
            build: self.build.clone(),
            session_nonce: self.session_nonce.clone(),
        }
    }
}

struct ActiveFlow {
    sequence: u64,
    flow: Flow,
    index: usize,
    wait_started: Option<Instant>,
}

struct ActionExecution {
    sequence: u64,
    step_index: usize,
    started: Instant,
    batches: VecDeque<QueuedInput>,
}

struct Stability {
    rect: protocol::Rect,
    revision: u64,
    frames: u8,
}

struct PendingCapture {
    sequence: u64,
    name: String,
    flow_step: Option<usize>,
    last_semantic_hash: Option<String>,
    last_geometry_hash: Option<String>,
    last_pixel_hash: Option<String>,
    attempts: u8,
}

pub struct RuntimeGuard {
    ctx: egui::Context,
    pilot: PilotHandle,
    transport: Box<dyn Transport>,
    config: RuntimeConfig,
    authenticated: bool,
    outbound: VecDeque<ServerEnvelope>,
    active_flow: Option<ActiveFlow>,
    action: Option<ActionExecution>,
    stability: HashMap<NodeId, Stability>,
    pending_capture: Option<PendingCapture>,
    capture_job: Option<Receiver<Result<CaptureResult, PilotError>>>,
    stopped: bool,
}

impl RuntimeGuard {
    pub fn attach(
        ctx: &egui::Context,
        transport: impl Transport + 'static,
        config: RuntimeConfig,
    ) -> Self {
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
            transport: Box::new(transport),
            config,
            authenticated: false,
            outbound: VecDeque::new(),
            active_flow: None,
            action: None,
            stability: HashMap::new(),
            pending_capture: None,
            capture_job: None,
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
        self.poll_capture_job();
        self.accept_captured_image();
        self.advance_action();
        self.advance_flow();

        for _ in 0..self.config.max_commands_per_frame {
            match self.transport.try_receive() {
                Ok(Some(envelope)) => self.handle_command(envelope),
                Ok(None) => break,
                Err(TransportError::Closed) => {
                    self.stopped = true;
                    break;
                }
                Err(error) => {
                    self.queue_error(0, "transport", error.to_string(), None);
                    break;
                }
            }
        }
        self.flush_outbound();
    }

    fn handle_command(&mut self, envelope: ClientEnvelope) {
        if envelope.session != self.config.session {
            self.queue_error(
                envelope.sequence,
                "session-mismatch",
                "command session does not match the target session".into(),
                None,
            );
            return;
        }
        if let ClientCommand::Handshake(handshake) = envelope.command {
            self.handshake(envelope.sequence, handshake);
            return;
        }
        if !self.authenticated {
            self.queue_error(
                envelope.sequence,
                "handshake-required",
                "a valid handshake is required before control commands".into(),
                None,
            );
            return;
        }
        match envelope.command {
            ClientCommand::Snapshot(request) => self.snapshot(envelope.sequence, &request),
            ClientCommand::Run(flow) => self.start_flow(envelope.sequence, flow),
            ClientCommand::Capture { name } => self.start_capture(envelope.sequence, name, None),
            ClientCommand::Status => self.status(envelope.sequence),
            ClientCommand::Stop => self.stop(envelope.sequence),
            ClientCommand::Handshake(_) => unreachable!("handshake handled above"),
        }
    }

    fn handshake(&mut self, sequence: u64, handshake: Handshake) {
        if handshake.protocol_version != PROTOCOL_VERSION {
            self.queue_error(
                sequence,
                "protocol-version",
                format!(
                    "target protocol is {PROTOCOL_VERSION}, client requested {}",
                    handshake.protocol_version
                ),
                None,
            );
            return;
        }
        if handshake.session_nonce != self.config.session_nonce {
            self.queue_error(
                sequence,
                "nonce-mismatch",
                "session nonce did not match the private child launch".into(),
                None,
            );
            return;
        }
        self.authenticated = true;
        self.queue(sequence, ServerEvent::Ready(self.config.handshake()));
    }

    fn snapshot(&mut self, sequence: u64, request: &SnapshotRequest) {
        match self.pilot.snapshot(request) {
            Ok(snapshot) => self.queue(sequence, ServerEvent::Snapshot(snapshot)),
            Err(error) => self.queue_error(sequence, "snapshot", error.to_string(), None),
        }
    }

    fn status(&mut self, sequence: u64) {
        let frame = self.pilot.latest_frame();
        let mut viewports: Vec<_> = frame.nodes.iter().map(|node| &node.viewport).collect();
        viewports.sort_by(|left, right| left.0.cmp(&right.0));
        viewports.dedup();
        self.queue(
            sequence,
            ServerEvent::Status(SessionStatus {
                running: !self.stopped,
                revision: frame.revision,
                pending_actions: self.pilot.pending_input() + usize::from(self.action.is_some()),
                busy_tokens: frame.diagnostics.busy_tokens.len(),
                viewport_count: viewports.len(),
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
            }),
        );
    }

    fn start_flow(&mut self, sequence: u64, flow: Flow) {
        if self.active_flow.is_some() || self.action.is_some() {
            self.queue_error(
                sequence,
                "busy",
                "another flow is already active".into(),
                self.failure_context(None),
            );
            return;
        }
        self.active_flow = Some(ActiveFlow {
            sequence,
            flow,
            index: 0,
            wait_started: None,
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
            return;
        };
        match step {
            Step::Action { action } => {
                match self.prepare_action(active.sequence, active.index, action) {
                    Ok(Some(execution)) => {
                        self.action = Some(execution);
                        self.active_flow = Some(active);
                        self.ctx.request_repaint();
                    }
                    Ok(None) => {
                        self.active_flow = Some(active);
                        self.ctx.request_repaint();
                    }
                    Err(error) => self.fail_flow(active.sequence, active.index, error),
                }
            }
            Step::Expect { expectation } => {
                let started = *active.wait_started.get_or_insert_with(Instant::now);
                match evaluate_expectation(&self.pilot.latest_frame(), &expectation) {
                    Ok(()) => {
                        self.queue(
                            active.sequence,
                            ServerEvent::Step(StepOutcome {
                                index: active.index,
                                passed: true,
                                elapsed_ms: started.elapsed().as_millis() as u64,
                                message: None,
                                context: None,
                            }),
                        );
                        active.index += 1;
                        active.wait_started = None;
                        self.active_flow = Some(active);
                        self.ctx.request_repaint();
                    }
                    Err(_) if started.elapsed() < Duration::from_secs(5) => {
                        self.active_flow = Some(active);
                        self.ctx.request_repaint_after(Duration::from_millis(1));
                    }
                    Err(error) => self.fail_flow(active.sequence, active.index, error),
                }
            }
            Step::Wait { milliseconds } => {
                let started = active.wait_started.get_or_insert_with(Instant::now);
                if started.elapsed() >= Duration::from_millis(milliseconds) {
                    self.queue(
                        active.sequence,
                        ServerEvent::Step(StepOutcome {
                            index: active.index,
                            passed: true,
                            elapsed_ms: started.elapsed().as_millis() as u64,
                            message: None,
                            context: None,
                        }),
                    );
                    active.index += 1;
                    active.wait_started = None;
                }
                self.active_flow = Some(active);
                self.ctx.request_repaint_after(Duration::from_millis(1));
            }
            Step::Capture { name } => {
                let index = active.index;
                let sequence = active.sequence;
                self.active_flow = Some(active);
                self.start_capture(sequence, name, Some(index));
            }
        }
    }

    fn prepare_action(
        &mut self,
        sequence: u64,
        step_index: usize,
        action: Action,
    ) -> Result<Option<ActionExecution>, PilotError> {
        let frame = self.pilot.latest_frame();
        let target = primary_target(&action)
            .map(|locator| resolve_locator(&frame, locator))
            .transpose()?;
        if let Some(node) = target {
            assert_actionable(node, &action)?;
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
            if stable.frames < 2 {
                return Ok(None);
            }
        }
        let batches = action_batches(&self.pilot, &frame, &action)?;
        Ok(Some(ActionExecution {
            sequence,
            step_index,
            started: Instant::now(),
            batches: batches.into(),
        }))
    }

    fn advance_action(&mut self) {
        let Some(mut action) = self.action.take() else {
            return;
        };
        if self.pilot.pending_input() > 0 {
            self.action = Some(action);
            self.ctx.request_repaint();
            return;
        }
        if let Some(batch) = action.batches.pop_front() {
            self.pilot.queue_input(batch);
            self.action = Some(action);
            self.ctx.request_repaint();
            return;
        }
        self.queue(
            action.sequence,
            ServerEvent::Step(StepOutcome {
                index: action.step_index,
                passed: true,
                elapsed_ms: action.started.elapsed().as_millis() as u64,
                message: None,
                context: None,
            }),
        );
        if let Some(active) = &mut self.active_flow {
            active.index += 1;
        }
        self.ctx.request_repaint();
    }

    fn start_capture(&mut self, sequence: u64, name: String, flow_step: Option<usize>) {
        if self.pending_capture.is_some() || self.capture_job.is_some() {
            self.queue_error(
                sequence,
                "capture-busy",
                "another capture is already active".into(),
                None,
            );
            return;
        }
        if let Err(motions) = self
            .pilot
            .prepare_motion(&self.ctx, MotionPolicy::Suppressed)
        {
            self.queue_error(
                sequence,
                "uncontrolled-motion",
                format!("capture blocked by uncontrolled motion: {motions:?}"),
                self.failure_context(None),
            );
            return;
        }
        self.pending_capture = Some(PendingCapture {
            sequence,
            name,
            flow_step,
            last_semantic_hash: None,
            last_geometry_hash: None,
            last_pixel_hash: None,
            attempts: 0,
        });
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        self.ctx.request_repaint();
    }

    fn accept_captured_image(&mut self) {
        let Some(capture) = self.pilot.take_capture() else {
            return;
        };
        let Some(mut pending) = self.pending_capture.take() else {
            return;
        };
        let frame = self.pilot.latest_frame();
        let semantic_hash = semantic_hash(&frame);
        let geometry_hash = geometry_hash(&frame);
        let pixel_hash = pixel_hash(capture.image.as_ref());
        let stable = pending.last_semantic_hash.as_ref() == Some(&semantic_hash)
            && pending.last_geometry_hash.as_ref() == Some(&geometry_hash)
            && pending.last_pixel_hash.as_ref() == Some(&pixel_hash);
        if !stable {
            pending.last_semantic_hash = Some(semantic_hash);
            pending.last_geometry_hash = Some(geometry_hash);
            pending.last_pixel_hash = Some(pixel_hash);
            pending.attempts = pending.attempts.saturating_add(1);
            if pending.attempts >= 30 {
                self.queue_error(
                    pending.sequence,
                    "unstable-capture",
                    "semantic, geometry, and pixel hashes did not settle within 30 frames".into(),
                    self.failure_context(None),
                );
                self.active_flow = None;
                return;
            }
            self.pending_capture = Some(pending);
            self.ctx
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.ctx.request_repaint();
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
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("eguipilotai-capture".into())
            .spawn(move || {
                let result = write_capture(&root, &name, &renderer_key, capture.image.as_ref());
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
                        self.queue(pending.sequence, ServerEvent::Capture(capture));
                        if let Some(index) = pending.flow_step {
                            self.queue(
                                pending.sequence,
                                ServerEvent::Step(StepOutcome {
                                    index,
                                    passed: true,
                                    elapsed_ms: 0,
                                    message: Some("capture stabilized and written".into()),
                                    context: None,
                                }),
                            );
                            if let Some(active) = &mut self.active_flow {
                                active.index += 1;
                            }
                        }
                    }
                    Err(error) => {
                        self.fail_flow(pending.sequence, pending.flow_step.unwrap_or(0), error)
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
                self.queue_error(
                    pending.sequence,
                    "capture-worker",
                    "capture worker disconnected".into(),
                    None,
                );
            }
        }
    }

    fn fail_flow(&mut self, sequence: u64, index: usize, mut error: PilotError) {
        if error.context.is_none() {
            error.context = self.failure_context(None).map(Box::new);
        }
        self.queue(
            sequence,
            ServerEvent::Step(StepOutcome {
                index,
                passed: false,
                elapsed_ms: 0,
                message: Some(error.message.clone()),
                context: error.context.clone(),
            }),
        );
        self.queue(sequence, ServerEvent::Error(error));
        self.active_flow = None;
        self.action = None;
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

    fn stop(&mut self, sequence: u64) {
        self.queue(sequence, ServerEvent::Stopped);
        self.flush_outbound();
        let _ = self.transport.close();
        self.stopped = true;
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn queue_error(
        &mut self,
        sequence: u64,
        code: &str,
        message: String,
        context: Option<AgentSnapshot>,
    ) {
        self.queue(
            sequence,
            ServerEvent::Error(PilotError {
                code: code.into(),
                message,
                context: context.map(Box::new),
            }),
        );
    }

    fn queue(&mut self, sequence: u64, event: ServerEvent) {
        self.outbound.push_back(ServerEnvelope {
            sequence,
            session: self.config.session.clone(),
            event,
        });
    }

    fn flush_outbound(&mut self) {
        while let Some(envelope) = self.outbound.pop_front() {
            match self.transport.try_send(envelope.clone()) {
                Ok(()) => {}
                Err(TransportError::Backpressure { .. }) => {
                    // Preserve ordering and retry next frame.
                    self.outbound.push_front(envelope);
                    break;
                }
                Err(_) => {
                    self.stopped = true;
                    break;
                }
            }
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

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        let _ = self.transport.close();
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
        Action::Press { .. } | Action::Scroll { .. } => None,
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
        Action::Press { .. } | Action::Scroll { .. } => None,
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
        Action::Press { key } => {
            let key = parse_key(key).ok_or_else(|| PilotError {
                code: "unknown-key".into(),
                message: format!("unsupported key `{key}`"),
                context: None,
            })?;
            let viewport = egui::ViewportId::ROOT;
            Ok(vec![batch(
                viewport,
                vec![
                    key_event(key, true, modifiers),
                    key_event(key, false, modifiers),
                ],
            )])
        }
        Action::Scroll { delta_x, delta_y } => Ok(vec![batch(
            egui::ViewportId::ROOT,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(*delta_x, *delta_y),
                phase: egui::TouchPhase::Move,
                modifiers,
            }],
        )]),
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
        .collect()
}

fn write_capture(
    root: &Path,
    name: &str,
    renderer_key: &str,
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
        name: name.into(),
        artifact: path.to_string_lossy().into_owned(),
        content_hash: hash,
        renderer_key: renderer_key.into(),
    })
}

fn semantic_hash(frame: &SemanticFrame) -> String {
    let mut nodes = frame.nodes.clone();
    for node in &mut nodes {
        node.changed_at = 0;
        node.rect = protocol::Rect::default();
        node.clip_rect = protocol::Rect::default();
    }
    protocol::stable_hash(&serde_json::to_vec(&nodes).unwrap_or_default())
}

fn geometry_hash(frame: &SemanticFrame) -> String {
    let geometry: Vec<_> = frame
        .nodes
        .iter()
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
            .try_send(ClientEnvelope {
                sequence: 1,
                session: SessionId::new("s"),
                command: ClientCommand::Status,
            })
            .unwrap();
        runtime.pump();
        let response = client.try_receive().unwrap().unwrap();
        assert!(matches!(response.event, ServerEvent::Error(_)));

        client
            .try_send(ClientEnvelope {
                sequence: 2,
                session: SessionId::new("s"),
                command: ClientCommand::Handshake(Handshake {
                    protocol_version: PROTOCOL_VERSION,
                    build: "runner".into(),
                    session_nonce: "n".into(),
                }),
            })
            .unwrap();
        runtime.pump();
        let response = client.try_receive().unwrap().unwrap();
        assert!(matches!(response.event, ServerEvent::Ready(_)));
    }
}
