#!/usr/bin/env bash
# Session OS — sessionStart: env + resume pointer (disk is source of truth).
set -euo pipefail
ROOT="${CURSOR_PROJECT_DIR:-$(pwd)}"
STATUS="$ROOT/docs/status.md"
INPUT="$(cat)"

DPL_OPS_ROOT="$ROOT/docs/ops" \
DPL_OPS_STATUS="$STATUS" \
DPL_DOCS_JOURNAL="$ROOT/docs/journal" \
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
status = pathlib.Path(os.environ["DPL_OPS_STATUS"])
journals = sorted(
    pathlib.Path(os.environ["DPL_DOCS_JOURNAL"]).glob("*.md"),
    key=lambda p: p.stat().st_mtime,
)

lines = [
    "Session OS boot: read docs/status.md, then docs/requirements.md (the operator's words), then docs/README.md (the doc map) before acting.",
    "Adopt skill session-os. Ops ledger ≠ product dpl-ledger.",
]
for label, path in (("STATUS", status), ("LATEST JOURNAL", journals[-1] if journals else None)):
    if path is not None and path.is_file():
        text = path.read_text(encoding="utf-8", errors="replace")
        snippet = text if len(text) <= 4000 else text[:4000] + "\n…[truncated; read full file]\n"
        lines.append(f"\n--- {label} ({path.name}) ---\n{snippet}")

print(
    json.dumps(
        {
            "env": {
                "DPL_OPS_ROOT": os.environ["DPL_OPS_ROOT"],
                "DPL_OPS_STATUS": str(status),
                "DPL_OPS_SESSION_ID": sid,
            },
            "additional_context": "\n".join(lines),
        }
    )
)
PY
