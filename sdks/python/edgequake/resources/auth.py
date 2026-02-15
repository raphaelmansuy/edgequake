"""Auth resource — Authentication, users, API keys, and tenants.

WHY: Maps to /api/v1/auth/*, /api/v1/users/*, /api/v1/api-keys/*,
and /api/v1/tenants/* endpoints.
"""

from __future__ import annotations

from typing import Any

from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.auth import (
    ApiKeyCreateRequest,
    ApiKeyInfo,
    ApiKeyResponse,
    CreateUserRequest,
    LoginRequest,
    TenantCreate,
    TenantDetail,
    TenantInfo,
    TenantUpdate,
    TokenResponse,
    UserInfo,
)


class AuthResource(SyncResource):
    """Authentication operations."""

    def login(self, username: str, password: str) -> TokenResponse:
        """Log in and get JWT token.

        POST /api/v1/auth/login
        """
        return self._post(
            "/api/v1/auth/login",
            json=LoginRequest(username=username, password=password).model_dump(),
            response_type=TokenResponse,
        )

    def refresh(self, refresh_token: str) -> TokenResponse:
        """Refresh JWT token.

        POST /api/v1/auth/refresh
        """
        return self._post(
            "/api/v1/auth/refresh",
            json={"refresh_token": refresh_token},
            response_type=TokenResponse,
        )

    def logout(self) -> None:
        """Log out and invalidate token.

        POST /api/v1/auth/logout
        """
        self._post("/api/v1/auth/logout")

    def me(self) -> UserInfo:
        """Get current user info.

        GET /api/v1/auth/me
        """
        return self._get("/api/v1/auth/me", response_type=UserInfo)


class UsersResource(SyncResource):
    """User management operations (admin)."""

    def create(self, user: CreateUserRequest) -> UserInfo:
        """Create a user.

        POST /api/v1/users
        """
        return self._post(
            "/api/v1/users",
            json=user.model_dump(exclude_none=True),
            response_type=UserInfo,
        )

    def list(self) -> list[UserInfo]:
        """List users.

        GET /api/v1/users
        """
        data = self._get("/api/v1/users")
        if isinstance(data, list):
            return [UserInfo.model_validate(u) for u in data]
        items = (
            data.get("users", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [UserInfo.model_validate(u) for u in items]

    def get(self, user_id: str) -> UserInfo:
        """Get user by ID.

        GET /api/v1/users/{user_id}
        """
        return self._get(f"/api/v1/users/{user_id}", response_type=UserInfo)

    def delete(self, user_id: str) -> None:
        """Delete a user.

        DELETE /api/v1/users/{user_id}
        """
        self._delete(f"/api/v1/users/{user_id}")


class ApiKeysResource(SyncResource):
    """API key management."""

    def create(
        self,
        name: str,
        *,
        expires_in: int | None = None,
        scopes: list[str] | None = None,
    ) -> ApiKeyResponse:
        """Create an API key.

        POST /api/v1/api-keys
        """
        body = ApiKeyCreateRequest(name=name, expires_in=expires_in, scopes=scopes)
        return self._post(
            "/api/v1/api-keys",
            json=body.model_dump(exclude_none=True),
            response_type=ApiKeyResponse,
        )

    def list(self) -> list[ApiKeyInfo]:
        """List API keys.

        GET /api/v1/api-keys
        """
        data = self._get("/api/v1/api-keys")
        if isinstance(data, list):
            return [ApiKeyInfo.model_validate(k) for k in data]
        items = (
            data.get("keys", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [ApiKeyInfo.model_validate(k) for k in items]

    def revoke(self, key_id: str) -> None:
        """Revoke an API key.

        DELETE /api/v1/api-keys/{key_id}
        """
        self._delete(f"/api/v1/api-keys/{key_id}")


class TenantsResource(SyncResource):
    """Multi-tenant management."""

    def create(self, tenant: TenantCreate) -> TenantInfo:
        """Create a tenant.

        POST /api/v1/tenants
        """
        return self._post(
            "/api/v1/tenants",
            json=tenant.model_dump(exclude_none=True),
            response_type=TenantInfo,
        )

    def list(self) -> list[TenantInfo]:
        """List tenants.

        GET /api/v1/tenants
        """
        data = self._get("/api/v1/tenants")
        if isinstance(data, list):
            return [TenantInfo.model_validate(t) for t in data]
        items = (
            data.get("tenants", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [TenantInfo.model_validate(t) for t in items]

    def get(self, tenant_id: str) -> TenantDetail:
        """Get tenant details.

        GET /api/v1/tenants/{tenant_id}
        """
        return self._get(f"/api/v1/tenants/{tenant_id}", response_type=TenantDetail)

    def update(self, tenant_id: str, update: TenantUpdate) -> TenantInfo:
        """Update a tenant.

        PUT /api/v1/tenants/{tenant_id}
        """
        return self._put(
            f"/api/v1/tenants/{tenant_id}",
            json=update.model_dump(exclude_none=True),
            response_type=TenantInfo,
        )

    def delete(self, tenant_id: str) -> None:
        """Delete a tenant.

        DELETE /api/v1/tenants/{tenant_id}
        """
        self._delete(f"/api/v1/tenants/{tenant_id}")


# --- Async versions ---


class AsyncAuthResource(AsyncResource):
    """Async authentication operations."""

    async def login(self, username: str, password: str) -> TokenResponse:
        return await self._post(
            "/api/v1/auth/login",
            json={"username": username, "password": password},
            response_type=TokenResponse,
        )

    async def refresh(self, refresh_token: str) -> TokenResponse:
        return await self._post(
            "/api/v1/auth/refresh",
            json={"refresh_token": refresh_token},
            response_type=TokenResponse,
        )

    async def logout(self) -> None:
        await self._post("/api/v1/auth/logout")

    async def me(self) -> UserInfo:
        return await self._get("/api/v1/auth/me", response_type=UserInfo)


class AsyncUsersResource(AsyncResource):
    """Async user management."""

    async def create(self, user: CreateUserRequest) -> UserInfo:
        return await self._post(
            "/api/v1/users",
            json=user.model_dump(exclude_none=True),
            response_type=UserInfo,
        )

    async def list(self) -> list[UserInfo]:
        data = await self._get("/api/v1/users")
        if isinstance(data, list):
            return [UserInfo.model_validate(u) for u in data]
        items = (
            data.get("users", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [UserInfo.model_validate(u) for u in items]

    async def get(self, user_id: str) -> UserInfo:
        return await self._get(f"/api/v1/users/{user_id}", response_type=UserInfo)

    async def delete(self, user_id: str) -> None:
        await self._delete(f"/api/v1/users/{user_id}")


class AsyncApiKeysResource(AsyncResource):
    """Async API key management."""

    async def create(
        self, name: str, *, expires_in: int | None = None
    ) -> ApiKeyResponse:
        body: dict[str, Any] = {"name": name}
        if expires_in is not None:
            body["expires_in"] = expires_in
        return await self._post(
            "/api/v1/api-keys", json=body, response_type=ApiKeyResponse
        )

    async def list(self) -> list[ApiKeyInfo]:
        data = await self._get("/api/v1/api-keys")
        if isinstance(data, list):
            return [ApiKeyInfo.model_validate(k) for k in data]
        items = (
            data.get("keys", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [ApiKeyInfo.model_validate(k) for k in items]

    async def revoke(self, key_id: str) -> None:
        await self._delete(f"/api/v1/api-keys/{key_id}")


class AsyncTenantsResource(AsyncResource):
    """Async tenant management."""

    async def create(self, tenant: TenantCreate) -> TenantInfo:
        return await self._post(
            "/api/v1/tenants",
            json=tenant.model_dump(exclude_none=True),
            response_type=TenantInfo,
        )

    async def list(self) -> list[TenantInfo]:
        data = await self._get("/api/v1/tenants")
        if isinstance(data, list):
            return [TenantInfo.model_validate(t) for t in data]
        items = (
            data.get("tenants", data.get("items", [])) if isinstance(data, dict) else []
        )
        return [TenantInfo.model_validate(t) for t in items]

    async def get(self, tenant_id: str) -> TenantDetail:
        return await self._get(
            f"/api/v1/tenants/{tenant_id}", response_type=TenantDetail
        )

    async def update(self, tenant_id: str, update: TenantUpdate) -> TenantInfo:
        return await self._put(
            f"/api/v1/tenants/{tenant_id}",
            json=update.model_dump(exclude_none=True),
            response_type=TenantInfo,
        )

    async def delete(self, tenant_id: str) -> None:
        await self._delete(f"/api/v1/tenants/{tenant_id}")
