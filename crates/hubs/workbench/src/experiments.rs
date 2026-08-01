//! Workbench › Experiments — each experiment its own domain.
//!
//! An experiment is an authored document, not generated: it is a human
//! intention, so it says what it hopes and what it needs. The runs beneath it
//! are read from the SAME summaries the Test Lab reads (`scan_runs`), grouped
//! by domain and arm. Two presentations of one scan — a second analyzer is
//! what would make experiments stop benefiting from the lab.
//!
//! Read-only over evidence. Nothing here runs, rewrites, or prunes a bundle.

use dpl_coldlight as cl;
use egui::RichText;
use egui_pilot_ai::protocol::Role;
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use egui_pilot_ai_lab::RunSummary;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The experiment domain list — one tracked node per experiment, and one per
/// arm, so a drill can assert the grouping rather than a screenshot of it.
pub const EXPERIMENTS_ID: PilotId = PilotId::new("workbench.experiments");

/// Where authored experiments live, beside the seeds they resemble.
const EXPERIMENTS_ROOT: &str = "experiments";

/// The runs root. `EguiPilotConfig.artifact_root` is the authority and
/// defaults to this; the surface reads the default rather than re-deriving
/// the config, and says so when the directory is simply absent.
const RUNS_ROOT: &str = "target/eguipilotai/runs";

#[derive(Debug, Clone, Deserialize)]
pub struct Variant {
    pub id: String,
    /// The scenario that IS this arm. The scenario declares the same pair
    /// back, which is what puts the dimension in the run manifest.
    pub scenario: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub hypothesis: String,
    /// The MINIMAL REQUIREMENTS. A bundle missing any of these is reported,
    /// never silently counted — capture is a contract, not a hope.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub variants: Vec<Variant>,
}

impl Experiment {
    /// Whether one bundle satisfies this experiment's stated requirements.
    fn satisfied_by(&self, run: &RunSummary) -> bool {
        self.requires
            .iter()
            .all(|requirement| match requirement.as_str() {
                "manifest" => run.has_manifest,
                "trace" => run.has_trace,
                "captures" => run.capture_count > 0,
                // An unknown requirement is never quietly treated as met.
                _ => false,
            })
    }

    fn unknown_requirements(&self) -> Vec<&str> {
        self.requires
            .iter()
            .map(String::as_str)
            .filter(|requirement| !matches!(*requirement, "manifest" | "trace" | "captures"))
            .collect()
    }
}

#[derive(Default)]
pub struct Experiments {
    domains: Vec<Experiment>,
    runs: Vec<RunSummary>,
    /// Honest state, not an empty list pretending to be an answer.
    notice: Option<String>,
    runs_notice: Option<String>,
    loaded: bool,
}

/// Read every `experiments/<id>/experiment.json`.
fn discover(root: &Path) -> Result<Vec<Experiment>, String> {
    let entries = std::fs::read_dir(root).map_err(|error| error.to_string())?;
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let document = entry.path().join("experiment.json");
        if !document.is_file() {
            continue;
        }
        let text = match std::fs::read_to_string(&document) {
            Ok(text) => text,
            Err(error) => return Err(format!("{}: {error}", document.display())),
        };
        match serde_json::from_str::<Experiment>(&text) {
            Ok(experiment) => found.push(experiment),
            // A malformed experiment is named, never skipped in silence.
            Err(error) => return Err(format!("{}: {error}", document.display())),
        }
    }
    found.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(found)
}

impl Experiments {
    fn load(&mut self) {
        self.loaded = true;
        match discover(Path::new(EXPERIMENTS_ROOT)) {
            Ok(domains) => {
                self.notice = None;
                self.domains = domains;
            }
            Err(error) => {
                self.domains.clear();
                self.notice = Some(error);
            }
        }
        match egui_pilot_ai_lab::scan_runs(PathBuf::from(RUNS_ROOT)) {
            Ok(runs) => {
                self.runs_notice = None;
                self.runs = runs;
            }
            Err(error) => {
                self.runs.clear();
                self.runs_notice = Some(format!("{RUNS_ROOT}: {error}"));
            }
        }
    }

    /// The runs claiming this experiment and arm, newest first (`scan_runs`
    /// already sorts, so the filter preserves it).
    fn arm_runs(&self, experiment: &str, variant: &str) -> Vec<&RunSummary> {
        self.runs
            .iter()
            .filter(|run| {
                run.experiment.as_deref() == Some(experiment)
                    && run.variant.as_deref() == Some(variant)
            })
            .collect()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if !self.loaded {
            self.load();
        }
        ui.horizontal(|ui| {
            ui.heading("Experiments");
            if ui.button("Rescan").clicked() {
                self.load();
            }
        });
        ui.label(
            RichText::new(
                "Each experiment is its own domain. Its arms' runs are grouped from the \
                 evidence already on disk — declaring the requirements is what makes the \
                 capture automatic.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S3);

        if let Some(notice) = &self.notice {
            ui.label(RichText::new(notice).small().color(cl::Tone::Coral.color()));
        }
        if let Some(notice) = &self.runs_notice {
            ui.label(RichText::new(notice).small().color(cl::Tone::Amber.color()));
        }
        if self.domains.is_empty() && self.notice.is_none() {
            ui.label(
                RichText::new(
                    "No experiments authored yet — add experiments/<id>/experiment.json.",
                )
                .small()
                .color(cl::ink::SUBTLE),
            );
            return;
        }

        let domains = std::mem::take(&mut self.domains);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for experiment in &domains {
                    self.domain(ui, experiment);
                    ui.add_space(cl::space::S3);
                }
            });
        self.domains = domains;
    }

    fn domain(&self, ui: &mut egui::Ui, experiment: &Experiment) {
        // Filled, not a bare group: the backdrop art runs behind every hub
        // surface, and an experiment's evidence has to be readable over it.
        egui::Frame::group(ui.style())
            .fill(cl::surface::RAISED)
            .stroke(egui::Stroke::new(1.0, cl::surface::HAIRLINE))
            .corner_radius(egui::CornerRadius::same(cl::RADIUS_SM))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let title = ui.label(
                    RichText::new(&experiment.label)
                        .strong()
                        .size(15.0)
                        .color(cl::ink::STRONG),
                );
                let mut metadata = NodeMetadata::new(
                    EXPERIMENTS_ID.child(&PilotKey::new(experiment.id.as_str())),
                    Role::Region,
                    experiment.label.clone(),
                );
                metadata.value = Some(experiment.variants.len().to_string());
                ui.pilot_track(&title, metadata);
                if !experiment.hypothesis.is_empty() {
                    ui.label(
                        RichText::new(&experiment.hypothesis)
                            .small()
                            .color(cl::ink::BASE),
                    );
                }
                ui.label(
                    RichText::new(format!(
                        "{}  ·  requires: {}",
                        experiment.id,
                        if experiment.requires.is_empty() {
                            "nothing stated".to_owned()
                        } else {
                            experiment.requires.join(", ")
                        }
                    ))
                    .small()
                    .color(cl::ink::SUBTLE),
                );
                let unknown = experiment.unknown_requirements();
                if !unknown.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "unrecognised requirement: {} — no run can satisfy it",
                            unknown.join(", ")
                        ))
                        .small()
                        .color(cl::Tone::Coral.color()),
                    );
                }
                ui.add_space(cl::space::S2);

                if experiment.variants.is_empty() {
                    ui.label(
                        RichText::new("No arms declared.")
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    return;
                }
                for variant in &experiment.variants {
                    let runs = self.arm_runs(&experiment.id, &variant.id);
                    let met = runs
                        .iter()
                        .filter(|run| experiment.satisfied_by(run))
                        .count();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(&variant.id)
                                .strong()
                                .color(cl::Tone::Cyan.color()),
                        );
                        ui.label(
                            RichText::new(&variant.scenario)
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                        ui.label(
                            RichText::new(if runs.is_empty() {
                                "no runs yet".to_owned()
                            } else if met == runs.len() {
                                format!("{} run(s) · all complete", runs.len())
                            } else {
                                // The gap is the point: a run that did not meet the
                                // stated requirements is not quietly counted.
                                format!("{} run(s) · {met} complete", runs.len())
                            })
                            .small()
                            .color(if runs.is_empty() {
                                cl::ink::SUBTLE
                            } else if met == runs.len() {
                                cl::Tone::Lime.color()
                            } else {
                                cl::Tone::Amber.color()
                            }),
                        );
                    });
                    let arm = ui.label(
                        RichText::new(match runs.first() {
                            Some(newest) => format!(
                                "   newest {} · {} capture(s) · {} B stderr",
                                newest.id, newest.capture_count, newest.stderr_bytes
                            ),
                            None => "   no evidence on disk yet".to_owned(),
                        })
                        .small()
                        .color(cl::ink::MUTED),
                    );
                    let mut metadata = NodeMetadata::new(
                        EXPERIMENTS_ID
                            .child(&PilotKey::new(experiment.id.as_str()))
                            .child(&PilotKey::new(variant.id.as_str())),
                        Role::Status,
                        variant.scenario.clone(),
                    );
                    metadata.value = Some(format!("{}/{met}", runs.len()));
                    ui.pilot_track(&arm, metadata);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_pilot_ai_lab::RunSummary;

    fn summary(has_manifest: bool, has_trace: bool, captures: usize) -> RunSummary {
        RunSummary {
            id: "run-1".into(),
            path: PathBuf::from("."),
            has_manifest,
            has_trace,
            capture_count: captures,
            stderr_bytes: 0,
            experiment: Some("e".into()),
            variant: Some("v".into()),
        }
    }

    fn tree(name: &str, body: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("devpulse-experiments-{}-{id}", std::process::id()));
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).expect("create experiment tree");
        std::fs::write(dir.join("experiment.json"), body).expect("write experiment");
        root
    }

    #[test]
    fn an_experiment_document_round_trips() {
        let root = tree(
            "alpha",
            r#"{"v":1,"id":"alpha","label":"A","hypothesis":"H",
                "requires":["manifest"],
                "variants":[{"id":"rows","scenario":"s"}]}"#,
        );
        let found = discover(&root).expect("discover");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "alpha");
        assert_eq!(found[0].variants[0].scenario, "s");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_malformed_experiment_is_named_not_skipped() {
        let root = tree("broken", "{ not json");
        let error = discover(&root).expect_err("malformed must not pass silently");
        assert!(error.contains("experiment.json"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_bundle_missing_a_required_artifact_is_not_counted() {
        let experiment = Experiment {
            id: "e".into(),
            label: "E".into(),
            hypothesis: String::new(),
            requires: vec!["manifest".into(), "trace".into()],
            variants: Vec::new(),
        };
        assert!(experiment.satisfied_by(&summary(true, true, 0)));
        // A bundle that never wrote its trace is evidence of nothing.
        assert!(!experiment.satisfied_by(&summary(true, false, 0)));
    }

    #[test]
    fn an_unknown_requirement_can_never_be_satisfied() {
        let experiment = Experiment {
            id: "e".into(),
            label: "E".into(),
            hypothesis: String::new(),
            requires: vec!["telemetry".into()],
            variants: Vec::new(),
        };
        assert!(!experiment.satisfied_by(&summary(true, true, 9)));
        assert_eq!(experiment.unknown_requirements(), vec!["telemetry"]);
    }

    #[test]
    fn runs_group_by_experiment_then_variant() {
        let surface = Experiments {
            runs: vec![
                RunSummary {
                    id: "run-b".into(),
                    experiment: Some("e".into()),
                    variant: Some("other".into()),
                    ..summary(true, true, 0)
                },
                RunSummary {
                    id: "run-a".into(),
                    ..summary(true, true, 0)
                },
            ],
            ..Experiments::default()
        };
        let mine = surface.arm_runs("e", "v");
        assert_eq!(mine.len(), 1, "an arm never claims another arm's evidence");
        assert_eq!(mine[0].id, "run-a");
        assert!(surface.arm_runs("e", "missing").is_empty());
    }

    #[test]
    fn a_run_without_an_experiment_is_never_claimed() {
        let surface = Experiments {
            runs: vec![RunSummary {
                experiment: None,
                variant: None,
                ..summary(true, true, 0)
            }],
            ..Experiments::default()
        };
        assert!(surface.arm_runs("e", "v").is_empty());
    }
}
