#!/usr/bin/env bash
# linux-runner-bootstrap.sh — blank Debian/Ubuntu x64|arm64 guest ->
# registered dpl-linux runner. Idempotent; every step verifies itself;
# exits nonzero naming the first unsatisfied requirement.
#
# Usage (fresh registration token, minted seconds before):
#   t=$(gh api -X POST repos/EonsofStupid/devpulselabs/actions/runners/registration-token --jq .token)
#   sudo ./linux-runner-bootstrap.sh --token "$t" [--name <runner-name>] [--labels dpl-linux]
#   sudo ./linux-runner-bootstrap.sh --verify-only     # heartbeat / post-restart audit
#
# The apt set is the PROVEN list (server1 2026-07-27 + LXC 121
# 2026-07-28 — khronos-egl and friends die without it). Rust pin
# matches rust-toolchain.toml.

set -euo pipefail

REPO_URL="https://github.com/EonsofStupid/devpulselabs"
TOKEN=""
NAME="$(hostname)"
LABELS="dpl-linux"
RUNNER_HOME="/opt/actions-runner"
RUNNER_USER="runner"
RUST_VER="1.92.0"
VERIFY_ONLY=0

while [ $# -gt 0 ]; do
  case "$1" in
    --token) TOKEN="$2"; shift 2 ;;
    --name) NAME="$2"; shift 2 ;;
    --labels) LABELS="$2"; shift 2 ;;
    --repo-url) REPO_URL="$2"; shift 2 ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

APT_DEPS=(build-essential cmake pkg-config ffmpeg libssl-dev
  libegl1-mesa-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev
  libxkbcommon-x11-dev libx11-dev libxcursor-dev libxrandr-dev
  libxi-dev libxtst-dev libxdo-dev libudev-dev libevdev-dev
  libasound2-dev libpipewire-0.3-dev libdbus-1-dev libgtk-3-dev
  libpango1.0-dev libcairo2-dev libglib2.0-dev libv4l-dev
  libgbm-dev libxcb1-dev
  clang libclang-dev llvm-dev curl git)

step() { echo "== $*"; }
skip() { echo "   SKIP (already satisfied): $*"; }

have_deps()  { dpkg -s "${APT_DEPS[@]}" >/dev/null 2>&1; }
have_egl()   { pkg-config --exists egl; }
have_cargo() { sudo -u "$RUNNER_USER" bash -lc 'command -v cargo' >/dev/null 2>&1; }
# Location-agnostic: the systemd unit is the truth (installs from other
# lanes/homes count); svc.sh is the fallback for non-systemd guests.
svc_active() {
  systemctl list-units --type=service --state=running 2>/dev/null \
    | grep -q "actions\.runner\..*${NAME}" && return 0
  cd "$RUNNER_HOME" 2>/dev/null && ./svc.sh status 2>/dev/null | grep -q 'active (running)'
}

if [ "$VERIFY_ONLY" = 1 ]; then
  step "verify-only audit"
  fails=0
  have_deps  || { echo "   FAIL: apt deps incomplete"; fails=$((fails+1)); }
  have_egl   || { echo "   FAIL: pkg-config egl missing"; fails=$((fails+1)); }
  id "$RUNNER_USER" >/dev/null 2>&1 || { echo "   FAIL: user $RUNNER_USER missing"; fails=$((fails+1)); }
  have_cargo || { echo "   FAIL: cargo missing for $RUNNER_USER"; fails=$((fails+1)); }
  svc_active || { echo "   FAIL: runner service not active"; fails=$((fails+1)); }
  [ "$fails" = 0 ] && { echo "VERIFY: all green"; exit 0; }
  echo "VERIFY: $fails problem(s)"; exit 1
fi

[ -n "$TOKEN" ] || { echo "--token is required (or --verify-only)" >&2; exit 2; }
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 2; }

step "apt deps (${#APT_DEPS[@]} packages)"
if have_deps; then skip "all present"; else
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y -qq "${APT_DEPS[@]}"
fi
have_egl || { echo "pkg-config egl STILL missing after install" >&2; exit 1; }

step "runner user"
if id "$RUNNER_USER" >/dev/null 2>&1; then skip "user exists"; else
  useradd -m -s /bin/bash "$RUNNER_USER"
fi

step "rust $RUST_VER for $RUNNER_USER"
if have_cargo; then skip "cargo present"; else
  sudo -u "$RUNNER_USER" bash -lc \
    "curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUST_VER --profile default"
  have_cargo || { echo "cargo STILL missing after rustup" >&2; exit 1; }
fi

step "actions runner -> $RUNNER_HOME"
if [ -x "$RUNNER_HOME/config.sh" ]; then skip "runner unpacked"; else
  arch="$(dpkg --print-architecture)"; case "$arch" in
    amd64) rarch=x64 ;; arm64) rarch=arm64 ;;
    *) echo "unsupported arch $arch" >&2; exit 1 ;;
  esac
  ver="$(curl -fsSL https://api.github.com/repos/actions/runner/releases/latest | grep -oP '"tag_name":\s*"v\K[^"]+')"
  mkdir -p "$RUNNER_HOME"
  curl -fsSL -o /tmp/runner.tgz \
    "https://github.com/actions/runner/releases/download/v${ver}/actions-runner-linux-${rarch}-${ver}.tar.gz"
  tar -xzf /tmp/runner.tgz -C "$RUNNER_HOME"; rm -f /tmp/runner.tgz
  chown -R "$RUNNER_USER:$RUNNER_USER" "$RUNNER_HOME"
fi

step "register + systemd service ($NAME, labels: $LABELS)"
cd "$RUNNER_HOME"
sudo -u "$RUNNER_USER" ./config.sh --unattended --replace \
  --url "$REPO_URL" --token "$TOKEN" --name "$NAME" --labels "$LABELS"
./svc.sh install "$RUNNER_USER" 2>/dev/null || true   # exists on re-run
./svc.sh start

step "final verify"
svc_active || { echo "runner service not active after start" >&2; exit 1; }
echo "BOOTSTRAP GREEN: $NAME registered and running."
echo "Confirm on the repo: gh api repos/<owner>/<repo>/actions/runners"
