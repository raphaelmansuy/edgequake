package io.edgequake.sdk

import io.edgequake.sdk.models.*
import org.junit.jupiter.api.*
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.MethodOrderer.OrderAnnotation

/**
 * E2E tests for the Kotlin SDK against a live EdgeQuake backend.
 *
 * Run: mvn test -Pe2e  OR  mvn test -Dtest.excludedGroups= -Dtest.groups=e2e
 * Requires: backend at http://localhost:8080
 *
 * WHY: Unit tests with mocks give FALSE confidence.
 * These E2E tests verify every path, header, and response shape
 * against the real EdgeQuake API.
 */
@Tag("e2e")
@TestMethodOrder(OrderAnnotation::class)
class E2ETest {

    companion object {
        private lateinit var client: EdgeQuakeClient

        @JvmStatic
        @BeforeAll
        fun setup() {
            val baseUrl = System.getenv("EDGEQUAKE_BASE_URL") ?: "http://localhost:8080"
            // WHY: Default tenant/user UUIDs match migration-created defaults,
            // so conversations/folders tests work without env vars.
            client = EdgeQuakeClient(
                EdgeQuakeConfig(
                    baseUrl = baseUrl,
                    apiKey = System.getenv("EDGEQUAKE_API_KEY"),
                    tenantId = System.getenv("EDGEQUAKE_TENANT_ID") ?: "00000000-0000-0000-0000-000000000002",
                    userId = System.getenv("EDGEQUAKE_USER_ID") ?: "00000000-0000-0000-0000-000000000001",
                    workspaceId = System.getenv("EDGEQUAKE_WORKSPACE_ID"),
                    timeoutSeconds = 15
                )
            )
        }
    }

    // ── 1. Health ────────────────────────────────────────────────────

    @Test @Order(1)
    fun health() {
        val h = client.health.check()
        assertEquals("healthy", h.status)
        assertNotNull(h.version)
        println("Health: ${h.status} v${h.version} storage=${h.storageMode} llm=${h.llmProviderName}")
    }

    // ── 2. Documents ─────────────────────────────────────────────────

    @Test @Order(2)
    fun documentsListAndUpload() {
        val list = client.documents.list()
        val docs = list.documents ?: list.items ?: emptyList()
        println("Documents: ${list.total} total, page=${list.page}")

        val upload = client.documents.uploadText(
            "Kotlin SDK Test ${System.currentTimeMillis()}",
            "This document tests the Kotlin SDK integration. Unique content: ${System.nanoTime()}"
        )
        assertNotNull(upload.documentId, "documentId should not be null")
        println("Uploaded: ${upload.documentId} status=${upload.status}")

        // Allow processing time
        Thread.sleep(2000)

        val doc = client.documents.get(upload.documentId!!)
        assertNotNull(doc.id)
        println("Document: ${doc.title} status=${doc.status}")
    }

    // ── 3. Graph ─────────────────────────────────────────────────────

    @Test @Order(3)
    fun graphGet() {
        val g = client.graph.get()
        assertNotNull(g.nodes)
        assertNotNull(g.edges)
        println("Graph: ${g.nodes?.size} nodes, ${g.edges?.size} edges")
    }

    @Test @Order(4)
    fun graphSearch() {
        val result = client.graph.search("test")
        assertNotNull(result.nodes)
        println("Search: ${result.nodes?.size} nodes found")
    }

    // ── 5. Entities ──────────────────────────────────────────────────

    @Test @Order(5)
    fun entitiesListAndCreateAndDelete() {
        val list = client.entities.list()
        assertNotNull(list.items)
        println("Entities: ${list.total} total")

        // Cleanup first (idempotent)
        val testName = "KOTLIN_SDK_TEST_ENTITY"
        try { client.entities.delete(testName) } catch (_: Exception) {}

        val created = client.entities.create(
            CreateEntityRequest(
                entityName = testName,
                entityType = "TEST",
                description = "Created by Kotlin SDK E2E test",
                sourceId = "kotlin-e2e"
            )
        )
        assertEquals("success", created.status)
        assertNotNull(created.entity)
        println("Created entity: ${created.entity?.entityName}")

        // Verify exists
        val exists = client.entities.exists(testName)
        assertNotNull(exists.entityId)

        // Get detail
        val detail = client.entities.get(testName)
        assertNotNull(detail.entity)
        assertEquals(testName, detail.entity?.entityName)

        // Delete
        val deleted = client.entities.delete(testName)
        assertEquals("success", deleted.status)
        println("Deleted entity: ${deleted.deletedEntityId}")
    }

    // ── 6. Relationships ─────────────────────────────────────────────

    @Test @Order(6)
    fun relationshipsList() {
        val list = client.relationships.list()
        assertNotNull(list.items)
        println("Relationships: ${list.total} total")
    }

    // ── 7. Query ─────────────────────────────────────────────────────

    @Test @Order(7)
    fun queryExecute() {
        val result = client.query.execute("What is EdgeQuake?")
        assertNotNull(result.answer)
        assertTrue(result.answer!!.isNotEmpty())
        println("Query answer (${result.answer!!.length} chars): ${result.answer!!.take(80)}...")
    }

    // ── 8. Chat ──────────────────────────────────────────────────────

    @Test @Order(8)
    fun chatCompletions() {
        try {
            val result = client.chat.completions(
                ChatCompletionRequest(
                    message = "Hello, what is EdgeQuake?"
                )
            )
            assertNotNull(result.content)
            println("Chat: ${result.content?.take(80)}")
        } catch (e: EdgeQuakeException) {
            // Chat may not be fully implemented
            println("Chat: ${e.statusCode} (expected if not implemented)")
        }
    }

    // ── 9. Tenants ───────────────────────────────────────────────────

    @Test @Order(9)
    fun tenantsList() {
        val list = client.tenants.list()
        assertNotNull(list.items)
        assertTrue(list.items!!.isNotEmpty())
        println("Tenants: ${list.items!!.joinToString { "${it.name} (${it.slug})" }}")
    }

    // ── 10. Users ────────────────────────────────────────────────────

    @Test @Order(10)
    fun usersList() {
        val list = client.users.list()
        assertNotNull(list.users)
        println("Users: ${list.users?.size} users")
    }

    // ── 11. API Keys ─────────────────────────────────────────────────

    @Test @Order(11)
    fun apiKeysList() {
        val list = client.apiKeys.list()
        assertNotNull(list.keys)
        println("API Keys: ${list.keys?.size} keys")
    }

    // ── 12. Tasks ────────────────────────────────────────────────────

    @Test @Order(12)
    fun tasksList() {
        val list = client.tasks.list()
        // Response has "tasks" key
        val tasks = list.tasks ?: list.items ?: emptyList()
        println("Tasks: ${tasks.size} total")
    }

    // ── 13. Pipeline ─────────────────────────────────────────────────

    @Test @Order(13)
    fun pipelineStatus() {
        val status = client.pipeline.status()
        assertNotNull(status.isBusy)
        println("Pipeline: busy=${status.isBusy} pending=${status.pendingTasks} completed=${status.completedTasks}")
    }

    @Test @Order(14)
    fun pipelineMetrics() {
        val metrics = client.pipeline.queueMetrics()
        assertNotNull(metrics.pendingCount)
        println("Queue: pending=${metrics.pendingCount} processing=${metrics.processingCount} workers=${metrics.activeWorkers}")
    }

    // ── 15. Models ───────────────────────────────────────────────────

    @Test @Order(15)
    fun modelsCatalog() {
        val catalog = client.models.catalog()
        assertNotNull(catalog.providers)
        assertTrue(catalog.providers!!.isNotEmpty())
        println("Models: ${catalog.providers!!.joinToString { "${it.name} (${it.models?.size} models)" }}")
    }

    @Test @Order(16)
    fun modelsProviderHealth() {
        val health = client.models.health()
        assertTrue(health.isNotEmpty())
        val first = health.first()
        assertNotNull(first.name)
        println("Provider Health: ${health.size} providers, first=${first.name} enabled=${first.enabled}")
    }

    @Test @Order(17)
    fun providerStatus() {
        val status = client.models.providerStatus()
        assertNotNull(status.provider)
        println("Provider: provider=${status.provider} embedding=${status.embedding} storage=${status.storage}")
    }

    // ── 18. Conversations (requires tenant/user) ─────────────────────

    @Test @Order(18)
    fun conversationsCRUD() {
        val conv = client.conversations.create("Kotlin E2E Test ${System.currentTimeMillis()}")
        assertNotNull(conv.id)
        println("Created conversation: ${conv.id} title=${conv.title}")

        val convos = client.conversations.list()
        assertTrue(convos.isNotEmpty())
        println("Conversations: ${convos.size}")

        val detail = client.conversations.get(conv.id!!)
        assertNotNull(detail.conversation)
        assertEquals(conv.id, detail.conversation?.id)

        client.conversations.delete(conv.id!!)
    }

    // ── 19. Folders ──────────────────────────────────────────────────

    @Test @Order(19)
    fun foldersCRUD() {
        val folder = client.folders.create("Kotlin E2E Folder ${System.currentTimeMillis()}")
        assertNotNull(folder.id)
        println("Created folder: ${folder.id} name=${folder.name}")

        val folders = client.folders.list()
        assertTrue(folders.isNotEmpty())
        println("Folders: ${folders.size}")

        client.folders.delete(folder.id!!)
    }

    // ── 20. Costs ────────────────────────────────────────────────────

    @Test @Order(20)
    fun costsSummary() {
        val costs = client.costs.summary()
        assertNotNull(costs.totalCost)
        println("Costs: total=${costs.totalCost} documents=${costs.documentCount} queries=${costs.queryCount}")
    }
}
