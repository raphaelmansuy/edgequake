"""SSE and WebSocket streaming utilities for the EdgeQuake SDK.

WHY: Server-Sent Events (SSE) is the primary streaming protocol for EdgeQuake
query and chat endpoints. This module provides typed stream parsers that yield
Pydantic models from SSE data lines.

Protocol format:
  data: {"chunk": "Hello "}
  data: {"chunk": "World"}
  data: {"done": true, "stats": {...}}
  data: [DONE]
"""

from __future__ import annotations

from collections.abc import AsyncIterator, Iterator
from typing import Generic, TypeVar

import httpx
from pydantic import BaseModel

from edgequake._errors import StreamError

T = TypeVar("T", bound=BaseModel)


class SSEStream(Generic[T]):
    """Synchronous SSE stream parser.

    Parses text/event-stream lines and yields Pydantic model instances.
    Supports context manager protocol for resource cleanup.

    Usage:
        with SSEStream(response, EventType) as stream:
            for event in stream:
                print(event)
    """

    def __init__(self, response: httpx.Response, event_type: type[T]) -> None:
        self._response = response
        self._event_type = event_type
        self._lines: Iterator[str] | None = None
        self._closed = False

    def __iter__(self) -> Iterator[T]:
        return self

    def __next__(self) -> T:
        if self._closed:
            raise StopIteration

        if self._lines is None:
            self._lines = self._response.iter_lines()

        try:
            for line in self._lines:
                line = line.strip()
                if not line or line.startswith(":"):
                    # WHY: Empty lines are SSE heartbeats; lines starting with ':'
                    # are comments per the SSE spec. Skip both.
                    continue
                if line.startswith("data: "):
                    data = line[6:]
                    if data == "[DONE]":
                        self.close()
                        raise StopIteration
                    try:
                        return self._event_type.model_validate_json(data)
                    except Exception as exc:
                        raise StreamError(f"Failed to parse SSE data: {exc}") from exc
        except StopIteration:
            raise
        except Exception as exc:
            raise StreamError(f"SSE stream error: {exc}") from exc

        raise StopIteration

    def to_string(self) -> str:
        """Collect all chunks into a single string.

        WHY: Convenience method for callers who want the full response
        as a string rather than processing chunks individually.
        Assumes each event has a 'chunk' attribute.
        """
        parts: list[str] = []
        for event in self:
            chunk = getattr(event, "chunk", None)
            if chunk:
                parts.append(chunk)
        return "".join(parts)

    def close(self) -> None:
        """Close the underlying response stream."""
        if not self._closed:
            self._closed = True
            self._response.close()

    def __enter__(self) -> SSEStream[T]:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()


class AsyncSSEStream(Generic[T]):
    """Asynchronous SSE stream parser.

    Same interface as SSEStream but for async/await.

    Usage:
        async with AsyncSSEStream(response, EventType) as stream:
            async for event in stream:
                print(event)
    """

    def __init__(self, response: httpx.Response, event_type: type[T]) -> None:
        self._response = response
        self._event_type = event_type
        self._lines: AsyncIterator[str] | None = None
        self._closed = False

    def __aiter__(self) -> AsyncIterator[T]:
        return self

    async def __anext__(self) -> T:
        if self._closed:
            raise StopAsyncIteration

        if self._lines is None:
            self._lines = self._response.aiter_lines()

        try:
            async for line in self._lines:
                line = line.strip()
                if not line or line.startswith(":"):
                    continue
                if line.startswith("data: "):
                    data = line[6:]
                    if data == "[DONE]":
                        await self.aclose()
                        raise StopAsyncIteration
                    try:
                        return self._event_type.model_validate_json(data)
                    except Exception as exc:
                        raise StreamError(f"Failed to parse SSE data: {exc}") from exc
        except StopAsyncIteration:
            raise
        except Exception as exc:
            raise StreamError(f"SSE stream error: {exc}") from exc

        raise StopAsyncIteration

    async def to_string(self) -> str:
        """Collect all chunks into a single string (async)."""
        parts: list[str] = []
        async for event in self:
            chunk = getattr(event, "chunk", None)
            if chunk:
                parts.append(chunk)
        return "".join(parts)

    async def aclose(self) -> None:
        """Close the underlying response stream."""
        if not self._closed:
            self._closed = True
            await self._response.aclose()

    async def __aenter__(self) -> AsyncSSEStream[T]:
        return self

    async def __aexit__(self, *args: object) -> None:
        await self.aclose()
