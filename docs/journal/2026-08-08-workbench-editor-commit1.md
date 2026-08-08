# Workbench Editor · Commit 1 + the 53 hues — journal (2026-08-08)

Branch `feat/coldlight-53-hues`, off `main`, **5 commits, each `cargo green` ✔**,
unpushed. Entry point for the next context: this file, then
`docs/plans/workbench-editor.md` §3.9 and §3.10 (both amendments written today).

## Done so far

| commit | what |
|---|---|
| `477d643` | the matrix reaches all **53** hues, not six |
| `1222daf` | the composition surface folded into the ratified preset mock + §3.9 |
| `0345422` | **Commit 1 of §7** — the five card signals |
| `cb63044` | §3.10 — Commit 2 ground-truthed, three plan claims corrected |
| `e3c777e` | repair: a blind `sed` had spliced a correction mid-sentence |

### The 53 hues (`477d643`)

`Tone` is a six-variant enum in front of a 53-hue pool, so the `colour` knob —
the one trait every preset MUST expose — could reach 11% of the authored
colours. That is why every surface is amber or cyan.

The six were never a limit of the ENGINE: `Tone::hue_id()` already returned a
`&'static str` and `step_on(hue, step)` already looked it up by name. Added
`ColorRef::resolve_on(hue)` and `matrix::resolve_on(&c, hue, state)`; the
`Tone` forms delegate to them. **Additive** — `Tone` untouched, so the 369
`Tone::*` literals across 66 files keep their exact pixels, pinned by
`the_tone_path_and_the_hue_path_agree`.

Operator's `#097d71` is `teal-accent` @700 (`#007d78`, distance 11.4) — it was
already authored, it just could not be selected.

**Finding, recorded because a test first read it as a bug:** a material whose
fill is a SURFACE constant is hue-INDEPENDENT by design (`paper` is
`{surface: RAISED}`); one whose fill is a TONE STEP follows the hue (`raw` is
`{tone: 400}`). The test asserts both directions.

### Commit 1 — the signals (`0345422`)

One signal per step of the §3.0 flow: `CardStarted` · `CardAdded` ·
`CardStyled` · `CardTraitChanged` · `CardSaved`. Five files exactly as §3.7's
blast radius predicted. Pre-release ruling §8.1.2 — no shims, no aliases.

Removed a lie shaped like data: `CardAuthored` carried `show`/`episode` and the
publisher passed `String::new()` for both, because neither is a fact when a
draft is minted. `CardStarted` carries only the draft id.

The Editor does not yet publish four of the five — it cannot until the surface
exists (Commit 5). They are wired end to end (enum → dispatch → wire shape →
ledger row → catalog id), so Commit 5 adds call sites, not plumbing.

## Exact next step — Commit 2, `SCHEMA_V` 2→3

**Read `docs/plans/workbench-editor.md` §3.10 first.** Every file was read in
full today; §3.10 records what the plan gets wrong. Work plan is §3.10.5.

Shape (§3.10.4): `Slide` gains `title` · `preset` · `tone` · `recipe`
(reserved); `Block::Text` gains `preset`/`tone` and `size` goes
required→optional; `Block::Media` gains `preset`/`tone`; `Block::Quote` is new;
`resolve()` (`doc/src/lib.rs:764`, 20 lines, pure) gains the middle level
`Theme → preset → entity`.

## Landmines

1. **EIGHT authored documents, not four** (the plan says four).
   `seeds/slides/static/{headline-clean,stat-callout,terminal-retro,title-neon}`
   plus `seeds/episodes/how-ai-works/{headline,title,stat,terminal}` plus
   `how-ai-works/theme.json`. The episode set is invisible to
   `list_template_docs()`, which walks only `seeds/slides`. Miss it and the
   seeded episode every `--clone-seed` proof uses is stranded.

2. **Every authored document is `"v": 1`** against `SCHEMA_V = 2`. The only
   check is `doc/src/lib.rs:169` — `if slide.v > SCHEMA_V` — so older is
   accepted and never migrated. The repo has run a version behind since W11 and
   nothing noticed. **v3 needs no `migrate()`**, only a test pinning it.

3. **The `size` hazard is on the WRITE path, not the read path.** Serde turns a
   present `"size"` into `Some(..)` — which IS the override — for free, and
   both render sites already fall back (`slides/src/document.rs:375,443`). But
   `dpl-editor` stamps `size` unconditionally (`editor/src/lib.rs:894,912`,
   struct field `:51`). Leave that and every opened card silently acquires an
   override and the preset can never own size again.

4. **`Theme` needs `preset`/`tone` too** (`doc/src/lib.rs:700`, version-checked
   at `:739`) or a show-level preset is unexpressible — a hole at the top of
   the cascade. The plan does not name this.

5. **Do not touch `crates/editor`'s role** — §5 says `dpl-editor` is the engine
   behind ADD → Text and is not replaced. Item 3 above is a narrow write-path
   fix inside it, not a rewrite.

## Open — operator decisions, blocking

- **§3.9.6 — the knob registry is wrong in BOTH documents, and it blocks
  Commit 4.** §3.1 lists `fit` and omits `treatment`; `editor-presets-mock.html`
  lists `treatment` and omits `fit`; the 45 presets use both (all 10 picture
  presets expose `fit`, 15 expose `treatment`). It is **ten** knobs. The
  per-entity build check is decided by exactly that table.
- **`treatment` on the operator's tray?** §3.2 rules treatments are "machinery
  a layman must never see", yet 15 presets expose it. Both cannot stand.
- **§3.9.5 — runtime-authored presets.** Recommendation standing unless
  corrected: Advanced ships as mix-and-match first; authoring is its own plan.
- **§8.3 #6 — `motion` means two things.** Running on the recommendation (the
  knob is `Slide.enter`; the background's field becomes `drift`).

## Verify commands

```
cargo green                          # all six phases
cargo test -p dpl-coldlight matrix   # 12, incl. the 53-hue reach
cargo test -p dpl-doc                # the schema layer
git log --oneline origin/main..HEAD  # the five commits
```

## Notes for whoever picks this up

- `git branch -a` and `gh pr list` BEFORE authoring anything. Four plan
  documents were written in this session that duplicated already-ratified ones,
  purely because that check was skipped.
- Extend the document that exists; do not add a sibling. Three mocks were
  written and then deleted today for exactly this reason — the composition
  surface now lives INSIDE `editor-presets-mock.html`.
- Proof shots in this repo **cannot be pixel-compared** — two runs of the same
  binary differ by ~1.13M of 3.07M subpixels (wall-clock animation, arbitrary
  capture phase). Recorded in `effects-motion-axes.md` §7.
- `resolve()` was measured before optimising: **86 ns/call, 34 µs for 400
  entities against a 16,600 µs frame budget.** Caching it is premature; that
  item came off my own list and was dropped.
