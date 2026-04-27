"""Conversation type definitions for the EdgeQuake Python SDK.

WHY: Maps conversation/chat-history-related types, matching
edgequake-api/src/handlers/conversation_types.rs.
"""

from __future__ import annotations

from typing import Any, Literal

from pydantic import AliasChoices, BaseModel, Field, model_validator


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
    """Detailed conversation with messages.

    Accepts both the API shape ``{ "conversation": {...}, "messages": [...] }``
    and a flat legacy shape for tests.
    """

    messages: list[Message] | None = None
    metadata: dict[str, Any] | None = None
    share_id: str | None = None

    @model_validator(mode="before")
    @classmethod
    def _unwrap_conversation_wrapper(cls, data: Any) -> Any:
        if isinstance(data, dict) and "conversation" in data:
            conv = data["conversation"]
            if isinstance(conv, dict):
                return {
                    **conv,
                    "messages": data.get("messages"),
                    "metadata": data.get("metadata"),
                }
        return data


class MessageCreate(BaseModel):
    """Request to create a message in a conversation."""

    role: Literal["user", "assistant", "system"] = "user"
    content: str
    parent_id: str | None = None
    metadata: dict[str, Any] | None = None


class MessageUpdate(BaseModel):
    """Request body for PATCH /api/v1/messages/{id} (matches edgequake-api)."""

    content: str | None = None
    tokens_used: int | None = None
    duration_ms: int | None = None
    thinking_time_ms: int | None = None
    context: Any | None = None
    is_error: bool | None = None
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


class CursorPaginationMeta(BaseModel):
    """Cursor pagination from list endpoints."""

    next_cursor: str | None = None
    prev_cursor: str | None = None
    total: int | None = None
    has_more: bool = False


class PaginatedConversations(BaseModel):
    """GET /api/v1/conversations — paginated wrapper."""

    items: list[ConversationInfo] = Field(default_factory=list)
    pagination: CursorPaginationMeta = Field(default_factory=CursorPaginationMeta)

    @model_validator(mode="before")
    @classmethod
    def _coerce(cls, data: Any) -> Any:
        if isinstance(data, list):
            return {"items": data, "pagination": {}}
        if isinstance(data, dict):
            items = data.get("items") or data.get("conversations") or []
            pag = data.get("pagination") if isinstance(data.get("pagination"), dict) else {}
            return {"items": items, "pagination": pag}
        return {"items": [], "pagination": {}}


class PaginatedMessages(BaseModel):
    """GET /api/v1/conversations/{id}/messages — paginated wrapper."""

    items: list[Message] = Field(default_factory=list)
    pagination: CursorPaginationMeta = Field(default_factory=CursorPaginationMeta)

    @model_validator(mode="before")
    @classmethod
    def _coerce(cls, data: Any) -> Any:
        if isinstance(data, list):
            return {"items": data, "pagination": {}}
        if isinstance(data, dict):
            items = data.get("items") or data.get("messages") or []
            pag = data.get("pagination") if isinstance(data.get("pagination"), dict) else {}
            return {"items": items, "pagination": pag}
        return {"items": [], "pagination": {}}


class ConversationListParams(BaseModel):
    """Query params for GET /api/v1/conversations (cursor + filter[…])."""

    cursor: str | None = None
    limit: int | None = Field(default=None, ge=1, le=100)
    filter_mode: str | None = None
    filter_archived: bool | None = None
    filter_pinned: bool | None = None
    filter_folder_id: str | None = None
    filter_unfiled: bool | None = None
    filter_search: str | None = None
    sort: str | None = None
    order: str | None = None

    def to_query_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {}
        if self.cursor is not None:
            d["cursor"] = self.cursor
        if self.limit is not None:
            d["limit"] = self.limit
        if self.filter_mode is not None:
            d["filter[mode]"] = self.filter_mode
        if self.filter_archived is not None:
            d["filter[archived]"] = self.filter_archived
        if self.filter_pinned is not None:
            d["filter[pinned]"] = self.filter_pinned
        if self.filter_folder_id is not None:
            d["filter[folder_id]"] = self.filter_folder_id
        if self.filter_unfiled is not None:
            d["filter[unfiled]"] = self.filter_unfiled
        if self.filter_search is not None:
            d["filter[search]"] = self.filter_search
        if self.sort is not None:
            d["sort"] = self.sort
        if self.order is not None:
            d["order"] = self.order
        return d


class ListMessagesParams(BaseModel):
    """Query for GET …/conversations/{id}/messages."""

    cursor: str | None = None
    limit: int | None = Field(default=None, ge=1, le=200)

    def to_query_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {}
        if self.cursor is not None:
            d["cursor"] = self.cursor
        if self.limit is not None:
            d["limit"] = self.limit
        return d


class ShareLink(BaseModel):
    """Sharing link for a conversation."""

    share_id: str
    share_url: str | None = Field(
        default=None,
        validation_alias=AliasChoices("share_url", "url"),
    )
    created_at: str | None = None
    expires_at: str | None = None


class SharedConversation(ConversationDetail):
    """Public shared conversation — same JSON shape as `ConversationDetail`."""


class BulkDeleteRequest(BaseModel):
    """API bulk body: ``conversation_ids``."""

    conversation_ids: list[str]


class BulkOpResponse(BaseModel):
    """`affected` count from bulk archive/move/delete."""

    affected: int = 0

    @model_validator(mode="before")
    @classmethod
    def _legacy_deleted_count(cls, data: Any) -> Any:
        if isinstance(data, dict) and "affected" not in data and "deleted_count" in data:
            return {**data, "affected": data["deleted_count"]}
        return data


class BulkDeleteResponse(BulkOpResponse):
    """Response from bulk delete."""


class BulkArchiveRequest(BaseModel):
    """Request for bulk archive."""

    conversation_ids: list[str]
    archive: bool = True


class BulkMoveRequest(BaseModel):
    """Request for bulk move to folder."""

    conversation_ids: list[str]
    folder_id: str | None = None


class ImportConversationsRequest(BaseModel):
    """Request for importing conversations."""

    conversations: list[dict[str, Any]]


class ImportErrorItem(BaseModel):
    """Single import error from API."""

    id: str
    error: str


class ImportConversationsResponse(BaseModel):
    """Response from import (matches edgequake-api)."""

    imported: int = 0
    failed: int = 0
    errors: list[ImportErrorItem] = Field(default_factory=list)

    @model_validator(mode="before")
    @classmethod
    def _legacy_counts(cls, data: Any) -> Any:
        if not isinstance(data, dict):
            return data
        out = dict(data)
        if "imported_count" in out and "imported" not in out:
            out["imported"] = out.pop("imported_count")
        if "skipped_count" in out and "failed" not in out:
            out["failed"] = out.pop("skipped_count")
        if out.get("errors") and isinstance(out["errors"][0], str):
            # Legacy tests used list[str] errors — drop if not structured
            out["errors"] = []
        return out


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
