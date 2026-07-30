# Journal — 2026-07-30: C2b `dpl-coach-overlay` (self-test pass 1)

## Verdict

**PASS** — first shippable `dpl-coach-overlay` binary proves the C2a ADR
gates on this box without wiring into production `app`.

## Done

1. **C2a ADR ratified** — `docs/plans/coach-overlay-adr.md` (operator
   authorized methodical build). `AGENTS.md` tipped: coach-overlay is the
   ratified **devtools-only** process exception.
2. **C2b plan** — `docs/plans/coach-c2b-overlay-daemon.md`.
3. **Crate** — workspace member `crates/coach-overlay` (`dpl-coach-overlay`):
   - `lib.rs` — compose / press-edge / refuse types (4 unit tests)
   - `bin/dpl-coach-overlay` — Linux → self-test; non-Linux stub exit 2
   - `linux_self_test.rs` — locate → stream identity → WINDOW paint →
     Sync-grab observe → XTEST Triggered
4. **Structure** — architecture §3 standing exception; `ORPHAN_ALLOWED`
   includes `coach-overlay`; `cargo xtask structure` green (66 members).

## Proof (this box, `DISPLAY=:1`)

| Gate | Result |
|---|---|
| Unit | `cargo test -p dpl-coach-overlay --lib` — 4/4 |
| Clippy | `cargo clippy -p dpl-coach-overlay --all-targets -- -D warnings` |
| Structure | `cargo xtask structure` — hygienic |
| Adversary | `--self-test --name bogon` → exit **2** refuse |
| Reality | `--self-test --name 7` → exit **0** Triggered |

Live log excerpt (calculator client `360×491@203,169`):

```
spotlight: screen_rect=64×44@219,462 · stream_hit="7" · proof_xtest=true
hotspot-watch: ButtonPress at 251,484 inside=true
hotspot-watch: TRIGGERED at 251,484
result: Triggered …
EXIT:0
```

Binary: `target/release/dpl-coach-overlay`.

## Boundaries held

- Never a dep of `crates/app`.
- Spike `foreign-target-probe` retained (copy, never delete).
- No stdio `CoachCommand` loop yet (plan Pass 2 / follow-up).
- No Windows/Wayland production claim; stub exits 2 off Linux.

## Next

1. Product spine: [`../plans/coach-operator-experience.md`](../plans/coach-operator-experience.md)
   — C2c mock → C2d inventory → C2e mint → C2f wire (Operator surface).
2. **Parked (Advanced):** Pass-2 stdio `CoachCommand` / `CoachEvent` loop
   under ADR launch gates — only when operator asks.
3. Strict pinning / DuckDB — after product path or operator redirect.
