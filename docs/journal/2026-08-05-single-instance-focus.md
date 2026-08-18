# 2026-08-05 — PARKED: the app opens a second window instead of raising the first

**Status:** RESEARCH NOTE. Not this pass — needs its own plan + PR (operator,
2026-08-05: *"ill create a separte PR for that"*). Recorded so it is not lost.

## The defect, observed

Launching `devpulselabs.exe` while an instance is already running produces a
SECOND, half-broken window. The running instance keeps the estate lock and the
seam port; the new one degrades silently and opens anyway.

Observed verbatim in a real launch (2026-08-05, lane exe):

```
WARN  devpulselabs: estate open failed — durable seam state disabled
      e=estate open …\.userdata\estate: FjallError: Locked
ERROR dpl_seam: agent seam bind failed — seam disabled
      e=Only one usage of each socket address … (os error 10048)  port=7341
```

**The app already detects the collision twice and acts on neither.** The
operator's words: *"why are we not building it so its fucking intelligent and
has awareness so it brings the one existing to surface before offering a
separate one being opened?"*

## Ground truth (read in-session 2026-08-05)

- **No single-instance guard exists anywhere.** `grep -rin
  "single.instance|already.running|focus_existing|bring.to.front|
  SetForegroundWindow"` across `crates/` returns only unrelated hits (chat
  login flows, render jobs).
- `crates/seam/src/lib.rs:926` `spawn()` — doc-comment says *"A failed bind
  logs and ends the thread — the app never dies for a busy port."* The bind is
  at `:965`; failure logs at `:968` and `return`s. Deliberate resilience, but
  it means a busy port is treated as "no seam" rather than "already running".
- The seam binds `127.0.0.1` only (`:20`, security boundary) — so it is a
  usable same-machine IPC channel with no new dependency.
- Port comes from `crates/app/src/main.rs:375`, default `7341`, overridable
  via `--seam-port`; `--no-seam` disables it entirely.

## The shape a fix would take

The seam is already an HTTP server on loopback, so no new dependency is needed:

1. **Startup, before the window is created:** attempt the bind. Success →
   this is the only instance, carry on.
2. **Bind fails** → an instance is already running → `POST /focus` to
   `127.0.0.1:<port>` → exit(0) without ever creating a viewport.
3. **The running instance** handles `/focus` by publishing a fabric shell
   event; the app raises itself on the next frame via
   `ctx.send_viewport_cmd(ViewportCommand::Focus)` plus
   `ViewportCommand::Minimized(false)` (egui 0.35 API).

### Constraints that MUST be honoured — these are why it needs a real plan

- **The proof harness must keep working.** `--screenshot` / `--route` / drill
  runs would start exiting the moment the operator has the app open. The
  memory law *"proof-shot exe runs are SEQUENTIAL (parallel runs corrupted the
  index)"* argues the guard is CORRECT for data safety, but automation needs a
  sanctioned bypass — `--no-seam` already exists and is the obvious candidate.
- **`--seam-port` must scope the guard**, or two intentionally-separate
  instances (different content roots) would fight.
- **The estate lock is the second signal** and is arguably the more truthful
  one (it guards the data, not the API). Worth deciding which is authoritative
  rather than checking both incidentally.
- **A stale port must not brick launch**: if the bind fails but `/focus`
  gets no answer, the correct behaviour is to carry on, not to exit.

## Next step

Its own plan (`plan-code-change`), its own branch, its own PR. Do not fold
into the WiredFRONT editor work.
