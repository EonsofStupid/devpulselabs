//! Whether a provider is activated, how the operator recovers when it is not,
//! and the guided values a device-authorization prompt carries.

use crate::provider::ProviderId;

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
}
