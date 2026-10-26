use crate::model::{Config, Marker, CONFIG_FILE, MARKER_FILE};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub(crate) fn absolute(path: &Path, base: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("The library path is empty".into());
    }
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    if !path.is_absolute() {
        return Err("The library path must resolve to an absolute path".into());
    }
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return Err("The library path escapes its filesystem root".into());
                }
            }
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}

pub(crate) fn no_symlinks(path: &Path) -> Result<(), String> {
    // Inspect each existing component, including dangling links. Root config
    // cannot quietly redirect account state through a symlink/junction.
    let mut cursor = PathBuf::new();
    for part in path.components() {
        cursor.push(part.as_os_str());
        // `D:` alone means that drive's current directory on Windows. Wait
        // for its RootDir component before inspecting an absolute path.
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&cursor) {
            Ok(meta) if is_link(&meta) => {
                return Err(format!(
                    "{} is a link; choose its real folder",
                    cursor.display()
                ))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", cursor.display())),
        }
    }
    Ok(())
}

fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT includes directory junctions.
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

/// Every existing component of a folder path must itself be a directory.
/// Missing folders may be created later, after the library identity is checked.
pub(crate) fn directory_path(path: &Path) -> Result<(), String> {
    no_symlinks(path)?;
    let mut cursor = PathBuf::new();
    for part in path.components() {
        cursor.push(part.as_os_str());
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&cursor) {
            Ok(meta) if !meta.is_dir() => {
                return Err(format!("{} must be a folder", cursor.display()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", cursor.display())),
        }
    }
    Ok(())
}

pub(crate) fn read_text(path: &Path) -> Result<String, String> {
    no_symlinks(path)?;
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > 65_536 {
        return Err(format!("{} is too large", path.display()));
    }
    String::from_utf8(bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn read_marker(library: &Path) -> Result<Marker, String> {
    let marker: Marker = serde_json::from_str(&read_text(&library.join(MARKER_FILE))?)
        .map_err(|e| format!("{}: {e}", library.join(MARKER_FILE).display()))?;
    marker.validate()?;
    Ok(marker)
}

pub(crate) fn read_config(program: &Path) -> Result<Config, String> {
    let config: Config = toml::from_str(&read_text(&program.join(CONFIG_FILE))?)
        .map_err(|e| format!("{}: {e}", program.join(CONFIG_FILE).display()))?;
    config.validate()?;
    Ok(config)
}

pub(crate) fn create_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    no_symlinks(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Cannot create {} without replacing it: {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| {
            format!(
                "Could not finish {}: {e}; the partial file is kept for inspection",
                path.display()
            )
        })
}

pub(crate) fn create_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let mut json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    json.push('\n');
    create_file(path, json.as_bytes())
}

pub(crate) fn create_config(program: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    let raw = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    create_file(&program.join(CONFIG_FILE), raw.as_bytes())
}
