"""EdgeQuake Python SDK — Official client for the EdgeQuake RAG API.

Public API:
    EdgeQuake         — Synchronous client (blocking I/O)
    AsyncEdgeQuake    — Asynchronous client (async/await)

Usage:
    from edgequake import EdgeQuake

    client = EdgeQuake(base_url="http://localhost:8080", api_key="key")
    health = client.health()
    print(health.status)

    # Access resources via dot notation
    docs = client.documents.list()
    result = client.query.execute(query="What is EdgeQuake?")
"""

from edgequake._client import AsyncEdgeQuake, EdgeQuake
from edgequake._config import ClientConfig
from edgequake._errors import (
    ApiError,
    BadRequestError,
    ConflictError,
    ConnectionError,
    EdgeQuakeError,
    ForbiddenError,
    InternalError,
    NotFoundError,
    RateLimitedError,
    ServiceUnavailableError,
    StreamError,
    TimeoutError,
    UnauthorizedError,
)
from edgequake.types.documents import PdfInfo, PdfUploadOptions, PdfUploadResponse
from edgequake.types.shared import HealthResponse

__all__ = [
    # Clients
    "EdgeQuake",
    "AsyncEdgeQuake",
    # Config
    "ClientConfig",
    # Health
    "HealthResponse",
    # PDF types
    "PdfInfo",
    "PdfUploadOptions",
    "PdfUploadResponse",
    # Errors
    "EdgeQuakeError",
    "ApiError",
    "BadRequestError",
    "UnauthorizedError",
    "ForbiddenError",
    "NotFoundError",
    "ConflictError",
    "RateLimitedError",
    "InternalError",
    "ServiceUnavailableError",
    "ConnectionError",
    "TimeoutError",
    "StreamError",
]

__version__ = "0.4.0"
