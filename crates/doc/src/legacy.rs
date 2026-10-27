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
//!
//! It also holds the ONE-TIME move of a content episode still in the
//! template shape into the library (LS3b2, ruled 2026-10-24 "Two PRs, dry
//! run first"): [`plan_move`] says exactly what would move and changes
//! nothing; [`move_episode`] does it, only when the operator asks.

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
                        enter: None,
                        unlinked_from: None,
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
                        enter: None,
                        unlinked_from: None,
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
                enter: None,
                unlinked_from: None,
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
    let dir = &crate::writable(dir).map_err(|e| e.to_string())?;
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

// ── the EPISODE move (LS3b2) ────────────────────────────────────────────

/// The template `episode.json`, kept beside the moved episode.
pub const EPISODE_BACKUP: &str = "episode.v1.json";

/// One slide folder an episode's move takes into the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovingSlide {
    /// Its folder name in the episode.
    pub name: String,
    pub from: PathBuf,
    pub to: PathBuf,
    /// The id it lands as: its own, or — when that id is already in the
    /// library — the fresh id of its copy.
    pub id: String,
    pub forked: bool,
    /// It has a script, which will name the card's id.
    pub script: bool,
    /// Media refs inside the folder that move with it: (now, after).
    pub media: Vec<(String, String)>,
}

/// One lesson a chapter becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovingLesson {
    /// "Episode — Chapter".
    pub title: String,
    /// The cards it uses, in order.
    pub cards: Vec<String>,
}

/// Exactly what moving one episode would do. Making it changes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePlan {
    pub dir: PathBuf,
    pub title: String,
    /// Each slide folder the chapters name, once, in chapter order.
    pub slides: Vec<MovingSlide>,
    /// Slide folders no chapter names — they move too (never deleted), in
    /// no lesson.
    pub orphans: Vec<MovingSlide>,
    pub lessons: Vec<MovingLesson>,
    /// Why the whole episode cannot move; it then stays exactly as it is.
    pub refused: Vec<String>,
}

/// Every content episode still in the template shape, as (show, episode,
/// folder). Seeds are templates by design and are never listed.
pub fn old_shape_episodes(content_root: &Path) -> Vec<(String, String, PathBuf)> {
    crate::list_content_episodes(content_root)
        .into_iter()
        .filter(|(_, _, dir)| {
            matches!(
                crate::episode::open(dir),
                Ok(crate::episode::Opened::Template(_))
            )
        })
        .collect()
}

/// What moving the template episode at `dir` into the library would do —
/// read-only. `Err` when `dir` is not a template episode at all.
pub fn plan_move(dir: &Path, content_root: &Path) -> Result<MovePlan, String> {
    let template = match crate::episode::open(dir) {
        Ok(crate::episode::Opened::Template(ep)) => ep,
        Ok(crate::episode::Opened::Lessons(_)) => {
            return Err("this episode is already built from lessons".into())
        }
        Err(e) => return Err(format!("episode.json does not load: {e}")),
    };
    let mut plan = MovePlan {
        dir: dir.to_path_buf(),
        title: template.title.clone(),
        slides: Vec::new(),
        orphans: Vec::new(),
        lessons: Vec::new(),
        refused: Vec::new(),
    };
    let mut taken: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut planned: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for chapter in &template.chapters {
        let mut cards = Vec::new();
        for name in &chapter.slides {
            if let Some(id) = planned.get(name) {
                cards.push(id.clone());
                continue;
            }
            match plan_slide(dir, name, content_root, &mut taken) {
                Ok(slide) => {
                    planned.insert(name.clone(), slide.id.clone());
                    cards.push(slide.id.clone());
                    plan.slides.push(slide);
                }
                Err(why) => plan.refused.push(format!("{name}: {why}")),
            }
        }
        plan.lessons.push(MovingLesson {
            title: crate::episode::lesson_title(&template.title, &chapter.title),
            cards,
        });
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|e| e.path().join(FILE).is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| !planned.contains_key(name))
            .collect();
        names.sort();
        for name in names {
            match plan_slide(dir, &name, content_root, &mut taken) {
                Ok(slide) => plan.orphans.push(slide),
                Err(why) => plan.refused.push(format!("{name}: {why}")),
            }
        }
    }
    Ok(plan)
}

/// One slide folder's part of a plan: where it lands, under which id, and
/// which of its refs follow it.
fn plan_slide(
    dir: &Path,
    name: &str,
    content_root: &Path,
    taken: &mut std::collections::BTreeSet<String>,
) -> Result<MovingSlide, String> {
    let from = dir.join(name);
    if !from.join(FILE).is_file() {
        return Err("the chapter names a folder that is not there".into());
    }
    let slide = crate::load(&from).map_err(|e| format!("does not load: {e}"))?;
    let collides = |id: &str| taken.contains(id) || library_slide_dir(content_root, id).exists();
    let (id, forked) = if collides(&slide.id) {
        let mut fresh = crate::new_id();
        while collides(&fresh) {
            fresh = crate::new_id();
        }
        (fresh, true)
    } else {
        (slide.id.clone(), false)
    };
    taken.insert(id.clone());
    let to = library_slide_dir(content_root, &id);
    let prefix = from
        .strip_prefix(content_root)
        .map(|rel| format!("{}/", rel.to_string_lossy().replace('\\', "/")))
        .unwrap_or_default();
    let after = format!("{}/slides/{id}/", crate::LIBRARY);
    let mut media = Vec::new();
    if !prefix.is_empty() {
        for block in &slide.blocks {
            if let Block::Media { src, poster, .. } = block {
                for path in std::iter::once(src).chain(poster.iter()) {
                    if let Some(rest) = path.strip_prefix(&prefix) {
                        media.push((path.clone(), format!("{after}{rest}")));
                    }
                }
            }
        }
    }
    Ok(MovingSlide {
        name: name.to_owned(),
        script: from.join(crate::script::SCRIPT_FILE).is_file(),
        from,
        to,
        id,
        forked,
        media,
    })
}

/// Move the template episode at `dir` into the library, as [`plan_move`]
/// says: the template `episode.json` is kept as [`EPISODE_BACKUP`] first
/// (never overwritten); every slide folder is renamed into the library; the
/// refs that move with it are rewritten and its script names its card; one
/// lesson per chapter is written; the episode's own `episode.json` becomes
/// `dpl.episode/2` last. A plan with refusals moves nothing; a failure part
/// way puts every folder and file back and leaves the episode as it was.
pub fn move_episode(dir: &Path, content_root: &Path) -> Result<MovePlan, String> {
    let content_root = &crate::writable(content_root).map_err(|e| e.to_string())?;
    let dir = &crate::writable(dir).map_err(|e| e.to_string())?;
    let plan = plan_move(dir, content_root)?;
    if !plan.refused.is_empty() {
        return Err(format!("not moved: {}", plan.refused.join("; ")));
    }
    let backup = dir.join(EPISODE_BACKUP);
    if !backup.exists() {
        std::fs::copy(dir.join(crate::EPISODE_FILE), &backup)
            .map_err(|e| format!("could not keep the old episode.json: {e}"))?;
    }
    let mut undo = Undo::default();
    match carry_out(&plan, content_root, &mut undo) {
        Ok(()) => Ok(plan),
        Err(why) => {
            undo.run();
            Err(format!("not moved, everything put back: {why}"))
        }
    }
}

/// What a failed move takes back, newest first.
#[derive(Default)]
struct Undo {
    renamed: Vec<(PathBuf, PathBuf)>,
    written: Vec<(PathBuf, Vec<u8>)>,
    made: Vec<PathBuf>,
}

impl Undo {
    fn run(self) {
        for (path, bytes) in self.written.into_iter().rev() {
            let _ = std::fs::write(path, bytes);
        }
        for (from, to) in self.renamed.into_iter().rev() {
            let _ = std::fs::rename(to, from);
        }
        for dir in self.made.into_iter().rev() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn carry_out(plan: &MovePlan, content_root: &Path, undo: &mut Undo) -> Result<(), String> {
    for slide in plan.slides.iter().chain(&plan.orphans) {
        if let Some(parent) = slide.to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("library: {e}"))?;
        }
        std::fs::rename(&slide.from, &slide.to)
            .map_err(|e| format!("{} could not move: {e}", slide.name))?;
        undo.renamed.push((slide.from.clone(), slide.to.clone()));
    }
    for slide in plan.slides.iter().chain(&plan.orphans) {
        for file in [FILE, crate::script::SCRIPT_FILE] {
            let path = slide.to.join(file);
            if let Ok(bytes) = std::fs::read(&path) {
                undo.written.push((path, bytes));
            }
        }
        let mut card = crate::load(&slide.to).map_err(|e| format!("{}: {e}", slide.name))?;
        if slide.forked {
            card = card.fork();
            card.id.clone_from(&slide.id);
        }
        for block in &mut card.blocks {
            if let Block::Media { src, poster, .. } = block {
                for path in std::iter::once(src).chain(poster.iter_mut()) {
                    if let Some((_, after)) = slide.media.iter().find(|(now, _)| now == path) {
                        path.clone_from(after);
                    }
                }
            }
        }
        save(&slide.to, &card).map_err(|e| format!("{}: {e}", slide.name))?;
        crate::script::rekey(&slide.to, &slide.id).map_err(|e| format!("{}: {e}", slide.name))?;
    }
    let mut built = crate::episode::LessonEpisode::new(&plan.title);
    for planned in &plan.lessons {
        let mut lesson = crate::lesson::Lesson::new(&planned.title);
        for id in &planned.cards {
            let card = crate::load(&library_slide_dir(content_root, id))
                .map_err(|e| format!("card {id}: {e}"))?;
            lesson.add(&card);
        }
        let at = crate::lesson::lesson_dir(content_root, &lesson.id);
        undo.made.push(at.clone());
        crate::lesson::save_lesson(&at, &lesson).map_err(|e| format!("lesson: {e}"))?;
        built
            .lessons
            .push(crate::episode::LessonRef { id: lesson.id });
    }
    crate::episode::save(&plan.dir, &built).map_err(|e| format!("episode.json: {e}"))
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

    /// A content root holding one old-shape episode: the seed's folders
    /// copied by hand (never the operator's content), with a script and a
    /// picture inside a slide folder.
    fn old_episode(tag: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("dpl-move-{tag}-{}", crate::new_id()));
        let dir = root.join("devpulse-news").join("ep1");
        let mut episode = crate::Episode {
            v: crate::SCHEMA_V,
            id: "ep1".into(),
            title: "Neon Drop".into(),
            chapters: Vec::new(),
        };
        for (chapter, names) in [("intro", vec!["title", "pic"]), ("end", vec!["wrap"])] {
            for name in &names {
                let mut slide = Slide::new("title@1");
                slide.blocks = vec![Block::text("title", format!("slide {name}"))];
                if *name == "pic" {
                    slide.blocks.push(Block::media(
                        "media",
                        "image",
                        "devpulse-news/ep1/pic/assets/shot.png",
                    ));
                }
                save(&dir.join(name), &slide).unwrap();
            }
            episode.chapters.push(crate::Chapter {
                id: chapter.into(),
                title: chapter.to_uppercase(),
                slides: names.iter().map(|n| (*n).to_owned()).collect(),
            });
        }
        std::fs::create_dir_all(dir.join("pic").join("assets")).unwrap();
        std::fs::write(dir.join("pic").join("assets").join("shot.png"), b"png").unwrap();
        std::fs::write(
            dir.join("title").join(crate::script::SCRIPT_FILE),
            "---\nslideId: title\ntitle: T\nbeats:\n  - id: setup\n    cue: hi\n---\n",
        )
        .unwrap();
        crate::save_episode(&dir, &episode).unwrap();
        (root, dir)
    }

    /// The dry run says exactly what would move and changes nothing.
    #[test]
    fn the_plan_names_every_move_and_touches_nothing() {
        let (root, dir) = old_episode("plan");
        let before = std::fs::read_to_string(dir.join(crate::EPISODE_FILE)).unwrap();
        let plan = plan_move(&dir, &root).unwrap();
        assert!(plan.refused.is_empty(), "{:?}", plan.refused);
        let names: Vec<&str> = plan.slides.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["title", "pic", "wrap"]);
        let titles: Vec<&str> = plan.lessons.iter().map(|l| l.title.as_str()).collect();
        assert_eq!(titles, ["Neon Drop — INTRO", "Neon Drop — END"]);
        assert!(plan.slides[0].script);
        let pic = &plan.slides[1];
        assert_eq!(
            pic.media,
            vec![(
                "devpulse-news/ep1/pic/assets/shot.png".to_owned(),
                format!("library/slides/{}/assets/shot.png", pic.id)
            )]
        );
        assert_eq!(old_shape_episodes(&root).len(), 1);
        assert_eq!(
            std::fs::read_to_string(dir.join(crate::EPISODE_FILE)).unwrap(),
            before
        );
        assert!(dir.join("title").is_dir(), "nothing moved");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The move: folders into the library, refs and scripts following, a
    /// lesson per chapter, the episode built from them, the old file kept —
    /// and the episode shows the same cards in the same order.
    #[test]
    fn a_moved_episode_shows_the_same_cards_from_the_library() {
        let (root, dir) = old_episode("move");
        let order = |shown: &crate::episode::Shown| -> Vec<String> {
            shown.cards().map(|c| c.label.clone()).collect()
        };
        let before = order(&crate::episode::shows(&dir, &root).unwrap());
        let plan = move_episode(&dir, &root).unwrap();
        assert!(
            dir.join(EPISODE_BACKUP).is_file(),
            "the old episode.json is kept"
        );
        assert!(!dir.join("title").exists(), "the folders left the episode");
        let shown = crate::episode::shows(&dir, &root).unwrap();
        assert_eq!(order(&shown), before);
        assert_eq!(shown.issues, Vec::new());
        let pic = &plan.slides[1];
        assert!(pic.to.join("assets").join("shot.png").is_file());
        let card = crate::load(&pic.to).unwrap();
        assert!(matches!(
            card.block("media"),
            Some(Block::Media { src, .. }) if src == &format!("library/slides/{}/assets/shot.png", pic.id)
        ));
        let script = crate::script::load_script(&plan.slides[0].to)
            .unwrap()
            .unwrap();
        assert_eq!(script.slide_id, plan.slides[0].id);
        let crate::episode::Opened::Lessons(built) = crate::episode::open(&dir).unwrap() else {
            panic!("built from lessons");
        };
        assert_eq!(crate::episode::validate(&dir, &built, &root), Vec::new());
        assert!(old_shape_episodes(&root).is_empty());
        assert!(
            plan_move(&dir, &root).is_err(),
            "a moved episode is not moved twice"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A slide that will not load refuses the whole episode: nothing moves.
    #[test]
    fn a_broken_slide_refuses_the_move_and_nothing_moves() {
        let (root, dir) = old_episode("refused");
        std::fs::write(dir.join("wrap").join(FILE), "{ not json").unwrap();
        let before = std::fs::read_to_string(dir.join(crate::EPISODE_FILE)).unwrap();
        let err = move_episode(&dir, &root).unwrap_err();
        assert!(err.contains("wrap"), "{err}");
        assert_eq!(
            std::fs::read_to_string(dir.join(crate::EPISODE_FILE)).unwrap(),
            before
        );
        assert!(dir.join("title").is_dir() && !dir.join(EPISODE_BACKUP).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// An id already in the library is not overwritten: that folder moves
    /// as a copy with its own id; an orphan folder moves too.
    #[test]
    fn a_taken_id_moves_as_a_copy_and_orphans_come_along() {
        let (root, dir) = old_episode("taken");
        let wrap = crate::load(&dir.join("wrap")).unwrap();
        save(&library_slide_dir(&root, &wrap.id), &wrap).unwrap();
        save(&dir.join("stray"), &Slide::new("title@1")).unwrap();
        let plan = move_episode(&dir, &root).unwrap();
        let moved = plan.slides.iter().find(|s| s.name == "wrap").unwrap();
        assert!(moved.forked && moved.id != wrap.id);
        assert_eq!(
            crate::load(&moved.to)
                .unwrap()
                .provenance
                .forked_from
                .map(|p| p.id),
            Some(wrap.id.clone())
        );
        assert!(
            crate::load(&library_slide_dir(&root, &wrap.id)).is_ok(),
            "the first stays"
        );
        assert_eq!(plan.orphans.len(), 1);
        assert!(plan.orphans[0].to.join(FILE).is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
