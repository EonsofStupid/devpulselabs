//! The PROVIDERS control (V1) — the operator's way into the frontier AI on
//! this machine, and the origin of the ink-bleed preset.
//!
//! > "I click on what will be a neat optimized bleed effect that as the mouse
//! > just overs ever so slowly like ink to paper think of it as a passive
//! > experience not meant to be instant but literally over the coarse of 5.6
//! > to 6.3 seconds before the full color takes over the icon and icon is
//! > brain and Providers and just equal at the height of P to the left we
//! > have a small circle with ai inside."
//!
//! Every design value here is AUTHORED and generated — the dwell window and
//! the easing curve from `tokens/motion.json`, the hue pool from
//! `tokens/presets.json` cross-checked against `tokens/hues.json`, the brain
//! from `tokens/icons.json` over the vendored Lucide pool. Nothing in this
//! file is a magic number that a designer cannot reach.
//!
//! What it does NOT do is open anything: the popup is V2, and the click
//! lands with the surface it opens.

use dpl_coldlight::{self as cl};
use egui::{Color32, Response, Sense, Ui};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotKey, UiPilotExt};

/// The bleed control, its seeded roll and its authored id all live in
/// ColdLight now — the chat rail needs the SAME control, and two copies of a
/// six-second animation is two places for it to drift.
pub use cl::patterns::{BleedRoll, BLEED};

/// What the control paints, once the shell has read the roster.
pub struct ProvidersControl<'a> {
    /// `Codex, Claude Code` — the activated providers, named.
    pub names: &'a str,
    /// Each provider's truthful limit, one per line.
    pub tooltip: &'a str,
    pub roll: BleedRoll,
}

impl ProvidersControl<'_> {
    /// Paint the control and return its response.
    ///
    /// The painting is `cl::patterns::bleed_control`; what stays here is the
    /// SHELL's framing of it — the tooltip naming each provider's truthful
    /// limit, which the chat rail does not want.
    pub fn show(&self, ui: &mut Ui) -> Response {
        let resp = cl::patterns::bleed_control(
            ui,
            // The one stable id this control has always used. A ui-derived
            // id is new every frame and egui snaps an unseen id straight to
            // its target, so the bleed would never play.
            egui::Id::new("dpl_providers_bleed"),
            "Providers",
            self.roll,
            None,
        );
        resp.on_hover_text(self.tooltip)
    }
}

/// The roster popup the control opens (V2).
///
/// Thumbnails for the frontier AI, each carrying its live state from the
/// registry — never a static list. A provider is shown whether or not it is
/// usable, because "Gemini is not installed" is an answer the operator wants
/// and an empty grid is not.
///
/// The tooltip is deliberately its own hoverable surface: V3 promotes it into
/// the provider modal, so it has to be a thing the pointer can reach rather
/// than a transient hint.
pub struct ProvidersPopup<'a> {
    pub providers: &'a [dpl_fabric::ProviderCapability],
}

/// What the popup asks the shell to do. The popup paints; the shell acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAction {
    /// Open this provider's full summary — V3's modal.
    Inspect(dpl_fabric::ProviderId),
}

/// The dot colour for each state. Green/amber/grey is the same vocabulary the
/// mock uses, resolved through the authored pool rather than hex literals.
fn presence_tone(presence: dpl_fabric::ProviderPresence) -> Color32 {
    use dpl_fabric::ProviderPresence as P;
    let hue = match presence {
        P::Ready => "viridian",
        P::SigningIn | P::SignedOut => "amber",
        P::NotInstalled | P::Unavailable => return cl::ink::SUBTLE,
    };
    // Through the authored ladder, never a hex literal in a widget.
    cl::ramp(hue)
        .zip(cl::step_index(cl::BLEED_STEP))
        .map_or(cl::ink::SUBTLE, |(ramp, step)| ramp.steps[step])
}

impl ProvidersPopup<'_> {
    pub fn show(&self, ui: &mut Ui) -> Option<PopupAction> {
        let mut action = None;
        ui.label(
            egui::RichText::new("FRONTIER AI ON THIS MACHINE")
                .size(10.5)
                .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S3);
        for provider in self.providers {
            let card = ui.push_id(provider.id.0, |ui| self.card(ui, provider));
            if let Some(found) = card.inner {
                action = Some(found);
            }
            ui.add_space(cl::space::S2);
        }
        if self.providers.is_empty() {
            ui.label(
                egui::RichText::new("No providers registered.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
        }
        action
    }

    fn card(&self, ui: &mut Ui, provider: &dpl_fabric::ProviderCapability) -> Option<PopupAction> {
        let tone = presence_tone(provider.presence);
        let response = egui::Frame::new()
            .fill(cl::surface::RAISED)
            .corner_radius(egui::CornerRadius::same(cl::RADIUS_SM))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(egui::vec2(7.0, 7.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 3.5, tone);
                    ui.label(
                        egui::RichText::new(&provider.label)
                            .size(13.0)
                            .color(cl::ink::STRONG),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(provider.presence.label())
                                .size(11.0)
                                .color(tone),
                        );
                    });
                });
                // What it can DO — the capability set, not a boolean. Absent
                // rather than empty when the provider cannot do anything yet.
                let caps = capability_line(provider);
                if !caps.is_empty() {
                    ui.label(egui::RichText::new(caps).size(10.5).color(cl::ink::SUBTLE));
                }
            })
            .response
            .interact(Sense::click());

        let tip = provider
            .note
            .clone()
            .unwrap_or_else(|| match provider.presence {
                dpl_fabric::ProviderPresence::Ready => {
                    format!("{} is signed in on this machine.", provider.label)
                }
                _ => provider.presence.label().to_owned(),
            });
        let response = response.on_hover_text(format!("{tip}\n\nClick for the full summary."));

        ui.pilot_track(
            &response,
            NodeMetadata::new(
                crate::status::PROVIDERS_PILOT_ID.child(&PilotKey::new(provider.id.0)),
                Role::Button,
                provider.label.clone(),
            )
            .value(provider.presence.label()),
        );
        response
            .clicked()
            .then_some(PopupAction::Inspect(provider.id))
    }
}

/// Days from the civil calendar to the Unix epoch (Howard Hinnant's
/// `days_from_civil`). The workspace has no date crate and this is the whole
/// of the arithmetic the tenure figure needs — pulling in a dependency to
/// subtract two dates would cost more than it explains.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parse `YYYY-MM-DD` into seconds since the Unix epoch. `None` on anything
/// malformed — a tenure the app cannot compute is simply not shown, never a
/// wrong number presented with confidence.
fn epoch_seconds(iso: &str) -> Option<i64> {
    let mut parts = iso.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d) * 86_400)
}

/// "10y 7mo · 3,887 days · 14:22:09" — the operator's live figure.
///
/// Years and months are the readable part; the day count and clock are what
/// make it visibly ALIVE, which is the point of asking for it.
pub fn tenure(since: &str, now_unix_seconds: i64) -> Option<String> {
    let start = epoch_seconds(since)?;
    let elapsed = now_unix_seconds.checked_sub(start)?;
    if elapsed < 0 {
        return None;
    }
    let days = elapsed / 86_400;
    let years = days / 365;
    let months = (days - years * 365) / 30;
    let (h, m, s) = (elapsed / 3_600 % 24, elapsed / 60 % 60, elapsed % 60);
    Some(format!(
        "{years}y {months}mo · {} days · {h:02}:{m:02}:{s:02}",
        thousands(days)
    ))
}

fn thousands(mut n: i64) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let mut parts = Vec::new();
    while n >= 1_000 {
        parts.push(format!("{:03}", n % 1_000));
        n /= 1_000;
    }
    parts.push(n.to_string());
    parts.reverse();
    parts.join(",")
}

fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// The provider modal (V3) — a card promoted into its full summary.
///
/// Returns `true` while it is still open.
pub fn provider_modal(
    ctx: &egui::Context,
    provider: &dpl_fabric::ProviderCapability,
    summary: &str,
) -> bool {
    let mut open = true;
    let tone = presence_tone(provider.presence);
    egui::Modal::new(egui::Id::new("dpl_provider_modal")).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 4.5, tone);
            ui.label(
                egui::RichText::new(&provider.label)
                    .size(18.0)
                    .color(cl::ink::STRONG),
            );
        });
        ui.label(
            egui::RichText::new(provider.presence.label())
                .size(11.5)
                .color(tone),
        );
        ui.add_space(cl::space::S3);
        if !summary.is_empty() {
            ui.label(egui::RichText::new(summary).size(12.5).color(cl::ink::BASE));
        }
        if let Some(note) = &provider.note {
            ui.add_space(cl::space::S2);
            ui.label(egui::RichText::new(note).size(11.5).color(cl::ink::SUBTLE));
        }

        // THE LIVE FIGURE. It ticks, so the frame has to keep coming —
        // once a second is enough and costs nothing when the modal is shut.
        ui.add_space(cl::space::S3);
        if let Some(tenure) = tenure(&provider.since, now_unix_seconds()) {
            ui.label(
                egui::RichText::new(tenure)
                    .monospace()
                    .size(13.0)
                    .color(cl::ink::STRONG),
            );
            ui.label(
                egui::RichText::new("time in operation, live")
                    .size(10.5)
                    .color(cl::ink::SUBTLE),
            );
            ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        }

        ui.add_space(cl::space::S3);
        let caps = capability_line(provider);
        ui.label(
            egui::RichText::new(if caps.is_empty() {
                "Capabilities —".to_owned()
            } else {
                format!("Capabilities  {caps}")
            })
            .size(11.5)
            .color(cl::ink::MUTED),
        );

        ui.add_space(cl::space::S4);
        let close = ui.button("Close");
        ui.pilot_track(
            &close,
            NodeMetadata::new(
                crate::status::PROVIDERS_PILOT_ID.child(&PilotKey::new("modal.close")),
                Role::Button,
                "Close provider summary",
            ),
        );
        if close.clicked() {
            open = false;
        }
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        open = false;
    }
    open
}

/// The capability set as one readable line, in the stored vocabulary.
fn capability_line(provider: &dpl_fabric::ProviderCapability) -> String {
    if !provider.activated || provider.capabilities.is_empty() {
        return String::new();
    }
    provider
        .capabilities
        .iter()
        .map(|c| c.slug())
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_roll_is_seeded_and_lands_inside_the_authored_window() {
        for seed in [0u64, 1, 42, 9_999] {
            let r = BleedRoll::from_seed(seed);
            assert_eq!(
                (r.a, r.b, r.dwell.to_bits()),
                (
                    BleedRoll::from_seed(seed).a,
                    BleedRoll::from_seed(seed).b,
                    BleedRoll::from_seed(seed).dwell.to_bits()
                ),
                "the same seed must draw the same bleed — proof shots depend on it"
            );
            assert!(
                (cl::motion::DUR_DWELL_MIN..=cl::motion::DUR_DWELL_MAX).contains(&r.dwell),
                "dwell {} escaped the authored window",
                r.dwell
            );
            assert_ne!(r.a, r.b, "a two-colour bleed needs two colours");
        }
    }

    fn cap(
        id: &'static str,
        presence: dpl_fabric::ProviderPresence,
        activated: bool,
    ) -> dpl_fabric::ProviderCapability {
        dpl_fabric::ProviderCapability {
            id: dpl_fabric::ProviderId(id),
            label: id.to_owned(),
            activated,
            note: None,
            capabilities: std::collections::BTreeSet::from([dpl_fabric::Capability::PageHistory]),
            summary: String::new(),
            since: "2021-01-28".into(),
            presence,
        }
    }

    #[test]
    fn the_tenure_counts_from_the_authored_date() {
        // 2015-12-11 -> 2025-12-11 is ten years to the second.
        let start = epoch_seconds("2015-12-11").expect("a real date parses");
        let ten_years = start + 10 * 365 * 86_400 + 3 * 86_400; // + leap days
        let figure = tenure("2015-12-11", ten_years).expect("tenure computes");
        assert!(figure.starts_with("10y"), "got {figure}");
        assert!(
            figure.contains("3,653 days"),
            "the day count reads: {figure}"
        );
        assert!(figure.contains("00:00:00"), "the clock reads: {figure}");
    }

    #[test]
    fn an_unparseable_date_shows_nothing_rather_than_a_wrong_number() {
        assert!(tenure("", 0).is_none());
        assert!(tenure("not-a-date", 0).is_none());
        assert!(tenure("2015-13-01", 1).is_none(), "month 13 is not a month");
        // A start in the future is not negative tenure, it is no tenure.
        assert!(tenure("2999-01-01", 0).is_none());
    }

    #[test]
    fn the_tenure_ticks() {
        // The figure has to CHANGE second to second — that is what makes it
        // live rather than a static age.
        let a = tenure("2015-12-11", 1_800_000_000).expect("tenure");
        let b = tenure("2015-12-11", 1_800_000_001).expect("tenure");
        assert_ne!(a, b);
    }

    #[test]
    fn the_popup_keeps_not_installed_and_signed_out_apart() {
        use dpl_fabric::ProviderPresence as P;
        // Different problems, different fixes. A shared grey dot would tell
        // the operator to sign in to something they never installed.
        assert_ne!(presence_tone(P::SignedOut), presence_tone(P::NotInstalled));
        assert_eq!(P::SignedOut.label(), "signed out");
        assert_eq!(P::NotInstalled.label(), "not installed");
        assert_ne!(presence_tone(P::Ready), presence_tone(P::SignedOut));
    }

    #[test]
    fn an_unusable_provider_advertises_no_capabilities() {
        use dpl_fabric::ProviderPresence as P;
        // A qualified set on a provider that is not activated would be a
        // promise the platform cannot keep.
        assert!(capability_line(&cap("gemini", P::NotInstalled, false)).is_empty());
        assert_eq!(
            capability_line(&cap("claude", P::Ready, true)),
            "page-history",
            "an activated provider names what it can do, in the stored vocabulary"
        );
    }

    #[test]
    fn the_dwell_is_a_passive_experience_not_a_transition() {
        // The whole point of the tier: nobody should be able to quietly
        // retune this to feel "snappy".
        let r = BleedRoll::from_seed(7);
        assert!(
            r.dwell > 5.0,
            "the bleed is passive by design, got {}s",
            r.dwell
        );
    }
}
