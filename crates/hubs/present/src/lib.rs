//! Present — the show machine's front door (D2), the presenter (D4), and
//! RUN mode (RL, ruled 2026-10-25).
//!
//! SHOWS lists the episodes; tapping **Run** opens the DeckHost. A lesson
//! or an episode can also be run from the Workbench: its Run buttons
//! publish `DeckCommand::Run` and this hub drains it. Run mode is locked
//! (nothing editable, the pinned cards): the slides go full-bleed and OS
//! full screen, and the PRESENTER opens at once in its own window — the
//! teleprompter: the active cue, its note, the beat list, up next, a timer
//! — with big Next / Back, tap-to-jump, and Swap screens. The presenter
//! drives the deck through this hub; the deck publishes `DeckEvent::Beat`
//! on every real move and the presenter follows it. Where the two windows
//! sit is remembered per machine (R289: the app attunes to the machine it
//! is on). Leaving the show (toolbar Home / home dot) hands everything
//! back.

use dpl_coldlight::{self as cl, Tone};
use dpl_doc::episode::Showing;
use dpl_fabric::{
    DeckCommand, DeckCommandReceiver, DeckEvent, DeckReceiver, Fabric, RunTarget, ShellEvent,
};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;
use egui_pilot_ai::{ContextPilotExt, NodeMetadata, PilotId, PilotKey, UiPilotExt};
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
        blurb: "The shows — tap Run to show one full screen on the big screen.",
    }],
};

/// One slide as the teleprompter knows it: its folder name, step count,
/// and the colocated script when one exists (D5).
struct PrompterSlide {
    name: String,
    steps: usize,
    script: Option<dpl_doc::script::Script>,
}

/// What the presenter asks of the show (RL, "presenter drives").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresenterAct {
    Next,
    Back,
    /// Card, tap.
    Jump(usize, usize),
    /// Put the slides on the presenter's screen and the presenter on the
    /// slides'.
    Swap,
}

/// A presenter button: big enough to hit on a touchscreen, a tooltip in
/// plain words, and a pilot id under `present.presenter`.
fn presenter_button(ui: &mut egui::Ui, key: &str, label: &str, tip: &str) -> bool {
    let resp = ui.add(
        egui::Button::new(RichText::new(label).size(18.0).strong())
            .min_size(egui::vec2(132.0, 48.0)),
    );
    ui.pilot_track(
        &resp,
        NodeMetadata::new(
            PilotId::new("present.presenter").child(&PilotKey::new(key)),
            egui_pilot_ai::protocol::Role::Button,
            label,
        ),
    );
    cl::tip(resp, tip).clicked()
}

/// The teleprompter's state — built once per open from what runs, then
/// driven ONLY by `DeckEvent::Beat` off the fabric.
struct Presenter {
    rx: DeckReceiver,
    episode_title: String,
    rows: Vec<PrompterSlide>,
    index: usize,
    step: usize,
    started: std::time::Instant,
}

impl Presenter {
    fn open(showing: &Showing, content_root: &Path, rx: DeckReceiver) -> Result<Self, String> {
        let shown = showing.shown(content_root).map_err(|e| e.to_string())?;
        // The same walk as the deck, so beat N here is beat N there. A
        // broken script never blocks the prompter — the doctor names it;
        // here it reads as "no script".
        let rows: Vec<PrompterSlide> = shown
            .cards()
            .map(|card| PrompterSlide {
                name: card.label.clone(),
                steps: card.slide.as_ref().map(|s| s.step_count()).unwrap_or(1),
                script: card.script.clone(),
            })
            .collect();
        if rows.is_empty() {
            return Err("there are no cards to show".into());
        }
        Ok(Self {
            rx,
            episode_title: shown.title,
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
    /// Returns what the presenter asked of the show this frame.
    fn ui(&mut self, ui: &mut egui::Ui) -> Option<PresenterAct> {
        self.drain();
        // Its own solid ground — the OS window has no panel under it, and
        // the harness's panel mode sits over the show.
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, cl::surface::WELL);
        let mut act = None;
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

        // Driving the show from here (RL): Back, Next, and the screens.
        ui.horizontal(|ui| {
            if presenter_button(ui, "back", "‹ Back", "Go back one tap") {
                act = Some(PresenterAct::Back);
            }
            if presenter_button(ui, "next", "Next ›", "Show the next tap") {
                act = Some(PresenterAct::Next);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if presenter_button(
                    ui,
                    "swap",
                    "Swap screens",
                    "Put the slides on this screen and these notes on the other",
                ) {
                    act = Some(PresenterAct::Swap);
                }
            });
        });
        ui.add_space(cl::space::S2);

        // The beat list: one chip per step, the active one lit; tap one to
        // go straight there.
        ui.horizontal_wrapped(|ui| {
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
                let chip = ui.add(
                    egui::Button::new(RichText::new(id).monospace().small().color(if active {
                        SPEC.tone.color()
                    } else {
                        cl::ink::MUTED
                    }))
                    .frame(false),
                );
                ui.pilot_track(
                    &chip,
                    NodeMetadata::new(
                        PilotId::new("present.presenter")
                            .child(&PilotKey::new(format!("beat/{s}"))),
                        egui_pilot_ai::protocol::Role::Button,
                        "Go to this tap",
                    ),
                );
                if cl::tip(chip, "Go straight to this tap").clicked() && !active {
                    act = Some(PresenterAct::Jump(self.index, s));
                }
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
        let next = ui.add(
            egui::Button::new(RichText::new(self.up_next()).color(cl::ink::BASE)).frame(false),
        );
        ui.pilot_track(
            &next,
            NodeMetadata::new(
                PilotId::new("present.presenter").child(&PilotKey::new("up-next")),
                egui_pilot_ai::protocol::Role::Button,
                "Up next",
            ),
        );
        if cl::tip(next, "Go on to this").clicked() {
            act = Some(PresenterAct::Next);
        }
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
        act
    }
}

/// Where the two windows of a run sit on one machine: the slides window's
/// top-left, and the presenter's top-left and size, in screen points.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Places {
    slides: egui::Pos2,
    presenter: egui::Rect,
}

/// This machine's name — the key its window places are kept under, so a
/// portable app opened on another machine never borrows this one's screens.
fn machine() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "this-machine".into())
}

/// Where the window places are kept: app state, keyed by machine.
fn places_file() -> PathBuf {
    dpl_engine_capture::userdata_root().join("present-screens.json")
}

/// This machine's remembered places, if any.
fn load_places(file: &Path, machine: &str) -> Option<Places> {
    let all: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    let here = all.get(machine)?;
    let num = |v: &serde_json::Value, i: usize| v.get(i)?.as_f64().map(|f| f as f32);
    let s = here.get("slides")?;
    let p = here.get("presenter")?;
    Some(Places {
        slides: egui::pos2(num(s, 0)?, num(s, 1)?),
        presenter: egui::Rect::from_min_size(
            egui::pos2(num(p, 0)?, num(p, 1)?),
            egui::vec2(num(p, 2)?, num(p, 3)?),
        ),
    })
}

/// Keep this machine's places, leaving every other machine's as it was.
fn save_places(file: &Path, machine: &str, places: Places) -> Result<(), String> {
    let mut all: serde_json::Value = std::fs::read_to_string(file)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    let r = places.presenter;
    all[machine] = serde_json::json!({
        "slides": [places.slides.x, places.slides.y],
        "presenter": [r.min.x, r.min.y, r.width(), r.height()],
    });
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        file,
        serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// What a run asked of the OS windows, applied on the next frame (the hub
/// needs the frame's context to send viewport commands).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Windows {
    /// Go full screen, at the remembered slides place when there is one.
    Enter,
    /// Leave full screen and go back to where the window was.
    Leave,
    /// Swap the slides and the presenter between their screens.
    Swap,
}

pub struct PresentHub {
    fabric: Fabric,
    /// Run asks from other hubs (RL) — drained every frame.
    commands: DeckCommandReceiver,
    deck: Option<dpl_deck::DeckHost>,
    /// What is running — the presenter builds from it.
    showing: Option<Showing>,
    presenter: Option<Presenter>,
    /// Harness only: render the presenter surface in the MAIN viewport
    /// (the screenshot harness captures only that window).
    presenter_panel: bool,
    /// The last open failure, shown honestly on the picker.
    error: Option<String>,
    /// A window change waiting for the next frame's context.
    windows: Option<Windows>,
    /// Where the main window was before the run took it full screen.
    before_run: Option<egui::Pos2>,
    /// The two windows' places as last seen this run.
    seen_slides: Option<egui::Rect>,
    seen_presenter: Option<egui::Rect>,
    /// This machine's remembered places, read once when a run starts.
    places: Option<Places>,
}

impl PresentHub {
    pub fn new(fabric: Fabric) -> Self {
        let commands = fabric.subscribe_deck_commands();
        Self {
            fabric,
            commands,
            deck: None,
            showing: None,
            presenter: None,
            presenter_panel: false,
            error: None,
            windows: None,
            before_run: None,
            seen_slides: None,
            seen_presenter: None,
            places: None,
        }
    }

    /// Run a lesson by its id (the `--run-lesson` harness path), from its
    /// first card, without taking the screen over (shots keep their real
    /// window size).
    pub fn run_lesson(&mut self, id: &str) {
        self.run(Showing::Lesson(id.to_owned()), 0, false);
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
        self.run(Showing::Episode(dir), 0, false);
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
        let Some(showing) = self.showing.clone() else {
            return;
        };
        // Subscribe FIRST, then ask for the snapshot — a receiver only
        // hears traffic after it subscribes.
        let rx = self.fabric.subscribe_deck();
        match Presenter::open(&showing, &dpl_engine_capture::content_root(), rx) {
            Ok(p) => {
                self.presenter = Some(p);
                if let Some(deck) = &self.deck {
                    deck.publish_position();
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// RUN (RL): open `showing` at card `start`, full-bleed. With
    /// `take_screen`, the slides also go OS full screen and the presenter
    /// opens in its own window at once (Q3, "script on laptop"); the
    /// harness paths leave the window as it is.
    fn run(&mut self, showing: Showing, start: usize, take_screen: bool) {
        let root = dpl_engine_capture::content_root();
        let looks = dpl_doc::Looks::load(&root);
        match dpl_deck::DeckHost::open(&showing, &root, looks, start) {
            Ok(mut host) => {
                host.wire_fabric(self.fabric.clone());
                self.deck = Some(host);
                self.showing = Some(showing);
                self.presenter = None;
                self.error = None;
                self.fabric
                    .publish_shell(ShellEvent::FullBleedRequest(true));
                if take_screen {
                    self.places = load_places(&places_file(), &machine());
                    self.windows = Some(Windows::Enter);
                    self.toggle_presenter();
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn close_deck(&mut self) {
        // Keep where the windows were, for the next run on this machine.
        if let (Some(slides), Some(presenter)) = (self.seen_slides, self.seen_presenter) {
            let places = Places {
                slides: slides.min,
                presenter,
            };
            if let Err(e) = save_places(&places_file(), &machine(), places) {
                self.error = Some(format!("could not remember the screens: {e}"));
            }
        }
        if self.before_run.is_some() {
            self.windows = Some(Windows::Leave);
        }
        self.deck = None;
        self.presenter = None;
        self.showing = None;
        self.fabric
            .publish_shell(ShellEvent::FullBleedRequest(false));
    }

    /// Run asks from other hubs: the latest one wins.
    fn drain_commands(&mut self) {
        let mut ask = None;
        while let Ok(command) = self.commands.try_recv() {
            ask = Some(command);
        }
        if let Some(DeckCommand::Run { target, start }) = ask {
            let showing = match target {
                RunTarget::Lesson(id) => Showing::Lesson(id),
                RunTarget::Episode(dir) => Showing::Episode(dir),
            };
            self.run(showing, start, true);
        }
    }

    /// Apply a waiting window change with this frame's context.
    fn apply_windows(&mut self, ctx: &egui::Context) {
        let Some(change) = self.windows.take() else {
            return;
        };
        let here = ctx.input(|i| i.viewport().outer_rect);
        match change {
            Windows::Enter => {
                self.before_run = here.map(|r| r.min);
                if let Some(places) = self.places {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(places.slides));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
            Windows::Leave => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                if let Some(at) = self.before_run.take() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(at));
                }
            }
            Windows::Swap => {
                if let (Some(slides), Some(presenter)) = (here, self.seen_presenter) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(presenter.min));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                    ctx.send_viewport_cmd_to(
                        presenter_viewport(),
                        egui::ViewportCommand::OuterPosition(slides.min),
                    );
                }
            }
        }
    }

    /// What the presenter asked, done to the deck.
    fn apply(&mut self, act: Option<PresenterAct>) {
        let Some(act) = act else {
            return;
        };
        match act {
            PresenterAct::Swap => self.windows = Some(Windows::Swap),
            other => {
                if let Some(deck) = self.deck.as_mut() {
                    match other {
                        PresenterAct::Next => deck.next(),
                        PresenterAct::Back => deck.back(),
                        PresenterAct::Jump(index, step) => deck.jump(index, step),
                        PresenterAct::Swap => {}
                    }
                }
            }
        }
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
                let root = dpl_engine_capture::content_root();
                for entry in dpl_doc::all_episodes(&root) {
                    let (title, count) = match dpl_doc::episode::summary(&entry.dir, &root) {
                        Some(summary) => {
                            let n = summary.cards();
                            (summary.title, n)
                        }
                        None => (entry.id.clone(), 0),
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
                                        RichText::new("Run ›").small().color(SPEC.tone.color()),
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
            self.run(Showing::Episode(dir), 0, true);
        }
    }
}

/// The presenter's own OS window.
fn presenter_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("dpl_presenter")
}

impl Hub for PresentHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        let _ = route; // one subroute — SHOWS is the whole hub.
        self.drain_commands();
        let ctx = ui.ctx().clone();
        self.apply_windows(&ctx);
        // Harness panel mode (D4 proofs): the presenter surface owns the
        // main well — the same pixels the second viewport would carry.
        if self.presenter_panel {
            if let Some(p) = self.presenter.as_mut() {
                let act = p.ui(ui);
                self.apply(act);
                return;
            }
        }
        if self.deck.is_some() {
            self.seen_slides = ctx.input(|i| i.viewport().outer_rect);
            let action = self.deck.as_mut().map(|deck| deck.ui(ui));
            match action {
                Some(dpl_deck::DeckAction::Exit) => {
                    self.close_deck();
                    return;
                }
                Some(dpl_deck::DeckAction::TogglePresenter) => self.toggle_presenter(),
                _ => {}
            }
            // The presenter rides in its own OS window while the show
            // runs on the big screen — where it sat last time on this
            // machine, else a tall window beside it.
            if let Some(p) = self.presenter.as_mut() {
                let mut builder = egui::ViewportBuilder::default()
                    .with_title("DevPULSE Presenter")
                    .with_inner_size([560.0, 720.0]);
                if let Some(places) = self.places {
                    builder = builder
                        .with_position(places.presenter.min)
                        .with_inner_size(places.presenter.size());
                }
                let mut close = false;
                let mut act = None;
                let mut seen = None;
                // Registered with the pilot as `present.presenter`, so a
                // scenario can see it open and tell it from the show.
                ctx.pilot_show_viewport_immediate(
                    egui_pilot_ai::protocol::PilotViewportKey::new("present.presenter"),
                    presenter_viewport(),
                    builder,
                    |ui, _class| {
                        act = p.ui(ui);
                        close = ui.ctx().input(|i| i.viewport().close_requested());
                        seen = ui.ctx().input(|i| i.viewport().outer_rect);
                    },
                );
                close |= ctx.pilot_take_viewport_close_request(presenter_viewport());
                if seen.is_some() {
                    self.seen_presenter = seen;
                }
                if close {
                    self.presenter = None;
                }
                self.apply(act);
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
        let showing = Showing::Episode(dir);
        let mut deck = dpl_deck::DeckHost::open(
            &showing,
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        deck.wire_fabric(fabric.clone());
        let mut p =
            Presenter::open(&showing, Path::new("/nowhere"), fabric.subscribe_deck()).unwrap();
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
        let mut p = Presenter::open(
            &Showing::Episode(dir),
            Path::new("/nowhere"),
            fabric.subscribe_deck(),
        )
        .unwrap();
        // At a/setup: next is b's opening cue (a has one step), b named by
        // its own words.
        assert_eq!(p.up_next(), "slide b \u{2014} set it up");
        // At b/setup: the next beat on the SAME slide.
        (p.index, p.step) = (1, 0);
        assert_eq!(p.up_next(), "sub \u{2014} land the reveal");
        // At b/sub: c follows, unscripted - its name, honestly.
        (p.index, p.step) = (1, 1);
        assert_eq!(p.up_next(), "slide c");
        // At c: nothing follows.
        (p.index, p.step) = (2, 0);
        assert_eq!(p.up_next(), "end of show");
        // c is unscripted - the active cue degrades honestly too.
        assert!(p.beat_at(2, 0).is_none());
    }

    /// RL "presenter drives": what the presenter asks moves the deck, and
    /// the deck's beat comes back to the presenter over the fabric.
    #[test]
    fn the_presenter_drives_the_deck_and_hears_it_back() {
        let dir = scratch("drive");
        let fabric = Fabric::new();
        let mut hub = PresentHub::new(fabric.clone());
        hub.run(Showing::Episode(dir), 0, false);
        hub.toggle_presenter();
        assert!(hub.presenter.is_some());
        let at = |hub: &PresentHub| hub.deck.as_ref().unwrap().position();
        hub.apply(Some(PresenterAct::Next));
        assert_eq!((at(&hub).index, at(&hub).step), (1, 0));
        hub.apply(Some(PresenterAct::Jump(1, 1)));
        assert_eq!((at(&hub).index, at(&hub).step), (1, 1));
        hub.apply(Some(PresenterAct::Back));
        assert_eq!((at(&hub).index, at(&hub).step), (1, 0));
        let p = hub.presenter.as_mut().unwrap();
        p.drain();
        assert_eq!((p.index, p.step), (1, 0), "the presenter follows");
        hub.apply(Some(PresenterAct::Swap));
        assert_eq!(hub.windows, Some(Windows::Swap));
    }

    /// A run asked from another hub opens here, at the card it names, and
    /// takes the screen with the presenter open.
    #[test]
    fn a_run_ask_from_another_hub_opens_the_show() {
        let dir = scratch("ask");
        let fabric = Fabric::new();
        let mut hub = PresentHub::new(fabric.clone());
        fabric.publish_deck_command(DeckCommand::Run {
            target: RunTarget::Episode(dir),
            start: 2,
        });
        hub.drain_commands();
        assert_eq!(hub.deck.as_ref().unwrap().position().index, 2);
        assert!(hub.presenter.is_some(), "the presenter opens at once");
        assert_eq!(hub.windows, Some(Windows::Enter));
    }

    /// Window places are kept per machine (R289 "attune"): saving one
    /// machine's leaves another's as it was, and an unknown machine has
    /// none.
    #[test]
    fn window_places_are_kept_per_machine() {
        let file = std::env::temp_dir()
            .join("dpl-present-tests")
            .join(format!("places-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&file);
        let a = Places {
            slides: egui::pos2(1920.0, 0.0),
            presenter: egui::Rect::from_min_size(egui::pos2(40.0, 60.0), egui::vec2(560.0, 720.0)),
        };
        let b = Places {
            slides: egui::pos2(0.0, 0.0),
            presenter: egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(500.0, 600.0)),
        };
        save_places(&file, "LAPTOP", a).unwrap();
        save_places(&file, "STUDIO", b).unwrap();
        assert_eq!(load_places(&file, "LAPTOP"), Some(a));
        assert_eq!(load_places(&file, "STUDIO"), Some(b));
        assert_eq!(load_places(&file, "ELSEWHERE"), None);
        let _ = std::fs::remove_file(&file);
    }
}
