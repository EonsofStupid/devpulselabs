//! Storage is selected before anything that could open or write library data.
//!
//! ST1 owns the first-launch choice and an honest retry screen; ST1b brings
//! today's folders into a new library on the operator's tap. ST2a: these
//! pages are the first stage of the app's one window, and a library that
//! isn't connected is looked for every two seconds and opens by itself.

use dpl_coldlight::{self as cl, Fluid};
use dpl_roots::Publish;
use egui_pilot_ai::protocol::Role;
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::Duration;

/// Open the configured library, if it is there. `main` asks before any
/// window, and the pages below ask again when the person chooses.
pub fn ready() -> Result<(), String> {
    let layout = dpl_roots::initialize()?;
    if let Some(notice) = dpl_roots::notice() {
        tracing::warn!(%notice, "library opened");
    }
    dpl_roots::ensure_rings()?;
    tracing::info!(
        library = %layout.library.display(),
        content = %layout.content.display(),
        assets = %layout.assets.display(),
        userdata = %layout.userdata.display(),
        "library ready"
    );
    Ok(())
}

/// The library pages: the first-launch choice, Bring in, the Disconnected
/// page and the honest retry. `main` shows them until `opened`.
pub struct Startup {
    /// Where `roots.toml` goes, and the program's own `library\`.
    program: PathBuf,
    opened: bool,
    setup: bool,
    /// The configured library while it isn't connected (ST2a).
    away: Option<dpl_roots::Status>,
    /// "Use a library on this machine": the first screen's choices,
    /// replacing `roots.toml` once one finishes.
    switching: bool,
    locating: bool,
    /// Where the person says the library is now (Locate…).
    place: String,
    located: Option<Result<dpl_roots::Located, String>>,
    /// Set by the watch's thread when the library is back.
    found: Arc<AtomicBool>,
    watch: Option<dpl_roots::Watch>,
    ctx: egui::Context,
    error: Option<String>,
    label: String,
    folder: String,
    linked: bool,
    bring: Option<BringIn>,
    /// Opens the library once it is chosen: `ready`, outside tests.
    open: fn() -> Result<(), String>,
    screenshot: Option<PathBuf>,
    shot_frame: u32,
    frames: u32,
}

/// "Bring in my existing library…": today's folders, copied into a new
/// library the person places (R291). The source is only ever read.
struct BringIn {
    /// Where `roots.toml` goes, and the program's own `library\`.
    program: PathBuf,
    publish: Publish,
    source: String,
    elsewhere: bool,
    folder: String,
    /// The From and destination fields are open.
    editing: bool,
    plan: Option<Result<dpl_roots::MovePlan, String>>,
    moving: Option<Moving>,
}

struct Moving {
    progress: Arc<dpl_roots::MoveProgress>,
    files: u64,
    bytes: u64,
    done: mpsc::Receiver<Result<(), String>>,
}

impl BringIn {
    fn new(publish: Publish) -> Self {
        Self {
            program: dpl_roots::program_folder().unwrap_or_default(),
            publish,
            source: dpl_roots::legacy_source()
                .map(|folder| folder.display().to_string())
                .unwrap_or_default(),
            elsewhere: false,
            folder: String::new(),
            editing: false,
            plan: None,
            moving: None,
        }
    }

    /// The program's own `library\` (a build's project), or the folder typed.
    fn library(&self) -> Result<PathBuf, String> {
        if !self.elsewhere {
            return Ok(self.program.join("library"));
        }
        let folder = Path::new(self.folder.trim());
        if folder.is_absolute() {
            Ok(folder.to_path_buf())
        } else {
            Err("Paste the full path of the folder for your library.".into())
        }
    }

    fn check(&mut self) {
        let source = Path::new(self.source.trim());
        self.plan = Some(if source.is_absolute() {
            self.library()
                .and_then(|library| dpl_roots::plan_move(source, &library))
        } else {
            Err(
                "Paste the full path of the folder that holds content, assets and .userdata."
                    .into(),
            )
        });
    }

    fn start(&mut self, label: &str, ctx: &egui::Context) {
        let Some(Ok(plan)) = &self.plan else {
            return;
        };
        let progress = Arc::new(dpl_roots::MoveProgress::default());
        let (send, done) = mpsc::channel();
        let (program, source, library, label, publish) = (
            self.program.clone(),
            plan.source.clone(),
            plan.library.clone(),
            label.to_owned(),
            self.publish,
        );
        let (counters, ctx) = (Arc::clone(&progress), ctx.clone());
        std::thread::spawn(move || {
            let result =
                dpl_roots::bring_in(&program, &source, &library, &label, &counters, publish)
                    .map(|_| ());
            let _ = send.send(result);
            ctx.request_repaint();
        });
        self.moving = Some(Moving {
            progress,
            files: plan.files(),
            bytes: plan.bytes(),
            done,
        });
    }
}

/// Bytes in the plainest unit that reads well.
fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["bytes", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} bytes")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// A full-width touch button with its label centred, greyed out when not
/// `enabled`.
fn wide(ui: &mut egui::Ui, fluid: Fluid, enabled: bool, button: egui::Button) -> egui::Response {
    let size = [ui.available_width(), fluid.tap_min()];
    ui.add_enabled_ui(enabled, |ui| ui.add_sized(size, button))
        .inner
}

/// A Disconnected-page control, named for EguiPilotAI.
fn track(ui: &mut egui::Ui, response: &egui::Response, key: &str, role: Role, name: &str) {
    ui.pilot_track(
        response,
        NodeMetadata::new(
            PilotId::new("library.away").child(&PilotKey::new(key)),
            role,
            name,
        ),
    );
}

fn files(count: u64) -> String {
    if count == 1 {
        "1 file".into()
    } else {
        format!("{count} files")
    }
}

impl Startup {
    /// `error` is why the library didn't open; `bring_in` opens "Bring in my
    /// existing library" with its plan at once.
    pub fn new(error: String, bring_in: bool, ctx: &egui::Context) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let argument = |name: &str| {
            args.iter()
                .position(|arg| arg == name)
                .and_then(|index| args.get(index + 1))
        };
        let setup = dpl_roots::needs_setup();
        let bring = (setup && bring_in).then(|| {
            let mut bring = BringIn::new(Publish::New);
            bring.check();
            bring
        });
        let away = dpl_roots::status();
        let linked = away.as_ref().is_some_and(|status| status.linked);
        let mut startup = Self {
            program: dpl_roots::program_folder().unwrap_or_default(),
            opened: false,
            setup,
            away,
            switching: false,
            locating: false,
            place: String::new(),
            located: None,
            found: Arc::new(AtomicBool::new(false)),
            watch: None,
            ctx: ctx.clone(),
            // The Disconnected page says it plainly; anything else is shown.
            error: (!setup && !linked).then_some(error),
            label: "My library".into(),
            folder: String::new(),
            linked: false,
            bring,
            open: ready,
            screenshot: argument("--screenshot").map(PathBuf::from),
            shot_frame: argument("--shot-frame")
                .and_then(|value| value.parse().ok())
                .unwrap_or(6),
            frames: 0,
        };
        if !setup {
            startup.look();
        }
        startup
    }

    /// True once a library is open and the app can start.
    pub fn opened(&self) -> bool {
        self.opened
    }

    /// Look for the library every two seconds until it is back.
    fn look(&mut self) {
        let (found, ctx) = (Arc::clone(&self.found), self.ctx.clone());
        match dpl_roots::watch(move || {
            found.store(true, Ordering::Release);
            ctx.request_repaint();
        }) {
            Ok(watch) => self.watch = Some(watch),
            Err(error) => tracing::warn!(%error, "the library won't be looked for by itself"),
        }
    }

    fn publish(&self) -> Publish {
        if self.switching {
            Publish::Replace
        } else {
            Publish::New
        }
    }

    fn finish(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.opened = true;
                self.watch = None;
            }
            Err(error) => {
                tracing::warn!(%error, "library startup refused");
                self.error = Some(error);
                if !self.switching {
                    self.setup = dpl_roots::needs_setup();
                }
            }
        }
    }

    fn choice(&mut self, ui: &mut egui::Ui, fluid: Fluid) {
        let build = dpl_roots::is_build();
        if self.bring.is_some() {
            self.bring_in(ui, fluid, build);
            return;
        }
        if self.switching {
            let label = self
                .away
                .as_ref()
                .map(|status| status.label.clone())
                .unwrap_or_default();
            if cl::tip(
                ui.add(egui::Button::new("‹ Back").min_size(egui::vec2(0.0, fluid.tap_min()))),
                "Go back and wait for your library to be plugged in.",
            )
            .clicked()
            {
                self.switching = false;
                self.setup = false;
                self.error = None;
                self.look();
                return;
            }
            ui.add_space(fluid.sp_3());
            cl::card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new(format!(
                        "'{label}' stays on its drive, unchanged. The library you choose here opens instead, and DevPULSE stops looking for '{label}'."
                    ))
                    .size(fluid.fs_body())
                    .color(cl::ink::BASE),
                );
            });
            ui.add_space(fluid.sp_3());
        }
        let home = self.program.join("library").display().to_string();
        ui.label(
            egui::RichText::new("Where should your library live?")
                .size(fluid.fs_h1())
                .color(cl::ink::STRONG),
        );
        ui.label(
            egui::RichText::new(
                "Your content, working files and app data stay together in one folder.",
            )
            .size(fluid.fs_body_lg()),
        );
        ui.add_space(fluid.sp_4());

        ui.label("Library name");
        cl::tip(
            ui.add_sized(
                [ui.available_width(), fluid.tap_min()],
                egui::TextEdit::singleline(&mut self.label)
                    .hint_text("My library")
                    .vertical_align(egui::Align::Center),
            ),
            "Name a new library. An existing library keeps its own name.",
        );
        ui.add_space(fluid.sp_3());

        let (keep, keep_tip) = if build {
            (
                "Keep my library in this project",
                format!("Create a new, empty library at {home}, inside this project and out of reach of a build clean."),
            )
        } else {
            (
                "Keep my library with the program",
                format!(
                    "Create a new, empty library at {home}. It travels with the program folder."
                ),
            )
        };
        let imported = cl::tip(
            wide(
                ui,
                fluid,
                !self.label.trim().is_empty(),
                egui::Button::new(keep),
            ),
            &keep_tip,
        );
        if imported.clicked() {
            let result =
                dpl_roots::setup_imported(&self.program, self.label.trim(), self.publish())
                    .and_then(|_| (self.open)());
            self.finish(result);
        }
        if cl::tip(
            ui.add_sized(
                [ui.available_width(), fluid.tap_min()],
                egui::Button::new("My library is on a drive or folder…"),
            ),
            "Open a library in place, or create one in an empty folder you choose.",
        )
        .clicked()
        {
            self.linked = !self.linked;
        }
        if self.linked {
            ui.add_space(fluid.sp_3());
            ui.label("Full folder path");
            cl::tip(
                ui.add_sized(
                    [ui.available_width(), fluid.tap_min()],
                    egui::TextEdit::singleline(&mut self.folder)
                        .hint_text("Paste the library folder's full path")
                        .vertical_align(egui::Align::Center),
                ),
                "Choose an existing DevPULSE library or an empty folder. Existing files are not moved.",
            );
            if cl::tip(
                wide(
                    ui,
                    fluid,
                    !self.folder.trim().is_empty() && !self.label.trim().is_empty(),
                    egui::Button::new("Use this folder"),
                ),
                "Verify this library, or create a new one in the empty folder, then open it.",
            )
            .clicked()
            {
                let folder = Path::new(self.folder.trim());
                let result = if folder.is_absolute() {
                    dpl_roots::setup_linked(
                        &self.program,
                        folder,
                        self.label.trim(),
                        self.publish(),
                    )
                    .and_then(|_| (self.open)())
                } else {
                    Err("Paste the full path to your library folder.".into())
                };
                self.finish(result);
            }
        }
        if cl::tip(
            ui.add_sized(
                [ui.available_width(), fluid.tap_min()],
                egui::Button::new("Bring in my existing library…"),
            ),
            "Copy the content, assets and .userdata folders you already have into a new library. Your folders aren't changed.",
        )
        .clicked()
        {
            let mut bring = BringIn::new(self.publish());
            bring.check();
            self.bring = Some(bring);
            self.linked = false;
        }
    }

    fn bring_in(&mut self, ui: &mut egui::Ui, fluid: Fluid, build: bool) {
        if let Some(moving) = self.bring.as_mut().and_then(|bring| bring.moving.as_ref()) {
            let copied = moving.progress.copied_files.load(Ordering::Relaxed);
            let checked = moving.progress.checked_files.load(Ordering::Relaxed);
            let total = moving.files.max(1) as f32;
            let (line, fraction) = if copied < moving.files {
                (
                    format!(
                        "Copying {copied} of {} ({} of {})…",
                        files(moving.files),
                        size(moving.progress.copied_bytes.load(Ordering::Relaxed)),
                        size(moving.bytes)
                    ),
                    copied as f32 / total / 2.0,
                )
            } else {
                (
                    format!(
                        "Checking {checked} of {}, byte for byte…",
                        files(moving.files)
                    ),
                    0.5 + checked as f32 / total / 2.0,
                )
            };
            ui.label(
                egui::RichText::new("Bringing in your library")
                    .size(fluid.fs_h1())
                    .color(cl::ink::STRONG),
            );
            ui.add_space(fluid.sp_3());
            ui.label(egui::RichText::new(line).size(fluid.fs_body_lg()));
            ui.add(egui::ProgressBar::new(fraction));
            let landed = match moving.done.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                    "The move stopped before it finished. Your folders are as they were.".into(),
                )),
            };
            match landed {
                Some(result) => {
                    if let Some(bring) = self.bring.as_mut() {
                        bring.moving = None;
                    }
                    self.finish(result.and_then(|()| (self.open)()));
                }
                None => ui.ctx().request_repaint_after(Duration::from_millis(100)),
            }
            return;
        }

        let back = cl::tip(
            ui.add(egui::Button::new("‹ Other choices").min_size(egui::vec2(0.0, fluid.tap_min()))),
            "Go back to the other ways to set up your library.",
        )
        .clicked();
        ui.add_space(fluid.sp_3());
        ui.label(
            egui::RichText::new("Bring in my existing library")
                .size(fluid.fs_h1())
                .color(cl::ink::STRONG),
        );
        ui.label(
            egui::RichText::new(
                "Your folders are copied into a new library and checked file by file. They stay exactly as they are.",
            )
            .size(fluid.fs_body_lg()),
        );
        ui.add_space(fluid.sp_4());
        ui.label("Library name");
        cl::tip(
            ui.add_sized(
                [ui.available_width(), fluid.tap_min()],
                egui::TextEdit::singleline(&mut self.label)
                    .hint_text("My library")
                    .vertical_align(egui::Align::Center),
            ),
            "Name the new library.",
        );
        let label = self.label.trim().to_owned();
        let Some(bring) = self.bring.as_mut() else {
            return;
        };
        let here = if build {
            "in this project"
        } else {
            "with the program"
        };

        // What will happen, in one card: from where, into where, and what.
        ui.add_space(fluid.sp_3());
        cl::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let line = |ui: &mut egui::Ui, text: String, ink: egui::Color32| {
                ui.label(egui::RichText::new(text).size(fluid.fs_body()).color(ink));
            };
            match &bring.plan {
                Some(Ok(plan)) => {
                    let place = if bring.elsewhere {
                        "a folder you chose"
                    } else {
                        here
                    };
                    line(
                        ui,
                        format!("From  {}", plan.source.display()),
                        cl::ink::BASE,
                    );
                    line(
                        ui,
                        format!("Into  {} ({place})", plan.library.display()),
                        cl::ink::BASE,
                    );
                    ui.add_space(fluid.sp_2());
                    for item in &plan.items {
                        line(
                            ui,
                            format!(
                                "{} — {}, {}",
                                item.ring,
                                files(item.files),
                                size(item.bytes)
                            ),
                            cl::ink::STRONG,
                        );
                    }
                    ui.add_space(fluid.sp_2());
                    line(
                        ui,
                        format!(
                            "{} and {} in all. Close any other DevPULSE window before you move.",
                            files(plan.files()),
                            size(plan.bytes())
                        ),
                        cl::ink::MUTED,
                    );
                }
                Some(Err(error)) => line(ui, error.clone(), cl::ink::BASE),
                None => line(
                    ui,
                    "Tap Check what moves to see what comes in.".into(),
                    cl::ink::MUTED,
                ),
            }
        });
        ui.add_space(fluid.sp_3());
        if matches!(bring.plan, Some(Ok(_)))
            && cl::tip(
                wide(
                    ui,
                    fluid,
                    !label.is_empty(),
                    egui::Button::new("Move it").selected(true),
                ),
                "Copy these folders into the new library, check every file, then open it. Nothing is deleted.",
            )
            .clicked()
        {
            bring.start(&label, ui.ctx());
        }
        let fix = !matches!(bring.plan, Some(Ok(_)));
        if !fix
            && cl::tip(
                ui.add_sized(
                    [ui.available_width(), fluid.tap_min()],
                    egui::Button::new("Change where…").selected(bring.editing),
                ),
                "Bring in from another folder, or put the library somewhere else.",
            )
            .clicked()
        {
            bring.editing = !bring.editing;
        }
        if bring.editing || fix {
            ui.add_space(fluid.sp_3());
            ui.label("From");
            let mut changed = cl::tip(
                ui.add_sized(
                    [ui.available_width(), fluid.tap_min()],
                    egui::TextEdit::singleline(&mut bring.source)
                        .hint_text("The folder that holds content, assets and .userdata")
                        .vertical_align(egui::Align::Center),
                ),
                "The folder that holds your content, assets and .userdata folders. It is only read.",
            )
            .changed();
            ui.add_space(fluid.sp_3());
            ui.label("Where your library goes");
            let first = if build {
                "In this project"
            } else {
                "With the program"
            };
            ui.horizontal_wrapped(|ui| {
                if cl::tip(
                    ui.add(
                        egui::Button::new(first)
                            .selected(!bring.elsewhere)
                            .min_size(egui::vec2(0.0, fluid.tap_min())),
                    ),
                    "Put the library in this program's library folder.",
                )
                .clicked()
                {
                    changed |= bring.elsewhere;
                    bring.elsewhere = false;
                }
                if cl::tip(
                    ui.add(
                        egui::Button::new("A drive or folder…")
                            .selected(bring.elsewhere)
                            .min_size(egui::vec2(0.0, fluid.tap_min())),
                    ),
                    "Put the library in a new or empty folder you choose, on any drive.",
                )
                .clicked()
                {
                    changed |= !bring.elsewhere;
                    bring.elsewhere = true;
                }
            });
            if bring.elsewhere {
                changed |= cl::tip(
                    ui.add_sized(
                        [ui.available_width(), fluid.tap_min()],
                        egui::TextEdit::singleline(&mut bring.folder)
                            .hint_text("Paste a new or empty folder's full path")
                            .vertical_align(egui::Align::Center),
                    ),
                    "A folder that doesn't exist yet, or is empty. The library is made there.",
                )
                .changed();
            }
            if changed {
                bring.plan = None;
            }
            ui.add_space(fluid.sp_3());
            if cl::tip(
                ui.add_sized(
                    [ui.available_width(), fluid.tap_min()],
                    egui::Button::new("Check what moves"),
                ),
                "List each folder and its size. Nothing is copied yet.",
            )
            .clicked()
            {
                bring.check();
            }
        }
        if back {
            self.bring = None;
        }
    }

    /// A linked library that isn't plugged in (ST2a). It is looked for
    /// every two seconds and opens by itself; Locate… and another library are
    /// the person's other ways on.
    fn disconnected(&mut self, ui: &mut egui::Ui, fluid: Fluid, status: &dpl_roots::Status) {
        let title = format!("Your library '{}' isn't connected", status.label);
        let heading = ui.label(
            egui::RichText::new(&title)
                .size(fluid.fs_h1())
                .color(cl::ink::STRONG),
        );
        track(ui, &heading, "heading", Role::Heading, &title);
        ui.label(
            egui::RichText::new("Plug it in and it reconnects by itself, on any drive letter.")
                .size(fluid.fs_body_lg()),
        );
        ui.add_space(fluid.sp_2());
        ui.label(
            egui::RichText::new(format!("Last seen at {}", status.last_path.display()))
                .size(fluid.fs_body())
                .color(cl::ink::MUTED),
        );
        ui.label(
            egui::RichText::new("Looking for it every few seconds…")
                .size(fluid.fs_body())
                .color(cl::ink::MUTED),
        );
        ui.add_space(fluid.sp_4());
        let locate = ui.add_sized(
            [ui.available_width(), fluid.tap_min()],
            egui::Button::new("Locate…").selected(self.locating),
        );
        track(ui, &locate, "locate", Role::Button, "Locate…");
        if cl::tip(
            locate,
            "Point at where your library's folder is now, on another drive or in another folder.",
        )
        .clicked()
        {
            self.locating = !self.locating;
            self.located = None;
        }
        if self.locating {
            ui.add_space(fluid.sp_3());
            ui.label("Where it is now");
            let place = ui.add_sized(
                [ui.available_width(), fluid.tap_min()],
                egui::TextEdit::singleline(&mut self.place)
                    .hint_text("Paste the full path of the folder that holds library.json")
                    .vertical_align(egui::Align::Center),
            );
            track(ui, &place, "place", Role::TextInput, "Where it is now");
            if cl::tip(
                place,
                "The library's own folder: the one with library.json in it. It is only read.",
            )
            .changed()
            {
                self.located = None;
            }
            ui.add_space(fluid.sp_3());
            let check = wide(
                ui,
                fluid,
                !self.place.trim().is_empty(),
                egui::Button::new("Check"),
            );
            track(ui, &check, "check", Role::Button, "Check");
            if cl::tip(check, "Read the library there. If it's yours, it opens.").clicked() {
                let located = dpl_roots::locate(&self.program, Path::new(self.place.trim()));
                if let Ok(dpl_roots::Located::Same(library)) = &located {
                    let result =
                        dpl_roots::link_to(&self.program, library).and_then(|()| (self.open)());
                    self.finish(result);
                }
                self.located = Some(located);
            }
            let mut open_other = None;
            match &self.located {
                Some(Ok(dpl_roots::Located::Other { library, label })) => {
                    ui.add_space(fluid.sp_3());
                    cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            egui::RichText::new(format!(
                                "That's '{label}', a different library. Open it instead? '{}' stays on its drive, and DevPULSE stops looking for it.",
                                status.label
                            ))
                            .size(fluid.fs_body())
                            .color(cl::ink::BASE),
                        );
                    });
                    ui.add_space(fluid.sp_3());
                    let open = format!("Open '{label}'");
                    let other = wide(ui, fluid, true, egui::Button::new(&open));
                    track(ui, &other, "other", Role::Button, &open);
                    if cl::tip(
                        other,
                        "Use this library from now on. Nothing in either library changes.",
                    )
                    .clicked()
                    {
                        open_other = Some(library.clone());
                    }
                }
                Some(Err(error)) => {
                    ui.add_space(fluid.sp_3());
                    cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            egui::RichText::new(error)
                                .size(fluid.fs_body())
                                .color(cl::ink::BASE),
                        );
                    });
                }
                _ => {}
            }
            if let Some(library) = open_other {
                let result =
                    dpl_roots::link_to(&self.program, &library).and_then(|()| (self.open)());
                self.finish(result);
            }
        }
        ui.add_space(fluid.sp_3());
        let here = ui.add_sized(
            [ui.available_width(), fluid.tap_min()],
            egui::Button::new("Use a library on this machine"),
        );
        track(
            ui,
            &here,
            "here",
            Role::Button,
            "Use a library on this machine",
        );
        if cl::tip(
            here,
            "Keep a library with the program, use another drive or folder, or bring in your existing folders. Your library on the drive is left as it is.",
        )
        .clicked()
        {
            self.switching = true;
            self.setup = true;
            self.locating = false;
            self.error = None;
            self.watch = None;
        }
    }

    fn unavailable(&mut self, ui: &mut egui::Ui, fluid: Fluid) {
        ui.label(
            egui::RichText::new("Your library couldn't be opened")
                .size(fluid.fs_h1())
                .color(cl::ink::STRONG),
        );
        ui.label(
            egui::RichText::new("If your library is on a drive, connect it, then try again.")
                .size(fluid.fs_body_lg()),
        );
        ui.add_space(fluid.sp_4());
        if cl::tip(
            ui.add_sized(
                [ui.available_width(), fluid.tap_min()],
                egui::Button::new("Try again"),
            ),
            "Read your library's saved location and check its identity again.",
        )
        .clicked()
        {
            let result = (self.open)();
            self.finish(result);
        }
    }
}

impl eframe::App for Startup {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.found.swap(false, Ordering::AcqRel) && !self.opened {
            let result = (self.open)();
            self.finish(result);
        }
        crate::fit_window(ui.ctx());
        let fluid = Fluid::of(ui.max_rect());
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(fluid.sp_6());
            let width = ui.available_width().min(fluid.panel_width() * 2.0);
            let inset = ((ui.available_width() - width) / 2.0).max(fluid.sp_4());
            ui.horizontal(|ui| {
                ui.add_space(inset);
                ui.vertical(|ui| {
                    ui.set_width((width - fluid.sp_4() * 2.0).max(0.0));
                    ui.label(
                        egui::RichText::new("DEVPULSE LABS")
                            .size(fluid.fs_eyebrow())
                            .color(cl::Tone::Cyan.color()),
                    );
                    ui.add_space(fluid.sp_4());
                    let away = self.away.clone().filter(|status| status.linked);
                    if self.setup {
                        self.choice(ui, fluid);
                    } else if let Some(status) = away {
                        self.disconnected(ui, fluid, &status);
                    } else {
                        self.unavailable(ui, fluid);
                    }
                    if let Some(error) = &self.error {
                        ui.add_space(fluid.sp_3());
                        cl::card_frame().show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(error)
                                    .size(fluid.fs_body())
                                    .color(cl::ink::BASE),
                            );
                        });
                    }
                    ui.add_space(fluid.sp_3());
                    if cl::tip(
                        ui.add_sized(
                            [ui.available_width(), fluid.tap_min()],
                            egui::Button::new("Quit"),
                        ),
                        "Close DevPULSE Labs.",
                    )
                    .clicked()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
            ui.add_space(fluid.sp_6());
        });
        if let Some(ok) = crate::screenshot::frame(
            ui.ctx(),
            self.screenshot.as_deref(),
            &mut self.frames,
            self.shot_frame,
        ) {
            std::process::exit(i32::from(!ok));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use std::fs;
    use std::sync::atomic::AtomicU64;

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    /// A fresh scratch folder.
    fn folder(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dpl-startup-{}-{nanos}-{}-{name}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// A scratch program folder holding today's three rings.
    fn scratch() -> PathBuf {
        let program = folder("program");
        fs::create_dir_all(program.join("content/library")).unwrap();
        fs::create_dir_all(program.join("assets/captures")).unwrap();
        fs::create_dir_all(program.join(".userdata/profiles")).unwrap();
        fs::write(program.join("content/library/slide.json"), b"{}").unwrap();
        fs::write(program.join("assets/captures/shot.png"), b"png").unwrap();
        fs::write(program.join(".userdata/index.db"), b"index").unwrap();
        program
    }

    /// The pages for `program`; opening a library only says so, so no test
    /// touches the process-wide library handle.
    fn page(program: &Path, setup: bool, away: Option<dpl_roots::Status>) -> Startup {
        Startup {
            program: program.to_path_buf(),
            opened: false,
            setup,
            away,
            switching: false,
            locating: false,
            place: String::new(),
            located: None,
            found: Arc::new(AtomicBool::new(false)),
            watch: None,
            ctx: egui::Context::default(),
            error: None,
            label: "My library".into(),
            folder: String::new(),
            linked: false,
            bring: None,
            open: || Ok(()),
            screenshot: None,
            shot_frame: 0,
            frames: 0,
        }
    }

    fn bring(program: &Path) -> BringIn {
        BringIn {
            program: program.to_path_buf(),
            publish: Publish::New,
            source: program.display().to_string(),
            elsewhere: false,
            folder: String::new(),
            editing: false,
            plan: None,
            moving: None,
        }
    }

    fn pages(startup: Startup) -> egui_kittest::Harness<'static, Startup> {
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(1100.0, 700.0))
            .build_ui_state(
                |ui, startup: &mut Startup| {
                    if startup.found.swap(false, Ordering::AcqRel) && !startup.opened {
                        let result = (startup.open)();
                        startup.finish(result);
                    }
                    let fluid = Fluid::of(ui.max_rect());
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let away = startup.away.clone().filter(|status| status.linked);
                        if startup.setup {
                            startup.choice(ui, fluid);
                        } else if let Some(status) = away {
                            startup.disconnected(ui, fluid, &status);
                        }
                    });
                },
                startup,
            )
    }

    /// `program` linked to a library named "DevPULSE drive", then unplugged:
    /// the page's status and where the library went.
    fn unplugged(program: &Path) -> (dpl_roots::Status, PathBuf) {
        let drive = folder("drive").join("library");
        dpl_roots::setup_linked(program, &drive, "DevPULSE drive", Publish::New).unwrap();
        let away = drive.with_file_name("moved");
        fs::rename(&drive, &away).unwrap();
        let status = dpl_roots::Status {
            label: "DevPULSE drive".into(),
            linked: true,
            last_path: drive,
            reason: "not connected".into(),
        };
        (status, away)
    }

    fn library_of(program: &Path) -> PathBuf {
        dpl_roots::Roots::open(program, None)
            .layout()
            .unwrap()
            .library
    }

    /// The page the way the operator meets it: the plan is shown, Move it
    /// copies the folders into the program's library, writes `roots.toml`
    /// and opens; the folders it came from are unchanged.
    #[test]
    fn move_it_brings_the_folders_in_and_opens() {
        let program = scratch();
        let mut bring = bring(&program);
        bring.check();
        let mut startup = page(&program, true, None);
        startup.bring = Some(bring);
        let mut harness = pages(startup);
        harness.run_steps(2);
        assert!(harness
            .query_by_label("content — 1 file, 2 bytes")
            .is_some());
        harness.get_by_label("Move it").click();
        for _ in 0..500 {
            harness.run_steps(1);
            if harness.state().opened {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(harness.state().opened, "{:?}", harness.state().error);
        let library = program.join("library");
        assert!(program.join(dpl_roots::CONFIG_FILE).is_file());
        assert!(library.join(dpl_roots::MARKER_FILE).is_file());
        assert_eq!(
            fs::read(library.join("accounts/me/content/library/slide.json")).unwrap(),
            b"{}"
        );
        assert_eq!(
            fs::read(program.join("content/library/slide.json")).unwrap(),
            b"{}"
        );
        assert_eq!(
            fs::read(program.join(".userdata/index.db")).unwrap(),
            b"index"
        );
    }

    /// A destination that already holds files is refused before anything
    /// is copied, and the fields open so the person can choose again.
    #[test]
    fn a_busy_destination_is_named_and_nothing_moves() {
        let program = scratch();
        fs::create_dir_all(program.join("library")).unwrap();
        fs::write(program.join("library/notes.txt"), b"mine").unwrap();
        let mut bring = bring(&program);
        bring.check();
        assert!(matches!(&bring.plan, Some(Err(e)) if e.contains("already holds files")));
        assert!(!program.join(dpl_roots::CONFIG_FILE).exists());
        assert_eq!(
            fs::read(program.join("library/notes.txt")).unwrap(),
            b"mine"
        );
    }

    /// The Disconnected page names the library and opens the app with no
    /// tap once the watch finds it.
    #[test]
    fn the_library_opens_by_itself_when_it_is_found() {
        let program = folder("program");
        let (status, _) = unplugged(&program);
        let mut harness = pages(page(&program, false, Some(status)));
        harness.run_steps(2);
        assert!(harness
            .query_by_label("Your library 'DevPULSE drive' isn't connected")
            .is_some());
        assert!(!harness.state().opened);
        harness.state().found.store(true, Ordering::Release);
        harness.run_steps(1);
        assert!(harness.state().opened);
    }

    /// Locate… on the library itself opens it where it is now; on another
    /// library it asks before anything changes.
    #[test]
    fn locate_opens_this_library_and_asks_about_another() {
        let program = folder("program");
        let (status, moved) = unplugged(&program);
        let config = fs::read(program.join(dpl_roots::CONFIG_FILE)).unwrap();
        let other = folder("other").join("library");
        dpl_roots::setup_linked(&folder("elsewhere"), &other, "Other", Publish::New).unwrap();

        let mut harness = pages(page(&program, false, Some(status.clone())));
        harness.run_steps(1);
        harness.get_by_label("Locate…").click();
        harness.run_steps(1);
        harness.state_mut().place = other.display().to_string();
        harness.get_by_label("Check").click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Open 'Other'").is_some());
        assert!(!harness.state().opened);
        assert_eq!(
            fs::read(program.join(dpl_roots::CONFIG_FILE)).unwrap(),
            config,
            "nothing changes before the person says so"
        );
        harness.get_by_label("Open 'Other'").click();
        harness.run_steps(1);
        assert!(harness.state().opened, "{:?}", harness.state().error);
        assert_eq!(library_of(&program), other);

        let program = folder("program");
        let (status, moved_too) = unplugged(&program);
        let mut harness = pages(page(&program, false, Some(status)));
        harness.run_steps(1);
        harness.get_by_label("Locate…").click();
        harness.run_steps(1);
        harness.state_mut().place = moved_too.display().to_string();
        harness.get_by_label("Check").click();
        harness.run_steps(1);
        assert!(harness.state().opened, "{:?}", harness.state().error);
        assert_eq!(library_of(&program), moved_too);
        assert!(moved.join(dpl_roots::MARKER_FILE).is_file());
    }

    /// "Use a library on this machine" shows the first screen's choices and
    /// replaces `roots.toml` only once one of them finishes.
    #[test]
    fn another_library_here_replaces_roots_toml_only_once_chosen() {
        let program = folder("program");
        let (status, moved) = unplugged(&program);
        let config = fs::read(program.join(dpl_roots::CONFIG_FILE)).unwrap();
        let mut harness = pages(page(&program, false, Some(status)));
        harness.run_steps(1);
        harness
            .get_by_label("Use a library on this machine")
            .click();
        harness.run_steps(2);
        assert!(harness.state().setup && harness.state().switching);
        assert_eq!(
            fs::read(program.join(dpl_roots::CONFIG_FILE)).unwrap(),
            config
        );
        harness
            .get_by_label("Keep my library with the program")
            .click();
        harness.run_steps(1);
        assert!(harness.state().opened, "{:?}", harness.state().error);
        assert_eq!(library_of(&program), program.join("library"));
        assert!(
            moved.join(dpl_roots::MARKER_FILE).is_file(),
            "the drive's library is left as it was"
        );
    }
}
