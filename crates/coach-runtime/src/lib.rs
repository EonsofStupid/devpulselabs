//! FabricGraph runtime bridge — resolve / invoke / extents by fabric address.
//!
//! Viewport-independent control uses Tier A Action (`doAction`), not click-xy.
//! Overlay paint is optional Mount feedback. Plan:
//! `docs/plans/coach-bridge-fabric-runtime.md`.

#![forbid(unsafe_code)]

mod resolve;

#[cfg(target_os = "linux")]
mod linux;

use dpl_coach_fabric::{Fabric, FabricNode, Refused};
use std::time::{SystemTime, UNIX_EPOCH};

/// Who may drive the bridge. Teacher uses the **same** resolve/invoke/mount
/// rails — never a second stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgencyMode {
    /// Operator leads; lesson typically Mount + observe.
    Coach,
    /// AI may open + execute via invoke on fabric addresses.
    Teacher,
}

impl AgencyMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Coach => "coach",
            Self::Teacher => "teacher",
        }
    }
}

/// Addressable node borrowed from a minted fabric (AI / Teacher speak this).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRef {
    pub path: String,
    pub role: String,
    pub name: String,
    pub actions: Vec<String>,
}

impl From<&FabricNode> for NodeRef {
    fn from(n: &FabricNode) -> Self {
        Self {
            path: n.path.clone(),
            role: n.role.clone(),
            name: n.name.clone(),
            actions: n.actions.clone(),
        }
    }
}

/// WINDOW-space integer rect (AT-SPI Component extents).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn as_csv(self) -> String {
        format!("{},{},{},{}", self.x, self.y, self.w, self.h)
    }
}

/// Session-held fabric for one open lesson / Teacher turn.
#[derive(Debug, Clone)]
pub struct FabricSession {
    pub fabric: Fabric,
    pub minted_unix_ms: u64,
    pub agency: AgencyMode,
}

impl FabricSession {
    pub fn new(fabric: Fabric, agency: AgencyMode) -> Self {
        let minted_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            fabric,
            minted_unix_ms,
            agency,
        }
    }

    pub fn seed_node(&self) -> Option<NodeRef> {
        let path = self.fabric.seed_path.as_deref()?;
        self.fabric
            .nodes
            .iter()
            .find(|n| n.path == path)
            .map(NodeRef::from)
    }

    pub fn node_by_path(&self, path: &str) -> Option<NodeRef> {
        self.fabric
            .nodes
            .iter()
            .find(|n| n.path == path)
            .map(NodeRef::from)
    }

    pub fn node_by_name(&self, name: &str) -> Option<NodeRef> {
        self.fabric
            .nodes
            .iter()
            .find(|n| n.name == name)
            .map(NodeRef::from)
    }
}

/// What invoke reported (no screenshots).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvokeEffect {
    pub action: String,
    pub agency: AgencyMode,
}

/// Spec for ADR overlay Mount — rect from bridge resolve, not name-locate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpec {
    pub atspi_name: String,
    pub expected_name: String,
    pub window_rect: IRect,
    pub agency: AgencyMode,
}

/// Platform-opaque live accessible after resolve.
#[derive(Debug)]
pub struct LiveHandle {
    pub node: NodeRef,
    #[cfg(target_os = "linux")]
    pub(crate) linux: linux::LinuxLive,
    #[cfg(not(target_os = "linux"))]
    pub(crate) _seal: (),
}

/// Resolve a fabric address to a live accessible.
pub fn resolve(session: &FabricSession, node: &NodeRef) -> Result<LiveHandle, Refused> {
    #[cfg(target_os = "linux")]
    {
        linux::resolve_live(session, node)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (session, node);
        Err(Refused {
            reason: "fabric bridge resolve is Linux-first".into(),
            tip: "Use the Linux seat for CX-bridge-01, or wait for the Windows UIA port.".into(),
        })
    }
}

/// Resolve the session seed (or refuse).
pub fn resolve_seed(session: &FabricSession) -> Result<LiveHandle, Refused> {
    let Some(node) = session.seed_node() else {
        return Err(Refused {
            reason: "fabric has no seed_path".into(),
            tip: "Re-import with a seed control (Cursor Search Ctrl+K, Calculator 7).".into(),
        });
    };
    resolve(session, &node)
}

/// Invoke a named Action on a live handle (viewport-independent).
pub fn invoke(
    handle: &LiveHandle,
    action_name: &str,
    agency: AgencyMode,
) -> Result<InvokeEffect, Refused> {
    #[cfg(target_os = "linux")]
    {
        linux::invoke_action(handle, action_name, agency)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (handle, action_name, agency);
        Err(Refused {
            reason: "fabric bridge invoke is Linux-first".into(),
            tip: "Use the Linux seat for CX-bridge-01.".into(),
        })
    }
}

/// Live WINDOW extents for optional Mount paint.
pub fn extents(handle: &LiveHandle) -> Result<IRect, Refused> {
    #[cfg(target_os = "linux")]
    {
        linux::live_extents(handle)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = handle;
        Err(Refused {
            reason: "fabric bridge extents is Linux-first".into(),
            tip: "Use the Linux seat for Mount paint.".into(),
        })
    }
}

/// Build overlay MountSpec from a resolved handle (paint feedback only).
pub fn mount_spec(session: &FabricSession, handle: &LiveHandle) -> Result<MountSpec, Refused> {
    let window_rect = extents(handle)?;
    Ok(MountSpec {
        atspi_name: session.fabric.atspi_name.clone(),
        expected_name: handle.node.name.clone(),
        window_rect,
        agency: session.agency,
    })
}

/// Parse `0/1/2` fabric paths into child indices (skips the root `0` segment).
pub fn path_indices(path: &str) -> Result<Vec<u32>, Refused> {
    resolve::path_indices(path)
}

/// Fingerprint match used after path walk (unit-tested).
pub fn fingerprint_matches(expected: &NodeRef, name: &str, role: &str) -> bool {
    resolve::fingerprint_matches(expected, name, role)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_coach_fabric::{walk_budgeted, MemTree, MintBudget, WalkParams};
    use dpl_coach_inventory::{Complexity, DesktopId};

    fn sample_fabric() -> Fabric {
        let mut t = MemTree::new("app", "application");
        let frame = t.add_child(t.root(), "Win", "frame");
        let bar = t.add_child(frame, "Bar", "panel");
        let _seed = t.add_child(bar, "Search Ctrl+K", "button");
        // Give seed a click action via MemTree helper — MemTree stores empty
        // actions by default; walk still records name/role/path.
        let outcome = walk_budgeted(
            &t,
            t.root(),
            WalkParams {
                target_id: DesktopId::from_raw("cursor.desktop"),
                atspi_name: "cursor".into(),
                complexity: Complexity::Medium,
                budget: MintBudget::for_complexity(Complexity::Medium),
                seed_name: Some("Search Ctrl+K"),
                cancel: None,
            },
            |_| {},
        );
        match outcome {
            dpl_coach_fabric::MintOutcome::Ready(f) => f,
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn path_indices_skip_root_segment() {
        assert_eq!(path_indices("0").unwrap(), Vec::<u32>::new());
        assert_eq!(path_indices("0/1/2").unwrap(), vec![1, 2]);
    }

    #[test]
    fn path_indices_refuse_garbage() {
        assert!(path_indices("").is_err());
        assert!(path_indices("a/b").is_err());
    }

    #[test]
    fn fingerprint_requires_name() {
        let n = NodeRef {
            path: "0/0".into(),
            role: "button".into(),
            name: "Search Ctrl+K".into(),
            actions: vec!["click".into()],
        };
        assert!(fingerprint_matches(&n, "Search Ctrl+K", "button"));
        assert!(fingerprint_matches(&n, "Search Ctrl+K", "push button"));
        assert!(!fingerprint_matches(&n, "Cursor Agents", "frame"));
    }

    #[test]
    fn session_seed_from_minted_fabric() {
        let fabric = sample_fabric();
        let session = FabricSession::new(fabric, AgencyMode::Coach);
        let seed = session.seed_node().expect("seed");
        assert_eq!(seed.name, "Search Ctrl+K");
        assert!(seed.path.starts_with('0'));
    }

    #[test]
    fn teacher_and_coach_are_distinct_modes_same_session_shape() {
        let fabric = sample_fabric();
        let coach = FabricSession::new(fabric.clone(), AgencyMode::Coach);
        let teacher = FabricSession::new(fabric, AgencyMode::Teacher);
        assert_eq!(coach.agency.as_str(), "coach");
        assert_eq!(teacher.agency.as_str(), "teacher");
        assert_eq!(coach.seed_node(), teacher.seed_node());
    }
}
