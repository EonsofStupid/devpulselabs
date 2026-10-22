//! The ONE-TIME upgrade of old slides (v1/v2) to `dpl.slide/3` — LS2,
//! ruled 2026-10-21: "upgrade once, keep backups".
//!
//! Each upgraded folder keeps its original beside it as [`BACKUP_FILE`]
//! (written BEFORE the rewrite, never overwritten). Block ids become the
//! component's slot ids, and every old style value is written through as an
//! explicit override (ruling #1: a present `size` stays, so no slide changes
//! appearance). Pre-release law: this module goes once every old slide has
//! been upgraded (the seeds in the repo with LS2a; the operator's content on
//! the first launch of the build that ships it).

use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::{library_slide_dir, save, Background, Block, Slide, SlideProvenance, FILE};

/// The original old-format file, kept beside its upgrade.
pub const BACKUP_FILE: &str = "slide.v2.json";

/// The old document shape (v1/v2), read only to upgrade it.
#[derive(Debug, Deserialize)]
struct OldSlide {
    v: u32,
    template: String,
    #[serde(default)]
    background: Option<Background>,
    #[serde(default)]
    blocks: Vec<OldBlock>,
    #[serde(default)]
    enter: Option<String>,
    #[serde(default)]
    reveals: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum OldBlock {
    Text {
        id: String,
        text: String,
        #[serde(default)]
        font: Option<String>,
        size: String,
        #[serde(default)]
        ink: Option<String>,
    },
    Media {
        id: String,
        #[serde(rename = "as")]
        show: String,
        src: String,
        #[serde(default)]
        fit: Option<String>,
        #[serde(default)]
        poster: Option<String>,
    },
}

/// The component an old `template` fills, and how its block ids map onto
/// that component's slots. Read from the contracts (2026-10-21):
/// - `title`: eyebrow/title/subtitle already are slots; the editor's
///   `picture` media block is the `media` slot.
/// - `stat`: the top line `label` is the `eyebrow`, `number` is the
///   `value`, the `caption` under it is the `label`.
/// - `quote` (the old convention): `said-by` is `name`, `source` is `role`.
/// - `media` (make-slide): already slot-named.
/// - anything else keeps its name and ids; `validate_against` names it.
fn component_for(template: &str) -> (&str, &'static [(&'static str, &'static str)]) {
    match template {
        "title" | "title-card" => ("title", &[("picture", "media")]),
        "stat" => (
            "stat",
            &[
                ("label", "eyebrow"),
                ("number", "value"),
                ("caption", "label"),
            ],
        ),
        "quote" => ("quote", &[("said-by", "name"), ("source", "role")]),
        other => (other, &[]),
    }
}

/// Upgrade one old document's text to a v3 slide (a fresh id, rev 1).
pub fn upgrade(raw: &str) -> Result<Slide, String> {
    let old: OldSlide = serde_json::from_str(raw).map_err(|e| format!("old slide: {e}"))?;
    let (component, ids) = component_for(&old.template);
    let rename = |id: &str| {
        ids.iter()
            .find(|(from, _)| *from == id)
            .map_or_else(|| id.to_owned(), |(_, to)| (*to).to_owned())
    };
    let mut slide = Slide::new(format!("{component}@1"));
    slide.background = old.background;
    slide.enter = old.enter;
    slide.reveals = old.reveals.iter().map(|id| rename(id)).collect();
    slide.provenance = SlideProvenance {
        upgraded_from: Some(format!("v{}", old.v)),
        ..SlideProvenance::default()
    };
    for block in old.blocks {
        slide.blocks.push(match block {
            OldBlock::Text {
                id,
                text,
                font,
                size,
                ink,
            } => {
                let slot = rename(&id);
                if component == "quote" && slot == "quote" {
                    Block::Quote {
                        id: slot,
                        preset: None,
                        tone: None,
                        text,
                        font,
                        size: Some(size),
                        ink,
                    }
                } else {
                    Block::Text {
                        id: slot,
                        preset: None,
                        tone: None,
                        text,
                        font,
                        size: Some(size),
                        ink,
                    }
                }
            }
            OldBlock::Media {
                id,
                show,
                src,
                fit,
                poster,
            } => Block::Media {
                id: rename(&id),
                preset: None,
                tone: None,
                show,
                src,
                fit,
                poster,
            },
        });
    }
    Ok(slide)
}

/// Is the slide in `dir` in the old format?
pub fn is_old(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .is_some_and(|value| value.get("schema").is_none() && value.get("v").is_some())
}

/// Upgrade the old slide in `dir` in place: the original is copied beside
/// it as [`BACKUP_FILE`] FIRST (an existing backup is never overwritten),
/// then the v3 slide is saved over `slide.json`.
pub fn upgrade_dir(dir: &Path) -> Result<Slide, String> {
    let raw = std::fs::read_to_string(dir.join(FILE)).map_err(|e| format!("read: {e}"))?;
    let slide = upgrade(&raw)?;
    let backup = dir.join(BACKUP_FILE);
    if !backup.exists() {
        std::fs::write(&backup, &raw).map_err(|e| format!("backup: {e}"))?;
    }
    save(dir, &slide).map_err(|e| format!("save: {e}"))?;
    Ok(slide)
}

/// One upgraded slide folder, for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgraded {
    pub from: PathBuf,
    /// Where it lives now (drafts move into the library).
    pub to: PathBuf,
    pub id: String,
}

/// The first-launch upgrade of the operator's content (ruled 2026-10-21):
/// every old slide under `content_root` is upgraded in place, and the inbox
/// drafts then move into the library under their new id. A folder that
/// fails is left exactly as it was and named; nothing is half-done.
pub fn upgrade_content(content_root: &Path) -> (Vec<Upgraded>, Vec<(PathBuf, String)>) {
    let mut done = Vec::new();
    let mut failed = Vec::new();
    let mut old = Vec::new();
    collect_old(content_root, &mut old);
    for dir in old {
        match upgrade_dir(&dir) {
            Ok(slide) => {
                let in_inbox = dir.parent() == Some(content_root.join("inbox").as_path());
                let to = if in_inbox {
                    let dest = library_slide_dir(content_root, &slide.id);
                    let moved = dest
                        .parent()
                        .map_or(Ok(()), std::fs::create_dir_all)
                        .and_then(|()| std::fs::rename(&dir, &dest));
                    match moved {
                        Ok(()) => dest,
                        Err(error) => {
                            failed.push((dir.clone(), format!("upgraded, but not moved: {error}")));
                            dir.clone()
                        }
                    }
                } else {
                    dir.clone()
                };
                done.push(Upgraded {
                    from: dir,
                    to,
                    id: slide.id,
                });
            }
            Err(error) => failed.push((dir, error)),
        }
    }
    (done, failed)
}

fn collect_old(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join(FILE).is_file() && is_old(&path) {
            out.push(path.clone());
        }
        collect_old(&path, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load, validate};

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-legacy-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    const OLD_STAT: &str = r#"{
      "v": 1, "template": "stat",
      "background": {"kind": "mood", "mood": "ember-drift"},
      "blocks": [
        {"kind": "text", "id": "label", "text": "Tokens", "size": "h4", "ink": "muted"},
        {"kind": "text", "id": "number", "text": "70x", "font": "orbitron", "size": "display-xl"},
        {"kind": "text", "id": "caption", "text": "cheaper", "size": "body-lg"}
      ],
      "reveals": ["number"]
    }"#;

    #[test]
    fn an_old_stat_maps_onto_the_stat_slots_and_keeps_every_size() {
        let slide = upgrade(OLD_STAT).expect("upgrades");
        assert_eq!(slide.component, "stat@1");
        let ids: Vec<&str> = slide.blocks.iter().map(Block::id).collect();
        assert_eq!(ids, ["eyebrow", "value", "label"]);
        assert_eq!(slide.reveals, ["value"]);
        assert!(matches!(
            slide.block("value"),
            Some(Block::Text { size: Some(size), font: Some(font), .. })
                if size == "display-xl" && font == "orbitron"
        ));
        assert_eq!(slide.provenance.upgraded_from.as_deref(), Some("v1"));
        assert_eq!(validate(&slide), Vec::new());
    }

    #[test]
    fn upgrading_a_folder_keeps_the_original_beside_it_and_reloads() {
        let dir = scratch("folder");
        std::fs::write(dir.join(FILE), OLD_STAT).expect("write old");
        assert!(is_old(&dir));
        let slide = upgrade_dir(&dir).expect("upgrade");
        assert_eq!(
            std::fs::read_to_string(dir.join(BACKUP_FILE)).expect("backup"),
            OLD_STAT
        );
        assert_eq!(load(&dir).expect("v3 loads"), slide);
        assert!(!is_old(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn inbox_drafts_move_into_the_library_and_others_stay() {
        let root = scratch("content");
        let draft = root.join("inbox").join("authored-draft-2");
        std::fs::create_dir_all(&draft).expect("draft dir");
        std::fs::write(
            draft.join(FILE),
            r#"{"v": 2, "template": "title", "blocks": [
                {"kind": "media", "id": "picture", "as": "image", "src": "inbox/x.png"}]}"#,
        )
        .expect("write draft");
        let episode_slide = root.join("show").join("ep").join("opener");
        std::fs::create_dir_all(&episode_slide).expect("episode slide");
        std::fs::write(episode_slide.join(FILE), OLD_STAT).expect("write");

        let (done, failed) = upgrade_content(&root);
        assert!(failed.is_empty(), "{failed:?}");
        assert_eq!(done.len(), 2);
        let moved = done.iter().find(|u| u.from == draft).expect("the draft");
        assert_eq!(moved.to, library_slide_dir(&root, &moved.id));
        let slide = load(&moved.to).expect("moved draft loads");
        assert!(
            slide.block("media").is_some(),
            "picture became the media slot"
        );
        assert!(moved.to.join(BACKUP_FILE).is_file());
        assert!(!draft.exists());
        assert!(
            load(&episode_slide).is_ok(),
            "an episode slide upgrades in place"
        );
        // Twice is a no-op: nothing old is left.
        assert_eq!(upgrade_content(&root), (Vec::new(), Vec::new()));
        let _ = std::fs::remove_dir_all(&root);
    }
}
