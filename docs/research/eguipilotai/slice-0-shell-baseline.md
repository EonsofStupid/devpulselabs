# Slice 0 DevPULSE shell baseline

Recorded 2026-07-22 before expanding UI instrumentation or Test Lab behavior.
This is a reproducible **provisional launch baseline**, not the later p95
per-frame or 10,000-stable-frame acceptance gate.

## Method

- Build: `cargo build -p devpulselabs --features eguipilotai-devtools`
- Target: the real native `eframe/wgpu` DevPULSE executable
- Route: `workbench/test-lab`
- Exit condition: native screenshot harness after frame 120
- EguiPilotAI attach variables: absent, so the devtools endpoint remained
  inactive
- Samples: three fresh target processes
- Measurements: wall time, process CPU time, peak working set, and emitted PNG
  size, polled by the parent process every 10 ms
- Source base: `8096862992346ec6e26c3332979be8cfde650525` plus the uncommitted Slice 0/1
  protocol-only pass; no shell instrumentation or UI expansion was present

## Environment

- Windows 11 Home `10.0.26200`
- AMD Ryzen 7 5800H, 16 logical processors
- AMD Radeon Graphics driver `31.0.21914.8004`
- NVIDIA GeForce RTX 3050 Ti Laptop GPU driver `32.0.15.9227`
- Rust `1.92.0 (ded5c06cf 2025-12-08)`

## Observations

| Run | Wall ms | CPU ms | Peak working set bytes | PNG bytes |
|---:|---:|---:|---:|---:|
| 1 | 3,628 | 1,984.38 | 393,146,368 | 1,638,782 |
| 2 | 2,433 | 1,890.62 | 393,170,944 | 1,631,576 |
| 3 | 2,287 | 1,921.88 | 390,877,184 | 1,602,181 |

Median wall time was 2,433 ms, median CPU time was 1,921.88 ms, and the
maximum observed working set was 393,170,944 bytes. PNG variation is expected
under natural ambient motion and is evidence that these captures are not a
suppressed visual-determinism gate.

## Use and limits

This record is the pre-expansion comparison point for gross regressions in the
same native shell path. Slice 7 must replace it with instrumented per-frame p95
CPU overhead and post-warm-up 10,000-frame memory-growth measurements under
pinned software and hardware renderer identities. Those later numbers must not
be inferred from this launch sample.
