# C1 — the Providers carousel in the CHAT rail

The first link of the providers surface **inside the chat peer window**.
V1–V3 built the `🧠 Providers` control in the SHELL status bar; C1 is the
different thing the operator has been asking for since 2026-08-01 — the chat
rail's provider section becoming a control, and the overlay it opens.

**Gated by** `docs/plans/coldlight-naming-and-the-pattern-library.md` — N1 is
landed (`29512e5`), so C1 authors in `FramePatch` vocabulary and owes the
slide catalog its generic items in this same pass.

Ground truth: `docs/plans/chat-surface-map.md` (and its §7 capture recipe).
Spec: `docs/plans/providers-carousel-and-the-matrix.md`.

## 1. What is being built

| | |
|---|---|
| **The control** | The chat rail's `AI PROVIDERS` micro-heading + N provider rows collapse to one `(ai) 🧠 Providers` cross-link. |
| **The `＋`** | A second trigger in the composer's input section. Same overlay, same intent. |
| **The overlay** | Full chat-window, two sections: subscriptions as a one-row carousel, API as a 4-row-floor grid. |
| **The chips** | `subscriptions` and `API · usage-based`, elevated, each above the first entry of its group, standing even when the group is empty. |
| **The rail** | Lists what is **activated**, not what is registered. |

## 2. The evidence this is not a preference

From the 2026-08-03 X11 capture of the running exe (recipe in the surface
map §7), at six registered providers:

- `More providers…` is **clipped mid-word** at the scroll boundary
- `PINNED`, `ACTIVE` and `RECENT CHATS` **do not render at all**
- four of six rows say **Blocked** and are spending rail height to do it

`provider_roster()` (`chat/window/src/lib.rs:2091`) paints one row per
REGISTERED provider unconditionally. Every provider added makes this worse.
A2 already added two; the pulse program will add more.

## 3. The four decisions the operator settled (2026-08-03)

### 3.1 API volume — dozens (20–60)

So: a **plain `ScrollArea`**, every tile laid out each frame. No
virtualization, no search field. egui's own guidance only calls for
`ScrollArea::show_rows` at the scale where per-frame layout hurts; forty
small frames is not that scale.

**Four rows is a FLOOR, not a fixed height.** The grid takes the height it is
given and never drops below four rows' worth. If it is ever given less, that
is a layout bug, not a graceful degradation.

### 3.2 On select — "on click if no existing it will trigger in the chat"

This lands on code that **already exists and needs no new surface**.

`ai_conversation` (`lib.rs:2462`) already does:

```rust
if !provider.registration.activation.is_activated() {
    self.provider_activation_ceremony(ui, &provider, intents);
    return;
}
```

`provider_activation_ceremony` (`lib.rs:3456`) paints a `PROVIDER CONNECTION`
card in the conversation pane, with a title and detail per activation state
(`SigningIn` · `AwaitingAuthorization` · `Verifying` · `Failed` · `SignedOut`
· unavailable), the action button, and the activation trace.

**So selection is just `SelectProvider`.** The row joins the rail, the
provider becomes selected, and because it is not activated the conversation
pane shows the ceremony. C1 builds **no** sign-in UI.

This also disposes of the Antigravity problem cleanly: `agy` has no
scriptable auth status, so its ceremony falls to the `_ =>` arm — "unavailable
… cannot start a qualified session yet" — which is the truth, visible, in the
pane, instead of a row hanging at *Auth unknown* forever.

### 3.3 The lift — reserve the grown size up front

egui 0.35, `ui.rs:2740`, verbatim:

> *"This only affects visuals, inputs will not be transformed. So this is
> mostly useful to create visual effects on interactions, e.g. scaling a
> button on hover / click."*

So `Ui::with_visual_transform` grows the PAINT and not the HIT BOX. Growing
the real rect instead causes the immediate-mode reflow loop: the grown tile
pushes its siblings, the pointer lands on a neighbour, the first tile
un-hovers and shrinks, everything shifts back — hover flicker at every
boundary.

**The build:** `allocate_exact_size` at the LIFTED dimensions; paint the
resting tile inset inside that rect; on hover let the paint fill the reserved
space via `with_visual_transform`. The hit box never moves, siblings never
reflow, and the pointer cannot fall out of the tile it is hovering.

Cost: ~14% more space per tile at rest. That is the price of the effect and it
is paid in layout, once.

`Context::set_transform_layer` is the only path where input scales too, and
it is explicitly the *persistent* mechanism — one `Area` created and destroyed
per pointer move is not what it is for. Rejected, recorded here so it is not
re-litigated.

### 3.4 Narrow window — control only, chips drop

Below `FULL_RAIL_WIDTH` (840px, `lib.rs:151`) the rail is
`Composition::PresenceRail` at 76px. There, the control is the **brain glyph
alone** — no word, no chips, no rows. The overlay still opens full-window and
remains the way to switch.

This matches what the narrow rail already does (`presence_rail`,
`lib.rs:1956`): the selected provider's short label plus a `•••` picker.

## 4. What C1 must NOT do

- **No sign-in UI.** §3.2 — the ceremony exists.
- **No new `HostIntent` variants.** `ActivateProvider`, `ProviderSelected`,
  `OpenProviderSetup`, `RefreshProvider` and `SaveProviderProfile` already
  cover every action a tile can raise.
- **No search field.** §3.1.
- **No virtualization.** §3.1.
- **No change to `chat/profiles`.** The grid READS the profile store; authoring
  a profile is `chat.provider.configuration`, which exists.
- **No colour-only meaning.** The narrow rail drops the chips rather than
  encoding them as tint — a distinction carried by hue alone is the thing an
  a11y pass exists to catch.

## 5. Accessibility — what is true, and the one real gap

**AccessKit is compiled in.** `eframe = { version = "0.35", features =
["wgpu"] }` does not disable default features, and `accesskit` is in eframe's
default set; the lock file carries `accesskit_winit 0.32.2`,
`accesskit_unix`, `accesskit_windows`, `accesskit_macos`.

**And the pilot tree already feeds it.** `pilot_track`
(`egui-pilot-ai/src/lib.rs:1569`) calls `response.widget_info(...)`, so every
one of the 51 pilot ids is simultaneously an accessibility node. That is a
better position than it looked.

**The gap, precisely:** `pilot_track` emits only

```rust
egui::WidgetInfo::labeled(widget_type(role), response.enabled(), name)
```

`WidgetInfo::labeled` carries **no selected state**. So `metadata.selected`
— populated on every provider row, session row and content row — reaches the
pilot tree and **never reaches a screen reader**. A tab a screen reader cannot
tell is selected is a tab a screen reader cannot use.

C1 fixes this at the source, once, for every tracked node in the app:
`pilot_track` uses `WidgetInfo::selected(...)` when `metadata.selected` is
`Some`, and `labeled` otherwise. Four call sites in `workbench` already hand-
roll `WidgetInfo::selected` beside their `pilot_track` — those collapse into
the one path.

**Keyboard:** roving tabindex across the tiles, arrows moving focus,
`scroll_to_rect_animation` bringing the focused tile into view, `Esc`
dismissing. `egui::Modal` (`containers/modal.rs`) supplies the backdrop and
`is_top_modal`, so the overlay does not hand-roll dismissal.

**Reduced motion:** the bleed, the lift and the burst all degrade to instant.
The harness already forces this — `MotionPolicy::Suppressed` — which is why
captures show settled end states and never a mid-animation frame.

## 6. The generic catalog items C1 owes (the §2 law)

Authored in the SAME pass, not after. Each is a renderer in `crates/slides`
plus an organized seed under `seeds/slides/<category>/`, registered so it
appears on the Workbench Library wall.

| item | category | what it generalizes |
|---|---|---|
| `elements/group-chip` | static | an elevated label introducing a group, valid while the group is empty |
| `elements/lift-tile` | interactive | reserve-then-fill elevation on focus, per §3.3 |
| `blocks/carousel-row` | interactive | one row, horizontal, arrows, kinetic scroll |
| `blocks/tile-grid` | interactive | N-row-floor scrolling grid, N authored |
| `motion.json` effect 20 | animation | the selection burst — **the first terminal (non-looping) effect**; the schema has to admit one |
| `motion.json` effect 21 | animation | the sporadic multi-hue wash — per-glyph phase, hue-step cadence, decay |

## 7. Proof

1. `cargo green` — the gate.
2. A pilot scenario driving: control hover → open → arrow across tiles →
   select → rail row appears → ceremony visible in the conversation pane.
3. An **X11 capture** of the chat at ≥840px and at <840px, per the surface-map
   recipe — the pilot cannot screenshot a deferred viewport, so the pixels
   come from `Xvfb` + `ffmpeg`.
4. The before/after set beside the 2026-08-03 capture: `More providers…` no
   longer clipped, session groups present.
5. The six catalog items on the Library wall, captured — a renderer with no
   organized seed is half-built.

## 8. Known limits, recorded rather than discovered later

- **The composer already has a `＋`.** `ATTACHMENT_ID` renders `"＋ Attach"`
  (`lib.rs:2738`). Two adjacent `＋` glyphs meaning different things is a
  collision. C1 puts the providers trigger FIRST in the row, before the route
  chip, and leaves Attach's label carrying its own word. If that still reads
  ambiguously in the capture, Attach loses the glyph — not the providers one.
- **egui has no scroll-snap.** The carousel's snap is hand-rolled against
  `ScrollArea` offset and `scroll_to_rect_animation`.
- **Kinetic scrolling is drag-only.** `ScrollArea` keeps a velocity
  (`scroll_area.rs:44`) for drag/touch. A mouse wheel gets egui's own smoothing
  and no momentum. "Seamless" over a wheel means the animated path, not
  inertia.
- **The API grid is empty on this machine.** `chat/profiles` has no authored
  profiles, so the section's honest first state is the empty one. It must be
  designed, not left as a void.
