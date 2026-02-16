"""Chat type definitions for the EdgeQuake Python SDK.

WHY: Maps chat completions request/response types, matching
edgequake-api/src/handlers/chat_types.rs. Re-uses SourceReference
and QueryStats from query types to avoid duplication (DRY).

NOTE: EdgeQuake chat API uses `message` (singular string), NOT `messages` (array).
This is NOT OpenAI-compatible — it is EdgeQuake's native RAG-aware chat format.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field

from edgequake.types.query import QueryStats, SourceReference


class ChatMessage(BaseModel):
    """A message in a chat conversation (used for conversation history display)."""

    role: Literal["system", "user", "assistant"] = "user"
    content: str


class ChatCompletionRequest(BaseModel):
    """Request body for POST /api/v1/chat/completions.

    WHY: EdgeQuake uses `message` (singular string) not `messages` (array).
    The conversation threading is handled server-side via conversation_id.
    """

    message: str
    stream: bool = False
    mode: str | None = None
    conversation_id: str | None = None
    max_tokens: int | None = None
    temperature: float | None = None
    top_k: int | None = None
    parent_id: str | None = None
    provider: str | None = None
    model: str | None = None


class ChatCompletionResponse(BaseModel):
    """Response from POST /api/v1/chat/completions.

    WHY: EdgeQuake returns conversation-threaded response with RAG sources,
    not OpenAI-style choices array.
    """

    conversation_id: str | None = None
    user_message_id: str | None = None
    assistant_message_id: str | None = None
    content: str | None = None
    mode: str | None = None
    sources: list[SourceReference] = Field(default_factory=list)
    stats: QueryStats | None = None


class ChatCompletionChunk(BaseModel):
    """SSE chunk event for streaming chat completions.

    WHY: EdgeQuake server sends tagged events with ``{type: "...", ...}`` format.
    This model can deserialize any event type; consumers should check ``type``
    to determine which fields are populated.
    """

    id: str | None = None
    object: str = "chat.completion.chunk"
    created: int | None = None
    model: str | None = None
    choices: list[ChatStreamChoice] | None = None
    sources: list[SourceReference] | None = None
    done: bool = False
    error: str | None = None

    # EdgeQuake-native tagged event fields (server sends {type: "...", ...})
    # @implements FEAT0505: Auto-generated conversation titles

    #: Event type discriminator.
    #: Values: "conversation", "context", "token", "thinking", "done",
    #: "title_update", "error"
    type: str | None = None

    #: Token or thinking content (present in token/thinking events).
    content: str | None = None

    #: Conversation ID (present in conversation, title_update events).
    conversation_id: str | None = None

    #: Auto-generated conversation title (present in title_update events).
    title: str | None = None

    #: User message ID (present in conversation event).
    user_message_id: str | None = None

    #: Assistant message ID (present in done event).
    assistant_message_id: str | None = None

    #: Tokens used (present in done event).
    tokens_used: int | None = None

    #: Duration in milliseconds (present in done event).
    duration_ms: int | None = None

    #: LLM provider used (present in done event, lineage tracking).
    llm_provider: str | None = None

    #: LLM model used (present in done event, lineage tracking).
    llm_model: str | None = None


class ChatStreamChoice(BaseModel):
    """A streaming choice delta."""

    index: int = 0
    delta: ChatStreamDelta | None = None
    finish_reason: str | None = None


class ChatStreamDelta(BaseModel):
    """Delta content in a streaming chunk."""

    role: str | None = None
    content: str | None = None


class ChatChoice(BaseModel):
    """OpenAI-compatible chat choice (for SDK test compatibility).

    WHY: Some tests validate OpenAI-style response shapes. This type
    enables testing both EdgeQuake-native and OpenAI-compatible formats.
    """

    index: int = 0
    message: ChatMessage | None = None
    finish_reason: str | None = None


class ChatUsage(BaseModel):
    """Token usage statistics (OpenAI-compatible).

    WHY: Provides token accounting for cost tracking and quota management.
    """

    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0


# WHY: Rebuild forward references
ChatCompletionChunk.model_rebuild()
