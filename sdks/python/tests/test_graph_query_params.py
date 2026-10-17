"""Query-parameter names must match the server contract.

Server: SearchLabelsQuery { q }, EntityExistsQuery { entity_name }
(edgequake-api handlers/graph_types.rs, handlers/entities_types.rs).
"""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake


def _resp(payload: dict) -> MagicMock:
    r = MagicMock()
    r.json.return_value = payload
    return r


@patch("edgequake._transport.SyncTransport.request")
def test_search_labels_sends_q_param(mock_req: MagicMock) -> None:
    mock_req.return_value = _resp({"labels": []})
    client = EdgeQuake()
    client.graph.search_labels("PER")
    assert mock_req.call_args.kwargs["params"]["q"] == "PER"
    assert "query" not in mock_req.call_args.kwargs["params"]
    client.close()


@patch("edgequake._transport.SyncTransport.request")
def test_entity_exists_sends_entity_name_param(mock_req: MagicMock) -> None:
    mock_req.return_value = _resp({"exists": True, "entity_name": "ALICE"})
    client = EdgeQuake()
    client.entities.exists("ALICE")
    assert mock_req.call_args.kwargs["params"] == {"entity_name": "ALICE"}
    client.close()


@pytest.mark.asyncio
@patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
async def test_async_search_labels_sends_q_param(mock_req: AsyncMock) -> None:
    mock_req.return_value = _resp({"labels": []})
    client = AsyncEdgeQuake()
    await client.graph.search_labels("PER")
    assert mock_req.call_args.kwargs["params"]["q"] == "PER"
    assert "query" not in mock_req.call_args.kwargs["params"]


@pytest.mark.asyncio
@patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
async def test_async_entity_exists_sends_entity_name_param(mock_req: AsyncMock) -> None:
    mock_req.return_value = _resp({"exists": True, "entity_name": "ALICE"})
    client = AsyncEdgeQuake()
    await client.entities.exists("ALICE")
    assert mock_req.call_args.kwargs["params"] == {"entity_name": "ALICE"}
