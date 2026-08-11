# Pass — Coach Seam S2: protocol rehoming (one vocabulary under EguiPilotAI)

**Status:** DRAFT 2026-08-08 — awaiting operator approval before code.
Executes ADR §2.1–2.2 + §8.2 (`docs/plans/coach-seam-adr.md`, RATIFIED).
Operator ruling 2026-08-08: **S2 before S3b** (absorb the vocabulary first;
the hub UI lands once, on the final protocol).
**Related:** [`coach-seam-adr.md`](coach-seam-adr.md) ·
[`coach-operator-verb-catalog-adr.md`](coach-operator-verb-catalog-adr.md) ·
[`coach-seam-s1-tracing-spine.md`](coach-seam-s1-tracing-spine.md) ·
[`coach-seam-s3a-identity-spine.md`](coach-seam-s3a-identity-spine.md) ·
[`coach-c2s-foreign-target-spike.md`](coach-c2s-foreign-target-spike.md) §3.4

## 1. Context

The 2026-08-07 audit named the violation: foreign perception/actuation must
emit the shape the pilot already speaks, "with no new agent surface"
(C2s §3.4). Post-compact reground (2026-08-08, every file below read in
full) sharpened what the violation actually IS — three corrections to the
record:

1. **There is no "private verb enum" to delete.** The vocabulary is
   stringly-typed end to end: the daemon matches `body.verb: String` in
   `execute_verb(&str)`; the catalog is 20 static `VerbInfo { name: &'static
   str, .. }` rows in `coach-runtime/src/verbs.rs`. The true shape of the
   violation is **bare strings for verb identity** (breaking the workspace
   law directly) plus eight crate-local effect structs living outside the
   protocol crate. S2 therefore *introduces* the typed vocabulary that never
   existed, rather than deleting an enum.
2. **"The 101" is the catalog's name, not a count.** The ratified catalog
   (2026-08-01) holds 19 verbs + `drag` (planned). Any doc reading "101
   verbs" as a quantity is wrong.
3. **The daemon executes `geom`/`window_geom` but `/verbs` never lists it**
   (battery report proves both facts). S2 reconciles the catalog with what
   `execute_verb` actually accepts, so introspection never under-claims.

Also found: the 2026-08-01 verb battery was a **manual curl session** — no
committed script exists. Its GREEN report captured zero reliability stats
(the S1 trigger). S2 commits the battery as a repeatable drill so "re-run
green over the new shape" is one command, and S1's spans make the re-run
produce per-verb reliability stats for the first time.

## 2. Goal / non-goals

**Goal.** One seam vocabulary, typed, living in `egui-pilot-ai-protocol`,
with two capability-gated consumers cleanly separated:

- **testing/demo lane** — EguiPilot runner/CLI over the inherited pipe,
  devtools-only; does NOT execute foreign verbs (the `seam.*` capability
  entries say so honestly, same discipline as `coach.desktop-overlay`).
- **coaching lane** — coach runtime + daemon on foreign apps (devtools
  today; Teacher consent per lesson when the product surface lands).

The daemon demotes to a translator: HTTP JSON in → protocol types through →
runtime engine → protocol `EffectContract` out. Bare verb strings die at
the HTTP boundary.

**Non-goals.** No hub UI (S3b). No Foundry (S3b). No Blender pack (S4). No
Optimize cards (S5). No runtime-internal rewrite — the eight effect structs
in `coach-runtime` are implementation detail behind the seam, not wire
vocabulary; they stay (translation happens at the daemon edge). No MCP. No
Wayland/UIA. No new HTTP endpoints.

## 3. Ground truth (read in full, 2026-08-08)

- `crates/egui-pilot-ai-protocol/src/lib.rs` (2,510 lines, v2.6):
  `string_id!` macro (l.18), `Locator`/`Action` (l.754/782), coach
  touchpoint router (l.843+), `capability` module + `capability_catalog()`
  (l.1371/1435) with the Supported/Unsupported evidence discipline. Single
  file, no submodules yet. Consumers: the seven `egui-pilot-ai-*` crates
  only — **no coach crate speaks it today**; S2 adds that edge.
- `crates/coach-daemon/src/main.rs` (1,155 lines): S1 spans wired on every
  `/verb`+`/import`+`/resolve`+`/invoke` execution AND refusal;
  `execute_verb` is a `&str` match over 13 wire names (incl. `geom`|
  `window_geom` alias); refusal for unknown names is honest. Bespoke JSON
  per verb response.
- `crates/coach-runtime/src/lib.rs` (641) + `verbs.rs` (132): `VerbInfo`
  catalog (20 rows), `AgencyMode`, eight effect structs
  (`WindowVerbEffect`, `LifecycleEffect`, `TextTypeEffect`,
  `KeyPressEffect`, `SelectEffect`, `ScrollEffect`, `WaitAssertEffect`,
  `InvokeEffect`), all `effect_observed`-bearing.
- `crates/coach-pack/src/id.rs` (236): `AppId` parse-validated (reverse-DNS
  ≥ 2 segments, `x.` reserved, `x.unresolved:<slug>`), `CellKey`,
  `PackId`, `IdError`, `slugify` — **no serde**; law table in tests.
  `lib.rs` re-exports; only consumers today are its own tests/examples
  (`hubs/coach` does not dep coach-pack yet — low blast radius for the
  move).
- `dpl-fabric` seam domain (S1): `TransportRung` labels `r0`..`r3` (proven
  in Parquet rows), `SeamAgency`, `SeamSpan`, `SeamEvent::{Executed,
  Refused}`.
- Battery: `docs/proof/coach-verb-battery/REPORT.md` (25 PASS · 1 SOFT ·
  GREEN) — the wire shapes on record; **no script committed**.
- Catalog ADR: 19 shipped wire names + `drag` P2; targets desktop / app /
  window / fabric / predicate; effect-contract law §2.3.

## 4. Design

### 4.0 Law reconciliation (operator ratifies; named, not assumed)

Three house laws touch this pass and two of them counter each other.
Stated here so the pass never picks silently:

1. **AGENTS.md Law 7:** "Never delete content; **copy, never move**;
   never clobber." (Carried verbatim from the prior repo's salvage
   laws.)
2. **CLAUDE.md:** "no re-export shims **on moves**" — a law that only
   exists because code moves are expected to happen.
3. **port-capture discipline** (`.claude/skills/port-capture`): any
   surface being rehomed gets a full capability inventory + disposition
   table — port-verbatim / reauthor / retire — **no silent drops**.

**Reading RATIFIED by operator, 2026-08-10:** the shim ban means "we
are in pre-release — take the time to align and move things to their
respective boundary." No conflict: Law 7 protects *content* — operator
data, salvage sources, proof artifacts, docs — which this pass honors
(the 2026-08-01 battery REPORT.md is never deleted or overwritten; the
new run writes a NEW dated report beside it). Code rehoming is the
move law: types land at their one true boundary, every caller updates,
no compatibility shims left behind — and nothing is lost because git
history holds every prior version. The port-capture discipline is what
makes a move safe: §4.5 below is the disposition table for every
element this pass touches, so "move" is proven to mean "absorbed
whole," never "deleted and approximated."

### 4.1 `egui_pilot_ai_protocol::seam` (new `src/seam.rs`, `pub mod seam`)

Same crate, same serde discipline, still zero egui/OS deps.
`PROTOCOL_VERSION` bumps 2.6 → 2.7 (additive).

- **Identity moves, validation intact.** `AppId`, `CellKey`, `PackId`,
  `IdError`, `UNRESOLVED_PREFIX` MOVE verbatim from `dpl-coach-pack` (law
  tables move with them). **ADR §2.1 refinement, named:** the ADR (written
  pre-S3a) said "new ids via `string_id!`" — a bare transparent string
  would DELETE the parse law S3a ratified. They land as validated newtypes;
  `Serialize` is transparent, `Deserialize` goes **through `parse`** so the
  wire cannot smuggle an invalid id. `AnchorId` (ADR-named, S3b-consumed)
  lands via `string_id!` — no format law exists for it yet, so none is
  invented.
- **`SeamVerb`** — typed enum, wire names exactly the ratified catalog
  (`snake_case` kept): `inventory`, `presence`, `import`, `resolve`,
  `invoke`, `click`, `mount`, `open`, `close`, `focus`, `move`, `resize`,
  `type_text`, `press_key`, `select`, `scroll`, `drag`, `wait_until`,
  `assert_state`, `observe`, **plus `geom`** (shipped in code + battery
  since 2026-08-01, honest catalog entry now; `window_geom` accepted as a
  parse alias, never emitted). Plus the §7.2 incubator arm:
  `Extension { id: SeamExtensionId, params: HashMap<String, String> }`
  where `SeamExtensionId` is a validated newtype requiring the `x.`
  namespace. `parse`/`wire_name` round-trip; unknown names refuse by name.
- **`SeamVerbDescriptor` + `seam_verb_catalog()`** — status
  (shipped/planned) + `SeamTarget` per verb; replaces
  `coach-runtime/src/verbs.rs` (deleted, **no shim**); `/verbs` is derived
  from the protocol from now on.
- **`SeamTarget`** — `Desktop | App | Window | Fabric | Predicate` (the
  catalog's target column, typed).
- **`TransportRung`** — `R0Native | R1Semantic | R2Injection | R3Vision`,
  wire labels `r0`..`r3`. **Ownership follows the in-code commitment**
  found on the full read of `pack.rs` (l.16: "the two stay decoupled
  *until S2 rehomes both into the protocol crate*" — a promise merged
  with #228 that the first draft of this plan almost contradicted):
  the protocol crate owns the ONE `TransportRung`; **both**
  `dpl-coach-pack::Rung` and `dpl-fabric::TransportRung` are replaced
  by it. `dpl-fabric` gains the `egui-pilot-ai-protocol` dep — a NEW
  named edge amended into architecture.md §named-exceptions in the
  same commit (the protocol crate is dependency-light and egui-free;
  fabric already carries named exception edges to hub/chat-core/
  intent). `SeamSpan` then carries the protocol rung — vocabulary
  identity by construction, not by parallel-enum discipline.
  Fallback if the operator refuses the fabric edge: fabric keeps a
  private copy + a label-equality test in coach-daemon (both crates
  visible there) — honest, but leaves two enums alive; the in-code
  promise says otherwise, so the edge is the recommendation.
  `PromotedRung` + `effective_rung` stay in coach-pack — the
  promote-only law is pack-domain, not wire vocabulary.
- **`EffectContract`** (§2.1 + §7.3) — `effect_observed: bool`, typed
  before/after observables (`text`, `rect`, `focus`, `selection`,
  `running`), and the open pack-declared observable map
  (`extra: HashMap<String, String>`, additive law). Every mutating verb's
  HTTP response carries one from S2 on.
- **`SeamEvent`** — the normalized fact stream (`moved`, `resized`,
  `focused`, `text_changed`, `state_changed`, `layout_changed`) +
  `downgraded_from: Option<TransportRung>`. Name collides with
  `dpl_fabric::SeamEvent` (ledger-domain, in-process) — accepted, with
  precedent: two `Fabric` types already coexist and the daemon already
  aliases (`Fabric as SignalFabric`). The daemon qualifies both.
- **Capability entries** — `seam.verb`, `seam.events`, `seam.pack-attach`
  consts + catalog descriptors. In the PILOT's catalog they are
  **Unsupported with evidence** naming the coach daemon as the honest
  surface (exactly the `coach.desktop-overlay` pattern) — that unsupported
  entry IS the testing-vs-coaching separation made visible and queryable.

### 4.2 The daemon demotes to a translator

- `+egui-pilot-ai-protocol` dep. `body.verb` parses to `SeamVerb` at the
  HTTP edge; unknown → refused by name (same behavior, now typed).
  `execute_verb` matches on `SeamVerb` variants.
- `/verbs` derives from `seam_verb_catalog()` — one source of truth;
  `verbs.rs` deleted from coach-runtime.
- Responses carry a serialized `EffectContract` built at the daemon edge
  from the runtime's effect structs (runtime internals untouched this
  slice). ADR §8.2 sanctions the new response shape ("battery re-run green
  over the new shape") — the battery report is regenerated, not held
  wire-frozen; the HTTP plane is devtools-only.
- `/events` SSE emits typed protocol `SeamEvent` facts.
- S1 span emission is untouched (rung/agency mapping now has an equality
  test backing it).

### 4.3 `dpl-coach-pack` after the move

- `id.rs` shrinks to a re-import site — `use egui_pilot_ai_protocol::seam
  ::{AppId, CellKey, PackId, ...}` internally; the `pub use id::*` exports
  in `lib.rs` are DELETED, not aliased (**no re-export shims on moves** —
  callers import ids from the protocol crate). `slugify` moves with
  `AppId::unresolved`.
- `Rung` replaced by protocol `TransportRung` in the pack schema; TOML
  wire values `"r0"`/`"r1"` are unchanged, so **packs on disk from the S3a
  drill parse unchanged** (proof §6).

### 4.4 The battery becomes a committed drill

`crates/coach-daemon/examples/verb_battery.rs` — drives the running daemon
over localhost HTTP through the full 2026-08-01 matrix (health → verbs →
inventory → presence/open/close → window verbs → text/key/select/scroll →
import/resolve/invoke/mount → drag-refusal → close), prints PASS/SOFT/FAIL
per step and exits nonzero on FAIL. Needs a tiny HTTP client: **`ureq`
(blocking, no tokio entanglement) as a dev-dependency of coach-daemon** —
flagged for approval since it is a new dep, examples-only.

### 4.5 Disposition table — every touched element, no silent drops

port-capture discipline applied intra-repo. Source files were read IN
FULL this session (2026-08-08) before any disposition was written:
`verbs.rs` (132), runtime `lib.rs` (641), daemon `main.rs` (1,155),
`id.rs` (236), `bundle.rs` (141), `pack.rs` (792), `registry.rs` (241),
`resolve.rs` (413), protocol `lib.rs` structure + key sections.

**`coach-runtime/src/verbs.rs` → retire file, absorb everything:**

| Element | Disposition | Destination |
|---|---|---|
| `VerbInfo { name, status, target }` | reauthor | `seam::SeamVerbDescriptor { verb, status: VerbStatus, target: SeamTarget }` — every field carried, strings become types |
| all 20 catalog rows (names, statuses, targets) | port-verbatim | `seam_verb_catalog()`; §6 test asserts the new catalog ⊇ the 20 old names with identical status/target |
| `catalog_includes_window_verbs` test | reauthor | superseded by the superset test above (strictly stronger) |

**`coach-runtime/src/lib.rs` — NOTHING moves this slice** (the effect
structs are engine internals, not wire vocabulary; the daemon edge
translates). Kept as-is: `AgencyMode`, `NodeRef`, `IRect`, `OuterGeom`,
`FabricSession`, `LiveHandle`, `MountSpec`, all verb functions. The
eight effect structs' fields disposition INTO the wire contract at the
daemon edge as follows:

| Effect struct fields | EffectContract destination |
|---|---|
| `verb` (all eight) | envelope `verb` (typed `SeamVerb` wire name) |
| `effect_observed` (all) | `effect_observed` |
| `WindowVerbEffect.before/after: OuterGeom` | typed `rect` before/after |
| `LifecycleEffect.real_before/real_after` | typed `running` before/after |
| `LifecycleEffect.pid` | `extra["pid"]` |
| `TextTypeEffect`/`KeyPressEffect.display_before/after` | typed `text` before/after |
| `TextTypeEffect.text` / `KeyPressEffect.key` | `extra["text"]` / `extra["key"]` |
| `SelectEffect.state_before/after` | typed `selection` before/after |
| `SelectEffect.name` | `extra["name"]` |
| `ScrollEffect.direction/delta/via` | `extra["direction"/"delta"/"via"]` (`via` also stays the span mechanism, S1 unchanged) |
| `ScrollEffect.snapshot_before/after` | typed `text` before/after |
| `WaitAssertEffect.ok/matched/predicate/detail/real/pid` | `effect_observed`=matched; `extra["predicate"/"detail"]`; `running`=real; `extra["pid"]` |
| `InvokeEffect.action/agency/display_*` | `extra["action"]`; agency rides the span (S1); `text` before/after |
| `atspi_name` (several) | envelope `app` (unchanged from today's JSON) |

**`coach-pack/src/id.rs` → move whole, law intact:**

| Element | Disposition | Destination |
|---|---|---|
| `AppId` + parse laws + `unresolved` + `slugify` | port-verbatim | `seam` module; + hand-written `Deserialize` via `parse` |
| `CellKey` (+ `this_machine`, `current_locale`) | port-verbatim | `seam` module |
| `PackId`, `IdError`, `UNRESOLVED_PREFIX` | port-verbatim | `seam` module |
| the 4 law-table tests | port-verbatim | move with the types, stay green |
| `id.rs` file itself | retire | coach-pack imports from protocol; `pub use id::*` exports in coach-pack `lib.rs` retired (callers today: coach-pack internals + its examples only — verified) |

**`coach-pack/src/pack.rs`:**

| Element | Disposition | Destination |
|---|---|---|
| `Rung` enum + `label()` | retire | protocol `TransportRung` (same variants, same labels — TOML values `"r0"`/`"r1"` byte-identical, §6 test) |
| the l.16 "until S2 rehomes both" comment | reauthor | comment updated to state the protocol type is now the one home |
| everything else (`PromotedRung`, `effective_rung`, `Pack`, parser, hash, writer + all 8 tests) | keep | untouched |

**`coach-pack` `bundle.rs` / `registry.rs` / `resolve.rs`:** keep —
only their `use` lines change (ids from protocol). All 17 tests stay.

**`coach-daemon/src/main.rs`:**

| Element | Disposition | Destination |
|---|---|---|
| `execute_verb(&str)` string match | reauthor | match on `SeamVerb`; parse at the HTTP edge; unknown refused by name (behavior identical, identity typed) |
| `geom`\|`window_geom` alias arm | keep | `window_geom` = parse alias of `Geom`, never emitted |
| per-verb bespoke JSON responses | reauthor | `EffectContract` envelope per the field table above (ADR §8.2 sanctions the new shape; battery re-run proves it) |
| `/verbs` handler over `verb_catalog()` | reauthor | derives from `seam_verb_catalog()` |
| `/events` SSE ad-hoc `moved`/`resized` JSON | reauthor | typed protocol `SeamEvent` facts (same event names on the wire) |
| S1 span publishing (all handlers) | keep | rung type becomes the protocol's (labels unchanged in Parquet) |
| everything else (state, tap, drain, overlay spawn, presence, import/resolve/invoke/mount flow) | keep | untouched |

**`dpl-fabric` (S1 types):** `TransportRung` retired for the protocol's
(new named edge, §4.1); `SeamSpan`/`SeamEvent`/`SeamAgency` keep —
ledger-domain, not wire vocabulary. Parquet output byte-compatible
(labels identical; §6 test).

**Proof artifacts:** `docs/proof/coach-verb-battery/REPORT.md` (2026-08-01)
kept forever; the S2 re-run writes a NEW dated report beside it (Law 7).

### 4.6 Docs amended in the same PR

- `coach-operator-verb-catalog-adr.md` — amendment: home moved to
  `egui_pilot_ai_protocol::seam`; `geom` cataloged (shipped since
  CX-events-01, battery-proven); count honesty note ("the 101" is a name).
- `architecture.md` §3 — coach-daemon + coach-pack gain the
  `egui-pilot-ai-protocol` edge; coach-runtime loses `verbs.rs`; protocol
  crate gains the seam module note. xtask audit table amended in the same
  commit if any censused edge changes.
- `coach-seam-adr.md` — one-line amendment recording the §2.1 refinement
  (validated ids, not bare `string_id!`).

## 5. Wiring (dependency edges, all additive)

```
egui-pilot-ai-protocol  (no new deps; +seam module; v2.7)
        ▲               ▲                ▲
        │               │                │
  dpl-coach-pack   dpl-fabric      dpl-coach-daemon  (+dev-dep ureq
  (ids + rung      (rung — NEW     (translator; typed edge)
   imported)        named edge,          │
                    §4.0/§4.1)     dpl-coach-runtime
                                   (−verbs.rs; otherwise untouched)
```

The `dpl-fabric → egui-pilot-ai-protocol` edge is the one NEW named
exception — amended into architecture.md §named-exceptions in the same
commit, per the pack.rs l.16 in-code commitment ("S2 rehomes both").
No crate gains an egui dep; EGUI_FREE census unchanged.

## 6. Proof

**Unit (all in-crate):**
1. `SeamVerb` wire round-trip: every catalog descriptor's name parses to
   its variant and `wire_name` round-trips; `window_geom` aliases to
   `Geom`; garbage refuses by name.
2. `SeamExtensionId` requires the `x.` namespace; `x.unresolved:` ids
   still parse post-move (the S3a law table moves and stays green).
3. `AppId` wire honesty: `Deserialize` refuses an invalid id (serde cannot
   bypass `parse`).
4. `EffectContract` additive-law compat: a v2.7 contract with unknown
   extra keys deserializes under a reader that does not know them (§7.1
   compat-test-per-addition).
5. Rung label continuity: the ONE protocol `TransportRung`'s labels are
   exactly `r0`/`r1`/`r2`/`r3` — a fixture test in `dpl-ledger` proves a
   post-S2 `SeamSpan` row serializes the same rung strings the S1-era
   Parquet rows carry (old segments and new answer one query).
6. S3a pack files parse unchanged: the drill-written `pack.toml` fixture
   (Blender shape) round-trips through the post-move loader with `"r0"`/
   `"r1"` values intact.
7. **No-silent-drop census (disposition table §4.5 enforced in code):**
   `seam_verb_catalog()` ⊇ the 2026-08-01 catalog — all 20 names with
   identical status/target, asserted against a literal copy of the old
   rows kept in the test.

**Live (this seat, DISPLAY=:1, sequential):**
8. Rebuild daemon; run `verb_battery` example against gnome-calculator →
   **GREEN required** (25-step matrix, drag-refusal included).
9. `seam_reliability` example over the battery run's analytics root →
   per-verb reliability table printed from real Parquet — the first
   battery in this repo's history that yields stats (closing S1's origin
   story).

**Gate:** full `cargo green`; PR; merge on double-green only
(`gh pr checks --watch`); `docs/STATE.md` + journal; exe rebuilt on main;
ops docs ride this PR (house pattern) including the S3a journal +
legitimately drifted `Cargo.lock`.

## 7. Risks

- **Protocol consumers (7 crates).** Additive module + minor version bump
  only; no existing type touched. Mitigation: `cargo green` compiles the
  whole workspace; capability catalog tests exercise the new entries.
- **Serde on validated ids.** A derived transparent `Deserialize` would
  silently bypass `parse`. Mitigation: hand-written `Deserialize` +
  unit test 3.
- **Response-shape change vs battery.** Sanctioned by ADR §8.2; the
  battery is re-run and its report regenerated in the same PR — the old
  REPORT.md stays in git history (never delete content; the new report is
  a new dated file).
- **`SeamEvent` name collision.** Accepted with precedent (two `Fabric`s);
  qualified paths in the daemon.
- **`ureq` dev-dep.** New dependency, examples-only, no production edge —
  flagged here for the operator's red pen; alternative is a hand-rolled
  `TcpStream` HTTP/1.1 client (no dep, more code to trust).
- **Battery needs the real display.** Same seat + DISPLAY=:1 that proved
  S1 and the 2026-08-01 battery; runs are sequential per house law.
- **The `dpl-fabric → protocol` edge.** New named exception on a side
  rail every surface crate reaches — the protocol crate is egui-free and
  dependency-light, but the edge is permanent once taken. Flagged in
  §4.0/§4.1 with the fallback (private copy + equality test) if the
  operator refuses it; the in-code commitment (pack.rs l.16, merged
  #228) is why the edge is the recommendation.
- **Law 7 vs move law.** RESOLVED — operator ratified the reading
  2026-08-10 (§4.0): pre-release means moves are done properly to the
  respective boundary, no shims; Law 7 protects content.

## 8. Evidence

(To be filled at landing: test counts, battery report path + verdict,
reliability table sample, PR number, double-green run ids, exe mtime.)
