"""Admin and effective-config API resources."""

from __future__ import annotations

from typing import Any

from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.operations import ServerDefaultsResponse, UpdateTenantQuotaResponse


class AdminResource(SyncResource):
    """PATCH/GET /api/v1/admin/* (quota + server defaults)."""

    def patch_tenant_quota(
        self, tenant_id: str, max_workspaces: int
    ) -> UpdateTenantQuotaResponse:
        response = self._transport.request(
            "PATCH",
            f"/api/v1/admin/tenants/{tenant_id}/quota",
            json={"max_workspaces": max_workspaces},
        )
        return UpdateTenantQuotaResponse.model_validate(response.json())

    def get_server_defaults(self) -> ServerDefaultsResponse:
        return self._get(
            "/api/v1/admin/config/defaults",
            response_type=ServerDefaultsResponse,
        )

    def patch_server_defaults(
        self, default_max_workspaces: int
    ) -> ServerDefaultsResponse:
        response = self._transport.request(
            "PATCH",
            "/api/v1/admin/config/defaults",
            json={"default_max_workspaces": default_max_workspaces},
        )
        return ServerDefaultsResponse.model_validate(response.json())


class EffectiveConfigResource(SyncResource):
    """GET /api/v1/config/effective."""

    def get(self) -> dict[str, Any]:
        return self._get("/api/v1/config/effective")


class AsyncAdminResource(AsyncResource):
    """Async admin API."""

    async def patch_tenant_quota(
        self, tenant_id: str, max_workspaces: int
    ) -> UpdateTenantQuotaResponse:
        response = await self._transport.request(
            "PATCH",
            f"/api/v1/admin/tenants/{tenant_id}/quota",
            json={"max_workspaces": max_workspaces},
        )
        return UpdateTenantQuotaResponse.model_validate(response.json())

    async def get_server_defaults(self) -> ServerDefaultsResponse:
        return await self._get(
            "/api/v1/admin/config/defaults",
            response_type=ServerDefaultsResponse,
        )

    async def patch_server_defaults(
        self, default_max_workspaces: int
    ) -> ServerDefaultsResponse:
        response = await self._transport.request(
            "PATCH",
            "/api/v1/admin/config/defaults",
            json={"default_max_workspaces": default_max_workspaces},
        )
        return ServerDefaultsResponse.model_validate(response.json())


class AsyncEffectiveConfigResource(AsyncResource):
    """Async effective config."""

    async def get(self) -> dict[str, Any]:
        return await self._get("/api/v1/config/effective")
