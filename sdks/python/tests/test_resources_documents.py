"""Tests for document and PDF resources."""

from __future__ import annotations

from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake.types.documents import (
    DeleteAllResponse,
    DeletionImpactResponse,
    DocumentDetail,
    FailedChunkInfo,
    ListDocumentsResponse,
    PdfContentResponse,
    PdfInfo,
    PdfProgressResponse,
    PdfUploadResponse,
    ScanResponse,
    TrackStatusResponse,
    UploadDocumentResponse,
)


class TestDocumentsResource:
    """Test sync DocumentsResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_upload(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "document_id": "doc-1",
            "status": "processing",
            "message": "Upload received",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.upload(content="Hello world")
        assert isinstance(result, UploadDocumentResponse)
        assert result.document_id == "doc-1"
        assert result.status == "processing"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_upload_with_all_params(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"document_id": "doc-2", "status": "processing"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.documents.upload(
            content="Hello",
            title="Test Doc",
            metadata={"source": "test"},
            extract_entities=False,
        )
        body = mock_req.call_args[1]["json"]
        assert body["title"] == "Test Doc"
        assert body["metadata"] == {"source": "test"}
        assert body["extract_entities"] is False
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "documents": [
                {"id": "doc-1", "status": "completed"},
            ],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.list()
        assert isinstance(result, ListDocumentsResponse)
        assert len(result.documents) == 1
        assert result.documents[0].id == "doc-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_with_filters(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"documents": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.documents.list(page=2, page_size=10, status="completed", search="test")
        params = mock_req.call_args[1]["params"]
        assert params["page"] == 2
        assert params["page_size"] == 10
        assert params["status"] == "completed"
        assert params["search"] == "test"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "doc-1",
            "status": "completed",
            "content": "Hello world",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.get("doc-1")
        assert isinstance(result, DocumentDetail)
        assert result.id == "doc-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.documents.delete("doc-1")
        mock_req.assert_called_once()
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete_all(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "deleted_count": 5,
            "message": "All documents deleted",
        }
        mock_resp.status_code = 200
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.delete_all()
        assert isinstance(result, DeleteAllResponse)
        assert result.deleted_count == 5
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_track(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "track_id": "track-1",
            "status": "processing",
            "progress": 0.5,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.track("track-1")
        assert isinstance(result, TrackStatusResponse)
        assert result.track_id == "track-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_scan(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "files_found": 3,
            "files_queued": 1,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.scan("/path/to/dir")
        assert isinstance(result, ScanResponse)
        assert result.files_found == 3
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_scan_with_params(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"files_found": 1, "files_queued": 1}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.documents.scan("/dir", recursive=False, extensions=[".pdf", ".txt"])
        body = mock_req.call_args[1]["json"]
        assert body["recursive"] is False
        assert body["extensions"] == [".pdf", ".txt"]
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_deletion_impact(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "document_id": "doc-1",
            "entity_count": 5,
            "relationship_count": 3,
            "chunk_count": 10,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.deletion_impact("doc-1")
        assert isinstance(result, DeletionImpactResponse)
        assert result.entity_count == 5
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_reprocess_failed(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"queued": 2}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.reprocess_failed()
        assert result == {"queued": 2}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_recover_stuck(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"recovered": 1}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.recover_stuck()
        assert result == {"recovered": 1}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_retry_chunks(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"retried": 3}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.retry_chunks("doc-1")
        assert result == {"retried": 3}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_failed_chunks(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"chunk_id": "c1", "error": "timeout"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.failed_chunks("doc-1")
        assert len(result) == 1
        assert isinstance(result[0], FailedChunkInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_failed_chunks_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "chunks": [{"chunk_id": "c1", "error": "timeout"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.documents.failed_chunks("doc-1")
        assert len(result) == 1
        client.close()


class TestPdfResource:
    """Test sync PdfResource."""

    @patch("edgequake._transport.SyncTransport.upload")
    def test_upload(self, mock_upload: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "pdf-1",
            "status": "processing",
        }
        mock_upload.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.upload(file=Path("/tmp/test.pdf"))
        assert isinstance(result, PdfUploadResponse)
        assert result.id == "pdf-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "pdf-1", "page_count": 10},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.list()
        assert isinstance(result, list)
        assert len(result) == 1
        assert isinstance(result[0], PdfInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"pdfs": [{"id": "pdf-1", "page_count": 5}]}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.list()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "pdf-1",
            "page_count": 5,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.get("pdf-1")
        assert isinstance(result, PdfInfo)
        assert result.page_count == 5
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.pdf.delete("pdf-1")
        mock_req.assert_called_once()
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_download(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.content = b"%PDF-1.4 test content"
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.download("pdf-1")
        assert result == b"%PDF-1.4 test content"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_progress(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "track_id": "track-1",
            "status": "processing",
            "progress": 0.5,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.progress("track-1")
        assert isinstance(result, PdfProgressResponse)
        assert result.progress == 0.5
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_content(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "pdf-1",
            "markdown": "# Test Document\n\nHello world",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.content("pdf-1")
        assert isinstance(result, PdfContentResponse)
        assert "Hello world" in result.markdown
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_retry(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"status": "queued"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.pdf.retry("pdf-1")
        assert result == {"status": "queued"}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_cancel(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.pdf.cancel("pdf-1")
        mock_req.assert_called_once()
        client.close()


# --- Async Tests ---


class TestAsyncDocumentsResource:
    """Test async DocumentsResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_upload(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "document_id": "doc-1",
            "status": "processing",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.upload(content="Hello world")
        assert isinstance(result, UploadDocumentResponse)
        assert result.document_id == "doc-1"

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "documents": [{"id": "doc-1", "status": "completed"}]
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.list()
        assert isinstance(result, ListDocumentsResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "doc-1", "status": "completed"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.get("doc-1")
        assert isinstance(result, DocumentDetail)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.documents.delete("doc-1")
        mock_req.assert_called_once()

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete_all(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"deleted_count": 3}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.delete_all()
        assert result["deleted_count"] == 3

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_track(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "track_id": "t1",
            "status": "processing",
            "progress": 0.8,
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.track("t1")
        assert isinstance(result, TrackStatusResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_scan(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"files_found": 5, "files_queued": 3}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.scan("/path")
        assert isinstance(result, ScanResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_reprocess_failed(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"queued": 2}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.reprocess_failed()
        assert result == {"queued": 2}

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_recover_stuck(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"recovered": 1}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.documents.recover_stuck()
        assert result == {"recovered": 1}


class TestAsyncPdfResource:
    """Test async PdfResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "pdf-1", "page_count": 10}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pdf.list()
        assert len(result) == 1
        assert isinstance(result[0], PdfInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "pdf-1", "page_count": 5}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pdf.get("pdf-1")
        assert isinstance(result, PdfInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.pdf.delete("pdf-1")
        mock_req.assert_called_once()

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_content(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "pdf-1", "markdown": "# Hello"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pdf.content("pdf-1")
        assert isinstance(result, PdfContentResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_progress(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "track_id": "t1",
            "status": "done",
            "progress": 1.0,
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pdf.progress("t1")
        assert isinstance(result, PdfProgressResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_retry(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"status": "queued"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.pdf.retry("pdf-1")
        assert result == {"status": "queued"}

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_cancel(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.pdf.cancel("pdf-1")
        mock_req.assert_called_once()


class TestResourceAccessFromClient:
    """Test that all resources are properly accessible from the client."""

    def test_sync_client_has_all_resources(self) -> None:
        client = EdgeQuake()
        assert client.documents is not None
        assert client.pdf is not None
        assert client.query is not None
        assert client.chat is not None
        assert client.graph is not None
        assert client.entities is not None
        assert client.relationships is not None
        assert client.auth is not None
        assert client.users is not None
        assert client.api_keys is not None
        assert client.tenants is not None
        assert client.workspaces is not None
        assert client.conversations is not None
        assert client.folders is not None
        assert client.tasks is not None
        assert client.pipeline is not None
        assert client.costs is not None
        assert client.lineage is not None
        assert client.chunks is not None
        assert client.provenance is not None
        assert client.settings is not None
        assert client.models is not None
        client.close()

    def test_async_client_has_all_resources(self) -> None:
        client = AsyncEdgeQuake()
        assert client.documents is not None
        assert client.pdf is not None
        assert client.query is not None
        assert client.chat is not None
        assert client.graph is not None
        assert client.entities is not None
        assert client.relationships is not None
        assert client.auth is not None
        assert client.users is not None
        assert client.api_keys is not None
        assert client.tenants is not None
        assert client.workspaces is not None
        assert client.conversations is not None
        assert client.folders is not None
        assert client.tasks is not None
        assert client.pipeline is not None
        assert client.costs is not None
        assert client.lineage is not None
        assert client.chunks is not None
        assert client.provenance is not None
        assert client.settings is not None
        assert client.models is not None

    def test_resources_are_cached(self) -> None:
        """Verify cached_property returns same instance."""
        client = EdgeQuake()
        assert client.documents is client.documents
        assert client.query is client.query
        assert client.graph is client.graph
        client.close()
