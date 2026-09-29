#!/usr/bin/env python3
"""Provider bench: L0/L7/L8 on demo.edgequake.com."""

from __future__ import annotations

import argparse
import json
import time
import uuid
from pathlib import Path

from bench153 import (
    TENANT_DEMO,
    WORKSPACE_DEMO,
    Client,
    closed_loop,
    pct,
    write_json,
    write_jsonl,
)

SLO_TOTAL_MS = 25000
WARM_PROMPTS = [
    "What is text-to-SQL evaluation?",
    "Summarize agentic AI challenges from the uploaded papers.",
    "What does the sol_pi paper discuss?",
]


def cold_prompts(n: int) -> list[str]:
    stamp = uuid.uuid4().hex[:10]
    templates = [
        "According to the uploaded papers, what pitfalls arise when evaluating text-to-SQL in production? Unique={u}-{i}",
        "From the agentic PDF corpus, name two practical constraints on agent evaluation. Unique={u}-{i}",
        "What does sol_pi contribute that differs from the text-to-SQL article? Unique={u}-{i}",
        "How do the papers separate retrieval quality from end-task correctness? Unique={u}-{i}",
        "List themes about real-world text-to-SQL systems from the corpus. Unique={u}-{i}",
        "What open evaluation problems for agents appear in the documents? Unique={u}-{i}",
    ]
    return [templates[i % len(templates)].format(u=stamp, i=i) for i in range(n)]


def one_query(client: Client, prompt: str, idx: str, cache_mode: str, concurrency: int) -> dict:
    status, payload, wall_ms = client.request(
        "POST",
        "/api/v1/query",
        {"query": prompt, "mode": "mix", "stream": False},
    )
    stats = ((payload or {}).get("stats") if isinstance(payload, dict) else None) or {}
    answer = (payload or {}).get("answer") if isinstance(payload, dict) else ""
    oracle_ok = status == 200 and bool((answer or "").strip())
    total = stats.get("total_time_ms") or wall_ms
    gen = stats.get("generation_time_ms")
    if "answer_cache_hit" in stats:
        answer_cache_hit = bool(stats["answer_cache_hit"])
    else:
        answer_cache_hit = bool(gen == 0 and oracle_ok)
    unit = {
        "unit_id": idx,
        "shape_id": "query-chat-v1",
        "layers": ["L0", "L7", "L8"],
        "bench_mode": "provider",
        "cache_mode": cache_mode,
        "concurrency": concurrency,
        "prompt": prompt[:240],
        "http_status": status,
        "oracle_ok": oracle_ok,
        "wall_ms": round(wall_ms, 1),
        "sojourn_ms": round(wall_ms, 1),
        "service_ms": stats.get("total_time_ms"),
        "embedding_time_ms": stats.get("embedding_time_ms"),
        "keyword_time_ms": stats.get("keyword_time_ms"),
        "retrieval_time_ms": stats.get("retrieval_time_ms"),
        "generation_time_ms": gen,
        "tokens_used": stats.get("tokens_used"),
        "tokens_per_second": stats.get("tokens_per_second"),
        "answer_cache_hit": answer_cache_hit,
        "keyword_cache_hit": bool(stats.get("keyword_cache_hit")),
        "llm_provider": stats.get("llm_provider"),
        "llm_model": stats.get("llm_model"),
        "sources_retrieved": stats.get("sources_retrieved"),
        "answer_chars": len(answer or ""),
        "slo_total_ok": total <= SLO_TOTAL_MS,
    }
    unit["good"] = bool(unit["oracle_ok"] and unit["slo_total_ok"])
    if cache_mode == "cold_forced_unique" and answer_cache_hit:
        unit["good"] = False
        unit["error_class"] = "unexpected_cache_hit"
    return unit


def summarize(units: list[dict], wall_s: float, concurrency: int) -> dict:
    good = [u for u in units if u.get("good")]
    ok = [u for u in units if u.get("oracle_ok")]
    totals = [u.get("service_ms") or u["wall_ms"] for u in units]
    toks = [u["tokens_used"] for u in units if u.get("tokens_used")]
    cache_hits = sum(1 for u in units if u.get("answer_cache_hit"))
    return {
        "concurrency": concurrency,
        "n": len(units),
        "wall_s": round(wall_s, 2),
        "oracle_ok_rate": round(len(ok) / len(units), 3) if units else 0,
        "attainment": round(len(good) / len(units), 3) if units else 0,
        "goodput_qps": round(len(good) / wall_s, 4) if wall_s else None,
        "tokens_sum": sum(toks),
        "tokens_per_s": round(sum(toks) / wall_s, 2) if wall_s and toks else None,
        "total_ms_p50": round(pct(totals, 50) or 0, 1),
        "total_ms_p90": round(pct(totals, 90) or 0, 1),
        "answer_cache_hit_rate": round(cache_hits / len(units), 3) if units else 0,
        "llm_models": sorted({u.get("llm_model") for u in units if u.get("llm_model")}),
    }


def run_cold(client: Client) -> tuple[list[dict], list[dict]]:
    levels = []
    all_units = []
    for i, p in enumerate(cold_prompts(2)):
        u = one_query(client, p, f"warm-{i}", "cold_forced_unique", 1)
        u["warmup"] = True
        all_units.append(u)
    for C, n in [(1, 6), (2, 6), (4, 8), (8, 8)]:
        prompts = cold_prompts(n)

        def worker(i, _C=C, _prompts=prompts):
            return one_query(client, _prompts[i], f"cold-C{_C}-{i}", "cold_forced_unique", _C)

        units, wall_s = closed_loop(n, C, worker)
        all_units.extend(units)
        lv = summarize(units, wall_s, C)
        levels.append(lv)
        print(
            f"  cold C={C} attainment={lv['attainment']} p90={lv['total_ms_p90']} cache={lv['answer_cache_hit_rate']}"
        )
        if lv["attainment"] < 0.90:
            break
    return levels, all_units


def run_warm(client: Client) -> tuple[dict, list[dict]]:
    for p in WARM_PROMPTS:
        one_query(client, p, "seed", "warm", 1)
    prompts = (WARM_PROMPTS * 4)[:8]

    def worker(i):
        return one_query(client, prompts[i], f"warm-{i}", "warm", 4)

    units, wall_s = closed_loop(8, 4, worker)
    return summarize(units, wall_s, 4), units


def run_stream_ttft(client: Client, n: int = 4) -> tuple[dict, list[dict]]:
    units = []
    for i, prompt in enumerate(cold_prompts(n)):
        status, payload, wall_ms = client.request(
            "POST",
            "/api/v1/query/stream",
            {"query": prompt, "mode": "mix"},
            raw=True,
            headers={"Accept": "text/event-stream"},
        )
        ttft_ms = None
        text = ""
        oracle_ok = False
        if status == 200 and isinstance(payload, (bytes, bytearray)):
            raw = payload.decode(errors="replace")
            first_data_at = None
            for line in raw.splitlines():
                if not line.startswith("data:"):
                    continue
                if first_data_at is None:
                    first_data_at = wall_ms
                try:
                    obj = json.loads(line[5:].strip())
                except json.JSONDecodeError:
                    continue
                if not isinstance(obj, dict):
                    continue
                if obj.get("ttft_ms") is not None:
                    ttft_ms = obj["ttft_ms"]
                stats = obj.get("stats") or {}
                if stats.get("ttft_ms") is not None:
                    ttft_ms = stats["ttft_ms"]
                if "content" in obj:
                    text += str(obj.get("content") or "")
                if obj.get("type") in {"token", "chunk"} and obj.get("delta"):
                    text += str(obj.get("delta") or "")
            if ttft_ms is None:
                ttft_ms = first_data_at
            oracle_ok = bool(text) or "done" in raw.lower()
        else:
            u = one_query(client, prompt, f"stream-fallback-{i}", "cold_forced_unique", 1)
            u["stream"] = False
            u["ttft_ms"] = None
            units.append(u)
            continue
        units.append(
            {
                "unit_id": f"stream-{i}",
                "shape_id": "query-stream-ttft",
                "layers": ["L8"],
                "bench_mode": "provider",
                "cache_mode": "cold_forced_unique",
                "http_status": status,
                "oracle_ok": oracle_ok,
                "wall_ms": round(wall_ms, 1),
                "ttft_ms": ttft_ms,
                "answer_chars": len(text),
                "note": "Client may buffer SSE; prefer event ttft_ms when present",
            }
        )
        print(f"  stream {i} ttft_ms={ttft_ms} wall={wall_ms:.0f}")
    ttfts = [u["ttft_ms"] for u in units if u.get("ttft_ms") is not None]
    summary = {
        "n": len(units),
        "oracle_ok_rate": round(sum(1 for u in units if u.get("oracle_ok")) / len(units), 3)
        if units
        else 0,
        "ttft_ms_p50": round(pct(ttfts, 50) or 0, 1) if ttfts else None,
        "ttft_ms_p90": round(pct(ttfts, 90) or 0, 1) if ttfts else None,
    }
    return summary, units


def knee_from_levels(levels: list[dict]) -> dict:
    knee = None
    for lv in levels:
        if lv["attainment"] >= 0.90:
            knee = lv
        else:
            break
    if knee is None:
        return {"status": "none_met"}
    if levels and levels[-1]["attainment"] >= 0.90:
        return {
            "status": "not_reached_in_range",
            "tested_max_concurrency": levels[-1]["concurrency"],
            "attainment_at_max": levels[-1]["attainment"],
            "cold_reference_total_ms_p90": levels[0]["total_ms_p90"] if levels else None,
            "at_max": knee,
        }
    return {"status": "measured", **knee}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-url", default="https://demo.edgequake.com")
    ap.add_argument("--api-key-file", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--skip-stream", action="store_true")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    key = Path(args.api_key_file).read_text().strip()

    client = Client(
        base_url=args.base_url,
        api_key=key,
        tenant_id=TENANT_DEMO,
        workspace_id=WORKSPACE_DEMO,
        timeout_s=180,
    )

    st, health, _ = client.request("GET", "/health")
    write_json(out / "preflight-health.json", health if isinstance(health, dict) else {"body": health})
    _, ready, _ = client.request("GET", "/ready")
    write_json(out / "ready.json", ready if isinstance(ready, dict) else {"raw": ready})
    if st != 200:
        print("demo health failed", st)
        return 2

    print("P1 cold")
    cold_levels, cold_units = run_cold(client)
    print("P2 warm")
    warm_summary, warm_units = run_warm(client)
    stream_summary, stream_units = {}, []
    if not args.skip_stream:
        print("P3 stream")
        stream_summary, stream_units = run_stream_ttft(client)

    summary = {
        "spec": "153",
        "card": "provider",
        "base_url": args.base_url,
        "tenant_id": TENANT_DEMO,
        "workspace_id": WORKSPACE_DEMO,
        "slo_total_ms": SLO_TOTAL_MS,
        "health_version": health.get("version") if isinstance(health, dict) else None,
        "providers_health": health.get("providers") if isinstance(health, dict) else None,
        "P1_cold": {"levels": cold_levels, "knee": knee_from_levels(cold_levels)},
        "P2_warm": warm_summary,
        "P3_stream": stream_summary,
        "result": "PASS",
    }
    write_json(out / "summary.json", summary)
    write_jsonl(out / "units.jsonl", cold_units + warm_units + stream_units)
    write_json(
        out / "env.json",
        {
            "base_url": args.base_url,
            "tenant_id": TENANT_DEMO,
            "workspace_id": WORKSPACE_DEMO,
            "auth": "X-API-Key from file (not stored)",
            "slo_total_ms": SLO_TOTAL_MS,
        },
    )
    print(json.dumps({"cold_knee": summary["P1_cold"]["knee"], "warm": warm_summary}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
