"""Graph resource — Knowledge graph operations for EdgeQuake.

WHY: Maps to /api/v1/graph/* endpoints including nodes, search, labels,
entities, and relationships sub-resources.
"""

from __future__ import annotations

from typing import Any

from edgequake._streaming import AsyncSSEStream, SSEStream
from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.graph import (
    DegreesBatchResponse,
    Entity,
    EntityCreate,
    EntityDetail,
    EntityExistsResponse,
    EntityUpdate,
    GraphNode,
    GraphResponse,
    GraphStreamEvent,
    MergeEntitiesRequest,
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


class GraphResource(SyncResource):
    """Synchronous Graph API."""

    @property
    def entities(self) -> EntitiesResource:
        """Entity sub-namespace."""
        return EntitiesResource(self._transport)

    @property
    def relationships(self) -> RelationshipsResource:
        """Relationship sub-namespace."""
        return RelationshipsResource(self._transport)

    def get(
        self,
        *,
        label: str | None = None,
        limit: int | None = None,
    ) -> GraphResponse:
        """Get the knowledge graph.

        GET /api/v1/graph
        """
        params: dict[str, Any] = {}
        if label:
            params["label"] = label
        if limit:
            params["limit"] = limit
        return self._get("/api/v1/graph", params=params, response_type=GraphResponse)

    def stream(
        self,
        *,
        label: str | None = None,
    ) -> SSEStream[GraphStreamEvent]:
        """Stream the graph via SSE.

        GET /api/v1/graph/stream
        """
        params: dict[str, Any] = {}
        if label:
            params["label"] = label
        response = self._transport.stream("GET", "/api/v1/graph/stream", params=params)
        return SSEStream(response, GraphStreamEvent)

    def get_node(self, node_id: str) -> GraphNode:
        """Get a specific node.

        GET /api/v1/graph/nodes/{node_id}
        """
        return self._get(f"/api/v1/graph/nodes/{node_id}", response_type=GraphNode)

    def search_nodes(
        self,
        query: str | None = None,
        *,
        label: str | None = None,
        limit: int = 20,
    ) -> SearchNodesResponse:
        """Search graph nodes.

        GET /api/v1/graph/nodes/search
        """
        params: dict[str, Any] = {"limit": limit}
        if query:
            # WHY: API expects `q` not `query`
            params["q"] = query
        if label:
            params["label"] = label
        return self._get(
            "/api/v1/graph/nodes/search",
            params=params,
            response_type=SearchNodesResponse,
        )

    def search_labels(
        self,
        query: str | None = None,
        *,
        limit: int = 50,
    ) -> SearchLabelsResponse:
        """Search label names.

        GET /api/v1/graph/labels/search
        """
        params: dict[str, Any] = {"limit": limit}
        if query:
            params["query"] = query
        return self._get(
            "/api/v1/graph/labels/search",
            params=params,
            response_type=SearchLabelsResponse,
        )

    def popular_labels(self, *, limit: int = 20) -> PopularLabelsResponse:
        """Get popular labels.

        GET /api/v1/graph/labels/popular
        """
        return self._get(
            "/api/v1/graph/labels/popular",
            params={"limit": limit},
            response_type=PopularLabelsResponse,
        )

    def degrees_batch(self, node_ids: list[str]) -> DegreesBatchResponse:
        """Get node degrees in batch.

        POST /api/v1/graph/degrees/batch
        """
        return self._post(
            "/api/v1/graph/degrees/batch",
            json={"node_ids": node_ids},
            response_type=DegreesBatchResponse,
        )


class EntitiesResource(SyncResource):
    """Entity CRUD operations under /api/v1/graph/entities."""

    def list(
        self,
        *,
        page: int = 1,
        per_page: int = 20,
        entity_type: str | None = None,
    ) -> list[Entity]:
        """List entities.

        GET /api/v1/graph/entities
        """
        params: dict[str, Any] = {"page": page, "per_page": per_page}
        if entity_type:
            params["entity_type"] = entity_type
        data = self._get("/api/v1/graph/entities", params=params)
        if isinstance(data, list):
            return [Entity.model_validate(e) for e in data]
        items = (
            data.get("entities", data.get("items", []))
            if isinstance(data, dict)
            else []
        )
        return [Entity.model_validate(e) for e in items]

    def create(self, entity: EntityCreate) -> Entity:
        """Create an entity.

        POST /api/v1/graph/entities
        """
        # WHY: by_alias=True enables serialization_alias (name -> entity_name)
        data = self._post(
            "/api/v1/graph/entities",
            json=entity.model_dump(exclude_none=True, by_alias=True),
        )
        # WHY: API returns {status, message, entity: {...}} — unwrap
        if isinstance(data, dict) and "entity" in data:
            return Entity.model_validate(data["entity"])
        return Entity.model_validate(data)

    def get(self, entity_name: str) -> EntityDetail:
        """Get entity details.

        GET /api/v1/graph/entities/{entity_name}
        """
        return self._get(
            f"/api/v1/graph/entities/{entity_name}",
            response_type=EntityDetail,
        )

    def update(self, entity_name: str, update: EntityUpdate) -> Entity:
        """Update an entity.

        PUT /api/v1/graph/entities/{entity_name}
        """
        return self._put(
            f"/api/v1/graph/entities/{entity_name}",
            json=update.model_dump(exclude_none=True),
            response_type=Entity,
        )

    def delete(self, entity_name: str, *, confirm: bool = True) -> None:
        """Delete an entity.

        DELETE /api/v1/graph/entities/{entity_name}
        """
        # WHY: API requires `confirm` query parameter
        self._delete(
            f"/api/v1/graph/entities/{entity_name}",
            params={"confirm": str(confirm).lower()},
        )

    def exists(self, entity_name: str) -> EntityExistsResponse:
        """Check if an entity exists.

        GET /api/v1/graph/entities/exists
        """
        return self._get(
            "/api/v1/graph/entities/exists",
            params={"name": entity_name},
            response_type=EntityExistsResponse,
        )

    def merge(self, source: str, target: str) -> MergeEntitiesResponse:
        """Merge two entities.

        POST /api/v1/graph/entities/merge
        """
        return self._post(
            "/api/v1/graph/entities/merge",
            json=MergeEntitiesRequest(source=source, target=target).model_dump(),
            response_type=MergeEntitiesResponse,
        )

    def neighborhood(self, entity_name: str, *, depth: int = 1) -> NeighborhoodResponse:
        """Get entity neighborhood graph.

        GET /api/v1/graph/entities/{entity_name}/neighborhood
        """
        return self._get(
            f"/api/v1/graph/entities/{entity_name}/neighborhood",
            params={"depth": depth},
            response_type=NeighborhoodResponse,
        )


class RelationshipsResource(SyncResource):
    """Relationship CRUD operations under /api/v1/graph/relationships."""

    def list(
        self,
        *,
        page: int = 1,
        per_page: int = 20,
        source: str | None = None,
    ) -> list[Relationship]:
        """List relationships.

        GET /api/v1/graph/relationships
        """
        params: dict[str, Any] = {"page": page, "per_page": per_page}
        if source:
            params["source"] = source
        data = self._get("/api/v1/graph/relationships", params=params)
        if isinstance(data, list):
            return [Relationship.model_validate(r) for r in data]
        items = (
            data.get("relationships", data.get("items", []))
            if isinstance(data, dict)
            else []
        )
        return [Relationship.model_validate(r) for r in items]

    def create(self, rel: RelationshipCreate) -> Relationship:
        """Create a relationship.

        POST /api/v1/graph/relationships
        """
        return self._post(
            "/api/v1/graph/relationships",
            json=rel.model_dump(exclude_none=True),
            response_type=Relationship,
        )

    def get(self, relationship_id: str) -> RelationshipDetail:
        """Get relationship details.

        GET /api/v1/graph/relationships/{relationship_id}
        """
        return self._get(
            f"/api/v1/graph/relationships/{relationship_id}",
            response_type=RelationshipDetail,
        )

    def update(self, relationship_id: str, update: RelationshipUpdate) -> Relationship:
        """Update a relationship.

        PUT /api/v1/graph/relationships/{relationship_id}
        """
        return self._put(
            f"/api/v1/graph/relationships/{relationship_id}",
            json=update.model_dump(exclude_none=True),
            response_type=Relationship,
        )

    def delete(self, relationship_id: str) -> None:
        """Delete a relationship.

        DELETE /api/v1/graph/relationships/{relationship_id}
        """
        self._delete(f"/api/v1/graph/relationships/{relationship_id}")


# --- Async versions ---


class AsyncGraphResource(AsyncResource):
    """Asynchronous Graph API."""

    @property
    def entities(self) -> AsyncEntitiesResource:
        return AsyncEntitiesResource(self._transport)

    @property
    def relationships(self) -> AsyncRelationshipsResource:
        return AsyncRelationshipsResource(self._transport)

    async def get(
        self, *, label: str | None = None, limit: int | None = None
    ) -> GraphResponse:
        params: dict[str, Any] = {}
        if label:
            params["label"] = label
        if limit:
            params["limit"] = limit
        return await self._get(
            "/api/v1/graph", params=params, response_type=GraphResponse
        )

    async def stream(
        self, *, label: str | None = None
    ) -> AsyncSSEStream[GraphStreamEvent]:
        params: dict[str, Any] = {}
        if label:
            params["label"] = label
        response = await self._transport.stream(
            "GET", "/api/v1/graph/stream", params=params
        )
        return AsyncSSEStream(response, GraphStreamEvent)

    async def get_node(self, node_id: str) -> GraphNode:
        return await self._get(
            f"/api/v1/graph/nodes/{node_id}", response_type=GraphNode
        )

    async def search_nodes(
        self, query: str | None = None, *, limit: int = 20
    ) -> SearchNodesResponse:
        params: dict[str, Any] = {"limit": limit}
        if query:
            # WHY: API expects `q` not `query`
            params["q"] = query
        return await self._get(
            "/api/v1/graph/nodes/search",
            params=params,
            response_type=SearchNodesResponse,
        )

    async def search_labels(
        self, query: str | None = None, *, limit: int = 50
    ) -> SearchLabelsResponse:
        params: dict[str, Any] = {"limit": limit}
        if query:
            params["query"] = query
        return await self._get(
            "/api/v1/graph/labels/search",
            params=params,
            response_type=SearchLabelsResponse,
        )

    async def popular_labels(self, *, limit: int = 20) -> PopularLabelsResponse:
        return await self._get(
            "/api/v1/graph/labels/popular",
            params={"limit": limit},
            response_type=PopularLabelsResponse,
        )

    async def degrees_batch(self, node_ids: list[str]) -> DegreesBatchResponse:
        return await self._post(
            "/api/v1/graph/degrees/batch",
            json={"node_ids": node_ids},
            response_type=DegreesBatchResponse,
        )


class AsyncEntitiesResource(AsyncResource):
    """Async entity CRUD."""

    async def list(
        self, *, page: int = 1, per_page: int = 20, entity_type: str | None = None
    ) -> list[Entity]:
        params: dict[str, Any] = {"page": page, "per_page": per_page}
        if entity_type:
            params["entity_type"] = entity_type
        data = await self._get("/api/v1/graph/entities", params=params)
        if isinstance(data, list):
            return [Entity.model_validate(e) for e in data]
        items = (
            data.get("entities", data.get("items", []))
            if isinstance(data, dict)
            else []
        )
        return [Entity.model_validate(e) for e in items]

    async def create(self, entity: EntityCreate) -> Entity:
        # WHY: by_alias=True enables serialization_alias (name -> entity_name)
        data = await self._post(
            "/api/v1/graph/entities",
            json=entity.model_dump(exclude_none=True, by_alias=True),
        )
        # WHY: API returns {status, message, entity: {...}} — unwrap
        if isinstance(data, dict) and "entity" in data:
            return Entity.model_validate(data["entity"])
        return Entity.model_validate(data)

    async def get(self, entity_name: str) -> EntityDetail:
        return await self._get(
            f"/api/v1/graph/entities/{entity_name}",
            response_type=EntityDetail,
        )

    async def update(self, entity_name: str, update: EntityUpdate) -> Entity:
        return await self._put(
            f"/api/v1/graph/entities/{entity_name}",
            json=update.model_dump(exclude_none=True),
            response_type=Entity,
        )

    async def delete(self, entity_name: str, *, confirm: bool = True) -> None:
        # WHY: API requires `confirm` query parameter
        await self._delete(
            f"/api/v1/graph/entities/{entity_name}",
            params={"confirm": str(confirm).lower()},
        )

    async def exists(self, entity_name: str) -> EntityExistsResponse:
        return await self._get(
            "/api/v1/graph/entities/exists",
            params={"name": entity_name},
            response_type=EntityExistsResponse,
        )

    async def merge(self, source: str, target: str) -> MergeEntitiesResponse:
        return await self._post(
            "/api/v1/graph/entities/merge",
            json={"source": source, "target": target},
            response_type=MergeEntitiesResponse,
        )

    async def neighborhood(
        self, entity_name: str, *, depth: int = 1
    ) -> NeighborhoodResponse:
        return await self._get(
            f"/api/v1/graph/entities/{entity_name}/neighborhood",
            params={"depth": depth},
            response_type=NeighborhoodResponse,
        )


class AsyncRelationshipsResource(AsyncResource):
    """Async relationship CRUD."""

    async def list(
        self, *, page: int = 1, per_page: int = 20, source: str | None = None
    ) -> list[Relationship]:
        params: dict[str, Any] = {"page": page, "per_page": per_page}
        if source:
            params["source"] = source
        data = await self._get("/api/v1/graph/relationships", params=params)
        if isinstance(data, list):
            return [Relationship.model_validate(r) for r in data]
        items = (
            data.get("relationships", data.get("items", []))
            if isinstance(data, dict)
            else []
        )
        return [Relationship.model_validate(r) for r in items]

    async def create(self, rel: RelationshipCreate) -> Relationship:
        return await self._post(
            "/api/v1/graph/relationships",
            json=rel.model_dump(exclude_none=True),
            response_type=Relationship,
        )

    async def get(self, relationship_id: str) -> RelationshipDetail:
        return await self._get(
            f"/api/v1/graph/relationships/{relationship_id}",
            response_type=RelationshipDetail,
        )

    async def update(
        self, relationship_id: str, update: RelationshipUpdate
    ) -> Relationship:
        return await self._put(
            f"/api/v1/graph/relationships/{relationship_id}",
            json=update.model_dump(exclude_none=True),
            response_type=Relationship,
        )

    async def delete(self, relationship_id: str) -> None:
        await self._delete(f"/api/v1/graph/relationships/{relationship_id}")
