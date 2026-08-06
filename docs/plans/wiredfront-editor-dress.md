# Pass — the WiredFRONT editor wears its mock

**Status:** PLAN, 2026-08-05. Blocked on ONE ruling (§7). Everything else is
measured, not inferred.

## 1. Context

Operator, 2026-08-05:

> "start planning this out please and get this right … its fucking this
> `docs/design/mocks/wiredfront-editor-mock.html` … EDITOR.PNG IS FUCKING PART
> OF WORKBENCH IM RENAMING THAT RIGHT NOW"

**The target is the mock, not a reference shot.** `editor.png` and
`editor-2.png` are **Workbench** and were wrongly recorded in
`visual-language.md` as the WiredFRONT target; that is now corrected.

### 1.1 What this invalidates — stated plainly

`visual-language-v3-mock.html:401` says:

> "the two editor shots map to flat wells + liquid-glass/cyber chrome + glow
> spotlights (**WiredFRONT**, Guide Workbench)"

That sentence is **the entire origin** of the `aero`/`neon-glass` reading for
WiredFRONT — it is what produced the proposed coordinate `aero · measured ·
neon-glass · neon`, and what "aero+hacker" was subsequently attached to. Its
premise is false: those shots are Workbench. **The line must be amended and
the derivation re-run from the real mock.**

Nothing about `aero+hacker` is wrong *for Workbench*. It is wrong as the
WiredFRONT editor's dress, because it was derived from Workbench references.

## 2. Current state — the mock, MEASURED (read in full 2026-08-05)

`docs/design/mocks/wiredfront-editor-mock.html`, verbatim CSS:

### 2.1 Palette

```
--well:#0a0c10  --base:#12151c  --overlay:#1a1e28  --hairline:#2a2f3d
--ink:#c8cdd8   --strong:#f0f3fa --subtle:#7a8296  --muted:#565d70
--magenta:#ff2d95 --cyan:#22d3ee  --lime:#a3e635   --coral:#ff6b5e
--mag-wash:rgba(255,45,149,.14)   --cyan-wash:rgba(34,211,238,.12)
--lime-wash:rgba(163,230,53,.12)
```

**This is the C1 palette conflict**, still unresolved: these are *lighter*
than the v3/shipped values (`well #07090e`, `base #0a0c12`, `overlay #181c27`,
`hairline #232836`). The operator ruled *"align to v3"* for the palette, so
§3 uses **v3's surface values with the mock's STRUCTURE** — see §7.2.

### 2.2 The treatment vocabulary

| element | fill | edge | radius | shadow |
|---|---|---|---|---|
| `.panel` | `--base` | **2px** `--hairline` | 6 | **`4px 4px 0` rgba(0,0,0,.9)** |
| `.chip` | `--overlay` | **2px** `--hairline` | 6 | **`3px 3px 0`** |
| `.tool` | `--overlay` | **2px** `--hairline` | 5 | **`2px 2px 0`** |
| `.tool.active` | `--mag-wash` | **2px `--magenta`** | 5 | **`1px 1px 0`** ← presses IN |
| `.tool.on` | `--lime-wash` | 2px `--lime` | 5 | inherited |
| `.ptab` | `--overlay` | 2px `--hairline`, **no bottom** | 6 6 0 0 | — |

**Three facts that decide everything:**

1. **Every surface carries a HARD OFFSET SHADOW, blur 0.** That is
   `atmosphere: hard-cast`, exactly as authored in `matrix.json:44`.
2. **Active is a tone-coloured EDGE plus a ~12–14% WASH — never a saturated
   fill.** Today's `neubrutalist.fill = {tone:400}` is opaque, which is
   precisely why the shipped editor reads as pills.
3. **Press shrinks the shadow** `2px → 1px`: the control sinks onto its own
   shadow. `hard-cast`'s authored `active` does exactly this
   (`[3,3] → [1,1]`).

### 2.3 The eight regions (layout contract)

① route + workflow tabs · ② tool strip · ③ Project panel · ④ Composition
viewer + transport · ⑤ Info · ⑥ Effects & Presets · ⑦ Timeline (outline +
lanes + CTI) · ⑧ status bar + chat chip.

`crates/composer/src/lib.rs` `ae_screen` already computes all eight rects.
**The layout is not the problem; the dress is.**

## 3. Target — how the mock maps onto the matrix

Facet by facet, against authored values only:

| facet | mock says | authored value | match |
|---|---|---|---|
| STRUCTURE | radius 6, edge 2px | **`massive`** (radius 6, edge_width 2.0) | **exact** |
| ATMOSPHERE | `[4,4]`/`[3,3]` blur 0, press `[1,1]` | **`hard-cast`** (`[3,3]`→`[1,1]`) | **exact** |
| MATERIAL | fill `OVERLAY`, ink `BASE`, edge `HAIRLINE` | *nothing authored matches* | **GAP** |
| ANCESTOR | — | `brutalist` (native `massive` + `hard-cast`) | fits |

### 3.1 The material gap — one new authored value

No existing material is fill=`OVERLAY` + ink=`BASE` + edge=`HAIRLINE`:

- `glass`/`metallic` = fill `OVERLAY`, ink **`STRONG`**, edge **`tone 200`**
- `clay`/`paper` = fill **`RAISED`**, ink `BASE`, edge `HAIRLINE`

The mock wants clay's *quiet ink and hairline edge* on glass's *overlay fill*.
Proposed, and it is one entry, not a redesign:

```jsonc
{ "id": "plate", "label": "Plate",
  "blurb": "A quiet chrome plate — overlay fill, hairline edge, body ink. The
            surface a dense tool screen is built from.",
  "fill": { "surface": "OVERLAY" },
  "ink":  { "ink": "BASE" },
  "edge_color": { "surface": "HAIRLINE" } }
```

Cost: **8 rulings** (every ancestor must classify it — §5 legality law), which
`generate_matrix` already enforces.

### 3.2 Active state — the correction that fixes the pills

`plate` + a STATE overlay that swaps edge to the tone and fills with a
**wash**, never the solid tone:

```jsonc
"active": { "edge_color": { "tone": 400 },
            "fill": { "tone": 500, "alpha": 36 },   // ≈14%
            "ink": { "ink": "STRONG" } }
```

This is the single change that makes the editor read like the mock: **the
loud thing becomes the one selected control**, and everything else is a quiet
plate.

### 3.3 The resulting coordinate

> **`brutalist · massive · plate · hard-cast`**, tone = per-surface
> (magenta for the editor's active accent, matching the mock).

Checked against `matrix.json:79-83`: brutalist natively wears `massive` and
`hard-cast`; `plate` needs its ruling added as `allowed` for brutalist.

**This is NOT `aero`** — and it cannot be, because `aero` lists `hard-cast`
as **forbidden** (`matrix.json:95`) while every surface in the mock casts one.

## 4. Changes, in execution order

1. `docs/design/mocks/visual-language-v3-mock.html:401` — amend: the two
   editor shots are **Workbench**, not WiredFRONT.
2. `crates/coldlight/tokens/matrix.json` — add the `plate` material (§3.1) and
   its 8 ancestor rulings; add its `active` state overlay (§3.2).
3. `crates/composer/src/lib.rs` — the widget helpers (`tool_btn`,
   `tool_cap_btn`, `icon_btn`, `icon_word_btn`, `small_btn`, `tag_chip`,
   `chip`) paint **plate + hairline + hard shadow**; ACTIVE gets tone edge +
   wash; press shrinks the shadow.
4. Same file — `ae_screen`'s panel rects gain the mock's **2px hairline edge +
   `[4,4]` shadow**; `.ptab`-style panel tabs on Project / Composition / Info /
   Effects & Presets / Timeline.
5. `crates/composer/src/player.rs` — the preview modal wears the same plate;
   **replace the `＋` tofu glyph with the `add` registry icon** (a real defect
   visible in `docs/proof/preview-player-1600x900.png`).
6. Proof: `--route wiredfront/editor` at maximized, `1100x700`, `1600x900`,
   plus `--preview-drill` for the player, each beside the mock region by
   region.

## 5. Boundaries

**🚫 Never touch**
- `/content` — operator data.
- The mock itself — it is the target, not a working file.
- `ae_screen`'s LAYOUT maths — the eight rects are already right.

**⚠️ Ask first**
- Any *additional* facet value beyond `plate`.
- Changing the mock's magenta active accent to another tone.

**Out of scope**
- Workbench's dress (`editor.png`/`editor-2.png` are its references — its own
  pass).
- `resolve()` — this rides `cl::treated` and converts later with no visual
  change.

## 6. Verification

- `cargo green` exits 0; the matrix census test still reports every ancestor
  ruling on every value (now 8 × 22 = 176).
- Proof shots at three sizes **beside the mock**, region by region: panels
  bordered and shadowed; tool strip icons on quiet plates; **exactly one
  control reads as active**; press sinks.
- **The feel bar is the gate** — if a screenshot still reads as a bag of
  saturated pills, it is not done.

## 7. BLOCKED — one ruling needed before step 2

### 7.1 `aero+hacker` vs the mock · **needs your word**

You said *"i agreed v3 but as aero hacker."* Measured against the mock, those
conflict:

- the mock casts **hard offset shadows on every surface** = `hard-cast`;
- **`aero` forbids `hard-cast`** (`matrix.json:95`).

Three ways out, and I will not pick for you:

1. **The mock wins** → the editor is `brutalist · massive · plate · hard-cast`
   (§3.3). `aero+hacker` becomes **Workbench's** dress, which is where its
   references came from anyway. *This is the reading I find most consistent
   with everything on disk.*
2. **`aero+hacker` wins** → the editor drops its hard shadows for aero's
   `bloom`/`sheen`, and the mock's shadow language is abandoned.
3. **`aero` is amended** to allow `hard-cast` — cheap in data, but it dissolves
   what makes aero *aero* (§3.1 of `coldlight-matrix.md` already wrestles with
   aero vs the no-blur law).

### 7.2 Palette · minor, but it must be stated

The mock's surfaces are **lighter** than v3/shipped (§2.1). You ruled *align
to v3*. §3 therefore uses **v3's surface values with the mock's structure**
(2px hairline, hard shadow, wash-on-active). If you want the mock's exact
lighter greys instead, that is the C1 decision flipping — one word, and it
becomes a second coordinate rather than a conflict.

## 8. Evidence

*(filled during execution)*
