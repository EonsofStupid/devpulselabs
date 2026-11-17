//! Cursor provider descriptor.
//!
//! The operator, 2026-08-02: *"if cursor has a CLI I want it added a
//! subscription provider."* It does — `curl https://cursor.com/install` puts
//! `cursor-agent` on PATH, and its auth is the SAME Cursor login as the
//! editor, so the operator's plan and limits carry over. That makes it a
//! subscription provider in exactly the sense Codex and Grok are, not an
//! API-key integration.
//!
//! Live turn transport is a later slice. Cursor's descriptor and sign-in
//! check are entries in the AI catalog (dpl_ai_core::catalog, AH1a), so an
//! installed-and-signed-in Cursor becomes visible the moment it exists on the
//! machine.

use dpl_ai_core::provider::ProviderId;

pub const PROVIDER: ProviderId = ProviderId("cursor");

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_ai_core::presence::{PresenceSpec, PresenceVerdict};

    fn presence() -> PresenceSpec {
        dpl_ai_core::catalog::catalog()
            .get(PROVIDER)
            .and_then(|entry| entry.presence)
            .expect("Cursor's catalog entry declares its sign-in check")
    }

    #[test]
    fn a_signed_out_cursor_is_not_read_as_signed_in() {
        // The negative has to win: a signed-out message commonly contains the
        // words a signed-in one does.
        assert_eq!(
            presence().classify("Not logged in. Run `cursor-agent login` to log in."),
            PresenceVerdict::SignedOut
        );
    }

    #[test]
    fn unrecognized_output_is_unknown_not_a_guess() {
        assert_eq!(presence().classify("wat"), PresenceVerdict::Unknown);
    }
}
