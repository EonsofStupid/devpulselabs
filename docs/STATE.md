# STATE — the living ledger

Updated every pass. This file is the first read of every session.

**2026-07-30 — PUTTING A CARD IN ITS CONTAINER (M7,
`docs/plans/author-place-into-container.md`):** the filmstrip has been telling
the operator "N started here, not put in yet" since #146 — accurate, and an
outstanding promise the band offered no way to close. Now it does.
`dpl_doc::place_slide` already did the mutation carefully (COPIES not moves,
refuses an unfinished slide, refuses to create an episode, never clobbers,
rolls the episode back if the result would be invalid), so the correct amount
of new mutation code was ZERO — this pass added the gesture, the chapter
choice, and honest reporting. One chapter is not a decision and is used; more
than one is, so the strip offers them by their authored titles. Only an
EPISODE holds slide folders: guide/lesson/deck each say why via
`cannot_take_a_card`, and a test asserts the guide wording contains no "yet"
because a guide is a different content model, not a shortfall. The count is
RE-READ from disk after placing, never incremented. FINDING: `place_draft`
first read `content_root()`, a process-wide `OnceLock` that cannot be pointed
at a scratch dir from a test — so the root became a parameter. Better design
independently, and the only reason the two surface-level tests exist;
"it cannot be tested" turned out to be "it reads a global it did not need to".
44 tests, 6 new — including that a REFUSED place keeps the card and says why,
because silent success is the one failure this gesture must never have.

**2026-07-30 — THE AUTHORING SURFACE, ALIGNED (`docs/plans/author-surface-alignment.md`):**
five tabs and three censuses were built in a day, each correct alone and each
slightly different. Pre-release, so the drift was REWRITTEN OUT before the
Picture tab landed rather than merged and cleaned up after. Four alignments:
one `pretty()` in a new `words.rs` (it had been copied one layer above the
exact place the lane already paid twice to stop copying vocabulary); two
`pub(crate) enum Kind` meaning unrelated things become `ContainerKind` and
`PictureKind`; "a thing you can pick" had three shapes (`slug`/`label`,
`id`/`name`, `rel_path`/`name`) and now every one carries `slug` + `label`, so
the data types follow the law the methods already did; and all four tab
functions take `&mut CardEditor` instead of two taking each. A fifth fell out:
one shared `card_editor()` test builder, so a test cannot construct the
surface's state differently from the surface. NOT aligned, deliberately: the
census return types — `Shelf` carries `more`/`unavailable` the others have no
concept of, and symmetry would mean inventing state. Nothing drawn changed:
the four `no_*_speaks_engineering` tests pass untouched and the proof shot is
structurally identical. 38 tests, up from 37 — none deleted, one added.

**2026-07-30 — THE PICTURE TAB (`docs/plans/author-tab-picture.md`):**
fifth of the seven card tabs; M5 is now 5 of 7 and **the two that remain are
BLOCKED, not pending**. The picker is the artifact INDEX, not a file dialog,
and that is the design: every `Record.rel_path` is already
content-root-relative, so `dpl_doc`'s media content-jail **cannot be violated
by construction** — this module never builds a path, it passes one through.
`show` comes from the record's own `kind`, never sniffed from the extension.
`cover` is not written (schema default; sparse documents). One picture per
card — a second replaces. TWO defects the proof run caught, both in §8.3: (1)
`picture_tab` called the census EVERY FRAME, and the census opens a database
connection — the exe never reached its screenshot; the shelf is now cached on
`CardEditor` and re-read only by the gesture that changes it, the rule
`author-filmstrip.md` §6 had already written down. (2) `dpl_video::locate_asset`
was NOT content-root aware, so **every drill using `--content-root` resolved
nothing** — media cards included, not just this picker. Fixed in `dpl-video` by
reading `--content-root`/`CONTENT_ROOT` from the process args, the same
contract `engine_capture::explicit_root` implements, with no new dependency.

**2026-07-30 — C2c MOCK LOOK-APPROVED (`docs/ops/references/coach-operator-mock-c2c.html`):**
Operator: “looks good.” Coach hub home locked for implementation.
Product spine: `docs/plans/coach-operator-experience.md`. Coach NEXT =
**C2d** lightweight target inventory (Phase 0+1).

**2026-07-30 — COACH PRODUCT SPINE + C2c MOCK (`docs/plans/coach-operator-experience.md`):**
Physics (C2s/C2b #158) stays proven; Operator Coach UX path — picker,
lightweight inventory, fabric mint from a11y streams (not screenshots),
seeds + refuse tips; Advanced/EguiPilot is a second surface. Mock:
`docs/ops/references/coach-operator-mock-c2c.html` (**Coach hub**).
Pass-2 stdio / strict pinning **parked**.

**2026-07-30 — THE ALIGNMENT PASS (operator: "I want my project back
under control… getting it all aligned"):** every stray landed, one truth
on main. (1) THE checkout is BACK ON MAIN, clean, exe fresh (23:58
rebuild covers #162–#165 incl. the Lucide engine); the `devpulselabs-fx`
worktree was squatting the `main` branch ref — detached (nothing
deleted). (2) The parallel icon-language session's uncommitted work is
FILED: its ADR (was a second "ADR-015") renumbered to
`adr-016-icon-language.md` verbatim with a supersession header — the
engine decision was settled by the LANDED zero-dep `dpl-svg` (#163,
ADR-015 + 2026-07-30 amendment); its plan `icon-language-v3a.md` filed
with an alignment note. (3) OPERATOR LAW captured from working trees
onto main: the doctrine RULING "clay IS a button" (2026-07-30,
doctrine.md); the V3 redirect ("Lucide for now…"); the MOCK GATE ("We
should have mocks built first for a clear vision to work from") — the
vision mock + the operator's 5 reference images now live at
`docs/design/mocks/` + `docs/design/references/`, and NO styling pass
(V3 M2 migration, variants, V4, V5) executes until the operator
ratifies the mock (POAM V3 + sequencing table updated). (4) Orphans
landed: `adr-010-workbench-assembly.md` (ADR-010 finally authored,
awaiting ratification), `wiredfront-capture-share-spec.md` (operator
spec, verbatim), the L4-closeout journal, the signals ledger line.
Open PRs after alignment: #160 (Picture tab, another session's),
dependabot #84/#80 (major bumps — operator decision). NEXT: the
operator ratifies/redlines the vision mock, then M2 + V3b/V4 proceed.

**2026-07-30 — THE "A QUOTE" TAB (`docs/plans/author-tab-quote.md`):**
fourth of the seven card tabs. Operator-ratified 2026-07-29: **a quote is a
CONVENTION over text blocks**, not a `Block` kind — and the decision held all
the way down, because nothing needed special-casing at render. `dpl_doc::
quote_lines()` declares the shape beside `enters()` (`quote` h2/strong ·
`said-by` body-lg/tone · `source` body-sm/subtle) so the surface can never
drift from the validator, and a test in `dpl-doc` asserts the convention's own
defaults validate. `Editor::set_quote` adds/updates/removes ordinary blocks: a
BLANK line REMOVES its block (an empty text block is a rendering artefact),
writing twice UPDATES rather than appending a duplicate, and a card's existing
lines are left alone with the tab saying so. Typing does not write per
keystroke — `CardEditor` holds the buffer, because `set_quote` snapshots undo
and per-character writes would put fifty entries on the stack for one
sentence. Three plain fields (What was said · Who said it · Where it's from)
with the M5b live pane rendering the real card beside them. M5 = 4 of 7.

**2026-07-30 — C2B `dpl-coach-overlay` SELF-TEST PASS (`docs/journal/2026-07-30-c2b-coach-overlay.md`):**
Workspace member `crates/coach-overlay`; ADR ratified; `--self-test --name 7`
→ Triggered exit 0 on calculator (`stream_hit="7"`); bogon refuses exit 2.
Never a dep of `app`. Landed #158. Pass 2 stdio parked (Advanced).

**2026-07-30 — C2A COACH-OVERLAY ADR RATIFIED (`docs/plans/coach-overlay-adr.md`):**
Boundary for `dpl-coach-overlay` — feature-gated / devtools-only process,
WINDOW-space paint, Sync-grab observe, identity gate, forbidden set,
launch gates. Operator authorized build 2026-07-30. C2b self-test closed
same day.

**2026-07-30 — C2S LIVE TRIGGERED (`docs/journal/2026-07-30-c2s-triggered-live.md`):**
Operator press on spotlit `gnome-calculator` `7` → `Triggered` (ButtonPress
inside hotspot). Stream identity via `GetAccessibleAtPoint(WINDOW)`;
paint on calculator client (WINDOW space); caption above ring (caption-below
had looked like `5`). Wrong-digit no longer false-Triggers.

**2026-07-29 — C2S TRIGGERED OBSERVE (§10 of `coach-c2s-foreign-target-spike.md`):**
`foreign-target-probe spotlight` reports `Triggered` on Button1 press-edge
inside the composed hotspot via `QueryPointer` (overlay stays
`mouse_passthrough` — never eats the click). Display Text polling retired
as the gate. `--proof-xtest` proves unattended; live operator press is the
product path.

**2026-07-29 — THE OWNED LUCIDE ICON ENGINE (#163, ADR-015, POAM V3 M1,
plan `docs/plans/lucide-icon-engine.md`):** the operator's "pull lucide
and properly own it as part of coldlight" landed double-green. Lucide
1.27.0 vendored WHOLESALE (1,756 SVGs + LICENSE + VERSION,
`crates/coldlight/assets/lucide/` — the pool is the authored source, only
registry-referenced icons enter the binary, the fonts law). New ∅-dep
ENGINE crate `dpl-svg` owns the icon subset of SVG (xml reader, full
d-grammar with arc→cubic, adaptive flatten, round cap/join stroking →
pure triangle buffers; refusals are NAMED; the corpus test tessellates
the whole pool every run). `tokens/icons.json` is the 8th authored
source (24 semantic ids); geometry bakes at BUILD time — runtime never
parses SVG. Styling composes at paint (the visx law): ink / tone /
BRUTAL (the icon's own mesh as a hard offset shadow shape, never blur).
Gallery-gated: `LibSection::Icons`, the library's 10th wall
(`--lib-section icons`; proof `docs/proof/lucide-icons-gallery.png`).
NEXT (M2): migrate the 40 Phosphor glyph call sites (11 files) so the
app wears Lucide, then retire `cl::icon`'s consts. ~~NOTE: the release exe
in THE checkout is OWED a rebuild~~ — SETTLED 2026-07-29 23:58: the
provider-tab session rebuilt `target\release\devpulselabs.exe` at
`4acfd86`, which includes this pass.

**2026-07-29 — CONNECTED PROVIDERS EARN A TAB (POA&M M3, plan
`docs/plans/chat-connected-provider-tabs.md`):** the third clause of the
operator's regression report is built — an ACTIVATED provider
(`is_activated()` = Ready or Experimental) gets its own tab in the
title-bar strip beside AI and Social, and entering it scopes the AI rail
to that provider's sessions. `provider_tab: Option<ProviderId>` on the
window, NOT a `ChatDomain::Provider` variant (that costs
`ChatDomainStates::get/get_mut` their `const fn` and grows every
exhaustive match for no behaviour this pass ships — §3.1). Because
`focused_provider_tab()` filters the focus through the live tab census,
SIGN-OUT DROPS BOTH the tab and the scoping in the same frame with no
intent sent. `session_groups` needed zero change — scoping is
`slice::from_ref`. Proof `docs/proof/chat-provider-tab-native.png`
(AI · Codex · Gemini · Social; Blocked/SignedOut providers tab-less).
Lessons: **the first EguiPilot run after any rebuild fails the handshake
— warm-up race, re-run it**; `chat-native-proof` needs the RELEASE exe
built WITH `--features eguipilotai-devtools` (it spawns `--chat-proof`,
which is gated — otherwise the viewport never opens and the shim blames
window discovery); `hidden` cannot prove a node was never minted, use
`count = 0`. PARKED SIDE-FIND: `chat-peer-window.toml` fails on `main`
itself (`chat.context.route` reads `workbench`, expects `wiredfront` —
bisected against unmodified `faf5132`); default landing route seems to
have moved under #155/#156, decide stale-expectation vs regression.

**2026-07-29 — PROVIDER EASY-SELECT RESTORED (POA&M M1+M2, plan
`docs/plans/chat-provider-easy-select.md`):** the #94 regression is
undone — the AI rail again shows EVERY provider as an always-visible
one-click row (accent mark · status dot · activation label · session
count; `provider_roster` replaces the dropdown `provider_selector`),
with the catalog overlay demoted to the `＋ More providers…` escape
hatch (Add local / Add custom / census). Catalog copies of the rows
re-homed to `chat.provider.catalog/<id>` so rail + overlay never mint
duplicate pilot ids. Activation paths UNTOUCHED (M2: zero non-window
lines in the diff; scenarios `chat-provider-catalog`, `chat-peer-window`,
`chat-assist-toggle` pass unchanged; new scenario `chat-provider-rows`
proves roster-without-overlay + escape hatch). Proof:
`docs/proof/chat-provider-rows-native.png`. Session POA&M:
`docs/plans/poam-2026-07-29.md` (M3 = connected-provider tabs NEXT).
Lesson: `chat-native-proof` captures the RELEASE exe by window title —
rebuild `devpulselabs.exe` BEFORE shooting proofs.

**2026-07-29 — THE OVERLAY'S LIVE PANE (M5b, `docs/plans/author-overlay-live-pane.md`):**
the card editor becomes what the mock specifies — a tab rail down the left,
the tab's controls in the middle, and THE CARD ITSELF, LIVE, on the right
("as it looks right now"). The operator asked for this in as many words: "the
card would then appear show live preview as i start to fill it out". Sequenced
AHEAD of the four remaining tabs so they arrive into the right shape, and it
closes the below-the-fold problem from author-tab-background §8.3. ONE
deliberate deviation from the mock's uniform two-pane body: **Words keeps its
own stage and gets no live pane** — everywhere else the card is a preview, but
on Words it is the CONTROL (you tap a line on the slide to select it), and two
cards where only one answers a tap is worse than one. Pinned by
`CardTab::shows_live_card`. The pane renders the editor's IN-MEMORY slide via
`render_thumbnail` with a clock owned by `CardEditor` — not a file re-read,
not a rebuilt `DocSlide`, either of which would restart the entrance on every
keystroke; `Editor::look` is new so the preview cascades like the card does.
Build notes worth keeping (§8.3): nested egui layouts allocated three columns
and painted nothing — explicit rects, the carousel's idiom, worked at once;
and `scope_builder` walked the footer into the band because the band had
already claimed its height (`new_child` does not). NEXT = the four remaining
tabs (A quote · Picture · On tap · Ask for help).

**2026-07-29 — THE MOVEMENT TAB (`docs/plans/author-tab-movement.md`):**
second of the six placeholders, and deliberately small because the truth is.
GROUND TRUTH — `dpl_doc::ENTERS` permits exactly `rise` and `none`, and
`document.rs` implements `rise` alone; the other seventeen motion effects
belong to TEMPLATES for their own elements, so offering them would offer refs
the document cannot hold. The shelf is therefore THREE: follow the show ·
rise into place · stay still. `dpl_doc::enters()` now exposes the vocabulary
`validate` enforces, so a picker can never drift from it, and
`the_movement_row_is_the_schemas_vocabulary` fails the day a fourth entrance
lands unoffered. "Follow the show" names what it resolves to
(`Editor::look_enter`) because inheriting is invisible by nature; picking
restarts the clock so the entrance plays. The shortfall is stated ON the tab:
arriving is all that is built, and getting noticed / going out each need a
movement authored + the schema taught to name it + the player taught to play
it — which is why none of that shipped under a tab's name. NEXT = A quote,
Picture, On tap, Ask for help.

**2026-07-29 — THE VISUAL-EXPERIENCE POAM (`docs/plans/poam-visual-experience.md`):**
ratified from the operator's directive: V1 animation becomes a HOME
privilege (every other route gets a static mood wash, zero idle repaints);
V2 mood-derived card backgrounds as a generated ColdLight ingredient; V3
the operator's years of SVGs become the icon language (intake → ADR on the
SVG→egui path → generated registry → the icon-only rail wears them); V4
rail links become ACTUAL clear buttons (neubrutalist actionable trait); V5
the Home showcase — clay + brutal, first frame is the finished look. Each
milestone is its own plan-gated pass. V3 is gated on the operator pointing
at the SVG set and its ADR.

**2026-07-29 — THE INSPECTOR IS AN OVERLAY (#148, `docs/plans/inspector-overlay.md`):**
the shell's right panel was ported as a DOCKED `egui::Panel::right` eating
`panel_width()` (320–448 px) on every route — but the operator's directive has
been in docs/shell.md verbatim since task #103: "rightside **overlay** the
content". Now faithful: closed by default (`Zones.panel == Rect::ZERO`, the
well spans to the root's right edge — ~30% more editor width at 1100×700);
the topbar toggle beside full-bleed (or `--inspector`) floats the plate OVER
the well at `panel_width()` — the content rect never shrinks, nothing
reflows; close via the plate's ✕ or the toggle. Existing chrome vocabulary
only (RAISED + sheen + hard-edged left diffusion band) — no new ingredients,
no token changes. docs/shell.md Law 8 names re-docking a regression;
`the_inspector_overlays_the_content_instead_of_docking` pins the open plate
INSIDE the content rect. KNOWN (parity-kept, future polish): the inspector
cards spill under the plate's right margin exactly as they did when docked —
clipped identically. NEXT candidates in this lane: per-hub tool docks (P1)
mount into the overlay; an open/close slide animation.

**2026-07-29 — THE BACKGROUND TAB (`docs/plans/author-tab-background.md`):**
the first of the overlay editor's six placeholders becomes real. Three rows in
the order a person decides — the colour, blend a second, something moving over
it — every tile a LIVE render of the backdrop it offers, painted through the
same path the card uses. Rows two and three stay hidden until row one is
answered. `backgrounds.rs` is the census: colours are `cl::MOODS` from tokens,
movement is the `Ambient` vocabulary `document.rs` resolves, mixes are
`cl::MOTION_PRESETS`. GROUND TRUTH — the mock promises "around forty"; the
registry holds **TEN** colours, and the forty is real only through COMBINATION
(ten flat, forty-five pairs, each with a mix and optional movement). The
heading states the real count off `colours().len()`; the shelf was not padded
to reach a number. Writes go through a new `Editor::set_background`, so a
tapped colour joins the undo stack and saves like any other edit. Swatches
gained `widget_info` names — they are painted, not widgets, so neither a
screen reader nor the harness could find them. Seam `--card-tab <slug>`
(reopens the newest inbox card, mints only if there is none). 8 new tests, 4
driving the real widget under `egui_kittest`. NEXT = the Movement tab, and
§8.4 records what it will find: 18 effects but only three true entrances, and
**no exit effects at all**, so "how things go out" must be authored before it
can be offered.

**2026-07-29 — THUMBNAIL MODE (`docs/plans/slide-thumbnail-mode.md`):** the
seed-thumbnail crop, deferred twice since #136, is fixed at its cause. It was
never a surface bug: `Fluid::eval` ends in `.clamp(min, max)`, so below ~46
rem the type stops shrinking while the stage keeps going and a whole slide
crops inside a card. That floor is RIGHT for a real stage and wrong for a
picture of one. `Fluid::thumbnail(rect)` evaluates at a nominal 16:9 stage
and scales down uniformly; `NOMINAL_REM = 48` is DERIVED (the authored ramp
brings every type token to its min at ≈46.3 rem) and pinned by
`nothing_is_clamped_at_the_nominal_stage`. One factor suffices because
`document.rs` sources every size from `Fluid` and holds zero fixed
`cl::space::*`. `Stage::{Full,Thumbnail}` + `render_thumbnail` +
`DocSlide::thumbnail_ui`; `render_inner`'s step/now/t0/stage became a
`Playhead`. Carousel cards and filmstrip tiles switched over — tiles are real
miniatures now, not labelled plates, and keep their names. The seeds and
episode walls were never callers (they size to full width). Two shot-driven
fixes beyond the plan (§8.3): strip tiles resized to the authored
`ctl-thumb-*` after starving the carousel at 1100×700, and the carousel
caption capped at a third of the band with the sub-line DROPPED rather than
clipped when it will not fit. NEXT in this lane = the six card tabs.

**2026-07-29 — THE CONTAINER-LABELLED FILMSTRIP (`docs/plans/author-filmstrip.md`):**
the authoring surface now says what it is building INTO and shows what has
been started. `containers.rs` is the container census: Lesson · Guide ·
Episode · Deck, `slug()`/`label()` split, one `containers()` function reading
disk. GROUND TRUTH — only **two of the four are stored anywhere**: an episode
is `episode.json` under a show, a guide is `guide.json` in an inbox folder; a
deck is a way of PLAYING an episode, and "lesson" appears nowhere in the
codebase. Both gaps are REGISTERED and say why in plain words, with
`every_kind_is_backed_or_says_why` pinning the backed set to
`{episode, guide}` — when lesson storage lands, that test fails and forces
the wiring with it. The strip's head is drawn from the loaded container, and
its wording does not claim a placement that has not happened ("N started
here, not put in yet") — drafts are in `inbox/` until something copies them
in, which is the NEXT pass (`dpl_doc::place_slide`). The band measures its own
height and the carousel reserves exactly that. Seams `--author-container
<kind>` + `--author-draft <n>`. KNOWN: the band makes #136's seed-thumbnail
crop bite sooner at small windows (see §8.3) — the fix is a thumbnail mode in
`dpl-slides`, the next item.

**2026-07-29 — CI INCIDENT + MERGE-DISCIPLINE LAW (CLAUDE.md):** both
lanes went red in sequence because PRs merged before their green
finished: #139 broke `green-windows` (unconditional x11rb → `xcb.lib`
link death, every Windows run red until #142's linux-gating), then #142
merged mid-run and broke `green-linux` (clippy `manual_is_multiple_of`
in the linux-only probe — invisible to Windows-side clippy; fix = #144).
TWO sessions authored the same #144 fix concurrently (duplicates stood
down). New law in CLAUDE.md: **merge only on double-green** —
`gh pr checks <n> --watch` until BOTH lanes are COMPLETED SUCCESS; a
running check is a red check; `gh pr list` before authoring any fix.
Estate hygiene same pass: stale worktrees (`devpulselabs-l3` clean but
holding the `main` ref, `devpulselabs-chat-vision` gone) flagged for
removal — the ONE-checkout law was being violated by parallel-session
leftovers; release exe re-stamped at `c929b29`.

**2026-07-29 — C2S FOREIGN-TARGET SPIKE (`docs/plans/coach-c2s-foreign-target-spike.md` §9):**
`foreign-target-probe` resolves `gnome-calculator` button `7` via AT-SPI,
composes screen coords from X11 origin + `_GTK_FRAME_EXTENTS` (never
trusts SCREEN), paints ColdLight spotlight (`docs/proof/c2s-foreign-spotlight.png`),
and semantic-invokes via `Action.doAction("click")`. Linux-only gated so
Windows green does not compile atspi/x11rb. Coach NEXT = **C2a** ADR.

**2026-07-29 — P4 RAIL ACCORDION DEGRADE (`docs/plans/p4-rail-accordion-degrade.md`):**
rail body scrolls when the accordion exceeds the zone; opening a house
queues a reveal so the unfold is not clipped to an inverted unreachable
rect. Kittest `cramped_rail_keeps_the_open_house_reachable` + real-exe
proofs at 640×480 / 1280×800.

**2026-07-29 — P2B OVERLAY MOUSE-PASSTHROUGH (`docs/plans/p2b-overlay-passthrough.md`):**
`passthrough-probe` in `crates/desktop-spike` — witness under transparent
always-on-top overlay, XTEST click at center. Control pair on this X11
box (NVIDIA GB10 / Vulkan): passthrough ON → HIT; `--no-passthrough`
(opaque overlay) → no HIT. Evidence in `lesson-overlay-spike.md` §8.1.
Caveat: hollow transparent + passthrough-off is not a valid negative
control here.

**2026-07-29 — STRICT REFUSAL WITNESS (`docs/plans/strict-refusal-witness.md`):**
slash mint-site policy extracted as `resolve_operator_slash` in
`crates/app/src/chat.rs`. Unit matrix: agent-synthetic `/strict` →
`IntentRefused · synthetic-input` (intent domain / SSE envelope path),
mode stays behavioral; real escalate; evidence-locked behavioral refuse;
already-behavioral silent. Authoring note: Pass C overlay editor already
lives on the carousel (#136) — filmstrip + seed-thumbnail crop remain.

**2026-07-29 — THE AUTHORING RESET: LADDER, PLAIN WORDS, CAROUSEL
(PRs #135, #136; #128/#130 closed unmerged).** Three passes off one
operator red-pen on the authoring mock (artifact `a3db4aee`).

**DS3 — the ingredient ladder (#135, `docs/plans/ds3-ingredient-ladder.md`).**
`crates/slides/src/blocks/` held six ATOMS that DS2 had already typed
`ElementId`/`ElementSpec`/`elements()` — types said element, folder said
block — while the chunk tier had no home. Now three rungs, three folders,
one census each: `elements/` (git mv, 34 refs across 26 files, no shim per
the no-shim law), a NEW `blocks/` (`BlockId`/`BlockSpec`/`BlockDemo`/
`blocks()` with `made_of: &[ElementId]`), `templates/` untouched. **The
chunk tier ships EMPTY by design** — the promotion rule says a composite
earns the rung by recurring in ≥2 templates, and the census test asserts
the emptiness so promotion and registration land together. Library split
accordingly: `LibSection::Elements` added, `blocks_wall` → `elements_wall`
plus a new honest-empty `blocks_wall`. Structure audit needed NO amendment
— it is crate-graph only, and `dpl-slides` is named once as a SURFACE
crate. New harness seam **`--lib-section <slug>`**: `--focus` pins
TEMPLATES by design, so no wall but templates was provable on the exe.

**Plain words (same PR).** The chip row still read `seeds templates blocks
elements looks…` with "Eyebrow — Mono uppercase micro-label, the tracked
kicker over a title" under it. Operator: *"elements the eyebrow would be
confusing"*. Two vocabularies now split — `slug()` is the machine id
(harness arg, pilot key, never drawn), `label()` is what the operator
reads. Drawn: **Starters · Layouts · Combos · Pieces · Looks · Colors ·
Styles · Type · Moods**. Pieces: Eyebrow→**Small label**, Stat→**Big
number**, Badge→**Tag**, Bar→**Progress bar**, Divider→**Line**, Media→
**Picture**; `ElementCategory` type/mark/meter→Words/Tags/Levels. Every
blurb rewritten from the reader's side. Two tests hold the line:
`no_chip_speaks_engineering`, `no_piece_speaks_engineering`.

**The carousel (#136).** Operator: *"you still have never made the
carousel … easily able to select from premades"*. A shelf picker over ONE
carousel of large cards — Starters · Layouts · Combos · Pieces · Looks ·
Interactions — every card drawn LIVE from the real registries (seeds via
`DocSlide`, the Library's own paint path). Tap inserts through the SAME
mint path as the blank-card button, so a carousel card is the same real
document and publishes `CardAuthored` after the write. Arrows page and dim
at the end (never move); "keeps going" is the checkbox; empty shelves say
WHY. Fit-guarded on both axes — three layout bugs fixed and named in the
PR: clips must INTERSECT not replace (cards were painting over the
Inspector), height binds as well as width (captions pushed off a short
window), card count adapts 1–4 across.

**Passes B/C retired** (`docs/journal/2026-07-29-authoring-reset.md`).
They were built to the pre-red-pen mock: `enum Bucket {Blocks, Elements,
Effects, Templates}` — the code's own words drawn on screen, with Effects
since ruled into two tiers — over a "comp filmstrip" colliding with
`crates/comp`. Also stale against main. Closed unmerged, branches KEPT,
with a salvage map: **Pass C's overlay editor survives** — `editor_modal`
and `CardEditor` over the REAL `dpl_editor::Editor` with auto-save port
largely as-is; the palette/bucket/comp layer does not.

**Also:** `dpl-ledger`'s `overflow_increments_dropped_instead_of_blocking`
raced the writer (its "stuck consumer" was never stuck) and intermittently
blocked the pre-push hook. Replaced by two deterministic tests — a
`LedgerHandle::stalled()` seam pins the full-channel behaviour with no
thread at all, and a live-writer test asserts what holds either way
(never blocks; queued + dropped accounts for every row).

**KNOWN, outstanding:** a seed document drawn into a card-sized stage
crops its headline — the type ramp has a floor, so a full slide at
thumbnail scale overflows (visible in both #136 proof shots). The fix is a
thumbnail mode in the document renderer and belongs to `dpl-slides`.
**NEXT on this lane:** port the overlay editor per the salvage map, then
the filmstrip labelled by whichever container (lesson · guide · episode ·
deck) is loaded.

**2026-07-29 — SESSION OS RECONCILED WITH MILESTONE-LOOP (PR #133):**
one operating loop, two surfaces. Durable index = `docs/ops/` (SESSION +
resume + signals). Cursor: skill `session-os`, `.cursor/hooks.json`,
agents grounder→gap-closer→executor. Claude: skill `milestone-loop`,
`.claude/ledger/*` hooks (SessionStart / PreCompact / SessionEnd). Same
caps (≤3 plan / ≤2 web), journal at ~75–85%, disk beats chat memory.
Product NEXT after this lands: coach **live `/strict` refusal** (p6
Library `--lib-section` landed with DS3 #135). Journals:
`docs/journal/2026-07-28-session-os.md` +
`docs/journal/2026-07-29-session-os-reconcile.md`.

**2026-07-28 — DS3 INGREDIENT LADDER + LIBRARY SECTION HARNESS (PR #135):**
`elements/` earns its name; `blocks/` is the chunk tier; Library speaks
plain words; harness `--lib-section <slug>` proves every Library wall
(coach p6 CLOSED). Proofs under `docs/proof/ds3-library-*.png`.

**2026-07-28 — THE MILESTONE-LOOP LEDGER (agent workflow infra, PR #131):**
the agent's internal workflow is SELF-INSTRUMENTING. `.claude/ledger/` =
JSONL signal stream + emit.sh + SessionStart/PreCompact/SessionEnd hooks;
skill `milestone-loop`. Composes with Session OS (`docs/ops/`) — not a
second competing product. Journal:
`docs/journal/2026-07-28-milestone-loop-ledger.md`.

**2026-07-28 — COACH / LEDGER LANE RESUME:** narrative close-out
`docs/journal/2026-07-28-determinism-and-ledger.md`; ordered remaining
work `docs/journal/2026-07-28-remaining-work.md` (p6 CLOSED via #135;
NEXT = live `/strict` refusal → overlay passthrough → rail degrade →
C2s → C2a/C2b → strict pinning → DuckDB ADR → classification). Ops index:
`docs/ops/SESSION.md`.

**2026-07-28 — THE WORKFLOW WIRE CONTRACT + DOCTRINE (PR #129, squash
`c803e51`; plan `docs/plans/workflow-doctrine-and-schemas.md`):**
workflows are now schema-governed for scale — `schemas/
workflow.v1.schema.json` + `storyboard.v1.schema.json` (strict JSON
Schema 2020-12, `v` const-pinned, kebab ids, jail-safe refs; vN files
are append-only, v2 lands BESIDE v1) PINNED BY TEST: 4 `schema_pin`
tests in `dpl-workflows` (dev-dep `jsonschema`, no default features,
outside the ceiling audit) validate the real registered types AND prove
refusal (unknown kinds/fields, non-kebab ids, future `v`, jailbreak
refs). `docs/design/workflow-doctrine.md` = 11 laws ratifying WF1's
patterns + THE WORKFLOW-PR CHECKLIST every workflow PR copies into its
plan — the "all workflow PRs similar and consistent" gate. Agents and
platform tools author/validate workflow documents schema-first, the
same convergence seam the signal envelope rides.

**2026-07-28 — CI MOVED TO THE ESTATE, RUNNERS BECAME CATTLE (PRs
#122/#125/#127; runbook `docs/runbook/ci-runners.md`):** both green
lanes run on the dedicated server's guests — `devpulse-runner-win-01`
(Proxmox VM 119, 32 GB, per-repo root `C:\actions\runner-devpulselabs`,
SYSTEM service, toolchain git 2.55/rust 1.92/MSVC 14.44/LLVM 18.1.8) +
`devpulse-runner-lnx-01` (LXC 121, full dep set) + the ARM64 clyffy-lab
runner; the laptop is UNREGISTERED as a runner. Idempotent bootstraps
(`win-runner-bootstrap.ps1` with `-VerifyOnly`, `linux-runner-bootstrap
.sh --verify-only`) are committed and were PROVEN live — the Windows one
re-provisioned its own runner through two real failure modes (stale
local config; the clock-skew incident that got a registration deleted —
VM clocks must stay NTP-true). Known-good snapshots
`known-good-2026-07-28` exist for 119 + 121; proof = green runs on both
lanes incl. the post-migration main dispatch. KNOWN: single Windows
runner = queue bottleneck under multi-lane load (second runner/VM is an
ask-first lever); a runner-health watchdog script is drafted but
unlanded (logging gap named by the operator — its own small pass).
NOTE: this session's exe-law was satisfied at `d367390` (12:00 rebuild
+ live witness); everything merged after by this session is docs/CI/
schemas + dev-dep only — no shipped-code drift.

**2026-07-28 — L4: BLEND · MATTE · LUT — THE ESTATE UNLOCK (PR #123,
squash `867bff3`; plan `docs/plans/l4-blend-matte-lut.md`):** the fourth
AE-independence rung, sized by the 30-pack/~60 GB estate census (censused
+ ffprobe-verified this session; triage in the census section of the L4
plan §1-2): most non-template media is NOT straight alpha — leaks/dust
want SCREEN/ADD, matte packs are grayscale luma wipes, the Bundle ships
860 `.cube` LUTs. Pieces: (1) `dpl-comp` `Blend {Normal|Screen|Add}` +
`matte`/`lut` refs on Layer (additive v1, empty refs = named faults);
(2) `engine-media` — blended legs pad to the FULL frame on BLACK (the
screen/add neutral) at the layer's offset and run `blend=all_mode` on
gbrp with the bounded window; mattes join as inputs and `alphamerge`
(a blended leg SKIPS its matte — blend drops alpha); `lut3d` grades base
+ overlay legs via `filter_path` (Windows colon escaping, unit-tested);
(3) preview — `PipeGrade` on ClipPlayer: LUT + matte preview EXACTLY
(one pipe; matte as a second input at the same seek), lanes reopen on
grade change; blends approximate under a NAMED badge (`screen ·
export-exact`); (4) inspector GRADE rows — blend chips, matte picker
(matte-tagged first) + the operator-ratified AUTO-PAIR suggestion
(shared leading word + exactly one match → tappable "try: Brush Matte
01", offered never applied), `.cube` picker on EVERY layer; (5) enricher:
`LUTs/` buckets tag `lut`, bare `X Matte NN` stems tag `matte`;
(6) `--l4-drill`. Mechanisms proven LIVE on ffmpeg 8.1.2 against real
estate files BEFORE the plan (cinepunch 8K leak · RTFX Brush Matte ·
Bundle cube); the real-exe drill reproduced all three in one render
(graded base + screened leak + brush-carved tinted overlay, frame-
verified). Gate `cargo green ✔` server1; green-linux ✔ on the new
`devpulse-runner-lnx-01` (post-provision); green-windows on
`devpulse-runner-win-01` died mid-clippy twice (runner drop, NO step
failure — suspect VM resources during workspace clippy; watch the next
PR), operator called the merge on the server1 gate. CLOSE-OUT: main's
exe built from `867bff3` and placed in THE checkout (mtime 2026-07-28
17:13; the checkout itself was held on a concurrent session's branch —
built in the `devpulselabs-l3` worktree lane). KNOWN FOLLOW-UPS: grade
rows sit below the Info scroll fold at short window heights (scrollable,
no overlap — a taller-window or scrolled-shot proof nicety remains);
the real-RTFX pin+ingest drill remains operator-driven (D: is back;
pin `2. Prerendered animations` + `6. Sound FX`); template-variants rung
awaits the operator's Bodymovin exports (mogrt title · Toko slide ·
cyberpunk HUD → drop the .json files in).

**2026-07-28 — SIGNAL LEDGER PHASE 2 (`docs/plans/signal-ledger-phase2.md`):**
`dpl-ledger` writes hive-partitioned Parquet under `.userdata/analytics/`
(pure-Rust `parquet` + `snap`, ADR-009 clean). Off by default; `--ledger`
or `DPL_LEDGER=1` starts the bounded writer thread. Schema v1 columns:
`seq · ms · wall_ms · domain · type · body · class · dropped`. Stamped
domains only (capture/jobs/providers/build/coach/intent). Files are the
truth; DuckDB stays Phase 3.

**2026-07-28 — CHAT ASSIST MODE: ONE TOGGLE, ONE BLANKET ACKNOWLEDGEMENT
(PR #126):** the consent gate the assist/bake-off/preference arc stands
on (`docs/plans/chat-assist-mode.md`; operator: "a clear simple
acknowledgement that will blanket for all providers"). Pieces: (1) the
⚡ Assist chip beside the route-context chip in the chat composer row —
NEVER provider- or capability-gated; node `chat.assist.toggle`
(value on/off); (2) typed intents `ToggleAssist { enable }` +
`ResolveAssistAcknowledgement { accepted }` across the window→host
boundary — the chip never flips itself, the HOST projects policy truth
via `set_assist`; (3) the ONE acknowledgement: a host-authored Approval
overlay, TWO buttons ("Turn on assist — all providers" / "Not now"), no
"always" (approving IS the durable answer), copy names the blanket
across all providers current and future; asked once EVER
(`acknowledged` persists); (4) `AssistPolicy { enabled, acknowledged }`
→ `assist-policy.json` in userdata (imagine-policy shape; authored
fixtures never write disk; disabling is instant, no ceremony);
(5) proofs: window tests 22/22 (3 new); scenario `chat-assist-toggle`
(authored) + the real-runtime pair `chat-assist-enables`/`-persists` on
scratch rings proving write + fresh-launch restore with no re-ask; six
existing chat scenarios stay green; `proof-chat-assist-ack.png` via the
new chat-native-proof `assist` state. CI note: green-linux's first run
died on LXC 121 missing `libgbm-dev` — installed via `pct exec` from
server1, lane green after (memory updated). NEXT on this arc: the first
bake-off interaction (choice-with-previews) + the estate-backed
preference record it feeds (assist ON is now the consent that permits
them).

**2026-07-28 — AUTHORING PASS A: THE SIGNAL SPINE (`feat/authoring-a-signals`;
charter `docs/plans/authoring-charter.md` + build plan `authoring-shell.md`,
both on PR #119; resume journal `docs/journal/2026-07-28-authoring-shell-handoff.md`):**
the first build pass of the authoring surface, built signal-first. THREE
additive pieces: (1) `dpl_fabric::BuildEvent::CardAuthored{show,episode,slide}`
— additive, serde-free; the seam maps it to the `cardAuthored` envelope
(`build_json`, pinned by test) and `build.card-authored` joins the coach
`COARSE` catalog (drift-swept, so an AI/coach touchpoint can wait on an authored
card). (2) The new **`author` subroute** on the Workbench hub — a ColdLight
surface: an intent bar (charter §3b, captured-first) + a "Seed a draft card"
action that mints a REAL schema-valid `slide.json` draft into `content/inbox/`
(carrying the typed intent as a block, no-clobber probe) and publishes
`CardAuthored` ONLY after the write succeeds (emit-with-mutation; anti-loop —
the surface publishes `build.*`, never subscribes). (3) The LIVE ROUND proven
in-gate: a `crates/seam` `#[tokio::test]` opens the real `/signals` SSE stream,
publishes `CardAuthored`, and reads the stamped envelope back off the wire
(bounded, fail-fast). Gate: `cargo green ✔` (fmt · spawn+structure audits ·
clippy -D · tests · release). Real-exe proof: the surface renders at 1280×800
(`docs/proof/authoring-a-surface-1280x800.png`) — breadcrumb Workbench › Author,
Inspector route `workbench/author`, live SIGNALS feed. Tofu glyph fixed
(`＋`→`+`, R1s font-safe law). Scope deferred per plan: `EntityRegistered`
lands with the pass that actually registers entities (Pass D–F, no fabricated
fact); NEXT = **Pass B** (four-bucket infinite gallery + filmstrip over the real
registries). NOTE: built in a worktree off `main`; charter #119 + DS2 #114
merges remain the operator's calls.

**2026-07-28 — DS2 ELEMENT LIBRARY (`feat/ds2-element-library`,
`docs/plans/ds2-element-library.md`, design-system build order §7.2):** the
element tier brought under the **Census law**. New
`crates/slides/src/blocks/census.rs` — `ElementSpec {id, label, category,
cost_tier, blurb}` + the `ElementDemo` trait + `elements()`, mirroring
`registry::demos()` for templates, with the gallery-gate test painting all 6
elements (eyebrow · stat · badge · bar · divider · media) at entrance AND rest
without panic. `ElementCategory` (type/mark/meter/divider/media/**container**/
**control**; the last two honestly EMPTY until promotion / recipes) is the
Library SHELF, not a style; `CostTier::{Baseline,Enhanced,Max}` is DECLARED now
(all elements Baseline — none paints an animated wash), to be HONORED by the
`Caps→Tier` fidelity probe in a later step (§5). `workbench::library::
blocks_wall` rewritten **census-driven** — the hand-list DIED (it had gone
stale: `bar`/`divider` shipped but never showed); a new element now appears the
day it registers, and nothing enumerates elements by hand (Census-law
compliance). Elements stay ergonomic free fns; a stateless demo wraps each,
centralized in the census module (zero element files touched). OUT-OF-SCOPE,
each its own build-order step: treatments/`ResolvedStyle` threading (§3),
recipes (§4), the probe honoring `cost_tier` (§5), template migration (§6).
Gate: full `cargo green` GREEN on the Linux worktree (fmt · spawn+structure
audits · clippy -D warnings · workspace tests incl. the 2 new census tests ·
release build rebuilt `target/release/devpulselabs`). CLOSE-OUT (operator,
Windows checkout): rebuild `devpulselabs.exe` + mtime, screenshot the Library
element wall at a REAL window size showing all 6 shelved by category (`bar`/
`divider` now present), land the PR.

**2026-07-28 — JOBS-BOARD MIGRATION: ONE JOBBOARD, THE THREE POLLED STATES
COLLAPSE (PR #120, squash `d367390`; plan `docs/plans/jobs-board-migration.md`;
journal NEXT SERIES item 2 CLOSED):** THE JOBBOARD lives inside the Fabric —
`publish_job` folds every `Queued/Done/Failed` into a board BEFORE
broadcasting; each row carries a latest-wins `progress` counter (doctrine
law 7's PROGRESS class, centralized; NO Progress broadcast variant, wire
shape unchanged). Accessors `job/jobs/jobs_running/ack_job`; running rows
never evict, unacked resolved rows cap at 64. Composer's
`RenderState`/`IngestState`/`ExportState` + `ingest_done` are GONE — three
`JobId` handles read the board (pills, keep flows, repaint keep-alives);
composer's fabric is now ALWAYS-present (private `Fabric::new()` default,
`wire_fabric` swaps in the app's — drills/tests keep a working board).
Seam's `RenderStatus` mirror deleted — `renders` maps rel → `JobId`,
`GET /render` answers from the board (wire unchanged). Shell gains the
`jobs` status cell: `working · <label> [· n]` from EVERY route, ticking
at 300ms while jobs run. Doctrine jobs law amended to record the
migration. Gate: `cargo green ✔` server1 + pre-push + BOTH CI lanes
(linux 2m50s, windows 24m13s); fabric 16 · shell 19 · seam 20 · composer
52 tests. WITNESSED LIVE on the rebuilt exe (build `d367390`): seam
`POST render` → `GET /render` answered `running` → `done` off the board
(real 180s stitch, render.mp4 landed), and the status bar showed
`working · render inbox/comp-jobs` from the ViziWiz route mid-render —
`docs/proof/jobs-board-working-cell-2026-07-28.png`. NOTE for drills:
seam JSON documents must be BOM-free (a PS5.1 `Set-Content -Encoding
utf8` BOM makes the comp loader refuse the doc). CLOSE-OUT:
`target\release\devpulselabs.exe` rebuilt HERE (mtime 2026-07-28 12:00).

**2026-07-28 — L3: ALPHA OVERLAY — FX ELEMENTS COMPOSITE ON THE TIMELINE
(PR #110, squash `eac44d2`; plan `docs/plans/l3-alpha-overlay.md`,
deep-planned + operator-ratified):** the third AE-independence rung — the
operator's videohive packs become USABLE: an element sits ON TOP of
footage with correct straight alpha, positioned/scaled/tinted, live in
the preview AND in the export. Pieces: (1) `dpl-comp` Layer grows
`x`/`y` (center-px) · `scale` (1.0 = aspect-fit) · `tint [r,g,b,a]`
(alpha = opacity) — additive, pre-L3 comps byte-identical; (2)
`engine-media` `comp_args`/`render_comp` — ONE ffmpeg pass: proven
concat legs → `[base]`, overlays chain `trim → setpts+at/TB → scale →
colorchannelmixer → overlay enable='between' eof_action=pass`;
`CompBase::Black` gap legs keep windows timeline-true; PNG-seq inputs
(Lottie) + held stills; every arg form proven LIVE on ffmpeg 8.1.2
against the real RTFX pack first; (3) composer layered preview —
per-overlay lanes, painted back-to-front with tint at both viewer
sites; caps MACHINE-PROBED (sysinfo RAM+cores; 64GB rig 6+2, this
laptop 3+1), topmost wins, skips named, export never capped; Lottie
layers now export via helper temp sequences (L2's honesty note CLOSED);
(4) editor UX (ColdLight): FX timeline lane, `fx` corner on footage
rows (overlay at the playhead), OVERLAY inspector (size/x·y/center
snap/tone-pool tint swatches/opacity, noCT advisory), tag-filter chips
on FOOTAGE; (5) `rtfx.rs` — the 711-file census grammar → `Record.tags`
at ingest for every kind + the pack's own `(Footage)/previews` jpg as
poster (an alpha element's first frame is blank); (6) **BUG FIXED**:
pack ingest wrote the ADR-014-RETIRED legacy `artifacts.db` (invisible
library) — `ingest_copy` now takes the caller's index, app passes
`open_default()`; (7) `--fx-drill <clip-rel> <fx-rel>` synchronous
headless proof. Gate: `cargo green ✔` server1 + BOTH CI lanes (linux
3m02s, windows 23m36s). Proof: drill render ffprobed (1920×1080@30,
comp.json exactly the ratified shape) + frame extracts (tinted overlay
at t=1.5, absent at t=0.5) + editor shots at 1280×800 AND 1100×700 (an
Info-pane overflow the shots caught was fixed + re-proven). CLOSE-OUT:
`target\release\devpulselabs.exe` rebuilt HERE (mtime 2026-07-28
10:30), `dpl-lottie-render.exe` beside it. KNOWN FOLLOW-UPS: the D:
media drive went OFFLINE mid-session — re-run the ingest drill on the
real RTFX pack when it returns (pin `2. Prerendered animations` +
`6. Sound FX`, the ratified roots); the agent-seam render is still
concat-only (overlay-aware seam = the agent-placement rung); browse
window has no tag chips yet (FOOTAGE palette does). Build ran in the
`devpulselabs-l3` worktree (concurrent-session lane).

**2026-07-28 — DETERMINISM MODES BEHIND THE INTENT GATE
(`docs/plans/determinism-intent-gate.md`):** `dpl-intent`'s first consumer.
`DeterminismGate` owns `Behavioral | Strict` with `escalate(OperatorIntent)`,
`deescalate`, and `force_evidence` (evidence outranks the operator; cannot
be talked down). The INTENT fabric domain (doctrine law 11) carries
`DeterminismChanged` / `IntentRefused` through the seam SSE and the shell
activity feed. Two mint sites only: chat composer slash at submit
(`/strict` · `/behavioral`, whole-buffer exact, draft cleared before it
becomes a message) and the inspector DETERMINISM card (arm → confirm).
Both refuse synthetic (agent-injected) activation and publish the refusal.
`--screenshot` boots force evidence. Pinning (clock/RNG/adapters) is NOT
this pass — the switch and its guards land first so authorization is
settled law when replay arrives.

**RESUME POINT (2026-07-28, Fable session):** journal + next-series
notes: `docs/journal/2026-07-28-fable-session.md` — read before
starting guide-UX-redlines / jobs-migration / R2c work. (The parallel
lane's phase notes: `docs/journal/2026-07-28-next-phase-notes.md`.)

**2026-07-28 — PHASE 1.5 VERIFIED LIVE (three witnesses, one session):**
one devtools run (`devpulse-cloud`/`library-wall` @1280×832, build `c333b69`)
proved the coach domain end to end — the seam SSE carried the pinned coach
envelopes with strictly increasing `seq`, the pipe's `trace.jsonl` carried
the identical lifecycle (the tap steals nothing, live), and the SIGNALS card
rendered all six feed lines (`docs/proof/coach-domain-live-2026-07-28.png`).
`Satisfied` fired on the MOUNTING pass for an already-true postcondition
(`activated.type=satisfied`); an agent flow click produced
`triggered · press · actor=agent`; the C1.5f typed rail split witnessed live
(`value=active` + `state.expanded=false`). NUANCE FOR THE LEDGER: shell
events are unstamped by design — cross-domain `seq` ordering holds among
stamped domains only; shell facts order by stream position. Full findings:
`docs/journal/2026-07-28-next-phase-notes.md` §A.

**2026-07-28 — PHASE 1.5: THE COACH SPEAKS ON THE RECORD, AND INTENT IS A TYPE
(`docs/plans/signal-ledger-and-lesson-foundation.md` §9 + Phase 1.5):** the
2026-07-28 rulings are ratified into the plan — the closed loop (analytics
recommends, recommendation triggers the guided workflow), determinism as a
dynamic named mode (behavioral default for measurement, strict escalated for
evidence), the PROVENANCE LAW (escalation is authorized by the provenance of
the keystroke, never by a pattern in a string; model output is never re-parsed
for commands), redaction outranks reproducibility, and classification as three
separated layers (CUI/Presidio base vocabulary storing the CLASS never the
value · per-app tuning packs authored offline · HMAC pseudonymization).
LANDED AS CODE: (1) the **COACH fabric domain** (doctrine law 10) —
`CoachEvent { Armed · Mounted · Triggered · Satisfied · Unmounted }` stamped
like every fact domain; the publisher is a devtools-gated TAP on the plugin's
event record path (a witness, never a consumer — the devtools pipe still
drains its own copy, pinned by test), bridged in `crates/app/src/
coach_fabric.rs`; the seam's `/signals` envelope and the shell activity feed
both carry it ("step satisfied · lesson-1/step-2" is now an operator-visible
line). ANTI-LOOP LAW: `coach_signals.rs` never subscribes to the coach domain
— a touchpoint waiting on a coach event would self-trigger. (2) **`dpl-intent`**
— `OperatorIntent`, a single-use proof with no Clone/Default/serde and a
dependency-free manifest pinned by test, whose two constructors
(`ui_acknowledged` · `slash_command`) make `rg 'OperatorIntent::'` the
complete audit surface; first consumer is the determinism-mode switch (next
pass). The shell subscribes its coach ear at construction (before the pilot
attaches), so no publish precedes the subscription.

**2026-07-28 — ARCHITECTURE DOCTRINE + THE STRUCTURE AUDIT (PR #107):**
the code-structure law, written and armed (operator ask: strict, organized,
methodical authoring aligned to house patterns + industry practice).
`docs/design/architecture.md` = the layer law authored from the REAL
graph (layer stack, exact dep ceilings for 7 pure crates, standing
exceptions recorded with their why — dpl-doc→coldlight token edge, the
FFI leaf's ratified unsafe override, the desktop-spike orphan — plus the
census/document/ADR-009 laws and a new-crate checklist). `cargo xtask
structure` = the doctrine as FITNESS FUNCTIONS inside `cargo green`
(after the spawn audit): ceiling exact-sets · egui-free census (35) · no
lateral hub deps · `-sys` only in dpl-lottie-render · manifest hygiene
(63 members) · orphan folders (named allowlist) · harness-flag census
(app ⇔ CLAUDE.md, both directions, 33 flags). Amending doc or audit table
alone fails the gate — they move in the same commit (CLAUDE.md law
added). First light caught real drift: dpl-chat-imagine (no lints/
publish, a #98 miss) + dpl-engine-capture (no publish pin) — fixed
in-PR. Negative proofs in plan §8 (`architecture-doctrine.md`).

**2026-07-28 — THE WINDOWS LANE UNBLOCKED (#106) AND THE PR QUEUE ABSORBED:**
every open PR was stuck on `green-windows`. Three stacked causes, each proven:
(1) the runner was OFFLINE (the ONLOGON task never fired after reboot) — hours
of queued runs; (2) `LNK1318: Unexpected PDB error; LIMIT (12)` — the MSVC PDB
server overwhelmed linking the big test exes (`dpl-chat-grok` every time) with
debug info on the 16GB laptop; (3) full-parallel rebuilds starved the runner
heartbeat into "lost communication" cancellations. THE FIX (#106, green.yml
`green-windows` env only): `CARGO_PROFILE_DEV_DEBUG=0` +
`CARGO_PROFILE_TEST_DEBUG=0` (CI never reads PDBs; no PDB work = no LNK1318),
`CARGO_BUILD_JOBS=8`, `timeout-minutes` 30→45 for the one full rebuild per
cache. TWO LAWS LEARNED THE HARD WAY: the Windows runner does NOT inject
`.env`-file or user-registry env into jobs (both tried, link lines still
carried `/DEBUG`) — the workflow file is the ONLY reliable env carrier, and it
rides each PR's merge ref (existing runs re-run the OLD merge commit; fresh
runs need a rebase/synchronize); and **a CONFLICTING PR gets NO Actions runs
at all** (no test-merge ref → close/reopen and empty commits do nothing) —
#103 sat silent until main was merged in. ABSORBED (squash): deps checkout#77
upload-artifact#78 tungstenite#81 toml#82 sha2#83 ravif#85 kamadak-exif#86,
then coach #103 (STATE.md ledger conflict resolved keep-both). PARKED AS REAL
MIGRATIONS: fjall 2→3 (#80, `crates/estate` Keyspace API) and sysinfo
0.33→0.39 (#84). Runner ops: prefer a service install over the logon task
(the task also can't be killed cross-elevation when someone runs it as admin).
NEXT CONSIDERED: a Windows x64 VM on the wardenclyffe Proxmox estate takes the
`dpl-win` label off this laptop entirely (blocked on estate reachability —
`\\10.0.0.116` was down at session end).

**2026-07-27 — C1.5f: THE COACH HAD NOTHING TO POINT AT
(`docs/plans/operator-coach-a2ui.md` §14, PROTOCOL_VERSION 2.5 → **2.6**):**
a live snapshot of `workbench/library` returned **6 nodes** — five rail tiles
and the chat button. `hubs/workbench` had ZERO instrumentation and did not even
depend on `egui-pilot-ai`. Now **14 nodes**, eight of them Library section tabs.
(1) **Two facts stopped sharing one string** — the rail reported `active+open`,
which no exact-match condition could question; it now reports `value=active`
plus typed `expanded`. The fields always existed on `NodeState`; the three
BUILDERS (`expanded`/`selected`/`checked`) did not, which is the whole reason
the codebase encodes state as prose in `.value()`. (2) **`CoachCondition::
NodeFlag`** over `NodeFlagKind` makes typed state askable, so splitting the
value did not make fold state unreachable. **An unreported flag is neither true
nor false** — `read` returns `Option<bool>`, and silence satisfies neither
`on:true` nor `on:false`. (3) Sub-rows keep their value string (a committed
fixture depends on it) and gain typed `selected` additively. **A first reading
blamed scroll — measurement refuted it**: the `library` fixture passes
`--focus title` (solo mode, no wall) and the new `library-wall` fixture lands on
the default **seeds** section, because the Library renders exactly ONE section
wall at a time by design. Both report ZERO clipped and ZERO hidden nodes, so
there is no measured case of a node the coach cannot reach; `ScrollIntoView`
stays unimplemented but unneeded so far. STILL OPEN: no harness flag selects a
Library section without also soloing a template, which is what actually blocks
witnessing the templates wall and Episodes. Also: the backdrop asset test no
longer fails under a shared `CARGO_TARGET_DIR` (it anchored to the exe, not the
source tree).

**2026-07-27 — ANALYTICS FOUNDATION RATIFIED WITHOUT THE DEPENDENCY
(`docs/plans/signal-ledger-and-lesson-foundation.md`):** ADR-014 already names
`.userdata\analytics.duckdb`, fenced everywhere as "own C-dep ADR, never
smuggled" — and ADR-009's `deny.toml` BANS ffmpeg crates on the rule that C
libraries are subprocesses here. Reconciled: **DuckDB reads Parquet and JSONL in
place**, with predicate/projection pushdown and no ingest, so the substrate is a
FILE FORMAT AND A SCHEMA, not a database. The Signal Ledger writes the stamped
fabric stream (doctrine law 5 calls it a flight recorder; today it is thrown
away) to date/domain-partitioned segments. Operator ratified **Parquet from the
start** — verified pure Rust: `parquet` with `--no-default-features --features
snap` resolves 31 crates and ZERO `-sys`/cc/cmake deps. The DuckDB C-dep ADR is
deferred to Phase 3 so it argues from measured volume. Phase order: 1 (coverage)
→ 2 (ledger) → 3 (ADR).

**2026-07-27 — C1.5d/C1.5e: A COACH STEP COMPLETES ON STATE, NOT ON INPUT
(`docs/plans/operator-coach-a2ui.md` §12–13, PROTOCOL_VERSION 2.4 → **2.5**):**
two gaps found by walking a real three-step lesson, both fixed and proven on the
real exe. (1) **A mount is a subscription, not a resolution** — any node-id
target that has not appeared now ARMS whatever its trigger, because "click the
thing the last step revealed" is the shape of every lesson; only a name/role
locator is refused. (2) **Completion is a level, not an edge** — measured: a
touchpoint waiting on `shell.route-changed/workbench/library` while already on
that route NEVER fires (edge triggers are documented as edges), so every step
whose goal is already met hangs forever, which kills resumed lessons and the
"from any starting point" premise. New `Touchpoint.until` carries a narrow
`CoachCondition` (`node-value`/`node-visible`/`node-gone`) read EVERY pass, and
`CoachEvent::Satisfied` is distinct from `Triggered`: the step is done because
the app REACHED THE STATE. It lives in `egui-pilot-ai` (not the devtools-gated
runtime) so shipped lessons need no pilot pipe. Corollary discovered while
proving it: **a trigger must not end a step that has a postcondition** — the
press fired on the click that merely OPENED the rail accordion and `once` tore
the step down before navigation; `push_fired` now skips auto-unmount when
`until` is set (the press is still reported — a hint, not the truth). Also:
`pilot_start` takes `window: "<w>x<h>"`, and a handshake timeout now NAMES the
usual cause (a target built without `--features eguipilotai-devtools` is
healthy and completely mute). **Two blockers for the first real lesson, both
open:** a full snapshot on `workbench/library` returns **SIX nodes** (five hub
tiles + chat window — Library content carries no pilot ids, accordion sub-rows
do not exist until their tile is clicked), and rail values pack route + fold
state into one string (`active+open`), so node state is not cleanly queryable —
openness belongs in `NodeState`. Instrumentation coverage, not coach
capability, is now the binding constraint. Spikes filed:
`docs/plans/lesson-overlay-spike.md` (X11 + wgpu transparency PROVEN by probe;
`mouse_passthrough` still unproven) and
`docs/plans/coach-c2s-foreign-target-spike.md`.

**2026-07-27 — WF1: THE WORKFLOW ENGINE + SLOTTED STORYBOARD (PR #105):**
the first pass of the ratified Workflows program (`docs/plans/
wiredfront-workflows-program.md`; plan `wf1-workflow-engine.md` §3a = the
slotted-storyboard + two-surface refinement, evidence in §8). Pieces:
(1) pure crate **`dpl-workflows`** (deps exactly serde/serde_json/
dpl-comp/dpl-doc) — a workflow is a TYPED DOCUMENT (D3): `WorkflowDoc` v1
(linear = the degenerate case of WF2's graph; stage ids from day one;
`surface` declares the two-libraries side), the slotted `StoryboardDoc`
(role + agent-readable brief + typed fill; "slots remaining" = derived
query), census registry (D6), named-issue validation + jailed refs,
crash-safe board save, executor + honest `DryRunner` (12 tests);
(2) composer **Workflows tab** (sixth `Tab`; `workflow_tabs`→`tab_strip`):
census cards, Run chip, per-stage verdict chips, slots as EguiPilot nodes
(`wiredfront.storyboard.slot/<id>`, value = fill, description = brief);
`WfRunner` materializes stages under no-clobber `inbox/workflow-<id>/`;
per-stage `jobs.*` facts + ONE `InboxChanged`; generation NEVER spends
(ADR-011 presence-check, honest skip); an assembled comp opens in the
Editor and rides the existing render job; (3) `--run-workflow <id>`
harness seam + `wiredfront-workflows` fixture/scenario (scratch
CONTENT_ROOT), proof shots at 1280x800 + 1600x1000; (4) **poppler
staged**: pdftoppm 26.02.0 as a Media-family TREE in BOTH depots (xtask
tree gate generalized, per-id `-v` probe, verify-all-staged);
`rasterize_pdf` consumes the injected depot payload, PATH fallback for
the Linux lane; (5) T0 record hygiene (gate table rows 2/3/4/5/10 DONE
with W1 `067646d`/W1.2 `9a95fc7`/W2 `60db7be`; ladder rungs 1–2 retired;
CLAUDE.md harness flags ground-truthed; port-capture skill records the
gutted reference repo). **KNOWN BROKEN (pre-existing on main, verified
against a main-built devtools exe):** EguiPilotAI `shell-navigation`
(rail flyout `Test Lab` locator dead) and `chat-peer-window`
(`chat.context.route` ≠ "wiredfront") both fail — next-session repair;
`chat-grok-signed-out-readiness` stays the documented signed-out gate.
NEXT (§3a): workbench/workflows mount + teleprompter-script stage; then
T1a (element render leg) per the ratified tier order.

**2026-07-27 — DESIGN-SYSTEM ARCHITECTURE RATIFIED (`docs/design/
design-system.md`):** the composable pivot. THE MODEL: *a template is a layout
of ELEMENTS; each element wears a TREATMENT (semantic | composite); any
interaction is a RECIPE; all of it paints through one 2D pipeline under a
FIDELITY budget.* Nothing hand-painted that could be composed. Layers map 1:1
to industry practice (design tokens → primitives/atoms → components → variant
recipes → progressive-enhancement graphics tiers). Treatment MECHANICS stay in
`doctrine.md` (compose→ResolvedStyle→paint, additive/override axes, the
no-blur/hard-shadow+back-glow law = the cheap-2D anchor); the spec adds the
layers around it: an organized ELEMENT library (today's `blocks/`, elevated);
SEMANTIC treatments (intent aliases `primary/danger/…` bound to composites —
the Material core→semantic split) vs COMPOSITE (explicit base+stack); RECIPES
(interaction as reusable pure state machines — `advance/toggle/flip/match/poll`
lifted out of the templates, wired to the C-series touchpoint triggers); and a
FIDELITY budget where **Tier 0 = old i5 is the DEFAULT (not degraded)** and
better hardware ONLY ADDS layers (2D forever, no 3D; each treatment/recipe
declares a cost tier + authored cheap fallback; one boot `Caps→Tier` probe
extends the sysinfo/engine-ae precedent). RATIFIED DECISIONS: element = UI
primitive (composer pack `ELEMENTS → pack assets`, copy-only later); treatments
in `tokens/treatments.json` + a `dpl-slides` recipe registry; **BIG-BANG**
catalog re-author once the system stands. BUILD ORDER (§7): spec ✅ → element
library → treatments → recipes → fidelity probe → composed renderer + big-bang
migration. The W-series hand-painted templates (incl. the 2026-07-27 diverse
catalog, #97) are the stopgap that step §7.6 flips over.

**2026-07-27 — L2: NATIVE LOTTIE ON THE TIMELINE (PR #99, merged to `main`
`deffec4`):** the second rung of the AE-independence ladder — Bodymovin
`.json`/`.lottie` imports become first-class timeline layers rendered
natively, no After Effects. `docs/plans/l2-lottie-timeline.md`. Pieces:
(1) `dpl_comp::Source::Lottie { rel }` (serde `kind:"lottie"`, validate +
round-trip test); (2) `dpl-lottie` — a pure-Rust, `unsafe`-free façade
(`Meta`/`lottie_meta` reads w·h·fps·in·out from raw JSON or `.lottie` zip;
`is_bodymovin_json` import sniff; `Player::open`/`frame_at`/`export_sequence`)
that only SPAWNS the helper; (3) `dpl-lottie-render` — the ONE FFI leaf
(ADR-009), a ThorVG subprocess with `frame`/`export`/`serve` modes,
`ABGR8888S` straight alpha for correct egui/PNG compositing, `tvg_paint_ref`
ownership; (4) `dpl-composer` `LottiePlayer` live preview (cached texture,
aspect-fit at both viewer + annotate paint sites), `add_lottie`,
tick_playback Lottie arm; (5) `import.rs` `.json`/`.lottie` → `"lottie"`
kind (Bodymovin-sniffed); (6) `--lottie-import <file>` harness flag +
`wiredfront.import_lottie` delegate. Gate: `cargo green ✔` on **server1**
(the 128GB/88-core Debian Linux build box, `ssh.clyffy.ai` — the 16GB laptop
OOMs the full gate; server1 is now the standing Linux gate box), run on the
branch AND on the merge commit with current `main`. `dpl-comp` (9) +
`dpl-lottie` (4) unit tests pass. Merge into `main` (post-#98): 3 additive
"keep both" conflicts resolved (`--lottie-import` alongside W11's
`--make-slide`; Cargo.lock reconciled). CLOSE-OUT: `target\release\
devpulselabs.exe` rebuilt HERE on the laptop (mtime 2026-07-27 15:56,
ThorVG-free — the helper is spawned, never linked, so the main exe builds
in 16GB). **KNOWN GAP:** `dpl-lottie-render.exe` (the ThorVG helper) is NOT
yet built on Windows — first-time ThorVG C++ build — so Lottie import/preview
won't RENDER until the helper sits beside the exe (or on PATH /
`DPL_LOTTIE_RENDER`). Build it before the Lottie drill. NEXT: L3 (Lottie
export via helper→ffmpeg; slot recolor/retext).


**2026-07-27 — CATALOG: A DIVERSE LIBRARY (branch
`feat/catalog-diverse-library`, PR):** the slide catalog grew from 19 to **28**
registered designs (20 templates + 8 infographics) — `docs/plans/
catalog-diverse-library.md`. NEW blocks: `divider::rule`, `bar::bar` (both meet
the ≥2-template promotion rule). NEW templates across all four classes:
`quote`·`section_divider`·`agenda` (static), `compare_vs`·`feature_grid`
(split), `timeline` (animation), `checklist` (interactive). NEW infographics:
`gauge_dial`, `radial_progress` (vector-engine dials). All follow doctrine
(presenter law = pure clock functions resting once; Fluid sizing; tone through
the pool; tofu-safe glyphs). NEW gallery gate WITH TEETH:
`registry::every_demo_renders_at_start_and_at_rest` paints all 28 demos
headless at animation-start + rest without panicking (catches arc math / NaN /
galley bugs construction never exercises). Gate: `cargo green ✔`. Live spot-
check: the Library wall reports "28 templates registered"; `timeline` +
`compare_vs` render clean. CLOSE-OUT (Windows): rebuild the exe + screenshot
the wall at real sizes.


**2026-07-27 — W11 MEDIA → SLIDE (branch `feat/w11-media-to-slide`, PR):**
the gap "no template lets me add an image/pdf/video/session-replay and have
it become a slide" is closed. `docs/plans/w11-media-to-slide.md`. THREE
additive pieces: (1) `dpl_doc::Block::Media {id, as, src, fit, poster}` —
refs-not-values, `src` content-jailed at validate, `SCHEMA_V 1→2` (a v2
media slide fails LOUD on a v1 build); (2) the `media_frame` template (one
`registry::demos` line) rendering media FULL-BLEED via the reused
`media_backdrop` painter + the `document.rs` media pre-pass (backdrop under
text, reveal-honoring); (3) the shelf "Make slide" chip +
`Composer::make_slide_from_library` / `make_slides_from_pdf` /
`make_slide_from_rel`. PDF EXPLODES to per-page image slides via `pdftoppm`
(subprocess, ADR-009 — `import::rasterize_pdf`, quiet-spawn); a REPLAY is a
video artifact → video media slide on the session light `.mp4` (no special
case). W11 mints the slide; **W10a associates it** (upstream/downstream
split). `dpl_video::locate_asset` now probes `content/` (bridges the
content-relative artifact path space). Harness seam `--make-slide <rel>`.
PROVEN on the real exe (headless, scratch `--content-root`): image →
`inbox/poster/slide.json` (v2 media), a 2-page PDF → `inbox/report-{1,2}/`
page slides with pages imported. Gate: fmt + clippy clean (no `#[allow]`),
`cargo check --workspace` clean; doc/slides/composer/editor/video tests
green; the `#[ignore]` pdftoppm drill passes. Ported from the
`devpulse_labs` divergent snapshot onto live main (W10a/estate/coach
preserved — no file clobber; four diverged files hand-merged). KNOWN
follow-ups: `fit:"contain"` letterbox (data carried, renders cover today);
the shelf→W10a association hop. CLOSE-OUT (Windows checkout): rebuild
`target\release\devpulselabs.exe` + mtime, screenshot the "Make slide" chip
on a real indexed card at real sizes.
**2026-07-27 — CHAT IMAGINE DRAFTS LANDED (ADR-011 pending ratification).**
In-chat image generation: the composer's ▧ Image/video button opens the
Create-with-AI panel (image live, video truthfully staged); prompt = the
composer draft; one bounded worker POSTs the compiled-in xAI Imagine
endpoint (`grok-imagine-image-quality`, `b64_json`); results land in the
Ring-0 draft cache (`.userdata/cache/imagine/`) with preview + Save to
library (`import_generated` → `content/inbox`, source `generated`) or
Ignore; 30-day flush behind an Approval consent ("yes / never ask again /
not today", `imagine-policy.json`). New crate `dpl-chat-imagine` (the ONE
outbound HTTPS client, `ureq`); key = `XAI_API_KEY` read at call time,
never stored/logged — feature inert without it, and the fixture gate
refuses spend in authored mode. Grounding: Grok Build has NO generation
surface; Imagine is API-key-only → narrow exception drafted as
`docs/design/adr-011-imagine-generation.md` (operator tick pending; first
live generation = ratification-in-practice). Proofs: `chat-imagine-panel`
EguiPilot + `proof-chat-imagine-panel.png`; 7+17+41 crate tests; all six
prior chat scenarios green. Program map
`docs/plans/chat-creation-and-workflows-vision.md` (route-aware chat
actions + WiredFRONT Workflows are the next tiers). Plan
`docs/plans/chat-imagine-drafts.md` §8. **Next: operator provisions
XAI_API_KEY + first live generation; then video slice or storyboarding
workflow.**

**2026-07-27 — CHATGPT/CODEX ACTIVATION QUALIFIED.** Codex post-login
verification moved off the UI thread behind a `CodexStatusProbe` seam with a
visible `Verifying` state; activation evidence now mirrors Grok (attempt
ids, fabric facts feeding the activation trace, Turso ledger rows with
`method=chatgpt`); `RefreshProvider` rediscovers Codex; window ceremony copy
is provider-generic. Ready still qualifies `SubscriptionAuth` only —
conversations stay CH4-gated, proven by `chat-codex-persisted-ready.toml`
(Codex Ready + New chat/Send disabled; Grok independent). Native proof
`proof-chat-codex-activation.png`. Merge repair: #93×#94 semantic conflict
(`providers.profile-configured` missing from coach catalog) — the devtools
build is NOT covered by `cargo green`; consider adding it to the gate.
Contract `docs/plans/chat-codex-activation.md` + execution plan
`chat-codex-activation-pass.md` §8. **Operator gate: real ChatGPT sign-out →
login ceremony. Next: CH4 Codex app-server adapter (conversations) or
Claude/Gemini activation slices.**

**2026-07-27 — CHAT PROVIDER CATALOG LANDED.** The AI rail no longer lists
every provider: one AI Providers selector (current provider + active-session
count) opens the full catalog — ChatGPT/Codex, Claude Code, Gemini, Grok,
Clyffy, Add local (loopback-only), Add custom (HTTPS-only) — all visible
without scrolling. New `dpl-chat-profiles` crate owns typed provider
profiles; protocol events are laned Conversation/Activity/Diagnostics so raw
status/EndTurn/unknown ACP events never render as transcript cards. Contract
`docs/plans/chat-provider-catalog.md` §8; proof
`proof-chat-provider-catalog.png`. EguiPilot: `chat-provider-catalog` (new),
`chat-steam-social`, `chat-peer-window` (rewritten to catalog flow),
`chat-grok-persisted-ready`, `chat-grok-device-authorization` all green;
`chat-grok-signed-out-readiness` is the paired operator-state proof and
needs a signed-out Grok client to run. **Next: ChatGPT/Codex provider
activation pass.**

**2026-07-27 — COACH C1.5c LANDED: the whole nervous system is coachable, and
discoverable.** All six fabric domains map to coach signals — 31 facts across
`shell`/`deck`/`capture`/`jobs`/`providers`/`build`, up from 3 — named exactly
as the agent seam names them (`<domain>.<type>`, kebab-cased), so a fact has
one name on both surfaces. `shell.route/…` renamed to `shell.route-changed/…`
for that rule. Each fact gets a coarse id plus a specific one where the key is
enumerable (routes from the registry, providers from the registrations,
booleans); free-form runtime strings get no specific id on purpose.
`SessionStatus.coach_signals` closes discovery: the real exe declares **121
ids** including the shows discovered at boot, readable from `pilot_status` and
printed by `eguipilotai inspect`. Held closed by tests, not prose — published
⊆ declared, declared ⊆ publishable, every domain genuinely subscribed, and
exhaustive matches so a new fabric variant fails compilation (it already
caught #91's `VerificationRetryScheduled` on rebase). Mapping is
`crates/app/src/coach_signals.rs`. Evidence:
`docs/plans/operator-coach-a2ui.md` §11.

**2026-07-27 — COACH C1.5a LANDED: touchpoints wait for things now.**
Protocol 2.4. Every trigger fires — `press`, `hover`, `appeared`,
`value-changed`, `signal:<id>` — all as edges. A touchpoint aimed at a node
that does not exist yet mounts **armed** (`CoachEvent::Armed`) and mounts for
real when its target appears. The app publishes its own coachable signals one
way behind the devtools gate (`shell.route/<hub>/<sub>`, `build.slide-placed`,
`build.episode-changed`); the toolkit still cannot read the fabric. New
`CoachActor::App` for triggers no press explains. Multi-touchpoint dimming
fixed: one scatter layer with a hole per target instead of one dim each.
Proof `tests/eguipilotai/coach-dynamic-triggers.toml` +
`docs/proof/coach-armed-sub-episodes.png`. Evidence:
`docs/plans/operator-coach-a2ui.md` §10. **Next: C1.5b semantic revision
ticker** (deferred until polling measurably hurts) or C2a overlay ADR.

**2026-07-27 — COACH C1 LANDED: the agent can point at the screen.**
Protocol 2.3 adds the touchpoint router — `eguipilotai coach <profile>
<node-id> --message "..."` spotlights a live control (ColdLight ring +
gradient tint + black scatter dim + caption plate) and blocks until it is
pressed, reporting `actor: operator` vs `agent`. The overlay **observes but
never intercepts**: the press still reaches the real control, so coaching
works on real workflows. Scenario `tests/eguipilotai/coach-touchpoint.toml`
proves the whole lifecycle on the real exe; baseline
`docs/proof/coach-spotlight-rail-workbench.png`. Seventh MCP tool
`pilot_coach`. Only `spotlight`/`press` exist — every other visual/trigger is
capability-refused before a run starts. Evidence:
`docs/plans/operator-coach-a2ui.md` §9. **Next: C1.5 live semantic
snapshot/delta stream** (unlocks `NodeAppeared`/`ValueChanged` triggers and
demo-mode pacing).

**2026-07-27 — OPERATOR COACH PROGRAM PLAN LANDED (#88).** New program map
`docs/plans/operator-coach-a2ui.md` — touchpoint router
(`Mount`/`Unmount`/`Triggered` on the devtools pipe; spotlight = first
visual; coach vs demo = `Triggered.actor`), C1.5 live semantic
snapshot/delta stream (screenshots demoted to proof artifacts), C2 desktop
overlay daemon (ADR required first; X11 XShape → Win32 → layer-shell), C3
ColdLight A2UI catalog, C4 AT-SPI desktop control. Grounded against A2UI
v1.0-candidate / AG-UI / 2026 computer-use SOTA; verdict: stay pure Rust.
**Next: C1 pass** (in-app spotlight + press-to-trigger), amends delivery-doc
Slice 8. Operator box confirmed GNOME/X11; AT-SPI toolkit flag still off
(only needed at C4).

**2026-07-27 — GITHUB REPO HARDENING LANDED (#75/#79).** Dependency graph +
vulnerability alerts + automated security fixes; `delete_branch_on_merge`;
Dependabot weekly Cargo/Actions (live PRs #77–#86 — triage majors);
rolling Release tag `nightly` (exe-only; linux aarch64 proven); pull via
`./scripts/pull-nightly.sh`. Plan `docs/plans/github-repo-hardening.md` §8.
Open product PRs: Dependabot triage only.

**2026-07-27 — GROK SYSTEM-BROWSER AUTH LANDED (#67).** Device URL opens in
the OS default browser (not CDP/bundled Chromium); Cloudflare automation
reject fixed. Proof `proof-chat-grok-system-browser-auth.png`. Real-account
retest remains the operator gate. Contract: `docs/plans/chat-grok-activation.md`.

**2026-07-27 — W10a SLIDE ASSOCIATION LANDED (#72).** `dpl_doc::place_slide`
(copy into existing episode chapter; never-clobber; transactional rollback);
`BuildEvent` fabric domain + seam `build_json`; thin Workbench Studio place
form. EguiPilot `workbench-studio` +
`docs/proof/workbench-studio-place.png`. Plan
`docs/plans/w10-slide-association.md` §8a/§8.

**2026-07-26 — WORKBENCH CATALOG FIDELITY LANDED (#70).** Experience
inventory authored; Library gains Blocks chip (live eyebrow/badge/stat/
ambient); Templates + Seeds sectioned by class with honest empties;
EguiPilot fixture fixed to `workbench/library`. Plan
`docs/plans/workbench-catalog-fidelity.md` §8 +
`docs/plans/experience-inventory.md`.

**2026-07-26 — ESTATE→SEAM WIRE LANDED (#68).** Ring-0 estate at
`userdata_root()/estate`; seam seeds/persists `shell/route`; WiredFRONT
dual-writes `lastComp`; `GET /state` exposes `estate` + `lastComp`; cold boot
restores hydrated route when no `--route`. Plan
`docs/plans/estate-seam-wire.md` §8 — EguiPilot + restart drill PASS.
Preceded by **estate crate (#65)**.

**2026-07-26 — ESTATE CRATE LANDED (#65).** `dpl-estate` — Fjall LSM KV +
Moka read cache; partitions Docs/State/Manifest/Agents. Design ADR
`docs/design/estate-and-agent-seam.md`.

**2026-07-26 — W9c UNIFIED LANDING LANDED (#63).** Every PixVix vehicle
routes through `PendingLanding` + `land()` behind Auto/Ask; video/session
mint organized paths at start; LandingCard + policy strip (mode + bucket);
policy/card tags on index rows. Plan `docs/plans/w9c-unified-landing.md` §8.
Also merged same-day: **Grok signal-driven activation (#62)** (Codex).
**Hygiene:** pruned CLOSED stacked remotes (`feat/r1*`, chat-consolidation,
`design/coldlight-traits`, old estate park).

**2026-07-26 — WF EXTERNAL ASSETS LANDED (#60).** Reference-in-place imports
(`import_reference` single-file symlink + `⧉ copy` ⇄ `⌁ in place` toggle) and
cross-platform Browse roots + operator libraries (`★ Add/Remove library` →
`.userdata/libraries.json`). Plan `docs/plans/wf-external-assets.md` §8.

**2026-07-26 — W9a/W9b GUIDED LANDING (foundation + organized stills).**
**W9a** (#57): shared `dpl-landing` crate — `CaptureKind`, `LandingMode`
(Auto/Ask), pattern + policy JSON under `.userdata/`. **W9b** (#58): stills
land as `content/inbox/<bucket>/<kind>/<name>` (not flat `pix.png`). Plans:
`docs/plans/guided-landing-charter.md`, `w9-capture-landing.md`,
`w10-slide-association.md`. **W9c completes the program (#63).**

**2026-07-26 — RESPONSIVE RENDERING (RR) — RR0–RR2 LANDED.**
**RR0** (#54): the `fit()` law — no layout panic when floor > ceiling.
**RR1** (#55, `docs/plans/rr1-resolution-sweep.md`): `cargo xtask sweep`
— 6 routes × 7 sizes, isolated subprocess, software-GL; all 42 cells
no-panic on a real HDMI 1920×1080 (headless mutter @592×440 made `--window`
a NO-OP — display lesson). **RR2** (#56, `docs/plans/rr2-editor-responsive.md`):
editor uncramped below 1280px (fit panels, scroll tool strip / fx pane,
truncating NEXT hint). Proven @1024×640. DEFERRED: RR1.1 = offscreen render
+ `editor` eguiPilot fixture. **Runner note:** `dpl-linux` needs
`sudo ./svc.sh install` to persist across session teardown.

**2026-07-26 — GROK SYSTEM-BROWSER AUTH CORRECTION LOCALLY QUALIFIED:** the first
real-account attempt proved that visible bundled Chromium was still
automation-controlled through CDP and was rejected by xAI's Cloudflare
boundary. The corrected contract sends only the strict validated device URL
to the computer's default browser; bundled Chromium remains EguiPilotAI and
capture infrastructure. Four browser-boundary tests, 61 dependent
signal/UI tests, the full app check, all fourteen EguiPilotAI ceremony steps,
and native proof `proof-chat-grok-system-browser-auth.png` pass. The
real-account retest remains the operator gate. Canonical contract:
`docs/plans/chat-grok-activation.md`.

**2026-07-26 — GROK SEAMLESS ACTIVATION LOCALLY QUALIFIED
(`codex/grok-seamless-activation`):** the first real provider activation now
uses the exact bundled `grok login --device-auth` ceremony, a strict
`accounts.x.ai` URL/code parser, DevPULSE-owned visible Chromium, ACP
verification before Ready, and a non-secret local Turso evidence ledger.
The portable tool authority is consolidated under `dptools-deps/` with typed
`providers/`, `browsers/`, and `media/` families; Chromium is verified as a
complete tree. The authored eight-step EguiPilotAI flow and two byte-identical
real native-window captures pass. Personal xAI account completion remains the
explicit operator gate. Canonical contract:
`docs/plans/chat-grok-activation.md`.

**2026-07-25 — PRIVATE PORTABLE DEPENDENCIES LOCALLY QUALIFIED
(`codex/private-embedded-deps`):** `dpl-tools-deps` is now the sole
production authority for app-relative, manifest-declared, byte-length and
SHA-256 verified private tools. Codex, Grok, and ffmpeg no longer search
machine installs or `PATH`; Windows and Linux payloads stage through
`cargo xtask stage-tool-deps`. Real Windows and WSL Linux bundles passed
version probes and resolver verification. The real Chat peer passed, the
signed-out Grok surface exposes enabled provider-owned Login while Send stays
disabled, relocated release startup passed with provider-bearing `PATH`
entries removed, and the complete local `cargo green` gate passed. Canonical
contract and evidence: `docs/plans/private-tool-depot.md`.

**Tip:** on `main` (2026-07-27) at GitHub repo hardening (#75/#79). **DG1 LANDED** (#43,
`feat/dg1-mixed-guide`, `docs/plans/dg1-mixed-media-guide.md` — operator:
stills for where-to-click + session-replay clips when appropriate, ORDERED
in one `.guide`): replay-segment steps land — `Step.kind:"replay"` +
`clipIn/clipOut/clip` (recorder guides unchanged); export cuts segments
from the ORIGINAL session via `engine-media::render_clip_cut`
(replay-light law) into `step-00N.mp4` interleaved in place;
`guide.html`'s stage swaps still↔clip in one ordered sequence (embed
default; jailed sidecar mode); `guide_video_args` evolved to ordered
`VideoLeg`s; the reviewer round-trip (engine-steps::edit +
composer::review_edit) carries clip identity so caption edits never
drop media. PROVEN live: real ffmpeg cut + REAL headless Chrome walking
still → clip → still via dpl-engine-browser. **RING-1 LANDED** (#42, `feat/three-ring-root`,
`docs/plans/three-ring-root.md`, ADR-014 — operator-ratified after the
capture-location failure, senior conditions 1–6 as acceptance): the
THREE-RING data root — `content\` (CONFIRMED library, unchanged) ·
`assets\` (WORKING: auto-filed `assets\captures\<kind>\<job>\`) ·
`.userdata\` (STATE: `index.db` Turso, cache/temp/profiles + deletability
README). Index moved ADDITIVELY (`content\artifacts.db` → copy → verify →
labeled `.migrated-to-userdata`, `Index::open_adopting`); promotion =
`composer::import::promote_asset` (transactional: import commits, then
the assets original — and ONLY an assets original — is removed);
browser captures auto-file (`browser_cli shot <url> --job <job>`),
ephemeral profiles in `.userdata\temp\browser\`, named profiles on the
`.userdata\profiles\` shelf; boot `ensure_rings()` logs all three.
WiredFRONT = PREP house over assets; Workbench = BUILD house over
content. DuckDB analytics = RING-2 (own C-dep ADR, never smuggled).
UI surfacing (assets shelf, promote chips) = its own experience-gated
pass. **DEMO-B landed**
(`feat/demo-browser-driver`, `docs/plans/demo-browser-driver.md`,
ADR-013): `dpl-engine-browser` — headless Chromium as a SUBPROCESS over
CDP. Resolve-never-download (injected → `DPL_BROWSER` → bundled →
Chrome installs → Edge, the Windows zero-install fallback → PATH incl.
the Linux names); `--headless=new` + ephemeral profile + port from
`DevToolsActivePort`; navigate/evaluate/click/insert-text/screenshot;
`browser_cli shot <url> <png> [--window WxH] [--profile <dir>]`; live
`#[ignore]` drill PROVEN on real Chrome (1.3 s end-to-end). New
workspace dep ratified: tungstenite 0.27 (no TLS — localhost only).
**DG-B.1 folded in same-day** (operator: self-contained/portable, NO
extension): persistent named demo profiles (never deleted, never the
operator's real profile) + headful mode + `browser_cli login <dir>` —
the one-time visible sign-in ceremony; captures then ride the saved
session headlessly. First capture round banked: 3 public Twilio stills
(headless, 1366×768) + ordered manifest in the session scratchpad;
learning: Twilio login is EMAIL-FIRST (two-beat sign-in step). Paper trail filed: the
demo-pipeline charter (`docs/plans/demo-guide-pipeline.md`, DRAFT —
DG1/DG2 ordering awaits ratification) + the first storyboard
(`docs/plans/demos/demo-001-twilio-notification-keys.md`). Demo
identity: clyffy@angrygaming.org (operator-established; credentials
stay operator-side — the agent never enters passwords; captures ride a
signed-in demo profile). NOTE: `feat/lottie-1-spike` work is
uncommitted; its Cargo lines were parked and restored during this pass.

2026-07-24 (late), THE COURSE CORRECTION SESSION: the
operator ruled the R1 sequencing a failure (plumbing landed while the
editor stayed a chip list, and R1u's editor spine was GUIDE-maker DNA —
Capture/Annotate/Edit/Narrate/Preview/Export + an Export .guide chip).
The fix, ratified via a mock-first loop: `docs/design/mocks/
wiredfront-editor-mock.html` (v2.1) is THE editor spec — **After Effects,
panel for panel** — and `docs/plans/wiredfront-experience.md` is the UX
GATE: every editor pass must move a row of its §5 friction audit.
**W1 landed** (PR #28): the editor rebuilt to the mock. **W2 landed**
(`feat/r1y-media-windows`, `docs/plans/w2-guided-stitch.md`): R1y's
media windows (Browse peer viewports onto local folders + Explorer
drag-drop, both through ONE import seam — copy to inbox/elements,
poster+seconds, index write-through, sources never touched) PLUS the
guided stitch workflow (NEXT-step hint strip · seeded comp names on
Save · renders indexed with posters · KEEP chips file the render into
any show from the editor) and the GUIDE tab as the tailored 1-2-3-4
export flow (seeded name chips, honest player-pass note).
**RATIFIED 2026-07-24 (operator): `.aep` + `.lottie` are first-class
imports** (personal-use tool) — they enter as ELEMENT artifacts
(`content/elements/imported/`, indexed); PARSING into comp layers is
its own spike, never faked. New harness seam: `--import-drill <dir>` +
the ignored FS drill `composer/tests/import_drill.rs`.
**W3 landed** (`feat/w3-pack-shelf`, `docs/plans/w3-pack-shelf.md`):
the PACK SHELF — the operator's themeforest stores pinned IN PLACE
(census: video = 70,018 files/79.6 GB incl. 9,021 .mogrt + 5,392 .aep
+ 860 .cube; audio = 250/2.6 GB; **C: had 14 GB free → the copy-all
ratification is SUPERSEDED by arithmetic; pin-in-place + copy-on-use
is law**). `elements/sources.json` (app-written registry) + ELEMENTS —
your packs in the Project panel (real `videohive-*`/`audiojungle-*`
rows, capped counts) → click opens a Browse window on the pack →
click a file copies JUST IT through the import seam. Kind matrix grew:
audio (wav/mp3/ogg/flac/m4a/aac → inbox) + mogrt/cube/ffx/prfpset
elements. Pin pack chip in Browse windows; `--pin-pack <dir>` harness
seam; D: trees are READ-ONLY (mtime-asserted). ALSO: the 51 GB
`target\debug` tree was deleted mid-pass when C: ran out of disk.
**W4 landed** (`feat/w4-pack-ingest`, `docs/plans/w4-pack-ingest.md`):
BOTH modes, as the operator asked — pins stay the in-place reference
(the app-level symlink) AND a per-pack **Ingest** grabs favorites
local: two-tap armed chip in the pack's Browse window (arm shows the
real `N files · X.XX GB` before anything moves), one background ingest
at a time with a live progress pill, copies land in
`content/elements/<pack>/` (indexed + postered, `source="ingest"`),
`.part`-then-rename (no torn files), SKIP-IF-EXISTS (resumable),
disk-full aborts honestly; ingested packs read **local** on the shelf.
Sources on D: stay read-only, always. Literal NTFS symlinks under
/content were REJECTED (they'd drag 70k files into rescans and blur
the artifact jail).
**W5 landed** (`feat/w5-guide-export`, `docs/plans/w5-guide-export.md`
— the execution plan superseding the proposal): **the real .guide
export**. RATIFIED (operator, this session): deliverable = BOTH the
self-contained `guide.html` AND a single `guide-video.mp4`; replay
EMBEDDED in the html by default with a sidecar toggle. NEW crate
`dpl-guide` (manifest round-tripping the recorder's guide.json ·
hand-rolled base64, zero new deps · `write_html` — stills as data URIs,
swipe/arrows, do/done toggle, recorded-click marker dot, replay with
tappable event chapters, ZERO external refs · `export_guide` — copy +
renumber kept stills, light replay re-encode, forward-contract folder
the native player will zip). engine-media grew `render_replay`
(≤720p/15fps/crf30 — 88→44 KB on the drill) + `render_guide_video`
(stills held 3s then the replay, one mp4). The Guide tab is the REAL
flow now: pick (guides + sessions from the index) → REVIEW & TUNE (the
W5 reviewer `engine-steps::edit` was waiting for: thumbs, Keep/Skip,
reorder, captions, two-tap Apply that prunes source frames) → name →
export (embed + video toggles, off-thread) → keep chips. ONE path
`dpl_composer::run_export` (jail, no-clobber mint, index write-through
kind "guide" source "export") shared by the tab and the new
`--export-guide <guide-rel> [<session-rel>]` headless seam. Proof: §8
of the plan (real-exe drill: 4 embedded stills + embedded mp4 + 0 http
refs, guide-video exactly 15.000s; shots
docs/proof/w5-guide-tab-{1100x700,1600x1000}.png). Proof-only env
`DPL_GUIDE_PICK` opens the reviewer for screenshots.
**W5.3 landed** (same branch, operator mid-session ask "full test by
you ... exported as gif or whatever ... for linkedin"): the SOCIAL GIF
(`replay.gif`, palettegen two-pass, 480px/12fps, Guide-tab toggle;
LinkedIn-native = the bundled light session.mp4) + the AI-DRIVEN FULL
TEST seam `--steps-drill <n>` (REAL recorder start via the palette
handler with a drill-only self-capture opt-in, n real marked window
captures, real Save-card save). FULL TEST RAN, all real: 3 stills
captured by the drill, real 8s session (490 KB -> 44 KB light re-encode),
replay.gif 179 KB/79 frames, guide-video 15.6s, self-contained html.
FINDING FIXED: seeded guide names now clamp to 48 chars (a path-length
window title minted a folder that broke Win32 cwd limits).
**P1 landed** (same branch, `docs/plans/p1-data-root.md`): KNOWN #3
FIXED -- the portable user-data root. Operator-ratified ONE folder
(content/ stays the single tree); resolution law ported from the
original: `--content-root` arg -> CONTENT_ROOT env -> exe-adjacent ->
exe-parents walk-up -> cwd, first existing; nothing -> self-creates
BESIDE THE EXE; `checked_roots()` honesty logged at boot. PROOF: exe
run from target/release resolved the repo content root, no scatter.
Operator-approved migration: the 9 scattered captures moved into the
real inbox (2 renamed `-migrated-N`), stale tree removed. NOTE: one
Gallery `rescan` tap reindexes them.
**R1z landed** (`feat/r1z-editor-safety`,
`docs/plans/r1z-editor-safety.md`): NEVER LOSE WORK — friction rows 1+8
closed. Snapshot UNDO/REDO (comp is pure `Clone+PartialEq` data; cap
100, deduped, ONE step per completed drag; history per-comp) ·
debounced AUTOSAVE (2s after the last edit on a SAVED comp, crash-safe
`dpl_comp::save`, flush on New/Open + `Drop` at exit; facts line shows
`· unsaved edits` honestly; unsaved comps still wait for the guided
Save) · REOPEN-LAST (`composer-state.json` at the content root — the
`sources.json` precedent — written by Save/Open, restored once-lazily
only when the editor would open empty; drills/tests stay hermetic) ·
the KEYBOARD MAP, one pure unit-proven `key_act` path guarded by
Editor-tab + `egui_wants_keyboard_input`: Space play/pause · Del/
Backspace remove-selected · Home/End · ←/→ step 1s · Ctrl+Z /
Ctrl+Shift+Z / Ctrl+Y (operator ratified the arrows/Home/End extras).
BUG FIXED en passant: New was `*self = default()` — it severed the
in-flight render/ingest/export Arcs, closed Browse windows, and dumped
every cache; it is now a comp-only reset (`new_comp`). New harness seam
`--save-comp <name>` (Save-chip path headlessly). PROOF: 27 composer
tests; real-exe SEQUENTIAL drill — run 1 imports 2 clips + saves
`r1z-proof`, cold-boot runs 2+3 REOPEN it ("Reopened inbox/r1z-proof —
your last comp.", both layers on the timeline); shots
`docs/proof/r1z-reopen-{1280x800,1600x1000}.png`.
**R2a.1 landed** (`feat/r2a1-deep-pack-browse`,
`docs/plans/r2a1-deep-pack-browse.md`): deep-pack browse, the
D:-independent half of R2a — ground truth 2026-07-25: the
`D:\webwork\themeforest` store is OFFLINE (drive absent; G: is Google
Drive) and the real root has NO sources.json — the operator never
pinned the real stores. Shipped: ① OFFLINE-PACK HONESTY — an
unreadable pinned source lists as one `offline` shelf row (tooltip:
plug it back in; ingested copies stay local) instead of silently
vanishing (the old `continue` bug, exposed by the missing drive
itself); ② Browse-window SEARCH — type-to-filter the open folder live
+ Enter runs a recursive name search under it OFF-thread (cap 500,
said in the UI), results in the same click-to-import cards, nav/x
clears; ③ census fix — `sources.json` no longer counts as an element
(phantom "1 imported element" since W3). PROOF: 32 composer tests;
pin→unplug→boot drill shots
`docs/proof/r2a1-offline-pack-{1280x800,1600x1000}.png`.
**R2a.2 landed** (`feat/r2a2-mogrt-reader`,
`docs/plans/r2a2-mogrt-reader.md`): the drive RETURNED mid-session and
the REAL stores are PINNED in the real root. SPIKE VERDICTS (real
specimens, 2 unrelated packs): `.mogrt` = plain zip
(definition.json + project.aegraphic + thumb.png; DEFLATE) — the
metadata layer of the ratified preset-editing is READABLE without AE;
`.aep` = RIFX `Egg!` opaque — **the parsing question stays OPEN by
operator ruling ("we will keep it open as I research")**, files remain
elements; `.cube` = trivial text (R2b confirmed cheap); the stores
carry Mac `._*` AppleDouble twins (dot-skip already excludes).
RATIFIED: dep = the FULL `zip` crate ("I will need it eventually") —
workspace `zip = "8"`, deflate-only. SHIPPED: `composer/src/mogrt.rs`
reader (capsuleName/capsuleTags/thumb.png, stem fallback, honest
errors); import + ingest ENRICH mogrt element records (real name, tags,
extracted `-poster.png`); Browse cards paint REAL mogrt previews via a
budgeted lazy cache (≤8 zip reads/frame, `bytes://` egui uris); the
import status says what was actually read. `clientControls` (editable
text/font/size decls) documented, read when the controls UI pass
consumes it. PROOF: 36 tests; real `Title 01.mogrt` through the exe →
sqlite-verified enriched row + byte-identical extracted poster; shots
`docs/proof/r2a2-mogrt-import-{1280x800,1600x1000}.png`.
**W6 landed** (`feat/w6-frictionless-media-in`,
`docs/plans/w6-frictionless-media-in.md`) — operator redlines killed:
(a) the LIBRARY is the real thumbnailed MEDIA SHELF — double-click any
clip/image and the Editor takes it (video → timeline via the one
add_clip path, undo/autosave riding; image → FOOTAGE honestly until
image layers) and the app navigates there; one outside seam
`Composer::add_from_library`. (b) MANAGED FFMPEG at boot (the original
tools.rs law): bin/ beside exe → process PATH → winget Links → the
winget Packages scan (FINDING: Gyan.FFmpeg never gets a Links shim),
injected into all three engine seams; none found warns naming every
location. PROOF: PATH=System32-only export drill runs end to end; the
drill also proves P1's `--content-root` arg live. Friction rows 13+14
DONE. Shots docs/proof/w6-library-{1100x700,1600x1000}.png.
**W7 landed** (`feat/w7-image-layers`,
`docs/plans/w7-image-layers.md`; operator: images are "kind of
essential for all of this to matter"): IMAGES ARE FIRST-CLASS TIMELINE
LAYERS. No schema change (a still is `in_pt=0, out_pt=5.0`, trimmable);
`StitchClip.still` + loop/hold ffmpeg legs (+ `format=yuv420p` on every
leg — mixed concat law); every add path appends (FOOTAGE tap, Library
double-click, Explorer drop, import drill); the preview well shows the
held frame (no ffmpeg pipe for stills); the seam maps stills so agents
get image layers over HTTP free. PROOF: real-exe import of png+mp4 ->
both on the timeline, preview shows the still; seam render =
8.000000s exactly (5s still + 3s video) at 1920x1080. Friction row 15
DONE; the "image-layers pass" copy is GONE repo-wide. Shots
docs/proof/w7-image-timeline-{1100x700,1600x1000}.png.
**W8 landed** (`feat/w8-faithfulness`,
`docs/plans/w8-faithfulness.md` + THE LOCKED
`docs/design/signal-doctrine.md`; operator: "stop rushing … establish
clean patterns … my signalkind"): THREE HOUSE LAWS. (1) QUIET-SPAWN —
`dpl-proc::quiet` at every GUI-reachable subprocess (video pipes,
engine factories, chat providers' collapsed duplicates, browser,
lottie) + a cargo-green AUDIT: an unflagged `Command::new` fails the
gate. (2) PROACTIVE-BUFFER (doctrine.md §perf) — Browse scans
off-thread, warm cache seeded at first visit, roots probed once off
the paint thread; the 20s freeze's both causes structurally gone.
(3) THE SIGNAL DOCTRINE (ratified: stamp-at-publish w/ ONE global seq
across domains, activity feed in W8, commands contract-only until
R2c) — fabric grows CAPTURE + JOBS domains + the WAKER; every
mutation signals through index+signal seams (viziwiz's one seam, the
composer, hubs, THE AGENT SEAM's render — a missed site caught live
by the silent SSE proof); views invalidate on signal (refresh chips
no longer load-bearing); session events.jsonl + seam `/signals` speak
ONE ENVELOPE `{domain,seq,ms,type,…}` pinned by test; the ACTIVITY
FEED lives in the inspector (proof shot: the import drill's "saved
image" line — TWO boot-ordering bugs fixed: wire-before-drills +
ears-at-fabric-birth). Proofs in plan §8 (SSE capture of a live
agent-render's stamped queued/failed; green ✔ incl. the audit).
RESOLVED (2026-07-27, operator: "it is in dptools-deps — consolidate
properly"): the tool supply is CONSOLIDATED on the Central Supply
Depot. `C:/localdev/dptools-deps/artifacts/windows-x86_64/` now holds
all four tools (codex 0.145.0-alpha.18 pulled from the OpenAI hashed
install, ffmpeg 8.1.2 from winget, chromium extracted, grok);
`dptools-deps/sources.local.toml` (gitignored) points EVERY entry at
the depot; the staged release bundle verifies and a real seam render
ran through the verified ffmpeg (23.0s, exact). The repo's stray
`embedded-deps/` husk is deleted and the last two in-code
"embedded-deps" strings renamed — grep is clean.
**AEP ORCHESTRATOR — new pass, PAUSES R2b** (operator 2026-07-25:
installing AE/Animate; "pull aerender.exe into ours and have wiredfront
editor viziwiz actually be an orchestrator"). This RESOLVES the open
`.aep` thread the house way — never parse RIFX, ORCHESTRATE Adobe's own
headless `aerender.exe` as a subprocess (ADR-009, the ffmpeg pattern).
Planned via the NEW `/deep-plan` skill (Fable architects; one Opus
agent filled the AE-execution gap, spot-checked): full plan
`docs/plans/aep-orchestrator.md`. GROUND TRUTH (verified): aerender
26.3x87 is fully headless (~13s boot, parseable `PROGRESS:`/`INFO:`/
`ERROR:` stdout); comp enumeration needs `AfterFX.exe -r` which is
blocked by AE's Welcome/Home screen (`"Show Welcome Screen" = 01`, one
pref toggle); default OM templates incl. `H.264 - Match Render Settings
- {5,15,40} Mbps` (mp4) + Lossless; ffmpeg 8.1.2 has libwebp_anim/
libaom-av1/svtav1/gif for the export selector. RATIFIED decisions: full
comp picker in v1; app toggles the Welcome pref on the operator's tap;
export selector (mp4/webp/avif/av1/gif); DYNAMIC perf tuning (16GB
laptop is the operator's WEAKEST — 32/64/128GB rigs coming for content/
streaming); dep `sysinfo` for the RAM probe.
**Phase 1 LANDED** (`feat/aep-1-engine`): NEW crate `dpl-engine-ae`
(the ADR-009 sibling of ffmpeg) — `locate` (AE 2026 found), `Caps`
probe + `RenderTuning::for_caps` dynamic policy (MFR OFF/RAM-conservative
@16GB, ON @128GB; Balanced/MaxThroughput/Gentle profiles), RIFX gate,
aerender arg-VECTOR builder (spaced template names survive — the
quoting gotcha), stdout grammar parser, safe Welcome-pref toggle
(AE-closed guard + `.dpl-bak` backup, ending-preserving). 10 tests; a
`--example probe` per-rig diagnostic. NEXT PHASES: 2 enumeration
(first real-AE run, after the pref toggle) → 3 render+timeline ingest →
4 ffmpeg export selector → 5 automation tap + tuning UI.
**THEN R2b** (resumes after the orchestrator): the 860 `.cube` LUTs via
ffmpeg lut3d (gate row 7), then R2c chat-edits-comp. Ratified:
3D/camera/tracker/motion-blur LATER; AI ephemeral in-app + chat (one
seam); WiredFRONT before Workbench.
Memory: `experience-first-gate`, `wiredfront-orchestrates-adobe`.
KNOWN (pre-existing, observed by the W2 cross-process drill): opening
`artifacts.db` with a WAL left by ANOTHER process trips the index's
quarantine (`.corrupt`); in-app (same-process) flows unaffected.
NOTE: `seeds/slides/static/headline-clean/slide.json` carries an
operator edit (fonts→orbitron, moods) left uncommitted on purpose.

**Architecture (ratified):** three products, all Rust/egui —
`docs/design/viziwiz-architecture-spec.md` + the rewritten
`docs/plans/port-viziwiz.md`: WiredFRONT capture / Workbench authoring /
Native Guide Player (winit+wgpu+libmpv; `.guide` ZIP + portable `.exe`).

**Chat is DONE as a peer window** (`docs/plans/chat-consolidation.md`): a
status-bar-triggered, Steam-overlay-like native peer window — NOT a hub route.
It carries the read-only session histories + trait provider chips, and a lite
**route launcher** that publishes `NavigateRequest` so the MAIN window
navigates (chat launches routes, never embeds them). The rail-hub `dpl-hub-chat`
is DELETED. Memory: `chat-is-a-peer-window`.

## Proven working (engine layer)

- Programs L (IA/accordion rail), D (deck/present/presenter/scripts),
  X (index/gallery/keep), C (capture parity: stills C1, steps C2,
  video C3, ViziWiz C4) — each with plan doc §8 evidence + exe drills.
- OS-global hotkeys Ctrl+Shift+S/9/X (rdev); Steps Save card; video
  format/audio matrix; ink flatten (source never mutated).
- EguiPilotAI client seat works: doctor green, live pixvix snapshot.
- **WiredFRONT Composer (R1a)** — the "After Effects, chat-route-aware"
  video editor's FOUNDATION landed (branch `feat/r1a-comp-document`, off
  `refactor/chat-consolidation`): `dpl-comp` (the `comp.json` composition
  document + doctor — tracks→layers→sources, refs-never-values, crash-safe
  save, `validate → Vec<Issue>`, folder contract; mirrors `dpl-doc`; 8
  tests) + `dpl-composer` (four-pane shell — Palette · Timeline clip-list ·
  Inspector · single-source Preview via the S2 ClipPlayer; ColdLight paint,
  cl::tip everywhere; 4 tests) mounted as the `wiredfront/composer`
  subroute. `cargo green ✔`; proof
  docs/proof/r1a-composer-{1100x700,maximized}.png. ADR
  `r1-editor-adr-proposal.md` — four forks RATIFIED 2026-07-24: hybrid
  gen backend (local ComfyUI + hosted media APIs behind one `GenBackend`
  trait, chat mesh orchestrates), local ort+BiRefNet matting, copy-all
  elements into `content/elements/`. Plan `docs/plans/r1a-comp-document.md`.
  R1c (elements ingest) → R1d (ViziWiz mode) → R1e (matting) → R1f (agent
  tools) remain. Memory: [[wiredfront-editor-is-r1]].
- **R1b.1 — the stitch RENDER landed** (branch `feat/r1b-stitch-render`,
  stacked on R1a): `engine-media::stitch_args`/`render_stitch` (decoupled,
  primitive inputs — no dpl-comp dep) build the ffmpeg CONCAT filtergraph
  (trim → fps-normalize → scale+pad → concat, `-an`, faststart, crf 21,
  ADR-009 subprocess) + a **Render** chip in the Composer that stitches the
  clip-list into `inbox/<comp>/render.mp4` OFF the UI thread (polled
  `RenderState`). `cargo green ✔`. END-TO-END PROOF: two mismatched clips
  (640×480@30 + 800×600@24) → one 1280×720@30 file of exactly 5.00s
  (docs/proof/r1b-stitch-render-demo.mp4). Plan
  `docs/plans/r1b-stitch-render.md`. R1b.2 (xfade + composited preview +
  egui_tiles docking) is the next rider; then R1c–R1f.
- **R1s — editor SHELL landed** (branch `feat/r1s-editor-shell`, stacked on
  R1b): the operator's real design reference (two images, 2026-07-24) is now
  captured in `docs/design/wiredfront-editor-inventory.md` — THE fidelity
  bar (the gate KNOWN BROKEN #5 said was never authored). The Composer was
  reshaped from a flat 4-pane skeleton to the **WiredFRONT editor spine**: a
  workflow-tab bar (Capture · Annotate · Edit · Narrate · Preview · Export +
  Export .guide, active glows voltage), a SCENES/ASSETS left rail, and the
  active mode's surface — EDIT hosts the working timeline editor, the other
  five tabs are honest styled stubs naming what each will do. Tofu-box
  glyphs (＋/↑/✕) replaced with font-safe labels. `cargo green ✔`; proof
  docs/proof/r1s-editor-shell.png. Pass order to full fidelity is in the
  inventory doc §"Pass order": R1-scene → R1-annotate → R1-steps →
  R1-narrate → R1-preview → R1-export, interleaved with R1c–R1f.
- **R1t — Preview + Annotate modes** (branch `feat/r1t-preview-annotate`,
  stacked on R1s): the ANNOTATE tab now shows the tool row
  (Select/Arrow/Callout/Box/Spotlight/Text) over a scene canvas; the
  PREVIEW tab shows a 16:9 device frame + Touch/Mouse/Keyboard input chips;
  both render the selected clip's frame via the ClipPlayer. A
  `DPL_COMPOSER_TAB=<mode>` env override opens a specific tab for
  screenshots (proof affordance, inert in normal use). Proof:
  docs/proof/r1t-{annotate,preview}.png. Still stubs: tool behaviour, the
  annotation data model, input-mode selection, sequence preview — the
  annotation/preview passes fill them. PR #24.
- **R1u — editor FIDELITY** (branch `feat/r1u-editor-fidelity`, stacked on
  R1t, PR #25): the operator's explicit layout ask landed — the transport
  toolbar (New/Save/Render + stats) spans the full width along the BOTTOM
  with the timeline directly above it; TOOL SELECTION is a real, persisted
  LEFT rail (Select/Arrow/Callout/Box/Spot/Text, Active state presses in)
  shared by Edit + Annotate; the inspector is adaptive (170–240) so
  1100x700 never starves the preview. The RAIL ACCORDION unfolds
  wiredFRONT → **Editor · Library** · Overview · ViziWiz · Gallery ·
  Sessions on a recessed plate (surface::BASE), rows indented 10px, hover
  wakes the ink (`composer` SubId renamed `editor`; no shim). NEW
  `wiredfront/library`: the generated INGREDIENT LIBRARY
  (`dpl_composer::library_ui`) — renders ONLY ColdLight's generated
  registries: TRAITS composed live via `compose()`, LOOK/MOTION presets,
  FONTS in their own faces, MOODS sampled via `Mood::at`. Never a one-off;
  the Nth ingredient = JSON + regenerate, zero UI code. Gate ✔ (fmt,
  clippy, all tests, release; ran in the pre-push hook). Proof:
  docs/proof/r1u-{editor-1280,editor-1100,library,rail-open}.png. Plan:
  docs/plans/r1u-editor-fidelity.md.
- **R1v — the AGENT SEAM landed** (branch `feat/r1v-agent-seam`, stacked on
  R1u): `dpl-seam` — axum on `127.0.0.1:7341` (own thread + runtime, egui
  never touched; `--seam-port <n>` / `--no-seam`): `GET /state` (last-heard
  route + the full hub/subroute registry built from the SAME HubSpecs the
  shell registers), `POST /command` (navigate / viziwiz-summon — unknown
  routes refused BY NAME, 422 + issues, doctor-style), `GET /signals` (every
  ShellEvent as SSE; wire shapes pinned by test, fabric stays serde-free).
  5 tests. LIVE PROOF: POST navigate → the real exe navigated (GET /state
  flipped to wiredfront/editor) and SSE captured navigate-request +
  route-changed (§8 of docs/plans/r1v-agent-seam.md). This is how chat
  drives the editors. Estate (Fjall+Moka) is pushed on
  `feat/estate-fjall-moka`, ready to join the seam for agent persistence.
- **R1w — CHAT RENDERS VIDEO over the seam** (branch
  `feat/r1w-comp-endpoints`, stacked on R1v): the comp-document surface —
  `GET /comps` (inbox census) · `GET /comp?dir=` (document + live doctor
  report) · `POST /comp` (DOCTOR-GATED write: invalid → 422 with every
  issue named, NOTHING touches disk; absent dir mints `inbox/comp-N`) ·
  `POST /command {"cmd":"render"}` + `GET /render?dir=` (the ffmpeg stitch
  off-thread, polled; sources never mutated, output is always the NEW
  render.mp4). Rel dirs JAILED inside the content root (`..`/absolute/
  drive-letter refused by name). 9 tests. LIVE PROOF (scratchpad
  CONTENT_ROOT): POST a 2-clip comp over HTTP → render → ffprobe
  `duration=5.000000` exactly, mismatched sources (640x480@30 + 800x600@24)
  normalized to 1280x720@30 — an agent authored a composition and got a
  rendered video via HTTP end to end (§8 of docs/plans/
  r1w-comp-endpoints.md). NEXT riders: R1c elements ingest → Library;
  annotation model for the left-rail tools; estate-backed agent state.

## KNOWN BROKEN / regressed (the honest list)

1. **Footer/status bar invisible unless maximized** — FIXED (this PR):
   continuous `fit_window` guard (the old repo's work-area law, ported) —
   any restored window exceeding the monitor is auto-shrunk (decoration
   delta honored) and nudged on-screen, every frame, so DPI changes and
   monitor drags self-correct. Proof: `--window 2000x820` on a smaller
   monitor auto-fits to 1522×778 with the footer visible
   (docs/proof/footer-2000x820-fit.png). `--window <w>x<h>` is now a
   standing proof flag — UI proofs must include non-maximized sizes.
   NOTE: egui exposes monitor size, not the work area — the guard
   reserves 48 logical px for the taskbar; if a taller taskbar clips,
   raise the allowance or port a Win32 work-area query.
2. **The experience layer was never specced.** NOW BEING FIXED:
   `docs/design/doctrine.md` DEFINES the language (BRUTAL = actionable,
   CLAY = passive, tooltip law, token law, any-shape law, the feel bar)
   and the `coldlight-system` skill (formerly `dpl-style`) gates every UI
   pass on it. ColdLight grew the primitives (`brutal_frame`/`clay_frame`/
   `tip`, `RADIUS_LG`).
   SUPERSEDED (2026-07-24): the thin BRUTAL/CLAY doctrine is being
   replaced by the composable TRAIT system faithfully ported from the
   original ColdLight (neubrutalism/claymorphism base + cyber/retro
   stack, per-axis compositor, generated manifest the Workbench editor
   reads). Plan: `docs/plans/pass0-coldlight-treatments.md`. Skills:
   `coldlight-system` (foundation gate) + `slide-catalog-pass` (per-slide
   faithful port). See KNOWN BROKEN #5 below.
   First application landed: the shared `filter_chip` is brutal (hard
   offset shadow, hover press) with layman tips on gallery filters,
   rescan, and ViziWiz Undo/Clear/Flatten.
   Sweep 1 DONE: pixvix palette — every chip brutal via the tip-mandatory
   `pilot_chip` (18 controls with layman tips: snap modes, delay, re-snip,
   video matrix, session, guide Save/Discard, REC pill Mark/Stop) + tab
   tips + refresh buttons. NOTE: the palette is its own OS viewport —
   visual proof awaits the operator's eyes / pilot capture (immediate-
   viewport capture unsupported); compile+green verified, tips visible in
   pilot snapshots.
   Sweep 2 DONE: shell chrome — footer launchers (Snip/Agents) brutal
   with layman tips; rail house tiles tip their hub name; unfolded
   subroute rows tip their blurbs (proof:
   docs/proof/doctrine-footer-launchers.png).
   Sweep 3 DONE: hub placeholder subroute cards → CLAY (passive planes;
   active keeps the tone scatter). FINDING for its own pass: these
   cards LOOK tappable but are inert — hubs have no navigation seam;
   needs the deferred fabric NAVIGATE event, then the cards go brutal
   and tap-to-navigate (frictionless law).
   Sweep 4 DONE: gallery + sessions — layman tips on the session thumb
   ("Play this session right here"), the two-tap Dump (armed state
   explains itself), Keep + every keep-target chip, the session list
   cards, the player's back/play controls, and the seek rail (names the
   orange event ticks).
   Sweep 5 DONE: workbench Library — section chips explain their walls
   in plain words, the solo view's back/replay tip, wall titles say
   "Tap to see this one big". Deck toolbar audited: already compliant
   (tool_button carries tips).
   Sweep 6 DONE: ViziWiz — tone dots name their color, width slider,
   back, and picker cards all tip in plain words.
   NAVIGATE seam DONE (plan docs/plans/navigate-seam.md): hub subroute
   cards are tap-to-navigate — `ShellEvent::NavigateRequest` published
   by chat/workbench/wiredfront (wire_fabric), drained by the shell;
   cards show the tone edge + "Go to <label>" tip on hover; the session
   EventLog records `navigate-request` honestly.
   **Remaining**: the feel-bar review pass across every route against
   the old app (screenshot-by-screenshot). Every UI pass MUST invoke
   `coldlight-system` (the gate, formerly `dpl-style`).

5. **The visual language was invented, not captured (operator, 2026-07-24).**
   The port replaced the original's real ColdLight design system — authored
   trait JSON (neumorphic/cyber/retro) composed per-axis, a manifest the
   editor reads, documented in the original's `src/lib/design/TRUTH.md` —
   with a hand-invented BRUTAL/CLAY doctrine + magic-number `*_frame`
   helpers. The `experience-inventory.md` gate that both CLAUDE.md and
   doctrine.md require WAS NEVER AUTHORED, so Sweeps 1–6 ran against the
   invention with no reference to the original's look. AND the Workbench
   Library holds only 4 authored seed slides (all `static`) of ~27 catalog
   types — the renderers are ~90% ported but the slides aren't organized.
   FIX (ratified 2026-07-24 "go"): `docs/plans/pass0-coldlight-treatments.md`.
   Foundation LANDED on main (`compose` / `cl::treated` / neubrutalist+
   claymorphic). PR #16 (`design/coldlight-traits`) was CLOSED without merge
   after the stack landed via later PRs — remote branch kept for triage of
   any leftover tip commits. REMAINING in this program: tints painting +
   cyber/retro anim (needs motion.json + painter), the generated manifest
   the Workbench editor reads, the experience-inventory doc, then the
   `slide-catalog-pass` loop (~27 slides). Skills: `coldlight-system` +
   `slide-catalog-pass`. Memory: `port-fidelity-standard`.

## CI + process (live as of github-repo-hardening)

- Self-hosted `dpl-win-laptop` (Windows) + `dpl-linux` / `dpl-linux-clyffy-lab`
  (ARM64 mini-PC) run the green gate. Branch protection is Pro-gated — CI
  is advisory; enforcement = local pre-push hook (`core.hooksPath=.githooks`,
  mode 100755) + discipline. NEVER squash-merge a code PR unless
  `green-linux` is green (Windows pending alone is not a fail).
- **Dependency graph + Dependabot:** vulnerability alerts + automated
  security fixes enabled; `.github/dependabot.yml` weekly Cargo + Actions
  version PRs. GraphQL proves Cargo manifests on the graph.
- **Nightly Releases:** `.github/workflows/nightly.yml` → rolling tag
  `nightly` (exe-only zips). Pull: `./scripts/pull-nightly.sh` /
  `.\scripts\pull-nightly.ps1`. Plan: `docs/plans/github-repo-hardening.md`.
- `delete_branch_on_merge=true` on the repo.
- Docs-only PRs skip CI (paths-ignore) and may merge immediately.
- Runner persistence: Windows → schtasks (CLAUDE.md); Linux →
  `sudo ./svc.sh install` (session `./run.sh` dies on logout).
3. **FIXED (P1, 2026-07-25)** — the portable user-data root law ported
   (docs/plans/p1-data-root.md): resolution is arg -> env -> exe-adjacent ->
   walk-up -> cwd; double-click finds the repo content root (proven);
   scattered target/release/content migrated + removed.
4. Stale EguiPilotAI scenarios/fixtures (pre-Program-L flyout/Library refs).

## Next

1. **W10b** — LandingCard Episode polish, live budget strip, activity-feed
   line, full chapters×slots compose wall (`w10-slide-association.md`).
2. Estate deeper wire (selection/undo, Workbench session, Agents tree).
3. Seed empty classes / animation templates (catalog follow-ons).
4. Parallel backlog: RR1.1; AEP Phase 2; ColdLight remainder; mac-parity.
   Ops: triage Dependabot PRs (#77–#86, don't blind-merge majors); bring
   `dpl-win` online for Windows nightly asset; `dpl-linux` `svc.sh install`;
   unset Cursor `CARGO_TARGET_DIR` for local green. Operator: real xAI
   account retest for Grok system-browser auth (#67).

## Process facts

- Work lands via PR. The operator's exe rebuild + this file's update are
  part of DONE. Pushes can be slow — background them; commit is what counts.
- Old app reference: `C:\localdev\devpulse labs` (READ-ONLY).
