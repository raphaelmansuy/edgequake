error id: file://<WORKSPACE>/sdks/java/src/test/java/io/edgequake/sdk/UnitTest.java
file://<WORKSPACE>/sdks/java/src/test/java/io/edgequake/sdk/UnitTest.java
### com.thoughtworks.qdox.parser.ParseException: syntax error @[1075,1]

error in qdox parser
file content:
```java
offset: 34939
uri: file://<WORKSPACE>/sdks/java/src/test/java/io/edgequake/sdk/UnitTest.java
text:
```scala
package io.edgequake.sdk;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.*;
import io.edgequake.sdk.models.AuthModels.*;
import io.edgequake.sdk.models.DocumentModels.*;
import io.edgequake.sdk.models.GraphModels.*;
import io.edgequake.sdk.models.OperationModels.*;
import io.edgequake.sdk.models.QueryModels.*;
import io.edgequake.sdk.resources.*;

import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;

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
            {"id":"chat-1","choices":[{"index":0,"message":{"role":"assistant","content":"Hello!"},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}
            """);
        var svc = new ChatService(http);
        var req = new ChatCompletionRequest(List.of(new ChatMessage("user", "Hi")));
        var result = svc.completions(req);
        assertEquals("chat-1", result.id);
        assertEquals("Hello!", result.choices.get(0).message.content);
        assertEquals(8, result.usage.totalTokens);
    }

    @Test
    void chatError() {
        fake.respondWithError(500);
        assertThrows(EdgeQuakeException.class, () ->
                new ChatService(http).completions(new ChatCompletionRequest(List.of(new ChatMessage("user", "Hi")))));
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
            [{"id":"c1","title":"Test Chat","message_count":5}]
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
            {"id":"c1","title":"Chat","messages":[{"id":"m1","role":"user","content":"Hello"}]}
            """);
        var svc = new ConversationService(http);
        var result = svc.get("c1");
        assertEquals("c1", result.id);
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
}
@@
```

```



#### Error stacktrace:

```
com.thoughtworks.qdox.parser.impl.Parser.yyerror(Parser.java:2025)
	com.thoughtworks.qdox.parser.impl.Parser.yyparse(Parser.java:2147)
	com.thoughtworks.qdox.parser.impl.Parser.parse(Parser.java:2006)
	com.thoughtworks.qdox.library.SourceLibrary.parse(SourceLibrary.java:232)
	com.thoughtworks.qdox.library.SourceLibrary.parse(SourceLibrary.java:190)
	com.thoughtworks.qdox.library.SourceLibrary.addSource(SourceLibrary.java:94)
	com.thoughtworks.qdox.library.SourceLibrary.addSource(SourceLibrary.java:89)
	com.thoughtworks.qdox.library.SortedClassLibraryBuilder.addSource(SortedClassLibraryBuilder.java:162)
	com.thoughtworks.qdox.JavaProjectBuilder.addSource(JavaProjectBuilder.java:174)
	scala.meta.internal.mtags.JavaMtags.indexRoot(JavaMtags.scala:49)
	scala.meta.internal.metals.SemanticdbDefinition$.foreachWithReturnMtags(SemanticdbDefinition.scala:99)
	scala.meta.internal.metals.Indexer.indexSourceFile(Indexer.scala:560)
	scala.meta.internal.metals.Indexer.$anonfun$reindexWorkspaceSources$3(Indexer.scala:691)
	scala.meta.internal.metals.Indexer.$anonfun$reindexWorkspaceSources$3$adapted(Indexer.scala:688)
	scala.collection.IterableOnceOps.foreach(IterableOnce.scala:630)
	scala.collection.IterableOnceOps.foreach$(IterableOnce.scala:628)
	scala.collection.AbstractIterator.foreach(Iterator.scala:1313)
	scala.meta.internal.metals.Indexer.reindexWorkspaceSources(Indexer.scala:688)
	scala.meta.internal.metals.MetalsLspService.$anonfun$onChange$2(MetalsLspService.scala:936)
	scala.runtime.java8.JFunction0$mcV$sp.apply(JFunction0$mcV$sp.scala:18)
	scala.concurrent.Future$.$anonfun$apply$1(Future.scala:691)
	scala.concurrent.impl.Promise$Transformation.run(Promise.scala:500)
	java.base/java.util.concurrent.ThreadPoolExecutor.runWorker(ThreadPoolExecutor.java:1136)
	java.base/java.util.concurrent.ThreadPoolExecutor$Worker.run(ThreadPoolExecutor.java:635)
	java.base/java.lang.Thread.run(Thread.java:840)
```
#### Short summary: 

QDox parse error in file://<WORKSPACE>/sdks/java/src/test/java/io/edgequake/sdk/UnitTest.java