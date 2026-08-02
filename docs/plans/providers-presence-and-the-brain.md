# The Providers program — presence, subscription, and the brain

Program map, not a pass plan. It fixes WHAT IS BEING BUILT, the dependency
order, and the laws each link obeys. Each link gets its own 7-section plan
authored immediately before it is built, ground-truthed then.

Parent map: `docs/plans/chat-operations-console.md` (this program supplies
P1's roster with its real content and its face).

## 1. The operator's vision, in their words

> "When I open this laptop and I work in grok build subscription and if i
> have chatGPT already open that means this system is auth already the same
> as claude, they are running and we can absolutely have devpulse labs
> despite being portable have that awareness. We need to have the UI
> actually be my vision. The same for gemini. That said we can also have it
> initialize a subscription and it should easily be able to open the laptops
> browser that needs to be separate from the built in browser used for other
> tasks."

> "we have the area that is listing the major known frontier AI. We want to
> have all of that become space available for our sessions."

> "I click on what will be a neat optimized bleed effect that as the mouse
> just overs ever so slowly like ink to paper think of it as a passive
> experience not meant to be instant but literally over the coarse of 5.6 to
> 6.3 seconds before the full color takes over the icon and icon is brain
> and Providers and just equal at the height of P to the left we have a small
> circle with ai inside. The effect will have two color bleed where we have a
> pool of 4-5 colors that can randomly be pulled as this will become an
> elaborate preset that can be used globally for design. When we click this
> we want to have a popup where we have thumnails, and dynamic tooltips that
> also be clicked to evolve into a popup modal that gives a summary of that
> provider and their dynamic real time how long they have been in business."

### 1.1 Read back, so the understanding is on the record

1. **Presence awareness.** The machine already knows things. If Grok is
   running on a build subscription, if ChatGPT is open, if Claude is
   running — DevPULSE should reflect that those are AUTHENTICATED AND
   AVAILABLE without asking the operator to sign in again. Same for Gemini.
   Portable app, machine-aware.
2. **Subscription initialization.** When a provider is NOT signed in, the
   app starts that flow by opening **the laptop's own default browser** —
   explicitly NOT the built-in browser used for other tasks.
3. **The Providers surface.** A named area listing the major frontier AI,
   and everything listed there becomes space available for our sessions.
4. **The entry point** is a brain icon and the word **Providers**, with a
   small circle containing **ai** to the LEFT, aligned to the cap-height of
   the "P". Hovering starts a slow ink-to-paper bleed — **passive, 5.6–6.3
   seconds** to full color — a two-colour bleed drawn from a pool of 4–5
   colours picked at random. **This becomes a global, reusable preset.**
5. **The click** opens a popup of provider thumbnails with dynamic
   tooltips. A tooltip is itself clickable and **evolves into a modal**
   carrying that provider's summary and a live-ticking "how long they have
   been in business" figure.

## 2. Two things in this that change the build

Stated up front because they shape every link below.

### 2.1 Presence is answered by the vendor's own tool, never by its cookies

The house already does this and it is the right mechanism.
`CodexActivationCoordinator::discover()`
(`crates/chat/providers/codex/src/activation.rs:294`) finds the CLI, then
`SystemCodexStatusProbe` runs **`codex login status`** and turns the answer
into an `ActivationState`. That is literally "am I already signed in on this
machine", answered by the party that knows.

So the operator's awareness is built from:

- **the vendor's own status command** (`codex login status`, `claude`'s
  auth state, `gemini` CLI status) — authoritative, and it is how the
  vendor intends to be asked;
- **local session artifacts the vendor documents** — H1 already uses
  `~/.claude/projects` and `~/.codex/sessions` this way;
- **process presence** — "ChatGPT is open" as a signal that the desktop
  app is running.

It is NOT built by reading another application's cookie jar or credential
store. That path is encrypted per-OS, breaks on every vendor update, and is
credential handling this project deliberately does not do (the standing rule
that Claude never enters credentials points the same way). The probe route
gives the operator exactly the awareness described, and it keeps working.

**Consequence for the UI:** a provider's row must be able to say three
different true things — *signed in and ready*, *installed but signed out*,
and *not installed* — because that is what the probes can actually
distinguish. The design must carry all three honestly.

### 2.2 5.6–6.3 seconds is a NEW MOTION CLASS, not a long transition

`crates/coldlight/tokens/motion.json` authors every duration in the system:

```
"durations": { "fast": 0.12, "base": 0.2, "slow": 0.32, "settle": 0.3 }
```

The operator's bleed is **~18× the slowest thing the vocabulary has**. It is
not a tweak to `slow`; it is a different category of motion — one the
operator named precisely: *"a passive experience not meant to be instant."*

So this program adds a **`dwell` duration tier** to the authored tokens, and
the ink-bleed becomes a named effect beside the existing eighteen
(`ambient-drift`, `blend-drift`, `enter-bloom`, …). It is authored in
`tokens/motion.json` and GENERATED into `presets_gen.rs`, per the
generated-tokens-only law — never hand-written into a widget.

**And the randomness must be deterministic under proof.** The gate takes
screenshots and the app has a determinism gate (`--mood 0`,
`DeterminismMode::Strict` forced for evidence). A freely random colour pick
would make every proof shot flaky. The pool pick is therefore **seeded** —
random per session for the operator, pinned under determinism-strict — the
same discipline `tooltip_mood: random_mood_index()` already lives under.

## 3. What already exists (ground-truthed 2026-07-31, `989e145`)

| piece | state | where |
|---|---|---|
| System-browser handoff, separate from the built-in browser | **BUILT** | `crates/chat/browser-auth` — `BrowserAuthCoordinator::open_once/reopen`, a worker thread per provider, bounded and retryable; the built-in browser is the unrelated `dpl-engine-browser` |
| `ProviderEvent::SystemBrowserDispatched` | **BUILT** | `crates/fabric/src/lib.rs:248` |
| Codex presence probe + login launcher | **BUILT** | `providers/codex/src/activation.rs` |
| Grok activation coordinator | **BUILT** | `providers/grok` |
| Claude local-transcript activation | **BUILT (H1)** | `with_local_history`, `app/src/main.rs` |
| Gemini | **`planned` only** — no probe, no source | `providers/gemini` |
| Provider roster readable by the platform | **PLANNED (P1)** | `docs/plans/chat-providers-become-platform.md` |
| Hue pool (12 brand + 25 accent named hues) | **BUILT** | `coldlight/tokens/hues.json` |
| `LookPreset` / `MotionPreset`, generated from tokens | **BUILT** | `coldlight/src/lib.rs:242`, `tokens/presets.json` |
| Icon vocabulary (`draw_icon`, `icon_label`) | **BUILT** | `coldlight/src/lib.rs:156-240` — a **brain** icon must be authored into `tokens/icons.json` |
| A `dwell` duration tier / ink-bleed effect | **MISSING** | this program |

The honest summary: **the plumbing for presence and for system-browser
sign-in is largely built and unwired to a face. The face does not exist at
all.** This program is mostly a surface program with one real capability
gap (Gemini, and Claude/Grok status probes).

## 4. The dependency chain

Each link is its own pass: plan → build → `cargo green` → EguiPilot on the
LIVE fixture → proof shot → journal → double-green PR. No link starts before
its predecessor merges.

| link | scope | why here | mutates? |
|---|---|---|---|
| **P1** | **The provider capability registry** (ADR-017). A capability SET per provider, folded onto the fabric, PERSISTED to the estate's `Tree::Agents`, projected onto the agent seam, and read by a first consumer — all in one pass. | Already planned, and **re-scoped 2026-07-31**: the operator named that activation cross-links into the platform's config stores and that A2UI, workflows / mixture-of-experts routing, A/B duplicate runs, and Clyffy's training capture all bind to its shape. Everything below needs the platform to SEE a provider; those four need it to see what a provider CAN DO. | no |
| **V1** | **The ink-bleed preset.** A `dwell` duration tier + a two-colour `ink-bleed` effect authored in `tokens/motion.json`, a seeded 4–5 hue pool, and the brain icon in `tokens/icons.json`. Lands with the **Providers** control itself: brain + "Providers" + the `ai` circle at the P's cap-height, bleeding on hover over 5.6–6.3s. | The operator called it "an elaborate preset that can be used globally". A preset proves itself by being USED once, at its origin, before anything else adopts it. Generated-tokens-only means it is authored once and available everywhere for free. | no |
| **V2** | **The Providers popup.** Thumbnails for the frontier AI, each carrying its live state from P1's roster; dynamic tooltips. | Needs V1's control to open it and P1's roster to fill it. Read-only: it shows what is true. | no |
| **V3** | **The provider modal.** A tooltip promoted to a modal — summary, capabilities, and the live-ticking tenure figure. | Needs V2's tooltip to grow from. The tooltip→modal promotion is the interaction, so the tooltip must exist and be right first. | no |
| **A1** | **Presence probes for Claude, Grok and Gemini.** Status probes in the shape `SystemCodexStatusProbe` already established, plus desktop/browser process presence, feeding real `ActivationState`s. | This is the CAPABILITY gap. It lands after the face exists so that every new true state has somewhere honest to appear — otherwise it is invisible work, which is what happened to the 165 transcripts. | no |
| **A2** | **Subscription initialization.** "Sign in" on a signed-out provider hands off to the LAPTOP'S default browser via the existing `BrowserAuthCoordinator`, and the row updates when the probe confirms. | Needs A1's probes to know when it worked. This is the first link that changes state on the machine, so it lands last and behind a real operator action. | **yes** (auth state, via the vendor) |

**V1 before A1 is deliberate.** H1's lesson was that capability without a
face is invisible — 165 transcripts were decoded and shown as a number. The
face lands first here so that each probe A1 adds is immediately visible and
provable.

## 5. The laws this program obeys

1. **Presence is probed, never scraped.** §2.1. No cookie jars, no
   credential stores, no keychain reads. The vendor's own tool is the
   authority on whether the operator is signed in.
2. **Every state the UI shows must be one a probe can actually
   distinguish** — ready / signed-out / not-installed. No row ever implies
   more certainty than the probe gives it.
3. **Tokens are authored, never hand-written.** The bleed, the dwell tier,
   the hue pool and the brain icon live in `crates/coldlight/tokens/*.json`
   and are GENERATED. A widget that hard-codes a duration or a colour is
   the bug, not the feature.
4. **Randomness is seeded and pinned under determinism.** §2.2 — proofs
   must be reproducible or the experience gate is worthless.
5. **The system browser and the built-in browser never merge.** Sign-in
   goes to the operator's default browser through `BrowserAuthCoordinator`;
   `dpl-engine-browser` remains for in-app tasks. They are different tools
   with different trust.
6. **Experience-first.** Each link is proved on the real exe at a real
   window size. The bleed is the whole point of V1 — a proof shot mid-bleed
   and at full colour, or it is not done.
7. **Operator-parity.** Nothing here becomes AI-only.

## 6. What this map does NOT decide

Per-link design — the exact hue pool membership, the bleed's easing curve,
the popup's grid, the modal's copy, which providers count as "frontier" —
is decided in that link's own plan against the code as it stands then, and
against the mock. If a link's ground truth contradicts this map, the map is
amended in the same commit.

## 6b. Where this program ends (recorded 2026-07-31, not yet planned)

The operator's own framing, so the near links are not designed in a way
that forecloses it:

- **Workflows are mixture-of-experts.** A beat is routed to whoever can
  take it. `dpl-workflows` already ships `WorkflowDoc`/`Stage`/`StageKind`/
  `Beat` — "beats" is literally the type — and today routes to nobody.
- **A/B duplicate runs are the tuning mechanism**, not a test harness: the
  same beat dispatched to N providers at once, the operator PICKS, and the
  pick is the label.
- **The captured picks fine-tune Clyffy**, so the platform is not 100%
  frontier-dependent — the general conversational tier becomes the
  operator's own tuned model.
- **EguiPilot has become a teacher/coach system**, which is what makes the
  parallel-run capture observable in the first place.

None of this is built here. It is recorded because ADR-017 §4 shows the
registry's SHAPE is load-bearing for all four, and a boolean pair would
have made them impossible.

## 7. Status

The mock for this whole program is `docs/design/mocks/providers.html` — one
surface driven by one registry. Every link below is a slice of it, and a
link is done when the real app matches that slice.

- **P1** — the registry, built (`docs/plans/chat-providers-become-platform.md`).
- **V1** — the control and the ink-bleed preset, built
  (`docs/plans/providers-v1-ink-bleed-control.md`). The `dwell` tier, the
  `ink-bleed` effect, the seeded hue pool and the brain are authored tokens;
  the control carries no design value of its own.
- **V2** — the roster popup the control opens. **V3** — the tooltip promoted
  to the modal with live tenure.
- A1 (presence probes) → A2 (subscription init).

Note for every motion link after this one: the capture harness runs under
`MotionPolicy::Suppressed`, so it cannot photograph an animation mid-flight.
Prove the ends with shots and the duration with a pin on the authored token.
