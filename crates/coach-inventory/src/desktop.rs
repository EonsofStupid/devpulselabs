//! Phase 0 — XDG Desktop Entry cold catalog (no AT-SPI).

use std::fs;
use std::path::PathBuf;

/// Desktop file id (basename), e.g. `org.gnome.Calculator.desktop`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DesktopId(String);

impl DesktopId {
    pub fn from_raw(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Parsed cold catalog row (visible Application entries only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdEntry {
    pub id: DesktopId,
    pub display_name: String,
    pub exec_hint: String,
    pub startup_wm_class: Option<String>,
    pub path: PathBuf,
}

/// XDG application search path + current desktop for OnlyShowIn.
#[derive(Debug, Clone)]
pub struct ApplicationDirs {
    pub paths: Vec<PathBuf>,
    pub current_desktop: String,
}

impl ApplicationDirs {
    pub fn from_env() -> Self {
        let mut paths = Vec::new();
        let home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut h = dirs_home();
                h.push(".local/share");
                h
            });
        paths.push(home.join("applications"));
        let data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        for part in data_dirs.split(':').filter(|s| !s.is_empty()) {
            paths.push(PathBuf::from(part).join("applications"));
        }
        let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        Self {
            paths,
            current_desktop,
        }
    }
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Refresh the cold catalog from disk (Phase 0). Safe to call off the UI thread.
pub fn refresh_cold() -> Result<Vec<ColdEntry>, String> {
    cold_catalog(&ApplicationDirs::from_env())
}

/// Scan application dirs; first desktop-id wins (Menu Spec).
pub fn cold_catalog(dirs: &ApplicationDirs) -> Result<Vec<ColdEntry>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in &dirs.paths {
        if !dir.is_dir() {
            continue;
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|e| format!("read_dir {}: {e}", dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("desktop"))
            .collect();
        entries.sort();
        for path in entries {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !seen.insert(name.to_string()) {
                continue;
            }
            let text = fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {e}", path.display()))?;
            if let Some(mut entry) = parse_desktop_entry(name, &text, &dirs.current_desktop) {
                entry.path = path;
                out.push(entry);
            }
        }
    }
    out.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    Ok(out)
}

/// Parse a single `.desktop` body. Returns `None` when filtered out.
pub fn parse_desktop_entry(
    desktop_id: &str,
    body: &str,
    current_desktop: &str,
) -> Option<ColdEntry> {
    let mut in_entry = false;
    let mut ty = String::new();
    let mut name = String::new();
    let mut exec = String::new();
    let mut try_exec = String::new();
    let mut dbus_activatable = false;
    let mut hidden = false;
    let mut no_display = false;
    let mut only_show_in = String::new();
    let mut not_show_in = String::new();
    let mut startup_wm_class = None;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k {
            "Type" => ty = v.to_string(),
            "Name" if name.is_empty() => name = v.to_string(),
            "Exec" if exec.is_empty() => exec = v.to_string(),
            "TryExec" if try_exec.is_empty() => try_exec = v.to_string(),
            "DBusActivatable" => dbus_activatable = v.eq_ignore_ascii_case("true"),
            "Hidden" => hidden = v.eq_ignore_ascii_case("true"),
            "NoDisplay" => no_display = v.eq_ignore_ascii_case("true"),
            "OnlyShowIn" => only_show_in = v.to_string(),
            "NotShowIn" => not_show_in = v.to_string(),
            "StartupWMClass" => startup_wm_class = Some(v.to_string()),
            _ => {}
        }
    }

    if ty != "Application" || name.is_empty() {
        return None;
    }
    if hidden || no_display {
        return None;
    }
    let exec_hint = if !exec.is_empty() {
        exec.split_whitespace().next().unwrap_or(&exec).to_string()
    } else if !try_exec.is_empty() {
        try_exec
    } else if dbus_activatable {
        String::new()
    } else {
        return None;
    };

    if !only_show_in.is_empty() && !desktop_list_matches(&only_show_in, current_desktop) {
        return None;
    }
    if !not_show_in.is_empty() && desktop_list_matches(&not_show_in, current_desktop) {
        return None;
    }

    Some(ColdEntry {
        id: DesktopId::from_raw(desktop_id),
        display_name: name,
        exec_hint,
        startup_wm_class,
        path: PathBuf::from(desktop_id),
    })
}

fn desktop_list_matches(list: &str, current: &str) -> bool {
    if current.is_empty() {
        return false;
    }
    let currents: Vec<&str> = current.split(':').filter(|s| !s.is_empty()).collect();
    list.split(';')
        .filter(|s| !s.is_empty())
        .any(|item| currents.iter().any(|c| c.eq_ignore_ascii_case(item)))
}

