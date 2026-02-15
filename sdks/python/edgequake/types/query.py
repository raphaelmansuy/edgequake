"""Query type definitions for the EdgeQuake Python SDK.

WHY: Maps query/RAG request/response types to Pydantic models,
matching the Rust API types in edgequake-api/src/handlers/query_types.rs.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class QueryRequest(BaseModel):
    """Request body for POST /api/v1/query."""

    query: str
    mode: Literal["local", "global", "hybrid", "naive"] = "hybrid"
    top_k: int = 10
    rerank: bool = False
    provider: str | None = None
    model: str | None = None
    conversation_id: str | None = None
    response_type: str | None = None


class SourceReference(BaseModel):
    """A source reference in the query response."""

    source_type: str | None = None
    id: str | None = None
    score: float | None = None
    rerank_score: float | None = None
    snippet: str | None = None
    # WHY: API returns int but OpenAPI says string — accept both
    reference_id: int | str | None = None
    document_id: str | None = None
    file_path: str | None = None
    start_line: int | None = None
    end_line: int | None = None
    chunk_index: int | None = None


# WHY: Legacy alias for backward compatibility
QuerySource = SourceReference


class QueryStats(BaseModel):
    """Statistics about the query execution."""

    total_time_ms: float | None = None
    retrieval_time_ms: float | None = None
    generation_time_ms: float | None = None
    rerank_time_ms: float | None = None
    input_tokens: int | None = None
    output_tokens: int | None = None
    total_tokens: int | None = None
    model: str | None = None
    provider: str | None = None
    retrieval_count: int | None = None


class QueryResponse(BaseModel):
    """Response from POST /api/v1/query."""

    answer: str = ""
    sources: list[SourceReference] = Field(default_factory=list)
    stats: QueryStats | None = None
    conversation_id: str | None = None
    reranked: bool = False
    mode: str | None = None


class QueryStreamEvent(BaseModel):
    """SSE event for streaming query responses."""

    chunk: str | None = None
    done: bool = False
    sources: list[SourceReference] | None = None
    stats: QueryStats | None = None
    error: str | None = None
