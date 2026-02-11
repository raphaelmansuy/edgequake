"""Workspace type definitions for the EdgeQuake Python SDK.

WHY: Maps workspace-related request/response types, matching
edgequake-api/src/handlers/workspace_types.rs.
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field


class WorkspaceCreate(BaseModel):
    """Request to create a workspace."""

    name: str
    slug: str | None = None
    description: str | None = None
    settings: dict[str, Any] | None = None


class WorkspaceUpdate(BaseModel):
    """Request to update a workspace."""

    name: str | None = None
    description: str | None = None
    settings: dict[str, Any] | None = None


class WorkspaceInfo(BaseModel):
    """Workspace summary information."""

    id: str
    name: str
    slug: str | None = None
    description: str | None = None
    tenant_id: str | None = None
    created_at: str | None = None
    updated_at: str | None = None


class WorkspaceDetail(WorkspaceInfo):
    """Detailed workspace information."""

    settings: dict[str, Any] | None = None
    document_count: int | None = None
    entity_count: int | None = None
    relationship_count: int | None = None
    storage_size_bytes: int | None = None


class WorkspaceStats(BaseModel):
    """Workspace statistics from GET /workspaces/{id}/stats."""

    workspace_id: str
    document_count: int = 0
    entity_count: int = 0
    relationship_count: int = 0
    chunk_count: int = 0
    query_count: int = 0
    storage_size_bytes: int = 0
    last_activity: str | None = None


class MetricsHistoryEntry(BaseModel):
    """A single metrics history data point."""

    timestamp: str
    document_count: int | None = None
    entity_count: int | None = None
    relationship_count: int | None = None
    query_count: int | None = None


class MetricsHistoryResponse(BaseModel):
    """Response from GET /workspaces/{id}/metrics-history."""

    workspace_id: str
    entries: list[MetricsHistoryEntry] = Field(default_factory=list)


class RebuildResponse(BaseModel):
    """Response from rebuild operations."""

    status: str
    message: str | None = None
    track_id: str | None = None
    estimated_time_ms: int | None = None
