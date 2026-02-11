"""Tests for edgequake._transport helper functions."""

from __future__ import annotations

from unittest.mock import MagicMock

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
