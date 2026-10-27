use crate::{discovery, disk, AccountId, Config, DrillRoots, Layout, Mode, State, CONFIG_FILE};
use std::path::{Path, PathBuf};

/// One library's current location. Resolving and refreshing only read;
/// `remember` writes a linked library's new path back to `roots.toml`.
#[derive(Clone, Debug)]
pub struct Roots {
    pub(crate) program: PathBuf,
    pub(crate) config: Option<Config>,
    pub(crate) drill: bool,
    pub(crate) state: State,
    checked: Vec<PathBuf>,
    /// `last_path` as `roots.toml` holds it on disk.
    recorded: Option<PathBuf>,
    /// What opening noticed without failing, for the app to log.
    pub(crate) notice: Option<String>,
}

impl Roots {
    pub fn open(program: &Path, drill: Option<DrillRoots>) -> Self {
        let mut roots = Self {
            program: program.to_path_buf(),
            config: None,
            drill: false,
            state: State::Unconfigured,
            checked: vec![program.join(CONFIG_FILE)],
            recorded: None,
            notice: None,
        };
        if !program.is_absolute() {
            roots.fail("The program directory must be absolute".into());
            return roots;
        }
        if let Some(drill) = drill {
            roots.drill = true;
            match drill.layout() {
                Ok(layout) => {
                    roots.checked = vec![layout.content.clone()];
                    roots.state = State::Connected(layout);
                }
                Err(error) => roots.fail(error),
            }
            return roots;
        }
        match std::fs::symlink_metadata(program.join(CONFIG_FILE)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return roots,
            _ => {}
        }
        match disk::read_config(program) {
            Ok(config) => {
                roots.recorded = config.last_path.clone();
                roots.config = Some(config);
                let _ = roots.refresh_with_candidates(&[]);
            }
            Err(error) => roots.fail(error),
        }
        roots
    }

    pub(crate) fn failed(program: PathBuf, error: String) -> Self {
        Self {
            program,
            config: None,
            drill: false,
            state: State::Disconnected { reason: error },
            checked: Vec::new(),
            recorded: None,
            notice: None,
        }
    }

    fn fail(&mut self, reason: String) {
        self.state = State::Disconnected { reason };
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn layout(&self) -> Result<Layout, String> {
        match &self.state {
            State::Connected(layout) => Ok(layout.clone()),
            State::Unconfigured => Err("Choose where to keep your library".into()),
            State::Disconnected { reason } | State::Unplugged { reason, .. } => Err(reason.clone()),
        }
    }

    /// The folders to read from: the library while it is open, and the last
    /// ones it had while it is unplugged mid-session.
    pub(crate) fn paths(&self) -> Option<Layout> {
        match &self.state {
            State::Connected(layout) | State::Unplugged { last: layout, .. } => {
                Some(layout.clone())
            }
            _ => None,
        }
    }

    /// An open library has gone (ST2b): keep its folders for readers.
    pub(crate) fn unplug(&mut self, reason: String) {
        if let State::Connected(last) = &self.state {
            self.state = State::Unplugged {
                last: last.clone(),
                reason,
            };
        }
    }

    pub fn checked_roots(&self) -> &[PathBuf] {
        &self.checked
    }

    pub fn is_drill(&self) -> bool {
        self.drill
    }

    pub fn config(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    /// A linked library found somewhere new (another drive letter) keeps
    /// that path in `roots.toml`, so the next launch looks there first.
    /// Returns true when the file was rewritten. An imported library and a
    /// seam never write.
    pub fn remember(&mut self) -> Result<bool, String> {
        let Some(config) = self.config.as_ref() else {
            return Ok(false);
        };
        if self.drill
            || config.mode != Mode::Linked
            || !matches!(self.state, State::Connected(_))
            || config.last_path == self.recorded
        {
            return Ok(false);
        }
        disk::replace_config(&self.program, config)?;
        self.recorded = config.last_path.clone();
        Ok(true)
    }

    /// Check the remembered path first. Only a disconnected linked library
    /// probes the same suffix on other drive letters or supported mount points.
    pub fn refresh(&mut self) -> Result<Layout, String> {
        if self.drill {
            return self.layout();
        }
        if let Ok(layout) = self.refresh_with_candidates(&[]) {
            return Ok(layout);
        }
        let candidates = match self.config.as_ref() {
            Some(config) if config.mode == Mode::Linked => {
                discovery::drive_candidates(&config.library_path(&self.program)?)
            }
            _ => Vec::new(),
        };
        self.refresh_with_candidates(&candidates)
    }

    /// Exact candidate library paths, not directories to scan. Duplicate
    /// identities at distinct paths are ambiguous and never picked arbitrarily.
    pub fn refresh_with_candidates(&mut self, candidates: &[PathBuf]) -> Result<Layout, String> {
        if self.drill {
            return self.layout();
        }
        let Some(config) = self.config.clone() else {
            return self.layout();
        };
        let last = config.library_path(&self.program)?;
        let mut paths = vec![last.clone()];
        if config.mode == Mode::Linked {
            paths.extend(candidates.iter().cloned());
        }
        self.checked = vec![self.program.join(CONFIG_FILE)];
        let mut matches = Vec::new();
        let mut canonical_matches = Vec::new();
        let mut last_error = None;
        for path in paths {
            if !path.is_absolute() {
                continue;
            }
            if self.checked.contains(&path) {
                continue;
            }
            self.checked.push(path.clone());
            match disk::read_marker(&path) {
                Ok(marker) if marker.id == config.id => {
                    let canonical = match std::fs::canonicalize(&path) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    if !canonical_matches.contains(&canonical) {
                        canonical_matches.push(canonical);
                        matches.push(path);
                    }
                }
                Ok(_) if path == last => {
                    last_error = Some("That folder belongs to a different library".to_owned())
                }
                Err(error) if path == last => last_error = Some(error),
                _ => {}
            }
        }
        let result = match matches.as_slice() {
            [path] => {
                let layout = Layout::for_library(path, &config.account);
                validate_layout(&layout, &config.account).map(|()| layout)
            },
            [] => Err(format!("Your library '{}' is not connected at {}. {}", config.label, last.display(), last_error.unwrap_or_default())),
            _ => Err(format!("More than one connected folder has the identity of '{}'. Disconnect the extra copy and try again", config.label)),
        };
        match result {
            Ok(layout) => {
                if let Some(config) = self.config.as_mut() {
                    if config.mode == Mode::Linked {
                        config.last_path = Some(layout.library.clone());
                    }
                }
                self.state = State::Connected(layout.clone());
                Ok(layout)
            }
            Err(reason) => {
                self.fail(reason.clone());
                Err(reason)
            }
        }
    }

    /// Creates only the folders of an already resolved library. A missing or
    /// replaced marker is refused, never repaired or replaced automatically.
    pub fn ensure_rings(&mut self) -> Result<Layout, String> {
        let layout = self.layout()?;
        if let Some(config) = &self.config {
            let validation = disk::read_marker(&layout.library).and_then(|marker| {
                if marker.id != config.id {
                    return Err("The library at this path has changed".into());
                }
                validate_layout(&layout, &config.account)
            });
            if let Err(error) = validation {
                self.fail(error);
                return self.layout();
            }
        }
        for dir in [
            &layout.content,
            &layout.assets,
            &layout.userdata,
            &layout.aidata,
            &layout.cache,
            &layout.temp,
            &layout.userdata.join("profiles"),
        ] {
            disk::no_symlinks(dir)?;
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let readme = layout.userdata.join("README.txt");
        match std::fs::symlink_metadata(&readme) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => disk::create_file(&readme, b"App state: keep this folder. Databases and browser profiles are precious.\nThe library common/cache and common/temp folders are safe to delete when the app is closed.\n")?,
            Ok(_) => disk::no_symlinks(&readme)?,
            Err(e) => return Err(e.to_string()),
        }
        Ok(layout)
    }

    pub fn locate_asset(&self, rel: &str) -> Option<PathBuf> {
        let path = Path::new(rel);
        if path.is_absolute() {
            return path.is_file().then(|| path.to_path_buf());
        }
        if path.components().any(|c| {
            !matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        }) {
            return None;
        }
        let layout = self.layout().ok()?;
        // Content-relative media first; ring-prefixed and shipped program assets
        // remain supported without searching ancestors or the launch directory.
        let mut candidates = vec![layout.content.join(path)];
        // A linked file (ST5): wherever its location is now. An export's
        // copy under `content/locations/` is found first, above.
        if let Some(file) = (crate::locations::Store {
            library: &layout.library,
            userdata: &layout.userdata,
        })
        .resolve(path)
        {
            candidates.push(file);
        }
        if let Ok(suffix) = path.strip_prefix("assets") {
            candidates.push(layout.assets.join(suffix));
        }
        if let Ok(suffix) = path.strip_prefix("content") {
            candidates.push(layout.content.join(suffix));
        }
        candidates.push(self.program.join(path));
        candidates.into_iter().find(|candidate| candidate.is_file())
    }
}

pub(crate) fn validate_layout(layout: &Layout, account: &AccountId) -> Result<(), String> {
    for path in [
        &layout.content,
        &layout.assets,
        &layout.userdata,
        &layout.aidata,
        &layout.cache,
        &layout.temp,
        &layout.userdata.join("profiles"),
    ] {
        disk::directory_path(path)?;
    }
    let file = layout
        .library
        .join("accounts")
        .join(account.as_str())
        .join("account.json");
    let value: crate::setup::Account = serde_json::from_str(&disk::read_text(&file)?)
        .map_err(|e| format!("{}: {e}", file.display()))?;
    crate::model::version(value.v, "account.json")?;
    if &value.id != account {
        return Err("The account identity does not match its library folder".into());
    }
    Ok(())
}

impl DrillRoots {
    fn layout(self) -> Result<Layout, String> {
        for path in [
            &self.content,
            self.assets.as_ref().unwrap_or(&self.content),
            self.userdata.as_ref().unwrap_or(&self.content),
        ] {
            if !path.is_absolute() {
                return Err(
                    "Explicit scratch roots must be absolute after argument resolution".into(),
                );
            }
        }
        let library = self.content.parent().unwrap_or(&self.content).to_path_buf();
        Ok(Layout {
            content: self.content,
            assets: self.assets.unwrap_or_else(|| library.join("assets")),
            userdata: self.userdata.unwrap_or_else(|| library.join(".userdata")),
            aidata: library.join(".aidata"),
            cache: library.join("common/cache"),
            temp: library.join("common/temp"),
            library,
        })
    }
}
