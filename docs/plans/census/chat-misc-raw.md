# Census raw — CHAT/AGENTS + APP-WIDE MISC (old repo) — 61 capabilities + register

## A) Agent console ("Council of C") — C1 built, C2+ planned
1. Agent console domain, 3 surfaces from one body (dock + /agent route) — solid
2. Session sidebar: every Claude Code + Codex session on disk, newest-first, project filter, no spawning — solid
3. Transcript reader/normalizer: Claude & Codex JSONL → AgentEvent[]; unknown lines shown raw never dropped — solid
4. Unified AgentEvent model (session/message/tool/quota/result/unknown) — solid
5. Event-renderer registry (Map, per-kind render, unknown degrades) — solid
6. Six event renderers — solid
7. Rust read-only session reader (path-jail, LAW never writes) — solid
8. Native seam list/read session — solid
9. Agent store: tabs (MAX 8 oldest-evict), dock visibility, session-only — solid
10. Agent launcher tile (fx motion tier; only moving shell element at rest) — solid
11. Agent dock persistent footer panel — solid
12. /agent hub route + nav-dock entry — solid
13. Quota telemetry (Claude rate_limit_event normalized; Codex never faked) — solid
14. Live-tail seam (normalizeLine exported, nothing calls it) — PARTIAL
15. C2 headless Claude driving (start/send/stop, stream-json) — PLANNED ONLY
16. C3 Codex exec --json driving — PLANNED ONLY
17. C4 Council of C (roles, reversible direction, handoff, turn-cap) — PLANNED ONLY
18. COSTAR composer registry — PLANNED ONLY
19. Provider mesh 5-wide table — PLANNED ONLY (hardcoded 2-wide in 5 sites)
20. Context-provider registry — PLANNED ONLY
21. Signals/fabric — PARTIAL: dpl:capture typed enum + validated; steps/session events untyped; Signals v2 planned

## B) Shell chrome
22. AppShell composition (TopBar/LeftRail/main/RightPanel/NavDock/BottomBar/AgentDock) — solid
23. TopBar: brand, pathname readout, BackdropPicker, full-bleed toggle — solid
24. BottomBar launcher inventory: Snip, Dock grip, w×h·dpr readout, shortcut legend, AgentLauncher — solid
25. LeftRail: collapsible, fluid width, per-row --rail-tone, RAC tooltips — solid (static icon sizes = open #15)
26. NavDock trapezoid command surface: hub tabs → sub-routes + tools, Ctrl+Shift+Z/2 — solid
27. NavDock config AS DATA (hubs/sub-routes/tools incl. wiredFRONT 5 subs + PixVix/ViziWiz tools) — solid
28. RightPanel tabs notes/queue/inspector/data — STUB (all "coming soon", Pass G)
29. Shell store + persisted adapter (session-only vs validated persisted — "the fullBleed lesson") — solid
30. Shell keyboard + shell-size context — solid

## fx/style layer
31. fx classes: fx-card applicator, fx-halo, fx-glass-strong, fx-sheen, fx-bg-*, 9 @property slots — solid
32. Tone system: --t-* slots, [data-fx-tone] subtree retone, .fx-<cat>-<tone> mixers — solid
33. Halo tier rest/hover/press — solid
34. fx matrix generator — solid
35. RenderForge motion/keyframes + neumorphic "diffusion" gradient traits — solid
36. Token build pipeline + contrast/a11y gate — solid

## Minigame input
37. useDragToken: pointer-capture, multi-chip, scale-aware, hit-test, pointercancel→tray — solid
38. drag-match keyboard path (Enter/arrows/Escape) — solid
39. Minigame contract 9 requirements — solid (doc + partial doctor)
40. useKeyboardNav/useSwipeNav/usePinchZoom/useReducedMotion — solid
41. Roster: reveal, choice, quiz, drag-match, hotspots, predict, cost-meter — solid

## Session replay (rrweb)
42. In-process app session recorder (bundled rrweb; dpl:session-cmd/state; write_inproc_replay) — solid
43. Replay player (scrub, speed w/o rebuild, keyboard seek) — solid
44. Masking law + FS service + seam — solid
45. MV3 browser replay extension v0.3 — PARTIAL (awaiting operator verify)
46. Import/use pipeline solid; rrweb→VIDEO export bridge OPEN

## Routing
47. Full route tree (deck/present splats, content.$, mcp.ts + well-known oauth, overlay.pixvix) — solid

## Boundary tooling
48. check-fluid — solid
49. check-routes (bans side-effect imports in routes) — solid
50. check-bundle — solid
51. Doctor — solid
52. shell-measure + freeform-check + backnav-check drills — solid

## Lint
53. Biome 2.5.4 hardened — solid
54. Composite lint gate — solid

## Harnesses
55. green.mjs single gate — solid
56. exe-smoke (WebView2 CDP walks real exe) — solid
57. steps-check — solid
58. pixvix-drill — solid
59. portable-drill — solid
60. workbench-drill — solid
61. package scripts (package-portable, fetch-ffmpeg, optimize-media, tree, ext-episodes, pixvix-shot) — solid

## OPEN-ITEMS REGISTER (verbatim status)
1 T program — in progress. 2 MV3 ext — v0.3 awaiting verify. 3 replay→video export — OPEN. 4 EOS-001 built, operator-gated. 5 F' long-form ep — OPEN (needs stream doctor profile). 6 Pass G right-panel — OPEN. 7 Pass H hardening — OPEN. 8 content.$ may serve source/ — OPEN debt (security-adjacent). 11 tooltip.variants interpolated classes — OPEN. 12 visx stroke convention — OPEN. 13 format-sweep ruling — NEEDS RULING. 14 media verified Windows-only; bg-removal segfaults Windows — OPEN. 15 static icon sizes in chrome — OPEN. 16 video/video-loop fixtures need demo clip — OPEN. 18 Wayland capture — OPEN. 20 region multi-monitor — OPEN (primary only). 22 assetProtocol scope ["**"] — OPEN debt (security-adjacent).
(9, 10, 17, 19, 21 resolved.)

Notes: agent C2+/mesh/COSTAR = detailed plan docs, ZERO implementation. "diffusion" = neumorphic gradient traits, not a token.
