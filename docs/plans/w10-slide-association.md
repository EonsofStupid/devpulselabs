# W10 — Slide association: a finished slide, guided into an episode/presentation

Governed by `docs/plans/guided-landing-charter.md` (the BUILD-house sibling
of W9). Gate: `docs/plans/wiredfront-experience.md`. Builds on — never
re-does — `x3-keep-associate.md` (the content tree + media keep/associate),
`w4-editor.md` (the slide editor), `w8-faithfulness.md` +
`docs/design/signal-doctrine.md` (the signal domains). Ground-truthed
in-session 2026-07-26.

## 1. Context

The operator's ask: *"a separate workflow that will be the same concept for
workbench when we have the finished slide to associate with an episode or
presentation… guided and ensure the UI produces senior dev clean code."*
And the anti-dump law applies here too: a finished slide must land in the
right place, not accumulate as an orphan.

The distinction the operator stresses: **Workbench is where the finished
product COMES TO LIFE** — the slide carries real interactive components /
animations that video/image post-processing cannot bake in. W10 is the
guided, organized handoff of such a finished slide INTO an episode (or a
presentation, which in this codebase is an episode run through the deck).

Gate rows moved (`wiredfront-experience.md` §5): the Library→surface and
signals-live rows, plus it lights up the currently-placeholder Studio
compose surface.

## 2. Current state (ground-truthed 2026-07-26)

- **A "slide" is a folder** holding `slide.json` — `crates/doc/src/lib.rs:22-43`,
  `FILE="slide.json"` `:124`; style fields are refs, never values.
  Episode slides live at `content/<show>/<episode>/<slide>/slide.json`.
- **Episode model**: `Episode { v, id, title, chapters: Vec<Chapter> }`;
  `Chapter { id, title, slides: Vec<String> }` — each string is a slide
  SUBFOLDER NAME — `doc/src/lib.rs:337-351`. Content tree
  `content/<show>/<episode>/` (episode.json · look.json · assets · slide
  folders) — `doc/src/lib.rs:786-792`, `x3-keep-associate.md:8-13`.
- **"Presentation" is not a distinct type** — a presentation = an episode
  run through the Present hub's deck (`DeckEvent::Beat`,
  `fabric/src/lib.rs:74-83`; `crates/hubs/present`). W10 treats
  episode/presentation as one destination: an Episode.
- **THE GAP — no single-slide→episode gesture.** The only authoring path
  into content is `dpl_doc::clone_episode` (whole seed episode copy) —
  `doc/src/lib.rs:891-910`; surfaced as the Workbench Episodes "copy to my
  shows" chip (`workbench/src/lib.rs:471-475`). To add ONE finished slide
  to an existing episode you must hand-edit `episode.json`'s
  `chapters[].slides`. `save_episode` EXISTS (`doc/src/lib.rs:374-376`) but
  has **no production caller** — only tests (`deck` `:777`, `present`
  `:496`, `doc` `:1139`, all `#[cfg(test)]`).
- **Studio subroute is a placeholder** — `workbench/src/lib.rs:311-320`
  falls to `dpl_hub::placeholder`; its declared job is exactly "compose
  episodes… onto sequenced slides" (`:48-51`). This is the empty home the
  handoff belongs in.
- **No "finished" flag** on a slide (`Slide`, `doc/src/lib.rs:22-43`). The
  only quality gate is `dpl_doc::validate(&slide)` → zero `Issue`s
  (`:224-300`). Readiness must be DEFINED (validate-clean + operator
  confirm).
- **Budgets exist but are UNENFORCED live** — `validate_episode`
  (`doc/src/lib.rs:449-551`) checks structure only; show-kind budgets are
  `TODO(doctrine)` (`:446-448`). The numeric rules live only in PORT:
  **exactly 3 chapters**, **15–24 slides**, **≤3 interactive** slides
  (`PORT/devpulse labs/src/domains/content/services/content-rules.service.ts:19-21`).
  Interactive count comes from the registry `TemplateClass::Interactive`
  (`crates/slides/src/registry.rs:24-43`; interactive templates
  `drag_match/reveal_list/predict_rank/cost_meter/choice_poll/quiz_panel`).
- **Dumping-ground risk is real & anticipated** — the doctor already has an
  "orphan slide folder — not in any chapter" check (`doc/src/lib.rs:537-548`).
  Finished bespoke slides have nowhere organized to land today.
- **No BUILD-side signal.** `dpl_editor::Editor::save` writes silently
  (`editor/src/lib.rs:103-105`); `clone_episode` emits nothing. There is no
  `SlideSaved`/`EpisodeChanged` event — the fabric has shell/deck/capture/
  jobs domains only (`fabric/src/lib.rs`). A guided flow needs a new
  content-domain signal (signal-doctrine law 1: one enum + one channel +
  two methods).

## 3. Target design

**3a. Shared model, extended `Destination` (the reason `landing.rs` is
SHARED, not viziwiz-local).** The charter's `LandingPolicy` /
`LandingPattern` / `LandingCard` are reused verbatim; the `Destination`
enum grows a BUILD variant so ONE typed model serves both houses:

```
enum Destination {
    LibraryBucket { bucket },                              // W9 (PREP)
    Episode { show, episode, chapter, slot: Option<usize> },// W10 (BUILD)
}
```

This is the concrete answer to the charter's open module-home question:
`landing.rs` lives in a SHARED crate because `Destination` must resolve for
both W9 and W10. `LandingCard` renders a bucket picker for the PREP variant
and an episode→chapter→slot picker for the BUILD variant, off ONE component.

**3b. The placement engine — `dpl_doc::place_slide` (greenfield, pure).**
Mirrors `clone_episode`'s directory work but for ONE slide into an EXISTING
episode:
- `place_slide(content_root, slide_dir, show, episode, chapter_id, slot)
  -> Result<PlacedSlide, String>`.
- Copies (default) or moves the slide FOLDER into
  `content/<show>/<episode>/<slide-name>/` via `copy_dir`
  (`doc/src/lib.rs:912-925`), never-clobber naming; refuses if the episode
  doesn't exist (the X3 law — associate never creates shows).
- Appends the slide-folder name to the chosen `Chapter.slides` at `slot`
  (or end) and calls the existing-but-unused `save_episode`
  (`doc/src/lib.rs:374`).
- Runs `validate_episode` after; on any `Issue`, ROLLS BACK (the write is
  transactional — the slide never half-lands, no orphan created).
- Sources are never mutated; a seed slide is COPIED (X3 discipline).

**3c. Readiness + live budgets (the "come to life", checked).** The card
gates on: `validate(&slide)` clean AND operator confirm. It SURFACES the
episode budgets LIVE as it places — chapter count, slide band (15–24), and
the **interactive budget** (≤3, counted from `TemplateClass::Interactive`).
This ports the PORT budgets into `dpl_doc` (closing the `TODO(doctrine)` at
`doc/src/lib.rs:446-448`) as WARNINGS surfaced in the card, not hard
blocks — the operator stays in control.

**3d. A new content-domain signal (signal-doctrine law 1).** Add a
`BuildEvent` domain to `dpl-fabric` — one enum, one broadcast channel,
`publish_build`/`subscribe_build`, stamped like capture/jobs:

```
enum BuildEvent { SlidePlaced { show, episode, chapter, slide },
                  EpisodeChanged { show, episode } }
```

Emitted AFTER `place_slide` commits (emit-with-mutation, law 3). Consumers:
the Workbench Episodes/Studio views invalidate and re-read; the seam's
`/signals` gains the BUILD nervous system; the activity feed shows
"slide placed · <episode>". This is the BUILD-side analog of how X3's
`keep` + `InboxChanged` announced media landing.

**3e. The Studio surface becomes the organized home.** Implement the
`studio` subroute (`workbench/src/lib.rs:311-320` placeholder) as the
compose surface: the episode's chapters × slots with the finished slide
being placed, the LandingCard for destination, and the live budget strip.

## 4. Changes, in execution order

1. **Shared `landing.rs`**: move the charter/W9 model to a shared crate;
   extend `Destination` with the `Episode` variant. (If W9 shipped it
   viziwiz-local, this step promotes it — additive.)
2. **`dpl_doc::place_slide`** + budget helpers (`chapter_count`,
   `slide_count`, `interactive_count` via the registry) + transactional
   rollback; unit tests (places into a chapter; refuses missing episode;
   rolls back on invalid; copy leaves the seed; never-clobber).
3. **`dpl-fabric` `BuildEvent`** domain (enum + channel + publish/subscribe
   + stamp), per signal-doctrine; fabric test.
4. **Seam envelope**: `build_json` for `/signals` + session log, pinned by
   test (law 4 — camelCase, exhaustive).
5. **Workbench Studio** subroute implemented: the compose surface + the
   shared `LandingCard` (Episode destination) + live budget strip; emits
   `place_slide` → `BuildEvent`.
6. **Activity feed line** for `BuildEvent` in the shell inspector.

## 5. Boundaries (what W10 does NOT touch)

- **No W9 capture work** — that pass owns PREP landing.
- **No new "presentation" type** — presentation = episode in Present mode.
- **Budgets surface as warnings, not hard blocks** — no auto-rejection.
- **Guides never auto-enter episodes** — the X3 law holds
  (`x3-keep-associate.md:104`); placement is an explicit slide gesture.
- **No slide CREATE/authoring** (add/delete blocks is the editor's own
  lane) — W10 places FINISHED slides, it does not author them.
- **Seeds/sources never mutated**; slide COPY by default; no `#[allow]`.

## 6. Verification

- `cargo green` (fmt + clippy -D warnings + tests + release exe).
- Unit: `place_slide` (place/refuse/rollback/copy/never-clobber); budget
  counters; `BuildEvent` round-trip; seam envelope pinned.
- Drills (real exe, SEQUENTIAL): place a finished seed slide into a cloned
  content episode → it appears in the chapter, `validate_episode` clean,
  `BuildEvent::SlidePlaced` on `/signals`, the Present deck walks it; an
  over-budget place shows the warning strip but still lets the operator
  proceed; an invalid slide is refused with no orphan left behind.
- Proof-shots at REAL window sizes (`--window WxH`): the Studio compose
  surface + budget strip; the episode gaining the slide.

## 7. Commit plan

Branch `feat/w10-slide-association`. Commits: (1) shared landing model +
`Destination::Episode`; (2) `dpl_doc::place_slide` + budgets + tests;
(3) `BuildEvent` domain + seam envelope; (4) Workbench Studio surface +
LandingCard + budget strip; (5) activity-feed line. PR → gate → merge →
exe rebuild → STATE.

## 8. Evidence

_(Filled at execution on the Windows checkout `C:\localdev\devpulselabs`.
`cargo green`, sequential exe drills, and real-window-size proof-shots run
there; STATE.md updated on land.)_
