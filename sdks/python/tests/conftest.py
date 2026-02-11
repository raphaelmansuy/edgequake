"""Shared test fixtures for the EdgeQuake Python SDK tests."""

from __future__ import annotations

import pytest

from edgequake import EdgeQuake
from edgequake._config import ClientConfig
from edgequake._transport import SyncTransport


@pytest.fixture
def config() -> ClientConfig:
    """Default test client configuration."""
    return ClientConfig(
        base_url="http://test-server:8080",
        api_key="test-key-12345",
        tenant_id="test-tenant",
        workspace_id="test-workspace",
        timeout=5.0,
        max_retries=0,  # WHY: No retries in unit tests for fast failures
    )


@pytest.fixture
def sync_transport(config: ClientConfig) -> SyncTransport:
    """SyncTransport instance for tests."""
    transport = SyncTransport(config)
    yield transport
    transport.close()


@pytest.fixture
def client() -> EdgeQuake:
    """Default sync client for tests."""
    c = EdgeQuake(
        base_url="http://test-server:8080",
        api_key="test-key-12345",
        max_retries=0,
    )
    yield c
    c.close()
