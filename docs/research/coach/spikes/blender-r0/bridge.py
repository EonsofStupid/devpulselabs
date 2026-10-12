# Blender R0 spike bridge — loaded at launch: blender --python bridge.py
#
# stdlib only. A socket thread ENQUEUES commands; bpy executes ONLY on
# the main thread via bpy.app.timers (bpy is not thread-safe). Blender's
# own handlers emit timestamped facts (selection/load) to a JSONL file —
# the operator-pace stream. Spike plan:
# docs-history-2026-10-12:docs/plans/coach-seam-s4-spike-blender-r0.md

import json
import os
import queue
import socket
import threading
import time

import bpy
from bpy.app.handlers import persistent

PORT = int(os.environ.get("BRIDGE_PORT", "7844"))
EVENTS_PATH = os.environ.get(
    "BRIDGE_EVENTS", os.path.join(os.path.dirname(__file__), "spike-events.jsonl")
)

_commands = queue.Queue()  # (payload_dict, reply_queue)
_last_selection = None  # main-thread cache for change detection


def now_ms():
    return int(time.time() * 1000)


def emit_fact(kind, **fields):
    fact = {"ts_ms": now_ms(), "kind": kind}
    fact.update(fields)
    with open(EVENTS_PATH, "a", encoding="utf-8") as f:
        f.write(json.dumps(fact) + "\n")


def selected_names():
    return sorted(o.name for o in bpy.context.selected_objects)


# ── facts: Blender tells us what happened, in-process ──────────────────


@persistent
def on_depsgraph(scene, depsgraph):
    global _last_selection
    names = selected_names()
    if names != _last_selection:
        emit_fact("selection_changed", objects=names, was=_last_selection)
        _last_selection = names


@persistent
def on_load(filepath):
    emit_fact("file_loaded", filepath=bpy.data.filepath)


# ── verbs: executed on the MAIN thread only ─────────────────────────────


def run_command(cmd):
    verb = cmd.get("verb", "")
    started = now_ms()
    if verb == "ping":
        return {"ok": True, "verb": verb, "pid": os.getpid(), "ms": 0}
    if verb == "list":
        return {
            "ok": True,
            "verb": verb,
            "objects": sorted(o.name for o in bpy.data.objects),
            "selected": selected_names(),
            "filepath": bpy.data.filepath,
            "ms": now_ms() - started,
        }
    if verb == "open":
        path = cmd.get("path", "")
        before = bpy.data.filepath
        bpy.ops.wm.open_mainfile(filepath=path)
        after = bpy.data.filepath
        return {
            "ok": True,
            "verb": verb,
            "effect_observed": after != before and after.endswith(os.path.basename(path)),
            "before": before,
            "after": after,
            "ms": now_ms() - started,
        }
    if verb == "select":
        name = cmd.get("name", "")
        obj = bpy.data.objects.get(name)
        if obj is None:
            return {"ok": False, "verb": verb, "reason": f"no object named {name!r}"}
        before = selected_names()
        for o in bpy.context.selected_objects:
            o.select_set(False)
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        after = selected_names()
        return {
            "ok": True,
            "verb": verb,
            "effect_observed": after == [name],
            "before": before,
            "after": after,
            "ms": now_ms() - started,
        }
    if verb == "deselect_all":
        before = selected_names()
        for o in bpy.context.selected_objects:
            o.select_set(False)
        after = selected_names()
        return {
            "ok": True,
            "verb": verb,
            "effect_observed": after == [],
            "before": before,
            "after": after,
            "ms": now_ms() - started,
        }
    return {"ok": False, "verb": verb, "reason": "unknown verb"}


def drain_queue():
    """bpy.app.timers callback — main thread, every 10 ms."""
    while True:
        try:
            cmd, reply_q = _commands.get_nowait()
        except queue.Empty:
            break
        try:
            reply = run_command(cmd)
        except Exception as e:  # a broken verb must never kill the timer
            reply = {"ok": False, "verb": cmd.get("verb", ""), "reason": repr(e)}
        reply_q.put(reply)
    return 0.01


# ── socket thread: transport only, never touches bpy ────────────────────


def serve():
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", PORT))
    srv.listen(4)
    emit_fact("bridge_listening", port=PORT)
    while True:
        conn, _ = srv.accept()
        with conn:
            data = conn.makefile("r", encoding="utf-8").readline()
            if not data.strip():
                continue
            try:
                cmd = json.loads(data)
            except json.JSONDecodeError as e:
                conn.sendall((json.dumps({"ok": False, "reason": repr(e)}) + "\n").encode())
                continue
            reply_q = queue.Queue()
            _commands.put((cmd, reply_q))
            try:
                reply = reply_q.get(timeout=30)
            except queue.Empty:
                reply = {"ok": False, "reason": "main-thread timeout"}
            conn.sendall((json.dumps(reply) + "\n").encode())


bpy.app.handlers.depsgraph_update_post.append(on_depsgraph)
bpy.app.handlers.load_post.append(on_load)
bpy.app.timers.register(drain_queue, persistent=True)
threading.Thread(target=serve, daemon=True).start()
emit_fact("bridge_started", blender=bpy.app.version_string, pid=os.getpid())
print(f"[bridge] listening on 127.0.0.1:{PORT}, events -> {EVENTS_PATH}")
