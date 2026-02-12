#!/usr/bin/env python3
"""
Configuration — EdgeQuake Python SDK

WHY: Demonstrates different ways to configure the EdgeQuake SDK client,
including environment variables, explicit config, and multi-environment setup.

Usage:
    # Minimal configuration (uses defaults)
    python examples/configuration.py

    # Environment-based configuration
    export EDGEQUAKE_BASE_URL="https://api.edgequake.example.com"
    export EDGEQUAKE_API_KEY="sk-..."
    export EDGEQUAKE_WORKSPACE_ID="my-workspace"
    python examples/configuration.py
"""
import os

from edgequake import EdgequakeClient


# --- Pattern 1: Minimal configuration ---
# WHY: base_url defaults to http://localhost:8080 if not provided
def create_simple_client():
    return EdgequakeClient()


# --- Pattern 2: Explicit configuration ---
def create_explicit_client():
    return EdgequakeClient(
        base_url="https://api.edgequake.example.com",
        api_key="your-api-key-here",
        timeout=30,  # 30 seconds
    )


# --- Pattern 3: Environment-based configuration ---
# Set these environment variables:
#   EDGEQUAKE_BASE_URL=https://api.edgequake.example.com
#   EDGEQUAKE_API_KEY=sk-...
#   EDGEQUAKE_WORKSPACE_ID=my-workspace
def create_env_client():
    return EdgequakeClient(
        base_url=os.environ.get("EDGEQUAKE_BASE_URL", "http://localhost:8080"),
        api_key=os.environ.get("EDGEQUAKE_API_KEY"),
        workspace_id=os.environ.get("EDGEQUAKE_WORKSPACE_ID"),
    )


# --- Pattern 4: Multi-tenant configuration ---
def create_tenant_client():
    return EdgequakeClient(
        base_url="http://localhost:8080",
        api_key="admin-api-key",
        workspace_id="tenant-workspace-123",
        tenant_id="tenant-456",
    )


# --- Pattern 5: Per-environment factory ---
def create_client(env: str = "development"):
    """
    Create a client configured for a specific environment.

    Args:
        env: One of 'development', 'staging', 'production'

    Returns:
        EdgequakeClient instance configured for the environment
    """
    configs = {
        "development": {
            "base_url": "http://localhost:8080",
            "timeout": 60,  # Longer timeout for dev
        },
        "staging": {
            "base_url": "https://staging-api.edgequake.example.com",
            "api_key": os.environ.get("STAGING_API_KEY"),
            "timeout": 30,
        },
        "production": {
            "base_url": "https://api.edgequake.example.com",
            "api_key": os.environ.get("PRODUCTION_API_KEY"),
            "timeout": 15,
        },
    }

    return EdgequakeClient(**configs.get(env, configs["development"]))


# --- Pattern 6: Health check before use ---
def get_healthy_client():
    """
    Create a client and verify the backend is healthy before returning it.

    Returns:
        EdgequakeClient instance if healthy

    Raises:
        RuntimeError: If backend is unhealthy
    """
    client = EdgequakeClient(base_url="http://localhost:8080")

    health = client.health()
    if health.get("status") != "healthy":
        raise RuntimeError(f"Backend unhealthy: {health}")

    print(
        "Connected to EdgeQuake:",
        {
            "version": health.get("version"),
            "storage": health.get("storage_mode"),
            "provider": health.get("llm_provider_name"),
        },
    )

    return client


def main():
    # Demo: use the environment factory
    env = os.environ.get("ENVIRONMENT", "development")
    client = create_client(env)
    print(f"Created client for {env} environment")

    # Demo: health check pattern
    try:
        healthy_client = get_healthy_client()
        docs = healthy_client.documents.list(page=1, page_size=5)
        total = docs.get("total", 0) if isinstance(docs, dict) else len(docs)
        print(f"Found {total} documents")
    except Exception as error:
        print(f"Could not connect: {error}")


if __name__ == "__main__":
    main()
