"""Tests for edgequake._config module."""

from __future__ import annotations

import pytest

from edgequake._config import ClientConfig


class TestClientConfig:
    """Test ClientConfig validation and behavior."""

    def test_default_values(self) -> None:
        """Default config has sensible values."""
        config = ClientConfig()
        assert config.base_url == "http://localhost:8080"
        assert config.api_key is None
        assert config.jwt is None
        assert config.tenant_id is None
        assert config.workspace_id is None
        assert config.timeout == 30.0
        assert config.max_retries == 3

    def test_custom_values(self) -> None:
        """Config accepts custom values."""
        config = ClientConfig(
            base_url="https://api.example.com",
            api_key="key-123",
            jwt="jwt-token",
            tenant_id="tenant-1",
            workspace_id="ws-1",
            user_id="user-1",
            timeout=60.0,
            max_retries=5,
        )
        assert config.base_url == "https://api.example.com"
        assert config.api_key == "key-123"
        assert config.jwt == "jwt-token"
        assert config.tenant_id == "tenant-1"
        assert config.workspace_id == "ws-1"
        assert config.user_id == "user-1"
        assert config.timeout == 60.0
        assert config.max_retries == 5

    def test_normalized_base_url_strips_trailing_slash(self) -> None:
        """normalized_base_url removes trailing slash."""
        config = ClientConfig(base_url="http://localhost:8080/")
        assert config.normalized_base_url == "http://localhost:8080"

    def test_normalized_base_url_no_trailing_slash(self) -> None:
        """normalized_base_url works when there's no trailing slash."""
        config = ClientConfig(base_url="http://localhost:8080")
        assert config.normalized_base_url == "http://localhost:8080"

    def test_build_headers_api_key(self) -> None:
        """build_headers includes X-API-Key when api_key is set."""
        config = ClientConfig(api_key="my-key")
        headers = config.build_headers()
        assert headers["X-API-Key"] == "my-key"
        assert "Authorization" not in headers

    def test_build_headers_jwt(self) -> None:
        """build_headers includes Authorization when jwt is set."""
        config = ClientConfig(jwt="my-jwt")
        headers = config.build_headers()
        assert headers["Authorization"] == "Bearer my-jwt"
        assert "X-API-Key" not in headers

    def test_build_headers_both_auth(self) -> None:
        """When both api_key and jwt are set, both headers are included."""
        config = ClientConfig(api_key="key", jwt="jwt")
        headers = config.build_headers()
        assert headers["X-API-Key"] == "key"
        assert headers["Authorization"] == "Bearer jwt"

    def test_build_headers_tenant_workspace(self) -> None:
        """build_headers includes tenant and workspace headers when set."""
        config = ClientConfig(
            tenant_id="t-1",
            workspace_id="ws-1",
            user_id="u-1",
        )
        headers = config.build_headers()
        assert headers["X-Tenant-ID"] == "t-1"
        assert headers["X-Workspace-ID"] == "ws-1"
        assert headers["X-User-ID"] == "u-1"

    def test_build_headers_user_agent(self) -> None:
        """build_headers always includes User-Agent."""
        config = ClientConfig()
        headers = config.build_headers()
        assert "edgequake-python" in headers["User-Agent"]

    def test_build_headers_accept(self) -> None:
        """build_headers always includes Accept: application/json."""
        config = ClientConfig()
        headers = config.build_headers()
        assert headers["Accept"] == "application/json"

    def test_timeout_validation(self) -> None:
        """Timeout must be positive."""
        with pytest.raises(ValueError):
            ClientConfig(timeout=0)
        with pytest.raises(ValueError):
            ClientConfig(timeout=-1)

    def test_max_retries_validation(self) -> None:
        """max_retries must be 0-10."""
        config = ClientConfig(max_retries=0)
        assert config.max_retries == 0
        config = ClientConfig(max_retries=10)
        assert config.max_retries == 10
        with pytest.raises(ValueError):
            ClientConfig(max_retries=-1)
        with pytest.raises(ValueError):
            ClientConfig(max_retries=11)
