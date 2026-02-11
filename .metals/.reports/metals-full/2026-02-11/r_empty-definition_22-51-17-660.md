error id: file://<WORKSPACE>/sdks/java/src/test/java/io/edgequake/sdk/E2ETest.java:_empty_/EdgeQuakeClient#entities#delete#message#
file://<WORKSPACE>/sdks/java/src/test/java/io/edgequake/sdk/E2ETest.java
empty definition using pc, found symbol in pc: _empty_/EdgeQuakeClient#entities#delete#message#
empty definition using semanticdb
empty definition using fallback
non-local guesses:

offset: 5418
uri: file://<WORKSPACE>/sdks/java/src/test/java/io/edgequake/sdk/E2ETest.java
text:
```scala
package io.edgequake.sdk;

import io.edgequake.sdk.models.AuthModels.*;
import io.edgequake.sdk.models.GraphModels.*;
import io.edgequake.sdk.models.OperationModels.*;
import io.edgequake.sdk.models.QueryModels.*;
import org.junit.jupiter.api.*;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * E2E tests for the Java SDK against a live EdgeQuake backend.
 *
 * Run with: mvn test -Pe2e
 * Or: mvn test -Dgroups=e2e
 *
 * Requires: backend running at http://localhost:8080
 *
 * WHY: Unit tests with mocked servers give FALSE confidence.
 * These E2E tests verify every path, header, and response shape
 * against the real EdgeQuake API.
 */
@Tag("e2e")
@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
class E2ETest {

    private static EdgeQuakeClient client;

    @BeforeAll
    static void setup() {
        var baseUrl = System.getenv("EDGEQUAKE_BASE_URL");
        if (baseUrl == null || baseUrl.isEmpty()) baseUrl = "http://localhost:8080";

        var builder = EdgeQuakeConfig.builder()
                .baseUrl(baseUrl)
                .timeoutSeconds(15);

        var apiKey = System.getenv("EDGEQUAKE_API_KEY");
        if (apiKey != null && !apiKey.isEmpty()) builder.apiKey(apiKey);

        var tenantId = System.getenv("EDGEQUAKE_TENANT_ID");
        if (tenantId != null && !tenantId.isEmpty()) builder.tenantId(tenantId);

        var userId = System.getenv("EDGEQUAKE_USER_ID");
        if (userId != null && !userId.isEmpty()) builder.userId(userId);

        var workspaceId = System.getenv("EDGEQUAKE_WORKSPACE_ID");
        if (workspaceId != null && !workspaceId.isEmpty()) builder.workspaceId(workspaceId);

        client = new EdgeQuakeClient(builder.build());
    }

    // ── 1. Health ────────────────────────────────────────────────────

    @Test @Order(1)
    void health() {
        var h = client.health().check();
        assertEquals("healthy", h.status);
        assertNotNull(h.version);
        assertNotNull(h.storageMode);
        System.out.printf("Health: status=%s version=%s storage=%s llm=%s%n",
                h.status, h.version, h.storageMode, h.llmProviderName);
    }

    // ── 2. Documents ─────────────────────────────────────────────────

    @Test @Order(2)
    void documentsListAndUpload() {
        var list = client.documents().list(1, 10);
        assertNotNull(list.documents);
        System.out.printf("Documents: %d total%n", list.documents.size());

        var content = "EdgeQuake is an advanced Retrieval-Augmented Generation RAG framework " +
                "implemented in Rust. It uses graph-based knowledge representation for enhanced " +
                "retrieval and entity extraction with deduplication algorithms.";
        var upload = client.documents().uploadText(content, "Java SDK E2E Test");
        assertNotNull(upload.documentId, "upload should return document_id");
        System.out.printf("Uploaded: id=%s status=%s%n", upload.documentId, upload.status);

        var doc = client.documents().get(upload.documentId);
        assertNotNull(doc.id);
        System.out.printf("Document: id=%s title=%s status=%s%n", doc.id, doc.title, doc.status);

        try { client.documents().delete(upload.documentId); }
        catch (Exception e) { System.out.println("Cleanup warning: " + e.getMessage()); }
    }

    // ── 3. Graph ─────────────────────────────────────────────────────

    @Test @Order(3)
    void graphGet() {
        var g = client.graph().get(10);
        assertNotNull(g.nodes);
        assertNotNull(g.edges);
        System.out.printf("Graph: %d nodes, %d edges%n", g.nodes.size(), g.edges.size());
    }

    @Test @Order(4)
    void graphSearch() {
        var results = client.graph().search("test", 5);
        assertNotNull(results.nodes);
        System.out.printf("Search: %d nodes%n", results.nodes.size());
    }

    // ── 5. Entities ──────────────────────────────────────────────────

    @Test @Order(5)
    void entitiesListAndCreateAndDelete() {
        // WHY: Clean up from previous runs to make test idempotent.
        try { client.entities().delete("JAVA_E2E_TEST_ENTITY"); }
        catch (Exception ignored) {}

        // List
        var list = client.entities().list(1, 10, null);
        assertNotNull(list.items);
        System.out.printf("Entities: %d total (page %d)%n", list.total, list.page);

        // Create
        var req = new CreateEntityRequest(
                "JAVA_E2E_TEST_ENTITY", "TEST",
                "Created by Java SDK E2E test", "manual_entry");
        var created = client.entities().create(req);
        assertNotNull(created.entity);
        assertEquals("JAVA_E2E_TEST_ENTITY", created.entity.entityName);
        System.out.printf("Created: %s status=%s%n",
                created.entity.entityName, created.status);

        // Exists
        var exists = client.entities().exists("JAVA_E2E_TEST_ENTITY");
        assertTrue(exists.exists, "Entity should exist after creation");
        assertNotNull(exists.entityId);

        // Get detail
        var detail = client.entities().get("JAVA_E2E_TEST_ENTITY");
        assertNotNull(detail.entity);
        assertEquals("JAVA_E2E_TEST_ENTITY", detail.entity.entityName);

        // Delete
        var deleteResp = client.entities().delete("JAVA_E2E_TEST_ENTITY");
        assertNotNull(deleteResp);
        System.out.printf("Deleted: %s%n", deleteResp.@@message);
    }

    // ── 6. Relationships ────────────────────────────────────────────

    @Test @Order(6)
    void relationshipsList() {
        var list = client.relationships().list(1, 10);
        assertNotNull(list.items);
        System.out.printf("Relationships: %d total (page %d)%n", list.total, list.page);
    }

    // ── 7. Query ─────────────────────────────────────────────────────

    @Test @Order(7)
    void queryExecute() {
        try {
            var resp = client.query().execute(new QueryRequest("What is EdgeQuake?", "hybrid"));
            System.out.printf("Query: answer=%s sources=%d%n",
                    truncate(resp.answer, 80), resp.sources != null ? resp.sources.size() : 0);
        } catch (Exception e) {
            System.out.println("Query may need LLM: " + e.getMessage());
        }
    }

    // ── 8. Chat ──────────────────────────────────────────────────────

    @Test @Order(8)
    void chatCompletions() {
        try {
            var req = new ChatCompletionRequest(List.of(
                    new ChatMessage("user", "Hello, what is EdgeQuake?")));
            var resp = client.chat().completions(req);
            if (resp.choices != null && !resp.choices.isEmpty()) {
                System.out.printf("Chat: %s%n",
                        truncate(resp.choices.get(0).message.content, 80));
            }
        } catch (Exception e) {
            System.out.println("Chat may need LLM: " + e.getMessage());
        }
    }

    // ── 9. Tenants ───────────────────────────────────────────────────

    @Test @Order(9)
    void tenantsList() {
        var list = client.tenants().list();
        assertNotNull(list.items);
        assertTrue(list.items.size() > 0, "Should have at least one tenant");
        System.out.printf("Tenants: %d items%n", list.items.size());
        for (var t : list.items) {
            System.out.printf("  - %s (id=%s, slug=%s)%n", t.name, t.id, t.slug);
        }
    }

    // ── 10. Users ────────────────────────────────────────────────────

    @Test @Order(10)
    void usersList() {
        var list = client.users().list();
        assertNotNull(list.users);
        System.out.printf("Users: %d users%n", list.users.size());
    }

    // ── 11. API Keys ─────────────────────────────────────────────────

    @Test @Order(11)
    void apiKeysList() {
        var list = client.apiKeys().list();
        assertNotNull(list.keys);
        System.out.printf("API Keys: %d keys%n", list.keys.size());
    }

    // ── 12. Tasks ────────────────────────────────────────────────────

    @Test @Order(12)
    void tasksList() {
        var list = client.tasks().list(null, 1, 10);
        assertNotNull(list.tasks);
        System.out.printf("Tasks: %d total%n", list.total);
    }

    // ── 13. Pipeline Status ──────────────────────────────────────────

    @Test @Order(13)
    void pipelineStatus() {
        var status = client.pipeline().status();
        // WHY: Uses is_busy, pending_tasks, processing_tasks fields.
        System.out.printf("Pipeline: busy=%s pending=%d processing=%d completed=%d%n",
                status.isBusy, status.pendingTasks, status.processingTasks, status.completedTasks);
    }

    // ── 14. Pipeline Metrics ─────────────────────────────────────────

    @Test @Order(14)
    void pipelineMetrics() {
        var metrics = client.pipeline().metrics();
        assertNotNull(metrics);
        System.out.printf("Queue: depth=%d processing=%d completed_hour=%d%n",
                metrics.queueDepth, metrics.processing, metrics.completedLastHour);
    }

    // ── 15. Models (Provider Catalog) ────────────────────────────────

    @Test @Order(15)
    void modelsList() {
        var catalog = client.models().list();
        assertNotNull(catalog.providers);
        assertTrue(catalog.providers.size() > 0, "Should have providers");
        System.out.printf("Models: %d providers%n", catalog.providers.size());
        for (var p : catalog.providers) {
            System.out.printf("  - %s (%s) models=%d%n",
                    p.name, p.displayName, p.models != null ? p.models.size() : 0);
        }
    }

    // ── 16. Provider Health ──────────────────────────────────────────

    @Test @Order(16)
    void modelsProviderHealth() {
        var health = client.models().providerHealth();
        assertNotNull(health);
        assertTrue(health.size() > 0, "Should have provider health entries");
        var first = health.get(0);
        assertNotNull(first.name);
        System.out.printf("Provider Health: %d providers, first=%s enabled=%s%n",
                health.size(), first.name, first.enabled);
    }

    // ── 17. Provider Status ──────────────────────────────────────────

    @Test @Order(17)
    void providerStatus() {
        var status = client.models().providerStatus();
        assertNotNull(status);
        System.out.printf("Provider: provider=%s model=%s status=%s%n",
                status.currentProvider, status.currentModel, status.status);
    }

    // ── 18. Conversations (requires tenant/user headers) ────────────

    @Test @Order(18)
    void conversationsCRUD() {
        try {
            var conv = client.conversations().create(
                    new CreateConversationRequest("Java SDK E2E Conversation"));
            assertNotNull(conv.id);
            System.out.printf("Created conversation: id=%s title=%s%n", conv.id, conv.title);

            var convs = client.conversations().list();
            assertNotNull(convs);
            System.out.printf("Conversations: %d total%n", convs.size());

            var detail = client.conversations().get(conv.id);
            assertNotNull(detail.id);

            var msg = client.conversations().createMessage(conv.id,
                    new CreateMessageRequest("user", "Hello from Java SDK"));
            assertNotNull(msg.id);

            client.conversations().delete(conv.id);
        } catch (EdgeQuakeException e) {
            if (e.statusCode() == 400 || e.statusCode() == 401) {
                System.out.println("Conversations need tenant/user headers — skipping");
                Assumptions.assumeTrue(false, "Requires EDGEQUAKE_TENANT_ID and EDGEQUAKE_USER_ID");
            }
            throw e;
        }
    }

    // ── 19. Folders (requires tenant/user headers) ───────────────────

    @Test @Order(19)
    void foldersCRUD() {
        try {
            var folder = client.folders().create(new CreateFolderRequest("Java SDK E2E Folder"));
            assertNotNull(folder.id);
            System.out.printf("Created folder: id=%s name=%s%n", folder.id, folder.name);

            var folders = client.folders().list();
            assertNotNull(folders);

            client.folders().delete(folder.id);
        } catch (EdgeQuakeException e) {
            if (e.statusCode() == 400 || e.statusCode() == 401) {
                System.out.println("Folders need tenant/user headers — skipping");
                Assumptions.assumeTrue(false, "Requires EDGEQUAKE_TENANT_ID and EDGEQUAKE_USER_ID");
            }
            throw e;
        }
    }

    // ── 20. Costs ────────────────────────────────────────────────────

    @Test @Order(20)
    void costsSummary() {
        try {
            var summary = client.costs().summary();
            assertNotNull(summary);
            System.out.printf("Costs: total=%.4f documents=%d queries=%d%n",
                    summary.totalCostUsd, summary.documentCount, summary.queryCount);
        } catch (EdgeQuakeException e) {
            System.out.println("Costs endpoint may not be available: " + e.getMessage());
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    private static String truncate(String s, int max) {
        if (s == null) return "<null>";
        return s.length() <= max ? s : s.substring(0, max) + "...";
    }
}

```


#### Short summary: 

empty definition using pc, found symbol in pc: _empty_/EdgeQuakeClient#entities#delete#message#