"""Tests for workspace, task, pipeline, cost, and other operations resources."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake.types.operations import (
    AvailableProviders,
    BudgetInfo,
    BudgetUpdate,
    ChunkDetail,
    CostEntry,
    CostEstimateResponse,
    CostSummary,
    LineageGraph,
    ModelDetail,
    ModelInfo,
    PipelineStatus,
    ProvenanceRecord,
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


class TestWorkspacesResource:
    """Test sync WorkspacesResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "ws-1",
            "name": "Test Workspace",
            "slug": "test-workspace",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.create(
            "tenant-1",
            WorkspaceCreate(name="Test Workspace"),
        )
        assert isinstance(result, WorkspaceInfo)
        assert result.name == "Test Workspace"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "ws-1", "name": "Default", "slug": "default"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.list("tenant-1")
        assert isinstance(result, list)
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "workspaces": [{"id": "ws-1", "name": "Default", "slug": "default"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.list("t-1")
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "ws-1",
            "name": "Default",
            "slug": "default",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.get("ws-1")
        assert isinstance(result, WorkspaceDetail)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get_by_slug(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "ws-1",
            "name": "Default",
            "slug": "default",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.get_by_slug("t-1", "default")
        assert isinstance(result, WorkspaceInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "ws-1",
            "name": "Updated",
            "slug": "updated",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.update("ws-1", WorkspaceUpdate(name="Updated"))
        assert isinstance(result, WorkspaceInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.workspaces.delete("ws-1")
        mock_req.assert_called_once()
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_stats(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "workspace_id": "ws-1",
            "document_count": 10,
            "entity_count": 50,
            "relationship_count": 30,
            "chunk_count": 100,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.stats("ws-1")
        assert isinstance(result, WorkspaceStats)
        assert result.document_count == 10
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_metrics_history(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"workspace_id": "ws-1", "entries": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.metrics_history("ws-1")
        assert isinstance(result, MetricsHistoryResponse)
        assert result.workspace_id == "ws-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_trigger_metrics_snapshot(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"status": "ok"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.trigger_metrics_snapshot("ws-1")
        assert result == {"status": "ok"}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_rebuild_embeddings(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "status": "started",
            "track_id": "t-1",
            "message": "Rebuilding embeddings",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.rebuild_embeddings("ws-1")
        assert isinstance(result, RebuildResponse)
        assert result.status == "started"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_rebuild_knowledge_graph(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "status": "started",
            "track_id": "t-2",
            "message": "Rebuilding knowledge graph",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.rebuild_knowledge_graph("ws-1")
        assert isinstance(result, RebuildResponse)
        assert result.status == "started"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_reprocess_documents(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "status": "started",
            "track_id": "t-3",
            "message": "Reprocessing documents",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.workspaces.reprocess_documents("ws-1")
        assert isinstance(result, RebuildResponse)
        assert result.status == "started"
        client.close()


class TestTasksResource:
    """Test sync TasksResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "track_id": "task-1",
            "status": "running",
            "task_type": "entity_extraction",
            "progress": 0.5,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tasks.get("task-1")
        assert isinstance(result, TaskInfo)
        assert result.status == "running"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "tasks": [{"track_id": "t-1", "status": "completed"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tasks.list()
        assert isinstance(result, TaskListResponse)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_cancel(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tasks.cancel("task-1")
        assert result is None
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_retry(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "track_id": "task-1",
            "status": "pending",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tasks.retry("task-1")
        assert isinstance(result, TaskInfo)
        client.close()


class TestPipelineResource:
    """Test sync PipelineResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_status(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "status": "idle",
            "active_tasks": 0,
            "queued_tasks": 0,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pipeline.status()
        assert isinstance(result, PipelineStatus)
        assert result.status == "idle"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_cancel(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"message": "Cancelled"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pipeline.cancel()
        assert result == {"message": "Cancelled"}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_queue_metrics(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "queue_depth": 0,
            "processing": 0,
            "completed_last_hour": 100,
            "failed_last_hour": 2,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pipeline.queue_metrics()
        assert isinstance(result, QueueMetrics)
        assert result.completed_last_hour == 100
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_estimate_cost(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "estimated_cost_usd": 0.05,
            "estimated_tokens": 1000,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pipeline.estimate_cost(
            content_length=5000, model="gpt-4", provider="openai"
        )
        assert isinstance(result, CostEstimateResponse)
        client.close()


class TestCostsResource:
    """Test sync CostsResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_summary(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "total_cost_usd": 1.50,
            "total_tokens": 10000,
            "total_input_tokens": 6000,
            "total_output_tokens": 4000,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.costs.summary()
        assert isinstance(result, CostSummary)
        assert result.total_cost_usd == 1.50
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_history(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"date": "2024-01-01", "cost_usd": 0.50, "tokens": 5000},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.costs.history(days=7)
        assert isinstance(result, list)
        assert len(result) == 1
        assert isinstance(result[0], CostEntry)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_history_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "entries": [{"date": "2024-01-01", "cost_usd": 0.50}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.costs.history()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_budget(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "monthly_limit_usd": 100.0,
            "current_spend_usd": 15.0,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.costs.budget()
        assert isinstance(result, BudgetInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update_budget(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "monthly_limit_usd": 200.0,
            "current_spend_usd": 15.0,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.costs.update_budget(BudgetUpdate(monthly_limit_usd=200.0))
        assert isinstance(result, BudgetInfo)
        client.close()


class TestLineageResource:
    """Test sync LineageResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_entity(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "edges": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.lineage.entity("ALICE")
        assert isinstance(result, LineageGraph)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_document(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "edges": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.lineage.document("doc-1")
        assert isinstance(result, LineageGraph)
        client.close()


class TestChunksResource:
    """Test sync ChunksResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "chunk-1",
            "document_id": "doc-1",
            "content": "This is a chunk of text.",
            "chunk_index": 0,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.chunks.get("chunk-1")
        assert isinstance(result, ChunkDetail)
        assert result.content == "This is a chunk of text."
        client.close()


class TestProvenanceResource:
    """Test sync ProvenanceResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {
                "chunk_id": "chunk-1",
                "document_id": "doc-1",
                "extraction_method": "llm",
            }
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.provenance.get("ent-1")
        assert isinstance(result, list)
        assert len(result) == 1
        assert isinstance(result[0], ProvenanceRecord)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "records": [{"chunk_id": "c1", "document_id": "d1"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.provenance.get("ent-1")
        assert len(result) == 1
        client.close()


class TestSettingsResource:
    """Test sync SettingsResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_provider_status(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "provider": "ollama",
            "status": "available",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.settings.provider_status()
        assert isinstance(result, ProviderStatus)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_providers(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "providers": [
                {"name": "ollama", "is_available": True},
                {"name": "openai", "is_available": True},
            ],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.settings.providers()
        assert isinstance(result, AvailableProviders)
        client.close()


class TestModelsResource:
    """Test sync ModelsResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"name": "gpt-4", "provider": "openai"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.list()
        assert isinstance(result, list)
        assert len(result) == 1
        assert isinstance(result[0], ModelInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "models": [{"name": "gpt-4", "provider": "openai"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.list()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_llm(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"name": "gpt-4", "provider": "openai"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.list_llm()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_embedding(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"name": "text-embedding-3-small", "provider": "openai"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.list_embedding()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_health_list_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"name": "ollama", "status": "healthy", "enabled": True},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.health()
        assert isinstance(result, ProvidersHealth)
        assert len(result.providers) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_health_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "providers": [{"name": "ollama", "status": "healthy", "enabled": True}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.health()
        assert isinstance(result, ProvidersHealth)
        assert len(result.providers) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_provider(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "name": "openai",
            "models": [{"name": "gpt-4"}],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.provider("openai")
        assert isinstance(result, ProviderDetail)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_model(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "name": "gpt-4",
            "provider": "openai",
            "type": "llm",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.models.model("openai", "gpt-4")
        assert isinstance(result, ModelDetail)
        client.close()


# --- Async Tests ---


class TestAsyncWorkspacesResource:
    """Test async WorkspacesResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "ws-1",
            "name": "Test",
            "slug": "test",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.workspaces.create("t-1", WorkspaceCreate(name="Test"))
        assert isinstance(result, WorkspaceInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "ws-1", "name": "Default", "slug": "d"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.workspaces.list("t-1")
        assert len(result) == 1

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "ws-1", "name": "Default", "slug": "d"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.workspaces.get("ws-1")
        assert isinstance(result, WorkspaceDetail)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_update(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "ws-1", "name": "Up", "slug": "up"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.workspaces.update("ws-1", WorkspaceUpdate(name="Up"))
        assert isinstance(result, WorkspaceInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.workspaces.delete("ws-1")
        mock_req.assert_called_once()

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_stats(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "workspace_id": "ws-1",
            "document_count": 5,
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.workspaces.stats("ws-1")
        assert isinstance(result, WorkspaceStats)


class TestAsyncTasksResource:
    """Test async TasksResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"track_id": "t-1", "status": "running"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.tasks.get("t-1")
        assert isinstance(result, TaskInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"tasks": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.tasks.list()
        assert isinstance(result, TaskListResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_cancel(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.tasks.cancel("t-1")

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_retry(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"track_id": "t-1", "status": "pending"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.tasks.retry("t-1")
        assert isinstance(result, TaskInfo)


class TestAsyncPipelineResource:
    """Test async PipelineResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_status(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"status": "idle"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pipeline.status()
        assert isinstance(result, PipelineStatus)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_cancel(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"message": "ok"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pipeline.cancel()
        assert result == {"message": "ok"}

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_queue_metrics(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"queue_depth": 0}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pipeline.queue_metrics()
        assert isinstance(result, QueueMetrics)


class TestAsyncCostsResource:
    """Test async CostsResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_summary(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"total_cost_usd": 2.0}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.costs.summary()
        assert isinstance(result, CostSummary)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_history(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"date": "2024-01-01", "cost_usd": 0.5}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.costs.history(days=7)
        assert len(result) == 1
        assert isinstance(result[0], CostEntry)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_budget(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"monthly_limit_usd": 100.0}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.costs.budget()
        assert isinstance(result, BudgetInfo)


class TestAsyncLineageResource:
    """Test async LineageResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_entity(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "edges": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.lineage.entity("ALICE")
        assert isinstance(result, LineageGraph)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_document(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"nodes": [], "edges": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.lineage.document("doc-1")
        assert isinstance(result, LineageGraph)


class TestAsyncSettingsResource:
    """Test async SettingsResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_provider_status(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"provider": "ollama", "status": "ok"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.settings.provider_status()
        assert isinstance(result, ProviderStatus)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_providers(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "providers": [{"name": "ollama", "is_available": True}]
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.settings.providers()
        assert isinstance(result, AvailableProviders)


class TestAsyncChunksResource:
    """Test async ChunksResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "c-1",
            "document_id": "d-1",
            "content": "chunk text",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.chunks.get("c-1")
        assert isinstance(result, ChunkDetail)


class TestAsyncProvenanceResource:
    """Test async ProvenanceResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"chunk_id": "c-1", "document_id": "d-1"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.provenance.get("e-1")
        assert len(result) == 1
        assert isinstance(result[0], ProvenanceRecord)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get_dict_response(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"records": [{"chunk_id": "c-1"}]}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.provenance.get("e-1")
        assert len(result) == 1


class TestAsyncModelsResource:
    """Test async ModelsResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"name": "gpt-4", "provider": "openai"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.models.list()
        assert len(result) == 1

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list_llm(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"name": "gpt-4"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.models.list_llm()
        assert len(result) == 1

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_health_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"name": "ollama", "status": "ok", "enabled": True}
        ]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.models.health()
        assert isinstance(result, ProvidersHealth)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_provider(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"name": "openai", "models": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.models.provider("openai")
        assert isinstance(result, ProviderDetail)
