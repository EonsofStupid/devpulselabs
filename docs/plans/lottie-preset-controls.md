# Lottie preset controls — text that renders, and colour you can change without After Effects

Branch: `feat/lottie-preset-controls` off `main`.

## 1. Context

The operator, 2026-08-07, on why they kept pushing to patch Adobe's licence:

> "if you can seriously prove it will be able to be made just as friendly and
> easy to edit the presets as after effects and it will truly present the
> exactly quality results and ease of change color/font and having it live
> preview and so forth okay."

Their constraint is real: the work laptop has no internet, and Adobe is
withdrawing the offline licence that made AE usable there. The lawful answer
is a preset system that needs no Adobe at all — and the operator has agreed to
that **conditional on it actually working**, which is the right condition.

This pass makes the condition testable rather than promised.

## 2. Current state

Measured in-session 2026-08-07 by rendering real fixtures, not by reading
docs. Three findings, in order of how badly they bite.

### 2.1 Lottie text renders as NOTHING, silently

A Bodymovin fixture with one text layer (`ty: 5`, a `fonts.list` entry, and
`"t": "HELLO"`) renders a **fully blank frame**. Exit code 0. PNG written.
Nothing on it.

Cause, confirmed: `crates/lottie-render/src/main.rs` contains the string
"font" **zero times**, while the generated `thorvg-sys` bindings expose
`tvg_font_load`, `tvg_font_load_data` and `tvg_font_unload`. ThorVG will not
render text for a family it has never been given, and it does not complain —
it draws nothing.

`crates/lottie/src/lib.rs`'s own header claims the scope law is *"shapes,
keyframes, masks, mattes, gradients, and text"*. Text is aspirational.

**This is the load-bearing finding.** A preset system whose headline feature
is "change the text" would have been built on a renderer that draws no text,
and every proof would have been a blank frame that exited 0.

### 2.2 Frame 0 was unrenderable, so `export` produced nothing

Fixed earlier today (`fix/lottie-frame-zero`): `tvg_animation_set_frame`
returns `INSUFFICIENT_CONDITION` when the frame is already current, a fresh
animation sits on frame 0, and `ok()` treated that no-op as fatal. `export`
— the PNG-sequence path into ffmpeg — had never emitted a single file.

### 2.3 What DOES work, proven by artifacts

- **Animation.** t=0 renders the rect axis-aligned; t=1s renders it at 45°.
  Keyframes, easing, shape animation: real.
- **Resolution independence.** One 512×512-authored source rendered clean at
  256² and 3840² — a 15× range, re-rendered rather than upscaled.
- **The live-preview transport.** `Player` (helper `serve` mode) streams a
  frame at any `t` over a persistent process — the scrub path already exists.
- **Batch export.** `export_sequence` → PNG sequence for ffmpeg (post-2.2).

### 2.4 What is NOT the gap

`dpl-comp` has no keyframes — `Layer` carries only static `x`/`y`/`scale`/
`tint`/`z`. That is **correct and not this pass's problem**: `dpl-comp` is the
stitcher that places clips on a timeline; a preset's motion lives inside the
Lottie, which already animates. Adding keyframes to `dpl-comp` would also
force the export path off its static ffmpeg filtergraph (`composer:1569`
builds one fixed `(x, y, scale)` per overlay) — a much larger change, and not
one presets need.

### 2.5 The app already owns a font vocabulary

`crates/coldlight/assets/fonts/` vendors `Orbitron`, `Michroma`, `Audiowide`,
`Exo2`, `ChakraPetch`, `Monoton`, `Oxanium`, `JetBrainsMono` (SIL OFL,
licences alongside), declared in `tokens/fonts.json` as `{id, label, class,
file, blurb}`.

So "change the font" is a **token pick**, and the fonts ship with the app —
no system-font dependency, and identical output on the offline laptop.

### 2.6 Both render entry points take a PATH

`Player::open(lottie: &Path, w, h)` and `export_sequence(lottie: &Path, …)`.
Parameterisation can therefore patch JSON to a scratch file and point the
existing API at it — **no helper-protocol change**.

## 3. Target design

### 3.1 The helper learns fonts (§2.1's cure)

`dpl-lottie-render` gains `--font <path>` (repeatable) and `--fonts <dir>`,
calling `tvg_font_load` before rendering. `dpl-lottie` passes ColdLight's
vendored font directory by default, resolved beside the exe exactly as
`helper_path()` already resolves the helper.

A font that fails to load is reported, never swallowed — the whole point of
§2.1 is that silence is the defect.

### 3.2 Controls — a declaration, not a bespoke UI

```rust
pub enum ControlKind { Color, Text }

pub struct Control {
    pub id: String,        // stable key the UI and the values map share
    pub label: String,     // what the operator reads
    pub kind: ControlKind,
    pub pointer: String,   // RFC-6901 JSON pointer into the Lottie
}
```

Two pure functions, both testable without a process:

- `discover(&Value) -> Vec<Control>` — walk the document and surface what is
  editable: shape fills (`ty:"fl"` → `/c/k`), text content
  (`ty:5` → `/t/d/k/0/s/t`) and text colour (`…/s/fc`). An un-annotated
  Lottie from any source therefore gets controls **without** being re-authored.
- `apply(&Value, &[(id, ControlValue)]) -> Value` — patch by pointer.

This is deliberately the same shape as `.mogrt`'s `clientControls`
(`composer/src/mogrt.rs`), which is the format the operator already knows —
except ours works, because Lottie JSON is readable and `project.aegraphic`
is not.

### 3.3 Colour binds to the token pool

A `Color` control accepts a ColdLight hue token id, resolved through
`hues.json`, as well as a raw RGBA. Changing a preset's colour is then the
same act as changing anything else in the app's design system — which is the
part that is genuinely *friendlier* than AE, where each template must be
wired for Essential Graphics by hand.

### 3.4 Render is unchanged

`patched = apply(read(src), values)` → write to a scratch path → hand to the
existing `Player` / `export_sequence`. No new render path, no protocol change,
and the preview and the export consume the identical patched document — so
what the operator scrubs is what they get.

## 4. Changes, in execution order

1. `crates/lottie-render/src/main.rs` — `--font`/`--fonts` args + `tvg_font_load`,
   loud on failure.
2. Prove text renders: same fixture as §2.1, now legible. Look at the PNG.
3. `crates/lottie/src/controls.rs` (new module) — `Control`, `ControlKind`,
   `ControlValue`, `discover`, `apply`. Pure; no process.
4. `crates/lottie/src/lib.rs` — pass the font directory from `Player::open`
   and `export_sequence`; `pub mod controls;`.
5. Unit pins (§6.2).
6. The proof drill: one preset, three different colour+text values, rendered
   at 512² and 3840².

## 5. Boundaries

🚫 **Never touch**

- `dpl-comp` keyframes (§2.4) — a separate, larger pass, and not what presets
  need.
- The ffmpeg filtergraph in `composer::render_parts`. Untouched by design.
- Adobe licence validation. Declined; not revisited.
- `/content` — operator data. Fixtures live in the scratchpad.

⚠️ **Ask first**

- Any control kind beyond `Color`/`Text` (font-family swapping, numbers,
  images) — each is a new editing surface, not a mapping.
- Writing controls back into the operator's Lottie files. This pass reads
  them and patches to scratch; mutating a source is the sources-are-never-
  mutated law.

**Out of scope:** the controls PANEL (its own pass, once the engine is proven)
· font-family swapping · `.mogrt` client controls · ffmpeg encode of the
sequence.

## 6. Verification

1. `cargo clean -p dpl-lottie -p dpl-lottie-render`, then
   `cargo clippy --workspace --all-targets -- -D warnings`, then `cargo green`.
2. Unit pins:
   - `discover_finds_a_shape_fill_and_a_text_layer`
   - `apply_patches_only_the_named_pointer`
   - `an_unknown_control_id_is_refused_not_ignored`
   - `a_color_token_resolves_through_the_hue_pool`
   - `applying_no_values_returns_the_document_unchanged`
3. **The proof, and the only one that counts: LOOK AT THE FRAMES.** §2.1
   passed its exit code while rendering nothing, so no step here is complete
   on an exit code. Every render is opened and read.
4. One preset → three colour+text variants → rendered at 512² and 3840²,
   inspected, and visibly different from each other in the way the values say.

## 7. Commit plan

Branch `feat/lottie-preset-controls`. `gh pr list` first; merge ONLY on
double-green.

```
feat(lottie): presets you can retext and recolour, with no After Effects
```

## 8. Evidence

Filled during execution.

**Recorded before any code:** the reason this pass exists in this shape is
§2.1 — text rendered blank at exit code 0. Every capability claim below is
therefore proven by opening the image, never by the process succeeding. If
that discipline had been applied to the crate header's scope law, "Lottie
carries … text" would have been caught as untrue much earlier.
