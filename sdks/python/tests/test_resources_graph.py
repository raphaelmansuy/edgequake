"""Tests for graph, entity, and relationship resources."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake._streaming import AsyncSSEStream, SSEStream
from edgequake.types.graph import (
    DegreesBatchResponse,
    Entity,
    EntityCreate,
    EntityDetail,
    EntityExistsResponse,
    EntityUpdate,
    GraphNode,
    GraphResponse,
    MergeEntitiesResponse,
    NeighborhoodResponse,
    PopularLabelsResponse,
    Relationship,
    RelationshipCreate,
    RelationshipDetail,
    RelationshipUpdate,
    SearchLabelsResponse,
    SearchNodesResponse,
)


class TestGraphResource:
    """Test sync GraphResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "nodes": [{"id": "n1", "label": "PERSON", "properties": {"name": "Alice"}}],
            "edges": [
                {
                    "source": "n1",
                    "target": "n2",
                    "edge_type": "KNOWS",
                }
            ],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.graph.get()
        assert isinstance(result, GraphResponse)
        assert len(result.nodes) == 1
        assert result.nodes[0].label == "PERSON"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get_with_params(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "edges": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.graph.get(label="PERSON", limit=50)
        call_kwargs = mock_req.call_args[1]
        assert call_kwargs["params"]["label"] == "PERSON"
        assert call_kwargs["params"]["limit"] == 50
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get_node(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "n1",
            "label": "PERSON",
            "properties": {"name": "Alice"},
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.graph.get_node("n1")
        assert isinstance(result, GraphNode)
        assert result.id == "n1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_search_nodes(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "nodes": [{"id": "n1", "label": "PERSON", "properties": {"name": "Alice"}}],
            "total_matches": 1,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.graph.search_nodes(query="Alice")
        assert isinstance(result, SearchNodesResponse)
        assert result.total_matches == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_search_nodes_with_label(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "total_matches": 0}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.graph.search_nodes(query="test", label="PERSON", limit=10)
        params = mock_req.call_args[1]["params"]
        assert params["q"] == "test"
        assert params["label"] == "PERSON"
        assert params["limit"] == 10
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_search_labels(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "labels": ["PERSON", "ORGANIZATION", "LOCATION"],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.graph.search_labels(query="PER")
        assert isinstance(result, SearchLabelsResponse)
        assert "PERSON" in result.labels
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_popular_labels(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "labels": [{"label": "PERSON", "entity_type": "node", "degree": 100}],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.graph.popular_labels(limit=5)
        assert isinstance(result, PopularLabelsResponse)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_degrees_batch(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "degrees": {"n1": 5, "n2": 3},
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.graph.degrees_batch(["n1", "n2"])
        assert isinstance(result, DegreesBatchResponse)
        client.close()

    @patch("edgequake._transport.SyncTransport.stream")
    def test_stream(self, mock_stream: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = EdgeQuake()
        result = client.graph.stream(label="PERSON")
        assert isinstance(result, SSEStream)
        call_args = mock_stream.call_args
        assert call_args[1]["params"]["label"] == "PERSON"
        client.close()


class TestEntitiesResource:
    """Test sync EntitiesResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"name": "ALICE", "entity_type": "PERSON"},
            {"name": "BOB", "entity_type": "PERSON"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.list()
        assert isinstance(result, list)
        assert len(result) == 2
        assert all(isinstance(e, Entity) for e in result)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "entities": [{"name": "ALICE", "entity_type": "PERSON"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.list(entity_type="PERSON", page=2, per_page=10)
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "name": "ALICE",
            "entity_type": "PERSON",
            "description": "A character",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.create(
            EntityCreate(
                name="ALICE",
                entity_type="PERSON",
                description="A character",
            )
        )
        assert isinstance(result, Entity)
        assert result.name == "ALICE"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create_unwrap_entity(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "status": "created",
            "message": "Entity created",
            "entity": {"name": "BOB", "entity_type": "PERSON"},
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.create(EntityCreate(name="BOB", entity_type="PERSON"))
        assert result.name == "BOB"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "name": "ALICE",
            "entity_type": "PERSON",
            "description": "desc",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.get("ALICE")
        assert isinstance(result, EntityDetail)
        assert result.name == "ALICE"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "name": "ALICE",
            "entity_type": "PERSON",
            "description": "Updated desc",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.update(
            "ALICE", EntityUpdate(description="Updated desc")
        )
        assert isinstance(result, Entity)
        assert result.description == "Updated desc"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_exists(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"exists": True, "entity_name": "ALICE"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.exists("ALICE")
        assert isinstance(result, EntityExistsResponse)
        assert result.exists is True
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_merge(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "merged_entity": {"name": "ALICE", "entity_type": "PERSON"},
            "merged_count": 2,
            "message": "Merged successfully",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.merge(source="ALICE_2", target="ALICE")
        assert isinstance(result, MergeEntitiesResponse)
        assert result.merged_count == 2
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_neighborhood(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "nodes": [{"id": "n1", "label": "PERSON"}],
            "edges": [],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.entities.neighborhood("ALICE", depth=2)
        assert isinstance(result, NeighborhoodResponse)
        params = mock_req.call_args[1]["params"]
        assert params["depth"] == 2
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.entities.delete("ALICE")
        mock_req.assert_called_once()
        client.close()


class TestRelationshipsResource:
    """Test sync RelationshipsResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "rel-1",
            "source": "ALICE",
            "target": "BOB",
            "relationship_type": "KNOWS",
            "weight": 1.0,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.relationships.create(
            RelationshipCreate(
                source="ALICE",
                target="BOB",
                relationship_type="KNOWS",
            )
        )
        assert isinstance(result, Relationship)
        assert result.relationship_type == "KNOWS"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {
                "id": "rel-1",
                "source": "ALICE",
                "target": "BOB",
                "relationship_type": "KNOWS",
            }
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.relationships.list()
        assert isinstance(result, list)
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "relationships": [
                {"id": "rel-1", "source": "A", "target": "B", "relationship_type": "X"}
            ]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.relationships.list(source="A", page=2, per_page=10)
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "rel-1",
            "source": "ALICE",
            "target": "BOB",
            "relationship_type": "KNOWS",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.relationships.get("rel-1")
        assert isinstance(result, RelationshipDetail)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "rel-1",
            "source": "ALICE",
            "target": "BOB",
            "relationship_type": "KNOWS",
            "weight": 2.0,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.relationships.update("rel-1", RelationshipUpdate(weight=2.0))
        assert isinstance(result, Relationship)
        assert result.weight == 2.0
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.relationships.delete("rel-1")
        mock_req.assert_called_once()
        client.close()


class TestAsyncGraphResource:
    """Test async GraphResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "edges": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.graph.get(label="PERSON", limit=10)
        assert isinstance(result, GraphResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get_node(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "n1", "label": "PERSON"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.graph.get_node("n1")
        assert isinstance(result, GraphNode)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_search_nodes(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "total_matches": 0}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.graph.search_nodes("test", limit=5)
        assert isinstance(result, SearchNodesResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_search_labels(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"labels": ["PERSON"]}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.graph.search_labels("PER")
        assert isinstance(result, SearchLabelsResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_popular_labels(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"labels": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.graph.popular_labels(limit=3)
        assert isinstance(result, PopularLabelsResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_degrees_batch(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"degrees": {"n1": 5}}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.graph.degrees_batch(["n1"])
        assert isinstance(result, DegreesBatchResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.stream", new_callable=AsyncMock)
    async def test_stream(self, mock_stream: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.graph.stream(label="PERSON")
        assert isinstance(result, AsyncSSEStream)


class TestAsyncEntitiesResource:
    """Test async EntitiesResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"name": "ALICE", "entity_type": "PERSON"},
        ]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.list(entity_type="PERSON")
        assert len(result) == 1
        assert isinstance(result[0], Entity)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"name": "BOB", "entity_type": "PERSON"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.create(
            EntityCreate(name="BOB", entity_type="PERSON")
        )
        assert isinstance(result, Entity)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create_unwrap(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "status": "ok",
            "entity": {"name": "BOB", "entity_type": "PERSON"},
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.create(
            EntityCreate(name="BOB", entity_type="PERSON")
        )
        assert result.name == "BOB"

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"name": "ALICE", "entity_type": "PERSON"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.get("ALICE")
        assert isinstance(result, EntityDetail)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_update(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "name": "ALICE",
            "entity_type": "PERSON",
            "description": "Updated",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.update(
            "ALICE", EntityUpdate(description="Updated")
        )
        assert isinstance(result, Entity)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.entities.delete("ALICE")
        mock_req.assert_called_once()

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_exists(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"exists": False, "entity_name": "NOPE"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.exists("NOPE")
        assert isinstance(result, EntityExistsResponse)
        assert result.exists is False

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_merge(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "merged_entity": {"name": "ALICE"},
            "merged_count": 2,
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.merge("A", "B")
        assert isinstance(result, MergeEntitiesResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_neighborhood(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "edges": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.entities.neighborhood("ALICE", depth=3)
        assert isinstance(result, NeighborhoodResponse)


class TestAsyncRelationshipsResource:
    """Test async RelationshipsResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "r1", "source": "A", "target": "B", "relationship_type": "KNOWS"}
        ]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.relationships.list()
        assert len(result) == 1
        assert isinstance(result[0], Relationship)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "r1",
            "source": "A",
            "target": "B",
            "relationship_type": "KNOWS",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.relationships.create(
            RelationshipCreate(source="A", target="B", relationship_type="KNOWS")
        )
        assert isinstance(result, Relationship)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "r1",
            "source": "A",
            "target": "B",
            "relationship_type": "KNOWS",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.relationships.get("r1")
        assert isinstance(result, RelationshipDetail)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_update(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "r1",
            "source": "A",
            "target": "B",
            "relationship_type": "KNOWS",
            "weight": 3.0,
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.relationships.update("r1", RelationshipUpdate(weight=3.0))
        assert isinstance(result, Relationship)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.relationships.delete("r1")
        mock_req.assert_called_once()
