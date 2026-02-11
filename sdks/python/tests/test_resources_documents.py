"""Tests for document and PDF resources."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake.types.documents import (
    DeleteAllResponse,
    DeletionImpactResponse,
    DocumentDetail,
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
        from pathlib import Path

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
