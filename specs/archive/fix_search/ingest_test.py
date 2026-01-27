#!/usr/bin/env python3
"""Quick document ingestion for precision testing."""

import requests
import os
import sys

BASE_URL = "http://localhost:8080"
TEST_DATA = "/Users/raphaelmansuy/Github/03-working/edgequake/specs/fix_search/test_data"

# Key files for precision testing - we want to test if "2008" returns 2008 and not 208/3008
DOCS = [
    ("peugeot-2008-envy.md", "Fiche Peugeot 2008 ENVY"),
    ("peugeot-208.md", "Fiche Peugeot 208"),
    ("peugeot-3008.md", "Fiche Peugeot 3008"),
    ("peugeot-5008.md", "Fiche Peugeot 5008"),
]

def ingest_doc(filename: str, title: str):
    """Ingest a document."""
    filepath = os.path.join(TEST_DATA, filename)
    if not os.path.exists(filepath):
        print(f"SKIP: {filename} not found")
        return None
    
    with open(filepath, "r") as f:
        content = f.read()[:20000]  # Truncate for speed
    
    resp = requests.post(
        f"{BASE_URL}/api/v1/documents",
        json={"title": title, "content": content},
        timeout=60
    )
    
    if resp.status_code == 200:
        doc_id = resp.json().get("id", "?")
        print(f"OK: {title} -> {doc_id}")
        return doc_id
    else:
        print(f"FAIL: {title} -> {resp.status_code}: {resp.text[:100]}")
        return None

def main():
    print("Ingesting documents for precision testing...")
    
    doc_ids = []
    for filename, title in DOCS:
        doc_id = ingest_doc(filename, title)
        if doc_id:
            doc_ids.append((title, doc_id))
    
    print(f"\nIngested {len(doc_ids)} documents")
    return 0

if __name__ == "__main__":
    sys.exit(main())
