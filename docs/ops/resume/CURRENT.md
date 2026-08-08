# Resume pack — CURRENT

**Updated:** 2026-08-08
**Lanes:** coach-seam (this session) · ai-layer (2026-08-03 state below;
its "uncommitted seam" note is stale — the tree is clean of it as of #212)
**Dirty:** false. (`editor-presets-mock.html` + `workbench-editor.md`
edits are the operator's own parallel work — do not touch.)

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

**Next (coach):** S2 protocol rehoming → S3 profile registry → S4
Blender pack → S5 Optimize cards (+ DataFusion lens when its arrow-59
release ships).

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
