# R8 — EguiPilot coach unification program

**Status:** AUTHORED 2026-08-07 from three research lanes (industry SOTA,
fabric-versioning patterns, house-convention census) — execution blocked on
operator ratification of ADR-019 and redlines on the R8a mock. Rungs run
one-branch-one-PR each, per-rung plans authored via /plan-code-change before
their code.

**Reads with:** `docs/design/adr-019-eguipilot-coaching-mode.md` (decisions) ·
`docs/design/mocks/coach-one-system-mock.html` (the ratifiable picture) ·
`docs/plans/loss-recovery-program.md` L18/Arc R8 (provenance).

## 1. Goal

The operator's ask, translated: one system (EguiPilot: debug mode + coaching
mode) that (a) censuses installed applications, (b) mints a **tailored
fabric** per app — the 101 common verbs plus that app's unique captures —
**pinned to the app version and maintained against its changelog**, (c)
exposes ONE typed seam that AI providers drive, (d) paints live objects
(spotlight, ring, arrow, callout, floating command) over foreign apps that
**respond to clicks and return a clear signal**, (e) answers with
**task-completed** UI, and (f) rewards **grounded progress** — for the
operator and for Clyffy's graduated-trust training.

## 2. Done-when (program level)

1. `grep -r "dpl-coach" crates/` → 0; the family lives at
   `crates/egui-pilot-ai/<leaf>/` under the ratified prefix exception.
2. ONE verb vocabulary: protocol `verbs.rs` is the source; the daemon's
   snake_case wire names resolve through an explicit serde compat table with
   a golden test; `UnsupportedStyle` refusals for Arrow/Outline/DimAround → 0.
3. ONE event truth: every coach/verb/progress fact stamped in the fabric
   COACH domain; the SSE stream is a projection (AG-UI grammar), proven by a
   test that replays both encodings from one fixture.
4. Fabrics carry locks: `.userdata/coach/fabric/<app-id>.lock.json` with
   `original`/`locked`/`digest`/`toolchain`/`changelog.feed`; the drift
   pipeline demo (DRIFT chip → heal-and-propose → re-mint) runs REAL against
   an app update on the estate VM.
5. The R8a mock's §1 scene reproduced on Windows (devpulse-runner-win-01):
   overlay over a real terminal, operator click → Triggered → effect
   contract → Satisfied → task-completed → progress tick. Proof shot beside
   `docs/proof/c2s-foreign-spotlight.png`.
6. 19 daemon verbs each have ≥1 non-curl caller (Teacher/Clyffy client).
7. Every registration in §5's table green under `cargo green`.

## 3. Boundaries

ADR-019 §3 verbatim (process exception, localhost, roles/names/actions cap,
content/ law, screenshots-never-hot-path, two-surface rule). Plus: no rung
starts before its /plan-code-change plan; the mock is the redline surface —
UI disagreements are settled there, not in code review.

## 4. The rungs

### R8b — one vocabulary (protocol)
**Builds:** `egui-pilot-ai-protocol/src/{coach.rs,verbs.rs}` — Touchpoint
gains `target: Locator | ScreenRect | FabricId`; the 101 catalog lands as
typed enums with the serde compat table; `CoachStyle` extended visuals
painted in the ONE shared painter (`egui-pilot-ai/src/coach.rs`), overlay
renders the same catalog components (ADR-019 D5). `PROTOCOL_VERSION` minor
bump + `tests/v2_7_golden.rs`. New capability consts `coach.verb.*`.
**Gate:** refusals → 0 for the three styles; both painters render one
fixture identically (golden image pair); compat-table golden green.

### R8c — one transport + one event truth
**Builds:** overlay speaks A2UI over the un-parked stdio loop
(`createSurface`/`updateComponents`/`deleteSurface`, catalog URI); daemon
facts → `publish_coach` (stamped); SSE re-emitted as projection in AG-UI
grammar with `STATE_DELTA` (RFC 6902); `discover` + dated
`DPL-Protocol-Version`; `pending_confirmations`/`acknowledged_confirmations`
on risky verbs; MCP thin adapter (snake_case verb_noun tools, stateless
2026-07-28 shape); hand-copied palette deleted.
**Gate:** one-fixture/two-encodings replay test; palette grep → 0;
confirmation round-trip proven in a scenario; MCP adapter lists tools
mirroring `/verbs` 1:1.

### R8d — the Windows lane + the changelog watch
**Builds:** `windows_{,interact,lifecycle,presence,window}.rs` (UIA per
ADR-020's answer; layered/transparent overlay window) behind the unchanged
`compose_screen_rect` algebra; `FabricStore` + `fabric.lock`
(`schemas/coach-fabric.v1.schema.json` + jsonschema pin test, Document law);
drift stages `observe`/`checkver` (winget manifests + registry
DisplayVersion first) /`diff`/`classify`/`can-i-coach`; census fn + drift
test per the census law.
**Gate:** the mock's §1 scene REAL over wezterm on devpulse-runner-win-01
(proof shot); a staged app update flips CURRENT → DRIFT → healed re-mint
with the approval step exercised; `cargo green` on the VM's runner.

### R8e — Teacher + Clyffy clients, gamify live
**Builds:** `teach` subcommand on the existing `eguipilotai` bin (one
product door, no second exe) driving the seam end-to-end; provider loop
(chat provider → verb → effect → task-completed) through the same door;
`LessonProgress` + `.userdata/coach/progress.*`; progress ticks as
`CoachEvent` variants; Clyffy's ladder reads/writes the same store; OTel
GenAI spans on every verb.
**Gate:** 19 verbs ≥1 non-curl caller each; one full lesson end-to-end on
video; meters move only on grounded events (a spam-messages test moves
nothing).

## 5. The naming table (ruled by ADR-019 D1/D2)

| Today | Becomes | Registrations (same-commit, per the doc-and-audit law) |
|---|---|---|
| `crates/coach-inventory` (dpl-coach-inventory) | `crates/egui-pilot-ai/desktop-inventory` (egui-pilot-ai-desktop-inventory) | members · EGUI_FREE (missing today) · CEILINGS row · architecture.md §3 bullet |
| `crates/coach-fabric` (dpl-coach-fabric) | `crates/egui-pilot-ai/desktop-fabric` (egui-pilot-ai-desktop-fabric) | same |
| `crates/coach-runtime` (dpl-coach-runtime) | `crates/egui-pilot-ai/desktop` (egui-pilot-ai-desktop) | same + spawn-audit notes carry over |
| `crates/coach-overlay` (dpl-coach-overlay) | `crates/egui-pilot-ai/overlay` (egui-pilot-ai-overlay), bin `eguipilotai-overlay` | members · bin declared · NOT EGUI_FREE · ORPHAN_ALLOWED pruned (both stale entries) |
| `crates/coach-daemon` (dpl-coach-daemon) | `crates/egui-pilot-ai/daemon` (egui-pilot-ai-daemon), bin `eguipilotai-daemon` | members · EGUI_FREE · `/health` self-description → ADR-019 |
| `crates/hubs/coach` (dpl-hub-coach) | stays (hub law) — repoints to the renamed crates | hub-lateral check unchanged |
| `crates/desktop-spike` | stays as the archaeology spike, ORPHAN_ALLOWED entry pruned-or-honest | comment refreshed |
| verb catalog ADR + overlay ADR + control-plane ADR (plans) | supersession headers → ADR-019, bodies verbatim | STATE + roadmap rows |
| bins `coach-inventory-list`/`coach-fabric-mint`/`coach-bridge-probe` | `eguipilotai-inventory`/`eguipilotai-mint`/`eguipilotai-probe` (kebab, family-consistent — fixes the split bin styling) | [[bin]] declared |

## 6. Research appendix (the bleeding edge, with sources in the reports)

**SOTA (agent-desktop):** Anthropic `computer_20251124` dated tool types;
OpenAI GA `computer` tool + `pending_safety_checks`/acknowledge (the typed
HITL wire pattern we adopt); MCP `2026-07-28` stateless (+`server/discover`,
reverse-DNS extensions, `input_required` round-trips, CIMD auth); A2UI v1.0
(`createSurface` family, catalogId-as-URI, component consts); AG-UI 17-event
grammar (`STATE_DELTA` = RFC 6902); MCP Apps `ui://` for trusted panels in
chat clients; UFO² control-graph + hybrid tree-first/visual-fallback (still
SOTA); OTel GenAI `invoke_agent`/`execute_tool` spans. Full findings +
URLs: the R8 research reports (session 2026-08-07).

**Fabric versioning:** correlate-on-observation (winget/Intune idiom);
ARP DisplayVersion ↔ AppsAndFeaturesEntries bridge; AppStream `<releases>` +
Flathub API v2 as machine-readable changelog feeds; Scoop `checkver` as the
upstream watch primitive; flake.lock `original`/`locked` + SRI digest;
Playwright toolchain-coupling + aria-snapshots as golden trees; oasdiff
ERR/WARN/INFO with policy overrides; buf consumer-perspective severity;
Pact can-i-deploy matrix; UiPath Object Repository versions + Healing Agent
(heal-and-propose, in-place, never silent); NVDA appModules as the
per-app-semantic-map precedent.

**House census:** naming law (`dpl-` + folder; `egui-pilot-ai*` ratified
exception; kebab bins, snake bin files, declared); grouped-folder threshold;
`linux_*.rs` sibling-file platform pattern (no platform/ dirs);
13 xtask registration points enumerated; ADR register state (next: 019/020;
012 stays a gap; supersession-header precedent adr-016); Ring 0
`.userdata/<subsystem>/` persistence law + Document law (`v`, refuse-newer,
tmp→bak→rename) + schema pin precedent.

## 7. Sequencing + collaboration

1. **Operator:** redline the mock; rule ADR-019 (`go` / edits). Q-open:
   ADR-020's UIA route (pure-Rust UIA vs ADR-009 subprocess helper).
2. R8b → R8c → R8d → R8e, one branch/PR each, per-rung plan first, journal
   per pass, STATE row per landing — the loss-recovery §3 ledger closes
   L18 at R8e.
3. Estate: R8d develops against devpulse-runner-win-01 (the Windows VM on
   server1) — the same seat CI already proves.
