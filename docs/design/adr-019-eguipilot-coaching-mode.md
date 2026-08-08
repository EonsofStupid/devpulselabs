# ADR-019 — EguiPilot's coaching mode: one system, one vocabulary, two doors

**Status:** PROPOSED 2026-08-07 — authored from the operator's rulings of the
same day; awaiting operator `go`. The mock gate is already satisfied:
`docs/design/mocks/coach-one-system-mock.html` (R8a, render-verified).

**Ratification record.** Operator, 2026-08-07, verbatim essence: *"this and
eguiPilot are the same — we have its debug mode and coaching mode… properly
have it properly expanding eguiPilot"*; *"I created an API that had all of the
common any APP blender, solid works would have and then the different unique
get captured so we can have tailored fabrics that get maintained following
their changelogs… have the seam for the API so providers can utilize and not
only create visuals that respond to clicks and send a clear signal and the UI
responds with the task completed"*; *"the file and folder naming and patterns
align with platform industry and how a true senior dev would author."*

**Related:** `docs/plans/r8-eguipilot-coach-unification-program.md` (the arc) ·
`docs/plans/loss-recovery-program.md` L18/Arc R8 (the census that found the
split) · supersedes the plans-filed coach ADRs (see §6).

## 1. Context

The L18 census (2026-08-07) established: the live-overlay-over-foreign-apps
capability is fully merged and proven (verb battery 25 PASS; foreign spotlight
proof shot; Cursor located by fabric path) but stranded twice — everything is
`cfg(linux)`, and every unification seam between the coach constellation and
EguiPilot was parked: two verb vocabularies, two transports, two event
streams, a hand-copied palette, zero crate references in either direction.
The operator ruled the two ARE one system. This ADR is the unification's
decision record; the program doc carries execution.

## 2. Decisions

### D1 — the family name is `egui-pilot-ai-*`
The coach crates join the EguiPilot family. The `egui-pilot-ai*` prefix is an
already-ratified naming exception (`crates/xtask/src/main.rs` NAME_EXCEPT_PREFIX;
`docs/design/architecture.md` §"naming law", operator 2026-07-30 "not
everything has to be dpl-"). Taking these names requires ZERO new naming law —
which is the strongest possible house-pattern signal that this is a rename
home, not a new family. `dpl-coach-*` names retire.

### D2 — grouped folder `crates/egui-pilot-ai/<leaf>/`
At 7+ crates the family passes the architecture doctrine's ~3 threshold for
grouped folders, mirroring `crates/chat/<leaf>/` exactly. The structure audit
checks `name.ends_with(folder)`, so nesting composes with D1 unchanged.

### D3 — one protocol crate, one casing ruling
The 101-verb catalog merges INTO `egui-pilot-ai-protocol` (new `coach.rs` +
`verbs.rs` modules; `PROTOCOL_VERSION` minor bump; a `v2_7_golden.rs` wire
pin). **Casing law, three registers, each matching its home ecosystem** (the
2026 industry layering — MCP/Anthropic, A2UI, AG-UI):
- **kebab-case** stays canonical on the typed pilot wire (the existing
  `rename_all = "kebab-case"` law across the protocol crate);
- **snake_case `verb_noun`** is the register of the daemon verbs and the MCP
  adapter tools (the >90% MCP convergence; consistency-within-server is the
  accuracy-critical rule). The existing daemon wire names are kept via an
  explicit serde rename/compat table with its own golden test — never a
  silent break;
- **PascalCase** component consts + **camelCase** message names for the
  overlay surface protocol (A2UI v1.0 verbatim — D5);
- **SCREAMING_SNAKE** for the provider-facing event stream (AG-UI grammar — D6).

### D4 — two doors, stateless, confirmable
The pilot stdio pipe remains the debug/harness door. The axum daemon remains
the agent door (localhost `127.0.0.1:7843`, flat single-segment routes
unchanged). Additions, all industry-shaped:
- **Dated protocol versioning**: `DPL-Protocol-Version` header + dated tool
  types (`coach_20260807` pattern per Anthropic `computer_20251124`).
- **`discover`** capability RPC; capability strings namespaced reverse-DNS
  `ai.devpulse/pilot`, `ai.devpulse/coach`, `ai.devpulse/coach.overlay`
  (MCP 2026-07-28 stateless model).
- **Typed confirmations**: risky verbs return
  `pending_confirmations: [{code, message}]` with codes `sensitive_target` ·
  `destructive_action` · `foreign_app` · `injected_content`; the caller
  resubmits `acknowledged_confirmations` (OpenAI `pending_safety_checks`
  loop; maps 1:1 to MCP `input_required` round-trips). This is the wire form
  of the operator's graduated-trust ladder.
- **MCP thin adapter**: snake_case verb_noun tools mirroring the catalog;
  our daemon stays the source of truth; MCP is never a second brain (the
  ratified verb-catalog ADR's own law, kept).

### D5 — the overlay speaks A2UI
The separate overlay PROCESS stands (ratified exception: egui-wgpu fixes the
transparent backbuffer at root-viewport creation). The parked Pass-2 stdio
loop un-parks as an **A2UI v1.0 surface protocol**: `createSurface` /
`updateComponents` / `updateDataModel` / `deleteSurface`, `surfaceId` +
required `catalogId` (string-formatted-as-URI:
`https://devpulse.ai/coach/1.0/catalog.json`). The trusted component catalog
v1: `Spotlight` · `Ring` · `Arrow` · `Callout` · `CommandBar` · `Mask` —
the model supplies data + allowed actions, never executable UI. The in-app
coach painter and the overlay render the SAME catalog (one palette, the
hand-copy deleted).

### D6 — one event fabric, projected
Every coach fact — mount, trigger (with `CoachActor`), satisfy, verb effect,
drift, progress tick — is stamped into the dpl-fabric COACH domain (the
existing law: stamp-at-publish, the coach never subscribes to its own
domain). The daemon's SSE stream becomes a **projection** of those same
facts, emitted in AG-UI grammar (`RUN_*`, `STEP_*`, `VERB_*` mirroring
TOOL_CALL_*, `STATE_SNAPSHOT`, `STATE_DELTA` as RFC 6902 JSON Patch). One
truth, two encodings — never two truths.

### D7 — tailored fabrics, maintained following their changelogs
The operator's changelog requirement lands as an industry-standard artifact
pair in Ring 0 (ADR-014): `.userdata/coach/fabric/<app-id>.json` (the fabric
document) + `<app-id>.lock.json` (the pin). The lock is flake.lock-shaped:
`original` (what was asked) vs `locked` (what was resolved: app version, ARP
DisplayVersion, exe FileVersion + sha256, package ref), SRI `digest` over the
golden tree snapshot, `toolchain` coupling (minter + version, the Playwright
browsers.json pattern), and a `changelog.feed` (winget manifest / AppStream
releases / Flathub API v2 / Scoop-checkver-style URL watch).

**The drift pipeline, seven stages, each from a named precedent:**
`observe` (correlate-on-observation at attach: registry DisplayVersion → exe
FileVersion/sha → flatpak commit — nobody trusts push events) →
`checkver` (background feed poll: "update available, fabric will drift") →
`diff` (re-walk vs golden aria-style snapshot; typed change records) →
`classify` (severity per CONSUMER contract, oasdiff/buf-style: **ERR** = verb
target/seed removed or role changed · **WARN** = renamed/moved/action-set
shrank · **INFO** = additive; policy-overridable) →
`can-i-coach` (Pact-style fabric×app-version compatibility gate; ERR ⇒
fabric `stale`, coaching restricted to surviving nodes) →
`heal-and-propose` (re-anchor by role+name+neighborhood voting; PROPOSE a
re-mint diff for approval — update descriptors in place, never delete,
never silent: the UiPath doctrine) →
`re-mint` (approved ⇒ new lock; old locks retained per version; the release
notes attach so the coach can narrate "what changed in 4.3").

### D8 — telemetry speaks OTel GenAI
Spans: `invoke_agent <name>` / `execute_tool <verb>` with `gen_ai.tool.name`,
`gen_ai.tool.call.id`, `gen_ai.provider.name`; our attributes namespaced
`dpl.coach.*` / `dpl.pilot.*` (gen_ai.* is pre-stable). Content is sensitive
by default. The Parquet ledger already writes; it gains the coach lane.

### D9 — grounded progress, two learners
The gamify research folds in under the PBS contract verbatim: rewards
reflect **grounded progress** — ambiguity resolved, evidence added, decisions
approved, tests passed, foundations established — **never message volume,
agent count, or spend**. `LessonProgress` lives in the protocol crate; the
store in `.userdata/coach/progress.*` (Document law or Turso per ADR-017
precedent); ticks publish as new `CoachEvent` variants — not a second
channel. Clyffy's graduated-trust ladder (manual gates → whitelist → tracked
approvals, ruled 2026-08-06) is a grounded-progress track on the same store:
every approved impersonation step IS "decisions approved / tests passed".

### D10 — the Windows lane
`windows_*.rs` sibling files mirroring the linux five (no `platform/` dir —
nothing in this repo uses one), a third `#[cfg]` arm per verb, deps under
`[target.'cfg(target_os = "windows")'.dependencies]`. UIA must arrive
**without a `-sys` crate** or via an ADR-009 subprocess — that choice is
ADR-020's single question, answered before R8d code. Platform honesty holds:
X11 proven · Windows next (on the estate VM devpulse-runner-win-01) ·
wlroots third · GNOME Wayland unsupported.

## 3. Boundaries

- 🚫 The overlay and daemon are NEVER crate deps of `app` — spawn/HTTP only
  (the ratified process exception, kept verbatim).
- 🚫 Foreign-app reading stays capped at roles/names/actions (overlay ADR
  law); app-sourced content NEVER becomes instructions (OWASP LLM posture);
  the daemon never binds beyond localhost.
- 🚫 `content/` is operator data — fabrics live in Ring 0, never Ring 2.
- 🚫 Screenshots never the lesson hot path (import/harvest/proof only).
- The two-surface rule stands: Operator Coach and Advanced/EguiPilot must
  not collapse into each other — one system, two modes, two doors.

## 4. Launch gates

Per-rung gates live in the program doc §4. The ADR-level gate: after R8e,
`grep` finds ONE verb vocabulary, ONE event truth, ZERO `dpl-coach-*` crates,
and the §1 mock scene reproduced REAL on the Windows VM.

## 5. Consequences

Renames touch every xtask register (members, EGUI_FREE, CEILINGS, bin naming,
ORPHAN_ALLOWED pruning) and `architecture.md` §3's six coach bullets — each
in the SAME commit as its table, per the doc-and-audit law. The daemon's
`/health` self-description repoints here.

## 6. Supersessions

`docs/plans/coach-overlay-adr.md`, `docs/plans/coach-control-plane-adr.md`,
and `docs/plans/coach-operator-verb-catalog-adr.md` gain supersession headers
pointing at THIS ADR (the adr-016 renumber precedent: verbatim body kept,
header added — never deleted, never renumbered in place). Their ratified
boundaries (process exception, localhost binding, catalog-not-MCP-brain) are
carried forward unchanged above. ADR-012 remains a number gap by ruling.
