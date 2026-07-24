//! ViziWiz (C4) — ink over an image artifact, flatten, file.
//!
//! The picker wall lists every indexed image; tapping one opens the
//! editor: strokes draw over the image (round caps, six ColdLight
//! tones, width 2–24, undo/clear), and **Flatten & file** composites
//! base + ink into ONE NEW PNG through the forced-metadata inbox flow
//! (blank name seeds `<src>-annot`, blank tags seed "annotated"). The
//! SOURCE IS NEVER MODIFIED — the never-mutate content law; the deck's
//! stream-time ink (D3) is a different system and untouched.

use dpl_coldlight::{self as cl, Tone};
use dpl_engine_media::ink::InkStroke;
use dpl_engine_steps::{normalize_meta, CaptureMeta};
use egui::{RichText, Sense};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::{Path, PathBuf};

use crate::{filter_chip, thumb_uri};

/// The six ink tones — the old fx accent set is exactly ColdLight's.
const INK_TONES: [Tone; 6] = [
    Tone::Cyan,
    Tone::Magenta,
    Tone::Lime,
    Tone::Coral,
    Tone::Violet,
    Tone::Amber,
];

/// One stroke, IMAGE pixel space (resolution-true regardless of zoom).
struct Stroke {
    tone: usize,
    width: f32,
    points: Vec<[f32; 2]>,
}

/// The open editor.
pub(crate) struct VizEdit {
    rel: String,
    src_name: String,
    tex: Option<egui::TextureHandle>,
    size: [usize; 2],
    strokes: Vec<Stroke>,
    drawing: Option<Stroke>,
    tone: usize,
    width: f32,
    name: String,
    tags: String,
    library: bool,
    show: String,
    episode: String,
}

/// The VIZIWIZ surface: the image wall, or one open editor.
pub(crate) struct VizView {
    rows: Vec<dpl_index::Record>,
    open: Option<VizEdit>,
    status: String,
}

impl VizView {
    pub(crate) fn load() -> Self {
        let (rows, status) =
            match dpl_index::Index::open_default().and_then(|ix| ix.list(Some("image"), None)) {
                Ok(rows) => {
                    let status = format!("{} images", rows.len());
                    (rows, status)
                }
                Err(e) => (Vec::new(), e),
            };
        Self {
            rows,
            open: None,
            status,
        }
    }
}

/// The metadata the flatten files with — shared by the editor action
/// and the headless drill.
struct MetaIn {
    name: String,
    tags: String,
    library: bool,
    show: String,
    episode: String,
}

/// Strip a trailing `_vN` so `pix_v1` seeds `pix-annot` (old behavior).
fn strip_v(name: &str) -> &str {
    match name.rsplit_once("_v") {
        Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => head,
        _ => name,
    }
}

/// Flatten + file, exactly once: forced metadata → a NEW unique inbox
/// PNG → sidecar + index (tags + episode association). The source is
/// read-only throughout.
fn file_flattened(
    root: &Path,
    rel: &str,
    src_name: &str,
    ink: &[InkStroke],
    meta_in: &MetaIn,
) -> Result<String, String> {
    let name = if meta_in.name.trim().is_empty() {
        format!("{}-annot", strip_v(src_name))
    } else {
        meta_in.name.trim().to_string()
    };
    let mut tags: Vec<String> = meta_in
        .tags
        .split([' ', ','])
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();
    if tags.is_empty() {
        tags = vec!["annotated".into()];
    }
    let meta = normalize_meta(CaptureMeta {
        library: meta_in.library,
        show: meta_in.show.trim().to_string(),
        episode: meta_in.episode.trim().to_string(),
        name,
        tags,
    })?;

    let inbox = root.join("inbox");
    std::fs::create_dir_all(&inbox).map_err(|e| format!("inbox: {e}"))?;
    let dest = unique_png(&inbox, &meta.name);
    let src_abs = root.join(rel);
    dpl_engine_media::ink::flatten_ink(&src_abs, &dest, ink)?;

    let mut record = dpl_index::record_for_capture(&dest, root, "image", "viziwiz");
    record.tags = meta.tags.clone();
    if !meta.library {
        record.show_id = Some(meta.show.clone());
        record.episode_folder = Some(meta.episode.clone());
    }
    let warn = match dpl_index::sidecar::write(&dest, &record)
        .and_then(|()| dpl_index::Index::open_default()?.upsert(&record))
    {
        Ok(()) => String::new(),
        Err(e) => format!(" · index: {e}"),
    };
    let file = dest
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(format!("flattened → inbox/{file} (source untouched){warn}"))
}

/// Never-clobber PNG name.
fn unique_png(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.png"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.png")))
        .find(|p| !p.exists())
        .expect("unbounded range")
}

/// The headless drill (`--ink-drill <rel>`): a corner-to-corner stroke
/// through the REAL flatten path with blank metadata (auto `-annot`),
/// asserting the source's bytes never move.
pub(crate) fn drill(root: &Path, rel: &str) -> Result<String, String> {
    let src_abs = root.join(rel);
    let img = image::open(&src_abs).map_err(|e| format!("open {rel}: {e}"))?;
    let (w, h) = (img.width() as f32, img.height() as f32);
    let before = std::fs::read(&src_abs).map_err(|e| format!("read {rel}: {e}"))?;
    let src_name = Path::new(rel)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".into());
    let ink = [InkStroke {
        color: [34, 211, 238],
        width: 8.0,
        points: vec![[0.0, 0.0], [w - 1.0, h - 1.0]],
    }];
    let meta = MetaIn {
        name: String::new(),
        tags: String::new(),
        library: true,
        show: String::new(),
        episode: String::new(),
    };
    let msg = file_flattened(root, rel, &src_name, &ink, &meta)?;
    let after = std::fs::read(&src_abs).map_err(|e| format!("re-read {rel}: {e}"))?;
    if before != after {
        return Err(format!(
            "the source CHANGED — never-mutate violated ({rel})"
        ));
    }
    Ok(msg)
}

/// The route surface.
pub(crate) fn surface(view: &mut Option<VizView>, ui: &mut egui::Ui) {
    let view = view.get_or_insert_with(VizView::load);
    if view.open.is_some() {
        editor(view, ui);
        return;
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("ViziWiz — draw on any image; flattening files a NEW inbox image")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(&view.status).small().color(cl::ink::SUBTLE));
        });
    });
    ui.add_space(cl::space::S2);
    if view.rows.is_empty() {
        ui.label(
            RichText::new("no image artifacts yet — Pix something first").color(cl::ink::MUTED),
        );
        return;
    }

    // The picker wall — same geometry family as the Gallery.
    let root = dpl_engine_capture::content_root();
    let mut open: Option<(String, String)> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail = ui.available_width();
            let cols = ((avail / 240.0).floor() as usize).clamp(1, 6);
            let card_w = (avail - cl::space::S3 * (cols - 1) as f32) / cols as f32;
            let thumb_h = card_w * 9.0 / 16.0;
            egui::Grid::new("dpl_viziwiz_grid")
                .num_columns(cols)
                .spacing([cl::space::S3, cl::space::S3])
                .show(ui, |ui| {
                    for (n, r) in view.rows.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.set_width(card_w);
                            let (rect, resp) =
                                ui.allocate_exact_size(egui::vec2(card_w, thumb_h), Sense::click());
                            ui.pilot_track(
                                &resp,
                                NodeMetadata::new(
                                    PilotId::new("viziwiz.pick").child(&PilotKey::new(&r.rel_path)),
                                    egui_pilot_ai::protocol::Role::Button,
                                    "Annotate this image",
                                ),
                            );
                            ui.painter()
                                .rect_filled(rect, cl::RADIUS_SM, cl::surface::WELL);
                            if let Some(uri) = thumb_uri(r, &root) {
                                egui::Image::new(uri)
                                    .corner_radius(cl::RADIUS_SM)
                                    .paint_at(ui, rect);
                            }
                            ui.painter().rect_stroke(
                                rect,
                                cl::RADIUS_SM,
                                egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                                egui::StrokeKind::Inside,
                            );
                            if resp.clicked() {
                                open = Some((r.rel_path.clone(), r.name.clone()));
                            }
                            ui.label(RichText::new(&r.name).small().color(cl::ink::BASE));
                        });
                        if (n + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

    if let Some((rel, name)) = open {
        match image::open(root.join(&rel)) {
            Ok(img) => {
                view.open = Some(VizEdit {
                    rel,
                    src_name: name,
                    tex: None,
                    size: [img.width() as usize, img.height() as usize],
                    strokes: Vec::new(),
                    drawing: None,
                    tone: 0,
                    width: 6.0,
                    name: String::new(),
                    tags: String::new(),
                    library: true,
                    show: String::new(),
                    episode: String::new(),
                });
            }
            Err(e) => view.status = format!("open {rel}: {e}"),
        }
    }
}

/// The open editor: toolbar · canvas · metadata · flatten.
fn editor(view: &mut VizView, ui: &mut egui::Ui) {
    let root = dpl_engine_capture::content_root();
    let mut close = false;
    let mut flatten = false;
    // Taken out for the frame so `view.status` stays writable; restored
    // at the end unless back closed the editor.
    let Some(mut e) = view.open.take() else {
        return;
    };

    ui.horizontal(|ui| {
        let back = ui
            .label(RichText::new("‹ images").color(Tone::Magenta.color()))
            .interact(Sense::click());
        ui.pilot_track(
            &back,
            NodeMetadata::new(
                PilotId::new("viziwiz.back"),
                egui_pilot_ai::protocol::Role::Button,
                "Back to the images",
            ),
        );
        if back.clicked() {
            close = true;
        }
        ui.label(RichText::new(&e.src_name).strong().color(cl::ink::STRONG));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(&view.status).small().color(cl::ink::SUBTLE));
        });
    });
    ui.add_space(cl::space::S1);

    // Toolbar: tone dots · width · undo · clear.
    ui.horizontal(|ui| {
        for (i, tone) in INK_TONES.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("viziwiz.tone").child(&PilotKey::new(tone.hue_id())),
                    egui_pilot_ai::protocol::Role::Button,
                    format!("ink {}", tone.hue_id()),
                )
                .value(if e.tone == i { "active" } else { "available" }),
            );
            ui.painter().circle_filled(rect.center(), 8.0, tone.color());
            if e.tone == i {
                ui.painter().circle_stroke(
                    rect.center(),
                    11.0,
                    egui::Stroke::new(1.5, cl::ink::STRONG),
                );
            }
            if resp.clicked() {
                e.tone = i;
            }
        }
        ui.add_space(cl::space::S2);
        ui.add(egui::Slider::new(&mut e.width, 2.0..=24.0).text("width"));
        if cl::tip(filter_chip(ui, "Undo", false), "Remove the last stroke").clicked() {
            e.strokes.pop();
        }
        if cl::tip(filter_chip(ui, "Clear", false), "Remove every stroke").clicked() {
            e.strokes.clear();
        }
    });
    ui.add_space(cl::space::S1);

    // The canvas: image fit to the space, strokes captured in image
    // coordinates so the flatten is resolution-true.
    if e.tex.is_none() {
        if let Ok(img) = image::open(root.join(&e.rel)) {
            let rgba = img.to_rgba8();
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                &rgba,
            );
            e.tex = Some(ui.ctx().load_texture(
                format!("viziwiz-{}", e.rel),
                color,
                egui::TextureOptions::LINEAR,
            ));
        }
    }
    let (img_w, img_h) = (e.size[0] as f32, e.size[1] as f32);
    let avail_w = ui.available_width();
    let avail_h = (ui.available_height() - 130.0).max(160.0); // metadata rows below
    let scale = (avail_w / img_w).min(avail_h / img_h).min(1.5);
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(img_w * scale, img_h * scale),
        Sense::click_and_drag(),
    );
    ui.pilot_track(
        &resp,
        NodeMetadata::new(
            PilotId::new("viziwiz.canvas"),
            egui_pilot_ai::protocol::Role::Region,
            "Ink canvas",
        )
        .value(format!("{} strokes", e.strokes.len())),
    );
    let to_image = |p: egui::Pos2| [(p.x - rect.min.x) / scale, (p.y - rect.min.y) / scale];
    if resp.drag_started() {
        if let Some(p) = resp.interact_pointer_pos() {
            e.drawing = Some(Stroke {
                tone: e.tone,
                width: e.width,
                points: vec![to_image(p)],
            });
        }
    }
    if resp.dragged() {
        if let (Some(stroke), Some(p)) = (e.drawing.as_mut(), resp.interact_pointer_pos()) {
            stroke.points.push(to_image(p));
        }
    }
    if resp.drag_stopped() {
        if let Some(stroke) = e.drawing.take() {
            if stroke.points.len() > 1 {
                e.strokes.push(stroke);
            }
        }
    }
    let painter = ui.painter_at(rect);
    match &e.tex {
        Some(tex) => {
            painter.image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        None => {
            painter.rect_filled(rect, 0, cl::surface::WELL);
        }
    }
    let paint_stroke = |s: &Stroke| {
        let color = INK_TONES[s.tone].color();
        for pair in s.points.windows(2) {
            painter.line_segment(
                [
                    rect.min + egui::vec2(pair[0][0] * scale, pair[0][1] * scale),
                    rect.min + egui::vec2(pair[1][0] * scale, pair[1][1] * scale),
                ],
                egui::Stroke::new(s.width * scale, color),
            );
        }
    };
    for s in &e.strokes {
        paint_stroke(s);
    }
    if let Some(s) = &e.drawing {
        paint_stroke(s);
    }
    painter.rect_stroke(
        rect,
        0,
        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
        egui::StrokeKind::Inside,
    );
    ui.add_space(cl::space::S2);

    // Metadata + flatten — the forced-metadata shape (C2's card family).
    ui.horizontal(|ui| {
        ui.label(RichText::new("name").small().color(cl::ink::MUTED));
        let name = ui.add(
            egui::TextEdit::singleline(&mut e.name)
                .hint_text(format!("{}-annot", strip_v(&e.src_name)))
                .desired_width(180.0),
        );
        ui.pilot_track(
            &name,
            NodeMetadata::new(
                PilotId::new("viziwiz.meta.name"),
                egui_pilot_ai::protocol::Role::TextInput,
                "flattened name",
            )
            .value(e.name.clone()),
        );
        ui.label(RichText::new("tags").small().color(cl::ink::MUTED));
        let tags = ui.add(
            egui::TextEdit::singleline(&mut e.tags)
                .hint_text("annotated")
                .desired_width(140.0),
        );
        ui.pilot_track(
            &tags,
            NodeMetadata::new(
                PilotId::new("viziwiz.meta.tags"),
                egui_pilot_ai::protocol::Role::TextInput,
                "flattened tags",
            )
            .value(e.tags.clone()),
        );
        if ui.selectable_label(e.library, "General").clicked() {
            e.library = true;
        }
        let episode = ui.selectable_label(!e.library, "Episode");
        ui.pilot_track(
            &episode,
            NodeMetadata::new(
                PilotId::new("viziwiz.meta.dest"),
                egui_pilot_ai::protocol::Role::Button,
                "destination",
            )
            .value(if e.library { "general" } else { "episode" }),
        );
        if episode.clicked() {
            e.library = false;
        }
        if !e.library {
            ui.add(
                egui::TextEdit::singleline(&mut e.show)
                    .hint_text("show")
                    .desired_width(70.0),
            );
            ui.add(
                egui::TextEdit::singleline(&mut e.episode)
                    .hint_text("episode")
                    .desired_width(70.0),
            );
        }
        let go = cl::tip(
            filter_chip(ui, "Flatten & file", false),
            "Save your drawing as a NEW image in the inbox — the original is never touched",
        );
        ui.pilot_track(
            &go,
            NodeMetadata::new(
                PilotId::new("viziwiz.flatten"),
                egui_pilot_ai::protocol::Role::Button,
                "Flatten and file to inbox",
            ),
        );
        if go.clicked() {
            flatten = true;
        }
    });

    if flatten {
        if e.strokes.is_empty() {
            view.status = "draw something first — flatten files the ink".into();
        } else {
            let ink: Vec<InkStroke> = e
                .strokes
                .iter()
                .map(|s| {
                    let c = INK_TONES[s.tone].color();
                    InkStroke {
                        color: [c.r(), c.g(), c.b()],
                        width: s.width,
                        points: s.points.clone(),
                    }
                })
                .collect();
            let meta = MetaIn {
                name: e.name.clone(),
                tags: e.tags.clone(),
                library: e.library,
                show: e.show.clone(),
                episode: e.episode.clone(),
            };
            match file_flattened(&root, &e.rel, &e.src_name, &ink, &meta) {
                Ok(msg) => {
                    e.strokes.clear();
                    view.status = msg;
                }
                Err(err) => view.status = err, // strokes + card kept
            }
        }
    }
    if !close {
        view.open = Some(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_v_only_eats_real_version_suffixes() {
        assert_eq!(strip_v("pix_v1"), "pix");
        assert_eq!(strip_v("pix_v12"), "pix");
        assert_eq!(strip_v("pix_vNext"), "pix_vNext");
        assert_eq!(strip_v("pix"), "pix");
    }
}
