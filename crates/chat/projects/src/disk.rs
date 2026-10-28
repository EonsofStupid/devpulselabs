use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub(crate) const MAX_RECORD_BYTES: u64 = 32 * 1024 * 1024;

pub(crate) fn path(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("Choose an absolute project folder without '..' components".into());
    }
    let mut cursor = PathBuf::new();
    for part in path.components() {
        cursor.push(part.as_os_str());
        // A bare drive prefix refers to that drive's current directory.
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&cursor) {
            Ok(meta) if is_link(&meta) => {
                return Err(format!(
                    "{} is a link; choose its real folder",
                    cursor.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", cursor.display())),
        }
    }
    Ok(())
}

pub(crate) fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

pub(crate) fn existing_directory(dir: &Path) -> Result<(), String> {
    path(dir)?;
    if !dir.is_dir() {
        return Err(format!("{} is not an available folder", dir.display()));
    }
    Ok(())
}

pub(crate) fn empty_or_absent(dir: &Path) -> Result<(), String> {
    path(dir)?;
    match fs::read_dir(dir) {
        Ok(mut entries) => match entries.next() {
            None => Ok(()),
            Some(Ok(_)) => Err(format!(
                "{} already contains unrecognized files; nothing was replaced",
                dir.display()
            )),
            Some(Err(error)) => Err(error.to_string()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", dir.display())),
    }
}

pub(crate) fn read(file: &Path, maximum: u64) -> Result<Vec<u8>, String> {
    path(file)?;
    if !fs::symlink_metadata(file)
        .map_err(|error| format!("{}: {error}", file.display()))?
        .is_file()
    {
        return Err(format!("{} must be a regular file", file.display()));
    }
    let file_handle = File::open(file).map_err(|error| format!("{}: {error}", file.display()))?;
    if !file_handle
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err(format!("{} must be a regular file", file.display()));
    }
    let mut bytes = Vec::new();
    file_handle
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > maximum {
        return Err(format!("{} exceeds the storage limit", file.display()));
    }
    Ok(bytes)
}

pub(crate) fn json<T: serde::de::DeserializeOwned>(file: &Path, maximum: u64) -> Result<T, String> {
    serde_json::from_slice(&read(file, maximum)?)
        .map_err(|error| format!("{} is not valid data: {error}", file.display()))
}

pub(crate) fn create(file: &Path, bytes: &[u8]) -> Result<(), String> {
    path(file)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)
        .map_err(|error| {
            format!(
                "Cannot create {} without replacing it: {error}",
                file.display()
            )
        })?;
    output
        .write_all(bytes)
        .and_then(|()| output.sync_all())
        .map_err(|error| {
            format!(
                "Could not finish {}; its partial data is retained: {error}",
                file.display()
            )
        })?;
    #[cfg(unix)]
    if let Some(parent) = file.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

pub(crate) fn create_directories(dir: &Path) -> Result<(), String> {
    path(dir)?;
    #[cfg(unix)]
    let missing = {
        let mut missing = Vec::new();
        let mut cursor = Some(dir);
        while let Some(candidate) = cursor {
            if candidate.exists() {
                break;
            }
            missing.push(candidate.to_path_buf());
            cursor = candidate.parent();
        }
        missing
    };
    fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    for created in missing {
        sync_directory(&created)?;
        if let Some(parent) = created.parent() {
            sync_directory(parent)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(dir: &Path) -> Result<(), String> {
    path(dir)?;
    File::open(dir)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("Could not sync directory {}: {error}", dir.display()))
}

pub(crate) fn create_json(file: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    create(file, &bytes)
}

/// The lock file contains no user data. OS locks release when its handle or
/// process closes, so an interrupted write does not leave a stale PID lease.
pub(crate) fn lock(dir: &Path) -> Result<File, String> {
    let file = dir.join("writer.lock");
    path(&file)?;
    match fs::symlink_metadata(&file) {
        Ok(meta) if !meta.is_file() => {
            return Err("Project chat writer lock is not a regular file".into());
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.to_string()),
        _ => {}
    }
    let handle = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&file)
        .map_err(|error| format!("{}: {error}", file.display()))?;
    handle.try_lock().map_err(|_| {
        "Project chat storage is busy; try again when the other save finishes".to_owned()
    })?;
    Ok(handle)
}
