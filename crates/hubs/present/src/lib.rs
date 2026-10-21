//! Present — the show machine's front door (D2) + the presenter (D4).
//!
//! SHOWS lists the episodes; tapping one opens the DeckHost and asks the
//! shell for full-bleed over the fabric (`ShellEvent::FullBleedRequest`).
//! Leaving the show (toolbar Home / home dot) hands the chrome back.
//!
//! The PRESENTER (D4): the deck's Cue button opens a second viewport —
//! the teleprompter — showing the active script cue, its note, the beat
//! list, up-next, and a running timer. Sync is FABRIC-ONLY: the deck
//! publishes `DeckEvent::Beat` on every real move and the presenter
//! consumes; nothing reaches into the deck's state. One-way for now —
//! the presenter never drives the deck.

use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::{DeckEvent, DeckReceiver, Fabric, ShellEvent};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;
use std::path::{Path, PathBuf};

pub const HUB: HubId = HubId("present");
pub const SHOWS: SubId = SubId("shows");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Present",
    icon: cl::icon::PRESENT,
    tone: Tone::Coral,
    subroutes: &[SubRoute {
        id: SHOWS,
        label: "Shows",
        blurb: "The shows — tap one to present it full-bleed on the big screen.",
    }],
};

/// One slide as the teleprompter knows it: its folder name, step count,
/// and the colocated script when one exists (D5).
struct PrompterSlide {
    name: String,
    steps: usize,
    script: Option<dpl_doc::script::Script>,
}

/// The teleprompter's state — built once per open from the episode dir,
/// then driven ONLY by `DeckEvent::Beat` off the fabric.
struct Presenter {
    rx: DeckReceiver,
    episode_title: String,
    rows: Vec<PrompterSlide>,
    index: usize,
    step: usize,
    started: std::time::Instant,
}

impl Presenter {
    fn open(dir: &Path, rx: DeckReceiver) -> Result<Self, String> {
        let ep = dpl_doc::load_episode(dir).map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        for r in dpl_doc::episode_slides(dir, &ep) {
            let steps = dpl_doc::load(&r.dir).map(|s| s.step_count()).unwrap_or(1);
            // A broken script never blocks the prompter — the doctor
            // (validate_episode) names it; here it reads as "no script".
            let script = dpl_doc::script::load_script(&r.dir).ok().flatten();
            rows.push(PrompterSlide {
                name: r.name,
                steps,
                script,
            });
        }
        if rows.is_empty() {
            return Err("the episode has no slides".into());
        }
        Ok(Self {
            rx,
            episode_title: ep.title,
            rows,
            index: 0,
            step: 0,
            started: std::time::Instant::now(),
        })
    }

    /// Apply every pending beat — the latest wins (signals, not jobs).
    fn drain(&mut self) {
        while let Ok(DeckEvent::Beat { index, step, .. }) = self.rx.try_recv() {
            self.index = index.min(self.rows.len().saturating_sub(1));
            self.step = step;
        }
    }

    /// The beat id at (slide, step) under the step semantics — beats[0]
    /// is "setup", then one per reveal (the D5 parity law).
    fn beat_at(&self, index: usize, step: usize) -> Option<&dpl_doc::script::ScriptBeat> {
        self.rows.get(index)?.script.as_ref()?.beats.get(step)
    }

    /// What follows the current beat: the next beat here, or the next
    /// slide's opening cue (else its name).
    fn up_next(&self) -> String {
        let here = &self.rows[self.index];
        if self.step + 1 < here.steps {
            if let Some(b) = self.beat_at(self.index, self.step + 1) {
                return format!("{} — {}", b.id, b.cue);
            }
            return format!("step {}", self.step + 1);
        }
        match self.rows.get(self.index + 1) {
            Some(next) => match self.beat_at(self.index + 1, 0) {
                Some(b) => format!("{} — {}", next.name, b.cue),
                None => next.name.clone(),
            },
            None => "end of show".into(),
        }
    }

    /// The teleprompter surface — one body for the real second viewport
    /// AND the harness's panel mode (same pixels, minus the OS window).
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.drain();
        let here = &self.rows[self.index];
        let elapsed = self.started.elapsed().as_secs();

        ui.horizontal(|ui| {
            ui.label(RichText::new("PRESENTER").small().color(SPEC.tone.color()));
            ui.label(
                RichText::new(&self.episode_title)
                    .strong()
                    .color(cl::ink::STRONG),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{:02}:{:02}", elapsed / 60, elapsed % 60))
                        .monospace()
                        .color(cl::ink::BASE),
                );
                ui.label(
                    RichText::new(format!(
                        "slide {}/{} · step {}",
                        self.index + 1,
                        self.rows.len(),
                        self.step
                    ))
                    .monospace()
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            });
        });
        ui.add_space(cl::space::S2);

        // The beat list: one chip per step, the active one lit.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(here.name.as_str())
                    .monospace()
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            for s in 0..here.steps {
                let id = self
                    .beat_at(self.index, s)
                    .map(|b| b.id.clone())
                    .unwrap_or_else(|| format!("step {s}"));
                let active = s == self.step;
                ui.label(RichText::new(id).monospace().small().color(if active {
                    SPEC.tone.color()
                } else {
                    cl::ink::MUTED
                }));
            }
        });
        ui.add_space(cl::space::S3);

        // The ACTIVE CUE — the line to land, verbatim and big.
        match self.beat_at(self.index, self.step) {
            Some(beat) => {
                ui.label(
                    RichText::new(beat.cue.as_str())
                        .size(26.0)
                        .strong()
                        .color(cl::ink::STRONG),
                );
                if let Some(note) = &beat.note {
                    ui.add_space(cl::space::S1);
                    ui.label(RichText::new(note.as_str()).italics().color(cl::ink::MUTED));
                }
            }
            None => {
                ui.label(
                    RichText::new(format!("no script for this slide — {}", here.name))
                        .color(cl::ink::MUTED),
                );
            }
        }
        ui.add_space(cl::space::S3);

        ui.label(RichText::new("UP NEXT").small().color(cl::ink::SUBTLE));
        ui.label(RichText::new(self.up_next()).color(cl::ink::BASE));
        ui.add_space(cl::space::S3);

        // The prompter prose — the script body, inert by law.
        if let Some(script) = &here.script {
            if !script.body.trim().is_empty() {
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(script.body.as_str())
                                .small()
                                .color(cl::ink::BASE),
                        );
                    });
            }
        }
        // The clock ticks even between beats.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(250));
    }
}

pub struct PresentHub {
    fabric: Fabric,
    deck: Option<dpl_deck::DeckHost>,
    /// The open episode's folder — the presenter builds from it.
    dir: Option<PathBuf>,
    presenter: Option<Presenter>,
    /// Harness only: render the presenter surface in the MAIN viewport
    /// (the screenshot harness captures only that window).
    presenter_panel: bool,
    /// The last open failure, shown honestly on the picker.
    error: Option<String>,
}

impl PresentHub {
    pub fn new(fabric: Fabric) -> Self {
        Self {
            fabric,
            deck: None,
            dir: None,
            presenter: None,
            presenter_panel: false,
            error: None,
        }
    }

    /// Open a show by its episode id, `show/episode`, or `template/<id>`
    /// (the picker-badge vocabulary; also the `--present` harness path).
    /// The union lists content first, so a real show wins a bare-id
    /// collision with a template. Unknown ids degrade to the picker.
    pub fn open_show(&mut self, id: &str) {
        let entries = dpl_doc::all_episodes(&dpl_engine_capture::content_root());
        let Some(entry) = entries.iter().find(|e| {
            e.id == id
                || e.show
                    .as_deref()
                    .is_some_and(|s| format!("{s}/{}", e.id) == id)
                || (e.show.is_none() && format!("template/{}", e.id) == id)
        }) else {
            return;
        };
        let dir = entry.dir.clone();
        self.open_dir(&dir);
    }

    /// Harness (D4 proofs): open the presenter in PANEL mode and walk
    /// the deck `n` beats through its real key path.
    pub fn presenter_drill(&mut self, n: usize) {
        self.presenter_panel = true;
        self.toggle_presenter();
        if let Some(deck) = self.deck.as_mut() {
            for _ in 0..n {
                deck.press(egui::Key::ArrowRight);
            }
        }
    }

    fn toggle_presenter(&mut self) {
        if self.presenter.is_some() {
            self.presenter = None;
            return;
        }
        let Some(dir) = self.dir.clone() else {
            return;
        };
        // Subscribe FIRST, then ask for the snapshot — a receiver only
        // hears traffic after it subscribes.
        let rx = self.fabric.subscribe_deck();
        match Presenter::open(&dir, rx) {
            Ok(p) => {
                self.presenter = Some(p);
                if let Some(deck) = &self.deck {
                    deck.publish_position();
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn open_dir(&mut self, dir: &std::path::Path) {
        match dpl_deck::DeckHost::open(dir) {
            Ok(mut host) => {
                host.wire_fabric(self.fabric.clone());
                self.deck = Some(host);
                self.dir = Some(dir.to_path_buf());
                self.error = None;
                self.fabric
                    .publish_shell(ShellEvent::FullBleedRequest(true));
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn close_deck(&mut self) {
        self.deck = None;
        self.presenter = None;
        self.dir = None;
        self.fabric
            .publish_shell(ShellEvent::FullBleedRequest(false));
    }

    fn picker(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("The shows — tap one to present it")
                .small()
                .color(cl::ink::SUBTLE),
        );
        if let Some(e) = &self.error {
            ui.label(
                RichText::new(format!("could not open the show: {e}"))
                    .small()
                    .color(SPEC.tone.color()),
            );
        }
        ui.add_space(cl::space::S2);
        let mut open: Option<PathBuf> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // The union (X3): the operator's content shows FIRST,
                // then the seed templates, labelled as such.
                for entry in dpl_doc::all_episodes(&dpl_engine_capture::content_root()) {
                    let (title, count) = match dpl_doc::load_episode(&entry.dir) {
                        Ok(ep) => {
                            let n = dpl_doc::episode_slides(&entry.dir, &ep).len();
                            (ep.title, n)
                        }
                        Err(_) => (entry.id.clone(), 0),
                    };
                    let badge = match &entry.show {
                        Some(show) => format!("{show}/{}", entry.id),
                        None => format!("template · {}", entry.id),
                    };
                    let resp = cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width().min(560.0));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&title).strong().color(cl::ink::STRONG));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("present ›").small().color(SPEC.tone.color()),
                                    );
                                    ui.label(
                                        RichText::new(format!("{count} slides"))
                                            .small()
                                            .color(cl::ink::SUBTLE),
                                    );
                                    ui.label(
                                        RichText::new(&badge)
                                            .monospace()
                                            .small()
                                            .color(cl::ink::SUBTLE),
                                    );
                                },
                            );
                        });
                    });
                    if resp.response.interact(egui::Sense::click()).clicked() {
                        open = Some(entry.dir.clone());
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        if let Some(dir) = open {
            self.open_dir(&dir);
        }
    }
}

impl Hub for PresentHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        let _ = route; // one subroute — SHOWS is the whole hub.
                       // Harness panel mode (D4 proofs): the presenter surface owns the
                       // main well — the same pixels the second viewport would carry.
        if self.presenter_panel {
            if let Some(p) = self.presenter.as_mut() {
                p.ui(ui);
                return;
            }
        }
        if let Some(deck) = self.deck.as_mut() {
            match deck.ui(ui) {
                dpl_deck::DeckAction::Exit => self.close_deck(),
                dpl_deck::DeckAction::TogglePresenter => self.toggle_presenter(),
                dpl_deck::DeckAction::None => {}
            }
            // The presenter rides in its own OS window while the show
            // runs on the big screen.
            if let Some(p) = self.presenter.as_mut() {
                let ctx = ui.ctx().clone();
                let mut close = false;
                ctx.show_viewport_immediate(
                    egui::ViewportId::from_hash_of("dpl_presenter"),
                    egui::ViewportBuilder::default()
                        .with_title("DevPULSE Presenter")
                        .with_inner_size([560.0, 720.0]),
                    |ctx, _class| {
                        egui::CentralPanel::default().show(ctx, |ui| p.ui(ui));
                        close = ctx.input(|i| i.viewport().close_requested());
                    },
                );
                if close {
                    self.presenter = None;
                }
            }
            return;
        }
        self.picker(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch episode: slide "a" (1 step, scripted), slide "b"
    /// (2 steps - one reveal - scripted), slide "c" (1 step, NO script).
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-present-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let slide = |name: &str, reveals: Vec<String>| {
            let mut doc = dpl_doc::Slide::new("title@1");
            doc.blocks = vec![
                dpl_doc::Block::text("title", format!("slide {name}")),
                dpl_doc::Block::text("sub", "beat"),
            ];
            doc.reveals = reveals;
            dpl_doc::save(&dir.join(name), &doc).unwrap();
        };
        slide("a", vec![]);
        slide("b", vec!["sub".into()]);
        slide("c", vec![]);
        std::fs::write(
            dir.join("a").join(dpl_doc::script::SCRIPT_FILE),
            "---\nslideId: a\ntitle: A\nbeats:\n  - id: setup\n    cue: open strong\n---\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("b").join(dpl_doc::script::SCRIPT_FILE),
            "---\nslideId: b\ntitle: B\nbeats:\n  - id: setup\n    cue: set it up\n  - id: sub\n    cue: land the reveal\n    note: softly\n---\n",
        )
        .unwrap();
        let ep = dpl_doc::Episode {
            v: dpl_doc::SCHEMA_V,
            id: "t".into(),
            title: "Test show".into(),
            chapters: vec![dpl_doc::Chapter {
                id: "one".into(),
                title: "One".into(),
                slides: vec!["a".into(), "b".into(), "c".into()],
            }],
        };
        dpl_doc::save_episode(&dir, &ep).unwrap();
        dir
    }

    /// D4's whole loop headlessly: the deck publishes on its real key
    /// path, the presenter hears it over the fabric and resolves the
    /// scripted cue - one-way, typed, no reach-in.
    #[test]
    fn the_presenter_follows_the_deck_over_the_fabric() {
        let dir = scratch("sync");
        let fabric = Fabric::new();
        let mut deck = dpl_deck::DeckHost::open(&dir).unwrap();
        deck.wire_fabric(fabric.clone());
        let mut p = Presenter::open(&dir, fabric.subscribe_deck()).unwrap();
        deck.publish_position();
        p.drain();
        assert_eq!((p.index, p.step), (0, 0));
        assert_eq!(p.beat_at(0, 0).unwrap().cue, "open strong");

        deck.press(egui::Key::ArrowRight); // -> b/setup
        deck.press(egui::Key::ArrowRight); // -> b/sub
        p.drain();
        assert_eq!((p.index, p.step), (1, 1), "the latest beat wins");
        let beat = p.beat_at(p.index, p.step).unwrap();
        assert_eq!(beat.cue, "land the reveal");
        assert_eq!(beat.note.as_deref(), Some("softly"));
    }

    #[test]
    fn up_next_walks_beats_then_slides_then_ends_honestly() {
        let dir = scratch("upnext");
        let fabric = Fabric::new();
        let mut p = Presenter::open(&dir, fabric.subscribe_deck()).unwrap();
        // At a/setup: next is b's opening cue (a has one step).
        assert_eq!(p.up_next(), "b \u{2014} set it up");
        // At b/setup: the next beat on the SAME slide.
        (p.index, p.step) = (1, 0);
        assert_eq!(p.up_next(), "sub \u{2014} land the reveal");
        // At b/sub: c follows, unscripted - its name, honestly.
        (p.index, p.step) = (1, 1);
        assert_eq!(p.up_next(), "c");
        // At c: nothing follows.
        (p.index, p.step) = (2, 0);
        assert_eq!(p.up_next(), "end of show");
        // c is unscripted - the active cue degrades honestly too.
        assert!(p.beat_at(2, 0).is_none());
    }
}
