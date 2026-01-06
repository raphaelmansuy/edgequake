#!/usr/bin/env python3
"""
Search Quality Testing Script for EdgeQuake
OODA Loop Testing Framework
"""

import json
import os
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

import requests

BASE_URL = "http://localhost:8080"
DATA_DIR = Path(__file__).parent / "data"
QUESTIONS_DIR = Path(__file__).parent / "questions"


@dataclass
class SearchResult:
    query: str
    answer: str
    entities: list
    relationships: list
    chunks: list
    mode: str
    stats: dict


def health_check() -> bool:
    """Check if the backend is healthy."""
    try:
        resp = requests.get(f"{BASE_URL}/health", timeout=5)
        return resp.status_code == 200
    except Exception as e:
        print(f"Health check failed: {e}")
        return False


def ingest_document(content: str, title: str = None) -> dict:
    """Ingest a document into edgequake."""
    payload = {
        "content": content,
        "title": title,
        "async_processing": False,  # Synchronous for testing
        "enable_gleaning": True,
        "max_gleaning": 1,
        "use_llm_summarization": True,
    }
    resp = requests.post(f"{BASE_URL}/api/v1/documents", json=payload, timeout=120)
    return resp.json()


def query_edgequake(query: str, mode: str = "hybrid") -> dict:
    """Execute a query against edgequake."""
    payload = {"query": query, "mode": mode, "context_only": False}
    resp = requests.post(f"{BASE_URL}/api/v1/query", json=payload, timeout=60)
    return resp.json()


def query_context_only(query: str, mode: str = "hybrid") -> dict:
    """Get only the context for debugging."""
    payload = {"query": query, "mode": mode, "context_only": True}
    resp = requests.post(f"{BASE_URL}/api/v1/query", json=payload, timeout=60)
    return resp.json()


def load_all_documents() -> list[tuple[str, str]]:
    """Load all documents from the data directory."""
    docs = []
    for f in DATA_DIR.glob("*.md"):
        content = f.read_text()
        title = f.stem
        docs.append((title, content))
    return docs


def load_questions() -> list[str]:
    """Load all test questions."""
    questions = []
    for f in QUESTIONS_DIR.glob("*.md"):
        content = f.read_text()
        # Parse questions from markdown
        lines = content.split("\n")
        for line in lines:
            if line.strip().startswith("_«"):
                # Extract question from markdown format
                q = line.strip().strip("_").strip("«").strip("»").strip()
                if q:
                    questions.append(q)
    return questions


def ingest_all_documents() -> dict:
    """Ingest all documents and return statistics."""
    stats = {
        "total": 0,
        "success": 0,
        "failed": 0,
        "entities_total": 0,
        "relationships_total": 0,
        "chunks_total": 0,
        "details": [],
    }

    docs = load_all_documents()
    print(f"Found {len(docs)} documents to ingest")

    for title, content in docs:
        stats["total"] += 1
        print(f"Ingesting: {title}...", end=" ", flush=True)
        try:
            result = ingest_document(content, title)
            if "error" in result:
                print(f"FAILED: {result['error']}")
                stats["failed"] += 1
                stats["details"].append({"title": title, "error": result["error"]})
            else:
                entities = result.get("entity_count", 0) or 0
                relationships = result.get("relationship_count", 0) or 0
                chunks = result.get("chunk_count", 0) or 0
                print(f"OK (E:{entities}, R:{relationships}, C:{chunks})")
                stats["success"] += 1
                stats["entities_total"] += entities
                stats["relationships_total"] += relationships
                stats["chunks_total"] += chunks
                stats["details"].append(
                    {
                        "title": title,
                        "document_id": result.get("document_id"),
                        "entities": entities,
                        "relationships": relationships,
                        "chunks": chunks,
                    }
                )
        except Exception as e:
            print(f"ERROR: {e}")
            stats["failed"] += 1
            stats["details"].append({"title": title, "error": str(e)})

    return stats


def run_search_test(query: str, modes: list = None) -> dict:
    """Run a search test with multiple modes."""
    if modes is None:
        modes = ["hybrid", "local", "global", "naive"]

    results = {}
    for mode in modes:
        try:
            result = query_edgequake(query, mode)
            results[mode] = {
                "answer": result.get("answer", ""),
                "entity_count": len(result.get("context", {}).get("entities", [])),
                "relationship_count": len(
                    result.get("context", {}).get("relationships", [])
                ),
                "chunk_count": len(result.get("context", {}).get("chunks", [])),
                "stats": result.get("stats", {}),
                "raw_result": result,
            }
        except Exception as e:
            results[mode] = {"error": str(e)}

    return results


def run_all_search_tests() -> dict:
    """Run all test questions and gather results."""
    questions = load_questions()
    print(f"Found {len(questions)} test questions")

    all_results = []
    for i, q in enumerate(questions):
        print(f"[{i+1}/{len(questions)}] Testing: {q[:60]}...")
        result = run_search_test(q)
        all_results.append({"question": q, "results": result})

    return {"total_questions": len(questions), "results": all_results}


def analyze_search_results(results: dict) -> dict:
    """Analyze search results for recall/precision issues."""
    analysis = {
        "total_queries": results["total_questions"],
        "empty_results": 0,
        "no_entities": 0,
        "no_relationships": 0,
        "no_chunks": 0,
        "mode_comparison": {},
    }

    modes = ["hybrid", "local", "global", "naive"]
    for mode in modes:
        analysis["mode_comparison"][mode] = {
            "avg_entities": 0,
            "avg_relationships": 0,
            "avg_chunks": 0,
            "empty_count": 0,
        }

    for r in results["results"]:
        for mode in modes:
            mode_result = r["results"].get(mode, {})
            if "error" in mode_result:
                continue

            entities = mode_result.get("entity_count", 0)
            rels = mode_result.get("relationship_count", 0)
            chunks = mode_result.get("chunk_count", 0)

            analysis["mode_comparison"][mode]["avg_entities"] += entities
            analysis["mode_comparison"][mode]["avg_relationships"] += rels
            analysis["mode_comparison"][mode]["avg_chunks"] += chunks

            if entities == 0 and rels == 0 and chunks == 0:
                analysis["mode_comparison"][mode]["empty_count"] += 1

    # Calculate averages
    n = results["total_questions"]
    for mode in modes:
        if n > 0:
            analysis["mode_comparison"][mode]["avg_entities"] /= n
            analysis["mode_comparison"][mode]["avg_relationships"] /= n
            analysis["mode_comparison"][mode]["avg_chunks"] /= n

    return analysis


def main():
    """Main entry point."""
    print("=" * 60)
    print("EdgeQuake Search Quality Testing")
    print("=" * 60)

    # Check health
    if not health_check():
        print("ERROR: Backend is not healthy!")
        return
    print("✓ Backend is healthy")

    # Step 1: Ingest documents
    print("\n--- STEP 1: Ingesting Documents ---")
    ingest_stats = ingest_all_documents()
    print(f"\nIngestion Summary:")
    print(f"  Total: {ingest_stats['total']}")
    print(f"  Success: {ingest_stats['success']}")
    print(f"  Failed: {ingest_stats['failed']}")
    print(f"  Entities: {ingest_stats['entities_total']}")
    print(f"  Relationships: {ingest_stats['relationships_total']}")
    print(f"  Chunks: {ingest_stats['chunks_total']}")

    # Save ingestion stats
    with open(
        Path(__file__).parent / "ooda_loop" / "iteration_01" / "ingestion_stats.json",
        "w",
    ) as f:
        json.dump(ingest_stats, f, indent=2)

    # Step 2: Run search tests
    print("\n--- STEP 2: Running Search Tests ---")
    search_results = run_all_search_tests()

    # Save search results
    with open(
        Path(__file__).parent / "ooda_loop" / "iteration_01" / "search_results.json",
        "w",
    ) as f:
        json.dump(search_results, f, indent=2, ensure_ascii=False)

    # Step 3: Analyze results
    print("\n--- STEP 3: Analyzing Results ---")
    analysis = analyze_search_results(search_results)

    # Save analysis
    with open(
        Path(__file__).parent / "ooda_loop" / "iteration_01" / "analysis.json", "w"
    ) as f:
        json.dump(analysis, f, indent=2)

    print("\nAnalysis Summary:")
    print(json.dumps(analysis, indent=2))

    print("\n✓ All results saved to ooda_loop/iteration_01/")


if __name__ == "__main__":
    main()
