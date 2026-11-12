//! The provider vocabulary every AI crate speaks: identities, capabilities,
//! presence, activation, approval, sessions, turns, usage, events, history
//! decoding, the runtime adapter contract and the provider registration.
//!
//! Pure data. This crate knows nothing about egui, chat, provider processes,
//! filesystem discovery or transport protocols. Its only dependency is
//! `serde` (architecture.md §2).

pub mod activation;
pub mod adapter;
pub mod approval;
pub mod capability;
pub mod event;
pub mod history;
pub mod presence;
pub mod provider;
pub mod registration;
pub mod session;
pub mod turn;
pub mod usage;
