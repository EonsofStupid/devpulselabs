//! The coach touchpoint store and its in-app renderer.
//!
//! A touchpoint is a mounted matcher plus a visual and a trigger contract.
//! The agent mounts one, the operator (or the agent, demonstrating) activates
//! it, and the store answers with typed lifecycle events. Nothing here knows
//! about a transport: the store is drained by whoever owns the runtime.
//!
//! The renderer paints, it never intercepts. A spotlight is drawn on a
//! foreground layer with no interaction area at all, so the press the
//! operator is being coached to make still reaches the real widget. The
//! trigger is observed from the same frame's pointer events.

use crate::protocol::{
    CoachActor, CoachEvent, CoachId, CoachRefusal, CoachUnmountReason, NodeId, SemanticNode,
    SignalId, TriggerKind,
};
use std::collections::VecDeque;

/// How many recent app signals stay observable. Touchpoints read this ring by
/// sequence rather than draining it, so every viewport's pass in a frame sees
/// the same signals regardless of which one runs first.
const SIGNAL_RING: usize = 64;

/// Host-supplied paint tokens. The toolkit ships a neutral default; DevPULSE
/// hands it the generated ColdLight tones so a coached surface looks like the
/// rest of the app.
#[derive(Debug, Clone)]
pub struct CoachPalette {
    /// The crisp edge around the target.
    pub ring: egui::Color32,
    pub ring_width: f32,
    /// The gradient tint head painted inside the target; it fades to nothing.
    pub tint: egui::Color32,
    /// Black diffusion scatter dimming everything outside the target.
    pub scatter: egui::Color32,
    pub caption_plate: egui::Color32,
    pub caption_edge: egui::Color32,
    pub caption_ink: egui::Color32,
    pub caption_font: egui::FontId,
    pub corner_radius: u8,
}

impl Default for CoachPalette {
    fn default() -> Self {
        Self {
            ring: egui::Color32::from_rgb(0x38, 0xd9, 0xe8),
            ring_width: 2.0,
            tint: egui::Color32::from_rgba_unmultiplied(0x38, 0xd9, 0xe8, 36),
            scatter: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 132),
            caption_plate: egui::Color32::from_rgb(0x11, 0x14, 0x1d),
            caption_edge: egui::Color32::from_rgb(0x23, 0x28, 0x36),
            caption_ink: egui::Color32::from_rgb(0xee, 0xf2, 0xf9),
            caption_font: egui::FontId::proportional(13.0),
            corner_radius: 6,
        }
    }
}

/// One mount request.
///
/// `viewport`/`rect` are `None` for a touchpoint that mounts **armed** —
/// waiting for a target that is not on screen yet. Any viewport's pass may
/// then resolve it and bind it.
#[derive(Debug, Clone)]
pub struct CoachMount {
    pub coach_id: CoachId,
    pub node: NodeId,
    pub viewport: Option<egui::ViewportId>,
    pub rect: Option<crate::protocol::Rect>,
    pub on: TriggerKind,
    pub message: Option<String>,
    pub ttl_ms: Option<u64>,
    pub once: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MountedTouchpoint {
    id: CoachId,
    node: NodeId,
    viewport: Option<egui::ViewportId>,
    on: TriggerKind,
    message: Option<String>,
    ttl_ms: Option<u64>,
    once: bool,
    /// Target clock seconds, stamped the first pass that observes it — armed
    /// or not, so a touchpoint waiting for a target still honours its TTL.
    started_at: Option<f64>,
    /// Whether the target was on screen in the previous pass. The
    /// absent → present edge is what `NodeAppeared` fires on.
    present: bool,
    /// Whether the pointer was inside the target in the previous pass.
    hovered: bool,
    /// The value the target reported when first observed. `None` means not
    /// observed yet, which is distinct from a node whose value is `None`.
    value: Option<Option<String>>,
    /// The newest signal sequence this touchpoint has already accounted for.
    signals_seen: u64,
}

/// The mounted touchpoints, the app signals they can fire on, and the events
/// they have produced.
#[derive(Default)]
pub(crate) struct CoachStore {
    mounted: Vec<MountedTouchpoint>,
    events: VecDeque<CoachEvent>,
    palette: CoachPalette,
    /// Recent app-published signals, newest last. Read by sequence, never
    /// drained, so ordering between viewport passes cannot lose one.
    signals: VecDeque<(u64, SignalId)>,
    signal_head: u64,
    /// What the host says it can publish. The toolkit cannot verify this — it
    /// cannot see the host's fabric — so it is carried, not trusted.
    declared: Vec<SignalId>,
}

impl CoachStore {
    pub(crate) fn set_palette(&mut self, palette: CoachPalette) {
        self.palette = palette;
    }

    pub(crate) fn declare_signals(&mut self, signals: Vec<SignalId>) {
        self.declared = signals;
    }

    pub(crate) fn declared_signals(&self) -> Vec<SignalId> {
        self.declared.clone()
    }

    /// Record a signal the application chose to publish to the coach.
    pub(crate) fn note_signal(&mut self, signal: SignalId) {
        self.signal_head = self.signal_head.saturating_add(1);
        self.signals.push_back((self.signal_head, signal));
        while self.signals.len() > SIGNAL_RING {
            self.signals.pop_front();
        }
    }

    pub(crate) fn mount(&mut self, request: CoachMount) -> Result<(), CoachRefusal> {
        if self
            .mounted
            .iter()
            .any(|touchpoint| touchpoint.id == request.coach_id)
        {
            return Err(CoachRefusal::DuplicateId);
        }
        self.events.push_back(match request.rect {
            Some(rect) => CoachEvent::Mounted {
                coach_id: request.coach_id.clone(),
                rect,
            },
            None => CoachEvent::Armed {
                coach_id: request.coach_id.clone(),
            },
        });
        self.mounted.push(MountedTouchpoint {
            id: request.coach_id,
            node: request.node,
            viewport: request.viewport,
            on: request.on,
            message: request.message,
            ttl_ms: request.ttl_ms,
            once: request.once,
            started_at: None,
            present: request.rect.is_some(),
            hovered: false,
            value: None,
            signals_seen: self.signal_head,
        });
        Ok(())
    }

    pub(crate) fn unmount(&mut self, coach_id: &CoachId, reason: CoachUnmountReason) -> bool {
        let Some(index) = self
            .mounted
            .iter()
            .position(|touchpoint| &touchpoint.id == coach_id)
        else {
            return false;
        };
        self.mounted.remove(index);
        self.events.push_back(CoachEvent::Unmounted {
            coach_id: coach_id.clone(),
            reason,
        });
        true
    }

    pub(crate) fn unmount_all(&mut self, reason: CoachUnmountReason) -> Vec<CoachId> {
        let ids: Vec<CoachId> = self
            .mounted
            .drain(..)
            .map(|touchpoint| touchpoint.id)
            .collect();
        for coach_id in &ids {
            self.events.push_back(CoachEvent::Unmounted {
                coach_id: coach_id.clone(),
                reason,
            });
        }
        ids
    }

    /// End the lifetime of every touchpoint bound to a viewport that is
    /// going away.
    pub(crate) fn unmount_viewport(&mut self, viewport: egui::ViewportId) {
        let lost: Vec<CoachId> = self
            .mounted
            .iter()
            .filter(|touchpoint| touchpoint.viewport == Some(viewport))
            .map(|touchpoint| touchpoint.id.clone())
            .collect();
        for coach_id in lost {
            self.unmount(&coach_id, CoachUnmountReason::TargetLost);
        }
    }

    pub(crate) fn take_events(&mut self) -> Vec<CoachEvent> {
        self.events.drain(..).collect()
    }

    pub(crate) fn len(&self) -> usize {
        self.mounted.len()
    }

    /// Whether this viewport's pass has coach work to do. An unbound
    /// touchpoint is still waiting to discover which viewport owns its
    /// target, so every pass must consider it.
    pub(crate) fn targets_viewport(&self, viewport: egui::ViewportId) -> bool {
        self.mounted.iter().any(|touchpoint| {
            touchpoint.viewport.is_none() || touchpoint.viewport == Some(viewport)
        })
    }
}

/// What one viewport's pass knows about this frame.
pub(crate) struct CoachFrame<'a> {
    pub(crate) viewport: egui::ViewportId,
    pub(crate) screen: egui::Rect,
    pub(crate) now: f64,
    /// This pass's committed nodes, sorted by id.
    pub(crate) nodes: &'a [SemanticNode],
    /// Primary pointer presses in this frame's input.
    pub(crate) presses: &'a [egui::Pos2],
    /// Where the pointer is, if it is over this viewport at all.
    pub(crate) pointer: Option<egui::Pos2>,
    /// Who produced this frame's input, if anything did. `None` means the
    /// frame carried no input, so nothing a touchpoint notices can be
    /// attributed to a person or an agent.
    pub(crate) input_actor: Option<CoachActor>,
}

/// One frame of coach work for a single viewport: expire, resolve, paint, and
/// observe. Every trigger below is an edge against the state the touchpoint
/// carries from the previous pass.
pub(crate) fn run_pass(store: &mut CoachStore, painter: &egui::Painter, frame: &CoachFrame<'_>) {
    let mut expired: Vec<(CoachId, CoachUnmountReason)> = Vec::new();
    let mut fired: Vec<(CoachId, TriggerKind, CoachActor)> = Vec::new();
    let mut resolved: Vec<(CoachId, crate::protocol::Rect)> = Vec::new();
    let mut lit: Vec<(egui::Rect, Option<String>)> = Vec::new();
    let input_actor = frame.input_actor.unwrap_or(CoachActor::App);
    let signal_head = store.signal_head;
    for touchpoint in &mut store.mounted {
        if touchpoint
            .viewport
            .is_some_and(|bound| bound != frame.viewport)
        {
            continue;
        }
        let started_at = *touchpoint.started_at.get_or_insert(frame.now);
        if touchpoint
            .ttl_ms
            .is_some_and(|ttl| frame.now - started_at >= ttl as f64 / 1000.0)
        {
            expired.push((touchpoint.id.clone(), CoachUnmountReason::Ttl));
            continue;
        }

        // A signal fires wherever the touchpoint is in its life: waiting for
        // "the build finished" must not require the target to be on screen.
        let mut triggers: Vec<TriggerKind> = Vec::new();
        if let TriggerKind::FabricEvent { signal } = &touchpoint.on {
            if store
                .signals
                .iter()
                .any(|(seq, published)| *seq > touchpoint.signals_seen && published == signal)
            {
                triggers.push(touchpoint.on.clone());
            }
        }
        touchpoint.signals_seen = signal_head;

        let node = find_node(frame.nodes, &touchpoint.node);
        let Some(node) = node else {
            // A touchpoint waiting on an appearance tolerates absence; every
            // other kind has lost the thing it was pointing at.
            if touchpoint.on == TriggerKind::NodeAppeared {
                touchpoint.present = false;
                touchpoint.hovered = false;
            } else if touchpoint.viewport.is_some() {
                expired.push((touchpoint.id.clone(), CoachUnmountReason::TargetLost));
            }
            push_fired(&mut fired, &mut expired, touchpoint, triggers, input_actor);
            continue;
        };
        touchpoint.viewport = Some(frame.viewport);
        let rect = crate::egui_rect(node.rect);

        if !touchpoint.present {
            touchpoint.present = true;
            resolved.push((touchpoint.id.clone(), node.rect));
            if touchpoint.on == TriggerKind::NodeAppeared {
                triggers.push(TriggerKind::NodeAppeared);
            }
        }

        let baseline = touchpoint.value.get_or_insert_with(|| node.value.clone());
        if touchpoint.on == TriggerKind::ValueChanged && *baseline != node.value {
            *baseline = node.value.clone();
            triggers.push(TriggerKind::ValueChanged);
        }

        let hovered = frame.pointer.is_some_and(|pointer| rect.contains(pointer));
        if touchpoint.on == TriggerKind::Hover && hovered && !touchpoint.hovered {
            triggers.push(TriggerKind::Hover);
        }
        touchpoint.hovered = hovered;

        if touchpoint.on == TriggerKind::Press
            && frame.presses.iter().any(|press| rect.contains(*press))
        {
            triggers.push(TriggerKind::Press);
        }

        lit.push((rect, touchpoint.message.clone()));
        push_fired(&mut fired, &mut expired, touchpoint, triggers, input_actor);
    }
    paint_spotlights(painter, &store.palette, frame.screen, &lit);
    for (coach_id, rect) in resolved {
        store
            .events
            .push_back(CoachEvent::Mounted { coach_id, rect });
    }
    for (coach_id, kind, actor) in fired {
        store.events.push_back(CoachEvent::Triggered {
            coach_id,
            kind,
            actor,
        });
    }
    for (coach_id, reason) in expired {
        store.mounted.retain(|touchpoint| touchpoint.id != coach_id);
        store
            .events
            .push_back(CoachEvent::Unmounted { coach_id, reason });
    }
}

/// Queue this pass's triggers for a touchpoint, and its auto-unmount if it
/// only had one activation to give.
fn push_fired(
    fired: &mut Vec<(CoachId, TriggerKind, CoachActor)>,
    expired: &mut Vec<(CoachId, CoachUnmountReason)>,
    touchpoint: &MountedTouchpoint,
    triggers: Vec<TriggerKind>,
    input_actor: CoachActor,
) {
    if triggers.is_empty() {
        return;
    }
    for kind in triggers {
        // An appearance or an app signal is nobody's press. Attributing it to
        // the operator would be a lie the agent then reasons from.
        let actor = match kind {
            TriggerKind::NodeAppeared | TriggerKind::FabricEvent { .. } => CoachActor::App,
            _ => input_actor,
        };
        fired.push((touchpoint.id.clone(), kind, actor));
    }
    if touchpoint.once {
        expired.push((touchpoint.id.clone(), CoachUnmountReason::Once));
    }
}

/// The committed nodes are sorted by id, so a touchpoint costs a binary
/// search per pass rather than a per-frame map allocation.
fn find_node<'a>(nodes: &'a [SemanticNode], id: &NodeId) -> Option<&'a SemanticNode> {
    nodes
        .binary_search_by(|node| node.id.0.as_str().cmp(id.0.as_str()))
        .ok()
        .map(|index| &nodes[index])
}

/// The ColdLight paint vocabulary: black diffusion scatter outside, a
/// gradient tint inside, a crisp edge ring. Never a backdrop blur.
///
/// Every lit target is painted in one operation because the scatter is a
/// single layer with holes in it. Dimming once per touchpoint would stack,
/// and two touchpoints would black out each other's target and caption.
fn paint_spotlights(
    painter: &egui::Painter,
    palette: &CoachPalette,
    screen: egui::Rect,
    lit: &[(egui::Rect, Option<String>)],
) {
    let lit: Vec<_> = lit
        .iter()
        .map(|(target, message)| (target.intersect(screen), message))
        .filter(|(target, _)| target.width() > 0.0 && target.height() > 0.0)
        .collect();
    if lit.is_empty() {
        return;
    }
    let cutouts: Vec<egui::Rect> = lit.iter().map(|(target, _)| *target).collect();
    for band in scatter_bands(screen, &cutouts) {
        painter.rect_filled(band, 0, palette.scatter);
    }
    for (target, message) in lit {
        gradient_tint(painter, target, palette.tint);
        painter.rect_stroke(
            target,
            palette.corner_radius,
            egui::Stroke::new(palette.ring_width, palette.ring),
            egui::StrokeKind::Outside,
        );
        if let Some(message) = message.as_deref().filter(|message| !message.is_empty()) {
            paint_caption(painter, palette, screen, target, message);
        }
    }
}

/// The dimmed area: the screen minus every cutout, as non-overlapping bands.
/// A crisp rectangular hole per target, never a blur — the operator's eye
/// lands on exact controls.
///
/// Scanline decomposition: cut the screen into horizontal strips at every
/// cutout edge, then within each strip emit the gaps between the cutouts that
/// span it.
fn scatter_bands(screen: egui::Rect, cutouts: &[egui::Rect]) -> Vec<egui::Rect> {
    let cutouts: Vec<egui::Rect> = cutouts
        .iter()
        .map(|cutout| cutout.intersect(screen))
        .filter(|cutout| cutout.width() > 0.0 && cutout.height() > 0.0)
        .collect();
    if cutouts.is_empty() {
        return vec![screen];
    }
    let mut edges: Vec<f32> = Vec::with_capacity(cutouts.len() * 2 + 2);
    edges.push(screen.min.y);
    edges.push(screen.max.y);
    for cutout in &cutouts {
        edges.push(cutout.min.y);
        edges.push(cutout.max.y);
    }
    edges.sort_by(f32::total_cmp);
    edges.dedup();

    let mut bands = Vec::new();
    for pair in edges.windows(2) {
        let (top, bottom) = (pair[0], pair[1]);
        if bottom <= top {
            continue;
        }
        let mut spans: Vec<(f32, f32)> = cutouts
            .iter()
            .filter(|cutout| cutout.min.y <= top && cutout.max.y >= bottom)
            .map(|cutout| (cutout.min.x, cutout.max.x))
            .collect();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut x = screen.min.x;
        for (start, end) in spans {
            if start > x {
                bands.push(egui::Rect::from_min_max(
                    egui::pos2(x, top),
                    egui::pos2(start, bottom),
                ));
            }
            x = x.max(end);
        }
        if x < screen.max.x {
            bands.push(egui::Rect::from_min_max(
                egui::pos2(x, top),
                egui::pos2(screen.max.x, bottom),
            ));
        }
    }
    bands
}

fn gradient_tint(painter: &egui::Painter, rect: egui::Rect, head: egui::Color32) {
    let tail = egui::Color32::from_rgba_unmultiplied(head.r(), head.g(), head.b(), 0);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), head);
    mesh.colored_vertex(rect.right_top(), head);
    mesh.colored_vertex(rect.left_bottom(), tail);
    mesh.colored_vertex(rect.right_bottom(), tail);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    painter.add(egui::Shape::mesh(mesh));
}

fn paint_caption(
    painter: &egui::Painter,
    palette: &CoachPalette,
    screen: egui::Rect,
    target: egui::Rect,
    message: &str,
) {
    const GAP: f32 = 8.0;
    let padding = egui::vec2(10.0, 6.0);
    let galley = painter.layout_no_wrap(
        message.to_string(),
        palette.caption_font.clone(),
        palette.caption_ink,
    );
    let size = galley.size() + padding * 2.0;
    let above = target.min.y - GAP - size.y >= screen.min.y;
    let top_left = egui::pos2(
        (target.center().x - size.x / 2.0).clamp(
            screen.min.x + GAP,
            (screen.max.x - GAP - size.x).max(screen.min.x + GAP),
        ),
        if above {
            target.min.y - GAP - size.y
        } else {
            target.max.y + GAP
        },
    );
    let plate = egui::Rect::from_min_size(top_left, size);
    painter.rect_filled(plate, palette.corner_radius, palette.caption_plate);
    painter.rect_stroke(
        plate,
        palette.corner_radius,
        egui::Stroke::new(1.0, palette.caption_edge),
        egui::StrokeKind::Inside,
    );
    painter.galley(plate.min + padding, galley, palette.caption_ink);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Rect as WireRect;

    fn mount_request(id: &str, once: bool, ttl_ms: Option<u64>) -> CoachMount {
        CoachMount {
            coach_id: CoachId::new(id),
            node: NodeId::new("root/settings.save"),
            viewport: Some(egui::ViewportId::ROOT),
            rect: Some(WireRect::default()),
            on: TriggerKind::Press,
            message: None,
            ttl_ms,
            once,
        }
    }

    #[test]
    fn a_touchpoint_id_can_only_be_mounted_once() {
        let mut store = CoachStore::default();
        assert!(store.mount(mount_request("coach-1", true, None)).is_ok());
        assert_eq!(
            store.mount(mount_request("coach-1", true, None)),
            Err(CoachRefusal::DuplicateId)
        );
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn unmounting_an_unknown_touchpoint_produces_no_event() {
        let mut store = CoachStore::default();
        assert!(!store.unmount(&CoachId::new("ghost"), CoachUnmountReason::Agent));
        assert!(store.take_events().is_empty());
    }

    #[test]
    fn the_scatter_bands_tile_the_screen_around_the_cutout_without_overlap() {
        let screen = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let target = egui::Rect::from_min_max(egui::pos2(40.0, 30.0), egui::pos2(60.0, 50.0));
        let bands = scatter_bands(screen, &[target]);
        let covered: f32 = bands.iter().map(|band| band.width() * band.height()).sum();
        assert_eq!(
            covered,
            screen.width() * screen.height() - target.width() * target.height()
        );
        assert!(bands
            .iter()
            .all(|band| !band.intersects(target.shrink(0.5))));
    }

    /// Two touchpoints must dim the screen once, with a hole for each: a dim
    /// per touchpoint would stack and black out the other's target.
    #[test]
    fn two_cutouts_share_one_dim_and_neither_is_covered() {
        let screen = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let left = egui::Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(30.0, 40.0));
        let right = egui::Rect::from_min_max(egui::pos2(60.0, 55.0), egui::pos2(90.0, 70.0));
        let bands = scatter_bands(screen, &[left, right]);

        let covered: f32 = bands.iter().map(|band| band.width() * band.height()).sum();
        assert_eq!(
            covered,
            screen.width() * screen.height()
                - left.width() * left.height()
                - right.width() * right.height()
        );
        for cutout in [left, right] {
            assert!(
                bands
                    .iter()
                    .all(|band| !band.intersects(cutout.shrink(0.5))),
                "a band covered {cutout:?}"
            );
        }
        for (index, band) in bands.iter().enumerate() {
            for other in &bands[index + 1..] {
                assert!(
                    !band.intersects(other.shrink(0.5)),
                    "bands overlap, so the dim would stack: {band:?} and {other:?}"
                );
            }
        }
    }

    #[test]
    fn overlapping_cutouts_are_still_dimmed_exactly_once() {
        let screen = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let first = egui::Rect::from_min_max(egui::pos2(20.0, 20.0), egui::pos2(60.0, 60.0));
        let second = egui::Rect::from_min_max(egui::pos2(40.0, 20.0), egui::pos2(80.0, 60.0));
        let bands = scatter_bands(screen, &[first, second]);

        let union = 60.0 * 40.0;
        let covered: f32 = bands.iter().map(|band| band.width() * band.height()).sum();
        assert_eq!(covered, screen.width() * screen.height() - union);
    }
}
