# Plan — the coverage audit: what is authored, and what actually reaches a pixel

**Status:** PLAN, 2026-08-11. Pass 1 of the debug program
(`design-system.md` §7 instrumentation split). Operator: *"i am not using
debug properly at all… that is the whole reason you have it, to build your
overlays and tools for optimizing."*

---

## 1. Context

Every defect found in the 2026-08-09 design-system audit was **invisible**, and
each was found by reading 3,342 lines of `coldlight/src/lib.rs` by hand:

- `ResolvedStyle.anim` is resolved on every call and has **zero readers**
  app-wide;
- three of `Motion`'s five channels (`scale`, `rotation`, `clip`) never reach a
  pixel from a slide;
- the ColdLight matrix paints **exactly one role** in the whole app
  (`wiredfront.chrome`);
- **1 of 19** motion effects has a `track` — the other 18 still take the legacy
  path whose meaning is hardcoded in Rust, the exact defect
  `keyframe-engine.md` was written to kill.

None of these failed a test. None failed `cargo green`. None failed clippy.
They are all *authored data that nothing consumes* — and there is currently no
mechanism in this repo that can notice that class of defect at all.

This is the same shape as the `smoke_audit` precedent (`xtask/src/main.rs:242`):
three ADR-009 failures shipped because the gate asserted on **process success**
instead of on the **artifact**. The lesson generalises. A gate that only proves
things compile cannot see a pool that nothing draws from.

This pass builds the instrument. It is deliberately the smallest useful one: it
needs no schema change, no `Node`, no `Ctx`, and no runtime.

---

## 2. Goal / non-goals

**Goal.** `cargo xtask coverage` prints, for every authored pool in
`crates/coldlight/tokens/*.json`, how much of it is actually reachable — split
into **reach** and **depth** (§4.1) — and names every item that nothing
consumes.

**Non-goals, explicitly:**

- **Not a runtime overlay.** The in-app inspector (provenance, live motion
  channels, layer stack) depends on `Node`/`Ctx` from `design-system.md` §4 and
  belongs to a later pass. This one is static.
- **Not a gate, yet.** It must not fail `cargo green` in this pass (§4.4). We
  already know the numbers are bad; a gate that is red on arrival gets
  bypassed, and a bypassed gate is worse than no gate.
- **Not a fix.** It does not migrate one effect, bind one role, or delete one
  dead token. It measures. The fixing passes are `design-system.md` §7.3′/3″/3‴.
- **Never reads `/content`.** Operator data is gitignored and absent in CI; a
  gate that depends on it is not reproducible.

---

## 3. Ground truth (read in-session, 2026-08-11)

### 3.1 The authored pools — counted from the JSON, not remembered

| file | pool | count |
|---|---|---|
| `hues.json` | groups (`brand`,`accent`,`utility`,`surface`,`achromatic`) → `hues` | 5 groups; `HUE_COUNT` = 53 |
| `matrix.json` | ancestors · structures · materials · atmospheres | 8 · 6 · 10 · 7 (+5 `$laws`) |
| `presets.json` | ancestral · theme · motion · bleed | 25 · 5 · 3 · 1 |
| `motion.json` | effects · eases · durations | 19 · 7 · 7 |
| `traits.json` | traits | 5 (retired by §0.6, still live today) |
| `moods.json` | moods | 10 |
| `fonts.json` | fonts | 13 |
| `icons.json` | icons | 61 |
| `fluid.json` | groups | 5 (`FLUID_TOKEN_COUNT` = 36) |
| `roles.json` | default roles · surfaces | 4 · 3 |

### 3.2 The generated const naming is deterministic — so static reach is provable

`build.rs` writes consts through one `ident(name).to_uppercase()` path:

- `build.rs:855` → `DUR_{IDENT}` · `:873` → `EASE_{IDENT}` · `:900,993` →
  `{IDENT}` for effects · `:1088` → `R_{IDENT}` for ramps.

So `enter-rise` → `cl::motion::ENTER_RISE`, and a grep for
`cl::motion::[A-Z_]*` is a **sound** reach test for that pool.

### 3.3 The finding that shaped the design — reach ≠ depth

Live references, counted this session:

```
17 of 19 effects are statically referenced   (missing: chart-fade, ink-bleed)
 1 of 19 effects has a `track`               (enter-bloom only)
```

**A reach-only audit prints "effects 17/19 ✔" and misses the real defect
entirely.** That is precisely the lying-inspector failure mode
(`design-system.md` §0.6 discussion): an instrument that is confidently wrong is
worse than no instrument, because it will be trusted.

Therefore the audit measures **two independent axes** (§4.1).

### 3.4 A grep is not a parser — one false positive already found

`grep -o "cl::motion::[A-Z_]*"` returns `cl::motion::M` — a module alias, not an
effect. The matcher must **intersect against the authored id set** rather than
accept whatever matches the shape. Anything matching the pattern but absent
from the pool is reported as `UNKNOWN REF`, which is its own useful signal.

### 3.5 The host

`crates/xtask/src/main.rs` (1,367 lines) already hosts static audits with the
exact shape needed: `lock_audit` (:155), `docs_status` (:384), `spawn_audit`
(:804), `structure_audit` (:854), `smoke_audit` (:242), dispatched from
`main` (:1328) with tasks `green · sweep · docs-status · structure ·
stage-tool-deps · fix`. `green()` (:174) runs phases in order, stopping at the
first red.

**This pass extends that host. It does not add a sibling tool.**

---

## 4. Design

### 4.1 Two axes, never conflated

| axis | question | example |
|---|---|---|
| **REACH** | does anything reference this id at all? | `enter-rise` → yes, 8 sites |
| **DEPTH** | does it use the capability the system offers? | `enter-rise` → **no track**, so it cannot express per-channel motion |

Depth is **per-pool** and each rule is authored explicitly — there is no generic
notion of depth:

| pool | depth rule |
|---|---|
| effects | has a `track` (else legacy hardcoded-meaning path) |
| ancestral presets | its `coord` is bound by at least one `roles.json` entry |
| roles | the binding is a `coordinate` (not a retired `trait` ref) |
| hues | reached by a `Coordinate` resolve, not only by the 6-variant enum |
| matrix facet values | appears in ≥1 authored coordinate |
| moods · fonts · icons · fluid | reach only — no depth rule; stated as such |

A pool with no depth rule prints `depth: n/a`, never a fake ✔.

### 4.2 Three verdicts per item — and the honest third one

| verdict | meaning |
|---|---|
| `REACHED` | a proven static reference (const path, or an id in another token file) |
| `UNPROVABLE` | reachable **only** through a dynamic lookup — `preset(id)`, `theme_preset(id)`, `trait_def(id)`, `step_on(hue, step)` — where the id is computed at runtime |
| `DEAD` | zero references of any kind |

`UNPROVABLE` is not a hedge; it is the correctness requirement. Calling a
dynamically-reached token "dead" would be the instrument lying. The audit
enumerates the dynamic entry points it knows about and reports how many items
fall behind each, so the unprovable set is itself visible and shrinkable.

### 4.3 What counts as a reference

1. **Const path** — `cl::motion::ENTER_RISE`, `cl::R_TEAL_ACCENT`, intersected
   against the authored id set (§3.4).
2. **Token-to-token** — an id appearing as a value in another `tokens/*.json`
   (e.g. `roles.json` → a trait/coordinate id; `presets.json` → an ancestor).
3. **In-repo authored documents** — seed `slide.json` / `theme.json` under the
   repo's own seed tree. **Never `/content`** (§2).

### 4.4 Output, and why it does not fail the build

Standalone `cargo xtask coverage` prints the full table. `green()` prints a
**one-line-per-pool summary and always returns Ok** in this pass.

Rationale, stated plainly: the numbers are already bad (1/19 depth on effects,
1 role bound of 7). A phase that is red the day it lands teaches the operator to
skip it. The ratchet — fail when `DEAD` count *increases* against a checked-in
baseline — is pass 2, once §7.3′/3″/3‴ have moved the numbers.

```
── coverage: authored vs reached ─────────────
effects      reach 17/19   depth  1/19   dead: chart-fade, ink-bleed
roles         bound 1/7    depth  1/1    (6 still trait refs)
ancestral    reach 25/25   depth  1/25
hues         reach 53/53   depth  6/53   (6-variant enum only)
...
```

---

## 5. Boundaries

- **`crates/xtask/src/main.rs` only**, plus one new task arm in `main()` and
  one call in `green()`. No new crate. No change to `coldlight`, `doc`, or any
  renderer.
- **Read-only.** The audit opens files and prints. It writes nothing.
- **No new dependency.** `serde_json` is already an xtask dep (`DependencySources`
  at :437 uses serde); token parsing reuses it.
- **Structure law:** no new crate ⇒ no `architecture.md` amendment and no
  change to the xtask audit table. *(To be re-verified at implementation — if a
  new crate is somehow needed, both are amended in the SAME commit.)*

---

## 6. Proof

1. `cargo xtask coverage` prints the table.

   **§3.3's hand-derived prediction was WRONG, and the audit is right.** This
   is recorded rather than quietly reconciled, because it is the first
   evidence that the instrument is worth having.

   Predicted: `reach 17/19`, with `chart-fade` and `ink-bleed` dead.
   Actual: **`reach 18/19, depth 1/19`**, `chart-fade` UNPROVABLE, nothing
   dead.

   The hand count grepped `cl::motion::[A-Z_]*` — **const paths only**. It
   could not see `motion::effect("ink-bleed")` at
   `crates/coldlight/src/lib.rs:2936`, a by-name lookup with a string literal.
   `ink-bleed` was never dead. And `chart-fade`, which genuinely appears
   nowhere, is `UNPROVABLE` rather than `DEAD` because `effect(` is elsewhere
   called with a computed id — exactly the §4.2 rule doing its job on the
   first run.

   The depth number — **1/19** — was predicted correctly and is the finding
   that matters.
2. **The false-positive guard:** `cl::motion::M` does **not** appear as an
   effect; it is reported under `UNKNOWN REF` (§3.4).
3. **The unprovable guard:** a unit test feeds a pool item reachable only via
   `preset(id)` and asserts the verdict is `UNPROVABLE`, never `DEAD`.
4. `cargo green` passes end to end and prints the summary block; the phase
   returns Ok even with dead items present (§4.4).
5. No `/content` path is opened — asserted by test.

---

## 7. Commit plan

One commit, one PR.

1. `feat(xtask): the coverage audit — authored pools vs what reaches a pixel`
   — the `coverage_audit()` fn, the `coverage` task arm, the summary line in
   `green()`, and the three unit tests from §6.

Sequencing against work already in flight:

- **First**, land `docs/design/design-system.md` (the 2026-08-09 revision,
  currently uncommitted) — this plan cites §0.6 and §7 as law.
- **Then** rebase `feat/coldlight-53-hues` onto current main (proven
  conflict-free via `git merge-tree`) and apply the §0.6 vocabulary fix to its
  fabric signals (`entity → node`, `CardTraitChanged → CardAxisChanged`) before
  its PR, because a published signal name is a wire format.
- **Then** this pass.

---

## 8. Evidence

### 8.1 The first run — `cargo xtask coverage`, 2026-08-11

```
── coverage: authored vs reached ─────────────
  effects              reach  18/19   depth   1/19   unprovable 1   (chart-fade)
  eases                reach   4/7    depth   n/a    dead: bounce, dwell, spring-crisp
  durations            reach   5/7    depth   n/a    dead: fast, settle
  hues                 reach  46/53   depth  45/53   unprovable 7
  ancestors            reach   8/8    depth   8/8
  structures           reach   6/6    depth   6/6
  materials            reach  10/10   depth  10/10
  atmospheres          reach   7/7    depth   7/7
  presets (ancestral)  reach   6/25   depth   1/25   unprovable 19
  presets (theme)      reach   1/5    depth   n/a    unprovable 4
  presets (motion)     reach   0/3    depth   n/a    unprovable 3
  presets (bleed)      reach   1/1    depth   n/a
  traits               reach   5/5    depth   n/a
  moods                reach  10/10   depth   n/a
  fonts                reach  11/13   depth   n/a    unprovable 2
  icons                reach  61/61   depth   n/a
  unknown refs        M
coverage: 16 pools, 5 dead (reporting only — the ratchet is pass 2)
```

**What it confirms, mechanically, that previously took a hand-read:**

- **`presets (ancestral)` — reach 6/25, depth 1/25.** This IS "the matrix
  paints exactly one role," now a number that prints on every gate run.
- **`effects` — depth 1/19.** The keyframe migration's true state.
- **`eases`/`durations` — 5 genuinely dead tokens** nobody had named:
  `bounce`, `dwell`, `spring-crisp`, `fast`, `settle`.
- **`unknown refs: M`** — the §3.4 false positive, correctly quarantined
  instead of being counted as an effect.
- **The matrix facets are all at 8/8 · 6/6 · 10/10 · 7/7** — every authored
  facet value stands in at least one coordinate. The *facets* are healthy;
  it is the *bindings* that are not. Reach-only would have blurred those two
  into one number.

### 8.2 Two bugs the first run found in the audit itself

Recorded because both were the audit committing the defect it exists to catch:

1. **Six pools printed `0/0` with a confident tick.** `object_keys` handled
   only id-keyed objects; every pool except `motion.json`'s three is an array
   of `{id, …}` rows. Fixed to accept both shapes (`name` for moods).
2. **`unknown refs` reported `NAME`** — read out of xtask's *own doc comment*
   explaining the scanner. The corpus now skips `crates/xtask`: an instrument
   that reads its own documentation as evidence is measuring the wrong thing.

### 8.3 Tests

`crates/xtask/src/main.rs`, `mod coverage_tests`:

- `const_names_mirror_the_design_build` — pins the derivation to
  `coldlight/build.rs:84`; if that changes, every reach number would silently
  go to zero.
- `a_module_alias_is_never_mistaken_for_an_effect` — the §3.4 guard.
- `a_computed_lookup_is_unprovable_not_dead` — the §4.2 guard.

### 8.4 Gate

`cargo green` ✔ with the coverage phase wired between `structure_audit` and
clippy, reporting and not gating (§4.4).
