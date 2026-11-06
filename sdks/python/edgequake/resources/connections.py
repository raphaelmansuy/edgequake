"""Connections resource — saved LLM provider endpoints (SPEC-163).

Admin routes. The API key is write-only and never appears on Connection.
"""

from __future__ import annotations

from typing import Any

from edgequake.resources._base import AsyncResource, SyncResource
from edgequake.types.connections import Connection, ProbeResult


def _upsert_body(
    *,
    slug: str,
    display_name: str,
    api_shape: str,
    base_url: str,
    locality: str | None = None,
    auth_scheme: str | None = None,
    api_key: str | None = None,
    timeout_secs: int | None = None,
    allow_private_network: bool | None = None,
) -> dict[str, Any]:
    body: dict[str, Any] = {
        "slug": slug,
        "display_name": display_name,
        "api_shape": api_shape,
        "base_url": base_url,
    }
    if locality is not None:
        body["locality"] = locality
    if auth_scheme is not None:
        body["auth_scheme"] = auth_scheme
    if api_key is not None:
        body["api_key"] = api_key
    if timeout_secs is not None:
        body["timeout_secs"] = timeout_secs
    if allow_private_network is not None:
        body["allow_private_network"] = allow_private_network
    return body


def _probe_body(
    *,
    shape: str,
    base_url: str | None = None,
    model: str | None = None,
    embedding_model: str | None = None,
    api_key: str | None = None,
    auth_scheme: str | None = None,
    allow_private_network: bool | None = None,
    expected_dimension: int | None = None,
) -> dict[str, Any]:
    body: dict[str, Any] = {"shape": shape}
    if base_url is not None:
        body["base_url"] = base_url
    if model is not None:
        body["model"] = model
    if embedding_model is not None:
        body["embedding_model"] = embedding_model
    if api_key is not None:
        body["api_key"] = api_key
    if auth_scheme is not None:
        body["auth_scheme"] = auth_scheme
    if allow_private_network is not None:
        body["allow_private_network"] = allow_private_network
    if expected_dimension is not None:
        body["expected_dimension"] = expected_dimension
    return body


class ConnectionsResource(SyncResource):
    """Synchronous Connections API."""

    def list(self) -> list[Connection]:
        """GET /api/v1/connections"""
        data = self._get("/api/v1/connections")
        return [Connection.model_validate(row) for row in data]

    def create(
        self,
        *,
        slug: str,
        display_name: str,
        api_shape: str,
        base_url: str,
        locality: str | None = None,
        auth_scheme: str | None = None,
        api_key: str | None = None,
        timeout_secs: int | None = None,
        allow_private_network: bool | None = None,
    ) -> Connection:
        """POST /api/v1/connections?scope=deployment"""
        return self._post(
            "/api/v1/connections?scope=deployment",
            json=_upsert_body(
                slug=slug,
                display_name=display_name,
                api_shape=api_shape,
                base_url=base_url,
                locality=locality,
                auth_scheme=auth_scheme,
                api_key=api_key,
                timeout_secs=timeout_secs,
                allow_private_network=allow_private_network,
            ),
            response_type=Connection,
        )

    def update(
        self,
        connection_id: str,
        *,
        slug: str,
        display_name: str,
        api_shape: str,
        base_url: str,
        locality: str | None = None,
        auth_scheme: str | None = None,
        api_key: str | None = None,
        timeout_secs: int | None = None,
        allow_private_network: bool | None = None,
    ) -> Connection:
        """PUT /api/v1/connections/{id}?scope=deployment. Omit api_key to keep the stored key."""
        return self._put(
            f"/api/v1/connections/{connection_id}?scope=deployment",
            json=_upsert_body(
                slug=slug,
                display_name=display_name,
                api_shape=api_shape,
                base_url=base_url,
                locality=locality,
                auth_scheme=auth_scheme,
                api_key=api_key,
                timeout_secs=timeout_secs,
                allow_private_network=allow_private_network,
            ),
            response_type=Connection,
        )

    def delete(self, connection_id: str) -> None:
        """DELETE /api/v1/connections/{id}?scope=deployment"""
        self._delete(f"/api/v1/connections/{connection_id}?scope=deployment")

    def test(self, connection_id: str) -> ProbeResult:
        """POST /api/v1/connections/{id}/test?scope=deployment"""
        return self._post(
            f"/api/v1/connections/{connection_id}/test?scope=deployment",
            response_type=ProbeResult,
        )

    def test_provider(
        self,
        *,
        shape: str,
        base_url: str | None = None,
        model: str | None = None,
        embedding_model: str | None = None,
        api_key: str | None = None,
        auth_scheme: str | None = None,
        allow_private_network: bool | None = None,
        expected_dimension: int | None = None,
    ) -> ProbeResult:
        """POST /api/v1/providers/test — probe without storing a connection."""
        return self._post(
            "/api/v1/providers/test",
            json=_probe_body(
                shape=shape,
                base_url=base_url,
                model=model,
                embedding_model=embedding_model,
                api_key=api_key,
                auth_scheme=auth_scheme,
                allow_private_network=allow_private_network,
                expected_dimension=expected_dimension,
            ),
            response_type=ProbeResult,
        )


class AsyncConnectionsResource(AsyncResource):
    """Asynchronous Connections API."""

    async def list(self) -> list[Connection]:
        data = await self._get("/api/v1/connections")
        return [Connection.model_validate(row) for row in data]

    async def create(
        self,
        *,
        slug: str,
        display_name: str,
        api_shape: str,
        base_url: str,
        locality: str | None = None,
        auth_scheme: str | None = None,
        api_key: str | None = None,
        timeout_secs: int | None = None,
        allow_private_network: bool | None = None,
    ) -> Connection:
        return await self._post(
            "/api/v1/connections?scope=deployment",
            json=_upsert_body(
                slug=slug,
                display_name=display_name,
                api_shape=api_shape,
                base_url=base_url,
                locality=locality,
                auth_scheme=auth_scheme,
                api_key=api_key,
                timeout_secs=timeout_secs,
                allow_private_network=allow_private_network,
            ),
            response_type=Connection,
        )

    async def update(
        self,
        connection_id: str,
        *,
        slug: str,
        display_name: str,
        api_shape: str,
        base_url: str,
        locality: str | None = None,
        auth_scheme: str | None = None,
        api_key: str | None = None,
        timeout_secs: int | None = None,
        allow_private_network: bool | None = None,
    ) -> Connection:
        return await self._put(
            f"/api/v1/connections/{connection_id}?scope=deployment",
            json=_upsert_body(
                slug=slug,
                display_name=display_name,
                api_shape=api_shape,
                base_url=base_url,
                locality=locality,
                auth_scheme=auth_scheme,
                api_key=api_key,
                timeout_secs=timeout_secs,
                allow_private_network=allow_private_network,
            ),
            response_type=Connection,
        )

    async def delete(self, connection_id: str) -> None:
        await self._delete(f"/api/v1/connections/{connection_id}?scope=deployment")

    async def test(self, connection_id: str) -> ProbeResult:
        return await self._post(
            f"/api/v1/connections/{connection_id}/test?scope=deployment",
            response_type=ProbeResult,
        )

    async def test_provider(
        self,
        *,
        shape: str,
        base_url: str | None = None,
        model: str | None = None,
        embedding_model: str | None = None,
        api_key: str | None = None,
        auth_scheme: str | None = None,
        allow_private_network: bool | None = None,
        expected_dimension: int | None = None,
    ) -> ProbeResult:
        return await self._post(
            "/api/v1/providers/test",
            json=_probe_body(
                shape=shape,
                base_url=base_url,
                model=model,
                embedding_model=embedding_model,
                api_key=api_key,
                auth_scheme=auth_scheme,
                allow_private_network=allow_private_network,
                expected_dimension=expected_dimension,
            ),
            response_type=ProbeResult,
        )
