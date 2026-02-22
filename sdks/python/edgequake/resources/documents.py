"""Documents resource — CRUD and ingestion operations for EdgeQuake documents.

WHY: Maps to /api/v1/documents/* endpoints. Supports text upload, file upload,
batch upload, PDF operations, and document lifecycle management.

WHY OODA-06: Aliased built-in `list` to `_list` to avoid shadowing by method name.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, BinaryIO
from typing import List as _list

from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.documents import (
    BatchUploadResponse,
    DeleteAllResponse,
    DeletionImpactResponse,
    DocumentDetail,
    FailedChunkInfo,
    ListDocumentsResponse,
    PdfContentResponse,
    PdfInfo,
    PdfProgressResponse,
    PdfUploadOptions,
    PdfUploadResponse,
    ScanResponse,
    TrackStatusResponse,
    UploadDocumentResponse,
)
from edgequake.types.operations import DocumentFullLineage


class DocumentsResource(SyncResource):
    """Synchronous Documents API.

    Provides document upload, listing, retrieval, deletion, and PDF operations.
    """

    @property
    def pdf(self) -> PdfResource:
        """PDF sub-namespace for PDF-specific operations."""
        return PdfResource(self._transport)

    def upload(
        self,
        content: str,
        *,
        title: str | None = None,
        metadata: dict[str, str] | None = None,
        extract_entities: bool = True,
    ) -> UploadDocumentResponse:
        """Upload a text document.

        POST /api/v1/documents
        """
        body: dict[str, Any] = {
            "content": content,
            "extract_entities": extract_entities,
        }
        if title:
            body["title"] = title
        if metadata:
            body["metadata"] = metadata
        return self._post(
            "/api/v1/documents", json=body, response_type=UploadDocumentResponse
        )

    def list(
        self,
        *,
        page: int = 1,
        page_size: int = 50,
        status: str | None = None,
        search: str | None = None,
    ) -> ListDocumentsResponse:
        """List documents with pagination.

        GET /api/v1/documents
        """
        params: dict[str, Any] = {"page": page, "page_size": page_size}
        if status:
            params["status"] = status
        if search:
            params["search"] = search
        return self._get(
            "/api/v1/documents", params=params, response_type=ListDocumentsResponse
        )

    def get(self, document_id: str) -> DocumentDetail:
        """Get document details.

        GET /api/v1/documents/{document_id}
        """
        return self._get(
            f"/api/v1/documents/{document_id}", response_type=DocumentDetail
        )

    def delete(self, document_id: str) -> None:
        """Delete a document.

        DELETE /api/v1/documents/{document_id}
        """
        self._delete(f"/api/v1/documents/{document_id}")

    def delete_all(self) -> DeleteAllResponse:
        """Delete all documents.

        DELETE /api/v1/documents
        """
        return self._delete_with_response(
            "/api/v1/documents", response_type=DeleteAllResponse
        )

    def track(self, track_id: str) -> TrackStatusResponse:
        """Get processing status by track ID.

        GET /api/v1/documents/track/{track_id}
        """
        return self._get(
            f"/api/v1/documents/track/{track_id}",
            response_type=TrackStatusResponse,
        )

    def upload_file(
        self,
        file: Path | BinaryIO,
        *,
        title: str | None = None,
        metadata: dict[str, str] | None = None,
    ) -> UploadDocumentResponse:
        """Upload a file via multipart/form-data.

        POST /api/v1/documents/upload
        """
        meta = metadata or {}
        if title:
            meta["title"] = title
        response = self._transport.upload(
            "/api/v1/documents/upload", file=file, metadata=meta
        )
        return UploadDocumentResponse.model_validate(response.json())

    def upload_batch(
        self,
        files: _list[Path | BinaryIO],
        *,
        metadata: dict[str, str] | None = None,
    ) -> BatchUploadResponse:
        """Upload multiple files in a batch.

        POST /api/v1/documents/upload/batch
        """
        # WHY: Batch upload sends multiple files in a single request.
        # We iterate and upload individually since the API may not support
        # true multipart batch in all deployments.
        results = []
        for f in files:
            try:
                result = self.upload_file(f, metadata=metadata)
                results.append(result)
            except Exception:
                pass
        return BatchUploadResponse(
            results=results,
            total=len(files),
            success_count=len(results),
            failure_count=len(files) - len(results),
        )

    def scan(
        self,
        path: str,
        *,
        recursive: bool = True,
        extensions: _list[str] | None = None,
    ) -> ScanResponse:
        """Scan a directory for documents to ingest.

        POST /api/v1/documents/scan
        """
        body: dict[str, Any] = {"path": path, "recursive": recursive}
        if extensions:
            body["extensions"] = extensions
        return self._post(
            "/api/v1/documents/scan", json=body, response_type=ScanResponse
        )

    def reprocess_failed(self) -> dict[str, Any]:
        """Reprocess all failed documents.

        POST /api/v1/documents/reprocess
        """
        return self._post("/api/v1/documents/reprocess")

    def recover_stuck(self) -> dict[str, Any]:
        """Recover stuck processing documents.

        POST /api/v1/documents/recover-stuck
        """
        return self._post("/api/v1/documents/recover-stuck")

    def deletion_impact(self, document_id: str) -> DeletionImpactResponse:
        """Analyze impact of deleting a document.

        GET /api/v1/documents/{document_id}/deletion-impact
        """
        return self._get(
            f"/api/v1/documents/{document_id}/deletion-impact",
            response_type=DeletionImpactResponse,
        )

    def retry_chunks(self, document_id: str) -> dict[str, Any]:
        """Retry failed chunks for a document.

        POST /api/v1/documents/{document_id}/retry-chunks
        """
        return self._post(f"/api/v1/documents/{document_id}/retry-chunks")

    def failed_chunks(self, document_id: str) -> _list[FailedChunkInfo]:
        """List failed chunks for a document.

        GET /api/v1/documents/{document_id}/failed-chunks
        """
        data = self._get(f"/api/v1/documents/{document_id}/failed-chunks")
        if isinstance(data, list):
            return [FailedChunkInfo.model_validate(c) for c in data]
        # WHY: Some responses wrap in {"chunks": [...]}
        chunks = data.get("chunks", []) if isinstance(data, dict) else []
        return [FailedChunkInfo.model_validate(c) for c in chunks]

    # ========================================================================
    # Lineage Methods (OODA-16)
    # ========================================================================

    def get_lineage(self, document_id: str) -> DocumentFullLineage:
        """Get complete document lineage.

        GET /api/v1/documents/{document_id}/lineage

        Returns persisted pipeline lineage + document metadata in a single call.
        @implements F5 — Single API call retrieves complete lineage tree.
        """
        return self._get(
            f"/api/v1/documents/{document_id}/lineage",
            response_type=DocumentFullLineage,
        )

    def get_metadata(self, document_id: str) -> dict[str, Any]:
        """Get all document metadata from KV storage.

        GET /api/v1/documents/{document_id}/metadata

        @implements F1 — All document metadata retrievable.
        """
        return self._get(f"/api/v1/documents/{document_id}/metadata")

    def export_lineage(self, document_id: str, *, format: str = "json") -> bytes:
        """Export document lineage as a downloadable file.

        GET /api/v1/documents/{document_id}/lineage/export?format={format}

        WHY: Enables compliance audit trails and data pipeline ingestion.
        Supports 'json' (default) and 'csv' formats.

        Args:
            document_id: Document ID to export lineage for.
            format: Export format — 'json' or 'csv'.

        Returns:
            Raw bytes of the exported file content.
        """
        response = self._transport.request(
            "GET",
            f"/api/v1/documents/{document_id}/lineage/export",
            params={"format": format},
        )
        # WHY: Export returns raw file content, not JSON
        if hasattr(response, "content"):
            return response.content
        return response.read()

    def _delete_with_response(
        self, path: str, *, response_type: type | None = None
    ) -> Any:
        """DELETE that returns a response body (not 204)."""
        response = self._transport.request("DELETE", path)
        if response_type:
            return response_type.model_validate(response.json())
        return response.json()


class PdfResource(SyncResource):
    """PDF sub-resource for PDF-specific document operations."""

    def upload(
        self,
        file: Path | BinaryIO,
        *,
        filename: str | None = None,
        metadata: dict[str, str] | None = None,
        enable_vision: bool = False,
        vision_provider: str | None = None,
        vision_model: str | None = None,
        title: str | None = None,
        track_id: str | None = None,
        force_reindex: bool = False,
        options: PdfUploadOptions | None = None,
    ) -> PdfUploadResponse:
        """Upload a PDF file.

        POST /api/v1/documents/pdf

        Args:
            file: PDF file path or binary stream.
            filename: Override filename sent to the server.
            metadata: Key/value metadata fields.
            enable_vision: Use LLM vision pipeline for extraction (0.4.0+).
            vision_provider: Override vision LLM provider (e.g. "openai").
            vision_model: Override vision LLM model (e.g. "gpt-4o").
            title: Human-readable title.
            track_id: Batch track ID.
            force_reindex: Re-process even if document already exists.
            options: PdfUploadOptions dataclass (overrides individual params).
        """
        # WHY: PdfUploadOptions is a convenience wrapper; explicit params take
        # precedence, then options, then defaults.
        if options is not None:
            enable_vision = options.enable_vision
            vision_provider = options.vision_provider or vision_provider
            vision_model = options.vision_model or vision_model
            title = options.title or title
            track_id = options.track_id or track_id
            force_reindex = options.force_reindex

        # Build merged form-data: metadata + vision fields (str-serialised)
        form_data: dict[str, str] = dict(metadata or {})
        if enable_vision:
            form_data["enable_vision"] = "true"
        if vision_provider:
            form_data["vision_provider"] = vision_provider
        if vision_model:
            form_data["vision_model"] = vision_model
        if title:
            form_data["title"] = title
        if track_id:
            form_data["track_id"] = track_id
        if force_reindex:
            form_data["force_reindex"] = "true"

        response = self._transport.upload(
            "/api/v1/documents/pdf",
            file=file,
            filename=filename,
            metadata=form_data or None,
        )
        return PdfUploadResponse.model_validate(response.json())

    def list(self) -> _list[PdfInfo]:
        """List all PDF documents.

        GET /api/v1/documents/pdf
        """
        data = self._get("/api/v1/documents/pdf")
        if isinstance(data, list):
            return [PdfInfo.model_validate(p) for p in data]
        items = (
            data.get("pdfs", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [PdfInfo.model_validate(p) for p in items]

    def get(self, pdf_id: str) -> PdfInfo:
        """Get PDF status.

        GET /api/v1/documents/pdf/{pdf_id}
        """
        return self._get(f"/api/v1/documents/pdf/{pdf_id}", response_type=PdfInfo)

    def delete(self, pdf_id: str) -> None:
        """Delete a PDF document.

        DELETE /api/v1/documents/pdf/{pdf_id}
        """
        self._delete(f"/api/v1/documents/pdf/{pdf_id}")

    def download(self, pdf_id: str) -> bytes:
        """Download the raw PDF file.

        GET /api/v1/documents/pdf/{pdf_id}/download
        """
        response = self._transport.request(
            "GET", f"/api/v1/documents/pdf/{pdf_id}/download"
        )
        return response.content

    def content(self, pdf_id: str) -> PdfContentResponse:
        """Get PDF extracted content (markdown).

        GET /api/v1/documents/pdf/{pdf_id}/content
        """
        return self._get(
            f"/api/v1/documents/pdf/{pdf_id}/content",
            response_type=PdfContentResponse,
        )

    def progress(self, track_id: str) -> PdfProgressResponse:
        """Get PDF processing progress.

        GET /api/v1/documents/pdf/progress/{track_id}
        """
        return self._get(
            f"/api/v1/documents/pdf/progress/{track_id}",
            response_type=PdfProgressResponse,
        )

    def retry(self, pdf_id: str) -> dict[str, Any]:
        """Retry PDF processing.

        POST /api/v1/documents/pdf/{pdf_id}/retry
        """
        return self._post(f"/api/v1/documents/pdf/{pdf_id}/retry")

    def cancel(self, pdf_id: str) -> None:
        """Cancel PDF processing.

        DELETE /api/v1/documents/pdf/{pdf_id}/cancel
        """
        self._delete(f"/api/v1/documents/pdf/{pdf_id}/cancel")


# --- Async versions ---


class AsyncDocumentsResource(AsyncResource):
    """Asynchronous Documents API."""

    @property
    def pdf(self) -> AsyncPdfResource:
        return AsyncPdfResource(self._transport)

    async def upload(
        self,
        content: str,
        *,
        title: str | None = None,
        metadata: dict[str, str] | None = None,
        extract_entities: bool = True,
    ) -> UploadDocumentResponse:
        body: dict[str, Any] = {
            "content": content,
            "extract_entities": extract_entities,
        }
        if title:
            body["title"] = title
        if metadata:
            body["metadata"] = metadata
        return await self._post(
            "/api/v1/documents", json=body, response_type=UploadDocumentResponse
        )

    async def list(
        self,
        *,
        page: int = 1,
        page_size: int = 50,
        status: str | None = None,
        search: str | None = None,
    ) -> ListDocumentsResponse:
        params: dict[str, Any] = {"page": page, "page_size": page_size}
        if status:
            params["status"] = status
        if search:
            params["search"] = search
        return await self._get(
            "/api/v1/documents", params=params, response_type=ListDocumentsResponse
        )

    async def get(self, document_id: str) -> DocumentDetail:
        return await self._get(
            f"/api/v1/documents/{document_id}", response_type=DocumentDetail
        )

    async def delete(self, document_id: str) -> None:
        await self._delete(f"/api/v1/documents/{document_id}")

    async def delete_all(self) -> dict[str, Any]:
        response = await self._transport.request("DELETE", "/api/v1/documents")
        return response.json()

    async def track(self, track_id: str) -> TrackStatusResponse:
        return await self._get(
            f"/api/v1/documents/track/{track_id}",
            response_type=TrackStatusResponse,
        )

    async def upload_file(
        self,
        file: Path | BinaryIO,
        *,
        title: str | None = None,
        metadata: dict[str, str] | None = None,
    ) -> UploadDocumentResponse:
        meta = metadata or {}
        if title:
            meta["title"] = title
        response = await self._transport.upload(
            "/api/v1/documents/upload", file=file, metadata=meta
        )
        return UploadDocumentResponse.model_validate(response.json())

    async def scan(
        self,
        path: str,
        *,
        recursive: bool = True,
        extensions: _list[str] | None = None,
    ) -> ScanResponse:
        body: dict[str, Any] = {"path": path, "recursive": recursive}
        if extensions:
            body["extensions"] = extensions
        return await self._post(
            "/api/v1/documents/scan", json=body, response_type=ScanResponse
        )

    async def reprocess_failed(self) -> dict[str, Any]:
        return await self._post("/api/v1/documents/reprocess")

    async def recover_stuck(self) -> dict[str, Any]:
        return await self._post("/api/v1/documents/recover-stuck")

    # ========================================================================
    # Lineage Methods (OODA-16)
    # ========================================================================

    async def get_lineage(self, document_id: str) -> DocumentFullLineage:
        """Get complete document lineage.

        GET /api/v1/documents/{document_id}/lineage
        """
        return await self._get(
            f"/api/v1/documents/{document_id}/lineage",
            response_type=DocumentFullLineage,
        )

    async def get_metadata(self, document_id: str) -> dict[str, Any]:
        """Get all document metadata from KV storage.

        GET /api/v1/documents/{document_id}/metadata
        """
        return await self._get(f"/api/v1/documents/{document_id}/metadata")

    async def export_lineage(self, document_id: str, *, format: str = "json") -> bytes:
        """Export document lineage as a downloadable file.

        GET /api/v1/documents/{document_id}/lineage/export?format={format}

        WHY: Enables compliance audit trails and data pipeline ingestion.
        """
        response = await self._transport.request(
            "GET",
            f"/api/v1/documents/{document_id}/lineage/export",
            params={"format": format},
        )
        if hasattr(response, "content"):
            return response.content
        return response.read()


class AsyncPdfResource(AsyncResource):
    """Asynchronous PDF sub-resource."""

    async def upload(
        self,
        file: Path | BinaryIO,
        *,
        filename: str | None = None,
        metadata: dict[str, str] | None = None,
        enable_vision: bool = False,
        vision_provider: str | None = None,
        vision_model: str | None = None,
        title: str | None = None,
        track_id: str | None = None,
        force_reindex: bool = False,
        options: PdfUploadOptions | None = None,
    ) -> PdfUploadResponse:
        """Upload a PDF file asynchronously.

        POST /api/v1/documents/pdf

        Args:
            file: PDF file path or binary stream.
            filename: Override filename sent to the server.
            metadata: Key/value metadata fields.
            enable_vision: Use LLM vision pipeline for extraction (0.4.0+).
            vision_provider: Override vision LLM provider (e.g. "openai").
            vision_model: Override vision LLM model (e.g. "gpt-4o").
            title: Human-readable title.
            track_id: Batch track ID.
            force_reindex: Re-process even if document already exists.
            options: PdfUploadOptions dataclass (overrides individual params).
        """
        if options is not None:
            enable_vision = options.enable_vision
            vision_provider = options.vision_provider or vision_provider
            vision_model = options.vision_model or vision_model
            title = options.title or title
            track_id = options.track_id or track_id
            force_reindex = options.force_reindex

        form_data: dict[str, str] = dict(metadata or {})
        if enable_vision:
            form_data["enable_vision"] = "true"
        if vision_provider:
            form_data["vision_provider"] = vision_provider
        if vision_model:
            form_data["vision_model"] = vision_model
        if title:
            form_data["title"] = title
        if track_id:
            form_data["track_id"] = track_id
        if force_reindex:
            form_data["force_reindex"] = "true"

        response = await self._transport.upload(
            "/api/v1/documents/pdf",
            file=file,
            filename=filename,
            metadata=form_data or None,
        )
        return PdfUploadResponse.model_validate(response.json())

    async def list(self) -> _list[PdfInfo]:
        data = await self._get("/api/v1/documents/pdf")
        if isinstance(data, list):
            return [PdfInfo.model_validate(p) for p in data]
        items = (
            data.get("pdfs", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [PdfInfo.model_validate(p) for p in items]

    async def get(self, pdf_id: str) -> PdfInfo:
        return await self._get(f"/api/v1/documents/pdf/{pdf_id}", response_type=PdfInfo)

    async def delete(self, pdf_id: str) -> None:
        await self._delete(f"/api/v1/documents/pdf/{pdf_id}")

    async def content(self, pdf_id: str) -> PdfContentResponse:
        return await self._get(
            f"/api/v1/documents/pdf/{pdf_id}/content",
            response_type=PdfContentResponse,
        )

    async def progress(self, track_id: str) -> PdfProgressResponse:
        return await self._get(
            f"/api/v1/documents/pdf/progress/{track_id}",
            response_type=PdfProgressResponse,
        )

    async def retry(self, pdf_id: str) -> dict[str, Any]:
        return await self._post(f"/api/v1/documents/pdf/{pdf_id}/retry")

    async def cancel(self, pdf_id: str) -> None:
        await self._delete(f"/api/v1/documents/pdf/{pdf_id}/cancel")
