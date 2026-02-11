"""Chat resource — Chat completions API for EdgeQuake.

WHY: Maps to /api/v1/chat/completions endpoints. Supports both synchronous
and streaming chat completions with RAG context.
"""

from __future__ import annotations

from typing import Any

from edgequake._streaming import AsyncSSEStream, SSEStream
from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.chat import (
    ChatCompletionChunk,
    ChatCompletionResponse,
    ChatMessage,
)


class ChatResource(SyncResource):
    """Synchronous Chat API."""

    def complete(
        self,
        messages: list[ChatMessage] | list[dict[str, str]],
        *,
        model: str = "edgequake",
        temperature: float = 0.7,
        max_tokens: int | None = None,
        provider: str | None = None,
        conversation_id: str | None = None,
        mode: str | None = None,
    ) -> ChatCompletionResponse:
        """Create a chat completion.

        POST /api/v1/chat/completions
        """
        msgs = [m.model_dump() if isinstance(m, ChatMessage) else m for m in messages]
        body: dict[str, Any] = {
            "messages": msgs,
            "model": model,
            "temperature": temperature,
            "stream": False,
        }
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if provider:
            body["provider"] = provider
        if conversation_id:
            body["conversation_id"] = conversation_id
        if mode:
            body["mode"] = mode
        return self._post(
            "/api/v1/chat/completions",
            json=body,
            response_type=ChatCompletionResponse,
        )

    def stream(
        self,
        messages: list[ChatMessage] | list[dict[str, str]],
        *,
        model: str = "edgequake",
        temperature: float = 0.7,
        max_tokens: int | None = None,
        provider: str | None = None,
        conversation_id: str | None = None,
    ) -> SSEStream[ChatCompletionChunk]:
        """Create a streaming chat completion via SSE.

        POST /api/v1/chat/completions/stream
        """
        msgs = [m.model_dump() if isinstance(m, ChatMessage) else m for m in messages]
        body: dict[str, Any] = {
            "messages": msgs,
            "model": model,
            "temperature": temperature,
            "stream": True,
        }
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if provider:
            body["provider"] = provider
        if conversation_id:
            body["conversation_id"] = conversation_id
        response = self._transport.stream(
            "POST", "/api/v1/chat/completions/stream", json=body
        )
        return SSEStream(response, ChatCompletionChunk)


class AsyncChatResource(AsyncResource):
    """Asynchronous Chat API."""

    async def complete(
        self,
        messages: list[ChatMessage] | list[dict[str, str]],
        *,
        model: str = "edgequake",
        temperature: float = 0.7,
        max_tokens: int | None = None,
        provider: str | None = None,
        conversation_id: str | None = None,
        mode: str | None = None,
    ) -> ChatCompletionResponse:
        msgs = [m.model_dump() if isinstance(m, ChatMessage) else m for m in messages]
        body: dict[str, Any] = {
            "messages": msgs,
            "model": model,
            "temperature": temperature,
            "stream": False,
        }
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if provider:
            body["provider"] = provider
        if conversation_id:
            body["conversation_id"] = conversation_id
        if mode:
            body["mode"] = mode
        return await self._post(
            "/api/v1/chat/completions",
            json=body,
            response_type=ChatCompletionResponse,
        )

    async def stream(
        self,
        messages: list[ChatMessage] | list[dict[str, str]],
        *,
        model: str = "edgequake",
        temperature: float = 0.7,
        max_tokens: int | None = None,
        provider: str | None = None,
    ) -> AsyncSSEStream[ChatCompletionChunk]:
        msgs = [m.model_dump() if isinstance(m, ChatMessage) else m for m in messages]
        body: dict[str, Any] = {
            "messages": msgs,
            "model": model,
            "temperature": temperature,
            "stream": True,
        }
        if max_tokens is not None:
            body["max_tokens"] = max_tokens
        if provider:
            body["provider"] = provider
        response = await self._transport.stream(
            "POST", "/api/v1/chat/completions/stream", json=body
        )
        return AsyncSSEStream(response, ChatCompletionChunk)
