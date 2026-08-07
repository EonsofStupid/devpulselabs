# DevPULSE automation and instrumentation census

Status: source audit on 2026-07-22. This records what the live Rust code
actually exposes. Existing proof images demonstrate rendering, not semantic,
accessibility, keyboard, or runner coverage.

## Executive finding

DevPULSE currently has a narrow pilot vertical slice:

- shell rail hub buttons and rail-flyout subroutes register pilot semantics;
- backdrop registers its ambient video motion;
- a real shell navigation flow is exercised through the runner.

Almost every other interactive surface uses raw `allocate_rect`, `interact`,
standard egui buttons, or selectable labels without pilot registration. The
18-slide Library catalog has no pilot semantics in its templates. Most slide
motion reads `ui.input(|i| i.time)` directly and is not registered with
`PilotClock`. PixVix has multiple real viewports and asynchronous capture work,
but no pilot identities, busy tokens, or motion registration.

This is expected at the current phase, but it means the following claims are
not yet earned: complete Library agent snapshot, AccessKit coverage, keyboard
operation, deterministic template capture, full multi-viewport control, or
Playwright-class scenario coverage.

## Cross-cutting corrections

1. `TemplateClass::Static` does not mean non-interactive. Code Block copies,
   Stats Grid flips, and Donut Share explodes slices while registered as
   `Static`. Automation capability must be derived from semantic registration,
   never the content taxonomy.
2. A proof screenshot is not a visual baseline until it records renderer key,
   deterministic motion preflight, comparison policy, immutable approval, and
   repeatability evidence.
3. Direct `ui.input().time` is deterministic only when the runtime owns
   `RawInput.time`; authored motion still needs registration so capture can
   identify terminal/canonical states.
4. Custom-painted controls need `pilot_track` attached to the real response at
   their real hit rectangles. Adding invisible standard widgets would create a
   false second UI.
5. Every interactive slide needs a keyboard path. Drag Match explicitly marks
   its keyboard path as deferred.

## Shell and hubs

| Surface | Live behavior | Pilot status | Required stable IDs / work |
|---|---|---:|---|
| rail hub tiles | click route; hover opens flyout | partial implemented | existing `shell.rail/<hub>` should move to `devpulse.shell.rail/<hub>` only through a migration alias; assert active state and focus |
| rail flyout routes | hover persistence and subroute click | partial implemented | existing `shell.flyout/<hub>/<sub>`; add expanded relation, menu/menu-item structure, keyboard opening/navigation/dismissal |
| full-bleed top button | enter full bleed | missing | `devpulse.shell.fullbleed.enter`; button, tooltip/name, keyboard shortcut assertion |
| full-bleed exit | click or Escape | missing | `devpulse.shell.fullbleed.exit`; button and Escape flow |
| backdrop loop tiles | choose Drift/Pulse and reroll mood | missing semantics; ambient motion registered | `devpulse.shell.backdrop.loop/<loop>`; selected state, mood status, deterministic seeded mood fixture |
| status Snip | opens PixVix through fabric | missing | `devpulse.shell.status.snip`; assert viewport-created event |
| status Agents | routes to Chat | missing | `devpulse.shell.status.agents`; assert route |
| inspector | route/signals/status text | missing vital text | headings/status IDs and values; no action |
| Library template wall title | focus a live template | missing | `devpulse.library.template/<template>` using `TemplateId` key |
| Library solo back/replay | return/reset template | missing | `devpulse.library.solo.back`, `.replay`; replay must assert semantic reset and motion restart |
| Library scroll area | scroll through 18 live stages | missing | scroll region and optional template reveal adapter |
| Workbench seed reload | reload seed documents | missing | `devpulse.workbench.seeds.reload`; fixture-owned data root and busy token if loading becomes async |
| Test Lab | passive tabs/views | partial | consume runner/session/artifact APIs; no duplicate control implementation |

Existing IDs are already public scenario inputs. Renaming them must use a
versioned alias/deprecation table rather than silently breaking durable flows.

## Slide catalog

All dynamic child IDs use the template owner plus a domain key, never an array
position. Demo labels may be keys only when the authoring schema guarantees
uniqueness and stability; otherwise the data type needs an explicit key.

| Template (`TemplateId`) | Interaction/state | Motion | Required semantic pattern and scenarios |
|---|---|---|---|
| Title Card (`title`) | none | finite entrance rise | heading/subtitle semantics; natural/fast-forward/suppressed capture |
| Big Number (`stat`) | none | finite bloom/fade | heading, statistic value/label/source; all motion policies |
| Callout Card (`callout`) | none | finite entrance rise | heading/callout region/body; settled capture |
| Split Panel (`split`) | none | staggered finite layer fade | heading and two named regions; reading order and settled capture |
| Reveal List (`reveal`) | whole-stage click advances count | finite pop per item | `...reveal.advance` button plus list/status; Enter/Space; stepwise count and clamp/restart |
| Choice Poll (`choice`) | one of two options commits once and reveals truth | reward pulse and reveal fade | radio group/options keyed in data, selected/disabled/reveal status; pointer and keyboard commit |
| Quiz Panel (`quiz`) | single radio or multi checkbox selection, then lock | reveal fade/reward pulse | group, keyed choices, checked/selected state, lock button enabled state, verdict status; single and multi cases |
| Stats Grid (`stats`) | cards with details flip; others inert | entrance and flip fades | each card region; flippable cards buttons with expanded state; keyboard flip and restart |
| Code Block (`code`) | Copy writes clipboard and temporary copied state | entrance fade and copied hold | code region, copy button, copied status; clipboard output assertion through typed platform output, not OS clipboard read |
| Cost Meter (`cost-meter`) | sequential Send actions, then Flip | fill/flip/reveal fades | stable send/flip control whose name/state changes intentionally, progress/value/status; full state-machine scenario |
| Predict Rank (`predict`) | reveal ranking then next round | bar grow/reveal fade | reveal/next buttons, round status, ranked list semantics; multi-round clamp/restart |
| Drag Match (`drag-match`) | drag four keyed chips into two targets; wrong returns, right locks | pulse/shake/bloom | keyed drag sources and drop targets, placed/error/progress status, deterministic drag trajectories, wrong/correct drop, keyboard pickup/place |
| Donut Share (`donut-share`) | ring slice click toggles exploded slice despite `Static` class | finite chart sweep plus explode state | chart region and keyed slice buttons with selected/expanded state; click/keyboard and capture during/after sweep |
| Bar Compare (`bar-compare`) | none | finite chart rise | chart summary plus series/category values; settled capture |
| Line Trend (`line-trend`) | none | finite chart draw | chart summary/series values; settled capture |
| Probability Bars (`probability-bars`) | none | finite bar grow | prompt and ordered prediction/value list; settled capture |
| Stack Layers (`stack-layers`) | none | staggered layer fade | two named stacks and layer ordering; settled capture |
| Cost Meter Face (`cost-meter-face`) | none | finite gauge sweep | progress/gauge numeric value and label; settled capture |

### Slide owner API additions

`TemplateSpec` needs stable automation metadata without turning it into a test
script:

- stable template ID (already present);
- declared semantic root role/name;
- required input modes (`pointer`, `keyboard`);
- declared finite/ambient motion IDs and terminal/canonical timing;
- data keys for repeated choices/items/slices/layers;
- optional deterministic reset checkpoint exposed by `restart` semantics.

The gallery supplies the template root scope. Each template owns descendants,
state, and motion. The Test Lab and runner discover behavior from the committed
semantic tree, not `TemplateClass`.

## PixVix multi-viewport census

PixVix creates `pixvix` and `pixvix-region` immediate viewports. Its current
surface includes:

- close;
- Img/Video/Session/Steps tabs;
- Region, Full, Free placeholder;
- window combo, window choices, Snap, refresh;
- start/stop video;
- start/mark/stop steps;
- full-screen region drag and Escape cancel;
- status text and an ambient recording pulse;
- background recording/capture completion and filesystem outputs.

None has pilot semantics today. Initial stable scopes:

```text
devpulse.pixvix.palette.*
devpulse.pixvix.tab/<tab>
devpulse.pixvix.image.*
devpulse.pixvix.video.*
devpulse.pixvix.steps.*
devpulse.pixvix.region.selection
devpulse.pixvix.status
```

Required instrumentation:

- viewport-created/destroyed events correlated to semantic roots;
- `Tab` roles and selected state;
- ComboBox/current selection/options;
- status/alert nodes for saved/error/deferred behavior;
- busy tokens for scan, still capture, video stop/finalize, and step export;
- finite/ambient motion registration for the recording pulse;
- region selection drag source/geometry and Escape path;
- jailed output fixture plus typed file manifest probe;
- honest boundaries: `Free` remains a disabled/placeholder status until the
  polygon editor exists; OS capture and global hooks are engine drills, not
  fake egui actions.

## Keyboard coverage contract

At minimum:

- Tab/Shift+Tab reaches every enabled shell, Library, slide, and PixVix
  control in a deterministic order;
- Enter/Space activates button-like controls;
- arrow keys move radio/tab selection where the component contract declares
  group navigation;
- Escape dismisses flyout, full-bleed, and region selection appropriately;
- Drag Match supports focus source -> pickup -> focus target -> place -> cancel;
- focus is visually evident and present in both pilot and AccessKit snapshots;
- no pointer-only surface is accepted without an explicit documented reason.

## Motion registration inventory

Register at authorship, using stable IDs:

- backdrop video loop: ambient (already registered; verify canonical phase);
- PixVix REC pulse: ambient;
- every template entrance/chart sweep: finite with terminal time derived from
  effect duration plus maximum data stagger;
- interaction effects: choice reward/reveal, quiz reveal/reward, stat flip,
  copy hold, cost fill/flip/reveal, prediction bars/reveal, drag pulse/shake/
  bloom, donut explode.

An interaction that starts a new finite animation updates/registers a motion
instance keyed by semantic owner. Restart clears instances and returns the
template semantic state to its declared initial checkpoint.

## Fixture and scenario catalog

Initial named fixtures:

- `shell-deterministic`: pinned loop/mood, viewport, DPI, locale, fonts;
- `library-all-templates`: all 18 demos with deterministic initial state;
- `library-template-<id>` should be a parameterized case, not 18 duplicated
  fixture implementations;
- `workbench-seeded`: temporary copied seed root;
- `pixvix-jailed`: temporary inbox/output root and deterministic target list;
- `pixvix-two-window`: palette plus region viewport with capture services
  replaced only at their typed engine boundary.

Durable scenario families:

```text
tests/eguipilotai/shell/
tests/eguipilotai/library/
tests/eguipilotai/templates/
tests/eguipilotai/workbench/
tests/eguipilotai/pixvix/
tests/eguipilotai/accessibility/
tests/eguipilotai/failures/
```

Prefer one state-machine scenario per interactive template plus parameterized
motion/accessibility matrix cases. Static visual templates need semantic and
capture cases, not gratuitous click flows.

## Completion gates for this census

- every live actionable response has a stable owner-declared pilot ID;
- every actionable node has a truthful accessible name, role, state, and
  supported action in real AccessKit output;
- every repeated item uses an explicit domain key;
- every async lifetime has a busy token;
- every visual motion is registered and tested under required policies;
- every pointer behavior has keyboard coverage or an approved exception;
- Library's default pruned snapshot remains within 12 KiB while retaining all
  visible actions and route context;
- each interaction family has a real-process durable scenario;
- every required visual has a renderer-keyed approved baseline and fifty-run
  Lavapipe repeatability evidence;
- PixVix proves viewport discovery, interaction, capture, cancellation, and
  shutdown without controlling the Test Lab in-process.
