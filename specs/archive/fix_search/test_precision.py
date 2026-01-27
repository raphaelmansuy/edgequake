#!/usr/bin/env python3
"""Test precision: Does searching for '2008' return 2008 chunks first, not 208/3008/5008?"""

import requests
import json

BASE_URL = "http://localhost:8080"

def test_precision(query: str, expected_model: str):
    """Test if the expected model appears first in results."""
    print(f"\n{'='*60}")
    print(f"QUERY: {query}")
    print(f"EXPECTED: {expected_model} should rank highest")
    print(f"{'='*60}")
    
    # Test Local mode
    resp = requests.post(
        f"{BASE_URL}/api/v1/query",
        json={"query": query, "mode": "local"},
        timeout=60
    )
    
    if resp.status_code != 200:
        print(f"ERROR: {resp.status_code} - {resp.text[:200]}")
        return False
    
    data = resp.json()
    
    # Print sources
    sources = data.get("sources", [])
    print(f"\nSOURCES ({len(sources)} total):")
    
    correct_count = 0
    wrong_count = 0
    
    for i, src in enumerate(sources[:10]):  # Top 10
        # Get content from snippet or other fields
        snippet = src.get("snippet", src.get("content", ""))[:150].replace("\n", " ")
        score = src.get("score", 0)
        rerank_score = src.get("rerank_score", score)
        
        # Check if this source is for the expected model
        is_correct = expected_model in snippet
        status = "✅" if is_correct else "❌"
        
        if is_correct:
            correct_count += 1
        else:
            wrong_count += 1
        
        print(f"  {i+1}. {status} score={score:.3f} rerank={rerank_score:.3f}")
        print(f"      {snippet}...")
    
    # Calculate precision
    precision = correct_count / max(1, correct_count + wrong_count)
    print(f"\nPRECISION: {correct_count}/{correct_count + wrong_count} = {precision:.1%}")
    
    # Check if first result is correct
    first_snippet = sources[0].get("snippet", "") if sources else ""
    first_correct = expected_model in first_snippet
    print(f"FIRST RESULT CORRECT: {first_correct}")
    
    return first_correct

def main():
    print("="*60)
    print("PRECISION TEST - Model Name Discrimination")
    print("="*60)
    
    tests = [
        ("Prix du Peugeot 2008 ENVY", "2008"),
        ("Dimensions de la Peugeot 208", "208"),
        ("Équipements du Peugeot 3008 GT", "3008"),
        ("Peugeot 5008 7 places", "5008"),
    ]
    
    passed = 0
    failed = 0
    
    for query, expected in tests:
        if test_precision(query, expected):
            passed += 1
        else:
            failed += 1
    
    print(f"\n{'='*60}")
    print(f"SUMMARY: {passed}/{passed+failed} tests passed")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
