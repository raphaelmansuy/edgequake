using System.Text.Json;
using System.Text.Json.Serialization;

namespace EdgeQuakeSDK;

// WHY: Each service maps 1:1 to an API resource for discoverability.
// OODA-36: Enhanced with complete API coverage.

public class HealthService(HttpHelper http)
{
    public Task<HealthResponse> CheckAsync() => http.GetAsync<HealthResponse>("/health");
    public Task<ReadinessResponse> ReadyAsync() => http.GetAsync<ReadinessResponse>("/ready");
    public Task<LivenessResponse> LiveAsync() => http.GetAsync<LivenessResponse>("/live");
    public Task<MetricsResponse> MetricsAsync() => http.GetAsync<MetricsResponse>("/metrics");
}

public class DocumentService(HttpHelper http)
{
    public Task<DocumentListResponse> ListAsync(int page = 1, int pageSize = 20) =>
        http.GetAsync<DocumentListResponse>($"/api/v1/documents?page={page}&page_size={pageSize}");

    public Task<DocumentDetailResponse> GetAsync(string id) =>
        http.GetAsync<DocumentDetailResponse>($"/api/v1/documents/{id}");

    public Task<DocumentChunksResponse> ChunksAsync(string id) =>
        http.GetAsync<DocumentChunksResponse>($"/api/v1/documents/{id}/chunks");

    public Task<DocumentStatusResponse> StatusAsync(string id) =>
        http.GetAsync<DocumentStatusResponse>($"/api/v1/documents/{id}/status");

    public Task<UploadResponse> UploadTextAsync(string title, string content, string fileType = "txt") =>
        http.PostAsync<UploadResponse>("/api/v1/documents",
            new { title, content, file_type = fileType });

    public Task<UploadResponse> ReprocessAsync(string id) =>
        http.PostAsync<UploadResponse>($"/api/v1/documents/{id}/reprocess", null);

    public Task<UploadResponse> RecoverStuckAsync() =>
        http.PostAsync<UploadResponse>("/api/v1/documents/recover-stuck", null);

    /// <summary>WHY: DELETE returns 204 No Content — no body to deserialize.</summary>
    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/documents/{id}");
}

public class EntityService(HttpHelper http)
{
    public Task<EntityListResponse> ListAsync(int page = 1, int pageSize = 20) =>
        http.GetAsync<EntityListResponse>($"/api/v1/graph/entities?page={page}&page_size={pageSize}");

    public Task<EntityDetailResponse> GetAsync(string name) =>
        http.GetAsync<EntityDetailResponse>($"/api/v1/graph/entities/{name}");

    public Task<EntityNeighborhoodResponse> NeighborhoodAsync(string name, int depth = 1) =>
        http.GetAsync<EntityNeighborhoodResponse>($"/api/v1/graph/entities/{name}/neighborhood?depth={depth}");

    public Task<CreateEntityResponse> CreateAsync(string entityName, string entityType, string description, string sourceId) =>
        http.PostAsync<CreateEntityResponse>("/api/v1/graph/entities",
            new { entity_name = entityName, entity_type = entityType, description, source_id = sourceId });

    public Task<MergeEntitiesResponse> MergeAsync(string primaryId, string secondaryId) =>
        http.PostAsync<MergeEntitiesResponse>("/api/v1/graph/entities/merge",
            new { primary_id = primaryId, secondary_id = secondaryId });

    public Task<EntityTypesResponse> TypesAsync() =>
        http.GetAsync<EntityTypesResponse>("/api/v1/graph/entities/types");

    public Task<EntityDeleteResponse> DeleteAsync(string name) =>
        http.DeleteAsync<EntityDeleteResponse>($"/api/v1/graph/entities/{name}?confirm=true");
}

public class RelationshipService(HttpHelper http)
{
    public Task<RelationshipListResponse> ListAsync(int page = 1, int pageSize = 20) =>
        http.GetAsync<RelationshipListResponse>($"/api/v1/graph/relationships?page={page}&page_size={pageSize}");

    public Task<RelationshipDetailResponse> GetAsync(string source, string target) =>
        http.GetAsync<RelationshipDetailResponse>($"/api/v1/graph/relationships/{source}/{target}");

    public Task<CreateRelationshipResponse> CreateAsync(string source, string target, string[] keywords, string description) =>
        http.PostAsync<CreateRelationshipResponse>("/api/v1/graph/relationships",
            new { source, target, keywords, description });

    public Task<RelationshipTypesResponse> TypesAsync() =>
        http.GetAsync<RelationshipTypesResponse>("/api/v1/graph/relationships/types");

    public Task DeleteAsync(string source, string target) =>
        http.DeleteNoContentAsync($"/api/v1/graph/relationships/{source}/{target}");
}

public class GraphService(HttpHelper http)
{
    public Task<GraphResponse> GetAsync() => http.GetAsync<GraphResponse>("/api/v1/graph");

    public Task<GraphStatsResponse> StatsAsync() =>
        http.GetAsync<GraphStatsResponse>("/api/v1/graph/stats");

    public Task<SearchResponse> SearchAsync(string query) =>
        http.GetAsync<SearchResponse>($"/api/v1/graph/nodes/search?q={Uri.EscapeDataString(query)}");

    public Task<LabelSearchResponse> LabelSearchAsync(string label) =>
        http.GetAsync<LabelSearchResponse>($"/api/v1/graph/labels/search?label={Uri.EscapeDataString(label)}");

    public Task<PopularLabelsResponse> PopularLabelsAsync(int limit = 10) =>
        http.GetAsync<PopularLabelsResponse>($"/api/v1/graph/labels/popular?limit={limit}");

    public Task<BatchDegreesResponse> BatchDegreesAsync(string[] nodeIds) =>
        http.PostAsync<BatchDegreesResponse>("/api/v1/graph/degrees/batch", new { node_ids = nodeIds });
}

public class QueryService(HttpHelper http)
{
    public Task<QueryResponse> ExecuteAsync(string query, string mode = "hybrid") =>
        http.PostAsync<QueryResponse>("/api/v1/query", new { query, mode });

    public Task<string> StreamAsync(string query, string mode = "hybrid") =>
        http.PostRawAsync("/api/v1/query/stream", new { query, mode, stream = true });
}

public class ChatService(HttpHelper http)
{
    public Task<ChatCompletionResponse> CompletionsAsync(string message, string mode = "hybrid", bool stream = false) =>
        http.PostAsync<ChatCompletionResponse>("/api/v1/chat/completions",
            new { message, mode, stream });

    public Task<ChatCompletionResponse> CompletionsWithConversationAsync(string conversationId, string message, string mode = "hybrid") =>
        http.PostAsync<ChatCompletionResponse>("/api/v1/chat/completions",
            new { conversation_id = conversationId, message, mode });

    public Task<string> StreamAsync(string message, string mode = "hybrid") =>
        http.PostRawAsync("/api/v1/chat/completions/stream", new { message, mode, stream = true });
}

public class TenantService(HttpHelper http)
{
    public Task<TenantListResponse> ListAsync() => http.GetAsync<TenantListResponse>("/api/v1/tenants");

    public Task<TenantInfo> GetAsync(string id) =>
        http.GetAsync<TenantInfo>($"/api/v1/tenants/{id}");

    public Task<TenantInfo> CreateAsync(string name, string displayName) =>
        http.PostAsync<TenantInfo>("/api/v1/tenants", new { name, display_name = displayName });

    public Task<TenantInfo> UpdateAsync(string id, string displayName) =>
        http.PutAsync<TenantInfo>($"/api/v1/tenants/{id}", new { display_name = displayName });

    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/tenants/{id}");
}

public class UserService(HttpHelper http)
{
    public Task<UserListResponse> ListAsync() => http.GetAsync<UserListResponse>("/api/v1/users");

    public Task<UserInfo> GetAsync(string id) =>
        http.GetAsync<UserInfo>($"/api/v1/users/{id}");

    public Task<UserInfo> CreateAsync(string email, string name, string role = "user") =>
        http.PostAsync<UserInfo>("/api/v1/users", new { email, name, role });

    public Task<UserInfo> UpdateAsync(string id, string name, string role) =>
        http.PutAsync<UserInfo>($"/api/v1/users/{id}", new { name, role });

    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/users/{id}");
}

public class ApiKeyService(HttpHelper http)
{
    public Task<ApiKeyListResponse> ListAsync() => http.GetAsync<ApiKeyListResponse>("/api/v1/api-keys");

    public Task<ApiKeyInfo> GetAsync(string id) =>
        http.GetAsync<ApiKeyInfo>($"/api/v1/api-keys/{id}");

    public Task<CreateApiKeyResponse> CreateAsync(string name, string[] scopes) =>
        http.PostAsync<CreateApiKeyResponse>("/api/v1/api-keys", new { name, scopes });

    public Task RevokeAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/api-keys/{id}");

    public Task<CreateApiKeyResponse> RotateAsync(string id) =>
        http.PostAsync<CreateApiKeyResponse>($"/api/v1/api-keys/{id}/rotate", null);
}

public class TaskService(HttpHelper http)
{
    public Task<TaskListResponse> ListAsync() => http.GetAsync<TaskListResponse>("/api/v1/tasks");

    public Task<TaskInfo> GetAsync(string id) =>
        http.GetAsync<TaskInfo>($"/api/v1/tasks/{id}");

    public Task<TaskInfo> CreateAsync(string type, string documentId) =>
        http.PostAsync<TaskInfo>("/api/v1/tasks", new { type, document_id = documentId });

    public Task<StatusResponse> CancelAsync(string id) =>
        http.PostAsync<StatusResponse>($"/api/v1/tasks/{id}/cancel", null);

    public Task<TaskStatusResponse> StatusAsync(string id) =>
        http.GetAsync<TaskStatusResponse>($"/api/v1/tasks/{id}/status");

    public Task<TaskInfo> RetryAsync(string id) =>
        http.PostAsync<TaskInfo>($"/api/v1/tasks/{id}/retry", null);
}

public class PipelineService(HttpHelper http)
{
    public Task<PipelineStatusResponse> StatusAsync() =>
        http.GetAsync<PipelineStatusResponse>("/api/v1/pipeline/status");

    public Task<QueueMetricsResponse> QueueMetricsAsync() =>
        http.GetAsync<QueueMetricsResponse>("/api/v1/pipeline/queue-metrics");

    public Task<ProcessingListResponse> ProcessingAsync() =>
        http.GetAsync<ProcessingListResponse>("/api/v1/pipeline/processing");

    public Task<StatusResponse> PauseAsync() =>
        http.PostAsync<StatusResponse>("/api/v1/pipeline/pause", null);

    public Task<StatusResponse> ResumeAsync() =>
        http.PostAsync<StatusResponse>("/api/v1/pipeline/resume", null);

    public Task<StatusResponse> CancelAsync(string documentId) =>
        http.PostAsync<StatusResponse>("/api/v1/pipeline/cancel", new { document_id = documentId });

    public Task<CostEstimateResponse> CostEstimateAsync(string documentId) =>
        http.GetAsync<CostEstimateResponse>($"/api/v1/pipeline/costs/estimate?document_id={documentId}");
}

public class ModelService(HttpHelper http)
{
    public Task<ProviderCatalog> CatalogAsync() =>
        http.GetAsync<ProviderCatalog>("/api/v1/models");

    public Task<ModelListResponse> ListAsync() =>
        http.GetAsync<ModelListResponse>("/api/v1/models/list");

    public Task<ModelInfo> GetAsync(string modelId) =>
        http.GetAsync<ModelInfo>($"/api/v1/models/{modelId}");

    public async Task<List<ProviderHealthInfo>> HealthAsync()
    {
        var raw = await http.GetRawAsync("/api/v1/models/health");
        return JsonSerializer.Deserialize<List<ProviderHealthInfo>>(raw, HttpHelper.JsonOptions)
            ?? new List<ProviderHealthInfo>();
    }

    public Task<ProviderStatus> ProviderStatusAsync() =>
        http.GetAsync<ProviderStatus>("/api/v1/settings/provider/status");

    public Task<ProviderListResponse> ProvidersAsync() =>
        http.GetAsync<ProviderListResponse>("/api/v1/settings/providers");

    public Task<StatusResponse> SetDefaultAsync(string modelId) =>
        http.PostAsync<StatusResponse>("/api/v1/models/default", new { model_id = modelId });

    public Task<ModelTestResponse> TestAsync(string modelId, string prompt) =>
        http.PostAsync<ModelTestResponse>("/api/v1/models/test", new { model_id = modelId, prompt });
}

public class CostService(HttpHelper http)
{
    public Task<CostSummary> SummaryAsync() =>
        http.GetAsync<CostSummary>("/api/v1/costs/summary");

    public Task<DailyCostResponse> DailyAsync(string startDate, string endDate) =>
        http.GetAsync<DailyCostResponse>($"/api/v1/costs/daily?start_date={startDate}&end_date={endDate}");

    public Task<ProviderCostResponse> ByProviderAsync() =>
        http.GetAsync<ProviderCostResponse>("/api/v1/costs/by-provider");

    public Task<ModelCostResponse> ByModelAsync() =>
        http.GetAsync<ModelCostResponse>("/api/v1/costs/by-model");

    public Task<CostHistoryResponse> HistoryAsync(int page = 1, int pageSize = 20) =>
        http.GetAsync<CostHistoryResponse>($"/api/v1/costs/history?page={page}&page_size={pageSize}");

    public Task<string> ExportAsync(string format = "csv") =>
        http.GetRawAsync($"/api/v1/costs/export?format={format}");

    public Task<BudgetInfo> BudgetAsync() =>
        http.GetAsync<BudgetInfo>("/api/v1/costs/budget");

    public Task<BudgetInfo> SetBudgetAsync(double limit, string period = "monthly") =>
        http.PostAsync<BudgetInfo>("/api/v1/costs/budget", new { limit, period });
}

public class ConversationService(HttpHelper http)
{
    /// <summary>WHY: GET /api/v1/conversations returns {"items":[...]} wrapper.</summary>
    public async Task<List<ConversationInfo>> ListAsync()
    {
        var wrapper = await http.GetAsync<ConversationListResponse>("/api/v1/conversations");
        return wrapper.Items ?? new List<ConversationInfo>();
    }

    public Task<ConversationInfo> CreateAsync(string title) =>
        http.PostAsync<ConversationInfo>("/api/v1/conversations", new { title });

    public Task<ConversationDetail> GetAsync(string id) =>
        http.GetAsync<ConversationDetail>($"/api/v1/conversations/{id}");

    public Task<ConversationInfo> UpdateAsync(string id, string title, bool isPinned = false) =>
        http.PutAsync<ConversationInfo>($"/api/v1/conversations/{id}", new { title, is_pinned = isPinned });

    /// <summary>WHY: DELETE returns 204 No Content — no body to deserialize.</summary>
    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/conversations/{id}");

    public Task<BulkDeleteResponse> BulkDeleteAsync(List<string> ids) =>
        http.PostAsync<BulkDeleteResponse>("/api/v1/conversations/bulk/delete", new { ids });

    public Task<MessageListResponse> MessagesAsync(string conversationId) =>
        http.GetAsync<MessageListResponse>($"/api/v1/conversations/{conversationId}/messages");

    public Task<ConversationMessage> AddMessageAsync(string conversationId, string role, string content) =>
        http.PostAsync<ConversationMessage>($"/api/v1/conversations/{conversationId}/messages",
            new { role, content });

    public Task DeleteMessageAsync(string conversationId, string messageId) =>
        http.DeleteNoContentAsync($"/api/v1/conversations/{conversationId}/messages/{messageId}");

    public Task<SearchResponse> SearchAsync(string query) =>
        http.GetAsync<SearchResponse>($"/api/v1/conversations/search?q={Uri.EscapeDataString(query)}");

    public Task<ShareLinkResponse> ShareAsync(string id) =>
        http.PostAsync<ShareLinkResponse>($"/api/v1/conversations/{id}/share", null);

    public Task<ImportResponse> ImportAsync(List<ConversationImport> conversations) =>
        http.PostAsync<ImportResponse>("/api/v1/conversations/import", new { conversations });
}

public class FolderService(HttpHelper http)
{
    public Task<List<FolderInfo>> ListAsync() =>
        http.GetAsync<List<FolderInfo>>("/api/v1/folders");

    public Task<FolderInfo> GetAsync(string id) =>
        http.GetAsync<FolderInfo>($"/api/v1/folders/{id}");

    public Task<FolderInfo> CreateAsync(string name) =>
        http.PostAsync<FolderInfo>("/api/v1/folders", new { name });

    public Task<FolderInfo> UpdateAsync(string id, string name) =>
        http.PutAsync<FolderInfo>($"/api/v1/folders/{id}", new { name });

    /// <summary>WHY: DELETE returns 204 No Content — no body to deserialize.</summary>
    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/folders/{id}");

    public Task<StatusResponse> MoveConversationAsync(string folderId, string conversationId) =>
        http.PostAsync<StatusResponse>($"/api/v1/folders/{folderId}/conversations",
            new { conversation_id = conversationId });

    public Task<FolderConversationsResponse> ConversationsAsync(string folderId) =>
        http.GetAsync<FolderConversationsResponse>($"/api/v1/folders/{folderId}/conversations");
}

// ── Auth Service (OODA-36) ─────────────────────────────────────

public class AuthService(HttpHelper http)
{
    public Task<AuthTokenResponse> LoginAsync(string email, string password) =>
        http.PostAsync<AuthTokenResponse>("/api/v1/auth/login", new { email, password });

    public Task LogoutAsync() =>
        http.PostRawAsync("/api/v1/auth/logout", null);

    public Task<AuthTokenResponse> RefreshAsync(string refreshToken) =>
        http.PostAsync<AuthTokenResponse>("/api/v1/auth/refresh", new { refresh_token = refreshToken });

    public Task<AuthUserResponse> MeAsync() =>
        http.GetAsync<AuthUserResponse>("/api/v1/auth/me");

    public Task<StatusResponse> ChangePasswordAsync(string currentPassword, string newPassword) =>
        http.PostAsync<StatusResponse>("/api/v1/auth/change-password",
            new { current_password = currentPassword, new_password = newPassword });
}

// ── Workspace Service (OODA-36) ────────────────────────────────

public class WorkspaceService(HttpHelper http)
{
    public Task<WorkspaceListResponse> ListAsync() =>
        http.GetAsync<WorkspaceListResponse>("/api/v1/workspaces");

    public Task<WorkspaceInfo> GetAsync(string id) =>
        http.GetAsync<WorkspaceInfo>($"/api/v1/workspaces/{id}");

    public Task<WorkspaceInfo> CreateAsync(string name, string description) =>
        http.PostAsync<WorkspaceInfo>("/api/v1/workspaces", new { name, description });

    public Task<WorkspaceInfo> UpdateAsync(string id, string name, string description) =>
        http.PutAsync<WorkspaceInfo>($"/api/v1/workspaces/{id}", new { name, description });

    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/workspaces/{id}");

    public Task<WorkspaceStatsResponse> StatsAsync(string id) =>
        http.GetAsync<WorkspaceStatsResponse>($"/api/v1/workspaces/{id}/stats");

    public Task<StatusResponse> SwitchAsync(string id) =>
        http.PostAsync<StatusResponse>($"/api/v1/workspaces/{id}/switch", null);

    public Task<StatusResponse> RebuildAsync(string id) =>
        http.PostAsync<StatusResponse>($"/api/v1/workspaces/{id}/rebuild", null);
}

// ── Shared Service (OODA-36) ───────────────────────────────────

public class SharedService(HttpHelper http)
{
    public Task<SharedLinkResponse> CreateLinkAsync(string conversationId) =>
        http.PostAsync<SharedLinkResponse>("/api/v1/shared", new { conversation_id = conversationId });

    public Task<SharedLinkResponse> GetLinkAsync(string shareId) =>
        http.GetAsync<SharedLinkResponse>($"/api/v1/shared/{shareId}");

    public Task DeleteLinkAsync(string shareId) =>
        http.DeleteNoContentAsync($"/api/v1/shared/{shareId}");

    public Task<SharedAccessResponse> AccessAsync(string shareId) =>
        http.GetAsync<SharedAccessResponse>($"/api/v1/shared/{shareId}/access");

    public Task<SharedLinksListResponse> ListLinksAsync() =>
        http.GetAsync<SharedLinksListResponse>("/api/v1/shared");
}
