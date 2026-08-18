//! THE AI CONTRACT — what the platform must know to reach a provider.
//!
//! Kernel-side and egui-free. This crate exists because the alternative was
//! `dpl-fabric` re-exporting `Capability` and `ProviderId` **from a chat
//! crate** — which made AI a dependency OF the platform core instead of a
//! capability attached AT its boundary, and left
//! `Fabric::providers_that_can()` unreachable for any hub unwilling to depend
//! on the whole chat tree. That is why it had no callers.
//!
//! What belongs here: provider IDENTITY, what a provider CAN DO, whether it
//! is present and signed in, and the PROVENANCE of any value one seeds. What
//! does not belong here is conversation — sessions, turns and events stay in
//! `dpl-chat-core`. A hub asking for a provider should never have to learn
//! what a turn is.
//!
//! Plan: `docs/plans/kernel-boundary.md`.

pub mod composer;

use std::fmt;

/// A compile-time identity: a `&'static str` newtype that prints as itself.
///
/// Exported because `dpl-chat-core` mints its own ids the same way, and two
/// copies of a twelve-line macro is two places for the shape to drift.
#[macro_export]
macro_rules! static_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub &'static str);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.0)
            }
        }
    };
}
static_id!(ProviderId);
static_id!(ProviderProfileId);

/// Where a guided value came from. The source is retained so authored UI can
/// distinguish provider truth from operator input or an AI suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidedValueSource {
    Provider,
    Route,
    Session,
    Operator,
    Ai,
}

/// How strongly a consumer may rely on a seeded value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidedValueConfidence {
    Authoritative,
    Suggested(u8),
}

/// Whether a guided value may cross diagnostics or persistence boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidedValueSensitivity {
    Public,
    Private,
    TransientSecret,
}

/// A pre-populated value with explicit provenance and operator policy.
///
/// This is the boring foundation for route-aware defaults and AI suggestions.
/// Consumers never need to infer whether a value is editable, copyable, or
/// safe to persist from its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidedValue<T> {
    pub value: T,
    pub source: GuidedValueSource,
    pub confidence: GuidedValueConfidence,
    pub editable: bool,
    pub copyable: bool,
    pub sensitivity: GuidedValueSensitivity,
    pub overridden: bool,
}

impl<T> GuidedValue<T> {
    pub const fn new(
        value: T,
        source: GuidedValueSource,
        confidence: GuidedValueConfidence,
        editable: bool,
        copyable: bool,
        sensitivity: GuidedValueSensitivity,
    ) -> Self {
        Self {
            value,
            source,
            confidence,
            editable,
            copyable,
            sensitivity,
            overridden: false,
        }
    }

    pub const fn provider_authoritative(
        value: T,
        copyable: bool,
        sensitivity: GuidedValueSensitivity,
    ) -> Self {
        Self::new(
            value,
            GuidedValueSource::Provider,
            GuidedValueConfidence::Authoritative,
            false,
            copyable,
            sensitivity,
        )
    }

    pub fn replace_by_operator(&mut self, value: T) -> bool {
        if !self.editable {
            return false;
        }
        self.value = value;
        self.source = GuidedValueSource::Operator;
        self.confidence = GuidedValueConfidence::Authoritative;
        self.overridden = true;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderAccent {
    Amber,
    Cyan,
    Violet,
    Magenta,
    Lime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub label: &'static str,
    pub short_label: &'static str,
    pub glyph: &'static str,
    pub accent: ProviderAccent,
    pub summary: &'static str,
    /// `YYYY-MM-DD` the vendor started — what the operator's "how long they
    /// have been in business" figure counts from.
    ///
    /// AUTHORED, and approximate by nature: for some vendors the meaningful
    /// date is the company's founding and for others it is the product's
    /// launch. Each provider crate states which it chose, so a wrong one is
    /// corrected in one obvious place rather than argued about in a widget.
    pub since: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    ReadSessions,
    PageHistory,
    LiveTail,
    StartSession,
    ResumeSession,
    ForkSession,
    StreamTurns,
    InterruptTurn,
    SteerTurn,
    Approvals,
    Tools,
    Diffs,
    Plans,
    Attachments,
    Images,
    Video,
    Models,
    Modes,
    RateLimits,
    SubscriptionAuth,
    DeviceCodeAuth,
    Headless,
}

impl Capability {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadSessions => "read sessions",
            Self::PageHistory => "paged history",
            Self::LiveTail => "live tail",
            Self::StartSession => "start session",
            Self::ResumeSession => "resume session",
            Self::ForkSession => "fork session",
            Self::StreamTurns => "stream turns",
            Self::InterruptTurn => "interrupt turn",
            Self::SteerTurn => "steer turn",
            Self::Approvals => "approvals",
            Self::Tools => "tools",
            Self::Diffs => "diffs",
            Self::Plans => "plans",
            Self::Attachments => "attachments",
            Self::Images => "images",
            Self::Video => "video",
            Self::Models => "models",
            Self::Modes => "modes",
            Self::RateLimits => "rate limits",
            Self::SubscriptionAuth => "subscription auth",
            Self::DeviceCodeAuth => "device-code auth",
            Self::Headless => "headless",
        }
    }

    /// Every capability in the vocabulary. Exhaustive by construction —
    /// `slug`'s match fails to compile if a variant is added without one,
    /// and this array is what proves the round-trip covers them all.
    pub const ALL: [Self; 22] = [
        Self::ReadSessions,
        Self::PageHistory,
        Self::LiveTail,
        Self::StartSession,
        Self::ResumeSession,
        Self::ForkSession,
        Self::StreamTurns,
        Self::InterruptTurn,
        Self::SteerTurn,
        Self::Approvals,
        Self::Tools,
        Self::Diffs,
        Self::Plans,
        Self::Attachments,
        Self::Images,
        Self::Video,
        Self::Models,
        Self::Modes,
        Self::RateLimits,
        Self::SubscriptionAuth,
        Self::DeviceCodeAuth,
        Self::Headless,
    ];

    /// The STABLE wire name — what persistence and the agent seam speak.
    ///
    /// Deliberately not `label`: that is a human string free to be reworded,
    /// and a stored value must not change when a caption does.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::ReadSessions => "read-sessions",
            Self::PageHistory => "page-history",
            Self::LiveTail => "live-tail",
            Self::StartSession => "start-session",
            Self::ResumeSession => "resume-session",
            Self::ForkSession => "fork-session",
            Self::StreamTurns => "stream-turns",
            Self::InterruptTurn => "interrupt-turn",
            Self::SteerTurn => "steer-turn",
            Self::Approvals => "approvals",
            Self::Tools => "tools",
            Self::Diffs => "diffs",
            Self::Plans => "plans",
            Self::Attachments => "attachments",
            Self::Images => "images",
            Self::Video => "video",
            Self::Models => "models",
            Self::Modes => "modes",
            Self::RateLimits => "rate-limits",
            Self::SubscriptionAuth => "subscription-auth",
            Self::DeviceCodeAuth => "device-code-auth",
            Self::Headless => "headless",
        }
    }

    /// Resolve a stored slug. Unknown returns `None` — a capability this
    /// build does not have is DROPPED, never invented (the doctor law).
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.slug() == slug)
    }
}

/// How to ask a provider's OWN tool whether the operator is signed in.
///
/// Plain data, deliberately: this crate spawns nothing and knows nothing
/// about processes (see the module header). The host runs the command on a
/// worker and classifies the output with [`PresenceSpec::classify`].
///
/// Presence is answered by the vendor, never by reading its cookie jar or
/// credential store — that path is encrypted per-OS, breaks on every vendor
/// update, and is credential handling this project does not do. A status
/// subcommand is how the vendor intends to be asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenceSpec {
    /// The CLI as it appears on PATH.
    pub program: &'static str,
    /// The NON-SECRET status subcommand. It must never print a token.
    pub args: &'static [&'static str],
    /// Lowercase needles that mean SIGNED IN.
    pub signed_in: &'static [&'static str],
    /// Lowercase needles that mean installed but SIGNED OUT.
    pub signed_out: &'static [&'static str],
    /// The vendor's OWN sign-in subcommand. Empty means this provider has no
    /// CLI sign-in and the host must not invent one.
    ///
    /// Sign-in is delegated to the vendor for the same reason presence is
    /// read from it: the CLI opens the operator's default browser itself,
    /// runs the vendor's real flow, and DevPULSE never sees a credential.
    pub login: &'static [&'static str],
}

/// What a presence probe found. Exactly the three states a status command can
/// actually distinguish, plus the honest fourth for output nobody taught it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceVerdict {
    SignedIn,
    SignedOut,
    NotInstalled,
    /// The tool ran and said something unrecognized. Never guessed into one
    /// of the others — an unreadable answer is its own fact.
    Unknown,
}

impl PresenceSpec {
    /// Classify a tool's combined stdout/stderr.
    ///
    /// SIGNED OUT is checked first: a signed-out message often still contains
    /// the words a signed-in one does ("run `x login` to log in"), so the
    /// narrower negative has to win.
    pub fn classify(&self, output: &str) -> PresenceVerdict {
        let haystack = output.to_ascii_lowercase();
        if self
            .signed_out
            .iter()
            .any(|needle| haystack.contains(needle))
        {
            return PresenceVerdict::SignedOut;
        }
        if self
            .signed_in
            .iter()
            .any(|needle| haystack.contains(needle))
        {
            return PresenceVerdict::SignedIn;
        }
        PresenceVerdict::Unknown
    }
}

/// Capabilities a provider intends to qualify. These are not active merely
/// because they are present in a manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderManifest {
    pub descriptor: ProviderDescriptor,
    pub profile: ProviderProfileId,
    pub target_capabilities: &'static [Capability],
    pub delivery_slice: &'static str,
    pub evidence: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorizationPrompt {
    pub provider_id: ProviderId,
    pub verification_url: GuidedValue<String>,
    pub user_code: GuidedValue<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationState {
    NotInstalled {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    VersionUnsupported {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    SignedOut {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    SigningIn,
    AwaitingAuthorization(DeviceAuthorizationPrompt),
    Verifying,
    Ready,
    Degraded {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Experimental {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Blocked {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Failed {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Disabled {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
}

impl ActivationState {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::NotInstalled { .. } => "Not installed",
            Self::VersionUnsupported { .. } => "Version unsupported",
            Self::SignedOut { .. } => "Signed out",
            Self::SigningIn => "Signing in",
            Self::AwaitingAuthorization(_) => "Authorize in browser",
            Self::Verifying => "Verifying",
            Self::Ready => "Ready",
            Self::Degraded { .. } => "Degraded",
            Self::Experimental { .. } => "Experimental",
            Self::Blocked { .. } => "Blocked",
            Self::Failed { .. } => "Failed",
            Self::Disabled { .. } => "Disabled",
        }
    }

    pub const fn is_activated(&self) -> bool {
        matches!(self, Self::Ready | Self::Experimental { .. })
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::NotInstalled { reason, .. }
            | Self::VersionUnsupported { reason, .. }
            | Self::SignedOut { reason, .. }
            | Self::Degraded { reason, .. }
            | Self::Experimental { reason, .. }
            | Self::Blocked { reason, .. }
            | Self::Failed { reason, .. }
            | Self::Disabled { reason, .. } => Some(reason),
            Self::SigningIn | Self::AwaitingAuthorization(_) | Self::Verifying | Self::Ready => {
                None
            }
        }
    }

    pub fn recovery(&self) -> &[RecoveryAction] {
        match self {
            Self::NotInstalled { recovery, .. }
            | Self::VersionUnsupported { recovery, .. }
            | Self::SignedOut { recovery, .. }
            | Self::Degraded { recovery, .. }
            | Self::Experimental { recovery, .. }
            | Self::Blocked { recovery, .. }
            | Self::Failed { recovery, .. }
            | Self::Disabled { recovery, .. } => recovery,
            Self::SigningIn | Self::AwaitingAuthorization(_) | Self::Verifying | Self::Ready => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    pub display_name: String,
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryActionKind {
    InstallInstructions,
    SignIn,
    Retry,
    UpdateProvider,
    OpenDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAction {
    pub kind: RecoveryActionKind,
    pub label: String,
}

impl RecoveryAction {
    pub fn new(kind: RecoveryActionKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guided_values_retain_provenance_and_enforce_edit_policy() {
        let mut code = GuidedValue::provider_authoritative(
            "EH38-T38A".to_owned(),
            true,
            GuidedValueSensitivity::TransientSecret,
        );
        assert_eq!(code.source, GuidedValueSource::Provider);
        assert_eq!(code.confidence, GuidedValueConfidence::Authoritative);
        assert!(code.copyable);
        assert!(!code.editable);
        assert!(!code.replace_by_operator("different".to_owned()));
        assert_eq!(code.value, "EH38-T38A");

        let mut suggestion = GuidedValue::new(
            "DevPULSE".to_owned(),
            GuidedValueSource::Ai,
            GuidedValueConfidence::Suggested(82),
            true,
            true,
            GuidedValueSensitivity::Private,
        );
        assert!(suggestion.replace_by_operator("DevPULSE Labs".to_owned()));
        assert_eq!(suggestion.source, GuidedValueSource::Operator);
        assert!(suggestion.overridden);
    }
    #[test]
    fn only_ready_and_explicit_experimental_states_are_activated() {
        assert!(ActivationState::Ready.is_activated());
        assert!(ActivationState::Experimental {
            reason: "fixture".into(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::OpenDiagnostics,
                "Inspect",
            )],
        }
        .is_activated());
        assert!(!ActivationState::Blocked {
            reason: "not qualified".into(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::OpenDiagnostics,
                "Read the plan",
            )],
        }
        .is_activated());
    }
    #[test]
    fn every_capability_slug_round_trips_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for capability in Capability::ALL {
            let slug = capability.slug();
            assert!(
                seen.insert(slug),
                "{slug} is used by two capabilities — a stored set would be ambiguous"
            );
            assert_eq!(
                Capability::from_slug(slug),
                Some(capability),
                "{slug} must resolve back to the capability that wrote it"
            );
        }
        assert_eq!(
            seen.len(),
            Capability::ALL.len(),
            "ALL must list every variant — it is what makes this proof exhaustive"
        );
    }
    #[test]
    fn an_unknown_capability_slug_is_dropped_not_invented() {
        assert_eq!(Capability::from_slug("teleportation"), None);
        assert_eq!(
            Capability::from_slug("read sessions"),
            None,
            "the human label is not the wire name — only the slug resolves"
        );
    }
}
