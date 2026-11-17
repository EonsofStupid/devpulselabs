//! The provider vocabulary every AI crate speaks: identities, capabilities,
//! presence, activation, approval, sessions, turns, usage, events, history
//! decoding, the runtime adapter contract, the provider registration, and the
//! AI catalog.
//!
//! Pure data. This crate knows nothing about egui, chat, provider processes,
//! filesystem discovery or transport protocols. Its dependencies are `serde`
//! and `serde_json`, which parses the compiled-in catalog (architecture.md §2).

pub mod activation;
pub mod adapter;
pub mod approval;
pub mod capability;
pub mod catalog;
pub mod event;
pub mod history;
pub mod presence;
pub mod provider;
pub mod registration;
pub mod session;
pub mod turn;
pub mod usage;
