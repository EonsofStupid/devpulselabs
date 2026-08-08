# Journal — 2026-08-08: S1 tracing spine landed · S3 identity design ratified for execution

## Where the lane stands

The seam ADR (`docs/plans/coach-seam-adr.md`) was **ratified** this
session (operator: "so lets plan and implement"; §6 defaults stand until
individually overruled). One amendment was made with a named cause
(§10): DataFusion's stable release (54.1) still pins arrow 58 while the
ledger is on arrow/parquet 59 — pulling it now means a second arrow in
the tree for a proof the in-tree pure-Rust reader already serves. The
DataFusion lens waits for an arrow-59 release (upstream merged 2026-07)
or S5, whichever first. Files stay Arrow/Parquet; the ruling's substance
is unchanged.

## S1 — the tracing spine, LANDED (#225, double-green, squash-merged)

Plan + evidence: `docs/plans/coach-seam-s1-tracing-spine.md`.

- **`dpl-fabric`**: new `seam` domain — `SeamSpan` v1 (verb, app, rung
  R0–R3, `downgraded_from`, mechanism, agency, ok, effect_observed,
  duration_ms, lesson/beat from day one, open attrs map per the §7
  evolution contract). Refusals are first-class `SeamEvent::Refused`
  facts, not missing rows.
- **`dpl-ledger`**: seam projection with `type`=verb (reliability per
  verb is a plain GROUP BY), `seam_reliability` fold, `read_all_rows`,
  and two examples: `seam_reliability` (the interim lens) and
  `dump_rows` (the eyeball drill).
- **`dpl-coach-daemon`**: owns a daemon-local signal fabric + ledger tap
  (`--analytics-root` → `DPL_ANALYTICS_ROOT` → `USERDATA_ROOT` →
  `./.userdata/analytics`, resolved root logged loudly at boot — silent
  non-recording is how weeks of verb stats were lost). Every `/verb`,
  `/import`, `/resolve`, `/invoke` execution AND refusal is timed and
  published; 250 ms drain task; flush on SIGINT.
- **Live proof** (this seat, DISPLAY=:1): gnome-calculator driven
  through open → focus → move → type_text("7") → assert_state → scroll →
  drag(refused) → close; 8 `seam-*.parquet` rows landed; the scroll row
  carries `rung:r2, downgraded_from:r1, mechanism:xdotool_wheel` — the
  transport ladder's first live downgrade, visible as data instead of a
  claim.
- `architecture.md` coach-daemon entry amended (fabric+ledger edges)
  in the same commit; full `cargo green` gate before push; both estate
  lanes green before merge.

## The next ruling: no more hardcoding — identity resolved, never assumed

Operator (2026-08-08): build the UI that selects an application and
builds its fabric, starting with Blender, as a **truly app-agnostic
system** — "trigger off the actual app being scanned, alias to align to
our source of truth."

Ground truth (explore agent over the coach crates): today the hub is one
route (Pick → Import → mint → in-memory `FabricSession`); apps are keyed
by `.desktop` basename (`DesktopId`); the minted fabric dies with the
process; seed logic hardcodes "cursor"/"calculator" strings; **no pack,
profile, or identity persistence exists in crates** — only in the ADR.

**The design (proposed and approved for execution; operator will peer
review the work directly):**

1. **`AppId` is the source of truth** — minted once per pack
   (`org.blender.Blender`), never derived at runtime. Nothing in code
   names an app.
2. **Evidence-based identity resolver** — every scan observes a
   fingerprint bundle (desktop id, StartupWMClass, X11 WM_CLASS, AT-SPI
   name, exe basename/path, version string) and scores it against every
   installed pack's `[identity]` fingerprints/aliases. Match → the
   pack's `AppId`. No match → synthesized generic profile under
   `x.unresolved:<slug>` (ADR §7 extension namespace), promotable by the
   Foundry later. Onboarding new apps is DATA, not code.
3. **Packs are files, the registry is a directory** —
   `packs/<app-id>/pack.toml`, schema=1 + `min_seam`, unknown keys
   warn-never-fail; per-stage Foundry artifacts beside the pack make
   builds checkpointed/resumable. The registry crate takes a root
   `PathBuf` (like the ledger) and stays root-agnostic; the app passes
   its userdata root.
4. **Foundry v0 rides S1** — census → passive fabric (existing Tier A
   mint) → verb probing (existing runtime verbs, every probe emitting a
   seam span) → pack induction → verification battery → publish on
   green. Pack reliability numbers come from the ledger, not claims.
5. **UI (mock stages 1–3 made real)** — Pick list gains "Build fabric
   (Foundry)" beside Import; a Foundry console (stage checklist, live
   progress, honest refusals); a Profile card once a pack exists.

**Blender honesty:** custom OpenGL UI → AT-SPI reads thin. That is WHY
it is Tier B: its real rail is R0 (Blender Python API), a later slice.
Foundry v0 against Blender proves identity, lifecycle, window and input
verbs and RECORDS the thin a11y in the pack rather than pretending.

**Slices:**
- **S3a — identity spine + pack schema/registry** (new crate
  `dpl-coach-pack`): `AppId`, fingerprint bundles, resolver, TOML
  loader/writer, generic-profile synthesis, promote-only law in types.
  Unit-testable without X11. Architecture.md + xtask amended in-commit.
- **S3b — Foundry v0 + hub UI**: staged checkpointed job (thread +
  progress channel like mint), console + profile card surfaces, spans
  per probe.
- **S4 — Blender first pack**: battery run on this seat, published on
  green.

## Review contract (operator ruling, this session)

Operator compacts; agent works S3a fully through; operator performs a
**direct peer review** to confirm the build is on track, tested, and
progress is real. Every slice therefore lands with: its plan doc
(7 sections + evidence), unit tests, a live drill where applicable,
`cargo green`, PR on double-green, and STATE/journal updates — evidence
first, claims second.

## Process notes

- The operator works parallel lanes in this checkout (branch
  `feat/coldlight-53-hues` active at journal time;
  `editor-presets-mock.html` + `workbench-editor.md` dirt is theirs) —
  do not touch, do not steal lanes; S3a branches from `main`.
- Stale CI-watch notifications after an interrupted `gh pr checks
  --watch` are noise once the merge is done; verify with a fresh
  `gh pr checks` instead of re-watching.
