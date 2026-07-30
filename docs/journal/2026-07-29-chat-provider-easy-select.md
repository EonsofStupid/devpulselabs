# Journal — chat-provider-easy-select (2026-07-29, POA&M M1+M2)

## Done so far

- The #94 regression undone: `provider_roster` (always-visible one-click
  provider rows) replaced the dropdown `provider_selector` in the AI
  rail; catalog overlay demoted to the `＋ More providers…` escape hatch
  (pilot id `chat.sidebar.providers` PRESERVED on the hatch so existing
  scenarios keep their entry point). Catalog row copies re-homed to
  `chat.provider.catalog/<id>` — no duplicate pilot ids with the overlay
  open. `provider_row` gained `in_catalog: bool` (two call sites).
- Verified: 22/22 `dpl-chat-window` tests; scenarios
  `chat-provider-rows` (NEW — roster visible with zero clicks, row click
  selects, hatch opens catalog) + `chat-provider-catalog` +
  `chat-peer-window` + `chat-assist-toggle` all pass on the devtools
  exe; proof `docs/proof/chat-provider-rows-native.png` on the REAL
  release exe (real provider states: Codex Ready, Grok Ready).
- M2 evidence: the diff contains ZERO lines outside
  `crates/chat/window/src/lib.rs` UI fns — activation paths untouched.

## Exact next step

Land: commit on `feat/m1-provider-easy-select`, bundle → server1 gate →
push → PR → merge on DOUBLE-GREEN (the incident law) → rebuild exe HERE.
Then M3 (connected-provider tabs) per `docs/plans/poam-2026-07-29.md` —
plan first; the tab-strip design (Ready providers earn a tab beside
AI/Social, sessions beneath) needs its own 7-section plan.

## Landmines

- `chat-native-proof.exe` is a 1.5MB shim that captures the RELEASE
  `devpulselabs.exe` by window title — rebuild the release exe BEFORE
  shooting proofs or you screenshot the old build (bit me once today).
- Live-login Grok/Codex scenarios are operator-state-gated (need a real
  signed-out/ready client) — not runnable in this environment; the
  authored-fixture scenarios are the regression net.
- STATE.md tip collisions with parallel sessions remain ROUTINE —
  keep-both, newest first, Edit tool only, verify zero conflict markers
  BEFORE staging (the #131 near-miss law).
- The operator's reference image is still NOT in the repo; `fcd8196` +
  proof shots were the M1 ground truth. Re-check M3 against the image
  if the operator drops it.

## Verify commands

- `cargo test -p dpl-chat-window` → 22 passed.
- `target/release/eguipilotai.exe test devpulse-windows
  tests/eguipilotai/chat-provider-rows.toml` → `"error": null`, no
  `"passed": false`.
- Eye-check `docs/proof/chat-provider-rows-native.png` vs
  `docs/proof/chat-steam-ai-native.png` (the pre-#94 interaction).
