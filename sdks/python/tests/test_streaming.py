"""Tests for edgequake._streaming module."""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest
from pydantic import BaseModel

from edgequake._errors import StreamError
from edgequake._streaming import SSEStream


class ChunkEvent(BaseModel):
    """Test SSE event model."""

    chunk: str


class StatusEvent(BaseModel):
    """Test SSE event without chunk field."""

    status: str


class TestSSEStream:
    """Test synchronous SSE stream parsing."""

    def _make_response(self, lines: list[str]) -> MagicMock:
        """Create a mock httpx.Response with line iterator."""
        response = MagicMock()
        response.iter_lines.return_value = iter(lines)
        response.close = MagicMock()
        return response

    def test_parse_single_event(self) -> None:
        resp = self._make_response(
            [
                'data: {"chunk": "Hello"}',
                "data: [DONE]",
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        events = list(stream)
        assert len(events) == 1
        assert events[0].chunk == "Hello"

    def test_parse_multiple_events(self) -> None:
        resp = self._make_response(
            [
                'data: {"chunk": "Hello "}',
                'data: {"chunk": "World"}',
                "data: [DONE]",
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        events = list(stream)
        assert len(events) == 2
        assert events[0].chunk == "Hello "
        assert events[1].chunk == "World"

    def test_skip_empty_lines(self) -> None:
        resp = self._make_response(
            [
                "",
                'data: {"chunk": "Hello"}',
                "",
                "data: [DONE]",
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        events = list(stream)
        assert len(events) == 1

    def test_skip_comment_lines(self) -> None:
        resp = self._make_response(
            [
                ": this is a comment",
                'data: {"chunk": "Hi"}',
                "data: [DONE]",
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        events = list(stream)
        assert len(events) == 1

    def test_done_sentinel_closes_stream(self) -> None:
        resp = self._make_response(
            [
                'data: {"chunk": "Hello"}',
                "data: [DONE]",
                'data: {"chunk": "should not appear"}',
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        events = list(stream)
        assert len(events) == 1
        resp.close.assert_called()

    def test_to_string(self) -> None:
        resp = self._make_response(
            [
                'data: {"chunk": "Hello "}',
                'data: {"chunk": "World"}',
                "data: [DONE]",
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        assert stream.to_string() == "Hello World"

    def test_to_string_no_chunk_attr(self) -> None:
        resp = self._make_response(
            [
                'data: {"status": "ok"}',
                "data: [DONE]",
            ]
        )
        stream = SSEStream(resp, StatusEvent)
        assert stream.to_string() == ""

    def test_invalid_json_raises_stream_error(self) -> None:
        resp = self._make_response(
            [
                "data: not-json",
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        with pytest.raises(StreamError, match="Failed to parse SSE data"):
            list(stream)

    def test_context_manager(self) -> None:
        resp = self._make_response(
            [
                'data: {"chunk": "Hi"}',
                "data: [DONE]",
            ]
        )
        with SSEStream(resp, ChunkEvent) as stream:
            events = list(stream)
        assert len(events) == 1
        resp.close.assert_called()

    def test_close_idempotent(self) -> None:
        resp = self._make_response(["data: [DONE]"])
        stream = SSEStream(resp, ChunkEvent)
        stream.close()
        stream.close()  # Should not raise
        # close called once by [DONE] + once by explicit close
        assert resp.close.call_count >= 1

    def test_iteration_after_close(self) -> None:
        resp = self._make_response(
            [
                'data: {"chunk": "Hi"}',
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        stream.close()
        events = list(stream)
        assert events == []

    def test_whitespace_handling(self) -> None:
        resp = self._make_response(
            [
                '  data: {"chunk": "Hi"}  ',
                "data: [DONE]",
            ]
        )
        stream = SSEStream(resp, ChunkEvent)
        events = list(stream)
        # Lines are stripped, so "  data: ..." becomes "data: ..."
        assert len(events) == 1
        assert events[0].chunk == "Hi"

    def test_empty_stream(self) -> None:
        resp = self._make_response([])
        stream = SSEStream(resp, ChunkEvent)
        events = list(stream)
        assert events == []
