#!/usr/bin/env bash
# Session OS — stop: nudge once if SESSION dirty.
set -euo pipefail
ROOT="${CURSOR_PROJECT_DIR:-$(pwd)}"
SESSION="$ROOT/docs/ops/SESSION.md"
INPUT="$(cat)"

DPL_OPS_SESSION="$SESSION" \
HOOK_INPUT="$INPUT" \
python3 <<'PY'
import json, os, re, pathlib

payload = {}
raw = os.environ.get("HOOK_INPUT", "")
if raw.strip():
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        payload = {}

loop_count = int(payload.get("loop_count") or 0)
session = pathlib.Path(os.environ["DPL_OPS_SESSION"])
dirty = False
if session.is_file():
    text = session.read_text(encoding="utf-8", errors="replace")
    m = re.search(r"(?m)^dirty:\s*(true|false)\s*$", text)
    if m and m.group(1) == "true":
        dirty = True

out = {}
if dirty and loop_count < 1:
    out["followup_message"] = (
        "Session OS: docs/ops/SESSION.md is dirty: true. "
        "Before stopping, refresh docs/ops/resume/CURRENT.md, append journal if needed, "
        "emit journal.wrote, set dirty: false."
    )
print(json.dumps(out))
PY
