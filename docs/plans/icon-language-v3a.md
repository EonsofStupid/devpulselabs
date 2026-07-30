> **ALIGNMENT NOTE (2026-07-30):** authored in a parallel session while
> M1 (#163) landed the engine, registry, and gallery differently (the
> zero-dep `dpl-svg` engine, meshes baked at build, `assets/lucide/`).
> The engine/registry sections below are HISTORICAL; the live remainder
> — styled treatment variants + V3b — is gated on the vision mock and
> will be re-planned against the landed engine. Engine truth:
> `docs/design/adr-015-svg-icon-engine.md`; the filed proposal:
> `docs/design/adr-016-icon-language.md`.

# V3a — The icon language: Lucide sources → generated stroke geometry

POAM: `docs/plans/poam-visual-experience.md` V3, deliverables 1–3 (intake,
ADR, registry + styled paint vocabulary). V3b (the rail wears them, pairs
with V4) is the NEXT pass — no UI is touched here.

## 1. Context

The operator redirected V3's source on 2026-07-29: *"Lucide for now that we
will style in neubrutalist and claymorphism variants"* — their personal SVG
library (years of owned assets) becomes a later intake, not the gate. Icons
today are Phosphor font glyphs styled by color only; the operator wants a
styled icon LANGUAGE (neubrutalist + clay variants) and, eventually, their
own SVGs flowing through it. This pass builds the pipeline once — Lucide
first through it, operator SVGs later through the SAME seam — per
`docs/design/adr-015-icon-language.md` (rides this PR).

Note: `cl::icon`'s doc-header says the Phosphor codepoints sit "under the
OLD lucide inventory's semantic names" — Lucide is the original app's
actual icon family. Port fidelity, not a new direction.

## 2. Current state (all read in-session 2026-07-29)

- `crates/coldlight/src/lib.rs:687-717` — `pub mod icon`: ~24 Phosphor
  codepoint consts, one per MEANING. **38 call sites across 10 crates**
  (app, deck, shell×2 files, viziwiz, hubs: workbench/shows/home/
  wiredfront/present) — all keep compiling untouched; this pass is additive.
- `crates/coldlight/src/lib.rs:16-1146` — eight `include!(OUT_DIR/…_gen.rs)`
  sites; the trait compositor `compose(tone, base, stack, state) →
  ResolvedStyle` (line 368); `treated`/`brutal`/`clay` `Treated` surface API
  (lines 470-533); inline `mod trait_tests` (line 536) and `mod tests`
  (line 1373).
- `crates/coldlight/build.rs` — `main()` (lines 88-107): rerun-if-changed
  per token file + eight `generate_*` calls; `generate_traits` (line 688)
  is the validate-at-build precedent (unknown ref → panic with context).
- `crates/coldlight/Cargo.toml` — deps: `egui`; build-deps: `serde_json`,
  `palette`. `crates/coldlight/assets/` holds `Phosphor.ttf` +
  `Phosphor-LICENSE-MIT.txt` (the vendoring precedent).
- `crates/coldlight/tokens/traits.json` — neubrutalist shadow axis
  `offset [3,3], blur 0, black alpha 230`; claymorphic = content-plane.
- `crates/hub/src/lib.rs:51` — `HubSpec.icon: &'static str` ("until the
  sprite atlas lands") — NOT touched this pass.
- `crates/xtask/src/main.rs:635` — the structure audit ceilings dpl-doc
  only w.r.t. coldlight; coldlight itself has no ceiling and build-deps
  are not audited. No audit-table change needed (ADR records the deps).
- No SVG tooling exists anywhere in the workspace.
- API facts verified against docs.rs (svgtypes 0.16.1):
  `SimplifyingPathParser` yields `SimplePathSegment::{MoveTo{x,y},
  LineTo{x,y}, CurveTo{x1,y1,x2,y2,x,y}, Quadratic{x1,y1,x,y}, ClosePath}`
  — no Arc variant (arcs pre-converted to curves).

## 3. Target design

```
assets/icons/lucide/*.svg  (vendored, pinned tag, ISC license beside)
tokens/icons.json          (AUTHORED — one entry per MEANING; census law)
   ↓ build.rs generate_icons  (parse → flatten → validate; bad input FAILS)
OUT_DIR/icons_gen.rs       (const ICONS: &[IconShape] — 24-box polylines)
   ↓ include! in pub mod icons
cl::icons::{ICONS, shape(id), paint, paint_brutal, clay_chip}
```

- **Registry ids are MEANINGS** (`home`, `workbench`, …) mirroring
  `cl::icon`'s law; two meanings may share a source file (today SHOWS and
  LIBRARY share one glyph — kept faithfully).
- **Scale story:** icon N+1 = one vendored SVG + one json line. Operator
  SVG intake later = a sibling source dir + entries; zero pipeline change.
- **Styling is paint-time** (ADR-015 §4): plain tintable stroke; brutal =
  the trait's hard-offset shadow at icon scale; clay = icon IN
  `cl::clay()` — no bespoke frames, trait data stays the styling truth.
- **Failure law:** unknown id at runtime → `None` (caller degrades
  honestly); missing/invalid source at build → panic naming the icon.

The 24 meanings → 20 Lucide files (names verified at vendor time; a wrong
name 404s and stops the step):

| ids | Lucide file |
|---|---|
| home | house.svg |
| workbench | hammer.svg |
| library, shows | layout-grid.svg |
| wiredfront | clapperboard.svg |
| bot | bot.svg |
| present, session | monitor-play.svg |
| snip, img | camera.svg |
| dock | chevron-up.svg |
| fullbleed | maximize-2.svg |
| fullbleed-exit | minimize-2.svg |
| close | x.svg |
| back | chevron-left.svg |
| video | video.svg |
| steps, mark | footprints.svg |
| rec | circle-dot.svg |
| stop | square.svg |
| region | scan.svg |
| window | app-window.svg |
| full | monitor.svg |
| free | lasso.svg |

## 4. Changes, in execution order

**Step 0 — the mock gate.** This plan executes ONLY after the operator
ratifies the vision mock (`docs/design/mocks/visual-language-v3-mock.html`
— operator 2026-07-29: "mocks built first for a clear vision to work
from"). Redlines on the mock amend THIS plan before any code.

**Step 0b — lane discipline.** Branch `feat/v3a-icon-language` cut from
`origin/main` (NOT from `feat/m1-provider-easy-select` — that is the chat
lane). Invoke the `coldlight-system` gate at execution start (this pass
extends the design-system crate). Re-check `gh pr list` for a competing
icon PR before authoring (adopt, don't duplicate).

**Step 1 — vendor Lucide (create ~21 files).**
`crates/coldlight/assets/icons/lucide/*.svg` + `LICENSE.txt`:

```powershell
$tag = gh api repos/lucide-icons/lucide/releases/latest --jq .tag_name
New-Item -ItemType Directory -Force crates\coldlight\assets\icons\lucide
$names = "house","hammer","layout-grid","clapperboard","bot","monitor-play",
         "camera","chevron-up","maximize-2","minimize-2","x","chevron-left",
         "video","footprints","circle-dot","square","scan","app-window",
         "monitor","lasso"
foreach ($n in $names) {
  curl.exe -fsSL "https://raw.githubusercontent.com/lucide-icons/lucide/$tag/icons/$n.svg" `
    -o "crates\coldlight\assets\icons\lucide\$n.svg"   # -f: a wrong name FAILS here
}
curl.exe -fsSL "https://raw.githubusercontent.com/lucide-icons/lucide/$tag/LICENSE" `
  -o "crates\coldlight\assets\icons\lucide\LICENSE.txt"
```

Record `$tag` in `tokens/icons.json` `source.tag` (step 2).

**Step 2 — create `crates/coldlight/tokens/icons.json`** (full contents;
`source.tag` filled with step 1's value):

```json
{
	"$description": "AUTHORED — the icon language (ADR-015): one entry per MEANING, mirroring mod icon's semantic law. file paths are relative to assets/icons/; two meanings may share a file. Generated to icons_gen.rs; a missing file, unsupported SVG element, or empty geometry fails the build. Unknown ids at runtime degrade to None (callers fall back to Phosphor glyphs).",
	"source": { "set": "lucide", "tag": "<PINNED-AT-VENDOR>", "license": "assets/icons/lucide/LICENSE.txt" },
	"icons": [
		{ "id": "home", "label": "Home", "file": "lucide/house.svg" },
		{ "id": "workbench", "label": "Workbench", "file": "lucide/hammer.svg" },
		{ "id": "library", "label": "Library", "file": "lucide/layout-grid.svg" },
		{ "id": "shows", "label": "Shows", "file": "lucide/layout-grid.svg" },
		{ "id": "wiredfront", "label": "WiredFRONT", "file": "lucide/clapperboard.svg" },
		{ "id": "bot", "label": "Bot", "file": "lucide/bot.svg" },
		{ "id": "present", "label": "Present", "file": "lucide/monitor-play.svg" },
		{ "id": "session", "label": "Session", "file": "lucide/monitor-play.svg" },
		{ "id": "snip", "label": "Snip", "file": "lucide/camera.svg" },
		{ "id": "img", "label": "Image", "file": "lucide/camera.svg" },
		{ "id": "dock", "label": "Dock", "file": "lucide/chevron-up.svg" },
		{ "id": "fullbleed", "label": "Full-bleed", "file": "lucide/maximize-2.svg" },
		{ "id": "fullbleed-exit", "label": "Exit full-bleed", "file": "lucide/minimize-2.svg" },
		{ "id": "close", "label": "Close", "file": "lucide/x.svg" },
		{ "id": "back", "label": "Back", "file": "lucide/chevron-left.svg" },
		{ "id": "video", "label": "Video", "file": "lucide/video.svg" },
		{ "id": "steps", "label": "Steps", "file": "lucide/footprints.svg" },
		{ "id": "mark", "label": "Mark", "file": "lucide/footprints.svg" },
		{ "id": "rec", "label": "Record", "file": "lucide/circle-dot.svg" },
		{ "id": "stop", "label": "Stop", "file": "lucide/square.svg" },
		{ "id": "region", "label": "Region", "file": "lucide/scan.svg" },
		{ "id": "window", "label": "Window", "file": "lucide/app-window.svg" },
		{ "id": "full", "label": "Full screen", "file": "lucide/monitor.svg" },
		{ "id": "free", "label": "Freeform", "file": "lucide/lasso.svg" }
	]
}
```

**Step 3 — edit `crates/coldlight/Cargo.toml`** build-deps (versions =
current latest on crates.io at execution; the APIs used are stable):

```toml
[build-dependencies]
serde_json = "1"
palette = "0.7"
roxmltree = "0.20"
svgtypes = "0.16"
kurbo = "0.11"
```

**Step 4 — edit `crates/coldlight/build.rs`.** In `main()` after
`println!("cargo:rerun-if-changed=tokens/traits.json");` add:

```rust
    println!("cargo:rerun-if-changed=tokens/icons.json");
    println!("cargo:rerun-if-changed=assets/icons");
```

after `generate_traits(&out_dir);` add `generate_icons(&out_dir);`, and at
the end of the file append:

```rust
// ── icons — the icon language (ADR-015) ─────────────────────────────────

/// tokens/icons.json + assets/icons/** → the ICONS registry. Each icon's
/// stroke CENTERLINES are flattened once here into 24-box polylines and
/// emitted as const data; paint scales + strokes at runtime (any size,
/// any tone). Vocabulary: path/circle/ellipse/rect/line/polyline/polygon —
/// anything else, a missing file, or empty geometry FAILS the build.
fn generate_icons(out_dir: &str) {
    let raw = std::fs::read_to_string("tokens/icons.json").expect("read icons.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse icons.json");
    let icons = doc["icons"].as_array().expect("icons.json: icons[]");
    assert!(!icons.is_empty(), "icons.json: empty");

    let mut seen = std::collections::BTreeSet::new();
    let mut code = String::from(
        "// GENERATED by build.rs from tokens/icons.json + assets/icons — DO NOT EDIT.\n\npub const ICONS: &[IconShape] = &[\n",
    );
    for ic in icons {
        let id = ic["id"].as_str().expect("icon.id");
        let label = ic["label"].as_str().expect("icon.label");
        let file = ic["file"].as_str().expect("icon.file");
        assert!(seen.insert(id.to_string()), "icons.json: duplicate id {id:?}");
        let path = Path::new("assets/icons").join(file);
        let svg = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("icon {id}: read {}: {e}", path.display()));
        let lines = icon_lines(&svg, id);
        assert!(!lines.is_empty(), "icon {id}: no geometry");
        let _ = write!(code, "    IconShape {{ id: {id:?}, label: {label:?}, lines: &[");
        for (pts, closed) in &lines {
            assert!(pts.len() > 1, "icon {id}: degenerate polyline");
            let _ = write!(code, "(&[");
            for &(x, y) in pts {
                assert!(
                    (-1.0..=25.0).contains(&x) && (-1.0..=25.0).contains(&y),
                    "icon {id}: point ({x},{y}) outside the 24-box"
                );
                let _ = write!(code, "egui::Pos2 {{ x: {}f32, y: {}f32 }}, ", x as f32, y as f32);
            }
            let _ = write!(code, "], {closed}), ");
        }
        let _ = writeln!(code, "] }},");
    }
    code.push_str("];\n");
    std::fs::write(Path::new(out_dir).join("icons_gen.rs"), code).expect("write icons_gen.rs");
}

/// One SVG's drawable elements → flattened centerline polylines in its
/// 24-unit box. Flattening splits at every MoveTo; `true` = closed loop.
fn icon_lines(svg: &str, id: &str) -> Vec<(Vec<(f64, f64)>, bool)> {
    use kurbo::Shape as _;
    let tree =
        roxmltree::Document::parse(svg).unwrap_or_else(|e| panic!("icon {id}: bad xml: {e}"));
    let mut out: Vec<(Vec<(f64, f64)>, bool)> = Vec::new();
    for node in tree.descendants().filter(|n| n.is_element()) {
        let name = node.tag_name().name();
        let num = |k: &str| -> f64 {
            node.attribute(k)
                .unwrap_or_else(|| panic!("icon {id}: <{name}> missing {k}"))
                .parse()
                .unwrap_or_else(|_| panic!("icon {id}: <{name}> {k} not a number"))
        };
        match name {
            "svg" => assert_eq!(
                node.attribute("viewBox"),
                Some("0 0 24 24"),
                "icon {id}: not a 24-box"
            ),
            "path" => {
                let d = node
                    .attribute("d")
                    .unwrap_or_else(|| panic!("icon {id}: <path> missing d"));
                let mut bp = kurbo::BezPath::new();
                for seg in svgtypes::SimplifyingPathParser::from(d) {
                    use svgtypes::SimplePathSegment as S;
                    match seg.unwrap_or_else(|e| panic!("icon {id}: path data: {e}")) {
                        S::MoveTo { x, y } => bp.move_to((x, y)),
                        S::LineTo { x, y } => bp.line_to((x, y)),
                        S::CurveTo { x1, y1, x2, y2, x, y } => {
                            bp.curve_to((x1, y1), (x2, y2), (x, y))
                        }
                        S::Quadratic { x1, y1, x, y } => bp.quad_to((x1, y1), (x, y)),
                        S::ClosePath => bp.close_path(),
                    }
                }
                flatten_into(bp, &mut out);
            }
            "circle" => flatten_into(
                kurbo::Circle::new((num("cx"), num("cy")), num("r")).to_path(0.05),
                &mut out,
            ),
            "ellipse" => flatten_into(
                kurbo::Ellipse::new((num("cx"), num("cy")), (num("rx"), num("ry")), 0.0)
                    .to_path(0.05),
                &mut out,
            ),
            "rect" => {
                let r = kurbo::Rect::new(
                    num("x"),
                    num("y"),
                    num("x") + num("width"),
                    num("y") + num("height"),
                );
                let rx = node
                    .attribute("rx")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                flatten_into(kurbo::RoundedRect::from_rect(r, rx).to_path(0.05), &mut out);
            }
            "line" => out.push((vec![(num("x1"), num("y1")), (num("x2"), num("y2"))], false)),
            "polyline" | "polygon" => {
                let pts_attr = node
                    .attribute("points")
                    .unwrap_or_else(|| panic!("icon {id}: <{name}> missing points"));
                let nums: Vec<f64> = pts_attr
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        s.parse()
                            .unwrap_or_else(|_| panic!("icon {id}: <{name}> bad point {s:?}"))
                    })
                    .collect();
                assert!(
                    nums.len() >= 4 && nums.len() % 2 == 0,
                    "icon {id}: <{name}> odd points"
                );
                let pts: Vec<(f64, f64)> = nums.chunks(2).map(|c| (c[0], c[1])).collect();
                out.push((pts, name == "polygon"));
            }
            other => panic!(
                "icon {id}: unsupported element <{other}> — extend the vocabulary deliberately"
            ),
        }
    }
    out
}

/// Flatten a kurbo path into `out`, splitting at each MoveTo.
fn flatten_into(bp: kurbo::BezPath, out: &mut Vec<(Vec<(f64, f64)>, bool)>) {
    let mut cur: Vec<(f64, f64)> = Vec::new();
    let mut closed = false;
    bp.flatten(0.05, |el| match el {
        kurbo::PathEl::MoveTo(p) => {
            if cur.len() > 1 {
                out.push((std::mem::take(&mut cur), closed));
            }
            cur = vec![(p.x, p.y)];
            closed = false;
        }
        kurbo::PathEl::LineTo(p) => cur.push((p.x, p.y)),
        kurbo::PathEl::ClosePath => closed = true,
        _ => unreachable!("flatten yields Move/Line/Close only"),
    });
    if cur.len() > 1 {
        out.push((cur, closed));
    }
}
```

**Step 5 — edit `crates/coldlight/src/lib.rs`.** Directly after `pub mod
icon { … }` (line 717) insert:

```rust
/// The icon LANGUAGE (ADR-015): stroke icons flattened at build from
/// vendored SVG sources (Lucide today; the operator's library later)
/// into const 24-box polylines — painted at any size in any tone.
/// `icon` above (Phosphor font glyphs) stays the fallback vocabulary;
/// surfaces adopt this registry route by route (V3b: the rail).
pub mod icons {
    /// One icon: its stroke centerlines in the 24×24 design box.
    #[derive(Debug)]
    pub struct IconShape {
        pub id: &'static str,
        pub label: &'static str,
        /// (points, closed) — centerlines; the stroke is applied at paint.
        pub lines: &'static [(&'static [egui::Pos2], bool)],
    }

    include!(concat!(env!("OUT_DIR"), "/icons_gen.rs"));

    /// An icon by id, or None — the caller degrades to a Phosphor glyph
    /// or a bullet, never a panic.
    pub fn shape(id: &str) -> Option<&'static IconShape> {
        ICONS.iter().find(|i| i.id == id)
    }

    /// Paint contain-fit into `rect`. Lucide's native 2.0 stroke scales
    /// with the box; `weight` multiplies it (1.0 = faithful).
    pub fn paint(
        painter: &egui::Painter,
        icon: &IconShape,
        rect: egui::Rect,
        color: egui::Color32,
        weight: f32,
    ) {
        let scale = rect.width().min(rect.height()) / 24.0;
        let origin = rect.center() - egui::vec2(12.0, 12.0) * scale;
        let stroke = egui::Stroke::new(2.0 * scale * weight, color);
        for (pts, closed) in icon.lines {
            let mapped: Vec<egui::Pos2> =
                pts.iter().map(|p| origin + p.to_vec2() * scale).collect();
            painter.add(if *closed {
                egui::Shape::closed_line(mapped, stroke)
            } else {
                egui::Shape::line(mapped, stroke)
            });
        }
    }

    /// The NEUBRUTALIST icon: the trait's hard offset shadow axis
    /// ([3,3], blur 0 — tokens/traits.json) as a black under-copy at
    /// icon scale, the tone's stroke on top. For bare glyph seats.
    pub fn paint_brutal(
        painter: &egui::Painter,
        icon: &IconShape,
        rect: egui::Rect,
        tone: super::Tone,
    ) {
        let scale = rect.width().min(rect.height()) / 24.0;
        let off = egui::vec2(3.0, 3.0) * scale;
        paint(
            painter,
            icon,
            rect.translate(off),
            egui::Color32::from_black_alpha(230),
            1.0,
        );
        paint(painter, icon, rect, tone.color(), 1.0);
    }

    /// The CLAYMORPHIC icon chip: the icon sits IN the clay plane
    /// (doctrine: content sits in it; clay is never a button). Returns
    /// the response for the tooltip law.
    pub fn clay_chip(
        ui: &mut egui::Ui,
        icon: &IconShape,
        size: f32,
        tip: &str,
    ) -> egui::Response {
        let ink =
            super::compose(super::Tone::Cyan, "claymorphic", &[], super::TraitState::Idle).ink;
        let inner = super::clay().margin(egui::Margin::same(6)).show(ui, |ui| {
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            paint(ui.painter(), icon, rect, ink, 1.0);
        });
        inner.response.on_hover_text(tip)
    }
}
```

**Step 6 — tests.** In the same file, immediately after `mod trait_tests`
(line 595's closing brace) insert:

```rust
#[cfg(test)]
mod icon_language_tests {
    use super::icons;

    #[test]
    fn the_registry_is_a_census() {
        assert!(!icons::ICONS.is_empty());
        let mut seen = std::collections::BTreeSet::new();
        for i in icons::ICONS {
            assert!(seen.insert(i.id), "duplicate icon id {:?}", i.id);
            assert!(!i.lines.is_empty(), "icon {} has no geometry", i.id);
            for (pts, _) in i.lines {
                assert!(pts.len() > 1, "icon {}: degenerate polyline", i.id);
                for p in *pts {
                    assert!(
                        (-1.0..=25.0).contains(&p.x) && (-1.0..=25.0).contains(&p.y),
                        "icon {}: point outside the 24-box: {p:?}",
                        i.id
                    );
                }
            }
        }
    }

    #[test]
    fn every_phosphor_meaning_has_a_language_icon() {
        // mod icon (Phosphor) and mod icons stay in lockstep: one styled
        // icon per existing MEANING. Extending mod icon extends this list.
        for id in [
            "home", "workbench", "library", "wiredfront", "shows", "bot", "present",
            "snip", "dock", "fullbleed", "fullbleed-exit", "close", "back", "img",
            "video", "session", "steps", "rec", "stop", "mark", "region", "window",
            "full", "free",
        ] {
            assert!(icons::shape(id).is_some(), "meaning {id:?} missing from icons.json");
        }
    }

    #[test]
    fn unknown_id_degrades_to_none() {
        assert!(icons::shape("definitely-not-an-icon").is_none());
    }
}
```

**Step 7 — docs.** `docs/design/adr-015-icon-language.md` (already
authored, rides this PR); POAM V3 amendment (already applied); STATE.md
entry at session end.

## 5. Boundaries

- 🚫 `pub mod icon` (Phosphor) — 38 call sites in 10 crates depend on it;
  it REMAINS the fallback vocabulary. Not one line changes.
- 🚫 No UI: no rail, no `HubSpec`, no shell/hub edits — that is V3b.
- 🚫 No runtime deps added to coldlight (`egui` stays its only dep).
- 🚫 `/content`, no `#[allow]`, sources never mutated (vendored SVGs are
  checked in verbatim and never edited).
- ⚠️ Ask first: any icon meaning beyond the 24, any non-Lucide source,
  any change to the `IconShape` wire shape after V3b starts consuming it.
- Out of scope: operator SVG intake (later pass, same pipe), V4 buttons,
  icon-only rail adoption (V3b).

## 6. Verification (in order; each pass/fail)

1. `cargo build -p dpl-coldlight` → exit 0 (generator ran; `icons_gen.rs`
   exists under the crate's OUT_DIR).
2. `cargo test -p dpl-coldlight` → all green including the 3 new tests.
3. Negative proof (evidence only, then reverted): point `home` at
   `lucide/does-not-exist.svg` in icons.json → `cargo build -p
   dpl-coldlight` FAILS with `icon home: read …` in the message → revert.
4. `cargo green` → exit 0 (fmt, clippy, tests, structure audit, release
   build — proves the workspace-wide graph and the audit are untouched).
5. `cargo build --release -p devpulselabs -p dpl-lottie-render` → exe
   mtime moves (session-end law; no behavior change expected — the app
   does not consume the registry until V3b).
6. PR checks: `gh pr checks <n> --watch` → BOTH `green-windows` +
   `green-linux` COMPLETED SUCCESS before merge (double-green law).

End-to-end proof for THIS pass = the census tests + generated-code excerpt
in §8. The VISUAL proof (styled icons on the real exe at real window
sizes) is V3b's evidence gate by design — stated honestly, not skipped.

## 7. Commit plan

- Branch: `feat/v3a-icon-language` off `origin/main`.
- `git add`: `docs/design/adr-015-icon-language.md`,
  `docs/plans/icon-language-v3a.md`, `docs/plans/poam-visual-experience.md`,
  `crates/coldlight/tokens/icons.json`, `crates/coldlight/assets/icons/`,
  `crates/coldlight/Cargo.toml`, `Cargo.lock`, `crates/coldlight/build.rs`,
  `crates/coldlight/src/lib.rs`.
- Message: `feat(coldlight): the icon language — Lucide sources →
  generated stroke geometry (ADR-015, V3a)`.
- `gh pr create` → double-green watch → merge. Never `--no-verify`.

## 8. Evidence (filled at execution)

- [ ] build + test output (steps 1–2)
- [ ] negative-proof output (step 3)
- [ ] `cargo green` tail (step 4)
- [ ] exe mtime (step 5)
- [ ] `icons_gen.rs` excerpt (one icon entry) + vendored tag recorded
- [ ] PR number + both lanes green
