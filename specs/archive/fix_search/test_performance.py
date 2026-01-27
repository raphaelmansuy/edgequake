#!/usr/bin/env python3
"""Performance test for EdgeQuake search."""

import requests
import time
import statistics

BASE_URL = "http://localhost:8080"

def time_query(query: str, mode: str = "local") -> tuple:
    """Time a query and return (success, time_ms, answer_len, sources)."""
    start = time.time()
    try:
        resp = requests.post(
            f"{BASE_URL}/api/v1/query",
            json={"query": query, "mode": mode},
            timeout=60
        )
        elapsed_ms = (time.time() - start) * 1000
        
        if resp.status_code == 200:
            data = resp.json()
            return (True, elapsed_ms, len(data.get("answer", "")), len(data.get("sources", [])))
        return (False, elapsed_ms, 0, 0)
    except Exception as e:
        return (False, (time.time() - start) * 1000, 0, 0)

def run_benchmark(name: str, query: str, mode: str, runs: int = 5):
    """Run a benchmark test."""
    print(f"\n{name}:")
    print(f"  Query: '{query[:30]}...' Mode: {mode}")
    
    times = []
    for i in range(runs):
        success, ms, answer_len, sources = time_query(query, mode)
        if success:
            times.append(ms)
            print(f"  Run {i+1}: {ms:.0f}ms (answer={answer_len} chars, sources={sources})")
        else:
            print(f"  Run {i+1}: FAILED")
    
    if times:
        print(f"  Summary: min={min(times):.0f}ms, max={max(times):.0f}ms, avg={statistics.mean(times):.0f}ms")
    return times

def main():
    print("=" * 60)
    print("PERFORMANCE BENCHMARK")
    print("=" * 60)
    
    # Get stats from API response
    resp = requests.post(
        f"{BASE_URL}/api/v1/query",
        json={"query": "Prix 2008", "mode": "local"},
        timeout=30
    )
    if resp.status_code == 200:
        data = resp.json()
        stats = data.get("stats", {})
        print("\nAPI Stats from sample query:")
        print(f"  Embedding time: {stats.get('embedding_time_ms', 0)}ms")
        print(f"  Retrieval time: {stats.get('retrieval_time_ms', 0)}ms")
        print(f"  Generation time: {stats.get('generation_time_ms', 0)}ms")
        print(f"  Total time: {stats.get('total_time_ms', 0)}ms")
    
    # Run benchmarks
    all_times = []
    
    # Simple queries
    times = run_benchmark("Simple Query (local)", "Prix 2008", "local", 3)
    all_times.extend(times)
    
    times = run_benchmark("Simple Query (hybrid)", "Prix 2008", "hybrid", 3)
    all_times.extend(times)
    
    times = run_benchmark("Simple Query (naive)", "Prix 2008", "naive", 3)
    all_times.extend(times)
    
    # Complex queries
    times = run_benchmark("Complex Query", "Comparez les prix et dimensions de tous les modèles Peugeot", "hybrid", 3)
    all_times.extend(times)
    
    # Summary
    print("\n" + "=" * 60)
    print("OVERALL SUMMARY")
    print("=" * 60)
    if all_times:
        print(f"Total runs: {len(all_times)}")
        print(f"Min latency: {min(all_times):.0f}ms")
        print(f"Max latency: {max(all_times):.0f}ms")
        print(f"Avg latency: {statistics.mean(all_times):.0f}ms")
        print(f"Median latency: {statistics.median(all_times):.0f}ms")
        
        # Categorize
        under_1s = sum(1 for t in all_times if t < 1000)
        under_2s = sum(1 for t in all_times if t < 2000)
        under_3s = sum(1 for t in all_times if t < 3000)
        print(f"\nLatency distribution:")
        print(f"  <1s: {under_1s}/{len(all_times)} ({under_1s/len(all_times)*100:.0f}%)")
        print(f"  <2s: {under_2s}/{len(all_times)} ({under_2s/len(all_times)*100:.0f}%)")
        print(f"  <3s: {under_3s}/{len(all_times)} ({under_3s/len(all_times)*100:.0f}%)")

if __name__ == "__main__":
    main()
