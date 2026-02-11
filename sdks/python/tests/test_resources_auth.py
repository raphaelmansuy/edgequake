"""Tests for auth, users, api_keys, and tenants resources."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from edgequake import EdgeQuake
from edgequake.types.auth import (
    ApiKeyInfo,
    ApiKeyResponse,
    CreateUserRequest,
    TenantCreate,
    TenantInfo,
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
