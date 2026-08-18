# Spike driver — sends one JSON-line command to the bridge, prints the
# reply plus round-trip latency. stdlib only.
#   python3 drive.py '{"verb":"select","name":"Cube"}'

import json
import socket
import sys
import time

cmd = sys.argv[1] if len(sys.argv) > 1 else '{"verb":"ping"}'
port = int(sys.argv[2]) if len(sys.argv) > 2 else 7844

t0 = time.time()
s = socket.create_connection(("127.0.0.1", port), timeout=35)
s.sendall((cmd.strip() + "\n").encode())
reply = s.makefile("r", encoding="utf-8").readline()
rtt_ms = int((time.time() - t0) * 1000)
s.close()

parsed = json.loads(reply)
parsed["rtt_ms"] = rtt_ms
print(json.dumps(parsed, indent=2))
