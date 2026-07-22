#!/usr/bin/env bash
set -euo pipefail

export WGPU_BACKEND=vulkan
export LIBGL_ALWAYS_SOFTWARE=true
export VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json
export EGUIPILOTAI_ADAPTER_POLICY=software-required
export TZ=UTC
export LANG=C.UTF-8

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check \
  -p egui-pilot-ai-protocol \
  -p egui-pilot-ai-transport \
  -p egui-pilot-ai \
  -p egui-pilot-ai-runtime \
  --target wasm32-unknown-unknown
cargo build -p devpulselabs --features eguipilotai-devtools
cargo build -p egui-pilot-ai-cli

# The exact profile must prove it selected Lavapipe/CPU, then produce the
# same content-addressed capture fifty times under suppressed motion.
for _ in $(seq 1 50); do
  xvfb-run -a target/debug/eguipilotai \
    test devpulse-cloud tests/eguipilotai/shell-navigation.toml test-lab
done

capture_hash_count="$({
  find target/eguipilotai/runs -type f -path '*/captures/*.png' -printf '%f\n'
} | sort -u | wc -l)"
test "$capture_hash_count" -eq 1

# Persist the exact distro, Mesa, Vulkan, locale, and font package versions
# into the job artifacts so a renderer key can be audited and rebuilt.
dpkg-query -W \
  mesa-vulkan-drivers libvulkan1 fontconfig fonts-dejavu-core \
  > target/eguipilotai/software-renderer-packages.txt
fc-list > target/eguipilotai/font-manifest.txt
