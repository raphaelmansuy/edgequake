#!/usr/bin/env python3
"""
Streaming Query — EdgeQuake Python SDK

WHY: Streaming delivers tokens incrementally via Server-Sent Events (SSE),
enabling real-time UI updates without waiting for the full response.

Requirements:
    - EdgeQuake server running on http://localhost:8080
    - EDGEQUAKE_API_KEY environment variable set
    - Documents uploaded and indexed

Usage:
    export EDGEQUAKE_API_KEY="demo-key"
    python examples/streaming_query.py
"""
import os
import sys

from edgequake import EdgequakeClient


def main():
    client = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "demo-key"),
        base_url=os.environ.get("EDGEQUAKE_URL", "http://localhost:8080"),
    )

    # ── 1. Streaming query via SSE ────────────────────────────

    # WHY: `client.query.stream()` returns an iterator that yields
    # parsed JSON chunks as they arrive from the server.
    print("Streaming query response:")
    try:
        for event in client.query.stream(
            query="Explain how knowledge graphs enhance RAG systems",
            mode="hybrid",
        ):
            # WHY: Each event is a parsed object from the SSE data line.
            # The exact shape depends on the backend's streaming format.
            if isinstance(event, dict) and "chunk" in event:
                sys.stdout.write(event["chunk"])
                sys.stdout.flush()
            elif isinstance(event, str):
                sys.stdout.write(event)
                sys.stdout.flush()
    except Exception as e:
        print(f"\n[Streaming error: {e}]")
    print("\n")

    # ── 2. Streaming chat (OpenAI-compatible) ─────────────────

    # WHY: Chat streaming follows the OpenAI delta format, making it
    # compatible with existing OpenAI-based UIs and libraries.
    print("Streaming chat response:")
    try:
        for chunk in client.chat.stream(
            model="edgequake",
            messages=[
                {
                    "role": "user",
                    "content": "What are the benefits of graph-based RAG?",
                },
            ],
        ):
            if isinstance(chunk, dict):
                delta = chunk.get("choices", [{}])[0].get("delta", {}).get("content")
                if delta:
                    sys.stdout.write(delta)
                    sys.stdout.flush()
    except Exception as e:
        print(f"\n[Streaming error: {e}]")
    print("\n")


if __name__ == "__main__":
    main()
