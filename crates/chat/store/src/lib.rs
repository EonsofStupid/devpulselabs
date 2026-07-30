//! Bounded, read-only provider history discovery and paging.
//!
//! Provider crates own schema interpretation. This crate owns the filesystem
//! jail, resource ceilings, opaque session resolution, and cursor mechanics.

use dpl_chat_core::{
    BoundedNativePayload, ChatEvent, ChatEventContent, ChatEventId, ChatSessionId, ProviderId,
    ProviderSessionId, SessionState, SessionSummary, TokenUsage, TurnId,
};
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryLimits {
    pub max_depth: u8,
    pub max_files: usize,
    pub prefix_bytes: usize,
    pub page_bytes: usize,
    pub page_events: usize,
    pub max_line_bytes: usize,
}

impl Default for HistoryLimits {
    fn default() -> Self {
        Self {
            max_depth: 6,
            max_files: 512,
            prefix_bytes: 64 * 1024,
            page_bytes: 256 * 1024,
            page_events: 200,
            max_line_bytes: 1024 * 1024,
        }
    }
}

impl HistoryLimits {
    fn normalized(self) -> Self {
        Self {
            max_depth: self.max_depth.max(1),
            max_files: self.max_files.max(1),
            prefix_bytes: self.prefix_bytes.clamp(1024, 1024 * 1024),
            page_bytes: self.page_bytes.clamp(4 * 1024, 4 * 1024 * 1024),
            page_events: self.page_events.clamp(1, 1_000),
            max_line_bytes: self.max_line_bytes.clamp(4 * 1024, 4 * 1024 * 1024),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryIndexMetadata {
    pub native_id: ProviderSessionId,
    pub title: String,
    pub workspace: String,
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHistoryEvent {
    pub turn_id: Option<TurnId>,
    pub content: ChatEventContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeOutcome {
    Event(DecodedHistoryEvent),
    IgnoreKnown,
}

pub trait HistoryDecoder {
    fn provider_id(&self) -> ProviderId;
    fn index_metadata(&self, path: &Path, prefix: &str) -> Option<HistoryIndexMetadata>;
    fn decode_line(&self, line: &str) -> DecodeOutcome;
}

/// What a provider is ABLE to report about token usage.
///
/// Declared, never discovered. This is the honesty mechanism: a surface
/// can say "Grok does not report an input/output split" instead of
/// rendering a silent zero, and a half-implemented provider stays legible
/// instead of looking broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageReporting {
    /// Separate prompt and completion counts.
    pub input_output_split: bool,
    /// A single combined total.
    pub total_tokens: bool,
    /// Context-window occupancy (used / size).
    pub context_window: bool,
    /// The provider computes real money itself — trusted over our table.
    pub provider_priced: bool,
    /// The provider names the model it is running.
    pub model: bool,
    /// Usage can be recovered from ALREADY-STORED sessions, so the
    /// backfill pass can walk this provider's history and recover cost
    /// the operator has already spent. False for live-only transports
    /// (Grok's ACP), where usage exists on the wire and nowhere else.
    pub history_backfill: bool,
}

impl UsageReporting {
    /// A provider that reports nothing yet. A truthful state, not a bug —
    /// it makes "not implemented" legible and turns adding support into
    /// filling in one file rather than discovering an absence.
    pub const NOTHING: Self = Self {
        input_output_split: false,
        total_tokens: false,
        context_window: false,
        provider_priced: false,
        model: false,
        history_backfill: false,
    };

    pub const fn reports_anything(self) -> bool {
        self.input_output_split || self.total_tokens || self.context_window
    }
}

/// ONE provider's usage dialect.
///
/// Providers report different things, in different shapes, and CHANGE
/// them between releases. Each adapter owns its own dialect so a new
/// provider — or a new release of an existing one — extends HERE without
/// touching any other provider's code.
///
/// Follows `HistoryDecoder`'s discipline exactly: `&str` in, typed out,
/// so this crate and `dpl-chat-core` stay dependency-free and every
/// provider keeps parsing its native format with its own serde.
pub trait ProviderUsage: Send + Sync {
    fn provider_id(&self) -> ProviderId;

    /// What this provider can report. Callers consult this BEFORE
    /// treating an absent number as meaningful.
    fn reports(&self) -> UsageReporting;

    /// Token accounting carried by one native record, if any.
    fn usage_from_line(&self, line: &str) -> Option<TokenUsage>;

    /// The model named by one native record, if any.
    fn model_from_line(&self, _line: &str) -> Option<String> {
        None
    }

    /// Cost the PROVIDER itself computed, in micros. When a provider
    /// reports real money (Claude does), it is authoritative over any
    /// price table of ours — it knows its own billing, we infer it.
    fn cost_micros_from_line(&self, _line: &str) -> Option<u64> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryCursor {
    before_byte: u64,
}

impl HistoryCursor {
    pub const fn before_byte(self) -> u64 {
        self.before_byte
    }
}

/// Opaque store-owned identity for one provider history artifact.
///
/// Provider-native session IDs are metadata, not unique file locators: a
/// provider may persist the same logical session across multiple artifacts.
/// Callers can retain this value, but cannot use it to name arbitrary paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HistoryLocator(u64);

impl HistoryLocator {
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for HistoryLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "history-{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSession {
    pub locator: HistoryLocator,
    pub summary: SessionSummary,
    pub started_at: String,
    pub modified_millis: u64,
    pub source_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryIndex {
    pub provider_id: ProviderId,
    pub sessions: Vec<IndexedSession>,
    pub scanned_files: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPage {
    pub provider_id: ProviderId,
    pub locator: HistoryLocator,
    pub session_id: ChatSessionId,
    pub native_id: ProviderSessionId,
    pub events: Vec<ChatEvent>,
    pub next_cursor: Option<HistoryCursor>,
    pub source_bytes: u64,
    pub truncated_before: bool,
}

#[derive(Debug)]
pub enum StoreError {
    SourceUnavailable {
        provider: ProviderId,
        reason: String,
    },
    DuplicateSource(ProviderId),
    UnknownSource(ProviderId),
    InvalidRoot(PathBuf),
    EscapedRoot(PathBuf),
    SessionMissing {
        provider: ProviderId,
        locator: HistoryLocator,
    },
    SessionChanged {
        native_id: ProviderSessionId,
    },
    RecordTooLarge {
        native_id: ProviderSessionId,
        at_byte: u64,
        limit: usize,
    },
    IdentityCollision {
        provider: ProviderId,
        session_id: ChatSessionId,
    },
    Io {
        operation: &'static str,
        message: String,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceUnavailable { provider, reason } => {
                write!(f, "{provider} history is unavailable: {reason}")
            }
            Self::DuplicateSource(provider) => {
                write!(f, "history source {provider} was registered twice")
            }
            Self::UnknownSource(provider) => {
                write!(f, "history source {provider} is not registered")
            }
            Self::InvalidRoot(path) => {
                write!(f, "history root is not a readable directory: {path:?}")
            }
            Self::EscapedRoot(path) => {
                write!(f, "history entry escapes its canonical root: {path:?}")
            }
            Self::SessionMissing { provider, locator } => {
                write!(f, "{provider} {locator} is not available")
            }
            Self::SessionChanged { native_id } => {
                write!(
                    f,
                    "session {:?} changed while it was being paged",
                    native_id.0
                )
            }
            Self::RecordTooLarge {
                native_id,
                at_byte,
                limit,
            } => write!(
                f,
                "session {:?} has a record larger than {limit} bytes near byte {at_byte}",
                native_id.0
            ),
            Self::IdentityCollision {
                provider,
                session_id,
            } => write!(
                f,
                "{provider} history produced duplicate identity {session_id}"
            ),
            Self::Io { operation, message } => write!(f, "{operation}: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

pub trait HistorySource {
    fn provider_id(&self) -> ProviderId;
    fn list(&self, workspace_filter: Option<&str>) -> Result<HistoryIndex, StoreError>;
    fn page(
        &self,
        locator: HistoryLocator,
        cursor: Option<HistoryCursor>,
    ) -> Result<HistoryPage, StoreError>;
}

struct ReadJail {
    canonical_root: PathBuf,
}

impl ReadJail {
    fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(StoreError::InvalidRoot(root.to_path_buf()));
        }
        let canonical_root = root.canonicalize().map_err(|error| StoreError::Io {
            operation: "canonicalize history root",
            message: error.to_string(),
        })?;
        Ok(Self { canonical_root })
    }

    fn checked_file(&self, path: &Path) -> Result<PathBuf, StoreError> {
        let canonical = path.canonicalize().map_err(|error| StoreError::Io {
            operation: "canonicalize history entry",
            message: error.to_string(),
        })?;
        if !canonical.starts_with(&self.canonical_root) {
            return Err(StoreError::EscapedRoot(canonical));
        }
        Ok(canonical)
    }

    fn jsonl_files(&self, limits: HistoryLimits) -> Result<(Vec<PathBuf>, bool), StoreError> {
        let mut files = Vec::new();
        let mut truncated = false;
        self.walk(
            &self.canonical_root,
            limits.max_depth,
            limits.max_files,
            &mut files,
            &mut truncated,
        )?;
        Ok((files, truncated))
    }

    fn walk(
        &self,
        directory: &Path,
        depth: u8,
        max_files: usize,
        files: &mut Vec<PathBuf>,
        truncated: &mut bool,
    ) -> Result<(), StoreError> {
        if depth == 0 || *truncated {
            return Ok(());
        }
        let entries = std::fs::read_dir(directory).map_err(|error| StoreError::Io {
            operation: "read history directory",
            message: error.to_string(),
        })?;
        let mut entries =
            entries
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| StoreError::Io {
                    operation: "read history entry",
                    message: error.to_string(),
                })?;
        entries.sort_by_key(|right| std::cmp::Reverse(right.file_name()));
        for entry in entries {
            let file_type = entry.file_type().map_err(|error| StoreError::Io {
                operation: "inspect history entry",
                message: error.to_string(),
            })?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                let canonical = path.canonicalize().map_err(|error| StoreError::Io {
                    operation: "canonicalize history directory",
                    message: error.to_string(),
                })?;
                if !canonical.starts_with(&self.canonical_root) {
                    return Err(StoreError::EscapedRoot(canonical));
                }
                self.walk(&canonical, depth - 1, max_files, files, truncated)?;
            } else if file_type.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("jsonl")
            {
                if files.len() == max_files {
                    *truncated = true;
                    return Ok(());
                }
                files.push(self.checked_file(&path)?);
            }
        }
        Ok(())
    }
}

pub struct JsonlHistorySource {
    jail: ReadJail,
    decoder: Box<dyn HistoryDecoder>,
    limits: HistoryLimits,
}

impl JsonlHistorySource {
    pub fn open(
        root: impl AsRef<Path>,
        decoder: impl HistoryDecoder + 'static,
        limits: HistoryLimits,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            jail: ReadJail::open(root)?,
            decoder: Box::new(decoder),
            limits: limits.normalized(),
        })
    }

    fn files(&self) -> Result<(Vec<PathBuf>, bool), StoreError> {
        self.jail.jsonl_files(self.limits)
    }

    fn metadata_for(&self, path: &Path) -> Result<Option<HistoryIndexMetadata>, StoreError> {
        let prefix = read_prefix(path, self.limits.prefix_bytes)?;
        Ok(self
            .decoder
            .index_metadata(path, &String::from_utf8_lossy(&prefix)))
    }

    fn resolve(
        &self,
        locator: HistoryLocator,
    ) -> Result<(PathBuf, HistoryIndexMetadata), StoreError> {
        let (files, _) = self.files()?;
        for path in files {
            if self.locator_for(&path)? != locator {
                continue;
            }
            if let Some(metadata) = self.metadata_for(&path)? {
                return Ok((path, metadata));
            }
        }
        Err(StoreError::SessionMissing {
            provider: self.provider_id(),
            locator,
        })
    }

    fn locator_for(&self, path: &Path) -> Result<HistoryLocator, StoreError> {
        let relative = path
            .strip_prefix(&self.jail.canonical_root)
            .map_err(|_| StoreError::EscapedRoot(path.to_path_buf()))?;
        let normalized = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        // Windows paths are case-insensitive; lowercase only there. Shadow
        // with a mutable local so the base binding stays immutable (no
        // unused-mut on non-Windows, no #[allow] — ADR: the gate is -D warnings).
        #[cfg(windows)]
        let normalized = {
            let mut normalized = normalized;
            normalized.make_ascii_lowercase();
            normalized
        };
        Ok(HistoryLocator(stable_hash(&[
            self.provider_id().0.as_bytes(),
            b"\0history\0",
            normalized.as_bytes(),
        ])))
    }
}

impl HistorySource for JsonlHistorySource {
    fn provider_id(&self) -> ProviderId {
        self.decoder.provider_id()
    }

    fn list(&self, workspace_filter: Option<&str>) -> Result<HistoryIndex, StoreError> {
        let (files, mut truncated) = self.files()?;
        let scanned_files = files.len();
        let normalized_filter = workspace_filter.map(str::to_lowercase);
        let mut sessions = Vec::new();
        let mut ids = HashSet::new();

        for path in files {
            let Some(metadata) = self.metadata_for(&path)? else {
                continue;
            };
            if normalized_filter
                .as_ref()
                .is_some_and(|filter| !metadata.workspace.to_lowercase().contains(filter.as_str()))
            {
                continue;
            }
            let locator = self.locator_for(&path)?;
            let session_id = stable_session_id(self.provider_id(), locator);
            if !ids.insert(session_id) {
                return Err(StoreError::IdentityCollision {
                    provider: self.provider_id(),
                    session_id,
                });
            }
            let file_metadata = std::fs::metadata(&path).map_err(|error| StoreError::Io {
                operation: "inspect history file",
                message: error.to_string(),
            })?;
            let modified_millis = file_metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_millis() as u64);
            sessions.push(IndexedSession {
                locator,
                summary: SessionSummary {
                    id: session_id,
                    provider_id: self.provider_id(),
                    provider_session_id: Some(metadata.native_id),
                    title: metadata.title,
                    workspace: metadata.workspace,
                    group: dpl_chat_core::SessionGroup::Recent,
                    state: SessionState::ReadOnly,
                },
                started_at: metadata.started_at,
                modified_millis,
                source_bytes: file_metadata.len(),
            });
            if sessions.len() == self.limits.max_files {
                truncated = true;
                break;
            }
        }
        sessions.sort_by(|left, right| right.modified_millis.cmp(&left.modified_millis));
        Ok(HistoryIndex {
            provider_id: self.provider_id(),
            sessions,
            scanned_files,
            truncated,
        })
    }

    fn page(
        &self,
        locator: HistoryLocator,
        cursor: Option<HistoryCursor>,
    ) -> Result<HistoryPage, StoreError> {
        let (path, metadata) = self.resolve(locator)?;
        let native_id = metadata.native_id;
        let mut file = File::open(&path).map_err(|error| StoreError::Io {
            operation: "open history file",
            message: error.to_string(),
        })?;
        let source_bytes = file
            .metadata()
            .map_err(|error| StoreError::Io {
                operation: "inspect history file",
                message: error.to_string(),
            })?
            .len();
        let before = cursor.map_or(source_bytes, |value| value.before_byte);
        if before > source_bytes {
            return Err(StoreError::SessionChanged {
                native_id: native_id.clone(),
            });
        }

        let mut window_bytes = self.limits.page_bytes;
        let (aligned_start, buffer, data_offset) = loop {
            let requested_start = before.saturating_sub(window_bytes as u64);
            file.seek(SeekFrom::Start(requested_start))
                .map_err(|error| StoreError::Io {
                    operation: "seek history page",
                    message: error.to_string(),
                })?;
            let byte_count = (before - requested_start) as usize;
            let mut buffer = vec![0_u8; byte_count];
            file.read_exact(&mut buffer)
                .map_err(|error| StoreError::Io {
                    operation: "read history page",
                    message: error.to_string(),
                })?;
            if requested_start == 0 {
                break (0_u64, buffer, 0_usize);
            }
            if let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
                let offset = newline + 1;
                break (requested_start + offset as u64, buffer, offset);
            }
            if window_bytes == self.limits.max_line_bytes {
                return Err(StoreError::RecordTooLarge {
                    native_id: native_id.clone(),
                    at_byte: requested_start,
                    limit: self.limits.max_line_bytes,
                });
            }
            window_bytes = window_bytes
                .saturating_mul(2)
                .min(self.limits.max_line_bytes);
        };
        let data = &buffer[data_offset..];

        let records = line_records(data, aligned_start);
        let session_id = stable_session_id(self.provider_id(), locator);
        let mut events = Vec::new();
        let mut next_before = if aligned_start > 0 {
            Some(aligned_start)
        } else {
            None
        };

        for record in records.iter().rev() {
            if record.bytes.len() > self.limits.max_line_bytes {
                return Err(StoreError::RecordTooLarge {
                    native_id: native_id.clone(),
                    at_byte: record.offset,
                    limit: self.limits.max_line_bytes,
                });
            }
            let outcome = match std::str::from_utf8(record.bytes) {
                Ok(line) => self.decoder.decode_line(line),
                Err(_) => DecodeOutcome::Event(DecodedHistoryEvent {
                    turn_id: None,
                    content: ChatEventContent::Unknown(BoundedNativePayload::redacted(
                        "invalid-utf8",
                        format!("invalid UTF-8 record at byte {}", record.offset),
                    )),
                }),
            };
            if let DecodeOutcome::Event(decoded) = outcome {
                events.push(ChatEvent {
                    id: stable_event_id(self.provider_id(), locator, record.offset),
                    provider_id: self.provider_id(),
                    session_id,
                    turn_id: decoded.turn_id,
                    content: decoded.content,
                });
                if events.len() == self.limits.page_events {
                    next_before = Some(record.offset);
                    break;
                }
            }
        }
        events.reverse();
        let next_cursor = next_before
            .filter(|offset| *offset > 0)
            .map(|before_byte| HistoryCursor { before_byte });
        Ok(HistoryPage {
            provider_id: self.provider_id(),
            locator,
            session_id,
            native_id,
            events,
            next_cursor,
            source_bytes,
            truncated_before: next_cursor.is_some(),
        })
    }
}

#[derive(Default)]
pub struct HistoryCatalog {
    sources: Vec<Box<dyn HistorySource>>,
}

impl HistoryCatalog {
    pub fn register(&mut self, source: impl HistorySource + 'static) -> Result<(), StoreError> {
        let provider = source.provider_id();
        if self
            .sources
            .iter()
            .any(|existing| existing.provider_id() == provider)
        {
            return Err(StoreError::DuplicateSource(provider));
        }
        self.sources.push(Box::new(source));
        Ok(())
    }

    pub fn has_source(&self, provider: ProviderId) -> bool {
        self.sources
            .iter()
            .any(|source| source.provider_id() == provider)
    }

    pub fn provider_ids(&self) -> impl Iterator<Item = ProviderId> + '_ {
        self.sources.iter().map(|source| source.provider_id())
    }

    pub fn list(
        &self,
        provider: ProviderId,
        workspace_filter: Option<&str>,
    ) -> Result<HistoryIndex, StoreError> {
        self.source(provider)?.list(workspace_filter)
    }

    pub fn page(
        &self,
        provider: ProviderId,
        locator: HistoryLocator,
        cursor: Option<HistoryCursor>,
    ) -> Result<HistoryPage, StoreError> {
        self.source(provider)?.page(locator, cursor)
    }

    fn source(&self, provider: ProviderId) -> Result<&dyn HistorySource, StoreError> {
        self.sources
            .iter()
            .find(|source| source.provider_id() == provider)
            .map(AsRef::as_ref)
            .ok_or(StoreError::UnknownSource(provider))
    }
}

struct LineRecord<'a> {
    offset: u64,
    bytes: &'a [u8],
}

fn line_records(data: &[u8], base_offset: u64) -> Vec<LineRecord<'_>> {
    let mut records = Vec::new();
    let mut start = 0;
    for (index, byte) in data.iter().enumerate() {
        if *byte == b'\n' {
            let mut end = index;
            if end > start && data[end - 1] == b'\r' {
                end -= 1;
            }
            if end > start {
                records.push(LineRecord {
                    offset: base_offset + start as u64,
                    bytes: &data[start..end],
                });
            }
            start = index + 1;
        }
    }
    if start < data.len() {
        records.push(LineRecord {
            offset: base_offset + start as u64,
            bytes: &data[start..],
        });
    }
    records
}

fn read_prefix(path: &Path, limit: usize) -> Result<Vec<u8>, StoreError> {
    let file = File::open(path).map_err(|error| StoreError::Io {
        operation: "open history prefix",
        message: error.to_string(),
    })?;
    let mut bytes = Vec::new();
    file.take(limit as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| StoreError::Io {
            operation: "read history prefix",
            message: error.to_string(),
        })?;
    Ok(bytes)
}

fn stable_session_id(provider: ProviderId, locator: HistoryLocator) -> ChatSessionId {
    ChatSessionId(stable_hash(&[
        provider.0.as_bytes(),
        b"\0session\0",
        &locator.0.to_le_bytes(),
    ]))
}

fn stable_event_id(provider: ProviderId, locator: HistoryLocator, byte_offset: u64) -> ChatEventId {
    ChatEventId(stable_hash(&[
        provider.0.as_bytes(),
        b"\0event\0",
        &locator.0.to_le_bytes(),
        &byte_offset.to_le_bytes(),
    ]))
}

fn stable_hash(parts: &[&[u8]]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in *part {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    const TEST_PROVIDER: ProviderId = ProviderId("store-fixture");

    struct FixtureDecoder;

    impl HistoryDecoder for FixtureDecoder {
        fn provider_id(&self) -> ProviderId {
            TEST_PROVIDER
        }

        fn index_metadata(&self, path: &Path, _prefix: &str) -> Option<HistoryIndexMetadata> {
            Some(HistoryIndexMetadata {
                native_id: ProviderSessionId(path.file_stem()?.to_string_lossy().into_owned()),
                title: path.file_stem()?.to_string_lossy().into_owned(),
                workspace: "C:/fixture".into(),
                started_at: "2026-07-23T00:00:00Z".into(),
            })
        }

        fn decode_line(&self, line: &str) -> DecodeOutcome {
            DecodeOutcome::Event(DecodedHistoryEvent {
                turn_id: None,
                content: ChatEventContent::AssistantMessage(line.to_owned()),
            })
        }
    }

    struct TestTree {
        root: PathBuf,
    }

    impl TestTree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(1);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("devpulse-chat-store-{}-{id}", std::process::id()));
            fs::create_dir_all(&root).expect("create test tree");
            Self { root }
        }
    }

    impl Drop for TestTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn source(tree: &TestTree, page_events: usize) -> JsonlHistorySource {
        JsonlHistorySource::open(
            &tree.root,
            FixtureDecoder,
            HistoryLimits {
                page_events,
                page_bytes: 4 * 1024,
                ..HistoryLimits::default()
            },
        )
        .expect("open fixture history")
    }

    #[test]
    fn discovery_and_pages_are_bounded_and_stable() {
        let tree = TestTree::new();
        fs::write(tree.root.join("session-a.jsonl"), "one\ntwo\nthree\nfour\n")
            .expect("write fixture");
        let source = source(&tree, 2);
        let index = source.list(None).expect("list");
        assert_eq!(index.sessions.len(), 1);
        let locator = index.sessions[0].locator;

        let newest = source.page(locator, None).expect("newest page");
        assert_eq!(newest.events.len(), 2);
        assert!(newest.truncated_before);
        let older = source
            .page(locator, newest.next_cursor)
            .expect("older page");
        assert_eq!(older.events.len(), 2);
        assert!(!older.truncated_before);

        let all: Vec<_> = older
            .events
            .iter()
            .chain(newest.events.iter())
            .map(|event| match &event.content {
                ChatEventContent::AssistantMessage(text) => text.as_str(),
                _ => panic!("unexpected event"),
            })
            .collect();
        assert_eq!(all, ["one", "two", "three", "four"]);
        assert_ne!(older.events[0].id, newest.events[0].id);
    }

    #[test]
    fn cursors_fail_when_a_file_shrinks() {
        let tree = TestTree::new();
        let path = tree.root.join("session-a.jsonl");
        fs::write(&path, "one\ntwo\nthree\n").expect("write fixture");
        let source = source(&tree, 1);
        let locator = source.list(None).expect("index").sessions[0].locator;
        let page = source.page(locator, None).expect("page");
        fs::write(path, "one\n").expect("shrink fixture");
        assert!(matches!(
            source.page(locator, page.next_cursor),
            Err(StoreError::SessionChanged { .. })
        ));
    }

    #[test]
    fn catalogs_reject_duplicate_provider_sources() {
        let left = TestTree::new();
        let right = TestTree::new();
        let mut catalog = HistoryCatalog::default();
        catalog.register(source(&left, 2)).expect("first");
        assert!(matches!(
            catalog.register(source(&right, 2)),
            Err(StoreError::DuplicateSource(TEST_PROVIDER))
        ));
    }

    #[test]
    fn file_count_limits_report_truncation() {
        let tree = TestTree::new();
        fs::write(tree.root.join("a.jsonl"), "a\n").expect("a");
        fs::write(tree.root.join("b.jsonl"), "b\n").expect("b");
        let source = JsonlHistorySource::open(
            &tree.root,
            FixtureDecoder,
            HistoryLimits {
                max_files: 1,
                ..HistoryLimits::default()
            },
        )
        .expect("source");
        let index = source.list(None).expect("index");
        assert_eq!(index.scanned_files, 1);
        assert!(index.truncated);
    }

    #[test]
    fn fifty_thousand_records_stay_in_one_bounded_page() {
        let tree = TestTree::new();
        let mut transcript = String::new();
        for index in 0..50_000 {
            transcript.push_str(&format!("event-{index}\n"));
        }
        fs::write(tree.root.join("large.jsonl"), transcript).expect("large fixture");
        let source = source(&tree, 200);
        let locator = source.list(None).expect("index").sessions[0].locator;
        let page = source.page(locator, None).expect("bounded page");
        assert_eq!(page.events.len(), 200);
        assert!(page.truncated_before);
        assert!(page.next_cursor.is_some());
    }

    #[test]
    fn repeated_provider_native_ids_remain_distinct_history_artifacts() {
        struct RepeatedNativeIdDecoder;

        impl HistoryDecoder for RepeatedNativeIdDecoder {
            fn provider_id(&self) -> ProviderId {
                TEST_PROVIDER
            }

            fn index_metadata(&self, path: &Path, _prefix: &str) -> Option<HistoryIndexMetadata> {
                Some(HistoryIndexMetadata {
                    native_id: ProviderSessionId("shared-native-id".into()),
                    title: path.file_stem()?.to_string_lossy().into_owned(),
                    workspace: "C:/fixture".into(),
                    started_at: "2026-07-23T00:00:00Z".into(),
                })
            }

            fn decode_line(&self, line: &str) -> DecodeOutcome {
                DecodeOutcome::Event(DecodedHistoryEvent {
                    turn_id: None,
                    content: ChatEventContent::AssistantMessage(line.to_owned()),
                })
            }
        }

        let tree = TestTree::new();
        fs::write(tree.root.join("first.jsonl"), "first\n").expect("first");
        fs::write(tree.root.join("second.jsonl"), "second\n").expect("second");
        let source = JsonlHistorySource::open(
            &tree.root,
            RepeatedNativeIdDecoder,
            HistoryLimits::default(),
        )
        .expect("source");

        let index = source.list(None).expect("index");
        assert_eq!(index.sessions.len(), 2);
        assert_ne!(index.sessions[0].locator, index.sessions[1].locator);
        assert_ne!(index.sessions[0].summary.id, index.sessions[1].summary.id);
        assert_eq!(
            index.sessions[0].summary.provider_session_id,
            index.sessions[1].summary.provider_session_id
        );

        let left = source
            .page(index.sessions[0].locator, None)
            .expect("left page");
        let right = source
            .page(index.sessions[1].locator, None)
            .expect("right page");
        assert_ne!(left.session_id, right.session_id);
        assert_ne!(left.events[0].id, right.events[0].id);
    }
}
