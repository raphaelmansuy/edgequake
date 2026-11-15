"""entities.merge must send the server's field names.

Server: MergeEntitiesRequest { source_entity, target_entity, merge_strategy }
(handlers/entities_types.rs); both names are required, so `source`/`target`
produced a 422 on every merge.
"""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake

_PAYLOAD = {"merged_entity": {"entity_name": "A"}, "merged_count": 1}


@patch("edgequake._transport.SyncTransport.request")
def test_merge_body_uses_source_entity_target_entity(mock_req: MagicMock) -> None:
    resp = MagicMock()
    resp.json.return_value = _PAYLOAD
    mock_req.return_value = resp
    client = EdgeQuake()
    client.entities.merge("A", "B")
    assert mock_req.call_args.kwargs["json"] == {
        "source_entity": "A",
        "target_entity": "B",
    }
    client.close()


@pytest.mark.asyncio
@patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
async def test_async_merge_body_uses_source_entity_target_entity(
    mock_req: AsyncMock,
) -> None:
    resp = MagicMock()
    resp.json.return_value = _PAYLOAD
    mock_req.return_value = resp
    client = AsyncEdgeQuake()
    await client.entities.merge("A", "B")
    assert mock_req.call_args.kwargs["json"] == {
        "source_entity": "A",
        "target_entity": "B",
    }
