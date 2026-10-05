"""Regression tests for benchmark gates; no database or packages required."""
from __future__ import annotations

import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import compare_eq_perf_jsonl as perf


def report(op: str = "ann", p95: float = 100.0, **extra) -> dict:
    return {"op": op, "p95_ms": p95, "pass": True, **extra}


class PerformanceGateTests(unittest.TestCase):
    def run_gate(self, baseline, candidate, *args, same_names=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = [root / "base" / "report.jsonl", root / "candidate" / "report.jsonl"] if same_names else [root / "baseline.jsonl", root / "candidate.jsonl"]
            for path, rows in zip(paths, [baseline, candidate]):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("\n".join(json.dumps(row) for row in rows))
            with patch.object(sys, "argv", ["compare", *map(str, paths), *args]), patch.dict("os.environ", {"EQ_PERF_ALLOW_OPS": ""}), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                return perf.main()

    def test_regression_fails_and_improvement_passes(self):
        self.assertEqual(self.run_gate([report()], [report(p95=201)]), 1)
        self.assertEqual(self.run_gate([report()], [report(p95=200)]), 0)
        self.assertEqual(self.run_gate([report()], [report(p95=25)]), 0)

    def test_missing_operations_fail_in_both_modes(self):
        for args in [(), ("--cross-major",)]:
            self.assertEqual(self.run_gate([report(), report("fts")], [report()], *args), 1)

    def test_empty_or_failed_reports_fail(self):
        self.assertEqual(self.run_gate([], []), 1)
        self.assertEqual(self.run_gate([report()], [report(**{"pass": False})]), 1)
        self.assertEqual(self.run_gate([report()], [report(samples_ms=[])]), 1)

    def test_invalid_numbers_fail(self):
        for value in [float("nan"), float("inf"), -1, True, None, "123"]:
            self.assertEqual(self.run_gate([report()], [report(p95=value)]), 1)

    def test_duplicate_operations_fail(self):
        self.assertEqual(self.run_gate([report()], [report(), report()]), 1)

    def test_same_filename_in_different_directories_does_not_hide_regression(self):
        self.assertEqual(self.run_gate([report()], [report(p95=300)], "--cross-major", same_names=True), 1)

    def test_allowlist_does_not_hide_bad_evidence(self):
        self.assertEqual(self.run_gate([report()], [report(p95=300)], "--allow", "ann"), 0)
        self.assertEqual(self.run_gate([report()], [report(samples_ms=[])], "--allow", "ann"), 1)

    def test_zero_baseline_is_handled(self):
        self.assertEqual(self.run_gate([report(p95=0)], [report(p95=0)]), 0)
        self.assertEqual(self.run_gate([report(p95=0)], [report(p95=1)]), 1)

    def test_sample_count_must_match(self):
        self.assertEqual(self.run_gate([report()], [report(samples_ms=[100], sample_count=2)]), 1)
        self.assertEqual(self.run_gate([report()], [report(samples_ms=[100], sample_count=1)]), 0)

    def test_cross_major_latency_regression_fails(self):
        self.assertEqual(self.run_gate([report()], [report(p95=300)], "--cross-major"), 1)

    def test_fast_or_named_operations_do_not_get_implicit_exemptions(self):
        self.assertEqual(self.run_gate([report(p95=1)], [report(p95=10)], "--cross-major"), 1)
        self.assertEqual(self.run_gate([report("stress_pool_saturation")], [report("stress_pool_saturation", p95=300)], "--cross-major"), 1)

    def test_invalid_ratio_is_configuration_error(self):
        for ratio in ["nan", "inf", "0.5"]:
            with self.assertRaises(SystemExit) as error:
                self.run_gate([report()], [report()], "--ratio", ratio)
            self.assertEqual(error.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
