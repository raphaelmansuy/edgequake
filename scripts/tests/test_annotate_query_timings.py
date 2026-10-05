"""Query evidence must not overstate execution or source-call-site coverage."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import annotate_query_timings as timings


class QueryTimingTests(unittest.TestCase):
    def test_exact_shape_matches_do_not_claim_caller_or_success(self):
        catalog = [{"site": {"sql": sql}} for sql in [" SELECT 1 ", "SELECT 1", None, "SELECT  1"]]
        result = timings.annotate(catalog, {"SELECT 1": [1, 2, 3]})
        self.assertEqual(result["static_sites_with_matching_sql_shape"], 2)
        self.assertEqual(result["dynamic_sites_require_execution_contracts"], 1)
        evidence = result["sites"][0]["matching_sql_shape_timing"]
        self.assertFalse(evidence["function_execution_verified"])
        self.assertFalse(evidence["successful_execution_verified"])
        self.assertEqual(evidence["p95_ms"], 3)
        self.assertIsNone(result["sites"][3]["matching_sql_shape_timing"])

    def test_short_queries_use_summary_and_elapsed_is_milliseconds(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.jsonl"
            path.write_text(json.dumps({"target": "sqlx::query", "fields": {
                "db.statement": "", "summary": "SELECT 1", "elapsed_secs": .01}}))
            self.assertEqual(timings.query_samples([path]), {"SELECT 1": [10]})

    def test_empty_and_invalid_measurements_fail(self):
        with self.assertRaises(ValueError):
            timings.annotate([], {})
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.jsonl"
            path.write_text("")
            with self.assertRaises(ValueError):
                timings.query_samples([path])
            for value in [None, True, -1, float("nan"), float("inf"), "0.1"]:
                path.write_text(json.dumps({"target": "sqlx::query", "fields": {
                    "summary": "SELECT 1", "elapsed_secs": value}}))
                with self.subTest(value=value), self.assertRaises(ValueError):
                    timings.query_samples([path])


if __name__ == "__main__":
    unittest.main()
