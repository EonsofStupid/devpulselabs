//! Browse windows — the operator's "separate windows appear on click":
//! one native peer viewport per open folder, a thumbnail grid, click a
//! file to pull it into the project. Pure state + the window BODY; the
//! Composer owns viewport summoning and drains `picked`.

use crate::import::media_kind;
use dpl_coldlight::{self as cl, Tone};
use egui::{RichText, Sense};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::{Path, PathBuf};

pub struct Browser {
    /// Distinguishes this window's viewport/pilot ids for its lifetime.
    pub slot: usize,
    pub cwd: PathBuf,
    /// None = rescan on next frame (set by navigation).
    pub entries: Option<Vec<Entry>>,
    /// Media files clicked since the composer last drained.
    pub picked: Vec<PathBuf>,
    /// Folders "Pin pack"-ed since the composer last drained.
    pub pinned: Vec<PathBuf>,
    /// A confirmed "Ingest pack" ask (this window's cwd), drained once.
    pub ingest_req: Option<PathBuf>,
    /// The armed ingest confirm — (files, bytes) shown before tap two.
    pub arm: Option<(usize, u64)>,
    pub open: bool,
}

pub enum EntryKind {
    Dir,
    Image,
    Video,
    Audio,
    Element,
}

pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub kind: EntryKind,
}

impl Browser {
    pub fn new(slot: usize, cwd: PathBuf) -> Self {
        Self {
            slot,
            cwd,
            entries: None,
            picked: Vec::new(),
            pinned: Vec::new(),
            ingest_req: None,
            arm: None,
            open: true,
        }
    }

    pub fn title(&self) -> String {
        let name = self
            .cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.cwd.to_string_lossy().into_owned());
        format!("Browse — {name}")
    }

    fn go(&mut self, to: PathBuf) {
        self.cwd = to;
        self.entries = None;
        // Navigation disarms a pending ingest confirm (the two-tap law).
        self.arm = None;
    }
}

/// The quick-jump roots: the profile's media folders that exist, then
/// every present drive. Pure std — no dialog crate, no new deps.
pub fn roots() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    if let Ok(profile) = std::env::var("USERPROFILE") {
        for name in ["Videos", "Pictures", "Downloads", "Desktop"] {
            let p = Path::new(&profile).join(name);
            if p.is_dir() {
                out.push((name.to_string(), p));
            }
        }
    }
    for c in b'A'..=b'Z' {
        let p = PathBuf::from(format!("{}:\\", c as char));
        if p.is_dir() {
            out.push((format!("{}:", c as char), p));
        }
    }
    out
}

/// One folder, honestly: dirs first, then media files, both name-sorted;
/// hidden (dot-prefixed) entries and non-media files are skipped.
pub fn scan(dir: &Path) -> Vec<Entry> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for e in rd.flatten() {
        let path = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            dirs.push(Entry {
                path,
                name,
                kind: EntryKind::Dir,
            });
        } else if let Some(kind) = media_kind(&path) {
            let kind = match kind {
                "video" => EntryKind::Video,
                "audio" => EntryKind::Audio,
                "element" => EntryKind::Element,
                _ => EntryKind::Image,
            };
            files.push(Entry { path, name, kind });
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.extend(files);
    dirs
}

/// The window BODY (the deferred viewport calls this). Ordinary egui —
/// scan lazily, paint the bar + grid, record picks.
pub fn window_ui(ui: &mut egui::Ui, b: &mut Browser) {
    if b.entries.is_none() {
        b.entries = Some(scan(&b.cwd));
    }
    let ns = b.slot;
    let mut go: Option<PathBuf> = None;

    ui.horizontal_wrapped(|ui| {
        let up = cl::tip(bar_chip(ui, "up", ns, "up"), "Up one folder");
        ui.pilot_track(
            &up,
            NodeMetadata::new(
                PilotId::new("browse.up").child(&PilotKey::new(format!("{ns}"))),
                Role::Button,
                "Up one folder",
            ),
        );
        if up.clicked() {
            if let Some(parent) = b.cwd.parent() {
                go = Some(parent.to_path_buf());
            }
        }
        for (label, path) in roots() {
            let here = b.cwd == path;
            let resp = cl::tip(
                bar_chip(ui, &label, ns, &label),
                &format!("Jump to {}", path.display()),
            );
            if resp.clicked() && !here {
                go = Some(path);
            }
        }
        let pin = cl::tip(
            bar_chip(ui, "Pin pack", ns, "pin-pack"),
            "Add this folder to your Elements shelf — the files stay right \
             here; using one copies just that file into your project",
        );
        ui.pilot_track(
            &pin,
            NodeMetadata::new(
                PilotId::new("browse.pin").child(&PilotKey::new(format!("{ns}"))),
                Role::Button,
                "Pin this folder as a pack",
            ),
        );
        if pin.clicked() {
            b.pinned.push(b.cwd.clone());
        }
        let ingest_label = match &b.arm {
            Some((files, bytes)) => {
                format!("Ingest {files} files · {:.2} GB?", *bytes as f64 / 1e9)
            }
            None => "Ingest pack".to_string(),
        };
        let ing = cl::tip(
            bar_chip(ui, &ingest_label, ns, "ingest"),
            if b.arm.is_some() {
                "Tap again to copy this whole folder into your library — \
                 the originals here are never touched"
            } else {
                "Copy this folder into your library so it's local — two taps; \
                 the size shows before anything moves"
            },
        );
        ui.pilot_track(
            &ing,
            NodeMetadata::new(
                PilotId::new("browse.ingest").child(&PilotKey::new(format!("{ns}"))),
                Role::Button,
                "Ingest this pack",
            )
            .value(if b.arm.is_some() { "armed" } else { "idle" }),
        );
        if ing.clicked() {
            match b.arm.take() {
                Some(_) => b.ingest_req = Some(b.cwd.clone()),
                None => b.arm = Some(crate::packs::pack_size(&b.cwd)),
            }
        }
        ui.label(
            RichText::new(b.cwd.to_string_lossy())
                .small()
                .color(cl::ink::SUBTLE),
        );
    });
    ui.add_space(cl::space::S2);

    let entries = b.entries.as_deref().unwrap_or(&[]);
    if entries.is_empty() {
        ui.label(
            RichText::new("Nothing here — no folders and no media files this window imports.")
                .color(cl::ink::MUTED),
        );
    }
    let mut picked: Option<PathBuf> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail = ui.available_width();
            let cols = ((avail / 180.0).floor() as usize).clamp(1, 6);
            let card_w = (avail - cl::space::S2 * (cols - 1) as f32) / cols as f32;
            let thumb_h = card_w * 9.0 / 16.0;
            egui::Grid::new(("dpl_browse_grid", ns))
                .num_columns(cols)
                .spacing([cl::space::S2, cl::space::S2])
                .show(ui, |ui| {
                    for (n, e) in entries.iter().enumerate() {
                        if card(ui, e, card_w, thumb_h, ns) {
                            match e.kind {
                                EntryKind::Dir => go = Some(e.path.clone()),
                                _ => picked = Some(e.path.clone()),
                            }
                        }
                        if (n + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    if let Some(p) = picked {
        b.picked.push(p);
    }
    if let Some(to) = go {
        b.go(to);
    }
}

/// One entry card: folder = named plate; image = the image itself;
/// video/element = honest badge plates (posters are made at import,
/// never by probing folders). The whole card is the hit target.
fn card(ui: &mut egui::Ui, e: &Entry, w: f32, thumb_h: f32, ns: usize) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, thumb_h + 18.0), Sense::click());
    let thumb = egui::Rect::from_min_size(rect.min, egui::vec2(w, thumb_h));
    let painter = ui.painter();
    painter.rect_filled(thumb, cl::RADIUS_SM, cl::surface::WELL);
    let (center_text, center_tone) = match e.kind {
        EntryKind::Image => {
            let uri = format!("file:///{}", e.path.to_string_lossy().replace('\\', "/"));
            egui::Image::new(uri)
                .corner_radius(cl::RADIUS_SM)
                .paint_at(ui, thumb);
            ("", Tone::Cyan)
        }
        EntryKind::Video => ("VIDEO", Tone::Cyan),
        EntryKind::Audio => ("AUDIO", Tone::Coral),
        EntryKind::Element => ("ELEMENT", Tone::Lime),
        EntryKind::Dir => ("FOLDER", Tone::Magenta),
    };
    if !center_text.is_empty() {
        painter.text(
            thumb.center(),
            egui::Align2::CENTER_CENTER,
            center_text,
            egui::FontId::monospace(12.0),
            center_tone.color(),
        );
        let ext = e
            .path
            .extension()
            .map(|x| x.to_string_lossy().to_ascii_uppercase())
            .unwrap_or_default();
        if !matches!(e.kind, EntryKind::Dir) {
            painter.text(
                egui::pos2(thumb.max.x - 5.0, thumb.max.y - 4.0),
                egui::Align2::RIGHT_BOTTOM,
                ext,
                egui::FontId::monospace(10.0),
                cl::ink::STRONG,
            );
        }
    }
    let hovered = resp.hovered();
    painter.rect_stroke(
        thumb,
        cl::RADIUS_SM,
        egui::Stroke::new(
            if hovered { 2.0 } else { 1.0 },
            if hovered {
                Tone::Magenta.color()
            } else {
                cl::surface::HAIRLINE
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(rect.min.x + 2.0, rect.max.y - 2.0),
        egui::Align2::LEFT_BOTTOM,
        &e.name,
        egui::FontId::proportional(11.0),
        if hovered {
            cl::ink::STRONG
        } else {
            cl::ink::BASE
        },
    );
    let tipped = cl::tip(
        resp,
        match e.kind {
            EntryKind::Dir => "Open this folder",
            EntryKind::Video => "Pull this clip into your project — it lands on the timeline",
            EntryKind::Image => "Pull this image into your library",
            EntryKind::Audio => {
                "Pull this track into your library (timeline audio lands with the Narrate pass)"
            }
            EntryKind::Element => {
                "File this element into your library (parsing lands with the .aep pass)"
            }
        },
    );
    ui.pilot_track(
        &tipped,
        NodeMetadata::new(
            PilotId::new("browse.entry").child(&PilotKey::new(format!("{ns}:{}", e.name))),
            Role::Button,
            &e.name,
        ),
    );
    tipped.clicked()
}

/// A small brutal chip for the window's top bar.
fn bar_chip(ui: &mut egui::Ui, text: &str, ns: usize, key: &str) -> egui::Response {
    let inner = cl::brutal(Tone::Cyan)
        .margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().strong());
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("browse-chip", ns, key.to_string())),
        Sense::click(),
    )
}
