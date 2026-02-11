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
    ConnectionError,
    ForbiddenError,
    InternalError,
    NotFoundError,
    RateLimitedError,
    ServiceUnavailableError,
    StreamError,
    TimeoutError,
    UnauthorizedError,
)
from edgequake._transport import AsyncTransport, SyncTransport


class TestSyncTransportHTTPErrors:
    """Test HTTP error response handling."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport instance."""
        config = ClientConfig(base_url="http://test", api_key="test")
        return SyncTransport(config=config)

    def test_request_404_not_found(self, transport: SyncTransport) -> None:
        """Test 404 raises NotFoundError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Not found",
                request=MagicMock(),
                response=MagicMock(status_code=404, text="Not found"),
            )
            with pytest.raises(NotFoundError, match="Not found"):
                transport.request("GET", "/not-found")

    def test_request_401_unauthorized(self, transport: SyncTransport) -> None:
        """Test 401 raises UnauthorizedError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Unauthorized",
                request=MagicMock(),
                response=MagicMock(status_code=401, text="Unauthorized"),
            )
            with pytest.raises(UnauthorizedError, match="Unauthorized"):
                transport.request("GET", "/protected")

    def test_request_403_forbidden(self, transport: SyncTransport) -> None:
        """Test 403 raises ForbiddenError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Forbidden",
                request=MagicMock(),
                response=MagicMock(status_code=403, text="Forbidden"),
            )
            with pytest.raises(ForbiddenError, match="Forbidden"):
                transport.request("GET", "/forbidden")

    def test_request_422_validation_error(self, transport: SyncTransport) -> None:
        """Test 422 raises BadRequestError (validation)."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Validation failed",
                request=MagicMock(),
                response=MagicMock(status_code=422, text="Invalid data"),
            )
            with pytest.raises(BadRequestError, match="Validation failed"):
                transport.request("POST", "/create")

    def test_request_429_rate_limit(self, transport: SyncTransport) -> None:
        """Test 429 raises RateLimitedError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Rate limit exceeded",
                request=MagicMock(),
                response=MagicMock(status_code=429, text="Too many requests"),
            )
            with pytest.raises(RateLimitedError, match="Rate limit exceeded"):
                transport.request("GET", "/api")

    def test_request_500_server_error(self, transport: SyncTransport) -> None:
        """Test 500 raises InternalError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Internal server error",
                request=MagicMock(),
                response=MagicMock(status_code=500, text="Server error"),
            )
            with pytest.raises(InternalError, match="Internal server error"):
                transport.request("GET", "/api")

    def test_request_503_service_unavailable(self, transport: SyncTransport) -> None:
        """Test 503 raises ServiceUnavailableError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Service unavailable",
                request=MagicMock(),
                response=MagicMock(status_code=503, text="Unavailable"),
            )
            with pytest.raises(ServiceUnavailableError, match="Service unavailable"):
                transport.request("GET", "/api")


class TestSyncTransportNetworkErrors:
    """Test network error handling."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport instance."""
        config = ClientConfig(base_url="http://test", api_key="test")
        return SyncTransport(config=config)

    def test_request_timeout(self, transport: SyncTransport) -> None:
        """Test timeout raises TimeoutError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.TimeoutException("Request timeout")
            with pytest.raises(TimeoutError, match="Request timeout"):
                transport.request("GET", "/slow")

    def test_request_connection_refused(self, transport: SyncTransport) -> None:
        """Test connection refused raises ConnectionError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.ConnectError("Connection refused")
            with pytest.raises(ConnectionError, match="Connection refused"):
                transport.request("GET", "/api")

    def test_request_network_error(self, transport: SyncTransport) ->None:
        """Test general network error raises ConnectionError."""
        with patch("httpx.Client.request") as mock_req:
            mock_req.side_effect = httpx.NetworkError("Network unreachable")
            with pytest.raises(ConnectionError, match="Network unreachable"):
                transport.request("GET", "/api")


class TestSyncTransportRequestHandling:
    """Test request construction and parameter handling."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport instance."""
        config = ClientConfig(base_url="http://test", api_key="test")
        return SyncTransport(config=config)

    def test_request_with_headers(self, transport: SyncTransport) -> None:
        """Test custom headers are included in request."""
        with patch("httpx.Client.request") as mock_req:
            mock_resp = MagicMock()
            mock_resp.status_code = 200
            mock_resp.json.return_value = {"result": "ok"}
            mock_req.return_value = mock_resp

            transport.request("GET", "/api", headers={"X-Custom": "value"})

            # Verify headers include API key and custom header
            call_kwargs = mock_req.call_args[1]
            assert "headers" in call_kwargs
            headers = call_kwargs["headers"]
            assert headers.get("Authorization") == "Bearer test"
            assert headers.get("X-Custom") == "value"

    def test_request_with_query_params(self, transport: SyncTransport) -> None:
        """Test query parameters are encoded."""
        with patch("httpx.Client.request") as mock_req:
            mock_resp = MagicMock()
            mock_resp.status_code = 200
            mock_resp.json.return_value = {"result": "ok"}
            mock_req.return_value = mock_resp

            transport.request("GET", "/api", params={"page": 1, "limit": 10})

            call_kwargs = mock_req.call_args[1]
            assert "params" in call_kwargs
            assert call_kwargs["params"]["page"] == 1
            assert call_kwargs["params"]["limit"] == 10

    def test_request_with_json_body(self, transport: SyncTransport) -> None:
        """Test JSON body is serialized."""
        with patch("httpx.Client.request") as mock_req:
            mock_resp = MagicMock()
            mock_resp.status_code = 200
            mock_resp.json.return_value = {"result": "ok"}
            mock_req.return_value = mock_resp

            transport.request("POST", "/api", json={"data": "value"})

            call_kwargs = mock_req.call_args[1]
            assert "json" in call_kwargs
            assert call_kwargs["json"]["data"] == "value"

    def test_response_json_parsing(self, transport: SyncTransport) -> None:
        """Test response JSON is parsed correctly."""
        with patch("httpx.Client.request") as mock_req:
            mock_resp = MagicMock()
            mock_resp.status_code = 200
            mock_resp.json.return_value = {"id": "123", "name": "test"}
            mock_req.return_value = mock_resp

            result = transport.request("GET", "/api")
            assert result.json()["id"] == "123"
            assert result.json()["name"] == "test"

    def test_response_empty_body_204(self, transport: SyncTransport) -> None:
        """Test 204 No Content response."""
        with patch("httpx.Client.request") as mock_req:
            mock_resp = MagicMock()
            mock_resp.status_code = 204
            mock_resp.content = b""
            mock_req.return_value = mock_resp

            result = transport.request("DELETE", "/api/resource/123")
            assert result.status_code == 204


class TestSyncTransportStreaming:
    """Test streaming SSE responses."""

    @pytest.fixture
    def transport(self) -> SyncTransport:
        """Create sync transport instance."""
        config = ClientConfig(base_url="http://test", api_key="test")
        return SyncTransport(config=config)

    def test_stream_basic(self, transport: SyncTransport) -> None:
        """Test basic SSE streaming."""
        with patch("httpx.Client.stream") as mock_stream:
            mock_resp = MagicMock()
            mock_resp.__enter__ = MagicMock(return_value=mock_resp)
            mock_resp.__exit__ = MagicMock()
            mock_resp.iter_lines = MagicMock(
                return_value=iter([
                    "data: {\"chunk\": \"Hello\"}",
                    "",
                    "data: {\"chunk\": \" world\"}",
                    "",
                ])
            )
            mock_stream.return_value = mock_resp

            chunks = list(transport.stream("POST", "/stream", json={"query": "test"}))
            assert len(chunks) == 2
            assert chunks[0]["chunk"] == "Hello"
            assert chunks[1]["chunk"] == " world"

    def test_stream_connection_drop(self, transport: SyncTransport) -> None:
        """Test connection drop mid-stream raises error."""

        def failing_stream():
            yield "data: {\"chunk\": \"Hello\"}"
            yield ""
            raise httpx.NetworkError("Connection lost")

        with patch("httpx.Client.stream") as mock_stream:
            mock_resp = MagicMock()
            mock_resp.__enter__ = MagicMock(return_value=mock_resp)
            mock_resp.__exit__ = MagicMock()
            mock_resp.iter_lines = MagicMock(return_value=failing_stream())
            mock_stream.return_value = mock_resp

            with pytest.raises(ConnectionError, match="Connection lost"):
                list(transport.stream("POST", "/stream"))

    def test_stream_malformed_sse(self, transport: SyncTransport) -> None:
        """Test malformed SSE data is skipped."""
        with patch("httpx.Client.stream") as mock_stream:
            mock_resp = MagicMock()
            mock_resp.__enter__ = MagicMock(return_value=mock_resp)
            mock_resp.__exit__ = MagicMock()
            mock_resp.iter_lines = MagicMock(
                return_value=iter([
                    "data: {\"chunk\": \"Valid\"}",
                    "",
                    "data: not-json",  # Invalid JSON
                    "",
                    "data: {\"chunk\": \"Also valid\"}",
                    "",
                ])
            )
            mock_stream.return_value = mock_resp

            # Should skip invalid JSON and continue
            chunks = list(transport.stream("POST", "/stream"))
            assert len(chunks) >= 1  # At least valid chunks
            assert chunks[0]["chunk"] == "Valid"


class TestAsyncTransportHTTPErrors:
    """Test async transport HTTP error handling."""

    @pytest.fixture
    def transport(self) -> AsyncTransport:
        """Create async transport instance."""
        config = ClientConfig(base_url="http://test", api_key="test")
        return AsyncTransport(config=config)

    @pytest.mark.asyncio
    async def test_request_404_not_found(self, transport: AsyncTransport) -> None:
        """Test 404 raises NotFoundError in async transport."""
        with patch("httpx.AsyncClient.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Not found",
                request=MagicMock(),
                response=MagicMock(status_code=404, text="Not found"),
            )
            with pytest.raises(NotFoundError, match="Not found"):
                await transport.request("GET", "/not-found")

    @pytest.mark.asyncio
    async def test_request_401_unauthorized(self, transport: AsyncTransport) -> None:
        """Test 401 raises UnauthorizedError in async transport."""
        with patch("httpx.AsyncClient.request") as mock_req:
            mock_req.side_effect = httpx.HTTPStatusError(
                "Unauthorized",
                request=MagicMock(),
                response=MagicMock(status_code=401, text="Unauthorized"),
            )
            with pytest.raises(UnauthorizedError, match="Unauthorized"):
                await transport.request("GET", "/protected")

    @pytest.mark.asyncio
    async def test_request_timeout(self, transport: AsyncTransport) -> None:
        """Test timeout raises TimeoutError in async transport."""
        with patch("httpx.AsyncClient.request") as mock_req:
            mock_req.side_effect = httpx.TimeoutException("Request timeout")
            with pytest.raises(TimeoutError, match="Request timeout"):
                await transport.request("GET", "/slow")


class TestAsyncTransportStreaming:
    """Test async transport streaming."""

    @pytest.fixture
    def transport(self) -> AsyncTransport:
        """Create async transport instance."""
        config = ClientConfig(base_url="http://test", api_key="test")
        return AsyncTransport(config=config)

    @pytest.mark.asyncio
    async def test_stream_basic(self, transport: AsyncTransport) -> None:
        """Test basic async SSE streaming."""

        async def async_iter_lines():
            yield "data: {\"chunk\": \"Hello\"}"
            yield ""
            yield "data: {\"chunk\": \" world\"}"
            yield ""

        with patch("httpx.AsyncClient.stream") as mock_stream:
            mock_resp = MagicMock()
            mock_resp.__aenter__ = MagicMock(return_value=mock_resp)
            mock_resp.__aexit__ = MagicMock()
            mock_resp.aiter_lines = MagicMock(return_value=async_iter_lines())
            mock_stream.return_value = mock_resp

            chunks = []
            async for chunk in transport.stream("POST", "/stream", json={"query": "test"}):
                chunks.append(chunk)

            assert len(chunks) == 2
            assert chunks[0]["chunk"] == "Hello"
            assert chunks[1]["chunk"] == " world"

    @pytest.mark.asyncio
    async def test_stream_connection_drop(self, transport: AsyncTransport) -> None:
        """Test connection drop mid-stream raises error in async transport."""

        async def failing_stream():
            yield "data: {\"chunk\": \"Hello\"}"
            yield ""
            raise httpx.NetworkError("Connection lost")

        with patch("httpx.AsyncClient.stream") as mock_stream:
            mock_resp = MagicMock()
            mock_resp.__aenter__ = MagicMock(return_value=mock_resp)
            mock_resp.__aexit__ = MagicMock()
            mock_resp.aiter_lines = MagicMock(return_value=failing_stream())
            mock_stream.return_value = mock_stream

            with pytest.raises(ConnectionError, match="Connection lost"):
                async for _ in transport.stream("POST", "/stream"):
                    pass
