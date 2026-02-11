"""Tests for query and chat resources."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from edgequake import EdgeQuake
from edgequake.types.query import QueryRequest, QueryResponse


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
        # Verify the mode was passed in the JSON body
        call_kwargs = mock_req.call_args
        assert call_kwargs[1]["json"]["mode"] == "graph"
        client.close()


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

        from edgequake.types.chat import ChatCompletionResponse

        client = EdgeQuake()
        result = client.chat.complete(messages=[{"role": "user", "content": "Hello"}])
        assert isinstance(result, ChatCompletionResponse)
        assert result.choices[0].message.content == "Hello! How can I help?"
        assert result.usage.total_tokens == 18
        client.close()
