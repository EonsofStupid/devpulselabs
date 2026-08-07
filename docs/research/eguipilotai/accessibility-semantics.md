# Accessibility and semantic registration contract

Status: specified, not implemented. The current integration is partial and
must be corrected before EguiPilotAI claims accessibility snapshot coverage.

## Finding: the two trees are not yet one contract

The canonical architecture correctly requires automation and accessibility
metadata to originate from the same registration. The current code does not
fully satisfy that requirement:

- `EguiPilotPlugin::setup` enables AccessKit.
- `pilot_track` passes only `WidgetInfo::labeled(widget_type, enabled, name)`
  to egui, then separately records richer `NodeMetadata` in the pilot tree.
- heading, status, and alert roles collapse to an egui label; other roles can
  collapse to `Other`.
- value, checked/selected/expanded/error/description and many supported
  actions are not projected into the AccessKit node.
- the output hook records only whether an AccessKit update existed. It does
  not retain, apply, inspect, or assert the real `TreeUpdate`.
- AccessKit `ActionRequest` input is not a tested automation path.

Therefore `PilotSemanticSnapshot` is currently useful, but it is not evidence
that the operating system accessibility tree contains the same information.
Documentation and reports must call the two products by their exact names.

## One registration, two verified projections

The owner creates one `SemanticRegistration` beside the real response. It is
the source for:

1. `PilotSemanticNode`: stable domain identity, hierarchy, geometry, clip,
   source location, state, supported pilot actions, and agent projection data.
2. `AccessKit` enrichment: real egui/AccessKit role, label, description,
   value, numeric range, toggled/selected/expanded/invalid state, relations,
   supported accessibility actions, bounds, and focus.

The projections may differ only where their platforms genuinely differ. Every
difference is covered by an explicit mapping rule and parity test. There is no
second hand-authored accessibility tree.

## Identity and correlation

Each live registration records:

- stable `PilotPath`;
- owning `ViewportId` and semantic parent;
- the real `egui::Id` from the response;
- `accesskit::TreeId` and `accesskit::NodeId` derived from that response;
- source location;
- current semantic revision and AccessKit update revision.

Standard wrappers create the widget inside a `push_id(PilotPath)` scope.
Custom-painted controls need a response-creating registration helper that
supplies an explicit egui ID derived from the path; that helper has never been
built, and today custom paint attaches `pilot_track` to a response the owner
creates. Under this contract, `pilot_track` on an arbitrary auto-generated
response remains possible only as an escape hatch and emits a diagnostic until
its stable ID correlation is proved.

Duplicate `PilotPath` remains an immediate error with both source locations.
Duplicate AccessKit correlation or one pilot path changing AccessKit node ID
within a viewport lifetime is also an immediate error.

## Widget helper surface

The boring public lane should cover the recurring DevPULSE controls:

- `pilot_button`, `pilot_link`, `pilot_tab`, `pilot_menu_item`;
- `pilot_label`, `pilot_heading`, `pilot_status`, `pilot_alert`;
- `pilot_checkbox`, `pilot_radio`, `pilot_radio_group`;
- `pilot_text_edit`, `pilot_slider`, `pilot_combo_box`;
- `pilot_progress`, `pilot_image`;
- `pilot_region`, `pilot_dialog`;
- a response-creating interaction helper for custom paint and hit behavior
  (never built; today's `pilot_track` only instruments an owner-created
  response);
- `pilot_drag_source` and `pilot_drop_target` as semantic specializations of
  that interaction helper.

Wrappers return the original egui `Response` and never hide application state
mutation. They register after the widget has finalized its changed/focused
state. A wrapper may enrich egui's existing `WidgetInfo`; it must not discard
information emitted by the native widget.

## Required projection mapping

| Pilot metadata | AccessKit projection | Parity expectation |
|---|---|---|
| button/link/checkbox/radio/text input/slider/combo/image/progress/window | corresponding AccessKit role | exact |
| heading/status/alert/region/dialog/tab/menu item | explicit AccessKit role through node builder | exact where AccessKit supports it |
| `name` | label, or value for a pure label node | normalized text equal |
| `description` | description/help text | equal |
| text `value` | string value | equal |
| numeric value/min/max/step | numeric fields | equal within serialized precision |
| `checked` | toggled state | exact, including mixed when supported |
| `selected` | selected state, not overloaded toggled state | exact |
| `expanded` | expanded/collapsed state | exact |
| `error`/invalid | invalid state plus described error node/relation | exact relationship |
| disabled/editable/focused | native state/action exposure | exact |
| semantic parent/label relation | AccessKit child and relation IDs | structurally equivalent |
| pilot action | AccessKit action when assistive technology can invoke it | declared mapping |

When egui's `WidgetInfo` cannot express a field, the plugin uses
`Context::accesskit_node_builder(response.id, ...)` after native widget info is
registered. Direct enrichment must preserve egui's bounds, focus, text
selection, and native actions.

## Capturing the real AccessKit tree

AccessKit defines `TreeUpdate` as an atomic delta against a synchronized prior
tree and includes the latest focus. The plugin therefore maintains a separate
mirror per AccessKit tree/viewport:

1. capture the actual `FullOutput.platform_output.accesskit_update`;
2. validate its tree identity and focus;
3. apply changed nodes atomically to the prior mirror;
4. retain the raw delta for trace and the reconstructed tree for assertions;
5. tie the applied update to the pilot semantic revision committed in the
   same pass;
6. fail with `AccessibilityDesynchronized` on unknown removals/parents,
   duplicate nodes, invalid focus, or missed revision continuity.

The complete reconstructed AccessKit tree stays local. Agent snapshots remain
the pruned pilot projection because it carries stable domain IDs, but every
actionable node includes an optional accessibility parity status. A parity
failure is never pruned.

## Snapshot products

Names are explicit:

- `PilotSemanticSnapshot`: stable automation view, full or pruned.
- `AccessKitSnapshot`: normalized reconstruction of the actual AccessKit
  output.
- `AccessibilityParityReport`: mapping and differences between the two.

An `accessibility-snapshot` assertion targets `AccessKitSnapshot`. A
`semantic-snapshot` assertion targets `PilotSemanticSnapshot`. A scenario can
require both. Reports must never label a pilot-only snapshot “AccessKit” or
“accessibility tree.”

Normalized AccessKit snapshots omit generated numeric node IDs by default and
key matched nodes by `PilotPath`. Uncorrelated native nodes remain visible
under a deterministic generated key so coverage gaps cannot disappear.

## Accessibility actions

The runtime may exercise the actual assistive-technology path by queuing
`egui::Event::AccessKitActionRequest` with the correlated tree/node IDs.
Supported v1 actions:

- focus;
- click/activate;
- increment/decrement for native range controls;
- set value where egui and AccessKit support it;
- scroll into view where an owner adapter exists.

An accessibility action is traced as `accessibility-action`, not pointer or
keyboard input. Its postcondition is asserted against both the application
semantics and the next real AccessKit update. Unsupported actions fail during
preflight.

Keyboard-only testing remains separate and mandatory. Passing an AccessKit
click does not prove Tab order, focus visibility, arrow-key group behavior,
Enter/Space activation, or Escape dismissal.

## Authoring pattern

For a custom choice control, the owner supplies stable identity and state at
the real hit rectangle:

```rust,ignore
let id = CHOICE.child(&PilotKey::new(choice.id));
let response = ui.interact(rect, ui.id().with(id.as_str()), egui::Sense::click());
ui.pilot_track(
    &response,
    NodeMetadata::new(id, Role::Radio, choice.label)
        .parent(CHOICE_GROUP)
        .selected(model.selected == Some(choice.id)),
);
if response.clicked() {
    model.select(choice.id);
}
```

The example uses today's real `pilot_track` surface. The egui ID scope is the
caller's convention, not a recorded correlation, and disabled state is read
from the real response rather than the metadata — the correlation and
enrichment contract above remains unimplemented. The important rules are
stable ID, real response, truthful state, semantic parent, and one
registration.

## Diagnostics

Required errors:

- `DuplicatePilotId`;
- `UnstableAccessKitCorrelation`;
- `UnsupportedRoleProjection`;
- `SemanticAccessibilityMismatch`;
- `AccessibilityDesynchronized`;
- `InvalidAccessibilityFocus`;
- `UnsupportedAccessibilityAction`;
- `MissingAccessibleName` for actionable controls;
- `KeyboardPathMissing` where a component contract requires one.

Each diagnostic includes source location, pilot path, viewport, pilot and
AccessKit revisions, normalized expected/actual fields, and nearby hierarchy.

## Conformance gates

- every standard wrapper produces the expected real AccessKit role/state;
- custom-paint registration has stable pilot/egui/AccessKit correlation,
  which requires the response-creating helper that does not yet exist;
- TreeUpdate deltas reconstruct the same tree as a full rebuild;
- focus is current on every applied update;
- label/value/checked/selected/expanded/disabled changes appear in both
  projections on the same committed pass;
- focus, click, increment/decrement, and supported set-value requests reach
  the real widget behavior;
- semantic and AccessKit snapshot matchers produce reviewable diffs;
- uncorrelated native nodes and pilot-only actionable nodes fail parity;
- keyboard traversal order and visible focus pass in the real DevPULSE
  process;
- the default agent snapshot budget remains unchanged by retaining full
  AccessKit data locally.
