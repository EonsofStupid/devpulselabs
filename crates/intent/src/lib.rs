//! The operator intent gate — the provenance law as a type.
//!
//! Ratified 2026-07-28 (`docs-history-2026-10-12:docs/plans/signal-ledger-and-lesson-foundation.md`
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

/// The two determinism modes (signal-ledger plan §9.2).
///
/// Behavioral is the measurement mode: the same trace produces the same
/// step outcomes while pixels and wall-clock may vary. Strict is the
/// evidence mode: byte-identical output. This crate owns the SWITCH and
/// its guards; the pinning strict mode will eventually drive (fixed clock,
/// pinned RNG, locked adapters) lands with replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeterminismMode {
    #[default]
    Behavioral,
    Strict,
}

/// Why the gate is strict right now. Evidence outranks operator demand:
/// when both hold, the cause reads `Evidence`, and de-escalation refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictCause {
    Operator(IntentChannel),
    /// "Evidence forces strict" — policy, not operator whim. Set by runs
    /// whose output will be compared byte-for-byte later (proof shots,
    /// baselines). No proof is consumed; none could be.
    Evidence,
}

/// What `DeterminismGate::deescalate` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deescalation {
    /// Operator-demanded strict cleared; the mode is behavioral again.
    Cleared,
    /// Refused: this run is evidence, and the run is the evidence.
    /// Redaction and policy outrank convenience (§9.4 spirit).
    RefusedEvidence,
    /// Nothing to clear.
    AlreadyBehavioral,
}

#[derive(Default)]
struct GateState {
    operator: Option<IntentChannel>,
    evidence: bool,
}

/// The determinism mode behind the gate — `dpl-intent`'s first gated state.
///
/// A cloneable handle over one state machine. The ONLY way to reach strict
/// on operator demand is [`DeterminismGate::escalate`], which consumes an
/// [`OperatorIntent`] — so every escalation in the program is one of the
/// greppable mint sites, spent here. The gate returns facts and never
/// publishes; callers own the signal-fabric edge, which is how this crate
/// stays dependency-free.
#[derive(Clone, Default)]
pub struct DeterminismGate {
    state: std::sync::Arc<std::sync::Mutex<GateState>>,
}

impl DeterminismGate {
    fn lock(&self) -> std::sync::MutexGuard<'_, GateState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn mode(&self) -> DeterminismMode {
        let state = self.lock();
        if state.evidence || state.operator.is_some() {
            DeterminismMode::Strict
        } else {
            DeterminismMode::Behavioral
        }
    }

    /// Why strict, if strict. Evidence outranks operator demand.
    pub fn strict_cause(&self) -> Option<StrictCause> {
        let state = self.lock();
        if state.evidence {
            Some(StrictCause::Evidence)
        } else {
            state.operator.map(StrictCause::Operator)
        }
    }

    /// Spend a proof to demand strict mode. Returns the channel for the
    /// audit record the caller publishes.
    pub fn escalate(&self, intent: OperatorIntent) -> IntentChannel {
        let channel = intent.consume();
        self.lock().operator = Some(channel);
        channel
    }

    /// Clear operator-demanded strict. No proof — de-escalation is cheap —
    /// but every call site must be provenance-checked operator input, the
    /// same law as the mint sites. Evidence-forced strict refuses: the run
    /// is the evidence.
    pub fn deescalate(&self) -> Deescalation {
        let mut state = self.lock();
        if state.evidence {
            return Deescalation::RefusedEvidence;
        }
        if state.operator.take().is_some() {
            Deescalation::Cleared
        } else {
            Deescalation::AlreadyBehavioral
        }
    }

    /// Policy: this run's output is evidence, so strict is not optional
    /// (§9.2 "evidence forces strict; measurement stays behavioral").
    pub fn force_evidence(&self) {
        self.lock().evidence = true;
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
    fn the_gate_starts_behavioral_and_escalates_only_by_spending_a_proof() {
        let gate = DeterminismGate::default();
        assert_eq!(gate.mode(), DeterminismMode::Behavioral);
        assert_eq!(gate.strict_cause(), None);

        let channel = gate.escalate(OperatorIntent::slash_command());
        assert_eq!(channel, IntentChannel::SlashCommand);
        assert_eq!(gate.mode(), DeterminismMode::Strict);
        assert_eq!(
            gate.strict_cause(),
            Some(StrictCause::Operator(IntentChannel::SlashCommand))
        );
        // The proof is gone — escalating again requires minting again,
        // which is exactly the audit surface working.
    }

    #[test]
    fn deescalation_clears_operator_demand_and_only_operator_demand() {
        let gate = DeterminismGate::default();
        assert_eq!(gate.deescalate(), Deescalation::AlreadyBehavioral);

        gate.escalate(OperatorIntent::ui_acknowledged());
        assert_eq!(gate.deescalate(), Deescalation::Cleared);
        assert_eq!(gate.mode(), DeterminismMode::Behavioral);
    }

    #[test]
    fn evidence_outranks_the_operator_in_cause_and_refuses_deescalation() {
        let gate = DeterminismGate::default();
        gate.force_evidence();
        assert_eq!(gate.mode(), DeterminismMode::Strict);
        assert_eq!(gate.strict_cause(), Some(StrictCause::Evidence));

        // The operator may also demand strict; evidence still names the run.
        gate.escalate(OperatorIntent::slash_command());
        assert_eq!(gate.strict_cause(), Some(StrictCause::Evidence));

        // The run is the evidence — nobody talks it back down.
        assert_eq!(gate.deescalate(), Deescalation::RefusedEvidence);
        assert_eq!(gate.mode(), DeterminismMode::Strict);
    }

    #[test]
    fn clones_share_one_state_because_one_session_has_one_mode() {
        let gate = DeterminismGate::default();
        let other = gate.clone();
        other.escalate(OperatorIntent::ui_acknowledged());
        assert_eq!(gate.mode(), DeterminismMode::Strict);
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
