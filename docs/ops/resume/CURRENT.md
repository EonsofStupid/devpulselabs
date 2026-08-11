# Resume pack — CURRENT

**Updated:** 2026-08-11 (**S2 protocol rehoming LANDED** — operator
`gfo` approved the plan; built in an ephemeral worktree
`../devpulselabs-s2` on `feat/coach-seam-s2-protocol` so the operator's
live lane stayed untouched. Journal:
`docs/journal/2026-08-11-coach-seam-s2-landed.md`.)
**Lanes:** coach-seam (this session) · ai-layer (2026-08-03 state below;
its "uncommitted seam" note is stale — the tree is clean of it as of
#212) · operator live lane `feat/coldlight-53-hues` — as of 2026-08-11
it holds uncommitted edits to `crates/{app,fabric,ledger,seam}` — DO
NOT TOUCH those files; this is WHY S2 shipped the §4.1 rung fallback.
**Dirty (operator checkout):** `docs/ops/signals.jsonl` has a few
uncommitted operator-lane lines — not ours, leave them.

## Session 2026-08-11 — S2 protocol rehoming LANDED

Plan `docs/plans/coach-seam-s2-protocol-rehoming.md` (approved `gfo`),
§8 evidence filled in-plan. The short shape:

- **Protocol v2.6 → v2.7**, new `egui_pilot_ai_protocol::seam`: the
  identity spine (`AppId`/`CellKey`/`PackId`, parse law intact — wire
  Deserialize goes through `parse`) MOVED from `dpl-coach-pack` (its
  `id.rs` deleted, no shims); typed `SeamVerb` catalog — 21 entries
  (`geom` cataloged honestly, `window_geom` a parse alias, `x.*`
  extension arm); `SeamTarget`; `TransportRung` `r0`..`r3`;
  `EffectContract` (typed observables + open `extra` map);
  `SeamEvent` fact stream; three `seam.*` capabilities registered
  honest-`Unsupported` on the pilot pipe (testing-vs-coaching
  separation is now VISIBLE in the capability catalog).
- **Daemon = translator**: verb strings die at the HTTP edge
  (`SeamVerb::parse`, unknown names 409 by name), responses are typed
  `EffectContract`s, `/verbs` derives from the protocol catalog,
  `/events` emits typed `SeamEvent`s. `coach-runtime/src/verbs.rs`
  retired — a census test in the protocol crate proves the absorption
  kept every 2026-08-01 row (name/status/target identical).
- **Named deviation (plan §4.1 landing note):** `dpl-fabric`'s
  `TransportRung` copy SURVIVES — `crates/fabric/src/lib.rs` sits
  uncommitted-dirty in the operator's live coldlight lane (lane law).
  The daemon's `protocol_and_fabric_rung_labels_are_one_vocabulary`
  test + a `fabric_rung` mapper pin the two enums to one wire
  vocabulary. **Follow-up micro-slice owed:** retire the fabric copy
  (and that test) once the coldlight lane lands — pack.rs's l.16
  promise is half-kept until then.
- **Battery is a committed drill now**:
  `cargo run -p dpl-coach-daemon --example verb_battery` (ureq
  dev-dep). Live on this seat (DISPLAY=:1, gnome-calculator): **25
  PASS / 1 SOFT (scroll, known limit) / 0 FAIL** →
  `docs/proof/coach-verb-battery/REPORT-2026-08-11-s2.md` (old report
  kept, Law 7). The S1 `seam_reliability` fold read the run's 44 spans
  unchanged — first battery ever WITH per-verb stats; typed refusals
  (`drag` planned, `teleport` unknown) counted as facts.
- Docs amended in-PR: catalog ADR (home moved to protocol, geom, §2.1
  superseded note), seam ADR §8.2 landing record, architecture.md
  coach-pack/runtime/daemon entries (new protocol edges named).
- Full `cargo green` (fmt+clippy+tests+structure+release+smoke).

**Next in order:** S3b — Foundry v0 + hub UI (FIRST CLICKABLE DEMO;
also carries the fabric-inspector overlay + purge/remake ops the
operator asked for) → S4 Blender pack (seat has Blender + proven
bridge) → S5 Optimize cards. Micro-slices owed: fabric rung retirement
(after coldlight lands) · overlay R0/X11 presence arm (operator's call
whether it rides S3b).

## Session 2026-08-10 — Blender R0 spike + mock rev 4 (read before S2)

**Blender R0 spike PROVEN** (operator demanded real proof, pulled
forward from S4; plan + full evidence table:
`docs/plans/coach-seam-s4-spike-blender-r0.md` §5; journal:
`docs/journal/2026-08-10-blender-r0-spike-mock-rev4.md`):
- Packaged Blender 4.0.2 (apt, this seat, DISPLAY=:1) driven live via a
  stdlib-only Python bridge loaded with `--python` — socket thread
  enqueues, `bpy.app.timers` executes on the main thread (bpy is not
  thread-safe), Blender's own handlers emit timestamped facts to JSONL.
- Numbers: ping 9 ms · `open model.blend` 16 ms RTT with
  `effect_observed:true` · select/deselect 4–21 ms, named before/after ·
  operator viewport click → **named-object fact in 109 ms, zero pixels**.
- **Named gap:** `dpl-coach-overlay` presence gate is AT-SPI-only
  (`linux_self_test.rs::ensure_target_running`); Blender is invisible to
  AT-SPI, so R0 apps can be driven+observed but NOT painted yet. Fix is
  a small follow-on: an R0/X11 presence arm — `x11_window_for_class`
  already exists in the same file. NOT yet coded (operator redirected).
- Artifacts: `docs/research/coach/spikes/blender-r0/` (bridge.py,
  drive.py, spike-events.jsonl; model.blend is untracked scratch).

**Mock rev 4** (`docs/design/mocks/coach-seam-mock.html`, operator:
"find our existing mocks and fix them and have them clearly named"):
- Three systems named in plain words at the top: **1 EguiPilot** —
  inside lane, chat/AI drives the platform itself (teach or perform
  live); **2 Fabric Foundry → the Catalog** — progress-bar build that
  generates an app's fabric into the catalog = the source-of-truth
  app-agnostic layer; **3 Coach + HUD** — outside lane at lesson time.
- **Rung realignment:** rev ≤3 said R1–R4; merged code + ratified ADR
  say **R0 native / R1 a11y / R2 injection / R3 vision**. Fixed
  everywhere (legend, per-app ladders, trace table, TOMLs, downgrades).
- **Stage 8 · Coach HUD added:** persistent floating toolbar opposite
  focus; **Ctrl+Shift+Space freeze** with visible indicator (hotkey
  actually works in the mock — browser-verified); hover chips (steps
  2/5 · points · community · avg act ms · actions · tokens) each
  tooltip-naming its REAL ledger source: coach-domain beats, S1 seam
  spans, ai-usage domain (#169). HUD is a lens over signals we already
  emit — nothing new invented for it.
- Foundry stage reframed "building X into the catalog" + cites the real
  spike numbers for Blender. Red-pen queue grew to 8: (6) HUD placement
  law — a11y focus vs pointer dwell; (7) freeze hotkey confirm; (8)
  **community score vs local-first law — needs an explicit opt-in
  ruling before anything ships.**
- Render-verified in browser (stages 1/2/8 + hotkey). Note: the mock's
  palette has no `--panel` var — HUD uses `--raised` (a transparent-HUD
  bug caught and fixed in-session).

**Operator questions answered in-session:** "am I wrong / better
solution?" — no: catalog/pack approach validated by the spike (109 ms
named-object click vs vision-first guessing); refinement = for R0 apps
the fabric IS the native API surface, the pack records which rail. The
Rust "connector" term = trait + adapter; Axum is just the daemon's HTTP
lib, not the architecture.

**Next in order:** operator red-pens mock rev 4 (esp. items 6–8) →
docs PR (mock + spike plan + journals + ops) → **S2 plan approval** →
S2 code → S3b (first clickable demo) → S4 Blender pack proper (seat now
HAS Blender + a proven bridge shape) → S5. Overlay R0 presence arm can
ride S3b or a micro-slice — operator's call.

## Coach-seam lane (session 2026-08-07) — READ FIRST if resuming coach

**Trigger:** operator stopped the lane — the CX daemon was a second agent
surface; tracing/profiles/mock were skipped. Audit confirmed. Full story:
`docs/journal/2026-08-07-coach-seam-audit.md`.

**Done this session:**
- Audit: daemon violates C2s §3.4 "no new agent surface"; runtime engine
  (~2,500 lines) survives, HTTP wrapper (~800) gets demoted/rehomed.
- Research brief: UFO² validates per-app profiles; injection must be a
  trait (x11 now, portal+libei later); OTel `execute_tool` spans → DuckDB.
- **`docs/design/mocks/coach-seam-mock.html`** authored + browser-verified,
  now rev 3 (7 stages: Pick / Foundry / Profile card / Verb console /
  Lesson / Trace ledger / **Optimize**). Stage 7 is the closed loop per
  signal-ledger §9.1: insight cards computed from `dpl-ledger` Parquet +
  seam spans via DataFusion, each carrying its provenance query and a
  one-click action (re-run Foundry probe / review beat / re-run battery) —
  never a passive dashboard. UNCOMMITTED — awaiting red-pen.
- Real-world challenge delivered: fingerprint anchors (locale!), sandboxed
  checkpointed builds, pack verification batteries, signed distribution,
  the version×locale matrix → the Foundry pipeline is the asset.

**Rulings 2026-08-07:** mock-first (review before seam ADR) · Blender is
the first Tier B pack.

**RATIFIED + S1 LANDED (2026-08-08):** operator ratified the ADR
(`so lets plan and implement`; §6 defaults stand until individually
overruled). §10 amendment: DataFusion dep WAITS for a release tracking
arrow 59 (stable 54.1 pins arrow 58 — no second arrow in the tree); the
in-tree pure-Rust reader is the interim lens.

**S1 — the tracing spine, LANDED** (plan + evidence:
`docs/plans/coach-seam-s1-tracing-spine.md`):
- `dpl-fabric`: `seam` domain — `SeamSpan` v1 (verb, app, rung R0–R3,
  `downgraded_from`, mechanism, agency, ok, effect_observed,
  duration_ms, lesson/beat from day one, open attrs map) +
  `SeamEvent::Refused` as a first-class fact.
- `dpl-ledger`: tap subscribes seam; `type`=verb so reliability is a
  GROUP BY; `seam_reliability` fold + `read_all_rows`; examples
  `seam_reliability` (the lens) and `dump_rows` (the eyeball drill).
- `dpl-coach-daemon`: owns a local signal fabric + ledger tap
  (`--analytics-root` → `DPL_ANALYTICS_ROOT` → `USERDATA_ROOT` →
  `./.userdata/analytics`, logged loudly at boot); every
  `/verb`+`/import`+`/resolve`+`/invoke` execution AND refusal is timed
  and published; drain task 250 ms; flush on SIGINT.
- Live-proven on gnome-calculator (DISPLAY=:1): 8 spans → Parquet; the
  scroll row landed `rung:r2, downgraded_from:r1,
  mechanism:xdotool_wheel` — the ladder downgrade is queryable data.
- `architecture.md` coach-daemon entry amended (fabric+ledger edges) in
  the same commit; `cargo green` full gate passed.

**S3a — identity spine + pack registry, LANDED 2026-08-08** (plan +
evidence: `docs/plans/coach-seam-s3a-identity-spine.md`; design text:
`docs/journal/2026-08-08-coach-seam-s1-spine-s3-design.md`):
- New egui-free crate **`dpl-coach-pack`** (workspace member, xtask
  `EGUI_FREE` census + architecture.md §3 amended in-commit; deps:
  toml + sha2 + dpl-coach-inventory; never a dep of `app`).
- `AppId` minted once, parse-validated (reverse-DNS ≥ 2 segments; `x.`
  reserved; `x.unresolved:<slug>` for strangers); `CellKey`
  (version×os×locale) first-class in `PackId` from day one.
- `FingerprintBundle::from_inventory` — honest evidence only
  (`live-only:` synthetics are NOT desktop ids; `wm_class`/version
  never invented, S3b observes them live).
- Resolver: weights desktop_id 4 · startup_wm_class/wm_class 3 ·
  atspi_name/exe_basename 2 · alias 1; accept ≥ 3.0; ties = honest
  `Unresolved`; miss synthesizes a promotable generic profile.
- Registry `packs/<app-id>/pack.toml`: schema-1 tolerant TOML (unknown
  keys warn AND survive rewrites), content-hash in `[signing]`
  (§7.7), tmp→bak→rename, `UnresolvedWrite` refused (promotion mints a
  real id first). Promote-only IN TYPES: `PromotedRung` cannot express
  r2/r3; `effective_rung` never demotes.
- Proof: 25 unit tests green; live drill on this seat resolved the
  RUNNING gnome-calculator (score 8 with live AT-SPI evidence) among
  65 real inventory rows, 64 honest `x.unresolved:` strangers; Blender
  pack written + resolved by shape (Blender NOT installed here — S4
  seat prep). Full `cargo green`.

**RULED 2026-08-08 (operator, after post-compact reground): S2 next.**
S2 plan drafted — `docs/plans/coach-seam-s2-protocol-rehoming.md` —
**halted for operator approval before code.** The reground (all key
files re-read in full) corrected the record; the corrections are IN the
plan's §1 and must not be re-lost by a future compact:
- NO "private verb enum" exists — the violation is bare verb strings
  end to end (`execute_verb(&str)` match; 20 static `VerbInfo` rows in
  `coach-runtime/src/verbs.rs`) + eight crate-local effect structs. S2
  *introduces* the typed vocabulary, it does not delete an enum.
- "The 101" is the catalog's NAME (19 shipped verbs + `drag` planned),
  never a count of 101 verbs.
- Daemon executes `geom`/`window_geom` but `/verbs` omits it — S2
  catalogs it honestly.
- The 2026-08-01 battery was MANUAL curl (no committed script; GREEN
  report at `docs/proof/coach-verb-battery/REPORT.md`); S2 commits it
  as `coach-daemon/examples/verb_battery.rs` (`ureq` dev-dep flagged
  for red pen) so the re-run is one command and — via S1 spans — yields
  per-verb reliability stats for the first time.

**S2 shape (plan §4, ADR §2.1–2.2 + §8.2):** `seam` module in
`egui-pilot-ai-protocol` (v2.6→2.7, additive): `SeamVerb` (catalog wire
names + `geom` + `Extension{x.*}` incubator), `SeamTarget`,
`SeamVerbDescriptor`/`seam_verb_catalog()` (replaces `verbs.rs`, no
shim), `TransportRung` (labels `r0`..`r3` == dpl-fabric labels,
equality-TESTED, no fabric→protocol dep edge), `EffectContract` (typed
core + open extra map), protocol `SeamEvent` fact stream
(name-collides with dpl-fabric's ledger event — accepted, two-Fabrics
precedent), `seam.*` capability entries Unsupported-with-evidence in
the pilot catalog (= testing-vs-coaching separation, visible).
Identity ids (`AppId`/`CellKey`/`PackId`/`IdError`) MOVE from
dpl-coach-pack **with parse law intact** — ADR §2.1 refinement: NOT
bare `string_id!`; Deserialize goes through `parse`. coach-pack's
`Rung` replaced by protocol `TransportRung` (TOML values unchanged —
S3a packs on disk parse unchanged, tested); `PromotedRung` stays
pack-domain. Daemon: `+protocol` dep, verb strings die at the HTTP
edge, `/verbs` derives from the protocol, responses carry
`EffectContract`, `/events` emits typed facts. Docs amended in-PR:
catalog ADR (home moved, geom, count honesty), architecture.md §3 +
xtask if censused, seam ADR one-line §2.1 refinement note.

**Then:**
1. **S3b** — Foundry v0 (census → passive fabric via existing mint →
   verb probes emitting seam spans → induction → battery → publish) +
   hub UI: "Build fabric (Foundry)" on Pick, job console, profile card.
   Hub swaps heuristic matching for `FingerprintBundle` + registry
   resolve — the hardcoded cursor/calculator seed strings die here.
   FIRST CLICKABLE DEMO lands here.
2. **S4** — Blender first pack ON THIS SEAT (install Blender first —
   not present); thin a11y recorded honestly (its real rail is R0
   Python, later slice); publish on battery green.
3. Then S5 Optimize cards (+ DataFusion lens when its arrow-59 release
   ships).

**Honesty ledger (operator review 2026-08-08):** S1+S3a are foundations
— NOTHING clickable exists yet; never call a proof example a demo. The
pre-audit waste is bounded to the ~800-line daemon HTTP wrapper (S2
repurposes it); verb engine, inventory, mint, spans, identity are all
load-bearing and merged.

Review contract: every slice lands with plan doc, tests, live drill
where applicable, `cargo green`, PR merged on double-green only,
STATE/journal updated — evidence first. Operator branch dirt
(`feat/coldlight-53-hues`, editor mock, workbench-editor plan) is NOT
ours; S3a branches from `main`.

**History — how the ADR got here:**
1. **Seam ADR** — `docs/plans/coach-seam-adr.md` (2026-08-07):
   vocabulary rehomes into `egui-pilot-ai-protocol` (`seam` module:
   `SeamVerb` keeping the ratified 101 wire names, `SeamTarget`,
   `EffectContract`, `SeamEvent` with `downgraded_from`); daemon demotes
   to devtools adapter; per-verb ladder R0–R3 with `Injector` trait;
   AppProfile TOML (promote-only law, synthesized generic profile);
   Foundry pipeline; `seam` fabric domain → `dpl-ledger` (rides free) →
   DataFusion → Optimize cards. §6 = the 5 mock questions answered as
   overrulable defaults. §7 = the EVOLUTION CONTRACT (operator ruling
   2026-08-07, "10x foresight and scale"): additive-only wire law with a
   compat test per addition · experimental verbs ride an `x.`-namespaced
   `Extension` arm, promoted to typed variants on ratification · effect
   contracts carry an open pack-declared observable map · packs declare
   `schema` + `min_seam`, unknown keys warn-never-fail, cell key
   first-class · spans carry a version + open attrs map (DataFusion
   schema-merge) · Optimize cards are DATA (query+threshold+action
   registry) · registry content-addressed + signed day one · transports
   slot behind traits, rungs append · bounded snapshots / partitioned
   ledger / parallel Foundry cells. Anti-law: no speculative
   implementation — seams cost design now, near-zero code.
   §8 = slices S1 tracing → S2 rehoming → S3 registry → S4 Blender pack →
   S5 Optimize cards, each implementing its slice of §7 as it lands.
2. Operator red-pens ADR (§6 defaults, §7 mechanisms, §8 order, naming) +
   mock rev 3 → ratify → S1.

**Resolved 2026-08-07 (operator go):** seam trace sink is
**Arrow/DataFusion over Parquet** (ADR-014-consistent). Mock rev 2 shows
the Foundry job console + DataFusion query. Challenge captured in
`docs/research/coach/seam-real-world-challenge.md`.

---

# ai-layer lane — as of 2026-08-03 (verify before trusting; #212 landed since)

## Goal

Build the **AI layer**: the persona (Clyffy), the session primitive, and the
one typed funnel every frontier provider is forced through. The Providers
program that preceded it is complete and merged.

Program map: `docs/plans/providers-presence-and-the-brain.md` — §7 Clyffy,
§8 cockpit + group chat, §9 the research council. ADRs: 014 (analytics),
017 (capability registry, amended).

## The rulings that govern this lane (operator, 2026-08-03)

1. **Chat owns its store.** A bounded context owns its datastore. Chat's
   data is currently split — `chat/provider-profiles.db` (turso, its own
   tokio runtime) AND the shared estate's `Tree::Agents`. It converges on
   ONE chat store; the platform reads the PUBLISHED fact via
   `fabric.provider_roster()`, never chat's store.
   **ADR-017 §2 is superseded** — the `Tree::Agents` persistence it
   specified was wrong, and any doc still claiming it is stale.
2. **Analytics is Arrow/DataFusion over Parquet, not DuckDB.** ADR-014
   amended. `dpl-ledger` already writes Parquet via Arrow; nothing in
   `crates/` ever linked DuckDB.
3. **Clyffy is the global/per-session face of the AI, and is ABSENT from
   group chat.** The provider backing him is VISIBLE — his font, his symbol
   carrying the provider's mark, the chat's typeface. Deliberately the
   opposite of every gateway pattern, because hiding who answered destroys
   the label the tuning loop runs on.
4. **Round-robin IS group chat** — one shared transcript, participants take
   turns, the operator in the ring. Fan-out is the research council, which
   is a different thing.
5. **The research council** — each selected provider deep-researches in its
   OWN isolated session (10–15 min); a default provider synthesises. Staging
   surfaces **disagreement FIRST** (agreement is coverage, not truth), and
   results are disclosed **as each lands**, never held for the slowest.
6. **Passes law:** every pass names the existing boundary it aligns to, and
   STOPS for the operator when it creates a new one.
7. **Method:** locate with tools, then READ THE FILE IN FULL before editing
   it. Editing around a grep hit is how drift gets introduced — it happened
   twice this session and was caught both times by reading first.

## Done — closed, nothing elided

- **The Providers program, all six links, merged.**
  - **P1** capability registry — `ProviderCapability` folded onto the fabric
    as THE ROSTER, projected onto the seam under `registry.providers`, read
    by a consumer outside the chat crates. *(Its persistence is amended per
    ruling 1 above — the `Tree::Agents` write is superseded, not the model.)*
  - **V1** Providers control + ink-bleed preset — `dwell` tier + ease +
    `ink-bleed` in `tokens/motion.json`, `brain` in `tokens/icons.json`,
    authored `bleed` preset with a build-checked hue pool.
  - **V2** roster popup — live presence per card; not-installed and
    signed-out stay distinct.
  - **V3** provider modal — live-ticking tenure; roster stays up behind it.
  - **A1** presence probes — `crates/app/src/presence.rs` is the only place
    that spawns. Measured live: codex/claude/grok SignedIn, cursor/gemini
    NotInstalled.
  - **A2** subscription sign-in — hands off to the vendor's own CLI, which
    opens the operator's DEFAULT browser; a watcher polls until the probe
    confirms. A provider with no declared sign-in verb is refused by name.
- **D1 + H1** (#180) — chat reads the episode spine; 165 local transcripts
  became real activation and clickable, readable rows.
- **`/experiments`** (#185) — experiments as domains under Workbench, one
  analyzer, no new crate. Run bundles carry an experiment/variant dimension.
- **Run-bundle contract typed** (#190) — `RunManifest` in the protocol
  crate; `WindowSize` moved there; no re-export shim.
- **Configured artifact root honoured** (#191).
- **Enter sends / Shift+Enter newlines** — built.
- **The provider RUNTIME seam** — `crates/chat/runtime/src/seam.rs`.
  `ProviderRuntime` (provider · try_send · try_event · capabilities) plus
  the neutral vocabulary; Grok aligned by translation via
  `grok/src/seam.rs`, its ACP transport untouched. 30 tests green.
  **BUILT, NOT COMMITTED.**

## Next

1. **Commit + land the provider runtime seam** (uncommitted in the tree).
2. **Land `docs/align-store-and-analytics`** — ADR-014, ADR-017 amendment,
   program map §7–§9. Committed as `ec805f9`, not pushed.
3. **Land #206** — a harness `--route` must never become the operator's
   persisted landing route. My own drills wrote `workbench/experiments` into
   the operator's estate; the fix stops new corruption, and one navigation
   clears the stale value.
4. **Codex conversation transport** — THE gap. `providers/codex` has
   `activation.rs` only: it detects the ChatGPT login and has no runtime, so
   nothing qualifies `StreamTurns` for it. Grok's `runtime.rs` is the
   template and the seam is now the target shape.
5. **The folder/layer restructure** — operator is doing this themselves.
   Finding: §4's naming law already mandates grouped folders past ~3 crates,
   and **45 crates sit flat** across four layers that each exceed it.
   `chat/*` and `hubs/*` complied; the rest predate the law. Recommendation
   is one layer per pass, smallest first (`support` → `document` →
   `surface` → `engine`), each amending architecture.md AND the xtask audit
   in the same commit.

## Key files

- `docs/plans/providers-presence-and-the-brain.md` — §7 Clyffy, §8 group
  chat, §9 research council
- `docs/design/adr-014-three-ring-root.md` · `adr-017-provider-capability-registry.md`
- `docs/design/architecture.md` — the layer law, gate-enforced
- `crates/chat/runtime/src/seam.rs` · `crates/chat/providers/grok/src/seam.rs`
- `crates/fabric/src/lib.rs` (THE ROSTER) · `crates/seam/src/lib.rs` (`/command`)
- `crates/app/src/ai_usage.rs` (usage + cost, #169)

## Known stale / deprecated — do not trust

- Any doc claiming the roster persists to `Tree::Agents` (superseded).
- Any doc naming **DuckDB** for analytics (superseded).
- `docs/plans/coach-lab-experiment-program.md` names "DuckDB baselines" —
  another workstream owns it; flagged, not edited.
- `docs/STATE.md` is 2098 lines and last updated 2026-08-01; it is an
  append-only ledger, not the resume truth. THIS file is.

## Parked lanes

- **coach-ledger** — C2f Operator Import UI (#187). Notepad on Windows
  after. CX-harvest / Pass-2 parked.
- **`chat-renderers` is dead and duplicated** — a full `EventRendererRegistry`
  nobody calls, while `chat/window::event_row` is a second exhaustive
  implementation. #169 merged MORE into the dead crate. Unresolved.
- **`chat-extensions`** — zero references.
- **Claude decoder** emits `UNKNOWN` for `last-prompt` / `ai-title` / `mode`
  / `permission-mode`, so real transcripts read noisily.
