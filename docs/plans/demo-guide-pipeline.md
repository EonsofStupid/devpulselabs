# DEMO-GUIDE PIPELINE — request → capture → customer `.guide`

| Field | Value |
|-------|-------|
| **Status** | `DRAFT — awaiting operator ratification` |
| **Ledger** | Extends #117 (Program DEMO) and Program G; composes S1/S2, Steps, W5, EguiPilotAI, PixVix |
| **Authored** | 2026-07-25 |

## 1. The target experience (the operator's sentence)

> "I request the demonstration for you to capture for my customer, and
> then I can provide them the self-contained `.guide`."

The loop, end to end:

1. Operator asks (chat later; harness drill first): *"capture a
   demonstration of X."*
2. An **EguiPilotAI scenario drives the real exe** deterministically.
3. **Recorders capture while the pilot drives**: the Steps recorder
   takes do→done stills at semantic beats; the S1 session recorder
   captures motion where a still can't carry it (drags, animation,
   timing feel). PixVix snap fills any one-off framing.
4. The captures assemble into ONE guide that **mixes still steps and
   replay segments** in sequence.
5. Operator reviews/tweaks in the W5 reviewer (reorder, caption, trim).
6. **Export ONE self-contained `.guide`** (html + mp4 + gif, embed
   default — W5/P1 ratification) and hand it to the customer.

The customer is the new audience: the exported player is now the face
of the product to an outsider, so ColdLight polish on the player is a
requirement, not a nicety.

## 2. Inventory — what already exists (ground-truthed 2026-07-25)

- `crates/guide/src/lib.rs` — `export_guide`: still `Step`s +
  **one** `SessionRef` (`session.mp4`), embed-vs-sidecar
  (`ExportOpts.embed_video`), `replay.gif`, optional `guide-video.mp4`,
  closed-bundle tests.
- EguiPilotAI — deterministic driving via named `PilotId`s,
  renderer-keyed captures, CLI + `eguipilotai-devtools` exe; scenario
  files in `tests/eguipilotai/`.
- S1/S2 — native session record + in-app playback (S3 export pending).
- Steps recorder — OS-global click loop, do→done frames,
  `--steps-drill` / `--steps-hotkey-drill`.
- PixVix — `--pixvix` / `--pixvix-snap` harness seams.
- W5 reviewer — reorder/delete/caption over `guide.json`
  (`engine-steps::edit::update_guide`).
- ffmpeg subprocess lane (`engine-media`, ADR-009) — the only legal
  encode/cut path.

## 3. The gaps (untracked until this doc)

- **G-1 · Mixed media schema.** `guide.json` has no replay-segment
  step kind. A guide is stills + at most one whole-session attachment.
  Needed: `Step` kind `still | replay { src, in_s, out_s }` so replay
  clips interleave with stills as first-class steps.
- **G-2 · Segment extraction.** Cutting `session.mp4` into per-step
  clips — ffmpeg subprocess, never a crate dep.
- **G-3 · The composition itself.** "Pilot drives while recorders
  capture" is described in Program DEMO but no code path runs it. The
  chat tier waits on F3 (typed chat tools); a **harness drill can
  compose it today** and becomes the tuning instrument (see §5).
- **G-4 · Player transport over mixed steps.** The exported player
  must step still→clip→still with one VLC-like transport (G1's spec),
  scrub within clips, keyboard + touch.
- **G-5 · Customer polish bar.** G2 items (poster/OG card, size-budget
  report, autoplay demo mode) get pulled forward — a customer
  deliverable can't ship without a size budget and a poster.
- **G-6 · Secret redaction.** Surfaced by DEMO-001: external-tier
  captures (credential consoles, admin forms) contain secrets. A
  customer `.guide` ships ONLY after a redaction pass (blur/patch in
  stills and clips); the exporter eventually enforces a per-guide
  redaction checklist. Interim: manual redaction + demo accounts with
  keys revoked immediately after capture.

## 4. Pass breakdown (proposal)

- **DG-B — browser driver. ✅ LANDED 2026-07-25**
  (`docs/plans/demo-browser-driver.md`, ADR-013): `dpl-engine-browser`
  drives an installed Chrome/Edge headless over CDP — the external
  tier's capture vehicle. Screencast-to-clip is additive on this seam.
- **DG-B.1 — persistent demo profile. ✅ LANDED same-day** (operator:
  the flow must be self-contained/portable — no extension): opt-in
  `BrowserConfig` named profile dir (never the operator's real one) +
  headful mode; `browser_cli login <dir>` = the one-time sign-in
  ceremony (Clyffy, demo Twilio), then captures ride the session.
  Credentials stay operator-side always — the agent never enters
  passwords.

- **DG1 — mixed-media guide.** Schema extension (G-1) + segment cut
  (G-2) + export bundles N clips + player handles mixed steps (G-4).
  Pilot: fixture guide (2 stills + 1 clip) → engine-unit asserts the
  bundle is CLOSED; headless-browser drill steps it end-to-end.
- **DG2 — `--demo-drill <scenario>`.** One flag: pilot scenario drives
  the exe while Steps + S1 record; the run assembles a draft
  mixed-media guide and files it through the inbox (sources never
  touched). This is the request→capture loop minus chat, and the
  reproducible tuning instrument.
- **DG3 — chat request tier.** After F3: "record a demonstration of X"
  in chat triggers DG2's engine path. No new capture code — routing
  only.
- **DG4 — customer polish.** G2 pulled forward: poster, size-budget
  report at export, optional autoplay demo mode.

Sequencing: DG1 → DG2 (the tuning loop opens here) → DG4; DG3 rides
F3's lane whenever it lands. W5 reviewer grows replay-step awareness
inside DG1 (caption/trim a clip step).

## 5. The tuning protocol (how "get this working like I would like" is tracked)

The wiredfront-experience §5 pattern, applied here:

1. Every demonstration request becomes a **named scenario file**
   `tests/eguipilotai/demos/<name>.toml` — deterministic, re-runnable,
   diffable.
2. Each tuning round = ONE sequential `--demo-drill` run producing the
   draft guide + exported `.guide` (proof-shot runs stay sequential —
   index law).
3. Each round appends a row to the friction audit below. A round
   without a moved row is polish on assumption (W5-shape memory:
   don't).
4. Screenshots/exports at REAL window sizes, never only maximized.

### Registered scenarios

| # | Scenario | Tier | Storyboard | Status |
|---|----------|------|------------|--------|
| 001 | Twilio keys for the admin-layer notification service | EXTERNAL (browser) | [demos/demo-001-twilio-notification-keys.md](demos/demo-001-twilio-notification-keys.md) | Storyboard authored; capture round 1 pending (needs demo Twilio account + operator's Chrome session) |

Note: the first example being EXTERNAL-tier is deliberate — it forces
the redaction law (G-6) and exercises the browser lane early. Round 1
capture is sanctioned as hand-cranked (Claude-in-Chrome + Steps + screen
record) BEFORE the browser-driver ADR exists; the ADR pass still happens
before external demos become routine.

### Friction audit (append one row per tuning round)

| Date | Scenario | What felt wrong | Ruling / fix |
|------|----------|-----------------|--------------|
| — | — | *(first capture round opens this table)* | — |

## 6. Open questions for ratification

| ID | Question | Recommendation |
|----|----------|----------------|
| DG-Q1 | Interleaved clips in ONE `.guide` vs stills-guide + attached whole replay (today's shape)? | Interleave — it's the operator's stated mix |
| DG-Q2 | Customer default: single-file embed or folder + sidecars? | Embed (already the W5/P1 default), size budget enforced by DG4 |
| DG-Q3 | Clip caps for customer delivery (max seconds/clip, total MB)? | Budget report first (DG4), caps only if reality demands |
| DG-Q4 | Do DG1/DG2 precede F3? | Yes — tuning must not wait on the chat lane |
| DG-Q5 | Where do demo scenarios live? | `tests/eguipilotai/demos/` (code, versioned) — never `/content` |

## 7. Roadmap amendment (apply in this pass's PR)

Under **Program DEMO**, after the in-app/external tier bullets:

> - **DG passes (docs/plans/demo-guide-pipeline.md):** DG1 mixed-media
>   guide (replay-segment steps) → DG2 `--demo-drill` (pilot drives,
>   recorders capture, guide assembles — the tuning instrument) → DG3
>   chat tier (after F3) → DG4 customer polish (G2 pulled forward).
>   The customer-deliverable `.guide` loop is tracked and tuned here.

## 8. Evidence

- `crates/guide/src/lib.rs:31-140` — `Step`, `SessionRef` (single),
  `ExportOpts.embed_video`/`make_gif`, `session.mp4` copy path.
- `crates/guide/src/lib.rs:419-495` — `export_guide`: one session,
  `replay.gif`, `guide-video.mp4`.
- `docs/plans/roadmap.md:194-220` — Program G (G1/G2, closed-bundle
  pilot law); `:292-314` — Program DEMO (ledger #117, in-app tier
  waits on F3, external tier needs its own ADR).
- `docs/design/wiredfront-capture-share-spec.md` — PixVix/capture
  engine spec (capture side, not the guide side).
- Memory (W5/P1): `.guide` = html + mp4 + gif, embed default; W5 shape
  not final — tune via friction audit, don't polish on assumption.
