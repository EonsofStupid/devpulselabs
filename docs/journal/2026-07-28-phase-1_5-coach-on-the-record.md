# Journal — 2026-07-28: the coach speaks on the record, and intent becomes a type

First entry of the journal. STATE.md stays the terse ledger; this folder
holds the narrative — what a session actually felt like, what surprised us,
and why the decisions went the way they did. Entries are append-only history:
never edited after the fact, corrected only by a later entry.

## What this session set out to do

Yesterday closed with a long design conversation — determinism, escalation,
sensitive data — and an operator "go". The commitment: ratify those rulings
into the signal-ledger plan as law, then land the first two pieces of code
under them — the coach fabric domain and the operator intent gate.

## What landed (PR #109, squash `5f0f42c`)

**Plan §9 — the rulings, written down.** Five of them: the closed loop is the
product shape (analytics recommends → the recommendation can trigger the
guided workflow itself, with DuckDB as a lens for generating tools and visual
widgets over the ledger, never a store of record); determinism is a named,
dynamic mode (behavioral by default for measurement, strict escalated for
evidence); escalation is authorized by the provenance of the keystroke, never
by a pattern in a string; redaction outranks reproducibility; classification
is three separated layers (CUI/Presidio base vocabulary storing the class
never the value · per-app tuning packs authored offline · HMAC
pseudonymization for identity without recovery). Games stay parked —
training-only was the ruling, anti-cheat hazard noted.

While ratifying, a drift surfaced in the plan itself: the status header
already carried the "Parquet from the start" amendment but §3.2's body still
argued JSONL-first. Both now say the same thing, and §3.1's layout example
reads `.parquet`.

**The COACH fabric domain (doctrine law 10).** Touchpoint lifecycle — armed,
mounted, triggered, satisfied, unmounted — as stamped signals like every
other fact domain. The load-bearing mechanism: the devtools runtime *drains*
`take_coach_events` for the pipe, so a naive second consumer would have
stolen events from it. The answer is a **witness tap** on the plugin's
single event-record path (`CoachStore::record`) — invoked before the event
lands in the drain queue, independent of who drains. Pinned by
`the_event_tap_witnesses_without_consuming`: the tap sees exactly what the
drain returns, and steals nothing. `Refused` never reaches the tap because a
refusal is a command result on the pipe, not a lifecycle fact.

Consumers landed with the domain, per doctrine: the seam's `/signals`
envelope gained `coach_json` (wire shape pinned by test) and the shell
activity feed now prints lines like `step satisfied · lesson-1/step-2`. The
shell subscribes its coach ear at construction — before the pilot runtime
attaches — so no publish can precede the subscription, and the `with_ears`
signature never changed.

**The anti-loop law.** `coach_signals` forwards fabric domains *into* the
coach as waitable trigger signals. If it subscribed to the new coach domain,
a touchpoint could wait on a coach event and self-trigger. The exclusion is
now written at the struct that would otherwise get "completed" by a helpful
future editor, in doctrine law 10, and in plan Phase 1.5.

**`dpl-intent`.** `OperatorIntent`: single-use proof, no `Clone`, no
`Default`, no serde — it cannot arrive over a wire, be minted from config, or
be replayed from a ledger, and consuming it spends it (`compile_fail`
doctests pin both). The two constructors, `ui_acknowledged` and
`slash_command`, make `rg 'OperatorIntent::'` the complete audit surface.
The crate is dependency-free, so `serde::Serialize` cannot even be *named*
inside it — and a test reads its own manifest to hold that sentence next to
the code. First consumer: the determinism-mode switch, next pass.

A small honesty correction mid-build: the first draft "asserted" non-serde
with a trait-bound trick that was in fact vacuous — it would have kept
compiling if serde were derived. It was replaced with the structural
guarantee above. A test that cannot fail is worse than no test.

## What surprised us

**Main moved mid-pass, and a conflicting PR gets no CI at all.** #107
(architecture doctrine + structure audit) merged while this branch was being
built. The PR sat `CONFLICTING`, which on this repo means *zero* Actions runs
— the exact failure mode STATE.md recorded from #103, now experienced
firsthand. Merged main in (STATE.md keep-both), and the new gate immediately
mattered: the structure audit carries exact censuses, so `dpl-intent` was
registered as the 8th ceiling (with the empty set — the audit now fails the
moment the intent proof learns a dependency) and the 36th egui-free crate,
with `docs/design/architecture.md` amended in the same commit per the change
protocol. The census law worked exactly as designed on its first new crate.

**The sandbox `CARGO_TARGET_DIR` strikes again, mildly.** `cargo green`
printed `exe: target/release/devpulselabs` while actually writing the binary
into the sandbox cache path. The repo-local exe was rebuilt explicitly with
the variable unset — the mtime law is about THE checkout's exe, not any exe.

**A second lane exists now.** `green-linux` ran on the mini-PC runner
(`dpl-linux-clyffy-lab`) and passed in 2m49s against Windows' 31m50s. The
cross-platform gate CLAUDE.md called for is real.

## Evidence

- `cargo green` ×3 (twice locally, once in the pre-push hook), plus the
  structure audit standalone: `8 ceilings · 36 egui-free · 64 members
  hygienic · 33 flags in census`.
- CI: green-linux pass 2m49s, green-windows pass 31m50s (run 30373048965).
- New unit tests: fabric coach round-trip/ordering + domain isolation; tap
  witness-not-consumer; bridge mapping incl. `Refused → None`; seam coach
  envelope pin; intent consumption + dependency-free manifest.
- `target/release/devpulselabs` rebuilt on merged main, mtime 11:55.

## Next

`docs/journal/2026-07-28-next-phase-notes.md` — verification of Phase 1.5 on
the real exe, then the determinism-mode switch as `dpl-intent`'s first
consumer.
