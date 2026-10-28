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

/// The same path inside the volume on every other drive: other drive
/// letters on Windows, other removable mounts elsewhere.
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
            .map(|d| PathBuf::from(format!("{}:\\", char::from(d))).join(suffix))
            .collect()
    }
    #[cfg(not(windows))]
    {
        let mounts = removable_mounts();
        // A drive that is away has no mount; its root is guessed from the
        // path's shape, and the location's files decide.
        let previous = root_among(last, &mounts)
            .map(|root| vec![root])
            .unwrap_or_else(|| shape_roots(last));
        let mut found = Vec::new();
        for root in &previous {
            for candidate in candidates_on_roots(last, root, &mounts) {
                if candidate != last && !found.contains(&candidate) {
                    found.push(candidate);
                }
            }
        }
        found
    }
}

/// Where removable drives appear outside Windows, and how many folders down
/// a drive's root usually sits: udisks's `/media/<user>/<label>` and
/// `/run/media/<user>/<label>`, `/mnt/<name>`, and macOS's
/// `/Volumes/<name>`. usbmount's `/media/usb0` sits one down, which the
/// mount table knows and the shape guess also tries.
#[cfg(any(not(windows), test))]
const BASES: [(&str, usize); 4] = [
    ("/media", 2),
    ("/run/media", 2),
    ("/mnt", 1),
    ("/Volumes", 1),
];

/// The removable drives mounted now: Linux (any architecture) names them
/// exactly in its mount table; without one (macOS), the folders under each
/// base stand in.
#[cfg(not(windows))]
fn removable_mounts() -> Vec<PathBuf> {
    match std::fs::read_to_string("/proc/self/mounts") {
        Ok(table) => mounts_in(&table),
        Err(_) => BASES
            .iter()
            .flat_map(|(base, depth)| dirs_at(Path::new(base), *depth))
            .collect(),
    }
}

#[cfg(not(windows))]
fn dirs_at(base: &Path, depth: usize) -> Vec<PathBuf> {
    let mut level = vec![base.to_path_buf()];
    for _ in 0..depth {
        level = level.iter().flat_map(|dir| child_dirs(dir)).collect();
    }
    level
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

/// The mount points under `BASES` in a kernel mount table
/// (`/proc/self/mounts`): the second field of each line, with the kernel's
/// `\040`-style escapes for spaces, tabs and backslashes decoded.
#[cfg(any(not(windows), test))]
pub(crate) fn mounts_in(table: &str) -> Vec<PathBuf> {
    table
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(unescape)
        .filter(|mount| {
            BASES
                .iter()
                .any(|(base, _)| mount.starts_with(base) && mount != Path::new(base))
        })
        .collect()
}

#[cfg(any(not(windows), test))]
fn unescape(field: &str) -> PathBuf {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let octal = bytes
            .get(i + 1..i + 4)
            .filter(|digits| bytes[i] == b'\\' && digits.iter().all(|d| (b'0'..=b'7').contains(d)))
            .and_then(|digits| {
                let value = digits
                    .iter()
                    .fold(0u32, |value, digit| value * 8 + u32::from(digit - b'0'));
                u8::try_from(value).ok()
            });
        match octal {
            Some(byte) => {
                out.push(byte);
                i += 4;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(out).into()
    }
    #[cfg(not(unix))]
    {
        String::from_utf8_lossy(&out).into_owned().into()
    }
}

/// The deepest of `mounts` that holds `path`.
#[cfg(any(not(windows), test))]
pub(crate) fn root_among(path: &Path, mounts: &[PathBuf]) -> Option<PathBuf> {
    mounts
        .iter()
        .filter(|mount| path.starts_with(mount))
        .max_by_key(|mount| mount.components().count())
        .cloned()
}

/// Where a drive's root could have been, from the path's shape alone: one
/// folder down a base, and two where the base usually nests a user folder.
#[cfg(any(not(windows), test))]
pub(crate) fn shape_roots(path: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for (base, depth) in BASES {
        let Ok(rest) = path.strip_prefix(base) else {
            continue;
        };
        let parts: Vec<_> = rest.components().take(depth).collect();
        for n in 1..=parts.len() {
            if parts[..n].iter().all(|p| matches!(p, Component::Normal(_))) {
                roots.push(
                    parts[..n]
                        .iter()
                        .fold(PathBuf::from(base), |mut root, part| {
                            root.push(part.as_os_str());
                            root
                        }),
                );
            }
        }
    }
    roots
}

/// The removable drive a path is on: the mounted one that holds it, else the
/// root its shape names.
#[cfg(not(windows))]
pub(crate) fn mount_root(path: &Path) -> Option<PathBuf> {
    root_among(path, &removable_mounts()).or_else(|| shape_roots(path).pop())
}
