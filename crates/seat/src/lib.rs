//! Who is in the chair — the canonical seat definition, and nothing else.
//!
//! Ceiling: `serde`, `serde_json`. A leaf on purpose (see below).
//!
//! ## The definition
//!
//! > A seat is a licensed occupant of a workspace.
//!
//! Two axes, independent of each other
//! (`shippin/docs/platform/NAMING-AND-SSOT.md` §1.1):
//!
//! | axis | values | means |
//! |---|---|---|
//! | occupant | human · agent | who is in the chair |
//! | basis | subscription · usage · local · unknown | how its capacity is paid for |
//!
//! Neither axis was written down before, so four documents each picked one
//! and drifted: one said customers buy seats, another that they buy
//! workspaces, Automaton called a vendor CLI a seat, and a provenance table
//! called a licence one. All four are correct under this definition. Nothing
//! needed renaming — the word was always right and its axes were missing.
//!
//! ## Why this is its own crate and not a module somewhere
//!
//! Two layers need it and neither may depend on the other. The document layer
//! attributes a board item to whoever put it there; the AI subsystem knows a
//! provider IS an agent seat. `dpl-workflows`'s ceiling is exactly
//! `[dpl-comp, dpl-doc, serde, serde_json]`, so it cannot reach
//! `dpl-ai-core`; and `dpl-ai-core` depending on a document crate would invert
//! the layer stack. A leaf both can see is the only shape that breaks
//! neither, which is why this is four types and no behaviour.
//!
//! ## A VM is not a seat
//!
//! Seats *occupy* workspaces; the workspace is the runtime unit. That is the
//! distinction that reconciles "customers buy seats" with "customers buy
//! workspaces" — they are different purchases.

use serde::{Deserialize, Serialize};

/// Who is in the chair.
///
/// `Agent` is the customer-legible half of "non-human identity" (NHI) — the
/// field's term. Say agent in product copy, NHI to security people; they are
/// the same thing.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Occupant {
    #[default]
    Human,
    Agent,
}

/// How a seat's capacity is paid for.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Basis {
    Subscription,
    Usage,
    Local,
    /// Not measured yet. **Never promotes to `Subscription`** — see
    /// [`Basis::promote`].
    #[default]
    Unknown,
}

impl Basis {
    /// Take a newly measured basis, refusing the one promotion that is a bill.
    ///
    /// `unknown` → `subscription` is the move this exists to block. A basis
    /// that has not been measured is not a subscription, and guessing is how
    /// a per-usage bill arrives that nobody chose. Law, not an implementation
    /// detail: Automaton enforces the same rule in its own operator layer.
    ///
    /// Returns whether the value changed.
    pub fn promote(&mut self, measured: Self) -> bool {
        if *self == Self::Unknown && measured == Self::Subscription {
            return false;
        }
        if *self == measured {
            return false;
        }
        *self = measured;
        true
    }
}

/// A licensed occupant of a workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Seat {
    pub occupant: Occupant,
    #[serde(default)]
    pub basis: Basis,
    /// The principal this seat authenticates as — a provider id for an agent,
    /// the operator's own id for a human. Free-form because the shape differs
    /// per surface and pinning it here would make this crate know about them.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
}

impl Seat {
    /// The person at the keyboard.
    pub fn operator() -> Self {
        Self {
            occupant: Occupant::Human,
            basis: Basis::Subscription,
            id: String::new(),
        }
    }

    /// An agent seat backed by a named provider.
    pub fn agent(id: impl Into<String>, basis: Basis) -> Self {
        Self {
            occupant: Occupant::Agent,
            basis,
            id: id.into(),
        }
    }

    pub const fn is_human(&self) -> bool {
        matches!(self.occupant, Occupant::Human)
    }

    pub const fn is_agent(&self) -> bool {
        matches!(self.occupant, Occupant::Agent)
    }

    /// What a surface calls this seat. Falls back to the axis when no
    /// principal was recorded, because "agent" is still true and still useful.
    pub fn label(&self) -> String {
        if self.id.is_empty() {
            return match self.occupant {
                Occupant::Human => "you".to_owned(),
                Occupant::Agent => "an agent".to_owned(),
            };
        }
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_axes_are_independent() {
        // The whole point of the definition: every combination is meaningful,
        // and none of them is derivable from the other axis.
        let human_sub = Seat::operator();
        assert!(human_sub.is_human() && !human_sub.is_agent());

        let metered = Seat::agent("fallback", Basis::Usage);
        assert!(metered.is_agent());
        assert_eq!(metered.basis, Basis::Usage);

        let local = Seat::agent("llama", Basis::Local);
        assert_eq!(local.basis, Basis::Local);
    }

    /// The law. A basis that has not been measured is not a subscription, and
    /// guessing it into one is how somebody gets a bill they never chose.
    #[test]
    fn unknown_never_promotes_to_subscription() {
        let mut basis = Basis::Unknown;
        assert!(!basis.promote(Basis::Subscription), "refused");
        assert_eq!(basis, Basis::Unknown, "and really did not move");

        // Every other measurement lands, including from unknown.
        assert!(basis.promote(Basis::Usage));
        assert_eq!(basis, Basis::Usage);
        // And once measured, subscription is a legitimate later reading —
        // the rule is about promoting from ignorance, not about the value.
        assert!(basis.promote(Basis::Subscription));
        assert_eq!(basis, Basis::Subscription);
        assert!(
            !basis.promote(Basis::Subscription),
            "no change is no change"
        );
    }

    #[test]
    fn a_seat_names_itself_even_with_no_principal() {
        assert_eq!(Seat::operator().label(), "you");
        assert_eq!(
            Seat {
                occupant: Occupant::Agent,
                ..Seat::default()
            }
            .label(),
            "an agent"
        );
        assert_eq!(Seat::agent("codex", Basis::Subscription).label(), "codex");
    }

    /// Defaults are the honest ones: nobody in particular, and a basis nobody
    /// has measured.
    #[test]
    fn the_defaults_claim_nothing() {
        let seat = Seat::default();
        assert_eq!(seat.occupant, Occupant::Human);
        assert_eq!(seat.basis, Basis::Unknown);
        assert!(seat.id.is_empty());
    }

    #[test]
    fn the_wire_form_is_the_vocabulary() {
        let seat = Seat::agent("codex", Basis::Subscription);
        let json = serde_json::to_string(&seat).expect("serialises");
        assert!(json.contains(r#""occupant":"agent""#), "{json}");
        assert!(json.contains(r#""basis":"subscription""#), "{json}");
        let back: Seat = serde_json::from_str(&json).expect("round trips");
        assert_eq!(back, seat);
    }
}
