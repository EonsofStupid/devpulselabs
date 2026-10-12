# ADR — The Coach Seam (app-agnostic teach-and-drive, one protocol)

**Status:** DRAFT 2026-08-07 — mock rev 3 approved (operator `go`); awaiting
ratification of this ADR. §6 answers the mock's five red-pen questions as
**proposed defaults** — each is overrulable in review. §7 is the evolution
contract (operator ruling 2026-08-07: this ADR is the starting point,
"built with 10x foresight and scale" — every surface open for addition
without a breaking change).
**Related:** [`coach-operator-verb-catalog-adr.md`](coach-operator-verb-catalog-adr.md) ·
[`coach-overlay-adr.md`](coach-overlay-adr.md) ·
[`eguipilotai.md`](eguipilotai.md) ·
[`docs-history-2026-10-12:docs/plans/signal-ledger-and-lesson-foundation.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/signal-ledger-and-lesson-foundation.md) ·
[`../research/coach/seam-real-world-challenge.md`](../research/coach/seam-real-world-challenge.md) ·
[`../design/mocks/coach-seam-mock.html`](../design/mocks/coach-seam-mock.html) (rev 3, 7 stages) ·
[`../journal/2026-08-07-coach-seam-audit.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/journal/2026-08-07-coach-seam-audit.md)

## 1. Context

The 2026-08-07 audit found the coach lane had grown a **second agent
surface**: `dpl-coach-daemon` speaks its own verb enum over Axum HTTP/SSE,
while `egui-pilot-ai-protocol` (v2.6, ground-truthed in-session) already owns
the control vocabulary — `Locator`, `Action`, `Role`/`NodeState`/`Rect`,
`CoachCommand`/`CoachEvent`, a typed capability catalog with
`coach.desktop-overlay` honestly unsupported. C2s §3.4 mandates foreign
perception emit the SAME shape "with no new agent surface". The daemon
violates that; its ~2,500-line runtime engine is transport-agnostic and
survives; the ~800-line HTTP wrapper is the part that demotes.

Two operator-mandated foundations were also skipped and are repaired here:
**tracing** (the verb battery proved 25 verbs and captured zero reliability
stats) and **per-app profiles** (sessions produced no reusable pack).

The verb *content* is not in question — the 101 catalog
(`coach-operator-verb-catalog-adr.md`, RATIFIED 2026-08-01) keeps its names,
targets, and effect contracts. This ADR moves its **home** and binds the
missing foundations around it.

## 2. Decision — one voice

1. **The seam vocabulary rehomes into `egui-pilot-ai-protocol`.** A `seam`
   module (same crate, same version discipline, no egui/OS deps) adds the
   foreign-target arms:
   - New ids via the existing `string_id!` macro: `AppId`, `PackId`,
     `AnchorId`.
   - `SeamVerb` — a typed enum whose wire names are exactly the ratified
     101 catalog (`snake_case` preserved: `open`, `close`, `focus`, `move`,
     `resize`, `type_text`, `press_key`, `select`, `scroll`, `invoke`,
     `resolve`, `import`, `inventory`, `presence`, `mount`, `wait_until`,
     `assert_state`, `observe`, …). No renames — scenario files and the
     Teacher already speak these.
   - `SeamTarget` — fabric anchor (`AnchorId` fingerprint), app (`AppId`),
     or window handle; presence-gated exactly as the catalog ADR rules.
   - `EffectContract` — `effect_observed: bool` + typed before/after
     observables (text, rect, focus, selection, running). Every mutating
     verb returns one.
   - `SeamEvent` — the normalized fact stream (`moved`, `resized`,
     `focused`, `text_changed`, `state_changed`, `layout_changed`) with a
     `downgraded_from` field when the transport ladder fell through.
   - Foreign trees reuse `Role`, `NodeState`, `Rect`, `Locator` — one
     semantic vocabulary whether the node is egui-native or AT-SPI-foreign.
   - The capability catalog grows `seam.*` entries (`seam.verb`,
     `seam.events`, `seam.pack-attach`), each Supported/Unsupported with
     evidence, same honesty discipline as `coach.desktop-overlay`.
2. **The daemon demotes to a devtools adapter.** `dpl-coach-daemon` keeps
   its runtime engine and becomes a translation layer: HTTP/SSE in,
   protocol types through. Its private verb enum is deleted. It stays
   feature-gated devtools-only, same law as EguiPilot's inherited pipe and
   `dpl-coach-overlay` — normal production builds contain no endpoint.
3. **The transport ladder is resolved per verb, never per app:**
   - **R0 — native API**, only when a pack declares one (Blender `bpy`
     socket addon, VS Code command bridge, SolidWorks COM add-in).
   - **R1 — a11y semantic action** (AT-SPI2 now; UIA on the Windows lane).
   - **R2 — targeted input injection**, behind an `Injector` trait
     (X11/XTEST now; Wayland portal + libei later — the trait is the law,
     the backend is a detail).
   - **R3 — vision-grounded** — import/harvest/proof fuel ONLY, never the
     lesson hot path. Screenshots remain banned from teaching decisions.
   Downgrades are first-class events, not silent fallbacks.

## 3. AppProfile packs — the config series

One TOML per app cell, the declarative unit the Foundry builds and the
registry distributes:

```toml
[identity]           # app id, exe fingerprints, version detection
[launch]             # spawn spec, ready oracle, scratch-profile flags
[ladder]             # per-verb rung promotions (R0 socket for bpy.ops, ...)
[quirks]             # a11y flags (e.g. --force-renderer-accessibility)
[[anchor]]           # FINGERPRINT BUNDLES: role + tree-path + relations +
                     # a11y-id + action set; name is one weighted signal,
                     # never the anchor (locale law)
[[verb]]             # capability matrix: verb, rung, oracle, reliability
                     # score from the last battery run
[teaching]           # landmarks, lesson seeds, consent notes
[revalidation]       # on-attach census policy + degradation thresholds
[signing]            # pack hash, adapter hashes pinned, signature
```

Laws: an **unprofiled app synthesizes a generic profile** (a11y + injection
only) — the seam never refuses to look. A profile can only **promote** verbs
above the default ladder; it can never bypass presence or effect gates.
Data-only packs where possible; packs carrying code (addons/bridges) are
signed with hashes pinned in the manifest.

## 4. The Fabric Foundry — build once, offer to users

The pipeline the "Building fabric…" console runs (challenge note is the
research foundation):

```
census → passive fabric (fingerprint anchors)
→ active exploration (sandboxed, checkpointed, crash-resume)
→ verb probing with oracles (reliability per verb)
→ pack induction
→ VERIFICATION BATTERY (ship gate)
→ fingerprint + sign → registry publish
```

Binding laws: **checkpoint to disk** (a job that loses hour three is a
pipeline nobody re-runs) · **sandbox law** (scratch profile, op whitelist,
revert-or-restart between probes) · **battery green or no publish**, battery
re-runs on app update · on the user's machine the pack **revalidates on
attach** and degrades honestly ("built on 4.2-en, you run 4.5-de, 87%
anchors verified") · the matrix (app × version × OS × locale) is paid per
cell — the automated pipeline is the asset, not any one pack. **Blender is
the ratified first Tier B pack** (locale-stable `bpy.ops`, deterministic
enumeration).

## 5. The tracing spine — the `seam` fabric domain

- A new **`seam` domain** is stamped on the fabric like every existing
  domain (`capture`, `jobs`, `providers`, `build`, `coach`, `intent`).
  `dpl-ledger` already drains every stamped domain into Hive-partitioned
  Parquet — the sink requires **zero new infrastructure**, one new enum.
- Span shape follows OTel GenAI `execute_tool` semantics: verb, target,
  transport rung, `downgraded_from`, `effect_observed`, duration,
  lesson/beat ids when teaching. Every verb execution emits one — the
  battery's zero-stats failure can never recur.
- **Arrow/DataFusion over Parquet is the lens** (operator ruling
  2026-08-07, ADR-014-consistent; DuckDB reconsidered only on a concrete
  need Parquet cannot serve). One engine means seam spans JOIN product
  signals: "which verbs are least reliable on Blender 4.5" and "which
  lesson beats stall operators" is one query. Evidence frames stay on disk
  referenced by span id.
- **The closed loop (§9.1) is the product shape:** Optimize cards (mock
  stage 7) are one query + one button — re-run Foundry probe, review the
  stalling beat, re-run the battery. The button's run emits new spans,
  which update the card. Never a passive dashboard.

## 6. The five mock questions — proposed rulings

| # | Question | Proposed default |
|---|---|---|
| 1 | Where does the Profile card live? | **Coach hub** shows the operator-facing card (tier, battery state, pack id); full TOML detail under Advanced |
| 2 | Verb console visibility? | **Devtools/Advanced only** — it is a probe surface, not a product surface |
| 3 | Teacher consent granularity? | **Per lesson**, with an always-visible kill switch; standing per-app grants only via an explicit Advanced setting |
| 4 | Show the fingerprinted pack id on the card? | **Yes** — honesty over cleanliness; the id is what degradation reports cite |
| 5 | Lesson dock relocation? | **Only when overlapping** the action target — unprompted motion is noise |

## 7. Evolution contract — authored for 10x

This ADR is a **starting point by design** (operator ruling 2026-08-07:
"built with 10x foresight and scale"). Foresight here means every surface
is open for addition without a breaking change — NOT speculative features
built early. The mechanisms, each binding:

1. **The wire law is additive-only.** New fields arrive with
   `#[serde(default)]` + `skip_serializing_if`; old readers skip what they
   do not know; nothing is renamed or repurposed. This is already the
   house pattern — `RunManifest` proves pre-experiment bundles still read
   — and the `seam` module inherits it as law, with a compat test per
   addition.
2. **Verbs grow without protocol surgery.** Ratified verbs are typed
   `SeamVerb` variants. New/experimental verbs ride a single
   `Extension { id, params }` variant under an `x.`-namespaced id
   (`x.blender.set-mode`), declared by a pack, capability-gated, and
   refused by name where unsupported. Ratification **promotes** an `x.`
   verb to a typed variant with its wire name kept; the extension arm is
   the incubator, never the permanent home. No bare strings for identity:
   extension ids are a newtype with a validated namespace.
3. **Effect contracts carry an open observable map.** The typed core
   (text, rect, focus, selection, running) stays fixed; packs may declare
   additional named observables (e.g. Blender's active mode) that ride a
   typed key→value map in the same contract. Oracles can gate on them
   without an ADR per observable.
4. **Packs are schema-versioned and tolerant.** Every pack declares
   `schema = 1` and `min_seam` (the protocol version it needs). Unknown
   keys WARN and are preserved, never fail — a newer pack degrades
   honestly on an older seam instead of refusing to parse. The cell key
   (app × version × OS × locale) is first-class in the pack id from day
   one, so multi-cell was never a retrofit.
5. **Spans carry a version and an open attribute map.** The typed columns
   (§5) are the query surface; a `seam.attrs` map takes new dimensions
   without schema surgery. Parquet + DataFusion schema-merge means old
   and new segments answer one query — analytics never forces a
   migration.
6. **Optimize cards are data, not code.** A card is a declaration: query +
   threshold + the action verb its button mounts. The three launch cards
   (§5) are the first entries in a card registry; the 10th card is an
   authored file, not a code pass.
7. **The registry is content-addressed and signed from day one.** Local
   directory now; a remote index later reuses the identical layout (pack
   hash = identity), so "offer packs to users" is a transport decision,
   not a format change.
8. **Transports slot in behind traits.** A new injection backend (portal +
   libei, UIA, a game-safe coach-marks-only mode) implements the
   `Injector`/rung trait and slots into an existing rung; new rung
   classes append after R3 without renumbering.
9. **Scale posture.** Snapshots stay bounded (the protocol's budget
   discipline extends to foreign trees — Blender's tree is huge); the
   ledger partitions by date + domain and never rewrites; Foundry cells
   are independent jobs, parallelizable across the estate's runners when
   the matrix grows.

Anti-law, stated to keep §7 honest: no speculative implementation. The
seams above cost their design now and near-zero code; anything beyond
(remote registry service, UIA lane, games mode) waits for its own pass.

## 8. Delivery slices (in order, each gated by `cargo green` + real proof)

1. **S1 — tracing spine.** `seam` domain + span schema stamped on the
   fabric; ledger writes it; one DataFusion query proven over real rows.
   First because every later slice must land already-instrumented.
2. **S2 — protocol rehoming.** `seam` module in `egui-pilot-ai-protocol`;
   daemon's verb enum deleted, daemon translates to protocol types;
   capability entries added; existing verb battery re-run green over the
   new shape (now WITH reliability stats, per S1). **LANDED 2026-08-11**
   (plan `docs-history-2026-10-12:docs/plans/coach-seam-s2-protocol-rehoming.md`): protocol v2.7 ships
   `seam::{AppId, CellKey, PackId, SeamVerb, SeamTarget, TransportRung,
   EffectContract, SeamEvent}` + the 21-verb catalog (`geom` cataloged);
   the identity spine MOVED out of `dpl-coach-pack` (no shims); the
   daemon parses verbs at the HTTP edge and answers typed contracts; the
   battery is a committed drill (`coach-daemon --example verb_battery`,
   report `docs/proof/coach-verb-battery/REPORT-2026-08-11-s2.md`, 25
   PASS / 1 SOFT / 0 FAIL) and the S1 reliability fold reads the new
   spans unchanged. One named deviation: `dpl-fabric`'s `TransportRung`
   copy stays (its file was held by a parallel operator lane); a daemon
   test pins the two enums to one wire vocabulary until the retirement
   micro-slice.
3. **S3 — profile registry.** TOML schema + loader, synthesized generic
   profile, promote-only law enforced in types.
4. **S4 — Blender pack.** Foundry pipeline v0 (census → anchors → probe →
   battery), checkpoint/resume proven by killing it mid-run; first pack
   published on battery green.
5. **S5 — Optimize cards.** The three card queries from mock stage 7 wired
   to real ledger data, each with its mounting button — as registry
   entries per §7.6, so the card set is open from the first card.

Each slice implements its slice of §7 as it lands (S1 carries the span
attrs map + version; S2 the wire law + extension verb arm; S3 pack schema
versioning; S4 the content-addressed registry layout; S5 cards-as-data).

## 9. Evidence

- Protocol ground-truth: `crates/egui-pilot-ai-protocol/src/lib.rs` read in
  full 2026-08-07 (v2.6; `string_id!`, `Locator`, `Action`, coach router,
  capability catalog).
- Ledger ground-truth: `crates/ledger/src/lib.rs` — domain-keyed Parquet
  segments, drains every stamped domain; `seam` rides for free.
- Audit: `docs-history-2026-10-12:docs/journal/2026-08-07-coach-seam-audit.md` (three parallel
  investigations, C2s §3.4 violation confirmed).
- Challenge: `docs/research/coach/seam-real-world-challenge.md` (five gates,
  Foundry pipeline, trace sink ruling).
- Mock: `coach-seam-mock.html` rev 3, browser-verified 2026-08-07, all
  seven stages interactive.
- Verb catalog: RATIFIED 2026-08-01; names and effect contracts carried
  verbatim, home moved.

## 10. Ratification

**RATIFIED** 2026-08-07 by operator (`so lets plan and implement` after the
rev 3 mock and the §7 evolution contract). §6 defaults stand until
individually overruled.

**Amendment 2026-08-07 (S1, named cause):** DataFusion's current stable
release (54.1) pins arrow 58 while `dpl-ledger` is on arrow/parquet 59 —
adding it today means a second arrow in the tree for a proof the in-tree
pure-Rust Parquet reader already serves. S1 therefore proves the spine and
the reliability-per-verb aggregation with the in-tree reader; the
**DataFusion lens lands as its own step when DataFusion tracks arrow 59**
(upstream merged 2026-07; unreleased) or at S5, whichever comes first. The
files stay Arrow/Parquet — the ruling's substance is unchanged.
