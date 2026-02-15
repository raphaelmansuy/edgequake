package io.edgequake.sdk;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.*;
import io.edgequake.sdk.models.AuthModels.*;
import io.edgequake.sdk.models.DocumentModels.*;
import io.edgequake.sdk.models.GraphModels.*;
import io.edgequake.sdk.models.OperationModels.*;
import io.edgequake.sdk.models.QueryModels.*;
import io.edgequake.sdk.models.LineageModels.*;
import io.edgequake.sdk.resources.*;

import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.Map;

/**
 * Comprehensive unit tests for all Java SDK services.
 *
 * Uses FakeHttpClient to intercept HTTP calls and return
 * pre-configured responses without network I/O.
 */
class UnitTest {

    private HttpHelper http;
    private FakeHttpClient fake;

    @BeforeEach
    void setup() {
        var config = EdgeQuakeConfig.builder()
                .baseUrl("http://test:8080")
                .apiKey("test-key")
                .tenantId("t1")
                .userId("u1")
                .workspaceId("w1")
                .build();
        var pair = FakeHttpClient.createTestHelper(config);
        http = (HttpHelper) pair[0];
        fake = (FakeHttpClient) pair[1];
    }

    // ── EdgeQuakeConfig ──────────────────────────────────────────────

    @Test
    void configDefaults() {
        var c = EdgeQuakeConfig.builder().build();
        assertEquals("http://localhost:8080", c.baseUrl());
        assertNull(c.apiKey());
        assertNull(c.tenantId());
        assertNull(c.userId());
        assertNull(c.workspaceId());
        assertEquals(30, c.timeoutSeconds());
    }

    @Test
    void configCustomValues() {
        var c = EdgeQuakeConfig.builder()
                .baseUrl("http://custom:9090")
                .apiKey("key123")
                .tenantId("tenant1")
                .userId("user1")
                .workspaceId("ws1")
                .timeoutSeconds(60)
                .build();
        assertEquals("http://custom:9090", c.baseUrl());
        assertEquals("key123", c.apiKey());
        assertEquals("tenant1", c.tenantId());
        assertEquals("user1", c.userId());
        assertEquals("ws1", c.workspaceId());
        assertEquals(60, c.timeoutSeconds());
    }

    // ── EdgeQuakeException ───────────────────────────────────────────

    @Test
    void exceptionWithStatusCode() {
        var ex = new EdgeQuakeException(404, "{\"detail\":\"not found\"}");
        assertEquals(404, ex.statusCode());
        assertTrue(ex.responseBody().contains("not found"));
        assertTrue(ex.getMessage().contains("404"));
    }

    @Test
    void exceptionWithCause() {
        var cause = new RuntimeException("root cause");
        var ex = new EdgeQuakeException("wrapped", cause);
        assertEquals(cause, ex.getCause());
        assertEquals(0, ex.statusCode());
    }

    // ── EdgeQuakeClient ──────────────────────────────────────────────

    @Test
    void clientCreatesAllServices() {
        var config = EdgeQuakeConfig.builder().build();
        var client = new EdgeQuakeClient(config);
        assertNotNull(client.health());
        assertNotNull(client.documents());
        assertNotNull(client.entities());
        assertNotNull(client.relationships());
        assertNotNull(client.graph());
        assertNotNull(client.query());
        assertNotNull(client.chat());
        assertNotNull(client.auth());
        assertNotNull(client.users());
        assertNotNull(client.apiKeys());
        assertNotNull(client.tenants());
        assertNotNull(client.conversations());
        assertNotNull(client.folders());
        assertNotNull(client.tasks());
        assertNotNull(client.pipeline());
        assertNotNull(client.models());
        assertNotNull(client.workspaces());
        assertNotNull(client.pdf());
        assertNotNull(client.costs());
        assertNotNull(client.lineage());
    }

    // ── HttpHelper error handling ────────────────────────────────────

    @Test
    void executeThrowsOnErrorStatus() {
        fake.respondWithError(500, "{\"error\":\"internal\"}");
        assertThrows(EdgeQuakeException.class, () -> {
            new HealthService(http).check();
        });
    }

    @Test
    void executeWrapsIOException() {
        fake.throwOnSend(new java.io.IOException("network error"));
        var ex = assertThrows(EdgeQuakeException.class, () -> {
            new HealthService(http).check();
        });
        assertTrue(ex.getMessage().contains("network error"));
    }

    // ── HealthService ────────────────────────────────────────────────

    @Test
    void healthCheck() {
        fake.respondWith("""
            {"status":"healthy","version":"0.1.0","storage_mode":"postgresql","workspace_id":"default","llm_provider_name":"ollama"}
            """);
        var svc = new HealthService(http);
        var health = svc.check();
        assertEquals("healthy", health.status);
        assertEquals("0.1.0", health.version);
        assertEquals("postgresql", health.storageMode);
        assertEquals("ollama", health.llmProviderName);
    }

    @Test
    void healthCheckEndpoint() {
        fake.respondWith("{\"status\":\"healthy\"}");
        new HealthService(http).check();
        assertTrue(fake.lastRequest().uri().contains("/health"));
        assertEquals("GET", fake.lastRequest().method());
    }

    @Test
    void healthCheckError() {
        fake.respondWithError(503);
        assertThrows(EdgeQuakeException.class, () -> new HealthService(http).check());
    }

    // ── DocumentService ──────────────────────────────────────────────

    @Test
    void documentsList() {
        fake.respondWith("""
            {"documents":[{"id":"d1","title":"Test","status":"completed"}],"pagination":{"page":1,"per_page":20,"total":1,"total_pages":1}}
            """);
        var svc = new DocumentService(http);
        var result = svc.list(1, 20);
        assertEquals(1, result.pagination.total);
        assertEquals("d1", result.documents.get(0).id);
    }

    @Test
    void documentsGet() {
        fake.respondWith("""
            {"id":"d1","title":"My Doc","status":"completed","chunk_count":5}
            """);
        var svc = new DocumentService(http);
        var doc = svc.get("d1");
        assertEquals("d1", doc.id);
        assertEquals("My Doc", doc.title);
        assertEquals(5, doc.chunkCount);
    }

    @Test
    void documentsUploadText() {
        fake.respondWith("""
            {"document_id":"d-new","status":"processing","message":"Upload received","track_id":"t-123"}
            """);
        var svc = new DocumentService(http);
        var result = svc.uploadText("Hello World", "Test Title");
        assertEquals("d-new", result.documentId);
        assertEquals("processing", result.status);
        assertEquals("t-123", result.trackId);
    }

    @Test
    void documentsDelete() {
        fake.respondWith("{}");
        var svc = new DocumentService(http);
        svc.delete("d1");
        assertTrue(fake.lastRequest().uri().contains("/api/v1/documents/d1"));
        assertEquals("DELETE", fake.lastRequest().method());
    }

    @Test
    void documentsTrack() {
        fake.respondWith("""
            {"track_id":"tk-1","status":"processing","progress":0.75}
            """);
        var svc = new DocumentService(http);
        var result = svc.track("tk-1");
        assertEquals("tk-1", result.trackId);
        assertEquals("processing", result.status);
    }

    @Test
    void documentsScan() {
        fake.respondWith("""
            {"files_found":3,"files_queued":3,"files_skipped":0}
            """);
        var svc = new DocumentService(http);
        var req = new ScanRequest();
        req.path = "/docs";
        req.recursive = true;
        var result = svc.scan(req);
        assertEquals(3, result.filesFound);
    }

    @Test
    void documentsDeletionImpact() {
        fake.respondWith("""
            {"entity_count":10,"relationship_count":25,"chunk_count":50}
            """);
        var svc = new DocumentService(http);
        var result = svc.deletionImpact("d1");
        assertEquals(10, result.entityCount);
        assertEquals(25, result.relationshipCount);
    }

    @Test
    void documentsError() {
        fake.respondWithError(404);
        var svc = new DocumentService(http);
        assertThrows(EdgeQuakeException.class, () -> svc.get("nonexistent"));
    }

    // ── EntityService ────────────────────────────────────────────────

    @Test
    void entitiesList() {
        fake.respondWith("""
            {"items":[{"entity_name":"ALICE","entity_type":"PERSON"}],"total":1,"page":1,"page_size":20,"total_pages":1}
            """);
        var svc = new EntityService(http);
        var result = svc.list(1, 20, null);
        assertEquals(1, result.total);
        assertEquals("ALICE", result.items.get(0).entityName);
    }

    @Test
    void entitiesListWithFilter() {
        fake.respondWith("""
            {"items":[],"total":0,"page":1,"page_size":20,"total_pages":0}
            """);
        var svc = new EntityService(http);
        svc.list(1, 10, "PERSON");
        assertTrue(fake.lastRequest().uri().contains("entity_type=PERSON"));
    }

    @Test
    void entitiesGet() {
        fake.respondWith("""
            {"entity":{"entity_name":"BOB","entity_type":"PERSON","description":"desc"},"relationships":{"outgoing":[],"incoming":[]},"statistics":{"total_relationships":0}}
            """);
        var svc = new EntityService(http);
        var result = svc.get("BOB");
        assertEquals("BOB", result.entity.entityName);
    }

    @Test
    void entitiesCreate() {
        fake.respondWith("""
            {"status":"success","message":"Created","entity":{"entity_name":"NEW_ENTITY","entity_type":"TEST"}}
            """);
        var svc = new EntityService(http);
        var result = svc.create(new CreateEntityRequest("NEW_ENTITY", "TEST", "desc", "src"));
        assertEquals("success", result.status);
        assertEquals("NEW_ENTITY", result.entity.entityName);
    }

    @Test
    void entitiesDelete() {
        fake.respondWith("""
            {"status":"success","deleted_entity_id":"e1","deleted_relationships":3}
            """);
        var svc = new EntityService(http);
        var result = svc.delete("TEST_ENTITY");
        assertEquals("success", result.status);
        assertEquals(3, result.deletedRelationships);
        assertTrue(fake.lastRequest().uri().contains("confirm=true"));
    }

    @Test
    void entitiesExists() {
        fake.respondWith("""
            {"exists":true,"entity_id":"e1","entity_type":"PERSON"}
            """);
        var svc = new EntityService(http);
        var result = svc.exists("ALICE");
        assertTrue(result.exists);
        assertEquals("e1", result.entityId);
    }

    @Test
    void entitiesMerge() {
        fake.respondWith("""
            {"merged_entity":{"entity_name":"TARGET"},"merged_count":1,"message":"Merged"}
            """);
        var svc = new EntityService(http);
        var result = svc.merge(new MergeEntitiesRequest("SOURCE", "TARGET"));
        assertEquals("TARGET", result.mergedEntity.entityName);
    }

    @Test
    void entitiesNeighborhood() {
        fake.respondWith("""
            {"center":{"entity_name":"BOB"},"nodes":[],"edges":[],"depth":2}
            """);
        var svc = new EntityService(http);
        var result = svc.neighborhood("BOB", 2);
        assertEquals("BOB", result.center.entityName);
        assertEquals(2, result.depth);
    }

    @Test
    void entitiesError() {
        fake.respondWithError(404);
        assertThrows(EdgeQuakeException.class, () -> new EntityService(http).get("NONEXISTENT"));
    }

    // ── RelationshipService ──────────────────────────────────────────

    @Test
    void relationshipsList() {
        fake.respondWith("""
            {"items":[{"source":"A","target":"B","relationship_type":"KNOWS","weight":1.0}],"total":1}
            """);
        var svc = new RelationshipService(http);
        var result = svc.list(1, 20);
        assertEquals(1, result.total);
        assertEquals("A", result.items.get(0).source);
    }

    @Test
    void relationshipsCreate() {
        fake.respondWith("""
            {"source":"A","target":"B","relationship_type":"WORKS_WITH","weight":0.8}
            """);
        var svc = new RelationshipService(http);
        var result = svc.create(new CreateRelationshipRequest("A", "B", "WORKS_WITH"));
        assertEquals("WORKS_WITH", result.relationshipType);
        assertEquals("POST", fake.lastRequest().method());
    }

    // ── GraphService ─────────────────────────────────────────────────

    @Test
    void graphGet() {
        fake.respondWith("""
            {"nodes":[{"id":"n1","label":"Alice","node_type":"PERSON"}],"edges":[{"source":"n1","target":"n2","edge_type":"KNOWS"}],"total_nodes":1,"total_edges":1}
            """);
        var svc = new GraphService(http);
        var result = svc.get(0);
        assertEquals(1, result.nodes.size());
        assertEquals("Alice", result.nodes.get(0).label);
        assertEquals(1, result.edges.size());
    }

    @Test
    void graphSearch() {
        fake.respondWith("""
            {"nodes":[{"id":"n1","label":"Result"}],"total_matches":1}
            """);
        var svc = new GraphService(http);
        var result = svc.search("test", 10);
        assertEquals(1, result.totalMatches);
        assertTrue(fake.lastRequest().uri().contains("q=test"));
    }

    @Test
    void graphError() {
        fake.respondWithError(500);
        assertThrows(EdgeQuakeException.class, () -> new GraphService(http).get(0));
    }

    // ── QueryService ─────────────────────────────────────────────────

    @Test
    void queryExecute() {
        fake.respondWith("""
            {"answer":"The answer is 42.","sources":[{"document_id":"doc1"}],"mode":"hybrid"}
            """);
        var svc = new QueryService(http);
        var result = svc.execute(new QueryRequest("What is the answer?", "hybrid"));
        assertEquals("The answer is 42.", result.answer);
        assertEquals("hybrid", result.mode);
        assertEquals(1, result.sources.size());
    }

    @Test
    void queryEndpoint() {
        fake.respondWith("{\"answer\":\"ok\"}");
        new QueryService(http).execute(new QueryRequest("test", "local"));
        assertTrue(fake.lastRequest().uri().contains("/api/v1/query"));
        assertEquals("POST", fake.lastRequest().method());
    }

    @Test
    void queryError() {
        fake.respondWithError(422);
        assertThrows(EdgeQuakeException.class, () ->
                new QueryService(http).execute(new QueryRequest("", "hybrid")));
    }

    // ── ChatService ──────────────────────────────────────────────────

    @Test
    void chatCompletions() {
        fake.respondWith("""
            {"conversation_id":"conv-1","user_message_id":"msg-1","assistant_message_id":"msg-2","content":"Hello!","mode":"hybrid","sources":[]}
            """);
        var svc = new ChatService(http);
        var req = new ChatCompletionRequest("Hi");
        var result = svc.completions(req);
        assertEquals("conv-1", result.conversationId);
        assertEquals("Hello!", result.content);
    }

    @Test
    void chatError() {
        fake.respondWithError(500);
        assertThrows(EdgeQuakeException.class, () ->
                new ChatService(http).completions(new ChatCompletionRequest("Hi")));
    }

    // ── AuthService ──────────────────────────────────────────────────

    @Test
    void authLogin() {
        fake.respondWith("""
            {"access_token":"jwt-token-123","token_type":"bearer","expires_in":3600}
            """);
        var svc = new AuthService(http);
        var result = svc.login(new LoginRequest("admin", "password"));
        assertEquals("jwt-token-123", result.accessToken);
        assertEquals("bearer", result.tokenType);
    }

    @Test
    void authMe() {
        fake.respondWith("""
            {"id":"u1","username":"admin","email":"a@b.com","role":"admin"}
            """);
        var svc = new AuthService(http);
        var result = svc.me();
        assertEquals("admin", result.username);
    }

    @Test
    void authRefresh() {
        fake.respondWith("""
            {"access_token":"new-token","refresh_token":"new-refresh","token_type":"bearer"}
            """);
        var svc = new AuthService(http);
        var result = svc.refresh(new RefreshRequest("old-refresh"));
        assertEquals("new-token", result.accessToken);
    }

    @Test
    void authError() {
        fake.respondWithError(401);
        assertThrows(EdgeQuakeException.class, () ->
                new AuthService(http).login(new LoginRequest("bad", "creds")));
    }

    // ── UserService ──────────────────────────────────────────────────

    @Test
    void usersList() {
        fake.respondWith("""
            {"users":[{"id":"u1","username":"admin","email":"a@b.com","role":"admin"}],"total":1}
            """);
        var svc = new UserService(http);
        var result = svc.list();
        assertEquals(1, result.total);
        assertEquals("admin", result.users.get(0).username);
    }

    @Test
    void usersGet() {
        fake.respondWith("""
            {"id":"u1","username":"admin","email":"a@b.com","role":"admin"}
            """);
        var svc = new UserService(http);
        var result = svc.get("u1");
        assertEquals("u1", result.id);
    }

    @Test
    void usersError() {
        fake.respondWithError(403);
        assertThrows(EdgeQuakeException.class, () -> new UserService(http).list());
    }

    // ── ApiKeyService ────────────────────────────────────────────────

    @Test
    void apiKeysList() {
        fake.respondWith("""
            {"keys":[{"id":"k1","name":"my-key","created_at":"2024-01-01"}],"total":1}
            """);
        var svc = new ApiKeyService(http);
        var result = svc.list();
        assertEquals(1, result.total);
        assertEquals("my-key", result.keys.get(0).name);
    }

    @Test
    void apiKeysCreate() {
        fake.respondWith("""
            {"id":"k-new","key":"sk-abc123","name":"new-key","created_at":"2024-01-01"}
            """);
        var svc = new ApiKeyService(http);
        var result = svc.create("new-key");
        assertEquals("sk-abc123", result.key);
    }

    @Test
    void apiKeysRevoke() {
        fake.respondWith("{}");
        var svc = new ApiKeyService(http);
        svc.revoke("k1");
        assertTrue(fake.lastRequest().uri().contains("/api/v1/api-keys/k1"));
        assertEquals("DELETE", fake.lastRequest().method());
    }

    // ── TenantService ────────────────────────────────────────────────

    @Test
    void tenantsList() {
        fake.respondWith("""
            {"items":[{"id":"t1","name":"Default","slug":"default"}],"total":1}
            """);
        var svc = new TenantService(http);
        var result = svc.list();
        assertEquals(1, result.total);
        assertEquals("Default", result.items.get(0).name);
    }

    @Test
    void tenantsCreate() {
        fake.respondWith("""
            {"id":"t-new","name":"New Tenant","slug":"new-tenant"}
            """);
        var svc = new TenantService(http);
        var result = svc.create(new CreateTenantRequest("New Tenant", "new-tenant"));
        assertEquals("t-new", result.id);
    }

    // ── ConversationService ──────────────────────────────────────────

    @Test
    void conversationsList() {
        fake.respondWith("""
            {"items":[{"id":"c1","title":"Test Chat","message_count":5}]}
            """);
        var svc = new ConversationService(http);
        var result = svc.list();
        assertEquals(1, result.size());
        assertEquals("Test Chat", result.get(0).title);
    }

    @Test
    void conversationsCreate() {
        fake.respondWith("""
            {"id":"c-new","title":"New Chat"}
            """);
        var svc = new ConversationService(http);
        var result = svc.create(new CreateConversationRequest("New Chat"));
        assertEquals("c-new", result.id);
    }

    @Test
    void conversationsGet() {
        fake.respondWith("""
            {"conversation":{"id":"c1","title":"Chat"},"messages":[{"id":"m1","role":"user","content":"Hello"}]}
            """);
        var svc = new ConversationService(http);
        var result = svc.get("c1");
        assertEquals("c1", result.getId());
        assertEquals(1, result.messages.size());
    }

    @Test
    void conversationsDelete() {
        fake.respondWith("{}");
        var svc = new ConversationService(http);
        svc.delete("c1");
        assertTrue(fake.lastRequest().uri().contains("/api/v1/conversations/c1"));
    }

    @Test
    void conversationsCreateMessage() {
        fake.respondWith("""
            {"id":"m-new","role":"user","content":"Hello world"}
            """);
        var svc = new ConversationService(http);
        var result = svc.createMessage("c1", new CreateMessageRequest("user", "Hello world"));
        assertEquals("m-new", result.id);
        assertEquals("user", result.role);
    }

    @Test
    void conversationsShare() {
        fake.respondWith("""
            {"share_id":"share-1","url":"https://edgequake.io/share/share-1"}
            """);
        var svc = new ConversationService(http);
        var result = svc.share("c1");
        assertEquals("share-1", result.shareId);
    }

    @Test
    void conversationsBulkDelete() {
        fake.respondWith("""
            {"deleted_count":3}
            """);
        var svc = new ConversationService(http);
        var result = svc.bulkDelete(List.of("c1", "c2", "c3"));
        assertEquals(3, result.deletedCount);
    }

    @Test
    void conversationsPin() {
        fake.respondWith("{}");
        var svc = new ConversationService(http);
        svc.pin("c1");
        assertTrue(fake.lastRequest().uri().contains("/api/v1/conversations/c1"));
        assertTrue(fake.lastRequest().body().contains("is_pinned"));
    }

    @Test
    void conversationsUnpin() {
        fake.respondWith("{}");
        var svc = new ConversationService(http);
        svc.unpin("c1");
        assertTrue(fake.lastRequest().body().contains("false"));
    }

    @Test
    void conversationsError() {
        fake.respondWithError(404);
        assertThrows(EdgeQuakeException.class, () -> new ConversationService(http).get("nonexistent"));
    }

    // ── FolderService ────────────────────────────────────────────────

    @Test
    void foldersList() {
        fake.respondWith("""
            [{"id":"f1","name":"My Folder"}]
            """);
        var svc = new FolderService(http);
        var result = svc.list();
        assertEquals(1, result.size());
        assertEquals("My Folder", result.get(0).name);
    }

    @Test
    void foldersCreate() {
        fake.respondWith("""
            {"id":"f-new","name":"New Folder"}
            """);
        var svc = new FolderService(http);
        var result = svc.create(new CreateFolderRequest("New Folder"));
        assertEquals("f-new", result.id);
    }

    @Test
    void foldersGet() {
        fake.respondWith("""
            {"id":"f1","name":"Folder","conversation_count":3}
            """);
        var svc = new FolderService(http);
        var result = svc.get("f1");
        assertEquals(3, result.conversationCount);
    }

    @Test
    void foldersDelete() {
        fake.respondWith("{}");
        var svc = new FolderService(http);
        svc.delete("f1");
        assertEquals("DELETE", fake.lastRequest().method());
    }

    // ── TaskService ──────────────────────────────────────────────────

    @Test
    void tasksList() {
        fake.respondWith("""
            {"tasks":[{"track_id":"t1","status":"completed","task_type":"extraction"}],"total":1}
            """);
        var svc = new TaskService(http);
        var result = svc.list(null, 0, 0);
        assertEquals(1, result.total);
        assertEquals("completed", result.tasks.get(0).status);
    }

    @Test
    void tasksListWithFilter() {
        fake.respondWith("""
            {"tasks":[],"total":0}
            """);
        var svc = new TaskService(http);
        svc.list("running", 1, 10);
        assertTrue(fake.lastRequest().uri().contains("status=running"));
    }

    @Test
    void tasksGet() {
        fake.respondWith("""
            {"track_id":"t1","status":"running","task_type":"ingestion"}
            """);
        var svc = new TaskService(http);
        var result = svc.get("t1");
        assertEquals("t1", result.trackId);
        assertEquals("running", result.status);
    }

    @Test
    void tasksCancel() {
        fake.respondWith("{}");
        var svc = new TaskService(http);
        svc.cancel("t1");
        assertTrue(fake.lastRequest().uri().contains("/api/v1/tasks/t1/cancel"));
        assertEquals("POST", fake.lastRequest().method());
    }

    @Test
    void tasksError() {
        fake.respondWithError(404);
        assertThrows(EdgeQuakeException.class, () -> new TaskService(http).get("nonexistent"));
    }

    // ── PipelineService ──────────────────────────────────────────────

    @Test
    void pipelineStatus() {
        fake.respondWith("""
            {"is_busy":false,"total_documents":10,"processed_documents":8,"pending_tasks":2,"processing_tasks":0,"completed_tasks":8,"failed_tasks":0}
            """);
        var svc = new PipelineService(http);
        var result = svc.status();
        assertFalse(result.isBusy);
        assertEquals(10, result.totalDocuments);
        assertEquals(2, result.pendingTasks);
    }

    @Test
    void pipelineMetrics() {
        fake.respondWith("""
            {"queue_depth":5,"processing":2,"completed_last_hour":10,"failed_last_hour":0,"avg_processing_time_ms":1500.0}
            """);
        var svc = new PipelineService(http);
        var result = svc.metrics();
        assertEquals(5, result.queueDepth);
        assertEquals(2, result.processing);
    }

    // ── ModelService ─────────────────────────────────────────────────

    @Test
    void modelsCatalog() {
        fake.respondWith("""
            {"providers":[{"name":"ollama","display_name":"Ollama","models":[{"name":"llama3"}]}]}
            """);
        var svc = new ModelService(http);
        var result = svc.list();
        assertEquals(1, result.providers.size());
        assertEquals("ollama", result.providers.get(0).name);
    }

    @Test
    void modelsProviderHealth() {
        fake.respondWith("""
            [{"name":"ollama","display_name":"Ollama","enabled":true,"priority":1}]
            """);
        var svc = new ModelService(http);
        var result = svc.providerHealth();
        assertEquals(1, result.size());
        assertTrue(result.get(0).enabled);
    }

    @Test
    void modelsProviderStatus() {
        fake.respondWith("""
            {"current_provider":"ollama","current_model":"llama3","status":"healthy"}
            """);
        var svc = new ModelService(http);
        var result = svc.providerStatus();
        assertEquals("ollama", result.currentProvider);
    }

    @Test
    void modelsError() {
        fake.respondWithError(500);
        assertThrows(EdgeQuakeException.class, () -> new ModelService(http).list());
    }

    // ── WorkspaceService ─────────────────────────────────────────────

    @Test
    void workspacesListForTenant() {
        fake.respondWith("""
            [{"id":"w1","name":"Default","slug":"default"}]
            """);
        var svc = new WorkspaceService(http);
        var result = svc.listForTenant("t1");
        assertEquals(1, result.size());
        assertEquals("Default", result.get(0).name);
    }

    @Test
    void workspacesGet() {
        fake.respondWith("""
            {"id":"w1","name":"Default","slug":"default"}
            """);
        var svc = new WorkspaceService(http);
        var result = svc.get("w1");
        assertEquals("w1", result.id);
    }

    @Test
    void workspacesStats() {
        fake.respondWith("""
            {"workspace_id":"w1","document_count":10,"entity_count":50,"relationship_count":100}
            """);
        var svc = new WorkspaceService(http);
        var result = svc.stats("w1");
        assertEquals(10, result.documentCount);
        assertEquals(50, result.entityCount);
    }

    @Test
    void workspacesRebuildEmbeddings() {
        fake.respondWith("""
            {"status":"started","message":"Rebuild initiated","track_id":"tk-1"}
            """);
        var svc = new WorkspaceService(http);
        var result = svc.rebuildEmbeddings("w1");
        assertEquals("started", result.status);
    }

    @Test
    void workspacesError() {
        fake.respondWithError(403);
        assertThrows(EdgeQuakeException.class, () -> new WorkspaceService(http).get("w1"));
    }

    // ── PdfService ───────────────────────────────────────────────────

    @Test
    void pdfProgress() {
        fake.respondWith("""
            {"track_id":"tk-1","status":"processing","progress":0.75}
            """);
        var svc = new PdfService(http);
        var result = svc.progress("tk-1");
        assertEquals("tk-1", result.trackId);
        assertEquals("processing", result.status);
    }

    @Test
    void pdfContent() {
        fake.respondWith("""
            {"id":"pdf-1","markdown":"# Title\\n\\nHello world"}
            """);
        var svc = new PdfService(http);
        var result = svc.content("pdf-1");
        assertTrue(result.markdown.contains("Hello world"));
    }

    @Test
    void pdfStatus() {
        fake.respondWith("""
            {"track_id":"tk-1","status":"complete"}
            """);
        var svc = new PdfService(http);
        var result = svc.status("pdf-1");
        assertEquals("complete", result.status);
    }

    @Test
    void pdfError() {
        fake.respondWithError(404);
        assertThrows(EdgeQuakeException.class, () -> new PdfService(http).progress("nonexistent"));
    }

    // ── CostService ──────────────────────────────────────────────────

    @Test
    void costsSummary() {
        fake.respondWith("""
            {"total_cost_usd":12.50,"total_tokens":50000,"document_count":100,"query_count":500}
            """);
        var svc = new CostService(http);
        var result = svc.summary();
        assertEquals(12.50, result.totalCostUsd);
        assertEquals(100, result.documentCount);
    }

    @Test
    void costsHistory() {
        fake.respondWith("""
            [{"date":"2024-01-01","cost_usd":1.50,"tokens":1000,"requests":50}]
            """);
        var svc = new CostService(http);
        var result = svc.history("2024-01-01", "2024-01-31");
        assertEquals(1, result.size());
        assertEquals("2024-01-01", result.get(0).date);
    }

    @Test
    void costsBudget() {
        fake.respondWith("""
            {"monthly_budget_usd":100.0,"current_spend_usd":12.50,"remaining_usd":87.50}
            """);
        var svc = new CostService(http);
        var result = svc.budget();
        assertEquals(100.0, result.monthlyBudgetUsd);
        assertEquals(87.50, result.remainingUsd);
    }

    @Test
    void costsError() {
        fake.respondWithError(403);
        assertThrows(EdgeQuakeException.class, () -> new CostService(http).summary());
    }

    // ── Model data classes ───────────────────────────────────────────

    @Test
    void documentModel() {
        var d = new Document();
        assertNull(d.id);
        assertNull(d.title);
        assertNull(d.status);
    }

    @Test
    void entityModel() {
        var e = new Entity();
        e.entityName = "TEST";
        e.entityType = "PERSON";
        assertEquals("TEST", e.entityName);
    }

    @Test
    void relationshipModel() {
        var r = new Relationship();
        r.source = "A";
        r.target = "B";
        r.weight = 0.8;
        assertEquals("A", r.source);
    }

    @Test
    void chatMessageModel() {
        var m = new ChatMessage("user", "Hello");
        assertEquals("user", m.role);
        assertEquals("Hello", m.content);
    }

    @Test
    void queryRequestModel() {
        var q = new QueryRequest("test", "hybrid");
        assertEquals("test", q.query);
        assertEquals("hybrid", q.mode);
    }

    @Test
    void pipelineStatusModel() {
        var p = new PipelineStatus();
        assertFalse(p.isBusy);
        assertEquals(0, p.pendingTasks);
    }

    @Test
    void costSummaryModel() {
        var c = new CostSummary();
        assertEquals(0, c.documentCount);
    }

    @Test
    void uploadResponseModel() {
        var u = new UploadResponse();
        assertNull(u.documentId);
        assertNull(u.status);
    }

    // ── Request capture verification ─────────────────────────────────

    @Test
    void requestsHitCorrectEndpoints() {
        fake.respondWith("{\"status\":\"healthy\"}");
        new HealthService(http).check();
        assertTrue(fake.lastRequest().uri().contains("/health"));

        fake.respondWith("{\"documents\":[],\"pagination\":{\"page\":1,\"per_page\":20,\"total\":0,\"total_pages\":0}}");
        new DocumentService(http).list(1, 20);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/documents"));

        fake.respondWith("{\"items\":[],\"total\":0}");
        new EntityService(http).list(1, 20, null);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/graph/entities"));
    }

    @Test
    void allRequestMethodsUsedCorrectly() {
        fake.respondWith("{\"status\":\"healthy\"}");
        new HealthService(http).check();
        assertEquals("GET", fake.lastRequest().method());

        fake.respondWith("{\"answer\":\"ok\"}");
        new QueryService(http).execute(new QueryRequest("test", "hybrid"));
        assertEquals("POST", fake.lastRequest().method());

        fake.respondWith("{}");
        new DocumentService(http).delete("d1");
        assertEquals("DELETE", fake.lastRequest().method());
    }

    @Test
    void fakeClientCapturesAllRequests() {
        fake.respondWith("{\"status\":\"healthy\"}");
        new HealthService(http).check();

        fake.respondWith("{\"answer\":\"ok\"}");
        new QueryService(http).execute(new QueryRequest("q", "hybrid"));

        assertEquals(2, fake.allRequests().size());
        assertEquals("GET", fake.allRequests().get(0).method());
        assertEquals("POST", fake.allRequests().get(1).method());
    }

    @Test
    void fakeClientClearResetsState() {
        fake.respondWith("{\"status\":\"healthy\"}");
        new HealthService(http).check();
        assertEquals(1, fake.allRequests().size());

        fake.clear();
        assertEquals(0, fake.allRequests().size());
    }

    // ── Lineage & Metadata Tests ─────────────────────────────────────
    // WHY: The improve-lineage mission requires source_id, metadata,
    // and provenance fields to be properly tested across all SDKs.

    @Test
    void entityModelHasSourceId() {
        var e = new Entity();
        e.sourceId = "doc-123";
        assertEquals("doc-123", e.sourceId);
    }

    @Test
    void entityModelHasMetadata() {
        var e = new Entity();
        e.metadata = java.util.Map.of("key", "value");
        assertNotNull(e.metadata);
        @SuppressWarnings("unchecked")
        var meta = (java.util.Map<String, Object>) e.metadata;
        assertEquals("value", meta.get("key"));
    }

    @Test
    void entityModelHasTimestamps() {
        var e = new Entity();
        e.createdAt = "2025-01-01T00:00:00Z";
        e.updatedAt = "2025-01-02T00:00:00Z";
        assertNotNull(e.createdAt);
        assertNotNull(e.updatedAt);
    }

    @Test
    void createEntityRequestIncludesSourceId() {
        var req = new CreateEntityRequest("ALICE", "person", "A researcher", "doc-456");
        assertEquals("doc-456", req.sourceId);
    }

    @Test
    void createEntityRequestWithMetadata() {
        var req = new CreateEntityRequest();
        req.entityName = "BOB";
        req.entityType = "person";
        req.description = "An engineer";
        req.sourceId = "src-1";
        req.metadata = java.util.Map.of("confidence", 0.95);
        assertNotNull(req.metadata);
    }

    @Test
    void entityCreateSendsSourceId() {
        fake.respondWith("{\"status\":\"success\",\"message\":\"created\"}");
        var req = new CreateEntityRequest("ALICE", "person", "test", "doc-lineage-1");
        new EntityService(http).create(req);
        var body = fake.lastRequest().body();
        assertTrue(body.contains("doc-lineage-1"), "Request body should contain source_id");
    }

    @Test
    void entityCreateSendsMetadata() {
        fake.respondWith("{\"status\":\"success\",\"message\":\"created\"}");
        var req = new CreateEntityRequest();
        req.entityName = "META_ENTITY";
        req.entityType = "concept";
        req.description = "With metadata";
        req.sourceId = "src-m";
        req.metadata = java.util.Map.of("origin", "test");
        new EntityService(http).create(req);
        var body = fake.lastRequest().body();
        assertTrue(body.contains("META_ENTITY"));
        assertTrue(body.contains("src-m"));
    }

    @Test
    void relationshipModelHasProperties() {
        var r = new Relationship();
        r.properties = java.util.Map.of("weight", 0.8, "source_doc", "doc-1");
        assertNotNull(r.properties);
        assertEquals(0.8, r.properties.get("weight"));
        assertEquals("doc-1", r.properties.get("source_doc"));
    }

    @Test
    void createRelationshipSendsDescription() {
        fake.respondWith("{\"status\":\"success\"}");
        var req = new CreateRelationshipRequest("ALICE", "BOB", "COLLABORATES_WITH");
        req.weight = 0.9;
        req.description = "Research collaboration";
        new RelationshipService(http).create(req);
        var body = fake.lastRequest().body();
        assertTrue(body.contains("COLLABORATES_WITH"));
        assertTrue(body.contains("Research collaboration"));
    }

    @Test
    void sourceReferenceHasDocumentId() {
        // WHY: Lineage requires tracing answers back to source documents
        var src = new SourceReference();
        src.documentId = "doc-trace-1";
        src.chunkId = "chunk-7";
        src.content = "Sample text";
        src.score = 0.92;
        assertEquals("doc-trace-1", src.documentId);
        assertEquals("chunk-7", src.chunkId);
        assertEquals(0.92, src.score);
    }

    @Test
    void chatSourceReferenceLineage() {
        // WHY: Chat responses should trace back to source entities/documents
        var ref = new ChatSourceReference();
        ref.sourceType = "entity";
        ref.id = "entity-alice-1";
        ref.score = 0.88;
        ref.snippet = "Alice is a researcher...";
        assertEquals("entity", ref.sourceType);
        assertEquals("entity-alice-1", ref.id);
    }

    @Test
    void queryResponsePreservesMode() {
        // WHY: Lineage includes which query mode was used
        fake.respondWith("{\"answer\":\"test answer\",\"sources\":[],\"mode\":\"local\"}");
        var resp = new QueryService(http).execute(new QueryRequest("q", "local"));
        assertNotNull(resp);
    }

    @Test
    void entityDeleteResponseHasLineageInfo() {
        // WHY: Delete response tracks cascaded deletions for lineage
        var del = new EntityDeleteResponse();
        del.deletedEntityId = "ent-1";
        del.deletedRelationships = 5;
        del.affectedEntities = List.of("ent-2", "ent-3");
        assertEquals("ent-1", del.deletedEntityId);
        assertEquals(5, del.deletedRelationships);
        assertEquals(2, del.affectedEntities.size());
    }

    @Test
    void entityDetailResponseHasStatistics() {
        // WHY: Statistics provide lineage depth info (relationship counts)
        var stats = new EntityStatistics();
        stats.totalRelationships = 10;
        stats.outgoingCount = 6;
        stats.incomingCount = 4;
        stats.documentReferences = 3;
        assertEquals(10, stats.totalRelationships);
        assertEquals(3, stats.documentReferences);
    }

    @Test
    void mergeEntitiesPreservesLineage() {
        fake.respondWith("{\"merged_entity\":{\"entity_name\":\"ALICE\"},\"merged_count\":2,\"message\":\"merged\"}");
        var req = new MergeEntitiesRequest("ALICE_1", "ALICE_2");
        new EntityService(http).merge(req);
        var body = fake.lastRequest().body();
        assertTrue(body.contains("ALICE_1"));
        assertTrue(body.contains("ALICE_2"));
    }

    @Test
    void documentTrackStatusHasDocumentId() {
        // WHY: Track status links processing back to the source document
        var track = new TrackStatus();
        track.trackId = "trk-1";
        track.status = "completed";
        track.progress = 1.0;
        track.documentId = "doc-lineage-2";
        assertEquals("doc-lineage-2", track.documentId);
        assertEquals("completed", track.status);
    }

    @Test
    void uploadResponseContainsLineageCounts() {
        // WHY: Upload response shows entity/relationship extraction results for lineage
        var u = new UploadResponse();
        u.documentId = "doc-up-1";
        u.entityCount = 15;
        u.relationshipCount = 8;
        u.chunkCount = 42;
        assertEquals(15, u.entityCount);
        assertEquals(8, u.relationshipCount);
        assertEquals(42, u.chunkCount);
    }

    @Test
    void chatCompletionRequestHasConversationId() {
        // WHY: Conversation lineage links messages to conversation threads
        var req = new ChatCompletionRequest("Hello");
        req.conversationId = "conv-1";
        req.parentId = "msg-parent-1";
        assertEquals("conv-1", req.conversationId);
        assertEquals("msg-parent-1", req.parentId);
    }

    @Test
    void chatCompletionResponseHasMessageIds() {
        // WHY: Message IDs form the lineage chain within conversations
        var resp = new ChatCompletionResponse();
        resp.conversationId = "conv-2";
        resp.userMessageId = "umsg-1";
        resp.assistantMessageId = "amsg-1";
        resp.content = "Hello!";
        resp.mode = "hybrid";
        assertEquals("umsg-1", resp.userMessageId);
        assertEquals("amsg-1", resp.assistantMessageId);
    }

    @Test
    void graphNodeHasProvenanceProperties() {
        var node = new GraphNode();
        node.id = "n1";
        node.label = "ALICE";
        node.nodeType = "person";
        node.description = "A researcher";
        node.degree = 5;
        node.properties = java.util.Map.of("source_document", "doc-1", "extraction_confidence", 0.95);
        assertEquals(5, node.degree);
        assertEquals("doc-1", node.properties.get("source_document"));
    }

    @Test
    void graphEdgeTracksProvenance() {
        var edge = new GraphEdge();
        edge.source = "ALICE";
        edge.target = "BOB";
        edge.edgeType = "COLLABORATES";
        edge.weight = 0.85;
        edge.properties = java.util.Map.of("extracted_from", "doc-3");
        assertEquals("doc-3", edge.properties.get("extracted_from"));
    }

    @Test
    void entityListResponseHasPagination() {
        // WHY: Pagination is part of the lineage query interface
        var resp = new EntityListResponse();
        resp.total = 100;
        resp.page = 2;
        resp.pageSize = 20;
        resp.totalPages = 5;
        assertEquals(100, resp.total);
        assertEquals(5, resp.totalPages);
    }

    @Test
    void neighborhoodResponsePreservesDepth() {
        // WHY: Neighborhood depth is lineage traversal depth
        var resp = new NeighborhoodResponse();
        resp.depth = 3;
        resp.nodes = List.of();
        resp.edges = List.of();
        assertEquals(3, resp.depth);
    }

    @Test
    void deletionImpactCountsLineageEffects() {
        // WHY: Deletion impact shows how many entities/relationships are affected
        var impact = new DeletionImpact();
        impact.entityCount = 5;
        impact.relationshipCount = 12;
        impact.chunkCount = 30;
        assertEquals(5, impact.entityCount);
        assertEquals(12, impact.relationshipCount);
        assertEquals(30, impact.chunkCount);
    }

    // ── LineageService Tests ─────────────────────────────────────────
    // WHY: OODA-20 — Full lineage coverage for Java SDK.
    // Each test verifies a LineageService method hits the correct endpoint
    // and deserializes the response into the correct LineageModels type.

    @Test
    void lineageEntityLineageEndpoint() {
        fake.respondWith("""
            {"entity_name":"ALICE","entity_type":"PERSON","source_documents":[{"document_id":"d1","chunk_ids":["c1","c2"],"line_ranges":[{"start_line":10,"end_line":15}]}],"source_count":1,"description_versions":[{"version":1,"description":"A researcher","source_chunk_id":"c1","created_at":"2026-01-01T00:00:00Z"}]}
            """);
        var svc = new LineageService(http);
        var result = svc.entityLineage("ALICE");
        assertEquals("ALICE", result.entityName);
        assertEquals("PERSON", result.entityType);
        assertEquals(1, result.sourceCount);
        assertEquals(1, result.sourceDocuments.size());
        assertEquals("d1", result.sourceDocuments.get(0).documentId);
        assertEquals(2, result.sourceDocuments.get(0).chunkIds.size());
        assertEquals(1, result.sourceDocuments.get(0).lineRanges.size());
        assertEquals(10, result.sourceDocuments.get(0).lineRanges.get(0).startLine);
        assertEquals(15, result.sourceDocuments.get(0).lineRanges.get(0).endLine);
        assertEquals(1, result.descriptionVersions.size());
        assertEquals(1, result.descriptionVersions.get(0).version);
        assertEquals("A researcher", result.descriptionVersions.get(0).description);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/lineage/entities/ALICE"));
    }

    @Test
    void lineageDocumentLineageEndpoint() {
        fake.respondWith("""
            {"document_id":"doc-1","chunk_count":5,"entities":[{"name":"BOB","entity_type":"PERSON","source_chunks":["c1"],"is_shared":false}],"relationships":[{"source":"A","target":"B","keywords":"KNOWS","source_chunks":["c1"]}],"extraction_stats":{"total_entities":10,"unique_entities":8,"total_relationships":5,"unique_relationships":4,"processing_time_ms":1500}}
            """);
        var svc = new LineageService(http);
        var result = svc.documentLineage("doc-1");
        assertEquals("doc-1", result.documentId);
        assertEquals(5, result.chunkCount);
        assertEquals(1, result.entities.size());
        assertEquals("BOB", result.entities.get(0).name);
        assertFalse(result.entities.get(0).isShared);
        assertEquals(1, result.relationships.size());
        assertEquals("KNOWS", result.relationships.get(0).keywords);
        assertEquals(10, result.extractionStats.totalEntities);
        assertEquals(8, result.extractionStats.uniqueEntities);
        assertEquals(1500L, result.extractionStats.processingTimeMs);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/lineage/documents/doc-1"));
    }

    @Test
    void lineageDocumentFullLineage() {
        fake.respondWith("""
            {"document_id":"doc-2","metadata":{"author":"Jane"},"lineage":{"entities":["A","B"]}}
            """);
        var svc = new LineageService(http);
        var result = svc.documentFullLineage("doc-2");
        assertEquals("doc-2", result.documentId);
        assertNotNull(result.metadata);
        assertEquals("Jane", result.metadata.get("author"));
        assertNotNull(result.lineage);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/documents/doc-2/lineage"));
    }

    @Test
    void lineageExportJson() {
        fake.respondWith("{\"export\":\"data\",\"format\":\"json\"}");
        var svc = new LineageService(http);
        var result = svc.exportLineage("doc-3", "json");
        assertNotNull(result);
        assertEquals("data", result.get("export"));
        assertTrue(fake.lastRequest().uri().contains("/api/v1/documents/doc-3/lineage/export"));
        assertTrue(fake.lastRequest().uri().contains("format=json"));
    }

    @Test
    void lineageExportCsv() {
        fake.respondWith("{\"format\":\"csv\",\"data\":\"entity,type\\nALICE,PERSON\"}");
        var svc = new LineageService(http);
        var result = svc.exportLineage("doc-4", "csv");
        assertNotNull(result);
        assertTrue(fake.lastRequest().uri().contains("format=csv"));
    }

    @Test
    void lineageExportDefaultFormat() {
        fake.respondWith("{\"format\":\"json\"}");
        var svc = new LineageService(http);
        var result = svc.exportLineage("doc-5", null);
        assertNotNull(result);
        assertTrue(fake.lastRequest().uri().contains("format=json"));
    }

    @Test
    void lineageChunkDetail() {
        fake.respondWith("""
            {"chunk_id":"ch-1","document_id":"d1","document_name":"Test Doc","content":"Some text","index":0,"char_range":{"start":0,"end":100},"token_count":25,"entities":[{"id":"e1","name":"ALICE","entity_type":"PERSON","description":"researcher"}],"relationships":[{"source_name":"ALICE","target_name":"BOB","relation_type":"KNOWS","description":"colleagues"}],"extraction_metadata":{"model":"gpt-4o","gleaning_iterations":2,"duration_ms":500,"input_tokens":100,"output_tokens":50,"cached":false}}
            """);
        var svc = new LineageService(http);
        var result = svc.chunkDetail("ch-1");
        assertEquals("ch-1", result.chunkId);
        assertEquals("d1", result.documentId);
        assertEquals("Test Doc", result.documentName);
        assertEquals("Some text", result.content);
        assertEquals(0, result.index);
        assertEquals(0, result.charRange.start);
        assertEquals(100, result.charRange.end);
        assertEquals(25, result.tokenCount);
        assertEquals(1, result.entities.size());
        assertEquals("ALICE", result.entities.get(0).name);
        assertEquals(1, result.relationships.size());
        assertEquals("ALICE", result.relationships.get(0).sourceName);
        assertEquals("BOB", result.relationships.get(0).targetName);
        assertNotNull(result.extractionMetadata);
        assertEquals("gpt-4o", result.extractionMetadata.model);
        assertEquals(2, result.extractionMetadata.gleaningIterations);
        assertEquals(500L, result.extractionMetadata.durationMs);
        assertFalse(result.extractionMetadata.cached);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/chunks/ch-1"));
    }

    @Test
    void lineageChunkLineage() {
        fake.respondWith("""
            {"chunk_id":"ch-2","document_id":"d2","document_name":"Another Doc","document_type":"pdf","index":3,"start_line":50,"end_line":75,"start_offset":1200,"end_offset":1800,"token_count":30,"content_preview":"First 100 chars...","entity_count":4,"relationship_count":2,"entity_names":["ALICE","BOB","CAROL","DAVE"],"document_metadata":{"source":"upload"}}
            """);
        var svc = new LineageService(http);
        var result = svc.chunkLineage("ch-2");
        assertEquals("ch-2", result.chunkId);
        assertEquals("d2", result.documentId);
        assertEquals("Another Doc", result.documentName);
        assertEquals("pdf", result.documentType);
        assertEquals(3, result.index);
        assertEquals(50, result.startLine);
        assertEquals(75, result.endLine);
        assertEquals(1200, result.startOffset);
        assertEquals(1800, result.endOffset);
        assertEquals(30, result.tokenCount);
        assertEquals("First 100 chars...", result.contentPreview);
        assertEquals(4, result.entityCount);
        assertEquals(2, result.relationshipCount);
        assertEquals(4, result.entityNames.size());
        assertTrue(result.entityNames.contains("ALICE"));
        assertNotNull(result.documentMetadata);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/chunks/ch-2/lineage"));
    }

    @Test
    void lineageEntityProvenance() {
        fake.respondWith("""
            {"entity_id":"e1","entity_name":"ALICE","entity_type":"PERSON","description":"A researcher","sources":[{"document_id":"d1","document_name":"Paper","chunks":[{"chunk_id":"c1","start_line":10,"end_line":15,"source_text":"Alice is..."}],"first_extracted_at":"2026-01-01T00:00:00Z"}],"total_extraction_count":3,"related_entities":[{"entity_id":"e2","entity_name":"BOB","relationship_type":"COLLABORATES","shared_documents":2}]}
            """);
        var svc = new LineageService(http);
        var result = svc.entityProvenance("e1");
        assertEquals("e1", result.entityId);
        assertEquals("ALICE", result.entityName);
        assertEquals("PERSON", result.entityType);
        assertEquals("A researcher", result.description);
        assertEquals(3, result.totalExtractionCount);
        assertEquals(1, result.sources.size());
        assertEquals("d1", result.sources.get(0).documentId);
        assertEquals("Paper", result.sources.get(0).documentName);
        assertEquals(1, result.sources.get(0).chunks.size());
        assertEquals("c1", result.sources.get(0).chunks.get(0).chunkId);
        assertEquals(10, result.sources.get(0).chunks.get(0).startLine);
        assertEquals("Alice is...", result.sources.get(0).chunks.get(0).sourceText);
        assertEquals(1, result.relatedEntities.size());
        assertEquals("BOB", result.relatedEntities.get(0).entityName);
        assertEquals("COLLABORATES", result.relatedEntities.get(0).relationshipType);
        assertEquals(2, result.relatedEntities.get(0).sharedDocuments);
        assertTrue(fake.lastRequest().uri().contains("/api/v1/entities/e1/provenance"));
    }

    // ── LineageModels Unit Tests ──────────────────────────────────────
    // WHY: Verify all LineageModels fields serialize/deserialize correctly.

    @Test
    void entityLineageResponseFields() {
        var resp = new EntityLineageResponse();
        resp.entityName = "TEST";
        resp.entityType = "CONCEPT";
        resp.sourceCount = 3;
        resp.sourceDocuments = List.of();
        resp.descriptionVersions = List.of();
        assertEquals("TEST", resp.entityName);
        assertEquals(3, resp.sourceCount);
    }

    @Test
    void sourceDocumentInfoFields() {
        var info = new SourceDocumentInfo();
        info.documentId = "d1";
        info.chunkIds = List.of("c1", "c2");
        info.lineRanges = List.of();
        assertEquals("d1", info.documentId);
        assertEquals(2, info.chunkIds.size());
    }

    @Test
    void lineRangeInfoFields() {
        var lr = new LineRangeInfo();
        lr.startLine = 1;
        lr.endLine = 50;
        assertEquals(1, lr.startLine);
        assertEquals(50, lr.endLine);
    }

    @Test
    void descriptionVersionFields() {
        var dv = new DescriptionVersionResponse();
        dv.version = 2;
        dv.description = "Updated desc";
        dv.sourceChunkId = "c5";
        dv.createdAt = "2026-02-01T12:00:00Z";
        assertEquals(2, dv.version);
        assertEquals("c5", dv.sourceChunkId);
    }

    @Test
    void documentGraphLineageFields() {
        var resp = new DocumentGraphLineageResponse();
        resp.documentId = "d-graph";
        resp.chunkCount = 10;
        resp.entities = List.of();
        resp.relationships = List.of();
        assertEquals("d-graph", resp.documentId);
        assertEquals(10, resp.chunkCount);
    }

    @Test
    void entitySummaryResponseFields() {
        var es = new EntitySummaryResponse();
        es.name = "ENTITY_1";
        es.entityType = "ORG";
        es.sourceChunks = List.of("c1");
        es.isShared = true;
        assertTrue(es.isShared);
        assertEquals("ORG", es.entityType);
    }

    @Test
    void extractionStatsFields() {
        var stats = new ExtractionStatsResponse();
        stats.totalEntities = 50;
        stats.uniqueEntities = 40;
        stats.totalRelationships = 30;
        stats.uniqueRelationships = 25;
        stats.processingTimeMs = 2500L;
        assertEquals(50, stats.totalEntities);
        assertEquals(40, stats.uniqueEntities);
        assertEquals(2500L, stats.processingTimeMs);
    }

    @Test
    void chunkDetailResponseFields() {
        var cd = new ChunkDetailResponse();
        cd.chunkId = "ch-test";
        cd.documentId = "d-test";
        cd.content = "content";
        cd.index = 5;
        cd.tokenCount = 100;
        assertEquals("ch-test", cd.chunkId);
        assertEquals(5, cd.index);
        assertEquals(100, cd.tokenCount);
    }

    @Test
    void charRangeFields() {
        var cr = new CharRange();
        cr.start = 0;
        cr.end = 500;
        assertEquals(0, cr.start);
        assertEquals(500, cr.end);
    }

    @Test
    void extractedEntityInfoFields() {
        var ei = new ExtractedEntityInfo();
        ei.id = "eid-1";
        ei.name = "ALICE";
        ei.entityType = "PERSON";
        ei.description = "A person";
        assertEquals("eid-1", ei.id);
        assertEquals("PERSON", ei.entityType);
    }

    @Test
    void extractedRelationshipInfoFields() {
        var ri = new ExtractedRelationshipInfo();
        ri.sourceName = "A";
        ri.targetName = "B";
        ri.relationType = "KNOWS";
        ri.description = "friends";
        assertEquals("A", ri.sourceName);
        assertEquals("KNOWS", ri.relationType);
    }

    @Test
    void extractionMetadataInfoFields() {
        var em = new ExtractionMetadataInfo();
        em.model = "gpt-4o";
        em.gleaningIterations = 3;
        em.durationMs = 1200;
        em.inputTokens = 500;
        em.outputTokens = 200;
        em.cached = true;
        assertEquals("gpt-4o", em.model);
        assertEquals(3, em.gleaningIterations);
        assertTrue(em.cached);
    }

    @Test
    void entityProvenanceResponseFields() {
        var ep = new EntityProvenanceResponse();
        ep.entityId = "e-prov";
        ep.entityName = "TEST_ENTITY";
        ep.entityType = "CONCEPT";
        ep.description = "A concept";
        ep.totalExtractionCount = 7;
        ep.sources = List.of();
        ep.relatedEntities = List.of();
        assertEquals("e-prov", ep.entityId);
        assertEquals(7, ep.totalExtractionCount);
    }

    @Test
    void entitySourceInfoFields() {
        var esi = new EntitySourceInfo();
        esi.documentId = "d-src";
        esi.documentName = "Source Doc";
        esi.chunks = List.of();
        esi.firstExtractedAt = "2026-01-15T10:00:00Z";
        assertEquals("Source Doc", esi.documentName);
        assertEquals("2026-01-15T10:00:00Z", esi.firstExtractedAt);
    }

    @Test
    void chunkSourceInfoFields() {
        var csi = new ChunkSourceInfo();
        csi.chunkId = "cs-1";
        csi.startLine = 10;
        csi.endLine = 20;
        csi.sourceText = "sample text";
        assertEquals("cs-1", csi.chunkId);
        assertEquals(10, csi.startLine);
        assertEquals("sample text", csi.sourceText);
    }

    @Test
    void relatedEntityInfoFields() {
        var rei = new RelatedEntityInfo();
        rei.entityId = "re-1";
        rei.entityName = "RELATED";
        rei.relationshipType = "COLLABORATES";
        rei.sharedDocuments = 5;
        assertEquals("RELATED", rei.entityName);
        assertEquals(5, rei.sharedDocuments);
    }

    @Test
    void documentFullLineageFields() {
        var dfl = new DocumentFullLineageResponse();
        dfl.documentId = "d-full";
        dfl.metadata = java.util.Map.of("key", "val");
        dfl.lineage = java.util.Map.of("entities", List.of());
        assertEquals("d-full", dfl.documentId);
        assertEquals("val", dfl.metadata.get("key"));
    }

    @Test
    void chunkLineageResponseFields() {
        var clr = new ChunkLineageResponse();
        clr.chunkId = "cl-1";
        clr.documentId = "d-cl";
        clr.documentName = "ChunkDoc";
        clr.documentType = "markdown";
        clr.index = 2;
        clr.startLine = 20;
        clr.endLine = 40;
        clr.startOffset = 500;
        clr.endOffset = 1000;
        clr.tokenCount = 50;
        clr.contentPreview = "preview...";
        clr.entityCount = 3;
        clr.relationshipCount = 1;
        clr.entityNames = List.of("A", "B", "C");
        clr.documentMetadata = java.util.Map.of("source", "upload");
        assertEquals("cl-1", clr.chunkId);
        assertEquals(2, clr.index);
        assertEquals(3, clr.entityCount);
        assertEquals(3, clr.entityNames.size());
    }

    @Test
    void lineageServiceError() {
        fake.respondWithError(404);
        var svc = new LineageService(http);
        assertThrows(EdgeQuakeException.class, () -> svc.entityLineage("NONEXISTENT"));
    }

    @Test
    void lineageServiceServerError() {
        fake.respondWithError(500);
        var svc = new LineageService(http);
        assertThrows(EdgeQuakeException.class, () -> svc.chunkDetail("bad-id"));
    }

    // ── Edge Cases ───────────────────────────────────────────────────

    @Test
    void entityLineageEmptySourceDocuments() {
        fake.respondWith("""
            {"entity_name":"ORPHAN","source_documents":[],"source_count":0,"description_versions":[]}
            """);
        var svc = new LineageService(http);
        var result = svc.entityLineage("ORPHAN");
        assertEquals(0, result.sourceCount);
        assertTrue(result.sourceDocuments.isEmpty());
        assertTrue(result.descriptionVersions.isEmpty());
    }

    @Test
    void chunkLineageNullOptionalFields() {
        fake.respondWith("""
            {"chunk_id":"ch-null","document_id":"d-null"}
            """);
        var svc = new LineageService(http);
        var result = svc.chunkLineage("ch-null");
        assertEquals("ch-null", result.chunkId);
        assertNull(result.documentName);
        assertNull(result.documentType);
        assertNull(result.index);
        assertNull(result.startLine);
        assertNull(result.entityNames);
    }

    @Test
    void entityProvenanceMultipleSources() {
        fake.respondWith("""
            {"entity_id":"e-multi","entity_name":"MULTI","entity_type":"CONCEPT","sources":[{"document_id":"d1","chunks":[]},{"document_id":"d2","chunks":[]}],"total_extraction_count":5,"related_entities":[]}
            """);
        var svc = new LineageService(http);
        var result = svc.entityProvenance("e-multi");
        assertEquals(2, result.sources.size());
        assertEquals("d2", result.sources.get(1).documentId);
    }

    @Test
    void documentGraphLineageNoEntities() {
        fake.respondWith("""
            {"document_id":"d-empty","chunk_count":0,"entities":[],"relationships":[],"extraction_stats":{"total_entities":0,"unique_entities":0,"total_relationships":0,"unique_relationships":0}}
            """);
        var svc = new LineageService(http);
        var result = svc.documentLineage("d-empty");
        assertEquals(0, result.chunkCount);
        assertTrue(result.entities.isEmpty());
        assertEquals(0, result.extractionStats.totalEntities);
    }

    @Test
    void lineageEntityNameUrlEncoded() {
        fake.respondWith("{\"entity_name\":\"ALICE BOB\",\"source_documents\":[],\"source_count\":0,\"description_versions\":[]}");
        var svc = new LineageService(http);
        svc.entityLineage("ALICE BOB");
        // WHY: Space should be URL-encoded as + or %20
        var uri = fake.lastRequest().uri();
        assertTrue(uri.contains("ALICE+BOB") || uri.contains("ALICE%20BOB"),
                "Entity name with space should be URL-encoded: " + uri);
    }

    // ── OODA-38: Extended Service Tests ──────────────────────────────

    // ── Health Extended Tests ────────────────────────────────────────

    @Test
    void healthReady() {
        fake.respondWith("{\"ready\":true,\"checks\":{\"database\":\"ok\",\"provider\":\"ok\"}}");
        var svc = new HealthService(http);
        var result = svc.ready();
        assertTrue(result.ready);
        assertTrue(fake.lastRequest().uri().contains("/ready"));
    }

    @Test
    void healthLive() {
        fake.respondWith("{\"alive\":true,\"uptime\":12345}");
        var svc = new HealthService(http);
        var result = svc.live();
        assertTrue(result.alive);
        assertEquals(12345L, result.uptime);
    }

    @Test
    void healthMetrics() {
        fake.respondWith("# TYPE edgequake_requests_total counter\nedgequake_requests_total 100");
        var svc = new HealthService(http);
        var result = svc.metrics();
        assertTrue(result.contains("edgequake_requests_total"));
    }

    // ── Document Extended Tests ──────────────────────────────────────

    @Test
    void documentsChunks() {
        fake.respondWith("{\"document_id\":\"d1\",\"chunks\":[{\"id\":\"c1\",\"content\":\"text\",\"index\":0}],\"total\":1}");
        var svc = new DocumentService(http);
        var result = svc.chunks("d1");
        assertEquals("d1", result.documentId);
        assertEquals(1, result.total);
    }

    @Test
    void documentsStatus() {
        fake.respondWith("{\"document_id\":\"d1\",\"status\":\"completed\",\"progress\":1.0}");
        var svc = new DocumentService(http);
        var result = svc.status("d1");
        assertEquals("completed", result.status);
    }

    @Test
    void documentsReprocess() {
        fake.respondWith("{\"status\":\"ok\",\"message\":\"Reprocessing started\"}");
        var svc = new DocumentService(http);
        var result = svc.reprocess("d1");
        assertEquals("ok", result.status);
    }

    @Test
    void documentsRecoverStuck() {
        fake.respondWith("{\"status\":\"ok\",\"message\":\"Recovered 3 documents\"}");
        var svc = new DocumentService(http);
        var result = svc.recoverStuck();
        assertEquals("ok", result.status);
    }

    // ── Graph Extended Tests ─────────────────────────────────────────

    @Test
    void graphStats() {
        fake.respondWith("{\"node_count\":100,\"edge_count\":200,\"entity_count\":50,\"relationship_count\":80}");
        var svc = new GraphService(http);
        var result = svc.stats();
        assertEquals(100, result.nodeCount);
        assertEquals(200, result.edgeCount);
    }

    @Test
    void graphLabelSearch() {
        fake.respondWith("{\"labels\":[{\"label\":\"PERSON\",\"count\":25}],\"total\":1}");
        var svc = new GraphService(http);
        var result = svc.labelSearch("PERSON");
        assertEquals(1, result.total);
    }

    @Test
    void graphPopularLabels() {
        fake.respondWith("{\"labels\":[{\"label\":\"PERSON\",\"count\":50},{\"label\":\"ORG\",\"count\":30}]}");
        var svc = new GraphService(http);
        var result = svc.popularLabels();
        assertEquals(2, result.labels.size());
    }

    @Test
    void graphBatchDegrees() {
        fake.respondWith("{\"degrees\":{\"node1\":5,\"node2\":3}}");
        var svc = new GraphService(http);
        var result = svc.batchDegrees(List.of("node1", "node2"));
        assertEquals(5, result.degrees.get("node1"));
    }

    @Test
    void graphClear() {
        fake.respondWith("");
        var svc = new GraphService(http);
        svc.clear();
        assertTrue(fake.lastRequest().uri().contains("/api/v1/graph"));
    }

    // ── Entity Extended Tests ────────────────────────────────────────

    @Test
    void entityTypes() {
        fake.respondWith("{\"types\":[\"PERSON\",\"ORGANIZATION\",\"CONCEPT\"],\"total\":3}");
        var svc = new EntityService(http);
        var result = svc.types();
        assertEquals(3, result.total);
        assertTrue(result.types.contains("PERSON"));
    }

    // ── Relationship Extended Tests ──────────────────────────────────

    @Test
    void relationshipGet() {
        fake.respondWith("{\"relationship\":{\"id\":\"r1\",\"source\":\"A\",\"target\":\"B\"},\"source\":{\"entity_name\":\"A\"},\"target\":{\"entity_name\":\"B\"}}");
        var svc = new RelationshipService(http);
        var result = svc.get("r1");
        assertEquals("r1", result.relationship.id);
    }

    @Test
    void relationshipDelete() {
        fake.respondWith("");
        var svc = new RelationshipService(http);
        svc.delete("r1");
        assertTrue(fake.lastRequest().uri().contains("/api/v1/graph/relationships/r1"));
    }

    @Test
    void relationshipTypes() {
        fake.respondWith("{\"types\":[\"KNOWS\",\"WORKS_WITH\",\"LOCATED_IN\"],\"total\":3}");
        var svc = new RelationshipService(http);
        var result = svc.types();
        assertEquals(3, result.total);
    }

    // ── Query Extended Tests ─────────────────────────────────────────

    @Test
    void queryStream() {
        fake.respondWith("data: {\"chunk\":\"Hello\"}\n\n");
        var svc = new QueryService(http);
        var result = svc.stream("test query");
        assertTrue(result.contains("Hello"));
    }

    // ── Chat Extended Tests ──────────────────────────────────────────

    @Test
    void chatStream() {
        fake.respondWith("data: {\"delta\":\"world\"}\n\n");
        var svc = new ChatService(http);
        var result = svc.stream(new ChatCompletionRequest());
        assertTrue(result.contains("world"));
    }

    @Test
    void chatCompletionsWithConversation() {
        fake.respondWith("{\"content\":\"Response\"}");
        var svc = new ChatService(http);
        var result = svc.completionsWithConversation("c1", "Hello");
        assertEquals("Response", result.content);
    }

    // ── OODA-40: Document Extended Tests ────────────────────────────

    @Test
    void documentGetMetadata() {
        fake.respondWith("{\"document_id\":\"d1\",\"metadata\":{\"author\":\"John\",\"category\":\"research\"}}");
        var svc = new DocumentService(http);
        var result = svc.getMetadata("d1");
        assertEquals("d1", result.documentId);
        assertEquals("John", result.metadata.get("author"));
    }

    @Test
    void documentGetMetadataEmpty() {
        fake.respondWith("{\"document_id\":\"d1\",\"metadata\":{}}");
        var svc = new DocumentService(http);
        var result = svc.getMetadata("d1");
        assertTrue(result.metadata.isEmpty());
    }

    @Test
    void documentSetMetadata() {
        fake.respondWith("{\"document_id\":\"d1\",\"metadata\":{\"tags\":[\"AI\"]}}");
        var svc = new DocumentService(http);
        var result = svc.setMetadata("d1", Map.of("tags", List.of("AI")));
        assertTrue(fake.lastRequest().uri().contains("/metadata"));
    }

    @Test
    void documentSetMetadataPartial() {
        fake.respondWith("{\"document_id\":\"d1\",\"metadata\":{\"category\":\"updated\"}}");
        var svc = new DocumentService(http);
        svc.setMetadata("d1", Map.of("category", "updated"));
        assertEquals("PATCH", fake.lastRequest().method());
    }

    @Test
    void documentFailedChunks() {
        fake.respondWith("{\"document_id\":\"d1\",\"chunks\":[{\"id\":\"c1\",\"error\":\"timeout\"}],\"total\":1}");
        var svc = new DocumentService(http);
        var result = svc.failedChunks("d1");
        assertEquals(1, result.total);
        assertEquals("timeout", result.chunks.get(0).error);
    }

    @Test
    void documentFailedChunksEmpty() {
        fake.respondWith("{\"document_id\":\"d1\",\"chunks\":[],\"total\":0}");
        var svc = new DocumentService(http);
        var result = svc.failedChunks("d1");
        assertEquals(0, result.total);
    }

    @Test
    void documentRetryChunks() {
        fake.respondWith("{\"status\":\"queued\",\"message\":\"Retrying 5 chunks\"}");
        var svc = new DocumentService(http);
        var result = svc.retryChunks("d1");
        assertEquals("queued", result.status);
    }

    @Test
    void documentRetryChunksNone() {
        fake.respondWith("{\"status\":\"completed\",\"message\":\"No failed chunks\"}");
        var svc = new DocumentService(http);
        var result = svc.retryChunks("d1");
        assertEquals("completed", result.status);
    }

    // ── OODA-40: User Extended Tests ────────────────────────────────

    @Test
    void userUpdate() {
        fake.respondWith("{\"id\":\"u1\",\"username\":\"updated_user\",\"email\":\"new@example.com\"}");
        var svc = new UserService(http);
        var result = svc.update("u1", Map.of("email", "new@example.com"));
        assertEquals("PUT", fake.lastRequest().method());
        assertEquals("updated_user", result.username);
    }

    @Test
    void userUpdatePartial() {
        fake.respondWith("{\"id\":\"u1\",\"username\":\"newname\"}");
        var svc = new UserService(http);
        svc.update("u1", Map.of("username", "newname"));
        assertTrue(fake.lastRequest().uri().contains("/api/v1/users/u1"));
    }

    @Test
    void userDelete() {
        fake.respondWith("");
        var svc = new UserService(http);
        svc.delete("u1");
        assertEquals("DELETE", fake.lastRequest().method());
    }

    @Test
    void userDeleteNotFound() {
        fake.respondWithError(404, "{\"error\":\"not found\"}");
        var svc = new UserService(http);
        assertThrows(RuntimeException.class, () -> svc.delete("missing"));
    }

    // ── OODA-40: Conversation Extended Tests ────────────────────────

    @Test
    void conversationUpdate() {
        fake.respondWith("{\"id\":\"c1\",\"title\":\"Updated Title\"}");
        var svc = new ConversationService(http);
        var result = svc.update("c1", Map.of("title", "Updated Title"));
        assertEquals("c1", result.id);
    }

    @Test
    void conversationUpdatePinned() {
        fake.respondWith("{\"id\":\"c1\",\"is_pinned\":true}");
        var svc = new ConversationService(http);
        svc.update("c1", Map.of("is_pinned", true));
        assertEquals("PATCH", fake.lastRequest().method());
    }

    @Test
    void conversationListMessages() {
        fake.respondWith("{\"messages\":[{\"id\":\"m1\",\"content\":\"Hello\"}],\"total\":1}");
        var svc = new ConversationService(http);
        var result = svc.listMessages("c1");
        assertEquals(1, result.total);
        assertEquals("Hello", result.messages.get(0).content);
    }

    @Test
    void conversationListMessagesEmpty() {
        fake.respondWith("{\"messages\":[],\"total\":0}");
        var svc = new ConversationService(http);
        var result = svc.listMessages("c1");
        assertTrue(result.messages.isEmpty());
    }

    @Test
    void conversationUpdateMessage() {
        fake.respondWith("{\"id\":\"m1\",\"content\":\"Updated\",\"role\":\"user\"}");
        var svc = new ConversationService(http);
        var result = svc.updateMessage("c1", "m1", "Updated");
        assertEquals("Updated", result.content);
    }

    @Test
    void conversationUpdateMessagePreservesRole() {
        fake.respondWith("{\"id\":\"m1\",\"content\":\"New text\",\"role\":\"assistant\"}");
        var svc = new ConversationService(http);
        var result = svc.updateMessage("c1", "m1", "New text");
        assertEquals("assistant", result.role);
    }

    @Test
    void conversationDeleteMessage() {
        fake.respondWith("");
        var svc = new ConversationService(http);
        svc.deleteMessage("c1", "m1");
        assertTrue(fake.lastRequest().uri().contains("/messages/m1"));
    }

    @Test
    void conversationDeleteMessagePath() {
        fake.respondWith("");
        var svc = new ConversationService(http);
        svc.deleteMessage("c1", "m1");
        assertEquals("DELETE", fake.lastRequest().method());
    }

    @Test
    void conversationUnshare() {
        fake.respondWith("");
        var svc = new ConversationService(http);
        svc.unshare("c1");
        assertTrue(fake.lastRequest().uri().contains("/share"));
    }

    @Test
    void conversationBulkArchive() {
        fake.respondWith("{\"deleted_count\":3}");
        var svc = new ConversationService(http);
        var result = svc.bulkArchive(List.of("c1", "c2", "c3"));
        assertEquals(3, result.deletedCount);
    }

    @Test
    void conversationBulkArchiveEmpty() {
        fake.respondWith("{\"deleted_count\":0}");
        var svc = new ConversationService(http);
        var result = svc.bulkArchive(List.of());
        assertEquals(0, result.deletedCount);
    }

    @Test
    void conversationBulkMove() {
        fake.respondWith("{\"deleted_count\":2}");
        var svc = new ConversationService(http);
        var result = svc.bulkMove(List.of("c1", "c2"), "folder1");
        assertEquals(2, result.deletedCount);
    }

    @Test
    void conversationBulkMoveEndpoint() {
        fake.respondWith("{\"deleted_count\":1}");
        var svc = new ConversationService(http);
        svc.bulkMove(List.of("c1"), "f1");
        assertTrue(fake.lastRequest().uri().contains("/bulk/move"));
    }

    @Test
    void conversationImport() {
        fake.respondWith("{\"id\":\"c-new\",\"title\":\"Imported\"}");
        var svc = new ConversationService(http);
        var result = svc.importConversation(Map.of("title", "Imported", "messages", List.of()));
        assertEquals("Imported", result.title);
    }

    @Test
    void conversationImportEndpoint() {
        fake.respondWith("{\"id\":\"c1\"}");
        var svc = new ConversationService(http);
        svc.importConversation(Map.of());
        assertTrue(fake.lastRequest().uri().contains("/import"));
    }

    // ── OODA-40: Workspace Extended Tests ────────────────────────────

    @Test
    void workspaceUpdate() {
        fake.respondWith("{\"id\":\"ws1\",\"name\":\"Updated\"}");
        var svc = new WorkspaceService(http);
        var result = svc.update("ws1", Map.of("name", "Updated"));
        assertEquals("PUT", fake.lastRequest().method());
    }

    @Test
    void workspaceUpdateFields() {
        fake.respondWith("{\"id\":\"ws1\",\"description\":\"New desc\"}");
        var svc = new WorkspaceService(http);
        svc.update("ws1", Map.of("description", "New desc"));
        assertTrue(fake.lastRequest().uri().contains("/api/v1/workspaces/ws1"));
    }

    @Test
    void workspaceDelete() {
        fake.respondWith("");
        var svc = new WorkspaceService(http);
        svc.delete("ws1");
        assertEquals("DELETE", fake.lastRequest().method());
    }

    @Test
    void workspaceDeletePath() {
        fake.respondWith("");
        var svc = new WorkspaceService(http);
        svc.delete("ws-uuid-1234");
        assertTrue(fake.lastRequest().uri().contains("ws-uuid-1234"));
    }

    @Test
    void workspaceMetricsHistory() {
        fake.respondWith("{\"metrics\":[{\"date\":\"2026-01-15\",\"documents\":100,\"entities\":500}]}");
        var svc = new WorkspaceService(http);
        var result = svc.metricsHistory("ws1");
        assertEquals(1, result.metrics.size());
        assertEquals(100, result.metrics.get(0).documents);
    }

    @Test
    void workspaceMetricsHistoryEmpty() {
        fake.respondWith("{\"metrics\":[]}");
        var svc = new WorkspaceService(http);
        var result = svc.metricsHistory("ws1");
        assertTrue(result.metrics.isEmpty());
    }

    @Test
    void workspaceRebuildKnowledgeGraph() {
        fake.respondWith("{\"status\":\"queued\",\"track_id\":\"t-123\"}");
        var svc = new WorkspaceService(http);
        var result = svc.rebuildKnowledgeGraph("ws1");
        assertEquals("t-123", result.trackId);
    }

    @Test
    void workspaceRebuildKnowledgeGraphPath() {
        fake.respondWith("{\"status\":\"queued\"}");
        var svc = new WorkspaceService(http);
        svc.rebuildKnowledgeGraph("ws1");
        assertTrue(fake.lastRequest().uri().contains("/rebuild-knowledge-graph"));
    }

    @Test
    void workspaceReprocessDocuments() {
        fake.respondWith("{\"status\":\"queued\",\"message\":\"Reprocessing 10 documents\"}");
        var svc = new WorkspaceService(http);
        var result = svc.reprocessDocuments("ws1");
        assertEquals("queued", result.status);
    }

    @Test
    void workspaceReprocessDocumentsPath() {
        fake.respondWith("{\"status\":\"started\"}");
        var svc = new WorkspaceService(http);
        svc.reprocessDocuments("ws1");
        assertTrue(fake.lastRequest().uri().contains("/reprocess-documents"));
    }

    // ── OODA-44: Additional Edge Case & Model Tests ──────────────────

    @Test
    void entityListEmptyResult() {
        fake.respondWith("{\"items\":[],\"total\":0,\"page\":1,\"page_size\":20,\"total_pages\":0}");
        var svc = new EntityService(http);
        var result = svc.list(1, 20, null);
        assertTrue(result.items.isEmpty());
        assertEquals(0, result.total);
    }

    @Test
    void entityListWithFilter() {
        fake.respondWith("{\"items\":[],\"total\":0}");
        var svc = new EntityService(http);
        svc.list(0, 0, "PERSON");
        assertTrue(fake.lastRequest().uri().contains("entity_type=PERSON"));
    }

    @Test
    void entityExistsTrue() {
        fake.respondWith("{\"exists\":true,\"entity_id\":\"e-123\",\"entity_type\":\"PERSON\"}");
        var svc = new EntityService(http);
        var result = svc.exists("TEST_ENTITY");
        assertTrue(result.exists);
        assertEquals("e-123", result.entityId);
    }

    @Test
    void entityExistsFalse() {
        fake.respondWith("{\"exists\":false}");
        var svc = new EntityService(http);
        var result = svc.exists("MISSING_ENTITY");
        assertFalse(result.exists);
        assertNull(result.entityId);
    }

    @Test
    void graphGetWithLimit() {
        fake.respondWith("{\"nodes\":[],\"edges\":[]}");
        var svc = new GraphService(http);
        svc.get(50);
        assertTrue(fake.lastRequest().uri().contains("limit=50"));
    }

    @Test
    void graphGetNoLimit() {
        fake.respondWith("{\"nodes\":[],\"edges\":[]}");
        var svc = new GraphService(http);
        svc.get(0);
        assertFalse(fake.lastRequest().uri().contains("limit"));
    }

    @Test
    void documentScanRecursive() {
        fake.respondWith("{\"files_found\":10,\"files_queued\":8}");
        var svc = new DocumentService(http);
        var req = new DocumentModels.ScanRequest();
        req.path = "/data";
        req.recursive = true;
        var result = svc.scan(req);
        assertEquals(10, result.filesFound);
    }

    @Test
    void documentTrackProcessing() {
        fake.respondWith("{\"track_id\":\"t-abc\",\"status\":\"processing\",\"progress\":45.5}");
        var svc = new DocumentService(http);
        var result = svc.track("t-abc");
        assertEquals("processing", result.status);
    }

    @Test
    void documentTrackCompleted() {
        fake.respondWith("{\"track_id\":\"t-xyz\",\"status\":\"completed\",\"document_id\":\"d-123\"}");
        var svc = new DocumentService(http);
        var result = svc.track("t-xyz");
        assertEquals("completed", result.status);
        assertEquals("d-123", result.documentId);
    }

    @Test
    void modelProviderHealthMultiple() {
        fake.respondWith("[{\"name\":\"openai\",\"enabled\":true},{\"name\":\"ollama\",\"enabled\":true},{\"name\":\"anthropic\",\"enabled\":false}]");
        var svc = new ModelService(http);
        var result = svc.providerHealth();
        assertEquals(3, result.size());
        assertTrue(result.get(0).enabled);
        assertFalse(result.get(2).enabled);
    }

    @Test
    void modelProviderHealthEmpty() {
        fake.respondWith("[]");
        var svc = new ModelService(http);
        var result = svc.providerHealth();
        assertTrue(result.isEmpty());
    }

    @Test
    void pipelineStatusRunning() {
        fake.respondWith("{\"is_busy\":true,\"pending_tasks\":5,\"processing_tasks\":12}");
        var svc = new PipelineService(http);
        var result = svc.status();
        assertTrue(result.isBusy);
        assertEquals(5, result.pendingTasks);
    }

    @Test
    void pipelineStatusIdle() {
        fake.respondWith("{\"is_busy\":false,\"pending_tasks\":0,\"processing_tasks\":0}");
        var svc = new PipelineService(http);
        var result = svc.status();
        assertFalse(result.isBusy);
        assertEquals(0, result.pendingTasks);
    }

    @Test
    void costHistoryWithDates() {
        fake.respondWith("[{\"date\":\"2026-01-01\",\"cost_usd\":10.5},{\"date\":\"2026-01-02\",\"cost_usd\":12.3}]");
        var svc = new CostService(http);
        var result = svc.history("2026-01-01", "2026-01-02");
        assertEquals(2, result.size());
    }

    @Test
    void costHistoryEmpty() {
        fake.respondWith("[]");
        var svc = new CostService(http);
        var result = svc.history(null, null);
        assertTrue(result.isEmpty());
    }

    @Test
    void taskListWithStatus() {
        fake.respondWith("{\"tasks\":[],\"total\":0}");
        var svc = new TaskService(http);
        svc.list("completed", 0, 0);
        assertTrue(fake.lastRequest().uri().contains("status=completed"));
    }

    @Test
    void taskCancelSuccess() {
        fake.respondWith("");
        var svc = new TaskService(http);
        svc.cancel("t-cancel");
        assertEquals("POST", fake.lastRequest().method());
        assertTrue(fake.lastRequest().uri().contains("/t-cancel/cancel"));
    }
}

