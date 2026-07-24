# R1y — get media in from anywhere: Browse peer windows · Explorer drag-drop · import write-through

## 1. Context

Operator (2026-07-24): "It needs separate windows to appear on click i.e
my browsing local video and image folders or google drive even." The
editor's media bin only shows artifacts ALREADY in the content index —
the operator's actual media lives in ordinary folders. R1y makes getting
media in frictionless two ways: (1) **Browse windows** — real native peer
windows (the chat window's proven viewport machinery) onto local folders,
thumbnail grid, click a file to pull it into the project; (2) **Explorer
drag-drop** — drop media files onto the editor, they import. Both funnel
through ONE import seam: COPY into `content/inbox/` (sources never
mutated), poster + duration for videos, write-through index — so imports
are ordinary artifacts everywhere (bin, gallery, agent seam). Click-to-add
is the interaction (touch law); cross-viewport OS dragging is not
attempted — egui viewports don't support it, and Explorer drag-drop
covers the drag instinct. Google Drive is OUT (R2d — needs an HTTP dep
the operator hasn't approved). Experience gate rows 3–4
(`wiredfront-experience.md` §5).

## 2. Current state (all read in-session, 2026-07-24)

- `crates/composer/src/lib.rs` (R1x working tree, uncommitted — R1y
  builds ON TOP after R1x lands):
  - `pub struct Composer { comp, dir, selected, preview, palette:
    Option<Vec<Record>>, render, tab, tool, time, playing, px_per_sec,
    drag, status }` (lines 38–64).
  - `enum Act { New, Save, Render, Tab, Tool, Add(usize), Select, Remove,
    TogglePlay, Zoom }` (lines 99–110).
  - `fn ui(&mut self, ui)` (line 458): `ensure_palette()` →
    `tick_playback(ui.ctx())` → layout → `match act` applies.
  - `fn media_bin(&self, ui, act)` (line 824): header is two labels
    ("MEDIA" + hint), then thumbnail cards; `Act::Add(i)` on click.
  - `fn ensure_palette` (line 440): index `recent(500, None)` filtered
    `kind == "video"`.
  - `fn add_clip(&mut self, rec: &Record)` (line 249): probes
    `rec.seconds` → `dpl_video::clip::clip_duration(&path)` →
    `FALLBACK_CLIP_SECS`; appends to track 0; selects.
  - `fn resolve(rel) -> PathBuf` (line 219), `fn content_root()`
    (line 213) → `dpl_engine_capture::content_root()`.
  - Imports in scope already: `dpl_index::Record`, `egui_pilot_ai::{...,
    UiPilotExt}`, `std::path::{Path, PathBuf}`, `std::sync::{Arc, Mutex}`.
- `crates/app/src/chat.rs` — the peer-window law this pass copies:
  `egui::ViewportBuilder::default().with_title(..).with_inner_size(..)
  .with_min_inner_size(..).with_resizable(true).with_decorations(true)
  .with_taskbar(true)` then `ctx.pilot_show_viewport_deferred(key, id,
  builder, move |ui, class| { if class ==
  egui::ViewportClass::EmbeddedWindow { /* honest failure */ } ... })`
  with `Arc<Mutex<Shared>>` state (lines 116–198); close via
  `input.viewport().close_requested()`.
- `crates/egui-pilot-ai/src/lib.rs:1067` — `fn
  pilot_show_viewport_deferred<F>(&self, key: protocol::PilotViewportKey,
  id: egui::ViewportId, builder, F)`; `PilotViewportKey::new(impl
  Into<String>)` (runtime uses `format!` keys, lib.rs:873).
- `crates/engine-media/src/lib.rs` — PRIVATE `fn poster_args(src, out)`
  (line 261: `-y -v error -i <src> -frames:v 1 <out>`) + PRIVATE `fn
  run_ffmpeg(args)` (line 274). No public single-poster entry exists.
- `crates/index/src/lib.rs` — `Index::open_default()`, `upsert(&Record)`
  (line 156), `pub fn record_for_capture(path, root, kind, source) ->
  Record` (line 838: fills name/rel/location/times via `record_for`, sets
  `source`). `Record { rel_path, kind, name, seconds: Option<u32>,
  poster_rel_path: Option<String>, location, tags, .. }`.
- `crates/video/src/clip.rs` — `pub fn clip_duration(&Path) ->
  Option<f32>`.
- No `dropped_files`/`hovered_files` handling anywhere in the repo; no
  `reqwest`/`ureq` in any Cargo.toml.

## 3. Target design

One import seam, two feeders, N windows.

- **`dpl_engine_media::write_poster(src, out)`** — public wrapper over
  the existing private args/runner. Import's only new engine surface.
- **`composer::import`** — `import_file(src) -> Result<Record, String>`:
  classify by extension (video: mp4 mov mkv webm avi m4v · image: png
  jpg jpeg webp gif bmp; anything else errs naming the extension) → copy
  to `content/inbox/<stem>.<ext>` (no-clobber: `<stem>-2.<ext>`, `-3`, …)
  → videos get `<stem>-poster.jpg` (failure degrades to no poster, never
  blocks) + `seconds` from `clip_duration` → `record_for_capture(dest,
  root, kind, "import")` + the poster/seconds fields → `upsert`.
- **`composer::browse`** — `Browser { slot, cwd, entries, picked, open }`
  + pure helpers (`media_kind`, `scan`, `roots`, `no_clobber`) + the
  window body `window_ui`. Roots: `USERPROFILE\{Videos, Pictures,
  Downloads, Desktop}` that exist + present drive roots (`A:\`–`Z:\`).
  Grid: folder cards descend, image cards paint `file:///` thumbs, video
  cards are honest filmstrip cards (name + ext badge — no bulk probing);
  click a media card pushes its path into `picked`. Whole-card hits,
  tooltips everywhere, pilot-tracked.
- **Composer wiring** — `browsers: Vec<Arc<Mutex<Browser>>>` + spawn
  counter. `Browse…` chip in the media-bin header opens a window rooted
  at Videos (else USERPROFILE). Each frame: show every open browser as a
  deferred viewport (embedded class = honest failure label, chat's law);
  drain `picked` → `import_file` → video: `add_clip(&rec)`, image: status
  "added <name> to your library"; drain the MAIN viewport's
  `dropped_files` through the same seam; while `hovered_files` is
  non-empty paint a "Drop to import" voltage overlay; any successful
  import sets `self.palette = None` (bin refreshes). Bin stays
  videos-only (image layers on the timeline are a comp-schema question —
  ask-first, untouched).

Scale story: the Nth media source (Drive, a NAS, a phone dump) is one
more `Browser` root or one more feeder into `import_file` — the seam,
index, bin, and timeline never change.

## 4. Changes, in execution order

**Step 1 — edit `crates/engine-media/src/lib.rs`**: directly below
`poster_args` (after line 272) insert:

```rust
/// One poster frame for `src`, written to `out` — the import seam's
/// thumbnail source (same args the capture pipeline uses).
pub fn write_poster(src: &Path, out: &Path) -> Result<(), String> {
    run_ffmpeg(poster_args(src, out))
}
```

**Step 2 — create `crates/composer/src/import.rs`** (full contents):

```rust
//! The ONE import seam: an external media file becomes an ordinary
//! inbox artifact — copied (sources never mutated), postered, timed,
//! and write-through indexed. Browse windows and Explorer drops both
//! land here; later feeders (Drive) reuse it unchanged.

use dpl_index::Record;
use std::path::{Path, PathBuf};

/// "video" / "image" by extension; None = not media we import.
pub fn media_kind(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => Some("video"),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" => Some("image"),
        _ => None,
    }
}

/// The first free `<stem>.<ext>` / `<stem>-N.<ext>` inside `dir`.
pub fn no_clobber(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.{ext}")))
        .find(|p| !p.exists())
        .unwrap()
}

/// Copy `src` into the content inbox and index it. Returns the indexed
/// record. The SOURCE file is never touched (copy, never move).
pub fn import_file(src: &Path) -> Result<Record, String> {
    let kind = media_kind(src)
        .ok_or_else(|| format!("not a media file: {}", src.display()))?;
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("import")
        .to_string();
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let root = dpl_engine_capture::content_root();
    let inbox = root.join("inbox");
    std::fs::create_dir_all(&inbox).map_err(|e| format!("inbox: {e}"))?;
    let dest = no_clobber(&inbox, &stem, &ext);
    std::fs::copy(src, &dest).map_err(|e| format!("copy: {e}"))?;

    let mut rec = dpl_index::record_for_capture(&dest, &root, kind, "import");
    if kind == "video" {
        let poster = dest.with_file_name(format!(
            "{}-poster.jpg",
            dest.file_stem().and_then(|s| s.to_str()).unwrap_or(&stem)
        ));
        // A missing poster degrades the card, never the import.
        if dpl_engine_media::write_poster(&dest, &poster).is_ok() {
            rec.poster_rel_path = poster
                .strip_prefix(&root)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"));
        }
        rec.seconds = dpl_video::clip::clip_duration(&dest)
            .filter(|d| *d > 0.0)
            .map(|d| d.round() as u32);
    }
    dpl_index::Index::open_default()?.upsert(&rec)?;
    Ok(rec)
}
```

**Step 3 — create `crates/composer/src/browse.rs`** (full contents):

```rust
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
    pub open: bool,
}

pub enum EntryKind {
    Dir,
    Image,
    Video,
}

pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub kind: EntryKind,
}

impl Browser {
    pub fn new(slot: usize, cwd: PathBuf) -> Self {
        Self { slot, cwd, entries: None, picked: Vec::new(), open: true }
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
            dirs.push(Entry { path, name, kind: EntryKind::Dir });
        } else if let Some(kind) = media_kind(&path) {
            let kind = if kind == "video" { EntryKind::Video } else { EntryKind::Image };
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
        let up = cl::tip(bar_chip(ui, "‹ up", ns, "up"), "Up one folder");
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
            RichText::new("Nothing here — no folders and no video or image files.")
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
                        let (tip, act) = card(ui, e, card_w, thumb_h, ns);
                        if act {
                            match e.kind {
                                EntryKind::Dir => go = Some(e.path.clone()),
                                _ => picked = Some(e.path.clone()),
                            }
                        }
                        let _ = tip;
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

/// One entry card: folder = tone plate + name; image = the image itself;
/// video = honest filmstrip plate (name + ext badge — posters are made
/// at import, never by probing folders). Whole card is the hit target.
fn card(ui: &mut egui::Ui, e: &Entry, w: f32, thumb_h: f32, ns: usize) -> (egui::Response, bool) {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(w, thumb_h + 18.0), Sense::click());
    let thumb = egui::Rect::from_min_size(rect.min, egui::vec2(w, thumb_h));
    let painter = ui.painter();
    painter.rect_filled(thumb, cl::RADIUS_SM, cl::surface::WELL);
    match e.kind {
        EntryKind::Image => {
            let uri = format!(
                "file:///{}",
                e.path.to_string_lossy().replace('\\', "/")
            );
            egui::Image::new(uri).corner_radius(cl::RADIUS_SM).paint_at(ui, thumb);
        }
        EntryKind::Video => {
            painter.text(
                thumb.center(),
                egui::Align2::CENTER_CENTER,
                "▶",
                egui::FontId::proportional(24.0),
                Tone::Cyan.color(),
            );
            let ext = e
                .path
                .extension()
                .map(|x| x.to_string_lossy().to_ascii_uppercase())
                .unwrap_or_default();
            painter.text(
                egui::pos2(thumb.max.x - 5.0, thumb.max.y - 4.0),
                egui::Align2::RIGHT_BOTTOM,
                ext,
                egui::FontId::monospace(10.0),
                cl::ink::STRONG,
            );
        }
        EntryKind::Dir => {
            painter.text(
                thumb.center(),
                egui::Align2::CENTER_CENTER,
                "📁",
                egui::FontId::proportional(24.0),
                cl::ink::BASE,
            );
        }
    }
    let hovered = resp.hovered();
    painter.rect_stroke(
        thumb,
        cl::RADIUS_SM,
        egui::Stroke::new(
            if hovered { 2.0 } else { 1.0 },
            if hovered { Tone::Magenta.color() } else { cl::surface::HAIRLINE },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(rect.min.x + 2.0, rect.max.y - 2.0),
        egui::Align2::LEFT_BOTTOM,
        &e.name,
        egui::FontId::proportional(11.0),
        if hovered { cl::ink::STRONG } else { cl::ink::BASE },
    );
    let tipped = cl::tip(
        resp,
        match e.kind {
            EntryKind::Dir => "Open this folder",
            EntryKind::Video => "Pull this clip into your project — it lands on the timeline",
            EntryKind::Image => "Pull this image into your library",
        },
    );
    ui.pilot_track(
        &tipped,
        NodeMetadata::new(
            PilotId::new("browse.entry")
                .child(&PilotKey::new(format!("{ns}:{}", e.name))),
            Role::Button,
            &e.name,
        ),
    );
    let clicked = tipped.clicked();
    (tipped, clicked)
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
```

**Step 4 — edit `crates/composer/src/lib.rs`** (six edits):

4a. Module decls + imports. After `mod library;` / `pub use
library::library_ui;` (lines 18–19) add:

```rust
mod browse;
mod import;
```

Extend the `egui_pilot_ai` import (line 27) with `ContextPilotExt` and
`protocol::PilotViewportKey`:

```rust
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    ContextPilotExt, NodeMetadata, PilotId, PilotKey, UiPilotExt,
};
```

(then replace the two existing `egui_pilot_ai::protocol::Role` mentions
accordingly — `Role` is already imported today, so only the additions
change the line.)

4b. `Composer` fields (after `drag: Option<Drag>,` line 62):

```rust
    /// Open Browse windows (peer viewports); slots never reuse.
    browsers: Vec<Arc<Mutex<browse::Browser>>>,
    /// The next Browse window's slot id.
    next_browser: usize,
```

`Default` (after `drag: None,` line 92): `browsers: Vec::new(),` and
`next_browser: 0,`.

4c. `enum Act` gains `Browse,` (after `Zoom(f32),` line 109) and the
`match act` in `ui()` gains (before `None => {}` line 521):

```rust
            Some(Act::Browse) => {
                let start = std::env::var("USERPROFILE")
                    .map(|p| Path::new(&p).join("Videos"))
                    .ok()
                    .filter(|p| p.is_dir())
                    .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
                    .unwrap_or_else(|| PathBuf::from("C:\\"));
                self.browsers.push(Arc::new(Mutex::new(browse::Browser::new(
                    self.next_browser,
                    start,
                ))));
                self.next_browser += 1;
            }
```

4d. `ui()` gains the per-frame window/feeder calls. After
`self.tick_playback(ui.ctx());` (line 460) insert:

```rust
        self.show_browsers(ui.ctx());
        self.drain_media_feeds(ui.ctx());
```

and at the END of `ui()` (after the `match act` block) add the drop
hint overlay:

```rust
        if ui.ctx().input(|i| !i.raw.hovered_files.is_empty()) {
            let r = ui.max_rect();
            ui.painter()
                .rect_filled(r, CornerRadius::same(cl::RADIUS), Tone::Magenta.wash());
            ui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "Drop to import into your library",
                egui::FontId::proportional(22.0),
                cl::ink::STRONG,
            );
        }
```

4e. New `impl Composer` methods (place directly after `ensure_palette`,
line 453):

```rust
    /// Show every open Browse window as a DEFERRED peer viewport (the
    /// chat window's law: real window or an honest failure, never a
    /// silent embed). Closed windows are dropped.
    fn show_browsers(&mut self, ctx: &egui::Context) {
        for b in &self.browsers {
            let (slot, title, open) = {
                let b = b.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                (b.slot, b.title(), b.open)
            };
            if !open {
                continue;
            }
            let builder = egui::ViewportBuilder::default()
                .with_title(title)
                .with_inner_size([560.0, 640.0])
                .with_min_inner_size([380.0, 420.0])
                .with_resizable(true)
                .with_decorations(true)
                .with_taskbar(true);
            let shared = Arc::clone(b);
            ctx.pilot_show_viewport_deferred(
                PilotViewportKey::new(format!("devpulse.browse.{slot}")),
                egui::ViewportId::from_hash_of(("dpl.browse", slot)),
                builder,
                move |ui, class| {
                    let mut b = shared
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if class == egui::ViewportClass::EmbeddedWindow {
                        ui.colored_label(
                            Tone::Coral.color(),
                            "Browse needs native multi-viewport support; \
                             this backend embedded the window.",
                        );
                        b.open = false;
                        return;
                    }
                    if ui.ctx().input(|i| i.viewport().close_requested()) {
                        b.open = false;
                    }
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::new()
                                .fill(cl::surface::WELL)
                                .inner_margin(egui::Margin::same(8)),
                        )
                        .show(ui.ctx(), |ui| browse::window_ui(ui, &mut b));
                },
            );
        }
        self.browsers.retain(|b| {
            b.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .open
        });
    }

    /// ONE funnel for both feeders: Browse-window picks and files
    /// dropped from Explorer onto the main window. Imports copy into
    /// the inbox and write through the index; videos also land on the
    /// timeline. Any import refreshes the bin.
    fn drain_media_feeds(&mut self, ctx: &egui::Context) {
        let mut incoming: Vec<PathBuf> = Vec::new();
        for b in &self.browsers {
            incoming.append(
                &mut b
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .picked,
            );
        }
        incoming.extend(
            ctx.input(|i| i.raw.dropped_files.clone())
                .into_iter()
                .filter_map(|f| f.path),
        );
        if incoming.is_empty() {
            return;
        }
        for src in incoming {
            match import::import_file(&src) {
                Ok(rec) if rec.kind == "video" => self.add_clip(&rec),
                Ok(rec) => {
                    self.status = format!("Added {} to your library.", rec.name);
                }
                Err(e) => self.status = format!("Import failed: {e}"),
            }
        }
        self.palette = None;
    }
```

4f. `media_bin` header (line 825) gains the Browse chip — replace the
first `ui.label(RichText::new("MEDIA")...)` line with:

```rust
        ui.horizontal(|ui| {
            ui.label(RichText::new("MEDIA").small().strong());
            let resp = cl::tip(
                chip(ui, Tone::Cyan, "Browse…", "browse"),
                "Open a window onto one of your folders — click a clip \
                 there to pull it in, or drop files here from Explorer",
            );
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("composer.media").child(&PilotKey::new("browse")),
                    Role::Button,
                    "Browse folders",
                ),
            );
            if resp.clicked() {
                *act = Some(Act::Browse);
            }
        });
```

**Step 5 — tests** (append to `crates/composer/src/lib.rs` `mod tests`,
plus a `mod tests` in `import.rs`):

- `import.rs`: `media_kind` matrix (mp4→video, JPG case-insensitive→
  image, aep/txt/None-ext→None); `no_clobber` picks `<stem>.<ext>` in an
  empty tempdir, then `-2` after creating the first (std `env::temp_dir`
  scratch, same idiom as the index tests).
- `lib.rs`: routing test — a `Record` with `kind: "video"` through the
  `Ok(rec)` match arms appends a layer; `kind: "image"` leaves the track
  untouched and sets a library status (factor the match body into
  `fn apply_import(&mut self, rec: Record)` if needed to test without fs
  — allowed, same file).

## 5. Boundaries

- 🚫 `/content` law: import COPIES the operator's chosen file INTO
  `content/inbox/` — the sanctioned inbox-born path captures already use.
  NEVER move or delete the source; never write outside `inbox/`.
- 🚫 No new crate dependencies. No `rfd`/dialog crate (roots + descend
  navigation instead); no HTTP client (Drive is R2d, operator-gated).
- 🚫 No `#[allow]`; ffmpeg stays a subprocess (`write_poster` wraps the
  existing runner).
- 🚫 No comp.json schema change; image layers on the timeline are
  ⚠️ ask-first (out of scope — images import to the library only).
- Out of scope: cross-viewport OS dragging; favorites persistence
  (Fjall, later); bulk video thumbnails for external folders; Google
  Drive (R2d); `.aep` parsing (post-R2a spike, operator question open).

## 6. Verification

1. `cargo test -p dpl-composer` and `cargo test -p dpl-engine-media`
   exit 0 (old suites + §4.5 tests).
2. `cargo green` exits 0 (fmt + clippy + tests + release build), via the
   pre-push hook on push.
3. Sequential proof shots on the REAL exe:
   - `--route wiredfront/editor --window 1280x800`: media-bin header
     shows `Browse…`.
   - EguiPilotAI drill (devtools exe, per the `eguipilotai` skill): click
     `composer.media/browse`, snapshot the `devpulse.browse.0` viewport —
     the shot must show root chips, folder cards, and image thumbnails
     for a scratch folder staged with one png + one mp4; click the mp4
     card, then snapshot the MAIN viewport: a new bar on the timeline and
     the bin card for the imported clip (poster present when ffmpeg is on
     PATH). Kill any orphaned `devpulselabs` process after.
4. Filesystem proof: the staged source mp4 still exists at its origin
   (copy, never move); `content/inbox/` contains `<stem>.mp4` +
   `<stem>-poster.jpg`; a second import of the same file lands
   `<stem>-2.mp4`.
5. Operator drill (the real bar): drag an mp4 from Explorer onto the
   editor → overlay appears while hovering → on drop the clip is on the
   timeline within one second of the copy finishing.

## 7. Commit plan

Branch `feat/r1y-media-windows` off `main` AFTER the R1x branch lands
(R1y edits the same file). Two commits:

1. `feat(composer): R1y.1 — the import seam (inbox copy · poster ·
   write-through index) + engine-media write_poster` — adds `import.rs`,
   `write_poster`, their tests.
2. `feat(composer): R1y.2 — Browse peer windows + Explorer drag-drop
   feed the timeline` — adds `browse.rs`, the Composer wiring, bin chip,
   drop overlay, routing tests.

Push, `gh pr create`, merge on green; then the session-end law (exe
rebuild + STATE.md).

## 8. Evidence

(filled at execution)
