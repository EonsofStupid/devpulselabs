//! Non-secret Chat provider activation evidence in local Turso.
//!
//! This ledger records whether a provider connection was last *verified* by
//! its official runtime. It never stores credentials, device codes,
//! authorization URLs, cookies, auth-file contents, or provider output.

use dpl_chat_core::{PermissionMode, ProviderId};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use turso::Value;

const DATABASE_RELATIVE_PATH: &str = "chat/accounts.db";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Ready,
    SignedOut,
    Failed,
}

impl EvidenceState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::SignedOut => "signed_out",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(Self::Ready),
            "signed_out" => Some(Self::SignedOut),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationEvidence {
    pub provider_id: String,
    pub attempt_id: Option<u64>,
    pub state: EvidenceState,
    pub client_version: String,
    pub auth_method: String,
    pub validated_at_ms: Option<u64>,
    pub failure_class: Option<String>,
    pub updated_at_ms: u64,
}

pub struct ActivationLedger {
    path: PathBuf,
    runtime: tokio::runtime::Runtime,
}

struct EvidenceWrite<'a> {
    provider: ProviderId,
    attempt_id: Option<u64>,
    state: EvidenceState,
    client_version: &'a str,
    auth_method: &'a str,
    validated_at_ms: Option<u64>,
    failure_class: Option<String>,
    updated_at_ms: u64,
}

impl ActivationLedger {
    pub fn open_default() -> Result<Self, String> {
        Self::open(&dpl_roots::userdata_root().join(DATABASE_RELATIVE_PATH))
    }

    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Chat account ledger directory: {error}"))?;
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|error| format!("Chat account ledger runtime: {error}"))?;
        let ledger = Self {
            path: path.to_owned(),
            runtime,
        };
        ledger.migrate()?;
        Ok(ledger)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self, provider: ProviderId) -> Result<Option<ActivationEvidence>, String> {
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            let mut rows = connection
                .query(
                    "SELECT provider_id, attempt_id, state, client_version, auth_method,
                            validated_at_ms, failure_class, updated_at_ms
                     FROM chat_provider_activation WHERE provider_id = ?1",
                    turso::params![provider.0.to_owned()],
                )
                .await
                .map_err(|error| error.to_string())?;
            let Some(row) = rows.next().await.map_err(|error| error.to_string())? else {
                return Ok(None);
            };
            let state_text = text(&row, 2).unwrap_or_default();
            let state = EvidenceState::parse(&state_text)
                .ok_or_else(|| format!("unknown activation evidence state `{state_text}`"))?;
            Ok(Some(ActivationEvidence {
                provider_id: text(&row, 0).unwrap_or_default(),
                attempt_id: integer(&row, 1).map(|value| value as u64),
                state,
                client_version: text(&row, 3).unwrap_or_default(),
                auth_method: text(&row, 4).unwrap_or_default(),
                validated_at_ms: integer(&row, 5).map(|value| value as u64),
                failure_class: text(&row, 6),
                updated_at_ms: integer(&row, 7).unwrap_or_default() as u64,
            }))
        })
    }

    /// The operator's permission mode for `provider`, when one was saved.
    pub fn permission_mode(&self, provider: ProviderId) -> Result<Option<PermissionMode>, String> {
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            let mut rows = connection
                .query(
                    "SELECT permission_mode FROM chat_provider_settings WHERE provider_id = ?1",
                    turso::params![provider.0.to_owned()],
                )
                .await
                .map_err(|error| error.to_string())?;
            let Some(row) = rows.next().await.map_err(|error| error.to_string())? else {
                return Ok(None);
            };
            let stored = text(&row, 0).unwrap_or_default();
            PermissionMode::parse(&stored)
                .map(Some)
                .ok_or_else(|| format!("unknown permission mode `{stored}`"))
        })
    }

    /// Save the operator's permission mode for `provider`.
    pub fn set_permission_mode(
        &self,
        provider: ProviderId,
        mode: PermissionMode,
    ) -> Result<(), String> {
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            connection
                .execute(
                    "INSERT INTO chat_provider_settings (provider_id, permission_mode, updated_at_ms)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(provider_id) DO UPDATE SET
                         permission_mode = excluded.permission_mode,
                         updated_at_ms = excluded.updated_at_ms",
                    turso::params![
                        provider.0.to_owned(),
                        mode.as_str().to_owned(),
                        unix_time_ms() as i64,
                    ],
                )
                .await
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }

    pub fn record_ready(
        &self,
        provider: ProviderId,
        client_version: &str,
        auth_method: &str,
    ) -> Result<(), String> {
        self.record_ready_for_attempt(provider, None, client_version, auth_method)
    }

    pub fn record_ready_for_attempt(
        &self,
        provider: ProviderId,
        attempt_id: Option<u64>,
        client_version: &str,
        auth_method: &str,
    ) -> Result<(), String> {
        let now = unix_time_ms();
        self.upsert(EvidenceWrite {
            provider,
            attempt_id,
            state: EvidenceState::Ready,
            client_version,
            auth_method,
            validated_at_ms: Some(now),
            failure_class: None,
            updated_at_ms: now,
        })
    }

    pub fn record_signed_out(
        &self,
        provider: ProviderId,
        client_version: &str,
        auth_method: &str,
    ) -> Result<(), String> {
        self.record_signed_out_for_attempt(provider, None, client_version, auth_method)
    }

    pub fn record_signed_out_for_attempt(
        &self,
        provider: ProviderId,
        attempt_id: Option<u64>,
        client_version: &str,
        auth_method: &str,
    ) -> Result<(), String> {
        self.upsert(EvidenceWrite {
            provider,
            attempt_id,
            state: EvidenceState::SignedOut,
            client_version,
            auth_method,
            validated_at_ms: None,
            failure_class: None,
            updated_at_ms: unix_time_ms(),
        })
    }

    pub fn record_failed(
        &self,
        provider: ProviderId,
        client_version: &str,
        auth_method: &str,
        failure_class: &str,
    ) -> Result<(), String> {
        self.record_failed_for_attempt(provider, None, client_version, auth_method, failure_class)
    }

    pub fn record_failed_for_attempt(
        &self,
        provider: ProviderId,
        attempt_id: Option<u64>,
        client_version: &str,
        auth_method: &str,
        failure_class: &str,
    ) -> Result<(), String> {
        self.upsert(EvidenceWrite {
            provider,
            attempt_id,
            state: EvidenceState::Failed,
            client_version,
            auth_method,
            validated_at_ms: None,
            failure_class: Some(bounded_failure_class(failure_class)),
            updated_at_ms: unix_time_ms(),
        })
    }

    fn upsert(&self, write: EvidenceWrite<'_>) -> Result<(), String> {
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            connection
                .execute(
                    "INSERT INTO chat_provider_activation (
                         provider_id, attempt_id, state, client_version, auth_method,
                         validated_at_ms, failure_class, updated_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT(provider_id) DO UPDATE SET
                         attempt_id = ?2,
                         state = ?3,
                         client_version = ?4,
                         auth_method = ?5,
                         validated_at_ms = ?6,
                         failure_class = ?7,
                         updated_at_ms = ?8",
                    turso::params![
                        write.provider.0.to_owned(),
                        write.attempt_id.map(|value| value as i64),
                        write.state.as_str().to_owned(),
                        write.client_version.to_owned(),
                        write.auth_method.to_owned(),
                        write.validated_at_ms.map(|value| value as i64),
                        write.failure_class,
                        write.updated_at_ms as i64,
                    ],
                )
                .await
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }

    fn migrate(&self) -> Result<(), String> {
        self.runtime.block_on(async {
            let connection = Self::connect(&self.path).await?;
            connection
                .execute(
                    "CREATE TABLE IF NOT EXISTS chat_provider_activation (
                         provider_id TEXT PRIMARY KEY NOT NULL,
                         attempt_id INTEGER,
                         state TEXT NOT NULL,
                         client_version TEXT NOT NULL,
                         auth_method TEXT NOT NULL,
                         validated_at_ms INTEGER,
                         failure_class TEXT,
                         updated_at_ms INTEGER NOT NULL
                     )",
                    (),
                )
                .await
                .map_err(|error| error.to_string())?;
            // The operator's per-provider settings (CW1: the permission
            // mode). Non-secret, like everything in this file.
            connection
                .execute(
                    "CREATE TABLE IF NOT EXISTS chat_provider_settings (
                         provider_id TEXT PRIMARY KEY NOT NULL,
                         permission_mode TEXT NOT NULL,
                         updated_at_ms INTEGER NOT NULL
                     )",
                    (),
                )
                .await
                .map_err(|error| error.to_string())?;
            let mut rows = connection
                .query("PRAGMA table_info(chat_provider_activation)", ())
                .await
                .map_err(|error| error.to_string())?;
            let mut has_attempt_id = false;
            while let Some(row) = rows.next().await.map_err(|error| error.to_string())? {
                if text(&row, 1).as_deref() == Some("attempt_id") {
                    has_attempt_id = true;
                    break;
                }
            }
            if !has_attempt_id {
                connection
                    .execute(
                        "ALTER TABLE chat_provider_activation ADD COLUMN attempt_id INTEGER",
                        (),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })
    }

    async fn connect(path: &Path) -> Result<turso::Connection, String> {
        let database = turso::Builder::new_local(&path.to_string_lossy())
            .build()
            .await
            .map_err(|error| error.to_string())?;
        database.connect().map_err(|error| error.to_string())
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

fn bounded_failure_class(value: &str) -> String {
    value.chars().take(96).collect()
}

fn text(row: &turso::Row, index: usize) -> Option<String> {
    match row.get_value(index).ok()? {
        Value::Text(value) => Some(value),
        _ => None,
    }
}

fn integer(row: &turso::Row, index: usize) -> Option<i64> {
    match row.get_value(index).ok()? {
        Value::Integer(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    const GROK: ProviderId = ProviderId("grok");
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn ledger(name: &str) -> (PathBuf, ActivationLedger) {
        let path = std::env::temp_dir().join(format!(
            "dpl-chat-accounts-{name}-{}-{}.db",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // Windows recycles PIDs, so a leftover db from a dead test
        // process can alias this path and hand the test its own prior
        // run's final state. A fresh fixture starts from nothing.
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(path.with_extension(format!("db{suffix}")));
        }
        let _ = std::fs::remove_file(&path);
        let ledger = ActivationLedger::open(&path).expect("ledger");
        (path, ledger)
    }

    /// The operator's permission setting is remembered per provider, and
    /// survives a restart (a reopened store).
    #[test]
    fn the_permission_mode_survives_reopen_and_replaces_in_place() {
        let (path, ledger) = ledger("permission");
        assert_eq!(ledger.permission_mode(GROK).expect("get"), None);
        ledger
            .set_permission_mode(GROK, PermissionMode::Workspace)
            .expect("set");
        ledger
            .set_permission_mode(GROK, PermissionMode::ReadOnly)
            .expect("replace");
        drop(ledger);
        let reopened = ActivationLedger::open(&path).expect("reopen");
        assert_eq!(
            reopened.permission_mode(GROK).expect("get"),
            Some(PermissionMode::ReadOnly)
        );
    }

    #[test]
    fn evidence_replaces_state_and_survives_reopen() {
        let (path, ledger) = ledger("round-trip");
        assert_eq!(ledger.get(GROK).expect("get"), None);
        ledger
            .record_signed_out(GROK, "0.2.112", "cached_token")
            .expect("signed out");
        assert_eq!(
            ledger.get(GROK).expect("get").expect("record").state,
            EvidenceState::SignedOut
        );
        ledger
            .record_ready_for_attempt(GROK, Some(38), "0.2.112", "cached_token")
            .expect("ready");
        drop(ledger);
        let reopened = ActivationLedger::open(&path).expect("reopen");
        let evidence = reopened.get(GROK).expect("get").expect("record");
        assert_eq!(evidence.state, EvidenceState::Ready);
        assert_eq!(evidence.attempt_id, Some(38));
        assert!(evidence.validated_at_ms.is_some());
        assert_eq!(evidence.failure_class, None);
    }

    #[test]
    fn failure_metadata_is_bounded_and_contains_no_secret_columns() {
        let (path, ledger) = ledger("bounded");
        ledger
            .record_failed(GROK, "0.2.112", "device_code", &"x".repeat(200))
            .expect("failed");
        let evidence = ledger.get(GROK).expect("get").expect("record");
        assert_eq!(evidence.state, EvidenceState::Failed);
        assert_eq!(evidence.auth_method, "device_code");
        assert_eq!(evidence.failure_class.expect("failure").len(), 96);

        let columns = ledger
            .runtime
            .block_on(async {
                let connection = ActivationLedger::connect(&path).await?;
                let mut rows = connection
                    .query("PRAGMA table_info(chat_provider_activation)", ())
                    .await
                    .map_err(|error| error.to_string())?;
                let mut columns = Vec::new();
                while let Some(row) = rows.next().await.map_err(|error| error.to_string())? {
                    columns.push(text(&row, 1).unwrap_or_default());
                }
                Ok::<_, String>(columns)
            })
            .expect("schema columns");
        for forbidden in [
            "access_token",
            "refresh_token",
            "device_code",
            "auth_url",
            "cookie",
        ] {
            assert!(
                !columns.iter().any(|column| column == forbidden),
                "{forbidden} must not be a column"
            );
        }
    }
}
