//! The contract every hub signs. Nothing else in the workspace defines identity.
//!
//! Law 4 (registry over enumeration): the shell renders whatever hubs are
//! registered and never names them. Law 5 (no bare strings): `HubId`/`SubId`
//! are newtypes declared ONCE per hub as consts — call sites use the const,
//! and the compiler owns identity from there.

use dpl_coldlight::Tone;

/// A hub's identity. Declared exactly once, in the hub's own crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HubId(pub &'static str);

/// A subroute's identity within a hub.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubId(pub &'static str);

impl std::fmt::Display for HubId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::fmt::Display for SubId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Where the app is. Copy — routes are values, not references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub hub: HubId,
    pub sub: SubId,
}

/// One subcategory the rail shows when its hub is active.
#[derive(Debug, Clone, Copy)]
pub struct SubRoute {
    pub id: SubId,
    pub label: &'static str,
    /// One line of purpose, shown on the hub's surface.
    pub blurb: &'static str,
}

/// Everything the shell needs to render a hub without knowing it.
#[derive(Debug, Clone, Copy)]
pub struct HubSpec {
    pub id: HubId,
    pub label: &'static str,
    /// Text glyph for the rail until the sprite atlas lands (P1).
    pub icon: &'static str,
    pub tone: Tone,
    pub subroutes: &'static [SubRoute],
}

impl HubSpec {
    /// Where clicking this hub lands: its first subroute.
    pub fn default_route(&self) -> Route {
        Route {
            hub: self.id,
            sub: self.subroutes.first().map(|s| s.id).unwrap_or(SubId("_")),
        }
    }

    pub fn subroute(&self, id: SubId) -> Option<&'static SubRoute> {
        self.subroutes.iter().find(|s| s.id == id)
    }
}

/// A pluggable member of the shell. Register in `app/main.rs` — one line.
pub trait Hub {
    fn spec(&self) -> &'static HubSpec;
    /// Render this hub's surface for the given route. `route.hub` is
    /// guaranteed to equal `spec().id` by the shell's dispatch.
    fn ui(&mut self, ui: &mut egui::Ui, route: Route);
    /// The library came back after being unplugged (ST2b): save again what
    /// a refused save kept in memory. Most hubs keep nothing.
    fn library_back(&mut self) {}
}

/// P0-ONLY shared surface. Every hub renders through this until its real
/// surface lands, so the placeholder theme lives in ONE place instead of five.
/// Delete this module hub-by-hub as real surfaces arrive — never extend it.
pub mod placeholder {
    use super::{HubSpec, Route};
    use dpl_coldlight as cl;
    use egui::RichText;

    /// Returns the route of a TAPPED card (the NAVIGATE seam) — the hub
    /// publishes `ShellEvent::NavigateRequest` with it; the shell drains
    /// and navigates. What looks tappable now IS (frictionless law).
    pub fn surface(ui: &mut egui::Ui, spec: &HubSpec, route: Route, intro: &str) -> Option<Route> {
        let mut go: Option<Route> = None;
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(RichText::new(spec.icon).size(22.0).color(spec.tone.color()));
            ui.label(RichText::new(spec.label).heading().color(cl::ink::STRONG));
        });
        ui.label(RichText::new(intro).color(cl::ink::MUTED));
        ui.add_space(cl::space::S4);

        for sub in spec.subroutes {
            let active = sub.id == route.sub;
            // Doctrine: the card body is CLAY (it holds); the tappable
            // affordance is the hover's tone edge + the tip.
            let body = |ui: &mut egui::Ui| {
                ui.set_width(ui.available_width().min(560.0));
                ui.horizontal(|ui| {
                    if active {
                        cl::tone_dot(ui, spec.tone);
                    }
                    ui.label(RichText::new(sub.label).strong().color(if active {
                        cl::ink::STRONG
                    } else {
                        cl::ink::BASE
                    }));
                });
                ui.label(RichText::new(sub.blurb).small().color(cl::ink::MUTED));
            };
            let resp = if active {
                cl::scatter_frame(spec.tone).show(ui, body)
            } else {
                cl::clay().show(ui, body)
            };
            if active {
                // The gradient tint: painted over the card's top band.
                cl::gradient_tint(ui.painter(), resp.response.rect, spec.tone);
            } else {
                let click = cl::tip(
                    resp.response.interact(egui::Sense::click()),
                    &format!("Go to {}", sub.label),
                );
                if click.hovered() {
                    ui.painter().rect_stroke(
                        click.rect,
                        egui::CornerRadius::same(cl::RADIUS_LG),
                        egui::Stroke::new(2.0, spec.tone.color()),
                        egui::StrokeKind::Inside,
                    );
                }
                if click.clicked() {
                    go = Some(Route {
                        hub: spec.id,
                        sub: sub.id,
                    });
                }
            }
        }
        go
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static SPEC: HubSpec = HubSpec {
        id: HubId("t"),
        label: "T",
        icon: "•",
        tone: Tone::Cyan,
        subroutes: &[
            SubRoute {
                id: SubId("a"),
                label: "A",
                blurb: "first",
            },
            SubRoute {
                id: SubId("b"),
                label: "B",
                blurb: "second",
            },
        ],
    };

    #[test]
    fn default_route_is_the_first_subroute() {
        assert_eq!(
            SPEC.default_route(),
            Route {
                hub: HubId("t"),
                sub: SubId("a")
            }
        );
    }

    #[test]
    fn subroute_lookup_degrades_to_none_not_panic() {
        assert!(SPEC.subroute(SubId("missing")).is_none());
        assert_eq!(SPEC.subroute(SubId("b")).unwrap().label, "B");
    }
}
