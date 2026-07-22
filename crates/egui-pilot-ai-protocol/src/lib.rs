//! Versioned wire model shared by EguiPilotAI runtimes, runners, durable
//! scenarios, and MCP. This crate deliberately has no egui or OS dependency.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const PROTOCOL_VERSION: u32 = 1;
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
string_id!(FixtureId);
string_id!(ScenarioId);
string_id!(ProjectId);
string_id!(SessionId);
string_id!(MotionId);

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
    #[serde(default)]
    pub diagnostics: FrameDiagnostics,
}

impl Default for SemanticFrame {
    fn default() -> Self {
        Self {
            revision: 0,
            timestamp_seconds: 0.0,
            nodes: Vec::new(),
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
    pub viewport: Option<ViewportId>,
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
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContinuation => f.write_str("invalid snapshot continuation"),
            Self::StaleContinuation { expected, actual } => write!(
                f,
                "snapshot continuation is for revision {expected}, current revision is {actual}"
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

    let mut candidates: Vec<(u8, usize, AgentNode)> = frame
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| matches_filter(node, request, descendants.as_ref()))
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

    let mut nodes = Vec::new();
    let mut next = offset;
    let payload_budget = byte_limit.saturating_sub(512);
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
    let serialized = serde_json::to_vec(&nodes).unwrap_or_default();
    Ok(AgentSnapshot {
        revision: frame.revision,
        content_hash: stable_hash(&serialized),
        nodes,
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
) -> bool {
    if request
        .filter
        .viewport
        .as_ref()
        .is_some_and(|v| v != &node.viewport)
    {
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
    },
    DragTo {
        source: Locator,
        target: Locator,
    },
    Scroll {
        delta_x: f32,
        delta_y: f32,
    },
    ScrollIntoView {
        target: Locator,
    },
    SetValue {
        target: Locator,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Expectation {
    Visible { target: Locator },
    Hidden { target: Locator },
    Enabled { target: Locator },
    Disabled { target: Locator },
    Focused { target: Locator },
    Text { target: Locator, value: String },
    Value { target: Locator, value: String },
    Count { target: Locator, value: usize },
    NoDiagnostics,
    Screenshot { name: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Step {
    Action { action: Action },
    Expect { expectation: Expectation },
    Wait { milliseconds: u64 },
    Capture { name: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub id: ScenarioId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<FixtureId>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub record: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    pub protocol_version: u32,
    pub build: String,
    pub session_nonce: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientEnvelope {
    pub sequence: u64,
    pub session: SessionId,
    pub command: ClientCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientCommand {
    Handshake(Handshake),
    Snapshot(SnapshotRequest),
    Run(Flow),
    Capture { name: String },
    Status,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerEnvelope {
    pub sequence: u64,
    pub session: SessionId,
    pub event: ServerEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerEvent {
    Ready(Handshake),
    Snapshot(AgentSnapshot),
    Step(StepOutcome),
    Status(SessionStatus),
    Capture(CaptureResult),
    Error(PilotError),
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepOutcome {
    pub index: usize,
    pub passed: bool,
    pub elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Box<AgentSnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStatus {
    pub running: bool,
    pub revision: u64,
    pub pending_actions: usize,
    pub busy_tokens: usize,
    pub viewport_count: usize,
    pub renderer_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer_device_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureResult {
    pub name: String,
    pub artifact: String,
    pub content_hash: String,
    pub renderer_key: String,
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
}
