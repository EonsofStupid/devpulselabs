//! Typed, agent-facing semantic instrumentation for egui.
//!
//! This crate contains no process or network transport. Installing the plugin
//! only makes an egui context observable and able to accept queued egui input.

pub use egui_pilot_ai_protocol as protocol;
use protocol::{
    ActionKind, FrameDiagnostics, MotionId as WireMotionId, NodeId, NodeState, Rect, Role,
    SemanticFrame, SemanticNode, SnapshotError, SnapshotRequest, SourceLocation,
    ViewportClass as WireViewportClass, ViewportDescriptor, ViewportId, ViewportLifecycle,
    ViewportLifecycleEvent, ViewportSelector,
};
use std::collections::{HashMap, VecDeque};
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
    input: VecDeque<QueuedInput>,
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
    motion_policy: MotionPolicy,
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

    pub fn queue_input(&self, input: QueuedInput) {
        lock(&self.shared).input.push_back(input);
    }

    pub fn pending_input(&self) -> usize {
        lock(&self.shared).input.len()
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
                focused: response.has_focus(),
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
    }

    fn input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
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
            .position(|queued| queued.viewport == input.viewport_id)
        {
            let queued = shared.input.remove(index).expect("queued input index");
            input.events.extend(queued.events);
            if let Some(time) = queued.time {
                input.time = Some(time);
            }
            if let Some(modifiers) = queued.modifiers {
                input.modifiers = modifiers;
            }
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

fn sync_viewports(shared: &mut SharedState, input: &egui::RawInput) {
    if input.viewport_id == egui::ViewportId::ROOT {
        let observed: Vec<_> = input.viewports.keys().copied().collect();
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
    shared.input.retain(|input| input.viewport != egui_id);
    shared
        .captures
        .retain(|capture| capture.viewport != egui_id);
    shared
        .viewport_commands
        .retain(|(viewport, _)| *viewport != egui_id);
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

pub trait ContextPilotExt {
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
        let _ = ctx.run_ui(Default::default(), |ui| {
            ui.pilot_button(SAVE, "Save");
        });
        let frame = pilot.latest_frame();
        assert_eq!(frame.nodes.len(), 1);
        assert_eq!(frame.nodes[0].id, NodeId::new("root/settings.save"));
        assert_eq!(frame.nodes[0].role, Role::Button);
        assert!(frame.nodes[0].actions.contains(&ActionKind::Click));
    }

    #[test]
    #[should_panic(expected = "duplicate EguiPilotAI id")]
    fn duplicate_ids_are_hard_failures() {
        let ctx = egui::Context::default();
        install(&ctx);
        let _ = ctx.run_ui(Default::default(), |ui| {
            ui.pilot_button(SAVE, "Save");
            ui.pilot_button(SAVE, "Save again");
        });
    }

    #[test]
    fn busy_token_is_removed_on_drop() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        let token = pilot.begin_busy("loading");
        let _ = ctx.run_ui(Default::default(), |_ui| {});
        assert_eq!(pilot.latest_frame().diagnostics.busy_tokens, ["loading"]);
        drop(token);
        let _ = ctx.run_ui(Default::default(), |_ui| {});
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
    fn fixed_clock_overrides_raw_input_time() {
        let ctx = egui::Context::default();
        let pilot = install(&ctx);
        pilot.set_clock_mode(ClockMode::Fixed(42.0));
        let observed = std::cell::Cell::new(0.0);
        let _ = ctx.run_ui(Default::default(), |ui| {
            observed.set(ui.input(|input| input.time));
        });
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
}
