# Census raw — WIREDFRONT / CAPTURE SUITE (old repo) — 82 items

## Hub/shell/routing
1. wiredFRONT hub tab nav (Overview·PixVix·ViziWiz·Gallery·Sessions), authoring-only — solid
2. Overview dashboard: 4 launcher cards + inbox review; sweeps extension Downloads on load — solid
3. PixVixHome header: monitor/window-count readout + Open palette — solid
4. Footer Snip launcher opens palette window (NOT OS tray) — solid
5. Recents via shared ContentLibrary — solid

## Palette
6. Slim always-on-top toolbar, 4 tabs, per-tab tone, hide-warm/close-destroy — solid
7. open/hide/toggle commands (lazy create, warm reuse) — solid
8. GLOBAL hotkey Ctrl+Shift+S summon/dismiss — solid
9. "Files to" chip + meta popover; blank name auto-names (never blocks) — solid
10. Per-tab first-run help popover (seen-once, ? reopen), persisted — solid
11. Settings: auto-copy-clipboard toggle, persisted video defaults — solid
12. Honest capture-blocker banner via runtime status — partial

## Img stills
13. Modes Region/Window/Full/Freeform → inbox PNG — solid
14. Full-monitor still — solid
15. Window still (occlusion-independent surface capture) — solid
16. Region still, DPI-proof fractions — solid
17. Freeform polygon still, alpha-0 outside, bbox crop — solid
18. Capture delay 0/3/5/10s countdown — solid
19. Re-snip (replay last SnipDesc) — solid
20. Auto-copy to clipboard (best-effort) — solid
21. Window picker list (refreshable) — solid
22. capture_vix N-second clip (legacy) — partial

## Region overlay + loupe
23. Transparent decoration-less always-on-top overlay — solid
24. Screen FREEZE (frozen PNG to overlay) WYSIWYG — solid
25. 6× magnifier loupe following cursor — solid
26. Draw-then-confirm: drag, ARROW-KEY NUDGE (Shift×10), Enter/click-inside — solid
27. Freeform tracing: points, close-radius snap, dbl-click/Enter close — solid
28. Cancel paths Esc/right-click → cancel_snip, restores palette — solid
29. Headless hooks __lastRegion/__lastFreeform — solid

## Video
30. Start/Stop window-or-full → inbox clip, single-recording guard — solid
31. Engine: xcap + constant-fps + ffmpeg x264 ultrafast (managed binary) — solid
32. Format matrix MP4/H.264 · WebM/VP9 · WebM/AV1 (persisted) — partial
33. Audio: none/system-loopback/mic + list_audio_devices picker — partial
34. 720p default + max-second auto-stop cap — solid
35. Live REC timer + first-frame poster — solid

## Session replay
36. Same-origin rrweb record of THIS app in main window, live count, awaited stop — solid
37. Cross-window control channel dpl:session-cmd/state — solid
38. Record-time masking law — solid
39. write_inproc_replay full-array → inbox — solid
40. External-URL recorder: injected capture window + token-gated loopback ingest — partial (CSP blocks)
41. URL-recorder live status polling — solid
42. In-app ReplayPlayer: rrweb replay, fit-to-slide, scrub/±5s/speed/readout — solid
43. NOTE: DOM-first (rrweb), NOT video-first — partial framing
44. list_replays → asset URLs — solid
45. CaptureForm sessions surface; web Playwright fallback — partial
46. Full-page reload ends recording (known gap) — partial

## Steps recorder
47. Start/Stop, 5s prep countdown, auto-capture — solid
48. rdev GLOBAL mouse/keyboard hook: auto-step per click/Enter in FOCUSED window — solid
49. Window-FOLLOWING capture, stores app+title + screen origin — solid
50. do→done two-frame w/ fingerprint dedup (distance 0 → drop) — solid
51. Click coords rebased to window origin (click-here highlight) — solid
52. Forced-mark Ctrl+Shift+X (global) + Mark button, never deduped — solid
53. Global Ctrl+Shift+9 start/stop from any app — solid
54. REC pill: NON-ACTIVATING (never steals focus); enter/exit_steps_pill — solid
55. Live tally + skip-reason feedback (dpl:steps-state/skip) — solid
56. Ultra-light lossy WebP frames — solid
57. guide.json + step-NNN.webp on Stop, PENDING for Save card — solid
58. Seed-name from first window title (kebab, capped) — solid
59. Guide EDIT rewrite (keep/reorder/drop, deletes frames) — solid

## Save card / meta
60. capture-meta-fields: FORCED metadata (episode-or-General, kebab _v1, ≥1 tag) shared by every vehicle — solid
61. Steps Save card (name seeded + associate + tags; Discard) — solid
62. save_guide/discard_guide — solid

## ViziWiz
63. Ink-over-image canvas at natural size, 6 fx ink tones, width, undo/clear — solid
64. Image picker from artifacts (first 60 thumbs) — solid
65. Flatten → new PNG via forced-metadata inbox; source never mutated — solid
66. ort/BiRefNet cutout — ABSENT in old repo (net-new, not port)

## Gallery/inbox/index
67. Gallery: ContentLibrary over index, thumbs, filter/tag/associate/keep/clear — solid
68. Inbox review: keep-into-content vs dump, per-row edit — solid
69. Extension Downloads sweep — solid
70. Turso/SQLite index: open+migrate, startup reindex, recents, labels/meta update — solid
71. scan/list/associate/import (copy-into-episode law) — solid
72. Typed dpl:capture channel (started/saved/failed/inboxChanged/paletteVisibility) — solid

## Jails/infra
73. Inbox path jails (file vs entry vs read-only) — solid
74. Inbox JSON/asset readers for Studio — solid
75. list_capture_targets (monitors + visible windows) — solid
76. Managed ffmpeg injection + capture-cli — solid
77. Taskbar work-area clamp + --kiosk — solid
78. Clipboard + global-shortcut plugins wired — solid

## Absences
79. OS tray — ABSENT (old too)
80. BiRefNet cutout — ABSENT (old too)
81. Wayland capture — known gap
82. "Video-first" session player — actually rrweb DOM

Port notes: capture engine is a Tauri-free crate (xcap/image/ffmpeg-managed, rdev for steps). Region overlay is plain HTML/JS in public/capture/.
