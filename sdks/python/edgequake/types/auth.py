"""Authentication type definitions for the EdgeQuake Python SDK.

WHY: Maps auth endpoint request/response types to Pydantic models.
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel


class LoginRequest(BaseModel):
    """Request body for POST /api/v1/auth/login."""

    username: str
    password: str


class TokenResponse(BaseModel):
    """Response from login/refresh endpoints."""

    access_token: str
    refresh_token: str | None = None
    token_type: str = "Bearer"
    expires_in: int | None = None


class UserInfo(BaseModel):
    """User information from auth/users endpoints."""

    id: str
    username: str | None = None
    email: str | None = None
    role: str | None = None
    tenant_id: str | None = None
    created_at: str | None = None
    last_login: str | None = None


class CreateUserRequest(BaseModel):
    """Request to create a user."""

    username: str
    email: str | None = None
    password: str
    role: str = "user"


class ApiKeyCreateRequest(BaseModel):
    """Request to create an API key."""

    name: str
    expires_in: int | None = None
    scopes: list[str] | None = None


class ApiKeyResponse(BaseModel):
    """Response from creating an API key (includes secret)."""

    id: str
    key: str
    name: str
    created_at: str | None = None
    expires_at: str | None = None


class ApiKeyInfo(BaseModel):
    """API key info (without secret) from list endpoint."""

    id: str
    name: str
    prefix: str | None = None
    created_at: str | None = None
    expires_at: str | None = None
    last_used: str | None = None


class TenantCreate(BaseModel):
    """Request to create a tenant."""

    name: str
    slug: str | None = None
    settings: dict[str, Any] | None = None


class TenantUpdate(BaseModel):
    """Request to update a tenant."""

    name: str | None = None
    settings: dict[str, Any] | None = None


class TenantInfo(BaseModel):
    """Tenant information."""

    id: str
    name: str
    slug: str | None = None
    created_at: str | None = None
    updated_at: str | None = None


class TenantDetail(TenantInfo):
    """Detailed tenant info."""

    settings: dict[str, Any] | None = None
    workspace_count: int | None = None
    user_count: int | None = None
