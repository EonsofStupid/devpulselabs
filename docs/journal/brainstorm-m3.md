# BS1 · M3 journal — assist populate, promote, close-out (2026-08-07)

**Done this milestone:**
- **`SuggestClient`** (dpl-chat-imagine): the text sibling of
  `ImagineClient` — same vendor, same key law (`XAI_API_KEY` read at
  call time, never stored, never in an error), `suggest_request_json`
  (seed + already-on-board dedupe brief) + `parse_suggest_response`
  (line-split, bullet-strip, case-insensitive dedupe, cap 6) +
  blocking system impl on the bounded-worker discipline.
- **The populate drive** (app/chat.rs): `✦ Suggest ideas` (Diverge +
  assist ON only) → `BrainstormAssist` intent chain → host guards
  honestly (assist off / no board / unseeded / no key — each a named
  status, NO job minted), then mint-before-worker, `Queued { label:
  "brainstorm · populate" }`, one terminal transition;
  `poll_brainstorm_assist` in the frame drive lands proposals as
  Assist-origin items through the SAME `apply`, ONE checkpoint per
  wave (a proposal wave is one gesture, one undo step).
- **Promote announce**: a governed inline `ArtifactPreview`
  ("Intake pre-fill staged · N answers…") via
  `register_brainstorm_promoted` — the promote is a durable transcript
  fact, not a status line the next op overwrites.
- **`IntakePrefillDoc` document-law pin**: round-trip + newer-refusal
  + no tmp residue (`intake_prefill_round_trips_crash_safe`).
- **EguiPilotAI**: `tests/eguipilotai/chat-brainstorm.toml` + the
  `chat-brainstorm` fixture — PASSED, 6/6 steps: viewport open, focus,
  `chat.brainstorm.phase == "cluster"`,
  `chat.brainstorm.cluster/open == "The opening"`. Two findings fed
  back into the scenario: (a) a capture step among 3 live viewports
  needs `[steps.viewport]` (the platform-sees-providers idiom);
  (b) the chat peer window is a DEFERRED viewport and eframe 0.35
  cannot deliver its queued screenshot requests — capture dropped with
  the reason inline; chat-native-proof is the pixel contract.

**Proof:** 35 workflows · 44 window · 37 app · 7 imagine tests green;
scenario 6/6 `"outcome": "passed"`; orphaned exes killed after runs.

**Close-out:** STATE entry written; PR #220 → ready → double-green →
squash-merge; exe rebuilt in THE checkout with mtime verified.
