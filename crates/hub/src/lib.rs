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
}

/// P0-ONLY shared surface. Every hub renders through this until its real
/// surface lands, so the placeholder look lives in ONE place instead of five.
/// Delete this module hub-by-hub as real surfaces arrive — never extend it.
pub mod placeholder {
    use super::{HubSpec, Route};
    use dpl_coldlight as cl;
    use egui::RichText;

    pub fn surface(ui: &mut egui::Ui, spec: &HubSpec, route: Route, intro: &str) {
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(RichText::new(spec.icon).size(22.0).color(spec.tone.color()));
            ui.label(RichText::new(spec.label).heading().color(cl::ink::STRONG));
        });
        ui.label(RichText::new(intro).color(cl::ink::MUTED));
        ui.add_space(cl::space::S4);

        for sub in spec.subroutes {
            let active = sub.id == route.sub;
            let frame = if active { cl::scatter_frame(spec.tone) } else { cl::card_frame() };
            let resp = frame.show(ui, |ui| {
                ui.set_width(ui.available_width().min(560.0));
                ui.horizontal(|ui| {
                    if active {
                        cl::tone_dot(ui, spec.tone);
                    }
                    ui.label(
                        RichText::new(sub.label)
                            .strong()
                            .color(if active { cl::ink::STRONG } else { cl::ink::BASE }),
                    );
                });
                ui.label(RichText::new(sub.blurb).small().color(cl::ink::MUTED));
            });
            if active {
                // The gradient tint: painted over the card's top band.
                cl::gradient_tint(ui.painter(), resp.response.rect, spec.tone);
            }
        }
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
            SubRoute { id: SubId("a"), label: "A", blurb: "first" },
            SubRoute { id: SubId("b"), label: "B", blurb: "second" },
        ],
    };

    #[test]
    fn default_route_is_the_first_subroute() {
        assert_eq!(SPEC.default_route(), Route { hub: HubId("t"), sub: SubId("a") });
    }

    #[test]
    fn subroute_lookup_degrades_to_none_not_panic() {
        assert!(SPEC.subroute(SubId("missing")).is_none());
        assert_eq!(SPEC.subroute(SubId("b")).unwrap().label, "B");
    }
}
