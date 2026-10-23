//! The backdrop — the operator's ratified spec (2026-07-22), not the old
//! React system: **exactly two options, the DPL loop or the Dragon loop**,
//! and **every click pulls a random authored MOOD** — a multi-hue treatment
//! from the generated registry (`tokens/moods.json` → `cl::MOODS`), never
//! one flat color. Paint chain per frame, behind every route's content:
//! mood gradient → the loop (ffmpeg subprocess) → the mood's tinted scrim.
//!
//! No color is improvised here: gradients, scrims, and accents all resolve
//! through the pool via the mood registry. No RNG dep: a time-seeded
//! xorshift re-rolls, and a re-roll never repeats the current mood.

use dpl_coldlight::{self as cl, Mood};
use egui::{Rect, Ui};

/// The catalog — the two loops, nothing else (ratified).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loop {
    Dpl,
    Dragon,
}

impl Loop {
    pub const ALL: [Loop; 2] = [Loop::Dpl, Loop::Dragon];

    pub fn label(self) -> &'static str {
        match self {
            Loop::Dpl => "DPL",
            Loop::Dragon => "Dragon",
        }
    }

    /// Repo-relative video path (the committed copies of the operator's
    /// real files).
    pub fn video_rel(self) -> &'static str {
        match self {
            Loop::Dpl => "assets/backdrops/dpl.mp4",
            Loop::Dragon => "assets/backdrops/clyffy-chases-the-dragon.mp4",
        }
    }

    pub fn poster_rel(self) -> &'static str {
        match self {
            Loop::Dpl => "assets/backdrops/dpl-poster.jpg",
            Loop::Dragon => "assets/backdrops/clyffy-chases-the-dragon-poster.jpg",
        }
    }
}

/// One random mood index, time-seeded — the shell's boot tooltip mood
/// ("on reload so we have variety of color") rolls here so mood randomness
/// stays colocated with the backdrop domain.
pub fn random_mood_index() -> usize {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0x9E37_79B9)
        | 1;
    let mut x = seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x.wrapping_mul(0x2545_F491_4F6C_DD1D) % cl::MOOD_COUNT as u64) as usize
}

pub struct Backdrop {
    current: Loop,
    mood: usize,
    /// xorshift state for the re-roll — seeded once from wall time.
    rng: u64,
}

impl Default for Backdrop {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 | 1)
            .unwrap_or(0x9E37_79B9);
        let mut b = Self {
            current: Loop::Dpl,
            mood: 0,
            rng: seed | 1,
        };
        // Boot mood is itself a pull, not a constant.
        b.mood = b.roll_any();
        b
    }
}

impl Backdrop {
    pub fn current(&self) -> Loop {
        self.current
    }

    pub fn mood(&self) -> &'static Mood {
        &cl::MOODS[self.mood]
    }

    /// Harness: pin the mood (`--mood <n>`), so shots are reproducible.
    pub fn set_mood(&mut self, idx: usize) {
        self.mood = idx % cl::MOOD_COUNT;
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64* — tiny, dep-free; quality is irrelevant here beyond
        // "doesn't visibly repeat".
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn roll_any(&mut self) -> usize {
        (self.next_u64() % cl::MOOD_COUNT as u64) as usize
    }

    /// THE law: every click pulls a fresh mood — selecting a loop or
    /// re-clicking the active one, either way the mood re-rolls, and never
    /// lands on the one already showing.
    pub fn click(&mut self, which: Loop) {
        self.current = which;
        let was = self.mood;
        let mut next = self.roll_any();
        while next == was {
            next = self.roll_any();
        }
        self.mood = next;
    }

    /// Paint the full chain into `rect` (call under every route's content).
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let mood = self.mood();
        cl::mood_gradient(ui.painter(), rect, mood);

        let suppressed = egui_pilot_ai::handle(ui.ctx()).is_some_and(|pilot| {
            pilot.register_motion(
                egui_pilot_ai::MotionId::new("devpulse.backdrop.video"),
                egui_pilot_ai::MotionKind::Ambient {
                    canonical_phase_seconds: Some(0.0),
                },
            );
            pilot.motion_policy() == egui_pilot_ai::MotionPolicy::Suppressed
        });

        let video = (!suppressed)
            .then(|| dpl_video::locate_asset(self.current.video_rel()))
            .flatten();
        let played = match video {
            Some(file) => {
                dpl_video::loop_backdrop(ui, rect, &file, false, egui::Color32::WHITE).is_some()
                    || {
                        let poster = dpl_video::locate_asset(self.current.poster_rel());
                        match poster {
                            Some(p) => {
                                let uri = p.to_string_lossy().replace('\\', "/");
                                egui::Image::new(format!("file:///{uri}")).paint_at(ui, rect);
                                true
                            }
                            None => false,
                        }
                    }
            }
            None => false, // mood gradient alone — honest, never a crash
        };

        // The mood's tinted veil; a touch lighter when only the gradient
        // shows so the treatment itself stays visible.
        let scrim = if played {
            mood.scrim_color()
        } else {
            let c = mood.scrim_color();
            egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), c.a() / 2)
        };
        ui.painter().rect_filled(rect, 0, scrim);
    }

    /// The chrome tiles: two poster thumbnails. Click = select + re-roll.
    /// Returns true when anything changed (mood included).
    pub fn tiles(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        for which in Loop::ALL {
            let active = which == self.current;
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(42.0, 26.0), egui::Sense::click());
            let painter = ui.painter();
            if let Some(p) = dpl_video::locate_asset(which.poster_rel()) {
                let uri = p.to_string_lossy().replace('\\', "/");
                egui::Image::new(format!("file:///{uri}"))
                    .corner_radius(cl::RADIUS_SM)
                    .paint_at(ui, rect);
            } else {
                painter.rect_filled(rect, cl::RADIUS_SM, cl::surface::OVERLAY);
            }
            let edge = if active {
                self.mood().accent_color()
            } else if resp.hovered() {
                cl::ink::MUTED
            } else {
                cl::surface::HAIRLINE
            };
            painter.rect(
                rect,
                cl::RADIUS_SM,
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(if active { 2.0 } else { 1.0 }, edge),
                egui::StrokeKind::Inside,
            );
            let resp = resp.on_hover_text(format!(
                "{} — click pulls a new mood (now: {})",
                which.label(),
                self.mood().name
            ));
            if resp.clicked() {
                self.click(which);
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_exactly_the_two_ratified_loops() {
        assert_eq!(Loop::ALL.len(), 2);
        assert!(Loop::Dpl.video_rel().ends_with("dpl.mp4"));
        assert!(Loop::Dragon
            .video_rel()
            .ends_with("clyffy-chases-the-dragon.mp4"));
    }

    #[test]
    fn every_click_pulls_a_fresh_mood_never_repeating_in_place() {
        let mut b = Backdrop::default();
        for i in 0..50 {
            let before = b.mood;
            let which = if i % 2 == 0 { Loop::Dragon } else { Loop::Dpl };
            b.click(which);
            assert_eq!(b.current(), which);
            assert_ne!(b.mood, before, "click #{i} kept the same mood");
            assert!(b.mood < cl::MOOD_COUNT);
        }
    }

    #[test]
    fn clicks_actually_walk_the_whole_registry_not_a_short_cycle() {
        let mut b = Backdrop::default();
        let mut seen = [false; cl::MOOD_COUNT];
        for _ in 0..200 {
            b.click(Loop::Dpl);
            seen[b.mood] = true;
        }
        let distinct = seen.iter().filter(|s| **s).count();
        assert!(
            distinct >= cl::MOOD_COUNT - 1,
            "only {distinct} moods reachable"
        );
    }

    #[test]
    fn harness_mood_pin_wraps_safely() {
        let mut b = Backdrop::default();
        b.set_mood(3);
        assert_eq!(b.mood().name, cl::MOODS[3].name);
        b.set_mood(cl::MOOD_COUNT + 2);
        assert_eq!(b.mood, 2);
    }

    #[test]
    fn the_boot_tooltip_roll_always_lands_inside_the_registry() {
        for _ in 0..64 {
            assert!(random_mood_index() < cl::MOOD_COUNT);
        }
    }

    #[test]
    fn assets_referenced_by_the_catalog_exist_in_the_repo() {
        // Anchored to the source tree, not to wherever the test binary landed.
        // `locate_asset` walks up from the executable, so it only reaches the
        // repo when the target directory sits inside it — and a shared
        // CARGO_TARGET_DIR is ordinary, not exotic.
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("crates/<name> sits two levels below the repo root");
        for which in Loop::ALL {
            for rel in [which.video_rel(), which.poster_rel()] {
                assert!(
                    repo.join(rel).is_file(),
                    "catalog references a missing asset: {rel}"
                );
            }
        }
    }
}
