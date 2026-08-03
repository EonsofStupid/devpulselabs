# C2d — lightweight target inventory (Phase 0 + 1)

**Status:** PASS 2026-07-30 — operator approved plan; Phase 0+1 shipped  
**Spine:** [`coach-operator-experience.md`](coach-operator-experience.md) §3.4  
**Mock:** [`../ops/references/coach-operator-mock-c2c.html`](../ops/references/coach-operator-mock-c2c.html)  
**Prep journal:** [`../journal/2026-07-30-c2d-inventory-prep.md`](../journal/2026-07-30-c2d-inventory-prep.md)  
**Evidence journal:** [`../journal/2026-07-30-c2d-inventory.md`](../journal/2026-07-30-c2d-inventory.md)  
**Out of scope:** Phase 2 fabric mint (that is **C2e**)

## 1. Goal

Ship a **non-freezing** installed-app inventory that feeds the Operator
Coach picker: cold XDG catalog + shallow live a11y census (name/PID
only), with seed tips when join fails — so C2e can mint fabric for one
selected target without ever deep-walking at list time.

## 2. Current state

| Piece | Status |
|---|---|
| Product mock / home | LOOK-APPROVED (#167) — Coach hub |
| Foreign locate/paint | C2b proven on calculator |
| Inventory API | None |
| This box `.desktop` count | **152** unique (probed 2026-07-30) |
| Calculator desktop file | `org.gnome.Calculator.desktop` |

## 3. Design

### 3.1 Phases (hard split)

| Phase | When | Work | Forbidden |
|---|---|---|---|
| **0 Cold catalog** | Startup idle / refresh | Scan XDG `applications/*.desktop`; cache | AT-SPI, screenshots, UI-thread parse |
| **1 Live census** | After Phase 0 or Refresh | Desktop-root `GetChildren` only → name (+ PID if cheap) | Any deep tree / `GetChildren` on app internals |
| **2 Fabric mint** | **C2e only** | Budgeted walk of **one** selected app | Doing this inside C2d |

### 3.2 Types (sketch)

```
InventoryItem {
  id: DesktopId,           // e.g. org.gnome.Calculator.desktop
  display_name: String,    // Name= from .desktop
  exec_hint: String,       // Exec= first token / TryExec
  cold_ok: bool,           // Type=Application && !NoDisplay/Hidden
  live: Option<LiveHint>,  // filled by Phase 1 join
  complexity: Small|Medium|Deep|Unknown,  // heuristic for ETA copy
  seed_tip: Option<String>,
}

LiveHint { atspi_name: String, pid: Option<u32>, running: bool }
```

### 3.3 Phase 0 algorithm (Linux)

1. Collect dirs: `$XDG_DATA_HOME/applications` then
   `$XDG_DATA_DIRS` split (default `/usr/local/share:/usr/share`).  
2. Enumerate `*.desktop`; **first desktop-id wins** (Menu Spec).  
3. Parse `[Desktop Entry]`: keep `Type=Application` with `Name` and
   (`Exec` or `DBusActivatable=true`); drop `Hidden=true` /
   `NoDisplay=true`; apply `OnlyShowIn` / `NotShowIn` vs
   `$XDG_CURRENT_DESKTOP`.  
4. Cache to memory (+ optional disk under operator data later — not
   `/content`).  
5. Run off UI thread; expose `refresh_cold()` cancelable.

Sources: Freedesktop Desktop Entry Spec; Desktop Menu Spec dedupe;
ArchWiki desktop entries.

### 3.4 Phase 1 algorithm (Linux)

1. Open a11y bus via existing C2b connection pattern (`atspi` /
   `a11y_connection`) — **not** session-bus `org.a11y.atspi.Registry`
   guesswork (probed: ServiceUnknown on session bus).  
2. `GetChildren` on **desktop root only**.  
3. For each child: `name()` (+ pid if available without tree walk).  
4. Join to cold items: prefer `StartupWMClass` / Exec basename /
   case-insensitive Name; record unmatched live apps as
   `live-only` rows with seed tip “open once from menu to catalog.”  
5. **Hard refuse** any API that requests children of an application
   node in this crate/module for C2d.

Landmine encoded: LibreOffice-style huge `GetChildren` freezes apps
(fama11y-tree / LO 2024-06). C2d never deep-walks.

### 3.5 Complexity heuristic (for mock pills)

| Signal | Pill |
|---|---|
| Known seed (Calculator) | Ready · small |
| Running + small cold entry | Ready · small/medium |
| Keywords blender/cad/electron/chrome | Later · deep / tip |
| Not running | Ready (cold) — mint may launch |

Honest ETA copy comes from C2e; C2d only sets the pill.

### 3.6 Crate / placement (recommend)

New small library crate **`dpl-coach-inventory`** (or module under a
future `dpl-coach` hub crate) — **egui-free**, Linux deps `cfg`-gated,
Windows stub returns empty + tip. Never depend on `app`. Structure
audit: orphan-allowed until hub consumes it (same pattern as
coach-overlay) **or** wire as dep of a new hub in the same PR if hub
lands later.

C2d may ship **CLI proof** `coach-inventory-list` / unit tests first;
full Coach hub UI can wait for C2f — but list must be printable for
proof.

### 3.7 Windows (document only)

Phase 0: `FOLDERID_AppsFolder` / Start Menu `.lnk`.  
Phase 1: process list or UIA root apps **shallow**.  
No ship claim in C2d.

## 4. Changes (order after approve)

1. Crate + types + Phase 0 parser + unit fixtures (sample `.desktop`
   strings).  
2. Phase 1 shallow census behind `cfg(linux)`.  
3. Join + seed tips; Calculator appears Ready · small when running.  
4. Proof: `cargo test` + live list on this box (journal §8).  
5. Tip STATE / remaining-work; **no** deep mint.

## 5. Boundaries

- No Phase 2 / no screenshots / no overlay launch from inventory.  
- No UI-thread disk walk.  
- No `GetChildren` below desktop root.  
- Production overlay launch gates unchanged (ADR).

## 6. Verification

| Gate | Pass criteria |
|---|---|
| Unit | Parse fixture desktop files; Hidden/NoDisplay dropped; id dedupe |
| Adversary | API cannot request deep children (type-level or refuse) |
| Reality | Live list includes Calculator when running; finishes &lt; ~1s cold + &lt; ~2s live on this box; UI not required for CLI proof |
| Structure | `cargo xtask structure` green; Linux-only deps gated |

## 7. Commit plan

Branch `feat/c2d-coach-inventory` after operator `approve` on this plan.
One PR: crate + tests + live evidence journal tip.

## 8. Evidence — research + box probe

**Journal:** `docs/journal/2026-07-30-c2d-inventory-prep.md`

| Source | Takeaway |
|---|---|
| Desktop Entry Spec | Name/Exec/Type; visibility keys |
| Desktop Menu Spec | Path order; first id wins |
| LO / fama11y-tree 2024 | Deep `GetChildren` can freeze — budget + refuse |
| Ubuntu AT-SPI Accessible | GetChildren may error if prohibitive |
| This box | 152 `.desktop`; Calculator path known; C2b locate green |

Journal: `docs/journal/2026-07-30-c2d-inventory.md`.

| Check | Result |
|---|---|
| `cargo test -p dpl-coach-inventory --lib` | 7/7 |
| clippy `-D warnings` | clean |
| structure | `coach-inventory` orphan-allowed; hygienic |
| Live `coach-inventory-list` | Calculator **small · RUNNING** + seed tip; ~20 ms; 61 rows |

**Not done until approve:** ~~Rust implementation.~~ **Done.** Next = C2e.
