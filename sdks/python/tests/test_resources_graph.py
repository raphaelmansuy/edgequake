"""Tests for graph, entity, and relationship resources."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from edgequake import EdgeQuake
from edgequake.types.graph import (
    Entity,
    EntityCreate,
    EntityDetail,
    EntityExistsResponse,
    GraphNode,
    GraphResponse,
    MergeEntitiesResponse,
    NeighborhoodResponse,
    Relationship,
    RelationshipCreate,
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


class TestEntitiesResource:
    """Test sync EntitiesResource."""

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
