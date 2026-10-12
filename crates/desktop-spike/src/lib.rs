//! Desktop-integration probes (orphan spike — never a dep of `app`).
//!
//! C2s composition: AT-SPI `WINDOW` extents plus an X11 origin derived from
//! the window's absolute position and `_GTK_FRAME_EXTENTS`. SCREEN extents
//! are never trusted (GTK4 reports `(0,0)`).

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
/// Absent extents (non-GTK / SSD) → zero margins, which is correct.
#[must_use]
pub fn client_origin(window: WindowOrigin, extents: FrameExtents) -> (i32, i32) {
    (window.x + extents.left, window.y + extents.top)
}

/// Compose a descendant's AT-SPI `WINDOW` rect onto the screen.
///
/// `screen_rect = client_origin + atspi_WINDOW_rect`
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
///
/// Outside the rect never triggers (even on a down edge). Holding Button1
/// already down and sliding in does not trigger — only a press that begins
/// inside the hotspot. Matches C2s §10.
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

    /// Measured constants from `docs-history-2026-10-12:docs/plans/coach-c2s-foreign-target-spike.md` §2.
    #[test]
    fn section_2_reconciliation_and_button_seven() {
        let extents = FrameExtents {
            left: 61,
            right: 61,
            top: 55,
            bottom: 67,
        };
        assert_eq!(client_size(482, 613, extents), (360, 491));

        // Absolute origin chosen so button 7 lands at the documented screen
        // rect `(82, 325, 64×44)` with WINDOW `(16, 293, 64×44)`.
        let window = WindowOrigin {
            x: 82 - 61 - 16,
            y: 325 - 55 - 293,
        };
        assert_eq!(window, WindowOrigin { x: 5, y: -23 });

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
        let up_outside = PointerSample {
            x: 0,
            y: 0,
            button1_down: false,
        };

        assert!(press_edge_in_hotspot(hot, up_inside, down_inside));
        assert!(!press_edge_in_hotspot(hot, down_inside, down_inside));
        assert!(!press_edge_in_hotspot(hot, up_outside, down_outside));
        // Slide in while already held — not a press that began inside.
        assert!(!press_edge_in_hotspot(hot, down_outside, down_inside));
        assert!(!press_edge_in_hotspot(
            IRect::new(0, 0, 0, 10),
            up_inside,
            down_inside
        ));
    }
}
