//! Explicit, UNQUALIFIED Codex storage candidates. Owning a valid candidate
//! proves filesystem binding, never vendor footprint completeness or support.
//! No credential files are inspected and no profile is automatically adopted.

use crate::{disk, Project, ProjectId, Registry, MAX_MARKER_BYTES, VERSION};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

const OWNER: &str = "owner.json";
const PROFILE: &str = "profile.json";
const PENDING: &str = "pending.json";
const LEASE: &str = "clients.lock";
const DIRECTORIES: [&str; 2] = ["sessions", "archived_sessions"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileGeneration(ProjectId);

impl ProfileGeneration {
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        ProjectId::parse(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum CandidateLayout {
    CodexSessionsAndArchivesV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    v: u32,
    registry: ProjectId,
    project: Option<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeOwner {
    v: u32,
    project: ProjectId,
    layout: CandidateLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    v: u32,
    registry: ProjectId,
    project: ProjectId,
    generation: ProfileGeneration,
    workspace: PathBuf,
    layout: CandidateLayout,
}

/// A local home generation with precisely two candidate conversation links.
/// This type cannot grant native-save, native-resume or standalone support.
#[derive(Debug, Clone)]
pub struct CodexProfileCandidate {
    registry: Registry,
    project: Project,
    pool: PathBuf,
    directory: PathBuf,
    home: PathBuf,
    marker: Marker,
}

/// Shared lifetime guard for one candidate. Keep it alive until the named
/// provider child has exited. Cloning an Arc of this guard shares that lifetime;
/// a fresh lease independently locks the same project-wide control file.
#[derive(Debug)]
pub struct CandidateLease {
    candidate: CodexProfileCandidate,
    _lock: File,
}

impl CandidateLease {
    pub fn home(&self) -> &Path {
        self.candidate.home()
    }
    pub fn workspace(&self) -> &Path {
        self.candidate.workspace()
    }
    pub fn project_id(&self) -> &ProjectId {
        self.candidate.project_id()
    }
    pub fn generation(&self) -> &ProfileGeneration {
        self.candidate.generation()
    }
    pub fn verify(&self) -> Result<(), String> {
        self.candidate.verify()
    }
}

impl CodexProfileCandidate {
    /// Explicit qualification/setup action only, never startup discovery.
    /// Creates a new immutable home; prior generations and links are retained.
    /// Windows link privilege failure preserves the incomplete generation and
    /// returns an error without substituting copies or enabling any support.
    pub fn create_new(registry: &Registry, project: &Project) -> Result<Self, String> {
        registry.verify()?;
        project.verify()?;
        validate_placement(registry, project)?;
        let _registry_lock = disk::lock(registry.base())?;
        registry.verify()?;
        let profiles = registry.base().join("profiles");
        ensure_owned(
            &profiles,
            &Owner {
                v: VERSION,
                registry: registry.identity().clone(),
                project: None,
            },
        )?;
        let pool = profiles.join(project.id().as_str());
        let created = ensure_owned(&pool, &owner(registry, project))?;
        if created {
            disk::create(&pool.join(LEASE), b"")?;
        }
        let _exclusive = lock_clients(&pool, false)?;
        project.verify()?;
        verify_owner(&pool, &owner(registry, project))?;

        let native = native_root(project);
        disk::create_directories(native.parent().ok_or("Native archive has no parent")?)?;
        let native_owner = NativeOwner {
            v: VERSION,
            project: project.id().clone(),
            layout: CandidateLayout::CodexSessionsAndArchivesV1,
        };
        let created_native = ensure_owned(&native, &native_owner)?;
        for name in DIRECTORIES {
            let target = native.join(name);
            if created_native {
                disk::create_directories(&target)?;
            }
            disk::existing_directory(&target)?;
        }
        let generation = ProfileGeneration(ProjectId::mint()?);
        let directory = pool.join(generation.as_str());
        disk::path(&directory)?;
        fs::create_dir(&directory)
            .map_err(|error| format!("Cannot exclusively create profile generation: {error}"))?;
        let marker = Marker {
            v: VERSION,
            registry: registry.identity().clone(),
            project: project.id().clone(),
            generation,
            workspace: project.workspace().to_path_buf(),
            layout: CandidateLayout::CodexSessionsAndArchivesV1,
        };
        disk::create_json(&directory.join(PENDING), &marker)?;
        let home = directory.join("codex");
        disk::create_directories(&home)?;
        for name in DIRECTORIES {
            create_link(&native.join(name), &home.join(name))?;
        }
        project.verify()?;
        registry.verify()?;
        disk::create_json(&directory.join(PROFILE), &marker)?;
        let candidate = Self {
            registry: registry.clone(),
            project: project.clone(),
            pool,
            directory,
            home,
            marker,
        };
        candidate.verify()?;
        Ok(candidate)
    }

    pub fn open(
        registry: &Registry,
        project: &Project,
        generation: &ProfileGeneration,
    ) -> Result<Self, String> {
        registry.verify()?;
        project.verify()?;
        let pool = registry.base().join("profiles").join(project.id().as_str());
        let directory = pool.join(generation.as_str());
        let marker: Marker = disk::json(&directory.join(PROFILE), MAX_MARKER_BYTES)?;
        if marker.v != VERSION
            || &marker.generation != generation
            || &marker.project != project.id()
            || &marker.registry != registry.identity()
            || marker.workspace != project.workspace()
        {
            return Err(
                "The candidate profile does not match this project, registry or location".into(),
            );
        }
        let candidate = Self {
            registry: registry.clone(),
            project: project.clone(),
            home: directory.join("codex"),
            directory,
            pool,
            marker,
        };
        candidate.verify()?;
        Ok(candidate)
    }

    pub fn generation(&self) -> &ProfileGeneration {
        &self.marker.generation
    }
    pub fn project_id(&self) -> &ProjectId {
        &self.marker.project
    }
    pub fn workspace(&self) -> &Path {
        self.project.workspace()
    }
    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn verify(&self) -> Result<(), String> {
        self.registry.verify()?;
        self.project.verify()?;
        validate_placement(&self.registry, &self.project)?;
        verify_owner(
            &self.registry.base().join("profiles"),
            &Owner {
                v: VERSION,
                registry: self.registry.identity().clone(),
                project: None,
            },
        )?;
        verify_owner(&self.pool, &owner(&self.registry, &self.project))?;
        for name in [PROFILE, PENDING] {
            let marker: Marker = disk::json(&self.directory.join(name), MAX_MARKER_BYTES)?;
            if marker != self.marker {
                return Err("The candidate profile identity has changed".into());
            }
        }
        disk::existing_directory(&self.home)?;
        verify_owner(
            &native_root(&self.project),
            &NativeOwner {
                v: VERSION,
                project: self.project_id().clone(),
                layout: CandidateLayout::CodexSessionsAndArchivesV1,
            },
        )?;
        for name in DIRECTORIES {
            verify_link(
                &self.home.join(name),
                &native_root(&self.project).join(name),
            )?;
        }
        Ok(())
    }

    pub fn lease(&self) -> Result<CandidateLease, String> {
        self.verify()?;
        let lock = lock_clients(&self.pool, true)?;
        self.verify()?;
        Ok(CandidateLease {
            candidate: self.clone(),
            _lock: lock,
        })
    }
}

fn owner(registry: &Registry, project: &Project) -> Owner {
    Owner {
        v: VERSION,
        registry: registry.identity().clone(),
        project: Some(project.id().clone()),
    }
}

fn native_root(project: &Project) -> PathBuf {
    project.archive_dir().join("providers/codex")
}

fn validate_placement(registry: &Registry, project: &Project) -> Result<(), String> {
    let profiles = fs::canonicalize(registry.base())
        .map_err(|error| error.to_string())?
        .join("profiles");
    let workspace = fs::canonicalize(project.workspace()).map_err(|error| error.to_string())?;
    if profiles.starts_with(&workspace) || workspace.starts_with(&profiles) {
        return Err("Machine-local provider homes must be outside the portable workspace, and the workspace must not be inside provider homes".into());
    }
    Ok(())
}

fn ensure_owned<T: Serialize + serde::de::DeserializeOwned + PartialEq>(
    directory: &Path,
    expected: &T,
) -> Result<bool, String> {
    disk::path(directory)?;
    match fs::symlink_metadata(directory.join(OWNER)) {
        Ok(_) => {
            verify_owner(directory, expected)?;
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            disk::empty_or_absent(directory)?;
            disk::create_directories(directory)?;
            disk::create_json(&directory.join(OWNER), expected)?;
            Ok(true)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn verify_owner<T: Serialize + serde::de::DeserializeOwned + PartialEq>(
    directory: &Path,
    expected: &T,
) -> Result<(), String> {
    disk::existing_directory(directory)?;
    let found: T = disk::json(&directory.join(OWNER), MAX_MARKER_BYTES)?;
    if &found != expected {
        return Err("The profile storage owner does not match this binding".into());
    }
    Ok(())
}

fn lock_clients(pool: &Path, shared: bool) -> Result<File, String> {
    let path = pool.join(LEASE);
    disk::path(&path)?;
    if !fs::symlink_metadata(&path)
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("The profile lease must be an existing regular file".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    let result = if shared {
        file.try_lock_shared()
    } else {
        file.try_lock()
    };
    result.map_err(|_| {
        "The project profile is busy; close its clients before setting up or relocating it"
            .to_owned()
    })?;
    Ok(file)
}

fn create_link(target: &Path, link: &Path) -> Result<(), String> {
    disk::existing_directory(target)?;
    disk::path(link.parent().ok_or("Profile link has no parent")?)?;
    match fs::symlink_metadata(link) {
        Ok(_) => {
            return Err("The candidate link location already exists; nothing was replaced".into())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_dir(target, link);
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(target, link);
    #[cfg(not(any(windows, unix)))]
    let result: std::io::Result<()> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "directory links are unsupported",
    ));
    result.map_err(|error| format!("Candidate directory link could not be created; check platform link privileges. Partial owned setup is retained: {error}"))?;
    verify_link(link, target)
}

fn verify_link(link: &Path, target: &Path) -> Result<(), String> {
    disk::path(link.parent().ok_or("Profile link has no parent")?)?;
    disk::existing_directory(target)?;
    let meta = fs::symlink_metadata(link).map_err(|error| error.to_string())?;
    if !meta.file_type().is_symlink() {
        return Err("The candidate conversation path is no longer the owned symbolic link".into());
    }
    let destination = fs::read_link(link).map_err(|error| error.to_string())?;
    disk::existing_directory(&destination)?;
    if destination != target
        || fs::canonicalize(link).map_err(|error| error.to_string())?
            != fs::canonicalize(target).map_err(|error| error.to_string())?
    {
        return Err("The candidate conversation link points to a different location".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
