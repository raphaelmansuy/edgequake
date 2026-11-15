"""graph.get(limit=...) must use the server's `max_nodes` query parameter.

Server: GraphQueryParams { start_node, depth, max_nodes } (handlers/graph_types.rs).
`limit` is silently ignored by the server, so the cap never took effect.
"""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake


def _resp() -> MagicMock:
    r = MagicMock()
    r.json.return_value = {"nodes": [], "edges": []}
    return r


@patch("edgequake._transport.SyncTransport.request")
def test_graph_get_limit_sent_as_max_nodes(mock_req: MagicMock) -> None:
    mock_req.return_value = _resp()
    client = EdgeQuake()
    client.graph.get(limit=10)
    params = mock_req.call_args.kwargs["params"]
    assert params == {"max_nodes": 10}
    client.close()


@pytest.mark.asyncio
@patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
async def test_async_graph_get_limit_sent_as_max_nodes(mock_req: AsyncMock) -> None:
    mock_req.return_value = _resp()
    client = AsyncEdgeQuake()
    await client.graph.get(limit=10)
    assert mock_req.call_args.kwargs["params"] == {"max_nodes": 10}
