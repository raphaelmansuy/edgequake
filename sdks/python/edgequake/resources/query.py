"""Query resource — RAG query execution for EdgeQuake.

WHY: Maps to /api/v1/query endpoints. Supports both direct query execution
and streaming via SSE.
"""

from __future__ import annotations

from typing import Any, Literal

from edgequake._streaming import AsyncSSEStream, SSEStream
from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.query import QueryResponse, QueryStreamEvent


class QueryResource(SyncResource):
    """Synchronous Query API."""

    def execute(
        self,
        query: str,
        *,
        mode: Literal["local", "global", "hybrid", "naive"] = "hybrid",
        top_k: int = 10,
        rerank: bool = False,
        provider: str | None = None,
        model: str | None = None,
        conversation_id: str | None = None,
    ) -> QueryResponse:
        """Execute a RAG query.

        POST /api/v1/query
        """
        body: dict[str, Any] = {
            "query": query,
            "mode": mode,
            "top_k": top_k,
            "rerank": rerank,
        }
        if provider:
            body["provider"] = provider
        if model:
            body["model"] = model
        if conversation_id:
            body["conversation_id"] = conversation_id
        return self._post("/api/v1/query", json=body, response_type=QueryResponse)

    def stream(
        self,
        query: str,
        *,
        mode: Literal["local", "global", "hybrid", "naive"] = "hybrid",
        top_k: int = 10,
    ) -> SSEStream[QueryStreamEvent]:
        """Execute a streaming RAG query via SSE.

        POST /api/v1/query/stream
        """
        body: dict[str, Any] = {
            "query": query,
            "mode": mode,
            "top_k": top_k,
        }
        response = self._transport.stream("POST", "/api/v1/query/stream", json=body)
        return SSEStream(response, QueryStreamEvent)


class AsyncQueryResource(AsyncResource):
    """Asynchronous Query API."""

    async def execute(
        self,
        query: str,
        *,
        mode: Literal["local", "global", "hybrid", "naive"] = "hybrid",
        top_k: int = 10,
        rerank: bool = False,
        provider: str | None = None,
        model: str | None = None,
        conversation_id: str | None = None,
    ) -> QueryResponse:
        body: dict[str, Any] = {
            "query": query,
            "mode": mode,
            "top_k": top_k,
            "rerank": rerank,
        }
        if provider:
            body["provider"] = provider
        if model:
            body["model"] = model
        if conversation_id:
            body["conversation_id"] = conversation_id
        return await self._post("/api/v1/query", json=body, response_type=QueryResponse)

    async def stream(
        self,
        query: str,
        *,
        mode: Literal["local", "global", "hybrid", "naive"] = "hybrid",
        top_k: int = 10,
    ) -> AsyncSSEStream[QueryStreamEvent]:
        body: dict[str, Any] = {
            "query": query,
            "mode": mode,
            "top_k": top_k,
        }
        response = await self._transport.stream(
            "POST", "/api/v1/query/stream", json=body
        )
        return AsyncSSEStream(response, QueryStreamEvent)
