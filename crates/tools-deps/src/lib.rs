//! Verified, app-relative resolution for the DevPULSE tool depot.
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
use std::sync::{Arc, Mutex};

pub const MANIFEST_SCHEMA: u32 = 2;
pub const MANIFEST_FILE: &str = "manifest.json";
pub const TOOLS_DIRECTORY: &str = "dptools-deps";

pub const CODEX: DependencyId = DependencyId::new("codex", DependencyFamily::Provider);
pub const GROK: DependencyId = DependencyId::new("grok", DependencyFamily::Provider);
pub const FFMPEG: DependencyId = DependencyId::new("ffmpeg", DependencyFamily::Media);
pub const CHROMIUM: DependencyId = DependencyId::new("chromium", DependencyFamily::Browser);
/// Poppler's page rasterizer (W11 PDF slides; T3 carousel) — staged as a
/// TREE (the exe needs its sibling DLLs), entrypoint `Library/bin/pdftoppm.exe`.
pub const PDFTOPPM: DependencyId = DependencyId::new("pdftoppm", DependencyFamily::Media);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DependencyId {
    id: &'static str,
    family: DependencyFamily,
}

impl DependencyId {
    pub const fn new(id: &'static str, family: DependencyFamily) -> Self {
        Self { id, family }
    }

    pub const fn as_str(self) -> &'static str {
        self.id
    }

    pub const fn family(self) -> DependencyFamily {
        self.family
    }
}

impl fmt::Display for DependencyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyFamily {
    Provider,
    Browser,
    Media,
}

impl DependencyFamily {
    pub const fn directory(self) -> &'static str {
        match self {
            Self::Provider => "providers",
            Self::Browser => "browsers",
            Self::Media => "media",
        }
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
    pub family: DependencyFamily,
    pub version: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<ManifestTree>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestTree {
    pub root: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    id: String,
    family: DependencyFamily,
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

    pub const fn family(&self) -> DependencyFamily {
        self.family
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
        Self::from_tools_root(release_root.as_ref().join(TOOLS_DIRECTORY))
    }

    pub fn from_tools_root(root: impl AsRef<Path>) -> Result<Self, ResolveError> {
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
        Self::with_manifest(root, manifest)
    }

    /// A resolver over `root` that trusts `manifest` instead of reading one,
    /// so [`install_tool`] can verify a manifest BEFORE it is written.
    fn with_manifest(root: PathBuf, manifest: Manifest) -> Result<Self, ResolveError> {
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
        if let Some(resolved) = lock(&self.verified).get(id.as_str()).cloned() {
            return Ok(resolved);
        }
        let entry = self
            .manifest
            .dependencies
            .iter()
            .find(|entry| entry.id == id.as_str())
            .ok_or_else(|| ResolveError::MissingDependency(id.as_str().to_owned()))?;
        if entry.family != id.family() {
            return Err(ResolveError::FamilyMismatch {
                id: entry.id.clone(),
                expected: id.family(),
                actual: entry.family,
            });
        }
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
        if let Some(tree) = &entry.tree {
            let tree_relative = validate_relative_path(&tree.root)?;
            let tree_path = fs::canonicalize(self.root.join(tree_relative)).map_err(|error| {
                ResolveError::DependencyUnavailable {
                    id: entry.id.clone(),
                    reason: public_io_reason(&error),
                }
            })?;
            if !tree_path.starts_with(&self.root) || !path.starts_with(&tree_path) {
                return Err(ResolveError::EscapedRoot {
                    id: entry.id.clone(),
                });
            }
            if !tree_path.is_dir() {
                return Err(ResolveError::DependencyUnavailable {
                    id: entry.id.clone(),
                    reason: "the manifest tree root is not a directory".to_owned(),
                });
            }
            let (actual_bytes, actual_digest) =
                sha256_tree(&tree_path).map_err(|error| ResolveError::DependencyUnavailable {
                    id: entry.id.clone(),
                    reason: public_io_reason(&error),
                })?;
            if actual_bytes != tree.bytes {
                return Err(ResolveError::TreeLengthMismatch {
                    id: entry.id.clone(),
                    expected: tree.bytes,
                    actual: actual_bytes,
                });
            }
            if actual_digest != tree.sha256 {
                return Err(ResolveError::TreeDigestMismatch {
                    id: entry.id.clone(),
                    expected: tree.sha256.clone(),
                    actual: actual_digest,
                });
            }
        }
        ensure_executable(&entry.id, &metadata)?;
        let resolved = ResolvedDependency {
            id: entry.id.clone(),
            family: entry.family,
            version: entry.version.clone(),
            path,
            bytes: entry.bytes,
            sha256: entry.sha256.clone(),
        };
        lock(&self.verified).insert(entry.id.clone(), resolved.clone());
        Ok(resolved)
    }
}

/// The process-wide resolver over the bundle beside the running executable.
/// `None` until first use, and again after [`reload_current`].
static CURRENT: Mutex<Option<Arc<Result<Resolver, ResolveError>>>> = Mutex::new(None);

/// Resolves from one process-wide manifest/cache. Every private binary is
/// hashed at most once per DevPULSE process (per manifest; see
/// [`reload_current`]).
pub fn resolve_current(id: DependencyId) -> Result<ResolvedDependency, ResolveError> {
    let current = {
        let mut slot = lock(&CURRENT);
        Arc::clone(slot.get_or_insert_with(|| Arc::new(Resolver::for_current_process())))
    };
    current.as_ref().as_ref().map_err(Clone::clone)?.resolve(id)
}

/// Forgets the process-wide resolver, so the next [`resolve_current`] reads
/// the manifest again. Called after [`install_tool`] rewrites the manifest
/// beside the running executable (an in-app update).
pub fn reload_current() {
    *lock(&CURRENT) = None;
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
    FamilyMismatch {
        id: String,
        expected: DependencyFamily,
        actual: DependencyFamily,
    },
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
    TreeLengthMismatch {
        id: String,
        expected: u64,
        actual: u64,
    },
    TreeDigestMismatch {
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
                write!(formatter, "DevPULSE tool manifest is unavailable: {reason}")
            }
            Self::ManifestInvalid(reason) => {
                write!(formatter, "DevPULSE tool manifest is invalid: {reason}")
            }
            Self::SchemaUnsupported(schema) => {
                write!(formatter, "DevPULSE tool schema {schema} is unsupported")
            }
            Self::TargetMismatch { expected, actual } => write!(
                formatter,
                "DevPULSE tool target is {actual}; this build requires {expected}"
            ),
            Self::InvalidIdentity { field, value } => {
                write!(formatter, "DevPULSE tool {field} `{value}` is invalid")
            }
            Self::InvalidPath(path) => {
                write!(formatter, "DevPULSE tool path `{path}` is invalid")
            }
            Self::DuplicateDependency(id) => {
                write!(formatter, "DevPULSE tool `{id}` is registered more than once")
            }
            Self::MissingDependency(id) => {
                write!(formatter, "DevPULSE tool `{id}` is not in this release")
            }
            Self::FamilyMismatch {
                id,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "DevPULSE tool `{id}` is in {actual:?}; expected {expected:?}"
                )
            }
            Self::DependencyUnavailable { id, reason } => {
                write!(formatter, "DevPULSE tool `{id}` is unavailable: {reason}")
            }
            Self::EscapedRoot { id } => {
                write!(formatter, "DevPULSE tool `{id}` escapes the application bundle")
            }
            Self::LengthMismatch {
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "DevPULSE tool `{id}` has {actual} bytes; expected {expected}"
            ),
            Self::DigestMismatch {
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "DevPULSE tool `{id}` failed SHA-256 verification: expected {expected}, got {actual}"
            ),
            Self::TreeLengthMismatch {
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "DevPULSE tool tree `{id}` has {actual} bytes; expected {expected}"
            ),
            Self::TreeDigestMismatch {
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "DevPULSE tool tree `{id}` failed SHA-256 verification: expected {expected}, got {actual}"
            ),
            Self::NotExecutable(id) => {
                write!(formatter, "DevPULSE tool `{id}` is not executable")
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

/// Hashes a complete portable tool tree deterministically.
///
/// The digest covers each normalized relative path, byte length, and file
/// payload in lexical path order. Symlinks are rejected so a staged browser
/// cannot redirect resolution outside its verified root.
pub fn sha256_tree(root: impl AsRef<Path>) -> io::Result<(u64, String)> {
    let root = root.as_ref();
    let mut files = Vec::new();
    collect_tree_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut total = 0_u64;
    let mut digest = Sha256::new();
    for (relative, path) in files {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tool trees cannot contain symlinks",
            ));
        }
        let bytes = metadata.len();
        total = total.saturating_add(bytes);
        digest.update(relative.as_bytes());
        digest.update([0]);
        digest.update(bytes.to_le_bytes());
        let mut file = fs::File::open(path)?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
    }
    Ok((
        total,
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}

fn collect_tree_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tool trees cannot contain symlinks",
            ));
        }
        if file_type.is_dir() {
            collect_tree_files(root, &path, files)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "tree path escaped root"))?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, path));
        }
    }
    Ok(())
}

// ── Installing into a depot ─────────────────────────────────────────────────
//
// The ONE place a tool's bytes enter a depot: `cargo xtask stage-tool-deps`
// builds a new bundle with `place_tool` + `write_manifest`, and an in-place
// update (`install_tool`) swaps one tool in an existing bundle. Both hash the
// COPIED bytes, which are what the resolver will verify. This crate never
// spawns: whoever installs proves the tool first (signature, version,
// protocol) and only then hands it here.

/// One tool to place into a depot.
#[derive(Debug, Clone, Copy)]
pub struct ToolSource<'a> {
    pub id: DependencyId,
    pub version: &'a str,
    /// A single executable, or a directory shipped whole (a tree).
    pub path: &'a Path,
    /// The executable inside a directory source; `None` for a single file.
    pub entrypoint: Option<&'a Path>,
}

/// Copies `source` to `<tools_root>/<family>/<id>/<version>/` (replacing a
/// folder of that version left by an earlier attempt) and returns its
/// manifest entry.
pub fn place_tool(
    tools_root: &Path,
    source: &ToolSource<'_>,
) -> Result<ManifestDependency, String> {
    let id = source.id.as_str();
    validate_segment("version", source.version).map_err(|error| error.to_string())?;
    let family_root = Path::new(source.id.family().directory())
        .join(id)
        .join(source.version);
    let version_root = tools_root.join(&family_root);
    if version_root.exists() {
        fs::remove_dir_all(&version_root)
            .map_err(|error| format!("{}: {error}", version_root.display()))?;
    }
    let (relative, tree) = match (
        source.path.is_file(),
        source.path.is_dir(),
        source.entrypoint,
    ) {
        (true, false, None) => {
            let relative = family_root.join(format!("{id}{}", std::env::consts::EXE_SUFFIX));
            fs::create_dir_all(&version_root)
                .map_err(|error| format!("{}: {error}", version_root.display()))?;
            let destination = tools_root.join(&relative);
            fs::copy(source.path, &destination).map_err(|error| {
                format!(
                    "copy {} -> {}: {error}",
                    source.path.display(),
                    destination.display()
                )
            })?;
            (relative, None)
        }
        (false, true, Some(entrypoint)) => {
            let entrypoint = validate_relative_path(&entrypoint.to_string_lossy())
                .map_err(|error| error.to_string())?;
            copy_tree(source.path, &version_root)?;
            let (bytes, sha256) = sha256_tree(&version_root)
                .map_err(|error| format!("{}: {error}", version_root.display()))?;
            (
                family_root.join(entrypoint),
                Some(ManifestTree {
                    root: slashed(&family_root),
                    bytes,
                    sha256,
                }),
            )
        }
        (false, true, None) => {
            return Err(format!(
                "{} is a directory, so it needs an entrypoint",
                source.path.display()
            ))
        }
        (true, false, Some(_)) => {
            return Err(format!(
                "{} is a single file, so it takes no entrypoint",
                source.path.display()
            ))
        }
        _ => {
            return Err(format!(
                "{} is not a file or directory",
                source.path.display()
            ))
        }
    };
    let destination = tools_root.join(&relative);
    let metadata = destination
        .metadata()
        .map_err(|error| format!("{}: {error}", destination.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a file", destination.display()));
    }
    let sha256 =
        sha256_file(&destination).map_err(|error| format!("{}: {error}", destination.display()))?;
    Ok(ManifestDependency {
        id: id.to_owned(),
        family: source.id.family(),
        version: source.version.to_owned(),
        path: slashed(&relative),
        bytes: metadata.len(),
        sha256,
        tree,
    })
}

/// Writes `manifest` into `tools_root` atomically: a temporary file, then a
/// rename over the old one, so a crash never leaves half a manifest.
pub fn write_manifest(tools_root: &Path, manifest: &Manifest) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|error| format!("serialize manifest: {error}"))?;
    let path = tools_root.join(MANIFEST_FILE);
    let temporary = tools_root.join(format!("{MANIFEST_FILE}.tmp"));
    fs::write(&temporary, bytes).map_err(|error| format!("{}: {error}", temporary.display()))?;
    fs::rename(&temporary, &path).map_err(|error| format!("{}: {error}", path.display()))
}

/// What an in-place install did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub entry: ManifestDependency,
    /// The version this install replaced, when the tool was already there.
    pub replaced: Option<String>,
    /// An older version folder that could not be deleted (a program was
    /// still running from it). The next install of this tool removes it.
    pub left_behind: Option<PathBuf>,
}

/// Installs one tool into an EXISTING depot, in place.
///
/// 1. The tool is placed beside the current version.
/// 2. The new manifest is verified through the production resolver while it
///    is still in memory; on any refusal the depot is unchanged.
/// 3. The manifest is rewritten atomically, so the new version is current.
/// 4. Every other version of the tool is deleted: one baseline, never two
///    versions side by side.
pub fn install_tool(tools_root: &Path, source: &ToolSource<'_>) -> Result<Installed, String> {
    let root = fs::canonicalize(tools_root)
        .map_err(|error| format!("{}: {error}", tools_root.display()))?;
    let manifest_path = root.join(MANIFEST_FILE);
    let bytes = fs::read(&manifest_path)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let current: Manifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    validate_manifest(&current).map_err(|error| error.to_string())?;

    let entry = place_tool(&root, source)?;
    let replaced = current
        .dependencies
        .iter()
        .find(|dependency| dependency.id == entry.id)
        .map(|dependency| dependency.version.clone());
    let mut next = current;
    next.dependencies
        .retain(|dependency| dependency.id != entry.id);
    next.dependencies.push(entry.clone());
    next.dependencies
        .sort_by(|left, right| left.id.cmp(&right.id));

    Resolver::with_manifest(root.clone(), next.clone())
        .and_then(|resolver| resolver.resolve(source.id))
        .map_err(|error| format!("the installed {} was refused: {error}", entry.id))?;
    write_manifest(&root, &next)?;

    let tool_root = root
        .join(source.id.family().directory())
        .join(source.id.as_str());
    let mut left_behind = None;
    let others = fs::read_dir(&tool_root)
        .map_err(|error| format!("{}: {error}", tool_root.display()))?
        .filter_map(Result::ok)
        .filter(|folder| folder.file_name().to_string_lossy() != entry.version);
    for folder in others {
        if fs::remove_dir_all(folder.path()).is_err() && left_behind.is_none() {
            left_behind = Some(folder.path());
        }
    }
    Ok(Installed {
        entry,
        replaced,
        left_behind,
    })
}

/// Copies a tool tree, refusing symlinks (a tree must not point outside
/// itself once installed).
fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("{}: {error}", destination.display()))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("{}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", source.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "{}: tool trees cannot contain symlinks",
                entry.path().display()
            ));
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| {
                format!(
                    "copy {} -> {}: {error}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

/// A relative path as the manifest spells it: forward slashes on every OS.
fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
        let path = validate_relative_path(&entry.path)?;
        let expected_family = Path::new(entry.family.directory());
        if path
            .components()
            .next()
            .map(|component| component.as_os_str())
            != expected_family
                .components()
                .next()
                .map(|component| component.as_os_str())
        {
            return Err(ResolveError::InvalidPath(entry.path.clone()));
        }
        validate_digest(&entry.sha256)?;
        if let Some(tree) = &entry.tree {
            let root = validate_relative_path(&tree.root)?;
            if root
                .components()
                .next()
                .map(|component| component.as_os_str())
                != expected_family
                    .components()
                    .next()
                    .map(|component| component.as_os_str())
                || !path.starts_with(&root)
            {
                return Err(ResolveError::InvalidPath(tree.root.clone()));
            }
            validate_digest(&tree.sha256)?;
        }
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
            "dpl-tools-deps-{name}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(root.join(TOOLS_DIRECTORY)).expect("tools root");
        root
    }

    fn write_bundle(
        root: &Path,
        id: &str,
        family: DependencyFamily,
        version: &str,
        relative: &str,
        payload: &[u8],
    ) {
        let tools = root.join(TOOLS_DIRECTORY);
        let path = tools.join(relative);
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
                family,
                version: version.to_owned(),
                path: relative.to_owned(),
                bytes: payload.len() as u64,
                sha256: sha256_file(&path).expect("digest"),
                tree: None,
            }],
        };
        fs::write(
            tools.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).expect("manifest"),
        )
        .expect("manifest");
    }

    #[test]
    fn resolves_only_verified_app_relative_files() {
        let root = release_root("resolve");
        let relative = if cfg!(windows) {
            "providers/grok/0.2.112/grok.exe"
        } else {
            "providers/grok/0.2.112/grok"
        };
        write_bundle(
            &root,
            "grok",
            DependencyFamily::Provider,
            "0.2.112",
            relative,
            b"private tool",
        );
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        let dependency = resolver.resolve(GROK).expect("grok");
        assert_eq!(dependency.version(), "0.2.112");
        assert!(dependency.path().is_absolute());
        assert!(dependency.path().starts_with(resolver.root()));
    }

    #[test]
    fn resolves_and_verifies_complete_browser_trees() {
        let root = release_root("browser-tree");
        let tools = root.join(TOOLS_DIRECTORY);
        let tree_relative = "browsers/chromium/136.0.0/chrome";
        let tree = tools.join(tree_relative);
        fs::create_dir_all(tree.join("locales")).expect("browser tree");
        let executable_name = if cfg!(windows) {
            "chrome.exe"
        } else {
            "chrome"
        };
        let executable = tree.join(executable_name);
        fs::write(&executable, b"browser").expect("browser");
        fs::write(tree.join("locales/en-US.pak"), b"locale").expect("locale");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
                .expect("executable permission");
        }
        let (tree_bytes, tree_digest) = sha256_tree(&tree).expect("tree digest");
        let entrypoint_relative = format!("{tree_relative}/{executable_name}");
        let manifest = Manifest {
            schema: MANIFEST_SCHEMA,
            target: current_target().to_owned(),
            dependencies: vec![ManifestDependency {
                id: "chromium".to_owned(),
                family: DependencyFamily::Browser,
                version: "136.0.0".to_owned(),
                path: entrypoint_relative,
                bytes: executable.metadata().expect("metadata").len(),
                sha256: sha256_file(&executable).expect("entrypoint digest"),
                tree: Some(ManifestTree {
                    root: tree_relative.to_owned(),
                    bytes: tree_bytes,
                    sha256: tree_digest,
                }),
            }],
        };
        fs::write(
            tools.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).expect("manifest"),
        )
        .expect("manifest");

        let resolver = Resolver::from_release_root(&root).expect("resolver");
        assert_eq!(
            resolver.resolve(CHROMIUM).expect("chromium").family(),
            DependencyFamily::Browser
        );

        fs::write(tree.join("locales/en-US.pak"), b"tampered").expect("tamper");
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        assert!(matches!(
            resolver.resolve(CHROMIUM),
            Err(ResolveError::TreeLengthMismatch { .. })
                | Err(ResolveError::TreeDigestMismatch { .. })
        ));
    }

    #[test]
    fn target_and_duplicate_ids_fail_closed() {
        let root = release_root("manifest");
        let tools = root.join(TOOLS_DIRECTORY);
        let entry = ManifestDependency {
            id: "grok".to_owned(),
            family: DependencyFamily::Provider,
            version: "0.2.112".to_owned(),
            path: "providers/grok/0.2.112/grok".to_owned(),
            bytes: 0,
            sha256: "0".repeat(64),
            tree: None,
        };
        let wrong_target = Manifest {
            schema: MANIFEST_SCHEMA,
            target: "another-target".to_owned(),
            dependencies: vec![entry.clone()],
        };
        fs::write(
            tools.join(MANIFEST_FILE),
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
            tools.join(MANIFEST_FILE),
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
        let tools = root.join(TOOLS_DIRECTORY);
        let invalid = Manifest {
            schema: MANIFEST_SCHEMA,
            target: current_target().to_owned(),
            dependencies: vec![ManifestDependency {
                id: "grok".to_owned(),
                family: DependencyFamily::Provider,
                version: "0.2.112".to_owned(),
                path: "../grok".to_owned(),
                bytes: 1,
                sha256: "0".repeat(64),
                tree: None,
            }],
        };
        fs::write(
            tools.join(MANIFEST_FILE),
            serde_json::to_vec(&invalid).expect("manifest"),
        )
        .expect("manifest");
        assert!(matches!(
            Resolver::from_release_root(&root),
            Err(ResolveError::InvalidPath(_))
        ));

        write_bundle(
            &root,
            "grok",
            DependencyFamily::Provider,
            "0.2.112",
            "providers/grok/0.2.112/grok",
            b"expected",
        );
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        fs::write(
            root.join(TOOLS_DIRECTORY)
                .join("providers/grok/0.2.112/grok"),
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
        write_bundle(
            &root,
            "grok",
            DependencyFamily::Provider,
            "0.2.112",
            "providers/grok/0.2.112/grok",
            b"grok",
        );
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        assert_eq!(
            resolver.resolve(CODEX),
            Err(ResolveError::MissingDependency("codex".to_owned()))
        );
    }

    /// A source tree outside any depot: an executable and one companion,
    /// the shape of an official CLI shipped with a helper beside it.
    fn source_tree(name: &str, payload: &[u8]) -> (PathBuf, PathBuf) {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let source = std::env::temp_dir().join(format!(
            "dpl-tools-deps-src-{name}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&source).expect("source");
        let entrypoint = PathBuf::from(format!("codex{}", std::env::consts::EXE_SUFFIX));
        let executable = source.join(&entrypoint);
        fs::write(&executable, payload).expect("executable");
        fs::write(source.join("companion.bin"), b"helper").expect("companion");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
                .expect("executable permission");
        }
        (source, entrypoint)
    }

    fn codex_bundle(name: &str) -> PathBuf {
        let root = release_root(name);
        let relative = format!(
            "providers/codex/0.145.0/codex{}",
            std::env::consts::EXE_SUFFIX
        );
        write_bundle(
            &root,
            "codex",
            DependencyFamily::Provider,
            "0.145.0",
            &relative,
            b"old codex",
        );
        root
    }

    #[test]
    fn an_in_place_install_makes_the_new_version_current_and_removes_the_old() {
        let root = codex_bundle("install");
        let tools = root.join(TOOLS_DIRECTORY);
        let (source, entrypoint) = source_tree("install", b"new codex");
        let installed = install_tool(
            &tools,
            &ToolSource {
                id: CODEX,
                version: "0.163.0",
                path: &source,
                entrypoint: Some(&entrypoint),
            },
        )
        .expect("install");
        assert_eq!(installed.replaced.as_deref(), Some("0.145.0"));
        assert_eq!(installed.left_behind, None);
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        let codex = resolver.resolve(CODEX).expect("codex");
        assert_eq!(codex.version(), "0.163.0");
        assert_eq!(fs::read(codex.path()).expect("bytes"), b"new codex");
        assert!(
            codex.path().with_file_name("companion.bin").is_file(),
            "the companion ships with the executable"
        );
        assert!(
            !tools.join("providers/codex/0.145.0").exists(),
            "one baseline: the replaced version is gone"
        );
        assert!(
            !tools.join(format!("{MANIFEST_FILE}.tmp")).exists(),
            "the manifest switched by rename, leaving no temporary file"
        );
    }

    #[test]
    fn a_refused_install_leaves_the_depot_as_it_was() {
        let root = codex_bundle("refused");
        let tools = root.join(TOOLS_DIRECTORY);
        let before = fs::read(tools.join(MANIFEST_FILE)).expect("manifest");
        let (source, _) = source_tree("refused", b"new codex");
        let refused = install_tool(
            &tools,
            &ToolSource {
                id: CODEX,
                version: "0.163.0",
                path: &source,
                entrypoint: Some(Path::new("missing.exe")),
            },
        );
        assert!(
            refused.is_err(),
            "an entrypoint that is not there is refused"
        );
        assert_eq!(
            fs::read(tools.join(MANIFEST_FILE)).expect("manifest"),
            before,
            "the manifest is untouched"
        );
        let resolver = Resolver::from_release_root(&root).expect("resolver");
        assert_eq!(resolver.resolve(CODEX).expect("codex").version(), "0.145.0");
    }

    #[test]
    fn a_single_file_source_takes_no_entrypoint() {
        let root = release_root("single");
        let tools = root.join(TOOLS_DIRECTORY);
        let (source, entrypoint) = source_tree("single", b"codex");
        let executable = source.join(&entrypoint);
        let placed = place_tool(
            &tools,
            &ToolSource {
                id: CODEX,
                version: "0.163.0",
                path: &executable,
                entrypoint: None,
            },
        )
        .expect("placed");
        assert_eq!(
            placed.path,
            format!(
                "providers/codex/0.163.0/codex{}",
                std::env::consts::EXE_SUFFIX
            )
        );
        assert_eq!(placed.tree, None);
        assert!(place_tool(
            &tools,
            &ToolSource {
                id: CODEX,
                version: "0.163.0",
                path: &executable,
                entrypoint: Some(&entrypoint),
            },
        )
        .is_err());
    }
}
