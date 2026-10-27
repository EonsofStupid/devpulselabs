//! The operator's own looks (LK2, R286). A look is a document,
//! `dpl.look/1`, kept in the library beside the slides that follow it
//! (`<content>/library/looks/<id>/look.json`). [`Looks`] is the built-in
//! looks plus the operator's own; an own look with a built-in's id is the
//! operator's version of it and replaces it for them (ruled 2026-10-21).
//!
//! Looks are passed in, never held globally: this crate stays pure data
//! (docs/design/architecture.md). Plan: docs/plans/linked-looks-lk2.md.

use crate::{Background, Block, DocError, Issue, Slide, Theme, LIBRARY};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The look document's schema id.
pub const LOOK_SCHEMA: &str = "dpl.look/1";

const LOOK_FILE: &str = "look.json";

/// The colour a look takes when the card it is saved from has none —
/// ColdLight's primary accent.
pub const DEFAULT_LOOK_TONE: &str = "cyan";

/// One look: what it gives the slides that follow it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    pub schema: String,
    /// A ULID for the operator's own look; a built-in's id for their
    /// version of that built-in.
    pub id: String,
    /// Every saved change is a new revision (0 for a built-in).
    pub rev: u32,
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub blurb: String,
    /// The entity it dresses (`card` until LK3).
    #[serde(rename = "for")]
    pub entity: String,
    /// The knobs it lets a slide change while following it.
    pub exposes: Vec<String>,
    /// Its colour — a hue id.
    pub tone: String,
    /// The default font for every line of a slide that follows it.
    pub font: String,
    pub background: Background,
    #[serde(default, skip_serializing_if = "LookProvenance::is_empty")]
    pub provenance: LookProvenance,
}

/// Where a look came from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookProvenance {
    /// The look it was saved from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
}

impl LookProvenance {
    pub fn is_empty(&self) -> bool {
        self.forked_from.is_none()
    }
}

impl Look {
    /// A built-in look as a document (rev 0: it ships with the app).
    pub fn builtin(preset: &dpl_coldlight::ThemePreset) -> Self {
        Self {
            schema: LOOK_SCHEMA.into(),
            id: preset.id.into(),
            rev: 0,
            label: preset.label.into(),
            blurb: preset.blurb.into(),
            entity: preset.entity.into(),
            exposes: preset
                .exposes
                .iter()
                .map(|knob| (*knob).to_owned())
                .collect(),
            tone: preset.tone.into(),
            font: preset.font.into(),
            background: Background::MoodBlend {
                moods: vec![preset.moods.0.into(), preset.moods.1.into()],
                motion: preset.motion.into(),
                ambience: preset.ambience.map(str::to_owned),
            },
            provenance: LookProvenance::default(),
        }
    }

    /// Does it dress a whole card (the slide's own look)?
    pub fn is_card(&self) -> bool {
        self.entity == "card"
    }
}

/// The looks a surface can use: the built-in ones plus the operator's own.
#[derive(Debug, Clone)]
pub struct Looks {
    builtin: Vec<Look>,
    /// The operator's looks, newest first (ids sort by time).
    yours: Vec<Look>,
    issues: Vec<Issue>,
}

impl Looks {
    /// Only the built-in looks — for tests and any surface with no library.
    pub fn builtin_only() -> Self {
        Self {
            builtin: dpl_coldlight::THEME_PRESETS
                .iter()
                .map(Look::builtin)
                .collect(),
            yours: Vec::new(),
            issues: Vec::new(),
        }
    }

    /// The built-in looks plus every valid look in the library. A broken
    /// look file is skipped and named in [`Looks::issues`], never a crash.
    pub fn load(content_root: &Path) -> Self {
        let mut looks = Self::builtin_only();
        let Ok(entries) = std::fs::read_dir(looks_dir(content_root)) else {
            return looks;
        };
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.join(LOOK_FILE).is_file())
            .collect();
        dirs.sort();
        dirs.reverse();
        for dir in dirs {
            let name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned();
            match load_look(&dir) {
                Ok(look) => {
                    let mut issues = validate_look(&look);
                    if look.id != name {
                        issues.push(Issue {
                            path: "id".into(),
                            message: format!(
                                "the folder is {name:?} but the look says {:?}",
                                look.id
                            ),
                        });
                    }
                    if issues.is_empty() {
                        looks.yours.push(look);
                    } else {
                        looks.issues.extend(issues.into_iter().map(|issue| Issue {
                            path: format!("{name}/{}", issue.path),
                            message: issue.message,
                        }));
                    }
                }
                Err(e) => looks.issues.push(Issue {
                    path: name,
                    message: e.to_string(),
                }),
            }
        }
        looks
    }

    /// The look with this id: yours first (your version of a built-in
    /// replaces it), else the built-in.
    pub fn get(&self, id: &str) -> Option<&Look> {
        self.yours
            .iter()
            .find(|look| look.id == id)
            .or_else(|| self.builtin(id))
    }

    /// The built-in look with this id, even when yours replaces it.
    pub fn builtin(&self, id: &str) -> Option<&Look> {
        self.builtin.iter().find(|look| look.id == id)
    }

    /// Is this look the operator's own (or their version of a built-in)?
    pub fn is_yours(&self, id: &str) -> bool {
        self.yours.iter().any(|look| look.id == id)
    }

    /// Is this the operator's version of a built-in?
    pub fn replaces_builtin(&self, id: &str) -> bool {
        self.is_yours(id) && self.builtin(id).is_some()
    }

    /// The built-in looks in their authored order, your version in place
    /// of any you replaced.
    pub fn builtins_shown(&self) -> impl Iterator<Item = &Look> {
        self.builtin
            .iter()
            .map(|look| self.get(&look.id).unwrap_or(look))
    }

    /// Your own looks (not versions of built-ins), newest first.
    pub fn own(&self) -> impl Iterator<Item = &Look> {
        self.yours
            .iter()
            .filter(|look| self.builtin(&look.id).is_none())
    }

    /// The look files that could not be read, named.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Take a look that was just saved: it replaces the old revision.
    pub fn put(&mut self, look: Look) {
        self.yours.retain(|have| have.id != look.id);
        self.yours.insert(0, look);
    }

    /// Forget the operator's look `id` in memory, after it stepped aside
    /// ([`retire_look`]): a built-in it replaced shows again.
    pub fn retire(&mut self, id: &str) {
        self.yours.retain(|look| look.id != id);
    }

    /// The look a text, quote or picture block follows (LK3), when it
    /// names a real one for its own entity. The built-in block looks for
    /// now; the operator's own would join them here, as card looks did in LK2.
    pub fn line_look(&self, block: &crate::Block) -> Option<&'static dpl_coldlight::BlockPreset> {
        let id = block.style()?.preset?;
        dpl_coldlight::block_preset(id).filter(|look| look.entity == crate::block_entity(block))
    }

    /// The line looks for one entity (`text` or `quote`), in authored order.
    pub fn line_looks(
        &self,
        entity: &'static str,
    ) -> impl Iterator<Item = &'static dpl_coldlight::BlockPreset> {
        dpl_coldlight::BLOCK_PRESETS
            .iter()
            .filter(move |look| look.entity == entity)
    }
}

/// Where the library keeps looks: `<content>/library/looks/`.
pub fn looks_dir(content_root: &Path) -> PathBuf {
    content_root.join(LIBRARY).join("looks")
}

/// Where the library keeps one look: `<content>/library/looks/<id>/`.
pub fn look_dir(content_root: &Path, id: &str) -> PathBuf {
    looks_dir(content_root).join(id)
}

/// Load `<dir>/look.json`. The `schema` is read first, so a different
/// document is [`DocError::Schema`], never a misread.
pub fn load_look(dir: &Path) -> Result<Look, DocError> {
    let raw = std::fs::read_to_string(dir.join(LOOK_FILE)).map_err(DocError::Io)?;
    let value: Value = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    match value.get("schema").and_then(Value::as_str) {
        Some(LOOK_SCHEMA) => serde_json::from_value(value).map_err(DocError::Parse),
        Some(other) => Err(DocError::Schema(other.to_owned())),
        None => Err(DocError::Schema("none".into())),
    }
}

/// Save to `<dir>/look.json`.
pub fn save_look(dir: &Path, look: &Look) -> Result<(), DocError> {
    crate::save_json(dir, LOOK_FILE, look)
}

/// The knobs a card look may expose (the build's own table).
fn card_knobs() -> &'static [&'static str] {
    dpl_coldlight::PRESET_KNOBS
        .iter()
        .find(|(entity, _)| *entity == "card")
        .map_or(&[], |(_, knobs)| *knobs)
}

/// Name every problem with a look — the same law `build.rs` applies to
/// the built-in ones. Empty = clean.
pub fn validate_look(look: &Look) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: &str, message: String| {
        issues.push(Issue {
            path: path.into(),
            message,
        });
    };
    if look.schema != LOOK_SCHEMA {
        push(
            "schema",
            format!("{:?} is not {LOOK_SCHEMA:?}", look.schema),
        );
    }
    let builtin = dpl_coldlight::theme_preset(&look.id).is_some();
    if !crate::is_id(&look.id) && !builtin {
        push(
            "id",
            format!("{:?} is neither a look id nor a built-in look", look.id),
        );
    }
    if look.rev == 0 {
        push("rev", "a saved look starts at revision 1".into());
    }
    if look.label.trim().is_empty() {
        push("label", "a look needs a name".into());
    }
    let legal = dpl_coldlight::PRESET_KNOBS
        .iter()
        .find(|(entity, _)| *entity == look.entity)
        .map(|(_, knobs)| *knobs);
    match legal {
        None => push("for", format!("unknown entity {:?}", look.entity)),
        Some(_) if !look.is_card() => push(
            "for",
            format!(
                "a {} look arrives with LK3; only card looks for now",
                look.entity
            ),
        ),
        Some(knobs) => {
            if !(4..=7).contains(&look.exposes.len()) {
                push(
                    "exposes",
                    format!("{} knobs; a look exposes 4-7", look.exposes.len()),
                );
            }
            if !look.exposes.iter().any(|knob| knob == "colour") {
                push("exposes", "colour is always one of the knobs".into());
            }
            for knob in &look.exposes {
                if !knobs.contains(&knob.as_str()) {
                    push("exposes", format!("a {} has no knob {knob:?}", look.entity));
                }
            }
        }
    }
    if !crate::tone_ok(&look.tone) {
        push("tone", format!("unknown tone {:?}", look.tone));
    }
    if !crate::font_ok(&look.font) {
        push("font", format!("unknown font {:?}", look.font));
    }
    crate::validate_background(&look.background, "background", &mut issues);
    issues
}

/// Does the look a slide names exist here? The half of the check
/// [`crate::validate`] cannot make without the library. Empty = clean.
pub fn validate_looks(slide: &Slide, looks: &Looks) -> Vec<Issue> {
    let mut issues = match &slide.preset {
        Some(id) if crate::card_preset(slide, looks).is_none() => vec![Issue {
            path: "preset".into(),
            message: format!("no card look {id:?} — the slide shows the show's style"),
        }],
        _ => Vec::new(),
    };
    // Each block's look (LK3): real, and for its own entity.
    for block in &slide.blocks {
        if let Some(id) = block.style().and_then(|style| style.preset) {
            if looks.line_look(block).is_none() {
                issues.push(Issue {
                    path: format!("blocks.{}.preset", block.id()),
                    message: format!(
                        "no {} look {id:?} — it shows the card's style",
                        crate::block_entity(block)
                    ),
                });
            }
        }
    }
    issues
}

/// Save the card as it shows now as a new look named `label`. The slide
/// then follows the new look, and its values the look now holds are given
/// back, so it shows no changes; what a look cannot hold (sizes, inks, a
/// line whose font differs from the rest) stays the slide's own. Returns
/// the look to save.
pub fn save_as_new_look(slide: &mut Slide, theme: &Theme, looks: &Looks, label: &str) -> Look {
    let shown = crate::resolve(slide, theme, looks);
    let current = crate::card_preset(slide, looks);
    // The font the card's own lines show — not lines with a look of their own.
    let fonts: Vec<&str> = shown
        .blocks
        .iter()
        .filter_map(Block::words)
        .filter(|words| words.preset.is_none())
        .filter_map(|words| words.font)
        .collect();
    let font = match fonts.first() {
        Some(first) if fonts.iter().all(|font| font == first) => (*first).to_owned(),
        _ => current.map_or_else(|| theme.font.clone(), |look| look.font.clone()),
    };
    let look = Look {
        schema: LOOK_SCHEMA.into(),
        id: crate::new_id(),
        rev: 1,
        label: label.trim().to_owned(),
        blurb: String::new(),
        entity: "card".into(),
        exposes: card_knobs().iter().map(|knob| (*knob).to_owned()).collect(),
        tone: shown
            .tone
            .clone()
            .unwrap_or_else(|| DEFAULT_LOOK_TONE.into()),
        font,
        background: shown
            .background
            .clone()
            .unwrap_or_else(|| theme.background.clone()),
        provenance: LookProvenance {
            forked_from: current
                .map(|look| look.id.clone())
                .or_else(|| slide.provenance.unlinked_from.clone()),
        },
    };
    slide.preset = Some(look.id.clone());
    slide.provenance.unlinked_from = None;
    crate::link::give_back_matching(slide, &look);
    look
}

/// The one font all of the slide's own line fonts agree on, when they do.
/// Lines following their own line look (LK3) are that look's business, not
/// the card's, so they are left out.
fn agreed_font(slide: &Slide) -> Option<String> {
    let own: Vec<&str> = slide
        .blocks
        .iter()
        .filter_map(Block::words)
        .filter(|words| words.preset.is_none())
        .filter_map(|words| words.font)
        .collect();
    match own.first() {
        Some(first) if own.iter().all(|font| font == first) => Some((*first).to_owned()),
        _ => None,
    }
}

/// Does the slide hold a change `look` could take as its new style — its
/// own background or colour, or one font all its own line fonts share?
pub fn holds_style_for(slide: &Slide, look: &Look) -> bool {
    slide
        .background
        .as_ref()
        .is_some_and(|background| *background != look.background)
        || slide.tone.as_ref().is_some_and(|tone| *tone != look.tone)
        || agreed_font(slide).is_some_and(|font| font != look.font)
}

/// "Make these the look's new style" (LK2b): the look takes the slide's own
/// background, colour and — when its lines agree — font, as its next
/// revision (for a built-in, the operator's version at revision 1). The
/// slide gives those values back, so it follows with no changes but what a
/// look cannot hold. Returns the look to save.
pub fn push_back(slide: &mut Slide, look: &Look) -> Look {
    let mut next = look.clone();
    next.rev = look.rev + 1;
    if let Some(background) = &slide.background {
        next.background = background.clone();
    }
    if let Some(tone) = &slide.tone {
        next.tone = tone.clone();
    }
    if let Some(font) = agreed_font(slide) {
        next.font = font;
    }
    crate::link::give_back_matching(slide, &next);
    next
}

/// "Back to the built-in" (LK2b): the operator's version steps aside to
/// `<look>/replaced/<rev>.json` — kept, never deleted — so the built-in
/// shows again. Returns where it went.
pub fn retire_look(content_root: &Path, id: &str) -> Result<PathBuf, DocError> {
    let content_root = &crate::writable(content_root)?;
    let dir = look_dir(content_root, id);
    let look = load_look(&dir)?;
    let replaced = dir.join("replaced");
    std::fs::create_dir_all(&replaced).map_err(DocError::Io)?;
    let mut dest = replaced.join(format!("{}.json", look.rev));
    let mut n = 1;
    while dest.exists() {
        dest = replaced.join(format!("{}-{n}.json", look.rev));
        n += 1;
    }
    std::fs::rename(dir.join(LOOK_FILE), &dest).map_err(DocError::Io)?;
    Ok(dest)
}

/// Every slide of the operator's that follows the look `id`: the
/// library's (newest first), then every episode's. The read-only seeds
/// are not counted — a change to a look never edits them.
pub fn slides_following(content_root: &Path, id: &str) -> Vec<PathBuf> {
    let mut dirs = crate::library_slides(content_root);
    for entry in crate::all_episodes(content_root) {
        if crate::is_seed_dir(&entry.dir) {
            continue;
        }
        if let Ok(episode) = crate::load_episode(&entry.dir) {
            dirs.extend(
                crate::episode_slides(&entry.dir, &episode)
                    .into_iter()
                    .map(|slide| slide.dir),
            );
        }
    }
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|dir| {
        seen.insert(dir.clone())
            && crate::load(dir).is_ok_and(|slide| slide.preset.as_deref() == Some(id))
    });
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-look-{tag}-{}", crate::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn card() -> Slide {
        let mut slide = Slide::new("title@1");
        slide.blocks = vec![
            Block::text("title", "Hello"),
            Block::text("subtitle", "there"),
        ];
        slide
    }

    #[test]
    fn every_builtin_is_a_valid_look_once_saved() {
        for look in Looks::builtin_only().builtins_shown() {
            let mut saved = look.clone();
            saved.rev = 1;
            assert_eq!(validate_look(&saved), Vec::new(), "{}", look.id);
        }
    }

    /// Every look this build writes passes `schemas/look.v1.schema.json`:
    /// each built-in saved as your version, and a new look saved from a card.
    #[test]
    fn looks_pass_the_look_schema() {
        let schema: Value = serde_json::from_str(
            &std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/look.v1.schema.json"),
            )
            .expect("the look schema"),
        )
        .expect("the schema is JSON");
        let validator = jsonschema::validator_for(&schema).expect("the schema compiles");
        let looks = Looks::builtin_only();
        let mut docs: Vec<Look> = looks
            .builtins_shown()
            .map(|look| Look {
                rev: 1,
                ..look.clone()
            })
            .collect();
        let mut slide = card();
        slide.preset = Some("neon-intro".into());
        docs.push(save_as_new_look(
            &mut slide,
            &Theme::fallback(),
            &looks,
            "Mine",
        ));
        for look in docs {
            let value = serde_json::to_value(&look).unwrap();
            let errors: Vec<String> = validator
                .iter_errors(&value)
                .map(|e| format!("{} at {}", e, e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{}: {errors:?}", look.id);
        }
    }

    #[test]
    fn a_saved_look_round_trips_and_a_broken_one_is_named() {
        let root = scratch("roundtrip");
        let mut slide = card();
        slide.preset = Some("neon-intro".into());
        slide.tone = Some("coral".into());
        let looks = Looks::builtin_only();
        let look = save_as_new_look(&mut slide, &Theme::fallback(), &looks, "  Warm intro ");
        assert_eq!(look.label, "Warm intro");
        assert_eq!(look.tone, "coral");
        assert_eq!(look.provenance.forked_from.as_deref(), Some("neon-intro"));
        save_look(&look_dir(&root, &look.id), &look).unwrap();
        assert_eq!(load_look(&look_dir(&root, &look.id)).unwrap(), look);

        // A broken one beside it is skipped and named, never a crash.
        let mut broken = look.clone();
        broken.id = crate::new_id();
        broken.font = "no-such-font".into();
        save_look(&look_dir(&root, &broken.id), &broken).unwrap();
        let loaded = Looks::load(&root);
        assert_eq!(loaded.get(&look.id), Some(&look));
        assert!(loaded.get(&broken.id).is_none());
        assert!(loaded
            .issues()
            .iter()
            .any(|issue| issue.message.contains("no-such-font")));
        assert_eq!(loaded.own().count(), 1);
    }

    #[test]
    fn saving_a_new_look_leaves_the_slide_following_it_with_no_changes() {
        let theme = Theme::fallback();
        let looks = Looks::builtin_only();
        let mut slide = card();
        slide.preset = Some("neon-intro".into());
        slide.background = Some(Background::Mood {
            mood: "orchid-haze".into(),
        });
        let before = crate::resolve(&slide, &theme, &looks);
        let look = save_as_new_look(&mut slide, &theme, &looks, "Orchid");
        let mut with = looks.clone();
        with.put(look.clone());
        assert_eq!(slide.preset.as_deref(), Some(look.id.as_str()));
        assert_eq!(link::changes(&slide), Vec::new());
        // Nothing changes on screen.
        let after = crate::resolve(&slide, &theme, &with);
        assert_eq!(after.background, before.background);
        assert_eq!(after.blocks, before.blocks);
        assert_eq!(validate_looks(&slide, &with), Vec::new());
        assert_eq!(crate::validate(&slide), Vec::new());
    }

    #[test]
    fn what_a_look_cannot_hold_stays_the_slides_own() {
        let theme = Theme::fallback();
        let looks = Looks::builtin_only();
        let mut slide = card();
        slide.blocks[0] = Block::Text {
            id: "title".into(),
            preset: None,
            tone: None,
            text: "Hello".into(),
            font: Some("vt323".into()),
            size: Some("h1".into()),
            ink: None,
            enter: None,
            unlinked_from: None,
        };
        let look = save_as_new_look(&mut slide, &theme, &looks, "Mixed");
        // The lines disagree, so the look keeps the show's font and the
        // title keeps its own font and size.
        assert_eq!(look.font, theme.font);
        let kept = link::changes(&slide);
        assert!(kept.contains(&link::Knob::Line {
            id: "title".into(),
            field: link::LineField::Font
        }));
        assert!(kept.contains(&link::Knob::Line {
            id: "title".into(),
            field: link::LineField::Size
        }));
    }

    #[test]
    fn your_version_replaces_a_builtin_and_the_original_stays_findable() {
        let mut looks = Looks::builtin_only();
        let mut mine = looks.get("neon-intro").unwrap().clone();
        mine.rev = 1;
        mine.tone = "lime".into();
        looks.put(mine.clone());
        assert_eq!(looks.get("neon-intro"), Some(&mine));
        assert!(looks.replaces_builtin("neon-intro"));
        assert_eq!(looks.builtin("neon-intro").unwrap().tone, "amber");
        assert_eq!(looks.own().count(), 0, "a version is not a new look");
        assert_eq!(
            looks.builtins_shown().find(|look| look.id == "neon-intro"),
            Some(&mine)
        );
    }

    #[test]
    fn a_missing_look_is_named_and_shows_the_shows_style() {
        let looks = Looks::builtin_only();
        let mut slide = card();
        slide.preset = Some(crate::new_id());
        assert_eq!(crate::validate(&slide), Vec::new(), "the shape is fine");
        assert_eq!(validate_looks(&slide, &looks).len(), 1);
        let theme = Theme::fallback();
        let shown = crate::resolve(&slide, &theme, &looks);
        assert_eq!(shown.background, Some(theme.background));
        assert!(matches!(
            link::link_state(&slide, &looks),
            link::LinkState::Missing { .. }
        ));
    }

    /// Pushing a change back to a built-in makes the operator's version at
    /// revision 1; the slide then follows it with no changes.
    #[test]
    fn pushing_back_to_a_builtin_makes_your_version() {
        let looks = Looks::builtin_only();
        let neon = looks.get("neon-intro").unwrap().clone();
        let mut slide = card();
        slide.preset = Some(neon.id.clone());
        assert!(!holds_style_for(&slide, &neon), "nothing to push yet");
        slide.background = Some(Background::Mood {
            mood: "orchid-haze".into(),
        });
        assert!(holds_style_for(&slide, &neon));
        let mine = push_back(&mut slide, &neon);
        assert_eq!((mine.id.as_str(), mine.rev), ("neon-intro", 1));
        assert_eq!(
            mine.background,
            Background::Mood {
                mood: "orchid-haze".into()
            }
        );
        assert_eq!(
            (mine.tone.as_str(), mine.font.as_str()),
            (neon.tone.as_str(), neon.font.as_str())
        );
        assert_eq!(link::changes(&slide), Vec::new());
        assert_eq!(validate_look(&mine), Vec::new());
    }

    /// A look takes one font only when the slide's lines agree on it; what
    /// it cannot hold stays the slide's.
    #[test]
    fn pushing_back_leaves_what_a_look_cannot_hold() {
        let looks = Looks::builtin_only();
        let neon = looks.get("neon-intro").unwrap().clone();
        let mut slide = card();
        slide.preset = Some(neon.id.clone());
        slide.blocks[0] = Block::Text {
            id: "title".into(),
            preset: None,
            tone: None,
            text: "Hello".into(),
            font: Some("vt323".into()),
            size: Some("h1".into()),
            ink: None,
            enter: None,
            unlinked_from: None,
        };
        slide.tone = Some("lime".into());
        let next = push_back(&mut slide, &neon);
        assert_eq!(next.font, "vt323", "the only own font is agreed");
        assert_eq!(next.tone, "lime");
        assert_eq!(
            link::changes(&slide),
            vec![link::Knob::Line {
                id: "title".into(),
                field: link::LineField::Size
            }]
        );
    }

    /// Back to the built-in: your version steps aside (kept on disk) and
    /// the built-in shows again; only the operator's own slides follow.
    #[test]
    fn your_version_steps_aside_and_only_your_slides_are_counted() {
        let root = scratch("retire");
        let looks = Looks::builtin_only();
        let mut mine = looks.get("neon-intro").unwrap().clone();
        mine.rev = 1;
        mine.tone = "lime".into();
        save_look(&look_dir(&root, &mine.id), &mine).unwrap();
        assert!(Looks::load(&root).replaces_builtin("neon-intro"));
        let kept = retire_look(&root, "neon-intro").unwrap();
        assert!(kept.is_file(), "your version is kept, not deleted");
        let after = Looks::load(&root);
        assert!(!after.is_yours("neon-intro"));
        assert_eq!(after.get("neon-intro").unwrap().tone, "amber");

        // Two library slides follow it; a third does not.
        for preset in [Some("neon-intro"), Some("neon-intro"), None] {
            let mut slide = card();
            slide.preset = preset.map(str::to_owned);
            crate::save(&crate::library_slide_dir(&root, &slide.id), &slide).unwrap();
        }
        assert_eq!(slides_following(&root, "neon-intro").len(), 2);
        assert!(slides_following(&root, "no-such-look").is_empty());
    }
}
