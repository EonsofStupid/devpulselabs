//! Shows — the operator's CONTENT on the rail (doctrine §2, §5.1).
//!
//! One subroute per stream category: the three RATIFIED shows always
//! list (EffingAI · DevPulse News · Eons of Stupidity), and any other
//! show folder found under `content/` at boot appends after them —
//! boot-discovery, leaked once (the main.rs route-string precedent).
//!
//! A show's page = **Episodes · Gallery · Social** (ruling §5.1): the
//! show's episodes, everything media under `content/<show>/`
//! thumbnailed, and its social lane whose ratified disk home is
//! `content/<show>/social/`. This hub is READ-ONLY over content.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const HUB: HubId = HubId("shows");

/// The operator's ratified stream categories (2026-07-23) — always on
/// the rail, folders or not, in the operator's own order.
const RATIFIED: [(&str, &str); 3] = [
    ("effingai", "EffingAI"),
    ("devpulse-news", "DevPulse News"),
    ("eons-of-stupidity", "Eons of Stupidity"),
];

/// The subroute list for THIS boot: the ratified categories, then any
/// other show discovered on disk (kebab id as its label).
fn discover_subroutes(content_root: &Path) -> Vec<SubRoute> {
    let mut subs: Vec<SubRoute> = RATIFIED
        .iter()
        .map(|(id, label)| SubRoute {
            id: SubId(id),
            label,
            blurb: "Episodes · Gallery · Social — the show's world in one place.",
        })
        .collect();
    for (show, _dir) in dpl_doc::list_shows(content_root) {
        if RATIFIED.iter().any(|(id, _)| *id == show) {
            continue;
        }
        let leaked: &'static str = Box::leak(show.into_boxed_str());
        subs.push(SubRoute {
            id: SubId(leaked),
            label: leaked,
            blurb: "A discovered show folder — Episodes · Gallery · Social.",
        });
    }
    subs
}

/// A show page's open section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Section {
    #[default]
    Episodes,
    Gallery,
    Social,
}

impl Section {
    const ALL: [Section; 3] = [Section::Episodes, Section::Gallery, Section::Social];

    fn label(self) -> &'static str {
        match self {
            Section::Episodes => "episodes",
            Section::Gallery => "gallery",
            Section::Social => "social",
        }
    }
}

/// What the gallery makes of a file. Machine files are skipped entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaKind {
    Image,
    Video,
    Skip,
}

fn media_kind(path: &Path) -> MediaKind {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if name == "episode.json"
        || name == "slide.json"
        || name == "events.jsonl"
        || name.ends_with(".meta.json")
    {
        return MediaKind::Skip;
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "gif" => MediaKind::Image,
        "mp4" | "mov" | "mkv" | "webm" => MediaKind::Video,
        _ => MediaKind::Skip,
    }
}

/// Every media file under `dir`, recursively, sorted by rel path.
fn walk_media(dir: &Path) -> Vec<(PathBuf, MediaKind)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                match media_kind(&p) {
                    MediaKind::Skip => {}
                    kind => out.push((p, kind)),
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// file:/// with THREE slashes — two reads as a UNC host (the media
/// block's hard-won law).
fn file_uri(p: &Path) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/"))
}

pub struct ShowsHub {
    spec: &'static HubSpec,
    content_root: PathBuf,
    /// Each show page remembers its open section.
    sections: HashMap<String, Section>,
}

impl ShowsHub {
    /// Discover the shows once and leak the spec — subroutes are
    /// `&'static` by contract; this is a once-per-boot allocation.
    pub fn new() -> Self {
        let content_root = dpl_roots::content_root();
        let subs: &'static [SubRoute] =
            Box::leak(discover_subroutes(&content_root).into_boxed_slice());
        let spec: &'static HubSpec = Box::leak(Box::new(HubSpec {
            id: HUB,
            label: "Shows",
            icon: cl::icon::SHOWS,
            tone: Tone::Violet,
            subroutes: subs,
        }));
        Self {
            spec,
            content_root,
            sections: HashMap::new(),
        }
    }
}

impl Default for ShowsHub {
    fn default() -> Self {
        Self::new()
    }
}

impl Hub for ShowsHub {
    fn spec(&self) -> &'static HubSpec {
        self.spec
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        let Some(sub) = self.spec.subroute(route.sub) else {
            ui.label(
                RichText::new(format!("no show registered for \"{}\"", route.sub))
                    .color(cl::ink::MUTED),
            );
            return;
        };
        self.show_page(ui, route.sub.0, sub.label);
    }
}

impl ShowsHub {
    fn show_page(&mut self, ui: &mut egui::Ui, show: &str, label: &str) {
        let show_dir = self.content_root.join(show);
        let episodes: Vec<(String, PathBuf)> = dpl_doc::list_content_episodes(&self.content_root)
            .into_iter()
            .filter(|(s, _, _)| s == show)
            .map(|(_, ep, dir)| (ep, dir))
            .collect();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(cl::icon::SHOWS)
                    .size(22.0)
                    .color(self.spec.tone.color()),
            );
            ui.label(RichText::new(label).heading().color(cl::ink::STRONG));
            ui.label(
                RichText::new(format!(
                    "{} episode{} · content/{show}",
                    episodes.len(),
                    if episodes.len() == 1 { "" } else { "s" }
                ))
                .small()
                .color(cl::ink::SUBTLE),
            );
        });
        ui.add_space(cl::space::S2);

        let section = self.sections.entry(show.to_string()).or_default();
        ui.horizontal(|ui| {
            for s in Section::ALL {
                let active = *section == s;
                let chip = ui
                    .label(RichText::new(s.label()).monospace().color(if active {
                        self.spec.tone.color()
                    } else {
                        cl::ink::MUTED
                    }))
                    .interact(egui::Sense::click());
                if chip.clicked() {
                    *section = s;
                }
            }
        });
        ui.add_space(cl::space::S2);

        match *section {
            Section::Episodes => episodes_section(ui, self.spec, &episodes, &self.content_root),
            Section::Gallery => gallery_section(ui, &show_dir),
            Section::Social => social_section(ui, show, &show_dir.join("social")),
        }
    }
}

fn episodes_section(
    ui: &mut egui::Ui,
    spec: &HubSpec,
    episodes: &[(String, PathBuf)],
    content_root: &Path,
) {
    if episodes.is_empty() {
        ui.label(
            RichText::new(
                "no episodes yet — clone a seed in Workbench › Episodes and it lands here",
            )
            .color(cl::ink::MUTED),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (ep, dir) in episodes {
                let resp = cl::card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // Either shape (LS3b): chapters are lessons in an
                    // episode built from lessons.
                    match dpl_doc::episode::summary(dir, content_root) {
                        Some(doc) => {
                            let slides = doc.cards();
                            let chapters = doc.chapters.iter().filter(|c| c.2 > 0).count();
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&doc.title).strong().color(cl::ink::STRONG));
                                ui.label(
                                    RichText::new(ep.as_str())
                                        .monospace()
                                        .small()
                                        .color(cl::ink::SUBTLE),
                                );
                            });
                            ui.label(
                                RichText::new(format!(
                                    "{} chapter{} · {} slide{}",
                                    chapters,
                                    if chapters == 1 { "" } else { "s" },
                                    slides,
                                    if slides == 1 { "" } else { "s" }
                                ))
                                .small()
                                .color(spec.tone.color()),
                            );
                        }
                        // A broken episode.json is NAMED, never hidden.
                        None => {
                            let e = dpl_doc::episode::open(dir)
                                .err()
                                .map_or_else(|| "it cannot be read".to_owned(), |e| e.to_string());
                            ui.label(RichText::new(ep.as_str()).strong().color(cl::ink::STRONG));
                            ui.label(
                                RichText::new(format!("failed to load: {e}"))
                                    .small()
                                    .color(cl::ink::MUTED),
                            );
                        }
                    }
                });
                let _ = resp;
                ui.add_space(cl::space::S2);
            }
        });
}

fn gallery_section(ui: &mut egui::Ui, show_dir: &Path) {
    let media = walk_media(show_dir);
    if media.is_empty() {
        ui.label(
            RichText::new("nothing here yet — episode assets and social media land here")
                .color(cl::ink::MUTED),
        );
        return;
    }
    let base = show_dir;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let card = egui::vec2(236.0, 160.0);
            ui.horizontal_wrapped(|ui| {
                for (path, kind) in &media {
                    ui.vertical(|ui| {
                        let (rect, _) = ui.allocate_exact_size(card, egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, cl::RADIUS_SM, cl::surface::WELL);
                        match kind {
                            MediaKind::Image => {
                                egui::Image::new(file_uri(path))
                                    .corner_radius(cl::RADIUS_SM)
                                    .paint_at(ui, rect);
                            }
                            MediaKind::Video | MediaKind::Skip => {
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "video",
                                    egui::FontId::monospace(12.0),
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
                        let rel = path
                            .strip_prefix(base)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .replace('\\', "/");
                        ui.add_sized(
                            [card.x, 18.0],
                            egui::Label::new(
                                RichText::new(rel).monospace().small().color(cl::ink::MUTED),
                            )
                            .truncate(),
                        );
                    });
                }
            });
        });
}

fn social_section(ui: &mut egui::Ui, show: &str, social_dir: &Path) {
    let media = walk_media(social_dir);
    if media.is_empty() {
        ui.label(
            RichText::new(format!(
                "no social content yet — files under content/{show}/social/ appear here"
            ))
            .color(cl::ink::MUTED),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (path, kind) in &media {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(match kind {
                            MediaKind::Image => "image",
                            MediaKind::Video => "video",
                            MediaKind::Skip => "file",
                        })
                        .monospace()
                        .small()
                        .color(cl::ink::SUBTLE),
                    );
                    let rel = path
                        .strip_prefix(social_dir)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    ui.label(RichText::new(rel).color(cl::ink::BASE));
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ratified_categories_always_list_in_the_operators_order() {
        let scratch = std::env::temp_dir().join("dpl-shows-test-empty");
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).unwrap();
        let subs = discover_subroutes(&scratch);
        let ids: Vec<&str> = subs.iter().map(|s| s.id.0).collect();
        assert_eq!(ids, vec!["effingai", "devpulse-news", "eons-of-stupidity"]);
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_discovered_show_appends_after_the_ratified_three() {
        let scratch = std::env::temp_dir().join("dpl-shows-test-extra");
        let _ = std::fs::remove_dir_all(&scratch);
        // A show = a folder holding at least one episode (list_shows law).
        let ep = scratch.join("garage-sessions/pilot");
        std::fs::create_dir_all(&ep).unwrap();
        std::fs::write(
            ep.join("episode.json"),
            r#"{ "v": 1, "title": "Pilot", "chapters": [] }"#,
        )
        .unwrap();
        let subs = discover_subroutes(&scratch);
        let ids: Vec<&str> = subs.iter().map(|s| s.id.0).collect();
        assert_eq!(
            ids,
            vec![
                "effingai",
                "devpulse-news",
                "eons-of-stupidity",
                "garage-sessions"
            ]
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn media_kind_classifies_and_machine_files_skip() {
        use MediaKind::*;
        let k = |p: &str| media_kind(Path::new(p));
        assert_eq!(k("a/pix.png"), Image);
        assert_eq!(k("a/clip.MP4"), Video);
        assert_eq!(k("a/episode.json"), Skip);
        assert_eq!(k("a/slide.json"), Skip);
        assert_eq!(k("a/events.jsonl"), Skip);
        assert_eq!(k("a/pix.png.meta.json"), Skip);
        assert_eq!(k("a/notes.txt"), Skip);
    }
}
