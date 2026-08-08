//! The Coach seam's durable identity spine + pack registry (S3a):
//! `AppId` is the minted-once source of truth, scans produce fingerprint
//! bundles, an evidence-weighted resolver aliases observation onto
//! identity, and schema-1 TOML packs live in a root-agnostic directory
//! registry. Deps: toml + sha2 + dpl-coach-inventory; egui-free; never
//! a dep of `app` directly. ADR: `docs/plans/coach-seam-adr.md` §3,
//! §7.4, §7.7 · plan: `docs/plans/coach-seam-s3a-identity-spine.md`.

#![forbid(unsafe_code)]

mod bundle;
mod id;
mod pack;
mod registry;
mod resolve;

pub use bundle::FingerprintBundle;
pub use id::{AppId, CellKey, IdError, PackId, UNRESOLVED_PREFIX};
pub use pack::{
    effective_rung, Identity, Pack, PackError, ParsedPack, PromotedRung, Rung, VerbGrant,
    CURRENT_SEAM, DEFAULT_RUNG, PACK_SCHEMA,
};
pub use registry::PackRegistry;
pub use resolve::{
    resolve_against, score_bundle, synthesize_generic, Resolution, SignalMatch, ACCEPT_SCORE,
    SIGNAL_WEIGHTS,
};
