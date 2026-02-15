"""Base resource classes for the EdgeQuake SDK.

WHY: Resources are the main API surface — each one maps to a group of related
API endpoints (e.g. DocumentsResource → /api/v1/documents/*). The base class
provides shared HTTP method helpers (_get, _post, _put, _delete) that delegate
to the transport layer.

WHY OODA-06: Added @overload typing to eliminate mypy "Returning Any" errors.
When response_type is provided, return type is T; otherwise Any.

SyncResource uses SyncTransport; AsyncResource uses AsyncTransport.
"""

from __future__ import annotations

from typing import Any, TypeVar, overload

from pydantic import BaseModel

from edgequake._transport import AsyncTransport, SyncTransport

T = TypeVar("T", bound=BaseModel)


class SyncResource:
    """Base class for synchronous API resources.

    Each resource receives a SyncTransport instance, which handles
    HTTP communication, auth headers, retries, and error parsing.
    """

    def __init__(self, transport: SyncTransport) -> None:
        self._transport = transport

    @overload
    def _get(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        response_type: type[T],
    ) -> T: ...

    @overload
    def _get(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        response_type: None = None,
    ) -> Any: ...

    def _get(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        response_type: type[T] | None = None,
    ) -> T | Any:
        """Execute GET request and optionally deserialize to Pydantic model."""
        response = self._transport.request("GET", path, params=params)
        if response_type is not None:
            return response_type.model_validate(response.json())
        return response.json()

    @overload
    def _post(
        self,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        response_type: type[T],
    ) -> T: ...

    @overload
    def _post(
        self,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        response_type: None = None,
    ) -> Any: ...

    def _post(
        self,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        response_type: type[T] | None = None,
    ) -> T | Any:
        """Execute POST request and optionally deserialize to Pydantic model."""
        response = self._transport.request("POST", path, json=json, params=params)
        if response_type is not None:
            return response_type.model_validate(response.json())
        return response.json()

    @overload
    def _put(
        self,
        path: str,
        *,
        json: Any = None,
        response_type: type[T],
    ) -> T: ...

    @overload
    def _put(
        self,
        path: str,
        *,
        json: Any = None,
        response_type: None = None,
    ) -> Any: ...

    def _put(
        self,
        path: str,
        *,
        json: Any = None,
        response_type: type[T] | None = None,
    ) -> T | Any:
        """Execute PUT request and optionally deserialize to Pydantic model."""
        response = self._transport.request("PUT", path, json=json)
        if response_type is not None:
            return response_type.model_validate(response.json())
        return response.json()

    def _delete(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
    ) -> Any:
        """Execute DELETE request. Returns parsed JSON or None for 204."""
        response = self._transport.request("DELETE", path, params=params)
        if response.status_code == 204:
            return None
        try:
            return response.json()
        except Exception:
            return None


class AsyncResource:
    """Base class for asynchronous API resources.

    Same interface as SyncResource but all methods are async.
    """

    def __init__(self, transport: AsyncTransport) -> None:
        self._transport = transport

    @overload
    async def _get(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        response_type: type[T],
    ) -> T: ...

    @overload
    async def _get(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        response_type: None = None,
    ) -> Any: ...

    async def _get(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        response_type: type[T] | None = None,
    ) -> T | Any:
        """Execute async GET request."""
        response = await self._transport.request("GET", path, params=params)
        if response_type is not None:
            return response_type.model_validate(response.json())
        return response.json()

    @overload
    async def _post(
        self,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        response_type: type[T],
    ) -> T: ...

    @overload
    async def _post(
        self,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        response_type: None = None,
    ) -> Any: ...

    async def _post(
        self,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        response_type: type[T] | None = None,
    ) -> T | Any:
        """Execute async POST request."""
        response = await self._transport.request("POST", path, json=json, params=params)
        if response_type is not None:
            return response_type.model_validate(response.json())
        return response.json()

    @overload
    async def _put(
        self,
        path: str,
        *,
        json: Any = None,
        response_type: type[T],
    ) -> T: ...

    @overload
    async def _put(
        self,
        path: str,
        *,
        json: Any = None,
        response_type: None = None,
    ) -> Any: ...

    async def _put(
        self,
        path: str,
        *,
        json: Any = None,
        response_type: type[T] | None = None,
    ) -> T | Any:
        """Execute async PUT request."""
        response = await self._transport.request("PUT", path, json=json)
        if response_type is not None:
            return response_type.model_validate(response.json())
        return response.json()

    async def _delete(
        self,
        path: str,
        *,
        params: dict[str, Any] | None = None,
    ) -> Any:
        """Execute async DELETE request."""
        response = await self._transport.request("DELETE", path, params=params)
        if response.status_code == 204:
            return None
        try:
            return response.json()
        except Exception:
            return None
