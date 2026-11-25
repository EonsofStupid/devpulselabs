//! Which AIs the operator trusts in which project (AH1b).
//!
//! Stored on this computer, beside the project registry, never inside the
//! project folder: a project folder can be cloned from anywhere, and a grant
//! that travelled with it would trust an AI nobody here approved. That is the
//! class behind the 2025-26 workspace-trust CVEs, where project files ran
//! before trust was given (chat plan §4.8.2 rule 7).
//!
//! No grant means no AI process starts with the project folder.

use crate::registry::Registry;
use crate::{disk, journal, ProjectId};
use serde::{Deserialize, Serialize};

/// What the operator let one AI do in one project. Everything starts off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    /// Read files in the project folder.
    pub read: bool,
    /// Change files in the project folder.
    pub change: bool,
    /// Run commands in the project folder.
    pub run: bool,
}

impl Grant {
    /// Whether the AI may start in the project at all. Changing or running
    /// without reading is not a grant anyone can use.
    pub const fn allows_start(self) -> bool {
        self.read
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    project: ProjectId,
    ai: String,
    grant: Grant,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    grants: Vec<Entry>,
}

/// An AI id as the catalog writes them: lowercase letters, digits and `-`.
fn ai_id(ai: &str) -> Result<(), String> {
    if ai.is_empty()
        || ai.len() > 64
        || !ai
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(format!("{ai:?} is not an AI id"));
    }
    Ok(())
}

impl Registry {
    fn trust_state(&self) -> Result<State, String> {
        self.verify()?;
        Ok(match journal::latest(&self.base().join("trust"))? {
            Some((_, value)) => serde_json::from_value(value).map_err(|error| error.to_string())?,
            None => State::default(),
        })
    }

    /// What the operator granted `ai` in `project`, if they answered.
    pub fn trust(&self, project: &ProjectId, ai: &str) -> Result<Option<Grant>, String> {
        ai_id(ai)?;
        Ok(self
            .trust_state()?
            .grants
            .into_iter()
            .find(|entry| &entry.project == project && entry.ai == ai)
            .map(|entry| entry.grant))
    }

    /// Record the operator's answer for `ai` in `project`, replacing any
    /// earlier one.
    pub fn set_trust(&self, project: &ProjectId, ai: &str, grant: Grant) -> Result<(), String> {
        ai_id(ai)?;
        self.verify()?;
        let _lock = disk::lock(self.base())?;
        let mut state = self.trust_state()?;
        state
            .grants
            .retain(|entry| !(&entry.project == project && entry.ai == ai));
        state.grants.push(Entry {
            project: project.clone(),
            ai: ai.to_owned(),
            grant,
        });
        let value = serde_json::to_value(state).map_err(|error| error.to_string())?;
        journal::append(&self.base().join("trust"), &value, None).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Project;

    fn scratch(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir()
            .join("dpl-trust-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn work(root: &std::path::Path, name: &str) -> std::path::PathBuf {
        let dir = root.join(name);
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn nothing_is_trusted_until_the_operator_answers() {
        let root = scratch("unanswered");
        let registry = Registry::open(&root.join("machine")).unwrap();
        let project =
            Project::create_new(&root.join("store"), &work(&root, "work"), "Lesson 12").unwrap();
        assert_eq!(registry.trust(project.id(), "codex").unwrap(), None);
        assert!(!Grant::default().allows_start(), "everything starts off");
    }

    #[test]
    fn a_grant_is_per_project_and_per_ai_and_the_last_answer_wins() {
        let root = scratch("per-project");
        let registry = Registry::open(&root.join("machine")).unwrap();
        let one = Project::create_new(&root.join("store"), &work(&root, "one"), "One").unwrap();
        let two = Project::create_new(&root.join("store"), &work(&root, "two"), "Two").unwrap();
        let read = Grant {
            read: true,
            ..Grant::default()
        };
        registry.set_trust(one.id(), "codex", read).unwrap();
        assert_eq!(registry.trust(one.id(), "codex").unwrap(), Some(read));
        assert_eq!(registry.trust(one.id(), "grok").unwrap(), None);
        assert_eq!(registry.trust(two.id(), "codex").unwrap(), None);
        let all = Grant {
            read: true,
            change: true,
            run: true,
        };
        registry.set_trust(one.id(), "codex", all).unwrap();
        assert_eq!(registry.trust(one.id(), "codex").unwrap(), Some(all));
        // Reopening reads the same answer back.
        let reopened = Registry::open_existing(&root.join("machine")).unwrap();
        assert_eq!(reopened.trust(one.id(), "codex").unwrap(), Some(all));
    }

    #[test]
    fn the_grant_never_lives_in_the_project_folder() {
        let root = scratch("not-in-project");
        let registry = Registry::open(&root.join("machine")).unwrap();
        let project =
            Project::create_new(&root.join("store"), &work(&root, "work"), "Work").unwrap();
        let grant = Grant {
            read: true,
            change: true,
            run: true,
        };
        registry.set_trust(project.id(), "grok", grant).unwrap();
        let mut stack = vec![root.join("work")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                let inside = path.strip_prefix(root.join("work")).unwrap();
                assert!(
                    !inside.to_string_lossy().contains("trust"),
                    "{}",
                    path.display()
                );
                if path.is_dir() {
                    stack.push(path);
                }
            }
        }
        assert!(root.join("machine").join("trust").exists());
    }

    #[test]
    fn a_malformed_ai_id_is_refused() {
        let root = scratch("ai-id");
        let registry = Registry::open(&root.join("machine")).unwrap();
        let project =
            Project::create_new(&root.join("store"), &work(&root, "work"), "Work").unwrap();
        for bad in ["", "Codex", "../x", "a b"] {
            assert!(registry.trust(project.id(), bad).is_err(), "{bad:?}");
        }
    }
}
