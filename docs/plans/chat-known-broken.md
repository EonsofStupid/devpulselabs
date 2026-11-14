# Chat known-broken — provider readiness, the Grok sign-in, the compact tab bar

Status: implemented on branch `chat/known-broken` (base `af520bf`), not yet
landed. A fix pass inside the [chat plan](chat-native-program.md); it adds
no feature and no new crate. Line numbers below are at `af520bf`.

## 1. Context

`docs/status.md` listed four chat pilot scenarios failing on `main` and on C2a
at the same steps, and the compact chat window's tab bar overlapping at 720 px.
The guess recorded there ("saved sign-in state a scratch library lacks") was
not the cause: a scratch library holds no sign-in state at all, because sign-in
belongs to each provider's own client.

## 2. Goal and non-goals

Goal: each defect fixed at its cause, or shown to be this machine's state, with
the evidence.

Non-goals: no scenario or fixture is edited; no provider is signed in or sent a
turn; Codex still cannot send before a project is chosen (R292, PC1a); the
search field, window buttons and colours of the strip keep their current look.

## 3. Ground truth (read and probed in-session, 2026-11-13)

- `codex login status` from the depot's Codex 0.163.0 prints `Logged in using
  ChatGPT`.
- `grok models` from the depot's Grok 0.2.112 prints `You are not
  authenticated.` and exits 0. **Grok is signed out on this machine.**
- `grok` is not on PATH; `codex` is (the operator's own install).
- `%LOCALAPPDATA%\DevPULSE\chat` does not exist, so no project chat storage is
  set up and project boot ends in `Outcome::Empty`.
- Bundled-font coverage, read from each font's `cmap`: `◎` (U+25CE) is in
  egui's `emoji-icon-font.ttf`, which the Proportional family falls back to,
  so it is **not tofu**. `＋` (U+FF0B) is in none, which matches C2a's finding.

## 4. Root causes and fixes

### 4.1 Codex reads "Experimental", not "Ready" (`chat-codex-persisted-ready` step 5)

Four causes stacked up:

1. PC1a (#305) removed launch-directory discovery. Codex now boots with the
   inert `unconfigured_codex()` placeholder, `Blocked: Choose a project first`
   (`crates/app/src/main.rs:1289`, `crates/app/src/chat/project.rs:789`).
   Nothing replaced the machine-level sign-in check.
2. `with_local_history` (`crates/app/src/main.rs:1860-1880`) sees
   `~/.codex/sessions` and turns that placeholder into `Experimental: Local
   transcripts only`.
3. `apply_presence` (`crates/app/src/chat.rs:469-505`, the mapping at `:485`) skips any provider that
   is already activated, and `Experimental` counts as activated. Even when it
   applied, it mapped signed in to `Experimental`.
4. Presence answers were held back. `probe_all`
   (`crates/ai/host/src/presence.rs:152-170`) sent all five answers as one
   batch. `drain_presence` ran only from the root viewport's frame
   (`crates/app/src/chat.rs:1201`), and nothing woke the root when the batch
   landed. The answers were applied about 6 s after launch, about 40 ms after
   the step's 5 s poll had already failed.

Fixes:

- `crates/ai/host/src/presence.rs`: Codex and Grok presence asks the qualified
  depot client (`dpl_ai_codex::qualified_client`, `dpl_ai_grok::qualified_client`,
  new). Other providers, or a depot that does not resolve, keep the CLI on PATH.
  The Codex needles read `login status` the way the coordinator reads it: only
  a ChatGPT login is signed in, and API-key billing is signed out. Each answer
  is sent the moment its probe ends.
- `crates/app/src/chat.rs`: `apply_presence` speaks for Codex and Grok while no
  project has configured their coordinators (`project::configured`, new). A
  signed-in Codex or Grok is `Ready`, exactly as each coordinator maps it. The
  mapping is the pure `presence_activation`. `drain_presence` drains every
  waiting answer, and `service_project`, which both viewports run, now drains
  presence too and repaints while the probe runs. This follows the same rule
  as project progress: never depend on root paint.

### 4.2 Grok reads "Blocked", not "Ready" (`chat-grok-persisted-ready` step 5)

**Machine state.** The depot's `grok models` prints `You are not
authenticated.`, so Grok is signed out here. After the fixes, the catalog reads
Grok as `Signed out` with a sign-in button, which is the truth. This scenario
can only pass on a machine where Grok is signed in.

### 4.3 No Grok sign-in button (`chat-grok-signed-out-readiness` step 7)

The scenario applies here. Grok stayed on the `Blocked: Choose a project first`
placeholder, which has no recovery action and so no button. Presence could not
correct it for three reasons:

- it asked `grok` on PATH, which is absent, and got `NotInstalled`
  (`crates/ai/host/src/presence.rs:70`);
- the Grok needles did not know 0.2.112's signed-out words
  (`crates/ai/providers/grok/src/lib.rs:35-41`);
- the answer arrived too late (4.1, cause 4).

The fixes in 4.1, plus the `"you are not authenticated"` needle, make presence
report `SignedOut` with `Sign in with your browser`.

### 4.4 No Grok sign-in button in the authored fixture (`chat-steam-social` step 20)

The authored Grok (`crates/chat/testkit/src/lib.rs:243-256`, its activation at `:250`) is `SignedOut`
with a `SignIn` recovery and a saved read-only session. `ai_conversation`
(`crates/chat/window/src/lib.rs:3347-3351`) shows the saved transcript in place
of the connection card, so the only sign-in control was never drawn.

Fix: `provider_sign_in_strip` draws a strip above a saved transcript whenever
the provider is not activated and offers SignIn or Retry. The strip is a vibron
plate in the provider's tone, sized from Fluid, and holds the provider's reason
and the sign-in button. That button is `provider_sign_in_button`, which the
connection card now uses too. It keeps the same pilot id
(`chat.provider.login/<id>`) and gains the tooltip it lacked. The transcript and
the draft stay as they were.

### 4.5 The compact tab bar overlaps at 720 px

The cause is in `top_strip` (`crates/chat/window/src/lib.rs:1864-2034`):

- tab width was `(available − TITLE_WIDTH 166 − TITLE_BAR_RESERVED 250) / tabs`,
  clamped up to a 68 px floor;
- the floor made the row wider than the window, so the tabs ran under the
  right-to-left group;
- `mode_button` (`:5997-6067`) painted a 15 px label at a fixed 38 px inset
  with no clip, so "Content" ran into its neighbours;
- the right-hand group ("available" and the dots) drew over "Social".

Fix: the window buttons are laid out first and claim their measured width. The
title, tabs, search and status share only the room left of them, under a clip.
As the room shrinks, the strip sheds in a fixed order:

1. the search field;
2. the word "available" (the dots stay, with a tooltip);
3. the title's words (the brand mark stays and still drags the window).

Only then do the tabs narrow. `ModeTab` paints inside its own clip and ends its
label in "…". Every size comes from `cl::Fluid` of the window: `ctl_btn`, `sp_1`
to `sp_3`, `fs_body`, `fs_body_lg`, `r_md`, `ctl_icon` and `rail_expanded`.
The four constants are deleted.

## 5. Wiring

`dpl-ai-host` already depends on `dpl-ai-codex` and `dpl-ai-grok`, so no new
edge is added and the structure audit is unchanged. The new public items are
`dpl_ai_codex::qualified_client` and `dpl_ai_grok::qualified_client`, both
defined at the crate root, with no `pub use` added.

## 6. Proof

- Unit tests:
  - Presence: Codex and Grok needles, answers arriving one at a time, and only
    depot providers asking a qualified client.
  - Host: `presence_activation`.
  - Window: a signed-out saved chat keeps its transcript and its sign-in sends
    `ActivateProvider`; at 720 × 560 with seven tabs, no tab overlaps or reaches
    under the controls. This test fails on `af520bf`: Social ran under Show Main.
- The nine chat pilot scenarios on the devtools exe, with the depot junction and
  the profile's scratch `CONTENT_ROOT`. Live runs never click sign-in and never
  send.
- `cargo fmt`, both clippy runs, and `cargo green`.

## 7. Risks

- Before a project is chosen, a signed-in Codex now reads `Ready` while Send
  still waits for a project. The composer says "Choose or create a provider
  session first."
- Pressing a coordinated provider's sign-in before a project exists still goes
  to the placeholder coordinator. It then reads `Blocked: Choose a project
  first`. Sign-in before project setup needs its own pass.
- With no depot, presence falls back to the CLI on PATH, so `Ready` can name a
  client the runtime will not launch. Once a project is chosen, the coordinator
  corrects it.

## 8. Evidence

See `docs/status.md` (known broken) and the PR for the scenario table and the
gate line. Doctrine deviations that stay as they were: the strip's and the
tabs' fills still use the older `Color32::from_rgb` values, and the
brand mark keeps its 18-point proportions as ratios of its Fluid size. No
screenshots were taken: the task forbade capturing, and the 720 px geometry
is proved by the harness test instead.

Local run on 2026-11-13: `cargo green` passed (1,417 tests, 6 ignored) and
both clippy runs were clean. In the pilot, seven of the nine chat scenarios
pass. `chat-grok-persisted-ready` fails at its Grok Ready step and
`chat-codex-persisted-ready` at its step 6 (Grok Ready); both fail because Grok
is signed out on this machine. A scratch copy of the Codex scenario without
the Grok step stops at step 10 (Send enabled), because no project chat is set
up. Focus and catalog timing failed intermittently: 1 of 7
`chat-steam-social` runs at step 19, and 1 of 4 runs each for
`chat-grok-signed-out-readiness` and `chat-providers-carousel` at step 2
(viewport focus).
