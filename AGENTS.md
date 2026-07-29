# DevPULSE Labs — Pure Rust. The Contract.

All UI is driven by Rust. There is no webview, no JS, no second language, no
IPC boundary. Signals are Rust enums on tokio channels; rustc is the type
system, end to end.

EguiPilotAI's canonical architecture is `docs/plans/eguipilotai.md`. Its
private inherited-pipe control channel is the ratified test/devtools-only
exception to the no-IPC production law. It must remain feature-gated; normal
production builds contain no runtime endpoint or MCP server.
Implementation order is `docs/plans/eguipilotai-v1-delivery.md`; current
evidence, gaps, and the DevPULSE census are indexed at
`docs/research/eguipilotai/README.md`.

Chat's named provider adapters are the only ratified production-process
exception to the no-IPC law. The exact allowed boundary, launch gates, and
forbidden behavior are canonical in
`docs/plans/chat-provider-runtime-adr.md`; no generic command runner or
credential-handling path is permitted.

Chat's primary UI is an independent native peer window, never a hub route.
The canonical product, visual, route, and generated-surface contract is
`docs/plans/chat-window-product-vision.md`; architecture and delivery remain
canonical in `docs/plans/chat-architecture.md`.

## Session OS (every agent)

Default operations live in skill `session-os` (`.cursor/skills/session-os/`).
Boot from disk before acting: `docs/STATE.md` → `docs/ops/SESSION.md` →
`docs/ops/resume/CURRENT.md`. Journal near ~75–85% context / end of cut;
after compact or clear, trust the resume pack over chat memory. Ops ledger
≠ product `dpl-ledger`. Plan: `docs/plans/session-os.md`.

## Laws

1. **Plan before code.** Every pass has a plan in `docs/`. Reality contradicting
 the plan stops the pass — amend, then continue.
2. **Test the real artifact.** The exe proves itself: `cargo run -- --screenshot
   out.png` boots the real app, captures its own window, exits. Unit tests for
   logic; the screenshot for pixels. No claim without one of the two.
3. **Hyper-modular colocation.** One concern = one crate. Hubs are crates.
   Cross-crate access is public API only.
4. **Registry over enumeration.** New hub = implement `Hub`, one line in
   `app/main.rs`. The shell never names hubs. Unknown → degrade, never crash.
5. **No bare strings for identity.** `HubId`/`SubId` newtypes, declared once as
   consts in the owning crate.
6. **ColdLight is authored once.** Tokens come from the JSON source (prior
   repo); `crates/coldlight` is the generated Rust target (hand-seeded P0).
   The paint vocabulary: crisp edges, gradient tint, black diffusion scatter —
   never backdrop blur.
7. **Never delete content; copy, never move; never clobber.** Carried verbatim.
8. **ffmpeg is a subprocess, never a crate dep** (ADR-009, still law). The
   editor follows ADR-010 (`devpulse labs/docs/wiredfront/editor-spec.md`)
   with its render/job design intact.

## The migration pipeline (skills — `.claude/skills/`)

Porting the prior app runs through three skills, in order, each planning then
executing only its own part:

1. **`/port-capture <subsystem>`** — senior-dev audit of the prior repo: full
   capability inventory, disposition table (port-verbatim / reauthor / retire —
   no silent drops), produces the approved plan in `docs/plans/`.
2. **`/egui-build <plan>`** — executes the approved plan; Rust drives (typed ids,
   registry, pool colors, `Fluid::of`, paint vocabulary); mandatory self-review
   pass on the draft before commit.
3. **`/true-verify [routes]`** — the real-artifact gate: tests green, exe
   screenshots per route vs committed baselines in `docs/proof/`, gallery gate,
   and slides-are-blocks/templates enforcement.

## Reference

- `docs/foundation-plan.md` — the ratified start-over plan, salvage map included.
- The prior app (`C:\localdev\devpulse labs`) is the salvage source: engine
  Rust, token JSON, ADRs, research. Read from it; never build on it.
