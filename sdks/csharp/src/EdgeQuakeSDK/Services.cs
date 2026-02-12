using System.Text.Json;
using System.Text.Json.Serialization;

namespace EdgeQuakeSDK;

// WHY: Each service maps 1:1 to an API resource for discoverability.

public class HealthService(HttpHelper http)
{
    public Task<HealthResponse> CheckAsync() => http.GetAsync<HealthResponse>("/health");
}

public class DocumentService(HttpHelper http)
{
    public Task<DocumentListResponse> ListAsync(int page = 1, int pageSize = 20) =>
        http.GetAsync<DocumentListResponse>($"/api/v1/documents?page={page}&page_size={pageSize}");

    public Task<UploadResponse> UploadTextAsync(string title, string content, string fileType = "txt") =>
        http.PostAsync<UploadResponse>("/api/v1/documents",
            new { title, content, file_type = fileType });

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

    public Task<CreateEntityResponse> CreateAsync(string entityName, string entityType, string description, string sourceId) =>
        http.PostAsync<CreateEntityResponse>("/api/v1/graph/entities",
            new { entity_name = entityName, entity_type = entityType, description, source_id = sourceId });

    public Task<EntityDeleteResponse> DeleteAsync(string name) =>
        http.DeleteAsync<EntityDeleteResponse>($"/api/v1/graph/entities/{name}?confirm=true");
}

public class RelationshipService(HttpHelper http)
{
    public Task<RelationshipListResponse> ListAsync(int page = 1, int pageSize = 20) =>
        http.GetAsync<RelationshipListResponse>($"/api/v1/graph/relationships?page={page}&page_size={pageSize}");
}

public class GraphService(HttpHelper http)
{
    public Task<GraphResponse> GetAsync() => http.GetAsync<GraphResponse>("/api/v1/graph");

    public Task<SearchResponse> SearchAsync(string query) =>
        http.GetAsync<SearchResponse>($"/api/v1/graph/nodes/search?q={Uri.EscapeDataString(query)}");
}

public class QueryService(HttpHelper http)
{
    public Task<QueryResponse> ExecuteAsync(string query, string mode = "hybrid") =>
        http.PostAsync<QueryResponse>("/api/v1/query", new { query, mode });
}

public class ChatService(HttpHelper http)
{
    public Task<ChatCompletionResponse> CompletionsAsync(string message, string mode = "hybrid", bool stream = false) =>
        http.PostAsync<ChatCompletionResponse>("/api/v1/chat/completions",
            new { message, mode, stream });
}

public class TenantService(HttpHelper http)
{
    public Task<TenantListResponse> ListAsync() => http.GetAsync<TenantListResponse>("/api/v1/tenants");
}

public class UserService(HttpHelper http)
{
    public Task<UserListResponse> ListAsync() => http.GetAsync<UserListResponse>("/api/v1/users");
}

public class ApiKeyService(HttpHelper http)
{
    public Task<ApiKeyListResponse> ListAsync() => http.GetAsync<ApiKeyListResponse>("/api/v1/api-keys");
}

public class TaskService(HttpHelper http)
{
    public Task<TaskListResponse> ListAsync() => http.GetAsync<TaskListResponse>("/api/v1/tasks");
}

public class PipelineService(HttpHelper http)
{
    public Task<PipelineStatusResponse> StatusAsync() =>
        http.GetAsync<PipelineStatusResponse>("/api/v1/pipeline/status");

    public Task<QueueMetricsResponse> QueueMetricsAsync() =>
        http.GetAsync<QueueMetricsResponse>("/api/v1/pipeline/queue-metrics");
}

public class ModelService(HttpHelper http)
{
    public Task<ProviderCatalog> CatalogAsync() =>
        http.GetAsync<ProviderCatalog>("/api/v1/models");

    public async Task<List<ProviderHealthInfo>> HealthAsync()
    {
        var raw = await http.GetRawAsync("/api/v1/models/health");
        return JsonSerializer.Deserialize<List<ProviderHealthInfo>>(raw, HttpHelper.JsonOptions)
            ?? new List<ProviderHealthInfo>();
    }

    public Task<ProviderStatus> ProviderStatusAsync() =>
        http.GetAsync<ProviderStatus>("/api/v1/settings/provider/status");
}

public class CostService(HttpHelper http)
{
    public Task<CostSummary> SummaryAsync() =>
        http.GetAsync<CostSummary>("/api/v1/costs/summary");
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

    /// <summary>WHY: DELETE returns 204 No Content — no body to deserialize.</summary>
    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/conversations/{id}");

    public Task<BulkDeleteResponse> BulkDeleteAsync(List<string> ids) =>
        http.PostAsync<BulkDeleteResponse>("/api/v1/conversations/bulk/delete", new { ids });
}

public class FolderService(HttpHelper http)
{
    public Task<List<FolderInfo>> ListAsync() =>
        http.GetAsync<List<FolderInfo>>("/api/v1/folders");

    public Task<FolderInfo> CreateAsync(string name) =>
        http.PostAsync<FolderInfo>("/api/v1/folders", new { name });

    /// <summary>WHY: DELETE returns 204 No Content — no body to deserialize.</summary>
    public Task DeleteAsync(string id) =>
        http.DeleteNoContentAsync($"/api/v1/folders/{id}");
}
