"""Shared type definitions used across all EdgeQuake SDK modules.

WHY: Common response wrappers and base types live here to avoid circular imports
between specific type modules. PaginatedResponse is used by the pagination layer,
and ErrorResponse represents the standard API error format.
"""

from __future__ import annotations

from typing import Any, TypeVar

from pydantic import BaseModel

T = TypeVar("T")


class ErrorResponse(BaseModel):
    """Standard error response from the EdgeQuake API."""

    message: str
    code: str | None = None
    details: dict[str, Any] | None = None


class HealthResponse(BaseModel):
    """Response from GET /health."""

    status: str
    version: str | None = None
    storage_mode: str | None = None
    workspace_id: str | None = None
    components: dict[str, bool] | None = None
    llm_provider_name: str | None = None


class ReadyResponse(BaseModel):
    """Response from GET /ready."""

    ready: bool
    checks: dict[str, bool] | None = None
