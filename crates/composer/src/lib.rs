//! The Composer (R1a) — the WiredFRONT video editor surface: a four-pane
//! shell whose timeline is a CLIP LIST you build by appending source clips
//! from the artifact index, with single-source preview playback. It writes
//! through `dpl-comp` — the composition is plain, validated `comp.json`, the
//! same AI-editable document the later agent seam edits.
//!
//! R1a is the FOUNDATION: NO stitch render, NO generation, NO matting, NO
//! compositor. The Preview plays the SELECTED layer's source (via the S2
//! `ClipPlayer`); the COMPOSITED preview + ffmpeg stitch render land in R1b.
//! The two-editors law holds: this is never the Workbench slide editor.
//!
//! Panes use egui's native panels (fixed layout) — RS4's `egui_tiles`
//! drag-docking is R1b's rider, adopted when the layout needs rearranging.

use dpl_coldlight::{self as cl, Tone};
use dpl_comp::{Composition, Layer, Source};
use dpl_index::Record;
use dpl_video::{clip::ClipPlayer, DECODE_H, DECODE_W};
use egui::{Color32, CornerRadius, Rect, RichText, Sense, UiBuilder};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::PathBuf;
use std::time::Duration;

/// The default trimmed length for a clip whose duration we cannot probe.
const FALLBACK_CLIP_SECS: f32 = 6.0;

/// The editing session: the comp under edit, where it saves, the current
/// selection + its preview player, the lazily-loaded palette, and an honest
/// status line.
pub struct Composer {
    comp: Composition,
    /// Where `Save` writes (`inbox/<comp-id>/`); None until first save.
    dir: Option<PathBuf>,
    /// The selected layer id.
    selected: Option<String>,
    /// The preview player, keyed by the layer id it was opened for.
    preview: Option<(String, ClipPlayer)>,
    /// Video artifacts from the index — the clips you can append.
    palette: Option<Vec<Record>>,
    status: String,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            comp: Composition::default_1080p30(),
            dir: None,
            selected: None,
            preview: None,
            palette: None,
            status: "New composition — add clips from the palette on the left.".into(),
        }
    }
}

/// One deferred UI action, applied after the panels release their borrows.
enum Act {
    New,
    Save,
    Add(usize),
    Select(String),
    Remove(String),
    MoveUp(String),
    MoveDown(String),
}

impl Composer {
    /// The content root, resolved from the same law the Gallery uses.
    fn content_root() -> PathBuf {
        dpl_engine_capture::content_root()
    }

    /// Resolve a forward-slash artifact rel_path to a real file path under the
    /// content root (component-wise, so Windows separators stay correct).
    fn resolve(rel: &str) -> PathBuf {
        rel.split('/')
            .fold(Self::content_root(), |acc, seg| acc.join(seg))
    }

    /// The main track's layers, in order — R1a edits the first track.
    fn layers(&self) -> &[Layer] {
        self.comp
            .tracks
            .first()
            .map(|t| t.layers.as_slice())
            .unwrap_or(&[])
    }

    /// A layer id unique across the comp: the first free `clip-N`.
    fn next_layer_id(&self) -> String {
        let used: std::collections::BTreeSet<&str> = self
            .comp
            .tracks
            .iter()
            .flat_map(|t| t.layers.iter())
            .map(|l| l.id.as_str())
            .collect();
        (1..)
            .map(|n| format!("clip-{n}"))
            .find(|id| !used.contains(id.as_str()))
            .unwrap()
    }

    /// Append a palette record as a new layer, stitched onto the end.
    fn add_clip(&mut self, rec: &Record) {
        let path = Self::resolve(&rec.rel_path);
        let out_pt = rec
            .seconds
            .filter(|s| *s > 0)
            .map(|s| s as f32)
            .or_else(|| dpl_video::clip::clip_duration(&path).filter(|d| *d > 0.0))
            .unwrap_or(FALLBACK_CLIP_SECS);
        let start = self.comp.duration();
        let id = self.next_layer_id();
        // default_1080p30 guarantees a "main" track exists.
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.push(Layer {
                id: id.clone(),
                source: Source::Artifact {
                    rel: rec.rel_path.clone(),
                },
                start,
                in_pt: 0.0,
                out_pt,
            });
        }
        self.status = format!(
            "Added {} — {} clip(s), {:.1}s total.",
            rec.name,
            self.layers().len(),
            self.comp.duration()
        );
        self.select(&id);
    }

    /// Select a layer and (re)open its preview if it is a playable artifact.
    fn select(&mut self, id: &str) {
        self.selected = Some(id.to_string());
        let source = self
            .comp
            .tracks
            .iter()
            .flat_map(|t| t.layers.iter())
            .find(|l| l.id == id)
            .map(|l| l.source.clone());
        match source {
            Some(Source::Artifact { rel }) => {
                let path = Self::resolve(&rel);
                self.preview = Some((id.to_string(), ClipPlayer::open(path)));
            }
            // Elements have no captured file yet (R1c ingests them).
            _ => self.preview = None,
        }
    }

    fn remove(&mut self, id: &str) {
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.retain(|l| l.id != id);
        }
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
            self.preview = None;
        }
        self.status = format!("Removed {id} — {} clip(s) left.", self.layers().len());
    }

    fn move_by(&mut self, id: &str, delta: isize) {
        if let Some(track) = self.comp.tracks.first_mut() {
            if let Some(i) = track.layers.iter().position(|l| l.id == id) {
                let j = i as isize + delta;
                if j >= 0 && (j as usize) < track.layers.len() {
                    track.layers.swap(i, j as usize);
                }
            }
        }
    }

    /// Persist `comp.json` to an inbox folder (inbox-born per R1 §1). The
    /// index write-through + keep-into-episode are R1c.
    fn save(&mut self) {
        if self.dir.is_none() {
            self.dir = Some(Self::fresh_comp_dir());
        }
        let dir = self.dir.clone().unwrap();
        self.status = match dpl_comp::save(&dir, &self.comp) {
            Ok(()) => format!("Saved to {}", dir.display()),
            Err(e) => format!("Save failed: {e}"),
        };
    }

    /// The next free `inbox/comp-N/` directory — deterministic, no clobber.
    fn fresh_comp_dir() -> PathBuf {
        let inbox = Self::content_root().join("inbox");
        (1..)
            .map(|n| inbox.join(format!("comp-{n}")))
            .find(|p| !p.exists())
            .unwrap()
    }

    fn ensure_palette(&mut self) {
        if self.palette.is_some() {
            return;
        }
        let rows = dpl_index::Index::open_default()
            .and_then(|ix| ix.recent(500, None))
            .map(|rows| {
                rows.into_iter()
                    .filter(|r| r.kind == "video")
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.palette = Some(rows);
    }

    /// The Composer surface — what the WiredFRONT hub calls for its subroute.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_palette();
        let mut act: Option<Act> = None;

        // Manual five-region layout (the house idiom — `new_child` over
        // computed rects; egui panels attach to the viewport, not a hub's
        // sub-area). Rows: toolbar / body / timeline; body cols: palette /
        // preview / inspector.
        let full = ui.max_rect();
        let side_w = 240.0_f32;
        let toolbar_h = 40.0_f32;
        let timeline_h = 150.0_f32;
        let pad = cl::space::S2;
        let body_top = full.min.y + toolbar_h + pad;
        let body_bottom = (full.max.y - timeline_h - pad).max(body_top + 1.0);
        let toolbar_rect =
            Rect::from_min_max(full.min, egui::pos2(full.max.x, full.min.y + toolbar_h));
        let timeline_rect = Rect::from_min_max(egui::pos2(full.min.x, body_bottom + pad), full.max);
        let left_rect = Rect::from_min_max(
            egui::pos2(full.min.x, body_top),
            egui::pos2(full.min.x + side_w, body_bottom),
        );
        let right_rect = Rect::from_min_max(
            egui::pos2(full.max.x - side_w, body_top),
            egui::pos2(full.max.x, body_bottom),
        );
        let center_rect = Rect::from_min_max(
            egui::pos2(full.min.x + side_w + pad, body_top),
            egui::pos2(full.max.x - side_w - pad, body_bottom),
        );

        // The four-pane plates — WELL recesses at the token radius.
        for r in [left_rect, right_rect, timeline_rect, center_rect] {
            ui.painter()
                .rect_filled(r, CornerRadius::same(cl::RADIUS), cl::surface::WELL);
        }

        let mut pane = |rect: Rect| ui.new_child(UiBuilder::new().max_rect(rect.shrink(pad)));
        self.toolbar(&mut pane(toolbar_rect), &mut act);
        self.palette_pane(&mut pane(left_rect), &mut act);
        self.inspector_pane(&mut pane(right_rect));
        self.timeline_pane(&mut pane(timeline_rect), &mut act);
        self.preview_pane(&mut pane(center_rect));

        match act {
            Some(Act::New) => *self = Composer::default(),
            Some(Act::Save) => self.save(),
            Some(Act::Add(i)) => {
                if let Some(rec) = self.palette.as_ref().and_then(|p| p.get(i)).cloned() {
                    self.add_clip(&rec);
                }
            }
            Some(Act::Select(id)) => self.select(&id),
            Some(Act::Remove(id)) => self.remove(&id),
            Some(Act::MoveUp(id)) => self.move_by(&id, -1),
            Some(Act::MoveDown(id)) => self.move_by(&id, 1),
            None => {}
        }
    }

    fn toolbar(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.horizontal(|ui| {
            if cl::tip(
                chip(ui, Tone::Lime, "＋ New", "new"),
                "Start a fresh composition",
            )
            .clicked()
            {
                *act = Some(Act::New);
            }
            if cl::tip(
                chip(ui, Tone::Cyan, "▤ Save", "save"),
                "Write this composition to your inbox as comp.json",
            )
            .clicked()
            {
                *act = Some(Act::Save);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} clip(s) · {:.1}s · {}×{} @{}fps",
                        self.layers().len(),
                        self.comp.duration(),
                        self.comp.w,
                        self.comp.h,
                        self.comp.fps
                    ))
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            });
        });
    }

    fn palette_pane(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.label(RichText::new("PALETTE").small().strong());
        ui.label(
            RichText::new("Video clips in your library — tap to add.")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S2);
        let clips = self.palette.as_deref().unwrap_or(&[]);
        if clips.is_empty() {
            ui.label(
                RichText::new("No video clips yet — capture or generate some.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, rec) in clips.iter().enumerate() {
                let label = format!("{}  ·  {}", rec.name, fmt_secs(rec.seconds));
                let resp = cl::tip(
                    chip(ui, Tone::Magenta, &label, ("palette", i)),
                    "Add this clip to the end of your composition",
                );
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("composer.palette.add").child(&PilotKey::new(&rec.rel_path)),
                        Role::Button,
                        &rec.name,
                    ),
                );
                if resp.clicked() {
                    *act = Some(Act::Add(i));
                }
                ui.add_space(cl::space::S1);
            }
        });
    }

    fn timeline_pane(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.label(RichText::new("TIMELINE — the clip list").small().strong());
        ui.label(
            RichText::new("Clips play in this order. Tap to select; ↑ ↓ to reorder.")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S1);
        let layers = self.layers();
        if layers.is_empty() {
            ui.label(
                RichText::new("Empty — add a clip from the palette.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        }
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for layer in layers {
                    let selected = self.selected.as_deref() == Some(layer.id.as_str());
                    let tone = if selected { Tone::Lime } else { Tone::Cyan };
                    let title = format!("{}\n{:.1}s", layer.source.reference(), layer.span());
                    let resp = cl::tip(
                        chip(ui, tone, &title, ("clip", layer.id.as_str())),
                        "Select this clip to preview and edit it",
                    );
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("composer.clip").child(&PilotKey::new(&layer.id)),
                            Role::Button,
                            &layer.id,
                        )
                        .value(if selected {
                            "selected"
                        } else {
                            "available"
                        }),
                    );
                    if resp.clicked() {
                        *act = Some(Act::Select(layer.id.clone()));
                    }
                    if selected {
                        ui.vertical(|ui| {
                            if cl::tip(
                                small_btn(ui, "↑", ("up", layer.id.as_str())),
                                "Move this clip earlier",
                            )
                            .clicked()
                            {
                                *act = Some(Act::MoveUp(layer.id.clone()));
                            }
                            if cl::tip(
                                small_btn(ui, "↓", ("down", layer.id.as_str())),
                                "Move this clip later",
                            )
                            .clicked()
                            {
                                *act = Some(Act::MoveDown(layer.id.clone()));
                            }
                            if cl::tip(
                                small_btn(ui, "✕", ("rm", layer.id.as_str())),
                                "Remove this clip from the composition",
                            )
                            .clicked()
                            {
                                *act = Some(Act::Remove(layer.id.clone()));
                            }
                        });
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        });
    }

    fn inspector_pane(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("INSPECTOR").small().strong());
        let Some(id) = self.selected.as_deref() else {
            ui.label(
                RichText::new("Select a clip to see its details.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        };
        let Some(layer) = self.layers().iter().find(|l| l.id == id) else {
            return;
        };
        ui.add_space(cl::space::S2);
        cl::clay().show(ui, |ui| {
            ui.label(RichText::new(&layer.id).strong());
            ui.label(
                RichText::new(layer.source.reference())
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            ui.add_space(cl::space::S1);
            ui.label(RichText::new(format!("Starts at {:.2}s", layer.start)).small());
            ui.label(RichText::new(format!("Trim in {:.2}s", layer.in_pt)).small());
            ui.label(RichText::new(format!("Trim out {:.2}s", layer.out_pt)).small());
            ui.label(RichText::new(format!("Plays for {:.2}s", layer.span())).small());
        });
        ui.add_space(cl::space::S2);
        ui.label(
            RichText::new("Trim + transform editing lands in R1b.")
                .small()
                .color(cl::ink::SUBTLE),
        );
    }

    fn preview_pane(&self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        match &self.preview {
            Some((_, player)) => match player.texture(ui.ctx()) {
                Some(tex) => {
                    let uv =
                        dpl_video::cover_uv(DECODE_W as f32 / DECODE_H as f32, rect.aspect_ratio());
                    ui.painter().image(tex, rect, uv, Color32::WHITE);
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }
                None => {
                    plate(
                        ui,
                        "Loading preview…",
                        "The clip is decoding (needs ffmpeg on PATH).",
                    );
                    ui.ctx().request_repaint_after(Duration::from_millis(120));
                }
            },
            None => plate(
                ui,
                "Preview",
                "Select a clip to preview it. The composited preview lands in R1b.",
            ),
        }
    }
}

/// A full-rect tappable brutal chip (touch law: the whole surface is the hit
/// target), painted only via ColdLight.
fn chip(
    ui: &mut egui::Ui,
    tone: Tone,
    text: &str,
    key: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let inner = cl::brutal(tone).show(ui, |ui| {
        ui.label(RichText::new(text).strong());
    });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("composer-chip", key)),
        Sense::click(),
    )
}

/// A compact square button for the per-clip reorder/remove controls.
fn small_btn(
    ui: &mut egui::Ui,
    glyph: &str,
    key: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let inner = cl::brutal(Tone::Magenta)
        .margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(glyph).strong());
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("composer-sbtn", key)),
        Sense::click(),
    )
}

/// A passive claymorphic plate with a title + hint — the empty/loading state.
fn plate(ui: &mut egui::Ui, title: &str, hint: &str) {
    cl::clay().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(cl::space::S3);
            ui.label(RichText::new(title).heading());
            ui.label(RichText::new(hint).color(cl::ink::SUBTLE));
            ui.add_space(cl::space::S3);
        });
    });
}

/// Seconds → a compact "6s" / "1:04" / "—" label.
fn fmt_secs(secs: Option<u32>) -> String {
    match secs {
        None | Some(0) => "—".into(),
        Some(s) if s < 60 => format!("{s}s"),
        Some(s) => format!("{}:{:02}", s / 60, s % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_rec(name: &str, rel: &str, secs: Option<u32>) -> Record {
        Record {
            rel_path: rel.into(),
            kind: "video".into(),
            name: name.into(),
            seconds: secs,
            ..Default::default()
        }
    }

    #[test]
    fn add_clip_stitches_onto_the_end_and_stays_valid() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("Intro", "inbox/intro.mp4", Some(5)));
        c.add_clip(&video_rec("Body", "inbox/body.mp4", Some(8)));
        assert_eq!(c.layers().len(), 2);
        // Second clip starts where the first ends (end-to-end stitch).
        assert!((c.layers()[0].start - 0.0).abs() < 1e-6);
        assert!((c.layers()[1].start - 5.0).abs() < 1e-6);
        assert!((c.comp.duration() - 13.0).abs() < 1e-6);
        assert!(dpl_comp::validate(&c.comp).is_empty());
        // Layer ids are unique.
        assert_ne!(c.layers()[0].id, c.layers()[1].id);
    }

    #[test]
    fn seconds_missing_falls_back_not_zero() {
        // No probe possible in the test env → the fallback keeps the comp valid
        // (out_pt > in_pt), never a zero-length layer.
        let mut c = Composer::default();
        c.add_clip(&video_rec("Mystery", "inbox/x.mp4", None));
        assert!(c.layers()[0].out_pt > 0.0);
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    #[test]
    fn remove_and_reorder_keep_the_comp_valid() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(4)));
        c.add_clip(&video_rec("C", "inbox/c.mp4", Some(5)));
        let first = c.layers()[0].id.clone();
        c.move_by(&first, 1);
        assert_eq!(c.layers()[1].id, first, "moved later");
        c.remove(&c.layers()[0].id.clone());
        assert_eq!(c.layers().len(), 2);
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    #[test]
    fn fmt_secs_reads_in_plain_words() {
        assert_eq!(fmt_secs(None), "—");
        assert_eq!(fmt_secs(Some(0)), "—");
        assert_eq!(fmt_secs(Some(6)), "6s");
        assert_eq!(fmt_secs(Some(64)), "1:04");
    }
}
