#!/usr/bin/env python3
"""
Basic Usage — EdgeQuake Python SDK

WHY: Demonstrates the simplest possible setup — create a client,
check health, and run a basic query. Start here if you're new.

Requirements:
    - EdgeQuake server running on http://localhost:8080
    - EDGEQUAKE_API_KEY environment variable set

Usage:
    export EDGEQUAKE_API_KEY="demo-key"
    python examples/basic_usage.py
"""
import os
from edgequake import EdgequakeClient


def main():
    # WHY: base_url points to your EdgeQuake backend; api_key authenticates.
    client = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "demo-key"),
        base_url=os.environ.get("EDGEQUAKE_URL", "http://localhost:8080"),
    )

    # 1. Health check — verify the API is reachable
    health = client.health()
    print(f"Health: {health}")

    # 2. Upload a simple text document
    doc = client.documents.upload(
        content=(
            "EdgeQuake is a graph-based RAG framework written in Rust. "
            "It uses knowledge graphs to enhance retrieval-augmented generation."
        ),
        title="EdgeQuake Overview",
    )
    print(f"Uploaded document: {doc['document_id']}")

    # 3. Query the knowledge base
    result = client.query.execute(
        query="What is EdgeQuake?",
        mode="hybrid",
    )
    print(f"Answer: {result['answer']}")

    # 4. Explore the graph
    graph = client.graph.get()
    print(f"Graph stats: {graph}")


if __name__ == "__main__":
    main()
