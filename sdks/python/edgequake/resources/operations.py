"""Operations resource — Tasks, Pipeline, Costs, Lineage, Settings, Models.

WHY: Groups ancillary API resources that support the core document/query/graph
workflow. Keeps ancillary resources in one file for simplicity (SRP per resource class).

WHY OODA-06: Aliased built-in `list` to `_list` to avoid shadowing by method name.
"""

from __future__ import annotations

from typing import Any
from typing import List as _list

from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.operations import (
    AvailableProviders,
    BudgetInfo,
    BudgetUpdate,
    ChunkDetail,
    ChunkLineageInfo,
    CostEntry,
    CostEstimateRequest,
    CostEstimateResponse,
    CostSummary,
    EntityProvenanceResponse,
    LineageGraph,
    ModelDetail,
    ModelInfo,
    ModelPricing,
    PipelineStatus,
    ProviderDetail,
    ProvidersHealth,
    ProviderStatus,
    QueueMetrics,
    TaskInfo,
    TaskListResponse,
)
from edgequake.types.workspaces import (
    MetricsHistoryResponse,
    RebuildResponse,
    WorkspaceCreate,
    WorkspaceDetail,
    WorkspaceInfo,
    WorkspaceStats,
    WorkspaceUpdate,
)


class WorkspacesResource(SyncResource):
    """Workspace management operations."""

    def create(self, tenant_id: str, workspace: WorkspaceCreate) -> WorkspaceInfo:
        """Create a workspace.

        POST /api/v1/tenants/{tenant_id}/workspaces
        """
        return self._post(
            f"/api/v1/tenants/{tenant_id}/workspaces",
            json=workspace.model_dump(exclude_none=True),
            response_type=WorkspaceInfo,
        )

    def list(self, tenant_id: str) -> _list[WorkspaceInfo]:
        """List workspaces for a tenant.

        GET /api/v1/tenants/{tenant_id}/workspaces
        """
        data = self._get(f"/api/v1/tenants/{tenant_id}/workspaces")
        if isinstance(data, list):
            return [WorkspaceInfo.model_validate(w) for w in data]
        items = (
            data.get("workspaces", data.get("items", []))
            if isinstance(data, dict)
            else []
        )
        return [WorkspaceInfo.model_validate(w) for w in items]

    def get(self, workspace_id: str) -> WorkspaceDetail:
        """Get workspace details.

        GET /api/v1/workspaces/{workspace_id}
        """
        return self._get(
            f"/api/v1/workspaces/{workspace_id}",
            response_type=WorkspaceDetail,
        )

    def get_by_slug(self, tenant_id: str, slug: str) -> WorkspaceInfo:
        """Get workspace by slug.

        GET /api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}
        """
        return self._get(
            f"/api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}",
            response_type=WorkspaceInfo,
        )

    def update(self, workspace_id: str, update: WorkspaceUpdate) -> WorkspaceInfo:
        """Update a workspace.

        PUT /api/v1/workspaces/{workspace_id}
        """
        return self._put(
            f"/api/v1/workspaces/{workspace_id}",
            json=update.model_dump(exclude_none=True),
            response_type=WorkspaceInfo,
        )

    def delete(self, workspace_id: str) -> None:
        """Delete a workspace.

        DELETE /api/v1/workspaces/{workspace_id}
        """
        self._delete(f"/api/v1/workspaces/{workspace_id}")

    def stats(self, workspace_id: str) -> WorkspaceStats:
        """Get workspace statistics.

        GET /api/v1/workspaces/{workspace_id}/stats
        """
        return self._get(
            f"/api/v1/workspaces/{workspace_id}/stats",
            response_type=WorkspaceStats,
        )

    def metrics_history(self, workspace_id: str) -> MetricsHistoryResponse:
        """Get metrics history.

        GET /api/v1/workspaces/{workspace_id}/metrics-history
        """
        return self._get(
            f"/api/v1/workspaces/{workspace_id}/metrics-history",
            response_type=MetricsHistoryResponse,
        )

    def trigger_metrics_snapshot(self, workspace_id: str) -> dict[str, Any]:
        """Trigger a metrics snapshot.

        POST /api/v1/workspaces/{workspace_id}/metrics-snapshot
        """
        return self._post(f"/api/v1/workspaces/{workspace_id}/metrics-snapshot")

    def rebuild_embeddings(self, workspace_id: str) -> RebuildResponse:
        """Rebuild embeddings for a workspace.

        POST /api/v1/workspaces/{workspace_id}/rebuild-embeddings
        """
        return self._post(
            f"/api/v1/workspaces/{workspace_id}/rebuild-embeddings",
            response_type=RebuildResponse,
        )

    def rebuild_knowledge_graph(self, workspace_id: str) -> RebuildResponse:
        """Rebuild knowledge graph for a workspace.

        POST /api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph
        """
        return self._post(
            f"/api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph",
            response_type=RebuildResponse,
        )

    def reprocess_documents(self, workspace_id: str) -> RebuildResponse:
        """Reprocess all documents in a workspace.

        POST /api/v1/workspaces/{workspace_id}/reprocess-documents
        """
        return self._post(
            f"/api/v1/workspaces/{workspace_id}/reprocess-documents",
            response_type=RebuildResponse,
        )

    def put_injection(
        self, workspace_id: str, body: dict[str, Any]
    ) -> dict[str, Any]:
        """PUT /api/v1/workspaces/{workspace_id}/injection"""
        return self._put(
            f"/api/v1/workspaces/{workspace_id}/injection",
            json=body,
        )

    def put_injection_file(
        self,
        workspace_id: str,
        *,
        name: str,
        file: Any,
        filename: str | None = None,
    ) -> dict[str, Any]:
        """PUT /api/v1/workspaces/{workspace_id}/injection/file (multipart)."""
        response = self._transport.put_upload(
            f"/api/v1/workspaces/{workspace_id}/injection/file",
            file=file,
            filename=filename,
            metadata={"name": name},
        )
        return response.json()

    def list_injections(self, workspace_id: str) -> dict[str, Any]:
        return self._get(f"/api/v1/workspaces/{workspace_id}/injections")

    def get_injection(self, workspace_id: str, injection_id: str) -> dict[str, Any]:
        return self._get(
            f"/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
        )

    def patch_injection(
        self, workspace_id: str, injection_id: str, body: dict[str, Any]
    ) -> dict[str, Any]:
        response = self._transport.request(
            "PATCH",
            f"/api/v1/workspaces/{workspace_id}/injections/{injection_id}",
            json=body,
        )
        return response.json()

    def delete_injection(self, workspace_id: str, injection_id: str) -> None:
        self._delete(
            f"/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
        )


class TasksResource(SyncResource):
    """Async task management."""

    def get(self, track_id: str) -> TaskInfo:
        """Get task status.

        GET /api/v1/tasks/{track_id}
        """
        return self._get(f"/api/v1/tasks/{track_id}", response_type=TaskInfo)

    def list(self) -> TaskListResponse:
        """List all tasks.

        GET /api/v1/tasks
        """
        return self._get("/api/v1/tasks", response_type=TaskListResponse)

    def cancel(self, track_id: str) -> None:
        """Cancel a task.

        POST /api/v1/tasks/{track_id}/cancel
        """
        self._post(f"/api/v1/tasks/{track_id}/cancel")

    def retry(self, track_id: str) -> TaskInfo:
        """Retry a failed task.

        POST /api/v1/tasks/{track_id}/retry
        """
        return self._post(f"/api/v1/tasks/{track_id}/retry", response_type=TaskInfo)


class PipelineResource(SyncResource):
    """Pipeline management."""

    def status(self) -> PipelineStatus:
        """Get pipeline status.

        GET /api/v1/pipeline/status
        """
        return self._get("/api/v1/pipeline/status", response_type=PipelineStatus)

    def cancel(self) -> dict[str, Any]:
        """Cancel pipeline processing.

        POST /api/v1/pipeline/cancel
        """
        return self._post("/api/v1/pipeline/cancel")

    def queue_metrics(self) -> QueueMetrics:
        """Get queue metrics.

        GET /api/v1/pipeline/queue-metrics
        """
        return self._get("/api/v1/pipeline/queue-metrics", response_type=QueueMetrics)

    def pricing(self) -> ModelPricing:
        """Get model pricing.

        GET /api/v1/pipeline/costs/pricing
        """
        return self._get("/api/v1/pipeline/costs/pricing", response_type=ModelPricing)

    def estimate_cost(
        self,
        content_length: int,
        *,
        model: str | None = None,
        provider: str | None = None,
    ) -> CostEstimateResponse:
        """Estimate processing cost.

        POST /api/v1/pipeline/costs/estimate
        """
        body = CostEstimateRequest(
            content_length=content_length, model=model, provider=provider
        )
        return self._post(
            "/api/v1/pipeline/costs/estimate",
            json=body.model_dump(exclude_none=True),
            response_type=CostEstimateResponse,
        )


class CostsResource(SyncResource):
    """Cost tracking operations."""

    def summary(self) -> CostSummary:
        """Get cost summary.

        GET /api/v1/costs/summary
        """
        return self._get("/api/v1/costs/summary", response_type=CostSummary)

    def history(self, *, days: int = 30) -> _list[CostEntry]:
        """Get cost history.

        GET /api/v1/costs/history
        """
        data = self._get("/api/v1/costs/history", params={"days": days})
        if isinstance(data, list):
            return [CostEntry.model_validate(e) for e in data]
        items = (
            data.get("entries", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [CostEntry.model_validate(e) for e in items]

    def budget(self) -> BudgetInfo:
        """Get budget status.

        GET /api/v1/costs/budget
        """
        return self._get("/api/v1/costs/budget", response_type=BudgetInfo)

    def update_budget(self, update: BudgetUpdate) -> BudgetInfo:
        """Update budget settings.

        PATCH /api/v1/costs/budget
        """
        response = self._transport.request(
            "PATCH",
            "/api/v1/costs/budget",
            json=update.model_dump(exclude_none=True),
        )
        return BudgetInfo.model_validate(response.json())


class LineageResource(SyncResource):
    """Lineage tracing operations."""

    def entity(self, entity_name: str) -> LineageGraph:
        """Get entity lineage.

        GET /api/v1/lineage/entities/{entity_name}
        """
        return self._get(
            f"/api/v1/lineage/entities/{entity_name}",
            response_type=LineageGraph,
        )

    def document(self, document_id: str) -> LineageGraph:
        """Get document lineage.

        GET /api/v1/lineage/documents/{document_id}
        """
        return self._get(
            f"/api/v1/lineage/documents/{document_id}",
            response_type=LineageGraph,
        )


class ChunksResource(SyncResource):
    """Chunk access operations."""

    def get(self, chunk_id: str) -> ChunkDetail:
        """Get chunk detail.

        GET /api/v1/chunks/{chunk_id}
        """
        return self._get(f"/api/v1/chunks/{chunk_id}", response_type=ChunkDetail)

    def get_lineage(self, chunk_id: str) -> ChunkLineageInfo:
        """Get chunk lineage with parent document refs and position info.

        GET /api/v1/chunks/{chunk_id}/lineage

        @implements F3 — Every chunk contains parent_document_id and position info.
        @implements F8 — PDF → Document → Chunk → Entity chain traceable.
        """
        return self._get(
            f"/api/v1/chunks/{chunk_id}/lineage",
            response_type=ChunkLineageInfo,
        )


class ProvenanceResource(SyncResource):
    """Entity provenance operations."""

    def get(self, entity_id: str) -> EntityProvenanceResponse:
        """Get entity provenance.

        GET /api/v1/entities/{entity_id}/provenance
        """
        return self._get(
            f"/api/v1/entities/{entity_id}/provenance",
            response_type=EntityProvenanceResponse,
        )


class SettingsResource(SyncResource):
    """Settings operations."""

    def provider_status(self) -> ProviderStatus:
        """Get current provider status.

        GET /api/v1/settings/provider/status
        """
        return self._get(
            "/api/v1/settings/provider/status",
            response_type=ProviderStatus,
        )

    def providers(self) -> AvailableProviders:
        """List available providers.

        GET /api/v1/settings/providers
        """
        return self._get(
            "/api/v1/settings/providers",
            response_type=AvailableProviders,
        )


class ModelsResource(SyncResource):
    """Models configuration API."""

    def list(self) -> _list[ModelInfo]:
        """List all models.

        GET /api/v1/models
        """
        data = self._get("/api/v1/models")
        if isinstance(data, list):
            return [ModelInfo.model_validate(m) for m in data]
        items = (
            data.get("models", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [ModelInfo.model_validate(m) for m in items]

    def list_llm(self) -> _list[ModelInfo]:
        """List LLM models.

        GET /api/v1/models/llm
        """
        data = self._get("/api/v1/models/llm")
        if isinstance(data, list):
            return [ModelInfo.model_validate(m) for m in data]
        items = (
            data.get("models", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [ModelInfo.model_validate(m) for m in items]

    def list_embedding(self) -> _list[ModelInfo]:
        """List embedding models.

        GET /api/v1/models/embedding
        """
        data = self._get("/api/v1/models/embedding")
        if isinstance(data, list):
            return [ModelInfo.model_validate(m) for m in data]
        items = (
            data.get("models", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [ModelInfo.model_validate(m) for m in items]

    def health(self) -> ProvidersHealth:
        """Check providers health.

        GET /api/v1/models/health
        """
        # WHY: API returns a list directly, not {providers: [...]}
        data = self._get("/api/v1/models/health")
        if isinstance(data, list):
            return ProvidersHealth(providers=data)
        return ProvidersHealth.model_validate(data)

    def provider(self, provider_name: str) -> ProviderDetail:
        """Get provider details.

        GET /api/v1/models/{provider}
        """
        return self._get(
            f"/api/v1/models/{provider_name}",
            response_type=ProviderDetail,
        )

    def model(self, provider_name: str, model_name: str) -> ModelDetail:
        """Get specific model details.

        GET /api/v1/models/{provider}/{model}
        """
        return self._get(
            f"/api/v1/models/{provider_name}/{model_name}",
            response_type=ModelDetail,
        )


# --- Async Versions ---


class AsyncWorkspacesResource(AsyncResource):
    """Async workspace management (parity with WorkspacesResource)."""

    async def create(self, tenant_id: str, workspace: WorkspaceCreate) -> WorkspaceInfo:
        return await self._post(
            f"/api/v1/tenants/{tenant_id}/workspaces",
            json=workspace.model_dump(exclude_none=True),
            response_type=WorkspaceInfo,
        )

    async def list(self, tenant_id: str) -> _list[WorkspaceInfo]:
        data = await self._get(f"/api/v1/tenants/{tenant_id}/workspaces")
        if isinstance(data, list):
            return [WorkspaceInfo.model_validate(w) for w in data]
        items = (
            data.get("workspaces", data.get("items", []))
            if isinstance(data, dict)
            else []
        )
        return [WorkspaceInfo.model_validate(w) for w in items]

    async def get(self, workspace_id: str) -> WorkspaceDetail:
        return await self._get(
            f"/api/v1/workspaces/{workspace_id}",
            response_type=WorkspaceDetail,
        )

    async def get_by_slug(self, tenant_id: str, slug: str) -> WorkspaceInfo:
        return await self._get(
            f"/api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}",
            response_type=WorkspaceInfo,
        )

    async def update(self, workspace_id: str, update: WorkspaceUpdate) -> WorkspaceInfo:
        return await self._put(
            f"/api/v1/workspaces/{workspace_id}",
            json=update.model_dump(exclude_none=True),
            response_type=WorkspaceInfo,
        )

    async def delete(self, workspace_id: str) -> None:
        await self._delete(f"/api/v1/workspaces/{workspace_id}")

    async def stats(self, workspace_id: str) -> WorkspaceStats:
        return await self._get(
            f"/api/v1/workspaces/{workspace_id}/stats",
            response_type=WorkspaceStats,
        )

    async def metrics_history(self, workspace_id: str) -> MetricsHistoryResponse:
        return await self._get(
            f"/api/v1/workspaces/{workspace_id}/metrics-history",
            response_type=MetricsHistoryResponse,
        )

    async def trigger_metrics_snapshot(self, workspace_id: str) -> dict[str, Any]:
        return await self._post(
            f"/api/v1/workspaces/{workspace_id}/metrics-snapshot"
        )

    async def rebuild_embeddings(self, workspace_id: str) -> RebuildResponse:
        return await self._post(
            f"/api/v1/workspaces/{workspace_id}/rebuild-embeddings",
            response_type=RebuildResponse,
        )

    async def rebuild_knowledge_graph(self, workspace_id: str) -> RebuildResponse:
        return await self._post(
            f"/api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph",
            response_type=RebuildResponse,
        )

    async def reprocess_documents(self, workspace_id: str) -> RebuildResponse:
        return await self._post(
            f"/api/v1/workspaces/{workspace_id}/reprocess-documents",
            response_type=RebuildResponse,
        )

    async def put_injection(
        self, workspace_id: str, body: dict[str, Any]
    ) -> dict[str, Any]:
        return await self._put(
            f"/api/v1/workspaces/{workspace_id}/injection",
            json=body,
        )

    async def put_injection_file(
        self,
        workspace_id: str,
        *,
        name: str,
        file: Any,
        filename: str | None = None,
    ) -> dict[str, Any]:
        response = await self._transport.put_upload(
            f"/api/v1/workspaces/{workspace_id}/injection/file",
            file=file,
            filename=filename,
            metadata={"name": name},
        )
        return response.json()

    async def list_injections(self, workspace_id: str) -> dict[str, Any]:
        return await self._get(f"/api/v1/workspaces/{workspace_id}/injections")

    async def get_injection(
        self, workspace_id: str, injection_id: str
    ) -> dict[str, Any]:
        return await self._get(
            f"/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
        )

    async def patch_injection(
        self, workspace_id: str, injection_id: str, body: dict[str, Any]
    ) -> dict[str, Any]:
        response = await self._transport.request(
            "PATCH",
            f"/api/v1/workspaces/{workspace_id}/injections/{injection_id}",
            json=body,
        )
        return response.json()

    async def delete_injection(self, workspace_id: str, injection_id: str) -> None:
        await self._delete(
            f"/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
        )


class AsyncTasksResource(AsyncResource):
    """Async task management."""

    async def get(self, track_id: str) -> TaskInfo:
        return await self._get(f"/api/v1/tasks/{track_id}", response_type=TaskInfo)

    async def list(self) -> TaskListResponse:
        return await self._get("/api/v1/tasks", response_type=TaskListResponse)

    async def cancel(self, track_id: str) -> None:
        await self._post(f"/api/v1/tasks/{track_id}/cancel")

    async def retry(self, track_id: str) -> TaskInfo:
        return await self._post(
            f"/api/v1/tasks/{track_id}/retry", response_type=TaskInfo
        )


class AsyncPipelineResource(AsyncResource):
    """Async pipeline management."""

    async def status(self) -> PipelineStatus:
        return await self._get("/api/v1/pipeline/status", response_type=PipelineStatus)

    async def cancel(self) -> dict[str, Any]:
        return await self._post("/api/v1/pipeline/cancel")

    async def queue_metrics(self) -> QueueMetrics:
        return await self._get(
            "/api/v1/pipeline/queue-metrics", response_type=QueueMetrics
        )


class AsyncCostsResource(AsyncResource):
    """Async cost tracking."""

    async def summary(self) -> CostSummary:
        return await self._get("/api/v1/costs/summary", response_type=CostSummary)

    async def history(self, *, days: int = 30) -> _list[CostEntry]:
        data = await self._get("/api/v1/costs/history", params={"days": days})
        if isinstance(data, list):
            return [CostEntry.model_validate(e) for e in data]
        items = (
            data.get("entries", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [CostEntry.model_validate(e) for e in items]

    async def budget(self) -> BudgetInfo:
        return await self._get("/api/v1/costs/budget", response_type=BudgetInfo)

    async def update_budget(self, update: BudgetUpdate) -> BudgetInfo:
        response = await self._transport.request(
            "PATCH",
            "/api/v1/costs/budget",
            json=update.model_dump(exclude_none=True),
        )
        return BudgetInfo.model_validate(response.json())


class AsyncLineageResource(AsyncResource):
    """Async lineage tracing."""

    async def entity(self, entity_name: str) -> LineageGraph:
        return await self._get(
            f"/api/v1/lineage/entities/{entity_name}",
            response_type=LineageGraph,
        )

    async def document(self, document_id: str) -> LineageGraph:
        return await self._get(
            f"/api/v1/lineage/documents/{document_id}",
            response_type=LineageGraph,
        )


class AsyncSettingsResource(AsyncResource):
    """Async settings."""

    async def provider_status(self) -> ProviderStatus:
        return await self._get(
            "/api/v1/settings/provider/status",
            response_type=ProviderStatus,
        )

    async def providers(self) -> AvailableProviders:
        return await self._get(
            "/api/v1/settings/providers",
            response_type=AvailableProviders,
        )


class AsyncChunksResource(AsyncResource):
    """Async chunk access operations."""

    async def get(self, chunk_id: str) -> ChunkDetail:
        """Get chunk detail.

        GET /api/v1/chunks/{chunk_id}
        """
        return await self._get(f"/api/v1/chunks/{chunk_id}", response_type=ChunkDetail)

    async def get_lineage(self, chunk_id: str) -> ChunkLineageInfo:
        """Get chunk lineage with parent document refs and position info.

        GET /api/v1/chunks/{chunk_id}/lineage

        @implements F3 — Every chunk contains parent_document_id and position info.
        @implements F8 — PDF → Document → Chunk → Entity chain traceable.
        """
        return await self._get(
            f"/api/v1/chunks/{chunk_id}/lineage",
            response_type=ChunkLineageInfo,
        )


class AsyncProvenanceResource(AsyncResource):
    """Async entity provenance operations."""

    async def get(self, entity_id: str) -> EntityProvenanceResponse:
        """Get entity provenance.

        GET /api/v1/entities/{entity_id}/provenance
        """
        return await self._get(
            f"/api/v1/entities/{entity_id}/provenance",
            response_type=EntityProvenanceResponse,
        )


class AsyncModelsResource(AsyncResource):
    """Async models API."""

    async def list(self) -> _list[ModelInfo]:
        data = await self._get("/api/v1/models")
        if isinstance(data, list):
            return [ModelInfo.model_validate(m) for m in data]
        items = (
            data.get("models", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [ModelInfo.model_validate(m) for m in items]

    async def list_llm(self) -> _list[ModelInfo]:
        data = await self._get("/api/v1/models/llm")
        if isinstance(data, list):
            return [ModelInfo.model_validate(m) for m in data]
        items = (
            data.get("models", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [ModelInfo.model_validate(m) for m in items]

    async def health(self) -> ProvidersHealth:
        # WHY: API returns a list directly, not {providers: [...]}
        data = await self._get("/api/v1/models/health")
        if isinstance(data, list):
            return ProvidersHealth(providers=data)
        return ProvidersHealth.model_validate(data)

    async def provider(self, provider_name: str) -> ProviderDetail:
        return await self._get(
            f"/api/v1/models/{provider_name}",
            response_type=ProviderDetail,
        )
