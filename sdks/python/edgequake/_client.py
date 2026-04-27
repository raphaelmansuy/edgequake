"""EdgeQuake client classes — the primary public API.

WHY: Two client classes (sync and async) provide the main entry point for the SDK.
EdgeQuake wraps SyncTransport for blocking I/O; AsyncEdgeQuake wraps AsyncTransport
for async/await usage. Both expose the same resource namespaces.

Usage:
    from edgequake import EdgeQuake, AsyncEdgeQuake

    # Sync
    client = EdgeQuake(base_url="http://localhost:8080", api_key="key")
    health = client.health()

    # Async
    async with AsyncEdgeQuake(base_url="...", api_key="key") as client:
        health = await client.health()
"""

from __future__ import annotations

from collections.abc import Callable
from functools import cached_property

from edgequake._config import ClientConfig
from edgequake._transport import AsyncTransport, SyncTransport
from edgequake.resources.admin import (
    AdminResource,
    AsyncAdminResource,
    AsyncEffectiveConfigResource,
    EffectiveConfigResource,
)
from edgequake.resources.auth import (
    ApiKeysResource,
    AsyncApiKeysResource,
    AsyncAuthResource,
    AsyncTenantsResource,
    AsyncUsersResource,
    AuthResource,
    TenantsResource,
    UsersResource,
)
from edgequake.resources.chat import AsyncChatResource, ChatResource
from edgequake.resources.conversations import (
    AsyncConversationsResource,
    AsyncFoldersResource,
    ConversationsResource,
    FoldersResource,
)
from edgequake.resources.documents import (
    AsyncDocumentsResource,
    AsyncPdfResource,
    DocumentsResource,
    PdfResource,
)
from edgequake.resources.graph import (
    AsyncEntitiesResource,
    AsyncGraphResource,
    AsyncRelationshipsResource,
    EntitiesResource,
    GraphResource,
    RelationshipsResource,
)
from edgequake.resources.operations import (
    AsyncChunksResource,
    AsyncCostsResource,
    AsyncLineageResource,
    AsyncModelsResource,
    AsyncPipelineResource,
    AsyncProvenanceResource,
    AsyncSettingsResource,
    AsyncTasksResource,
    AsyncWorkspacesResource,
    ChunksResource,
    CostsResource,
    LineageResource,
    ModelsResource,
    PipelineResource,
    ProvenanceResource,
    SettingsResource,
    TasksResource,
    WorkspacesResource,
)
from edgequake.resources.query import AsyncQueryResource, QueryResource
from edgequake.types.shared import HealthResponse


class EdgeQuake:
    """Synchronous EdgeQuake API client.

    All API calls are blocking. Suitable for scripts, Jupyter notebooks,
    and synchronous web frameworks.

    Resource namespaces are available as cached properties:
        client.documents, client.pdf, client.query, client.chat,
        client.graph, client.entities, client.relationships,
        client.auth, client.users, client.api_keys, client.tenants,
        client.workspaces, client.conversations, client.folders,
        client.tasks, client.pipeline, client.costs, client.lineage,
        client.chunks, client.provenance, client.settings, client.models

    Args:
        base_url: EdgeQuake API server URL (default: http://localhost:8080)
        api_key: API key for X-API-Key authentication
        jwt: JWT bearer token for Authorization header
        tenant_id: Tenant ID for multi-tenant isolation
        workspace_id: Workspace ID for workspace scoping
        user_id: User ID for user-scoped operations
        timeout: Request timeout in seconds (default: 30.0)
        max_retries: Max retries on 429/503 errors (default: 3)
        on_token_refresh: Callback for JWT refresh on 401
    """

    def __init__(
        self,
        *,
        base_url: str = "http://localhost:8080",
        api_key: str | None = None,
        jwt: str | None = None,
        tenant_id: str | None = None,
        workspace_id: str | None = None,
        user_id: str | None = None,
        timeout: float = 30.0,
        max_retries: int = 3,
        on_token_refresh: Callable[[str], str] | None = None,
    ) -> None:
        self._config = ClientConfig(
            base_url=base_url,
            api_key=api_key,
            jwt=jwt,
            tenant_id=tenant_id,
            workspace_id=workspace_id,
            user_id=user_id,
            timeout=timeout,
            max_retries=max_retries,
            on_token_refresh=on_token_refresh,
        )
        self._transport = SyncTransport(self._config)

    # ── Resource namespaces (lazy, cached) ──

    @cached_property
    def documents(self) -> DocumentsResource:
        """Document management — upload, list, delete, track status."""
        return DocumentsResource(self._transport)

    @cached_property
    def pdf(self) -> PdfResource:
        """PDF-specific operations — upload, download, extract content."""
        return PdfResource(self._transport)

    @cached_property
    def query(self) -> QueryResource:
        """RAG query execution with optional streaming."""
        return QueryResource(self._transport)

    @cached_property
    def chat(self) -> ChatResource:
        """Chat completions with optional streaming."""
        return ChatResource(self._transport)

    @cached_property
    def graph(self) -> GraphResource:
        """Knowledge graph exploration and search."""
        return GraphResource(self._transport)

    @cached_property
    def entities(self) -> EntitiesResource:
        """Entity CRUD operations on the knowledge graph."""
        return EntitiesResource(self._transport)

    @cached_property
    def relationships(self) -> RelationshipsResource:
        """Relationship CRUD operations on the knowledge graph."""
        return RelationshipsResource(self._transport)

    @cached_property
    def auth(self) -> AuthResource:
        """Authentication — login, refresh, logout."""
        return AuthResource(self._transport)

    @cached_property
    def users(self) -> UsersResource:
        """User management — create, list, get, delete."""
        return UsersResource(self._transport)

    @cached_property
    def api_keys(self) -> ApiKeysResource:
        """API key management — create, list, revoke."""
        return ApiKeysResource(self._transport)

    @cached_property
    def tenants(self) -> TenantsResource:
        """Tenant management — multi-tenant CRUD."""
        return TenantsResource(self._transport)

    @cached_property
    def workspaces(self) -> WorkspacesResource:
        """Workspace management — create, list, stats, rebuild."""
        return WorkspacesResource(self._transport)

    @cached_property
    def conversations(self) -> ConversationsResource:
        """Conversation management — messages, sharing, bulk ops."""
        return ConversationsResource(self._transport)

    @cached_property
    def folders(self) -> FoldersResource:
        """Conversation folder management."""
        return FoldersResource(self._transport)

    @cached_property
    def tasks(self) -> TasksResource:
        """Background task monitoring — get, list, cancel, retry."""
        return TasksResource(self._transport)

    @cached_property
    def pipeline(self) -> PipelineResource:
        """Pipeline operations — status, queue metrics, cost estimates."""
        return PipelineResource(self._transport)

    @cached_property
    def costs(self) -> CostsResource:
        """Cost tracking — summary, history, budget management."""
        return CostsResource(self._transport)

    @cached_property
    def lineage(self) -> LineageResource:
        """Data lineage — entity and document lineage graphs."""
        return LineageResource(self._transport)

    @cached_property
    def chunks(self) -> ChunksResource:
        """Chunk retrieval by ID."""
        return ChunksResource(self._transport)

    @cached_property
    def provenance(self) -> ProvenanceResource:
        """Provenance tracking for chunks."""
        return ProvenanceResource(self._transport)

    @cached_property
    def settings(self) -> SettingsResource:
        """System settings — provider status, available providers."""
        return SettingsResource(self._transport)

    @cached_property
    def models(self) -> ModelsResource:
        """Model management — list, health, provider details."""
        return ModelsResource(self._transport)

    @cached_property
    def admin(self) -> AdminResource:
        """Admin — tenant quotas and server defaults."""
        return AdminResource(self._transport)

    @cached_property
    def effective_config(self) -> EffectiveConfigResource:
        """Resolved effective configuration."""
        return EffectiveConfigResource(self._transport)

    # ── Core methods ──

    def health(self) -> HealthResponse:
        """Check API server health.

        GET /health
        """
        response = self._transport.request("GET", "/health")
        return HealthResponse.model_validate(response.json())

    def with_workspace(self, workspace_id: str) -> EdgeQuake:
        """Create a new client scoped to a specific workspace.

        WHY: Returns a new client instance rather than mutating the current one,
        so the original client remains unchanged (immutable pattern).
        """
        return EdgeQuake(
            base_url=self._config.base_url,
            api_key=self._config.api_key,
            jwt=self._config.jwt,
            tenant_id=self._config.tenant_id,
            workspace_id=workspace_id,
            user_id=self._config.user_id,
            timeout=self._config.timeout,
            max_retries=self._config.max_retries,
            on_token_refresh=self._config.on_token_refresh,
        )

    def close(self) -> None:
        """Close the underlying HTTP client and release resources."""
        self._transport.close()

    def __enter__(self) -> EdgeQuake:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()

    def __repr__(self) -> str:
        return f"EdgeQuake(base_url={self._config.base_url!r})"


class AsyncEdgeQuake:
    """Asynchronous EdgeQuake API client.

    All API calls are async/await. Suitable for FastAPI, aiohttp,
    and other async frameworks.

    Resource namespaces are available as cached properties:
        client.documents, client.pdf, client.query, client.chat,
        client.graph, client.entities, client.relationships,
        client.auth, client.users, client.api_keys, client.tenants,
        client.workspaces, client.conversations, client.folders,
        client.tasks, client.pipeline, client.costs, client.lineage,
        client.chunks, client.provenance, client.settings, client.models

    Args:
        Same as EdgeQuake.
    """

    def __init__(
        self,
        *,
        base_url: str = "http://localhost:8080",
        api_key: str | None = None,
        jwt: str | None = None,
        tenant_id: str | None = None,
        workspace_id: str | None = None,
        user_id: str | None = None,
        timeout: float = 30.0,
        max_retries: int = 3,
        on_token_refresh: Callable[[str], str] | None = None,
    ) -> None:
        self._config = ClientConfig(
            base_url=base_url,
            api_key=api_key,
            jwt=jwt,
            tenant_id=tenant_id,
            workspace_id=workspace_id,
            user_id=user_id,
            timeout=timeout,
            max_retries=max_retries,
            on_token_refresh=on_token_refresh,
        )
        self._transport = AsyncTransport(self._config)

    # ── Resource namespaces (lazy, cached) ──

    @cached_property
    def documents(self) -> AsyncDocumentsResource:
        """Document management — upload, list, delete, track status."""
        return AsyncDocumentsResource(self._transport)

    @cached_property
    def pdf(self) -> AsyncPdfResource:
        """PDF-specific operations — upload, download, extract content."""
        return AsyncPdfResource(self._transport)

    @cached_property
    def query(self) -> AsyncQueryResource:
        """RAG query execution with optional streaming."""
        return AsyncQueryResource(self._transport)

    @cached_property
    def chat(self) -> AsyncChatResource:
        """Chat completions with optional streaming."""
        return AsyncChatResource(self._transport)

    @cached_property
    def graph(self) -> AsyncGraphResource:
        """Knowledge graph exploration and search."""
        return AsyncGraphResource(self._transport)

    @cached_property
    def entities(self) -> AsyncEntitiesResource:
        """Entity CRUD operations on the knowledge graph."""
        return AsyncEntitiesResource(self._transport)

    @cached_property
    def relationships(self) -> AsyncRelationshipsResource:
        """Relationship CRUD operations on the knowledge graph."""
        return AsyncRelationshipsResource(self._transport)

    @cached_property
    def auth(self) -> AsyncAuthResource:
        """Authentication — login, refresh, logout."""
        return AsyncAuthResource(self._transport)

    @cached_property
    def users(self) -> AsyncUsersResource:
        """User management — create, list, get, delete."""
        return AsyncUsersResource(self._transport)

    @cached_property
    def api_keys(self) -> AsyncApiKeysResource:
        """API key management — create, list, revoke."""
        return AsyncApiKeysResource(self._transport)

    @cached_property
    def tenants(self) -> AsyncTenantsResource:
        """Tenant management — multi-tenant CRUD."""
        return AsyncTenantsResource(self._transport)

    @cached_property
    def workspaces(self) -> AsyncWorkspacesResource:
        """Workspace management — create, list, stats, rebuild."""
        return AsyncWorkspacesResource(self._transport)

    @cached_property
    def conversations(self) -> AsyncConversationsResource:
        """Conversation management — messages, sharing, bulk ops."""
        return AsyncConversationsResource(self._transport)

    @cached_property
    def folders(self) -> AsyncFoldersResource:
        """Conversation folder management."""
        return AsyncFoldersResource(self._transport)

    @cached_property
    def tasks(self) -> AsyncTasksResource:
        """Background task monitoring — get, list, cancel, retry."""
        return AsyncTasksResource(self._transport)

    @cached_property
    def pipeline(self) -> AsyncPipelineResource:
        """Pipeline operations — status, queue metrics, cost estimates."""
        return AsyncPipelineResource(self._transport)

    @cached_property
    def costs(self) -> AsyncCostsResource:
        """Cost tracking — summary, history, budget management."""
        return AsyncCostsResource(self._transport)

    @cached_property
    def lineage(self) -> AsyncLineageResource:
        """Data lineage — entity and document lineage graphs."""
        return AsyncLineageResource(self._transport)

    @cached_property
    def chunks(self) -> AsyncChunksResource:
        """Chunk retrieval by ID."""
        return AsyncChunksResource(self._transport)

    @cached_property
    def provenance(self) -> AsyncProvenanceResource:
        """Provenance tracking for chunks."""
        return AsyncProvenanceResource(self._transport)

    @cached_property
    def settings(self) -> AsyncSettingsResource:
        """System settings — provider status, available providers."""
        return AsyncSettingsResource(self._transport)

    @cached_property
    def models(self) -> AsyncModelsResource:
        """Model management — list, health, provider details."""
        return AsyncModelsResource(self._transport)

    @cached_property
    def admin(self) -> AsyncAdminResource:
        """Admin — tenant quotas and server defaults."""
        return AsyncAdminResource(self._transport)

    @cached_property
    def effective_config(self) -> AsyncEffectiveConfigResource:
        """Resolved effective configuration."""
        return AsyncEffectiveConfigResource(self._transport)

    # ── Core methods ──

    async def health(self) -> HealthResponse:
        """Check API server health.

        GET /health
        """
        response = await self._transport.request("GET", "/health")
        return HealthResponse.model_validate(response.json())

    def with_workspace(self, workspace_id: str) -> AsyncEdgeQuake:
        """Create a new async client scoped to a specific workspace."""
        return AsyncEdgeQuake(
            base_url=self._config.base_url,
            api_key=self._config.api_key,
            jwt=self._config.jwt,
            tenant_id=self._config.tenant_id,
            workspace_id=workspace_id,
            user_id=self._config.user_id,
            timeout=self._config.timeout,
            max_retries=self._config.max_retries,
            on_token_refresh=self._config.on_token_refresh,
        )

    async def close(self) -> None:
        """Close the underlying async HTTP client."""
        await self._transport.close()

    async def __aenter__(self) -> AsyncEdgeQuake:
        return self

    async def __aexit__(self, *args: object) -> None:
        await self.close()

    def __repr__(self) -> str:
        return f"AsyncEdgeQuake(base_url={self._config.base_url!r})"
