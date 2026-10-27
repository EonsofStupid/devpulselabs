use crate::{
    disk, handle, AccountId, Config, Layout, LibraryId, Marker, Mode, CONFIG_FILE, MARKER_FILE,
    SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
pub(crate) struct Account {
    pub v: u32,
    pub id: AccountId,
    pub name: String,
    pub created: u64,
}

pub(crate) fn timestamp() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| e.to_string())
}

fn new_id() -> LibraryId {
    // Identity, not an authentication secret. Independently seeded standard
    // hashers avoid a timestamp/process-id collision across portable machines.
    let one = RandomState::new().build_hasher().finish();
    let two = RandomState::new().build_hasher().finish();
    LibraryId::parse(format!("library-{one:016x}{two:016x}"))
        .expect("generated identifier is valid")
}

pub(crate) fn config_available(program: &Path) -> Result<(), String> {
    disk::no_symlinks(program)?;
    if !program.is_absolute() || !program.is_dir() {
        return Err("Choose an existing absolute program folder".into());
    }
    match std::fs::symlink_metadata(program.join(CONFIG_FILE)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
        Ok(_) => Err("roots.toml already exists; it will not be replaced".into()),
    }
}

/// Explicit first-launch selection only. Existing content is not adopted:
/// a populated unmarked folder is brought in only by the shown move (ST1b).
pub fn setup_at(
    program: &Path,
    selected: &Path,
    mode: Mode,
    label: &str,
) -> Result<Layout, String> {
    config_available(program)?;
    crate::model::label(label)?;
    let library = disk::absolute(selected, program)?;
    reject_config_location(program, &library)?;
    if mode == Mode::Imported && library != program.join("library") {
        return Err("An imported library must be in the program's library folder".into());
    }
    disk::no_symlinks(&library)?;
    let marker = match std::fs::symlink_metadata(library.join(MARKER_FILE)) {
        Ok(_) => disk::read_marker(&library)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match std::fs::read_dir(&library) {
                Ok(mut entries) => {
                    if entries.next().is_some() {
                        return Err("This folder already contains files but has no library.json. Choose an empty folder or an existing DevPULSE library. To copy today's folders into a new library, use Bring in my existing library".into());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("{}: {e}", library.display())),
            }
            create_layout(&Layout::for_library(&library, &AccountId::default()))?;
            publish_identity(&library, label)?
        }
        Err(e) => return Err(e.to_string()),
    };
    let layout = Layout::for_library(&library, &AccountId::default());
    handle::validate_layout(&layout, &AccountId::default())?;
    write_config(program, &library, mode, marker)?;
    Ok(layout)
}

/// Every ring folder of a new library, before anything is published.
pub(crate) fn create_layout(layout: &Layout) -> Result<(), String> {
    for path in [
        &layout.content,
        &layout.assets,
        &layout.userdata,
        &layout.aidata,
        &layout.cache,
        &layout.temp,
    ] {
        disk::no_symlinks(path)?;
        std::fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// Publish identity only after a valid complete account layout exists: the
/// account first, the marker last.
pub(crate) fn publish_identity(library: &Path, label: &str) -> Result<Marker, String> {
    let marker = Marker {
        v: SCHEMA_VERSION,
        id: new_id(),
        label: label.to_owned(),
        created: timestamp()?,
    };
    let account = Account {
        v: SCHEMA_VERSION,
        id: AccountId::default(),
        name: "Me".into(),
        created: marker.created,
    };
    let account_file = library
        .join("accounts")
        .join(account.id.as_str())
        .join("account.json");
    disk::create_json(&account_file, &account)?;
    disk::create_json(&library.join(MARKER_FILE), &marker)?;
    Ok(marker)
}

/// The program's `roots.toml` for a published library; never replaces one.
pub(crate) fn write_config(
    program: &Path,
    library: &Path,
    mode: Mode,
    marker: Marker,
) -> Result<(), String> {
    let linked = mode == Mode::Linked;
    disk::create_config(
        program,
        &Config {
            v: SCHEMA_VERSION,
            mode,
            id: marker.id,
            label: marker.label,
            account: AccountId::default(),
            last_path: linked.then(|| library.to_path_buf()),
        },
    )
}

pub(crate) fn reject_config_location(program: &Path, library: &Path) -> Result<(), String> {
    let program = std::fs::canonicalize(program).map_err(|e| e.to_string())?;
    for ancestor in library.ancestors() {
        let Some(name) = ancestor.file_name() else {
            continue;
        };
        #[cfg(windows)]
        let is_config = name.to_string_lossy().eq_ignore_ascii_case(CONFIG_FILE);
        #[cfg(not(windows))]
        let is_config = name == CONFIG_FILE;
        if is_config
            && ancestor
                .parent()
                .and_then(|parent| std::fs::canonicalize(parent).ok())
                .is_some_and(|parent| parent == program)
        {
            return Err(
                "Choose a library folder outside the program's roots.toml configuration path"
                    .into(),
            );
        }
    }
    Ok(())
}

/// Stage an existing, identified library for an executable. Does not touch
/// that library or overwrite an existing executable configuration.
pub fn stage_roots(program: &Path, library: &Path) -> Result<(), String> {
    config_available(program)?;
    let library = disk::absolute(library, program)?;
    let marker = disk::read_marker(&library)?;
    let account = AccountId::default();
    handle::validate_layout(&Layout::for_library(&library, &account), &account)?;
    write_config(program, &library, mode_for(program, &library), marker)
}

/// Imported when the library is the program's own `library\`, else linked.
pub(crate) fn mode_for(program: &Path, library: &Path) -> Mode {
    if library == program.join("library") {
        Mode::Imported
    } else {
        Mode::Linked
    }
}
