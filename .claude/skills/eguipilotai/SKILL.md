---
name: eguipilotai
description: Work on or drive this project's EguiPilotAI testing toolkit and DevPULSE dogfood coverage.
---

# EguiPilotAI in DevPULSE

Read `docs/plans/eguipilotai.md` before architectural changes and follow the
gated order in `docs/plans/eguipilotai-v1-delivery.md`; current gaps and source
census are indexed in `docs/research/eguipilotai/README.md`. Use named profiles
and fixtures from `.eguipilotai.toml`; keep the runtime behind
`eguipilotai-devtools`; preserve immutable renderer-keyed baselines and run
evidence. Use the global EguiPilotAI contract for normal tool operation.

## Coaching the operator

To show the operator *where* to act instead of describing it, mount a coach
touchpoint: `eguipilotai coach <profile> <node-id> --message "..."` spotlights
that node in the live app and blocks until it is pressed, reporting whether
the operator or the agent pressed it. In scenarios use `kind = "coach"` steps
with `type = "coach"` expectations. The program map is
`docs/plans/operator-coach-a2ui.md`; only `visual = "spotlight"` and
`on = "press"` exist today — anything else is refused by design, so check
`doctor` capabilities before promising more.
