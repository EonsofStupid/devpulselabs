//! The PREVIEW player — look before it lands.
//!
//! A modal over the editor: tap a card in the Library or a Browse window and
//! the thing plays (video, Lottie) or shows (image) right there. Two ways out,
//! both one tap — **close** and nothing happened, or **add to edit** and it
//! becomes a new layer on the open comp.
//!
//! It is a PREVIEW, never a second editor: no trimming, no properties, no
//! saving. Deliberately modal rather than a peer viewport — a preview is
//! transient, so Esc and click-outside must close it and it must never get
//! lost behind the main window (Browse stays a viewport because Browse is a
//! workspace you keep open).
//!
//! Add-to-edit routes through the ONE import seam (`crate::import`) and then
//! `Composer::add_clip`. A second import path is never invented here.

use crate::import::media_kind;
use dpl_coldlight::{self as cl, Tone};
use dpl_index::Record;
use dpl_video::clip::ClipPlayer;
use egui::{RichText, Sense};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::PathBuf;

/// What the preview was opened on. An indexed record already lives in the
/// library; a bare path is a file the operator is looking at in a folder and
/// has not imported yet — the distinction decides what "add to edit" must do
/// first.
#[derive(Clone)]
pub(crate) enum Source {
    Indexed(Box<Record>),
    Loose(PathBuf),
}

impl Source {
    pub(crate) fn path(&self, root: &std::path::Path) -> PathBuf {
        match self {
            Source::Indexed(r) => root.join(&r.rel_path),
            Source::Loose(p) => p.clone(),
        }
    }
    fn name(&self) -> String {
        match self {
            Source::Indexed(r) => r.name.clone(),
            Source::Loose(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.to_string_lossy().into_owned()),
        }
    }
}

/// What the preview is showing — decided once on open from the extension, so
/// the paint path never re-sniffs.
pub(crate) enum Kind {
    /// Decoded frames through the ffmpeg pipe.
    Video(Box<ClipPlayer>),
    /// A still, painted straight from a `file://` uri.
    Still(String),
    /// AUDIO HAS NO PLAYBACK. Nothing in this workspace can make sound —
    /// no rodio, no cpal, no ffplay (verified by census 2026-08-05). Rather
    /// than fake a transport that does nothing, the card SAYS so and still
    /// offers "add to edit", which works perfectly well.
    SilentAudio,
    /// The file is media we accept but cannot show yet (an element/pack
    /// file). Honest plate, add-to-edit still available.
    Unviewable(&'static str),
}

pub(crate) struct Preview {
    pub(crate) source: Source,
    kind: Kind,
    /// Said out loud under the frame when the engine is missing, instead of
    /// showing a black rectangle forever.
    note: Option<String>,
}

impl Preview {
    /// Open a preview on `source`. Never fails — an unplayable file becomes an
    /// honest plate rather than a refusal, because the operator can still add
    /// it to the edit.
    pub(crate) fn open(source: Source, root: &std::path::Path) -> Self {
        let path = source.path(root);
        let kind = match media_kind(&path) {
            Some("video") => {
                if dpl_video::ffmpeg_available() {
                    Kind::Video(Box::new(ClipPlayer::open(path.clone())))
                } else {
                    Kind::Unviewable("The media engine is missing — reinstall the app")
                }
            }
            Some("image") => Kind::Still(format!(
                "file:///{}",
                path.to_string_lossy().replace('\\', "/")
            )),
            Some("audio") => Kind::SilentAudio,
            Some("lottie") => Kind::Unviewable("Lottie preview lands with the player pass"),
            _ => Kind::Unviewable("No preview for this kind yet"),
        };
        Self {
            source,
            kind,
            note: None,
        }
    }

    /// Keep a playing clip advancing. Called once a frame while open.
    pub(crate) fn tick(&mut self, ctx: &egui::Context) {
        if let Kind::Video(p) = &mut self.kind {
            if p.playing() {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
    }
}

/// What the operator asked the preview to do.
pub(crate) enum Act {
    Close,
    AddToEdit,
}

/// The modal body. Returns an action when the operator picked one.
///
/// Esc and click-outside close it — `egui::Modal` gives both, and the preview
/// owning no unsaved state means closing can never lose work.
pub(crate) fn modal(ctx: &egui::Context, pv: &mut Preview, tone: Tone) -> Option<Act> {
    let mut act = None;
    let title = pv.source.name();

    let resp = egui::Modal::new(egui::Id::new("composer.preview"))
        .backdrop_color(egui::Color32::from_black_alpha(184))
        .frame(
            egui::Frame::new()
                .fill(cl::surface::RAISED)
                .stroke(egui::Stroke::new(1.0, tone.color()))
                .corner_radius(egui::CornerRadius::same(cl::RADIUS_LG))
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ctx, |ui| {
            let room = ctx.content_rect();
            ui.set_width((room.width() * 0.62).min(980.0));

            // ── title row ──────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label(RichText::new(&title).strong().color(cl::ink::STRONG));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let x = cl::tip(
                        cl::icon_label(ui, "close", 16.0, cl::ink::SUBTLE)
                            .interact(Sense::click()),
                        "Close this preview — nothing is imported",
                    );
                    if x.clicked() {
                        act = Some(Act::Close);
                    }
                });
            });
            ui.add_space(cl::space::S2);

            // ── the stage ──────────────────────────────────────────────
            let w = ui.available_width();
            let (stage, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), Sense::hover());
            ui.painter()
                .rect_filled(stage, cl::RADIUS_SM, cl::surface::WELL);
            match &mut pv.kind {
                Kind::Video(p) => match p.texture(ui.ctx()) {
                    Some(tex) => {
                        let uv = dpl_video::cover_uv(
                            dpl_video::DECODE_W as f32 / dpl_video::DECODE_H as f32,
                            stage.aspect_ratio(),
                        );
                        ui.painter().image(tex, stage, uv, egui::Color32::WHITE);
                    }
                    None => plate(ui, stage, "Loading the frame…"),
                },
                Kind::Still(uri) => {
                    egui::Image::new(uri.as_str())
                        .corner_radius(cl::RADIUS_SM)
                        .paint_at(ui, stage);
                }
                Kind::SilentAudio => plate(
                    ui,
                    stage,
                    "Audio — this build has no sound engine yet, so it cannot be played here.\n\
                     Add to edit still works.",
                ),
                Kind::Unviewable(why) => plate(ui, stage, why),
            }

            // ── transport (video only — never a control that does nothing) ─
            if let Kind::Video(p) = &mut pv.kind {
                ui.add_space(cl::space::S1);
                ui.horizontal(|ui| {
                    let playing = p.playing();
                    let r = cl::tip(
                        icon_btn(
                            ui,
                            if playing {
                                "transport-pause"
                            } else {
                                "transport-play"
                            },
                            "pv-play",
                        ),
                        "Play or pause this preview",
                    );
                    ui.pilot_track(
                        &r,
                        NodeMetadata::new(
                            PilotId::new("composer.preview").child(&PilotKey::new("play")),
                            Role::Button,
                            "Play or pause the preview",
                        )
                        .value(if playing { "playing" } else { "paused" }),
                    );
                    if r.clicked() {
                        p.toggle();
                    }
                    if cl::tip(
                        icon_btn(ui, "transport-start", "pv-start"),
                        "Back to the beginning",
                    )
                    .clicked()
                    {
                        p.seek(0.0);
                    }
                    ui.label(
                        RichText::new(crate::mmss(p.position()))
                            .monospace()
                            .color(cl::ink::SUBTLE),
                    );
                });
            }

            if let Some(n) = &pv.note {
                ui.label(RichText::new(n).small().color(cl::ink::MUTED));
            }

            // ── the two ways out ───────────────────────────────────────
            ui.add_space(cl::space::S2);
            ui.horizontal(|ui| {
                let add = cl::tip(
                    word_btn(ui, "＋ Add to edit", Tone::Lime, "pv-add"),
                    "Drop this onto the timeline as a new layer — one undo takes it back",
                );
                ui.pilot_track(
                    &add,
                    NodeMetadata::new(
                        PilotId::new("composer.preview").child(&PilotKey::new("add")),
                        Role::Button,
                        "Add this to the edit",
                    ),
                );
                if add.clicked() {
                    act = Some(Act::AddToEdit);
                }
                let close = cl::tip(
                    word_btn(ui, "Close", Tone::Cyan, "pv-close"),
                    "Close without adding anything",
                );
                if close.clicked() {
                    act = Some(Act::Close);
                }
            });
        });

    // Click-outside and Esc both close — the modal owns no unsaved state, so
    // dismissing can never lose work.
    if resp.should_close() {
        act = Some(Act::Close);
    }
    act
}

fn plate(ui: &egui::Ui, rect: egui::Rect, text: &str) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(13.0),
        cl::ink::MUTED,
    );
}

fn icon_btn(ui: &mut egui::Ui, icon: &str, key: &str) -> egui::Response {
    let inner = cl::brutal(Tone::Magenta)
        .margin(egui::Margin::symmetric(7, 4))
        .show(ui, |ui| {
            let _ = cl::icon_label(ui, icon, 15.0, cl::ink::STRONG);
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("pv-btn", key)),
        Sense::click(),
    )
}

fn word_btn(ui: &mut egui::Ui, label: &str, tone: Tone, key: &str) -> egui::Response {
    let inner = cl::brutal(tone)
        .margin(egui::Margin::symmetric(11, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().strong());
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("pv-word", key)),
        Sense::click(),
    )
}
