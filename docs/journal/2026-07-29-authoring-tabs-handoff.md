# Authoring lane — handoff for the remaining card tabs

**2026-07-29.** Written at a compact boundary. Read this, then the POA&M, and
you can implement without re-deriving anything.

## Read in this order

1. `docs/plans/authoring-lane-poam.md` — the lane's one page. Goal,
   milestones, done, remaining, and the six references (R1–R6).
2. `docs/ops/references/authoring-mock-a3db4aee.html` — **R1**, the ratified
   mock, committed to the repo. Open it in a browser.
3. `docs/plans/author-tab-quote.md` and `docs/plans/author-tab-picture.md` —
   the two passes to implement. Both are ground-truthed and ready.

## Done so far

M1–M4, M5b and M6 are closed. M5 is 3 of 7 tabs.

| Pass | PR |
|---|---|
| Ingredient ladder · carousel · overlay editor | #135 · #136 · #141 |
| Container filmstrip · thumbnail mode | #146 · #147 |
| Background tab · Movement tab | #149 · #151 |
| The lane POA&M (+ the mock attached) | #152 |
| The overlay's live pane (M5b) | #155 |

Infrastructure: #144 restored the Linux CI lane. The **merge-only-on-double-green**
law (#145) came out of two incidents where PRs merged mid-CI.

## Exact next step

**Implement `author-tab-quote.md`, then `author-tab-picture.md`**, each as its
own PR, following that plan's §5 work plan step by step.

Neither is blocked. Both plans are complete through §7; only §8 is empty,
and filling §8 is part of the pass.

## The operator's ratified decision, absorbed

> **A quote is a convention over text blocks** — not a new `Block` kind.

This is right and the plan explains why: block ids are *already* semantic
(`eyebrow`/`title`/`subtitle`, `label`/`number`/`caption`), and **`document.rs`
never reads `slide.template`** — it flows the blocks and sizes each from its own
ref. A `Block::Quote` would have cost renderer work, a schema bump and a
degrade path for nothing.

## Landmines

- **Merge only on double-green** (#145). `gh pr checks <n> --watch` until BOTH
  `green-windows` and `green-linux` are COMPLETED SUCCESS. A running check is a
  red check. Windows takes ~20 min and queues behind other lanes.
- **`docs/STATE.md` collides constantly.** Three lanes are writing to `main`.
  It conflicted twice today; resolve by **keeping both entries**, newest first.
  Never drop another lane's.
- **`gh pr list` before authoring any fix.** Two sessions independently wrote
  the same #144 fix today.
- **This work happens in a worktree**, `/home/hades/Projects/devpulselabs-lane`,
  because the main checkout had a live session in it. #145's estate-hygiene
  note wants stale worktrees pruned — prune this one when the lane is done.
- **Nested egui layouts do not work in `editor_modal`.** `horizontal_top` +
  `allocate_ui_with_layout` allocated three columns and painted nothing.
  Use explicit rects, the idiom the carousel uses. And `ui.new_child`, not
  `ui.scope_builder`, for a pane inside an already-allocated band — a scope
  claims height on top and walks the footer into the band.
- **A kittest over a live document must use `run_steps`, not `run`.** The mood
  blend never rests, so `run()` exceeds `max_steps`. That is the preview being
  live, not a bug.
- **Painted controls need `Response::widget_info`.** The Background swatches
  carried no accessibility name until a test could not find them — screen
  readers could not either.
- **Never pad a shelf to match the mock's number.** Three times now the mock
  promised more than the registries hold. Say the real count, add a row to the
  POA&M's authoring-debt table, and move on.

## Verify commands

```bash
cd /home/hades/Projects/devpulselabs-lane

cargo green                       # fmt · spawn · structure · clippy -D warnings · tests · release
cargo test -p dpl-hub-workbench --lib

# Real-exe proof. Sequential runs. Scratch content root — never /content.
S=/tmp/dpl-proof && rm -rf $S && mkdir -p $S
./target/release/devpulselabs --content-root $S --clone-seed how-ai-works effingai ep04
./target/release/devpulselabs --content-root $S --route workbench/author \
  --author-container episode --card-tab quote \
  --window 1280x800 --shot-frame 60 --screenshot docs/proof/author-tab-quote-1280x800.png
```

Harness seams this lane added: `--author-container <kind>` · `--author-draft <n>`
· `--card-tab <slug>` (reopens the newest inbox card, mints only if none) ·
`--lib-section <slug>` · `--author-editor`. All are in CLAUDE.md's list, which
the structure audit checks **bidirectionally**.

## After those two tabs

**On tap** is genuinely blocked: no `Recipe` trait, no registry, no document
field, no player. All four must exist first — the POA&M says so precisely.
**Ask for help** is M10's entry point and waits for it deliberately.

So the next unblocked milestone after the tabs is **M7 — placing cards into
the container** (`dpl_doc::place_slide` exists; the filmstrip already says
"N started here, not put in yet" and means it), then **M8 — the dashboard**.

## The habit, in one line

Cite the reference in §1 · ground-truth before designing · update the POA&M in
the same PR as the work · journal only at a boundary like this one · when the
mock and the registry disagree, say so in §8.
