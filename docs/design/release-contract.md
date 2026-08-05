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

### 5.3 The fix that already works — verified in-session

**A staged depot is PORTABLE.** It carries its own `manifest.json` with
SHA-256 for every payload, and the resolver re-verifies on open. I copied a
staged `dptools-deps/` from one checkout to another and the app accepted it:

```
INFO verified embedded ffmpeg resolved … version="8.1.2" sha256="ad8f211b…"
INFO verified embedded pdftoppm resolved … version="26.02.0" sha256="575a2f66…"
```

**So the nightly test procedure is:**

```
1. unzip the nightly
2. copy an already-staged  dptools-deps/  folder beside devpulselabs.exe
3. run it — the resolver re-hashes and either trusts it or refuses honestly
```

No cargo, no checkout, no re-download. Stage once per machine, reuse for every
nightly. **This was undocumented anywhere** — it is the single most useful
sentence in this file.

### 5.4 Recommended, not yet built

Two options to close 5.2 properly, neither taken without a ruling:

- **`devpulselabs --stage-tools <sources.toml>`** — put staging in the app
  itself so a zip-only tester never needs cargo. Small; reuses
  `dpl-tools-deps` wholesale.
- **A `--doctor` flag** that prints exactly which payloads are missing and
  where it looked. Today the operator gets two WARN lines in a log they may
  never see, and a UI that silently cannot render.

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
