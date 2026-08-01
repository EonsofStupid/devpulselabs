//! Passive native previews for semantic inspection, run bundles, and traces.
//! Live target control, replay, and visual approval are intentionally marked
//! unsupported until their delivery slices land.

use egui_pilot_ai_protocol::{
    project_snapshot, AgentSnapshot, SemanticFrame, SessionStatus, SnapshotRequest,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    AgentTree,
    FullTree,
    Runs,
    Trace,
    Visual,
}

#[derive(Debug, Clone)]
pub struct RunSummary {
    pub id: String,
    pub path: PathBuf,
    pub has_manifest: bool,
    pub has_trace: bool,
    pub capture_count: usize,
    pub stderr_bytes: u64,
    /// The experiment dimension, read from the bundle's own manifest. `None`
    /// for an ordinary run — which is every run recorded before the field
    /// existed, and every run that is simply not part of an experiment.
    pub experiment: Option<String>,
    pub variant: Option<String>,
}

/// Read the experiment dimension out of a bundle's manifest.
///
/// A malformed or absent manifest yields `None` rather than an error: the
/// analyzer's job is to summarise whatever evidence exists, and a bundle
/// that cannot say what it belongs to is still a bundle.
fn manifest_experiment(path: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = fs::read_to_string(path.join("manifest.json")) else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (None, None);
    };
    let field = |key: &str| {
        value
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    };
    (field("experiment"), field("variant"))
}

#[derive(Default)]
pub struct TestLab {
    tab: Tab,
    full_frame: Option<SemanticFrame>,
    agent_snapshot: Option<AgentSnapshot>,
    status: Option<SessionStatus>,
    runs: Vec<RunSummary>,
    selected_run: Option<usize>,
    search: String,
    trace_preview: String,
    notice: Option<String>,
}

impl TestLab {
    pub fn set_frame(&mut self, frame: SemanticFrame) {
        self.agent_snapshot = project_snapshot(&frame, &SnapshotRequest::default()).ok();
        self.full_frame = Some(frame);
    }

    pub fn set_snapshot(&mut self, snapshot: AgentSnapshot) {
        self.agent_snapshot = Some(snapshot);
    }

    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = Some(status);
    }

    pub fn scan_runs(&mut self, root: impl AsRef<Path>) {
        let root = root.as_ref();
        match scan_runs(root) {
            Ok(runs) => {
                self.runs = runs;
                self.notice = Some(format!("{} run bundles discovered", self.runs.len()));
            }
            Err(error) => {
                self.runs.clear();
                self.notice = Some(format!("{}: {error}", root.display()));
            }
        }
    }
}

/// Scan a runs root into summaries.
///
/// Free rather than a `TestLab` method so the Workbench Experiments surface
/// reads the SAME summaries from the SAME walk. A second scanner is what
/// would make experiments stop benefiting from the lab.
pub fn scan_runs(root: impl AsRef<Path>) -> std::io::Result<Vec<RunSummary>> {
    let root = root.as_ref();
    let mut runs = Vec::new();
    {
        let entries = fs::read_dir(root)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let captures = path.join("captures");
            let capture_count = fs::read_dir(captures)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|entry| entry.path().is_file())
                        .count()
                })
                .unwrap_or(0);
            let (experiment, variant) = manifest_experiment(&path);
            runs.push(RunSummary {
                id: entry.file_name().to_string_lossy().into_owned(),
                experiment,
                variant,
                has_manifest: path.join("manifest.json").is_file(),
                has_trace: path.join("trace.jsonl").is_file(),
                stderr_bytes: fs::metadata(path.join("stderr.log"))
                    .map(|metadata| metadata.len())
                    .unwrap_or(0),
                capture_count,
                path,
            });
        }
    }
    runs.sort_by(|left, right| right.id.cmp(&left.id));
    Ok(runs)
}

impl TestLab {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("EguiPilotAI Test Lab");
            ui.label(
                egui::RichText::new("PASSIVE PREVIEW · LIVE CONTROL UNSUPPORTED")
                    .monospace()
                    .small()
                    .color(egui::Color32::from_rgb(130, 205, 255)),
            );
        });
        ui.label("Explore compact agent context, diagnose the full tree, and review immutable run evidence.");
        ui.add_space(8.0);
        self.status_strip(ui);
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            tab_button(ui, &mut self.tab, Tab::AgentTree, "Agent tree");
            tab_button(ui, &mut self.tab, Tab::FullTree, "Full tree");
            tab_button(ui, &mut self.tab, Tab::Runs, "Runs");
            tab_button(ui, &mut self.tab, Tab::Trace, "Trace");
            tab_button(ui, &mut self.tab, Tab::Visual, "Visual");
        });
        ui.separator();
        match self.tab {
            Tab::AgentTree => self.agent_tree(ui),
            Tab::FullTree => self.full_tree(ui),
            Tab::Runs => self.runs(ui),
            Tab::Trace => self.trace(ui),
            Tab::Visual => self.visual(ui),
        }
    }

    fn status_strip(&self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| match &self.status {
                Some(status) => {
                    ui.label(format!("revision {}", status.revision));
                    ui.separator();
                    ui.label(format!("{} viewports", status.viewport_count));
                    ui.separator();
                    ui.label(format!("{} pending", status.pending_actions));
                    ui.separator();
                    ui.label(format!("{} busy", status.busy_tokens));
                    if let Some(renderer) = &status.renderer_key {
                        ui.separator();
                        ui.monospace(renderer);
                    }
                }
                None => {
                    ui.label("No target connected. Launch a named profile through the runner or MCP server.");
                }
            });
        });
    }

    fn search_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Find node");
            ui.text_edit_singleline(&mut self.search);
        });
    }

    fn agent_tree(&mut self, ui: &mut egui::Ui) {
        self.search_box(ui);
        let Some(snapshot) = &self.agent_snapshot else {
            empty(ui, "No agent snapshot received yet.");
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!("rev {}", snapshot.revision));
            ui.monospace(format!("hash {}", snapshot.content_hash));
            ui.label(format!("{} projected nodes", snapshot.nodes.len()));
            if let Some(reason) = &snapshot.truncated {
                ui.colored_label(egui::Color32::YELLOW, reason);
            }
        });
        let search = self.search.to_ascii_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for node in snapshot.nodes.iter().filter(|node| {
                search.is_empty()
                    || node.id.0.to_ascii_lowercase().contains(&search)
                    || node.name.to_ascii_lowercase().contains(&search)
            }) {
                egui::CollapsingHeader::new(format!("{:?} · {}", node.role, node.name))
                    .id_salt(("agent-node", &node.id.0))
                    .show(ui, |ui| {
                        ui.monospace(&node.id.0);
                        ui.label(format!("actions: {:?}", node.actions));
                        ui.label(format!("state: {:?}", node.state));
                        ui.label(format!("rect: {:?}", node.rect));
                        if !node.regions.is_empty() {
                            ui.label(format!("regions: {}", node.regions.join(" › ")));
                        }
                    });
            }
        });
    }

    fn full_tree(&mut self, ui: &mut egui::Ui) {
        self.search_box(ui);
        let Some(frame) = &self.full_frame else {
            empty(
                ui,
                "Full frames stay local to traces and Test Lab; none is loaded.",
            );
            return;
        };
        ui.label(format!(
            "{} complete nodes · {} busy tokens · {} output events",
            frame.nodes.len(),
            frame.diagnostics.busy_tokens.len(),
            frame.diagnostics.output_events
        ));
        let search = self.search.to_ascii_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for node in frame.nodes.iter().filter(|node| {
                search.is_empty()
                    || node.id.0.to_ascii_lowercase().contains(&search)
                    || node.name.to_ascii_lowercase().contains(&search)
            }) {
                ui.horizontal_wrapped(|ui| {
                    ui.monospace(&node.id.0);
                    ui.label(format!("{:?}", node.role));
                    ui.label(&node.name);
                    if !node.visible() {
                        ui.weak("clipped/hidden");
                    }
                    if node.state.disabled {
                        ui.weak("disabled");
                    }
                });
            }
        });
    }

    fn runs(&mut self, ui: &mut egui::Ui) {
        if ui.button("Refresh target/eguipilotai/runs").clicked() {
            self.scan_runs("target/eguipilotai/runs");
        }
        if let Some(notice) = &self.notice {
            ui.weak(notice);
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, run) in self.runs.iter().enumerate() {
                let selected = self.selected_run == Some(index);
                if ui
                    .selectable_label(
                        selected,
                        format!(
                            "{} · {} captures · {} stderr bytes",
                            run.id, run.capture_count, run.stderr_bytes
                        ),
                    )
                    .clicked()
                {
                    self.selected_run = Some(index);
                }
                if selected {
                    ui.indent(("run", index), |ui| {
                        ui.monospace(run.path.display().to_string());
                        ui.label(format!(
                            "manifest: {} · trace: {}",
                            run.has_manifest, run.has_trace
                        ));
                    });
                }
            }
        });
    }

    fn trace(&mut self, ui: &mut egui::Ui) {
        if ui.button("Load selected trace").clicked() {
            self.trace_preview = self
                .selected_run
                .and_then(|index| self.runs.get(index))
                .and_then(|run| fs::read_to_string(run.path.join("trace.jsonl")).ok())
                .map(|trace| trace.chars().take(64 * 1024).collect())
                .unwrap_or_else(|| "No readable selected trace.".into());
        }
        egui::ScrollArea::both().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.trace_preview)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        });
    }

    fn visual(&mut self, ui: &mut egui::Ui) {
        empty(
            ui,
            "Preview only. Expected/actual/diff comparison, replay, and visual approval are unsupported until slice 5/6.",
        );
        if let Some(run) = self.selected_run.and_then(|index| self.runs.get(index)) {
            ui.monospace(run.path.join("captures").display().to_string());
        }
    }
}

fn tab_button(ui: &mut egui::Ui, current: &mut Tab, tab: Tab, label: &str) {
    if ui.selectable_label(*current == tab, label).clicked() {
        *current = tab;
    }
}

fn empty(ui: &mut egui::Ui, message: &str) {
    ui.add_space(20.0);
    ui.weak(message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lab_starts_detached() {
        let lab = TestLab::default();
        assert!(lab.status.is_none());
        assert!(lab.full_frame.is_none());
    }
}
