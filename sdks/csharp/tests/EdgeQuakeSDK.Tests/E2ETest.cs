using EdgeQuakeSDK;
using Xunit;
using Xunit.Abstractions;

namespace EdgeQuakeSDK.Tests;

/// <summary>
/// E2E tests for EdgeQuake C# SDK.
/// Requires running backend at localhost:8080 (or EDGEQUAKE_BASE_URL).
/// </summary>
[TestCaseOrderer("EdgeQuakeSDK.Tests.AlphabeticalOrderer", "EdgeQuakeSDK.Tests")]
public class E2ETest
{
    private readonly EdgeQuakeClient _client;
    private readonly ITestOutputHelper _output;

    public E2ETest(ITestOutputHelper output)
    {
        _output = output;
        var baseUrl = Environment.GetEnvironmentVariable("EDGEQUAKE_BASE_URL") ?? "http://localhost:8080";
        // WHY: Default tenant/user IDs from database migration — avoids Skip.
        var tenantId = Environment.GetEnvironmentVariable("EDGEQUAKE_TENANT_ID") ?? "00000000-0000-0000-0000-000000000002";
        var userId = Environment.GetEnvironmentVariable("EDGEQUAKE_USER_ID") ?? "00000000-0000-0000-0000-000000000001";
        _client = new EdgeQuakeClient(new EdgeQuakeConfig
        {
            BaseUrl = baseUrl,
            TenantId = tenantId,
            UserId = userId,
        });
    }

    // 1. Health
    [Fact]
    public async Task Test01_HealthCheck()
    {
        var h = await _client.Health.CheckAsync();
        Assert.Equal("healthy", h.Status);
        Assert.NotNull(h.Version);
    }

    // 2. Documents
    [Fact]
    public async Task Test02_DocumentsListAndUpload()
    {
        var list = await _client.Documents.ListAsync();
        Assert.NotNull(list.Documents);
        Assert.NotNull(list.Total);

        var resp = await _client.Documents.UploadTextAsync(
            $"CSharp SDK Test {Guid.NewGuid().ToString()[..8]}",
            "C# SDK integration test. Knowledge graphs are powerful."
        );
        Assert.NotNull(resp.DocumentId);
        Assert.NotNull(resp.Status);
    }

    // 3. Graph
    [Fact]
    public async Task Test03_GraphGet()
    {
        var g = await _client.Graph.GetAsync();
        Assert.NotNull(g);
    }

    [Fact]
    public async Task Test04_GraphSearch()
    {
        var r = await _client.Graph.SearchAsync("test");
        Assert.NotNull(r);
    }

    // 4. Entity CRUD
    [Fact]
    public async Task Test05_EntityCrud()
    {
        var name = $"CSHARP_TEST_ENTITY_{Guid.NewGuid().ToString()[..6].ToUpper()}";

        var created = await _client.Entities.CreateAsync(name, "TEST", "Created by C# E2E", "csharp-e2e");
        Assert.NotNull(created.Status);

        var list = await _client.Entities.ListAsync();
        Assert.NotNull(list.Items);

        var fetched = await _client.Entities.GetAsync(name);
        Assert.NotNull(fetched);

        var del = await _client.Entities.DeleteAsync(name);
        Assert.NotNull(del.Status);
    }

    // 5. Relationships
    [Fact]
    public async Task Test06_RelationshipsList()
    {
        var list = await _client.Relationships.ListAsync();
        Assert.NotNull(list.Items);
    }

    // 6. Query
    [Fact]
    public async Task Test07_Query()
    {
        var r = await _client.Query.ExecuteAsync("What is a knowledge graph?");
        Assert.NotNull(r.Answer);
    }

    // 7. Chat
    [Fact]
    public async Task Test08_Chat()
    {
        try
        {
            var r = await _client.Chat.CompletionsAsync("What entities exist?");
            Assert.NotNull(r.Content);
        }
        catch (EdgeQuakeException e) when (e.StatusCode is 401 or 403)
        {
            // Chat may require auth — acceptable
        }
    }

    // 8. Tenants
    [Fact]
    public async Task Test09_TenantsList()
    {
        var list = await _client.Tenants.ListAsync();
        Assert.NotNull(list.Items);
    }

    // 9. Users
    [Fact]
    public async Task Test10_UsersList()
    {
        var list = await _client.Users.ListAsync();
        Assert.NotNull(list.Users);
    }

    // 10. API Keys
    [Fact]
    public async Task Test11_ApiKeysList()
    {
        var list = await _client.ApiKeys.ListAsync();
        Assert.NotNull(list.Keys);
    }

    // 11. Tasks
    [Fact]
    public async Task Test12_TasksList()
    {
        var list = await _client.Tasks.ListAsync();
        Assert.NotNull(list.Tasks);
    }

    // 12. Pipeline Status
    [Fact]
    public async Task Test13_PipelineStatus()
    {
        var st = await _client.Pipeline.StatusAsync();
        Assert.NotNull(st.IsBusy);
    }

    // 13. Queue Metrics
    [Fact]
    public async Task Test14_QueueMetrics()
    {
        var m = await _client.Pipeline.QueueMetricsAsync();
        Assert.NotNull(m.PendingCount);
        Assert.NotNull(m.ActiveWorkers);
    }

    // 14. Models Catalog
    [Fact]
    public async Task Test15_ModelsCatalog()
    {
        var cat = await _client.Models.CatalogAsync();
        Assert.NotNull(cat.Providers);
    }

    // 15. Models Health
    [Fact]
    public async Task Test16_ModelsHealth()
    {
        var items = await _client.Models.HealthAsync();
        Assert.NotEmpty(items);
    }

    // 16. Provider Status
    [Fact]
    public async Task Test17_ProviderStatus()
    {
        var ps = await _client.Models.ProviderStatusAsync();
        Assert.NotNull(ps.Provider);
    }

    // 17. Conversations CRUD
    [Fact]
    public async Task Test18_ConversationsCRUD()
    {
        // Create
        var conv = await _client.Conversations.CreateAsync($"CSharp E2E Test {Guid.NewGuid().ToString()[..8]}");
        Assert.NotNull(conv.Id);
        _output.WriteLine($"Created conversation: {conv.Id} title={conv.Title}");

        // List
        var convos = await _client.Conversations.ListAsync();
        Assert.NotEmpty(convos);
        _output.WriteLine($"Conversations: {convos.Count}");

        // Get detail
        var detail = await _client.Conversations.GetAsync(conv.Id!);
        Assert.NotNull(detail.Conversation);
        Assert.Equal(conv.Id, detail.Id);

        // Delete (204 No Content)
        await _client.Conversations.DeleteAsync(conv.Id!);
    }

    // 18. Folders CRUD
    [Fact]
    public async Task Test19_FoldersCRUD()
    {
        // Create
        var folder = await _client.Folders.CreateAsync($"CSharp E2E Folder {Guid.NewGuid().ToString()[..8]}");
        Assert.NotNull(folder.Id);
        _output.WriteLine($"Created folder: {folder.Id} name={folder.Name}");

        // List
        var folders = await _client.Folders.ListAsync();
        Assert.NotEmpty(folders);
        _output.WriteLine($"Folders: {folders.Count}");

        // Delete (204 No Content)
        await _client.Folders.DeleteAsync(folder.Id!);
    }

    // 19. Costs
    [Fact]
    public async Task Test20_CostsSummary()
    {
        var c = await _client.Costs.SummaryAsync();
        Assert.NotNull(c);
    }

    // 20. Full Workflow
    [Fact]
    public async Task Test21_FullWorkflow()
    {
        var doc = await _client.Documents.UploadTextAsync(
            $"CSharp Workflow {Guid.NewGuid().ToString()[..8]}",
            "Knowledge graphs connect entities through relationships."
        );
        Assert.NotNull(doc.DocumentId);

        var qr = await _client.Query.ExecuteAsync("What do knowledge graphs connect?");
        Assert.NotNull(qr.Answer);

        var ps = await _client.Pipeline.StatusAsync();
        Assert.NotNull(ps.IsBusy);
    }
}
