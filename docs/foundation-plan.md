# DevPULSE Labs — Pure-Rust Foundation

| | |
|---|---|
| **Status** | Ratified by the operator, 2026-07-21: Path B — start over, all UI driven by Rust |
| **Stack** | Rust 1.92 · egui 0.35 + eframe (wgpu) · tokio (sync primitives) — **no webview, no JS, no IPC, no second language** |
| **Home** | `C:\localdev\devpulselabs` — the prior app at `devpulse labs` is untouched and remains the salvage source |
| **Verdict carried in** | Signals are plain Rust enums on tokio channels, typed end-to-end by rustc — zero codegen, zero mirrors. The boundary problem class does not exist here. |

## 1. Laws (carried forward, adapted)

1. **Plan before code.** Every pass gets a plan; reality contradicting the plan stops the pass.
2. **Test the real artifact.** The built exe is the truth. The app ships a `--screenshot <path>` harness: boot, paint, capture its own window to PNG, exit — so every pass proves itself visually without a human in the loop.
3. **Hyper-modular colocation.** One concern = one crate. A hub is a crate. The design system is a crate. The signal fabric is a crate. Nothing reaches into another crate's internals — public API only, enforced by the compiler.
4. **Registry over enumeration at the seams.** Hubs plug in: `Hub` trait + `HubSpec`, registered in one place (`app/main.rs`), rendered by a shell that does not know their names. Unknown → degrade, never crash.
5. **No bare strings for identity.** `HubId`/`SubId` are newtypes declared once per hub as consts. The compiler owns identity.
6. **Tokens are authored once.** ColdLight's JSON tokens remain the source; this repo's `coldlight` crate is their Rust output target (hand-seeded now, generated later — same Style-Dictionary thesis, second emitter).
7. **The paint vocabulary is the operator's:** crisp sprite-grade edges, solid fills with gradient tint, **black diffusion scatter** (dark falloff, never backdrop blur). Cheap on this renderer by design.

## 2. Workspace

```
devpulselabs/
  Cargo.toml            workspace + shared deps
  AGENTS.md             the contract
  docs/                 this plan, future ADRs
  crates/
    hub/                THE CONTRACT — HubId/SubId newtypes, SubRoute, HubSpec, Route, Hub trait
    coldlight/          design system: tones, theme, paint vocabulary (scatter_frame, gradient_tint)
    fabric/             typed signals: per-domain event enums over tokio broadcast/watch
    shell/              the frame: left rail (auto-extends active hub's subroutes), top bar,
                        status bar, central dispatch. Owns Route state, publishes RouteChanged.
    hubs/
      home/             Shows
      workbench/        Studio · Library · Tokens        (episode + slide building)
      wiredfront/       Overview · PixVix · ViziWiz · Gallery · Sessions   (capture + media editing)
      chat/             Sessions · Council               (claude · codex · gemini · grok · clyffy)
      library/          Component gallery
    app/                the binary: boot eframe, register hubs, wire fabric, screenshot harness
  engine/               (phase 2) ported from the old repo: capture, steps, index, ffmpeg — near-verbatim,
                        minus the tauri command macros they no longer need
```

Dependency direction (leaves left): `hub`, `coldlight` ← `fabric` ← `shell`, `hubs/*` ← `app`.
A hub crate depends on `hub` + `coldlight` + egui — never on `shell`, never on a sibling.

## 3. The shell behavior (the operator's spec, verbatim intent)

- Left rail lists every registered hub (icon + label, tone-accented).
- **Clicking a hub routes to it AND auto-extends the rail beneath it, showing that hub's
  subcategories.** Only the active hub is extended; switching hubs collapses the old, extends the new.
- Clicking a subcategory routes within the hub; the extension stays.
- Route = `(HubId, SubId)` held by the shell, broadcast on the fabric as `ShellEvent::RouteChanged`
  so any future consumer (chat context providers, telemetry, the engine) observes navigation without
  touching the shell.
- Top bar: current hub · subroute breadcrumb. Status bar: route id + fabric liveness, launcher slot reserved.

## 4. IA carried from the old app (its own nav-dock.config.ts, read today)

home/Shows · workbench(Studio, Library, Tokens) · library(Component gallery) ·
wiredfront(Overview, PixVix, ViziWiz, Gallery, Sessions) · chat(Sessions, Council — the five-provider
mesh from the fabric plan). Growth hubs noted, not scaffolded: EoS, News, Vibes, Dev.

## 5. Salvage map (nothing from the six months is discarded)

| From `devpulse labs` | Into here | How |
|---|---|---|
| Rust engine (~6.3k LOC: capture, steps, ffmpeg, artifact index) | `engine/` | Phase 2 port — logic is tauri-free already; command wrappers deleted |
| ColdLight token JSON | `coldlight` generator input | SD emits Rust constants instead of CSS |
| ADR-010 (editor), fabric plan, all research | `docs/` | Carried as-is; ADR-009 (ffmpeg subprocess) remains law |
| Content, captures, guides, replays | data dir | Untouched — files were always the truth |
| rrweb session replay | **retired** | Session capture pivots to the video recorder (the one browser-bound feature) |

## 6. Phases

- **P0 (this pass):** workspace + contract + coldlight seed + fabric exemplar + shell with working
  rail/extension/routing + five hub crates with themed placeholder surfaces + screenshot harness.
  Proof: compiles clean, unit tests green, `--screenshot` PNG shows the shell with the rail extended.
- **P1:** ColdLight generator (JSON → Rust), the crisp/tint/scatter vocabulary fleshed into a real
  widget set, fluid sizing as Rust fns.
- **P2:** engine port + the jobs substrate (ADR-010 D-7) on fabric channels.
- **P3+:** hubs fill in priority order the operator sets (editor per ADR-010, chat mesh per fabric plan §6).
