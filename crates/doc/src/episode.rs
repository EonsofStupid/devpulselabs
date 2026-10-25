//! Episodes built from lessons (LS3b, R285). An episode is a document,
//! `dpl.episode/2`, in its show's folder (`<content>/<show>/<episode>/
//! episode.json`): a title and its lessons, in order, by id. The lessons
//! float — a lesson's change shows in every episode using it (ruled
//! 2026-10-24, "Show lessons as they are"); the lessons pin their cards.
//!
//! A **template** is the other shape an episode folder can hold: chapters
//! of slide folders ([`crate::Episode`]). Every seed is one — a seed cannot
//! point at lessons, which live in the operator's content — and a content
//! episode is one until LS3b2 moves its slides into the library. Cloning a
//! seed turns its template into an episode built from lessons.
//!
//! Every surface reads an episode through ONE walk, [`shows`]: the chapters
//! (lessons, or a template's chapters), each card at the version shown, and
//! the episode's theme — never inferred from where a slide lives.
//! Plan: docs/plans/lessons-ls3.md §4.5b.

use crate::lesson::{self, Lesson};
use crate::{script, DocError, Episode, Issue, Slide, Theme, EPISODE_FILE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The episode document's schema id.
pub const EPISODE_SCHEMA: &str = "dpl.episode/2";

/// An episode built from lessons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonEpisode {
    pub schema: String,
    /// Permanent ULID — the folder name is the episode's address.
    pub id: String,
    pub title: String,
    /// Its lessons, in teaching order.
    pub lessons: Vec<LessonRef>,
}

/// One lesson of an episode, by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonRef {
    pub id: String,
}

/// What an episode folder holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Opened {
    /// Chapters of slide folders — a seed, or a content episode LS3b2 has
    /// not moved yet.
    Template(Episode),
    /// Lessons, by id.
    Lessons(LessonEpisode),
}

impl LessonEpisode {
    /// A fresh episode with nothing in it.
    pub fn new(title: &str) -> Self {
        Self {
            schema: EPISODE_SCHEMA.to_owned(),
            id: crate::new_id(),
            title: title.trim().to_owned(),
            lessons: Vec::new(),
        }
    }
}

/// Open `<dir>/episode.json`. The `schema` is read first: `dpl.episode/2`
/// is an episode built from lessons, no schema is a template, anything
/// else is [`DocError::Schema`] — never a misread.
pub fn open(dir: &Path) -> Result<Opened, DocError> {
    let raw = std::fs::read_to_string(dir.join(EPISODE_FILE)).map_err(DocError::Io)?;
    let value: Value = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    match value.get("schema").and_then(Value::as_str) {
        Some(EPISODE_SCHEMA) => serde_json::from_value(value)
            .map(Opened::Lessons)
            .map_err(DocError::Parse),
        Some(other) => Err(DocError::Schema(other.to_owned())),
        None => crate::load_episode(dir).map(Opened::Template),
    }
}

/// Save an episode built from lessons to `<dir>/episode.json`.
pub fn save(dir: &Path, episode: &LessonEpisode) -> Result<(), DocError> {
    crate::save_json(dir, EPISODE_FILE, episode)
}

/// The theme an episode folder authors (`theme.json`), else the fallback.
pub fn theme(dir: &Path) -> Theme {
    crate::load_theme(dir)
        .ok()
        .flatten()
        .unwrap_or_else(Theme::fallback)
}

/// An episode as every surface shows it.
#[derive(Debug, Clone)]
pub struct Shown {
    pub title: String,
    /// The episode's own theme — handed to every card, never inferred.
    pub theme: Theme,
    pub chapters: Vec<ShownChapter>,
    /// What could not be shown, named: a missing lesson, a card that will
    /// not load. The rest still shows.
    pub issues: Vec<Issue>,
}

/// One chapter: a lesson of an episode, or a template's chapter.
#[derive(Debug, Clone)]
pub struct ShownChapter {
    /// The lesson's id, or the template chapter's id.
    pub id: String,
    pub title: String,
    pub cards: Vec<ShownCard>,
}

/// One card, at the version the episode shows.
#[derive(Debug, Clone)]
pub struct ShownCard {
    /// The use's key in its lesson, or the folder name in a template.
    pub key: String,
    /// The card at the version shown — or why it will not load, so a
    /// surface degrades to an honest plate, never a crash.
    pub slide: Result<Slide, String>,
    /// What the card is called — [`Slide::label`], or what is wrong.
    pub label: String,
    /// Where the card is edited: the library card, or the template folder.
    pub dir: PathBuf,
    /// The card's script, when it has one.
    pub script: Option<script::Script>,
    /// Its position in the whole episode.
    pub index: usize,
}

impl Shown {
    /// Every card, in episode order.
    pub fn cards(&self) -> impl Iterator<Item = &ShownCard> {
        self.chapters.iter().flat_map(|c| c.cards.iter())
    }

    /// The chapter holding the card at episode position `index`.
    pub fn chapter_of(&self, index: usize) -> Option<&ShownChapter> {
        self.chapters
            .iter()
            .find(|c| c.cards.iter().any(|card| card.index == index))
    }
}

/// THE walk: an episode folder as every surface shows it — its chapters,
/// each card at the version shown (a lesson's pin, or a template's folder),
/// and its theme. Lessons are found in `content_root`'s library. Fails only
/// when `episode.json` itself cannot be read; anything else is named in
/// [`Shown::issues`] — a missing lesson is skipped, a card that will not
/// load stays in its place as an error.
pub fn shows(dir: &Path, content_root: &Path) -> Result<Shown, DocError> {
    let opened = open(dir)?;
    let mut issues = Vec::new();
    let mut index = 0;
    let mut card =
        |slide: Result<Slide, String>, key: String, at: PathBuf, issues: &mut Vec<Issue>| {
            if let Err(e) = &slide {
                issues.push(Issue {
                    path: key.clone(),
                    message: format!("card does not load: {e}"),
                });
            }
            let script = match script::load_script(&at) {
                Ok(script) => script,
                Err(e) => {
                    issues.push(Issue {
                        path: format!("{key} → script"),
                        message: e,
                    });
                    None
                }
            };
            let label = match &slide {
                Ok(slide) => slide.label(),
                Err(_) => format!("{key} — will not load"),
            };
            let shown = ShownCard {
                key,
                label,
                slide,
                dir: at,
                script,
                index,
            };
            index += 1;
            shown
        };
    let mut chapters = Vec::new();
    let title = match opened {
        Opened::Template(ep) => {
            for chapter in &ep.chapters {
                let mut cards = Vec::new();
                for name in &chapter.slides {
                    let at = dir.join(name);
                    let slide = crate::load(&at).map_err(|e| e.to_string());
                    cards.push(card(slide, name.clone(), at, &mut issues));
                }
                chapters.push(ShownChapter {
                    id: chapter.id.clone(),
                    title: chapter.title.clone(),
                    cards,
                });
            }
            ep.title
        }
        Opened::Lessons(ep) => {
            for (i, lesson_ref) in ep.lessons.iter().enumerate() {
                let at = format!("lessons[{i}]");
                let Ok(found) =
                    lesson::load_lesson(&lesson::lesson_dir(content_root, &lesson_ref.id))
                else {
                    issues.push(Issue {
                        path: at,
                        message: format!("lesson {} is not in your library", lesson_ref.id),
                    });
                    continue;
                };
                let mut cards = Vec::new();
                for use_ in &found.uses {
                    let slide = lesson::load_use(content_root, use_).map_err(|e| e.to_string());
                    let card_dir = crate::library_slide_dir(content_root, &use_.slide.id);
                    cards.push(card(slide, use_.key.clone(), card_dir, &mut issues));
                }
                chapters.push(ShownChapter {
                    id: found.id,
                    title: found.title,
                    cards,
                });
            }
            ep.title
        }
    };
    Ok(Shown {
        title,
        theme: theme(dir),
        chapters,
        issues,
    })
}

/// An episode at a glance — for pickers and counts, without loading a
/// single card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub title: String,
    /// Each chapter (a lesson, or a template chapter): id, title, and how
    /// many cards it shows.
    pub chapters: Vec<(String, String, usize)>,
}

impl Summary {
    /// How many cards the whole episode shows.
    pub fn cards(&self) -> usize {
        self.chapters.iter().map(|(_, _, n)| n).sum()
    }
}

/// [`Summary`] of the episode at `dir`, or `None` when it cannot be read.
/// A lesson that cannot be found is left out (the walk names it).
pub fn summary(dir: &Path, content_root: &Path) -> Option<Summary> {
    Some(match open(dir).ok()? {
        Opened::Template(ep) => Summary {
            chapters: ep
                .chapters
                .iter()
                .map(|c| (c.id.clone(), c.title.clone(), c.slides.len()))
                .collect(),
            title: ep.title,
        },
        Opened::Lessons(ep) => Summary {
            chapters: ep
                .lessons
                .iter()
                .filter_map(|l| lesson::load_lesson(&lesson::lesson_dir(content_root, &l.id)).ok())
                .map(|found| (found.id, found.title, found.uses.len()))
                .collect(),
            title: ep.title,
        },
    })
}

/// What a chapter becomes when a template turns into lessons (ruled
/// 2026-10-24, "Episode — Chapter"): "How AI works — Intro".
pub fn lesson_title(episode: &str, chapter: &str) -> String {
    match (episode.trim(), chapter.trim()) {
        ("", chapter) => chapter.to_owned(),
        (episode, "") => episode.to_owned(),
        (episode, chapter) => format!("{episode} — {chapter}"),
    }
}

/// Every lesson of the episode at `dir`, loaded, in order (an episode
/// built from lessons; a template has none).
pub fn lessons_of(dir: &Path, content_root: &Path) -> Vec<(PathBuf, Lesson)> {
    let Ok(Opened::Lessons(ep)) = open(dir) else {
        return Vec::new();
    };
    ep.lessons
        .iter()
        .filter_map(|l| {
            let at = lesson::lesson_dir(content_root, &l.id);
            lesson::load_lesson(&at).ok().map(|found| (at, found))
        })
        .collect()
}

/// An edit made from an episode is that episode's own change (ruled
/// 2026-10-24, "Show it straight away"): every use of the card `slide_id`
/// in the episode's lessons moves to revision `newest`. Returns how many
/// uses moved; every other lesson using the card still asks.
pub fn take_edit(
    dir: &Path,
    content_root: &Path,
    slide_id: &str,
    newest: u32,
) -> Result<usize, DocError> {
    let mut moved = 0;
    for (at, mut found) in lessons_of(dir, content_root) {
        let keys: Vec<String> = found
            .uses
            .iter()
            .filter(|u| u.slide.id == slide_id && u.slide.rev != newest)
            .map(|u| u.key.clone())
            .collect();
        if keys.is_empty() {
            continue;
        }
        for key in &keys {
            found.update(key, newest);
        }
        lesson::save_lesson(&at, &found)?;
        moved += keys.len();
    }
    Ok(moved)
}

/// The doctor for any episode folder: a template's rules or an episode
/// built from lessons' — whichever it holds. Empty = clean.
pub fn doctor(dir: &Path, content_root: &Path) -> Vec<Issue> {
    match open(dir) {
        Ok(Opened::Template(ep)) => crate::validate_episode(dir, &ep),
        Ok(Opened::Lessons(ep)) => validate(dir, &ep, content_root),
        Err(e) => vec![Issue {
            path: EPISODE_FILE.into(),
            message: format!("does not load: {e}"),
        }],
    }
}

/// The folder an episode built from lessons may hold — its slides live in
/// the library now, so a slide folder here is a stray.
const RESERVED: &[&str] = &["episode.json", "episode.v1.json", "theme.json", "assets"];

/// Name every problem with an episode built from lessons. Empty = clean.
pub fn validate(dir: &Path, episode: &LessonEpisode, content_root: &Path) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });
    if episode.schema != EPISODE_SCHEMA {
        push(
            "schema".into(),
            format!("{:?} is not {EPISODE_SCHEMA:?}", episode.schema),
        );
    }
    if !crate::is_id(&episode.id) {
        push(
            "id".into(),
            format!("{:?} is not an episode id", episode.id),
        );
    }
    if episode.title.trim().is_empty() {
        push("title".into(), "an episode needs a name".into());
    }
    if episode.lessons.is_empty() {
        push("lessons".into(), "episode has no lessons".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, lesson_ref) in episode.lessons.iter().enumerate() {
        let at = format!("lessons[{i}]");
        if !seen.insert(lesson_ref.id.as_str()) {
            push(
                at.clone(),
                format!("lesson {} is in it twice", lesson_ref.id),
            );
        }
        let Ok(found) = lesson::load_lesson(&lesson::lesson_dir(content_root, &lesson_ref.id))
        else {
            push(
                at,
                format!("lesson {} is not in your library", lesson_ref.id),
            );
            continue;
        };
        for sub in lesson::validate_lesson(&found) {
            push(format!("{at} → {}", sub.path), sub.message);
        }
        for use_ in &found.uses {
            let card_at = format!("{at}.{}", use_.key);
            match lesson::load_use(content_root, use_) {
                Ok(slide) => {
                    for sub in crate::validate(&slide) {
                        push(format!("{card_at} → {}", sub.path), sub.message);
                    }
                    let card_dir = crate::library_slide_dir(content_root, &use_.slide.id);
                    match script::load_script(&card_dir) {
                        Ok(Some(s)) => {
                            if s.slide_id != slide.id {
                                push(
                                    format!("{card_at} → script"),
                                    format!(
                                        "script names slideId {:?} but belongs to card {}",
                                        s.slide_id, slide.id
                                    ),
                                );
                            }
                            for sub in script::parity_issues(&slide, &s) {
                                push(format!("{card_at} → {}", sub.path), sub.message);
                            }
                        }
                        Ok(None) => {}
                        Err(e) => push(format!("{card_at} → script"), e),
                    }
                }
                Err(e) => push(card_at, format!("card does not load: {e}")),
            }
        }
    }
    match crate::load_theme(dir) {
        Ok(Some(theme)) => {
            for sub in crate::validate_theme(&theme) {
                push(format!("theme.{}", sub.path), sub.message);
            }
        }
        Ok(None) => {}
        Err(e) => push("theme".into(), format!("theme.json does not load: {e}")),
    }
    crate::check_folder_contract(dir, "folder", RESERVED, false, &mut issues);
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Block;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-episode-{tag}-{}", crate::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A library card saying `words`, saved.
    fn card(root: &Path, words: &str) -> Slide {
        let mut slide = Slide::new("title@1");
        slide.blocks = vec![Block::text("title", words)];
        crate::save(&crate::library_slide_dir(root, &slide.id), &slide).unwrap();
        slide
    }

    /// An episode at `<root>/show/ep` built from `lessons`, each a lesson
    /// of the given cards.
    fn built(root: &Path, lessons: &[(&str, &[&Slide])]) -> (PathBuf, LessonEpisode) {
        let dir = root.join("show").join("ep");
        let mut ep = LessonEpisode::new("The show");
        for (title, cards) in lessons {
            let (lesson_dir, mut found) = lesson::create_lesson(root, title).unwrap();
            for slide in *cards {
                found.add(slide);
            }
            lesson::save_lesson(&lesson_dir, &found).unwrap();
            ep.lessons.push(LessonRef { id: found.id });
        }
        save(&dir, &ep).unwrap();
        (dir, ep)
    }

    #[test]
    fn the_schema_decides_what_an_episode_folder_holds() {
        let root = scratch("open");
        let (dir, ep) = built(&root, &[]);
        assert_eq!(open(&dir).unwrap(), Opened::Lessons(ep));
        // A template has no schema — the seed is one.
        let (_, seed) = crate::list_episodes().into_iter().next().expect("a seed");
        assert!(matches!(open(&seed).unwrap(), Opened::Template(_)));
        // The template loader names an episode built from lessons.
        assert!(matches!(
            crate::load_episode(&dir),
            Err(DocError::Schema(s)) if s == EPISODE_SCHEMA
        ));
        // Any other schema is named, never misread.
        std::fs::write(dir.join(EPISODE_FILE), "{\"schema\":\"dpl.episode/9\"}").unwrap();
        assert!(matches!(open(&dir), Err(DocError::Schema(s)) if s == "dpl.episode/9"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// One walk for both shapes: a seed template and an episode built from
    /// lessons give the same chapters → cards → theme.
    #[test]
    fn one_walk_shows_a_template_and_an_episode_alike() {
        let (_, seed) = crate::list_episodes().into_iter().next().expect("a seed");
        let template = shows(&seed, Path::new("/nowhere")).unwrap();
        assert_eq!(template.issues, Vec::new());
        assert!(!template.chapters.is_empty());
        assert_eq!(
            template.theme,
            crate::load_theme(&seed).unwrap().unwrap(),
            "the episode's own theme"
        );
        let positions: Vec<usize> = template.cards().map(|c| c.index).collect();
        assert_eq!(positions, (0..positions.len()).collect::<Vec<_>>());

        let root = scratch("walk");
        let a = card(&root, "First");
        let b = card(&root, "Second");
        let (dir, _) = built(&root, &[("Intro", &[&a, &b]), ("Wrap", &[&a])]);
        let shown = shows(&dir, &root).unwrap();
        assert_eq!(shown.title, "The show");
        assert_eq!(shown.theme, Theme::fallback());
        let labels: Vec<&str> = shown.cards().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["First", "Second", "First"]);
        assert_eq!(shown.chapter_of(2).unwrap().title, "Wrap");
        assert_eq!(
            shown.cards().next().unwrap().dir,
            crate::library_slide_dir(&root, &a.id)
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The lessons float and pin their cards: a card's new revision shows
    /// only once its lesson takes it.
    #[test]
    fn an_episode_shows_each_card_at_its_lessons_pin() {
        let root = scratch("pins");
        let mut a = card(&root, "Old words");
        let (dir, _) = built(&root, &[("Intro", &[&a])]);
        a.blocks = vec![Block::text("title", "New words")];
        a.rev = 2;
        crate::save(&crate::library_slide_dir(&root, &a.id), &a).unwrap();
        assert_eq!(
            shows(&dir, &root).unwrap().cards().next().unwrap().label,
            "Old words"
        );
        // An edit made from the episode is the episode's own change.
        assert_eq!(take_edit(&dir, &root, &a.id, 2).unwrap(), 1);
        assert_eq!(
            shows(&dir, &root).unwrap().cards().next().unwrap().label,
            "New words"
        );
        assert_eq!(
            take_edit(&dir, &root, &a.id, 2).unwrap(),
            0,
            "nothing left to move"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// What cannot be shown is named and the rest still shows.
    #[test]
    fn a_missing_lesson_or_card_is_named_and_the_rest_shows() {
        let root = scratch("missing");
        let a = card(&root, "Here");
        let gone = Slide::new("title@1");
        let (dir, mut ep) = built(&root, &[("Intro", &[&a, &gone])]);
        ep.lessons.push(LessonRef {
            id: crate::new_id(),
        });
        save(&dir, &ep).unwrap();
        let shown = shows(&dir, &root).unwrap();
        let loaded: Vec<bool> = shown.cards().map(|c| c.slide.is_ok()).collect();
        assert_eq!(loaded, [true, false], "the broken card keeps its place");
        assert_eq!(shown.issues.len(), 2, "{:?}", shown.issues);
        let issues = validate(&dir, &ep, &root);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("not in your library")));
        assert!(issues
            .iter()
            .any(|i| i.message.contains("card does not load")));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A slide folder left inside an episode built from lessons is a stray —
    /// its slides live in the library.
    #[test]
    fn a_slide_folder_in_an_episode_built_from_lessons_is_a_stray() {
        let root = scratch("stray");
        let a = card(&root, "Card");
        let (dir, ep) = built(&root, &[("Intro", &[&a])]);
        assert_eq!(validate(&dir, &ep, &root), Vec::new());
        crate::save(&dir.join("old-slide"), &Slide::new("title@1")).unwrap();
        assert!(validate(&dir, &ep, &root)
            .iter()
            .any(|i| i.message.contains("old-slide")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_chapter_becomes_episode_then_chapter() {
        assert_eq!(
            lesson_title("How AI works", "Intro"),
            "How AI works — Intro"
        );
        assert_eq!(lesson_title("", "Intro"), "Intro");
        assert_eq!(lesson_title("How AI works", " "), "How AI works");
    }

    /// Every episode this build writes passes `schemas/episode.v2.schema.json`.
    #[test]
    fn episodes_pass_the_episode_schema() {
        let schema: Value = serde_json::from_str(
            &std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/episode.v2.schema.json"),
            )
            .expect("the episode schema"),
        )
        .expect("the schema is JSON");
        let validator = jsonschema::validator_for(&schema).expect("the schema compiles");
        let root = scratch("schema");
        let a = card(&root, "Card");
        let (_, ep) = built(&root, &[("Intro", &[&a]), ("Wrap", &[])]);
        let doc = serde_json::to_value(&ep).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&doc)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        let mut stray = doc.clone();
        stray["chapters"] = Value::Array(Vec::new());
        assert!(!validator.is_valid(&stray), "closed");
        let _ = std::fs::remove_dir_all(&root);
    }
}
