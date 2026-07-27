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
that node in the live app and blocks until it fires, reporting whether the
operator, the agent, or the app itself caused it.

`--on` chooses what fires it: `press` (default), `hover`, `appeared`,
`value-changed`, or `signal:<id>` for a fact the app publishes. A touchpoint
waiting on `appeared` or a signal mounts **armed**, so it can be set before its
target exists, and it needs a node id rather than a name locator.

Every fabric domain is coachable: `shell.*`, `deck.*`, `capture.*`, `jobs.*`,
`providers.*`, `build.*`, named exactly as the agent seam names them. Each fact
has a coarse id (`capture.saved`) and, where the key is enumerable, a specific
one (`shell.route-changed/workbench/library`, `providers.ready/grok`,
`capture.recording/on`). **Never guess an id** — `eguipilotai inspect` and MCP
`pilot_status` list every one this build publishes (`coach_signals`), and a
signal outside that list will never fire. The mapping is
`crates/app/src/coach_signals.rs`.

In scenarios use `kind = "coach"` steps with `type = "coach"` expectations
(`armed`, `mounted`, `triggered`, `unmounted`, `refused`). The program map is
`docs/plans/operator-coach-a2ui.md`; only `visual = "spotlight"` exists today
and anything else is refused by design, so check `doctor` capabilities before
promising more.
