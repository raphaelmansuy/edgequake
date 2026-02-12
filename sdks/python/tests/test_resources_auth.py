"""Tests for auth, users, api_keys, and tenants resources."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from edgequake import EdgeQuake
from edgequake._client import AsyncEdgeQuake
from edgequake.types.auth import (
    ApiKeyInfo,
    ApiKeyResponse,
    CreateUserRequest,
    TenantCreate,
    TenantDetail,
    TenantInfo,
    TenantUpdate,
    TokenResponse,
    UserInfo,
)


class TestAuthResource:
    """Test sync AuthResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_login(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "access_token": "jwt-token-123",
            "refresh_token": "refresh-123",
            "token_type": "bearer",
            "expires_in": 3600,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.auth.login(username="admin", password="secret")
        assert isinstance(result, TokenResponse)
        assert result.access_token == "jwt-token-123"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_refresh(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "access_token": "new-jwt",
            "refresh_token": "new-refresh",
            "token_type": "bearer",
            "expires_in": 3600,
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.auth.refresh(refresh_token="old-refresh")
        assert isinstance(result, TokenResponse)
        assert result.access_token == "new-jwt"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_me(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "user-1",
            "username": "admin",
            "email": "admin@test.com",
            "role": "admin",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.auth.me()
        assert isinstance(result, UserInfo)
        assert result.username == "admin"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_logout(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.auth.logout()
        mock_req.assert_called_once()
        client.close()


class TestUsersResource:
    """Test sync UsersResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "user-2",
            "username": "newuser",
            "email": "new@test.com",
            "role": "user",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.users.create(
            CreateUserRequest(
                username="newuser",
                email="new@test.com",
                password="pass123",
            )
        )
        assert isinstance(result, UserInfo)
        assert result.username == "newuser"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "u1", "username": "admin", "email": "a@t.com", "role": "admin"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.users.list()
        assert isinstance(result, list)
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "users": [{"id": "u1", "username": "admin", "email": "a@t.com"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.users.list()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "u1",
            "username": "admin",
            "email": "a@t.com",
            "role": "admin",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.users.get("u1")
        assert isinstance(result, UserInfo)
        assert result.id == "u1"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.users.delete("u1")
        mock_req.assert_called_once()
        client.close()


class TestApiKeysResource:
    """Test sync ApiKeysResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "key-1",
            "key": "eq-key-abc123",
            "name": "test-key",
            "created_at": "2024-01-01T00:00:00Z",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.api_keys.create(name="test-key")
        assert isinstance(result, ApiKeyResponse)
        assert result.key.startswith("eq-key")
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_create_with_scopes(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "key-2",
            "key": "eq-key-xyz",
            "name": "scoped-key",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.api_keys.create(
            name="scoped-key",
            expires_in=3600,
            scopes=["read", "write"],
        )
        assert isinstance(result, ApiKeyResponse)
        body = mock_req.call_args[1]["json"]
        assert body["name"] == "scoped-key"
        assert body["expires_in"] == 3600
        assert body["scopes"] == ["read", "write"]
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "k1", "name": "key-1", "created_at": "2024-01-01T00:00:00Z"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.api_keys.list()
        assert isinstance(result, list)
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"keys": [{"id": "k1", "name": "key-1"}]}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.api_keys.list()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_revoke(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.api_keys.revoke("k1")
        mock_req.assert_called_once()
        client.close()


class TestTenantsResource:
    """Test sync TenantsResource."""

    @patch("edgequake._transport.SyncTransport.request")
    def test_create(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "tenant-1",
            "name": "Acme Corp",
            "slug": "acme-corp",
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tenants.create(TenantCreate(name="Acme Corp"))
        assert isinstance(result, TenantInfo)
        assert result.name == "Acme Corp"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [
            {"id": "t1", "name": "Acme", "slug": "acme"},
        ]
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tenants.list()
        assert isinstance(result, list)
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_list_dict_response(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "tenants": [{"id": "t1", "name": "Acme", "slug": "acme"}]
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tenants.list()
        assert len(result) == 1
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_get(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "t1",
            "name": "Acme",
            "slug": "acme",
            "settings": {},
        }
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tenants.get("t1")
        assert isinstance(result, TenantDetail)
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_update(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "t1", "name": "Updated", "slug": "acme"}
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        result = client.tenants.update("t1", TenantUpdate(name="Updated"))
        assert isinstance(result, TenantInfo)
        assert result.name == "Updated"
        client.close()

    @patch("edgequake._transport.SyncTransport.request")
    def test_delete(self, mock_req: MagicMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = EdgeQuake()
        client.tenants.delete("t1")
        mock_req.assert_called_once()
        client.close()


# --- Async Tests ---


class TestAsyncAuthResource:
    """Test async AuthResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_login(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "access_token": "jwt",
            "refresh_token": "ref",
            "token_type": "bearer",
            "expires_in": 3600,
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.auth.login("admin", "pass")
        assert isinstance(result, TokenResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_refresh(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "access_token": "new",
            "token_type": "bearer",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.auth.refresh("old-refresh")
        assert isinstance(result, TokenResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_logout(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.auth.logout()
        mock_req.assert_called_once()

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_me(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "u1",
            "username": "admin",
            "email": "a@t.com",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.auth.me()
        assert isinstance(result, UserInfo)


class TestAsyncUsersResource:
    """Test async UsersResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "u2",
            "username": "new",
            "email": "n@t.com",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.users.create(
            CreateUserRequest(username="new", email="n@t.com", password="p")
        )
        assert isinstance(result, UserInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "u1", "username": "a"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.users.list()
        assert len(result) == 1

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "u1", "username": "a"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.users.get("u1")
        assert isinstance(result, UserInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.users.delete("u1")
        mock_req.assert_called_once()


class TestAsyncApiKeysResource:
    """Test async ApiKeysResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "k1",
            "key": "eq-key-async",
            "name": "test",
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.api_keys.create("test")
        assert isinstance(result, ApiKeyResponse)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "k1", "name": "test"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.api_keys.list()
        assert len(result) == 1
        assert isinstance(result[0], ApiKeyInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_revoke(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.api_keys.revoke("k1")
        mock_req.assert_called_once()


class TestAsyncTenantsResource:
    """Test async TenantsResource."""

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_create(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "t1", "name": "Acme", "slug": "acme"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.tenants.create(TenantCreate(name="Acme"))
        assert isinstance(result, TenantInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_list(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = [{"id": "t1", "name": "Acme"}]
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.tenants.list()
        assert len(result) == 1

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_get(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "id": "t1",
            "name": "Acme",
            "slug": "acme",
            "settings": {},
        }
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.tenants.get("t1")
        assert isinstance(result, TenantDetail)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_update(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"id": "t1", "name": "Updated", "slug": "acme"}
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        result = await client.tenants.update("t1", TenantUpdate(name="Updated"))
        assert isinstance(result, TenantInfo)

    @pytest.mark.asyncio
    @patch("edgequake._transport.AsyncTransport.request", new_callable=AsyncMock)
    async def test_delete(self, mock_req: AsyncMock) -> None:
        mock_resp = MagicMock()
        mock_resp.status_code = 204
        mock_req.return_value = mock_resp

        client = AsyncEdgeQuake()
        await client.tenants.delete("t1")
        mock_req.assert_called_once()
