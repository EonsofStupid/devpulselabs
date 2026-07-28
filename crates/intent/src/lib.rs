//! The operator intent gate — the provenance law as a type.
//!
//! Ratified 2026-07-28 (`docs/plans/signal-ledger-and-lesson-foundation.md`
//! §9.3). Privileged transitions — strict determinism on demand, and every
//! future state change expensive enough to require unambiguous operator
//! intent — are authorized by **the provenance of the keystroke, not the
//! pattern in a string**:
//!
//! - A slash command (e.g. `/strict`) is recognized at the operator's
//!   input-submit event, from the operator's own text buffer, *before* that
//!   text ever becomes a message. **Model output is never re-parsed for
//!   commands**; a transcript containing `/strict` is just text.
//! - The UI path is a second-acknowledgement control the coach cannot
//!   synthesize: the pilot attributes injected input as `Agent`, and only
//!   `Operator`-attributed activation may mint the proof. The widget that
//!   calls [`OperatorIntent::ui_acknowledged`] owns that check.
//!
//! Mechanically, [`OperatorIntent`] is a **single-use proof**: no `Clone`,
//! no `Default`, no serde. It cannot arrive over a wire, be minted from
//! config, be replayed from a ledger, or be duplicated once minted —
//! authorizing a transition consumes it. The two constructors name the two
//! channels, so `rg 'OperatorIntent::'` is the complete audit surface of
//! everywhere intent can originate.
//!
//! What this type does NOT claim: it is not a cryptographic boundary inside
//! one process. Its enforcement is that every construction site is greppable
//! and reviewable, and that the type system forbids storing, copying, or
//! deserializing a proof. That is the same trust model as the rest of the
//! no-IPC contract — rustc is the boundary.
//!
//! ```compile_fail
//! // A proof cannot be duplicated: consuming it twice does not compile.
//! let intent = dpl_intent::OperatorIntent::slash_command();
//! let a = intent.consume();
//! let b = intent.consume();
//! ```
//!
//! ```compile_fail
//! // A proof cannot be cloned.
//! let intent = dpl_intent::OperatorIntent::ui_acknowledged();
//! let copy = intent.clone();
//! ```

/// Which authorized channel minted the proof — kept for the audit record a
/// privileged transition writes when it consumes the intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentChannel {
    /// The second-acknowledgement UI control, activated by
    /// `Operator`-attributed input (never by pilot-injected input).
    UiAcknowledged,
    /// A slash command recognized at the operator's input-submit event,
    /// from the operator's own text buffer.
    SlashCommand,
}

/// Proof that the operator, and nobody else, asked for this.
///
/// Deliberately: no `Clone`, no `Copy`, no `Default`, no serde. One proof
/// authorizes one transition.
#[derive(Debug, PartialEq, Eq)]
pub struct OperatorIntent {
    channel: IntentChannel,
}

impl OperatorIntent {
    /// Mint a proof from the second-acknowledgement UI control.
    ///
    /// LAW: the calling widget must have verified the activating input is
    /// `Operator`-attributed. Pilot-injected input is attributed `Agent`
    /// and must be refused before this call.
    pub fn ui_acknowledged() -> Self {
        Self {
            channel: IntentChannel::UiAcknowledged,
        }
    }

    /// Mint a proof from a slash command at the operator's input-submit
    /// event.
    ///
    /// LAW: the caller is the input-submit handler reading the operator's
    /// own text buffer, before that text becomes a message. Nothing that
    /// reads model output, transcripts, or replayed traces may call this.
    pub fn slash_command() -> Self {
        Self {
            channel: IntentChannel::SlashCommand,
        }
    }

    /// The channel that minted this proof, without consuming it — for
    /// display while a confirmation is pending.
    pub fn channel(&self) -> IntentChannel {
        self.channel
    }

    /// Spend the proof, yielding the channel for the audit record the
    /// transition writes. Consumption is the point: after this, the value
    /// is gone and cannot authorize anything else.
    pub fn consume(self) -> IntentChannel {
        self.channel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proof_names_its_channel_and_is_spent_on_consume() {
        let intent = OperatorIntent::slash_command();
        assert_eq!(intent.channel(), IntentChannel::SlashCommand);
        assert_eq!(intent.consume(), IntentChannel::SlashCommand);
        // `intent` is moved; reusing it is a compile error — pinned by the
        // `compile_fail` doctests on the module.

        let intent = OperatorIntent::ui_acknowledged();
        assert_eq!(intent.consume(), IntentChannel::UiAcknowledged);
    }

    #[test]
    fn the_crate_has_no_dependencies_so_a_proof_cannot_learn_to_serialize() {
        // The wire-crossing guarantee is structural, not asserted: this
        // crate depends on nothing, so `serde::Serialize` cannot even be
        // named here. Deriving it would require adding the dependency —
        // a Cargo.toml diff no review misses. This test exists to hold
        // that sentence next to the code it protects.
        let manifest = include_str!("../Cargo.toml");
        let deps = manifest
            .split("[dependencies]")
            .nth(1)
            .expect("a dependencies section")
            .split('[')
            .next()
            .expect("the section body");
        assert!(
            deps.trim().is_empty(),
            "dpl-intent must stay dependency-free; found: {deps}"
        );
    }
}
