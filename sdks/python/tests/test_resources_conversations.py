"""Tests for conversation and folder resources."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from edgequake import EdgeQuake
from edgequake.types.conversations import (
    BulkDeleteResponse,
    ConversationDetail,
    ConversationInfo,
    FolderInfo,
    Message,
    MessageCreate,
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
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.conversations.delete("conv-1")
        mock_req.assert_called_once()
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
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.folders.delete("f1")
        mock_req.assert_called_once()
        client.close()
