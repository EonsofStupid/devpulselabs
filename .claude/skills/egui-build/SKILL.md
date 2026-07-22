---
name: egui-build
description: Execute an approved port plan in the pure-Rust egui app — Rust drives everything, no shortcuts, draft then a mandatory self-review/improve pass before commit. Carries the egui 0.35 API facts and the ColdLight paint vocabulary.
---

# egui-build — the execution pass

**Argument:** the path to an approved plan (normally `docs/plans/port-*.md` produced by
`/port-capture`, or another approved plan in `docs/`). No plan → stop and run
`/port-capture` first. Never build from memory or a compaction summary.

## Mission

Turn the plan into committed, working egui code with **Rust properly driving**: typed
identities, registry-driven surfaces, generated tokens, painted (not styled) chrome.
This is a focused-context pass — the plan already did the archaeology; you re-ground
against the plan and the target files, then execute cleanly, then run a genuine
review-and-improve pass on your own draft before the commit.

## Rust-drives checklist (violations = the pass is wrong, fix before review)

- **No bare strings for identity** — `HubId`/`SubId` newtypes, consts in the owning
  crate. Route data flows through `Route`, never parsed strings.
- **Registry over enumeration** — a new hub/component is one `Hub` impl + one line in
  `crates/app/src/main.rs`. The shell never names hubs. Unknown → degrade, never crash.
- **Color comes from the pool** — `dpl_coldlight::ramp(id)` / `Tone::color()`. Never a
  hardcoded hex/rgb in a hub or widget. New hues are authored in
  `crates/coldlight/tokens/hues.json` and arrive via `cargo build` (the build.rs
  generator; the never-pure-black floor panics the build).
- **Sizing comes from Fluid** — `cl::Fluid::of(rect)` (container-based) for anything
  that must scale; new fluid tokens go in `tokens/fluid.json`. No magic px ladders.
- **The paint vocabulary** — crisp edges, solid fill + `gradient_tint` mesh, black
  `diffusion` scatter, `sheen`/`seam`/`frame_overlay` for chrome. **Never blur, never
  feathered borders.** `scatter_frame`/`card_frame` for cards.
- **Signals on the fabric** — typed per-domain enums on the tokio broadcast
  (`ShellEvent` pattern); never stringly-typed events, never synthesized events a
  source didn't emit.
- **ffmpeg stays a subprocess** (ADR-009). No ffmpeg-sys/-next crates, ever.

## egui 0.35 API facts (learned by reading the crate — do not fight them)

- Panels are unified: `egui::Panel::{top,bottom,left,right}(...)` with `.exact_size()`,
  `.show_separator_line(false)`, `.frame(...)`; `.show(ui, ...)` takes `&mut Ui`.
  `TopBottomPanel`/`SidePanel` are gone.
- The app trait is `eframe::App::ui(&mut self, ui: &mut Ui, frame)` — not `update`.
- Styles are theme-scoped: `ctx.style_of(Theme::Dark)` / `set_style_of` /
  `set_theme(ThemePreference::Dark)`.
- `Shadow { offset: [i8; 2], blur: u8, spread: u8, color }`; `CornerRadius` is u8;
  strokes use `StrokeKind::Inside` for crisp inner edges.
- Screenshots: `ViewportCommand::Screenshot(UserData)` → `Event::Screenshot { image }`
  (the harness in `crates/app` already does this).
- Glyph coverage is narrow — many unicode symbols render □. Verify any new glyph in a
  screenshot, or use the sprite/vector path instead of text.
- epaint has no arc primitive — ring sectors and curved shapes are triangle-strip
  meshes (`Mesh` + `quad()`-style helpers in coldlight).
- When an API surprises you, read the crate source in `~/.cargo/registry` — do not
  guess from pre-0.35 knowledge.

## Protocol

1. **Re-ground.** Read the plan in full. Read every target file the plan names as it
   exists NOW. `git status` + `git log --oneline -3`. If reality contradicts the plan,
   STOP, amend the plan, say so — never improvise a divergent build.
2. **Execute in plan order.** One step at a time. Reality-contradiction rule applies
   at every step.
3. **Draft gate.** `taskkill //F //IM devpulselabs.exe` (strays lock the exe), then
   `cargo build` and `cargo test --workspace` — both must be clean before review.
4. **Review-and-improve pass (mandatory, not optional).** Re-read the full diff
   (`git diff`) against the plan and the checklist above, hunting: checklist
   violations, dead code / lying comments, duplicated paint code that belongs in
   coldlight, missed plan steps, and un-gated additions (anything generated or
   registered that no surface renders — the gallery-gate law). Apply the improvements.
   Rebuild, retest.
5. **Commit** per the plan's commit plan (heredoc for multi-line messages in Bash).
   Report any deviation from the approved plan explicitly — never silently.
6. **Hand off** to `/true-verify` for the real-artifact proof. Do not claim visual
   success yourself without a screenshot.
