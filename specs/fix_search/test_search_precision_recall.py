#!/usr/bin/env python3
"""
OODA Loop 35-36: Search Quality Precision/Recall Test
Tests EdgeQuake search with real queries against ingested car specification documents.
"""
import json
import time
from dataclasses import dataclass
from typing import Optional

import requests

API_BASE = "http://localhost:8080/api/v1"


@dataclass
class TestQuery:
    """Test query with expected entities and documents."""

    query: str
    expected_entities: list[str]  # Entities that SHOULD appear in results
    expected_documents: list[str]  # Document names that SHOULD appear
    category: str  # Category for grouping


# Define test queries with expected results based on the ingested data
TEST_QUERIES = [
    # =====================================================================
    # THEMATIC 1: Electrification and Autonomy
    # =====================================================================
    TestQuery(
        query="Quelles sont les caractéristiques de la Peugeot 2008?",
        expected_entities=["PEUGEOT 2008", "Peugeot", "VisioPark"],
        expected_documents=["EF-extract-2008.md"],
        category="Peugeot 2008",
    ),
    TestQuery(
        query="What is the battery technology in BYD Seal?",
        expected_entities=["BYD SEAL U DM-i", "BYD", "LFP Battery"],
        expected_documents=["EF-Extract-BYD-Seal.md"],
        category="BYD",
    ),
    TestQuery(
        query="Renault 5 électrique caractéristiques",
        expected_entities=["Renault 5", "Renault", "E-Tech"],
        expected_documents=["EF-extract-RENAULT 5-e-tech.md"],
        category="Renault 5",
    ),
    # =====================================================================
    # THEMATIC 2: Technology and Connectivity
    # =====================================================================
    TestQuery(
        query="i-Cockpit Peugeot 308 fonctionnalités",
        expected_entities=["PEUGEOT 308", "i-Cockpit", "i-Connect"],
        expected_documents=["EF-extract-new-308.md"],
        category="Peugeot 308",
    ),
    TestQuery(
        query="Google integration Renault vehicles",
        expected_entities=["Google", "Renault", "openR link"],
        expected_documents=[
            "EF-extract-Renault-CAPTUR.md",
            "EF-Extract-Renault-Autral.md",
        ],
        category="Renault Technology",
    ),
    TestQuery(
        query="Apple CarPlay Android Auto compatibility",
        expected_entities=["Apple CarPlay", "Android Auto"],
        expected_documents=["EF-Extract-BYD-Seal.md", "EF-extract-Renault-CAPTUR.md"],
        category="Connectivity",
    ),
    # =====================================================================
    # THEMATIC 3: Hybrid Motorization
    # =====================================================================
    TestQuery(
        query="E-Tech hybrid Renault motorisation",
        expected_entities=["E-Tech", "Renault", "Renault Captur"],
        expected_documents=[
            "EF-extract-Renault-CAPTUR.md",
            "EF-Extract-Renault-Autral.md",
        ],
        category="Renault Hybrid",
    ),
    TestQuery(
        query="Peugeot hybrid rechargeable 308",
        expected_entities=["PEUGEOT 308", "HYBRIDE RECHARGEABLE"],
        expected_documents=["EF-extract-new-308.md"],
        category="Peugeot Hybrid",
    ),
    # =====================================================================
    # THEMATIC 4: Safety Features
    # =====================================================================
    TestQuery(
        query="Aides à la conduite safety features",
        expected_entities=["Aides à la conduite", "Freinage automatique d'urgence"],
        expected_documents=["EF-extract-Renault-CAPTUR.md"],
        category="Safety",
    ),
    # =====================================================================
    # THEMATIC 5: Specific Models
    # =====================================================================
    TestQuery(
        query="BYD HAN specifications equipment",
        expected_entities=["BYD HAN", "BYD"],
        expected_documents=["EF-extract-BYD HAN.md"],
        category="BYD HAN",
    ),
    TestQuery(
        query="Peugeot 3008 trim levels GT Allure",
        expected_entities=["PEUGEOT 3008", "Allure"],
        expected_documents=["EF-extract-3008.md", "EF-extract-CT_3008.md"],
        category="Peugeot 3008",
    ),
    TestQuery(
        query="Renault Scenic E-Tech électrique",
        expected_entities=["RENAULT SCENIC E-TECH", "Renault"],
        expected_documents=["EF-Extract-Renault-Scenic.md"],
        category="Renault Scenic",
    ),
    TestQuery(
        query="Peugeot Traveller Expert specifications",
        expected_entities=["PEUGEOT E-Traveller", "Peugeot"],
        expected_documents=["EF-Extract-Peugeot-Traveller.md"],
        category="Peugeot Traveller",
    ),
    # =====================================================================
    # CROSS-BRAND COMPARISONS
    # =====================================================================
    TestQuery(
        query="Compare BYD vs Peugeot electric vehicles",
        expected_entities=["BYD", "Peugeot", "ÉLECTRIQUE"],
        expected_documents=["EF-Extract-BYD-Seal.md", "EF-extract-new-308.md"],
        category="Comparison",
    ),
    TestQuery(
        query="SUV électrique français",
        expected_entities=["Renault", "Peugeot"],
        expected_documents=["EF-extract-Renault-CAPTUR.md", "EF-extract-3008.md"],
        category="French EVs",
    ),
]


def check_api_health() -> bool:
    """Check API is healthy."""
    try:
        # Just test if the server responds to any request
        resp = requests.get("http://localhost:8080/", timeout=5)
        return True  # If we get any response, server is up
    except:
        return False


def execute_search(query: str, top_k: int = 10) -> dict:
    """Execute search query against EdgeQuake API."""
    try:
        resp = requests.post(
            f"{API_BASE}/query", json={"query": query, "top_k": top_k}, timeout=60
        )
        if resp.status_code == 200:
            return resp.json()
        else:
            return {"error": f"HTTP {resp.status_code}", "response": resp.text}
    except Exception as e:
        return {"error": str(e)}


def calculate_precision_recall(
    test: TestQuery, response: dict
) -> tuple[float, float, float, list[str], list[str]]:
    """
    Calculate precision and recall for a single query.

    Precision = (relevant entities found) / (all entities returned)
    Recall = (relevant entities found) / (expected entities)
    """
    # Extract entities from response
    found_entities = set()
    found_documents = set()

    # Check sources for entities (EdgeQuake returns them in sources with source_type="entity")
    if "sources" in response:
        for source in response.get("sources", []):
            if isinstance(source, dict):
                # Check for entity type sources
                if source.get("source_type") == "entity":
                    entity_id = source.get("id", "")
                    if entity_id:
                        found_entities.add(entity_id.upper())
                # Check for document sources
                doc_name = source.get("document_name", "") or source.get(
                    "file_name", ""
                )
                if doc_name:
                    found_documents.add(doc_name)
            else:
                found_documents.add(str(source))

    # Also check chunks
    if "chunks" in response:
        for chunk in response.get("chunks", []):
            if isinstance(chunk, dict):
                doc_name = chunk.get("document_name", "") or chunk.get("file_name", "")
                if doc_name:
                    found_documents.add(doc_name)

    # Check answer text for entity mentions
    answer = response.get("answer", "")

    # Calculate entity metrics
    expected_set = {e.upper() for e in test.expected_entities}

    # Count found in response OR mentioned in answer
    relevant_found = 0
    found_list = []
    for expected in test.expected_entities:
        if expected.upper() in found_entities or expected.lower() in answer.lower():
            relevant_found += 1
            found_list.append(expected)

    # Calculate metrics
    precision = relevant_found / len(found_entities) if found_entities else 0.0
    recall = relevant_found / len(expected_set) if expected_set else 0.0
    f1 = (
        2 * precision * recall / (precision + recall)
        if (precision + recall) > 0
        else 0.0
    )

    missing = [e for e in test.expected_entities if e not in found_list]

    return precision, recall, f1, found_list, missing


def run_search_quality_tests():
    """Run all search quality tests and generate report."""
    print("=" * 80)
    print("EDGEQUAKE SEARCH QUALITY TEST - OODA LOOPS 35-36")
    print("=" * 80)
    print()

    if not check_api_health():
        print("❌ ERROR: API is not healthy!")
        return

    print("✅ API is healthy\n")

    results = []
    total_precision = 0.0
    total_recall = 0.0
    total_f1 = 0.0
    total_time = 0.0

    for i, test in enumerate(TEST_QUERIES, 1):
        print(f"[{i:2d}/{len(TEST_QUERIES)}] Testing: {test.category}")
        print(f"         Query: {test.query[:60]}...")

        start = time.time()
        response = execute_search(test.query)
        elapsed = time.time() - start
        total_time += elapsed

        if "error" in response:
            print(f"         ❌ Error: {response['error']}")
            results.append(
                {
                    "query": test.query,
                    "category": test.category,
                    "error": response["error"],
                    "precision": 0.0,
                    "recall": 0.0,
                    "f1": 0.0,
                    "time_ms": elapsed * 1000,
                }
            )
            continue

        precision, recall, f1, found, missing = calculate_precision_recall(
            test, response
        )

        total_precision += precision
        total_recall += recall
        total_f1 += f1

        # Status indicator
        if recall >= 0.8:
            status = "✅"
        elif recall >= 0.5:
            status = "⚠️"
        else:
            status = "❌"

        print(
            f"         {status} Recall: {recall:.1%} | Precision: {precision:.1%} | F1: {f1:.1%} | {elapsed*1000:.0f}ms"
        )
        if found:
            print(f"         Found: {', '.join(found[:5])}")
        if missing:
            print(f"         Missing: {', '.join(missing[:5])}")

        # Check if answer exists and is substantive
        answer = response.get("answer", "")
        answer_quality = len(answer) > 100
        if not answer_quality:
            print(f"         ⚠️  Answer too short ({len(answer)} chars)")

        results.append(
            {
                "query": test.query,
                "category": test.category,
                "expected_entities": test.expected_entities,
                "found_entities": found,
                "missing_entities": missing,
                "precision": precision,
                "recall": recall,
                "f1": f1,
                "time_ms": elapsed * 1000,
                "answer_length": len(answer),
                "has_answer": answer_quality,
            }
        )
        print()

    # Summary
    n = len(TEST_QUERIES)
    avg_precision = total_precision / n
    avg_recall = total_recall / n
    avg_f1 = total_f1 / n
    avg_time = total_time / n

    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"Total Queries:     {n}")
    print(f"Average Precision: {avg_precision:.1%}")
    print(f"Average Recall:    {avg_recall:.1%}")
    print(f"Average F1 Score:  {avg_f1:.1%}")
    print(f"Average Response:  {avg_time*1000:.0f}ms")
    print()

    # By category
    print("BY CATEGORY:")
    categories = {}
    for r in results:
        cat = r["category"]
        if cat not in categories:
            categories[cat] = {"precision": [], "recall": [], "f1": []}
        categories[cat]["precision"].append(r["precision"])
        categories[cat]["recall"].append(r["recall"])
        categories[cat]["f1"].append(r["f1"])

    for cat, metrics in sorted(categories.items()):
        avg_r = sum(metrics["recall"]) / len(metrics["recall"])
        avg_p = sum(metrics["precision"]) / len(metrics["precision"])
        print(f"  {cat:25s} Recall: {avg_r:.1%} | Precision: {avg_p:.1%}")

    # Save report
    report = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "ooda_loop": "35-36",
        "summary": {
            "total_queries": n,
            "avg_precision": avg_precision,
            "avg_recall": avg_recall,
            "avg_f1": avg_f1,
            "avg_response_ms": avg_time * 1000,
        },
        "by_category": {
            cat: {
                "avg_precision": sum(m["precision"]) / len(m["precision"]),
                "avg_recall": sum(m["recall"]) / len(m["recall"]),
                "avg_f1": sum(m["f1"]) / len(m["f1"]),
            }
            for cat, m in categories.items()
        },
        "results": results,
    }

    report_path = "/Users/raphaelmansuy/Github/03-working/edgequake/specs/fix_search/search_quality_report.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\n📝 Report saved to: {report_path}")

    # Pass/Fail criteria
    print("\n" + "=" * 80)
    if avg_recall >= 0.6:
        print("✅ PASS: Average recall ≥ 60%")
    else:
        print("❌ FAIL: Average recall < 60%")

    if avg_precision >= 0.4:
        print("✅ PASS: Average precision ≥ 40%")
    else:
        print("❌ FAIL: Average precision < 40%")

    print("=" * 80)

    return report


if __name__ == "__main__":
    run_search_quality_tests()
