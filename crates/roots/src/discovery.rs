use std::path::{Component, Path, PathBuf};

/// Retain the path inside the volume and substitute only a volume root.
/// Callers can inject fake roots; this function never walks a directory tree.
pub fn candidates_on_roots(last: &Path, previous_root: &Path, roots: &[PathBuf]) -> Vec<PathBuf> {
    let Ok(suffix) = last.strip_prefix(previous_root) else {
        return Vec::new();
    };
    if suffix
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Vec::new();
    }
    roots.iter().map(|root| root.join(suffix)).collect()
}

pub(crate) fn drive_candidates(last: &Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        use std::path::Prefix;
        let mut components = last.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return Vec::new();
        };
        let drive = match prefix.kind() {
            Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d,
            _ => return Vec::new(), // Never search network shares.
        };
        if components.next() != Some(Component::RootDir) {
            return Vec::new();
        }
        let suffix = components.as_path();
        (b'A'..=b'Z')
            .filter(|d| *d != drive.to_ascii_uppercase())
            .map(|d| PathBuf::from(format!("{}:/", char::from(d))).join(suffix))
            .collect()
    }
    #[cfg(not(windows))]
    {
        let Some(old_mount) = mount_root(last) else {
            return Vec::new();
        };
        let mut mounts = Vec::new();
        for base in [Path::new("/media"), Path::new("/run/media")] {
            for user in child_dirs(base) {
                mounts.extend(child_dirs(&user));
            }
        }
        mounts.extend(child_dirs(Path::new("/mnt")));
        candidates_on_roots(last, &old_mount, &mounts)
    }
}

#[cfg(not(windows))]
fn child_dirs(base: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(base)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect()
}

#[cfg(not(windows))]
fn mount_root(path: &Path) -> Option<PathBuf> {
    for (base, depth) in [("/media", 2), ("/run/media", 2), ("/mnt", 1)] {
        if let Ok(rest) = path.strip_prefix(base) {
            let parts: Vec<_> = rest.components().take(depth).collect();
            if parts.len() == depth && parts.iter().all(|p| matches!(p, Component::Normal(_))) {
                return Some(parts.iter().fold(PathBuf::from(base), |mut root, part| {
                    root.push(part.as_os_str());
                    root
                }));
            }
        }
    }
    None
}
