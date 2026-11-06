"""Tests for the Connections resource."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake

CONNECTION = {
    "id": "11111111-1111-1111-1111-111111111111",
    "tenant_id": None,
    "slug": "my-ollama",
    "display_name": "Ollama",
    "api_shape": "ollama",
    "locality": "local",
    "base_url": "http://127.0.0.1:11434",
    "auth_scheme": "none",
    "key_fingerprint": None,
    "key_configured": False,
    "timeout_secs": 120,
    "allow_private_network": True,
    "source": "db",
    "last_test_ok": None,
    "last_test_error": None,
}

PROBE = {
    "ok": True,
    "kind": "ok",
    "latency_ms": 12,
    "message": "ok",
    "models": ["gemma3"],
    "embedding_dimension": None,
    "chat_ok": True,
    "embed_ok": False,
    "list_ok": True,
}


class TestConnectionsResource:
    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [CONNECTION]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        rows = client.connections.list()
        assert mock_req.call_args[0] == ("GET", "/api/v1/connections")
        assert rows[0].slug == "my-ollama"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create_sends_upsert_body(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = CONNECTION
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        created = client.connections.create(
            slug="my-ollama",
            display_name="Ollama",
            api_shape="ollama",
            base_url="http://127.0.0.1:11434",
            locality="local",
            api_key="secret",
            allow_private_network=False,
        )
        assert mock_req.call_args[0] == ("POST", "/api/v1/connections?scope=deployment")
        body = mock_req.call_args[1]["json"]
        assert body["slug"] == "my-ollama"
        assert body["api_key"] == "secret"
        assert body["allow_private_network"] is False
        assert "timeout_secs" not in body
        assert created.id == CONNECTION["id"]
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update_and_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = CONNECTION
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.connections.update(
            CONNECTION["id"],
            slug="my-ollama",
            display_name="Ollama",
            api_shape="ollama",
            base_url="http://127.0.0.1:11434",
        )
        assert mock_req.call_args[0] == (
            "PUT",
            f"/api/v1/connections/{CONNECTION['id']}?scope=deployment",
        )
        client.connections.delete(CONNECTION["id"])
        assert mock_req.call_args[0] == (
            "DELETE",
            f"/api/v1/connections/{CONNECTION['id']}?scope=deployment",
        )
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_stored_and_unsaved_probe(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = PROBE
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        stored = client.connections.test(CONNECTION["id"])
        assert mock_req.call_args[0] == (
            "POST",
            f"/api/v1/connections/{CONNECTION['id']}/test?scope=deployment",
        )
        assert stored.ok is True
        assert stored.kind == "ok"

        probed = client.connections.test_provider(
            shape="ollama",
            base_url="http://127.0.0.1:11434",
            model="gemma3",
            expected_dimension=768,
        )
        assert mock_req.call_args[0] == ("POST", "/api/v1/providers/test")
        body = mock_req.call_args[1]["json"]
        assert body == {
            "shape": "ollama",
            "base_url": "http://127.0.0.1:11434",
            "model": "gemma3",
            "expected_dimension": 768,
        }
        assert probed.models == ["gemma3"]
        client.close()


class TestAsyncConnectionsResource:
    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [CONNECTION]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        rows = await client.connections.list()
        assert mock_req.call_args[0] == ("GET", "/api/v1/connections")
        assert rows[0].source == "db"
