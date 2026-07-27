use crate::GrokRuntimeFailureKind;
use std::time::Duration;

const RETRY_DELAYS: [Duration; 5] = [
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokVerificationOrigin {
    Startup,
    PostLogin,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokRuntimeFailureDisposition {
    Retry {
        retry_index: u8,
        max_retries: u8,
        delay: Duration,
    },
    SignedOut,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokRuntimeRetryState {
    origin: GrokVerificationOrigin,
    retries_issued: usize,
}

impl GrokRuntimeRetryState {
    #[must_use]
    pub const fn new(origin: GrokVerificationOrigin) -> Self {
        Self {
            origin,
            retries_issued: 0,
        }
    }

    #[must_use]
    pub const fn origin(&self) -> GrokVerificationOrigin {
        self.origin
    }

    pub fn on_failure(&mut self, kind: GrokRuntimeFailureKind) -> GrokRuntimeFailureDisposition {
        let terminal = match kind {
            GrokRuntimeFailureKind::CredentialUnavailable
                if self.origin == GrokVerificationOrigin::Startup =>
            {
                GrokRuntimeFailureDisposition::SignedOut
            }
            GrokRuntimeFailureKind::CredentialUnavailable
            | GrokRuntimeFailureKind::Authentication => GrokRuntimeFailureDisposition::SignedOut,
            GrokRuntimeFailureKind::Launch
            | GrokRuntimeFailureKind::Timeout
            | GrokRuntimeFailureKind::Disconnected => GrokRuntimeFailureDisposition::Failed,
            GrokRuntimeFailureKind::Protocol
            | GrokRuntimeFailureKind::Unsupported
            | GrokRuntimeFailureKind::InvalidCommand
            | GrokRuntimeFailureKind::Backpressure => GrokRuntimeFailureDisposition::Failed,
        };

        let retryable = match kind {
            GrokRuntimeFailureKind::CredentialUnavailable => {
                self.origin != GrokVerificationOrigin::Startup
            }
            GrokRuntimeFailureKind::Authentication
            | GrokRuntimeFailureKind::Launch
            | GrokRuntimeFailureKind::Timeout
            | GrokRuntimeFailureKind::Disconnected => true,
            GrokRuntimeFailureKind::Protocol
            | GrokRuntimeFailureKind::Unsupported
            | GrokRuntimeFailureKind::InvalidCommand
            | GrokRuntimeFailureKind::Backpressure => false,
        };

        if retryable && self.retries_issued < RETRY_DELAYS.len() {
            let delay = RETRY_DELAYS[self.retries_issued];
            self.retries_issued += 1;
            GrokRuntimeFailureDisposition::Retry {
                retry_index: u8::try_from(self.retries_issued).unwrap_or(u8::MAX),
                max_retries: u8::try_from(RETRY_DELAYS.len()).unwrap_or(u8::MAX),
                delay,
            }
        } else {
            terminal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_without_cached_credentials_is_immediately_signed_out() {
        let mut state = GrokRuntimeRetryState::new(GrokVerificationOrigin::Startup);
        assert_eq!(
            state.on_failure(GrokRuntimeFailureKind::CredentialUnavailable),
            GrokRuntimeFailureDisposition::SignedOut
        );
    }

    #[test]
    fn post_login_authentication_uses_the_bounded_backoff_then_signs_out() {
        let mut state = GrokRuntimeRetryState::new(GrokVerificationOrigin::PostLogin);
        let expected = [100, 250, 500, 1_000, 2_000];
        for (index, expected_ms) in expected.into_iter().enumerate() {
            assert_eq!(
                state.on_failure(GrokRuntimeFailureKind::Authentication),
                GrokRuntimeFailureDisposition::Retry {
                    retry_index: u8::try_from(index + 1).unwrap(),
                    max_retries: 5,
                    delay: Duration::from_millis(expected_ms),
                }
            );
        }
        assert_eq!(
            state.on_failure(GrokRuntimeFailureKind::Authentication),
            GrokRuntimeFailureDisposition::SignedOut
        );
    }

    #[test]
    fn recovery_disconnect_retries_then_fails_without_claiming_signed_out() {
        let mut state = GrokRuntimeRetryState::new(GrokVerificationOrigin::Recovery);
        for _ in 0..RETRY_DELAYS.len() {
            assert!(matches!(
                state.on_failure(GrokRuntimeFailureKind::Disconnected),
                GrokRuntimeFailureDisposition::Retry { .. }
            ));
        }
        assert_eq!(
            state.on_failure(GrokRuntimeFailureKind::Disconnected),
            GrokRuntimeFailureDisposition::Failed
        );
    }

    #[test]
    fn protocol_incompatibility_never_retries() {
        let mut state = GrokRuntimeRetryState::new(GrokVerificationOrigin::PostLogin);
        assert_eq!(
            state.on_failure(GrokRuntimeFailureKind::Protocol),
            GrokRuntimeFailureDisposition::Failed
        );
    }
}
