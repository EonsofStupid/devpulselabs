# CENSUS — the old repo's full capability surface vs the Rust rewrite

Compiled 2026-07-22 from five parallel read-only audits of
`C:\localdev\devpulse labs` (deck/present · workbench/studio · wiredFRONT
capture · content system/native seam · chat/agents + app-wide), diffed
against the rewrite at this commit. **No silent drops**: every audited
capability appears below. Evidence paths live in the old repo.

Legend: ✅ CAPTURED in the rewrite · 🟡 PARTIAL (exists, gaps named) ·
❌ MISSING (not in the rewrite) · ⬆ rewrite is AHEAD of the old repo ·
✖ was never built in the old repo either (planned-only there).

---

## 1. Deck / Present runtime — ❌ THE BIGGEST GAP (essentially all missing)

| Capability (old) | Disposition |
|---|---|
| Episode manifest schema (schemaVersion union, brick-safe) + per-slide contract (component/props/beat/chrome/tone/fx/density/steps) | ❌ — dpl-doc has the SLIDE document (slide-as-folder, ratified as the new form); the EPISODE-level document (sequence of slide folders, chapters, show binding) does not exist yet |
| Structural integrity (dup ids, dangling chapters) + show-kind doctrine (chapter-lock, ≤3 interactive budget, 15–24 band) | ❌ |
| Manifest→deck adapter + broken-slide escape hatch + chapter markers | ❌ (document renderer exists for ONE slide; no deck) |
| Beat-grammar reveal steps (advance/retreat, step0=setup, roll-over, retreat-to-last) | ❌ — slides have entrance motion, no step navigation |
| Route/URL as position source of truth + clamping + legacy redirects | ❌ (n/a form — needs Rust-native equivalent: shell route + slide/step state) |
| DeckHost: keyboard nav (←/→/Space beat, ↑/↓ slide, g/a/h), swipe nav, 12vw edge tap zones, progress rail + chapter ticks, runtime toolbar, slide navigator overlay | ❌ all |
| Touchscreen escape hatches: Home dot, bottom-edge reveal strip, per-slide chrome/annotate overrides | ❌ — critical for the 75" prop |
| Stream-time ink annotation layer (pressure width, brand ink token, per-slide clear) | ❌ |
| Presenter view: teleprompter, beat jump-list, active-beat script, up-next, timer, window reuse | ❌ |
| Audience↔presenter position sync | ❌ (BroadcastChannel n/a — Rust form: fabric event or second viewport) |
| MDX-lite scripts (YAML frontmatter + inert body, per-file degrade, beats-parity) | ❌ |
| ScaledStage/FitScale (fit content to box) | 🟡 — `Fluid::of(rect)` is the container-based UPGRADE; an explicit fit-scale helper for fixed-logical-size stages is absent |
| HoloScroll (holo arrows, rAF scroll) | ❌ (egui ScrollArea used; the affordance styling not ported) |
| SlideFrame safe-area/align/bleed | 🟡 — Stage exists in slides/frame.rs; bleed/align modes unverified |
| Pinch-zoom + pan (zoomable still) | ❌ — touchscreen need |
| Reduced-motion support | ❌ |

## 2. Slide templates & blocks — 🟡 (roster diverges)

| | |
|---|---|
| Ported live w/ demos (18): title, stat, callout, code, split, reveal, choice, quiz, drag-match, predict, stats, cost-meter(+face), bar-compare, line-trend, donut-share, probability-bars, stack-layers | ✅ (gallery-gated on the Library wall) |
| Missing templates from the old 19: **still** (+zoomable), **video**, **video-loop**, **bullets**, **hotspots**, **diagram**, **replay** | ❌ (Media BLOCK covers image/video slots inside templates, but the standalone slide templates don't exist) |
| Chart infographics + stack-layers + Beat motion context | ⬆ net-new in rewrite |
| drag-match keyboard path (Enter/arrows/Escape) | ❌ (ledgered) |
| Minigame contract (9 requirements incl. commit-before-reveal, aria-live, no-dead-ends) | ❌ as enforced doctrine |
| Registry + fixtures + class taxonomy | ✅ (TemplateSpec/TemplateClass + demos()) |
| formHints for the guided inspector | ❌ (W4 will need the equivalent; old had 2/19 anyway) |
| Custom per-episode component home | ✖ stub in old too |

## 3. Content system + filing — ❌ mostly missing (dpl-doc is the new seed)

| | |
|---|---|
| show.json / episode.json / script schemas + kebab-id law | ❌ (dpl-doc Slide schema v1 = the new-form start; show/episode/script tiers absent) |
| Structure doctor (9-rule platform-free core) + doctor report UI | ❌ — dpl-doc `validate()` is the seed of it (slide-level only) |
| Episode template builder + creator wizard + author stepper (6-stage golden path, dirty guard, budget readout) | ❌ |
| Scan/load (shows, episode, scripts) + content-root resolution (CLI→portable anchor→bundled→./content) + content_status honesty | ❌ (engine-capture has content_root() only) |
| content-cli headless verification bin | ❌ (verify-content skill targets the old repo) |
| Asset resolution (per-episode, content-root-relative, absolute) | 🟡 — locate_asset covers repo-relative; episode-relative model absent |
| **Artifact index DB** (embedded sqlite/turso: recents, 15-field record, upsert write-through, reindex-from-disk truth, rekey/soft-clear, tag edit, additive migrations, ai_labels seat) | ❌ entirely |
| Sidecar law (.meta.json + .poster.jpg companions travel) | ❌ |
| Inbox review: keep (file-per-metadata into EXISTING episode, companions, rekey), dump (inbox-only delete), pull-downloads sweep, jails (file/entry/read-only w/ symlink check) | 🟡 — write-side inbox + unique_path exist; keep/dump/review/jail surface missing |
| associate_artifact / import_inbox_asset (copy-never-move self-enclosure) | ❌ |
| write_inbox_image (editor composite → inbox) | ❌ (ViziWiz path) |
| Media pipeline: image AVIF/WebP/JPEG optimize, video transcode+poster, animation webp, sequence stitch | ✅ engine-media (+ media-cli) |
| Background-removal cutout | 🟡 — engine-media carries the cutout arm; ort/BiRefNet worker NOT ported (old ran it as an isolated subprocess; segfaulted on Windows — revisit) |
| Portable packaging: fetch-ffmpeg ✅ · package-portable ❌ · data_root law ❌ · runtime_status self-report ❌ · portable-drill ❌ · kiosk --kiosk ❌ | 🟡 |
| ext-episodes / import-replays / use-replay scripts | ❌ (replay vehicle TBD, see §5) |

## 4. Workbench / Authoring — 🟡 (W-passes are the replacement track)

| | |
|---|---|
| Component gallery (live, boundaries, degrade states) | ✅ as Library templates wall + focus solo |
| Token explorer: pool grid ✅ · moods ✅ · fonts ✅ (⬆ old had none) · gradients grid ❌ · fx tone matrix ❌ · fx ingredients chips ❌ · click-to-copy ❌ | 🟡 |
| Authoring Studio: palette / compose canvas / inspector / episode draft mutations / drag-or-tap placement / PropsForm schema forms / preview validation | ❌ — ratified replacement = W3 presets + W4 tap-to-edit editor over seed documents (layman-touch UI law); parity target, different shape |
| Guide reviewer (do→done frames, click-highlight ring, edit: reorder/delete/caption, save) | ❌ |
| Frame-to-slide import (capture → episode asset → still slide) | ❌ |
| Content Library panel (unified grid, rescan, filters, cards w/ honest fallbacks) | ❌ (wiredFRONT Gallery subroute is a placeholder) |
| RROFlow | ✖ stub in old too |
| RenderForge recipe layer (resolver, trait compositor, duoGradient) | 🟡 — reauthored as the token/registry/preset architecture; duo-gradient concept = mood_blend ⬆; recipe/trait compositor itself = W3 presets |
| Seeds wall + document renderer + mood-blend + fonts pool | ⬆ net-new |

## 5. wiredFRONT capture suite — 🟡 (core vehicles alive, pro affordances missing)

| | |
|---|---|
| Palette overlay window, tabs Img/Video/Session/Steps, warm hide | ✅ (pixvix crate) |
| Global summon hotkey (Ctrl+Shift+S OS-level) | ❌ — in-app chords only; rdev OS-global on ledger |
| Filing chip + meta popover + auto-name-never-blocks | ❌ |
| First-run per-tab help, settings (clipboard toggle, format defaults) persisted | ❌ |
| Honest capture-blocker banner / runtime_status | ❌ |
| Stills: full ✅ · region + frozen frame + loupe ✅ (4× vs old 6×) · window-mode + picker ❌ · **freeform polygon** ❌ (ledgered) |
| Capture delay countdown / re-snip / auto-clipboard-copy | ❌ all three |
| Overlay: arrow-key nudge (Shift×10), draw-then-confirm, cancel paths, headless hooks | 🟡 — select+cancel exist; nudge/confirm ceremony missing |
| Video: start/stop window-or-full ✅ (record_ctl_to) · format matrix mp4/webm-vp9/av1 ❌ · audio (loopback/mic + device list) ❌ · 720p default + max-cap ❌ · REC elapsed + poster 🟡 |
| Steps: recorder + mark + save ✅ · **rdev global click hook + window-following + click coords + do→done fingerprint dedup** ❌ (in-app only today) · REC non-activating pill ✅ · global Ctrl+Shift+9/X (OS-level) ❌ · WebP frames ✅ · guide.json ✅ · seed-name from window title ❌ · guide EDIT rewrite ❌ · forced-metadata Save card ❌ (PV3) |
| Session replay: old = rrweb DOM (webview-only tech) → **needs the ratified video-first rethink**; nothing in rewrite. URL-recorder ✖ (fragile even in old) | ❌/rethink |
| ViziWiz ink editor (ink tones, width, undo, flatten→inbox) | ❌ — becomes a wiredFRONT editor feature (never mixed with Workbench) |
| Cutout ort/BiRefNet | ✖ absent in old too (net-new work when wanted) |
| OS tray icon | ✖ absent in old too |
| Gallery route (ContentLibrary over index) | ❌ (needs §3 index first) |
| Typed capture signal channel | ✅ fabric (⬆ typed end-to-end, no serialization) |
| list_capture_targets (monitors + windows) | 🟡 engine-capture has monitor/region; window enumeration unverified |

## 6. Chat / Agent console — ❌ (C1 was real in old; rewrite has a placeholder)

| | |
|---|---|
| C1 read-only console: session sidebar (Claude+Codex on disk), transcript normalizer (never drops lines), event-renderer registry, 6 renderers, tabs store, quota telemetry, jailed Rust reader | ❌ all — chat hub is a placeholder |
| Agent launcher tile + dock | 🟡 — footer Agents chip routes to the hub (no dock) |
| C2 drive Claude · C3 drive Codex · C4 Council · COSTAR · 5-provider mesh · context registry | ✖ planned-only in old too (plan docs are the salvage) |
| Signals v2 typed events | ⬆ the fabric already IS typed Rust enums end-to-end |

## 7. Shell / app-wide — 🟡

| | |
|---|---|
| Chrome composition, launchers, viewport readout, fluid rail widths, tooltips | ✅ (zones v2; status grid ⬆ registry-based) |
| NavDock trapezoid command surface (hubs/subs/tools as data, Ctrl+Shift+Z) | ❌ — rail flyout covers subroute nav; dock's tool-launch surface absent |
| RightPanel tools notes/queue/inspector/data | ✖ stub in old too (Pass G open) — inspector panel skeleton exists in rewrite |
| Shell shortcuts: [ ] rails, \ fullbleed, Esc cascade | 🟡 — fullbleed+Esc ✅; rail/panel toggles + \ binding ❌ |
| Backdrop picker | ✅ (topbar tiles + mood law ⬆) |
| Persisted-state law (validated persist vs session-only) | ❌ — nothing persists yet (rail state, settings) |
| fx ingredients (glass-strong, sheen ✅, scanlines ❌, noise ❌, bg-hero/aurora ✅/duotone ❌/spot ❌/conic ❌), halo tier 🟡 (chip hover washes), @property slots n/a | 🟡 — sprite vocabulary reauthored; specific ingredient inventory incomplete |
| Boundary tooling: green gate ✅ (xtask) · check-fluid n/a (rustc+tokens enforce) · check-bundle ❌ equiv (font/asset presence asserts partly in tests) · drills → EguiPilotAI ⬆ |
| exe-smoke/steps-check/pixvix-drill CDP harnesses | ⬆ superseded by EguiPilotAI (typed protocol, deterministic clocks) |
| MCP routes in-app | ⬆ superseded by egui-pilot-ai-cli MCP server |
| Kiosk mode + work-area clamp | ❌ (maximized boot only) |
| Wayland capture, multi-monitor region | ✖ open debt in old too |

---

## THE MISSING LIST — ranked for passes

1. **Deck/Present runtime** (§1 wholesale): episode document tier over dpl-doc
   → reveal-step grammar → DeckHost (touch nav, edge zones, progress, chrome
   hide + escape hatches) → ink layer → presenter+sync → scripts. The 75"
   prop cannot run a show without this. (Old evidence is complete; port-capture
   ready.)
2. **Artifact index + library** (§3): embedded index DB, sidecars, reindex
   truth, inbox keep/dump review, Gallery/ContentLibrary UI, associate/import
   into episodes. Unblocks reviewing every capture already being filed.
3. **W3+W4 authoring track** (§4): presets → tap-to-edit editor (ratified
   replacement for Studio/PropsForm), then guide reviewer + frame-to-slide.
4. **Capture parity** (§5): freeform polygon, window stills + picker, OS-global
   hotkeys (rdev), steps global click hook + window-following + do→done dedup +
   click-coords, forced-metadata Save card, guide edit, delay/re-snip/clipboard,
   video format/audio matrix, ViziWiz ink editor (wiredFRONT-side).
5. **Missing slide templates** (§2): still(+zoom), video, video-loop, bullets,
   hotspots, diagram (+ replay when sessions return); drag-match keyboard path;
   minigame doctrine; reduced-motion.
6. **Content doctrine** (§3): show/episode schemas, doctor, creator wizard,
   author flow — folds into the episode-document design (1).
7. **Agent console C1** (§6): sidebar + transcript reader + renderers (chat hub
   is empty; old C1 is a clean port source). C2+/mesh from the plan docs after.
8. **Session replay, rethought** (§5): rrweb is webview tech — design the
   video-first session vehicle (ADR) instead of porting.
9. **Portable/kiosk** (§3/§7): package-portable, data_root law, runtime_status,
   portable drill, --kiosk.
10. **Shell polish debt** (§7): NavDock-equivalent tool surface, persisted
    state law, remaining fx ingredients, shortcut parity.

**Never-existed (don't mourn, decide):** ort/BiRefNet cutout, OS tray,
C2+ agent driving, provider mesh, RROFlow, right-panel tools, ai_labels
producer — all were plans/stubs in the old repo too; they enter the ledger as
net-new features, not port debt.

**Rewrite is AHEAD on:** moods/mood-blend backdrop, fonts pool, fluid-as-
container math, typed fabric, status-cell registry, chart infographics,
document layer (refs-never-values), the green gate, and EguiPilotAI.
