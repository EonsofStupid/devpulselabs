//! The capability vocabulary, and which capabilities a connected runtime has
//! earned.

use crate::adapter::RuntimeCapabilities;

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

/// Which chat capabilities a connected runtime has EARNED.
///
/// One definition, so Codex and Grok cannot drift into advertising different
/// things for the same protocol facts. This is what makes `can_send()` true —
/// and it is derived from what the provider said, never from a hard-coded
/// list in the app's boot path.
pub fn qualified_capabilities(capabilities: RuntimeCapabilities) -> Vec<Capability> {
    // A connected runtime can always start a session and stream a turn —
    // that IS the protocol. Everything else is advertised.
    let mut qualified = vec![
        Capability::StartSession,
        Capability::StreamTurns,
        Capability::InterruptTurn,
    ];
    if capabilities.load_session {
        qualified.push(Capability::ResumeSession);
    }
    if capabilities.list_sessions {
        qualified.push(Capability::ReadSessions);
    }
    qualified
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_connected_runtime_can_always_start_and_stream() {
        let qualified = qualified_capabilities(RuntimeCapabilities::default());
        assert!(qualified.contains(&Capability::StartSession));
        assert!(qualified.contains(&Capability::StreamTurns));
    }

    /// Advertised capability is EARNED, never assumed: a provider that never
    /// said it can list sessions must not have the surface offer it.
    #[test]
    fn unadvertised_capability_is_never_qualified() {
        let qualified = qualified_capabilities(RuntimeCapabilities::default());
        assert!(!qualified.contains(&Capability::ReadSessions));
        assert!(!qualified.contains(&Capability::ResumeSession));

        let full = qualified_capabilities(RuntimeCapabilities {
            load_session: true,
            list_sessions: true,
            permission_modes: true,
        });
        assert!(full.contains(&Capability::ReadSessions));
        assert!(full.contains(&Capability::ResumeSession));
    }
}
