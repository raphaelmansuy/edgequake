"""Tests for edgequake._client module."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest

from edgequake._client import AsyncEdgeQuake, EdgeQuake
from edgequake.types.shared import HealthResponse


class TestEdgeQuakeClient:
    """Test synchronous EdgeQuake client."""

    def test_default_construction(self) -> None:
        client = EdgeQuake()
        assert repr(client) == "EdgeQuake(base_url='http://localhost:8080')"

    def test_custom_construction(self) -> None:
        client = EdgeQuake(
            base_url="https://api.example.com",
            api_key="key-123",
            tenant_id="t-1",
            workspace_id="ws-1",
            user_id="u-1",
        )
        assert "api.example.com" in repr(client)
        client.close()

    def test_context_manager(self) -> None:
        with EdgeQuake() as client:
            assert isinstance(client, EdgeQuake)
        # Should not raise after context manager exit

    def test_with_workspace_returns_new_client(self) -> None:
        original = EdgeQuake(api_key="k", tenant_id="t")
        scoped = original.with_workspace("ws-new")
        assert scoped is not original
        assert scoped._config.workspace_id == "ws-new"
        assert scoped._config.api_key == "k"
        assert scoped._config.tenant_id == "t"
        original.close()
        scoped.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_health(self, mock_request: MagicMock) -> None:
        mock_response = MagicMock()
        mock_response.json.return_value = {
            "status": "healthy",
            "version": "0.1.0",
        }
        mock_request.return_value = mock_response

        client = EdgeQuake()
        health = client.health()
        assert isinstance(health, HealthResponse)
        assert health.status == "healthy"
        mock_request.assert_called_once_with("GET", "/health")
        client.close()


class TestAsyncEdgeQuakeClient:
    """Test async EdgeQuake client."""

    def test_default_construction(self) -> None:
        client = AsyncEdgeQuake()
        assert repr(client) == "AsyncEdgeQuake(base_url='http://localhost:8080')"

    def test_with_workspace_returns_new_client(self) -> None:
        original = AsyncEdgeQuake(api_key="k")
        scoped = original.with_workspace("ws-2")
        assert scoped is not original
        assert scoped._config.workspace_id == "ws-2"

    @pytest.mark.asyncio
    async def test_context_manager(self) -> None:
        async with AsyncEdgeQuake() as client:
            assert isinstance(client, AsyncEdgeQuake)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request")
    async def test_health(self, mock_request: MagicMock) -> None:
        mock_response = MagicMock()
        mock_response.json.return_value = {
            "status": "healthy",
            "version": "0.1.0",
        }
        mock_request.return_value = mock_response

        client = AsyncEdgeQuake()
        health = await client.health()
        assert isinstance(health, HealthResponse)
        assert health.status == "healthy"
        await client.close()
