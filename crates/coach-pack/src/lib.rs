//! The Coach seam's pack registry + evidence resolver (S3a): scans
//! produce fingerprint bundles, an evidence-weighted resolver aliases
//! observation onto identity, and schema-1 TOML packs live in a
//! root-agnostic directory registry. The identity spine (`AppId`,
//! `CellKey`, `PackId`) and the transport rung were REHOMED into
//! `egui_pilot_ai_protocol::seam` by S2 — import them from the protocol
//! crate (no re-export shims on moves). Deps: the protocol crate plus
//! toml, sha2, dpl-coach-inventory; egui-free; never a dep of `app`
//! directly. ADR: `docs/plans/coach-seam-adr.md` §3, §7.4, §7.7 ·
//! plans: `docs-history-2026-10-12:docs/plans/coach-seam-s3a-identity-spine.md` and
//! `docs-history-2026-10-12:docs/plans/coach-seam-s2-protocol-rehoming.md`.

#![forbid(unsafe_code)]

mod bundle;
mod pack;
mod registry;
mod resolve;

pub use bundle::FingerprintBundle;
pub use pack::{
    effective_rung, Identity, Pack, PackError, ParsedPack, PromotedRung, VerbGrant, CURRENT_SEAM,
    DEFAULT_RUNG, PACK_SCHEMA,
};
pub use registry::PackRegistry;
pub use resolve::{
    resolve_against, score_bundle, synthesize_generic, Resolution, SignalMatch, ACCEPT_SCORE,
    SIGNAL_WEIGHTS,
};
