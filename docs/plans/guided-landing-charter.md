# THE GUIDED-LANDING CHARTER — organize-on-landing across both houses

Status: DRAFT (operator-commissioned 2026-07-26; awaiting ratification).
This charter is the north star for the guided "land it, organized" QoL
program. It governs two executable passes — **W9** (wiredFRONT capture
landing) and **W10** (Workbench finished-slide association) — which share
ONE model and ONE reusable component. W9/W10 each carry their own
7-section + §8 plan; this charter is what they must not drift from.

Ground-truthed in-session (2026-07-26) via four reads of the real tree:
the ViziWiz capture spine, the three-ring root + import seam, the
composer (wiredFRONT) editor intake, and the `wiredfront-experience.md`
UX gate. File:line evidence lives in the pass plans.

## 1. The operator's vision (verbatim intent)

> "when I take a picture it should have a workflow that guides me to
> effortlessly save it, and get it saved so its properly organized in the
> raw assets for editing and post processing inside wiredfront editor. I
> should be able to have this properly for session replays and snipping
> imgs or videos and the guide tool."

> "there will be a separate workflow that will be the same concept for
> workbench when we have the finished slide to associate with an episode
> or presentation… build this with the flexibility to evolve the internal
> organization and structure for scale and growth but guided and ensure
> the UI produces senior dev clean code."

> "anything via pixvix needs to allow the inbox/library not become a pure
> dumping ground of raw and be organized there."

> "Wiredfront is all about getting the finished product to be then
> combined with the actual component and slide creation where we have now
> interactive animations… that i cannot achieve with video post
> processing or image post processing."

## 2. The two houses (the distinction the operator keeps stressing)

- **wiredFRONT = PREP house = POST-PROCESSING.** Video/image editing —
  cut, color, stitch — over raw captured media. Output: a FINISHED
  PRODUCT. The ceiling of what pixel post-processing achieves.
- **Workbench = BUILD house = COME-TO-LIFE.** The finished product is
  combined with real interactive components + slide creation: interactive
  animations, live behavior — what post-processing CANNOT do. Output: a
  slide/component associated with an episode or presentation.

The pipeline:

```
 PixVix capture ──▶ [LAND ORGANIZED] ──▶ wiredFRONT post-process ──▶ finished product
                    (W9: this pass)        (existing editor)              │
                                                                          ▼
                       episode / presentation ◀── [ASSOCIATE] ◀── Workbench BUILD
                                                   (W10: sibling pass)   (interactive slides)
```

Both `[LAND ORGANIZED]` and `[ASSOCIATE]` are the SAME guided concept:
a policy-driven, organized handoff that never dumps.

## 3. The shared model — the Landing Policy

One typed model, resolved once, reused by every vehicle in both houses.

```
LandingPolicy {
    mode:    Auto | Ask,        // the operator TOGGLE
    pattern: LandingPattern {
        name_template,          // meaningful names — never bare "pix"/"vix"
        destination,            // bucket / route (library group, later show/episode)
        tags,                   // default tags applied on landing
        // extensible: future keys added here, every vehicle inherits
    },
}
```

- **Auto mode**: the artifact files itself SILENTLY by the pattern.
  Effortless — rapid capture is never interrupted.
- **Ask mode**: a blocking Save card on every capture, **pre-filled from
  the same pattern** — one-Enter fast even when it asks.
- The pattern SEEDS both modes, so there is exactly ONE code path and ONE
  UI component. This is the senior-clean core: no per-vehicle copy-paste,
  no bespoke card per capture kind (today the guide vehicle has a bespoke
  card; every other vehicle has none — W9 replaces both with the shared
  component).
- **Scope now**: one GLOBAL toggle + one GLOBAL pattern. **Designed to
  grow** to per-vehicle / per-kind patterns — the policy is resolved
  through a keyed lookup that currently returns the global, so widening it
  later is additive, not a rewrite.
- Persisted operator state (not source, not content) → lives in
  `.userdata/` (Ring 0, STATE).

## 4. The organization law (the anti-dumping-ground guarantee)

Anything that lands via PixVix MUST land organized. Concretely:

- **Meaningful names** from the template — never `pix.png` / `pix-2.png`
  / `vix.mp4` flat.
- **Real folders on disk** (RESOLVED, operator 2026-07-26):
  `content/inbox/<bucket>/<kind>/<name>.<ext>` — by working-set bucket,
  then by kind (`image`/`video`/`session`/`guide`), meaningful name. The
  index carries `tags` + bucket so the library groups mirror the folders.
- **Buckets are working-sets with SUGGESTED seed values** — the picker
  offers a starter list (show categories + a dated `shoot-…` + `scratch`),
  any new name is just a kebab-forced folder. See `w9-capture-landing.md`
  §3d.
- **The library is a curated store, not an intake dump.** The inbox
  remains the machine-tier landing, but with organization applied on the
  way in, it reads as an organized library, not a pile.
- The final taxonomy (bucket scheme, folder-vs-index-only, default
  templates) is OPERATOR-RATIFIABLE in the W9 plan — this charter fixes
  the LAW (organize-on-landing), not the exact buckets.

## 5. The clean-UI mandate

- ONE reusable `LandingCard` component driven by `LandingPolicy` — used by
  every vehicle and (in W10) by the slide-association flow.
- Typed taxonomy, not scattered string literals. Today capture kinds are
  free strings (`"image"`, `"video"`, `"session"`, `"guide"`, `"web"`)
  spread across call sites — W9 centralizes them so a new vehicle can't
  drift.
- Every control tooltips in plain words; every hit target is the whole
  chip/card (the tooltip + touch laws from the experience gate).
- Sources never mutated; captures/imports COPY; peer windows fail
  honestly. No `#[allow]`.

## 6. Pass map

- **W9 — wiredFRONT capture landing.** Unify all PixVix palette vehicles
  (stills full/window/region/free, video, session, guide) behind the
  Landing Policy + LandingCard, organize-on-landing, index+signal
  write-through, editor auto-pickup (already works). Browser capture
  on-spine is a scoped fast-follow (W9.1), not core W9. Plan:
  `docs/plans/w9-capture-landing.md`.
- **W10 — Workbench finished-slide association.** The sibling flow: a
  finished slide → guided ASSOCIATE with an episode/presentation, same
  policy/component, organizing into the content confirmed-library under
  the right show/episode (with live episode budgets, and no orphan
  slides). Designed here, built after W9. Plan:
  `docs/plans/w10-slide-association.md` (AUTHORED 2026-07-26).

**Resolved by W10 (the shared-module decision):** `landing.rs` lives in a
SHARED crate, not viziwiz-local, because the `Destination` type must
resolve for BOTH houses — `Destination::LibraryBucket{…}` (W9, PREP) and
`Destination::Episode{show,episode,chapter,slot}` (W10, BUILD) — off ONE
`LandingPolicy` + ONE `LandingCard`. W9 §4 step 1 must build it shared.

## 7. What this charter is NOT

- NOT an assets/-ring migration. Intake stays `content/inbox`; the
  `assets/` WORKING ring (ADR-014) is out of scope — the goal is
  organization, not relocation.
- NOT a change to how the editor reads footage (index-driven; unchanged).
- NOT the experience-inventory gate (still unauthored; acknowledged, not
  depended on).

## 8. Gate alignment

The program lives in the WiredFRONT experience lane. Each pass must move a
row of `wiredfront-experience.md` §5: W9 moves the media-bin-thumbnails
(organize), Library→editor, and capture-signal rows; it honors the touch,
tooltip, two-libraries, sources-COPY, and proof-shot laws.
