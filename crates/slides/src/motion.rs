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

    /// The frame's motion context as ONE value — paint helpers take a
    /// `Beat`, never loose `(now, t0)` float pairs.
    pub fn beat(&mut self, now: f64) -> Beat {
        Beat {
            now,
            t0: self.tick(now),
        }
    }

    /// Replay: clear the zero; the next `tick` re-arms it.
    pub fn restart(&mut self) {
        self.t0 = None;
    }
}

/// One frame of a slide's clock: the current time and the slide's zero.
/// Copy — a beat is a value that flows into paint helpers.
#[derive(Debug, Clone, Copy)]
pub struct Beat {
    pub now: f64,
    pub t0: f64,
}

impl Beat {
    /// The same frame with the zero pushed later — per-element stagger
    /// without threading extra offset arguments through helpers.
    pub fn delayed(self, by: f64) -> Beat {
        Beat {
            now: self.now,
            t0: self.t0 + by,
        }
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
