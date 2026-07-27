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
    CoachActor, CoachEvent, CoachId, CoachRefusal, CoachUnmountReason, NodeId, TriggerKind,
};
use std::collections::HashMap;

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

/// One mount request, already resolved against a live semantic node.
#[derive(Debug, Clone)]
pub struct CoachMount {
    pub coach_id: CoachId,
    pub node: NodeId,
    pub viewport: egui::ViewportId,
    pub rect: crate::protocol::Rect,
    pub message: Option<String>,
    pub ttl_ms: Option<u64>,
    pub once: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MountedTouchpoint {
    id: CoachId,
    node: NodeId,
    viewport: egui::ViewportId,
    message: Option<String>,
    ttl_ms: Option<u64>,
    once: bool,
    /// Target clock seconds, stamped on the first frame the touchpoint paints.
    started_at: Option<f64>,
}

/// The mounted touchpoints and the events they have produced.
#[derive(Default)]
pub(crate) struct CoachStore {
    mounted: Vec<MountedTouchpoint>,
    events: std::collections::VecDeque<CoachEvent>,
    palette: CoachPalette,
}

impl CoachStore {
    pub(crate) fn set_palette(&mut self, palette: CoachPalette) {
        self.palette = palette;
    }

    pub(crate) fn mount(&mut self, request: CoachMount) -> Result<(), CoachRefusal> {
        if self
            .mounted
            .iter()
            .any(|touchpoint| touchpoint.id == request.coach_id)
        {
            return Err(CoachRefusal::DuplicateId);
        }
        self.events.push_back(CoachEvent::Mounted {
            coach_id: request.coach_id.clone(),
            rect: request.rect,
        });
        self.mounted.push(MountedTouchpoint {
            id: request.coach_id,
            node: request.node,
            viewport: request.viewport,
            message: request.message,
            ttl_ms: request.ttl_ms,
            once: request.once,
            started_at: None,
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
            .filter(|touchpoint| touchpoint.viewport == viewport)
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

    pub(crate) fn targets_viewport(&self, viewport: egui::ViewportId) -> bool {
        self.mounted
            .iter()
            .any(|touchpoint| touchpoint.viewport == viewport)
    }
}

/// What one viewport's pass knows about this frame.
pub(crate) struct CoachFrame<'a> {
    pub(crate) viewport: egui::ViewportId,
    pub(crate) screen: egui::Rect,
    pub(crate) now: f64,
    /// This pass's committed node rects.
    pub(crate) rects: &'a HashMap<NodeId, egui::Rect>,
    /// Primary pointer presses in this frame's input.
    pub(crate) presses: &'a [egui::Pos2],
    /// Who produced those presses.
    pub(crate) actor: CoachActor,
}

/// One frame of coach work for a single viewport: expire, paint, and observe.
pub(crate) fn run_pass(store: &mut CoachStore, painter: &egui::Painter, frame: &CoachFrame<'_>) {
    let mut expired: Vec<(CoachId, CoachUnmountReason)> = Vec::new();
    let mut triggered: Vec<CoachId> = Vec::new();
    for touchpoint in &mut store.mounted {
        if touchpoint.viewport != frame.viewport {
            continue;
        }
        let Some(rect) = frame.rects.get(&touchpoint.node).copied() else {
            expired.push((touchpoint.id.clone(), CoachUnmountReason::TargetLost));
            continue;
        };
        let started_at = *touchpoint.started_at.get_or_insert(frame.now);
        if touchpoint
            .ttl_ms
            .is_some_and(|ttl| frame.now - started_at >= ttl as f64 / 1000.0)
        {
            expired.push((touchpoint.id.clone(), CoachUnmountReason::Ttl));
            continue;
        }
        paint_spotlight(
            painter,
            &store.palette,
            frame.screen,
            rect,
            touchpoint.message.as_deref(),
        );
        if frame.presses.iter().any(|press| rect.contains(*press)) {
            triggered.push(touchpoint.id.clone());
            if touchpoint.once {
                expired.push((touchpoint.id.clone(), CoachUnmountReason::Once));
            }
        }
    }
    for coach_id in triggered {
        store.events.push_back(CoachEvent::Triggered {
            coach_id,
            kind: TriggerKind::Press,
            actor: frame.actor,
        });
    }
    for (coach_id, reason) in expired {
        store.mounted.retain(|touchpoint| touchpoint.id != coach_id);
        store
            .events
            .push_back(CoachEvent::Unmounted { coach_id, reason });
    }
}

/// The ColdLight paint vocabulary: black diffusion scatter outside, a
/// gradient tint inside, a crisp edge ring. Never a backdrop blur.
fn paint_spotlight(
    painter: &egui::Painter,
    palette: &CoachPalette,
    screen: egui::Rect,
    target: egui::Rect,
    message: Option<&str>,
) {
    let target = target.intersect(screen);
    if target.width() <= 0.0 || target.height() <= 0.0 {
        return;
    }
    for band in scatter_bands(screen, target) {
        painter.rect_filled(band, 0, palette.scatter);
    }
    gradient_tint(painter, target, palette.tint);
    painter.rect_stroke(
        target,
        palette.corner_radius,
        egui::Stroke::new(palette.ring_width, palette.ring),
        egui::StrokeKind::Outside,
    );
    if let Some(message) = message.filter(|message| !message.is_empty()) {
        paint_caption(painter, palette, screen, target, message);
    }
}

/// The four dimmed bands around the cutout. A crisp rectangular hole, not a
/// blur: the operator's eye lands on one exact control.
fn scatter_bands(screen: egui::Rect, target: egui::Rect) -> [egui::Rect; 4] {
    [
        egui::Rect::from_min_max(screen.min, egui::pos2(screen.max.x, target.min.y)),
        egui::Rect::from_min_max(egui::pos2(screen.min.x, target.max.y), screen.max),
        egui::Rect::from_min_max(
            egui::pos2(screen.min.x, target.min.y),
            egui::pos2(target.min.x, target.max.y),
        ),
        egui::Rect::from_min_max(
            egui::pos2(target.max.x, target.min.y),
            egui::pos2(screen.max.x, target.max.y),
        ),
    ]
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
            viewport: egui::ViewportId::ROOT,
            rect: WireRect::default(),
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
        let bands = scatter_bands(screen, target);
        let covered: f32 = bands.iter().map(|band| band.width() * band.height()).sum();
        assert_eq!(
            covered,
            screen.width() * screen.height() - target.width() * target.height()
        );
        assert!(bands
            .iter()
            .all(|band| !band.intersects(target.shrink(0.5))));
    }
}
