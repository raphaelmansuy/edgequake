#!/usr/bin/env python3
"""
Challenge Query Test - Validate EdgeQuake search quality for French automotive query
Tests the query shown in the screenshot about BYD Seal U vs STLA Medium E-3008
"""

import json
import time
from typing import Any, Dict, List

import requests

BASE_URL = "http://localhost:8080"
API_V1 = f"{BASE_URL}/api/v1"


def wait_for_api(timeout=30):
    """Wait for API to be ready"""
    start = time.time()
    while time.time() - start < timeout:
        try:
            response = requests.get(f"{BASE_URL}/health", timeout=2)
            if response.status_code == 200:
                print("✓ API is ready")
                return True
        except:
            time.sleep(1)
    print("✗ API failed to start")
    return False


def get_documents() -> List[Dict]:
    """Get all documents in the system"""
    try:
        response = requests.get(f"{API_V1}/documents", timeout=30)
        response.raise_for_status()
        data = response.json()
        return data.get("documents", [])
    except Exception as e:
        print(f"✗ Failed to get documents: {e}")
        return []


def query_system(query: str, mode: str = "hybrid", top_k: int = 10) -> Dict[str, Any]:
    """Execute a query and return results"""
    try:
        payload = {"query": query, "mode": mode, "top_k": top_k}
        response = requests.post(f"{API_V1}/query", json=payload, timeout=60)
        response.raise_for_status()
        return response.json()
    except Exception as e:
        print(f"✗ Query failed: {e}")
        if hasattr(e, "response"):
            print(f"Response: {e.response.text}")
        return {}


def challenge_query():
    """Run the challenge query test"""
    print("\n" + "=" * 80)
    print("CHALLENGE QUERY TEST - EdgeQuake Search Quality Validation")
    print("=" * 80)

    # Wait for API
    if not wait_for_api():
        return

    # Check documents
    print("\n📚 Checking available documents...")
    docs = get_documents()
    print(f"   Total documents: {len(docs)}")
    if docs:
        print(f"   Sample documents:")
        for doc in docs[:5]:
            title = doc.get("title") or doc.get("filename", "Untitled")
            print(f"   - {title[:60]}")

    # Test queries
    test_queries = [
        {
            "query": "BYD Seal U battery capacity efficiency charging speed",
            "expected": "Should find BYD Seal U specifications",
            "mode": "hybrid",
        },
        {
            "query": "STLA Medium platform E-3008 Peugeot battery efficiency",
            "expected": "Should find STLA Medium / E-3008 specifications",
            "mode": "hybrid",
        },
        {
            "query": "électrique autonomie recharge voiture",  # French: electric range charging car
            "expected": "Should find EV specs in French documents",
            "mode": "hybrid",
        },
        {
            "query": "BYD vs Peugeot efficiency comparison",
            "expected": "Should find comparison data",
            "mode": "local",
        },
    ]

    print("\n" + "=" * 80)
    print("TESTING QUERIES")
    print("=" * 80)

    for i, test in enumerate(test_queries, 1):
        print(f"\n🔍 Query {i}: {test['query']}")
        print(f"   Mode: {test['mode']}")
        print(f"   Expected: {test['expected']}")

        result = query_system(test["query"], mode=test["mode"])

        if result:
            answer = result.get("answer", "No answer")
            sources = result.get("sources", [])
            topics = result.get("topics", [])

            print(f"\n   ✓ Answer ({len(answer)} chars):")
            print(f"     {answer[:200]}...")
            print(f"\n   ✓ Sources: {len(sources)}")
            for src in sources[:3]:
                print(
                    f"     - {src.get('title', 'Untitled')}: {src.get('score', 0):.3f}"
                )
            print(f"\n   ✓ Topics: {len(topics)}")
            for topic in topics[:5]:
                print(f"     - {topic}")
        else:
            print("   ✗ No result")

        time.sleep(1)

    # CHALLENGE: Test the exact query from screenshot
    print("\n" + "=" * 80)
    print("SCREENSHOT CHALLENGE QUERY (French)")
    print("=" * 80)

    french_query = """J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas. 
    Concrètement, qu'est-ce que la plateforme STLA Medium du E-3008 m'apporte de plus en termes 
    d'efficience réelle sur autoroute et de vitesse de recharge par rapport au chinois ?"""

    print(f"\nQuery (French): {french_query[:100]}...")

    for mode in ["hybrid", "global", "local"]:
        print(f"\n--- Testing mode: {mode.upper()} ---")
        result = query_system(french_query, mode=mode, top_k=10)

        if result:
            answer = result.get("answer", "")
            sources = result.get("sources", [])

            print(f"Answer quality: {len(answer)} chars")
            print(f"Sources found: {len(sources)}")

            # Check if answer is informative or generic
            if len(answer) < 100:
                print("⚠️  Very short answer - may be low quality")
            elif "ne contient pas" in answer.lower() or "no specific" in answer.lower():
                print("⚠️  Generic 'no information' response")
            else:
                print("✓ Detailed answer provided")

            # Check source relevance
            if sources:
                top_source = sources[0]
                print(
                    f"Top source: {top_source.get('title', 'Untitled')} (score: {top_source.get('score', 0):.3f})"
                )
            else:
                print("⚠️  No sources found")

        time.sleep(1)

    print("\n" + "=" * 80)
    print("CHALLENGE COMPLETE")
    print("=" * 80)


if __name__ == "__main__":
    challenge_query()
