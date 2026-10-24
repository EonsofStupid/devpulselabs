//! Linked looks (LK1, R286): a slide FOLLOWS its look, any single value can
//! be made yours (and given back with one tap), and the whole slide can stop
//! following — reversibly. The state is DERIVED from the slide's own sparse
//! values, never stored, so it can never drift (docs/plans/linked-looks-lk1.md).
//! The look comes from [`Looks`], built-in or the operator's own (LK2).
//!
//! Plain words in every surface: "follows", "yours", "the look's", "own
//! style" — never "override", "instance" or "detach".

use crate::{card_preset, resolve, Block, Look, Looks, Slide, Theme};

/// Where a value on screen comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The slide's own value — a change from its look (or its own style).
    Yours,
    /// The look the slide follows (its id).
    Look(String),
    /// The show's theme (or the renderer's default).
    Show,
}

/// One thing on a slide that can follow, or be yours.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Knob {
    /// The card's background (its colours, movement and moving pattern).
    Background,
    /// The card's colour.
    Colour,
    /// How the words arrive.
    Enter,
    /// One line's font, size, ink or colour.
    Line { id: String, field: LineField },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LineField {
    Font,
    Size,
    Ink,
    Colour,
}

impl Knob {
    /// The knob in plain words ("the background", "the title's font").
    pub fn label(&self) -> String {
        match self {
            Knob::Background => "the background".into(),
            Knob::Colour => "the colour".into(),
            Knob::Enter => "how the words arrive".into(),
            Knob::Line { id, field } => {
                let what = match field {
                    LineField::Font => "font",
                    LineField::Size => "size",
                    LineField::Ink => "ink",
                    LineField::Colour => "colour",
                };
                format!("the {id}'s {what}")
            }
        }
    }
}

/// The slide's link to its look, worked out from its own values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkState {
    /// Follows a look; `changes` of its values are yours.
    Follows { look: String, changes: usize },
    /// Names a look that cannot be found: it shows the show's style.
    Missing { look: String },
    /// Stopped following (`was` names the look it followed).
    OwnStyle { was: Option<String> },
    /// No look: the show's style, with `changes` of your own.
    Show { changes: usize },
}

impl LinkState {
    /// What the look chip says.
    pub fn chip(&self, look_label: impl Fn(&str) -> String) -> String {
        let changes = |n: usize| match n {
            0 => String::new(),
            1 => " · 1 change".into(),
            n => format!(" · {n} changes"),
        };
        match self {
            LinkState::Follows { look, changes: n } => {
                format!("Look: {}{}", look_label(look), changes(*n))
            }
            LinkState::Missing { .. } => "Look missing — the show's style".into(),
            LinkState::OwnStyle { was: Some(was) } => {
                format!("Own style (was {})", look_label(was))
            }
            LinkState::OwnStyle { was: None } => "Own style".into(),
            LinkState::Show { changes: n } => format!("The show's style{}", changes(*n)),
        }
    }
}

fn line_value(block: &Block, field: LineField) -> Option<&str> {
    let words = block.words();
    match field {
        LineField::Font => words.and_then(|w| w.font),
        LineField::Size => words.and_then(|w| w.size),
        LineField::Ink => words.and_then(|w| w.ink),
        LineField::Colour => match block {
            Block::Text { tone, .. } | Block::Media { tone, .. } | Block::Quote { tone, .. } => {
                tone.as_deref()
            }
            Block::Data { .. } => None,
        },
    }
}

/// Where `knob`'s value comes from on this slide.
pub fn source(slide: &Slide, looks: &Looks, knob: &Knob) -> Source {
    let look = card_preset(slide, looks).map(|look| look.id.clone());
    let followed = |yours: bool, look_gives_it: bool| match (yours, &look) {
        (true, _) => Source::Yours,
        (false, Some(id)) if look_gives_it => Source::Look(id.clone()),
        _ => Source::Show,
    };
    match knob {
        Knob::Background => followed(slide.background.is_some(), true),
        Knob::Colour => followed(slide.tone.is_some(), true),
        Knob::Enter => followed(slide.enter.is_some(), false),
        Knob::Line { id, field } => {
            let yours = slide
                .block(id)
                .and_then(|block| line_value(block, *field))
                .is_some();
            followed(yours, *field == LineField::Font)
        }
    }
}

/// Every value on the slide that is yours, in a stable order.
pub fn changes(slide: &Slide) -> Vec<Knob> {
    let mut out = Vec::new();
    for (knob, yours) in [
        (Knob::Background, slide.background.is_some()),
        (Knob::Colour, slide.tone.is_some()),
        (Knob::Enter, slide.enter.is_some()),
    ] {
        if yours {
            out.push(knob);
        }
    }
    for block in &slide.blocks {
        for field in [
            LineField::Font,
            LineField::Size,
            LineField::Ink,
            LineField::Colour,
        ] {
            if line_value(block, field).is_some() {
                out.push(Knob::Line {
                    id: block.id().to_owned(),
                    field,
                });
            }
        }
    }
    out
}

/// The slide's link, worked out (never stored).
pub fn link_state(slide: &Slide, looks: &Looks) -> LinkState {
    match (card_preset(slide, looks), &slide.preset) {
        (Some(look), _) => LinkState::Follows {
            look: look.id.clone(),
            changes: changes(slide).len(),
        },
        (None, Some(missing)) => LinkState::Missing {
            look: missing.clone(),
        },
        (None, None) if slide.provenance.unlinked_from.is_some() => LinkState::OwnStyle {
            was: slide.provenance.unlinked_from.clone(),
        },
        (None, None) => LinkState::Show {
            changes: changes(slide).len(),
        },
    }
}

/// Give one value back to the look (or the show).
pub fn reset(slide: &mut Slide, knob: &Knob) {
    match knob {
        Knob::Background => slide.background = None,
        Knob::Colour => slide.tone = None,
        Knob::Enter => slide.enter = None,
        Knob::Line { id, field } => {
            if let Some(block) = slide.blocks.iter_mut().find(|b| b.id() == id) {
                clear_line(block, *field);
            }
        }
    }
}

fn clear_line(block: &mut Block, field: LineField) {
    if field == LineField::Colour {
        if let Block::Text { tone, .. } | Block::Media { tone, .. } | Block::Quote { tone, .. } =
            block
        {
            *tone = None;
        }
        return;
    }
    if let Some(words) = block.words_mut() {
        match field {
            LineField::Font => *words.font = None,
            LineField::Size => *words.size = None,
            LineField::Ink => *words.ink = None,
            LineField::Colour => {}
        }
    }
}

/// Give every value back — "Undo all my changes".
pub fn reset_all(slide: &mut Slide) {
    for knob in changes(slide) {
        reset(slide, &knob);
    }
}

/// Stop following the look: its values are written onto the slide as yours,
/// so NOTHING changes on screen, then the reference is dropped and the look
/// remembered (`provenance.unlinked_from`) so following again is one tap.
/// Only the look's own tier is copied — the show's values stay inherited.
pub fn stop_following(slide: &mut Slide, theme: &Theme, looks: &Looks) {
    let Some(look) = card_preset(slide, looks) else {
        return;
    };
    let resolved = resolve(slide, theme, looks);
    if slide.background.is_none() {
        slide.background = resolved.background;
    }
    if slide.tone.is_none() {
        slide.tone = Some(look.tone.clone());
    }
    for block in &mut slide.blocks {
        if let Some(words) = block.words_mut() {
            if words.font.is_none() {
                *words.font = Some(look.font.clone());
            }
        }
    }
    slide.provenance.unlinked_from = Some(look.id.clone());
    slide.preset = None;
}

/// Follow the look again. "Keep my changes" keeps every value that differs
/// from the look (values that only repeat the look are given back, so they
/// do not count as changes); "start fresh" gives back everything the look
/// governs, as picking the look does.
pub fn follow_again(slide: &mut Slide, looks: &Looks, keep_my_changes: bool) {
    let Some(id) = slide.provenance.unlinked_from.clone() else {
        return;
    };
    slide.preset = Some(id);
    slide.provenance.unlinked_from = None;
    let Some(look) = card_preset(slide, looks) else {
        return;
    };
    if keep_my_changes {
        give_back_matching(slide, look);
    } else {
        slide.background = None;
        slide.tone = None;
        for block in &mut slide.blocks {
            if let Some(words) = block.words_mut() {
                *words.font = None;
            }
        }
    }
}

/// Give back every value that only repeats what `look` gives — so a value
/// counts as a change only when it really differs from the look.
pub(crate) fn give_back_matching(slide: &mut Slide, look: &Look) {
    if slide.background.as_ref() == Some(&look.background) {
        slide.background = None;
    }
    if slide.tone.as_deref() == Some(look.tone.as_str()) {
        slide.tone = None;
    }
    for block in &mut slide.blocks {
        if let Some(words) = block.words_mut() {
            if words.font.as_deref() == Some(look.font.as_str()) {
                *words.font = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looked() -> Slide {
        let mut slide = Slide::new("title@1");
        slide.preset = Some(dpl_coldlight::THEME_PRESETS[0].id.into());
        slide.blocks = vec![
            Block::text("title", "Hello"),
            Block::text("subtitle", "there"),
        ];
        slide
    }

    #[test]
    fn sources_name_the_look_until_a_value_is_yours() {
        let looks = Looks::builtin_only();
        let mut slide = looked();
        let look = dpl_coldlight::THEME_PRESETS[0].id.to_owned();
        assert_eq!(
            source(&slide, &looks, &Knob::Background),
            Source::Look(look.clone())
        );
        let font = Knob::Line {
            id: "title".into(),
            field: LineField::Font,
        };
        assert_eq!(source(&slide, &looks, &font), Source::Look(look.clone()));
        let size = Knob::Line {
            id: "title".into(),
            field: LineField::Size,
        };
        assert_eq!(
            source(&slide, &looks, &size),
            Source::Show,
            "a look gives no size"
        );
        assert_eq!(
            link_state(&slide, &looks),
            LinkState::Follows {
                look: look.clone(),
                changes: 0
            }
        );

        slide.tone = Some("coral".into());
        assert_eq!(source(&slide, &looks, &Knob::Colour), Source::Yours);
        assert_eq!(
            link_state(&slide, &looks),
            LinkState::Follows { look, changes: 1 }
        );
        assert_eq!(
            link_state(&slide, &looks).chip(|id| id.to_uppercase()),
            format!(
                "Look: {} · 1 change",
                dpl_coldlight::THEME_PRESETS[0].id.to_uppercase()
            )
        );
        reset(&mut slide, &Knob::Colour);
        assert_eq!(
            source(&slide, &looks, &Knob::Colour),
            Source::Look(dpl_coldlight::THEME_PRESETS[0].id.into())
        );
    }

    #[test]
    fn stopping_changes_nothing_on_screen_and_following_again_round_trips() {
        let theme = Theme::fallback();
        let looks = Looks::builtin_only();
        let mut slide = looked();
        slide.tone = Some("coral".into());
        let before = resolve(&slide, &theme, &looks);

        stop_following(&mut slide, &theme, &looks);
        assert_eq!(slide.preset, None);
        assert_eq!(
            link_state(&slide, &looks),
            LinkState::OwnStyle {
                was: Some(dpl_coldlight::THEME_PRESETS[0].id.into())
            }
        );
        let after = resolve(&slide, &theme, &looks);
        assert_eq!(after.background, before.background);
        assert_eq!(after.tone, before.tone);
        assert_eq!(after.blocks, before.blocks, "nothing changes on screen");
        assert_eq!(crate::validate(&slide), Vec::new());

        // Follow again, keeping my changes: only the real one remains.
        let mut kept = slide.clone();
        follow_again(&mut kept, &looks, true);
        assert_eq!(changes(&kept), vec![Knob::Colour]);
        assert_eq!(resolve(&kept, &theme, &looks).blocks, before.blocks);

        // Or start fresh: everything the look governs follows it again.
        follow_again(&mut slide, &looks, false);
        assert_eq!(changes(&slide), Vec::new());
    }

    #[test]
    fn undo_all_my_changes_gives_everything_back() {
        let looks = Looks::builtin_only();
        let mut slide = looked();
        slide.background = Some(crate::Background::None);
        slide.enter = Some("none".into());
        slide.blocks[0] = Block::Text {
            id: "title".into(),
            preset: None,
            tone: Some("lime".into()),
            text: "Hello".into(),
            font: Some("vt323".into()),
            size: Some("h1".into()),
            ink: Some("tone".into()),
        };
        assert_eq!(changes(&slide).len(), 6);
        reset_all(&mut slide);
        assert_eq!(changes(&slide), Vec::new());
        assert!(matches!(
            link_state(&slide, &looks),
            LinkState::Follows { changes: 0, .. }
        ));
    }
}
