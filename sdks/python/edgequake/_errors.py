"""Exception hierarchy for the EdgeQuake SDK.

WHY: Structured exception hierarchy allows callers to catch specific error
types (e.g. NotFoundError) or broad categories (e.g. ApiError). This mirrors
HTTP status code semantics while providing rich error context (request_id,
error code, details) for debugging.

Exception Tree:
  EdgeQuakeError (base)
  ├── ApiError (HTTP errors from EdgeQuake server)
  │   ├── BadRequestError      (400)
  │   ├── UnauthorizedError    (401)
  │   ├── ForbiddenError       (403)
  │   ├── NotFoundError        (404)
  │   ├── ConflictError        (409)
  │   ├── RateLimitedError     (429)  → retry_after: float
  │   ├── InternalError        (500)
  │   └── ServiceUnavailableError (503)
  ├── ConnectionError          (network unreachable)
  ├── TimeoutError             (request timeout)
  └── StreamError              (SSE/WebSocket errors)
"""

from __future__ import annotations

from typing import Any


class EdgeQuakeError(Exception):
    """Base exception for all EdgeQuake SDK errors."""

    def __init__(self, message: str) -> None:
        self.message = message
        super().__init__(message)


class ApiError(EdgeQuakeError):
    """HTTP error response from the EdgeQuake API.

    Attributes:
        status_code: HTTP status code (e.g. 404, 500)
        code: Machine-readable error code from the API (e.g. "NOT_FOUND")
        details: Additional error details dictionary
        request_id: X-Request-ID header value for tracing
    """

    def __init__(
        self,
        message: str,
        *,
        status_code: int,
        code: str | None = None,
        details: dict[str, Any] | None = None,
        request_id: str | None = None,
    ) -> None:
        self.status_code = status_code
        self.code = code
        self.details = details
        self.request_id = request_id
        super().__init__(message)

    def __str__(self) -> str:
        parts = [f"[{self.status_code}]"]
        if self.code:
            parts.append(self.code)
        parts.append(self.message)
        return " ".join(parts)

    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}(status_code={self.status_code}, "
            f"code={self.code!r}, message={self.message!r})"
        )


class BadRequestError(ApiError):
    """400 Bad Request — invalid input parameters."""

    pass


class UnauthorizedError(ApiError):
    """401 Unauthorized — missing or invalid credentials."""

    pass


class ForbiddenError(ApiError):
    """403 Forbidden — insufficient permissions for the requested resource."""

    pass


class NotFoundError(ApiError):
    """404 Not Found — the requested resource does not exist."""

    pass


class ConflictError(ApiError):
    """409 Conflict — resource already exists or version conflict."""

    pass


class RateLimitedError(ApiError):
    """429 Too Many Requests — rate limit exceeded.

    Attributes:
        retry_after: Seconds to wait before retrying (from Retry-After header)
    """

    def __init__(
        self,
        message: str,
        *,
        retry_after: float | None = None,
        **kwargs: Any,
    ) -> None:
        self.retry_after = retry_after
        super().__init__(message, **kwargs)


class InternalError(ApiError):
    """500 Internal Server Error."""

    pass


class ServiceUnavailableError(ApiError):
    """503 Service Unavailable — server overloaded or in maintenance."""

    pass


class ConnectionError(EdgeQuakeError):
    """Network connection error — server unreachable."""

    pass


class TimeoutError(EdgeQuakeError):
    """Request timeout — server did not respond in time."""

    pass


class StreamError(EdgeQuakeError):
    """SSE or WebSocket streaming error."""

    pass


# WHY: Status code → exception class mapping for _raise_for_status().
# This allows O(1) lookup instead of a chain of if/elif statements.
STATUS_MAP: dict[int, type[ApiError]] = {
    400: BadRequestError,
    401: UnauthorizedError,
    403: ForbiddenError,
    404: NotFoundError,
    409: ConflictError,
    429: RateLimitedError,
    500: InternalError,
    503: ServiceUnavailableError,
}


def raise_for_status(response: Any) -> None:
    """Parse EdgeQuake error response and raise the appropriate exception.

    WHY: Centralized error parsing ensures consistent error handling across
    all transport methods (sync and async). The response is typed as Any to
    avoid importing httpx at module level (it's imported lazily in _transport).

    Args:
        response: An httpx.Response object
    """
    if response.is_success:
        return

    error_cls = STATUS_MAP.get(response.status_code, ApiError)

    try:
        body = response.json()
        message = (
            body.get("message") or body.get("error") or response.reason_phrase or ""
        )
        code = body.get("code")
        details = body.get("details")
    except Exception:
        message = response.reason_phrase or f"HTTP {response.status_code}"
        code = None
        details = None

    kwargs: dict[str, Any] = {
        "status_code": response.status_code,
        "code": code,
        "details": details,
        "request_id": response.headers.get("x-request-id"),
    }

    if error_cls is RateLimitedError:
        retry_after_header = response.headers.get("retry-after")
        kwargs["retry_after"] = (
            float(retry_after_header) if retry_after_header else None
        )

    raise error_cls(message, **kwargs)
