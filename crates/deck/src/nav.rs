//! Deck navigation math — pure, side-effect free. Ported VERBATIM from the
//! old repo's `deck-nav.service.ts` (the beat grammar).
//!
//! Step semantics: `step_count = reveals + 1` — step 0 is the SETUP rest
//! state (nothing revealed; arriving never pre-reveals), step k = k items
//! revealed. Slides without reveals have exactly one step, so callers
//! never branch. Retreating into a previous slide lands on its LAST step
//! (fully revealed rest state).
//!
//! All functions take `steps: &[usize]` — the per-slide step counts the
//! deck derives once from its documents (`Slide::step_count()`).

/// One beat position in a deck. Copy — positions are values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub index: usize,
    pub step: usize,
}

pub fn clamp_index(index: usize, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    index.min(total - 1)
}

pub fn clamp_step(step: usize, steps: usize) -> usize {
    step.min(steps.saturating_sub(1))
}

/// Advance one beat: next reveal step within the current slide, or step 0
/// of the next slide when this slide's steps are exhausted. Holds at the
/// deck's last slide + last step.
pub fn advance(steps: &[usize], pos: Position) -> Position {
    let total = steps.len();
    if total == 0 {
        return Position { index: 0, step: 0 };
    }
    let index = clamp_index(pos.index, total);
    let step = clamp_step(pos.step, steps[index]);
    if step + 1 < steps[index] {
        return Position {
            index,
            step: step + 1,
        };
    }
    if index + 1 >= total {
        return Position { index, step };
    }
    Position {
        index: index + 1,
        step: 0,
    }
}

/// Retreat one beat: previous reveal step, or the LAST step of the
/// previous slide when at step 0 (a fully revealed rest state). Holds at
/// `{ 0, 0 }`.
pub fn retreat(steps: &[usize], pos: Position) -> Position {
    let total = steps.len();
    if total == 0 {
        return Position { index: 0, step: 0 };
    }
    let index = clamp_index(pos.index, total);
    let step = clamp_step(pos.step, steps[index]);
    if step > 0 {
        return Position {
            index,
            step: step - 1,
        };
    }
    if index == 0 {
        return Position { index: 0, step: 0 };
    }
    Position {
        index: index - 1,
        step: steps[index - 1].saturating_sub(1),
    }
}

pub fn can_advance(steps: &[usize], pos: Position) -> bool {
    advance(steps, pos) != normalize(steps, pos)
}

pub fn can_retreat(steps: &[usize], pos: Position) -> bool {
    retreat(steps, pos) != normalize(steps, pos)
}

/// The clamped form of a position — comparisons in `can_*` must judge the
/// REAL position, not raw out-of-range input.
fn normalize(steps: &[usize], pos: Position) -> Position {
    let total = steps.len();
    if total == 0 {
        return Position { index: 0, step: 0 };
    }
    let index = clamp_index(pos.index, total);
    Position {
        index,
        step: clamp_step(pos.step, steps[index]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seed-show shape: 1-step, 2-step (one reveal), 1-step, 1-step.
    const DECK: &[usize] = &[1, 2, 1, 1];

    fn p(index: usize, step: usize) -> Position {
        Position { index, step }
    }

    #[test]
    fn advance_walks_steps_then_slides_and_holds_at_the_end() {
        // The full expected beat list for DECK, from the top.
        let beats = [p(0, 0), p(1, 0), p(1, 1), p(2, 0), p(3, 0)];
        let mut pos = beats[0];
        for expected in &beats[1..] {
            pos = advance(DECK, pos);
            assert_eq!(pos, *expected);
        }
        // Holds at the last beat.
        assert_eq!(advance(DECK, pos), pos);
        assert!(!can_advance(DECK, pos));
    }

    #[test]
    fn retreat_lands_on_the_previous_slides_last_step() {
        // From slide 2 step 0, retreating enters slide 1 at its LAST step
        // (fully revealed rest state) — the old service's exact rule.
        assert_eq!(retreat(DECK, p(2, 0)), p(1, 1));
        assert_eq!(retreat(DECK, p(1, 1)), p(1, 0));
        assert_eq!(retreat(DECK, p(1, 0)), p(0, 0));
        // Holds at { 0, 0 }.
        assert_eq!(retreat(DECK, p(0, 0)), p(0, 0));
        assert!(!can_retreat(DECK, p(0, 0)));
    }

    #[test]
    fn a_full_round_trip_is_symmetric() {
        // advance* to the end then retreat* back reproduces every beat.
        let mut forward = vec![p(0, 0)];
        loop {
            let next = advance(DECK, *forward.last().unwrap());
            if next == *forward.last().unwrap() {
                break;
            }
            forward.push(next);
        }
        let mut backward = vec![*forward.last().unwrap()];
        loop {
            let prev = retreat(DECK, *backward.last().unwrap());
            if prev == *backward.last().unwrap() {
                break;
            }
            backward.push(prev);
        }
        backward.reverse();
        assert_eq!(forward, backward, "the beat list must be reversible");
    }

    #[test]
    fn single_step_slides_never_branch() {
        let flat = &[1, 1, 1];
        assert_eq!(advance(flat, p(0, 0)), p(1, 0));
        assert_eq!(advance(flat, p(1, 0)), p(2, 0));
        assert_eq!(retreat(flat, p(2, 0)), p(1, 0));
    }

    #[test]
    fn out_of_range_input_clamps_instead_of_panicking() {
        assert_eq!(advance(DECK, p(99, 99)), p(3, 0), "clamped to the end");
        assert_eq!(retreat(DECK, p(99, 99)), p(2, 0));
        assert_eq!(clamp_index(5, 4), 3);
        assert_eq!(clamp_step(5, 2), 1);
        // The can_* pair judges the NORMALIZED position.
        assert!(!can_advance(DECK, p(99, 99)));
        assert!(can_retreat(DECK, p(99, 99)));
    }

    #[test]
    fn an_empty_deck_is_inert_not_a_panic() {
        assert_eq!(advance(&[], p(0, 0)), p(0, 0));
        assert_eq!(retreat(&[], p(3, 1)), p(0, 0));
        assert!(!can_advance(&[], p(0, 0)));
        assert!(!can_retreat(&[], p(0, 0)));
    }
}
