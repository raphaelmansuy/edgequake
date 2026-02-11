"""Graph type definitions for the EdgeQuake Python SDK.

WHY: Maps knowledge graph API types to Pydantic models, matching
edgequake-api/src/handlers/graph_types.rs.
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field


class GraphNode(BaseModel):
    """A node in the knowledge graph."""

    id: str
    label: str
    node_type: str | None = None
    description: str | None = None
    properties: dict[str, Any] | None = None
    degree: int | None = None


class GraphEdge(BaseModel):
    """An edge in the knowledge graph."""

    source: str
    target: str
    edge_type: str | None = None
    weight: float | None = None
    properties: dict[str, Any] | None = None


class GraphResponse(BaseModel):
    """Response from GET /api/v1/graph."""

    nodes: list[GraphNode] = Field(default_factory=list)
    edges: list[GraphEdge] = Field(default_factory=list)
    total_nodes: int | None = None
    total_edges: int | None = None
    is_truncated: bool | None = None


class GraphStreamEvent(BaseModel):
    """SSE event for streaming graph data."""

    nodes: list[GraphNode] | None = None
    edges: list[GraphEdge] | None = None
    done: bool = False
    progress: float | None = None


class SearchNodesResponse(BaseModel):
    """Response from GET /api/v1/graph/nodes/search."""

    nodes: list[GraphNode] = Field(default_factory=list)
    edges: list[GraphEdge] = Field(default_factory=list)
    total_matches: int | None = None
    is_truncated: bool | None = None


class SearchLabelsResponse(BaseModel):
    """Response from GET /api/v1/graph/labels/search."""

    labels: list[str] = Field(default_factory=list)


class PopularLabelInfo(BaseModel):
    """Info about a popular label."""

    label: str
    entity_type: str | None = None
    degree: int | None = None
    description: str | None = None


class PopularLabelsResponse(BaseModel):
    """Response from GET /api/v1/graph/labels/popular."""

    labels: list[PopularLabelInfo] = Field(default_factory=list)


class DegreesBatchResponse(BaseModel):
    """Response from POST /api/v1/graph/degrees/batch."""

    degrees: dict[str, int] = Field(default_factory=dict)


# --- Entity types ---


class EntityCreate(BaseModel):
    """Request to create an entity."""

    # WHY: API expects `entity_name` — we expose `name` for SDK convenience
    name: str = Field(serialization_alias="entity_name")
    entity_type: str
    description: str | None = None
    properties: dict[str, Any] | None = None
    source_id: str | None = None

    model_config = {"populate_by_name": True}


class EntityUpdate(BaseModel):
    """Request to update an entity."""

    entity_type: str | None = None
    description: str | None = None
    properties: dict[str, Any] | None = None


class Entity(BaseModel):
    """An entity in the knowledge graph."""

    # WHY: API returns `entity_name` and/or `id` — handle both
    name: str | None = Field(default=None, validation_alias="entity_name")
    id: str | None = None
    entity_type: str | None = None
    description: str | None = None
    properties: dict[str, Any] | None = None
    degree: int | None = None
    source_count: int | None = None
    source_id: str | None = None
    created_at: str | None = None
    updated_at: str | None = None
    metadata: dict[str, Any] | None = None

    model_config = {"populate_by_name": True}


class EntityDetail(Entity):
    """Detailed entity info with neighborhood data."""

    neighbors: list[Entity] | None = None
    relationships: list[dict[str, Any]] | None = None
    sources: list[dict[str, Any]] | None = None


class EntityExistsResponse(BaseModel):
    """Response from GET /api/v1/graph/entities/exists."""

    exists: bool
    entity_name: str | None = None


class MergeEntitiesRequest(BaseModel):
    """Request to merge entities."""

    source: str
    target: str
    strategy: str | None = None


class MergeEntitiesResponse(BaseModel):
    """Response from POST /api/v1/graph/entities/merge."""

    merged_entity: Entity | None = None
    merged_count: int = 0
    message: str | None = None


class NeighborhoodResponse(BaseModel):
    """Response from GET /api/v1/graph/entities/{name}/neighborhood."""

    center: Entity | None = None
    nodes: list[GraphNode] = Field(default_factory=list)
    edges: list[GraphEdge] = Field(default_factory=list)
    depth: int = 1


# --- Relationship types ---


class RelationshipCreate(BaseModel):
    """Request to create a relationship."""

    source: str
    target: str
    relationship_type: str
    weight: float | None = None
    description: str | None = None
    properties: dict[str, Any] | None = None
    source_id: str | None = None


class RelationshipUpdate(BaseModel):
    """Request to update a relationship."""

    relationship_type: str | None = None
    weight: float | None = None
    description: str | None = None
    properties: dict[str, Any] | None = None


class Relationship(BaseModel):
    """A relationship between entities."""

    id: str | None = None
    source: str
    target: str
    relationship_type: str | None = None
    weight: float | None = None
    description: str | None = None
    properties: dict[str, Any] | None = None
    created_at: str | None = None
    updated_at: str | None = None


class RelationshipDetail(Relationship):
    """Detailed relationship info."""

    source_entity: Entity | None = None
    target_entity: Entity | None = None
