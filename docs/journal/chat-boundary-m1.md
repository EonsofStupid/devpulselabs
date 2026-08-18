# BND1 · M1 journal — the command layer (2026-08-08)

**Objective (POAM):** typing `/` reveals the real command surface;
commands carry guided seeds and refuse bad input by naming what IS
valid; Enter never dies silently.

**Done this milestone:**
- **`dpl-chat-core` gains the command vocabulary** (346 lines, EGUI_FREE
  holds): `CommandSpec`/`CommandArg` census — SIX v1 commands (/route
  /model /routines /brainstorm /strict /behavioral, count-pinned) —
  ONE `recognize()` (whole-buffer provenance law: `/x` unknown stays
  text, patterns-in-strings never escalate, word aliases exact),
  `validate_command()` refusing by NAMING the census (the
  --brainstorm-drill message shape; argless commands refuse trailing
  text), `seed_arg()` riding `GuidedValue` with its dormant
  Route/Provider/Session sources — no second provenance vocabulary.
- **The window's two hand parsers are GONE** (deleted, not wrapped):
  `recognize_command` = the registry; `privileged_command` maps ONLY
  whole-buffer-exact /strict //behavioral onto the audited
  `OperatorSlash` wire (synthetic refusal untouched, the four mint
  pins pass unchanged); everything else crosses as
  `Command { draft, synthetic }` — the WHOLE census refuses
  model-minted input.
- **THE ENTER-HONESTY LAW**: `submit_key && !enabled` now paints the
  already-authored reason under the composer (`composer_status` +
  `chat.composer.status` pilot node), cleared by the next keystroke or
  a successful submit. Pinned by `a_refused_enter_paints_its_reason`
  (kittest: focus, bare Enter on an empty draft, assert the painted
  string) — the first test the silent path ever had.
- **Host dispatch** (`dispatch_command`): validates against the LIVE
  censuses the host owns (route table, roster, workflow registry);
  `/route <hub/sub>` publishes `NavigateRequest` — **NavigateTo's
  first real emitter ever** (the routes plumbing was dead weight
  behind a count badge); `/routines` lists the workflow census
  honestly; `/model` is a NAMED skip until the house session
  (BND1-M3). Refusals and outcomes all paint on the honesty line.
- **The route census went FULL** (main.rs): every hub/sub in the one
  `hub/sub` grammar (the seam's route_str law) instead of hub defaults
  — /route validates against exactly what the platform registers.

**Proof:** 16 core (4 new: census, provenance, refusals, seeds) + 45
window (Enter pin + rewritten provenance pins) + 37 app tests green;
fmt + clippy zero across the three touched crates.

**Deviations (plan §9.1):** `/strict please` now recognized-and-refused
("takes no arguments") instead of silently becoming a message — the
audited mapping stays exact. The window:4199 sensitivity leak parked
with a dated line.

**Remaining (POAM):** M2 typeahead strip + proof state · M3 Clyffy
house session + binding + /model · M4 close-out.
