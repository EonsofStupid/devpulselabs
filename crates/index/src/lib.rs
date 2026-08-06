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
//! Lives at `.userdata/index.db` (ADR-014 Ring 0); a legacy
//! `content_root()/artifacts.db` is ADOPTED additively on first open —
//! copy → verify → labeled rename, never deletion. This crate takes an
//! explicit path everywhere and only `open_default` names the location.

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

    /// The app's index: `.userdata/index.db` (ADR-014 Ring 0), adopting
    /// a legacy `content/artifacts.db` if one is still in place.
    pub fn open_default() -> Result<Self, String> {
        let root = dpl_engine_capture::content_root();
        Self::open_adopting(
            &dpl_engine_capture::userdata_root().join("index.db"),
            &root.join("artifacts.db"),
        )
    }

    /// ADR-014 condition 6 — the ADDITIVE migration: when `path` doesn't
    /// exist but `legacy` does, copy → verify the copy opens → retire the
    /// legacy files under a `.migrated-to-userdata` label. Never deletion;
    /// a failed copy or verify leaves the legacy files untouched.
    ///
    /// The copy carries EVERY sidecar (`-wal`, `-shm`): turso's local WAL
    /// holds recent writes, and a main-file-only copy silently loses them
    /// (caught live by the adoption unit test).
    pub fn open_adopting(path: &Path, legacy: &Path) -> Result<Self, String> {
        const SIDECARS: [&str; 3] = ["", "-wal", "-shm"];
        if !path.exists() && legacy.is_file() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("index dir: {e}"))?;
            }
            for suffix in SIDECARS {
                let from = Self::append_suffix(legacy, suffix);
                if from.is_file() {
                    std::fs::copy(&from, Self::append_suffix(path, suffix))
                        .map_err(|e| format!("index adopt copy{suffix}: {e}"))?;
                }
            }
            let ix = Self::open(path)?;
            for suffix in SIDECARS {
                let from = Self::append_suffix(legacy, suffix);
                if from.is_file() {
                    let retired = Self::append_suffix(&from, ".migrated-to-userdata");
                    let _ = std::fs::remove_file(&retired);
                    std::fs::rename(&from, &retired)
                        .map_err(|e| format!("index adopt retire{suffix}: {e}"))?;
                }
            }
            return Ok(ix);
        }
        Self::open(path)
    }

    fn append_suffix(p: &Path, suffix: &str) -> PathBuf {
        let mut os = p.as_os_str().to_os_string();
        os.push(suffix);
        PathBuf::from(os)
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
    /// Unlink a symlink ITSELF, never what it points at.
    ///
    /// Windows has two kinds of symlink and they are removed by different
    /// calls: a DIRECTORY symlink must go through `remove_dir`, and
    /// `remove_file` refuses it outright. Unix has one kind, which
    /// `remove_file` unlinks. Neither call follows the link on either OS, so
    /// the operator's original is untouched whichever branch runs.
    ///
    /// This existed as a bare `remove_file`, which meant a REFERENCED folder
    /// could never be dumped on Windows — the operator's remove would fail and
    /// the link would stay in their inbox. The file-symlink case worked, which
    /// is why it went unnoticed.
    fn remove_symlink(link: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            // `symlink_metadata` does not follow — we are asking what the LINK
            // is, not what it points at.
            if std::fs::symlink_metadata(link)?
                .file_type()
                .is_symlink_dir()
            {
                return std::fs::remove_dir(link);
            }
        }
        std::fs::remove_file(link)
    }

    pub fn dump(&self, root: &Path, rel_path: &str) -> Result<(), String> {
        let target = jail_inbox(root, rel_path)?;
        // `symlink_metadata` does NOT follow — which is the whole point here.
        // `Path::is_dir()` DOES follow, so a folder symlink would have sent a
        // `remove_dir_all` straight into the operator's own directory.
        let meta = std::fs::symlink_metadata(&target).map_err(|e| format!("dump: {e}"))?;
        if meta.file_type().is_symlink() {
            // A REFERENCED artifact: remove the LINK, never what it points at.
            // The operator's original is somewhere outside `content/` and is
            // not ours to delete — referencing was the promise that we would
            // not copy it, and removing must keep that promise.
            Self::remove_symlink(&target).map_err(|e| format!("dump link: {e}"))?;
            let _ = std::fs::remove_file(sidecar::path_for(&target));
            if kind_of(&target) == Some("video") {
                let _ = std::fs::remove_file(target.with_extension("jpg"));
            }
        } else if meta.is_dir() {
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

    /// KEEP (X3): MOVE an inbox artifact into an EXISTING content episode
    /// — `content/<show>/<episode>/assets/` — companions travelling
    /// (sidecar; a video's poster; a guide folder whole), the sidecar
    /// rewritten, the row re-keyed. Keep never creates shows: a missing
    /// destination episode is refused (the old filing law). The source is
    /// inbox-jailed like dump. Returns the new rel_path.
    pub fn keep(
        &self,
        root: &Path,
        rel_path: &str,
        show: &str,
        episode: &str,
    ) -> Result<String, String> {
        self.file_into_episode(root, rel_path, show, episode, FileOp::Move)
    }

    /// ASSOCIATE (X3): COPY, never move — the library master (file, row,
    /// sidecar) stays exactly where it is; the copy gets its own sidecar
    /// and its own row. Same destination law as keep.
    pub fn associate(
        &self,
        root: &Path,
        rel_path: &str,
        show: &str,
        episode: &str,
    ) -> Result<String, String> {
        self.file_into_episode(root, rel_path, show, episode, FileOp::Copy)
    }

    fn file_into_episode(
        &self,
        root: &Path,
        rel_path: &str,
        show: &str,
        episode: &str,
        op: FileOp,
    ) -> Result<String, String> {
        let source = jail_inbox(root, rel_path)?;
        let ep_dir = root.join(show).join(episode);
        if !ep_dir.join("episode.json").is_file() {
            return Err(format!(
                "refused: {show}/{episode} is not an existing episode (keep never creates shows)"
            ));
        }
        let assets = ep_dir.join("assets");
        std::fs::create_dir_all(&assets).map_err(|e| format!("assets: {e}"))?;
        let file_name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| format!("no file name in {rel_path:?}"))?;

        if source.is_dir() {
            // A folder artifact (guide, session) travels WHOLE — frames,
            // manifest, and its artifact.meta.json sidecar are inside it.
            let kind = folder_kind(&source).unwrap_or("folder");
            let dest = never_clobber(&assets, &file_name, false);
            match op {
                FileOp::Move => {
                    std::fs::rename(&source, &dest).map_err(|e| format!("keep: {e}"))?
                }
                FileOp::Copy => copy_dir(&source, &dest)?,
            }
            let new_rel = rel_path_of(&dest, root);
            let mut r = record_for(&dest, root, kind, &file_name);
            self.relocate(&dest, &mut r, &new_rel, show, episode)?;
            if op == FileOp::Move {
                self.rekey(rel_path, &new_rel, Some(show), Some(episode))?;
            }
            self.upsert(&r)?;
            return Ok(new_rel);
        }

        // A media file: authored truth is read BEFORE the move, then the
        // file + companions land under one never-clobbered stem (the
        // poster law needs `vix-2.jpg` beside `vix-2.mp4`).
        let authored = sidecar::read(&source);
        let kind = kind_of(&source).unwrap_or("file");
        let dest = never_clobber(&assets, &file_name, true);
        transfer(&source, &dest, op)?;
        let mut poster_rel = None;
        if kind == "video" {
            let poster = source.with_extension("jpg");
            if poster.is_file() {
                let poster_dest = dest.with_extension("jpg");
                transfer(&poster, &poster_dest, op)?;
                poster_rel = Some(rel_path_of(&poster_dest, root));
            }
        }
        if op == FileOp::Move {
            let _ = std::fs::remove_file(sidecar::path_for(&source));
        }

        let dest_name = dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or(file_name);
        let mut r = record_for(&dest, root, kind, &dest_name);
        if let Some(side) = authored {
            r.name = side.name;
            r.tags = side.tags;
            r.source = side.source;
            r.target = side.target;
            r.seconds = side.seconds;
            r.ai_labels = side.ai_labels;
            if !side.captured_at.is_empty() {
                r.captured_at = side.captured_at;
            }
        }
        r.poster_rel_path = poster_rel;
        let new_rel = r.rel_path.clone();
        self.relocate(&dest, &mut r, &new_rel, show, episode)?;
        if op == FileOp::Move {
            self.rekey(rel_path, &new_rel, Some(show), Some(episode))?;
        }
        self.upsert(&r)?;
        Ok(new_rel)
    }

    /// Stamp the content location onto a record and write its sidecar —
    /// the file tier stays the truth.
    fn relocate(
        &self,
        media: &Path,
        r: &mut Record,
        new_rel: &str,
        show: &str,
        episode: &str,
    ) -> Result<(), String> {
        r.rel_path = new_rel.to_string();
        r.location = "content".into();
        r.show_id = Some(show.to_string());
        r.episode_folder = Some(episode.to_string());
        sidecar::write(media, r)
    }
}

/// Keep vs associate — the ONLY difference is move vs copy.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FileOp {
    Move,
    Copy,
}

fn transfer(from: &Path, to: &Path, op: FileOp) -> Result<(), String> {
    match op {
        FileOp::Move => std::fs::rename(from, to).map_err(|e| format!("keep: {e}")),
        FileOp::Copy => std::fs::copy(from, to)
            .map(|_| ())
            .map_err(|e| format!("associate: {e}")),
    }
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("copy {}: {e}", to.display()))?;
    let entries = std::fs::read_dir(from).map_err(|e| format!("copy {}: {e}", from.display()))?;
    for entry in entries.flatten() {
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| format!("copy {}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// The never-clobber name: `pix.png` → `pix-2.png` → `pix-3.png`…
/// (`split_ext: false` for folders — a guide folder's dots are content,
/// not an extension).
fn never_clobber(dir: &Path, name: &str, split_ext: bool) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = if split_ext {
        match name.rsplit_once('.') {
            Some((s, e)) => (s.to_string(), Some(e.to_string())),
            None => (name.to_string(), None),
        }
    } else {
        (name.to_string(), None)
    };
    let mut n = 2u32;
    loop {
        let candidate = match &ext {
            Some(e) => dir.join(format!("{stem}-{n}.{e}")),
            None => dir.join(format!("{stem}-{n}")),
        };
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// The inbox JAIL (dump's source AND keep/associate's source): only
/// `inbox/<name...>` rel paths, no traversal or absolute components, and
/// the resolved target must exist UNDER the canonical inbox (symlink
/// escapes included).
fn jail_inbox(root: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let rel = Path::new(rel_path);
    let mut parts = rel.components();
    if parts.next() != Some(std::path::Component::Normal("inbox".as_ref())) {
        return Err(format!("refused: {rel_path:?} is not in the inbox"));
    }
    // Every remaining component must be a plain name — no `..`, no root, no
    // prefix. That is what makes the LEXICAL jail below sound.
    if !parts.all(|c| matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("refused: invalid path {rel_path:?}"));
    }
    let linkish = root.join(rel);

    // A REFERENCED artifact (`import_reference`) is a symlink INSIDE the inbox
    // whose target is the operator's own file, deliberately outside
    // `content/`. Canonicalizing it would point at that original — which made
    // the containment check below refuse it, so a referenced clip could never
    // be removed at all ("escapes the inbox", which reads like a security
    // refusal rather than "this is a link"). Worse, had the check ever passed,
    // dump would have deleted the operator's source.
    //
    // So a symlink is jailed LEXICALLY — the component check above already
    // proves it cannot escape — and returned AS THE LINK, never resolved.
    if std::fs::symlink_metadata(&linkish)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Ok(linkish);
    }

    let inbox = root
        .join("inbox")
        .canonicalize()
        .map_err(|e| format!("inbox missing: {e}"))?;
    let target = linkish
        .canonicalize()
        .map_err(|e| format!("not found: {rel_path:?} ({e})"))?;
    if !target.starts_with(&inbox) {
        return Err(format!("refused: {rel_path:?} escapes the inbox"));
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

/// What a FOLDER artifact is, by its marker file: a guide (guide.json)
/// or a session (session.mp4 / events.jsonl — a partial session with
/// only the event track still counts; files are the truth). None = not
/// an artifact folder.
fn folder_kind(dir: &Path) -> Option<&'static str> {
    if dir.join("guide.json").is_file() {
        Some("guide")
    } else if dir.join("session.mp4").is_file() || dir.join("events.jsonl").is_file() {
        Some("session")
    } else {
        None
    }
}

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
            if let Some(kind) = folder_kind(&path) {
                out.push(record_for(&path, root, kind, &name));
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
    let rel = rel_path_of(path, root);
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
            r.poster_rel_path = Some(rel_path_of(&poster, root));
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

fn rel_path_of(path: &Path, root: &Path) -> String {
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
    fn adoption_migrates_labels_and_never_reruns() {
        let root = scratch("adopt");
        let legacy = root.join("artifacts.db");
        let new = root.join(".userdata").join("index.db");
        {
            let ix = Index::open(&legacy).unwrap();
            ix.upsert(&rec("inbox/old.png", "image")).unwrap();
        }
        // Old-only → copied, verified, legacy retired under a label.
        let ix = Index::open_adopting(&new, &legacy).unwrap();
        assert_eq!(ix.recent(10, None).unwrap()[0].rel_path, "inbox/old.png");
        assert!(new.is_file(), "new home exists");
        assert!(!legacy.exists(), "legacy retired, not left to drift");
        assert!(
            root.join("artifacts.db.migrated-to-userdata").is_file(),
            "labeled rename, never deletion"
        );
        // Second open: new-exists path, no legacy needed.
        let ix = Index::open_adopting(&new, &legacy).unwrap();
        assert_eq!(ix.recent(10, None).unwrap().len(), 1);
        // Neither exists → a fresh index, no error.
        let fresh_root = scratch("adopt-fresh");
        let ix = Index::open_adopting(
            &fresh_root.join(".userdata/index.db"),
            &fresh_root.join("artifacts.db"),
        )
        .unwrap();
        assert!(ix.recent(10, None).unwrap().is_empty());
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
        authored.source = "viziwiz".into();
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

    /// Make a symlink for a test, or return false when the OS denies it.
    /// Windows needs the create-symlink privilege (or Developer Mode) — the
    /// same reason `import_reference` falls back to a copy — so a test must
    /// SKIP rather than fail on a machine without it.
    fn try_symlink(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        let r = std::os::unix::fs::symlink(target, link);
        #[cfg(windows)]
        let r = if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        };
        r.is_ok()
    }

    /// A REFERENCED artifact must be removable, and removing it must take the
    /// LINK and leave the operator's original alone. Before this, canonicalize
    /// resolved the link to their file, the containment check refused it, and
    /// a referenced clip could never be removed at all.
    #[test]
    fn dumping_a_reference_removes_the_link_and_never_the_original() {
        let root = scratch("dump-ref");
        let inbox = root.join("inbox");
        // The operator's own file, deliberately OUTSIDE content/.
        let theirs = root.join("their-drive");
        std::fs::create_dir_all(&theirs).unwrap();
        let original = theirs.join("clip.mp4");
        std::fs::write(&original, b"THEIR BYTES").unwrap();
        // What import_reference lands: a single-file symlink in the inbox.
        let link = inbox.join("clip.mp4");
        if !try_symlink(&original, &link) {
            // NOT A PASS. This machine cannot create symlinks (Windows
            // without Developer Mode or admin), so the assertions below never
            // run here — only CI, whose runner is privileged, truly exercises
            // them. A silent skip reading as "ok" is exactly how a Windows
            // folder-symlink bug survived a local `green` and died in CI.
            eprintln!("SKIPPED (NOT PROOF): this OS denied symlink creation");
            return;
        }
        sidecar::write(&link, &rec("inbox/clip.mp4", "video")).unwrap();

        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.reindex(&root).unwrap();

        ix.dump(&root, "inbox/clip.mp4")
            .expect("a referenced artifact must be removable");

        assert!(
            !link.exists() && std::fs::symlink_metadata(&link).is_err(),
            "the LINK is gone"
        );
        assert!(original.is_file(), "the operator's original SURVIVES");
        assert_eq!(
            std::fs::read(&original).unwrap(),
            b"THEIR BYTES",
            "and is untouched"
        );
    }

    /// `Path::is_dir()` follows symlinks, so a FOLDER symlink in the inbox
    /// would have sent `remove_dir_all` into the operator's real directory.
    /// `symlink_metadata` does not follow — this pins that.
    #[test]
    fn a_folder_symlink_is_unlinked_never_recursively_deleted() {
        let root = scratch("dump-dirlink");
        let inbox = root.join("inbox");
        let theirs = root.join("their-folder");
        std::fs::create_dir_all(&theirs).unwrap();
        std::fs::write(theirs.join("precious.mp4"), b"KEEP").unwrap();
        let link = inbox.join("pack");
        if !try_symlink(&theirs, &link) {
            // NOT A PASS. This machine cannot create symlinks (Windows
            // without Developer Mode or admin), so the assertions below never
            // run here — only CI, whose runner is privileged, truly exercises
            // them. A silent skip reading as "ok" is exactly how a Windows
            // folder-symlink bug survived a local `green` and died in CI.
            eprintln!("SKIPPED (NOT PROOF): this OS denied symlink creation");
            return;
        }

        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        // dump may or may not have a row; the FILESYSTEM outcome is the point.
        let _ = ix.dump(&root, "inbox/pack");

        assert!(
            std::fs::symlink_metadata(&link).is_err(),
            "the link is gone"
        );
        assert!(theirs.is_dir(), "their folder SURVIVES");
        assert!(
            theirs.join("precious.mp4").is_file(),
            "and its contents survive"
        );
    }

    /// The lexical jail still holds for links — `..` cannot smuggle a symlink
    /// outside the inbox, so skipping canonicalize costs no safety.
    #[test]
    fn the_jail_still_refuses_traversal() {
        // NOTE the distinct scratch tag: `scratch()` is keyed by tag and the
        // suite runs in parallel, so reusing "jail" raced the neighbouring
        // test and wiped its fixtures.
        let root = scratch("jail-lexical");
        for bad in [
            "../secrets.png",
            "inbox/../../etc/passwd",
            "elsewhere/x.png",
        ] {
            assert!(jail_inbox(&root, bad).is_err(), "{bad:?} must be refused");
        }
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

    // ── keep + associate (X3) ───────────────────────────────────────────

    /// An existing content episode — keep's precondition (keep never
    /// creates shows; the doc tier's clone does).
    fn content_episode(root: &Path, show: &str, episode: &str) {
        let dir = root.join(show).join(episode);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("episode.json"), "{}").unwrap();
    }

    #[test]
    fn keep_refuses_a_missing_episode_and_a_non_inbox_source() {
        let root = scratch("keep-refuse");
        std::fs::write(root.join("inbox").join("pix.png"), b"img").unwrap();
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        // No episode exists — refused, nothing created.
        let err = ix
            .keep(&root, "inbox/pix.png", "devpulse-news", "ep1")
            .unwrap_err();
        assert!(err.contains("refused"), "{err}");
        assert!(!root.join("devpulse-news").exists(), "keep never creates");
        assert!(root.join("inbox").join("pix.png").exists(), "source stays");
        // The jail binds keep + associate sources exactly like dump.
        content_episode(&root, "devpulse-news", "ep1");
        std::fs::create_dir_all(root.join("shows")).unwrap();
        std::fs::write(root.join("shows").join("x.png"), b"content").unwrap();
        for bad in ["shows/x.png", "inbox/../shows/x.png", "../etc/passwd"] {
            assert!(ix.keep(&root, bad, "devpulse-news", "ep1").is_err());
            assert!(ix.associate(&root, bad, "devpulse-news", "ep1").is_err());
        }
    }

    #[test]
    fn keep_moves_the_artifact_its_companions_and_rekeys_the_row() {
        let root = scratch("keep-move");
        let inbox = root.join("inbox");
        std::fs::write(inbox.join("pix.png"), b"img").unwrap();
        let mut authored = rec("inbox/pix.png", "image");
        authored.name = "gpu-explainer".into();
        authored.tags = vec!["ai".into()];
        sidecar::write(&inbox.join("pix.png"), &authored).unwrap();
        std::fs::write(inbox.join("vix.mp4"), b"vid").unwrap();
        std::fs::write(inbox.join("vix.jpg"), b"poster").unwrap();
        content_episode(&root, "devpulse-news", "ep1");
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.reindex(&root).unwrap();

        let new_rel = ix
            .keep(&root, "inbox/pix.png", "devpulse-news", "ep1")
            .unwrap();
        assert_eq!(new_rel, "devpulse-news/ep1/assets/pix.png");
        let dest = root.join("devpulse-news/ep1/assets/pix.png");
        assert!(dest.is_file(), "the file moved");
        assert!(!inbox.join("pix.png").exists(), "the inbox copy is gone");
        assert!(
            !inbox.join("pix.png.meta.json").exists(),
            "the sidecar travelled"
        );
        let side = sidecar::read(&dest).unwrap();
        assert_eq!(side.rel_path, new_rel, "sidecar rewritten");
        assert_eq!(side.location, "content");
        assert_eq!(side.show_id.as_deref(), Some("devpulse-news"));
        assert_eq!(side.name, "gpu-explainer", "authored truth survived");
        let content_rows = ix.list(None, Some("content")).unwrap();
        assert_eq!(content_rows.len(), 1, "the row re-keyed");
        assert_eq!(content_rows[0].rel_path, new_rel);
        assert_eq!(
            ix.list(None, Some("inbox")).unwrap().len(),
            1,
            "only the video still sits in the inbox"
        );

        // The video: its poster rides along, in file, sidecar AND row.
        let new_rel = ix
            .keep(&root, "inbox/vix.mp4", "devpulse-news", "ep1")
            .unwrap();
        assert!(root.join("devpulse-news/ep1/assets/vix.jpg").is_file());
        assert!(!inbox.join("vix.jpg").exists());
        let rows = ix.list(Some("video"), Some("content")).unwrap();
        assert_eq!(rows[0].rel_path, new_rel);
        assert_eq!(
            rows[0].poster_rel_path.as_deref(),
            Some("devpulse-news/ep1/assets/vix.jpg")
        );
    }

    #[test]
    fn keep_never_clobbers_an_existing_name() {
        let root = scratch("keep-clobber");
        std::fs::write(root.join("inbox").join("pix.png"), b"new").unwrap();
        content_episode(&root, "effingai", "pilot");
        let assets = root.join("effingai").join("pilot").join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("pix.png"), b"old").unwrap();
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        let new_rel = ix
            .keep(&root, "inbox/pix.png", "effingai", "pilot")
            .unwrap();
        assert_eq!(new_rel, "effingai/pilot/assets/pix-2.png");
        assert_eq!(std::fs::read(assets.join("pix.png")).unwrap(), b"old");
        assert_eq!(std::fs::read(assets.join("pix-2.png")).unwrap(), b"new");
    }

    #[test]
    fn a_guide_folder_keeps_whole() {
        let root = scratch("keep-guide");
        let guide = root.join("inbox").join("guide-1");
        std::fs::create_dir_all(&guide).unwrap();
        std::fs::write(guide.join("guide.json"), "{}").unwrap();
        std::fs::write(guide.join("step-001.webp"), b"frame").unwrap();
        content_episode(&root, "eons-of-stupidity", "ep1");
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.reindex(&root).unwrap();

        let new_rel = ix
            .keep(&root, "inbox/guide-1", "eons-of-stupidity", "ep1")
            .unwrap();
        assert_eq!(new_rel, "eons-of-stupidity/ep1/assets/guide-1");
        let dest = root.join("eons-of-stupidity/ep1/assets/guide-1");
        assert!(dest.join("guide.json").is_file());
        assert!(dest.join("step-001.webp").is_file(), "frames travelled");
        assert!(!guide.exists(), "the inbox folder is gone");
        let side = sidecar::read(&dest).unwrap();
        assert_eq!(side.location, "content");
        assert_eq!(side.kind, "guide");
        let rows = ix.list(Some("guide"), Some("content")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rel_path, new_rel);
    }

    // ── sessions (S1) — folder artifacts beyond guides ──────────────────

    #[test]
    fn a_session_folder_reindexes_even_partial_and_keeps_its_kind() {
        let root = scratch("session");
        let ses = root.join("inbox").join("session-20260723-120000");
        std::fs::create_dir_all(&ses).unwrap();
        std::fs::write(ses.join("session.mp4"), b"vid").unwrap();
        std::fs::write(ses.join("session.jpg"), b"poster").unwrap();
        std::fs::write(ses.join("events.jsonl"), "{\"ms\":0}\n").unwrap();
        // A PARTIAL session — the app died before the video finished.
        let part = root.join("inbox").join("session-partial");
        std::fs::create_dir_all(&part).unwrap();
        std::fs::write(part.join("events.jsonl"), "{\"ms\":0}\n").unwrap();

        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        let report = ix.reindex(&root).unwrap();
        assert_eq!(report.indexed, 2, "full + partial both index");
        let rows = ix.list(Some("session"), None).unwrap();
        assert_eq!(rows.len(), 2);

        // Keep preserves the kind — no guide mislabel.
        content_episode(&root, "devpulse-news", "ep1");
        let new_rel = ix
            .keep(
                &root,
                "inbox/session-20260723-120000",
                "devpulse-news",
                "ep1",
            )
            .unwrap();
        let kept = ix.list(Some("session"), Some("content")).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].rel_path, new_rel);
        assert!(root
            .join("devpulse-news/ep1/assets/session-20260723-120000/events.jsonl")
            .is_file());
    }

    #[test]
    fn associate_copies_and_the_library_master_stays() {
        let root = scratch("associate");
        let inbox = root.join("inbox");
        std::fs::write(inbox.join("pix.png"), b"img").unwrap();
        sidecar::write(&inbox.join("pix.png"), &rec("inbox/pix.png", "image")).unwrap();
        content_episode(&root, "devpulse-news", "ep1");
        let ix = Index::open(&root.join("artifacts.db")).unwrap();
        ix.reindex(&root).unwrap();

        let new_rel = ix
            .associate(&root, "inbox/pix.png", "devpulse-news", "ep1")
            .unwrap();
        assert_eq!(new_rel, "devpulse-news/ep1/assets/pix.png");
        assert!(inbox.join("pix.png").exists(), "the master stays");
        assert!(
            inbox.join("pix.png.meta.json").exists(),
            "the master's sidecar stays"
        );
        assert!(root.join("devpulse-news/ep1/assets/pix.png").is_file());
        let side = sidecar::read(&root.join("devpulse-news/ep1/assets/pix.png")).unwrap();
        assert_eq!(side.location, "content", "the copy's own sidecar");
        assert_eq!(ix.list(None, Some("inbox")).unwrap().len(), 1);
        assert_eq!(
            ix.list(None, Some("content")).unwrap().len(),
            1,
            "a NEW row for the copy — two rows total"
        );
    }
}
