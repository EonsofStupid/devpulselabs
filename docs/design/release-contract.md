# THE RELEASE CONTRACT — what ships, what is embedded, what must be staged

**Status:** CONSOLIDATION, 2026-08-05. Operator: *"ensuring that this is
portable but truly pulling its assets so they will actually when I make a
nightly and test that release."*

This file exists because **no single document said what a release IS.** The
truth was spread across `.github/workflows/nightly.yml` comments,
`dptools-deps/README.md`, ADR-009, ADR-014, `CLAUDE.md`'s command list, and
two runtime lookup functions. Every claim below was verified in-session by
reading the code or by running the real exe.

---

## 1 · The one-line answer

> A release is **two executables in a folder**. Everything the app draws is
> compiled INTO the app. The only thing that is neither embedded nor shipped
> is the **private tool depot** — and without it every media operation fails.

---

## 2 · What is EMBEDDED in the exe (verified)

Nothing here can go missing, because it is not a file at runtime.

| asset | how | verified at |
|---|---|---|
| **Fonts** (13 vendored TTFs) | `include_bytes!(CARGO_MANIFEST_DIR/assets/fonts/…)` → `FontData::from_static` | `coldlight/src/lib.rs:749,774`; `build.rs:368` |
| **Icons** (57 baked Lucide) | build.rs tessellates → vertex/index consts | `build.rs:270` `generate_icons` |
| **Colour ladders, fluid, motion, moods, presets, traits, matrix** | build.rs → `*_gen.rs` consts | `coldlight/build.rs` (10 generators) |
| **Keyframe tracks** | authored `tokens/motion.json` → const `Track` | `build.rs` `generate_motion` |

**Consequence:** a nightly zip cannot have a missing font or a missing icon.
Those failure modes do not exist by construction.

## 3 · What ships BESIDE the exe

| file | why | found at runtime by |
|---|---|---|
| `devpulselabs[.exe]` | the app | — |
| `dpl-lottie-render[.exe]` | ADR-009 — the façade SPAWNS it; an app-only zip is a broken nightly for every Lottie surface | `lottie/src/lib.rs:30` `helper_path()`: `DPL_LOTTIE_RENDER` env → **beside `current_exe()`** → bare name on PATH |

`nightly.yml` builds and zips both on all three platforms
(`cargo build --release -p devpulselabs -p dpl-lottie-render`). **This is
correct and needs no change.**

## 4 · What is CREATED at first run

The three-ring root (ADR-014), beside the exe, verified in a real launch:

```
content=<exe dir>/content   assets=<exe dir>/assets   userdata=<exe dir>/.userdata
```

Overridable with `--content-root <path>` or the `CONTENT_ROOT` env. **Nothing
needs shipping** — the app mints them.

## 5 · What must be STAGED, and the gap that bites a nightly tester

`dptools-deps/` — ffmpeg, pdftoppm, chromium, codex, grok. **Deliberately not
in the zip** (`nightly.yml:15`: *"No /content, no invented dptools-deps"*),
because they are private/licensed binaries.

Resolution is **executable-relative and SHA-256 verified** — the app never
reaches back into a dev checkout (`tools-deps/src/lib.rs:1-6`). That design is
right and portable.

### 5.1 What actually happens without it — measured, not guessed

Running the real exe with no depot:

```
WARN verified embedded ffmpeg is unavailable … dptools-deps\manifest.json: required file was not found
WARN verified embedded pdftoppm is unavailable …
```

→ **previews, probes, renders, and PDF slides all fail.** The app still
starts and the UI still paints (fonts and icons are embedded, §2), so it looks
healthy while every media path is dead. That is the exact state a nightly
tester lands in.

### 5.2 The gap

`NIGHTLY.md` in the zip says *"Stage private tools with `cargo xtask
stage-tool-deps <release-root>`."* **A zip-only tester cannot do that**, because
`stage-tool-deps` requires:

1. a **repo checkout** and `cargo`;
2. `dptools-deps/sources.local.toml`, which is **gitignored** (`.gitignore:12`)
   and holds machine-local absolute paths to private binaries;
3. those private binaries present on that machine.

So "download the nightly and test it" yields a half-dead app unless the tester
is also a dev on a staged machine.

### 5.3 RULED 2026-08-05 — the deps SHIP WITH THE RELEASE

**Operator:** *"no remote … that sentence unzip copy a staged dptools deps is
stupid we are not doing that."*

Both rejected, and rightly:

- **no remote fetch** — the app never reaches out for its own tools;
- **no manual copy** — "unzip, then go find a staged depot" is a workaround,
  not a design. A release that needs a second artifact to function is not a
  release.

**Therefore the nightly BUNDLES the depot.** Unzip and it works. That is the
whole requirement, and it is the right one for an artifact whose purpose is
testing.

### 5.4 The one constraint on bundling — the repo is PUBLIC

`nightly.yml:5-8` records it: the estate runners were deregistered *"when the
repo went public"*, and the Release is a public GitHub Release. The depot's
five payloads do not share a distribution story:

| family | payloads | bundle into a PUBLIC release? |
|---|---|---|
| **Media** | `ffmpeg`, `pdftoppm` | the tools testing actually needs — previews, probes, renders, PDF slides |
| **Browser** | `chromium` | large; only the browser-driver surfaces need it |
| **Provider** | `codex`, `grok` | **the operator's private binaries** — publishing them is a redistribution decision, not a packaging one |

**Media is what makes a test build work.** Every WARN in §5.1 is ffmpeg or
pdftoppm; nothing in the editor's preview/render path touches a provider.

### 5.5 Recommended shape

- **Bundle Media** in the nightly zip → unzip and previews/renders work.
- **Leave Provider out** until the operator rules on redistribution; the app
  already degrades honestly when a payload is absent (§5.1), so their absence
  costs nothing the editor cares about.
- **Browser** is the operator's call on size.
- `--doctor` still earns its place: it turns "two WARN lines in a log" into a
  plain statement of what is present and what is not.

## 6 · Where the truth WAS scattered

Consolidated here; these remain the implementations, not the explanations.

| what | lived in |
|---|---|
| nightly ships 2 exes, no depot | `.github/workflows/nightly.yml:12-16` (comment) |
| how to stage a depot | `dptools-deps/README.md` |
| depot is verified + app-relative | `crates/tools-deps/src/lib.rs:1-6` |
| helper must sit beside the exe | ADR-009 + `lottie/src/lib.rs:30` |
| three-ring root beside the exe | ADR-014 |
| fonts/icons embedded | `coldlight/build.rs` — stated nowhere in prose |
| **what a release IS** | **nowhere — this file** |

## 7 · Verification a nightly is genuinely testable

Run against an unzipped nightly, in order:

1. `devpulselabs --route wiredfront/editor --screenshot a.png --window 1600x900`
   → paints with fonts and icons **before any depot exists** (proves §2).
2. Confirm `dpl-lottie-render` sits beside the exe (proves §3).
3. Launch once → `content/`, `assets/`, `.userdata/` appear (proves §4).
4. **Without** a depot: the two WARN lines appear and media fails (§5.1).
5. Copy a staged `dptools-deps/` beside the exe → relaunch → both
   `verified … resolved` INFO lines appear (§5.3).
6. `--preview-drill <rel>` on a real clip → frames decode.

Steps 1–5 were performed in-session; step 6 was performed on a staged lane
build and the proof shot is `docs/proof/preview-player-1600x900.png`.

## 8 · Open

- **§5.4** — does staging move into the app (`--stage-tools`), and does a
  `--doctor` land? Both are small; neither is mine to decide.
- The nightly zip could carry §5.3's copy-the-depot procedure in `NIGHTLY.md`
  instead of pointing at a cargo command a tester cannot run. One-line change
  to `nightly.yml`, deferred to the operator's word.
