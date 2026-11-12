//! Token accounting, and each provider's usage dialect.

use crate::provider::ProviderId;
use crate::session::ProviderSessionId;

/// Structured token accounting from a provider's usage report.
///
/// Every field is optional because providers report DIFFERENT things:
/// Codex reports an input/output split, Grok reports context-window
/// occupancy. A `None` means "this provider did not say", never zero —
/// the distinction matters, because zero is a measurement and absent is
/// not, and cost computed from a guessed zero would be a lie.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    /// FRESH input — tokens the provider actually processed anew.
    pub input_tokens: Option<u64>,
    /// Tokens served from cache. Billed at a small fraction of fresh
    /// input, so they MUST stay separate: folding them into `input`
    /// makes cost an overestimate, and the split cannot be recovered
    /// afterwards from a folded number.
    pub cache_read_tokens: Option<u64>,
    /// Tokens written INTO the cache — billed above fresh input.
    pub cache_write_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    /// Context-window occupancy, when the provider reports it that way.
    pub context_used: Option<u64>,
    pub context_size: Option<u64>,
}

impl TokenUsage {
    pub const fn is_empty(self) -> bool {
        self.input_tokens.is_none()
            && self.cache_read_tokens.is_none()
            && self.cache_write_tokens.is_none()
            && self.output_tokens.is_none()
            && self.total_tokens.is_none()
            && self.context_used.is_none()
            && self.context_size.is_none()
    }

    /// Accumulate per-turn reports into a session total. Absent stays
    /// absent; a present value adds to whatever came before, so summing
    /// never invents a number out of two absences.
    pub fn accumulate(self, turn: Self) -> Self {
        fn plus(a: Option<u64>, b: Option<u64>) -> Option<u64> {
            match (a, b) {
                (Some(a), Some(b)) => Some(a.saturating_add(b)),
                (Some(v), None) | (None, Some(v)) => Some(v),
                (None, None) => None,
            }
        }
        Self {
            input_tokens: plus(self.input_tokens, turn.input_tokens),
            cache_read_tokens: plus(self.cache_read_tokens, turn.cache_read_tokens),
            cache_write_tokens: plus(self.cache_write_tokens, turn.cache_write_tokens),
            output_tokens: plus(self.output_tokens, turn.output_tokens),
            total_tokens: plus(self.total_tokens, turn.total_tokens),
            // Context occupancy is a LEVEL, never a sum — the latest
            // reading is the truth, adding them would be meaningless.
            context_used: turn.context_used.or(self.context_used),
            context_size: turn.context_size.or(self.context_size),
        }
    }

    /// Later reports supersede earlier ones field-by-field, so a provider
    /// that reports input now and output later still yields one whole
    /// picture at the end of the turn.
    pub fn merge(self, newer: Self) -> Self {
        Self {
            input_tokens: newer.input_tokens.or(self.input_tokens),
            cache_read_tokens: newer.cache_read_tokens.or(self.cache_read_tokens),
            cache_write_tokens: newer.cache_write_tokens.or(self.cache_write_tokens),
            output_tokens: newer.output_tokens.or(self.output_tokens),
            total_tokens: newer.total_tokens.or(self.total_tokens),
            context_used: newer.context_used.or(self.context_used),
            context_size: newer.context_size.or(self.context_size),
        }
    }
}

/// What a provider is ABLE to report about token usage.
///
/// Declared, never discovered. This is the honesty mechanism: a surface
/// can say "Grok does not report an input/output split" instead of
/// rendering a silent zero, and a half-implemented provider stays legible
/// instead of looking broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageReporting {
    /// Separate prompt and completion counts.
    pub input_output_split: bool,
    /// A single combined total.
    pub total_tokens: bool,
    /// Context-window occupancy (used / size).
    pub context_window: bool,
    /// The provider computes real money itself — trusted over our table.
    pub provider_priced: bool,
    /// The provider names the model it is running.
    pub model: bool,
    /// Counts are CUMULATIVE — each report restates the running total for
    /// the session (Codex's `total_token_usage`), so the last report is
    /// the whole picture and reports must NOT be summed. When false, each
    /// report covers one turn (Claude's per-message `usage`) and a
    /// session total is the SUM. Getting this backwards silently
    /// under- or over-counts real spend.
    pub counts_are_cumulative: bool,
    /// Usage can be recovered from ALREADY-STORED sessions, so the
    /// backfill pass can walk this provider's history and recover cost
    /// the operator has already spent. False for live-only transports
    /// (Grok's ACP), where usage exists on the wire and nowhere else.
    pub history_backfill: bool,
}

impl UsageReporting {
    /// A provider that reports nothing yet. A truthful state, not a bug —
    /// it makes "not implemented" legible and turns adding support into
    /// filling in one file rather than discovering an absence.
    pub const NOTHING: Self = Self {
        input_output_split: false,
        total_tokens: false,
        context_window: false,
        provider_priced: false,
        model: false,
        counts_are_cumulative: false,
        history_backfill: false,
    };

    pub const fn reports_anything(self) -> bool {
        self.input_output_split || self.total_tokens || self.context_window
    }
}

/// ONE provider's usage dialect.
///
/// Providers report different things, in different shapes, and CHANGE
/// them between releases. Each adapter owns its own dialect so a new
/// provider — or a new release of an existing one — extends HERE without
/// touching any other provider's code.
///
/// Follows `HistoryDecoder`'s discipline exactly: `&str` in, typed out,
/// so this crate stays dependency-free and every provider keeps parsing
/// its native format with its own serde.
pub trait ProviderUsage: Send + Sync {
    fn provider_id(&self) -> ProviderId;

    /// What this provider can report. Callers consult this BEFORE
    /// treating an absent number as meaningful.
    fn reports(&self) -> UsageReporting;

    /// Token accounting carried by one native record, if any.
    fn usage_from_line(&self, line: &str) -> Option<TokenUsage>;

    /// The model named by one native record, if any.
    fn model_from_line(&self, _line: &str) -> Option<String> {
        None
    }

    /// Cost the PROVIDER itself computed, in micros. When a provider
    /// reports real money (Claude does), it is authoritative over any
    /// price table of ours — it knows its own billing, we infer it.
    fn cost_micros_from_line(&self, _line: &str) -> Option<u64> {
        None
    }
}

/// One stored session's recovered accounting (the piggyback result).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionUsage {
    pub native_id: ProviderSessionId,
    /// Merged across every record that carried accounting.
    pub usage: TokenUsage,
    pub model: Option<String>,
    /// Provider-reported money, when the provider prices itself.
    pub cost_micros: Option<u64>,
    /// How many records carried accounting — zero means this session had
    /// nothing to recover, which is a fact, not a failure.
    pub records: usize,
}

impl SessionUsage {
    pub const fn is_empty(&self) -> bool {
        self.records == 0
    }
}
