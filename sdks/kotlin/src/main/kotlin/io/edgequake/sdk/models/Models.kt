@file:Suppress("unused")
package io.edgequake.sdk.models

import com.fasterxml.jackson.annotation.JsonProperty

// ── Health ──────────────────────────────────────────────────────────

data class HealthResponse(
    val status: String? = null,
    val version: String? = null,
    @JsonProperty("storage_mode") val storageMode: String? = null,
    @JsonProperty("workspace_id") val workspaceId: String? = null,
    val components: Map<String, Any?>? = null,
    @JsonProperty("llm_provider_name") val llmProviderName: String? = null
)

// ── Documents ───────────────────────────────────────────────────────

data class Document(
    val id: String? = null,
    val title: String? = null,
    val status: String? = null,
    @JsonProperty("file_type") val fileType: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null,
    @JsonProperty("updated_at") val updatedAt: String? = null,
    @JsonProperty("file_size") val fileSize: Long? = null,
    @JsonProperty("chunk_count") val chunkCount: Int? = null
)

data class UploadResponse(
    @JsonProperty("document_id") val documentId: String? = null,
    val status: String? = null,
    val message: String? = null,
    @JsonProperty("track_id") val trackId: String? = null
)

data class ListDocumentsResponse(
    val documents: List<Document>? = null,
    val items: List<Document>? = null,
    val total: Int? = null,
    val page: Int? = null,
    @JsonProperty("page_size") val pageSize: Int? = null,
    @JsonProperty("total_pages") val totalPages: Int? = null,
    @JsonProperty("has_more") val hasMore: Boolean? = null
)

data class TextUploadRequest(
    val title: String,
    val content: String,
    @JsonProperty("file_type") val fileType: String = "txt"
)

data class ScanRequest(
    val path: String,
    val recursive: Boolean = true
)

data class ScanResponse(
    val status: String? = null,
    val message: String? = null,
    @JsonProperty("files_found") val filesFound: Int? = null
)

// ── Entities ────────────────────────────────────────────────────────

data class Entity(
    val id: String? = null,
    @JsonProperty("entity_name") val entityName: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    val description: String? = null,
    @JsonProperty("source_id") val sourceId: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null,
    @JsonProperty("updated_at") val updatedAt: String? = null,
    val degree: Int? = null,
    val metadata: Map<String, Any?>? = null
)

data class CreateEntityRequest(
    @JsonProperty("entity_name") val entityName: String,
    @JsonProperty("entity_type") val entityType: String,
    val description: String,
    @JsonProperty("source_id") val sourceId: String
)

data class CreateEntityResponse(
    val status: String? = null,
    val message: String? = null,
    val entity: Entity? = null
)

data class EntityDetailResponse(
    val entity: Entity? = null,
    val relationships: Map<String, Any?>? = null,
    val statistics: Map<String, Any?>? = null
)

data class EntityListResponse(
    val items: List<Entity>? = null,
    val total: Int? = null,
    val page: Int? = null,
    @JsonProperty("page_size") val pageSize: Int? = null,
    @JsonProperty("total_pages") val totalPages: Int? = null
)

data class EntityExistsResponse(
    @JsonProperty("entity_id") val entityId: String? = null,
    val exists: Boolean? = null
)

data class EntityDeleteResponse(
    val status: String? = null,
    val message: String? = null,
    @JsonProperty("deleted_entity_id") val deletedEntityId: String? = null,
    @JsonProperty("deleted_relationships") val deletedRelationships: Int? = null,
    @JsonProperty("affected_entities") val affectedEntities: List<String>? = null
)

data class MergeEntitiesRequest(
    @JsonProperty("source_entity") val sourceEntity: String,
    @JsonProperty("target_entity") val targetEntity: String
)

// ── Relationships ───────────────────────────────────────────────────

data class Relationship(
    val id: String? = null,
    val source: String? = null,
    val target: String? = null,
    @JsonProperty("relationship_type") val relationshipType: String? = null,
    val weight: Double? = null,
    val description: String? = null,
    @JsonProperty("source_id") val sourceId: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)

data class RelationshipListResponse(
    val items: List<Relationship>? = null,
    val total: Int? = null,
    val page: Int? = null,
    @JsonProperty("page_size") val pageSize: Int? = null,
    @JsonProperty("total_pages") val totalPages: Int? = null
)

// ── Graph ───────────────────────────────────────────────────────────

data class GraphNode(
    val id: String? = null,
    val label: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    val properties: Map<String, Any?>? = null
)

data class GraphEdge(
    val source: String? = null,
    val target: String? = null,
    val label: String? = null,
    val weight: Double? = null
)

data class GraphResponse(
    val nodes: List<GraphNode>? = null,
    val edges: List<GraphEdge>? = null
)

data class SearchNodesResponse(
    val nodes: List<GraphNode>? = null,
    val total: Int? = null
)

// ── Query & Chat ────────────────────────────────────────────────────

data class QueryRequest(
    val query: String,
    val mode: String = "hybrid"
)

data class QueryResponse(
    val answer: String? = null,
    val sources: List<Map<String, Any?>>? = null,
    val mode: String? = null
)

// WHY: EdgeQuake uses `message` (singular string), NOT `messages` (array).
// This is EdgeQuake's native RAG-aware chat format.

data class ChatMessage(
    val role: String,
    val content: String
)

data class ChatCompletionRequest(
    val message: String,
    val stream: Boolean = false,
    val mode: String? = null,
    @JsonProperty("conversation_id") val conversationId: String? = null,
    @JsonProperty("max_tokens") val maxTokens: Int? = null,
    val temperature: Double? = null,
    @JsonProperty("top_k") val topK: Int? = null,
    @JsonProperty("parent_id") val parentId: String? = null,
    val provider: String? = null,
    val model: String? = null
)

data class ChatSourceReference(
    @JsonProperty("source_type") val sourceType: String? = null,
    val id: String? = null,
    val score: Double? = null,
    val snippet: String? = null
)

data class ChatCompletionResponse(
    @JsonProperty("conversation_id") val conversationId: String? = null,
    @JsonProperty("user_message_id") val userMessageId: String? = null,
    @JsonProperty("assistant_message_id") val assistantMessageId: String? = null,
    val content: String? = null,
    val mode: String? = null,
    val sources: List<ChatSourceReference>? = null
)

// ── Auth & Multi-tenant ─────────────────────────────────────────────

data class LoginRequest(val username: String, val password: String)
data class TokenResponse(val token: String? = null, @JsonProperty("expires_at") val expiresAt: String? = null)

data class UserInfo(
    val id: String? = null,
    val username: String? = null,
    val email: String? = null,
    val role: String? = null
)
data class UserListResponse(val users: List<UserInfo>? = null)

data class ApiKeyInfo(
    val id: String? = null,
    val name: String? = null,
    val prefix: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)
data class ApiKeyListResponse(val keys: List<ApiKeyInfo>? = null)

data class TenantInfo(
    val id: String? = null,
    val name: String? = null,
    val slug: String? = null
)
data class TenantListResponse(val items: List<TenantInfo>? = null)

// ── Conversations ───────────────────────────────────────────────────

data class ConversationInfo(
    val id: String? = null,
    val title: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null,
    @JsonProperty("updated_at") val updatedAt: String? = null,
    @JsonProperty("message_count") val messageCount: Int? = null
)

// WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array.
data class ConversationListResponse(
    val items: List<ConversationInfo>? = null
)

/** WHY: GET /conversations/{id} returns {"conversation":{...},"messages":[...]} wrapper. */
data class ConversationDetail(
    val conversation: ConversationInfo? = null,
    val messages: List<Message>? = null
)

data class Message(
    val id: String? = null,
    val role: String? = null,
    val content: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)

data class BulkDeleteResponse(
    val deleted: Int? = null,
    val status: String? = null
)

// ── Folders ─────────────────────────────────────────────────────────

data class FolderInfo(
    val id: String? = null,
    val name: String? = null,
    @JsonProperty("parent_id") val parentId: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)

// ── Workspaces ──────────────────────────────────────────────────────

data class WorkspaceInfo(
    val id: String? = null,
    val name: String? = null,
    val slug: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)

// ── Operations ──────────────────────────────────────────────────────

data class PipelineStatus(
    @JsonProperty("is_busy") val isBusy: Boolean? = null,
    @JsonProperty("total_documents") val totalDocuments: Int? = null,
    @JsonProperty("processed_documents") val processedDocuments: Int? = null,
    @JsonProperty("pending_tasks") val pendingTasks: Int? = null,
    @JsonProperty("processing_tasks") val processingTasks: Int? = null,
    @JsonProperty("completed_tasks") val completedTasks: Int? = null,
    @JsonProperty("failed_tasks") val failedTasks: Int? = null,
    @JsonProperty("cancellation_requested") val cancellationRequested: Boolean? = null
)

data class QueueMetrics(
    @JsonProperty("pending_count") val pendingCount: Int? = null,
    @JsonProperty("processing_count") val processingCount: Int? = null,
    @JsonProperty("active_workers") val activeWorkers: Int? = null,
    @JsonProperty("max_workers") val maxWorkers: Int? = null,
    @JsonProperty("worker_utilization") val workerUtilization: Int? = null,
    @JsonProperty("avg_wait_time_seconds") val avgWaitTimeSeconds: Double? = null,
    @JsonProperty("throughput_per_minute") val throughputPerMinute: Double? = null,
    @JsonProperty("rate_limited") val rateLimited: Boolean? = null
)

data class TaskInfo(
    val id: String? = null,
    val status: String? = null,
    @JsonProperty("task_type") val taskType: String? = null,
    val progress: Any? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)

data class TaskListResponse(
    val tasks: List<TaskInfo>? = null,
    val items: List<TaskInfo>? = null,
    val total: Int? = null
)

// ── Models / Providers ──────────────────────────────────────────────

data class ProviderCatalog(
    val providers: List<ProviderInfo>? = null
)

data class ProviderInfo(
    val name: String? = null,
    @JsonProperty("display_name") val displayName: String? = null,
    val models: List<Map<String, Any?>>? = null
)

data class ProviderHealthInfo(
    val name: String? = null,
    @JsonProperty("display_name") val displayName: String? = null,
    @JsonProperty("provider_type") val providerType: String? = null,
    val enabled: Boolean? = null,
    val priority: Int? = null,
    val models: List<Map<String, Any?>>? = null
)

data class ProviderStatus(
    val provider: Map<String, Any?>? = null,
    val embedding: Map<String, Any?>? = null,
    val storage: Map<String, Any?>? = null,
    val metadata: Map<String, Any?>? = null
)

// ── Costs ───────────────────────────────────────────────────────────

data class CostSummary(
    @JsonProperty("total_cost") val totalCost: Double? = null,
    @JsonProperty("document_count") val documentCount: Int? = null,
    @JsonProperty("query_count") val queryCount: Int? = null,
    val entries: List<Map<String, Any?>>? = null
)

// ── PDF ─────────────────────────────────────────────────────────────

data class PdfProgressResponse(
    @JsonProperty("track_id") val trackId: String? = null,
    val status: String? = null,
    val progress: Any? = null
)

data class PdfContentResponse(
    val content: String? = null,
    @JsonProperty("page_count") val pageCount: Int? = null
)
