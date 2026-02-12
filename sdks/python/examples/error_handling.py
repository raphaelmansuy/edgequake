#!/usr/bin/env python3
"""
Error Handling — EdgeQuake Python SDK

WHY: Demonstrates how to handle different error types from the EdgeQuake SDK,
including retries, rate limiting, and graceful degradation.

Requirements:
    - EdgeQuake server running on http://localhost:8080 (may be offline for some tests)

Usage:
    export EDGEQUAKE_API_KEY="your-api-key"
    python examples/error_handling.py
"""
import os
import time

from edgequake import EdgequakeClient
from edgequake.exceptions import (
    EdgeQuakeError,
    NetworkError,
    NotFoundError,
    RateLimitedError,
    TimeoutError,
    UnauthorizedError,
    ValidationError,
)


def main():
    client = EdgequakeClient(
        api_key=os.environ.get("EDGEQUAKE_API_KEY", "your-api-key"),
        base_url="http://localhost:8080",
    )

    # --- Pattern 1: Specific error handling ---
    print("=== Pattern 1: Specific error types ===")
    try:
        client.documents.get("non-existent-id")
    except NotFoundError:
        print("Document not found — could prompt user to upload")
    except UnauthorizedError:
        print("Invalid API key — redirect to login")
    except RateLimitedError:
        print("Rate limited — back off and retry")
    except Exception as error:
        print(f"Unexpected error: {error}")

    # --- Pattern 2: Retry with exponential backoff ---
    print("\n=== Pattern 2: Retry with backoff ===")

    def query_with_retry(query: str, max_retries: int = 3):
        for attempt in range(1, max_retries + 1):
            try:
                return client.query.execute(query=query)
            except RateLimitedError:
                if attempt < max_retries:
                    delay = pow(2, attempt)  # 2s, 4s, 8s
                    print(
                        f"Rate limited, retrying in {delay}s "
                        f"(attempt {attempt}/{max_retries})"
                    )
                    time.sleep(delay)
                    continue
                raise  # Max retries exceeded
            except NetworkError:
                if attempt < max_retries:
                    print(
                        f"Network error, retrying " f"(attempt {attempt}/{max_retries})"
                    )
                    time.sleep(1)
                    continue
                raise  # Non-retryable or max retries exceeded

    try:
        result = query_with_retry("What is EdgeQuake?")
        print(f"Query result: {result.get('answer')}")
    except Exception as error:
        print(f"All retries exhausted: {error}")

    # --- Pattern 3: Graceful degradation ---
    print("\n=== Pattern 3: Graceful degradation ===")
    try:
        health = client.health()
        print(f"Backend healthy: {health.get('status')}")
    except (NetworkError, TimeoutError):
        print("Backend unreachable — showing cached data")
    except Exception as error:
        print(f"Unexpected health check failure: {error}")

    # --- Pattern 4: Validation error details ---
    print("\n=== Pattern 4: Validation errors ===")
    try:
        client.documents.upload(content="")  # Empty content
    except ValidationError as error:
        print(f"Validation failed: {error}")
        if hasattr(error, "status"):
            print(f"Status: {error.status}")  # 400 or 422
        if hasattr(error, "code"):
            print(f"Code: {error.code}")
    except Exception as error:
        print(f"Unexpected error: {error}")

    # --- Pattern 5: Generic EdgeQuakeError catch-all ---
    print("\n=== Pattern 5: Catch-all ===")
    try:
        client.documents.get("some-id")
    except EdgeQuakeError as error:
        # All SDK errors extend EdgeQuakeError
        status = getattr(error, "status", "N/A")
        code = getattr(error, "code", "N/A")
        print(f"API error [{status}] {code}: {error}")
    except Exception as error:
        # Non-SDK error (e.g., TypeError from bad config)
        print(f"Unexpected error: {error}")


if __name__ == "__main__":
    main()
