# Notes — next phase: test & verification, then the determinism switch

Working notes, not a ratified plan. The next pass that writes code still
authors its own `docs/plans/<pass>.md` first (law 1); these notes are the
raw material and the checklist of proofs owed.

## A. Verification owed on Phase 1.5 (the real exe, not the unit tests)

**STATUS: VERIFIED 2026-07-28, same session** — one devtools session
(`devpulse-cloud` / `library-wall` @1280×832, build `c333b69`), three
simultaneous witnesses, all agreeing. Proof shot:
`docs/proof/coach-domain-live-2026-07-28.png`. Findings per item below.

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

### Verification results (2026-07-28)

1. **VERIFIED.** All three surfaces saw the identical lifecycle at once:
   the SSE stream carried the pinned envelopes
   (`{"type":"triggered","coachId":"verify-press","trigger":"press",
   "actor":"agent","domain":"coach","seq":4,…}`), the run's `trace.jsonl`
   carried the same events on the pipe (the tap stole nothing — live, not
   just in the unit), and the SIGNALS card rendered all six lines
   (`coach mounted/fired/done · verify-press`, `coach mounted / step
   satisfied / coach done · verify-satisfied`). The typed rail split from
   C1.5f was also witnessed live: `workbench` reported `value=active`
   with `state.expanded=false`.
2. **VERIFIED.** `until: node-value(workbench=active)` was true at mount;
   the tool returned `activated.type=satisfied` on the mounting pass and
   `step satisfied · verify-satisfied` reached the feed — the A/B outcome
   row exists in the wild.
3. **VERIFIED with one nuance.** Coach events carry strictly increasing
   global `seq` (0–5 in-session). The shell's `route-changed` interleaved
   positionally but carries **no seq — the shell domain is unstamped by
   design** (doctrine law 4). Cross-domain seq ordering is therefore
   witnessable against stamped domains (capture/jobs/build/coach), not
   shell. Not a defect; worth remembering when the ledger reconstructs
   order: shell facts order by stream position only.
4. **No stall observed.** The whole session (start → snapshot → two coach
   mounts → agent flow → capture → stop) ran in ~4.5 s with the tap
   publishing from inside the plugin lock each event. No hang, no repaint
   storm in the trace. Not a load test — a soak under a busy lesson
   remains future evidence.
5. **VERIFIED.** Footer read `signals · 8` after the run; the card counted
   the coach lines alongside route changes.

## B. Determinism-mode switch (dpl-intent's first consumer)

**STATUS: LANDED 2026-07-28** — plan
`docs/plans/determinism-intent-gate.md`. Harness proof:
`docs/proof/determinism-evidence-2026-07-28.png` shows the DETERMINISM
card reading `strict · evidence` / "locked — this run is evidence" and
the SIGNALS feed line `determinism · strict · evidence`.

Shape pinned and implemented:

- **A mode value, held once**: `DeterminismGate` in `dpl-intent`
  (`Behavioral | Strict`). Behavioral default; `force_evidence` for
  proof shots.
- **`/strict` at the operator's input-submit path**: chat Ai composer
  Send click parses whole-buffer `/strict` · `/behavioral` before the
  draft becomes a message; clears draft; host is the ONE slash mint site.
  Synthetic submit → `IntentRefused · synthetic-input`.
- **The UI second-acknowledgement**: shell inspector DETERMINISM card
  (arm → confirm); refuses synthetic; evidence-locked when forced.
- **INTENT fabric domain** (doctrine law 11): `DeterminismChanged` /
  `IntentRefused` through seam + activity feed.
- **Evidence forces strict**: `--screenshot` boots call `force_evidence`.
- **NOT this pass**: clock/RNG/adapter pinning (replay); pipe surfacing.

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
