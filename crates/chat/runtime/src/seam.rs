//! THE PROVIDER RUNTIME SEAM — one shape every live provider speaks.
//!
//! This crate's header carried an open question from CH1: *"CH1 contains no
//! process transport. Live provider ownership is blocked on the Chat
//! provider-runtime ADR."* This module is the answer.
//!
//! ## Why a seam and not a second Grok
//!
//! Grok grew a working runtime (`GrokAcpRuntime`) and Codex did not, so the
//! app ended up with a provider you can talk to and a provider that only
//! knows you are logged in. The instinct is to copy Grok's file. That yields
//! two runtimes with two vocabularies and two sets of signals, and the third
//! provider makes it three.
//!
//! Instead: the vocabulary below is provider-NEUTRAL, and each provider
//! implements [`ProviderRuntime`] over its own transport. The host then drives
//! every provider through one code path and emits ONE set of fabric signals,
//! so an operator's Codex turn and an operator's Grok turn are observable in
//! exactly the same way.
//!
//! Grok's existing types map onto these one-for-one — the vocabulary was
//! already neutral in shape, only Grok-named — so aligning it is a
//! translation, not a rewrite of a working runtime.
//!
//! ## The contract
//!
//! A runtime is a CHANNEL, never a blocking call: commands go in, events come
//! out, and the caller polls. Nothing here may block the UI thread, because
//! every implementation is driven from a frame.

use crate::ProviderId;
use std::fmt;
use std::path::PathBuf;

/// What a connected runtime turned out to support.
///
/// Advertised by the provider at connect time, not assumed. A provider that
/// cannot list sessions is not broken — the surface simply must not offer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimeCapabilities {
    pub load_session: bool,
    pub list_sessions: bool,
}

/// One session as the PROVIDER describes it.
///
/// Distinct from [`crate::SessionSummary`], which is how the APP describes a
/// session. The host translates; conflating them is how a provider's id
/// vocabulary leaks into the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionSummary {
    pub provider_session_id: String,
    pub title: Option<String>,
    pub working_directory: PathBuf,
    pub updated_at: Option<String>,
}

/// A fragment of a turn, as it arrives.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeUpdate {
    UserText(String),
    AssistantText(String),
    ThoughtText(String),
    Tool(String),
    Plan(String),
    SessionInfo(String),
    AvailableCommands {
        count: usize,
    },
    Usage {
        used: u64,
        size: u64,
    },
    /// Anything the adapter recognised as real but has no typed slot for.
    /// Kept rather than dropped — an unknown update is evidence.
    Other(String),
}

/// Why a runtime could not do the thing.
///
/// Typed because the SURFACE reacts differently to each: a credential problem
/// offers sign-in, a launch problem offers install, backpressure is transient
/// and must not look like a failure the operator caused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFailureKind {
    Launch,
    Protocol,
    CredentialUnavailable,
    Authentication,
    Unsupported,
    InvalidCommand,
    Backpressure,
    Timeout,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFailure {
    pub kind: RuntimeFailureKind,
    pub reason: String,
    pub provider_session_id: Option<String>,
}

impl RuntimeFailure {
    pub fn new(kind: RuntimeFailureKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
            provider_session_id: None,
        }
    }

    pub fn for_session(
        kind: RuntimeFailureKind,
        reason: impl Into<String>,
        provider_session_id: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            reason: reason.into(),
            provider_session_id: Some(provider_session_id.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Exactly the four the protocol defines. No `Other`: a permission choice
/// the app cannot name is one it must not offer.
pub enum RuntimePermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePermissionOption {
    /// Named `option_id` to match `RuntimeCommand::ResolvePermission`, so the
    /// value the operator picks and the value sent back read the same.
    pub option_id: String,
    pub label: String,
    pub kind: RuntimePermissionOptionKind,
}

/// What the operator asked the provider to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCommand {
    ListSessions {
        cursor: Option<String>,
    },
    NewSession,
    /// Ask what models exist and what each one accepts.
    ListModels,
    /// Ask which MCP servers are configured, how they authenticate, and what
    /// tools they bring.
    ListMcpServers,
    /// Begin OAuth for one server. The answer is a URL for the operator to
    /// open; completion arrives later as a notification.
    McpOauthLogin {
        name: String,
    },
    /// Re-read the MCP server configuration from disk.
    ReloadMcpServers,
    LoadSession {
        provider_session_id: String,
    },
    Prompt {
        provider_session_id: String,
        text: String,
        /// The modes standing on the composer when this turn was sent.
        ///
        /// Carried as canonical slugs so an adapter maps ONE vocabulary to its
        /// own wire, and so a mode a provider cannot honour can be named back
        /// to the operator rather than dropped in silence.
        modes: dpl_ai_core::composer::ComposerModes,
    },
    Cancel {
        provider_session_id: String,
    },
    ResolvePermission {
        request_id: u64,
        option_id: Option<String>,
    },
    Shutdown,
}

/// What one model will actually accept, as the provider advertises it.
///
/// Provider-neutral on purpose: every adapter that can describe its models
/// fills this in, and an adapter that cannot leaves it `None` — which is read
/// as "nothing verified", never as "anything goes".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelCapabilities {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
    /// The reasoning-effort values THIS model advertises. Opaque strings by
    /// design — the protocol calls them "advertised by the model", so a set
    /// hardcoded here would be a guess with a version number on it.
    pub reasoning_efforts: Vec<String>,
    pub default_reasoning_effort: Option<String>,
    pub supports_personality: bool,
}

impl ModelCapabilities {
    /// Does this model advertise the named effort, exactly?
    pub fn advertises_effort(&self, effort: &str) -> bool {
        self.reasoning_efforts.iter().any(|e| e == effort)
    }
}

/// How a connected MCP server is authenticated, as the provider reports it.
///
/// `schema/v2/ListMcpServerStatusResponse.json`: unknown | unsupported |
/// notLoggedIn | bearerToken | oAuth. Anything unrecognised reads as
/// [`Self::Unknown`] rather than being guessed into a state — "we do not know"
/// is a real answer and the operator can act on it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum McpAuthStatus {
    #[default]
    Unknown,
    Unsupported,
    NotLoggedIn,
    BearerToken,
    OAuth,
}

impl McpAuthStatus {
    pub fn from_wire(value: &str) -> Self {
        match value {
            "unsupported" => Self::Unsupported,
            "notLoggedIn" => Self::NotLoggedIn,
            "bearerToken" => Self::BearerToken,
            "oAuth" => Self::OAuth,
            _ => Self::Unknown,
        }
    }

    /// Is this server usable right now?
    ///
    /// `NotLoggedIn` is the one that matters: the server is configured, its
    /// tools are listed, and NONE of them will work until somebody signs in.
    /// A surface that showed it as connected would be lying by omission.
    ///
    /// Auth ONLY. Whether the process is up is a separate question with a
    /// separate answer — see [`McpServerStatus::is_usable`], which is the one
    /// callers should ask.
    pub const fn is_authenticated(self) -> bool {
        matches!(self, Self::BearerToken | Self::OAuth | Self::Unsupported)
    }
}

/// Whether the server process actually came up.
///
/// `schema/v2/...`: starting | ready | failed | cancelled. ORTHOGONAL to
/// authentication — a server can be perfectly signed in and still have crashed
/// on boot, and asking only about auth is how a dead connector gets reported
/// as usable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum McpStartupState {
    /// Nothing has said yet. Not the same as broken: the list response carries
    /// no startup field, so this is where every server begins.
    #[default]
    Unreported,
    Starting,
    Ready,
    Failed,
    Cancelled,
}

impl McpStartupState {
    pub fn from_wire(value: &str) -> Self {
        match value {
            "starting" => Self::Starting,
            "ready" => Self::Ready,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Unreported,
        }
    }

    /// Has this server definitively failed to come up?
    ///
    /// `Unreported` is deliberately NOT a failure. The list response has no
    /// startup field, so a server that answered with its tools is evidence it
    /// is running; treating silence as broken would hide every working
    /// connector until a notification happened to arrive.
    pub const fn is_down(self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled)
    }
}

/// One MCP server the provider has, and what it brings.
///
/// `tools` is the load-bearing field: a mode like web search is not a flag on
/// a turn, it is whether some connected server exposes a tool that does it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpServerStatus {
    pub name: String,
    pub auth: McpAuthStatus,
    /// Tool names, sorted so a surface paints them stably.
    pub tools: Vec<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// Whether the process came up. Filled by
    /// `mcpServer/startupStatus/updated`, so it stays live rather than being
    /// a snapshot from whenever the list was last asked.
    pub startup: McpStartupState,
    /// Why it failed, when it did. Shown verbatim — a connector that is down
    /// for a reason the operator can fix is only fixable if they are told it.
    pub failure: Option<String>,
}

/// Which composer modes a connected MCP server makes AVAILABLE to the agent.
///
/// Read the word carefully: available, not carried, and not enforced.
///
/// A connected search tool means the agent CAN search. It does not mean this
/// turn will. No protocol here has a field that compels a tool call, so a
/// toggle cannot make one happen — the agent decides, per turn, from the
/// prompt.
///
/// Treating available as carried is the trap. It removes the honest refusal
/// and replaces it with silence: the operator flips web search, nothing
/// objects, and nothing searches. That is worse than the refusal it replaced,
/// which is why [`ModeStanding`] has three states and not two.
///
/// Matching is on the tool name containing the marker, because MCP tool naming
/// is a convention rather than a registry: `web_search`, `brave_web_search`
/// and `search_web` all mean the same thing and no vendor coordinates them.
/// The cost of the loose match is a false positive on a tool that merely has
/// "search" in its name; the cost of an exact list is a connector that works
/// and is reported as absent.
pub fn modes_from_tools(servers: &[McpServerStatus]) -> Vec<&'static str> {
    let usable: Vec<&str> = servers
        .iter()
        .filter(|server| server.is_usable())
        .flat_map(|server| server.tools.iter().map(String::as_str))
        .collect();
    let has = |marker: &str| {
        usable
            .iter()
            .any(|tool| tool.to_ascii_lowercase().contains(marker))
    };
    let mut modes = Vec::new();
    if has("search") {
        modes.push("web-search");
    }
    if has("research") || has("deep_research") {
        modes.push("research");
    }
    modes
}

/// Where one standing mode actually got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeStanding {
    /// On the wire. The provider was told, in a field its schema defines.
    Carried,
    /// A connected tool can do this, but nothing compels the agent to use it
    /// on this turn. Honest middle ground, and the one that did not exist.
    Available,
    /// Neither. The operator is told, by name.
    Refused,
}

impl McpServerStatus {
    /// Can this server's tools actually run right now?
    ///
    /// BOTH questions, because either one alone is a half-truth: signed in but
    /// crashed is not usable, and running but signed out is not usable. The
    /// first version of this asked only about auth, which would have reported
    /// a dead connector as ready to serve.
    pub fn is_usable(&self) -> bool {
        self.auth.is_authenticated() && !self.startup.is_down()
    }

    /// Why this server cannot serve, in words for an operator. `None` when it
    /// can.
    pub fn blocked_reason(&self) -> Option<String> {
        if self.startup.is_down() {
            return Some(
                self.failure
                    .clone()
                    .unwrap_or_else(|| format!("{} did not start", self.name)),
            );
        }
        if !self.auth.is_authenticated() {
            return Some(format!("{} is not signed in", self.name));
        }
        None
    }
}

/// What the provider says back.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    Connected {
        capabilities: RuntimeCapabilities,
    },
    SessionsPage {
        sessions: Vec<RuntimeSessionSummary>,
        next_cursor: Option<String>,
        replace: bool,
    },
    /// What the provider's default model advertises, once it has said so.
    ///
    /// Not session-scoped: the answer is about the model, not a thread, and it
    /// is what turns a composer mode from "refused, unverified" into something
    /// this adapter will actually put on a turn.
    ModelsListed {
        models: Vec<ModelCapabilities>,
    },
    /// The MCP servers this provider holds. Not session-scoped: a connector is
    /// a property of the provider, not of one thread.
    McpServersListed {
        servers: Vec<McpServerStatus>,
    },
    /// One server's process state changed. Arrives unprompted, which is the
    /// point: a list is a snapshot and a connector can die after it was taken.
    McpServerStartup {
        name: String,
        startup: McpStartupState,
        failure: Option<String>,
    },
    /// An OAuth sign-in finished, for good or ill.
    McpOauthCompleted {
        name: String,
        success: bool,
        error: Option<String>,
    },
    /// Where to send the operator to authorise a server.
    McpOauthUrl {
        name: String,
        url: String,
    },
    SessionReady {
        provider_session_id: String,
        is_new: bool,
    },
    TurnStarted {
        provider_session_id: String,
    },
    SessionUpdate {
        provider_session_id: String,
        update: RuntimeUpdate,
    },
    PermissionRequested {
        request_id: u64,
        provider_session_id: String,
        summary: String,
        options: Vec<RuntimePermissionOption>,
    },
    TurnFinished {
        provider_session_id: String,
        stop_reason: String,
    },
    Failure(RuntimeFailure),
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSendError {
    /// The command queue is full. Transient — retry, never a failure to show.
    Full,
    /// The runtime is gone. Terminal for this handle.
    Closed,
}

impl fmt::Display for RuntimeSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => formatter.write_str("the provider command queue is full"),
            Self::Closed => formatter.write_str("the provider runtime is closed"),
        }
    }
}

impl std::error::Error for RuntimeSendError {}

/// A live provider process, as the host sees it.
///
/// Deliberately tiny. Everything specific — the executable, the wire format,
/// the handshake, the retry policy — lives behind the implementation, because
/// that is the ONLY part that legitimately differs between providers.
///
/// `Send` because the host owns these in shared state.
pub trait ProviderRuntime: Send {
    /// Which provider this runtime speaks for. The host stamps every signal
    /// with it, so one code path can serve them all.
    fn provider(&self) -> ProviderId;

    /// Queue a command. Never blocks.
    fn try_send(&self, command: RuntimeCommand) -> Result<(), RuntimeSendError>;

    /// Take the next event, if one is waiting. Never blocks.
    fn try_event(&self) -> Result<Option<RuntimeEvent>, RuntimeSendError>;

    /// The capabilities the provider advertised at connect, if it has
    /// connected. `None` before `Connected` arrives — a runtime must not
    /// claim capability it has not been told it has.
    fn capabilities(&self) -> Option<RuntimeCapabilities>;
}

/// Which chat capabilities a connected runtime has EARNED.
///
/// One definition, so Codex and Grok cannot drift into advertising different
/// things for the same protocol facts. This is what makes `can_send()` true —
/// and it is derived from what the provider said, never from a hard-coded
/// list in the app's boot path.
pub fn qualified_capabilities(capabilities: RuntimeCapabilities) -> Vec<crate::Capability> {
    use crate::Capability;
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
    use crate::Capability;

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
        });
        assert!(full.contains(&Capability::ReadSessions));
        assert!(full.contains(&Capability::ResumeSession));
    }

    #[test]
    fn a_full_queue_is_transient_and_a_closed_runtime_is_not() {
        assert_eq!(
            RuntimeSendError::Full.to_string(),
            "the provider command queue is full"
        );
        assert_ne!(RuntimeSendError::Full, RuntimeSendError::Closed);
    }

    #[test]
    fn a_failure_can_name_the_session_it_belongs_to() {
        let loose = RuntimeFailure::new(RuntimeFailureKind::Launch, "no executable");
        assert_eq!(loose.provider_session_id, None);
        let scoped =
            RuntimeFailure::for_session(RuntimeFailureKind::Protocol, "bad frame", "session-1");
        assert_eq!(scoped.provider_session_id.as_deref(), Some("session-1"));
    }
}

#[cfg(test)]
mod composer_mode_tests {
    use super::*;

    fn server(name: &str, auth: McpAuthStatus, tools: &[&str]) -> McpServerStatus {
        McpServerStatus {
            name: name.to_owned(),
            auth,
            tools: tools.iter().map(|t| (*t).to_owned()).collect(),
            version: None,
            description: None,
            startup: McpStartupState::Unreported,
            failure: None,
        }
    }

    fn down(name: &str) -> McpServerStatus {
        let mut server = server(name, McpAuthStatus::OAuth, &["web_search"]);
        server.startup = McpStartupState::Failed;
        server.failure = Some("port already in use".to_owned());
        server
    }

    /// The whole point of Phase 1: web search becomes real because a CONNECTOR
    /// does it, not because a turn carries a flag.
    #[test]
    fn a_signed_in_search_tool_makes_web_search_honourable() {
        let connected = [server("brave", McpAuthStatus::OAuth, &["brave_web_search"])];
        assert_eq!(modes_from_tools(&connected), ["web-search"]);
    }

    /// The trap this exists to close: configured, tools listed, and NOTHING
    /// works because nobody signed in. It must not count.
    #[test]
    fn a_signed_out_server_honours_nothing() {
        let waiting = [server(
            "brave",
            McpAuthStatus::NotLoggedIn,
            &["brave_web_search"],
        )];
        assert!(modes_from_tools(&waiting).is_empty());
        let unknown = [server("x", McpAuthStatus::Unknown, &["web_search"])];
        assert!(modes_from_tools(&unknown).is_empty());
    }

    #[test]
    fn tool_naming_is_a_convention_so_the_match_is_loose() {
        for name in [
            "web_search",
            "brave_web_search",
            "search_web",
            "SearchTheWeb",
        ] {
            let connected = [server("s", McpAuthStatus::BearerToken, &[name])];
            assert_eq!(modes_from_tools(&connected), ["web-search"], "{name}");
        }
        let none = [server(
            "s",
            McpAuthStatus::OAuth,
            &["read_file", "write_file"],
        )];
        assert!(
            modes_from_tools(&none).is_empty(),
            "no search tool, no mode"
        );
    }

    /// The regression this file exists to prevent, written down.
    ///
    /// A connected tool must NEVER end up in the carried list. Carried means
    /// the provider was told in a field its schema defines; a tool means the
    /// agent could choose to. Folding the second into the first deletes the
    /// refusal and puts nothing in its place — the operator flips web search,
    /// nothing objects, and nothing searches.
    #[test]
    fn a_tool_is_available_and_never_carried() {
        let connected = [server("brave", McpAuthStatus::OAuth, &["brave_web_search"])];
        let from_tools = modes_from_tools(&connected);
        assert_eq!(from_tools, ["web-search"], "available");

        // What the WIRE carries is a different question with a different
        // answer, and nothing here may widen it.
        let wire_carried = ["extended-thinking", "style"];
        for mode in &from_tools {
            assert!(
                !wire_carried.contains(mode),
                "{mode} came from a tool and must not be treated as carried"
            );
        }
    }

    /// The gap the first version of this had: it asked only about auth, so a
    /// signed-in connector that crashed on boot counted as usable.
    #[test]
    fn a_crashed_server_is_not_usable_however_well_signed_in() {
        let dead = down("brave");
        assert!(dead.auth.is_authenticated(), "auth really is fine");
        assert!(!dead.is_usable(), "and it still cannot serve");
        assert_eq!(
            dead.blocked_reason().as_deref(),
            Some("port already in use")
        );
        assert!(
            modes_from_tools(&[dead]).is_empty(),
            "a dead server offers no modes, whatever tools it listed"
        );
    }

    /// Unreported is not broken. The list response carries no startup field,
    /// so treating silence as failure would hide every working connector.
    #[test]
    fn an_unreported_startup_is_not_a_failure() {
        let fresh = server("brave", McpAuthStatus::OAuth, &["web_search"]);
        assert_eq!(fresh.startup, McpStartupState::Unreported);
        assert!(fresh.is_usable());
        assert_eq!(fresh.blocked_reason(), None);
        assert_eq!(modes_from_tools(&[fresh]), ["web-search"]);
    }

    /// Both blockers are reported with their OWN reason. "not signed in" and
    /// "did not start" are different problems with different fixes, and one
    /// vague line would send the operator after the wrong one.
    #[test]
    fn each_blocker_names_itself() {
        let signed_out = server("github", McpAuthStatus::NotLoggedIn, &["search_issues"]);
        assert_eq!(
            signed_out.blocked_reason().as_deref(),
            Some("github is not signed in")
        );
        let mut crashed_no_reason = server("x", McpAuthStatus::OAuth, &[]);
        crashed_no_reason.startup = McpStartupState::Cancelled;
        assert_eq!(
            crashed_no_reason.blocked_reason().as_deref(),
            Some("x did not start"),
            "no message from the provider still gets a sentence"
        );
    }

    #[test]
    fn research_is_its_own_tool_not_implied_by_search() {
        let search_only = [server("s", McpAuthStatus::OAuth, &["web_search"])];
        assert_eq!(modes_from_tools(&search_only), ["web-search"]);
        let both = [server(
            "s",
            McpAuthStatus::OAuth,
            &["web_search", "deep_research"],
        )];
        assert_eq!(modes_from_tools(&both), ["web-search", "research"]);
    }
}
