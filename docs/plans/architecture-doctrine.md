# Architecture doctrine — the written layer law + the structure audit

**Status:** APPROVED IN INTENT (operator, 2026-07-28: "lets get this all
properly planned out and then implemented please"); executing
**Date:** 2026-07-28
**Origin:** operator ask — "ensure this code structure folder and file
structure is getting authored strict organized and methodically to align
to my patterns … that would translate to this platform industry best
practices."

## 1. Context

The workspace already practices a consistent architecture — pure document
crates with dep ceilings, subprocess-leaf engines (ADR-009), census
registries with teeth, one-concern crates, hubs that never touch each
other — but the structure itself is recorded nowhere and enforced only by
habit. Industry practice (the rust-analyzer `ARCHITECTURE.md` convention +
evolutionary-architecture fitness functions) says: write the layer law
down ONCE, then make the gate fail when a change breaks it. This pass does
exactly that and nothing more: a doctrine document authored from the real
dependency graph, and a cheap `structure` phase in `cargo green`.

## 2. Current state (grounded 2026-07-28, main @ 2e7bcea)

- `crates/xtask/src/main.rs`: `green()` (:110) runs fmt → `docs_status()`
  (:152) → `spawn_audit()` → clippy → tests → release build + mtime
  assert. xtask deps: dpl-tools-deps, serde, serde_json, toml (+ pelite on
  Windows) — `cargo metadata` JSON is parseable with what's already there.
- Dependency truth (full workspace sweep of `crates/**/Cargo.toml`):
  - Exact ceilings hold today: `dpl-comp` = {serde, serde_json};
    `dpl-workflows` = {serde, serde_json, dpl-comp, dpl-doc}; `dpl-proc` =
    {}; `dpl-tools-deps` = {serde, serde_json, sha2}; `dpl-landing` =
    {serde, serde_json}; `dpl-estate` = {fjall, moka, serde, serde_json}.
  - `dpl-doc` = {serde, serde_json, **dpl-coldlight**} — the document
    layer's ONE styling edge (Look/resolve speaks the token vocabulary).
    Recorded as law, not "fixed": the doc crate references tokens, never
    paints.
  - Hub→hub edges: NONE (hubs depend on surfaces/engines/dpl-hub only).
  - FFI: `thorvg-sys` appears ONLY in `dpl-lottie-render` (ADR-009 leaf).
  - egui-free crates today: proc, comp, workflows, estate, landing, index,
    tools-deps, engine-{capture,media,steps,ae,browser}, lottie, guide,
    chat-{core,store,accounts,browser-auth,interactions,profiles,runtime,
    social,testkit}, chat providers (codex/claude/gemini/grok), imagine,
    egui-pilot-ai-{protocol,transport,runner,cli}, xtask.
  - Known truths the doctrine records rather than hides: `dpl-video` and
    `dpl-vector` are egui-linked support crates (players/paint);
    `dpl-seam` depends on dpl-coldlight (envelope speaks tone ids);
    `dpl-fabric` depends on dpl-hub + dpl-chat-core (Route/provider
    types in facts).
  - `crates/desktop-spike/` holds a Cargo.toml but is NOT a workspace
    member — an orphan spike folder.
- Every member Cargo.toml carries `[lints] workspace = true` and effective
  `publish = false` (explicit or via `publish.workspace`).
- Harness flags: parsed in `crates/app/src/main.rs` as `"--…"` literals;
  `--content-root` alone is parsed in `crates/engine-capture/src/lib.rs`
  (:136-147). CLAUDE.md's harness list was ground-truthed in #105.
- `docs/design/` holds doctrine.md + design-system.md — the house's
  design-law shelf; no code-structure law exists anywhere.

## 3. Target design

**A. `docs/design/architecture.md`** — the layer law, authored from §2's
graph (never aspirational). Contents: the layer stack (authored tokens →
generated consts → document crates → engines (subprocess leaves) →
surfaces (egui via ColdLight) → hubs (one-line registry, never lateral) →
shell → app; xtask + EguiPilot as side rails); the dependency-direction
law (always downward; the named standing exceptions from §2 with WHY);
the naming law (`dpl-` prefix, kebab ids, folder groups `chat/*`,
`hubs/*`); the census law (any list of things = ONE registry fn + a
census test); the document law (schema_v + newer-refusal + crash-safe
tmp/bak writes); the NEW-CRATE CHECKLIST (one concern in the doc header,
`version/edition.workspace`, effective `publish = false`, `[lints]
workspace = true`, inline `#[cfg(test)]` tests, member entry in layer
order, ceiling declared in the structure audit). Plus a pointer map to
the sibling laws (ADR-009, doctrine.md, signal doctrine, two-libraries).

**B. xtask `structure_audit()`** — a fitness-function phase in `green()`
after `spawn_audit()` (cheap, fails fast, before clippy), also exposed as
`cargo xtask structure`. Checks, each with a named failure message:

1. **Ceilings (exact-set)**: for the §2 ceiling crates, direct deps must
   EQUAL the declared set (from `cargo metadata --no-deps` resolved
   against each manifest's `[dependencies]`). A new edge fails the gate
   until the table (and doctrine) are consciously amended.
2. **egui-free census**: the §2 list of egui-free crates must have no
   direct dep matching `^(egui|eframe)` — adding UI to a pure crate is a
   one-line conscious act in the census, like registering a slide.
3. **No lateral hubs**: no `crates/hubs/*` member may depend on another
   `dpl-hub-*` crate (dpl-hub itself allowed).
4. **FFI leaf law**: a direct dep ending `-sys` is allowed ONLY in
   `dpl-lottie-render` (allowlist of one).
5. **Manifest hygiene**: every member carries `[lints] workspace = true`
   and effective publish=false.
6. **Orphan crates**: a `crates/**/Cargo.toml` not in the members list
   fails, behind a named allowlist = {`desktop-spike`} (its deletion is
   the operator's call, not this pass's).
7. **Flag census**: the set of `"--…"` literals parsed in
   `crates/app/src/main.rs` + `--content-root` (engine-capture) must
   equal the flags named in CLAUDE.md's harness block — both directions
   (undocumented flag OR phantom doc entry fails).

## 4. Changes, in execution order

1. `docs/design/architecture.md` — create (content per §3A, every claim
   from §2).
2. `crates/xtask/src/main.rs` — add `structure_audit()` (+ helpers:
   metadata load via `cargo metadata --format-version 1 --no-deps`,
   manifest scan reusing the existing `toml` dep); call it in `green()`
   after `spawn_audit()`; add `"structure"` arm to the CLI dispatch.
3. `CLAUDE.md` — add `docs/design/architecture.md` to the read chain
   (one line under "Non-negotiable laws": structure law + the audit).
4. `docs/STATE.md` — tip.
5. This plan §8 — evidence.

## 5. Boundaries

- 🚫 Never: "fix" the recorded exceptions (dpl-doc→coldlight,
  seam→coldlight, fabric→hub, video/vector egui) in this pass — they are
  RECORDED, their removal is its own ratified pass each; delete
  `desktop-spike`; add any xtask dependency; touch crate code outside
  xtask; `#[allow]`.
- ⚠️ Ask first: tightening any ceiling beyond today's truth; making the
  audit fail on doc-comment/file-layout style (out of scope — this pass
  enforces GRAPH shape only).
- Out of scope: module-level conventions inside crates (a later doctrine
  section), CI changes, per-crate README files.

## 6. Verification

1. `cargo xtask structure` exit 0 on the unmodified tree, printing one
   summary line per check (counts, not noise).
2. Negative proof, each reverted after: (a) add `egui.workspace = true`
   to dpl-comp → audit names `dpl-comp` and the ceiling; (b) add
   `dpl-hub-home` to dpl-hub-shows deps → lateral-hub failure names both;
   (c) add a phantom flag to CLAUDE.md's list → flag-census failure names
   it. (Manifest edits only; no build needed — audit reads metadata.)
3. `cargo green` exit 0 end-to-end with the new phase in the order
   fmt → docs → spawn → structure → clippy → tests → build.
4. Docs render sanity: architecture.md claims spot-checked against
   `cargo tree` for two crates (comp, workflows).
5. Release exe mtime moves (gate's own assert).

## 7. Commit plan

Branch `feat/architecture-doctrine` → PR → squash-merge. Commit:
`docs+xtask: architecture doctrine + the structure audit (fitness gate)`.
Session-end trio after merge.

## 8. Evidence (2026-07-28)

- **First light caught real drift** — the audit's very first run failed on
  three genuine hygiene gaps: `dpl-chat-imagine` (no workspace lints, no
  publish pin — a #98 miss), `dpl-engine-capture` (no publish pin),
  `dpl-lottie-render` (per-crate lint override the check didn't yet
  recognize). Fixed the two drifts; recorded the FFI leaf's RATIFIED
  override (`unsafe_code = allow`, quarantined) in architecture.md §3 +
  a named audit allowlist. One matcher bug fixed: `egui-pilot-ai-*` are
  harness crates, not egui links — the UI match is now precise
  (`egui`/`eframe`/`egui-wgpu`/`egui_*`).
- **§6.1**: clean tree → `structure: 7 ceilings · 35 egui-free · 63
  members hygienic · 33 flags in census`, exit 0.
- **§6.2 negative proofs** (each reverted): (a) `egui` added to dpl-comp
  → BOTH the ceiling and the egui-free census name `dpl-comp`;
  (b) `dpl-hub-home` added to dpl-hub-shows deps → `hub depends on hub
  'dpl-hub-home' — hubs never touch laterally`; (c) `--phantom-flag`
  inside CLAUDE.md's harness block → `documented in CLAUDE.md but no app
  code parses it (phantom flag)`. Bonus proof: a dep line misplaced under
  `[lints]` breaks the manifest and the audit propagates cargo metadata's
  refusal — a broken manifest cannot pass.
- **§6.3**: `cargo green` with the phase order fmt → docs → spawn →
  structure → clippy → tests → build (result recorded at land).
- **§6.4**: architecture.md §2 table spot-checked against `cargo tree -p
  dpl-comp` / `-p dpl-workflows` (exact sets hold).
- Flag-census subtlety recorded: the parser reads ONLY the "- Harness
  flags" block (CLAUDE.md legitimately names `--no-verify`/`--features`
  elsewhere); `--content-root` is the one engine-parsed addition.
