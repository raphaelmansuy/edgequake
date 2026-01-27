#!/usr/bin/env python3
"""
Ingest all test documents from specs/fix_search/data/ into EdgeQuake PostgreSQL backend.
"""

import os
import requests
import time
import glob

BASE_URL = "http://localhost:8080"
DATA_DIR = "/Users/raphaelmansuy/Github/03-working/edgequake/specs/fix_search/data"

def check_health():
    """Check if server is healthy."""
    try:
        resp = requests.get(f"{BASE_URL}/health", timeout=5)
        data = resp.json()
        print(f"Server: {data.get('status')} ({data.get('storage_mode')})")
        return data.get("status") == "healthy"
    except Exception as e:
        print(f"Server error: {e}")
        return False

def ingest_document(filepath: str) -> bool:
    """Ingest a single document."""
    filename = os.path.basename(filepath)
    
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()
    
    try:
        resp = requests.post(
            f"{BASE_URL}/api/v1/documents",
            json={
                "title": filename,
                "content": content
            },
            timeout=120
        )
        
        if resp.status_code in [200, 201]:
            data = resp.json()
            doc_id = data.get("document_id", data.get("id", "unknown"))
            print(f"  ✅ {filename} -> {doc_id[:8]}...")
            return True
        else:
            print(f"  ❌ {filename}: {resp.status_code} - {resp.text[:100]}")
            return False
    except Exception as e:
        print(f"  ❌ {filename}: {e}")
        return False

def main():
    print("=" * 60)
    print("EDGEQUAKE POSTGRESQL DATA INGESTION")
    print("=" * 60)
    
    if not check_health():
        print("Server not healthy, aborting.")
        return
    
    # Find all markdown files
    files = glob.glob(os.path.join(DATA_DIR, "*.md"))
    print(f"\nFound {len(files)} documents to ingest:")
    
    success = 0
    failed = 0
    
    for filepath in sorted(files):
        result = ingest_document(filepath)
        if result:
            success += 1
        else:
            failed += 1
        time.sleep(1)  # Rate limiting
    
    print("\n" + "=" * 60)
    print(f"INGESTION COMPLETE: {success} success, {failed} failed")
    print("=" * 60)

if __name__ == "__main__":
    main()
