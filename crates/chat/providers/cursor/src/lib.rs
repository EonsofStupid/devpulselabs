//! Cursor provider descriptor.
//!
//! The operator, 2026-08-02: *"if cursor has a CLI I want it added a
//! subscription provider."* It does — `curl https://cursor.com/install` puts
//! `cursor-agent` on PATH, and its auth is the SAME Cursor login as the
//! editor, so the operator's plan and limits carry over. That makes it a
//! subscription provider in exactly the sense Codex and Grok are, not an
//! API-key integration.
//!
//! Live turn transport is a later slice. What lands here is the descriptor
//! and the presence probe, so an installed-and-signed-in Cursor becomes
//! visible to the platform the moment it exists on the machine.

use dpl_ai_core::{
    Capability, PresenceSpec, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest,
    ProviderProfileId,
};

pub const PROVIDER: ProviderId = ProviderId("cursor");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("cursor-default");

/// `cursor-agent status` reports the signed-in Cursor account.
///
/// NOT YET VERIFIED against a live run: the CLI is not installed on the
/// operator's machine, so the probe truthfully reports `NotInstalled` today
/// and these needles are confirmed the first time it is present. Recorded as
/// unverified rather than presented as measured — the other three specs were
/// read off real output and this one was not.
pub const PRESENCE: PresenceSpec = PresenceSpec {
    program: "cursor-agent",
    args: &["status"],
    signed_in: &["logged in", "signed in"],
    signed_out: &["not logged in", "not signed in", "run `cursor-agent login`"],
    login: &["login"],
};

/// What Cursor's CLI advertises. Nothing here is active until a slice
/// qualifies it — a manifest is an intention, not a capability.
const TARGETS: &[Capability] = &[
    Capability::ReadSessions,
    Capability::StartSession,
    Capability::ResumeSession,
    Capability::StreamTurns,
    Capability::InterruptTurn,
    Capability::Tools,
    Capability::Diffs,
    Capability::Plans,
    Capability::Models,
    Capability::SubscriptionAuth,
    Capability::Headless,
];

pub const MANIFEST: ProviderManifest = ProviderManifest {
    descriptor: ProviderDescriptor {
        id: PROVIDER,
        label: "Cursor",
        short_label: "Cursor",
        glyph: "▮",
        accent: ProviderAccent::Cyan,
        summary: "Cursor's agent CLI, carrying the same subscription as the editor.",
        // Anysphere, Cursor's maker, founded 2022 — day unknown, so the
        // year's start is used rather than a precise-looking invention.
        since: "2022-01-01",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "A1",
    evidence: "https://cursor.com/docs/cli/using",
};

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_ai_core::PresenceVerdict;

    #[test]
    fn a_signed_out_cursor_is_not_read_as_signed_in() {
        // The negative has to win: a signed-out message commonly contains the
        // words a signed-in one does.
        assert_eq!(
            PRESENCE.classify("Not logged in. Run `cursor-agent login` to log in."),
            PresenceVerdict::SignedOut
        );
    }

    #[test]
    fn unrecognized_output_is_unknown_not_a_guess() {
        assert_eq!(PRESENCE.classify("wat"), PresenceVerdict::Unknown);
    }
}
