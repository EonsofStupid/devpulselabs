#!/usr/bin/env bash
# Session OS — sessionStart: env + resume pointer (disk is source of truth).
set -euo pipefail
ROOT="${CURSOR_PROJECT_DIR:-$(pwd)}"
RESUME="$ROOT/docs/ops/resume/CURRENT.md"
SESSION="$ROOT/docs/ops/SESSION.md"
STATE="$ROOT/docs/STATE.md"
INPUT="$(cat)"

DPL_OPS_ROOT="$ROOT/docs/ops" \
DPL_OPS_RESUME="$RESUME" \
DPL_OPS_SESSION="$SESSION" \
DPL_OPS_STATE="$STATE" \
HOOK_INPUT="$INPUT" \
python3 <<'PY'
import json, os, pathlib

payload = {}
raw = os.environ.get("HOOK_INPUT", "")
if raw.strip():
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        payload = {}

sid = payload.get("session_id") or payload.get("conversation_id") or ""
state = pathlib.Path(os.environ["DPL_OPS_STATE"])
session = pathlib.Path(os.environ["DPL_OPS_SESSION"])
resume = pathlib.Path(os.environ["DPL_OPS_RESUME"])

lines = [
    "Session OS boot: read docs/STATE.md, docs/ops/SESSION.md, docs/ops/resume/CURRENT.md before acting.",
    "Adopt skill session-os. Ops ledger ≠ product dpl-ledger.",
]
for label, path in (("STATE", state), ("SESSION", session), ("RESUME", resume)):
    if path.is_file():
        text = path.read_text(encoding="utf-8", errors="replace")
        snippet = text if len(text) <= 4000 else text[:4000] + "\n…[truncated; read full file]\n"
        lines.append(f"\n--- {label} ({path.name}) ---\n{snippet}")

print(
    json.dumps(
        {
            "env": {
                "DPL_OPS_ROOT": os.environ["DPL_OPS_ROOT"],
                "DPL_OPS_RESUME": str(resume),
                "DPL_OPS_SESSION": str(session),
                "DPL_OPS_SESSION_ID": sid,
            },
            "additional_context": "\n".join(lines),
        }
    )
)
PY
