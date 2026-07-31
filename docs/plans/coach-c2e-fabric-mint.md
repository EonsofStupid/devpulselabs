# C2e — fabric mint for selected Calculator (Phase 2)

**Status:** PASS 2026-07-31 — operator `go`; crate + live Calculator mint  
**Evidence journal:** [`../journal/2026-07-31-c2e-fabric-mint.md`](../journal/2026-07-31-c2e-fabric-mint.md)  

**Spine:** [`coach-operator-experience.md`](coach-operator-experience.md) §3.4 / §3.6  
**Prior:** [`coach-c2d-lightweight-inventory.md`](coach-c2d-lightweight-inventory.md) PASS (#172)  
**Mock:** [`../ops/references/coach-operator-mock-c2c.html`](../ops/references/coach-operator-mock-c2c.html)  
**Out of scope:** Coach hub UI chrome, overlay paint wiring (**C2f**), Pass-2 stdio

## 1. Goal

Mint a **budgeted Tier A semantic fabric** for **one** selected inventory
target (seed: Calculator), with cancelable progress + honest ETA copy and
typed refuse tips — so the Operator Coach “Build signal map” step is real
without freezing apps or using screenshots.

## 2. Current state

| Piece | Status |
|---|---|
| Inventory Phase 0+1 | PASS — `dpl-coach-inventory` + CLI (#172) |
| Calculator seed | small · RUNNING + tip; desktop-root join works |
| Overlay locate/paint | PASS (C2b) — single-control search, not full fabric |
| Phase 2 mint API | **None** |
| Progress / refuse UX | Mock only (C2c LOOK-APPROVED) |
| Hub select → paint | **C2f** |

## 3. Design

### 3.1 Hard split from inventory

`dpl-coach-inventory` **stays** desktop-root-only. C2e does **not** add
deep `GetChildren` there (adversary gate from C2d).

New crate **`dpl-coach-fabric`** (egui-free, Linux `cfg`-gated, orphan-
allowed until hub consumes it):

- Input: selected `DesktopId` / live AT-SPI name (+ optional complexity).
- Output: `MintOutcome` + progress stream.
- May **depend on** `dpl-coach-inventory` for join keys only — never the reverse.

### 3.2 Types (sketch)

```
FabricNode {
  path: String,           // stable-ish path / index chain
  role: String,           // AT-SPI role display
  name: String,
  actions: Vec<String>,   // cheap Action iface names if present
  // extents optional — fetch for seed lesson node only if needed
}

Fabric {
  target_id: DesktopId,
  atspi_name: String,
  nodes: Vec<FabricNode>,
  minted_ms: u64,
}

MintProgress {
  visited: u32,
  budget: u32,
  depth: u32,
  phase: Locating | Walking | Seeding | Done,
  eta_hint: String,       // layman copy from complexity + visited
}

Refused { reason: String, tip: String }

MintOutcome = Ready(Fabric) | Refused | Cancelled
```

Progress is a typed channel / callback (`MintProgress`), not screenshots.
Full in-app `FabricEvent` touchpoint wiring stays C1.5a / **C2f** — C2e
proves mint progress + seed node presence (e.g. button `7`).

### 3.3 Mint algorithm (Linux)

1. **Locate app root** — reuse C2b/`atspi` a11y bus pattern (same as
   inventory Phase 1 + overlay `find_app_root`). Refuse if not running
   with tip: “Open Calculator, then Build again.”
2. **Budgeted BFS** from app root (or first Frame/Window child):
   - Prefer `ChildCount` + indexed child fetch when available.
   - Hard caps (defaults; Small complexity):
     | Cap | Default (Small) | Medium | Deep |
     |---|---|---|---|
     | `max_nodes` | 400 | 1500 | 4000 |
     | `max_depth` | 12 | 16 | 20 |
     | `max_ms` | 2_000 | 8_000 | 20_000 |
     | `max_children_at_node` | 256 | 256 | 256 |
   - Node with `ChildCount` / children len > `max_children_at_node` →
     **skip that node** (or refuse whole mint if root itself oversized)
     with tip citing freeze landmine — never dump LibreOffice-scale lists.
3. **Progress** every N nodes (e.g. 25) + phase changes; cancel flag checked
   each batch.
4. **Seed check (Calculator):** after walk, require a node whose name is
   `7` (or role+name match). Missing → `Refused` tip: “Open the basic
   keypad view, then retry.” Ready fabric may mark seed path for C2f.
5. **No screenshots. No overlay launch. No XTEST.**

### 3.4 Complexity → copy

Reuse `Complexity` from inventory for ETA strings only (mock vocabulary):

| Pill | Confirm copy (example) |
|---|---|
| small | “About a second — Calculator is a small signal map.” |
| medium | “A few seconds — walking menus.” |
| deep | “May take a while — or refuse if the tree is too large.” |
| unknown | “We’ll try with a tight budget and tip if we stop.” |

### 3.5 CLI proof

`coach-fabric-mint --name Calculator` (or `--id org.gnome.Calculator.desktop`)

- Default: human progress lines + Ready/Refused summary.
- `--json`: final outcome (+ optional progress events).
- `--budget-nodes N` for adversary drills.

### 3.6 Windows

Document-only: UIA tree walk with the same budget shape. No ship claim.

## 4. Changes (order after approve)

1. Crate `dpl-coach-fabric` + types + budget defaults + unit tests
   (fake tree / refuse oversized node / cancel).
2. Linux mint walker (a11y bus) + progress + Calculator seed check.
3. CLI `coach-fabric-mint`; structure orphan-allow + architecture tip.
4. Live proof on this box; evidence journal; tip STATE / remaining-work /
   spine NEXT → C2f.
5. **No** Coach hub UI; **no** overlay paint call (C2f).

## 5. Boundaries

- Inventory crate remains deep-walk free (C2d adversary holds).
- No Tier C / screenshots / continuous capture.
- No production `app` dep; no overlay auto-launch (ADR gates).
- No Pass-2 stdio; no DuckDB; no hub chrome.
- Copy never move; do not delete C2b/C2d evidence.

## 6. Verification

| Gate | Pass criteria |
|---|---|
| Unit | Budget refuse; cancel; seed-name match on fixture graph |
| Adversary | Node with >`max_children_at_node` never fully dumped; inventory API still cannot deep-walk |
| Reality | Live mint Calculator → `Ready` with node `7`; finishes under Small `max_ms`; progress printed; journal |
| Structure | `cargo xtask structure` green; Linux deps gated |

**clyffy-verify modes:** correctness (unit) · reality (live Calculator mint) ·
adversary (oversized-child refuse + inventory invariant).

## 7. Commit plan

Branch `feat/c2e-coach-fabric` after operator `approve` on this plan.
One PR: crate + CLI + tests + live evidence journal.

## 8. Evidence — research + prior probes

| Source | Takeaway |
|---|---|
| C2d prep journal | Deep naive `GetChildren` freezes (LO / fama11y-tree); use ChildCount + budget + refuse |
| C2d live | Calculator join ~20 ms; desktop-root only is safe |
| C2b overlay | `find_app_root` / locate `7` already prove Tier A seed exists |
| Spine §3.4 | Phase 2 = one target, cancelable, progress, refuse dark/oversize |
| Mock C2c | Progress + Cancel + refuse tip copy already LOOK-APPROVED |

| Check | Result |
|---|---|
| `cargo test -p dpl-coach-fabric --lib` | 8/8 |
| clippy `-D warnings` | clean |
| structure | `coach-fabric` orphan-allowed |
| Live `coach-fabric-mint --name Calculator` | Ready · 82 nodes · ~255 ms · seed `7` |

**Done.** Next = **C2f**.
