# AH1 — the AI catalog as data, what is on this machine, and the trust prompt

**Pass 3 of the chat plan's §4.8.4 order** (R302, R303; rulings "Approve the
order", "CLIs in the depot"). Status: **draft, waiting for the operator's OK.**

## 1. Context

R303 asks for the chat to know every AI on the machine by its "tell", and for
a built-in trust prompt before any AI gets read or write access. Today:

- The chat knows **five** AIs. Each is a compiled `ProviderManifest` const in
  its provider crate (`dpl_ai_codex::MANIFEST`, `claude`, `cursor`, `gemini`,
  `grok`). `crates/app/src/main.rs:1309-1356` lists them by hand, twice
  (devtools and release builds).
- "More AIs" (C2b) can only show those five.
- **Signed in** is asked of each vendor's own status command
  (`crates/ai/host/src/presence.rs`). That is the right law and stays.
- **Installed** is "the status command could be spawned". Nothing looks for
  vendor install locations.
- **Running** is not detected at all. Desktop apps are not detected.
- **No trust prompt exists.** An AI that runs in a project folder gets that
  folder without being asked.

## 2. Goal and non-goals

**Goal.** One catalog, as data, that lists every frontier AI the chat can know
about. For each one the chat shows plainly whether it is **installed**,
**running** or **signed in** on this computer, and what it can do here. No AI
reads, writes or runs anything in a project folder until the operator grants
it once for that project.

**Not in AH1:**

- Chatting with the new AIs. Kimi, Gemini, Copilot and the rest only gain a
  conversation in A2b (the generic ACP adapter). In AH1 they show their
  state and the route to install or sign in.
- Installing anything. Install buttons open the vendor's page or the depot
  route that already exists; AH1 adds no installer.
- Attaching to a desktop app. Desktop apps are detected and offered **open or
  bring to front** only (R12). They expose no local chat API.
- Storage moves (ST4).

## 3. Ground truth (read in this session, 2026-11-16)

| What | Where | Fact |
|---|---|---|
| Provider identity | `crates/ai/core/src/provider.rs` | `ProviderId(&'static str)`; `ProviderDescriptor` has label, short label, glyph, accent, summary and `since`; `ProviderManifest` adds profile, target capabilities, delivery slice and evidence. |
| Presence spec | `crates/ai/core/src/presence.rs` | Program, status args, signed-in and signed-out needles, login verb. Plain data, spawns nothing. |
| Presence probe | `crates/ai/host/src/presence.rs` | One worker thread, a 5 s bounded probe each; depot client for Codex and Grok; `specs()` lists five providers. A missing program is `NotInstalled`. |
| Registration | `crates/app/src/main.rs:1309-1356` | Five registrations built by hand, in two copies. |
| The ACP registry | `cdn.agentclientprotocol.com/registry/v1/latest/registry.json`, format in the registry repo's `FORMAT.md` | `{version, agents[]}`. An agent has `id`, `name`, `version` (plain X.Y.Z), `description`, `repository`, `website`, `authors`, `license`, `license_url`, `icon` and `distribution`. `distribution` holds `binary` (per platform target: `archive`, `cmd`, `args`, `env`, optional `sha256`), `npx` or `uvx`. Targets: `windows-x86_64`, `windows-aarch64`, `linux-x86_64`, `linux-aarch64` (plus `-musl`), `darwin-x86_64`, `darwin-aarch64`. About 47 entries, updated hourly by a bot. The index itself is not signed. |
| Running AIs here | `Get-Process` on this laptop, 2026-11-16 | Claude Desktop is `…\WindowsApps\Claude_*\app\Claude.exe`. The Claude Code CLI is also `claude.exe`, under `.vscode\extensions\anthropic.claude-code-*`. ChatGPT is `…\WindowsApps\OpenAI.Codex_*\app\ChatGPT.exe`. Codex CLIs are under `AppData\Local\OpenAI\Codex\bin` and `.codex\packages`. Grok Bot is `C:\Program Files\Grok Bot\Grok Bot.exe`. **A process name alone is not a tell**: two different products are both `claude.exe`. |
| Project binding | `crates/chat/projects/src/{lib,registry}.rs` | `Project` (id, label, workspace, archive dir, state) and `Registry` (bindings, the active one). No trust record. |
| `sysinfo` | workspace dependency | Already in the workspace (§4.8.3). |

## 4. Design

### 4.1 One catalog, as data

- **`crates/ai/core/catalog/acp-registry.json`** is a vendored copy of the ACP
  registry, pinned: the file records the date fetched and the sha256 of the
  bytes. The app reads only this copy and never fetches at run time (§4.8.2
  rule 1).
- **`cargo xtask ai-catalog refresh`** downloads the live index, validates it
  against the format above, and rewrites the vendored file and its hash. The
  update arrives as a reviewable diff in a PR, like a dependency bump.
- **`crates/ai/core/catalog/devpulse.json`** holds only what DevPULSE adds,
  keyed by the same `id`:
  - the presence spec (today's consts move here as data);
  - desktop tells and known install locations per OS (4.2);
  - look: short label, glyph, accent;
  - the adapter that serves it, if one is qualified (`codex`, `grok`; the
    others say "planned").
  It also holds the AIs the ACP registry does not list: Claude Desktop,
  ChatGPT, Grok Bot, Meta AI and Hermes. (A "custom" AI the operator adds
  by hand needs a form and somewhere to save it, so it is left for its own
  pass and named under §8's gaps.)
- **`dpl_ai_core::catalog`** parses both files once (`include_str!`, zero-copy
  for ids so `ProviderId(&'static str)` keeps working), and merges them by
  `id`. An overlay entry for an id the registry does not have is allowed; a
  duplicate id, a malformed tell or an unknown adapter fails a unit test.
- **The five `MANIFEST` consts are deleted.** Their facts move into
  `devpulse.json`. The provider crates keep their adapters and are looked up
  by catalog id. `main.rs` builds the registrations from the catalog in one
  place, so the two hand-written lists become one loop (R301, no parallel
  lists).

### 4.2 Installed, running, signed in

All three are answered on the existing presence worker, never on the frame
loop, and each answer is published the moment its probe ends.

- **Installed:** any of
  1. the depot build (as today for Codex and Grok);
  2. the program on PATH;
  3. a known install location for this OS, from `devpulse.json`, checked by
     existence only (no file is opened). For example
     `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin\codex.exe`,
     `%ProgramFiles%\Grok Bot\Grok Bot.exe` and the MSIX package folders
     under `WindowsApps`.
- **Running:** one `sysinfo` process scan on the worker, at chat open and on
  Refresh. No polling loop. A tell matches the process name **and** a path
  pattern, because a name alone is ambiguous (`claude.exe` is two products).
  Only the name and path are read; no command line, environment or memory.
- **Signed in:** unchanged (vendor status command). A desktop-only AI has no
  status command, so its sign-in state is "unknown", shown as such and never
  guessed.
- What the scan saw rides on the registration (`OnMachine { installed,
  running }`) and shows on every More AIs row ("On this computer · running
  now"). Only a desktop app's state changes from it, to **Open** or **Bring
  to front**; an agent's state stays its own sign-in check's answer.
- More AIs groups its rows by what the operator can do: sign in, open,
  install, then not on this computer (catalog order within a group). With
  about fifty AIs, a flat list in registration order would bury the few that
  matter.

The UI shows these three facts in the More AIs list and on each AI's tab, with
one action each: **Sign in**, **Install** (opens the vendor's page),
**Open** / **Bring to front** for a desktop app, or **Not on this computer**.

Other operating systems: tells and paths for Linux and macOS are authored
from vendor documentation and marked "unverified" in `devpulse.json` until
they are seen on a real machine. They are not claimed in proof.

### 4.3 The trust prompt

- Before an AI starts a session whose working folder is a project's
  workspace, the host checks a trust record for that project and AI.
- With no record, the chat shows a ColdLight dialog (scrim, focus trap,
  tooltips): *"Let Claude work in 'Lesson 12'?"* with three separate
  switches, **Read files**, **Change files** and **Run commands**, all off by
  default, plus **Allow** and **Not now**.
- The answer is stored per project and AI in that project's state
  (`Project::save_state`), and can be changed later from the AI's tab.
- **Not now**, or no record, means the session is not started. The AI is
  denied, never started with a guess (§4.8.2 rule 7).
- What each switch maps to depends on the adapter. In AH1, Codex and Grok
  read it: Codex's sandbox and approval mode, and Grok's equivalent flags.
  An adapter that cannot honour a switch says so in the dialog, and that
  switch is greyed out.

### 4.4 Sub-passes (one milestone; two PRs, merged on the local gate, R304)

- **AH1a** — the catalog (4.1) and detection (4.2), feeding More AIs and the
  tabs.
- **AH1b** — the trust prompt (4.3), enforced in the host before every spawn.

## 5. Wiring

- `dpl-ai-core`: a new module `catalog`, with the two JSON files. No new
  crate. `serde_json` joins its dependencies (already in the workspace).
- `dpl-ai-host::presence`: installed paths and the running scan; `sysinfo`
  joins `dpl-ai-host`'s dependencies.
- Provider crates: lose `MANIFEST` and the presence consts; keep their
  adapters.
- `crates/app`: one registration loop from the catalog; the trust dialog's
  host call.
- `crates/chat/window`: More AIs reads the catalog's states; the trust dialog
  uses existing ColdLight patterns and the chat colour roles.
- `crates/chat/projects`: the trust record in project state.
- `xtask`: the `ai-catalog refresh` command.
- `architecture.md` and the xtask structure audit are amended in the same
  commit if any dependency ceiling changes.

### 5b. Reach outside the repo, for sign-off

- `cdn.agentclientprotocol.com`: fetched **only** by `cargo xtask ai-catalog
  refresh`, run by hand. The app never fetches it.
- Vendor install locations and running processes on this machine: **read
  only** (existence checks and a process list of names and paths).

## 6. Proof

- Unit tests:
  - the catalog parses, has no duplicate ids, and every overlay tell, path and
    adapter is valid;
  - tell matching tells Claude Desktop and the Claude Code CLI apart, using
    the real paths in §3;
  - a trust record round-trips, and no session starts without one (a test
    adapter, never a live AI).
- **Real run on this laptop, with a scratch library:** More AIs shows the
  catalog. Claude Desktop, ChatGPT and Grok Bot show "running". Codex and Grok
  show their real sign-in state. The trust dialog appears before the first
  Codex session in a scratch project, and "Not now" leaves no Codex process
  behind.
- Pilot scenarios: More AIs from the catalog; the trust dialog's switches,
  Allow and Not now, with focus return.
- Chat-window captures at 1100×700 and the 720×560 minimum, on a scratch
  library and a scratch home folder, so no real transcript can appear
  (consent given 2026-11-16: "Yes, scratch library").
- No chat turn is sent and no sign-in is started in a test.
- Full `cargo green`, then merge (R304), then read `main`'s hosted run. Then
  rebuild the release exe and check its stamp.

## 7. Risks

- **The registry changes hourly.** The vendored copy is deliberately stale:
  only a refresh PR changes it, and only when we choose.
- **A tell can go stale** when a vendor moves its install. The worst case is a
  "not running" that is wrong, never a false "signed in": sign-in stays with
  the vendor's command.
- **Removing the `MANIFEST` consts touches many tests**
  (`chat/window`, `chat/runtime`, `chat/testkit`). The testkit gets a
  catalog-built fixture in their place, with no alias or shim.
- **The trust dialog can block a flow that works today.** Codex and Grok
  sessions will ask once per project. That is the point of the ruling, and it
  is listed here so it is not read as a regression.

## 8. Evidence

**AH1a (2026-11-16).**

- The catalog lists 51 AIs: DevPULSE's 10 entries (5 of them claim
  `codex-acp`, `claude-acp`, `cursor`, `gemini` and `grok-build`) and the
  other 41 registry agents. `cargo xtask ai-catalog refresh` ran once against
  the live index and found the same bytes (sha256 `a22a5823…`).
- The five `MANIFEST`, `PRESENCE` and `TARGETS` consts are deleted; their facts,
  including which sign-in checks are unverified, are catalog data. `main.rs`'s
  two hand-written registration lists are one loop.
- Unit tests: the registry is the pinned copy; the merge rules and refusals;
  Claude Desktop and the Claude Code CLI told apart by the real paths; the scan
  rules on this laptop's processes; every catalog AI registers; More AIs
  groups, says Open, and never claims what the scan did not see; at 720×560 the
  LAST of 51 AIs' setup opens in view.
- Pilot (devtools exe, scratch library, run one at a time): the new
  `chat-more-ais-catalog`, plus `chat-general-tabs-composer`,
  `chat-provider-catalog`, `chat-providers-carousel`, `chat-provider-rows`,
  `chat-provider-tab`, `chat-peer-window`, `chat-grok-signed-out-readiness`,
  `chat-steam-social` and `chat-imagine-panel` pass.
  `chat-local-histories` fails at RECENT CHATS on `main` too (96e415d), so it is
  recorded as known broken, not caused here.
- The first pilot run caught a real defect: with fifty rows, a tapped AI's
  setup opened below the list, out of sight, and the page could not scroll.
  The list now scrolls, the setup opens under the tapped row, and the row
  scrolls to the top.
- Captures, live on this laptop: [1100×700](../proof/ah1a-ai-catalog/more-ais.png)
  and [720×560](../proof/ah1a-ai-catalog/more-ais-compact.png). Claude Desktop,
  ChatGPT and Grok Bot read "On this computer · running now" with Open app.

**Gaps.** A custom AI added by hand needs a form and storage; it is its own
pass. Linux and macOS tells and paths are authored, marked unverified, and not
claimed. A running agent CLI is shown on its row but does not change its state,
by design.

## Questions for the operator

Asked directly in the session.
