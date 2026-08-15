# Program — Teacher-dev (Clyffy teaches from runtime events)

**Status:** DRAFT 2026-08-14 — first cut T0 executing in the same
session. Operator: `plan and implement do not rush we will keep working
this in passes`.
**Lane:** coach-seam · **Branch:** `feat/coach-teacher-dev-t0` (from
`origin/main`; does **not** ride the unmerged S3b Foundry branch).
**Related:** [`coach-seam-adr.md`](coach-seam-adr.md) ·
[`brainstorm-and-clyffy-drive-capture.md`](brainstorm-and-clyffy-drive-capture.md)
§3 · [`providers-presence-and-the-brain.md`](providers-presence-and-the-brain.md)
§7 · [`chat-assist-mode.md`](chat-assist-mode.md) ·
[`coach-lab-experiment-program.md`](coach-lab-experiment-program.md)
(`CX-teacher-agency-01`) ·
[`../research/coach/README.md`](../research/coach/README.md)

## 1. The real problem

Frontier AI cannot teach — or be watched teaching — from chat
paragraphs or screenshots. A change on the desktop is a **typed
runtime event**. Clyffy is the house face. Teacher is who may drive.
**Teacher-dev** is the working mode: obtain a goal, execute it on the
existing seam rails, and let the operator watch the SME↔AI pair on
that same event stream.

Three things already exist and must not be forked:

| Piece | What it is | What it is not |
|---|---|---|
| Clyffy | Pinned house session; backing provider visible | A second agent stack. Send-through-binding is still the next **chat** slice (M3b) — Teacher-dev must not pretend that is done. |
| `AgencyMode::Teacher` | Who may invoke a fabric address | A Clyffy chat mode. Today it is a Coach-hub toggle. `CX-teacher-agency-01` is still PLANNED. |
| `SeamEvent` / `SeamSpan` | Observation stream (geom/focus/text) and execute/refuse spans | A lesson lifecycle. There is no goal, no step, no disagreement, no `plan` / `invoke_agent` fact. |

The missing binding is the product: **Clyffy session mode × seam event
stream**. Not another fabric crate.

## 2. Design (chosen)

### 2.1 Approaches considered

1. **New `dpl-teacher-dev` crate now** — names the concern, but there
   is no engine yet. A crate with only types violates one-concern and
   invites a second vocabulary. **Rejected for T0–T3.**
2. **Stuff Teacher-dev into `ClyffySession` immediately** — the face
   is the right home for the *toggle*, but send is unfinished and the
   event vocabulary would be trapped in `app`. **Rejected as first
   cut.** Chat must consume the protocol, not own it.
3. **Protocol-first vocabulary + seam-domain ledger facts** (S1/S2
   pattern) — types live in `egui_pilot_ai_protocol::seam`; the fabric
   mirrors a serde-free `TeacherFact` (fabric has no protocol dep, same
   as `TransportRung` / `SeamAgency`); the ledger writes
   `teacher.*` rows on the existing `seam` domain. Clyffy and the hub
   grow later as *consumers*. **Chosen.**

### 2.2 Two planes, one stream

- **Planning plane (tokens):** Clyffy reads compact facts — pack,
  reliability fold, last N `teacher.*` + verb spans, the goal. Not
  trees, not pixels.
- **Execution plane (local, free):** cataloged `SeamVerb`s on fabric
  addresses. Agency stays `teacher`. The model names a verb + anchor;
  the runtime returns an `EffectContract`.

Industry alignment (2026, not cargo-cult): AG-UI typed event stream +
interrupts; OTel GenAI `plan` / `invoke_agent` / `execute_tool`;
watch → attend → unattend from the Clyffy drive capture. We imitate
the *vocabulary*, we do not take the deps.

### 2.3 Modes (do not collapse these)

| Name | Who leads | What happens |
|---|---|---|
| **Off** | — | Clyffy talks. No drive. |
| **Teacher** | AI | May execute on the same resolve/invoke/mount rails. Existing `AgencyMode::Teacher`. |
| **Teacher-dev** | AI, operator watches | Obtain a typed goal, execute steps, advance a watch surface on runtime events. Disagreement is a fact. |
| **Coach** | Operator | Mount + observe. Unchanged. |

`AgencyMode` stays `{Coach, Teacher}` — who may drive a verb.
`TeacherMode` is the Clyffy/session face: `{Off, Teacher, TeacherDev}`.
Teacher-dev **implies** agency Teacher. It is not a third agency and
not a second verb stack.

### 2.4 Runtime events (the fuel)

A **new** protocol enum `TeacherEvent` — do **not** extend protocol
`SeamEvent` (that is the foreign-app observation stream: moved,
focused, text_changed). Do **not** invent a new ledger domain (lab
law: no new free-form domains). Teacher lifecycle rides the **seam**
domain as `teacher.*` row types so `seam_reliability` can ignore them
and keep GROUP BY verb honest.

| Event | OTel-shaped operation | Meaning |
|---|---|---|
| `goal_obtained` | `plan` | The goal is a typed object, not a chat paragraph. |
| `step_started` / `step_cleared` | `execute_tool` | A cataloged verb bound to that goal. |
| `disagreement` | `invoke_agent` | SME path and AI path named different facts. |
| `interrupted` | `invoke_agent` | Operator kill / freeze / refuse. |

Existing `SeamSpan` execute/refuse rows stay the reliability surface.
Teacher events *point at* them (`goal_id`, `step`, `verb`); they do
not replace them.

### 2.5 Watch surface (later passes)

The HUD / overlay / inspect outlines already exist as pieces. T4
binds them: a step list advances when `step_cleared` (or a matching
observation `SeamEvent`) fires — not on a timer, not on a screenshot.
T0 does not draw.

## 3. Passes (one cut at a time)

| Pass | Cut | Proof |
|---|---|---|
| **T0** | Vocabulary + ledger. Protocol types, capability honesty, fabric `TeacherFact`, `teacher.*` Parquet rows, reliability fold ignores them. **No UI. No live execute. No protocol minor bump** (handshake is exact-match; types are additive). | Unit tests + one in-process publish→Parquet→fold. |
| **T1** | Clyffy `TeacherMode` as machine-legible session state (toggle + persist). Composer chip. No send, no execute. | Window/app tests; proof shot of the chip. |
| **T2** | Goal obtain. A `TeacherGoal` can be authored from chat (or a drill) and emits `goal_obtained`. Still no foreign execute. | Ledger row exists for a real Clyffy session id. |
| **T3** | Execute bound to a goal. Teacher invoke / daemon verb stamps `goal_id`+`step` and emits `step_*`. `CX-teacher-agency-01` lands here. | Calculator: one goal, one seed invoke, `step_cleared` + existing span. |
| **T4** | Watch surface. Overlay/HUD/step list advances on those events. | Screenshot of the pair (app + watch) at a real window size. |
| **T5** | Calculator live paired demo (SME click vs teacher path). | Disagreement fact when they diverge. |
| **T6** | Blender. Blocked on overlay R0/X11 presence arm + S4 pack. | Spike numbers stay the bar (named object < perception). |

S3b Foundry, S4 Blender pack, S5 Optimize cards, overlay R0 arm, and
Clyffy M3b send stay **their** lanes. Teacher-dev consumes them; it
does not steal them.

## 4. Laws this program will not break

- Same rails. No second verb enum, no second daemon, no screenshot hot
  path.
- Additive wire only (`#[serde(default)]`, unknown keys survive).
- Capability catalog stays honest: `seam.teacher-dev` is
  **Unsupported** on the pilot pipe until a later pass actually
  executes through it.
- Fabric does not gain an `egui-pilot-ai-protocol` edge (layer law).
  Types are mirrored, labels pinned by test — same as `TransportRung`.
- Clyffy send-through-binding is a chat slice. Teacher-dev T1 may
  show a mode; it may not fake a working composer.
- Community score / remote training stays opt-in (mock red-pen #8).
  T0–T5 are local-first ledger facts.

## 5. What we will measure

- Every Teacher-dev turn leaves `teacher.*` rows that a fold can
  count without polluting per-verb reliability.
- A goal has an id. A step names a cataloged verb. Disagreement names
  both sides.
- Token burn on the planning plane is bounded by compact facts (pack
  + last N events + goal), never a tree dump. Measured from T3.
- Watch latency: event → surface advance. Bar is the Blender spike
  class (tens to low hundreds of ms), not “after the next frame we
  felt like painting.”

## 6. Open questions (do not block T0)

1. T1 chip copy: "Teacher" vs "Teacher-dev" as one control with a
   second click, or two chips. Default proposal: one chip cycling
   Off → Teacher → Teacher-dev, kill switch always visible (seam ADR
   §6.3).
2. Does T2 wait for Clyffy M3b send, or does a drill/harness author
   the first goal? Default: drill first, chat authors when send lands.
3. Approval surface for T3 mutating verbs: native dialog (Clyffy drive
   capture, unforgeable) vs per-lesson consent already ruled in the
   seam ADR. Default: seam ADR §6.3 (per lesson + kill switch) until
   T4 watch makes a native interrupt worth it.

## 7. T0 is the only cut in this session

See [`coach-teacher-dev-t0-runtime-events.md`](coach-teacher-dev-t0-runtime-events.md).
Later passes each get their own 7-section plan and an explicit go.
