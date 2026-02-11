"""Document type definitions for the EdgeQuake Python SDK.

WHY: Maps all document-related request/response types to Pydantic models,
matching the Rust API types in edgequake-api/src/handlers/documents_types.rs.
"""

from __future__ import annotations

from datetime import datetime
from typing import Any

from pydantic import BaseModel, Field


class UploadDocumentRequest(BaseModel):
    """Request body for POST /api/v1/documents."""

    content: str
    title: str | None = None
    metadata: dict[str, str] | None = None
    extract_entities: bool = True


class UploadDocumentResponse(BaseModel):
    """Response from POST /api/v1/documents."""

    # WHY: API returns "document_id" — we alias it for SDK consistency
    document_id: str = Field(alias="document_id")
    status: str | None = None
    message: str | None = None
    track_id: str | None = None
    task_id: str | None = None
    duplicate_of: str | None = None
    chunk_count: int | None = None
    entity_count: int | None = None
    relationship_count: int | None = None
    cost: DocumentCostInfo | None = None
    # WHY: API may also return embedding_model
    embedding_model: str | None = None

    model_config = {"populate_by_name": True}


class DocumentCostInfo(BaseModel):
    """Cost information for a document processing operation."""

    total_cost_usd: float | None = None
    formatted_cost: str | None = None
    input_tokens: int | None = None
    output_tokens: int | None = None
    total_tokens: int | None = None
    model: str | None = None
    provider: str | None = None


class StatusCounts(BaseModel):
    """Document status aggregation counts."""

    pending: int = 0
    processing: int = 0
    completed: int = 0
    failed: int = 0
    cancelled: int = 0
    total: int = 0


class DocumentSummary(BaseModel):
    """Summary information for a document in list responses."""

    id: str
    title: str | None = None
    status: str = "pending"
    content_length: int | None = None
    chunk_count: int | None = None
    entity_count: int | None = None
    relationship_count: int | None = None
    file_type: str | None = None
    file_name: str | None = None
    file_size: int | None = None
    source: str | None = None
    created_at: str | None = None
    updated_at: str | None = None
    completed_at: str | None = None
    error_message: str | None = None
    track_id: str | None = None
    metadata: dict[str, Any] | None = None
    cost: DocumentCostInfo | None = None
    processing_time_ms: int | None = None


# WHY: Legacy alias for backward compatibility
DocumentInfo = DocumentSummary


class PaginationInfo(BaseModel):
    """Pagination metadata in list responses."""

    page: int = 1
    page_size: int = 50
    total: int = 0
    total_pages: int = 0


class ListDocumentsResponse(BaseModel):
    """Response from GET /api/v1/documents."""

    documents: list[DocumentSummary] = Field(default_factory=list)
    pagination: PaginationInfo | None = None
    status_counts: StatusCounts | None = None


class DocumentDetail(BaseModel):
    """Detailed document information from GET /api/v1/documents/{id}."""

    id: str
    title: str | None = None
    content: str | None = None
    status: str = "pending"
    chunk_count: int | None = None
    entity_count: int | None = None
    relationship_count: int | None = None
    file_type: str | None = None
    file_name: str | None = None
    file_size: int | None = None
    source: str | None = None
    created_at: str | None = None
    updated_at: str | None = None
    completed_at: str | None = None
    error_message: str | None = None
    track_id: str | None = None
    metadata: dict[str, Any] | None = None
    cost: DocumentCostInfo | None = None
    processing_time_ms: int | None = None
    chunks: list[dict[str, Any]] | None = None
    entities: list[dict[str, Any]] | None = None
    relationships: list[dict[str, Any]] | None = None


class DeleteAllResponse(BaseModel):
    """Response from DELETE /api/v1/documents."""

    deleted_count: int = 0
    message: str | None = None


class TrackStatusResponse(BaseModel):
    """Response from GET /api/v1/documents/track/{track_id}."""

    track_id: str
    status: str
    progress: float | None = None
    message: str | None = None
    document_id: str | None = None
    created_at: str | None = None
    updated_at: str | None = None
    error: str | None = None
    steps: list[dict[str, Any]] | None = None


class BatchUploadResponse(BaseModel):
    """Response from POST /api/v1/documents/upload/batch."""

    results: list[UploadDocumentResponse] = Field(default_factory=list)
    total: int = 0
    success_count: int = 0
    failure_count: int = 0


class ScanDirectoryRequest(BaseModel):
    """Request body for POST /api/v1/documents/scan."""

    path: str
    recursive: bool = True
    extensions: list[str] | None = None
    exclude_patterns: list[str] | None = None


class ScanResponse(BaseModel):
    """Response from POST /api/v1/documents/scan."""

    files_found: int = 0
    files_queued: int = 0
    files_skipped: int = 0
    track_id: str | None = None
    message: str | None = None


class DeletionImpactResponse(BaseModel):
    """Response from GET /api/v1/documents/{id}/deletion-impact."""

    document_id: str
    entity_count: int = 0
    relationship_count: int = 0
    chunk_count: int = 0
    exclusive_entities: int = 0
    exclusive_relationships: int = 0
    message: str | None = None


class FailedChunkInfo(BaseModel):
    """Info about a failed chunk."""

    chunk_id: str
    error: str | None = None
    retry_count: int = 0


class PdfUploadResponse(BaseModel):
    """Response from POST /api/v1/documents/pdf."""

    id: str
    status: str
    track_id: str | None = None
    message: str | None = None
    file_name: str | None = None


class PdfInfo(BaseModel):
    """PDF document info from list endpoint."""

    id: str
    file_name: str | None = None
    status: str = "pending"
    page_count: int | None = None
    file_size: int | None = None
    created_at: str | None = None
    updated_at: str | None = None
    track_id: str | None = None
    error_message: str | None = None


class PdfProgressResponse(BaseModel):
    """Response from GET /api/v1/documents/pdf/progress/{track_id}."""

    track_id: str
    status: str
    progress: float | None = None
    current_page: int | None = None
    total_pages: int | None = None
    message: str | None = None


class PdfContentResponse(BaseModel):
    """Response from GET /api/v1/documents/pdf/{id}/content."""

    id: str
    content: str | None = None
    markdown: str | None = None
    page_count: int | None = None
    metadata: dict[str, Any] | None = None


# WHY: Rebuild UploadDocumentResponse model references after DocumentCostInfo is defined
UploadDocumentResponse.model_rebuild()
