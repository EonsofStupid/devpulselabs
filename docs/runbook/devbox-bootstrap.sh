#!/usr/bin/env bash
# devbox-bootstrap.sh — blank Debian/Ubuntu x64|arm64 guest -> a browser-
# reachable Claude Code devbox. Idempotent; every step verifies itself;
# exits nonzero naming the first unsatisfied requirement.
#
# This is the GOLDEN-IMAGE SOURCE. Packer runs this once against a Proxmox
# template and once against an AWS AMI so the estate and the fleet cannot
# drift apart. Sibling of linux-runner-bootstrap.sh (same contract, same
# verify discipline) — that one builds a CI runner, this one builds a seat.
#
# Usage:
#   sudo ./devbox-bootstrap.sh [--user devbox] [--repo-url <url>] [--port 7681]
#   sudo ./devbox-bootstrap.sh --verify-only     # post-boot / post-restart audit
#
# NOT baked by this script, deliberately:
#   - Claude Code credentials. The seat holder authenticates on first boot
#     (`claude` prints a device URL). A shared credential in an image is a
#     licensing problem AND a blast-radius problem.
#   - Any cloud role/secret. The instance profile supplies those at runtime,
#     scoped to one seat's volume and bucket.
#
# ttyd binds to LOOPBACK ONLY. Terminating TLS and putting auth in front of
# it is the caller's job (your panel, or a reverse proxy). A world-reachable
# ttyd is a root shell with no password.

set -euo pipefail

REPO_URL="https://github.com/EonsofStupid/devpulselabs"
DEV_USER="devbox"
RUST_VER="1.92.0"      # matches rust-toolchain.toml
NODE_MAJOR="22"        # Claude Code needs >= 18
TTYD_PORT="7681"
ZELLIJ_SESSION="devbox"
VERIFY_ONLY=0

while [ $# -gt 0 ]; do
  case "$1" in
    --user) DEV_USER="$2"; shift 2 ;;
    --repo-url) REPO_URL="$2"; shift 2 ;;
    --port) TTYD_PORT="$2"; shift 2 ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

REPO_DIR="/home/$DEV_USER/$(basename "$REPO_URL" .git)"

# The PROVEN apt set from linux-runner-bootstrap.sh — a devbox has to be able
# to BUILD the product, not just edit it. That is the whole pitch: the seat
# compiles and tests, so the agent can prove a change works.
APT_DEPS=(build-essential cmake pkg-config ffmpeg libssl-dev
  libegl1-mesa-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev
  libxkbcommon-x11-dev libx11-dev libxcursor-dev libxrandr-dev
  libxi-dev libxtst-dev libxdo-dev libudev-dev libevdev-dev
  libasound2-dev libpipewire-0.3-dev libdbus-1-dev libgtk-3-dev
  libpango1.0-dev libcairo2-dev libglib2.0-dev libv4l-dev
  libgbm-dev libxcb1-dev
  clang libclang-dev llvm-dev curl git ca-certificates)

step() { echo "== $*"; }
skip() { echo "   SKIP (already satisfied): $*"; }

as_dev() { sudo -u "$DEV_USER" bash -lc "$1"; }

have_deps()   { dpkg -s "${APT_DEPS[@]}" >/dev/null 2>&1; }
have_egl()    { pkg-config --exists egl; }
have_node()   { command -v node >/dev/null 2>&1 && \
                [ "$(node -v | sed 's/^v\([0-9]*\).*/\1/')" -ge 18 ]; }
have_claude() { command -v claude >/dev/null 2>&1; }
have_cargo()  { as_dev 'command -v cargo' >/dev/null 2>&1; }
have_zellij() { command -v zellij >/dev/null 2>&1; }
have_ttyd()   { command -v ttyd >/dev/null 2>&1; }
have_repo()   { [ -d "$REPO_DIR/.git" ]; }
hooks_wired() { [ "$(as_dev "cd '$REPO_DIR' && git config core.hooksPath" 2>/dev/null)" = ".githooks" ]; }
ttyd_active() { systemctl is-active --quiet "devbox-ttyd@$DEV_USER.service" 2>/dev/null; }

arch_tag() {
  case "$(dpkg --print-architecture)" in
    amd64) echo x86_64 ;;
    arm64) echo aarch64 ;;
    *) echo "unsupported arch $(dpkg --print-architecture)" >&2; return 1 ;;
  esac
}

if [ "$VERIFY_ONLY" = 1 ]; then
  step "verify-only audit"
  fails=0
  have_deps    || { echo "   FAIL: apt deps incomplete"; fails=$((fails+1)); }
  have_egl     || { echo "   FAIL: pkg-config egl missing"; fails=$((fails+1)); }
  id "$DEV_USER" >/dev/null 2>&1 || { echo "   FAIL: user $DEV_USER missing"; fails=$((fails+1)); }
  have_node    || { echo "   FAIL: node >= 18 missing"; fails=$((fails+1)); }
  have_claude  || { echo "   FAIL: claude CLI missing"; fails=$((fails+1)); }
  have_cargo   || { echo "   FAIL: cargo missing for $DEV_USER"; fails=$((fails+1)); }
  have_zellij  || { echo "   FAIL: zellij missing"; fails=$((fails+1)); }
  have_ttyd    || { echo "   FAIL: ttyd missing"; fails=$((fails+1)); }
  have_repo    || { echo "   FAIL: repo not cloned at $REPO_DIR"; fails=$((fails+1)); }
  hooks_wired  || { echo "   FAIL: core.hooksPath != .githooks (gate bypassed)"; fails=$((fails+1)); }
  ttyd_active  || { echo "   FAIL: devbox-ttyd@$DEV_USER not active"; fails=$((fails+1)); }
  [ "$fails" = 0 ] && { echo "VERIFY: all green"; exit 0; }
  echo "VERIFY: $fails problem(s)"; exit 1
fi

[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 2; }

step "apt deps (${#APT_DEPS[@]} packages)"
if have_deps; then skip "all present"; else
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y -qq "${APT_DEPS[@]}"
fi
have_egl || { echo "pkg-config egl STILL missing after install" >&2; exit 1; }

step "devbox user"
if id "$DEV_USER" >/dev/null 2>&1; then skip "user exists"; else
  useradd -m -s /bin/bash "$DEV_USER"
fi

step "node $NODE_MAJOR"
if have_node; then skip "node $(node -v) present"; else
  curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR}.x" | bash -
  DEBIAN_FRONTEND=noninteractive apt-get install -y -qq nodejs
  have_node || { echo "node STILL missing after nodesource" >&2; exit 1; }
fi

step "claude code CLI"
if have_claude; then skip "claude $(claude --version 2>/dev/null || echo present)"; else
  npm install -g @anthropic-ai/claude-code
  have_claude || { echo "claude STILL missing after npm install" >&2; exit 1; }
fi

step "rust $RUST_VER for $DEV_USER"
if have_cargo; then skip "cargo present"; else
  as_dev "curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUST_VER --profile default"
  have_cargo || { echo "cargo STILL missing after rustup" >&2; exit 1; }
fi

step "zellij"
if have_zellij; then skip "zellij present"; else
  a="$(arch_tag)"
  curl -fsSL -o /tmp/zellij.tgz \
    "https://github.com/zellij-org/zellij/releases/latest/download/zellij-${a}-unknown-linux-musl.tar.gz"
  tar -xzf /tmp/zellij.tgz -C /usr/local/bin zellij
  rm -f /tmp/zellij.tgz
  chmod +x /usr/local/bin/zellij
  have_zellij || { echo "zellij STILL missing after install" >&2; exit 1; }
fi

step "ttyd"
if have_ttyd; then skip "ttyd present"; else
  # Debian bookworm+ / Ubuntu 24.04 carry it; static binary is the fallback.
  DEBIAN_FRONTEND=noninteractive apt-get install -y -qq ttyd 2>/dev/null || {
    a="$(arch_tag)"
    curl -fsSL -o /usr/local/bin/ttyd \
      "https://github.com/tsl0922/ttyd/releases/latest/download/ttyd.${a}"
    chmod +x /usr/local/bin/ttyd
  }
  have_ttyd || { echo "ttyd STILL missing after install" >&2; exit 1; }
fi

step "repo -> $REPO_DIR"
if have_repo; then skip "clone exists"; else
  as_dev "git clone --quiet '$REPO_URL' '$REPO_DIR'"
  have_repo || { echo "clone failed" >&2; exit 1; }
fi
# The fresh-clone law: without this the pre-push gate silently does nothing.
as_dev "cd '$REPO_DIR' && git config core.hooksPath .githooks"
hooks_wired || { echo "core.hooksPath did not stick" >&2; exit 1; }

step "ttyd service on 127.0.0.1:$TTYD_PORT"
cat >/etc/systemd/system/"devbox-ttyd@.service" <<UNIT
[Unit]
Description=devbox terminal (ttyd -> zellij) for %i
After=network.target

[Service]
User=%i
WorkingDirectory=/home/%i
# --interface lo is load-bearing: this is an unauthenticated root-equivalent
# shell. Terminate TLS and authenticate in front of it.
ExecStart=/usr/local/bin/ttyd --port $TTYD_PORT --interface lo --writable \\
  /usr/local/bin/zellij attach --create $ZELLIJ_SESSION
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
UNIT
# The unit hardcodes ttyd at /usr/local/bin; apt puts it in /usr/bin.
if [ ! -x /usr/local/bin/ttyd ]; then
  ln -sf "$(command -v ttyd)" /usr/local/bin/ttyd
fi
systemctl daemon-reload
systemctl enable --now "devbox-ttyd@$DEV_USER.service"

step "final verify"
ttyd_active || { echo "devbox-ttyd@$DEV_USER not active after start" >&2; exit 1; }
echo "BOOTSTRAP GREEN: devbox ready."
echo "  terminal:  http://127.0.0.1:$TTYD_PORT  (loopback — proxy + auth in front)"
echo "  repo:      $REPO_DIR"
echo "  next:      sudo -u $DEV_USER -i, cd $REPO_DIR, run 'claude' and authenticate."
echo "             Auth is per-seat and is NOT baked into the image."
