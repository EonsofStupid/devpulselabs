# Pass — live `/strict` synthetic refusal witness

## 1. Intent

Close the honesty gap left by #116: unit + provenance law cover synthetic
refusal, but the optional SSE/feed witness path for agent-driven `/strict`
Send was never pinned as a testable mint-site policy. This pass extracts
that policy and proves it.

## 2. In / out / blast radius

**In:** `crates/app/src/chat.rs` (slash mint-site extract + unit tests);
docs (remaining-work, SESSION, resume, STATE tip, journal).

**Out:** live pilot GUI run (still operator/devtools optional); pinning;
shell DETERMINISM card changes; authoring filmstrip.

**Blast radius:** chat host slash path only — behavior unchanged, testable.

## 3. Design

Extract `resolve_operator_slash(gate, command, synthetic) -> SlashOutcome`
so the refuse/escalate/deescalate matrix is unit-tested without constructing
the full Chat `Shared`. `apply_operator_slash` publishes the outcome.

## 4. Verification

- Unit: synthetic `/strict` → `IntentRefused{action:strict, reason:synthetic-input}`;
  gate stays Behavioral. Real escalate changes mode. Evidence-locked
  behavioral refuses with `evidence-locked`.
- `cargo test -p devpulselabs` (or targeted chat module tests) green.
- Docs: p6 already closed; this item CLOSED; NEXT = p2b.

## 5. Done when

The synthetic refusal path is named, tested, and the ops ledger advances.
