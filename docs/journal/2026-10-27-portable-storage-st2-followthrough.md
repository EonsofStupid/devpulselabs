# ST2 follow-through — retained preferences and guarded estate writes

The operator said execute. The durable resume initially ended at ST1 (#297),
but refreshing upstream found ST1b (#299), ST2a (#300) and ST2b (#301) already
merged. Both ST2 PRs had completed Windows and Linux SUCCESS checks before
merge. This pass builds on `050bdee`, not a second ST2 implementation.

## Scope and findings

The isolated checkout remains
`D:/localdev/devpulselabs/.worktrees/portable-storage-st1`, now on
`feat/portable-storage-st2`. The operator's primary checkout, library and
release are untouched. The parent checkout remains `wip/c1-paused` and holds
only session bookkeeping; its untracked `.worktrees/` must never be staged.

Code grounding found two gaps in the delivered ST2 behavior:

- Chrome preset persistence advanced its saved snapshot even on failure,
  preventing a retry of the same in-memory choice after reconnect.
- Estate mutations and explicit flushes bypassed the root write guard.
  Reopening later did not protect writes through existing handles.

The canonical storage plan records this follow-through. `ChromeChoices`
now advances only after a successful save and skips repeated attempts while
the library is unplugged. The app opens its shared estate with a per-instance
guard. That guard covers open, put, remove, persist and reopen; an approved
new directory cannot authorize old database handles. A refused reopen keeps
the existing handles and cache. The estate dependency ceiling is unchanged.

## Review and verification

Session-os grounding, gap-close and execution used inherited-model agents;
the skill's named model roles are unavailable in this runtime. Review found
the sole production estate constructor wired, with other plain constructors
limited to tests. No further defects were found in the scoped diff.

Before the follow-through, the merged roots/estate focused suite passed
(35 roots tests and 5 estate tests). Four regression tests were added:
failed preference persistence followed by successful retry of the same choice;
denied estate creation; refused estate mutations/flush/reopen preserving saved
state; and rebased paths refusing old handles until explicit reopen.

Full `cargo green` passed: 1,212 tests, zero failures, six existing live-tool
drills ignored. Formatting, Clippy, docs/status, structure and contracts
passed, as did the release build and renderer smoke (16,616 shape pixels,
10,634 text pixels, all 60 export frames). Log:
`target/st2-followthrough-green.log` in the isolated checkout. Existing
native ST2 proof remains under `docs/proof/portable-storage-st2a` and
`docs/proof/portable-storage-st2b`. This follow-through changes persistence
logic and no pixels, routes, components or gallery registrations. It does not
claim a fresh USB or native reconnect drill.

## Limits and next

The guard covers estate API operations and explicit flushes. Fjall may still
finish background work accepted earlier. The existing ST2 rulings remain:
no connected heartbeat, 500 ms identity cache, and a changed drive letter
saves supported retained edits then offers Restart now. Existing documented
script/brainstorm retry and banner layout limits remain unchanged.

After verification: commit, push through the mandatory gate, PR, wait for
both CI lanes to complete successfully, merge, and rebuild the isolated
release. ST5 Locations remains the next product pass. The operator's own
library migration and physical USB check remain operator actions.
