#!/usr/bin/env python3
"""Test edge cases for EdgeQuake search."""

import requests
import json
import time

BASE_URL = "http://localhost:8080"

def test_query(query: str, mode: str = "local", name: str = "") -> dict:
    """Run a test query."""
    start = time.time()
    try:
        resp = requests.post(
            f"{BASE_URL}/api/v1/query",
            json={"query": query, "mode": mode},
            timeout=30
        )
        elapsed = time.time() - start
        
        if resp.status_code == 200:
            data = resp.json()
            return {
                "name": name,
                "status": "OK",
                "sources": len(data.get("sources", [])),
                "answer_len": len(data.get("answer", "")),
                "time_ms": int(elapsed * 1000)
            }
        else:
            return {
                "name": name,
                "status": "ERROR",
                "code": resp.status_code,
                "message": resp.text[:100]
            }
    except Exception as e:
        return {
            "name": name,
            "status": "EXCEPTION",
            "error": str(e)[:100]
        }

def main():
    print("=" * 60)
    print("EDGE CASE TESTS")
    print("=" * 60)
    
    tests = [
        # Basic tests
        ("Prix 2008", "local", "Basic French query"),
        ("What is the price?", "local", "English query"),
        
        # Edge cases
        ("", "local", "Empty query"),
        ("x", "local", "Single character"),
        ("?", "local", "Just punctuation"),
        ("123", "local", "Just numbers"),
        ("Peugeot " * 10, "local", "Repeated word"),
        
        # Mode tests
        ("Prix", "global", "Global mode"),
        ("Prix", "hybrid", "Hybrid mode"),
        ("Prix", "naive", "Naive mode"),
        
        # Unicode
        ("Véhicule électrique", "local", "French accents"),
        ("日本語", "local", "Japanese characters"),
        ("🚗 voiture", "local", "Emoji in query"),
    ]
    
    results = []
    for query, mode, name in tests:
        print(f"\nTesting: {name}...")
        result = test_query(query, mode, name)
        results.append(result)
        print(f"  Result: {result['status']}, sources={result.get('sources', 'N/A')}")
    
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    
    passed = sum(1 for r in results if r["status"] == "OK")
    errors = sum(1 for r in results if r["status"] == "ERROR")
    exceptions = sum(1 for r in results if r["status"] == "EXCEPTION")
    
    print(f"Passed: {passed}/{len(results)}")
    print(f"Errors: {errors}/{len(results)}")
    print(f"Exceptions: {exceptions}/{len(results)}")
    
    # Show failures
    for r in results:
        if r["status"] != "OK":
            print(f"\n  FAIL: {r['name']}")
            print(f"    {r}")

if __name__ == "__main__":
    main()
