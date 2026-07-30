//! The entry — SVG text in, stroked triangle buffers out. The shape
//! vocabulary is exactly the vendored pool's census: path, circle, rect,
//! line, polyline, polygon, ellipse. Anything else is a refusal.

use crate::flatten::flatten;
use crate::path::{parse, Cmd};
use crate::stroke::stroke;
use crate::xml::{elements, Element};
use crate::{err, SvgError};

/// Stroked triangle buffers in viewBox units — pure data, no egui types
/// (the crate's ∅-dep law). Callers map the viewBox onto a target rect
/// and bring the color.
#[derive(Debug, Clone, PartialEq)]
pub struct Tess {
    /// x, y, w, h of the source viewBox (Lucide: 0 0 24 24).
    pub viewbox: (f32, f32, f32, f32),
    pub vertices: Vec<(f32, f32)>,
    pub indices: Vec<u32>,
}

/// The circle-from-cubics constant, 4/3·tan(π/8).
const KAPPA: f32 = 0.552_285;

pub fn tessellate_icon(svg: &str, tolerance: f32) -> Result<Tess, SvgError> {
    if tolerance.is_nan() || tolerance <= 0.0 {
        return Err(err("tolerance must be positive"));
    }
    let els = elements(svg)?;
    let root = match els.first() {
        Some(el) if el.name == "svg" => el,
        _ => return Err(err("no <svg> root")),
    };
    let viewbox = parse_viewbox(
        root.attr("viewBox")
            .ok_or_else(|| err("<svg> has no viewBox"))?,
    )?;
    let root_width = match root.attr("stroke-width") {
        Some(v) => num(v, "stroke-width")?,
        None => 2.0,
    };
    for key in ["stroke-linecap", "stroke-linejoin"] {
        if let Some(v) = root.attr(key) {
            if v != "round" {
                return Err(err(format!(
                    "{key}={v:?} is outside the icon subset (round only)"
                )));
            }
        }
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for el in &els[1..] {
        let width = match el.attr("stroke-width") {
            Some(v) => num(v, "stroke-width")?,
            None => root_width,
        };
        // The pool's only non-none fill is currentColor dot-circles whose
        // disc the stroke pass already covers (r ≤ width/2) — accepted
        // under that proof, refused otherwise. No fill tessellation, by
        // design; the corpus test pins the rule against the whole pool.
        if let Some(f) = el.attr("fill") {
            if f != "none" {
                let covered =
                    el.name == "circle" && num(el.attr("r").unwrap_or("0"), "r")? <= width * 0.5;
                if !covered {
                    return Err(err(format!(
                        "<{}> fill={f:?} needs fill tessellation — outside the icon subset",
                        el.name
                    )));
                }
            }
        }
        let cmds = shape_cmds(el)?;
        for sub in flatten(&cmds, tolerance) {
            stroke(&sub, width, &mut vertices, &mut indices);
        }
    }
    if indices.is_empty() {
        return Err(err("empty icon — nothing tessellated"));
    }
    Ok(Tess {
        viewbox,
        vertices,
        indices,
    })
}

fn shape_cmds(el: &Element) -> Result<Vec<Cmd>, SvgError> {
    let get = |k: &str| -> Result<f32, SvgError> {
        num(
            el.attr(k)
                .ok_or_else(|| err(format!("<{}> is missing {k}", el.name)))?,
            k,
        )
    };
    let opt =
        |k: &str| -> Result<Option<f32>, SvgError> { el.attr(k).map(|v| num(v, k)).transpose() };
    match el.name.as_str() {
        "path" => parse(el.attr("d").ok_or_else(|| err("<path> without d"))?),
        "circle" => {
            let r = get("r")?;
            Ok(ellipse_cmds(get("cx")?, get("cy")?, r, r))
        }
        "ellipse" => Ok(ellipse_cmds(get("cx")?, get("cy")?, get("rx")?, get("ry")?)),
        "line" => Ok(vec![
            Cmd::MoveTo((get("x1")?, get("y1")?)),
            Cmd::LineTo((get("x2")?, get("y2")?)),
        ]),
        "rect" => {
            let (w, h) = (get("width")?, get("height")?);
            let r = match opt("rx")? {
                Some(v) => v,
                None => opt("ry")?.unwrap_or(0.0),
            };
            Ok(rect_cmds(
                get("x")?,
                get("y")?,
                w,
                h,
                r.min(w / 2.0).min(h / 2.0),
            ))
        }
        "polyline" | "polygon" => {
            let vals = number_list(
                el.attr("points")
                    .ok_or_else(|| err(format!("<{}> without points", el.name)))?,
            )?;
            if vals.len() < 4 || vals.len() % 2 != 0 {
                return Err(err(format!("<{}> points list is malformed", el.name)));
            }
            let mut cmds = vec![Cmd::MoveTo((vals[0], vals[1]))];
            for xy in vals[2..].chunks_exact(2) {
                cmds.push(Cmd::LineTo((xy[0], xy[1])));
            }
            if el.name == "polygon" {
                cmds.push(Cmd::Close);
            }
            Ok(cmds)
        }
        other => Err(err(format!("<{other}> is outside the icon subset"))),
    }
}

fn ellipse_cmds(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<Cmd> {
    let (kx, ky) = (KAPPA * rx, KAPPA * ry);
    vec![
        Cmd::MoveTo((cx + rx, cy)),
        Cmd::CubicTo((cx + rx, cy + ky), (cx + kx, cy + ry), (cx, cy + ry)),
        Cmd::CubicTo((cx - kx, cy + ry), (cx - rx, cy + ky), (cx - rx, cy)),
        Cmd::CubicTo((cx - rx, cy - ky), (cx - kx, cy - ry), (cx, cy - ry)),
        Cmd::CubicTo((cx + kx, cy - ry), (cx + rx, cy - ky), (cx + rx, cy)),
        Cmd::Close,
    ]
}

fn rect_cmds(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Cmd> {
    if r <= 0.0 {
        return vec![
            Cmd::MoveTo((x, y)),
            Cmd::LineTo((x + w, y)),
            Cmd::LineTo((x + w, y + h)),
            Cmd::LineTo((x, y + h)),
            Cmd::Close,
        ];
    }
    let k = KAPPA * r;
    vec![
        Cmd::MoveTo((x + r, y)),
        Cmd::LineTo((x + w - r, y)),
        Cmd::CubicTo((x + w - r + k, y), (x + w, y + r - k), (x + w, y + r)),
        Cmd::LineTo((x + w, y + h - r)),
        Cmd::CubicTo(
            (x + w, y + h - r + k),
            (x + w - r + k, y + h),
            (x + w - r, y + h),
        ),
        Cmd::LineTo((x + r, y + h)),
        Cmd::CubicTo((x + r - k, y + h), (x, y + h - r + k), (x, y + h - r)),
        Cmd::LineTo((x, y + r)),
        Cmd::CubicTo((x, y + r - k), (x + r - k, y), (x + r, y)),
        Cmd::Close,
    ]
}

fn parse_viewbox(v: &str) -> Result<(f32, f32, f32, f32), SvgError> {
    let vals = number_list(v)?;
    if vals.len() != 4 {
        return Err(err(format!("viewBox {v:?} is not four numbers")));
    }
    Ok((vals[0], vals[1], vals[2], vals[3]))
}

fn number_list(v: &str) -> Result<Vec<f32>, SvgError> {
    v.split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .map(|t| num(t, "list entry"))
        .collect()
}

fn num(v: &str, what: &str) -> Result<f32, SvgError> {
    v.trim()
        .parse()
        .map_err(|_| err(format!("{what}={v:?} is not a number")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">";

    fn icon(body: &str) -> String {
        format!("{ROOT}{body}</svg>")
    }

    #[test]
    fn a_line_tessellates_to_a_capped_bar() {
        let t = tessellate_icon(
            &icon("<line x1=\"4\" y1=\"12\" x2=\"20\" y2=\"12\" />"),
            0.02,
        )
        .expect("tessellate");
        assert_eq!(t.viewbox, (0.0, 0.0, 24.0, 24.0));
        assert!(!t.indices.is_empty());
        assert_eq!(t.indices.len() % 3, 0);
    }

    #[test]
    fn a_circle_hugs_its_radius() {
        let t = tessellate_icon(&icon("<circle cx=\"12\" cy=\"12\" r=\"10\" />"), 0.02)
            .expect("tessellate");
        // Every vertex sits within stroke half-width + tolerance of r=10.
        for &(x, y) in &t.vertices {
            let d = ((x - 12.0).powi(2) + (y - 12.0).powi(2)).sqrt();
            assert!((d - 10.0).abs() <= 1.0 + 0.05, "({x}, {y}) strays: d={d}");
        }
    }

    #[test]
    fn a_covered_fill_dot_is_accepted() {
        let t = tessellate_icon(
            &icon("<circle cx=\"7.5\" cy=\"7.5\" r=\".5\" fill=\"currentColor\" />"),
            0.02,
        )
        .expect("the stroke covers the dot");
        assert!(!t.indices.is_empty());
    }

    #[test]
    fn an_uncovered_fill_is_refused() {
        let e = tessellate_icon(
            &icon("<circle cx=\"12\" cy=\"12\" r=\"6\" fill=\"currentColor\" />"),
            0.02,
        )
        .expect_err("must refuse");
        assert!(e.0.contains("fill"), "{e}");
    }

    #[test]
    fn an_unknown_element_is_refused_by_name() {
        let e = tessellate_icon(&icon("<text x=\"0\" y=\"0\" />"), 0.02).expect_err("must refuse");
        assert!(e.0.contains("<text>"), "{e}");
    }

    #[test]
    fn the_whole_vendored_pool_tessellates() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../coldlight/assets/lucide/icons"
        );
        let mut count = 0;
        for entry in std::fs::read_dir(dir).expect("the vendored lucide pool is missing") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("svg") {
                continue;
            }
            let svg = std::fs::read_to_string(&path).expect("read svg");
            let t =
                tessellate_icon(&svg, 0.02).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(!t.indices.is_empty(), "{}: empty", path.display());
            let max = *t.indices.iter().max().expect("indices") as usize;
            assert!(
                max < t.vertices.len(),
                "{}: index out of range",
                path.display()
            );
            count += 1;
        }
        assert!(
            count > 1700,
            "pool census: expected the full lucide inventory, found {count}"
        );
    }
}
