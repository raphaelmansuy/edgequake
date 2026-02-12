#!/usr/bin/env python3
"""
Query Demo — EdgeQuake Python SDK

WHY: Queries are how you retrieve knowledge from EdgeQuake.
This example demonstrates simple, hybrid, and parametric queries.

Requirements:
    - EdgeQuake server running on http://localhost:8080
    - EDGEQUAKE_API_KEY environment variable set
    - Documents uploaded and indexed

Usage:
    export EDGEQUAKE_API_KEY="demo-key"
    python examples/query_demo.py
"""
import os

from edgequake import EdgequakeClient


def main():
    client = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "demo-key"),
        base_url=os.environ.get("EDGEQUAKE_URL", "http://localhost:8080"),
    )

    # ── 1. Simple query ───────────────────────────────────────

    # WHY: Default mode uses the backend's configured retrieval strategy.
    simple = client.query.execute(query="What is retrieval-augmented generation?")
    print(f"Simple query answer: {simple['answer']}")

    # ── 2. Hybrid mode query ──────────────────────────────────

    # WHY: Hybrid mode combines local (entity-centric) and global
    # (community-level) retrieval for comprehensive answers.
    hybrid = client.query.execute(
        query="How do knowledge graphs improve RAG?",
        mode="hybrid",
        top_k=10,
    )
    print(f"\nHybrid query answer: {hybrid['answer']}")

    # ── 3. Chat completion (OpenAI-compatible) ────────────────

    # WHY: Chat endpoint lets you use EdgeQuake as a drop-in replacement
    # for OpenAI's chat API, with RAG context automatically injected.
    chat = client.chat.completions(
        model="edgequake",
        messages=[
            {
                "role": "system",
                "content": "You are a helpful assistant powered by EdgeQuake.",
            },
            {
                "role": "user",
                "content": "What entities are in the knowledge graph?",
            },
        ],
    )
    message_content = chat.get("choices", [{}])[0].get("message", {}).get("content")
    print(f"\nChat response: {message_content}")


if __name__ == "__main__":
    main()
