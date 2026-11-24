<!-- Preserved draft evidence; see main-reconciliation.md for current disposition. -->
> Historical draft at 2a7d0a5, based on obsolete main 1807459. The captures
> and earlier green runs below do not qualify the reconciled build. The
> build/library-folder project substitution was rejected; current main's
> project/trust code is retained. Current delivery: docs/plans/main-reconciliation.md.

# Testable Windows chat — R308

Authorized by the operator's 2026-11-20 request. This is the active chat pass,
extending the native chat program in the existing owning crates.

## Outcome

Open the native Chat peer, choose New chat, see this computer's provider
availability, choose an existing role and prepare a real conversation. Ship
a freshly built dev executable with a repeatable launch and proof path.

## Ground truth

At `1807459`, New chat is disabled for unavailable providers. Roles are
read-only reference cards. Presence checks run once, use PATH and can leave
stale status. Codex and Grok have transports; Claude has a descriptor and
history decoder only. Existing executables in this checkout predate main.
Finding an installed app must not claim that its conversation adapter works.
Meta Muse's exact product identity is awaiting clarification.

## Scope and boundaries

Rust-only native UI, existing provider registry and role documents. Preserve
provider launch qualification, project storage, owned-process containment and
credentials remaining with their providers. Never label a fixture as live.
Do not introduce generic process execution or silently change permissions.
This pass does not claim multi-agent rooms or desktop-app attachment.

## Steps

**Operator correction, R308 follow-up:** chat is aware of the platform's
project; it must not require a separate chat project. The former draft's
project picker and setup prerequisite were wrong. The following correction
precedes the remaining verification steps:

0. The app supplies its workspace and chat storage context. For a development
   build the workspace is the existing `dpl_roots::project_of_build` result;
   for a packaged app it is the already selected platform library. Chat state
   belongs under the platform's existing userdata/chat/conversations. The host
   opens this archive automatically and initializes providers against the
   supplied workspace. No CWD guessing, extra project registry, or chat-owned
   setup flow. Keep old archives intact and preserve the legacy storage APIs.
   Normal chat shows context as information, not a project-selection form.
   Reuse the app's current route awareness. Test automatic boot, restart/draft
   restoration, distinct supplied workspaces, and the real New chat flow.

1. Add a responsive New chat popup in `chat/window`, accessible even without
   a ready provider. Show provider readiness, inherited platform context and role choices;
   expose setup actions and an editable starter message. Selected roles become
   visible instructions in the new conversation's draft, never an automatic send.
   Preserve an existing draft and refuse overlapping session creation.
2. Refresh bounded provider observations off the paint thread. Project machine
   observations separately from transport readiness; never downgrade an owned
   active runtime. Add a manual refresh. Resolve named installed native Windows
   candidates before PATH wrappers; never read credentials or execute a guessed
   Meta binary. Existing approved provider discovery is the external reach.
3. Add devtools proof states/scenarios for the popup and compact layout; use the
   existing EguiPilot instrumentation. Supply a repeatable dev launch command.
4. Run focused logic/UI tests, formatting/clippy, build the real dev executable,
   inspect native screenshots and exercise the available live provider path.
   Record exact results and remaining provider failures in proof/status/journal.

## Acceptance

Live-proof amendment: opening New chat on a connected provider exposed a
duplicate permission-control ID shared with the conversation header. Scope
both egui and EguiPilot IDs by surface, and give the authored native fixture
the same permission mode as the real session. The native scenario must assert
exactly one header control and exactly one popup control before accepting it.
The live role draft also exposed an unbounded multiline composer that pushed
Send below the window. Bound its editor with a scrolling viewport, keep the
new-chat footer outside its scrolling choices, and scope provider-failure
messages to the selected or pending provider. Verify a long role draft can
actually be sent at the default window size.

Verification amendment: the full app suite exposed the existing chrome-save
test racing the process-wide library handle initialized by other UI tests.
Isolate that disk retry test in its own test process before rerunning the gate;
no production storage guard is relaxed.

- New chat is reachable when no provider is ready and explains what to do.
- Built-in roles load from the single role library and affect the prepared text.
- No draft is lost or sent without the operator pressing Send.
- Periodic checks do not overlap, block paint or replace active runtime truth.
- A fresh Windows exe and inspected 1100×700 / 1440×900 screenshots exist.
- Live provider evidence is separately recorded; synthetic UI tests do not count
  as a provider turn. Unimplemented adapters remain explicit gaps.
