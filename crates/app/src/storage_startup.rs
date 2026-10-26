//! Storage is selected before anything that could open or write library data.
//!
//! ST1 owns the first-launch choice and an honest retry screen. ST2 will add
//! live disconnection and automatic reconnection; this screen never polls.

use dpl_coldlight::{self as cl, Fluid};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Returns false when the operator closes startup without opening a library.
pub fn prepare() -> eframe::Result<bool> {
    let error = match ready() {
        Ok(()) => return Ok(true),
        Err(error) => error,
    };
    tracing::warn!(%error, "library startup needs attention");
    let completed = Arc::new(AtomicBool::new(false));
    let result = Arc::clone(&completed);
    let mut options = crate::native_options();
    // eframe caches and reuses winit's event loop for run-on-demand. Once
    // setup closes successfully, main opens the full app in this process.
    options.run_and_return = true;
    eframe::run_native(
        "DevPULSE Labs — your library",
        options,
        Box::new(move |cc| {
            cl::apply_theme(&cc.egui_ctx);
            egui_pilot_ai::install(&cc.egui_ctx);
            Ok(Box::new(Startup::new(error, result)))
        }),
    )?;
    Ok(completed.load(Ordering::Acquire))
}

fn ready() -> Result<(), String> {
    let layout = dpl_roots::initialize()?;
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

struct Startup {
    completed: Arc<AtomicBool>,
    setup: bool,
    error: Option<String>,
    label: String,
    folder: String,
    linked: bool,
    screenshot: Option<PathBuf>,
    shot_frame: u32,
    frames: u32,
}

impl Startup {
    fn new(error: String, completed: Arc<AtomicBool>) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let argument = |name: &str| {
            args.iter()
                .position(|arg| arg == name)
                .and_then(|index| args.get(index + 1))
        };
        let setup = dpl_roots::needs_setup();
        Self {
            completed,
            setup,
            error: (!setup).then_some(error),
            label: "My library".into(),
            folder: String::new(),
            linked: false,
            screenshot: argument("--screenshot").map(PathBuf::from),
            shot_frame: argument("--shot-frame")
                .and_then(|value| value.parse().ok())
                .unwrap_or(6),
            frames: 0,
        }
    }

    fn finish(&mut self, result: Result<(), String>, ctx: &egui::Context) {
        match result {
            Ok(()) => {
                self.completed.store(true, Ordering::Release);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => {
                tracing::warn!(%error, "library startup refused");
                self.error = Some(error);
                self.setup = dpl_roots::needs_setup();
            }
        }
    }

    fn choice(&mut self, ui: &mut egui::Ui, fluid: Fluid) {
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
                egui::TextEdit::singleline(&mut self.label).hint_text("My library"),
            ),
            "Name a new library. An existing library keeps its own name.",
        );
        ui.add_space(fluid.sp_3());

        let imported = cl::tip(
            ui.add_enabled(
                !self.label.trim().is_empty(),
                egui::Button::new("Keep my library with the program")
                    .min_size(egui::vec2(ui.available_width(), fluid.tap_min())),
            ),
            "Create a library folder beside the program. It travels with the program folder.",
        );
        if imported.clicked() {
            let result = dpl_roots::setup_imported(self.label.trim()).and_then(|_| ready());
            self.finish(result, ui.ctx());
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
                        .hint_text("Paste the library folder's full path"),
                ),
                "Choose an existing DevPULSE library or an empty folder. Existing files are not moved.",
            );
            if cl::tip(
                ui.add_enabled(
                    !self.folder.trim().is_empty() && !self.label.trim().is_empty(),
                    egui::Button::new("Use this folder")
                        .min_size(egui::vec2(ui.available_width(), fluid.tap_min())),
                ),
                "Verify this library, or create a new one in the empty folder, then open it.",
            )
            .clicked()
            {
                let folder = Path::new(self.folder.trim());
                let result = if folder.is_absolute() {
                    dpl_roots::setup_linked(folder, self.label.trim()).and_then(|_| ready())
                } else {
                    Err("Paste the full path to your library folder.".into())
                };
                self.finish(result, ui.ctx());
            }
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
            self.finish(ready(), ui.ctx());
        }
    }
}

impl eframe::App for Startup {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
                    if self.setup {
                        self.choice(ui, fluid);
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
