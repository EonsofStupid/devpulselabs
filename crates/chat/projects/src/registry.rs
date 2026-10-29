use crate::{disk, journal, label, Project, ProjectId, MAX_MARKER_BYTES, VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const MARKER: &str = "registry.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: ProjectId,
    pub label: String,
    pub workspace: PathBuf,
}

impl Binding {
    /// Open this exact identity, not whichever project happens to occupy its
    /// old path after a drive change.
    pub fn open(&self) -> Result<Project, String> {
        let project = Project::open(&self.workspace)?;
        if project.id() != &self.id {
            return Err(format!(
                "{} now holds a different project",
                self.workspace.display()
            ));
        }
        Ok(project)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
struct RegistryId(ProjectId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    v: u32,
    id: RegistryId,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    bindings: Vec<Binding>,
    active: Option<ProjectId>,
}

#[derive(Debug, Clone)]
pub struct Registry {
    base: PathBuf,
    marker: Marker,
}

impl Registry {
    /// Open/create only this explicit machine-local base. No environment or
    /// working-directory fallback is performed by the storage owner.
    pub fn open(base: &Path) -> Result<Self, String> {
        disk::path(base)?;
        let marker_file = base.join(MARKER);
        if fs::symlink_metadata(&marker_file).is_err() {
            disk::empty_or_absent(base)?;
            disk::create_directories(base)?;
            disk::create_json(
                &marker_file,
                &Marker {
                    v: VERSION,
                    id: RegistryId(ProjectId::mint()?),
                },
            )?;
        }
        let marker: Marker = disk::json(&marker_file, MAX_MARKER_BYTES)?;
        if marker.v != VERSION {
            return Err("This project registry format is not supported".into());
        }
        let registry = Self {
            base: base.to_path_buf(),
            marker,
        };
        registry.state()?;
        Ok(registry)
    }

    fn verify(&self) -> Result<(), String> {
        disk::existing_directory(&self.base)?;
        let marker: Marker = disk::json(&self.base.join(MARKER), MAX_MARKER_BYTES)?;
        if marker != self.marker {
            return Err(
                "The project registry changed; its previous bindings were not replaced".into(),
            );
        }
        Ok(())
    }

    fn state(&self) -> Result<State, String> {
        self.verify()?;
        let state: State = match journal::latest(&self.base.join("bindings"))? {
            Some((_, value)) => serde_json::from_value(value).map_err(|error| error.to_string())?,
            None => State::default(),
        };
        let mut seen = BTreeSet::new();
        for binding in &state.bindings {
            label(&binding.label)?;
            // Validate syntax without requiring disconnected projects to exist.
            if !binding.workspace.is_absolute()
                || binding
                    .workspace
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err("The saved project path is not absolute or contains '..'".into());
            }
            if !seen.insert(binding.id.clone()) {
                return Err("The project registry contains duplicate identities".into());
            }
        }
        if state
            .active
            .as_ref()
            .is_some_and(|active| !seen.contains(active))
        {
            return Err("The active project is absent from the registry".into());
        }
        Ok(state)
    }

    pub fn list(&self) -> Result<Vec<Binding>, String> {
        Ok(self.state()?.bindings)
    }

    pub fn active(&self) -> Result<Option<Binding>, String> {
        let state = self.state()?;
        Ok(state
            .bindings
            .into_iter()
            .find(|binding| Some(&binding.id) == state.active.as_ref()))
    }

    pub fn open_active(&self) -> Result<Option<Project>, String> {
        self.active()?.map(|binding| binding.open()).transpose()
    }

    /// Change only subsequent activation. Existing Project handles stay bound
    /// to their own identity, even when another project becomes active.
    pub fn activate(&self, project: &Project) -> Result<(), String> {
        self.verify()?;
        project.verify()?;
        let _lock = disk::lock(&self.base)?;
        let mut state = self.state()?;
        project.verify()?;
        let binding = Binding {
            id: project.id().clone(),
            label: project.label().to_owned(),
            workspace: project.workspace().to_path_buf(),
        };
        if let Some(existing) = state
            .bindings
            .iter_mut()
            .find(|existing| existing.id == binding.id)
        {
            if existing.workspace != binding.workspace
                && existing.open().is_ok()
                && fs::canonicalize(&existing.workspace).ok()
                    != fs::canonicalize(&binding.workspace).ok()
            {
                return Err("This project identity is still available at another folder; disconnect that copy before locating it elsewhere".into());
            }
            *existing = binding;
        } else {
            state.bindings.push(binding);
        }
        state.active = Some(project.id().clone());
        let value = serde_json::to_value(state).map_err(|error| error.to_string())?;
        self.verify()?;
        journal::append(&self.base.join("bindings"), &value, None).map(|_| ())
    }
}
