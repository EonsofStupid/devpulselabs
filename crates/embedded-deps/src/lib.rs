//! Verified, app-relative resolution for DevPULSE private subprocess tools.
//!
//! Production callers provide a typed [`DependencyId`] and receive an
//! absolute path only after the target, containment, size, and SHA-256 gates
//! pass. This crate never searches the working directory, `PATH`, provider
//! homes, or OS installation roots.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const MANIFEST_SCHEMA: u32 = 1;
pub const MANIFEST_FILE: &str = "manifest.json";
pub const EMBEDDED_DIRECTORY: &str = "embedded-deps";

pub const CODEX: DependencyId = DependencyId("codex");
pub const GROK: DependencyId = DependencyId("grok");
pub const FFMPEG: DependencyId = DependencyId("ffmpeg");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DependencyId(pub &'static str);

impl fmt::Display for DependencyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub target: String,
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestDependency {
    pub id: String,
    pub version: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    id: String,
    version: String,
    path: PathBuf,
    bytes: u64,
    sha256: String,
}

impl ResolvedDependency {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

#[derive(Debug)]
pub struct Resolver {
    root: PathBuf,
    manifest: Manifest,
    verified: Mutex<HashMap<String, ResolvedDependency>>,
}

impl Resolver {
    /// Opens the bundle beside the running executable.
    pub fn for_current_process() -> Result<Self, ResolveError> {
        let executable = std::env::current_exe().map_err(|error| {
            ResolveError::ApplicationRoot(format!(
                "the running executable could not be located: {}",
                public_io_reason(&error)
            ))
        })?;
        let release_root = executable.parent().ok_or_else(|| {
            ResolveError::ApplicationRoot(
                "the running executable has no application directory".to_owned(),
            )
        })?;
        Self::from_release_root(release_root)
    }

    /// Opens a staged release root. This is also the deterministic test seam.
    pub fn from_release_root(release_root: impl AsRef<Path>) -> Result<Self, ResolveError> {
        Self::from_embedded_root(release_root.as_ref().join(EMBEDDED_DIRECTORY))
    }

    pub fn from_embedded_root(root: impl AsRef<Path>) -> Result<Self, ResolveError> {
        let requested_root = root.as_ref();
        let root = fs::canonicalize(requested_root).map_err(|error| {
            ResolveError::ManifestUnavailable(format!(
                "{}: {}",
                requested_root.join(MANIFEST_FILE).display(),
                public_io_reason(&error)
            ))
        })?;
        if !root.is_dir() {
            return Err(ResolveError::ManifestUnavailable(format!(
                "{} is not a directory",
                root.display()
            )));
        }
        let manifest_path = root.join(MANIFEST_FILE);
        let bytes = fs::read(&manifest_path).map_err(|error| {
            ResolveError::ManifestUnavailable(format!(
                "{}: {}",
                manifest_path.display(),
                public_io_reason(&error)
            ))
        })?;
        let manifest: Manifest = serde_json::from_slice(&bytes)
            .map_err(|error| ResolveError::ManifestInvalid(error.to_string()))?;
        validate_manifest(&manifest)?;
        Ok(Self {
            root,
            manifest,
            verified: Mutex::new(HashMap::new()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn resolve(&self, id: DependencyId) -> Result<ResolvedDependency, ResolveError> {
        if let Some(resolved) = lock(&self.verified).get(id.0).cloned() {
            return Ok(resolved);
        }
        let entry = self
            .manifest
            .dependencies
            .iter()
            .find(|entry| entry.id == id.0)
            .ok_or_else(|| ResolveError::MissingDependency(id.0.to_owned()))?;
        let relative = validate_relative_path(&entry.path)?;
        let candidate = self.root.join(relative);
        let path =
            fs::canonicalize(&candidate).map_err(|error| ResolveError::DependencyUnavailable {
                id: entry.id.clone(),
                reason: public_io_reason(&error),
            })?;
        if !path.starts_with(&self.root) {
            return Err(ResolveError::EscapedRoot {
                id: entry.id.clone(),
            });
        }
        let metadata = path
            .metadata()
            .map_err(|error| ResolveError::DependencyUnavailable {
                id: entry.id.clone(),
                reason: public_io_reason(&error),
            })?;
        if !metadata.is_file() {
            return Err(ResolveError::DependencyUnavailable {
                id: entry.id.clone(),
                reason: "the manifest path is not a file".to_owned(),
            });
        }
        if metadata.len() != entry.bytes {
            return Err(ResolveError::LengthMismatch {
                id: entry.id.clone(),
                expected: entry.bytes,
                actual: metadata.len(),
            });
        }
        let actual = sha256_file(&path).map_err(|error| ResolveError::DependencyUnavailable {
            id: entry.id.clone(),
            reason: public_io_reason(&error),
        })?;
        if actual != entry.sha256 {
            return Err(ResolveError::DigestMismatch {
                id: entry.id.clone(),
                expected: entry.sha256.clone(),
                actual,
            });
        }
        ensure_executable(&entry.id, &metadata)?;
        let resolved = ResolvedDependency {
            id: entry.id.clone(),
            version: entry.version.clone(),
            path,
            bytes: entry.bytes,
            sha256: entry.sha256.clone(),
        };
        lock(&self.verified).insert(entry.id.clone(), resolved.clone());
        Ok(resolved)
    }
}

/// Resolves from one process-wide manifest/cache. Every private binary is
/// hashed at most once per DevPULSE process.
pub fn resolve_current(id: DependencyId) -> Result<ResolvedDependency, ResolveError> {
    static CURRENT: OnceLock<Result<Resolver, ResolveError>> = OnceLock::new();
    CURRENT
        .get_or_init(Resolver::for_current_process)
        .as_ref()
        .map_err(Clone::clone)?
        .resolve(id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    ApplicationRoot(String),
    ManifestUnavailable(String),
    ManifestInvalid(String),
    SchemaUnsupported(u32),
    TargetMismatch {
        expected: String,
        actual: String,
    },
    InvalidIdentity {
        field: &'static str,
        value: String,
    },
    InvalidPath(String),
    DuplicateDependency(String),
    MissingDependency(String),
    DependencyUnavailable {
        id: String,
        reason: String,
    },
    EscapedRoot {
        id: String,
    },
    LengthMismatch {
        id: String,
        expected: u64,
        actual: u64,
    },
    DigestMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    NotExecutable(String),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApplicationRoot(reason) => {
                write!(formatter, "DevPULSE application root is unavailable: {reason}")
            }
            Self::ManifestUnavailable(reason) => {
                write!(formatter, "embedded dependency manifest is unavailable: {reason}")
            }
            Self::ManifestInvalid(reason) => {
                write!(formatter, "embedded dependency manifest is invalid: {reason}")
            }
            Self::SchemaUnsupported(schema) => {
                write!(formatter, "embedded dependency schema {schema} is unsupported")
            }
            Self::TargetMismatch { expected, actual } => write!(
                formatter,
                "embedded dependency target is {actual}; this build requires {expected}"
            ),
            Self::InvalidIdentity { field, value } => {
                write!(formatter, "embedded dependency {field} `{value}` is invalid")
            }
            Self::InvalidPath(path) => {
                write!(formatter, "embedded dependency path `{path}` is invalid")
            }
            Self::DuplicateDependency(id) => {
                write!(formatter, "embedded dependency `{id}` is registered more than once")
            }
            Self::MissingDependency(id) => {
                write!(formatter, "embedded dependency `{id}` is not in this release")
            }
            Self::DependencyUnavailable { id, reason } => {
                write!(formatter, "embedded dependency `{id}` is unavailable: {reason}")
            }
            Self::EscapedRoot { id } => {
                write!(formatter, "embedded dependency `{id}` escapes the application bundle")
            }
            Self::LengthMismatch {
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "embedded dependency `{id}` has {actual} bytes; expected {expected}"
            ),
            Self::DigestMismatch {
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "embedded dependency `{id}` failed SHA-256 verification: expected {expected}, got {actual}"
            ),
            Self::NotExecutable(id) => {
                write!(formatter, "embedded dependency `{id}` is not executable")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

pub fn current_target() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        "aarch64-pc-windows-msvc"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-gnu"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "aarch64-unknown-linux-gnu"
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64")
    )))]
    {
        "unsupported"
    }
}

pub fn sha256_file(path: impl AsRef<Path>) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn validate_manifest(manifest: &Manifest) -> Result<(), ResolveError> {
    if manifest.schema != MANIFEST_SCHEMA {
        return Err(ResolveError::SchemaUnsupported(manifest.schema));
    }
    let expected = current_target();
    if manifest.target != expected {
        return Err(ResolveError::TargetMismatch {
            expected: expected.to_owned(),
            actual: manifest.target.clone(),
        });
    }
    let mut identities = HashSet::with_capacity(manifest.dependencies.len());
    for entry in &manifest.dependencies {
        validate_segment("id", &entry.id)?;
        validate_segment("version", &entry.version)?;
        validate_relative_path(&entry.path)?;
        validate_digest(&entry.sha256)?;
        if !identities.insert(entry.id.clone()) {
            return Err(ResolveError::DuplicateDependency(entry.id.clone()));
        }
    }
    Ok(())
}

fn validate_segment(field: &'static str, value: &str) -> Result<(), ResolveError> {
    let valid = !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-._+".contains(character));
    if valid {
        Ok(())
    } else {
        Err(ResolveError::InvalidIdentity {
            field,
            value: value.to_owned(),
        })
    }
}

fn validate_relative_path(path: &str) -> Result<PathBuf, ResolveError> {
    let path = Path::new(path);
    let valid = !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if valid {
        Ok(path.to_owned())
    } else {
        Err(ResolveError::InvalidPath(path.display().to_string()))
    }
}

fn validate_digest(digest: &str) -> Result<(), ResolveError> {
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ResolveError::ManifestInvalid(
            "SHA-256 values must be 64 lowercase hexadecimal characters".to_owned(),
        ))
    }
}

#[cfg(unix)]
fn ensure_executable(id: &str, metadata: &fs::Metadata) -> Result<(), ResolveError> {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o111 == 0 {
        Err(ResolveError::NotExecutable(id.to_owned()))
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn ensure_executable(_id: &str, _metadata: &fs::Metadata) -> Result<(), ResolveError> {
    Ok(())
}

fn public_io_reason(error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "required file was not found".to_owned(),
        io::ErrorKind::PermissionDenied => "permission was denied".to_owned(),
        io::ErrorKind::TimedOut => "the operation timed out".to_owned(),
        kind => format!("{kind:?}"),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn release_root(name: &str) -> PathBuf {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "dpl-embedded-deps-{name}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(root.join(EMBEDDED_DIRECTORY)).expect("embedded root");
        root
    }

    fn write_bundle(root: &Path, id: &str, version: &str, relative: &str, payload: &[u8]) {
        let embedded = root.join(EMBEDDED_DIRECTORY);
        let path = embedded.join(relative);
        fs::create_dir_all(path.parent().expect("dependency parent")).expect("dependency parent");
        fs::write(&path, payload).expect("dependency");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("executable permission");
        }
        let manifest = Manifest {
            schema: MANIFEST_SCHEMA,
            target: current_target().to_owned(),
            dependencies: vec![ManifestDependency {
                id: id.to_owned(),
                version: version.to_owned(),
                path: relative.to_owned(),
                bytes: payload.len() as u64,
                sha256: sha256_file(&path).expect("digest"),
            }],
        };
        fs::write(
            embedded.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).expect("manifest"),
        )
        .expect("manifest");
    }

    #[test]
    fn resolves_only_verified_app_relative_files() {
        let root = release_root("resolve");
        let relative = if cfg!(windows) {
            "grok/0.2.112/grok.exe"
        } else {
            "grok/0.2.112/grok"
        };
        write_bundle(&root, "grok", "0.2.112", relative, b"private tool");
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        let dependency = resolver.resolve(GROK).expect("grok");
        assert_eq!(dependency.version(), "0.2.112");
        assert!(dependency.path().is_absolute());
        assert!(dependency.path().starts_with(resolver.root()));
    }

    #[test]
    fn target_and_duplicate_ids_fail_closed() {
        let root = release_root("manifest");
        let embedded = root.join(EMBEDDED_DIRECTORY);
        let entry = ManifestDependency {
            id: "grok".to_owned(),
            version: "0.2.112".to_owned(),
            path: "grok/0.2.112/grok".to_owned(),
            bytes: 0,
            sha256: "0".repeat(64),
        };
        let wrong_target = Manifest {
            schema: MANIFEST_SCHEMA,
            target: "another-target".to_owned(),
            dependencies: vec![entry.clone()],
        };
        fs::write(
            embedded.join(MANIFEST_FILE),
            serde_json::to_vec(&wrong_target).expect("manifest"),
        )
        .expect("manifest");
        assert!(matches!(
            Resolver::from_release_root(&root),
            Err(ResolveError::TargetMismatch { .. })
        ));

        let duplicate = Manifest {
            schema: MANIFEST_SCHEMA,
            target: current_target().to_owned(),
            dependencies: vec![entry.clone(), entry],
        };
        fs::write(
            embedded.join(MANIFEST_FILE),
            serde_json::to_vec(&duplicate).expect("manifest"),
        )
        .expect("manifest");
        assert!(matches!(
            Resolver::from_release_root(&root),
            Err(ResolveError::DuplicateDependency(id)) if id == "grok"
        ));
    }

    #[test]
    fn traversal_and_corruption_fail_closed() {
        let root = release_root("corruption");
        let embedded = root.join(EMBEDDED_DIRECTORY);
        let invalid = Manifest {
            schema: MANIFEST_SCHEMA,
            target: current_target().to_owned(),
            dependencies: vec![ManifestDependency {
                id: "grok".to_owned(),
                version: "0.2.112".to_owned(),
                path: "../grok".to_owned(),
                bytes: 1,
                sha256: "0".repeat(64),
            }],
        };
        fs::write(
            embedded.join(MANIFEST_FILE),
            serde_json::to_vec(&invalid).expect("manifest"),
        )
        .expect("manifest");
        assert!(matches!(
            Resolver::from_release_root(&root),
            Err(ResolveError::InvalidPath(_))
        ));

        write_bundle(&root, "grok", "0.2.112", "grok/0.2.112/grok", b"expected");
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        fs::write(
            root.join(EMBEDDED_DIRECTORY).join("grok/0.2.112/grok"),
            b"tampered",
        )
        .expect("tamper");
        assert!(matches!(
            resolver.resolve(GROK),
            Err(ResolveError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn missing_dependency_is_scoped() {
        let root = release_root("missing");
        write_bundle(&root, "grok", "0.2.112", "grok/0.2.112/grok", b"grok");
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        assert_eq!(
            resolver.resolve(CODEX),
            Err(ResolveError::MissingDependency("codex".to_owned()))
        );
    }
}
