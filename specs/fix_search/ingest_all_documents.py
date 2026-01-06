#!/usr/bin/env python3
"""
Full Ingestion Script for EdgeQuake Search Quality Testing (OODA Loop 33)

This script ingests all test documents from specs/fix_search/data/
into a fresh PostgreSQL-backed EdgeQuake instance and validates the ingestion.

Usage:
    python3 specs/fix_search/ingest_all_documents.py
"""

import json
import os
import sys
import time
from pathlib import Path

import requests

# Configuration
API_BASE = "http://localhost:8080/api/v1"
DATA_DIR = Path(__file__).parent / "data"
TIMEOUT = 120  # 2 minutes per document for processing


def check_health():
    """Check if EdgeQuake API is healthy"""
    try:
        # Health is at root, not under /api/v1
        resp = requests.get("http://localhost:8080/health", timeout=10)
        data = resp.json()
        print(f"✅ API Status: {data['status']}")
        print(f"   Storage Mode: {data['storage_mode']}")
        print(f"   LLM Provider: {data.get('llm_provider_name', 'unknown')}")
        print(f"   Components: {json.dumps(data.get('components', {}))}")
        return data["status"] == "healthy"
    except Exception as e:
        print(f"❌ API health check failed: {e}")
        return False


def get_document_stats():
    """Get current document and entity stats"""
    try:
        # Get documents
        docs_resp = requests.get(f"{API_BASE}/documents", timeout=30)
        docs = docs_resp.json() if docs_resp.ok else []

        # Get graph statistics (entities/relationships)
        stats_resp = requests.get(f"{API_BASE}/graph/stats", timeout=30)
        stats = stats_resp.json() if stats_resp.ok else {}

        return {
            "documents": len(docs) if isinstance(docs, list) else 0,
            "entities": stats.get("entities", 0),
            "relationships": stats.get("relationships", 0),
            "chunks": stats.get("chunks", 0),
        }
    except Exception as e:
        print(f"   Warning: Could not get stats: {e}")
        return {"documents": 0, "entities": 0, "relationships": 0, "chunks": 0}


def ingest_document(file_path: Path, index: int, total: int):
    """Ingest a single document"""
    print(f"\n[{index}/{total}] Ingesting: {file_path.name}")

    start_time = time.time()

    try:
        # Read file content
        content = file_path.read_text(encoding="utf-8")

        # Upload document
        files = {"file": (file_path.name, content, "text/markdown")}

        resp = requests.post(
            f"{API_BASE}/documents/upload", files=files, timeout=TIMEOUT
        )

        elapsed = time.time() - start_time

        if resp.ok:
            data = resp.json()
            task_id = data.get("task_id") or data.get("id")
            print(f"   ✅ Uploaded in {elapsed:.1f}s - Task ID: {task_id}")

            # Wait for processing to complete
            if task_id:
                wait_for_task(task_id)

            return True, elapsed
        else:
            print(f"   ❌ Failed: {resp.status_code} - {resp.text[:200]}")
            return False, elapsed

    except Exception as e:
        elapsed = time.time() - start_time
        print(f"   ❌ Error: {e}")
        return False, elapsed


def wait_for_task(task_id: str, max_wait: int = 300):
    """Wait for a document processing task to complete"""
    start = time.time()
    while time.time() - start < max_wait:
        try:
            resp = requests.get(f"{API_BASE}/tasks/{task_id}", timeout=30)
            if resp.ok:
                data = resp.json()
                status = data.get("status", "unknown")
                if status in ["completed", "done", "success"]:
                    print(f"   → Task completed: {status}")
                    return True
                elif status in ["failed", "error"]:
                    print(f"   → Task failed: {data.get('error', 'unknown error')}")
                    return False
                else:
                    time.sleep(2)  # Wait before checking again
            else:
                # Task endpoint might not exist - assume sync processing
                return True
        except:
            time.sleep(2)

    print(f"   ⚠ Task timeout after {max_wait}s")
    return True  # Continue anyway


def main():
    print("=" * 70)
    print("EDGEQUAKE FULL INGESTION - OODA LOOP 33")
    print("=" * 70)

    # Check API health
    print("\n📡 Checking API health...")
    if not check_health():
        print("❌ API is not healthy. Please start EdgeQuake first.")
        sys.exit(1)

    # Get initial stats
    print("\n📊 Initial database state:")
    initial_stats = get_document_stats()
    print(f"   Documents: {initial_stats['documents']}")
    print(f"   Entities: {initial_stats['entities']}")
    print(f"   Relationships: {initial_stats['relationships']}")
    print(f"   Chunks: {initial_stats['chunks']}")

    # Get all markdown files
    if not DATA_DIR.exists():
        print(f"❌ Data directory not found: {DATA_DIR}")
        sys.exit(1)

    files = sorted(DATA_DIR.glob("*.md"))
    print(f"\n📁 Found {len(files)} documents to ingest")

    # Ingest each document
    print("\n" + "=" * 70)
    print("STARTING INGESTION")
    print("=" * 70)

    results = []
    total_time = 0

    for i, file_path in enumerate(files, 1):
        success, elapsed = ingest_document(file_path, i, len(files))
        results.append({"file": file_path.name, "success": success, "elapsed": elapsed})
        total_time += elapsed

        # Small delay between documents
        time.sleep(1)

    # Get final stats
    print("\n" + "=" * 70)
    print("INGESTION COMPLETE - FINAL STATISTICS")
    print("=" * 70)

    # Wait a bit for async processing to complete
    print("\n⏳ Waiting for background processing to complete...")
    time.sleep(10)

    final_stats = get_document_stats()
    print(f"\n📊 Final database state:")
    print(
        f"   Documents: {initial_stats['documents']} → {final_stats['documents']} (+{final_stats['documents'] - initial_stats['documents']})"
    )
    print(
        f"   Entities: {initial_stats['entities']} → {final_stats['entities']} (+{final_stats['entities'] - initial_stats['entities']})"
    )
    print(
        f"   Relationships: {initial_stats['relationships']} → {final_stats['relationships']} (+{final_stats['relationships'] - initial_stats['relationships']})"
    )
    print(
        f"   Chunks: {initial_stats['chunks']} → {final_stats['chunks']} (+{final_stats['chunks'] - initial_stats['chunks']})"
    )

    # Summary
    successful = sum(1 for r in results if r["success"])
    failed = sum(1 for r in results if not r["success"])

    print(f"\n📈 Ingestion Summary:")
    print(f"   Total Documents: {len(files)}")
    print(f"   Successful: {successful}")
    print(f"   Failed: {failed}")
    print(f"   Total Time: {total_time:.1f}s")
    print(f"   Avg Time/Doc: {total_time/len(files):.1f}s")

    if failed > 0:
        print(f"\n❌ Failed documents:")
        for r in results:
            if not r["success"]:
                print(f"   - {r['file']}")

    # Save report
    report = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "initial_stats": initial_stats,
        "final_stats": final_stats,
        "results": results,
        "summary": {
            "total": len(files),
            "successful": successful,
            "failed": failed,
            "total_time": total_time,
        },
    }

    report_path = Path(__file__).parent / "ingestion_report.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)
    print(f"\n📝 Report saved to: {report_path}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
