package io.edgequake.sdk.resources

import com.fasterxml.jackson.core.type.TypeReference
import io.edgequake.sdk.internal.HttpHelper
import io.edgequake.sdk.models.*

/** WHY: Each service maps 1:1 to an API resource for discoverability. */

class HealthService(private val http: HttpHelper) {
    fun check(): HealthResponse = http.get("/health")
}

class DocumentService(private val http: HttpHelper) {
    fun list(page: Int = 1, pageSize: Int = 20): ListDocumentsResponse =
        http.get("/api/v1/documents?page=$page&page_size=$pageSize")

    fun get(id: String): Document = http.get("/api/v1/documents/$id")

    fun uploadText(title: String, content: String): UploadResponse {
        val json = http.postRaw("/api/v1/documents", TextUploadRequest(title, content))
        return http.mapper.readValue(json, UploadResponse::class.java)
    }

    /** WHY: DELETE may return 204 No Content — use deleteRaw to avoid deserialization of empty body. */
    fun delete(id: String) { http.deleteRaw("/api/v1/documents/$id") }

    fun scan(path: String, recursive: Boolean = true): ScanResponse =
        http.post("/api/v1/documents/scan", ScanRequest(path, recursive))
}

class EntityService(private val http: HttpHelper) {
    fun list(page: Int = 1, pageSize: Int = 20): EntityListResponse =
        http.get("/api/v1/graph/entities?page=$page&page_size=$pageSize")

    fun get(name: String): EntityDetailResponse =
        http.get("/api/v1/graph/entities/$name")

    fun create(req: CreateEntityRequest): CreateEntityResponse =
        http.post("/api/v1/graph/entities", req)

    fun delete(name: String): EntityDeleteResponse =
        http.delete("/api/v1/graph/entities/$name?confirm=true")

    fun exists(name: String): EntityExistsResponse =
        http.get("/api/v1/graph/entities/exists?entity_name=$name")

    fun merge(source: String, target: String): Map<String, Any?> =
        http.post("/api/v1/graph/entities/merge", MergeEntitiesRequest(source, target))
}

class RelationshipService(private val http: HttpHelper) {
    fun list(page: Int = 1, pageSize: Int = 20): RelationshipListResponse =
        http.get("/api/v1/graph/relationships?page=$page&page_size=$pageSize")
}

class GraphService(private val http: HttpHelper) {
    fun get(): GraphResponse = http.get("/api/v1/graph")

    fun search(query: String): SearchNodesResponse =
        http.get("/api/v1/graph/nodes/search?q=$query")
}

class QueryService(private val http: HttpHelper) {
    fun execute(query: String, mode: String = "hybrid"): QueryResponse =
        http.post("/api/v1/query", QueryRequest(query, mode))
}

class ChatService(private val http: HttpHelper) {
    fun completions(req: ChatCompletionRequest): ChatCompletionResponse =
        http.post("/api/v1/chat/completions", req)
}

class AuthService(private val http: HttpHelper) {
    fun login(username: String, password: String): TokenResponse =
        http.post("/api/v1/auth/login", LoginRequest(username, password))
}

class UserService(private val http: HttpHelper) {
    fun list(): UserListResponse = http.get("/api/v1/users")
}

class ApiKeyService(private val http: HttpHelper) {
    fun list(): ApiKeyListResponse = http.get("/api/v1/api-keys")
}

class TenantService(private val http: HttpHelper) {
    fun list(): TenantListResponse = http.get("/api/v1/tenants")
}

class ConversationService(private val http: HttpHelper) {
    /** WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array. */
    fun list(): List<ConversationInfo> {
        val wrapper: ConversationListResponse = http.get("/api/v1/conversations")
        return wrapper.items ?: emptyList()
    }

    fun create(title: String): ConversationInfo =
        http.post("/api/v1/conversations", mapOf("title" to title))

    fun get(id: String): ConversationDetail = http.get("/api/v1/conversations/$id")

    /** WHY: DELETE returns 204 No Content — use deleteRaw to avoid deserialization of empty body. */
    fun delete(id: String) { http.deleteRaw("/api/v1/conversations/$id") }

    fun bulkDelete(ids: List<String>): BulkDeleteResponse =
        http.post("/api/v1/conversations/bulk/delete", mapOf("ids" to ids))
}

class FolderService(private val http: HttpHelper) {
    fun list(): List<FolderInfo> = http.get("/api/v1/folders")

    fun create(name: String): FolderInfo =
        http.post("/api/v1/folders", mapOf("name" to name))

    /** WHY: DELETE returns 204 No Content — use deleteRaw to avoid deserialization of empty body. */
    fun delete(id: String) { http.deleteRaw("/api/v1/folders/$id") }
}

class TaskService(private val http: HttpHelper) {
    fun list(): TaskListResponse = http.get("/api/v1/tasks")

    fun get(id: String): TaskInfo = http.get("/api/v1/tasks/$id")
}

class PipelineService(private val http: HttpHelper) {
    fun status(): PipelineStatus = http.get("/api/v1/pipeline/status")

    fun queueMetrics(): QueueMetrics = http.get("/api/v1/pipeline/queue-metrics")
}

class ModelService(private val http: HttpHelper) {
    fun catalog(): ProviderCatalog = http.get("/api/v1/models")

    fun health(): List<ProviderHealthInfo> {
        val json = http.getRaw("/api/v1/models/health")
        return http.mapper.readValue(json, object : TypeReference<List<ProviderHealthInfo>>() {})
    }

    fun providerStatus(): ProviderStatus =
        http.get("/api/v1/settings/provider/status")
}

class WorkspaceService(private val http: HttpHelper) {
    fun list(): List<WorkspaceInfo> = http.get("/api/v1/workspaces")
}

class PdfService(private val http: HttpHelper) {
    fun progress(trackId: String): PdfProgressResponse =
        http.get("/api/v1/documents/pdf/progress/$trackId")

    fun content(pdfId: String): PdfContentResponse =
        http.get("/api/v1/documents/pdf/$pdfId/content")
}

class CostService(private val http: HttpHelper) {
    fun summary(): CostSummary = http.get("/api/v1/costs/summary")
}
