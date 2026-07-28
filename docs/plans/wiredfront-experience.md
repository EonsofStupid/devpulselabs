# WiredFRONT experience contract — the frictionless editor

Status: AWAITING OPERATOR RATIFICATION. This document is the UX GATE for
every R1/R2 editor pass: a pass that does not move a row of §5's friction
audit is not an editor pass and must not be scheduled as one. It exists
because the operator's verdict (2026-07-24) was correct: the R1 passes
drifted into plumbing (comp document, axum seam, HTTP endpoints) while the
EXPERIENCE — the reason the product exists — stayed a text-chip list, and
R1u mounted Workbench's ingredient wall into WiredFRONT (the two-libraries
crossing, since corrected on paper, un-crossed in the R1x working tree).

## 1. The vision, in the operator's words (never re-derive)

- "After Effects but with our chat route aware." (2026-07-23)
- "Stitch series of 8–15 second AI-generated clips, then properly
  post-process, enhance, apply effects." (2026-07-23)
- "Gigs and gigs" of custom After Effects elements "that AI can leverage
  for animations and access for editing." (2026-07-23)
- "This is supposed to be a personal tool… all about making my personal
  life easier for creating content. WiredFront is my after effects editor.
  It needs separate windows to appear on click i.e my browsing local video
  and image folders or google drive even. It also needs to import my after
  effects." (2026-07-24)
- "THE UI IS EVERYTHING TO THIS EXPERIENCE… GET THE UI ACTUALLY BUILT SO I
  CAN DO THIS FRICTIONLESS." (2026-07-24)

The operator has NEVER pivoted. Every restatement is the same product.
Personal tool: licensing is a non-constraint; the operator's own media and
subscriptions are first-class inputs.

## 2. The bar — what "frictionless" means, measurably

1. **Zed standard**: separate REAL windows appear instantly on click,
   native decorations, resizable, taskbar-visible, no jank. An embedded
   fallback is an honest failure label, never a silent downgrade.
2. **Nothing is more than two clicks from the timeline.** Any clip on this
   machine reaches the timeline in ≤2 interactions once its window is open
   (click file → it's on the timeline), or ONE gesture from Explorer
   (drag-drop).
3. **The editor never opens empty-handed.** Reopening the editor restores
   the comp you were editing, playhead included.
4. **Everything visual is a thumbnail.** No filename-only lists for media,
   anywhere in WiredFRONT.
5. **Every control tooltips in plain words** (the tooltip law), every hit
   target is the whole card/chip (the touch law).
6. **Mistakes are free**: undo/redo across every timeline edit and every
   agent-applied edit; sources are never mutated; imports COPY.
7. **The AI is in the room, not in an API.** Chat sits in its peer window
   beside the editor; its comp edits are visible proposals that apply to
   the open timeline in front of you, doctor-gated.

## 3. The canonical session — "sit down, make a video"

Every step names its surface. This is the walkthrough every pass must
shorten, never lengthen.

1. Double-click `devpulselabs.exe` → sidebar → **wiredFRONT › Editor**.
   The comp you left last night loads; the playhead is where you left it.
2. The **media bin** (left) shows your captures/renders as thumbnail
   cards. You click **Browse…** — a REAL separate window opens on your
   `Videos` folder: thumbnail grid, folder cards, root chips
   (Videos · Pictures · Downloads · Desktop · drives). You open a second
   Browse window onto the folder of last night's AI clips. Both stay open.
3. You click four clips across the two windows — each lands appended on
   the timeline, proportional bar, poster thumbnail in the bin. You also
   drag one mp4 straight from Windows Explorer onto the editor — an
   overlay says "Drop to import"; it lands in your inbox and on the
   timeline.
4. You scrub the ruler, drag bar edges to trim heads/tails, drag bodies to
   reorder, Space to play the sequence. The preview follows the playhead
   across clip boundaries.
5. From the **elements shelf** (your ingested AE gigs, alpha previews) you
   click a lower-third onto the OVERLAY lane above clip 2.
6. In the **inspector**, you drop an `xfade` transition between clips 1–2
   and a `curves` bump on clip 3 — each effect a named chip from the
   registry, tooltipped.
7. You summon **Chat** (status bar) — it opens beside the editor. "Tighten
   clip 2 to the beat drop and add the glitch element over the cut." The
   proposal card shows the doctor-validated diff; you tap **Apply**; the
   timeline updates. Ctrl+Z would take it back.
8. **Render** on the transport bar → ffmpeg stitches with transitions →
   the render lands in **wiredFRONT › Library** (the MEDIA shelf: clips,
   images, elements, renders — never ColdLight ingredients), ready to
   become a slide in Workbench.

## 4. Surface & window inventory (the UX contracts)

- **Editor · Edit tab** — tools rail left · preview center · inspector
  right · ruler timeline over full-width transport bar. Contract: ruler
  marks + labels, bars proportional to duration, voltage playhead,
  edge-trim (8px zones, 0.25s min span), body-move (0.1s snap),
  click-to-seek, zoom 20–240 px/s. (R1x working tree — land it.)
- **Media bin** — thumbnail cards (poster/image + name + duration badge),
  click = append to timeline; header gains **Browse…**. Videos AND
  images feed the timeline (W7: stills held 5s, trimmable).
- **Browse window** (NEW, the operator's named requirement) — a native
  peer window per open folder. Top bar: Up · root chips · current path.
  Grid: folder cards descend; image cards show the image; video cards are
  honest filmstrip cards (name + ext + size) — the poster is made at
  import, never by bulk-probing folders. Click a media card = import
  (copy to inbox, write-through index) + append (video) / library (image).
  Multiple windows, independently navigated; close is instant; embedded
  fallback = honest failure (chat's law).
- **Explorer drag-drop** — dropping media files anywhere on the editor
  imports them; hover shows "Drop to import"; the status line names what
  landed.
- **Elements shelf** (R2a) — the ingested gigs as alpha-preview cards,
  filterable by pack/tag, click = overlay layer at the playhead.
- **Inspector** — selection facts now; effects/transition chips (R2b);
  keyframe/motion grammar later.
- **Transport bar** — New · Save · Render · play/pause · mm:ss/mm:ss ·
  comp facts. Keyboard: Space play/pause, Delete remove-selected,
  arrows nudge (R1z).
- **Chat beside the editor** (R2c) — the existing status-bar peer window
  gains comp tools over the R1v/R1w seam: proposal → doctor verdict →
  Apply/Discard; every apply is one undo step.
- **wiredFRONT › Library** — RAW MEDIA ONLY: elements census + rendered
  comps + imports. The ColdLight wall lives in Workbench (two-libraries
  law; R1x commit 2 un-crosses it).
- **Google Drive window** (R2d) — a Browse window whose root is Drive:
  same grid, same click-to-import (download → inbox → index). Needs an
  HTTP client dep + OAuth (keyring per ADR-008) — ask-first, own pass.

## 5. Friction audit — today vs the bar (the gate table)

| # | Walkthrough step | Today (re-ground-truthed 2026-07-27, deep-plan sweep) | Killed by |
|---|------------------|-----------------------------------|-----------|
| 1 | Editor restores your comp | ✅ R1z: reopen-last via `composer-state.json` + debounced autosave + Drop flush | DONE (R1z) |
| 2 | Media bin thumbnails | ✅ W1 (`067646d`): the editor rebuilt to the approved AE mock — thumbnail bin included | DONE (W1) |
| 3 | Browse windows onto folders | ✅ W2 (`60db7be`): native Browse peer windows | DONE (W2) |
| 4 | Drag-drop from Explorer | ✅ W2 (`60db7be`): drop-to-import through the one seam | DONE (W2) |
| 5 | Real timeline (ruler/bars/trim/scrub/sequence preview) | ✅ W1 (`067646d`) | DONE (W1) |
| 6 | AE elements usable | PARTIAL: pack shelf + ingest landed (W3/W4); `Source::Element` still renders NOTHING in preview or stitch — the dead render leg | T1a (workflows program) |
| 7 | Transitions/effects | None; render is bare concat — the deepest gap | T1c (= ratified R1d) |
| 8 | Undo/redo | ✅ R1z: snapshot undo/redo (cap 100, one step per drag) + Ctrl+Z/Ctrl+Shift+Z/Ctrl+Y + Space/Delete/arrows | DONE (R1z) |
| 9 | Chat edits the open comp | PARTIAL: seam drives comps end-to-end (R1v/w) but no editor-side proposal UI | T4d (= ratified R1f) |
| 10 | Renders/library correct | ✅ W1.2 (`9a95fc7`): the two-libraries un-crossing | DONE (W1.2) |
| 11 | Google Drive | `ureq` landed (#98, ADR-011) — the HTTP-client blocker is gone; needs OAuth + the Drive Browse window | T1h (R2d) |
| 12 | Background removal | Not started (ort mini-ADR owed) | R2e (= ratified R1e) |
| 13 | Double-click Library → editor | ✅ W6: the Library is the real thumbnailed shelf; double-click sends video → timeline / image → FOOTAGE (honest) and navigates to the Editor | DONE (W6) |
| 14 | ffmpeg found without PATH | ✅ W6: managed boot resolution (bin/ beside exe → PATH → winget links) injected into all three engine seams | DONE (W6) |
| 15 | Images are timeline layers | ✅ W7: stills held 5s (trimmable), preview shows the frame, render loops it — every add path (FOOTAGE, Library, drop, Browse) appends | DONE (W7) |
| 16 | No console windows over the app | ✅ W8: the quiet-spawn law (dpl-proc) at every GUI-reachable subprocess + a cargo-green audit that fails unflagged spawns | DONE (W8) |
| 17 | Browse windows open instantly | ✅ W8: the proactive-buffer law — warm cache seeded at boot, off-thread scans, roots probed once off the paint thread | DONE (W8) |
| 18 | Signals live, views never stale | ✅ W8: the signal doctrine — capture + jobs domains, stamp-at-publish, emit-with-mutation everywhere, views invalidate on signal, activity feed + SSE envelope | DONE (W8) |

## 6. The ladder — passes named by the friction they kill

1. **LAND R1x** — ✅ RETIRED: shipped as W1 (`067646d`) + W1.2
   (`9a95fc7`). Rows 2, 5, 10 closed.
2. **R1y — get media in from anywhere** — ✅ RETIRED: shipped as W2
   (`60db7be`). Rows 3, 4 closed.
3. **R1z — never lose work**: comp autosave + reopen-last + undo/redo +
   the keyboard map. Rows 1, 8. ✅ LANDED
   (`docs/plans/r1z-editor-safety.md`).
4. **R2a — my AE elements live here** (ratified R1c): ingest the gigs into
   `content/elements/<pack>/`, index, elements shelf + overlay lane. Row 6.
5. **R2b — cuts feel finished** (R1d slice): transition/effect registry v1
   (xfade · fades · eq · sharpen) + inspector chips + render honors it. Row 7.
6. **R2c — the chat edits with me** (ratified R1f): comp tools in the chat
   peer window over the existing seam; proposal/apply/undo. Row 9.
7. **R2d — Drive without leaving**: Drive-rooted Browse window. Row 11.
   GATED on the operator approving an HTTP client dependency.
8. **R2e — matting** (ratified R1e): after the ort mini-ADR. Row 12.

"Import my After Effects" has three honest tiers — the operator picks the
target: (a) rendered elements (alpha MOV/WebM/PNG sequences) = R2a;
(b) `.aep` PROJECT parsing into comp.json layers/keyframes = its own
spike after R2a (RIFX binary; partial recovery is the honest ceiling);
(c) Lottie/bodymovin JSON = cleanly parseable, rider on R2a.

## 7. Laws binding every pass here

Two-libraries law (raw media ↔ ColdLight, never crossed) · touch law ·
tooltip law · ColdLight gate (`coldlight-system` skill before any UI
pass) · sources never mutated (imports COPY into the inbox) · `/content`
never committed/authored/deleted · ffmpeg subprocess only (ADR-009) · no
`#[allow]` · proof shots at REAL window sizes, sequential exe runs ·
peer windows fail honestly (never silently embed).

## 8. Ratification asks (operator)

1. Ratify this contract + the §6 ladder order.
2. R1y plan approval (`r1y-media-windows.md`). — retired with rung 2 (shipped as W2).
3. "Import my After Effects": is tier (b) `.aep` project parsing required,
   or do rendered elements (a) + Lottie (c) cover the real workflow?
   (Partial answer landed: Lottie (c) shipped natively as L2, #99.)
4. ~~R2d dependency: approve adding an HTTP client crate when it
   schedules.~~ RETIRED 2026-07-27: `ureq` landed under ADR-011 (#98);
   the remaining R2d ask is OAuth custody, not the client.
