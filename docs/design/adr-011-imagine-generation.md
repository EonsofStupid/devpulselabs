# ADR-011 — Chat Imagine generation exception

**Status:** DRAFT — awaiting operator ratification; implementation may land
fail-closed (inert without an operator-provisioned key)
**Date:** 2026-07-27
**Owner:** DevPULSE operator
**Sits under:** [`../plans/chat-creation-and-workflows-vision.md`](../plans/chat-creation-and-workflows-vision.md)
**Sibling exception:** [`../plans/chat-provider-runtime-adr.md`](../plans/chat-provider-runtime-adr.md)

## Context

The operator commissioned in-chat image/video creation ("mode button to
trigger create video or create image", drafts previewable and promotable to
the library). Grounding on 2026-07-27 established:

- Grok Build 0.2.112 — the subscription-backed client DevPULSE activates —
  is a coding agent. Its full command surface (`--help` against the staged
  executable) has **no generation capability**, and the CLI docs mention
  none.
- xAI's only documented programmatic generation surface is the **Grok
  Imagine API**: `POST https://api.x.ai/v1/images/generations`
  (`grok-imagine-image-quality`; sync; `url`/`b64_json` out) and
  `POST https://api.x.ai/v1/videos/generations` +
  `GET /v1/videos/{request_id}` (`grok-imagine-video-1.5`; async polling;
  duration ≤ 15 s). Auth is **`Authorization: Bearer $XAI_API_KEY` only** —
  no OAuth or subscription path exists.

The runtime ADR rejected "direct HTTP APIs and API keys" **for conversation
transport**, where official subscription clients exist. For generation no
such client exists; the API key is the only official door. Without a new,
narrow decision the commissioned feature is impossible.

## Decision (proposed)

> Permit one named generation adapter (`dpl-chat-imagine`) to call exactly
> the xAI Imagine endpoints, authenticated by an operator-owned key that
> DevPULSE reads but never stores. Everything else keeps the existing laws.

## Allowed

- The adapter may POST to `https://api.x.ai/v1/images/generations` and (in
  a later slice) drive `POST /v1/videos/generations` + `GET
  /v1/videos/{request_id}`. No other hosts or paths; the base URL is
  compiled in, never configurable by model output, documents, routes, or
  MCP arguments.
- The key is read at call time from the `XAI_API_KEY` environment variable
  of the operator's session. A later slice may add OS-keychain custody
  (Windows Credential Manager / libsecret via a vetted crate); DevPULSE
  never writes, copies, logs, echoes, or persists the key, and it never
  appears in facts, evidence, traces, or errors.
- Generation runs only from an explicit operator click in Chat (mode
  button). No autonomous, retried, scheduled, or model-initiated
  generation. One in-flight request per Chat window; bounded timeouts.
- Prompts are the operator's composer text; sending them to xAI is the
  explicit purpose of the click.
- Results land in the app-owned **draft cache** (`.userdata/chat/
  imagine-cache/`), never directly in `content/`. Promotion to the
  WiredFRONT raw-media library is a separate explicit operator action using
  the existing import seam. The 30-day flush governs only the cache and
  asks consent (yes / never show again / remind me next time).

## Forbidden

- Using the Imagine key for chat conversations, or any Grok Build
  credential for Imagine — the two auth worlds never mix.
- Persisting the key or deriving app-owned credential files.
- Unbounded spending: no batch `n` > 4, no auto-regeneration, no polling
  without an operator-initiated request.
- Generation requests whose prompt is sourced from model output without the
  operator seeing and sending it.
- Writing cache artifacts anywhere but the imagine cache; deleting anything
  under `content/`.

## Cost

Imagine is usage-billed (order of $0.02/image). The UI labels the model
next to the action; the operator's click is the spend authorization.

## Ratification checklist

- [ ] Operator approves the narrow Imagine API exception and key custody
      (env-var read-at-use, keychain later).
- [x] Endpoints, schemas, and auth grounded against docs.x.ai (2026-07-27).
- [x] Draft-cache custody and flush-consent model defined (vision doc §2).
- [ ] First live image generation performed by the operator with their key.

Until ratified, the shipped code stays inert: without `XAI_API_KEY` the
mode button reports the truthful "no key provisioned" state and nothing
can be sent.
