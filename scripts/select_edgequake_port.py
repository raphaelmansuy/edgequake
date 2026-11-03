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
    """True when this port is served by EdgeQuake.

    WHY `/live` first for the backend: `/health` is a *deep* document. It pings storage and
    may consult the LLM provider, so its latency belongs to the provider, not to this
    process. This function only needs to answer "is EdgeQuake here?", and `/live` answers
    that in milliseconds. Falling back to `/health` keeps compatibility with older builds
    that predate `/live`, and a 2 s budget tolerates a cold connection pool.
    """
    if kind == "backend":
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/live", timeout=2.0) as response:
                if 200 <= response.status < 300:
                    return True
        except (urllib.error.URLError, TimeoutError, ValueError):
            pass  # older build without /live: try the deep document below
    url = (
        f"http://127.0.0.1:{port}/health"
        if kind == "backend"
        else f"http://127.0.0.1:{port}/"
    )
    try:
        with urllib.request.urlopen(url, timeout=2.0) as response:
            body = response.read(4096).decode("utf-8", errors="ignore")
        if kind == "backend":
            return '"status"' in body and (
                '"status":"healthy"' in body.lower()
                or '"status": "healthy"' in body.lower()
                or '"status":"degraded"' in body.lower()
                or '"status": "degraded"' in body.lower()
            )
        return "EdgeQuake" in body
    except (urllib.error.URLError, TimeoutError, ValueError):
        return False


def is_foreign_backend(port: int) -> bool:
    """True when something listens on `port` but it is not EdgeQuake."""
    return is_listening(port) and not is_edgequake("backend", port)


def choose_port(kind: str, preferred_port: int, scan_window: int) -> int:
    candidate_ports = range(preferred_port, preferred_port + scan_window + 1)

    for port in candidate_ports:
        if is_edgequake(kind, port):
            return port

    for port in candidate_ports:
        if kind == "backend" and is_foreign_backend(port):
            continue
        if kind == "frontend" and is_listening(port) and not is_edgequake(kind, port):
            continue
        if not is_listening(port):
            return port

    raise RuntimeError(
        f"No free {kind} port in range {preferred_port}-{preferred_port + scan_window}"
    )


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
