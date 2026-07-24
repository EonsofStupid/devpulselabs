use dpl_chat_runtime::ChatRuntime;
use dpl_chat_store::HistoryCatalog;
use dpl_chat_window::{ChatWindow, HostIntent};
use dpl_chat_windows::{
    ChatCloseDecision, ChatWindowId, ChatWindowRegistry, PeerLifetime, RootCloseDecision,
    WindowBinding, WindowLifecycle,
};
use dpl_fabric::{AppWindowAction, Fabric, ShellEvent, ShellReceiver};
use dpl_hub::Route;
use egui_pilot_ai::ContextPilotExt;
use std::sync::{Arc, Mutex, MutexGuard};

pub fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("devpulse.chat.primary")
}

struct Shared {
    registry: ChatWindowRegistry,
    lifetime: PeerLifetime,
    window: ChatWindow,
    /// A fabric handle for PUBLISHING the peer window's own navigation asks.
    /// The coordinator publishes `ShellEvent::NavigateRequest`; the shell
    /// drains it and navigates the main window.
    fabric: Fabric,
}

/// The sole owner of primary Chat viewport creation and peer lifetime policy.
pub struct ChatWindowCoordinator {
    shared: Arc<Mutex<Shared>>,
    events: ShellReceiver,
}

impl ChatWindowCoordinator {
    pub fn new(
        fabric: &Fabric,
        runtime: ChatRuntime,
        initial_route: Route,
        catalog: HistoryCatalog,
        routes: Vec<(String, Route)>,
        open_at_boot: bool,
    ) -> Self {
        let mut registry = ChatWindowRegistry::default();
        if open_at_boot {
            let id = registry.request_primary(WindowBinding::default());
            transition_opening(&mut registry, id);
        }
        Self {
            shared: Arc::new(Mutex::new(Shared {
                registry,
                lifetime: PeerLifetime::default(),
                window: ChatWindow::new(runtime, initial_route, catalog, routes),
                fabric: fabric.clone(),
            })),
            events: fabric.subscribe_shell(),
        }
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
        if ctx.pilot_take_viewport_close_request(viewport_id()) {
            close_chat(ctx, &self.shared);
        }

        let geometry = {
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
                .map(|record| record.geometry)
        };
        let Some(geometry) = geometry else {
            return;
        };

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
            viewport_id(),
            builder,
            move |ui, class| {
                if class == egui::ViewportClass::EmbeddedWindow {
                    embedded_failure(ui, &shared);
                    return;
                }

                let viewport = ui.ctx().input(|input| input.viewport().clone());
                let close_requested = viewport.close_requested();
                let geometry = viewport_geometry(&viewport);

                let intents = {
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
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::new()
                                .fill(dpl_coldlight::surface::WELL)
                                .inner_margin(egui::Margin::ZERO),
                        )
                        .show(ui, |ui| state.window.ui(ui))
                        .inner
                };

                let mut host_intents = Vec::new();
                {
                    let mut state = lock(&shared);
                    for intent in intents {
                        match state.window.apply(intent) {
                            Ok(Some(host)) => host_intents.push(host),
                            Ok(None) => {}
                            Err(error) => {
                                tracing::warn!(%error, "Chat window intent was rejected");
                            }
                        }
                    }
                }
                for intent in host_intents {
                    apply_host_intent(ui.ctx(), &shared, intent);
                }

                if close_requested {
                    close_chat(ui.ctx(), &shared);
                }
            },
        );
    }

    fn open_or_focus(&self, ctx: &egui::Context) {
        {
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
        }
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
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
        }
        HostIntent::QuitApplication => {
            lock(shared).lifetime.request_quit();
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
    }
}

fn close_chat(ctx: &egui::Context, shared: &Arc<Mutex<Shared>>) {
    let decision = {
        let mut state = lock(shared);
        if let Err(error) = state.registry.request_close(ChatWindowId::PRIMARY) {
            tracing::error!(%error, "Chat viewport failed to enter closing state");
        }
        if let Err(error) = state.registry.mark_closed(ChatWindowId::PRIMARY) {
            tracing::error!(%error, "Chat viewport failed to close");
        }
        state.lifetime.on_chat_close()
    };
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
