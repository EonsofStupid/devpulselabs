# Pass — the layer switches become real: visibility, lock, motion blur,
# parenting, and 3D position

## 1. Context

**The system.** `crates/composer` is the WiredFRONT editor. Its timeline
outline (`crates/composer/src/lib.rs:3804` `outline`) paints one row per
layer. `docs/design/mocks/wiredfront-editor-mock.html` region ⑦ shows
that outline with AE's switch columns — `◇` 3D, `⨀` motion blur, `＠`
Parent — plus AE's standard eye/lock, and region ② shows `⨀ m.blur` and
`◇ 3D` as **comp-level** switches in the tool strip (the mock's tooltip
is explicit: *"Motion blur — comp switch"*, *"3D — per-layer switch
lives in the timeline"*).

**None of it has backing state.** `dpl_comp::Layer`
(`crates/comp/src/lib.rs:45-79`) carries `id, source, start, in_pt,
out_pt, x, y, scale, tint, blend, matte, lut` — no visibility, no lock,
no parent, no depth, no motion blur.

**Operator directive (2026-08-04, verbatim):**

> "is it possible to have motion blue and the others added in my editor ?
> if you need to plan it out first what is the problem with doing the
> work right and methodically?"

and

> "so get them built plan this out research i do not understand why you
> are acting like real work is shit i am opposed too"

The prior pass proposed skipping these columns because the state was
missing. That was the wrong call: missing state is a reason to author it,
not to omit the feature. This plan authors it.

**What makes this tractable** (all read in-session 2026-08-04, §2):

1. `dpl-comp`'s own header states *"the schema grows additively"*, and
   every optional field already uses
   `#[serde(default, skip_serializing_if = …)]`. New fields need **no
   `SCHEMA_V` bump and no migration**.
2. The render already threads per-layer properties into the ffmpeg graph
   (`x, y, scale, tint, blend, matte, lut`). Adding one more is a proven
   pattern, not new ground.
3. `x`/`y`/`scale` are **exactly** what a pinhole camera projection
   produces. 3D *position* therefore needs **zero engine change** — it
   resolves into the transform both preview and render already honour.
4. The "approximate in preview, exact in export" convention already
   exists as painted badges — motion blur reuses it rather than
   inventing a story.

## 2. Current state (every claim read in-session 2026-08-04)

### 2.1 The document — `crates/comp/src/lib.rs` (714 lines)

Header, verbatim: *"R1a is the FOUNDATION. Transform keyframes, the
effect registry, and the `GenBackend` seam land in later R1 passes; **the
schema grows additively**."*

```rust
pub const SCHEMA_V: u32 = 1;                       // :16

pub struct Composition { pub v: u32, pub fps: u32, pub w: u32,
                         pub h: u32, pub tracks: Vec<Track> }   // :23
pub struct Track { pub id: String, pub layers: Vec<Layer> }     // :34

pub struct Layer {                                              // :45
    pub id: String, pub source: Source,
    pub start: f32, pub in_pt: f32, pub out_pt: f32,
    #[serde(default, skip_serializing_if = "is_zero")] pub x: f32,
    #[serde(default, skip_serializing_if = "is_zero")] pub y: f32,
    #[serde(default = "one", skip_serializing_if = "is_one")] pub scale: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub tint: Option<[u8;4]>,
    #[serde(default, skip_serializing_if = "Blend::is_normal")] pub blend: Blend,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub matte: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub lut: Option<String>,
}
```

`load` refuses a newer `v` honestly (`:245-248`,
`CompError::Version`); the round-trip, pretty-LF, and newer-refusal tests
live at `:464, :478, :490`. **`scale`'s `default = "one"` / `is_one` pair
is the exact precedent for a defaulted-true `visible`.**

### 2.2 The render — `crates/composer/src/lib.rs:1349` `render_parts`

Walks `self.layers()` (track 0 → `CompBase::Clip { clip, lut }`, base
timing accumulated in `base_end`), then `self.comp.tracks.iter().skip(1)`
(overlay tracks → `OverlayClip { input, in_pt, out_pt, at, x, y, scale,
tint, hold, blend, matte, lut }`). A short comp is padded with
`CompBase::Black { dur }` (`:1434`) — **the established way to occupy
time without picture**, which is how a hidden base cut must behave.

### 2.3 The ffmpeg graph — `crates/engine-media/src/lib.rs:610-683`

Per overlay, in order:

```
[k:v] trim=start:end , setpts=PTS-STARTPTS+at/TB ,
      scale=tw:th:force_original_aspect_ratio=decrease
      [, colorchannelmixer=…]        <- tint      (:628)
      [, lut3d=…]                    <- lut       (:637)
      → pad+blend (screen/add)  |  overlay=x=(W-w)/2±x:y=(H-h)/2±y  (:677)
```

`tw`/`th` derive from `o.scale` (`:614-615`); position from `o.x`/`o.y`
(`:678`). **A new per-layer filter slots in beside `tint`/`lut` — same
shape, same place.**

### 2.4 The preview — `crates/composer/src/lib.rs`

- `overlay_layers_at(t)` (`:613`) — overlay layers live at `t`.
- `active_layer_at(layers, t)` (`:961`) — the base layer under the
  playhead; drives `tick_playback` (`:970`).
- `paint_overlays` (`:805`) — `center = frame.center() + vec2(layer.x *
  px, layer.y * px)`; `bx = Rect::from_center_size(center, frame.size() *
  layer.scale)` (`:815-816`). **The two knobs a projection feeds.**
- **The honesty-badge precedent** (`:838-847`): `"screen · export-exact"`,
  `"add · export-exact"`, `"grades in export"` — painted when the viewer
  cannot reproduce what the render will do. Motion blur joins this set.
- `select` (`:955`) sets `self.selected`; `Act::Remove` fires from
  `timeline_panel` (`:3778`) and from Delete/Backspace (`:4926`).

### 2.5 The outline — `crates/composer/src/lib.rs:3804`

Per `Row::Layer(i)`: index text at `x+6`, colour chip at `x+26`, twirl
`+`/`-` at `x+40`, name at `x+56`, `mmss(layer.start)` right-aligned at
`max.x-6`. Hit zones: twirl = left 52 px, row = the rest.
`OUTLINE_W` bounds the column; `RULER_H` offsets the first row.

**There are no switch columns and no room reserved for them** — this pass
adds both.

## 3. Target design

### 3.1 The schema (additive, no version bump)

`crates/comp/src/lib.rs`, on `Layer`:

```rust
/// Hidden layers render as if absent (base cuts hold their time as
/// black, so hiding never re-times the comp).
#[serde(default = "yes", skip_serializing_if = "is_yes")] pub visible: bool,
/// Editor-side only: a locked layer cannot be selected, dragged,
/// trimmed, or removed. Never reaches the render.
#[serde(default, skip_serializing_if = "is_no")] pub locked: bool,
/// Per-layer motion blur; gated by the comp master switch.
#[serde(default, skip_serializing_if = "is_no")] pub motion_blur: bool,
/// Another layer's id whose transform this one rides.
#[serde(default, skip_serializing_if = "Option::is_none")] pub parent: Option<String>,
/// Depth in comp px. 0 = the camera's focal plane (unchanged).
/// Positive = further away (smaller); negative = nearer (larger).
#[serde(default, skip_serializing_if = "is_zero")] pub z: f32,
```

on `Composition`:

```rust
/// The comp motion-blur master (the mock's tool-strip ⨀). A layer
/// blurs only when BOTH this and its own switch are on — AE's rule.
#[serde(default, skip_serializing_if = "is_no")] pub motion_blur: bool,
/// Shutter angle in degrees (AE's default is 180). Drives how many
/// frames the render averages.
#[serde(default = "shutter_default", skip_serializing_if = "is_shutter_default")]
pub shutter: u32,
/// Camera focal length in comp px. `None` = no camera: `z` is inert
/// and the comp is flat, exactly as today.
#[serde(default, skip_serializing_if = "Option::is_none")] pub focal: Option<f32>,
```

`SCHEMA_V` **stays 1**. Every field is defaulted and skipped when
default, so an existing `comp.json` loads unchanged and a comp using no
new feature re-serializes **byte-identically** — proven by test (§6).

### 3.2 Parenting — a pure resolver in `dpl-comp`

```rust
pub fn resolved_transform(layers: &[Layer], id: &str) -> Option<Transform>
pub struct Transform { pub x: f32, pub y: f32, pub scale: f32 }
```

Child rides parent: `scale = parent.scale * child.scale`;
`x = parent.x + child.x * parent.scale` (same for `y`) — the child's
offset is expressed in the parent's scaled space, which is what makes a
parented layer track its parent when the parent scales.

**Cycle safety is structural, not hopeful**: the walk carries a visited
set and bails at `PARENT_MAX_DEPTH = 16`, returning the chain resolved so
far. A cycle can therefore never hang the paint thread or the render.
`set_parent` additionally refuses an edge that would close a cycle, so
the UI cannot author one in the first place. Both are tested.

This lives in `dpl-comp` (pure data, no egui, no ffmpeg — the crate's
law) so preview and render share **one** implementation and it is unit
testable without a window.

### 3.3 3D position — a pinhole projection, zero engine change

```rust
pub fn project(t: Transform, z: f32, focal: Option<f32>) -> Transform
```

With `focal = f` and depth `z`: `k = f / (f + z)`, clamped to
`k > 0` (a layer at or behind the camera is clamped to a minimum, never
inverted or NaN). Then `scale *= k`, `x *= k`, `y *= k`.

- `focal: None` → `k = 1`: **byte-identical behaviour to today.**
- `z = 0` → `k = 1`: a 3D-enabled layer on the focal plane is unchanged.
- `z > 0` → smaller and pulled toward centre — real perspective.

Because the output is `(x, y, scale)`, it feeds `paint_overlays`
(`:815-816`) and `OverlayClip` (`:1399-1412`) **unchanged**. No ffmpeg
change, no new filter, no compositor work.

**Named limit, stated in the UI, not hidden**: this is 3D *position*, not
3D *rotation*. Rotation needs ffmpeg's `perspective` filter and a corner
solve — a later pass. The switch's tooltip says so in plain words.

### 3.4 Motion blur — `tmix` in the render, badge in the preview

Frames averaged, from shutter angle:
`n = (shutter / 90).round().clamp(2, 8)` → 180° gives `n = 2`, 360° gives
`n = 4`, with a hard ceiling so a silly value cannot explode render time.
Weights are a symmetric triangle (`1 2 1` for `n=3`), normalised by
ffmpeg. The filter lands beside `tint`/`lut` in the chain (§2.3):

```
, tmix=frames=N:weights='…'
```

**The honest limit, stated plainly in the tooltip and the plan.** AE's
motion blur samples sub-frames of an *animated* transform. This comp has
no keyframes yet (`dpl-comp` header: *"Transform keyframes … land in
later R1 passes"*), so nothing animates and `tmix` blurs **the source
footage's own motion**. That is a real, useful effect and the correct
meaning of the switch today; when keyframes land, the same switch gains
its full AE meaning with no schema change. The tooltip says "smears the
movement already in the clip" rather than implying transform blur.

Preview paints `"motion blur · export-exact"` via the existing badge
mechanism (`:838-847`) — no frame blending on the paint thread.

### 3.5 Visibility, in both engines

- **Overlay layer hidden** → omitted from `overlay_layers_at` and from
  `render_parts`' overlay walk. Simply absent.
- **Base layer hidden** → becomes `CompBase::Black { dur: span }` in
  `render_parts`, and is skipped by `active_layer_at`. **Timing is
  preserved**; hiding a cut never re-times everything after it. This
  reuses the padding pattern already at `:1434`.

### 3.6 Lock

Editor-side only; never reaches `render_parts`. Guards `select`
(`:955`), the lane drag/trim handlers, and both remove paths (`:3778`
button, `:4926` key). A locked row paints its lock icon lit and its name
in `ink::MUTED`.

### 3.7 The outline columns (mock ⑦)

Right-aligned before the existing time column, each a 14 px icon with a
hit rect and `cl::tip`:

`eye` (visible/hidden) · `lock` · `box` (3D) · `aperture` (motion blur) ·
`link-2` (parent, opens a picker of eligible layers)

`OUTLINE_W` grows to fit; at narrow widths the columns are the first
thing to drop to icon-only (RR2 fit law), never overlapping the name —
the name's clip rect shrinks to the columns' left edge.

Tool strip `m.blur` and `3D` stop being inert affordances: `m.blur`
toggles `Composition.motion_blur`, `3D` toggles whether the comp has a
camera (`focal: Some(DEFAULT_FOCAL)` / `None`).

## 4. Changes, in execution order

**Prerequisite:** the icon pass (`docs/plans/wiredfront-editor-residency.md`)
merges first — it authors `layer-visible`, `layer-hidden`, `layer-lock`,
`tool-3d`, `tool-mblur`, `layer-parent` … Wait: that pass deliberately
dropped the switch ids. **This pass re-adds them** to `tokens/icons.json`:
`layer-visible`→`eye`, `layer-hidden`→`eye-off`, `layer-lock`→`lock`,
`layer-parent`→`link-2` (all four verified present in the pool
2026-08-04). `tool-3d`→`box` and `tool-mblur`→`aperture` already land in
the icon pass.

### S1 — the document (`crates/comp`)

1. `crates/comp/src/lib.rs` — add the five `Layer` fields and three
   `Composition` fields of §3.1, with the `yes`/`is_yes`,
   `is_no`, `shutter_default`/`is_shutter_default` helpers beside the
   existing `one`/`is_one`/`is_zero`.
2. Same file — add `Transform`, `resolved_transform`, `set_parent`
   (cycle-refusing), `project`, and `PARENT_MAX_DEPTH`.
3. Tests: byte-identical re-serialization of a comp using no new field;
   an existing `comp.json` without the fields loads with correct
   defaults (`visible = true`); parent chain resolves; a cycle is refused
   by `set_parent` AND survived by `resolved_transform`; `project` with
   `focal: None` is identity; `z = 0` is identity; `z > 0` shrinks.

### S2 — the render

4. `crates/engine-media/src/lib.rs` — `OverlayClip` gains
   `motion_blur_frames: Option<u32>`; emit
   `,tmix=frames=N:weights='…'` after the `lut3d` step (`:639`).
5. Same file — the base/stitch path (`:600`) gains the same for base
   cuts when the comp master is on.
6. `crates/composer/src/lib.rs` `render_parts` (`:1349`) — skip hidden
   overlays; convert hidden base clips to `CompBase::Black`; resolve
   parent then project `z` into `x/y/scale` before filling `OverlayClip`;
   pass `motion_blur_frames` when comp master ∧ layer switch.

### S3 — the preview

7. `overlay_layers_at` (`:613`) — filter `visible`.
8. `active_layer_at` (`:961`) — skip hidden base layers.
9. `paint_overlays` (`:805`) — resolve parent + project `z` for the
   painted `center`/`bx`; add `"motion blur · export-exact"` to the badge
   match (`:838`).

### S4 — the editor surface

10. `outline` (`:3804`) — the five switch columns of §3.7 with hit rects,
    icons, and tooltips; `OUTLINE_W` widened; the name clip rect
    tightened to the columns' left edge.
11. New `Act` variants — `ToggleVisible(String)`, `ToggleLock(String)`,
    `ToggleLayerBlur(String)`, `Toggle3d(String)`, `SetParent(String,
    Option<String>)` — each `checkpoint()`-ing for undo, exactly as
    `Act::Remove` does.
12. Lock guards in `select` (`:955`), the lane drag/trim handlers, and
    both remove paths (`:3778`, `:4926`).
13. `tool_strip` (`:2964`) — `m.blur` and `3D` become live comp toggles
    with `TraitState::Active` when on; their "ratified later phase"
    blurbs are replaced with what they now actually do.

### S5 — proof

14. `cargo green`.
15. Drill: build a two-layer comp, hide one, render, confirm the output
    holds its timing and the hidden layer is absent; parent one to the
    other, move the parent, confirm the child follows; enable comp +
    layer motion blur, render, confirm `tmix` in the emitted args and a
    visibly smeared output.
16. Proof shots `--route wiredfront/editor` at maximized, `1100x700`,
    `1600x900` — switch columns legible and non-overlapping at all three.

## 5. Boundaries

**🚫 Never touch**
- `/content` — operator data.
- `SCHEMA_V` — this pass is additive by construction; if a change seems
  to need a bump, the design is wrong and the plan is re-opened.
- The base track's tint/blend/matte semantics (`render_parts` comments
  are explicit that base cuts ignore overlay-only properties).
- Existing `comp.json` files on disk — they must load and round-trip
  untouched; that is a §6 test, not an aspiration.

**⚠️ Ask first**
- Any change to `Blend`, `Source`, or track semantics.
- Raising `PARENT_MAX_DEPTH` or the motion-blur frame ceiling.
- Making the preview actually blend frames for motion blur (cost on the
  paint thread — the badge is the ratified answer).

**Out of scope, named so it is not silently dropped**
- **3D rotation** — needs ffmpeg `perspective` + a corner solve. §3.3
  ships 3D *position*; the tooltip states the limit.
- **The tracker (⌖)** — motion tracking is computer vision; per ADR-009
  that is a subprocess helper, its own project. It stays an honest
  later-phase affordance in the tool strip.
- **Transform keyframes** — the later R1 pass `dpl-comp` already names.
  Motion blur gains its full AE meaning then, with no schema change.
- **A camera LAYER** (the mock's "Camera 1" row) — this pass gives the
  comp a focal length, not a movable camera object.

## 6. Verification

- `cargo test -p dpl-comp` — all of S1's tests; **critically**, the
  byte-identical re-serialization test and the "old comp.json loads with
  `visible = true`" test.
- `cargo test -p dpl-engine-media` — the emitted arg vector contains
  `tmix=frames=` exactly when blur is on, and does **not** when off.
- `cargo test -p dpl-composer` — hidden base becomes `CompBase::Black` of
  the right duration; hidden overlay is absent; parented child's resolved
  transform matches hand-computed values; `project` identity when the
  comp has no camera.
- `cargo green` exits 0.
- **Break-it**: hand-author a `comp.json` with `"parent"` cycling two
  layers → the app paints and renders without hanging (the depth cap),
  and `set_parent` refuses to create one through the UI.
- **Break-it**: `"shutter": 3600` → frames clamp to 8, render completes.
- Live drills of §S5.15 on the real exe, output inspected.
- Proof shots of §S5.16 beside mock region ⑦.
- **Feel bar**: the outline reads like AE's — switches legible, aligned,
  and obviously tappable — at every proof size.

## 7. Commit plan

ONE PR — `branch → gh pr create → double-green → merge`, branch deleted
after. `docs/STATE.md` updated; `target\release\devpulselabs.exe` rebuilt
here with its mtime verified at session end.

`feat(comp): the layer switches become real — visibility, lock, motion
blur, parenting, and 3D position`

Ordered commits inside it, so the diff reviews in the order it was built:
schema+resolver → render → preview → outline UI → proofs.

## 8. Evidence

*(filled during execution — real command output, deviations named
explicitly)*
