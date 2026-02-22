"""Workspace and tenant type definitions for the EdgeQuake Python SDK.

WHY: Maps workspace and tenant request/response types, matching
edgequake-api/src/handlers/workspaces_types.rs.
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field

# ── Tenants ────────────────────────────────────────────────────────────────


class TenantCreate(BaseModel):
    """Request to create a tenant (POST /api/v1/tenants)."""

    name: str
    slug: str | None = None
    description: str | None = None
    plan: str | None = None
    # Default LLM configuration for new workspaces.
    default_llm_model: str | None = None
    default_llm_provider: str | None = None
    # Default embedding configuration for new workspaces.
    default_embedding_model: str | None = None
    default_embedding_provider: str | None = None
    default_embedding_dimension: int | None = None
    # Default vision LLM for PDF image extraction (SPEC-041).
    default_vision_llm_model: str | None = None
    default_vision_llm_provider: str | None = None


class TenantInfo(BaseModel):
    """Tenant summary returned by the API."""

    id: str
    name: str
    slug: str | None = None
    plan: str | None = None
    is_active: bool | None = None
    max_workspaces: int | None = None
    # Default LLM configuration.
    default_llm_model: str | None = None
    default_llm_provider: str | None = None
    default_llm_full_id: str | None = None
    # Default embedding configuration.
    default_embedding_model: str | None = None
    default_embedding_provider: str | None = None
    default_embedding_dimension: int | None = None
    default_embedding_full_id: str | None = None
    # Default vision LLM (SPEC-041) – only present when configured.
    default_vision_llm_model: str | None = None
    default_vision_llm_provider: str | None = None
    created_at: str | None = None
    updated_at: str | None = None


class TenantListResponse(BaseModel):
    """Paginated list of tenants from GET /api/v1/tenants."""

    items: list[TenantInfo] = Field(default_factory=list)
    total: int = 0
    page: int = 1
    page_size: int = 20
    total_pages: int = 1


# ── Workspaces ─────────────────────────────────────────────────────────────


class WorkspaceCreate(BaseModel):
    """Request to create a workspace."""

    name: str
    slug: str | None = None
    description: str | None = None
    # LLM configuration for knowledge graph generation.
    llm_model: str | None = None
    llm_provider: str | None = None
    # Embedding configuration.
    embedding_model: str | None = None
    embedding_provider: str | None = None
    embedding_dimension: int | None = None
    # Vision LLM for PDF image extraction (SPEC-041). Inherits from tenant if not set.
    vision_llm_model: str | None = None
    vision_llm_provider: str | None = None


class WorkspaceUpdate(BaseModel):
    """Request to update a workspace."""

    name: str | None = None
    description: str | None = None
    llm_model: str | None = None
    llm_provider: str | None = None
    embedding_model: str | None = None
    embedding_provider: str | None = None
    embedding_dimension: int | None = None
    vision_llm_model: str | None = None
    vision_llm_provider: str | None = None


class WorkspaceInfo(BaseModel):
    """Workspace summary information."""

    id: str
    name: str
    slug: str | None = None
    description: str | None = None
    tenant_id: str | None = None
    is_active: bool | None = None
    # LLM configuration.
    llm_model: str | None = None
    llm_provider: str | None = None
    llm_full_id: str | None = None
    # Embedding configuration.
    embedding_model: str | None = None
    embedding_provider: str | None = None
    embedding_dimension: int | None = None
    embedding_full_id: str | None = None
    # Vision LLM (SPEC-041) – only present when configured or inherited from tenant.
    vision_llm_model: str | None = None
    vision_llm_provider: str | None = None
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
