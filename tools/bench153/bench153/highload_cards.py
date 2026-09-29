"""SPEC-153 §07 workload cards H1–H4. Each returns (levels_or_summary, units)."""

from __future__ import annotations

import time

from bench153 import Client, closed_loop, closed_loop_timed, open_loop
from bench153.query_unit import (
    FAQ_PROMPTS, cold_prompt, cold_prompts, failing, hard_abort, one_query, summarize,
)

COLD = "cold_forced_unique"


def _stamp(lv: dict, t0: float) -> dict:
    lv["t_start"], lv["t_end"] = round(t0, 2), round(time.time(), 2)
    return lv


def _log(tag: str, lv: dict) -> None:
    print(f"  {tag} attainment={lv['attainment']} p50={lv['total_ms_p50']} p90={lv['total_ms_p90']} "
          f"err={lv['error_rate']} 429={lv['http_429_count']} cache={lv['answer_cache_hit_rate']}", flush=True)


def _should_stop(lv: dict, explore: bool, fails_seen: int, max_fails: int) -> bool:
    if hard_abort(lv):
        return True
    if failing(lv):
        return (not explore) or fails_seen >= max_fails
    return False


def run_h1(client: Client, levels_c: list[int], explore: bool, max_fails: int = 3):
    """Fresh questions, fixed number of simultaneous askers (closed loop)."""
    levels, units, fails = [], [], 0
    for i, p in enumerate(cold_prompts(2)):
        u = one_query(client, p, f"h1-warm-{i}", COLD, 1, "H1")
        u["warmup"] = True
        units.append(u)
    for C in levels_c:
        n = min(48, max(16, 2 * C))
        prompts = cold_prompts(n)
        t0 = time.time()
        us, wall = closed_loop(n, C, lambda i, _C=C, _p=prompts: one_query(client, _p[i], f"h1-C{_C}-{i}", COLD, _C, "H1"))
        units += us
        lv = _stamp(summarize(us, wall, C), t0)
        levels.append(lv)
        _log(f"H1 C={C}", lv)
        fails += failing(lv)
        if _should_stop(lv, explore, fails, max_fails):
            print(f"  H1 stop at C={C}", flush=True)
            break
    return levels, units


def run_h2(client: Client, rates: list[float], duration_s: float, explore: bool, max_fails: int = 2):
    """Fresh questions arriving at a steady rate whether or not earlier ones finished."""
    levels, units, fails = [], [], 0
    for lam in rates:
        def worker(i, _lam=lam):
            return one_query(client, cold_prompt(i), f"h2-l{_lam}-{i}", COLD, 0, "H2")
        t0 = time.time()
        us, wall, offered = open_loop(lam, duration_s, worker, max_inflight=64)
        units += us
        lv = _stamp(summarize(us, wall, None, lambda_qps=lam, offered_qps=lam,
                              offered_count=offered, completed_count=len(us)), t0)
        levels.append(lv)
        _log(f"H2 λ={lam}/s offered={offered}", lv)
        fails += failing(lv)
        if _should_stop(lv, explore, fails, max_fails):
            print(f"  H2 stop at λ={lam}", flush=True)
            break
    return levels, units


def run_h3(client: Client, levels_c: list[int], n_per: int):
    """Same three FAQ questions asked repeatedly (answer cache should absorb them)."""
    for p in FAQ_PROMPTS:
        one_query(client, p, "h3-seed", "warm", 1, "H3")
    levels, units = [], []
    for C in levels_c:
        prompts = (FAQ_PROMPTS * (n_per // len(FAQ_PROMPTS) + 1))[:n_per]
        t0 = time.time()
        us, wall = closed_loop(n_per, C, lambda i, _C=C, _p=prompts: one_query(client, _p[i], f"h3-C{_C}-{i}", "warm", _C, "H3"))
        units += us
        lv = _stamp(summarize(us, wall, C), t0)
        levels.append(lv)
        _log(f"H3 C={C}", lv)
    return levels, units


def run_h4_soak(client: Client, concurrency: int, duration_s: float):
    """Hold the comfortable load for minutes to see if latency drifts (stability)."""
    t0 = time.time()
    us, wall = closed_loop_timed(
        concurrency, duration_s,
        lambda i: one_query(client, cold_prompt(i), f"h4-{i}", COLD, concurrency, "H4"),
    )
    lv = _stamp(summarize(us, wall, concurrency, duration_s=duration_s), t0)
    _log(f"H4 soak C={concurrency} {duration_s:.0f}s n={lv['n']}", lv)
    return lv, us
