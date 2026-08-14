//! # dpl-coach-foundry — the Fabric Foundry, v0 (seam ADR §3.5, S3b)
//!
//! The staged, checkpointed pipeline that builds a pack cell for one
//! live foreign app:
//!
//! ```text
//! census → resolve → fabric → probe → induct → battery → publish
//! ```
//!
//! Laws carried in code, not prose:
//!
//! - **Checkpoint = artifact.** Expensive stages leave a TOML beside the
//!   pack (`packs/<app-id>/foundry/`); a killed job resumes by skipping
//!   any stage whose artifact survived. Census/Resolve re-observe live
//!   truth every run.
//! - **Reliability is measured, never asserted.** Every probe and
//!   battery execution is an S1 seam span into the same Parquet layout
//!   the daemon and the app write; the pack's `[[verb]] reliability` is
//!   the observed effect rate.
//! - **Identity is never guessed.** Evidence resolves against installed
//!   packs; a reverse-DNS desktop id may mint an identity; anything
//!   else stays unresolved and the Foundry stops, honestly.
//! - **Purge moves, never destroys.** `purge` trashes a cell into
//!   `packs/.trash/`; `remake` is purge + run — the multi-round
//!   iteration loop the operator asked for.
//!
//! Egui-free. Consumed by `dpl-hub-coach`; never a dep of `app` directly.
//! Plan: `docs/plans/coach-seam-s3b-foundry-v0.md`.

mod artifacts;
mod job;
mod probe;
mod stage;

pub use artifacts::{
    battery_from_table, probes_from_table, read_table, AnchorNode, Anchors, BatteryRow, ProbeRow,
};
pub use job::{
    elect_seed, foundry_dir, induct_pack, is_checkpointed, mint_app_id_from_evidence, purge,
    remake, run, FoundryOutcome, FoundryRequest, JobState, StageReport, StageStatus,
};
pub use probe::{probe_verbs, SpanSink};
pub use stage::FoundryStage;
