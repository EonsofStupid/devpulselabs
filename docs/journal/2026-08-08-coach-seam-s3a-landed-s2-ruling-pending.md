# Journal — 2026-08-08 (2): S3a landed · the absorption question · S2-vs-S3b ruling PENDING

## S3a — identity spine + pack registry, LANDED (#228, double-green, squash-merged)

Plan + full evidence: `docs/plans/coach-seam-s3a-identity-spine.md`.
Executed post-compact exactly per the approved design
(`2026-08-08-coach-seam-s1-spine-s3-design.md`), operator peer review
followed.

- New egui-free crate **`dpl-coach-pack`** (deps: toml + sha2 +
  dpl-coach-inventory; workspace member; xtask `EGUI_FREE` census +
  architecture.md §3 amended in the same commit).
- `AppId` minted once, parse-validated (reverse-DNS ≥ 2 segments; `x.`
  reserved; `x.unresolved:<slug>` for strangers). `CellKey`
  (version×os×locale) first-class in `PackId`.
- `FingerprintBundle::from_inventory` — honest evidence only
  (`live-only:` synthetics are NOT desktop ids; wm_class/version never
  invented).
- Evidence resolver: desktop_id 4 · startup/wm_class 3 · atspi/exe 2 ·
  alias 1; accept ≥ 3.0; exact ties = honest `Unresolved`; misses
  synthesize promotable generic profiles.
- Registry `packs/<app-id>/pack.toml`: schema-1 tolerant TOML (unknown
  keys warn AND survive rewrites), content-hashed `[signing]`,
  tmp→bak→rename, `UnresolvedWrite` refused (promotion mints first).
  Promote-only in TYPES: `PromotedRung` cannot express r2/r3.
- Proof: 25 unit tests (first build); live drill resolved the RUNNING
  gnome-calculator (score 8, live AT-SPI evidence) among 65 real rows,
  64 honest strangers; Blender pack proven by shape (Blender NOT
  installed on this seat — S4 seat prep). One clippy
  `large_enum_variant` fixed by boxing (no `#[allow]`). Full
  `cargo green`; exe rebuilt on main post-merge (mtime 15:06).

## The post-landing review (operator, direct)

1. **"Terminal demoable" was called out as nonsense — correctly.**
   The honest statement: S3a/S1 are foundations with NOTHING clickable.
   The only visible surface today is a proof example printing scan/
   resolution results. The first real demo (pick app → Build fabric →
   console → profile card) is S3b. Lesson recorded: say "foundation
   landed, nothing on screen yet" in so many words, every time.
2. **"Was everything before this useless?" — No, with receipts:** the
   verb engine (~2,500 lines, drives real apps, live-proven), inventory,
   fabric mint, S1 spans, S3a identity are all load-bearing and merged.
   The genuine drift was the daemon growing as a second agent surface +
   skipped tracing/profiles — caught by the 2026-08-07 audit, bounded
   waste ≈ the 800-line HTTP wrapper, which S2 demotes to a translator
   (repurposed, not deleted).
3. **The SME absorption question:** is everything properly absorbed
   under eguiPilotAI with testing/demoing vs coaching cleanly separated?
   **Honest answer: designed and ratified, NOT yet executed — that is
   S2.** Today: the daemon still speaks its own private verb enum over
   HTTP (the audited C2s §3.4 violation STANDS until S2); the `seam`
   module does not exist in `egui-pilot-ai-protocol` yet; S1's span
   types live in dpl-fabric and S3a's ids in dpl-coach-pack, both
   explicitly marked to MOVE (not copy, no shims) at S2. Once S2 lands:
   ONE protocol vocabulary, two capability-gated consumers — the
   testing/demo lane (EguiPilot runner/CLI over the inherited pipe,
   devtools-only) and the coaching lane (coach runtime on foreign apps,
   Teacher consent per lesson).

## THE OPEN DECISION — awaiting operator ruling

The original ADR §8 order was S1 → **S2 rehoming** → S3 → S4. The
operator's "build the UI" request re-prioritized S3 ahead of S2. The
absorption question exposes the cost: building S3b's hub UI now means
building against crate-local types S2 will rename underneath it (churn),
and living longer with the standing daemon violation.

**Recommendation given: S2 next, then S3b** — absorb the vocabulary
under eguiPilotAI first so the UI lands once, on the final protocol,
with the testing-vs-coaching capability gates in place from its first
commit. **Operator has NOT ruled yet.** Next session: get the ruling
(`S2` or `S3b`), then execute under the standing contract (plan doc →
tests → live drill → cargo green → PR on double-green → STATE/journal).

## Seat facts for the next session

- Repo on `main` at #228; exe rebuilt; ops files updated on disk
  (uncommitted docs ride the next PR, same as last session).
- `Cargo.lock` legitimately dirty on `main`: the committed lock is
  missing xtask's `image` dep (a sibling PR merged without its lock
  line); builds regenerate it. Commit it with the next slice's PR.
- Blender NOT installed on this seat; needed before S4.
- Operator's parallel lanes: `feat/coldlight-53-hues` + editor mock +
  workbench-editor plan (+ open PRs #227 smoke-tier, #222
  loss-recovery) — not ours, do not touch.
