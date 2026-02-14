using System.Net;
using System.Text.Json;
using Xunit;

namespace EdgeQuakeSDK.Tests;

/// <summary>
/// Lineage, metadata, and provenance tests for C# SDK models and services.
/// WHY: Validates rich JSON deserialization of all model fields beyond basic endpoint mapping.
/// OODA-18: Iteration 18 — C# SDK lineage coverage expansion.
/// </summary>
public class LineageTest
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

    // ── HealthResponse Full Fields ─────────────────────────────────

    [Fact]
    public async Task Health_AllFields_Deserialized()
    {
        var json = @"{
            ""status"": ""healthy"",
            ""version"": ""0.2.0"",
            ""storage_mode"": ""postgresql"",
            ""workspace_id"": ""ws-main"",
            ""components"": {""kv_storage"":true,""vector_storage"":true,""graph_storage"":false,""llm_provider"":true},
            ""llm_provider_name"": ""ollama""
        }";
        var http = MockHelper(json);
        var result = await new HealthService(http).CheckAsync();
        Assert.Equal("healthy", result.Status);
        Assert.Equal("0.2.0", result.Version);
        Assert.Equal("postgresql", result.StorageMode);
        Assert.Equal("ws-main", result.WorkspaceId);
        Assert.Equal("ollama", result.LlmProviderName);
        Assert.NotNull(result.Components);
        Assert.Equal(4, result.Components!.Count);
        Assert.True(result.Components["kv_storage"]);
        Assert.False(result.Components["graph_storage"]);
    }

    [Fact]
    public async Task Health_NullableFieldsDefaultToNull()
    {
        var http = MockHelper(@"{""status"":""healthy""}");
        var result = await new HealthService(http).CheckAsync();
        Assert.Equal("healthy", result.Status);
        Assert.Null(result.Version);
        Assert.Null(result.StorageMode);
        Assert.Null(result.WorkspaceId);
        Assert.Null(result.Components);
        Assert.Null(result.LlmProviderName);
    }

    // ── EntityDetailResponse Rich Deserialization ──────────────────

    [Fact]
    public async Task EntityDetail_FullDeserialization()
    {
        var json = @"{
            ""entity"": {
                ""entity_name"": ""SARAH_CHEN"",
                ""entity_type"": ""PERSON"",
                ""description"": ""Lead researcher"",
                ""source_id"": ""doc-1"",
                ""metadata"": {""department"": ""AI"", ""level"": 5}
            },
            ""relationships"": [
                {""source"":""SARAH_CHEN"",""target"":""MIT"",""relationship"":""AFFILIATED_WITH"",""weight"":0.95}
            ],
            ""statistics"": {""degree"":3,""in_degree"":1,""out_degree"":2,""centrality"":0.78}
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var svc = new EntityService(http);
        var result = await svc.GetAsync("SARAH_CHEN");

        // Entity
        Assert.NotNull(result.Entity);
        var entity = result.Entity!.Value;
        Assert.Equal("SARAH_CHEN", entity.GetProperty("entity_name").GetString());
        Assert.Equal("PERSON", entity.GetProperty("entity_type").GetString());
        Assert.Equal("doc-1", entity.GetProperty("source_id").GetString());

        // Metadata nested access
        var meta = entity.GetProperty("metadata");
        Assert.Equal("AI", meta.GetProperty("department").GetString());
        Assert.Equal(5, meta.GetProperty("level").GetInt32());

        // Relationships
        Assert.NotNull(result.Relationships);
        var rels = result.Relationships!.Value;
        Assert.Equal(JsonValueKind.Array, rels.ValueKind);
        Assert.Equal(1, rels.GetArrayLength());
        var rel = rels[0];
        Assert.Equal("SARAH_CHEN", rel.GetProperty("source").GetString());
        Assert.Equal("MIT", rel.GetProperty("target").GetString());
        Assert.Equal(0.95, rel.GetProperty("weight").GetDouble(), 2);

        // Statistics
        Assert.NotNull(result.Statistics);
        var stats = result.Statistics!.Value;
        Assert.Equal(3, stats.GetProperty("degree").GetInt32());
        Assert.Equal(0.78, stats.GetProperty("centrality").GetDouble(), 2);
    }

    [Fact]
    public async Task EntityDetail_EmptyRelationships()
    {
        var json = @"{""entity"":{""entity_name"":""LONE_NODE""},""relationships"":[],""statistics"":{""degree"":0}}";
        var http = MockHelper(json);
        var result = await new EntityService(http).GetAsync("LONE_NODE");
        Assert.Equal(0, result.Relationships!.Value.GetArrayLength());
        Assert.Equal(0, result.Statistics!.Value.GetProperty("degree").GetInt32());
    }

    // ── EntityDeleteResponse Fields ────────────────────────────────

    [Fact]
    public async Task EntityDelete_AllFields()
    {
        var json = @"{
            ""status"": ""deleted"",
            ""message"": ""Entity removed"",
            ""deleted_entity_id"": ""ent-42"",
            ""deleted_relationships"": 5,
            ""affected_entities"": [""ALICE"", ""BOB"", ""CAROL""]
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new EntityService(http).DeleteAsync("ent-42");
        Assert.Equal("deleted", result.Status);
        Assert.Equal("Entity removed", result.Message);
        Assert.Equal("ent-42", result.DeletedEntityId);
        Assert.Equal(5, result.DeletedRelationships);
        Assert.NotNull(result.AffectedEntities);
        Assert.Equal(3, result.AffectedEntities!.Count);
        Assert.Contains("BOB", result.AffectedEntities);
    }

    [Fact]
    public async Task EntityDelete_ZeroRelationships()
    {
        var json = @"{""status"":""deleted"",""deleted_entity_id"":""x"",""deleted_relationships"":0,""affected_entities"":[]}";
        var http = MockHelper(json);
        var result = await new EntityService(http).DeleteAsync("x");
        Assert.Equal(0, result.DeletedRelationships);
        Assert.Empty(result.AffectedEntities!);
    }

    // ── CreateEntityResponse Fields ────────────────────────────────

    [Fact]
    public async Task EntityCreate_ResponseWithNestedEntity()
    {
        var json = @"{
            ""status"": ""created"",
            ""message"": ""Entity ALICE created successfully"",
            ""entity"": {
                ""entity_name"": ""ALICE"",
                ""entity_type"": ""PERSON"",
                ""description"": ""A researcher"",
                ""source_id"": ""doc-99"",
                ""created_at"": ""2025-01-15T10:00:00Z"",
                ""metadata"": {""confidence"": 0.92, ""extraction_method"": ""llm""}
            }
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new EntityService(http).CreateAsync("ALICE", "PERSON", "A researcher", "doc-99");
        Assert.Equal("created", result.Status);
        Assert.Contains("ALICE", result.Message!);

        var entity = result.Entity!.Value;
        Assert.Equal("ALICE", entity.GetProperty("entity_name").GetString());
        Assert.Equal("doc-99", entity.GetProperty("source_id").GetString());
        Assert.Equal("2025-01-15T10:00:00Z", entity.GetProperty("created_at").GetString());
        Assert.Equal(0.92, entity.GetProperty("metadata").GetProperty("confidence").GetDouble(), 2);
    }

    [Fact]
    public async Task EntityCreate_RequestBodyContainsAllFields()
    {
        var (http, mock) = MockHelperWithCalls(@"{""status"":""created""}");
        await new EntityService(http).CreateAsync("BOB", "ORGANIZATION", "A company", "src-x");
        var body = mock.LastCall!.Body!;
        Assert.Contains("BOB", body);
        Assert.Contains("ORGANIZATION", body);
        Assert.Contains("A company", body);
        Assert.Contains("src-x", body);
    }

    // ── GraphResponse With Lineage Data ────────────────────────────

    [Fact]
    public async Task Graph_WithRichNodes()
    {
        var json = @"{
            ""nodes"": [
                {""id"":""n1"",""entity_name"":""ALICE"",""entity_type"":""PERSON"",""description"":""Doctor"",""source_id"":""d1"",""metadata"":{""degree"":5}},
                {""id"":""n2"",""entity_name"":""MIT"",""entity_type"":""ORGANIZATION"",""description"":""University"",""source_id"":""d2"",""metadata"":{""degree"":12}}
            ],
            ""edges"": [
                {""source"":""ALICE"",""target"":""MIT"",""relationship"":""WORKS_AT"",""weight"":0.88,""source_id"":""d1"",""metadata"":{""extraction_method"":""llm""}}
            ]
        }";
        var http = MockHelper(json);
        var result = await new GraphService(http).GetAsync();
        Assert.NotNull(result.Nodes);
        Assert.Equal(2, result.Nodes!.Count);

        var node1 = result.Nodes[0];
        Assert.Equal("ALICE", node1.GetProperty("entity_name").GetString());
        Assert.Equal(5, node1.GetProperty("metadata").GetProperty("degree").GetInt32());

        Assert.NotNull(result.Edges);
        Assert.Single(result.Edges!);
        var edge = result.Edges[0];
        Assert.Equal("WORKS_AT", edge.GetProperty("relationship").GetString());
        Assert.Equal(0.88, edge.GetProperty("weight").GetDouble(), 2);
        Assert.Equal("llm", edge.GetProperty("metadata").GetProperty("extraction_method").GetString());
    }

    [Fact]
    public async Task Graph_EmptyGraphResponse()
    {
        var http = MockHelper(@"{""nodes"":[],""edges"":[]}");
        var result = await new GraphService(http).GetAsync();
        Assert.Empty(result.Nodes!);
        Assert.Empty(result.Edges!);
    }

    // ── DocumentListResponse Pagination ────────────────────────────

    [Fact]
    public async Task DocumentList_FullPagination()
    {
        var json = @"{
            ""documents"": [{""id"":""d1""},{""id"":""d2""}],
            ""items"": [{""id"":""d1""},{""id"":""d2""}],
            ""total"": 50,
            ""page"": 2,
            ""page_size"": 2,
            ""total_pages"": 25,
            ""has_more"": true
        }";
        var http = MockHelper(json);
        var result = await new DocumentService(http).ListAsync(2, 2);
        Assert.NotNull(result.Documents);
        Assert.Equal(2, result.Documents!.Count);
        Assert.Equal(50, result.Total);
        Assert.Equal(2, result.Page);
        Assert.Equal(2, result.PageSize);
        Assert.Equal(25, result.TotalPages);
        Assert.True(result.HasMore);
    }

    [Fact]
    public async Task DocumentList_LastPage()
    {
        var json = @"{""documents"":[{""id"":""d50""}],""total"":50,""page"":25,""page_size"":2,""total_pages"":25,""has_more"":false}";
        var http = MockHelper(json);
        var result = await new DocumentService(http).ListAsync(25, 2);
        Assert.False(result.HasMore);
        Assert.Equal(25, result.Page);
    }

    // ── UploadResponse Edge Cases ──────────────────────────────────

    [Fact]
    public async Task Upload_DuplicateDetection()
    {
        var json = @"{
            ""document_id"": ""d-dup"",
            ""status"": ""duplicate"",
            ""track_id"": ""trk-99"",
            ""duplicate_of"": ""d-original""
        }";
        var http = MockHelper(json);
        var result = await new DocumentService(http).UploadTextAsync("Dup Doc", "Content");
        Assert.Equal("duplicate", result.Status);
        Assert.Equal("d-original", result.DuplicateOf);
        Assert.Equal("trk-99", result.TrackId);
    }

    [Fact]
    public async Task Upload_ProcessingStatus()
    {
        var json = @"{""document_id"":""d-new"",""status"":""processing"",""track_id"":""trk-1""}";
        var http = MockHelper(json);
        var result = await new DocumentService(http).UploadTextAsync("New", "Text");
        Assert.Equal("processing", result.Status);
        Assert.Null(result.DuplicateOf);
    }

    // ── PipelineStatusResponse Full Fields ─────────────────────────

    [Fact]
    public async Task Pipeline_AllFieldsDeserialized()
    {
        var json = @"{
            ""is_busy"": true,
            ""total_documents"": 100,
            ""processed_documents"": 75,
            ""pending_tasks"": 10,
            ""processing_tasks"": 5,
            ""completed_tasks"": 80,
            ""failed_tasks"": 5
        }";
        var http = MockHelper(json);
        var result = await new PipelineService(http).StatusAsync();
        Assert.True(result.IsBusy);
        Assert.Equal(100, result.TotalDocuments);
        Assert.Equal(75, result.ProcessedDocuments);
        Assert.Equal(10, result.PendingTasks);
        Assert.Equal(5, result.ProcessingTasks);
        Assert.Equal(80, result.CompletedTasks);
        Assert.Equal(5, result.FailedTasks);
    }

    [Fact]
    public async Task Pipeline_IdleState()
    {
        var json = @"{""is_busy"":false,""total_documents"":100,""processed_documents"":100,""pending_tasks"":0,""processing_tasks"":0,""completed_tasks"":100,""failed_tasks"":0}";
        var http = MockHelper(json);
        var result = await new PipelineService(http).StatusAsync();
        Assert.False(result.IsBusy);
        Assert.Equal(result.TotalDocuments, result.ProcessedDocuments);
        Assert.Equal(0, result.PendingTasks);
    }

    // ── QueueMetricsResponse Full Fields ───────────────────────────

    [Fact]
    public async Task QueueMetrics_AllFields()
    {
        var json = @"{
            ""pending_count"": 15,
            ""processing_count"": 3,
            ""active_workers"": 3,
            ""max_workers"": 8,
            ""worker_utilization"": 0.375,
            ""avg_wait_time_seconds"": 2.5,
            ""throughput_per_minute"": 12.8,
            ""rate_limited"": false
        }";
        var http = MockHelper(json);
        var result = await new PipelineService(http).QueueMetricsAsync();
        Assert.Equal(15, result.PendingCount);
        Assert.Equal(3, result.ProcessingCount);
        Assert.Equal(3, result.ActiveWorkers);
        Assert.Equal(8, result.MaxWorkers);
        Assert.Equal(0.375, result.WorkerUtilization!.Value, 3);
        Assert.Equal(2.5, result.AvgWaitTimeSeconds!.Value, 1);
        Assert.Equal(12.8, result.ThroughputPerMinute!.Value, 1);
        Assert.False(result.RateLimited);
    }

    [Fact]
    public async Task QueueMetrics_RateLimited()
    {
        var json = @"{""pending_count"":50,""processing_count"":8,""active_workers"":8,""max_workers"":8,""worker_utilization"":1.0,""rate_limited"":true}";
        var http = MockHelper(json);
        var result = await new PipelineService(http).QueueMetricsAsync();
        Assert.True(result.RateLimited);
        Assert.Equal(1.0, result.WorkerUtilization!.Value, 1);
    }

    // ── ChatCompletionResponse Full Fields ─────────────────────────

    [Fact]
    public async Task Chat_AllFieldsDeserialized()
    {
        var json = @"{
            ""conversation_id"": ""conv-abc"",
            ""user_message_id"": ""msg-u1"",
            ""assistant_message_id"": ""msg-a1"",
            ""content"": ""The answer is 42"",
            ""mode"": ""hybrid"",
            ""sources"": [{""entity"":""DEEP_THOUGHT"",""confidence"":0.99}],
            ""tokens_used"": 150,
            ""duration_ms"": 1200
        }";
        var http = MockHelper(json);
        var result = await new ChatService(http).CompletionsAsync("What is the answer?");
        Assert.Equal("conv-abc", result.ConversationId);
        Assert.Equal("msg-u1", result.UserMessageId);
        Assert.Equal("msg-a1", result.AssistantMessageId);
        Assert.Equal("The answer is 42", result.Content);
        Assert.Equal("hybrid", result.Mode);
        Assert.NotNull(result.Sources);
        Assert.Single(result.Sources!);
        Assert.Equal(150, result.TokensUsed);
        Assert.Equal(1200, result.DurationMs);

        // Source entity lineage
        var source = result.Sources![0];
        Assert.Equal("DEEP_THOUGHT", source.GetProperty("entity").GetString());
        Assert.Equal(0.99, source.GetProperty("confidence").GetDouble(), 2);
    }

    [Fact]
    public async Task Chat_NoSources()
    {
        var json = @"{""content"":""I don't know"",""sources"":[],""tokens_used"":10}";
        var http = MockHelper(json);
        var result = await new ChatService(http).CompletionsAsync("test");
        Assert.Empty(result.Sources!);
        Assert.Equal(10, result.TokensUsed);
    }

    // ── QueryResponse With Sources ─────────────────────────────────

    [Fact]
    public async Task Query_SourcesWithLineage()
    {
        var json = @"{
            ""answer"": ""Paris is the capital"",
            ""sources"": [
                {""entity"":""PARIS"",""type"":""CITY"",""source_document"":""doc-geo"",""confidence"":0.95},
                {""entity"":""FRANCE"",""type"":""COUNTRY"",""source_document"":""doc-geo"",""confidence"":0.88}
            ],
            ""mode"": ""global""
        }";
        var http = MockHelper(json);
        var result = await new QueryService(http).ExecuteAsync("What is the capital of France?", "global");
        Assert.Equal("Paris is the capital", result.Answer);
        Assert.Equal("global", result.Mode);
        Assert.Equal(2, result.Sources!.Count);
        Assert.Equal("PARIS", result.Sources[0].GetProperty("entity").GetString());
        Assert.Equal("doc-geo", result.Sources[1].GetProperty("source_document").GetString());
    }

    // ── CostSummary Full Fields ────────────────────────────────────

    [Fact]
    public async Task Costs_AllFields()
    {
        var json = @"{
            ""total_cost"": 45.67,
            ""document_count"": 120,
            ""query_count"": 350,
            ""entries"": [
                {""date"":""2025-01-15"",""cost"":12.5,""type"":""extraction""},
                {""date"":""2025-01-15"",""cost"":33.17,""type"":""query""}
            ]
        }";
        var http = MockHelper(json);
        var result = await new CostService(http).SummaryAsync();
        Assert.Equal(45.67, result.TotalCost);
        Assert.Equal(120, result.DocumentCount);
        Assert.Equal(350, result.QueryCount);
        Assert.NotNull(result.Entries);
        Assert.Equal(2, result.Entries!.Count);
        Assert.Equal("extraction", result.Entries[0].GetProperty("type").GetString());
    }

    [Fact]
    public async Task Costs_ZeroCost()
    {
        var json = @"{""total_cost"":0.0,""document_count"":0,""query_count"":0,""entries"":[]}";
        var http = MockHelper(json);
        var result = await new CostService(http).SummaryAsync();
        Assert.Equal(0.0, result.TotalCost);
        Assert.Equal(0, result.DocumentCount);
        Assert.Empty(result.Entries!);
    }

    // ── ConversationDetail With Messages ───────────────────────────

    [Fact]
    public async Task ConversationDetail_FullDeserialization()
    {
        var json = @"{
            ""conversation"": {
                ""id"": ""conv-1"",
                ""tenant_id"": ""t-main"",
                ""workspace_id"": ""ws-1"",
                ""title"": ""Research Chat"",
                ""mode"": ""hybrid"",
                ""is_pinned"": true,
                ""folder_id"": ""f-research"",
                ""created_at"": ""2025-01-10T08:00:00Z"",
                ""updated_at"": ""2025-01-15T16:30:00Z"",
                ""message_count"": 5
            },
            ""messages"": [
                {""id"":""m1"",""conversation_id"":""conv-1"",""parent_id"":null,""role"":""user"",""content"":""Hello"",""mode"":""hybrid"",""tokens_used"":3,""created_at"":""2025-01-10T08:01:00Z""},
                {""id"":""m2"",""conversation_id"":""conv-1"",""parent_id"":""m1"",""role"":""assistant"",""content"":""Hi!"",""mode"":""hybrid"",""tokens_used"":50,""created_at"":""2025-01-10T08:01:05Z""}
            ]
        }";
        var http = MockHelper(json);
        var result = await new ConversationService(http).GetAsync("conv-1");

        // Conversation metadata
        Assert.Equal("conv-1", result.Id);
        Assert.NotNull(result.Conversation);
        Assert.Equal("t-main", result.Conversation!.TenantId);
        Assert.Equal("ws-1", result.Conversation.WorkspaceId);
        Assert.Equal("Research Chat", result.Conversation.Title);
        Assert.Equal("hybrid", result.Conversation.Mode);
        Assert.True(result.Conversation.IsPinned);
        Assert.Equal("f-research", result.Conversation.FolderId);
        Assert.Equal("2025-01-10T08:00:00Z", result.Conversation.CreatedAt);
        Assert.Equal("2025-01-15T16:30:00Z", result.Conversation.UpdatedAt);
        Assert.Equal(5, result.Conversation.MessageCount);

        // Messages
        Assert.NotNull(result.Messages);
        Assert.Equal(2, result.Messages!.Count);
        var msg1 = result.Messages[0];
        Assert.Equal("m1", msg1.Id);
        Assert.Equal("user", msg1.Role);
        Assert.Equal("Hello", msg1.Content);
        Assert.Equal(3, msg1.TokensUsed);
        Assert.Null(msg1.ParentId);

        var msg2 = result.Messages[1];
        Assert.Equal("m1", msg2.ParentId);
        Assert.Equal("assistant", msg2.Role);
        Assert.Equal(50, msg2.TokensUsed);
    }

    [Fact]
    public async Task ConversationInfo_UnpinnedDefaults()
    {
        var json = @"{""id"":""c-simple"",""title"":""Quick Chat"",""is_pinned"":false}";
        var http = MockHelper(json);
        var result = await new ConversationService(http).CreateAsync("Quick Chat");
        Assert.False(result.IsPinned);
        Assert.Null(result.FolderId);
        Assert.Null(result.TenantId);
    }

    // ── ProviderHealthInfo Full Fields ──────────────────────────────

    [Fact]
    public async Task ProviderHealth_AllFields()
    {
        var json = @"[{
            ""name"": ""ollama"",
            ""display_name"": ""Ollama Local"",
            ""provider_type"": ""ollama"",
            ""enabled"": true,
            ""priority"": 1,
            ""models"": [
                {""name"":""gemma3:latest"",""size"":""4B"",""quantization"":""q4_0""},
                {""name"":""qwen2.5:latest"",""size"":""7B"",""quantization"":""q4_K_M""}
            ]
        }]";
        var http = MockHelper(json);
        var result = await new ModelService(http).HealthAsync();
        Assert.Single(result);
        var provider = result[0];
        Assert.Equal("ollama", provider.Name);
        Assert.Equal("Ollama Local", provider.DisplayName);
        Assert.Equal("ollama", provider.ProviderType);
        Assert.True(provider.Enabled);
        Assert.Equal(1, provider.Priority);
        Assert.NotNull(provider.Models);
        Assert.Equal(2, provider.Models!.Count);
        Assert.Equal("gemma3:latest", provider.Models[0].GetProperty("name").GetString());
    }

    [Fact]
    public async Task ProviderHealth_DisabledProvider()
    {
        var json = @"[{""name"":""openai"",""enabled"":false,""priority"":2,""models"":[]}]";
        var http = MockHelper(json);
        var result = await new ModelService(http).HealthAsync();
        Assert.False(result[0].Enabled);
        Assert.Empty(result[0].Models!);
    }

    // ── ProviderStatus Nested Fields ───────────────────────────────

    [Fact]
    public async Task ProviderStatus_AllSections()
    {
        var json = @"{
            ""provider"": {""name"":""ollama"",""status"":""connected"",""model"":""gemma3:latest""},
            ""embedding"": {""provider"":""ollama"",""model"":""nomic-embed-text"",""dimension"":768},
            ""storage"": {""type"":""postgresql"",""connected"":true,""version"":""16.2""},
            ""metadata"": {""workspace_id"":""default"",""tenant_id"":""system""}
        }";
        var http = MockHelper(json);
        var result = await new ModelService(http).ProviderStatusAsync();

        Assert.NotNull(result.Provider);
        Assert.Equal("ollama", result.Provider!.Value.GetProperty("name").GetString());
        Assert.Equal("connected", result.Provider!.Value.GetProperty("status").GetString());

        Assert.NotNull(result.Embedding);
        Assert.Equal(768, result.Embedding!.Value.GetProperty("dimension").GetInt32());

        Assert.NotNull(result.Storage);
        Assert.True(result.Storage!.Value.GetProperty("connected").GetBoolean());

        Assert.NotNull(result.Metadata);
        Assert.Equal("default", result.Metadata!.Value.GetProperty("workspace_id").GetString());
    }

    // ── EntityListResponse Pagination ──────────────────────────────

    [Fact]
    public async Task EntityList_FullPagination()
    {
        var json = @"{
            ""items"": [{""entity_name"":""A""},{""entity_name"":""B""},{""entity_name"":""C""}],
            ""total"": 100,
            ""page"": 5,
            ""page_size"": 3,
            ""total_pages"": 34
        }";
        var http = MockHelper(json);
        var result = await new EntityService(http).ListAsync(5, 3);
        Assert.Equal(3, result.Items!.Count);
        Assert.Equal(100, result.Total);
        Assert.Equal(5, result.Page);
        Assert.Equal(3, result.PageSize);
        Assert.Equal(34, result.TotalPages);
    }

    // ── RelationshipListResponse ───────────────────────────────────

    [Fact]
    public async Task RelationshipList_WithMetadata()
    {
        var json = @"{
            ""items"": [
                {""source"":""ALICE"",""target"":""BOB"",""relationship"":""KNOWS"",""weight"":0.75,""source_id"":""doc-1"",""metadata"":{""context"":""meeting""}},
                {""source"":""BOB"",""target"":""CAROL"",""relationship"":""MANAGES"",""weight"":0.9,""source_id"":""doc-2"",""metadata"":{""since"":""2024""}}
            ],
            ""total"": 2
        }";
        var http = MockHelper(json);
        var result = await new RelationshipService(http).ListAsync();
        Assert.Equal(2, result.Items!.Count);
        Assert.Equal(2, result.Total);
        var rel0 = result.Items[0];
        Assert.Equal("ALICE", rel0.GetProperty("source").GetString());
        Assert.Equal("meeting", rel0.GetProperty("metadata").GetProperty("context").GetString());
    }

    // ── SearchResponse With Lineage ────────────────────────────────

    [Fact]
    public async Task Search_ResultsWithLineage()
    {
        var json = @"{
            ""results"": [
                {""id"":""n1"",""entity_name"":""QUANTUM"",""entity_type"":""CONCEPT"",""score"":0.95,""source_documents"":[""doc-1"",""doc-2""]},
                {""id"":""n2"",""entity_name"":""COMPUTING"",""entity_type"":""CONCEPT"",""score"":0.82,""source_documents"":[""doc-1""]}
            ]
        }";
        var http = MockHelper(json);
        var result = await new GraphService(http).SearchAsync("quantum");
        Assert.Equal(2, result.Results!.Count);
        Assert.Equal(0.95, result.Results[0].GetProperty("score").GetDouble(), 2);
        Assert.Equal(2, result.Results[0].GetProperty("source_documents").GetArrayLength());
    }

    // ── Edge Cases ─────────────────────────────────────────────────

    [Fact]
    public async Task NullableFieldsRobustness()
    {
        // All nullable fields should survive empty JSON
        var http = MockHelper(@"{}");

        var entity = await new EntityService(http).GetAsync("x");
        Assert.Null(entity.Entity);
        Assert.Null(entity.Relationships);
        Assert.Null(entity.Statistics);

        var pipeline = await new PipelineService(MockHelper(@"{}")).StatusAsync();
        Assert.Null(pipeline.IsBusy);
        Assert.Null(pipeline.TotalDocuments);

        var queue = await new PipelineService(MockHelper(@"{}")).QueueMetricsAsync();
        Assert.Null(queue.PendingCount);
        Assert.Null(queue.RateLimited);
    }

    [Fact]
    public async Task LargeGraph_ManyNodes()
    {
        // Generate 50-node graph JSON
        var nodes = string.Join(",", Enumerable.Range(0, 50).Select(i =>
            $@"{{""id"":""n{i}"",""entity_name"":""ENTITY_{i}"",""entity_type"":""TYPE""}}"));
        var edges = string.Join(",", Enumerable.Range(0, 49).Select(i =>
            $@"{{""source"":""ENTITY_{i}"",""target"":""ENTITY_{i + 1}"",""relationship"":""NEXT""}}"));
        var json = $@"{{""nodes"":[{nodes}],""edges"":[{edges}]}}";

        var http = MockHelper(json);
        var result = await new GraphService(http).GetAsync();
        Assert.Equal(50, result.Nodes!.Count);
        Assert.Equal(49, result.Edges!.Count);
    }

    [Fact]
    public async Task BulkDelete_AllFields()
    {
        var json = @"{""deleted"":10,""status"":""ok""}";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new ConversationService(http).BulkDeleteAsync(
            new List<string> { "c1", "c2", "c3", "c4", "c5" });
        Assert.Equal(10, result.Deleted);
        Assert.Equal("ok", result.Status);
        Assert.Contains("c1", mock.LastCall!.Body!);
        Assert.Contains("c5", mock.LastCall.Body!);
    }

    [Fact]
    public async Task FolderInfo_AllFields()
    {
        var json = @"[{
            ""id"": ""f-1"",
            ""tenant_id"": ""t-main"",
            ""name"": ""Research Papers"",
            ""created_at"": ""2025-01-01T00:00:00Z"",
            ""updated_at"": ""2025-01-15T12:00:00Z""
        }]";
        var http = MockHelper(json);
        var result = await new FolderService(http).ListAsync();
        Assert.Single(result);
        Assert.Equal("f-1", result[0].Id);
        Assert.Equal("t-main", result[0].TenantId);
        Assert.Equal("Research Papers", result[0].Name);
        Assert.Equal("2025-01-01T00:00:00Z", result[0].CreatedAt);
        Assert.Equal("2025-01-15T12:00:00Z", result[0].UpdatedAt);
    }

    [Fact]
    public async Task TaskList_WithItems()
    {
        var json = @"{
            ""tasks"": [{""track_id"":""trk-1"",""status"":""completed"",""document_id"":""d1""}],
            ""items"": [{""track_id"":""trk-1"",""status"":""completed""}]
        }";
        var http = MockHelper(json);
        var result = await new TaskService(http).ListAsync();
        Assert.NotNull(result.Tasks);
        Assert.Single(result.Tasks!);
        Assert.NotNull(result.Items);
        Assert.Single(result.Items!);
        Assert.Equal("completed", result.Tasks[0].GetProperty("status").GetString());
    }

    // ── Config Edge Cases ──────────────────────────────────────────

    [Fact]
    public void Config_TrailingSlashHandled()
    {
        var c = new EdgeQuakeConfig { BaseUrl = "http://localhost:8080/" };
        // Should not throw, just store the value
        Assert.EndsWith("/", c.BaseUrl);
    }

    [Fact]
    public void Config_AllFieldsSettable()
    {
        var c = new EdgeQuakeConfig
        {
            BaseUrl = "https://edge.example.com",
            ApiKey = "key-123",
            TenantId = "tenant-x",
            UserId = "user-y",
            WorkspaceId = "workspace-z",
            TimeoutSeconds = 300,
        };
        Assert.Equal("https://edge.example.com", c.BaseUrl);
        Assert.Equal("key-123", c.ApiKey);
        Assert.Equal("tenant-x", c.TenantId);
        Assert.Equal("user-y", c.UserId);
        Assert.Equal("workspace-z", c.WorkspaceId);
        Assert.Equal(300, c.TimeoutSeconds);
    }

    // ── Client Service Count ───────────────────────────────────────

    [Fact]
    public void Client_Has20Services()
    {
        var client = new EdgeQuakeClient();
        var props = typeof(EdgeQuakeClient).GetProperties();
        // 20 services: Health, Documents, Entities, Relationships, Graph, Query,
        // Chat, Tenants, Users, ApiKeys, Tasks, Pipeline, Models, Costs,
        // Conversations, Folders, Lineage, Auth, Workspaces, Shared
        // OODA-36: Added Auth, Workspaces, Shared services
        Assert.Equal(20, props.Length);
    }

    // ── LineageService Endpoint Tests (OODA-25) ────────────────────

    [Fact]
    public async Task EntityLineage_AllFields()
    {
        var json = @"{
            ""entity_name"": ""SARAH_CHEN"",
            ""entity_type"": ""PERSON"",
            ""source_documents"": [{
                ""document_id"": ""doc-1"",
                ""chunk_ids"": [""c-a"", ""c-b""],
                ""line_ranges"": [{""start_line"": 10, ""end_line"": 15}]
            }],
            ""source_count"": 1,
            ""description_versions"": [{
                ""version"": 1,
                ""description"": ""Lead researcher"",
                ""source_chunk_id"": ""c-a"",
                ""created_at"": ""2025-01-15T10:00:00Z""
            }]
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new LineageService(http).EntityLineageAsync("SARAH_CHEN");
        Assert.Equal("SARAH_CHEN", result.EntityName);
        Assert.Equal("PERSON", result.EntityType);
        Assert.Single(result.SourceDocuments!);
        Assert.Equal("doc-1", result.SourceDocuments![0].DocumentId);
        Assert.Equal(2, result.SourceDocuments[0].ChunkIds!.Count);
        Assert.Equal(10, result.SourceDocuments[0].LineRanges![0].StartLine);
        Assert.Equal(15, result.SourceDocuments[0].LineRanges![0].EndLine);
        Assert.Equal(1, result.SourceCount);
        Assert.Single(result.DescriptionVersions!);
        Assert.Equal(1, result.DescriptionVersions![0].Version);
        Assert.Equal("Lead researcher", result.DescriptionVersions[0].Description);
        Assert.Equal("c-a", result.DescriptionVersions[0].SourceChunkId);
        Assert.Contains("/lineage/entities/SARAH_CHEN", mock.LastCall!.Url!);
    }

    [Fact]
    public async Task EntityLineage_UrlEncodesSpecialChars()
    {
        var (http, mock) = MockHelperWithCalls(@"{""entity_name"":""A B""}");
        await new LineageService(http).EntityLineageAsync("A B");
        Assert.Contains("A%20B", mock.LastCall!.Url!);
    }

    [Fact]
    public async Task DocumentLineage_AllFields()
    {
        var json = @"{
            ""document_id"": ""doc-42"",
            ""chunk_count"": 5,
            ""entities"": [{
                ""name"": ""ALICE"",
                ""entity_type"": ""PERSON"",
                ""source_chunks"": [""c1"", ""c2""],
                ""is_shared"": true
            }],
            ""relationships"": [{
                ""source"": ""ALICE"",
                ""target"": ""MIT"",
                ""keywords"": ""AFFILIATED_WITH"",
                ""source_chunks"": [""c1""]
            }],
            ""extraction_stats"": {
                ""total_entities"": 20,
                ""unique_entities"": 12,
                ""total_relationships"": 15,
                ""unique_relationships"": 10,
                ""processing_time_ms"": 3500
            }
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new LineageService(http).DocumentLineageAsync("doc-42");
        Assert.Equal("doc-42", result.DocumentId);
        Assert.Equal(5, result.ChunkCount);
        Assert.Single(result.Entities!);
        Assert.Equal("ALICE", result.Entities![0].Name);
        Assert.True(result.Entities[0].IsShared);
        Assert.Equal(2, result.Entities[0].SourceChunks!.Count);
        Assert.Single(result.Relationships!);
        Assert.Equal("MIT", result.Relationships![0].Target);
        Assert.Equal("AFFILIATED_WITH", result.Relationships[0].Keywords);
        Assert.Equal(20, result.ExtractionStats!.TotalEntities);
        Assert.Equal(12, result.ExtractionStats.UniqueEntities);
        Assert.Equal(3500, result.ExtractionStats.ProcessingTimeMs);
        Assert.Contains("/lineage/documents/doc-42", mock.LastCall!.Url!);
    }

    [Fact]
    public async Task DocumentFullLineage_AllFields()
    {
        var json = @"{
            ""document_id"": ""doc-99"",
            ""metadata"": {""title"": ""Research Paper"", ""author"": ""Jane""},
            ""lineage"": {""entities"": 5, ""relationships"": 3}
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new LineageService(http).DocumentFullLineageAsync("doc-99");
        Assert.Equal("doc-99", result.DocumentId);
        Assert.NotNull(result.Metadata);
        Assert.Equal("Research Paper", result.Metadata!.Value.GetProperty("title").GetString());
        Assert.NotNull(result.Lineage);
        Assert.Equal(5, result.Lineage!.Value.GetProperty("entities").GetInt32());
        Assert.Contains("/documents/doc-99/lineage", mock.LastCall!.Url!);
    }

    [Fact]
    public async Task ExportLineage_ReturnsJsonElement()
    {
        var json = @"{""format"": ""json"", ""data"": [{""entity"": ""ALICE""}]}";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new LineageService(http).ExportLineageAsync("doc-1", "json");
        Assert.Equal("json", result.GetProperty("format").GetString());
        Assert.Equal(1, result.GetProperty("data").GetArrayLength());
        Assert.Contains("/lineage/export", mock.LastCall!.Url!);
        Assert.Contains("format=json", mock.LastCall.Url!);
    }

    [Fact]
    public async Task ChunkDetail_AllFields()
    {
        var json = @"{
            ""chunk_id"": ""c-abc"",
            ""document_id"": ""doc-1"",
            ""document_name"": ""Research.pdf"",
            ""content"": ""Text content here"",
            ""index"": 3,
            ""char_range_info"": {""start"": 100, ""end"": 500},
            ""token_count"": 120,
            ""entities"": [{
                ""id"": ""e1"",
                ""name"": ""QUANTUM"",
                ""entity_type"": ""CONCEPT"",
                ""description"": ""Quantum computing""
            }],
            ""relationships"": [{
                ""source_name"": ""QUANTUM"",
                ""target_name"": ""COMPUTING"",
                ""relation_type"": ""RELATED_TO"",
                ""description"": ""Related concepts""
            }],
            ""extraction_metadata"": {
                ""model"": ""gpt-4o"",
                ""gleaning_iterations"": 2,
                ""duration_ms"": 1500,
                ""input_tokens"": 200,
                ""output_tokens"": 50,
                ""cached"": false
            }
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new LineageService(http).ChunkDetailAsync("c-abc");
        Assert.Equal("c-abc", result.ChunkId);
        Assert.Equal("doc-1", result.DocumentId);
        Assert.Equal("Research.pdf", result.DocumentName);
        Assert.Equal("Text content here", result.Content);
        Assert.Equal(3, result.Index);
        Assert.Equal(100, result.CharRangeInfo!.Start);
        Assert.Equal(500, result.CharRangeInfo.End);
        Assert.Equal(120, result.TokenCount);
        Assert.Single(result.Entities!);
        Assert.Equal("QUANTUM", result.Entities![0].Name);
        Assert.Equal("CONCEPT", result.Entities[0].EntityType);
        Assert.Single(result.Relationships!);
        Assert.Equal("QUANTUM", result.Relationships![0].SourceName);
        Assert.Equal("RELATED_TO", result.Relationships[0].RelationType);
        Assert.Equal("gpt-4o", result.ExtractionMetadata!.Model);
        Assert.Equal(2, result.ExtractionMetadata.GleaningIterations);
        Assert.False(result.ExtractionMetadata.Cached);
        Assert.Contains("/chunks/c-abc", mock.LastCall!.Url!);
    }

    [Fact]
    public async Task ChunkLineage_AllFields()
    {
        var json = @"{
            ""chunk_id"": ""c-x"",
            ""document_id"": ""doc-5"",
            ""document_name"": ""Thesis.pdf"",
            ""document_type"": ""pdf"",
            ""index"": 7,
            ""start_line"": 50,
            ""end_line"": 75,
            ""start_offset"": 2000,
            ""end_offset"": 3000,
            ""token_count"": 180,
            ""content_preview"": ""First 100 chars..."",
            ""entity_count"": 4,
            ""relationship_count"": 2,
            ""entity_names"": [""ALICE"", ""BOB"", ""MIT"", ""RESEARCH""],
            ""document_metadata"": {""author"": ""Dr. Smith""}
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new LineageService(http).ChunkLineageAsync("c-x");
        Assert.Equal("c-x", result.ChunkId);
        Assert.Equal("doc-5", result.DocumentId);
        Assert.Equal("Thesis.pdf", result.DocumentName);
        Assert.Equal("pdf", result.DocumentType);
        Assert.Equal(7, result.Index);
        Assert.Equal(50, result.StartLine);
        Assert.Equal(75, result.EndLine);
        Assert.Equal(2000, result.StartOffset);
        Assert.Equal(3000, result.EndOffset);
        Assert.Equal(180, result.TokenCount);
        Assert.Equal("First 100 chars...", result.ContentPreview);
        Assert.Equal(4, result.EntityCount);
        Assert.Equal(2, result.RelationshipCount);
        Assert.Equal(4, result.EntityNames!.Count);
        Assert.Contains("ALICE", result.EntityNames);
        Assert.Equal("Dr. Smith", result.DocumentMetadata!.Value.GetProperty("author").GetString());
        Assert.Contains("/chunks/c-x/lineage", mock.LastCall!.Url!);
    }

    [Fact]
    public async Task EntityProvenance_AllFields()
    {
        var json = @"{
            ""entity_id"": ""ent-1"",
            ""entity_name"": ""DEEP_THOUGHT"",
            ""entity_type"": ""AI_SYSTEM"",
            ""description"": ""The ultimate computer"",
            ""sources"": [{
                ""document_id"": ""doc-42"",
                ""document_name"": ""Guide.txt"",
                ""chunks"": [{
                    ""chunk_id"": ""c-1"",
                    ""start_line"": 10,
                    ""end_line"": 20,
                    ""source_text"": ""Deep Thought computed the answer""
                }],
                ""first_extracted_at"": ""2025-01-15T08:00:00Z""
            }],
            ""total_extraction_count"": 3,
            ""related_entities"": [{
                ""entity_id"": ""ent-2"",
                ""entity_name"": ""EARTH"",
                ""relationship_type"": ""CREATED"",
                ""shared_documents"": 2
            }]
        }";
        var (http, mock) = MockHelperWithCalls(json);
        var result = await new LineageService(http).EntityProvenanceAsync("ent-1");
        Assert.Equal("ent-1", result.EntityId);
        Assert.Equal("DEEP_THOUGHT", result.EntityName);
        Assert.Equal("AI_SYSTEM", result.EntityType);
        Assert.Equal("The ultimate computer", result.Description);
        Assert.Single(result.Sources!);
        Assert.Equal("doc-42", result.Sources![0].DocumentId);
        Assert.Equal("Guide.txt", result.Sources[0].DocumentName);
        Assert.Single(result.Sources[0].Chunks!);
        Assert.Equal("c-1", result.Sources[0].Chunks![0].ChunkId);
        Assert.Equal(10, result.Sources[0].Chunks![0].StartLine);
        Assert.Equal("Deep Thought computed the answer", result.Sources[0].Chunks![0].SourceText);
        Assert.Equal("2025-01-15T08:00:00Z", result.Sources[0].FirstExtractedAt);
        Assert.Equal(3, result.TotalExtractionCount);
        Assert.Single(result.RelatedEntities!);
        Assert.Equal("EARTH", result.RelatedEntities![0].EntityName);
        Assert.Equal("CREATED", result.RelatedEntities[0].RelationshipType);
        Assert.Equal(2, result.RelatedEntities[0].SharedDocuments);
        Assert.Contains("/entities/ent-1/provenance", mock.LastCall!.Url!);
    }

    // ── Lineage Model Edge Cases (OODA-25) ─────────────────────────

    [Fact]
    public async Task EntityLineage_EmptySourceDocuments()
    {
        var http = MockHelper(@"{""entity_name"":""ORPHAN"",""source_documents"":[],""source_count"":0,""description_versions"":[]}");
        var result = await new LineageService(http).EntityLineageAsync("ORPHAN");
        Assert.Equal("ORPHAN", result.EntityName);
        Assert.Empty(result.SourceDocuments!);
        Assert.Equal(0, result.SourceCount);
        Assert.Empty(result.DescriptionVersions!);
    }

    [Fact]
    public async Task DocumentLineage_EmptyGraph()
    {
        var http = MockHelper(@"{""document_id"":""d-empty"",""chunk_count"":0,""entities"":[],""relationships"":[],""extraction_stats"":{""total_entities"":0,""unique_entities"":0,""total_relationships"":0,""unique_relationships"":0}}");
        var result = await new LineageService(http).DocumentLineageAsync("d-empty");
        Assert.Empty(result.Entities!);
        Assert.Empty(result.Relationships!);
        Assert.Equal(0, result.ExtractionStats!.TotalEntities);
    }

    [Fact]
    public async Task ChunkDetail_NullOptionalFields()
    {
        var http = MockHelper(@"{""chunk_id"":""c-min"",""document_id"":""d1"",""content"":""text"",""index"":0,""token_count"":10,""entities"":[],""relationships"":[]}");
        var result = await new LineageService(http).ChunkDetailAsync("c-min");
        Assert.Equal("c-min", result.ChunkId);
        Assert.Null(result.DocumentName);
        Assert.Null(result.CharRangeInfo);
        Assert.Null(result.ExtractionMetadata);
        Assert.Empty(result.Entities!);
    }

    [Fact]
    public async Task ChunkLineage_MinimalFields()
    {
        var http = MockHelper(@"{""chunk_id"":""c-min""}");
        var result = await new LineageService(http).ChunkLineageAsync("c-min");
        Assert.Equal("c-min", result.ChunkId);
        Assert.Null(result.DocumentId);
        Assert.Null(result.EntityNames);
        Assert.Null(result.DocumentMetadata);
    }

    [Fact]
    public async Task EntityProvenance_NoRelatedEntities()
    {
        var http = MockHelper(@"{""entity_id"":""e-solo"",""entity_name"":""SOLO"",""sources"":[],""total_extraction_count"":0,""related_entities"":[]}");
        var result = await new LineageService(http).EntityProvenanceAsync("e-solo");
        Assert.Empty(result.Sources!);
        Assert.Equal(0, result.TotalExtractionCount);
        Assert.Empty(result.RelatedEntities!);
    }

    [Fact]
    public async Task EntityLineage_MultipleSourceDocuments()
    {
        var json = @"{
            ""entity_name"": ""SHARED_ENTITY"",
            ""source_documents"": [
                {""document_id"":""d1"",""chunk_ids"":[""c1""],""line_ranges"":[{""start_line"":1,""end_line"":5}]},
                {""document_id"":""d2"",""chunk_ids"":[""c2"",""c3""],""line_ranges"":[{""start_line"":10,""end_line"":15},{""start_line"":20,""end_line"":25}]},
                {""document_id"":""d3"",""chunk_ids"":[""c4""],""line_ranges"":[]}
            ],
            ""source_count"": 3,
            ""description_versions"": [
                {""version"":1,""description"":""V1"",""created_at"":""2025-01-01T00:00:00Z""},
                {""version"":2,""description"":""V2 updated"",""created_at"":""2025-06-01T00:00:00Z""}
            ]
        }";
        var http = MockHelper(json);
        var result = await new LineageService(http).EntityLineageAsync("SHARED_ENTITY");
        Assert.Equal(3, result.SourceDocuments!.Count);
        Assert.Equal(3, result.SourceCount);
        Assert.Equal(2, result.SourceDocuments[1].LineRanges!.Count);
        Assert.Equal(2, result.DescriptionVersions!.Count);
        Assert.Equal("V2 updated", result.DescriptionVersions[1].Description);
    }

    [Fact]
    public void Client_HasLineageServiceAccessor()
    {
        var client = new EdgeQuakeClient();
        Assert.NotNull(client.Lineage);
        Assert.IsType<LineageService>(client.Lineage);
    }
}
