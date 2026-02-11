"""Client configuration for the EdgeQuake SDK.

WHY: Centralized configuration using Pydantic for validation. This ensures
all client parameters are validated at construction time, preventing
runtime errors from misconfigured clients.
"""

from __future__ import annotations

from collections.abc import Callable

from pydantic import BaseModel, Field


class ClientConfig(BaseModel):
    """Configuration for EdgeQuake client instances.

    All HTTP transport, authentication, and retry settings are stored here.
    Validated by Pydantic at construction time to catch misconfigurations early.
    """

    base_url: str = Field(
        default="http://localhost:8080",
        description="Base URL of the EdgeQuake API server",
    )
    api_key: str | None = Field(
        default=None,
        description="API key for X-API-Key authentication",
    )
    jwt: str | None = Field(
        default=None,
        description="JWT bearer token for Authorization header",
    )
    tenant_id: str | None = Field(
        default=None,
        description="Tenant ID for multi-tenant X-Tenant-ID header",
    )
    workspace_id: str | None = Field(
        default=None,
        description="Workspace ID for X-Workspace-ID header",
    )
    user_id: str | None = Field(
        default=None,
        description="User ID for X-User-ID header",
    )
    timeout: float = Field(
        default=30.0,
        gt=0,
        description="Request timeout in seconds",
    )
    max_retries: int = Field(
        default=3,
        ge=0,
        le=10,
        description="Maximum number of retries on transient errors (429, 503)",
    )
    on_token_refresh: Callable[[str], str] | None = Field(
        default=None,
        description="Callback invoked on 401 to refresh JWT; receives expired JWT, returns new JWT",
        exclude=True,
    )

    model_config = {"arbitrary_types_allowed": True}

    @property
    def normalized_base_url(self) -> str:
        """Return base_url without trailing slash for consistent path joining."""
        return self.base_url.rstrip("/")

    def build_headers(self) -> dict[str, str]:
        """Build default request headers from configuration.

        WHY: Headers are built once per request cycle rather than stored
        statically, because JWT can change via on_token_refresh callback.
        """
        headers: dict[str, str] = {
            "User-Agent": "edgequake-python/0.1.0",
            "Accept": "application/json",
        }
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        if self.jwt:
            headers["Authorization"] = f"Bearer {self.jwt}"
        if self.tenant_id:
            headers["X-Tenant-ID"] = self.tenant_id
        if self.workspace_id:
            headers["X-Workspace-ID"] = self.workspace_id
        if self.user_id:
            headers["X-User-ID"] = self.user_id
        return headers
