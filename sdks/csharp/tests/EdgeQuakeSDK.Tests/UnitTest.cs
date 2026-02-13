using System.Net;
using Xunit;

namespace EdgeQuakeSDK.Tests;

/// <summary>
/// Unit tests for the EdgeQuake C# SDK.
/// WHY: Verify all components without making real HTTP calls.
/// </summary>
public class UnitTest
{
    private static HttpHelper MockHelper(string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        return new HttpHelper(new EdgeQuakeConfig(), handler);
    }

    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    // ── Config Tests ───────────────────────────────────────────────

    [Fact]
    public void Config_Defaults()
    {
        var c = new EdgeQuakeConfig();
        Assert.Equal("http://localhost:8080", c.BaseUrl);
        Assert.Null(c.ApiKey);
        Assert.Null(c.TenantId);
        Assert.Null(c.UserId);
        Assert.Null(c.WorkspaceId);
        Assert.Equal(60, c.TimeoutSeconds);
    }

    [Fact]
    public void Config_CustomValues()
    {
        var c = new EdgeQuakeConfig
        {
            BaseUrl = "https://api.example.com",
            ApiKey = "sk-test",
            TenantId = "t-1",
            UserId = "u-1",
            WorkspaceId = "ws-1",
            TimeoutSeconds = 120,
        };
        Assert.Equal("https://api.example.com", c.BaseUrl);
        Assert.Equal("sk-test", c.ApiKey);
        Assert.Equal("t-1", c.TenantId);
        Assert.Equal("u-1", c.UserId);
        Assert.Equal("ws-1", c.WorkspaceId);
        Assert.Equal(120, c.TimeoutSeconds);
    }

    // ── Exception Tests ────────────────────────────────────────────

    [Fact]
    public void Exception_Properties()
    {
        var ex = new EdgeQuakeException("bad request", 400, @"{""error"":""fail""}");
        Assert.Equal("bad request", ex.Message);
        Assert.Equal(400, ex.StatusCode);
        Assert.Equal(@"{""error"":""fail""}", ex.ResponseBody);
    }

    [Fact]
    public void Exception_IsSystemException()
    {
        var ex = new EdgeQuakeException("test");
        Assert.IsAssignableFrom<Exception>(ex);
    }

    [Fact]
    public void Exception_NullDefaults()
    {
        var ex = new EdgeQuakeException("test");
        Assert.Null(ex.StatusCode);
        Assert.Null(ex.ResponseBody);
    }

    // ── Client Tests ───────────────────────────────────────────────

    [Fact]
    public void Client_InitializesAllServices()
    {
        var client = new EdgeQuakeClient();
        Assert.NotNull(client.Health);
        Assert.NotNull(client.Documents);
        Assert.NotNull(client.Entities);
        Assert.NotNull(client.Relationships);
        Assert.NotNull(client.Graph);
        Assert.NotNull(client.Query);
        Assert.NotNull(client.Chat);
        Assert.NotNull(client.Tenants);
        Assert.NotNull(client.Users);
        Assert.NotNull(client.ApiKeys);
        Assert.NotNull(client.Tasks);
        Assert.NotNull(client.Pipeline);
        Assert.NotNull(client.Models);
        Assert.NotNull(client.Costs);
    }

    // ── Health Service ─────────────────────────────────────────────

    [Fact]
    public async Task Health_Check()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""healthy"",""version"":""0.1.0""}");
        var svc = new HealthService(http);
        var result = await svc.CheckAsync();
        Assert.Equal("healthy", result.Status);
        Assert.Equal("0.1.0", result.Version);
        Assert.Equal(HttpMethod.Get, mock.LastCall!.Method);
        Assert.Equal("/health", mock.LastCall.Url);
    }

    [Fact]
    public async Task Health_Check_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new HealthService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.CheckAsync());
    }

    // ── Document Service ───────────────────────────────────────────

    [Fact]
    public async Task Documents_List()
    {
        var (http, mock) = MockHelperWithCalls(@"{""documents"":[{""id"":""d1""}],""total"":1}");
        var svc = new DocumentService(http);
        var result = await svc.ListAsync();
        Assert.NotNull(result.Documents);
        Assert.Single(result.Documents!);
        Assert.Contains("page=1", mock.LastCall!.Url);
        Assert.Contains("page_size=20", mock.LastCall.Url);
    }

    [Fact]
    public async Task Documents_List_Pagination()
    {
        var (http, mock) = MockHelperWithCalls(@"{""documents"":[]}");
        var svc = new DocumentService(http);
        await svc.ListAsync(3, 50);
        Assert.Contains("page=3", mock.LastCall!.Url);
        Assert.Contains("page_size=50", mock.LastCall.Url);
    }

    [Fact]
    public async Task Documents_UploadText()
    {
        var (http, mock) = MockHelperWithCalls(@"{""document_id"":""d2"",""status"":""processing""}");
        var svc = new DocumentService(http);
        var result = await svc.UploadTextAsync("My Title", "Hello World");
        Assert.Equal("d2", result.DocumentId);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
        Assert.Contains("My Title", mock.LastCall.Body!);
    }

    [Fact]
    public async Task Documents_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""deleted""}");
        var svc = new DocumentService(http);
        await svc.DeleteAsync("d1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
        Assert.Contains("/api/v1/documents/d1", mock.LastCall.Url);
    }

    [Fact]
    public async Task Documents_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new DocumentService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── Entity Service ─────────────────────────────────────────────

    [Fact]
    public async Task Entities_List()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[{""entity_name"":""ALICE""}],""total"":1}");
        var svc = new EntityService(http);
        var result = await svc.ListAsync();
        Assert.NotNull(result.Items);
        Assert.Single(result.Items!);
    }

    [Fact]
    public async Task Entities_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""entity"":{""entity_name"":""ALICE""}}");
        var svc = new EntityService(http);
        await svc.GetAsync("ALICE");
        Assert.Contains("/api/v1/graph/entities/ALICE", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Entities_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""success""}");
        var svc = new EntityService(http);
        var result = await svc.CreateAsync("BOB", "person", "A person", "src-1");
        Assert.Equal("success", result.Status);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
        Assert.Contains("BOB", mock.LastCall.Body!);
    }

    [Fact]
    public async Task Entities_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""deleted""}");
        var svc = new EntityService(http);
        await svc.DeleteAsync("BOB");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
        Assert.Contains("confirm=true", mock.LastCall.Url);
    }

    [Fact]
    public async Task Entities_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new EntityService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── Relationship Service ───────────────────────────────────────

    [Fact]
    public async Task Relationships_List()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[{""source"":""A"",""target"":""B""}],""total"":1}");
        var svc = new RelationshipService(http);
        var result = await svc.ListAsync();
        Assert.NotNull(result.Items);
        Assert.Single(result.Items!);
    }

    [Fact]
    public async Task Relationships_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new RelationshipService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── Graph Service ──────────────────────────────────────────────

    [Fact]
    public async Task Graph_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""nodes"":[],""edges"":[]}");
        var svc = new GraphService(http);
        var result = await svc.GetAsync();
        Assert.NotNull(result.Nodes);
    }

    [Fact]
    public async Task Graph_Search()
    {
        var (http, mock) = MockHelperWithCalls(@"{""results"":[{""id"":""n1""}]}");
        var svc = new GraphService(http);
        var result = await svc.SearchAsync("Alice");
        Assert.NotNull(result.Results);
        Assert.Single(result.Results!);
        Assert.Contains("q=Alice", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Graph_Search_UrlEncoding()
    {
        var (http, mock) = MockHelperWithCalls(@"{""results"":[]}");
        var svc = new GraphService(http);
        await svc.SearchAsync("hello world");
        Assert.Contains("q=hello%20world", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Graph_Get_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new GraphService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.GetAsync());
    }

    // ── Query Service ──────────────────────────────────────────────

    [Fact]
    public async Task Query_Execute()
    {
        var (http, mock) = MockHelperWithCalls(@"{""answer"":""42"",""sources"":[]}");
        var svc = new QueryService(http);
        var result = await svc.ExecuteAsync("meaning of life");
        Assert.Equal("42", result.Answer);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
        Assert.Contains("meaning of life", mock.LastCall.Body!);
    }

    [Fact]
    public async Task Query_Execute_WithMode()
    {
        var (http, mock) = MockHelperWithCalls(@"{""answer"":""yes"",""mode"":""local""}");
        var svc = new QueryService(http);
        var result = await svc.ExecuteAsync("test", "local");
        Assert.Contains("local", mock.LastCall!.Body!);
    }

    [Fact]
    public async Task Query_Execute_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new QueryService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ExecuteAsync("test"));
    }

    // ── Chat Service ───────────────────────────────────────────────

    [Fact]
    public async Task Chat_Completions()
    {
        var (http, mock) = MockHelperWithCalls(@"{""content"":""Hello!""}");
        var svc = new ChatService(http);
        var result = await svc.CompletionsAsync("Hi");
        Assert.Equal("Hello!", result.Content);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Chat_Completions_WithOptions()
    {
        var (http, mock) = MockHelperWithCalls(@"{""content"":""ok""}");
        var svc = new ChatService(http);
        await svc.CompletionsAsync("hi", "global", true);
        Assert.Contains("global", mock.LastCall!.Body!);
        Assert.Contains("true", mock.LastCall.Body!);
    }

    [Fact]
    public async Task Chat_Completions_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new ChatService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.CompletionsAsync("test"));
    }

    // ── Tenant Service ─────────────────────────────────────────────

    [Fact]
    public async Task Tenants_List()
    {
        var http = MockHelper(@"{""items"":[{""id"":""t1""}]}");
        var svc = new TenantService(http);
        var result = await svc.ListAsync();
        Assert.NotNull(result.Items);
        Assert.Single(result.Items!);
    }

    [Fact]
    public async Task Tenants_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new TenantService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── User Service ───────────────────────────────────────────────

    [Fact]
    public async Task Users_List()
    {
        var http = MockHelper(@"{""users"":[{""id"":""u1""}]}");
        var svc = new UserService(http);
        var result = await svc.ListAsync();
        Assert.NotNull(result.Users);
    }

    [Fact]
    public async Task Users_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new UserService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── API Key Service ────────────────────────────────────────────

    [Fact]
    public async Task ApiKeys_List()
    {
        var http = MockHelper(@"{""keys"":[{""id"":""ak-1""}]}");
        var svc = new ApiKeyService(http);
        var result = await svc.ListAsync();
        Assert.NotNull(result.Keys);
    }

    [Fact]
    public async Task ApiKeys_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new ApiKeyService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── Task Service ───────────────────────────────────────────────

    [Fact]
    public async Task Tasks_List()
    {
        var http = MockHelper(@"{""tasks"":[{""track_id"":""trk-1""}]}");
        var svc = new TaskService(http);
        var result = await svc.ListAsync();
        Assert.NotNull(result.Tasks);
    }

    [Fact]
    public async Task Tasks_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new TaskService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── Pipeline Service ───────────────────────────────────────────

    [Fact]
    public async Task Pipeline_Status()
    {
        var http = MockHelper(@"{""is_busy"":true,""pending_tasks"":5}");
        var svc = new PipelineService(http);
        var result = await svc.StatusAsync();
        Assert.True(result.IsBusy);
    }

    [Fact]
    public async Task Pipeline_QueueMetrics()
    {
        var http = MockHelper(@"{""pending_count"":10}");
        var svc = new PipelineService(http);
        var result = await svc.QueueMetricsAsync();
        Assert.Equal(10, result.PendingCount);
    }

    [Fact]
    public async Task Pipeline_Status_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new PipelineService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.StatusAsync());
    }

    // ── Model Service ──────────────────────────────────────────────

    [Fact]
    public async Task Models_Catalog()
    {
        var http = MockHelper(@"{""providers"":[{""name"":""openai""}]}");
        var svc = new ModelService(http);
        var result = await svc.CatalogAsync();
        Assert.NotNull(result.Providers);
        Assert.Single(result.Providers!);
    }

    [Fact]
    public async Task Models_Health()
    {
        var http = MockHelper(@"[{""name"":""ollama"",""enabled"":true}]");
        var svc = new ModelService(http);
        var result = await svc.HealthAsync();
        Assert.Single(result);
        Assert.Equal("ollama", result[0].Name);
    }

    [Fact]
    public async Task Models_ProviderStatus()
    {
        var http = MockHelper(@"{""provider"":{""name"":""ollama""}}");
        var svc = new ModelService(http);
        var result = await svc.ProviderStatusAsync();
        Assert.NotNull(result.Provider);
    }

    [Fact]
    public async Task Models_Catalog_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new ModelService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.CatalogAsync());
    }

    // ── Cost Service ───────────────────────────────────────────────

    [Fact]
    public async Task Costs_Summary()
    {
        var http = MockHelper(@"{""total_cost"":12.5}");
        var svc = new CostService(http);
        var result = await svc.SummaryAsync();
        Assert.Equal(12.5, result.TotalCost);
    }

    [Fact]
    public async Task Costs_Summary_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new CostService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.SummaryAsync());
    }

    // ── Mock Tests ─────────────────────────────────────────────────

    [Fact]
    public async Task Mock_TracksAllCalls()
    {
        var handler = new MockHttpMessageHandler(@"{""status"":""healthy""}");
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        var svc = new HealthService(http);
        await svc.CheckAsync();
        await svc.CheckAsync();
        Assert.Equal(2, handler.Calls.Count);
    }

    [Fact]
    public async Task Mock_ErrorIncludesStatusCode()
    {
        var http = MockHelper(@"{""error"":""not found""}", HttpStatusCode.NotFound);
        var svc = new HealthService(http);
        var ex = await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.CheckAsync());
        Assert.Equal(404, ex.StatusCode);
    }

    [Fact]
    public async Task Mock_WillReturnChaining()
    {
        var handler = new MockHttpMessageHandler().WillReturn(@"{""status"":""ok""}", HttpStatusCode.OK);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        var svc = new HealthService(http);
        var result = await svc.CheckAsync();
        Assert.Equal("ok", result.Status);
    }

    // ── Conversation Service ───────────────────────────────────────

    [Fact]
    public async Task Conversations_List()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[{""id"":""c1"",""title"":""Test Chat""}]}");
        var svc = new ConversationService(http);
        var result = await svc.ListAsync();
        Assert.Single(result);
        Assert.Equal("c1", result[0].Id);
        Assert.Equal(HttpMethod.Get, mock.LastCall!.Method);
        Assert.Equal("/api/v1/conversations", mock.LastCall.Url);
    }

    [Fact]
    public async Task Conversations_List_EmptyItems()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[]}");
        var svc = new ConversationService(http);
        var result = await svc.ListAsync();
        Assert.Empty(result);
    }

    [Fact]
    public async Task Conversations_List_NullItems()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new ConversationService(http);
        var result = await svc.ListAsync();
        Assert.Empty(result);
    }

    [Fact]
    public async Task Conversations_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""c2"",""title"":""New Chat""}");
        var svc = new ConversationService(http);
        var result = await svc.CreateAsync("New Chat");
        Assert.Equal("c2", result.Id);
        Assert.Equal("New Chat", result.Title);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
        Assert.Contains("New Chat", mock.LastCall.Body!);
    }

    [Fact]
    public async Task Conversations_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""conversation"":{""id"":""c1"",""title"":""Test""},""messages"":[]}");
        var svc = new ConversationService(http);
        var result = await svc.GetAsync("c1");
        Assert.Equal("c1", result.Id);
        Assert.Contains("/api/v1/conversations/c1", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Conversations_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new ConversationService(http);
        await svc.DeleteAsync("c1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
        Assert.Contains("/api/v1/conversations/c1", mock.LastCall.Url);
    }

    [Fact]
    public async Task Conversations_BulkDelete()
    {
        var (http, mock) = MockHelperWithCalls(@"{""deleted"":3,""status"":""ok""}");
        var svc = new ConversationService(http);
        var result = await svc.BulkDeleteAsync(new List<string> { "c1", "c2", "c3" });
        Assert.Equal(3, result.Deleted);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
        Assert.Contains("bulk/delete", mock.LastCall.Url);
    }

    [Fact]
    public async Task Conversations_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new ConversationService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── Folder Service ─────────────────────────────────────────────

    [Fact]
    public async Task Folders_List()
    {
        var (http, mock) = MockHelperWithCalls(@"[{""id"":""f1"",""name"":""Research""}]");
        var svc = new FolderService(http);
        var result = await svc.ListAsync();
        Assert.Single(result);
        Assert.Equal("Research", result[0].Name);
        Assert.Equal(HttpMethod.Get, mock.LastCall!.Method);
        Assert.Equal("/api/v1/folders", mock.LastCall.Url);
    }

    [Fact]
    public async Task Folders_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""f2"",""name"":""New Folder""}");
        var svc = new FolderService(http);
        var result = await svc.CreateAsync("New Folder");
        Assert.Equal("f2", result.Id);
        Assert.Equal("New Folder", result.Name);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
        Assert.Contains("New Folder", mock.LastCall.Body!);
    }

    [Fact]
    public async Task Folders_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new FolderService(http);
        await svc.DeleteAsync("f1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
        Assert.Contains("/api/v1/folders/f1", mock.LastCall.Url);
    }

    [Fact]
    public async Task Folders_List_Error()
    {
        var http = MockHelper("{}", HttpStatusCode.InternalServerError);
        var svc = new FolderService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ListAsync());
    }

    // ── URL Validation Tests ───────────────────────────────────────

    [Fact]
    public async Task Health_CheckUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""ok""}");
        var svc = new HealthService(http);
        await svc.CheckAsync();
        Assert.Equal("/health", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Tasks_ListUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""tasks"":[]}");
        var svc = new TaskService(http);
        await svc.ListAsync();
        Assert.Equal("/api/v1/tasks", mock.LastCall!.Url);
    }

    [Fact]
    public async Task ApiKeys_ListUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""keys"":[]}");
        var svc = new ApiKeyService(http);
        await svc.ListAsync();
        Assert.Equal("/api/v1/api-keys", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Users_ListUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""users"":[]}");
        var svc = new UserService(http);
        await svc.ListAsync();
        Assert.Equal("/api/v1/users", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Tenants_ListUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[]}");
        var svc = new TenantService(http);
        await svc.ListAsync();
        Assert.Equal("/api/v1/tenants", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Pipeline_StatusUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""is_busy"":false}");
        var svc = new PipelineService(http);
        await svc.StatusAsync();
        Assert.Equal("/api/v1/pipeline/status", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Pipeline_QueueMetricsUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""pending_count"":0}");
        var svc = new PipelineService(http);
        await svc.QueueMetricsAsync();
        Assert.Equal("/api/v1/pipeline/queue-metrics", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Costs_SummaryUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""total_cost"":0}");
        var svc = new CostService(http);
        await svc.SummaryAsync();
        Assert.Equal("/api/v1/costs/summary", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Models_CatalogUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""providers"":[]}");
        var svc = new ModelService(http);
        await svc.CatalogAsync();
        Assert.Equal("/api/v1/models", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Models_ProviderStatusUrl()
    {
        var (http, mock) = MockHelperWithCalls(@"{""provider"":{}}");
        var svc = new ModelService(http);
        await svc.ProviderStatusAsync();
        Assert.Equal("/api/v1/settings/provider/status", mock.LastCall!.Url);
    }

    // ── Client Service Availability ────────────────────────────────

    [Fact]
    public void Client_HasConversations()
    {
        var client = new EdgeQuakeClient();
        Assert.NotNull(client.Conversations);
    }

    [Fact]
    public void Client_HasFolders()
    {
        var client = new EdgeQuakeClient();
        Assert.NotNull(client.Folders);
    }

    // ── Documents Edge Cases ───────────────────────────────────────

    [Fact]
    public async Task Documents_UploadText_DefaultFileType()
    {
        var (http, mock) = MockHelperWithCalls(@"{""document_id"":""d3"",""status"":""processing""}");
        var svc = new DocumentService(http);
        var result = await svc.UploadTextAsync("Title", "Body");
        Assert.Contains("txt", mock.LastCall!.Body!);
    }

    [Fact]
    public async Task Documents_Delete_Url()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new DocumentService(http);
        await svc.DeleteAsync("doc-abc");
        Assert.Contains("/api/v1/documents/doc-abc", mock.LastCall!.Url);
    }

    // ── Query Default Mode ─────────────────────────────────────────

    [Fact]
    public async Task Query_DefaultMode()
    {
        var (http, mock) = MockHelperWithCalls(@"{""answer"":""x""}");
        var svc = new QueryService(http);
        await svc.ExecuteAsync("test");
        Assert.Contains("hybrid", mock.LastCall!.Body!);
    }

    // ── Error Response Bodies ──────────────────────────────────────

    [Fact]
    public async Task Exception_Contains_ResponseBody()
    {
        var http = MockHelper(@"{""error"":""quota exceeded""}", HttpStatusCode.TooManyRequests);
        var svc = new QueryService(http);
        var ex = await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.ExecuteAsync("test"));
        Assert.Equal(429, ex.StatusCode);
    }

    [Fact]
    public async Task Exception_BadGateway()
    {
        var http = MockHelper("{}", HttpStatusCode.BadGateway);
        var svc = new HealthService(http);
        var ex = await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.CheckAsync());
        Assert.Equal(502, ex.StatusCode);
    }
}
