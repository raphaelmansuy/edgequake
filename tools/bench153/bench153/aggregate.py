#!/usr/bin/env python3
"""Merge system + provider cards into one SPEC-153 summary."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def load(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text())


def layer_row(status: str, evidence: str, claim: str) -> dict:
    return {"status": status, "evidence": evidence, "safe_claim": claim}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run-dir", required=True)
    args = ap.parse_args()
    run = Path(args.run_dir)
    system = load(run / "system" / "summary.json")
    provider = load(run / "provider" / "summary.json")

    cold = (provider.get("P1_cold") or {}).get("levels") or []
    cold1 = next((lv for lv in cold if lv.get("concurrency") == 1), cold[0] if cold else {})
    warm = provider.get("P2_warm") or {}
    stream = provider.get("P3_stream") or {}
    knee = (provider.get("P1_cold") or {}).get("knee") or {}

    s2 = ((system.get("cards") or {}).get("S2_ingest_short") or {}).get("levels") or []
    s1 = (system.get("cards") or {}).get("S1_queue_only") or {}
    s4 = (system.get("cards") or {}).get("S4_embed_batch") or {}

    scorecard = {
        "L0": layer_row(
            "measured" if provider else "not_run",
            "Demo HTTP query admission",
            "API accepted authenticated Q&A traffic for the pinned tenant",
        ),
        "L1": layer_row(
            "measured" if s1 else "not_run",
            f"Admit proxy {s1.get('admits_per_s')} admits/s p50={s1.get('admit_ms_p50')}ms"
            if s1
            else "",
            "Local admit path handled burst uploads (queue physics detail in SPEC-090)",
        ),
        "L2": layer_row(
            "gap",
            "G-153-VISION / no Provider PDF ingest on shared demo",
            "No PDF convert capacity claim",
        ),
        "L3": layer_row(
            "measured" if s2 else "not_run",
            "Included inside local ingest sojourn",
            "Chunking completed as part of local mock ingest",
        ),
        "L4": layer_row(
            "measured" if s2 else "not_run",
            "Mock extract inside local ingest",
            "Local mock extraction completed; not live LLM capacity",
        ),
        "L5": layer_row(
            "estimated" if s4 else "not_run",
            f"embed_est={s4.get('embed_est')} rate={s4.get('embed_est_per_s')}/s" if s4 else "",
            "Embedding work reported as estimate (chars/2.5), not provider usage",
        ),
        "L6": layer_row(
            "measured" if s2 else "not_run",
            "Persist included in local ingest sojourn",
            "Local merge/persist finished for completed documents",
        ),
        "L7": layer_row(
            "measured" if cold1 else "not_run",
            f"Cold Q&A total_ms p50={cold1.get('total_ms_p50')}",
            "Demo retrieval participated in successful Q&A",
        ),
        "L8": layer_row(
            "measured" if cold1 else "not_run",
            f"Cold p90={cold1.get('total_ms_p90')}ms; warm cache_rate={warm.get('answer_cache_hit_rate')}; ttft_p50={stream.get('ttft_ms_p50')}",
            "Fresh demo answers typically seconds-scale; repeats may be cached",
        ),
    }

    summary = {
        "spec": "153",
        "run_id": run.name,
        "suite": "hybrid",
        "system": {
            "present": bool(system),
            "result": system.get("result"),
            "base_url": system.get("base_url"),
            "cards": list((system.get("cards") or {}).keys()),
            "detail": system.get("cards"),
        },
        "provider": {
            "present": bool(provider),
            "result": provider.get("result"),
            "base_url": provider.get("base_url"),
            "tenant_id": provider.get("tenant_id"),
            "workspace_id": provider.get("workspace_id"),
            "cold_levels": cold,
            "cold_c1": cold1,
            "warm": warm,
            "stream": stream,
            "knee": knee,
            "health_version": provider.get("health_version"),
        },
        "layer_scorecard": scorecard,
        "business": {"bottom_line": [], "allowed_claims": [], "forbidden_claims": []},
        "gaps": ["G-153-VISION", "G-153-EMBED-USAGE", "G-153-HEALTH-LLM"],
        "result": "PASS"
        if (system.get("result") in {"PASS", "PARTIAL", None} or not system)
        and provider.get("result") == "PASS"
        else "PARTIAL",
    }

    bl = summary["business"]["bottom_line"]
    if cold1:
        bl.append(
            f"On the demo tenant, fresh unique questions completed with success rate "
            f"{(cold1.get('oracle_ok_rate') or 0)*100:.0f}% and typical server time about "
            f"{cold1.get('total_ms_p50')} ms (upper ~{cold1.get('total_ms_p90')} ms)."
        )
    if warm:
        bl.append(
            f"Repeated questions under concurrency 4 showed answer-cache hit rate "
            f"{warm.get('answer_cache_hit_rate')} and typical time {warm.get('total_ms_p50')} ms."
        )
    if s2:
        best = s2[0]
        bl.append(
            f"Local mock ingest (short docs) completed at ~{best.get('docs_per_s')} docs/s "
            f"at concurrency {best.get('concurrency')} (system bench, not live LLM)."
        )
    if knee.get("status") == "not_reached_in_range":
        bl.append(
            f"No capacity knee found up to concurrency {knee.get('tested_max_concurrency')} "
            "under the 25s comfort hypothesis for cold unique prompts."
        )

    summary["business"]["allowed_claims"] = [
        "Demo Q&A succeeded for sampled unique prompts on the pinned tenant/workspace",
        "Cold and warm query modes are reported separately",
        "Local system ingest with mock LLM completed for short/dense shapes",
        "Embedding rates are labeled estimates",
    ]
    summary["business"]["forbidden_claims"] = [
        "Do not merge warm cache throughput into cold LLM capacity",
        "Do not claim Vision/PDF token capacity",
        "Do not treat health llm_provider=true as a capacity probe",
        "Do not claim shared-demo Provider ingest capacity (not run)",
    ]

    (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(
        json.dumps(
            {"result": summary["result"], "layers": {k: v["status"] for k, v in scorecard.items()}},
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
