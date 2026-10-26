//! The frozen-frame editor (C1) — region rect + freeform polygon over a
//! `freeze_primary_to` still, in a borderless fullscreen viewport with
//! the magnifier loupe. Split out of `lib.rs` in the C2 alignment pass;
//! the owner (`Viziwiz`) holds the state, this module paints and finishes.

use dpl_coldlight::{self as cl};
use dpl_engine_capture as cap;
use dpl_landing::CaptureKind;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, ViewportBuilder, ViewportId};
use egui_pilot_ai::protocol::{ActionKind as PilotActionKind, PilotViewportKey, Role as PilotRole};
use egui_pilot_ai::{ContextPilotExt, NodeMetadata, PilotId, UiPilotExt};
use std::path::PathBuf;

use crate::{LastSnip, Viziwiz, TONE};

const REGION_KEY: &str = "devpulse.viziwiz.region";
const REGION_DRAG: PilotId = PilotId::new("viziwiz.region.drag");

/// Which shape the frozen-frame editor is collecting (C1).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnipMode {
    /// Drag → armed rect → nudge (arrows, Shift=10px) → Enter/double-click.
    Rect,
    /// Click vertices → close on the first vertex or Enter (≥3);
    /// Backspace removes the last vertex.
    Free,
}

/// The frozen-frame editor's state (region AND freeform ride one
/// viewport — same freeze, same veil, same loupe).
pub(crate) struct RegionSelect {
    tex: egui::TextureHandle,
    frozen: PathBuf,
    mode: SnipMode,
    /// Rect mode: the ARMED selection (survives release for the nudge
    /// ceremony; a new drag replaces it).
    sel: Option<Rect>,
    /// Rect mode: the live drag's anchor while the button is down.
    dragging: Option<egui::Pos2>,
    /// Free mode: the collected vertices, screen space.
    poly: Vec<egui::Pos2>,
}

impl Viziwiz {
    pub(crate) fn begin_editor(&mut self, ctx: &egui::Context, mode: SnipMode) {
        if self.capture_blocked() {
            return;
        }
        let result = (|| -> Result<RegionSelect, String> {
            let temp = dpl_roots::temp_root();
            std::fs::create_dir_all(&temp).map_err(|e| format!("freeze folder: {e}"))?;
            let frozen = temp.join(format!("dpl-viziwiz-freeze-{}.png", std::process::id()));
            cap::freeze_primary_to(&frozen)?;
            let img = image::open(&frozen).map_err(|e| format!("freeze decode: {e}"))?;
            let rgba = img.to_rgba8();
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                &rgba,
            );
            let tex = ctx.load_texture("viziwiz-freeze", color, egui::TextureOptions::LINEAR);
            Ok(RegionSelect {
                tex,
                frozen,
                mode,
                sel: None,
                dragging: None,
                poly: Vec::new(),
            })
        })();
        match result {
            Ok(r) => self.region = Some(r),
            Err(e) => self.status = format!("freeze: {e}"),
        }
    }

    pub(crate) fn region_viewport(&mut self, ctx: &egui::Context) {
        enum Done {
            Cancel,
            Rect(Rect, egui::Vec2),
            Free(Vec<egui::Pos2>, egui::Vec2),
        }
        let mut finish: Option<Done> = None;
        let region = self.region.as_mut().expect("region state");
        let builder = ViewportBuilder::default()
            .with_title("ViziWiz region")
            .with_fullscreen(true)
            .with_always_on_top()
            .with_decorations(false);

        let mut close_requested = false;
        ctx.pilot_show_viewport_immediate(
            PilotViewportKey::new(REGION_KEY),
            ViewportId::from_hash_of("viziwiz-region"),
            builder,
            |ctx, _| {
                close_requested = ctx.input(|input| input.viewport().close_requested());
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(Color32::BLACK))
                    .show(ctx, |ui| {
                        let full = ui.max_rect();
                        let painter = ui.painter().clone();
                        let tex = region.tex.id();
                        painter.image(
                            tex,
                            full,
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );

                        let resp = ui.interact(
                            full,
                            egui::Id::new("viziwiz-region-drag"),
                            Sense::click_and_drag(),
                        );
                        let state_word = match region.mode {
                            SnipMode::Rect if region.dragging.is_some() => "dragging".into(),
                            SnipMode::Rect if region.sel.is_some() => "armed".into(),
                            SnipMode::Rect => "drag to select".into(),
                            SnipMode::Free => format!("polygon · {} vertices", region.poly.len()),
                        };
                        let mut metadata = NodeMetadata::new(
                            REGION_DRAG,
                            PilotRole::Region,
                            "ViziWiz snip selection",
                        )
                        .value(state_word);
                        metadata.actions = vec![PilotActionKind::Drag];
                        ui.pilot_track(&resp, metadata);

                        match region.mode {
                            SnipMode::Rect => {
                                // Drag paints live; release ARMS (the old
                                // pro-snip ceremony) — a new drag replaces.
                                if resp.drag_started() {
                                    if let Some(p) = resp.interact_pointer_pos() {
                                        region.dragging = Some(p);
                                        region.sel = Some(Rect::from_two_pos(p, p));
                                    }
                                }
                                if let (Some(anchor), Some(p)) =
                                    (region.dragging, resp.interact_pointer_pos())
                                {
                                    if resp.dragged() {
                                        region.sel = Some(Rect::from_two_pos(anchor, p));
                                    }
                                }
                                if resp.drag_stopped() {
                                    region.dragging = None;
                                }
                                // The nudge: arrows move the armed rect 1px,
                                // Shift makes it 10.
                                if region.dragging.is_none() {
                                    if let Some(sel) = region.sel.as_mut() {
                                        let step = if ui.input(|i| i.modifiers.shift) {
                                            10.0
                                        } else {
                                            1.0
                                        };
                                        let mut d = egui::Vec2::ZERO;
                                        ui.input(|i| {
                                            if i.key_pressed(egui::Key::ArrowLeft) {
                                                d.x -= step;
                                            }
                                            if i.key_pressed(egui::Key::ArrowRight) {
                                                d.x += step;
                                            }
                                            if i.key_pressed(egui::Key::ArrowUp) {
                                                d.y -= step;
                                            }
                                            if i.key_pressed(egui::Key::ArrowDown) {
                                                d.y += step;
                                            }
                                        });
                                        if d != egui::Vec2::ZERO {
                                            *sel = sel.translate(d);
                                        }
                                    }
                                }
                                // Enter or double-click CAPTURES the armed rect.
                                let confirm = ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    || resp.double_clicked();
                                if confirm {
                                    if let Some(sel) = region.sel {
                                        finish = Some(Done::Rect(sel, full.size()));
                                    }
                                }
                            }
                            SnipMode::Free => {
                                // Every click adds a vertex; a click near the
                                // FIRST vertex (≥3) closes; Enter closes too;
                                // Backspace removes the last.
                                if resp.clicked() {
                                    if let Some(p) = resp.interact_pointer_pos() {
                                        let closes = region.poly.len() >= 3
                                            && (p - region.poly[0]).length() < 8.0;
                                        if closes {
                                            finish = Some(Done::Free(
                                                region.poly.clone(),
                                                full.size(),
                                            ));
                                        } else {
                                            region.poly.push(p);
                                        }
                                    }
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Backspace)) {
                                    region.poly.pop();
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    && region.poly.len() >= 3
                                {
                                    finish =
                                        Some(Done::Free(region.poly.clone(), full.size()));
                                }
                            }
                        }

                        // The veil + guides, per mode.
                        let sel = match region.mode {
                            SnipMode::Rect => region.sel,
                            SnipMode::Free => None,
                        };
                        if region.mode == SnipMode::Free {
                            painter.rect_filled(full, 0, Color32::from_black_alpha(60));
                            for w in region.poly.windows(2) {
                                painter.line_segment(
                                    [w[0], w[1]],
                                    egui::Stroke::new(1.5, TONE.color()),
                                );
                            }
                            if let (Some(&first), Some(p)) =
                                (region.poly.first(), ui.ctx().pointer_latest_pos())
                            {
                                if let Some(&last) = region.poly.last() {
                                    painter.line_segment(
                                        [last, p],
                                        egui::Stroke::new(
                                            1.0,
                                            TONE.color().gamma_multiply(0.5),
                                        ),
                                    );
                                }
                                // The close target glows once it can close.
                                if region.poly.len() >= 3 {
                                    painter.circle_stroke(
                                        first,
                                        8.0,
                                        egui::Stroke::new(1.5, TONE.color()),
                                    );
                                }
                            }
                            for v in &region.poly {
                                painter.circle_filled(*v, 3.0, TONE.color());
                            }
                            painter.text(
                                egui::pos2(full.center().x, full.max.y - 28.0),
                                Align2::CENTER_CENTER,
                                "click to add vertices · click the first (or Enter) closes · Backspace undoes · Esc cancels",
                                FontId::proportional(14.0),
                                cl::ink::STRONG,
                            );
                        }
                        let veil = Color32::from_black_alpha(140);
                        match sel {
                            Some(r) => {
                                for shade in [
                                    Rect::from_min_max(full.min, egui::pos2(full.max.x, r.min.y)),
                                    Rect::from_min_max(egui::pos2(full.min.x, r.max.y), full.max),
                                    Rect::from_min_max(
                                        egui::pos2(full.min.x, r.min.y),
                                        egui::pos2(r.min.x, r.max.y),
                                    ),
                                    Rect::from_min_max(
                                        egui::pos2(r.max.x, r.min.y),
                                        egui::pos2(full.max.x, r.max.y),
                                    ),
                                ] {
                                    painter.rect_filled(shade, 0, veil);
                                }
                                painter.rect(
                                    r,
                                    0,
                                    Color32::TRANSPARENT,
                                    egui::Stroke::new(1.5, TONE.color()),
                                    egui::StrokeKind::Outside,
                                );
                                painter.text(
                                    r.min - egui::vec2(0.0, 6.0),
                                    Align2::LEFT_BOTTOM,
                                    format!("{:.0} × {:.0}", r.width(), r.height()),
                                    FontId::monospace(12.0),
                                    cl::ink::STRONG,
                                );
                                // The ceremony hint, once armed.
                                if region.dragging.is_none() {
                                    painter.text(
                                        egui::pos2(full.center().x, full.max.y - 28.0),
                                        Align2::CENTER_CENTER,
                                        "arrows nudge (Shift = 10px) · Enter or double-click captures · drag again to redo · Esc cancels",
                                        FontId::proportional(14.0),
                                        cl::ink::STRONG,
                                    );
                                }
                            }
                            None if region.mode == SnipMode::Rect => {
                                painter.rect_filled(full, 0, Color32::from_black_alpha(60));
                                painter.text(
                                    full.center(),
                                    Align2::CENTER_CENTER,
                                    "drag to select · Esc cancels",
                                    FontId::proportional(15.0),
                                    cl::ink::STRONG,
                                );
                            }
                            None => {}
                        }

                        // The loupe: 6× zoom around the cursor, ColdLight-framed
                        // (the original PixVix magnifier — restored from the
                        // port's 4× to the authored 6×).
                        if let Some(p) = ui.ctx().pointer_latest_pos() {
                            let zoom = 6.0;
                            let box_size = 120.0;
                            let uv_half = box_size / zoom / 2.0;
                            let uv = Rect::from_min_max(
                                egui::pos2(
                                    ((p.x - uv_half) / full.width()).clamp(0.0, 1.0),
                                    ((p.y - uv_half) / full.height()).clamp(0.0, 1.0),
                                ),
                                egui::pos2(
                                    ((p.x + uv_half) / full.width()).clamp(0.0, 1.0),
                                    ((p.y + uv_half) / full.height()).clamp(0.0, 1.0),
                                ),
                            );
                            let offset = egui::vec2(24.0, 24.0);
                            let mut anchor = p + offset;
                            if anchor.x + box_size > full.max.x {
                                anchor.x = p.x - offset.x - box_size;
                            }
                            if anchor.y + box_size > full.max.y {
                                anchor.y = p.y - offset.y - box_size;
                            }
                            let loupe = Rect::from_min_size(anchor, egui::vec2(box_size, box_size));
                            painter.image(tex, loupe, uv, Color32::WHITE);
                            painter.rect(
                                loupe,
                                CornerRadius::same(cl::RADIUS_SM),
                                Color32::TRANSPARENT,
                                egui::Stroke::new(1.5, TONE.color()),
                                egui::StrokeKind::Outside,
                            );
                            // Crosshair at the loupe center = the cursor pixel.
                            let c = loupe.center();
                            painter.line_segment(
                                [c - egui::vec2(8.0, 0.0), c + egui::vec2(8.0, 0.0)],
                                egui::Stroke::new(1.0, TONE.color()),
                            );
                            painter.line_segment(
                                [c - egui::vec2(0.0, 8.0), c + egui::vec2(0.0, 8.0)],
                                egui::Stroke::new(1.0, TONE.color()),
                            );
                        }

                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            finish = Some(Done::Cancel);
                        }
                    });
            },
        );
        if close_requested {
            finish = Some(Done::Cancel);
        }

        if let Some(done) = finish {
            let state = self.region.take().expect("region state");
            match done {
                Done::Rect(r, screen) => {
                    if r.width() >= 4.0 && r.height() >= 4.0 {
                        let frac = cap::FracRect {
                            x: (r.min.x / screen.x) as f64,
                            y: (r.min.y / screen.y) as f64,
                            w: (r.width() / screen.x) as f64,
                            h: (r.height() / screen.y) as f64,
                        };
                        let target = self.land_target(CaptureKind::Image, "png", None);
                        let result = target.and_then(|out| {
                            cap::still_region_from_png(&state.frozen, &out, frac)?;
                            Ok(out)
                        });
                        self.status = match result {
                            Ok(p) => {
                                self.last = Some(LastSnip::Region(frac));
                                self.finish_still(ctx, p)
                            }
                            Err(e) => e,
                        };
                    } else {
                        self.status = "region too small".into();
                    }
                }
                Done::Free(poly, screen) => {
                    let points: Vec<[f64; 2]> = poly
                        .iter()
                        .map(|p| [(p.x / screen.x) as f64, (p.y / screen.y) as f64])
                        .collect();
                    let target = self.land_target(CaptureKind::Image, "png", None);
                    let result = target.and_then(|out| {
                        cap::still_freeform_from_png(&state.frozen, &out, &points)?;
                        Ok(out)
                    });
                    self.status = match result {
                        Ok(p) => {
                            self.last = Some(LastSnip::Free(points));
                            self.finish_still(ctx, p)
                        }
                        Err(e) => e,
                    };
                }
                Done::Cancel => self.status = "snip cancelled".into(),
            }
            let _ = std::fs::remove_file(&state.frozen); // temp freeze only
        }
    }
}
