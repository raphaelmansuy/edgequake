"""Tests for conversation and folder resources."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake.types.conversations import (
    BulkDeleteResponse,
    ConversationDetail,
    ConversationInfo,
    ConversationUpdate,
    FolderInfo,
    ImportConversationsResponse,
    Message,
    MessageCreate,
    MessageUpdate,
    SharedConversation,
    ShareLink,
)


class TestConversationsResource:
    """Test sync ConversationsResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {
                "id": "conv-1",
                "title": "Test Chat",
                "created_at": "2024-01-01T00:00:00Z",
            },
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.list()
        assert isinstance(result, list)
        assert len(result) == 1
        assert isinstance(result[0], ConversationInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_with_params(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = []
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.conversations.list(folder_id="f-1", page=2, page_size=10)
        params = mock_req.call_args[1]["params"]
        assert params["folder_id"] == "f-1"
        assert params["page"] == 2
        assert params["page_size"] == 10
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"conversations": [{"id": "c1", "title": "Chat"}]}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.list()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "conv-1",
            "title": "New Chat",
            "created_at": "2024-01-01T00:00:00Z",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.create(title="New Chat")
        assert isinstance(result, ConversationInfo)
        assert result.title == "New Chat"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create_with_folder(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "conv-1", "title": "Chat"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.conversations.create(title="Chat", folder_id="f-1")
        body = mock_req.call_args[1]["json"]
        assert body["folder_id"] == "f-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "conv-1",
            "title": "Test Chat",
            "messages": [
                {"id": "msg-1", "role": "user", "content": "Hello"},
            ],
            "created_at": "2024-01-01T00:00:00Z",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.get("conv-1")
        assert isinstance(result, ConversationDetail)
        assert result.title == "Test Chat"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "conv-1", "title": "Updated Title"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.update(
            "conv-1", ConversationUpdate(title="Updated Title")
        )
        assert isinstance(result, ConversationInfo)
        assert result.title == "Updated Title"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.conversations.delete("conv-1")
        mock_req.assert_called_once()
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_import_conversations(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"imported_count": 2}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.import_conversations(
            [{"title": "Chat 1"}, {"title": "Chat 2"}]
        )
        assert isinstance(result, ImportConversationsResponse)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_bulk_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "deleted_count": 3,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.bulk_delete(ids=["c1", "c2", "c3"])
        assert isinstance(result, BulkDeleteResponse)
        assert result.deleted_count == 3
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_bulk_archive(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"archived_count": 2}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.bulk_archive(["c1", "c2"], archive=True)
        assert result == {"archived_count": 2}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_bulk_move(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"moved_count": 2}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.bulk_move(["c1", "c2"], folder_id="f-1")
        assert result == {"moved_count": 2}
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_messages(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "m-1", "role": "user", "content": "Hello"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.list_messages("conv-1")
        assert len(result) == 1
        assert isinstance(result[0], Message)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_messages_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "messages": [{"id": "m-1", "role": "user", "content": "Hi"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.list_messages("conv-1")
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create_message(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "msg-1",
            "role": "user",
            "content": "Hello",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.create_message(
            "conv-1",
            MessageCreate(role="user", content="Hello"),
        )
        assert isinstance(result, Message)
        assert result.content == "Hello"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update_message(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "msg-1",
            "role": "user",
            "content": "Updated",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.update_message(
            "msg-1", MessageUpdate(content="Updated")
        )
        assert isinstance(result, Message)
        assert result.content == "Updated"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete_message(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.conversations.delete_message("msg-1")
        mock_req.assert_called_once()
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_share(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "share_id": "abc123",
            "url": "https://app.edgequake.io/share/abc123",
            "expires_at": "2024-12-31T23:59:59Z",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.share("conv-1")
        assert isinstance(result, ShareLink)
        assert result.share_id == "abc123"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_unshare(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.conversations.unshare("conv-1")
        mock_req.assert_called_once()
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get_shared(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "conv-1",
            "share_id": "abc123",
            "title": "Shared Chat",
            "message_count": 5,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.conversations.get_shared("abc123")
        assert isinstance(result, SharedConversation)
        assert result.share_id == "abc123"
        client.close()


class TestFoldersResource:
    """Test sync FoldersResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "f1", "name": "Work"},
            {"id": "f2", "name": "Personal"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.folders.list()
        assert isinstance(result, list)
        assert len(result) == 2
        assert isinstance(result[0], FolderInfo)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"folders": [{"id": "f1", "name": "Work"}]}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.folders.list()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "f3",
            "name": "Projects",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.folders.create(name="Projects")
        assert isinstance(result, FolderInfo)
        assert result.name == "Projects"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create_with_parent(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "f4", "name": "Sub"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.folders.create(name="Sub", parent_id="f3")
        body = mock_req.call_args[1]["json"]
        assert body["parent_id"] == "f3"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "f1", "name": "Updated"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.folders.update("f1", name="Updated")
        assert isinstance(result, FolderInfo)
        assert result.name == "Updated"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.folders.delete("f1")
        mock_req.assert_called_once()
        client.close()


# --- Async Tests ---


class TestAsyncConversationsResource:
    """Test async ConversationsResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "c1", "title": "Chat"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.conversations.list()
        assert len(result) == 1
        assert isinstance(result[0], ConversationInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "c1", "title": "New"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.conversations.create(title="New")
        assert isinstance(result, ConversationInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "c1", "title": "Chat", "messages": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.conversations.get("c1")
        assert isinstance(result, ConversationDetail)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.conversations.delete("c1")
        mock_req.assert_called_once()

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list_messages(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "m1", "role": "user", "content": "Hi"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.conversations.list_messages("c1")
        assert len(result) == 1
        assert isinstance(result[0], Message)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create_message(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "m1", "role": "user", "content": "Hi"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.conversations.create_message(
            "c1", MessageCreate(role="user", content="Hi")
        )
        assert isinstance(result, Message)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_share(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"share_id": "s1", "url": "http://x"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.conversations.share("c1")
        assert isinstance(result, ShareLink)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_unshare(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.conversations.unshare("c1")
        mock_req.assert_called_once()


class TestAsyncFoldersResource:
    """Test async FoldersResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "f1", "name": "Work"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.folders.list()
        assert len(result) == 1
        assert isinstance(result[0], FolderInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "f1", "name": "New"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.folders.create("New")
        assert isinstance(result, FolderInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.folders.delete("f1")
        mock_req.assert_called_once()
