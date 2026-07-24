//! Chat — the route-aware multi-agent platform.
//!
//! CH1 is deliberately provider-neutral: registered descriptors and
//! deterministic state render now; live provider processes remain blocked on
//! the provider-runtime ADR.

use dpl_chat_context::RouteContextState;
use dpl_chat_core::{
    ActivationState, Capability, ChatEvent, ChatSessionId, ProviderAccent, ProviderId,
    ProviderManifest, SessionState,
};
use dpl_chat_extensions::ExtensionRegistry;
use dpl_chat_renderers::{semantic_label, EventRendererRegistry, EVENT_ROW_HEIGHT};
use dpl_chat_runtime::{ChatRuntime, ProviderView};
use dpl_chat_store::{HistoryCatalog, HistoryCursor, HistoryIndex, HistoryLocator};
use dpl_chat_windows::ChatWindowRegistry;
use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::{Align, Layout, RichText, ScrollArea, TextEdit};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::collections::{HashMap, HashSet};

pub const HUB: HubId = HubId("chat");
pub const SESSIONS: SubId = SubId("sessions");
pub const COUNCIL: SubId = SubId("council");

const PILOT_PROVIDER: PilotId = PilotId::new("chat.provider");
const PILOT_SESSION: PilotId = PilotId::new("chat.session");
const PILOT_EVENT: PilotId = PilotId::new("chat.event");
const PILOT_TRANSCRIPT: PilotId = PilotId::new("chat.transcript");
const PILOT_LOAD_EARLIER: PilotId = PilotId::new("chat.transcript.load-earlier");
const PILOT_COMPOSER: PilotId = PilotId::new("chat.composer");
const PILOT_SEND: PilotId = PilotId::new("chat.send");
const PILOT_ATTACH_ROUTE: PilotId = PilotId::new("chat.context.attach-route");
const PILOT_REMOVE_ROUTE: PilotId = PilotId::new("chat.context.remove-route");
const MAX_TRANSCRIPT_EVENTS: usize = 1_000;

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Chat",
    icon: cl::icon::BOT,
    tone: Tone::Lime,
    subroutes: &[
        SubRoute {
            id: SESSIONS,
            label: "Sessions",
            blurb: "Provider-isolated sessions, explicit route context, tools, and artifacts.",
        },
        SubRoute {
            id: COUNCIL,
            label: "Council",
            blurb: "Cross-provider work after every participating adapter qualifies alone.",
        },
    ],
};

#[derive(Default)]
pub struct ChatHub {
    fabric: Option<dpl_fabric::Fabric>,
    shell_events: Option<dpl_fabric::ShellReceiver>,
    runtime: ChatRuntime,
    route_context: RouteContextState,
    extensions: ExtensionRegistry,
    windows: ChatWindowRegistry,
    history: HistoryCatalog,
    history_attempted: HashSet<ProviderId>,
    history_indices: HashMap<ProviderId, HistoryIndex>,
    history_errors: HashMap<ProviderId, String>,
    transcripts: HashMap<ChatSessionId, LoadedTranscript>,
    renderers: EventRendererRegistry,
}

struct LoadedTranscript {
    locator: HistoryLocator,
    events: Vec<ChatEvent>,
    next_cursor: Option<HistoryCursor>,
    source_bytes: u64,
    truncated_before: bool,
    capacity_reached: bool,
}

impl ChatHub {
    pub fn new(runtime: ChatRuntime) -> Self {
        Self {
            fabric: None,
            shell_events: None,
            runtime,
            route_context: RouteContextState::default(),
            extensions: ExtensionRegistry::default(),
            windows: ChatWindowRegistry::default(),
            history: HistoryCatalog::default(),
            history_attempted: HashSet::new(),
            history_indices: HashMap::new(),
            history_errors: HashMap::new(),
            transcripts: HashMap::new(),
            renderers: EventRendererRegistry::default(),
        }
    }

    pub fn with_history(runtime: ChatRuntime, history: HistoryCatalog) -> Self {
        Self {
            history,
            ..Self::new(runtime)
        }
    }

    pub fn wire_fabric(&mut self, fabric: &dpl_fabric::Fabric) {
        self.fabric = Some(fabric.clone());
        self.shell_events = Some(fabric.subscribe_shell());
    }

    fn drain_shell_events(&mut self) {
        let Some(events) = &mut self.shell_events else {
            return;
        };
        while let Ok(event) = events.try_recv() {
            if let dpl_fabric::ShellEvent::RouteChanged { route } = event {
                // Opening Chat does not replace the useful source context with
                // "Chat / Sessions". Keep the last non-Chat route as the offer.
                if route.hub != HUB {
                    self.route_context.observe(route);
                }
            }
        }
    }

    fn ensure_history_index(&mut self, provider_id: ProviderId) {
        if !self.history.has_source(provider_id) || !self.history_attempted.insert(provider_id) {
            return;
        }
        match self.history.list(provider_id, None) {
            Ok(index) => {
                let sessions = index
                    .sessions
                    .iter()
                    .map(|session| session.summary.clone())
                    .collect();
                if let Err(error) = self.runtime.replace_sessions(provider_id, sessions) {
                    self.history_errors.insert(provider_id, error.to_string());
                    return;
                }
                if let Err(error) = self.runtime.qualify_capabilities(
                    provider_id,
                    [Capability::ReadSessions, Capability::PageHistory],
                ) {
                    self.history_errors.insert(provider_id, error.to_string());
                    return;
                }
                self.history_errors.remove(&provider_id);
                self.history_indices.insert(provider_id, index);
            }
            Err(error) => {
                self.history_errors.insert(provider_id, error.to_string());
            }
        }
    }

    fn ensure_selected_transcript(&mut self) {
        let Some(provider) = self.runtime.selected_provider() else {
            return;
        };
        let provider_id = provider.registration.manifest.descriptor.id;
        let Some(session_id) = provider.active_session else {
            return;
        };
        if self.transcripts.contains_key(&session_id) || !self.history.has_source(provider_id) {
            return;
        }
        let Some(locator) = self
            .history_indices
            .get(&provider_id)
            .and_then(|index| {
                index
                    .sessions
                    .iter()
                    .find(|session| session.summary.id == session_id)
            })
            .map(|session| session.locator)
        else {
            return;
        };
        if !provider
            .registration
            .sessions
            .iter()
            .any(|session| session.id == session_id)
        {
            return;
        }
        match self.history.page(provider_id, locator, None) {
            Ok(page) => {
                self.transcripts.insert(
                    session_id,
                    LoadedTranscript {
                        locator,
                        events: page.events,
                        next_cursor: page.next_cursor,
                        source_bytes: page.source_bytes,
                        truncated_before: page.truncated_before,
                        capacity_reached: false,
                    },
                );
            }
            Err(error) => {
                self.history_errors.insert(provider_id, error.to_string());
            }
        }
    }

    fn load_earlier(&mut self, provider_id: ProviderId, session_id: ChatSessionId) {
        let Some(current) = self.transcripts.get(&session_id) else {
            return;
        };
        if current.capacity_reached {
            return;
        }
        let Some(cursor) = current.next_cursor else {
            return;
        };
        let locator = current.locator;
        match self.history.page(provider_id, locator, Some(cursor)) {
            Ok(page) => {
                let current = self
                    .transcripts
                    .get_mut(&session_id)
                    .expect("selected transcript remains loaded");
                if current.events.len() + page.events.len() > MAX_TRANSCRIPT_EVENTS {
                    current.capacity_reached = true;
                    return;
                }
                let mut combined = page.events;
                combined.append(&mut current.events);
                current.events = combined;
                current.next_cursor = page.next_cursor;
                current.truncated_before = page.truncated_before;
            }
            Err(error) => {
                self.history_errors.insert(provider_id, error.to_string());
            }
        }
    }

    fn sessions_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(provider_id) = self.runtime.selected_provider_id() {
            self.ensure_history_index(provider_id);
        }
        self.heading(ui);
        self.provider_bar(ui);
        ui.add_space(cl::space::S3);

        let available = ui.available_size();
        // egui measures logical points: a 1440px window at 1.25 DPR exposes
        // roughly 1,150 points before shell chrome. Keep all three zones at
        // that ordinary operator size; collapse context first, then sessions.
        let show_sessions = available.x >= 620.0;
        let show_context = available.x >= 900.0;
        let session_width = if show_sessions { 210.0 } else { 0.0 };
        let context_width = if show_context { 220.0 } else { 0.0 };
        let gutters = if show_sessions && show_context {
            cl::space::S4 * 2.0
        } else if show_sessions || show_context {
            cl::space::S4
        } else {
            0.0
        };
        let conversation_width = (available.x - session_width - context_width - gutters).max(280.0);

        ui.horizontal_top(|ui| {
            if show_sessions {
                ui.allocate_ui_with_layout(
                    egui::vec2(session_width, available.y),
                    Layout::top_down(Align::Min),
                    |ui| self.session_rail(ui),
                );
                ui.add_space(cl::space::S4);
            }

            ui.allocate_ui_with_layout(
                egui::vec2(conversation_width, available.y),
                Layout::top_down(Align::Min),
                |ui| self.conversation_zone(ui),
            );

            if show_context {
                ui.add_space(cl::space::S4);
                ui.allocate_ui_with_layout(
                    egui::vec2(context_width, available.y),
                    Layout::top_down(Align::Min),
                    |ui| self.context_inspector(ui),
                );
            }
        });
    }

    fn heading(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(cl::icon::BOT)
                    .size(22.0)
                    .color(Tone::Lime.color()),
            );
            ui.label(RichText::new("Chat").heading().color(cl::ink::STRONG));
            ui.label(
                RichText::new("CH2 · READ-ONLY SESSIONS")
                    .small()
                    .monospace()
                    .color(cl::ink::SUBTLE),
            );
        });
        ui.add(
            egui::Label::new(
                RichText::new(
                    "Each provider keeps its own account, sessions, draft, capabilities, and failures.",
                )
                .color(cl::ink::MUTED),
            )
            .wrap(),
        );
    }

    fn provider_bar(&mut self, ui: &mut egui::Ui) {
        let views = self.runtime.provider_views();
        if views.is_empty() {
            cl::clay_frame().show(ui, |ui| {
                ui.label(
                    RichText::new("No provider adapters registered")
                        .strong()
                        .color(cl::ink::STRONG),
                );
                ui.label(
                    RichText::new("The registry is empty; Chat will not invent a provider.")
                        .color(cl::ink::MUTED),
                );
            });
            return;
        }

        let selected = self.runtime.selected_provider_id();
        ScrollArea::horizontal()
            .id_salt("chat-provider-bar")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for view in views {
                        let manifest = view.registration.manifest;
                        let descriptor = manifest.descriptor;
                        let tone = provider_tone(descriptor.accent);
                        let is_selected = selected == Some(descriptor.id);
                        ui.push_id(descriptor.id.0, |ui| {
                            let frame = if is_selected {
                                cl::brutal_frame(tone)
                            } else {
                                cl::card_frame()
                            };
                            let response = frame
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(descriptor.glyph)
                                                .strong()
                                                .color(tone.color()),
                                        );
                                        ui.label(RichText::new(descriptor.short_label).color(
                                            if is_selected {
                                                cl::ink::STRONG
                                            } else {
                                                cl::ink::BASE
                                            },
                                        ));
                                        ui.label(
                                            RichText::new(view.registration.activation.label())
                                                .small()
                                                .color(activation_color(
                                                    &view.registration.activation,
                                                    tone,
                                                )),
                                        );
                                    });
                                })
                                .response
                                .interact(egui::Sense::click());
                            let response = cl::tip(
                                response,
                                &format!("Open {}. {}", descriptor.label, descriptor.summary),
                            );
                            let mut metadata = NodeMetadata::new(
                                PILOT_PROVIDER.child(&PilotKey::new(descriptor.id.0)),
                                egui_pilot_ai::protocol::Role::Tab,
                                descriptor.label,
                            );
                            metadata.selected = Some(is_selected);
                            ui.pilot_track(&response, metadata);
                            if response.clicked() {
                                let _ = self.runtime.select_provider(descriptor.id);
                            }
                        });
                    }
                });
            });
    }

    fn session_rail(&mut self, ui: &mut egui::Ui) {
        if let Some(provider_id) = self.runtime.selected_provider_id() {
            self.ensure_history_index(provider_id);
        }
        let response = cl::clay_frame()
            .show(ui, |ui| {
                // `clay_frame` adds inner/outer chrome around this content. Budget
                // it here so the plate never reaches behind the shell status bar.
                ui.set_min_height((ui.available_height() - 44.0).max(120.0));
                zone_label(ui, "SESSIONS");
                let Some(provider) = self.runtime.selected_provider() else {
                    ui.label(RichText::new("Select a provider.").color(cl::ink::MUTED));
                    return;
                };
                ui.label(
                    RichText::new(provider.registration.manifest.descriptor.label)
                        .strong()
                        .color(cl::ink::STRONG),
                );
                ui.add_space(cl::space::S2);

                if let Some(index) = self
                    .history_indices
                    .get(&provider.registration.manifest.descriptor.id)
                {
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
                }
                if let Some(error) = self
                    .history_errors
                    .get(&provider.registration.manifest.descriptor.id)
                {
                    ui.add(
                        egui::Label::new(RichText::new(error).small().color(Tone::Coral.color()))
                            .wrap(),
                    );
                }

                if provider.registration.sessions.is_empty() {
                    ui.label(
                        RichText::new("No sessions available.")
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    ui.label(
                        RichText::new("No bounded local history source is available.")
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    return;
                }

                ScrollArea::vertical()
                    .id_salt("chat-session-rail")
                    .show(ui, |ui| {
                        for session in &provider.registration.sessions {
                            let selected = provider.active_session == Some(session.id);
                            let label = format!(
                                "{}  {}",
                                session_state_glyph(session.state),
                                session.title
                            );
                            let response = ui.selectable_label(selected, label);
                            let response = cl::tip(
                                response,
                                &format!("Open {} in {}", session.title, session.workspace),
                            );
                            let mut metadata = NodeMetadata::new(
                                PILOT_SESSION.child(&PilotKey::new(session.id.to_string())),
                                egui_pilot_ai::protocol::Role::Button,
                                session.title.clone(),
                            )
                            .parent(PilotId::new("chat.session-rail"));
                            metadata.selected = Some(selected);
                            ui.pilot_track(&response, metadata);
                            if response.clicked() {
                                let _ = self.runtime.select_session(
                                    provider.registration.manifest.descriptor.id,
                                    session.id,
                                );
                            }
                        }
                    });
            })
            .response;
        ui.pilot_track(
            &response,
            NodeMetadata::new(
                PilotId::new("chat.session-rail"),
                egui_pilot_ai::protocol::Role::Region,
                "Chat sessions",
            ),
        );
    }

    fn conversation_zone(&mut self, ui: &mut egui::Ui) {
        self.ensure_selected_transcript();
        let Some(provider) = self.runtime.selected_provider() else {
            return;
        };
        let descriptor = provider.registration.manifest.descriptor;
        let tone = provider_tone(descriptor.accent);
        let active_session = provider.active_session;
        let loaded = active_session.and_then(|session_id| self.transcripts.get(&session_id));
        let events = loaded.map_or_else(
            || {
                provider
                    .registration
                    .events
                    .iter()
                    .filter(|event| Some(event.session_id) == active_session)
                    .collect::<Vec<_>>()
            },
            |transcript| transcript.events.iter().collect::<Vec<_>>(),
        );
        let next_cursor = loaded.and_then(|transcript| transcript.next_cursor);
        let source_bytes = loaded.map(|transcript| transcript.source_bytes);
        let capacity_reached = loaded.is_some_and(|transcript| transcript.capacity_reached);
        let composer_outer_height = 156.0;
        let transcript_outer_height =
            (ui.available_height() - composer_outer_height - cl::space::S3).max(180.0);
        let transcript_content_height = (transcript_outer_height - 44.0).max(136.0);

        let mut load_earlier = false;
        let transcript_response = cl::clay_frame()
            .show(ui, |ui| {
                ui.set_min_height(transcript_content_height);
                ui.set_max_height(transcript_content_height);
                if ui.available_width() < 520.0 {
                    zone_label(ui, "CONVERSATION");
                    ui.horizontal(|ui| {
                        transcript_controls(
                            ui,
                            &provider.registration.activation,
                            tone,
                            next_cursor,
                            capacity_reached,
                            source_bytes,
                            &mut load_earlier,
                        );
                    });
                } else {
                    ui.horizontal(|ui| {
                        zone_label(ui, "CONVERSATION");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            transcript_controls(
                                ui,
                                &provider.registration.activation,
                                tone,
                                next_cursor,
                                capacity_reached,
                                source_bytes,
                                &mut load_earlier,
                            );
                        });
                    });
                }
                ui.separator();

                if events.is_empty() {
                    if let Some(error) = self.history_errors.get(&descriptor.id) {
                        ui.add(
                            egui::Label::new(RichText::new(error).color(Tone::Coral.color()))
                                .wrap(),
                        );
                    } else {
                        activation_panel(
                            ui,
                            &provider.registration.activation,
                            provider.registration.manifest,
                            tone,
                        );
                    }
                } else {
                    ScrollArea::vertical()
                        .id_salt(("chat-transcript", descriptor.id.0))
                        .auto_shrink([false, false])
                        .show_rows(ui, EVENT_ROW_HEIGHT, events.len(), |ui, range| {
                            for event in &events[range] {
                                match self.renderers.render(ui, &event.content, tone) {
                                    Ok(response) => ui.pilot_track(
                                        &response,
                                        NodeMetadata::new(
                                            PILOT_EVENT.child(&PilotKey::new(event.id.to_string())),
                                            egui_pilot_ai::protocol::Role::Label,
                                            semantic_label(&event.content),
                                        )
                                        .parent(PILOT_TRANSCRIPT),
                                    ),
                                    Err(error) => {
                                        ui.label(
                                            RichText::new(error.to_string())
                                                .color(Tone::Coral.color()),
                                        );
                                    }
                                }
                            }
                        });
                }
            })
            .response;
        ui.pilot_track(
            &transcript_response,
            NodeMetadata::new(
                PILOT_TRANSCRIPT,
                egui_pilot_ai::protocol::Role::Region,
                "Chat transcript",
            ),
        );

        if load_earlier {
            if let Some(session_id) = active_session {
                self.load_earlier(descriptor.id, session_id);
            }
        }

        ui.add_space(cl::space::S3);
        self.composer(ui, provider, tone);
    }

    fn composer(&mut self, ui: &mut egui::Ui, provider: ProviderView, tone: Tone) {
        let provider_id = provider.registration.manifest.descriptor.id;
        let mut draft = provider.draft;
        let can_send = provider.registration.activation.can_send()
            && provider
                .registration
                .has_capability(Capability::StreamTurns)
            && provider.active_session.is_some();
        let composer_response = cl::clay_frame()
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    zone_label(ui, "COMPOSER");
                    ui.label(
                        RichText::new(provider.registration.manifest.descriptor.label)
                            .small()
                            .color(tone.color()),
                    );
                });
                let edit = ui.add_enabled(
                    can_send,
                    TextEdit::multiline(&mut draft)
                        .desired_rows(2)
                        .hint_text("Message this provider…"),
                );
                let edit = cl::tip(
                    edit,
                    if can_send {
                        "Write a message for this provider and session"
                    } else {
                        "Sending stays disabled until this provider and session are qualified"
                    },
                );
                ui.pilot_track(
                    &edit,
                    NodeMetadata::new(
                        PILOT_COMPOSER,
                        egui_pilot_ai::protocol::Role::TextInput,
                        format!(
                            "{} message composer",
                            provider.registration.manifest.descriptor.label
                        ),
                    )
                    .parent(PilotId::new("chat.composer-region")),
                );
                if edit.changed() {
                    let _ = self.runtime.set_draft(provider_id, draft);
                }
                ui.horizontal(|ui| {
                    let send =
                        ui.add_enabled(can_send, egui::Button::new(RichText::new("Send").strong()));
                    let send = cl::tip(
                        send,
                        if can_send {
                            "Send with the visible provider, workspace, and context"
                        } else {
                            "Unavailable: provider adapter or active session is not ready"
                        },
                    );
                    ui.pilot_track(
                        &send,
                        NodeMetadata::new(
                            PILOT_SEND,
                            egui_pilot_ai::protocol::Role::Button,
                            format!(
                                "Send with {}",
                                provider.registration.manifest.descriptor.label
                            ),
                        )
                        .parent(PilotId::new("chat.composer-region")),
                    );
                    match self.route_context.attached(provider_id) {
                        Some(snapshot) => {
                            let stale = self.route_context.is_stale(provider_id);
                            ui.label(
                                RichText::new(format!(
                                    "{} / {}{}",
                                    snapshot.route.hub,
                                    snapshot.route.sub,
                                    if stale { " · STALE" } else { "" }
                                ))
                                .small()
                                .color(if stale {
                                    Tone::Amber.color()
                                } else {
                                    cl::ink::MUTED
                                }),
                            );
                        }
                        None => {
                            ui.label(
                                RichText::new("No route context attached")
                                    .small()
                                    .color(cl::ink::SUBTLE),
                            );
                        }
                    }
                });
            })
            .response;
        ui.pilot_track(
            &composer_response,
            NodeMetadata::new(
                PilotId::new("chat.composer-region"),
                egui_pilot_ai::protocol::Role::Region,
                "Chat composer",
            ),
        );
    }

    fn context_inspector(&mut self, ui: &mut egui::Ui) {
        cl::clay_frame().show(ui, |ui| {
            ui.set_min_height((ui.available_height() - 44.0).max(120.0));
            zone_label(ui, "CONTEXT");
            let provider_id = self.runtime.selected_provider_id();
            if let Some(provider_id) = provider_id {
                if let Some(attached) = self.route_context.attached(provider_id) {
                    ui.label(
                        RichText::new(format!(
                            "Attached: {} / {}",
                            attached.route.hub, attached.route.sub
                        ))
                        .strong()
                        .color(cl::ink::STRONG),
                    );
                    if self.route_context.is_stale(provider_id) {
                        ui.label(
                            RichText::new("This attachment is older than the current route offer.")
                                .small()
                                .color(Tone::Amber.color()),
                        );
                    }
                    let remove = cl::tip(
                        ui.button("Remove"),
                        "Remove this context from the selected provider draft",
                    );
                    ui.pilot_track(
                        &remove,
                        NodeMetadata::new(
                            PILOT_REMOVE_ROUTE,
                            egui_pilot_ai::protocol::Role::Button,
                            "Remove attached route context",
                        ),
                    );
                    if remove.clicked() {
                        self.route_context.remove(provider_id);
                    }
                } else {
                    ui.label(
                        RichText::new("Nothing is attached")
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                }

                ui.add_space(cl::space::S3);
                zone_label(ui, "ROUTE OFFER");
                if let Some(offered) = self.route_context.offered() {
                    ui.label(
                        RichText::new(format!("{} / {}", offered.route.hub, offered.route.sub))
                            .color(cl::ink::BASE),
                    );
                    ui.label(
                        RichText::new("Observed only · never sent automatically")
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    let already_attached = self
                        .route_context
                        .attached(provider_id)
                        .is_some_and(|attached| attached.id == offered.id);
                    let attach = cl::tip(
                        ui.add_enabled(!already_attached, egui::Button::new("Attach route")),
                        "Explicitly attach this route to the selected provider draft",
                    );
                    ui.pilot_track(
                        &attach,
                        NodeMetadata::new(
                            PILOT_ATTACH_ROUTE,
                            egui_pilot_ai::protocol::Role::Button,
                            "Attach offered route context",
                        ),
                    );
                    if attach.clicked() {
                        let _ = self.route_context.attach_offered(provider_id);
                    }
                } else {
                    ui.label(
                        RichText::new("Navigate from another hub to offer its route.")
                            .small()
                            .color(cl::ink::MUTED),
                    );
                }
            }
            ui.add_space(cl::space::S4);
            zone_label(ui, "EXTENSIONS");
            ui.label(
                RichText::new(format!(
                    "{} registered · none active without qualified capabilities",
                    self.extensions.len()
                ))
                .small()
                .color(cl::ink::MUTED),
            );
            ui.add_space(cl::space::S4);
            zone_label(ui, "ATTACHED WINDOWS");
            ui.label(
                RichText::new(format!(
                    "{} known · {} visible",
                    self.windows.records().len(),
                    self.windows.visible_count()
                ))
                .small()
                .color(cl::ink::MUTED),
            );
        });
    }

    fn council_ui(&self, ui: &mut egui::Ui) {
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(cl::icon::BOT)
                    .size(22.0)
                    .color(Tone::Lime.color()),
            );
            ui.label(RichText::new("Council").heading().color(cl::ink::STRONG));
        });
        ui.add_space(cl::space::S4);
        cl::clay_frame().show(ui, |ui| {
            ui.label(
                RichText::new("Deliberately unavailable")
                    .strong()
                    .color(Tone::Amber.color()),
            );
            ui.label(
                RichText::new(
                    "Council is CH9. Each provider must authenticate, run, cancel, recover, and preserve approvals alone before cross-provider coordination exists.",
                )
                .color(cl::ink::BASE),
            );
            ui.add_space(cl::space::S2);
            ui.label(
                RichText::new(
                    "No hidden orchestration, automatic handoff, or simulated success is active.",
                )
                .small()
                .color(cl::ink::MUTED),
            );
        });
    }
}

impl Hub for ChatHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        self.drain_shell_events();
        match route.sub {
            SESSIONS => self.sessions_ui(ui),
            COUNCIL => self.council_ui(ui),
            _ => {
                if let Some(go) = dpl_hub::placeholder::surface(
                    ui,
                    &SPEC,
                    route,
                    "Unknown Chat route. Choose a registered Chat surface.",
                ) {
                    if let Some(fabric) = &self.fabric {
                        fabric.publish_shell(dpl_fabric::ShellEvent::NavigateRequest { route: go });
                    }
                }
            }
        }
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

fn activation_color(state: &ActivationState, tone: Tone) -> egui::Color32 {
    match state {
        ActivationState::Ready | ActivationState::Experimental { .. } => tone.color(),
        ActivationState::SigningIn => Tone::Amber.color(),
        ActivationState::Failed { .. } | ActivationState::VersionUnsupported { .. } => {
            Tone::Coral.color()
        }
        _ => cl::ink::MUTED,
    }
}

fn zone_label(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .small()
            .monospace()
            .color(cl::ink::SUBTLE),
    );
}

fn transcript_controls(
    ui: &mut egui::Ui,
    activation: &ActivationState,
    tone: Tone,
    next_cursor: Option<HistoryCursor>,
    capacity_reached: bool,
    source_bytes: Option<u64>,
    load_earlier: &mut bool,
) {
    ui.label(
        RichText::new(activation.label())
            .small()
            .monospace()
            .color(activation_color(activation, tone)),
    );
    if next_cursor.is_some() || capacity_reached {
        let button = ui.add_enabled(
            !capacity_reached,
            egui::Button::new(if capacity_reached {
                "Window limit reached"
            } else {
                "Load earlier"
            }),
        );
        let button = cl::tip(
            button,
            if capacity_reached {
                "The bounded 1,000-event window is full"
            } else {
                "Prepend one bounded history page"
            },
        );
        ui.pilot_track(
            &button,
            NodeMetadata::new(
                PILOT_LOAD_EARLIER,
                egui_pilot_ai::protocol::Role::Button,
                "Load earlier Chat history",
            )
            .parent(PILOT_TRANSCRIPT),
        );
        *load_earlier = button.clicked();
    }
    if let Some(bytes) = source_bytes {
        ui.label(
            RichText::new(format!("{} SOURCE", format_byte_count(bytes)))
                .small()
                .monospace()
                .color(cl::ink::SUBTLE),
        );
    }
}

fn format_byte_count(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    match bytes {
        value if value >= GIB => format!("{:.1} GiB", value as f64 / GIB as f64),
        value if value >= MIB => format!("{:.1} MiB", value as f64 / MIB as f64),
        value if value >= KIB => format!("{:.1} KiB", value as f64 / KIB as f64),
        value => format!("{value} B"),
    }
}

fn activation_panel(
    ui: &mut egui::Ui,
    state: &ActivationState,
    manifest: ProviderManifest,
    tone: Tone,
) {
    ui.add_space(cl::space::S5);
    ui.vertical_centered(|ui| {
        ui.add(
            egui::Label::new(
                RichText::new(state.label())
                    .size(22.0)
                    .color(activation_color(state, tone)),
            )
            .wrap(),
        );
        if let Some(reason) = state.reason() {
            ui.add(egui::Label::new(RichText::new(reason).color(cl::ink::BASE)).wrap());
        }
        for action in state.recovery() {
            ui.add(
                egui::Label::new(
                    RichText::new(&action.label)
                        .small()
                        .monospace()
                        .color(cl::ink::MUTED),
                )
                .wrap(),
            );
        }
        ui.add_space(cl::space::S3);
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{} target capabilities · none active before conformance",
                    manifest.target_capabilities.len()
                ))
                .small()
                .color(cl::ink::SUBTLE),
            )
            .wrap(),
        );
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::ProviderId;
    use dpl_chat_runtime::ProviderRegistration;

    #[test]
    fn provider_accent_mapping_is_total() {
        for accent in [
            ProviderAccent::Amber,
            ProviderAccent::Cyan,
            ProviderAccent::Violet,
            ProviderAccent::Magenta,
            ProviderAccent::Lime,
        ] {
            let _ = provider_tone(accent);
        }
    }

    #[test]
    fn fixture_surface_renders_at_wide_and_compact_sizes() {
        for size in [egui::vec2(1280.0, 800.0), egui::vec2(680.0, 720.0)] {
            let runtime = ChatRuntime::try_from_registrations([
                dpl_chat_testkit::ready_provider(),
                ProviderRegistration::planned(dpl_chat_testkit::FIXTURE_MANIFEST),
            ]);
            assert!(runtime.is_err(), "duplicates must remain a hard failure");

            let runtime = ChatRuntime::try_from_registrations([dpl_chat_testkit::ready_provider()])
                .expect("fixture registry");
            let hub = ChatHub::new(runtime);
            let mut harness = egui_kittest::Harness::builder()
                .with_size(size)
                .build_ui_state(
                    |ui, hub: &mut ChatHub| {
                        hub.ui(
                            ui,
                            Route {
                                hub: HUB,
                                sub: SESSIONS,
                            },
                        );
                    },
                    hub,
                );
            harness.run_steps(3);
        }
    }

    #[test]
    fn council_is_an_explicit_surface_not_a_fake_provider() {
        let hub = ChatHub::default();
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(900.0, 600.0))
            .build_ui_state(
                |ui, hub: &mut ChatHub| {
                    hub.ui(
                        ui,
                        Route {
                            hub: HUB,
                            sub: COUNCIL,
                        },
                    );
                },
                hub,
            );
        harness.run_steps(2);
    }

    #[test]
    fn route_signals_offer_context_but_never_attach_it() {
        let fabric = dpl_fabric::Fabric::new();
        let runtime = ChatRuntime::try_from_registrations([dpl_chat_testkit::ready_provider()])
            .expect("fixture registry");
        let mut hub = ChatHub::new(runtime);
        hub.wire_fabric(&fabric);
        let source_route = Route {
            hub: HubId("workbench"),
            sub: SubId("library"),
        };
        fabric.publish_shell(dpl_fabric::ShellEvent::RouteChanged {
            route: source_route,
        });

        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1200.0, 760.0))
            .build_ui_state(
                |ui, hub: &mut ChatHub| {
                    hub.ui(
                        ui,
                        Route {
                            hub: HUB,
                            sub: SESSIONS,
                        },
                    );
                },
                hub,
            );
        harness.run_steps(2);

        let state = harness.state();
        assert_eq!(
            state.route_context.offered().expect("route offer").route,
            source_route
        );
        assert!(
            state
                .route_context
                .attached(dpl_chat_testkit::FIXTURE_PROVIDER)
                .is_none(),
            "route observation must not attach context"
        );
    }

    #[test]
    fn provider_ids_remain_typed_at_the_ui_boundary() {
        let provider = ProviderId("fixture");
        assert_eq!(provider.0, "fixture");
    }

    #[test]
    fn primary_chat_controls_have_stable_pilot_semantics() {
        let runtime = ChatRuntime::try_from_registrations([dpl_chat_testkit::ready_provider()])
            .expect("fixture registry");
        let hub = ChatHub::new(runtime);
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1200.0, 760.0))
            .build_ui_state(
                |ui, hub: &mut ChatHub| {
                    hub.ui(
                        ui,
                        Route {
                            hub: HUB,
                            sub: SESSIONS,
                        },
                    );
                },
                hub,
            );
        egui_pilot_ai::install(&harness.ctx);
        harness.run_steps(3);

        let pilot = egui_pilot_ai::handle(&harness.ctx).expect("pilot installed");
        let frame = pilot.latest_frame();
        for expected in [
            "root/chat.provider/fixture",
            "root/chat.session/chat-1",
            "root/chat.transcript",
            "root/chat.composer",
            "root/chat.send",
        ] {
            assert!(
                frame
                    .nodes
                    .iter()
                    .any(|node| node.id == egui_pilot_ai::protocol::NodeId::new(expected)),
                "missing {expected}"
            );
        }
        assert!(
            frame.diagnostics.duplicate_ids.is_empty(),
            "Chat semantics must remain unique"
        );

        let snapshot = pilot
            .snapshot(&egui_pilot_ai::protocol::SnapshotRequest::default())
            .expect("agent snapshot");
        for expected in [
            "root/chat.session/chat-1",
            "root/chat.event/event-1",
            "root/chat.event/event-2",
        ] {
            assert!(
                snapshot
                    .nodes
                    .iter()
                    .any(|node| node.id == egui_pilot_ai::protocol::NodeId::new(expected)),
                "agent snapshot omitted {expected}"
            );
        }
    }
}
