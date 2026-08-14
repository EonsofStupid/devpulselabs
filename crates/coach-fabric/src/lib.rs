//! Budgeted Tier A semantic-fabric mint for Operator Coach (C2e).
//!
//! Deep-walks **one** selected AT-SPI app under hard caps. Inventory
//! (`dpl-coach-inventory`) stays desktop-root-only — this crate owns Phase 2.
//! Plan: `docs/plans/coach-c2e-fabric-mint.md`.

#![forbid(unsafe_code)]

mod walk;

#[cfg(target_os = "linux")]
mod linux_mint;

pub use walk::{walk_budgeted, MemTree, NodeSource, WalkParams};

use dpl_coach_inventory::{Complexity, DesktopId};
use std::sync::atomic::AtomicBool;

/// Hard caps for a mint walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MintBudget {
    pub max_nodes: u32,
    pub max_depth: u32,
    pub max_ms: u64,
    pub max_children_at_node: u32,
}

impl MintBudget {
    pub const fn for_complexity(c: Complexity) -> Self {
        match c {
            Complexity::Small => Self {
                max_nodes: 400,
                max_depth: 12,
                max_ms: 2_000,
                max_children_at_node: 256,
            },
            Complexity::Medium => Self {
                max_nodes: 1500,
                max_depth: 16,
                max_ms: 8_000,
                max_children_at_node: 256,
            },
            Complexity::Deep | Complexity::Unknown => Self {
                max_nodes: 4000,
                max_depth: 20,
                max_ms: 20_000,
                max_children_at_node: 256,
            },
        }
    }
}

/// Layman ETA copy keyed off inventory complexity pills.
pub fn eta_hint(c: Complexity) -> String {
    match c {
        Complexity::Small => "About a second — Calculator is a small signal map.".into(),
        Complexity::Medium => "A few seconds — walking menus.".into(),
        Complexity::Deep => "May take a while — or refuse if the tree is too large.".into(),
        Complexity::Unknown => "We'll try with a tight budget and tip if we stop.".into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MintPhase {
    Locating,
    Walking,
    Seeding,
    Done,
}

impl MintPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Locating => "locating",
            Self::Walking => "walking",
            Self::Seeding => "seeding",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintProgress {
    pub visited: u32,
    pub budget: u32,
    pub depth: u32,
    pub phase: MintPhase,
    pub eta_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricNode {
    pub path: String,
    pub role: String,
    pub name: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fabric {
    pub target_id: DesktopId,
    pub atspi_name: String,
    pub nodes: Vec<FabricNode>,
    pub seed_path: Option<String>,
    pub minted_ms: u64,
}

impl Fabric {
    /// First named, actionable node in walk order — the seam ADR §6.1
    /// fallback seed when a pack names no `[teaching] seed_control`.
    pub fn first_actionable(&self) -> Option<&FabricNode> {
        self.nodes
            .iter()
            .find(|n| !n.actions.is_empty() && !n.name.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub reason: String,
    pub tip: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MintOutcome {
    Ready(Fabric),
    Refused(Refused),
    Cancelled,
}

/// What to mint. Prefer resolving via inventory, then pass live AT-SPI name.
#[derive(Debug, Clone)]
pub struct MintRequest {
    pub target_id: DesktopId,
    pub atspi_name: String,
    pub complexity: Complexity,
    pub seed_name: Option<String>,
    pub budget: Option<MintBudget>,
}

/// Mint fabric for one target. Linux uses AT-SPI; other platforms refuse.
pub fn mint(
    request: &MintRequest,
    cancel: Option<&AtomicBool>,
    on_progress: impl FnMut(MintProgress),
) -> MintOutcome {
    let budget = request
        .budget
        .unwrap_or_else(|| MintBudget::for_complexity(request.complexity));
    #[cfg(target_os = "linux")]
    {
        linux_mint::mint_blocking(
            &request.atspi_name,
            request.target_id.clone(),
            request.complexity,
            budget,
            request.seed_name.as_deref(),
            cancel,
            on_progress,
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (budget, cancel, on_progress);
        MintOutcome::Refused(Refused {
            reason: "fabric mint is Linux-first in C2e".into(),
            tip: "Use the Linux seat for Operator Coach mint, or wait for the Windows UIA port."
                .into(),
        })
    }
}

/// Find one inventory row by id / display name / live AT-SPI name
/// (exact first, substring after) — the one selector the Coach picker,
/// the daemon, and the Foundry all share.
pub fn find_inventory_item(
    name_or_id: &str,
) -> Result<dpl_coach_inventory::InventoryItem, Refused> {
    let items = dpl_coach_inventory::inventory(true).map_err(|e| Refused {
        reason: format!("inventory failed: {e}"),
        tip: "Refresh the app list, then try again.".into(),
    })?;
    let needle = name_or_id.to_lowercase();
    let item = items.iter().find(|i| {
        i.id.as_str().eq_ignore_ascii_case(name_or_id)
            || i.display_name.eq_ignore_ascii_case(name_or_id)
            || i.live
                .as_ref()
                .is_some_and(|l| l.atspi_name.eq_ignore_ascii_case(name_or_id))
            || i.display_name.to_lowercase().contains(&needle)
            || i.id.as_str().to_lowercase().contains(&needle)
    });
    item.cloned().ok_or_else(|| Refused {
        reason: format!("no inventory row matching {name_or_id:?}"),
        tip: "Pick an installed app from the Coach list (try Calculator).".into(),
    })
}

/// Resolve an inventory row into a [`MintRequest`]. The seed comes from
/// the CALLER (a pack's `[teaching] seed_control`, an explicit request
/// field) — S3b killed the name-sniffed seed table that lived here; a
/// seedless mint falls back to the first actionable node at resolve
/// time, never to a hardcoded string.
pub fn request_from_inventory_name(
    name_or_id: &str,
    seed_name: Option<String>,
) -> Result<MintRequest, Refused> {
    let item = find_inventory_item(name_or_id)?;
    let Some(live) = item.live.as_ref() else {
        return Err(Refused {
            reason: format!("{} is not running", item.display_name),
            tip: format!("Open {}, then Build again.", item.display_name),
        });
    };
    Ok(MintRequest {
        target_id: item.id.clone(),
        atspi_name: live.atspi_name.clone(),
        complexity: item.complexity,
        seed_name,
        budget: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_budget_matches_plan() {
        let b = MintBudget::for_complexity(Complexity::Small);
        assert_eq!(b.max_nodes, 400);
        assert_eq!(b.max_depth, 12);
        assert_eq!(b.max_ms, 2_000);
        assert_eq!(b.max_children_at_node, 256);
    }

    #[test]
    fn eta_copy_mentions_calculator_for_small() {
        assert!(eta_hint(Complexity::Small).contains("second"));
    }
}
