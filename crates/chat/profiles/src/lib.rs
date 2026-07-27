//! Non-secret, operator-authored Chat provider profiles.
//!
//! A profile selects one reviewed configuration kind. It cannot carry an
//! executable, arguments, environment, shell, workspace, or credential.

use dpl_chat_core::{ProviderAccent, ProviderId};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use turso::Value;

const DATABASE_RELATIVE_PATH: &str = "chat/provider-profiles.db";
const MAX_DISPLAY_NAME_CHARS: usize = 64;
const MAX_ENDPOINT_CHARS: usize = 2_048;
const MAX_MODEL_CHARS: usize = 128;

pub const CLYFFY_PROVIDER: ProviderId = ProviderId("clyffy");
pub const LOCAL_PROVIDER: ProviderId = ProviderId("local");
pub const CUSTOM_PROVIDER: ProviderId = ProviderId("custom");

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProviderProfileKey(String);

impl ProviderProfileKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ProfileError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 96
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(ProfileError::InvalidKey);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProviderProfileKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderConfigurationKind {
    Local,
    Custom,
}

impl ProviderConfigurationKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Local => "Local provider",
            Self::Custom => "Custom provider",
        }
    }

    pub const fn provider_id(self) -> ProviderId {
        match self {
            Self::Local => LOCAL_PROVIDER,
            Self::Custom => CUSTOM_PROVIDER,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Custom => "custom",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "local" => Some(Self::Local),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfileDraft {
    pub kind: ProviderConfigurationKind,
    pub display_name: String,
    pub endpoint: String,
    pub model: String,
}

impl ProviderProfileDraft {
    pub fn for_kind(kind: ProviderConfigurationKind) -> Self {
        Self {
            kind,
            display_name: kind.label().to_owned(),
            endpoint: match kind {
                ProviderConfigurationKind::Local => "http://127.0.0.1:11434".to_owned(),
                ProviderConfigurationKind::Custom => "https://".to_owned(),
            },
            model: String::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        validate_text(
            &self.display_name,
            MAX_DISPLAY_NAME_CHARS,
            ProfileError::InvalidDisplayName,
        )?;
        validate_text(
            &self.endpoint,
            MAX_ENDPOINT_CHARS,
            ProfileError::InvalidEndpoint,
        )?;
        if !self.model.is_empty() {
            validate_text(&self.model, MAX_MODEL_CHARS, ProfileError::InvalidModel)?;
        }
        match self.kind {
            ProviderConfigurationKind::Local if !is_loopback_http_endpoint(&self.endpoint) => {
                Err(ProfileError::LocalEndpointMustBeLoopback)
            }
            ProviderConfigurationKind::Custom if !is_https_endpoint(&self.endpoint) => {
                Err(ProfileError::CustomEndpointMustUseHttps)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfile {
    pub key: ProviderProfileKey,
    pub kind: ProviderConfigurationKind,
    pub display_name: String,
    pub endpoint: String,
    pub model: String,
}

impl ProviderProfile {
    pub fn from_draft(
        key: ProviderProfileKey,
        draft: ProviderProfileDraft,
    ) -> Result<Self, ProfileError> {
        draft.validate()?;
        Ok(Self {
            key,
            kind: draft.kind,
            display_name: draft.display_name.trim().to_owned(),
            endpoint: draft.endpoint.trim().trim_end_matches('/').to_owned(),
            model: draft.model.trim().to_owned(),
        })
    }

    pub const fn activation_reason(&self) -> &'static str {
        "Profile saved. Its compatible transport adapter is not qualified yet."
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSupplementAction {
    FutureAdapter,
    Configure(ProviderConfigurationKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogSupplement {
    pub provider_id: ProviderId,
    pub label: &'static str,
    pub short_label: &'static str,
    pub glyph: &'static str,
    pub accent: ProviderAccent,
    pub summary: &'static str,
    pub action: CatalogSupplementAction,
}

pub const SUPPLEMENTS: &[CatalogSupplement] = &[
    CatalogSupplement {
        provider_id: CLYFFY_PROVIDER,
        label: "Clyffy",
        short_label: "Clyffy",
        glyph: "C",
        accent: ProviderAccent::Lime,
        summary: "DevPULSE-native orchestrator adapter; declared but not qualified.",
        action: CatalogSupplementAction::FutureAdapter,
    },
    CatalogSupplement {
        provider_id: LOCAL_PROVIDER,
        label: "Add local provider",
        short_label: "Local",
        glyph: "+",
        accent: ProviderAccent::Cyan,
        summary: "Create a loopback-only provider profile from a typed configuration.",
        action: CatalogSupplementAction::Configure(ProviderConfigurationKind::Local),
    },
    CatalogSupplement {
        provider_id: CUSTOM_PROVIDER,
        label: "Add custom provider",
        short_label: "Custom",
        glyph: "+",
        accent: ProviderAccent::Amber,
        summary: "Create an HTTPS provider profile from a typed configuration.",
        action: CatalogSupplementAction::Configure(ProviderConfigurationKind::Custom),
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    InvalidKey,
    InvalidDisplayName,
    InvalidEndpoint,
    InvalidModel,
    LocalEndpointMustBeLoopback,
    CustomEndpointMustUseHttps,
    UnknownConfigurationKind(String),
    Storage(String),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => f.write_str("profile identity is invalid"),
            Self::InvalidDisplayName => {
                write!(
                    f,
                    "display name must be 1-{MAX_DISPLAY_NAME_CHARS} safe characters"
                )
            }
            Self::InvalidEndpoint => {
                write!(f, "endpoint must be 1-{MAX_ENDPOINT_CHARS} safe characters")
            }
            Self::InvalidModel => {
                write!(f, "model must be at most {MAX_MODEL_CHARS} safe characters")
            }
            Self::LocalEndpointMustBeLoopback => {
                f.write_str("local providers require http(s)://localhost or a loopback IP")
            }
            Self::CustomEndpointMustUseHttps => {
                f.write_str("custom providers require an HTTPS endpoint")
            }
            Self::UnknownConfigurationKind(value) => {
                write!(f, "unknown provider configuration kind `{value}`")
            }
            Self::Storage(message) => write!(f, "provider profile storage: {message}"),
        }
    }
}

impl std::error::Error for ProfileError {}

pub struct ProfileStore {
    path: PathBuf,
    runtime: tokio::runtime::Runtime,
}

impl ProfileStore {
    pub fn open_default() -> Result<Self, ProfileError> {
        Self::open(&dpl_engine_capture::userdata_root().join(DATABASE_RELATIVE_PATH))
    }

    pub fn open(path: &Path) -> Result<Self, ProfileError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| ProfileError::Storage(error.to_string()))?;
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|error| ProfileError::Storage(error.to_string()))?;
        let store = Self {
            path: path.to_owned(),
            runtime,
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn list(&self) -> Result<Vec<ProviderProfile>, ProfileError> {
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            let mut rows = connection
                .query(
                    "SELECT profile_key, kind, display_name, endpoint, model
                     FROM chat_provider_profiles ORDER BY updated_at_ms, profile_key",
                    (),
                )
                .await
                .map_err(storage)?;
            let mut profiles = Vec::new();
            while let Some(row) = rows.next().await.map_err(storage)? {
                let kind_text = text(&row, 1).unwrap_or_default();
                let kind = ProviderConfigurationKind::parse(&kind_text)
                    .ok_or_else(|| ProfileError::UnknownConfigurationKind(kind_text.clone()))?;
                let profile = ProviderProfile {
                    key: ProviderProfileKey::new(text(&row, 0).unwrap_or_default())?,
                    kind,
                    display_name: text(&row, 2).unwrap_or_default(),
                    endpoint: text(&row, 3).unwrap_or_default(),
                    model: text(&row, 4).unwrap_or_default(),
                };
                ProviderProfileDraft {
                    kind: profile.kind,
                    display_name: profile.display_name.clone(),
                    endpoint: profile.endpoint.clone(),
                    model: profile.model.clone(),
                }
                .validate()?;
                profiles.push(profile);
            }
            Ok(profiles)
        })
    }

    pub fn create(&self, draft: ProviderProfileDraft) -> Result<ProviderProfile, ProfileError> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| ProfileError::Storage(error.to_string()))?
            .as_micros();
        let key = ProviderProfileKey::new(format!("{}-{stamp}", draft.kind.as_str()))?;
        let profile = ProviderProfile::from_draft(key, draft)?;
        self.upsert(&profile)?;
        Ok(profile)
    }

    pub fn upsert(&self, profile: &ProviderProfile) -> Result<(), ProfileError> {
        ProviderProfileDraft {
            kind: profile.kind,
            display_name: profile.display_name.clone(),
            endpoint: profile.endpoint.clone(),
            model: profile.model.clone(),
        }
        .validate()?;
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            connection
                .execute(
                    "INSERT INTO chat_provider_profiles (
                         profile_key, kind, display_name, endpoint, model, updated_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(profile_key) DO UPDATE SET
                         kind = ?2,
                         display_name = ?3,
                         endpoint = ?4,
                         model = ?5,
                         updated_at_ms = ?6",
                    turso::params![
                        profile.key.as_str().to_owned(),
                        profile.kind.as_str().to_owned(),
                        profile.display_name.clone(),
                        profile.endpoint.clone(),
                        profile.model.clone(),
                        unix_time_ms() as i64,
                    ],
                )
                .await
                .map_err(storage)?;
            Ok(())
        })
    }

    fn migrate(&self) -> Result<(), ProfileError> {
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            connection
                .execute(
                    "CREATE TABLE IF NOT EXISTS chat_provider_profiles (
                         profile_key TEXT PRIMARY KEY NOT NULL,
                         kind TEXT NOT NULL,
                         display_name TEXT NOT NULL,
                         endpoint TEXT NOT NULL,
                         model TEXT NOT NULL,
                         updated_at_ms INTEGER NOT NULL
                     )",
                    (),
                )
                .await
                .map_err(storage)?;
            Ok(())
        })
    }

    async fn connect(path: &Path) -> Result<turso::Connection, ProfileError> {
        let database = turso::Builder::new_local(&path.to_string_lossy())
            .build()
            .await
            .map_err(storage)?;
        database.connect().map_err(storage)
    }
}

fn validate_text(value: &str, limit: usize, error: ProfileError) -> Result<(), ProfileError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > limit
        || trimmed.chars().any(char::is_control)
    {
        return Err(error);
    }
    Ok(())
}

fn is_loopback_http_endpoint(value: &str) -> bool {
    let value = value.trim();
    let Some(authority) = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .and_then(|rest| rest.split('/').next())
    else {
        return false;
    };
    if authority.contains('@') {
        return false;
    }
    let host = authority
        .strip_prefix('[')
        .and_then(|rest| rest.split(']').next())
        .unwrap_or_else(|| authority.split(':').next().unwrap_or_default());
    matches!(host.to_ascii_lowercase().as_str(), "localhost" | "::1") || is_ipv4_loopback(host)
}

fn is_https_endpoint(value: &str) -> bool {
    let Some(authority) = value
        .trim()
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
    else {
        return false;
    };
    if authority.is_empty()
        || authority.contains('@')
        || authority.chars().any(char::is_whitespace)
        || authority.contains(['?', '#'])
    {
        return false;
    }
    let host = authority.split(':').next().unwrap_or_default();
    let labels = host.split('.').collect::<Vec<_>>();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn is_ipv4_loopback(host: &str) -> bool {
    let octets = host
        .split('.')
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>();
    octets.is_ok_and(|octets| octets.len() == 4 && octets[0] == 127)
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

fn text(row: &turso::Row, index: usize) -> Option<String> {
    match row.get_value(index).ok()? {
        Value::Text(value) => Some(value),
        _ => None,
    }
}

fn storage(error: impl fmt::Display) -> ProfileError {
    ProfileError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn local_profiles_are_loopback_only() {
        let mut draft = ProviderProfileDraft::for_kind(ProviderConfigurationKind::Local);
        assert_eq!(draft.validate(), Ok(()));
        draft.endpoint = "http://192.168.1.10:11434".into();
        assert_eq!(
            draft.validate(),
            Err(ProfileError::LocalEndpointMustBeLoopback)
        );
        draft.endpoint = "http://127.0.0.1.evil.example".into();
        assert_eq!(
            draft.validate(),
            Err(ProfileError::LocalEndpointMustBeLoopback)
        );
        draft.endpoint = "http://operator@example.com".into();
        assert_eq!(
            draft.validate(),
            Err(ProfileError::LocalEndpointMustBeLoopback)
        );
    }

    #[test]
    fn custom_profiles_require_https_without_inline_credentials() {
        let mut draft = ProviderProfileDraft::for_kind(ProviderConfigurationKind::Custom);
        draft.endpoint = "https://ai.example.com/v1".into();
        assert_eq!(draft.validate(), Ok(()));
        draft.endpoint = "http://ai.example.com/v1".into();
        assert_eq!(
            draft.validate(),
            Err(ProfileError::CustomEndpointMustUseHttps)
        );
        draft.endpoint = "https://ai example.com/v1".into();
        assert_eq!(
            draft.validate(),
            Err(ProfileError::CustomEndpointMustUseHttps)
        );
        draft.endpoint = "https://operator:secret@ai.example.com/v1".into();
        assert_eq!(
            draft.validate(),
            Err(ProfileError::CustomEndpointMustUseHttps)
        );
    }

    #[test]
    fn non_secret_profiles_round_trip() {
        let path = std::env::temp_dir().join(format!(
            "dpl-chat-profiles-{}-{}.db",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let store = ProfileStore::open(&path).expect("store");
        let mut draft = ProviderProfileDraft::for_kind(ProviderConfigurationKind::Local);
        draft.display_name = "Studio model".into();
        draft.model = "devpulse-8b".into();
        let saved = store.create(draft).expect("create");
        drop(store);

        let reopened = ProfileStore::open(&path).expect("reopen");
        assert_eq!(reopened.list().expect("list"), vec![saved]);
    }
}
