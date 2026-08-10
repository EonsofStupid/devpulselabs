# sccache — the shared build cache (org-wide)

Rust recompiles the same crate bytes over and over — across `cargo clean`,
across branches, across machines, across projects. sccache caches the
*compiler output* per crate and replays it. Measured on server1, 2026-08-10:

| egui + deps | wall time | hit rate |
|---|---|---|
| cold (empty cache) | **1m 07s** | 0% |
| warm (same crates) | **1.4s** | **100%** |

~45× on that slice. The win is every build after the first, and every machine
that reuses what another already compiled.

## What it does NOT do (so nobody is surprised)

- **Does not mix with incremental compilation.** `CARGO_INCREMENTAL=0` is
  required; sccache replaces incremental's job for clean/CI builds and costs a
  little on tiny iterative edits.
- **Does not cache everything.** Link steps and some `build.rs` outputs miss.
  Real speedup, never 100% of wall time.
- **Keyed by exact compiler + flags + TARGET TRIPLE.** So
  `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` are separate
  namespaces — the Windows laptop shares with the Windows CI runner, server1
  shares with the Linux runner, and Windows↔Linux never share. Correct, not a
  bug.
- **A toolchain bump invalidates the cache.** The first build on a new Rust is
  cold again. Also correct.

## Flushing — the part that matters for Rust

sccache does **LRU eviction at a hard bound**. Set `SCCACHE_CACHE_SIZE` and it
never grows past it — least-recently-used artifacts are dropped
automatically. No cron, no manual cleaning. This is the answer to "Rust piles
up": the *cache* is bounded; it is `target/` directories that grow unbounded,
and a warm sccache is what lets you delete `target/` freely because rebuilding
is cheap.

## Tier 1 — local bounded cache per machine (turnkey, do this first)

Immediate win, self-flushing, zero infrastructure. Each machine caches to a
bounded local dir.

**server1** (sccache already at `/usr/bin/sccache`) — add to
`~/.cargo/config.toml`:

```toml
[build]
rustc-wrapper = "/usr/bin/sccache"
incremental = false
```

and to the shell profile (so `cargo green` over SSH sees it):

```sh
export SCCACHE_DIR=/root/.cache/sccache
export SCCACHE_CACHE_SIZE=20G
```

**Windows laptop** — install (`winget install Mozilla.sccache` or
`cargo install sccache`), then put the cache on **D:** where there is
headroom (C: runs tight):

```toml
# %USERPROFILE%\.cargo\config.toml
[build]
rustc-wrapper = "sccache"
incremental = false
```
```
setx SCCACHE_DIR "D:\sccache"
setx SCCACHE_CACHE_SIZE "50G"
```

Verify: `sccache --show-stats` after two builds of the same crate — the second
must show a non-zero hit rate.

## Tier 2 — shared network cache (cross-machine, when tier 1 has proven out)

So server1's cache warms the laptop and the CI runners reuse both. sccache
speaks S3, so a small **MinIO** on a disk with headroom is the backend.

**Where it CANNOT go:** server1's root disk — 18 GB free of 98 GB. A shared
cache needs headroom the estate root does not have.

**Where it can:** the laptop's D: (721 GB free), the drive being added, or an
expanded estate volume. Decision pending — see §Open.

Backend wiring, once a host exists:

```sh
export SCCACHE_BUCKET=rust-cache
export SCCACHE_ENDPOINT=http://<minio-host>:9000
export SCCACHE_S3_USE_SSL=off
export AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=...
export RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0
```

Eviction on S3/MinIO is a **bucket lifecycle policy** (expire objects older
than N days), not `SCCACHE_CACHE_SIZE`.

**Security note (matters here):** a shared cache is a supply-chain surface —
anyone who can write to it can poison a build. Keep the bucket on the private
estate, never expose it publicly, and scope the write credential to CI +
trusted operators only. This is the same reasoning that keeps the self-hosted
runners off a public repo.

## Org-wide rollout

The config above is project-agnostic — it lives in each machine's
`~/.cargo/config.toml`, so **every Rust project the machine builds benefits**,
not just this one. Nothing per-repo is required. A new project is fast on
first checkout if its crates are already in a shared tier-2 cache.

## Open decisions

1. **Tier-2 backend host** — D: on the laptop, the new drive, or expand
   server1's LVM. Needs headroom the estate root lacks.
2. **CI wiring** — the estate runners can point `RUSTC_WRAPPER` at the same
   backend; do it only after tier 1 is proven and the backend exists.
3. **Whether tier 2 is worth it at all** — tier 1 alone may be enough. Measure
   the tier-1 delta on a real full build before standing up a service.
