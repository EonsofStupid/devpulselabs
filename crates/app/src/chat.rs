#[cfg(feature = "eguipilotai-devtools")]
use dpl_chat_core::ChatDomain;
use dpl_chat_core::ProviderId;
use dpl_chat_interactions::{
    InteractionContent, InteractionId, InteractionPlacement, InteractionRegistry,
};
use dpl_chat_runtime::ChatRuntime;
use dpl_chat_social::SocialModel;
use dpl_chat_store::HistoryCatalog;
use dpl_chat_window::{ChatWindow, HostIntent};
use dpl_chat_windows::{
    ChatCloseDecision, ChatWindowId, ChatWindowRecord, ChatWindowRegistry, PeerLifetime,
    RootCloseDecision, SnapEdge, ViewportGeneration, WindowAttachment, WindowBinding,
    WindowGeometry, WindowKind, WindowLifecycle,
};
use dpl_fabric::{AppWindowAction, Fabric, ShellEvent, ShellReceiver};
use dpl_hub::Route;
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    ContextPilotExt, NodeMetadata, PilotId, UiPilotExt,
};
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(feature = "eguipilotai-devtools")]
type PilotRuntime = egui_pilot_ai_runtime::RuntimeGuard<egui_pilot_ai_transport::StdioTransport>;

#[cfg(feature = "eguipilotai-devtools")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatProofMode {
    Approval,
    Ai,
    Social,
    ProviderHover,
    GrokLogin,
    Compact,
    Snapped,
}

#[cfg(feature = "eguipilotai-devtools")]
impl ChatProofMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "approval" => Some(Self::Approval),
            "ai" => Some(Self::Ai),
            "social" => Some(Self::Social),
            "hover" => Some(Self::ProviderHover),
            "grok" => Some(Self::GrokLogin),
            "compact" => Some(Self::Compact),
            "snapped" => Some(Self::Snapped),
            _ => None,
        }
    }
}

fn primary_viewport_id(generation: ViewportGeneration) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("devpulse.chat.primary", generation.0))
}

struct Shared {
    registry: ChatWindowRegistry,
    lifetime: PeerLifetime,
    window: ChatWindow,
    codex_activation: dpl_chat_codex::CodexActivationCoordinator,
    grok_activation: dpl_chat_grok::GrokActivationCoordinator,
    primary_monitor: Option<([u32; 2], f32)>,
    /// A fabric handle for PUBLISHING the peer window's own navigation asks.
    /// The coordinator publishes `ShellEvent::NavigateRequest`; the shell
    /// drains it and navigates the main window.
    fabric: Fabric,
    #[cfg(feature = "eguipilotai-devtools")]
    pilot_runtime: Option<Arc<Mutex<PilotRuntime>>>,
}

/// The sole owner of primary Chat viewport creation and peer lifetime policy.
pub struct ChatWindowCoordinator {
    shared: Arc<Mutex<Shared>>,
    events: ShellReceiver,
}

pub struct ChatWindowConfig {
    pub initial_route: Route,
    pub catalog: HistoryCatalog,
    pub routes: Vec<(String, Route)>,
    pub authored_models: Option<(SocialModel, InteractionRegistry)>,
    pub codex_activation: dpl_chat_codex::CodexActivationCoordinator,
    pub grok_activation: dpl_chat_grok::GrokActivationCoordinator,
    pub open_at_boot: bool,
    #[cfg(feature = "eguipilotai-devtools")]
    pub proof_mode: Option<ChatProofMode>,
}

impl ChatWindowCoordinator {
    pub fn new(fabric: &Fabric, runtime: ChatRuntime, config: ChatWindowConfig) -> Self {
        let ChatWindowConfig {
            initial_route,
            catalog,
            routes,
            authored_models,
            codex_activation,
            grok_activation,
            open_at_boot,
            #[cfg(feature = "eguipilotai-devtools")]
            proof_mode,
        } = config;
        let mut registry = ChatWindowRegistry::default();
        if open_at_boot {
            let id = registry.request_primary(WindowBinding::default());
            #[cfg(feature = "eguipilotai-devtools")]
            if proof_mode == Some(ChatProofMode::Compact) {
                registry
                    .update_geometry(id, Some([32, 32]), WindowGeometry::MINIMUM)
                    .expect("the authored compact proof geometry is valid");
            }
            transition_opening(&mut registry, id);
        }
        let window = match authored_models {
            Some((social, interactions)) => ChatWindow::with_models(
                runtime,
                initial_route,
                catalog,
                routes,
                social,
                interactions,
            ),
            None => ChatWindow::new(runtime, initial_route, catalog, routes),
        };
        #[cfg(feature = "eguipilotai-devtools")]
        let mut window = window;
        #[cfg(feature = "eguipilotai-devtools")]
        if let Some(mode) = proof_mode {
            if mode != ChatProofMode::Approval {
                window
                    .apply(dpl_chat_window::ChatWindowIntent::DismissInteraction(
                        dpl_chat_testkit::AUTHORED_APPROVAL_ID,
                    ))
                    .expect("the authored approval exists");
            }
            match mode {
                ChatProofMode::Social => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SelectDomain(
                            ChatDomain::Social,
                        ))
                        .expect("the authored Social domain is valid");
                }
                ChatProofMode::ProviderHover => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SetOverlay(Some(
                            dpl_chat_window::DomainOverlay::Provider(ProviderId("codex")),
                        )))
                        .expect("the authored Codex provider exists");
                }
                ChatProofMode::GrokLogin => {
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SelectProvider(
                            dpl_chat_grok::PROVIDER,
                        ))
                        .expect("the authored Grok provider exists");
                    window
                        .apply(dpl_chat_window::ChatWindowIntent::SetOverlay(Some(
                            dpl_chat_window::DomainOverlay::Provider(dpl_chat_grok::PROVIDER),
                        )))
                        .expect("the authored Grok provider overlay is valid");
                }
                ChatProofMode::Approval
                | ChatProofMode::Ai
                | ChatProofMode::Compact
                | ChatProofMode::Snapped => {}
            }
        }
        if open_at_boot {
            if let Some((interaction_id, placement)) = window.first_utility_interaction() {
                request_utility_record(&mut registry, interaction_id, placement);
            }
        }
        let primary_monitor = dpl_engine_capture::list_monitors()
            .into_iter()
            .find(|monitor| monitor.is_primary)
            .map(|monitor| {
                (
                    [monitor.width, monitor.height],
                    monitor.scale_factor.max(1.0),
                )
            });
        Self {
            shared: Arc::new(Mutex::new(Shared {
                registry,
                lifetime: PeerLifetime::default(),
                window,
                codex_activation,
                grok_activation,
                primary_monitor,
                fabric: fabric.clone(),
                #[cfg(feature = "eguipilotai-devtools")]
                pilot_runtime: None,
            })),
            events: fabric.subscribe_shell(),
        }
    }

    #[cfg(feature = "eguipilotai-devtools")]
    pub fn attach_pilot_runtime(&self, runtime: Arc<Mutex<PilotRuntime>>) {
        lock(&self.shared).pilot_runtime = Some(runtime);
    }

    #[cfg(feature = "eguipilotai-devtools")]
    pub fn consume_pilot_close_requests(&self, ctx: &egui::Context) {
        consume_pilot_close_requests(ctx, &self.shared);
    }

    pub fn root_close_requested(&self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        let decision = {
            let mut shared = lock(&self.shared);
            let chat_live = shared.registry.is_live(ChatWindowId::PRIMARY);
            shared.lifetime.on_root_close(chat_live)
        };
        if decision == RootCloseDecision::HideMain {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    pub fn frame(&mut self, ctx: &egui::Context, current_route: Route) {
        let mut open_requested = false;
        {
            let mut shared = lock(&self.shared);
            shared.window.observe_route(current_route);
            while let Ok(event) = self.events.try_recv() {
                match event {
                    ShellEvent::RouteChanged { route } => shared.window.observe_route(route),
                    ShellEvent::WindowRequest {
                        target,
                        action: AppWindowAction::OpenOrFocus,
                    } if target == dpl_chat_window::WINDOW_KEY => open_requested = true,
                    ShellEvent::ViziwizSummon
                    | ShellEvent::FullBleedRequest(_)
                    | ShellEvent::NavigateRequest { .. }
                    | ShellEvent::WindowRequest { .. } => {}
                }
            }
        }
        if open_requested {
            self.open_or_focus(ctx);
        }

        let primary = {
            let shared = lock(&self.shared);
            shared
                .registry
                .record(ChatWindowId::PRIMARY)
                .filter(|record| {
                    matches!(
                        record.lifecycle,
                        WindowLifecycle::Opening | WindowLifecycle::Visible
                    )
                })
                .cloned()
        };
        let Some(primary) = primary else {
            return;
        };
        let generation = primary.generation;
        let chat_viewport_id = primary_viewport_id(generation);
        if ctx.pilot_take_viewport_close_request(chat_viewport_id) {
            close_chat(ctx, &self.shared, generation);
            return;
        }
        let geometry = primary.geometry;

        let mut builder = egui::ViewportBuilder::default()
            .with_title("DevPULSE Chat")
            .with_inner_size([geometry.inner_size[0] as f32, geometry.inner_size[1] as f32])
            .with_min_inner_size([
                dpl_chat_windows::WindowGeometry::MINIMUM[0] as f32,
                dpl_chat_windows::WindowGeometry::MINIMUM[1] as f32,
            ])
            .with_resizable(true)
            .with_decorations(true)
            .with_taskbar(true);
        if let Some(position) = geometry.outer_position {
            builder = builder.with_position([position[0] as f32, position[1] as f32]);
        }

        let shared = Arc::clone(&self.shared);
        ctx.pilot_show_viewport_deferred(
            dpl_chat_window::viewport_key(),
            chat_viewport_id,
            builder,
            move |ui, class| {
                if class == egui::ViewportClass::EmbeddedWindow {
                    embedded_failure(ui, &shared);
                    return;
                }
                let current_generation_is_live = {
                    let state = lock(&shared);
                    state
                        .registry
                        .record(ChatWindowId::PRIMARY)
                        .is_some_and(|record| {
                            record.generation == generation
                                && matches!(
                                    record.lifecycle,
                                    WindowLifecycle::Opening | WindowLifecycle::Visible
                                )
                        })
                };
                if !current_generation_is_live {
                    return;
                }

                let viewport = ui.ctx().input(|input| input.viewport().clone());
                let close_requested = viewport.close_requested();
                let geometry = viewport_geometry(&viewport);

                let (intents, activation_changed, login_running) = {
                    let mut state = lock(&shared);
                    if matches!(
                        state
                            .registry
                            .record(ChatWindowId::PRIMARY)
                            .map(|record| &record.lifecycle),
                        Some(WindowLifecycle::Opening)
                    ) {
                        if let Err(error) = state.registry.mark_visible(ChatWindowId::PRIMARY) {
                            tracing::error!(%error, "Chat viewport failed to become visible");
                        }
                    }
                    if let Some((position, size)) = geometry {
                        if let Err(error) =
                            state
                                .registry
                                .update_geometry(ChatWindowId::PRIMARY, position, size)
                        {
                            tracing::warn!(%error, "Chat viewport geometry was rejected");
                        }
                    }
                    let activation_changed = poll_provider_activations(&mut state);
                    let login_running = state.codex_activation.is_login_running()
                        || state.grok_activation.is_login_running();
                    let intents = egui::CentralPanel::default()
                        .frame(
                            egui::Frame::new()
                                .fill(dpl_coldlight::surface::WELL)
                                .inner_margin(egui::Margin::ZERO),
                        )
                        .show(ui, |ui| state.window.ui(ui))
                        .inner;
                    (intents, activation_changed, login_running)
                };

                let mut host_intents = Vec::new();
                let mut window_changed = false;
                {
                    let mut state = lock(&shared);
                    for intent in intents {
                        match state.window.apply(intent) {
                            Ok(Some(host)) => {
                                window_changed = true;
                                host_intents.push(host);
                            }
                            Ok(None) => window_changed = true,
                            Err(error) => {
                                tracing::warn!(%error, "Chat window intent was rejected");
                            }
                        }
                    }
                }
                for intent in host_intents {
                    apply_host_intent(ui.ctx(), &shared, intent);
                }
                if window_changed || activation_changed {
                    ui.ctx().request_repaint_of(chat_viewport_id);
                }
                if login_running {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(100));
                }

                if close_requested {
                    close_chat(ui.ctx(), &shared, generation);
                    return;
                }
                declare_utilities(ui.ctx(), &shared);
                #[cfg(feature = "eguipilotai-devtools")]
                {
                    pump_pilot_runtime(&shared);
                    consume_pilot_close_requests(ui.ctx(), &shared);
                }
            },
        );
    }

    fn open_or_focus(&self, ctx: &egui::Context) {
        let generation = {
            let mut shared = lock(&self.shared);
            match shared
                .registry
                .record(ChatWindowId::PRIMARY)
                .map(|record| record.lifecycle.clone())
            {
                None => {
                    let id = shared.registry.request_primary(WindowBinding::default());
                    transition_opening(&mut shared.registry, id);
                }
                Some(WindowLifecycle::Closed | WindowLifecycle::Failed { .. }) => {
                    if let Err(error) = shared.registry.reopen(ChatWindowId::PRIMARY) {
                        tracing::error!(%error, "Chat viewport failed to reopen");
                    } else {
                        transition_opening(&mut shared.registry, ChatWindowId::PRIMARY);
                    }
                }
                Some(WindowLifecycle::Requested) => {
                    transition_opening(&mut shared.registry, ChatWindowId::PRIMARY);
                }
                Some(
                    WindowLifecycle::Opening | WindowLifecycle::Visible | WindowLifecycle::Closing,
                ) => {}
            }
            current_generation(&shared.registry, ChatWindowId::PRIMARY)
        };
        let Some(generation) = generation else {
            return;
        };
        let chat_viewport_id = primary_viewport_id(generation);
        ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Focus);
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }
}

fn transition_opening(registry: &mut ChatWindowRegistry, id: ChatWindowId) {
    if let Err(error) = registry.mark_opening(id) {
        tracing::error!(%error, "Chat viewport failed to enter opening state");
    }
}

fn viewport_geometry(viewport: &egui::ViewportInfo) -> Option<(Option<[i32; 2]>, [u32; 2])> {
    let inner = viewport.inner_rect?;
    let width = inner.width().round();
    let height = inner.height().round();
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let position = viewport.outer_rect.and_then(|outer| {
        let x = outer.min.x.round();
        let y = outer.min.y.round();
        (x.is_finite() && y.is_finite()).then_some([x as i32, y as i32])
    });
    Some((position, [width as u32, height as u32]))
}

fn apply_host_intent(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>, intent: HostIntent) {
    match intent {
        HostIntent::OpenUtility {
            interaction_id,
            placement,
        } => open_utility(ctx, shared, interaction_id, placement),
        HostIntent::ActivateProvider(provider) => {
            activate_provider(shared, provider);
            ctx.request_repaint();
        }
        HostIntent::OpenProviderSetup(provider) => {
            let url = match provider {
                id if id == dpl_chat_codex::PROVIDER => dpl_chat_codex::SETUP_URL,
                id if id == dpl_chat_grok::PROVIDER => dpl_chat_grok::SETUP_URL,
                _ => {
                    tracing::warn!(%provider, "provider setup action is not qualified");
                    return;
                }
            };
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
        HostIntent::NavigateTo(route) => {
            // Route loading lives in the MAIN app: publish the ask and let the
            // shell drain it and navigate. Chat never renders the route itself.
            lock(shared)
                .fabric
                .publish_shell(ShellEvent::NavigateRequest { route });
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
        HostIntent::ShowMain => {
            lock(shared).lifetime.restore_main();
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::Minimized(false),
            );
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
            // Native restoration completes asynchronously. Wake the root so
            // its next input frame records that lifecycle transition before
            // an agent advances to a viewport assertion.
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
        HostIntent::QuitApplication => {
            lock(shared).lifetime.request_quit();
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
    }
}

fn activate_provider(shared: &Arc<Mutex<Shared>>, provider: ProviderId) {
    let mut state = lock(shared);
    let activation = if provider == dpl_chat_codex::PROVIDER {
        let result = state.codex_activation.start_login();
        if let Err(error) = result {
            tracing::warn!(%error, "ChatGPT / Codex activation was rejected");
            if matches!(
                &error,
                dpl_chat_codex::CodexActivationError::LaunchFailed(_)
                    | dpl_chat_codex::CodexActivationError::PollFailed(_)
                    | dpl_chat_codex::CodexActivationError::StatusFailed(_)
            ) {
                state.codex_activation.fail(error);
            }
        }
        state.codex_activation.activation().clone()
    } else if provider == dpl_chat_grok::PROVIDER {
        let result = state.grok_activation.start_login();
        if let Err(error) = result {
            tracing::warn!(%error, "Grok activation was rejected");
            if matches!(
                &error,
                dpl_chat_grok::GrokActivationError::LaunchFailed(_)
                    | dpl_chat_grok::GrokActivationError::PollFailed(_)
            ) {
                state.grok_activation.fail(error);
            }
        }
        state.grok_activation.activation().clone()
    } else {
        tracing::warn!(%provider, "provider activation action is not qualified");
        return;
    };
    if let Err(error) = state.window.set_provider_activation(provider, activation) {
        tracing::warn!(%error, "provider activation state could not be projected");
    }
}

fn poll_provider_activations(state: &mut Shared) -> bool {
    let codex_changed = match state.codex_activation.poll() {
        Ok(changed) => changed,
        Err(error) => {
            tracing::warn!(%error, "ChatGPT / Codex activation polling failed");
            state.codex_activation.fail(error);
            true
        }
    };
    if codex_changed {
        let activation = state.codex_activation.activation().clone();
        if let Err(error) = state
            .window
            .set_provider_activation(dpl_chat_codex::PROVIDER, activation)
        {
            tracing::warn!(%error, "ChatGPT / Codex activation state could not be projected");
        }
    }

    let grok_changed = match state.grok_activation.poll() {
        Ok(changed) => changed,
        Err(error) => {
            tracing::warn!(%error, "Grok activation polling failed");
            state.grok_activation.fail(error);
            true
        }
    };
    if grok_changed {
        let activation = state.grok_activation.activation().clone();
        if let Err(error) = state
            .window
            .set_provider_activation(dpl_chat_grok::PROVIDER, activation)
        {
            tracing::warn!(%error, "Grok activation state could not be projected");
        }
    }

    codex_changed || grok_changed
}

fn request_utility_record(
    registry: &mut ChatWindowRegistry,
    interaction_id: InteractionId,
    placement: InteractionPlacement,
) -> Option<ChatWindowId> {
    let attachment = attachment_for(placement);
    match registry.request_attached(
        WindowKind::Artifact,
        WindowBinding {
            interaction_id: Some(interaction_id),
            ..WindowBinding::default()
        },
        attachment,
    ) {
        Ok(id) => {
            transition_opening(registry, id);
            Some(id)
        }
        Err(error) => {
            tracing::error!(%error, "Chat utility registration failed");
            None
        }
    }
}

fn attachment_for(placement: InteractionPlacement) -> WindowAttachment {
    match placement {
        InteractionPlacement::DetachedUtility => WindowAttachment::Detached,
        InteractionPlacement::Inline
        | InteractionPlacement::Overlay
        | InteractionPlacement::SnappedUtility => WindowAttachment::Snapped {
            parent: ChatWindowId::PRIMARY,
            edge: SnapEdge::Right,
            offset: [8, 0],
        },
    }
}

fn open_utility(
    ctx: &egui::Context,
    shared: &Arc<Mutex<Shared>>,
    interaction_id: InteractionId,
    placement: InteractionPlacement,
) {
    let attachment = attachment_for(placement);
    let target = {
        let mut state = lock(shared);
        let existing = state
            .registry
            .records()
            .iter()
            .find(|record| record.binding.interaction_id == Some(interaction_id))
            .map(|record| (record.id, record.lifecycle.clone(), record.attachment));
        match existing {
            Some((id, lifecycle, current_attachment)) => {
                if current_attachment != Some(attachment) {
                    if let Err(error) = state.registry.set_attachment(id, attachment) {
                        tracing::error!(%error, "Chat utility attachment failed");
                        return;
                    }
                    transition_opening(&mut state.registry, id);
                } else if matches!(
                    lifecycle,
                    WindowLifecycle::Closed | WindowLifecycle::Failed { .. }
                ) {
                    if let Err(error) = state.registry.reopen(id) {
                        tracing::error!(%error, "Chat utility reopen failed");
                        return;
                    }
                    transition_opening(&mut state.registry, id);
                }
                state
                    .registry
                    .record(id)
                    .map(|record| (id, record.generation))
            }
            None => request_utility_record(&mut state.registry, interaction_id, placement)
                .and_then(|id| {
                    state
                        .registry
                        .record(id)
                        .map(|record| (id, record.generation))
                }),
        }
    };
    if let Some((id, generation)) = target {
        let viewport = utility_viewport_id(id, generation);
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

fn show_utility(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>, record: ChatWindowRecord) {
    let Some(interaction_id) = record.binding.interaction_id else {
        tracing::error!(window = %record.id, "Chat utility has no interaction binding");
        return;
    };
    let spec = {
        let state = lock(shared);
        state.window.interaction(interaction_id).cloned()
    };
    let Some(spec) = spec else {
        tracing::error!(window = %record.id, %interaction_id, "Chat utility interaction is missing");
        return;
    };
    let title = match &spec.content {
        InteractionContent::ArtifactPreview { title, .. }
        | InteractionContent::Notice { title, .. }
        | InteractionContent::Approval { title, .. } => title.clone(),
        InteractionContent::RouteResult { route_label, .. } => route_label.clone(),
    };
    let utility_id = utility_viewport_id(record.id, record.generation);
    let (viewport_monitor, native_pixels_per_point) = ctx.input(|input| {
        (
            input.viewport().monitor_size.map(|size| [size.x, size.y]),
            input.viewport().native_pixels_per_point,
        )
    });
    let pixels_per_point = native_pixels_per_point.unwrap_or_else(|| ctx.pixels_per_point());
    let capture_monitor = lock(shared)
        .primary_monitor
        .map(|([width, height], scale_factor)| {
            [
                width as f32 / pixels_per_point.max(scale_factor),
                height as f32 / pixels_per_point.max(scale_factor),
            ]
        });
    let monitor_size = match (viewport_monitor, capture_monitor) {
        (Some(viewport), Some(capture)) => {
            Some([viewport[0].min(capture[0]), viewport[1].min(capture[1])])
        }
        (Some(viewport), None) => Some(viewport),
        (None, Some(capture)) => Some(capture),
        (None, None) => None,
    };
    let mut builder = utility_builder(shared, &record, &title, monitor_size);
    if let Some(position) = record.geometry.outer_position {
        if matches!(record.attachment, Some(WindowAttachment::Detached)) {
            builder = builder.with_position([position[0] as f32, position[1] as f32]);
        }
    }
    let pilot_key = PilotViewportKey::new(format!(
        "chat.utility.{}.{}",
        record.id.0, record.generation.0
    ));
    let shared = Arc::clone(shared);
    ctx.pilot_show_viewport_immediate(pilot_key, utility_id, builder, move |ui, class| {
        if class == egui::ViewportClass::EmbeddedWindow {
            ui.colored_label(
                dpl_coldlight::Tone::Coral.color(),
                "This utility requires a native viewport.",
            );
            let mut state = lock(&shared);
            if current_generation(&state.registry, record.id) == Some(record.generation) {
                if let Err(error) = state
                    .registry
                    .mark_failed(record.id, "backend embedded Chat utility viewport")
                {
                    tracing::error!(%error, "Chat utility failure could not be recorded");
                }
            }
            return;
        }

        let viewport = ui.ctx().input(|input| input.viewport().clone());
        let close_requested = viewport.close_requested();
        let geometry = viewport_geometry(&viewport);
        {
            let mut state = lock(&shared);
            if current_generation(&state.registry, record.id) == Some(record.generation) {
                if matches!(
                    state
                        .registry
                        .record(record.id)
                        .map(|current| &current.lifecycle),
                    Some(WindowLifecycle::Opening)
                ) {
                    if let Err(error) = state.registry.mark_visible(record.id) {
                        tracing::error!(%error, "Chat utility failed to become visible");
                    }
                }
                if let Some((position, size)) = geometry {
                    if let Err(error) = state.registry.update_geometry(record.id, position, size) {
                        tracing::warn!(%error, "Chat utility geometry was rejected");
                    }
                }
            }
        }

        let action = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(dpl_coldlight::surface::WELL)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| utility_ui(ui, &record, &spec))
            .inner;
        if let Some(action) = action {
            apply_utility_action(ui.ctx(), &shared, &record, action);
        }
        if close_requested
            && current_generation(&lock(&shared).registry, record.id) == Some(record.generation)
        {
            close_utility(&shared, record.id, record.generation);
        }
        #[cfg(feature = "eguipilotai-devtools")]
        {
            pump_pilot_runtime(&shared);
            consume_pilot_close_requests(ui.ctx(), &shared);
        }
    });
}

fn declare_utilities(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>) {
    let utilities = {
        let state = lock(shared);
        state
            .registry
            .records()
            .iter()
            .filter(|record| {
                record.id != ChatWindowId::PRIMARY
                    && matches!(
                        record.lifecycle,
                        WindowLifecycle::Opening | WindowLifecycle::Visible
                    )
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    for record in utilities {
        let utility_id = utility_viewport_id(record.id, record.generation);
        if ctx.pilot_take_viewport_close_request(utility_id) {
            close_utility(shared, record.id, record.generation);
            ctx.pilot_retire_viewport(utility_id);
        } else {
            show_utility(ctx, shared, record);
        }
    }
}

#[cfg(feature = "eguipilotai-devtools")]
fn consume_pilot_close_requests(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>) {
    let primary_generation = {
        let state = lock(shared);
        current_generation(&state.registry, ChatWindowId::PRIMARY)
    };
    if let Some(generation) = primary_generation {
        if ctx.pilot_take_viewport_close_request(primary_viewport_id(generation)) {
            close_chat(ctx, shared, generation);
            return;
        }
    }
    let utilities = {
        let state = lock(shared);
        state
            .registry
            .records()
            .iter()
            .filter(|record| record.id != ChatWindowId::PRIMARY)
            .map(|record| (record.id, record.generation))
            .collect::<Vec<_>>()
    };
    for (id, generation) in utilities {
        let utility_id = utility_viewport_id(id, generation);
        if ctx.pilot_take_viewport_close_request(utility_id) {
            close_utility(shared, id, generation);
            ctx.pilot_retire_viewport(utility_id);
        }
    }
}

#[cfg(feature = "eguipilotai-devtools")]
fn pump_pilot_runtime(shared: &Arc<Mutex<Shared>>) {
    let runtime = lock(shared).pilot_runtime.clone();
    if let Some(runtime) = runtime {
        lock(&runtime).pump();
    }
}

fn utility_builder(
    shared: &Arc<Mutex<Shared>>,
    record: &ChatWindowRecord,
    title: &str,
    monitor_size: Option<[f32; 2]>,
) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default()
        .with_title(format!("DevPULSE Chat — {title}"))
        .with_inner_size([
            record.geometry.inner_size[0] as f32,
            record.geometry.inner_size[1] as f32,
        ])
        .with_min_inner_size([
            WindowGeometry::UTILITY_MINIMUM[0] as f32,
            WindowGeometry::UTILITY_MINIMUM[1] as f32,
        ])
        .with_resizable(true);
    match record.attachment {
        Some(WindowAttachment::Snapped {
            parent,
            edge,
            offset,
        }) => {
            let parent_geometry = lock(shared)
                .registry
                .record(parent)
                .map(|record| record.geometry)
                .unwrap_or(WindowGeometry::DEFAULT);
            let parent_position = parent_geometry.outer_position.unwrap_or([48, 48]);
            let preferred_x = match edge {
                SnapEdge::Left => {
                    parent_position[0] - record.geometry.inner_size[0] as i32 + offset[0]
                }
                SnapEdge::Right => {
                    parent_position[0] + parent_geometry.inner_size[0] as i32 + offset[0]
                }
            };
            let fallback_x = match edge {
                SnapEdge::Left => {
                    parent_position[0] + parent_geometry.inner_size[0] as i32 - offset[0]
                }
                SnapEdge::Right => {
                    parent_position[0] - record.geometry.inner_size[0] as i32 - offset[0]
                }
            };
            let mut x = preferred_x;
            let mut y = parent_position[1] + offset[1];
            if let Some([monitor_width, monitor_height]) = monitor_size {
                let utility_width = record.geometry.inner_size[0] as i32;
                let utility_height = record.geometry.inner_size[1] as i32;
                let max_x = (monitor_width as i32 - utility_width - 8).max(8);
                let max_y = (monitor_height as i32 - utility_height - 8).max(8);
                if x < 8 || x > max_x {
                    x = fallback_x;
                }
                x = x.clamp(8, max_x);
                y = y.clamp(8, max_y);
            }
            builder = builder
                .with_position([x as f32, y as f32])
                .with_decorations(false)
                .with_taskbar(false);
        }
        Some(WindowAttachment::Detached) | None => {
            builder = builder.with_decorations(true).with_taskbar(true);
        }
    }
    builder
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UtilityAction {
    TogglePin,
    Detach,
    Redock,
    FocusChat,
    Close,
}

fn utility_ui(
    ui: &mut egui::Ui,
    record: &ChatWindowRecord,
    spec: &dpl_chat_interactions::InteractionSpec,
) -> Option<UtilityAction> {
    let mut action = None;
    let root = ui.max_rect();
    ui.painter().rect_filled(
        root,
        egui::CornerRadius::ZERO,
        egui::Color32::from_rgb(4, 15, 18),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(root.shrink(10.0))
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("DEV PULSE / UTILITY")
                        .monospace()
                        .small()
                        .color(dpl_coldlight::Tone::Cyan.color()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if utility_button(ui, "chat.utility.close", "×", "Close utility").clicked() {
                        action = Some(UtilityAction::Close);
                    }
                    if utility_button(ui, "chat.utility.focus-origin", "↙", "Focus Chat").clicked()
                    {
                        action = Some(UtilityAction::FocusChat);
                    }
                    match record.attachment {
                        Some(WindowAttachment::Snapped { .. }) => {
                            if utility_button(ui, "chat.utility.detach", "□", "Detach utility")
                                .clicked()
                            {
                                action = Some(UtilityAction::Detach);
                            }
                        }
                        Some(WindowAttachment::Detached) | None => {
                            if utility_button(ui, "chat.utility.redock", "▥", "Re-dock utility")
                                .clicked()
                            {
                                action = Some(UtilityAction::Redock);
                            }
                        }
                    }
                    if utility_button(
                        ui,
                        "chat.utility.pin",
                        if record.pinned { "●" } else { "○" },
                        "Pin utility",
                    )
                    .clicked()
                    {
                        action = Some(UtilityAction::TogglePin);
                    }
                });
            });
            ui.separator();
            match &spec.content {
                InteractionContent::ArtifactPreview {
                    title,
                    artifact_kind,
                    summary,
                } => {
                    ui.label(
                        egui::RichText::new(artifact_kind)
                            .monospace()
                            .small()
                            .color(dpl_coldlight::Tone::Amber.color()),
                    );
                    ui.label(
                        egui::RichText::new(title)
                            .strong()
                            .size(22.0)
                            .color(dpl_coldlight::ink::STRONG),
                    );
                    ui.label(egui::RichText::new(summary).color(dpl_coldlight::ink::MUTED));
                    ui.add_space(14.0);
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(8, 27, 30))
                        .stroke(egui::Stroke::new(1.0, dpl_coldlight::Tone::Cyan.color()))
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Library / Slide templates")
                                    .strong()
                                    .color(dpl_coldlight::ink::STRONG),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Observed route preview · context remains unattached",
                                )
                                .small()
                                .color(dpl_coldlight::ink::MUTED),
                            );
                        });
                }
                InteractionContent::RouteResult {
                    route_label,
                    detail,
                    ..
                } => {
                    ui.heading(route_label);
                    ui.label(detail);
                }
                InteractionContent::Notice { title, body } => {
                    ui.heading(title);
                    ui.label(body);
                }
                InteractionContent::Approval { title, summary, .. } => {
                    ui.heading(title);
                    ui.label(summary);
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} · generation {} · session preserved",
                        record.id, record.generation.0
                    ))
                    .monospace()
                    .small()
                    .color(dpl_coldlight::ink::SUBTLE),
                );
            });
        },
    );
    action
}

fn utility_button(
    ui: &mut egui::Ui,
    id: &'static str,
    label: &str,
    description: &str,
) -> egui::Response {
    let response = ui.small_button(label).on_hover_text(description);
    ui.pilot_track(
        &response,
        NodeMetadata::new(PilotId::new(id), Role::Button, description),
    );
    response
}

fn apply_utility_action(
    ctx: &egui::Context,
    shared: &Arc<Mutex<Shared>>,
    record: &ChatWindowRecord,
    action: UtilityAction,
) {
    match action {
        UtilityAction::TogglePin => {
            if let Err(error) = lock(shared).registry.toggle_pinned(record.id) {
                tracing::warn!(%error, "Chat utility pin failed");
            }
        }
        UtilityAction::Detach | UtilityAction::Redock => {
            let attachment = if action == UtilityAction::Detach {
                WindowAttachment::Detached
            } else {
                WindowAttachment::Snapped {
                    parent: ChatWindowId::PRIMARY,
                    edge: SnapEdge::Right,
                    offset: [8, 0],
                }
            };
            let mut state = lock(shared);
            if current_generation(&state.registry, record.id) != Some(record.generation) {
                return;
            }
            match state.registry.set_attachment(record.id, attachment) {
                Ok(true) => {
                    transition_opening(&mut state.registry, record.id);
                }
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "Chat utility move failed"),
            }
            drop(state);
            ctx.pilot_retire_viewport(utility_viewport_id(record.id, record.generation));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            if let Some(chat_viewport_id) = live_primary_viewport_id(shared) {
                ctx.request_repaint_of(chat_viewport_id);
            }
        }
        UtilityAction::FocusChat => {
            if let Some(chat_viewport_id) = live_primary_viewport_id(shared) {
                ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd_to(chat_viewport_id, egui::ViewportCommand::Focus);
            }
        }
        UtilityAction::Close => {
            close_utility(shared, record.id, record.generation);
            ctx.pilot_retire_viewport(utility_viewport_id(record.id, record.generation));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            if let Some(chat_viewport_id) = live_primary_viewport_id(shared) {
                ctx.request_repaint_of(chat_viewport_id);
            }
        }
    }
}

fn close_utility(shared: &Arc<Mutex<Shared>>, id: ChatWindowId, generation: ViewportGeneration) {
    let mut state = lock(shared);
    if current_generation(&state.registry, id) != Some(generation) {
        return;
    }
    if let Err(error) = state.registry.request_close(id) {
        tracing::warn!(%error, "Chat utility failed to enter closing state");
        return;
    }
    if let Err(error) = state.registry.mark_closed(id) {
        tracing::warn!(%error, "Chat utility failed to close");
    }
}

fn current_generation(
    registry: &ChatWindowRegistry,
    id: ChatWindowId,
) -> Option<ViewportGeneration> {
    registry.record(id).map(|record| record.generation)
}

fn live_primary_viewport_id(shared: &Arc<Mutex<Shared>>) -> Option<egui::ViewportId> {
    let state = lock(shared);
    state
        .registry
        .record(ChatWindowId::PRIMARY)
        .filter(|record| {
            matches!(
                record.lifecycle,
                WindowLifecycle::Opening | WindowLifecycle::Visible
            )
        })
        .map(|record| primary_viewport_id(record.generation))
}

fn utility_viewport_id(id: ChatWindowId, generation: ViewportGeneration) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("devpulse.chat.utility", id.0, generation.0))
}

fn close_chat(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>, generation: ViewportGeneration) {
    let decision = {
        let mut state = lock(shared);
        let Some(record) = state.registry.record(ChatWindowId::PRIMARY) else {
            return;
        };
        if record.generation != generation {
            return;
        }
        let lifecycle = record.lifecycle.clone();
        if !matches!(
            lifecycle,
            WindowLifecycle::Closing | WindowLifecycle::Closed
        ) {
            if let Err(error) = state.registry.request_close(ChatWindowId::PRIMARY) {
                tracing::error!(%error, "Chat viewport failed to enter closing state");
                return;
            }
        }
        if !matches!(lifecycle, WindowLifecycle::Closed) {
            if let Err(error) = state.registry.mark_closed(ChatWindowId::PRIMARY) {
                tracing::error!(%error, "Chat viewport failed to close");
                return;
            }
        }
        state.lifetime.on_chat_close()
    };
    ctx.pilot_retire_viewport(primary_viewport_id(generation));
    if decision == ChatCloseDecision::Exit {
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
    } else {
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }
}

fn embedded_failure(ui: &mut egui::Ui, shared: &Arc<Mutex<Shared>>) {
    ui.colored_label(
        dpl_coldlight::Tone::Coral.color(),
        "Chat requires native multi-viewport support; this backend embedded the peer window.",
    );
    let mut state = lock(shared);
    if let Err(error) = state.registry.mark_failed(
        ChatWindowId::PRIMARY,
        "backend embedded the primary Chat viewport",
    ) {
        tracing::error!(%error, "Chat embedded-viewport failure could not be recorded");
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
