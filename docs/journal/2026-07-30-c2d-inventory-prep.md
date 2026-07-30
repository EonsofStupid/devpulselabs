# Journal — 2026-07-30: C2d prep — lightweight inventory research

## Intent

Operator: journal first; web-research and notes prepared for true success
before C2d code. No implementation this pass.

## Product frame (already locked)

- Spine LOOK-APPROVED + mock look-approved (#167).
- NEXT = C2d Phase 0+1 only (cold catalog + shallow live census).
- Phase 2 deep mint = **C2e** (explicitly out of C2d).
- Dual surface: Operator list vs Advanced probes.

## Box ground truth (this seat, 2026-07-30)

| Probe | Result |
|---|---|
| XDG `.desktop` unique ids | **152** across `$XDG_DATA_HOME` + `$XDG_DATA_DIRS` |
| Calculator entry | `/usr/share/applications/org.gnome.Calculator.desktop` |
| Overlay locate still green | `dpl-coach-overlay locate --name 7` → WINDOW hotspot composed |
| Naive `gdbus` to `org.a11y.atspi.Registry` | ServiceUnknown on session bus — a11y is on the **a11y bus**; Rust `atspi` path already correct |

Lesson: Phase 0 is cheap disk (~150 files). Phase 1 must use the same
a11y connection we already trust in C2b — not a wrong-bus gdbus hack.

## Web / primary-source research (notes)

### Phase 0 — cold catalog (Freedesktop)

- **Spec:** [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest/)
  — `Type=Application`, required `Name`, `Exec` (or DBusActivatable),
  honor `NoDisplay` / `Hidden` / `OnlyShowIn` / `NotShowIn`.
- **Locations:** `$XDG_DATA_HOME/applications` then each
  `$XDG_DATA_DIRS/applications`; earlier desktop-id wins (Menu Spec
  dedupe).
- **Cache hint:** `mimeinfo.cache` exists for MIME→app maps; for Coach
  we need **apps**, not MIME — scanning `.desktop` once and caching is
  the right shape (ArchWiki + Menu Spec).
- **Anti-freeze:** parse on background / idle; never on UI frame path;
  refresh via mtime or inotify on applications dirs.

### Phase 1 — live census (AT-SPI / UIA)

- Desktop root `GetChildren` returns **application** children (name +
  path) — that is the shallow census. Matches our `find_app_root` walk.
- **Landmine (must encode in plan):** calling `GetChildren` on huge
  nodes freezes the target app. LibreOffice Calc exposed **2^31**
  children; D-Bus max message 128 MiB; fama11y-tree / LO a11y threads
  2024-06 document the freeze
  ([fdo list](https://lists.freedesktop.org/archives/libreoffice/2024-June/092041.html);
  Ubuntu AT-SPI docs: implementors may error on prohibitive child lists).
- **Rule for us:** Phase 1 = **desktop-root children only** (name/PID/
  role). Never deep `GetChildren` in C2d. C2e mint must use
  `ChildCount` + indexed fetch + **hard budget** + refuse oversized
  nodes — never naive full dumps.
- Industry preference: event-driven incremental cache
  (`object:children-changed`) for deep trees (waydriver #11) — belongs
  to C2e+, not C2d.

### Windows (parity note, not C2d ship claim)

- Prefer `FOLDERID_AppsFolder` / `shell:AppsFolder` enum (IShellItem),
  or Start Menu `.lnk` trees — **not** UIA deep walk for inventory.
- UIA deep walk = Phase 2 mint later. Document only; Linux-first.

### Screenshot tax

- Confirmed again: inventory and mint must stay Tier A. Screenshots are
  Tier C refuse-path only (spine law).

## Risks to design against

1. UI thread blocking on 152+ desktop parses → **async + cache**.  
2. Matching `.desktop` Name to AT-SPI app name (Calculator vs
   `gnome-calculator`) → explicit **join key** (Exec basename / StartupWMClass
   / desktop-id) with seed tips when unmatched.  
3. Deep walk creep into C2d → **hard gate**: C2d API returns
   `InventoryItem { cold, live_hint }` with **zero** tree nodes.  
4. Wrong D-Bus bus → reuse C2b `a11y_connection()`.  
5. Electron dark trees → Phase 1 still lists the app; mint (C2e) refuses
   with tip (`--force-renderer-accessibility`).

## Plan artifact

Full pass plan: `docs/plans/coach-c2d-lightweight-inventory.md`
(7 sections + §8 research evidence). **No code until operator approves
that plan.**

## Confidence

**High** on Phase 0 XDG approach and Phase 1 desktop-root-only census.  
**Med** on desktop↔AT-SPI join heuristics (needs one seed matrix on box).  
**Open:** Gio `DesktopAppInfo` vs hand parse; inotify vs poll mtime.

## Next

1. Operator reads / amends C2d plan.  
2. On `approve` → implement Linux Phase 0+1 behind a small crate or
   coach module; unit + live list proof; no hub chrome yet beyond what
   plan allows.
