//! Authoring surface — charter pass 1 (Pass A: signals + skeleton).
//!
//! The real egui authoring surface, wired to the fabric from its first
//! commit. Pass A is the SPINE, not the full flow: an intent bar (captured
//! first, charter §3b) and one honest mutation — "Seed a draft card" mints a
//! real, schema-valid `slide.json` draft in the content inbox and publishes
//! `BuildEvent::CardAuthored` AFTER the write (emit-with-mutation). The
//! four-bucket gallery, filmstrip, and the overlay editor land in Passes B–C.
//!
//! Law: the surface PUBLISHES `build.*` and never SUBSCRIBES to it
//! (anti-loop) — the coachable-signal catalog forwards the fact to touchpoints
//! so AI/coach overlays react without the surface self-triggering.

use dpl_coldlight::{self as cl, Tone};
use egui::RichText;

/// The authoring surface's transient state. Default-derivable so the hub
/// stays `#[derive(Default)]`.
#[derive(Default)]
pub(crate) struct AuthorSurface {
    /// The intent bar (charter §3b) — captured first, threads into the draft
    /// and (Pass E) the Ask-AI brief. Facts only; no fabric traffic until a
    /// mutation.
    intent: String,
    /// The last action's honest result line.
    status: String,
    /// A hint for the next draft index; `mint_draft` still probes disk for a
    /// free folder so nothing is ever clobbered.
    next_ix: u32,
}

impl AuthorSurface {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        ui.label(
            RichText::new("AUTHOR — turn an intention into a card")
                .heading()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "Say what you want, then seed a draft card. Pass A proves the spine: \
                 every authored card is a REAL slide document and rides the signal fabric. \
                 The four-bucket gallery, filmstrip, and the overlay editor land next.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S3);

        // Intent bar — charter §3b. Captured, editable, no signal yet.
        ui.label(
            RichText::new("What do you want to make?")
                .strong()
                .color(cl::ink::BASE),
        );
        cl::tip(
            ui.add(
                egui::TextEdit::singleline(&mut self.intent)
                    .hint_text("e.g. a title card that says “DevPULSE weekly”")
                    .desired_width(f32::INFINITY),
            ),
            "Your intention. It becomes the draft's text and (later) the Ask-AI brief.",
        );
        ui.add_space(cl::space::S3);

        let seed = cl::tip(
            ui.label(
                RichText::new("+ Seed a draft card")
                    .strong()
                    .color(Tone::Amber.color()),
            )
            .interact(egui::Sense::click()),
            "Write a real draft slide.json into the inbox and announce it on the fabric.",
        );
        if seed.clicked() {
            match self.mint_draft() {
                Ok(slide) => {
                    self.status = format!("authored inbox/{slide} — draft card minted");
                    // Emit-with-mutation: the fact is published AFTER the write
                    // succeeds, never before. show/episode are empty until the
                    // card is placed into an episode (Studio's `SlidePlaced`).
                    if let Some(f) = fabric {
                        f.publish_build(dpl_fabric::BuildEvent::CardAuthored {
                            show: String::new(),
                            episode: String::new(),
                            slide,
                        });
                    }
                }
                Err(e) => self.status = format!("could not author a draft: {e}"),
            }
        }

        if !self.status.is_empty() {
            ui.add_space(cl::space::S2);
            ui.label(RichText::new(&self.status).small().color(cl::ink::BASE));
        }
    }

    /// Mint a real, schema-valid draft `slide.json` in `content/inbox/`. The
    /// draft carries the intent text as one block so the mutation honestly
    /// reflects what the person typed. Returns the draft's folder id.
    ///
    /// No-clobber: probes `authored-draft-<n>` for the first free folder, so a
    /// second tap never overwrites the first (mirrors the composer's mint law).
    fn mint_draft(&mut self) -> Result<String, String> {
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
            template: "title".into(),
            background: None,
            blocks,
            enter: None,
            reveals: Vec::new(),
        };
        dpl_doc::save(&dir, &slide).map_err(|e| e.to_string())?;
        Ok(id)
    }
}
