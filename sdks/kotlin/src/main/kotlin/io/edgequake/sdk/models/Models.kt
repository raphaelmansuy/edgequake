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

/** WHY: /ready checks database/provider health. */
data class ReadinessResponse(
    val ready: Boolean? = null,
    val checks: Map<String, Any?>? = null
)

/** WHY: /live is lightweight server check. */
data class LivenessResponse(
    val alive: Boolean? = null,
    val uptime: Long? = null
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

/** WHY: Document chunks contain extracted entities. */
data class DocumentChunksResponse(
    @JsonProperty("document_id") val documentId: String? = null,
    val chunks: List<ChunkInfo>? = null,
    val total: Int? = null
)

data class ChunkInfo(
    val id: String? = null,
    val content: String? = null,
    val index: Int? = null,
    @JsonProperty("token_count") val tokenCount: Int? = null,
    val entities: List<String>? = null
)

/** WHY: Document status for tracking processing progress. */
data class DocumentStatusResponse(
    @JsonProperty("document_id") val documentId: String? = null,
    val status: String? = null,
    val progress: Double? = null,
    val error: String? = null
)

/** WHY: GET /api/v1/documents/track/{track_id} */
data class TrackStatusResponse(
    @JsonProperty("track_id") val trackId: String? = null,
    val status: String? = null,
    val progress: Double? = null,
    val message: String? = null,
    @JsonProperty("document_id") val documentId: String? = null,
)

/** WHY: Generic status response for operations. */
data class StatusResponse(
    val status: String? = null,
    val message: String? = null
)

// ── Admin / effective config ───────────────────────────────────────

data class UpdateTenantQuotaResponse(
    @JsonProperty("tenant_id") val tenantId: String? = null,
    @JsonProperty("max_workspaces") val maxWorkspaces: Int? = null,
    @JsonProperty("previous_max_workspaces") val previousMaxWorkspaces: Int? = null,
    @JsonProperty("current_workspace_count") val currentWorkspaceCount: Int? = null,
)

data class ServerDefaultsResponse(
    @JsonProperty("default_max_workspaces") val defaultMaxWorkspaces: Int? = null,
    val note: String? = null,
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
    @JsonProperty("source_name") val sourceName: String,
    @JsonProperty("target_name") val targetName: String
)

/** WHY: `GET /graph/entities/{name}/neighborhood` — nodes + edges subgraph. */
data class EntityNeighborhoodResponse(
    val nodes: List<NeighborhoodNode>? = null,
    val edges: List<NeighborhoodEdge>? = null
)

data class NeighborhoodNode(
    val id: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    val description: String? = null,
    val degree: Int? = null
)

data class NeighborhoodEdge(
    val id: String? = null,
    val source: String? = null,
    val target: String? = null,
    @JsonProperty("relation_type") val relationType: String? = null,
    val weight: Double? = null
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

/** WHY: Relationship detail response. */
data class RelationshipDetailResponse(
    val relationship: Relationship? = null,
    val source: Entity? = null,
    val target: Entity? = null
)

/** WHY: Request to create relationship. */
data class CreateRelationshipRequest(
    val source: String,
    val target: String,
    @JsonProperty("relationship_type") val relationshipType: String,
    val weight: Double = 1.0,
    val description: String? = null,
    @JsonProperty("source_id") val sourceId: String? = null
)

/** WHY: Response after creating relationship. */
data class CreateRelationshipResponse(
    val status: String? = null,
    val relationship: Relationship? = null
)

/** WHY: List of relationship types. */
data class RelationshipTypesResponse(
    val types: List<String>? = null,
    val total: Int? = null
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

/** WHY: Graph statistics for monitoring. */
data class GraphStatsResponse(
    @JsonProperty("node_count") val nodeCount: Int? = null,
    @JsonProperty("edge_count") val edgeCount: Int? = null,
    @JsonProperty("entity_count") val entityCount: Int? = null,
    @JsonProperty("relationship_count") val relationshipCount: Int? = null
)

/** WHY: `GET /graph/labels/search` returns string label hits. */
data class LabelSearchResponse(
    val labels: List<String>? = null
)

/** WHY: Popular labels include degree + entity metadata from API. */
data class PopularLabelsResponse(
    val labels: List<PopularLabelItem>? = null,
    @JsonProperty("total_entities") val totalEntities: Int? = null
)

data class PopularLabelItem(
    val label: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    val degree: Int? = null,
    val description: String? = null
)

/** WHY: `POST /graph/degrees/batch` returns ordered degree rows. */
data class BatchDegreesResponse(
    val degrees: List<NodeDegree>? = null,
    val count: Int? = null
)

data class NodeDegree(
    @JsonProperty("node_id") val nodeId: String? = null,
    val degree: Int? = null
)

// ── Query & Chat ────────────────────────────────────────────────────

data class QueryRequest(
    val query: String,
    val mode: String = "hybrid",
    @JsonProperty("system_prompt") val systemPrompt: String? = null
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
    val model: String? = null,
    @JsonProperty("system_prompt") val systemPrompt: String? = null
)

data class ChatSourceReference(
    @JsonProperty("source_type") val sourceType: String? = null,
    val id: String? = null,
    val score: Double? = null,
    val snippet: String? = null,
    @JsonProperty("document_id") val documentId: String? = null,
    @JsonProperty("file_path") val filePath: String? = null
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

/** WHY: Auth user info response. */
data class AuthUserResponse(
    val id: String? = null,
    val username: String? = null,
    val email: String? = null,
    val role: String? = null,
    val permissions: List<String>? = null
)

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

/** WHY: Response when creating new API key with secret. */
data class CreateApiKeyResponse(
    val id: String? = null,
    val name: String? = null,
    val key: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null,
    @JsonProperty("expires_at") val expiresAt: String? = null
)

data class TenantInfo(
    val id: String? = null,
    val name: String? = null,
    val slug: String? = null,
    val plan: String? = null,
    @JsonProperty("is_active") val isActive: Boolean? = null,
    @JsonProperty("max_workspaces") val maxWorkspaces: Int? = null,
    // Default LLM configuration for new workspaces.
    @JsonProperty("default_llm_model") val defaultLlmModel: String? = null,
    @JsonProperty("default_llm_provider") val defaultLlmProvider: String? = null,
    @JsonProperty("default_llm_full_id") val defaultLlmFullId: String? = null,
    // Default embedding configuration for new workspaces.
    @JsonProperty("default_embedding_model") val defaultEmbeddingModel: String? = null,
    @JsonProperty("default_embedding_provider") val defaultEmbeddingProvider: String? = null,
    @JsonProperty("default_embedding_dimension") val defaultEmbeddingDimension: Int? = null,
    @JsonProperty("default_embedding_full_id") val defaultEmbeddingFullId: String? = null,
    // Default vision LLM for PDF image extraction (SPEC-041).
    @JsonProperty("default_vision_llm_model") val defaultVisionLlmModel: String? = null,
    @JsonProperty("default_vision_llm_provider") val defaultVisionLlmProvider: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null,
    @JsonProperty("updated_at") val updatedAt: String? = null
)

data class CreateTenantRequest(
    val name: String,
    val slug: String? = null,
    val description: String? = null,
    val plan: String? = null,
    // Default LLM configuration.
    @JsonProperty("default_llm_model") val defaultLlmModel: String? = null,
    @JsonProperty("default_llm_provider") val defaultLlmProvider: String? = null,
    // Default embedding configuration.
    @JsonProperty("default_embedding_model") val defaultEmbeddingModel: String? = null,
    @JsonProperty("default_embedding_provider") val defaultEmbeddingProvider: String? = null,
    @JsonProperty("default_embedding_dimension") val defaultEmbeddingDimension: Int? = null,
    // Default vision LLM (SPEC-041).
    @JsonProperty("default_vision_llm_model") val defaultVisionLlmModel: String? = null,
    @JsonProperty("default_vision_llm_provider") val defaultVisionLlmProvider: String? = null
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
    @JsonProperty("affected") val affected: Int? = null,
    /** Legacy field from older mocks. */
    val deleted: Int? = null,
    val status: String? = null
)

/** WHY: Pagination meta from list messages API. */
data class MessagePaginationMeta(
    @JsonProperty("next_cursor") val nextCursor: String? = null,
    @JsonProperty("prev_cursor") val prevCursor: String? = null,
    val total: Int? = null,
    @JsonProperty("has_more") val hasMore: Boolean? = null
)

/** WHY: `GET /conversations/{id}/messages` returns items + pagination. */
data class MessageListResponse(
    val items: List<Message>? = null,
    val pagination: MessagePaginationMeta? = null
)

/** WHY: `POST /conversations/{id}/share` returns share id + relative URL. */
data class ShareLinkResponse(
    @JsonProperty("share_id") val shareId: String? = null,
    @JsonProperty("share_url") val shareUrl: String? = null
)

/** WHY: Import conversations from external source. */
data class ConversationImport(
    val conversations: List<Map<String, Any?>>? = null,
    val format: String = "json"
)

/** WHY: Import result. */
data class ImportResponse(
    val imported: Int? = null,
    val failed: Int? = null,
    val errors: List<String>? = null
)

// ── Folders ─────────────────────────────────────────────────────────

data class FolderInfo(
    val id: String? = null,
    val name: String? = null,
    @JsonProperty("parent_id") val parentId: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)

// ── Workspaces ──────────────────────────────────────────────────────

data class WorkspaceListResponse(
    val items: List<WorkspaceInfo>? = null,
    val total: Int? = null,
    val offset: Int? = null,
    val limit: Int? = null,
)

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

/** WHY: Lightweight task status check. */
data class TaskStatusResponse(
    val status: String? = null,
    val progress: Double? = null,
    @JsonProperty("completed_at") val completedAt: String? = null
)

/** WHY: List of items in processing queue. */
data class ProcessingListResponse(
    val items: List<ProcessingItem>? = null,
    val total: Int? = null
)

data class ProcessingItem(
    val id: String? = null,
    val status: String? = null,
    @JsonProperty("document_id") val documentId: String? = null,
    val progress: Double? = null,
    @JsonProperty("started_at") val startedAt: String? = null
)

/** WHY: Cost estimate for processing. */
data class CostEstimateResponse(
    @JsonProperty("estimated_cost") val estimatedCost: Double? = null,
    @JsonProperty("token_count") val tokenCount: Int? = null,
    @JsonProperty("model_used") val modelUsed: String? = null
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

/** WHY: List of available models. */
data class ModelListResponse(
    val models: List<ModelInfo>? = null,
    val total: Int? = null
)

/** WHY: Model information. */
data class ModelInfo(
    val id: String? = null,
    val name: String? = null,
    val provider: String? = null,
    @JsonProperty("context_length") val contextLength: Int? = null,
    @JsonProperty("cost_per_token") val costPerToken: Double? = null
)

/** WHY: List of providers. */
data class ProviderListResponse(
    val providers: List<ProviderSummary>? = null
)

data class ProviderSummary(
    val id: String? = null,
    val name: String? = null,
    val enabled: Boolean? = null
)

/** WHY: Model test result. */
data class ModelTestResponse(
    val success: Boolean? = null,
    @JsonProperty("response_time_ms") val responseTimeMs: Long? = null,
    val error: String? = null
)

// ── Costs ───────────────────────────────────────────────────────────

data class CostSummary(
    @JsonProperty("total_cost") val totalCost: Double? = null,
    @JsonProperty("document_count") val documentCount: Int? = null,
    @JsonProperty("query_count") val queryCount: Int? = null,
    val entries: List<Map<String, Any?>>? = null
)

/** WHY: Daily cost breakdown. */
data class DailyCostResponse(
    val date: String? = null,
    val cost: Double? = null,
    val breakdown: Map<String, Double>? = null
)

/** WHY: Costs by provider. */
data class ProviderCostResponse(
    val providers: Map<String, Double>? = null,
    val total: Double? = null
)

/** WHY: Costs by model. */
data class ModelCostResponse(
    val models: Map<String, Double>? = null,
    val total: Double? = null
)

/** WHY: Cost history for date range. */
data class CostHistoryResponse(
    val history: List<DailyCostEntry>? = null,
    val total: Double? = null
)

data class DailyCostEntry(
    val date: String? = null,
    val cost: Double? = null
)

/** WHY: Budget configuration. */
data class BudgetInfo(
    val amount: Double? = null,
    val period: String? = null,
    val used: Double? = null,
    val remaining: Double? = null
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

// ── Workspace Extended ──────────────────────────────────────────────

/** WHY: Workspace statistics. */
data class WorkspaceStatsResponse(
    @JsonProperty("workspace_id") val workspaceId: String? = null,
    @JsonProperty("document_count") val documentCount: Int? = null,
    @JsonProperty("entity_count") val entityCount: Int? = null,
    @JsonProperty("relationship_count") val relationshipCount: Int? = null,
    @JsonProperty("storage_bytes") val storageBytes: Long? = null
)

