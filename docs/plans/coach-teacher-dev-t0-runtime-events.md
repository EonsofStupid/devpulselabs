# Pass — Teacher-dev T0: runtime-event vocabulary + ledger

**Status:** EXECUTING 2026-08-14 · first slice of
[`coach-teacher-dev.md`](coach-teacher-dev.md).
**Branch:** `feat/coach-teacher-dev-t0` from `origin/main` (S3b
Foundry is a parallel unmerged lane — do not pile onto it).

## 1. Context

S1 gave every verb a seam span. S2 put the verb catalog in the
protocol. Teacher-dev needs a **lesson lifecycle** on that same spine:
goal obtained, step started/cleared, disagreement, interrupted. Those
facts do not exist. Without them, Clyffy cannot later consume a
compact event stream, and any UI we painted would be theatre.

Handshake is an **exact** `PROTOCOL_VERSION` match
(`egui-pilot-ai-runtime` rejects `!= 2.7`). T0 therefore adds types
**additively under 2.7** — no minor bump. A bump is its own decision
when a later pass actually speaks these types on the pilot pipe.

## 2. Goal / non-goals

**Goal:** a process can publish a typed Teacher-dev fact; that fact
lands in the same Hive-partitioned `seam-*.parquet` layout as verb
spans; `seam_reliability` still answers "per verb" and **ignores**
`teacher.*` rows; the protocol can serde the same facts; the pilot
capability catalog names `seam.teacher-dev` as honest-Unsupported.

**Non-goals (later passes, by program §3):** Clyffy toggle (T1) ·
goal authoring from chat (T2) · live invoke bound to a goal (T3) ·
watch HUD/overlay (T4) · Calculator/Blender demos (T5/T6) · protocol
2.8 · `dpl-fabric` taking a protocol dep · new ledger domain ·
changing `AgencyMode` · S3b Foundry · Clyffy M3b send.

## 3. Ground truth (read 2026-08-14)

- `crates/egui-pilot-ai-protocol/src/seam.rs` — identity, verbs,
  `EffectContract`, observation `SeamEvent` (moved/focused/…). No
  teacher lifecycle. Tests pin additive serde on `EffectContract`.
- `crates/egui-pilot-ai-protocol/src/lib.rs` — `PROTOCOL_VERSION`
  `{2,7}`; capabilities `seam.verb` / `seam.events` /
  `seam.pack-attach` all Unsupported on the pilot pipe (S2 honesty).
- `crates/egui-pilot-ai-runtime/src/lib.rs:355` — handshake
  `!= PROTOCOL_VERSION` is a hard error. Do not bump.
- `crates/fabric/src/lib.rs` — serde-free; `SeamEvent::{Executed,
  Refused}`; `SeamAgency::{Coach, Teacher}`; **no** protocol
  dependency (`Cargo.toml` is tokio + hub + chat-core + intent).
- `crates/ledger/src/lib.rs` — `row_seam` projects Executed/Refused;
  `type` = verb wire name; `seam_reliability` GROUP BYs `type_name`
  for every `domain=="seam"` row. A teacher row with `type=press_key`
  would corrupt the fold. Teacher rows must use a `teacher.*` type
  and the fold must skip that prefix.
- Exhaustive `SeamEvent` matches today: `row_seam`, fabric tests,
  ledger tests. Daemon **constructs** Executed/Refused only — adding
  a variant does not break it.
- `ClyffySession` (`crates/app/src/chat.rs`) is id + backing +
  events. Untouched this pass.
- `AgencyMode` (`dpl-coach-runtime`) stays two arms.

## 4. Design

### 4.0 Disposition (nothing silent)

| Surface | Fate |
|---|---|
| Protocol observation `SeamEvent` | **Keep.** Not extended. Teacher lifecycle is a sibling enum. |
| Protocol `TeacherMode` / `GoalId` / `TeacherGoal` / `TeacherEvent` | **New** in `seam.rs`. |
| Capability `seam.teacher-dev` | **New**, Unsupported, evidence `coach-teacher-dev-t0`. |
| `PROTOCOL_VERSION` | **Unchanged** (2.7). |
| Fabric `SeamEvent` | **Add** `Teacher(TeacherFact)`. Mirror, no protocol dep. |
| Fabric `SeamAgency` | **Unchanged.** |
| Ledger `row_seam` | **Extend** match; `teacher.*` types. |
| `seam_reliability` | **Filter** `type_name` starting `teacher.`. |
| Daemon / hub / Clyffy / overlay | **Untouched.** |
| New crate | **None.** |

### 4.1 Protocol types (`egui_pilot_ai_protocol::seam`)

```text
TeacherMode     off | teacher | teacher-dev
GoalId          parse: non-empty [a-z0-9-], no leading/trailing '-'
TeacherGoal     id, text, optional AppId, open extra map
TeacherEvent    tagged kind:
                  goal_obtained { goal, mode }
                  step_started  { goal_id, step, verb }
                  step_cleared  { goal_id, step, effect_observed, duration_ms }
                  disagreement  { goal_id, expected, observed }
                  interrupted   { goal_id, reason }
```

`TeacherEvent::operation()` returns the OTel-shaped name (`plan` /
`execute_tool` / `invoke_agent`) for attrs and docs — not a new
span schema. Unknown keys on structs default; tagged enum is closed
(new kinds are a later additive pass with their own compat test).

`GoalId` serde goes **through parse**, same law as `AppId`.

### 4.2 Fabric mirror

```text
TeacherMode     Off | Teacher | TeacherDev   + as_str()  (off/teacher/teacher-dev)
TeacherFact     same five arms, String fields (fabric is serde-free)
SeamEvent       Executed | Refused | Teacher(TeacherFact)
```

A protocol↔fabric label test lives in the **protocol** crate (it
already depends on nothing from fabric). The fabric crate documents
the labels next to `SeamAgency`. Daemon's existing
`protocol_and_fabric_rung_labels` pattern is **not** copied here —
T0 does not touch the daemon. A unit test on each side pins the
slug strings to the same literals.

### 4.3 Ledger

`row_seam` for `Teacher(fact)`:

- `type` = `teacher.<kind>` (`teacher.goal_obtained`, …)
- body `{ v: 1, operation, …fields, "teacher": true }`

`seam_reliability` skips `type_name` starting with `teacher.`.
Existing press_key fixture stays green.

### 4.4 Capability

`seam.teacher-dev` — Unsupported. Detail: types exist; the pilot
pipe does not obtain goals or execute foreign verbs. Evidence:
`coach-teacher-dev-t0`. Testing-vs-coaching separation stays visible.

## 5. Atomic tasks

1. Author this plan + the program doc.
2. Protocol types + tests (parse, serde, additive extra, operation
   names, capability present and Unsupported).
3. Fabric `TeacherFact` + `SeamEvent::Teacher` + subscribe test.
4. Ledger `row_seam` + reliability skip + Parquet proof test.
5. Amend seam ADR §8 (T-slices pointer), architecture.md (one
   sentence on the new fabric arm + capability), research census
   row for T0.
6. `cargo test` on the three crates. Full `cargo green` before PR.

## 6. Risks

| Risk | Mitigation |
|---|---|
| `teacher.press_key` colliding with verb `press_key` | Prefix is `teacher.` + kind, never a verb name. |
| Reliability fold counts teacher rows as verbs | Explicit prefix skip + test that a mixed batch still folds only `press_key`. |
| Handshake breakage from a version bump | No bump. |
| Fabric/protocol label drift | Identical slug literals + tests on both sides. |
| Scope creep into Clyffy/hub | Out list is law; no those files in the diff. |
| Exhaustive match miss | Compiler will name every `SeamEvent` match. |

Rollback: revert the branch. No on-disk pack format change. Old
Parquet readers that ignore unknown `type` values keep working;
new `teacher.*` types are additive rows.

## 7. Verification (clyffy-verify target)

1. **Correctness** — protocol + fabric + ledger unit tests green;
   `seam_reliability` fixture unchanged; new mixed-batch test.
2. **Reality** — in-process: publish five `TeacherFact`s → Parquet
   → read back `teacher.*` types and `operation` names. No live
   app required (T0 has no executor).
3. **Adversary** — invalid `GoalId` refused on parse and on serde;
   capability is Unsupported (a client requiring it must fail
   handshake the same way `seam.verb` already does); reliability
   fold cannot be poisoned by a `teacher.goal_obtained` row.

## 8. Evidence

**2026-08-14 — T0 types + ledger proof green** (worktree
`../devpulselabs-t0`, branch `feat/coach-teacher-dev-t0` from
`origin/main` @ `ea57cdc`). Full `cargo green` is the PR gate, not
this cut's close — do not rush.

- `cargo test -p egui-pilot-ai-protocol -p dpl-fabric -p dpl-ledger`
  + `cargo clippy -p … --all-targets -- -D warnings`: green.
  Protocol: 34 lib + 3 v1_compat + 2 v2_golden. Ledger: 9 lib
  (new `teacher_facts_land_as_teacher_types_and_do_not_poison_reliability`).
- Reality: in-process publish of `press_key` span + three
  `TeacherFact`s → Parquet. Rows: `teacher.goal_obtained`
  (`operation=plan`, `mode=teacher-dev`), `teacher.step_cleared`
  (`execute_tool`), `teacher.disagreement` (expected Cube / observed
  Camera). `seam_reliability` on the mixed batch returns **only**
  `press_key` (1/1 effect_ok).
- Adversary: `GoalId` parse+serde refuse empty / leading `-` /
  uppercase / spaces; `seam.teacher-dev` is cataloged
  `Unsupported` with evidence `coach-teacher-dev-t0`; handshake
  still exact-match 2.7 (no bump).
- Files: `crates/egui-pilot-ai-protocol/src/{lib,seam}.rs`,
  `crates/fabric/src/lib.rs`, `crates/ledger/src/lib.rs`,
  `docs/plans/coach-teacher-dev.md`,
  `docs/plans/coach-teacher-dev-t0-runtime-events.md`,
  `docs/plans/coach-seam-adr.md` §8 pointer,
  `docs/design/architecture.md`,
  `docs/research/coach/README.md`.
  **Not in the diff:** Clyffy, hub, daemon, overlay, S3b.
