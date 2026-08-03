# Journal — 2026-07-31: Cursor a11y / fabric probe (test before UI)

## Verdict

**PROBED** — Cursor is live on this Linux seat and **Standard Import
succeeds** today. Not a dark-tree fail. UI select + lesson still missing
(C2f / CX-cursor). Calculator remains regression seed only.

## Operator direction

- Next conquest: **Cursor** (not Calculator).  
- Then **Notepad** (Windows Standard Import).  
- Build UI to select + import; if a11y fails → Advanced Import.  
- **Test before fully building everything** — this journal is that gate.

## Box probes (DISPLAY=:1, Cursor running)

| Probe | Result |
|---|---|
| Process | `/usr/share/cursor/cursor` (Electron) running |
| Inventory | `cursor.desktop` · **RUNNING** · atspi=`cursor` · complexity=medium |
| `coach-fabric-mint --name Cursor` | **READY · 157 nodes · ~385–520 ms** · Medium budget |
| Seed `7` | N/A (Calculator seed only) |

### Named fabric sample (unique named ≈ 101)

**Present (lesson candidates):**  
`File` · `Edit` · `View` · `Help` · `Search Ctrl+K` · `New File` ·
`Refresh Explorer` · `Hide Sidebar` · `Settings` (toggle) · `New Agent` ·
Agents/chrome buttons.

**Thin / missing in this mint:**  
Command Palette string · Terminal · Editor line content · Activity Bar
as such. Many **page tab** nodes from Agents/browser chrome (PR tabs) —
noise for a guided IDE lesson; filter later.

**Frame name observed:** `Cursor Agents` — mint may be rooted on the
Agents/web document surface as much as the classic workbench. Still
enough chrome for a first lesson seed (e.g. **Search Ctrl+K** or **File**).

## Import Process classification (this box, this session)

| Check | Result |
|---|---|
| Accessibility stream | **PASS** — app root + deep children |
| Window focus | Assumed OK (live mint) |
| Interactive density | **PASS for chrome**; editor/Monaco thin |
| Screenshot-as-eyes | Not used |

→ **Workflow = Standard Import** for Cursor *today*.  
Advanced Import stays the divert path when a future seat shows
chrome-only / forced-renderer-a11y-needed (Electron dark cases elsewhere).

## What we do **not** build yet

- Full Coach hub chrome  
- Advanced harvest crawler  
- Teacher mode  
- Filtering Agents tab noise (note for C2f plan)

## Persistent next (ordered)

1. **CX-cursor-01** (spike, test-first): CLI seed lesson target —
   resolve `Search Ctrl+K` or `File` via fabric + overlay locate/spotlight
   on Cursor (journal PASS/FAIL). No full hub.  
2. **C2f plan** against Import Process: Operator UI list includes Cursor →
   honesty prompt → checks → Standard Import → Mount seed.  
3. **Notepad** on Windows seat after Cursor path is green.  
4. Calculator stays in list as regression seed.

## Commands to re-run

```
DISPLAY=:1 target/release/coach-inventory-list | rg -i cursor
DISPLAY=:1 target/release/coach-fabric-mint --name Cursor
DISPLAY=:1 target/release/coach-fabric-mint --name Cursor --json
```

## Boundaries

- Probe-only; example binary not shipped.  
- No screenshot loop.  
- Overlay ADR unchanged.
