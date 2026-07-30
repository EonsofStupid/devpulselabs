//! Polyline stroking — a quad per segment plus a disc where the path
//! needs one: open endpoints (round caps) and sharp joints (round
//! joins). Smooth flattened curves turn a few degrees per vertex, where
//! the quad-to-quad wedge stays sub-pixel — no disc needed. The overlap
//! at discs is deliberate and recorded: meshes are built for OPAQUE
//! tinting (doctrine ink); a semi-transparent tint would reveal the
//! double-cover — paint washes as separate layers instead.

use crate::flatten::SubPath;
use crate::path::P;

/// Disc resolution: at icon stroke radii (1 unit in a 24 box) sixteen
/// segments stay visually circular past 500 px renders.
const DISC_SEGS: u32 = 16;

/// A joint keeps its disc only past ~15° of turn (cos 0.966): below
/// that, the outer wedge gap is r·(1−cos θ/2) < r/100 — sub-pixel at
/// any icon size.
const SHARP_COS: f32 = 0.966;

pub(crate) fn stroke(sub: &SubPath, width: f32, verts: &mut Vec<P>, idx: &mut Vec<u32>) {
    let r = width * 0.5;
    // Exact consecutive duplicates add nothing a disc doesn't paint.
    let mut pts: Vec<P> = Vec::with_capacity(sub.points.len());
    for &p in &sub.points {
        if pts.last() != Some(&p) {
            pts.push(p);
        }
    }
    if sub.closed && pts.len() > 2 && pts.first() == pts.last() {
        pts.pop();
    }
    let n = pts.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        // A zero-length segment with round caps is a dot — the spec's
        // rule, and Lucide's ".01-long" dots depend on it.
        disc(pts[0], r, verts, idx);
        return;
    }
    let seg_count = if sub.closed { n } else { n - 1 };
    for s in 0..seg_count {
        quad(pts[s], pts[(s + 1) % n], r, verts, idx);
    }
    for v in 0..n {
        let needs_disc = if !sub.closed && (v == 0 || v == n - 1) {
            true // an open endpoint always wears its cap
        } else {
            turn_is_sharp(pts[(v + n - 1) % n], pts[v], pts[(v + 1) % n])
        };
        if needs_disc {
            disc(pts[v], r, verts, idx);
        }
    }
}

fn quad(a: P, b: P, r: f32, verts: &mut Vec<P>, idx: &mut Vec<u32>) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f32::EPSILON {
        return;
    }
    let (nx, ny) = (-dy / len * r, dx / len * r);
    let base = verts.len() as u32;
    verts.push((a.0 + nx, a.1 + ny));
    verts.push((a.0 - nx, a.1 - ny));
    verts.push((b.0 + nx, b.1 + ny));
    verts.push((b.0 - nx, b.1 - ny));
    idx.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
}

fn disc(c: P, r: f32, verts: &mut Vec<P>, idx: &mut Vec<u32>) {
    let base = verts.len() as u32;
    verts.push(c);
    for k in 0..DISC_SEGS {
        let t = k as f32 / DISC_SEGS as f32 * std::f32::consts::TAU;
        verts.push((c.0 + r * t.cos(), c.1 + r * t.sin()));
    }
    for k in 0..DISC_SEGS {
        let a = base + 1 + k;
        let b = base + 1 + (k + 1) % DISC_SEGS;
        idx.extend_from_slice(&[base, a, b]);
    }
}

fn turn_is_sharp(a: P, b: P, c: P) -> bool {
    let (ux, uy) = (b.0 - a.0, b.1 - a.1);
    let (vx, vy) = (c.0 - b.0, c.1 - b.1);
    let lu = (ux * ux + uy * uy).sqrt();
    let lv = (vx * vx + vy * vy).sqrt();
    if lu <= f32::EPSILON || lv <= f32::EPSILON {
        return true;
    }
    (ux * vx + uy * vy) / (lu * lv) < SHARP_COS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffers_of(sub: &SubPath) -> (Vec<P>, Vec<u32>) {
        let mut v = Vec::new();
        let mut i = Vec::new();
        stroke(sub, 2.0, &mut v, &mut i);
        (v, i)
    }

    #[test]
    fn indices_stay_in_range_and_triangulate() {
        let (v, i) = buffers_of(&SubPath {
            points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            closed: false,
        });
        assert!(!i.is_empty());
        assert_eq!(i.len() % 3, 0);
        let max = *i.iter().max().expect("indices") as usize;
        assert!(max < v.len());
    }

    #[test]
    fn a_dot_paints_a_disc() {
        let (v, i) = buffers_of(&SubPath {
            points: vec![(12.0, 12.0), (12.0, 12.0)],
            closed: false,
        });
        assert_eq!(v.len(), 1 + DISC_SEGS as usize);
        assert_eq!(i.len(), DISC_SEGS as usize * 3);
    }

    #[test]
    fn a_right_angle_earns_its_join_disc_but_a_gentle_turn_does_not() {
        let (v_sharp, _) = buffers_of(&SubPath {
            points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            closed: false,
        });
        let (v_gentle, _) = buffers_of(&SubPath {
            points: vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.5)],
            closed: false,
        });
        // Same shape count except the middle joint's disc.
        assert_eq!(v_sharp.len() - v_gentle.len(), 1 + DISC_SEGS as usize);
    }

    #[test]
    fn a_closed_ring_needs_no_caps() {
        // A hexagon ring: 6 quads, 6 sharp joins (60° turns), no caps.
        let mut points = Vec::new();
        for k in 0..6 {
            let t = k as f32 / 6.0 * std::f32::consts::TAU;
            points.push((t.cos() * 10.0, t.sin() * 10.0));
        }
        let (v, _) = buffers_of(&SubPath {
            points,
            closed: true,
        });
        assert_eq!(v.len(), 6 * 4 + 6 * (1 + DISC_SEGS as usize));
    }
}
