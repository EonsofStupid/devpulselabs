# 2026-08-11 — Unified AI kernel: merging DevPULSE chat + automaton

> **SUPERSEDED 2026-10-10.** The merge is cancelled by operator ruling; chat
> is built natively in this repo (`docs/plans/chat-native-program.md`). Its
> "exact next step" list below must NOT be resumed. Kept as history.

Cross-repo program. `EonsofStupid/automaton` (the kernel) ⇄ this repo (a
consumer). Also carries the day's DevPULSE work (egui 0.36, gates, sccache).

## Second checkout

**automaton is cloned at `C:\localdev\automaton`** (git-dependable private
repo). Its Rust workspace: `mato-{acp,providers,kernel,cli,daemon}`; a SEPARATE
Node `engine/` prototype (all-five providers demo — not in the cargo
workspace). Build automaton locally (small) or on server1.

## Done so far

**The merge program (plan: `docs/plans/unified-ai-kernel-automaton-merge.md`,
DevPULSE #243 merged).** Grounded both repos: same design pattern, incompatible
types, COMPLEMENTARY halves — automaton brings the leveled `CapabilityProfile`
+ decomposed identity (`AccessClass`/`ActivationKind`/`AuthState`/`Health`) +
signal journal + multi-product framing; DevPULSE brings the 22-wide session
vocabulary + the live `ProviderRuntime` execution seam (Codex + Grok spokes) +
store + egui. Three ratified decisions: **A** automaton is authority (DevPULSE
depends via git), **B** open `ProviderId` (not automaton's closed enum), **C**
Rust execution. Subsumes the never-executed ai-chat-social separation.

**Phase 1 — canonical capability vocabulary (automaton #8, MERGED, both lanes
green).** Unioned automaton's 15 + DevPULSE's 22 into one canonical
`Capability` in `mato-providers/src/capability.rs`, additive & wire-stable
(automaton's 15 kept exact; 17 DevPULSE concepts added). The reconciliation is
executable — `Capability::from_devpulse_slug` folds the 5 synonyms
(`stream_turns→streaming` etc.). 5 tests prove it lossless.

**The agnostic seam (automaton #9, MERGED) — `docs/providers/00-agnostic-seam.md`.**
Every provider's API listed one by one (codex app-server+exec, grok ACP+-p,
claude stream-json, gemini/cursor -p), all mapped to ONE canonical path:
DevPULSE's `RuntimeCommand` (out) + automaton's events (in) unified. Vendor
names stay inside adapters. The **alias layer**: canonical action = stable id
(keys logic); a rotating display word pool is pure surface
(`summarize → striving/vibin/churnin/flibbertygibbeting`), authored as data.

**Execution primitives (automaton #10, OPEN) — `docs/19-execution-primitives.md`.**
The differentiator: **ROUTINE** (scheduled task) · **WORKFLOW** (default vs
advanced; advanced = user sub-agent dial: none | capped(n)) · **PASS**
(single-concern specialist: security / canonical-consistency / UI-layout /
pattern-adherence, each a template with 3 seed configs spanning
depth×subagents×severity). Composition is RECURSIVE: a routine may run several
workflows; a workflow stage may be a nested workflow. Plus the **canvas**: a
visual builder to drag/wire/run flows and watch the agent DRY-RUN each node
(no side effects) before a real run — operator to supply example flows.

## Exact next step

1. **Merge automaton #10** (execution primitives) once its CI is green.
2. **Gemini + Cursor Rust-drivability probe** — Decision C's open risk. codex
   (app-server), grok (ACP), claude (stream-json) are Rust-drivable; gemini and
   cursor are `-p` headless. Probe whether they can do multi-turn in Rust, or
   accept they run one-shot behind the daemon. This decides whether all five
   collapse onto ONE Rust path.
3. **Phase 2 of the merge** — DevPULSE consumes `mato-providers` via git,
   deletes its duplicate `Capability`/roster, `qualified_capabilities` moves to
   the kernel. Biggest single diff; needs the estate CI runners to auth-clone
   the private automaton repo (server1 already can).
4. **The canvas** — awaits the operator's example flows; it is a VIEW over the
   routine/workflow/pass model, never a second model.

Open primitive questions to settle with the operator: exact schedule
vocabulary for routines; whether a pass may spawn sub-agents in DEFAULT
workflow mode or only advanced (leaning: pass inherits the workflow ceiling; a
standalone routine-pass sets its own).

## Landmines

- **NEVER `cargo fmt --all` in automaton.** It is hand-formatted with TABS and
  has no rustfmt.toml, so `fmt --all` reformats every file tabs→spaces. It bit
  me this session on `mato-daemon`/`mato-kernel` — reverted, and I PROVED no
  operator work was lost by regenerating the identical fmt diff. Touch only the
  file you mean to.
- **`C:\localdev\automaton\docs\18-depot-staging.md` is the OPERATOR's untracked
  file.** Not on remote, not mine. Leave it.
- **automaton's gate ≠ DevPULSE's.** It is `cargo test --workspace` + release
  build + `npm test` — NO `-D warnings` clippy. Match it; do not impose
  DevPULSE's stricter standard or "fix" its pre-existing `let tags;` lint.
- `ProviderId`: automaton CLOSED enum vs DevPULSE OPEN newtype — Decision B
  keeps OPEN (extensibility); do not convert DevPULSE to a closed enum.
- DevPULSE-side (unchanged): build on server1 not the laptop; sccache tier-1 is
  live on both (`D:\sccache` 50G / `/root/.cache/sccache` 20G); egui bumps are
  ONE PR; a manifest change MUST commit its lock (now gated).

## Verify commands

```
# automaton (small, local or server1):
cd C:/localdev/automaton && cargo test -p mato-providers      # 5 merge tests + rest
cargo test --workspace && cargo build -p mato-cli -p mato-daemon --release   # its CI gate

# DevPULSE gate (server1, fast):
ssh ssh.clyffy.ai; source $HOME/.cargo/env && source $HOME/.sccache_env
cd /root/dev/devpulselabs && cargo green      # green ✔, smoke + lock included
```
