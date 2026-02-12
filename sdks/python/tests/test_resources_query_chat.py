"""Tests for query and chat resources."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake._streaming import AsyncSSEStream, SSEStream
from edgequake.types.chat import (
    ChatCompletionChunk,
    ChatCompletionResponse,
    ChatMessage,
)
from edgequake.types.query import QueryRequest, QueryResponse, QueryStreamEvent


class TestQueryResource:
    """Test sync QueryResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "answer": "EdgeQuake is a RAG framework.",
            "sources": [
                {
                    "document_id": "doc-1",
                    "chunk_id": "chunk-1",
                    "content": "EdgeQuake is...",
                    "score": 0.95,
                }
            ],
            "stats": {
                "total_time_ms": 150,
                "retrieval_time_ms": 50,
                "generation_time_ms": 100,
            },
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.query.execute(query="What is EdgeQuake?")
        assert isinstance(result, QueryResponse)
        assert "RAG framework" in result.answer
        assert len(result.sources) == 1
        assert result.sources[0].score == 0.95
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_with_mode(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "answer": "Graph result",
            "sources": [],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.query.execute(query="test", mode="graph")
        assert isinstance(result, QueryResponse)
        mock_req.assert_called_once()
        call_kwargs = mock_req.call_args
        assert call_kwargs[1]["json"]["mode"] == "graph"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_with_all_params(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "ok", "sources": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.query.execute(
            query="test",
            mode="local",
            top_k=5,
            rerank=True,
            provider="openai",
            model="gpt-4",
            conversation_id="conv-1",
        )
        body = mock_req.call_args[1]["json"]
        assert body["mode"] == "local"
        assert body["top_k"] == 5
        assert body["rerank"] is True
        assert body["provider"] == "openai"
        assert body["model"] == "gpt-4"
        assert body["conversation_id"] == "conv-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.stream")
    def test_stream(self, mock_stream: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = EdgeQuake()
        result = client.query.stream(query="test", mode="hybrid", top_k=5)
        assert isinstance(result, SSEStream)
        mock_stream.assert_called_once()
        call_args = mock_stream.call_args
        assert call_args[0] == ("POST", "/api/v1/query/stream")
        assert call_args[1]["json"]["query"] == "test"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_empty_sources(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "no sources", "sources": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.query.execute(query="test")
        assert result.sources == []
        assert result.answer == "no sources"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_with_stats(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "answer": "ok",
            "sources": [],
            "stats": {
                "total_time_ms": 200,
                "input_tokens": 100,
                "output_tokens": 50,
                "total_tokens": 150,
                "model": "gpt-4",
                "provider": "openai",
            },
            "conversation_id": "conv-x",
            "mode": "hybrid",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.query.execute(query="test")
        assert result.stats.total_time_ms == 200
        assert result.stats.total_tokens == 150
        assert result.conversation_id == "conv-x"
        assert result.mode == "hybrid"
        client.close()


class TestAsyncQueryResource:
    """Test async QueryResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_execute(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "answer": "Async answer",
            "sources": [],
            "stats": {"total_time_ms": 100},
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.query.execute(query="What is EdgeQuake?")
        assert isinstance(result, QueryResponse)
        assert result.answer == "Async answer"

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_execute_with_all_params(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "ok", "sources": []}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.query.execute(
            query="test",
            mode="global",
            top_k=20,
            rerank=True,
            provider="ollama",
            model="llama2",
            conversation_id="c-1",
        )
        body = mock_req.call_args[1]["json"]
        assert body["mode"] == "global"
        assert body["top_k"] == 20
        assert body["rerank"] is True

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.stream", new_callable=AsyncMock)
    async def test_stream(self, mock_stream: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.query.stream(query="test")
        assert isinstance(result, AsyncSSEStream)
        mock_stream.assert_called_once()


class TestChatResource:
    """Test sync ChatResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "chat-1",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help?",
                    },
                    "finish_reason": "stop",
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18,
            },
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.complete(messages=[{"role": "user", "content": "Hello"}])
        assert isinstance(result, ChatCompletionResponse)
        assert result.choices[0].message.content == "Hello! How can I help?"
        assert result.usage.total_tokens == 18
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_all_params(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "chat-2",
            "choices": [
                {"index": 0, "message": {"role": "assistant", "content": "ok"}}
            ],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.chat.complete(
            messages=[ChatMessage(role="user", content="Hi")],
            model="gpt-4",
            temperature=0.5,
            max_tokens=100,
            provider="openai",
            conversation_id="conv-1",
            mode="hybrid",
        )
        body = mock_req.call_args[1]["json"]
        assert body["model"] == "gpt-4"
        assert body["temperature"] == 0.5
        assert body["max_tokens"] == 100
        assert body["provider"] == "openai"
        assert body["conversation_id"] == "conv-1"
        assert body["mode"] == "hybrid"
        assert body["stream"] is False
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_pydantic_messages(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "chat-3",
            "choices": [
                {"index": 0, "message": {"role": "assistant", "content": "ok"}}
            ],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.chat.complete(
            messages=[
                ChatMessage(role="system", content="Be helpful"),
                ChatMessage(role="user", content="Hello"),
            ],
        )
        body = mock_req.call_args[1]["json"]
        assert len(body["messages"]) == 2
        assert body["messages"][0]["role"] == "system"
        assert body["messages"][1]["role"] == "user"
        client.close()

    @patch("edgequake._transport.SyncTransport.stream")
    def test_stream(self, mock_stream: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.stream(
            messages=[{"role": "user", "content": "Hello"}],
            model="gpt-4",
            temperature=0.9,
            max_tokens=200,
            provider="openai",
            conversation_id="conv-1",
        )
        assert isinstance(result, SSEStream)
        call_args = mock_stream.call_args
        assert call_args[0] == ("POST", "/api/v1/chat/completions/stream")
        body = call_args[1]["json"]
        assert body["stream"] is True
        assert body["max_tokens"] == 200
        assert body["provider"] == "openai"
        assert body["conversation_id"] == "conv-1"
        client.close()

    @patch("edgequake._transport.SyncTransport.stream")
    def test_stream_minimal_params(self, mock_stream: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.stream(messages=[{"role": "user", "content": "Hi"}])
        assert isinstance(result, SSEStream)
        body = mock_stream.call_args[1]["json"]
        assert "provider" not in body
        assert "max_tokens" not in body
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_sources_and_stats(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "chat-4",
            "choices": [
                {"index": 0, "message": {"role": "assistant", "content": "ok"}}
            ],
            "sources": [{"document_id": "doc-1", "score": 0.9}],
            "stats": {"total_time_ms": 300},
            "conversation_id": "conv-x",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.complete(messages=[{"role": "user", "content": "test"}])
        assert result.sources is not None
        assert len(result.sources) == 1
        assert result.stats.total_time_ms == 300
        assert result.conversation_id == "conv-x"
        client.close()


class TestAsyncChatResource:
    """Test async ChatResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_complete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "chat-async-1",
            "choices": [
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": "Hi async!"},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8},
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.chat.complete(
            messages=[{"role": "user", "content": "Hello"}]
        )
        assert isinstance(result, ChatCompletionResponse)
        assert result.choices[0].message.content == "Hi async!"

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_complete_with_all_params(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "chat-async-2",
            "choices": [
                {"index": 0, "message": {"role": "assistant", "content": "ok"}}
            ],
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.chat.complete(
            messages=[ChatMessage(role="user", content="hi")],
            model="gpt-4",
            temperature=0.3,
            max_tokens=50,
            provider="openai",
            conversation_id="c-1",
            mode="local",
        )
        body = mock_req.call_args[1]["json"]
        assert body["mode"] == "local"
        assert body["max_tokens"] == 50

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.stream", new_callable=AsyncMock)
    async def test_stream(self, mock_stream: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.chat.stream(
            messages=[{"role": "user", "content": "Hello"}],
            provider="ollama",
        )
        assert isinstance(result, AsyncSSEStream)
        body = mock_stream.call_args[1]["json"]
        assert body["stream"] is True
        assert body["provider"] == "ollama"
