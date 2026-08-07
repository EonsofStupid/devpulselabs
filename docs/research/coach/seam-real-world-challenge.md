# Seam real-world challenge — build once, offer to users (2026-08-07)

Operator ask: "the UI says building fabric and then we have it actually
build it out so we only have to go through the tedious process once and
then can offer them for users." This note is the adversarial pass on that
claim, ratified as the research foundation for the seam ADR (operator
`go`, 2026-08-07). Companion mock: `docs/design/mocks/coach-seam-mock.html`
(rev 2 — Foundry job console).

## The five gates (a pack that skips one will fail in the field)

### 1. Name-anchored fabric breaks on locale, theme, version

Accessible names are locale-dependent ("Basic" → "Básico"); themes and
scaling shift geometry; Electron releases churn the DOM-derived tree.
**Gate:** anchors are *fingerprint bundles* — role + tree-path +
relations + a11y-id + declared action set, name as one weighted signal.
On attach the pack **revalidates**: cheap census, nearest-fingerprint
re-resolve, honest degradation report ("built on 4.2-en, you run 4.5-de,
87% of anchors verified"). Tier B dodges this class entirely —
`bpy.ops` identifiers are locale-stable API names (another reason
Blender is the ratified first pack).

### 2. A tree walk is not a fabric — the tedious part is active exploration

Most capability hides behind menus/dialogs/modes that do not exist in
the tree until opened. Building a true fabric means driving the app,
which mutates state. **Gate:** sandbox law — scratch profile / empty
project, op whitelist, revert-or-restart between probes. The target
*will* crash mid-exploration: the Foundry checkpoints to disk and
resumes (relaunch target, continue), never restarts from zero. An
hours-long job that loses progress at hour three is a pipeline nobody
re-runs.

### 3. A pack is a claim until a battery proves it

A verb recorded during a walk is a hypothesis. **Gate:** automated
verification battery per pack — every anchor resolved, every promoted
verb executed against its effect oracle on a clean seat, reliability
scored per verb. Pack publishes only on battery green; battery re-runs
on app update. This is `true-verify` culture applied to packs.

### 4. Distribution is a security product

Tier A packs can be data-only (TOML + fingerprints). High-value packs
carry code — Blender seam addon, VS Code bridge extension, SolidWorks
COM add-in: signing, hashes pinned in the manifest, marshal-only law
enforced by review. ToS reality: some apps forbid automation; games
with anti-cheat will flag input injection — the games roadmap requires
a **coach-marks-only, zero-injection** mode.

### 5. The matrix is the cost — the pipeline is the asset

Pack validity is app × version × OS × locale × display server. "Once"
means once *per supported cell*. The only scaling answer is that the
Foundry itself is fully automated, checkpointed, journaled, and
resumable, so the next cell is a button, not a week. Ship few cells,
verify them hard, degrade honestly everywhere else (UFO² landed in the
same place).

## Foundry pipeline (what "Building fabric…" actually runs)

```
census → passive fabric (fingerprint anchors)
→ active exploration (sandboxed, checkpointed, crash-resume)
→ verb probing with oracles (reliability per verb)
→ pack induction (Agent Workflow Memory)
→ VERIFICATION BATTERY (ship gate)
→ fingerprint + sign → registry publish
```

Every stage journaled, span-traced, cancelable, with kill criteria and
honest ETAs. On the user's machine: attach → revalidate → degrade
honestly → lessons run on the verified subset.

## Trace sink ruling (operator, 2026-08-07)

**Arrow record batches → date/app-partitioned Parquet, DataFusion
queries — not DuckDB** (consistent with ADR-014's analytics ruling).
Rationale: spans are append-only (Parquet's case, DuckDB's mutability
buys nothing) · pure Rust, no FFI (house law) · already in-tree
(`dpl-ledger` writes Parquet via Arrow; arrow/parquet 59.1 in lock) ·
one query engine means seam spans join product signals — "which verbs
are least reliable on Blender 4.5" and "which lessons stall operators"
is one query. Evidence frames stay on disk referenced by span id.
DuckDB is reconsidered only if a concrete need emerges that Parquet
cannot serve; none identified.

## Feeds

- Seam ADR (next pass, after operator red-pens mock rev 2): vocabulary
  rehomed into `egui-pilot-ai-protocol` · AppProfile TOML schema with
  fingerprint anchors + revalidation policy · per-verb transport ladder ·
  Foundry pipeline contract · Arrow/DataFusion trace sink.
- First pack: **Blender** (ratified) — locale-stable, deterministic
  enumeration, cheapest build-once proof.
