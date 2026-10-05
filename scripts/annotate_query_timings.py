#!/usr/bin/env python3
"""Attach conservative SQL-shape observations to the source query catalog.

SQLx logs queries on completion or drop, including failed/cancelled operations.
These timings do not prove successful execution, caller coverage, or an SLO.
Only exact SQL text (ignoring outer whitespace) matches; dynamic SQL remains
unresolved. Repeated identical statements cannot identify their source caller.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import json
import math
from pathlib import Path


def query_samples(paths: list[Path]) -> dict[str, list[float]]:
    samples: dict[str, list[float]] = defaultdict(list)
    for path in paths:
        for line_number, line in enumerate(path.read_text().splitlines(), 1):
            if not line.strip():
                continue
            event = json.loads(line)
            if event.get("target") != "sqlx::query":
                continue
            fields = event["fields"]
            sql = fields.get("db.statement") or fields.get("summary")
            seconds = fields.get("elapsed_secs")
            if not isinstance(sql, str) or not sql.strip():
                raise ValueError(f"{path}:{line_number}: missing SQL text")
            if (isinstance(seconds, bool) or not isinstance(seconds, (int, float))
                    or not math.isfinite(seconds) or seconds < 0):
                raise ValueError(f"{path}:{line_number}: invalid elapsed_secs")
            samples[sql.strip()].append(seconds * 1000)
    if not samples:
        raise ValueError("query traces contain no measurements")
    return dict(samples)


def distribution(samples: list[float]) -> dict:
    ordered = sorted(samples)
    return {
        "sample_count": len(ordered),
        "p50_ms": ordered[math.ceil(len(ordered) * .50) - 1],
        "p95_ms": ordered[math.ceil(len(ordered) * .95) - 1],
        "max_ms": ordered[-1],
    }


def annotate(catalog: list[dict], samples: dict[str, list[float]]) -> dict:
    if not catalog:
        raise ValueError("empty source catalog")
    matched = set()
    rows = []
    for row in catalog:
        sql = row["site"].get("sql")
        values = samples.get(sql.strip()) if sql else None
        observation = None
        if values:
            matched.add(sql.strip())
            observation = {
                "match": "exact_sql_text",
                "function_execution_verified": False,
                "successful_execution_verified": False,
                "includes_failed_or_cancelled_queries": True,
                **distribution(values),
            }
        rows.append({**row, "matching_sql_shape_timing": observation})
    return {
        "scope": "source inventory plus SQLx shape observations; not full execution coverage",
        "source_sites": len(rows),
        "static_sites_with_matching_sql_shape": sum(
            r["matching_sql_shape_timing"] is not None for r in rows),
        "dynamic_sites_require_execution_contracts": sum(
            r["site"].get("sql") is None for r in rows),
        "observed_distinct_sql_texts": len(samples),
        "observed_query_events": sum(len(v) for v in samples.values()),
        "sites": rows,
        "unmatched_runtime_sql": [
            {"sql": sql, **distribution(values)}
            for sql, values in sorted(samples.items()) if sql not in matched
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("catalog", type=Path)
    parser.add_argument("traces", type=Path, nargs="+")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = annotate(json.loads(args.catalog.read_text()), query_samples(args.traces))
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items()
                      if k not in {"sites", "unmatched_runtime_sql"}}))


if __name__ == "__main__":
    main()
