//! The ColdLight generator — the palette ENGINE, ported faithfully.
//!
//! `tokens/hues.json` (anchor {hue, chroma} + named curves) and
//! `tokens/fluid.json` (min/max rem pairs) are the AUTHORED sources, vendored
//! verbatim from the prior repo. This build script is the Rust port of
//! `generate-scales.mjs` + `fluid-axis.mjs`: every hue resolves to a 13-step
//! OKLCH ladder via a LIGHTNESS curve × a CHROMA-multiplier curve (NOT a flat
//! constant-chroma ramp — the curves are the whole point), converted to sRGB
//! with gamut clamping by `palette`, and emitted as `Color32` constants.
//! Fluid tokens become methods on `Fluid` with precomputed XY-clamp slopes.
//!
//! Editing a token JSON and running `cargo build` IS the design build.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use palette::{Clamp, IntoColor, Oklch, Srgb};

const LADDER_STEPS: [u32; 13] = [
    25, 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950, 975,
];
/// Never pure black from a curve — authored overrides opt out by existing.
const LIGHTNESS_FLOOR: f64 = 0.10;

/// Target lightness per step, per curve. Ported verbatim.
fn lightness(curve: &str) -> Option<[f64; 13]> {
    Some(match curve {
        "brand" | "perceptual" => [
            0.99, 0.98, 0.95, 0.90, 0.84, 0.79, 0.72, 0.62, 0.52, 0.42, 0.32, 0.22, 0.14,
        ],
        "neutral" => [
            0.99, 0.98, 0.94, 0.86, 0.72, 0.58, 0.44, 0.32, 0.24, 0.20, 0.16, 0.13, 0.11,
        ],
        "muted" => [
            0.97, 0.94, 0.88, 0.80, 0.72, 0.62, 0.52, 0.44, 0.36, 0.28, 0.22, 0.16, 0.12,
        ],
        "high-contrast" => [
            0.995, 0.99, 0.96, 0.90, 0.82, 0.72, 0.60, 0.48, 0.36, 0.26, 0.18, 0.14, 0.10,
        ],
        "low-key" => [
            0.85, 0.78, 0.70, 0.62, 0.54, 0.46, 0.38, 0.32, 0.26, 0.22, 0.18, 0.14, 0.11,
        ],
        "luminous" => [
            0.99, 0.97, 0.94, 0.90, 0.86, 0.82, 0.78, 0.72, 0.60, 0.46, 0.34, 0.24, 0.15,
        ],
        _ => return None,
    })
}

/// Chroma multiplier per step, per curve. Ported verbatim.
fn chroma_mult(curve: &str) -> Option<[f64; 13]> {
    Some(match curve {
        "linear-to-500" => [
            0.08, 0.15, 0.30, 0.55, 0.80, 0.95, 1.0, 1.0, 0.90, 0.75, 0.55, 0.35, 0.22,
        ],
        "flat" => [1.0; 13],
        "soft" => [
            0.25, 0.40, 0.55, 0.70, 0.85, 0.95, 1.0, 0.95, 0.85, 0.70, 0.55, 0.40, 0.28,
        ],
        "peak-at-400" => [
            0.12, 0.22, 0.40, 0.68, 0.90, 1.0, 0.95, 0.82, 0.68, 0.55, 0.42, 0.28, 0.18,
        ],
        "peak-at-600" => [
            0.08, 0.15, 0.28, 0.48, 0.68, 0.85, 0.95, 1.0, 0.98, 0.85, 0.65, 0.42, 0.26,
        ],
        "bloom" => [
            0.45, 0.60, 0.75, 0.88, 0.96, 1.0, 0.98, 0.88, 0.72, 0.55, 0.38, 0.24, 0.14,
        ],
        _ => return None,
    })
}

fn oklch_to_rgb8(l: f64, c: f64, h: f64) -> (u8, u8, u8) {
    let rgb: Srgb = Oklch::new(l as f32, c as f32, h as f32).into_color();
    // Gamut handling: clamp components — the same clipping culori's formatHex
    // applied in the JS pipeline ("hex is a convenience mirror; components are
    // truth" — and we, like the browser, must land in sRGB to paint).
    let rgb = rgb.clamp();
    let (r, g, b) = rgb.into_format::<u8>().into_components();
    (r, g, b)
}

fn ident(name: &str) -> String {
    name.replace('-', "_")
}

fn main() {
    println!("cargo:rerun-if-changed=tokens/hues.json");
    println!("cargo:rerun-if-changed=tokens/fluid.json");
    println!("cargo:rerun-if-changed=tokens/motion.json");
    println!("cargo:rerun-if-changed=tokens/moods.json");
    println!("cargo:rerun-if-changed=tokens/fonts.json");
    println!("cargo:rerun-if-changed=assets/fonts");
    println!("cargo:rerun-if-changed=tokens/presets.json");
    println!("cargo:rerun-if-changed=tokens/traits.json");
    println!("cargo:rerun-if-changed=tokens/icons.json");
    println!("cargo:rerun-if-changed=assets/lucide");
    println!("cargo:rerun-if-changed=tokens/matrix.json");
    println!("cargo:rerun-if-changed=tokens/roles.json");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");

    generate_ramps(&out_dir);
    generate_fluid(&out_dir);
    generate_motion(&out_dir);
    generate_aurora(&out_dir);
    generate_moods(&out_dir);
    generate_fonts(&out_dir);
    generate_presets(&out_dir);
    generate_traits(&out_dir);
    generate_icons(&out_dir);
    generate_roles(&out_dir);
    generate_matrix(&out_dir);
}

/// A coordinate is LEGAL when its lineage rules on every facet it names.
///
/// Shared by `generate_roles` and `generate_presets` so a role binding and an
/// authored preset can never be validated by two different rules. Fails the
/// BUILD rather than degrading: an illegal preset that ships is mush no one
/// can explain, and the matrix exists precisely to make that impossible.
fn assert_coordinate_legal(
    matrix: &serde_json::Value,
    ctx: &str,
    anc: &str,
    st: &str,
    mt: &str,
    at: &str,
) {
    let a = matrix["ancestors"]
        .as_array()
        .expect("matrix.json: ancestors[]")
        .iter()
        .find(|x| x["id"].as_str() == Some(anc))
        .unwrap_or_else(|| panic!("{ctx}: unknown ancestor {anc:?}"));
    for (facet, value) in [("structure", st), ("material", mt), ("atmosphere", at)] {
        let in_set = |which: &str| -> bool {
            a[which][facet]
                .as_array()
                .map(|x| x.iter().any(|v| v.as_str() == Some(value)))
                .unwrap_or(false)
        };
        assert!(
            !in_set("forbidden"),
            "{ctx} is ILLEGAL — {anc} forbids {facet} {value:?}"
        );
        assert!(
            a["native"][facet].as_str() == Some(value) || in_set("allowed"),
            "{ctx} names {facet} {value:?}, which {anc} does not rule on"
        );
    }
}

/// tokens/roles.json → the SEMANTIC layer: role → treatment, per surface.
///
/// Three refusals, all at build time:
/// 1. every role names a trait that EXISTS in traits.json;
/// 2. a pinned `tone` names a real hue;
/// 3. `default` carries every role — a surface may omit one and fall back,
///    but there must always be something to fall back TO.
fn generate_roles(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/roles.json").expect("read roles.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse roles.json");

    // The treatments a role is allowed to point at.
    let traits_raw = std::fs::read_to_string("tokens/traits.json").expect("read traits.json");
    let traits_doc: serde_json::Value =
        serde_json::from_str(&traits_raw).expect("parse traits.json");
    let trait_ids: Vec<String> = traits_doc["traits"]
        .as_array()
        .expect("traits.json: traits[]")
        .iter()
        .map(|t| t["id"].as_str().expect("trait id").to_owned())
        .collect();
    let hues_raw = std::fs::read_to_string("tokens/hues.json").expect("read hues.json");
    let hues_doc: serde_json::Value = serde_json::from_str(&hues_raw).expect("parse hues.json");

    const ROLES: [&str; 4] = ["chrome", "hero", "quiet", "danger"];

    // A role points at EITHER a legacy trait or a MATRIX COORDINATE. The
    // coordinate form is checked here against the real facet pools AND the
    // ancestor's own legality rulings, so an illegal preset cannot ship — it
    // fails the build exactly as an unknown trait already does.
    let matrix_raw = std::fs::read_to_string("tokens/matrix.json").expect("read matrix.json");
    let matrix_doc: serde_json::Value =
        serde_json::from_str(&matrix_raw).expect("parse matrix.json");

    let binding = |scope: &str, role: &str, v: &serde_json::Value| -> String {
        let dress = if let Some(c) = v.get("coordinate") {
            let get = |k: &str| -> String {
                c[k].as_str()
                    .unwrap_or_else(|| panic!("roles.json: {scope}.{role}.coordinate needs {k:?}"))
                    .to_owned()
            };
            let anc = get("ancestor");
            let st = get("structure");
            let mt = get("material");
            let at = get("atmosphere");
            let a = matrix_doc["ancestors"]
                .as_array()
                .expect("ancestors[]")
                .iter()
                .find(|x| x["id"].as_str() == Some(anc.as_str()))
                .unwrap_or_else(|| {
                    panic!("roles.json: {scope}.{role} names unknown ancestor {anc:?}")
                });
            let _ = a;
            assert_coordinate_legal(
                &matrix_doc,
                &format!("roles.json: {scope}.{role}"),
                &anc,
                &st,
                &mt,
                &at,
            );
            format!(
                "Dress::Coord(matrix::Coordinate {{ ancestor: {anc:?}, structure: {st:?}, \
                 material: {mt:?}, atmosphere: {at:?} }})"
            )
        } else {
            let t = v["trait"].as_str().unwrap_or_else(|| {
                panic!("roles.json: {scope}.{role} names neither `trait` nor `coordinate`")
            });
            assert!(
                trait_ids.iter().any(|x| x == t),
                "roles.json: {scope}.{role} names unknown treatment {t:?}"
            );
            format!("Dress::Trait({t:?})")
        };
        let tone = match v.get("tone").and_then(|x| x.as_str()) {
            Some(h) => {
                assert!(
                    hue_exists(&hues_doc, h),
                    "roles.json: {scope}.{role} pins unknown hue {h:?}"
                );
                format!("Some({h:?})")
            }
            None => "None".to_string(),
        };
        format!("Binding {{ dress: {dress}, tone: {tone} }}")
    };

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/roles.json — DO NOT EDIT."
    )
    .unwrap();

    // ── default: every role, no exceptions ──
    let def = doc["default"].as_object().expect("roles.json: default{}");
    writeln!(code, "const DEFAULT_ROLES: [Binding; 4] = [").unwrap();
    for role in ROLES {
        let v = def.get(role).unwrap_or_else(|| {
            panic!("roles.json: default is missing {role:?} — every role needs a fallback")
        });
        writeln!(code, "    {},", binding("default", role, v)).unwrap();
    }
    writeln!(code, "];").unwrap();

    // ── surfaces: sparse, each role Option<Binding> ──
    let surfaces = doc["surfaces"].as_object().expect("roles.json: surfaces{}");
    writeln!(
        code,
        "const SURFACE_ROLES: &[(&str, [Option<Binding>; 4])] = &["
    )
    .unwrap();
    for (sid, over) in surfaces {
        let o = over
            .as_object()
            .unwrap_or_else(|| panic!("roles.json: surface {sid:?} is not an object"));
        for k in o.keys() {
            assert!(
                ROLES.contains(&k.as_str()),
                "roles.json: surface {sid:?} names unknown role {k:?}"
            );
        }
        let cells: Vec<String> = ROLES
            .iter()
            .map(|r| match o.get(*r) {
                Some(v) => format!("Some({})", binding(sid, r, v)),
                None => "None".to_string(),
            })
            .collect();
        writeln!(code, "    ({sid:?}, [{}]),", cells.join(", ")).unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(
        code,
        "pub const ROLE_COUNT: usize = {};
pub const SURFACE_COUNT: usize = {};",
        ROLES.len(),
        surfaces.len()
    )
    .unwrap();

    std::fs::write(Path::new(out_dir).join("roles_gen.rs"), code).expect("write roles_gen.rs");
}

/// Is `id` a hue in the authored pool? (roles.json may pin one.)
fn hue_exists(hues: &serde_json::Value, id: &str) -> bool {
    fn scan(v: &serde_json::Value, id: &str) -> bool {
        match v {
            serde_json::Value::Object(m) => {
                if m.get("id").and_then(|x| x.as_str()) == Some(id) {
                    return true;
                }
                m.values().any(|x| scan(x, id))
            }
            serde_json::Value::Array(a) => a.iter().any(|x| scan(x, id)),
            _ => false,
        }
    }
    scan(hues, id)
}

// ── the MATRIX — faceted classification (docs/design/coldlight-matrix.md) ──

/// tokens/matrix.json → the facet id pools and the legality table.
///
/// Four checks, and each one FAILS THE BUILD rather than degrading, because
/// a matrix that silently permits a contradiction produces mush (§5):
///
/// 1. **Kebab ids** — the naming law (architecture.md §4).
/// 2. **Every font ref resolves** — the manifest⇄truth law fonts already use.
/// 3. **Total and disjoint** — every facet value appears in EXACTLY ONE of
///    an ancestor's `native`/`allowed`/`forbidden`, for EVERY ancestor. A
///    value left unclassified fails; a value classified twice fails. This is
///    what stops the matrix rotting as values are added: adding a material
///    forces all eight ancestors to rule on it.
/// 4. **`native` is one value per facet** — an ancestor alone must resolve
///    to a complete look, so its natives cannot be absent or plural.
fn generate_matrix(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/matrix.json").expect("read matrix.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse matrix.json");

    let kebab = |s: &str| {
        !s.is_empty()
            && s.split('-').all(|p| {
                !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
    };
    let ids_of = |key: &str| -> Vec<String> {
        doc[key]
            .as_array()
            .unwrap_or_else(|| panic!("matrix.json: {key}[]"))
            .iter()
            .map(|v| {
                let id = v["id"].as_str().expect("facet id").to_owned();
                assert!(kebab(&id), "matrix.json: id {id:?} is not kebab-case");
                id
            })
            .collect()
    };

    let structures = ids_of("structures");
    let materials = ids_of("materials");
    let atmospheres = ids_of("atmospheres");
    let ancestors = doc["ancestors"]
        .as_array()
        .expect("matrix.json: ancestors[]");

    // The font pool is the truth an ancestor's `font` must resolve against.
    let fonts_raw = std::fs::read_to_string("tokens/fonts.json").expect("read fonts.json");
    let fonts_doc: serde_json::Value = serde_json::from_str(&fonts_raw).expect("parse fonts.json");
    let font_ids: Vec<&str> = fonts_doc["fonts"]
        .as_array()
        .expect("fonts.json: fonts[]")
        .iter()
        .map(|f| f["id"].as_str().expect("font id"))
        .collect();

    let facets: [(&str, &Vec<String>); 3] = [
        ("structure", &structures),
        ("material", &materials),
        ("atmosphere", &atmospheres),
    ];

    let mut rulings = 0usize;
    for a in ancestors {
        let aid = a["id"].as_str().expect("ancestor id");
        assert!(
            kebab(aid),
            "matrix.json: ancestor {aid:?} is not kebab-case"
        );
        let font = a["font"].as_str().expect("ancestor font");
        assert!(
            font_ids.contains(&font),
            "matrix.json: ancestor {aid:?} wears unknown font {font:?}"
        );

        for (facet, pool) in facets {
            let native = a["native"][facet]
                .as_str()
                .unwrap_or_else(|| panic!("matrix.json: {aid}.native.{facet} must be ONE value"));
            let list = |set: &str| -> Vec<String> {
                a[set][facet]
                    .as_array()
                    .map(|v| {
                        v.iter()
                            .map(|x| x.as_str().expect("facet ref").to_owned())
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let mut seen: Vec<String> = vec![native.to_owned()];
            seen.extend(list("allowed"));
            seen.extend(list("forbidden"));

            for v in &seen {
                assert!(
                    pool.contains(v),
                    "matrix.json: {aid}.{facet} references unknown value {v:?}"
                );
            }
            for v in pool {
                let n = seen.iter().filter(|s| *s == v).count();
                assert!(
                    n == 1,
                    "matrix.json: {aid}.{facet} classifies {v:?} {n} times — every value must \
                     appear in EXACTLY ONE of native/allowed/forbidden (matrix §5)"
                );
                rulings += 1;
            }
        }
    }

    // ── emit the PAYLOADS, not just the ids ──────────────────────────
    // Until now every authored value (radius, fill, shadows, …) was parsed
    // for validation and discarded, so the matrix could be checked but never
    // painted. The facet defs below reuse the SAME ColorRef/Axes primitives
    // the traits emit, which is what lets `resolve()` and `compose()` produce
    // byte-identical output for the same look.
    let hue_ids = pool_hue_ids();
    let obj = |key: &str| doc[key].as_array().expect("facet array");
    let sq = |v: &serde_json::Value, k: &str| v[k].as_str().unwrap_or_default().to_owned();

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/matrix.json — DO NOT EDIT."
    )
    .unwrap();

    // The id pools stay, so the census tests and any id-keyed lookup keep working.
    for (name, pool) in [
        ("STRUCTURE_IDS", &structures),
        ("MATERIAL_IDS", &materials),
        ("ATMOSPHERE_IDS", &atmospheres),
    ] {
        writeln!(code, "pub static {name}: &[&str] = &[").unwrap();
        for id in pool {
            writeln!(code, "    {id:?},").unwrap();
        }
        writeln!(code, "];").unwrap();
    }
    writeln!(code, "pub static ANCESTOR_IDS: &[&str] = &[").unwrap();
    for a in ancestors {
        writeln!(code, "    {:?},", a["id"].as_str().unwrap()).unwrap();
    }
    writeln!(code, "];").unwrap();

    // STRUCTURE — geometry only: radius + edge WIDTH (the colour is the
    // material's, which is matrix law M4: an edge is assembled from two facets).
    writeln!(code, "pub static STRUCTURES: &[StructureDef] = &[").unwrap();
    for v in obj("structures") {
        writeln!(
            code,
            "    StructureDef {{ id: {:?}, label: {:?}, blurb: {:?}, radius: {}, edge_width: {} }},",
            sq(v, "id"),
            sq(v, "label"),
            sq(v, "blurb"),
            // egui's CornerRadius is u8 and treats 255 as fully rounded, so
            // `pill`'s authored 999 clamps rather than overflowing. The
            // authored value stays expressive; the emitted one stays legal.
            v["radius"].as_u64().expect("structure.radius").min(255),
            f32_lit_expr(v["edge_width"].as_f64().expect("structure.edge_width"))
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();

    // MATERIAL — surface response. Every field is Option because law M2 says
    // an axis a material does not declare falls back to the ancestor's native
    // material (neon-glass declares no fill and inherits clay's on aero).
    writeln!(code, "pub static MATERIALS: &[MaterialDef] = &[").unwrap();
    for v in obj("materials") {
        let id = sq(v, "id");
        let opt = |k: &str| match v.get(k) {
            Some(c) => format!(
                "Some({})",
                color_ref_expr(c, &format!("material[{id}].{k}"), &hue_ids)
            ),
            None => "None".to_string(),
        };
        writeln!(
            code,
            "    MaterialDef {{ id: {:?}, label: {:?}, blurb: {:?}, fill: {}, ink: {}, edge_color: {} }},",
            id,
            sq(v, "label"),
            sq(v, "blurb"),
            opt("fill"),
            opt("ink"),
            opt("edge_color")
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();

    // ATMOSPHERE — the additive light: shadows, back-glow, tints, plus the
    // state overlays. Emitted straight through `axes_expr`, so an atmosphere
    // and a trait carry the identical shape.
    writeln!(code, "pub static ATMOSPHERES: &[AtmosphereDef] = &[").unwrap();
    for v in obj("atmospheres") {
        let id = sq(v, "id");
        let state = |k: &str| match v.get(k) {
            Some(o) => format!(
                "Some({})",
                axes_expr(o, &format!("atmosphere[{id}].{k}"), &hue_ids)
            ),
            None => "None".to_string(),
        };
        writeln!(
            code,
            "    AtmosphereDef {{ id: {:?}, label: {:?}, blurb: {:?}, base: {}, hover: {}, active: {} }},",
            id,
            sq(v, "label"),
            sq(v, "blurb"),
            axes_expr(v, &format!("atmosphere[{id}]"), &hue_ids),
            state("hover"),
            state("active")
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();

    // ANCESTOR — the lineage: its font (law M5, never a free knob), its
    // natives (the M2 fallback target), and its legality sets.
    writeln!(code, "pub static ANCESTORS: &[AncestorDef] = &[").unwrap();
    for a in ancestors {
        let aid = sq(a, "id");
        let set = |which: &str, facet: &str| -> String {
            let items: Vec<String> = a[which][facet]
                .as_array()
                .map(|v| {
                    v.iter()
                        .map(|x| format!("{:?}", x.as_str().expect("facet ref")))
                        .collect()
                })
                .unwrap_or_default();
            format!("&[{}]", items.join(", "))
        };
        let sets = |which: &str| {
            format!(
                "Sets {{ structures: {}, materials: {}, atmospheres: {} }}",
                set(which, "structure"),
                set(which, "material"),
                set(which, "atmosphere")
            )
        };
        writeln!(
            code,
            "    AncestorDef {{ id: {:?}, label: {:?}, blurb: {:?}, font: {:?}, \
             native: Natives {{ structure: {:?}, material: {:?}, atmosphere: {:?} }}, \
             allowed: {}, forbidden: {} }},",
            aid,
            sq(a, "label"),
            sq(a, "blurb"),
            sq(a, "font"),
            a["native"]["structure"].as_str().unwrap(),
            a["native"]["material"].as_str().unwrap(),
            a["native"]["atmosphere"].as_str().unwrap(),
            sets("allowed"),
            sets("forbidden")
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();

    writeln!(
        code,
        "/// Authored legality rulings — every ancestor rules on every facet value."
    )
    .unwrap();
    writeln!(code, "pub const RULING_COUNT: usize = {rulings};").unwrap();

    std::fs::write(format!("{out_dir}/matrix_gen.rs"), code).expect("write matrix_gen.rs");
}

// ── icons — the 8th source: the vendored Lucide pool (ADR-015) ──────────

/// Flattening tolerance in the 24-unit viewBox: 0.02 units of error is
/// sub-pixel out past 500 px icon renders.
const ICON_TOLERANCE: f32 = 0.02;

/// tokens/icons.json → the ICONS registry. Geometry is tessellated HERE
/// (dpl-svg) so runtime never parses SVG; an entry whose SVG is missing
/// or sits outside the icon subset FAILS THE BUILD — the manifest⇄truth
/// law fonts use. Only registry-referenced icons enter the binary; the
/// vendored pool on disk is the authored source.
fn generate_icons(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/icons.json").expect("read icons.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse icons.json");
    let icons = doc["icons"].as_array().expect("icons.json: icons[]");
    assert!(!icons.is_empty(), "icons.json: empty");

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/icons.json + assets/lucide — DO NOT EDIT."
    )
    .unwrap();
    let mut match_arms = String::new();
    let mut seen = std::collections::BTreeSet::new();
    writeln!(code, "pub static ICONS: &[IconDef] = &[").unwrap();
    for (n, ic) in icons.iter().enumerate() {
        let id = ic["id"].as_str().expect("icon.id");
        let lucide = ic["lucide"].as_str().expect("icon.lucide");
        let label = ic["label"].as_str().expect("icon.label");
        let blurb = ic["blurb"].as_str().expect("icon.blurb");
        assert!(seen.insert(id.to_string()), "icon {id}: duplicate id");
        let path = format!("assets/lucide/icons/{lucide}.svg");
        let svg = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!("icon {id}: {path} is missing — the vendored pool is the source of truth")
        });
        let tess = dpl_svg::tessellate_icon(&svg, ICON_TOLERANCE)
            .unwrap_or_else(|e| panic!("icon {id} ({lucide}): {e}"));
        let (vx, vy, vw, vh) = tess.viewbox;
        let mut verts = String::new();
        for (x, y) in &tess.vertices {
            write!(verts, "({x}f32,{y}f32),").unwrap();
        }
        let mut idxs = String::new();
        for i in &tess.indices {
            write!(idxs, "{i},").unwrap();
        }
        writeln!(
            code,
            "    IconDef {{ id: {id:?}, label: {label:?}, lucide: {lucide:?}, blurb: {blurb:?}, \
             viewbox: ({vx}f32, {vy}f32, {vw}f32, {vh}f32), \
             vertices: &[{verts}], indices: &[{idxs}] }},"
        )
        .unwrap();
        writeln!(match_arms, "        {id:?} => Some(&ICONS[{n}]),").unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(code, "pub const ICON_COUNT: usize = {};", icons.len()).unwrap();
    writeln!(
        code,
        "/// Every registry icon, by semantic id. Unknown → None, never a panic."
    )
    .unwrap();
    writeln!(
        code,
        "pub fn icon_def(id: &str) -> Option<&'static IconDef> {{\n    match id {{"
    )
    .unwrap();
    code.push_str(&match_arms);
    writeln!(code, "        _ => None,\n    }}\n}}").unwrap();

    std::fs::write(Path::new(out_dir).join("icons_gen.rs"), code).expect("write icons_gen.rs");
}

// ── fonts — the vendored pool (the 5th token source) ────────────────────

/// tokens/fonts.json → the FONTS registry. The manifest⇄disk contract is
/// enforced HERE: an entry whose TTF is missing fails the build, so a font
/// can never silently vanish from the runtime.
fn generate_fonts(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/fonts.json").expect("read fonts.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse fonts.json");
    let fonts = doc["fonts"].as_array().expect("fonts.json: fonts");
    assert!(!fonts.is_empty(), "fonts.json: empty pool");

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/fonts.json — DO NOT EDIT."
    )
    .unwrap();
    writeln!(code, "pub static FONTS: &[FontDef] = &[").unwrap();
    for f in fonts {
        let id = f["id"].as_str().expect("font.id");
        let label = f["label"].as_str().expect("font.label");
        let class = f["class"].as_str().expect("font.class");
        let file = f["file"].as_str().expect("font.file");
        let blurb = f["blurb"].as_str().expect("font.blurb");
        assert!(
            matches!(class, "display" | "retro" | "ui" | "mono"),
            "font {id}: unknown class {class:?}"
        );
        assert!(
            std::path::Path::new("assets/fonts").join(file).is_file(),
            "font {id}: assets/fonts/{file} is missing — vendor the TTF"
        );
        writeln!(
            code,
            "    FontDef {{ id: {id:?}, label: {label:?}, class: {class:?}, blurb: {blurb:?}, \
             bytes: include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/fonts/{file}\")) }},"
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(code, "pub const FONT_COUNT: usize = {};", fonts.len()).unwrap();

    std::fs::write(Path::new(out_dir).join("fonts_gen.rs"), code).expect("write fonts_gen.rs");
}

// ── moods — the backdrop's authored multi-hue treatments ────────────────

/// tokens/moods.json → the mood registry. DATA only (hue ids + steps);
/// colors resolve through the generated pool at runtime, so the pool stays
/// the single source of color truth. The build FAILS on a mood with fewer
/// than 3 stops ("a mood is never one color") or an unknown ladder step.
fn generate_moods(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/moods.json").expect("read moods.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse moods.json");
    let moods = doc["moods"].as_array().expect("moods.json: moods[]");

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/moods.json — DO NOT EDIT."
    )
    .unwrap();
    writeln!(
        code,
        "/// An authored backdrop treatment: multi-hue gradient stops (resolved\n\
         /// through the pool), a legibility-scrim alpha, and an accent hue.\n\
         #[derive(Debug, Clone, Copy)]\npub struct Mood {{\n    pub name: &'static str,\n    \
         pub stops: &'static [(&'static str, u32)],\n    pub scrim: u8,\n    \
         pub accent: &'static str,\n}}"
    )
    .unwrap();

    let mut entries = String::new();
    for m in moods {
        let name = m["name"].as_str().expect("mood.name");
        let stops = m["stops"].as_array().expect("mood.stops");
        assert!(
            stops.len() >= 3,
            "mood \"{name}\": a mood is never one color — needs ≥3 stops"
        );
        let mut stop_code = String::new();
        for s in stops {
            let pair = s.as_array().expect("stop [hue, step]");
            let hue = pair[0].as_str().expect("stop hue id");
            let step = pair[1].as_u64().expect("stop step") as u32;
            assert!(
                LADDER_STEPS.contains(&step),
                "mood \"{name}\": step {step} is not in LADDER_STEPS"
            );
            write!(stop_code, "({hue:?}, {step}), ").unwrap();
        }
        let scrim = m["scrim"].as_u64().expect("mood.scrim") as u8;
        let accent = m["accent"].as_str().expect("mood.accent");
        writeln!(
            entries,
            "    Mood {{ name: {name:?}, stops: &[{stop_code}], scrim: {scrim}, accent: {accent:?} }},"
        )
        .unwrap();
    }
    writeln!(code, "pub static MOODS: [Mood; {}] = [", moods.len()).unwrap();
    code.push_str(&entries);
    writeln!(code, "];").unwrap();
    writeln!(code, "pub const MOOD_COUNT: usize = {};", moods.len()).unwrap();

    std::fs::write(Path::new(out_dir).join("moods_gen.rs"), code).expect("write moods_gen.rs");
}

// ── the aurora backdrop gradient ────────────────────────────────────────

/// The old mainpage base layer, ported verbatim: `--gradient-teal-aurora`,
/// a 135° linear gradient interpolated IN OKLCH (interpolating the same
/// stops in sRGB looks duller — the color space is part of the design).
/// Stops from the old generated theme.css: teal-900/500/300/100.
const AURORA_STOPS: [(f64, f64, f64, f64); 4] = [
    (0.00, 0.32, 0.0798, 190.0),
    (0.25, 0.72, 0.1450, 190.0),
    (0.55, 0.84, 0.1160, 190.0),
    (1.00, 0.95, 0.0435, 190.0),
];
/// The old `--color-surface` under the scrim: oklch(0.12 0.01 220).
const SCRIM_LCH: (f64, f64, f64) = (0.12, 0.01, 220.0);
const AURORA_SAMPLES: usize = 33;

fn generate_aurora(out_dir: &str) {
    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs (aurora stops + scrim base) — DO NOT EDIT."
    )
    .unwrap();
    write!(
        code,
        "pub static AURORA: [egui::Color32; {AURORA_SAMPLES}] = ["
    )
    .unwrap();
    for i in 0..AURORA_SAMPLES {
        let t = i as f64 / (AURORA_SAMPLES - 1) as f64;
        // Piecewise-linear between the authored stops, in oklch.
        let (mut l, mut c, mut h) = {
            let last = AURORA_STOPS[AURORA_STOPS.len() - 1];
            (last.1, last.2, last.3)
        };
        for w in AURORA_STOPS.windows(2) {
            let (t0, l0, c0, h0) = w[0];
            let (t1, l1, c1, h1) = w[1];
            if t <= t1 {
                let f = if t1 > t0 {
                    ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                l = l0 + (l1 - l0) * f;
                c = c0 + (c1 - c0) * f;
                h = h0 + (h1 - h0) * f;
                break;
            }
        }
        let (r, g, b) = oklch_to_rgb8(l, c, h);
        write!(code, "egui::Color32::from_rgb({r},{g},{b}),").unwrap();
    }
    writeln!(code, "];").unwrap();
    let (r, g, b) = oklch_to_rgb8(SCRIM_LCH.0, SCRIM_LCH.1, SCRIM_LCH.2);
    writeln!(code, "/// The surface color the old scrim was built from.").unwrap();
    writeln!(code, "pub const SCRIM_RGB: (u8, u8, u8) = ({r}, {g}, {b});").unwrap();
    std::fs::write(Path::new(out_dir).join("aurora_gen.rs"), code).expect("write aurora_gen.rs");
}

// ── motion ──────────────────────────────────────────────────────────────

/// tokens/motion.json → named `Effect` consts + the effect registry. The
/// eval math (bezier, stagger, loop) lives in src; only DATA is generated.
fn generate_motion(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/motion.json").expect("read motion.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse motion.json");
    let eases = doc["eases"].as_object().expect("motion.json: eases");
    let effects = doc["effects"].as_object().expect("motion.json: effects");
    let durations = doc["durations"]
        .as_object()
        .expect("motion.json: durations");

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/motion.json — DO NOT EDIT."
    )
    .unwrap();
    for (name, d) in durations {
        writeln!(
            code,
            "pub const DUR_{}: f32 = {}f32;",
            ident(name).to_uppercase(),
            d.as_f64().expect("duration")
        )
        .unwrap();
    }

    let ease_of = |name: &str| -> String {
        let p = eases
            .get(name)
            .and_then(|e| e.as_array())
            .unwrap_or_else(|| panic!("effect references unknown ease \"{name}\""));
        let v: Vec<f64> = p.iter().map(|x| x.as_f64().expect("ease point")).collect();
        format!("Ease({}f32, {}f32, {}f32, {}f32)", v[0], v[1], v[2], v[3])
    };
    for name in eases.keys() {
        writeln!(
            code,
            "pub const EASE_{}: Ease = {};",
            ident(name).to_uppercase(),
            ease_of(name)
        )
        .unwrap();
    }

    // The authored channel vocabulary — EXACTLY `Motion`'s fields. A typo in
    // a track fails the build naming the effect, the same manifest⇄truth law
    // icons and fonts already carry.
    const CHANS: [(&str, &str); 6] = [
        ("alpha", "Chan::Alpha"),
        ("offset_x", "Chan::OffsetX"),
        ("offset_y", "Chan::OffsetY"),
        ("scale", "Chan::Scale"),
        ("rotation", "Chan::Rotation"),
        ("clip", "Chan::Clip"),
    ];

    let mut match_arms = String::new();
    let mut tracks = String::new();
    for (name, fx) in effects {
        let dur = fx["dur"].as_f64().expect("effect.dur");
        let delay = fx.get("delay").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let stagger = fx.get("stagger").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let looped = fx.get("loop").and_then(|v| v.as_bool()).unwrap_or(false);
        let ease = ease_of(fx["ease"].as_str().expect("effect.ease"));
        let konst = ident(name).to_uppercase();

        // ── the optional keyframe track (docs/plans/keyframe-engine.md §3) ──
        let track_ref = match fx.get("track").and_then(|t| t.as_array()) {
            None => "None".to_string(),
            Some(stops) => {
                assert!(
                    stops.len() >= 2,
                    "effect {name:?}: a track needs at least 2 stops"
                );
                let mut emitted = String::new();
                let mut prev_off = f32::NEG_INFINITY;
                // Which channels the whole track touches, and whether the
                // first / last stop pin them (law 4 — no implicit endpoints).
                let mut seen: Vec<&str> = Vec::new();
                let (mut first_ch, mut last_ch): (Vec<&str>, Vec<&str>) = (vec![], vec![]);

                for (i, st) in stops.iter().enumerate() {
                    let off = st["offset"].as_f64().unwrap_or_else(|| {
                        panic!("effect {name:?}: stop {i} has no numeric offset")
                    }) as f32;
                    assert!(
                        (0.0..=1.0).contains(&off),
                        "effect {name:?}: stop {i} offset {off} is outside 0..=1"
                    );
                    assert!(
                        off > prev_off,
                        "effect {name:?}: stop {i} offset {off} is not ascending"
                    );
                    if i == 0 {
                        assert!(off == 0.0, "effect {name:?}: first stop must be offset 0.0");
                    }
                    if i == stops.len() - 1 {
                        assert!(off == 1.0, "effect {name:?}: last stop must be offset 1.0");
                    }
                    prev_off = off;

                    // A stop's ease is optional; it defaults to the effect's.
                    let st_ease = match st.get("ease").and_then(|e| e.as_str()) {
                        Some(e) => ease_of(e),
                        None => ease.clone(),
                    };

                    let mut pairs = String::new();
                    for (key, variant) in CHANS {
                        if let Some(v) = st.get(key).and_then(|v| v.as_f64()) {
                            write!(pairs, "({variant}, {v}f32),").unwrap();
                            if !seen.contains(&key) {
                                seen.push(key);
                            }
                            if i == 0 {
                                first_ch.push(key);
                            }
                            if i == stops.len() - 1 {
                                last_ch.push(key);
                            }
                        }
                    }
                    // Anything that is not `offset`, `ease` or a real channel
                    // is a typo — refuse it rather than ignoring it silently.
                    for k in st.as_object().expect("track stop is an object").keys() {
                        assert!(
                            k == "offset" || k == "ease" || CHANS.iter().any(|(c, _)| c == k),
                            "effect {name:?}: stop {i} has unknown channel {k:?}"
                        );
                    }
                    writeln!(
                        emitted,
                        "    Stop {{ offset: {off}f32, ease: {st_ease}, ch: &[{pairs}] }},"
                    )
                    .unwrap();
                }
                for c in &seen {
                    assert!(
                        first_ch.contains(c) && last_ch.contains(c),
                        "effect {name:?}: channel {c:?} must be declared on BOTH the \
                         first and last stop so its endpoints are never implicit"
                    );
                }
                // `const`, not `static`: the Effect consts reference these,
                // and a const may not refer to a static (E0013). A `&CONST`
                // promotes to 'static, which is exactly what we want.
                writeln!(
                    tracks,
                    "const TRACK_{konst}: Track = Track {{ stops: &[\n{emitted}] }};"
                )
                .unwrap();
                format!("Some(&TRACK_{konst})")
            }
        };

        writeln!(
            code,
            "pub const {konst}: Effect = Effect {{ dur: {dur}f32, delay: {delay}f32, \
             stagger: {stagger}f32, ease: {ease}, looped: {looped}, track: {track_ref} }};"
        )
        .unwrap();
        writeln!(match_arms, "        {name:?} => Some(&{konst}),").unwrap();
    }
    // Tracks are statics the Effect consts point at — emitted first so the
    // consts can reference them.
    code.push_str(&tracks);
    writeln!(code, "pub const EFFECT_COUNT: usize = {};", effects.len()).unwrap();
    writeln!(
        code,
        "/// Every named effect. Unknown → None, never a panic."
    )
    .unwrap();
    writeln!(
        code,
        "pub fn effect(name: &str) -> Option<&'static Effect> {{\n    match name {{"
    )
    .unwrap();
    code.push_str(&match_arms);
    writeln!(code, "        _ => None,\n    }}\n}}").unwrap();

    std::fs::write(Path::new(out_dir).join("motion_gen.rs"), code).expect("write motion_gen.rs");
}

// ── color ladders ───────────────────────────────────────────────────────

fn generate_ramps(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/hues.json").expect("read hues.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse hues.json");
    let groups = doc["groups"].as_object().expect("hues.json: groups");

    let mut code = String::new();
    let mut group_lists: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut match_arms = String::new();
    let mut total = 0usize;

    writeln!(
        code,
        "// GENERATED by build.rs from tokens/hues.json — DO NOT EDIT."
    )
    .unwrap();
    writeln!(
        code,
        "pub const LADDER_STEPS: [u32; 13] = {LADDER_STEPS:?};"
    )
    .unwrap();
    writeln!(
        code,
        "#[derive(Debug, Clone, Copy)]\npub struct Ramp {{ pub steps: [egui::Color32; 13] }}"
    )
    .unwrap();

    for (group_name, group) in groups {
        let hues = group["hues"].as_array().expect("group.hues");
        for hue in hues {
            let id = hue["id"].as_str().expect("hue.id").to_string();
            let anchor_h = hue["hue"].as_f64().unwrap_or(0.0);
            let anchor_c = hue["chroma"].as_f64().unwrap_or(0.0);
            let curve = hue["curve"].as_str().unwrap_or("brand");
            let chroma_curve = hue["chromaCurve"].as_str().unwrap_or("linear-to-500");
            let l_map = lightness(curve)
                .unwrap_or_else(|| panic!("hue \"{id}\": unknown curve \"{curve}\""));
            let c_map = chroma_mult(chroma_curve)
                .unwrap_or_else(|| panic!("hue \"{id}\": unknown chromaCurve \"{chroma_curve}\""));
            let overrides = hue.get("overrides").and_then(|o| o.as_object());

            let mut steps_code = String::new();
            for (i, step) in LADDER_STEPS.iter().enumerate() {
                let ov = overrides.and_then(|o| o.get(&step.to_string()));
                let l = ov
                    .and_then(|o| o.get("l"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(l_map[i]);
                let c = ov
                    .and_then(|o| o.get("c"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or((anchor_c * c_map[i]).clamp(0.0, 1.0));
                let h = ov
                    .and_then(|o| o.get("h"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(anchor_h);
                // The never-pure-black floor — curves may not go below it;
                // an authored override opts out by existing.
                if l < LIGHTNESS_FLOOR && ov.is_none() {
                    panic!(
                        "hue \"{id}\" step {step}: L={l:.3} < {LIGHTNESS_FLOOR} — never pure black. \
                         Raise the curve or author an override."
                    );
                }
                let (r, g, b) = oklch_to_rgb8(l, c, h);
                write!(steps_code, "egui::Color32::from_rgb({r},{g},{b}),").unwrap();
            }

            let konst = format!("R_{}", ident(&id).to_uppercase());
            writeln!(
                code,
                "pub static {konst}: Ramp = Ramp {{ steps: [{steps_code}] }};"
            )
            .unwrap();
            writeln!(match_arms, "        {:?} => Some(&{konst}),", id).unwrap();
            group_lists.entry(group_name.clone()).or_default().push(id);
            total += 1;
        }
    }

    writeln!(
        code,
        "/// Every hue in the pool, by id. Unknown → None, never a panic."
    )
    .unwrap();
    writeln!(
        code,
        "pub fn ramp(id: &str) -> Option<&'static Ramp> {{\n    match id {{"
    )
    .unwrap();
    code.push_str(&match_arms);
    writeln!(code, "        _ => None,\n    }}\n}}").unwrap();

    writeln!(code, "pub const HUE_COUNT: usize = {total};").unwrap();
    writeln!(
        code,
        "/// (group, hue ids) — registry order, for the gallery gate."
    )
    .unwrap();
    writeln!(
        code,
        "pub static HUE_GROUPS: [(&str, &[&str]); {}] = [",
        group_lists.len()
    )
    .unwrap();
    for (g, ids) in &group_lists {
        let list = ids
            .iter()
            .map(|i| format!("{i:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(code, "    ({g:?}, &[{list}]),").unwrap();
    }
    writeln!(code, "];").unwrap();

    std::fs::write(Path::new(out_dir).join("ramps_gen.rs"), code).expect("write ramps_gen.rs");
}

// ── fluid clamps ────────────────────────────────────────────────────────

/// The XY windows — ported verbatim from fluid-axis.mjs.
const FLUID_X: (f64, f64) = (24.0, 120.0); // rem
const FLUID_Y: (f64, f64) = (40.0, 100.0); // rem
const XY_Y_SHARE: f64 = 0.5;

fn generate_fluid(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/fluid.json").expect("read fluid.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse fluid.json");
    let groups = doc["groups"].as_object().expect("fluid.json: groups");

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/fluid.json — DO NOT EDIT."
    )
    .unwrap();
    writeln!(code, "impl Fluid {{").unwrap();

    let mut names: Vec<String> = Vec::new();
    for group in groups.values() {
        for t in group["tokens"].as_array().expect("group.tokens") {
            let name = t["name"].as_str().expect("token.name");
            let min = t["min"].as_f64().expect("token.min");
            let max = t["max"].as_f64().expect("token.max");
            // fluidRange, axis "xy": half the travel rides width, half height.
            let travel = max - min;
            let slope = (travel * (1.0 - XY_Y_SHARE)) / (FLUID_X.1 - FLUID_X.0);
            let slope_y = (travel * XY_Y_SHARE) / (FLUID_Y.1 - FLUID_Y.0);
            let intercept = min - slope * FLUID_X.0 - slope_y * FLUID_Y.0;
            // Emit shortest-round-trip f32 literals — raw f64 digits trip
            // the gate (clippy::excessive_precision) for zero gain at f32.
            let f = |v: f64| format!("{}", v as f32);
            writeln!(
                code,
                "    /// `{name}`: clamp({min}rem … {max}rem), XY-interpolated.\n    \
                 pub fn {}(&self) -> f32 {{ self.eval({}f32, {}f32, {}f32, {}f32, {}f32) }}",
                ident(name),
                f(min),
                f(max),
                f(slope),
                f(slope_y),
                f(intercept)
            )
            .unwrap();
            names.push(name.to_string());
        }
    }
    writeln!(code, "}}").unwrap();
    writeln!(
        code,
        "pub const FLUID_TOKEN_COUNT: usize = {};",
        names.len()
    )
    .unwrap();
    // The names, enumerable — dpl-doc validates size refs against this.
    writeln!(
        code,
        "pub static FLUID_TOKEN_NAMES: &[&str] = &[{}];",
        names
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
    .unwrap();

    std::fs::write(Path::new(out_dir).join("fluid_gen.rs"), code).expect("write fluid_gen.rs");
}

// ── presets — the 6th source: named bundles of refs ─────────────────────

/// tokens/presets.json → THEME_PRESETS / MOTION_PRESETS. A preset is DATA
/// over the other registries; every ref is cross-checked HERE against the
/// authored sources, so an unknown font/mood/effect/ambience fails the
/// build — the same manifest⇄truth law fonts use.
fn generate_presets(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/presets.json").expect("read presets.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse presets.json");

    let fonts_doc: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("tokens/fonts.json").expect("read fonts.json"),
    )
    .expect("parse fonts.json");
    let font_ids: Vec<String> = fonts_doc["fonts"]
        .as_array()
        .expect("fonts.json: fonts")
        .iter()
        .map(|f| f["id"].as_str().expect("font.id").to_string())
        .collect();

    let moods_doc: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("tokens/moods.json").expect("read moods.json"),
    )
    .expect("parse moods.json");
    let mood_names: Vec<String> = moods_doc["moods"]
        .as_array()
        .expect("moods.json: moods[]")
        .iter()
        .map(|m| m["name"].as_str().expect("mood.name").to_string())
        .collect();

    let motion_doc: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("tokens/motion.json").expect("read motion.json"),
    )
    .expect("parse motion.json");
    let effect_ids: Vec<String> = motion_doc["effects"]
        .as_object()
        .expect("motion.json: effects")
        .keys()
        .cloned()
        .collect();
    const AMBIENCES: [&str; 3] = ["pulse", "drift", "rings"];
    // Entity-scoped presets (ruling 2026-07-31; LS2b): what each entity's
    // preset may expose. `font` on the card is the operator's 2026-10-21
    // ruling — a card look carries a default font for its lines.
    const KNOBS: [(&str, &[&str]); 4] = [
        (
            "card",
            &["background", "ambience", "motion", "colour", "font"],
        ),
        ("text", &["font", "size", "ink", "colour", "motion"]),
        ("picture", &["fit", "colour", "motion"]),
        ("quote", &["font", "size", "ink", "colour"]),
    ];
    let hues_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tokens/hues.json").expect("read hues.json"))
            .expect("parse hues.json");
    let mut hue_ids: Vec<String> = Vec::new();
    collect_ids(&hues_doc, &mut hue_ids);

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/presets.json — DO NOT EDIT."
    )
    .unwrap();

    let themes = doc["theme"].as_array().expect("presets.json: theme[]");
    writeln!(code, "pub static THEME_PRESETS: &[ThemePreset] = &[").unwrap();
    for p in themes {
        let id = p["id"].as_str().expect("theme.id");
        let label = p["label"].as_str().expect("theme.label");
        let blurb = p["blurb"].as_str().expect("theme.blurb");
        let font = p["font"].as_str().expect("theme.font");
        let moods = p["moods"].as_array().expect("theme.moods");
        assert_eq!(moods.len(), 2, "theme {id}: exactly 2 moods");
        let (a, b) = (
            moods[0].as_str().expect("mood ref"),
            moods[1].as_str().expect("mood ref"),
        );
        let motion = p["motion"].as_str().expect("theme.motion");
        let ambience = p["ambience"].as_str();
        let entity = p["for"].as_str().expect("theme.for");
        let tone = p["tone"].as_str().expect("theme.tone");
        let exposes: Vec<&str> = p["exposes"]
            .as_array()
            .expect("theme.exposes")
            .iter()
            .map(|k| k.as_str().expect("knob"))
            .collect();
        let legal = KNOBS
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, knobs)| *knobs)
            .unwrap_or_else(|| panic!("theme {id}: unknown entity {entity:?}"));
        assert_eq!(
            entity, "card",
            "theme {id}: a one-tap look is a card preset"
        );
        assert!(
            (4..=7).contains(&exposes.len()),
            "theme {id}: exposes {} knobs; a preset exposes 4-7",
            exposes.len()
        );
        assert!(
            exposes.contains(&"colour"),
            "theme {id}: colour is always one of the knobs"
        );
        for knob in &exposes {
            assert!(
                legal.contains(knob),
                "theme {id}: a {entity} has no knob {knob:?}"
            );
        }
        assert!(
            hue_ids.iter().any(|h| h == tone),
            "theme {id}: unknown tone {tone:?}"
        );
        assert!(
            font_ids.iter().any(|f| f == font),
            "theme {id}: unknown font {font:?}"
        );
        for m in [a, b] {
            assert!(
                mood_names.iter().any(|n| n == m),
                "theme {id}: unknown mood {m:?}"
            );
        }
        assert!(
            effect_ids.iter().any(|e| e == motion),
            "theme {id}: unknown motion {motion:?}"
        );
        if let Some(amb) = ambience {
            assert!(
                AMBIENCES.contains(&amb),
                "theme {id}: unknown ambience {amb:?}"
            );
        }
        let amb_code = match ambience {
            Some(amb) => format!("Some({amb:?})"),
            None => "None".to_string(),
        };
        writeln!(
            code,
            "    ThemePreset {{ id: {id:?}, label: {label:?}, blurb: {blurb:?}, font: {font:?}, \
             moods: ({a:?}, {b:?}), motion: {motion:?}, ambience: {amb_code}, \
             entity: {entity:?}, exposes: &{exposes:?}, tone: {tone:?} }},"
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(
        code,
        "pub const THEME_PRESET_COUNT: usize = {};",
        themes.len()
    )
    .unwrap();
    // The knob table itself, so the runtime check of the operator's own
    // looks (dpl-doc, LK2) reads the same law as this build-time one.
    writeln!(
        code,
        "/// What each entity's preset may expose — the build-time law, \
         emitted for the runtime check of the operator's own looks."
    )
    .unwrap();
    writeln!(code, "pub static PRESET_KNOBS: &[(&str, &[&str])] = &[").unwrap();
    for (entity, knobs) in KNOBS {
        writeln!(code, "    ({entity:?}, &{knobs:?}),").unwrap();
    }
    writeln!(code, "];").unwrap();

    let motions = doc["motion"].as_array().expect("presets.json: motion[]");
    writeln!(code, "pub static MOTION_PRESETS: &[MotionPreset] = &[").unwrap();
    for p in motions {
        let id = p["id"].as_str().expect("motion.id");
        let label = p["label"].as_str().expect("motion.label");
        let blurb = p["blurb"].as_str().expect("motion.blurb");
        let enter = p["enter"].as_str().expect("motion.enter");
        assert!(
            effect_ids.iter().any(|e| e == enter),
            "motion preset {id}: unknown effect {enter:?}"
        );
        writeln!(
            code,
            "    MotionPreset {{ id: {id:?}, label: {label:?}, blurb: {blurb:?}, enter: {enter:?} }},"
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(
        code,
        "pub const MOTION_PRESET_COUNT: usize = {};",
        motions.len()
    )
    .unwrap();

    // BLEED presets — a hover wash over a PAIR of hues drawn from a pool.
    // Its own kind because `MotionPreset` carries one `enter` effect and no
    // colour, and widening that for one caller would change every existing
    // motion preset. The pool is AUTHORED: an unknown hue fails the build
    // here, which is what stops a widget carrying a list of hexes.
    let hues_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tokens/hues.json").expect("read hues.json"))
            .expect("parse hues.json");
    let mut hue_ids: Vec<String> = Vec::new();
    for group in hues_doc["groups"]
        .as_object()
        .expect("hues.json: groups")
        .values()
    {
        if let Some(hues) = group.get("hues").and_then(|h| h.as_array()) {
            hue_ids.extend(
                hues.iter()
                    .filter_map(|h| h["id"].as_str().map(str::to_owned)),
            );
        }
    }

    let bleeds = doc["bleed"].as_array().expect("presets.json: bleed[]");
    writeln!(code, "pub static BLEED_PRESETS: &[BleedPreset] = &[").unwrap();
    for p in bleeds {
        let id = p["id"].as_str().expect("bleed.id");
        let label = p["label"].as_str().expect("bleed.label");
        let blurb = p["blurb"].as_str().expect("bleed.blurb");
        let effect = p["effect"].as_str().expect("bleed.effect");
        assert!(
            effect_ids.iter().any(|e| e == effect),
            "bleed preset {id}: unknown effect {effect:?}"
        );
        let hues = p["hues"].as_array().expect("bleed.hues");
        assert!(
            hues.len() >= 2,
            "bleed preset {id}: a two-colour bleed needs a pool of at least 2"
        );
        let mut hue_code = String::new();
        for h in hues {
            let h = h.as_str().expect("bleed.hue ref");
            assert!(
                hue_ids.iter().any(|k| k == h),
                "bleed preset {id}: unknown hue {h:?}"
            );
            write!(hue_code, "{h:?},").unwrap();
        }
        writeln!(
            code,
            "    BleedPreset {{ id: {id:?}, label: {label:?}, blurb: {blurb:?}, \
             effect: {effect:?}, hues: &[{hue_code}] }},"
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(
        code,
        "pub const BLEED_PRESET_COUNT: usize = {};",
        bleeds.len()
    )
    .unwrap();

    // ── the ANCESTRAL presets — the 4th kind, and the only one that is a
    //    MATRIX COORDINATE rather than a bundle of refs. Every one is checked
    //    against its lineage's own rulings, so an illegal look fails the build.
    let matrix_for_presets: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("tokens/matrix.json").expect("read matrix.json"),
    )
    .expect("parse matrix.json");
    let hues_for_presets: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tokens/hues.json").expect("read hues.json"))
            .expect("parse hues.json");

    writeln!(
        code,
        "pub static ANCESTRAL_PRESETS: &[AncestralPreset] = &["
    )
    .unwrap();
    let mut preset_ids: Vec<String> = Vec::new();
    for p in doc["ancestral"]
        .as_array()
        .expect("presets.json: ancestral[]")
    {
        let id = p["id"].as_str().expect("ancestral.id").to_owned();
        assert!(
            !preset_ids.contains(&id),
            "presets.json: duplicate ancestral preset {id:?}"
        );
        let c = &p["coordinate"];
        let get = |k: &str| -> &str {
            c[k].as_str()
                .unwrap_or_else(|| panic!("presets.json: ancestral[{id}].coordinate needs {k:?}"))
        };
        let (anc, st, mt, at) = (
            get("ancestor"),
            get("structure"),
            get("material"),
            get("atmosphere"),
        );
        assert_coordinate_legal(
            &matrix_for_presets,
            &format!("presets.json: ancestral[{id}]"),
            anc,
            st,
            mt,
            at,
        );
        let tone = match p.get("tone").and_then(|t| t.as_str()) {
            Some(h) => {
                assert!(
                    hue_exists(&hues_for_presets, h),
                    "presets.json: ancestral[{id}] names unknown hue {h:?}"
                );
                format!("Some({h:?})")
            }
            None => "None".to_string(),
        };
        writeln!(
            code,
            "    AncestralPreset {{ id: {id:?}, label: {:?}, kicker: {:?}, blurb: {:?}, \
             coord: matrix::Coordinate {{ ancestor: {anc:?}, structure: {st:?}, \
             material: {mt:?}, atmosphere: {at:?} }}, tone: {tone} }},",
            p["label"].as_str().expect("ancestral.label"),
            p["kicker"].as_str().unwrap_or_default(),
            p["blurb"].as_str().unwrap_or_default(),
        )
        .unwrap();
        preset_ids.push(id);
    }
    writeln!(code, "];").unwrap();

    std::fs::write(Path::new(out_dir).join("presets_gen.rs"), code).expect("write presets_gen.rs");
}

// ── shared emitters: traits AND the matrix resolve through the SAME
//    primitives, so a facet payload and a trait payload can never drift.

/// Every hue id in the generated pool — the truth a color ref validates against.
fn pool_hue_ids() -> Vec<String> {
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tokens/hues.json").expect("read hues.json"))
            .expect("parse hues.json");
    let mut ids = Vec::new();
    for group in doc["groups"]
        .as_object()
        .expect("hues.json: groups")
        .values()
    {
        for hue in group["hues"].as_array().expect("group.hues") {
            ids.push(hue["id"].as_str().expect("hue.id").to_string());
        }
    }
    ids
}

fn f32_lit_expr(v: f64) -> String {
    format!("{}f32", v as f32)
}

const SURF_SLOTS: [&str; 5] = ["WELL", "BASE", "RAISED", "OVERLAY", "HAIRLINE"];
const INK_SLOTS: [&str; 5] = ["STRONG", "BASE", "MUTED", "SUBTLE", "ANNOTATE"];

fn color_ref_expr(v: &serde_json::Value, ctx: &str, hue_ids: &[String]) -> String {
    let alpha = v.get("alpha").and_then(|a| a.as_u64()).unwrap_or(255);
    assert!(alpha <= 255, "{ctx}: alpha {alpha} > 255");
    let base = if let Some(step) = v.get("tone").and_then(|s| s.as_u64()) {
        assert!(
            LADDER_STEPS.contains(&(step as u32)),
            "{ctx}: tone step {step} not in LADDER_STEPS"
        );
        format!("ColorBase::ToneStep({step})")
    } else if let Some(hue) = v.get("hue").and_then(|h| h.as_str()) {
        let step = v
            .get("step")
            .and_then(|s| s.as_u64())
            .unwrap_or_else(|| panic!("{ctx}: hue ref needs a step"));
        assert!(
            hue_ids.iter().any(|h| h == hue),
            "{ctx}: unknown hue {hue:?}"
        );
        assert!(
            LADDER_STEPS.contains(&(step as u32)),
            "{ctx}: hue step {step} not in LADDER_STEPS"
        );
        format!("ColorBase::Hue({hue:?}, {step})")
    } else if let Some(slot) = v.get("surface").and_then(|s| s.as_str()) {
        assert!(
            SURF_SLOTS.contains(&slot),
            "{ctx}: unknown surface slot {slot:?}"
        );
        format!("ColorBase::Fixed(surface::{slot})")
    } else if let Some(slot) = v.get("ink").and_then(|s| s.as_str()) {
        assert!(
            INK_SLOTS.contains(&slot),
            "{ctx}: unknown ink slot {slot:?}"
        );
        format!("ColorBase::Fixed(ink::{slot})")
    } else if v.get("black").and_then(|b| b.as_bool()).unwrap_or(false) {
        "ColorBase::Fixed(egui::Color32::BLACK)".to_string()
    } else {
        panic!("{ctx}: color ref names no base ({v})");
    };
    format!("ColorRef {{ base: {base}, alpha: {alpha} }}")
}

fn axes_expr(o: &serde_json::Value, ctx: &str, hue_ids: &[String]) -> String {
    let opt_color = |key: &str| match o.get(key) {
        Some(v) => format!(
            "Some({})",
            color_ref_expr(v, &format!("{ctx}.{key}"), hue_ids)
        ),
        None => "None".to_string(),
    };
    let edge = match o.get("edge") {
        Some(e) => format!(
            "Some(EdgeSpec {{ width: {}, color: {} }})",
            f32_lit_expr(e["width"].as_f64().expect("edge.width")),
            color_ref_expr(&e["color"], &format!("{ctx}.edge"), hue_ids)
        ),
        None => "None".to_string(),
    };
    let radius = match o.get("radius").and_then(|r| r.as_u64()) {
        Some(r) => format!("Some({r})"),
        None => "None".to_string(),
    };
    let font = match o.get("font").and_then(|f| f.as_str()) {
        Some(f) => format!("Some({f:?})"),
        None => "None".to_string(),
    };
    let shadows = match o.get("shadows").and_then(|s| s.as_array()) {
        Some(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|s| {
                    let off = s["offset"].as_array().expect("shadow.offset");
                    format!(
                        "ShadowSpec {{ offset: [{}, {}], blur: {}, spread: {}, color: {} }}",
                        off[0].as_i64().expect("offset.0"),
                        off[1].as_i64().expect("offset.1"),
                        s["blur"].as_u64().expect("shadow.blur"),
                        s["spread"].as_u64().expect("shadow.spread"),
                        color_ref_expr(&s["color"], &format!("{ctx}.shadow"), hue_ids)
                    )
                })
                .collect();
            format!("Some(&[{}])", items.join(", "))
        }
        None => "None".to_string(),
    };
    let back_glow = match o.get("back_glow").and_then(|s| s.as_array()) {
        Some(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|g| {
                    format!(
                        "GlowSpec {{ color: {}, spread: {} }}",
                        color_ref_expr(&g["color"], &format!("{ctx}.back_glow"), hue_ids),
                        f32_lit_expr(g["spread"].as_f64().expect("glow.spread"))
                    )
                })
                .collect();
            format!("Some(&[{}])", items.join(", "))
        }
        None => "None".to_string(),
    };
    let tints = match o.get("tints").and_then(|s| s.as_array()) {
        Some(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|t| {
                    format!(
                        "TintSpec {{ color: {}, dir: {} }}",
                        color_ref_expr(&t["color"], &format!("{ctx}.tint"), hue_ids),
                        f32_lit_expr(t["dir"].as_f64().expect("tint.dir"))
                    )
                })
                .collect();
            format!("Some(&[{}])", items.join(", "))
        }
        None => "None".to_string(),
    };
    let anim = match o.get("anim").and_then(|s| s.as_array()) {
        Some(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|a| format!("{:?}", a.as_str().expect("anim id")))
                .collect();
            format!("Some(&[{}])", items.join(", "))
        }
        None => "None".to_string(),
    };
    format!(
        "Axes {{ fill: {}, edge: {edge}, radius: {radius}, ink: {}, font: {font}, \
             shadows: {shadows}, back_glow: {back_glow}, tints: {tints}, anim: {anim} }}",
        opt_color("fill"),
        opt_color("ink")
    )
}

// ── traits — the stackable partial themes (docs/design/doctrine.md) ───────

/// tokens/traits.json → the TRAITS registry. A trait is DATA: per-axis
/// specs whose color refs are LADDER-RELATIVE ({tone:STEP}) or fixed pool
/// refs ({hue:ID,step:STEP}) or slots — resolved against a runtime Tone by
/// the compositor in src. Hue/step refs are cross-validated HERE against the
/// generated pool + LADDER_STEPS (unknown → build fails), the same
/// manifest⇄truth law fonts/presets use. NOTE: `anim` refs are carried
/// verbatim (deferred vocabulary) and not yet cross-checked against motion.
fn generate_traits(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/traits.json").expect("read traits.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse traits.json");
    let traits = doc["traits"].as_array().expect("traits.json: traits[]");
    assert!(!traits.is_empty(), "traits.json: empty");

    // Valid hue ids — collected from the same source generate_ramps reads.
    let hues_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tokens/hues.json").expect("read hues.json"))
            .expect("parse hues.json");
    let mut hue_ids: Vec<String> = Vec::new();
    for group in hues_doc["groups"]
        .as_object()
        .expect("hues.json: groups")
        .values()
    {
        for hue in group["hues"].as_array().expect("group.hues") {
            hue_ids.push(hue["id"].as_str().expect("hue.id").to_string());
        }
    }

    let axes = |o: &serde_json::Value, ctx: &str| axes_expr(o, ctx, &hue_ids);

    let mut code = String::new();
    writeln!(
        code,
        "// GENERATED by build.rs from tokens/traits.json — DO NOT EDIT."
    )
    .unwrap();
    writeln!(code, "pub static TRAITS: &[TraitDef] = &[").unwrap();
    for t in traits {
        let id = t["id"].as_str().expect("trait.id");
        let role = t["role"].as_str().expect("trait.role");
        assert!(
            matches!(role, "base" | "stack"),
            "trait {id}: role must be base|stack, got {role:?}"
        );
        let label = t["label"].as_str().expect("trait.label");
        let blurb = t["blurb"].as_str().expect("trait.blurb");
        let base = axes(t, &format!("trait[{id}]"));
        let hover = match t.get("hover") {
            Some(h) => format!("Some({})", axes(h, &format!("trait[{id}].hover"))),
            None => "None".to_string(),
        };
        let active = match t.get("active") {
            Some(a) => format!("Some({})", axes(a, &format!("trait[{id}].active"))),
            None => "None".to_string(),
        };
        writeln!(
            code,
            "    TraitDef {{ id: {id:?}, role: {role:?}, label: {label:?}, blurb: {blurb:?}, \
             base: {base}, hover: {hover}, active: {active} }},"
        )
        .unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(code, "pub const TRAIT_COUNT: usize = {};", traits.len()).unwrap();

    std::fs::write(Path::new(out_dir).join("traits_gen.rs"), code).expect("write traits_gen.rs");
}

/// Every `"id"` string anywhere in a token document (hues are grouped).
fn collect_ids(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(id) = map.get("id").and_then(|v| v.as_str()) {
                out.push(id.to_string());
            }
            for v in map.values() {
                collect_ids(v, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect_ids(v, out)),
        _ => {}
    }
}
