//! RTFX-aware ingest enrichment (L3) — the videohive naming grammar,
//! census-derived (docs/plans/l3-alpha-overlay.md §2: 711 prerendered
//! files + 703 SFX enumerated 2026-07-27), parsed into index tags so a
//! 440-element pack browses by category and an agent can ask for "a fire
//! flash" by tag.
//!
//! Tolerance is the law here: the pack carries a `MiX` casing typo, double
//! spaces, lowercase drift (`up`, `right`, `loop`), spelling typos
//! (`Explositon`), and duplicate variant numbers. Token matching is
//! case-insensitive; anything unrecognized stays in the NAME (searchable)
//! and never becomes a wrong tag. A non-RTFX-shaped name returns None and
//! the ingest proceeds untagged — never a guess.

use std::path::{Path, PathBuf};

/// The known modifier tokens → their tags. Case-insensitive.
fn modifier_tag(token: &str) -> Option<&'static str> {
    Some(match token.to_ascii_lowercase().as_str() {
        "loop" => "loop",
        "mix" => "mix",
        "stch" => "stretch",
        "noct" => "no-tint",
        "norsz" => "no-resize",
        "alpha" => "alpha",
        "ainv" | "inv" => "inverted",
        "wide" => "wide",
        "8bit" => "8bit",
        "inside" => "inside",
        "outside" => "outside",
        _ => return None,
    })
}

/// Direction tokens → `dir:*` tags. Case-insensitive ("Up", "up", "IN").
fn direction_tag(token: &str) -> Option<&'static str> {
    Some(match token.to_ascii_lowercase().as_str() {
        "up" => "dir:up",
        "down" => "dir:down",
        "left" => "dir:left",
        "right" => "dir:right",
        "radial" => "dir:radial",
        "out" => "dir:out",
        "in" => "dir:in",
        "horizontal" => "dir:horizontal",
        "vertical" => "dir:vertical",
        "diagonal" => "dir:diagonal",
        "cw" => "dir:cw",
        "ccw" => "dir:ccw",
        _ => return None,
    })
}

/// The bucket (the file's subfolder context inside the pack) → its
/// section tags. Matched loosely on path segments, case-insensitive.
fn bucket_tags(bucket: &str) -> Vec<String> {
    let lower = bucket.to_ascii_lowercase();
    let mut out = Vec::new();
    for (needle, tag) in [
        ("fx elements", "fx"),
        ("transitions", "transition"),
        ("matte", "matte"),
        ("backgrounds", "bg"),
        ("foregrounds", "fg"),
        ("sound fx", "sfx"),
    ] {
        if lower.contains(needle) {
            out.push(tag.to_string());
        }
    }
    out
}

/// Parse one RTFX-shaped file STEM (no extension) + its bucket path into
/// index tags. None = not RTFX-shaped (ingest proceeds untagged, honestly).
pub fn rtfx_tags(stem: &str, bucket: &str) -> Option<Vec<String>> {
    // Normalize the double-space drift first.
    let joined = stem.split_whitespace().collect::<Vec<_>>().join(" ");
    let rest = ["Elements - ", "Transition - ", "SFX - "]
        .iter()
        .find_map(|p| joined.strip_prefix(p))?;
    let mut tags: Vec<String> = vec!["rtfx".into()];
    if joined.starts_with("SFX - ") {
        tags.push("sfx".into());
    }
    // The fixed transition version marker is noise, not a tag.
    let rest = rest.strip_prefix("V02 ").unwrap_or(rest);
    let (rest, simple) = match rest.strip_prefix("Simple ") {
        Some(r) => (r, true),
        None => (rest, false),
    };
    let rest = rest.strip_prefix("Minimal ").unwrap_or(rest);

    // CATEGORY = the words before the variant number.
    let words: Vec<&str> = rest.split(' ').collect();
    let num_at = words
        .iter()
        .position(|w| w.chars().all(|c| c.is_ascii_digit()) && !w.is_empty())?;
    if num_at == 0 {
        return None;
    }
    let category = words[..num_at].join("-").to_ascii_lowercase();
    tags.push(category);
    if simple {
        tags.push("simple".into());
    }

    // Everything after the number: known modifiers + directions become
    // tags; free descriptor words (Ray, Slash, Hit…) stay name-only.
    for w in &words[num_at + 1..] {
        if let Some(t) = modifier_tag(w) {
            if !tags.iter().any(|x| x == t) {
                tags.push(t.into());
            }
        } else if let Some(t) = direction_tag(w) {
            if !tags.iter().any(|x| x == t) {
                tags.push(t.into());
            }
        }
    }
    for t in bucket_tags(bucket) {
        if !tags.contains(&t) {
            tags.push(t);
        }
    }
    Some(tags)
}

/// The pack's OWN preview jpg for a source file, if the pinned tree ships
/// one (`(Footage)/previews/<same basename>.jpg`, census-verified 5/5).
/// An alpha element's first frame is usually BLANK — the shipped preview
/// is the honest poster; a generated one would lie. Probes a few ancestor
/// shapes because the operator may pin the pack root or a subfolder.
pub fn preview_poster(src: &Path) -> Option<PathBuf> {
    let stem = src.file_stem()?.to_str()?;
    for anc in src.ancestors().skip(1).take(5) {
        for candidate in [
            anc.join("(Footage)").join("previews"),
            anc.join("1. RTFX pack").join("(Footage)").join("previews"),
        ] {
            let jpg = candidate.join(format!("{stem}.jpg"));
            if jpg.is_file() {
                return Some(jpg);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(stem: &str, bucket: &str) -> Vec<String> {
        rtfx_tags(stem, bucket).expect(stem)
    }

    #[test]
    fn the_canonical_element_shape_parses() {
        let t = tags("Elements - Fire 096 Ray Up noCT noRSZ", "FX elements");
        assert!(t.contains(&"rtfx".into()));
        assert!(t.contains(&"fire".into()));
        assert!(t.contains(&"dir:up".into()));
        assert!(t.contains(&"no-tint".into()));
        assert!(t.contains(&"no-resize".into()));
        assert!(t.contains(&"fx".into()));
        assert!(!t.iter().any(|x| x == "ray"), "descriptors stay name-only");
    }

    /// The census's real irregulars: MiX casing, double spaces, lowercase
    /// direction/loop drift — all parse to the same tags.
    #[test]
    fn census_typos_and_drift_are_tolerated() {
        let t = tags(
            "Elements - Fire 062 Slash Down MiX noCT noRSZ",
            "FX elements",
        );
        assert!(t.contains(&"mix".into()));
        let t = tags(
            "Elements - Simple Energy 086 Charge  Radial  noRSZ",
            "FX elements",
        );
        assert!(t.contains(&"energy".into()));
        assert!(t.contains(&"simple".into()));
        assert!(t.contains(&"dir:radial".into()));
        let t = tags("Elements - Sparks 012 Explosion right", "FX elements");
        assert!(t.contains(&"dir:right".into()));
        let t = tags("Elements - Smoke 067 Up loop noCT noRSZ", "FX elements");
        assert!(t.contains(&"loop".into()));
    }

    #[test]
    fn transitions_mattes_backgrounds_and_sfx_bucket_correctly() {
        let t = tags(
            "Transition - V02 Energy 006 Radial IN CW",
            "Transitions/Elemental",
        );
        assert!(t.contains(&"energy".into()));
        assert!(t.contains(&"transition".into()));
        assert!(t.contains(&"dir:in".into()));
        assert!(t.contains(&"dir:cw".into()));
        let t = tags("Elements - Matte 001 Right aInv Stch", "Transitions/Matte");
        assert!(t.contains(&"matte".into()));
        assert!(t.contains(&"inverted".into()));
        assert!(t.contains(&"stretch".into()));
        let t = tags("Elements - BG Grunge 01 Stch", "Backgrounds/Backgrounds");
        assert!(t.contains(&"bg-grunge".into()));
        assert!(t.contains(&"bg".into()));
        let t = tags("SFX - Energy 03 Laser 8bit", "6. Sound FX/8bit");
        assert!(t.contains(&"sfx".into()));
        assert!(t.contains(&"energy".into()));
        assert!(t.contains(&"8bit".into()));
    }

    /// Non-RTFX names refuse honestly — never a guessed tag set.
    #[test]
    fn non_rtfx_shapes_return_none() {
        assert!(rtfx_tags("holiday-clip-final", "FX elements").is_none());
        assert!(rtfx_tags("Elements - ", "FX elements").is_none());
        assert!(
            rtfx_tags("Elements - Blizzard", "Sources").is_none(),
            "no variant number = a source dir name, not a catalog item"
        );
    }

    #[test]
    fn preview_poster_probes_the_pack_shape() {
        let root = std::env::temp_dir()
            .join("dpl-composer-tests")
            .join(format!("rtfx-prev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let el = root.join("2. Prerendered animations").join("FX elements");
        let prev = root.join("1. RTFX pack").join("(Footage)").join("previews");
        std::fs::create_dir_all(&el).unwrap();
        std::fs::create_dir_all(&prev).unwrap();
        let mov = el.join("Elements - Fire 001 Ray Up noCT noRSZ.mov");
        std::fs::write(&mov, b"mov").unwrap();
        std::fs::write(
            prev.join("Elements - Fire 001 Ray Up noCT noRSZ.jpg"),
            b"jpg",
        )
        .unwrap();
        let found = preview_poster(&mov).expect("preview found");
        assert!(found
            .ends_with(Path::new("previews").join("Elements - Fire 001 Ray Up noCT noRSZ.jpg")));
        assert!(
            preview_poster(&el.join("Elements - Fire 002 Up.mov")).is_none(),
            "no shipped preview = None (write_poster fallback owns it)"
        );
    }
}
