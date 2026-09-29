"""H5 — sustained-capacity ladder.

Shared-core VMs (e.g. GCE e2-medium) can burst above their sustained CPU share for ~2 minutes, so a
short sweep over-states capacity. H5 holds each load for longer than the burst window, after an idle
rest that lets the allowance refill, and judges only the *steady* part (after the burst window).
"""

from __future__ import annotations

import time

from bench153 import Client, closed_loop_timed, open_loop
from bench153.query_unit import cold_prompt, failing, one_query, summarize

COLD = "cold_forced_unique"
BURST_WINDOW_S = 120  # GCP documents ~120 s of full-rate burst for e2-medium at 100% CPU

# (kind, value): ordered so consecutive steps differ in mode and intensity.
DEFAULT_PLAN = [("closed", 8), ("open", 0.5), ("closed", 5), ("open", 0.35), ("open", 0.25)]


def _label(kind: str, value: float) -> str:
    return f"C={value:g}" if kind == "closed" else f"λ={value:g}/s"


def _run_one(client: Client, kind: str, value: float, soak_s: float) -> tuple[list[dict], float]:
    if kind == "closed":
        return closed_loop_timed(
            int(value), soak_s,
            lambda i: one_query(client, cold_prompt(i), f"h5-C{value:g}-{i}", COLD, int(value), "H5"),
        )
    units, wall, _ = open_loop(
        value, soak_s,
        lambda i: one_query(client, cold_prompt(i), f"h5-l{value:g}-{i}", COLD, 0, "H5"),
        max_inflight=64,
    )
    return units, wall


def _steady(units: list[dict], t_start: float) -> list[dict]:
    return [u for u in units if u["started_at"] - t_start >= BURST_WINDOW_S]


def run_ladder(client: Client, plan: list[tuple[str, float]], soak_s: float, rest_s: float):
    """Return (levels, units). Each level carries whole-window and steady-state summaries."""
    levels, all_units = [], []
    for kind, value in plan:
        label = _label(kind, value)
        print(f"  H5 rest {rest_s:.0f}s before {label}", flush=True)
        time.sleep(rest_s)
        t0 = time.time()
        units, wall = _run_one(client, kind, value, soak_s)
        all_units += units
        steady = _steady(units, t0)
        whole = summarize(units, wall, int(value) if kind == "closed" else None)
        st = summarize(steady, max(soak_s - BURST_WINDOW_S, 1), int(value) if kind == "closed" else None) if steady else None
        lv = {"kind": kind, "value": value, "label": label, "soak_s": soak_s, "rest_s": rest_s,
              "t_start": round(t0, 2), "t_end": round(time.time(), 2), "whole": whole, "steady": st,
              "sustainable": bool(st and not failing(st))}
        levels.append(lv)
        print(f"  H5 {label}: whole {whole['attainment']} | steady "
              f"{st['attainment'] if st else 'n/a'} p50={st['total_ms_p50'] if st else 'n/a'} "
              f"-> {'SUSTAINABLE' if lv['sustainable'] else 'not sustainable'}", flush=True)
    return levels, all_units


def parse_plan(text: str) -> list[tuple[str, float]]:
    """'closed:10,open:0.6' -> [('closed', 10.0), ('open', 0.6)] (closed capped at 32, open at 0.6/s)."""
    plan = []
    for part in text.split(","):
        kind, _, val = part.strip().partition(":")
        if kind not in ("closed", "open") or not val:
            raise ValueError(f"bad sustain step {part!r}; use closed:N or open:RATE")
        v = float(val)
        if (kind == "closed" and not 1 <= v <= 32) or (kind == "open" and not 0 < v <= 0.6):
            raise ValueError(f"{part!r} exceeds the safety caps (C<=32, lambda<=0.6/s)")
        plan.append((kind, v))
    return plan
