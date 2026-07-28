# Notes — next phase: test & verification, then the determinism switch

Working notes, not a ratified plan. The next pass that writes code still
authors its own `docs/plans/<pass>.md` first (law 1); these notes are the
raw material and the checklist of proofs owed.

## A. Verification owed on Phase 1.5 (the real exe, not the unit tests)

Law 2: no claim without the exe or a test. The units are green; these claims
are still only *compiled*, not *witnessed*:

1. **The coach domain breathes end to end.** Build the devtools exe
   (`cargo build -p devpulselabs --features eguipilotai-devtools` — the
   plain build silently disarms the pilot; the CLI now names that failure),
   `pilot_start` with `--window 1280x832`, mount a touchpoint on a live
   workbench node, press it. Expect, simultaneously:
   - the shell inspector's SIGNALS card shows `coach mounted · …`,
     `coach fired · … · press`, `coach done · … · once` lines — screenshot;
   - `curl` the seam's `/signals` SSE and see the pinned coach envelope
     (`{"domain":"coach","type":"triggered",…}`) with `seq`/`ms`;
   - the pilot pipe still reports the same events (the tap stole nothing —
     the witness-not-consumer property, live this time).
2. **`Satisfied` on the record.** Run one `until:` postcondition step and
   confirm `step satisfied · <id>` reaches the feed — the A/B outcome
   stream's key row exists in the wild, not just in a unit.
3. **Ordering across domains.** Navigate (shell `route-changed`) between
   coach events; confirm the SSE stream interleaves them by one global
   `seq` — the flight-recorder claim the ledger will depend on.
4. **The waker under the lock.** `publish_coach` fires from the tap, inside
   the plugin's shared-state lock, mid-frame; `wake()` calls
   `request_repaint`. Reentrancy is believed safe — watch a live session for
   stalls or repaint storms anyway. Evidence beats belief.
5. **Signal counter.** The status grid's counter should tick on coach
   events like any other domain.

## B. Next build pass: the determinism-mode switch (dpl-intent's first consumer)

The shape, from plan §9.2/§9.3 — the pass plan will pin the details:

- **A mode value, held once**: `Behavioral | Strict`, owned by whichever
  seat owns the run (runner config for harness runs; the app's devtools
  block for live sessions). Behavioral is the default and needs no proof.
- **`/strict` at the operator's input-submit path**: recognized in the chat
  window's submit handler, from the operator's own text buffer, *before*
  the text becomes a message — mint `OperatorIntent::slash_command()` and
  strip the command. Nothing that reads model output, transcripts, or
  replayed traces can reach that code path; a test feeds a model message
  containing `/strict` and asserts nothing happens.
- **The UI second-acknowledgement**: a shell control whose handler refuses
  activation when the frame's input actor is `Agent` (the pilot already
  attributes injected input) — only `Operator`-attributed activation mints
  `OperatorIntent::ui_acknowledged()`.
- **Consumption writes an audit fact**: spending the proof yields its
  `IntentChannel` — record it (likely a small stamped fabric fact; decide in
  the pass plan whether that is a new variant or its own tiny domain).
- **Evidence forces strict** (§9.2): baseline approval / proof-shot paths
  demand strict mode regardless of the toggle — policy, not operator whim,
  and it does not consume an intent proof.
- **Tests owed**: provenance (model text is inert), single-use (already
  pinned by compile_fail; add the audit-record assertion), refusal of
  Agent-attributed UI activation.

## C. Carried backlog (unchanged priorities)

- **p6** — harness flag to select a Library section without soloing a
  template; still what blocks witnessing the templates wall and an Episodes
  lesson end to end.
- **p2b** — prove `mouse_passthrough` with a synthetic click and a witness
  window (overlay spike follow-up).
- **p4** — rail accordion degrades instead of clipping to nothing.
- **p5** — C2s foreign-target spike: AT-SPI on a Tier A app
  (`docs/plans/coach-c2s-foreign-target-spike.md`), findings feed the C2a ADR.
- **Phase 2** — `dpl-ledger` (Parquet from the start, §3.2 amended), whose
  coach rows now have a live domain to record.
