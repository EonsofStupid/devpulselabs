//! Authoring surface — charter passes A + B.
//!
//! Pass A gave the SIGNAL SPINE (additive `BuildEvent::CardAuthored`, published
//! after a real draft-mint; anti-loop). Pass B renders the real, mock-shaped
//! surface (`docs/design/mocks/authoring-surface-mock.html`): a header (intent +
//! New card + Ask AI + the Library-entities count), the comp FILMSTRIP (your
//! minted drafts), and the FOUR-BUCKET palette — Blocks · Elements · Effects ·
//! Templates — over the REAL registries (Census law: `dpl_slides::demos()`,
//! `dpl_slides::blocks::elements()`, `dpl_coldlight::TRAITS`), so a bucket can
//! never drift from what actually exists. Tapping a template mints a draft OF
//! that template and publishes `CardAuthored`. The overlay editor is Pass C;
//! real AI generation (Ask AI → Copy-prompt) is Pass E.
//!
//! Law: the surface PUBLISHES `build.*` and never SUBSCRIBES to it (anti-loop);
//! it paints ColdLight only; a mutation publishes only AFTER the write succeeds.

use dpl_coldlight::{self as cl, Tone};
use egui::RichText;

/// The Workbench house tone.
const TONE: Tone = Tone::Amber;

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
}

impl AuthorSurface {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        if self.palette.is_none() {
            self.palette = Some(Palette::load());
        }
        if self.drafts.is_none() {
            self.drafts = Some(load_drafts());
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
        let drafts = self.drafts.as_deref().unwrap_or(&[]);
        ui.horizontal(|ui| {
            ui.label(RichText::new("YOUR COMP").small().color(cl::ink::SUBTLE));
            ui.label(
                RichText::new(format!("· {} card{}", drafts.len(), plural(drafts.len())))
                    .small()
                    .color(cl::ink::MUTED),
            );
        });
        ui.add_space(cl::space::S2);

        if drafts.is_empty() {
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

        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for d in drafts {
                        comp_card(ui, d);
                        ui.add_space(cl::space::S2);
                    }
                });
            });
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
                        slide,
                    });
                }
            }
            Err(e) => self.status = format!("could not author a card: {e}"),
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
/// stretch the card vertically.
fn comp_card(ui: &mut egui::Ui, d: &DraftCard) {
    let title = if d.title.trim().is_empty() {
        "Untitled"
    } else {
        d.title.trim()
    };
    let resp = cl::clay()
        .show(ui, |ui| {
            ui.set_width(150.0);
            ui.set_min_height(88.0);
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(d.template.to_uppercase())
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
                    egui::Label::new(RichText::new(&d.id).small().color(cl::ink::SUBTLE))
                        .truncate(),
                );
            });
        })
        .response;
    cl::tip(
        resp,
        "Your authored draft. Opening it to edit lands with the overlay editor (Pass C).",
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
