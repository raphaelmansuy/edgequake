"""One question against /api/v1/query: prompts, unit record, level summary, knee logic."""

from __future__ import annotations

import time
import uuid

from bench153 import Client, pct

SLO_TOTAL_MS = 25000
SLO_FLOOR = 0.90

FAQ_PROMPTS = [
    "What is text-to-SQL evaluation?",
    "Summarize agentic AI challenges from the uploaded papers.",
    "What does the sol_pi paper discuss?",
]

_TEMPLATES = [
    "According to the uploaded papers, what pitfalls arise when evaluating text-to-SQL in production? Unique={u}-{i}",
    "From the agentic PDF corpus, name two practical constraints on agent evaluation. Unique={u}-{i}",
    "What does sol_pi contribute that differs from the text-to-SQL article? Unique={u}-{i}",
    "How do the papers separate retrieval quality from end-task correctness? Unique={u}-{i}",
    "List themes about real-world text-to-SQL systems from the corpus. Unique={u}-{i}",
    "What open evaluation problems for agents appear in the documents? Unique={u}-{i}",
]


def cold_prompt(i: int, stamp: str | None = None) -> str:
    """A never-repeated question (unique token defeats the answer cache)."""
    return _TEMPLATES[i % len(_TEMPLATES)].format(u=stamp or uuid.uuid4().hex[:10], i=i)


def cold_prompts(n: int) -> list[str]:
    stamp = uuid.uuid4().hex[:10]
    return [cold_prompt(i, stamp) for i in range(n)]


def _cache_hit(stats: dict, gen, oracle_ok: bool) -> bool:
    if "answer_cache_hit" in stats:
        return bool(stats["answer_cache_hit"])
    return bool(gen == 0 and oracle_ok)


def one_query(client: Client, prompt: str, idx: str, cache_mode: str, concurrency: int, card: str) -> dict:
    started = time.time()
    status, payload, wall_ms = client.request(
        "POST", "/api/v1/query", {"query": prompt, "mode": "mix", "stream": False}
    )
    stats = ((payload or {}).get("stats") if isinstance(payload, dict) else None) or {}
    answer = (payload or {}).get("answer") if isinstance(payload, dict) else ""
    oracle_ok = status == 200 and bool((answer or "").strip())
    total = stats.get("total_time_ms") or wall_ms
    gen = stats.get("generation_time_ms")
    hit = _cache_hit(stats, gen, oracle_ok)
    unit = {
        "unit_id": idx, "card": card, "shape_id": "query-chat-v1", "layers": ["L0", "L7", "L8"],
        "bench_mode": "provider_highload", "cache_mode": cache_mode, "concurrency": concurrency,
        "prompt": prompt[:240], "http_status": status, "oracle_ok": oracle_ok,
        "started_at": round(started, 2), "wall_ms": round(wall_ms, 1), "sojourn_ms": round(wall_ms, 1),
        "service_ms": stats.get("total_time_ms"),
        "embedding_time_ms": stats.get("embedding_time_ms"),
        "keyword_time_ms": stats.get("keyword_time_ms"),
        "retrieval_time_ms": stats.get("retrieval_time_ms"),
        "generation_time_ms": gen, "tokens_used": stats.get("tokens_used"),
        "tokens_per_second": stats.get("tokens_per_second"),
        "answer_cache_hit": hit, "keyword_cache_hit": bool(stats.get("keyword_cache_hit")),
        "llm_provider": stats.get("llm_provider"), "llm_model": stats.get("llm_model"),
        "sources_retrieved": stats.get("sources_retrieved"), "answer_chars": len(answer or ""),
        "slo_total_ok": total <= SLO_TOTAL_MS, "error_429": status == 429,
    }
    unit["good"] = bool(oracle_ok and unit["slo_total_ok"])
    if cache_mode == "cold_forced_unique" and hit:
        unit["good"], unit["error_class"] = False, "unexpected_cache_hit"
    if status >= 400:
        unit["error_class"] = f"http_{status}"
    return unit


def _mean(units: list[dict], key: str) -> float | None:
    xs = [u[key] for u in units if u.get(key) is not None]
    return round(sum(xs) / len(xs), 1) if xs else None


def summarize(units: list[dict], wall_s: float, concurrency: int | None = None, **extra) -> dict:
    good = [u for u in units if u.get("good")]
    ok = [u for u in units if u.get("oracle_ok")]
    totals = [u.get("service_ms") or u["wall_ms"] for u in units]
    walls = [u["wall_ms"] for u in units]
    toks = [u["tokens_used"] for u in units if u.get("tokens_used")]
    n = len(units)
    out = {
        "concurrency": concurrency, "n": n, "wall_s": round(wall_s, 2),
        "oracle_ok_rate": round(len(ok) / n, 3) if n else 0,
        "attainment": round(len(good) / n, 3) if n else 0,
        "goodput_qps": round(len(good) / wall_s, 4) if wall_s else None,
        "error_rate": round(sum(1 for u in units if (u.get("http_status") or 0) >= 400) / n, 3) if n else 0,
        "http_429_count": sum(1 for u in units if u.get("error_429")),
        "tokens_sum": sum(toks), "tokens_per_s": round(sum(toks) / wall_s, 2) if wall_s and toks else None,
        "total_ms_p50": round(pct(totals, 50) or 0, 1), "total_ms_p90": round(pct(totals, 90) or 0, 1),
        "total_ms_max": round(max(totals), 1) if totals else None,
        "wall_ms_p50": round(pct(walls, 50) or 0, 1), "wall_ms_p90": round(pct(walls, 90) or 0, 1),
        "keyword_ms_mean": _mean(units, "keyword_time_ms"),
        "embedding_ms_mean": _mean(units, "embedding_time_ms"),
        "retrieval_ms_mean": _mean(units, "retrieval_time_ms"),
        "generation_ms_mean": _mean(units, "generation_time_ms"),
        "answer_cache_hit_rate": round(sum(1 for u in units if u.get("answer_cache_hit")) / n, 3) if n else 0,
        "llm_models": sorted({u.get("llm_model") for u in units if u.get("llm_model")}),
    }
    out.update(extra)
    return out


def failing(lv: dict) -> bool:
    """Level misses the conjunction SLO (attainment floor, error rate, 429 storm)."""
    return lv.get("attainment", 0) < SLO_FLOOR or lv.get("error_rate", 0) > 0.05 or lv.get("http_429_count", 0) >= 3


def hard_abort(lv: dict) -> bool:
    """Safety stop even in explore-beyond-knee mode (protect the shared demo)."""
    return lv.get("error_rate", 0) > 0.25 or lv.get("http_429_count", 0) >= 10 or lv.get("attainment", 1) < 0.10


def knee_from_levels(levels: list[dict], key: str = "concurrency") -> dict:
    """Knee = last level before the first failing one (levels ascend in load)."""
    knee = None
    for lv in levels:
        if failing(lv):
            break
        knee = lv
    if knee is None:
        return {"status": "none_met"}
    if all(not failing(lv) for lv in levels):
        return {"status": "not_reached_in_range", f"tested_max_{key}": levels[-1].get(key),
                "attainment_at_max": levels[-1]["attainment"], "at_max": knee}
    first_fail = next(lv for lv in levels if failing(lv))
    return {"status": "measured", "first_failing": {key: first_fail.get(key),
            "attainment": first_fail["attainment"]}, **knee}
