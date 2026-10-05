#!/usr/bin/env python3
"""SPEC-062 — compare two /tmp/eq-perf-*.jsonl (or artifact) reports.

Usage:
  python3 scripts/compare_eq_perf_jsonl.py baseline.jsonl candidate.jsonl
  python3 scripts/compare_eq_perf_jsonl.py --cross-major \\
      specs/061-.../artifacts/eq-perf-pg16.jsonl \\
      specs/061-.../artifacts/eq-perf-pg17.jsonl \\
      specs/061-.../artifacts/eq-perf-pg18.jsonl

Exit 1 for invalid/missing evidence or a p95 regression > 2×.
Cross-major mode compares max/min; intentional workload differences require
an explicit allowlist or noise_ok in the recorded evidence.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import sys
from pathlib import Path
from typing import Any


def load_ops(path: Path) -> dict[str, dict[str, Any]]:
    out: dict[str, dict[str, Any]] = {}
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        o = json.loads(line)
        if not isinstance(o, dict) or not isinstance(o.get("op"), str) or not o["op"]:
            raise ValueError(f"{path}: report must have a nonempty operation name")
        op = o["op"]
        if op in out:
            raise ValueError(f"{path}: duplicate operation {op}")
        if o.get("pass") is False or scalar_p95(o) is None:
            raise ValueError(f"{path}: {op} failed or has an invalid p95")
        if "samples_ms" in o:
            samples = o["samples_ms"]
            if not isinstance(samples, list) or not samples or any(
                not valid_latency(sample) for sample in samples
            ):
                raise ValueError(f"{path}: {op} has missing or invalid samples")
            if "sample_count" in o and o["sample_count"] != len(samples):
                raise ValueError(f"{path}: {op} sample_count does not match samples")
        out[op] = o
    if not out:
        raise ValueError(f"{path}: no performance reports")
    return out


def valid_latency(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(value)
        and value >= 0
    )


def scalar_p95(o: dict[str, Any]) -> float | None:
    v = o.get("p95_ms")
    if valid_latency(v):
        return float(v)
    return None


def latency_ratio(baseline: float, candidate: float) -> float:
    return candidate / baseline if baseline > 0 else (1.0 if candidate == 0 else math.inf)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+", type=Path)
    ap.add_argument("--cross-major", action="store_true")
    ap.add_argument("--ratio", type=float, default=2.0)
    ap.add_argument(
        "--allow",
        action="append",
        default=[],
        help="Op name to treat as noise_ok (repeatable). Or set EQ_PERF_ALLOW_OPS=op1,op2",
    )
    args = ap.parse_args()
    if not math.isfinite(args.ratio) or args.ratio < 1:
        ap.error("--ratio must be finite and at least 1")
    if len(args.files) < 2 or (not args.cross_major and len(args.files) != 2):
        ap.error("provide two files, or at least two files with --cross-major")
    if len(set(f.resolve() for f in args.files)) != len(args.files):
        ap.error("comparison needs distinct report files")
    try:
        reports = [(f, load_ops(f)) for f in args.files]
    except (OSError, ValueError) as error:
        print(f"Invalid performance evidence: {error}", file=sys.stderr)
        return 1
    noise: set[str] = set(args.allow)
    noise.update(part.strip() for part in os.environ.get("EQ_PERF_ALLOW_OPS", "").split(",") if part.strip())

    if args.cross_major:
        by_op: dict[str, dict[str, float]] = {}
        for f, ops in reports:
            for op, o in ops.items():
                p = scalar_p95(o)
                if p is None:
                    continue
                by_op.setdefault(op, {})[str(f.resolve())] = p
                if o.get("noise_ok") or "noise_ok" in str(o.get("detail", "")):
                    noise.add(op)
        failed = False
        print("| Op | files… | max/min |")
        print("|----|--------|---------|")
        for op, vals in sorted(by_op.items()):
            if len(vals) != len(reports):
                print(f"| `{op}` | {vals} | FAIL missing operation |")
                failed = True
                continue
            mn, mx = min(vals.values()), max(vals.values())
            ratio = latency_ratio(mn, mx)
            flag = ""
            if ratio > args.ratio and op not in noise:
                flag = " FAIL"
                failed = True
            elif ratio > args.ratio:
                flag = " noise_ok"
            print(f"| `{op}` | {vals} | {ratio:.2f}{flag} |")
        return 1 if failed else 0

    a, b = reports[0][1], reports[1][1]
    ops = sorted(set(a) | set(b))
    print("| Op | baseline | candidate | delta |")
    print("|----|----------|-----------|-------|")
    failed = False
    for op in ops:
        pa, pb = scalar_p95(a.get(op, {})), scalar_p95(b.get(op, {}))
        if pa is None or pb is None:
            print(f"| `{op}` | {pa} | {pb} | FAIL missing operation |")
            failed = True
            continue
        ratio = latency_ratio(pa, pb)
        flag = ""
        if ratio > args.ratio:
            flag = " noise_ok" if op in noise else " FAIL"
            failed |= op not in noise
        delta = ((pb - pa) / pa * 100.0) if pa else 0.0
        print(f"| `{op}` | {pa:.3f} | {pb:.3f} | {delta:+.1f}%{flag} |")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
