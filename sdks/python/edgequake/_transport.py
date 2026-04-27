"""HTTP transport layer for the EdgeQuake SDK.

WHY: Transport abstraction decouples resources from HTTP implementation details.
SyncTransport wraps httpx.Client for synchronous usage; AsyncTransport wraps
httpx.AsyncClient for async/await usage. Both share the same interface patterns
so resources can be written generically.

Retry logic is implemented here at the transport level using exponential backoff
for transient errors (429, 503, connection errors).
"""

from __future__ import annotations

import logging
import time
from pathlib import Path
from typing import Any, BinaryIO

import httpx

from edgequake._config import ClientConfig
from edgequake._errors import ConnectionError as EQConnectionError
from edgequake._errors import TimeoutError as EQTimeoutError
from edgequake._errors import raise_for_status

logger = logging.getLogger("edgequake")

# WHY: Retry these status codes with exponential backoff.
# 429 = rate limited, 503 = service unavailable (both are transient).
_RETRYABLE_STATUS_CODES = {429, 503}

# WHY: Exponential backoff base delays in seconds.
# Attempt 1: 0.5s, Attempt 2: 1.0s, Attempt 3: 2.0s
_RETRY_DELAYS = [0.5, 1.0, 2.0, 4.0, 8.0]


class SyncTransport:
    """Synchronous HTTP transport wrapping httpx.Client.

    Handles auth headers, retries, error parsing, and response deserialization.
    """

    def __init__(self, config: ClientConfig) -> None:
        self._config = config
        self._client = httpx.Client(
            base_url=config.normalized_base_url,
            timeout=httpx.Timeout(config.timeout),
            headers=config.build_headers(),
        )

    def request(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Execute an HTTP request with retry logic.

        Returns the httpx.Response after checking for errors.
        Raises ApiError subclasses on non-2xx responses.
        """
        merged_headers = {**self._config.build_headers(), **(headers or {})}
        last_error: Exception | None = None

        for attempt in range(1 + self._config.max_retries):
            try:
                response = self._client.request(
                    method,
                    path,
                    json=json,
                    params=_clean_params(params),
                    headers=merged_headers,
                )

                # WHY: On 401, attempt JWT refresh if callback is set
                if (
                    response.status_code == 401
                    and self._config.on_token_refresh
                    and self._config.jwt
                ):
                    new_jwt = self._config.on_token_refresh(self._config.jwt)
                    self._config.jwt = new_jwt
                    merged_headers = {**self._config.build_headers(), **(headers or {})}
                    continue

                if (
                    response.status_code in _RETRYABLE_STATUS_CODES
                    and attempt < self._config.max_retries
                ):
                    delay = _get_retry_delay(response, attempt)
                    logger.warning(
                        "Retrying %s %s (status=%d, attempt=%d, delay=%.1fs)",
                        method,
                        path,
                        response.status_code,
                        attempt + 1,
                        delay,
                    )
                    time.sleep(delay)
                    continue

                raise_for_status(response)
                return response

            except (httpx.ConnectError, httpx.RemoteProtocolError) as exc:
                last_error = exc
                if attempt < self._config.max_retries:
                    delay = _RETRY_DELAYS[min(attempt, len(_RETRY_DELAYS) - 1)]
                    logger.warning(
                        "Connection error on %s %s (attempt=%d, delay=%.1fs): %s",
                        method,
                        path,
                        attempt + 1,
                        delay,
                        exc,
                    )
                    time.sleep(delay)
                    continue
                raise EQConnectionError(str(exc)) from exc

            except httpx.TimeoutException as exc:
                raise EQTimeoutError(str(exc)) from exc

        # WHY: Should not reach here, but safety net
        raise EQConnectionError(
            f"All {self._config.max_retries} retries exhausted"
        ) from last_error

    def stream(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Execute a streaming HTTP request.

        Returns the httpx.Response with stream=True for SSE parsing.
        Caller must close the response when done.
        """
        merged_headers = {**self._config.build_headers(), **(headers or {})}
        try:
            response = self._client.send(
                self._client.build_request(
                    method,
                    path,
                    json=json,
                    params=_clean_params(params),
                    headers=merged_headers,
                ),
                stream=True,
            )
            raise_for_status(response)
            return response
        except httpx.ConnectError as exc:
            raise EQConnectionError(str(exc)) from exc
        except httpx.TimeoutException as exc:
            raise EQTimeoutError(str(exc)) from exc

    def upload(
        self,
        path: str,
        *,
        file: Path | BinaryIO,
        filename: str | None = None,
        metadata: dict[str, str] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Upload a file via multipart/form-data.

        WHY: Separate method because multipart uploads use different
        Content-Type handling than JSON requests.
        """
        merged_headers = {**self._config.build_headers(), **(headers or {})}
        # WHY: Remove Content-Type — httpx sets it with boundary for multipart
        merged_headers.pop("Content-Type", None)

        if isinstance(file, Path):
            fname = filename or file.name
            file_obj: BinaryIO = open(file, "rb")  # noqa: SIM115
            should_close = True
        else:
            fname = filename or getattr(file, "name", "upload")
            file_obj = file
            should_close = False

        try:
            files = {"file": (fname, file_obj)}
            data = metadata or {}
            response = self._client.post(
                path,
                files=files,
                data=data,
                headers=merged_headers,
            )
            raise_for_status(response)
            return response
        except httpx.ConnectError as exc:
            raise EQConnectionError(str(exc)) from exc
        except httpx.TimeoutException as exc:
            raise EQTimeoutError(str(exc)) from exc
        finally:
            if should_close:
                file_obj.close()

    def put_upload(
        self,
        path: str,
        *,
        file: Path | BinaryIO,
        filename: str | None = None,
        metadata: dict[str, str] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Multipart PUT (e.g. knowledge injection file)."""
        merged_headers = {**self._config.build_headers(), **(headers or {})}
        merged_headers.pop("Content-Type", None)

        if isinstance(file, Path):
            fname = filename or file.name
            file_obj: BinaryIO = open(file, "rb")  # noqa: SIM115
            should_close = True
        else:
            fname = filename or getattr(file, "name", "upload")
            file_obj = file
            should_close = False

        try:
            files = {"file": (fname, file_obj)}
            data = metadata or {}
            response = self._client.put(
                path,
                files=files,
                data=data,
                headers=merged_headers,
            )
            raise_for_status(response)
            return response
        except httpx.ConnectError as exc:
            raise EQConnectionError(str(exc)) from exc
        except httpx.TimeoutException as exc:
            raise EQTimeoutError(str(exc)) from exc
        finally:
            if should_close:
                file_obj.close()

    def close(self) -> None:
        """Close the underlying httpx.Client."""
        self._client.close()


class AsyncTransport:
    """Asynchronous HTTP transport wrapping httpx.AsyncClient.

    Same interface patterns as SyncTransport but async/await.
    """

    def __init__(self, config: ClientConfig) -> None:
        self._config = config
        self._client = httpx.AsyncClient(
            base_url=config.normalized_base_url,
            timeout=httpx.Timeout(config.timeout),
            headers=config.build_headers(),
        )

    async def request(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Execute an async HTTP request with retry logic."""
        import asyncio

        merged_headers = {**self._config.build_headers(), **(headers or {})}
        last_error: Exception | None = None

        for attempt in range(1 + self._config.max_retries):
            try:
                response = await self._client.request(
                    method,
                    path,
                    json=json,
                    params=_clean_params(params),
                    headers=merged_headers,
                )

                if (
                    response.status_code == 401
                    and self._config.on_token_refresh
                    and self._config.jwt
                ):
                    new_jwt = self._config.on_token_refresh(self._config.jwt)
                    self._config.jwt = new_jwt
                    merged_headers = {**self._config.build_headers(), **(headers or {})}
                    continue

                if (
                    response.status_code in _RETRYABLE_STATUS_CODES
                    and attempt < self._config.max_retries
                ):
                    delay = _get_retry_delay(response, attempt)
                    logger.warning(
                        "Retrying %s %s (status=%d, attempt=%d, delay=%.1fs)",
                        method,
                        path,
                        response.status_code,
                        attempt + 1,
                        delay,
                    )
                    await asyncio.sleep(delay)
                    continue

                raise_for_status(response)
                return response

            except (httpx.ConnectError, httpx.RemoteProtocolError) as exc:
                last_error = exc
                if attempt < self._config.max_retries:
                    delay = _RETRY_DELAYS[min(attempt, len(_RETRY_DELAYS) - 1)]
                    await asyncio.sleep(delay)
                    continue
                raise EQConnectionError(str(exc)) from exc

            except httpx.TimeoutException as exc:
                raise EQTimeoutError(str(exc)) from exc

        raise EQConnectionError(
            f"All {self._config.max_retries} retries exhausted"
        ) from last_error

    async def stream(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Execute an async streaming HTTP request."""
        merged_headers = {**self._config.build_headers(), **(headers or {})}
        try:
            response = await self._client.send(
                self._client.build_request(
                    method,
                    path,
                    json=json,
                    params=_clean_params(params),
                    headers=merged_headers,
                ),
                stream=True,
            )
            raise_for_status(response)
            return response
        except httpx.ConnectError as exc:
            raise EQConnectionError(str(exc)) from exc
        except httpx.TimeoutException as exc:
            raise EQTimeoutError(str(exc)) from exc

    async def upload(
        self,
        path: str,
        *,
        file: Path | BinaryIO,
        filename: str | None = None,
        metadata: dict[str, str] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Upload a file via multipart/form-data (async)."""
        merged_headers = {**self._config.build_headers(), **(headers or {})}
        merged_headers.pop("Content-Type", None)

        if isinstance(file, Path):
            fname = filename or file.name
            file_obj: BinaryIO = open(file, "rb")  # noqa: SIM115
            should_close = True
        else:
            fname = filename or getattr(file, "name", "upload")
            file_obj = file
            should_close = False

        try:
            files = {"file": (fname, file_obj)}
            data = metadata or {}
            response = await self._client.post(
                path,
                files=files,
                data=data,
                headers=merged_headers,
            )
            raise_for_status(response)
            return response
        except httpx.ConnectError as exc:
            raise EQConnectionError(str(exc)) from exc
        except httpx.TimeoutException as exc:
            raise EQTimeoutError(str(exc)) from exc
        finally:
            if should_close:
                file_obj.close()

    async def put_upload(
        self,
        path: str,
        *,
        file: Path | BinaryIO,
        filename: str | None = None,
        metadata: dict[str, str] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Multipart PUT (async)."""
        merged_headers = {**self._config.build_headers(), **(headers or {})}
        merged_headers.pop("Content-Type", None)

        if isinstance(file, Path):
            fname = filename or file.name
            file_obj: BinaryIO = open(file, "rb")  # noqa: SIM115
            should_close = True
        else:
            fname = filename or getattr(file, "name", "upload")
            file_obj = file
            should_close = False

        try:
            files = {"file": (fname, file_obj)}
            data = metadata or {}
            response = await self._client.put(
                path,
                files=files,
                data=data,
                headers=merged_headers,
            )
            raise_for_status(response)
            return response
        except httpx.ConnectError as exc:
            raise EQConnectionError(str(exc)) from exc
        except httpx.TimeoutException as exc:
            raise EQTimeoutError(str(exc)) from exc
        finally:
            if should_close:
                file_obj.close()

    async def close(self) -> None:
        """Close the underlying httpx.AsyncClient."""
        await self._client.aclose()


# ── Helpers ──


def _clean_params(params: dict[str, Any] | None) -> dict[str, Any] | None:
    """Remove None values from query parameters.

    WHY: httpx serializes None as the string "None" which breaks API queries.
    We strip None values so only explicitly set params are sent.
    """
    if params is None:
        return None
    return {k: v for k, v in params.items() if v is not None}


def _get_retry_delay(response: httpx.Response, attempt: int) -> float:
    """Calculate retry delay from Retry-After header or exponential backoff.

    WHY: Respect server's Retry-After header when present (especially for 429).
    Fall back to exponential backoff for other retryable errors.
    """
    retry_after = response.headers.get("retry-after")
    if retry_after:
        try:
            return float(retry_after)
        except ValueError:
            pass
    return _RETRY_DELAYS[min(attempt, len(_RETRY_DELAYS) - 1)]
