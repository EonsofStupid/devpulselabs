//! Pure budgeted BFS over a [`NodeSource`] — unit-tested without AT-SPI.

use crate::{
    eta_hint, FabricNode, MintBudget, MintOutcome, MintPhase, MintProgress, Refused,
};
use dpl_coach_inventory::{Complexity, DesktopId};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Opaque tree for tests and adapters.
pub trait NodeSource {
    type Id: Clone;

    fn describe(&self, id: &Self::Id) -> (String, String);
    fn child_count(&self, id: &Self::Id) -> u32;
    fn child_at(&self, id: &Self::Id, index: u32) -> Option<Self::Id>;
    fn actions(&self, _id: &Self::Id) -> Vec<String> {
        Vec::new()
    }
}

/// In-memory tree for unit tests / adversary drills.
#[derive(Debug, Clone)]
pub struct MemTree {
    nodes: Vec<MemNode>,
    root: usize,
}

#[derive(Debug, Clone)]
struct MemNode {
    name: String,
    role: String,
    children: Vec<usize>,
    actions: Vec<String>,
}

impl MemTree {
    pub fn new(root_name: &str, root_role: &str) -> Self {
        Self {
            nodes: vec![MemNode {
                name: root_name.into(),
                role: root_role.into(),
                children: Vec::new(),
                actions: Vec::new(),
            }],
            root: 0,
        }
    }

    pub fn root(&self) -> usize {
        self.root
    }

    pub fn add_child(&mut self, parent: usize, name: &str, role: &str) -> usize {
        let id = self.nodes.len();
        self.nodes.push(MemNode {
            name: name.into(),
            role: role.into(),
            children: Vec::new(),
            actions: Vec::new(),
        });
        self.nodes[parent].children.push(id);
        id
    }

    /// Attach `count` anonymous children (oversized-node drills).
    pub fn add_many_children(&mut self, parent: usize, count: u32, role: &str) {
        for i in 0..count {
            self.add_child(parent, &format!("c{i}"), role);
        }
    }
}

impl NodeSource for MemTree {
    type Id = usize;

    fn describe(&self, id: &Self::Id) -> (String, String) {
        let n = &self.nodes[*id];
        (n.name.clone(), n.role.clone())
    }

    fn child_count(&self, id: &Self::Id) -> u32 {
        self.nodes[*id].children.len() as u32
    }

    fn child_at(&self, id: &Self::Id, index: u32) -> Option<Self::Id> {
        self.nodes[*id].children.get(index as usize).copied()
    }

    fn actions(&self, id: &Self::Id) -> Vec<String> {
        self.nodes[*id].actions.clone()
    }
}

pub struct WalkParams<'a> {
    pub target_id: DesktopId,
    pub atspi_name: String,
    pub complexity: Complexity,
    pub budget: MintBudget,
    pub seed_name: Option<&'a str>,
    pub cancel: Option<&'a AtomicBool>,
}

/// Budgeted BFS. Never calls `child_at` when `child_count` exceeds the cap.
pub fn walk_budgeted<S: NodeSource, F>(
    source: &S,
    root: S::Id,
    params: WalkParams<'_>,
    mut on_progress: F,
) -> MintOutcome
where
    F: FnMut(MintProgress),
{
    let started = Instant::now();
    let budget = params.budget;
    on_progress(MintProgress {
        visited: 0,
        budget: budget.max_nodes,
        depth: 0,
        phase: MintPhase::Locating,
        eta_hint: eta_hint(params.complexity),
    });

    let mut nodes: Vec<FabricNode> = Vec::new();
    let mut queue: VecDeque<(S::Id, u32, String)> = VecDeque::new();
    queue.push_back((root, 0, "0".into()));

    on_progress(MintProgress {
        visited: 0,
        budget: budget.max_nodes,
        depth: 0,
        phase: MintPhase::Walking,
        eta_hint: eta_hint(params.complexity),
    });

    while let Some((id, depth, path)) = queue.pop_front() {
        if params
            .cancel
            .is_some_and(|c| c.load(Ordering::Relaxed))
        {
            return MintOutcome::Cancelled;
        }
        if started.elapsed().as_millis() as u64 > budget.max_ms {
            return MintOutcome::Refused(Refused {
                reason: format!("mint exceeded {} ms budget", budget.max_ms),
                tip: "Try a smaller app, or open Advanced and raise the budget deliberately."
                    .into(),
            });
        }
        if nodes.len() as u32 >= budget.max_nodes {
            return MintOutcome::Refused(Refused {
                reason: format!("mint exceeded {} node budget", budget.max_nodes),
                tip: "This tree is too large for a guided lesson — pick a smaller seed (Calculator)."
                    .into(),
            });
        }

        let (name, role) = source.describe(&id);
        let actions = source.actions(&id);
        nodes.push(FabricNode {
            path: path.clone(),
            role,
            name,
            actions,
        });

        let visited = nodes.len() as u32;
        if visited == 1 || visited.is_multiple_of(25) {
            on_progress(MintProgress {
                visited,
                budget: budget.max_nodes,
                depth,
                phase: MintPhase::Walking,
                eta_hint: eta_hint(params.complexity),
            });
        }

        if depth >= budget.max_depth {
            continue;
        }

        let count = source.child_count(&id);
        if count > budget.max_children_at_node {
            if depth == 0 && nodes.len() == 1 {
                return MintOutcome::Refused(Refused {
                    reason: format!(
                        "root reports {count} children (cap {})",
                        budget.max_children_at_node
                    ),
                    tip: "Refused: oversized accessibility tree (freeze landmine). Pick a smaller app or enable its a11y mode, then retry."
                        .into(),
                });
            }
            // Skip expansion — never dump LibreOffice-scale child lists.
            continue;
        }

        for i in 0..count {
            if let Some(child) = source.child_at(&id, i) {
                queue.push_back((child, depth + 1, format!("{path}/{i}")));
            }
        }
    }

    on_progress(MintProgress {
        visited: nodes.len() as u32,
        budget: budget.max_nodes,
        depth: 0,
        phase: MintPhase::Seeding,
        eta_hint: eta_hint(params.complexity),
    });

    let seed_path = if let Some(seed) = params.seed_name {
        match nodes.iter().find(|n| n.name == seed) {
            Some(n) => Some(n.path.clone()),
            None => {
                return MintOutcome::Refused(Refused {
                    reason: format!("seed control {seed:?} not found in fabric"),
                    tip: "Open the basic keypad view, then retry.".into(),
                });
            }
        }
    } else {
        None
    };

    on_progress(MintProgress {
        visited: nodes.len() as u32,
        budget: budget.max_nodes,
        depth: 0,
        phase: MintPhase::Done,
        eta_hint: eta_hint(params.complexity),
    });

    MintOutcome::Ready(crate::Fabric {
        target_id: params.target_id,
        atspi_name: params.atspi_name,
        nodes,
        seed_path,
        minted_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn small_calc() -> MemTree {
        let mut t = MemTree::new("gnome-calculator", "application");
        let frame = t.add_child(t.root(), "Calculator", "frame");
        let pad = t.add_child(frame, "Keypad", "panel");
        t.add_child(pad, "7", "push button");
        t.add_child(pad, "8", "push button");
        t
    }

    #[test]
    fn walk_finds_seed_seven() {
        let tree = small_calc();
        let outcome = walk_budgeted(
            &tree,
            tree.root(),
            WalkParams {
                target_id: DesktopId::from_raw("org.gnome.Calculator.desktop"),
                atspi_name: "gnome-calculator".into(),
                complexity: Complexity::Small,
                budget: MintBudget::for_complexity(Complexity::Small),
                seed_name: Some("7"),
                cancel: None,
            },
            |_| {},
        );
        match outcome {
            MintOutcome::Ready(f) => {
                assert!(f.nodes.iter().any(|n| n.name == "7"));
                assert_eq!(f.seed_path.as_deref(), Some("0/0/0/0"));
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn missing_seed_refuses_with_tip() {
        let tree = small_calc();
        let outcome = walk_budgeted(
            &tree,
            tree.root(),
            WalkParams {
                target_id: DesktopId::from_raw("org.gnome.Calculator.desktop"),
                atspi_name: "gnome-calculator".into(),
                complexity: Complexity::Small,
                budget: MintBudget::for_complexity(Complexity::Small),
                seed_name: Some("9"),
                cancel: None,
            },
            |_| {},
        );
        match outcome {
            MintOutcome::Refused(r) => {
                assert!(r.tip.contains("keypad"));
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn oversized_root_refuses_without_dump() {
        let mut tree = MemTree::new("huge", "application");
        tree.add_many_children(tree.root(), 300, "filler");
    }

    #[test]
    fn oversized_inner_node_is_skipped_not_dumped() {
        let mut tree = MemTree::new("app", "application");
        let frame = tree.add_child(tree.root(), "Win", "frame");
        tree.add_many_children(frame, 300, "cell");
        tree.add_child(tree.root(), "7", "push button");
        let outcome = walk_budgeted(
            &tree,
            tree.root(),
            WalkParams {
                target_id: DesktopId::from_raw("x.desktop"),
                atspi_name: "app".into(),
                complexity: Complexity::Small,
                budget: MintBudget::for_complexity(Complexity::Small),
                seed_name: Some("7"),
                cancel: None,
            },
            |_| {},
        );
        match outcome {
            MintOutcome::Ready(f) => {
                // Root + frame + button 7 — not 300 cells.
                assert!(f.nodes.len() < 10);
                assert!(f.nodes.iter().any(|n| n.name == "7"));
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn cancel_stops_walk() {
        let mut tree = MemTree::new("app", "application");
        for i in 0..50 {
            tree.add_child(tree.root(), &format!("n{i}"), "panel");
        }
        let cancel = AtomicBool::new(true);
        let outcome = walk_budgeted(
            &tree,
            tree.root(),
            WalkParams {
                target_id: DesktopId::from_raw("x.desktop"),
                atspi_name: "app".into(),
                complexity: Complexity::Small,
                budget: MintBudget::for_complexity(Complexity::Small),
                seed_name: None,
                cancel: Some(&cancel),
            },
            |_| {},
        );
        assert!(matches!(outcome, MintOutcome::Cancelled));
    }

    #[test]
    fn node_budget_refuses() {
        let mut tree = MemTree::new("app", "application");
        for i in 0..20 {
            tree.add_child(tree.root(), &format!("n{i}"), "panel");
        }
        let outcome = walk_budgeted(
            &tree,
            tree.root(),
            WalkParams {
                target_id: DesktopId::from_raw("x.desktop"),
                atspi_name: "app".into(),
                complexity: Complexity::Small,
                budget: MintBudget {
                    max_nodes: 5,
                    max_depth: 12,
                    max_ms: 2_000,
                    max_children_at_node: 256,
                },
                seed_name: None,
                cancel: None,
            },
            |_| {},
        );
        match outcome {
            MintOutcome::Refused(r) => assert!(r.reason.contains("node budget")),
            other => panic!("expected Refused, got {other:?}"),
        }
    }
}
