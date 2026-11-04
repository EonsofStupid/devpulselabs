# Training: Python vs Rust, measured on this machine

Run 2026-11-04 for R294, on an RTX 5090 Laptop GPU (24 GB VRAM), 63 GB RAM and a
24-core CPU. Everything ran in scratch folders, on a throwaway library. The
scripts and raw results are in [`spike/`](spike/). Published work is in
[`demonstrations-to-skills.md`](demonstrations-to-skills.md) and
[`clips-and-capture.md`](clips-and-capture.md).

## The answer

**Train in Python, run as a subprocess. Keep everything else in Rust.**

- Python has the whole fine-tuning stack ready-made: `transformers`, `peft`,
  `trl`, `unsloth`, `accelerate`, `bitsandbytes`. I found no Rust equivalent.
- Rust's Burn does real training of models you define. It learned as well as
  PyTorch on the test below, about 12× slower. It can't fine-tune a downloaded
  vision model without writing that model's architecture and tuning layer in
  Rust first (inference; not attempted).
- Candle's README says its "core goal is to _make serverless inference
  possible_". It suits running a model, not this training.
- This matches how the app already treats ffmpeg (ADR-009): a tool beside the
  exe, spawned as a subprocess, never a crate. Burn stays worth revisiting for
  small models the app defines itself.

## Head-to-head: same model, same bytes

A small convolutional network, 1,000 steps, batch 64, Adam at 1e-3, three seeds,
on identical data files and batch order. Initial weights differ by framework.

| | PyTorch 2.14.1 on CUDA 13 | Burn 0.21 on Vulkan |
|---|---|---|
| Steps per second | 66.8, 69.3, 70.9 | 5.8, 5.8, 5.8 |
| Final error (MSE) | 0.0049, 0.0050, 0.0051 | 0.0058, 0.0050, 0.0049 |
| On disk | 4.0 GB environment | ~31 MB binary (the CUDA build; Vulkan's not measured) |

What this does and doesn't show:

- **Burn wasn't on CUDA.** Its CUDA backend needs `cuda_runtime.h` from NVIDIA's
  full toolkit (`CUDA_PATH`), a system install I didn't make. The DLL it also
  needs (`nvrtc`) ships inside the PyTorch wheel. So this is Vulkan against
  CUDA, not Burn at its best.
- **Burn's optimizers are opt-in.** Without `fusion` and `autotune` it ran at 2
  steps per second. Anyone testing Burn should turn them on.
- **One small model.** Other models could show a different gap.
- **The global Python's torch here is CPU-only** (`2.14.1+cpu`), so it could not
  have used this GPU. A separate environment has the GPU build; the global
  Python was left alone.

## Does training actually help? Real frames, labelled by the app

**The dataset is the "machine code" idea working** (R294). EguiPilotAI's semantic
tree gave every control's name, role and exact rectangle, in the same session as
the frame. No AI and no hand labelling.

- 96 frames, 16 screens, 6 window sizes, no failures, 1,251 labelled controls
  with 59 distinct names, about 17 s a frame.
- Labels were checked by drawing them on a frame; they land on the controls.

**The model:** Qwen3.5-2B (Apache-2.0, 4.5 GB), LoRA rank 16. **The question:**
"Where is the control named X?" A hit is an answer inside the real rectangle. The
untouched model and the tuned one answered the same questions, none of them in
training.

| Test | Questions | Before | After | Median miss |
|---|---|---|---|---|
| Known screens, unseen window size (first run) | 150 | 21.3% | 94.7% | 140 → 3.9 px |
| Unseen screens (first run; mostly shared menus) | 126 | 23.8% | 100% | 330 → 0.5 px |
| **Controls never seen in training, at any size** | 87 | **37.9%** | **90.8%** | 68 → 4.6 px |
| Known controls, unseen window size (same run) | 150 | 20.7% | 97.3% | 141 → 4.2 px |
| **Pages in a foreign look, unseen commands** | 288 | **58.3%** | **100%** | 33 → 4.7 px |

- **First run:** 3,000 steps, 99 minutes, 937 samples, 7.7 GB peak.
- **Never-seen run:** 1,200 steps, 47 minutes, 970 samples. It withheld 15 of the
  49 controls that aren't on every screen. The foreign pages used this model.

### Read these numbers carefully

- **"Unseen screens" at 100% is mostly memory.** The shared menus sit in the same
  place on every screen. The first run's only truly unseen control was
  `+ New lesson`, 6 questions, already 100% before training.
- **The foreign test is easy.** 24 generated pages of large text buttons on plain
  backgrounds, which is why the untouched model already scored 58%. It is **not**
  evidence about Excel, Chrome, or anything with dense or icon-only controls.
- **Never-seen controls were never asked about, but their pixels were in
  training frames.** The 87 questions cover 15 controls, not 87 independent cases.
- **One run, one seed each.** No variance is measured.
- **An empty library, and limited tags.** Many screens only tag their 10
  shared-menu controls, so the frames are simpler than real use.
- **For this app, AI isn't needed to find a control at all.** The semantic tree
  already answers exactly. The value is in apps with no such tree. That is the
  case this spike could not test.

### Known defects in the spike

- **The tuned model repeats its answer** instead of stopping, so replies take
  about 3× longer. Scoring reads the first line only. Training needs fixing.
- **Training ran on slow reference kernels,** about 2 s a step, because
  `flash-linear-attention` and `causal_conv1d` weren't installed.
- **`unsloth` wasn't used.** pip resolved an old version in the new environment.

## What this does not settle

1. **Real third-party apps.** The first job of the real recorder: label frames of
   real apps from Windows UI Automation, then score on those.
2. **Whether a tuned model makes an agent finish more tasks,** which is what
   counts. Finding a control is one step of that.
3. **Generated clips as training data.** Nothing here used them. The research says
   to treat them as test fixtures first, and to measure lift against held-out real
   recordings before training on them.
