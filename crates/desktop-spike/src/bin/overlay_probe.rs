//! Does a window show the desktop through it on this machine?
//!
//! `docs-history-2026-10-12:docs/plans/lesson-overlay-spike.md` §3. Measurement says no on X11 with
//! wgpu — Vulkan offers only opaque and inherit composite alpha, and
//! `egui-wgpu` asks for pre- or post-multiplied. This probe is the real
//! artifact that confirms or refutes that reading, because a design this
//! load-bearing should not rest on source reading alone.
//!
//! It clears to fully transparent and paints two opaque marks. A screen
//! capture then reads unambiguously: desktop inside the ring means alpha
//! works, a filled interior means it does not.
//!
//! Self-terminating by construction. An always-on-top click-through window
//! that outlives its probe is an operator hazard, so `--seconds` is honoured
//! even if nothing else works.

use std::time::{Duration, Instant};

fn main() -> eframe::Result<()> {
    let options = ProbeOptions::from_args();
    println!("overlay-probe: {options:?}");

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("DevPULSE overlay probe")
        .with_inner_size([options.width, options.height])
        .with_decorations(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_transparent(options.transparent)
        .with_mouse_passthrough(options.passthrough);
    if let Some(position) = options.position {
        viewport = viewport.with_position(position);
    }

    eframe::run_native(
        "overlay-probe",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| {
            report_renderer(cc);
            Ok(Box::new(Probe {
                options,
                started: Instant::now(),
            }))
        }),
    )
}

/// What we can observe about the renderer from inside eframe. The surface's
/// chosen alpha mode is not exposed by `RenderState`, so the honest report is
/// the adapter and backend we got; the capture answers the rest.
fn report_renderer(cc: &eframe::CreationContext<'_>) {
    match &cc.wgpu_render_state {
        Some(state) => {
            let info = state.adapter.get_info();
            println!(
                "renderer: wgpu · backend {:?} · adapter {} ({:?})",
                info.backend, info.name, info.device_type
            );
        }
        None => println!("renderer: not wgpu (glow build)"),
    }
}

struct Probe {
    options: ProbeOptions,
    started: Instant,
}

impl eframe::App for Probe {
    /// Fully transparent. If the surface honours alpha, the window's interior
    /// is the desktop; if it does not, this reads as opaque black and the
    /// capture says so.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let remaining = self
            .options
            .seconds
            .saturating_sub(self.started.elapsed().as_secs());
        if remaining == 0 {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        let rect = ui.max_rect();
        let painter = ui.painter();

        // Two opaque marks and nothing else: everything between them is the
        // question this probe asks.
        painter.circle_stroke(
            rect.center(),
            rect.height().min(rect.width()) * 0.35,
            egui::Stroke::new(6.0, egui::Color32::from_rgb(0, 229, 255)),
        );
        let card = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + 34.0),
            egui::vec2(rect.width() * 0.8, 44.0),
        );
        painter.rect_filled(card, 8.0, egui::Color32::from_rgb(12, 14, 20));
        painter.text(
            card.center(),
            egui::Align2::CENTER_CENTER,
            format!(
                "transparent={} passthrough={} · {remaining}s",
                self.options.transparent, self.options.passthrough
            ),
            egui::FontId::proportional(15.0),
            egui::Color32::from_rgb(0, 229, 255),
        );

        // The countdown must advance even when nothing else asks to paint.
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

#[derive(Debug, Clone)]
struct ProbeOptions {
    width: f32,
    height: f32,
    position: Option<[f32; 2]>,
    seconds: u64,
    transparent: bool,
    passthrough: bool,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            width: 480.0,
            height: 360.0,
            position: None,
            seconds: 6,
            transparent: true,
            passthrough: true,
        }
    }
}

impl ProbeOptions {
    /// `--size WxH --at X,Y --seconds N --opaque --no-passthrough`.
    ///
    /// `--opaque` exists so the same binary can produce the control image: if
    /// the opaque and transparent runs capture identically, alpha is ignored.
    fn from_args() -> Self {
        let mut options = Self::default();
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--size" => {
                    if let Some((width, height)) =
                        arguments.next().and_then(|v| parse_pair(&v, 'x'))
                    {
                        options.width = width;
                        options.height = height;
                    }
                }
                "--at" => {
                    options.position = arguments
                        .next()
                        .and_then(|v| parse_pair(&v, ','))
                        .map(|(x, y)| [x, y]);
                }
                "--seconds" => {
                    if let Some(seconds) = arguments.next().and_then(|v| v.parse().ok()) {
                        options.seconds = seconds;
                    }
                }
                "--opaque" => options.transparent = false,
                "--no-passthrough" => options.passthrough = false,
                other => eprintln!("overlay-probe: ignoring unknown argument `{other}`"),
            }
        }
        options
    }
}

fn parse_pair(value: &str, separator: char) -> Option<(f32, f32)> {
    let (left, right) = value.split_once(separator)?;
    Some((left.trim().parse().ok()?, right.trim().parse().ok()?))
}
