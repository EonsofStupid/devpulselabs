# P1 — the portable user-data root (KNOWN BROKEN #3, operator-directed)

Executed 2026-07-25 inside the W5 session (operator: "anything created
via the app should always be properly saved within the app … similar to
how the original was … truly portable, well organized").

## 1. Context

The app's root resolved from CWD only (`CONTENT_ROOT` env →
`./content`), so a double-clicked `target\release\devpulselabs.exe`
scattered a second content tree into `target\release\content` — tracked
as KNOWN BROKEN #3 ("Program P's data_root law not yet ported").

## 2. Current state (ground-truthed this session)

- `crates/engine-capture/src/lib.rs::content_root()` — env → `./content`.
- The ORIGINAL (`C:\localdev\devpulse labs\src-tauri\src\commands\
  content.rs`, read in full): turnkey resolution `--content-root` arg →
  exe-adjacent → bundled resources → `./content`, first existing,
  absolutized; `content_status` says where it looked; runtime writes in
  `data/` beside content, probe-writable, NO dotfolders/appdata; one-shot
  legacy migration.
- `target\release\content` existed with 9 scattered operator files
  (4 pix stills, vix videos + posters + meta, a rebuildable index db).

## 3. Target design (operator-RATIFIED via question)

ONE folder — today's `content/` tree stays the single user-data root
(inbox, elements, shows, artifacts.db together); the original's
content/data split is a possible later pass (needs an index rel-path
story). The RESOLUTION law is ported: ① `--content-root` CLI arg
(authoritative even if absent — drills mint fresh roots) ②
`CONTENT_ROOT` env (harness contract, kept) ③ ambient, first existing:
exe-adjacent `content/` → exe parents ≤4 up (finds the repo root from
`target/release/`) → cwd `./content` ④ nothing exists → exe-adjacent
(self-creates BESIDE THE EXE — portable, never cwd-scattered). Resolved
once (OnceLock), absolutized + normalized; `checked_roots()` is the
honesty surface, logged at boot.

## 4. Changes

1. `crates/engine-capture/src/lib.rs` — `content_root()` rewritten to
   the law above + `checked_roots()` + normalize/absolutize helpers +
   3 root_tests.
2. `crates/app/src/main.rs` — boot log: the resolved root (info), or a
   warn naming every checked location when it doesn't exist yet.
3. OPERATOR-APPROVED migration (one-shot, manual, this session): the 9
   scattered files MOVED into the real `content/inbox` (rename-uniquify,
   `-migrated-N` suffix on the 2 collisions), stale db dropped
   (rebuildable), `target\release\content` removed.

## 5. Boundaries

🚫 content/data split — NOT this pass (ratified single folder). 🚫 No
appdata/dotfolders (the original's portable law). `/content` untouched
except the operator-approved migration INTO it.

## 6–7. Verification + commit

`cargo green ✔` (root_tests 3/3 among them). REAL double-click
scenario: exe run with cwd `target\release` → boot log
`user-data root root=C:\localdev\devpulselabs\content`; no scattered
tree recreated; proof docs/proof/p1-doubleclick-gallery.png. Lands on
`feat/w5-guide-export` (PR #34) as its own commit.

## 8. Evidence

- Boot log (cwd=target/release): `user-data root
  root=C:\localdev\devpulselabs\content` · screenshot written ·
  `target/release/content NOT recreated`.
- Migration: 9 files moved (2 no-clobber renamed), tree removed.
- NOTE for the operator: the migrated captures appear in the Gallery
  after ONE `rescan` tap (the repo db had been quarantined by the known
  cross-process WAL issue — pre-existing, unchanged by this pass).
- Follow-up candidates: a `--rescan` headless seam; the content/data
  split pass; surfacing `checked_roots` in the status grid.
