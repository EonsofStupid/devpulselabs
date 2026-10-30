//! wiredFRONT — capture and media production. ViziWiz is the capture
//! overlay (snip · steps · session · video), launched as its own
//! always-on-top viewport; the hub floor is Overview + Gallery + Sessions.
//! (Live image/media EDITING belongs to the WiredFRONT/Workbench editors,
//! per the three-product architecture — never a hub subroute.)
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
pub const EDITOR: SubId = SubId("editor");
pub const LIBRARY: SubId = SubId("library");
pub const OVERVIEW: SubId = SubId("overview");
pub const VIZIWIZ: SubId = SubId("viziwiz");
pub const GALLERY: SubId = SubId("gallery");
pub const SESSIONS: SubId = SubId("sessions");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "wiredFRONT",
    icon: cl::icon::WIREDFRONT,
    tone: Tone::Magenta,
    subroutes: &[
        // The EDITOR leads — it is the product (the operator's After
        // Effects); the Library is its generated-ingredient shelf; the
        // capture floors follow.
        SubRoute {
            id: EDITOR,
            label: "Editor",
            blurb: "The WiredFRONT editor — stitch, post-process, effects, fonts, elements.",
        },
        SubRoute {
            id: LIBRARY,
            label: "Library",
            blurb: "Your MEDIA shelf — clips, images, elements, renders. Raw material, never app ingredients.",
        },
        SubRoute {
            id: OVERVIEW,
            label: "Overview",
            blurb: "The hub floor: inbox review, every capture vehicle one click away.",
        },
        SubRoute {
            id: VIZIWIZ,
            label: "ViziWiz",
            blurb: "The capture overlay — snip, steps, session, and video recording.",
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
/// armed dump target (two-tap law), the open Keep target list (X3), and
/// the honest status line.
struct GalleryView {
    rows: Vec<dpl_index::Record>,
    filter: Option<&'static str>,
    /// rel_path armed for dump — the second tap fires, anything else
    /// disarms (a destructive action never rides one stray tap).
    armed: Option<String>,
    /// rel_path whose Keep target list is open (inbox artifacts only).
    keep_open: Option<String>,
    /// The keep destinations: every CONTENT episode as (show, episode).
    /// Seeds are templates, never keep targets.
    targets: Vec<(String, String)>,
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
        let targets = dpl_doc::list_content_episodes(&dpl_roots::content_root())
            .into_iter()
            .map(|(show, ep, _)| (show, ep))
            .collect();
        Self {
            rows,
            filter,
            armed: None,
            keep_open: None,
            targets,
            status,
        }
    }
}

/// What one card asked for this frame.
enum CardAct {
    Dump,
    KeepToggle,
    /// Keep into (show, episode).
    KeepTo(String, String),
    /// Play a session card IN PLACE (doctrine §1.2).
    Open,
}

#[derive(Default)]
pub struct WiredfrontHub {
    /// The NAVIGATE seam: tapped placeholder cards publish through here.
    fabric: Option<dpl_fabric::Fabric>,
    /// Capture-domain ears (W8): any Saved/InboxChanged invalidates the
    /// gallery/sessions/library views — refresh chips stop being
    /// load-bearing.
    capture_rx: Option<dpl_fabric::CaptureReceiver>,
    gallery: Option<GalleryView>,
    sessions: Option<SessionsView>,
    /// The LIBRARY media shelf (W6): rows loaded once per visit.
    library: Option<LibraryView>,
    /// The Gallery's in-place session player (doctrine §1.2: "tapping a
    /// session card opens the player right there") + its status line.
    replay: Option<(SessionPlay, String)>,
    /// The R1a Composer surface — the WiredFRONT video editor.
    composer: dpl_composer::Composer,
    /// A transient confirmation for the "Make slide" gesture (W11) — the last
    /// slide minted from the shelf, shown beneath the library.
    slide_note: Option<String>,
}

/// One parsed events.jsonl line — exactly what S1 recorded, no
/// synthesis: bad lines skip, absent files mean no ticks.
struct SessionEvent {
    ms: u64,
    label: String,
}

/// The open session's playback state.
struct SessionPlay {
    name: String,
    /// None = a partial session (no session.mp4) — event track only.
    player: Option<dpl_video::clip::ClipPlayer>,
    poster: Option<String>,
    events: Vec<SessionEvent>,
}

/// The SESSIONS subroute: the session list, or one open player.
struct SessionsView {
    rows: Vec<dpl_index::Record>,
    open: Option<SessionPlay>,
    status: String,
}

impl SessionsView {
    fn load() -> Self {
        let (rows, status) =
            match dpl_index::Index::open_default().and_then(|ix| ix.list(Some("session"), None)) {
                Ok(rows) => {
                    let status = format!("{} sessions", rows.len());
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

/// events.jsonl → the tick list. Each line parses independently;
/// anything malformed is skipped, never invented.
fn parse_events(path: &std::path::Path) -> Vec<SessionEvent> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            let ms = v.get("ms")?.as_u64()?;
            let kind = v.get("type")?.as_str()?.to_string();
            let label = match kind.as_str() {
                "route-changed" => format!(
                    "{}/{}",
                    v.get("hub").and_then(|h| h.as_str()).unwrap_or("?"),
                    v.get("sub").and_then(|s| s.as_str()).unwrap_or("?"),
                ),
                "full-bleed" => {
                    if v.get("on").and_then(|o| o.as_bool()).unwrap_or(false) {
                        "full-bleed on".into()
                    } else {
                        "full-bleed off".into()
                    }
                }
                other => other.to_string(),
            };
            Some(SessionEvent { ms, label })
        })
        .collect()
}

fn mmss(secs: f32) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

const FILTERS: [(&str, Option<&'static str>); 5] = [
    ("all", None),
    ("image", Some("image")),
    ("video", Some("video")),
    ("guide", Some("guide")),
    ("session", Some("session")),
];

impl WiredfrontHub {
    fn gallery_surface(&mut self, ui: &mut egui::Ui) {
        // The in-place replay (doctrine §1.2): a tapped session card
        // plays RIGHT HERE; back returns to the wall.
        if let Some((play, status)) = self.replay.as_mut() {
            if session_player_ui(ui, play, status, "‹ gallery", "gallery.replay") {
                self.replay = None;
            }
            return;
        }
        let view = self.gallery.get_or_insert_with(|| GalleryView::load(None));

        // The header row: filters · rescan · status.
        let mut reload_with: Option<Option<&'static str>> = None;
        let mut rescan = false;
        ui.horizontal(|ui| {
            for (label, kind) in FILTERS {
                let on = view.filter == kind;
                let tip_text = match kind {
                    None => "Show everything in your library".to_string(),
                    Some(k) => format!("Show only your {k}s"),
                };
                let resp = cl::tip(filter_chip(ui, label, on), &tip_text);
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
            let resp = cl::tip(
                filter_chip(ui, "rescan", false),
                "Rebuild the library from what's actually on disk",
            );
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
                .and_then(|ix| ix.reindex(&dpl_roots::content_root()))
            {
                Ok(r) => {
                    // Batching law: a rescan is ONE InboxChanged.
                    Self::signal_via(&self.fabric, dpl_fabric::CaptureEvent::InboxChanged);
                    format!("rescan: indexed {} · soft-cleared {}", r.indexed, r.cleared)
                }
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
        let root = dpl_roots::content_root();
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
                    "nothing here yet — ViziWiz captures land in the inbox and show up here",
                )
                .color(cl::ink::MUTED),
            );
            return;
        }

        // The wall: adaptive card grid.
        let mut act: Option<(String, CardAct)> = None;
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
                            let state = CardState {
                                armed: view.armed.as_deref() == Some(r.rel_path.as_str()),
                                keep_open: view.keep_open.as_deref() == Some(r.rel_path.as_str()),
                                targets: &view.targets,
                                arm: &mut view.armed,
                            };
                            let (card_act, disarmed) =
                                artifact_card(ui, r, &root, card_w, thumb_h, state);
                            if let Some(a) = card_act {
                                act = Some((r.rel_path.clone(), a));
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
        match act {
            Some((rel, CardAct::Dump)) => {
                let status =
                    match dpl_index::Index::open_default().and_then(|ix| ix.dump(&root, &rel)) {
                        Ok(()) => {
                            Self::signal_via(&self.fabric, dpl_fabric::CaptureEvent::InboxChanged);
                            format!("dumped {rel}")
                        }
                        Err(e) => {
                            Self::signal_via(
                                &self.fabric,
                                dpl_fabric::CaptureEvent::Failed { error: e.clone() },
                            );
                            e
                        }
                    };
                let filter = view.filter;
                let mut fresh = GalleryView::load(filter);
                fresh.status = status;
                self.gallery = Some(fresh);
            }
            Some((rel, CardAct::KeepToggle)) => {
                view.armed = None;
                view.keep_open = (view.keep_open.as_deref() != Some(rel.as_str())).then_some(rel);
            }
            Some((rel, CardAct::Open)) => {
                self.open_replay(&rel);
            }
            Some((rel, CardAct::KeepTo(show, ep))) => {
                let status = match dpl_index::Index::open_default()
                    .and_then(|ix| ix.keep(&root, &rel, &show, &ep))
                {
                    Ok(new_rel) => {
                        Self::signal_via(&self.fabric, dpl_fabric::CaptureEvent::InboxChanged);
                        format!("kept → {new_rel}")
                    }
                    Err(e) => {
                        Self::signal_via(
                            &self.fabric,
                            dpl_fabric::CaptureEvent::Failed { error: e.clone() },
                        );
                        e
                    }
                };
                let filter = view.filter;
                let mut fresh = GalleryView::load(filter);
                fresh.status = status;
                self.gallery = Some(fresh);
            }
            None => {}
        }
    }
}

/// The view state one card renders against (keeps the card's signature
/// inside the argument budget — no `#[allow]`, the workspace law).
struct CardState<'a> {
    armed: bool,
    keep_open: bool,
    targets: &'a [(String, String)],
    arm: &'a mut Option<String>,
}

/// One artifact card: thumb (honest fallback), name, kind+tags line, the
/// two-tap Dump, and (inbox rows) the Keep flow — a toggle opening the
/// content-episode target list in place. Returns (action, disarm).
fn artifact_card(
    ui: &mut egui::Ui,
    r: &dpl_index::Record,
    root: &std::path::Path,
    w: f32,
    thumb_h: f32,
    state: CardState,
) -> (Option<CardAct>, bool) {
    let CardState {
        armed,
        keep_open,
        targets,
        arm,
    } = state;
    let mut act: Option<CardAct> = None;
    let mut disarm = false;
    ui.vertical(|ui| {
        ui.set_width(w);
        // The thumbnail well. A SESSION thumb is tappable — playback in
        // place (doctrine §1.2); other kinds stay inert here.
        let playable = r.kind == "session";
        let (rect, thumb_resp) = ui.allocate_exact_size(
            egui::vec2(w, thumb_h),
            if playable {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        if playable {
            ui.pilot_track(
                &thumb_resp,
                NodeMetadata::new(
                    PilotId::new("gallery.play").child(&PilotKey::new(&r.rel_path)),
                    egui_pilot_ai::protocol::Role::Button,
                    "Play the session in place",
                ),
            );
            if thumb_resp.clicked() {
                act = Some(CardAct::Open);
            }
            cl::tip(thumb_resp.clone(), "Play this session right here");
        }
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
                let resp = cl::tip(
                    ui.label(RichText::new(label).small().color(if armed {
                        cl::Tone::Coral.color()
                    } else {
                        cl::ink::SUBTLE
                    }))
                    .interact(Sense::click()),
                    if armed {
                        "Tap again to really delete it"
                    } else {
                        "Delete this — it takes two taps, so a stray tap never deletes"
                    },
                );
                if resp.clicked() {
                    if armed {
                        act = Some(CardAct::Dump);
                    } else {
                        *arm = Some(r.rel_path.clone());
                    }
                } else if armed && ui.input(|i| i.pointer.any_click()) {
                    // Any click elsewhere disarms — one stray tap never
                    // deletes.
                    disarm = true;
                }
                // Keep (X3) — inbox artifacts only; the jail binds the
                // source anyway, this just keeps the surface honest.
                if r.location == "inbox" {
                    let resp = cl::tip(
                        ui.label(RichText::new("Keep").small().color(if keep_open {
                            cl::Tone::Magenta.color()
                        } else {
                            cl::ink::SUBTLE
                        }))
                        .interact(Sense::click()),
                        "File this into one of your shows",
                    );
                    // Per-CARD id — a shared "gallery.keep" duplicates
                    // (and panics) the moment two inbox cards render.
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("gallery.keep").child(&PilotKey::new(&r.rel_path)),
                            egui_pilot_ai::protocol::Role::Button,
                            "Keep into a show",
                        )
                        .value(if keep_open { "open" } else { "closed" }),
                    );
                    if resp.clicked() {
                        act = Some(CardAct::KeepToggle);
                    }
                }
            });
        });
        let mut meta = format!("{} · {}", r.kind, r.captured_at);
        if !r.tags.is_empty() {
            meta = format!("{meta} · {}", r.tags.join(", "));
        }
        ui.label(RichText::new(meta).small().color(cl::ink::MUTED));

        // The Keep target list, in place: every CONTENT episode. Seeds are
        // templates, never targets; an empty content tree says so.
        if keep_open {
            if targets.is_empty() {
                ui.label(
                    RichText::new("no shows yet — use \"copy to my shows\" in Workbench")
                        .small()
                        .color(cl::ink::MUTED),
                );
            }
            ui.horizontal_wrapped(|ui| {
                for (show, ep) in targets {
                    let label = format!("{show}/{ep}");
                    let resp = cl::tip(
                        filter_chip(ui, &label, false),
                        &format!("Move it into {show} › {ep}"),
                    );
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("gallery.keep.target").child(&PilotKey::new(&label)),
                            egui_pilot_ai::protocol::Role::Button,
                            &label,
                        ),
                    );
                    if resp.clicked() {
                        act = Some(CardAct::KeepTo(show.clone(), ep.clone()));
                    }
                }
            });
        }
    });
    (act, disarm)
}

/// The LIBRARY shelf's state (W6): every media row (clips, images —
/// renders are ordinary kind-"video" rows), loaded once per visit.
struct LibraryView {
    rows: Vec<dpl_index::Record>,
    status: String,
}

impl LibraryView {
    fn load() -> Self {
        let (rows, status) =
            match dpl_index::Index::open_default().and_then(|ix| ix.recent(500, None)) {
                Ok(rows) => {
                    let rows: Vec<dpl_index::Record> = rows
                        .into_iter()
                        .filter(|r| r.kind == "video" || r.kind == "image")
                        .collect();
                    let status = format!("{} media file(s)", rows.len());
                    (rows, status)
                }
                Err(e) => (Vec::new(), e),
            };
        Self { rows, status }
    }
}

impl WiredfrontHub {
    /// The WiredFRONT LIBRARY — the real MEDIA shelf (W6; two-libraries
    /// law: raw media lives HERE, ColdLight ingredients in Workbench).
    /// A thumbnailed wall of every clip and image; DOUBLE-CLICK sends a
    /// row to the Editor (video → timeline, image → FOOTAGE, honestly)
    /// and navigates there — the frictionless law. The Gallery remains
    /// the capture wall with Dump/Keep.
    fn library_surface(&mut self, ui: &mut egui::Ui) {
        // The shelf sits on its own recessed plate — the mood backdrop
        // never fights the reading surface.
        let full = ui.max_rect();
        ui.painter().rect_filled(
            full,
            egui::CornerRadius::same(cl::RADIUS),
            cl::surface::WELL,
        );
        let mut inner = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(full.shrink(cl::space::S3))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut inner;
        let view = self.library.get_or_insert_with(LibraryView::load);
        let mut refresh = false;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("The media shelf")
                    .heading()
                    .color(cl::ink::STRONG),
            );
            let resp = cl::tip(
                filter_chip(ui, "refresh", false),
                "Reload the shelf from your library",
            );
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("library").child(&PilotKey::new("refresh")),
                    egui_pilot_ai::protocol::Role::Button,
                    "Refresh the shelf",
                ),
            );
            if resp.clicked() {
                refresh = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(&view.status).small().color(cl::ink::SUBTLE));
            });
        });
        ui.label(
            RichText::new(
                "Raw material for the editor — DOUBLE-CLICK any clip or image to send it \
                 to the Editor. App ingredients live in the Workbench Library.",
            )
            .small()
            .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S2);

        let root = dpl_roots::content_root();
        let mut action: Option<LibAction> = None;
        if view.rows.is_empty() {
            ui.label(
                RichText::new(
                    "nothing here yet — capture with ViziWiz, drop files on the Editor, \
                     or Browse… a folder; everything lands on this shelf",
                )
                .color(cl::ink::MUTED),
            );
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let avail = ui.available_width();
                    let cols = ((avail / 240.0).floor() as usize).clamp(1, 7);
                    let card_w = (avail - cl::space::S3 * (cols - 1) as f32) / cols as f32;
                    let thumb_h = card_w * 9.0 / 16.0;
                    egui::Grid::new("dpl_library_grid")
                        .num_columns(cols)
                        .spacing([cl::space::S3, cl::space::S3])
                        .show(ui, |ui| {
                            for (n, r) in view.rows.iter().enumerate() {
                                if let Some(a) = library_card(ui, r, &root, card_w, thumb_h) {
                                    action = Some(a);
                                }
                                if (n + 1) % cols == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
        }
        ui.add_space(cl::space::S2);
        let element_count = count_files(&root.join("elements"));
        ui.label(
            RichText::new(if element_count == 0 {
                "ELEMENTS: none yet — open a location in the Editor's Browse…, then Ingest pack"
                    .to_string()
            } else {
                format!("ELEMENTS: {element_count} file(s) copied into your library")
            })
            .small()
            .color(cl::ink::SUBTLE),
        );

        if refresh {
            self.library = Some(LibraryView::load());
            return;
        }
        if let Some(note) = &self.slide_note {
            ui.add_space(cl::space::S2);
            ui.label(RichText::new(note).small().color(cl::Tone::Lime.color()));
        }

        match action {
            // The frictionless law: the double-click IS the import — the
            // editor takes it and the app goes there.
            Some(LibAction::Send(rec)) => {
                self.composer.add_from_library(&rec);
                if let Some(fabric) = &self.fabric {
                    fabric.publish_shell(dpl_fabric::ShellEvent::NavigateRequest {
                        route: Route {
                            hub: HUB,
                            sub: EDITOR,
                        },
                    });
                }
            }
            // W11: mint a standalone media slide from this artifact — it
            // becomes available to any episode/stream (W10 associates it).
            Some(LibAction::MakeSlide(rec)) => {
                self.slide_note = Some(match self.composer.make_slide_from_library(&rec) {
                    Ok(_) => format!(
                        "Made a slide from “{}” — pick it when you build an episode.",
                        rec.name
                    ),
                    Err(e) => format!("Couldn't make a slide: {e}"),
                });
            }
            None => {}
        }
    }
}

/// What a shelf card asks for (W11): SEND feeds the timeline editor;
/// MAKE-SLIDE mints a standalone media slide from the artifact.
enum LibAction {
    Send(dpl_index::Record),
    MakeSlide(dpl_index::Record),
}

/// One shelf card: thumb (honest fallback) + name + facts + a "Make slide"
/// chip. Returns a [`LibAction`]: SEND on double-click (to the editor),
/// MAKE-SLIDE on the chip (a standalone media slide).
fn library_card(
    ui: &mut egui::Ui,
    r: &dpl_index::Record,
    root: &std::path::Path,
    w: f32,
    thumb_h: f32,
) -> Option<LibAction> {
    let mut send = None;
    ui.vertical(|ui| {
        ui.set_width(w);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, thumb_h), Sense::click());
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
        // Hover names the gesture — the double-click must never be a
        // secret handshake.
        ui.painter().rect_stroke(
            rect,
            cl::RADIUS_SM,
            egui::Stroke::new(
                if resp.hovered() { 2.0 } else { 1.0 },
                if resp.hovered() {
                    cl::Tone::Magenta.color()
                } else {
                    cl::surface::HAIRLINE
                },
            ),
            egui::StrokeKind::Inside,
        );
        let resp = cl::tip(resp, "Double-click to send this to the Editor");
        ui.pilot_track(
            &resp,
            NodeMetadata::new(
                PilotId::new("library.card").child(&PilotKey::new(&r.rel_path)),
                egui_pilot_ai::protocol::Role::Button,
                "Send to the Editor (double-click)",
            ),
        );
        if resp.double_clicked() {
            send = Some(LibAction::Send(r.clone()));
        }
        ui.label(RichText::new(&r.name).strong().color(cl::ink::STRONG));
        let facts = match r.seconds {
            Some(s) if s > 0 => format!("{} · {}:{:02}", r.kind, s / 60, s % 60),
            _ => format!("{} · {}", r.kind, r.captured_at),
        };
        ui.label(RichText::new(facts).small().color(cl::ink::MUTED));
        // A linked file says where it lives, and when its drive is away.
        if let Some(location) = dpl_roots::location_of(&r.rel_path) {
            let here = dpl_roots::locate_asset(&r.rel_path).is_some();
            let line = if here {
                format!("on '{}'", location.label)
            } else {
                format!("on '{}' · offline — plug it in", location.label)
            };
            ui.label(RichText::new(line).small().color(if here {
                cl::ink::MUTED
            } else {
                cl::Tone::Amber.color()
            }));
        }
        // W11: images and videos can become a slide right here (a still, a
        // clip, or a session replay — replays are videos). Other kinds
        // (audio, packs) aren't slide media, so no chip.
        if matches!(r.kind.as_str(), "image" | "video") {
            let b = ui.add(egui::Button::new(RichText::new("Make slide").small()));
            let b = cl::tip(b, "Turn this into a slide for an episode or stream");
            if b.clicked() {
                send = Some(LibAction::MakeSlide(r.clone()));
            }
        }
    });
    send
}

/// Count files under a directory tree (the elements census). Absent → 0.
fn count_files(dir: &std::path::Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                count_files(&p)
            } else {
                1
            }
        })
        .sum()
}

/// The thumbnail source, or None for the honest kind-label fallback:
/// image → itself; video → its poster; guide → its first frame.
fn thumb_uri(r: &dpl_index::Record, root: &std::path::Path) -> Option<String> {
    // A linked file (ST5): its cached poster, online or offline, so the
    // shelf never reaches into the drive while it paints.
    if let Some(poster) = dpl_roots::location_poster(&r.rel_path).filter(|p| p.is_file()) {
        return Some(format!(
            "file:///{}",
            poster.to_string_lossy().replace('\\', "/")
        ));
    }
    let rel = match r.kind.as_str() {
        "image" => Some(r.rel_path.clone()),
        "video" => r.poster_rel_path.clone(),
        "guide" => {
            let first = root.join(&r.rel_path).join("step-001.webp");
            first
                .is_file()
                .then_some(format!("{}/step-001.webp", r.rel_path))
        }
        "session" => {
            let poster = root.join(&r.rel_path).join("session.jpg");
            poster
                .is_file()
                .then_some(format!("{}/session.jpg", r.rel_path))
        }
        _ => None,
    }?;
    let abs = root.join(rel);
    // file:/// with THREE slashes — two reads as a UNC host (the media
    // block's hard-won law, crates/slides/blocks/media.rs).
    abs.is_file()
        .then(|| format!("file:///{}", abs.to_string_lossy().replace('\\', "/")))
}

/// A BRUTAL action chip (doctrine: actionable = neubrutal — thick tone
/// edge, HARD offset shadow, flat fill; the shadow is a shape, not a
/// glow). Hover lifts the chip onto its shadow; active fills with the
/// tone wash.
fn filter_chip(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.into(),
        egui::FontId::proportional(13.0),
        cl::ink::BASE,
    );
    let (slot, resp) =
        ui.allocate_exact_size(egui::vec2(galley.size().x + 26.0, 31.0), Sense::click());
    // The chip sits inside its slot leaving room for the hard shadow;
    // hover translates it ONTO the shadow (the tactile press).
    let lift = if resp.hovered() && !resp.is_pointer_button_down_on() {
        egui::Vec2::ZERO
    } else {
        egui::vec2(1.0, 1.0)
    };
    let chip = egui::Rect::from_min_size(slot.min + lift, slot.size() - egui::vec2(3.0, 3.0));
    let shadow = chip.translate(egui::vec2(3.0, 3.0) - lift);
    let painter = ui.painter();
    painter.rect_filled(
        shadow,
        egui::CornerRadius::same(cl::RADIUS_SM),
        egui::Color32::from_black_alpha(230),
    );
    painter.rect_filled(
        chip,
        egui::CornerRadius::same(cl::RADIUS_SM),
        if on {
            cl::Tone::Magenta.wash()
        } else {
            cl::surface::OVERLAY
        },
    );
    painter.rect_stroke(
        chip,
        egui::CornerRadius::same(cl::RADIUS_SM),
        egui::Stroke::new(
            2.0,
            if on {
                cl::Tone::Magenta.color()
            } else {
                cl::surface::HAIRLINE
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        chip.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        if on { cl::ink::STRONG } else { cl::ink::BASE },
    );
    resp
}

/// Build one session's playback state + its honest status line — the ONE
/// path every player entry takes (sessions list tap, gallery in-place
/// tap, both harness seams).
fn build_session_play(root: &std::path::Path, rel: &str, name: String) -> (SessionPlay, String) {
    let dir = root.join(rel);
    let mp4 = dir.join("session.mp4");
    let player = mp4
        .is_file()
        .then(|| dpl_video::clip::ClipPlayer::open(mp4));
    let poster = dir.is_dir().then(|| dir.join("session.jpg")).and_then(|p| {
        p.is_file()
            .then(|| format!("file:///{}", p.to_string_lossy().replace('\\', "/")))
    });
    let events = parse_events(&dir.join("events.jsonl"));
    let status = if player.is_none() {
        "partial session — event track only".into()
    } else {
        format!("{} events", events.len())
    };
    (
        SessionPlay {
            name,
            player,
            poster,
            events,
        },
        status,
    )
}

impl WiredfrontHub {
    pub fn wire_fabric(&mut self, fabric: &dpl_fabric::Fabric) {
        self.capture_rx = Some(fabric.subscribe_capture());
        self.composer.wire_fabric(fabric);
        self.fabric = Some(fabric.clone());
    }

    /// Share the Ring-0 estate into the editor (last-comp dual-write).
    pub fn wire_estate(&mut self, estate: std::sync::Arc<dpl_estate::Estate>) {
        self.composer.wire_estate(estate);
    }

    /// Publish a capture signal when wired (drills without a fabric
    /// stay silent, never panic). Free-standing over the FIELD so a
    /// mutably-borrowed view elsewhere on self never conflicts.
    fn signal_via(fabric: &Option<dpl_fabric::Fabric>, ev: dpl_fabric::CaptureEvent) {
        if let Some(f) = fabric {
            f.publish_capture(ev);
        }
    }

    /// Drain capture signals (W8): artifact changes from ANY surface
    /// invalidate this hub's cached views (lazy reload on next paint).
    fn drain_signals(&mut self) {
        let mut dirty = false;
        if let Some(rx) = self.capture_rx.as_mut() {
            while let Ok(stamped) = rx.try_recv() {
                dirty |= matches!(
                    stamped.ev,
                    dpl_fabric::CaptureEvent::Saved { .. } | dpl_fabric::CaptureEvent::InboxChanged
                );
            }
        }
        if dirty {
            // Keep any OPEN player/replay alive; only the lists reload.
            if self.replay.is_none() {
                self.gallery = None;
            }
            if self.sessions.as_ref().is_none_or(|s| s.open.is_none()) {
                self.sessions = None;
            }
            self.library = None;
        }
    }

    /// Open one session on the SESSIONS surface — the same path a list
    /// card tap takes (also the `--open-session` harness seam).
    pub fn open_session(&mut self, rel: &str) {
        let view = self.sessions.get_or_insert_with(SessionsView::load);
        let name = view
            .rows
            .iter()
            .find(|r| r.rel_path == rel)
            .map(|r| r.name.clone())
            .unwrap_or_else(|| rel.to_string());
        let (play, status) = build_session_play(&dpl_roots::content_root(), rel, name);
        view.status = status;
        view.open = Some(play);
    }

    /// The `--import-drill <dir>` harness seam (W2): the editor imports
    /// every media file in the staged folder — the Explorer-drop path.
    pub fn import_drill(&mut self, dir: &std::path::Path) {
        self.composer.import_dir(dir);
    }

    /// The `--save-comp <name>` harness seam (R1z): save the booted comp
    /// through the Save chip's exact path — the reopen-last drill.
    pub fn save_comp(&mut self, name: &str) {
        self.composer.save_comp(name);
    }

    /// The `--lottie-import <file>` harness seam (L2): import a Lottie and
    /// add it to the timeline — the native-Lottie drill.
    pub fn import_lottie(&mut self, path: &std::path::Path) {
        self.composer.import_lottie(path);
    }

    /// The `--fx-drill <clip-rel> <fx-rel>` harness seam (L3): one base
    /// cut + one transformed/tinted FX overlay, rendered synchronously —
    /// the headless alpha-overlay proof.
    pub fn preview_drill(&mut self, rel: &str) {
        self.composer.preview_drill(rel);
    }

    pub fn fx_drill(&mut self, clip_rel: &str, fx_rel: &str) {
        self.composer.fx_drill(clip_rel, fx_rel);
    }

    /// The `--l4-drill` harness seam: graded base + screen leak + matted
    /// overlay from conventional scratch-root files, rendered
    /// synchronously — the headless blend/matte/LUT proof.
    /// The motion-blur drill — see `Composer::mblur_drill`.
    pub fn mblur_drill(&mut self) {
        self.composer.mblur_drill();
    }

    pub fn l4_drill(&mut self) {
        self.composer.l4_drill();
    }

    /// The `--make-slide <asset-rel>` harness seam (W11): mint a media slide
    /// from a content-relative artifact (image/video → one slide; `.pdf` →
    /// page slides) through the shelf's exact "Make slide" path. Prints the
    /// outcome for the headless drill.
    pub fn make_slide(&mut self, rel: &str) {
        match self.composer.make_slide_from_rel(rel) {
            Ok(dirs) => {
                let list = dirs
                    .iter()
                    .map(|d| d.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.slide_note = Some(format!("Made {} slide(s): {list}", dirs.len()));
                println!("make-slide: {} slide(s) minted: {list}", dirs.len());
            }
            Err(e) => {
                self.slide_note = Some(format!("Make slide failed: {e}"));
                eprintln!("make-slide failed: {e}");
            }
        }
    }

    /// The `--run-workflow <id>` harness seam (WF1): run a registered
    /// workflow through the Workflows tab's exact Run path. Prints the
    /// verdict line for the headless drill (an unknown id goes to stderr
    /// naming the registered set; nothing is minted).
    pub fn run_workflow(&mut self, id: &str) {
        match self.composer.run_workflow(id) {
            Ok(verdict) => println!("{verdict}"),
            Err(e) => eprintln!("run-workflow failed: {e}"),
        }
    }

    /// The `--dry-run-workflow <id>` harness seam: preview a registered
    /// workflow through the Workflows tab's exact Dry-run path. Prints the
    /// would-do verdict line; nothing touches disk, nothing spends.
    pub fn dry_run_workflow(&mut self, id: &str) {
        match self.composer.dry_run_workflow(id) {
            Ok(verdict) => println!("{verdict}"),
            Err(e) => eprintln!("dry-run-workflow failed: {e}"),
        }
    }

    /// Open one session IN PLACE on the Gallery (doctrine §1.2) — the
    /// same path a session thumb tap takes (also `--open-replay`).
    pub fn open_replay(&mut self, rel: &str) {
        let name = dpl_index::Index::open_default()
            .and_then(|ix| ix.list(Some("session"), None))
            .ok()
            .and_then(|rows| rows.into_iter().find(|r| r.rel_path == rel).map(|r| r.name))
            .unwrap_or_else(|| rel.to_string());
        let (play, status) = build_session_play(&dpl_roots::content_root(), rel, name);
        self.replay = Some((play, status));
    }

    fn sessions_surface(&mut self, ui: &mut egui::Ui) {
        let view = self.sessions.get_or_insert_with(SessionsView::load);

        if let Some(play) = view.open.as_mut() {
            if session_player_ui(ui, play, &view.status, "‹ sessions", "sessions") {
                view.open = None;
            }
            return;
        }

        // ── the list ────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Sessions — the app on video, with its event track")
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
                RichText::new("no sessions yet — record one from ViziWiz's Session tab")
                    .color(cl::ink::MUTED),
            );
            return;
        }
        let root = dpl_roots::content_root();
        let mut open: Option<String> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for r in &view.rows {
                    let resp = cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width().min(720.0));
                        ui.horizontal(|ui| {
                            let (thumb, _) =
                                ui.allocate_exact_size(egui::vec2(128.0, 72.0), Sense::hover());
                            ui.painter()
                                .rect_filled(thumb, cl::RADIUS_SM, cl::surface::WELL);
                            if let Some(uri) = thumb_uri(r, &root) {
                                egui::Image::new(uri)
                                    .corner_radius(cl::RADIUS_SM)
                                    .paint_at(ui, thumb);
                            }
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&r.name).strong().color(cl::ink::STRONG));
                                ui.label(
                                    RichText::new(format!("{} · {}", r.captured_at, r.location))
                                        .small()
                                        .color(cl::ink::MUTED),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("play ›")
                                            .small()
                                            .color(cl::Tone::Magenta.color()),
                                    );
                                },
                            );
                        });
                    });
                    let click = cl::tip(
                        resp.response.interact(Sense::click()),
                        "Open and play this session",
                    );
                    ui.pilot_track(
                        &click,
                        NodeMetadata::new(
                            PilotId::new("sessions.open").child(&PilotKey::new(&r.rel_path)),
                            egui_pilot_ai::protocol::Role::Button,
                            &r.name,
                        ),
                    );
                    if click.clicked() {
                        open = Some(r.rel_path.clone());
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        if let Some(rel) = open {
            self.open_session(&rel);
        }
    }
}

/// The session player — back row · 16:9 stage · controls · the event-tick
/// seek rail (or the partial-session text track). ONE body for the
/// SESSIONS surface and the Gallery's in-place replay (doctrine §1.2);
/// `ns` keys the pilot ids per surface. Returns true when back is tapped.
fn session_player_ui(
    ui: &mut egui::Ui,
    play: &mut SessionPlay,
    status: &str,
    back_label: &str,
    ns: &'static str,
) -> bool {
    let back = ui
        .horizontal(|ui| {
            let b = cl::tip(
                ui.label(RichText::new(back_label).color(cl::Tone::Magenta.color()))
                    .interact(Sense::click()),
                "Back to the list",
            );
            ui.pilot_track(
                &b,
                NodeMetadata::new(
                    PilotId::new(ns).child(&PilotKey::new("back")),
                    egui_pilot_ai::protocol::Role::Button,
                    "Back to the list",
                ),
            );
            ui.label(RichText::new(&play.name).strong().color(cl::ink::STRONG));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(status).small().color(cl::ink::SUBTLE));
            });
            b
        })
        .inner;
    if back.clicked() {
        return true;
    }
    ui.add_space(cl::space::S2);

    // The 16:9 stage: decoded frames, else the poster, else an honest
    // label.
    let stage_w = (ui.available_width() - cl::space::S4).min(960.0);
    let stage_h = stage_w * 9.0 / 16.0;
    let (stage, _) = ui.allocate_exact_size(egui::vec2(stage_w, stage_h), Sense::hover());
    ui.painter()
        .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
    let mut painted = false;
    if let Some(player) = &play.player {
        if let Some(tex) = player.texture(ui.ctx()) {
            ui.painter().image(
                tex,
                stage,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            painted = true;
        }
        if player.playing() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
    if !painted {
        match &play.poster {
            Some(uri) => {
                egui::Image::new(uri.as_str())
                    .corner_radius(cl::RADIUS)
                    .paint_at(ui, stage);
            }
            None => {
                ui.painter().text(
                    stage.center(),
                    egui::Align2::CENTER_CENTER,
                    if play.player.is_none() {
                        "partial session — event track only"
                    } else {
                        "starting…"
                    },
                    egui::FontId::monospace(13.0),
                    cl::ink::SUBTLE,
                );
            }
        }
    }
    ui.painter().rect_stroke(
        stage,
        cl::RADIUS,
        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
        egui::StrokeKind::Inside,
    );
    ui.add_space(cl::space::S2);

    // Controls + the RAIL (ticks = the recorded events).
    let mut seek_to: Option<f32> = None;
    if let Some(player) = play.player.as_mut() {
        let pos = player.position();
        let span = player
            .duration
            .or_else(|| play.events.last().map(|e| e.ms as f32 / 1000.0))
            .unwrap_or(1.0)
            .max(0.1);
        ui.horizontal(|ui| {
            let label = if player.ended() {
                "Replay"
            } else if player.playing() {
                "Pause"
            } else {
                "Play"
            };
            let resp = cl::tip(
                filter_chip(ui, label, player.playing()),
                "Play or pause the recording",
            );
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new(ns).child(&PilotKey::new("play")),
                    egui_pilot_ai::protocol::Role::Button,
                    "Play/pause",
                )
                .value(label),
            );
            if resp.clicked() {
                player.toggle();
            }
            ui.label(
                RichText::new(format!("{} / {}", mmss(pos), mmss(span)))
                    .monospace()
                    .small()
                    .color(cl::ink::BASE),
            );
        });
        ui.add_space(cl::space::S1);

        // The rail: tap anywhere to seek; a tap near a tick snaps to
        // its event; hover names the nearest event.
        let (rail, resp) = ui.allocate_exact_size(egui::vec2(stage_w, 22.0), Sense::click());
        let resp = cl::tip(
            resp,
            "Tap anywhere to jump — the orange ticks are moments something happened",
        );
        ui.pilot_track(
            &resp,
            NodeMetadata::new(
                PilotId::new(ns).child(&PilotKey::new("rail")),
                egui_pilot_ai::protocol::Role::Slider,
                "Seek rail",
            )
            .value(format!("{pos:.1}/{span:.1}")),
        );
        let line_y = rail.center().y;
        let painter = ui.painter().clone();
        painter.line_segment(
            [
                egui::pos2(rail.left(), line_y),
                egui::pos2(rail.right(), line_y),
            ],
            egui::Stroke::new(3.0, cl::surface::OVERLAY),
        );
        let frac = (pos / span).clamp(0.0, 1.0);
        painter.line_segment(
            [
                egui::pos2(rail.left(), line_y),
                egui::pos2(rail.left() + rail.width() * frac, line_y),
            ],
            egui::Stroke::new(3.0, cl::Tone::Magenta.color()),
        );
        let tick_x =
            |ms: u64| rail.left() + rail.width() * ((ms as f32 / 1000.0) / span).clamp(0.0, 1.0);
        for ev in &play.events {
            let x = tick_x(ev.ms);
            painter.line_segment(
                [
                    egui::pos2(x, rail.top() + 3.0),
                    egui::pos2(x, rail.bottom() - 3.0),
                ],
                egui::Stroke::new(2.0, cl::Tone::Coral.color()),
            );
        }
        if let Some(p) = resp.hover_pos() {
            if let Some(near) = play
                .events
                .iter()
                .min_by_key(|e| (tick_x(e.ms) - p.x).abs() as u32)
                .filter(|e| (tick_x(e.ms) - p.x).abs() < 8.0)
            {
                resp.clone().on_hover_text(format!(
                    "{} · {}",
                    mmss(near.ms as f32 / 1000.0),
                    near.label
                ));
            }
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                let snapped = play
                    .events
                    .iter()
                    .find(|e| (tick_x(e.ms) - p.x).abs() < 8.0)
                    .map(|e| e.ms as f32 / 1000.0);
                seek_to = Some(snapped.unwrap_or_else(|| {
                    ((p.x - rail.left()) / rail.width()).clamp(0.0, 1.0) * span
                }));
            }
        }
    } else {
        // Partial session: the event track stands alone, readable —
        // nothing to seek.
        for ev in &play.events {
            ui.label(
                RichText::new(format!("{}  {}", mmss(ev.ms as f32 / 1000.0), ev.label))
                    .monospace()
                    .color(cl::ink::BASE),
            );
        }
    }
    if let (Some(t), Some(player)) = (seek_to, play.player.as_mut()) {
        player.seek(t);
    }
    false
}

impl Hub for WiredfrontHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        self.drain_signals();
        if route.sub == GALLERY {
            self.gallery_surface(ui);
            return;
        }
        if route.sub == SESSIONS {
            self.sessions_surface(ui);
            return;
        }
        if route.sub == EDITOR {
            self.composer.ui(ui);
            return;
        }
        if route.sub == LIBRARY {
            self.library_surface(ui);
            return;
        }
        if let Some(go) = dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "Capture, stitch, generate, edit. ADR-010 is this hub's law: shot slots, \
             ffmpeg subprocess, ComfyUI as the first pluggable backend.",
        ) {
            if let Some(fabric) = &self.fabric {
                fabric.publish_shell(dpl_fabric::ShellEvent::NavigateRequest { route: go });
            }
        }
    }
}
