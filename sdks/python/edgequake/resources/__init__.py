"""Resource module re-exports.

All resource classes are exported here for internal use by the client.
"""

from edgequake.resources._base import AsyncResource, SyncResource
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

__all__ = [
    "AsyncResource",
    "SyncResource",
    # Documents
    "DocumentsResource",
    "AsyncDocumentsResource",
    "PdfResource",
    "AsyncPdfResource",
    # Query
    "QueryResource",
    "AsyncQueryResource",
    # Chat
    "ChatResource",
    "AsyncChatResource",
    # Graph
    "GraphResource",
    "AsyncGraphResource",
    "EntitiesResource",
    "AsyncEntitiesResource",
    "RelationshipsResource",
    "AsyncRelationshipsResource",
    # Auth
    "AuthResource",
    "AsyncAuthResource",
    "UsersResource",
    "AsyncUsersResource",
    "ApiKeysResource",
    "AsyncApiKeysResource",
    "TenantsResource",
    "AsyncTenantsResource",
    # Conversations
    "ConversationsResource",
    "AsyncConversationsResource",
    "FoldersResource",
    "AsyncFoldersResource",
    # Operations
    "WorkspacesResource",
    "AsyncWorkspacesResource",
    "TasksResource",
    "AsyncTasksResource",
    "PipelineResource",
    "AsyncPipelineResource",
    "CostsResource",
    "AsyncCostsResource",
    "LineageResource",
    "AsyncLineageResource",
    "ChunksResource",
    "AsyncChunksResource",
    "ProvenanceResource",
    "AsyncProvenanceResource",
    "SettingsResource",
    "AsyncSettingsResource",
    "ModelsResource",
    "AsyncModelsResource",
]
