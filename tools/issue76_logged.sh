#!/usr/bin/env bash
# Issue #76 runner helper: run a command; on failure, also report the end of its output as a
# GitHub error annotation (readable through the REST API when raw logs are not reachable).
log=$(mktemp)
"$@" > >(tee "$log") 2>&1
code=$?
sleep 1
if [ "$code" -ne 0 ]; then
  python3 - "$log" "$*" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
lines = [l for l in text.splitlines() if l.strip()]
# Prefer compiler diagnostics; fall back to the tail.
keep = [l for l in lines if l.startswith(("error", "warning", "  -->", "   |", "test ", "thread ", "---- ", "assertion"))]
body = "\n".join((keep or lines)[-120:])[-12000:]
esc = body.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
print(f"::error title=command failed::{sys.argv[2][:80]}%0A{esc}")
PY
fi
exit "$code"
