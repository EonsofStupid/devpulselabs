# WIREDFRONT CAPTURE & SHARE SPEC — PixVix + DevPulse Live

> **Repo note (DevPULSE Labs, 2026-07-13):** operator-provided spec, filed verbatim
> below. THIS repo implements the **rrweb session-capture slice** of the vision:
> capture (in-app window + browser extension) → inbox → review keep/dump → library
> (see old-app:docs/desktop.md, Pass W). The platform engine concerns — scap/str0m forks,
> DevPulse Live P2P, Turso/Qortex gallery metadata, AI tagging/highlights, channels —
> live in the DevPulse platform, not here. Do not implement them from this file alone.
> The ADR numbers the verbatim spec cites (ADR-008/009/012, the "ADR-010 stack",
> the "ADR-011 fork policy") are the DevPulse PLATFORM's ADR series, not this repo's
> `docs/design/` register: this repo's ADR-011 is Imagine generation.

---

| Field | Value |
|-------|-------|
| **Status** | `DRAFT — awaiting operator ratification` |
| **Version** | `2.0.0-draft` |
| **Last Updated** | `2026-07-13` |
| **Owner** | `@jesse` |
| **Part of** | DevPulse Platform — WiredFront domain |
| **Supersedes** | (1) `WIREDFRONT_SPEC.md` v1.0.0-draft (2026-07-13) — built from wrong sources, invalid; (2) `HUB_ROUTE_HIERARCHY.md` v0.1.0 §Hub 4 — early sketch; (3) treats the following as consolidated source material: **ADR-009** (WiredFront + PixVix, locked), the operator-authored **"WiredFront: Technical Reference & Vision"** living document (April 2026), the **vision/capture spec v1.2.0** (Dec 2025 — manual capture UX, video settings, channels, module structure), and the **P2P serialization decision** (April 2026, protobuf rejected). |
| **Explicitly OUT of scope** | ViziWiz editors (video/audio editing, ADR-010 stack) — own spec. Minion animation (Rive / Blender / sprite question) — **parked, undecided, separate fork**. Gallery editing tools. |

---

## 1. Vision — The "Zed" Standard (non-negotiable framing)

WiredFront is **NOT** a wrapper for Twitch/YouTube and **NOT** a capture utility. It is the frontend for DevPulse's **proprietary in-house broadcasting and collaboration engine**, built to rival Zed's collaboration layer.

Three pillars (verbatim intent from the operator's living document):

1. **Native performance** — GPU-accelerated rendering for streams and captures.
2. **In-house protocol** — our own signaling and transport layer for real-time developer collaboration (**DevPulse Live**).
3. **Zero latency** — instant, peer-to-peer visual context sharing. Not "streaming to the void."

**External bridges (AngryViber, Twitch, YouTube) are destinations, not the definition of the tool.**

*Amnesia-proof definitions:*
- **DevPulse Live** — the proprietary P2P protocol, built on the `EonsofStupid/str0m` fork (pure-Rust WebRTC). Secure, team-based screen sharing and pairing.
- **PixVix** — the overlay capture tool ("The Snapper"). One tool, two modes: **Pix** (image) and **Vix** (video ring-buffer).
- **Artifact** — every capture. Metadata-first: attached Project, Task, and AI Tags at capture time.
- **Warp-share** — the operator's term for the core UX promise: capture → friend's chat, instantly, peer-to-peer, no upload-to-cloud round trip.

---

## 2. Architecture — "The Tab & The Tool"

### 2.1 The Tab: `/wiredfront` (The Hub)

Lives inside the AppFrame shell (keeps global Sidebar, Topbar, Statusbar).

| Function | Description |
|----------|-------------|
| **Control Center** | Manage Native Engine capture sessions and External Bridges |
| **Asset Gallery** | Intelligent grid of local and cloud assets (metadata store: see §7 drift correction) |
| **Session Manager** | Join/host Zed-like collaboration rooms |

### 2.2 The Tool: PixVix (The Snapper)

A **transient overlay window** — not a route, not a page.

| Property | Value |
|----------|-------|
| Triggers | Status Bar icon `[O]` · Hotkey · Hub button |
| Window | Transparent, GPU-accelerated `PixVixFrame` |
| Interface | Unified: one tool, two modes |
| **Mode A — Pix** | Precision screenshotting with automated context embedding |
| **Mode B — Vix** | Buffer-based recording (ShadowPlay-style) — the encoder is *always running into a ring buffer*; the hotkey **flushes the last N seconds to disk**, so the bug you just saw is already captured |
| Metadata | Every capture is an Artifact: Project, Task, AI Tags attached via `MetaModal` at capture time |
| AI highlights | Vix recordings get automated AI highlight detection (orchestrator-side, on-demand — never blocks capture) |

**⚠ Two ring buffers exist in DevPulse. Do not conflate them (known drift trap):**

| Ring buffer | Owner | Contents | Purpose |
|-------------|-------|----------|---------|
| **Vix ring buffer** | PixVix capture engine | Encoded video frames (last N seconds) | Instant-replay flush for bug capture |
| **VISION ring buffer** | VISION minion (ambient layer) | 50 SigLIP2 *embeddings* + metadata, raw frames discarded | "Have I seen this screen before?" novelty gating |

They share nothing but the name of the data structure.

### 2.3 Manual Capture UX (Pix mode picker)

```
[📷 Screenshot]
       │
       ▼
┌───────────┬───────────┬───────────┐
│ Monitors  │   Apps    │  Manual   │
└───────────┴───────────┴───────────┘
     │            │            │
   hover        hover        click
     ▼            ▼            ▼
 [Mon 1]     [VS Code]    Crosshair
 [Mon 2]     [Chrome]     → drag region
              ├─ Tab 1    → preview
              ├─ Tab 2    → [Share] [To Clyffy]
              └─ Tab 3
             [Cursor]
```

- Enumerates monitors, application windows, and **Chrome tabs individually**.
- Post-capture actions: **[Share]** → route to a channel (P2P/TEAM); **[To Clyffy]** → feed directly to the orchestrator for analysis. *(Superseded docs say "To Gus"; Gus → Clyffy under current naming.)*
- Region selection via `RegionOverlay` (GPU-accelerated selector).

---

## 3. Capture Engine

| Property | Value | Source |
|----------|-------|--------|
| Backend | `scap` forked from `CapSoftware/scap` (`EonsofStupid/scap`) — GPU-accelerated | ADR-009 / Forks Registry |
| C-dep status | Accepted C dep — no production pure-Rust equivalent | ADR-012 |
| Dependency style | `git=` dep, no submodules | ADR-011 fork policy |
| Video encode/mux | FFmpeg **subprocess** (never a crate dep) | ADR-009 |
| Codecs | H.264 (default), VP9, AV1 | ADR-009 |
| Resolution | **768p default**, configurable 480p–1080p | Dec 2025 v1.2.0 |
| Audio capture | Off (default) · System · Mic · Both | Dec 2025 v1.2.0 |
| Signals emitted | Typed capture signals onto the platform signal bus (tokio::broadcast) | Signal architecture |

Capture never blocks on model load — anything AI-touched (context embedding, highlight detection) is downstream and on-demand, per "spend compute proportional to signal."

### 3.1 Backend module structure (engine-side)

```
(engine) capture domain
├── capture.rs      ← scap integration, window/region capture
├── enumerate.rs    ← Monitors, app windows, Chrome tabs
├── video.rs        ← FFmpeg subprocess control, Vix ring buffer
├── share.rs        ← Channel integration (hands off to DevPulse Live)
├── types.rs        ← Screenshot, VideoFrame, CaptureTarget, Artifact
└── config.rs       ← Settings surface
```
*(Adapted from the Dec 2025 `shared/vision/` sketch; embed.rs belongs to the VISION minion pipeline, not the capture engine — separation preserved.)*

### 3.2 Frontend structure (Svelte 5, hypermodular — from the living document)

```
interfaces/desktop/src/lib/domains/wiredfront/
├── components/
│   ├── SessionManager.svelte      # Collaboration rooms
│   ├── GalleryGrid.svelte
│   └── StreamConfig.svelte
├── stores/
│   └── engine_socket.svelte.ts    # Connection to Native Engine
└── WiredHubFrame.svelte           # Layout for the Tab

interfaces/desktop/src/lib/domains/pixvix/
├── components/
│   ├── RegionOverlay.svelte       # GPU-accelerated selector
│   └── MetaModal.svelte           # "Context Capture" popup (Artifact metadata)
├── managers/
│   └── CapturePipeline.ts         # Rust bridge
└── PixVixFrame.svelte             # Transparent overlay window
```

---

## 4. DevPulse Live — P2P Sharing & Collaboration

### 4.1 The warp-share flow (the product promise)

```
Capture (Pix or Vix flush)
    │
    ▼
Artifact created (Project/Task/AI Tags via MetaModal — skippable for speed)
    │
    ▼
[Share] → pick friend / channel
    │
    ▼
DevPulse Live P2P data channel (str0m fork)
    │
    ▼
Friend's chat — media arrives directly, peer to peer.
No cloud upload. No "processing." Instant.
```

### 4.2 Channels

| Channel type | Purpose |
|--------------|---------|
| **P2P** | Direct user-to-user sharing (the warp-share path) |
| **TEAM** | Group / project channels |
| **BROADCAST** | Updates from infrastructure; external bridge fan-out |

Screenshots and video clips route through P2P and TEAM. External bridges (AngryViber feed, Twitch, YouTube, Discord relay) hang off BROADCAST — destinations, never the definition.

### 4.3 Transport & serialization (decided April 2026 — protobuf REJECTED)

| Boundary | Format | Reasoning |
|----------|--------|-----------|
| DevPulse ↔ DevPulse peers (Rust↔Rust over str0m data channels) | **bincode / rkyv** | We own both ends; zero-copy/compact beats protobuf on latency and bandwidth; rkyv 0.8.15 already the internal-boundary standard |
| Relay ↔ browser clients (AngryViber.com web) | **MsgPack / JSON** | Cross-language boundary, per locked serialization policy |
| Perimeter (Discord Gateway, any external API) | **JSON** | Locked perimeter format; Discord is WebSocket+JSON, non-negotiable |

### 4.4 Relay hub (cross-domain chat)

For DevPulse ↔ AngryViber.com user chat and the Discord relay, a relay service at devpulse.app is the hub:

```
DevPulse Desktop peer ↔ (rkyv/bincode) ↔ Relay service (devpulse.app)
                                            ↕ (MsgPack/JSON)
                                        AngryViber.com browser user
                                            ↕ (JSON)
                                        Discord Gateway (relay bot)
```

Direct peer↔peer media stays P2P over DevPulse Live; the relay exists for cross-domain presence/chat and Discord bridging, and as signaling infrastructure (see OQ-WF2-3).

### 4.5 Friends / social scaffolding (locked-in-architecture, not v1)

From the Dec 2025 decision: the friends layer is **designed into the bones now, implemented later** — never a bolt-on.

- Entity model must accommodate: Clyffy (primary AI), addressable Council minions, human friends (future).
- Thread types: AI threads, human DMs (future), shared project threads (future).
- UI scaffold instinct: **Steam-style contacts sidebar** (chat is the portal, friends always visible).
- Presence: Online / Away / In Flow — exact set undecided (OQ-WF2-5).
- Open: project-scoped collaborators vs global contacts vs both (OQ-WF2-5).

### 4.6 Security & privacy posture

- Session tokens / capability permissions via the engine security layer; credentials only in OS keyring (ADR-008 — never Turso, never TOML, never env).
- All sharing is **consent-gated**: raw operator SSOT never leaves the machine uninvited (ratified sync topology). A P2P share is an explicit per-Artifact operator action — it must not become a side-channel around the projection-DB consent model.
- TLS: rustls only (ADR-008).

---

## 5. Asset Gallery & Artifact Metadata

| Concern | Storage | Drift correction applied |
|---------|---------|--------------------------|
| Raw media | Filesystem blob store | unchanged |
| Artifact metadata (Project, Task, AI Tags, capture info, share history) | **Turso DB (CONTROL layer)** | Superseded docs say "libSQL-backed" — libSQL→Turso is locked platform-wide |
| Visual embeddings (similarity search) | SigLIP2 via `vision.embedder` slot → **Qortex** (TotalRecall) | Superseded docs say "Qdrant"/"cortex fork" — dead → Qortex |
| Project association | Filesystem symlinks | unchanged — association without duplication |
| Storage/usage analytics | DuckDB | unchanged |

Gallery schema itself remains undesigned (OQ-WF2-1).

---

## 6. Drift Corrections Ledger (superseded → current)

| Superseded material said | Current locked state |
|--------------------------|----------------------|
| "Gallery libSQL" / "libsql-backed assets" | Turso (CONTROL layer) |
| Qdrant / `EonsofStupid/cortex` fork | Qortex (TotalRecall fusion) |
| "Maestro" AI highlights / "[To Gus]" | Clyffy (operator-facing) / Council (backstage) per current ontology |
| PixVix as "image editing route" (v1.0.0-draft error) | PixVix is the overlay capture tool; editing belongs to the editors spec |
| DXGI + Win32 handwave (early sketches) | scap fork, locked |
| protobuf considered for P2P | Rejected — bincode/rkyv internal, MsgPack/JSON web, JSON perimeter |

---

## 7. ADR Register (restated as canonical for this domain)

**ADR-009 — WiredFront + PixVix** *(locked, restated)*: WiredFront is a full proprietary broadcasting and collaboration engine. DevPulse Live on `str0m` fork; PixVix overlay tool with Pix + Vix modes; scap fork capture; FFmpeg subprocess video (H.264/VP9/AV1, 768p default); external bridges are destinations.

**ADR-WF2-001 — P2P serialization boundaries** *(ratifying the April 2026 decision)*: bincode/rkyv for Rust↔Rust peers, MsgPack/JSON at the relay↔browser boundary, JSON at the perimeter. Protobuf rejected: its strengths (schema evolution across systems we don't control, cross-language codegen) don't apply where we own both ends, and it pays codegen + copy taxes with no latency win.

**ADR-WF2-002 — Capture/cognition separation**: The capture engine never blocks on, links against, or waits for model inference. Context embedding, AI tagging, and highlight detection are downstream consumers of capture signals. Reasoning: capture is the product's reflex arc — a dropped instant-replay flush because an embedder was cold is an unacceptable failure mode.

---

## 8. Verify-Before-Lock

| # | Item | Verify |
|---|------|--------|
| V-1 | scap fork | Windows capture API surface current (monitor/window/region + audio device enumeration); fork rev viable vs upstream |
| V-2 | str0m fork | Data-channel throughput at clip sizes; congestion behavior for multi-MB transfers; fork rev |
| V-3 | FFmpeg subprocess | Ring-buffer flush path: can the encoder run continuously into memory at <acceptable CPU, and flush N seconds without re-encode? |
| V-4 | Chrome tab enumeration | Mechanism (CDP? UIA?) — never specified; must be pinned before enumerate.rs is written |
| V-5 | Turso | Artifact-metadata write patterns on current build (GA pending, fallback clause active) |
| V-6 | Qortex | SigLIP2 vector registration in TotalRecall CF layout |

---

## 9. Open Questions (Parking Lot)

| ID | Question | Notes |
|----|----------|-------|
| OQ-WF2-1 | Gallery/Artifact schema (Turso tables, tag model, share history) | Unwritten |
| OQ-WF2-2 | Vix ring buffer sizing — duration (30s? 60s? configurable?), memory cap, encode-in-buffer vs raw-frames-in-buffer | Direct product-feel decision |
| OQ-WF2-3 | DevPulse Live signaling — does the devpulse.app relay double as the signaling server? NAT traversal / TURN fallback story? | Required before any P2P build |
| OQ-WF2-4 | Video frame-sampling config for AI processing (`keyframe_only`, `sample_rate_hz=0.5`, `max_frames=30`, `vl4b_strategy="summary"`) | Proposed Dec 2025, **never confirmed by operator** |
| OQ-WF2-5 | Friends model: project-scoped vs global contacts vs both; presence indicator set; PSYCH-derived mood presence (flagged "could be creepy") | Scaffold now, decide before social v1 |
| OQ-WF2-6 | Discord relay scope & ToS posture (bot vs user-token) | From April 2026 discussion |
| OQ-WF2-7 | MetaModal friction: mandatory vs skippable metadata on warp-share (speed vs hygiene) | UX decision |
| OQ-WF2-8 | AI highlight detection pipeline for Vix (what model slot, what triggers) | Downstream, post-capture-engine |

---

## 10. Changelog

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0-draft | 2026-07-13 | Full re-authoring from correct sources (ADR-009, operator's WiredFront living document, Dec 2025 capture spec v1.2.0, April 2026 serialization decision). Supersedes v1.0.0-draft, which misidentified PixVix as an image editor and omitted DevPulse Live, Vix, channels, and the relay architecture entirely. Scope narrowed to capture + share per operator directive; editors and minion-animation (Rive/Blender) parked as separate forks. Drift ledger added. Two-ring-buffers disambiguation added. |
| 1.0.0-draft | 2026-07-13 | **SUPERSEDED — built from wrong sources.** Retained per archive-never-delete. |

---

*Editors (ViziWiz / ADR-010 audio stack) and minion animation (Rive vs Blender vs sprite — undecided) get their own specs. This document owns capture and peer-to-peer sharing only.*
