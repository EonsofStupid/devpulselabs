//! The vector engine — pure geometry as epaint meshes. Timing lives in the
//! generated motion runtime (coldlight) — this crate is shape only.
//!
//! epaint has no arc primitive, so curved shapes are built here as triangle
//! meshes: ring sectors between two point rings, thick polylines as quad
//! strips, area fills as strips down to a baseline. This crate knows NOTHING
//! about tones, registries, or slides — callers bring a `Color32`, geometry
//! comes back. Colors live in coldlight; composition lives in slides.
//!
//! Angles are radians in egui screen convention: 0 points +x, positive turns
//! clockwise (y grows downward). Every mesh function appends to a fresh
//! `Mesh` the caller hands to a painter.

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{Color32, Pos2, Vec2};

/// Segments per full turn when tessellating arcs. 96 keeps a wall-sized
/// donut visually circular; small charts just waste a few triangles.
const SEGS_PER_TURN: f32 = 96.0;

fn vertex(pos: Pos2, color: Color32) -> Vertex {
    Vertex { pos, uv: WHITE_UV, color }
}

/// How many segments an arc of `sweep` radians deserves (at least 1).
fn arc_segments(sweep: f32) -> usize {
    ((sweep.abs() / std::f32::consts::TAU) * SEGS_PER_TURN).ceil().max(1.0) as usize
}

/// A filled ring sector (donut slice): the area between radii `r_in` and
/// `r_out`, from angle `a0` to `a1`. `r_in = 0` degenerates to a pie wedge
/// (the inner ring collapses to the center — harmless duplicate vertices).
/// `color_out` lets a slice carry a radial gradient tint; pass the same color
/// twice for a solid fill.
pub fn ring_sector(
    center: Pos2,
    r_in: f32,
    r_out: f32,
    a0: f32,
    a1: f32,
    color_in: Color32,
    color_out: Color32,
) -> Mesh {
    let segs = arc_segments(a1 - a0);
    let mut mesh = Mesh::default();
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let a = a0 + (a1 - a0) * t;
        let dir = Vec2::new(a.cos(), a.sin());
        mesh.vertices.push(vertex(center + dir * r_in, color_in));
        mesh.vertices.push(vertex(center + dir * r_out, color_out));
    }
    for i in 0..segs as u32 {
        let base = i * 2;
        // two triangles per quad between consecutive spokes
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    mesh
}

/// An arc stroke: a ring sector centered on radius `r` with thickness `width`.
/// The crisp way to draw meter tracks and progress sweeps.
pub fn arc_stroke(center: Pos2, r: f32, width: f32, a0: f32, a1: f32, color: Color32) -> Mesh {
    ring_sector(center, r - width * 0.5, r + width * 0.5, a0, a1, color, color)
}

/// A thick polyline as a quad strip with miter-free joins (each joint uses
/// the average of adjacent segment normals — right for the gentle curves
/// charts draw; sharp zigzags would want caps this crate doesn't need yet).
pub fn polyline_stroke(points: &[Pos2], width: f32, color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    if points.len() < 2 {
        return mesh;
    }
    let half = width * 0.5;
    let normal = |a: Pos2, b: Pos2| -> Vec2 {
        let d = b - a;
        let len = d.length().max(f32::EPSILON);
        Vec2::new(-d.y / len, d.x / len)
    };
    for i in 0..points.len() {
        let n = if i == 0 {
            normal(points[0], points[1])
        } else if i == points.len() - 1 {
            normal(points[i - 1], points[i])
        } else {
            let a = normal(points[i - 1], points[i]);
            let b = normal(points[i], points[i + 1]);
            let sum = a + b;
            let len = sum.length().max(f32::EPSILON);
            sum / len
        };
        mesh.vertices.push(vertex(points[i] + n * half, color));
        mesh.vertices.push(vertex(points[i] - n * half, color));
    }
    for i in 0..(points.len() - 1) as u32 {
        let base = i * 2;
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    mesh
}

/// The area under a curve: a quad strip from each point straight down to
/// `baseline_y`. `color_top` → `color_base` is the vertical wash gradient.
pub fn area_fill(points: &[Pos2], baseline_y: f32, color_top: Color32, color_base: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    if points.len() < 2 {
        return mesh;
    }
    for p in points {
        mesh.vertices.push(vertex(*p, color_top));
        mesh.vertices.push(vertex(Pos2::new(p.x, baseline_y), color_base));
    }
    for i in 0..(points.len() - 1) as u32 {
        let base = i * 2;
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    mesh
}

/// Sample a smooth monotone curve through `points` (Fritsch–Carlson tangents,
/// the same family d3's curveMonotoneX uses): no overshoot, no invented
/// extrema — a chart line never dips below a data floor. Returns `samples`
/// evenly-spaced positions from first to last x. Assumes points sorted by x.
pub fn sample_monotone(points: &[Pos2], samples: usize) -> Vec<Pos2> {
    if points.len() < 2 || samples < 2 {
        return points.to_vec();
    }
    let n = points.len();
    // Secant slopes, then monotone-limited tangents.
    let mut d = vec![0.0f32; n - 1];
    for i in 0..n - 1 {
        let dx = (points[i + 1].x - points[i].x).max(f32::EPSILON);
        d[i] = (points[i + 1].y - points[i].y) / dx;
    }
    let mut m = vec![0.0f32; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        m[i] = if d[i - 1] * d[i] <= 0.0 { 0.0 } else { (d[i - 1] + d[i]) * 0.5 };
    }
    for i in 0..n - 1 {
        if d[i] == 0.0 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
        } else {
            let a = m[i] / d[i];
            let b = m[i + 1] / d[i];
            let s = (a * a + b * b).sqrt();
            if s > 3.0 {
                m[i] = 3.0 * d[i] * a / s;
                m[i + 1] = 3.0 * d[i] * b / s;
            }
        }
    }
    let x0 = points[0].x;
    let x1 = points[n - 1].x;
    let mut out = Vec::with_capacity(samples);
    let mut seg = 0usize;
    for i in 0..samples {
        let x = x0 + (x1 - x0) * (i as f32 / (samples - 1) as f32);
        while seg < n - 2 && x > points[seg + 1].x {
            seg += 1;
        }
        let h = (points[seg + 1].x - points[seg].x).max(f32::EPSILON);
        let t = ((x - points[seg].x) / h).clamp(0.0, 1.0);
        let t2 = t * t;
        let t3 = t2 * t;
        let y = points[seg].y * (2.0 * t3 - 3.0 * t2 + 1.0)
            + m[seg] * h * (t3 - 2.0 * t2 + t)
            + points[seg + 1].y * (-2.0 * t3 + 3.0 * t2)
            + m[seg + 1] * h * (t3 - t2);
        out.push(Pos2::new(x, y));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{PI, TAU};

    #[test]
    fn ring_sector_meshes_a_quad_strip() {
        let m = ring_sector(Pos2::ZERO, 10.0, 20.0, 0.0, PI, Color32::WHITE, Color32::WHITE);
        let segs = arc_segments(PI);
        assert_eq!(m.vertices.len(), (segs + 1) * 2);
        assert_eq!(m.indices.len(), segs * 6);
        // Every inner vertex sits at r_in, every outer at r_out.
        for pair in m.vertices.chunks(2) {
            assert!((pair[0].pos.to_vec2().length() - 10.0).abs() < 0.001);
            assert!((pair[1].pos.to_vec2().length() - 20.0).abs() < 0.001);
        }
    }

    #[test]
    fn full_turn_closes_on_itself() {
        let m = ring_sector(Pos2::ZERO, 5.0, 8.0, 0.0, TAU, Color32::WHITE, Color32::WHITE);
        let first = m.vertices[0].pos;
        let last = m.vertices[m.vertices.len() - 2].pos;
        assert!((first - last).length() < 0.001, "first and last spokes must coincide");
    }

    #[test]
    fn polyline_needs_two_points() {
        assert!(polyline_stroke(&[Pos2::ZERO], 4.0, Color32::WHITE).vertices.is_empty());
        let m = polyline_stroke(&[Pos2::ZERO, Pos2::new(10.0, 0.0)], 4.0, Color32::WHITE);
        assert_eq!(m.vertices.len(), 4);
        assert_eq!(m.indices.len(), 6);
    }

    #[test]
    fn area_fill_reaches_the_baseline() {
        let pts = [Pos2::new(0.0, 5.0), Pos2::new(10.0, 2.0)];
        let m = area_fill(&pts, 20.0, Color32::WHITE, Color32::TRANSPARENT);
        assert!(m.vertices.iter().skip(1).step_by(2).all(|v| v.pos.y == 20.0));
    }

    #[test]
    fn monotone_curve_hits_the_data_points_and_never_overshoots() {
        let pts = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(20.0, 10.0),
            Pos2::new(30.0, 40.0),
        ];
        let s = sample_monotone(&pts, 61);
        assert!((s[0].y - 0.0).abs() < 0.01);
        assert!((s[60].y - 40.0).abs() < 0.01);
        // Monotone data segment [10,10]→[20,10] must stay flat — no invented dip.
        for p in s.iter().filter(|p| p.x >= 10.0 && p.x <= 20.0) {
            assert!((p.y - 10.0).abs() < 0.5, "overshoot at x={}: y={}", p.x, p.y);
        }
    }


}
