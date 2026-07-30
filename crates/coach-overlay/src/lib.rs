//! Coach overlay geometry + observe helpers (`dpl-coach-overlay`).
//!
//! Devtools-only process sibling of the C2s desktop spike. Owns the typed
//! WINDOW→screen compose algebra (never trusts GTK4 SCREEN extents). Never a
//! dependency of `crates/app` — ADR: `docs/plans/coach-overlay-adr.md`.

#![deny(unsafe_code)]

/// Inclusive pixel rectangle in screen or window space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    #[must_use]
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    #[must_use]
    pub fn is_positive(self) -> bool {
        self.w > 0 && self.h > 0
    }
}

/// Client-side decoration margins: left, right, top, bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameExtents {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

/// Absolute X11 window origin (outer frame, including CSD shadow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowOrigin {
    pub x: i32,
    pub y: i32,
}

/// Why a screen rect cannot be composed. Never paint at `(0,0)` on refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocateRefuse {
    /// No X11 window matched the AT-SPI application.
    NoWindow,
    /// Matched window reported a non-positive outer size.
    DegenerateWindow,
}

/// Visible client origin = X11 absolute + `_GTK_FRAME_EXTENTS` left/top.
#[must_use]
pub fn client_origin(window: WindowOrigin, extents: FrameExtents) -> (i32, i32) {
    (window.x + extents.left, window.y + extents.top)
}

/// Compose a descendant's AT-SPI `WINDOW` rect onto the screen.
pub fn compose_screen_rect(
    window: WindowOrigin,
    extents: FrameExtents,
    window_rect: IRect,
) -> Result<IRect, LocateRefuse> {
    if window_rect.w <= 0 || window_rect.h <= 0 {
        return Err(LocateRefuse::DegenerateWindow);
    }
    let (ox, oy) = client_origin(window, extents);
    Ok(IRect::new(
        ox + window_rect.x,
        oy + window_rect.y,
        window_rect.w,
        window_rect.h,
    ))
}

/// Reconcile outer X11 size with AT-SPI frame size via CSD margins.
#[must_use]
pub fn client_size(outer_w: i32, outer_h: i32, extents: FrameExtents) -> (i32, i32) {
    (
        outer_w - extents.left - extents.right,
        outer_h - extents.top - extents.bottom,
    )
}

/// Pointer sample used for observe-never-intercept Triggered detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerSample {
    pub x: i32,
    pub y: i32,
    pub button1_down: bool,
}

/// True on a Button1 up→down edge while the pointer is inside `hotspot`.
#[must_use]
pub fn press_edge_in_hotspot(
    hotspot: IRect,
    previous: PointerSample,
    current: PointerSample,
) -> bool {
    if !hotspot.is_positive() {
        return false;
    }
    let inside = current.x >= hotspot.x
        && current.y >= hotspot.y
        && current.x < hotspot.x + hotspot.w
        && current.y < hotspot.y + hotspot.h;
    inside && current.button1_down && !previous.button1_down
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_2_reconciliation_and_button_seven() {
        let extents = FrameExtents {
            left: 61,
            right: 61,
            top: 55,
            bottom: 67,
        };
        assert_eq!(client_size(482, 613, extents), (360, 491));
        let window = WindowOrigin {
            x: 82 - 61 - 16,
            y: 325 - 55 - 293,
        };
        let screen = compose_screen_rect(window, extents, IRect::new(16, 293, 64, 44))
            .expect("button 7 must compose");
        assert_eq!(screen, IRect::new(82, 325, 64, 44));
    }

    #[test]
    fn missing_extents_are_zero_margins() {
        let screen = compose_screen_rect(
            WindowOrigin { x: 100, y: 200 },
            FrameExtents::default(),
            IRect::new(10, 20, 30, 40),
        )
        .unwrap();
        assert_eq!(screen, IRect::new(110, 220, 30, 40));
    }

    #[test]
    fn degenerate_window_rect_refuses() {
        let err = compose_screen_rect(
            WindowOrigin { x: 0, y: 0 },
            FrameExtents::default(),
            IRect::new(0, 0, 0, 10),
        )
        .unwrap_err();
        assert_eq!(err, LocateRefuse::DegenerateWindow);
    }

    #[test]
    fn press_edge_triggers_only_inside_on_down_edge() {
        let hot = IRect::new(10, 20, 30, 40);
        let up_inside = PointerSample {
            x: 15,
            y: 25,
            button1_down: false,
        };
        let down_inside = PointerSample {
            x: 15,
            y: 25,
            button1_down: true,
        };
        let down_outside = PointerSample {
            x: 0,
            y: 0,
            button1_down: true,
        };
        assert!(press_edge_in_hotspot(hot, up_inside, down_inside));
        assert!(!press_edge_in_hotspot(hot, down_inside, down_inside));
        assert!(!press_edge_in_hotspot(hot, down_outside, down_inside));
    }
}
