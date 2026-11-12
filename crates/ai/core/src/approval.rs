//! What a provider may do without asking the operator first.

/// What a provider may do from chat: the operator's setting (CW1, ruled
/// "configurable permissions"). Provider-neutral; each adapter maps it onto
/// its own controls, and a provider that cannot honour it does not advertise
/// `permission_modes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PermissionMode {
    /// It may read. Anything more waits for the operator's Approve or Deny.
    #[default]
    AskFirst,
    /// It may run commands and edit files inside its workspace. Anything
    /// beyond the workspace still asks.
    Workspace,
    /// It may only read. Every request to do more is declined, with a note
    /// in the chat.
    ReadOnly,
}

impl PermissionMode {
    pub const ALL: [Self; 3] = [Self::AskFirst, Self::Workspace, Self::ReadOnly];

    pub const fn label(self) -> &'static str {
        match self {
            Self::AskFirst => "Ask first",
            Self::Workspace => "Work in the workspace",
            Self::ReadOnly => "Read only",
        }
    }

    pub const fn tooltip(self) -> &'static str {
        match self {
            Self::AskFirst => {
                "It can read your files. Before it runs a command or changes a file, it asks you."
            }
            Self::Workspace => {
                "It can run commands and change files in its workspace folder without asking. \
                 Anything outside that folder still asks you."
            }
            Self::ReadOnly => {
                "It can only read. Every request to run or change something is declined, \
                 and the chat says so."
            }
        }
    }

    /// The stored form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AskFirst => "ask-first",
            Self::Workspace => "workspace",
            Self::ReadOnly => "read-only",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }
}
