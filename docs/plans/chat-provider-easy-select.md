# M1 — provider easy-select restoration (POA&M 2026-07-29)

## 1. Context

The provider catalog pass (#94) replaced the AI rail's always-visible
provider rows with a dropdown card (`provider_selector`) that opens a
modal overlay. Operator (2026-07-29): "you took away my ability to easily
select a provider … it triggers inside the chat the ability to log in to
subscription." Experience-first law: restore the pre-#94 interaction
(ground truth `fcd8196` + `docs/proof/chat-steam-ai-native.png`) while
keeping all post-#94 plumbing. The catalog overlay survives ONLY as the
Add local / Add custom / full-census escape hatch.

## 2. Current state (read this session, main `28346ab`)

`crates/chat/window/src/lib.rs`:
- `ai_sidebar` (1298): scroll area renders `provider_selector` (1323)
  then `session_groups` (1325).
- `provider_selector` (1511–1580): the dropdown card — pilot node
  `PROVIDERS_ID` ("chat.sidebar.providers"), click → `SetOverlay(Some(
  DomainOverlay::ProviderCatalog))`. ONLY call site is 1323.
- `provider_row` (1582–1665): the full row (accent mark, label, status
  dot + activation label, session count, click → `SelectProvider` +
  `SetOverlay(None)`). Pilot node `PROVIDERS_ID.child(descriptor.id)`,
  Role::Tab. ONLY call site: catalog overlay (2497, `expanded=true`).
- Catalog overlay (2424+): `PROVIDER_CATALOG_ID` Role::Dialog; renders
  `provider_row` per provider + supplements (clyffy/local/custom).
- Test `provider_selector_summarizes_all_active_sessions` (4410): only
  asserts `active_session_count` over provider views — no rendering.
- Scenario `tests/eguipilotai/chat-provider-catalog.toml`: rejects
  overlay interaction-1003, clicks `chat.sidebar.providers`, expects
  catalog + `chat.sidebar.providers/<id>` rows + supplements, walks the
  Local configuration form.
- Proof harness `crates/app/src/bin/chat_native_proof.rs`: states incl.
  `ai` (the old chat-steam-ai-native.png state).

## 3. Target design

The AI rail navigation column becomes (top→bottom): `AI PROVIDERS`
micro-heading; one `provider_row` per registered provider (always
visible, one-click select, status at a glance — the pre-#94 interaction
with the post-#94 row visuals); a `＋  More providers…` button —
pilot node `PROVIDERS_ID` (unchanged id, so existing scenarios keep
their entry point) → opens the catalog overlay; then SESSIONS groups as
today. Rail rows own `PROVIDERS_ID.child(<id>)`; catalog copies of the
same rows re-home to `PROVIDER_CATALOG_ID.child(<id>)` so the two
surfaces never mint duplicate pilot ids while the overlay is open.

## 4. Changes, in execution order

1. `crates/chat/window/src/lib.rs` — `provider_row` signature gains
   `in_catalog: bool`; metadata parent becomes
   `if in_catalog { PROVIDER_CATALOG_ID } else { PROVIDERS_ID }`.
   Call site 2497 passes `true`.
2. Same file — add `provider_roster` (new fn beside
   `provider_selector`): micro-heading, `provider_row(…, false, …)` loop
   over `providers`, then the escape-hatch button
   (`sidebar_destination_button(ui, "＋  More providers…", true)`) with
   `NodeMetadata::new(PROVIDERS_ID, Role::Button, "Open AI Providers")`,
   description "All built-in providers plus Local or Custom setup.",
   click → `SetOverlay(Some(DomainOverlay::ProviderCatalog))`.
3. Same file — `ai_sidebar` 1323: `provider_selector` → `provider_roster`.
4. Same file — DELETE `provider_selector` (1511–1580). No dead code.
5. Same file — rename test 4410 →
   `active_session_count_spans_all_providers` (same asserts; the name no
   longer references the deleted fn).
6. NEW `tests/eguipilotai/chat-provider-rows.toml` (fixture
   `chat-authored`, record): open/wait/focus/reject interaction-1003;
   expect visible `chat.sidebar.providers/{codex,claude,gemini,grok}`
   WITHOUT any click; click `…/grok` (proves one-click select); click
   `chat.sidebar.providers` (escape hatch); expect
   `chat.provider.catalog` visible; close viewport.
7. Proof shot `docs/proof/chat-provider-rows-native.png` via
   chat-native-proof `--state ai` (sequential, real exe).

## 5. Boundaries

- 🚫 `/content`; provider activation flows (`chat.rs` host side, Grok/
  Codex runtimes) — M2 verifies them untouched; assist/imagine surfaces.
- 🚫 `presence_rail` (folded sidebar) — stays catalog-opening this pass;
  noted for M3.
- ⚠️ `chat-provider-catalog.toml` expectations should hold unchanged
  (entry node id preserved; rail rows satisfy `chat.sidebar.providers/
  <id>` visibility) — if the runner disagrees, amend the scenario in the
  SAME commit and record it.
- Out of scope: provider tabs + per-tab sessions (M3), reference-image
  redlines (operator gate).

## 6. Verification

1. `cargo test -p dpl-chat-window` — all pass.
2. `cargo green` (server1 lane) — green ✔.
3. EguiPilot: `chat-provider-rows` scenario PASSES; `chat-provider-catalog`
   + `chat-grok-signed-out-readiness`-family scenarios PASS unchanged (M2
   evidence).
4. Proof shot shows the roster in the rail (eye-check against
   `chat-steam-ai-native.png`).
5. PR merges only on DOUBLE-GREEN.

## 7. Commit plan

One commit on `feat/m1-provider-easy-select`:
"feat(chat): restore the provider easy-select rail (POA&M M1)" — lib.rs,
new scenario, proof png, POA&M status flip, STATE.md entry, journal.
PR → double-green → merge. Exe rebuilt HERE after merge.

## 8. Evidence

- Steps 1–5 executed as written; zero mid-execution amendments.
- `cargo test -p dpl-chat-window`: 22 passed / 0 failed.
- `cargo clippy -p dpl-chat-window --all-targets`: clean.
- Scenarios (devtools exe, `eguipilotai.exe test devpulse-windows …`):
  `chat-provider-rows` PASS · `chat-provider-catalog` PASS (unchanged,
  §5's ⚠ never fired) · `chat-peer-window` PASS · `chat-assist-toggle`
  PASS.
- Proof: `docs/proof/chat-provider-rows-native.png` (real release exe,
  1300×800) — roster matches the `chat-steam-ai-native.png` interaction.
- Lesson recorded (STATE + journal): chat-native-proof captures the
  release exe by title — rebuild before shooting.
