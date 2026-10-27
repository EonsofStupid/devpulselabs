use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "roots.toml";
pub const MARKER_FILE: &str = "library.json";
pub const SCHEMA_VERSION: u32 = 1;

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        && !matches!(
            value.to_ascii_uppercase().as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
}

macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, String> {
                let value = value.into();
                if !valid_id(&value) {
                    return Err(format!(
                        "Invalid {}: use a short identifier containing letters, numbers, - or _",
                        stringify!($name)
                    ));
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

identity!(LibraryId);
identity!(AccountId);

impl Default for AccountId {
    fn default() -> Self {
        Self("me".to_owned())
    }
}

/// Identity travels with the library; no drive letter or machine id lives here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub v: u32,
    pub id: LibraryId,
    pub label: String,
    /// Unix time in seconds, encoded as an integer to avoid a clock dependency.
    pub created: u64,
}

impl Marker {
    pub fn validate(&self) -> Result<(), String> {
        version(self.v, MARKER_FILE)?;
        label(&self.label)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Imported,
    Linked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub v: u32,
    pub mode: Mode,
    pub id: LibraryId,
    pub label: String,
    #[serde(default)]
    pub account: AccountId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_path: Option<PathBuf>,
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        version(self.v, CONFIG_FILE)?;
        label(&self.label)?;
        match (&self.mode, &self.last_path) {
            (Mode::Imported, None) => Ok(()),
            (Mode::Linked, Some(path)) if !path.as_os_str().is_empty() => Ok(()),
            (Mode::Imported, Some(_)) => {
                Err("Imported libraries must not specify last_path".into())
            }
            _ => Err("Linked libraries need last_path in roots.toml".into()),
        }
    }

    pub fn library_path(&self, program: &Path) -> Result<PathBuf, String> {
        self.validate()?;
        match &self.last_path {
            None => Ok(program.join("library")),
            Some(path) => super::disk::absolute(path, program),
        }
    }
}

pub(crate) fn version(value: u32, file: &str) -> Result<(), String> {
    if value == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(format!(
            "{file} uses schema {value}; this app supports schema {SCHEMA_VERSION}"
        ))
    }
}

pub(crate) fn label(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        Err("Give the library a name between 1 and 256 characters".into())
    } else {
        Ok(())
    }
}

/// A snapshot, never an independently cached root. ST2 owns live store reopening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub library: PathBuf,
    pub content: PathBuf,
    pub assets: PathBuf,
    pub userdata: PathBuf,
    pub aidata: PathBuf,
    pub cache: PathBuf,
    pub temp: PathBuf,
}

impl Layout {
    pub fn for_library(library: &Path, account: &AccountId) -> Self {
        let home = library.join("accounts").join(account.as_str());
        Self {
            library: library.to_path_buf(),
            content: home.join("content"),
            assets: home.join("assets"),
            userdata: home.join(".userdata"),
            aidata: home.join(".aidata"),
            cache: library.join("common/cache"),
            temp: library.join("common/temp"),
        }
    }
}

/// Explicit legacy/scratch seams. They never persist config or identity.
#[derive(Clone, Debug)]
pub struct DrillRoots {
    pub content: PathBuf,
    pub assets: Option<PathBuf>,
    pub userdata: Option<PathBuf>,
}

/// How a choice reaches `roots.toml`: a first choice never replaces one; a
/// change made on the Disconnected screen replaces it whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publish {
    New,
    Replace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Unconfigured,
    Connected(Layout),
    Disconnected { reason: String },
    /// Open this session, then unplugged (ST2b). Readers keep the folders
    /// they had and find nothing there; writers are refused until it is back.
    Unplugged { last: Layout, reason: String },
}
