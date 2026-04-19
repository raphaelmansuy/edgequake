#!/usr/bin/env python3
"""Pick a safe local port for EdgeQuake without interfering with other stacks.

Rules:
- Prefer the requested port when it is free.
- Reuse the requested port when EdgeQuake is already running there.
- If another app is listening on that port, scan upward for the next free port.
"""

from __future__ import annotations

import socket
import sys
import urllib.error
import urllib.request


def is_listening(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(0.2)
        return sock.connect_ex(("127.0.0.1", port)) == 0


def is_edgequake(kind: str, port: int) -> bool:
    url = (
        f"http://127.0.0.1:{port}/health"
        if kind == "backend"
        else f"http://127.0.0.1:{port}/"
    )
    try:
        with urllib.request.urlopen(url, timeout=0.5) as response:
            body = response.read(4096).decode("utf-8", errors="ignore")
        if kind == "backend":
            return '"status"' in body and "healthy" in body.lower()
        return "EdgeQuake" in body
    except (urllib.error.URLError, TimeoutError, ValueError):
        return False


def choose_port(kind: str, preferred_port: int, scan_window: int) -> int:
    candidate_ports = range(preferred_port, preferred_port + scan_window + 1)

    for port in candidate_ports:
        if is_edgequake(kind, port):
            return port

    for port in candidate_ports:
        if not is_listening(port):
            return port

    return preferred_port


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: select_edgequake_port.py <backend|frontend> <preferred_port> [scan_window]", file=sys.stderr)
        return 1

    kind = sys.argv[1].strip().lower()
    preferred_port = int(sys.argv[2])
    scan_window = int(sys.argv[3]) if len(sys.argv) > 3 else 20

    if kind not in {"backend", "frontend"}:
        print(preferred_port)
        return 0

    print(choose_port(kind, preferred_port, scan_window))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
