"""Chat type definitions for the EdgeQuake Python SDK.

WHY: Maps chat completions request/response types, matching
edgequake-api/src/handlers/chat_types.rs. Re-uses SourceReference
and QueryStats from query types to avoid duplication (DRY).
"""

from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, Field

from edgequake.types.query import QueryStats, SourceReference


class ChatMessage(BaseModel):
    """A message in a chat conversation."""

    role: Literal["system", "user", "assistant"] = "user"
    content: str


class ChatCompletionRequest(BaseModel):
    """Request body for POST /api/v1/chat/completions."""

    messages: list[ChatMessage]
    model: str = "edgequake"
    temperature: float = 0.7
    max_tokens: int | None = None
    top_k: int | None = None
    stream: bool = False
    provider: str | None = None
    conversation_id: str | None = None
    parent_id: str | None = None
    mode: str | None = None


class ChatChoice(BaseModel):
    """A completion choice in the response."""

    index: int = 0
    message: ChatMessage | None = None
    finish_reason: str | None = None


class ChatUsage(BaseModel):
    """Token usage in the chat completion response."""

    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0


class ChatCompletionResponse(BaseModel):
    """Response from POST /api/v1/chat/completions."""

    id: str | None = None
    object: str = "chat.completion"
    created: int | None = None
    model: str | None = None
    choices: list[ChatChoice] = Field(default_factory=list)
    usage: ChatUsage | None = None
    sources: list[SourceReference] | None = None
    stats: QueryStats | None = None
    conversation_id: str | None = None


class ChatCompletionChunk(BaseModel):
    """SSE chunk event for streaming chat completions."""

    id: str | None = None
    object: str = "chat.completion.chunk"
    created: int | None = None
    model: str | None = None
    choices: list[ChatStreamChoice] | None = None
    sources: list[SourceReference] | None = None
    done: bool = False
    error: str | None = None


class ChatStreamChoice(BaseModel):
    """A streaming choice delta."""

    index: int = 0
    delta: ChatStreamDelta | None = None
    finish_reason: str | None = None


class ChatStreamDelta(BaseModel):
    """Delta content in a streaming chunk."""

    role: str | None = None
    content: str | None = None


# WHY: Rebuild forward references
ChatCompletionChunk.model_rebuild()
