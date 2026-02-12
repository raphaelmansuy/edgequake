#!/usr/bin/env python3
"""
Document Upload — EdgeQuake Python SDK

WHY: Documents are the primary input to EdgeQuake. This example shows
text upload, PDF upload, batch upload, and async status tracking.

Requirements:
    - EdgeQuake server running on http://localhost:8080
    - EDGEQUAKE_API_KEY environment variable set

Usage:
    export EDGEQUAKE_API_KEY="demo-key"
    python examples/document_upload.py
"""
import os
import time
from pathlib import Path

from edgequake import EdgequakeClient


def main():
    client = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "demo-key"),
        base_url=os.environ.get("EDGEQUAKE_URL", "http://localhost:8080"),
    )

    # ── 1. Upload plain text ──────────────────────────────────

    text_doc = client.documents.upload(
        content=(
            "Knowledge graphs represent information as nodes and edges, "
            "enabling structured reasoning over unstructured data."
        ),
        title="Knowledge Graphs Introduction",
        metadata={"category": "research", "author": "EdgeQuake Team"},
    )
    print(f"Text document uploaded: {text_doc['document_id']}")

    # ── 2. Upload a PDF file ──────────────────────────────────

    # WHY: PDF upload uses multipart/form-data under the hood.
    # The SDK handles file reading and conversion automatically.
    sample_pdf = Path("sample.pdf")
    if sample_pdf.exists():
        with open(sample_pdf, "rb") as f:
            pdf_doc = client.documents.pdf.upload(
                file=f.read(),
                title="Sample PDF",
            )
        print(f"PDF uploaded: {pdf_doc['document_id']}")
    else:
        print("(Skipping PDF — no sample.pdf found in current directory)")

    # ── 3. Track processing status ────────────────────────────

    # WHY: Document processing (chunking, entity extraction) is async.
    # Poll the track endpoint to monitor progress.
    track_id = text_doc.get("track_id")
    if track_id:
        attempts = 0
        while attempts < 30:
            status = client.documents.get_track_status(track_id)
            print(f"Processing: {status['status']} — {status.get('message', '')}")
            if status["status"] in ("completed", "failed"):
                break
            time.sleep(2)
            attempts += 1

    # ── 4. List all documents (paginated) ─────────────────────

    # WHY: The SDK handles cursor-based pagination automatically.
    print("\nAll documents:")
    documents = client.documents.list()
    for doc in documents.get("items", []):
        print(f"  {doc['id']}: {doc['title']} ({doc['status']})")

    # ── 5. Get document details ───────────────────────────────

    detail = client.documents.get(text_doc["document_id"])
    print(f"\nDocument detail: {detail['title']}")
    print(f"  Chunks: {detail.get('chunk_count', 0)}")
    print(f"  Status: {detail['status']}")

    # ── 6. Delete a document ──────────────────────────────────

    client.documents.delete(text_doc["document_id"])
    print(f"\nDeleted document {text_doc['document_id']}")


if __name__ == "__main__":
    main()
