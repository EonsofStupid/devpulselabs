# DevPULSE Labs — the product vision

**Status:** OPERATOR VISION CAPTURED 2026-07-30 — awaiting operator
ratification/redline
**Source:** operator statement, 2026-07-30 session (quoted verbatim in §0)
**Scope:** program level. This is the one doc that says what the product IS
and how the five pillars compose. Every pillar already has its own deeper
plan; this names the spine they hang from.
**Sits over:**
[`chat-creation-and-workflows-vision.md`](chat-creation-and-workflows-vision.md) ·
[`chat-window-product-vision.md`](chat-window-product-vision.md) ·
[`coach-operator-experience.md`](coach-operator-experience.md) ·
[`authoring-charter.md`](authoring-charter.md) ·
[`../design/workflow-doctrine.md`](../design/workflow-doctrine.md)
**Bound by:** the two-libraries law (Workbench ColdLight ingredients vs
WiredFRONT raw media) · the subscription-only / no-API-key law
([`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)) · ADR-009
(ffmpeg is a subprocess) · `/content` is never deleted by automation

---

## 0. The operator's statement (verbatim, 2026-07-30)

> WiredFront - all of my raw editing needs for generating the finished
> results to then import into my slides via the workbench editor. the chat
> system has route awareness and and an assist toggle to know that it needs
> to work at all times that can eventually be triggered naturally. We have
> the abiltiy to allow the user to log into to eventually grok chatgpt
> gemini claude subscriptions that can be then pulled into a true council we
> have he ability to have workflows where we have defined schemas and tasks
> that the ai must fulfill to properly storyboard and prepare full ideas
> into vision with the methodic we have eguiPilot that has evolved into a
> whole coaching sysdtem that allows us to point at installed applications,
> create a signal and fabric system that allows the AI to create
> instructional guides and steps live on the viewport that allows the user
> to truly learn how to use that tool to its full capabilities and have an
> optimzied version of this built into the application platform

Kept verbatim on purpose. Where this doc paraphrases, the statement wins.

---

## 1. The spine — one sentence

**Raw material becomes a finished show, and the app teaches you to make it.**

WiredFRONT does the raw editing. Its finished results import into slides
through the Workbench editor. Chat rides along, route-aware and always
willing to help. A Council of the operator's own subscribed models does the
thinking. Workflows make the thinking methodical instead of improvised. The
Coach turns the whole thing — and any other application on the machine —
into something a person can actually learn.

The pipeline is not a metaphor. It is already typed:

```rust
// crates/workflows/src/lib.rs
pub enum Surface { Wiredfront, Workbench }
```

Two surfaces, one direction of travel. WiredFRONT is where raw media is
cut; Workbench is where the cut result becomes a slide, a card, an episode.
The two-libraries law is the same boundary seen from the asset side:
generated and captured media is **raw media** (WiredFRONT), authored design
ingredients are **ColdLight ingredients** (Workbench). Nothing crosses
without passing through the import seam.

---

## 2. The five pillars

### 2.1 WiredFRONT — raw editing into finished results

> "all of my raw editing needs for generating the finished results to then
> import into my slides via the workbench editor"

The AI-driven After Effects. `crates/composer` is the WiredFRONT editor,
built to the operator-approved mock **"After Effects, panel for panel"**
(`docs/design/mocks/wiredfront-editor-mock.html`, v2.1), living at the
`wiredfront/editor` subroute. Around it: `crates/viziwiz` (capture family,
Magenta tone), `crates/engine-media` (ffmpeg subprocess per ADR-009),
`crates/comp` (the comp document), `crates/engine-capture`.

The AE-independence rungs L1–L4 are landed: concat, Lottie, alpha overlay
compositing, and blend/matte/LUT grading against the real 30-pack estate.

**The import seam is the pillar's unfinished edge — and it is one gesture
wide.** "Import into my slides via the workbench editor" is the sentence the
whole product turns on. Ground-truthed against `origin/main` this pass, the
machinery is *already built*:

- `Composer::make_slide_from_rel(rel)` is the ONE entry — an image or video
  mints a media slide, a `.pdf` explodes to page slides, anything else is an
  honest error;
- `mint_media_slide` validates the document BEFORE touching disk and writes
  crash-safe; the media block refs the artifact **in place**, never a copy;
- `dpl_doc::place_slide` copies a draft into a chapter — no-clobber, never
  moves the seed, rolls back if the episode would end up invalid;
- the harness seam `--make-slide <asset-rel>` drives the shelf's exact path.

**What is missing is the operator's gesture, not the plumbing.** The RENDERS
shelf already censuses every `inbox/*/render.mp4`, but a render row's only
action is `Act::AddRender` — *"Add this render to the timeline as footage."*
A finished render can go back into the timeline; it cannot become a slide.
The FOOTAGE rows beside it already carry a second corner action (`fx` →
`Act::AddOverlay`), so the idiom for "a row with two verbs" exists and is
established.

So the spine's joint is: **a render row needs a second verb, and the
render → slide → `place_slide` → episode chain needs one end-to-end proof.**
Nothing needs inventing; it needs connecting and witnessing.

| Piece | Status |
|---|---|
| WiredFRONT editor to the ratified mock | Landed (`crates/composer`) |
| L1–L4 AE-independence rungs | Landed (concat · Lottie · alpha overlay · blend/matte/LUT) |
| Raw-media library + external assets (copy ⇄ in place) | Landed (#60) |
| Workbench card authoring (7 tabs) | 4 of 7 (`Words · Background · Movement · A quote`); Picture open as #160 |
| Media slide minting (`make_slide_from_rel`, `--make-slide`) | Landed — image · video · pdf-explode |
| `place_slide` into an episode chapter | Landed — no-clobber, validates, rolls back |
| **A render row offering "make a slide"** | **MISSING — the shelf's only verb is "add to timeline"** |
| **Render → slide → placed, proven end to end** | **NOT PROVEN — no drill chains it** |

### 2.2 Chat — route-aware, and always willing

> "the chat system has route awareness and and an assist toggle to know that
> it needs to work at all times that can eventually be triggered naturally"

Two distinct claims, and they are at different maturities.

**Route awareness** is real: `chat.context.route`, the route-context chip in
the composer row, and a chat that can ask the shell to navigate. The next
step (already captured in the 2026-07-27 vision, §1) is chat moving from
*knowing* the route to *acting on* it — adding library elements into the
working comp through the same typed intents, never a second command
vocabulary.

**The assist toggle** landed as PR #126: the ⚡ Assist chip beside the
route-context chip, never provider- or capability-gated, with exactly one
blanket acknowledgement across all providers current and future
(`AssistPolicy { enabled, acknowledged }` → `assist-policy.json`).

**"Knows it needs to work at all times" and "eventually triggered
naturally"** is the part that is *consented for but not built*. Assist ON is
the standing permission; what it does not yet have is the ambient trigger —
assist noticing on its own that the moment calls for it, rather than waiting
to be addressed. That is the arc's next real pass, and it should be
specified before it is coded, because "triggered naturally" is exactly the
kind of phrase that becomes a guessing game in implementation.

> **KNOWN, open (from STATE.md 2026-07-29):** `chat-peer-window.toml` fails
> on `main` — `chat.context.route` reads `workbench` where the scenario
> expects `wiredfront`. Bisected against unmodified `faf5132`; the default
> landing route appears to have moved under #155/#156. Stale expectation vs
> real regression is still undecided. It sits directly on this pillar's
> route awareness and should be settled, not carried.

### 2.3 The Council — the operator's own subscriptions, deliberating

> "allow the user to log into to eventually grok chatgpt gemini claude
> subscriptions that can be then pulled into a true council"

The law first, because it constrains everything: **subscription-backed
official client surfaces only — no API keys**
([`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md), RATIFIED
2026-07-23). Codex over JSON-RPC/stdio, Claude Code's structured CLI stream,
Gemini CLI in ACP mode, Grok Build in ACP or documented headless mode. The
operator logs into the subscriptions they already pay for; DevPULSE never
holds a key.

Landed toward this: the provider roster with every provider as a one-click
row, activation paths, connected providers earning their own tab in the
title-bar strip, and the Grok authorization proofs at the repo root.

**"A true council" is the unbuilt part, and it is the interesting one.**
Today `ChatDomain::Council` exists and the sidebar honestly reads
`♜ Councils · not qualified`. A council is not a provider picker and not a
group chat — it is several subscribed models deliberating on one question
with a resolution the operator can inspect. What it needs before code:

- What a council **produces** — a synthesis, a ranked set, a dissent record?
- How disagreement is **surfaced** rather than averaged away;
- Whether members see each other's answers (debate) or answer blind (panel);
- Which providers **qualify** for membership, and what "not qualified"
  concretely means today;
- How a council result re-enters the pipeline — does it become a workflow
  input, a storyboard, a slide?

Until those are answered, "not qualified" is the honest state and should
stay on screen.

### 2.4 Workflows — schemas and tasks the AI must fulfill

> "workflows where we have defined schemas and tasks that the ai must fulfill
> to properly storyboard and prepare full ideas into vision with the methodic"

This pillar is the furthest ahead on governance and it is the reason the
product can scale past improvisation. Landed in PR #129:

- `schemas/workflow.v1.schema.json` + `schemas/storyboard.v1.schema.json` —
  strict JSON Schema 2020-12, `v` const-pinned, kebab ids, jail-safe refs;
- **vN files are append-only** — v2 lands *beside* v1, never over it;
- four `schema_pin` tests in `dpl-workflows` validate the real registered
  types **and prove refusal** (unknown kinds/fields, non-kebab ids, future
  `v`, jailbreak refs);
- `docs/design/workflow-doctrine.md` — 11 laws plus the workflow-PR
  checklist every workflow PR copies into its plan.

The phrase that matters is **"must fulfill"**. The schema is not
documentation the AI is invited to follow; it is a gate the output has to
pass, and the refusal tests are what make that true. An AI that returns a
malformed storyboard gets rejected by a test, not by a reviewer's attention.
That is what "methodic" buys.

Named workflows from the 2026-07-27 vision, in the operator's stated order:
storyboarding (component → storyboard → full show/episode), how-to /
explainer, LinkedIn PDF carousel, episode (stitch · ad insert · post).

**Open:** the storyboard schema is pinned, but the *task* vocabulary — what
the AI is actually required to produce at each stage, and how a stage
reports fulfilment — is thinner than the document schema. `Stage`/
`StageKind` exist; the fulfilment contract per stage is where the next
specification effort belongs.

### 2.5 The Coach — EguiPilot evolved into teaching

> "eguiPilot that has evolved into a whole coaching sysdtem that allows us to
> point at installed applications, create a signal and fabric system that
> allows the AI to create instructional guides and steps live on the
> viewport that allows the user to truly learn how to use that tool to its
> full capabilities and have an optimzied version of this built into the
> application platform"

The most ambitious pillar, and the one with the most proof already on the
board. The operator points at any installed application; DevPULSE mints a
**semantic fabric** from accessibility streams — *not* screenshots — and the
AI mounts typed touchpoints that paint on the live viewport and wait for
real presses. Refusals teach with tips instead of failing silently.

The tier ladder is law: **Tier A** semantic tree (AT-SPI / UIA / AccessKit)
is primary and proven; Tier B first-party app APIs later; **Tier C bounded
vision is a last resort**. Screenshots are Tier C. The calculator proof is
pure Tier A.

The last clause — *"an optimized version of this built into the application
platform"* — is the two-surface rule already ratified in the coach spine:
**Operator Coach** (layman: pick app → mint fabric → lesson paints → tips on
refuse) and **Advanced / EguiPilot** (dev/agent: `dpl-coach-overlay`,
probes, `CoachCommand`, stream dumps). They must not collapse into each
other. DevPULSE coaching *itself* is the optimized in-platform case: the
same touchpoint machinery, but against a surface whose fabric we author
rather than discover.

| Layer | Status |
|---|---|
| In-app touchpoints (C1) | Landed — pilot `CoachCommand` / ColdLight spotlight |
| Foreign locate + compose + paint + `Triggered` (C2s) | **PASS** — live operator press on `gnome-calculator` |
| Overlay process exception + self-test (C2a/C2b, #158) | **PASS** |
| Operator picker / mint / tip UX (C2c) | Mock **LOOK-APPROVED** 2026-07-30 |
| Lightweight installed-app inventory (C2d) | **NEXT — not started** |
| Fabric mint progress + ETA | Not started |
| Stdio coach loop on overlay | Parked (Advanced) |

---

## 3. How the pillars compose

The pillars are not five products. Each one feeds the next, and the seams
between them are typed:

```
    Coach ─────────────────────────────────────────────┐
      teaches the operator to drive any of it          │
                                                        ▼
  Council ──▶ Workflows ──▶ WiredFRONT ──▶ Workbench ──▶ show
  (decide)    (make it      (raw edit)     (slides,      (episode,
              methodical)                   cards)        deck, guide)
      ▲            │                                       │
      └── Chat ────┴───────────────────────────────────────┘
          route-aware, assist-on, present at every step
```

Read it as: **the Council decides, Workflows make the decision methodical,
WiredFRONT cuts the raw material, Workbench assembles the show, Chat is
present throughout, and the Coach teaches the operator to drive any part of
it.** Every arrow is a typed seam on the fabric — `BuildEvent`,
`CoachEvent`, `FabricEvent`, the intent domain — never free-form control.

Two anti-loop laws already hold and must keep holding as these connect: a
surface that publishes `build.*` never subscribes to it, and
`coach_signals.rs` never subscribes to the coach domain. A touchpoint
waiting on a coach event would trigger itself.

---

## 4. Ground truth — the honest board

| Pillar | Strongest evidence | Weakest link |
|---|---|---|
| WiredFRONT | L1–L4 rungs landed; editor to ratified mock; slide-minting + `place_slide` both landed | **A render row has no "make a slide" verb; the chain is never proven end to end** |
| Chat | Route context + Assist toggle (#126) landed | "Triggered naturally" unspecified; `chat-peer-window` route failure open |
| Council | Provider law ratified; roster + tabs landed | Council itself **not qualified** — product shape undefined |
| Workflows | Schemas pinned with refusal tests; 11 doctrine laws | Per-stage **fulfilment contract** thinner than the doc schema |
| Coach | C2s live `Triggered`; C2c mock approved | C2d inventory not started |

Two things this doc deliberately does **not** do: invent a sequence the
operator has not set, and promise a date. The mock gate applies — "We should
have mocks built first for a clear vision to work from."

---

## 5. What this doc asks the operator to settle

1. **The import seam's gesture.** A render row gains a second verb beside
   "add to timeline". Does it follow the FOOTAGE rows' existing corner-action
   idiom (`fx` → `Act::AddOverlay`), or does it want its own mock first?
   Reference is already decided by the code — `media_slide_for` refs the
   artifact **in place**, so there is no copy-vs-reference question left to
   answer for media.
2. **"Triggered naturally."** What is assist allowed to notice on its own,
   and what must it never interrupt?
3. **A true council.** Panel or debate? What does it produce, and where does
   the product go next?
4. **Pillar order.** §4 names five weakest links; which is next.

---

## 6. Stale facts corrected by this pass

- `CLAUDE.md` states branch protection and required checks are "Pro-gated on
  this **private** repo". **The repo is public** (`EonsofStupid/devpulselabs`,
  `visibility: PUBLIC`). Branch protection and required status checks are
  therefore **available at no cost right now** — the local pre-push hook no
  longer has to be the only enforcement. This is a real unlock for the
  double-green merge law, which currently depends on discipline rather than
  mechanism.
- `CLAUDE.md` describes THE checkout as `C:\localdev\devpulselabs` with "no
  other worktree". A Linux checkout now exists at
  `/home/wardenop/Projects/devpulselabs`. Either the one-checkout law needs
  amending to name the Linux lane, or this clone is temporary and should be
  recorded as such.
