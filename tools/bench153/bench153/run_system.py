#!/usr/bin/env python3
"""System bench: L1–L6 against local mock-backed EdgeQuake."""

from __future__ import annotations

import argparse
import json
import time
import uuid
from pathlib import Path

from bench153 import (
    Client,
    closed_loop,
    dense_doc,
    estimate_embed_tokens,
    pct,
    short_doc,
    write_json,
    write_jsonl,
)


def wait_doc(client: Client, doc_id: str, timeout_s: float = 300.0) -> dict:
    deadline = time.time() + timeout_s
    last = {}
    while time.time() < deadline:
        status, payload, _ = client.request("GET", f"/api/v1/documents/{doc_id}")
        if status == 200 and isinstance(payload, dict):
            last = payload
            st = (payload.get("status") or payload.get("processing_status") or "").lower()
            if st in {"completed", "failed", "partial_failure", "partialfailure"}:
                return payload
        time.sleep(0.5)
    last["_timeout"] = True
    return last


def upload_and_wait(client: Client, title: str, content: str, idx: int, shape: str) -> dict:
    t0 = time.perf_counter()
    status, payload, admit_ms = client.request(
        "POST",
        "/api/v1/documents",
        {
            "content": content,
            "title": title,
            "async_processing": True,
        },
    )
    if status not in (200, 202) or not isinstance(payload, dict):
        return {
            "unit_id": f"{shape}-{idx}",
            "shape_id": shape,
            "oracle_ok": False,
            "http_status": status,
            "error": payload,
            "admit_ms": admit_ms,
            "wall_ms": (time.perf_counter() - t0) * 1000,
        }
    doc_id = payload.get("id") or payload.get("document_id")
    doc = wait_doc(client, str(doc_id)) if doc_id else {}
    wall_ms = (time.perf_counter() - t0) * 1000
    st = str(doc.get("status") or doc.get("processing_status") or "").lower()
    oracle_ok = st in {"completed", "partial_failure", "partialfailure", "done"}
    # cost fields vary by API version — harvest what exists
    cost = doc.get("cost_breakdown") or doc.get("stats") or {}
    return {
        "unit_id": f"{shape}-{idx}",
        "shape_id": shape,
        "layers": ["L1", "L3", "L4", "L5", "L6"],
        "bench_mode": "system",
        "document_id": doc_id,
        "http_status": status,
        "doc_status": st,
        "oracle_ok": oracle_ok,
        "admit_ms": round(admit_ms, 1),
        "wall_ms": round(wall_ms, 1),
        "sojourn_ms": round(wall_ms, 1),
        "embed_est": estimate_embed_tokens(content),
        "extraction_input_tokens": cost.get("extraction_input_tokens")
        or cost.get("prompt_tokens"),
        "extraction_output_tokens": cost.get("extraction_output_tokens")
        or cost.get("completion_tokens"),
        "embedding_tokens": cost.get("embedding_tokens"),
        "entity_count": doc.get("entity_count") or doc.get("entities_count"),
        "chunk_count": doc.get("chunk_count") or doc.get("chunks_count"),
    }


def summarize_level(units: list[dict], wall_s: float, concurrency: int) -> dict:
    ok = [u for u in units if u.get("oracle_ok")]
    walls = [u["wall_ms"] for u in units]
    embed = [u["embed_est"] for u in units if u.get("embed_est")]
    return {
        "concurrency": concurrency,
        "n": len(units),
        "oracle_ok_rate": round(len(ok) / len(units), 3) if units else 0,
        "wall_s": round(wall_s, 2),
        "docs_per_s": round(len(ok) / wall_s, 4) if wall_s else None,
        "wall_ms_p50": round(pct(walls, 50) or 0, 1),
        "wall_ms_p90": round(pct(walls, 90) or 0, 1),
        "embed_est_sum": sum(embed),
        "embed_est_per_s": round(sum(embed) / wall_s, 2) if wall_s and embed else None,
    }


def run_ingest_shape(client: Client, shape: str, maker, n: int, concurrencies: list[int]):
    levels = []
    all_units = []
    for C in concurrencies:
        def worker(i, _C=C):
            return upload_and_wait(client, f"{shape}-{_C}-{i}", maker(i), i, shape)

        units, wall_s = closed_loop(n, C, worker)
        for u in units:
            u["concurrency"] = C
        all_units.extend(units)
        levels.append(summarize_level(units, wall_s, C))
        print(f"  {shape} C={C} ok_rate={levels[-1]['oracle_ok_rate']} p50={levels[-1]['wall_ms_p50']}ms")
    return levels, all_units


def run_queue_proxy(client: Client, n: int = 20) -> tuple[dict, list[dict]]:
    """Approximate L1 pressure: rapid admits of tiny docs; measure admit latency."""
    units = []
    t0 = time.perf_counter()
    for i in range(n):
        content = f"# q{i}\nhello {uuid.uuid4().hex}\n"
        status, payload, admit_ms = client.request(
            "POST",
            "/api/v1/documents",
            {"content": content, "title": f"queue-proxy-{i}", "async_processing": True},
        )
        units.append(
            {
                "unit_id": f"queue-only-{i}",
                "shape_id": "queue-only-v1",
                "layers": ["L1"],
                "bench_mode": "system",
                "http_status": status,
                "oracle_ok": status in (200, 202),
                "admit_ms": round(admit_ms, 1),
                "document_id": (payload or {}).get("id") if isinstance(payload, dict) else None,
            }
        )
    wall_s = time.perf_counter() - t0
    admits = [u["admit_ms"] for u in units]
    # drain a few to avoid leaving junk pending forever
    for u in units[:5]:
        if u.get("document_id"):
            wait_doc(client, u["document_id"], timeout_s=120)
    summary = {
        "shape_id": "queue-only-v1",
        "n": n,
        "wall_s": round(wall_s, 2),
        "admits_per_s": round(n / wall_s, 2) if wall_s else None,
        "admit_ms_p50": round(pct(admits, 50) or 0, 1),
        "admit_ms_p90": round(pct(admits, 90) or 0, 1),
        "oracle_ok_rate": round(sum(1 for u in units if u["oracle_ok"]) / n, 3),
        "note": "Admit-path proxy for L1; full claim_next physics remains SPEC-090",
    }
    return summary, units


def run_embed_batch(client: Client, n_strings: int = 32) -> dict:
    """Embed path via one dense ingest; report embed_est from content size."""
    texts = [f"Embedding batch string {i}: " + ("vector " * 20) + uuid.uuid4().hex for i in range(n_strings)]
    content = "# embed-batch-v1\n\n" + "\n\n".join(texts)
    t0 = time.perf_counter()
    unit = upload_and_wait(client, "embed-batch-v1", content, 0, "embed-batch-v1")
    wall_s = time.perf_counter() - t0
    est = estimate_embed_tokens(content)
    return {
        "shape_id": "embed-batch-v1",
        "n_strings": n_strings,
        "wall_s": round(wall_s, 2),
        "oracle_ok": unit.get("oracle_ok"),
        "embed_est": est,
        "embed_est_per_s": round(est / wall_s, 2) if wall_s else None,
        "unit": unit,
        "trust": "embed_est",
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-url", default="http://127.0.0.1:8080")
    ap.add_argument("--out", required=True)
    ap.add_argument("--short-n", type=int, default=4)
    ap.add_argument("--dense-n", type=int, default=2)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    # Local mock uses server default tenant/workspace (do not send unknown UUIDs).
    client = Client(base_url=args.base_url)

    # health
    st, health, _ = client.request("GET", "/health")
    write_json(out / "preflight-health.json", health if st == 200 else {"status": st, "body": health})
    if st != 200:
        print("Local health failed", st, health)
        return 2

    cards = {}
    all_units: list[dict] = []

    print("S1 queue-only-v1")
    qsum, qunits = run_queue_proxy(client, n=16)
    cards["S1_queue_only"] = qsum
    all_units.extend(qunits)

    print("S2 ingest-short-v1")
    levels, units = run_ingest_shape(client, "ingest-short-v1", short_doc, args.short_n, [1, 2])
    cards["S2_ingest_short"] = {"levels": levels}
    all_units.extend(units)

    print("S3 ingest-dense-v1")
    levels, units = run_ingest_shape(client, "ingest-dense-v1", dense_doc, args.dense_n, [1])
    cards["S3_ingest_dense"] = {"levels": levels}
    all_units.extend(units)

    print("S4 embed-batch-v1")
    cards["S4_embed_batch"] = run_embed_batch(client)
    if cards["S4_embed_batch"].get("unit"):
        all_units.append(cards["S4_embed_batch"]["unit"])

    summary = {
        "spec": "153",
        "card": "system",
        "base_url": args.base_url,
        "tenant_id": "server_default",
        "workspace_id": "server_default",
        "bench_mode": "system",
        "llm_mode": "mock",
        "health_version": health.get("version") if isinstance(health, dict) else None,
        "cards": cards,
        "result": "PASS"
        if all(
            u.get("oracle_ok")
            for u in all_units
            if str(u.get("shape_id", "")).startswith("ingest")
        )
        else "PARTIAL",
    }
    write_json(out / "summary.json", summary)
    write_jsonl(out / "units.jsonl", all_units)
    write_json(
        out / "env.json",
        {
            "base_url": args.base_url,
            "tenant_id": "server_default",
            "workspace_id": "server_default",
            "provider": "mock",
            "cache": "EDGEQUAKE_LLM_CACHE=0 expected",
        },
    )
    print(json.dumps({"result": summary["result"], "cards": list(cards)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
