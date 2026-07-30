//! The `d` grammar — every SVG path command normalized to four moves:
//! MoveTo / LineTo / CubicTo / Close. H and V collapse to lines, Q and T
//! elevate to cubics, S and T reflect their sibling's control, and A
//! converts by the spec's center parameterization, split at 90° so each
//! piece is one well-behaved cubic.

use crate::{err, SvgError};
use std::f32::consts::{FRAC_PI_2, TAU};

pub(crate) type P = (f32, f32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Cmd {
    MoveTo(P),
    LineTo(P),
    CubicTo(P, P, P),
    Close,
}

pub(crate) fn parse(d: &str) -> Result<Vec<Cmd>, SvgError> {
    let mut s = Scan {
        b: d.as_bytes(),
        i: 0,
    };
    let mut out = Vec::new();
    let mut cur: P = (0.0, 0.0);
    let mut start: P = (0.0, 0.0);
    // Reflection memory: Some(control) only while the PREVIOUS command was
    // of the same family (cubic for S, quadratic for T) — the spec's rule.
    let mut prev_cubic_ctrl: Option<P> = None;
    let mut prev_quad_ctrl: Option<P> = None;
    let mut op: Option<u8> = None;

    loop {
        s.seps();
        if s.done() {
            return Ok(out);
        }
        if let Some(c) = s.command() {
            op = Some(c);
            if c == b'Z' || c == b'z' {
                out.push(Cmd::Close);
                cur = start;
                prev_cubic_ctrl = None;
                prev_quad_ctrl = None;
                continue;
            }
        }
        let Some(c) = op else {
            return Err(err("path data starts without a command"));
        };
        let rel = c.is_ascii_lowercase();
        let mut next_cubic_ctrl: Option<P> = None;
        let mut next_quad_ctrl: Option<P> = None;
        match c.to_ascii_uppercase() {
            b'M' => {
                let p = s.pair(rel, cur)?;
                out.push(Cmd::MoveTo(p));
                cur = p;
                start = p;
                // Implicit repetition: bare coordinates after M continue
                // as lines — the grammar's one quirk.
                op = Some(if rel { b'l' } else { b'L' });
            }
            b'L' => {
                let p = s.pair(rel, cur)?;
                out.push(Cmd::LineTo(p));
                cur = p;
            }
            b'H' => {
                let x = s.coord(rel, cur.0)?;
                cur = (x, cur.1);
                out.push(Cmd::LineTo(cur));
            }
            b'V' => {
                let y = s.coord(rel, cur.1)?;
                cur = (cur.0, y);
                out.push(Cmd::LineTo(cur));
            }
            b'C' => {
                let c1 = s.pair(rel, cur)?;
                let c2 = s.pair(rel, cur)?;
                let p = s.pair(rel, cur)?;
                out.push(Cmd::CubicTo(c1, c2, p));
                next_cubic_ctrl = Some(c2);
                cur = p;
            }
            b'S' => {
                let c1 = reflect(prev_cubic_ctrl, cur);
                let c2 = s.pair(rel, cur)?;
                let p = s.pair(rel, cur)?;
                out.push(Cmd::CubicTo(c1, c2, p));
                next_cubic_ctrl = Some(c2);
                cur = p;
            }
            b'Q' => {
                let q = s.pair(rel, cur)?;
                let p = s.pair(rel, cur)?;
                out.push(quad_to_cubic(cur, q, p));
                next_quad_ctrl = Some(q);
                cur = p;
            }
            b'T' => {
                let q = reflect(prev_quad_ctrl, cur);
                let p = s.pair(rel, cur)?;
                out.push(quad_to_cubic(cur, q, p));
                next_quad_ctrl = Some(q);
                cur = p;
            }
            b'A' => {
                let rx = s.number()?;
                let ry = s.number()?;
                let rot = s.number()?;
                let large = s.flag()?;
                let sweep = s.flag()?;
                let p = s.pair(rel, cur)?;
                arc_to_cubics(cur, (rx, ry), rot, large, sweep, p, &mut out);
                cur = p;
            }
            b'Z' => return Err(err("coordinates after Z")),
            other => {
                return Err(err(format!("unknown path command {:?}", other as char)));
            }
        }
        prev_cubic_ctrl = next_cubic_ctrl;
        prev_quad_ctrl = next_quad_ctrl;
    }
}

struct Scan<'a> {
    b: &'a [u8],
    i: usize,
}

impl Scan<'_> {
    fn seps(&mut self) {
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_whitespace() || self.b[self.i] == b',')
        {
            self.i += 1;
        }
    }

    fn done(&self) -> bool {
        self.i >= self.b.len()
    }

    /// Consumes the next byte if it is a command letter. Only ever called
    /// at token boundaries, so an exponent's `e` can never land here.
    fn command(&mut self) -> Option<u8> {
        let c = *self.b.get(self.i)?;
        if c.is_ascii_alphabetic() {
            self.i += 1;
            Some(c)
        } else {
            None
        }
    }

    fn number(&mut self) -> Result<f32, SvgError> {
        self.seps();
        let start = self.i;
        if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
            self.i += 1;
        }
        let mut digits = false;
        while matches!(self.b.get(self.i), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
            digits = true;
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            while matches!(self.b.get(self.i), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
                digits = true;
            }
        }
        if !digits {
            return Err(err(format!("expected a number at byte {start}")));
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            let mut exp_digits = false;
            while matches!(self.b.get(self.i), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
                exp_digits = true;
            }
            if !exp_digits {
                return Err(err("exponent without digits"));
            }
        }
        std::str::from_utf8(&self.b[start..self.i])
            .ok()
            .and_then(|t| t.parse::<f32>().ok())
            .ok_or_else(|| err(format!("unparseable number at byte {start}")))
    }

    /// Arc flags are single digits and may be smooshed against the next
    /// number ("…0 011 1…") — the grammar's second quirk.
    fn flag(&mut self) -> Result<bool, SvgError> {
        self.seps();
        match self.b.get(self.i) {
            Some(b'0') => {
                self.i += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.i += 1;
                Ok(true)
            }
            _ => Err(err("expected an arc flag (0 or 1)")),
        }
    }

    fn coord(&mut self, rel: bool, base: f32) -> Result<f32, SvgError> {
        let v = self.number()?;
        Ok(if rel { base + v } else { v })
    }

    fn pair(&mut self, rel: bool, base: P) -> Result<P, SvgError> {
        let x = self.coord(rel, base.0)?;
        let y = self.coord(rel, base.1)?;
        Ok((x, y))
    }
}

fn reflect(prev: Option<P>, cur: P) -> P {
    match prev {
        Some((cx, cy)) => (2.0 * cur.0 - cx, 2.0 * cur.1 - cy),
        None => cur,
    }
}

/// Degree elevation: a quadratic is exactly a cubic with controls at 2/3.
fn quad_to_cubic(from: P, q: P, to: P) -> Cmd {
    let c1 = (
        from.0 + 2.0 / 3.0 * (q.0 - from.0),
        from.1 + 2.0 / 3.0 * (q.1 - from.1),
    );
    let c2 = (
        to.0 + 2.0 / 3.0 * (q.0 - to.0),
        to.1 + 2.0 / 3.0 * (q.1 - to.1),
    );
    Cmd::CubicTo(c1, c2, to)
}

/// The spec's endpoint → center parameterization (F.6.5), emitted as one
/// cubic per ≤90° of sweep. The last cubic is pinned EXACTLY to the
/// endpoint so float drift never leaks into the next command's frame.
fn arc_to_cubics(from: P, r: P, rot_deg: f32, large: bool, sweep: bool, to: P, out: &mut Vec<Cmd>) {
    let (mut rx, mut ry) = (r.0.abs(), r.1.abs());
    if rx == 0.0 || ry == 0.0 || from == to {
        if from != to {
            out.push(Cmd::LineTo(to));
        }
        return;
    }
    let phi = rot_deg.to_radians();
    let (cos_phi, sin_phi) = (phi.cos(), phi.sin());
    let dx = (from.0 - to.0) / 2.0;
    let dy = (from.1 - to.1) / 2.0;
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;
    // Radii too small for the endpoints scale up to exactly fit.
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = (num / den).sqrt();
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let cx = cos_phi * cx1 - sin_phi * cy1 + (from.0 + to.0) / 2.0;
    let cy = sin_phi * cx1 + cos_phi * cy1 + (from.1 + to.1) / 2.0;
    let theta1 = ((y1 - cy1) / ry).atan2((x1 - cx1) / rx);
    let mut dtheta = ((-y1 - cy1) / ry).atan2((-x1 - cx1) / rx) - theta1;
    if !sweep && dtheta > 0.0 {
        dtheta -= TAU;
    }
    if sweep && dtheta < 0.0 {
        dtheta += TAU;
    }

    let segs = (dtheta.abs() / FRAC_PI_2).ceil().max(1.0);
    let delta = dtheta / segs;
    let alpha = 4.0 / 3.0 * (delta / 4.0).tan();
    let point = |t: f32| -> P {
        let x = rx * t.cos();
        let y = ry * t.sin();
        (
            cos_phi * x - sin_phi * y + cx,
            sin_phi * x + cos_phi * y + cy,
        )
    };
    let deriv = |t: f32| -> P {
        let x = -rx * t.sin();
        let y = ry * t.cos();
        (cos_phi * x - sin_phi * y, sin_phi * x + cos_phi * y)
    };
    let mut t = theta1;
    for _ in 0..segs as u32 {
        let t2 = t + delta;
        let (sx, sy) = point(t);
        let (ex, ey) = point(t2);
        let (dsx, dsy) = deriv(t);
        let (dex, dey) = deriv(t2);
        out.push(Cmd::CubicTo(
            (sx + alpha * dsx, sy + alpha * dsy),
            (ex - alpha * dex, ey - alpha * dey),
            (ex, ey),
        ));
        t = t2;
    }
    if let Some(Cmd::CubicTo(_, _, p)) = out.last_mut() {
        *p = to;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end_of(cmds: &[Cmd]) -> P {
        match *cmds.last().expect("nonempty") {
            Cmd::MoveTo(p) | Cmd::LineTo(p) | Cmd::CubicTo(_, _, p) => p,
            Cmd::Close => panic!("close has no point"),
        }
    }

    #[test]
    fn relative_and_absolute_agree() {
        let a = parse("M1 1 L2 1 L2 2").expect("abs");
        let b = parse("m1 1 l1 0 l0 1").expect("rel");
        assert_eq!(a, b);
    }

    #[test]
    fn h_and_v_collapse_to_lines() {
        let cmds = parse("M1 2 H5 v3").expect("parse");
        assert_eq!(
            cmds,
            vec![
                Cmd::MoveTo((1.0, 2.0)),
                Cmd::LineTo((5.0, 2.0)),
                Cmd::LineTo((5.0, 5.0)),
            ]
        );
    }

    #[test]
    fn s_reflects_the_previous_cubic_control() {
        let cmds = parse("M0 0 C1 1 2 1 3 0 S5 -1 6 0").expect("parse");
        let Cmd::CubicTo(c1, _, _) = cmds[2] else {
            panic!("expected a cubic");
        };
        // Reflection of (2,1) about (3,0) is (4,-1).
        assert_eq!(c1, (4.0, -1.0));
    }

    #[test]
    fn an_arc_lands_exactly_on_its_endpoint() {
        let cmds = parse("M0 0 A1 1 0 0 1 2 0").expect("parse");
        assert_eq!(end_of(&cmds), (2.0, 0.0));
    }

    #[test]
    fn the_sweep_flag_picks_the_screen_clockwise_arc() {
        // Sweep=1 from the circle's left point runs clockwise on screen
        // (y down): the arc bulges through (1, -1).
        let cmds = parse("M0 0 A1 1 0 0 1 2 0").expect("parse");
        let min_y = cmds
            .iter()
            .filter_map(|c| match *c {
                Cmd::CubicTo(_, _, (_, y)) => Some(y),
                _ => None,
            })
            .fold(f32::MAX, f32::min);
        assert!((min_y + 1.0).abs() < 0.01, "min_y = {min_y}");
    }

    #[test]
    fn smooshed_arc_flags_parse() {
        // "011 1" reads as flag 0, flag 1, then x=1 — the grammar quirk.
        let cmds = parse("M0 0a1 1 0 011 1").expect("parse");
        assert_eq!(end_of(&cmds), (1.0, 1.0));
    }

    #[test]
    fn compact_decimals_parse() {
        let cmds = parse("M12 12l.5.5").expect("parse");
        assert_eq!(end_of(&cmds), (12.5, 12.5));
    }

    #[test]
    fn coordinates_after_z_are_refused() {
        let e = parse("M0 0 L1 1 Z 2 2").expect_err("must refuse");
        assert!(e.0.contains("after Z"), "{e}");
    }
}
