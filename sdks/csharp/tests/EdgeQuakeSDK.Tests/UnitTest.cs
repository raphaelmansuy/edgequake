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
        var (http, mock) = MockHelperWithCalls(@"{""affected"":3,""status"":""ok""}");
        var svc = new ConversationService(http);
        var result = await svc.BulkDeleteAsync(new List<string> { "c1", "c2", "c3" });
        Assert.Equal(3, result.Affected);
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
    public async Task Documents_UploadText_SendsContentAndTitle()
    {
        var (http, mock) = MockHelperWithCalls(@"{""document_id"":""d3"",""status"":""processing""}");
        var svc = new DocumentService(http);
        var result = await svc.UploadTextAsync("Title", "Body");
        Assert.Contains("Body", mock.LastCall!.Body!);
        Assert.Contains("Title", mock.LastCall!.Body!);
        Assert.Equal("d3", result.DocumentId);
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

// ═══════════════════════════════════════════════════════════════════════════
// OODA-36: Extended test coverage for all new service methods
// ═══════════════════════════════════════════════════════════════════════════

/// <summary>Tests for extended HealthService methods (OODA-36).</summary>
public class HealthExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Health_Ready()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""ready"",""ready"":true}");
        var svc = new HealthService(http);
        var result = await svc.ReadyAsync();
        Assert.Equal("ready", result.Status);
        Assert.Equal("/ready", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Health_Live()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""alive"",""alive"":true}");
        var svc = new HealthService(http);
        var result = await svc.LiveAsync();
        Assert.Equal("alive", result.Status);
        Assert.Equal("/live", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Health_Metrics()
    {
        var (http, mock) = MockHelperWithCalls(@"{""metrics"":{}}");
        var svc = new HealthService(http);
        await svc.MetricsAsync();
        Assert.Equal("/metrics", mock.LastCall!.Url);
    }
}

/// <summary>Tests for extended DocumentService methods (OODA-36).</summary>
public class DocumentExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Documents_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""d1"",""title"":""Test""}");
        var svc = new DocumentService(http);
        var result = await svc.GetAsync("d1");
        Assert.Equal("d1", result.Id);
        Assert.Contains("/api/v1/documents/d1", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Documents_Track()
    {
        var (http, mock) = MockHelperWithCalls(
            @"{""track_id"":""tk1"",""status"":""processing"",""progress"":0.5}");
        var svc = new DocumentService(http);
        var result = await svc.TrackAsync("tk1");
        Assert.Equal("tk1", result.TrackId);
        Assert.Equal("processing", result.Status);
        Assert.Contains("/api/v1/documents/track/tk1", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Documents_ReprocessFailed()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""ok"",""message"":""Reprocessing started""}");
        var svc = new DocumentService(http);
        await svc.ReprocessFailedAsync();
        Assert.Contains("/api/v1/documents/reprocess", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Documents_RecoverStuck()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""recovered""}");
        var svc = new DocumentService(http);
        await svc.RecoverStuckAsync();
        Assert.Contains("/api/v1/documents/recover-stuck", mock.LastCall!.Url);
    }
}

/// <summary>Tests for extended EntityService methods (OODA-36).</summary>
public class EntityExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Entities_Neighborhood()
    {
        var (http, mock) = MockHelperWithCalls(@"{""entity"":{},""neighbors"":[]}");
        var svc = new EntityService(http);
        await svc.NeighborhoodAsync("ALICE", 2);
        Assert.Contains("/api/v1/graph/entities/ALICE/neighborhood", mock.LastCall!.Url);
        Assert.Contains("depth=2", mock.LastCall.Url);
    }

    [Fact]
    public async Task Entities_Merge()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""merged"",""merged_entity_id"":""e1""}");
        var svc = new EntityService(http);
        var result = await svc.MergeAsync("primary-id", "secondary-id");
        Assert.Equal("merged", result.Status);
        Assert.Contains("/api/v1/graph/entities/merge", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Entities_Exists()
    {
        var (http, mock) = MockHelperWithCalls(@"{""exists"":true,""entity_id"":""E1"",""entity_type"":""PERSON"",""degree"":2}");
        var svc = new EntityService(http);
        var result = await svc.ExistsAsync("ALICE");
        Assert.True(result.Exists);
        Assert.Contains("entity_name=ALICE", mock.LastCall!.Url);
        Assert.Contains("/api/v1/graph/entities/exists", mock.LastCall!.Url);
    }
}

/// <summary>Tests for extended RelationshipService methods (OODA-36).</summary>
public class RelationshipExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Relationships_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""source"":""A"",""target"":""B""}");
        var svc = new RelationshipService(http);
        var result = await svc.GetAsync("rel-1");
        Assert.Equal("A", result.Source);
        Assert.Contains("/api/v1/graph/relationships/rel-1", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Relationships_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""created""}");
        var svc = new RelationshipService(http);
        await svc.CreateAsync("A", "B", "KNOWS", "They know each other");
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
        Assert.Contains("src_id", mock.LastCall!.Body!);
        Assert.Contains("tgt_id", mock.LastCall!.Body);
    }

    [Fact]
    public async Task Relationships_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new RelationshipService(http);
        await svc.DeleteAsync("rel-1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
        Assert.Contains("/api/v1/graph/relationships/rel-1", mock.LastCall!.Url);
    }
}

/// <summary>Tests for extended GraphService methods (OODA-36).</summary>
public class GraphExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Graph_Get_CanonicalPath()
    {
        var (http, mock) = MockHelperWithCalls(@"{""nodes"":[],""edges"":[]}");
        var svc = new GraphService(http);
        _ = await svc.GetAsync();
        Assert.Equal("/api/v1/graph", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Graph_LabelSearch()
    {
        var (http, mock) = MockHelperWithCalls(@"{""labels"":[""PERSON""]}");
        var svc = new GraphService(http);
        await svc.LabelSearchAsync("PERSON");
        Assert.Contains("q=PERSON", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Graph_PopularLabels()
    {
        var (http, mock) = MockHelperWithCalls(@"{""labels"":[]}");
        var svc = new GraphService(http);
        await svc.PopularLabelsAsync(5);
        Assert.Contains("limit=5", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Graph_BatchDegrees()
    {
        var (http, mock) = MockHelperWithCalls(@"{""degrees"":{""n1"":3}}");
        var svc = new GraphService(http);
        var result = await svc.BatchDegreesAsync(new[] { "n1", "n2" });
        Assert.NotNull(result.Degrees);
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }
}

/// <summary>Tests for extended TenantService methods (OODA-36).</summary>
public class TenantExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Tenants_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""t1"",""name"":""test""}");
        var svc = new TenantService(http);
        var result = await svc.GetAsync("t1");
        Assert.Equal("t1", result.Id);
    }

    [Fact]
    public async Task Tenants_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""t2"",""name"":""new""}");
        var svc = new TenantService(http);
        var result = await svc.CreateAsync("new", "New Tenant");
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Tenants_Update()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""t1"",""display_name"":""Updated""}");
        var svc = new TenantService(http);
        await svc.UpdateAsync("t1", "Updated");
        Assert.Equal(HttpMethod.Put, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Tenants_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new TenantService(http);
        await svc.DeleteAsync("t1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
    }
}

/// <summary>Tests for extended UserService methods (OODA-36).</summary>
public class UserExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Users_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""u1"",""email"":""test@test.com""}");
        var svc = new UserService(http);
        var result = await svc.GetAsync("u1");
        Assert.Equal("u1", result.Id);
    }

    [Fact]
    public async Task Users_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""u2"",""email"":""new@test.com""}");
        var svc = new UserService(http);
        await svc.CreateAsync("new@test.com", "Test User");
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Users_Update()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""u1"",""name"":""Updated""}");
        var svc = new UserService(http);
        await svc.UpdateAsync("u1", "Updated", "admin");
        Assert.Equal(HttpMethod.Put, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Users_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new UserService(http);
        await svc.DeleteAsync("u1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
    }
}

/// <summary>Tests for extended ApiKeyService methods (OODA-36).</summary>
public class ApiKeyExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task ApiKeys_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""ak1"",""name"":""test""}");
        var svc = new ApiKeyService(http);
        var result = await svc.GetAsync("ak1");
        Assert.Equal("ak1", result.Id);
    }

    [Fact]
    public async Task ApiKeys_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""ak2"",""key"":""sk-...""}");
        var svc = new ApiKeyService(http);
        var result = await svc.CreateAsync("new-key", new[] { "read", "write" });
        Assert.NotNull(result.Key);
    }

    [Fact]
    public async Task ApiKeys_Revoke()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new ApiKeyService(http);
        await svc.RevokeAsync("ak1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
    }

    [Fact]
    public async Task ApiKeys_Rotate()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""ak1"",""key"":""sk-new...""}");
        var svc = new ApiKeyService(http);
        await svc.RotateAsync("ak1");
        Assert.Contains("/rotate", mock.LastCall!.Url);
    }
}

/// <summary>Tests for extended TaskService methods (OODA-36).</summary>
public class TaskExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Tasks_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""t1"",""status"":""running""}");
        var svc = new TaskService(http);
        var result = await svc.GetAsync("t1");
        Assert.Equal("t1", result.Id);
    }

    [Fact]
    public async Task Tasks_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""t2"",""type"":""extract""}");
        var svc = new TaskService(http);
        await svc.CreateAsync("extract", "doc-1");
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Tasks_Cancel()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""cancelled""}");
        var svc = new TaskService(http);
        await svc.CancelAsync("t1");
        Assert.Contains("/cancel", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Tasks_Status()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""running"",""progress"":75}");
        var svc = new TaskService(http);
        var result = await svc.StatusAsync("t1");
        Assert.Equal(75, result.Progress);
    }

    [Fact]
    public async Task Tasks_Retry()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""t1"",""status"":""queued""}");
        var svc = new TaskService(http);
        await svc.RetryAsync("t1");
        Assert.Contains("/retry", mock.LastCall!.Url);
    }
}

/// <summary>Tests for extended PipelineService methods (OODA-36).</summary>
public class PipelineExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Pipeline_Processing()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[],""total"":0}");
        var svc = new PipelineService(http);
        await svc.ProcessingAsync();
        Assert.Contains("/processing", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Pipeline_Pause()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""paused""}");
        var svc = new PipelineService(http);
        await svc.PauseAsync();
        Assert.Contains("/pause", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Pipeline_Resume()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""resumed""}");
        var svc = new PipelineService(http);
        await svc.ResumeAsync();
        Assert.Contains("/resume", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Pipeline_Cancel()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""cancelled""}");
        var svc = new PipelineService(http);
        await svc.CancelAsync("doc-1");
        Assert.Contains("/cancel", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Pipeline_CostEstimate()
    {
        var (http, mock) = MockHelperWithCalls(@"{""estimated_cost"":0.05,""estimated_tokens"":1000}");
        var svc = new PipelineService(http);
        var result = await svc.CostEstimateAsync("doc-1");
        Assert.Equal(0.05, result.EstimatedCost);
    }
}

/// <summary>Tests for extended ModelService methods (OODA-36).</summary>
public class ModelExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Models_List()
    {
        var (http, mock) = MockHelperWithCalls(@"{""models"":[]}");
        var svc = new ModelService(http);
        await svc.ListAsync();
        Assert.Contains("/list", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Models_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""gpt-4"",""name"":""GPT-4""}");
        var svc = new ModelService(http);
        var result = await svc.GetAsync("gpt-4");
        Assert.Equal("gpt-4", result.Id);
    }

    [Fact]
    public async Task Models_Providers()
    {
        var (http, mock) = MockHelperWithCalls(@"{""providers"":[]}");
        var svc = new ModelService(http);
        await svc.ProvidersAsync();
        Assert.Contains("/providers", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Models_SetDefault()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""success""}");
        var svc = new ModelService(http);
        await svc.SetDefaultAsync("gpt-4");
        Assert.Contains("/default", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Models_Test()
    {
        var (http, mock) = MockHelperWithCalls(@"{""success"":true,""response"":""Hello""}");
        var svc = new ModelService(http);
        var result = await svc.TestAsync("gpt-4", "Say hello");
        Assert.True(result.Success);
    }
}

/// <summary>Tests for extended CostService methods (OODA-36).</summary>
public class CostExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Costs_Daily()
    {
        var (http, mock) = MockHelperWithCalls(@"{""days"":[],""total_cost"":1.50}");
        var svc = new CostService(http);
        var result = await svc.DailyAsync("2026-01-01", "2026-01-31");
        Assert.Equal(1.50, result.TotalCost);
    }

    [Fact]
    public async Task Costs_ByProvider()
    {
        var (http, mock) = MockHelperWithCalls(@"{""providers"":[]}");
        var svc = new CostService(http);
        await svc.ByProviderAsync();
        Assert.Contains("/by-provider", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Costs_ByModel()
    {
        var (http, mock) = MockHelperWithCalls(@"{""models"":[]}");
        var svc = new CostService(http);
        await svc.ByModelAsync();
        Assert.Contains("/by-model", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Costs_History()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[],""total"":0}");
        var svc = new CostService(http);
        await svc.HistoryAsync(2, 50);
        Assert.Contains("page=2", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Costs_Budget()
    {
        var (http, mock) = MockHelperWithCalls(@"{""limit"":100,""current_usage"":25}");
        var svc = new CostService(http);
        var result = await svc.BudgetAsync();
        Assert.Equal(100, result.Limit);
    }

    [Fact]
    public async Task Costs_SetBudget()
    {
        var (http, mock) = MockHelperWithCalls(@"{""limit"":200,""period"":""monthly""}");
        var svc = new CostService(http);
        var result = await svc.SetBudgetAsync(200, "monthly");
        Assert.Equal(200, result.Limit);
    }
}

/// <summary>Tests for extended ConversationService methods (OODA-36).</summary>
public class ConversationExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Conversations_Update()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""c1"",""title"":""Updated""}");
        var svc = new ConversationService(http);
        var result = await svc.UpdateAsync("c1", "Updated", true);
        Assert.Equal(HttpMethod.Put, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Conversations_Messages()
    {
        var (http, mock) = MockHelperWithCalls(@"{""messages"":[]}");
        var svc = new ConversationService(http);
        await svc.MessagesAsync("c1");
        Assert.Contains("/messages", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Conversations_AddMessage()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""m1"",""role"":""user""}");
        var svc = new ConversationService(http);
        var result = await svc.AddMessageAsync("c1", "user", "Hello");
        Assert.Equal("m1", result.Id);
    }

    [Fact]
    public async Task Conversations_DeleteMessage()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new ConversationService(http);
        await svc.DeleteMessageAsync("c1", "m1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Conversations_Search()
    {
        var (http, mock) = MockHelperWithCalls(@"{""results"":[]}");
        var svc = new ConversationService(http);
        await svc.SearchAsync("test query");
        Assert.Contains("q=test%20query", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Conversations_Share()
    {
        var (http, mock) = MockHelperWithCalls(@"{""share_id"":""sh1"",""share_url"":""http://...""}");
        var svc = new ConversationService(http);
        var result = await svc.ShareAsync("c1");
        Assert.Equal("sh1", result.ShareId);
    }
}

/// <summary>Tests for extended FolderService methods (OODA-36).</summary>
public class FolderExtendedTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Folders_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""f1"",""name"":""Work""}");
        var svc = new FolderService(http);
        var result = await svc.GetAsync("f1");
        Assert.Equal("f1", result.Id);
    }

    [Fact]
    public async Task Folders_Update()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""f1"",""name"":""Updated""}");
        var svc = new FolderService(http);
        var result = await svc.UpdateAsync("f1", "Updated");
        Assert.Equal(HttpMethod.Put, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Folders_MoveConversation()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""moved""}");
        var svc = new FolderService(http);
        await svc.MoveConversationAsync("f1", "c1");
        Assert.Contains("/conversations", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Folders_Conversations()
    {
        var (http, mock) = MockHelperWithCalls(@"{""conversations"":[]}");
        var svc = new FolderService(http);
        await svc.ConversationsAsync("f1");
        Assert.Contains("/f1/conversations", mock.LastCall!.Url);
    }
}

/// <summary>Tests for AuthService (OODA-36).</summary>
public class AuthServiceTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Auth_Login()
    {
        var (http, mock) = MockHelperWithCalls(@"{""access_token"":""tok"",""refresh_token"":""ref""}");
        var svc = new AuthService(http);
        var result = await svc.LoginAsync("test@test.com", "password");
        Assert.Equal("tok", result.AccessToken);
    }

    [Fact]
    public async Task Auth_Logout()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new AuthService(http);
        await svc.LogoutAsync();
        Assert.Contains("/logout", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Auth_Refresh()
    {
        var (http, mock) = MockHelperWithCalls(@"{""access_token"":""new-tok""}");
        var svc = new AuthService(http);
        var result = await svc.RefreshAsync("ref-tok");
        Assert.Equal("new-tok", result.AccessToken);
    }

    [Fact]
    public async Task Auth_Me()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""u1"",""email"":""test@test.com""}");
        var svc = new AuthService(http);
        var result = await svc.MeAsync();
        Assert.Equal("u1", result.Id);
    }

    [Fact]
    public async Task Auth_ChangePassword()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""success""}");
        var svc = new AuthService(http);
        var result = await svc.ChangePasswordAsync("old", "new");
        Assert.Equal("success", result.Status);
    }
}

/// <summary>Tests for WorkspaceService (OODA-36).</summary>
public class WorkspaceServiceTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Workspaces_List()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[]}");
        var svc = new WorkspaceService(http);
        await svc.ListAsync();
        Assert.Contains("/workspaces", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Workspaces_Get()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""ws1"",""name"":""Test""}");
        var svc = new WorkspaceService(http);
        var result = await svc.GetAsync("ws1");
        Assert.Equal("ws1", result.Id);
    }

    [Fact]
    public async Task Workspaces_Create()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""ws2"",""name"":""New""}");
        var svc = new WorkspaceService(http);
        await svc.CreateAsync("New", "Description");
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Workspaces_Update()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""ws1"",""name"":""Updated""}");
        var svc = new WorkspaceService(http);
        await svc.UpdateAsync("ws1", "Updated", "New desc");
        Assert.Equal(HttpMethod.Put, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Workspaces_Delete()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new WorkspaceService(http);
        await svc.DeleteAsync("ws1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Workspaces_Stats()
    {
        var (http, mock) = MockHelperWithCalls(@"{""document_count"":10,""entity_count"":50}");
        var svc = new WorkspaceService(http);
        var result = await svc.StatsAsync("ws1");
        Assert.Equal(10, result.DocumentCount);
    }

    [Fact]
    public async Task Workspaces_Switch()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""switched""}");
        var svc = new WorkspaceService(http);
        await svc.SwitchAsync("ws1");
        Assert.Contains("/switch", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Workspaces_Rebuild()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""rebuilding""}");
        var svc = new WorkspaceService(http);
        await svc.RebuildAsync("ws1");
        Assert.Contains("/rebuild", mock.LastCall!.Url);
    }
}

/// <summary>Tests for SharedService (OODA-36).</summary>
public class SharedServiceTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Shared_CreateLink()
    {
        var (http, mock) = MockHelperWithCalls(@"{""share_id"":""sh1"",""share_url"":""http://...""}");
        var svc = new SharedService(http);
        var result = await svc.CreateLinkAsync("c1");
        Assert.Equal("sh1", result.ShareId);
    }

    [Fact]
    public async Task Shared_GetLink()
    {
        var (http, mock) = MockHelperWithCalls(@"{""share_id"":""sh1"",""conversation_id"":""c1""}");
        var svc = new SharedService(http);
        var result = await svc.GetLinkAsync("sh1");
        Assert.Equal("c1", result.ConversationId);
    }

    [Fact]
    public async Task Shared_DeleteLink()
    {
        var (http, mock) = MockHelperWithCalls(@"{}");
        var svc = new SharedService(http);
        await svc.DeleteLinkAsync("sh1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Shared_Access()
    {
        var (http, mock) = MockHelperWithCalls(@"{""conversation"":{},""messages"":[]}");
        var svc = new SharedService(http);
        await svc.AccessAsync("sh1");
        Assert.Contains("/access", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Shared_ListLinks()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[]}");
        var svc = new SharedService(http);
        await svc.ListLinksAsync();
        Assert.Contains("/shared", mock.LastCall!.Url);
    }
}

/// <summary>Tests for new client services availability (OODA-36).</summary>
public class ClientExtendedServiceAvailabilityTests
{
    [Fact]
    public void Client_HasAuth()
    {
        var client = new EdgeQuakeClient();
        Assert.NotNull(client.Auth);
    }

    [Fact]
    public void Client_HasWorkspaces()
    {
        var client = new EdgeQuakeClient();
        Assert.NotNull(client.Workspaces);
    }

    [Fact]
    public void Client_HasShared()
    {
        var client = new EdgeQuakeClient();
        Assert.NotNull(client.Shared);
    }
}

/// <summary>Tests for Query/Chat streaming methods (OODA-36).</summary>
public class StreamingTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Query_Stream()
    {
        var (http, mock) = MockHelperWithCalls(@"data: {""answer"":""test""}");
        var svc = new QueryService(http);
        await svc.StreamAsync("test");
        Assert.Contains("/stream", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Chat_Stream()
    {
        var (http, mock) = MockHelperWithCalls(@"data: {""content"":""Hello""}");
        var svc = new ChatService(http);
        await svc.StreamAsync("Hi");
        Assert.Contains("/stream", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Chat_WithConversation()
    {
        var (http, mock) = MockHelperWithCalls(@"{""content"":""Reply""}");
        var svc = new ChatService(http);
        await svc.CompletionsWithConversationAsync("conv-1", "Follow up");
        Assert.Contains("conversation_id", mock.LastCall!.Body!);
    }
}

/// <summary>OODA-47: Additional edge case tests for comprehensive coverage.</summary>
public class OODA47EdgeCaseTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Documents_Get_NotFound()
    {
        var (http, mock) = MockHelperWithCalls(@"{""error"":""not found""}", HttpStatusCode.NotFound);
        var svc = new DocumentService(http);
        await Assert.ThrowsAsync<EdgeQuakeException>(() => svc.GetAsync("nonexistent"));
    }

    [Fact]
    public async Task Entities_Create_Success()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""e1"",""name"":""Test Entity"",""type"":""PERSON""}");
        var svc = new EntityService(http);
        await svc.CreateAsync("Test Entity", "PERSON", "A test entity", "doc-1");
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Entities_List_Paginated()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[{""id"":""e1""}]}");
        var svc = new EntityService(http);
        await svc.ListAsync(page: 2, pageSize: 50);
        Assert.Contains("page=2", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Graph_Get_Success()
    {
        var (http, mock) = MockHelperWithCalls(@"{""nodes"":[],""edges"":[]}");
        var svc = new GraphService(http);
        await svc.GetAsync();
        Assert.Contains("/graph", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Pipeline_Status_Idle()
    {
        var (http, mock) = MockHelperWithCalls(@"{""is_busy"":false,""pending_tasks"":0}");
        var svc = new PipelineService(http);
        var status = await svc.StatusAsync();
        Assert.NotNull(status);
    }

    [Fact]
    public async Task Pipeline_Status_Busy()
    {
        var (http, mock) = MockHelperWithCalls(@"{""is_busy"":true,""pending_tasks"":5}");
        var svc = new PipelineService(http);
        var status = await svc.StatusAsync();
        Assert.NotNull(status);
    }

    [Fact]
    public async Task Tasks_Get_Completed()
    {
        var (http, mock) = MockHelperWithCalls(@"{""track_id"":""t1"",""status"":""completed""}");
        var svc = new TaskService(http);
        var task = await svc.GetAsync("t1");
        Assert.NotNull(task);
    }

    [Fact]
    public async Task Tasks_Cancel_Success()
    {
        var (http, mock) = MockHelperWithCalls(@"{""success"":true}");
        var svc = new TaskService(http);
        await svc.CancelAsync("t1");
        Assert.Contains("/cancel", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Models_List_Empty()
    {
        var (http, mock) = MockHelperWithCalls(@"{""models"":[]}");
        var svc = new ModelService(http);
        await svc.ListAsync();
        Assert.Contains("/models/list", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Models_HealthAsync()
    {
        var (http, mock) = MockHelperWithCalls(@"[{""provider"":""ollama"",""healthy"":true}]");
        var svc = new ModelService(http);
        await svc.HealthAsync();
        Assert.Contains("/health", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Costs_Summary_Monthly()
    {
        var (http, mock) = MockHelperWithCalls(@"{""total_cost_usd"":100.50,""document_count"":50}");
        var svc = new CostService(http);
        await svc.SummaryAsync();
        Assert.Contains("/summary", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Folders_List_Empty()
    {
        var (http, mock) = MockHelperWithCalls(@"[]");
        var svc = new FolderService(http);
        await svc.ListAsync();
        Assert.Equal(HttpMethod.Get, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Folders_Create_Success()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""f1"",""name"":""TestFolder""}");
        var svc = new FolderService(http);
        await svc.CreateAsync("TestFolder");
        Assert.Contains("name", mock.LastCall!.Body!);
    }

    [Fact]
    public async Task Conversations_Search()
    {
        var (http, mock) = MockHelperWithCalls(@"{""conversations"":[]}");
        var svc = new ConversationService(http);
        await svc.SearchAsync("query");
        Assert.Contains("/search", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Tenants_Get_Success()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""t1"",""name"":""Test Tenant""}");
        var svc = new TenantService(http);
        await svc.GetAsync("t1");
        Assert.Contains("/tenants/t1", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Users_List_Empty()
    {
        var (http, mock) = MockHelperWithCalls(@"{""users"":[]}");
        var svc = new UserService(http);
        await svc.ListAsync();
        Assert.Equal(HttpMethod.Get, mock.LastCall!.Method);
    }
}

/// <summary>OODA-47: Data validation tests.</summary>
public class OODA47ValidationTests
{
    private static (HttpHelper http, MockHttpMessageHandler mock) MockHelperWithCalls(
        string json = "{}", HttpStatusCode status = HttpStatusCode.OK)
    {
        var handler = new MockHttpMessageHandler(json, status);
        var http = new HttpHelper(new EdgeQuakeConfig(), handler);
        return (http, handler);
    }

    [Fact]
    public async Task Document_Upload_ValidatesContent()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""d1"",""title"":""Test""}");
        var svc = new DocumentService(http);
        await svc.UploadTextAsync("Test Document", "Test content");
        Assert.Contains("content", mock.LastCall!.Body!);
    }

    [Fact]
    public async Task Entity_Merge_TwoEntities()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""merged-1""}");
        var svc = new EntityService(http);
        await svc.MergeAsync("e1", "e2");
        Assert.Contains("/merge", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Relationships_Create_Success()
    {
        var (http, mock) = MockHelperWithCalls(@"{""id"":""r1""}");
        var svc = new RelationshipService(http);
        await svc.CreateAsync("e1", "e2", "works", "Employment relation");
        Assert.Equal(HttpMethod.Post, mock.LastCall!.Method);
    }

    [Fact]
    public async Task Relationships_ListPaginated()
    {
        var (http, mock) = MockHelperWithCalls(@"{""items"":[]}");
        var svc = new RelationshipService(http);
        await svc.ListAsync(page: 2, pageSize: 50);
        Assert.Contains("page=2", mock.LastCall!.Url);
    }

    [Fact]
    public async Task ApiKeys_List_Empty()
    {
        var (http, mock) = MockHelperWithCalls(@"{""keys"":[]}");
        var svc = new ApiKeyService(http);
        await svc.ListAsync();
        Assert.Contains("/api-keys", mock.LastCall!.Url);
    }

    [Fact]
    public async Task ApiKeys_Revoke()
    {
        var (http, mock) = MockHelperWithCalls(@"", HttpStatusCode.NoContent);
        var svc = new ApiKeyService(http);
        await svc.RevokeAsync("key-1");
        Assert.Equal(HttpMethod.Delete, mock.LastCall!.Method);
        Assert.Contains("/api-keys/key-1", mock.LastCall!.Url);
    }

    [Fact]
    public async Task Graph_SearchNodes()
    {
        var (http, mock) = MockHelperWithCalls(@"{""nodes"":[]}");
        var svc = new GraphService(http);
        await svc.SearchAsync("test query");
        Assert.Contains("/search", mock.LastCall!.Url);
    }

}
