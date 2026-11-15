"""relationships.create must match the server contract.

Server: CreateRelationshipRequest { src_id, tgt_id, keywords, weight,
description, source_id, metadata } with src_id/tgt_id/keywords/description/
source_id required; response is CreateRelationshipResponse
{ status, message, relationship: RelationshipResponse } where the
relationship carries src_id / tgt_id / relation_type.
"""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake.types.graph import Relationship, RelationshipCreate

_SERVER_RESPONSE = {
    "status": "success",
    "message": "created",
    "relationship": {
        "id": "r1",
        "src_id": "ALICE",
        "tgt_id": "BOB",
        "relation_type": "KNOWS",
        "keywords": "KNOWS",
        "weight": 0.8,
        "description": "",
        "source_id": "manual_entry",
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z",
        "metadata": {},
    },
}

_EXPECTED_BODY = {
    "src_id": "ALICE",
    "tgt_id": "BOB",
    "keywords": "KNOWS",
    "description": "",
    "source_id": "manual_entry",
}


def _resp() -> MagicMock:
    r = MagicMock()
    r.json.return_value = _SERVER_RESPONSE
    return r


@patch("edgequake._transport.SyncTransport.request")
def test_create_sends_server_field_names_and_decodes_response(
    mock_req: MagicMock,
) -> None:
    mock_req.return_value = _resp()
    client = EdgeQuake()
    result = client.relationships.create(
        RelationshipCreate(source="ALICE", target="BOB", relationship_type="KNOWS")
    )
    assert mock_req.call_args.kwargs["json"] == _EXPECTED_BODY
    assert isinstance(result, Relationship)
    assert (result.id, result.source, result.target) == ("r1", "ALICE", "BOB")
    assert result.relationship_type == "KNOWS"
    client.close()


@pytest.mark.asyncio
@patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
async def test_async_create_sends_server_field_names_and_decodes_response(
    mock_req: AsyncMock,
) -> None:
    mock_req.return_value = _resp()
    client = AsyncEdgeQuake()
    result = await client.relationships.create(
        RelationshipCreate(source="ALICE", target="BOB", relationship_type="KNOWS")
    )
    assert mock_req.call_args.kwargs["json"] == _EXPECTED_BODY
    assert (result.source, result.target) == ("ALICE", "BOB")


def test_relationship_decodes_server_list_item() -> None:
    rel = Relationship.model_validate(_SERVER_RESPONSE["relationship"])
    assert (rel.source, rel.target, rel.relationship_type) == (
        "ALICE",
        "BOB",
        "KNOWS",
    )
