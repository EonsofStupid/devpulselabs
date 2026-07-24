# R1a — the composition document + doctor + Composer panes shell

Pass R1a of the WiredFRONT editor (ledger #114). Parent ADR:
`r1-editor-adr-proposal.md` (PARTIALLY RATIFIED 2026-07-24, four forks
resolved). Stack ADR: `rendering-stack.md` RS4. Ground-truthed in-session
2026-07-24 against the real files (every excerpt below traces to a read
this session).

R1a is the FOUNDATION: the comp is a folder document (`comp.json`) with a
doctor, exactly like a slide (`dpl-doc`); the Composer is a new WiredFRONT
subroute — a four-pane editor whose timeline is a CLIP LIST you build by
appending source clips from the artifact index, with single-source
preview playback. NO stitch render, NO generation, NO matting, NO
compositor yet — those are R1b–R1f.

---

## 1. Context

The operator's vision (R1, 2026-07-23, restated 2026-07-24): *"After
Effects but chat-route-aware"* — stitch series of 8–15s AI clips,
post-process, apply the operator's custom elements, background removal,
generate/edit images+video through the chat mesh; output a video usable
as one slide or a presentation, prepared in WiredFRONT and then layered
into a show in Workbench (the two-editors law). R1a lays the keel: the
data document every later pass reads and writes, and the editor shell
every later pane hangs in. An agent can only safely edit a comp if a comp
is validated JSON — so the document + doctor come first, before any
pixel.

## 2. Current state (file-by-file, from reads this session)

**`crates/doc/src/lib.rs` — the pattern R1a mirrors.** Pure data, no egui.
`pub const SCHEMA_V: u32 = 1;` A document is a folder holding a colocated
JSON file; refs never values. The shapes that R1a copies verbatim in
spirit:

- Crash-safe writer (tmp → `.bak` swap, pretty JSON + trailing LF):
  `fn save_json<T: Serialize>(dir, file, value) -> Result<(), DocError>`
  (lines 140–157). `load` refuses a newer `v` honestly (`DocError::Version`).
- The doctor returns findings, never crashes: `pub struct Issue { path,
  message }` (lines 96–100); `pub fn validate(&Slide) -> Vec<Issue>`
  (line 224) names every unknown ref; empty = clean.
- The folder contract: `const SLIDE_RESERVED: &[&str] = &["slide.json",
  "ink.json", "notes.md", "script.mdx", "assets"];` +
  `check_folder_contract(...)` flags strays and `.tmp`/`.bak` transients
  (lines 400–439).
- `pub fn find_upward(rel: &str) -> Option<PathBuf>` (line 306) walks exe
  dir then CWD upward — deliberately reimplemented so the doc layer
  depends on no engine crate for a path walk (the repo's small-utility
  philosophy R1a follows: replicate, don't cross-depend).
- Tests mirror this shape (round-trip, newer-version-refused,
  unknown-refs-named, folder-contract-names-strays).

**`crates/video/src/clip.rs` — the finite clip player R1a reuses for
preview.** `pub struct ClipPlayer` with `open(PathBuf)`, `toggle()`,
`seek(f32)`, `position() -> f32`, `duration: Option<f32>`,
`texture(&Context) -> Option<TextureId>`, `playing()/ended()` (lines
86–207). `pub fn clip_duration(&Path) -> Option<f32>` via `ffmpeg -i`
banner parse (line 57). ffmpeg is a SUBPROCESS (ADR-009). Decode size
`DECODE_W=960 × DECODE_H=540` from `crates/video/src/lib.rs` lines 24–25;
`pub fn cover_uv(tex_a, rect_a) -> Rect` (line 216) for object-fit:cover;
`pub fn locate_asset(rel) -> Option<PathBuf>` (line 258).

**`crates/coldlight/src/lib.rs` — the paint vocabulary (the gate).**
`pub fn tip(resp: Response, text: &str) -> Response` (line 128) — the
tooltip law. `pub fn treated(tone, base, stack, state) -> Treated` (line
469); `pub fn brutal(tone: Tone) -> Treated` (line 490, actionable
neubrutalist); `pub fn clay() -> Treated` (line 496, passive plane). A
`Treated` paints via `.show(ui, |ui| …) -> InnerResponse<R>` (line 511).
This is the ONLY surface-painting path R1a's UI may use — no ad-hoc
`Color32::from_rgb`, no magic px (coldlight-system foundation law).

**`crates/hubs/wiredfront/src/lib.rs` — the host + mount seam.** The hub
is a `Hub` over a `static SPEC: HubSpec` whose `subroutes: &[SubRoute{
id, label, blurb }]` currently lists OVERVIEW, VIZIWIZ, GALLERY, SESSIONS
(lines 17–50). State lives on `pub struct WiredfrontHub { fabric, gallery,
sessions, replay }` (lines 104–113). Dispatch is by `route.sub` in
`fn ui(&mut self, ui, route: Route)` (lines 1014–1034): a `route.sub ==
GALLERY` branch calls `self.gallery_surface(ui)`, else the placeholder
surface. GalleryView already shows the reuse pattern R1a copies:
`dpl_index::Index::open_default().and_then(|ix| ix.recent(500, None))`
(line 72) to list artifacts, and `dpl_engine_capture::content_root()`
(line 79) for the content root. It already depends on `dpl_video::clip::
ClipPlayer` (line 126).

**`Cargo.toml` — workspace conventions.** `members` lists each crate
path; `[workspace.dependencies]` declares `egui = "0.35"`, `serde`,
`serde_json`, and every internal crate as `dpl-* = { path = ... }` (lines
64–124). New crates join `members` and get a `dpl-* = { path }` line. NO
`egui_tiles`/`vello` dep exists yet (grep confirmed).

## 3. Target design

Two new crates + a mount, all `dpl-*`:

**`crates/comp` (`dpl-comp`) — the composition document. Pure data, no
egui.** Mirrors `dpl-doc` exactly.

```rust
pub const SCHEMA_V: u32 = 1;

pub struct Composition { pub v: u32, pub fps: u32, pub w: u32, pub h: u32,
                         pub tracks: Vec<Track> }
pub struct Track { pub id: String, pub layers: Vec<Layer> }
pub struct Layer {
    pub id: String,
    pub source: Source,      // ref never value
    pub start: f32,          // placement on the track timeline (s)
    pub in_pt: f32,          // trim in within the source (s)
    pub out_pt: f32,         // trim out within the source (s)
}
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Source {
    Artifact { rel: String },   // an artifact rel_path under the content root
    Element  { id: String },    // a library element id (R1c ingests these)
}
```

- `load(dir)/save(dir, &Composition)` on `comp.json`, crash-safe
  tmp→bak→swap + pretty+LF, newer-`v`-refused — the `dpl-doc` writer
  replicated (not a cross-dep; the repo's stated philosophy).
- `validate(&Composition) -> Vec<Issue>` (same `Issue { path, message }`
  shape): `fps>0`, `w>0 && h>0`, unique track ids, unique layer ids
  across the comp, `in_pt >= 0`, `out_pt > in_pt`, non-empty `source`
  ref. Element/artifact EXISTENCE is a SOFT concern deferred to R1c (the
  index/ingest pass) — R1a validates shape, names nothing it cannot yet
  resolve.
- `find_upward` replicated; `const COMP_RESERVED: &[&str] = &["comp.json",
  "assets"]` + `check_folder_contract` for the comp folder.
- `Composition::duration()` = max over layers of `start + (out_pt -
  in_pt)`; `new(fps,w,h)` a 1080p30 empty default.
- Full test suite mirroring `dpl-doc`: round-trip, version-refused,
  unknown/bad refs named, folder contract names strays, refs-never-values
  (no `#` hex, style is structural).

**`crates/composer` (`dpl-composer`) — the WiredFRONT editor UI.** A
four-pane shell, ColdLight-painted, every control tipped. Owns the
editing session state; writes through `dpl-comp`.

- Layout — R1a uses egui NATIVE panels (fast, dependency-free, gate-clean):
  left `SidePanel` = **Palette** (index artifacts to append), right
  `SidePanel` = **Inspector** (selected layer: start/in/out drags, remove),
  bottom `TopBottomPanel` = **Timeline** (the ordered CLIP LIST: each
  layer a row, reorder ↑/↓, select, tips), center `CentralPanel` =
  **Preview** (the selected layer's source played via `ClipPlayer` +
  `cover_uv`; a `clay()` plate with "Composited preview lands R1b" when
  nothing is selected).
  DEFERRED (RS4, flagged not smuggled): `egui_tiles` drag-dock docking is
  R1b's rider, adopted when panes need rearranging — R1a's fixed panels
  are the honest foundation, recorded here + in §5 + STATE.
- State: `pub struct Composer { comp: Composition, dir: Option<PathBuf>,
  selected: Option<String>, preview: Option<ClipPlayer>, palette:
  Vec<dpl_index::Record>, status: String }`. `Default` = an empty 1080p30
  comp, palette lazily loaded from `Index::recent(500)`.
- Actions: **New comp** (fresh empty), **Add clip** (append a `Layer`
  from a palette artifact — `Source::Artifact{rel}`, `in_pt=0`,
  `out_pt=clip_duration.unwrap_or(6.0)`, `start=`current comp duration so
  clips stitch end-to-end by default), **reorder/remove**, **Save**
  (writes `comp.json` to `<content_root>/inbox/<comp-id>/`, inbox-born per
  R1 §1; index write-through is R1c). Selecting a layer (re)opens the
  `ClipPlayer` on its resolved source path.
- `pub fn ui(&mut self, ui: &mut egui::Ui)` — the surface wiredfront calls.
- PilotIds registered for each control (`devpulse.composer.*`) per the
  pilot law; no new animation (ClipPlayer already requests repaints), so
  no new PilotClock registration.

**`crates/hubs/wiredfront` — mount.** Add `pub const COMPOSER: SubId =
SubId("composer");` + a `SubRoute { id: COMPOSER, label: "Composer",
blurb: "Stitch clips into a video — the After-Effects-style editor." }`
to `SPEC.subroutes`; a `composer: dpl_composer::Composer` field (behind
`Option` or `Default`) on `WiredfrontHub`; and a `if route.sub ==
COMPOSER { self.composer.ui(ui); return; }` branch in `ui()`.

**Scale story:** the Nth effect/track/source-kind is authored data
(a `Source` variant, an effect ref) — the doctor grows one arm, the UI
one row. The compositor (vello) arrives once, behind its own trait, and
every layer flows through it. comp.json stays the single AI-editable
truth.

## 4. Changes, in execution order

1. **Create `crates/comp/Cargo.toml`** — `dpl-comp`, deps `serde`,
   `serde_json` (workspace). Add `"crates/comp"` to `members` and
   `dpl-comp = { path = "crates/comp" }` to `[workspace.dependencies]`.
2. **Create `crates/comp/src/lib.rs`** — the types, load/save, doctor,
   folder contract, `duration()`, `new()`, and the full `#[cfg(test)]`
   suite (full contents authored at build).
3. **Verify stage 1:** `cargo test -p dpl-comp` green; `cargo clippy
   -p dpl-comp -- -D warnings` clean.
4. **Create `crates/composer/Cargo.toml`** — `dpl-composer`, deps `egui`,
   `dpl-comp`, `dpl-coldlight`, `dpl-video`, `dpl-index`,
   `dpl-engine-capture`, `egui-pilot-ai`. Add to `members` +
   `[workspace.dependencies]`.
5. **Create `crates/composer/src/lib.rs`** — `Composer` state + `ui()` +
   the four panes + actions, ColdLight paint only, every control `cl::tip`,
   PilotIds. Unit-testable helpers (append-clip math, resolve path) get
   `#[cfg(test)]`.
6. **Edit `crates/hubs/wiredfront/Cargo.toml`** — add `dpl-composer`.
7. **Edit `crates/hubs/wiredfront/src/lib.rs`** — the COMPOSER const,
   SubRoute, struct field, and `ui()` branch (exact seams in §3).
8. **Verify stage 2 + gate:** `cargo green` (fmt+clippy+tests+release).
9. **Exe proof:** rebuild `target\release\devpulselabs.exe`; screenshot
   the Composer at `--window 1100x700` and maximized (any-shape law).

## 5. Boundaries

- 🚫 **Never touch** `crates/editor` (the Workbench slide editor — the
  two-editors law) or `crates/viziwiz` (ViziWiz mode grows into the
  Composer at R1d, not now). 🚫 `/content` is operator data — write only
  under `<content_root>/inbox/` at runtime; never author fixtures into it,
  never commit it.
- 🚫 No ffmpeg RENDER, no generation/agent tools, no matting, no vello
  compositor in R1a (R1b–R1f). Preview is single-source playback only.
- ⚠️ **Ask-first / flagged for operator visibility:** (a) deferring RS4's
  `egui_tiles` to R1b instead of adopting it now; (b) crate names
  `dpl-comp` / `dpl-composer` and the subroute label "Composer";
  (c) saving comps to `inbox/<comp-id>/` before the R1c index seam. All
  defensible foundation calls — redline at plan approval if unwanted.
- Out of scope: keep-into-episode for comps, comp indexing, transform
  keyframes, effect registry, the `GenBackend` trait.

## 6. Verification

Dependency order, each pass/fail:

1. `cargo test -p dpl-comp` → all green. Named tests prove: round-trip
   identity; newer `v` refused; `validate` names bad fps/size/dup-ids/
   inverted-trim; folder contract names a stray; saved JSON ends in `\n`
   and holds no `#` (refs never values).
2. `cargo clippy --workspace -- -D warnings` → clean (no `#[allow]`).
3. `cargo green` → fmt + clippy + tests + release build all green (the
   wired pre-push gate).
4. Exe: `target\release\devpulselabs.exe --route wiredfront/composer
   --window 1100x700 --screenshot docs/proof/r1a-composer-1100x700.png
   --shot-frame 30` renders the four panes with the empty-comp Preview
   plate and the Palette listing real artifacts — footer visible (fit
   guard). Repeat maximized. **Feel-bar check:** does it read like the
   first app's polish? If not, fix before PR.
5. End-to-end proof of the operator's requirement (a comp is a real,
   doctor-clean, AI-editable document): a `dpl-comp` test builds a
   two-layer comp, saves, reloads to an identical value, and validates
   empty — i.e. two stitched clips persist as one validated `comp.json`.

## 7. Commit plan

Branch `feat/r1a-comp-document` off `main`. One PR. `git add` the two new
crates, the `Cargo.toml` edits, the wiredfront edits, the proof PNGs, this
plan, and the amended `r1-editor-adr-proposal.md`. Message:
`feat(wiredfront): R1a — comp document + doctor + Composer panes shell`.
Gate runs on the self-hosted runner; squash-merge. Then rebuild the exe
here + update STATE (session-end law).

## 8. Evidence (executed 2026-07-24)

- [x] `cargo test -p dpl-comp` → **8 passed**; `cargo test -p dpl-composer`
  → **4 passed**. Includes `two_stitched_clips_persist_as_one_validated_comp`
  (§6.5 — two layers save + reload identical + validate empty) and
  `add_clip_stitches_onto_the_end_and_stays_valid` (end-to-end stitch math).
- [x] `cargo green` → **green ✔** (fmt + clippy `-D warnings` + full test
  suite + release build, 1m28s; exe `target/release/devpulselabs.exe`
  rebuilt).
- [x] `docs/proof/r1a-composer-1100x700.png` — four panes render, brutal
  New/Save chips, clay Preview plate, honest empty Palette/Timeline,
  footer visible at the restored size (fit guard).
- [x] `docs/proof/r1a-composer-maximized.png` — layout scales; footer
  visible.
- **Deviations from the plan:** none in scope. As-planned foundation calls
  (flagged in §5): egui_tiles deferred to R1b; native fixed-panel layout
  realized via the house `new_child(UiBuilder::max_rect)` idiom (egui's
  `SidePanel`/`TopBottomPanel` don't resolve in this workspace's egui and
  are unused anywhere in the repo — the manual-rect split is the house
  pattern). Preview empty-state clay plate is top-anchored within its pane
  (cosmetic; the real clip fills the pane on select) — polish noted for
  R1b. The shell's own right-rail "INSPECTOR" dock coexists with the
  Composer's in-well "INSPECTOR" pane (two distinct docks, same word) —
  left as-is; the shell is out of scope (§5).
