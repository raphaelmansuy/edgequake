"""Tests for edgequake package-level imports and __init__.py."""
from __future__ import annotations


class TestPackageImports:
    """Verify all public symbols are importable from edgequake."""

    def test_version(self) -> None:
        from edgequake import __version__
        assert __version__ == "0.4.0"

    def test_client_classes(self) -> None:
        from edgequake import AsyncEdgeQuake, EdgeQuake
        assert EdgeQuake is not None
        assert AsyncEdgeQuake is not None

    def test_error_classes(self) -> None:
        from edgequake import (
            ApiError,
            BadRequestError,
            ConnectionError,
            EdgeQuakeError,
            NotFoundError,
            StreamError,
            TimeoutError,
        )
        # Verify exception hierarchy
        assert issubclass(ApiError, EdgeQuakeError)
        assert issubclass(BadRequestError, ApiError)
        assert issubclass(NotFoundError, ApiError)
        assert issubclass(ConnectionError, EdgeQuakeError)
        assert issubclass(TimeoutError, EdgeQuakeError)
        assert issubclass(StreamError, EdgeQuakeError)

    def test_type_models(self) -> None:
        from edgequake.types import ErrorResponse, HealthResponse, ReadyResponse
        assert ErrorResponse is not None
        assert HealthResponse is not None
        assert ReadyResponse is not None

    def test_resource_base_classes(self) -> None:
        from edgequake.resources import AsyncResource, SyncResource
        assert SyncResource is not None
        assert AsyncResource is not None
