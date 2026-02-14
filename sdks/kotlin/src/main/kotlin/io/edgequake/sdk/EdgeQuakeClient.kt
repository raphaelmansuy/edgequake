package io.edgequake.sdk

import io.edgequake.sdk.internal.HttpHelper
import io.edgequake.sdk.resources.*

/**
 * Main client for the EdgeQuake API.
 *
 * Usage:
 * ```kotlin
 * val client = EdgeQuakeClient(EdgeQuakeConfig(baseUrl = "http://localhost:8080"))
 * val health = client.health.check()
 * println(health.status)
 * ```
 *
 * WHY: Single entry point with lazy service accessors for clean API.
 */
class EdgeQuakeClient(config: EdgeQuakeConfig = EdgeQuakeConfig()) {

    private val http = HttpHelper(config)

    val health = HealthService(http)
    val documents = DocumentService(http)
    val entities = EntityService(http)
    val relationships = RelationshipService(http)
    val graph = GraphService(http)
    val query = QueryService(http)
    val chat = ChatService(http)
    val auth = AuthService(http)
    val users = UserService(http)
    val apiKeys = ApiKeyService(http)
    val tenants = TenantService(http)
    val conversations = ConversationService(http)
    val folders = FolderService(http)
    val tasks = TaskService(http)
    val pipeline = PipelineService(http)
    val models = ModelService(http)
    val workspaces = WorkspaceService(http)
    val pdf = PdfService(http)
    val costs = CostService(http)
    val lineage = LineageService(http)
    val shared = SharedService(http)
}
