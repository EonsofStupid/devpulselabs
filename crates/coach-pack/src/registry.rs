//! The pack registry — packs are files, the registry is a directory
//! (`<root>/packs/<app-id>/pack.toml`). Root-agnostic like the ledger:
//! the app hands its userdata root, tests hand a temp dir. Per-stage
//! Foundry artifacts land BESIDE the pack (S3b) — the pack's directory
//! is the checkpoint unit.

use crate::bundle::FingerprintBundle;
use crate::id::AppId;
use crate::pack::{Pack, PackError, ParsedPack};
use crate::resolve::{resolve_against, Resolution};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PackRegistry {
    root: PathBuf,
}

impl PackRegistry {
    /// Open a registry rooted anywhere. Nothing is created until `write`.
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn packs_dir(&self) -> PathBuf {
        self.root.join("packs")
    }

    pub fn pack_dir(&self, id: &AppId) -> PathBuf {
        self.packs_dir().join(id.as_str())
    }

    pub fn pack_path(&self, id: &AppId) -> PathBuf {
        self.pack_dir(id).join("pack.toml")
    }

    /// Installed app ids — directories under `packs/` whose name parses
    /// as an `AppId` and that contain a `pack.toml`. Sorted, stable.
    pub fn list(&self) -> Vec<AppId> {
        let Ok(entries) = fs::read_dir(self.packs_dir()) else {
            return Vec::new();
        };
        let mut ids: Vec<AppId> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let id = AppId::parse(&name).ok()?;
                e.path().join("pack.toml").is_file().then_some(id)
            })
            .collect();
        ids.sort();
        ids
    }

    pub fn load(&self, id: &AppId) -> Result<ParsedPack, PackError> {
        let path = self.pack_path(id);
        let text = fs::read_to_string(&path)
            .map_err(|e| PackError::Io(format!("read {}: {e}", path.display())))?;
        Pack::parse(&text)
    }

    /// Every loadable installed pack. A broken pack becomes a warning
    /// beside the good ones — one bad file never blinds the resolver.
    pub fn installed(&self) -> (Vec<ParsedPack>, Vec<String>) {
        let mut packs = Vec::new();
        let mut warnings = Vec::new();
        for id in self.list() {
            match self.load(&id) {
                Ok(parsed) => packs.push(parsed),
                Err(e) => warnings.push(format!("pack {id} unreadable: {e}")),
            }
        }
        (packs, warnings)
    }

    /// Persist a pack: content-hash signed, tmp → bak step-aside →
    /// rename (document law). Refuses unresolved identities — promotion
    /// mints a real `AppId` first; `x.unresolved:*` profiles are
    /// ephemeral by design.
    pub fn write(&self, pack: &Pack) -> Result<PathBuf, PackError> {
        let id = &pack.identity.app_id;
        if id.is_unresolved() {
            return Err(PackError::UnresolvedWrite(id.clone()));
        }
        let dir = self.pack_dir(id);
        fs::create_dir_all(&dir)
            .map_err(|e| PackError::Io(format!("create {}: {e}", dir.display())))?;
        let text = pack.to_signed_toml()?;
        let path = dir.join("pack.toml");
        let tmp = dir.join("pack.toml.tmp");
        fs::write(&tmp, &text)
            .map_err(|e| PackError::Io(format!("write {}: {e}", tmp.display())))?;
        if path.exists() {
            let bak = dir.join("pack.toml.bak");
            if bak.exists() {
                fs::remove_file(&bak)
                    .map_err(|e| PackError::Io(format!("clear {}: {e}", bak.display())))?;
            }
            fs::rename(&path, &bak)
                .map_err(|e| PackError::Io(format!("step aside {}: {e}", path.display())))?;
        }
        fs::rename(&tmp, &path)
            .map_err(|e| PackError::Io(format!("rename {}: {e}", path.display())))?;
        Ok(path)
    }

    /// Resolve one scan's bundle against everything installed here.
    pub fn resolve(&self, bundle: &FingerprintBundle) -> Resolution {
        let (packs, _warnings) = self.installed();
        resolve_against(bundle, packs.iter().map(|p| &p.pack))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::CellKey;
    use crate::pack::{Identity, CURRENT_SEAM, PACK_SCHEMA};
    use crate::resolve::synthesize_generic;

    fn temp_registry(tag: &str) -> PackRegistry {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PackRegistry::open(std::env::temp_dir().join(format!("dpl-coach-pack-{tag}-{ms}")))
    }

    fn calculator_pack() -> Pack {
        Pack {
            schema: PACK_SCHEMA,
            min_seam: CURRENT_SEAM,
            identity: Identity {
                app_id: AppId::parse("org.gnome.Calculator").unwrap(),
                display_name: "Calculator".into(),
                desktop_ids: vec!["org.gnome.Calculator.desktop".into()],
                wm_classes: vec!["gnome-calculator".into()],
                atspi_names: vec!["gnome-calculator".into()],
                exe_basenames: vec!["gnome-calculator".into()],
                aliases: vec![],
            },
            cell: CellKey::new("48", "linux", "en"),
            ladder: Vec::new(),
            verbs: Vec::new(),
            teaching_landmarks: Vec::new(),
            extras: toml::Table::new(),
        }
    }

    #[test]
    fn write_load_list_round_trip_with_bak_step_aside() {
        let reg = temp_registry("rt");
        let pack = calculator_pack();
        let path = reg.write(&pack).unwrap();
        assert!(path.ends_with("packs/org.gnome.Calculator/pack.toml"));

        assert_eq!(reg.list(), vec![pack.identity.app_id.clone()]);
        let loaded = reg.load(&pack.identity.app_id).unwrap();
        assert_eq!(loaded.pack, pack);
        assert_eq!(loaded.hash_verified, Some(true));

        // Second write steps the first aside as .bak (document law).
        let mut v2 = pack.clone();
        v2.identity.aliases.push("calc".into());
        reg.write(&v2).unwrap();
        assert!(reg
            .pack_dir(&pack.identity.app_id)
            .join("pack.toml.bak")
            .is_file());
        assert_eq!(reg.load(&pack.identity.app_id).unwrap().pack, v2);

        let _ = std::fs::remove_dir_all(reg.root());
    }

    #[test]
    fn unresolved_identities_are_refused_persistence() {
        let reg = temp_registry("unres");
        let synthesized = synthesize_generic(&FingerprintBundle {
            atspi_name: Some("Mystery".into()),
            ..Default::default()
        });
        match reg.write(&synthesized) {
            Err(PackError::UnresolvedWrite(id)) => {
                assert_eq!(id.as_str(), "x.unresolved:mystery");
            }
            other => panic!("expected UnresolvedWrite, got {other:?}"),
        }
        assert!(reg.list().is_empty());
    }

    #[test]
    fn registry_resolves_installed_and_synthesizes_strangers() {
        let reg = temp_registry("resolve");
        reg.write(&calculator_pack()).unwrap();

        let calc_bundle = FingerprintBundle {
            desktop_id: Some("org.gnome.Calculator.desktop".into()),
            atspi_name: Some("gnome-calculator".into()),
            ..Default::default()
        };
        match reg.resolve(&calc_bundle) {
            Resolution::Resolved { app_id, score, .. } => {
                assert_eq!(app_id.as_str(), "org.gnome.Calculator");
                assert_eq!(score, 6.0);
            }
            other => panic!("expected resolved, got {other:?}"),
        }

        let stranger = FingerprintBundle {
            atspi_name: Some("Stranger".into()),
            ..Default::default()
        };
        assert!(matches!(
            reg.resolve(&stranger),
            Resolution::Unresolved { .. }
        ));

        let _ = std::fs::remove_dir_all(reg.root());
    }

    #[test]
    fn a_broken_pack_warns_and_never_blinds_the_resolver() {
        let reg = temp_registry("broken");
        reg.write(&calculator_pack()).unwrap();
        // A second, corrupt pack directory.
        let bad_dir = reg.packs_dir().join("org.bad.App");
        std::fs::create_dir_all(&bad_dir).unwrap();
        std::fs::write(bad_dir.join("pack.toml"), "this is not toml [[[").unwrap();

        let (packs, warnings) = reg.installed();
        assert_eq!(packs.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("org.bad.App"));

        let _ = std::fs::remove_dir_all(reg.root());
    }
}
