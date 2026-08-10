# The preset controls panel — edit a Lottie like an overlay

Branch: `feat/lottie-controls-panel` off `main`.
Follows: `docs/plans/lottie-preset-controls.md` (the engine, landed #221).

## 1. Context

The operator's condition, 2026-08-07:

> "if you can seriously prove it will be able to be made just as friendly and
> easy to edit the presets as after effects ... ease of change color/font and
> having it live preview and so forth okay."

The ENGINE now does this — `discover` finds a preset's editable text and
colours, `apply` patches them, and the renderer draws the result at any
resolution. What does not exist is a way to do it **without writing Rust**.

This pass is the difference between a capability and a product. Until a
swatch exists, the honest answer to "is it as friendly as AE" is *no*,
regardless of what the engine can do.

## 2. Current state

Ground-truthed in-session 2026-08-08.

### 2.1 The engine, landed and proven

`dpl_lottie::controls` — `discover(&Value) -> Vec<Control>`,
`apply(&Value, &[Control], &[(String, ControlValue)]) -> Result<Value, _>`.
`ControlKind::{Color, Text}`; `ControlValue::Color([u8;4])` in 0–255 (the
repo's array-not-hex shape law) and `::Text(String)`. 13 unit pins.

### 2.2 The inspector pattern to match

`composer::overlay_inspector` (`crates/composer/src/lib.rs:4039`) is the
model: local `mut` copies, `fresh`/`changed` flags, one `Act` emitted at the
end. Its tint row is ALREADY ColdLight tone swatches —
`swatch(ui, Some(tone.color()), active, tone.hue_id())` over Cyan/Lime/
Violet/Amber/Magenta/Coral, with a `none` swatch and a tooltip on each.

A Lottie colour control must reuse that exact row. Two different colour
pickers in one inspector would be the drift the operator has called out
repeatedly.

### 2.3 Preview opens the ORIGINAL file

`crates/composer/src/lib.rs:1214` — on a `Source::Lottie` layer,
`LottiePlayer::open(&path)` opens the artifact as authored. There is no seam
for a modified document, so a patched preview needs a patched FILE.

`Player::open` costs ~24 ms warm (measured), so re-opening on a committed
edit is affordable; re-opening per keystroke is not.

### 2.4 Export renders from the same rel path

`render_parts` (`:1569`) hands each layer's `rel` to the ffmpeg graph. If the
preview patches and the export does not, **what the operator sees is not what
they get** — the exact class of defect this program keeps paying for.

So preview and export patch through ONE function or this pass does not ship.

### 2.5 `dpl-comp`'s ceiling forbids the obvious storage

`Layer` would be the natural home for control values, but `dpl-comp`'s
gate-enforced ceiling is EXACTLY `serde, serde_json` (`architecture.md` §2).
It cannot name `dpl_lottie::ControlValue`.

Therefore values are stored as plain data — `BTreeMap<String,
serde_json::Value>` — and `dpl-lottie` interprets them. The document records
*what the operator chose*; the renderer knows what it means. That is the same
split `Source` already uses: a REF, never a value.

## 3. Target design

### 3.1 The schema addition, additive

```rust
/// Operator-chosen values for this layer's preset controls, keyed by
/// `dpl_lottie::controls::Control::id`. Absent on every comp authored
/// before presets were editable, so `comp.json` round-trips byte-identically
/// and SCHEMA_V does not move.
#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
pub controls: BTreeMap<String, serde_json::Value>,
```

Matches the file's stated law verbatim: *"Every one is ADDITIVE and skipped at
its default."*

### 3.2 ONE patch function, used by both paths

```rust
/// Resolve a Lottie layer to the document that should actually render:
/// the authored file when the layer has no control values, or a patched
/// copy in the scratch cache when it does.
fn resolved_lottie(rel: &str, controls: &BTreeMap<String, Value>) -> PathBuf
```

Keyed by a hash of (rel, values) into `.userdata/cache/lottie/`, so an
unchanged preset re-uses its patched file and the preview does not re-render
the world on every frame. Cache location follows the ratified
`.userdata/cache/` convention (thumbs precedent).

Preview (§2.3) and export (§2.4) both call it. That is the whole guarantee.

### 3.3 The panel

Inside `overlay_inspector`, when the layer is `Source::Lottie`:

- `discover` the artifact once per selection, cached by layer id;
- a `Text` control renders a single-line edit committing on focus-loss or
  Enter — NOT per keystroke, because each commit costs a ~24 ms re-open;
- a `Color` control renders the SAME tone-swatch row as tint (§2.2), plus a
  `none` swatch meaning "the preset's own colour";
- each row carries a tooltip, per the experience law (controls get tooltips).

A preset with no discoverable controls says so plainly rather than showing an
empty box.

### 3.4 What the operator sees

Select a lower-third → its text and colours appear as controls → type a name,
tap a hue → the preview updates → export renders exactly that. No Rust, no
After Effects, and the same preset re-used against the next thing.

## 4. Changes, in execution order

1. `crates/comp/src/lib.rs` — the `controls` field (§3.1) + a round-trip test
   proving a comp without it is byte-identical.
2. `crates/composer/src/lib.rs` — `resolved_lottie` (§3.2) + its cache.
3. …preview (`:1214`) calls it.
4. …`render_parts` (`:1569`) calls it.
5. …`Act::SetControl { layer, id, value }` + the reducer.
6. …the panel in `overlay_inspector` (§3.3).
7. Smoke: extend `xtask smoke` with a PATCHED render — assert the patched
   document differs in pixels from the authored one.

## 5. Boundaries

🚫 **Never touch**

- The authored artifact. Patches go to the scratch cache; **sources are never
  mutated** — this pass reads the operator's Lottie and writes elsewhere.
- `dpl-comp`'s dependency ceiling (§2.5).
- The tint row's existing behaviour.
- `/content` — operator data.

⚠️ **Ask first**

- Any control kind beyond `Color`/`Text` (font family, numbers, images).
- Persisting discovered controls into the comp (this pass discovers at
  selection time and stores only VALUES).

**Out of scope:** font-family swapping · `.mogrt` controls · keyframing a
control · the effects gap.

## 6. Verification

1. `cargo clean -p dpl-comp -p dpl-composer`, then cold clippy, then
   `cargo green` (which now runs smoke).
2. Round-trip pin: a `comp.json` without `controls` serialises byte-identically.
3. `resolved_lottie` pins: no values → the authored path; values → a patched
   path whose document differs at the expected pointer; identical values →
   the SAME cached path (no churn).
4. Smoke gains a patched-render check (§4.7) — the patched frame's pixels must
   differ from the authored frame's. An exit code proves nothing here.
5. **The experience proof, at a real window size:** open the composer on a
   Lottie layer, change text and hue, screenshot at 1300×800, and LOOK at it.
   Then export and confirm the rendered frame matches the preview.

## 7. Commit plan

Branch `feat/lottie-controls-panel`. `gh pr list` first; merge ONLY on
double-green.

```
feat(composer): presets you edit with a swatch, not a function call
```

## 8. Evidence

Filled during execution. §6.5 is the measure — the operator asked for
"friendly", and friendly is not provable from a unit test.

**Recorded before any code:** §2.4 is the one that can silently ruin this. If
preview patches and export does not, the operator edits a preset, sees it
change, renders it, and gets the old one. Both paths go through §3.2 or this
pass does not ship.
