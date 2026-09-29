"""SPEC-153 workload benchmark harness."""

from __future__ import annotations

import json
import math
import statistics
import time
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterable
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

TENANT_DEMO = "00000000-0000-0000-0000-000000000002"
WORKSPACE_DEMO = "00000000-0000-0000-0000-000000000003"
TENANT_LOCAL = "00000000-0000-0000-0000-0000000000b1"
WORKSPACE_LOCAL = "00000000-0000-0000-0000-0000000000b2"


def pct(xs: list[float], p: float) -> float | None:
    if not xs:
        return None
    ys = sorted(xs)
    k = (len(ys) - 1) * p / 100.0
    f = int(k)
    c = min(f + 1, len(ys) - 1)
    if f == c:
        return float(ys[f])
    return float(ys[f] + (ys[c] - ys[f]) * (k - f))


def write_json(path: Path, obj: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2) + "\n")


def write_jsonl(path: Path, rows: Iterable[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(json.dumps(r) for r in rows) + "\n")


@dataclass
class Client:
    base_url: str
    api_key: str | None = None
    tenant_id: str | None = None
    workspace_id: str | None = None
    timeout_s: float = 180.0

    def _headers(self, extra: dict | None = None) -> dict[str, str]:
        h = {"Content-Type": "application/json", "Accept": "application/json"}
        if self.api_key:
            h["X-API-Key"] = self.api_key
        if self.tenant_id:
            h["X-Tenant-ID"] = self.tenant_id
        if self.workspace_id:
            h["X-Workspace-ID"] = self.workspace_id
        if extra:
            h.update(extra)
        return h

    def request(
        self,
        method: str,
        path: str,
        body: dict | None = None,
        raw: bool = False,
        headers: dict | None = None,
    ) -> tuple[int, Any, float]:
        data = None if body is None else json.dumps(body).encode()
        req = Request(
            self.base_url.rstrip("/") + path,
            data=data,
            method=method,
            headers=self._headers(headers),
        )
        t0 = time.perf_counter()
        try:
            with urlopen(req, timeout=self.timeout_s) as resp:
                status = resp.status
                payload = resp.read()
        except HTTPError as e:
            status = e.code
            payload = e.read() if e.fp else b""
        except URLError as e:
            return 0, {"error": str(e)}, (time.perf_counter() - t0) * 1000
        wall_ms = (time.perf_counter() - t0) * 1000
        if raw:
            return status, payload, wall_ms
        try:
            parsed = json.loads(payload.decode() or "null")
        except json.JSONDecodeError:
            parsed = {"raw": payload.decode(errors="replace")[:500]}
        return status, parsed, wall_ms


def closed_loop(
    n: int,
    concurrency: int,
    worker: Callable[[int], dict],
) -> tuple[list[dict], float]:
    units: list[dict] = []
    t0 = time.perf_counter()
    with ThreadPoolExecutor(max_workers=max(1, concurrency)) as ex:
        futs = [ex.submit(worker, i) for i in range(n)]
        for f in as_completed(futs):
            units.append(f.result())
    return units, time.perf_counter() - t0


def closed_loop_timed(
    concurrency: int,
    duration_s: float,
    worker: Callable[[int], dict],
) -> tuple[list[dict], float]:
    """Keep `concurrency` workers busy for `duration_s`, each asking back-to-back."""
    import itertools
    import threading

    counter = itertools.count()
    lock = threading.Lock()
    units: list[dict] = []
    t0 = time.perf_counter()
    deadline = t0 + duration_s

    def loop() -> None:
        while time.perf_counter() < deadline:
            with lock:
                idx = next(counter)
            u = worker(idx)
            with lock:
                units.append(u)

    with ThreadPoolExecutor(max_workers=max(1, concurrency)) as ex:
        futs = [ex.submit(loop) for _ in range(max(1, concurrency))]
        for f in futs:
            f.result()
    return units, time.perf_counter() - t0


def open_loop(
    rate_qps: float,
    duration_s: float,
    worker: Callable[[int], dict],
    max_inflight: int = 64,
) -> tuple[list[dict], float, int]:
    """Open-loop arrivals at `rate_qps` for `duration_s`.

    Returns (completed_units, wall_s, offered_count).
    Offered count is how many starts were scheduled; completions may trail.
    """
    if rate_qps <= 0:
        raise ValueError("rate_qps must be > 0")
    interval = 1.0 / rate_qps
    units: list[dict] = []
    offered = 0
    t0 = time.perf_counter()
    deadline = t0 + duration_s
    next_at = t0
    with ThreadPoolExecutor(max_workers=max(1, max_inflight)) as ex:
        futs = []
        while time.perf_counter() < deadline:
            now = time.perf_counter()
            if now < next_at:
                time.sleep(min(0.005, next_at - now))
                continue
            idx = offered
            offered += 1
            futs.append(ex.submit(worker, idx))
            next_at += interval
            # Drain finished to bound memory
            still = []
            for f in futs:
                if f.done():
                    units.append(f.result())
                else:
                    still.append(f)
            futs = still
        for f in as_completed(futs):
            units.append(f.result())
    return units, time.perf_counter() - t0, offered


def estimate_embed_tokens(text: str) -> int:
    return int(math.ceil(len(text) / 2.5))


def short_doc(i: int) -> str:
    return (
        f"# Bench153 short doc {i}\n\n"
        "EdgeQuake extracts entities from technical prose. "
        "Alice works at Acme Corp in Paris. Bob reports to Alice on Project Orion. "
        "The team uses PostgreSQL and pgvector for retrieval. "
        f"Marker {uuid.uuid4().hex[:8]}.\n"
    )


def dense_doc(i: int) -> str:
    para = (
        "Knowledge graphs link entities and relationships across documents. "
        "Sarah Chen founded Nova Labs in 2019. Nova Labs partnered with Helix Bio. "
        "Their product Aurora uses hybrid retrieval mixing local and global graph arms. "
        "Evaluation must separate answer correctness from retrieval recall. "
    )
    body = (para * 40) + f"\n\nUnique marker {uuid.uuid4().hex} doc-{i}.\n"
    return f"# Bench153 dense doc {i}\n\n{body}"
