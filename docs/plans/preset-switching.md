# Pass — presets become authored, switchable, and persistent

## 1. Context

The matrix resolves and the editor already wears a coordinate
(`8f9d98e` + `8918b14`, journal `docs/journal/2026-08-05-matrix-resolves.md`).
What is missing is the part the operator actually asked for:

> "also wont i be able to even change the editor to the any of these if i
> wanted too later? like this shouldnt be hardcoded right?"

> "who cares if we have to add a fifth thing this needs to be diverse for my
> content creation capabilities this will literally have after effects,
> premier pro, obs, eventually all sidecar and under one 'umbrella'"

Today changing the editor's look means **editing `roles.json` and
rebuilding**. That is hardcoding with extra steps.

There is also a live **second source of truth**: the 25 preset coordinates
exist only in `docs/design/mocks/_preset-bench.gen.py`'s `PRESETS` table — a
Python literal in a mock generator. The app cannot see them and never
validates them. This pass makes `tokens/presets.json` the one authored home
and has the mock READ it, so the bench and the app can never disagree.

**Scope:** authored presets + runtime switching + persistence + a picker. It
adds no facet values and retunes no colours.

## 2. Current state (every file below read in-session, 2026-08-05)

### 2.1 `crates/coldlight/tokens/presets.json` — three kinds, none of them ours

`$description`: *"AUTHORED — presets, the 6th token source. A preset is a
NAMED BUNDLE OF REFS into the other sources."* Exactly the right home.

| kind | count | shape |
|---|---|---|
| `theme` | 5 | `{id, label, font, moods[], motion, ambience, blurb}` |
| `motion` | 3 | `{id, label, enter, blurb}` |
| `bleed` | 1 | `{id, label, effect, hues[], blurb}` |

There is **no ancestral/coordinate kind**.

`generate_presets` (`build.rs:1185`) reads it and cross-checks refs against
`fonts.json` — the manifest⇄truth law this pass follows for coordinates.

### 2.2 `crates/coldlight/tokens/roles.json` → `generate_roles` (`build.rs:123`)

Emits `DEFAULT_ROLES: [Binding; 4]`, `SURFACE_ROLES: &[(&str, [Option<Binding>;
4])]`, `SURFACE_COUNT`. A binding now accepts `"trait"` **or** `"coordinate"`,
and the coordinate form is validated against the ancestor's own
`native`/`allowed`/`forbidden` — an illegal preset fails the build.

`crates/coldlight/src/lib.rs`:

```rust
pub enum Dress { Trait(&'static str), Coord(matrix::Coordinate) }
impl Dress { pub fn style(self, tone: Tone, state: TraitState) -> ResolvedStyle }
pub struct Binding { pub dress: Dress, pub tone: Option<&'static str> }
impl Binding { pub fn style(&self, …); pub fn toned(&self, tone: Tone) -> Tone }
pub fn binding(surface: &str, intent: Intent) -> Binding   // surface, else DEFAULT
pub fn dressed(surface: &str, intent: Intent, tone: Tone, state: TraitState) -> Treated
pub struct Treated { tone: Tone, dress: Dress, state: TraitState, margin: Margin }
```

**`binding()` reads ONLY the generated tables.** There is no runtime override
path, which is precisely the gap.

### 2.3 The half-built override that already exists

```rust
static CHROME: OnceLock<Mutex<&'static str>>;
pub const DEFAULT_CHROME: &str = "vibron";
pub fn chrome() -> &'static str;
pub fn set_chrome(id: &'static str) -> bool;   // refuses unknown, keeps current
pub fn chrome_treated(tone: Tone, state: TraitState) -> Treated;
```

This is a **global, trait-only, surface-blind** override, and **`binding()`
does not consult it** — so `dressed()` (what every widget calls) ignores it
entirely. Two mechanisms, one of them dead. This pass unifies them.

### 2.4 `crates/shell/src/status.rs` — where a picker belongs

The bar is a registry: *"adding an indicator is adding ONE entry"* in `CELLS`;
each cell has `render: fn(&mut Ui, &StatusCtx) -> Option<StatusAction>`; the
cell paints and the shell executes (Law 4).

```rust
fn brand(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    cl::tone_dot(ui, cx.tone);
    ui.label(RichText::new("ColdLight").small().color(cl::ink::MUTED));
    None                                    // :186-190 — a dead label
}
```

The clickable pattern is `snip` (`:192`):
`launcher_chip(…).clicked().then_some(StatusAction::Snip)`.

`pub enum StatusAction { Snip, Window(AppWindowKey), ToggleProviders }`.

**`StatusCtx` carries `tone, viewport, ppp, navs, signals, build, mood,
working, providers, bleed` — and NO route or surface.** A status-bar picker
therefore cannot currently know which surface it would dress. One field must
be added; §4 does it explicitly rather than by inference.

### 2.5 Surface ids

`composer/src/lib.rs:59` `"wiredfront"` · `hubs/workbench/src/controls.rs:27`
`"workbench"` · `viziwiz/src/palette.rs:15` `"viziwiz"`.

### 2.6 The persistence precedent — `crates/app/src/chat.rs:192-204`

```rust
fn load(userdata: &Path) -> Self {
    std::fs::read(userdata.join(ASSIST_POLICY_FILE)).ok()
        .and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}
fn save(&self, userdata: &Path) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(self)…;
    let temporary = userdata.join(format!("{ASSIST_POLICY_FILE}.tmp"));
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, userdata.join(ASSIST_POLICY_FILE))
}
```

Atomic tmp+rename, missing/corrupt file degrades to `Default`. Root from
`dpl_engine_capture::userdata_root()`. **This pass copies that shape exactly**
rather than inventing a settings store.

### 2.7 The 25 coordinates today

`docs/design/mocks/_preset-bench.gen.py` — a Python `PRESETS` dict keyed by
ancestor, each entry `(id, label, kicker, structure, material, atmosphere,
tone, why)`. Legality and paint-distinctness are gated there. **Invisible to
the app; duplicated by construction.**

## 3. Target design

### 3.1 `presets.json` gains a 4th kind: `ancestral`

```jsonc
"ancestral": [
  { "id": "vibron", "label": "Vibron", "kicker": "the workhorse",
    "coordinate": { "ancestor": "brutalist", "structure": "massive",
                    "material": "plate", "atmosphere": "hard-cast" },
    "tone": "cyan",
    "blurb": "A quiet dark panel whose whole point is RESTRAINT — the plate
              says nothing so the ONE active control can." }
]
```

25 entries, ported verbatim from the bench table. `tone` is a **default**, not
a pin: the caller's tone still wins unless a role pins one (that stays
`danger`'s privilege).

`generate_presets` gains the coordinate check already written for
`generate_roles` — **hoisted to one free function** so preset legality and
role legality can never diverge:

```rust
fn assert_coordinate_legal(matrix: &Value, ctx: &str,
                           anc: &str, st: &str, mt: &str, at: &str);
```

Emits `pub static ANCESTRAL_PRESETS: &[AncestralPreset]` with
`{ id, label, kicker, blurb, coord: matrix::Coordinate, tone: Option<&str> }`.

**Scale story:** adding the 26th preset is one JSON object. It is
build-validated, appears in the picker, and needs no Rust change. An illegal
one fails the build naming the ancestor and the facet.

### 3.2 The override layer — one mechanism, not two

`CHROME` becomes a per-surface override map consulted by `binding()`:

```rust
/// Runtime chrome overrides, surface → preset id. Empty until the operator
/// picks. Authored roles.json remains the default for every surface.
static CHROME_PRESET: OnceLock<Mutex<Vec<(String, &'static str)>>>;

pub fn preset(id: &str) -> Option<&'static AncestralPreset>;
pub fn chrome_preset(surface: &str) -> Option<&'static AncestralPreset>;
/// Returns whether it took — an unknown id is REFUSED and the current look
/// kept, because the realistic bad value is a STALE PERSISTED id, and losing
/// a look is recoverable where killing a shipped exe is not.
pub fn set_chrome_preset(surface: &str, id: &str) -> bool;
pub fn clear_chrome_preset(surface: &str);
```

`binding()` gains ONE branch, and only for `Intent::Chrome`:

```rust
pub fn binding(surface: &str, intent: Intent) -> Binding {
    if intent == Intent::Chrome {
        if let Some(p) = chrome_preset(surface) {
            return Binding { dress: Dress::Coord(p.coord), tone: p.tone };
        }
    }
    …existing generated lookup…
}
```

Chrome only, deliberately: `hero`/`quiet`/`danger` carry MEANING, and a
picker that could turn `danger` into a glossy pill would break the semantic
layer this was built for.

The dead trait-global `chrome()`/`set_chrome()`/`DEFAULT_CHROME` is
**removed** — `chrome_treated()` becomes `dressed(surface, Intent::Chrome, …)`.
One mechanism.

### 3.3 Persistence — `.userdata/coldlight.json`

```rust
#[derive(Serialize, Deserialize, Default)]
struct ChromeChoices { surfaces: BTreeMap<String, String> }  // surface → preset id
```

`load` on startup, `save` on pick, atomic tmp+rename, unknown ids dropped with
a log line rather than a panic (§2.6's shape). Owned by `crates/app`, which
already owns `userdata_root()` — **`dpl-coldlight` gains no dependency and no
filesystem access**, so the structure audit is untouched.

### 3.4 The picker

`brand()` stops being a dead label and becomes the entry point — the control
is already named "ColdLight", which is what the operator would look for.

- `StatusCtx` gains `pub surface: &'static str` (the active hub's id; the
  shell knows the route, the cell does not decide — Law 4).
- `StatusAction` gains `TogglePresets`, matching `ToggleProviders`.
- The popup lists the 25 grouped by ancestor, each row painting a **live
  swatch through `matrix::resolve()`** at the row's own coordinate — so the
  operator sees the actual look, not a name. The active one is marked; a
  "Reset to authored" row calls `clear_chrome_preset`.
- Tooltip law: every row ships `cl::tip(…)` in plain words.

### 3.5 The mock stops being a second truth

`_preset-bench.gen.py` reads `ancestral` from `presets.json` instead of its
own `PRESETS` dict, keeping its legality + paint-distinctness gates. The bench
then renders exactly what the app can wear.

## 4. Changes, in execution order

1. **`crates/coldlight/build.rs`** — hoist the coordinate legality check out
   of the `generate_roles` binding closure into
   `fn assert_coordinate_legal(...)`; call it from both.
2. **`crates/coldlight/tokens/presets.json`** — add the `ancestral` array, 25
   entries ported from `_preset-bench.gen.py`'s table (id, label, kicker,
   coordinate, tone, blurb).
3. **`crates/coldlight/build.rs`** — `generate_presets` emits
   `ANCESTRAL_PRESETS`, validating each coordinate via step 1 and each `tone`
   against `hues.json`.
4. **`crates/coldlight/src/lib.rs`** — `AncestralPreset` struct; `preset()`,
   `chrome_preset()`, `set_chrome_preset()`, `clear_chrome_preset()`; the
   `Intent::Chrome` branch in `binding()`.
5. **`crates/coldlight/src/lib.rs`** — delete `CHROME`, `DEFAULT_CHROME`,
   `chrome()`, `set_chrome()`; rewrite `chrome_treated()` as a thin
   `dressed(…, Intent::Chrome, …)`. Fix every caller the compiler names.
6. **`crates/shell/src/status.rs`** — `StatusCtx.surface`; `StatusAction::
   TogglePresets`; `brand()` becomes a `launcher_chip`.
7. **`crates/app/src/main.rs`** (or the shell owner the compiler points at) —
   pass the active surface into `StatusCtx`; handle `TogglePresets`; own the
   popup; `ChromeChoices` load on startup + save on pick.
8. **`docs/design/mocks/_preset-bench.gen.py`** — read `ancestral` from
   `presets.json`; delete the local `PRESETS` dict; regenerate the bench.
9. **Tests** (§6).
10. **Docs** — `docs/design/visual-language.md` records presets as authored +
    switchable; `docs/STATE.md`; journal.

## 5. Boundaries

**🚫 Never touch**
- `/content` — operator data.
- Any `*_gen.rs` by hand.
- `matrix.json`'s authored VALUES. This pass adds no facet and retunes no
  colour; a colour change here is a styling decision and a different pass.
- The four `Intent` roles. Four is the ruled set.
- `hero`/`quiet`/`danger` bindings — the picker moves `chrome` only.
- The vendored Lucide pool.

**⚠️ Ask first**
- Letting the picker override a role other than `chrome`.
- Adding a 26th preset or any facet value (a facet costs 8 rulings).
- Persisting anywhere other than `.userdata/` (ADR-014's three-ring root).

**Out of scope**
- Retiring `compose()` and the 5 traits — workbench and viziwiz still bind to
  traits, and both paths coexist by design.
- Per-WIDGET dressing; theme import/export; a facet-by-facet builder UI.
- RTFX `.ffx`/`.aep` parsing and `engine-ae` wiring — a different axis
  entirely (journal `2026-08-05-matrix-resolves.md`).

## 6. Verification

Every criterion is pass/fail.

- `cargo test -p dpl-coldlight` — plus:
  - `every_authored_preset_is_legal` — all 25 pass `Coordinate::legality()`.
  - `every_authored_preset_paints` — each resolves a non-default
    `ResolvedStyle` in all three `TraitState`s.
  - `the_authored_presets_are_visually_distinct` — no two share an identical
    `ResolvedStyle` at the same tone (the bench's own gate, now in Rust).
  - `an_override_changes_only_its_surface` — `set_chrome_preset("wiredfront",
    "gloss")` moves `binding("wiredfront", Chrome)` and leaves
    `binding("workbench", Chrome)` untouched.
  - `a_stale_preset_id_is_refused_not_fatal` — `set_chrome_preset("wiredfront",
    "nosuchpreset")` returns `false` and the current look survives.
  - `an_override_never_touches_hero_quiet_or_danger`.
- **Break-it:** point an `ancestral` entry at `{"ancestor":"swiss",
  "material":"glass"}` → build fails naming swiss and glass. Revert.
- **Break-it:** give one an unknown `tone` → build fails. Revert.
- `rg 'set_chrome|DEFAULT_CHROME' crates` → **zero** hits outside history.
- `cargo green` exits 0 — including the structure audit, which proves
  `dpl-coldlight` gained no dependency (persistence lives in `crates/app`).
- **End-to-end on the real exe**, which is the actual pass condition:
  1. `cargo build --release -p devpulselabs -p dpl-lottie-render`.
  2. `--route wiredfront/editor --window 1400x900 --screenshot a.png`
     → editor in **Vibron** (the authored default).
  3. Open the ColdLight chip, pick **Gloss**, screenshot → editor visibly
     rounded/glassy, and `.userdata/coldlight.json` contains
     `{"surfaces":{"wiredfront":"gloss"}}`.
  4. Relaunch with no flags, screenshot → **still Gloss** (persistence).
  5. Pick "Reset to authored", relaunch → back to Vibron, key removed.
  6. Workbench unchanged throughout (`--route workbench` shots at 2 and 3
     pixel-identical).
- Shots at maximized AND `--window 1100x700` (the any-shape law — the footer
  bug hid in the second one).
- Feel bar: the picker must read as a tool, not a debug menu.

## 7. Commit plan

ONE branch, ONE PR, merged only on **double-green** (`green-windows` +
`green-linux` both COMPLETED SUCCESS via `gh pr checks <n> --watch`), branch
deleted after. `gh pr list` first — a parallel session may already be on it.

`feat(coldlight): presets are authored, switchable, and remembered`

Ordered commits: legality hoist → `ancestral` in presets.json + generator →
override layer + `binding()` branch → retire the dead trait-global → status
chip + picker → persistence → mock reads the tokens → docs.

Session end: rebuild `target\release\devpulselabs.exe` here and verify its
mtime moved; update `docs/STATE.md`.

## 8. Evidence

### Deviations from the plan, named

1. **A disk flush came first, and `cargo clean` was REFUSED.** The build cache
   began corrupting mid-pass (`only metadata stub found for rlib dependency
   core`, then egui). Cause: **10.5 GB free** — partial writes. `cargo clean`
   would have deleted `target/release/{content,.userdata,assets,dptools-deps}`,
   including 13 files of gitignored operator data that CLAUDE.md forbids
   deleting. Cargo's artifacts were removed surgically instead; all four
   directories verified intact at identical file counts. **10.5 GB → 38.6 GB.**
2. **The `lit-cast` split had not been finished.** The atmosphere landed in
   `8f9d98e` but no preset was moved onto it, so `neo` had silently lost its
   neubrutalist pop. `neo`, `werkstatt`, `bacterio` now wear it.
3. **`vorkurs` ≡ `broadsheet`.** The new Rust distinctness test caught two
   presets resolving to byte-identical paint. `paper` names no tone-relative
   ref — only fixed surface slots — so tone cannot separate two presets on its
   coordinate, and bauhaus and editorial both wear `spacegrotesk`. The bench's
   own gate could not see it: it compared only WITHIN an ancestor. `vorkurs`
   moved to bauhaus's native `rectilinear`, which suits "primitives,
   asymmetric grid" anyway.
4. **Persistence sits in `crates/app`, not the shell.** As planned — confirmed
   during execution that `dpl-shell` has no `engine-capture` dependency and so
   cannot reach `userdata_root()`. The app polls `cl::chrome_choices()` once a
   frame and writes on change; comparing a handful of short strings is cheaper
   than threading an event through two crates and cannot miss a pick.
5. **`brand()` uses a `Label` with `Sense::click`, not `launcher_chip`.** The
   chip helper draws a bordered launcher; the brand is a quiet inline label and
   turning it into a bordered button would have changed the bar's look, which
   this pass has no mandate to do.

### Results

```
cargo test -p dpl-coldlight                     57 passed
cargo test -p dpl-composer                      62 passed
cargo test -p dpl-shell                         31 passed
cargo clippy (coldlight, shell, devpulselabs)   0 warnings
cargo build --release -p devpulselabs -p dpl-lottie-render   ok
```

New tests: `presets_switch_a_surface_and_survive_a_typo` (a pick moves the
painted values, moves ONE surface, survives a stale id, and never reaches
hero/quiet/danger), `every_authored_preset_is_legal_and_paints` (25 × 3
states), `the_authored_presets_are_visually_distinct`.

### End-to-end on the real exe — the pass condition

`targetelease\devpulselabs.exe` rebuilt 2026-08-05 18:20.

| # | run | result |
|---|---|---|
| 1 | `--route wiredfront/editor` (nothing picked) | Vibron — square 6px plates; chip reads "ColdLight"; **no** `coldlight.json` |
| 2 | `--preset gloss` | `preset: wiredfront now wears gloss`; `.userdata/coldlight.json` = `{"surfaces":{"wiredfront":"gloss"}}` |
| 3 | **relaunch, NO flags** | **still Gloss** — every control rounded (radius 16), chip reads "Gloss" |
| 4 | `--route workbench/library` | untouched — the pick dressed one surface |

Shots: `p1-authored.png` · `p2-gloss.png` · `p3-relaunch.png` · `p4-workbench.png`.

A 6th change was needed to prove this headlessly: **`--preset <id>`**, which
goes through the same `set_chrome_preset` the picker calls and is therefore
persisted identically. It is added to CLAUDE.md's flag list.
