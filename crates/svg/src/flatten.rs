//! Adaptive cubic flattening — de Casteljau subdivision until both
//! controls hug the chord within tolerance. Tolerance is in viewBox
//! units: generation picks it once (ADR-015: 0.02 in the 24-box keeps
//! error sub-pixel out past 500 px renders).

use crate::path::{Cmd, P};

#[derive(Debug)]
pub(crate) struct SubPath {
    pub points: Vec<P>,
    pub closed: bool,
}

pub(crate) fn flatten(cmds: &[Cmd], tolerance: f32) -> Vec<SubPath> {
    let mut out = Vec::new();
    let mut pts: Vec<P> = Vec::new();
    let mut cur: P = (0.0, 0.0);
    let mut start: P = (0.0, 0.0);
    for cmd in cmds {
        match *cmd {
            Cmd::MoveTo(p) => {
                finish(&mut out, &mut pts, false);
                pts.push(p);
                cur = p;
                start = p;
            }
            Cmd::LineTo(p) => {
                if pts.is_empty() {
                    pts.push(cur);
                }
                pts.push(p);
                cur = p;
            }
            Cmd::CubicTo(c1, c2, p) => {
                if pts.is_empty() {
                    pts.push(cur);
                }
                cubic(cur, c1, c2, p, tolerance, 0, &mut pts);
                cur = p;
            }
            Cmd::Close => {
                finish(&mut out, &mut pts, true);
                cur = start;
            }
        }
    }
    finish(&mut out, &mut pts, false);
    out
}

/// A lone MoveTo point draws nothing (no segment, no cap — the spec's
/// rule); anything longer is a real subpath.
fn finish(out: &mut Vec<SubPath>, pts: &mut Vec<P>, closed: bool) {
    if pts.len() > 1 {
        out.push(SubPath {
            points: std::mem::take(pts),
            closed,
        });
    } else {
        pts.clear();
    }
}

/// Subdivision depth cap — 2^16 points from one cubic means the tolerance
/// was nonsense; the cap keeps a bad input finite, never a hang.
const MAX_DEPTH: u32 = 16;

fn cubic(p0: P, c1: P, c2: P, p1: P, tol: f32, depth: u32, out: &mut Vec<P>) {
    if depth >= MAX_DEPTH || flat_enough(p0, c1, c2, p1, tol) {
        out.push(p1);
        return;
    }
    let mid = |a: P, b: P| ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    let ab = mid(p0, c1);
    let bc = mid(c1, c2);
    let cd = mid(c2, p1);
    let abbc = mid(ab, bc);
    let bccd = mid(bc, cd);
    let center = mid(abbc, bccd);
    cubic(p0, ab, abbc, center, tol, depth + 1, out);
    cubic(center, bccd, cd, p1, tol, depth + 1, out);
}

fn flat_enough(p0: P, c1: P, c2: P, p1: P, tol: f32) -> bool {
    dist_to_segment(c1, p0, p1) <= tol && dist_to_segment(c2, p0, p1) <= tol
}

fn dist_to_segment(p: P, a: P, b: P) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let (wx, wy) = (p.0 - a.0, p.1 - a.1);
    let len2 = vx * vx + vy * vy;
    if len2 <= f32::EPSILON {
        return (wx * wx + wy * wy).sqrt();
    }
    let t = ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0);
    let (dx, dy) = (wx - t * vx, wy - t * vy);
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_quarter_circle() -> Vec<Cmd> {
        // (1,0) → (0,1), the standard kappa cubic.
        vec![
            Cmd::MoveTo((1.0, 0.0)),
            Cmd::CubicTo((1.0, 0.552_285), (0.552_285, 1.0), (0.0, 1.0)),
        ]
    }

    #[test]
    fn endpoints_are_exact() {
        let subs = flatten(&unit_quarter_circle(), 0.02);
        assert_eq!(subs.len(), 1);
        let pts = &subs[0].points;
        assert_eq!(pts[0], (1.0, 0.0));
        assert_eq!(*pts.last().expect("points"), (0.0, 1.0));
    }

    #[test]
    fn tighter_tolerance_means_more_points() {
        let loose = flatten(&unit_quarter_circle(), 0.05)[0].points.len();
        let tight = flatten(&unit_quarter_circle(), 0.001)[0].points.len();
        assert!(tight > loose, "tight {tight} vs loose {loose}");
    }

    #[test]
    fn every_flattened_point_stays_near_the_unit_circle() {
        let subs = flatten(&unit_quarter_circle(), 0.01);
        for &(x, y) in &subs[0].points {
            let r = (x * x + y * y).sqrt();
            assert!((r - 1.0).abs() < 0.02, "({x}, {y}) is off the circle");
        }
    }

    #[test]
    fn a_lone_moveto_draws_nothing() {
        assert!(flatten(&[Cmd::MoveTo((5.0, 5.0))], 0.02).is_empty());
    }

    #[test]
    fn close_marks_the_subpath_closed() {
        let subs = flatten(
            &[
                Cmd::MoveTo((0.0, 0.0)),
                Cmd::LineTo((1.0, 0.0)),
                Cmd::LineTo((1.0, 1.0)),
                Cmd::Close,
            ],
            0.02,
        );
        assert_eq!(subs.len(), 1);
        assert!(subs[0].closed);
    }
}
