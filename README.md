# DevPULSE Labs

**Raw material becomes a finished show, and the app teaches you to make it.**

A desktop studio for making instructional and narrative content end to end —
cut the raw media, assemble it into slides and episodes, and have an AI
council help you plan it. Pure Rust, native, one binary.

> **Status: pre-release.** The `main` branch builds and passes its gate on
> Windows and Linux, but this is an application under active construction, not
> a released product. Interfaces change without notice. There is no packaged
> installer yet — see [Building](#building).

---

## What it is

Five pieces that compose, rather than five separate tools:

| | |
|---|---|
| **WiredFRONT** | The raw-media editor — capture, cut, composite, grade. ffmpeg drives the render as a subprocess; the timeline, preview, and effects are native. |
| **Workbench** | Where cut material becomes a *show* — slides, cards, episodes, decks, guides. |
| **Chat** | Route-aware, and present throughout. It knows which surface you are on and can act on it through typed intents, never free-form control. |
| **The Council** | Your own AI subscriptions — Codex, Claude, Gemini, Grok — reached through their official client surfaces. DevPULSE never holds an API key. |
| **The Coach** | Point at any installed application and get live, on-screen instruction. Built on accessibility trees, not screenshots. |

The pipeline is not a metaphor — it is typed:

```rust
// crates/workflows/src/lib.rs
pub enum Surface { Wiredfront, Workbench }
```

Two surfaces, one direction of travel. Generated and captured media is **raw
media** (WiredFRONT); authored design ingredients are **ColdLight ingredients**
(Workbench). Nothing crosses without passing through an explicit import seam.

---

## Building

### Requirements

- **Rust 1.97.1** — pinned by `rust-toolchain.toml`, so `rustup` selects it
  automatically. Do not override it.
- **ffmpeg** on `PATH` — rendering is a subprocess by design ([ADR-009]).
- A GPU that can run `wgpu` (the UI is `egui` + `eframe`).

**Linux** additionally needs a dev package set — EGL, GBM, Wayland, X11,
PipeWire, GTK, ALSA, clang. The canonical, tested list lives in
[`docs/runbook/linux-runner-bootstrap.sh`](docs/runbook/linux-runner-bootstrap.sh)
(`APT_DEPS`), which is idempotent and safe to run on a workstation:

```bash
sudo bash docs/runbook/linux-runner-bootstrap.sh
```

**Windows** needs MSVC Build Tools and the Windows SDK.

### Build and run

```bash
# the application, plus the Lottie render helper that must ship BESIDE it
cargo build --release -p devpulselabs -p dpl-lottie-render

./target/release/devpulselabs          # Linux
.\target\release\devpulselabs.exe      # Windows
```

### The gate

One command decides whether the tree is good. It is the same check both CI
lanes run, so local green and CI green mean the same thing:

```bash
cargo green
```

Seven phases, stopping at the first red: `fmt --check` → docs status → spawn
audit → **structure audit** → `clippy -D warnings` → workspace tests → release
build with an exe-regression assert.

It is wired as a pre-push hook. Fresh clones enable it once:

```bash
git config core.hooksPath .githooks
```

---

## Layout

68 crates in one workspace, layered so dependencies point **down** and never
sideways. The layering is not a convention — it is enforced by the `structure`
phase of the gate, which fails on a new edge that the architecture document
does not declare.

```
DOCUMENT   dpl-doc · dpl-comp · dpl-workflows      pure serde data, versioned
ENGINE     dpl-engine-{capture,media,steps,ae,browser} · dpl-index ·
           dpl-estate · dpl-ledger · dpl-svg …     IO + subprocess leaves, egui-free
SUPPORT    dpl-coldlight · dpl-vector · dpl-video · dpl-backdrop
SURFACE    dpl-slides · dpl-editor · dpl-deck · dpl-composer · dpl-viziwiz ·
           dpl-chat-window/-windows/-renderers     egui; paint via ColdLight only
HUB        crates/hubs/*                           never depends on another hub
SHELL→APP  dpl-shell → devpulselabs                the only crate that wires all
side rails dpl-fabric (signals) · dpl-seam (agent HTTP) · egui-pilot-ai-* · xtask
```

Full law, ceilings, and the standing exceptions:
[`docs/design/architecture.md`](docs/design/architecture.md).

---

## Conventions worth knowing before reading the code

- **Naming.** A crate is `dpl-` + its folder; binaries are kebab-case; a
  `src/bin/*.rs` file is snake_case with its binary name *declared*. Ratified
  exceptions are listed in architecture.md §4 and enforced by the gate.
- **Census law.** Wherever a *list* of things exists — templates, workflows,
  stages, harness flags — one registry function enumerates it and a test fails
  when reality and the list drift. Nothing enumerates by hand.
- **Document law.** Persisted JSON carries a schema version, `load` refuses
  anything newer, and `save` is crash-safe (tmp → step-aside → rename).
- **ADR-009.** External capability is a subprocess behind a seam — ffmpeg,
  pdftoppm, aerender. `-sys` FFI links are permitted in exactly one leaf.
- **Signals.** Mutations publish typed facts on one fabric. A surface that
  publishes a domain never subscribes to it.

---

## Documentation

| | |
|---|---|
| [`docs/status.md`](docs/status.md) | Where we are: now, known broken, next. Read first. |
| [`docs/requirements.md`](docs/requirements.md) | The operator's requirements, verbatim: what gets built |
| [`docs/README.md`](docs/README.md) | The map: the one doc that is the truth for each subject |
| [`docs/design/architecture.md`](docs/design/architecture.md) | The layer law, gate-enforced |
| `CLAUDE.md` / `AGENTS.md` | The operating contract for AI agents working in this repo |

---

## Contributing

This is a personal project in pre-release and is not accepting outside
contributions. The repository is private (since 2026-08-07).

If you are working in it:

1. Plan before code — every pass gets a document.
2. Branch, open a PR, and **merge only on double-green** — both
   `green-linux` and `green-windows` COMPLETED SUCCESS. A running check is a
   red check.
3. Never bypass the gate with `--no-verify`.

---

## License

**Closed — all rights reserved.** See [`LICENSE`](LICENSE).

The repository is private. No right is granted: no use, copying,
modification, or redistribution. A license will be chosen and published when
the project is ready for one.

[ADR-009]: docs/design/
