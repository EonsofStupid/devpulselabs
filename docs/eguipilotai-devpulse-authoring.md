# EguiPilotAI authoring for DevPULSE

This is the boring path. A feature should look like every feature before it.
If the path cannot express a need, extend EguiPilotAI once through the vertical
extension contract in `docs/plans/eguipilotai.md`; do not create a local test
harness.

Implementation note: the full helper/accessibility/fixture/runner contracts
below are the target v1 path. Current support is tracked honestly in
`docs/eguipilotai-playwright-review.md`; the detailed contracts are indexed in
`docs/research/eguipilotai/README.md`.

## Where each concern lives

| Concern | One home |
|---|---|
| Static/dynamic control identity | The crate that owns and renders the control |
| Semantic role, name, value and state | Beside the real widget response |
| Async settlement | `BusyToken` held by the actual work lifetime |
| Motion determinism | Registration beside the authored animation |
| Launch/render matrix | `.eguipilotai.toml` named profiles/projects |
| Seeded state/external seams | Named fixtures and typed app adapters |
| Durable user behavior | `tests/eguipilotai/<domain>/<behavior>.toml` |
| Visual approval | `tests/ui/baselines/<renderer-key>/` through additive approval |
| Toolkit behavior | The owning EguiPilotAI crate plus conformance tests |

## 1. Declare identity once

Static controls use a constant in their owning crate:

```rust
pub const SAVE: PilotId = PilotId::new("devpulse.editor.save");
```

Repeated controls use one parent constant and a domain-stable key:

```rust
pub const GALLERY_CARD: PilotId = PilotId::new("devpulse.gallery.card");
let card = GALLERY_CARD.child(&PilotKey::new(artifact.id.as_str()));
```

Never derive identity from row index, visible text, geometry, animation frame,
or a random/runtime-only value. Text remains useful for exploration, but a
durable scenario selects the declared ID.

## 2. Instrument the real response

Use `UiPilotExt` for standard widgets. The custom-painted path is
`pilot_track(&response, metadata)`, attached to the real response the owner
already creates: it feeds egui's `WidgetInfo` and the pilot semantic tree but
records no egui/AccessKit ID correlation, so treat it as partial accessibility
coverage. A one-call helper that would also create the response with stable
pilot/egui/AccessKit identity was only ever described in prose and has never
been built.
Register the smallest truthful semantics: role, accessible name, value/state,
supported actions, parent region and actual response rectangle. Do not build a
second invisible control tree.

The shell rail is the current reference implementation in
`crates/shell/src/lib.rs`: its ID key comes from `HubId`, and the metadata is
attached to the same response that receives the user's pointer. It is not yet
the AccessKit-complete reference; that requires the projection/parity slice.

## 3. Make settlement truthful

- Start a `BusyToken` when real async work starts; drop it exactly when the UI
  can be acted on.
- Register finite or ambient motion where it is authored.
- Read time from `PilotClock` for visual state.
- Supply a canonical ambient phase or allow suppressed capture to fail with
  `UncontrolledMotion`.
- Do not add sleeps to make a scenario pass. Wait on a semantic expectation.

## 4. Choose one deterministic fixture

A fixture names an application state, not a bag of test commands. Examples:

- `library-seeded`
- `empty-inbox`
- `episode-basic`
- `pixvix-two-window`

The trusted `.eguipilotai.toml` maps the name to launch arguments and safe
environment selected by the runner. Future typed setup/teardown adapters own
temporary data roots and external-service fakes. MCP only names the fixture.

## 5. Write one Arrange → Act → Assert → Capture flow

Every durable scenario follows the same anatomy:

1. **Arrange** through its named profile and fixture; first assert a stable
   route/surface anchor.
2. **Act** through a declared ID using real pointer or keyboard input.
3. **Assert** semantic state or accessibility-visible output; expectations
   retry until their deadline.
4. **Capture** only when pixels are part of the contract.

```toml
id = "editor-save-title"
fixture = "episode-basic"
record = true

[[steps]]
kind = "expect"
[steps.expectation]
type = "visible"
[steps.expectation.target]
id = "devpulse.editor.title"

[[steps]]
kind = "action"
[steps.action]
type = "fill"
text = "A clearer title"
[steps.action.target]
id = "devpulse.editor.title"

[[steps]]
kind = "action"
[steps.action]
type = "click"
[steps.action.target]
id = "devpulse.editor.save"

[[steps]]
kind = "expect"
[steps.expectation]
type = "text"
value = "Saved"
[steps.expectation.target]
role = "status"
name = "Document status"
```

Use explicit `wait` only when elapsed time is the behavior under test. A hard
wait is never a substitute for settlement or an expectation.

## 6. Keep native boundaries honest

Playwright features that exist because a browser has DOM, cookies, network
routes or JavaScript evaluation do not belong in the generic protocol.

- Disk assertions use a named jailed document probe.
- External service behavior uses a typed fake/record/replay fixture adapter.
- OS-global hooks use engine tests and explicit drills; the pilot does not
  pretend it generated an OS event.
- Modal windows and PixVix use native viewport semantics.
- File inputs/outputs stay inside the fixture's declared sandbox.

Every white-box operation is labeled as an adapter operation in traces. It is
never reported as a click, keystroke or drag.

## 7. Definition of done for a DevPULSE feature

- Owner-declared static IDs and stable dynamic keys exist.
- Roles, accessible names, state and supported actions are truthful.
- Keyboard operation is covered when a keyboard path exists.
- Busy work and motion register at authorship.
- One deterministic fixture exists or the feature proves it needs none.
- One real-process durable flow covers the golden path.
- Important failure behavior has a focused scenario or unit test.
- Visual claims have renderer-keyed evidence; semantic-only behavior does not
  create a gratuitous baseline.
- The agent snapshot remains concise and action-complete.
- The reconstructed real AccessKit snapshot matches the declared semantics;
  a pilot-only semantic node is not accepted as accessibility evidence.
- A required toolkit gap was extended through protocol → runtime → runner →
  authoring → Test Lab → conformance, never patched only in DevPULSE.

## Current references

- Architecture: `docs/plans/eguipilotai.md`
- Playwright parity/gaps: `docs/eguipilotai-playwright-review.md`
- Standing product roadmap: `docs/plans/roadmap.md`
- Engineering research index: `docs/research/eguipilotai/README.md`
- Dependency-ordered delivery: `docs/plans/eguipilotai-v1-delivery.md`
- Launch matrix: `.eguipilotai.toml`
- Shell example: `tests/eguipilotai/shell-navigation.toml`
