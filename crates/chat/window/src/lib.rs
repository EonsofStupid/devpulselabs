//! Primary native Chat-window composition.
//!
//! This crate paints one viewport and emits typed intents. It does not create
//! windows, publish signals, navigate the main app, scan stores, or launch a
//! provider process.

use dpl_chat_context::RouteContextState;
use dpl_chat_core::{ActivationState, ProviderAccent, ProviderId, SessionState};
use dpl_chat_runtime::{ChatRuntime, ProviderView, RegistryError};
use dpl_chat_store::{HistoryCatalog, HistoryIndex, IndexedSession};
use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::AppWindowKey;
use dpl_hub::Route;
use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, Stroke, UiBuilder};
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    NodeMetadata, PilotId, PilotKey, UiPilotExt,
};
use std::collections::HashMap;

pub const WINDOW_KEY: AppWindowKey = AppWindowKey("chat.primary");
pub const LAUNCHER_ID: PilotId = PilotId::new("shell.window.chat");
pub const ROUTE_ID: PilotId = PilotId::new("chat.header.route");
pub const SHOW_MAIN_ID: PilotId = PilotId::new("chat.header.show-main");
pub const QUIT_ID: PilotId = PilotId::new("chat.header.quit");
pub const PROVIDERS_ID: PilotId = PilotId::new("chat.providers");
pub const SESSIONS_ID: PilotId = PilotId::new("chat.sessions");
pub const ROUTE_LAUNCHER_ID: PilotId = PilotId::new("chat.route-launcher");
pub const WORKSPACE_ID: PilotId = PilotId::new("chat.workspace.new-session");
pub const COMPOSER_ID: PilotId = PilotId::new("chat.composer");
pub const SEND_ID: PilotId = PilotId::new("chat.composer.send");
pub const ACTIVITY_ID: PilotId = PilotId::new("chat.activity");

pub fn viewport_key() -> PilotViewportKey {
    PilotViewportKey::new(WINDOW_KEY.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Composition {
    Companion,
    Standard,
    Wide,
}

impl Composition {
    pub fn for_width(width: f32) -> Self {
        if width >= 1180.0 {
            Self::Wide
        } else if width >= 760.0 {
            Self::Standard
        } else {
            Self::Companion
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatWindowIntent {
    SelectProvider(ProviderId),
    UpdateDraft {
        provider: ProviderId,
        value: String,
    },
    /// The lite route launcher was tapped: ask the host to open a main-app
    /// route. Chat never renders the route itself — it only requests it.
    NavigateTo(Route),
    ShowMain,
    QuitApplication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostIntent {
    /// Publish a `NavigateRequest` on the shell fabric; the shell drains it
    /// and navigates the MAIN window. Chat stays "portable lite".
    NavigateTo(Route),
    ShowMain,
    QuitApplication,
}

/// One provider's bounded, read-only history index (or the read error), taken
/// once from the catalog at construction. The peer window never mutates
/// provider stores — it only displays what the bounded reader surfaced.
#[derive(Debug, Clone)]
struct ProviderHistory {
    index: Option<HistoryIndex>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatWindow {
    runtime: ChatRuntime,
    route_context: RouteContextState,
    /// Bounded read-only session indexes, keyed by provider, read once from
    /// the catalog at construction.
    histories: HashMap<ProviderId, ProviderHistory>,
    /// The app's navigable routes for the lite launcher: `(label, route)`.
    routes: Vec<(String, Route)>,
}

impl ChatWindow {
    pub fn new(
        runtime: ChatRuntime,
        initial_route: Route,
        catalog: HistoryCatalog,
        routes: Vec<(String, Route)>,
    ) -> Self {
        let mut route_context = RouteContextState::default();
        route_context.observe(initial_route);
        // Read the bounded index once per provider that owns a source. This is
        // the only history read the peer window performs; it is read-only and
        // never writes a provider store.
        let mut histories = HashMap::new();
        for view in runtime.provider_views() {
            let provider = view.registration.manifest.descriptor.id;
            if !catalog.has_source(provider) {
                continue;
            }
            let entry = match catalog.list(provider, None) {
                Ok(index) => ProviderHistory {
                    index: Some(index),
                    error: None,
                },
                Err(error) => ProviderHistory {
                    index: None,
                    error: Some(error.to_string()),
                },
            };
            histories.insert(provider, entry);
        }
        Self {
            runtime,
            route_context,
            histories,
            routes,
        }
    }

    /// The bounded read-only session index for a provider, if a source exists
    /// and it read successfully. Read-only accessor for the sessions panel and
    /// its tests.
    pub fn session_history(&self, provider: ProviderId) -> Option<&HistoryIndex> {
        self.histories
            .get(&provider)
            .and_then(|history| history.index.as_ref())
    }

    pub fn observe_route(&mut self, route: Route) {
        self.route_context.observe(route);
    }

    pub fn selected_provider_id(&self) -> Option<ProviderId> {
        self.runtime.selected_provider_id()
    }

    pub fn apply(&mut self, intent: ChatWindowIntent) -> Result<Option<HostIntent>, RegistryError> {
        match intent {
            ChatWindowIntent::SelectProvider(provider) => {
                self.runtime.select_provider(provider)?;
                Ok(None)
            }
            ChatWindowIntent::UpdateDraft { provider, value } => {
                self.runtime.set_draft(provider, value)?;
                Ok(None)
            }
            ChatWindowIntent::NavigateTo(route) => Ok(Some(HostIntent::NavigateTo(route))),
            ChatWindowIntent::ShowMain => Ok(Some(HostIntent::ShowMain)),
            ChatWindowIntent::QuitApplication => Ok(Some(HostIntent::QuitApplication)),
        }
    }

    pub fn ui(&self, ui: &mut egui::Ui) -> Vec<ChatWindowIntent> {
        let mut intents = Vec::new();
        let root = ui.max_rect();
        paint_ember_field(ui.painter(), root);

        let composition = Composition::for_width(root.width());
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(root.shrink(12.0))
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                self.header(ui, composition, &mut intents);
                ui.add_space(cl::space::S3);

                let activity_height = 32.0;
                let composer_height = if composition == Composition::Companion {
                    118.0
                } else {
                    104.0
                };
                let workspace_height =
                    (ui.available_height() - composer_height - activity_height - 20.0).max(220.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), workspace_height),
                    Layout::top_down(Align::Min),
                    |ui| self.workspace(ui, composition, &mut intents),
                );
                ui.add_space(cl::space::S2);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), composer_height),
                    Layout::top_down(Align::Min),
                    |ui| self.composer(ui, &mut intents),
                );
                ui.add_space(cl::space::S1);
                self.activity(ui, composition);
            },
        );
        intents
    }

    fn header(
        &self,
        ui: &mut egui::Ui,
        composition: Composition,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let response = egui::Frame::new()
            .fill(Color32::from_rgb(8, 17, 21))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(cl::RADIUS))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(cl::icon::BOT)
                            .size(20.0)
                            .color(Tone::Cyan.color()),
                    );
                    ui.label(
                        RichText::new("DevPULSE Chat")
                            .strong()
                            .size(if composition == Composition::Companion {
                                16.0
                            } else {
                                18.0
                            })
                            .color(cl::ink::STRONG),
                    );
                    ui.label(
                        RichText::new("AGENT WORKSPACE")
                            .monospace()
                            .small()
                            .color(cl::ink::SUBTLE),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if action_button(
                            ui,
                            QUIT_ID,
                            "Quit",
                            Tone::Amber,
                            "Quit DevPULSE and close every peer window",
                        )
                        .clicked()
                        {
                            intents.push(ChatWindowIntent::QuitApplication);
                        }
                        if action_button(
                            ui,
                            SHOW_MAIN_ID,
                            "Show Main",
                            Tone::Cyan,
                            "Restore and focus the DevPULSE main window",
                        )
                        .clicked()
                        {
                            intents.push(ChatWindowIntent::ShowMain);
                        }
                    });
                });
                ui.add_space(cl::space::S2);
                let route = self.route_context.offered().map(|offer| offer.route);
                let route_text = route.map_or_else(
                    || "Main route unavailable".to_owned(),
                    |route| format!("Main  /  {}  /  {}", route.hub, route.sub),
                );
                ui.pilot_label(ROUTE_ID, &route_text);
                ui.label(
                    RichText::new("Observed only · never attached or sent without your action")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            })
            .response;
        cl::gradient_tint(ui.painter(), response.rect, Tone::Cyan);
    }

    fn workspace(
        &self,
        ui: &mut egui::Ui,
        composition: Composition,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        match composition {
            Composition::Wide => {
                ui.horizontal_top(|ui| {
                    let total = ui.available_width();
                    let library = 224.0;
                    let stage = 278.0;
                    let conversation = (total - library - stage - 24.0).max(360.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(library, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| self.provider_library(ui, false, true, intents),
                    );
                    ui.add_space(cl::space::S3);
                    ui.allocate_ui_with_layout(
                        egui::vec2(conversation, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| self.conversation(ui),
                    );
                    ui.add_space(cl::space::S3);
                    // The work stage carries the read-only sessions panel and the
                    // lite route launcher below the context slots; the column
                    // scrolls so nothing reaches under the composer.
                    ui.allocate_ui_with_layout(
                        egui::vec2(stage, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("chat-window-stage")
                                .show(ui, |ui| {
                                    self.work_stage(ui);
                                    ui.add_space(cl::space::S3);
                                    self.sessions(ui);
                                    ui.add_space(cl::space::S3);
                                    self.route_launcher(ui, intents);
                                });
                        },
                    );
                });
            }
            Composition::Standard => {
                ui.horizontal_top(|ui| {
                    let library = 210.0;
                    // Provider library, then the sessions panel and route
                    // launcher stacked below it, all in one scrolling column.
                    ui.allocate_ui_with_layout(
                        egui::vec2(library, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("chat-window-library-standard")
                                .show(ui, |ui| {
                                    self.provider_library(ui, false, false, intents);
                                    ui.add_space(cl::space::S3);
                                    self.sessions(ui);
                                    ui.add_space(cl::space::S3);
                                    self.route_launcher(ui, intents);
                                });
                        },
                    );
                    ui.add_space(cl::space::S3);
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            (ui.available_width() - cl::space::S3).max(320.0),
                            ui.available_height(),
                        ),
                        Layout::top_down(Align::Min),
                        |ui| self.conversation(ui),
                    );
                });
            }
            Composition::Companion => {
                // The narrowest composition keeps the primary controls
                // (conversation + composer) prominent and unclipped. The
                // sessions panel and route launcher belong to the roomier
                // Standard/Wide compositions; widening the window surfaces
                // them. Forcing them here would push the composer off-screen.
                self.provider_library(ui, true, false, intents);
                ui.add_space(cl::space::S2);
                self.conversation(ui);
            }
        }
    }

    fn provider_library(
        &self,
        ui: &mut egui::Ui,
        compact: bool,
        pin_footer: bool,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let response = egui::Frame::new()
            .fill(Color32::from_rgb(8, 14, 18))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(cl::RADIUS))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(if compact { "AGENTS" } else { "AGENT LIBRARY" })
                        .monospace()
                        .small()
                        .color(cl::ink::MUTED),
                );
                ui.add_space(cl::space::S2);
                let views = self.runtime.provider_views();
                if compact {
                    egui::ScrollArea::horizontal()
                        .id_salt("chat-provider-strip")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for provider in &views {
                                    self.provider_control(ui, provider, intents);
                                }
                            });
                        });
                } else {
                    for provider in &views {
                        self.provider_control(ui, provider, intents);
                        ui.add_space(cl::space::S2);
                    }
                    // Pin the footer to the column bottom only when the library
                    // owns the whole column (Wide); when the sessions panel and
                    // launcher stack beneath it, keep the note inline.
                    if pin_footer {
                        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                            ui.label(
                                RichText::new("Adapters qualify independently")
                                    .small()
                                    .color(cl::ink::SUBTLE),
                            );
                        });
                    } else {
                        ui.label(
                            RichText::new("Adapters qualify independently")
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                    }
                }
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(PROVIDERS_ID, Role::Region, "Agent library"),
        );
    }

    fn provider_control(
        &self,
        ui: &mut egui::Ui,
        provider: &ProviderView,
        intents: &mut Vec<ChatWindowIntent>,
    ) {
        let descriptor = provider.registration.manifest.descriptor;
        let selected = self.runtime.selected_provider_id() == Some(descriptor.id);
        let tone = provider_tone(descriptor.accent);
        let response = ui
            .push_id(descriptor.id.0, |ui| {
                // Selected → the neubrutalist ColdLight treatment; unselected →
                // the quiet dark card. One body, so both branches unify on the
                // `.show(..)` `InnerResponse`.
                let body = |ui: &mut egui::Ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(descriptor.glyph).strong().color(tone.color()));
                        ui.label(RichText::new(descriptor.short_label).strong().color(
                            if selected {
                                cl::ink::STRONG
                            } else {
                                cl::ink::BASE
                            },
                        ));
                    });
                    ui.label(
                        RichText::new(provider.registration.activation.label())
                            .small()
                            .color(activation_color(&provider.registration.activation, tone)),
                    );
                };
                if selected {
                    cl::brutal(tone).show(ui, body)
                } else {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(14, 19, 25))
                        .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                        .corner_radius(CornerRadius::same(cl::RADIUS_SM))
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, body)
                }
                .response
                .interact(Sense::click())
            })
            .inner;
        let response = cl::tip(
            response,
            &format!("Switch to {} · {}", descriptor.label, descriptor.summary),
        );
        let mut metadata = NodeMetadata::new(
            PROVIDERS_ID.child(&PilotKey::new(descriptor.id.0)),
            Role::Tab,
            descriptor.label,
        );
        metadata.selected = Some(selected);
        metadata.value = Some(provider.registration.activation.label().to_owned());
        ui.pilot_track(&response, metadata);
        if response.clicked() {
            intents.push(ChatWindowIntent::SelectProvider(descriptor.id));
        }
    }

    fn conversation(&self, ui: &mut egui::Ui) {
        let selected = self.runtime.selected_provider();
        let response = egui::Frame::new()
            .fill(Color32::from_rgb(6, 12, 15))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(cl::RADIUS))
            .inner_margin(egui::Margin::same(18))
            .show(ui, |ui| {
                ui.set_min_height((ui.available_height() - 36.0).max(190.0));
                ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height() * 0.18).min(72.0));
                    ui.label(
                        RichText::new("Start with an agent")
                            .size(28.0)
                            .color(cl::ink::STRONG),
                    );
                    ui.label(
                        RichText::new(
                            "One workspace for conversations, plans, tools, routes, and native working surfaces.",
                        )
                        .color(cl::ink::MUTED),
                    );
                    ui.add_space(cl::space::S4);
                    if let Some(provider) = selected {
                        let descriptor = provider.registration.manifest.descriptor;
                        let tone = provider_tone(descriptor.accent);
                        ui.label(
                            RichText::new(format!(
                                "{} · {}",
                                descriptor.label,
                                provider.registration.activation.label()
                            ))
                            .strong()
                            .color(tone.color()),
                        );
                        if let Some(reason) = provider.registration.activation.reason() {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(reason).small().color(cl::ink::MUTED),
                                )
                                .wrap(),
                            );
                        }
                    } else {
                        ui.label(
                            RichText::new("No provider is registered.")
                                .color(Tone::Amber.color()),
                        );
                    }
                });
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(WORKSPACE_ID, Role::Region, "New session workspace"),
        );
    }

    fn work_stage(&self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(Color32::from_rgb(8, 14, 18))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(cl::RADIUS))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                // Compact fixed height: the stage now shares its column with the
                // sessions panel and route launcher inside a scroll area, so it
                // must not grab the full column height.
                ui.set_min_height(180.0);
                ui.label(
                    RichText::new("WORK STAGE")
                        .monospace()
                        .small()
                        .color(cl::ink::MUTED),
                );
                ui.add_space(cl::space::S3);
                ui.label(
                    RichText::new("Context surfaces arrive here")
                        .strong()
                        .color(cl::ink::BASE),
                );
                ui.label(
                    RichText::new(
                        "Plans, diffs, artifacts, approvals, and route previews use governed native renderers.",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
                ui.add_space(cl::space::S4);
                stage_slot(ui, "PLAN", Tone::Lime);
                stage_slot(ui, "DIFF", Tone::Amber);
                stage_slot(ui, "ROUTE", Tone::Cyan);
            });
    }

    /// The read-only sessions panel: the selected provider's bounded local
    /// history index, ported from the retired rail hub's session rail. It
    /// never writes a provider store and never mutates runtime state.
    fn sessions(&self, ui: &mut egui::Ui) {
        let response = egui::Frame::new()
            .fill(Color32::from_rgb(8, 14, 18))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(cl::RADIUS))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("SESSIONS")
                        .monospace()
                        .small()
                        .color(cl::ink::MUTED),
                );
                ui.label(
                    RichText::new("Read-only · never written")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
                ui.add_space(cl::space::S2);

                let Some(provider) = self.runtime.selected_provider() else {
                    ui.label(
                        RichText::new("Select an agent.")
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    return;
                };
                let descriptor = provider.registration.manifest.descriptor;
                ui.label(
                    RichText::new(descriptor.label)
                        .strong()
                        .color(cl::ink::STRONG),
                );
                ui.add_space(cl::space::S2);

                let Some(history) = self.histories.get(&descriptor.id) else {
                    ui.label(
                        RichText::new("No bounded local history source is available.")
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    return;
                };
                if let Some(error) = &history.error {
                    ui.add(
                        egui::Label::new(RichText::new(error).small().color(Tone::Coral.color()))
                            .wrap(),
                    );
                }
                let Some(index) = &history.index else {
                    return;
                };
                ui.label(
                    RichText::new(format!(
                        "{} local read-only session{}{}",
                        index.sessions.len(),
                        if index.sessions.len() == 1 { "" } else { "s" },
                        if index.truncated {
                            " · INDEX CAPPED"
                        } else {
                            ""
                        }
                    ))
                    .small()
                    .color(cl::ink::MUTED),
                );
                ui.add_space(cl::space::S2);
                if index.sessions.is_empty() {
                    ui.label(
                        RichText::new("No sessions available.")
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt(("chat-window-sessions", descriptor.id.0))
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for session in &index.sessions {
                            self.session_card(ui, session);
                        }
                    });
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(SESSIONS_ID, Role::Region, "Chat sessions"),
        );
    }

    fn session_card(&self, ui: &mut egui::Ui, session: &IndexedSession) {
        egui::Frame::new()
            .fill(Color32::from_rgb(14, 19, 25))
            .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
            .corner_radius(CornerRadius::same(cl::RADIUS_SM))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(session_state_glyph(session.summary.state))
                            .color(Tone::Cyan.color()),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(&session.summary.title)
                                .strong()
                                .color(cl::ink::BASE),
                        )
                        .truncate(),
                    );
                });
                ui.add(
                    egui::Label::new(
                        RichText::new(&session.summary.workspace)
                            .small()
                            .color(cl::ink::SUBTLE),
                    )
                    .truncate(),
                );
            });
        ui.add_space(cl::space::S2);
    }

    /// The lite route launcher: taps emit `NavigateTo(route)` so the host can
    /// publish a `NavigateRequest` and the MAIN window navigates. Chat launches
    /// routes; it never embeds them.
    fn route_launcher(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let response = egui::Frame::new()
            .fill(Color32::from_rgb(8, 14, 18))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(cl::RADIUS))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("LOAD ROUTE")
                        .monospace()
                        .small()
                        .color(cl::ink::MUTED),
                );
                ui.label(
                    RichText::new("Jump the main window to an app route")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
                ui.add_space(cl::space::S2);
                if self.routes.is_empty() {
                    ui.label(
                        RichText::new("No routes are available.")
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt("chat-window-route-launcher")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for (label, route) in &self.routes {
                            let button = cl::brutal(Tone::Cyan)
                                .show(ui, |ui| {
                                    ui.label(RichText::new(label).strong().color(cl::ink::STRONG));
                                })
                                .response
                                .interact(Sense::click());
                            let button = cl::tip(
                                button,
                                &format!("Open {label} in the main DevPULSE window"),
                            );
                            ui.pilot_track(
                                &button,
                                NodeMetadata::new(
                                    ROUTE_LAUNCHER_ID.child(&PilotKey::new(format!(
                                        "{}/{}",
                                        route.hub, route.sub
                                    ))),
                                    Role::Button,
                                    label.clone(),
                                ),
                            );
                            if button.clicked() {
                                intents.push(ChatWindowIntent::NavigateTo(*route));
                            }
                            ui.add_space(cl::space::S2);
                        }
                    });
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(ROUTE_LAUNCHER_ID, Role::Region, "Route launcher"),
        );
    }

    fn composer(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        let selected = self.runtime.selected_provider();
        let can_send = selected
            .as_ref()
            .is_some_and(|provider| provider.registration.activation.can_send());
        let mut draft = selected
            .as_ref()
            .map(|provider| provider.draft.clone())
            .unwrap_or_default();
        let provider_id = selected
            .as_ref()
            .map(|provider| provider.registration.manifest.descriptor.id);

        egui::Frame::new()
            .fill(Color32::from_rgb(10, 17, 21))
            .stroke(Stroke::new(1.0, teal_hairline()))
            .corner_radius(CornerRadius::same(cl::RADIUS))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 110.0).max(180.0);
                    let edit = ui
                        .add_enabled_ui(can_send, |ui| {
                            ui.add_sized(
                                [width, 66.0],
                                egui::TextEdit::multiline(&mut draft).hint_text(if can_send {
                                    "Ask, plan, or build…"
                                } else {
                                    "Provider adapter must qualify before sending"
                                }),
                            )
                        })
                        .inner;
                    let mut metadata =
                        NodeMetadata::new(COMPOSER_ID, Role::TextInput, "Message composer");
                    metadata.editable = can_send;
                    metadata.value = (!draft.is_empty()).then(|| draft.clone());
                    ui.pilot_track(&edit, metadata);
                    if edit.changed() {
                        if let Some(provider) = provider_id {
                            intents.push(ChatWindowIntent::UpdateDraft {
                                provider,
                                value: draft.clone(),
                            });
                        }
                    }

                    let send = ui.add_enabled(
                        can_send && !draft.trim().is_empty(),
                        egui::Button::new("Send").min_size(egui::vec2(84.0, 42.0)),
                    );
                    let send = cl::tip(
                        send,
                        if can_send {
                            "Send this message"
                        } else {
                            "Sending is unavailable until this provider adapter qualifies"
                        },
                    );
                    ui.pilot_track(
                        &send,
                        NodeMetadata::new(SEND_ID, Role::Button, "Send message"),
                    );
                });
            });
    }

    fn activity(&self, ui: &mut egui::Ui, composition: Composition) {
        let selected = self.runtime.selected_provider();
        let status = selected.map_or_else(
            || "No provider".to_owned(),
            |provider| {
                format!(
                    "{} · {} · no active turn",
                    provider.registration.manifest.descriptor.label,
                    provider.registration.activation.label()
                )
            },
        );
        let response = ui
            .horizontal(|ui| {
                ui.label(
                    RichText::new("ACTIVITY")
                        .monospace()
                        .small()
                        .color(Tone::Amber.color()),
                );
                ui.label(RichText::new(status).small().color(cl::ink::MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{composition:?}"))
                            .monospace()
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                });
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(ACTIVITY_ID, Role::Status, "Chat activity"),
        );
    }
}

fn action_button(
    ui: &mut egui::Ui,
    id: PilotId,
    label: &str,
    tone: Tone,
    tooltip: &str,
) -> egui::Response {
    let response = cl::brutal(tone)
        .show(ui, |ui| {
            ui.label(RichText::new(label).strong().color(cl::ink::STRONG));
        })
        .response
        .interact(Sense::click());
    let response = cl::tip(response, tooltip);
    ui.pilot_track(
        &response,
        NodeMetadata::new(id, Role::Button, label.to_owned()),
    );
    response
}

fn stage_slot(ui: &mut egui::Ui, label: &str, tone: Tone) {
    let rect = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 42.0), Sense::hover())
        .0;
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(cl::RADIUS_SM),
        Color32::from_rgb(12, 20, 24),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(cl::RADIUS_SM),
        Stroke::new(1.0, tone.wash()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::monospace(11.0),
        tone.color(),
    );
    ui.add_space(cl::space::S2);
}

fn paint_ember_field(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(4, 8, 11));

    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let mut mesh = Mesh::default();
    let top = Color32::from_rgb(4, 8, 11);
    let bottom_left = Color32::from_rgb(5, 20, 22);
    let bottom_right = Color32::from_rgb(8, 13, 16);
    mesh.vertices.extend_from_slice(&[
        Vertex {
            pos: rect.left_top(),
            uv: WHITE_UV,
            color: top,
        },
        Vertex {
            pos: rect.right_top(),
            uv: WHITE_UV,
            color: Color32::from_rgb(5, 10, 13),
        },
        Vertex {
            pos: rect.left_bottom(),
            uv: WHITE_UV,
            color: bottom_left,
        },
        Vertex {
            pos: rect.right_bottom(),
            uv: WHITE_UV,
            color: bottom_right,
        },
    ]);
    mesh.indices.extend_from_slice(&[0, 1, 2, 2, 1, 3]);
    painter.add(egui::Shape::mesh(mesh));

    let amber = Tone::Amber.color();
    for index in 0..34_u32 {
        let x_hash = index.wrapping_mul(73).wrapping_add(19) % 997;
        let y_hash = index.wrapping_mul(151).wrapping_add(47) % 991;
        let x = rect.left() + rect.width() * (x_hash as f32 / 997.0);
        let rise = (y_hash as f32 / 991.0).powf(1.8);
        let y = rect.bottom() - rect.height() * 0.34 * rise;
        let size = 1.0 + (index % 3) as f32;
        let alpha = 28 + (index % 5) as u8 * 9;
        let color = Color32::from_rgba_unmultiplied(amber.r(), amber.g(), amber.b(), alpha);
        painter.rect_filled(
            Rect::from_center_size(egui::pos2(x, y), egui::vec2(size, size)),
            CornerRadius::same(1),
            color,
        );
    }
}

fn teal_hairline() -> Color32 {
    let cyan = Tone::Cyan.color();
    Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), 52)
}

fn session_state_glyph(state: SessionState) -> &'static str {
    match state {
        SessionState::ReadOnly => "◇",
        SessionState::Idle => "○",
        SessionState::Running => "●",
        SessionState::Attention => "!",
        SessionState::Failed => "×",
        SessionState::Expired => "–",
    }
}

fn provider_tone(accent: ProviderAccent) -> Tone {
    match accent {
        ProviderAccent::Amber => Tone::Amber,
        ProviderAccent::Cyan => Tone::Cyan,
        ProviderAccent::Violet => Tone::Violet,
        ProviderAccent::Magenta => Tone::Magenta,
        ProviderAccent::Lime => Tone::Lime,
    }
}

fn activation_color(state: &ActivationState, tone: Tone) -> Color32 {
    match state {
        ActivationState::Ready | ActivationState::Experimental { .. } => tone.color(),
        ActivationState::SigningIn => Tone::Amber.color(),
        ActivationState::Failed { .. } | ActivationState::VersionUnsupported { .. } => {
            Tone::Coral.color()
        }
        ActivationState::NotInstalled { .. }
        | ActivationState::SignedOut { .. }
        | ActivationState::Degraded { .. }
        | ActivationState::Blocked { .. }
        | ActivationState::Disabled { .. } => cl::ink::MUTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::{
        Capability, ChatEventContent, ProviderDescriptor, ProviderManifest, ProviderProfileId,
        ProviderSessionId,
    };
    use dpl_chat_runtime::ProviderRegistration;
    use dpl_chat_store::{
        DecodeOutcome, DecodedHistoryEvent, HistoryDecoder, HistoryIndexMetadata, HistoryLimits,
        JsonlHistorySource,
    };
    use dpl_hub::{HubId, SubId};

    const PROVIDER: ProviderId = ProviderId("fixture");
    const CAPS: &[Capability] = &[Capability::ReadSessions];
    const MANIFEST: ProviderManifest = ProviderManifest {
        descriptor: ProviderDescriptor {
            id: PROVIDER,
            label: "Fixture",
            short_label: "Fixture",
            glyph: "F",
            accent: ProviderAccent::Cyan,
            summary: "fixture provider",
        },
        profile: ProviderProfileId("fixture-default"),
        target_capabilities: CAPS,
        delivery_slice: "test",
        evidence: "fixture",
    };
    const ROUTE: Route = Route {
        hub: HubId("home"),
        sub: SubId("shows"),
    };

    fn window() -> ChatWindow {
        ChatWindow::new(
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("registry"),
            ROUTE,
            HistoryCatalog::default(),
            Vec::new(),
        )
    }

    struct FixtureHistoryDecoder;

    impl HistoryDecoder for FixtureHistoryDecoder {
        fn provider_id(&self) -> ProviderId {
            PROVIDER
        }

        fn index_metadata(
            &self,
            path: &std::path::Path,
            _prefix: &str,
        ) -> Option<HistoryIndexMetadata> {
            Some(HistoryIndexMetadata {
                native_id: ProviderSessionId(path.file_stem()?.to_string_lossy().into_owned()),
                title: path.file_stem()?.to_string_lossy().into_owned(),
                workspace: "C:/fixture".into(),
                started_at: "2026-07-24T00:00:00Z".into(),
            })
        }

        fn decode_line(&self, line: &str) -> DecodeOutcome {
            DecodeOutcome::Event(DecodedHistoryEvent {
                turn_id: None,
                content: ChatEventContent::AssistantMessage(line.to_owned()),
            })
        }
    }

    #[test]
    fn navigate_to_yields_a_host_navigation_intent_and_leaves_selection_alone() {
        let target = Route {
            hub: HubId("workbench"),
            sub: SubId("library"),
        };
        let mut window = window();
        assert_eq!(
            window
                .apply(ChatWindowIntent::NavigateTo(target))
                .expect("host intent"),
            Some(HostIntent::NavigateTo(target)),
        );
        // Launching a route must not disturb the selected provider.
        assert_eq!(window.selected_provider_id(), Some(PROVIDER));
    }

    #[test]
    fn sessions_panel_honors_the_catalog_bounded_cap() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "devpulse-chat-window-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create fixture tree");
        std::fs::write(root.join("a.jsonl"), "one\n").expect("write a");
        std::fs::write(root.join("b.jsonl"), "two\n").expect("write b");

        let mut catalog = HistoryCatalog::default();
        catalog
            .register(
                JsonlHistorySource::open(
                    &root,
                    FixtureHistoryDecoder,
                    HistoryLimits {
                        max_files: 1,
                        ..HistoryLimits::default()
                    },
                )
                .expect("open fixture source"),
            )
            .expect("register source");

        let window = ChatWindow::new(
            ChatRuntime::try_from_registrations([ProviderRegistration::planned(MANIFEST)])
                .expect("registry"),
            ROUTE,
            catalog,
            Vec::new(),
        );

        let index = window
            .session_history(PROVIDER)
            .expect("a bounded read-only index");
        assert_eq!(
            index.sessions.len(),
            1,
            "the catalog cap must bound the session list"
        );
        assert!(
            index.truncated,
            "the truncation flag must surface the INDEX CAPPED note"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn composition_boundaries_are_explicit() {
        assert_eq!(Composition::for_width(519.0), Composition::Companion);
        assert_eq!(Composition::for_width(759.9), Composition::Companion);
        assert_eq!(Composition::for_width(760.0), Composition::Standard);
        assert_eq!(Composition::for_width(1179.9), Composition::Standard);
        assert_eq!(Composition::for_width(1180.0), Composition::Wide);
    }

    #[test]
    fn host_intents_do_not_mutate_provider_state() {
        let mut window = window();
        assert_eq!(
            window
                .apply(ChatWindowIntent::ShowMain)
                .expect("host intent"),
            Some(HostIntent::ShowMain),
        );
        assert_eq!(window.selected_provider_id(), Some(PROVIDER));
    }

    #[test]
    fn route_observation_is_passive() {
        let mut window = window();
        let next = Route {
            hub: HubId("workbench"),
            sub: SubId("library"),
        };
        window.observe_route(next);
        assert_eq!(window.route_context.offered().expect("offer").route, next);
        assert!(window.route_context.attached(PROVIDER).is_none());
    }

    #[test]
    fn ember_field_has_a_fixed_primitive_budget() {
        fn shape_count() -> usize {
            let ctx = egui::Context::default();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1120.0, 640.0));
                paint_ember_field(ui.painter(), rect);
            })
            .shapes
            .len()
        }

        assert_eq!(shape_count(), 36);
        assert_eq!(shape_count(), 36);
    }

    #[test]
    fn responsive_shell_keeps_primary_controls_visible_and_actionable() {
        for size in [
            egui::vec2(520.0, 560.0),
            egui::vec2(900.0, 640.0),
            egui::vec2(1280.0, 720.0),
        ] {
            let mut harness = egui_kittest::Harness::builder()
                .with_size(size)
                .build_ui_state(
                    |ui, window: &mut ChatWindow| {
                        let _ = window.ui(ui);
                    },
                    window(),
                );
            egui_pilot_ai::install(&harness.ctx);
            harness.run_steps(3);

            let frame = egui_pilot_ai::handle(&harness.ctx)
                .expect("pilot installed")
                .latest_frame();
            for path in [
                "chat.header.show-main",
                "chat.header.quit",
                "chat.providers/fixture",
                "chat.workspace.new-session",
                "chat.composer",
                "chat.composer.send",
                "chat.activity",
            ] {
                let id = egui_pilot_ai::protocol::NodeId::new(format!("root/{path}"));
                let node = frame
                    .nodes
                    .iter()
                    .find(|node| node.id == id)
                    .unwrap_or_else(|| panic!("missing {id} at {size:?}"));
                assert!(
                    !node.state.hidden && !node.state.clipped,
                    "{id} is not visibly actionable at {size:?}: {:?}",
                    node.state
                );
            }

            let composer = frame
                .nodes
                .iter()
                .find(|node| node.id == egui_pilot_ai::protocol::NodeId::new("root/chat.composer"))
                .expect("composer");
            assert!(composer.state.disabled);
            assert!(!composer.state.editable);
        }
    }
}
