# Pass — the WiredFRONT editor becomes the mock: icon residency, panel
# fidelity, and media that links from anywhere on the machine

## 1. Context

**The system.** `crates/composer` (`dpl-composer`, 6,394 lines in lib.rs)
is the WiredFRONT editor — the operator's After Effects. It renders at
`wiredfront/editor` through `crates/hubs/wiredfront/src/lib.rs:1459`
(`self.composer.ui(ui)`). Its intended shape is drawn panel-for-panel in
`docs/design/mocks/wiredfront-editor-mock.html` (v2.1, operator redlines
applied, 2026-07-24): eight numbered regions — route bar, tool strip,
Project panel, Composition viewer, Info, Effects & Presets, Timeline,
status bar.

**The problem, in the operator's words (2026-08-04):**

> "you have been wasting my time. The entire purpose of getting icons in
> this platform are to have them properly embedded and get my editor
> actually looking at least close to my mock. the gallery(s)/library(s)
> are suppose to allow us to effortlessly symlink them and then cache.
> We will first focus on the wiredfront editor but this is an issue for
> wiredfront and workbench"

and (2026-08-04, the scope directive):

> "i want you to plan it out methodically and understand if you need
> rewrite code you are to align to the mock and get this actually being
> an editor where I have the ability to browse and link outside of the
> app to shares/and drives in the system is running on so i can properly
> pull in and edit effortlessly."

**Why the icon work did not reach the editor.** ADR-015's engine (M1,
#163) landed and works. Its migration milestone (M2) was scoped as
"retire the Phosphor glyph consts" — a census of `cl::icon::` consumers.
The composer was never a `cl::icon::` consumer (it paints text words and
raw Unicode, never glyph consts), so it was invisible to that census and
the migration walked past the one surface the icons existed for. The
engine is sound; its *reach* is the defect.

**SCOPE RULING (operator, 2026-08-04, after this plan's first draft):**

> "correct we are focusing only on the wiredfront editor first focusing
> on the editor itself icons and UI. We will work this methodically not
> rush and get to the browsing etc after"

This pass is therefore **the editor's icons and the editor's UI only**.
The pool browser (Tier 2/3 below), the gallery's link surface, and the
drive/share browse reach are DEFERRED to named follow-up passes — their
designs stay recorded here so nothing is re-derived later.

**Consequence worth stating plainly: the narrowed pass amends no laws.**
The pool tier is what required the ADR-015 amendment, the
`architecture.md` §1 change, and the runtime `dpl-svg` dependency.
Baking the editor's icons uses the existing ratified mechanism exactly as
designed. Defect 1 below is deferred; defect 2 is this pass.

**Three defects, one arc:**

1. **Residency** — 25 semantic ids are baked; the other 1,731 vendored
   Lucide icons are unreachable from any surface. Using one costs a
   hand-edit of `tokens/icons.json` plus a full rebuild. That is the
   opposite of "effortlessly symlink them and then cache."
2. **Fidelity** — the composer paints zero icons. Its tool strip renders
   the *words* `Select · Hand · Pen · Camera · Text · null · pre-comp ·
   track · m.blur · 3D`; the mock renders glyphs. Transport, project
   rows, and timeline switches are the same story.
3. **Reach** — Browse windows can only walk down from a fixed root list.
   There is no way to type a path, so an unmapped UNC share
   (`\\server\share`) is unreachable, and "link, don't copy" exists as a
   hidden global mode rather than a per-file choice.

## 2. Current state (every claim read in-session 2026-08-04)

### 2.1 Branch reality — read this first

`git status -sb` in `C:\localdev\dpl-lane-lucide` (branch
`feat/lucide-m2-migration`): **ahead 3, behind 57** of `origin/main`.

The three unmerged commits are `9a915e4` (Lucide M2 migration),
`0607ad9` (drift ratchet), `f816501` (generated slots). **M2 has never
landed on main.** Verified: `git show origin/main:crates/coldlight/src/
lib.rs` line 827 still declares `pub mod icon` with the Phosphor
codepoint consts, and `git grep -c "cl::icon::" origin/main` reports
**38 call sites across 10 files**:

```
deck 5 · hubs/coach 1 · hubs/home 1 · hubs/present 1 · hubs/shows 2 ·
hubs/wiredfront 1 · hubs/workbench 1 · shell 4 · shell/status 1 ·
viziwiz/palette 21
```

`crates/hubs/coach/src/lib.rs` is a **new consumer that landed on main
while this lane sat** — the lane's M2 commit does not know it exists. A
naive merge leaves the Coach hub on retired consts.

`origin/main`'s `tokens/icons.json` carries **25** ids (the lane's 24 plus
`brain`). Both counts are chrome-only; **neither contains a single editor
icon.**

### 2.2 The icon engine (`origin/main` is the baseline)

- `crates/coldlight/assets/lucide/` — `icons/` (**1,756** `.svg`,
  **706,500 bytes** total, mean **402 B**), `LICENSE`, `VERSION`
  (Lucide 1.27.0, ISC). Verified by `ls | wc -l` and `cat | wc -c`.
- `crates/coldlight/tokens/icons.json` — the 8th authored source, 25
  entries of `{ id, lucide, label, blurb }`.
- `crates/coldlight/build.rs:241` `generate_icons` — reads icons.json,
  for each entry reads `assets/lucide/icons/{lucide}.svg`, calls
  `dpl_svg::tessellate_icon(&svg, ICON_TOLERANCE)` (`ICON_TOLERANCE =
  0.02`, build.rs:234), and writes literal vertex/index arrays into
  `icons_gen.rs`. A missing SVG panics the build naming the file.
- **Measured cost**: `target/debug/build/dpl-coldlight-*/out/icons_gen.rs`
  is **283,796 bytes for 24 icons ≈ 11.8 KB of Rust source per icon.**
  Extrapolated to the full pool: **~20.7 MB of generated Rust.** Baking
  the pool wholesale is not viable — that is a rustc const-array workload,
  not a data file.
- `crates/coldlight/src/lib.rs` — `IconDef { id, label, lucide, blurb,
  viewbox, vertices, indices }`, `include!(icons_gen.rs)`,
  `IconDef::mesh(rect, color)`, `draw_icon`, `draw_icon_brutal`,
  `icon_label`. An unknown id paints an honest hairline box, never
  nothing.
- `crates/svg` (`dpl-svg`) — ceiling ∅, egui-free.
  `tessellate_icon(svg: &str, tolerance: f32) -> Result<Tess, SvgError>`;
  `Tess { viewbox: (f32,f32,f32,f32), vertices: Vec<(f32,f32)>, indices:
  Vec<u32> }` (`crates/svg/src/icon.rs:15,25`). Pure data — no egui
  types. **It is already a complete runtime-capable tessellator; only
  the wiring declares it build-only.**
- `crates/coldlight/Cargo.toml` — `[dependencies] egui`;
  `[build-dependencies] serde_json, palette, dpl-svg`.

### 2.3 The laws this pass must amend (not violate)

- `docs/design/architecture.md` §1, ENGINE block, verbatim: *"dpl-svg
  rides as coldlight's BUILD-dep — icon geometry is generated, never
  parsed at runtime, ADR-015"*.
- `docs/design/adr-015-svg-icon-engine.md` §3: *"Zero runtime parsing,
  zero runtime deps, zero per-frame tessellation."* Its **Rejected**
  list: *"Runtime SVG parsing — a runtime dep + cache complexity for
  **icons that never change between builds**; the generated-consts
  pattern is the house pattern."*
- **The premise named in that rejection is what the operator has now
  changed.** Icons that never change between builds do not need runtime
  parsing; icons the operator browses and links on demand do. The
  rejection was correct for its premise and is stale for the new one.
  ADR-015 already anticipated this door: its Consequences leave
  *"arbitrary operator SVGs … OPEN"* pending a follow-up ADR.
- **Structure-audit impact is nil.** `crates/xtask/src/main.rs:631`
  `CEILINGS` constrains `("dpl-svg", &[])` — dpl-svg's *own* deps, which
  do not change. `EGUI_FREE` (line 648) lists `dpl-svg` — it stays
  egui-free (`Tess` is pure tuples). **`dpl-coldlight` has no ceiling
  entry** (ADR-016 states this explicitly: *"dpl-coldlight is a SUPPORT
  crate with no §2 ceiling"*). Adding `dpl-svg` to coldlight's runtime
  deps therefore passes `cargo xtask structure` unchanged; the amendment
  is **documentary**, and must still be made in the same commit.

### 2.4 The gallery today

`crates/hubs/workbench/src/library.rs:784` `icons_wall`:

```rust
for def in cl::ICONS.iter() {
    let resp = cl::card_frame().show(ui, |ui| {
        ui.set_width(168.0);
        ui.horizontal(|ui| {
            let _ = cl::icon_label(ui, def.id, 28.0, cl::ink::STRONG);
            let _ = cl::icon_label(ui, def.id, 28.0, SPEC.tone.color());
            // + draw_icon_brutal
```

It iterates `cl::ICONS` — the 25 baked ids. It is a **display case, not a
browser**: no search, no pool, no way to act on an icon. `LibSection::
Icons` (library.rs:47, slug `icons`) reaches it via `--lib-section icons`.

### 2.5 The composer — what the mock asks for vs what is painted

`grep -rn "draw_icon\|icon_label\|cl::icon" crates/composer/src/` →
**zero hits across all 7 files.**

Layout already matches the mock's skeleton. `ae_screen` (lib.rs:2863)
computes `tools_rect · project_rect · viewer_rect · info_rect · fx_rect ·
timeline_rect · status_rect` and dispatches to `tool_strip` (2964),
`project_panel` (3018), `viewer_pane` (3268), `info_pane` (3362),
`fx_pane` (3721), `timeline_panel` (3754), `status_row` (4243). **The
bones are right; the vocabulary is wrong.**

| Mock | Region | Code today | Path |
|---|---|---|---|
| `▲ ✋ ✒ 🎥 T` | ② tools | `tool_btn(ui, t.label(), …)` → words `Select`… | lib.rs:2974 |
| `⊹ ⧉ ⌖ ⨀ ◇` | ② tools | `small_btn(ui, label, …)` → words `null`,`pre-comp`,`track`,`m.blur`,`3D` | lib.rs:2993-3006 |
| `⏮ ◀◀ ▶ ▶▶ ⏭` | ④ transport | raw Unicode in `small_btn`: `("⏮",…),("◀",…),(if playing {"⏸"} else {"▶"},…),("▶|",…),("⏭",…)` | lib.rs:3327-3336 |
| comp/vid/img/folder swatches | ③ project | `row_line(ui, "cmp"/"vid"/"img"/"pak"/"rnd", …)` — 3-letter monospace text at `FontId::monospace(10.0)` | lib.rs:3091,3125,3202,3257 + helper 5320 |
| `＋ Import`, `Browse folders…` | ③ project | `small_btn(ui, "Browse…", "browse")` | lib.rs:3022 |
| — (no mock equivalent) | ③ project | reference toggle painted as `"⌁ in place"` / `"⧉ copy"` | lib.rs:3037-3050 |
| `✕` close | ⑤ Info | `ui.label(RichText::new("x"))` | lib.rs:3367 |
| `🔍 search effects…` | ⑥ FX | `fx_pane` | lib.rs:3721 |
| `◇ ⨀ ＠` per-layer switches, `◆`/`●` keyframes | ⑦ timeline | `outline` paints `"+"`/`"-"` twirls as monospace text; no switch columns exist | lib.rs:3804-3843 |
| `💬 Chat` | ⑧ status | `status_row` | lib.rs:4243 |

Widget helpers that must learn icons — `tool_btn` (5206, `RichText`
label only), `small_btn` (5226, `glyph: &str` → `RichText`), `row_line`
(5320, paints `glyph` at `FontId::monospace(10.0)`, name at x+32),
`tag_chip` (5246), `section_header` (5309).

ColdLight vocabulary in use and correct: `cl::treated(tone,
"neubrutalist", &[], state)`, `cl::brutal(tone)`, `cl::tip(resp, …)`,
`cl::space::S1/S2/S3`, `cl::ink::*`, `cl::surface::*`, `Tone::*`,
`cl::RADIUS_SM`. `tokens/traits.json` carries exactly four ids:
`neubrutalist · claymorphic · cyber · retro`.

### 2.6 Browse and import — far more capable than assumed

`crates/composer/src/browse.rs` (934 lines) **already has**: a shared
off-thread `ScanCache` (`Arc<Mutex<HashMap<PathBuf, Vec<Entry>>>>`, line
19) with `warm_cache` seeding at boot (26); `roots()` (221) returning
operator libraries → home folders → **every present Windows drive letter
`A..=Z`** (245) / unix mounts; persistent operator libraries in
`.userdata/libraries.json` via `add_library`/`remove_library` (300/317);
deep search off-thread capped at `SEARCH_CAP = 500`; a thumbnail grid
with `THUMBS_PER_FRAME = 8` budgeting (47) — **the house precedent for
per-frame work budgets**; pin-pack and two-tap ingest.

`crates/composer/src/import.rs` (513 lines) **already has**
`import_reference` (191): canonicalizes the source, creates a
**single-file symlink** into `content/` (`symlink_file`, 224/229 —
`std::os::windows::fs::symlink_file` on Windows), **falls back to a copy
when the OS denies symlinks**, and finalizes identically to a copy
(poster, duration, write-through index). Its doc-comment is precisely the
operator's ask: *"their paid AE presets / image folders stay where they
live."*

**The gaps, exactly:**

1. **No address bar.** `window_ui` (360) renders `b.cwd` as a *read-only
   label* (520-524). There is no `TextEdit` bound to a path. The only
   `TextEdit` is `b.query`, the filter/search field (531). **An unmapped
   UNC path cannot be entered, therefore cannot be reached.** Mapped
   network drives already work — they surface as drive letters in
   `roots()`.
2. **Link-vs-copy is a hidden global mode.** `handle_import` (2336)
   branches on `self.prefer_reference` — one toggle in the Project panel
   header, persisted via `reference_pref_load/save` (83/86). There is no
   per-file choice at the moment of picking, and the Browse window — where
   the operator is actually looking at the file — never mentions it.
3. **No offline story for links.** `packs.rs` tracks `p.online` for
   pinned packs, but an indexed *record* whose symlink target is on a
   disconnected share has no honest state; it simply fails to load.
4. **`roots()` cannot express a network location** — no UNC entries, and
   `\\server\share` has no drive letter to enumerate.

### 2.7 Verified pool names (a wrong name fails the build)

All **49** names below were confirmed present in
`crates/coldlight/assets/lucide/icons/` by direct `-f` test on
2026-08-04:

```
mouse-pointer-2 hand pen-tool video type crosshair group scan-line
aperture box play pause skip-back skip-forward rewind fast-forward film
folder folder-open file-video-camera image sparkles package import plus
link copy unlink search sliders-horizontal chevron-down chevron-right
palette timer eye eye-off lock diamond circle-dot link-2 hard-drive
network server share-2 arrow-up star undo-2 redo-2 trash-2
```

Two candidates were **rejected as absent** and substituted:
`file-video` → **`file-video-camera`**; `waveform` → **`audio-waveform`**.

## 3. Target design

### 3.1 The two-tier icon registry

The insight the measurements force: **baked geometry and browsable pool
cannot be the same tier.** 11.8 KB of Rust per icon makes 1,756 icons a
20 MB compile; 402 B of SVG text per icon makes the same pool a 706 KB
string table. So carry the *pool as text* and the *chrome as geometry*.

**Tier 1 — BAKED (`ICONS`, unchanged mechanism).** Every icon the app
chrome paints every frame. Authored in `tokens/icons.json`, tessellated
by build.rs, zero runtime cost. Grows from 25 → ~74 in this pass (the
editor set). Adding one is still a JSON line plus a rebuild — correct,
because chrome is not browsed.

**Tier 2 — POOL (new). DEFERRED — not built in this pass.** The design
below is ratified and recorded; it lands in the follow-up "icon pool
residency" pass together with the ADR-015 amendment, the
`architecture.md` §1 change, and coldlight's runtime `dpl-svg` edge.
All 1,756 SVGs embedded as `&'static str` via a
new `generate_pool` in build.rs (`POOL: &[(&str, &str)]` — name, svg
text; ~706 KB of string literals, which rustc handles trivially).
Tessellated **lazily at runtime** through the same `dpl_svg::
tessellate_icon`, memoized in a process-wide cache. This is what makes
the gallery browsable and "linking" instant.

**Tier 3 — LINKED. DEFERRED — not built in this pass** (ships with Tier
2; the operator's "symlink" for icons).  A semantic id bound
to a pool name **at runtime**, persisted in operator userdata
(`.userdata/icons-linked.json`), resolved through Tier 2's cache. This is
the direct analogue of `import_reference` for media: the icon is used
where it lives, nothing is copied, no rebuild. It is also the seam
ADR-015 left open for the operator's own SVG collection.

Resolution order — `cl::icon_any(id)`: Tier 1 → Tier 3 → `None`. Chrome
always wins; a link can never shadow a baked id (predictability), and an
unknown id still paints the honest hairline box.

**Why not persist links into `tokens/icons.json`?** `tokens/` is repo
source and is absent from a shipped install. A runtime write there would
work only on a dev checkout. Operator bindings belong in operator data;
promoting a link to Tier 1 is a deliberate authoring act (§4 step E1.9
provides the copyable JSON line).

### 3.2 The cache

```rust
static POOL_CACHE: OnceLock<Mutex<HashMap<&'static str, Option<Arc<Tess>>>>>
```

Keyed by pool name; tolerance is the fixed `ICON_TOLERANCE`, so it is not
part of the key. `Option` memoizes *failures too* — a malformed icon is
parsed once, never once per frame. Miss cost is one parse + flatten of
~400 B.

**Per-frame budget**, following `browse.rs`'s `THUMBS_PER_FRAME`
precedent exactly: `POOL_TESS_PER_FRAME = 12`. A gallery scroll fills
over a few frames instead of hitching once; cards past the budget paint
their name plate and request a repaint. **No unbounded work on the paint
thread** — the same law browse.rs already obeys.

### 3.3 Scale story

| Action | Cost after this pass |
|---|---|
| App chrome needs a new icon | 1 line in `icons.json` + rebuild (unchanged) |
| Operator wants pool icon N of 1,756 | browse → tap **Link** → in use, no rebuild |
| Operator's own SVG collection (ADR-015 open item) | same Tier-2 seam; a source dir joins `POOL` |
| Binary growth | +706 KB (pool text), one time, regardless of how many are used |
| Compile growth | +49 baked icons ≈ +580 KB generated Rust |

### 3.4 The editor's icon language (Tier 1 additions)

49 new ids, every `lucide` value verified in §2.7.

**Tools (mock ②)** — `tool-select`→mouse-pointer-2, `tool-hand`→hand,
`tool-pen`→pen-tool, `tool-camera`→video, `tool-text`→type,
`tool-null`→crosshair, `tool-precomp`→group, `tool-track`→scan-line,
`tool-mblur`→aperture, `tool-3d`→box.

**Transport (mock ④)** — `transport-start`→skip-back,
`transport-back`→rewind, `transport-play`→play,
`transport-pause`→pause, `transport-forward`→fast-forward,
`transport-end`→skip-forward.

**Project (mock ③)** — `proj-comp`→film, `proj-folder`→folder,
`proj-video`→file-video-camera, `proj-image`→image,
`proj-element`→sparkles, `proj-render`→package, `proj-pack`→folder-open,
`browse`→folder-open, `import`→import, `add`→plus.

**Link/copy (mock ③ note + E3)** — `ref-link`→link, `ref-copy`→copy,
`ref-offline`→unlink, `drive`→hard-drive, `network`→network,
`server`→server, `share`→share-2, `nav-up`→arrow-up, `library-star`→star.

**FX & Info (mock ⑤⑥)** — `fx`→sliders-horizontal, `search`→search,
`group-open`→chevron-down, `group-closed`→chevron-right,
`preset-color`→palette, `preset-ease`→timer.

**Timeline (mock ⑦)** — `layer-visible`→eye, `layer-hidden`→eye-off,
`layer-lock`→lock, `layer-parent`→link-2, `keyframe`→diamond,
`keyframe-ease`→circle-dot, `audio`→audio-waveform.

**Edit verbs (⑧)** — `undo`→undo-2, `redo`→redo-2, `delete`→trash-2.

### 3.5 Browse reach (E3)

- **Address bar** — a `TextEdit` bound to a new `Browser.addr`, seeded
  from `cwd`, committing on Enter. Accepts absolute paths *and* UNC
  (`\\server\share`). `std::fs::read_dir` and `Path::is_dir` already
  handle UNC on Windows; **no new dependency, no FFI.** A path that does
  not resolve says so in place and does not navigate.
- **Network locations** — operator-added UNC roots persist through the
  existing `libraries.json` mechanism, so a share entered once is a
  one-tap chip in every Browse window forever. Root chips gain
  `drive` / `network` / `star` icons so a share reads as a share.
- **Per-file link vs copy** — the Browse card gains an explicit **Link**
  affordance beside its pick, routing to `import_reference` regardless of
  the global mode; the plain pick honours the mode as today. The global
  toggle stays and its tooltip stops being the only place the concept is
  explained.
- **Offline honesty** — `Record` gains no schema change; instead the
  Project panel resolves a referenced path at paint from the already-warm
  index and paints `ref-offline` + a muted row when the target is
  unreachable, matching the pack shelf's existing `online` language.
- **Discovery of shares is explicitly OUT.** Enumerating unmapped shares
  needs `WNetEnumResource` (FFI) or `net view` (ADR-009 subprocess).
  Neither is justified when typing the path once and starring it solves
  the operator's stated need. Recorded in §5, revisit if the operator
  asks.

## 4. Changes, in execution order

### E0 — Base the work on main (NOT the stranded lane)

**Corrected 2026-08-04.** An earlier draft made "land the stranded lane"
a precondition. It is not one, and saying so was wrong:
`IconDef`, `draw_icon`, `draw_icon_brutal`, and `icon_label` all exist on
`origin/main` (read there, §2.2), and the composer has **zero**
`cl::icon::` call sites — so the Phosphor consts' continued presence on
main is irrelevant to this pass. The two were coupled only by both being
"icon work."

Landing `feat/lucide-m2-migration` is a real pass with genuine semantic
conflicts (main's `generate_matrix`/`matrix.json` vs the lane's
`generate_slots`/`slots.json` at the same build.rs position; main's
`surface`/`ink` hex consts vs the lane's generated slots). It is
**owed work, tracked as D0** — not something to smuggle inside a pass
scoped to the editor.

1. `git checkout -b feat/editor-icons origin/main` — branch fresh from
   main. Expect a full rebuild (57 commits of drift).
2. Re-verify the §2.5 line numbers against main before editing: the
   lane/main composer delta is small (`lib.rs | 2 +-`, `browse.rs | 8
   +-`) but line anchors must be re-read, never assumed (the
   ground-truth law).

**D0 — land the stranded lane** (owed, operator to schedule): rebase
`feat/lucide-m2-migration` onto main; reconcile slots⇄matrix (they are
COMPLEMENTARY — main's matrix references the slot names the lane
generates, and main still hardcodes them as hex); migrate the **1** new
`cl::icon::` site in `crates/hubs/coach/src/lib.rs` that landed on main
while the lane sat; prove `rg "cl::icon::"` → zero. Until D0 lands,
main keeps `pub mod icon` and its 38 call sites. **The one collision
this pass creates for D0 is `tokens/icons.json`** — both add array
entries; a mechanical conflict, resolved by keeping both sets.

### E1 — The editor's icon set, baked (same PR as E2)

The 49 ids ride Tier 1 through the existing generator. No new crate, no
new dependency, no law amended.

5. `crates/coldlight/tokens/icons.json` — add the **49** entries of
   §3.4, each with `label` + layman `blurb`. Every `lucide` value is
   verified present (§2.7); a wrong one fails the build by design.
6. Test in `crates/coldlight/src/lib.rs` — the census law: assert
   `ICON_COUNT == 74` and that every id in §3.4's editor set resolves
   through `icon_def`, so a dropped entry fails the gate rather than
   silently painting hairline boxes.

*(Separate PR only if E2's diff grows unreviewable — otherwise these two
steps land WITH the composer wiring, per the one-target-one-PR law: the
icons and the surface that paints them are one coherent change.)*

### E2 — The editor wears the mock (PR 2)

7. `crates/composer/src/lib.rs` — extend the widget helpers to carry
    icons, preserving every existing signature's caller:
    - `tool_btn(ui, icon_id, label, active)` — icon above/beside label
      per the mock's `.tool` (icon + `<small>` caption);
    - `small_btn_icon(ui, icon_id, key)` — icon-only, `cl::tip`
      mandatory;
    - `row_line(ui, icon_id, name, trail, key)` — replace the
      `FontId::monospace(10.0)` 3-letter text at `rect.min.x + 4.0` with
      `cl::draw_icon` into a 14×14 rect at the same anchor; name x-offset
      stays 32.0.
8. `tool_strip` (2964) — `EditorTool` gains `fn icon(self) -> &'static
   str` returning the §3.4 ids; the five later-phase affordances map to
   `tool-null · tool-precomp · tool-track · tool-mblur · tool-3d`.
   Honest blurbs unchanged.
9. `viewer_pane` transport (3327-3336) — replace the five raw Unicode
   strings with `transport-*` ids; play/pause swaps
   `transport-play`/`transport-pause` on `self.playing`.
10. `project_panel` (3018) — `Browse…` gains `browse`; the reference
    toggle paints `ref-link`/`ref-copy` beside its word; comp/pack/
    footage/render rows pass `proj-comp`/`proj-pack`/`proj-video`|
    `proj-image`/`proj-render`; the `fx` corner (3233) paints `fx`.
11. `info_pane` (3367) — the `"x"` label becomes `cl::icon_label(ui,
    "close", …)`, keeping the same click behaviour and tooltip.
12. `fx_pane` (3721) — search line gains `search`; tree group rows gain
    `group-open`/`group-closed`; the applied-preset rows gain
    `preset-color`/`preset-ease` per mock ⑥.
13. `outline` (3804) — twirl `"+"`/`"-"` monospace becomes
    `group-closed`/`group-open`; add the mock's per-layer switch columns
    painting `layer-visible`/`layer-hidden`, `layer-lock`,
    `layer-parent`, right-aligned before the existing time column.
    Keyframe marks use `keyframe`/`keyframe-ease`.
14. `status_row` (4243) — `undo`/`redo`/`delete` verbs gain their ids.
15. **Any-shape proof** — the added switch columns change outline
    metrics; re-check `fit()` bounds at `--window 1100x700` so nothing
    overlaps (RR2 law).
16. `cargo green`; proof shots `--route wiredfront/editor` at maximized,
    `--window 1100x700`, and `--window 1600x900`; **feel-bar check
    against the mock side by side**; PR; double-green; merge.

### Deferred milestones (designed above, NOT built in this pass)

- **D1 — icon pool residency.** §3.1 Tier 2/3 + §3.2 cache + the gallery
  browser, with the ADR-015 amendment, `architecture.md` §1 change, and
  coldlight's runtime `dpl-svg` edge. Unblocks "link any of 1,756
  without a rebuild."
- **D2 — browse reaches shares and drives.** §3.5 in full: the address
  bar (the single change that makes unmapped UNC reachable), typed root
  chips, the per-card Link action, and offline honesty for referenced
  records.
- **D3 — Workbench adoption.** The same icon wiring for the Workbench
  hub's surfaces, once the operator has ratified the editor's look.

## 5. Boundaries

**🚫 Never touch**
- `/content` — operator data (CLAUDE.md law): never commit, author into,
  or delete.
- `crates/coldlight/assets/lucide/**` — vendored sources are never
  mutated; derived geometry is generated, never written back.
- Any `*_gen.rs` — generated, never hand-edited.
- Source media on any external drive or share — this pass adds *linking*;
  it never moves, renames, or deletes a file outside `content/`.
- `import_file`'s copy semantics — copy stays the default; linking is
  additive.

**⚠️ Ask first**
- Retiring `Phosphor.ttf` (still vendored/registered; a separate
  census-gated cleanup).
- Any new runtime dependency beyond `dpl-svg` (ceiling decisions).
- Changing `Record`'s schema for offline state — §3.5 deliberately avoids
  it; if paint-time resolution proves too slow, the schema change is its
  own ratified pass.
- Raising `POOL_TESS_PER_FRAME` above 12, or embedding a second icon
  source dir.

**Out of scope for this pass** (operator scope ruling, §1)
- **The icon pool browser and runtime linking** (D1) — with it, the
  ADR-015 amendment, the `architecture.md` §1 change, and coldlight's
  runtime `dpl-svg` dependency. **This pass amends no laws.**
- **Browse reach: address bar, UNC, per-card Link, offline rows** (D2).
  Note the standing limitation this leaves in place: an unmapped share
  stays unreachable until D2, because there is still nowhere to type a
  path.
- Unmapped-share *discovery* (WNetEnumResource / `net view`) — §3.5.
- **Workbench's own icon adoption** (D3) — named so it is not silently
  dropped.
- The mock's 3D / camera / tracker / motion-blur *engine* work. The mock
  itself says the UI contract is committed now and the build phases
  follow the 2D core; this pass ships the honest affordances with their
  existing "lands with pass X" blurbs, not fake capability.
- Arbitrary operator-SVG intake — the Tier-2 seam is built for it; the
  curation pass is separate (ADR-015 open item).

## 6. Verification

Run in order; each is pass/fail, no judgement calls.

**E0**
- `git rev-parse HEAD` equals `git rev-parse origin/main` at branch
  creation — the work is based on main, not the stranded lane.
- The §2.5 line anchors re-read against main before any edit.

**E1**
- `cargo test -p dpl-coldlight` → `ICON_COUNT == 74`; every editor id of
  §3.4 resolves through `icon_def`.
- **Break-it**: add `{"id":"nope","lucide":"not-a-real-icon",…}` to
  `icons.json` → build fails naming the missing file. Revert.
- `target\release\devpulselabs.exe --lib-section icons --screenshot
  docs\proof\icon-registry-74.png --window 1100x700` (and maximized) —
  the gallery gate: all 74 baked ids render in three renditions. **An
  icon no surface can browse is a gallery-gate violation** (ADR-015 §5),
  and the existing `icons_wall` already satisfies it for Tier 1.

**E2**
- `rg "draw_icon|icon_label" crates/composer/src/` → **> 0** (today: 0).
- `rg 'RichText::new\("[⏮⏭▶◀⏸⧉⌁]' crates/composer/src/` → **zero**.
- Every new `.clicked()` control has a `cl::tip` within its block
  (tooltip law — grep the diff).
- `--route wiredfront/editor --screenshot` at maximized, `1100x700`,
  `1600x900`: tool strip shows icons; transport shows icons; project rows
  show kind icons; timeline shows switch columns; **no overlap at any
  size** (RR2).
- **Feel bar**: the three shots beside `wiredfront-editor-mock.html`,
  region by region. If it does not read as the mock, it is not done.

**End-to-end proof (this pass's criterion):** open
`--route wiredfront/editor` beside `wiredfront-editor-mock.html` at
`1100x700` and maximized. Every mock region — tool strip, project rows,
transport, Info close, FX tree, timeline switches, status verbs — reads
as the mock, in icons, with nothing overlapping at either size. The
operator's word on the feel bar is the gate.

## 7. Commit plan

**ONE PR** — `branch → gh pr create → double-green → merge`, branch
deleted after (the one-target-one-PR law). `docs/STATE.md` updated after
the merge; `target\release\devpulselabs.exe` rebuilt **here** and its
mtime verified at session end.

`feat(composer): the editor wears its icons — the mock, panel for panel`
— the 49 registry entries plus the composer wiring: helpers, tool strip,
transport, project, info, fx, timeline switches, status.

Owed/deferred passes get their own plans when the operator calls them;
their designs are recorded in §3–§4 so nothing is re-derived:
**D0** land the stranded lane · **D1** icon pool residency ·
**D2** browse reach · **D3** Workbench adoption.

## 8. Evidence

*(filled during execution — commands run, real output, deviations named
explicitly per the execution-discipline law)*
