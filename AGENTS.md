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

Chat's named provider adapters are the only ratified **production-process**
exception to the no-IPC law. The exact allowed boundary, launch gates, and
forbidden behavior are canonical in
`docs/plans/chat-provider-runtime-adr.md`; no generic command runner or
credential-handling path is permitted.

`dpl-coach-overlay` is the ratified **devtools-only** process exception for
foreign/whole-monitor coach chrome (`docs/plans/coach-overlay-adr.md`).
Normal production builds contain no overlay endpoint; C2b lands behind the
same discipline as EguiPilot's inherited pipe.

Chat's primary UI is an independent native peer window, never a hub route.
Chat is built natively in THIS repo from the operator's own quoted
requirements — never copied from, linked to, or run beside chatmaton,
automaton or Shippin (operator ruling 2026-10-10). The canonical program is
`docs/plans/chat-native-program.md`. The earlier chat visions were deleted
(readable at `git show docs-history-2026-10-12:<path>`): a chat item counts
only where `docs/requirements.md` carries the operator's words.
Anything reaching outside this repo must be listed in
`docs/plans/external-reach-inventory.md` for operator confirmation first.

## Session OS / milestone-loop (every agent)

One loop, two surfaces. Read, in order:
1. `docs/status.md` (where we are);
2. `docs/requirements.md` (the operator's words);
3. `docs/README.md` (the doc map: one doc per subject).

The surfaces:
- Cursor: skill `session-os`, with hooks and agents under `.cursor/`.
- Claude: skill `milestone-loop`, with hooks under `.claude/ledger/`.

Journal at about 75–85% context or at the end of a cut. After a compact or
clear, trust `status.md` and the newest journal over chat memory. The ops
ledger is not the product `dpl-ledger`. The loop contracts are the skills
themselves.

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
   WiredFRONT editor's truth is the doc `docs/README.md` names for it.

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

- `docs/README.md` — the map of every doc, and the one truth for each subject.
- The prior app (`D:\localdev\review-archive\devpulse labs`) is the salvage source: engine
  Rust, token JSON, ADRs, research. Read from it; never build on it.
