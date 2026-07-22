#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "EguiPilotAI Linux preflight: $*" >&2
  exit 1
}

expected_rust="$(
  awk -F '"' '/^channel = / { print $2; exit }' rust-toolchain.toml
)"
actual_rust="$(rustc --version | awk '{ print $2 }')"

test -n "$expected_rust" || fail "rust-toolchain.toml has no pinned channel"
test "$actual_rust" = "$expected_rust" ||
  fail "Rust drift: expected $expected_rust, found $actual_rust"

cargo fmt --version >/dev/null || fail "rustfmt component is unavailable"
cargo clippy --version >/dev/null || fail "Clippy component is unavailable"
rustup target list --installed | grep -Fxq wasm32-unknown-unknown ||
  fail "wasm32-unknown-unknown target is unavailable"

# Keep this list aligned with Linux-only workspace crates. Checking it before
# Cargo avoids discovering a missing native SDK late in a clean CI build.
native_modules=(
  alsa
  egl
  fontconfig
  libpipewire-0.3
  libudev
  wayland-client
  x11-xcb
  xcursor
  xi
  xinerama
  xkbcommon
  xrandr
)

for module in "${native_modules[@]}"; do
  pkg-config --exists "$module" ||
    fail "pkg-config module '$module' is unavailable"
done

test -n "${VK_DRIVER_FILES:-}" || fail "VK_DRIVER_FILES is unset"
test -r "$VK_DRIVER_FILES" ||
  fail "Vulkan software-renderer manifest is unreadable: $VK_DRIVER_FILES"

echo "EguiPilotAI Linux preflight passed (Rust $actual_rust)."
