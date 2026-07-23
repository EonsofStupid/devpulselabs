//! wiredFRONT — capture and media editing: PixVix, the video editor
//! (ADR-010: stitch, ComfyUI workflows), image ops (cutout via ort).
//!
//! GALLERY (X2) is live: the content library wall over the artifact
//! index — every capture thumbnailed (honest fallbacks, never a broken
//! image), filter chips, rescan, and the inbox DUMP (two-tap armed;
//! engine-jailed to the inbox).

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::{RichText, Sense};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};

pub const HUB: HubId = HubId("wiredfront");
pub const OVERVIEW: SubId = SubId("overview");
pub const PIXVIX: SubId = SubId("pixvix");
pub const VIZIWIZ: SubId = SubId("viziwiz");
pub const GALLERY: SubId = SubId("gallery");
pub const SESSIONS: SubId = SubId("sessions");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "wiredFRONT",
    icon: cl::icon::WIREDFRONT,
    tone: Tone::Magenta,
    subroutes: &[
        SubRoute {
            id: OVERVIEW,
            label: "Overview",
            blurb: "The hub floor: inbox review, every capture vehicle one click away.",
        },
        SubRoute {
            id: PIXVIX,
            label: "PixVix",
            blurb:
                "The snapper — stills, clips, region, freeform. The engine port lands here first.",
        },
        SubRoute {
            id: VIZIWIZ,
            label: "ViziWiz",
            blurb: "Image ops: annotate, flatten, cutout (ort + BiRefNet — no Node, no segfault).",
        },
        SubRoute {
            id: GALLERY,
            label: "Gallery",
            blurb: "The content library: every artifact, thumbnailed, previewed in place.",
        },
        SubRoute {
            id: SESSIONS,
            label: "Sessions",
            blurb: "Session recordings — video-first (the rrweb path retired with the webview).",
        },
    ],
};

/// The Gallery's state: rows loaded once per visit, a kind filter, the
/// armed dump target (two-tap law), and the honest status line.
struct GalleryView {
    rows: Vec<dpl_index::Record>,
    filter: Option<&'static str>,
    /// rel_path armed for dump — the second tap fires, anything else
    /// disarms (a destructive action never rides one stray tap).
    armed: Option<String>,
    status: String,
}

impl GalleryView {
    fn load(filter: Option<&'static str>) -> Self {
        let (rows, status) =
            match dpl_index::Index::open_default().and_then(|ix| ix.recent(500, None)) {
                Ok(rows) => {
                    let status = format!("{} artifacts", rows.len());
                    (rows, status)
                }
                Err(e) => (Vec::new(), e),
            };
        Self {
            rows,
            filter,
            armed: None,
            status,
        }
    }
}

#[derive(Default)]
pub struct WiredfrontHub {
    gallery: Option<GalleryView>,
}

const FILTERS: [(&str, Option<&'static str>); 4] = [
    ("all", None),
    ("image", Some("image")),
    ("video", Some("video")),
    ("guide", Some("guide")),
];

impl WiredfrontHub {
    fn gallery_surface(&mut self, ui: &mut egui::Ui) {
        let view = self.gallery.get_or_insert_with(|| GalleryView::load(None));

        // The header row: filters · rescan · status.
        let mut reload_with: Option<Option<&'static str>> = None;
        let mut rescan = false;
        ui.horizontal(|ui| {
            for (label, kind) in FILTERS {
                let on = view.filter == kind;
                let resp = filter_chip(ui, label, on);
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("gallery.filter").child(&PilotKey::new(label)),
                        egui_pilot_ai::protocol::Role::Button,
                        label,
                    )
                    .value(if on { "active" } else { "available" }),
                );
                if resp.clicked() && !on {
                    reload_with = Some(kind);
                }
            }
            ui.add_space(cl::space::S3);
            let resp = filter_chip(ui, "rescan", false);
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("gallery").child(&PilotKey::new("rescan")),
                    egui_pilot_ai::protocol::Role::Button,
                    "Rescan the inbox",
                ),
            );
            if resp.clicked() {
                rescan = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(&view.status).small().color(cl::ink::SUBTLE));
            });
        });

        if rescan {
            let status = match dpl_index::Index::open_default()
                .and_then(|ix| ix.reindex(&dpl_engine_capture::content_root()))
            {
                Ok(r) => format!("rescan: indexed {} · soft-cleared {}", r.indexed, r.cleared),
                Err(e) => e,
            };
            let filter = view.filter;
            let mut fresh = GalleryView::load(filter);
            fresh.status = status;
            self.gallery = Some(fresh);
            return;
        }
        if let Some(kind) = reload_with {
            self.gallery = Some(GalleryView::load(kind));
            return;
        }

        ui.add_space(cl::space::S2);
        let root = dpl_engine_capture::content_root();
        let filter = view.filter;
        let visible: Vec<usize> = view
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| filter.is_none_or(|k| r.kind == k))
            .map(|(i, _)| i)
            .collect();

        if visible.is_empty() {
            ui.label(
                RichText::new(
                    "nothing here yet — PixVix captures land in the inbox and show up here",
                )
                .color(cl::ink::MUTED),
            );
            return;
        }

        // The wall: adaptive card grid.
        let mut dump: Option<String> = None;
        let mut disarm = false;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail = ui.available_width();
                let cols = ((avail / 300.0).floor() as usize).clamp(1, 6);
                let card_w = (avail - cl::space::S3 * (cols - 1) as f32) / cols as f32;
                let thumb_h = card_w * 9.0 / 16.0;
                egui::Grid::new("dpl_gallery_grid")
                    .num_columns(cols)
                    .spacing([cl::space::S3, cl::space::S3])
                    .show(ui, |ui| {
                        for (n, i) in visible.iter().enumerate() {
                            let r = &view.rows[*i];
                            let (fired, disarmed) = artifact_card(
                                ui,
                                r,
                                &root,
                                card_w,
                                thumb_h,
                                view.armed.as_deref() == Some(r.rel_path.as_str()),
                                &mut view.armed,
                            );
                            if fired {
                                dump = Some(r.rel_path.clone());
                            }
                            if disarmed {
                                disarm = true;
                            }
                            if (n + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        if disarm {
            view.armed = None;
        }
        if let Some(rel) = dump {
            let status = match dpl_index::Index::open_default().and_then(|ix| ix.dump(&root, &rel))
            {
                Ok(()) => format!("dumped {rel}"),
                Err(e) => e,
            };
            let filter = view.filter;
            let mut fresh = GalleryView::load(filter);
            fresh.status = status;
            self.gallery = Some(fresh);
        }
    }
}

/// One artifact card: thumb (honest fallback), name, kind+tags line, and
/// the two-tap Dump. Returns (dump_fired, disarm_requested).
fn artifact_card(
    ui: &mut egui::Ui,
    r: &dpl_index::Record,
    root: &std::path::Path,
    w: f32,
    thumb_h: f32,
    armed: bool,
    arm: &mut Option<String>,
) -> (bool, bool) {
    let mut fired = false;
    let mut disarm = false;
    ui.vertical(|ui| {
        ui.set_width(w);
        // The thumbnail well.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, thumb_h), Sense::hover());
        ui.painter()
            .rect_filled(rect, cl::RADIUS_SM, cl::surface::WELL);
        match thumb_uri(r, root) {
            Some(uri) => {
                egui::Image::new(uri)
                    .corner_radius(cl::RADIUS_SM)
                    .paint_at(ui, rect);
            }
            None => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &r.kind,
                    egui::FontId::monospace(13.0),
                    cl::ink::SUBTLE,
                );
            }
        }
        ui.painter().rect_stroke(
            rect,
            cl::RADIUS_SM,
            egui::Stroke::new(1.0, cl::surface::HAIRLINE),
            egui::StrokeKind::Inside,
        );

        ui.horizontal(|ui| {
            ui.label(RichText::new(&r.name).strong().color(cl::ink::STRONG));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if armed { "Dump?" } else { "Dump" };
                let resp = ui
                    .label(RichText::new(label).small().color(if armed {
                        cl::Tone::Coral.color()
                    } else {
                        cl::ink::SUBTLE
                    }))
                    .interact(Sense::click());
                if resp.clicked() {
                    if armed {
                        fired = true;
                    } else {
                        *arm = Some(r.rel_path.clone());
                    }
                } else if armed && ui.input(|i| i.pointer.any_click()) {
                    // Any click elsewhere disarms — one stray tap never
                    // deletes.
                    disarm = true;
                }
            });
        });
        let mut meta = format!("{} · {}", r.kind, r.captured_at);
        if !r.tags.is_empty() {
            meta = format!("{meta} · {}", r.tags.join(", "));
        }
        ui.label(RichText::new(meta).small().color(cl::ink::MUTED));
    });
    (fired, disarm)
}

/// The thumbnail source, or None for the honest kind-label fallback:
/// image → itself; video → its poster; guide → its first frame.
fn thumb_uri(r: &dpl_index::Record, root: &std::path::Path) -> Option<String> {
    let rel = match r.kind.as_str() {
        "image" => Some(r.rel_path.clone()),
        "video" => r.poster_rel_path.clone(),
        "guide" => {
            let first = root.join(&r.rel_path).join("step-001.webp");
            first
                .is_file()
                .then_some(format!("{}/step-001.webp", r.rel_path))
        }
        _ => None,
    }?;
    let abs = root.join(rel);
    // file:/// with THREE slashes — two reads as a UNC host (the media
    // block's hard-won law, crates/slides/blocks/media.rs).
    abs.is_file()
        .then(|| format!("file:///{}", abs.to_string_lossy().replace('\\', "/")))
}

/// A quiet filter chip (tap-min height rides the label padding).
fn filter_chip(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.into(),
        egui::FontId::proportional(13.0),
        cl::ink::BASE,
    );
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(galley.size().x + 22.0, 28.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(cl::RADIUS_SM),
        if on {
            cl::Tone::Magenta.wash()
        } else if resp.hovered() {
            cl::surface::OVERLAY
        } else {
            cl::surface::RAISED
        },
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        if on { cl::ink::STRONG } else { cl::ink::BASE },
    );
    resp
}

impl Hub for WiredfrontHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        if route.sub == GALLERY {
            self.gallery_surface(ui);
            return;
        }
        dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "Capture, stitch, generate, edit. ADR-010 is this hub's law: shot slots, \
             ffmpeg subprocess, ComfyUI as the first pluggable backend.",
        );
    }
}
