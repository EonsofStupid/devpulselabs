//! The slide clock — each slide colocates one and every effect reads it.
//!
//! `tick(now)` arms the clock on the slide's first painted frame;
//! `restart()` clears it, so the next frame replays every effect from zero.
//! The preview flow (click a card → solo) restarts the clock — the operator
//! watches the full motion on every entry, and the rest state holds after.

#[derive(Debug, Default, Clone, Copy)]
pub struct Clock {
    t0: Option<f64>,
}

impl Clock {
    /// The slide's zero time, armed on first call.
    pub fn tick(&mut self, now: f64) -> f64 {
        *self.t0.get_or_insert(now)
    }

    /// Replay: clear the zero; the next `tick` re-arms it.
    pub fn restart(&mut self) {
        self.t0 = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arms_once_and_replays_on_restart() {
        let mut c = Clock::default();
        assert_eq!(c.tick(5.0), 5.0);
        assert_eq!(c.tick(9.0), 5.0, "t0 holds until restart");
        c.restart();
        assert_eq!(c.tick(9.0), 9.0, "restart re-arms at the next frame");
    }
}
