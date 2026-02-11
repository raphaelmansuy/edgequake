"""Tests for workspace, task, pipeline, cost, and other operations resources."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from edgequake import EdgeQuake
from edgequake.types.operations import (
    ChunkDetail,
    CostSummary,
    ModelInfo,
    PipelineStatus,
    ProvenanceRecord,
    QueueMetrics,
    TaskInfo,
)
from edgequake.types.workspaces import (
    WorkspaceCreate,
    WorkspaceDetail,
    WorkspaceInfo,
    WorkspaceStats,
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
    def test_cancel(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tasks.cancel("task-1")
        assert result is None
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
