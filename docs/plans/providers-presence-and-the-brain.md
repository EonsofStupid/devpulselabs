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

## 3. What already exists (re-measured 2026-08-03 on `0f5dd69`)

| piece | state | where |
|---|---|---|
| System-browser handoff, separate from the built-in browser | **BUILT** | `crates/chat/browser-auth` — `BrowserAuthCoordinator::open_once/reopen`, a worker thread per provider, bounded and retryable; the built-in browser is the unrelated `dpl-engine-browser` |
| `ProviderEvent::SystemBrowserDispatched` | **BUILT** | `crates/fabric/src/lib.rs:248` |
| Codex presence probe + login launcher | **BUILT** | `providers/codex/src/activation.rs` |
| Grok activation coordinator | **BUILT** | `providers/grok` |
| Claude local-transcript activation | **BUILT (H1)** | `with_local_history`, `app/src/main.rs` |
| Presence probes for every provider (A1) | **BUILT** | `crates/app/src/presence.rs` — the only place that spawns; each provider names a non-secret status command as plain data |
| Sign-in hand-off to the default browser (A2) | **BUILT** | vendor CLI opens the browser; a watcher polls the probe until it confirms |
| Provider roster readable by the platform | **BUILT** | `fabric.provider_roster()` / `publish_provider_roster()` — P1 landed |
| Hue pool (12 brand + 25 accent named hues) | **BUILT** | `coldlight/tokens/hues.json` |
| `LookPreset` / `MotionPreset`, generated from tokens | **BUILT** | `coldlight/src/lib.rs:242`, `tokens/presets.json` |
| Icon vocabulary (`draw_icon`, `icon_label`) | **BUILT** | `coldlight/src/lib.rs:156-240` — a **brain** icon must be authored into `tokens/icons.json` |
| A `dwell` duration tier / ink-bleed effect | **BUILT** | `coldlight/tokens/motion.json` (`ink-bleed`), `tokens/presets.json` |
| The Providers control + popup + modal | **BUILT** | `shell/src/lib.rs` `providers_popup()`, drill `tests/eguipilotai/providers-popup.toml` |
| Enter sends / Shift+Enter newline | **BUILT** | `chat/window/src/lib.rs:111` |
| Usage + cost per provider | **BUILT (#169)** | `app/src/ai_usage.rs` — `ProviderUsage`, `usage_for()`, `TokenUsage` |
| The provider RUNTIME seam | **BUILT** | `chat/runtime/src/seam.rs` — `ProviderRuntime`, Grok aligned via `grok/src/seam.rs` |
| Codex conversation transport | **MISSING** | `providers/codex` has `activation.rs` only — login detection, no runtime |
| Clyffy (the persona layer) | **MISSING** | §7 |
| Group chat / research council | **MISSING** | §8, §9 |

**Re-measured 2026-08-03 and the summary INVERTED.** The face now exists —
V1/V2/V3 and P1 all landed while this map still called them planned. What
remains is not surface, it is CAPABILITY:

- **Codex cannot converse.** It detects the operator's ChatGPT login and has
  no transport. Only Grok grew a runtime. This is the single largest gap and
  the operator has named it since the first message of the session.
- **Gemini and Cursor are NotInstalled on this machine** — §10 records the
  live measurement. Their probes EXIST (A1 built); there is simply no CLI
  here to answer them. That is a machine fact, not a gap in the code.
- **Clyffy does not exist** — there is no persona layer between the operator
  and the interchangeable providers (§7).
- **There is no session primitive**, so group chat (§8) and the research
  council (§9) have no substrate. `grep session crates/seam` = 2 hits,
  neither a conversation.

The lesson this map should have caught earlier: a status table is a claim,
and a claim decays. Re-measure before planning against it.

## 4. The dependency chain

Each link is its own pass: plan → build → `cargo green` → EguiPilot on the
LIVE fixture → proof shot → journal → double-green PR. No link starts before
its predecessor merges.

| link | scope | why here | mutates? |
|---|---|---|---|
| **P1** ✅ | **The provider capability registry** (ADR-017). A capability SET per provider, folded onto the fabric, projected onto the agent seam, and read by a first consumer. **Persistence amended 2026-08-03: chat owns its store, NOT the estate's `Tree::Agents`** (ADR-017 amendment). | Already planned, and **re-scoped 2026-07-31**: the operator named that activation cross-links into the platform's config stores and that A2UI, workflows / mixture-of-experts routing, A/B duplicate runs, and Clyffy's training capture all bind to its shape. Everything below needs the platform to SEE a provider; those four need it to see what a provider CAN DO. | no |
| **V1** ✅ | **The ink-bleed preset.** A `dwell` duration tier + a two-colour `ink-bleed` effect authored in `tokens/motion.json`, a seeded 4–5 hue pool, and the brain icon in `tokens/icons.json`. Lands with the **Providers** control itself: brain + "Providers" + the `ai` circle at the P's cap-height, bleeding on hover over 5.6–6.3s. | The operator called it "an elaborate preset that can be used globally". A preset proves itself by being USED once, at its origin, before anything else adopts it. Generated-tokens-only means it is authored once and available everywhere for free. | no |
| **V2** ✅ | **The Providers popup.** Thumbnails for the frontier AI, each carrying its live state from P1's roster; dynamic tooltips. | Needs V1's control to open it and P1's roster to fill it. Read-only: it shows what is true. | no |
| **V3** ✅ | **The provider modal.** A tooltip promoted to a modal — summary, capabilities, and the live-ticking tenure figure. | Needs V2's tooltip to grow from. The tooltip→modal promotion is the interaction, so the tooltip must exist and be right first. | no |
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

## 6b. Where this program ends (recorded 2026-07-31; §7–§9 now carry it)

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

**Superseded in detail by §7–§9**, which carry the operator's 2026-08-03
signal: Clyffy as the persona layer, chat as the secondary cockpit with
group chat, and the research council. This section stays as the record of
when the shape was first recognised.

## 7. Clyffy — the persona layer

**Operator ruling, 2026-08-03:** *"Clyffy will be our global/per session face
of the entire 'AI'. Clyffy will not become the group. Clyffy will not exist
in the group chat."*

### 7.1 What Clyffy is

ONE persona the operator talks to, backed by interchangeable providers. The
provider is the engine; Clyffy is the face. Scope is **global or
per-session** — not per-message, so the operator always knows who they are
addressing.

### 7.2 The identity is VISIBLE, and that is deliberate

Whatever provider currently enforces Clyffy is expressed in the UI:

- **Clyffy's font** shifts to that provider's typeface;
- **Clyffy's symbol** carries that provider's mark — `Clyffy (symbol)`;
- **the chat's typeface** aligns to the same provider.

Every mainstream abstraction layer (LiteLLM, gateways, LangChain) sells the
opposite: the swap is invisible, and that is the point. This program takes
the OTHER side on purpose. An invisible swap hides *who answered you* — and
for a platform whose whole tuning loop is "which answer did the operator
keep", hiding the author destroys the label (ADR-017 §5).

### 7.3 Substrate

`ProviderDescriptor` already carries `glyph` and `accent` per provider, and
ColdLight has an authored font vocabulary. **The binding is what is
missing** — no `font` on the descriptor, and no persona type that resolves
"Clyffy, currently enforced by X" into font + glyph + accent.

Per law 3 the font is an AUTHORED token, never hand-written at a call site.

### 7.4 Where it lives

`dpl-chat-runtime` — the crate that already owns provider-neutral state and
whose `seam.rs` answered the transport question. The persona is
provider-neutral state that happens to persist. **No new crate.**

## 8. Chat as the secondary cockpit, and group chat

**Operator, 2026-08-03:** *"the chat becomes a secondary cockpit basically it
will have AI like capabilities i.e the chat has route awareness from the
platform and then even allows us to load routes, and have addition popup
windows appear and evolve the chat and have task windows that can easily be
minimized and brought back."*

### 8.1 Cockpit capabilities

| capability | state |
|---|---|
| route awareness from the platform | **BUILT** — `observe_route`, the rail reads it |
| LOADING a route from chat | intent exists (`NavigateTo`); the operator-facing verb does not |
| popup windows that evolve the chat | not built |
| task windows, minimizable and restorable | not built |

Task windows are the substantive one: a long operation (§9 runs 10–15
minutes) must be parkable, or the cockpit is blocked by its own work.

### 8.2 Group chat

Round-robin **is** group chat — settled 2026-08-03. One SHARED transcript,
participants take turns, the operator in the ring. Not fan-out; fan-out is
§9.

**Clyffy is absent from group chat.** The operator offered an alternative —
Clyffy present, rendered in each provider's font and symbol — and ruled
either acceptable. Recorded as the operator's choice to make at build time;
the DEFAULT is absent, because in a group the providers speak as themselves
and a persona over the top would obscure exactly the attribution §7.2
exists to preserve.

### 8.3 The popup group chat opens with

A generated modal showing **currently available subscriptions and API usage
/ billing**. Substrate exists: `fabric.provider_roster()` for availability,
`ai_usage::usage_for()` + `TokenUsage` for spend (#169). Nothing new is
needed to make it true — only to draw it.

## 9. The research council

**Operator, 2026-08-03:** each selected provider — API, subscription, or
both — runs a DEEP research pass in its OWN session, 10–15 minutes. When all
return, a default provider processes the results into layman-stylized
visuals.

### 9.1 Shape

Fan-OUT (isolated sessions, no cross-talk) → synthesis. Distinct from §8's
shared transcript, and the industry's settled 2026 shape: *orchestrator +
isolated subagents with summary returns*.

Isolation is load-bearing. A single persuasive agent in a shared debate can
drag consensus toward a wrong answer and RAISE agreement around it, so the
council must not deliberate — it must report independently and let the
comparison happen afterwards.

### 9.2 The staging, with one inversion of the operator's draft

The operator described stage 1 as: where results ALIGN, then where they
DIFFER. **The research says invert it** — surface disagreement FIRST:

> *"Better council prompts should explicitly ask for disagreement first…
> disagreements must be identified before synthesis, helping prevent the
> council from becoming a rubber stamp."*

And more sharply: **agreement is not truth.** Where every model agrees but
live evidence contradicts them, that pattern detects stale training-data
consensus. So agreement is presented as **coverage**, never correctness, and
divergence is the primary signal — *disagreement as data*.

The merge UI therefore OPENS on the conflicts. The operator's few
layman-friendly questions decide how much to take from which research — and
that choice is the label, exactly as in ADR-017 §5.

### 9.3 The crunch experience — and the rule it must not break

A reusable "deep crunching" treatment for every freeze moment (patches,
updates, system analytics, staged processes): animations pulled at random,
periodic REAL status, rotating Did-You-Know AI facts.

**The rule:** for waits over ~10 seconds a stalled bar is worse than none,
and an undifferentiated wait breeds doubt. The strong pattern is
**progressive disclosure — show each provider's result AS IT LANDS** rather
than holding all of them for the slowest. With four providers at 10–15
minutes, that is the difference between one 15-minute void and four
arrivals.

So: Did-You-Know is the TEXTURE; real per-provider milestones are the SPINE.
An animation that entertains while telling the operator nothing is the
failure mode, not the feature.

Determinism note: the random pull obeys law 4 — seeded, pinned under
determinism-strict, or every proof shot goes flaky.

### 9.4 How these passes are sequenced

**Operator, 2026-08-03:** *"we do it in passes where as the project grows the
workflow grows as it then starts to demand that you align with the existing
code patterns consistent naming patterns any existing boundaries to align
too or if new boundaries are being created we tell the operator so they can
give their input or just select recommendations."*

Binding on every link from here:

1. Each pass names which EXISTING boundary it aligns to.
2. A pass that creates a NEW boundary says so explicitly and stops for the
   operator — input, or a recommendation to accept. New boundaries are never
   a drive-by. (ADR-017's amendment names one: "chat's store".)
3. Naming follows what is already there; a new vocabulary needs the same
   flag as a new boundary.

## 10. Status

The mock for this whole program is `docs/design/mocks/providers.html` — one
surface driven by one registry. Every link below is a slice of it, and a
link is done when the real app matches that slice.

- **P1** — the registry, built (`docs/plans/chat-providers-become-platform.md`).
- **V1** — the control and the ink-bleed preset, built
  (`docs/plans/providers-v1-ink-bleed-control.md`). The `dwell` tier, the
  `ink-bleed` effect, the seeded hue pool and the brain are authored tokens;
  the control carries no design value of its own.
- **V2** — the roster popup, built. Cards carry LIVE state from P1's roster
  (`shell/src/providers.rs::ProvidersPopup`); proof
  `tests/eguipilotai/providers-popup.toml`, shot
  `docs/proof/providers-popup.png`.
- **V3** — the modal, built. A card promotes into a summary carrying the
  live-ticking "how long they have been in business" figure; the roster stays
  up behind it. Proof `tests/eguipilotai/providers-modal.toml`, shot
  `docs/proof/providers-modal.png`.

**Every link in this program is now built.** What remains is qualification,
not surface: live turn transport for Cursor, and Gemini's probe + sign-in
verb once its CLI is on a machine to read them off.
- **A1 — presence probes**, built. Each provider names a non-secret status
  command as plain data; `crates/app/src/presence.rs` is the only place that
  spawns. Measured live: codex/claude/grok SignedIn, cursor/gemini
  NotInstalled.
- **A2 — subscription init**, built. "Sign in" hands off to the vendor's own
  CLI, which opens the operator's DEFAULT browser, and a watcher polls the
  probe until it confirms. A provider with no declared sign-in verb is
  refused by name rather than guessed at.
- **Cursor** is a subscription provider (`crates/chat/providers/cursor`) —
  its CLI carries the same login as the editor.

Note for every motion link after this one: the capture harness runs under
`MotionPolicy::Suppressed`, so it cannot photograph an animation mid-flight.
Prove the ends with shots and the duration with a pin on the authored token.
