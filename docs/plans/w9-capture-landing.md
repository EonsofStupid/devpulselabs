# W9 — Capture landing: one guided, organized save for every PixVix vehicle

Governed by `docs/plans/guided-landing-charter.md`. Gate:
`docs/plans/wiredfront-experience.md` (§5 rows moved in §6 below).
Builds on — never re-does — `w2-guided-stitch.md` (guided save + KEEP),
`c2-steps-parity.md` (the pending Save card), `w6-frictionless-media-in.md`
(`add_from_library`), `w8-faithfulness.md` (capture signal domain),
`x1-index.md` (index write-through). Ground-truthed in-session 2026-07-26.

## 1. Context

The operator's ask: capture (still / snip / video / session / guide)
should carry a workflow that **guides me to effortlessly save it, properly
organized** in the library for editing in wiredFRONT — and **"anything via
pixvix needs to allow the inbox/library not become a pure dumping ground of
raw and be organized there."**

Today only the guide vehicle is guided; every other vehicle auto-files
SILENTLY as `pix.png` / `vix.mp4` / `session-<stamp>` into a flat
`content/inbox` — the dumping ground the operator is calling out. W9 makes
capture→save→organize **consistent, guided, and organized** across all
vehicles behind one operator-toggled Landing Policy and one reusable card.

Gate rows moved (`wiredfront-experience.md` §5): **#2** media-bin
thumbnails / organize, **#13** Library→editor, **#18** signals-live.

## 2. Current state (ground-truthed 2026-07-26)

The capture spine — `crates/viziwiz/src/`:

- **One inbox for everything**: `Viziwiz::inbox()` = `content_root()/inbox`
  — `viziwiz/src/lib.rs:261-265`. `content_root()` is Ring 2 —
  `engine-capture/src/lib.rs:114-131`.
- **One write-through seam** `index_and_signal()` — `viziwiz/src/lib.rs:196-219`:
  sidecar + `dpl_index` upsert + `CaptureEvent::Saved` on success (`:205`),
  `Failed` on error (`:215`). Thin builders: `index_capture` (image,
  `:223-227`), `index_guide` (guide, `:232-241`), `index_video` (video,
  `:245-259`), `index_session` (session, `:821-828`).
- **The inconsistency (the target list):**
  1. **Save card vs silent dump.** Only Steps/guide shows a guided pending
     card (`palette.rs:478-611`, `lib.rs:483-517`). Full, Window, Region,
     Free, Re-snip, Video, Session **auto-file silently** (`lib.rs:548,
     876-889, 896-915`; `editor.rs:384-421`) — no name, tag, or route.
  2. **Names are fixed stems.** stills → `pix` (`lib.rs:544`), video →
     `vix` (`lib.rs:293`), session → `session-<stamp>` (`session.rs:177`).
     Only guide seeds a real name from the window title (`seed_name`
     `lib.rs:1163`).
  3. **Tags/destination only exist for guides** — `index_guide` sets
     `tags`+`show_id`/`episode_folder` (`lib.rs:232-241`); the other
     builders set none, so stills/video/session land untagged, unroutable.
  4. **Kind is a scattered string literal** (`"image"`/`"video"`/
     `"session"`/`"guide"`, plus browser's `"web"`) at `lib.rs:223/245/821`
     — no shared type; a new vehicle can drift.
- **The editor auto-picks-up** anything indexed+signaled: composer FOOTAGE
  = `Index::recent(500,None)` filtered image/video — `composer/src/lib.rs:958-966`;
  `drain_signals` invalidates caches on `Saved`/`InboxChanged` —
  `composer/src/lib.rs:1402-1420`. So organization is the gap, not plumbing.
- **Index `Record`** — the 15-field shape, `index/src/lib.rs:29-59`; carries
  `name`, `kind`, `location`, `tags`, `show_id`, `episode_folder`,
  `target`. `record_for_capture` defaults `location:"inbox"` (`:886-894`,
  `:811`). Organization can be index-first (name/tags/target) since the
  library is index-driven.
- **Persisted operator state** convention: `.userdata/` (Ring 0) —
  `engine-capture/src/lib.rs:201`; editor already persists
  `composer-state.json` (r1z). Landing Policy follows that convention.

## 3. Target design

**3a. `LandingPolicy` (new typed module — the extensibility + clean-code core).**
A small crate-local module (candidate home: `crates/viziwiz/src/landing.rs`,
or a shared `crates/fabric`-adjacent module if W10 will share the type —
decided in §4 step 1):

```
enum CaptureKind { Image, Video, Session, Guide }   // replaces the string literals
enum LandingMode { Auto, Ask }
struct LandingPattern { name_template: String, destination: Destination,
                        tags: Vec<String> }
struct LandingPolicy  { mode: LandingMode, pattern: LandingPattern }
```

- Resolved through `policy_for(kind) -> &LandingPolicy` that returns the
  GLOBAL policy today; the keyed signature is the growth seam to
  per-vehicle later (charter §3).
- `name_template` expands tokens (`{app}`, `{title}`, `{seq}`, `{date}`)
  to a meaningful, kebabed, clamped name — generalizes the existing
  `seed_name` (`lib.rs:1163`) so EVERY vehicle names like the guide does.
- Persisted to `.userdata/landing-policy.json`; loaded at ViziWiz
  construction (`Viziwiz::new`, `lib.rs:132`).

**3b. `LandingCard` (one reusable component).** Generalize the bespoke
guide `save_card` (`palette.rs:528`) into a policy-driven card taking
`(&mut PendingLanding)` and returning Save/Discard actions. Used by every
vehicle. In **Ask** mode it renders pre-filled from the pattern; in **Auto**
mode it is never drawn — the policy applies headlessly.

**3c. The unified landing path.** Every vehicle, on capture-complete,
builds a `PendingLanding { kind, temp_path, seeded: from policy }` and
calls ONE `land(pending)`:
- Auto → apply pattern, file to the organized name/target, `index_and_signal`.
- Ask → hold pending, show `LandingCard`; Save applies + files + signals,
  Discard deletes. (This is exactly today's guide pending flow, widened.)
- Organized filing lands on disk as **real folders**:
  `content/inbox/<bucket>/<kind>/<name>.<ext>` — by bucket (working-set)
  then by kind (image/video/session/guide from `CaptureKind`), meaningful
  name from the template. The `Record` also carries `tags` + the bucket
  (index-side grouping mirrors the folders).

**3d. Organization taxonomy (RESOLVED — operator, 2026-07-26).**
- **Disk layout**: folders — `content/inbox/<bucket>/<kind>/<name>.<ext>`.
  `<bucket>` = the current WORKING-SET (operator-named); `<kind>` =
  `image` | `video` | `session` | `guide` (the `CaptureKind` folder);
  `<name>` from `name_template` (default `"{app|kind}-{seq}"`), kebabed,
  never-clobber.
- **Bucket = working-set with SUGGESTED seed values.** The bucket picker
  offers a starter list (any new name is just a folder, kebab-forced —
  the X3 metadata law). Suggested seeds (operator-adjustable):
  the three show categories `devpulse-news` · `eons-of-stupidity` ·
  `effingai` (reusing `SHOW_SUGGESTIONS`, `workbench/src/lib.rs:110`),
  plus a dated `shoot-YYYYMMDD` and a `scratch` catch-all. The active
  bucket persists in the Landing Policy so a capture run stays grouped.
- **Kind folders** come from the `CaptureKind` enum (§3a) — one source of
  truth, so a new vehicle can't invent a stray folder name.
- The library/gallery groups by bucket→kind off the index, mirroring the
  disk tree.

## 4. Changes, in execution order

1. **Decide the module home** (viziwiz-local vs shared for W10) and add
   `landing.rs`: `CaptureKind`, `LandingMode`, `LandingPattern`,
   `Destination`, `LandingPolicy`, `policy_for`, token expansion, JSON
   load/save under `.userdata/`. Unit tests: template expansion, kebab/
   clamp, round-trip persistence.
2. **Migrate kind literals** to `CaptureKind` at the four index builders
   (`lib.rs:223/232/245/821`) — mechanical, compile-enforced.
3. **`PendingLanding` + `land()`** in viziwiz: one path, Auto/Ask branch;
   wraps the existing `index_and_signal` (`lib.rs:196-219`).
4. **Generalize the card**: `save_card` → `LandingCard(&mut PendingLanding)`
   in `palette.rs`; keep the guide's strict-metadata behavior as the
   `Guide` kind's policy defaults.
5. **Route every vehicle through `land()`**: Full/Window/Region/Free/
   Re-snip (`lib.rs:537-635`, `editor.rs:379-421`), Video (`lib.rs:876-889`),
   Session (`lib.rs:896-915`), Guide (fold existing pending into the shared
   path). Stills also gain a `Started` emit for feed symmetry.
6. **Policy toggle + pattern UI**: a small settings surface in the palette
   (mode toggle + pattern fields) writing `.userdata/landing-policy.json`.
   Tooltips + full-card hit targets (gate laws).
7. **Organized library grouping**: ensure the wiredFRONT library/gallery
   groups by bucket/kind/tags from the index (read-side; composer FOOTAGE/
   gallery already index-driven — add grouping, no new scan).

## 5. Boundaries (what W9 does NOT touch)

- **No assets/-ring migration.** Intake stays `content/inbox` (charter §7).
- **No editor-intake change.** FOOTAGE stays index-driven; auto-pickup
  already works (`composer/src/lib.rs:1402-1420`).
- **Browser capture on-spine = W9.1 fast-follow**, not core W9 (it writes
  `assets/captures/web/<job>`, no index/signal — `browser_cli.rs:86-98`).
  Bringing it through `import_file`/`land()` is scoped separately.
- **No Workbench work** — that is W10.
- **No sources mutated**; captures COPY; no `#[allow]`.

## 6. Verification

- `cargo green` (fmt + clippy -D warnings + tests + release exe).
- Unit: landing template expansion, kebab/clamp, policy round-trip; kind
  enum exhaustiveness at the builders.
- Drills (real exe, SEQUENTIAL): (a) `--pixvix-snap` in Auto mode →
  organized name lands, indexed, `Saved` on `/signals`, appears in FOOTAGE;
  (b) Ask mode → card pre-filled, Save files organized, Discard leaves
  nothing; (c) a session + a video through `land()` land organized; (d) the
  guide vehicle still enforces strict metadata.
- Gate proof: library/gallery shows an ORGANIZED grouped view (not a flat
  pix/pix-2 pile) — proof-shots at REAL window sizes (`--window WxH`), not
  only maximized.
- Confirm §5 rows #2/#13/#18 move.

## 7. Commit plan

Branch `feat/w9-capture-landing`. Commits: (1) `landing.rs` model + tests;
(2) kind-enum migration; (3) `land()` + unified path; (4) `LandingCard`
generalization + vehicle routing; (5) policy toggle/pattern UI + organized
grouping. PR → gate → merge → exe rebuild → STATE.

## 8. Evidence

_(Filled at execution on the Windows checkout `C:\localdev\devpulselabs` —
this Linux copy is not the release/exe environment. `cargo green`, the
sequential exe drills, and the real-window-size proof-shots run there;
STATE.md updated on land.)_
