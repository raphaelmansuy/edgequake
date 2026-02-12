"""Tests for edgequake._transport helper functions."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from edgequake._transport import _clean_params, _get_retry_delay


class TestCleanParams:
    """Test _clean_params helper."""

    def test_none_input(self) -> None:
        assert _clean_params(None) is None

    def test_empty_dict(self) -> None:
        assert _clean_params({}) == {}

    def test_removes_none_values(self) -> None:
        result = _clean_params({"a": 1, "b": None, "c": "hi"})
        assert result == {"a": 1, "c": "hi"}

    def test_keeps_all_non_none(self) -> None:
        result = _clean_params({"x": 0, "y": "", "z": False})
        assert result == {"x": 0, "y": "", "z": False}

    def test_all_none_values(self) -> None:
        result = _clean_params({"a": None, "b": None})
        assert result == {}


class TestGetRetryDelay:
    """Test _get_retry_delay helper."""

    def test_uses_retry_after_header(self) -> None:
        resp = MagicMock()
        resp.headers = {"retry-after": "3"}
        assert _get_retry_delay(resp, 0) == 3.0

    def test_fallback_to_exponential_backoff(self) -> None:
        resp = MagicMock()
        resp.headers = {}
        assert _get_retry_delay(resp, 0) == 0.5
        assert _get_retry_delay(resp, 1) == 1.0
        assert _get_retry_delay(resp, 2) == 2.0

    def test_invalid_retry_after_uses_backoff(self) -> None:
        resp = MagicMock()
        resp.headers = {"retry-after": "invalid"}
        assert _get_retry_delay(resp, 0) == 0.5

    def test_caps_at_max_delay(self) -> None:
        resp = MagicMock()
        resp.headers = {}
        # Attempt way beyond delay list length
        assert _get_retry_delay(resp, 100) == 8.0


# ============================================================================
# COMPREHENSIVE TRANSPORT LAYER TESTS (added for 90% coverage)
# ============================================================================


import httpx
import pytest

from edgequake._config import ClientConfig
from edgequake._errors import (
    BadRequestError,
    ForbiddenError,
    InternalError,
    NotFoundError,
    RateLimitedError,
    ServiceUnavailableError,
    UnauthorizedError,
)
from edgequake._errors import ConnectionError as EQConnectionError
from edgequake._errors import TimeoutError as EQTimeoutError
from edgequake._transport import AsyncTransport, SyncTransport


def _make_error_response(status_code: int, message: str = "error") -> MagicMock:
    """Create a mock httpx.Response that will trigger raise_for_status."""
    resp = MagicMock(spec=httpx.Response)
    resp.status_code = status_code
    resp.is_success = False
    resp.reason_phrase = message
    resp.headers = {}
    resp.json.return_value = {"message": message}
    return resp


def _make_success_response(
    status_code: int = 200, data: dict | None = None
) -> MagicMock:
    """Create a mock httpx.Response for a successful request."""
    resp = MagicMock(spec=httpx.Response)
    resp.status_code = status_code
    resp.is_success = True
    resp.headers = {}
    resp.json.return_value = data or {}
    resp.content = b""
    return resp


class TestSyncTransportHTTPErrors:
    """Test HTTP error response handling via raise_for_status."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport with max_retries=0 to avoid retry loops."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return SyncTransport(config=config)

    def test_request_400_bad_request(self, transport: SyncTransport) -> None:
        """Test 400 raises BadRequestError."""
        transport._client.request = MagicMock(
            return_value=_make_error_response(400, "Bad request")
        )
        with pytest.raises(BadRequestError):
            transport.request("POST", "/bad")

    def test_request_401_unauthorized(self, transport: SyncTransport) -> None:
        """Test 401 raises UnauthorizedError."""
        transport._client.request = MagicMock(
            return_value=_make_error_response(401, "Unauthorized")
        )
        with pytest.raises(UnauthorizedError):
            transport.request("GET", "/protected")

    def test_request_403_forbidden(self, transport: SyncTransport) -> None:
        """Test 403 raises ForbiddenError."""
        transport._client.request = MagicMock(
            return_value=_make_error_response(403, "Forbidden")
        )
        with pytest.raises(ForbiddenError):
            transport.request("GET", "/forbidden")

    def test_request_404_not_found(self, transport: SyncTransport) -> None:
        """Test 404 raises NotFoundError."""
        transport._client.request = MagicMock(
            return_value=_make_error_response(404, "Not found")
        )
        with pytest.raises(NotFoundError):
            transport.request("GET", "/missing")

    def test_request_429_rate_limit(self, transport: SyncTransport) -> None:
        """Test 429 raises RateLimitedError."""
        transport._client.request = MagicMock(
            return_value=_make_error_response(429, "Rate limited")
        )
        with pytest.raises(RateLimitedError):
            transport.request("GET", "/api")

    def test_request_500_internal_error(self, transport: SyncTransport) -> None:
        """Test 500 raises InternalError."""
        transport._client.request = MagicMock(
            return_value=_make_error_response(500, "Server error")
        )
        with pytest.raises(InternalError):
            transport.request("GET", "/api")

    def test_request_503_service_unavailable(self, transport: SyncTransport) -> None:
        """Test 503 raises ServiceUnavailableError."""
        transport._client.request = MagicMock(
            return_value=_make_error_response(503, "Unavailable")
        )
        with pytest.raises(ServiceUnavailableError):
            transport.request("GET", "/api")


class TestSyncTransportNetworkErrors:
    """Test network error handling."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport with max_retries=0 for immediate failure."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return SyncTransport(config=config)

    def test_request_timeout(self, transport: SyncTransport) -> None:
        """Test timeout raises TimeoutError."""
        transport._client.request = MagicMock(
            side_effect=httpx.TimeoutException("Timed out")
        )
        with pytest.raises(EQTimeoutError, match="Timed out"):
            transport.request("GET", "/slow")

    def test_request_connection_refused(self, transport: SyncTransport) -> None:
        """Test connection refused raises ConnectionError."""
        transport._client.request = MagicMock(
            side_effect=httpx.ConnectError("Connection refused")
        )
        with pytest.raises(EQConnectionError, match="Connection refused"):
            transport.request("GET", "/api")

    def test_request_remote_protocol_error(self, transport: SyncTransport) -> None:
        """Test remote protocol error raises ConnectionError."""
        transport._client.request = MagicMock(
            side_effect=httpx.RemoteProtocolError("Protocol error")
        )
        with pytest.raises(EQConnectionError, match="Protocol error"):
            transport.request("GET", "/api")


class TestSyncTransportRetryLogic:
    """Test retry behavior for transient errors."""

    def test_retry_on_429_then_success(self) -> None:
        """Test 429 is retried and eventually succeeds."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = SyncTransport(config=config)

        rate_limit_resp = _make_error_response(429, "Rate limited")
        rate_limit_resp.headers = {"retry-after": "0"}
        success_resp = _make_success_response(200, {"ok": True})

        transport._client.request = MagicMock(
            side_effect=[rate_limit_resp, success_resp]
        )

        with patch("time.sleep"):  # Don't actually sleep
            result = transport.request("GET", "/api")
            assert result.status_code == 200

    def test_retry_on_503_then_success(self) -> None:
        """Test 503 is retried then succeeds."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = SyncTransport(config=config)

        unavailable_resp = _make_error_response(503, "Unavailable")
        unavailable_resp.headers = {"retry-after": "0"}
        success_resp = _make_success_response(200, {"ok": True})

        transport._client.request = MagicMock(
            side_effect=[unavailable_resp, success_resp]
        )

        with patch("time.sleep"):
            result = transport.request("GET", "/api")
            assert result.status_code == 200

    def test_retry_on_connect_error_then_success(self) -> None:
        """Test connection error is retried then succeeds."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = SyncTransport(config=config)

        success_resp = _make_success_response(200, {"ok": True})
        transport._client.request = MagicMock(
            side_effect=[httpx.ConnectError("refused"), success_resp]
        )

        with patch("time.sleep"):
            result = transport.request("GET", "/api")
            assert result.status_code == 200

    def test_retries_exhausted_raises_connection_error(self) -> None:
        """Test all retries exhausted raises ConnectionError."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = SyncTransport(config=config)

        transport._client.request = MagicMock(
            side_effect=httpx.ConnectError("Connection refused")
        )

        with patch("time.sleep"):
            with pytest.raises(EQConnectionError, match="Connection refused"):
                transport.request("GET", "/api")

        # Should have been called max_retries + 1 times
        assert transport._client.request.call_count == 3

    def test_jwt_refresh_on_401(self) -> None:
        """Test JWT token refresh on 401 response."""

        def refresh_token(old_jwt: str) -> str:
            return "new-jwt-token"

        config = ClientConfig(
            base_url="http://test",
            jwt="old-jwt-token",
            max_retries=1,
            on_token_refresh=refresh_token,
        )
        transport = SyncTransport(config=config)

        unauthorized_resp = _make_error_response(401, "Token expired")
        success_resp = _make_success_response(200, {"ok": True})

        transport._client.request = MagicMock(
            side_effect=[unauthorized_resp, success_resp]
        )

        result = transport.request("GET", "/api")
        assert result.status_code == 200
        assert config.jwt == "new-jwt-token"


class TestSyncTransportRequestHandling:
    """Test request construction and parameter handling."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return SyncTransport(config=config)

    def test_request_with_custom_headers(self, transport: SyncTransport) -> None:
        """Test custom headers are merged with default headers."""
        success_resp = _make_success_response(200, {"ok": True})
        transport._client.request = MagicMock(return_value=success_resp)

        transport.request("GET", "/api", headers={"X-Custom": "value"})

        call_kwargs = transport._client.request.call_args
        headers = call_kwargs.kwargs.get("headers") or call_kwargs[1].get("headers", {})
        assert "X-Custom" in headers
        assert headers["X-Custom"] == "value"

    def test_request_with_query_params(self, transport: SyncTransport) -> None:
        """Test query parameters are passed correctly."""
        success_resp = _make_success_response(200)
        transport._client.request = MagicMock(return_value=success_resp)

        transport.request("GET", "/api", params={"page": 1, "limit": 10})

        call_kwargs = transport._client.request.call_args
        params = call_kwargs.kwargs.get("params") or call_kwargs[1].get("params")
        assert params["page"] == 1
        assert params["limit"] == 10

    def test_request_with_json_body(self, transport: SyncTransport) -> None:
        """Test JSON body is sent correctly."""
        success_resp = _make_success_response(200)
        transport._client.request = MagicMock(return_value=success_resp)

        transport.request("POST", "/api", json={"key": "value"})

        call_kwargs = transport._client.request.call_args
        json_body = call_kwargs.kwargs.get("json") or call_kwargs[1].get("json")
        assert json_body["key"] == "value"

    def test_request_params_none_values_removed(self, transport: SyncTransport) -> None:
        """Test None values in params are removed by _clean_params."""
        success_resp = _make_success_response(200)
        transport._client.request = MagicMock(return_value=success_resp)

        transport.request("GET", "/api", params={"a": 1, "b": None, "c": "hi"})

        call_kwargs = transport._client.request.call_args
        params = call_kwargs.kwargs.get("params") or call_kwargs[1].get("params")
        assert params == {"a": 1, "c": "hi"}

    def test_response_json_parsing(self, transport: SyncTransport) -> None:
        """Test response JSON is parsed correctly."""
        success_resp = _make_success_response(200, {"id": "123", "name": "test"})
        transport._client.request = MagicMock(return_value=success_resp)

        result = transport.request("GET", "/api")
        assert result.json()["id"] == "123"

    def test_response_204_no_content(self, transport: SyncTransport) -> None:
        """Test 204 No Content response is handled."""
        resp = _make_success_response(204)
        transport._client.request = MagicMock(return_value=resp)

        result = transport.request("DELETE", "/api/item/1")
        assert result.status_code == 204


class TestSyncTransportStreaming:
    """Test streaming request via send()."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return SyncTransport(config=config)

    def test_stream_returns_response(self, transport: SyncTransport) -> None:
        """Test stream returns a response object."""
        mock_resp = _make_success_response(200)
        transport._client.build_request = MagicMock(return_value=MagicMock())
        transport._client.send = MagicMock(return_value=mock_resp)

        result = transport.stream("POST", "/stream", json={"query": "test"})
        assert result.status_code == 200
        transport._client.send.assert_called_once()

    def test_stream_connection_error(self, transport: SyncTransport) -> None:
        """Test stream raises ConnectionError on ConnectError."""
        transport._client.build_request = MagicMock(return_value=MagicMock())
        transport._client.send = MagicMock(side_effect=httpx.ConnectError("Lost"))

        with pytest.raises(EQConnectionError, match="Lost"):
            transport.stream("POST", "/stream")

    def test_stream_timeout_error(self, transport: SyncTransport) -> None:
        """Test stream raises TimeoutError on timeout."""
        transport._client.build_request = MagicMock(return_value=MagicMock())
        transport._client.send = MagicMock(
            side_effect=httpx.TimeoutException("Timeout")
        )

        with pytest.raises(EQTimeoutError, match="Timeout"):
            transport.stream("POST", "/stream")


class TestSyncTransportUpload:
    """Test file upload via multipart/form-data."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return SyncTransport(config=config)

    def test_upload_binary_io(self, transport: SyncTransport) -> None:
        """Test upload from BinaryIO object."""
        import io

        mock_resp = _make_success_response(200, {"document_id": "doc-1"})
        transport._client.post = MagicMock(return_value=mock_resp)

        file_obj = io.BytesIO(b"Hello world")
        file_obj.name = "test.txt"
        result = transport.upload("/upload", file=file_obj)
        assert result.status_code == 200

    def test_upload_connection_error(self, transport: SyncTransport) -> None:
        """Test upload raises ConnectionError on connect failure."""
        import io

        transport._client.post = MagicMock(
            side_effect=httpx.ConnectError("Connection refused")
        )

        file_obj = io.BytesIO(b"data")
        file_obj.name = "test.txt"
        with pytest.raises(EQConnectionError, match="Connection refused"):
            transport.upload("/upload", file=file_obj)

    def test_upload_timeout_error(self, transport: SyncTransport) -> None:
        """Test upload raises TimeoutError on timeout."""
        import io

        transport._client.post = MagicMock(
            side_effect=httpx.TimeoutException("Upload timed out")
        )

        file_obj = io.BytesIO(b"data")
        file_obj.name = "test.txt"
        with pytest.raises(EQTimeoutError, match="Upload timed out"):
            transport.upload("/upload", file=file_obj)


class TestSyncTransportClose:
    """Test transport close."""

    def test_close_closes_client(self) -> None:
        """Test close delegates to httpx.Client.close."""
        config = ClientConfig(base_url="http://test", api_key="test")
        transport = SyncTransport(config=config)
        transport._client.close = MagicMock()
        transport.close()
        transport._client.close.assert_called_once()


# ── Async Transport Tests ──


class TestAsyncTransportHTTPErrors:
    """Test async transport HTTP error handling."""

    @pytest.fixture
    def transport(self) -> AsyncTransport:
        """Create async transport with no retries."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return AsyncTransport(config=config)

    @pytest.mark.asyncio
    async def test_request_404_not_found(self, transport: AsyncTransport) -> None:
        """Test 404 raises NotFoundError."""
        from unittest.mock import AsyncMock

        transport._client.request = AsyncMock(
            return_value=_make_error_response(404, "Not found")
        )
        with pytest.raises(NotFoundError):
            await transport.request("GET", "/missing")

    @pytest.mark.asyncio
    async def test_request_401_unauthorized(self, transport: AsyncTransport) -> None:
        """Test 401 raises UnauthorizedError."""
        from unittest.mock import AsyncMock

        transport._client.request = AsyncMock(
            return_value=_make_error_response(401, "Unauthorized")
        )
        with pytest.raises(UnauthorizedError):
            await transport.request("GET", "/protected")

    @pytest.mark.asyncio
    async def test_request_timeout(self, transport: AsyncTransport) -> None:
        """Test timeout raises TimeoutError."""
        from unittest.mock import AsyncMock

        transport._client.request = AsyncMock(
            side_effect=httpx.TimeoutException("Timed out")
        )
        with pytest.raises(EQTimeoutError, match="Timed out"):
            await transport.request("GET", "/slow")

    @pytest.mark.asyncio
    async def test_request_connection_error(self, transport: AsyncTransport) -> None:
        """Test connection error raises ConnectionError."""
        from unittest.mock import AsyncMock

        transport._client.request = AsyncMock(side_effect=httpx.ConnectError("refused"))
        with pytest.raises(EQConnectionError, match="refused"):
            await transport.request("GET", "/api")

    @pytest.mark.asyncio
    async def test_request_success(self, transport: AsyncTransport) -> None:
        """Test successful async request."""
        from unittest.mock import AsyncMock

        transport._client.request = AsyncMock(
            return_value=_make_success_response(200, {"ok": True})
        )
        result = await transport.request("GET", "/api")
        assert result.status_code == 200


class TestAsyncTransportStreaming:
    """Test async transport streaming."""

    @pytest.fixture
    def transport(self) -> AsyncTransport:
        """Create async transport."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return AsyncTransport(config=config)

    @pytest.mark.asyncio
    async def test_stream_returns_response(self, transport: AsyncTransport) -> None:
        """Test async stream returns a response."""
        from unittest.mock import AsyncMock

        mock_resp = _make_success_response(200)
        transport._client.build_request = MagicMock(return_value=MagicMock())
        transport._client.send = AsyncMock(return_value=mock_resp)

        result = await transport.stream("POST", "/stream")
        assert result.status_code == 200

    @pytest.mark.asyncio
    async def test_stream_connection_error(self, transport: AsyncTransport) -> None:
        """Test async stream raises ConnectionError."""
        from unittest.mock import AsyncMock

        transport._client.build_request = MagicMock(return_value=MagicMock())
        transport._client.send = AsyncMock(side_effect=httpx.ConnectError("Lost"))

        with pytest.raises(EQConnectionError, match="Lost"):
            await transport.stream("POST", "/stream")

    @pytest.mark.asyncio
    async def test_stream_timeout_error(self, transport: AsyncTransport) -> None:
        """Test async stream raises TimeoutError."""
        from unittest.mock import AsyncMock

        transport._client.build_request = MagicMock(return_value=MagicMock())
        transport._client.send = AsyncMock(
            side_effect=httpx.TimeoutException("Timeout")
        )

        with pytest.raises(EQTimeoutError, match="Timeout"):
            await transport.stream("POST", "/stream")


class TestAsyncTransportUpload:
    """Test async file upload."""

    @pytest.fixture
    def transport(self) -> AsyncTransport:
        """Create async transport."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=0)
        return AsyncTransport(config=config)

    @pytest.mark.asyncio
    async def test_upload_binary_io(self, transport: AsyncTransport) -> None:
        """Test async upload from BinaryIO."""
        import io
        from unittest.mock import AsyncMock

        mock_resp = _make_success_response(200, {"document_id": "doc-1"})
        transport._client.post = AsyncMock(return_value=mock_resp)

        file_obj = io.BytesIO(b"Hello world")
        file_obj.name = "test.txt"
        result = await transport.upload("/upload", file=file_obj)
        assert result.status_code == 200

    @pytest.mark.asyncio
    async def test_upload_connection_error(self, transport: AsyncTransport) -> None:
        """Test async upload connection error."""
        import io
        from unittest.mock import AsyncMock

        transport._client.post = AsyncMock(
            side_effect=httpx.ConnectError("Connection refused")
        )

        file_obj = io.BytesIO(b"data")
        file_obj.name = "test.txt"
        with pytest.raises(EQConnectionError, match="Connection refused"):
            await transport.upload("/upload", file=file_obj)


class TestAsyncTransportClose:
    """Test async transport close."""

    @pytest.mark.asyncio
    async def test_close_closes_client(self) -> None:
        """Test close delegates to httpx.AsyncClient.aclose."""
        from unittest.mock import AsyncMock

        config = ClientConfig(base_url="http://test", api_key="test")
        transport = AsyncTransport(config=config)
        transport._client.aclose = AsyncMock()
        await transport.close()
        transport._client.aclose.assert_called_once()


class TestAsyncTransportRetryLogic:
    """Test async retry behavior for transient errors."""

    @pytest.mark.asyncio
    async def test_retry_on_429_then_success(self) -> None:
        """Test async 429 is retried and eventually succeeds."""
        import asyncio
        from unittest.mock import AsyncMock

        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = AsyncTransport(config=config)

        rate_limit_resp = _make_error_response(429, "Rate limited")
        rate_limit_resp.headers = {"retry-after": "0"}
        success_resp = _make_success_response(200, {"ok": True})

        transport._client.request = AsyncMock(
            side_effect=[rate_limit_resp, success_resp]
        )

        with patch("asyncio.sleep", new_callable=AsyncMock):
            result = await transport.request("GET", "/api")
            assert result.status_code == 200

    @pytest.mark.asyncio
    async def test_retry_on_connect_error_then_success(self) -> None:
        """Test async connection error retry then success."""
        from unittest.mock import AsyncMock

        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = AsyncTransport(config=config)

        success_resp = _make_success_response(200, {"ok": True})
        transport._client.request = AsyncMock(
            side_effect=[httpx.ConnectError("refused"), success_resp]
        )

        with patch("asyncio.sleep", new_callable=AsyncMock):
            result = await transport.request("GET", "/api")
            assert result.status_code == 200

    @pytest.mark.asyncio
    async def test_async_retries_exhausted(self) -> None:
        """Test all async retries exhausted raises ConnectionError."""
        from unittest.mock import AsyncMock

        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = AsyncTransport(config=config)

        transport._client.request = AsyncMock(
            side_effect=httpx.ConnectError("Connection refused")
        )

        with patch("asyncio.sleep", new_callable=AsyncMock):
            with pytest.raises(EQConnectionError, match="Connection refused"):
                await transport.request("GET", "/api")

        assert transport._client.request.call_count == 3

    @pytest.mark.asyncio
    async def test_async_jwt_refresh_on_401(self) -> None:
        """Test async JWT token refresh on 401."""
        from unittest.mock import AsyncMock

        def refresh_token(old_jwt: str) -> str:
            return "new-jwt-token"

        config = ClientConfig(
            base_url="http://test",
            jwt="old-jwt-token",
            max_retries=1,
            on_token_refresh=refresh_token,
        )
        transport = AsyncTransport(config=config)

        unauthorized_resp = _make_error_response(401, "Token expired")
        success_resp = _make_success_response(200, {"ok": True})

        transport._client.request = AsyncMock(
            side_effect=[unauthorized_resp, success_resp]
        )

        result = await transport.request("GET", "/api")
        assert result.status_code == 200
        assert config.jwt == "new-jwt-token"


class TestSyncTransportRetryWithLogger:
    """Test retry logging paths."""

    def test_retry_429_with_logger_warning(self) -> None:
        """Test that 429 retry logs a warning."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=1)
        transport = SyncTransport(config=config)

        rate_limit_resp = _make_error_response(429, "Rate limited")
        rate_limit_resp.headers = {"retry-after": "0.01"}
        success_resp = _make_success_response(200, {"ok": True})

        transport._client.request = MagicMock(
            side_effect=[rate_limit_resp, success_resp]
        )

        with patch("time.sleep"):
            result = transport.request("GET", "/api")
            assert result.status_code == 200
            assert transport._client.request.call_count == 2

    def test_retry_503_exhausted(self) -> None:
        """Test 503 all retries exhausted raises ServiceUnavailableError."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=1)
        transport = SyncTransport(config=config)

        unavailable_resp = _make_error_response(503, "Unavailable")
        unavailable_resp.headers = {"retry-after": "0.01"}

        transport._client.request = MagicMock(
            side_effect=[unavailable_resp, unavailable_resp]
        )

        with patch("time.sleep"):
            with pytest.raises(ServiceUnavailableError):
                transport.request("GET", "/api")

    def test_connection_retry_with_delay(self) -> None:
        """Test connection error retry uses exponential backoff."""
        config = ClientConfig(base_url="http://test", api_key="test", max_retries=2)
        transport = SyncTransport(config=config)

        success_resp = _make_success_response(200, {"ok": True})
        transport._client.request = MagicMock(
            side_effect=[
                httpx.ConnectError("refused"),
                httpx.ConnectError("refused"),
                success_resp,
            ]
        )

        with patch("time.sleep") as mock_sleep:
            result = transport.request("GET", "/api")
            assert result.status_code == 200
            assert mock_sleep.call_count == 2
