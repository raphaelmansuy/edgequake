using System.Text.Json;
using System.Text.Json.Serialization;

namespace EdgeQuakeSDK;

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

public class UploadResponse
{
    public string? DocumentId { get; set; }
    public string? Status { get; set; }
    public string? TrackId { get; set; }
    public string? DuplicateOf { get; set; }
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

public class CreateEntityResponse
{
    public string? Status { get; set; }
    public string? Message { get; set; }
    public JsonElement? Entity { get; set; }
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

public class UserListResponse
{
    public List<JsonElement>? Users { get; set; }
}

public class ApiKeyListResponse
{
    public List<JsonElement>? Keys { get; set; }
}

// ── Tasks ──
public class TaskListResponse
{
    public List<JsonElement>? Tasks { get; set; }
    public List<JsonElement>? Items { get; set; }
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

// ── Models ──
public class ProviderCatalog
{
    public List<JsonElement>? Providers { get; set; }
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

// ── Costs ──
public class CostSummary
{
    public double? TotalCost { get; set; }
    public int? DocumentCount { get; set; }
    public int? QueryCount { get; set; }
    public List<JsonElement>? Entries { get; set; }
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

public class BulkDeleteResponse
{
    public int? Deleted { get; set; }
    public string? Status { get; set; }
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
