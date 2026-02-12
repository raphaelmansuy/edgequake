"""Chat resource — Chat completions API for EdgeQuake.

WHY: Maps to /api/v1/chat/completions endpoints. Supports both synchronous
and streaming chat completions with RAG context.

NOTE: EdgeQuake chat API uses `message` (singular string), NOT `messages` (array).
"""

from __future__ import annotations

from typing import Any

from edgequake._streaming import AsyncSSEStream, SSEStream
from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.chat import (
    ChatCompletionChunk,
    ChatCompletionResponse,
)


class ChatResource(SyncResource):
    """Synchronous Chat API."""

    def complete(
        self,
        message: str,
        *,
        mode: str | None = None,
        conversation_id: str | None = None,
        max_tokens: int | None = None,
        temperature: float | None = None,
        top_k: int | None = None,
        provider: str | None = None,
        model: str | None = None,
    ) -> ChatCompletionResponse:
        """Create a chat completion.

        POST /api/v1/chat/completions

        WHY: EdgeQuake uses `message` (singular string). The backend handles
        conversation threading via conversation_id.
        """
        body: dict[str, Any] = {
            "message": message,
            "stream": False,
        }
        if mode:
            body["mode"] = mode
        if conversation_id:
            body["conversation_id"] = conversation_id
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if temperature is not None:
            body["temperature"] = temperature
        if top_k is not None:
            body["top_k"] = top_k
        if provider:
            body["provider"] = provider
        if model:
            body["model"] = model
        return self._post(
            "/api/v1/chat/completions",
            json=body,
            response_type=ChatCompletionResponse,
        )

    def stream(
        self,
        message: str,
        *,
        mode: str | None = None,
        conversation_id: str | None = None,
        max_tokens: int | None = None,
        temperature: float | None = None,
        provider: str | None = None,
        model: str | None = None,
    ) -> SSEStream[ChatCompletionChunk]:
        """Create a streaming chat completion via SSE.

        POST /api/v1/chat/completions/stream
        """
        body: dict[str, Any] = {
            "message": message,
            "stream": True,
        }
        if mode:
            body["mode"] = mode
        if conversation_id:
            body["conversation_id"] = conversation_id
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if temperature is not None:
            body["temperature"] = temperature
        if provider:
            body["provider"] = provider
        if model:
            body["model"] = model
        response = self._transport.stream(
            "POST", "/api/v1/chat/completions/stream", json=body
        )
        return SSEStream(response, ChatCompletionChunk)


class AsyncChatResource(AsyncResource):
    """Asynchronous Chat API."""

    async def complete(
        self,
        message: str,
        *,
        mode: str | None = None,
        conversation_id: str | None = None,
        max_tokens: int | None = None,
        temperature: float | None = None,
        top_k: int | None = None,
        provider: str | None = None,
        model: str | None = None,
    ) -> ChatCompletionResponse:
        """Create a chat completion (async)."""
        body: dict[str, Any] = {
            "message": message,
            "stream": False,
        }
        if mode:
            body["mode"] = mode
        if conversation_id:
            body["conversation_id"] = conversation_id
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if temperature is not None:
            body["temperature"] = temperature
        if top_k is not None:
            body["top_k"] = top_k
        if provider:
            body["provider"] = provider
        if model:
            body["model"] = model
        return await self._post(
            "/api/v1/chat/completions",
            json=body,
            response_type=ChatCompletionResponse,
        )

    async def stream(
        self,
        message: str,
        *,
        mode: str | None = None,
        conversation_id: str | None = None,
        max_tokens: int | None = None,
        temperature: float | None = None,
        provider: str | None = None,
        model: str | None = None,
    ) -> AsyncSSEStream[ChatCompletionChunk]:
        """Create a streaming chat completion (async)."""
        body: dict[str, Any] = {
            "message": message,
            "stream": True,
        }
        if mode:
            body["mode"] = mode
        if conversation_id:
            body["conversation_id"] = conversation_id
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if temperature is not None:
            body["temperature"] = temperature
        if provider:
            body["provider"] = provider
        if model:
            body["model"] = model
        response = await self._transport.stream(
            "POST", "/api/v1/chat/completions/stream", json=body
        )
        return AsyncSSEStream(response, ChatCompletionChunk)
