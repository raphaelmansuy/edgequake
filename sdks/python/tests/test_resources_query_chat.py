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
            "conversation_id": "conv-1",
            "content": "Hello! How can I help?",
            "sources": [],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.complete(message="Hello")
        assert isinstance(result, ChatCompletionResponse)
        assert result.content == "Hello! How can I help?"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_all_params(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "conv-2",
            "content": "ok",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.chat.complete(
            message="Hi",
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
        """WHY: Test that message string is correctly passed to API."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "conv-3",
            "content": "ok",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.chat.complete(message="Hello, be helpful")
        body = mock_req.call_args[1]["json"]
        assert body["message"] == "Hello, be helpful"
        assert body["stream"] is False
        client.close()

    @patch("edgequake._transport.SyncTransport.stream")
    def test_stream(self, mock_stream: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.stream(
            message="Hello",
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
        result = client.chat.stream(message="Hi")
        assert isinstance(result, SSEStream)
        body = mock_stream.call_args[1]["json"]
        assert "provider" not in body
        assert "max_tokens" not in body
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_sources_and_stats(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "conv-x",
            "content": "ok",
            "sources": [{"document_id": "doc-1", "score": 0.9}],
            "stats": {"total_time_ms": 300},
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.complete(message="test")
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
            "conversation_id": "conv-async-1",
            "content": "Hi async!",
            "sources": [],
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.chat.complete(message="Hello")
        assert isinstance(result, ChatCompletionResponse)
        assert result.content == "Hi async!"

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_complete_with_all_params(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "conv-async-2",
            "content": "ok",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.chat.complete(
            message="hi",
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
            message="Hello",
            provider="ollama",
        )
        assert isinstance(result, AsyncSSEStream)
        body = mock_stream.call_args[1]["json"]
        assert body["stream"] is True
        assert body["provider"] == "ollama"


class TestQueryEdgeCases:
    """WHY: Edge cases for query resource to increase test coverage."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_with_multiple_sources(self, mock_req: MagicMock) -> None:
        """WHY: Verify handling of multiple ranked sources."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "answer": "Combined answer from multiple docs",
            "sources": [
                {"document_id": "doc-1", "chunk_id": "c-1", "content": "First", "score": 0.99},
                {"document_id": "doc-2", "chunk_id": "c-2", "content": "Second", "score": 0.85},
                {"document_id": "doc-3", "chunk_id": "c-3", "content": "Third", "score": 0.72},
            ],
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.query.execute(query="multi-source query")
        assert len(result.sources) == 3
        assert result.sources[0].score == 0.99
        assert result.sources[2].score == 0.72
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_naive_mode(self, mock_req: MagicMock) -> None:
        """WHY: Verify naive mode param is passed correctly."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "naive", "sources": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.query.execute(query="test", mode="naive")
        body = mock_req.call_args[1]["json"]
        assert body["mode"] == "naive"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_global_mode(self, mock_req: MagicMock) -> None:
        """WHY: Verify global mode param is passed correctly."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "global", "sources": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.query.execute(query="test", mode="global")
        body = mock_req.call_args[1]["json"]
        assert body["mode"] == "global"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_default_params(self, mock_req: MagicMock) -> None:
        """WHY: Verify default parameter values are sent correctly."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "ok", "sources": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.query.execute(query="just a query")
        body = mock_req.call_args[1]["json"]
        assert body["query"] == "just a query"
        assert body["mode"] == "hybrid"
        assert body["top_k"] == 10
        assert body["rerank"] is False
        assert "provider" not in body
        assert "model" not in body
        assert "conversation_id" not in body
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_rerank_enabled(self, mock_req: MagicMock) -> None:
        """WHY: Verify rerank boolean is passed correctly."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "reranked", "sources": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.query.execute(query="test", rerank=True)
        body = mock_req.call_args[1]["json"]
        assert body["rerank"] is True
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_execute_custom_top_k(self, mock_req: MagicMock) -> None:
        """WHY: Verify custom top_k values."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"answer": "ok", "sources": []}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.query.execute(query="test", top_k=50)
        body = mock_req.call_args[1]["json"]
        assert body["top_k"] == 50
        client.close()


class TestChatEdgeCases:
    """WHY: Edge cases for chat resource to increase test coverage."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_temperature_zero(self, mock_req: MagicMock) -> None:
        """WHY: Temperature=0 is valid and must be sent (not filtered out)."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "c-1",
            "content": "deterministic",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.chat.complete(message="test", temperature=0.0)
        body = mock_req.call_args[1]["json"]
        assert body["temperature"] == 0.0  # 0.0 must be sent, not omitted
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_max_tokens_zero(self, mock_req: MagicMock) -> None:
        """WHY: max_tokens=0 is an edge case — should still be sent."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "c-1",
            "content": "",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.chat.complete(message="test", max_tokens=0)
        body = mock_req.call_args[1]["json"]
        assert body["max_tokens"] == 0
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_with_top_k(self, mock_req: MagicMock) -> None:
        """WHY: top_k param on chat affects RAG retrieval depth."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "c-1",
            "content": "deep retrieval",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.chat.complete(message="test", top_k=25)
        body = mock_req.call_args[1]["json"]
        assert body["top_k"] == 25
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_minimal_response(self, mock_req: MagicMock) -> None:
        """WHY: API may return minimal response without optional fields."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "c-min",
            "content": "minimal",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.complete(message="test")
        assert result.content == "minimal"
        assert result.conversation_id == "c-min"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_complete_empty_content(self, mock_req: MagicMock) -> None:
        """WHY: Empty content is a valid edge case (model refusal)."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "conversation_id": "c-empty",
            "content": "",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.chat.complete(message="test")
        assert result.content == ""
        client.close()

    @patch("edgequake._transport.SyncTransport.stream")
    def test_stream_with_all_params(self, mock_stream: MagicMock) -> None:
        """WHY: Verify all optional stream parameters are passed."""
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = EdgeQuake()
        client.chat.stream(
            message="Hello",
            mode="local",
            conversation_id="c-1",
            max_tokens=500,
            temperature=0.7,
            provider="openai",
            model="gpt-4",
        )
        body = mock_stream.call_args[1]["json"]
        assert body["stream"] is True
        assert body["mode"] == "local"
        assert body["conversation_id"] == "c-1"
        assert body["max_tokens"] == 500
        assert body["temperature"] == 0.7
        assert body["provider"] == "openai"
        assert body["model"] == "gpt-4"
        client.close()

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.stream", new_callable=AsyncMock)
    async def test_async_stream_with_mode(self, mock_stream: AsyncMock) -> None:
        """WHY: Verify async stream passes mode parameter."""
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.chat.stream(message="test", mode="global")
        assert isinstance(result, AsyncSSEStream)
        body = mock_stream.call_args[1]["json"]
        assert body["mode"] == "global"

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.stream", new_callable=AsyncMock)
    async def test_async_stream_minimal(self, mock_stream: AsyncMock) -> None:
        """WHY: Verify async stream works with minimal params."""
        mock_resp = MagicMock()
        mock_stream.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.chat.stream(message="hello")
        body = mock_stream.call_args[1]["json"]
        assert body["message"] == "hello"
        assert body["stream"] is True
        assert "provider" not in body


class TestQueryRequestModel:
    """WHY: Verify QueryRequest Pydantic model validation."""

    def test_query_request_defaults(self) -> None:
        req = QueryRequest(query="test")
        assert req.mode == "hybrid"
        assert req.top_k == 10
        assert req.rerank is False

    def test_query_request_custom(self) -> None:
        req = QueryRequest(query="test", mode="local", top_k=5, rerank=True)
        assert req.mode == "local"
        assert req.top_k == 5
        assert req.rerank is True

    def test_query_request_serialization(self) -> None:
        req = QueryRequest(query="test", mode="global")
        data = req.model_dump()
        assert data["query"] == "test"
        assert data["mode"] == "global"
