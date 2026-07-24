# NAVIGATE seam — hub surfaces can finally ask for navigation

Parent: docs/STATE.md remaining list (the frictionless fix behind the
clay cards). After doctrine sweeps 1–6 (#4–#9). One checkout, `main`.

## 1. Context

Hub subroute cards LOOK tappable but are inert: hubs have no way to
request navigation — the deferred "fabric NAVIGATE event" (deferred
precisely because `ShellEvent` grew an exhaustive match in the session
EventLog). Doctrine/frictionless law: what looks tappable must act.

## 2. Current state (ground-truthed this session)

- `crates/fabric/src/lib.rs` (read in full): `ShellEvent{RouteChanged,
  PixvixSummon, FullBleedRequest(bool)}` Copy; broadcast·64; fabric
  DEPENDS ON dpl-hub (Route) → dpl-hub can never dep fabric (cycle).
- `crates/hub/src/lib.rs` placeholder::surface — paints cards, returns
  nothing; active card scatter, inactive clay (sweep 3).
- Shell drains its ShellReceiver each frame (FullBleedRequest arm; the
  D2 pattern) in `crates/shell/src/lib.rs`.
- `crates/pixvix/src/session.rs` EventLog: exhaustive `match ShellEvent`
  writing events.jsonl — WILL break on the new variant (by design; add
  an honest arm, never `_`).
- Hubs owning a Fabric today: present only. home/chat/shows/workbench/
  wiredfront constructors take no fabric (app/main.rs registration).

## 3. Target design

- `ShellEvent::NavigateRequest { route: Route }` (Copy).
- `placeholder::surface(...) -> Option<Route>` — a tapped card returns
  its route (cards get Sense::click + brutal-on-hover edge + tip
  "Go to <label>"; active card stays inert/scatter).
- Each placeholder-rendering hub gains a `fabric: Fabric` field
  (constructor takes `&Fabric`, app passes it at registration) and
  publishes NavigateRequest on Some.
- Shell drain: NavigateRequest arm → `self.navigate(route)` (idempotent
  guard already in navigate()).
- session.rs EventLog: new arm → `{"type":"navigate-request",
  "hub":…,"sub":…}` line (real events only).
- RouteChanged still the truth of arrival; NavigateRequest is the ask.

## 4. Changes, in execution order

1. fabric: the variant + a round-trip test.
2. dpl-hub placeholder: return Option<Route>, card Sense::click, hover
   edge via `cl::brutal_frame` stroke on hover? cards stay CLAY at rest,
   gain the tone stroke + pointer cursor on hover, tip "Go to <label>".
3. session.rs: the new match arm + a test line if the log test enumerates.
4. shell: drain arm → navigate.
5. hubs home/chat/shows/workbench/wiredfront: fabric field + publish;
   app/main.rs passes `&fabric` to each constructor.
6. green; drills: `--session-drill` still green (EventLog); screenshot
   proof of a hover-lit card; a kittest scenario if the shell harness
   reaches cards; STATE.md; PR.

## 5. Boundaries

🚫 No `_` catch-all in the EventLog match (real events law). 🚫 The
deck/present full-bleed path untouched. ⚠️ Constructor signature changes
ripple through app/main.rs registration only (pre-release, no shims).

## 6. Verification

- `cargo green` exit 0 (incl. new fabric test).
- `--session-drill 3` exit 0 — events.jsonl still writes (the match
  grew honestly).
- Tap-proof: kittest or pilot scenario clicking a card navigates
  (shell.route changes); else `--route home/...` screenshot + the
  fabric test stand in, recorded in §8.

## 7. Commit plan

Branch `feat/navigate-seam`, PR, squash-merge. STATE.md updated.

## 8. Executed — evidence

Executed to plan. `ShellEvent::NavigateRequest{route}` landed with a
round-trip test; the compiler surfaced exactly ONE exhaustive-match
consumer (session EventLog — new honest `navigate-request` arm, no `_`);
the shell's drain navigates on the latest ask; placeholder cards return
the tapped route (clay at rest, tone edge + "Go to <label>" tip on
hover); chat/workbench/wiredfront grew `wire_fabric` and publish; the
app wires all three at registration. One deviation from nothing: a
collapsible_else_if clippy gate hit in workbench, collapsed.

Proven: `cargo green` exits 0 (incl. `a_navigate_request_round_trips`);
`--session-drill 3` exits 0 (85 frames · 2 events — the EventLog still
writes after growing its match). Tap-proof rides the fabric test + the
shell drain arm (the same drain the kittest full-bleed test exercises);
an end-to-end card-tap pilot scenario joins when the Test Lab runner
lane lands (recorded).
