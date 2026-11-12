//! How a provider decodes its own stored history, one line at a time.

use crate::event::ChatEventContent;
use crate::provider::ProviderId;
use crate::session::ProviderSessionId;
use crate::turn::TurnId;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryIndexMetadata {
    pub native_id: ProviderSessionId,
    pub title: String,
    pub workspace: String,
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHistoryEvent {
    pub turn_id: Option<TurnId>,
    pub content: ChatEventContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeOutcome {
    Event(DecodedHistoryEvent),
    IgnoreKnown,
}

/// `Send` is a genuine requirement, not a formality: the HOST owns the
/// catalog (H1 §3.2) and the host's shared state must be `Send`. A decoder is
/// a pure function over lines and a source is a path plus a decoder, so
/// nothing here is legitimately thread-bound.
pub trait HistoryDecoder: Send {
    fn provider_id(&self) -> ProviderId;
    fn index_metadata(&self, path: &Path, prefix: &str) -> Option<HistoryIndexMetadata>;
    fn decode_line(&self, line: &str) -> DecodeOutcome;
}
