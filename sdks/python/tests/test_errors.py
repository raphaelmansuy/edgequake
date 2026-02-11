"""Tests for edgequake._errors module."""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from edgequake._errors import (
    ApiError,
    BadRequestError,
    ConflictError,
    ConnectionError,
    EdgeQuakeError,
    ForbiddenError,
    InternalError,
    NotFoundError,
    RateLimitedError,
    ServiceUnavailableError,
    StreamError,
    TimeoutError,
    UnauthorizedError,
    raise_for_status,
)


class TestExceptionHierarchy:
    """Verify exception inheritance chain."""

    def test_base_error(self) -> None:
        err = EdgeQuakeError("something broke")
        assert str(err) == "something broke"
        assert err.message == "something broke"
        assert isinstance(err, Exception)

    def test_api_error_attributes(self) -> None:
        err = ApiError(
            "not found",
            status_code=404,
            code="NOT_FOUND",
            details={"id": "123"},
            request_id="req-abc",
        )
        assert err.status_code == 404
        assert err.code == "NOT_FOUND"
        assert err.details == {"id": "123"}
        assert err.request_id == "req-abc"
        assert isinstance(err, EdgeQuakeError)

    def test_api_error_str_format(self) -> None:
        err = ApiError("resource missing", status_code=404, code="NOT_FOUND")
        assert str(err) == "[404] NOT_FOUND resource missing"

    def test_api_error_repr(self) -> None:
        err = ApiError("bad", status_code=400, code="INVALID")
        assert "BadRequestError" not in repr(err)  # It's ApiError, not subclass
        assert "ApiError" in repr(err)
        assert "400" in repr(err)

    @pytest.mark.parametrize(
        "cls,parent",
        [
            (BadRequestError, ApiError),
            (UnauthorizedError, ApiError),
            (ForbiddenError, ApiError),
            (NotFoundError, ApiError),
            (ConflictError, ApiError),
            (RateLimitedError, ApiError),
            (InternalError, ApiError),
            (ServiceUnavailableError, ApiError),
            (ConnectionError, EdgeQuakeError),
            (TimeoutError, EdgeQuakeError),
            (StreamError, EdgeQuakeError),
        ],
    )
    def test_subclass_hierarchy(self, cls: type, parent: type) -> None:
        assert issubclass(cls, parent)

    def test_rate_limited_error_retry_after(self) -> None:
        err = RateLimitedError(
            "rate limited",
            retry_after=2.5,
            status_code=429,
        )
        assert err.retry_after == 2.5
        assert err.status_code == 429

    def test_rate_limited_error_no_retry_after(self) -> None:
        err = RateLimitedError("rate limited", status_code=429)
        assert err.retry_after is None


class TestRaiseForStatus:
    """Test raise_for_status() error parsing."""

    def _make_response(
        self,
        status_code: int,
        *,
        json_body: dict | None = None,
        headers: dict | None = None,
        reason: str = "Error",
    ) -> MagicMock:
        """Create a mock httpx.Response."""
        response = MagicMock()
        response.status_code = status_code
        response.is_success = 200 <= status_code < 300
        response.reason_phrase = reason
        response.headers = headers or {}
        if json_body is not None:
            response.json.return_value = json_body
        else:
            response.json.side_effect = Exception("no json")
        return response

    def test_success_no_raise(self) -> None:
        resp = self._make_response(200)
        raise_for_status(resp)  # Should not raise

    def test_400_raises_bad_request(self) -> None:
        resp = self._make_response(
            400, json_body={"message": "invalid input", "code": "VALIDATION_ERROR"}
        )
        with pytest.raises(BadRequestError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.status_code == 400
        assert exc_info.value.code == "VALIDATION_ERROR"
        assert exc_info.value.message == "invalid input"

    def test_401_raises_unauthorized(self) -> None:
        resp = self._make_response(401, json_body={"message": "invalid token"})
        with pytest.raises(UnauthorizedError):
            raise_for_status(resp)

    def test_403_raises_forbidden(self) -> None:
        resp = self._make_response(403, json_body={"message": "access denied"})
        with pytest.raises(ForbiddenError):
            raise_for_status(resp)

    def test_404_raises_not_found(self) -> None:
        resp = self._make_response(
            404, json_body={"message": "not found", "code": "NOT_FOUND"}
        )
        with pytest.raises(NotFoundError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.code == "NOT_FOUND"

    def test_409_raises_conflict(self) -> None:
        resp = self._make_response(409, json_body={"message": "already exists"})
        with pytest.raises(ConflictError):
            raise_for_status(resp)

    def test_429_raises_rate_limited_with_retry_after(self) -> None:
        resp = self._make_response(
            429,
            json_body={"message": "too many requests"},
            headers={"retry-after": "5"},
        )
        with pytest.raises(RateLimitedError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.retry_after == 5.0

    def test_429_no_retry_after_header(self) -> None:
        resp = self._make_response(429, json_body={"message": "slow down"})
        with pytest.raises(RateLimitedError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.retry_after is None

    def test_500_raises_internal(self) -> None:
        resp = self._make_response(500, json_body={"message": "internal error"})
        with pytest.raises(InternalError):
            raise_for_status(resp)

    def test_503_raises_service_unavailable(self) -> None:
        resp = self._make_response(503, json_body={"message": "maintenance"})
        with pytest.raises(ServiceUnavailableError):
            raise_for_status(resp)

    def test_unknown_status_raises_api_error(self) -> None:
        resp = self._make_response(418, json_body={"message": "I'm a teapot"})
        with pytest.raises(ApiError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.status_code == 418

    def test_non_json_response(self) -> None:
        resp = self._make_response(500, reason="Internal Server Error")
        with pytest.raises(InternalError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.message == "Internal Server Error"
        assert exc_info.value.code is None

    def test_request_id_from_headers(self) -> None:
        resp = self._make_response(
            400,
            json_body={"message": "bad"},
            headers={"x-request-id": "trace-123"},
        )
        with pytest.raises(BadRequestError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.request_id == "trace-123"

    def test_error_field_fallback(self) -> None:
        """When 'message' is missing, fallback to 'error' field."""
        resp = self._make_response(400, json_body={"error": "validation failed"})
        with pytest.raises(BadRequestError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.message == "validation failed"

    def test_details_field(self) -> None:
        resp = self._make_response(
            400,
            json_body={
                "message": "invalid",
                "details": {"field": "name", "reason": "required"},
            },
        )
        with pytest.raises(BadRequestError) as exc_info:
            raise_for_status(resp)
        assert exc_info.value.details == {"field": "name", "reason": "required"}
