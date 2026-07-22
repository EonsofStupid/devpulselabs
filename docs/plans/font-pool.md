# F2 — the ColdLight font pool: local, authored, on a wall

Repo `C:\localdev\devpulselabs`, branch `main` at `77f8e98`. Ratified by the
operator 2026-07-22: *"We should also be downloading and having the fonts
local. Can we take a moment to properly download some retro, futuristic,
pool of fonts to have available?"* — pass order approved ("go").

## 1. Context

ColdLight ships one vendored font today (Phosphor icons). All text renders
in egui's built-in families. The operator wants a curated retro/futuristic
pool, vendored locally, available to every surface — and choosable by eye,
not by name. Per the LIBRARIES-FIRST law, fonts become the FIFTH authored
token source: manifest → generated registry → runtime → gallery wall.

## 2. Current state (read this session)

- `crates/coldlight/build.rs` — generators for ramps/fluid/motion/moods/
  aurora; the pattern to extend.
- `crates/coldlight/src/lib.rs:114-169` — `apply_theme` builds
  `FontDefinitions::default()`, inserts "phosphor", pushes it as fallback on
  both default families, sets text styles.
- `crates/hubs/library/src/lib.rs` — subroutes COMPONENTS/TEMPLATES/MOODS;
  `moods_wall` is the wall pattern (horizontal_wrapped cards in a
  ScrollArea).

## 3. Target design

13 families, all SIL OFL, vendored from the google/fonts repo with license
files:

| class   | fonts |
|---------|-------|
| display | Orbitron, Michroma, Audiowide, Monoton |
| retro   | Press Start 2P, VT323 |
| ui      | Exo 2, Rajdhani, Chakra Petch, Oxanium, Space Grotesk |
| mono    | Share Tech Mono, JetBrains Mono |

`tokens/fonts.json` (authored) → `generate_fonts()` in build.rs →
`fonts_gen.rs` (`static FONTS: &[FontDef]`, bytes via include_bytes!) →
`apply_theme` registers every family as `FontFamily::Name(id)` with the
default sans + phosphor as fallback → `cl::font_family(id)` helper → a
**Fonts wall** in Library (gallery gate: every vendored font renders, named,
with a sample line, in itself). Default chrome font is UNCHANGED this pass —
the operator picks from the wall in the design session (D1).

Scale story: a new font = drop the TTF + one manifest entry; it registers
and appears on the wall with zero further code.

## 4. Changes, in execution order

1. Download 13 TTFs → `crates/coldlight/assets/fonts/`, licenses →
   `assets/fonts/licenses/<family>-OFL.txt` (raw.githubusercontent.com/
   google/fonts/main/ofl/...; variable-font files use the default instance —
   ab_glyph loads them fine).
2. **create `crates/coldlight/tokens/fonts.json`** — `{fonts:[{id,label,
   class,file,blurb}]}`, classes display|retro|ui|mono.
3. **edit `crates/coldlight/build.rs`** — `generate_fonts()`: parse manifest,
   assert each file exists, emit `FONTS`/`FONT_COUNT` with
   `include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/…"))`;
   rerun-if-changed on fonts.json + assets/fonts.
4. **edit `crates/coldlight/src/lib.rs`** — `pub struct FontDef {id,label,
   class,blurb,bytes}` + `include!(fonts_gen.rs)` + `pub fn
   font_family(id) -> egui::FontFamily`; `apply_theme` loops FONTS into
   `fonts.font_data` and `FontFamily::Name(id)` (fallback: default sans,
   phosphor). Tests: ids unique; every family lays out a glyph through a
   real `Context::run` (a corrupt TTF fails here, not at app boot).
5. **edit `crates/hubs/library/src/lib.rs`** — `FONTS` subroute ("Fonts") +
   `fonts_wall()`: one card per FontDef — label rendered IN the font (26pt),
   sample "The quick brown fox — 0123456789" (17pt), class + id monospace.
6. `cargo green`; exe proof `--route library/fonts` screenshot; commit.

## 5. Boundaries

- 🚫 Do not change the default chrome font or text styles this pass (D1,
  operator-driven).
- 🚫 Phosphor registration and `cl::icon` untouched.
- Out of scope: per-surface font pickers; weight variants beyond each
  file's default instance.

## 6. Verification

1. `cargo green` exits 0 (includes the new parse-every-font test).
2. Manifest⇄disk integrity enforced at build time (missing file = build
   error, not a runtime plate).
3. Real exe: `--route library/fonts --screenshot docs/proof/fonts-wall.png`
   — all 13 families visibly distinct, each label in its own face.

## 7. Commit plan

`feat(coldlight): the font pool — 13 vendored OFL families, authored
manifest, generated registry, Library fonts wall`. Adds assets/fonts (TTFs +
licenses), tokens/fonts.json, build.rs + lib.rs changes, library hub wall,
proof shot. No push.
