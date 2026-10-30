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

**Linux (Ubuntu/Debian)** additionally needs the system packages below for
EGL, GBM, Wayland, X11, PipeWire, GTK, ALSA and clang. This is the `APT_DEPS`
set from [`linux-runner-bootstrap.sh`](docs/runbook/linux-runner-bootstrap.sh).
Install just the packages on a development machine:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential cmake pkg-config ffmpeg libssl-dev \
  libegl1-mesa-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev \
  libxkbcommon-x11-dev libx11-dev libxcursor-dev libxrandr-dev \
  libxi-dev libxtst-dev libxdo-dev libudev-dev libevdev-dev \
  libasound2-dev libpipewire-0.3-dev libdbus-1-dev libgtk-3-dev \
  libpango1.0-dev libcairo2-dev libglib2.0-dev libv4l-dev \
  libgbm-dev libxcb1-dev clang libclang-dev llvm-dev curl git
```

The bootstrap script itself provisions and registers a self-hosted Actions
runner and requires a registration token; it is not workstation setup.

**Windows** needs MSVC Build Tools, the Windows SDK and standalone LLVM
(`libclang.dll` for bindgen; the estate bootstrap pins LLVM 18.1.8). Set
`LIBCLANG_PATH` to LLVM's `bin` directory if it is not discovered automatically.

### Build and run

```bash
# the application, plus the Lottie render helper that must ship BESIDE it
cargo build --release -p devpulselabs -p dpl-lottie-render

./target/release/devpulselabs          # Linux
.\target\release\devpulselabs.exe      # Windows
```

### The gate

One command decides whether the tree is good. Both GitHub-hosted CI lanes
run it, on Windows and Linux; a local run proves the current machine's OS:

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
| [`SECURITY.md`](SECURITY.md) | How to report a suspected vulnerability privately |

---

## Contributing

This is a personal project in pre-release and is not accepting outside
contributions. The repository is public for reading; publication does not
change its license or contribution policy.

If you are working in it:

1. Plan before code — every pass gets a document.
2. Branch, open a PR, and **merge only on double-green** — both
   `green-linux` and `green-windows` COMPLETED SUCCESS. A running check is a
   red check.
3. Never bypass the gate with `--no-verify`.

---

## License

**Closed — all rights reserved.** See [`LICENSE`](LICENSE).

The source is public for reading under the terms in `LICENSE`; this is not
an open-source license. A license for use, modification or redistribution
will be chosen and published when the project is ready for one.

[ADR-009]: docs/design/
