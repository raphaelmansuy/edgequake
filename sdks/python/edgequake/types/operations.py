"""Task, Pipeline, Cost, and Lineage type definitions for the EdgeQuake Python SDK.

WHY: Groups ancillary API types used by supporting resource APIs — tasks,
pipeline management, cost tracking, and lineage tracing.
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field

# --- Task types ---


class TaskProgress(BaseModel):
    """Task progress details."""

    current_step: str | None = None
    percent_complete: float | int | None = None
    total_steps: int | None = None


class TaskInfo(BaseModel):
    """Async task information from GET /api/v1/tasks/{track_id}."""

    track_id: str
    status: str
    # WHY: API returns progress as dict with current_step, percent_complete, total_steps
    progress: TaskProgress | dict[str, Any] | float | None = None
    message: str | None = None
    document_id: str | None = None
    task_type: str | None = None
    created_at: str | None = None
    updated_at: str | None = None
    completed_at: str | None = None
    error: str | None = None
    result: dict[str, Any] | None = None
    # WHY: API also returns these fields
    tenant_id: str | None = None
    workspace_id: str | None = None
    started_at: str | None = None
    error_message: str | None = None
    retry_count: int | None = None
    max_retries: int | None = None
    metadata: dict[str, Any] | None = None


class TaskListResponse(BaseModel):
    """Response from GET /api/v1/tasks."""

    tasks: list[TaskInfo] = Field(default_factory=list)
    total: int = 0


# --- Pipeline types ---


class PipelineStatus(BaseModel):
    """Response from GET /api/v1/pipeline/status."""

    # WHY: API returns is_busy, total_documents, etc. instead of status/active_tasks
    is_busy: bool = False
    total_documents: int = 0
    processed_documents: int = 0
    current_batch: int = 0
    total_batches: int = 0
    history_messages: list[str] = Field(default_factory=list)
    cancellation_requested: bool = False
    pending_tasks: int = 0
    processing_tasks: int = 0
    completed_tasks: int = 0
    failed_tasks: int = 0
    # Legacy fields for backward compatibility
    status: str | None = None
    active_tasks: int | None = None
    queued_tasks: int | None = None
    worker_count: int | None = None
    uptime_seconds: int | None = None


class QueueMetrics(BaseModel):
    """Response from GET /api/v1/pipeline/queue-metrics."""

    queue_depth: int = 0
    processing: int = 0
    completed_last_hour: int = 0
    failed_last_hour: int = 0
    avg_processing_time_ms: float | None = None
    estimated_wait_time_ms: float | None = None


class ModelPricing(BaseModel):
    """Response from GET /api/v1/pipeline/costs/pricing."""

    models: list[ModelPriceInfo] = Field(default_factory=list)


class ModelPriceInfo(BaseModel):
    """Pricing info for a single model."""

    provider: str
    model: str
    input_cost_per_1k: float | None = None
    output_cost_per_1k: float | None = None
    currency: str = "USD"


class CostEstimateRequest(BaseModel):
    """Request body for POST /api/v1/pipeline/costs/estimate."""

    content_length: int
    model: str | None = None
    provider: str | None = None


class CostEstimateResponse(BaseModel):
    """Response from cost estimate endpoint."""

    estimated_cost_usd: float | None = None
    estimated_tokens: int | None = None
    model: str | None = None
    provider: str | None = None


# WHY: Rebuild forward references
ModelPricing.model_rebuild()


# --- Cost types ---


class CostSummary(BaseModel):
    """Response from GET /api/v1/costs/summary."""

    total_cost_usd: float = 0.0
    total_tokens: int = 0
    total_input_tokens: int = 0
    total_output_tokens: int = 0
    document_count: int = 0
    query_count: int = 0
    period: str | None = None
    by_model: list[ModelCostBreakdown] | None = None
    by_provider: list[ProviderCostBreakdown] | None = None


class ModelCostBreakdown(BaseModel):
    """Cost breakdown by model."""

    model: str
    provider: str | None = None
    cost_usd: float = 0.0
    token_count: int = 0
    request_count: int = 0


class ProviderCostBreakdown(BaseModel):
    """Cost breakdown by provider."""

    provider: str
    cost_usd: float = 0.0
    token_count: int = 0
    request_count: int = 0


class CostEntry(BaseModel):
    """A single cost history entry."""

    date: str
    cost_usd: float = 0.0
    tokens: int = 0
    requests: int = 0
    model: str | None = None
    provider: str | None = None


class BudgetInfo(BaseModel):
    """Response from GET /api/v1/costs/budget."""

    monthly_budget_usd: float | None = None
    current_spend_usd: float = 0.0
    remaining_usd: float | None = None
    utilization_pct: float | None = None
    alert_threshold_pct: float | None = None
    period_start: str | None = None
    period_end: str | None = None


class BudgetUpdate(BaseModel):
    """Request to update budget settings."""

    monthly_budget_usd: float | None = None
    alert_threshold_pct: float | None = None


# WHY: Rebuild forward references
CostSummary.model_rebuild()


# --- Lineage types ---


class LineageNode(BaseModel):
    """A node in a lineage trace."""

    id: str
    name: str | None = None
    node_type: str | None = None
    properties: dict[str, Any] | None = None


class LineageEdge(BaseModel):
    """An edge in a lineage trace."""

    source: str
    target: str
    relationship: str | None = None
    metadata: dict[str, Any] | None = None


class LineageGraph(BaseModel):
    """Response from lineage endpoints."""

    nodes: list[LineageNode] = Field(default_factory=list)
    edges: list[LineageEdge] = Field(default_factory=list)
    root_id: str | None = None


# --- Document Full Lineage (OODA-16) ---


class DocumentFullLineage(BaseModel):
    """Complete document lineage from GET /documents/:id/lineage.

    WHY: Returns persisted DocumentLineage + document metadata in a single call.
    @implements F5 — Single API call retrieves complete lineage tree.
    """

    document_id: str
    metadata: dict[str, Any] | None = None
    lineage: dict[str, Any] | None = None


class ChunkLineageInfo(BaseModel):
    """Chunk lineage from GET /chunks/:id/lineage.

    WHY: Lightweight chunk lineage with parent document refs and position info.
    @implements F3 — Every chunk contains parent_document_id and position info.
    @implements F8 — PDF → Document → Chunk → Entity chain traceable.
    """

    chunk_id: str
    document_id: str | None = None
    document_name: str | None = None
    document_type: str | None = None
    index: int | None = None
    start_line: int | None = None
    end_line: int | None = None
    start_offset: int | None = None
    end_offset: int | None = None
    token_count: int | None = None
    content_preview: str | None = None
    entity_count: int | None = None
    relationship_count: int | None = None
    entity_names: list[str] = Field(default_factory=list)
    document_metadata: dict[str, Any] | None = None


# --- Chunk types ---


class ChunkDetail(BaseModel):
    """Response from GET /api/v1/chunks/{chunk_id}."""

    id: str
    document_id: str | None = None
    content: str | None = None
    chunk_index: int | None = None
    token_count: int | None = None
    embedding_model: str | None = None
    metadata: dict[str, Any] | None = None


# --- Provenance types ---


class ProvenanceRecord(BaseModel):
    """A provenance record for an entity."""

    entity_id: str | None = None
    entity_name: str | None = None
    document_id: str | None = None
    document_title: str | None = None
    chunk_id: str | None = None
    extraction_method: str | None = None
    confidence: float | None = None
    created_at: str | None = None


# --- Settings types ---


class ProviderStatus(BaseModel):
    """Response from GET /api/v1/settings/provider/status."""

    current_provider: str | None = None
    current_model: str | None = None
    embedding_provider: str | None = None
    embedding_model: str | None = None
    status: str | None = None


class AvailableProviders(BaseModel):
    """Response from GET /api/v1/settings/providers."""

    providers: list[ProviderInfo] = Field(default_factory=list)


class ProviderInfo(BaseModel):
    """Information about an available provider."""

    name: str
    display_name: str | None = None
    is_available: bool = False
    is_current: bool = False
    models: list[str] = Field(default_factory=list)


# WHY: Rebuild forward references
AvailableProviders.model_rebuild()


# --- Models types ---


class ModelInfo(BaseModel):
    """Model information from models API."""

    name: str
    provider: str | None = None
    model_type: str | None = None
    is_available: bool = True
    context_length: int | None = None
    max_tokens: int | None = None


class ModelDetail(ModelInfo):
    """Detailed model info."""

    description: str | None = None
    pricing: ModelPriceInfo | None = None
    capabilities: list[str] | None = None


class ProviderHealthInfo(BaseModel):
    """Health status for a single provider."""

    name: str
    display_name: str | None = None
    provider_type: str | None = None
    enabled: bool = True
    priority: int | None = None
    description: str | None = None
    models: list[dict[str, Any]] | None = None
    # Legacy fields
    status: str | None = None
    latency_ms: float | None = None
    error: str | None = None
    model_count: int | None = None


class ProvidersHealth(BaseModel):
    """Response from GET /api/v1/models/health."""

    providers: list[ProviderHealthInfo] = Field(default_factory=list)


# WHY: Rebuild forward references
ProvidersHealth.model_rebuild()


class ProviderDetail(BaseModel):
    """Detailed provider info from GET /api/v1/models/{provider}."""

    name: str
    display_name: str | None = None
    is_available: bool = False
    models: list[ModelInfo] = Field(default_factory=list)
    base_url: str | None = None
    status: str | None = None
