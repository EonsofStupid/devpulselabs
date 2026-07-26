//! The shell — the persistent frame every hub lives inside.
//!
//! The operator's spec, implemented literally: five chrome plates — topbar,
//! left rail, right rail, footer, and the overlay seams that join them —
//! FRAME the content, which sits in a recessed well. Every plate wears the
//! sprite vocabulary: solid fill + top sheen (gradient tint), a crisp 2px
//! bevel at every chrome/well boundary, black diffusion scattered into the
//! well on all four sides. The topbar seam answers the active hub's tone.
//!
//! Layout v2 (docs/shell.md): five named ZONES — TOP, STATUS, RAIL, PANEL,
//! CONTENT — every dimension resolved through the generated fluid `shell`
//! token group at the live viewport; nothing hardcoded (workspace law). The
//! rail is an ACCORDION (Program L, doctrine §5.2): tapping a house unfolds
//! its subroutes downward inside the rail — the houses below push down —
//! and folds whatever was open; one open at a time, folded is icon-mode
//! compact. The status bar is a data-driven cell registry ([`status`]).
//! Every navigation publishes `ShellEvent::RouteChanged` on the fabric.

pub mod status;

use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::{AppWindowAction, AppWindowKey, Fabric, ShellEvent};
use dpl_hub::{Hub, HubId, Route, SubId};
use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};

/// The five named zones (docs/shell.md), captured from the REAL panel rects
/// every frame — the layout is observable, therefore testable.
#[derive(Debug, Clone, Copy)]
pub struct Zones {
    /// The space the shell was handed this frame (the whole viewport under
    /// eframe — its root ui carries no margin; test harnesses may inset it).
    pub root: Rect,
    pub top: Rect,
    pub status: Rect,
    pub rail: Rect,
    pub panel: Rect,
    pub content: Rect,
}

impl Default for Zones {
    fn default() -> Self {
        Self {
            root: Rect::ZERO,
            top: Rect::ZERO,
            status: Rect::ZERO,
            rail: Rect::ZERO,
            panel: Rect::ZERO,
            content: Rect::ZERO,
        }
    }
}

/// One capture signal as a feed line — plain words, layman-first.
fn activity_capture_line(ev: &dpl_fabric::CaptureEvent) -> String {
    use dpl_fabric::CaptureEvent as E;
    match ev {
        E::Started { mode, .. } => format!("{mode} started"),
        E::Saved { kind, name, .. } => format!("saved {kind} · {name}"),
        E::Failed { error } => format!("failed · {error}"),
        E::InboxChanged => "inbox changed".into(),
        E::PaletteVisibility { visible } => {
            if *visible {
                "palette shown".into()
            } else {
                "palette hidden".into()
            }
        }
    }
}

/// One job transition as a feed line.
fn activity_job_line(ev: &dpl_fabric::JobEvent) -> String {
    use dpl_fabric::JobEvent as E;
    match ev {
        E::Queued { label, .. } => format!("job queued · {label}"),
        E::Done { out, .. } => format!("job done · {out}"),
        E::Failed { error, .. } => format!("job failed · {error}"),
    }
}

pub struct Shell {
    hubs: Vec<Box<dyn Hub>>,
    route: Route,
    fabric: Fabric,
    /// The shell's own ear on the fabric (D2): hubs may request
    /// full-bleed (`ShellEvent::FullBleedRequest`); drained every frame.
    events: dpl_fabric::ShellReceiver,
    /// The app backdrop — two loops + authored random moods, painted under
    /// EVERY route's content; tiles live in the topbar.
    backdrop: dpl_backdrop::Backdrop,
    /// Navigations this session — the inspector shows it; proof the fabric fires.
    nav_count: u64,
    /// Every fabric publish this session (nav + summon) — the footer/inspector
    /// proof counter.
    signal_count: u64,
    /// THE ACTIVITY FEED (W8, signal-doctrine.md): capture + jobs ears
    /// and a ring of the last N stamped lines — the operator SEES the
    /// nervous system firing, and it doubles as the debug surface.
    capture_rx: dpl_fabric::CaptureReceiver,
    jobs_rx: dpl_fabric::JobReceiver,
    activity: std::collections::VecDeque<String>,
    /// Full-bleed: all chrome hidden, content owns the viewport (old `\` /
    /// Maximize2 behavior). Esc or the floating exit chip leaves.
    full_bleed: bool,
    /// The build stamp the footer shows (git hash · utc) — the running app
    /// itself proves which code it is.
    build: &'static str,
    /// The boot-rolled mood tinting the accordion's unfold strip ("on
    /// reload so we have variety of color") — fixed for the session, fresh
    /// every launch.
    tooltip_mood: usize,
    /// The accordion's ONE open house (doctrine §5.2) — its subroutes
    /// unfold beneath its rail row; `None` = fully folded, icon-compact.
    open_house: Option<HubId>,
    /// Last frame's zone rects (docs/shell.md).
    zones: Zones,
    /// Data-only launchers for app-owned native windows. The shell renders
    /// these generically and never knows which feature owns a target.
    window_accessories: Vec<status::WindowAccessory>,
}

impl Shell {
    /// The first registered hub's default route is home base. An empty
    /// registry degrades to a sentinel route the dispatcher renders as an
    /// honest "no hubs registered" — never a panic.
    pub fn new(hubs: Vec<Box<dyn Hub>>, fabric: Fabric) -> Self {
        let capture_rx = fabric.subscribe_capture();
        let jobs_rx = fabric.subscribe_jobs();
        Self::with_ears(hubs, fabric, capture_rx, jobs_rx)
    }

    /// Construct with PRE-OPENED signal ears (W8): the app subscribes at
    /// fabric birth — BEFORE the boot drills publish — so harness
    /// mutations land in the activity feed exactly like real ones
    /// (broadcast receivers only hear post-subscription traffic).
    pub fn with_ears(
        hubs: Vec<Box<dyn Hub>>,
        fabric: Fabric,
        capture_rx: dpl_fabric::CaptureReceiver,
        jobs_rx: dpl_fabric::JobReceiver,
    ) -> Self {
        let route = hubs
            .first()
            .map(|h| h.spec().default_route())
            .unwrap_or(Route {
                hub: HubId("_none"),
                sub: SubId("_none"),
            });
        let events = fabric.subscribe_shell();
        Self {
            hubs,
            route,
            fabric,
            events,
            capture_rx,
            jobs_rx,
            activity: std::collections::VecDeque::new(),
            backdrop: dpl_backdrop::Backdrop::default(),
            nav_count: 0,
            signal_count: 0,
            full_bleed: false,
            build: "",
            tooltip_mood: dpl_backdrop::random_mood_index(),
            open_house: None,
            zones: Zones::default(),
            window_accessories: Vec::new(),
        }
    }

    /// Register one app-owned native-window launcher.
    ///
    /// Duplicate keys are a wiring error because one accessory must resolve
    /// to exactly one coordinator target.
    pub fn register_window_accessory(
        &mut self,
        accessory: status::WindowAccessory,
    ) -> Result<(), AppWindowKey> {
        if self
            .window_accessories
            .iter()
            .any(|registered| registered.key == accessory.key)
        {
            return Err(accessory.key);
        }
        self.window_accessories.push(accessory);
        Ok(())
    }

    /// Last frame's zone rects — the layout contract, observable.
    pub fn zones(&self) -> Zones {
        self.zones
    }

    /// The mood tinting the accordion's unfold strip — rolled once at boot.
    pub fn tooltip_mood(&self) -> &'static cl::Mood {
        &cl::MOODS[self.tooltip_mood]
    }

    pub fn set_build(&mut self, stamp: &'static str) {
        self.build = stamp;
    }

    /// Direct backdrop access for operator interactions and focused tests.
    pub fn backdrop_mut(&mut self) -> &mut dpl_backdrop::Backdrop {
        &mut self.backdrop
    }

    /// Pins every boot-randomized shell mood for deterministic harness runs.
    pub fn set_harness_mood(&mut self, index: usize) {
        let index = index % cl::MOOD_COUNT;
        self.backdrop.set_mood(index);
        self.tooltip_mood = index;
    }

    pub fn full_bleed(&self) -> bool {
        self.full_bleed
    }

    pub fn set_full_bleed(&mut self, on: bool) {
        self.full_bleed = on;
    }

    /// The footer Snip launcher (and the global hotkey, later): summon the
    /// ViziWiz overlay via the fabric.
    pub fn summon_viziwiz(&mut self) {
        self.signal_count += 1;
        self.fabric.publish_shell(ShellEvent::ViziwizSummon);
    }

    pub fn signal_count(&self) -> u64 {
        self.signal_count
    }

    pub fn route(&self) -> Route {
        self.route
    }

    pub fn nav_count(&self) -> u64 {
        self.nav_count
    }

    /// Navigate and broadcast. Idempotent — re-clicking the current route
    /// publishes nothing.
    pub fn navigate(&mut self, route: Route) {
        if self.route != route {
            self.route = route;
            self.nav_count += 1;
            self.signal_count += 1;
            self.fabric
                .publish_shell(ShellEvent::RouteChanged { route });
        }
    }

    /// Clicking a hub in the rail: land on its default subroute.
    pub fn activate_hub(&mut self, id: HubId) {
        if let Some(spec) = self
            .hubs
            .iter()
            .find(|h| h.spec().id == id)
            .map(|h| h.spec())
        {
            self.navigate(spec.default_route());
        }
    }

    /// The accordion's open house (doctrine §5.2) — `None` = folded.
    pub fn open_house(&self) -> Option<HubId> {
        self.open_house
    }

    /// Harness access (`--open-rail <hub>` pins the accordion open for
    /// reproducible proof shots).
    pub fn set_open_house(&mut self, id: Option<HubId>) {
        self.open_house = id;
    }

    /// Tapping a house in the rail — THE accordion gesture: the open house
    /// folds (no navigation); any other house becomes the one open house
    /// (folding the previous implicitly) AND navigates to its default
    /// subroute, so one tap still goes somewhere (touch-first law).
    pub fn toggle_house(&mut self, id: HubId) {
        if self.open_house == Some(id) {
            self.open_house = None;
        } else {
            self.open_house = Some(id);
            self.activate_hub(id);
        }
    }

    /// One frame of the whole shell. Panel order matters (outermost first,
    /// CentralPanel last); the frame overlay paints after everything.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // EVERY chrome dimension resolves through the generated fluid `shell`
        // token group at the live viewport — nothing hardcoded (workspace law).
        let fl = cl::Fluid::of(ui.ctx().content_rect());
        let root = ui.max_rect();

        // Drain the fabric ear: full-bleed asks (D2) and navigation asks
        // (the NAVIGATE seam — tapped hub cards). Lagged is fine — these
        // are signals, the latest state wins.
        let mut nav_request: Option<Route> = None;
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                ShellEvent::FullBleedRequest(on) => self.full_bleed = on,
                ShellEvent::NavigateRequest { route } => nav_request = Some(route),
                ShellEvent::RouteChanged { .. }
                | ShellEvent::ViziwizSummon
                | ShellEvent::WindowRequest { .. } => {}
            }
        }
        if let Some(route) = nav_request {
            self.navigate(route);
        }
        // THE ACTIVITY FEED drain (W8): capture + jobs signals become
        // human lines in the ring (cap 256); the inspector renders the
        // tail. Lossy is fine — this is a window, not a ledger.
        while let Ok(st) = self.capture_rx.try_recv() {
            self.signal_count += 1;
            self.push_activity(st.ms, activity_capture_line(&st.ev));
        }
        while let Ok(st) = self.jobs_rx.try_recv() {
            self.signal_count += 1;
            self.push_activity(st.ms, activity_job_line(&st.ev));
        }

        // Esc leaves full-bleed (the old shell's escape hatch).
        if self.full_bleed && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.full_bleed = false;
        }

        if self.full_bleed {
            // Content owns the viewport; the only chrome is the exit chip.
            let content = self.central(ui);
            self.zones = Zones {
                root,
                content,
                ..Zones::default()
            };
            self.fullbleed_exit(ui, content);
            return;
        }

        let tone = self.active_tone();
        let (top, go_fullbleed) = self.top_bar(ui, &fl);
        if go_fullbleed {
            self.full_bleed = true;
        }
        let (status_rect, actions) = self.status_bar(ui, &fl);
        for action in actions {
            match action {
                status::StatusAction::Snip => self.summon_viziwiz(),
                status::StatusAction::Window(target) => {
                    self.signal_count += 1;
                    self.fabric.publish_shell(ShellEvent::WindowRequest {
                        target,
                        action: AppWindowAction::OpenOrFocus,
                    });
                }
            }
        }
        let (rail, clicked) = self.rail(ui, &fl);
        if let Some(route) = clicked {
            self.navigate(route);
        }
        let panel = self.inspector(ui, &fl);
        let content = self.central(ui);
        self.zones = Zones {
            root,
            top,
            status: status_rect,
            rail,
            panel,
            content,
        };
        // The overlay: seams + diffusion framing the well. Painted last, on
        // top of every plate edge — this is what makes the chrome float.
        cl::frame_overlay(ui.painter(), content, tone);
    }

    /// The floating full-bleed exit chip (old `FullbleedExit`): top-right,
    /// the only chrome while everything else is hidden.
    fn fullbleed_exit(&mut self, ui: &mut egui::Ui, content: Rect) {
        let size = egui::vec2(34.0, 28.0);
        let rect = Rect::from_min_size(
            egui::pos2(content.max.x - size.x - 12.0, content.min.y + 12.0),
            size,
        );
        let resp = ui.interact(rect, egui::Id::new("dpl_fullbleed_exit"), Sense::click());
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            CornerRadius::same(cl::RADIUS_SM),
            cl::surface::OVERLAY,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            cl::icon::FULLBLEED_EXIT,
            egui::FontId::proportional(15.0),
            if resp.hovered() {
                cl::ink::STRONG
            } else {
                cl::ink::MUTED
            },
        );
        if resp.clicked() {
            self.full_bleed = false;
        }
    }

    // ── TOP: identity + breadcrumb; backdrop tiles + full-bleed right ────
    /// Returns the zone rect and whether the full-bleed toggle was clicked.
    fn top_bar(&mut self, ui: &mut egui::Ui, fl: &cl::Fluid) -> (Rect, bool) {
        let (hub_label, sub_label, tone) = self.breadcrumb();
        let backdrop = &mut self.backdrop;
        let mut go_fullbleed = false;
        let resp = egui::Panel::top("dpl_top")
            .exact_size(fl.topbar_height())
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .inner_margin(egui::Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| {
                cl::sheen(ui.painter(), ui.max_rect().expand(8.0));
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new("DevPULSE")
                            .strong()
                            .color(cl::ink::STRONG)
                            .size(15.0),
                    );
                    ui.label(RichText::new("LABS").color(tone.color()).size(15.0));
                    ui.add_space(cl::space::S4);
                    ui.label(RichText::new(hub_label).color(cl::ink::BASE));
                    ui.label(RichText::new("›").color(cl::ink::SUBTLE));
                    ui.label(RichText::new(sub_label).color(tone.color()));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, cl::icon::FULLBLEED, "full-bleed (Esc exits)") {
                            go_fullbleed = true;
                        }
                        ui.add_space(cl::space::S2);
                        // The two loops; every click pulls a fresh mood.
                        backdrop.tiles(ui);
                    });
                });
            });
        (resp.response.rect, go_fullbleed)
    }

    // ── STATUS: the indicator grid — three lanes of registered cells ────
    fn status_bar(&self, ui: &mut egui::Ui, fl: &cl::Fluid) -> (Rect, Vec<status::StatusAction>) {
        let mut actions = Vec::new();
        let tone = self.active_tone();
        let resp = egui::Panel::bottom("dpl_status")
            .exact_size(fl.bottombar_height())
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .inner_margin(egui::Margin::symmetric(14, 6)),
            )
            .show(ui, |ui| {
                cl::sheen(ui.painter(), ui.max_rect().expand(6.0));
                let cx = status::StatusCtx {
                    tone,
                    viewport: ui.ctx().content_rect().size(),
                    ppp: ui.ctx().pixels_per_point(),
                    navs: self.nav_count,
                    signals: self.signal_count,
                    build: self.build,
                    mood: self.backdrop.mood(),
                };
                actions = status::grid(ui, &cx, &self.window_accessories);
            });
        (resp.response.rect, actions)
    }

    // ── RAIL: the accordion — tap a house, its subroutes unfold beneath
    // it and the houses below push down; ONE open at a time (doctrine
    // §5.2), folded is icon-compact. ─────────────────────────────────────
    fn rail(&mut self, ui: &mut egui::Ui, fl: &cl::Fluid) -> (Rect, Option<Route>) {
        let mut clicked: Option<Route> = None;
        let mut toggled: Option<HubId> = None;
        // One unfold factor per house, animated toward "is the open house".
        // Computed BEFORE the panel because the rail's WIDTH answers the
        // accordion: folded rides the collapsed token, an open house widens
        // it to the expanded token (both generated — workspace law).
        let factors: Vec<f32> = self
            .hubs
            .iter()
            .map(|h| {
                let id = h.spec().id;
                ui.ctx().animate_bool_with_time(
                    egui::Id::new(("dpl_rail_unfold", id.0)),
                    self.open_house == Some(id),
                    0.18,
                )
            })
            .collect();
        let max_f = factors.iter().copied().fold(0.0_f32, f32::max);
        let collapsed = fl.rail_collapsed();
        let width = collapsed + (fl.rail_expanded() - collapsed) * max_f;
        // The icon keeps the collapsed column's center while the rail
        // widens (6+6 = the panel's symmetric inner margin).
        let icon_col = collapsed - 12.0;
        let mood = self.tooltip_mood();
        let route = self.route;
        let open_house = self.open_house;
        let hubs = &self.hubs;
        let resp = egui::Panel::left("dpl_rail")
            .exact_size(width)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .inner_margin(egui::Margin::symmetric(6, 10)),
            )
            .show(ui, |ui| {
                cl::sheen(ui.painter(), ui.max_rect().expand(10.0));
                for (hub, &f) in hubs.iter().zip(&factors) {
                    let spec = hub.spec();
                    let active = spec.id == route.hub;
                    let open = open_house == Some(spec.id);
                    let r = cl::tip(
                        house_tile(
                            ui,
                            &HouseTile {
                                icon: spec.icon,
                                label: spec.label,
                                tone: spec.tone,
                                active,
                                expand: max_f,
                                height: fl.tap_min(),
                                icon_col,
                            },
                        ),
                        spec.label,
                    );
                    ui.pilot_track(
                        &r,
                        NodeMetadata::new(
                            PilotId::new("shell.rail").child(&PilotKey::new(spec.id.0)),
                            egui_pilot_ai::protocol::Role::Button,
                            spec.label,
                        )
                        .value(match (active, open) {
                            (true, true) => "active+open",
                            (true, false) => "active",
                            (false, true) => "open",
                            (false, false) => "inactive",
                        }),
                    );
                    if r.clicked() {
                        toggled = Some(spec.id);
                    }
                    // The unfold: the section allocates its full height
                    // scaled by the factor and clip-reveals from the top —
                    // subroutes extend DOWN, the houses below push down.
                    if f > 0.001 {
                        let w = ui.available_width();
                        let gap = ui.spacing().item_spacing.y;
                        let rows = spec.subroutes.len() as f32;
                        let full_h = 8.0 + rows * 26.0 + (rows - 1.0).max(0.0) * gap;
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(w, full_h * f), Sense::hover());
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(Rect::from_min_size(rect.min, egui::vec2(w, full_h)))
                                .layout(Layout::top_down(Align::Min)),
                        );
                        child.set_clip_rect(rect.intersect(ui.clip_rect()));
                        // The unfold PLATE: the open house's subroutes sit on
                        // their own recessed background — visibly a different
                        // layer than the rail (operator ask: "slightly
                        // indented with a different background").
                        child.painter().rect_filled(
                            Rect::from_min_size(rect.min, egui::vec2(w, full_h)),
                            CornerRadius::same(cl::RADIUS_SM),
                            cl::surface::BASE,
                        );
                        // The boot-rolled mood strip — the "random mood on
                        // reload", carried over from the flyout it replaces.
                        let strip = Rect::from_min_size(rect.min, egui::vec2(w, 3.0));
                        cl::mood_gradient(child.painter(), strip, mood);
                        child.add_space(8.0);
                        for sub in spec.subroutes {
                            let selected = active && sub.id == route.sub;
                            let response = cl::tip(
                                sub_item(&mut child, sub.label, spec.tone, selected),
                                sub.blurb,
                            );
                            child.pilot_track(
                                &response,
                                NodeMetadata::new(
                                    PilotId::new("shell.rail.sub").child(&PilotKey::new(format!(
                                        "{}/{}",
                                        spec.id.0, sub.id.0
                                    ))),
                                    egui_pilot_ai::protocol::Role::MenuItem,
                                    sub.label,
                                )
                                .value(if selected {
                                    "selected"
                                } else {
                                    "available"
                                }),
                            );
                            if response.clicked() {
                                clicked = Some(Route {
                                    hub: spec.id,
                                    sub: sub.id,
                                });
                            }
                        }
                    }
                    ui.add_space(cl::space::S1);
                }
            });
        if let Some(id) = toggled {
            self.toggle_house(id);
        }
        (resp.response.rect, clicked)
    }

    /// Ring-push one activity line (cap 256 — the flight-recorder WINDOW;
    /// the full truth lives in session logs and the seam stream).
    fn push_activity(&mut self, ms: u64, line: String) {
        let secs = ms / 1000;
        self.activity
            .push_back(format!("{:02}:{:02} {line}", secs / 60, secs % 60));
        while self.activity.len() > 256 {
            self.activity.pop_front();
        }
    }

    // ── PANEL: the inspector ────────────────────────────────────────────
    fn inspector(&self, ui: &mut egui::Ui, fl: &cl::Fluid) -> Rect {
        let (hub_label, sub_label, tone) = self.breadcrumb();
        let resp = egui::Panel::right("dpl_inspector")
            .exact_size(fl.panel_width())
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .inner_margin(egui::Margin::symmetric(12, 12)),
            )
            .show(ui, |ui| {
                cl::sheen(ui.painter(), ui.max_rect().expand(10.0));

                ui.label(RichText::new("INSPECTOR").small().color(cl::ink::SUBTLE));
                cl::card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        cl::tone_dot(ui, tone);
                        ui.label(RichText::new(hub_label).strong().color(cl::ink::STRONG));
                    });
                    ui.label(RichText::new(sub_label).color(tone.color()));
                    ui.label(
                        RichText::new(format!("{}/{}", self.route.hub, self.route.sub))
                            .monospace()
                            .small()
                            .color(cl::ink::MUTED),
                    );
                });

                ui.add_space(cl::space::S3);
                ui.label(RichText::new("SIGNALS").small().color(cl::ink::SUBTLE));
                cl::card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("route changes").small().color(cl::ink::MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{}", self.nav_count))
                                    .monospace()
                                    .color(cl::ink::BASE),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("shell bus").small().color(cl::ink::MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new("broadcast·64")
                                    .monospace()
                                    .color(cl::ink::BASE),
                            );
                        });
                    });
                    // THE ACTIVITY FEED (W8): the nervous system, live —
                    // newest first, plain words, mm:ss since boot.
                    if self.activity.is_empty() {
                        ui.label(
                            RichText::new("quiet — saves, imports, and jobs appear here live")
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                    } else {
                        for line in self.activity.iter().rev().take(9) {
                            ui.label(
                                RichText::new(line.as_str())
                                    .monospace()
                                    .small()
                                    .color(cl::ink::BASE),
                            );
                        }
                    }
                });

                ui.add_space(cl::space::S3);
                ui.label(RichText::new("TOOLS").small().color(cl::ink::SUBTLE));
                ui.label(
                    RichText::new("Per-hub tool docks mount here (P1).")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            });
        resp.response.rect
    }

    // ── central: the recessed well, dispatch to the active hub ──────────
    // The backdrop (loop + mood) paints under EVERY route's content — the
    // well is never flat.
    fn central(&mut self, ui: &mut egui::Ui) -> Rect {
        let route = self.route;
        let idx = self.hubs.iter().position(|h| h.spec().id == route.hub);
        let resp = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::WELL)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| {
                // clip_rect = the panel's full plate, margins included.
                self.backdrop.paint(ui, ui.clip_rect());
                match idx {
                    Some(i) => self.hubs[i].ui(ui, route),
                    None => {
                        // Unknown → degrade, never crash (Law 4).
                        ui.label(
                            RichText::new(format!("no hub registered for \"{}\"", route.hub))
                                .color(cl::ink::MUTED),
                        );
                    }
                }
            });
        resp.response.rect
    }

    fn active_tone(&self) -> Tone {
        self.hubs
            .iter()
            .find(|h| h.spec().id == self.route.hub)
            .map(|h| h.spec().tone)
            .unwrap_or(Tone::Cyan)
    }

    fn breadcrumb(&self) -> (&'static str, &'static str, Tone) {
        self.hubs
            .iter()
            .find(|h| h.spec().id == self.route.hub)
            .map(|h| {
                let spec = h.spec();
                let sub = spec
                    .subroute(self.route.sub)
                    .map(|s| s.label)
                    .unwrap_or("—");
                (spec.label, sub, spec.tone)
            })
            .unwrap_or(("—", "—", Tone::Cyan))
    }
}

/// A quiet icon-only button (topbar/full-bleed chrome).
fn icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), Sense::click());
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(
            rect,
            CornerRadius::same(cl::RADIUS_SM),
            cl::surface::OVERLAY,
        );
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(15.0),
        if resp.hovered() {
            cl::ink::STRONG
        } else {
            cl::ink::MUTED
        },
    );
    resp.on_hover_text(tip).clicked()
}

/// A footer launcher chip — BRUTAL (doctrine): hard offset shadow, tone
/// edge, hover presses the chip onto its shadow. The `tip` is LAW.
pub(crate) fn launcher_chip(
    ui: &mut egui::Ui,
    glyph: &str,
    label: &str,
    tone: Tone,
    tip: &str,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.into(),
        egui::FontId::proportional(12.5),
        cl::ink::BASE,
    );
    let w = 22.0 + galley.size().x + 19.0;
    let (slot, resp) = ui.allocate_exact_size(egui::vec2(w, 28.0), Sense::click());
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
        CornerRadius::same(cl::RADIUS_SM),
        egui::Color32::from_black_alpha(230),
    );
    painter.rect_filled(
        chip,
        CornerRadius::same(cl::RADIUS_SM),
        if resp.hovered() {
            tone.wash()
        } else {
            cl::surface::OVERLAY
        },
    );
    painter.rect_stroke(
        chip,
        CornerRadius::same(cl::RADIUS_SM),
        egui::Stroke::new(2.0, tone.wash()),
        egui::StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(chip.min.x + 8.0, chip.center().y),
        egui::Align2::LEFT_CENTER,
        glyph,
        egui::FontId::proportional(14.0),
        tone.color(),
    );
    painter.text(
        egui::pos2(chip.min.x + 24.0, chip.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.5),
        if resp.hovered() {
            cl::ink::STRONG
        } else {
            cl::ink::BASE
        },
    );
    cl::tip(resp, tip)
}

/// One house row's paint inputs (bundled — the workspace's no-`#[allow]`
/// law meets clippy's argument budget the honest way).
struct HouseTile<'a> {
    icon: &'a str,
    label: &'a str,
    tone: Tone,
    active: bool,
    /// The rail's widen factor 0..1 — the label fades in with it.
    expand: f32,
    /// Row height — the `tap-min` token.
    height: f32,
    /// The COLLAPSED column's inner width; the glyph keeps its center
    /// while the rail widens.
    icon_col: f32,
}

/// A house row in the accordion rail: glyph centered in the collapsed
/// column, the hub label fading in beside it as the rail widens; tone
/// wash, sprite bevel (1px lit top, 1px dark bottom), and the left accent
/// bar when active — the icon-rail sprite vocabulary, width-aware.
fn house_tile(ui: &mut egui::Ui, t: &HouseTile) -> egui::Response {
    let HouseTile {
        icon,
        label,
        tone,
        active,
        expand,
        height,
        icon_col,
    } = *t;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let painter = ui.painter();

    if active {
        painter.rect_filled(rect, CornerRadius::same(cl::RADIUS_SM), tone.wash());
        let lit = Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 14);
        let dark = Color32::from_black_alpha(120);
        painter.rect_filled(
            Rect::from_min_max(
                egui::pos2(rect.min.x + 3.0, rect.min.y),
                egui::pos2(rect.max.x - 3.0, rect.min.y + 1.0),
            ),
            0,
            lit,
        );
        painter.rect_filled(
            Rect::from_min_max(
                egui::pos2(rect.min.x + 3.0, rect.max.y - 1.0),
                egui::pos2(rect.max.x - 3.0, rect.max.y),
            ),
            0,
            dark,
        );
        // The accent bar — a crisp 3px tone edge.
        let bar = Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height()));
        painter.rect_filled(bar, CornerRadius::same(1), tone.color());
    } else if response.hovered() {
        painter.rect_filled(
            rect,
            CornerRadius::same(cl::RADIUS_SM),
            cl::surface::OVERLAY,
        );
    }

    painter.text(
        egui::pos2(rect.min.x + icon_col / 2.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(19.0),
        if active { tone.color() } else { cl::ink::MUTED },
    );
    if expand > 0.01 {
        painter.text(
            egui::pos2(rect.min.x + icon_col + 2.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.5),
            (if active {
                cl::ink::STRONG
            } else {
                cl::ink::BASE
            })
            .gamma_multiply(expand),
        );
    }
    response
}

/// A subroute row in the accordion's unfold — quiet, tone dot when
/// selected.
fn sub_item(ui: &mut egui::Ui, label: &str, tone: Tone, selected: bool) -> egui::Response {
    let width = ui.available_width();
    let (slot, response) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::click());
    // The row sits INDENTED on the unfold plate — the accordion's rows read
    // as children of the house, not siblings.
    let rect = Rect::from_min_max(egui::pos2(slot.min.x + 10.0, slot.min.y), slot.max);
    let painter = ui.painter();

    if selected {
        let r = rect.shrink2(egui::vec2(0.0, 1.0));
        painter.rect(
            r,
            CornerRadius::same(cl::RADIUS_SM),
            Color32::TRANSPARENT,
            egui::Stroke::new(1.0, tone.wash()),
            egui::StrokeKind::Inside,
        );
        painter.circle_filled(
            egui::pos2(rect.min.x + 10.0, rect.center().y),
            2.5,
            tone.color(),
        );
    } else if response.hovered() {
        painter.rect_filled(
            rect,
            CornerRadius::same(cl::RADIUS_SM),
            cl::surface::OVERLAY,
        );
    }

    painter.text(
        egui::pos2(rect.min.x + 22.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        if selected {
            tone.color()
        } else if response.hovered() {
            // The mouse-over effect: the ink wakes up under the pointer.
            cl::ink::STRONG
        } else {
            cl::ink::MUTED
        },
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_hub::{HubSpec, SubRoute};

    static SPEC_A: HubSpec = HubSpec {
        id: HubId("a"),
        label: "A",
        icon: "•",
        tone: Tone::Cyan,
        subroutes: &[
            SubRoute {
                id: SubId("one"),
                label: "One",
                blurb: "",
            },
            SubRoute {
                id: SubId("two"),
                label: "Two",
                blurb: "",
            },
        ],
    };
    static SPEC_B: HubSpec = HubSpec {
        id: HubId("b"),
        label: "B",
        icon: "•",
        tone: Tone::Lime,
        subroutes: &[SubRoute {
            id: SubId("only"),
            label: "Only",
            blurb: "",
        }],
    };

    struct Fake(&'static HubSpec);
    impl Hub for Fake {
        fn spec(&self) -> &'static HubSpec {
            self.0
        }
        fn ui(&mut self, _ui: &mut egui::Ui, _route: Route) {}
    }

    fn shell() -> (Shell, tokio::sync::broadcast::Receiver<ShellEvent>) {
        let fabric = Fabric::new();
        let rx = fabric.subscribe_shell();
        (
            Shell::new(
                vec![Box::new(Fake(&SPEC_A)), Box::new(Fake(&SPEC_B))],
                fabric,
            ),
            rx,
        )
    }

    #[test]
    fn boots_on_the_first_hubs_default_subroute() {
        let (shell, _rx) = shell();
        assert_eq!(
            shell.route(),
            Route {
                hub: HubId("a"),
                sub: SubId("one")
            }
        );
    }

    #[test]
    fn activating_a_hub_lands_on_its_default_and_publishes() {
        let (mut shell, mut rx) = shell();
        shell.activate_hub(HubId("b"));
        let expected = Route {
            hub: HubId("b"),
            sub: SubId("only"),
        };
        assert_eq!(shell.route(), expected);
        assert_eq!(
            rx.try_recv().unwrap(),
            ShellEvent::RouteChanged { route: expected }
        );
    }

    #[test]
    fn renavigating_to_the_same_route_publishes_nothing() {
        let (mut shell, mut rx) = shell();
        let here = shell.route();
        shell.navigate(here);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn nav_count_tracks_real_navigations_only() {
        let (mut shell, _rx) = shell();
        assert_eq!(shell.nav_count(), 0);
        shell.navigate(shell.route()); // no-op
        assert_eq!(shell.nav_count(), 0);
        shell.activate_hub(HubId("b"));
        assert_eq!(shell.nav_count(), 1);
    }

    #[test]
    fn tapping_a_closed_house_opens_it_and_navigates() {
        let (mut shell, mut rx) = shell();
        shell.toggle_house(HubId("b"));
        assert_eq!(shell.open_house(), Some(HubId("b")));
        let expected = Route {
            hub: HubId("b"),
            sub: SubId("only"),
        };
        assert_eq!(shell.route(), expected);
        assert_eq!(
            rx.try_recv().unwrap(),
            ShellEvent::RouteChanged { route: expected }
        );
    }

    #[test]
    fn tapping_the_open_house_folds_without_navigating() {
        let (mut shell, mut rx) = shell();
        shell.toggle_house(HubId("b"));
        let _ = rx.try_recv();
        let before = shell.route();
        shell.toggle_house(HubId("b"));
        assert_eq!(shell.open_house(), None, "the open house must fold");
        assert_eq!(shell.route(), before, "folding never navigates");
        assert!(rx.try_recv().is_err(), "folding publishes nothing");
    }

    #[test]
    fn opening_another_house_folds_the_first() {
        let (mut shell, _rx) = shell();
        shell.toggle_house(HubId("b"));
        shell.toggle_house(HubId("a"));
        // ONE open at a time (doctrine §5.2) — b folded when a opened.
        assert_eq!(shell.open_house(), Some(HubId("a")));
        assert_eq!(shell.route().hub, HubId("a"));
    }

    /// The accordion's width contract: an open house widens the rail from
    /// the collapsed token to the expanded token (both generated).
    #[test]
    fn an_open_house_widens_the_rail_to_the_expanded_token() {
        let size = egui::vec2(1280.0, 800.0);
        let (mut sh, _rx) = shell();
        sh.set_open_house(Some(HubId("a")));
        let mut harness = egui_kittest::Harness::builder()
            .with_size(size)
            .build_ui_state(|ui, sh: &mut Shell| sh.ui(ui), sh);
        // Enough steps for the 0.18s unfold animation to settle.
        harness.run_steps(90);
        let z = harness.state().zones();
        let fl = cl::Fluid::of(Rect::from_min_size(egui::Pos2::ZERO, size));
        assert!(
            (z.rail.width() - fl.rail_expanded()).abs() < 0.75,
            "open rail width {} != rail_expanded {}",
            z.rail.width(),
            fl.rail_expanded()
        );
    }

    #[test]
    fn activating_an_unknown_hub_degrades_to_a_noop() {
        let (mut shell, mut rx) = shell();
        let before = shell.route();
        shell.activate_hub(HubId("ghost"));
        assert_eq!(shell.route(), before);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn an_empty_registry_boots_on_the_sentinel_not_a_panic() {
        let shell = Shell::new(vec![], Fabric::new());
        assert_eq!(shell.route().hub, HubId("_none"));
    }

    #[test]
    fn the_snip_launcher_publishes_viziwiz_summon() {
        let (mut shell, mut rx) = shell();
        shell.summon_viziwiz();
        assert_eq!(rx.try_recv().unwrap(), ShellEvent::ViziwizSummon);
        assert_eq!(shell.signal_count(), 1);
    }

    #[test]
    fn window_accessory_keys_are_unique() {
        let (mut shell, _rx) = shell();
        let accessory = status::WindowAccessory {
            key: AppWindowKey("test.window"),
            pilot_id: PilotId::new("shell.window.test"),
            glyph: "W",
            label: "Window",
            tone: Tone::Cyan,
            tooltip: "Open the test window",
        };
        assert!(shell.register_window_accessory(accessory).is_ok());
        assert_eq!(
            shell.register_window_accessory(accessory),
            Err(AppWindowKey("test.window"))
        );
    }

    #[test]
    fn signals_count_every_publish_navs_included() {
        let (mut shell, _rx) = shell();
        shell.activate_hub(HubId("b"));
        shell.summon_viziwiz();
        assert_eq!(shell.nav_count(), 1);
        assert_eq!(shell.signal_count(), 2);
    }

    #[test]
    fn full_bleed_is_off_by_default_and_togglable() {
        let (mut shell, _rx) = shell();
        assert!(!shell.full_bleed());
        shell.set_full_bleed(true);
        assert!(shell.full_bleed());
    }

    /// D2: a hub's FullBleedRequest on the fabric flips the shell on the
    /// next frame — the shell's first fabric CONSUMER.
    #[test]
    fn a_full_bleed_request_on_the_fabric_takes_effect_next_frame() {
        let fabric = Fabric::new();
        let sh = Shell::new(vec![Box::new(Fake(&SPEC_A))], fabric.clone());
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_ui_state(|ui, sh: &mut Shell| sh.ui(ui), sh);
        harness.run_steps(1);
        assert!(!harness.state().full_bleed());
        fabric.publish_shell(ShellEvent::FullBleedRequest(true));
        harness.run_steps(1);
        assert!(harness.state().full_bleed(), "the shell must hear the hub");
        fabric.publish_shell(ShellEvent::FullBleedRequest(false));
        harness.run_steps(1);
        assert!(!harness.state().full_bleed());
    }

    #[test]
    fn the_boot_tooltip_mood_comes_from_the_registry() {
        let (shell, _rx) = shell();
        assert!(cl::MOODS
            .iter()
            .any(|m| m.name == shell.tooltip_mood().name));
    }

    #[test]
    fn harness_mood_pins_backdrop_and_flyout_together() {
        let (mut shell, _rx) = shell();
        shell.set_harness_mood(3);
        assert_eq!(shell.backdrop.mood().name, cl::MOODS[3].name);
        assert_eq!(shell.tooltip_mood().name, cl::MOODS[3].name);
    }

    /// THE layout contract (docs/shell.md), pinned headlessly at two real
    /// viewport sizes: chrome dimensions ARE the generated fluid `shell`
    /// tokens, the status bar ends exactly at the viewport bottom (the v1
    /// regression), and the well tiles inside the chrome without overlap.
    #[test]
    fn zones_pin_chrome_to_the_viewport_at_any_size() {
        for size in [egui::vec2(1280.0, 800.0), egui::vec2(1920.0, 1080.0)] {
            let (sh, _rx) = shell();
            let mut harness = egui_kittest::Harness::builder()
                .with_size(size)
                .build_ui_state(|ui, sh: &mut Shell| sh.ui(ui), sh);
            harness.run_steps(3);
            let z = harness.state().zones();
            let fl = cl::Fluid::of(Rect::from_min_size(egui::Pos2::ZERO, size));
            let eq = |a: f32, b: f32| (a - b).abs() < 0.75;

            // Kittest insets the root by a small test-only outer margin
            // (eframe's real root has none) — the root must still span
            // essentially the whole viewport.
            assert!(size.y - z.root.height() <= 20.0, "root lost the viewport");
            assert!(
                eq(z.status.max.y, z.root.max.y),
                "status bottom {} != root bottom {} — the off-screen-footer regression",
                z.status.max.y,
                z.root.max.y
            );
            assert!(
                eq(z.top.min.y, z.root.min.y),
                "topbar not pinned to the top"
            );
            assert!(eq(z.top.height(), fl.topbar_height()));
            assert!(eq(z.status.height(), fl.bottombar_height()));
            assert!(eq(z.rail.width(), fl.rail_collapsed()));
            assert!(eq(z.panel.width(), fl.panel_width()));

            assert!(z.content.min.y >= z.top.max.y - 0.5);
            assert!(z.content.max.y <= z.status.min.y + 0.5);
            assert!(z.content.min.x >= z.rail.max.x - 0.5);
            assert!(z.content.max.x <= z.panel.min.x + 0.5);
        }
    }
}
