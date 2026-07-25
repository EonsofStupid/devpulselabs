//! Browse windows — the operator's "separate windows appear on click":
//! one native peer viewport per open folder, a thumbnail grid, click a
//! file to pull it into the project. Pure state + the window BODY; the
//! Composer owns viewport summoning and drains `picked`.

use crate::import::media_kind;
use dpl_coldlight::{self as cl, Tone};
use egui::{RichText, Sense};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The deep-search result cap — SAID in the UI, never a silent cut.
pub const SEARCH_CAP: usize = 500;
/// Mogrt zip-reads per painted frame — a 300-card folder fills over a
/// few frames instead of hitching the window once.
const THUMBS_PER_FRAME: usize = 8;

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
    /// The filter/search text: typing filters the open folder live;
    /// Enter launches the deep search below.
    pub query: String,
    /// A deep search under `cwd`, running or finished, off the UI
    /// thread. None = the ordinary folder view.
    pub search: Option<Arc<Mutex<Search>>>,
    /// Mogrt preview thumbs (R2a.2), lazily read from each file's
    /// embedded `thumb.png`; None = tried, no thumb (badge card).
    pub thumbs: HashMap<PathBuf, Option<Arc<[u8]>>>,
    pub open: bool,
}

/// One deep search: the worker fills `hits` once and drops `running`.
pub struct Search {
    pub hits: Vec<Entry>,
    pub running: bool,
}

#[derive(Clone, Copy)]
pub enum EntryKind {
    Dir,
    Image,
    Video,
    Audio,
    Element,
}

#[derive(Clone)]
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
            query: String::new(),
            search: None,
            thumbs: HashMap::new(),
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
        // Navigation disarms a pending ingest confirm (the two-tap law)
        // and leaves any search behind — new folder, fresh view.
        self.arm = None;
        self.query.clear();
        self.search = None;
    }

    /// Launch the deep search: every media file under `cwd` whose name
    /// matches the query, walked OFF the UI thread (70k-file packs on a
    /// USB disk must never freeze the window).
    fn start_search(&mut self) {
        let state = Arc::new(Mutex::new(Search {
            hits: Vec::new(),
            running: true,
        }));
        self.search = Some(Arc::clone(&state));
        let root = self.cwd.clone();
        let query = self.query.clone();
        let _ = std::thread::Builder::new()
            .name("dpl-browse-search".into())
            .spawn(move || {
                let hits = search_files(&root, &query, SEARCH_CAP);
                let mut s = state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                s.hits = hits;
                s.running = false;
            });
    }
}

/// `media_kind`'s string → the card kind.
fn entry_kind(kind: &str) -> EntryKind {
    match kind {
        "video" => EntryKind::Video,
        "audio" => EntryKind::Audio,
        "element" => EntryKind::Element,
        _ => EntryKind::Image,
    }
}

/// Every media file under `root` whose NAME contains `query`
/// (case-insensitive), dot-junk skipped, capped at `cap`, name-sorted —
/// the deep-search worker's whole job, pure and testable.
pub fn search_files(root: &Path, query: &str, cap: usize) -> Vec<Entry> {
    let q = query.to_lowercase();
    let mut stack = vec![root.to_path_buf()];
    let mut out = Vec::new();
    'walk: while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if let Some(kind) = media_kind(&path) {
                if name.to_lowercase().contains(&q) {
                    out.push(Entry {
                        path,
                        name,
                        kind: entry_kind(kind),
                    });
                    if out.len() >= cap {
                        break 'walk;
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
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
            files.push(Entry {
                path,
                name,
                kind: entry_kind(kind),
            });
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
    // The filter/search line: typing filters this folder live; Enter
    // searches EVERYTHING under it (off-thread, capped honestly).
    let mut launch = false;
    let mut clear = false;
    ui.horizontal(|ui| {
        let field = egui::TextEdit::singleline(&mut b.query)
            .hint_text("filter this folder — Enter searches every folder under it")
            .desired_width(300.0);
        let fr = cl::tip(
            ui.add(field),
            "Type to narrow what you see; press Enter to search every folder \
             under this one by name (first 500 matches)",
        );
        ui.pilot_track(
            &fr,
            NodeMetadata::new(
                PilotId::new("browse.search").child(&PilotKey::new(format!("{ns}"))),
                Role::TextInput,
                "Search this pack",
            )
            .value(b.query.clone()),
        );
        if fr.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && !b.query.trim().is_empty()
        {
            launch = true;
        }
        if (b.search.is_some() || !b.query.is_empty())
            && cl::tip(
                bar_chip(ui, "x", ns, "clear-search"),
                "Clear the search and show the folder again",
            )
            .clicked()
        {
            clear = true;
        }
    });
    if launch {
        b.start_search();
    }
    if clear {
        b.query.clear();
        b.search = None;
    }
    ui.add_space(cl::space::S2);

    // Snapshot the search OUTSIDE the paint (never hold the lock across
    // a frame): running flag + finished hits.
    let search_view: Option<(bool, Vec<Entry>)> = b.search.as_ref().map(|s| {
        let s = s.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        (
            s.running,
            if s.running {
                Vec::new()
            } else {
                s.hits.clone()
            },
        )
    });
    let here = b
        .cwd
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| b.cwd.to_string_lossy().into_owned());
    let entries = b.entries.as_deref().unwrap_or(&[]);
    let mut picked: Option<PathBuf> = None;
    let thumbs = &mut b.thumbs;
    let mut thumb_budget = THUMBS_PER_FRAME;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| match &search_view {
            Some((true, _)) => {
                ui.label(
                    RichText::new(format!("Searching everything under {here}…"))
                        .color(cl::ink::MUTED),
                );
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(200));
            }
            Some((false, hits)) => {
                let line = if hits.is_empty() {
                    format!("Nothing named like \"{}\" under {here}.", b.query)
                } else if hits.len() >= SEARCH_CAP {
                    format!(
                        "{SEARCH_CAP}+ found under {here} — the first {SEARCH_CAP} shown; \
                         narrow the search for the rest."
                    )
                } else {
                    format!("{} found under {here}.", hits.len())
                };
                ui.label(RichText::new(line).small().color(cl::ink::SUBTLE));
                ui.add_space(cl::space::S1);
                let refs: Vec<&Entry> = hits.iter().collect();
                if let Some(i) = entry_grid(ui, &refs, ns, "hits", thumbs, &mut thumb_budget) {
                    picked = Some(hits[i].path.clone());
                }
            }
            None => {
                let q = b.query.to_lowercase();
                let shown: Vec<&Entry> = entries
                    .iter()
                    .filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q))
                    .collect();
                if shown.is_empty() {
                    ui.label(
                        RichText::new(if q.is_empty() {
                            "Nothing here — no folders and no media files this window imports."
                                .to_string()
                        } else {
                            format!(
                                "Nothing in this folder matches \"{}\" — Enter searches \
                                 deeper.",
                                b.query
                            )
                        })
                        .color(cl::ink::MUTED),
                    );
                }
                if let Some(i) = entry_grid(ui, &shown, ns, "folder", thumbs, &mut thumb_budget) {
                    match shown[i].kind {
                        EntryKind::Dir => go = Some(shown[i].path.clone()),
                        _ => picked = Some(shown[i].path.clone()),
                    }
                }
            }
        });
    // Budget spent = thumbs still unread below the fold; the next frame
    // continues filling.
    if thumb_budget == 0 {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(60));
    }
    if let Some(p) = picked {
        b.picked.push(p);
    }
    if let Some(to) = go {
        b.go(to);
    }
}

/// A mogrt entry's preview bytes: cached, or read NOW if the frame's
/// budget allows (the miss stays a badge until a later frame fills it).
fn mogrt_thumb(
    e: &Entry,
    cache: &mut HashMap<PathBuf, Option<Arc<[u8]>>>,
    budget: &mut usize,
) -> Option<Arc<[u8]>> {
    if !matches!(e.kind, EntryKind::Element) {
        return None;
    }
    let is_mogrt = e
        .path
        .extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| x.eq_ignore_ascii_case("mogrt"));
    if !is_mogrt {
        return None;
    }
    if let Some(cached) = cache.get(&e.path) {
        return cached.clone();
    }
    if *budget == 0 {
        return None;
    }
    *budget -= 1;
    let loaded: Option<Arc<[u8]>> = crate::mogrt::read(&e.path)
        .ok()
        .filter(|m| !m.thumb_png.is_empty())
        .map(|m| Arc::from(m.thumb_png.into_boxed_slice()));
    cache.insert(e.path.clone(), loaded.clone());
    loaded
}

/// The thumbnail grid over any entry list; returns the clicked index.
fn entry_grid(
    ui: &mut egui::Ui,
    items: &[&Entry],
    ns: usize,
    tag: &str,
    thumbs: &mut HashMap<PathBuf, Option<Arc<[u8]>>>,
    budget: &mut usize,
) -> Option<usize> {
    let mut clicked = None;
    let avail = ui.available_width();
    let cols = ((avail / 180.0).floor() as usize).clamp(1, 6);
    let card_w = (avail - cl::space::S2 * (cols - 1) as f32) / cols as f32;
    let thumb_h = card_w * 9.0 / 16.0;
    egui::Grid::new(("dpl_browse_grid", ns, tag.to_string()))
        .num_columns(cols)
        .spacing([cl::space::S2, cl::space::S2])
        .show(ui, |ui| {
            for (n, e) in items.iter().enumerate() {
                let png = mogrt_thumb(e, thumbs, budget);
                if card(ui, e, card_w, thumb_h, ns, png) {
                    clicked = Some(n);
                }
                if (n + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
    clicked
}

/// One entry card: folder = named plate; image = the image itself;
/// mogrt = its embedded thumb when read (R2a.2); video/other element =
/// honest badge plates (video posters are made at import, never by
/// probing folders). The whole card is the hit target.
fn card(
    ui: &mut egui::Ui,
    e: &Entry,
    w: f32,
    thumb_h: f32,
    ns: usize,
    mogrt_png: Option<Arc<[u8]>>,
) -> bool {
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
        EntryKind::Element => match &mogrt_png {
            Some(png) => {
                // bytes:// with a stable per-file uri — egui caches the
                // decode; the Arc clone is refcount-cheap.
                let uri = format!("bytes://mogrt-thumb/{}", e.path.to_string_lossy());
                egui::Image::from_bytes(uri, egui::load::Bytes::Shared(Arc::clone(png)))
                    .corner_radius(cl::RADIUS_SM)
                    .paint_at(ui, thumb);
                ("", Tone::Lime)
            }
            None => ("ELEMENT", Tone::Lime),
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-composer-tests")
            .join(format!("browse-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn search_finds_nested_matches_case_insensitively_and_sorted() {
        let root = scratch("deep");
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::create_dir_all(root.join("b").join("c")).unwrap();
        std::fs::create_dir_all(root.join(".junk")).unwrap();
        std::fs::write(root.join("a").join("Lower-Third.mp4"), b"x").unwrap();
        std::fs::write(root.join("b").join("c").join("LOWER_thirds_02.mov"), b"x").unwrap();
        std::fs::write(root.join("b").join("lower-notes.txt"), b"x").unwrap();
        std::fs::write(root.join(".junk").join("lower.mp4"), b"x").unwrap();
        std::fs::write(root.join("song.wav"), b"x").unwrap();

        let hits = search_files(&root, "lower", SEARCH_CAP);
        let names: Vec<&str> = hits.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Lower-Third.mp4", "LOWER_thirds_02.mov"],
            "nested + case-insensitive, name-sorted; txt and dot-dirs never"
        );
        assert!(search_files(&root, "zzz", SEARCH_CAP).is_empty());
        // The empty query matches every media file (the browse-all sweep).
        assert_eq!(search_files(&root, "", SEARCH_CAP).len(), 3);
    }

    #[test]
    fn search_caps_honestly() {
        let root = scratch("cap");
        for i in 0..7 {
            std::fs::write(root.join(format!("clip-{i}.mp4")), b"x").unwrap();
        }
        assert_eq!(search_files(&root, "clip", 3).len(), 3, "cap honored");
        assert_eq!(search_files(&root, "clip", 100).len(), 7);
    }

    #[test]
    fn navigation_clears_the_search_view() {
        let root = scratch("nav");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let mut b = Browser::new(0, root.clone());
        b.query = "x".into();
        b.search = Some(Arc::new(Mutex::new(Search {
            hits: Vec::new(),
            running: false,
        })));
        b.go(root.join("sub"));
        assert!(b.query.is_empty() && b.search.is_none(), "fresh view");
        assert!(b.arm.is_none());
    }
}
