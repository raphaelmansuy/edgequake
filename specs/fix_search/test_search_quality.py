#!/usr/bin/env python3
"""
Comprehensive Search Quality Test Suite for EdgeQuake.

This suite tests recall, precision, edge cases, and all query modes.
Run with: python3 test_search_quality.py
"""

import requests
import json
import time
from dataclasses import dataclass
from typing import Optional

BASE_URL = "http://localhost:8080"

@dataclass
class TestResult:
    name: str
    passed: bool
    details: str
    time_ms: int

class SearchQualityTester:
    """Test suite for EdgeQuake search quality."""
    
    def __init__(self):
        self.results: list[TestResult] = []
    
    def query(self, query: str, mode: str = "local", timeout: int = 30) -> dict:
        """Execute a query and return the response."""
        start = time.time()
        try:
            resp = requests.post(
                f"{BASE_URL}/api/v1/query",
                json={"query": query, "mode": mode},
                timeout=timeout
            )
            elapsed = int((time.time() - start) * 1000)
            if resp.status_code == 200:
                data = resp.json()
                data["_time_ms"] = elapsed
                return data
            return {"error": resp.text, "status": resp.status_code, "_time_ms": elapsed}
        except Exception as e:
            return {"error": str(e), "_time_ms": int((time.time() - start) * 1000)}
    
    def test_precision(self, query: str, expected_keyword: str, mode: str = "local") -> TestResult:
        """Test if expected keyword appears in first result."""
        result = self.query(query, mode)
        
        if "error" in result:
            return TestResult(f"Precision: {query[:30]}", False, f"Error: {result['error']}", result["_time_ms"])
        
        sources = result.get("sources", [])
        if not sources:
            return TestResult(f"Precision: {query[:30]}", False, "No sources returned", result["_time_ms"])
        
        first_snippet = sources[0].get("snippet", "")
        passed = expected_keyword.lower() in first_snippet.lower()
        
        return TestResult(
            f"Precision: {query[:30]}",
            passed,
            f"Expected '{expected_keyword}' in first result, got: {first_snippet[:50]}...",
            result["_time_ms"]
        )
    
    def test_recall(self, query: str, min_sources: int, mode: str = "local") -> TestResult:
        """Test if enough sources are retrieved."""
        result = self.query(query, mode)
        
        if "error" in result:
            return TestResult(f"Recall: {query[:30]}", False, f"Error: {result['error']}", result["_time_ms"])
        
        sources = result.get("sources", [])
        passed = len(sources) >= min_sources
        
        return TestResult(
            f"Recall: {query[:30]}",
            passed,
            f"Got {len(sources)} sources, expected >= {min_sources}",
            result["_time_ms"]
        )
    
    def test_answer_quality(self, query: str, expected_in_answer: str, mode: str = "local") -> TestResult:
        """Test if answer contains expected content."""
        result = self.query(query, mode)
        
        if "error" in result:
            return TestResult(f"Answer: {query[:30]}", False, f"Error: {result['error']}", result["_time_ms"])
        
        answer = result.get("answer", "")
        passed = expected_in_answer.lower() in answer.lower()
        
        return TestResult(
            f"Answer: {query[:30]}",
            passed,
            f"Expected '{expected_in_answer}' in answer: {answer[:80]}...",
            result["_time_ms"]
        )
    
    def test_mode(self, mode: str) -> TestResult:
        """Test that a query mode works."""
        result = self.query("test", mode)
        
        if "error" in result:
            return TestResult(f"Mode: {mode}", False, f"Error: {result['error']}", result["_time_ms"])
        
        return TestResult(f"Mode: {mode}", True, f"Mode works, got {len(result.get('sources', []))} sources", result["_time_ms"])
    
    def run_all_tests(self):
        """Run the complete test suite."""
        print("=" * 70)
        print("EDGEQUAKE SEARCH QUALITY TEST SUITE")
        print("=" * 70)
        
        # Test 1: API Health
        print("\n[1/6] Testing API Health...")
        try:
            resp = requests.get(f"{BASE_URL}/health", timeout=5)
            if resp.status_code == 200:
                self.results.append(TestResult("API Health", True, "Server healthy", 0))
            else:
                self.results.append(TestResult("API Health", False, f"Status {resp.status_code}", 0))
        except Exception as e:
            self.results.append(TestResult("API Health", False, str(e), 0))
            print("  ERROR: Server not responding. Aborting tests.")
            return
        
        # Test 2: Query Modes
        print("\n[2/6] Testing Query Modes...")
        for mode in ["local", "global", "hybrid", "naive"]:
            result = self.test_mode(mode)
            self.results.append(result)
            print(f"  {mode}: {'✅' if result.passed else '❌'} ({result.time_ms}ms)")
        
        # Test 3: Precision Tests
        print("\n[3/6] Testing Precision...")
        precision_tests = [
            ("Prix Peugeot 2008 ENVY", "2008"),
            ("Dimensions Peugeot 208", "208"),
            ("Équipements Peugeot 3008", "3008"),
            ("Peugeot 5008 places", "5008"),
        ]
        for query, expected in precision_tests:
            result = self.test_precision(query, expected)
            self.results.append(result)
            print(f"  {query[:40]}: {'✅' if result.passed else '❌'} ({result.time_ms}ms)")
        
        # Test 4: Recall Tests
        print("\n[4/6] Testing Recall...")
        recall_tests = [
            ("Peugeot", 4, "local"),  # Should find all 4 documents
            ("motorisation", 3, "local"),  # Multiple mentions
            ("prix", 4, "local"),  # All docs have prices
        ]
        for query, min_sources, mode in recall_tests:
            result = self.test_recall(query, min_sources, mode)
            self.results.append(result)
            print(f"  {query}: {'✅' if result.passed else '❌'} ({result.time_ms}ms)")
        
        # Test 5: Answer Quality
        print("\n[5/6] Testing Answer Quality...")
        answer_tests = [
            ("Quel est le prix du 2008 ENVY?", "32", "local"),  # Price 32,450€
            ("Combien de places dans le 5008?", "7", "local"),  # 7 places
            ("Quelle est la puissance du 3008?", "180", "local"),  # 180 ch
        ]
        for query, expected, mode in answer_tests:
            result = self.test_answer_quality(query, expected, mode)
            self.results.append(result)
            print(f"  {query[:40]}: {'✅' if result.passed else '❌'} ({result.time_ms}ms)")
        
        # Test 6: Edge Cases
        print("\n[6/6] Testing Edge Cases...")
        edge_cases = [
            ("", "local", "Empty query should fail"),
            ("x", "local", "Single char should work"),
            ("Véhicule électrique", "local", "French accents"),
        ]
        for query, mode, name in edge_cases:
            result = self.query(query, mode)
            if query == "":
                # Empty should return error
                passed = "error" in result or result.get("status") in [400, 422]
            else:
                passed = "error" not in result
            self.results.append(TestResult(f"Edge: {name}", passed, str(result.get("sources", result.get("error", "?"))), result.get("_time_ms", 0)))
            print(f"  {name}: {'✅' if passed else '❌'}")
        
        # Summary
        self.print_summary()
    
    def print_summary(self):
        """Print test summary."""
        print("\n" + "=" * 70)
        print("TEST SUMMARY")
        print("=" * 70)
        
        passed = sum(1 for r in self.results if r.passed)
        failed = sum(1 for r in self.results if not r.passed)
        total = len(self.results)
        
        print(f"\nTotal: {total} tests")
        print(f"Passed: {passed} ({passed/total*100:.0f}%)")
        print(f"Failed: {failed} ({failed/total*100:.0f}%)")
        
        if failed > 0:
            print("\nFailed Tests:")
            for r in self.results:
                if not r.passed:
                    print(f"  ❌ {r.name}")
                    print(f"     {r.details[:80]}")
        
        # Calculate average latency
        times = [r.time_ms for r in self.results if r.time_ms > 0]
        if times:
            print(f"\nAverage Latency: {sum(times)/len(times):.0f}ms")
        
        print("\n" + "=" * 70)
        if failed == 0:
            print("✅ ALL TESTS PASSED")
        else:
            print(f"❌ {failed} TESTS FAILED")
        print("=" * 70)

if __name__ == "__main__":
    tester = SearchQualityTester()
    tester.run_all_tests()
