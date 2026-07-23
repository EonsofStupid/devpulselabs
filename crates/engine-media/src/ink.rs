//! Ink flattening (C4 — ViziWiz): rasterize strokes onto an image and
//! save the composite as a NEW file. The source is never modified
//! (the never-mutate content law); the caller owns naming and filing.
//!
//! Strokes are polylines in IMAGE pixel space. Each segment is stamped
//! as filled discs marched at half-radius spacing — round caps and
//! joins fall out of the disc shape, matching the old canvas editor's
//! `lineCap: round` look without a path-stroking dependency.

use std::path::Path;

/// One stroke, image pixel space. `width` is the full stroke width.
#[derive(Clone, Debug)]
pub struct InkStroke {
    pub color: [u8; 3],
    pub width: f32,
    pub points: Vec<[f32; 2]>,
}

/// Stamp a filled disc. Bounds-checked; opaque ink.
fn disc(img: &mut image::RgbaImage, cx: f32, cy: f32, r: f32, color: [u8; 3]) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let r2 = r * r;
    let (x0, x1) = ((cx - r).floor() as i64, (cx + r).ceil() as i64);
    let (y0, y1) = ((cy - r).floor() as i64, (cy + r).ceil() as i64);
    for y in y0.max(0)..=y1.min(h - 1) {
        for x in x0.max(0)..=x1.min(w - 1) {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            if dx * dx + dy * dy <= r2 {
                img.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgba([color[0], color[1], color[2], 255]),
                );
            }
        }
    }
}

/// March a segment stamping discs at ≤ r/2 spacing (continuous cover).
fn segment(img: &mut image::RgbaImage, a: [f32; 2], b: [f32; 2], r: f32, color: [u8; 3]) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len / (r * 0.5).max(0.5)).ceil().max(1.0) as u32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        disc(img, a[0] + dx * t, a[1] + dy * t, r, color);
    }
}

/// Composite `strokes` over `src` and save the result to `dest` (PNG by
/// extension). `src` is read-only; refusals are strings, never panics.
pub fn flatten_ink(src: &Path, dest: &Path, strokes: &[InkStroke]) -> Result<(), String> {
    let img = image::open(src).map_err(|e| format!("open {}: {e}", src.display()))?;
    let mut out = img.to_rgba8();
    for s in strokes {
        let r = (s.width / 2.0).max(1.0);
        match s.points.as_slice() {
            [] => {}
            [only] => disc(&mut out, only[0], only[1], r, s.color),
            many => {
                for pair in many.windows(2) {
                    segment(&mut out, pair[0], pair[1], r, s.color);
                }
            }
        }
    }
    out.save(dest)
        .map_err(|e| format!("save {}: {e}", dest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_inks_the_path_and_never_touches_the_source() {
        let dir = std::env::temp_dir().join(format!("dpl-ink-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("base.png");
        let dest = dir.join("annot.png");
        image::RgbaImage::from_pixel(32, 32, image::Rgba([255, 255, 255, 255]))
            .save(&src)
            .unwrap();
        let before = std::fs::read(&src).unwrap();

        flatten_ink(
            &src,
            &dest,
            &[InkStroke {
                color: [255, 0, 0],
                width: 4.0,
                points: vec![[0.0, 0.0], [31.0, 31.0]],
            }],
        )
        .unwrap();

        let out = image::open(&dest).unwrap().to_rgba8();
        assert_eq!(
            out.get_pixel(16, 16),
            &image::Rgba([255, 0, 0, 255]),
            "the diagonal runs through the center"
        );
        assert_eq!(
            out.get_pixel(30, 2),
            &image::Rgba([255, 255, 255, 255]),
            "off-path pixels stay untouched"
        );
        assert_eq!(
            std::fs::read(&src).unwrap(),
            before,
            "the source is NEVER modified"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refusals_are_strings() {
        assert!(flatten_ink(Path::new("does-not-exist.png"), Path::new("out.png"), &[]).is_err());
    }
}
