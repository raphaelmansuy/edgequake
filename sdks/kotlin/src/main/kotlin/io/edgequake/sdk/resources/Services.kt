package io.edgequake.sdk.resources

import com.fasterxml.jackson.core.type.TypeReference
import io.edgequake.sdk.internal.HttpHelper
import io.edgequake.sdk.models.*
import java.net.URLEncoder
import java.nio.charset.StandardCharsets

/** WHY: Each service maps 1:1 to an API resource for discoverability. */

class HealthService(private val http: HttpHelper) {
    fun check(): HealthResponse = http.get("/health")
    
    /** WHY: /ready checks database and provider connectivity. */
    fun ready(): ReadinessResponse = http.get("/ready")
    
    /** WHY: /live checks if server is responding (lightweight). */
    fun live(): LivenessResponse = http.get("/live")
    
    /** WHY: /metrics returns Prometheus-style metrics. */
    fun metrics(): String = http.getRaw("/metrics")
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

    /** WHY: GET /api/v1/documents/track/{track_id} */
    fun track(trackId: String): TrackStatusResponse =
        http.get("/api/v1/documents/track/$trackId")

    /** WHY: POST /api/v1/documents/reprocess — workspace-scoped via headers. */
    fun reprocessFailed(): StatusResponse =
        http.post("/api/v1/documents/reprocess")

    /** WHY: Recover documents stuck in processing state. */
    fun recoverStuck(): StatusResponse =
        http.post("/api/v1/documents/recover-stuck")
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

    /** WHY: Get entity's neighborhood for graph traversal. */
    fun neighborhood(entityName: String, depth: Int = 1): EntityNeighborhoodResponse {
        val enc = URLEncoder.encode(entityName, StandardCharsets.UTF_8).replace("+", "%20")
        return http.get("/api/v1/graph/entities/$enc/neighborhood?depth=$depth")
    }
}

class RelationshipService(private val http: HttpHelper) {
    fun list(page: Int = 1, pageSize: Int = 20): RelationshipListResponse =
        http.get("/api/v1/graph/relationships?page=$page&page_size=$pageSize")
    
    /** WHY: Get specific relationship by ID. */
    fun get(id: String): RelationshipDetailResponse =
        http.get("/api/v1/graph/relationships/$id")
    
    /** WHY: Create new relationship between entities. */
    fun create(req: CreateRelationshipRequest): CreateRelationshipResponse =
        http.post("/api/v1/graph/relationships", req)
    
    /** WHY: Delete a relationship. */
    fun delete(id: String) { http.deleteRaw("/api/v1/graph/relationships/$id") }
}

class GraphService(private val http: HttpHelper) {
    fun get(): GraphResponse = http.get("/api/v1/graph")

    fun search(query: String): SearchNodesResponse =
        http.get("/api/v1/graph/nodes/search?q=$query")

    /** WHY: Search labels across graph. */
    fun labelSearch(query: String): LabelSearchResponse =
        http.get("/api/v1/graph/labels/search?q=$query")

    /** WHY: Get most popular labels. */
    fun popularLabels(limit: Int = 10): PopularLabelsResponse =
        http.get("/api/v1/graph/labels/popular?limit=$limit")

    /** WHY: Batch degree calculation for multiple nodes. */
    fun batchDegrees(nodeIds: List<String>): BatchDegreesResponse =
        http.post("/api/v1/graph/degrees/batch", mapOf("node_ids" to nodeIds))
}

class QueryService(private val http: HttpHelper) {
    fun execute(query: String, mode: String = "hybrid"): QueryResponse =
        http.post("/api/v1/query", QueryRequest(query, mode))
    
    /** WHY: Stream query results via SSE. Returns raw SSE text. */
    fun stream(query: String, mode: String = "hybrid"): String =
        http.postRaw("/api/v1/query/stream", QueryRequest(query, mode))
}

class ChatService(private val http: HttpHelper) {
    fun completions(req: ChatCompletionRequest): ChatCompletionResponse =
        http.post("/api/v1/chat/completions", req)
    
    /** WHY: Stream chat completions via SSE. Returns raw SSE text. */
    fun stream(req: ChatCompletionRequest): String {
        val streamingReq = req.copy(stream = true)
        return http.postRaw("/api/v1/chat/completions/stream", streamingReq)
    }
    
    /** WHY: Chat with conversation history. */
    fun completionsWithConversation(conversationId: String, message: String): ChatCompletionResponse {
        val req = ChatCompletionRequest(message = message, conversationId = conversationId)
        return http.post("/api/v1/chat/completions", req)
    }
}

class AuthService(private val http: HttpHelper) {
    fun login(username: String, password: String): TokenResponse =
        http.post("/api/v1/auth/login", LoginRequest(username, password))
    
    /** WHY: Logout invalidates current token. */
    fun logout() { http.postRaw("/api/v1/auth/logout", null) }
    
    /** WHY: Refresh token before expiration. */
    fun refresh(): TokenResponse = http.post("/api/v1/auth/refresh")
    
    /** WHY: Get current user info. */
    fun me(): AuthUserResponse = http.get("/api/v1/auth/me")
    
    /** WHY: Change password for current user. */
    fun changePassword(oldPassword: String, newPassword: String): StatusResponse =
        http.post("/api/v1/auth/change-password", mapOf("old_password" to oldPassword, "new_password" to newPassword))
}

class UserService(private val http: HttpHelper) {
    fun list(): UserListResponse = http.get("/api/v1/users")
    
    /** WHY: Get user by ID. */
    fun get(id: String): UserInfo = http.get("/api/v1/users/$id")
    
    /** WHY: Create new user (admin only). */
    fun create(username: String, email: String, password: String, role: String = "user"): UserInfo =
        http.post("/api/v1/users", mapOf("username" to username, "email" to email, "password" to password, "role" to role))
    
    /** WHY: Update user (admin only). */
    fun update(id: String, updates: Map<String, Any?>): UserInfo =
        http.put("/api/v1/users/$id", updates)
    
    /** WHY: Delete user (admin only). */
    fun delete(id: String) { http.deleteRaw("/api/v1/users/$id") }
}

class ApiKeyService(private val http: HttpHelper) {
    fun list(): ApiKeyListResponse = http.get("/api/v1/api-keys")
    
    /** WHY: Get API key by ID. */
    fun get(id: String): ApiKeyInfo = http.get("/api/v1/api-keys/$id")
    
    /** WHY: Create new API key. */
    fun create(name: String, expiresAt: String? = null): CreateApiKeyResponse =
        http.post("/api/v1/api-keys", mapOf("name" to name, "expires_at" to expiresAt))
    
    /** WHY: Revoke an API key. */
    fun revoke(id: String) { http.deleteRaw("/api/v1/api-keys/$id") }
    
    /** WHY: Rotate an API key (create new, revoke old). */
    fun rotate(id: String): CreateApiKeyResponse =
        http.post("/api/v1/api-keys/$id/rotate")
}

class TenantService(private val http: HttpHelper) {
    fun list(): TenantListResponse = http.get("/api/v1/tenants")
    
    /** WHY: Get tenant by ID. */
    fun get(id: String): TenantInfo = http.get("/api/v1/tenants/$id")
    
    /** WHY: Create new tenant. */
    fun create(name: String, slug: String): TenantInfo =
        http.post("/api/v1/tenants", mapOf("name" to name, "slug" to slug))
    
    /** WHY: Update tenant. */
    fun update(id: String, updates: Map<String, Any?>): TenantInfo =
        http.put("/api/v1/tenants/$id", updates)
    
    /** WHY: Delete tenant. */
    fun delete(id: String) { http.deleteRaw("/api/v1/tenants/$id") }
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
        http.post("/api/v1/conversations/bulk/delete", mapOf("conversation_ids" to ids))
    
    /** WHY: Update conversation title (PATCH). */
    fun update(id: String, title: String): ConversationInfo =
        http.patch("/api/v1/conversations/$id", mapOf("title" to title))

    /** WHY: Get messages for a conversation. */
    fun messages(id: String): MessageListResponse =
        http.get("/api/v1/conversations/$id/messages")

    /** WHY: Add message to conversation. */
    fun addMessage(id: String, role: String, content: String): Message =
        http.post("/api/v1/conversations/$id/messages", mapOf("role" to role, "content" to content))

    /** WHY: Delete message — DELETE /api/v1/messages/{message_id}. */
    fun deleteMessage(messageId: String) {
        http.deleteRaw("/api/v1/messages/$messageId")
    }

    /** WHY: Share conversation via link. */
    fun share(id: String): ShareLinkResponse =
        http.post("/api/v1/conversations/$id/share")

    /** WHY: Remove share link for a conversation. */
    fun unshare(id: String) {
        http.deleteRaw("/api/v1/conversations/$id/share")
    }
    
    /** WHY: Import conversations from external source. */
    fun import(data: ConversationImport): ImportResponse =
        http.post("/api/v1/conversations/import", data)
}

class FolderService(private val http: HttpHelper) {
    fun list(): List<FolderInfo> = http.get("/api/v1/folders")

    fun create(name: String): FolderInfo =
        http.post("/api/v1/folders", mapOf("name" to name))

    /** WHY: DELETE returns 204 No Content — use deleteRaw to avoid deserialization of empty body. */
    fun delete(id: String) { http.deleteRaw("/api/v1/folders/$id") }

    /** WHY: Update folder name (PATCH). */
    fun update(id: String, name: String): FolderInfo =
        http.patch("/api/v1/folders/$id", mapOf("name" to name))
}

class TaskService(private val http: HttpHelper) {
    fun list(): TaskListResponse = http.get("/api/v1/tasks")

    fun get(id: String): TaskInfo = http.get("/api/v1/tasks/$id")
    
    /** WHY: Create new background task. */
    fun create(taskType: String, params: Map<String, Any?> = emptyMap()): TaskInfo =
        http.post("/api/v1/tasks", mapOf("task_type" to taskType, "params" to params))
    
    /** WHY: Cancel running task. */
    fun cancel(id: String): StatusResponse =
        http.post("/api/v1/tasks/$id/cancel")
    
    /** WHY: Get task status only (lightweight). */
    fun status(id: String): TaskStatusResponse =
        http.get("/api/v1/tasks/$id/status")
    
    /** WHY: Retry failed task. */
    fun retry(id: String): TaskInfo =
        http.post("/api/v1/tasks/$id/retry")
}

class PipelineService(private val http: HttpHelper) {
    fun status(): PipelineStatus = http.get("/api/v1/pipeline/status")

    fun queueMetrics(): QueueMetrics = http.get("/api/v1/pipeline/queue-metrics")
    
    /** WHY: Get processing queue items. */
    fun processing(): ProcessingListResponse =
        http.get("/api/v1/pipeline/processing")
    
    /** WHY: Pause pipeline processing. */
    fun pause(): StatusResponse = http.post("/api/v1/pipeline/pause")
    
    /** WHY: Resume pipeline processing. */
    fun resume(): StatusResponse = http.post("/api/v1/pipeline/resume")
    
    /** WHY: Cancel current pipeline run. */
    fun cancel(): StatusResponse = http.post("/api/v1/pipeline/cancel")
    
    /** WHY: Estimate cost for document processing. */
    fun costEstimate(documentCount: Int, avgTokens: Int = 1000): CostEstimateResponse =
        http.post("/api/v1/pipeline/costs/estimate", mapOf("document_count" to documentCount, "avg_tokens" to avgTokens))
}

class ModelService(private val http: HttpHelper) {
    fun catalog(): ProviderCatalog = http.get("/api/v1/models")

    fun health(): List<ProviderHealthInfo> {
        val json = http.getRaw("/api/v1/models/health")
        return http.mapper.readValue(json, object : TypeReference<List<ProviderHealthInfo>>() {})
    }

    fun providerStatus(): ProviderStatus =
        http.get("/api/v1/settings/provider/status")
    
    /** WHY: List available models. */
    fun list(): ModelListResponse = http.get("/api/v1/models/list")
    
    /** WHY: Get specific model info. */
    fun get(modelId: String): ModelInfo = http.get("/api/v1/models/$modelId")
    
    /** WHY: List available providers. */
    fun providers(): ProviderListResponse = http.get("/api/v1/settings/providers")
    
    /** WHY: Set default model for provider. */
    fun setDefault(providerId: String, modelId: String): StatusResponse =
        http.put("/api/v1/models/default", mapOf("provider_id" to providerId, "model_id" to modelId))
    
    /** WHY: Test model connectivity. */
    fun test(modelId: String): ModelTestResponse =
        http.post("/api/v1/models/$modelId/test")
}

class WorkspaceService(private val http: HttpHelper) {
    /** WHY: GET /api/v1/tenants/{tenant_id}/workspaces — returns paginated wrapper. */
    fun list(tenantId: String): List<WorkspaceInfo> {
        val page: WorkspaceListResponse = http.get("/api/v1/tenants/$tenantId/workspaces")
        return page.items ?: emptyList()
    }

    /** WHY: POST /api/v1/tenants/{tenant_id}/workspaces */
    fun create(tenantId: String, name: String, slug: String? = null): WorkspaceInfo =
        http.post(
            "/api/v1/tenants/$tenantId/workspaces",
            mapOf("name" to name, "slug" to slug),
        )

    fun getBySlug(tenantId: String, slug: String): WorkspaceInfo =
        http.get("/api/v1/tenants/$tenantId/workspaces/by-slug/$slug")

    fun get(id: String): WorkspaceInfo = http.get("/api/v1/workspaces/$id")

    fun update(id: String, updates: Map<String, Any?>): WorkspaceInfo =
        http.put("/api/v1/workspaces/$id", updates)

    fun delete(id: String) {
        http.deleteRaw("/api/v1/workspaces/$id")
    }

    fun stats(id: String): WorkspaceStatsResponse =
        http.get("/api/v1/workspaces/$id/stats")

    fun rebuildEmbeddings(id: String): StatusResponse =
        http.post("/api/v1/workspaces/$id/rebuild-embeddings")

    fun putInjection(id: String, body: Map<String, Any?>): Map<String, Any?> =
        http.put("/api/v1/workspaces/$id/injection", body)

    fun listInjections(id: String): Map<String, Any?> =
        http.get("/api/v1/workspaces/$id/injections")
}

class PdfService(private val http: HttpHelper) {
    fun progress(trackId: String): PdfProgressResponse =
        http.get("/api/v1/documents/pdf/progress/$trackId")

    fun content(pdfId: String): PdfContentResponse =
        http.get("/api/v1/documents/pdf/$pdfId/content")
}

class CostService(private val http: HttpHelper) {
    fun summary(): CostSummary = http.get("/api/v1/costs/summary")
    
    /** WHY: Get daily cost breakdown. */
    fun daily(date: String? = null): DailyCostResponse {
        val path = if (date != null) "/api/v1/costs/daily?date=$date" else "/api/v1/costs/daily"
        return http.get(path)
    }
    
    /** WHY: Get costs grouped by provider. */
    fun byProvider(): ProviderCostResponse = http.get("/api/v1/costs/by-provider")
    
    /** WHY: Get costs grouped by model. */
    fun byModel(): ModelCostResponse = http.get("/api/v1/costs/by-model")
    
    /** WHY: Get cost history for date range. */
    fun history(startDate: String, endDate: String): CostHistoryResponse =
        http.get("/api/v1/costs/history?start_date=$startDate&end_date=$endDate")
    
    /** WHY: Export cost data. */
    fun export(format: String = "csv"): String =
        http.getRaw("/api/v1/costs/export?format=$format")
    
    /** WHY: Get current budget info. */
    fun budget(): BudgetInfo = http.get("/api/v1/costs/budget")
    
    /** WHY: Set cost budget. */
    fun setBudget(amount: Double, period: String = "monthly"): StatusResponse =
        http.post("/api/v1/costs/budget", mapOf("amount" to amount, "period" to period))
}

/** WHY: PATCH/GET under /api/v1/admin/ (quota, config defaults). */
class AdminService(private val http: HttpHelper) {
    fun patchTenantQuota(tenantId: String, maxWorkspaces: Int): UpdateTenantQuotaResponse =
        http.patch(
            "/api/v1/admin/tenants/$tenantId/quota",
            mapOf("max_workspaces" to maxWorkspaces),
        )

    fun getServerDefaults(): ServerDefaultsResponse =
        http.get("/api/v1/admin/config/defaults")

    fun patchServerDefaults(defaultMaxWorkspaces: Int): ServerDefaultsResponse =
        http.patch(
            "/api/v1/admin/config/defaults",
            mapOf("default_max_workspaces" to defaultMaxWorkspaces),
        )
}

/** WHY: GET /api/v1/config/effective */
class EffectiveConfigService(private val http: HttpHelper) {
    fun get(): Map<String, Any?> = http.get("/api/v1/config/effective")
}

/** WHY: Public shared conversation — GET /api/v1/shared/{share_id} only. */
class SharedService(private val http: HttpHelper) {
    fun get(shareId: String): ConversationDetail =
        http.get("/api/v1/shared/$shareId")
}
