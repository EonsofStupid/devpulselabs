//! Lessons (LS3, R285). A lesson is a document, `dpl.lesson/1`, kept in
//! the library beside the slides it uses
//! (`<content>/library/lessons/<id>/lesson.json`). It never holds a slide's
//! content: each **use** points at a library slide by id and pins one
//! revision of it. When the slide moves on, the use says so — Update
//! available — and the operator takes the newer version or keeps theirs
//! (a "Not now" is remembered for that version). Looks, presets and
//! components float, so refreshing visuals never touches a lesson.
//!
//! Plan: docs/plans/lessons-ls3.md.

use crate::{DocError, Issue, Slide, SlidePin, LIBRARY};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The lesson document's schema id.
pub const LESSON_SCHEMA: &str = "dpl.lesson/1";

const LESSON_FILE: &str = "lesson.json";

/// One lesson: a titled, ordered run of uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    pub schema: String,
    /// Permanent ULID — never a folder name.
    pub id: String,
    /// Raised by one on every change; an episode pins it (LS3b).
    pub rev: u32,
    pub title: String,
    /// In teaching order. The same slide may be used more than once.
    pub uses: Vec<Use>,
}

/// One place a slide is shown in a lesson.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Use {
    /// Names this use within its lesson, so one slide can be used twice.
    pub key: String,
    /// The library slide and the revision this lesson shows.
    pub slide: SlidePin,
    /// The job this use does in the lesson — one of [`crate::roles`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The newer revision the operator said "Not now" to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declined: Option<u32>,
    /// Variant axes chosen for this lesson only (LS4a, ruled 2026-10-24: "a
    /// one off variant for that specific lesson") — over the card's own,
    /// without touching the card.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub variant: std::collections::BTreeMap<String, String>,
}

/// Where a use stands against its slide's newest revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseState {
    /// The lesson shows the newest version.
    Current,
    /// The slide has moved on to `newest`.
    UpdateAvailable { newest: u32 },
    /// The slide has moved on to `newest` and the operator said "Not now".
    NotNow { newest: u32 },
    /// The slide is not in the library.
    Missing,
}

impl Lesson {
    /// A fresh lesson with nothing in it.
    pub fn new(title: &str) -> Self {
        Self {
            schema: LESSON_SCHEMA.to_owned(),
            id: crate::new_id(),
            rev: 1,
            title: title.trim().to_owned(),
            uses: Vec::new(),
        }
    }

    /// Give the lesson a new name (a blank one is refused).
    pub fn rename(&mut self, title: &str) -> bool {
        let title = title.trim();
        if title.is_empty() || title == self.title {
            return false;
        }
        title.clone_into(&mut self.title);
        self.rev += 1;
        true
    }

    /// The use named `key`.
    pub fn get(&self, key: &str) -> Option<&Use> {
        self.uses.iter().find(|u| u.key == key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Use> {
        self.uses.iter_mut().find(|u| u.key == key)
    }

    /// Use `slide` at the end, pinned to its current revision, with the
    /// role the slide itself names. Returns the new use's key.
    pub fn add(&mut self, slide: &Slide) -> String {
        let n = self
            .uses
            .iter()
            .filter_map(|u| u.key.strip_prefix('u')?.parse::<u32>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        let key = format!("u{n}");
        self.uses.push(Use {
            key: key.clone(),
            slide: SlidePin {
                id: slide.id.clone(),
                rev: slide.rev,
            },
            role: slide.role.clone(),
            declined: None,
            variant: std::collections::BTreeMap::new(),
        });
        self.rev += 1;
        key
    }

    /// Take a use out. The slide stays in the library.
    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.uses.len();
        self.uses.retain(|u| u.key != key);
        let removed = self.uses.len() != before;
        if removed {
            self.rev += 1;
        }
        removed
    }

    /// Move a use to position `to` (clamped to the end).
    pub fn move_to(&mut self, key: &str, to: usize) -> bool {
        let Some(from) = self.uses.iter().position(|u| u.key == key) else {
            return false;
        };
        let to = to.min(self.uses.len() - 1);
        if from == to {
            return false;
        }
        let moving = self.uses.remove(from);
        self.uses.insert(to, moving);
        self.rev += 1;
        true
    }

    /// Give a use its job in the lesson, or none.
    pub fn set_role(&mut self, key: &str, role: Option<&str>) -> bool {
        let Some(found) = self.get_mut(key) else {
            return false;
        };
        if found.role.as_deref() == role {
            return false;
        }
        found.role = role.map(str::to_owned);
        self.rev += 1;
        true
    }

    /// "Just this lesson": axis `axis` shows option `option` here only;
    /// `None` gives it back to the card's own choice.
    pub fn set_variant(&mut self, key: &str, axis: &str, option: Option<&str>) -> bool {
        let Some(found) = self.get_mut(key) else {
            return false;
        };
        let changed = match option {
            Some(option) => {
                found.variant.insert(axis.to_owned(), option.to_owned()) != Some(option.to_owned())
            }
            None => found.variant.remove(axis).is_some(),
        };
        if changed {
            self.rev += 1;
        }
        changed
    }

    /// "Update": show revision `newest` from now on.
    pub fn update(&mut self, key: &str, newest: u32) -> bool {
        let Some(found) = self.get_mut(key) else {
            return false;
        };
        if found.slide.rev == newest && found.declined.is_none() {
            return false;
        }
        found.slide.rev = newest;
        found.declined = None;
        self.rev += 1;
        true
    }

    /// "Not now": keep the pinned version and stop asking about `newest`.
    pub fn not_now(&mut self, key: &str, newest: u32) -> bool {
        let Some(found) = self.get_mut(key) else {
            return false;
        };
        if found.declined == Some(newest) {
            return false;
        }
        found.declined = Some(newest);
        self.rev += 1;
        true
    }
}

/// Where the library keeps lessons: `<content>/library/lessons/`.
pub fn lessons_dir(content_root: &Path) -> PathBuf {
    content_root.join(LIBRARY).join("lessons")
}

/// Where the library keeps one lesson: `<content>/library/lessons/<id>/`.
pub fn lesson_dir(content_root: &Path, id: &str) -> PathBuf {
    lessons_dir(content_root).join(id)
}

/// Load `<dir>/lesson.json`. The `schema` is read first, so a different
/// document is [`DocError::Schema`], never a misread.
pub fn load_lesson(dir: &Path) -> Result<Lesson, DocError> {
    let raw = std::fs::read_to_string(dir.join(LESSON_FILE)).map_err(DocError::Io)?;
    let value: Value = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    match value.get("schema").and_then(Value::as_str) {
        Some(LESSON_SCHEMA) => serde_json::from_value(value).map_err(DocError::Parse),
        Some(other) => Err(DocError::Schema(other.to_owned())),
        None => Err(DocError::Schema("none".into())),
    }
}

/// Save to `<dir>/lesson.json`, keeping the revision it replaces.
pub fn save_lesson(dir: &Path, lesson: &Lesson) -> Result<(), DocError> {
    crate::keep_superseded(dir, LESSON_FILE, &lesson.id, lesson.rev)?;
    crate::save_json(dir, LESSON_FILE, lesson)
}

/// "New lesson": a fresh, empty lesson saved in the library. A blank name
/// becomes "New lesson".
pub fn create_lesson(content_root: &Path, title: &str) -> Result<(PathBuf, Lesson), DocError> {
    let title = if title.trim().is_empty() {
        "New lesson"
    } else {
        title
    };
    let lesson = Lesson::new(title);
    let dir = lesson_dir(content_root, &lesson.id);
    save_lesson(&dir, &lesson)?;
    Ok((dir, lesson))
}

/// Every lesson that uses the library slide `slide_id`, as its folder.
pub fn lessons_using(content_root: &Path, slide_id: &str) -> Vec<PathBuf> {
    list_lessons(content_root)
        .into_iter()
        .filter(|(_, lesson)| {
            lesson
                .as_ref()
                .is_ok_and(|l| l.uses.iter().any(|u| u.slide.id == slide_id))
        })
        .map(|(dir, _)| dir)
        .collect()
}

/// Every lesson folder in the library, newest first (ids sort by time),
/// each loaded — a broken one is listed with what is wrong, never hidden.
pub fn list_lessons(content_root: &Path) -> Vec<(PathBuf, Result<Lesson, String>)> {
    let Ok(entries) = std::fs::read_dir(lessons_dir(content_root)) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(LESSON_FILE).is_file())
        .collect();
    dirs.sort();
    dirs.reverse();
    dirs.into_iter()
        .map(|dir| {
            let lesson = load_lesson(&dir).map_err(|e| e.to_string());
            (dir, lesson)
        })
        .collect()
}

/// The newest revision of a library slide, or `None` when it is not there.
pub fn newest_rev(content_root: &Path, slide_id: &str) -> Option<u32> {
    crate::load(&crate::library_slide_dir(content_root, slide_id))
        .ok()
        .map(|slide| slide.rev)
}

/// Where `use_` stands against its slide's newest revision.
pub fn use_state(content_root: &Path, use_: &Use) -> UseState {
    match newest_rev(content_root, &use_.slide.id) {
        None => UseState::Missing,
        Some(newest) if newest <= use_.slide.rev => UseState::Current,
        Some(newest) if use_.declined == Some(newest) => UseState::NotNow { newest },
        Some(newest) => UseState::UpdateAvailable { newest },
    }
}

/// The slide exactly as this use shows it: its pinned revision, with this
/// lesson's own variant choices over the card's.
pub fn load_use(content_root: &Path, use_: &Use) -> Result<Slide, DocError> {
    let mut slide = crate::load_rev(
        &crate::library_slide_dir(content_root, &use_.slide.id),
        use_.slide.rev,
    )?;
    for (axis, option) in &use_.variant {
        slide.variant.insert(axis.clone(), option.clone());
    }
    Ok(slide)
}

/// Every use with an update waiting (not the ones put off), as
/// `(key, newest)`.
pub fn updates(content_root: &Path, lesson: &Lesson) -> Vec<(String, u32)> {
    lesson
        .uses
        .iter()
        .filter_map(|use_| match use_state(content_root, use_) {
            UseState::UpdateAvailable { newest } => Some((use_.key.clone(), newest)),
            _ => None,
        })
        .collect()
}

/// "Update all": take every waiting update. Returns how many it took.
pub fn update_all(content_root: &Path, lesson: &mut Lesson) -> usize {
    let waiting = updates(content_root, lesson);
    if waiting.is_empty() {
        return 0;
    }
    for (key, newest) in &waiting {
        if let Some(found) = lesson.get_mut(key) {
            found.slide.rev = *newest;
            found.declined = None;
        }
    }
    lesson.rev += 1;
    waiting.len()
}

/// Name every problem with a lesson. Empty = clean.
pub fn validate_lesson(lesson: &Lesson) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });
    if lesson.schema != LESSON_SCHEMA {
        push(
            "schema".into(),
            format!("{:?} is not {LESSON_SCHEMA:?}", lesson.schema),
        );
    }
    if !crate::is_id(&lesson.id) {
        push("id".into(), format!("{:?} is not a lesson id", lesson.id));
    }
    if lesson.rev == 0 {
        push("rev".into(), "revisions start at 1".into());
    }
    if lesson.title.trim().is_empty() {
        push("title".into(), "a lesson needs a name".into());
    }
    let mut keys = std::collections::BTreeSet::new();
    for (i, use_) in lesson.uses.iter().enumerate() {
        let at = format!("uses[{i}]");
        if use_.key.trim().is_empty() {
            push(format!("{at}.key"), "a use needs a key".into());
        } else if !keys.insert(use_.key.as_str()) {
            push(
                format!("{at}.key"),
                format!("{:?} is used twice in this lesson", use_.key),
            );
        }
        if !crate::is_id(&use_.slide.id) {
            push(
                format!("{at}.slide.id"),
                format!("{:?} is not a slide id", use_.slide.id),
            );
        }
        if use_.slide.rev == 0 {
            push(format!("{at}.slide.rev"), "revisions start at 1".into());
        }
        if let Some(role) = &use_.role {
            if !crate::roles().contains(&role.as_str()) {
                push(format!("{at}.role"), format!("unknown role {role:?}"));
            }
        }
        if let Some(declined) = use_.declined {
            if declined <= use_.slide.rev {
                push(
                    format!("{at}.declined"),
                    format!(
                        "put off version {declined}, which is not newer than the pinned {}",
                        use_.slide.rev
                    ),
                );
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Block;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-lesson-{tag}-{}", crate::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A library slide saying `words`, saved.
    fn library_slide(root: &Path, words: &str) -> Slide {
        let mut slide = Slide::new("title@1");
        slide.blocks = vec![Block::text("title", words)];
        crate::save(&crate::library_slide_dir(root, &slide.id), &slide).unwrap();
        slide
    }

    /// The slide's next revision, saved the way the editor saves one.
    fn edit(root: &Path, slide: &mut Slide, words: &str) {
        slide.blocks = vec![Block::text("title", words)];
        slide.rev += 1;
        crate::save(&crate::library_slide_dir(root, &slide.id), slide).unwrap();
    }

    fn title_of(slide: &Slide) -> String {
        slide
            .block("title")
            .and_then(Block::words)
            .map(|w| w.text.to_owned())
            .unwrap_or_default()
    }

    #[test]
    fn a_lesson_round_trips_and_a_broken_one_is_listed() {
        let root = scratch("roundtrip");
        let one = library_slide(&root, "One");
        let mut lesson = Lesson::new("  How the cost adds up ");
        assert_eq!(lesson.title, "How the cost adds up");
        let key = lesson.add(&one);
        lesson.set_role(&key, Some("present"));
        assert_eq!(validate_lesson(&lesson), Vec::new());
        let dir = lesson_dir(&root, &lesson.id);
        save_lesson(&dir, &lesson).unwrap();
        assert_eq!(load_lesson(&dir).unwrap(), lesson);

        let broken = lessons_dir(&root).join(crate::new_id());
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join(LESSON_FILE), "{\"schema\":\"dpl.lesson/9\"}").unwrap();
        let listed = list_lessons(&root);
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().any(|(_, l)| l.as_ref() == Ok(&lesson)));
        assert!(listed
            .iter()
            .any(|(_, l)| l.as_ref().is_err_and(|e| e.contains("dpl.lesson/9"))));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The point of a pin: after the slide moves on, the lesson still shows
    /// the version it chose — until Update.
    #[test]
    fn a_pinned_use_shows_its_version_after_the_slide_moves_on() {
        let root = scratch("pinned");
        let mut slide = library_slide(&root, "First words");
        let mut lesson = Lesson::new("Pinned");
        let key = lesson.add(&slide);
        edit(&root, &mut slide, "Second words");

        let use_ = lesson.get(&key).unwrap().clone();
        assert_eq!(
            use_state(&root, &use_),
            UseState::UpdateAvailable { newest: 2 }
        );
        assert_eq!(title_of(&load_use(&root, &use_).unwrap()), "First words");

        assert!(lesson.update(&key, 2));
        let use_ = lesson.get(&key).unwrap();
        assert_eq!(use_state(&root, use_), UseState::Current);
        assert_eq!(title_of(&load_use(&root, use_).unwrap()), "Second words");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// "Not now" is remembered for that version only: the next one asks
    /// again.
    #[test]
    fn not_now_is_remembered_until_the_slide_moves_on_again() {
        let root = scratch("not-now");
        let mut slide = library_slide(&root, "v1");
        let mut lesson = Lesson::new("Declines");
        let key = lesson.add(&slide);
        edit(&root, &mut slide, "v2");
        assert!(lesson.not_now(&key, 2));
        assert_eq!(validate_lesson(&lesson), Vec::new());
        let use_ = lesson.get(&key).unwrap().clone();
        assert_eq!(use_state(&root, &use_), UseState::NotNow { newest: 2 });
        assert!(updates(&root, &lesson).is_empty());

        edit(&root, &mut slide, "v3");
        assert_eq!(
            use_state(&root, &use_),
            UseState::UpdateAvailable { newest: 3 }
        );
        assert_eq!(updates(&root, &lesson), vec![(key.clone(), 3)]);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// One slide used twice is two uses; Update all takes every waiting
    /// update in one revision and leaves the put-off ones alone.
    #[test]
    fn update_all_takes_every_waiting_update_once() {
        let root = scratch("update-all");
        let mut a = library_slide(&root, "a1");
        let mut b = library_slide(&root, "b1");
        let mut c = library_slide(&root, "c1");
        let mut lesson = Lesson::new("All");
        let first = lesson.add(&a);
        let again = lesson.add(&a);
        let other = lesson.add(&b);
        let put_off = lesson.add(&c);
        assert_ne!(first, again);
        edit(&root, &mut a, "a2");
        edit(&root, &mut b, "b2");
        edit(&root, &mut c, "c2");
        lesson.not_now(&put_off, 2);
        let rev = lesson.rev;
        assert_eq!(update_all(&root, &mut lesson), 3);
        assert_eq!(lesson.rev, rev + 1);
        for key in [&first, &again, &other] {
            assert_eq!(lesson.get(key).unwrap().slide.rev, 2);
        }
        assert_eq!(lesson.get(&put_off).unwrap().slide.rev, 1);
        assert_eq!(update_all(&root, &mut lesson), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reorder_remove_and_role_are_one_revision_each() {
        let root = scratch("order");
        let a = library_slide(&root, "a");
        let b = library_slide(&root, "b");
        let mut lesson = Lesson::new("Order");
        let ka = lesson.add(&a);
        let kb = lesson.add(&b);
        let rev = lesson.rev;
        assert!(lesson.move_to(&kb, 0));
        assert_eq!(lesson.uses[0].key, kb);
        assert!(!lesson.move_to(&kb, 0), "already there");
        assert!(lesson.set_role(&ka, Some("practice")));
        assert!(!lesson.set_role(&ka, Some("practice")));
        assert!(lesson.remove(&kb));
        assert_eq!(lesson.rev, rev + 3);
        assert_eq!(lesson.uses.len(), 1);
        assert!(
            crate::load(&crate::library_slide_dir(&root, &b.id)).is_ok(),
            "removing a use never touches the slide"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_new_lesson_is_saved_named_and_found_by_its_slides() {
        let root = scratch("create");
        let slide = library_slide(&root, "shared");
        let (dir, mut first) = create_lesson(&root, "  ").unwrap();
        assert_eq!(first.title, "New lesson");
        assert!(first.rename("Costs"));
        assert!(!first.rename("   "), "a blank name is refused");
        first.add(&slide);
        save_lesson(&dir, &first).unwrap();
        let (_, second) = create_lesson(&root, "Unrelated").unwrap();
        assert_eq!(validate_lesson(&second), Vec::new());
        assert_eq!(lessons_using(&root, &slide.id), vec![dir]);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A lesson's earlier revisions are kept like a slide's.
    #[test]
    fn a_lessons_replaced_revision_is_kept() {
        let root = scratch("lesson-revs");
        let slide = library_slide(&root, "x");
        let mut lesson = Lesson::new("Kept");
        let dir = lesson_dir(&root, &lesson.id);
        save_lesson(&dir, &lesson).unwrap();
        lesson.add(&slide);
        save_lesson(&dir, &lesson).unwrap();
        assert!(dir.join(crate::REVS).join("1.json").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_slide_and_bad_fields_are_named() {
        let root = scratch("missing");
        let mut lesson = Lesson::new("");
        let gone = Slide::new("title@1");
        let key = lesson.add(&gone);
        assert_eq!(
            use_state(&root, lesson.get(&key).unwrap()),
            UseState::Missing
        );
        lesson.uses.push(lesson.uses[0].clone());
        lesson.uses[0].role = Some("lecture".into());
        lesson.uses[0].declined = Some(1);
        let issues = validate_lesson(&lesson);
        for needle in ["title", "uses[1].key", "uses[0].role", "uses[0].declined"] {
            assert!(
                issues.iter().any(|i| i.path == needle),
                "{needle}: {issues:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Every lesson this build writes passes `schemas/lesson.v1.schema.json`.
    #[test]
    fn lessons_pass_the_lesson_schema() {
        let schema: Value = serde_json::from_str(
            &std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/lesson.v1.schema.json"),
            )
            .expect("the lesson schema"),
        )
        .expect("the schema is JSON");
        let validator = jsonschema::validator_for(&schema).expect("the schema compiles");
        let mut lesson = Lesson::new("Schema");
        let empty = serde_json::to_value(&lesson).unwrap();
        let slide = Slide::new("title@1");
        let key = lesson.add(&slide);
        lesson.add(&slide);
        lesson.set_role(&key, Some("attention"));
        lesson.not_now(&key, 4);
        for doc in [empty, serde_json::to_value(&lesson).unwrap()] {
            let errors: Vec<String> = validator
                .iter_errors(&doc)
                .map(|e| format!("{} at {}", e, e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{errors:?}");
        }
        // Closed: a stray field is refused.
        let mut stray = serde_json::to_value(&lesson).unwrap();
        stray["content"] = Value::String("never here".into());
        assert!(!validator.is_valid(&stray));
    }

    /// "Just this lesson": the use's own variant shows over the card's,
    /// the card never changes, and the choice can be given back.
    #[test]
    fn a_lessons_own_variant_shows_over_the_cards() {
        let root = scratch("variant");
        let mut card = library_slide(&root, "Hello");
        card.variant.insert("align".into(), "left".into());
        crate::save(&crate::library_slide_dir(&root, &card.id), &card).unwrap();
        let mut lesson = Lesson::new("Shaped");
        let key = lesson.add(&card);
        let rev = lesson.rev;
        assert!(lesson.set_variant(&key, "align", Some("center")));
        assert!(
            !lesson.set_variant(&key, "align", Some("center")),
            "no change, no revision"
        );
        assert_eq!(lesson.rev, rev + 1);
        let shown = load_use(&root, lesson.get(&key).unwrap()).unwrap();
        assert_eq!(
            shown.variant.get("align").map(String::as_str),
            Some("center")
        );
        let stored = crate::load(&crate::library_slide_dir(&root, &card.id)).unwrap();
        assert_eq!(
            stored.variant.get("align").map(String::as_str),
            Some("left")
        );
        assert!(lesson.set_variant(&key, "align", None));
        let shown = load_use(&root, lesson.get(&key).unwrap()).unwrap();
        assert_eq!(shown.variant.get("align").map(String::as_str), Some("left"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
