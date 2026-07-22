//! The shell — the persistent frame every hub lives inside.
//!
//! The operator's spec, implemented literally: five chrome plates — topbar,
//! left rail, right rail, footer, and the overlay seams that join them —
//! FRAME the content, which sits in a recessed well. Every plate wears the
//! sprite vocabulary: solid fill + top sheen (gradient tint), a crisp 2px
//! bevel at every chrome/well boundary, black diffusion scattered into the
//! well on all four sides. The topbar seam answers the active hub's tone.
//!
//! The left rail lists every registered hub; **clicking a hub routes to it
//! and auto-extends the rail beneath it, showing that hub's subcategories** —
//! only the active hub is extended. Every navigation publishes
//! `ShellEvent::RouteChanged` on the fabric.

use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::{Fabric, ShellEvent};
use dpl_hub::{Hub, HubId, Route, SubId};
use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense};

const RAIL_WIDTH: f32 = 232.0;
const INSPECTOR_WIDTH: f32 = 252.0;
const TOP_HEIGHT: f32 = 44.0;
const FOOT_HEIGHT: f32 = 30.0;

pub struct Shell {
    hubs: Vec<Box<dyn Hub>>,
    route: Route,
    fabric: Fabric,
    /// Navigations this session — the inspector shows it; proof the fabric fires.
    nav_count: u64,
}

impl Shell {
    /// The first registered hub's default route is home base. An empty
    /// registry degrades to a sentinel route the dispatcher renders as an
    /// honest "no hubs registered" — never a panic.
    pub fn new(hubs: Vec<Box<dyn Hub>>, fabric: Fabric) -> Self {
        let route = hubs
            .first()
            .map(|h| h.spec().default_route())
            .unwrap_or(Route { hub: HubId("_none"), sub: SubId("_none") });
        Self { hubs, route, fabric, nav_count: 0 }
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
            self.fabric.publish_shell(ShellEvent::RouteChanged { route });
        }
    }

    /// Clicking a hub in the rail: land on its default subroute.
    pub fn activate_hub(&mut self, id: HubId) {
        if let Some(spec) = self.hubs.iter().find(|h| h.spec().id == id).map(|h| h.spec()) {
            self.navigate(spec.default_route());
        }
    }

    /// One frame of the whole shell. Panel order matters (outermost first,
    /// CentralPanel last); the frame overlay paints after everything.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let tone = self.active_tone();
        self.top_bar(ui);
        self.status_bar(ui);
        let clicked = self.rail(ui);
        if let Some(route) = clicked {
            self.navigate(route);
        }
        self.inspector(ui);
        let content = self.central(ui);
        // The overlay: seams + diffusion framing the well. Painted last, on
        // top of every plate edge — this is what makes the chrome float.
        cl::frame_overlay(ui.painter(), content, tone);
    }

    // ── top bar: identity + breadcrumb, tone-tinted sheen ───────────────
    fn top_bar(&self, ui: &mut egui::Ui) {
        let (hub_label, sub_label, tone) = self.breadcrumb();
        egui::Panel::top("dpl_top")
            .exact_size(TOP_HEIGHT)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .inner_margin(egui::Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| {
                cl::sheen(ui.painter(), ui.max_rect().expand(8.0));
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("DevPULSE").strong().color(cl::ink::STRONG).size(15.0));
                    ui.label(RichText::new("LABS").color(tone.color()).size(15.0));
                    ui.add_space(cl::space::S4);
                    ui.label(RichText::new(hub_label).color(cl::ink::BASE));
                    ui.label(RichText::new("›").color(cl::ink::SUBTLE));
                    ui.label(RichText::new(sub_label).color(tone.color()));
                });
            });
    }

    // ── footer ──────────────────────────────────────────────────────────
    fn status_bar(&self, ui: &mut egui::Ui) {
        egui::Panel::bottom("dpl_status")
            .exact_size(FOOT_HEIGHT)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .inner_margin(egui::Margin::symmetric(14, 6)),
            )
            .show(ui, |ui| {
                cl::sheen(ui.painter(), ui.max_rect().expand(6.0));
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new(format!("route  {}/{}", self.route.hub, self.route.sub))
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Reserved: the launcher lands here (fabric plan §6).
                        ui.label(RichText::new("fabric · live").small().color(cl::ink::SUBTLE));
                    });
                });
            });
    }

    // ── left rail: hubs, active one auto-extended ───────────────────────
    fn rail(&self, ui: &mut egui::Ui) -> Option<Route> {
        let mut clicked: Option<Route> = None;
        egui::Panel::left("dpl_rail")
            .exact_size(RAIL_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .inner_margin(egui::Margin::symmetric(10, 12)),
            )
            .show(ui, |ui| {
                cl::sheen(ui.painter(), ui.max_rect().expand(10.0));
                for hub in &self.hubs {
                    let spec = hub.spec();
                    let active = spec.id == self.route.hub;
                    if rail_item(ui, spec.icon, spec.label, spec.tone, active) {
                        clicked = Some(spec.default_route());
                    }
                    // The auto-extension: ONLY the active hub shows its
                    // subcategories, indented beneath it.
                    if active {
                        for sub in spec.subroutes {
                            let selected = sub.id == self.route.sub;
                            if sub_item(ui, sub.label, spec.tone, selected) {
                                clicked = Some(Route { hub: spec.id, sub: sub.id });
                            }
                        }
                        ui.add_space(cl::space::S2);
                    }
                }
            });
        clicked
    }

    // ── right rail: the inspector ───────────────────────────────────────
    fn inspector(&self, ui: &mut egui::Ui) {
        let (hub_label, sub_label, tone) = self.breadcrumb();
        egui::Panel::right("dpl_inspector")
            .exact_size(INSPECTOR_WIDTH)
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
                            ui.label(RichText::new("broadcast·64").monospace().color(cl::ink::BASE));
                        });
                    });
                });

                ui.add_space(cl::space::S3);
                ui.label(RichText::new("TOOLS").small().color(cl::ink::SUBTLE));
                ui.label(
                    RichText::new("Per-hub tool docks mount here (P1).")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            });
    }

    // ── central: the recessed well, dispatch to the active hub ──────────
    fn central(&mut self, ui: &mut egui::Ui) -> Rect {
        let route = self.route;
        let idx = self.hubs.iter().position(|h| h.spec().id == route.hub);
        let resp = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::WELL)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| match idx {
                Some(i) => self.hubs[i].ui(ui, route),
                None => {
                    // Unknown → degrade, never crash (Law 4).
                    ui.label(
                        RichText::new(format!("no hub registered for \"{}\"", route.hub))
                            .color(cl::ink::MUTED),
                    );
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
                let sub = spec.subroute(self.route.sub).map(|s| s.label).unwrap_or("—");
                (spec.label, sub, spec.tone)
            })
            .unwrap_or(("—", "—", Tone::Cyan))
    }
}

/// A hub row as a sprite: crisp wash when active with a 1px lit top edge and
/// 1px dark bottom edge (the bevel), tone accent bar on the left, hairline
/// wash on hover. Hand-painted for pixel control.
fn rail_item(ui: &mut egui::Ui, icon: &str, label: &str, tone: Tone, active: bool) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 34.0), Sense::click());
    let painter = ui.painter();

    if active {
        painter.rect_filled(rect, CornerRadius::same(cl::RADIUS_SM), tone.wash());
        // Sprite bevel: lit top, dark bottom — one pixel each, no feathered halo.
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
        painter.rect_filled(rect, CornerRadius::same(cl::RADIUS_SM), cl::surface::OVERLAY);
    }

    let ink = if active { cl::ink::STRONG } else { cl::ink::BASE };
    painter.text(
        egui::pos2(rect.min.x + 14.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(15.0),
        if active { tone.color() } else { cl::ink::MUTED },
    );
    painter.text(
        egui::pos2(rect.min.x + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.5),
        ink,
    );
    response.clicked()
}

/// A subcategory row under the active hub — indented, quieter, tone dot when
/// selected.
fn sub_item(ui: &mut egui::Ui, label: &str, tone: Tone, selected: bool) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::click());
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
        painter.circle_filled(egui::pos2(rect.min.x + 26.0, rect.center().y), 2.5, tone.color());
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(cl::RADIUS_SM), cl::surface::OVERLAY);
    }

    painter.text(
        egui::pos2(rect.min.x + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        if selected { tone.color() } else { cl::ink::MUTED },
    );
    response.clicked()
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
            SubRoute { id: SubId("one"), label: "One", blurb: "" },
            SubRoute { id: SubId("two"), label: "Two", blurb: "" },
        ],
    };
    static SPEC_B: HubSpec = HubSpec {
        id: HubId("b"),
        label: "B",
        icon: "•",
        tone: Tone::Lime,
        subroutes: &[SubRoute { id: SubId("only"), label: "Only", blurb: "" }],
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
        (Shell::new(vec![Box::new(Fake(&SPEC_A)), Box::new(Fake(&SPEC_B))], fabric), rx)
    }

    #[test]
    fn boots_on_the_first_hubs_default_subroute() {
        let (shell, _rx) = shell();
        assert_eq!(shell.route(), Route { hub: HubId("a"), sub: SubId("one") });
    }

    #[test]
    fn activating_a_hub_lands_on_its_default_and_publishes() {
        let (mut shell, mut rx) = shell();
        shell.activate_hub(HubId("b"));
        let expected = Route { hub: HubId("b"), sub: SubId("only") };
        assert_eq!(shell.route(), expected);
        assert_eq!(rx.try_recv().unwrap(), ShellEvent::RouteChanged { route: expected });
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
}
