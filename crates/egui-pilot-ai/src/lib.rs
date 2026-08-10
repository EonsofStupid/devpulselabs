//! Typed, agent-facing semantic instrumentation for egui.
//!
//! This crate contains no process or network transport. Installing the plugin
//! only makes an egui context observable and able to accept queued egui input.

pub mod coach;

use coach::CoachStore;
pub use coach::{CoachMount, CoachPalette};
pub use egui_pilot_ai_protocol as protocol;
use protocol::{
    ActionKind, CoachActor, CoachEvent, CoachId, CoachRefusal, CoachUnmountReason,
    FrameDiagnostics, MotionId as WireMotionId, NodeId, NodeState, Rect, Role, SemanticFrame,
    SemanticNode, SignalId, SnapshotError, SnapshotRequest, SourceLocation,
    ViewportClass as WireViewportClass, ViewportDescriptor, ViewportId, ViewportLifecycle,
    ViewportLifecycleEvent, ViewportSelector,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::panic::Location;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PilotId(&'static str);

impl PilotId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }

    pub fn path(self) -> PilotPath {
        PilotPath(self.0.to_string())
    }

    pub fn child(self, key: &PilotKey) -> PilotPath {
        self.path().child(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PilotKey(String);

impl PilotKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PilotPath(String);

impl PilotPath {
    pub fn child(&self, key: &PilotKey) -> Self {
        Self(format!("{}/{}", self.0, key.0))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<PilotId> for PilotPath {
    fn from(value: PilotId) -> Self {
        value.path()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MotionId(&'static str);

impl MotionId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionKind {
    Finite {
        terminal_seconds: f64,
    },
    Ambient {
        canonical_phase_seconds: Option<f64>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MotionPolicy {
    #[default]
    Natural,
    FastForward,
    Suppressed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionStatus {
    pub id: MotionId,
    pub kind: MotionKind,
    pub controlled: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ClockMode {
    #[default]
    Real,
    Fixed(f64),
    AutoStep {
        now: f64,
        step: f64,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PilotClock {
    mode: ClockMode,
    last: f64,
}

impl PilotClock {
    pub fn mode(self) -> ClockMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: ClockMode) {
        self.mode = mode;
    }

    pub fn now(self) -> f64 {
        match self.mode {
            ClockMode::Real => self.last,
            ClockMode::Fixed(now) | ClockMode::AutoStep { now, .. } => now,
        }
    }

    fn resolve_input(&mut self, real: Option<f64>) -> Option<f64> {
        match &mut self.mode {
            ClockMode::Real => {
                if let Some(now) = real {
                    self.last = now;
                }
                real
            }
            ClockMode::Fixed(now) => {
                self.last = *now;
                Some(*now)
            }
            ClockMode::AutoStep { now, step } => {
                let value = *now;
                *now += *step;
                self.last = value;
                Some(value)
            }
        }
    }
}

#[derive(Clone)]
pub struct QueuedInput {
    pub viewport: egui::ViewportId,
    pub events: Vec<egui::Event>,
    pub time: Option<f64>,
    pub modifiers: Option<egui::Modifiers>,
}

struct PendingInput {
    generation: u64,
    input: QueuedInput,
}

#[derive(Clone)]
pub struct CapturedImage {
    pub viewport: egui::ViewportId,
    pub image: Arc<egui::ColorImage>,
}

#[derive(Debug, Clone)]
pub struct NodeMetadata {
    pub path: PilotPath,
    pub parent: Option<PilotPath>,
    pub role: Role,
    pub name: String,
    pub description: Option<String>,
    pub value: Option<String>,
    pub actions: Vec<ActionKind>,
    pub decorative: bool,
    pub editable: bool,
    pub checked: Option<bool>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub error: Option<String>,
}

impl NodeMetadata {
    pub fn new(path: impl Into<PilotPath>, role: Role, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            parent: None,
            role,
            name: name.into(),
            description: None,
            value: None,
            actions: default_actions(role),
            decorative: false,
            editable: role == Role::TextInput,
            checked: None,
            selected: None,
            expanded: None,
            error: None,
        }
    }

    pub fn parent(mut self, path: impl Into<PilotPath>) -> Self {
        self.parent = Some(path.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Whether this node is unfolded.
    ///
    /// These three exist so a caller never has to pack two facts into one
    /// value string. `"active+open"` cannot be asked a question — neither
    /// "is it active" nor "is it open" is answerable without string surgery,
    /// and an author who writes `"active"` gets silence.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn decorative(mut self) -> Self {
        self.decorative = true;
        self
    }
}

fn default_actions(role: Role) -> Vec<ActionKind> {
    match role {
        Role::Button | Role::Link | Role::Tab | Role::MenuItem => {
            vec![ActionKind::Click, ActionKind::Hover, ActionKind::Focus]
        }
        Role::Checkbox => vec![ActionKind::Check, ActionKind::Uncheck, ActionKind::Focus],
        Role::Radio => vec![ActionKind::Select, ActionKind::Focus],
        Role::TextInput => vec![
            ActionKind::Focus,
            ActionKind::Fill,
            ActionKind::Clear,
            ActionKind::Type,
        ],
        Role::Slider => vec![ActionKind::Focus, ActionKind::SetValue, ActionKind::Drag],
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone)]
struct RegisteredNode {
    node: SemanticNode,
}

#[derive(Debug, Clone, Copy)]
struct MotionRegistration {
    id: MotionId,
    kind: MotionKind,
    controlled: bool,
}

#[derive(Default)]
struct ViewportFrame {
    nodes: Vec<RegisteredNode>,
    ids: HashMap<NodeId, &'static Location<'static>>,
}

#[derive(Debug, Clone)]
struct ViewportDeclaration {
    key: protocol::PilotViewportKey,
    class: WireViewportClass,
    title: String,
}

#[derive(Debug, Clone)]
struct LiveViewport {
    egui_id: egui::ViewportId,
    descriptor: ViewportDescriptor,
}

#[derive(Default)]
struct SharedState {
    revision: u64,
    viewport_revision: u64,
    staging: HashMap<egui::ViewportId, ViewportFrame>,
    committed: HashMap<egui::ViewportId, Vec<SemanticNode>>,
    latest: SemanticFrame,
    input: VecDeque<PendingInput>,
    next_input_generation: u64,
    viewport_pending_input_commit: HashMap<egui::ViewportId, u64>,
    viewport_committed_input_generation: HashMap<egui::ViewportId, u64>,
    captures: VecDeque<CapturedImage>,
    clock: PilotClock,
    motions: HashMap<MotionId, MotionRegistration>,
    busy: HashMap<u64, String>,
    next_busy: u64,
    declarations: HashMap<egui::ViewportId, ViewportDeclaration>,
    live_viewports: HashMap<egui::ViewportId, LiveViewport>,
    viewport_ids: HashMap<String, egui::ViewportId>,
    viewport_generations: HashMap<String, u64>,
    viewport_events: VecDeque<ViewportLifecycleEvent>,
    viewport_commands: VecDeque<(egui::ViewportId, egui::ViewportCommand)>,
    viewport_close_requests: HashSet<egui::ViewportId>,
    retired_viewports: HashSet<egui::ViewportId>,
    parent_wake_after_pass: HashMap<egui::ViewportId, u8>,
    viewport_semantic_revisions: HashMap<egui::ViewportId, u64>,
    motion_policy: MotionPolicy,
    coach: CoachStore,
    /// Viewports whose current pass received automation-injected input. A
    /// coach trigger in the same pass is the agent demonstrating, not the
    /// operator answering.
    synthetic_input: HashSet<egui::ViewportId>,
    /// The same fact for the APPLICATION's own handlers — but never
    /// consumed by the coach pass, so any number of widgets in the pass can
    /// ask "was this activation the operator's own hand?" (the provenance
    /// law: privileged transitions refuse agent-attributed input).
    /// Maintained at the input hook: present exactly while the viewport's
    /// current pass carries injected events.
    synthetic_frame: HashSet<egui::ViewportId>,
}

#[derive(Clone)]
pub struct PilotHandle {
    shared: Arc<Mutex<SharedState>>,
}

impl PilotHandle {
    pub fn latest_frame(&self) -> SemanticFrame {
        lock(&self.shared).latest.clone()
    }

    pub fn snapshot(
        &self,
        request: &SnapshotRequest,
    ) -> Result<protocol::AgentSnapshot, SnapshotError> {
        protocol::project_snapshot(&self.latest_frame(), request)
    }

    pub fn queue_input(&self, input: QueuedInput) -> u64 {
        let mut shared = lock(&self.shared);
        shared.next_input_generation = shared.next_input_generation.saturating_add(1);
        let generation = shared.next_input_generation;
        shared.input.push_back(PendingInput { generation, input });
        generation
    }

    pub fn pending_input(&self) -> usize {
        lock(&self.shared).input.len()
    }

    pub fn pending_input_viewports(&self) -> Vec<egui::ViewportId> {
        let shared = lock(&self.shared);
        let mut viewports: Vec<_> = shared
            .input
            .iter()
            .map(|pending| pending.input.viewport)
            .collect();
        viewports.sort_unstable();
        viewports.dedup();
        viewports
    }

    /// Returns the latest semantic revision committed by one viewport.
    ///
    /// The global frame revision can advance when a sibling native window
    /// paints. Action settlement must therefore use this viewport-scoped
    /// revision or an unrelated utility window could make a target appear
    /// settled before it has consumed its input.
    pub fn viewport_semantic_revision(&self, viewport: egui::ViewportId) -> Option<u64> {
        lock(&self.shared)
            .viewport_semantic_revisions
            .get(&viewport)
            .copied()
    }

    /// Returns the latest queued-input generation whose target viewport pass
    /// has fully committed semantics.
    pub fn viewport_committed_input_generation(&self, viewport: egui::ViewportId) -> u64 {
        lock(&self.shared)
            .viewport_committed_input_generation
            .get(&viewport)
            .copied()
            .unwrap_or_default()
    }

    /// Wake the application-owned parent coordinator only after `viewport`
    /// completes its next semantic pass.
    ///
    /// Deferred native viewports and their parent can otherwise race: waking
    /// the parent before the child consumes input lets egui coalesce away the
    /// completion wake.
    pub fn request_parent_wake_after_next_pass(&self, viewport: egui::ViewportId) {
        if viewport != egui::ViewportId::ROOT {
            lock(&self.shared)
                .parent_wake_after_pass
                .entry(viewport)
                .and_modify(|remaining| *remaining = (*remaining).max(2))
                .or_insert(2);
        }
    }

    /// Discards automation input that has not yet been injected into an egui frame.
    ///
    /// The runtime calls this on cancellation and shutdown so a later session can
    /// never inherit a queued press, release, key, or text event.
    pub fn clear_input(&self) {
        lock(&self.shared).input.clear();
    }

    pub fn egui_viewport(&self, viewport: &ViewportId) -> Option<egui::ViewportId> {
        lock(&self.shared).viewport_ids.get(&viewport.0).copied()
    }

    pub fn viewports(&self) -> Vec<ViewportDescriptor> {
        let mut viewports: Vec<_> = lock(&self.shared)
            .live_viewports
            .values()
            .map(|live| live.descriptor.clone())
            .collect();
        viewports.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        viewports
    }

    pub fn resolve_viewport(
        &self,
        selector: &ViewportSelector,
    ) -> Result<(ViewportDescriptor, egui::ViewportId), String> {
        let shared = lock(&self.shared);
        let matches: Vec<_> = shared
            .live_viewports
            .values()
            .filter(|live| {
                selector
                    .id
                    .as_ref()
                    .is_none_or(|id| id == &live.descriptor.id)
                    && selector
                        .key
                        .as_ref()
                        .is_none_or(|key| key == &live.descriptor.key)
            })
            .collect();
        match matches.as_slice() {
            [live] => Ok((live.descriptor.clone(), live.egui_id)),
            [] => Err(format!(
                "viewport selector matched no live viewport: {selector:?}"
            )),
            matches => Err(format!(
                "viewport selector matched {} live viewports: {}",
                matches.len(),
                matches
                    .iter()
                    .map(|live| live.descriptor.id.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    pub fn only_viewport(&self) -> Result<(ViewportDescriptor, egui::ViewportId), String> {
        self.resolve_viewport(&ViewportSelector::default())
    }

    pub fn take_viewport_events(&self) -> Vec<ViewportLifecycleEvent> {
        lock(&self.shared).viewport_events.drain(..).collect()
    }

    pub fn declare_viewport(
        &self,
        egui_id: egui::ViewportId,
        key: protocol::PilotViewportKey,
        class: WireViewportClass,
        title: impl Into<String>,
    ) {
        let mut shared = lock(&self.shared);
        let declaration = ViewportDeclaration {
            key,
            class,
            title: title.into(),
        };
        if let Some(existing) = shared.declarations.get(&egui_id) {
            assert_eq!(
                existing.key, declaration.key,
                "egui viewport id reused for a different PilotViewportKey"
            );
            assert_eq!(
                existing.class, declaration.class,
                "egui viewport id reused for a different viewport class"
            );
        }
        shared.declarations.insert(egui_id, declaration);
    }

    pub fn queue_viewport_command(
        &self,
        viewport: egui::ViewportId,
        command: egui::ViewportCommand,
    ) {
        lock(&self.shared)
            .viewport_commands
            .push_back((viewport, command));
    }

    /// Ask the application-owned deferred viewport registry to stop declaring
    /// this viewport. Native `Close` commands alone are insufficient for a
    /// deferred viewport because its owner may immediately declare it again.
    pub fn request_viewport_close(&self, viewport: egui::ViewportId) {
        lock(&self.shared).viewport_close_requests.insert(viewport);
    }

    /// Retire a viewport that the application has intentionally stopped
    /// declaring or is replacing with a new native generation.
    ///
    /// Native integrations can continue delivering a few frames for the old
    /// OS window after `Close`. Keeping that stale lifetime live would make
    /// action settlement wait forever even though its final input pass
    /// committed. The retired id remains ignored until the integration drops
    /// it from `RawInput::viewports`.
    pub fn retire_viewport(&self, viewport: egui::ViewportId) {
        let mut shared = lock(&self.shared);
        let pending_commit = shared.viewport_pending_input_commit.get(&viewport).copied();
        close_viewport(&mut shared, viewport);
        if let Some(generation) = pending_commit {
            shared
                .viewport_pending_input_commit
                .insert(viewport, generation);
        }
        shared.retired_viewports.insert(viewport);
    }

    fn take_viewport_close_request(&self, viewport: egui::ViewportId) -> bool {
        lock(&self.shared).viewport_close_requests.remove(&viewport)
    }

    fn take_viewport_commands(&self, viewport: egui::ViewportId) -> Vec<egui::ViewportCommand> {
        let mut shared = lock(&self.shared);
        let mut commands = Vec::new();
        while let Some(index) = shared
            .viewport_commands
            .iter()
            .position(|(target, _)| *target == viewport)
        {
            let (_, command) = shared
                .viewport_commands
                .remove(index)
                .expect("viewport command index");
            commands.push(command);
        }
        commands
    }

    pub fn pending_viewport_commands(&self) -> usize {
        lock(&self.shared).viewport_commands.len()
    }

    pub fn take_capture(&self) -> Option<CapturedImage> {
        lock(&self.shared).captures.pop_front()
    }

    pub fn take_capture_for(&self, viewport: egui::ViewportId) -> Option<CapturedImage> {
        let mut shared = lock(&self.shared);
        let index = shared
            .captures
            .iter()
            .position(|capture| capture.viewport == viewport)?;
        shared.captures.remove(index)
    }

    /// Discards screenshot events left over from an earlier capture request.
    pub fn clear_captures(&self) {
        lock(&self.shared).captures.clear();
    }

    pub fn clear_captures_for(&self, viewport: egui::ViewportId) {
        lock(&self.shared)
            .captures
            .retain(|capture| capture.viewport != viewport);
    }

    /// Hand the renderer the host's paint tokens. DevPULSE supplies the
    /// generated ColdLight tones; the toolkit default is neutral.
    pub fn set_coach_palette(&self, palette: CoachPalette) {
        lock(&self.shared).coach.set_palette(palette);
    }

    /// Mount a touchpoint on an already-resolved semantic node.
    ///
    /// Resolution belongs to the caller because it owns the locator
    /// machinery; the store only guarantees identity is unique and that
    /// every mount terminates in exactly one unmount.
    pub fn mount_coach(&self, request: CoachMount) -> Result<(), CoachRefusal> {
        let mut shared = lock(&self.shared);
        shared.coach.mount(request)
    }

    pub fn unmount_coach(&self, coach_id: &CoachId, reason: CoachUnmountReason) -> bool {
        lock(&self.shared).coach.unmount(coach_id, reason)
    }

    pub fn unmount_all_coach(&self, reason: CoachUnmountReason) -> Vec<CoachId> {
        lock(&self.shared).coach.unmount_all(reason)
    }

    pub fn take_coach_events(&self) -> Vec<CoachEvent> {
        lock(&self.shared).coach.take_events()
    }

    /// Install a witness on every coach lifecycle event, invoked at record
    /// time — before the event lands in the drain queue, independent of who
    /// drains it. The host bridges these onto its own signal fabric.
    ///
    /// A witness, never a consumer: `take_coach_events` still returns every
    /// event, so the devtools pipe and the host cannot steal from each other.
    /// `Refused` never reaches the tap — a refusal is a command result, not
    /// a lifecycle fact.
    pub fn set_coach_event_tap(&self, tap: impl Fn(&CoachEvent) + Send + Sync + 'static) {
        lock(&self.shared).coach.set_tap(std::sync::Arc::new(tap));
    }

    /// Whether this viewport's current pass carries automation-injected
    /// input. Unlike the coach's one-shot attribution flag, this query is
    /// stable for the whole pass — any number of privileged-mode handlers
    /// can ask, and none of them consume the answer.
    pub fn frame_input_synthetic(&self, viewport: egui::ViewportId) -> bool {
        lock(&self.shared).synthetic_frame.contains(&viewport)
    }

    pub fn mounted_coach_count(&self) -> usize {
        lock(&self.shared).coach.len()
    }

    /// Publish an application signal a touchpoint may be waiting on.
    ///
    /// One-way and app-chosen: the toolkit never reads the application's
    /// signal fabric, so an agent can only wait for what the app decided to
    /// expose here.
    pub fn note_coach_signal(&self, signal: SignalId) {
        lock(&self.shared).coach.note_signal(signal);
    }

    /// Declare every signal this host can publish, so an agent can discover
    /// what a `FabricEvent` touchpoint may wait for instead of being told.
    ///
    /// The toolkit cannot derive this list and does not police it: a host that
    /// declares nothing simply reports nothing waitable.
    pub fn declare_coach_signals(&self, signals: Vec<SignalId>) {
        lock(&self.shared).coach.declare_signals(signals);
    }

    pub fn coach_signals(&self) -> Vec<SignalId> {
        lock(&self.shared).coach.declared_signals()
    }

    pub fn set_clock_mode(&self, mode: ClockMode) {
        lock(&self.shared).clock.set_mode(mode);
    }

    pub fn clock(&self) -> PilotClock {
        lock(&self.shared).clock
    }

    pub fn register_motion(&self, id: MotionId, kind: MotionKind) {
        lock(&self.shared).motions.insert(
            id,
            MotionRegistration {
                id,
                kind,
                controlled: true,
            },
        );
    }

    pub fn mark_uncontrolled_motion(&self, id: MotionId, kind: MotionKind) {
        lock(&self.shared).motions.insert(
            id,
            MotionRegistration {
                id,
                kind,
                controlled: false,
            },
        );
    }

    pub fn motion_status(&self) -> Vec<MotionStatus> {
        lock(&self.shared)
            .motions
            .values()
            .map(|motion| MotionStatus {
                id: motion.id,
                kind: motion.kind,
                controlled: motion.controlled,
            })
            .collect()
    }

    pub fn motion_policy(&self) -> MotionPolicy {
        lock(&self.shared).motion_policy
    }

    pub fn begin_busy(&self, label: impl Into<String>) -> BusyToken {
        let mut shared = lock(&self.shared);
        let id = shared.next_busy;
        shared.next_busy += 1;
        shared.busy.insert(id, label.into());
        BusyToken {
            shared: Arc::downgrade(&self.shared),
            id,
        }
    }

    pub fn prepare_motion(
        &self,
        ctx: &egui::Context,
        policy: MotionPolicy,
    ) -> Result<(), Vec<MotionId>> {
        let mut shared = lock(&self.shared);
        let uncontrolled: Vec<MotionId> = shared
            .motions
            .values()
            .filter(|motion| {
                !motion.controlled
                    || (policy == MotionPolicy::Suppressed
                        && matches!(
                            motion.kind,
                            MotionKind::Ambient {
                                canonical_phase_seconds: None
                            }
                        ))
            })
            .map(|motion| motion.id)
            .collect();
        if !uncontrolled.is_empty() {
            return Err(uncontrolled);
        }

        shared.motion_policy = policy;
        match policy {
            MotionPolicy::Natural => {}
            MotionPolicy::FastForward => {
                let advance = shared
                    .motions
                    .values()
                    .filter_map(|motion| match motion.kind {
                        MotionKind::Finite { terminal_seconds } => Some(terminal_seconds),
                        MotionKind::Ambient { .. } => None,
                    })
                    .fold(1.0f64, f64::max);
                let now = shared.clock.now() + advance + 1.0;
                shared.clock.set_mode(ClockMode::Fixed(now));
            }
            MotionPolicy::Suppressed => {
                let canonical = shared
                    .motions
                    .values()
                    .filter_map(|motion| match motion.kind {
                        MotionKind::Finite { terminal_seconds } => Some(terminal_seconds + 1.0),
                        MotionKind::Ambient {
                            canonical_phase_seconds,
                        } => canonical_phase_seconds,
                    })
                    .fold(shared.clock.now(), f64::max);
                shared.clock.set_mode(ClockMode::Fixed(canonical));
                ctx.all_styles_mut(|style| style.animation_time = 0.0);
                ctx.clear_animations();
            }
        }
        Ok(())
    }
}

pub struct BusyToken {
    shared: Weak<Mutex<SharedState>>,
    id: u64,
}

impl Drop for BusyToken {
    fn drop(&mut self) {
        if let Some(shared) = self.shared.upgrade() {
            lock(&shared).busy.remove(&self.id);
        }
    }
}

pub struct EguiPilotPlugin {
    shared: Arc<Mutex<SharedState>>,
}

impl EguiPilotPlugin {
    fn new() -> (Self, PilotHandle) {
        let shared = Arc::new(Mutex::new(SharedState::default()));
        (
            Self {
                shared: Arc::clone(&shared),
            },
            PilotHandle { shared },
        )
    }

    fn handle(&self) -> PilotHandle {
        PilotHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    #[track_caller]
    fn register(&mut self, ui: &egui::Ui, response: &egui::Response, metadata: NodeMetadata) {
        let source = Location::caller();
        let viewport = ui.ctx().viewport_id();
        if lock(&self.shared).retired_viewports.contains(&viewport) {
            return;
        }
        let (viewport_lifetime, viewport_key) = {
            let mut shared = lock(&self.shared);
            let lifetime = ensure_viewport(&mut shared, viewport, None);
            let key = shared
                .live_viewports
                .get(&viewport)
                .expect("registered viewport")
                .descriptor
                .key
                .clone();
            (lifetime, key)
        };
        let id = NodeId::new(format!("{viewport_key}/{}", metadata.path.as_str()));
        let parent = metadata
            .parent
            .as_ref()
            .map(|path| NodeId::new(format!("{viewport_key}/{}", path.as_str())));
        let clip_rect = ui.clip_rect();
        let visible = ui.is_rect_visible(response.rect);
        let node = SemanticNode {
            id: id.clone(),
            viewport: viewport_lifetime,
            parent,
            role: metadata.role,
            name: metadata.name,
            description: metadata.description,
            value: metadata.value,
            rect: rect(response.interact_rect),
            clip_rect: rect(clip_rect),
            state: NodeState {
                hidden: !visible,
                clipped: !response.interact_rect.intersects(clip_rect),
                decorative: metadata.decorative,
                disabled: !response.enabled(),
                editable: metadata.editable,
                // Logical widget focus is stable even when a native deferred
                // viewport temporarily reports `RawInput::focused = false`.
                // Native OS focus remains available on the viewport descriptor.
                focused: ui.memory(|memory| memory.has_focus(response.id)),
                checked: metadata.checked,
                selected: metadata.selected,
                expanded: metadata.expanded,
                error: metadata.error,
            },
            actions: metadata.actions,
            layer: 0,
            changed_at: 0,
            source: Some(SourceLocation {
                file: source.file().to_string(),
                line: source.line(),
                column: source.column(),
            }),
        };

        let mut shared = lock(&self.shared);
        let frame = shared.staging.entry(viewport).or_default();
        if let Some(previous) = frame.ids.get(&id) {
            panic!(
                "duplicate EguiPilotAI id `{id}` at {}:{}:{}; first registered at {}:{}:{}",
                source.file(),
                source.line(),
                source.column(),
                previous.file(),
                previous.line(),
                previous.column()
            );
        }
        frame.ids.insert(id, source);
        frame.nodes.push(RegisteredNode { node });
    }
}

impl egui::Plugin for EguiPilotPlugin {
    fn debug_name(&self) -> &'static str {
        "EguiPilotAI"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        ctx.enable_accesskit();
    }

    fn on_begin_pass(&mut self, ui: &mut egui::Ui) {
        lock(&self.shared)
            .staging
            .insert(ui.ctx().viewport_id(), ViewportFrame::default());
    }

    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        let viewport = ui.ctx().viewport_id();
        let time = ui.input(|input| input.time);
        let mut shared = lock(&self.shared);
        if shared.retired_viewports.contains(&viewport) {
            shared.staging.remove(&viewport);
            if let Some(generation) = shared.viewport_pending_input_commit.remove(&viewport) {
                shared
                    .viewport_committed_input_generation
                    .insert(viewport, generation);
            }
            let wake_parent = shared
                .parent_wake_after_pass
                .remove(&viewport)
                .and_then(|_| {
                    ui.input(|input| input.viewport().parent)
                        .or(Some(egui::ViewportId::ROOT))
                });
            drop(shared);
            if let Some(parent) = wake_parent {
                ui.ctx().request_repaint_of(parent);
            }
            return;
        }
        let staged = shared.staging.remove(&viewport).unwrap_or_default();
        shared.revision += 1;
        let revision = shared.revision;
        let previous: HashMap<NodeId, SemanticNode> = shared
            .committed
            .get(&viewport)
            .into_iter()
            .flatten()
            .map(|node| (node.id.clone(), node.clone()))
            .collect();
        let mut nodes: Vec<SemanticNode> = staged
            .nodes
            .into_iter()
            .map(|registered| {
                let mut node = registered.node;
                node.changed_at = previous
                    .get(&node.id)
                    .filter(|old| semantically_equal(old, &node))
                    .map_or(revision, |old| old.changed_at);
                node
            })
            .collect();
        nodes.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        shared.committed.insert(viewport, nodes);
        let mut all_nodes: Vec<SemanticNode> =
            shared.committed.values().flatten().cloned().collect();
        all_nodes.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let diagnostics = FrameDiagnostics {
            busy_tokens: shared.busy.values().cloned().collect(),
            uncontrolled_motion: shared
                .motions
                .values()
                .filter(|motion| !motion.controlled)
                .map(|motion| WireMotionId::new(motion.id.0))
                .collect(),
            ..shared.latest.diagnostics.clone()
        };
        shared.latest = SemanticFrame {
            revision,
            timestamp_seconds: time,
            nodes: all_nodes,
            viewports: live_descriptors(&shared),
            diagnostics,
        };
        shared
            .viewport_semantic_revisions
            .insert(viewport, revision);
        coach_pass(&mut shared, ui, viewport);
        if let Some(generation) = shared.viewport_pending_input_commit.remove(&viewport) {
            shared
                .viewport_committed_input_generation
                .insert(viewport, generation);
        }
        let wake_parent = shared
            .parent_wake_after_pass
            .get_mut(&viewport)
            .map(|remaining| {
                *remaining = remaining.saturating_sub(1);
                *remaining
            });
        if wake_parent == Some(0) {
            shared.parent_wake_after_pass.remove(&viewport);
        }
        let wake_parent = wake_parent.map(|_| {
            shared
                .live_viewports
                .get(&viewport)
                .and_then(|live| live.descriptor.parent.as_ref())
                .and_then(|parent| shared.viewport_ids.get(&parent.0))
                .copied()
                .unwrap_or(egui::ViewportId::ROOT)
        });
        drop(shared);
        if let Some(parent) = wake_parent {
            ui.ctx().request_repaint_of(parent);
        }
    }

    fn input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let mut shared = lock(&self.shared);
        sync_viewports(&mut shared, input);
        input.time = shared.clock.resolve_input(input.time);
        for event in &input.events {
            if let egui::Event::Screenshot {
                viewport_id, image, ..
            } = event
            {
                shared.captures.push_back(CapturedImage {
                    viewport: *viewport_id,
                    image: Arc::clone(image),
                });
            }
        }
        if let Some(index) = shared
            .input
            .iter()
            .position(|pending| pending.input.viewport == input.viewport_id)
        {
            let pending = shared.input.remove(index).expect("queued input index");
            let queued = pending.input;
            // Targeted automation input represents an active interaction with
            // this native viewport. Preserve the real OS focus command in the
            // runtime, while ensuring egui does not discard the injected
            // RawInput merely because a sibling peer window still owned the
            // platform focus at the start of this pass.
            input.focused = true;
            if let Some(viewport) = input.viewports.get_mut(&input.viewport_id) {
                viewport.focused = Some(true);
            }
            if !queued.events.is_empty() {
                shared.synthetic_input.insert(input.viewport_id);
                shared.synthetic_frame.insert(input.viewport_id);
            } else {
                shared.synthetic_frame.remove(&input.viewport_id);
            }
            // egui 0.36 REMOVED `RawInput::modifiers`. Modifiers now ride on
            // the events that carry them, so a queued modifier state has to be
            // stamped onto the injected events instead of set once globally —
            // otherwise a synthetic Ctrl+Z arrives as a bare Z.
            let mut events = queued.events;
            if let Some(modifiers) = queued.modifiers {
                for event in &mut events {
                    match event {
                        egui::Event::Key {
                            modifiers: stamp, ..
                        }
                        | egui::Event::PointerButton {
                            modifiers: stamp, ..
                        }
                        | egui::Event::MouseWheel {
                            modifiers: stamp, ..
                        } => *stamp = modifiers,
                        _ => {}
                    }
                }
            }
            input.events.extend(events);
            if let Some(time) = queued.time {
                input.time = Some(time);
            }
            shared
                .viewport_pending_input_commit
                .insert(input.viewport_id, pending.generation);
            // Guarantee one settling pass after the input pass. Combined with
            // the two-pass parent wake barrier, this prevents either native
            // redraw queue from losing the final one-tick hover/key action.
            ctx.request_repaint_of(input.viewport_id);
        } else {
            // A pass with no queued injection is not synthetic, whatever
            // the previous pass was.
            shared.synthetic_frame.remove(&input.viewport_id);
        }
    }

    fn output_hook(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput) {
        let mut shared = lock(&self.shared);
        shared.latest.diagnostics.output_events = output.platform_output.events.len();
        shared.latest.diagnostics.accesskit_updated =
            output.platform_output.accesskit_update.is_some();
        let viewport = ctx.viewport_id();
        if let Some(viewport_output) = output.viewport_output.get_mut(&viewport) {
            while let Some(index) = shared
                .viewport_commands
                .iter()
                .position(|(target, _)| *target == viewport)
            {
                let (_, command) = shared
                    .viewport_commands
                    .remove(index)
                    .expect("viewport command index");
                viewport_output.commands.push(command);
            }
        }
    }
}

/// Expire, paint, and observe this viewport's mounted touchpoints.
///
/// The overlay is a bare foreground layer with no interaction area, so the
/// press the operator is coached to make still lands on the real widget; the
/// trigger is read out of the same frame's pointer events.
fn coach_pass(shared: &mut SharedState, ui: &egui::Ui, viewport: egui::ViewportId) {
    let synthetic = shared.synthetic_input.remove(&viewport);
    if !shared.coach.targets_viewport(viewport) {
        return;
    }
    let (now, presses, pointer, had_input) = ui.input(|input| {
        (
            input.time,
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } => Some(*pos),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            input.pointer.latest_pos(),
            !input.events.is_empty(),
        )
    });
    // Attribution is derived from what moved this frame: injected input is the
    // agent demonstrating, real input is the operator, and a frame with no
    // input at all cannot honestly be credited to either.
    let input_actor = match (synthetic, had_input) {
        (true, _) => Some(CoachActor::Agent),
        (false, true) => Some(CoachActor::Operator),
        (false, false) => None,
    };
    let nodes = shared
        .committed
        .get(&viewport)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("eguipilotai.coach"),
    ));
    let frame = coach::CoachFrame {
        viewport,
        screen: ui.ctx().viewport_rect(),
        now,
        nodes,
        presses: &presses,
        pointer,
        input_actor,
    };
    coach::run_pass(&mut shared.coach, &painter, &frame);
}

fn sync_viewports(shared: &mut SharedState, input: &egui::RawInput) {
    if input.viewport_id == egui::ViewportId::ROOT {
        shared
            .retired_viewports
            .retain(|viewport| input.viewports.contains_key(viewport));
        let observed: Vec<_> = input
            .viewports
            .keys()
            .filter(|viewport| !shared.retired_viewports.contains(viewport))
            .copied()
            .collect();
        for viewport in &observed {
            let info = input.viewports.get(viewport);
            ensure_viewport(shared, *viewport, info);
        }
        for viewport in &observed {
            if let Some(info) = input.viewports.get(viewport) {
                update_viewport(shared, *viewport, info);
            }
        }

        let closed: Vec<_> = shared
            .live_viewports
            .keys()
            .filter(|viewport| **viewport != egui::ViewportId::ROOT)
            .filter(|viewport| !input.viewports.contains_key(viewport))
            .copied()
            .collect();
        for viewport in closed {
            close_viewport(shared, viewport);
        }
    } else {
        if shared.retired_viewports.contains(&input.viewport_id) {
            return;
        }
        let info = input.viewports.get(&input.viewport_id);
        ensure_viewport(shared, input.viewport_id, info);
        if let Some(info) = info {
            update_viewport(shared, input.viewport_id, info);
        }
    }
}

fn ensure_viewport(
    shared: &mut SharedState,
    egui_id: egui::ViewportId,
    info: Option<&egui::ViewportInfo>,
) -> ViewportId {
    if let Some(live) = shared.live_viewports.get(&egui_id) {
        return live.descriptor.id.clone();
    }

    let declaration = if egui_id == egui::ViewportId::ROOT {
        ViewportDeclaration {
            key: protocol::PilotViewportKey::new("root"),
            class: WireViewportClass::Root,
            title: info
                .and_then(|value| value.title.clone())
                .unwrap_or_else(|| "Root".into()),
        }
    } else {
        shared
            .declarations
            .get(&egui_id)
            .cloned()
            .unwrap_or_else(|| ViewportDeclaration {
                key: protocol::PilotViewportKey::new(format!(
                    "egui.unregistered.{}",
                    sanitize_viewport_debug(egui_id)
                )),
                class: WireViewportClass::Embedded,
                title: info
                    .and_then(|value| value.title.clone())
                    .unwrap_or_default(),
            })
    };
    if let Some(existing) = shared
        .live_viewports
        .values()
        .find(|live| live.egui_id != egui_id && live.descriptor.key == declaration.key)
    {
        panic!(
            "duplicate live EguiPilotAI viewport key `{}`: existing lifetime `{}` and egui viewport {egui_id:?}",
            declaration.key, existing.descriptor.id
        );
    }
    let generation = if egui_id == egui::ViewportId::ROOT {
        1
    } else {
        let generation = shared
            .viewport_generations
            .entry(declaration.key.0.clone())
            .or_default();
        *generation = generation.saturating_add(1);
        *generation
    };
    let id = if egui_id == egui::ViewportId::ROOT {
        ViewportId::new("root")
    } else {
        ViewportId::new(format!("{}#{generation}", declaration.key))
    };
    shared.viewport_revision = shared.viewport_revision.saturating_add(1);
    let parent = info.and_then(|value| value.parent).and_then(|parent| {
        shared
            .live_viewports
            .get(&parent)
            .map(|live| live.descriptor.id.clone())
    });
    let descriptor = ViewportDescriptor {
        id: id.clone(),
        key: declaration.key,
        parent,
        class: declaration.class,
        title: declaration.title,
        lifecycle: ViewportLifecycle::Open,
        lifecycle_revision: shared.viewport_revision,
        close_requested: false,
        visible: info.and_then(egui::ViewportInfo::visible).unwrap_or(true),
        focused: info.and_then(|value| value.focused).unwrap_or(false),
        minimized: info.and_then(|value| value.minimized).unwrap_or(false),
        maximized: info.and_then(|value| value.maximized).unwrap_or(false),
        occluded: info.and_then(|value| value.occluded).unwrap_or(false),
        inner_rect: info.and_then(|value| value.inner_rect).map(rect),
        outer_rect: info.and_then(|value| value.outer_rect).map(rect),
        native_pixels_per_point: info.and_then(|value| value.native_pixels_per_point),
    };
    shared.viewport_ids.insert(id.0.clone(), egui_id);
    shared.live_viewports.insert(
        egui_id,
        LiveViewport {
            egui_id,
            descriptor: descriptor.clone(),
        },
    );
    shared.viewport_events.push_back(ViewportLifecycleEvent {
        kind: protocol::ViewportEventKind::Created,
        viewport: descriptor,
        state: ViewportLifecycle::Open,
    });
    id
}

fn update_viewport(shared: &mut SharedState, egui_id: egui::ViewportId, info: &egui::ViewportInfo) {
    let parent = info.parent.and_then(|parent| {
        shared
            .live_viewports
            .get(&parent)
            .map(|live| live.descriptor.id.clone())
    });
    let Some(live) = shared.live_viewports.get_mut(&egui_id) else {
        return;
    };
    let previous = live.descriptor.clone();
    if let Some(title) = &info.title {
        live.descriptor.title.clone_from(title);
    }
    live.descriptor.parent = parent;
    live.descriptor.visible = info.visible().unwrap_or(true);
    live.descriptor.focused = info.focused.unwrap_or(false);
    live.descriptor.minimized = info.minimized.unwrap_or(false);
    live.descriptor.maximized = info.maximized.unwrap_or(false);
    live.descriptor.occluded = info.occluded.unwrap_or(false);
    live.descriptor.inner_rect = info.inner_rect.map(rect);
    live.descriptor.outer_rect = info.outer_rect.map(rect);
    live.descriptor.native_pixels_per_point = info.native_pixels_per_point;
    live.descriptor.close_requested = info.close_requested();
    live.descriptor.lifecycle = if live.descriptor.close_requested {
        ViewportLifecycle::CloseRequested
    } else {
        ViewportLifecycle::Open
    };
    if previous != live.descriptor {
        shared.viewport_revision = shared.viewport_revision.saturating_add(1);
        live.descriptor.lifecycle_revision = shared.viewport_revision;
        let kind = if !previous.close_requested && live.descriptor.close_requested {
            protocol::ViewportEventKind::CloseRequested
        } else if !previous.focused && live.descriptor.focused {
            protocol::ViewportEventKind::Focused
        } else {
            protocol::ViewportEventKind::Changed
        };
        shared.viewport_events.push_back(ViewportLifecycleEvent {
            kind,
            viewport: live.descriptor.clone(),
            state: live.descriptor.lifecycle,
        });
    }
}

fn close_viewport(shared: &mut SharedState, egui_id: egui::ViewportId) {
    let Some(mut live) = shared.live_viewports.remove(&egui_id) else {
        return;
    };
    shared.viewport_revision = shared.viewport_revision.saturating_add(1);
    live.descriptor.lifecycle = ViewportLifecycle::Closed;
    live.descriptor.lifecycle_revision = shared.viewport_revision;
    shared.viewport_ids.remove(&live.descriptor.id.0);
    shared.staging.remove(&egui_id);
    shared.committed.remove(&egui_id);
    shared
        .input
        .retain(|pending| pending.input.viewport != egui_id);
    shared
        .captures
        .retain(|capture| capture.viewport != egui_id);
    shared
        .viewport_commands
        .retain(|(viewport, _)| *viewport != egui_id);
    shared.viewport_close_requests.remove(&egui_id);
    shared.synthetic_input.remove(&egui_id);
    shared.synthetic_frame.remove(&egui_id);
    // A closed viewport never paints again, so its touchpoints can no longer
    // notice their own target leaving. End their lifetimes here instead.
    shared.coach.unmount_viewport(egui_id);
    shared.parent_wake_after_pass.remove(&egui_id);
    shared.viewport_semantic_revisions.remove(&egui_id);
    shared.viewport_pending_input_commit.remove(&egui_id);
    // Keep the committed-input generation as a bounded session tombstone.
    // A click can intentionally replace its deferred viewport (detach,
    // re-dock, pop-out). The runtime must still prove that the initiating
    // input pass committed after the old viewport leaves the live registry.
    shared.viewport_events.push_back(ViewportLifecycleEvent {
        kind: protocol::ViewportEventKind::Closed,
        viewport: live.descriptor,
        state: ViewportLifecycle::Closed,
    });
}

fn live_descriptors(shared: &SharedState) -> Vec<ViewportDescriptor> {
    let mut viewports: Vec<_> = shared
        .live_viewports
        .values()
        .map(|live| live.descriptor.clone())
        .collect();
    viewports.sort_by(|left, right| left.id.0.cmp(&right.id.0));
    viewports
}

fn sanitize_viewport_debug(viewport: egui::ViewportId) -> String {
    format!("{viewport:?}")
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn install(ctx: &egui::Context) -> PilotHandle {
    if let Some(handle) = ctx.with_plugin::<EguiPilotPlugin, _>(|plugin| plugin.handle()) {
        return handle;
    }
    let (plugin, handle) = EguiPilotPlugin::new();
    ctx.add_plugin(plugin);
    handle
}

pub fn handle(ctx: &egui::Context) -> Option<PilotHandle> {
    ctx.with_plugin::<EguiPilotPlugin, _>(|plugin| plugin.handle())
}

/// Whether the current viewport's pass carries automation-injected input.
///
/// Returns `false` when no plugin is installed — every production build —
/// because real input is the only input a production build can receive.
/// Privileged-mode handlers call this at activation time so an agent that
/// fills a composer and clicks Send gets a recorded refusal, not a mode
/// change (determinism-intent-gate plan §3.5).
pub fn input_is_synthetic(ctx: &egui::Context) -> bool {
    let Some(pilot) = handle(ctx) else {
        return false;
    };
    pilot.frame_input_synthetic(ctx.viewport_id())
}

pub trait ContextPilotExt {
    /// Consume a one-shot close request from the automation runtime.
    ///
    /// The code that owns the deferred viewport's declaration should consume
    /// this request and stop declaring the viewport. The viewport callback
    /// might not repaint while its native window is in the background.
    fn pilot_take_viewport_close_request(&self, viewport_id: egui::ViewportId) -> bool;

    /// Mark an application-owned viewport lifetime as intentionally retired.
    fn pilot_retire_viewport(&self, viewport_id: egui::ViewportId);

    fn pilot_show_viewport_deferred<F>(
        &self,
        key: protocol::PilotViewportKey,
        viewport_id: egui::ViewportId,
        builder: egui::ViewportBuilder,
        viewport_ui_cb: F,
    ) where
        F: Fn(&mut egui::Ui, egui::ViewportClass) + Send + Sync + 'static;

    fn pilot_show_viewport_immediate<T, F>(
        &self,
        key: protocol::PilotViewportKey,
        viewport_id: egui::ViewportId,
        builder: egui::ViewportBuilder,
        viewport_ui_cb: F,
    ) -> T
    where
        F: FnMut(&mut egui::Ui, egui::ViewportClass) -> T;
}

impl ContextPilotExt for egui::Context {
    fn pilot_take_viewport_close_request(&self, viewport_id: egui::ViewportId) -> bool {
        handle(self).is_some_and(|pilot| pilot.take_viewport_close_request(viewport_id))
    }

    fn pilot_retire_viewport(&self, viewport_id: egui::ViewportId) {
        if let Some(pilot) = handle(self) {
            pilot.retire_viewport(viewport_id);
        }
    }

    fn pilot_show_viewport_deferred<F>(
        &self,
        key: protocol::PilotViewportKey,
        viewport_id: egui::ViewportId,
        builder: egui::ViewportBuilder,
        viewport_ui_cb: F,
    ) where
        F: Fn(&mut egui::Ui, egui::ViewportClass) + Send + Sync + 'static,
    {
        let pilot = handle(self);
        if let Some(handle) = &pilot {
            handle.declare_viewport(
                viewport_id,
                key,
                WireViewportClass::Deferred,
                builder.title.clone().unwrap_or_default(),
            );
            // A minimized or background deferred viewport may not run its own
            // callback, so it cannot drain commands that would restore or
            // focus itself. Its declaration owner is already repaintable and
            // can target native commands at the child without transferring
            // rendering ownership.
            for command in handle.take_viewport_commands(viewport_id) {
                self.send_viewport_cmd_to(viewport_id, command);
            }
        }
        self.show_viewport_deferred(viewport_id, builder, viewport_ui_cb);
    }

    fn pilot_show_viewport_immediate<T, F>(
        &self,
        key: protocol::PilotViewportKey,
        viewport_id: egui::ViewportId,
        builder: egui::ViewportBuilder,
        mut viewport_ui_cb: F,
    ) -> T
    where
        F: FnMut(&mut egui::Ui, egui::ViewportClass) -> T,
    {
        let pilot = handle(self);
        if let Some(handle) = &pilot {
            handle.declare_viewport(
                viewport_id,
                key,
                WireViewportClass::Immediate,
                builder.title.clone().unwrap_or_default(),
            );
        }
        self.show_viewport_immediate(viewport_id, builder, move |ui, class| {
            if let Some(handle) = &pilot {
                for command in handle.take_viewport_commands(viewport_id) {
                    ui.ctx().send_viewport_cmd(command);
                }
            }
            viewport_ui_cb(ui, class)
        })
    }
}

pub trait UiPilotExt {
    fn pilot_button(&mut self, id: PilotId, text: &str) -> egui::Response;
    fn pilot_label(&mut self, id: PilotId, text: &str) -> egui::Response;
    fn pilot_track(&mut self, response: &egui::Response, metadata: NodeMetadata);
}

impl UiPilotExt for egui::Ui {
    #[track_caller]
    fn pilot_button(&mut self, id: PilotId, text: &str) -> egui::Response {
        let response = self.push_id(id.as_str(), |ui| ui.button(text)).inner;
        self.pilot_track(
            &response,
            NodeMetadata::new(id, Role::Button, text.to_string()),
        );
        response
    }

    #[track_caller]
    fn pilot_label(&mut self, id: PilotId, text: &str) -> egui::Response {
        let response = self.push_id(id.as_str(), |ui| ui.label(text)).inner;
        self.pilot_track(
            &response,
            NodeMetadata::new(id, Role::Label, text.to_string()),
        );
        response
    }

    #[track_caller]
    fn pilot_track(&mut self, response: &egui::Response, metadata: NodeMetadata) {
        let role = metadata.role;
        let name = metadata.name.clone();
        response.widget_info(|| {
            egui::WidgetInfo::labeled(widget_type(role), response.enabled(), name.clone())
        });
        self.ctx().with_plugin::<EguiPilotPlugin, _>(|plugin| {
            plugin.register(self, response, metadata);
        });
    }
}

fn widget_type(role: Role) -> egui::WidgetType {
    match role {
        Role::Button | Role::Tab | Role::MenuItem => egui::WidgetType::Button,
        Role::Link => egui::WidgetType::Link,
        Role::Checkbox => egui::WidgetType::Checkbox,
        Role::Radio => egui::WidgetType::RadioButton,
        Role::TextInput => egui::WidgetType::TextEdit,
        Role::Slider => egui::WidgetType::Slider,
        Role::ComboBox => egui::WidgetType::ComboBox,
        Role::Image => egui::WidgetType::Image,
        Role::Progress => egui::WidgetType::ProgressIndicator,
        Role::Window | Role::Dialog => egui::WidgetType::Window,
        Role::Label | Role::Heading | Role::Status | Role::Alert => egui::WidgetType::Label,
        _ => egui::WidgetType::Other,
    }
}

fn rect(value: egui::Rect) -> Rect {
    Rect {
        min_x: value.min.x,
        min_y: value.min.y,
        max_x: value.max.x,
        max_y: value.max.y,
    }
}

fn egui_rect(value: Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(value.min_x, value.min_y),
        egui::pos2(value.max_x, value.max_y),
    )
}

fn semantically_equal(left: &SemanticNode, right: &SemanticNode) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.changed_at = 0;
    right.changed_at = 0;
    left == right
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAVE: PilotId = PilotId::new("settings.save");

    #[test]
    fn plugin_captures_typed_widget_semantics() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        ctx.run_ui(Default::default(), |ui| {
            ui.pilot_button(SAVE, "Save");
        })
        .textures_delta
        .clear();
        let frame = pilot.latest_frame();
        assert_eq!(frame.nodes.len(), 1);
        assert_eq!(frame.nodes[0].id, NodeId::new("root/settings.save"));
        assert_eq!(frame.nodes[0].role, Role::Button);
        assert!(frame.nodes[0].actions.contains(&ActionKind::Click));
    }

    #[test]
    fn semantic_focus_is_logical_and_independent_from_native_window_focus() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let mut input = egui::RawInput {
            focused: false,
            ..Default::default()
        };
        input.viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                focused: Some(false),
                ..Default::default()
            },
        );
        ctx.run_ui(input, |ui| {
            let (_, rect) = ui.allocate_space(egui::vec2(80.0, 24.0));
            let response = ui.interact(rect, egui::Id::new("logical-focus"), egui::Sense::click());
            response.request_focus();
            assert!(!response.has_focus());
            ui.pilot_track(&response, NodeMetadata::new(SAVE, Role::Button, "Save"));
        })
        .textures_delta
        .clear();

        let frame = pilot.latest_frame();
        assert!(frame.nodes[0].state.focused);
    }

    #[test]
    fn requested_parent_wake_runs_after_the_child_pass() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let child = egui::ViewportId::from_hash_of("pilot-input-child");
        pilot.declare_viewport(
            child,
            protocol::PilotViewportKey::new("test.input-child"),
            WireViewportClass::Deferred,
            "Input child",
        );
        pilot.request_parent_wake_after_next_pass(child);

        let repaints = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&repaints);
        ctx.set_request_repaint_callback(move |request| {
            lock(&recorded).push(request.viewport_id);
        });

        let mut input = egui::RawInput {
            viewport_id: child,
            ..Default::default()
        };
        input
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        input.viewports.insert(
            child,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                ..Default::default()
            },
        );
        ctx.run_ui(input, |_ui| {}).textures_delta.clear();

        assert!(lock(&repaints).contains(&egui::ViewportId::ROOT));
    }

    #[test]
    fn semantic_revisions_are_scoped_to_the_viewport_that_committed() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let child = egui::ViewportId::from_hash_of("pilot-revision-child");

        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();
        let root_revision = pilot
            .viewport_semantic_revision(egui::ViewportId::ROOT)
            .expect("root semantic revision");
        assert_eq!(pilot.viewport_semantic_revision(child), None);

        let mut input = egui::RawInput {
            viewport_id: child,
            ..Default::default()
        };
        input
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        input.viewports.insert(
            child,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                ..Default::default()
            },
        );
        ctx.run_ui(input, |_ui| {}).textures_delta.clear();

        assert_eq!(
            pilot.viewport_semantic_revision(egui::ViewportId::ROOT),
            Some(root_revision)
        );
        assert!(pilot
            .viewport_semantic_revision(child)
            .is_some_and(|revision| revision > root_revision));
    }

    fn save_button_frame(ctx: &egui::Context, input: egui::RawInput) {
        ctx.run_ui(input, |ui| {
            ui.pilot_button(SAVE, "Save");
        })
        .textures_delta
        .clear();
    }

    fn mount_save_touchpoint(pilot: &PilotHandle, once: bool) -> Rect {
        mount_on_save(pilot, "coach-save", protocol::TriggerKind::Press, once)
            .expect("the save node is on screen")
    }

    /// Mount a touchpoint on the save button, arming it when the trigger can
    /// wait and the node is not rendered yet.
    fn mount_on_save(
        pilot: &PilotHandle,
        coach_id: &str,
        on: protocol::TriggerKind,
        once: bool,
    ) -> Option<Rect> {
        let frame = pilot.latest_frame();
        let node = frame
            .nodes
            .iter()
            .find(|node| node.id == NodeId::new("root/settings.save"));
        // An appearance trigger always arms, even over a node that happens to
        // be on screen already — the runtime applies the same rule.
        let arm = node.is_none() || on == protocol::TriggerKind::NodeAppeared;
        let rect = node.map(|node| node.rect);
        pilot
            .mount_coach(CoachMount {
                coach_id: CoachId::new(coach_id),
                node: NodeId::new("root/settings.save"),
                viewport: (!arm).then_some(egui::ViewportId::ROOT),
                rect: (!arm).then_some(()).and(rect),
                on,
                until: None,
                message: Some("Press Save".into()),
                ttl_ms: None,
                once,
            })
            .expect("mount");
        rect
    }

    /// The failure this exists to prevent: a lesson step whose goal is already
    /// met must complete. An edge trigger cannot answer that, because the
    /// transition it waits for has already happened and will not repeat.
    #[test]
    fn a_condition_that_already_holds_satisfies_without_an_input_edge() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, egui::RawInput::default());
        let rect = pilot.latest_frame().nodes[0].rect;

        pilot
            .mount_coach(CoachMount {
                coach_id: CoachId::new("coach-already"),
                node: NodeId::new("root/settings.save"),
                viewport: Some(egui::ViewportId::ROOT),
                rect: Some(rect),
                on: protocol::TriggerKind::Press,
                until: Some(protocol::CoachCondition::NodeVisible {
                    node: NodeId::new("root/settings.save"),
                }),
                message: None,
                ttl_ms: None,
                once: true,
            })
            .expect("mount");
        let _ = pilot.take_coach_events();
        save_button_frame(&ctx, egui::RawInput::default());

        let events = pilot.take_coach_events();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, protocol::CoachEvent::Satisfied { .. })),
            "nobody pressed anything, but the state the step asks for holds: {events:?}"
        );
    }

    fn press_at(center: egui::Pos2) -> egui::RawInput {
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
        }
    }

    /// The shape of nearly every lesson step: the click that opens a menu is
    /// not the click that finishes the step. A press must report itself and
    /// leave a touchpoint with a postcondition standing.
    #[test]
    fn a_press_reports_but_does_not_end_a_step_that_has_a_postcondition() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, egui::RawInput::default());
        let rect = pilot.latest_frame().nodes[0].rect;
        pilot
            .mount_coach(CoachMount {
                coach_id: CoachId::new("coach-hint"),
                node: NodeId::new("root/settings.save"),
                viewport: Some(egui::ViewportId::ROOT),
                rect: Some(rect),
                on: protocol::TriggerKind::Press,
                until: Some(protocol::CoachCondition::NodeValue {
                    node: NodeId::new("root/settings.save"),
                    value: "never".into(),
                }),
                message: None,
                ttl_ms: None,
                once: true,
            })
            .expect("mount");
        let _ = pilot.take_coach_events();

        save_button_frame(&ctx, press_at(egui_rect(rect).center()));

        let events = pilot.take_coach_events();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, CoachEvent::Triggered { .. })),
            "the press is still worth reporting: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, CoachEvent::Unmounted { .. })),
            "the state never became true, so the step is not over: {events:?}"
        );
    }

    #[test]
    fn an_operator_press_inside_a_once_touchpoint_triggers_then_unmounts() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, Default::default());
        let rect = mount_save_touchpoint(&pilot, true);
        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Mounted {
                coach_id: CoachId::new("coach-save"),
                rect,
            }]
        );

        save_button_frame(&ctx, press_at(egui_rect(rect).center()));

        assert_eq!(
            pilot.take_coach_events(),
            vec![
                CoachEvent::Triggered {
                    coach_id: CoachId::new("coach-save"),
                    kind: protocol::TriggerKind::Press,
                    actor: CoachActor::Operator,
                },
                CoachEvent::Unmounted {
                    coach_id: CoachId::new("coach-save"),
                    reason: CoachUnmountReason::Once,
                },
            ]
        );
        assert_eq!(pilot.mounted_coach_count(), 0);
    }

    #[test]
    fn a_press_outside_the_target_is_not_a_trigger() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, Default::default());
        let rect = mount_save_touchpoint(&pilot, true);
        let _ = pilot.take_coach_events();

        save_button_frame(
            &ctx,
            press_at(egui::pos2(rect.max_x + 40.0, rect.max_y + 40.0)),
        );

        assert!(pilot.take_coach_events().is_empty());
        assert_eq!(pilot.mounted_coach_count(), 1);
    }

    #[test]
    fn the_event_tap_witnesses_without_consuming() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let witness = std::sync::Arc::clone(&seen);
        pilot.set_coach_event_tap(move |event| witness.lock().unwrap().push(event.clone()));

        save_button_frame(&ctx, Default::default());
        let rect = mount_save_touchpoint(&pilot, true);
        save_button_frame(&ctx, press_at(egui_rect(rect).center()));

        let drained = pilot.take_coach_events();
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen, drained,
            "the tap is a witness: it sees exactly what the drain returns, and steals nothing"
        );
        assert!(
            seen.iter()
                .any(|event| matches!(event, CoachEvent::Triggered { .. })),
            "{seen:?}"
        );
    }

    #[test]
    fn injected_input_attributes_the_trigger_to_the_agent() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, Default::default());
        let rect = mount_save_touchpoint(&pilot, true);
        let _ = pilot.take_coach_events();

        pilot.queue_input(QueuedInput {
            viewport: egui::ViewportId::ROOT,
            events: press_at(egui_rect(rect).center()).events,
            time: None,
            modifiers: None,
        });
        save_button_frame(&ctx, Default::default());

        assert!(pilot.take_coach_events().contains(&CoachEvent::Triggered {
            coach_id: CoachId::new("coach-save"),
            kind: protocol::TriggerKind::Press,
            actor: CoachActor::Agent,
        }));
    }

    #[test]
    fn the_synthetic_frame_query_survives_the_coach_pass() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        assert!(
            !input_is_synthetic(&ctx),
            "no injection yet — real (or absent) input"
        );

        save_button_frame(&ctx, Default::default());
        let rect = mount_save_touchpoint(&pilot, true);
        let _ = pilot.take_coach_events();

        pilot.queue_input(QueuedInput {
            viewport: egui::ViewportId::ROOT,
            events: press_at(egui_rect(rect).center()).events,
            time: None,
            modifiers: None,
        });
        // The input hook marks the frame synthetic; the coach pass must
        // not clear it — any number of app handlers can still ask.
        save_button_frame(&ctx, Default::default());
        assert!(
            pilot.frame_input_synthetic(egui::ViewportId::ROOT),
            "injected input is still queryable after the coach pass"
        );
        assert!(input_is_synthetic(&ctx));

        // A subsequent pass with no queued injection clears the flag.
        save_button_frame(&ctx, Default::default());
        assert!(!pilot.frame_input_synthetic(egui::ViewportId::ROOT));
        assert!(!input_is_synthetic(&ctx));
    }

    #[test]
    fn input_is_synthetic_is_false_without_a_plugin() {
        let ctx = egui::Context::default();
        assert!(
            !input_is_synthetic(&ctx),
            "production builds have no plugin — real input is the only input"
        );
    }

    #[test]
    fn a_touchpoint_whose_target_stops_rendering_ends_its_own_lifetime() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, Default::default());
        mount_save_touchpoint(&pilot, false);
        let _ = pilot.take_coach_events();

        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();

        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Unmounted {
                coach_id: CoachId::new("coach-save"),
                reason: CoachUnmountReason::TargetLost,
            }]
        );
        assert_eq!(pilot.mounted_coach_count(), 0);
    }

    #[test]
    fn a_touchpoint_expires_on_its_own_ttl() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        pilot.set_clock_mode(ClockMode::Fixed(100.0));
        save_button_frame(&ctx, Default::default());
        let frame = pilot.latest_frame();
        let node = frame
            .nodes
            .iter()
            .find(|node| node.id == NodeId::new("root/settings.save"))
            .expect("save node");
        pilot
            .mount_coach(CoachMount {
                coach_id: CoachId::new("coach-ttl"),
                node: node.id.clone(),
                viewport: Some(egui::ViewportId::ROOT),
                rect: Some(node.rect),
                on: protocol::TriggerKind::Press,
                until: None,
                message: None,
                ttl_ms: Some(500),
                once: false,
            })
            .expect("mount");
        let _ = pilot.take_coach_events();

        save_button_frame(&ctx, Default::default());
        assert!(pilot.take_coach_events().is_empty());

        pilot.set_clock_mode(ClockMode::Fixed(101.0));
        save_button_frame(&ctx, Default::default());

        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Unmounted {
                coach_id: CoachId::new("coach-ttl"),
                reason: CoachUnmountReason::Ttl,
            }]
        );
    }

    #[test]
    fn an_armed_touchpoint_mounts_and_fires_when_its_target_appears() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();
        assert_eq!(
            mount_on_save(
                &pilot,
                "coach-appear",
                protocol::TriggerKind::NodeAppeared,
                true
            ),
            None,
            "the save button is not rendered yet"
        );
        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Armed {
                coach_id: CoachId::new("coach-appear"),
            }]
        );

        // Nothing to see: the target is still absent, and an armed touchpoint
        // must not mistake that for a lost target.
        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();
        assert!(pilot.take_coach_events().is_empty());
        assert_eq!(pilot.mounted_coach_count(), 1);

        save_button_frame(&ctx, Default::default());

        let events = pilot.take_coach_events();
        assert!(
            matches!(events.first(), Some(CoachEvent::Mounted { coach_id, .. })
                if coach_id == &CoachId::new("coach-appear")),
            "{events:?}"
        );
        assert_eq!(
            &events[1..],
            [
                CoachEvent::Triggered {
                    coach_id: CoachId::new("coach-appear"),
                    kind: protocol::TriggerKind::NodeAppeared,
                    // Nobody pressed anything; the app rendered it.
                    actor: CoachActor::App,
                },
                CoachEvent::Unmounted {
                    coach_id: CoachId::new("coach-appear"),
                    reason: CoachUnmountReason::Once,
                },
            ]
        );
    }

    #[test]
    fn hover_fires_on_entering_the_target_and_not_again_while_the_pointer_rests() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, Default::default());
        let rect = mount_on_save(&pilot, "coach-hover", protocol::TriggerKind::Hover, false)
            .expect("save node");
        let _ = pilot.take_coach_events();
        let center = egui_rect(rect).center();

        let moved = |pos| egui::RawInput {
            events: vec![egui::Event::PointerMoved(pos)],
            ..Default::default()
        };
        save_button_frame(&ctx, moved(center));
        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Triggered {
                coach_id: CoachId::new("coach-hover"),
                kind: protocol::TriggerKind::Hover,
                actor: CoachActor::Operator,
            }]
        );

        // Still inside: a level, not an edge. Silence.
        save_button_frame(&ctx, moved(center + egui::vec2(1.0, 0.0)));
        assert!(pilot.take_coach_events().is_empty());

        save_button_frame(
            &ctx,
            moved(egui::pos2(rect.max_x + 80.0, rect.max_y + 80.0)),
        );
        save_button_frame(&ctx, moved(center));
        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Triggered {
                coach_id: CoachId::new("coach-hover"),
                kind: protocol::TriggerKind::Hover,
                actor: CoachActor::Operator,
            }]
        );
    }

    #[test]
    fn a_published_app_signal_fires_a_waiting_touchpoint_exactly_once() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        save_button_frame(&ctx, Default::default());
        let signal = SignalId::new("build.slide-placed");
        mount_on_save(
            &pilot,
            "coach-build",
            protocol::TriggerKind::FabricEvent {
                signal: signal.clone(),
            },
            false,
        );
        let _ = pilot.take_coach_events();

        // An unrelated signal is not this touchpoint's signal.
        pilot.note_coach_signal(SignalId::new("build.episode-changed"));
        save_button_frame(&ctx, Default::default());
        assert!(pilot.take_coach_events().is_empty());

        pilot.note_coach_signal(signal.clone());
        save_button_frame(&ctx, Default::default());
        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Triggered {
                coach_id: CoachId::new("coach-build"),
                kind: protocol::TriggerKind::FabricEvent { signal },
                actor: CoachActor::App,
            }]
        );

        // The signal is history now; it must not re-fire every frame.
        save_button_frame(&ctx, Default::default());
        assert!(pilot.take_coach_events().is_empty());
    }

    #[test]
    fn a_value_change_fires_once_per_change_not_once_per_frame() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let frame = |value: &str| {
            let value = value.to_string();
            ctx.run_ui(Default::default(), |ui| {
                let (_, rect) = ui.allocate_space(egui::vec2(80.0, 24.0));
                let response = ui.interact(rect, egui::Id::new("status"), egui::Sense::click());
                ui.pilot_track(
                    &response,
                    NodeMetadata::new(SAVE, Role::Status, "Status").value(value.clone()),
                );
            })
            .textures_delta
            .clear();
        };
        let mut value = String::from("draft");
        frame(&value);
        mount_on_save(
            &pilot,
            "coach-value",
            protocol::TriggerKind::ValueChanged,
            false,
        );
        let _ = pilot.take_coach_events();

        frame(&value);
        assert!(pilot.take_coach_events().is_empty());

        value = String::from("published");
        frame(&value);
        assert_eq!(
            pilot.take_coach_events(),
            vec![CoachEvent::Triggered {
                coach_id: CoachId::new("coach-value"),
                kind: protocol::TriggerKind::ValueChanged,
                actor: CoachActor::App,
            }]
        );

        frame(&value);
        assert!(pilot.take_coach_events().is_empty());
    }

    #[test]
    #[should_panic(expected = "duplicate EguiPilotAI id")]
    fn duplicate_ids_are_hard_failures() {
        let ctx = egui::Context::default();
        install(&ctx);
        ctx.run_ui(Default::default(), |ui| {
            ui.pilot_button(SAVE, "Save");
            ui.pilot_button(SAVE, "Save again");
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn busy_token_is_removed_on_drop() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let token = pilot.begin_busy("loading");
        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();
        assert_eq!(pilot.latest_frame().diagnostics.busy_tokens, ["loading"]);
        drop(token);
        ctx.run_ui(Default::default(), |_ui| {})
            .textures_delta
            .clear();
        assert!(pilot.latest_frame().diagnostics.busy_tokens.is_empty());
    }

    #[test]
    fn suppression_rejects_uncontrolled_ambient_motion() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        const LOOP: MotionId = MotionId::new("ambient.loop");
        pilot.mark_uncontrolled_motion(
            LOOP,
            MotionKind::Ambient {
                canonical_phase_seconds: None,
            },
        );
        assert_eq!(
            pilot.prepare_motion(&ctx, MotionPolicy::Suppressed),
            Err(vec![LOOP])
        );
    }

    #[test]
    fn input_generation_commits_only_after_its_target_viewport_pass() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let child = egui::ViewportId::from_hash_of("pilot-input-generation-child");
        let generation = pilot.queue_input(QueuedInput {
            viewport: child,
            events: vec![egui::Event::PointerMoved(egui::pos2(8.0, 9.0))],
            time: None,
            modifiers: None,
        });
        assert_eq!(pilot.viewport_committed_input_generation(child), 0);

        let mut input = egui::RawInput {
            viewport_id: child,
            ..Default::default()
        };
        input
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        input.viewports.insert(
            child,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                ..Default::default()
            },
        );
        let mut output = ctx.run_ui(input, |_ui| {});
        // epaint 0.36 panics if this drops with unapplied deltas.
        output.textures_delta.clear();

        assert!(output.platform_output.events.is_empty());
        assert_eq!(pilot.viewport_committed_input_generation(child), generation);
    }

    #[test]
    fn retiring_a_viewport_preserves_its_final_input_commit() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let child = egui::ViewportId::from_hash_of("pilot-retired-input-child");
        pilot.declare_viewport(
            child,
            protocol::PilotViewportKey::new("test.retired-input-child"),
            WireViewportClass::Immediate,
            "Retired input child",
        );
        let generation = pilot.queue_input(QueuedInput {
            viewport: child,
            events: vec![egui::Event::PointerMoved(egui::pos2(8.0, 9.0))],
            time: None,
            modifiers: None,
        });
        let mut input = egui::RawInput {
            viewport_id: child,
            ..Default::default()
        };
        input
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        input.viewports.insert(
            child,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                ..Default::default()
            },
        );

        ctx.run_ui(input, |_ui| pilot.retire_viewport(child))
            .textures_delta
            .clear();

        assert_eq!(pilot.viewport_committed_input_generation(child), generation);
        assert!(pilot
            .viewports()
            .iter()
            .all(|viewport| viewport.key.0 != "test.retired-input-child"));
    }

    #[test]
    fn fixed_clock_overrides_raw_input_time() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        pilot.set_clock_mode(ClockMode::Fixed(42.0));
        let observed = std::cell::Cell::new(0.0);
        ctx.run_ui(Default::default(), |ui| {
            observed.set(ui.input(|input| input.time));
        })
        .textures_delta
        .clear();
        assert_eq!(observed.get(), 42.0);
    }

    #[test]
    fn viewport_registry_removes_closed_state_and_advances_generation() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let child = egui::ViewportId::from_hash_of("pilot-child");
        let child_key = protocol::PilotViewportKey::new("test.child");
        pilot.declare_viewport(
            child,
            child_key.clone(),
            WireViewportClass::Immediate,
            "Child",
        );

        let mut input = egui::RawInput::default();
        input
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        input.viewports.insert(
            child,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                title: Some("Child".into()),
                focused: Some(true),
                ..Default::default()
            },
        );
        sync_viewports(&mut lock(&pilot.shared), &input);
        let first = pilot
            .viewports()
            .into_iter()
            .find(|viewport| viewport.key == child_key)
            .expect("child viewport");
        assert_eq!(first.id, ViewportId::new("test.child#1"));
        pilot.request_viewport_close(child);

        let mut root_only = egui::RawInput::default();
        root_only
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        sync_viewports(&mut lock(&pilot.shared), &root_only);
        assert!(pilot
            .viewports()
            .iter()
            .all(|viewport| viewport.key != child_key));
        assert!(pilot.take_viewport_events().iter().any(|event| {
            event.viewport.id == first.id && event.state == ViewportLifecycle::Closed
        }));
        assert!(!ctx.pilot_take_viewport_close_request(child));

        let mut reopened = egui::RawInput::default();
        reopened
            .viewports
            .insert(egui::ViewportId::ROOT, egui::ViewportInfo::default());
        reopened
            .viewports
            .insert(child, egui::ViewportInfo::default());
        sync_viewports(&mut lock(&pilot.shared), &reopened);
        let second = pilot
            .viewports()
            .into_iter()
            .find(|viewport| viewport.key == child_key)
            .expect("reopened child viewport");
        assert_eq!(second.id, ViewportId::new("test.child#2"));
    }

    #[test]
    fn deferred_close_request_is_consumed_once_by_its_owner() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let child = egui::ViewportId::from_hash_of("deferred-child");

        pilot.request_viewport_close(child);

        assert!(ctx.pilot_take_viewport_close_request(child));
        assert!(!ctx.pilot_take_viewport_close_request(child));
    }
}
