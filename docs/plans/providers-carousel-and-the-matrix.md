# The Providers carousel, and the trait matrix it belongs to

**PLAN — not yet built.** Captures the operator's direction in their words
(2026-08-02), then states it back in platform vocabulary so it can be built
without another reset. Everything in §5, §8 and §9 is a REQUIREMENT; §6 is
the honest list of what does not exist yet.

Read this with the surface inventory before touching code — the map says what
is already there, this says what changes.

Surface inventory: `docs/plans/chat-surface-map.md`.
Program: `docs/plans/providers-presence-and-the-brain.md`.

## 1. What changes, in the operator's words

> "the section where the original you had all of the providers listed in the
> rail, that becomes Brain small circle ai Providers with a mouse over color
> change i gave you those colors to include as global part of the matrix you
> are suppose to be building. When we click on it the providers appear in a
> seamless scrolling. We have them blow up and come off the canvas on
> selection the square will be accessible so you can use the mouse to
> seamless scroll down if focused in the overlay and when you focus over one
> it rises off the canvas and blows up in size and then explodes on selection
> in a fun 2d effect."

## 2. The change, stated precisely

**Before:** the AI rail listed every provider as a stack of tiles — one row
per provider, always on screen, consuming rail height that the sessions need.

**After:** that section collapses to ONE control — `(ai) 🧠 Providers` — and
the providers themselves move into an overlay the control opens.

**The rail stays.** This is not the rail being replaced; it is the provider
SECTION of the rail becoming a single entry point. Sessions, history groups
and the domain switcher are untouched.

## 3. Naming it properly

The operator's phrasing, translated to the terms the rest of the system
already uses — so a plan can reference them and a token can carry them.

| the operator said | the platform term | what it means concretely |
|---|---|---|
| "mouse over color change" | **hover state transition on the `hover` axis**, driven by the `dwell` duration tier | the authored ink-bleed, 5.6–6.3s, two seeded hues |
| "those colors… global part of the matrix" | **a named hue POOL, authored once and referenced by id** | `bleed` preset in `tokens/presets.json`, pool cross-checked against `hues.json` at build time |
| "seamless scrolling" | **inertial (momentum) scrolling inside a focus-scoped scroll container** | wheel and trackpad are captured only while the overlay owns focus |
| "the square will be accessible" | **roving tabindex, visible focus ring, keyboard scrolling, `prefers-reduced-motion` fallback** | every tile reachable and operable without a mouse |
| "rises off the canvas" | **elevation change** — a step up the elevation scale | shadow depth + a slight scale, not a literal Z translate |
| "blows up in size" | **focus scale transform** with the tile's own centre as transform-origin | siblings do not reflow; the tile grows over them |
| "explodes… in a fun 2d effect" | **selection burst** — a radial 2D shard emission on commit | a terminal, non-looping effect authored beside the other 19 |
| "the matrix you are suppose to be building" | **the patch matrix: seed lineages × axes**, composed per-axis | `tokens/frame-patches.json`, generated, never hand-written |

## 4. The matrix — ALREADY BUILT, and bigger than described here

> **Correction, 2026-08-03.** This section previously claimed the matrix held
> "four traits and no seed lineages" and that the five ancestrals were "not in
> it yet". Both were false. `main` has carried **THE MATRIX** since
> 2026-07-31 — see `docs/design/coldlight-matrix.md` and
> `crates/coldlight/tokens/matrix.json`.

A look is a **coordinate** across four facets (Ranganathan's Colon
Classification), not a hand-authored bundle:
**ANCESTOR** (owns `font`) × **STRUCTURE** (`radius`, `edge.width`) ×
**MATERIAL** (`fill`, `ink`, `edge.color`) × **ATMOSPHERE** (`shadows`,
`back_glow`, `tints`, `anim`), with **STATE** as the overlay.

Generated and build-validated on main:

- **8 ancestors** — `bauhaus · art-deco · brutalist · swiss · aero · memphis ·
  constructivist · editorial`
- **6 structures** · **9 materials** · **6 atmospheres**
- ~22 authored values → ~900 valid coordinates. Legality is DATA: every value
  is native/allowed/forbidden for every ancestor, and leaving one
  unclassified **fails the build**.

So the operator's five are five of eight, and the four old traits reclassify
rather than survive: `neubrutalist` is the **Brutalist ancestor**,
`claymorphic` is a **material**, `cyber` and `retro` are **atmospheres**.

The bleed pool remains a matrix citizen: authored once in
`tokens/presets.json`, cross-checked against `hues.json` at build time, and
referenced by id — never a list of hexes in a widget.

## 5. The interaction, end to end

1. **At rest** — the control is line-art in muted ink. No provider tiles on
   screen.
2. **Hover** — the ink bleed runs over the seeded dwell; brain, `ai` circle
   and word arrive at full colour together.
3. **Click** — the overlay opens. Providers are a scroll container with
   inertial scrolling, scoped to the overlay's focus.
4. **Hover / keyboard focus on a tile** — the tile elevates and scales over
   its siblings. Exactly one tile is elevated at a time.
5. **Select** — the selection burst fires, the overlay dismisses, and the
   provider becomes the active one with a **small clear label** in the rail
   confirming what is now active.
6. **Reduced motion** — bleed, elevation and burst all degrade to an instant
   state change. Nothing becomes unreachable.

## 6. What this makes true that is not true today

- The rail's provider section is a fixed vertical list; it does not collapse.
- There is no overlay, no scroll container, no elevation-on-focus, and no
  selection burst.
- `tokens/motion.json` has 19 effects; **none is a terminal 2D burst.**
- ~~The trait matrix has four traits and no seed lineages.~~ **FALSE — see §4.** The matrix is built, with eight ancestors.
- The activation label after selection does not exist.

## 7. What is already in place to build on

- The control itself — brain icon, `ai` circle at the P's cap-height, the
  `dwell` tier and the two-colour `ink-bleed`, all authored and generated.
- The `bleed` preset kind, whose hue pool is validated against `hues.json` at
  build time — an unknown hue fails the build.
- The roster behind it: every provider's live presence, so the tiles have
  real state to show rather than a static list.

## 8. SETTLED — the overlay is two sections, not one

> "carousel for providers where it seamless scrolls they popoff off the
> canvas the API section will be 4 rows minimal that seamlessly scroll due to
> the volume of choices … so it goes across viewport for chat and providers
> with subscriptions only"

The overlay spans the **full viewport width** and holds two populations that
are shaped by how many of each there are:

```
┌─────────────────────────────────────────────────────────────────┐
│  SUBSCRIPTIONS            ← one row, carousel, across viewport  │
│  ◀  [GPT] [CL] [GK] [CU] [AGY] [GM]  ▶     few, each one large  │
├─────────────────────────────────────────────────────────────────┤
│  API                      ← 4 rows minimum, seamless scroll     │
│  [ ][ ][ ][ ][ ][ ][ ][ ]                                       │
│  [ ][ ][ ][ ][ ][ ][ ][ ]        many — volume is the reason    │
│  [ ][ ][ ][ ][ ][ ][ ][ ]        it is a grid and not a row     │
│  [ ][ ][ ][ ][ ][ ][ ][ ]                                       │
└─────────────────────────────────────────────────────────────────┘
```

**Subscriptions — a horizontal carousel, one row, across the viewport.**
These are the accounts the operator already pays for and signs into with the
vendor's own flow. There are few of them, so each tile is large and the row
scrolls horizontally. This is the section the chat actually runs on.

**API — a grid, four rows minimum, vertically scrolling.** The operator's
reason is the right one and it is a real design constraint: *"due to the
volume of choices."* An API list is endpoints × models and runs to dozens;
a carousel would make the operator page through it one screen at a time.
Four rows minimum means the section is never a cramped strip.

Both scroll seamlessly, both use the same tile behaviour — elevate on focus,
burst on selection.

### 8.1 The split already exists in the code

This is not a new taxonomy; it is one the system already carries and has
never surfaced:

- **Subscription** is a capability: `Capability::SubscriptionAuth`
  (`chat/core:282`), beside `DeviceCodeAuth`. Codex, Claude, Grok, Cursor and
  Antigravity all declare it. Cursor's CLI carries the same login as the
  editor; Antigravity signs in against the OS keyring.
- **API** is a profile: `ProviderProfile { key, kind, display_name,
  endpoint, model }` in `chat/profiles` (556 loc), with
  `chat.provider.configuration.endpoint` / `.model` / `.display-name`
  already existing as named surfaces.

So the carousel reads the **roster** (live presence per provider) and the API
grid reads the **profile store**. Two sources, two shapes, one overlay.

## 9. SETTLED — the rail groups both populations under chips

> "the rail will show a small label that will appear just above the first
> entry for those that are subscriptions and have in small lettering as it
> will subscriptions that will be a chip that has subscriptions small but
> easy to read in a chip container that has elevation and then subscription
> lights up in sporadic colors on mouse over and then another chip that says
> API *usage-based*"

**Both populations appear in the rail**, each introduced by a group chip
sitting directly above its first entry:

```
  ╭──────────────╮
  │ subscriptions│  ← chip, elevated, small but legible
  ╰──────────────╯
     [GPT]  Codex
     [CL]   Claude Code
     [GK]   Grok

  ╭──────────────────────╮
  │ API · usage-based    │  ← chip, elevated, the qualifier is the point
  ╰──────────────────────╯
     [ ]  gpt-5 · api
     [ ]  sonnet · api
```

### 9.1 The qualifier is information, not decoration

`usage-based` is the single most operationally important word on the rail.
Subscriptions are already paid for; an API call costs money per token. The
chip is where the operator learns which group they are about to spend from,
BEFORE they pick. It is not a style label and must never be shortened away.

### 9.2 The chip

- **A chip container with elevation** — it reads as a raised object over the
  rail, not as a text run. The `eyebrow` treatment currently used for session
  groups is flat lettering; this is deliberately a step up the elevation
  scale from that.
- **Small but easy to read** — small is a size instruction, not a legibility
  one. Type-size floor and contrast come from the authored ink ramp; the chip
  never drops below what the pool guarantees against its own fill.
- **Positioned above the first entry of its group** — the chip introduces the
  group; it is not a header for the whole rail.

### 9.3 "Sporadic colours" is a SECOND effect, not the bleed

This matters because it would be easy to reuse the wrong one:

| | Providers control | group chip |
|---|---|---|
| effect | `ink-bleed` | **new — sporadic** |
| duration | `dwell`, 5.6–6.3s | fast, sub-second |
| character | one slow uniform wash | **scattered, irregular, multi-hue** |
| hues | two, seeded per session | many, from the same pool |

The control's bleed is *passive — not meant to be instant.* The chip's is the
opposite: it flickers alive across the lettering on hover. **Same global hue
pool, different temperament.** That is exactly what makes the pool a matrix
citizen rather than a one-off — two effects, one authored source of colour.

Its authored form (per-glyph phase offset, hue-step cadence, decay) is
decided in the link's own plan, in `tokens/motion.json`, beside the other
effects. It is the **third** thing the pool feeds, after the bleed and the
selection burst.

## 10. SETTLED — activation fills the rail, and there are two ways in

> "that chip appears and then as i activate the providers they appear in the
> rail after clicking on + symbol in the chat itself input section and a cross
> link in the rail to trigger it as well."

**The chip comes first, empty.** Both chips stand in the rail before their
group has a single entry. The chip introduces the group; it is not a header
for rows that already exist.

**Activation is what puts a row there.** Today `provider_roster()`
(`chat/window/src/lib.rs:2091`) paints one row per REGISTERED provider,
unconditionally — which is why the live capture shows six rows, four of them
saying *Blocked*, with `More providers…` clipped mid-word and no room left for
the session groups at all. After this change the rail carries what the
operator has turned on, and everything else lives in the carousel.

**Two triggers, one overlay:**

| trigger | where | today |
|---|---|---|
| `＋` | the composer's input section, beside the route chip | does not exist |
| the cross-link | the rail, where `AI PROVIDERS` + the rows are now | exists only as the clipped `More providers…` row |

Both raise the same overlay and are the same intent. `PROVIDERS_ID` currently
hangs off that clipped row, which is the single worst place in the rail to put
the only way in.

### 10.1 Every pattern here owes the slide catalog a generic item

Per `docs/plans/coldlight-naming-and-the-pattern-library.md` §2: the group
chip, the carousel row, the tile grid, the lift-on-focus tile, the selection
burst and the sporadic wash each land a generic renderer plus an organized
seed **in the same pass as the Chat use** — never afterwards, or they get
generalized to fit Chat rather than the library.

## 11. Still open

- The burst's and the sporadic effect's authored forms — counts, spread,
  cadence, decay — belong in `tokens/motion.json`, and need the mock first.
- Whether the five lineages become literal entries in `traits.json` or a
  documented provenance field on the existing traits.
