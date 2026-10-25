//! The tap-to-edit editor — a VIEW over a slide document (W4).
//!
//! The operator's law: layman-friendly, touch-first, the stack invisible.
//! You tap the real slide — a text line, or its background — and a tray of
//! the THING ITSELF slides up (fonts drawn as fonts, moods as living
//! gradient cards). Tap one, it applies on the slide instantly and writes
//! back the colocated `slide.json`. No save button. One big Undo. No JSON,
//! no ids, no "token" — ever, on screen.
//!
//! W4 is select-and-restyle-existing (the operator's literal spec). Drag
//! reorder / add / delete blocks are W4b (egui_dnd, RS2).

use dpl_coldlight::{self as cl};
use dpl_slides::document;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Ui};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::{Path, PathBuf};

/// What the operator has tapped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Selection {
    /// Nothing selected — the LOOK tray offers one-tap presets.
    #[default]
    None,
    /// The slide background — the Colours/Motion trays.
    Background,
    /// A text line, by block id — the Font/Size/Ink/Text trays.
    Block(String),
}

/// One step Undo takes back: a slide revision, or a slide revision plus
/// the look a change spread through (LK2b).
enum Step {
    Slide(dpl_doc::Slide),
    Look {
        slide: dpl_doc::Slide,
        look: Box<LookBefore>,
    },
}

/// What a look was before a change spread through it.
struct LookBefore {
    id: String,
    /// The operator's revision before the change; `None` when the built-in
    /// showed (Undo then steps the operator's version aside again).
    was: Option<dpl_doc::Look>,
}

/// A change about to spread through a look, waiting for the operator's OK.
struct Spread {
    kind: SpreadKind,
    /// How many other slides follow the look.
    others: usize,
    /// One slide that follows the look, shown before and after.
    preview: dpl_doc::Slide,
    before: dpl_doc::Styling,
    after: dpl_doc::Styling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpreadKind {
    /// "Make these the look's new style".
    PushBack,
    /// "Back to the built-in".
    BackToBuiltin,
}

/// The editor over ONE slide document folder.
pub struct Editor {
    dir: PathBuf,
    slide: dpl_doc::Slide,
    /// The tiers above the slide: the show's theme (W3b) — auto-detected
    /// from the parent episode folder, else the fallback — and the looks,
    /// built-in plus the operator's own (LK2). Trays show RESOLVED values.
    styling: dpl_doc::Styling,
    /// Where the library is: own looks load from and save to it.
    content_root: PathBuf,
    clock: dpl_slides::motion::Clock,
    selection: Selection,
    /// Snapshots for Undo — the doc is a small value, snapshot per edit;
    /// a change that spread through a look carries the look's too (LK2b).
    undo: Vec<Step>,
    /// Last save's outcome, shown as an honest footer crumb.
    saved: Result<(), String>,
    /// A look being TRIED (LK1): the stage shows it, nothing is saved until
    /// Keep.
    trying: Option<String>,
    /// Is the look chip's menu open?
    link_menu: bool,
    /// Every knob, not just the look's (LK1 "More controls").
    more_controls: bool,
    /// The edit burst in progress — typing settles into ONE revision.
    burst: Option<(String, std::time::Instant)>,
    /// The yours-dot whose comparison is open ("Yours: … · The look's: …").
    compare: Option<dpl_doc::link::Knob>,
    /// "Save as a new look…" is open, with the name typed so far (LK2).
    naming: Option<String>,
    /// What the last look action did, said once in the chip's menu.
    notice: Option<String>,
    /// A change waiting to spread through a look, with its count and
    /// before/after (LK2b).
    spread: Option<Spread>,
    /// "Open the look": background, colour and font changes go to the
    /// look itself; the number is how many slides show it (LK2b).
    editing_look: Option<usize>,
    /// A line look being TRIED on one line (LK3): the line's id and the
    /// look (`None` = the card's style). Nothing is saved until Keep.
    trying_line: Option<(String, Option<String>)>,
    /// Is the line chip's menu open?
    line_menu: bool,
    /// The card's teleprompter script (RL), when it has one — its beats
    /// follow the card's taps; written as you type.
    script: Option<dpl_doc::script::Script>,
    /// The last script write's outcome.
    script_saved: Result<(), String>,
    /// The tap the On tap panel previews (0 = before any tap).
    preview: usize,
    /// The preview's own clock.
    preview_clock: dpl_slides::motion::Clock,
}

/// A label in a fixed-width column, left-aligned and cut short with an
/// ellipsis — the On tap panel's line and point names, so the controls
/// beside them line up.
fn column_label(ui: &mut Ui, text: RichText, width: f32, height: f32) -> egui::Response {
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.add(egui::Label::new(text).truncate())
        },
    )
    .inner
}

/// A line as the On tap panel names it: its first words, or what it is.
fn line_name(block: &dpl_doc::Block) -> String {
    match block.words() {
        Some(words) if !words.text.trim().is_empty() => {
            let text = words.text.trim();
            match text.char_indices().nth(22) {
                Some((cut, _)) => format!("{}…", &text[..cut]),
                None => text.to_owned(),
            }
        }
        Some(_) => "An empty line".to_owned(),
        None => match block {
            dpl_doc::Block::Media { .. } => "The picture".to_owned(),
            _ => format!("The {}", block.id()),
        },
    }
}

/// A text or quote line's tray view: every value RESOLVED through the
/// cascade (its own, its line look's, the card look's, the show's).
struct TextFields {
    text: String,
    font: String,
    size: String,
    ink: String,
    /// The line's colour, when it has one (its own or its look's).
    tone: Option<String>,
    /// How the line arrives: `rise` or `none`.
    enter: String,
    /// `text` or `quote` — which looks the line can take.
    entity: &'static str,
}

const UNDO_CAP: usize = 50;
/// The shortest the tray gets while there is room — enough for its title,
/// one line, and a row of whole tiles.
const TRAY_MIN: f32 = 140.0;
/// A pause longer than this ends a typing burst — the next keystroke is a
/// new revision and a new undo step.
const BURST_PAUSE: std::time::Duration = std::time::Duration::from_millis(1500);
/// What an unsized line paints at — the renderer's own fallback.
const DEFAULT_SIZE: &str = "body";
/// The slot a card's picture fills in every component that has one.
const MEDIA_SLOT: &str = "media";
/// The component a card becomes when it holds a quote.
const QUOTE_COMPONENT: &str = "quote@1";
/// The quote component's slots the quote tab writes, in reading order.
pub const QUOTE_SLOTS: [&str; 3] = [QUOTE_LINES[0].0, QUOTE_LINES[1].0, QUOTE_LINES[2].0];

/// The quote tab's three lines, in reading order: the slot, and the size
/// and ink a NEW line starts with (ordinary overrides afterwards).
const QUOTE_LINES: [(&str, &str, &str); 3] = [
    ("quote", "h2", "strong"),
    ("name", "body-lg", "tone"),
    ("role", "body-sm", "subtle"),
];
const INKS: [&str; 5] = ["strong", "base", "muted", "subtle", "tone"];
const AMBIENCES: [&str; 3] = ["pulse", "drift", "rings"];
const SIZES: [&str; 10] = [
    "eyebrow",
    "body-sm",
    "body",
    "body-lg",
    "h4",
    "h3",
    "h2",
    "h1",
    "display",
    "display-xl",
];

impl Editor {
    /// Open the editor on a slide folder (the colocated slide.json). Its
    /// looks are the built-in ones plus those in `content_root`'s library,
    /// where a look saved here lands too (LK2).
    pub fn open(dir: &Path, content_root: &Path) -> Result<Self, String> {
        let slide = dpl_doc::load(dir).map_err(|e| e.to_string())?;
        Ok(Self {
            dir: dir.to_path_buf(),
            slide,
            styling: dpl_doc::Styling {
                theme: dpl_doc::theme_for_slide(dir),
                looks: dpl_doc::Looks::load(content_root),
            },
            content_root: content_root.to_path_buf(),
            clock: dpl_slides::motion::Clock::default(),
            selection: Selection::None,
            trying: None,
            link_menu: false,
            more_controls: false,
            burst: None,
            compare: None,
            naming: None,
            notice: None,
            spread: None,
            editing_look: None,
            trying_line: None,
            line_menu: false,
            script: dpl_doc::script::load_script(dir).ok().flatten(),
            script_saved: Ok(()),
            preview: 0,
            preview_clock: dpl_slides::motion::Clock::default(),
            undo: Vec::new(),
            saved: Ok(()),
        })
    }

    pub fn slide(&self) -> &dpl_doc::Slide {
        &self.slide
    }

    /// Paint against `theme` — the show this card is edited from (LS3b). A
    /// card in the library has no show of its own, so whoever opens it from
    /// an episode hands the episode's theme in, and the card looks here as
    /// it does there.
    pub fn with_theme(mut self, theme: dpl_doc::Theme) -> Self {
        self.styling.theme = theme;
        self
    }

    /// Set what sits behind the words, or `None` to inherit the show's theme.
    ///
    /// The authoring surface's Background tab is a VIEW over this document,
    /// so it asks the editor to change the slide rather than writing the file
    /// itself. Going through `mutate` means a tapped colour lands on the undo
    /// stack and saves like every other edit — one mutation path, not two.
    pub fn set_background(&mut self, background: Option<dpl_doc::Background>) {
        self.mutate(|slide| slide.background = background);
    }

    /// The tiers this slide resolves against — its looks and the show's
    /// theme. A preview drawn beside the editor has to cascade the same way
    /// the card does, or it shows something the deck never will.
    pub fn styling(&self) -> &dpl_doc::Styling {
        &self.styling
    }

    /// What the show's theme says the entrance is — what "follow the show"
    /// resolves to, so a picker can name it instead of showing an invisible
    /// default.
    pub fn look_enter(&self) -> Option<String> {
        self.styling.theme.enter.clone()
    }

    /// The three lines of a quote — the quotation, who said it, where —
    /// empty where the card has no such block. Read for the tab's fields.
    pub fn quote(&self) -> [String; 3] {
        let mut out = [String::new(), String::new(), String::new()];
        for (slot, (id, _, _)) in out.iter_mut().zip(QUOTE_LINES) {
            if let Some(words) = self.slide.block(id).and_then(dpl_doc::Block::words) {
                *slot = words.text.to_owned();
            }
        }
        out
    }

    /// Write the three lines of a quote: the quotation as a real
    /// [`dpl_doc::Block::Quote`] (ruling #3), who said it and where in the
    /// quote component's own `name` and `role` slots.
    ///
    /// - a line with text UPDATES the block with that id, or appends it
    ///   (never a second block with the same id);
    /// - a BLANK line removes its block, because a document carrying an empty
    ///   text block is a rendering artefact, not an empty field;
    /// - the first non-blank line makes the card fill the `quote` component.
    pub fn set_quote(&mut self, lines: [String; 3]) {
        self.mutate(|slide| {
            for (line, (id, size, ink)) in lines.iter().zip(QUOTE_LINES) {
                let words = line.trim();
                let at = slide.blocks.iter().position(|b| b.id() == id);
                match (words.is_empty(), at) {
                    // Blank and present — the block goes.
                    (true, Some(i)) => {
                        slide.blocks.remove(i);
                    }
                    (true, None) => {}
                    // Present — update in place, keeping whatever restyling
                    // the operator has since done to it.
                    (false, Some(i)) => {
                        if let Some(block) = slide.blocks[i].words_mut() {
                            *block.text = words.to_string();
                        }
                    }
                    // New — append with the quote's default styling.
                    (false, None) => {
                        let text = words.to_string();
                        let size = Some(size.to_string());
                        let ink = Some(ink.to_string());
                        slide.blocks.push(if id == "quote" {
                            dpl_doc::Block::Quote {
                                id: id.to_string(),
                                preset: None,
                                tone: None,
                                text,
                                font: None,
                                size,
                                ink,
                                enter: None,
                                unlinked_from: None,
                            }
                        } else {
                            dpl_doc::Block::Text {
                                id: id.to_string(),
                                preset: None,
                                tone: None,
                                text,
                                font: None,
                                size,
                                ink,
                                enter: None,
                                unlinked_from: None,
                            }
                        });
                    }
                }
            }
            if lines.iter().any(|l| !l.trim().is_empty()) {
                slide.component = QUOTE_COMPONENT.to_string();
            }
        });
    }

    /// The card's picture, if it has one: `(src, show, fit)`.
    pub fn media(&self) -> Option<(String, String, String)> {
        self.slide.blocks.iter().find_map(|b| match b {
            dpl_doc::Block::Media { show, src, fit, .. } => Some((
                src.clone(),
                show.clone(),
                fit.clone().unwrap_or_else(|| "cover".to_string()),
            )),
            _ => None,
        })
    }

    /// Put a picture on the card, or take it off with `None`.
    ///
    /// `src` must be content-relative — the Picture tab passes through a path
    /// the artifact index already holds, so the content jail is satisfied by
    /// construction rather than by a check here. `show` comes from the index's
    /// own `kind`, never sniffed from the extension: the schema authors that
    /// discriminator so the doctor can name a mismatch.
    ///
    /// One picture per card. A second call replaces rather than appends,
    /// because `Block::Media` is a backdrop and two would fight.
    pub fn set_media(&mut self, media: Option<(String, String, String)>) {
        self.mutate(|slide| {
            let Some((src, show, fit)) = media else {
                slide
                    .blocks
                    .retain(|b| !matches!(b, dpl_doc::Block::Media { .. }));
                return;
            };
            // The schema's default is cover; write it only when it is the
            // other one, so a document stays sparse.
            let fit = (fit != "cover").then_some(fit);
            let existing = slide
                .blocks
                .iter_mut()
                .find(|b| matches!(b, dpl_doc::Block::Media { .. }));
            match existing {
                // A different picture keeps the picture's look, frame and
                // entrance (LK3b) — only the file and its sitting change.
                Some(dpl_doc::Block::Media {
                    src: at,
                    show: kind,
                    fit: sits,
                    poster,
                    ..
                }) => {
                    *at = src;
                    *kind = show;
                    *sits = fit;
                    *poster = None;
                }
                _ => slide.blocks.insert(
                    0,
                    dpl_doc::Block::Media {
                        id: MEDIA_SLOT.to_string(),
                        preset: None,
                        tone: None,
                        show,
                        src,
                        fit,
                        poster: None,
                        enter: None,
                        unlinked_from: None,
                    },
                ),
            }
        });
    }

    /// Set how the words arrive, or `None` to follow the show's theme.
    ///
    /// Restarts the clock so the entrance actually plays: a movement picker
    /// whose preview does not move is a contradiction.
    pub fn set_enter(&mut self, enter: Option<String>) {
        self.mutate(|slide| slide.enter = enter);
        self.clock.restart();
    }

    /// Apply an edit: snapshot for undo, mutate, then write back the
    /// colocated file immediately (no save ceremony — the file is truth).
    fn mutate(&mut self, f: impl FnOnce(&mut dpl_doc::Slide)) {
        self.burst = None;
        self.mutate_in(f);
    }

    fn mutate_in(&mut self, f: impl FnOnce(&mut dpl_doc::Slide)) {
        let before = self.slide.clone();
        f(&mut self.slide);
        if self.slide == before {
            return;
        }
        // Editing the look (LK2b): what the look can hold goes to it.
        let look = match self.editing_look {
            Some(_) => self.push_into_look(),
            None => None,
        };
        self.record(before, look);
    }

    /// Close an edit: a new revision (a lesson pins one, LS3), one Undo
    /// step — carrying the look's earlier state when the change spread
    /// through it — and the file written.
    fn record(&mut self, before: dpl_doc::Slide, look: Option<LookBefore>) {
        if self.undo.len() >= UNDO_CAP {
            self.undo.remove(0);
        }
        self.slide.rev = before.rev + 1;
        self.undo.push(match look {
            Some(look) => Step::Look {
                slide: before,
                look: Box::new(look),
            },
            None => Step::Slide(before),
        });
        self.save();
    }

    /// Move what the slide holds for its look into the look (LK2b): the
    /// look's next revision is saved, the slide gives those values back.
    /// Returns the look's earlier state for Undo, or `None` when there was
    /// nothing to move.
    fn push_into_look(&mut self) -> Option<LookBefore> {
        let look = dpl_doc::card_preset(&self.slide, &self.styling.looks)?.clone();
        if !dpl_doc::look::holds_style_for(&self.slide, &look) {
            return None;
        }
        let was = self.styling.looks.is_yours(&look.id).then(|| look.clone());
        let mut slide = self.slide.clone();
        let next = dpl_doc::look::push_back(&mut slide, &look);
        if let Err(e) = dpl_doc::look::save_look(
            &dpl_doc::look::look_dir(&self.content_root, &next.id),
            &next,
        ) {
            self.notice = Some(format!("Could not change the look: {e}"));
            return None;
        }
        self.styling.looks.put(next);
        self.slide = slide;
        Some(LookBefore { id: look.id, was })
    }

    /// One edit of a burst (typing): the first keystroke is a revision and
    /// an undo step; the rest of the burst — same field, no pause longer
    /// than [`BURST_PAUSE`] — saves into it. Ten letters, one Undo.
    fn mutate_burst(&mut self, key: &str, f: impl FnOnce(&mut dpl_doc::Slide)) {
        let now = std::time::Instant::now();
        let continuing = self
            .burst
            .as_ref()
            .is_some_and(|(k, at)| k == key && now.duration_since(*at) < BURST_PAUSE);
        if continuing {
            f(&mut self.slide);
            self.save();
        } else {
            self.mutate_in(f);
        }
        self.burst = Some((key.to_owned(), now));
    }

    /// The slide's link to its look, worked out from its own values (LK1).
    pub fn link_state(&self) -> dpl_doc::link::LinkState {
        dpl_doc::link::link_state(&self.slide, &self.styling.looks)
    }

    /// Where a knob's value comes from: yours, the look's, or the show's.
    pub fn source(&self, knob: &dpl_doc::link::Knob) -> dpl_doc::link::Source {
        dpl_doc::link::source(&self.slide, &self.styling.looks, knob)
    }

    /// Give one value back to the look (or the show).
    pub fn reset(&mut self, knob: &dpl_doc::link::Knob) {
        let knob = knob.clone();
        self.mutate(|s| dpl_doc::link::reset(s, &knob));
    }

    /// "Undo all my changes".
    pub fn reset_all(&mut self) {
        self.mutate(dpl_doc::link::reset_all);
    }

    /// "Stop following this look" — nothing changes on screen.
    pub fn stop_following(&mut self) {
        let styling = self.styling.clone();
        self.mutate(|s| dpl_doc::link::stop_following(s, &styling.theme, &styling.looks));
    }

    /// "Follow the look again", keeping my changes or starting fresh.
    pub fn follow_again(&mut self, keep_my_changes: bool) {
        let looks = self.styling.looks.clone();
        self.mutate(|s| dpl_doc::link::follow_again(s, &looks, keep_my_changes));
    }

    /// "Save as a new look…" (LK2): the card as it shows now becomes a look
    /// in the library, and the slide follows it with no changes — except
    /// what a look cannot hold, which stays the slide's own. One revision,
    /// one Undo (the look itself is kept). Returns the new look's id.
    pub fn save_as_new_look(&mut self, label: &str) -> Result<String, String> {
        let mut follows = self.slide.clone();
        let look = dpl_doc::look::save_as_new_look(
            &mut follows,
            &self.styling.theme,
            &self.styling.looks,
            label,
        );
        dpl_doc::look::save_look(
            &dpl_doc::look::look_dir(&self.content_root, &look.id),
            &look,
        )
        .map_err(|e| e.to_string())?;
        let id = look.id.clone();
        self.styling.looks.put(look);
        self.mutate(move |s| *s = follows);
        Ok(id)
    }

    /// The looks this editor offers: the built-in ones plus yours.
    pub fn looks(&self) -> &dpl_doc::Looks {
        &self.styling.looks
    }

    /// "Make these the look's new style" (LK2b): the look takes what the
    /// slide holds for it, as its next revision (for a built-in, your
    /// version), and every slide that follows it shows the change. One
    /// Undo restores the look and the slide.
    pub fn push_back(&mut self) -> Result<(), String> {
        let before = self.slide.clone();
        match self.push_into_look() {
            Some(look) => {
                self.record(before, Some(look));
                Ok(())
            }
            None => Err(self
                .notice
                .take()
                .unwrap_or_else(|| "nothing here for the look to take".into())),
        }
    }

    /// "Back to the built-in" (LK2b): your version of the slide's look
    /// steps aside (kept on disk) and the built-in shows again everywhere.
    /// One Undo brings your version back.
    pub fn back_to_builtin(&mut self) -> Result<(), String> {
        let id = self
            .slide
            .preset
            .clone()
            .ok_or("the slide follows no look")?;
        if !self.styling.looks.replaces_builtin(&id) {
            return Err("there is no version of yours to step aside".into());
        }
        let was = self.styling.looks.get(&id).cloned();
        dpl_doc::look::retire_look(&self.content_root, &id).map_err(|e| e.to_string())?;
        self.styling.looks.retire(&id);
        let before = self.slide.clone();
        self.record(before, Some(LookBefore { id, was }));
        Ok(())
    }

    /// "Open the look" (LK2b): from here, background, colour and font
    /// changes go to the look itself, until Done.
    pub fn open_look(&mut self) {
        if let Some(look) = dpl_doc::card_preset(&self.slide, &self.styling.looks) {
            let shows_on = dpl_doc::look::slides_following(&self.content_root, &look.id)
                .len()
                .max(1);
            self.editing_look = Some(shows_on);
            self.link_menu = false;
            self.notice = None;
        }
    }

    /// A line's link to its line look (LK3), worked out from its values.
    pub fn line_state(&self, id: &str) -> Option<dpl_doc::link::LinkState> {
        dpl_doc::link::line_state(&self.slide, &self.styling.looks, id)
    }

    /// Follow a line look on one line (`None` = no line look: the card's
    /// style). The look's values show; the words stay.
    pub fn use_line_look(&mut self, id: &str, look: Option<&str>) {
        let (id, look) = (id.to_owned(), look.map(str::to_owned));
        self.mutate(move |s| match &look {
            Some(look) => dpl_doc::link::use_line_look(s, &id, look),
            None => {
                dpl_doc::link::reset_line(s, &id);
                if let Some(style) = s
                    .blocks
                    .iter_mut()
                    .find(|b| b.id() == id)
                    .and_then(dpl_doc::Block::style_mut)
                {
                    *style.preset = None;
                    *style.unlinked_from = None;
                }
            }
        });
    }

    /// A line's "Undo all my changes".
    pub fn reset_line(&mut self, id: &str) {
        let id = id.to_owned();
        self.mutate(move |s| dpl_doc::link::reset_line(s, &id));
    }

    /// Stop following a line's look — nothing changes on screen.
    pub fn stop_following_line(&mut self, id: &str) {
        let (id, looks) = (id.to_owned(), self.styling.looks.clone());
        self.mutate(move |s| dpl_doc::link::stop_following_line(s, &looks, &id));
    }

    /// Follow a line's look again, keeping my changes or starting fresh.
    pub fn follow_line_again(&mut self, id: &str, keep_my_changes: bool) {
        let (id, looks) = (id.to_owned(), self.styling.looks.clone());
        self.mutate(move |s| dpl_doc::link::follow_line_again(s, &looks, &id, keep_my_changes));
    }

    /// Set a block's own colour (a hue id; "none" = a picture with no
    /// frame), entrance (`rise` / `none`) or — a picture — fit.
    fn set_line(&mut self, id: &str, field: dpl_doc::link::LineField, value: &str) {
        let (id, value) = (id.to_owned(), value.to_owned());
        self.mutate(move |s| {
            let Some(block) = s.blocks.iter_mut().find(|b| b.id() == id) else {
                return;
            };
            match field {
                dpl_doc::link::LineField::Fit => {
                    if let Some(fit) = block.fit_mut() {
                        *fit = Some(value);
                    }
                }
                dpl_doc::link::LineField::Colour | dpl_doc::link::LineField::Enter => {
                    if let Some(style) = block.style_mut() {
                        match field {
                            dpl_doc::link::LineField::Colour => *style.tone = Some(value),
                            _ => *style.enter = Some(value),
                        }
                    }
                }
                _ => {}
            }
        });
    }

    /// Is this block the card's picture?
    fn is_picture(&self, id: &str) -> bool {
        matches!(self.slide.block(id), Some(dpl_doc::Block::Media { .. }))
    }

    /// A picture's values as they show: fit (`cover` / `contain`), frame
    /// colour (a hue id, or none) and entrance.
    fn picture_fields(&self, id: &str) -> Option<(String, Option<String>, String)> {
        let shown = document::shown(&self.slide, &self.styling);
        let block = shown.block(id)?;
        let style = block.style()?;
        Some((
            block.fit().unwrap_or("cover").to_owned(),
            style.tone.filter(|tone| *tone != "none").map(str::to_owned),
            style.enter.unwrap_or("none").to_owned(),
        ))
    }

    /// The card's picture controls (LK3b) — its look, how it sits, its
    /// frame and its movement — for a surface that shows the picture on
    /// its own tab (the Workbench's Picture tab). Nothing when the card has
    /// no picture.
    pub fn picture_controls(&mut self, ui: &mut Ui) {
        if self.is_picture(MEDIA_SLOT) {
            self.picture_rows(ui, MEDIA_SLOT);
        }
    }

    /// A line look's name ("the card's style" for none).
    fn line_look_label(id: Option<&str>) -> String {
        id.and_then(cl::block_preset).map_or_else(
            || "the card's style".to_owned(),
            |look| look.label.to_owned(),
        )
    }

    /// Is the editor changing the look itself ("Open the look")?
    pub fn is_editing_look(&self) -> bool {
        self.editing_look.is_some()
    }

    /// Get a change ready to spread: count the other slides that follow
    /// the look and paint one of them before and after.
    fn open_spread(&mut self, kind: SpreadKind) {
        let Some(look) = dpl_doc::card_preset(&self.slide, &self.styling.looks).cloned() else {
            return;
        };
        let others: Vec<PathBuf> = dpl_doc::look::slides_following(&self.content_root, &look.id)
            .into_iter()
            .filter(|dir| !same_dir(dir, &self.dir))
            .collect();
        let mut after = self.styling.looks.clone();
        let mut given_back = self.slide.clone();
        match kind {
            SpreadKind::PushBack => after.put(dpl_doc::look::push_back(&mut given_back, &look)),
            SpreadKind::BackToBuiltin => after.retire(&look.id),
        }
        // Another slide that follows the look, else this one as a follower
        // with nothing of its own — what the change does to the others.
        let (preview, theme) = others
            .first()
            .and_then(|dir| {
                dpl_doc::load(dir)
                    .ok()
                    .map(|slide| (slide, dpl_doc::theme_for_slide(dir)))
            })
            .unwrap_or_else(|| (given_back, self.styling.theme.clone()));
        self.spread = Some(Spread {
            kind,
            others: others.len(),
            preview,
            before: dpl_doc::Styling {
                theme: theme.clone(),
                looks: self.styling.looks.clone(),
            },
            after: dpl_doc::Styling {
                theme,
                looks: after,
            },
        });
        self.link_menu = false;
        self.notice = None;
    }

    /// A look's name, by id ("a missing look" when it cannot be found).
    fn look_label(&self, id: &str) -> String {
        self.styling
            .looks
            .get(id)
            .map_or_else(|| "a missing look".to_owned(), |look| look.label.clone())
    }

    /// What "Save as a new look…" suggests calling it.
    fn suggested_look_name(&self) -> String {
        match self.link_state() {
            dpl_doc::link::LinkState::Follows { look, .. }
            | dpl_doc::link::LinkState::OwnStyle { was: Some(look) } => {
                format!("{} — mine", self.look_label(&look))
            }
            _ => "My look".into(),
        }
    }

    /// Is there anything to undo?
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    fn save(&mut self) {
        self.saved = dpl_doc::save(&self.dir, &self.slide).map_err(|e| e.to_string());
    }

    /// Undo restores the earlier CONTENT as a new revision: history only
    /// moves forward, so a lesson pinned to a later rev is never handed an
    /// older document under the same number.
    pub fn undo(&mut self) {
        self.burst = None;
        let Some(step) = self.undo.pop() else {
            return;
        };
        let prev = match step {
            Step::Slide(prev) => prev,
            Step::Look { slide, look } => {
                self.restore_look(*look);
                slide
            }
        };
        let rev = self.slide.rev + 1;
        self.slide = prev;
        self.slide.rev = rev;
        self.save();
    }

    /// Put a look back as it was before a change spread through it — as
    /// its next revision, since history only moves forward; or, when the
    /// built-in showed, step the operator's version aside again.
    fn restore_look(&mut self, before: LookBefore) {
        let outcome = match before.was {
            Some(mut was) => {
                let now = self
                    .styling
                    .looks
                    .get(&before.id)
                    .filter(|_| self.styling.looks.is_yours(&before.id))
                    .map_or(0, |look| look.rev);
                was.rev = was.rev.max(now) + 1;
                let saved = dpl_doc::look::save_look(
                    &dpl_doc::look::look_dir(&self.content_root, &was.id),
                    &was,
                );
                self.styling.looks.put(was);
                saved.map(|_| ())
            }
            None => {
                let retired = dpl_doc::look::retire_look(&self.content_root, &before.id);
                self.styling.looks.retire(&before.id);
                retired.map(|_| ())
            }
        };
        if let Err(e) = outcome {
            self.notice = Some(format!("Could not put the look back: {e}"));
        }
    }

    /// One frame of the editor.
    pub fn ui(&mut self, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        // Toolbar row: title, Undo, saved crumb.
        let toolbar_h = cl::space::S6 + cl::space::S2;
        let bar = Rect::from_min_size(full.min, egui::vec2(full.width(), toolbar_h));
        self.toolbar(ui, bar);

        // The stage: a 16:9 well under the toolbar, room for the tray below.
        let below = Rect::from_min_max(egui::pos2(full.min.x, bar.max.y), full.max);
        // A short window never squeezes the tray past tapping: it takes up
        // to half the room, and the stage shrinks instead. A tapped line's
        // tray (its chip, its words, its looks — LK3) takes half outright.
        let share = match self.selection {
            Selection::Block(_) => 0.5,
            _ => 0.34,
        };
        let tray_h = (below.height() * share).max(TRAY_MIN.min(below.height() * 0.5));
        let stage_area = Rect::from_min_max(
            below.min,
            egui::pos2(below.max.x, below.max.y - tray_h - cl::space::S3),
        );
        let stage = fit_stage(stage_area);
        ui.painter()
            .rect_filled(stage, cl::RADIUS, cl::surface::WELL);

        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        // Trying a look (LK1): the stage shows the slide AS IF it used it;
        // the file is untouched until Keep.
        let mut shown = match &self.trying {
            Some(id) => {
                let mut tried = self.slide.clone();
                use_look(&mut tried, id);
                tried
            }
            None => self.slide.clone(),
        };
        // Trying a line look (LK3): the line shows it; nothing is saved.
        if let Some((line, look)) = &self.trying_line {
            match look {
                Some(look) => dpl_doc::link::use_line_look(&mut shown, line, look),
                None => {
                    dpl_doc::link::reset_line(&mut shown, line);
                    if let Some(style) = shown
                        .blocks
                        .iter_mut()
                        .find(|b| b.id() == *line)
                        .and_then(dpl_doc::Block::style_mut)
                    {
                        *style.preset = None;
                    }
                }
            }
        }
        let map = {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(stage.shrink(2.0)));
            child.set_clip_rect(stage);
            document::render_traced(
                &mut child,
                stage.shrink(2.0),
                &shown,
                &self.styling,
                now,
                t0,
            )
        };
        ui.painter().rect_stroke(
            stage,
            cl::RADIUS,
            Stroke::new(1.0, cl::surface::HAIRLINE),
            StrokeKind::Inside,
        );

        // Tap → select (topmost block, else background).
        let resp = ui.interact(stage, ui.id().with("dpl_editor_stage"), Sense::click());
        let mut picked = None;
        if let Some(pos) = resp.interact_pointer_pos() {
            if resp.clicked() {
                picked = Some(match map.block_at(pos) {
                    Some(id) => Selection::Block(id.to_string()),
                    None => Selection::Background,
                });
            }
        }
        // Each line on the stage is its own tap target, named and reachable
        // (LK3) — a person taps the words; a pilot finds `editor.line/<id>`.
        for (block_id, rect) in &map.blocks {
            let Some(words) = self.slide.block(block_id).and_then(dpl_doc::Block::words) else {
                continue;
            };
            let text = words.text.to_owned();
            let line = ui.interact(
                *rect,
                ui.id().with(("dpl_editor_line", block_id)),
                Sense::click(),
            );
            line.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &text));
            ui.pilot_track(
                &line,
                NodeMetadata::new(
                    PilotId::new("editor.line").child(&PilotKey::new(block_id)),
                    egui_pilot_ai::protocol::Role::Button,
                    &text,
                ),
            );
            if cl::tip(line, "Tap to change this line").clicked() {
                picked = Some(Selection::Block(block_id.clone()));
            }
        }
        if let Some(selection) = picked {
            self.selection = selection;
            self.trying_line = None;
            self.line_menu = false;
        }
        self.paint_selection(ui, &map);

        // The contextual tray.
        let tray = Rect::from_min_max(
            egui::pos2(below.min.x, stage.max.y + cl::space::S3),
            below.max,
        );
        self.tray(ui, tray);

        ui.ctx().request_repaint(); // the document's blend never rests
    }

    fn toolbar(&mut self, ui: &mut Ui, bar: Rect) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let chip_rect = self.look_chip(&mut child);
        child.add_space(cl::space::S3);
        child.label(
            RichText::new("Tap the slide to change it")
                .color(cl::ink::MUTED)
                .size(cl::Fluid::of(bar).fs_body()),
        );
        if self.link_menu {
            self.link_menu_ui(ui, chip_rect);
        }
        if self.naming.is_some() {
            self.naming_ui(ui, chip_rect);
        }
        if self.spread.is_some() {
            self.spread_ui(ui, chip_rect);
        }
        child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let crumb = match &self.saved {
                Ok(()) if !self.undo.is_empty() => "saved".to_string(),
                Ok(()) => String::new(),
                Err(e) => format!("save failed: {e}"),
            };
            if !crumb.is_empty() {
                ui.label(RichText::new(crumb).small().color(cl::ink::SUBTLE));
                ui.add_space(cl::space::S3);
            }
            let can_undo = !self.undo.is_empty();
            if big_button(ui, "undo", "Undo", can_undo, "Take back the last change") {
                self.undo();
            }
        });
    }

    /// The link bar for surfaces that host the editor's controls without
    /// its toolbar (the Workbench card editor's tabs): the look chip with
    /// its menu, and Undo — so every tab shows what the slide follows and
    /// can take the last change back (LK1). It wraps rather than run past
    /// a narrow column, so Done and Undo are always within reach.
    pub fn link_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            let chip_rect = self.look_chip(ui);
            if self.link_menu {
                self.link_menu_ui(ui, chip_rect);
            }
            if self.naming.is_some() {
                self.naming_ui(ui, chip_rect);
            }
            if self.spread.is_some() {
                self.spread_ui(ui, chip_rect);
            }
            ui.add_space(cl::space::S3);
            let can_undo = !self.undo.is_empty();
            if big_button(ui, "undo", "Undo", can_undo, "Take back the last change") {
                self.undo();
            }
        });
    }

    /// The background the slide SHOWS — its own, else its look's, else the
    /// show's.
    pub fn shown_background(&self) -> dpl_doc::Background {
        self.inherited_background()
    }

    /// The look chip (LK1): "Look: Neon intro · 3 changes", "Own style (was
    /// Neon intro)" or "The show's style". Tap for the menu.
    fn look_chip(&mut self, ui: &mut Ui) -> Rect {
        if let Some(shows_on) = self.editing_look {
            // Editing the look (LK2b): the chip is the banner, Done beside it.
            let label = self
                .slide
                .preset
                .as_deref()
                .map_or_else(String::new, |id| self.look_label(id));
            let text = match shows_on {
                1 => format!("Editing the look {label} — shows on 1 slide"),
                n => format!("Editing the look {label} — shows on {n} slides"),
            };
            let resp = chip_response(
                ui,
                "look-chip",
                &text,
                true,
                "Background, colour and font changes go to the look and every slide that follows it; sizes and inks stay on this slide",
            );
            if hero_chip(ui, "look-done", "Done", "Stop editing the look") {
                self.editing_look = None;
            }
            return resp.rect;
        }
        if let Some(id) = &self.trying {
            // Trying (LK1): the stage shows the look, nothing is saved — the
            // chip says so instead of naming a link that has not changed.
            let text = format!("Trying: {}", self.look_label(id));
            let resp = chip_response(
                ui,
                "look-chip",
                &text,
                true,
                "Keep the look, or Cancel to go back — nothing is saved until you Keep",
            );
            return resp.rect;
        }
        let state = self.link_state();
        // The chip says when the look is your version of a built-in (LK2b).
        let text = state.chip(|id| {
            if self.styling.looks.replaces_builtin(id) {
                format!("{} (your version)", self.look_label(id))
            } else {
                self.look_label(id)
            }
        });
        let tip = match &state {
            dpl_doc::link::LinkState::Follows { .. } => {
                "This slide follows its look. Tap to see your changes, give them back, or stop following"
            }
            dpl_doc::link::LinkState::OwnStyle { .. } => {
                "This slide has its own style. Tap to follow its look again"
            }
            dpl_doc::link::LinkState::Show { .. } => {
                "This slide uses the show's style. Pick a look below to follow one"
            }
            dpl_doc::link::LinkState::Missing { .. } => {
                "This slide's look can't be found, so it shows the show's style. Pick a look below"
            }
        };
        let resp = chip_response(ui, "look-chip", &text, self.link_menu, tip);
        if resp.clicked() {
            self.link_menu = !self.link_menu;
            self.naming = None;
            if !self.link_menu {
                self.notice = None;
            }
        }
        resp.rect
    }

    /// The look chip's menu: your changes (each with its own way back),
    /// "Undo all my changes", and stop / follow again.
    fn link_menu_ui(&mut self, ui: &mut Ui, under: Rect) {
        let state = self.link_state();
        let changes = dpl_doc::link::changes(&self.slide);
        let area = egui::Area::new(ui.id().with("dpl_editor_link_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(under.left_bottom() + egui::vec2(0.0, cl::space::S1));
        area.show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(cl::surface::RAISED)
                .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                .corner_radius(cl::RADIUS)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    if let Some(note) = &self.notice {
                        ui.label(RichText::new(note).small().color(cl::ink::STRONG));
                        if !changes.is_empty() {
                            ui.label(small(
                                "A look doesn't hold these, so they stay this slide's own:",
                            ));
                        }
                        ui.add_space(cl::space::S2);
                    }
                    ui.label(tray_title(match &state {
                        dpl_doc::link::LinkState::OwnStyle { .. } => "Own style",
                        _ => "Your changes",
                    }));
                    if changes.is_empty() {
                        ui.label(small("Nothing yet — everything follows."));
                    }
                    for knob in &changes {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("• {}", knob.label()))
                                    .small()
                                    .color(cl::ink::BASE),
                            );
                            if !matches!(state, dpl_doc::link::LinkState::OwnStyle { .. }) {
                                let back = give_back_label(&self.slide, &self.styling.looks, knob);
                                if chip(
                                    ui,
                                    &format!("reset-{}", knob_key(knob)),
                                    back,
                                    false,
                                    "Give this value back — the slide follows again here",
                                ) {
                                    self.reset(knob);
                                }
                            }
                        });
                    }
                    ui.add_space(cl::space::S2);
                    match &state {
                        dpl_doc::link::LinkState::Follows { look, .. } => {
                            if !changes.is_empty()
                                && chip(
                                    ui,
                                    "reset-all",
                                    "Undo all my changes",
                                    false,
                                    "Give every value back to the look",
                                )
                            {
                                self.reset_all();
                            }
                            if chip(
                                ui,
                                "stop-following",
                                "Stop following this look",
                                false,
                                &format!(
                                    "Make the slide fully yours. Nothing changes on screen; {} stops updating it",
                                    self.look_label(look)
                                ),
                            ) {
                                self.stop_following();
                                self.link_menu = false;
                            }
                            let follows = dpl_doc::card_preset(&self.slide, &self.styling.looks);
                            if follows
                                .is_some_and(|look| dpl_doc::look::holds_style_for(&self.slide, look))
                                && chip(
                                    ui,
                                    "push-look",
                                    "Make these the look's new style…",
                                    false,
                                    "Your changes become the look's style, on every slide that follows it — you see how many first",
                                )
                            {
                                self.open_spread(SpreadKind::PushBack);
                            }
                            if chip(
                                ui,
                                "open-look",
                                "Open the look",
                                false,
                                "Change the look itself — every slide that follows it changes too",
                            ) {
                                self.open_look();
                            }
                            if self.styling.looks.replaces_builtin(look)
                                && chip(
                                    ui,
                                    "back-to-builtin",
                                    "Back to the built-in",
                                    false,
                                    "Your version steps aside (it is kept) and the built-in look shows again",
                                )
                            {
                                self.open_spread(SpreadKind::BackToBuiltin);
                            }
                        }
                        dpl_doc::link::LinkState::OwnStyle { was: Some(was) } => {
                            if chip(
                                ui,
                                "follow-keep",
                                "Follow the look again — keep my changes",
                                false,
                                &format!(
                                    "Follow {} again; what differs stays yours",
                                    self.look_label(was)
                                ),
                            ) {
                                self.follow_again(true);
                                self.link_menu = false;
                            }
                            if chip(
                                ui,
                                "follow-fresh",
                                "Follow the look again — start fresh",
                                false,
                                &format!(
                                    "Follow {} again and give back everything it styles",
                                    self.look_label(was)
                                ),
                            ) {
                                self.follow_again(false);
                                self.link_menu = false;
                            }
                        }
                        dpl_doc::link::LinkState::OwnStyle { was: None }
                        | dpl_doc::link::LinkState::Show { .. } => {
                            if !changes.is_empty()
                                && chip(
                                    ui,
                                    "reset-all",
                                    "Undo all my changes",
                                    false,
                                    "Give every value back to the show",
                                )
                            {
                                self.reset_all();
                            }
                        }
                        dpl_doc::link::LinkState::Missing { .. } => {
                            if chip(
                                ui,
                                "use-show",
                                "Use the show's style",
                                false,
                                "Stop naming the look that can't be found; the slide keeps the show's style",
                            ) {
                                self.mutate(|s| s.preset = None);
                                self.link_menu = false;
                            }
                        }
                    }
                    if chip(
                        ui,
                        "save-look",
                        "Save as a new look…",
                        false,
                        "Make this card's background, colour and font a look you can use on any slide",
                    ) {
                        self.naming = Some(self.suggested_look_name());
                        self.notice = None;
                        self.link_menu = false;
                    }
                    if chip(ui, "menu-done", "Done", false, "Close this menu") {
                        self.link_menu = false;
                        self.notice = None;
                    }
                });
        });
    }

    /// "Save as a new look…" (LK2): name it, then Save. The sheet floats
    /// under the chip like its menu.
    fn naming_ui(&mut self, ui: &mut Ui, under: Rect) {
        let Some(mut name) = self.naming.clone() else {
            return;
        };
        let (mut save, mut cancel) = (false, false);
        let area = egui::Area::new(ui.id().with("dpl_editor_naming"))
            .order(egui::Order::Foreground)
            .fixed_pos(under.left_bottom() + egui::vec2(0.0, cl::space::S1));
        area.show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(cl::surface::RAISED)
                .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                .corner_radius(cl::RADIUS)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.label(tray_title("Save as a new look"));
                    ui.label(small(
                        "This card's background, colour and font become a look you can use on any slide. This slide follows it.",
                    ));
                    ui.add_space(cl::space::S2);
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut name)
                            .hint_text("Name the look")
                            .desired_width(320.0),
                    );
                    ui.pilot_track(
                        &field,
                        NodeMetadata::new(
                            PilotId::new("editor").child(&PilotKey::new("look-name")),
                            egui_pilot_ai::protocol::Role::TextInput,
                            "Name the look",
                        ),
                    );
                    let field = cl::tip(field, "What the look is called among your looks");
                    if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        save = true;
                    }
                    if let Some(note) = &self.notice {
                        ui.label(RichText::new(note).small().color(cl::Tone::Coral.color()));
                    }
                    ui.add_space(cl::space::S2);
                    ui.horizontal(|ui| {
                        if hero_chip(
                            ui,
                            "look-save",
                            "Save",
                            "Save the look and make this slide follow it",
                        ) {
                            save = true;
                        }
                        if chip(
                            ui,
                            "look-cancel",
                            "Cancel",
                            false,
                            "Close without saving a look",
                        ) {
                            cancel = true;
                        }
                    });
                });
        });
        if cancel {
            self.naming = None;
            self.notice = None;
            return;
        }
        if save {
            let label = name.trim().to_owned();
            if label.is_empty() {
                self.notice = Some("Give the look a name first.".into());
            } else {
                match self.save_as_new_look(&label) {
                    Ok(_) => {
                        self.naming = None;
                        self.notice =
                            Some(format!("Saved as “{label}” — this slide follows it now."));
                        self.link_menu = true;
                        return;
                    }
                    Err(e) => self.notice = Some(format!("Could not save the look: {e}")),
                }
            }
        }
        self.naming = Some(name);
    }

    /// The sheet before a change spreads (LK2b): how many other slides it
    /// changes, one of them before and after, then OK or Cancel.
    fn spread_ui(&mut self, ui: &mut Ui, under: Rect) {
        let Some(spread) = &self.spread else {
            return;
        };
        let kind = spread.kind;
        let label = self
            .slide
            .preset
            .as_deref()
            .map_or_else(String::new, |id| self.look_label(id));
        let (title, confirm) = match kind {
            SpreadKind::PushBack => (
                "Make these the look's new style",
                "Make it the look's style",
            ),
            SpreadKind::BackToBuiltin => ("Back to the built-in", "Back to the built-in"),
        };
        let count = match spread.others {
            0 => format!("Only this slide follows {label}."),
            1 => "This changes 1 other slide.".to_owned(),
            n => format!("This changes {n} other slides."),
        };
        let (mut ok, mut cancel) = (false, false);
        let now = ui.input(|i| i.time);
        let area = egui::Area::new(ui.id().with("dpl_editor_spread"))
            .order(egui::Order::Foreground)
            .fixed_pos(under.left_bottom() + egui::vec2(0.0, cl::space::S1));
        area.show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(cl::surface::RAISED)
                .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                .corner_radius(cl::RADIUS)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.label(tray_title(title));
                    let line = ui.label(RichText::new(&count).color(cl::ink::STRONG));
                    ui.pilot_track(
                        &line,
                        NodeMetadata::new(
                            PilotId::new("editor").child(&PilotKey::new("spread-count")),
                            egui_pilot_ai::protocol::Role::Label,
                            &count,
                        ),
                    );
                    ui.add_space(cl::space::S2);
                    ui.horizontal(|ui| {
                        for (caption, styling) in
                            [("Before", &spread.before), ("After", &spread.after)]
                        {
                            ui.vertical(|ui| {
                                ui.label(small(caption));
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::vec2(164.0, 92.0), Sense::hover());
                                ui.painter()
                                    .rect_filled(rect, cl::RADIUS_SM, cl::surface::WELL);
                                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                                inner.set_clip_rect(rect.intersect(ui.clip_rect()));
                                // Still frames: the entrance has long played.
                                let _ = document::render_thumbnail(
                                    &mut inner,
                                    rect,
                                    &spread.preview,
                                    styling,
                                    now,
                                    now - 10.0,
                                );
                            });
                        }
                    });
                    ui.add_space(cl::space::S2);
                    ui.horizontal(|ui| {
                        if hero_chip(
                            ui,
                            "spread-confirm",
                            confirm,
                            "Do it — one Undo takes it back",
                        ) {
                            ok = true;
                        }
                        if chip(
                            ui,
                            "spread-cancel",
                            "Cancel",
                            false,
                            "Leave the look as it is",
                        ) {
                            cancel = true;
                        }
                    });
                });
        });
        if cancel {
            self.spread = None;
        } else if ok {
            let others = spread.others;
            self.spread = None;
            let done = match kind {
                SpreadKind::PushBack => self.push_back().map(|()| match others {
                    0 => format!("{label} has your changes now."),
                    1 => format!("{label} has your changes now — 1 other slide shows them."),
                    n => format!("{label} has your changes now — {n} other slides show them."),
                }),
                SpreadKind::BackToBuiltin => self
                    .back_to_builtin()
                    .map(|()| format!("{label} is the built-in again — your version is kept.")),
            };
            self.notice = Some(done.unwrap_or_else(|e| format!("Could not change the look: {e}")));
            self.link_menu = true;
        }
    }

    fn paint_selection(&self, ui: &Ui, map: &document::RenderMap) {
        let (rect, tone) = match &self.selection {
            Selection::None => return,
            Selection::Background => (map.background, cl::Tone::Cyan),
            Selection::Block(id) => match map.blocks.iter().find(|(b, _)| b == id) {
                Some((_, r)) => (r.expand(cl::space::S2), cl::Tone::Amber),
                None => return,
            },
        };
        ui.painter().rect_stroke(
            rect,
            cl::RADIUS_SM,
            Stroke::new(2.0, tone.color()),
            StrokeKind::Outside,
        );
    }

    // ── the trays — the THING ITSELF, tap to apply ──────────────────────

    fn tray(&mut self, ui: &mut Ui, area: Rect) {
        ui.painter()
            .rect_filled(area, cl::RADIUS, cl::surface::RAISED);
        let inner = area.shrink(cl::space::S4);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.set_clip_rect(inner);
        match self.selection.clone() {
            Selection::None => self.look_tray(&mut child),
            Selection::Background => self.background_tray(&mut child),
            Selection::Block(id) if self.is_picture(&id) => self.picture_tray(&mut child, &id),
            Selection::Block(id) => self.text_tray(&mut child, &id),
        }
    }

    /// Nothing selected → the LOOK presets (W3), applied whole.
    /// The layout's own choices (LS4a, Q5 level 3): one row per variant axis
    /// of the slide's component — Alignment: Left · Centered. Tapping one
    /// sets this slide's variant (one revision, one Undo); tapping the
    /// layout's default gives the choice back to it.
    fn layout_rows(&mut self, ui: &mut Ui) {
        let Some(contract) = dpl_slides::contract_of(self.slide.component_id()) else {
            return;
        };
        let mut pick: Option<(String, String, bool)> = None;
        for (index, axis) in contract.variant_axes.iter().enumerate() {
            let current = contract
                .axis_value(&self.slide.variant, &axis.id)
                .unwrap_or(axis.default.as_str())
                .to_owned();
            ui.horizontal_wrapped(|ui| {
                if index == 0 {
                    ui.label(tray_title("Layout"));
                }
                ui.label(small(&axis.label));
                for option in &axis.options {
                    let on = current == option.id;
                    let tip = if on {
                        format!("{} is {} on this slide", axis.label, option.label)
                    } else {
                        format!("{} — {}", axis.tooltip, option.label.to_lowercase())
                    };
                    if chip(
                        ui,
                        &format!("variant-{}-{}", axis.id, option.id),
                        &option.label,
                        on,
                        &tip,
                    ) && !on
                    {
                        pick = Some((
                            axis.id.clone(),
                            option.id.clone(),
                            option.id == axis.default,
                        ));
                    }
                }
            });
        }
        if let Some((axis, option, is_default)) = pick {
            self.mutate(|slide| {
                if is_default {
                    slide.variant.remove(&axis);
                } else {
                    slide.variant.insert(axis, option);
                }
            });
        }
    }

    /// The tap the On tap panel is previewing — 0 is before any tap.
    pub fn preview_step(&self) -> usize {
        self.preview.min(self.slide.reveals.len())
    }

    /// Put line `id` on tap `tap` (counting from 1), or at the start
    /// (`None`) — one revision, one Undo — and let the script follow.
    fn set_tap(&mut self, id: &str, tap: Option<usize>) {
        let mut reveals = self.slide.reveals.clone();
        reveals.retain(|r| r != id);
        if let Some(tap) = tap {
            let at = tap.saturating_sub(1).min(reveals.len());
            reveals.insert(at, id.to_owned());
        }
        if reveals != self.slide.reveals {
            self.mutate(|slide| slide.reveals = reveals);
            self.follow_taps();
        }
    }

    /// The script's beats follow the card's taps — after a change here, an
    /// Undo, or anything else that moved them. A card with no words yet
    /// keeps no script file.
    fn follow_taps(&mut self) {
        if let Some(script) = &self.script {
            let next =
                dpl_doc::script::follow_reveals(Some(script), &self.slide, &self.slide.label());
            if &next != script {
                self.write_script(next);
            }
        }
    }

    fn write_script(&mut self, script: dpl_doc::script::Script) {
        self.script_saved = dpl_doc::script::save_script(&self.dir, &script);
        self.script = Some(script);
    }

    /// The On tap panel (RL, ruled 2026-10-25 "Beats now, card taps in
    /// LS4c"): what each tap of the screen shows — every line starts on the
    /// card, or waits for a tap — and the teleprompter's words for each tap,
    /// written into the card's script as you type; and the card as it looks
    /// at the chosen tap — beside them when wide, after them otherwise, so
    /// taps and words always show first.
    pub fn taps_ui(&mut self, ui: &mut Ui) {
        self.follow_taps();
        let lines: Vec<(String, String)> = self
            .slide
            .blocks
            .iter()
            .map(|b| (b.id().to_owned(), line_name(b)))
            .collect();
        // Wide: taps and words on the left, the card on the right. Else one
        // column, taps and words first — the preview is a scroll away.
        let w = ui.available_width();
        if w >= 760.0 {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(w * 0.62);
                    self.lines_part(ui, &lines);
                    ui.add_space(cl::space::S3);
                    self.words_part(ui, &lines);
                });
                ui.add_space(cl::space::S3);
                ui.vertical(|ui| self.preview_part(ui));
            });
        } else {
            self.lines_part(ui, &lines);
            ui.add_space(cl::space::S3);
            self.words_part(ui, &lines);
            ui.add_space(cl::space::S3);
            self.preview_part(ui);
        }
    }

    /// What each tap shows: each line at the start or on a tap.
    fn lines_part(&mut self, ui: &mut Ui, lines: &[(String, String)]) {
        let taps = self.slide.reveals.len();
        ui.label(tray_title("What each tap shows"));
        ui.label(small("Put a line on a tap and it waits for that tap."));
        ui.add_space(cl::space::S1);
        let mut change: Option<(String, Option<usize>)> = None;
        for (id, name) in lines {
            let at = self
                .slide
                .reveals
                .iter()
                .position(|r| r == id)
                .map(|i| i + 1);
            ui.horizontal_wrapped(|ui| {
                // A fixed narrow name column keeps a line and its taps on
                // one row in the common case.
                let name_label = column_label(
                    ui,
                    RichText::new(name).small().color(cl::ink::BASE),
                    150.0,
                    30.0,
                );
                cl::tip(name_label, name);
                if chip(
                    ui,
                    &format!("tap-{id}-start"),
                    "At the start",
                    at.is_none(),
                    "Show this line as soon as the card appears",
                ) && at.is_some()
                {
                    change = Some((id.clone(), None));
                }
                for tap in 1..=taps {
                    if chip(
                        ui,
                        &format!("tap-{id}-{tap}"),
                        &format!("Tap {tap}"),
                        at == Some(tap),
                        &format!("Show this line on tap {tap}"),
                    ) && at != Some(tap)
                    {
                        change = Some((id.clone(), Some(tap)));
                    }
                }
                if at != Some(taps)
                    && chip(
                        ui,
                        &format!("tap-{id}-new"),
                        "+ New tap",
                        false,
                        "Show this line on a tap of its own, after the others",
                    )
                {
                    change = Some((id.clone(), Some(taps + 1)));
                }
            });
        }
        if let Some((id, tap)) = change {
            self.set_tap(&id, tap);
        }
    }

    /// The card as it looks at the chosen point, and the chips that choose
    /// it.
    fn preview_part(&mut self, ui: &mut Ui) {
        let taps = self.slide.reveals.len();
        ui.horizontal_wrapped(|ui| {
            ui.label(small("See it"));
            for step in 0..=taps {
                let label = if step == 0 {
                    "Before any tap".to_owned()
                } else {
                    format!("After tap {step}")
                };
                if chip(
                    ui,
                    &format!("preview-{step}"),
                    &label,
                    self.preview_step() == step,
                    "See the card as it looks at this point",
                ) {
                    self.preview = step;
                    self.preview_clock.restart();
                }
            }
        });
        let w = ui.available_width().min(420.0);
        let (stage, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), Sense::hover());
        ui.painter()
            .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
        let now = ui.input(|i| i.time);
        let t0 = self.preview_clock.tick(now);
        let inner_rect = stage.shrink(1.0);
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
        inner.set_clip_rect(stage.intersect(ui.clip_rect()));
        let _ = document::render_thumbnail_step(
            &mut inner,
            inner_rect,
            &self.slide,
            &self.styling,
            self.preview_step(),
            now,
            t0,
        );
        ui.painter().rect_stroke(
            stage,
            cl::RADIUS,
            egui::Stroke::new(1.0, cl::surface::HAIRLINE),
            egui::StrokeKind::Inside,
        );
    }

    /// What you'll say: a cue and a note per tap, and notes for the card.
    fn words_part(&mut self, ui: &mut Ui, lines: &[(String, String)]) {
        ui.label(tray_title("What you'll say"));
        ui.label(small("Your teleprompter, point by point."));
        let label = self.slide.label();
        let mut script = self
            .script
            .clone()
            .unwrap_or_else(|| dpl_doc::script::follow_reveals(None, &self.slide, &label));
        let mut edited = false;
        for (i, beat) in script.beats.iter_mut().enumerate() {
            let heading = if i == 0 {
                "When it appears".to_owned()
            } else {
                let name = lines
                    .iter()
                    .find(|(id, _)| *id == beat.id)
                    .map_or(beat.id.as_str(), |(_, name)| name.as_str());
                format!("Tap {i} — {name}")
            };
            let mut note = beat.note.clone().unwrap_or_default();
            let row_w = ui.available_width();
            let wide = row_w >= 420.0;
            let (cue, note_field) = if wide {
                // One row a point: where it is (the same narrow column as
                // the lines above), what to say, how to say it.
                ui.horizontal(|ui| {
                    let where_ = column_label(ui, small(&heading), 150.0, 24.0);
                    cl::tip(where_, &heading);
                    let cue = ui.add(
                        egui::TextEdit::singleline(&mut beat.cue)
                            .hint_text("What you'll say")
                            .desired_width((row_w - 150.0) * 0.6),
                    );
                    let note_field = ui.add(
                        egui::TextEdit::singleline(&mut note)
                            .hint_text("How to say it (optional)")
                            .desired_width(f32::INFINITY),
                    );
                    (cue, note_field)
                })
                .inner
            } else {
                ui.label(small(&heading));
                let cue = ui.add(
                    egui::TextEdit::singleline(&mut beat.cue)
                        .hint_text("What you'll say")
                        .desired_width(f32::INFINITY),
                );
                let note_field = ui.add(
                    egui::TextEdit::singleline(&mut note)
                        .hint_text("How to say it (optional)")
                        .desired_width(f32::INFINITY),
                );
                (cue, note_field)
            };
            ui.pilot_track(
                &cue,
                NodeMetadata::new(
                    PilotId::new("editor").child(&PilotKey::new(format!("cue-{i}"))),
                    egui_pilot_ai::protocol::Role::TextInput,
                    "What you'll say",
                ),
            );
            edited |= cl::tip(cue, "The line your teleprompter shows here").changed();
            if cl::tip(note_field, "A reminder of how to say it, shown smaller").changed() {
                beat.note = (!note.trim().is_empty()).then_some(note);
                edited = true;
            }
            ui.add_space(cl::space::S1);
        }
        ui.label(small("Notes for the whole card"));
        let body = ui.add(
            egui::TextEdit::multiline(&mut script.body)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        edited |= cl::tip(body, "Anything else to remember on this card").changed();
        if edited {
            self.write_script(script);
        }
        if let Err(e) = &self.script_saved {
            ui.label(
                RichText::new(format!("Your words could not be saved: {e}"))
                    .color(cl::Tone::Coral.color()),
            );
        }
    }

    fn look_tray(&mut self, ui: &mut Ui) {
        // The layout's own choices come first, under their own name — they
        // arrange this card, they are not a look.
        self.layout_rows(ui);
        ui.horizontal(|ui| {
            ui.label(tray_title("Looks — tap one to try it on"));
            // Trying (LK1): nothing is saved until Keep.
            if let Some(id) = self.trying.clone() {
                ui.add_space(cl::space::S3);
                if hero_chip(
                    ui,
                    "keep",
                    "Keep",
                    "Use this look on the slide (one Undo takes it back)",
                ) {
                    self.editing_look = None;
                    self.mutate(|s| use_look(s, &id));
                    self.trying = None;
                }
                if chip(
                    ui,
                    "cancel",
                    "Cancel",
                    false,
                    "Go back to how the slide was",
                ) {
                    self.trying = None;
                }
            }
        });
        // The chosen look's knobs, in plain words (its `exposes`): what the
        // tabs above may change without leaving the look.
        if let Some(look) = dpl_doc::card_preset(&self.slide, &self.styling.looks) {
            let knobs: Vec<&str> = look
                .exposes
                .iter()
                .map(|knob| match knob.as_str() {
                    "ambience" => "moving pattern",
                    "motion" => "movement",
                    other => other,
                })
                .collect();
            // One line, never two: a short window keeps room for the tiles,
            // and the whole sentence is in the tooltip.
            let using = format!(
                "Using {} — you can change its {}",
                look.label,
                knobs.join(", ")
            );
            let line = ui.add(
                egui::Label::new(RichText::new(&using).small().color(cl::ink::MUTED)).truncate(),
            );
            cl::tip(line, &using);
        }
        // Your own looks lead the row, newest first, each marked "Yours";
        // then the built-in ones, "Your version" where you replaced one.
        let looks = &self.styling.looks;
        let tiles: Vec<(dpl_doc::Look, Option<&'static str>)> = looks
            .own()
            .map(|look| (look.clone(), Some("Yours")))
            .chain(looks.builtins_shown().map(|look| {
                let badge = looks.replaces_builtin(&look.id).then_some("Your version");
                (look.clone(), badge)
            }))
            .collect();
        // The tiles take the height that is left (a short window shrinks
        // them, never hides them), keeping their 150:84 shape.
        let tile_h = (ui.available_height() - cl::space::S3).clamp(48.0, 84.0);
        let tile = egui::vec2(tile_h * 150.0 / 84.0, tile_h);
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (look, badge) in &tiles {
                        let (rect, resp) = ui.allocate_exact_size(tile, Sense::click());
                        let painter = ui.painter_at(rect);
                        paint_backdrop(&painter, rect, &look.background);
                        painter.text(
                            rect.left_top() + egui::vec2(8.0, 14.0),
                            Align2::LEFT_CENTER,
                            &look.label,
                            FontId::new(15.0, cl::font_family(&look.font)),
                            cl::ink::STRONG,
                        );
                        // Under the name, where the tray never clips it.
                        if let Some(badge) = badge {
                            painter.text(
                                rect.left_top() + egui::vec2(8.0, 34.0),
                                Align2::LEFT_CENTER,
                                *badge,
                                FontId::proportional(12.0),
                                cl::Tone::Amber.color(),
                            );
                        }
                        let chosen = self.slide.preset.as_deref() == Some(look.id.as_str());
                        let tried = self.trying.as_deref() == Some(look.id.as_str());
                        card_edge(&painter, rect, resp.hovered() || chosen || tried);
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                true,
                                chosen,
                                &look.label,
                            )
                        });
                        ui.pilot_track(
                            &resp,
                            NodeMetadata::new(
                                PilotId::new("editor.look").child(&PilotKey::new(&look.id)),
                                egui_pilot_ai::protocol::Role::Button,
                                &look.label,
                            ),
                        );
                        let resp = cl::tip(
                            resp,
                            if chosen {
                                "This slide's look — its background, colour and font come from it"
                            } else {
                                "Try this look on the slide; Keep makes the slide follow it"
                            },
                        );
                        if resp.clicked() && !chosen {
                            self.trying = Some(look.id.clone());
                        }
                    }
                });
            });
    }

    /// Background selected → Colours (mood A/B) + Ambience. Shows the
    /// RESOLVED background; "Show theme" clears the slide's override.
    fn background_tray(&mut self, ui: &mut Ui) {
        let knob = dpl_doc::link::Knob::Background;
        ui.horizontal(|ui| {
            ui.label(tray_title("Background"));
            self.yours_row(ui, &knob);
        });
        let resolved = self.inherited_background();
        let (cur_a, cur_b, cur_amb) = match &resolved {
            dpl_doc::Background::MoodBlend {
                moods, ambience, ..
            } => (
                moods.first().cloned(),
                moods.get(1).cloned(),
                ambience.clone(),
            ),
            dpl_doc::Background::Mood { mood } => (Some(mood.clone()), None, None),
            dpl_doc::Background::None => (None, None, None),
        };

        egui::ScrollArea::horizontal()
            .id_salt("bg_colours")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(small("Colour A"));
                        if let Some(pick) = mood_row(ui, "a", cur_a.as_deref()) {
                            self.set_blend_mood(0, pick);
                        }
                    });
                    ui.add_space(cl::space::S3);
                    ui.vertical(|ui| {
                        ui.label(small("Colour B"));
                        if let Some(pick) = mood_row(ui, "b", cur_b.as_deref()) {
                            self.set_blend_mood(1, pick);
                        }
                    });
                });
            });

        ui.add_space(cl::space::S2);
        ui.label(small("Motion"));
        ui.horizontal(|ui| {
            for amb in [None, Some("pulse"), Some("drift"), Some("rings")] {
                let label = amb.unwrap_or("none");
                let on = cur_amb.as_deref() == amb;
                if chip(
                    ui,
                    &format!("ambience-{label}"),
                    label,
                    on,
                    "A moving pattern over the background",
                ) {
                    let amb = amb.map(str::to_string);
                    self.set_ambience(amb);
                }
            }
        });
        let _ = AMBIENCES;
    }

    /// Text block selected → Font / Size / Ink / Text. Font and ink show
    /// the RESOLVED value; "Show theme" clears an override back to the show.
    fn text_tray(&mut self, ui: &mut Ui, id: &str) {
        // The tray grew with line looks (LK3): it scrolls, so a short window
        // never hides a row.
        egui::ScrollArea::vertical()
            .id_salt("text_tray")
            .auto_shrink([false, false])
            .show(ui, |ui| self.text_tray_rows(ui, id));
    }

    /// A tapped picture (LK3b): its look, how it sits, its frame and its
    /// movement — scrolling, like the text tray.
    fn picture_tray(&mut self, ui: &mut Ui, id: &str) {
        egui::ScrollArea::vertical()
            .id_salt("picture_tray")
            .auto_shrink([false, false])
            .show(ui, |ui| self.picture_rows(ui, id));
    }

    fn picture_rows(&mut self, ui: &mut Ui, id: &str) {
        let Some((fit, frame, enter)) = self.picture_fields(id) else {
            return;
        };
        self.block_head(ui, id, "Picture");
        ui.add_space(cl::space::S2);
        self.line_look_row(ui, id, "picture");
        let knob = |field| dpl_doc::link::Knob::Line {
            id: id.to_string(),
            field,
        };
        // One row each, stacked: a narrow column (the Workbench's Picture
        // tab) never has a row run past its edge.
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(small("How it sits"));
            self.yours_row(ui, &knob(dpl_doc::link::LineField::Fit));
        });
        ui.horizontal_wrapped(|ui| {
            for (value, label, tip) in [
                (
                    "cover",
                    "Fill the card",
                    "Fills the whole card, cropping the edges if it has to",
                ),
                (
                    "contain",
                    "Show all of it",
                    "Shows the whole picture, with space around it",
                ),
            ] {
                if chip(ui, &format!("fit-{value}"), label, fit == value, tip) {
                    self.set_line(id, dpl_doc::link::LineField::Fit, value);
                }
            }
        });
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(small("Movement"));
            self.yours_row(ui, &knob(dpl_doc::link::LineField::Enter));
        });
        ui.horizontal_wrapped(|ui| {
            for (value, label, tip) in [
                (
                    "rise",
                    "Rise in",
                    "The picture rises in when the slide shows",
                ),
                (
                    "none",
                    "Stay still",
                    "The picture is simply there — no movement",
                ),
            ] {
                if chip(ui, &format!("enter-{value}"), label, enter == value, tip) {
                    self.set_line(id, dpl_doc::link::LineField::Enter, value);
                }
            }
        });
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(small("Frame"));
            self.yours_row(ui, &knob(dpl_doc::link::LineField::Colour));
        });
        ui.horizontal_wrapped(|ui| {
            if chip(
                ui,
                "frame-none",
                "No frame",
                frame.is_none(),
                "No edge around the picture",
            ) {
                self.set_line(id, dpl_doc::link::LineField::Colour, "none");
            }
            for tone in cl::Tone::ALL {
                let hue = tone.hue_id();
                if swatch_chip(
                    ui,
                    &format!("frame-{hue}"),
                    &pretty(hue),
                    tone.color(),
                    frame.as_deref() == Some(hue),
                    "A hard edge in this colour around the picture",
                ) {
                    self.set_line(id, dpl_doc::link::LineField::Colour, hue);
                }
            }
        });
    }

    fn text_tray_rows(&mut self, ui: &mut Ui, id: &str) {
        let Some(tf) = self.text_fields(id) else {
            return;
        };
        let (text, font, size, ink) = (tf.text, tf.font, tf.size, tf.ink);
        self.block_head(
            ui,
            id,
            if tf.entity == "quote" {
                "Quote"
            } else {
                "Text"
            },
        );

        // The words.
        let mut edited = text.clone();
        let field = ui.add(egui::TextEdit::singleline(&mut edited).desired_width(f32::INFINITY));
        ui.pilot_track(
            &field,
            NodeMetadata::new(
                PilotId::new("editor").child(&PilotKey::new("text")),
                egui_pilot_ai::protocol::Role::TextInput,
                "The words",
            ),
        );
        let field = cl::tip(field, "The words on this line — it saves as you type");
        if field.changed() {
            let key = format!("text-{id}");
            let id = id.to_string();
            self.mutate_burst(&key, |s| set_text(s, &id, |b| b.0 = edited.clone()));
        }
        if field.lost_focus() {
            self.burst = None;
        }

        let line = |field| dpl_doc::link::Knob::Line {
            id: id.to_string(),
            field,
        };
        ui.add_space(cl::space::S2);
        self.line_look_row(ui, id, tf.entity);
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(small("Font"));
            self.yours_row(ui, &line(dpl_doc::link::LineField::Font));
        });
        egui::ScrollArea::horizontal()
            .id_salt("font_tray")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for def in cl::FONTS.iter() {
                        let picked = def.id == font;
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(132.0, 48.0), Sense::click());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(
                            rect,
                            cl::RADIUS_SM,
                            if picked {
                                cl::Tone::Amber.wash()
                            } else {
                                cl::surface::OVERLAY
                            },
                        );
                        painter.text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            def.label,
                            FontId::new(16.0, cl::font_family(def.id)),
                            if picked {
                                cl::ink::STRONG
                            } else {
                                cl::ink::BASE
                            },
                        );
                        card_edge(&painter, rect, resp.hovered());
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                true,
                                picked,
                                def.label,
                            )
                        });
                        ui.pilot_track(
                            &resp,
                            NodeMetadata::new(
                                PilotId::new("editor.font").child(&PilotKey::new(def.id)),
                                egui_pilot_ai::protocol::Role::Button,
                                def.label,
                            ),
                        );
                        let resp = cl::tip(resp, def.blurb);
                        if resp.clicked() {
                            let (id, fid) = (id.to_string(), def.id.to_string());
                            self.mutate(|s| set_text(s, &id, |b| b.1 = fid.clone()));
                        }
                    }
                });
            });

        // A look shows its own knobs (`exposes`); "More controls" opens
        // every knob — full control without leaving the look (LK1). A line
        // look exposes its own (LK3); the card's look only the font.
        let line_look = self
            .slide
            .block(id)
            .and_then(|block| self.styling.looks.line_look(block));
        let exposed: Vec<&str> = match line_look {
            Some(look) => look.exposes.to_vec(),
            None if dpl_doc::card_preset(&self.slide, &self.styling.looks).is_some() => {
                vec!["font"]
            }
            None => vec!["font", "size", "ink", "colour", "motion"],
        };
        let shows = |knob: &str, more: bool| more || exposed.contains(&knob);
        let hidden = ["size", "ink", "colour", "motion"]
            .iter()
            .any(|knob| !exposed.contains(knob));
        if hidden {
            ui.add_space(cl::space::S2);
            let label = if self.more_controls {
                "Fewer controls"
            } else {
                "More controls"
            };
            if chip(
                ui,
                "more-controls",
                label,
                self.more_controls,
                "Show every setting for this line — size, ink, colour and movement too",
            ) {
                self.more_controls = !self.more_controls;
            }
        }
        let more = self.more_controls;

        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            if shows("size", more) {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(small("Size"));
                        self.yours_row(ui, &line(dpl_doc::link::LineField::Size));
                    });
                    ui.horizontal_wrapped(|ui| {
                        for name in SIZES {
                            if chip(
                                ui,
                                &format!("size-{name}"),
                                name,
                                name == size,
                                "How big this line is",
                            ) {
                                let (id, name) = (id.to_string(), name.to_string());
                                self.mutate(|s| set_text(s, &id, |b| b.2 = name.clone()));
                            }
                        }
                    });
                });
                ui.add_space(cl::space::S4);
            }
            if shows("ink", more) {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(small("Ink"));
                        self.yours_row(ui, &line(dpl_doc::link::LineField::Ink));
                    });
                    ui.horizontal(|ui| {
                        for name in INKS {
                            if chip(
                                ui,
                                &format!("ink-{name}"),
                                name,
                                name == ink,
                                "How strong this line's colour is",
                            ) {
                                let (id, name) = (id.to_string(), name.to_string());
                                self.mutate(|s| set_text(s, &id, |b| b.3 = name.clone()));
                            }
                        }
                    });
                });
            }
        });

        // The line's own colour and entrance (LK3).
        ui.add_space(cl::space::S2);
        ui.horizontal_wrapped(|ui| {
            if shows("colour", more) {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(small("Colour"));
                        self.yours_row(ui, &line(dpl_doc::link::LineField::Colour));
                    });
                    ui.horizontal(|ui| {
                        for tone in cl::Tone::ALL {
                            let hue = tone.hue_id();
                            if swatch_chip(
                                ui,
                                &format!("colour-{hue}"),
                                &pretty(hue),
                                tone.color(),
                                tf.tone.as_deref() == Some(hue),
                                "This line's colour — what the tone ink paints",
                            ) {
                                self.set_line(id, dpl_doc::link::LineField::Colour, hue);
                            }
                        }
                    });
                });
                ui.add_space(cl::space::S4);
            }
            if shows("motion", more) {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(small("Movement"));
                        self.yours_row(ui, &line(dpl_doc::link::LineField::Enter));
                    });
                    ui.horizontal(|ui| {
                        for (value, label, tip) in [
                            ("rise", "Rise in", "This line rises in when the slide shows"),
                            (
                                "none",
                                "Stay still",
                                "This line is simply there — no movement",
                            ),
                        ] {
                            if chip(ui, &format!("enter-{value}"), label, tf.enter == value, tip) {
                                self.set_line(id, dpl_doc::link::LineField::Enter, value);
                            }
                        }
                    });
                });
            }
        });
    }

    /// A block tray's head (LK3): its title, the block's chip and menu,
    /// and Keep / Cancel while a look is on trial (else the hint).
    fn block_head(&mut self, ui: &mut Ui, id: &str, title: &str) {
        ui.horizontal_wrapped(|ui| {
            ui.label(tray_title(title));
            ui.add_space(cl::space::S3);
            let chip_rect = self.line_chip(ui, id);
            if self.line_menu {
                self.line_menu_ui(ui, id, chip_rect);
            }
            // Trying a line look: Keep / Cancel sit right by the chip, so the
            // looks row below needs no header of its own (a short window
            // keeps its tiles whole).
            let trying = self
                .trying_line
                .as_ref()
                .filter(|(line, _)| line == id)
                .map(|(_, look)| look.clone());
            match trying {
                Some(look) => {
                    if hero_chip(
                        ui,
                        "line-keep",
                        "Keep",
                        "Use this look here (one Undo takes it back)",
                    ) {
                        self.trying_line = None;
                        self.use_line_look(id, look.as_deref());
                    }
                    if chip(ui, "line-cancel", "Cancel", false, "Go back to how it was") {
                        self.trying_line = None;
                    }
                }
                None => {
                    ui.label(small("Tap a look below to try it on"));
                }
            }
        });
    }

    /// The line's chip (LK3): "Look: Sci-fi headline · 1 change", "Own style
    /// (was …)", "Card style". Tap for the menu.
    fn line_chip(&mut self, ui: &mut Ui, id: &str) -> Rect {
        if let Some((line, look)) = &self.trying_line {
            if line == id {
                let text = format!("Trying: {}", Self::line_look_label(look.as_deref()));
                let resp = chip_response(
                    ui,
                    "line-chip",
                    &text,
                    true,
                    "Keep the line look, or Cancel to go back — nothing is saved until you Keep",
                );
                return resp.rect;
            }
        }
        let state = self.line_state(id);
        let plain = if self.is_picture(id) {
            "Plain picture"
        } else {
            "Card style"
        };
        let text = match &state {
            Some(dpl_doc::link::LinkState::Show { changes: 0 }) | None => plain.to_owned(),
            Some(dpl_doc::link::LinkState::Show { changes }) => match changes {
                1 => format!("{plain} · 1 change"),
                n => format!("{plain} · {n} changes"),
            },
            Some(dpl_doc::link::LinkState::Missing { .. }) => {
                "Line look missing — the card's style".to_owned()
            }
            Some(state) => state.chip(|look| Self::line_look_label(Some(look))),
        };
        let resp = chip_response(
            ui,
            "line-chip",
            &text,
            self.line_menu,
            "What this line follows. Tap to see its changes, give them back, or stop following its look",
        );
        if resp.clicked() {
            self.line_menu = !self.line_menu;
        }
        resp.rect
    }

    /// The line chip's menu: the line's changes (each with its way back),
    /// "Undo all my changes", and stop / follow again.
    fn line_menu_ui(&mut self, ui: &mut Ui, id: &str, under: Rect) {
        let state = self.line_state(id);
        let changes = dpl_doc::link::line_changes(&self.slide, id);
        let area = egui::Area::new(ui.id().with("dpl_editor_line_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(under.left_bottom() + egui::vec2(0.0, cl::space::S1));
        area.show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(cl::surface::RAISED)
                .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                .corner_radius(cl::RADIUS)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.label(tray_title("This line's changes"));
                    if changes.is_empty() {
                        ui.label(small("Nothing yet — everything follows."));
                    }
                    for knob in &changes {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("• {}", knob.label()))
                                    .small()
                                    .color(cl::ink::BASE),
                            );
                            let back = give_back_label(&self.slide, &self.styling.looks, knob);
                            if chip(
                                ui,
                                &format!("line-reset-{}", knob_key(knob)),
                                back,
                                false,
                                "Give this value back — the line follows again here",
                            ) {
                                self.reset(knob);
                            }
                        });
                    }
                    ui.add_space(cl::space::S2);
                    if !changes.is_empty()
                        && chip(
                            ui,
                            "line-reset-all",
                            "Undo all my changes",
                            false,
                            "Give every value of this line back",
                        )
                    {
                        self.reset_line(id);
                    }
                    match &state {
                        Some(dpl_doc::link::LinkState::Follows { .. }) => {
                            if chip(
                                ui,
                                "line-stop",
                                "Stop following this look",
                                false,
                                "Make the line fully yours. Nothing changes on screen",
                            ) {
                                self.stop_following_line(id);
                                self.line_menu = false;
                            }
                        }
                        Some(dpl_doc::link::LinkState::OwnStyle { was: Some(_) }) => {
                            if chip(
                                ui,
                                "line-follow-keep",
                                "Follow the look again — keep my changes",
                                false,
                                "Follow the line look again; what differs stays yours",
                            ) {
                                self.follow_line_again(id, true);
                                self.line_menu = false;
                            }
                            if chip(
                                ui,
                                "line-follow-fresh",
                                "Follow the look again — start fresh",
                                false,
                                "Follow the line look again and give back everything it styles",
                            ) {
                                self.follow_line_again(id, false);
                                self.line_menu = false;
                            }
                        }
                        Some(dpl_doc::link::LinkState::Missing { .. })
                            if chip(
                                ui,
                                "line-none",
                                "Use the card's style",
                                false,
                                "Stop naming the line look that can't be found",
                            ) =>
                        {
                            self.use_line_look(id, None);
                            self.line_menu = false;
                        }
                        _ => {}
                    }
                    if chip(ui, "line-menu-done", "Done", false, "Close this menu") {
                        self.line_menu = false;
                    }
                });
        });
    }

    /// The line looks (LK3): "Card style" first, then the text or quote
    /// looks, each painted as a sample line. Tap to try; Keep or Cancel.
    fn line_look_row(&mut self, ui: &mut Ui, id: &str, entity: &'static str) {
        let current = self
            .slide
            .block(id)
            .and_then(dpl_doc::Block::style)
            .and_then(|style| style.preset)
            .map(str::to_owned);
        let plain = if entity == "picture" {
            "Plain picture"
        } else {
            "Card style"
        };
        let trying = self
            .trying_line
            .as_ref()
            .filter(|(line, _)| line == id)
            .map(|(_, look)| look.clone());
        let tiles: Vec<(Option<&'static dpl_coldlight::BlockPreset>, String)> =
            std::iter::once((None, plain.to_owned()))
                .chain(
                    self.styling
                        .looks
                        .line_looks(entity)
                        .map(|look| (Some(look), look.label.to_owned())),
                )
                .collect();
        egui::ScrollArea::horizontal()
            .id_salt("line_looks")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (look, label) in &tiles {
                        let look_id = look.map(|look| look.id);
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(150.0, 44.0), Sense::click());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, cl::RADIUS_SM, cl::surface::OVERLAY);
                        let accent = look.and_then(|look| look.tone).and_then(cl::hue_accent);
                        let (font, color) = match look {
                            Some(look) if look.entity != "picture" => (
                                look.font.unwrap_or("spacegrotesk"),
                                cl::ink::named(
                                    look.ink.unwrap_or("base"),
                                    accent.unwrap_or(cl::ink::BASE),
                                ),
                            ),
                            Some(_) => ("spacegrotesk", cl::ink::STRONG),
                            None => ("spacegrotesk", cl::ink::MUTED),
                        };
                        // A picture look's sample: a small picture plate,
                        // whole or filling, in its frame.
                        if let Some(look) = look.filter(|look| look.entity == "picture") {
                            let plate = Rect::from_min_size(
                                rect.right_top() + egui::vec2(-44.0, 8.0),
                                egui::vec2(36.0, rect.height() - 16.0),
                            );
                            let inner = if look.fit == Some("contain") {
                                plate.shrink2(egui::vec2(2.0, 8.0))
                            } else {
                                plate
                            };
                            painter.rect_filled(inner, cl::RADIUS_SM, cl::ink::SUBTLE);
                            if let Some(edge) = accent {
                                painter.rect_stroke(
                                    inner,
                                    cl::RADIUS_SM,
                                    Stroke::new(2.0, edge),
                                    StrokeKind::Inside,
                                );
                            }
                        }
                        painter.text(
                            rect.left_center() + egui::vec2(10.0, 0.0),
                            Align2::LEFT_CENTER,
                            label,
                            FontId::new(16.0, cl::font_family(font)),
                            color,
                        );
                        let chosen = current.as_deref() == look_id;
                        let tried = trying.as_ref().is_some_and(|t| t.as_deref() == look_id);
                        card_edge(&painter, rect, resp.hovered() || chosen || tried);
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                true,
                                chosen,
                                label,
                            )
                        });
                        ui.pilot_track(
                            &resp,
                            NodeMetadata::new(
                                PilotId::new("editor.line-look")
                                    .child(&PilotKey::new(look_id.unwrap_or("none"))),
                                egui_pilot_ai::protocol::Role::Button,
                                label,
                            ),
                        );
                        let tip = look.map_or(
                            if entity == "picture" {
                                "No picture look — the picture fills the card, unframed"
                            } else {
                                "No line look — the line takes the card's style"
                            },
                            |look| look.blurb,
                        );
                        let resp = cl::tip(resp, tip);
                        if resp.clicked() && !chosen {
                            self.trying_line = Some((id.to_owned(), look_id.map(str::to_owned)));
                            self.line_menu = false;
                        }
                    }
                });
            });
    }

    /// Beside a control's label: when its value is yours, a dot (colour plus
    /// a ring — never colour alone) and the honest way back ("Use the
    /// look's" or "Use the show's"). Tapping the dot compares the two
    /// values in words. Following values show nothing.
    pub fn yours_row(&mut self, ui: &mut Ui, knob: &dpl_doc::link::Knob) {
        if self.source(knob) != dpl_doc::link::Source::Yours {
            if self.compare.as_ref() == Some(knob) {
                self.compare = None;
            }
            return;
        }
        let (dot, resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::click());
        ui.painter()
            .circle_filled(dot.center(), 4.0, cl::Tone::Amber.color());
        ui.painter()
            .circle_stroke(dot.center(), 7.0, Stroke::new(1.5, cl::Tone::Amber.color()));
        let back = give_back_label(&self.slide, &self.styling.looks, knob);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, back));
        ui.pilot_track(
            &resp,
            NodeMetadata::new(
                PilotId::new("editor.yours").child(&PilotKey::new(knob_key(knob))),
                egui_pilot_ai::protocol::Role::Button,
                "Yours",
            ),
        );
        let resp = cl::tip(resp, "Yours — tap to compare it with what it would follow");
        if resp.clicked() {
            self.compare = match &self.compare {
                Some(open) if open == knob => None,
                _ => Some(knob.clone()),
            };
        }
        if chip(
            ui,
            &format!("back-{}", knob_key(knob)),
            back,
            false,
            "Give this value back — the slide follows again here",
        ) {
            self.compare = None;
            self.reset(knob);
            return;
        }
        if self.compare.as_ref() == Some(knob) {
            // A small sheet under the row: it floats, so a long value never
            // pushes the control's row (or the tab) wider. One line per
            // value, so a name never breaks across lines.
            let (yours, whose, theirs) = self.comparison_parts(knob);
            let words = format!("Yours: {yours} · {whose}: {theirs}");
            egui::Area::new(ui.id().with(("dpl_editor_compare", knob_key(knob))))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(
                    dot.left(),
                    ui.min_rect().bottom() + cl::space::S1,
                ))
                .show(ui.ctx(), |ui| {
                    let sheet = egui::Frame::new()
                        .fill(cl::surface::RAISED)
                        .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                        .corner_radius(cl::RADIUS)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Yours: {yours}"))
                                    .small()
                                    .color(cl::ink::STRONG),
                            );
                            ui.label(
                                RichText::new(format!("{whose}: {theirs}"))
                                    .small()
                                    .color(cl::ink::MUTED),
                            );
                        });
                    ui.pilot_track(
                        &sheet.response,
                        NodeMetadata::new(
                            PilotId::new("editor.compare").child(&PilotKey::new(knob_key(knob))),
                            egui_pilot_ai::protocol::Role::Label,
                            &words,
                        ),
                    );
                });
        }
    }

    /// "Yours: Orchid haze · The look's: Ember drift + Deep lagoon" — the
    /// value the slide holds beside the one it would follow, in words.
    pub fn comparison(&self, knob: &dpl_doc::link::Knob) -> String {
        let (yours, whose, theirs) = self.comparison_parts(knob);
        format!("Yours: {yours} · {whose}: {theirs}")
    }

    /// The comparison's three parts: your value, whose value it would
    /// follow ("The look's" or "The show's"), and that value.
    fn comparison_parts(&self, knob: &dpl_doc::link::Knob) -> (String, &'static str, String) {
        let mut given_back = self.slide.clone();
        dpl_doc::link::reset(&mut given_back, knob);
        let whose = match dpl_doc::link::source(&given_back, &self.styling.looks, knob) {
            dpl_doc::link::Source::Look(_) => "The look's",
            _ => "The show's",
        };
        (
            value_words(&self.slide, &self.styling, knob),
            whose,
            value_words(&given_back, &self.styling, knob),
        )
    }

    // ── mutation helpers ────────────────────────────────────────────────

    /// The background the slide shows: its own, else its look's, else the
    /// show's — the cascade's answer, preset tier included.
    fn inherited_background(&self) -> dpl_doc::Background {
        self.styling
            .resolve(&self.slide)
            .background
            .unwrap_or_else(|| self.styling.theme.background.clone())
    }

    /// A block's fields RESOLVED through the theme, plus whether font/ink
    /// are slide-level overrides (vs inherited from the show).
    fn text_fields(&self, id: &str) -> Option<TextFields> {
        let shown = document::shown(&self.slide, &self.styling);
        let block = shown.block(id)?;
        let entity = match block {
            dpl_doc::Block::Quote { .. } => "quote",
            _ => "text",
        };
        let words = block.words()?;
        Some(TextFields {
            text: words.text.to_owned(),
            font: words
                .font
                .map_or_else(|| self.styling.theme.font.clone(), str::to_owned),
            size: words.size.unwrap_or(DEFAULT_SIZE).to_owned(),
            ink: words
                .ink
                .map_or_else(|| self.styling.theme.ink.clone(), str::to_owned),
            tone: words.tone.map(str::to_owned),
            enter: words.enter.unwrap_or("none").to_owned(),
            entity,
        })
    }

    fn set_blend_mood(&mut self, slot: usize, pick: String) {
        // Editing an INHERITED background starts from the theme's values —
        // the first touch materializes an override with the show as base.
        let base = self.inherited_background();
        self.mutate(move |s| {
            let (mut a, mut b, motion, ambience) = match &base {
                dpl_doc::Background::MoodBlend {
                    moods,
                    motion,
                    ambience,
                } => (
                    moods.first().cloned().unwrap_or_default(),
                    moods.get(1).cloned().unwrap_or_default(),
                    motion.clone(),
                    ambience.clone(),
                ),
                dpl_doc::Background::Mood { mood } => {
                    (mood.clone(), mood.clone(), "blend-drift".into(), None)
                }
                dpl_doc::Background::None => {
                    (pick.clone(), pick.clone(), "blend-drift".into(), None)
                }
            };
            if slot == 0 {
                a = pick.clone();
            } else {
                b = pick.clone();
            }
            s.background = Some(dpl_doc::Background::MoodBlend {
                moods: vec![a, b],
                motion,
                ambience,
            });
        });
    }

    fn set_ambience(&mut self, amb: Option<String>) {
        let base = self.inherited_background();
        self.mutate(move |s| {
            let mut bg = base;
            if let dpl_doc::Background::MoodBlend { ambience, .. } = &mut bg {
                *ambience = amb;
            }
            s.background = Some(bg);
        });
    }
}

// ── free helpers ────────────────────────────────────────────────────────

/// Use a card look (LS2b): the slide stores the preset's REFERENCE, never
/// a copy of its values (the July ruling, "not bake"). Picking a look
/// adopts it: the overrides it governs — the background, the colour and
/// each line's font — are cleared, so the look shows (Keynote's "change
/// theme" without keeping style changes). Size and ink stay the slide's.
fn use_look(s: &mut dpl_doc::Slide, id: &str) {
    s.preset = Some(id.to_owned());
    s.background = None;
    s.tone = None;
    for block in &mut s.blocks {
        if let Some(words) = block.words_mut() {
            *words.font = None;
        }
    }
}

/// Edit a text block's (text, font, size, ink) tuple in place. Font/ink
/// become overrides ONLY when the closure actually changes them — an
/// untouched inherited field stays absent on disk (sparse stays sparse).
fn set_text(
    s: &mut dpl_doc::Slide,
    id: &str,
    f: impl FnOnce(&mut (String, String, String, String)),
) {
    for b in &mut s.blocks {
        let Some(words) = b.words_mut() else {
            continue;
        };
        if words.id == id {
            let before = (
                words.text.clone(),
                words.font.clone().unwrap_or_default(),
                words
                    .size
                    .clone()
                    .unwrap_or_else(|| DEFAULT_SIZE.to_owned()),
                words.ink.clone().unwrap_or_default(),
            );
            let mut t = before.clone();
            f(&mut t);
            *words.text = t.0;
            if t.1 != before.1 {
                *words.font = Some(t.1);
            }
            if t.2 != before.2 {
                *words.size = Some(t.2);
            }
            if t.3 != before.3 {
                *words.ink = Some(t.3);
            }
            return;
        }
    }
}

fn mood(name: &str) -> Option<&'static cl::Mood> {
    cl::MOODS.iter().find(|m| m.name == name)
}

/// A horizontal row of mood gradient cards; returns a tapped mood name.
fn mood_row(ui: &mut Ui, salt: &str, current: Option<&str>) -> Option<String> {
    let mut picked = None;
    egui::ScrollArea::horizontal()
        .id_salt(salt)
        .auto_shrink([false, false])
        .max_width(360.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for m in cl::MOODS.iter() {
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(72.0, 44.0), Sense::click());
                    let painter = ui.painter_at(rect);
                    cl::mood_gradient(&painter, rect, m);
                    let on = current == Some(m.name);
                    card_edge_tone(&painter, rect, on || resp.hovered(), on);
                    resp.clone().on_hover_text(m.name);
                    if resp.clicked() {
                        picked = Some(m.name.to_string());
                    }
                }
            });
        });
    picked
}

fn fit_stage(area: Rect) -> Rect {
    // Largest 16:9 rect centered in `area`.
    let (aw, ah) = (area.width(), area.height());
    let (w, h) = if aw / ah > 16.0 / 9.0 {
        (ah * 16.0 / 9.0, ah)
    } else {
        (aw, aw * 9.0 / 16.0)
    };
    Rect::from_center_size(area.center(), egui::vec2(w, h))
}

fn tray_title(t: &str) -> RichText {
    RichText::new(t).strong().color(cl::ink::STRONG)
}
fn small(t: &str) -> RichText {
    RichText::new(t).small().color(cl::ink::SUBTLE)
}

fn card_edge(painter: &egui::Painter, rect: Rect, hot: bool) {
    painter.rect_stroke(
        rect,
        cl::RADIUS_SM,
        Stroke::new(
            1.0,
            if hot {
                cl::ink::MUTED
            } else {
                cl::surface::HAIRLINE
            },
        ),
        StrokeKind::Inside,
    );
}
fn card_edge_tone(painter: &egui::Painter, rect: Rect, hot: bool, on: bool) {
    let c = if on {
        cl::Tone::Amber.color()
    } else if hot {
        cl::ink::MUTED
    } else {
        cl::surface::HAIRLINE
    };
    painter.rect(
        rect,
        cl::RADIUS_SM,
        Color32::TRANSPARENT,
        Stroke::new(if on { 2.0 } else { 1.0 }, c),
        StrokeKind::Inside,
    );
}

/// A chip with a colour dot before its name — the Colour row.
fn swatch_chip(ui: &mut Ui, key: &str, label: &str, colour: Color32, on: bool, tip: &str) -> bool {
    let resp = chip_response(ui, key, &format!("   {label}"), on, tip);
    ui.painter()
        .circle_filled(resp.rect.left_center() + egui::vec2(12.0, 0.0), 5.0, colour);
    resp.clicked()
}

/// A look's backdrop, painted as its tile: a blend, one colour, or the
/// bare well.
fn paint_backdrop(painter: &egui::Painter, rect: Rect, background: &dpl_doc::Background) {
    match background {
        dpl_doc::Background::MoodBlend { moods, .. } => {
            let at = |i: usize| moods.get(i).and_then(|name| mood(name));
            if let (Some(a), Some(b)) = (at(0), at(1)) {
                cl::mood_blend(painter, rect, a, b, 0.45, 0.0);
            }
        }
        dpl_doc::Background::Mood { mood: name } => {
            if let Some(one) = mood(name) {
                cl::mood_gradient(painter, rect, one);
            }
        }
        dpl_doc::Background::None => {
            painter.rect_filled(rect, cl::RADIUS_SM, cl::surface::WELL);
        }
    }
}

/// Do two paths name the same folder (however they were spelled)?
fn same_dir(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}

/// The honest way back for a value that is yours.
fn give_back_label(
    slide: &dpl_doc::Slide,
    looks: &dpl_doc::Looks,
    knob: &dpl_doc::link::Knob,
) -> &'static str {
    let mut probe = slide.clone();
    dpl_doc::link::reset(&mut probe, knob);
    match dpl_doc::link::source(&probe, looks, knob) {
        dpl_doc::link::Source::Look(_) => "Use the look's",
        _ => "Use the show's",
    }
}

/// What `knob` shows on `slide`, in plain words (through the cascade).
fn value_words(
    slide: &dpl_doc::Slide,
    styling: &dpl_doc::Styling,
    knob: &dpl_doc::link::Knob,
) -> String {
    use dpl_doc::link::{Knob, LineField};
    let shown = styling.resolve(slide);
    let or_none = |v: Option<&str>| v.map_or_else(|| "None".to_owned(), pretty);
    match knob {
        Knob::Background => match shown.background {
            Some(dpl_doc::Background::Mood { mood }) => pretty(&mood),
            Some(dpl_doc::Background::MoodBlend { moods, .. }) => moods
                .iter()
                .map(|m| pretty(m))
                .collect::<Vec<_>>()
                .join(" + "),
            Some(dpl_doc::Background::None) | None => "Nothing".into(),
        },
        Knob::Colour => or_none(shown.tone.as_deref()),
        Knob::Enter => or_none(shown.enter.as_deref()),
        Knob::Line { id, field } => {
            let Some(block) = shown.block(id) else {
                return "None".into();
            };
            let words = block.words();
            match field {
                LineField::Font => words.and_then(|w| w.font).map_or_else(
                    || "None".into(),
                    |font| {
                        cl::FONTS
                            .iter()
                            .find(|def| def.id == font)
                            .map_or_else(|| pretty(font), |def| def.label.to_owned())
                    },
                ),
                LineField::Size => pretty(words.and_then(|w| w.size).unwrap_or(DEFAULT_SIZE)),
                LineField::Ink => or_none(words.and_then(|w| w.ink)),
                LineField::Enter => match block.style().and_then(|s| s.enter) {
                    Some("rise") => "Rises in".into(),
                    Some(_) => "Stays still".into(),
                    None => "None".into(),
                },
                LineField::Fit => match block.fit() {
                    Some("contain") => "Shows all of it".into(),
                    _ => "Fills the card".into(),
                },
                LineField::Colour => match block {
                    dpl_doc::Block::Text { tone, .. }
                    | dpl_doc::Block::Media { tone, .. }
                    | dpl_doc::Block::Quote { tone, .. } => {
                        tone.as_deref().map_or_else(|| "The card's".into(), pretty)
                    }
                    dpl_doc::Block::Data { .. } => "None".into(),
                },
            }
        }
    }
}

/// A registry id as a person would say it: `orchid-haze` → "Orchid haze".
fn pretty(id: &str) -> String {
    let mut out = id.replace(['-', '_'], " ");
    if let Some(first) = out.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    out
}

/// A knob's stable key for pilot ids ("background", "line-title-font").
fn knob_key(knob: &dpl_doc::link::Knob) -> String {
    match knob {
        dpl_doc::link::Knob::Background => "background".into(),
        dpl_doc::link::Knob::Colour => "colour".into(),
        dpl_doc::link::Knob::Enter => "enter".into(),
        dpl_doc::link::Knob::Line { id, field } => format!("line-{id}-{field:?}").to_lowercase(),
    }
}

/// A chip: plain label, tooltip, pilot id `editor/<key>`. True when tapped.
fn chip(ui: &mut Ui, key: &str, label: &str, on: bool, tip: &str) -> bool {
    chip_response(ui, key, label, on, tip).clicked()
}

fn chip_response(ui: &mut Ui, key: &str, label: &str, on: bool, tip: &str) -> egui::Response {
    chip_drawn(ui, key, label, on, false, tip)
}

/// The primary chip of a row (Keep): the house Hero weight — a 2px amber
/// edge and amber words — so the one action that saves stands out.
fn hero_chip(ui: &mut Ui, key: &str, label: &str, tip: &str) -> bool {
    chip_drawn(ui, key, label, false, true, tip).clicked()
}

fn chip_drawn(
    ui: &mut Ui,
    key: &str,
    label: &str,
    on: bool,
    hero: bool,
    tip: &str,
) -> egui::Response {
    let galley =
        ui.painter()
            .layout_no_wrap(label.into(), FontId::proportional(13.0), cl::ink::BASE);
    let w = galley.size().x + 22.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 30.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        cl::RADIUS_SM,
        if on || hero {
            cl::Tone::Amber.wash()
        } else {
            cl::surface::OVERLAY
        },
    );
    if hero {
        painter.rect_stroke(
            rect,
            cl::RADIUS_SM,
            Stroke::new(2.0, cl::Tone::Amber.color()),
            egui::StrokeKind::Inside,
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        if hero {
            cl::Tone::Amber.color()
        } else if on {
            cl::ink::STRONG
        } else {
            cl::ink::BASE
        },
    );
    ui.add_space(cl::space::S1);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, label));
    ui.pilot_track(
        &resp,
        NodeMetadata::new(
            PilotId::new("editor").child(&PilotKey::new(key)),
            egui_pilot_ai::protocol::Role::Button,
            label,
        ),
    );
    cl::tip(resp, tip)
}

/// A big, obvious button (Undo). `enabled=false` dims + ignores taps.
fn big_button(ui: &mut Ui, key: &str, label: &str, enabled: bool, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(84.0, 30.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(cl::RADIUS_SM),
        if enabled && resp.hovered() {
            cl::surface::OVERLAY
        } else {
            cl::surface::RAISED
        },
    );
    painter.rect_stroke(
        rect,
        cl::RADIUS_SM,
        Stroke::new(1.0, cl::surface::HAIRLINE),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.5),
        if enabled {
            cl::ink::BASE
        } else {
            cl::ink::SUBTLE
        },
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    ui.pilot_track(
        &resp,
        NodeMetadata::new(
            PilotId::new("editor").child(&PilotKey::new(key)),
            egui_pilot_ai::protocol::Role::Button,
            label,
        ),
    );
    let resp = cl::tip(resp, tip);
    enabled && resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fully specified — the pre-W3b shape; overrides everywhere.
    fn seed() -> dpl_doc::Slide {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.background = Some(dpl_doc::Background::MoodBlend {
            moods: vec!["ember-drift".into(), "deep-lagoon".into()],
            motion: "blend-drift".into(),
            ambience: None,
        });
        slide.blocks = vec![dpl_doc::Block::Text {
            id: "title".into(),
            preset: None,
            tone: None,
            text: "Hello".into(),
            font: Some("spacegrotesk".into()),
            size: Some("display".into()),
            ink: Some("strong".into()),
            enter: None,
            unlinked_from: None,
        }];
        slide.enter = Some("rise".into());
        slide
    }

    /// Styling-free — everything inherits the theme.
    fn sparse_seed() -> dpl_doc::Slide {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.blocks = vec![dpl_doc::Block::text("title", "Hello")];
        slide
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join("dpl-editor-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Three lines, all on the card at the start.
    fn three_lines() -> dpl_doc::Slide {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.blocks = vec![
            dpl_doc::Block::text("eyebrow", "Part one"),
            dpl_doc::Block::text("title", "Hello"),
            dpl_doc::Block::text("subtitle", "Under it"),
        ];
        slide
    }

    /// RL: putting lines on taps writes `reveals` in tap order, one
    /// revision each, and Undo takes a tap back.
    #[test]
    fn lines_go_on_taps_and_come_back() {
        let dir = scratch("taps");
        dpl_doc::save(&dir, &three_lines()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.set_tap("subtitle", Some(1));
        assert_eq!(ed.slide().reveals, ["subtitle"]);
        ed.set_tap("title", Some(1));
        assert_eq!(
            ed.slide().reveals,
            ["title", "subtitle"],
            "tap 1 goes first"
        );
        ed.set_tap("subtitle", Some(3));
        assert_eq!(
            ed.slide().reveals,
            ["title", "subtitle"],
            "a tap past the end is the last"
        );
        ed.set_tap("title", None);
        assert_eq!(ed.slide().reveals, ["subtitle"]);
        assert_eq!(dpl_doc::load(&dir).unwrap().reveals, ["subtitle"], "saved");
        ed.undo();
        assert_eq!(ed.slide().reveals, ["title", "subtitle"]);
        assert_eq!(ed.slide().step_count(), 3);
    }

    /// RL: taps alone write no script; once words are written the script
    /// follows the taps, keeping each line's words with its tap.
    #[test]
    fn the_script_follows_the_taps_once_it_has_words() {
        let dir = scratch("script");
        dpl_doc::save(&dir, &three_lines()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.set_tap("subtitle", Some(1));
        assert!(
            dpl_doc::script::load_script(&dir).unwrap().is_none(),
            "no words, no file"
        );
        let mut script = dpl_doc::script::follow_reveals(None, ed.slide(), &ed.slide().label());
        script.beats[1].cue = "And then this".into();
        ed.write_script(script);
        ed.set_tap("eyebrow", Some(1));
        let on_disk = dpl_doc::script::load_script(&dir).unwrap().unwrap();
        let beats: Vec<(&str, &str)> = on_disk
            .beats
            .iter()
            .map(|b| (b.id.as_str(), b.cue.as_str()))
            .collect();
        assert_eq!(
            beats,
            [
                ("setup", ""),
                ("eyebrow", ""),
                ("subtitle", "And then this")
            ]
        );
        assert!(dpl_doc::script::parity_issues(ed.slide(), &on_disk)
            .iter()
            .all(|i| !i.message.contains("step semantics")));
    }

    /// The panel draws every line, its taps, the words and the preview at a
    /// real size without panicking — side by side and stacked.
    #[test]
    fn the_on_tap_panel_draws() {
        let dir = scratch("panel");
        let mut slide = three_lines();
        slide.reveals = vec!["subtitle".into()];
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.preview = 1;
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        // Wide (side by side) and narrow (stacked).
        for width in [1280.0, 420.0] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 800.0),
                )),
                ..Default::default()
            };
            ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| ed.taps_ui(ui));
            })
            .textures_delta
            .clear();
        }
        assert_eq!(ed.preview_step(), 1);
    }

    #[test]
    fn a_font_edit_changes_exactly_one_ref_on_disk() {
        let dir = scratch("font");
        dpl_doc::save(&dir, &seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));

        let on_disk = dpl_doc::load(&dir).unwrap();
        let dpl_doc::Block::Text {
            font,
            text,
            size,
            ink,
            ..
        } = &on_disk.blocks[0]
        else {
            panic!("block 0 is text");
        };
        assert_eq!(font.as_deref(), Some("orbitron"), "font ref swapped");
        // Nothing else moved.
        assert_eq!(text, "Hello");
        assert_eq!(size.as_deref(), Some("display"));
        assert_eq!(ink.as_deref(), Some("strong"));
        assert_eq!(on_disk.background, seed().background);
        // The edit still validates against the real registries.
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    /// Every saved change is a new revision, and undo restores the earlier
    /// CONTENT as a further revision — history never runs backwards.
    #[test]
    fn undo_restores_the_content_as_a_new_revision() {
        let dir = scratch("undo");
        let original = seed();
        dpl_doc::save(&dir, &original).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));
        assert_eq!(dpl_doc::load(&dir).unwrap().rev, 2);
        // A change that changes nothing is not a revision.
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));
        assert_eq!(dpl_doc::load(&dir).unwrap().rev, 2);
        ed.undo();
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.rev, 3);
        let mut expected = original;
        expected.rev = 3;
        assert_eq!(on_disk, expected);
    }

    /// A quote is a real block (ruling #3) with the quote component's own
    /// name and role slots, and the card fills the quote component.
    #[test]
    fn the_quote_tab_writes_a_quote_block_and_its_slots() {
        let dir = scratch("quote");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.set_quote(["Ship it.".into(), "Grace".into(), String::new()]);
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.component, "quote@1");
        assert!(
            matches!(on_disk.block("quote"), Some(dpl_doc::Block::Quote { text, .. }) if text == "Ship it.")
        );
        assert!(matches!(
            on_disk.block("name"),
            Some(dpl_doc::Block::Text { .. })
        ));
        assert!(
            on_disk.block("role").is_none(),
            "a blank line writes no block"
        );
        assert_eq!(
            ed.quote(),
            ["Ship it.".to_string(), "Grace".into(), String::new()]
        );
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    /// The picture fills the component's `media` slot.
    #[test]
    fn a_picture_fills_the_media_slot() {
        let dir = scratch("picture");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.set_media(Some(("inbox/x.png".into(), "image".into(), "cover".into())));
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert!(matches!(
            on_disk.block("media"),
            Some(dpl_doc::Block::Media { .. })
        ));
    }

    /// Picking a look stores the REFERENCE (LS2b, the July ruling "not
    /// bake"): the file names the preset and holds none of its values, and
    /// the slide resolves to the look's background and font.
    #[test]
    fn picking_a_look_stores_the_reference() {
        let dir = scratch("theme");
        dpl_doc::save(&dir, &seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        let p = cl::THEME_PRESETS[0];
        ed.mutate(|s| use_look(s, p.id));
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.preset.as_deref(), Some(p.id));
        assert_eq!(
            on_disk.background, None,
            "the look's background is not copied"
        );
        let dpl_doc::Block::Text { font, .. } = &on_disk.blocks[0] else {
            panic!("block 0 is text");
        };
        assert_eq!(*font, None, "the look's font is not copied");
        let resolved = dpl_doc::Styling::standalone().resolve(&on_disk);
        let dpl_doc::Block::Text { font, .. } = &resolved.blocks[0] else {
            panic!("block 0 is text");
        };
        assert_eq!(font.as_deref(), Some(p.font));
        if let Some(dpl_doc::Background::MoodBlend { moods, .. }) = &resolved.background {
            assert_eq!(moods, &vec![p.moods.0.to_string(), p.moods.1.to_string()]);
        } else {
            panic!("the look gives a mood-blend background");
        }
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    #[test]
    fn setting_a_blend_mood_slot_keeps_the_other_and_validates() {
        let dir = scratch("mood");
        dpl_doc::save(&dir, &seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.set_blend_mood(1, "jade-current".into());
        let on_disk = dpl_doc::load(&dir).unwrap();
        if let Some(dpl_doc::Background::MoodBlend { moods, .. }) = &on_disk.background {
            assert_eq!(moods[0], "ember-drift");
            assert_eq!(moods[1], "jade-current");
        } else {
            panic!("still a blend");
        }
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    // ── the cascade in the editor (W3b) ─────────────────────────────────

    #[test]
    fn editing_text_on_a_sparse_slide_does_not_densify_styling() {
        let dir = scratch("sparse-text");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.mutate(|s| set_text(s, "title", |b| b.0 = "New words".into()));
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(raw.contains("New words"));
        for absent in ["\"font\"", "\"ink\"", "\"background\""] {
            assert!(!raw.contains(absent), "{absent} crept in: {raw}");
        }
    }

    #[test]
    fn giving_a_font_back_clears_it_on_disk() {
        let dir = scratch("show-theme");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        // Make the font yours, then give it back.
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(raw.contains("\"font\": \"orbitron\""), "yours, on disk");
        let knob = dpl_doc::link::Knob::Line {
            id: "title".into(),
            field: dpl_doc::link::LineField::Font,
        };
        assert_eq!(ed.source(&knob), dpl_doc::link::Source::Yours);
        ed.reset(&knob);
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(!raw.contains("\"font\""), "given back on disk: {raw}");
        // The tray resolves back to the show's font.
        let tf = ed.text_fields("title").unwrap();
        assert_eq!(tf.font, dpl_doc::Theme::fallback().font);
        assert_eq!(ed.source(&knob), dpl_doc::link::Source::Show);
    }

    /// The dot's comparison names the value that is yours and the one the
    /// slide would follow — the look's while it follows one.
    #[test]
    fn the_comparison_names_yours_and_the_looks() {
        let dir = scratch("compare");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        let look = cl::THEME_PRESETS
            .iter()
            .find(|p| p.id == "neon-intro")
            .unwrap();
        ed.mutate(|s| use_look(s, look.id));
        ed.mutate(|s| {
            s.background = Some(dpl_doc::Background::Mood {
                mood: "orchid-haze".into(),
            });
        });
        assert_eq!(
            ed.comparison(&dpl_doc::link::Knob::Background),
            format!(
                "Yours: Orchid haze · The look's: {} + {}",
                pretty(look.moods.0),
                pretty(look.moods.1)
            )
        );
        let font = dpl_doc::link::Knob::Line {
            id: "title".into(),
            field: dpl_doc::link::LineField::Font,
        };
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "vt323".into()));
        assert!(
            ed.comparison(&font).ends_with(&format!(
                "The look's: {}",
                cl::FONTS.iter().find(|d| d.id == look.font).unwrap().label
            )),
            "{}",
            ed.comparison(&font)
        );
    }

    /// Typing is one revision and one undo step, not one per letter.
    #[test]
    fn a_typing_burst_is_one_revision() {
        let dir = scratch("burst");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        for word in ["H", "He", "Hel", "Hell", "Hello!"] {
            let word = word.to_string();
            ed.mutate_burst("text-title", |s| {
                set_text(s, "title", |b| b.0 = word.clone())
            });
        }
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.rev, 2, "one revision for the burst");
        assert!(
            matches!(on_disk.block("title"), Some(dpl_doc::Block::Text { text, .. }) if text == "Hello!")
        );
        ed.undo();
        assert!(!ed.can_undo(), "one undo step took the whole burst back");
        assert!(
            matches!(dpl_doc::load(&dir).unwrap().block("title"), Some(dpl_doc::Block::Text { text, .. }) if text == "Hello")
        );
    }

    /// Stopping following changes nothing on screen; following again with
    /// a fresh start gives the look's values back (LK1).
    #[test]
    fn stop_and_follow_again_through_the_editor() {
        let dir = scratch("link");
        let mut slide = sparse_seed();
        slide.preset = Some(cl::THEME_PRESETS[0].id.into());
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        let styling = dpl_doc::Styling::standalone();
        let before = styling.resolve(ed.slide());
        ed.stop_following();
        assert!(matches!(
            ed.link_state(),
            dpl_doc::link::LinkState::OwnStyle { .. }
        ));
        assert_eq!(styling.resolve(ed.slide()).blocks, before.blocks);
        ed.follow_again(false);
        assert!(matches!(
            ed.link_state(),
            dpl_doc::link::LinkState::Follows { changes: 0, .. }
        ));
        assert!(dpl_doc::validate(&dpl_doc::load(&dir).unwrap()).is_empty());
    }

    /// "Save as a new look…" (LK2): the look lands in the library, the
    /// slide follows it with no changes, and a fresh editor finds it.
    #[test]
    fn saving_a_new_look_lands_in_the_library_and_the_slide_follows_it() {
        let dir = scratch("new-look");
        let content = scratch("new-look-content");
        let mut slide = sparse_seed();
        slide.preset = Some("neon-intro".into());
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir, &content).unwrap();
        ed.set_background(Some(dpl_doc::Background::Mood {
            mood: "orchid-haze".into(),
        }));
        assert!(matches!(
            ed.link_state(),
            dpl_doc::link::LinkState::Follows { changes: 1, .. }
        ));
        let id = ed.save_as_new_look("Orchid intro").unwrap();
        assert!(matches!(
            ed.link_state(),
            dpl_doc::link::LinkState::Follows { changes: 0, ref look } if *look == id
        ));
        let on_disk = dpl_doc::look::load_look(&dpl_doc::look::look_dir(&content, &id)).unwrap();
        assert_eq!(on_disk.label, "Orchid intro");
        assert_eq!(
            on_disk.background,
            dpl_doc::Background::Mood {
                mood: "orchid-haze".into()
            }
        );
        assert_eq!(
            dpl_doc::load(&dir).unwrap().preset.as_deref(),
            Some(id.as_str())
        );
        let again = Editor::open(&dir, &content).unwrap();
        assert!(again.looks().is_yours(&id));
        assert_eq!(again.look_label(&id), "Orchid intro");
        // One Undo takes the slide back to the look it followed.
        ed.undo();
        assert_eq!(ed.slide().preset.as_deref(), Some("neon-intro"));
    }

    /// Pushing a change back to a built-in makes your version; one Undo
    /// steps it aside and gives the slide its change back; Back to the
    /// built-in keeps your version on disk (LK2b).
    #[test]
    fn pushing_back_undo_and_back_to_the_builtin() {
        let content = scratch("spread-content");
        let mut slide = sparse_seed();
        slide.preset = Some("neon-intro".into());
        let dir = dpl_doc::library_slide_dir(&content, &slide.id);
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir, &content).unwrap();
        let orchid = dpl_doc::Background::Mood {
            mood: "orchid-haze".into(),
        };
        ed.set_background(Some(orchid.clone()));
        ed.push_back().unwrap();
        assert!(ed.looks().replaces_builtin("neon-intro"));
        assert!(matches!(
            ed.link_state(),
            dpl_doc::link::LinkState::Follows { changes: 0, .. }
        ));
        let look_file = dpl_doc::look::look_dir(&content, "neon-intro");
        assert_eq!(
            dpl_doc::look::load_look(&look_file).unwrap().background,
            orchid
        );

        // One Undo: the built-in shows again and the change is the slide's.
        ed.undo();
        assert!(!ed.looks().is_yours("neon-intro"));
        assert!(!look_file.join("look.json").exists());
        assert_eq!(ed.slide().background, Some(orchid.clone()));

        // Push again, then Back to the built-in: your version is kept.
        ed.push_back().unwrap();
        ed.back_to_builtin().unwrap();
        assert!(!ed.looks().is_yours("neon-intro"));
        let kept = std::fs::read_dir(look_file.join("replaced"))
            .unwrap()
            .count();
        assert_eq!(kept, 2, "both stepped-aside versions are kept");
        // And Undo brings your version back.
        ed.undo();
        assert!(ed.looks().replaces_builtin("neon-intro"));
    }

    /// Editing the look: a colour change lands in the look, not the slide.
    #[test]
    fn editing_the_look_changes_the_look_not_the_slide() {
        let content = scratch("open-look-content");
        let mut slide = sparse_seed();
        slide.preset = Some("neon-intro".into());
        let dir = dpl_doc::library_slide_dir(&content, &slide.id);
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir, &content).unwrap();
        ed.open_look();
        assert!(ed.is_editing_look());
        ed.set_background(Some(dpl_doc::Background::Mood {
            mood: "jade-current".into(),
        }));
        assert_eq!(ed.slide().background, None, "the slide keeps following");
        assert!(matches!(
            ed.looks().get("neon-intro").map(|look| &look.background),
            Some(dpl_doc::Background::Mood { mood }) if mood == "jade-current"
        ));
    }

    /// A line's own size, colour and movement are changes from its line
    /// look (LK3) — the rows below the tray's fold, which the real-exe
    /// scenario cannot scroll to — and giving them back shows the look's.
    #[test]
    fn a_lines_own_size_colour_and_movement_are_changes_from_its_look() {
        let dir = scratch("line-knobs");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        ed.use_line_look("title", Some("neon-word"));
        assert_eq!(
            ed.line_state("title"),
            Some(dpl_doc::link::LinkState::Follows {
                look: "neon-word".into(),
                changes: 0
            })
        );
        let tf = ed.text_fields("title").unwrap();
        assert_eq!(
            (
                tf.font.as_str(),
                tf.size.as_str(),
                tf.tone.as_deref(),
                tf.enter.as_str()
            ),
            ("monoton", "display", Some("magenta"), "rise"),
            "the line shows its look"
        );
        ed.mutate(|s| set_text(s, "title", |b| b.2 = "h1".into()));
        ed.set_line("title", dpl_doc::link::LineField::Colour, "lime");
        ed.set_line("title", dpl_doc::link::LineField::Enter, "none");
        assert!(matches!(
            ed.line_state("title"),
            Some(dpl_doc::link::LinkState::Follows { changes: 3, .. })
        ));
        let colour = dpl_doc::link::Knob::Line {
            id: "title".into(),
            field: dpl_doc::link::LineField::Colour,
        };
        assert_eq!(ed.source(&colour), dpl_doc::link::Source::Yours);
        assert_eq!(ed.comparison(&colour), "Yours: Lime · The look's: Magenta");
        ed.reset_line("title");
        let tf = ed.text_fields("title").unwrap();
        assert_eq!(
            (tf.size.as_str(), tf.tone.as_deref(), tf.enter.as_str()),
            ("display", Some("magenta"), "rise")
        );
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(raw.contains("\"preset\": \"neon-word\""), "{raw}");
        assert!(
            !raw.contains("\"enter\"") && !raw.contains("\"tone\""),
            "{raw}"
        );
    }

    /// A picture's look, fit, frame and movement through the editor (LK3b):
    /// the look gives them; a fit and "No frame" of its own are changes; a
    /// different picture keeps the look; giving back shows the look's again.
    #[test]
    fn a_pictures_look_fit_frame_and_movement() {
        let dir = scratch("picture-look");
        let mut slide = sparse_seed();
        slide
            .blocks
            .insert(0, dpl_doc::Block::media(MEDIA_SLOT, "image", "inbox/x.png"));
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir, &dir).unwrap();
        assert!(ed.is_picture(MEDIA_SLOT));
        ed.use_line_look(MEDIA_SLOT, Some("evidence"));
        assert_eq!(
            ed.picture_fields(MEDIA_SLOT),
            Some(("contain".into(), Some("amber".into()), "rise".into()))
        );
        ed.set_line(MEDIA_SLOT, dpl_doc::link::LineField::Fit, "cover");
        ed.set_line(MEDIA_SLOT, dpl_doc::link::LineField::Colour, "none");
        assert_eq!(
            ed.picture_fields(MEDIA_SLOT),
            Some(("cover".into(), None, "rise".into()))
        );
        assert!(matches!(
            ed.line_state(MEDIA_SLOT),
            Some(dpl_doc::link::LinkState::Follows { changes: 2, .. })
        ));
        // A different picture keeps the picture's look.
        ed.set_media(Some(("inbox/y.png".into(), "image".into(), "cover".into())));
        let picture = ed.slide().block(MEDIA_SLOT).unwrap().clone();
        assert_eq!(picture.style().unwrap().preset, Some("evidence"));
        ed.reset_line(MEDIA_SLOT);
        assert_eq!(
            ed.picture_fields(MEDIA_SLOT),
            Some(("contain".into(), Some("amber".into()), "rise".into()))
        );
        assert!(dpl_doc::validate(&dpl_doc::load(&dir).unwrap()).is_empty());
    }

    /// The spread sheet opens and paints again after an Undo put the
    /// built-in back (the real-exe run stalled exactly here).
    #[test]
    fn the_spread_sheet_paints_after_an_undo() {
        let content = scratch("spread-undo-content");
        let mut other = sparse_seed();
        other.preset = Some("neon-intro".into());
        dpl_doc::save(&dpl_doc::library_slide_dir(&content, &other.id), &other).unwrap();
        let mut slide = sparse_seed();
        slide.preset = Some("neon-intro".into());
        let dir = dpl_doc::library_slide_dir(&content, &slide.id);
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir, &content).unwrap();
        ed.set_background(Some(dpl_doc::Background::Mood {
            mood: "orchid-haze".into(),
        }));
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let frame = |ed: &mut Editor| {
            ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| ed.ui(ui));
            })
            .textures_delta
            .clear();
        };
        ed.open_spread(SpreadKind::PushBack);
        assert_eq!(ed.spread.as_ref().map(|s| s.others), Some(1));
        frame(&mut ed);
        ed.push_back().unwrap();
        ed.spread = None;
        ed.undo();
        ed.open_spread(SpreadKind::PushBack);
        assert_eq!(ed.spread.as_ref().map(|s| s.others), Some(1));
        frame(&mut ed);
        frame(&mut ed);
    }
}
