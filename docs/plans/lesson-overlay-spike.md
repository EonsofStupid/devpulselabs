# Lesson overlay spike — can a window show the desktop through it?

Status: probe authorized 2026-07-27. Non-product; decides the shape of the
lesson overlay and of C2's foreign-app painting.

## 1. The question

The operator's original ask is "a semitransparent window that provides the
steps of the lesson". Two very different mechanisms hide inside that phrase,
and conflating them is the trap:

- **Over our own app** — no OS transparency is involved. The coach already
  paints a dimming scrim and a spotlight inside our own window, proven by
  `docs/proof` and by the C1.5d capture. Nothing here is unknown.
- **Over foreign applications** (Blender, SolidWorks, a browser) — the window
  must show *other processes' pixels* through it. This needs either real
  surface alpha or a shaped window, and neither has ever run in this repo.

This spike answers only the second, because it gates both the overlay half of
the lesson product and the "paint the whole monitor" half of C2.

## 2. Ground truth, measured this session

Not recalled — run on this box (ARM64, X11, GNOME 46, `DISPLAY=:1`).

**The APIs exist.** egui 0.35 `ViewportBuilder` carries `with_transparent`,
`with_mouse_passthrough`, `with_window_level`, and `with_always_on_top`. The
repo uses only the last, in `crates/viziwiz` (palette and editor).
Transparency, passthrough, and window level have never been exercised here.

**Vulkan on this X11 surface cannot composite alpha:**

```
supportedCompositeAlpha: count = 2
    COMPOSITE_ALPHA_OPAQUE_BIT_KHR
    COMPOSITE_ALPHA_INHERIT_BIT_KHR
```

Only one surface block is reported and it offers neither pre- nor
post-multiplied alpha. This is the ordinary Vulkan/X11 result, not a local
misconfiguration.

**egui-wgpu has no `INHERIT` path.** `egui-wgpu-0.35/src/winit.rs` selects
`PreMultiplied`, else `PostMultiplied`, else the surface default, and only when
`support_transparent_backbuffer` is set. That flag is decided **once** in
`eframe-0.35/src/native/wgpu_integration.rs` from the **root** viewport's
`transparent`, then shared by every viewport on that painter.

**The inference drawn from the two facts above was wrong, and the probe caught
it.** Reading them together says "no pre- or post-multiplied alpha, and
`egui-wgpu` asks for nothing else, therefore transparency is impossible on
X11". §8 shows a see-through window. `COMPOSITE_ALPHA_INHERIT_BIT_KHR` does not
mean *no* alpha; it means the alpha behaviour is **inherited from the native
window**. When `transparent(true)` makes winit pick the depth-32 ARGB visual
the X server already advertises, the compositor honours the alpha channel
regardless of the swapchain's composite-alpha hint. Capability enumeration
described the swapchain, not the outcome.

This is law 2 doing its job, and it is worth leaving in the record rather than
quietly editing out: a confident, evidence-backed, source-corroborated
prediction was refuted by ten seconds of running the actual binary. The
evidence was real; only the conclusion drawn from it was not.

One consequence does survive, and it shapes the architecture. The transparent
backbuffer is decided **once**, from the root viewport, and shared across every
viewport on that painter. A transparent *peer* window inside `devpulselabs`
would therefore require making the **main** window transparent-capable — an
app-wide change to satisfy one panel.

**Operator ruling, 2026-07-27: the overlay is its own process**
(`dpl-coach-overlay`), with its own root viewport. That avoids the constraint
rather than working around it, keeps the main window's rendering untouched,
and is already the shape C2 wants for whole-monitor painting — so the lesson
overlay and the foreign-app overlay are one artifact, not two.

**Findings remain platform-local.** DXGI supports pre-multiplied alpha and
Wayland carries surface alpha natively, so both other targets are plausible,
but neither is proven until the probe runs there.

## 3. What the probe must decide

One question, empirically, on the real artifact: **with `transparent(true)` and
`mouse_passthrough(true)`, does the desktop show through the window's cleared
pixels on this box?**

**Answered yes** (§8). Real per-pixel alpha is available, so the overlay is not
restricted to the binary in-or-out marks an X11 SHAPE cut would give. Partial
dimming, soft-edged spotlights, and the existing ColdLight scrim vocabulary all
transfer directly to a foreign-app overlay — the paint language does not have
to fork between "over our window" and "over Blender".

The shaped-window fallback is therefore **not needed** and is recorded here as
a considered, rejected alternative rather than a pending option.

## 4. The probe

`crates/desktop-spike`, a new workspace member and **not** a dependency of
`crates/app`, so nothing it contains can reach a production build. No new
external dependencies: `eframe` and `egui` are already workspace deps.

One binary, `overlay-probe`:

- builds its viewport with `with_transparent(true)`,
  `with_mouse_passthrough(true)`, `with_always_on_top()`,
  `with_decorations(false)`, at a given size and position;
- returns a fully transparent `clear_color`;
- paints only two opaque marks — a thick ring and a labelled card — so a
  screen capture can distinguish "cleared to transparent" from "cleared to
  black";
- prints the adapter and backend it actually got;
- exits on its own after `--seconds N`, so a failed probe can never strand a
  click-through always-on-top window over the operator's desktop.

## 5. Boundaries

- Declares no new capability, ships no product surface, and is not reachable
  from `devpulselabs`.
- Does not touch the coach, the pilot protocol, or the shell.
- Does not decide C2. It supplies C2's paint-layer premise; the ADR still
  follows the C2s findings.
- Self-terminating by construction (§4), because an always-on-top
  click-through window that outlives its probe is an operator hazard.

## 6. Verification

The exe proves itself, per law 2:

1. Run `overlay-probe` positioned over a known, visually busy region.
2. Capture the **screen** (not the window) with `gnome-screenshot`, since a
   window capture cannot show what is behind it.
3. Read the capture: desktop visible inside the ring means alpha works; a
   black or solid interior means it does not.
4. Record the verdict here in §8 either way. A negative result is the
   deliverable, not a failure.

## 7. Risks

- **A capture may composite differently from the screen.** Mitigated by
  capturing the full screen through the compositor rather than the window.
- **`mouse_passthrough` is not verified by this probe.** Reported honestly:
  proving click-through needs a synthetic click and a witness window beneath,
  which is its own small pass.
- **Findings are platform-local.** Nothing measured here transfers to the
  Windows laptop or the Wayland mini-PC without re-running it there.

## 8. Evidence

Run 2026-07-27 on this box (ARM64, X11, GNOME 46, mutter compositing).

Both runs placed a 480×360 overlay at (200, 200), over a Cursor window, and
captured the **screen** with `gnome-screenshot`:

```
overlay-probe --at 200,200 --size 480x360 --seconds 9
overlay-probe --at 200,200 --size 480x360 --seconds 9 --opaque
```

Both reported the same renderer, so the renderer is not a variable between
them:

```
renderer: wgpu · backend Vulkan
adapter llvmpipe (LLVM 20.1.2, 128 bits) (Cpu)
```

With `transparent = true`, the Cursor sidebar reads clearly through the
overlay and only the cyan ring and the label card are painted. With
`transparent = false`, the identical region is a solid black rectangle that
fully occludes the window beneath.

**Verdict: `with_transparent(true)` produces a genuinely see-through window on
X11 with `eframe` + `wgpu`.** The control run is what makes this conclusive —
one transparent capture alone could have been a compositor artifact; the pair
differing only in that flag could not.

Not proven by this run, and not to be assumed:

- **`mouse_passthrough`.** The window was built with it, but nothing here
  demonstrates a click reaching the window beneath. Needs a synthetic click and
  a witness — its own small pass.
- **Hardware adapters.** llvmpipe was selected. NVIDIA may enumerate different
  alpha modes.
- **Windows and Wayland.** Unrun.

Captures: `/tmp/shot_transparent.png`, `/tmp/shot_opaque.png` (scratch, not
committed; re-runnable in about twenty seconds from the two commands above).

### 8.1 Mouse passthrough (p2b) — 2026-07-29

Binary: `passthrough-probe` in `crates/desktop-spike` (plan
`docs/plans/p2b-overlay-passthrough.md`). Same box: ARM64, `DISPLAY=:1`,
X11, wgpu Vulkan / **NVIDIA GB10**.

Control pair (orchestrator spawns witness → overlay → XTEST click at
overlay center):

```
passthrough-probe --hit-file /tmp/dpl-passthrough-on.txt \
  --seconds 8 --at 200,200 --size 400x300
# → hit=true passthrough=true expect_hit=true → PASS

passthrough-probe --hit-file /tmp/dpl-passthrough-off.txt \
  --seconds 8 --at 200,200 --size 400x300 --no-passthrough
# → hit=false passthrough=false expect_hit=false → PASS
  # (control forces --opaque on the overlay so it can own the pointer)
```

**Verdict: `with_mouse_passthrough(true)` on a transparent always-on-top
overlay delivers a synthetic click to a witness underneath on this X11
box.** The opaque `--no-passthrough` control is what makes it conclusive.

Caveat recorded during the probe: a *transparent* overlay with
passthrough off and only a hollow painted ring still let the click
reach the witness (alpha / hollow center). The negative control therefore
uses an opaque overlay window. Product chrome that must eat clicks needs
opaque (or input-shaped) pixels under the pointer — not hollow
transparent interiors alone.

Still unproven: Windows laptop, Wayland mini-PC.
