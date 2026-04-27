using System.Text.Json;
using System.Text.Json.Serialization;

namespace EdgeQuakeSDK;

// OODA-36: Enhanced with complete API response types.

// ── Health ──
public class HealthResponse
{
    public string? Status { get; set; }
    public string? Version { get; set; }
    public string? StorageMode { get; set; }
    public string? WorkspaceId { get; set; }
    public Dictionary<string, bool>? Components { get; set; }
    public string? LlmProviderName { get; set; }
}

public class ReadinessResponse
{
    public string? Status { get; set; }
    public bool? Ready { get; set; }
    public Dictionary<string, bool>? Checks { get; set; }
}

public class LivenessResponse
{
    public string? Status { get; set; }
    public bool? Alive { get; set; }
}

public class MetricsResponse
{
    public Dictionary<string, object>? Metrics { get; set; }
}

// ── Documents ──
public class DocumentListResponse
{
    public List<JsonElement>? Documents { get; set; }
    public List<JsonElement>? Items { get; set; }
    public int? Total { get; set; }
    public int? Page { get; set; }
    public int? PageSize { get; set; }
    public int? TotalPages { get; set; }
    public bool? HasMore { get; set; }
}

public class DocumentDetailResponse
{
    public string? Id { get; set; }
    public string? Title { get; set; }
    public string? Status { get; set; }
    public string? CreatedAt { get; set; }
    public int? ChunkCount { get; set; }
    public int? EntityCount { get; set; }
    public Dictionary<string, object>? Metadata { get; set; }
}

public class DocumentChunksResponse
{
    public List<JsonElement>? Chunks { get; set; }
    public int? Total { get; set; }
}

public class DocumentStatusResponse
{
    public string? Status { get; set; }
    public int? Progress { get; set; }
    public string? Stage { get; set; }
    public string? Message { get; set; }
}

/// <summary>GET /api/v1/documents/track/{track_id}</summary>
public class TrackStatusResponse
{
    public string? TrackId { get; set; }
    public string? Status { get; set; }
    public double? Progress { get; set; }
    public string? Message { get; set; }
    public string? DocumentId { get; set; }
}

public class UploadResponse
{
    public string? DocumentId { get; set; }
    public string? Status { get; set; }
    public string? TrackId { get; set; }
    public string? DuplicateOf { get; set; }
}

// ── Admin / config ──
public class UpdateTenantQuotaResponse
{
    public string? TenantId { get; set; }
    public int MaxWorkspaces { get; set; }
    public int PreviousMaxWorkspaces { get; set; }
    public int CurrentWorkspaceCount { get; set; }
}

public class ServerDefaultsResponse
{
    public int DefaultMaxWorkspaces { get; set; }
    public string? Note { get; set; }
}

// ── Entities ──
public class EntityListResponse
{
    public List<JsonElement>? Items { get; set; }
    public int? Total { get; set; }
    public int? Page { get; set; }
    public int? PageSize { get; set; }
    public int? TotalPages { get; set; }
}

public class EntityDetailResponse
{
    public JsonElement? Entity { get; set; }
    public JsonElement? Relationships { get; set; }
    public JsonElement? Statistics { get; set; }
}

public class EntityNeighborhoodResponse
{
    public JsonElement? Entity { get; set; }
    public List<JsonElement>? Neighbors { get; set; }
    public List<JsonElement>? Relationships { get; set; }
    public int? Depth { get; set; }
}

public class CreateEntityResponse
{
    public string? Status { get; set; }
    public string? Message { get; set; }
    public JsonElement? Entity { get; set; }
}

public class MergeEntitiesResponse
{
    public string? Status { get; set; }
    public string? Message { get; set; }
    public string? MergedEntityId { get; set; }
    public int? MergedRelationships { get; set; }
}

public class EntityExistsResponse
{
    public bool? Exists { get; set; }
    public string? EntityId { get; set; }
    public string? EntityType { get; set; }
    public int? Degree { get; set; }
}

public class EntityDeleteResponse
{
    public string? Status { get; set; }
    public string? Message { get; set; }
    public string? DeletedEntityId { get; set; }
    public int? DeletedRelationships { get; set; }
    public List<string>? AffectedEntities { get; set; }
}

// ── Relationships ──
public class RelationshipListResponse
{
    public List<JsonElement>? Items { get; set; }
    public int? Total { get; set; }
}

public class RelationshipDetailResponse
{
    public string? Source { get; set; }
    public string? Target { get; set; }
    public List<string>? Keywords { get; set; }
    public string? Description { get; set; }
    public double? Weight { get; set; }
}

public class CreateRelationshipResponse
{
    public string? Status { get; set; }
    public string? Message { get; set; }
}

// ── Graph ──
public class GraphResponse
{
    public List<JsonElement>? Nodes { get; set; }
    public List<JsonElement>? Edges { get; set; }
}

public class SearchResponse
{
    public List<JsonElement>? Results { get; set; }
}

public class LabelSearchResponse
{
    /// <summary>Canonical API: GET /graph/labels/search returns string label matches.</summary>
    public List<string>? Labels { get; set; }
}

public class PopularLabelsResponse
{
    public List<LabelInfo>? Labels { get; set; }
}

public class LabelInfo
{
    public string? Label { get; set; }
    public int? Count { get; set; }
}

public class BatchDegreesResponse
{
    public Dictionary<string, int>? Degrees { get; set; }
}

// ── Query ──
public class QueryResponse
{
    public string? Answer { get; set; }
    public List<JsonElement>? Sources { get; set; }
    public string? Mode { get; set; }
}

// ── Chat ──
public class ChatCompletionResponse
{
    public string? ConversationId { get; set; }
    public string? UserMessageId { get; set; }
    public string? AssistantMessageId { get; set; }
    public string? Content { get; set; }
    public string? Mode { get; set; }
    public List<JsonElement>? Sources { get; set; }
    public int? TokensUsed { get; set; }
    public long? DurationMs { get; set; }
}

// ── Auth ──
public class TenantListResponse
{
    public List<JsonElement>? Items { get; set; }
}

public class TenantInfo
{
    public string? Id { get; set; }
    public string? Name { get; set; }
    public string? Slug { get; set; }
    public string? Plan { get; set; }
    [JsonPropertyName("is_active")]
    public bool? IsActive { get; set; }
    [JsonPropertyName("max_workspaces")]
    public int? MaxWorkspaces { get; set; }
    // Default LLM configuration for new workspaces.
    [JsonPropertyName("default_llm_model")]
    public string? DefaultLlmModel { get; set; }
    [JsonPropertyName("default_llm_provider")]
    public string? DefaultLlmProvider { get; set; }
    [JsonPropertyName("default_llm_full_id")]
    public string? DefaultLlmFullId { get; set; }
    // Default embedding configuration for new workspaces.
    [JsonPropertyName("default_embedding_model")]
    public string? DefaultEmbeddingModel { get; set; }
    [JsonPropertyName("default_embedding_provider")]
    public string? DefaultEmbeddingProvider { get; set; }
    [JsonPropertyName("default_embedding_dimension")]
    public int? DefaultEmbeddingDimension { get; set; }
    [JsonPropertyName("default_embedding_full_id")]
    public string? DefaultEmbeddingFullId { get; set; }
    // Default vision LLM for PDF image extraction (SPEC-041).
    [JsonPropertyName("default_vision_llm_model")]
    public string? DefaultVisionLlmModel { get; set; }
    [JsonPropertyName("default_vision_llm_provider")]
    public string? DefaultVisionLlmProvider { get; set; }
    [JsonPropertyName("created_at")]
    public string? CreatedAt { get; set; }
    [JsonPropertyName("updated_at")]
    public string? UpdatedAt { get; set; }
}

public class UserListResponse
{
    public List<JsonElement>? Users { get; set; }
}

public class UserInfo
{
    public string? Id { get; set; }
    public string? Email { get; set; }
    public string? Name { get; set; }
    public string? Role { get; set; }
    public string? CreatedAt { get; set; }
}

public class ApiKeyListResponse
{
    public List<JsonElement>? Keys { get; set; }
}

public class ApiKeyInfo
{
    public string? Id { get; set; }
    public string? Name { get; set; }
    public string? Prefix { get; set; }
    public List<string>? Scopes { get; set; }
    public string? CreatedAt { get; set; }
    public string? ExpiresAt { get; set; }
}

public class CreateApiKeyResponse
{
    public string? Id { get; set; }
    public string? Key { get; set; }
    public string? Name { get; set; }
    public List<string>? Scopes { get; set; }
}

// ── Tasks ──
public class TaskListResponse
{
    public List<JsonElement>? Tasks { get; set; }
    public List<JsonElement>? Items { get; set; }
}

public class TaskInfo
{
    public string? Id { get; set; }
    public string? Type { get; set; }
    public string? Status { get; set; }
    public string? DocumentId { get; set; }
    public int? Progress { get; set; }
    public string? CreatedAt { get; set; }
    public string? CompletedAt { get; set; }
}

public class TaskStatusResponse
{
    public string? Status { get; set; }
    public int? Progress { get; set; }
    public string? Stage { get; set; }
    public string? Message { get; set; }
}

public class StatusResponse
{
    public string? Status { get; set; }
    public string? Message { get; set; }
}

// ── Pipeline ──
public class PipelineStatusResponse
{
    public bool? IsBusy { get; set; }
    public int? TotalDocuments { get; set; }
    public int? ProcessedDocuments { get; set; }
    public int? PendingTasks { get; set; }
    public int? ProcessingTasks { get; set; }
    public int? CompletedTasks { get; set; }
    public int? FailedTasks { get; set; }
}

public class QueueMetricsResponse
{
    public int? PendingCount { get; set; }
    public int? ProcessingCount { get; set; }
    public int? ActiveWorkers { get; set; }
    public int? MaxWorkers { get; set; }
    public double? WorkerUtilization { get; set; }
    public double? AvgWaitTimeSeconds { get; set; }
    public double? ThroughputPerMinute { get; set; }
    public bool? RateLimited { get; set; }
}

public class ProcessingListResponse
{
    public List<JsonElement>? Items { get; set; }
    public int? Total { get; set; }
}

public class CostEstimateResponse
{
    public double? EstimatedCost { get; set; }
    public int? EstimatedTokens { get; set; }
    public string? Model { get; set; }
}

// ── Models ──
public class ProviderCatalog
{
    public List<JsonElement>? Providers { get; set; }
}

public class ModelListResponse
{
    public List<ModelInfo>? Models { get; set; }
}

public class ModelInfo
{
    public string? Id { get; set; }
    public string? Name { get; set; }
    public string? Provider { get; set; }
    public string? Type { get; set; }
    public bool? IsDefault { get; set; }
    public int? ContextLength { get; set; }
}

public class ModelTestResponse
{
    public bool? Success { get; set; }
    public string? Response { get; set; }
    public int? TokensUsed { get; set; }
    public long? DurationMs { get; set; }
    public string? Error { get; set; }
}

public class ProviderHealthInfo
{
    public string? Name { get; set; }
    public string? DisplayName { get; set; }
    public string? ProviderType { get; set; }
    public bool? Enabled { get; set; }
    public int? Priority { get; set; }
    public List<JsonElement>? Models { get; set; }
}

public class ProviderStatus
{
    public JsonElement? Provider { get; set; }
    public JsonElement? Embedding { get; set; }
    public JsonElement? Storage { get; set; }
    public JsonElement? Metadata { get; set; }
}

public class ProviderListResponse
{
    public List<ProviderHealthInfo>? Providers { get; set; }
}

// ── Costs ──
public class CostSummary
{
    public double? TotalCost { get; set; }
    public int? DocumentCount { get; set; }
    public int? QueryCount { get; set; }
    public List<JsonElement>? Entries { get; set; }
}

public class DailyCostResponse
{
    public List<DailyCostEntry>? Days { get; set; }
    public double? TotalCost { get; set; }
}

public class DailyCostEntry
{
    public string? Date { get; set; }
    public double? Cost { get; set; }
    public int? Queries { get; set; }
    public int? Documents { get; set; }
}

public class ProviderCostResponse
{
    public List<ProviderCostEntry>? Providers { get; set; }
}

public class ProviderCostEntry
{
    public string? Provider { get; set; }
    public double? Cost { get; set; }
    public int? Requests { get; set; }
}

public class ModelCostResponse
{
    public List<ModelCostEntry>? Models { get; set; }
}

public class ModelCostEntry
{
    public string? Model { get; set; }
    public double? Cost { get; set; }
    public int? Requests { get; set; }
    public int? Tokens { get; set; }
}

public class CostHistoryResponse
{
    public List<JsonElement>? Items { get; set; }
    public int? Total { get; set; }
    public int? Page { get; set; }
    public int? PageSize { get; set; }
}

public class BudgetInfo
{
    public double? Limit { get; set; }
    public double? CurrentUsage { get; set; }
    public string? Period { get; set; }
    public double? PercentUsed { get; set; }
}

// ── Conversations ──
public class ConversationInfo
{
    public string? Id { get; set; }
    public string? TenantId { get; set; }
    public string? WorkspaceId { get; set; }
    public string? Title { get; set; }
    public string? Mode { get; set; }
    public bool? IsPinned { get; set; }
    public string? FolderId { get; set; }
    public string? CreatedAt { get; set; }
    public string? UpdatedAt { get; set; }
    public int? MessageCount { get; set; }
}

/// <summary>
/// WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array.
/// </summary>
public class ConversationListResponse
{
    public List<ConversationInfo>? Items { get; set; }
}

/// <summary>
/// WHY: GET /api/v1/conversations/{id} returns {"conversation":{...},"messages":[...]} wrapper.
/// </summary>
public class ConversationDetail
{
    public ConversationInfo? Conversation { get; set; }
    public List<ConversationMessage>? Messages { get; set; }

    /// <summary>Convenience accessor for conversation ID.</summary>
    public string? Id => Conversation?.Id;
}

public class ConversationMessage
{
    public string? Id { get; set; }
    public string? ConversationId { get; set; }
    public string? ParentId { get; set; }
    public string? Role { get; set; }
    public string? Content { get; set; }
    public string? Mode { get; set; }
    public int? TokensUsed { get; set; }
    public string? CreatedAt { get; set; }
}

public class MessageListResponse
{
    public List<ConversationMessage>? Messages { get; set; }
}

public class BulkDeleteResponse
{
    [JsonPropertyName("affected")]
    public int? Affected { get; set; }

    [JsonPropertyName("deleted")]
    public int? Deleted { get; set; }

    public string? Status { get; set; }
}

public class ShareLinkResponse
{
    public string? ShareId { get; set; }
    public string? ShareUrl { get; set; }
    public string? ExpiresAt { get; set; }
}

public class ConversationImport
{
    public string? Title { get; set; }
    public List<ConversationMessage>? Messages { get; set; }
}

public class ImportResponse
{
    public int? Imported { get; set; }
    public List<string>? ConversationIds { get; set; }
}

// ── Folders ──
public class FolderInfo
{
    public string? Id { get; set; }
    public string? TenantId { get; set; }
    public string? Name { get; set; }
    public string? CreatedAt { get; set; }
    public string? UpdatedAt { get; set; }
}

public class FolderConversationsResponse
{
    public List<ConversationInfo>? Conversations { get; set; }
}

// ── Auth (OODA-36) ──
public class AuthTokenResponse
{
    public string? AccessToken { get; set; }
    public string? RefreshToken { get; set; }
    public string? TokenType { get; set; }
    public int? ExpiresIn { get; set; }
}

public class AuthUserResponse
{
    public string? Id { get; set; }
    public string? Email { get; set; }
    public string? Name { get; set; }
    public string? Role { get; set; }
    public string? TenantId { get; set; }
}

// ── Workspaces (OODA-36) ──
public class WorkspaceListResponse
{
    public List<WorkspaceInfo>? Items { get; set; }
}

public class WorkspaceInfo
{
    public string? Id { get; set; }
    public string? TenantId { get; set; }
    public string? Name { get; set; }
    public string? Description { get; set; }
    public string? CreatedAt { get; set; }
    public string? UpdatedAt { get; set; }
}

public class WorkspaceStatsResponse
{
    public int? DocumentCount { get; set; }
    public int? EntityCount { get; set; }
    public int? RelationshipCount { get; set; }
    public int? ChunkCount { get; set; }
    public double? StorageUsedMb { get; set; }
}

// ── Shared (OODA-36) ──
public class SharedLinkResponse
{
    public string? ShareId { get; set; }
    public string? ConversationId { get; set; }
    public string? ShareUrl { get; set; }
    public string? CreatedAt { get; set; }
    public string? ExpiresAt { get; set; }
}

public class SharedAccessResponse
{
    public ConversationInfo? Conversation { get; set; }
    public List<ConversationMessage>? Messages { get; set; }
}

public class SharedLinksListResponse
{
    public List<SharedLinkResponse>? Items { get; set; }
}
