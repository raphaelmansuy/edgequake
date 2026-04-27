"""Conversations resource — Conversation and message management.

WHY: Maps to /api/v1/conversations/*, /api/v1/messages/*,
/api/v1/folders/*, and /api/v1/shared/* endpoints.

WHY OODA-06: Aliased built-in `list` to `_list` to avoid shadowing by method name.
"""

from __future__ import annotations

from typing import Any
from typing import List as _list

from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.conversations import (
    BulkArchiveRequest,
    BulkDeleteRequest,
    BulkDeleteResponse,
    BulkMoveRequest,
    BulkOpResponse,
    ConversationCreate,
    ConversationDetail,
    ConversationInfo,
    ConversationListParams,
    ConversationUpdate,
    FolderCreate,
    FolderInfo,
    FolderUpdate,
    ImportConversationsResponse,
    ListMessagesParams,
    Message,
    MessageCreate,
    MessageUpdate,
    PaginatedConversations,
    PaginatedMessages,
    SharedConversation,
    ShareLink,
)


def _conversation_list_query_dict(
    *,
    params: ConversationListParams | None,
    folder_id: str | None,
    page: int | None,
    page_size: int | None,
) -> dict[str, Any]:
    """Build GET /conversations query: canon params, or legacy folder_id/page_size shim."""
    if params is not None:
        return params.to_query_dict()
    out: dict[str, Any] = {}
    if folder_id is not None:
        out["filter[folder_id]"] = folder_id
    if page_size is not None:
        out["limit"] = min(max(page_size, 1), 100)
    elif page is not None:
        out["limit"] = 20
    return out


class ConversationsResource(SyncResource):
    """Synchronous Conversations API."""

    def list(
        self,
        *,
        params: ConversationListParams | None = None,
        folder_id: str | None = None,
        page: int | None = None,
        page_size: int | None = None,
    ) -> PaginatedConversations:
        """List conversations (cursor + ``filter[…]``). Legacy ``folder_id`` / ``page_size`` map to canon query keys.

        GET /api/v1/conversations
        """
        q = _conversation_list_query_dict(
            params=params,
            folder_id=folder_id,
            page=page,
            page_size=page_size,
        )
        data = self._get("/api/v1/conversations", params=q or None)
        return PaginatedConversations.model_validate(data)

    def create(
        self,
        *,
        title: str | None = None,
        folder_id: str | None = None,
    ) -> ConversationInfo:
        """Create a conversation.

        POST /api/v1/conversations
        """
        body = ConversationCreate(title=title, folder_id=folder_id)
        return self._post(
            "/api/v1/conversations",
            json=body.model_dump(exclude_none=True),
            response_type=ConversationInfo,
        )

    def get(self, conversation_id: str) -> ConversationDetail:
        """Get conversation details.

        GET /api/v1/conversations/{id}
        """
        return self._get(
            f"/api/v1/conversations/{conversation_id}",
            response_type=ConversationDetail,
        )

    def update(
        self, conversation_id: str, update: ConversationUpdate
    ) -> ConversationInfo:
        """Update a conversation.

        PATCH /api/v1/conversations/{id}
        """
        response = self._transport.request(
            "PATCH",
            f"/api/v1/conversations/{conversation_id}",
            json=update.model_dump(exclude_none=True),
        )
        return ConversationInfo.model_validate(response.json())

    def delete(self, conversation_id: str) -> None:
        """Delete a conversation.

        DELETE /api/v1/conversations/{id}
        """
        self._delete(f"/api/v1/conversations/{conversation_id}")

    def import_conversations(
        self, conversations: _list[dict[str, Any]]
    ) -> ImportConversationsResponse:
        """Import conversations.

        POST /api/v1/conversations/import
        """
        return self._post(
            "/api/v1/conversations/import",
            json={"conversations": conversations},
            response_type=ImportConversationsResponse,
        )

    def bulk_delete(self, ids: _list[str]) -> BulkDeleteResponse:
        """Bulk delete conversations.

        POST /api/v1/conversations/bulk/delete
        """
        return self._post(
            "/api/v1/conversations/bulk/delete",
            json=BulkDeleteRequest(conversation_ids=list(ids)).model_dump(),
            response_type=BulkDeleteResponse,
        )

    def bulk_archive(
        self, ids: _list[str], *, archive: bool = True
    ) -> BulkOpResponse:
        """Bulk archive/unarchive conversations.

        POST /api/v1/conversations/bulk/archive
        """
        return self._post(
            "/api/v1/conversations/bulk/archive",
            json=BulkArchiveRequest(
                conversation_ids=list(ids), archive=archive
            ).model_dump(),
            response_type=BulkOpResponse,
        )

    def bulk_move(
        self, ids: _list[str], *, folder_id: str | None = None
    ) -> BulkOpResponse:
        """Bulk move conversations to a folder.

        POST /api/v1/conversations/bulk/move
        """
        return self._post(
            "/api/v1/conversations/bulk/move",
            json=BulkMoveRequest(
                conversation_ids=list(ids), folder_id=folder_id
            ).model_dump(),
            response_type=BulkOpResponse,
        )

    # --- Messages sub-operations ---

    def list_messages(
        self,
        conversation_id: str,
        *,
        params: ListMessagesParams | None = None,
    ) -> PaginatedMessages:
        """List messages in a conversation.

        GET /api/v1/conversations/{id}/messages
        """
        q = params.to_query_dict() if params is not None else None
        data = self._get(
            f"/api/v1/conversations/{conversation_id}/messages",
            params=q,
        )
        return PaginatedMessages.model_validate(data)

    def create_message(self, conversation_id: str, message: MessageCreate) -> Message:
        """Add a message to a conversation.

        POST /api/v1/conversations/{id}/messages
        """
        return self._post(
            f"/api/v1/conversations/{conversation_id}/messages",
            json=message.model_dump(exclude_none=True),
            response_type=Message,
        )

    def update_message(self, message_id: str, update: MessageUpdate) -> Message:
        """Update a message.

        PATCH /api/v1/messages/{message_id}
        """
        response = self._transport.request(
            "PATCH",
            f"/api/v1/messages/{message_id}",
            json=update.model_dump(exclude_none=True),
        )
        return Message.model_validate(response.json())

    def delete_message(self, message_id: str) -> None:
        """Delete a message.

        DELETE /api/v1/messages/{message_id}
        """
        self._delete(f"/api/v1/messages/{message_id}")

    # --- Sharing ---

    def share(self, conversation_id: str) -> ShareLink:
        """Share a conversation.

        POST /api/v1/conversations/{id}/share
        """
        return self._post(
            f"/api/v1/conversations/{conversation_id}/share",
            response_type=ShareLink,
        )

    def unshare(self, conversation_id: str) -> None:
        """Unshare a conversation.

        DELETE /api/v1/conversations/{id}/share
        """
        self._delete(f"/api/v1/conversations/{conversation_id}/share")

    def get_shared(self, share_id: str) -> SharedConversation:
        """Get a shared conversation (public access).

        GET /api/v1/shared/{share_id}
        """
        return self._get(f"/api/v1/shared/{share_id}", response_type=SharedConversation)


class FoldersResource(SyncResource):
    """Folder management for conversations."""

    def list(self) -> _list[FolderInfo]:
        """List folders.

        GET /api/v1/folders
        """
        data = self._get("/api/v1/folders")
        if isinstance(data, list):
            return [FolderInfo.model_validate(f) for f in data]
        items = (
            data.get("folders", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [FolderInfo.model_validate(f) for f in items]

    def create(self, name: str, *, parent_id: str | None = None) -> FolderInfo:
        """Create a folder.

        POST /api/v1/folders
        """
        body = FolderCreate(name=name, parent_id=parent_id)
        return self._post(
            "/api/v1/folders",
            json=body.model_dump(exclude_none=True),
            response_type=FolderInfo,
        )

    def update(self, folder_id: str, name: str) -> FolderInfo:
        """Update a folder.

        PATCH /api/v1/folders/{folder_id}
        """
        response = self._transport.request(
            "PATCH",
            f"/api/v1/folders/{folder_id}",
            json=FolderUpdate(name=name).model_dump(exclude_none=True),
        )
        return FolderInfo.model_validate(response.json())

    def delete(self, folder_id: str) -> None:
        """Delete a folder.

        DELETE /api/v1/folders/{folder_id}
        """
        self._delete(f"/api/v1/folders/{folder_id}")


# --- Async versions ---


class AsyncConversationsResource(AsyncResource):
    """Async conversations API."""

    async def list(
        self,
        *,
        params: ConversationListParams | None = None,
        folder_id: str | None = None,
        page: int | None = None,
        page_size: int | None = None,
    ) -> PaginatedConversations:
        q = _conversation_list_query_dict(
            params=params,
            folder_id=folder_id,
            page=page,
            page_size=page_size,
        )
        data = await self._get("/api/v1/conversations", params=q or None)
        return PaginatedConversations.model_validate(data)

    async def create(
        self, *, title: str | None = None, folder_id: str | None = None
    ) -> ConversationInfo:
        body = ConversationCreate(title=title, folder_id=folder_id)
        return await self._post(
            "/api/v1/conversations",
            json=body.model_dump(exclude_none=True),
            response_type=ConversationInfo,
        )

    async def get(self, conversation_id: str) -> ConversationDetail:
        return await self._get(
            f"/api/v1/conversations/{conversation_id}",
            response_type=ConversationDetail,
        )

    async def update(
        self, conversation_id: str, update: ConversationUpdate
    ) -> ConversationInfo:
        response = await self._transport.request(
            "PATCH",
            f"/api/v1/conversations/{conversation_id}",
            json=update.model_dump(exclude_none=True),
        )
        return ConversationInfo.model_validate(response.json())

    async def delete(self, conversation_id: str) -> None:
        await self._delete(f"/api/v1/conversations/{conversation_id}")

    async def import_conversations(
        self, conversations: _list[dict[str, Any]]
    ) -> ImportConversationsResponse:
        return await self._post(
            "/api/v1/conversations/import",
            json={"conversations": conversations},
            response_type=ImportConversationsResponse,
        )

    async def bulk_delete(self, ids: _list[str]) -> BulkDeleteResponse:
        return await self._post(
            "/api/v1/conversations/bulk/delete",
            json=BulkDeleteRequest(conversation_ids=list(ids)).model_dump(),
            response_type=BulkDeleteResponse,
        )

    async def bulk_archive(
        self, ids: _list[str], *, archive: bool = True
    ) -> BulkOpResponse:
        return await self._post(
            "/api/v1/conversations/bulk/archive",
            json=BulkArchiveRequest(
                conversation_ids=list(ids), archive=archive
            ).model_dump(),
            response_type=BulkOpResponse,
        )

    async def bulk_move(
        self, ids: _list[str], *, folder_id: str | None = None
    ) -> BulkOpResponse:
        return await self._post(
            "/api/v1/conversations/bulk/move",
            json=BulkMoveRequest(
                conversation_ids=list(ids), folder_id=folder_id
            ).model_dump(),
            response_type=BulkOpResponse,
        )

    async def list_messages(
        self,
        conversation_id: str,
        *,
        params: ListMessagesParams | None = None,
    ) -> PaginatedMessages:
        q = params.to_query_dict() if params is not None else None
        data = await self._get(
            f"/api/v1/conversations/{conversation_id}/messages",
            params=q,
        )
        return PaginatedMessages.model_validate(data)

    async def create_message(
        self, conversation_id: str, message: MessageCreate
    ) -> Message:
        return await self._post(
            f"/api/v1/conversations/{conversation_id}/messages",
            json=message.model_dump(exclude_none=True),
            response_type=Message,
        )

    async def update_message(self, message_id: str, update: MessageUpdate) -> Message:
        response = await self._transport.request(
            "PATCH",
            f"/api/v1/messages/{message_id}",
            json=update.model_dump(exclude_none=True),
        )
        return Message.model_validate(response.json())

    async def delete_message(self, message_id: str) -> None:
        await self._delete(f"/api/v1/messages/{message_id}")

    async def share(self, conversation_id: str) -> ShareLink:
        return await self._post(
            f"/api/v1/conversations/{conversation_id}/share",
            response_type=ShareLink,
        )

    async def unshare(self, conversation_id: str) -> None:
        await self._delete(f"/api/v1/conversations/{conversation_id}/share")

    async def get_shared(self, share_id: str) -> SharedConversation:
        return await self._get(
            f"/api/v1/shared/{share_id}", response_type=SharedConversation
        )


class AsyncFoldersResource(AsyncResource):
    """Async folder management."""

    async def list(self) -> _list[FolderInfo]:
        data = await self._get("/api/v1/folders")
        if isinstance(data, list):
            return [FolderInfo.model_validate(f) for f in data]
        items = (
            data.get("folders", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [FolderInfo.model_validate(f) for f in items]

    async def create(self, name: str, *, parent_id: str | None = None) -> FolderInfo:
        body = FolderCreate(name=name, parent_id=parent_id)
        return await self._post(
            "/api/v1/folders",
            json=body.model_dump(exclude_none=True),
            response_type=FolderInfo,
        )

    async def delete(self, folder_id: str) -> None:
        await self._delete(f"/api/v1/folders/{folder_id}")
