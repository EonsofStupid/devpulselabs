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

    /// The most a project's trust grant allows (AH1b): read, change and run,
    /// as the operator ticked them. `None` when it may not even read, so it is
    /// not started in the project at all.
    ///
    /// A grant that allows only one of change and run maps to ask-first,
    /// because no adapter can allow one without the other: ask-first asks
    /// before both, so it never allows more than was granted.
    pub const fn ceiling(read: bool, change: bool, run: bool) -> Option<Self> {
        match (read, change, run) {
            (false, _, _) => None,
            (true, false, false) => Some(Self::ReadOnly),
            (true, true, true) => Some(Self::Workspace),
            (true, _, _) => Some(Self::AskFirst),
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::ReadOnly => 0,
            Self::AskFirst => 1,
            Self::Workspace => 2,
        }
    }

    /// Whether this mode allows no more than `ceiling` does.
    pub const fn within(self, ceiling: Self) -> bool {
        self.rank() <= ceiling.rank()
    }

    /// This mode, lowered to `ceiling` when it would allow more.
    pub const fn capped(self, ceiling: Self) -> Self {
        if self.within(ceiling) {
            self
        } else {
            ceiling
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PermissionMode;

    #[test]
    fn a_grant_never_maps_to_more_than_it_allows() {
        assert_eq!(PermissionMode::ceiling(false, true, true), None);
        assert_eq!(
            PermissionMode::ceiling(true, false, false),
            Some(PermissionMode::ReadOnly)
        );
        assert_eq!(
            PermissionMode::ceiling(true, true, false),
            Some(PermissionMode::AskFirst)
        );
        assert_eq!(
            PermissionMode::ceiling(true, false, true),
            Some(PermissionMode::AskFirst)
        );
        assert_eq!(
            PermissionMode::ceiling(true, true, true),
            Some(PermissionMode::Workspace)
        );
    }

    #[test]
    fn a_saved_mode_above_the_ceiling_is_lowered() {
        let workspace = PermissionMode::Workspace;
        assert_eq!(
            workspace.capped(PermissionMode::ReadOnly),
            PermissionMode::ReadOnly
        );
        assert_eq!(
            PermissionMode::ReadOnly.capped(PermissionMode::Workspace),
            PermissionMode::ReadOnly
        );
        assert!(PermissionMode::AskFirst.within(PermissionMode::Workspace));
        assert!(!PermissionMode::AskFirst.within(PermissionMode::ReadOnly));
    }
}
