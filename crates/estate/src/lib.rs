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

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle, PersistMode};
use moka::sync::Cache;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::sync::Arc;

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

/// One project's estate: a Fjall keyspace of partitions + a Moka read cache.
/// Clone-cheap is not offered — an estate is opened once and shared by `&`.
pub struct Estate {
    keyspace: Keyspace,
    docs: PartitionHandle,
    state: PartitionHandle,
    manifest: PartitionHandle,
    agents: PartitionHandle,
    /// Read cache keyed by "<tree>/<key>" — bytes shared as `Arc<[u8]>`.
    cache: Cache<String, Arc<[u8]>>,
}

impl Estate {
    /// Open (creating if absent) the estate rooted at `dir`.
    pub fn open(dir: &Path) -> Result<Self, String> {
        let keyspace = Config::new(dir)
            .open()
            .map_err(|e| format!("estate open {}: {e}", dir.display()))?;
        let mut parts = Vec::with_capacity(Tree::ALL.len());
        for t in Tree::ALL {
            let p = keyspace
                .open_partition(t.name(), PartitionCreateOptions::default())
                .map_err(|e| format!("estate partition {}: {e}", t.name()))?;
            parts.push(p);
        }
        let mut it = parts.into_iter();
        Ok(Self {
            docs: it.next().unwrap(),
            state: it.next().unwrap(),
            manifest: it.next().unwrap(),
            agents: it.next().unwrap(),
            cache: Cache::new(8192),
            keyspace,
        })
    }

    fn partition(&self, tree: Tree) -> &PartitionHandle {
        match tree {
            Tree::Docs => &self.docs,
            Tree::State => &self.state,
            Tree::Manifest => &self.manifest,
            Tree::Agents => &self.agents,
        }
    }

    fn cache_key(tree: Tree, key: &str) -> String {
        format!("{}/{key}", tree.name())
    }

    /// Write bytes; updates the cache. Durability is batched — call
    /// [`Estate::persist`] at a save point (or rely on Fjall's recovery).
    pub fn put(&self, tree: Tree, key: &str, bytes: &[u8]) -> Result<(), String> {
        self.partition(tree)
            .insert(key, bytes)
            .map_err(|e| format!("estate put {}/{key}: {e}", tree.name()))?;
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
        match self
            .partition(tree)
            .get(key)
            .map_err(|e| format!("estate get {}/{key}: {e}", tree.name()))?
        {
            Some(slice) => {
                let arc: Arc<[u8]> = Arc::from(slice.as_ref());
                self.cache.insert(ck, arc.clone());
                Ok(Some(arc))
            }
            None => Ok(None),
        }
    }

    /// Delete a key (and evict it from cache).
    pub fn remove(&self, tree: Tree, key: &str) -> Result<(), String> {
        self.partition(tree)
            .remove(key)
            .map_err(|e| format!("estate remove {}/{key}: {e}", tree.name()))?;
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
        self.partition(tree)
            .keys()
            .filter_map(|r| r.ok())
            .filter_map(|k| String::from_utf8(k.to_vec()).ok())
            .collect()
    }

    /// Flush to disk durably — call at editor save points.
    pub fn persist(&self) -> Result<(), String> {
        self.keyspace
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
}
