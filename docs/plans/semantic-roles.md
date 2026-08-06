# Pass — semantic roles: widgets bind to MEANING, treatments fill it

## 1. Context

**Operator ruling, 2026-08-05:**

> "chrome (ordinary), hero (the one primary action), quiet (passive/content),
> danger (destructive). Widgets bind to those; treatments fill them."

This is the **semantic layer** — the tier industry practice puts between raw
tokens and components (core → *semantic* → component). Its absence is why, an
hour ago, every composer helper had `vibron` written into it, and why the
workbench has `"neubrutalist"` written into *its* control function.

**It already exists, half-built and trapped in one hub.**
`crates/hubs/workbench/src/controls.rs:28` declares:

```rust
pub(crate) enum Weight { Quiet, Loud }
```

…and `:69` hardcodes the treatment anyway:

```rust
let style = cl::compose(tone, "neubrutalist", &[], state);
```

So the workbench independently invented half of this, kept it private, and
still hardcoded the answer. That is the exact "six competing implementations
of a pressable thing" the preset POA&M documented. **This pass promotes it,
widens it to the four ruled roles, and makes the binding data.**

## 2. Current state (read in-session 2026-08-05)

| where | what it does | defect |
|---|---|---|
| `hubs/workbench/src/controls.rs:28` | `Weight { Quiet, Loud }`, `pub(crate)` | 2 roles not 4; private; hardcodes `"neubrutalist"` at `:69`; hardcodes the Loud edge at `:83` |
| `composer/src/lib.rs` | `cl::chrome_treated(...)` everywhere | **1 role only** — no hero/quiet/danger, so New/Save/Render are still saturated by accident rather than by meaning |
| `coldlight/src/lib.rs` | `chrome()` / `set_chrome()` / `chrome_treated()` | global, single-role. The right shape, one tier short |
| `tokens/traits.json` | 5 treatments: `neubrutalist · claymorphic · cyber · retro · vibron` | no binding from meaning → treatment |

**Nothing binds meaning to treatment.** A widget must therefore name a
treatment, which is the hardcoding.

## 3. Target design

### 3.1 The 10th authored source — `tokens/roles.json`

```jsonc
{
  "$comment": "The 10th authored source. A ROLE is what a control MEANS; a
               treatment is how that meaning is painted. Widgets bind to
               roles and never to treatments — that is what makes the whole
               app re-dressable from data.",
  "default": {
    "chrome": { "trait": "vibron" },
    "hero":   { "trait": "neubrutalist" },
    "quiet":  { "trait": "claymorphic" },
    "danger": { "trait": "neubrutalist", "tone": "coral" }
  },
  "surfaces": {
    "wiredfront": { "chrome": { "trait": "vibron" } },
    "workbench":  { "chrome": { "trait": "claymorphic" } }
  }
}
```

**Per-surface, with per-role fallback to `default`** — the same M2 rule the
matrix already uses, so a surface only states what it *changes*. Global
dressing is then the special case where no surface overrides anything, which
means we get both models from one mechanism rather than choosing.

**A role may pin its own tone.** `danger` is coral on every surface — "are
you sure you want to delete this" must not turn lime because the hub's tone
is lime. Roles that omit `tone` inherit the caller's.

### 3.2 The API

```rust
pub enum Intent { Chrome, Hero, Quiet, Danger }

/// A control dressed by what it MEANS on a given surface.
pub fn dressed(surface: &str, intent: Intent, tone: Tone, state: TraitState) -> Treated;
```

Named `Intent`, not `Role`, because `egui_pilot_ai::protocol::Role` is
already imported across these files and a second `Role` would be a
readability trap.

`chrome_treated` becomes `dressed(surface, Intent::Chrome, …)`; the existing
`vibron`/`brutal`/`clay` stay for the case where a component means one
treatment ON PURPOSE — which is what lets glass sit inside a brutalist
container.

### 3.3 What this buys immediately

The editor's remaining saturated controls stop being accidents:

| control | today | with roles |
|---|---|---|
| tool strip, transport, rows | vibron (correct) | `Chrome` |
| **New / Save** | saturated lime/cyan | `Chrome` — they are ordinary |
| **Render** | saturated magenta | **`Hero`** — it IS the primary action |
| **the timeline's delete** | saturated magenta | **`Danger`** — coral, on every surface |
| Info / FX panels | ad-hoc | `Quiet` |

One saturated hero per surface, which is what `neubrutalist.png` shows and
what the mock shows.

## 4. Changes, in execution order

1. `tokens/roles.json` — the source above.
2. `coldlight/build.rs` — `generate_roles`: emit `ROLES`, the surface table,
   and a resolver fn. **Build fails** on an unknown trait ref, an unknown
   tone, or a role missing from `default` (every role must have a fallback).
3. `coldlight/src/lib.rs` — `Intent`, `dressed()`; keep `chrome()`/
   `set_chrome()` as the runtime override of the *default* surface.
4. `composer/src/lib.rs` — helpers take an `Intent`; New/Save → Chrome,
   Render → Hero, delete → Danger, panels → Quiet.
5. `hubs/workbench/src/controls.rs` — `Weight` → `Intent`, delete the
   hardcoded `"neubrutalist"` and the hardcoded Loud edge; `control()` calls
   `dressed("workbench", …)`.
6. Tests: every role resolves on every surface; a surface stating one role
   inherits the rest; `danger` keeps coral against a lime caller; an unknown
   trait ref fails the build.
7. Proof: `--route wiredfront/editor` at three sizes — exactly ONE saturated
   control (Render), everything else a plate.

## 5. Boundaries

**🚫 Never touch**
- `/content`; generated `*_gen.rs`; the mock.
- `traits.json`'s five treatments — this pass binds them, it does not retune
  them.

**⚠️ Ask first**
- Adding a FIFTH role. Four is the ruled set; a fifth is a vocabulary change.
- Changing `danger`'s tone away from coral.

**Out of scope**
- `resolve()` / the matrix migration — roles bind to TRAITS today and will
  bind to COORDINATES afterwards with no call-site change, which is the point
  of the indirection.
- The manifest; panel borders and `.ptab`s; symlink browse reach.

## 6. Verification

- `cargo green`.
- **Break-it:** point a role at `"vibrn"` → build fails naming the role.
  Delete `hero` from `default` → build fails (no fallback). Revert both.
- `rg '"neubrutalist"' crates/hubs crates/composer` → **zero** hardcoded
  treatments outside coldlight.
- Proof shots: one hero per surface; `danger` reads coral on a lime hub.

## 7. OPEN — presets for the other ancestors

The operator asked this back rather than ruling, so it is recorded, not
guessed:

> "Do the other ancestors get presets now? aero + neo/hacker/retro, or is
> brutalist the only lineage with presets until later?"

**Recommendation: author what is NAMED, let the structure carry the rest.**
Today that is brutalist's four (`neo · hacker · retro-os · vibron`) plus
`aero + hacker`, which the operator named for Workbench. Authoring 3 presets
× 8 ancestors would be 24 presets, most of them invented rather than
designed — and the legality system means an unused one still costs rulings.

Roles make this cheap either way: a preset is just another treatment a role
can point at, so adding one later changes `roles.json`, never a widget.

## 8. Evidence

*(filled during execution)*
