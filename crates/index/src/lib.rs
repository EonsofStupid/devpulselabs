//! The artifact INDEX (X1) — the census's #2 missing capability, ported.
//!
//! LAW: files + their `.meta.json` sidecars are the TRUTH; this DB is a
//! queryable index, rebuilt any time by [`Index::reindex`]. Every capture
//! path writes through; a missing or corrupt DB is recreated, never a
//! data loss. Dump is a SOFT clear — the row survives for history.
//! `ai_labels` is the reserved local-AI column: an entry point exists,
//! and NOTHING here ever synthesizes a label.
//!
//! Engine: TURSO (the pure-Rust SQLite, operator-ratified — the same
//! engine the old repo proved). Its async API stays INSIDE this crate:
//! a current-thread tokio runtime + a fresh connection per op (a local
//! file opens cheaply; the old repo's exact pattern), so every public
//! method here is plain sync.
//!
//! Lives at `content_root()/artifacts.db`. (The wider data-root law is
//! Program P; this crate takes an explicit path everywhere and only
//! `open_default` names the location.)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use turso::Value;

/// The 15-field artifact record — the old repo's wire shape, verbatim.
/// This is what sidecars serialize and every surface (Gallery, reviewer,
/// exporter) reads.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Record {
    /// Forward-slash path relative to the content root ("inbox/pix.png").
    pub rel_path: String,
    /// "inbox" | "content".
    pub location: String,
    /// "image" | "video" | "guide" | …
    pub kind: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_folder: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
    #[serde(default)]
    pub captured_at: String,
    #[serde(default)]
    pub ai_labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster_rel_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_kb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_ms: Option<u64>,
}

/// What a reindex did — honest numbers for the CLI/status line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReindexReport {
    pub indexed: usize,
    pub cleared: usize,
}

pub struct Index {
    path: PathBuf,
    /// The private async seam — turso's futures run here and nowhere else.
    rt: tokio::runtime::Runtime,
}

/// Column order IS the `row_to_record` index order — keep the two in step.
const RECORD_COLS: &str = "rel_path, location, kind, name, show_id, episode_folder, tags, \
     source, target, seconds, captured_at, ai_labels, poster_rel_path, size_kb, modified_ms";

impl Index {
    /// Open (creating/migrating as needed). A corrupt file steps aside as
    /// `.corrupt` and a fresh index is built — files stay the truth.
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("index dir: {e}"))?;
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|e| format!("index runtime: {e}"))?;
        let ix = Self {
            path: path.to_path_buf(),
            rt,
        };
        match ix.migrate() {
            Ok(()) => Ok(ix),
            Err(_) => {
                let corrupt = path.with_extension("db.corrupt");
                let _ = std::fs::remove_file(&corrupt);
                std::fs::rename(path, &corrupt).map_err(|e| format!("index quarantine: {e}"))?;
                ix.migrate()?;
                Ok(ix)
            }
        }
    }

    /// The app's index: `content_root()/artifacts.db`.
    pub fn open_default() -> Result<Self, String> {
        Self::open(&dpl_engine_capture::content_root().join("artifacts.db"))
    }

    async fn connect(path: &Path) -> Result<turso::Connection, String> {
        let db = turso::Builder::new_local(&path.to_string_lossy())
            .build()
            .await
            .map_err(|e| e.to_string())?;
        db.connect().map_err(|e| e.to_string())
    }

    fn migrate(&self) -> Result<(), String> {
        self.rt.block_on(async {
            let conn = Self::connect(&self.path).await?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS artifacts (
                   id INTEGER PRIMARY KEY,
                   rel_path TEXT UNIQUE NOT NULL,
                   location TEXT NOT NULL,
                   kind TEXT NOT NULL,
                   name TEXT NOT NULL,
                   show_id TEXT,
                   episode_folder TEXT,
                   tags TEXT NOT NULL DEFAULT '[]',
                   source TEXT NOT NULL DEFAULT '',
                   target TEXT,
                   seconds INTEGER,
                   captured_at TEXT NOT NULL DEFAULT '',
                   ai_labels TEXT NOT NULL DEFAULT '[]',
                   cleared_at TEXT,
                   poster_rel_path TEXT,
                   size_kb INTEGER,
                   modified_ms INTEGER
                 )",
                (),
            )
            .await
            .map_err(|e| e.to_string())?;
            // Additive migrations, best-effort — a duplicate column is a
            // harmless error; the index is rebuildable BY DESIGN. (The
            // base schema already carries every current column; this loop
            // is where the NEXT column lands.)
            for ddl in [] as [&str; 0] {
                let _ = conn.execute(ddl, ()).await;
            }
            Ok(())
        })
    }

    /// Upsert (write-through from every capture path). A previously
    /// soft-cleared row REACTIVATES — the file is demonstrably back.
    pub fn upsert(&self, r: &Record) -> Result<(), String> {
        let tags = serde_json::to_string(&r.tags).unwrap_or_else(|_| "[]".into());
        let labels = serde_json::to_string(&r.ai_labels).unwrap_or_else(|_| "[]".into());
        self.rt.block_on(async {
            let conn = Self::connect(&self.path).await?;
            conn.execute(
                "INSERT INTO artifacts (rel_path, location, kind, name, show_id,
                   episode_folder, tags, source, target, seconds, captured_at,
                   ai_labels, poster_rel_path, size_kb, modified_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
                 ON CONFLICT(rel_path) DO UPDATE SET location=?2, kind=?3,
                   name=?4, show_id=?5, episode_folder=?6, tags=?7, source=?8,
                   target=?9, seconds=?10, captured_at=?11, ai_labels=?12,
                   poster_rel_path=?13, size_kb=?14, modified_ms=?15,
                   cleared_at=NULL",
                turso::params![
                    r.rel_path.clone(),
                    r.location.clone(),
                    r.kind.clone(),
                    r.name.clone(),
                    r.show_id.clone(),
                    r.episode_folder.clone(),
                    tags,
                    r.source.clone(),
                    r.target.clone(),
                    r.seconds,
                    r.captured_at.clone(),
                    labels,
                    r.poster_rel_path.clone(),
                    // SQLite INTEGER is i64 and turso has no u64 binding —
                    // cast at the boundary (KB sizes and ms timestamps sit
                    // far from i64::MAX).
                    r.size_kb.map(|v| v as i64),
                    r.modified_ms.map(|v| v as i64),
                ],
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    /// Keep moved a file into an episode: re-key + content-locate the row.
    pub fn rekey(
        &self,
        old_rel: &str,
        new_rel: &str,
        show_id: Option<&str>,
        episode_folder: Option<&str>,
    ) -> Result<(), String> {
        self.rt.block_on(async {
            let conn = Self::connect(&self.path).await?;
            conn.execute(
                "UPDATE artifacts SET rel_path=?2, location='content', show_id=?3,
                   episode_folder=?4 WHERE rel_path=?1",
                turso::params![
                    old_rel.to_string(),
                    new_rel.to_string(),
                    show_id.map(str::to_string),
                    episode_folder.map(str::to_string),
                ],
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    /// Dump = SOFT clear: the row survives for history; the file is gone.
    pub fn soft_clear(&self, rel_path: &str) -> Result<(), String> {
        self.rt.block_on(async {
            let conn = Self::connect(&self.path).await?;
            conn.execute(
                "UPDATE artifacts SET cleared_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE rel_path = ?1",
                turso::params![rel_path.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    /// Operator tag edit; the caller also rewrites the sidecar (the file
    /// tier stays the truth).
    pub fn set_tags(&self, rel_path: &str, tags: &[String]) -> Result<(), String> {
        let json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".into());
        self.execute_pair(
            "UPDATE artifacts SET tags=?2 WHERE rel_path=?1",
            rel_path,
            json,
        )
    }

    /// The reserved local-AI entry point. NOTHING in this workspace calls
    /// it yet — labels are never synthesized (provider law).
    pub fn set_ai_labels(&self, rel_path: &str, labels: &[String]) -> Result<(), String> {
        let json = serde_json::to_string(labels).unwrap_or_else(|_| "[]".into());
        self.execute_pair(
            "UPDATE artifacts SET ai_labels=?2 WHERE rel_path=?1",
            rel_path,
            json,
        )
    }

    fn execute_pair(&self, sql: &str, rel_path: &str, json: String) -> Result<(), String> {
        self.rt.block_on(async {
            let conn = Self::connect(&self.path).await?;
            conn.execute(sql, turso::params![rel_path.to_string(), json])
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    /// Newest active rows first, optionally filtered by location.
    pub fn recent(&self, limit: usize, location: Option<&str>) -> Result<Vec<Record>, String> {
        let sql = format!(
            "SELECT {RECORD_COLS} FROM artifacts
             WHERE cleared_at IS NULL AND (?1 IS NULL OR location = ?1)
             ORDER BY modified_ms DESC, id DESC LIMIT ?2"
        );
        self.query(
            sql,
            turso::params![location.map(str::to_string), limit as i64],
        )
    }

    /// Active rows, filterable by kind and/or location, newest first.
    pub fn list(&self, kind: Option<&str>, location: Option<&str>) -> Result<Vec<Record>, String> {
        let sql = format!(
            "SELECT {RECORD_COLS} FROM artifacts
             WHERE cleared_at IS NULL AND (?1 IS NULL OR kind = ?1)
             AND (?2 IS NULL OR location = ?2)
             ORDER BY modified_ms DESC, id DESC LIMIT 10000"
        );
        self.query(
            sql,
            turso::params![kind.map(str::to_string), location.map(str::to_string)],
        )
    }

    fn query(&self, sql: String, params: impl turso::IntoParams) -> Result<Vec<Record>, String> {
        self.rt.block_on(async {
            let conn = Self::connect(&self.path).await?;
            let mut rows = conn.query(&sql, params).await.map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                out.push(row_to_record(&row));
            }
            Ok(out)
        })
    }

    fn active_count(&self) -> Result<usize, String> {
        self.rt.block_on(async {
            let conn = Self::connect(&self.path).await?;
            let mut rows = conn
                .query(
                    "SELECT COUNT(*) FROM artifacts WHERE cleared_at IS NULL",
                    (),
                )
                .await
                .map_err(|e| e.to_string())?;
            let row = rows
                .next()
                .await
                .map_err(|e| e.to_string())?
                .ok_or("count returned no row")?;
            Ok(int(&row, 0).unwrap_or(0) as usize)
        })
    }

    /// DUMP an inbox artifact (X2): delete the file (a guide folder
    /// recursively) + its companions (sidecar; a video's poster), then
    /// SOFT-clear the row (history survives). Jailed to `root/inbox` BY
    /// CONSTRUCTION — a rel_path outside it is refused, never resolved.
    pub fn dump(&self, root: &Path, rel_path: &str) -> Result<(), String> {
        let target = jail_inbox(root, rel_path)?;
        if target.is_dir() {
            // A guide folder: everything inside it (frames, guide.json,
            // artifact.meta.json) goes together.
            std::fs::remove_dir_all(&target).map_err(|e| format!("dump: {e}"))?;
        } else {
            std::fs::remove_file(&target).map_err(|e| format!("dump: {e}"))?;
            let side = sidecar::path_for(&target);
            let _ = std::fs::remove_file(side);
            // The poster companion travels with its video.
            if kind_of(&target) == Some("video") {
                let _ = std::fs::remove_file(target.with_extension("jpg"));
            }
        }
        self.soft_clear(rel_path)
    }

    /// Rebuild from the disk TRUTH: walk `root/inbox`, merge each find
    /// with its sidecar (sidecar wins for authored fields; the fs supplies
    /// size/modified), upsert everything, and soft-clear inbox rows whose
    /// file vanished.
    ///
    /// EMPTY-SCAN GUARD: a scan that finds nothing while the index holds
    /// active rows ABORTS — an unmounted or mistyped root must never
    /// erase history.
    pub fn reindex(&self, root: &Path) -> Result<ReindexReport, String> {
        let found = scan_inbox(root)?;
        if found.is_empty() && self.active_count()? > 0 {
            return Err(
                "reindex refused: the scan found nothing but the index has rows \
                 (wrong root? unmounted drive?)"
                    .into(),
            );
        }
        for r in &found {
            self.upsert(r)?;
        }
        let live: std::collections::BTreeSet<&str> =
            found.iter().map(|r| r.rel_path.as_str()).collect();
        let mut cleared = 0;
        for row in self.list(None, Some("inbox"))? {
            if !live.contains(row.rel_path.as_str()) {
                self.soft_clear(&row.rel_path)?;
                cleared += 1;
            }
        }
        Ok(ReindexReport {
            indexed: found.len(),
            cleared,
        })
    }
}

/// The inbox JAIL: only `inbox/<name...>` rel paths, no traversal or
/// absolute components, and the resolved target must exist UNDER the
/// canonical inbox (symlink escapes included).
fn jail_inbox(root: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let rel = Path::new(rel_path);
    let mut parts = rel.components();
    if parts.next() != Some(std::path::Component::Normal("inbox".as_ref())) {
        return Err(format!("dump refused: {rel_path:?} is not in the inbox"));
    }
    if !parts.all(|c| matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("dump refused: invalid path {rel_path:?}"));
    }
    let inbox = root
        .join("inbox")
        .canonicalize()
        .map_err(|e| format!("dump: inbox missing: {e}"))?;
    let target = root
        .join(rel)
        .canonicalize()
        .map_err(|e| format!("dump: not found: {rel_path:?} ({e})"))?;
    if !target.starts_with(&inbox) {
        return Err(format!("dump refused: {rel_path:?} escapes the inbox"));
    }
    Ok(target)
}

// ── turso value readers (pre-1.0 FromValue set is narrow; read the enum) ─

fn text(row: &turso::Row, idx: usize) -> Option<String> {
    match row.get_value(idx).ok()? {
        Value::Text(s) => Some(s),
        _ => None,
    }
}

fn int(row: &turso::Row, idx: usize) -> Option<i64> {
    match row.get_value(idx).ok()? {
        Value::Integer(i) => Some(i),
        _ => None,
    }
}

fn row_to_record(row: &turso::Row) -> Record {
    Record {
        rel_path: text(row, 0).unwrap_or_default(),
        location: text(row, 1).unwrap_or_default(),
        kind: text(row, 2).unwrap_or_default(),
        name: text(row, 3).unwrap_or_default(),
        show_id: text(row, 4),
        episode_folder: text(row, 5),
        tags: json_vec(text(row, 6).unwrap_or_default()),
        source: text(row, 7).unwrap_or_default(),
        target: text(row, 8),
        seconds: int(row, 9).map(|i| i as u32),
        captured_at: text(row, 10).unwrap_or_default(),
        ai_labels: json_vec(text(row, 11).unwrap_or_default()),
        poster_rel_path: text(row, 12),
        size_kb: int(row, 13).map(|i| i as u64),
        modified_ms: int(row, 14).map(|i| i as u64),
    }
}

fn json_vec(s: String) -> Vec<String> {
    serde_json::from_str(&s).unwrap_or_default()
}

// ── sidecars — the truth that travels with the media ────────────────────

pub mod sidecar {
    use super::Record;
    use std::path::{Path, PathBuf};

    /// A media FILE's sidecar sits beside it (`pix.png.meta.json`); a
    /// guide FOLDER's sidecar lives inside it (`artifact.meta.json` —
    /// guide.json stays the steps manifest, untouched).
    pub fn path_for(media: &Path) -> PathBuf {
        if media.is_dir() {
            media.join("artifact.meta.json")
        } else {
            let mut s = media.as_os_str().to_owned();
            s.push(".meta.json");
            PathBuf::from(s)
        }
    }

    pub fn write(media: &Path, record: &Record) -> Result<(), String> {
        let mut json = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
        json.push('\n');
        std::fs::write(path_for(media), json).map_err(|e| format!("sidecar: {e}"))
    }

    pub fn read(media: &Path) -> Option<Record> {
        let raw = std::fs::read_to_string(path_for(media)).ok()?;
        serde_json::from_str(&raw).ok()
    }
}

// ── the disk scan (reindex's truth feed) ────────────────────────────────

fn kind_of(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|e| e.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "webp" | "gif" => Some("image"),
        "mp4" | "webm" | "mkv" => Some("video"),
        _ => None,
    }
}

/// Scan `root/inbox` for artifacts: media files (skipping sidecars,
/// transients, and a video's poster jpg) and guide folders (a subfolder
/// holding guide.json). Fs facts + sidecar-authored fields merge into
/// full Records.
fn scan_inbox(root: &Path) -> Result<Vec<Record>, String> {
    let inbox = root.join("inbox");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&inbox) else {
        return Ok(out); // no inbox yet = an honest empty scan
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if path.join("guide.json").is_file() {
                out.push(record_for(&path, root, "guide", &name));
            }
            continue;
        }
        if name.ends_with(".meta.json")
            || name.ends_with(".tmp")
            || name.ends_with(".bak")
            || name == "artifacts.db"
        {
            continue;
        }
        let Some(kind) = kind_of(&path) else { continue };
        // The poster law: `vix.jpg` beside `vix.mp4` is a companion, not
        // an artifact of its own.
        if kind == "image" {
            let stem = path.with_extension("");
            if ["mp4", "webm", "mkv"]
                .iter()
                .any(|v| stem.with_extension(v).is_file())
            {
                continue;
            }
        }
        out.push(record_for(&path, root, kind, &name));
    }
    Ok(out)
}

/// Build the record for one on-disk find: fs facts first, then the
/// sidecar's authored fields WIN where present.
fn record_for(path: &Path, root: &Path, kind: &str, file_name: &str) -> Record {
    let rel = rel_path(path, root);
    let (size_kb, modified_ms) = fs_facts(path);
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name);
    let mut r = Record {
        rel_path: rel,
        location: "inbox".into(),
        kind: kind.into(),
        name: stem.into(),
        size_kb,
        modified_ms,
        captured_at: modified_ms.map(iso_from_ms).unwrap_or_default(),
        ..Record::default()
    };
    if kind == "video" {
        let poster = path.with_extension("jpg");
        if poster.is_file() {
            r.poster_rel_path = Some(rel_path(&poster, root));
        }
    }
    if let Some(side) = sidecar::read(path) {
        // Authored truth wins; fs facts stay fresh.
        r.name = side.name;
        r.tags = side.tags;
        r.source = side.source;
        r.target = side.target;
        r.seconds = side.seconds;
        r.show_id = side.show_id;
        r.episode_folder = side.episode_folder;
        r.ai_labels = side.ai_labels;
        if !side.captured_at.is_empty() {
            r.captured_at = side.captured_at;
        }
        if side.poster_rel_path.is_some() {
            r.poster_rel_path = side.poster_rel_path;
        }
    }
    r
}

fn rel_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn fs_facts(path: &Path) -> (Option<u64>, Option<u64>) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (None, None);
    };
    let size_kb = Some(meta.len() / 1024);
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    (size_kb, modified_ms)
}

/// Epoch millis → "YYYY-MM-DDTHH:MM:SSZ" without a chrono dependency
/// (Howard Hinnant's civil-from-days).
pub fn iso_from_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (h, m, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };
    format!("{y:04}-{mth:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// The record a capture site writes through at save time.
pub fn record_for_capture(path: &Path, root: &Path, kind: &str, source: &str) -> Record {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut r = record_for(path, root, kind, &file_name);
    r.source = source.into();
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-index-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("inbox")).unwrap();
        dir
    }

    fn rec(rel: &str, kind: &str) -> Record {
        Record {
            rel_path: rel.into(),
            location: "inbox".into(),
            kind: kind.into(),
            name: rel.rsplit('/').next().unwrap_or(rel).into(),
            modified_ms: Some(1_753_000_000_000),
            ..Record::default()
        }
    }

    #[test]
    fn open_twice_is_idempotent_and_upsert_round_trips() {
        let root = scratch("open");
        let db = root.join("artifacts.db");
        {
            let ix = Index::open(&db).unwrap();
            ix.upsert(&rec("inbox/a.png", "image")).unwrap();
        }
        let ix = Index::open(&db).unwrap();
        let rows = ix.recent(10, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rel_path, "inbox/a.png");
        assert_eq!(rows[0].kind, "image");
    }

    #[test]
    fn soft_clear_hides_and_upsert_reactivates() {
        let root = scratch("clear");
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.upsert(&rec("inbox/a.png", "image")).unwrap();
        ix.soft_clear("inbox/a.png").unwrap();
        assert!(ix.recent(10, None).unwrap().is_empty(), "cleared = hidden");
        // The file came back — the row REACTIVATES.
        ix.upsert(&rec("inbox/a.png", "image")).unwrap();
        assert_eq!(ix.recent(10, None).unwrap().len(), 1);
    }

    #[test]
    fn rekey_moves_the_row_into_content() {
        let root = scratch("rekey");
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.upsert(&rec("inbox/a.png", "image")).unwrap();
        ix.rekey(
            "inbox/a.png",
            "shows/eos/ep1/generated/a.png",
            Some("eos"),
            Some("ep1"),
        )
        .unwrap();
        let rows = ix.list(None, Some("content")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rel_path, "shows/eos/ep1/generated/a.png");
        assert_eq!(rows[0].show_id.as_deref(), Some("eos"));
    }

    #[test]
    fn sidecars_write_read_and_win_on_reindex() {
        let root = scratch("sidecar");
        let media = root.join("inbox").join("pix.png");
        std::fs::write(&media, b"png-bytes").unwrap();
        let mut authored = rec("inbox/pix.png", "image");
        authored.name = "gpu-explainer".into();
        authored.tags = vec!["ai".into(), "gpu".into()];
        authored.source = "pixvix".into();
        sidecar::write(&media, &authored).unwrap();
        assert_eq!(sidecar::read(&media).unwrap().name, "gpu-explainer");

        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        let report = ix.reindex(&root).unwrap();
        assert_eq!(report.indexed, 1);
        let rows = ix.recent(10, None).unwrap();
        assert_eq!(rows[0].name, "gpu-explainer", "the sidecar is the truth");
        assert_eq!(rows[0].tags, vec!["ai", "gpu"]);
        assert!(rows[0].size_kb.is_some(), "fs facts still supplied");
    }

    #[test]
    fn reindex_finds_guides_skips_posters_and_soft_clears_vanished() {
        let root = scratch("reindex");
        let inbox = root.join("inbox");
        std::fs::write(inbox.join("pix.png"), b"img").unwrap();
        std::fs::write(inbox.join("vix.mp4"), b"vid").unwrap();
        std::fs::write(inbox.join("vix.jpg"), b"poster").unwrap();
        std::fs::create_dir_all(inbox.join("guide-1")).unwrap();
        std::fs::write(inbox.join("guide-1").join("guide.json"), "{}").unwrap();

        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        let report = ix.reindex(&root).unwrap();
        assert_eq!(report.indexed, 3, "pix + vix + guide; poster skipped");
        let kinds: Vec<String> = ix
            .recent(10, None)
            .unwrap()
            .into_iter()
            .map(|r| r.kind)
            .collect();
        assert!(kinds.contains(&"guide".to_string()));
        let video = ix.list(Some("video"), None).unwrap();
        assert_eq!(
            video[0].poster_rel_path.as_deref(),
            Some("inbox/vix.jpg"),
            "the poster rides its video"
        );

        // A file vanishes → exactly its row soft-clears.
        std::fs::remove_file(inbox.join("pix.png")).unwrap();
        let report = ix.reindex(&root).unwrap();
        assert_eq!(
            report,
            ReindexReport {
                indexed: 2,
                cleared: 1
            }
        );
        assert_eq!(ix.recent(10, None).unwrap().len(), 2);
    }

    #[test]
    fn the_empty_scan_guard_refuses_to_erase_history() {
        let root = scratch("guard");
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.upsert(&rec("inbox/a.png", "image")).unwrap();
        // Point at a root with an EMPTY inbox — the guard must refuse.
        let empty = scratch("guard-empty");
        let err = ix.reindex(&empty).unwrap_err();
        assert!(err.contains("refused"), "{err}");
        assert_eq!(ix.recent(10, None).unwrap().len(), 1, "history intact");
    }

    #[test]
    fn a_corrupt_db_is_quarantined_not_fatal() {
        let root = scratch("corrupt");
        let db = root.join("artifacts.db");
        std::fs::write(&db, b"this is not sqlite").unwrap();
        let ix = Index::open(&db).unwrap();
        ix.upsert(&rec("inbox/a.png", "image")).unwrap();
        assert!(db.with_extension("db.corrupt").exists(), "quarantined");
    }

    // ── dump + the inbox jail (X2) ──────────────────────────────────────

    #[test]
    fn dump_removes_the_file_its_companions_and_soft_clears_only_it() {
        let root = scratch("dump");
        let inbox = root.join("inbox");
        std::fs::write(inbox.join("pix.png"), b"img").unwrap();
        std::fs::write(inbox.join("keep.png"), b"img2").unwrap();
        std::fs::write(inbox.join("vix.mp4"), b"vid").unwrap();
        std::fs::write(inbox.join("vix.jpg"), b"poster").unwrap();
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        for f in ["pix.png", "keep.png"] {
            sidecar::write(&inbox.join(f), &rec(&format!("inbox/{f}"), "image")).unwrap();
        }
        ix.reindex(&root).unwrap();
        assert_eq!(ix.recent(10, None).unwrap().len(), 3);

        ix.dump(&root, "inbox/pix.png").unwrap();
        assert!(!inbox.join("pix.png").exists());
        assert!(
            !inbox.join("pix.png.meta.json").exists(),
            "sidecar went too"
        );
        assert!(inbox.join("keep.png").exists(), "neighbors untouched");
        assert_eq!(ix.recent(10, None).unwrap().len(), 2, "row soft-cleared");

        ix.dump(&root, "inbox/vix.mp4").unwrap();
        assert!(!inbox.join("vix.jpg").exists(), "the poster rides along");
    }

    #[test]
    fn dump_takes_a_guide_folder_recursively() {
        let root = scratch("dump-guide");
        let guide = root.join("inbox").join("guide-1");
        std::fs::create_dir_all(&guide).unwrap();
        std::fs::write(guide.join("guide.json"), "{}").unwrap();
        std::fs::write(guide.join("step-001.webp"), b"frame").unwrap();
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.reindex(&root).unwrap();
        ix.dump(&root, "inbox/guide-1").unwrap();
        assert!(!guide.exists(), "the whole folder went");
        assert!(ix.recent(10, None).unwrap().is_empty());
    }

    #[test]
    fn the_jail_refuses_everything_outside_the_inbox() {
        let root = scratch("jail");
        std::fs::create_dir_all(root.join("shows")).unwrap();
        std::fs::write(root.join("shows").join("x.png"), b"content").unwrap();
        std::fs::write(root.join("inbox").join("a.png"), b"img").unwrap();
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        for bad in [
            "shows/x.png",
            "inbox/../shows/x.png",
            "../etc/passwd",
            "C:/windows/system32",
            "inbox/./../shows/x.png",
        ] {
            let err = ix.dump(&root, bad).unwrap_err();
            assert!(err.contains("refused"), "{bad} must refuse: {err}");
        }
        assert!(root.join("shows").join("x.png").exists(), "content intact");
        // A vanished target is an honest not-found, not a panic.
        assert!(ix
            .dump(&root, "inbox/ghost.png")
            .unwrap_err()
            .contains("not found"));
    }

    #[test]
    fn iso_from_ms_formats_real_dates() {
        // 2026-07-23T00:00:00Z == 1784764800000 ms.
        assert_eq!(iso_from_ms(1_784_764_800_000), "2026-07-23T00:00:00Z");
        assert_eq!(iso_from_ms(0), "1970-01-01T00:00:00Z");
    }
}
