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
//! token group at the live viewport; nothing hardcoded (workspace law).
//! PANEL is an OPTIONAL OVERLAY (the operator's "rightside overlay the
//! content"): closed by default — the content well owns the full width —
//! and when opened it floats OVER the well's right edge, never docking. The
//! rail is an ACCORDION (Program L, doctrine §5.2): tapping a house unfolds
//! its subroutes downward inside the rail — the houses below push down —
//! and folds whatever was open; one open at a time, folded is icon-mode
//! compact. When the accordion exceeds the rail zone the body scrolls so
//! the open house stays reachable (p4). The status bar is a data-driven
//! cell registry ([`status`]).
//! Every navigation publishes `ShellEvent::RouteChanged` on the fabric.

pub mod providers;
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
    /// The inspector overlay's plate — `Rect::ZERO` while it is closed
    /// (closed reserves no width; the content spans to the root's edge).
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
        E::Recording { on, count } => {
            if *on {
                format!("recording · {count} steps")
            } else {
                format!("recording stopped · {count} steps")
            }
        }
        E::Skipped { reason } => format!("skipped · {reason}"),
    }
}

/// One coach lifecycle fact as a feed line — the lesson breathing, live.
fn activity_coach_line(ev: &dpl_fabric::CoachEvent) -> String {
    use dpl_fabric::CoachEvent as E;
    match ev {
        E::Armed { coach_id } => format!("coach armed · {coach_id}"),
        E::Mounted { coach_id } => format!("coach mounted · {coach_id}"),
        E::Triggered {
            coach_id, trigger, ..
        } => format!("coach fired · {coach_id} · {trigger}"),
        E::Satisfied { coach_id } => format!("step satisfied · {coach_id}"),
        E::Unmounted { coach_id, reason } => format!("coach done · {coach_id} · {reason}"),
    }
}

fn activity_intent_line(ev: &dpl_fabric::IntentEvent) -> String {
    use dpl_fabric::IntentEvent as E;
    match ev {
        E::DeterminismChanged { mode, cause } => {
            let mode = match mode {
                dpl_intent::DeterminismMode::Behavioral => "behavioral",
                dpl_intent::DeterminismMode::Strict => "strict",
            };
            match cause {
                None => format!("determinism · {mode}"),
                Some(dpl_intent::StrictCause::Operator(
                    dpl_intent::IntentChannel::UiAcknowledged,
                )) => format!("determinism · {mode} · ui"),
                Some(dpl_intent::StrictCause::Operator(
                    dpl_intent::IntentChannel::SlashCommand,
                )) => format!("determinism · {mode} · slash"),
                Some(dpl_intent::StrictCause::Evidence) => {
                    format!("determinism · {mode} · evidence")
                }
            }
        }
        E::IntentRefused { action, reason } => format!("intent refused · {action} · {reason}"),
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

impl Shell {
    /// Paint the Providers roster popup above the status bar (V2).
    ///
    /// An `egui::Area` rather than a panel: it floats over the content the
    /// way the mock draws it, and it must not reflow the surface underneath —
    /// opening a roster should never move the operator's work.
    fn providers_popup(&mut self, ui: &mut egui::Ui, status: Rect, just_toggled: bool) {
        if !self.providers_open {
            return;
        }
        let roster = self.fabric.provider_roster();
        let width = 320.0;
        let area = egui::Area::new(egui::Id::new("dpl_providers_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(
                (status.max.x - width - cl::space::S4).max(status.min.x),
                status.min.y - 12.0,
            ))
            .pivot(egui::Align2::LEFT_BOTTOM);
        let response = area.show(ui.ctx(), |ui| {
            ui.set_width(width);
            egui::Frame::new()
                .fill(cl::surface::OVERLAY)
                .corner_radius(egui::CornerRadius::same(cl::RADIUS_LG))
                .inner_margin(egui::Margin::same(14))
                .stroke(egui::Stroke::new(1.0, cl::surface::RAISED))
                .show(ui, |ui| {
                    providers::ProvidersPopup { providers: &roster }.show(ui)
                })
                .inner
        });
        // V3 promotes a card into the provider modal. Until then the click is
        // inert BY DESIGN, and the card's tooltip already carries the summary
        // so the operator is not left staring at a dead control.
        let _ = response.inner;
        // Click anywhere outside closes it, the way every menu in the app does.
        if !just_toggled && response.response.clicked_elsewhere() {
            self.providers_open = false;
        }
        // Esc closes it too — the keyboard has to be able to leave.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.providers_open = false;
        }
    }
}

/// THE ROSTER, read straight off the fabric — no subscription, no history
/// (P1 §3.2). This is the whole point of the fold: the shell asks what is
/// available NOW and gets a true answer, including for a provider that
/// activated by having transcripts on disk and so never published a
/// lifecycle event at all.
///
/// `None` when nothing is activated: an empty chip would be noise, and a
/// chip claiming providers it cannot name would be a lie.
fn provider_chip(fabric: &Fabric) -> Option<status::ProviderChip> {
    let activated: Vec<dpl_fabric::ProviderCapability> = fabric
        .provider_roster()
        .into_iter()
        .filter(|p| p.activated)
        .collect();
    if activated.is_empty() {
        return None;
    }
    let names: Vec<&str> = activated.iter().map(|p| p.label.as_str()).collect();
    let tooltip = activated
        .iter()
        .map(|p| match &p.note {
            // The truthful limitation, never hidden behind the name.
            Some(note) => format!("{} — {note}", p.label),
            None => p.label.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(status::ProviderChip {
        summary: format!("AI · {}", names.join(", ")),
        tooltip,
    })
}

fn activity_provider_line(ev: &dpl_fabric::ProviderEvent) -> String {
    use dpl_fabric::ProviderEvent as E;
    match ev {
        E::Selected { provider } => format!("{provider} selected"),
        E::ProfileConfigured {
            provider,
            profile_key,
        } => format!("{provider} profile configured · {profile_key}"),
        E::ActivationRequested {
            provider, attempt, ..
        } => format!("{provider} connect requested · {attempt}"),
        E::ClientStarted {
            provider,
            client_version,
            ..
        } => format!("{provider} client started · {client_version}"),
        E::DeviceCodeReceived { provider, .. } => {
            format!("{provider} sign-in code received")
        }
        E::CodeCopied { provider, .. } => format!("{provider} sign-in code copied"),
        E::SystemBrowserDispatched { provider, .. } => {
            format!("{provider} sign-in sent to system browser")
        }
        E::AuthorizationCompleted { provider, .. } => {
            format!("{provider} authorization completed")
        }
        E::RuntimeStarted { provider, .. } => format!("{provider} runtime started"),
        E::VerificationRetryScheduled {
            provider,
            retry_index,
            max_retries,
            ..
        } => format!("{provider} verification retry {retry_index}/{max_retries}"),
        E::Ready { provider, .. } => format!("{provider} ready"),
        E::Failed {
            provider,
            stage,
            class,
            ..
        } => format!("{provider} failed · {} · {}", stage.label(), class.label()),
        E::Cancelled { provider, .. } => format!("{provider} activation cancelled"),
        E::Expired { provider, .. } => format!("{provider} activation expired"),
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
    /// THE ACTIVITY FEED (W8, signal-doctrine.md): capture + jobs + providers ears
    /// and a ring of the last N stamped lines — the operator SEES the
    /// nervous system firing, and it doubles as the debug surface.
    capture_rx: dpl_fabric::CaptureReceiver,
    jobs_rx: dpl_fabric::JobReceiver,
    providers_rx: dpl_fabric::ProviderReceiver,
    /// Coach lifecycle ear — opened at construction like the shell's own,
    /// not passed in: the coach publisher (the devtools pilot bridge)
    /// attaches after the shell exists, so no publish can precede this
    /// subscription. In production builds the domain is simply silent.
    coach_rx: dpl_fabric::CoachReceiver,
    /// Privileged-mode transitions and refusals — same ear pattern as coach.
    intent_rx: dpl_fabric::IntentReceiver,
    /// The determinism gate (optional until the app attaches it at boot).
    determinism: Option<dpl_intent::DeterminismGate>,
    /// Two-step UI arm for demanding strict — confirm spends the proof.
    strict_armed: bool,
    activity: std::collections::VecDeque<String>,
    /// Full-bleed: all chrome hidden, content owns the viewport (old `\` /
    /// Maximize2 behavior). Esc or the floating exit chip leaves.
    full_bleed: bool,
    /// The inspector overlay (PANEL zone): OPTIONAL — closed by default so
    /// the content well keeps the full width; open, it floats OVER the
    /// well's right edge (docs/shell.md, "rightside overlay the content").
    inspector_open: bool,
    /// The build stamp the footer shows (git hash · utc) — the running app
    /// itself proves which code it is.
    build: &'static str,
    /// The boot-rolled mood tinting the accordion's unfold strip ("on
    /// reload so we have variety of color") — fixed for the session, fresh
    /// every launch.
    tooltip_mood: usize,
    /// The session's seeded ink-bleed (V1): the hue pair and the dwell.
    bleed: providers::BleedRoll,
    /// Is the Providers roster popup open (V2)?
    providers_open: bool,
    /// The accordion's ONE open house (doctrine §5.2) — its subroutes
    /// unfold beneath its rail row; `None` = fully folded, icon-compact.
    open_house: Option<HubId>,
    /// After an open-house change, scroll the rail body until that house
    /// (and its unfold) intersects the visible clip (p4 cramped degrade).
    rail_reveal_pending: bool,
    /// Clip-intersected rect of the open house tile ∪ unfold after the
    /// last `rail()` paint — `None` when folded. Kittest contract (p4).
    last_open_rail_body: Option<Rect>,
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
        let providers_rx = fabric.subscribe_providers();
        Self::with_ears(hubs, fabric, capture_rx, jobs_rx, providers_rx)
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
        providers_rx: dpl_fabric::ProviderReceiver,
    ) -> Self {
        let route = hubs
            .first()
            .map(|h| h.spec().default_route())
            .unwrap_or(Route {
                hub: HubId("_none"),
                sub: SubId("_none"),
            });
        let events = fabric.subscribe_shell();
        let coach_rx = fabric.subscribe_coach();
        let intent_rx = fabric.subscribe_intent();
        Self {
            hubs,
            route,
            fabric,
            events,
            capture_rx,
            jobs_rx,
            providers_rx,
            coach_rx,
            intent_rx,
            determinism: None,
            strict_armed: false,
            activity: std::collections::VecDeque::new(),
            backdrop: dpl_backdrop::Backdrop::default(),
            nav_count: 0,
            signal_count: 0,
            full_bleed: false,
            inspector_open: false,
            build: "",
            tooltip_mood: dpl_backdrop::random_mood_index(),
            // The bleed's pair and dwell are drawn ONCE per launch, off the
            // same boot roll the tooltip mood uses, and pinned together by
            // `set_harness_mood` so a capture reproduces.
            bleed: providers::BleedRoll::from_seed(dpl_backdrop::random_mood_index() as u64),
            providers_open: false,
            open_house: None,
            rail_reveal_pending: false,
            last_open_rail_body: None,
            zones: Zones::default(),
            window_accessories: Vec::new(),
        }
    }

    /// Attach the session's determinism gate. The shell owns the UI
    /// second-acknowledgement channel; the app owns construction and the
    /// slash mint site. No constructor churn — same pattern as estate wires.
    pub fn attach_determinism(&mut self, gate: dpl_intent::DeterminismGate) {
        self.determinism = Some(gate);
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
        // The ink bleed is boot-randomized too, so it pins here or the
        // capture of a six-second wash is a different colour every run.
        self.bleed = providers::BleedRoll::from_seed(index as u64);
    }

    pub fn full_bleed(&self) -> bool {
        self.full_bleed
    }

    pub fn set_full_bleed(&mut self, on: bool) {
        self.full_bleed = on;
    }

    pub fn inspector_open(&self) -> bool {
        self.inspector_open
    }

    /// Harness access (`--inspector` pins the overlay open for proof shots).
    pub fn set_inspector_open(&mut self, on: bool) {
        self.inspector_open = on;
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
        self.rail_reveal_pending = id.is_some();
        if id.is_none() {
            self.last_open_rail_body = None;
        }
    }

    /// Clip-intersected open accordion body after the last rail paint
    /// (`None` when folded). Used by the p4 cramped-rail contract.
    pub fn last_open_rail_body(&self) -> Option<Rect> {
        self.last_open_rail_body
    }

    /// Tapping a house in the rail — THE accordion gesture: the open house
    /// folds (no navigation); any other house becomes the one open house
    /// (folding the previous implicitly) AND navigates to its default
    /// subroute, so one tap still goes somewhere (touch-first law).
    pub fn toggle_house(&mut self, id: HubId) {
        if self.open_house == Some(id) {
            self.open_house = None;
            self.rail_reveal_pending = false;
            self.last_open_rail_body = None;
        } else {
            self.open_house = Some(id);
            self.rail_reveal_pending = true;
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
        // THE ACTIVITY FEED drain (W8): capture + jobs + providers signals become
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
        while let Ok(st) = self.providers_rx.try_recv() {
            self.signal_count += 1;
            self.push_activity(st.ms, activity_provider_line(&st.ev));
        }
        while let Ok(st) = self.coach_rx.try_recv() {
            self.signal_count += 1;
            self.push_activity(st.ms, activity_coach_line(&st.ev));
        }
        while let Ok(st) = self.intent_rx.try_recv() {
            self.signal_count += 1;
            self.push_activity(st.ms, activity_intent_line(&st.ev));
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
        let (top, go_fullbleed, toggle_inspector) = self.top_bar(ui, &fl);
        if go_fullbleed {
            self.full_bleed = true;
        }
        if toggle_inspector {
            self.inspector_open = !self.inspector_open;
        }
        let (status_rect, actions) = self.status_bar(ui, &fl);
        // The click that OPENS the roster is itself "elsewhere" as far as an
        // area that did not exist yet is concerned, so the dismiss check has
        // to sit out the frame that opened it or the popup shuts instantly.
        let mut just_toggled = false;
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
                status::StatusAction::ToggleProviders => {
                    self.providers_open = !self.providers_open;
                    just_toggled = true;
                }
            }
        }
        self.providers_popup(ui, status_rect, just_toggled);
        let (rail, clicked) = self.rail(ui, &fl);
        if let Some(route) = clicked {
            self.navigate(route);
        }
        let content = self.central(ui);
        // The frame overlay: seams + diffusion framing the well, on top of
        // every plate edge — this is what makes the chrome float.
        cl::frame_overlay(ui.painter(), content, tone);
        // The inspector rides ABOVE the well when open; closed it reserves
        // nothing — the content just spanned to the root's right edge.
        let panel = if self.inspector_open {
            self.inspector(ui, &fl, content)
        } else {
            Rect::ZERO
        };
        self.zones = Zones {
            root,
            top,
            status: status_rect,
            rail,
            panel,
            content,
        };
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
    /// Returns the zone rect, whether the full-bleed toggle was clicked,
    /// and whether the inspector-overlay toggle was clicked.
    fn top_bar(&mut self, ui: &mut egui::Ui, fl: &cl::Fluid) -> (Rect, bool, bool) {
        let (hub_label, sub_label, tone) = self.breadcrumb();
        let inspector_open = self.inspector_open;
        let backdrop = &mut self.backdrop;
        let mut go_fullbleed = false;
        let mut toggle_inspector = false;
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
                        if icon_button(
                            ui,
                            cl::icon::WINDOW,
                            if inspector_open {
                                "close the inspector — the content keeps this space"
                            } else {
                                "inspector — floats over the content, takes no space"
                            },
                        ) {
                            toggle_inspector = true;
                        }
                        ui.add_space(cl::space::S2);
                        // The two loops; every click pulls a fresh mood.
                        backdrop.tiles(ui);
                    });
                });
            });
        (resp.response.rect, go_fullbleed, toggle_inspector)
    }

    // ── STATUS: the indicator grid — three lanes of registered cells ────
    fn status_bar(&self, ui: &mut egui::Ui, fl: &cl::Fluid) -> (Rect, Vec<status::StatusAction>) {
        let mut actions = Vec::new();
        let tone = self.active_tone();
        // The JobBoard's "working…" truth: one running job names itself
        // (with its latest-wins progress count when reported); several
        // running jobs just count themselves.
        let running: Vec<dpl_fabric::JobRow> = self
            .fabric
            .jobs()
            .into_iter()
            .filter(|r| r.running())
            .collect();
        let working = match running.as_slice() {
            [] => None,
            [one] => {
                let n = one.progress.load(std::sync::atomic::Ordering::Relaxed);
                Some(if n > 0 {
                    format!("working · {} · {n}", one.label)
                } else {
                    format!("working · {}", one.label)
                })
            }
            many => Some(format!("working · {} jobs", many.len())),
        };
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
                    working,
                    providers: provider_chip(&self.fabric),
                    bleed: self.bleed,
                };
                actions = status::grid(ui, &cx, &self.window_accessories);
            });
        (resp.response.rect, actions)
    }

    // ── RAIL: the accordion — tap a house, its subroutes unfold beneath
    // it and the houses below push down; ONE open at a time (doctrine
    // §5.2), folded is icon-compact. When the accordion exceeds the rail
    // zone height the BODY scrolls (p4) — chrome zones themselves do not.
    // ────────────────────────────────────────────────────────────────────
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
        let reveal_pending = self.rail_reveal_pending;
        let hubs = &self.hubs;
        let mut open_body: Option<Rect> = None;
        let mut reveal_done = false;
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
                // p4: cramped heights used to clip the unfold to nothing
                // (inverted pilot rects). Scroll the accordion body inside
                // the rail zone so the open house stays reachable.
                egui::ScrollArea::vertical()
                    .id_salt("dpl_rail_accordion")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
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
                                // Two independent facts, reported as two. They used to
                                // be one string (`active+open`), which no condition
                                // could ask a question of.
                                .value(if active { "active" } else { "inactive" })
                                .expanded(open),
                            );
                            if r.clicked() {
                                toggled = Some(spec.id);
                            }
                            let mut block = r.rect;
                            // The unfold: the section allocates its full height
                            // scaled by the factor and clip-reveals from the top —
                            // subroutes extend DOWN, the houses below push down.
                            if f > 0.001 {
                                let w = ui.available_width();
                                let gap = ui.spacing().item_spacing.y;
                                let rows = spec.subroutes.len() as f32;
                                let full_h = 8.0 + rows * 26.0 + (rows - 1.0).max(0.0) * gap;
                                let (rect, _) = ui
                                    .allocate_exact_size(egui::vec2(w, full_h * f), Sense::hover());
                                block = block.union(rect);
                                let mut child = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(Rect::from_min_size(
                                            rect.min,
                                            egui::vec2(w, full_h),
                                        ))
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
                                            PilotId::new("shell.rail.sub").child(&PilotKey::new(
                                                format!("{}/{}", spec.id.0, sub.id.0),
                                            )),
                                            egui_pilot_ai::protocol::Role::MenuItem,
                                            sub.label,
                                        )
                                        .value(if selected { "selected" } else { "available" })
                                        .selected(selected),
                                    );
                                    if response.clicked() {
                                        clicked = Some(Route {
                                            hub: spec.id,
                                            sub: sub.id,
                                        });
                                    }
                                }
                            }
                            if open {
                                let visible = block.intersect(ui.clip_rect());
                                open_body = Some(visible);
                                if reveal_pending {
                                    ui.scroll_to_rect(block, Some(Align::TOP));
                                    // Enough of the open body is on-screen to
                                    // click a house tile or the first subrows.
                                    if visible.height() >= fl.tap_min() * 0.5 {
                                        reveal_done = true;
                                    }
                                }
                            }
                            ui.add_space(cl::space::S1);
                        }
                    });
            });
        self.last_open_rail_body = open_body;
        if reveal_done {
            self.rail_reveal_pending = false;
        }
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

    // ── PANEL: the inspector — an OPTIONAL OVERLAY riding the well's
    // right edge (the operator's "rightside overlay the content"): the
    // content rect is untouched beneath it; closed it reserves nothing. ──
    fn inspector(&mut self, ui: &mut egui::Ui, fl: &cl::Fluid, content: Rect) -> Rect {
        let (hub_label, sub_label, tone) = self.breadcrumb();
        let w = fl.panel_width();
        let rect = Rect::from_min_max(egui::pos2(content.max.x - w, content.min.y), content.max);
        // The plate swallows pointer input so content beneath the overlay
        // never receives stray clicks (the fullbleed-exit precedent).
        ui.interact(rect, egui::Id::new("dpl_inspector_bg"), Sense::click());
        // The exposed left edge wears the well's hard-edged black
        // diffusion — the same scatter `frame_overlay` paints; painted
        // depth, never blur (doctrine).
        ui.painter().rect_filled(
            Rect::from_min_size(
                rect.min - egui::vec2(6.0, 0.0),
                egui::vec2(6.0, rect.height()),
            ),
            0,
            Color32::from_black_alpha(90),
        );
        ui.painter().rect_filled(rect, 0, cl::surface::RAISED);
        cl::sheen(ui.painter(), rect);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(12.0))
                .layout(Layout::top_down(Align::Min)),
        );
        child.set_clip_rect(rect);
        let mut close = false;
        {
            let ui = &mut child;
            ui.horizontal(|ui| {
                ui.label(RichText::new("INSPECTOR").small().color(cl::ink::SUBTLE));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, cl::icon::CLOSE, "close — the content keeps this space") {
                        close = true;
                    }
                });
            });
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
            self.determinism_card(ui);

            ui.add_space(cl::space::S3);
            ui.label(RichText::new("TOOLS").small().color(cl::ink::SUBTLE));
            ui.label(
                RichText::new("Per-hub tool docks mount here (P1).")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
        }
        if close {
            self.inspector_open = false;
        }
        rect
    }

    /// The UI second-acknowledgement channel for determinism mode.
    /// Synthetic (agent-injected) clicks publish a refusal and mint nothing.
    fn determinism_card(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("DETERMINISM").small().color(cl::ink::SUBTLE));
        let Some(gate) = self.determinism.clone() else {
            cl::card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("gate not attached")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            });
            return;
        };
        let mode = gate.mode();
        let cause = gate.strict_cause();
        let mode_label = match mode {
            dpl_intent::DeterminismMode::Behavioral => "behavioral",
            dpl_intent::DeterminismMode::Strict => "strict",
        };
        let cause_label = match cause {
            None => None,
            Some(dpl_intent::StrictCause::Operator(dpl_intent::IntentChannel::UiAcknowledged)) => {
                Some("ui")
            }
            Some(dpl_intent::StrictCause::Operator(dpl_intent::IntentChannel::SlashCommand)) => {
                Some("slash")
            }
            Some(dpl_intent::StrictCause::Evidence) => Some("evidence"),
        };
        let status = match cause_label {
            Some(cause) => format!("{mode_label} · {cause}"),
            None => mode_label.to_owned(),
        };

        let mut arm = false;
        let mut confirm = false;
        let mut cancel_arm = false;
        let mut deescalate = false;
        cl::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(status).monospace().color(cl::ink::BASE));
            match cause {
                Some(dpl_intent::StrictCause::Evidence) => {
                    ui.label(
                        RichText::new("locked — this run is evidence")
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                }
                Some(dpl_intent::StrictCause::Operator(_)) => {
                    let btn = ui.button("return to behavioral");
                    let btn = cl::tip(btn, "Clear operator-demanded strict.");
                    let mut meta = NodeMetadata::new(
                        PilotId::new("shell.determinism.behavioral"),
                        egui_pilot_ai::protocol::Role::Button,
                        "Return to behavioral",
                    );
                    meta.description = Some("Clear operator-demanded strict.".into());
                    ui.pilot_track(&btn, meta);
                    if btn.clicked() {
                        deescalate = true;
                    }
                }
                None if self.strict_armed => {
                    ui.label(
                        RichText::new("confirm to demand strict")
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    ui.horizontal(|ui| {
                        let yes = ui.button("confirm strict");
                        let yes = cl::tip(yes, "Second acknowledgement — demand strict mode.");
                        let mut meta = NodeMetadata::new(
                            PilotId::new("shell.determinism.confirm"),
                            egui_pilot_ai::protocol::Role::Button,
                            "Confirm strict",
                        );
                        meta.description =
                            Some("Second acknowledgement — demand strict mode.".into());
                        ui.pilot_track(&yes, meta);
                        if yes.clicked() {
                            confirm = true;
                        }
                        let no = ui.button("cancel");
                        if no.clicked() {
                            cancel_arm = true;
                        }
                    });
                }
                None => {
                    let btn = ui.button("arm strict");
                    let btn = cl::tip(btn, "Arm a second acknowledgement for strict mode.");
                    let mut meta = NodeMetadata::new(
                        PilotId::new("shell.determinism.arm"),
                        egui_pilot_ai::protocol::Role::Button,
                        "Arm strict",
                    );
                    meta.description = Some("Arm a second acknowledgement for strict mode.".into());
                    ui.pilot_track(&btn, meta);
                    if btn.clicked() {
                        arm = true;
                    }
                }
            }
        });

        let synthetic = egui_pilot_ai::input_is_synthetic(ui.ctx());
        if arm {
            if synthetic {
                self.fabric
                    .publish_intent(dpl_fabric::IntentEvent::IntentRefused {
                        action: "arm-strict".into(),
                        reason: "synthetic-input".into(),
                    });
            } else {
                self.strict_armed = true;
            }
        }
        if cancel_arm {
            self.strict_armed = false;
        }
        if confirm {
            self.strict_armed = false;
            if synthetic {
                self.fabric
                    .publish_intent(dpl_fabric::IntentEvent::IntentRefused {
                        action: "strict".into(),
                        reason: "synthetic-input".into(),
                    });
            } else {
                let channel = gate.escalate(dpl_intent::OperatorIntent::ui_acknowledged());
                self.fabric
                    .publish_intent(dpl_fabric::IntentEvent::DeterminismChanged {
                        mode: dpl_intent::DeterminismMode::Strict,
                        cause: Some(dpl_intent::StrictCause::Operator(channel)),
                    });
            }
        }
        if deescalate {
            if synthetic {
                self.fabric
                    .publish_intent(dpl_fabric::IntentEvent::IntentRefused {
                        action: "behavioral".into(),
                        reason: "synthetic-input".into(),
                    });
            } else {
                match gate.deescalate() {
                    dpl_intent::Deescalation::Cleared => {
                        self.fabric
                            .publish_intent(dpl_fabric::IntentEvent::DeterminismChanged {
                                mode: dpl_intent::DeterminismMode::Behavioral,
                                cause: None,
                            });
                    }
                    dpl_intent::Deescalation::RefusedEvidence => {
                        self.fabric
                            .publish_intent(dpl_fabric::IntentEvent::IntentRefused {
                                action: "behavioral".into(),
                                reason: "evidence-locked".into(),
                            });
                    }
                    dpl_intent::Deescalation::AlreadyBehavioral => {}
                }
            }
        }
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

    /// p4: at the app's minimum window the accordion used to clip the
    /// open unfold to an inverted unreachable rect. With five houses and
    /// a fat last unfold, the open body must still report a positive
    /// non-inverted clip intersection inside the rail zone.
    #[test]
    fn cramped_rail_keeps_the_open_house_reachable() {
        static SPEC_C: HubSpec = HubSpec {
            id: HubId("c"),
            label: "C",
            icon: "•",
            tone: Tone::Amber,
            subroutes: &[SubRoute {
                id: SubId("only"),
                label: "Only",
                blurb: "",
            }],
        };
        static SPEC_D: HubSpec = HubSpec {
            id: HubId("d"),
            label: "D",
            icon: "•",
            tone: Tone::Violet,
            subroutes: &[SubRoute {
                id: SubId("only"),
                label: "Only",
                blurb: "",
            }],
        };
        static FAT: HubSpec = HubSpec {
            id: HubId("fat"),
            label: "Fat",
            icon: "•",
            tone: Tone::Magenta,
            subroutes: &[
                SubRoute {
                    id: SubId("s1"),
                    label: "S1",
                    blurb: "",
                },
                SubRoute {
                    id: SubId("s2"),
                    label: "S2",
                    blurb: "",
                },
                SubRoute {
                    id: SubId("s3"),
                    label: "S3",
                    blurb: "",
                },
                SubRoute {
                    id: SubId("s4"),
                    label: "S4",
                    blurb: "",
                },
                SubRoute {
                    id: SubId("s5"),
                    label: "S5",
                    blurb: "",
                },
                SubRoute {
                    id: SubId("s6"),
                    label: "S6",
                    blurb: "",
                },
                SubRoute {
                    id: SubId("s7"),
                    label: "S7",
                    blurb: "",
                },
                SubRoute {
                    id: SubId("s8"),
                    label: "S8",
                    blurb: "",
                },
            ],
        };
        let size = egui::vec2(640.0, 480.0);
        let fabric = Fabric::new();
        let sh = Shell::new(
            vec![
                Box::new(Fake(&SPEC_A)),
                Box::new(Fake(&SPEC_B)),
                Box::new(Fake(&SPEC_C)),
                Box::new(Fake(&SPEC_D)),
                Box::new(Fake(&FAT)),
            ],
            fabric,
        );
        let mut harness = egui_kittest::Harness::builder()
            .with_size(size)
            .build_ui_state(|ui, sh: &mut Shell| sh.ui(ui), sh);
        harness.state_mut().set_open_house(Some(HubId("fat")));
        // Unfold animation (0.18s) + scroll reveal settle.
        harness.run_steps(120);
        let body = harness
            .state()
            .last_open_rail_body()
            .expect("open house must report a body rect");
        let rail = harness.state().zones().rail;
        assert!(
            body.is_positive() && body.height() > 1.0 && body.width() > 1.0,
            "open body must not be inverted/empty: {body:?}"
        );
        assert!(
            body.min.y <= body.max.y && body.min.x <= body.max.x,
            "open body must not be inverted: {body:?}"
        );
        let hit = body.intersect(rail);
        assert!(
            hit.height() >= 20.0,
            "open body must intersect the rail zone (body={body:?} rail={rail:?})"
        );
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
            assert_eq!(
                z.panel,
                Rect::ZERO,
                "the closed inspector must reserve no zone"
            );

            assert!(z.content.min.y >= z.top.max.y - 0.5);
            assert!(z.content.max.y <= z.status.min.y + 0.5);
            assert!(z.content.min.x >= z.rail.max.x - 0.5);
            assert!(
                eq(z.content.max.x, z.root.max.x),
                "content must own the full width to the root's right edge"
            );
        }
    }

    /// The PANEL zone is an OPTIONAL OVERLAY (the operator's "rightside
    /// overlay the content", docs/shell.md): closed it reserves nothing —
    /// pinned above — and open it floats OVER the well: the panel rect is
    /// token-wide, sits INSIDE the content rect, and the content still
    /// spans to the root's right edge (no reflow, no lost space).
    #[test]
    fn the_inspector_overlays_the_content_instead_of_docking() {
        let size = egui::vec2(1280.0, 800.0);
        let (mut sh, _rx) = shell();
        sh.set_inspector_open(true);
        let mut harness = egui_kittest::Harness::builder()
            .with_size(size)
            .build_ui_state(|ui, sh: &mut Shell| sh.ui(ui), sh);
        harness.run_steps(3);
        let z = harness.state().zones();
        let fl = cl::Fluid::of(Rect::from_min_size(egui::Pos2::ZERO, size));
        let eq = |a: f32, b: f32| (a - b).abs() < 0.75;
        assert!(
            eq(z.panel.width(), fl.panel_width()),
            "open panel width {} != panel_width token {}",
            z.panel.width(),
            fl.panel_width()
        );
        assert!(
            eq(z.content.max.x, z.root.max.x),
            "the open overlay must not shrink the content"
        );
        assert!(
            z.content.contains_rect(z.panel),
            "the panel must float INSIDE the well (content={:?} panel={:?})",
            z.content,
            z.panel
        );
    }

    fn capability(
        id: &'static str,
        label: &str,
        activated: bool,
        note: Option<&str>,
    ) -> dpl_fabric::ProviderCapability {
        dpl_fabric::ProviderCapability {
            id: dpl_fabric::ProviderId(id),
            label: label.to_owned(),
            activated,
            note: note.map(str::to_owned),
            capabilities: std::collections::BTreeSet::from([dpl_fabric::Capability::PageHistory]),
            presence: if activated {
                dpl_fabric::ProviderPresence::Ready
            } else {
                dpl_fabric::ProviderPresence::SignedOut
            },
        }
    }

    #[test]
    fn the_shell_names_the_activated_providers() {
        let fabric = Fabric::new();
        fabric.publish_provider_roster(vec![
            capability(
                "claude",
                "Claude Code",
                true,
                Some("local transcripts only"),
            ),
            capability("codex", "Codex", true, None),
            capability("gemini", "Gemini", false, Some("no qualified adapter")),
        ]);
        let chip = provider_chip(&fabric).expect("two providers are activated");
        assert_eq!(
            chip.summary, "AI · Claude Code, Codex",
            "the chip names what the PLATFORM can do — and never an unactivated provider"
        );
        assert!(
            chip.tooltip
                .contains("Claude Code — local transcripts only"),
            "the truthful limit rides with the name, not hidden behind it: {}",
            chip.tooltip
        );
        assert!(
            !chip.tooltip.contains("Gemini"),
            "an unactivated provider is absent entirely: {}",
            chip.tooltip
        );
    }

    #[test]
    fn the_shell_says_nothing_when_no_provider_is_activated() {
        let fabric = Fabric::new();
        assert!(
            provider_chip(&fabric).is_none(),
            "an empty roster paints no chip — silence, not an empty claim"
        );
        fabric.publish_provider_roster(vec![capability("gemini", "Gemini", false, None)]);
        assert!(
            provider_chip(&fabric).is_none(),
            "a registered-but-unactivated provider is not a capability"
        );
    }
}
