# W9c — unified `land()`: Ask mode · LandingCard · every vehicle

Program: Guided Landing (charter `guided-landing-charter.md`). Completes
W9 after W9a (`dpl-landing` #57) + W9b (organized Auto stills #58).
Gate: `wiredfront-experience.md`. Ground-truthed 2026-07-26 on
`crates/viziwiz` + `crates/landing`.

## 1. Context

W9a/W9b gave every still an organized Auto path
(`inbox/<bucket>/<kind>/<name>.<ext>`) and the shared policy model.
Video/session still dump flat (`vix` / `session-<stamp>`); Ask mode and
the reusable card exist only as types/comments; guide keeps a bespoke
`PendingGuide`/`save_card`. W9c makes ONE landing path for every PixVix
vehicle behind the operator's Auto/Ask toggle.

## 2. Current state (ground-truthed)

- `dpl_landing::{CaptureKind,LandingMode,LandingPolicy,…}` — complete;
  `LandingMode::Ask` never read by viziwiz.
- Stills: `land_target` + immediate `index_capture` (Auto-only).
- Video: `unique_video_path` → flat `vix.*`; finish → `index_video`.
- Session: `session::session_dir` → flat `session-<stamp>/`; finish →
  `index_session`.
- Guide: `PendingGuide` + `save_card` (strict meta); files flat inbox.
- Policy JSON load at `Viziwiz::new`; no palette toggle/save UI.
- `BUCKET_SEEDS` / `pattern.tags` unused in UI and index rows.

## 3. Target design

1. **`PendingLanding`** — kind, path (file OR folder), name, tags, guide
   dest fields, optional step count. Retires `PendingGuide`.
2. **`Viziwiz::land(pending)`** — Auto: organize + index + signal;
   Ask: hold + show card; Discard deletes the artifact.
3. **Every vehicle** finishes through `land()`; video/session mint
   organized targets at START (poster-stem / folder-contract preserved).
4. **`LandingCard`** — generalized `save_card`; Guide keeps strict meta.
5. **Policy strip** — Auto/Ask + bucket + name template + default tags →
   `.userdata/landing-policy.json`.

## 4. Changes (execution order)

1. `PendingLanding` + `land()` / discard; stills routed through it.
2. Video + session organized mint + `land()` at finish.
3. Guide folded into `PendingLanding` + `LandingCard`.
4. Policy strip UI + `pattern.tags` on index rows; `CaptureKind` at all
   index builders; `{date}` filled in `NameContext`.

## 5. Boundaries

- No assets/-ring migration; no W10; no browser on-spine (W9.1).
- No gallery regrouping pass (index tags/bucket first; UI grouping later).
- No `#[allow]`; sources never mutated.

## 6. Verification

- `cargo green` (`env -u CARGO_TARGET_DIR` on this host).
- Units: land Auto/Ask for image; discard removes file; guide still
  refuses empty meta; video poster stem unique under organized path.
- `green-linux` on the PR before squash-merge.

## 7. Commit plan

Branch `feat/w9c-unified-landing`. Commits follow §4. PR → green-linux →
squash-merge → tip STATE. Estate remotes kept separate (rebase conflicted;
own pass).

## 8. Evidence (2026-07-26)

- `PendingLanding` + `land()` / Auto|Ask; stills/video/session/guide routed.
- Video/session mint organized paths at start; LandingCard + policy strip
  (Auto/Ask + bucket cycle) persist to `.userdata/landing-policy.json`.
- `cargo green` ✔ locally (`env -u CARGO_TARGET_DIR`); `green-linux` on PR.
- Units: dpl-landing 5 + dpl-viziwiz 7 green; clippy `-D warnings` clean.
