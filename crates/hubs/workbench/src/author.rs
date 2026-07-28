//! Authoring surface — charter passes A + B + C (the red-pen-ready walkable
//! shell).
//!
//! Pass A gave the SIGNAL SPINE (additive `BuildEvent::CardAuthored`, published
//! after a real draft-mint; anti-loop). Pass B renders the real, mock-shaped
//! surface (`docs/design/mocks/authoring-surface-mock.html`): a header (intent +
//! New card + Ask AI + the Library-entities count), the comp FILMSTRIP (your
//! minted drafts), and the FOUR-BUCKET palette — Blocks · Elements · Effects ·
//! Templates — over the REAL registries (Census law: `dpl_slides::demos()`,
//! `dpl_slides::blocks::elements()`, `dpl_coldlight::TRAITS`), so a bucket can
//! never drift from what actually exists. Tapping a template mints a draft OF
//! that template and publishes `CardAuthored`. Pass C adds the OVERLAY EDITOR —
//! a scrim `egui::Modal` opened by New card / a template / a comp-card tap, with
//! the five tabs (Content · Quote · Media · Interactions · Ask AI). Content is
//! the REAL `dpl_editor::Editor` over the draft's `slide.json` (edits auto-save,
//! the editor stays fabric-free per the layer law); the other tabs are honest
//! guided stubs until their constrained-capture forms land (D/E). Real AI
//! generation (Ask AI → Copy-prompt) is Pass E.
//!
//! Law: the surface PUBLISHES `build.*` and never SUBSCRIBES to it (anti-loop);
//! it paints ColdLight only; a mutation publishes only AFTER the write succeeds.

use dpl_coldlight::{self as cl, Tone};
use egui::RichText;

/// The Workbench house tone.
const TONE: Tone = Tone::Amber;

/// The overlay editor's tabs (charter §3c / mock). Content is the real editor
/// (reusing `dpl_editor::Editor`); the rest are honest guided stubs until their
/// constrained-capture forms land.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum EditorTab {
    #[default]
    Content,
    Quote,
    Media,
    Interactions,
    AskAi,
}

impl EditorTab {
    const ALL: [EditorTab; 5] = [
        EditorTab::Content,
        EditorTab::Quote,
        EditorTab::Media,
        EditorTab::Interactions,
        EditorTab::AskAi,
    ];

    fn label(self) -> &'static str {
        match self {
            EditorTab::Content => "Content",
            EditorTab::Quote => "Quote",
            EditorTab::Media => "Media",
            EditorTab::Interactions => "Interactions",
            EditorTab::AskAi => "Ask AI",
        }
    }
}

/// The open overlay editor — a real `dpl_editor::Editor` over the draft's
/// `slide.json` (edits auto-save; the editor stays fabric-free per the layer
/// law) plus the active tab.
struct CardEditor {
    title: String,
    editor: dpl_editor::Editor,
    tab: EditorTab,
}

/// The four user-facing buckets — the ENTIRE user-facing vocabulary (charter):
/// all 7 census categories, treatments, cost tiers etc. sort into these four.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Bucket {
    Blocks,
    Elements,
    Effects,
    #[default]
    Templates,
}

impl Bucket {
    const ALL: [Bucket; 4] = [
        Bucket::Blocks,
        Bucket::Elements,
        Bucket::Effects,
        Bucket::Templates,
    ];

    fn label(self) -> &'static str {
        match self {
            Bucket::Blocks => "Blocks",
            Bucket::Elements => "Elements",
            Bucket::Effects => "Effects",
            Bucket::Templates => "Templates",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Bucket::Blocks => "chunks",
            Bucket::Elements => "atoms",
            Bucket::Effects => "looks & recipes",
            Bucket::Templates => "whole slides",
        }
    }

    fn tip(self) -> &'static str {
        match self {
            Bucket::Blocks => "Assembled chunks — bigger than an atom, smaller than a slide.",
            Bucket::Elements => "The atoms every slide is built from — type, marks, meters, media.",
            Bucket::Effects => "The looks (and, soon, recipes) a card can be dressed in.",
            Bucket::Templates => "Whole slide designs — tap one to start a card from it.",
        }
    }
}

/// One card in the comp filmstrip — a real draft loaded from `content/inbox`.
struct DraftCard {
    id: String,
    template: String,
    title: String,
}

/// The palette's registry snapshot, cached once (the registries are `'static`,
/// but `demos()` constructs instances — we read specs once, not per frame).
struct Palette {
    /// (template id, label, class label).
    templates: Vec<(&'static str, &'static str, &'static str)>,
    /// (element label, category label).
    elements: Vec<(&'static str, &'static str)>,
    /// (effect/trait label, role).
    effects: Vec<(&'static str, &'static str)>,
    /// The Library-entities count — the number that "grows itself".
    entities: usize,
}

impl Palette {
    fn load() -> Self {
        let templates: Vec<_> = dpl_slides::demos()
            .iter()
            .map(|t| {
                let s = t.spec();
                (s.id.0, s.label, s.class.label())
            })
            .collect();
        let elements: Vec<_> = dpl_slides::blocks::elements()
            .iter()
            .map(|e| {
                let s = e.spec();
                (s.label, s.category.label())
            })
            .collect();
        let effects: Vec<_> = cl::TRAITS.iter().map(|d| (d.label, d.role)).collect();
        let entities = templates.len() + elements.len() + effects.len();
        Self {
            templates,
            elements,
            effects,
            entities,
        }
    }
}

#[derive(Default)]
pub(crate) struct AuthorSurface {
    /// The intent bar (charter §3b) — captured first; becomes the draft's text
    /// and (Pass E) the Ask-AI brief. No fabric traffic until a mutation.
    intent: String,
    /// The last action's honest result line.
    status: String,
    /// Next-draft hint; `mint_draft` still probes disk so nothing is clobbered.
    next_ix: u32,
    /// The open palette bucket.
    bucket: Bucket,
    /// The comp filmstrip, lazily loaded; invalidated (set `None`) on every mint.
    drafts: Option<Vec<DraftCard>>,
    /// The registry snapshot, loaded once.
    palette: Option<Palette>,
    /// The open overlay editor (Pass C) — `None` = closed.
    editing: Option<CardEditor>,
    /// One-shot: whether the `DPL_AUTHOR_EDIT` proof affordance has been read.
    env_checked: bool,
}

impl AuthorSurface {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        if self.palette.is_none() {
            self.palette = Some(Palette::load());
        }
        if self.drafts.is_none() {
            self.drafts = Some(load_drafts());
        }
        // Proof affordance (inert in normal use): open the editor on a draft so
        // the overlay can be screenshotted headlessly. Mirrors `DPL_COMPOSER_TAB`.
        if !self.env_checked {
            self.env_checked = true;
            if let Ok(id) = std::env::var("DPL_AUTHOR_EDIT") {
                let dir = dpl_engine_capture::content_root().join("inbox").join(&id);
                self.open_editor(&dir, id);
            }
        }

        self.header(ui, fabric);
        ui.add_space(cl::space::S3);

        let full = ui.available_size();
        let rail_w = 320.0_f32.min(full.x * 0.42);
        let gap = cl::space::S3;
        let comp_w = (full.x - rail_w - gap).max(220.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(comp_w, full.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.comp_deck(ui),
            );
            ui.add_space(gap);
            ui.allocate_ui_with_layout(
                egui::vec2(rail_w, full.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.palette_rail(ui, fabric),
            );
        });

        // The overlay editor rides on top of everything (Pass C).
        self.editor_modal(ui);
    }

    /// The header: the promise, the intent bar, and the actions.
    fn header(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        ui.label(
            RichText::new("DevPULSE · Author")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Build a card. ")
                    .heading()
                    .color(cl::ink::STRONG),
            );
            ui.label(
                RichText::new("The library grows itself.")
                    .heading()
                    .color(TONE.color()),
            );
        });
        ui.label(
            RichText::new(
                "Say what you want, then pull a template to start a card. Every card you author \
                 is a real slide document and rides the signal fabric — and the four buckets are \
                 the whole vocabulary, drawn live from what actually exists.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S3);

        // Intent bar (charter §3b) — captured, editable, no signal yet.
        cl::tip(
            ui.add(
                egui::TextEdit::singleline(&mut self.intent)
                    .hint_text(
                        "What do you want to make? e.g. a title card that says “DevPULSE weekly”",
                    )
                    .desired_width(f32::INFINITY),
            ),
            "Your intention. It becomes the new card's text and (later) the Ask-AI brief.",
        );
        ui.add_space(cl::space::S2);

        ui.horizontal(|ui| {
            // New card — mints a blank draft (the "title" template) carrying intent.
            let new = cl::tip(
                ui.label(RichText::new("+ New card").strong().color(TONE.color()))
                    .interact(egui::Sense::click()),
                "Start a fresh card and announce it on the fabric.",
            );
            if new.clicked() {
                self.author_card(None, fabric);
            }
            ui.add_space(cl::space::S3);

            // Ask AI — the Copy-prompt composer lands in Pass E.
            let ask = cl::tip(
                ui.label(RichText::new("Ask AI").strong().color(cl::ink::MUTED))
                    .interact(egui::Sense::click()),
                "Compose a paste-ready prompt from what you've picked (Pass E).",
            );
            if ask.clicked() {
                self.status =
                    "Ask AI composes a paste-ready, entity-first prompt — it lands in Pass E."
                        .to_string();
            }

            // Library-entities count — the number that "grows itself".
            let entities = self.palette.as_ref().map(|p| p.entities).unwrap_or(0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cl::tip(
                    ui.label(RichText::new("entities").small().color(cl::ink::SUBTLE)),
                    "Registered, reusable entities in your library — every ask leaves one behind.",
                );
                ui.label(
                    RichText::new(entities.to_string())
                        .strong()
                        .color(TONE.color()),
                );
                ui.label(RichText::new("Library").small().color(cl::ink::SUBTLE));
            });
        });

        if !self.status.is_empty() {
            ui.add_space(cl::space::S2);
            ui.label(RichText::new(&self.status).small().color(cl::ink::BASE));
        }
    }

    /// The comp filmstrip — the cards you've authored, newest work in place.
    fn comp_deck(&mut self, ui: &mut egui::Ui) {
        // Own the render data so a click can mutate `self` (open the editor)
        // without a borrow clash against `self.drafts`.
        let cards: Vec<(String, String, String)> = self
            .drafts
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|d| (d.id.clone(), d.template.clone(), d.title.clone()))
            .collect();

        ui.horizontal(|ui| {
            ui.label(RichText::new("YOUR COMP").small().color(cl::ink::SUBTLE));
            ui.label(
                RichText::new(format!("· {} card{}", cards.len(), plural(cards.len())))
                    .small()
                    .color(cl::ink::MUTED),
            );
        });
        ui.add_space(cl::space::S2);

        if cards.is_empty() {
            cl::clay().show(ui, |ui| {
                ui.set_min_height(120.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(cl::space::S3);
                    ui.label(RichText::new("No cards yet").strong().color(cl::ink::MUTED));
                    ui.label(
                        RichText::new("Tap “New card”, or pull a template from the Library rail")
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                });
            });
            return;
        }

        let mut open: Option<String> = None;
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (id, template, title) in &cards {
                        if comp_card(ui, template, title, id) {
                            open = Some(id.clone());
                        }
                        ui.add_space(cl::space::S2);
                    }
                });
            });
        if let Some(id) = open {
            let dir = dpl_engine_capture::content_root().join("inbox").join(&id);
            self.open_editor(&dir, id);
        }
    }

    /// The four-bucket palette — the raw pieces, drawn live from the registries.
    fn palette_rail(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        cl::clay().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("LIBRARY — pull the raw")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            ui.add_space(cl::space::S2);

            // Bucket chips (the four-word vocabulary).
            ui.horizontal_wrapped(|ui| {
                for b in Bucket::ALL {
                    let active = self.bucket == b;
                    let chip = cl::tip(
                        ui.label(RichText::new(b.label()).strong().color(if active {
                            TONE.color()
                        } else {
                            cl::ink::MUTED
                        }))
                        .interact(egui::Sense::click()),
                        b.tip(),
                    );
                    if chip.clicked() {
                        self.bucket = b;
                    }
                    ui.add_space(cl::space::S2);
                }
            });
            ui.label(
                RichText::new(self.bucket.hint())
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            ui.add_space(cl::space::S2);
            ui.separator();

            let h = (ui.available_height() - 8.0).max(80.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(h)
                .show(ui, |ui| self.bucket_items(ui, fabric));
        });
    }

    /// The active bucket's items — one row each, name + kind tag.
    fn bucket_items(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        let palette = match &self.palette {
            Some(p) => p,
            None => return,
        };
        match self.bucket {
            Bucket::Templates => {
                // Own the rows so we can mutate self on click without a borrow clash.
                let rows: Vec<(&'static str, &'static str, &'static str)> =
                    palette.templates.clone();
                for (id, label, class) in rows {
                    if palette_row(ui, label, class) {
                        self.author_card(Some(id), fabric);
                    }
                }
            }
            Bucket::Elements => {
                let rows: Vec<(&'static str, &'static str)> = palette.elements.clone();
                for (label, category) in rows {
                    if palette_row(ui, label, category) {
                        self.status = format!(
                            "“{label}” attaches inside the card editor — that overlay lands in Pass C."
                        );
                    }
                }
            }
            Bucket::Effects => {
                let rows: Vec<(&'static str, &'static str)> = palette.effects.clone();
                for (label, role) in rows {
                    if palette_row(ui, label, role) {
                        self.status = format!(
                            "“{label}” dresses a card — treatments thread through in a later step."
                        );
                    }
                }
            }
            Bucket::Blocks => {
                ui.add_space(cl::space::S2);
                ui.label(
                    RichText::new("No assembled chunks yet")
                        .strong()
                        .color(cl::ink::MUTED),
                );
                ui.label(
                    RichText::new(
                        "Blocks are chunks bigger than an Element and smaller than a slide. \
                         Where exactly that line sits is a pending charter decision (§8.1), so \
                         this shelf stays honestly empty until it's ratified — never faked.",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            }
        }
    }

    /// Author a card: mint a real draft (optionally OF a template), then publish
    /// `CardAuthored` — but ONLY after the write succeeds (emit-with-mutation).
    fn author_card(&mut self, template: Option<&str>, fabric: &Option<dpl_fabric::Fabric>) {
        match self.mint_draft(template) {
            Ok(slide) => {
                self.status = format!("authored inbox/{slide} — card minted");
                self.drafts = None; // the filmstrip reloads
                if let Some(f) = fabric {
                    f.publish_build(dpl_fabric::BuildEvent::CardAuthored {
                        show: String::new(),
                        episode: String::new(),
                        slide: slide.clone(),
                    });
                }
                // Open the overlay editor on the fresh card so authoring flows
                // straight into refining it (charter §3c).
                let dir = dpl_engine_capture::content_root()
                    .join("inbox")
                    .join(&slide);
                self.open_editor(&dir, slide);
            }
            Err(e) => self.status = format!("could not author a card: {e}"),
        }
    }

    /// Open the overlay editor on a draft folder (a fresh mint or a tapped comp
    /// card). The embedded `dpl_editor::Editor` edits the real `slide.json`.
    fn open_editor(&mut self, dir: &std::path::Path, title: String) {
        match dpl_editor::Editor::open(dir) {
            Ok(editor) => {
                self.editing = Some(CardEditor {
                    title,
                    editor,
                    tab: EditorTab::default(),
                });
            }
            Err(e) => self.status = format!("could not open the card editor: {e}"),
        }
    }

    /// The overlay card editor (Pass C) — a scrim modal with the five tabs.
    /// Content is the real `dpl_editor::Editor`; the rest are honest guided
    /// stubs. Edits auto-save; closing reloads the filmstrip.
    fn editor_modal(&mut self, ui: &mut egui::Ui) {
        let mut close = false;
        if let Some(ed) = self.editing.as_mut() {
            let modal = egui::Modal::new(egui::Id::new("author.card-editor"))
                .backdrop_color(egui::Color32::from_black_alpha(184))
                .frame(
                    egui::Frame::new()
                        .fill(cl::surface::RAISED)
                        .stroke(egui::Stroke::new(1.0, TONE.color()))
                        .corner_radius(egui::CornerRadius::same(cl::RADIUS_LG))
                        .inner_margin(egui::Margin::same(16)),
                )
                .show(ui.ctx(), |ui| {
                    ui.set_width(720.0);

                    // Head — title + close.
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("EDIT CARD")
                                .monospace()
                                .small()
                                .color(TONE.color()),
                        );
                        ui.label(RichText::new(&ed.title).strong().color(cl::ink::STRONG));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if cl::tip(
                                ui.label(RichText::new("Close").small().color(cl::ink::MUTED))
                                    .interact(egui::Sense::click()),
                                "Close — your edits are already saved.",
                            )
                            .clicked()
                            {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(cl::space::S2);

                    // Tab bar.
                    ui.horizontal_wrapped(|ui| {
                        for t in EditorTab::ALL {
                            let active = ed.tab == t;
                            if ui
                                .label(RichText::new(t.label()).strong().color(if active {
                                    TONE.color()
                                } else {
                                    cl::ink::MUTED
                                }))
                                .interact(egui::Sense::click())
                                .clicked()
                            {
                                ed.tab = t;
                            }
                            ui.add_space(cl::space::S3);
                        }
                    });
                    ui.separator();

                    // Body — the active tab.
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(440.0)
                        .show(ui, |ui| match ed.tab {
                            EditorTab::Content => ed.editor.ui(ui),
                            other => tab_stub(ui, other),
                        });

                    ui.separator();
                    // Foot.
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("edits save as you type")
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if cl::tip(
                                ui.label(RichText::new("Done").strong().color(TONE.color()))
                                    .interact(egui::Sense::click()),
                                "Close the editor — your card is saved.",
                            )
                            .clicked()
                            {
                                close = true;
                            }
                        });
                    });
                });
            if modal.should_close() {
                close = true;
            }
        }
        if close {
            self.editing = None;
            self.drafts = None; // the filmstrip reloads (title may have changed)
        }
    }

    /// Mint a real, schema-valid draft `slide.json` in `content/inbox/`. The
    /// draft uses `template` (default `"title"`) and carries the intent text as
    /// one block so the mutation honestly reflects what the person asked for.
    ///
    /// No-clobber: probes `authored-draft-<n>` for the first free folder.
    fn mint_draft(&mut self, template: Option<&str>) -> Result<String, String> {
        let inbox = dpl_engine_capture::content_root().join("inbox");

        let mut n = self.next_ix.max(1);
        let (id, dir) = loop {
            let id = format!("authored-draft-{n}");
            let dir = inbox.join(&id);
            if !dir.exists() {
                break (id, dir);
            }
            n += 1;
            if n > 100_000 {
                return Err("inbox is full of drafts".into());
            }
        };
        self.next_ix = n + 1;

        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let intent = self.intent.trim();
        let blocks = if intent.is_empty() {
            Vec::new()
        } else {
            vec![dpl_doc::Block::Text {
                id: "intent".into(),
                text: intent.to_string(),
                font: None,
                size: "h3".into(),
                ink: None,
            }]
        };
        let slide = dpl_doc::Slide {
            v: dpl_doc::SCHEMA_V,
            template: template.unwrap_or("title").to_string(),
            background: None,
            blocks,
            enter: None,
            reveals: Vec::new(),
        };
        dpl_doc::save(&dir, &slide).map_err(|e| e.to_string())?;
        Ok(id)
    }
}

/// One comp card — a FIXED-size clay plate showing the draft's template + its
/// intent text. Every label truncates (never wraps), so a long id/title can't
/// stretch the card vertically. Returns true when tapped (opens the editor).
fn comp_card(ui: &mut egui::Ui, template: &str, title: &str, id: &str) -> bool {
    let title = if title.trim().is_empty() {
        "Untitled"
    } else {
        title.trim()
    };
    let resp = cl::clay()
        .show(ui, |ui| {
            ui.set_width(150.0);
            ui.set_min_height(88.0);
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(template.to_uppercase())
                            .small()
                            .color(TONE.color()),
                    )
                    .truncate(),
                );
                ui.add_space(cl::space::S1);
                ui.add(
                    egui::Label::new(RichText::new(title).strong().color(cl::ink::BASE)).truncate(),
                );
                ui.add_space(cl::space::S1);
                ui.add(
                    egui::Label::new(RichText::new(id).small().color(cl::ink::SUBTLE)).truncate(),
                );
            });
        })
        .response
        .interact(egui::Sense::click());
    cl::tip(resp, "Tap to open this card in the editor.").clicked()
}

/// A tab that isn't wired yet — named honestly, never faked (charter passes
/// D/E fill these). Keeps the modal walkable end to end.
fn tab_stub(ui: &mut egui::Ui, tab: EditorTab) {
    let (what, when) = match tab {
        EditorTab::Quote => (
            "Guided quote capture — attribution, source, and a bounded length.",
            "the constrained-fields pass",
        ),
        EditorTab::Media => (
            "Attach an image, video, or PDF page as a content-jailed media block.",
            "the media pass",
        ),
        EditorTab::Interactions => (
            "Dress the card with a registered recipe (reveal, flip, explode) — the \
             AI registers the entity first, then applies it.",
            "Pass D",
        ),
        EditorTab::AskAi => (
            "Compose a paste-ready, entity-first prompt from what you've picked, \
             then hand it to the context-aware chat.",
            "Pass E",
        ),
        EditorTab::Content => ("", ""),
    };
    ui.add_space(cl::space::S3);
    ui.label(RichText::new(tab.label()).strong().color(cl::ink::STRONG));
    ui.add_space(cl::space::S1);
    ui.label(RichText::new(what).color(cl::ink::BASE));
    ui.add_space(cl::space::S2);
    ui.label(
        RichText::new(format!(
            "Lands in {when} — the card stays valid until then."
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
}

/// One palette row — name + kind tag; returns true when tapped.
fn palette_row(ui: &mut egui::Ui, name: &str, kind: &str) -> bool {
    let resp = ui
        .horizontal(|ui| {
            let r = ui
                .label(RichText::new(name).color(cl::ink::BASE))
                .interact(egui::Sense::click());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(kind)
                        .small()
                        .monospace()
                        .color(cl::ink::SUBTLE),
                );
            });
            r
        })
        .inner;
    let resp = cl::tip(
        resp,
        "Tap to start a card from this — or drag onto one (Pass C).",
    );
    ui.add_space(cl::space::S1);
    resp.clicked()
}

/// Load the comp filmstrip from `content/inbox/authored-draft-*`.
fn load_drafts() -> Vec<DraftCard> {
    let inbox = dpl_engine_capture::content_root().join("inbox");
    let mut cards = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&inbox) {
        for entry in rd.flatten() {
            let id = entry.file_name().to_string_lossy().into_owned();
            if !id.starts_with("authored-draft-") {
                continue;
            }
            if let Ok(slide) = dpl_doc::load(&entry.path()) {
                let title = slide
                    .blocks
                    .iter()
                    .find_map(|b| match b {
                        dpl_doc::Block::Text { text, .. } => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                cards.push(DraftCard {
                    id,
                    template: slide.template,
                    title,
                });
            }
        }
    }
    cards.sort_by(|a, b| a.id.cmp(&b.id));
    cards
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}
