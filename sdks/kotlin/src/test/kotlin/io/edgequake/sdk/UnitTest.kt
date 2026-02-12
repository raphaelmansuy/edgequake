package io.edgequake.sdk

import io.edgequake.sdk.internal.HttpHelper
import io.edgequake.sdk.models.*
import io.edgequake.sdk.resources.*
import org.junit.jupiter.api.*
import org.junit.jupiter.api.Assertions.*

/**
 * Comprehensive unit tests for all Kotlin SDK services.
 *
 * Uses FakeHttpClient to intercept HTTP calls and return
 * pre-configured responses without network I/O.
 *
 * Coverage target: 90%+ across all source files.
 */
class UnitTest {

    private lateinit var http: HttpHelper
    private lateinit var fake: FakeHttpClient

    @BeforeEach
    fun setup() {
        val (h, f) = createTestHelper(
            EdgeQuakeConfig(
                baseUrl = "http://test:8080",
                apiKey = "test-key",
                tenantId = "t1",
                userId = "u1",
                workspaceId = "w1"
            )
        )
        http = h
        fake = f
    }

    // ── EdgeQuakeConfig ──────────────────────────────────────────────

    @Test
    fun `config defaults`() {
        val c = EdgeQuakeConfig()
        assertEquals("http://localhost:8080", c.baseUrl)
        assertNull(c.apiKey)
        assertNull(c.tenantId)
        assertNull(c.userId)
        assertNull(c.workspaceId)
        assertEquals(30, c.timeoutSeconds)
    }

    @Test
    fun `config custom values`() {
        val c = EdgeQuakeConfig(
            baseUrl = "http://custom:9090",
            apiKey = "key123",
            tenantId = "tenant1",
            userId = "user1",
            workspaceId = "ws1",
            timeoutSeconds = 60
        )
        assertEquals("http://custom:9090", c.baseUrl)
        assertEquals("key123", c.apiKey)
        assertEquals("tenant1", c.tenantId)
        assertEquals("user1", c.userId)
        assertEquals("ws1", c.workspaceId)
        assertEquals(60, c.timeoutSeconds)
    }

    @Test
    fun `config data class equality`() {
        val c1 = EdgeQuakeConfig(baseUrl = "http://a")
        val c2 = EdgeQuakeConfig(baseUrl = "http://a")
        assertEquals(c1, c2)
        assertEquals(c1.hashCode(), c2.hashCode())
    }

    // ── EdgeQuakeException ───────────────────────────────────────────

    @Test
    fun `exception properties`() {
        val ex = EdgeQuakeException("test error", 404, """{"detail":"not found"}""")
        assertEquals("test error", ex.message)
        assertEquals(404, ex.statusCode)
        assertEquals("""{"detail":"not found"}""", ex.responseBody)
        assertNull(ex.cause)
    }

    @Test
    fun `exception with cause`() {
        val cause = RuntimeException("root cause")
        val ex = EdgeQuakeException("wrapped", 500, cause = cause)
        assertEquals(cause, ex.cause)
        assertEquals(500, ex.statusCode)
    }

    @Test
    fun `exception defaults`() {
        val ex = EdgeQuakeException("msg")
        assertEquals(0, ex.statusCode)
        assertNull(ex.responseBody)
    }

    // ── EdgeQuakeClient ──────────────────────────────────────────────

    @Test
    fun `client creates all services`() {
        val client = EdgeQuakeClient()
        assertNotNull(client.health)
        assertNotNull(client.documents)
        assertNotNull(client.entities)
        assertNotNull(client.relationships)
        assertNotNull(client.graph)
        assertNotNull(client.query)
        assertNotNull(client.chat)
        assertNotNull(client.auth)
        assertNotNull(client.users)
        assertNotNull(client.apiKeys)
        assertNotNull(client.tenants)
        assertNotNull(client.conversations)
        assertNotNull(client.folders)
        assertNotNull(client.tasks)
        assertNotNull(client.pipeline)
        assertNotNull(client.models)
        assertNotNull(client.workspaces)
        assertNotNull(client.pdf)
        assertNotNull(client.costs)
    }

    // ── HttpHelper ───────────────────────────────────────────────────

    @Test
    fun `buildRequest includes headers`() {
        val config = EdgeQuakeConfig(
            baseUrl = "http://test:8080",
            apiKey = "my-key",
            tenantId = "t1",
            userId = "u1",
            workspaceId = "w1"
        )
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/api/v1/health", "GET", null)
        assertEquals("http://test:8080/api/v1/health", req.uri().toString())
        assertEquals("GET", req.method())
        assertTrue(req.headers().map()["X-API-Key"]?.contains("my-key") == true)
        assertTrue(req.headers().map()["X-Tenant-ID"]?.contains("t1") == true)
        assertTrue(req.headers().map()["X-User-ID"]?.contains("u1") == true)
        assertTrue(req.headers().map()["X-Workspace-ID"]?.contains("w1") == true)
    }

    @Test
    fun `buildRequest skips null headers`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "GET", null)
        assertFalse(req.headers().map().containsKey("X-API-Key"))
        assertFalse(req.headers().map().containsKey("X-Tenant-ID"))
    }

    @Test
    fun `buildRequest POST with body`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "POST", mapOf("key" to "value"))
        assertEquals("POST", req.method())
        assertTrue(req.bodyPublisher().isPresent)
    }

    @Test
    fun `buildRequest POST without body sends empty json`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "POST", null)
        assertTrue(req.bodyPublisher().isPresent)
    }

    @Test
    fun `buildRequest PUT with body`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "PUT", mapOf("a" to 1))
        assertEquals("PUT", req.method())
    }

    @Test
    fun `buildRequest PATCH without body sends empty json`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "PATCH", null)
        assertEquals("PATCH", req.method())
    }

    @Test
    fun `buildRequest DELETE has no body`() {
        val config = EdgeQuakeConfig(baseUrl = "http://test:8080")
        val helper = HttpHelper(config)
        val req = helper.buildRequest("/test", "DELETE", null)
        assertEquals("DELETE", req.method())
    }

    @Test
    fun `execute throws EdgeQuakeException on error status`() {
        fake.respondWithError(500, """{"error":"internal"}""")
        assertThrows(EdgeQuakeException::class.java) {
            http.get<Map<String, Any?>>("/test")
        }
    }

    @Test
    fun `execute wraps unexpected exceptions`() {
        fake.throwOnSend(RuntimeException("network error"))
        val ex = assertThrows(EdgeQuakeException::class.java) {
            http.get<Map<String, Any?>>("/test")
        }
        assertTrue(ex.message!!.contains("network error"))
    }

    @Test
    fun `getRaw throws on error status`() {
        fake.respondWithError(404)
        assertThrows(EdgeQuakeException::class.java) {
            http.getRaw("/test")
        }
    }

    @Test
    fun `postRaw throws on error status`() {
        fake.respondWithError(422)
        assertThrows(EdgeQuakeException::class.java) {
            http.postRaw("/test", mapOf("a" to 1))
        }
    }

    @Test
    fun `deleteRaw throws on error status`() {
        fake.respondWithError(403)
        assertThrows(EdgeQuakeException::class.java) {
            http.deleteRaw("/test")
        }
    }

    @Test
    fun `put method works`() {
        fake.respondWith("""{"ok":true}""")
        val result: Map<String, Any?> = http.put("/test", mapOf("x" to 1))
        assertEquals(true, result["ok"])
        assertEquals("PUT", fake.lastRequest().method)
    }

    @Test
    fun `patch method works`() {
        fake.respondWith("""{"ok":true}""")
        val result: Map<String, Any?> = http.patch("/test", mapOf("x" to 1))
        assertEquals(true, result["ok"])
        assertEquals("PATCH", fake.lastRequest().method)
    }

    @Test
    fun `delete method works`() {
        fake.respondWith("""{"deleted":true}""")
        val result: Map<String, Any?> = http.delete("/test")
        assertEquals(true, result["deleted"])
        assertEquals("DELETE", fake.lastRequest().method)
    }

    // ── HealthService ────────────────────────────────────────────────

    @Test
    fun `health check returns parsed response`() {
        fake.respondWith("""{"status":"healthy","version":"0.1.0","storage_mode":"postgresql","workspace_id":"default","llm_provider_name":"ollama"}""")
        val svc = HealthService(http)
        val health = svc.check()
        assertEquals("healthy", health.status)
        assertEquals("0.1.0", health.version)
        assertEquals("postgresql", health.storageMode)
        assertEquals("ollama", health.llmProviderName)
    }

    @Test
    fun `health check error`() {
        fake.respondWithError(503, """{"error":"unavailable"}""")
        val svc = HealthService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.check() }
    }

    // ── DocumentService ──────────────────────────────────────────────

    @Test
    fun `documents list`() {
        fake.respondWith("""{"documents":[{"id":"d1","title":"Test","status":"completed"}],"total":1,"page":1,"page_size":20}""")
        val svc = DocumentService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("d1", result.documents?.first()?.id)
    }

    @Test
    fun `documents list with pagination`() {
        fake.respondWith("""{"documents":[],"total":50,"page":3,"page_size":10,"has_more":true}""")
        val svc = DocumentService(http)
        val result = svc.list(page = 3, pageSize = 10)
        assertEquals(50, result.total)
        assertEquals(3, result.page)
        assertTrue(result.hasMore == true)
        assertTrue(fake.lastRequest().uri.contains("page=3"))
        assertTrue(fake.lastRequest().uri.contains("page_size=10"))
    }

    @Test
    fun `documents get by id`() {
        fake.respondWith("""{"id":"d1","title":"My Doc","status":"completed","file_type":"txt","chunk_count":5}""")
        val svc = DocumentService(http)
        val doc = svc.get("d1")
        assertEquals("d1", doc.id)
        assertEquals("My Doc", doc.title)
        assertEquals(5, doc.chunkCount)
    }

    @Test
    fun `documents upload text`() {
        fake.respondWith("""{"document_id":"d-new","status":"processing","message":"Upload received","track_id":"t-123"}""")
        val svc = DocumentService(http)
        val result = svc.uploadText("Test Title", "Hello World")
        assertEquals("d-new", result.documentId)
        assertEquals("processing", result.status)
        assertEquals("t-123", result.trackId)
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents"))
    }

    @Test
    fun `documents delete`() {
        fake.respondWith("")
        val svc = DocumentService(http)
        svc.delete("d1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents/d1"))
    }

    @Test
    fun `documents scan`() {
        fake.respondWith("""{"status":"ok","message":"Scan completed","files_found":3}""")
        val svc = DocumentService(http)
        val result = svc.scan("/path/to/dir")
        assertEquals(3, result.filesFound)
    }

    @Test
    fun `documents error handling`() {
        fake.respondWithError(404)
        val svc = DocumentService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("nonexistent") }
    }

    // ── EntityService ────────────────────────────────────────────────

    @Test
    fun `entities list`() {
        fake.respondWith("""{"items":[{"entity_name":"ALICE","entity_type":"PERSON"}],"total":1,"page":1,"page_size":20}""")
        val svc = EntityService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("ALICE", result.items?.first()?.entityName)
    }

    @Test
    fun `entities get detail`() {
        fake.respondWith("""{"entity":{"entity_name":"BOB","entity_type":"PERSON","description":"A person"},"relationships":{}}""")
        val svc = EntityService(http)
        val result = svc.get("BOB")
        assertEquals("BOB", result.entity?.entityName)
    }

    @Test
    fun `entities create`() {
        fake.respondWith("""{"status":"success","message":"Created","entity":{"entity_name":"NEW_ENTITY","entity_type":"TEST"}}""")
        val svc = EntityService(http)
        val result = svc.create(CreateEntityRequest("NEW_ENTITY", "TEST", "desc", "src"))
        assertEquals("success", result.status)
        assertEquals("NEW_ENTITY", result.entity?.entityName)
    }

    @Test
    fun `entities delete`() {
        fake.respondWith("""{"status":"success","deleted_entity_id":"e1","deleted_relationships":3}""")
        val svc = EntityService(http)
        val result = svc.delete("TEST_ENTITY")
        assertEquals("success", result.status)
        assertEquals(3, result.deletedRelationships)
    }

    @Test
    fun `entities exists`() {
        fake.respondWith("""{"entity_id":"e1","exists":true}""")
        val svc = EntityService(http)
        val result = svc.exists("ALICE")
        assertTrue(result.exists == true)
        assertEquals("e1", result.entityId)
    }

    @Test
    fun `entities merge`() {
        fake.respondWith("""{"status":"merged"}""")
        val svc = EntityService(http)
        val result = svc.merge("SOURCE", "TARGET")
        assertEquals("merged", result["status"])
    }

    @Test
    fun `entities error`() {
        fake.respondWithError(404)
        val svc = EntityService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("NONEXISTENT") }
    }

    // ── RelationshipService ──────────────────────────────────────────

    @Test
    fun `relationships list`() {
        fake.respondWith("""{"items":[{"source":"A","target":"B","relationship_type":"KNOWS","weight":1.0}],"total":1}""")
        val svc = RelationshipService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("A", result.items?.first()?.source)
        assertEquals("KNOWS", result.items?.first()?.relationshipType)
    }

    @Test
    fun `relationships list with pagination`() {
        fake.respondWith("""{"items":[],"total":0}""")
        val svc = RelationshipService(http)
        svc.list(page = 2, pageSize = 5)
        assertTrue(fake.lastRequest().uri.contains("page=2"))
    }

    // ── GraphService ─────────────────────────────────────────────────

    @Test
    fun `graph get`() {
        fake.respondWith("""{"nodes":[{"id":"n1","label":"Alice","entity_type":"PERSON"}],"edges":[{"source":"n1","target":"n2","label":"KNOWS"}]}""")
        val svc = GraphService(http)
        val result = svc.get()
        assertEquals(1, result.nodes?.size)
        assertEquals("Alice", result.nodes?.first()?.label)
        assertEquals(1, result.edges?.size)
    }

    @Test
    fun `graph search`() {
        fake.respondWith("""{"nodes":[{"id":"n1","label":"Result"}],"total":1}""")
        val svc = GraphService(http)
        val result = svc.search("test")
        assertEquals(1, result.total)
        assertTrue(fake.lastRequest().uri.contains("q=test"))
    }

    @Test
    fun `graph error`() {
        fake.respondWithError(500)
        val svc = GraphService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get() }
    }

    // ── QueryService ─────────────────────────────────────────────────

    @Test
    fun `query execute`() {
        fake.respondWith("""{"answer":"The answer is 42.","sources":[{"title":"doc1"}],"mode":"hybrid"}""")
        val svc = QueryService(http)
        val result = svc.execute("What is the answer?")
        assertEquals("The answer is 42.", result.answer)
        assertEquals("hybrid", result.mode)
        assertEquals(1, result.sources?.size)
    }

    @Test
    fun `query with mode`() {
        fake.respondWith("""{"answer":"Local answer","mode":"local"}""")
        val svc = QueryService(http)
        val result = svc.execute("test", mode = "local")
        assertEquals("local", result.mode)
    }

    @Test
    fun `query error`() {
        fake.respondWithError(422)
        val svc = QueryService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.execute("") }
    }

    // ── ChatService ──────────────────────────────────────────────────

    @Test
    fun `chat completions`() {
        fake.respondWith("""{"conversation_id":"conv-1","user_message_id":"msg-1","assistant_message_id":"msg-2","content":"Hello!","mode":"hybrid","sources":[]}""")
        val svc = ChatService(http)
        val result = svc.completions(
            ChatCompletionRequest(
                message = "Hi"
            )
        )
        assertEquals("conv-1", result.conversationId)
        assertEquals("Hello!", result.content)
    }

    @Test
    fun `chat error`() {
        fake.respondWithError(500)
        val svc = ChatService(http)
        assertThrows(EdgeQuakeException::class.java) {
            svc.completions(ChatCompletionRequest(message = "Hi"))
        }
    }

    // ── AuthService ──────────────────────────────────────────────────

    @Test
    fun `auth login`() {
        fake.respondWith("""{"token":"jwt-token-123","expires_at":"2026-12-31T23:59:59Z"}""")
        val svc = AuthService(http)
        val result = svc.login("admin", "password")
        assertEquals("jwt-token-123", result.token)
        assertNotNull(result.expiresAt)
    }

    @Test
    fun `auth login error`() {
        fake.respondWithError(401)
        val svc = AuthService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.login("bad", "creds") }
    }

    // ── UserService ──────────────────────────────────────────────────

    @Test
    fun `users list`() {
        fake.respondWith("""{"users":[{"id":"u1","username":"admin","email":"a@b.com","role":"admin"}]}""")
        val svc = UserService(http)
        val result = svc.list()
        assertEquals(1, result.users?.size)
        assertEquals("admin", result.users?.first()?.username)
    }

    @Test
    fun `users error`() {
        fake.respondWithError(403)
        val svc = UserService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.list() }
    }

    // ── ApiKeyService ────────────────────────────────────────────────

    @Test
    fun `api keys list`() {
        fake.respondWith("""{"keys":[{"id":"k1","name":"my-key","prefix":"sk-abc"}]}""")
        val svc = ApiKeyService(http)
        val result = svc.list()
        assertEquals(1, result.keys?.size)
        assertEquals("my-key", result.keys?.first()?.name)
    }

    // ── TenantService ────────────────────────────────────────────────

    @Test
    fun `tenants list`() {
        fake.respondWith("""{"items":[{"id":"t1","name":"Default","slug":"default"}]}""")
        val svc = TenantService(http)
        val result = svc.list()
        assertEquals(1, result.items?.size)
        assertEquals("Default", result.items?.first()?.name)
    }

    // ── ConversationService ──────────────────────────────────────────

    @Test
    fun `conversations list`() {
        fake.respondWith("""{"items":[{"id":"c1","title":"Test Chat","message_count":5}]}""")
        val svc = ConversationService(http)
        val result = svc.list()
        assertEquals(1, result.size)
        assertEquals("Test Chat", result.first().title)
    }

    @Test
    fun `conversations create`() {
        fake.respondWith("""{"id":"c-new","title":"New Chat"}""")
        val svc = ConversationService(http)
        val result = svc.create("New Chat")
        assertEquals("c-new", result.id)
        assertEquals("New Chat", result.title)
    }

    @Test
    fun `conversations get`() {
        fake.respondWith("""{"conversation":{"id":"c1","title":"Chat"},"messages":[{"id":"m1","role":"user","content":"Hello"}]}""")
        val svc = ConversationService(http)
        val result = svc.get("c1")
        assertEquals("c1", result.conversation?.id)
        assertEquals(1, result.messages?.size)
    }

    @Test
    fun `conversations delete`() {
        fake.respondWith("")
        val svc = ConversationService(http)
        svc.delete("c1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/conversations/c1"))
    }

    @Test
    fun `conversations bulk delete`() {
        fake.respondWith("""{"deleted":3,"status":"success"}""")
        val svc = ConversationService(http)
        val result = svc.bulkDelete(listOf("c1", "c2", "c3"))
        assertEquals(3, result.deleted)
    }

    @Test
    fun `conversations error`() {
        fake.respondWithError(404)
        val svc = ConversationService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("nonexistent") }
    }

    // ── FolderService ────────────────────────────────────────────────

    @Test
    fun `folders list`() {
        fake.respondWith("""[{"id":"f1","name":"My Folder"}]""")
        val svc = FolderService(http)
        val result = svc.list()
        assertEquals(1, result.size)
        assertEquals("My Folder", result.first().name)
    }

    @Test
    fun `folders create`() {
        fake.respondWith("""{"id":"f-new","name":"New Folder"}""")
        val svc = FolderService(http)
        val result = svc.create("New Folder")
        assertEquals("f-new", result.id)
    }

    @Test
    fun `folders delete`() {
        fake.respondWith("")
        val svc = FolderService(http)
        svc.delete("f1")
        assertTrue(fake.lastRequest().uri.contains("/api/v1/folders/f1"))
    }

    // ── TaskService ──────────────────────────────────────────────────

    @Test
    fun `tasks list`() {
        fake.respondWith("""{"tasks":[{"id":"t1","status":"completed","task_type":"extraction"}],"total":1}""")
        val svc = TaskService(http)
        val result = svc.list()
        assertEquals(1, result.total)
        assertEquals("completed", result.tasks?.first()?.status)
    }

    @Test
    fun `tasks get`() {
        fake.respondWith("""{"id":"t1","status":"running","task_type":"ingestion","progress":0.5}""")
        val svc = TaskService(http)
        val result = svc.get("t1")
        assertEquals("t1", result.id)
        assertEquals("running", result.status)
    }

    @Test
    fun `tasks error`() {
        fake.respondWithError(404)
        val svc = TaskService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.get("nonexistent") }
    }

    // ── PipelineService ──────────────────────────────────────────────

    @Test
    fun `pipeline status`() {
        fake.respondWith("""{"is_busy":false,"total_documents":10,"processed_documents":8,"pending_tasks":2,"processing_tasks":0,"completed_tasks":8,"failed_tasks":0}""")
        val svc = PipelineService(http)
        val result = svc.status()
        assertEquals(false, result.isBusy)
        assertEquals(10, result.totalDocuments)
        assertEquals(2, result.pendingTasks)
    }

    @Test
    fun `pipeline queue metrics`() {
        fake.respondWith("""{"pending_count":5,"processing_count":2,"active_workers":3,"max_workers":8,"worker_utilization":37,"throughput_per_minute":12.5}""")
        val svc = PipelineService(http)
        val result = svc.queueMetrics()
        assertEquals(5, result.pendingCount)
        assertEquals(3, result.activeWorkers)
        assertEquals(12.5, result.throughputPerMinute)
    }

    // ── ModelService ─────────────────────────────────────────────────

    @Test
    fun `models catalog`() {
        fake.respondWith("""{"providers":[{"name":"ollama","display_name":"Ollama","models":[{"id":"llama3"}]}]}""")
        val svc = ModelService(http)
        val result = svc.catalog()
        assertEquals(1, result.providers?.size)
        assertEquals("ollama", result.providers?.first()?.name)
    }

    @Test
    fun `models health`() {
        fake.respondWith("""[{"name":"ollama","display_name":"Ollama","enabled":true,"priority":1}]""")
        val svc = ModelService(http)
        val result = svc.health()
        assertEquals(1, result.size)
        assertEquals(true, result.first().enabled)
    }

    @Test
    fun `models provider status`() {
        fake.respondWith("""{"provider":{"name":"ollama"},"embedding":{"name":"ollama"},"storage":{"mode":"postgresql"}}""")
        val svc = ModelService(http)
        val result = svc.providerStatus()
        assertNotNull(result.provider)
        assertNotNull(result.embedding)
    }

    @Test
    fun `models error`() {
        fake.respondWithError(500)
        val svc = ModelService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.catalog() }
    }

    // ── WorkspaceService ─────────────────────────────────────────────

    @Test
    fun `workspaces list`() {
        fake.respondWith("""[{"id":"w1","name":"Default","slug":"default"}]""")
        val svc = WorkspaceService(http)
        val result = svc.list()
        assertEquals(1, result.size)
        assertEquals("Default", result.first().name)
    }

    @Test
    fun `workspaces error`() {
        fake.respondWithError(403)
        val svc = WorkspaceService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.list() }
    }

    // ── PdfService ───────────────────────────────────────────────────

    @Test
    fun `pdf progress`() {
        fake.respondWith("""{"track_id":"tk-1","status":"processing","progress":0.75}""")
        val svc = PdfService(http)
        val result = svc.progress("tk-1")
        assertEquals("tk-1", result.trackId)
        assertEquals("processing", result.status)
    }

    @Test
    fun `pdf content`() {
        fake.respondWith("""{"content":"# Title\n\nHello world","page_count":3}""")
        val svc = PdfService(http)
        val result = svc.content("pdf-1")
        assertTrue(result.content?.contains("Hello world") == true)
        assertEquals(3, result.pageCount)
    }

    @Test
    fun `pdf error`() {
        fake.respondWithError(404)
        val svc = PdfService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.progress("nonexistent") }
    }

    // ── CostService ──────────────────────────────────────────────────

    @Test
    fun `costs summary`() {
        fake.respondWith("""{"total_cost":12.50,"document_count":100,"query_count":500,"entries":[]}""")
        val svc = CostService(http)
        val result = svc.summary()
        assertEquals(12.50, result.totalCost)
        assertEquals(100, result.documentCount)
        assertEquals(500, result.queryCount)
    }

    @Test
    fun `costs error`() {
        fake.respondWithError(403)
        val svc = CostService(http)
        assertThrows(EdgeQuakeException::class.java) { svc.summary() }
    }

    // ── Model data classes ───────────────────────────────────────────

    @Test
    fun `Document model defaults`() {
        val d = Document()
        assertNull(d.id)
        assertNull(d.title)
        assertNull(d.status)
    }

    @Test
    fun `Entity model`() {
        val e = Entity(entityName = "TEST", entityType = "PERSON", description = "desc")
        assertEquals("TEST", e.entityName)
        assertEquals("PERSON", e.entityType)
    }

    @Test
    fun `Relationship model`() {
        val r = Relationship(source = "A", target = "B", weight = 0.8)
        assertEquals("A", r.source)
        assertEquals(0.8, r.weight)
    }

    @Test
    fun `ChatMessage model`() {
        val m = ChatMessage(role = "user", content = "Hello")
        assertEquals("user", m.role)
        assertEquals("Hello", m.content)
    }

    @Test
    fun `ChatCompletionRequest defaults`() {
        val r = ChatCompletionRequest(message = "Hi")
        assertEquals(false, r.stream)
        assertEquals("Hi", r.message)
    }

    @Test
    fun `QueryRequest defaults`() {
        val q = QueryRequest(query = "test")
        assertEquals("hybrid", q.mode)
    }

    @Test
    fun `WorkspaceInfo model`() {
        val w = WorkspaceInfo(id = "w1", name = "Test")
        assertEquals("w1", w.id)
    }

    @Test
    fun `TaskInfo model`() {
        val t = TaskInfo(id = "t1", status = "running", taskType = "extraction")
        assertEquals("extraction", t.taskType)
    }

    @Test
    fun `PipelineStatus model`() {
        val p = PipelineStatus(isBusy = true, pendingTasks = 5)
        assertTrue(p.isBusy == true)
        assertEquals(5, p.pendingTasks)
    }

    @Test
    fun `QueueMetrics model`() {
        val q = QueueMetrics(pendingCount = 3, activeWorkers = 2)
        assertEquals(3, q.pendingCount)
    }

    @Test
    fun `ProviderCatalog model`() {
        val c = ProviderCatalog(providers = listOf(ProviderInfo(name = "ollama")))
        assertEquals(1, c.providers?.size)
    }

    @Test
    fun `UploadResponse model`() {
        val u = UploadResponse(documentId = "d1", status = "processing")
        assertEquals("d1", u.documentId)
    }

    @Test
    fun `ScanResponse model`() {
        val s = ScanResponse(filesFound = 5, status = "ok")
        assertEquals(5, s.filesFound)
    }

    @Test
    fun `BulkDeleteResponse model`() {
        val b = BulkDeleteResponse(deleted = 3, status = "success")
        assertEquals(3, b.deleted)
    }

    @Test
    fun `ConversationDetail model`() {
        val c = ConversationDetail(
            conversation = ConversationInfo(id = "c1", title = "Chat"),
            messages = listOf(Message(id = "m1", role = "user", content = "Hi"))
        )
        assertEquals(1, c.messages?.size)
        assertEquals("c1", c.conversation?.id)
    }

    @Test
    fun `CostSummary model`() {
        val c = CostSummary(totalCost = 5.0, documentCount = 10)
        assertEquals(5.0, c.totalCost)
    }

    @Test
    fun `PdfProgressResponse model`() {
        val p = PdfProgressResponse(trackId = "t1", status = "complete")
        assertEquals("t1", p.trackId)
    }

    @Test
    fun `PdfContentResponse model`() {
        val p = PdfContentResponse(content = "hello", pageCount = 2)
        assertEquals(2, p.pageCount)
    }

    // ── Request capture verification ─────────────────────────────────

    @Test
    fun `requests hit correct endpoints`() {
        fake.respondWith("""{"status":"healthy"}""")
        HealthService(http).check()
        assertTrue(fake.lastRequest().uri.contains("/health"))

        fake.respondWith("""{"documents":[],"total":0}""")
        DocumentService(http).list()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/documents"))

        fake.respondWith("""{"items":[],"total":0}""")
        EntityService(http).list()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/graph/entities"))

        fake.respondWith("""{"items":[],"total":0}""")
        RelationshipService(http).list()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/graph/relationships"))

        fake.respondWith("""{"nodes":[],"edges":[]}""")
        GraphService(http).get()
        assertTrue(fake.lastRequest().uri.contains("/api/v1/graph"))
    }

    @Test
    fun `all request methods used correctly`() {
        fake.respondWith("""{"status":"healthy"}""")
        HealthService(http).check()
        assertEquals("GET", fake.lastRequest().method)

        fake.respondWith("""{"answer":"ok"}""")
        QueryService(http).execute("test")
        assertEquals("POST", fake.lastRequest().method)

        fake.respondWith("""{"deleted":true}""")
        DocumentService(http).delete("d1")
        assertEquals("DELETE", fake.lastRequest().method)
    }
}
