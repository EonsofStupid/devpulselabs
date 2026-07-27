# Chat Imagine drafts — image generation with a governed draft cache

**Status:** EXECUTED — see §8
**Date:** 2026-07-27
**Governing ADR:** [`../design/adr-011-imagine-generation.md`](../design/adr-011-imagine-generation.md) (draft; code ships inert without a key)
**Program:** [`chat-creation-and-workflows-vision.md`](chat-creation-and-workflows-vision.md) §2

## 1. Context

The operator commissioned in-chat image/video creation with a draft cache:
mode button → generate → preview → save to library or ignore; unsaved
drafts flushed after 30 days behind a consent prompt. Grounding (2026-07-27)
established Grok Build has no generation surface; the only official door is
the xAI Imagine API (`POST https://api.x.ai/v1/images/generations`, model
`grok-imagine-image-quality`, Bearer `XAI_API_KEY`, `b64_json` response).
This pass ships **image generation end-to-end** and stages video (async
polling API) for the next slice. Without `XAI_API_KEY` in the operator's
environment everything renders truthfully disabled.

## 2. Current state (grounded in-session)

- `crates/chat/window/src/lib.rs` — `MEDIA_ID = "chat.composer.media"`
  (:67) rendered at :1745–1751 via `composer_capability_button` (:3172),
  enabled by `has_capability(Images|Video)`; **response discarded** — no
  click handling. `DomainOverlay` (:106) is the overlay enum
  (`ProviderCatalog`, `ProviderConfiguration`, …); `apply()` (:476) routes
  `ChatWindowIntent` → optional `HostIntent`. Approval overlays render in
  `interaction_overlays` (:2664) with a `provider_permissions` binding map;
  host-authored approvals currently have **no host callback seam**.
- `crates/composer/src/import.rs` — `import_file` (:61) copies into
  `content/inbox/` and indexes write-through (`finalize_import` :148,
  `dpl_index::…upsert`); `source` is `"import"`/`"reference"`; a generated
  artifact needs a new `"generated"` source wrapper.
- `crates/engine-capture/src/lib.rs` — `userdata_root()` (:201);
  `.userdata/cache/` is created by `ensure_rings` and documented "always
  safe to delete", currently unused.
- **No HTTP client/TLS in the workspace** (verified: no reqwest/ureq/rustls
  anywhere in Cargo.lock; axum is server-only; tungstenite is plain-TCP).
- Interaction Library: `InteractionContent::Approval { approve / always /
  reject }` must be `Overlay` placement; authored template at
  `testkit/src/lib.rs:437`. Persisted-pref precedent: `landing-policy.json`
  (`LandingPolicy::load/save`) under `userdata_root()`.
- Fabric `JobEvent { Queued, Done, Failed }` + coach ids `jobs.*` exist.

## 3. Target design

New crate **`dpl-chat-imagine`** owns: request/response types, an
`ImagineClient` seam (`SystemImagineClient` = `ureq` on a named worker
thread, one in-flight request), the **draft cache** at
`.userdata/cache/imagine/` (artifact files + `index.json`), the 30-day
retention computation, and the flush policy file
(`.userdata/imagine-policy.json`, `{ "auto_flush": bool }`).

Flow: media button (enabled when a key is provisioned OR the provider
qualifies Images/Video) opens a **Generation overlay**; the prompt is the
composer draft; Generate posts `n=1`, `response_format=b64_json`; the
result lands in the cache with a downscaled RGBA thumbnail; the draft strip
shows preview / **Save to library** (→ `import_generated` → `content/
inbox/`, then removed from cache) / **Ignore** (removed now). On boot, if
any unsaved draft is older than 30 days: auto-flush silently when
`auto_flush`, else raise one Approval overlay — "Yes, clear old drafts"
(flush, ask again next cycle — this is the operator's "yes" and "remind me
next time"), "Yes — never ask again" (flush + persist `auto_flush`),
"Not today" (defer, flush nothing). Generation lifecycle publishes
`JobEvent::Queued/Done/Failed` ("imagine image" label; no prompt content).
Video appears as a disabled mode with a truthful "staged next pass" reason.

## 4. Changes, in execution order

1. **`Cargo.toml`** (workspace): member `crates/chat/imagine`; deps
   `dpl-chat-imagine = { path = … }`, `ureq = "2"` (default features =
   rustls TLS), `data-encoding` (already in the lock via tungstenite).
2. **`crates/chat/imagine/Cargo.toml` + `src/lib.rs`** (new): types
   (`DraftId`, `GenerationDraft { id, prompt, model, created_ms, file,
   thumb_png? }`), `ImagineCache` (`load/add/list/remove/promote_source`),
   `retention_due(now)`, `flush_expired(now)`, `ImaginePolicy
   { auto_flush }` load/save, `ImagineClient` trait +
   `SystemImagineClient::generate_image(prompt) ->
   Result<Vec<u8>, String>` (POST, 90 s timeout, parses `b64_json`,
   decodes via `data-encoding::BASE64`), `resolve_api_key()` =
   `std::env::var("XAI_API_KEY")` (never logged). Unit tests: cache
   roundtrip, retention boundary (30 d), flush removes only expired,
   policy roundtrip, request JSON shape, response parse, key redaction
   (error strings never contain the key).
3. **`crates/composer/src/import.rs`**: `pub fn import_generated(src:
   &Path) -> Result<Record, String>` — same body shape as `import_file`
   but `source: "generated"`.
4. **`crates/chat/window/src/lib.rs`**:
   - `DomainOverlay::Generation` variant; media button click →
     `SetOverlay(Some(Generation))` (capture the discarded response).
   - `GenerationStatus { image_available, unavailable_reason, in_flight,
     last_error }` + `GenerationDraftView { id, prompt, created_ms,
     thumb: Option<(u32, u32, Vec<u8>)> }`; setters
     `set_generation_status`, `replace_generation_drafts`; lazy
     `TextureHandle` cache keyed by draft id.
   - New intents `RequestImageGeneration { prompt }`,
     `SaveGenerationDraft { id }`, `DiscardGenerationDraft { id }` →
     matching `HostIntent`s (pass-through).
   - `register_imagine_flush_consent(InteractionId)` binding; in
     `interaction_overlays`, that binding's approve/always/reject emit
     `HostIntent::ResolveImagineFlush { decision: FlushNow | FlushAlways |
     Defer }` (new `FlushDecision` enum) and transition the interaction.
   - `fn generation_overlay(…)`: nodes `chat.generation.panel` (dialog),
     `chat.generation.mode.image` (selected), `chat.generation.mode.video`
     (disabled, reason), `chat.generation.generate` (label
     "Generate image · grok-imagine-image-quality · usage-billed";
     enabled iff available && prompt non-empty && !in_flight),
     `chat.generation.draft/<id>` rows + `…/save`, `…/discard`.
5. **`crates/app/src/chat.rs`**: `Shared.imagine:
   ImagineState { cache, policy, worker: Option<Receiver>, job:
   Option<JobId> }`; boot: load cache + policy, project status + drafts,
   retention check → auto-flush or register the Approval (authored-host
   spec, overlay); host-intent arms for the three new intents +
   `ResolveImagineFlush`; frame poll drains the worker channel (decode
   image bytes → cache add → thumbnail via `image` crate downscale →
   re-project; publish `JobEvent`s). Save → `dpl_composer::
   import_generated` then cache remove; app depends on `dpl-chat-imagine`
   (+ `dpl-composer` already? verify — else route the import through the
   existing fabric/seam used by capture, fallback: direct dep).
6. **`crates/chat/testkit`** or coordinator authored-mode: authored
   fixtures set `GenerationStatus { image_available: true, … }` so the
   panel is deterministic for proofs.
7. **`tests/eguipilotai/chat-imagine-panel.toml`** (authored fixture):
   open Chat → dismiss authored approval → click `chat.composer.media` →
   expect `chat.generation.panel`, image mode selected, video mode
   disabled, generate button present; close.
8. **Docs**: STATE tip; ADR-011 already drafted; this §8 filled with
   evidence; vision doc committed alongside.

## 5. Boundaries

- 🚫 Never: persist/log the key; other hosts/paths than the compiled
  Imagine endpoint; auto/retried/model-initiated generation; writing cache
  outside `.userdata/cache/imagine/`; deleting anything in `content/`;
  new fabric variants; touching Grok/Codex activation code; `#[allow]`.
- ⚠️ Ask first: video slice; keychain custody; batch `n` > 1; any further
  dependency beyond `ureq`.
- Out of scope: video generation (staged), workflows (§3 of the vision),
  route-aware library actions, provider-qualified Images capability.

## 6. Verification

1. `cargo test -p dpl-chat-imagine` exit 0 (tests in step 2).
2. `cargo test -p dpl-chat-window -p devpulselabs` exit 0.
3. `cargo green` exit 0 (full gate; release exe mtime moves).
4. EguiPilot `chat-imagine-panel` passes on the devtools exe; the five
   existing chat scenarios stay green (sequential).
5. Real-exe truthfulness: without `XAI_API_KEY`, the generate action is
   disabled with "no key provisioned"; with a key present the button
   enables (operator-only live test — their click is the spend).
6. Cache lifecycle proof (unit-level): add draft aged 31 days → retention
   due; flush removes it; a saved draft is out of scope after promotion.

## 7. Commit plan

Branch `feat/chat-imagine-drafts`; one commit
(`feat(chat): in-chat image generation with governed draft cache (ADR-011)`);
includes the vision doc + ADR; push (gate), PR, squash-merge, release exe
rebuild + depot intact check, STATE tip in-PR.

## 8. Evidence (2026-07-27)

- Grounding: staged Grok Build 0.2.112 `--help` shows a coding agent with
  no generation surface; docs.x.ai confirms Imagine is API-key-only
  (`POST /v1/images/generations`, `grok-imagine-image-quality`,
  `b64_json`; video is async `POST /v1/videos/generations` + poll).
  ADR-011 drafted with the ratification checklist.
- `cargo test`: dpl-chat-imagine 7/7 (cache roundtrip + reopen, 30-day
  boundary + selective flush, orphan artifact drop, policy roundtrip,
  exact 4-field request shape, fail-closed response parsing, format
  sniffing); dpl-chat-window 17/17; dpl-composer 41/41.
- EguiPilotAI `chat-imagine-panel` (authored fixture) passed on the final
  build: media button opens `chat.generation.panel`; image mode
  `selected`; video mode disabled (staged); generate disabled with an
  empty prompt and enabled after filling the composer. The six existing
  chat scenarios all stayed green (sequential run).
- Visual proof `proof-chat-imagine-panel.png` (native peer viewport,
  `--state generation`): the Create-with-AI panel with the billed
  generate action and the 30-day cache statement.
- Truthful inert state: without `XAI_API_KEY` the real exe reports "Set
  XAI_API_KEY in your environment to enable image generation (ADR-011)"
  and nothing can be sent. The first live generation is the operator's
  ratification-in-practice (their key, their click, their spend).
- Deviation from plan (reported): a `generation` proof state was added to
  `ChatProofMode` + `chat-native-proof` for the visual capture — same
  pattern as the existing `codex`/`grok` states.
- `cargo green` enforced on push (§7).
