"""Conversation type definitions for the EdgeQuake Python SDK.

WHY: Maps conversation/chat-history-related types, matching
edgequake-api/src/handlers/conversation_types.rs.
"""

from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, Field


class ConversationCreate(BaseModel):
    """Request to create a conversation."""

    title: str | None = None
    mode: str | None = None
    folder_id: str | None = None
    metadata: dict[str, Any] | None = None


class ConversationUpdate(BaseModel):
    """Request to update a conversation."""

    title: str | None = None
    folder_id: str | None = None
    is_pinned: bool | None = None


class ConversationInfo(BaseModel):
    """Conversation summary."""

    id: str
    title: str | None = None
    folder_id: str | None = None
    message_count: int | None = 0
    is_pinned: bool = False
    is_shared: bool = False
    is_archived: bool = False
    created_at: str | None = None
    updated_at: str | None = None
    last_message_at: str | None = None


class ConversationDetail(ConversationInfo):
    """Detailed conversation with messages."""

    messages: list[Message] | None = None
    metadata: dict[str, Any] | None = None
    share_id: str | None = None


class MessageCreate(BaseModel):
    """Request to create a message in a conversation."""

    role: Literal["user", "assistant", "system"] = "user"
    content: str
    parent_id: str | None = None
    metadata: dict[str, Any] | None = None


class MessageUpdate(BaseModel):
    """Request to update a message."""

    content: str | None = None
    metadata: dict[str, Any] | None = None


class Message(BaseModel):
    """A message in a conversation."""

    id: str
    conversation_id: str | None = None
    role: str = "user"
    content: str = ""
    parent_id: str | None = None
    created_at: str | None = None
    updated_at: str | None = None
    metadata: dict[str, Any] | None = None
    sources: list[dict[str, Any]] | None = None


class ShareLink(BaseModel):
    """Sharing link for a conversation."""

    share_id: str
    url: str | None = None
    created_at: str | None = None
    expires_at: str | None = None


class SharedConversation(BaseModel):
    """A publicly shared conversation."""

    id: str
    share_id: str
    title: str | None = None
    message_count: int = 0
    created_at: str | None = None


class BulkDeleteRequest(BaseModel):
    """Request for bulk conversation operations."""

    ids: list[str]


class BulkDeleteResponse(BaseModel):
    """Response from bulk delete."""

    deleted_count: int = 0


class BulkArchiveRequest(BaseModel):
    """Request for bulk archive."""

    ids: list[str]
    archive: bool = True


class BulkMoveRequest(BaseModel):
    """Request for bulk move to folder."""

    ids: list[str]
    folder_id: str | None = None


class ImportConversationsRequest(BaseModel):
    """Request for importing conversations."""

    conversations: list[dict[str, Any]]


class ImportConversationsResponse(BaseModel):
    """Response from import."""

    imported_count: int = 0
    skipped_count: int = 0
    errors: list[str] = Field(default_factory=list)


class FolderCreate(BaseModel):
    """Request to create a folder."""

    name: str
    parent_id: str | None = None


class FolderUpdate(BaseModel):
    """Request to update a folder."""

    name: str | None = None


class FolderInfo(BaseModel):
    """Folder information."""

    id: str
    name: str
    parent_id: str | None = None
    conversation_count: int = 0
    created_at: str | None = None
    updated_at: str | None = None


# WHY: Rebuild forward references
ConversationDetail.model_rebuild()
