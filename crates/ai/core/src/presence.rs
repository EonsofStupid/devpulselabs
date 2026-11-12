//! Asking a provider's own tool whether the operator is signed in.

/// How to ask a provider's OWN tool whether the operator is signed in.
///
/// Plain data, deliberately: this crate spawns nothing and knows nothing
/// about processes (see the crate header). The host runs the command on a
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
