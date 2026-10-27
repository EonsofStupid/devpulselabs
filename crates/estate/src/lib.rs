//! The per-project ESTATE — the local persistence + cache layer that makes
//! the editors (and the agent seam) durable and fast.
//!
//! Two industry-standard, pure-Rust picks, one job each:
//! - **Fjall** — an embedded LSM-tree key/value store: the persistent
//!   estate. Editor/doc blobs, undo history, the generated ingredient
//!   manifest, agent session state — survive restart, no server, no SQL.
//! - **Moka** — a concurrent in-process cache in front of every read, so the
//!   hot path (resolved styles, decoded thumbnails, ingredient lookups)
//!   never touches disk twice.
//!
//! This complements the artifact INDEX (SQLite/TURSO, structured media
//! queries) — a different job; the estate is KV blobs + cache, not a query
//! engine. Values are bytes; `*_json` helpers ride serde for typed docs.

// fjall 3 renamed the whole vocabulary one rung DOWN: what v2 called a
// `Keyspace` (the store) is now a `Database`, and what it called a
// `PartitionHandle` (one named tree inside it) is now a `Keyspace`. The
// estate's own `Tree` naming is unaffected — it was never fjall's word.
use fjall::{Config, Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use moka::sync::Cache;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// `Tree::State` key — last shell route as `"hub/sub"` (registry-validated).
pub const KEY_SHELL_ROUTE: &str = "shell/route";
/// `Tree::State` key — WiredFRONT last-opened comp (content-root rel path).
pub const KEY_WIREDFRONT_LAST_COMP: &str = "wiredfront/last-comp";

/// The named partitions of the estate — one keyspace, several trees, so a
/// doc read never scans agent sessions. Additive: new concerns claim a new
/// partition, they never squat an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tree {
    /// Editor + composition/scene/annotation document blobs, by id.
    Docs,
    /// Editor UI + session state (open tab, selection, undo cursor).
    State,
    /// The generated ingredient manifest (traits/elements/blocks/templates).
    Manifest,
    /// Agent (chat provider) session state for the HTTP seam.
    Agents,
}

impl Tree {
    const ALL: [Tree; 4] = [Tree::Docs, Tree::State, Tree::Manifest, Tree::Agents];
    fn name(self) -> &'static str {
        match self {
            Tree::Docs => "docs",
            Tree::State => "state",
            Tree::Manifest => "manifest",
            Tree::Agents => "agents",
        }
    }
}

/// The open Fjall database and its trees, swapped whole by [`Estate::reopen`].
struct Stores {
    db: Database,
    docs: Keyspace,
    state: Keyspace,
    manifest: Keyspace,
    agents: Keyspace,
}

impl Stores {
    /// Open (creating if absent) at `dir`; also returns where an unreadable
    /// older store was stepped aside, if one was.
    fn open(dir: &Path) -> Result<(Self, Option<PathBuf>), String> {
        // fjall 3 uses a new on-disk format and REFUSES a v2 store outright
        // (`InvalidVersion`) rather than migrating it. That is the right
        // default for a database holding authored work — but this estate does
        // not: its only production keys are `shell/route` and
        // `wiredfront/last-comp`, both pure reopen-convenience. So a stale
        // store is stepped ASIDE (never deleted — the document law's
        // bak-step-aside, applied to a directory) and a fresh one opened, and
        // the move is REPORTED so the caller can say so out loud. The
        // alternative — returning Err — disables durable seam state on every
        // launch from here to forever, behind one `warn!`.
        let mut stepped_aside = None;
        let db = match Database::open(Config::new(dir)) {
            Ok(db) => db,
            Err(fjall::Error::InvalidVersion(found)) if dir.exists() => {
                let aside = Estate::aside_path(dir);
                std::fs::rename(dir, &aside).map_err(|e| {
                    format!(
                        "estate {}: unreadable format ({found:?}) and stepping it aside to {} failed: {e}",
                        dir.display(),
                        aside.display()
                    )
                })?;
                stepped_aside = Some(aside);
                Database::open(Config::new(dir)).map_err(|e| {
                    format!("estate reopen after stepping aside {}: {e}", dir.display())
                })?
            }
            Err(e) => return Err(format!("estate open {}: {e}", dir.display())),
        };
        let mut parts = Vec::with_capacity(Tree::ALL.len());
        for t in Tree::ALL {
            // v3 takes the create options as a CLOSURE — it is only called
            // when the keyspace does not already exist.
            let p = db
                .keyspace(t.name(), KeyspaceCreateOptions::default)
                .map_err(|e| format!("estate partition {}: {e}", t.name()))?;
            parts.push(p);
        }
        let mut it = parts.into_iter();
        Ok((
            Self {
                docs: it.next().unwrap(),
                state: it.next().unwrap(),
                manifest: it.next().unwrap(),
                agents: it.next().unwrap(),
                db,
            },
            stepped_aside,
        ))
    }

    fn partition(&self, tree: Tree) -> &Keyspace {
        match tree {
            Tree::Docs => &self.docs,
            Tree::State => &self.state,
            Tree::Manifest => &self.manifest,
            Tree::Agents => &self.agents,
        }
    }
}

/// One project's estate: a Fjall keyspace of partitions + a Moka read cache.
/// Clone-cheap is not offered — an estate is opened once and shared by `&`.
pub struct Estate {
    /// `None` only inside [`Estate::reopen`], between closing and opening.
    stores: RwLock<Option<Stores>>,
    /// Read cache keyed by "<tree>/<key>" — bytes shared as `Arc<[u8]>`.
    cache: Cache<String, Arc<[u8]>>,
    /// Set when [`Estate::open`] found a store it could not read and stepped
    /// it aside. The caller LOGS this — the ceiling (fjall, moka, serde,
    /// serde_json) admits no logger, and a reset that only whispers into a
    /// `warn!` nobody reads is how durable state silently stays off forever.
    stepped_aside: Option<PathBuf>,
}

impl Estate {
    /// Open (creating if absent) the estate rooted at `dir`.
    pub fn open(dir: &Path) -> Result<Self, String> {
        let (stores, stepped_aside) = Stores::open(dir)?;
        Ok(Self {
            stores: RwLock::new(Some(stores)),
            cache: Cache::new(8192),
            stepped_aside,
        })
    }

    /// Close the store and open it again at `dir` (ST2b): after the library
    /// was unplugged its open files are dead. Everyone holding this estate
    /// keeps working. The old database closes first, because Fjall locks
    /// its folder.
    pub fn reopen(&self, dir: &Path) -> Result<(), String> {
        let mut stores = self
            .stores
            .write()
            .map_err(|_| "estate lock poisoned".to_string())?;
        *stores = None;
        self.cache.invalidate_all();
        *stores = Some(Stores::open(dir)?.0);
        Ok(())
    }

    /// The store this estate could not read, moved out of the way at open —
    /// `None` on every normal launch. The caller logs it; nothing reads it
    /// back. No-clobber, so a second stale store never overwrites the first.
    pub fn stepped_aside(&self) -> Option<&Path> {
        self.stepped_aside.as_deref()
    }

    /// `<dir>.stale`, then `.stale-2`, `.stale-3`… — never an existing path.
    fn aside_path(dir: &Path) -> PathBuf {
        let first = dir.with_extension("stale");
        if !first.exists() {
            return first;
        }
        (2..)
            .map(|n| dir.with_extension(format!("stale-{n}")))
            .find(|p| !p.exists())
            .expect("an unused suffix exists")
    }

    /// Run `read` against one tree of the open store.
    fn with<R>(
        &self,
        tree: Tree,
        read: impl FnOnce(&Keyspace) -> Result<R, String>,
    ) -> Result<R, String> {
        let stores = self
            .stores
            .read()
            .map_err(|_| "estate lock poisoned".to_string())?;
        let stores = stores
            .as_ref()
            .ok_or_else(|| "estate is reopening".to_string())?;
        read(stores.partition(tree))
    }

    fn cache_key(tree: Tree, key: &str) -> String {
        format!("{}/{key}", tree.name())
    }

    /// Write bytes; updates the cache. Durability is batched — call
    /// [`Estate::persist`] at a save point (or rely on Fjall's recovery).
    pub fn put(&self, tree: Tree, key: &str, bytes: &[u8]) -> Result<(), String> {
        self.with(tree, |part| {
            part.insert(key, bytes)
                .map_err(|e| format!("estate put {}/{key}: {e}", tree.name()))
        })?;
        self.cache
            .insert(Self::cache_key(tree, key), Arc::from(bytes));
        Ok(())
    }

    /// Read bytes — cache first, then the partition (populating the cache).
    pub fn get(&self, tree: Tree, key: &str) -> Result<Option<Arc<[u8]>>, String> {
        let ck = Self::cache_key(tree, key);
        if let Some(v) = self.cache.get(&ck) {
            return Ok(Some(v));
        }
        let found = self.with(tree, |part| {
            part.get(key)
                .map_err(|e| format!("estate get {}/{key}: {e}", tree.name()))
        })?;
        match found {
            Some(slice) => {
                let arc: Arc<[u8]> = Arc::from(&slice[..]);
                self.cache.insert(ck, arc.clone());
                Ok(Some(arc))
            }
            None => Ok(None),
        }
    }

    /// Delete a key (and evict it from cache).
    pub fn remove(&self, tree: Tree, key: &str) -> Result<(), String> {
        self.with(tree, |part| {
            part.remove(key)
                .map_err(|e| format!("estate remove {}/{key}: {e}", tree.name()))
        })?;
        self.cache.invalidate(&Self::cache_key(tree, key));
        Ok(())
    }

    /// Store a serde value as JSON bytes.
    pub fn put_json<T: Serialize>(&self, tree: Tree, key: &str, value: &T) -> Result<(), String> {
        let bytes = serde_json::to_vec(value).map_err(|e| format!("estate encode {key}: {e}"))?;
        self.put(tree, key, &bytes)
    }

    /// Load a serde value from JSON bytes; `Ok(None)` when the key is absent.
    pub fn get_json<T: DeserializeOwned>(
        &self,
        tree: Tree,
        key: &str,
    ) -> Result<Option<T>, String> {
        match self.get(tree, key)? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| format!("estate decode {key}: {e}")),
            None => Ok(None),
        }
    }

    /// Every key in a tree (sorted by Fjall's key order) — for listing docs,
    /// manifest entries, sessions. Malformed rows are skipped, never invented.
    pub fn keys(&self, tree: Tree) -> Vec<String> {
        // v3 has no `keys()`: iteration yields a `Guard` that lends the key
        // without forcing the value off disk, so `key()` IS the cheap read.
        self.with(tree, |part| {
            Ok(part
                .iter()
                .filter_map(|guard| guard.key().ok())
                .filter_map(|k| String::from_utf8(k.to_vec()).ok())
                .collect())
        })
        .unwrap_or_default()
    }

    /// Flush to disk durably — call at editor save points.
    pub fn persist(&self) -> Result<(), String> {
        let stores = self
            .stores
            .read()
            .map_err(|_| "estate lock poisoned".to_string())?;
        stores
            .as_ref()
            .ok_or_else(|| "estate is reopening".to_string())?
            .db
            .persist(PersistMode::SyncAll)
            .map_err(|e| format!("estate persist: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-estate-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// The fjall 2 → 3 case, exercised through the REAL open path: a store
    /// whose version marker fjall refuses must be stepped aside (not deleted,
    /// not fatal), a fresh estate must open in its place, and the move must
    /// be REPORTED so the caller can say it out loud. Writing a bogus marker
    /// is how a v2 store presents itself to v3 — an unreadable version, which
    /// is exactly the branch under test.
    #[test]
    fn an_unreadable_store_is_stepped_aside_not_fatal() {
        let dir = scratch("stale-format");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("version"), [b'F', b'J', b'L', b'L', 2]).unwrap();

        let e = Estate::open(&dir).expect("a stale store must not be fatal");
        let aside = e
            .stepped_aside()
            .expect("the move must be reported, not silent")
            .to_path_buf();
        assert!(aside.exists(), "the old store is preserved, never deleted");
        assert_ne!(
            aside, dir,
            "it is moved out of the way, not opened in place"
        );

        // The fresh estate is fully usable, and survives a reopen.
        e.put_json(Tree::State, KEY_SHELL_ROUTE, &"web/home".to_string())
            .unwrap();
        e.persist().unwrap();
        drop(e);

        let reopened = Estate::open(&dir).unwrap();
        assert!(
            reopened.stepped_aside().is_none(),
            "a healthy store reports no move"
        );
        assert_eq!(
            reopened
                .get_json::<String>(Tree::State, KEY_SHELL_ROUTE)
                .unwrap()
                .as_deref(),
            Some("web/home")
        );
    }

    #[test]
    fn put_get_round_trips_through_cache_and_disk() {
        let dir = scratch("roundtrip");
        let e = Estate::open(&dir).unwrap();
        e.put(Tree::Docs, "a", b"hello").unwrap();
        assert_eq!(
            e.get(Tree::Docs, "a").unwrap().as_deref(),
            Some(&b"hello"[..])
        );
        // A second read is served from the Moka cache (same bytes).
        assert_eq!(
            e.get(Tree::Docs, "a").unwrap().as_deref(),
            Some(&b"hello"[..])
        );
        // Absent keys are honest Nones.
        assert!(e.get(Tree::Docs, "missing").unwrap().is_none());
        // Trees are isolated: the same key in another tree is independent.
        assert!(e.get(Tree::State, "a").unwrap().is_none());
    }

    #[test]
    fn json_docs_round_trip_and_list_by_key() {
        let dir = scratch("json");
        let e = Estate::open(&dir).unwrap();
        e.put_json(Tree::Docs, "comp-1", &vec![1u32, 2, 3]).unwrap();
        e.put_json(Tree::Docs, "comp-2", &vec![9u32]).unwrap();
        assert_eq!(
            e.get_json::<Vec<u32>>(Tree::Docs, "comp-1").unwrap(),
            Some(vec![1, 2, 3])
        );
        let mut keys = e.keys(Tree::Docs);
        keys.sort();
        assert_eq!(keys, vec!["comp-1".to_string(), "comp-2".to_string()]);
        e.remove(Tree::Docs, "comp-1").unwrap();
        assert!(e.get(Tree::Docs, "comp-1").unwrap().is_none());
    }

    /// The whole point of the estate: state SURVIVES a restart.
    #[test]
    fn state_survives_reopen() {
        let dir = scratch("persist");
        {
            let e = Estate::open(&dir).unwrap();
            e.put_json(Tree::State, "open-tab", &"annotate".to_string())
                .unwrap();
            e.persist().unwrap();
        }
        let e2 = Estate::open(&dir).unwrap();
        assert_eq!(
            e2.get_json::<String>(Tree::State, "open-tab").unwrap(),
            Some("annotate".to_string())
        );
    }

    /// ST2b: after a reopen every holder of the estate still reads what was
    /// written and can write again.
    #[test]
    fn reopen_keeps_every_holder_working() {
        let dir = scratch("reopen");
        let estate = Arc::new(Estate::open(&dir).unwrap());
        let holder = Arc::clone(&estate);
        estate.put(Tree::State, "k", b"before").unwrap();
        estate.persist().unwrap();
        estate.reopen(&dir).unwrap();
        assert_eq!(&*holder.get(Tree::State, "k").unwrap().unwrap(), b"before");
        holder.put(Tree::State, "k", b"after").unwrap();
        assert_eq!(&*estate.get(Tree::State, "k").unwrap().unwrap(), b"after");
        assert_eq!(estate.keys(Tree::State), ["k"]);
    }
}
