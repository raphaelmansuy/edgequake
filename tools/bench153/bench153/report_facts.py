"""Derive plain-English facts + verdicts from measured data (no hard-coded results)."""

from __future__ import annotations

from typing import Any

from bench153.charts_data import peak_stats
from bench153.query_unit import SLO_FLOOR


def _med(xs: list[float]) -> float:
    ys = sorted(xs)
    return ys[len(ys) // 2] if ys else 0.0


def _lv(summary, key):
    return (summary.get(key) or {}).get("levels") or []


def _knee_level(levels, knee, field):
    val = (knee or {}).get(field)
    return next((l for l in levels if l.get(field) == val), None)


def soak_facts(summary, units) -> dict[str, Any] | None:
    h4 = summary.get("H4_soak")
    rows = sorted((u["started_at"], (u.get("service_ms") or u["wall_ms"]) / 1000)
                  for u in units if u.get("card") == "H4")
    if not h4 or len(rows) < 6:
        return None
    third = len(rows) // 3
    first, last = _med([r[1] for r in rows[:third]]), _med([r[1] for r in rows[-third:]])
    drift = (last / first - 1) * 100 if first else 0
    stable = drift <= 15 and h4["attainment"] >= SLO_FLOOR
    return {"first": first, "last": last, "drift_pct": drift, "stable": stable, **h4}


def time_split(level: dict | None) -> dict[str, float] | None:
    """Seconds per answer spent in the external-LLM steps vs the lookup on our VM."""
    if not level or level.get("generation_ms_mean") is None:
        return None
    llm = sum(level.get(k) or 0 for k in ("keyword_ms_mean", "embedding_ms_mean", "generation_ms_mean")) / 1000
    vm = (level.get("retrieval_ms_mean") or 0) / 1000
    tot = llm + vm
    return {"llm_s": llm, "vm_s": vm, "vm_share": 100 * vm / tot if tot else 0, "llm_share": 100 * llm / tot if tot else 0}


def bottleneck(h1: list[dict]) -> dict[str, Any]:
    """Which side grows with load? Compare the lowest vs the highest tested crowd."""
    a, b = time_split(h1[0] if h1 else None), time_split(h1[-1] if h1 else None)
    if not a or not b:
        return {"who": "unknown"}
    vm_growth, llm_growth = b["vm_s"] - a["vm_s"], b["llm_s"] - a["llm_s"]
    if vm_growth > 2 * max(llm_growth, 0.5):
        who = "vm"
    elif llm_growth > 2 * max(vm_growth, 0.5):
        who = "llm"
    else:
        who = "mixed"
    return {"who": who, "vm_low": a["vm_s"], "vm_high": b["vm_s"], "llm_low": a["llm_s"], "llm_high": b["llm_s"],
            "low_C": h1[0]["concurrency"], "high_C": h1[-1]["concurrency"]}


def server_verdict(peaks: dict[str, float], telemetry: list[dict], bn: dict) -> tuple[str, str]:
    """(css class, sentence) on whether the VM limited the test."""
    if not peaks or not telemetry:
        return "info", "Server telemetry could not be collected, so we cannot say whether the VM or the LLM set the limit."
    frac = 100 * sum(1 for s in telemetry if s["cpu_busy_pct"] >= 90) / len(telemetry)
    if frac >= 10 or bn.get("who") == "vm":
        return "warn", (f"The VM was the bottleneck: its CPU sat at 90% or more for {frac:.0f}% of the measured time (peak {peaks['cpu_busy_max']:.0f}%), "
                        f"and the “look up” step running on the VM grew from {bn.get('vm_low', 0):.1f} s to {bn.get('vm_high', 0):.1f} s per answer "
                        f"as the crowd grew, while the external LLM steps barely moved ({bn.get('llm_low', 0):.1f} s → {bn.get('llm_high', 0):.1f} s).")
    if peaks["cpu_busy_max"] < 60:
        return "", f"The VM was never the bottleneck: CPU peaked at {peaks['cpu_busy_max']:.0f}%. Waiting on the external LLM set the limit."
    return "info", f"The VM was moderately loaded (CPU peak {peaks['cpu_busy_max']:.0f}%); neither side clearly saturated."


def sustain_facts(sustain: dict | None) -> dict[str, Any] | None:
    if not sustain or not sustain.get("levels"):
        return None
    lv = sustain["levels"]
    closed = [l for l in lv if l["kind"] == "closed"]
    opened = [l for l in lv if l["kind"] == "open"]

    def best(ls):
        ok = [l["value"] for l in ls if l["sustainable"]]
        return max(ok) if ok else None

    return {"levels": lv, "best_closed": best(closed), "best_open": best(opened),
            "min_closed_tested": min((l["value"] for l in closed), default=None),
            "min_open_tested": min((l["value"] for l in opened), default=None),
            "soak_s": sustain["soak_s"], "rest_s": sustain["rest_s"]}


def gather(summary, units, telemetry, machines, sustain=None) -> dict[str, Any]:
    h1, h2, h3 = _lv(summary, "H1_cold_closed"), _lv(summary, "H2_open_loop"), _lv(summary, "H3_faq_warm")
    k1, k2 = summary["H1_cold_closed"].get("knee") or {}, summary["H2_open_loop"].get("knee") or {}
    kl1, kl2 = _knee_level(h1, k1, "concurrency"), _knee_level(h2, k2, "lambda_qps")
    not_reached = k2.get("status") == "not_reached_in_range"
    if not_reached:  # every tested rate passed: the top tested rate is the best we can honestly report
        kl2 = k2.get("at_max")
        k2 = {**k2, "lambda_qps": k2.get("tested_max_lambda_qps")}
    peaks = peak_stats(telemetry)
    bn = bottleneck(h1)
    best = max(h1, key=lambda l: l["goodput_qps"] or 0) if h1 else None
    vm = ((machines.get("sut") or {}).get("gcp") or {}).get("vm") or {}
    return {
        "h1": h1, "h2": h2, "h3": h3, "k1": k1, "k2": k2, "kl1": kl1, "kl2": kl2,
        "kC": k1.get("concurrency"), "kLam": k2.get("lambda_qps"),
        "h2_not_reached": not_reached,
        "h2_max": h2[-1]["lambda_qps"] if h2 else None,
        "fail_C": (k1.get("first_failing") or {}).get("concurrency"),
        "fail_lam": (k2.get("first_failing") or {}).get("lambda_qps"),
        "max_C": h1[-1]["concurrency"] if h1 else None,
        "best_goodput": best, "soak": soak_facts(summary, units), "peaks": peaks, "bottleneck": bn,
        "server_verdict": server_verdict(peaks, telemetry, bn),
        "split_knee": time_split(kl1), "split_low": time_split(h1[0] if h1 else None),
        "split_high": time_split(h1[-1] if h1 else None), "sustain": sustain_facts(sustain),
        "vm_type": vm.get("machine_type", "GCP VM"), "shared_core": bool(vm.get("shared_core")),
        "n_units": len([u for u in units if not u.get("warmup")]),
        "n_errors": sum(1 for u in units if (u.get("http_status") or 0) >= 400),
    }
