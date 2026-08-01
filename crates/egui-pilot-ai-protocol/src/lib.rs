//! Versioned wire model shared by EguiPilotAI runtimes, runners, durable
//! scenarios, and MCP. This crate deliberately has no egui or OS dependency.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion { major: 2, minor: 6 };
pub const DEFAULT_AGENT_NODES: usize = 200;
pub const DEFAULT_AGENT_BYTES: usize = 12 * 1024;
pub const MAX_AGENT_NODES: usize = 1_000;
pub const MAX_AGENT_BYTES: usize = 64 * 1024;
pub const MAX_ENVELOPE_BYTES: usize = 4 * 1024 * 1024;

fn is_false(value: &bool) -> bool {
    !*value
}

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(NodeId);
string_id!(ViewportId);
string_id!(PilotViewportKey);
string_id!(FixtureId);
string_id!(ScenarioId);
string_id!(ProjectId);
string_id!(SessionId);
string_id!(MotionId);
string_id!(RequestId);
string_id!(RunId);
string_id!(StepId);
string_id!(AttachmentId);
string_id!(CapabilityId);
string_id!(EventId);
string_id!(CoachId);
string_id!(SignalId);

impl EventId {
    pub fn from_sequence(sequence: u64) -> Self {
        Self(format!("event-{sequence:020}"))
    }

    pub fn sequence(&self) -> Option<u64> {
        self.0.strip_prefix("event-")?.parse().ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Rect {
    pub fn width(self) -> f32 {
        (self.max_x - self.min_x).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.max_y - self.min_y).max(0.0)
    }

    pub fn intersects(self, other: Self) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewportClass {
    #[default]
    Root,
    Immediate,
    Deferred,
    Embedded,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewportLifecycle {
    #[default]
    Open,
    CloseRequested,
    Closed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewportSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ViewportId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<PilotViewportKey>,
}

impl ViewportSelector {
    pub fn by_id(id: impl Into<String>) -> Self {
        Self {
            id: Some(ViewportId::new(id)),
            key: None,
        }
    }

    pub fn by_key(key: impl Into<String>) -> Self {
        Self {
            id: None,
            key: Some(PilotViewportKey::new(key)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewportDescriptor {
    pub id: ViewportId,
    pub key: PilotViewportKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ViewportId>,
    #[serde(default)]
    pub class: ViewportClass,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default)]
    pub lifecycle: ViewportLifecycle,
    #[serde(default)]
    pub lifecycle_revision: u64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub close_requested: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub focused: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub minimized: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub maximized: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub occluded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_rect: Option<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outer_rect: Option<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_pixels_per_point: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewportExpectationState {
    Open,
    Focused,
    Minimized,
    Restored,
    CloseRequested,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewportLifecycleEvent {
    pub kind: ViewportEventKind,
    pub viewport: ViewportDescriptor,
    pub state: ViewportLifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewportEventKind {
    Created,
    Changed,
    CloseRequested,
    Focused,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Application,
    Window,
    Dialog,
    Region,
    Group,
    Heading,
    Label,
    Button,
    Link,
    Checkbox,
    Radio,
    TextInput,
    Slider,
    ComboBox,
    Tab,
    MenuItem,
    List,
    ListItem,
    Image,
    Progress,
    Status,
    Alert,
    Separator,
    Generic,
}

impl Role {
    pub fn is_vital_text(self) -> bool {
        matches!(
            self,
            Self::Heading | Self::Label | Self::Status | Self::Alert
        )
    }

    pub fn is_region(self) -> bool {
        matches!(
            self,
            Self::Application | Self::Window | Self::Dialog | Self::Region
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    Click,
    DoubleClick,
    RightClick,
    Hover,
    Focus,
    Fill,
    Clear,
    Type,
    Press,
    Check,
    Uncheck,
    Select,
    SetValue,
    Drag,
    Scroll,
    ScrollIntoView,
    Copy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeState {
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub clipped: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub decorative: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub editable: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: NodeId,
    pub viewport: ViewportId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<NodeId>,
    pub role: Role,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub rect: Rect,
    pub clip_rect: Rect,
    #[serde(default)]
    pub state: NodeState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionKind>,
    #[serde(default)]
    pub layer: i32,
    #[serde(default)]
    pub changed_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceLocation>,
}

impl SemanticNode {
    pub fn actionable(&self) -> bool {
        !self.actions.is_empty()
    }

    pub fn visible(&self) -> bool {
        !self.state.hidden
            && !self.state.clipped
            && self.rect.width() > 0.0
            && self.rect.height() > 0.0
            && self.rect.intersects(self.clip_rect)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameDiagnostics {
    #[serde(default)]
    pub duplicate_ids: Vec<NodeId>,
    #[serde(default)]
    pub busy_tokens: Vec<String>,
    #[serde(default)]
    pub uncontrolled_motion: Vec<MotionId>,
    #[serde(default)]
    pub output_events: usize,
    #[serde(default)]
    pub accesskit_updated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticFrame {
    pub revision: u64,
    pub timestamp_seconds: f64,
    pub nodes: Vec<SemanticNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewports: Vec<ViewportDescriptor>,
    #[serde(default)]
    pub diagnostics: FrameDiagnostics,
}

impl Default for SemanticFrame {
    fn default() -> Self {
        Self {
            revision: 0,
            timestamp_seconds: 0.0,
            nodes: Vec::new(),
            viewports: Vec::new(),
            diagnostics: FrameDiagnostics::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotMode {
    #[default]
    Agent,
    Changes,
    Full,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<ViewportSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtree: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub include_decorative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRequest {
    #[serde(default)]
    pub mode: SnapshotMode,
    #[serde(default)]
    pub filter: SnapshotFilter,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_nodes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<usize>,
}

impl Default for SnapshotRequest {
    fn default() -> Self {
        Self {
            mode: SnapshotMode::Agent,
            filter: SnapshotFilter::default(),
            since_revision: None,
            continuation: None,
            max_nodes: None,
            max_bytes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNode {
    pub id: NodeId,
    pub role: Role,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionKind>,
    #[serde(default)]
    pub state: NodeState,
    pub rect: Rect,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub revision: u64,
    pub content_hash: String,
    pub nodes: Vec<AgentNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewports: Vec<ViewportDescriptor>,
    pub diagnostics: FrameDiagnostics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    InvalidContinuation,
    StaleContinuation { expected: u64, actual: u64 },
    ViewportBudgetExceeded { bytes: usize, maximum: usize },
    ViewportNotFound,
    ViewportAmbiguous { matches: usize },
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContinuation => f.write_str("invalid snapshot continuation"),
            Self::StaleContinuation { expected, actual } => write!(
                f,
                "snapshot continuation is for revision {expected}, current revision is {actual}"
            ),
            Self::ViewportBudgetExceeded { bytes, maximum } => write!(
                f,
                "viewport inventory and diagnostics require {bytes} bytes, exceeding snapshot budget {maximum}"
            ),
            Self::ViewportNotFound => {
                f.write_str("snapshot viewport selector matched no live viewport")
            }
            Self::ViewportAmbiguous { matches } => write!(
                f,
                "snapshot viewport selector matched {matches} live viewports"
            ),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Deterministically projects the complete diagnostic tree into the bounded
/// view sent to an agent.
pub fn project_snapshot(
    frame: &SemanticFrame,
    request: &SnapshotRequest,
) -> Result<AgentSnapshot, SnapshotError> {
    let (continuation_revision, offset) = match &request.continuation {
        Some(token) => parse_continuation(token)?,
        None => (frame.revision, 0),
    };
    if continuation_revision != frame.revision {
        return Err(SnapshotError::StaleContinuation {
            expected: continuation_revision,
            actual: frame.revision,
        });
    }

    let node_limit = request
        .max_nodes
        .unwrap_or(if request.mode == SnapshotMode::Full {
            100
        } else {
            DEFAULT_AGENT_NODES
        })
        .clamp(1, MAX_AGENT_NODES);
    let byte_limit = request
        .max_bytes
        .unwrap_or(DEFAULT_AGENT_BYTES)
        .clamp(512, MAX_AGENT_BYTES);

    let by_id: HashMap<&NodeId, &SemanticNode> =
        frame.nodes.iter().map(|node| (&node.id, node)).collect();
    let descendants = request
        .filter
        .subtree
        .as_ref()
        .map(|root| descendant_ids(root, &frame.nodes));
    let selected_viewport = request
        .filter
        .viewport
        .as_ref()
        .map(|selector| {
            let matches: Vec<_> = frame
                .viewports
                .iter()
                .filter(|viewport| {
                    selector.id.as_ref().is_none_or(|id| id == &viewport.id)
                        && selector.key.as_ref().is_none_or(|key| key == &viewport.key)
                })
                .collect();
            match matches.as_slice() {
                [] => Err(SnapshotError::ViewportNotFound),
                [viewport] => Ok(viewport.id.clone()),
                _ => Err(SnapshotError::ViewportAmbiguous {
                    matches: matches.len(),
                }),
            }
        })
        .transpose()?;
    let projected_viewports: Vec<_> = frame
        .viewports
        .iter()
        .filter(|viewport| {
            selected_viewport
                .as_ref()
                .is_none_or(|selected| selected == &viewport.id)
        })
        .cloned()
        .collect();

    let mut candidates: Vec<(u8, usize, AgentNode)> = frame
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            matches_filter(
                node,
                request,
                descendants.as_ref(),
                selected_viewport.as_ref(),
            )
        })
        .filter(|(_, node)| include_node(node, request))
        .map(|(index, node)| {
            (
                priority(node),
                index,
                AgentNode {
                    id: node.id.clone(),
                    role: node.role,
                    name: node.name.clone(),
                    value: node.value.clone(),
                    actions: node.actions.clone(),
                    state: node.state.clone(),
                    rect: node.rect,
                    regions: region_path(node, &by_id),
                },
            )
        })
        .collect();
    candidates.sort_by_key(|(priority, index, _)| (*priority, *index));

    let fixed_payload = serde_json::to_vec(&(&projected_viewports, &frame.diagnostics))
        .unwrap_or_default()
        .len()
        .saturating_add(512);
    if fixed_payload >= byte_limit {
        return Err(SnapshotError::ViewportBudgetExceeded {
            bytes: fixed_payload,
            maximum: byte_limit,
        });
    }

    let mut nodes = Vec::new();
    let mut next = offset;
    let payload_budget = byte_limit.saturating_sub(fixed_payload);
    let mut used = 2usize;
    for (_, _, node) in candidates.iter().skip(offset) {
        let encoded = serde_json::to_vec(node).unwrap_or_default();
        if nodes.len() >= node_limit || used + encoded.len() + 1 > payload_budget {
            break;
        }
        used += encoded.len() + 1;
        nodes.push(node.clone());
        next += 1;
    }

    let has_more = next < candidates.len();
    let serialized =
        serde_json::to_vec(&(&nodes, &projected_viewports, &frame.diagnostics)).unwrap_or_default();
    Ok(AgentSnapshot {
        revision: frame.revision,
        content_hash: stable_hash(&serialized),
        nodes,
        viewports: projected_viewports,
        diagnostics: frame.diagnostics.clone(),
        continuation: has_more.then(|| format!("{}:{next}", frame.revision)),
        truncated: has_more.then(|| {
            format!(
                "agent snapshot budget reached; {} nodes remain",
                candidates.len() - next
            )
        }),
    })
}

fn parse_continuation(token: &str) -> Result<(u64, usize), SnapshotError> {
    let (revision, offset) = token
        .split_once(':')
        .ok_or(SnapshotError::InvalidContinuation)?;
    Ok((
        revision
            .parse()
            .map_err(|_| SnapshotError::InvalidContinuation)?,
        offset
            .parse()
            .map_err(|_| SnapshotError::InvalidContinuation)?,
    ))
}

fn descendant_ids(root: &NodeId, nodes: &[SemanticNode]) -> Vec<NodeId> {
    let mut result = vec![root.clone()];
    let mut cursor = 0;
    while cursor < result.len() {
        let parent = result[cursor].clone();
        result.extend(
            nodes
                .iter()
                .filter(|node| node.parent.as_ref() == Some(&parent))
                .map(|node| node.id.clone()),
        );
        cursor += 1;
    }
    result
}

fn matches_filter(
    node: &SemanticNode,
    request: &SnapshotRequest,
    descendants: Option<&Vec<NodeId>>,
    selected_viewport: Option<&ViewportId>,
) -> bool {
    if selected_viewport.is_some_and(|viewport| viewport != &node.viewport) {
        return false;
    }
    if request.filter.role.is_some_and(|role| role != node.role) {
        return false;
    }
    if descendants.is_some_and(|ids| !ids.contains(&node.id)) {
        return false;
    }
    if request.mode == SnapshotMode::Changes
        && node.changed_at <= request.since_revision.unwrap_or(0)
    {
        return false;
    }
    true
}

fn include_node(node: &SemanticNode, request: &SnapshotRequest) -> bool {
    if node.state.hidden && !request.filter.include_hidden {
        return false;
    }
    if node.state.decorative && !request.filter.include_decorative {
        return false;
    }
    if request.mode == SnapshotMode::Full {
        return true;
    }
    node.visible()
        && (node.actionable()
            || node.role.is_vital_text()
            || node.state.focused
            || node.state.error.is_some())
}

fn priority(node: &SemanticNode) -> u8 {
    if node.state.focused || node.state.error.is_some() {
        0
    } else if node.actionable() {
        1
    } else if matches!(node.role, Role::Alert | Role::Status | Role::Heading) {
        2
    } else {
        3
    }
}

fn region_path(node: &SemanticNode, by_id: &HashMap<&NodeId, &SemanticNode>) -> Vec<String> {
    let mut result = Vec::new();
    let mut parent = node.parent.as_ref();
    while let Some(id) = parent {
        let Some(node) = by_id.get(id) else {
            break;
        };
        if node.role.is_region() && !node.name.is_empty() {
            result.push(node.name.clone());
        }
        parent = node.parent.as_ref();
    }
    result.reverse();
    result
}

pub fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Locator {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<ViewportSelector>,
    #[serde(default)]
    pub volatile: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PointerButton {
    #[default]
    Primary,
    Secondary,
    Middle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Action {
    Click {
        target: Locator,
        #[serde(default)]
        button: PointerButton,
        #[serde(default)]
        force: bool,
    },
    DoubleClick {
        target: Locator,
    },
    Hover {
        target: Locator,
    },
    Fill {
        target: Locator,
        text: String,
    },
    Clear {
        target: Locator,
    },
    Type {
        target: Locator,
        text: String,
    },
    Press {
        key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        viewport: Option<ViewportSelector>,
    },
    DragTo {
        source: Locator,
        target: Locator,
    },
    Scroll {
        delta_x: f32,
        delta_y: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        viewport: Option<ViewportSelector>,
    },
    FocusViewport {
        viewport: ViewportSelector,
    },
    MinimizeViewport {
        viewport: ViewportSelector,
    },
    RestoreViewport {
        viewport: ViewportSelector,
    },
    CloseViewport {
        viewport: ViewportSelector,
    },
    ScrollIntoView {
        target: Locator,
    },
    SetValue {
        target: Locator,
        value: String,
    },
}

// ── the coach touchpoint router ────────────────────────────────────────
//
// A touchpoint is a mounted matcher + visual + trigger contract, in the
// axum sense: the agent mounts it, the target fires typed events when it
// activates, and the agent (or a `once` trigger) unmounts it. A spotlight
// highlight the operator presses is one mounted touchpoint; an invisible
// tripwire is the same primitive with a different visual and trigger.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoachStyle {
    /// Dim the surroundings, tint and ring the target. The only C1 visual.
    #[default]
    Spotlight,
    Outline,
    Arrow,
    DimAround,
    /// An invisible sensor — a tripwire that paints nothing.
    None,
}

/// Every trigger is an **edge**, never a level: a touchpoint fires on the
/// transition, so a `once: false` touchpoint over a hovered node does not
/// re-fire every frame the pointer sits there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerKind {
    /// A pointer press inside the target rect.
    #[default]
    Press,
    /// The pointer entered the target rect.
    Hover,
    /// The target resolved after not being on screen. The only trigger that
    /// may be mounted against a target that does not exist yet.
    NodeAppeared,
    /// The target's reported value differs from the one it had at mount.
    ValueChanged,
    /// The application published this signal on its own fabric. The app
    /// chooses what to publish; the toolkit never reads fabric itself.
    FabricEvent { signal: SignalId },
}

/// Who activated the touchpoint: the human being coached, the agent
/// demonstrating, or neither. Coach mode and demo mode are one system,
/// split here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoachActor {
    Operator,
    Agent,
    /// The application state moved with no input to explain it — an appeared
    /// node, a published signal. Calling that an operator press would be a
    /// lie the agent would then reason from.
    App,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoachUnmountReason {
    Ttl,
    Operator,
    Agent,
    Once,
    TargetLost,
    SessionEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoachRefusal {
    TargetNotFound,
    TargetAmbiguous,
    TargetNotVisible,
    DuplicateId,
    UnknownTouchpoint,
    UnsupportedStyle,
    UnsupportedTrigger,
}

/// A **level**, not an edge — the deliberate counterpart to [`TriggerKind`].
///
/// A trigger reports that something *happened*; a condition reports that the
/// application *is* in a state. A lesson step needs the second: "you are on
/// the Episodes route" must be true for someone who navigated there a moment
/// ago and equally for someone who was already there, or every step a learner
/// has already satisfied hangs waiting for a transition that will not repeat.
///
/// The vocabulary is deliberately narrow because the semantic tree already
/// carries what a lesson asks about — an active rail subroute reports
/// `selected` while its siblings report `available`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CoachCondition {
    /// The node is on screen and reports this value.
    NodeValue { node: NodeId, value: String },
    /// The node is on screen.
    NodeVisible { node: NodeId },
    /// The node is not on screen. The state a step reaches by closing
    /// something.
    NodeGone { node: NodeId },
    /// A typed state flag, which a value string cannot be trusted to carry.
    /// Unset is not `false`: a widget that does not report the flag at all
    /// answers neither `true` nor `false`, and this condition stays unmet
    /// rather than guessing.
    NodeFlag {
        node: NodeId,
        flag: NodeFlagKind,
        on: bool,
    },
}

/// The flags on [`NodeState`] a condition may ask about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeFlagKind {
    Expanded,
    Selected,
    Checked,
    Focused,
    Disabled,
}

impl NodeFlagKind {
    /// `None` when the node does not report this flag at all.
    pub fn read(self, state: &NodeState) -> Option<bool> {
        match self {
            Self::Expanded => state.expanded,
            Self::Selected => state.selected,
            Self::Checked => state.checked,
            Self::Focused => Some(state.focused),
            Self::Disabled => Some(state.disabled),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Touchpoint {
    pub coach_id: CoachId,
    pub target: Locator,
    #[serde(default)]
    pub visual: CoachStyle,
    #[serde(default)]
    pub on: TriggerKind,
    /// The postcondition that actually completes the step, read every pass and
    /// satisfied at once when already true. Separate from `on`, which only
    /// says what input to expect: a press is a hint, the state is the truth.
    /// `None` keeps the pre-2.5 behavior of completing on the trigger alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<CoachCondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    #[serde(default)]
    pub once: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CoachCommand {
    Mount { touchpoint: Box<Touchpoint> },
    Unmount { coach_id: CoachId },
    UnmountAll,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CoachEvent {
    /// Accepted, but its target is not on screen yet. Any trigger aimed by
    /// node id mounts this way when its target has not appeared — a mount is
    /// a subscription, not a resolution — and `Mounted` follows once the
    /// target resolves. A name or role locator cannot be waited for.
    Armed {
        coach_id: CoachId,
    },
    Mounted {
        coach_id: CoachId,
        rect: Rect,
    },
    Triggered {
        coach_id: CoachId,
        kind: TriggerKind,
        actor: CoachActor,
    },
    /// The touchpoint's postcondition holds. Distinct from `Triggered`, which
    /// reports an input edge: the step is complete because the application
    /// reached the state, not because a button was pressed. Emitted on the
    /// first pass the condition is true, which may be the mounting pass.
    Satisfied {
        coach_id: CoachId,
    },
    Unmounted {
        coach_id: CoachId,
        reason: CoachUnmountReason,
    },
    Refused {
        coach_id: CoachId,
        reason: CoachRefusal,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        detail: String,
    },
}

impl CoachEvent {
    pub fn coach_id(&self) -> &CoachId {
        match self {
            Self::Armed { coach_id }
            | Self::Mounted { coach_id, .. }
            | Self::Triggered { coach_id, .. }
            | Self::Satisfied { coach_id }
            | Self::Unmounted { coach_id, .. }
            | Self::Refused { coach_id, .. } => coach_id,
        }
    }

    pub fn state(&self) -> CoachExpectationState {
        match self {
            Self::Armed { .. } => CoachExpectationState::Armed,
            Self::Mounted { .. } => CoachExpectationState::Mounted,
            Self::Triggered { .. } => CoachExpectationState::Triggered,
            Self::Satisfied { .. } => CoachExpectationState::Satisfied,
            Self::Unmounted { .. } => CoachExpectationState::Unmounted,
            Self::Refused { .. } => CoachExpectationState::Refused,
        }
    }

    /// A mounted touchpoint's lifetime ends on exactly one of these.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Unmounted { .. } | Self::Refused { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoachExpectationState {
    Armed,
    Mounted,
    Triggered,
    Satisfied,
    Unmounted,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Expectation {
    Visible {
        target: Locator,
    },
    Hidden {
        target: Locator,
    },
    Enabled {
        target: Locator,
    },
    Disabled {
        target: Locator,
    },
    Focused {
        target: Locator,
    },
    Text {
        target: Locator,
        value: String,
    },
    Value {
        target: Locator,
        value: String,
    },
    Count {
        target: Locator,
        value: usize,
    },
    Viewport {
        viewport: ViewportSelector,
        state: ViewportExpectationState,
    },
    Coach {
        coach_id: CoachId,
        state: CoachExpectationState,
    },
    NoDiagnostics,
    Screenshot {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Step {
    Action {
        action: Action,
    },
    Coach {
        coach: CoachCommand,
    },
    Expect {
        expectation: Expectation,
    },
    Wait {
        milliseconds: u64,
    },
    Capture {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        viewport: Option<ViewportSelector>,
    },
}

/// Which experiment, and which arm of it, a scenario belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentRef {
    /// The experiment document's id.
    pub experiment: String,
    /// The arm. Two scenarios sharing an experiment with different variants
    /// are an A/B; one variant alone is a plain experiment.
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub id: ScenarioId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<FixtureId>,
    /// The experiment this scenario is an arm of, if any.
    ///
    /// Declared HERE rather than looked up from the experiment document at
    /// run time, so a run bundle is self-describing: evidence has to be
    /// interpretable standing alone, without the authoring tree beside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment: Option<ExperimentRef>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub record: bool,
}

impl Flow {
    /// Derive a durable step identity from scenario identity, canonical step
    /// content, and the occurrence count of an identical step. Inserting an
    /// unrelated step does not renumber the existing identities.
    pub fn step_id(&self, index: usize) -> StepId {
        let Some(step) = self.steps.get(index) else {
            return StepId::new(format!("{}:missing-{index}", self.id));
        };
        let bytes = serde_json::to_vec(step).unwrap_or_default();
        let hash = stable_hash(&bytes);
        let occurrence = self.steps[..index]
            .iter()
            .filter(|candidate| serde_json::to_vec(candidate).unwrap_or_default() == bytes)
            .count();
        StepId::new(format!("{}:{hash}:{occurrence}", self.id))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterPolicy {
    SoftwareRequired,
    HardwareRequired,
    HardwarePreferred,
    #[default]
    HardwareThenSoftware,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub profile: String,
    #[serde(default)]
    pub adapter_policy: AdapterPolicy,
    #[serde(default)]
    pub required: bool,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default)]
    pub retries: usize,
}

fn default_workers() -> usize {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RendererIdentity {
    pub os: String,
    pub backend: String,
    pub adapter: String,
    pub device_type: String,
    pub driver: String,
    pub renderer_version: String,
    pub surface_format: String,
    pub pixels_per_point: f32,
    pub font_bundle: String,
    pub dithering: bool,
}

impl RendererIdentity {
    pub fn baseline_key(&self) -> String {
        let raw = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.os,
            self.backend,
            self.adapter,
            self.device_type,
            self.driver,
            self.renderer_version,
            self.surface_format,
            self.pixels_per_point,
            self.font_bundle,
            self.dithering
        );
        format!("{}-{}", sanitize_key(&self.os), stable_hash(raw.as_bytes()))
    }
}

fn sanitize_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub mod capability {
    pub const PROTOCOL_V2: &str = "protocol.v2";
    pub const INTERLEAVED_EVENTS: &str = "protocol.interleaved-events";
    pub const CANCELLATION: &str = "protocol.cancellation";
    pub const SNAPSHOT_AGENT: &str = "snapshot.agent";
    pub const SNAPSHOT_CHANGES: &str = "snapshot.changes";
    pub const SNAPSHOT_FULL: &str = "snapshot.full";
    pub const ACTION_CLICK: &str = "action.click";
    pub const ACTION_DOUBLE_CLICK: &str = "action.double-click";
    pub const ACTION_HOVER: &str = "action.hover";
    pub const ACTION_FILL: &str = "action.fill";
    pub const ACTION_CLEAR: &str = "action.clear";
    pub const ACTION_TYPE: &str = "action.type";
    pub const ACTION_PRESS: &str = "action.press";
    pub const ACTION_DRAG: &str = "action.drag";
    pub const ACTION_SCROLL: &str = "action.scroll";
    pub const ACTION_SET_VALUE: &str = "action.set-value";
    pub const ACTION_SCROLL_INTO_VIEW: &str = "action.scroll-into-view";
    pub const EXPECT_SEMANTIC: &str = "expect.semantic";
    pub const EXPECT_SCREENSHOT: &str = "expect.screenshot";
    pub const CAPTURE_DETERMINISTIC: &str = "capture.deterministic";
    pub const VIEWPORT_INVENTORY: &str = "viewport.inventory";
    pub const VIEWPORT_LIFECYCLE: &str = "viewport.lifecycle";
    pub const VIEWPORT_TARGETED_INPUT: &str = "viewport.targeted-input";
    pub const VIEWPORT_TARGETED_CAPTURE: &str = "viewport.targeted-capture";
    pub const VIEWPORT_ROOT_CAPTURE: &str = "viewport.capture.root";
    pub const VIEWPORT_IMMEDIATE_CAPTURE: &str = "viewport.capture.immediate";
    pub const VIEWPORT_DEFERRED_CAPTURE: &str = "viewport.capture.deferred";
    pub const VIEWPORT_FOCUS: &str = "viewport.focus";
    pub const VIEWPORT_MINIMIZE: &str = "viewport.minimize";
    pub const VIEWPORT_RESTORE: &str = "viewport.restore";
    pub const VIEWPORT_CLOSE: &str = "viewport.close";
    pub const VIEWPORT_IMMEDIATE: &str = "viewport.immediate";
    pub const VIEWPORT_DEFERRED: &str = "viewport.deferred";
    pub const VIEWPORT_NATIVE: &str = "viewport.native";
    pub const RUNNER_WORKERS: &str = "runner.workers";
    pub const RUNNER_RETRIES: &str = "runner.retries";
    pub const COACH_TOUCHPOINT: &str = "coach.touchpoint";
    pub const COACH_VISUAL_EXTENDED: &str = "coach.visual.extended";
    pub const COACH_TRIGGER_DYNAMIC: &str = "coach.trigger.dynamic";
    pub const COACH_SEMANTIC_TICKER: &str = "coach.semantic-ticker";
    pub const COACH_DESKTOP_OVERLAY: &str = "coach.desktop-overlay";
    pub const CODEGEN_RECORDER: &str = "codegen.recorder";
    pub const TRACE_REPLAY: &str = "trace.replay";
    pub const VISUAL_APPROVAL: &str = "visual.approval";
    pub const TEST_LAB_LIVE_CONTROL: &str = "test-lab.live-control";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityState {
    Supported,
    Experimental,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub state: CapabilityState,
    pub detail: String,
    pub evidence: String,
}

pub fn capability_catalog() -> Vec<CapabilityDescriptor> {
    use capability as id;
    let supported = |value: &str, detail: &str, evidence: &str| CapabilityDescriptor {
        id: CapabilityId::new(value),
        state: CapabilityState::Supported,
        detail: detail.into(),
        evidence: evidence.into(),
    };
    let unsupported = |value: &str, detail: &str, evidence: &str| CapabilityDescriptor {
        id: CapabilityId::new(value),
        state: CapabilityState::Unsupported,
        detail: detail.into(),
        evidence: evidence.into(),
    };
    vec![
        supported(
            id::PROTOCOL_V2,
            "typed command/response/event envelopes",
            "slice-1",
        ),
        supported(
            id::INTERLEAVED_EVENTS,
            "events are routed independently from responses",
            "slice-1",
        ),
        supported(
            id::CANCELLATION,
            "active run cancellation is idempotent",
            "slice-1",
        ),
        supported(
            id::SNAPSHOT_AGENT,
            "bounded pruned agent projection",
            "protocol::agent_projection_prunes_layout_and_decorative_nodes",
        ),
        supported(
            id::SNAPSHOT_CHANGES,
            "revision-scoped projection",
            "protocol::changes_mode_uses_node_revision",
        ),
        supported(
            id::SNAPSHOT_FULL,
            "bounded diagnostic full projection",
            "protocol::projection_is_bounded_and_continuable_without_loss",
        ),
        supported(
            id::ACTION_CLICK,
            "real pointer click frames",
            "runtime::action_batches",
        ),
        supported(
            id::ACTION_DOUBLE_CLICK,
            "two real pointer click sequences",
            "runtime::action_batches",
        ),
        supported(
            id::ACTION_HOVER,
            "real pointer move",
            "runtime::action_batches",
        ),
        supported(
            id::ACTION_FILL,
            "focus, select-all, and text input",
            "runtime::action_batches",
        ),
        supported(
            id::ACTION_CLEAR,
            "focus, select-all, and delete",
            "runtime::action_batches",
        ),
        supported(
            id::ACTION_TYPE,
            "focus and text input",
            "runtime::action_batches",
        ),
        supported(
            id::ACTION_PRESS,
            "key down/up input",
            "runtime::action_batches",
        ),
        supported(
            id::ACTION_DRAG,
            "pointer move/press/move/release",
            "runtime::action_batches",
        ),
        supported(id::ACTION_SCROLL, "wheel input", "runtime::action_batches"),
        supported(
            id::EXPECT_SEMANTIC,
            "semantic state and text assertions",
            "runtime::evaluate_expectation",
        ),
        supported(
            id::CAPTURE_DETERMINISTIC,
            "motion-suppressed stable capture",
            "runtime::capture_waits_for_three_stable_hashes",
        ),
        supported(
            id::VIEWPORT_INVENTORY,
            "typed live viewport inventory with durable keys and lifetime ids",
            "slice-2b",
        ),
        supported(
            id::VIEWPORT_LIFECYCLE,
            "created, changed, close-requested, and closed lifecycle events",
            "slice-2b",
        ),
        supported(
            id::VIEWPORT_TARGETED_INPUT,
            "viewport-bound keyboard and pointer input",
            "slice-2b",
        ),
        supported(
            id::VIEWPORT_TARGETED_CAPTURE,
            "strict target resolution, viewport-matched screenshot events, and viewport-scoped settlement",
            "runtime::start_capture",
        ),
        supported(
            id::VIEWPORT_ROOT_CAPTURE,
            "eframe wgpu screenshot delivery for the root native viewport",
            "shell-navigation-real-process",
        ),
        unsupported(
            id::VIEWPORT_IMMEDIATE_CAPTURE,
            "eframe 0.35 native wgpu renders immediate viewports with an empty screenshot request list",
            "eframe-0.35.0/native/wgpu_integration.rs::render_immediate_viewport",
        ),
        unsupported(
            id::VIEWPORT_DEFERRED_CAPTURE,
            "eframe 0.35 native wgpu does not deliver queued screenshot requests for background deferred viewports",
            "chat-peer-window-real-process",
        ),
        supported(
            id::VIEWPORT_FOCUS,
            "native viewport focus command with observable confirmation",
            "slice-2b",
        ),
        supported(
            id::VIEWPORT_MINIMIZE,
            "native viewport minimize command with observable confirmation",
            "chat-peer-window-real-process",
        ),
        supported(
            id::VIEWPORT_RESTORE,
            "native viewport restore command with observable confirmation",
            "chat-peer-window-real-process",
        ),
        supported(
            id::VIEWPORT_CLOSE,
            "native viewport close command with lifecycle confirmation",
            "slice-2b",
        ),
        supported(
            id::VIEWPORT_IMMEDIATE,
            "declared immediate viewport registration",
            "viziwiz-real-process",
        ),
        supported(
            id::VIEWPORT_DEFERRED,
            "declared deferred viewport registration, independent repaint, input, focus, and lifecycle",
            "chat-peer-window-real-process",
        ),
        supported(
            id::VIEWPORT_NATIVE,
            "native viewport operation rejects embedded fallback",
            "slice-2b",
        ),
        unsupported(
            id::ACTION_SET_VALUE,
            "requires a typed value adapter",
            "slice-3",
        ),
        unsupported(
            id::ACTION_SCROLL_INTO_VIEW,
            "requires a registered scroll adapter",
            "slice-3",
        ),
        unsupported(
            id::EXPECT_SCREENSHOT,
            "use a capture step; baseline assertions are not wired",
            "slice-5",
        ),
        unsupported(
            id::RUNNER_WORKERS,
            "the scheduler is not implemented",
            "slice-4",
        ),
        unsupported(
            id::RUNNER_RETRIES,
            "isolated retry attempts are not implemented",
            "slice-4",
        ),
        supported(
            id::COACH_TOUCHPOINT,
            "mount a spotlight touchpoint on an instrumented node and observe an operator or agent press",
            "runtime::coach_press_triggers_once_and_unmounts",
        ),
        unsupported(
            id::COACH_VISUAL_EXTENDED,
            "outline, arrow, dim-around, and invisible touchpoint visuals are not painted yet",
            "coach-c1",
        ),
        supported(
            id::COACH_TRIGGER_DYNAMIC,
            "hover, node-appeared, value-changed, and app-published fabric signals fire touchpoints from in-app state",
            "runtime::an_armed_touchpoint_mounts_when_its_target_appears",
        ),
        unsupported(
            id::COACH_SEMANTIC_TICKER,
            "agents still poll snapshots; the semantic revision ticker is not published yet",
            "coach-c1.5b",
        ),
        unsupported(
            id::COACH_DESKTOP_OVERLAY,
            "painting touchpoints outside the app window needs the coach-overlay daemon and its ADR",
            "coach-c2",
        ),
        unsupported(
            id::CODEGEN_RECORDER,
            "codegen only normalizes an existing scenario",
            "slice-6",
        ),
        unsupported(id::TRACE_REPLAY, "trace preview is passive", "slice-5"),
        unsupported(
            id::VISUAL_APPROVAL,
            "approval primitives are not an operator workflow",
            "slice-5",
        ),
        unsupported(
            id::TEST_LAB_LIVE_CONTROL,
            "Test Lab does not own a live target session",
            "slice-6",
        ),
    ]
}

pub fn supported_capabilities() -> Vec<CapabilityId> {
    capability_catalog()
        .into_iter()
        .filter(|entry| entry.state == CapabilityState::Supported)
        .map(|entry| entry.id)
        .collect()
}

pub fn required_capabilities(flow: &Flow) -> Vec<CapabilityId> {
    use capability as id;
    fn push(required: &mut Vec<CapabilityId>, value: &str) {
        let capability = CapabilityId::new(value);
        if !required.contains(&capability) {
            required.push(capability);
        }
    }
    let mut required = Vec::new();
    for step in &flow.steps {
        let value = match step {
            Step::Action { action } => match action {
                Action::Click { .. } => id::ACTION_CLICK,
                Action::DoubleClick { .. } => id::ACTION_DOUBLE_CLICK,
                Action::Hover { .. } => id::ACTION_HOVER,
                Action::Fill { .. } => id::ACTION_FILL,
                Action::Clear { .. } => id::ACTION_CLEAR,
                Action::Type { .. } => id::ACTION_TYPE,
                Action::Press { .. } => id::ACTION_PRESS,
                Action::DragTo { .. } => id::ACTION_DRAG,
                Action::Scroll { .. } => id::ACTION_SCROLL,
                Action::FocusViewport { .. } => id::VIEWPORT_FOCUS,
                Action::MinimizeViewport { .. } => id::VIEWPORT_MINIMIZE,
                Action::RestoreViewport { .. } => id::VIEWPORT_RESTORE,
                Action::CloseViewport { .. } => id::VIEWPORT_CLOSE,
                Action::ScrollIntoView { .. } => id::ACTION_SCROLL_INTO_VIEW,
                Action::SetValue { .. } => id::ACTION_SET_VALUE,
            },
            Step::Coach { coach } => {
                if let CoachCommand::Mount { touchpoint } = coach {
                    if touchpoint.visual != CoachStyle::Spotlight {
                        push(&mut required, id::COACH_VISUAL_EXTENDED);
                    }
                    if touchpoint.on != TriggerKind::Press {
                        push(&mut required, id::COACH_TRIGGER_DYNAMIC);
                    }
                }
                id::COACH_TOUCHPOINT
            }
            Step::Expect {
                expectation: Expectation::Screenshot { .. },
            } => id::EXPECT_SCREENSHOT,
            Step::Expect {
                expectation: Expectation::Viewport { .. },
            } => id::VIEWPORT_LIFECYCLE,
            Step::Expect {
                expectation: Expectation::Coach { .. },
            } => id::COACH_TOUCHPOINT,
            Step::Expect { .. } => id::EXPECT_SEMANTIC,
            Step::Capture { viewport, .. } => {
                if viewport.is_some() {
                    push(&mut required, id::VIEWPORT_TARGETED_CAPTURE);
                }
                id::CAPTURE_DETERMINISTIC
            }
            Step::Wait { .. } => continue,
        };
        push(&mut required, value);
    }
    required
}

pub fn unsupported_capabilities(flow: &Flow) -> Vec<CapabilityDescriptor> {
    let required = required_capabilities(flow);
    capability_catalog()
        .into_iter()
        .filter(|entry| required.contains(&entry.id) && entry.state != CapabilityState::Supported)
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceLevel {
    Off,
    Failure,
    #[default]
    RetainOnFailure,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolLimits {
    pub max_command_bytes: usize,
    pub max_event_bytes: usize,
    pub max_attachment_descriptor_bytes: usize,
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        Self {
            max_command_bytes: MAX_ENVELOPE_BYTES,
            max_event_bytes: MAX_ENVELOPE_BYTES,
            max_attachment_descriptor_bytes: 64 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuePolicy {
    pub command_capacity: usize,
    pub required_event_capacity: usize,
    pub telemetry_capacity: usize,
    pub telemetry_coalesces: bool,
}

impl Default for QueuePolicy {
    fn default() -> Self {
        Self {
            command_capacity: 64,
            required_event_capacity: 256,
            telemetry_capacity: 32,
            telemetry_coalesces: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub protocol: ProtocolVersion,
    pub build: String,
    pub session_nonce: String,
    #[serde(default)]
    pub required_capabilities: Vec<CapabilityId>,
    #[serde(default)]
    pub trace_level: TraceLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Handshake {
    pub protocol: ProtocolVersion,
    pub build: String,
    pub session_nonce: String,
    #[serde(default)]
    pub features: Vec<String>,
    pub capabilities: Vec<CapabilityId>,
    pub limits: ProtocolLimits,
    pub queues: QueuePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<RendererIdentity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub request_id: RequestId,
    pub session: SessionId,
    pub command: ClientCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientCommand {
    Handshake(HandshakeRequest),
    Snapshot(SnapshotRequest),
    Run {
        run_id: RunId,
        flow: Flow,
    },
    Capture {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        viewport: Option<ViewportSelector>,
    },
    Coach {
        coach: CoachCommand,
    },
    Status,
    Cancel {
        run_id: RunId,
    },
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub request_id: RequestId,
    pub session: SessionId,
    pub outcome: ResponseOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ResponseOutcome {
    Success { result: Box<CommandResult> },
    Error { error: PilotError },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CommandResult {
    Ready { handshake: Handshake },
    Snapshot { snapshot: AgentSnapshot },
    Run { result: RunResult },
    Status { status: SessionStatus },
    Capture { capture: CaptureResult },
    Coach { events: Vec<CoachEvent> },
    Cancel { cancellation: CancellationResult },
    Stopped,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<RunId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<StepId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport_id: Option<ViewportId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEventEnvelope {
    pub event_id: EventId,
    pub session: SessionId,
    #[serde(default)]
    pub scope: EventScope,
    pub event: RuntimeEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RuntimeEvent {
    RunStarted { scenario: ScenarioId },
    StepStarted { index: usize },
    StepFinished { outcome: StepOutcome },
    AttachmentCommitted { capture: CaptureResult },
    Diagnostic { error: PilotError },
    QueuePressure { queue: String, dropped: u64 },
    Telemetry { channel: String, revision: u64 },
    InputCleanup,
    ViewportLifecycle { event: ViewportLifecycleEvent },
    Coach { event: CoachEvent },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ServerMessage {
    Response {
        #[serde(flatten)]
        envelope: ResponseEnvelope,
    },
    Event {
        #[serde(flatten)]
        envelope: Box<RuntimeEventEnvelope>,
    },
}

impl ServerMessage {
    pub fn response(envelope: ResponseEnvelope) -> Self {
        Self::Response { envelope }
    }

    pub fn event(envelope: RuntimeEventEnvelope) -> Self {
        Self::Event {
            envelope: Box::new(envelope),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepOutcome {
    pub id: StepId,
    pub index: usize,
    pub passed: bool,
    pub elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Box<AgentSnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionStatus {
    pub running: bool,
    pub revision: u64,
    pub pending_actions: usize,
    pub busy_tokens: usize,
    pub viewport_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewports: Vec<ViewportDescriptor>,
    pub renderer_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer_device_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_run: Option<RunId>,
    /// Every signal a `FabricEvent` touchpoint may wait for in this build.
    ///
    /// The host declares it; the toolkit cannot discover it, because the
    /// toolkit cannot read the host's signal fabric. An empty list means the
    /// host publishes nothing, not that anything is waitable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coach_signals: Vec<SignalId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureResult {
    pub attachment: AttachmentId,
    pub name: String,
    pub artifact: String,
    pub content_hash: String,
    pub renderer_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<ViewportDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_size: Option<[u32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunOutcome {
    Passed,
    Failed,
    Cancelled,
    InfrastructureFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunResult {
    pub run_id: RunId,
    pub scenario: ScenarioId,
    pub outcome: RunOutcome,
    #[serde(default)]
    pub steps: Vec<StepOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<PilotError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CancellationState {
    Requested,
    AlreadyRequested,
    AlreadyTerminal,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancellationResult {
    pub run_id: RunId,
    pub state: CancellationState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PilotError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Box<AgentSnapshot>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, role: Role, actions: Vec<ActionKind>) -> SemanticNode {
        SemanticNode {
            id: NodeId::new(id),
            viewport: ViewportId::new("root"),
            parent: None,
            role,
            name: id.to_string(),
            description: None,
            value: None,
            rect: Rect {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 100.0,
                max_y: 20.0,
            },
            clip_rect: Rect {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 100.0,
                max_y: 100.0,
            },
            state: NodeState::default(),
            actions,
            layer: 0,
            changed_at: 1,
            source: None,
        }
    }

    fn root_viewport(title: &str) -> ViewportDescriptor {
        ViewportDescriptor {
            id: ViewportId::new("root"),
            key: PilotViewportKey::new("root"),
            parent: None,
            class: ViewportClass::Root,
            title: title.into(),
            lifecycle: ViewportLifecycle::Open,
            lifecycle_revision: 1,
            close_requested: false,
            visible: true,
            focused: true,
            minimized: false,
            maximized: false,
            occluded: false,
            inner_rect: None,
            outer_rect: None,
            native_pixels_per_point: Some(1.0),
        }
    }

    #[test]
    fn agent_projection_prunes_layout_and_decorative_nodes() {
        let mut layout = node("layout", Role::Generic, vec![]);
        layout.state.decorative = true;
        let frame = SemanticFrame {
            revision: 4,
            timestamp_seconds: 0.0,
            nodes: vec![
                layout,
                node("save", Role::Button, vec![ActionKind::Click]),
                node("title", Role::Heading, vec![]),
            ],
            viewports: Vec::new(),
            diagnostics: FrameDiagnostics::default(),
        };
        let snapshot = project_snapshot(&frame, &SnapshotRequest::default()).unwrap();
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.nodes[0].id, NodeId::new("save"));
        assert_eq!(snapshot.nodes[1].id, NodeId::new("title"));
    }

    #[test]
    fn projection_is_bounded_and_continuable_without_loss() {
        let frame = SemanticFrame {
            revision: 9,
            timestamp_seconds: 0.0,
            nodes: (0..25)
                .map(|index| {
                    node(
                        &format!("button-{index}"),
                        Role::Button,
                        vec![ActionKind::Click],
                    )
                })
                .collect(),
            viewports: Vec::new(),
            diagnostics: FrameDiagnostics::default(),
        };
        let mut request = SnapshotRequest {
            max_nodes: Some(7),
            ..SnapshotRequest::default()
        };
        let mut ids = Vec::new();
        loop {
            let page = project_snapshot(&frame, &request).unwrap();
            ids.extend(page.nodes.into_iter().map(|node| node.id));
            let Some(token) = page.continuation else {
                break;
            };
            request.continuation = Some(token);
        }
        assert_eq!(ids.len(), 25);
        ids.sort_by(|a, b| a.0.cmp(&b.0));
        ids.dedup();
        assert_eq!(ids.len(), 25);
    }

    #[test]
    fn changes_mode_uses_node_revision() {
        let mut old = node("old", Role::Button, vec![ActionKind::Click]);
        old.changed_at = 2;
        let mut fresh = node("fresh", Role::Button, vec![ActionKind::Click]);
        fresh.changed_at = 6;
        let frame = SemanticFrame {
            revision: 6,
            timestamp_seconds: 0.0,
            nodes: vec![old, fresh],
            viewports: Vec::new(),
            diagnostics: FrameDiagnostics::default(),
        };
        let request = SnapshotRequest {
            mode: SnapshotMode::Changes,
            since_revision: Some(3),
            ..SnapshotRequest::default()
        };
        let snapshot = project_snapshot(&frame, &request).unwrap();
        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].id, NodeId::new("fresh"));
    }

    #[test]
    fn snapshot_viewport_keys_are_strict_and_part_of_the_hash() {
        let mut frame = SemanticFrame {
            revision: 1,
            timestamp_seconds: 0.0,
            nodes: vec![node("save", Role::Button, vec![ActionKind::Click])],
            viewports: vec![root_viewport("DevPULSE")],
            diagnostics: FrameDiagnostics::default(),
        };
        let request = SnapshotRequest {
            filter: SnapshotFilter {
                viewport: Some(ViewportSelector::by_key("root")),
                ..Default::default()
            },
            ..Default::default()
        };
        let first = project_snapshot(&frame, &request).expect("key-scoped snapshot");
        assert_eq!(first.nodes.len(), 1);

        frame.viewports[0].title = "Changed".into();
        let second = project_snapshot(&frame, &request).expect("updated snapshot");
        assert_ne!(first.content_hash, second.content_hash);

        let missing = SnapshotRequest {
            filter: SnapshotFilter {
                viewport: Some(ViewportSelector::by_key("missing")),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            project_snapshot(&frame, &missing),
            Err(SnapshotError::ViewportNotFound)
        );
    }

    #[test]
    fn renderer_key_separates_adapter_families() {
        let base = RendererIdentity {
            os: "linux".into(),
            backend: "vulkan".into(),
            adapter: "llvmpipe".into(),
            device_type: "cpu".into(),
            driver: "mesa".into(),
            renderer_version: "1".into(),
            surface_format: "rgba8-srgb".into(),
            pixels_per_point: 1.0,
            font_bundle: "dpl-v1".into(),
            dithering: false,
        };
        let mut hardware = base.clone();
        hardware.adapter = "nvidia".into();
        hardware.device_type = "discrete-gpu".into();
        assert_ne!(base.baseline_key(), hardware.baseline_key());
    }

    #[test]
    fn immediate_viewport_capture_is_explicitly_unsupported() {
        let capability = capability_catalog()
            .into_iter()
            .find(|entry| entry.id == CapabilityId::new(capability::VIEWPORT_IMMEDIATE_CAPTURE))
            .expect("immediate viewport capture capability");
        assert_eq!(capability.state, CapabilityState::Unsupported);
        assert!(capability.detail.contains("eframe 0.35"));
        assert!(capability.evidence.contains("render_immediate_viewport"));
    }

    fn mount(coach_id: &str, visual: CoachStyle, on: TriggerKind) -> Step {
        Step::Coach {
            coach: CoachCommand::Mount {
                touchpoint: Box::new(Touchpoint {
                    coach_id: CoachId::new(coach_id),
                    target: Locator {
                        id: Some(NodeId::new("root/settings.save")),
                        ..Locator::default()
                    },
                    visual,
                    on,
                    until: None,
                    message: Some("Press Save".into()),
                    ttl_ms: None,
                    once: true,
                }),
            },
        }
    }

    fn coach_flow(step: Step) -> Flow {
        Flow {
            id: ScenarioId::new("coach"),
            fixture: None,
            experiment: None,
            steps: vec![step],
            record: false,
        }
    }

    #[test]
    fn a_spotlight_press_touchpoint_needs_only_the_supported_coach_capability() {
        let flow = coach_flow(mount("coach-1", CoachStyle::Spotlight, TriggerKind::Press));
        assert_eq!(
            required_capabilities(&flow),
            vec![CapabilityId::new(capability::COACH_TOUCHPOINT)]
        );
        assert!(unsupported_capabilities(&flow).is_empty());
    }

    #[test]
    fn unshipped_visuals_are_rejected_before_a_run_starts() {
        let outline = coach_flow(mount("coach-2", CoachStyle::Outline, TriggerKind::Press));
        let refusals: Vec<_> = unsupported_capabilities(&outline)
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(
            refusals,
            vec![CapabilityId::new(capability::COACH_VISUAL_EXTENDED)]
        );
    }

    #[test]
    fn every_dynamic_trigger_declares_the_one_shipped_trigger_capability() {
        for trigger in [
            TriggerKind::Hover,
            TriggerKind::NodeAppeared,
            TriggerKind::ValueChanged,
            TriggerKind::FabricEvent {
                signal: SignalId::new("build.slide-placed"),
            },
        ] {
            let flow = coach_flow(mount("coach-3", CoachStyle::Spotlight, trigger.clone()));
            assert_eq!(
                required_capabilities(&flow),
                vec![
                    CapabilityId::new(capability::COACH_TRIGGER_DYNAMIC),
                    CapabilityId::new(capability::COACH_TOUCHPOINT),
                ],
                "{trigger:?}"
            );
            assert!(
                unsupported_capabilities(&flow).is_empty(),
                "{trigger:?} must not refuse"
            );
        }
    }

    #[test]
    fn exactly_one_coach_event_kind_is_terminal_per_lifetime() {
        let coach_id = CoachId::new("coach-4");
        let lifetime = [
            CoachEvent::Mounted {
                coach_id: coach_id.clone(),
                rect: Rect::default(),
            },
            CoachEvent::Triggered {
                coach_id: coach_id.clone(),
                kind: TriggerKind::Press,
                actor: CoachActor::Operator,
            },
            CoachEvent::Unmounted {
                coach_id: coach_id.clone(),
                reason: CoachUnmountReason::Once,
            },
        ];
        assert_eq!(
            lifetime.iter().filter(|event| event.is_terminal()).count(),
            1
        );
        assert!(lifetime.iter().all(|event| event.coach_id() == &coach_id));

        let refusal = CoachEvent::Refused {
            coach_id,
            reason: CoachRefusal::TargetNotFound,
            detail: "locator did not match".into(),
        };
        assert!(refusal.is_terminal());
        assert_eq!(refusal.state(), CoachExpectationState::Refused);
    }

    #[test]
    fn coach_commands_survive_a_wire_roundtrip() {
        let command = ClientCommand::Coach {
            coach: CoachCommand::Mount {
                touchpoint: Box::new(Touchpoint {
                    coach_id: CoachId::new("coach-5"),
                    target: Locator {
                        id: Some(NodeId::new("root/settings.save")),
                        ..Locator::default()
                    },
                    visual: CoachStyle::Spotlight,
                    on: TriggerKind::Press,
                    until: None,
                    message: Some("Press Save".into()),
                    ttl_ms: Some(15_000),
                    once: true,
                }),
            },
        };
        let encoded = serde_json::to_string(&command).expect("encode coach command");
        assert_eq!(
            serde_json::from_str::<ClientCommand>(&encoded).expect("decode coach command"),
            command
        );

        let event = RuntimeEvent::Coach {
            event: CoachEvent::Triggered {
                coach_id: CoachId::new("coach-5"),
                kind: TriggerKind::Press,
                actor: CoachActor::Agent,
            },
        };
        let encoded = serde_json::to_string(&event).expect("encode coach event");
        assert!(encoded.contains("\"actor\":\"agent\""));
        assert_eq!(
            serde_json::from_str::<RuntimeEvent>(&encoded).expect("decode coach event"),
            event
        );
    }

    #[test]
    fn a_fabric_trigger_carries_its_signal_and_keeps_unit_triggers_as_plain_strings() {
        let fabric = TriggerKind::FabricEvent {
            signal: SignalId::new("build.slide-placed"),
        };
        let encoded = serde_json::to_string(&fabric).expect("encode fabric trigger");
        assert_eq!(
            encoded,
            r#"{"fabric-event":{"signal":"build.slide-placed"}}"#
        );
        assert_eq!(
            serde_json::from_str::<TriggerKind>(&encoded).expect("decode fabric trigger"),
            fabric
        );
        // Scenario TOML already writes `on = "press"`; adding a data-carrying
        // variant must not turn the unit triggers into maps.
        assert_eq!(
            serde_json::to_string(&TriggerKind::NodeAppeared).expect("encode unit trigger"),
            r#""node-appeared""#
        );
    }

    #[test]
    fn arming_is_a_distinct_non_terminal_state_from_being_mounted() {
        let armed = CoachEvent::Armed {
            coach_id: CoachId::new("coach-6"),
        };
        assert!(!armed.is_terminal());
        assert_eq!(armed.state(), CoachExpectationState::Armed);
        assert_ne!(armed.state(), CoachExpectationState::Mounted);
    }

    #[test]
    fn deferred_viewport_lifecycle_is_supported_without_claiming_capture() {
        let catalog = capability_catalog();
        let state = |id: &str| {
            catalog
                .iter()
                .find(|entry| entry.id == CapabilityId::new(id))
                .map(|entry| entry.state)
                .expect("catalog capability")
        };

        assert_eq!(
            state(capability::VIEWPORT_DEFERRED),
            CapabilityState::Supported
        );
        assert_eq!(
            state(capability::VIEWPORT_MINIMIZE),
            CapabilityState::Supported
        );
        assert_eq!(
            state(capability::VIEWPORT_RESTORE),
            CapabilityState::Supported
        );
        assert_eq!(
            state(capability::VIEWPORT_DEFERRED_CAPTURE),
            CapabilityState::Unsupported
        );
    }
}
