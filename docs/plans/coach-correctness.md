# GF1c — Coach boundary and matching correctness

Status: open. F3-F6 are not fixed. Two first files for F3 were never wired in and
are kept in the local tag `retired/guided-workflow-snapshot`, not on main.

## F3 grounded cut

The normal app registers CoachHub, whose current module imports foreign inventory,
fabric and runtime unconditionally and can invoke/launch a foreign overlay. These
dependencies bring foreign accessibility code into the normal app graph. The
ratified coach-overlay ADR permits this only in devtools owners.

Retain the Coach route and its owning HubId/SubId constants. Put the existing
foreign implementation behind the existing eguipilotai-devtools feature, with
optional hub dependencies and feature forwarding from the app. Normal builds
show an honest unavailable state without scanning, invocation or launch code.
The overlay executable itself must require explicit devtools feature selection.
Preserve the existing foreign implementation and its tests, with no claim of new
Windows/Wayland support. Add an executable dependency-graph gate proving a normal
app has no foreign Coach crates; qualify both default and devtools build paths.

## F4-F6 contract to ground before implementation

Visibility must use committed visibility rather than mere node presence. Armed
mounts must preserve the complete locator and distinguish temporary absence from
invalid, ambiguous or mismatched targets. Outcome predicates must not infer child
absence from a root-only frame; their scope and lifetime must be independent of
highlight painting. Keep asynchronous outcome waiters alive after a highlight
disappears, bounded by explicit TTL/unmount. Source-grounded details and tests will
be added before this portion is edited; do not claim it fixed from F3 alone.

## Evidence

Run affected unit tests, default/devtools dependency and compile checks, full
repository gate, and a real native candidate startup/screenshot. Record actual
runtime qualification separately from metadata/fake-input tests. Do not weaken
the ratified devtools boundary or assert attempted input proves task success.
