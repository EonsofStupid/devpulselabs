#!/usr/bin/env bash
# Session OS — preCompact: observe-only; snapshot usage + transcript tail to disk.
set -euo pipefail
ROOT="${CURSOR_PROJECT_DIR:-$(pwd)}"
RAW_DIR="$ROOT/docs/ops/resume/raw"
SIGNALS="$ROOT/docs/ops/signals.jsonl"
mkdir -p "$RAW_DIR"
INPUT="$(cat)"

CURSOR_PROJECT_DIR="$ROOT" \
DPL_OPS_RAW="$RAW_DIR" \
DPL_OPS_SIGNALS="$SIGNALS" \
HOOK_INPUT="$INPUT" \
python3 <<'PY'
import json, os, pathlib
from datetime import datetime, timezone

payload = {}
raw = os.environ.get("HOOK_INPUT", "")
if raw.strip():
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        payload = {}

root = os.environ["CURSOR_PROJECT_DIR"]
raw_dir = pathlib.Path(os.environ["DPL_OPS_RAW"])
signals = pathlib.Path(os.environ["DPL_OPS_SIGNALS"])

pct = payload.get("context_usage_percent")
tokens = payload.get("context_tokens")
trigger = payload.get("trigger", "unknown")
sid = payload.get("session_id") or os.environ.get("DPL_OPS_SESSION_ID", "")
ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")

tpath = payload.get("transcript_path") or os.environ.get("CURSOR_TRANSCRIPT_PATH") or ""
tail_note = "no transcript path"
if tpath and pathlib.Path(tpath).is_file():
    try:
        lines = pathlib.Path(tpath).read_text(encoding="utf-8", errors="replace").splitlines()
        tail = lines[-500:] if len(lines) > 500 else lines
        out_path = raw_dir / f"precompact-{stamp}.txt"
        out_path.write_text("\n".join(tail) + "\n", encoding="utf-8")
        ptr = pathlib.Path(root) / "docs/ops/resume/LAST_COMPACT.txt"
        ptr.write_text(
            f"ts={ts}\npct={pct}\ntrigger={trigger}\ntail={out_path}\nlines={len(tail)}\n",
            encoding="utf-8",
        )
        tail_note = str(out_path)
    except OSError as e:
        tail_note = f"transcript read failed: {e}"

try:
    with signals.open("a", encoding="utf-8") as f:
        f.write(
            json.dumps(
                {
                    "ts": ts,
                    "signal": "compact.observed",
                    "session_id": sid,
                    "detail": {
                        "context_usage_percent": pct,
                        "context_tokens": tokens,
                        "trigger": trigger,
                        "tail": tail_note,
                    },
                },
                separators=(",", ":"),
            )
            + "\n"
        )
        f.write(
            json.dumps(
                {
                    "ts": ts,
                    "signal": "context.budget",
                    "session_id": sid,
                    "detail": {"context_usage_percent": pct, "source": "preCompact"},
                },
                separators=(",", ":"),
            )
            + "\n"
        )
except OSError:
    pass

msg = (
    f"Session OS: compaction ({trigger}) at ~{pct}% context. "
    f"Transcript tail → {tail_note}. "
    "Re-read docs/status.md (and refresh the journal if dirty) before continuing."
)
print(json.dumps({"user_message": msg}))
PY
